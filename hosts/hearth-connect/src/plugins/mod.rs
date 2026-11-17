//! Hearth Connect's other features, one file each (KDE Connect calls them "plugins"). Each speaks KDE Connect's own
//! packets, so the stock KDE Connect app on Android (and iPhone, where Apple allows the feature) works with them:
//!
//! - runcommand.rs: commands you set up here that the phone starts with a tap
//! - telephony.rs: who's calling, Mute ringer, and what's playing here paused during a call
//! - sms.rs: the phone's text messages: read the conversations here and send a message from here
//! - presenter.rs: the phone as a presentation remote (its pointer; next/previous come as keys)
//! - systemvolume.rs: this PC's volume and mute from the phone
//! - sftp.rs: browse the phone's files from here (the phone's SFTP server, mounted with sshfs)
//! - contacts.rs: the phone's contacts, for names in Messages and calls
//! - connectivity: the phone's signal and network (kdeconnect.connectivity_report, below)
//! - photo: take a picture with the phone's camera (kdeconnect.photo.request; the picture comes back as a file)
//! - notify_bus.rs: the phone's notifications in Mantel with their buttons and a Reply box, and calls' Mute ringer
//! - lock.rs: lock this PC from the phone, and the phone sees whether it's locked (never unlock)
//! - sendnotify.rs: this PC's notifications on the phone (with the choices of what goes)
//! - remote.rs: this PC as the phone's keyboard and touchpad (the phone's Remote keyboard and mouse receiver)
//! - share notices: text and links from the phone with Copy / Open buttons (share_notice, below)
//! - stay awake: daemon.rs set_awake (no screen-off or sleep while a phone set to keep it awake is linked)
//!
//! What they remember about a phone lives here while the service runs (the phone has all of it; nothing is saved).
//! The person's choices for a phone (pause media in calls, run commands) are in paired.json with the others.

pub mod contacts;
pub mod lock;
pub mod notify_bus;
pub mod remote;
pub mod sendnotify;
pub mod presenter;
pub mod runcommand;
pub mod sftp;
pub mod sms;
pub mod systemvolume;
pub mod telephony;

use crate::daemon::{Daemon, PairedDev};
use crate::packet::Packet;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

/// What the plugins know about one phone right now.
#[derive(Default)]
pub struct DevState {
    pub sms: sms::State,
    pub call: telephony::State,
    pub sftp: sftp::State,
    pub contacts: contacts::State,
    /// The phone's network and signal (0-4), per SIM.
    pub signal: Option<serde_json::Value>,
    /// The "wants to pair" notification.
    pub pair_notif: u32,
    /// When this PC asked the phone to take a photo (Take a photo): that photo is kept without asking.
    pub photo_asked: Option<std::time::Instant>,
}

/// Did this PC ask for a photo in the last 5 minutes? (Once: the next photo needs asking again.)
pub fn photo_expected(id: &str) -> bool {
    with(id, |s| s.photo_asked.take().map(|t| t.elapsed() < std::time::Duration::from_secs(300)).unwrap_or(false))
}

fn all() -> &'static Mutex<HashMap<String, DevState>> {
    static S: OnceLock<Mutex<HashMap<String, DevState>>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Works on one phone's state. Never call the daemon (or anything that locks it) inside `f`.
pub fn with<T>(id: &str, f: impl FnOnce(&mut DevState) -> T) -> T {
    let mut g = crate::lock(all());
    f(g.entry(id.to_string()).or_default())
}

/// A packet from a paired phone that daemon.rs doesn't handle itself. True when one of these took it.
pub fn handle(d: &Arc<Daemon>, id: &str, from: &str, p: &Packet, pd: &PairedDev) -> bool {
    match p.kind.as_str() {
        "kdeconnect.runcommand.request" => runcommand::packet(d, id, from, p, pd),
        "kdeconnect.telephony" => telephony::packet(d, id, from, p, pd),
        "kdeconnect.sms.messages" => sms::packet(d, id, p),
        "kdeconnect.presenter" => presenter::packet(d, id, p, pd),
        "kdeconnect.systemvolume.request" => if pd.volume { systemvolume::packet(d, id, p) },
        "kdeconnect.sftp" => sftp::packet(d, id, from, p),
        "kdeconnect.contacts.response_uids_timestamps" | "kdeconnect.contacts.response_vcards" => contacts::packet(d, id, p),
        "kdeconnect.lock.request" => if pd.lock { lock::packet(d, id, from, p) } else if p.has("requestLocked") || p.has("setLocked") {
            // Off for this phone: it hears "not locked" and can't lock it.
            let _ = d.send_body(id, "kdeconnect.lock", json!({"isLocked": false}));
        },
        "kdeconnect.notification.request" => sendnotify::phone_request(d, id, p),
        "kdeconnect.mousepad.echo" => d.emit("remote-echo", json!({"device": id, "body": p.body})),
        "kdeconnect.connectivity_report" => {
            let v = p.body.get("signalStrengths").cloned().unwrap_or(json!({}));
            with(id, |s| s.signal = Some(v));
            d.changed();
        }
        _ => return false,
    }
    true
}

