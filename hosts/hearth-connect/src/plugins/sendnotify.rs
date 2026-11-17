//! This PC's notifications on the phone (KDE Connect's "Send notifications"): what apps here show in Mantel's
//! pop-ups goes to each linked phone whose "Send this PC's notifications" switch is on (off until you turn it on).
//!   PC → phone  kdeconnect.notification {id, appName, title, text, ticker, isClearable, time} and {id, isCancel}
//!   phone → PC  kdeconnect.notification.request {"cancel": id} (dismissed on the phone: closed here too) or
//!               {"request": true} (send the ones still showing)
//! It watches the session bus the way KDE Connect does (org.freedesktop.DBus.Monitoring, this user's own bus only):
//! the Notify calls apps make, the notification service's answer (the id it gave), and NotificationClosed. Nothing
//! is saved; the choices (leave out the text, only notifications that stay, how urgent, apps left out) are
//! "notifyOut" in settings.json, changed only from Settings › Phone (merge-on-write, kept 0600 with a .bak).
//! Hearth Connect's own notifications (the phone's, mirrored here) never go back to the phone.

use crate::daemon::Daemon;
use crate::packet::Packet;
use serde_json::{json, Value};
use std::path::Path;
use std::sync::{Arc, Mutex};

/// The notifications sent and still showing: (the service's id, the packet's body).
static ACTIVE: Mutex<Vec<(u32, Value)>> = Mutex::new(Vec::new());
/// Apps seen sending notifications since the service started (for Settings › Phone's "Apps left out").
static SEEN: Mutex<Vec<String>> = Mutex::new(Vec::new());

pub fn defaults() -> Value { json!({"body": true, "persistentOnly": false, "urgency": 0, "blocked": []}) }

/// The choices, with defaults for what was never set.
pub fn prefs(home: &Path) -> Value {
    let mut out = defaults();
    if let Some(v) = crate::appfile::read_json(&home.join("settings.json")) {
        if let (Some(o), Some(saved)) = (out.as_object_mut(), v.get("notifyOut").and_then(|x| x.as_object())) {
            for (k, x) in saved { if o.contains_key(k) { o.insert(k.clone(), x.clone()); } }
        }
    }
    out
}

/// Changes only the keys given (and only known ones, of the right kind); the rest of settings.json stays as it is.
pub fn set_prefs(home: &Path, changes: &Value) -> Result<Value, String> {
    let ch = changes.as_object().ok_or("nothing to change")?.clone();
    let _ = std::fs::create_dir_all(home);
    let path = home.join("settings.json");
    let existed = path.exists();
    // Worked out from the file as it is under its lock (another change a moment ago stays), and never written over a
    // file that couldn't be read (nor its .bak): then nothing changes.
    crate::appfile::update_private(&path, move |cur| {
        if cur.is_none() && existed { return (None, Err("Couldn't read Hearth Connect's settings, so nothing was changed".to_string())); }
        let mut all = cur.and_then(|v| v.as_object().cloned()).unwrap_or_default();
        let mut next = defaults();
        if let (Some(o), Some(saved)) = (next.as_object_mut(), all.get("notifyOut").and_then(|x| x.as_object())) {
            for (k, x) in saved { if o.contains_key(k) { o.insert(k.clone(), x.clone()); } }
        }
        if let Some(o) = next.as_object_mut() {
            for (k, v) in &ch {
                let ok = match k.as_str() {
                    "body" | "persistentOnly" => v.is_boolean(),
                    "urgency" => v.as_i64().map(|u| (0..=2).contains(&u)).unwrap_or(false),
                    "blocked" => v.as_array().map(|a| a.iter().all(|x| x.is_string())).unwrap_or(false),
                    _ => false,
                };
                if ok { o.insert(k.clone(), v.clone()); }
            }
        }
        all.insert("notifyOut".into(), next.clone());
        (Some(Value::Object(all)), Ok(next))
    }).map_err(|e| e.to_string())?
}

