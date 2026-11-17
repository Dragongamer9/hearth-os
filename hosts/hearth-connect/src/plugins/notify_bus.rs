//! Desktop notifications with buttons and a Reply box (org.freedesktop.Notifications on the session bus: Mantel shows
//! them in Desktop Mode). The phone's notifications come with the phone's own buttons ("Mark as read"...) and, for
//! apps that take replies (messages), a Reply box: Mantel's inline reply (the "inline-reply" action and its
//! NotificationReplied signal, as KDE Plasma does), or, where the notification service has no inline reply, a Reply
//! button that opens Settings › Phone at the notification. Clicking the notification opens Settings › Phone;
//! dismissing it (×) dismisses it on the phone too, when the phone allows.
//!
//! One connection to the session bus for all of them, so the service's answers (which may be sent to that
//! connection only) reach the thread that listens.

use crate::daemon::Daemon;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

pub enum Event {
    /// A button: its key ("default" for a click on the notification itself).
    Action(String),
    /// Text typed in the Reply box.
    Reply(String),
    /// Closed, with the reason (1 expired, 2 dismissed by the person, 3 closed by Hearth).
    Closed(u32),
}

pub type Callback = Arc<dyn Fn(Event) + Send + Sync>;

fn callbacks() -> &'static Mutex<HashMap<u32, Callback>> {
    static C: OnceLock<Mutex<HashMap<u32, Callback>>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(HashMap::new()))
}

fn fire(id: u32, ev: Event) {
    let cb = {
        let mut g = crate::lock(callbacks());
        if matches!(ev, Event::Closed(_)) { g.remove(&id) } else { g.get(&id).cloned() }
    };
    if let Some(cb) = cb { cb(ev); }
}

#[cfg(target_os = "linux")]
fn conn() -> Option<&'static zbus::blocking::Connection> {
    static CONN: OnceLock<Option<zbus::blocking::Connection>> = OnceLock::new();
    CONN.get_or_init(|| {
        let c = zbus::blocking::Connection::session().ok()?;
        listen(c.clone());
        Some(c)
    }).as_ref()
}

/// The notification service's signals: buttons pressed, replies typed, notifications closed.
#[cfg(target_os = "linux")]
fn listen(c: zbus::blocking::Connection) {
    let (tx, rx) = std::sync::mpsc::channel::<()>();
    std::thread::spawn(move || {
        let rule = zbus::MatchRule::builder().msg_type(zbus::message::Type::Signal).interface("org.freedesktop.Notifications").map(|b| b.build());
        let it = match rule { Ok(r) => zbus::blocking::MessageIterator::for_match_rule(r, &c, Some(256)), Err(e) => Err(e) };
        let _ = tx.send(());
        let Ok(it) = it else { return };
        for m in it {
            let Ok(m) = m else { continue };
            let member = m.header().member().map(|x| x.as_str().to_string()).unwrap_or_default();
            match member.as_str() {
                "ActionInvoked" => if let Ok((id, key)) = m.body().deserialize::<(u32, String)>() { fire(id, Event::Action(key)); },
                "NotificationReplied" => if let Ok((id, text)) = m.body().deserialize::<(u32, String)>() { fire(id, Event::Reply(text)); },
                "NotificationClosed" => if let Ok((id, why)) = m.body().deserialize::<(u32, u32)>() { fire(id, Event::Closed(why)); },
                _ => {}
            }
        }
    });
    // Listening before the first notification goes out (a quick answer isn't missed).
    let _ = rx.recv_timeout(std::time::Duration::from_secs(2));
}

/// Shows a notification with buttons ([(key, label)]) and, with `reply` (its placeholder text), a Reply box. `on`
/// hears what the person does with it. Its id, if a notification service answered.
pub fn notify(summary: &str, body: &str, icon: &str, replaces: u32, urgent: bool, actions: &[(String, String)], reply: Option<&str>, on: Callback) -> Option<u32> {
    #[cfg(target_os = "linux")]
    {
        use zbus::zvariant::Value as V;
        let c = conn()?;
        let mut hints: HashMap<&str, V> = HashMap::new();
        hints.insert("desktop-entry", V::from("hearth-connect"));
        if urgent { hints.insert("urgency", V::from(2u8)); }
        let mut acts: Vec<&str> = vec![];
        for (k, l) in actions { acts.push(k); acts.push(l); }
        if let Some(r) = reply {
            acts.push("inline-reply");
            acts.push("Reply");
            hints.insert("x-kde-reply-placeholder-text", V::from(r));
        }
        let r = c.call_method(Some("org.freedesktop.Notifications"), "/org/freedesktop/Notifications", Some("org.freedesktop.Notifications"), "Notify",
            &("Hearth Connect", replaces, icon, summary, body, acts, hints, -1i32)).ok()?;
        let id = r.body().deserialize::<u32>().ok()?;
        crate::lock(callbacks()).insert(id, on);
        Some(id)
    }
    #[cfg(not(target_os = "linux"))]
    { let _ = (summary, body, icon, replaces, urgent, actions, reply, on); None }
}

pub fn close(id: u32) {
    crate::lock(callbacks()).remove(&id);
    crate::desktop::close_notification(id);
}

/// Opens Settings › Phone at its notifications (Mantel's settings, or the Hearth Connect window where there's none).
pub fn open_settings() {
    let ok = crate::desktop::cmd("hearth-settings").arg("phone:notifications").spawn().map(|mut c| { std::thread::spawn(move || { let _ = c.wait(); }); }).is_ok();
    if !ok { let _ = crate::desktop::cmd("hearth-connect").spawn(); }
}

/// A notification from the phone (daemon.rs notification_in), with its buttons and Reply box.
pub fn phone_notification(d: &Arc<Daemon>, id: &str, entry: &Value, summary: &str, body: &str, replaces: u32) -> Option<u32> {
    let key = entry["id"].as_str().unwrap_or("").to_string();
    let reply_id = entry["replyId"].as_str().filter(|s| !s.is_empty()).map(String::from);
    let dismissable = entry["dismissable"].as_bool().unwrap_or(true);
    let mut actions: Vec<(String, String)> = vec![("default".into(), "Open".into())];
    for a in entry["actions"].as_array().into_iter().flatten().filter_map(|a| a.as_str()) { actions.push((format!("phone:{a}"), a.to_string())); }
    let wants_reply = reply_id.is_some();
    let (d2, dev) = (d.clone(), id.to_string());
    let on: Callback = Arc::new(move |ev| match ev {
        Event::Action(k) if k == "default" || k == "inline-reply" => open_settings(),
        Event::Action(k) => if let Some(a) = k.strip_prefix("phone:") {
            let _ = d2.send_body(&dev, "kdeconnect.notification.action", json!({"key": key, "action": a}));
        },
        Event::Reply(text) => if let Some(r) = &reply_id {
            if d2.send_body(&dev, "kdeconnect.notification.reply", json!({"requestReplyId": r, "message": text})).is_ok() {
                d2.emit("replied", json!({"device": dev, "notification": key}));
            }
        },
        Event::Closed(2) => if dismissable { let _ = d2.send_body(&dev, "kdeconnect.notification.request", json!({"cancel": key})); },
        Event::Closed(_) => {}
    });
    notify(summary, body, "smartphone", replaces, false, &actions, if wants_reply { Some("Reply…") } else { None }, on)
}