/// Text or a link the phone shared: a notification with Copy (and Open for a link). Nothing opens by itself: a link
/// opens only when you press Open (KDE Connect's desktop opens it at once; a phone shouldn't open pages here unasked).
pub fn share_notice(d: &Arc<Daemon>, _id: &str, from: &str, kind: &str, content: &str) {
    let (summary, icon) = if kind == "url" { (format!("{from} shared a link"), "applications-internet") } else { (format!("{from} shared text"), "edit-paste") };
    let shown: String = content.chars().take(200).collect();
    let mut actions = vec![("copy".to_string(), "Copy".to_string())];
    if kind == "url" { actions.insert(0, ("open".to_string(), "Open".to_string())); }
    let (d2, text, is_url) = (d.clone(), content.to_string(), kind == "url");
    let n = notify_bus::notify(&summary, &shown, icon, 0, false, &actions, None, Arc::new(move |ev| if let notify_bus::Event::Action(k) = ev {
        match k.as_str() {
            "copy" => { let t = text.clone(); std::thread::spawn(move || crate::desktop::clipboard_set(&t)); d2.emit("copied", json!({})); }
            "open" if is_url => {
                let t = text.clone();
                if t.starts_with("http://") || t.starts_with("https://") {
                    std::thread::spawn(move || { if let Ok(mut c) = crate::desktop::cmd("xdg-open").arg(&t).spawn() { let _ = c.wait(); } });
                }
            }
            "default" => notify_bus::open_page("phone"),
            _ => {}
        }
    }));
    if n.is_none() { d.notify_desktop(&summary, &shown, icon, false); }
}

/// "Pixel 8 wants to pair", with Accept and Decline (and the key to check); it goes once the request is answered.
pub fn pair_notice(d: &Arc<Daemon>, id: &str, who: &str, key: &str) {
    let (d2, dev) = (d.clone(), id.to_string());
    let n = notify_bus::notify(&format!("{who} wants to pair"), &format!("Check that the phone shows {key}, then accept here or in Settings › Phone."), "smartphone", 0, true,
        &[("accept".to_string(), "Accept".to_string()), ("reject".to_string(), "Decline".to_string()), ("default".to_string(), "Open".to_string())], None,
        Arc::new(move |ev| match ev {
            notify_bus::Event::Action(k) if k == "accept" => { let _ = d2.accept_pair(&dev); }
            notify_bus::Event::Action(k) if k == "reject" => { let _ = d2.reject_pair(&dev); }
            notify_bus::Event::Action(k) if k == "default" => notify_bus::open_page("phone"),
            _ => {}
        }));
    match n {
        Some(n) => { let old = with(id, |s| std::mem::replace(&mut s.pair_notif, n)); if old != 0 && old != n { notify_bus::close(old); } }
        None => d.notify_desktop(&format!("{who} wants to pair"), &format!("Accept in Settings › Phone. Check that the phone shows {key}."), "smartphone", true),
    }
}

pub fn pair_notice_done(id: &str) {
    let n = with(id, |s| std::mem::take(&mut s.pair_notif));
    if n != 0 { notify_bus::close(n); }
}

/// A paired phone just linked.
pub fn on_paired_link(d: &Daemon, id: &str) {
    runcommand::send_list(d, id);
    if d.peer_offers(id, "kdeconnect.sms.messages") { sms::request_threads(d, id); }
    if d.peer_offers(id, "kdeconnect.contacts.response_uids_timestamps") { contacts::request(d, id); }
    if d.peer_offers(id, "kdeconnect.connectivity_report") { let _ = d.send_body(id, "kdeconnect.connectivity_report.request", json!({})); }
}

/// The link to a phone dropped: a call can't end by itself any more, so the music comes back now.
pub fn link_lost(d: &Arc<Daemon>, id: &str) { telephony::end(d, id); }

/// A phone was unpaired: everything about it goes.
pub fn forget(d: &Arc<Daemon>, id: &str) {
    telephony::end(d, id);
    sftp::unmount(id);
    crate::lock(all()).remove(id);
}

/// For the state the window and Settings › Phone read.
pub fn device_state(id: &str) -> Value {
    let g = crate::lock(all());
    match g.get(id) {
        Some(s) => json!({"call": s.call.json(), "sms": {"threads": s.sms.threads.len()}, "sftp": s.sftp.json(), "contacts": s.contacts.people.len(), "signal": s.signal}),
        None => json!({"call": null, "sms": {"threads": 0}, "sftp": sftp::State::default().json(), "contacts": 0, "signal": null}),
    }
}

const COMMANDS: &[&str] = &[
    "commands", "command_add", "command_remove", "contacts", "photo", "sms_threads", "sms_thread", "sms_refresh", "sms_send", "mute_call", "browse", "unmount",
    "notify_out", "notify_out_set", "remote_key", "remote_text", "remote_mouse", "send", "lock_state", "copy_shared", "open_shared",
];

pub fn knows(cmd: &str) -> bool { COMMANDS.contains(&cmd) }