/// Should this notification go to phones? (urgency 0 low, 1 normal, 2 critical)
pub fn wanted(prefs: &Value, app: &str, urgency: u8, transient: bool, stays: bool) -> bool {
    if app == "Hearth Connect" || app == "hearth-connect" { return false; }
    if transient { return false; }
    if prefs["persistentOnly"].as_bool().unwrap_or(false) && !stays { return false; }
    if (urgency as i64) < prefs["urgency"].as_i64().unwrap_or(0) { return false; }
    let blocked = prefs["blocked"].as_array().map(|a| a.iter().any(|x| x.as_str().map(|s| s.eq_ignore_ascii_case(app)).unwrap_or(false))).unwrap_or(false);
    !blocked
}

/// Notification bodies may carry a little markup (<b>, <a href>, &amp;): plain text for the phone.
pub fn plain(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut tag = false;
    for c in s.chars() {
        match c { '<' => tag = true, '>' if tag => tag = false, _ if !tag => out.push(c), _ => {} }
    }
    out.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'").replace("&#39;", "'").replace("&amp;", "&").trim().to_string()
}

/// The packet's body for one notification.
pub fn body(prefs: &Value, sid: u32, app: &str, summary: &str, text: &str) -> Value {
    let title = plain(summary);
    let text = if prefs["body"].as_bool().unwrap_or(true) { plain(text) } else { String::new() };
    let ticker = if text.is_empty() { title.clone() } else { format!("{title}: {text}") };
    json!({"id": sid.to_string(), "appName": app, "title": title, "text": text, "ticker": ticker, "isClearable": true,
           "time": crate::now_ms().to_string(), "silent": false})
}

fn targets(d: &Daemon) -> Vec<String> {
    d.linked_paired().into_iter()
        .filter(|k| d.paired_dev(k).map(|p| p.send_notifications).unwrap_or(false) && d.peer_accepts(k, "kdeconnect.notification"))
        .collect()
}

/// One notification shown here (the service answered with its id): to the phones.
pub fn forward(d: &Daemon, sid: u32, b: Value) {
    {
        let mut a = crate::lock(&ACTIVE);
        a.retain(|(x, _)| *x != sid);
        a.push((sid, b.clone()));
        let n = a.len();
        if n > 50 { a.drain(0..n - 50); }
    }
    let to = targets(d);
    for id in &to { let _ = d.send_body(id, "kdeconnect.notification", b.clone()); }
    if !to.is_empty() { d.emit("notification-sent", json!({"title": b["title"], "phones": to.len()})); }
}

/// Closed here (by the person, the app, or timing out): gone from the phones too.
pub fn closed(d: &Daemon, sid: u32) {
    let had = { let mut a = crate::lock(&ACTIVE); let n = a.len(); a.retain(|(x, _)| *x != sid); n != a.len() };
    if !had { return; }
    for id in targets(d) { let _ = d.send_body(&id, "kdeconnect.notification", json!({"id": sid.to_string(), "isCancel": true})); }
}

/// The phone dismissed one of this PC's notifications, or wants the ones showing.
pub fn phone_request(d: &Arc<Daemon>, id: &str, p: &Packet) {
    if !d.paired_dev(id).map(|x| x.send_notifications).unwrap_or(false) { return; }
    if let Some(c) = p.str("cancel") {
        let sid = c.parse::<u32>().ok().filter(|s| crate::lock(&ACTIVE).iter().any(|(x, _)| x == s));
        if let Some(sid) = sid { std::thread::spawn(move || crate::desktop::close_notification(sid)); }
    }
    if p.bool("request") {
        let all: Vec<Value> = crate::lock(&ACTIVE).iter().map(|(_, b)| b.clone()).collect();
        for b in all { let _ = d.send_body(id, "kdeconnect.notification", b); }
    }
}

