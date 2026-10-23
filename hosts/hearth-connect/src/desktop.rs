//! The PC side of the phone's features, on Linux: desktop notifications (org.freedesktop.Notifications: Mantel shows
//! them in Desktop Mode, and the window lists them in both modes), the clipboard (wl-clipboard in Desktop Mode,
//! xclip if present in Game Mode), this PC's media players (playerctl, MPRIS), its battery, and a sound.
//!
//! The service starts before either mode and keeps running across switches, so the display it should use is read
//! from systemd's user environment each time (Hearth's session keeps WAYLAND_DISPLAY/DISPLAY there).

use serde_json::{json, Value};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

static ENV: Mutex<Option<(Instant, Vec<(String, String)>)>> = Mutex::new(None);

/// The session's display variables right now (WAYLAND_DISPLAY, DISPLAY, XAUTHORITY), cached for a few seconds.
pub fn session_env() -> Vec<(String, String)> {
    let mut g = crate::lock(&ENV);
    if let Some((t, v)) = g.as_ref() { if t.elapsed() < Duration::from_secs(5) { return v.clone(); } }
    let mut v: Vec<(String, String)> = vec![];
    if let Ok(o) = Command::new("systemctl").args(["--user", "show-environment"]).stderr(Stdio::null()).output() {
        for line in String::from_utf8_lossy(&o.stdout).lines() {
            if let Some((k, val)) = line.split_once('=') {
                if matches!(k, "WAYLAND_DISPLAY" | "DISPLAY" | "XAUTHORITY" | "XDG_RUNTIME_DIR") { v.push((k.to_string(), val.to_string())); }
            }
        }
    }
    // Started from a terminal or the window instead: its own environment.
    for k in ["WAYLAND_DISPLAY", "DISPLAY", "XAUTHORITY", "XDG_RUNTIME_DIR"] {
        if !v.iter().any(|(x, _)| x == k) { if let Ok(val) = std::env::var(k) { v.push((k.into(), val)); } }
    }
    *g = Some((Instant::now(), v.clone()));
    v
}

fn env_of(k: &str) -> Option<String> { session_env().into_iter().find(|(x, _)| x == k).map(|(_, v)| v).filter(|v| !v.is_empty()) }

fn has(prog: &str) -> bool {
    std::env::var_os("PATH").map(|p| std::env::split_paths(&p).any(|d| d.join(prog).is_file())).unwrap_or(false)
}

fn cmd(prog: &str) -> Command {
    let mut c = Command::new(prog);
    c.env_remove("WAYLAND_DISPLAY").env_remove("DISPLAY");
    for (k, v) in session_env() { if !v.is_empty() { c.env(k, v); } }
    c.stdin(Stdio::null()).stderr(Stdio::null());
    c
}

// ------------------------------------------------------------------ notifications

/// Shows a desktop notification; its id (to replace or close it later), if a notification service answered.
pub fn notify(summary: &str, body: &str, icon: &str, replaces: u32, urgent: bool) -> Option<u32> {
    #[cfg(target_os = "linux")]
    {
        use std::collections::HashMap;
        use zbus::zvariant::Value as V;
        let c = zbus::blocking::Connection::session().ok()?;
        let mut hints: HashMap<&str, V> = HashMap::new();
        hints.insert("desktop-entry", V::from("hearth-connect"));
        if urgent { hints.insert("urgency", V::from(2u8)); }
        let actions: Vec<&str> = vec![];
        let r = c.call_method(Some("org.freedesktop.Notifications"), "/org/freedesktop/Notifications", Some("org.freedesktop.Notifications"), "Notify",
            &("Hearth Connect", replaces, icon, summary, body, actions, hints, -1i32)).ok()?;
        r.body().deserialize::<u32>().ok()
    }
    #[cfg(not(target_os = "linux"))]
    { let _ = (summary, body, icon, replaces, urgent); None }
}

