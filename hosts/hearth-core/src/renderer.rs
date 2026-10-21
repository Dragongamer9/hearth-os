//! Which engine draws Hearth's screens (docs/RENDERERS.md):
//!   webkit    the system's WebKit, through Tauri (hearth-rs). The usual one.
//!   chromium  Hearth's Electron window app (/usr/lib/hearth-electron), which runs the same Hearth core windowless.
//!             Chosen by itself on Nvidia graphics, where WebKit's fast drawing fails (RTX 50 cards) and its
//!             compatible drawing is sluggish.
//! Settings › Display › Drawing engine writes ~/.config/nimbus/renderer ("webkit" or "chromium"; no file: automatic);
//! HEARTH_RENDERER (webkit|chromium|auto) wins over it. Chromium that didn't bring Hearth's screen up twice within 10
//! minutes is skipped (back to WebKit) until it works again or the setting is changed.

use std::path::{Path, PathBuf};

/// Hearth's Electron window app, where the Hearth OS image puts it.
pub const ELECTRON_APP: &str = "/usr/lib/hearth-electron/hearth-electron";
const FAIL_WINDOW_MS: u64 = 10 * 60_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Engine { WebKit, Chromium }

impl Engine { pub fn name(self) -> &'static str { match self { Engine::WebKit => "webkit", Engine::Chromium => "chromium" } } }

fn now_ms() -> u64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0) }
fn setting_file() -> PathBuf { crate::paths::config_dir().join("renderer") }
fn starts_file() -> PathBuf { crate::paths::config_dir().join("renderer-chromium-starts") }

/// An Nvidia graphics card in this PC (a display-class PCI device from vendor 0x10de). `pci`: /sys/bus/pci/devices.
pub fn has_nvidia(pci: &Path) -> bool {
    std::fs::read_dir(pci).into_iter().flatten().flatten().any(|e| {
        let read = |n: &str| std::fs::read_to_string(e.path().join(n)).map(|s| s.trim().to_ascii_lowercase()).unwrap_or_default();
        read("vendor") == "0x10de" && read("class").starts_with("0x03")
    })
}

/// The choice made: "webkit", "chromium" or "auto" (the env var, else the setting file, else automatic).
pub fn choice() -> String {
    let norm = |s: &str| match s.trim() { "webkit" => Some("webkit"), "chromium" => Some("chromium"), "auto" => Some("auto"), _ => None };
    if let Some(c) = std::env::var("HEARTH_RENDERER").ok().as_deref().and_then(norm) { return c.to_string(); }
    std::fs::read_to_string(setting_file()).ok().as_deref().and_then(norm).unwrap_or("auto").to_string()
}

/// What draws, from the choice, the graphics card, whether the Electron app is there and recent failed starts.
pub fn decide(choice: &str, nvidia: bool, electron_present: bool, recent_failures: usize) -> Engine {
    if !electron_present || recent_failures >= 2 { return Engine::WebKit; }
    match choice {
        "chromium" => Engine::Chromium,
        "webkit" => Engine::WebKit,
        _ => if nvidia { Engine::Chromium } else { Engine::WebKit },
    }
}

/// Chromium starts that never brought Hearth's screen up, in the last 10 minutes (from the start records).
pub fn recent_failures(records: &str, now: u64) -> usize {
    records.lines().filter_map(|l| l.trim().parse::<u64>().ok()).filter(|t| now.saturating_sub(*t) < FAIL_WINDOW_MS).count()
}

/// The Electron app to start instead of drawing with WebKit here, or None (draw with WebKit).
pub fn chromium_to_run() -> Option<PathBuf> {
    let app = std::env::var("HEARTH_ELECTRON_APP").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from(ELECTRON_APP));
    let fails = recent_failures(&std::fs::read_to_string(starts_file()).unwrap_or_default(), now_ms());
    let engine = decide(&choice(), has_nvidia(Path::new("/sys/bus/pci/devices")), app.is_file(), fails);
    if fails >= 2 && engine == Engine::WebKit && choice() != "webkit" {
        crate::bridge::log("renderer: Chromium didn't bring Hearth's screen up twice lately; drawing with WebKit");
    }
    (engine == Engine::Chromium).then_some(app)
}

