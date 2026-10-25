//! Which engine draws Hearth's screens (docs/RENDERERS.md):
//!   webkit    the system's WebKit, through Tauri (hearth-rs). The usual one.
//!   chromium  Hearth's Electron window app (/usr/lib/hearth-electron), which runs the same Hearth core windowless.
//!             Chosen by itself on Nvidia graphics, where WebKit's fast drawing fails (RTX 50 cards) and its
//!             compatible drawing is sluggish.
//!   qt        Hearth's Qt Quick window app (/usr/lib/hearth-qml, hosts/hearth-qml), Experimental: Game Mode's main
//!             screens drawn natively with QML, on the same windowless core. Never chosen by itself.
//! Settings › Display › Renderer writes ~/.config/nimbus/renderer ("webkit", "chromium" or "qt"; no file: automatic);
//! HEARTH_RENDERER (webkit|chromium|qt|auto) wins over it, and ~/.config/nimbus/renderer-once (written by the Qt
//! renderer's "Open in the web renderer") wins for one start only.
//! Chromium that didn't bring Hearth's screen up twice within 10 minutes is skipped (back to WebKit) until it works
//! again or the setting is changed. Qt that didn't bring Hearth's screen up once, or crashed 3 times within 5 minutes,
//! is given up: the setting goes back to Automatic and the next screen says so (`take_note`).

use std::path::{Path, PathBuf};

/// Hearth's Electron window app, where the Hearth OS image puts it.
pub const ELECTRON_APP: &str = "/usr/lib/hearth-electron/hearth-electron";
/// Hearth's Qt Quick window app (Experimental), where the Hearth OS image puts it.
pub const QT_APP: &str = "/usr/lib/hearth-qml/hearth-qml";
const FAIL_WINDOW_MS: u64 = 10 * 60_000;
/// Qt crashes counted together: 3 within this time give it up.
const QT_CRASH_WINDOW_MS: u64 = 5 * 60_000;
const QT_CRASHES_MAX: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Engine { WebKit, Chromium, Qt }

impl Engine {
    pub fn name(self) -> &'static str { match self { Engine::WebKit => "webkit", Engine::Chromium => "chromium", Engine::Qt => "qt" } }
}

fn now_ms() -> u64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0) }
fn cfg(name: &str) -> PathBuf { crate::paths::config_dir().join(name) }
fn setting_file() -> PathBuf { cfg("renderer") }
fn starts_file() -> PathBuf { cfg("renderer-chromium-starts") }
fn qt_starts_file() -> PathBuf { cfg("renderer-qt-starts") }
fn qt_crashes_file() -> PathBuf { cfg("renderer-qt-crashes") }
fn once_file() -> PathBuf { cfg("renderer-once") }
fn note_file() -> PathBuf { cfg("renderer-note") }

fn electron_app() -> PathBuf { std::env::var("HEARTH_ELECTRON_APP").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from(ELECTRON_APP)) }
fn qt_app() -> PathBuf { std::env::var("HEARTH_QT_APP").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from(QT_APP)) }

/// An Nvidia graphics card in this PC (a display-class PCI device from vendor 0x10de). `pci`: /sys/bus/pci/devices.
pub fn has_nvidia(pci: &Path) -> bool {
    std::fs::read_dir(pci).into_iter().flatten().flatten().any(|e| {
        let read = |n: &str| std::fs::read_to_string(e.path().join(n)).map(|s| s.trim().to_ascii_lowercase()).unwrap_or_default();
        read("vendor") == "0x10de" && read("class").starts_with("0x03")
    })
}

/// A choice as written ("webkit", "chromium", "qt", "auto"; "web" is the one-start "automatic, but not Qt"), or None.
fn norm(s: &str) -> Option<&'static str> {
    match s.trim() { "webkit" => Some("webkit"), "chromium" | "electron" => Some("chromium"), "qt" => Some("qt"), "auto" => Some("auto"), "web" => Some("web"), _ => None }
}