/// The window's (and `hearth-connect --call`'s) requests for these features.
pub async fn command(d: &Arc<Daemon>, cmd: &str, a: &Value) -> Result<Value, String> {
    let s = |k: &str| a.get(k).and_then(|v| v.as_str()).map(String::from).ok_or_else(|| format!("missing {k}"));
    let paired = |id: &str| if d.is_paired(id) { Ok(()) } else { Err("Pair with it first".to_string()) };
    match cmd {
        "commands" => Ok(json!(runcommand::list(&d.home))),
        "command_add" => runcommand::add(d, &s("name")?, &s("command")?),
        "command_remove" => runcommand::remove(d, &s("key")?),
        "contacts" => { let id = s("id")?; paired(&id)?; if a.get("refresh").and_then(|v| v.as_bool()).unwrap_or(false) { contacts::request(d, &id); } Ok(json!(contacts::list(&id))) }
        "photo" => {
            let id = s("id")?;
            paired(&id)?;
            d.send_body(&id, "kdeconnect.photo.request", json!({}))?;
            with(&id, |st| st.photo_asked = Some(std::time::Instant::now()));
            Ok(json!({}))
        }
        "sms_threads" => { let id = s("id")?; paired(&id)?; Ok(json!(sms::threads(&id))) }
        "sms_thread" => {
            let id = s("id")?;
            paired(&id)?;
            let t = a.get("thread").and_then(|v| v.as_i64().or_else(|| v.as_str().and_then(|x| x.parse().ok()))).ok_or("missing thread")?;
            let _ = sms::request_thread(d, &id, t);
            Ok(json!(sms::messages(&id, t)))
        }
        "sms_refresh" => { let id = s("id")?; paired(&id)?; sms::request_threads(d, &id); Ok(json!({})) }
        "sms_send" => {
            let id = s("id")?;
            paired(&id)?;
            let to: Vec<String> = match a.get("to") {
                Some(Value::Array(v)) => v.iter().filter_map(|x| x.as_str().map(String::from)).collect(),
                Some(Value::String(t)) => t.split([',', ';']).map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect(),
                _ => a.get("thread").and_then(|v| v.as_i64()).map(|t| sms::addresses_of(&id, t)).unwrap_or_default(),
            };
            sms::send(d, &id, to, &s("text")?)
        }
        "mute_call" => { let id = s("id")?; paired(&id)?; d.send_body(&id, "kdeconnect.telephony.request_mute", json!({})).map(|_| json!({})) }
        "browse" => { let id = s("id")?; paired(&id)?; sftp::browse(d, &id) }
        "unmount" => { let id = s("id")?; sftp::unmount(&id); d.changed(); Ok(json!({})) }
        "notify_out" => Ok(json!({"prefs": sendnotify::prefs(&d.home), "apps": sendnotify::seen()})),
        "notify_out_set" => { let r = sendnotify::set_prefs(&d.home, a.get("prefs").unwrap_or(&json!({})))?; d.changed(); Ok(r) }
        "remote_key" => { let id = s("id")?; paired(&id)?; remote::key(d, &id, a) }
        "remote_text" => { let id = s("id")?; paired(&id)?; remote::text(d, &id, &s("text")?) }
        "remote_mouse" => { let id = s("id")?; paired(&id)?; remote::mouse(d, &id, a) }
        // Settings › Phone's "Send text or a link": a link goes as a link (the phone offers to open it), anything else as text.
        "send" => {
            let id = s("id")?;
            paired(&id)?;
            let t = s("text")?.trim().to_string();
            if t.is_empty() { return Err("Type something to send".into()); }
            let link = !t.contains(char::is_whitespace) && (t.starts_with("http://") || t.starts_with("https://"));
            d.send_body(&id, "kdeconnect.share.request", if link { json!({"url": t}) } else { json!({"text": t}) })?;
            Ok(json!({"kind": if link { "url" } else { "text" }}))
        }
        "lock_state" => Ok(json!({"locked": tokio::task::spawn_blocking(lock::is_locked).await.ok().flatten()})),
        // A shared text or link from the list in Settings › Phone (only what a phone shared: d.texts).
        "copy_shared" | "open_shared" => {
            let at = a.get("at").and_then(|v| v.as_i64()).ok_or("missing at")?;
            let item = crate::lock(&d.texts).iter().find(|t| t["at"].as_i64() == Some(at)).cloned().ok_or("That isn't in the list any more")?;
            let content = item["url"].as_str().or_else(|| item["text"].as_str()).unwrap_or("").to_string();
            if cmd == "copy_shared" {
                let ok = tokio::task::spawn_blocking(move || crate::desktop::clipboard_set(&content)).await.unwrap_or(false);
                if ok { Ok(json!({})) } else { Err("The clipboard can't be reached here".into()) }
            } else {
                if !(content.starts_with("http://") || content.starts_with("https://")) { return Err("Only web links open".into()); }
                tokio::task::spawn_blocking(move || crate::desktop::cmd("xdg-open").arg(&content).spawn().map(|_| ()).map_err(|e| e.to_string())).await.map_err(|e| e.to_string())??;
                Ok(json!({}))
            }
        }
        _ => Err(format!("unknown command {cmd}")),
    }
}