/// A Chromium start (counted as failed until `note_chromium_ok`).
pub fn note_chromium_start() {
    let f = starts_file();
    let now = now_ms();
    let kept: Vec<String> = std::fs::read_to_string(&f).unwrap_or_default().lines()
        .filter(|l| l.trim().parse::<u64>().map(|t| now.saturating_sub(t) < FAIL_WINDOW_MS).unwrap_or(false)).map(str::to_string).collect();
    let _ = std::fs::create_dir_all(crate::paths::config_dir());
    let _ = std::fs::write(&f, format!("{}{now}\n", kept.iter().map(|l| format!("{l}\n")).collect::<String>()));
}
/// Hearth's screen came up in Chromium: no failed starts.
pub fn note_chromium_ok() { let _ = std::fs::remove_file(starts_file()); }

/// Settings › Display › Drawing engine: what's chosen, what's drawing now, whether Chromium is there.
pub fn status() -> serde_json::Value {
    let running = if std::env::var("HEARTH_RENDERER_RUNNING").as_deref() == Ok("chromium") { "chromium" } else { "webkit" };
    let pick = std::fs::read_to_string(setting_file()).unwrap_or_default().trim().to_string();
    serde_json::json!({
        "pick": if pick == "webkit" || pick == "chromium" { pick } else { "auto".to_string() },
        "running": running,
        "available": Path::new(ELECTRON_APP).is_file(),
        "nvidia": has_nvidia(Path::new("/sys/bus/pci/devices")),
    })
}
/// Sets the choice ("auto" removes the file). Takes effect when Hearth starts again.
pub fn set(pick: &str) -> Result<(), String> {
    if !["auto", "webkit", "chromium"].contains(&pick) { return Err("Unknown choice".into()); }
    let _ = std::fs::create_dir_all(crate::paths::config_dir());
    note_chromium_ok();   // a new choice gets a fresh try
    if pick == "auto" { let _ = std::fs::remove_file(setting_file()); Ok(()) } else { std::fs::write(setting_file(), pick).map_err(|e| e.to_string()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_nvidia_graphics() {
        let t = tempfile::tempdir().unwrap();
        let dev = |name: &str, vendor: &str, class: &str| {
            let d = t.path().join(name);
            std::fs::create_dir_all(&d).unwrap();
            std::fs::write(d.join("vendor"), vendor).unwrap();
            std::fs::write(d.join("class"), class).unwrap();
        };
        dev("0000:00:02.0", "0x8086\n", "0x030000\n");   // Intel graphics
        dev("0000:00:1f.3", "0x10de\n", "0x040300\n");   // Nvidia's HDMI sound: not graphics
        assert!(!has_nvidia(t.path()));
        dev("0000:01:00.0", "0x10de\n", "0x030000\n");
        assert!(has_nvidia(t.path()));
        assert!(!has_nvidia(&t.path().join("missing")));
    }

    #[test]
    fn decides() {
        assert_eq!(decide("auto", true, true, 0), Engine::Chromium);
        assert_eq!(decide("auto", false, true, 0), Engine::WebKit);
        assert_eq!(decide("chromium", false, true, 0), Engine::Chromium);
        assert_eq!(decide("webkit", true, true, 0), Engine::WebKit);
        assert_eq!(decide("auto", true, false, 0), Engine::WebKit, "no Electron app: WebKit");
        assert_eq!(decide("chromium", true, true, 2), Engine::WebKit, "two failed starts: WebKit");
        assert_eq!(decide("auto", true, true, 1), Engine::Chromium);
    }

    #[test]
    fn counts_recent_failed_starts() {
        let now = 100 * 60_000;
        assert_eq!(recent_failures("", now), 0);
        let r = format!("{}\n{}\n{}\nnot a number\n", now - 20 * 60_000, now - 60_000, now - 1000);
        assert_eq!(recent_failures(&r, now), 2);
    }
}
