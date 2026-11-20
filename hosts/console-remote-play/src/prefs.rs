//! Your choices (red flag 9): ~/.config/nimbus/console-remote-play.json, only through hearth-ui's settings_file.rs
//! (atomic, a .bak kept, merged under its lock, never defaults over a file that didn't read). Opening the app or a
//! page writes nothing: a choice is written the moment you change it, and only that choice.

use hearth_ui::settings_file;
use serde_json::{Map, Value};
use std::path::PathBuf;

pub fn path() -> PathBuf {
    crate::home().join(".config/nimbus/console-remote-play.json")
}

#[derive(Debug, Clone, PartialEq)]
pub struct Prefs {
    /// "auto", "720p", "1080p".
    pub quality: String,
    /// "off", "hotspot", "always".
    pub saver: String,
    pub limit_on: bool,
    pub limit_gb: u32,
    pub warn_pct: u32,
    /// The day of the month the count starts again (1: on the 1st).
    pub reset_day: u32,
    /// "builtin" (the Ally's own controls) or "paired".
    pub play_with: String,
    pub vibration: bool,
    pub full_screen: bool,
    pub keep_awake: bool,
    /// The sound output's name (PipeWire's node), "" for the default.
    pub sound: String,
    pub volume: u32,
    pub show_stats: bool,
    /// The file couldn't be read (and wasn't restored from its .bak): nothing is written until it can be.
    pub unreadable: bool,
}

impl Default for Prefs {
    fn default() -> Self {
        Prefs {
            quality: "auto".into(),
            saver: "hotspot".into(),
            limit_on: true,
            limit_gb: 20,
            warn_pct: 90,
            reset_day: 1,
            play_with: "builtin".into(),
            vibration: true,
            full_screen: true,
            keep_awake: true,
            sound: String::new(),
            volume: 80,
            show_stats: false,
            unreadable: false,
        }
    }
}

pub const LIMITS: [u32; 7] = [5, 10, 15, 20, 30, 50, 100];
pub const WARNS: [u32; 4] = [75, 80, 90, 95];

fn s(m: &Map<String, Value>, k: &str, ok: &[&str], d: &str) -> String {
    m.get(k).and_then(Value::as_str).filter(|v| ok.contains(v)).unwrap_or(d).to_string()
}
fn b(m: &Map<String, Value>, k: &str, d: bool) -> bool {
    m.get(k).and_then(Value::as_bool).unwrap_or(d)
}
fn n(m: &Map<String, Value>, k: &str, lo: u64, hi: u64, d: u32) -> u32 {
    m.get(k).and_then(Value::as_u64).filter(|v| (lo..=hi).contains(v)).map(|v| v as u32).unwrap_or(d)
}

pub fn from_map(m: &Map<String, Value>) -> Prefs {
    let d = Prefs::default();
    Prefs {
        quality: s(m, "quality", &["auto", "720p", "1080p"], &d.quality),
        saver: s(m, "dataSaver", &["off", "hotspot", "always"], &d.saver),
        limit_on: b(m, "limitOn", d.limit_on),
        limit_gb: n(m, "limitGb", 1, 2000, d.limit_gb),
        warn_pct: n(m, "warnPct", 50, 99, d.warn_pct),
        reset_day: n(m, "resetDay", 1, 28, d.reset_day),
        play_with: s(m, "playWith", &["builtin", "paired"], &d.play_with),
        vibration: b(m, "vibration", d.vibration),
        full_screen: b(m, "startFullScreen", d.full_screen),
        keep_awake: b(m, "keepAwake", d.keep_awake),
        sound: m.get("soundOutput").and_then(Value::as_str).unwrap_or("").to_string(),
        volume: n(m, "volume", 0, 100, d.volume),
        show_stats: b(m, "showStats", d.show_stats),
        unreadable: false,
    }
}

pub fn load() -> Prefs {
    load_from(&path())
}

pub fn load_from(p: &std::path::Path) -> Prefs {
    match settings_file::read(p) {
        settings_file::Read::Ok(m) | settings_file::Read::Restored(m) => from_map(&m),
        settings_file::Read::Missing => Prefs::default(),
        _ => Prefs { unreadable: true, ..Prefs::default() },
    }
}

/// Writes one change (merged into what's there). Refused while the file can't be read.
pub fn set(p: &Prefs, key: &str, v: Value) -> Result<(), String> {
    set_in(&path(), p, key, v)
}

pub fn set_in(file: &std::path::Path, p: &Prefs, key: &str, v: Value) -> Result<(), String> {
    if p.unreadable {
        return Err("Your settings file couldn't be read, so nothing was saved (it's left as it is).".into());
    }
    let mut m = Map::new();
    m.insert(key.to_string(), v);
    settings_file::merge_json(file, &m).map(|_| ()).map_err(|e| format!("Couldn't save it: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn defaults_choices_and_a_broken_file() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("console-remote-play.json");
        let p = load_from(&f);
        assert_eq!(p, Prefs::default());
        assert!(!f.exists(), "reading writes nothing");
        set_in(&f, &p, "quality", json!("720p")).unwrap();
        set_in(&f, &p, "limitGb", json!(30)).unwrap();
        let q = load_from(&f);
        assert_eq!((q.quality.as_str(), q.limit_gb, q.warn_pct), ("720p", 30, 90));
        // Something else in the file stays (merge-on-write).
        std::fs::write(&f, "{\"quality\": \"1080p\", \"other\": 1}").unwrap();
        set_in(&f, &load_from(&f), "vibration", json!(false)).unwrap();
        let t = std::fs::read_to_string(&f).unwrap();
        assert!(t.contains("\"other\"") && t.contains("1080p"));
        // Unknown values fall back to the default, and a broken file is never written over.
        assert_eq!(from_map(json!({"quality": "8k"}).as_object().unwrap()).quality, "auto");
        std::fs::write(&f, "{broken").unwrap();
        let _ = std::fs::remove_file(settings_file::bak_path(&f));
        let bad = load_from(&f);
        assert!(bad.unreadable);
        assert!(set_in(&f, &bad, "quality", json!("auto")).is_err());
    }
}
