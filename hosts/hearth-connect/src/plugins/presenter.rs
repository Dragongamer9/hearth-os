//! The phone as a presentation remote (KDE Connect's "Presentation remote"): its next / previous slide buttons come as
//! keys (kdeconnect.mousepad.request with a special key, handled with the touchpad in input.rs), and its pointer as
//! kdeconnect.presenter: moving the phone moves this PC's pointer (a fraction of the screen each time). Uses the same
//! virtual mouse as the touchpad, and only while the phone's "Remote input" switch is on.

use crate::daemon::{Daemon, PairedDev};
use crate::packet::Packet;
use serde_json::json;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// How far a full sweep of the phone moves the pointer (pixels across and down).
const SPAN: (f64, f64) = (1920.0, 1080.0);

static LAST: Mutex<Option<Instant>> = Mutex::new(None);

pub fn packet(d: &Arc<Daemon>, id: &str, p: &Packet, pd: &PairedDev) {
    if !pd.input { return; }
    if p.bool("stop") { d.emit("presenter", json!({"device": id, "stop": true})); return; }
    let (dx, dy) = (p.float("dx").unwrap_or(0.0), p.float("dy").unwrap_or(0.0));
    if !dx.is_finite() || !dy.is_finite() || (dx == 0.0 && dy == 0.0) { return; }
    let r = crate::lock(&d.input).apply(&crate::input::Action::Move((dx * SPAN.0).clamp(-4000.0, 4000.0), (dy * SPAN.1).clamp(-4000.0, 4000.0)));
    // The phone sends these many times a second: the window hears about it at most twice a second.
    let tell = { let mut l = crate::lock(&LAST); let t = l.map(|t| t.elapsed() > Duration::from_millis(500)).unwrap_or(true); if t { *l = Some(Instant::now()); } t };
    if tell { d.emit("presenter", json!({"device": id, "moving": true, "ok": r.is_ok()})); }
}