/// The choice made: the env var, else the setting file, else automatic ("web" is never a setting).
pub fn choice() -> String {
    if let Some(c) = std::env::var("HEARTH_RENDERER").ok().as_deref().and_then(norm) { return c.to_string(); }
    std::fs::read_to_string(setting_file()).ok().as_deref().and_then(norm).filter(|c| *c != "web").unwrap_or("auto").to_string()
}

/// What's known at start, for `decide`.
#[derive(Debug, Clone, Copy, Default)]
pub struct Facts { pub nvidia: bool, pub electron: bool, pub qt: bool, pub chromium_fails: usize, pub qt_fails: usize, pub qt_crashes: usize }

/// Whether Qt is given up on this PC for now (it never brought the screen up, or keeps crashing).
pub fn qt_given_up(f: &Facts) -> bool { f.qt_fails >= 1 || f.qt_crashes >= QT_CRASHES_MAX }

/// What draws, from the choice and the facts.
pub fn decide(choice: &str, f: Facts) -> Engine {
    let chromium_ok = f.electron && f.chromium_fails < 2;
    let auto = if f.nvidia && chromium_ok { Engine::Chromium } else { Engine::WebKit };
    match choice {
        "qt" if f.qt && !qt_given_up(&f) => Engine::Qt,
        "chromium" => if chromium_ok { Engine::Chromium } else { Engine::WebKit },
        "webkit" => Engine::WebKit,
        _ => auto,   // "auto", "web", and "qt" that can't run
    }
}

/// Starts (or crashes) recorded in `records` (one time a line) within `window` ms of `now`.
pub fn recent(records: &str, now: u64, window: u64) -> usize {
    records.lines().filter_map(|l| l.trim().parse::<u64>().ok()).filter(|t| now.saturating_sub(*t) < window).count()
}
/// Chromium starts that never brought Hearth's screen up, in the last 10 minutes (from the start records).
pub fn recent_failures(records: &str, now: u64) -> usize { recent(records, now, FAIL_WINDOW_MS) }

fn read(p: &Path) -> String { std::fs::read_to_string(p).unwrap_or_default() }
fn facts() -> Facts {
    let now = now_ms();
    Facts {
        nvidia: has_nvidia(Path::new("/sys/bus/pci/devices")),
        electron: electron_app().is_file(),
        qt: qt_app().is_file(),
        chromium_fails: recent_failures(&read(&starts_file()), now),
        qt_fails: recent(&read(&qt_starts_file()), now, FAIL_WINDOW_MS),
        qt_crashes: recent(&read(&qt_crashes_file()), now, QT_CRASH_WINDOW_MS),
    }
}

/// What to start instead of drawing with WebKit in this process.
#[derive(Debug, Clone, PartialEq)]
pub enum Start { WebKit, Chromium(PathBuf), Qt(PathBuf) }

/// Decided once, when `nimbus` starts: reads (and uses up) a one-start choice, gives Qt up when it failed (the setting
/// goes back to Automatic, and the next screen says why), and logs what draws.
pub fn pick_at_start() -> Start {
    let once = std::fs::read_to_string(once_file()).ok().as_deref().and_then(norm).map(str::to_string);
    if once.is_some() { let _ = std::fs::remove_file(once_file()); }
    let f = facts();
    let mut c = once.clone().unwrap_or_else(choice);
    if c == "qt" && f.qt && qt_given_up(&f) {
        let why = if f.qt_fails >= 1 { "didn't bring Hearth's screen up" } else { "crashed 3 times within a few minutes" };
        let text = format!("The Qt renderer (Experimental) {why}, so Hearth went back to Automatic. Settings › Display › Renderer can turn it on again.");
        crate::bridge::log(&format!("renderer: Qt {why}; the setting goes back to Automatic"));
        give_up_qt(&text);
        c = "auto".into();
    } else if c == "qt" && !f.qt {
        crate::bridge::log("renderer: Qt is chosen but Hearth's Qt window app isn't installed; drawing the usual way");
    }
    if f.chromium_fails >= 2 && f.electron && (c == "chromium" || (c != "webkit" && f.nvidia)) {
        crate::bridge::log("renderer: Chromium didn't bring Hearth's screen up twice lately; drawing with WebKit");
    }
    if let Some(o) = &once { crate::bridge::log(&format!("renderer: \"{o}\" for this start only (Open in the web renderer)")); }
    match decide(&c, f) {
        Engine::Chromium => Start::Chromium(electron_app()),
        Engine::Qt => Start::Qt(qt_app()),
        Engine::WebKit => Start::WebKit,
    }
}

