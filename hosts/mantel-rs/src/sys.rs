//! The little things the bar shows, read from the system the cheap way: the clock (12 or 24 hour, Mantel's setting),
//! the battery (/sys/class/power_supply), Wi-Fi or wired (/sys/class/net, /proc/net/wireless), the volume (wpctl,
//! only every few ticks), and the dock's pinned apps (~/.config/nimbus/desk.json, written by the QML Mantel too:
//! read only here, never written, so the two Mantels always agree).

use serde_json::Value;
use std::path::PathBuf;

fn home() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default()
}

fn desktop_settings() -> Value {
    std::fs::read(hearth_ui::theme::desktop_json()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null)
}

/// "2:05 PM" or "14:05" (Settings › Date & time › 24-hour clock, desktop.json "clock24").
pub fn clock_text() -> String {
    let now = chrono::Local::now();
    if desktop_settings().get("clock24").and_then(Value::as_bool) == Some(true) {
        now.format("%H:%M").to_string()
    } else {
        now.format("%-I:%M %p").to_string()
    }
}

/// "Sat 22 Nov".
pub fn date_text() -> String {
    chrono::Local::now().format("%a %-d %b").to_string()
}

/// The dock's pinned apps, in order (.desktop ids).
pub fn pins() -> Vec<String> {
    let p = home().join(".config/nimbus/desk.json");
    let v: Value = std::fs::read(&p).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null);
    v.get("pins").and_then(Value::as_array).map(|a| a.iter().filter_map(|x| x.as_str().map(|s| s.trim_end_matches(".desktop").to_string())).collect()).unwrap_or_default()
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Status {
    /// Wi-Fi's signal 0-100, or Some(-1) on a cable; None: not connected.
    pub wifi: Option<i32>,
    /// (percent, charging); None: no battery.
    pub battery: Option<(u8, bool)>,
    /// (percent, muted); None: wpctl didn't answer (or this tick didn't ask).
    pub volume: Option<(u8, bool)>,
}

impl Status {
    /// Read everything; `volume` asks wpctl too (a process: only every few ticks).
    pub fn read(volume: bool) -> Status {
        Status { wifi: network(), battery: battery(), volume: if volume { volume_now() } else { None } }
    }
}

fn read(p: impl AsRef<std::path::Path>) -> String {
    std::fs::read_to_string(p).map(|s| s.trim().to_string()).unwrap_or_default()
}

fn battery() -> Option<(u8, bool)> {
    let rd = std::fs::read_dir("/sys/class/power_supply").ok()?;
    for e in rd.flatten() {
        let d = e.path();
        if read(d.join("type")) != "Battery" || read(d.join("scope")) == "Device" {
            continue;
        }
        let pct: u8 = read(d.join("capacity")).parse().ok()?;
        let st = read(d.join("status"));
        return Some((pct.min(100), st == "Charging" || st == "Full"));
    }
    None
}

fn network() -> Option<i32> {
    let rd = std::fs::read_dir("/sys/class/net").ok()?;
    let mut wired = false;
    for e in rd.flatten() {
        let d = e.path();
        let name = e.file_name().to_string_lossy().to_string();
        if name == "lo" || read(d.join("operstate")) != "up" {
            continue;
        }
        if d.join("wireless").exists() {
            return Some(wifi_signal(&name).unwrap_or(70));
        }
        if !name.starts_with("tailscale") && !name.starts_with("docker") && !name.starts_with("virbr") && !name.starts_with("veth") {
            wired = true;
        }
    }
    wired.then_some(-1)
}

/// /proc/net/wireless's link quality (out of 70) as 0-100.
fn wifi_signal(iface: &str) -> Option<i32> {
    let t = std::fs::read_to_string("/proc/net/wireless").ok()?;
    for l in t.lines().skip(2) {
        let mut it = l.split_whitespace();
        if it.next()?.trim_end_matches(':') != iface {
            continue;
        }
        let _status = it.next()?;
        let q: f32 = it.next()?.trim_end_matches('.').parse().ok()?;
        return Some(((q / 70.0) * 100.0).round().clamp(0.0, 100.0) as i32);
    }
    None
}

/// `wpctl get-volume @DEFAULT_AUDIO_SINK@` -> "Volume: 0.45 [MUTED]".
pub fn parse_volume(s: &str) -> Option<(u8, bool)> {
    let v: f32 = s.split_whitespace().nth(1)?.parse().ok()?;
    Some(((v * 100.0).round().clamp(0.0, 150.0) as u8, s.contains("MUTED")))
}

fn volume_now() -> Option<(u8, bool)> {
    let o = std::process::Command::new("wpctl").args(["get-volume", "@DEFAULT_AUDIO_SINK@"]).output().ok()?;
    parse_volume(&String::from_utf8_lossy(&o.stdout))
}

/// The volume up or down by 5 (the bar's scroll), or muted on and off (middle-click), as Mantel does.
pub fn volume_step(up: bool) {
    let _ = std::process::Command::new("wpctl").args(["set-volume", "-l", "1.0", "@DEFAULT_AUDIO_SINK@", if up { "5%+" } else { "5%-" }]).status();
}
pub fn volume_mute() {
    let _ = std::process::Command::new("wpctl").args(["set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"]).status();
}

/// The runtime folder (the menu's pid file).
pub fn run_dir() -> PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/tmp"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wpctl_volume_is_read() {
        assert_eq!(parse_volume("Volume: 0.45\n"), Some((45, false)));
        assert_eq!(parse_volume("Volume: 1.00 [MUTED]\n"), Some((100, true)));
        assert_eq!(parse_volume("nothing"), None);
    }
}
