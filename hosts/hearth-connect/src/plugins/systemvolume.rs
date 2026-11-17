//! This PC's volume from the phone (KDE Connect's "System volume"): the phone's slider and mute button for the sound
//! output in use, through PipeWire's wpctl (what Hearth OS uses).

use crate::daemon::Daemon;
use crate::packet::Packet;
use serde_json::json;
use std::sync::Arc;

const SINK: &str = "@DEFAULT_AUDIO_SINK@";

/// wpctl's "Volume: 0.40 [MUTED]" as (percent, muted).
pub fn parse(out: &str) -> Option<(i64, bool)> {
    let rest = out.trim().strip_prefix("Volume:")?.trim();
    let v: f64 = rest.split_whitespace().next()?.parse().ok()?;
    Some(((v * 100.0).round() as i64, rest.contains("[MUTED]")))
}

fn read() -> Option<(i64, bool)> {
    let o = crate::desktop::cmd("wpctl").args(["get-volume", SINK]).output().ok()?;
    parse(&String::from_utf8_lossy(&o.stdout))
}

pub fn packet(d: &Arc<Daemon>, id: &str, p: &Packet) {
    let (d, id, p) = (d.clone(), id.to_string(), p.clone());
    std::thread::spawn(move || {
        if let Some(v) = p.int("volume") {
            let _ = crate::desktop::cmd("wpctl").args(["set-volume", SINK, &format!("{:.2}", v.clamp(0, 100) as f64 / 100.0)]).status();
        }
        if p.has("muted") {
            let _ = crate::desktop::cmd("wpctl").args(["set-mute", SINK, if p.bool("muted") { "1" } else { "0" }]).status();
        }
        if !(p.bool("requestSinks") || p.has("volume") || p.has("muted")) { return; }
        let Some((v, m)) = read() else { return };
        let _ = d.send_body(&id, "kdeconnect.systemvolume", json!({"sinkList": [{
            "name": SINK, "description": "This PC's sound", "muted": m, "volume": v, "maxVolume": 100, "enabled": true,
        }]}));
        d.emit("volume", json!({"device": id, "volume": v, "muted": m}));
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn reads_wpctl() {
        assert_eq!(super::parse("Volume: 0.40\n"), Some((40, false)));
        assert_eq!(super::parse("Volume: 1.00 [MUTED]"), Some((100, true)));
        assert_eq!(super::parse("nothing"), None);
    }
}