/// The old way in: the Electron app to start, or None.
pub fn chromium_to_run() -> Option<PathBuf> { match pick_at_start() { Start::Chromium(p) => Some(p), _ => None } }

fn push_record(f: &Path, window: u64) {
    let now = now_ms();
    let kept: Vec<String> = read(f).lines()
        .filter(|l| l.trim().parse::<u64>().map(|t| now.saturating_sub(t) < window).unwrap_or(false)).map(str::to_string).collect();
    let _ = std::fs::create_dir_all(crate::paths::config_dir());
    let _ = std::fs::write(f, format!("{}{now}\n", kept.iter().map(|l| format!("{l}\n")).collect::<String>()));
}

/// A Chromium start (counted as failed until `note_chromium_ok`).
pub fn note_chromium_start() { push_record(&starts_file(), FAIL_WINDOW_MS); }
/// Hearth's screen came up in Chromium: no failed starts.
pub fn note_chromium_ok() { let _ = std::fs::remove_file(starts_file()); }
/// A Qt start (counted as failed until `note_qt_ok`).
pub fn note_qt_start() { push_record(&qt_starts_file(), FAIL_WINDOW_MS); }
/// Hearth's screen came up in Qt.
pub fn note_qt_ok() { let _ = std::fs::remove_file(qt_starts_file()); }
/// The Qt window app ended badly (a crash, or an exit code that isn't Hearth's own).
pub fn note_qt_crash() { push_record(&qt_crashes_file(), QT_CRASH_WINDOW_MS); }
/// The page came up: whichever engine draws it worked (from the core, `HEARTH_RENDERER_RUNNING`).
pub fn note_ok_running() {
    match std::env::var("HEARTH_RENDERER_RUNNING").as_deref() { Ok("qt") => note_qt_ok(), Ok("chromium") => note_chromium_ok(), _ => {} }
}
fn give_up_qt(text: &str) {
    let _ = std::fs::remove_file(setting_file());
    let _ = std::fs::remove_file(qt_starts_file());
    let _ = std::fs::remove_file(qt_crashes_file());
    let _ = std::fs::create_dir_all(crate::paths::config_dir());
    let _ = std::fs::write(note_file(), text);
}
/// What the next screen should say about the renderer (once): Qt was given up.
pub fn take_note() -> Option<String> {
    let t = std::fs::read_to_string(note_file()).ok()?;
    let _ = std::fs::remove_file(note_file());
    Some(t.trim().to_string()).filter(|t| !t.is_empty())
}

/// What draws now (from the window app that started the core).
pub fn running() -> &'static str {
    match std::env::var("HEARTH_RENDERER_RUNNING").as_deref() { Ok("chromium") => "chromium", Ok("qt") => "qt", _ => "webkit" }
}

