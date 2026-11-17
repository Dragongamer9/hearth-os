//! Calls (KDE Connect's "Telephony"): when the phone rings this PC says who's calling, with a Mute ringer button, and
//! pauses what's playing here (the phone's "Pause media during calls" switch, on unless you turn it off). When the call
//! ends the players Hearth paused play again: only those, so music you had paused yourself stays paused. Like KDE
//! Connect, it can wait until you answer ("Only once I answer") and mute this PC's sound too ("Mute this PC's sound
//! during calls", off unless you turn it on): unmuted again after the call, only if Hearth muted it.

use crate::daemon::{Daemon, PairedDev};
use crate::packet::Packet;
use serde_json::{json, Value};
use std::sync::Arc;

#[derive(Default)]
pub struct State {
    /// "" (no call), "ringing" or "talking".
    pub state: String,
    pub who: String,
    /// The players Hearth paused for this call (playerctl names).
    pub paused: Vec<String>,
    /// The "is calling" notification.
    pub notif: u32,
    /// Players paused (or asked to be) for this call already.
    pub acted: bool,
    /// Hearth muted this PC's sound for this call (and unmutes it after).
    pub muted: bool,
}

impl State {
    pub fn json(&self) -> Value { if self.state.is_empty() { Value::Null } else { json!({"state": self.state, "who": self.who}) } }
}

pub fn packet(d: &Arc<Daemon>, id: &str, from: &str, p: &Packet, pd: &PairedDev) {
    let event = p.str("event").unwrap_or("").to_string();
    let number = p.str("phoneNumber").unwrap_or("").to_string();
    let who = p.str("contactName").filter(|s| !s.is_empty()).map(String::from)
        .or_else(|| super::contacts::name_for(id, &number))
        .or_else(|| Some(number.clone()).filter(|s| !s.is_empty()))
        .unwrap_or_else(|| "Unknown number".to_string());
    if p.bool("isCancel") {
        end(d, id);
        d.emit("call", json!({"device": id, "state": "ended", "who": who}));
        d.changed();
        return;
    }
    match event.as_str() {
        "ringing" | "talking" => {
            let first = super::with(id, |s| { let f = s.call.state.is_empty(); s.call.state = event.clone(); s.call.who = who.clone(); f });
            if first && event == "ringing" {
                let (d2, id2) = (d.clone(), id.to_string());
                let n = super::notify_bus::notify(&format!("{who} is calling"), &format!("On {from}"), "smartphone", 0, true,
                    &[("mute".to_string(), "Mute ringer".to_string())], None,
                    Arc::new(move |ev| if let super::notify_bus::Event::Action(k) = ev {
                        if k == "mute" { let _ = d2.send_body(&id2, "kdeconnect.telephony.request_mute", json!({})); }
                    }));
                if let Some(n) = n { super::with(id, |s| s.call.notif = n); }
            }
            // Pause (and mute) when it rings, or only once it's answered.
            let now = event == "talking" || !pd.pause_talking_only;
            let act = now && super::with(id, |s| !std::mem::replace(&mut s.call.acted, true));
            if act && pd.pause_on_call { pause(d, id); }
            if act && pd.mute_on_call { mute(d, id); }
            d.emit("call", json!({"device": id, "state": event, "who": who}));
        }
        "missedCall" => {
            d.notify_desktop(&format!("Missed call from {who}"), &format!("On {from}"), "smartphone", false);
            d.emit("call", json!({"device": id, "state": "missed", "who": who}));
        }
        _ => {}
    }
    d.changed();
}

/// Pauses every player that's playing, and remembers which (in a thread: playerctl can take a moment).
fn pause(d: &Arc<Daemon>, id: &str) {
    let (d, id) = (d.clone(), id.to_string());
    std::thread::spawn(move || {
        let mut paused = vec![];
        for (name, _) in crate::desktop::players() {
            let st = crate::desktop::cmd("playerctl").args(["-p", name.as_str(), "status"]).output().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default();
            if st == "Playing" && crate::desktop::cmd("playerctl").args(["-p", name.as_str(), "pause"]).status().map(|s| s.success()).unwrap_or(false) { paused.push(name); }
        }
        // The call may have ended while this ran: then they play again right away.
        let ended = super::with(&id, |s| if s.call.state.is_empty() { true } else { s.call.paused.extend(paused.iter().cloned()); false });
        if ended { resume(&paused); }
        if !paused.is_empty() { d.emit("media-paused", json!({"device": id, "players": paused})); }
    });
}

/// Mutes this PC's sound for the call, unless it's muted already (then it's left muted afterwards too).
fn mute(d: &Arc<Daemon>, id: &str) {
    let (d, id) = (d.clone(), id.to_string());
    std::thread::spawn(move || {
        let o = crate::desktop::cmd("wpctl").args(["get-volume", "@DEFAULT_AUDIO_SINK@"]).output().map(|o| String::from_utf8_lossy(&o.stdout).to_string()).unwrap_or_default();
        if super::systemvolume::parse(&o).map(|(_, m)| m).unwrap_or(true) { return; }
        if !crate::desktop::cmd("wpctl").args(["set-mute", "@DEFAULT_AUDIO_SINK@", "1"]).status().map(|s| s.success()).unwrap_or(false) { return; }
        let ended = super::with(&id, |s| if s.call.state.is_empty() { true } else { s.call.muted = true; false });
        if ended { unmute(); } else { d.emit("sound-muted", json!({"device": id})); }
    });
}

fn unmute() { let _ = crate::desktop::cmd("wpctl").args(["set-mute", "@DEFAULT_AUDIO_SINK@", "0"]).status(); }

fn resume(players: &[String]) {
    for name in players { let _ = crate::desktop::cmd("playerctl").args(["-p", name.as_str(), "play"]).status(); }
}

/// The call is over (or the phone went away): what Hearth paused plays again, the notification goes.
pub fn end(d: &Arc<Daemon>, id: &str) {
    let (paused, notif, was, muted) = super::with(id, |s| {
        let was = !s.call.state.is_empty();
        s.call.state.clear();
        s.call.who.clear();
        s.call.acted = false;
        (std::mem::take(&mut s.call.paused), std::mem::take(&mut s.call.notif), was, std::mem::take(&mut s.call.muted))
    });
    if muted { std::thread::spawn(unmute); }
    if notif != 0 { super::notify_bus::close(notif); }
    if !paused.is_empty() {
        let (d, id, p2) = (d.clone(), id.to_string(), paused.clone());
        std::thread::spawn(move || { resume(&p2); d.emit("media-resumed", json!({"device": id, "players": p2})); });
    }
    let _ = was;
}
