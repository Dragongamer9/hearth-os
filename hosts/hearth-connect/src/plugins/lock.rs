//! Lock this PC from the phone (KDE Connect's "Lock"): the phone's lock button and whether this PC is locked now.
//!   phone → PC  kdeconnect.lock.request {"requestLocked": true} or {"setLocked": true}
//!   PC → phone  kdeconnect.lock {"isLocked": bool}, when asked and whenever it changes
//! Locking goes the way every app's "lock now" does: KWin's locker (org.freedesktop.ScreenSaver.Lock) on KWin, else
//! the session's lock through logind (`loginctl lock-session`, which Mantel's lock screen answers on labwc). Whether
//! it's locked: KWin's locker (GetActive), else logind's LockedHint (Mantel sets it). A phone can't unlock the PC:
//! that takes your password here (KDE Connect's desktop allows it; Hearth doesn't, on purpose). Only while the
//! phone's "Lock this PC" switch is on (on unless you turn it off).

use crate::daemon::Daemon;
use crate::packet::Packet;
use serde_json::json;
use std::sync::{Arc, Mutex};

/// The last state each phone was told (only changes go out).
static TOLD: Mutex<Option<bool>> = Mutex::new(None);

/// logind's session for this user (the one with the display), for `loginctl`.
fn session_id() -> Option<String> {
    #[cfg(target_os = "linux")]
    let uid = unsafe { libc::getuid() }.to_string();
    #[cfg(not(target_os = "linux"))]
    let uid = String::from("0");
    let o = crate::desktop::cmd("loginctl").args(["show-user", uid.as_str(), "-p", "Display", "--value"]).output().ok()?;
    let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
    if s.is_empty() || s.contains(char::is_whitespace) { None } else { Some(s) }
}

#[cfg(target_os = "linux")]
fn screensaver(method: &str) -> Option<zbus::Message> {
    let c = zbus::blocking::Connection::session().ok()?;
    c.call_method(Some("org.freedesktop.ScreenSaver"), "/ScreenSaver", Some("org.freedesktop.ScreenSaver"), method, &()).ok()
}

/// Is this PC locked right now? None when nothing can tell.
pub fn is_locked() -> Option<bool> {
    #[cfg(target_os = "linux")]
    if let Some(m) = screensaver("GetActive") { if let Ok(b) = m.body().deserialize::<bool>() { return Some(b); } }
    let id = session_id()?;
    let o = crate::desktop::cmd("loginctl").args(["show-session", id.as_str(), "-p", "LockedHint", "--value"]).output().ok()?;
    match String::from_utf8_lossy(&o.stdout).trim() { "yes" => Some(true), "no" => Some(false), _ => None }
}

/// Locks this PC. True when something took the request.
pub fn lock_now() -> bool {
    #[cfg(target_os = "linux")]
    if screensaver("Lock").is_some() { return true; }
    match session_id() {
        Some(id) => crate::desktop::cmd("loginctl").args(["lock-session", id.as_str()]).status().map(|s| s.success()).unwrap_or(false),
        None => crate::desktop::cmd("loginctl").arg("lock-sessions").status().map(|s| s.success()).unwrap_or(false),
    }
}

fn tell(d: &Daemon, id: &str, locked: bool) {
    let _ = d.send_body(id, "kdeconnect.lock", json!({"isLocked": locked}));
}

pub fn packet(d: &Arc<Daemon>, id: &str, from: &str, p: &Packet) {
    let (d, id, from, p) = (d.clone(), id.to_string(), from.to_string(), p.clone());
    std::thread::spawn(move || {
        if p.has("setLocked") {
            if p.bool("setLocked") {
                let ok = lock_now();
                d.emit("locked", json!({"device": id, "from": from, "ok": ok}));
                // The lock screen takes a moment to say so.
                for _ in 0..10 { if is_locked() == Some(true) { break; } std::thread::sleep(std::time::Duration::from_millis(300)); }
            } else {
                d.emit("warning", json!({"device": id, "text": format!("{from} asked to unlock this PC. Phones can only lock it: unlock it here with your password.")}));
            }
        }
        let now = is_locked().unwrap_or(false);
        *crate::lock(&TOLD) = Some(now);
        tell(&d, &id, now);
    });
}

/// Every few seconds (daemon housekeeping): a change goes to the linked phones that may lock this PC.
pub fn watch(d: &Arc<Daemon>) {
    let targets: Vec<String> = d.linked_paired().into_iter().filter(|k| d.paired_dev(k).map(|p| p.lock).unwrap_or(false) && d.peer_offers(k, "kdeconnect.lock.request")).collect();
    if targets.is_empty() { return; }
    let d = d.clone();
    std::thread::spawn(move || {
        let Some(now) = is_locked() else { return };
        let changed = { let mut t = crate::lock(&TOLD); let c = *t != Some(now); *t = Some(now); c };
        if changed { for id in targets { tell(&d, &id, now); } }
    });
}
