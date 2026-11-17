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
//!
//! What they remember about a phone lives here while the service runs (the phone has all of it; nothing is saved).
//! The person's choices for a phone (pause media in calls, run commands) are in paired.json with the others.

pub mod contacts;
pub mod notify_bus;
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
        "kdeconnect.systemvolume.request" => systemvolume::packet(d, id, p),
        "kdeconnect.sftp" => sftp::packet(d, id, from, p),
        "kdeconnect.contacts.response_uids_timestamps" | "kdeconnect.contacts.response_vcards" => contacts::packet(d, id, p),
        "kdeconnect.connectivity_report" => {
            let v = p.body.get("signalStrengths").cloned().unwrap_or(json!({}));
            with(id, |s| s.signal = Some(v));
            d.changed();
        }
        _ => return false,
    }
    true
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
        "photo" => { let id = s("id")?; paired(&id)?; d.send_body(&id, "kdeconnect.photo.request", json!({})).map(|_| json!({})) }
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
        _ => Err(format!("unknown command {cmd}")),
    }
}