pub fn close_notification(id: u32) {
    #[cfg(target_os = "linux")]
    if let Ok(c) = zbus::blocking::Connection::session() {
        let _ = c.call_method(Some("org.freedesktop.Notifications"), "/org/freedesktop/Notifications", Some("org.freedesktop.Notifications"), "CloseNotification", &(id,));
    }
    #[cfg(not(target_os = "linux"))]
    let _ = id;
}

// ------------------------------------------------------------------ clipboard

/// What's on the clipboard (text), if it can be read here.
pub fn clipboard_get() -> Option<String> {
    let out = if env_of("WAYLAND_DISPLAY").is_some() && has("wl-paste") {
        cmd("wl-paste").args(["--no-newline", "--type", "text"]).output().ok()?
    } else if env_of("DISPLAY").is_some() && has("xclip") {
        cmd("xclip").args(["-selection", "clipboard", "-o"]).output().ok()?
    } else { return None };
    if !out.status.success() { return None; }
    String::from_utf8(out.stdout).ok()
}

pub fn clipboard_set(text: &str) -> bool {
    use std::io::Write;
    let child = if env_of("WAYLAND_DISPLAY").is_some() && has("wl-copy") {
        cmd("wl-copy").stdin(Stdio::piped()).stdout(Stdio::null()).spawn()
    } else if env_of("DISPLAY").is_some() && has("xclip") {
        cmd("xclip").args(["-selection", "clipboard", "-i"]).stdin(Stdio::piped()).stdout(Stdio::null()).spawn()
    } else { return false };
    let Ok(mut child) = child else { return false };
    if let Some(mut i) = child.stdin.take() { let _ = i.write_all(text.as_bytes()); }
    // wl-copy stays around serving the clipboard; don't wait for it.
    std::thread::spawn(move || { let _ = child.wait(); });
    true
}

/// Types text the US layout can't (accents, other scripts) in Desktop Mode.
pub fn type_text(text: &str) -> bool {
    if text.is_empty() { return true; }
    if env_of("WAYLAND_DISPLAY").is_some() && has("wtype") { return cmd("wtype").arg("--").arg(text).status().map(|s| s.success()).unwrap_or(false); }
    if env_of("DISPLAY").is_some() && has("xdotool") { return cmd("xdotool").args(["type", "--", text]).status().map(|s| s.success()).unwrap_or(false); }
    false
}

// ------------------------------------------------------------------ media players (MPRIS through playerctl)

/// This PC's media players: (playerctl name, name shown on the phone).
pub fn players() -> Vec<(String, String)> {
    if !has("playerctl") { return vec![]; }
    let Ok(o) = cmd("playerctl").arg("-l").output() else { return vec![] };
    let mut out: Vec<(String, String)> = vec![];
    for name in String::from_utf8_lossy(&o.stdout).lines().map(|l| l.trim()).filter(|l| !l.is_empty()) {
        let base = name.split('.').next().unwrap_or(name);
        let mut shown = base.chars().next().map(|c| c.to_uppercase().collect::<String>() + &base[c.len_utf8()..]).unwrap_or_default();
        if out.iter().any(|(_, s)| *s == shown) { shown = format!("{shown} ({name})"); }
        out.push((name.to_string(), shown));
    }
    out
}

/// What a player is playing, as KDE Connect's mpris packet body.
pub fn player_status(name: &str, shown: &str) -> Value {
    let fmt = "{{status}}\u{1f}{{xesam:title}}\u{1f}{{xesam:artist}}\u{1f}{{xesam:album}}\u{1f}{{position}}\u{1f}{{mpris:length}}\u{1f}{{volume}}";
    let out = cmd("playerctl").args(["-p", name, "metadata", "--format", fmt]).output().ok().map(|o| String::from_utf8_lossy(&o.stdout).trim_end().to_string()).unwrap_or_default();
    let f: Vec<&str> = out.split('\u{1f}').collect();
    let get = |i: usize| f.get(i).copied().unwrap_or("");
    let us = |s: &str| s.parse::<f64>().unwrap_or(0.0);
    let playing = get(0) == "Playing";
    json!({
        "player": shown, "title": get(1), "artist": get(2), "album": get(3), "nowPlaying": get(1),
        "isPlaying": playing, "pos": (us(get(4)) / 1000.0) as i64, "length": (us(get(5)) / 1000.0) as i64,
        "volume": (us(get(6)) * 100.0).round() as i64, "canPlay": true, "canPause": true, "canGoNext": true, "canGoPrevious": true, "canSeek": true,
    })
}