pub fn seen() -> Vec<String> { crate::lock(&SEEN).clone() }

fn saw(app: &str) {
    if app.is_empty() || app == "Hearth Connect" { return; }
    let mut s = crate::lock(&SEEN);
    if !s.iter().any(|x| x == app) { s.push(app.to_string()); if s.len() > 60 { s.remove(0); } }
}

/// Watches the session bus for as long as the service runs (started once from main.rs). A watch that ends (the
/// notification service restarted, the bus went away) starts again a few seconds later.
pub fn start(d: Arc<Daemon>) {
    #[cfg(target_os = "linux")]
    std::thread::spawn(move || {
        let stop = d.stop_tx.subscribe();
        loop {
            if *stop.borrow() { return; }
            // Ended (the notification service restarted): again in a moment. Couldn't watch at all: once a minute.
            let wait = match monitor(&d) { Ok(()) => 2, Err(e) => { eprintln!("notifications to the phone: {e}"); 60 } };
            std::thread::sleep(std::time::Duration::from_secs(wait));
        }
    });
    #[cfg(not(target_os = "linux"))]
    let _ = d;
}

#[cfg(target_os = "linux")]
fn monitor(d: &Arc<Daemon>) -> Result<(), String> {
    use std::collections::HashMap;
    use zbus::message::Type;
    use zbus::zvariant::{OwnedValue, Value as V};
    let c = zbus::blocking::Connection::session().map_err(|e| e.to_string())?;
    let owner: Option<String> = c.call_method(Some("org.freedesktop.DBus"), "/org/freedesktop/DBus", Some("org.freedesktop.DBus"), "GetNameOwner", &("org.freedesktop.Notifications",))
        .ok().and_then(|r| r.body().deserialize::<String>().ok());
    let mut rules: Vec<String> = vec![
        "type='method_call',interface='org.freedesktop.Notifications',member='Notify'".into(),
        "type='signal',interface='org.freedesktop.Notifications',member='NotificationClosed'".into(),
        "type='signal',sender='org.freedesktop.DBus',interface='org.freedesktop.DBus',member='NameOwnerChanged',arg0='org.freedesktop.Notifications'".into(),
    ];
    if let Some(o) = &owner { rules.push(format!("type='method_return',sender='{o}'")); }
    c.call_method(Some("org.freedesktop.DBus"), "/org/freedesktop/DBus", Some("org.freedesktop.DBus.Monitoring"), "BecomeMonitor", &(rules, 0u32))
        .map_err(|e| format!("can't watch notifications: {e}"))?;
    // Notify calls waiting for the service's answer: (caller, serial) → the packet's body (its id comes with the answer).
    let mut pending: HashMap<(String, u32), Value> = HashMap::new();
    let stop = d.stop_tx.subscribe();
    for m in zbus::blocking::MessageIterator::from(&c) {
        if *stop.borrow() { return Ok(()); }
        let Ok(m) = m else { continue };
        let h = m.header();
        let member = h.member().map(|x| x.as_str().to_string()).unwrap_or_default();
        match h.message_type() {
            Type::MethodCall if member == "Notify" => {
                type Args = (String, u32, String, String, String, Vec<String>, HashMap<String, OwnedValue>, i32);
                let Ok((app, _replaces, _icon, summary, text, _actions, hints, timeout)) = m.body().deserialize::<Args>() else { continue };
                let hint_bool = |k: &str| hints.get(k).map(|v| matches!(&**v, V::Bool(true))).unwrap_or(false);
                let urgency = hints.get("urgency").map(|v| match &**v { V::U8(u) => *u, V::I32(i) => *i as u8, V::U32(u) => *u as u8, _ => 1 }).unwrap_or(1);
                let entry = hints.get("desktop-entry").and_then(|v| if let V::Str(s) = &**v { Some(s.as_str().to_string()) } else { None }).unwrap_or_default();
                if entry == "hearth-connect" { continue; }
                saw(&app);
                let p = prefs(&d.home);
                if !wanted(&p, &app, urgency, hint_bool("transient"), timeout == 0 || hint_bool("resident")) { continue; }
                if targets(d).is_empty() { continue; }
                let caller = h.sender().map(|s| s.as_str().to_string()).unwrap_or_default();
                let serial = m.primary_header().serial_num().get();
                pending.insert((caller, serial), body(&p, 0, &app, &summary, &text));
                if pending.len() > 100 { pending.clear(); }
            }
            Type::MethodReturn => {
                let (Some(rs), Some(dest)) = (h.reply_serial(), h.destination().map(|x| x.as_str().to_string())) else { continue };
                if let Some(mut b) = pending.remove(&(dest, rs.get())) {
                    let Ok(sid) = m.body().deserialize::<u32>() else { continue };
                    b["id"] = json!(sid.to_string());
                    forward(d, sid, b);
                }
            }
            Type::Signal if member == "NotificationClosed" => {
                let from_owner = owner.as_deref().map(|o| h.sender().map(|s| s.as_str() == o).unwrap_or(false)).unwrap_or(false);
                if !from_owner { continue; }
                if let Ok((sid, _why)) = m.body().deserialize::<(u32, u32)>() { closed(d, sid); }
            }
            // The notification service came, went or restarted: watch again with its new name.
            Type::Signal if member == "NameOwnerChanged" => return Ok(()),
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn which_notifications_go() {
        let p = defaults();
        assert!(wanted(&p, "Thunderbird", 1, false, false));
        assert!(!wanted(&p, "Hearth Connect", 1, false, false), "the phone's own notifications never go back");
        assert!(!wanted(&p, "Volume", 1, true, false), "transient pop-ups (volume, brightness) stay here");
        let strict = json!({"body": true, "persistentOnly": true, "urgency": 2, "blocked": ["steam"]});
        assert!(!wanted(&strict, "Thunderbird", 1, false, true), "not urgent enough");
        assert!(!wanted(&strict, "Thunderbird", 2, false, false), "goes away by itself");
        assert!(wanted(&strict, "Thunderbird", 2, false, true));
        assert!(!wanted(&strict, "Steam", 2, false, true), "an app left out (any case)");
    }

    #[test]
    fn packets_for_the_phone() {
        let b = body(&defaults(), 7, "Mail", "New <b>mail</b>", "From Ada &amp; Bob");
        assert_eq!(b["id"], "7");
        assert_eq!(b["title"], "New mail");
        assert_eq!(b["text"], "From Ada & Bob");
        assert_eq!(b["ticker"], "New mail: From Ada & Bob");
        assert_eq!(b["isClearable"], true);
        let quiet = body(&json!({"body": false}), 8, "Mail", "New mail", "secret");
        assert_eq!(quiet["text"], "");
        assert_eq!(quiet["ticker"], "New mail");
    }

    #[test]
    fn choices_saved_without_touching_the_rest() {
        let dir = std::env::temp_dir().join(format!("connect-n-{}", crate::random_hex(4)));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("settings.json"), r#"{"enabled": true, "name": "Hearth PC"}"#).unwrap();
        assert_eq!(prefs(&dir), defaults());
        let n = set_prefs(&dir, &json!({"body": false, "urgency": 9, "blocked": ["Steam"], "evil": 1})).unwrap();
        assert_eq!(n["body"], false);
        assert_eq!(n["urgency"], 0, "out of range: kept as it was");
        let all: Value = serde_json::from_str(&std::fs::read_to_string(dir.join("settings.json")).unwrap()).unwrap();
        assert_eq!(all["enabled"], true);
        assert_eq!(all["name"], "Hearth PC");
        assert_eq!(all["notifyOut"]["blocked"], json!(["Steam"]));
        assert!(all["notifyOut"].get("evil").is_none());
        let _ = std::fs::remove_dir_all(dir);
    }
}