/// Settings › Display › Renderer: what's chosen, what's drawing now, which engines are there.
pub fn status() -> serde_json::Value {
    let pick = std::fs::read_to_string(setting_file()).ok().as_deref().and_then(norm).filter(|c| *c != "web").unwrap_or("auto");
    let f = facts();
    serde_json::json!({
        "pick": pick,
        "running": running(),
        "available": Path::new(ELECTRON_APP).is_file(),
        "electron": Path::new(ELECTRON_APP).is_file(),
        "qt": Path::new(QT_APP).is_file(),
        "nvidia": f.nvidia,
        "qtCrashes": f.qt_crashes,
        "env": std::env::var("HEARTH_RENDERER").ok(),
    })
}
/// Sets the choice ("auto" removes the file). Takes effect when Hearth starts again.
pub fn set(pick: &str) -> Result<(), String> {
    let p = norm(pick).filter(|c| *c != "web").ok_or("Unknown choice")?;
    let _ = std::fs::create_dir_all(crate::paths::config_dir());
    // A new choice gets a fresh try.
    note_chromium_ok(); note_qt_ok(); let _ = std::fs::remove_file(qt_crashes_file());
    crate::bridge::log(&format!("renderer: Settings chose {p} (from the next start)"));
    if p == "auto" { let _ = std::fs::remove_file(setting_file()); Ok(()) } else { std::fs::write(setting_file(), p).map_err(|e| e.to_string()) }
}
/// One start only with another engine ("web": automatic without Qt; the Qt renderer's Open in the web renderer).
pub fn set_once(pick: &str) -> Result<(), String> {
    let p = norm(pick).ok_or("Unknown choice")?;
    let _ = std::fs::create_dir_all(crate::paths::config_dir());
    crate::bridge::log(&format!("renderer: {p} for the next start only"));
    std::fs::write(once_file(), p).map_err(|e| e.to_string())
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

    fn f(nvidia: bool, electron: bool, qt: bool) -> Facts { Facts { nvidia, electron, qt, ..Default::default() } }

    #[test]
    fn decides() {
        assert_eq!(decide("auto", f(true, true, true)), Engine::Chromium);
        assert_eq!(decide("auto", f(false, true, true)), Engine::WebKit);
        assert_eq!(decide("chromium", f(false, true, false)), Engine::Chromium);
        assert_eq!(decide("webkit", f(true, true, true)), Engine::WebKit);
        assert_eq!(decide("auto", f(true, false, false)), Engine::WebKit, "no Electron app: WebKit");
        assert_eq!(decide("chromium", Facts { chromium_fails: 2, ..f(true, true, false) }), Engine::WebKit, "two failed starts: WebKit");
        assert_eq!(decide("auto", Facts { chromium_fails: 1, ..f(true, true, false) }), Engine::Chromium);
    }

    #[test]
    fn decides_qt() {
        assert_eq!(decide("qt", f(false, true, true)), Engine::Qt);
        assert_eq!(decide("qt", f(true, true, true)), Engine::Qt, "chosen Qt wins over Nvidia's automatic Chromium");
        assert_eq!(decide("auto", f(false, false, true)), Engine::WebKit, "Qt is never automatic");
        assert_eq!(decide("qt", f(true, true, false)), Engine::Chromium, "Qt not installed: automatic");
        assert_eq!(decide("qt", Facts { qt_fails: 1, ..f(false, true, true) }), Engine::WebKit, "a start that never came up gives Qt up");
        assert_eq!(decide("qt", Facts { qt_crashes: 2, ..f(false, true, true) }), Engine::Qt);
        assert_eq!(decide("qt", Facts { qt_crashes: 3, ..f(true, true, true) }), Engine::Chromium, "3 crashes: automatic");
        assert_eq!(decide("web", f(true, true, true)), Engine::Chromium, "one start with the web renderer");
        assert_eq!(decide("web", f(false, true, true)), Engine::WebKit);
    }

    #[test]
    fn counts_recent_failed_starts() {
        let now = 100 * 60_000;
        assert_eq!(recent_failures("", now), 0);
        let r = format!("{}\n{}\n{}\nnot a number\n", now - 20 * 60_000, now - 60_000, now - 1000);
        assert_eq!(recent_failures(&r, now), 2);
        assert_eq!(recent(&r, now, 30_000), 1);
    }

    #[test]
    fn choices_read() {
        assert_eq!(norm("qt\n"), Some("qt"));
        assert_eq!(norm("electron"), Some("chromium"));
        assert_eq!(norm("nope"), None);
    }
}