/// The phone's media buttons for a player: "PlayPause", "Next", setVolume 0-100, Seek (microseconds)...
pub fn player_control(name: &str, body: &Value) -> bool {
    let mut args: Vec<String> = vec!["-p".into(), name.into()];
    if let Some(a) = body.get("action").and_then(|v| v.as_str()) {
        let c = match a { "PlayPause" => "play-pause", "Play" => "play", "Pause" => "pause", "Stop" => "stop", "Next" => "next", "Previous" => "previous", _ => return false };
        args.push(c.into());
    } else if let Some(v) = body.get("setVolume").and_then(|v| v.as_i64()) {
        args.extend(["volume".into(), format!("{:.2}", (v.clamp(0, 100) as f64) / 100.0)]);
    } else if let Some(us) = body.get("Seek").and_then(|v| v.as_i64()) {
        let s = us as f64 / 1_000_000.0;
        args.extend(["position".into(), if s >= 0.0 { format!("{s:.1}+") } else { format!("{:.1}-", -s) }]);
    } else if let Some(ms) = body.get("SetPosition").and_then(|v| v.as_i64()) {
        args.extend(["position".into(), format!("{:.1}", ms as f64 / 1000.0)]);
    } else { return false; }
    cmd("playerctl").args(&args).status().map(|s| s.success()).unwrap_or(false)
}

// ------------------------------------------------------------------ battery and sound

/// This PC's battery: (percent, charging). None on a desktop. HEARTH_CONNECT_FAKE_BATTERY=57 for tests.
pub fn battery() -> Option<(i64, bool)> {
    if let Ok(v) = std::env::var("HEARTH_CONNECT_FAKE_BATTERY") { return v.parse().ok().map(|p| (p, false)); }
    for e in std::fs::read_dir("/sys/class/power_supply").ok()?.flatten() {
        let p = e.path();
        if std::fs::read_to_string(p.join("type")).map(|t| t.trim() == "Battery").unwrap_or(false) {
            let cap = std::fs::read_to_string(p.join("capacity")).ok()?.trim().parse().ok()?;
            let st = std::fs::read_to_string(p.join("status")).unwrap_or_default();
            return Some((cap, matches!(st.trim(), "Charging" | "Full")));
        }
    }
    None
}

/// "Find my PC": a ringing sound a few times (and the notification the caller shows).
pub fn ring() {
    std::thread::spawn(|| {
        let sounds = ["/usr/share/sounds/freedesktop/stereo/phone-incoming-call.oga", "/usr/share/sounds/freedesktop/stereo/alarm-clock-elapsed.oga", "/usr/share/sounds/freedesktop/stereo/bell.oga"];
        let Some(s) = sounds.iter().find(|p| std::path::Path::new(p).exists()) else { return };
        let player = ["pw-play", "paplay", "ogg123"].into_iter().find(|p| has(p));
        let Some(player) = player else { return };
        for _ in 0..4 { let _ = Command::new(player).arg(s).stdout(Stdio::null()).stderr(Stdio::null()).status(); }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fake_battery_for_tests() {
        std::env::set_var("HEARTH_CONNECT_FAKE_BATTERY", "57");
        assert_eq!(battery(), Some((57, false)));
        std::env::remove_var("HEARTH_CONNECT_FAKE_BATTERY");
    }
}
