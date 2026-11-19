//! ~/.config/nimbus/g15.json: everything the G15 remembers, read and written only through settings_file.rs (red flag
//! 9: atomic writes, .bak, merged under the lock, never defaults over a file that failed to read). Nothing is written
//! by opening the app or plugging the keyboard in; only what you change (in the app, or with the keyboard's own keys:
//! M1–M3 pick a set, MR saves a macro, the light button sets the key light) is saved, and only those keys.
//!
//! {
//!   "set": 1,                                   the key set in use (M1–M3)
//!   "keys": {"m1": {"g1": ACTION, …}, "m2": {…}, "m3": {…}},
//!   "names": {"m1": "Everyday", "m2": "Trainz"}, a set's name (the screen shows it when you switch)
//!   "kbdLight": 0|1|2, "lcdLight": 0|1|2,       off, low, high (missing: as the keyboard is)
//!   "lightsOffWhenLocked": bool, "nightDim": bool (follows Night light's own schedule: lights_auto.rs), "mLights": bool,
//!   "pages": ["clock", "system", "music", "alerts", "<own screen id>"…],   the screen's pages, in order
//!   "screens": [SCREEN…],                       your own screens (lcd/screen.rs)
//!   "alerts": bool,                              Hearth's messages pop up on the screen for 5 s
//!   "gameSets": [{"game": "Trainz", "set": 2}],  a set picked by itself while a game runs (back after)
//!   "recordDelays": bool                         MR keeps the pauses between keys
//! }
//! ACTION: {"kind": "keys", "keys": "Ctrl+C"} | {"kind": "text", "text": "…"} |
//!         {"kind": "macro", "name": "…", "steps": [{"key": "A", "down": true, "ms": 0}…]} |
//!         {"kind": "app", "desktop": "firefox.desktop"} or {"kind": "app", "command": "…"} |
//!         {"kind": "hearth", "action": "screenshot"} | {"kind": "media", "action": "play-pause"}

use hearth_ui::settings_file;
use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};

pub fn path() -> PathBuf {
    if let Some(p) = std::env::var_os("HEARTH_G15_CONFIG").filter(|p| !p.is_empty()) {
        return PathBuf::from(p);
    }
    crate::root::config_dir().join("g15.json")
}

/// What's saved ({} when there's no file or it can't be read: shown with defaults, never saved over).
pub fn load_from(p: &Path) -> Value {
    settings_file::read_json(p)
}

pub fn load() -> Value {
    load_from(&path())
}

/// Changes only what `change` changes, on the latest file, under the lock.
pub fn update_at(p: &Path, change: impl FnOnce(&mut Map<String, Value>)) -> std::io::Result<()> {
    settings_file::update_json(p, change).map(|_| ())
}

pub fn update(change: impl FnOnce(&mut Map<String, Value>)) -> std::io::Result<()> {
    update_at(&path(), change)
}

/// Sets one G-key in one set; every other key and set stays as it is.
pub fn set_key(m: &mut Map<String, Value>, set: u8, g: u8, action: Option<Value>) {
    let sets = m.entry("keys").or_insert_with(|| json!({}));
    if !sets.is_object() {
        *sets = json!({});
    }
    let s = sets.as_object_mut().unwrap().entry(format!("m{set}")).or_insert_with(|| json!({}));
    if !s.is_object() {
        *s = json!({});
    }
    let s = s.as_object_mut().unwrap();
    match action {
        Some(a) => {
            s.insert(format!("g{g}"), a);
        }
        None => {
            s.remove(&format!("g{g}"));
        }
    }
}

pub fn key_action(cfg: &Value, set: u8, g: u8) -> Option<&Value> {
    cfg.get("keys")?.get(format!("m{set}"))?.get(format!("g{g}")).filter(|v| v.is_object())
}

pub fn set_in_use(cfg: &Value) -> u8 {
    cfg.get("set").and_then(|v| v.as_u64()).filter(|&s| (1..=3).contains(&s)).unwrap_or(1) as u8
}

pub fn flag(cfg: &Value, k: &str, default: bool) -> bool {
    cfg.get(k).and_then(|v| v.as_bool()).unwrap_or(default)
}

pub fn light(cfg: &Value, k: &str) -> Option<u32> {
    cfg.get(k).and_then(|v| v.as_u64()).map(|v| v.min(2) as u32)
}

pub fn text<'a>(cfg: &'a Value, k: &str, default: &'a str) -> &'a str {
    cfg.get(k).and_then(|v| v.as_str()).unwrap_or(default)
}

/// The screen's pages, in order (the built-in four when none are saved).
pub fn pages(cfg: &Value) -> Vec<String> {
    let v: Vec<String> = cfg.get("pages").and_then(|p| p.as_array()).map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
    if v.is_empty() {
        crate::lcd::screen::BUILT_IN.iter().map(|s| s.to_string()).collect()
    } else {
        v
    }
}

/// How many G-keys are set in a set (the screen shows it when you switch sets).
pub fn keys_set(cfg: &Value, set: u8) -> usize {
    cfg.get("keys").and_then(|k| k.get(format!("m{set}"))).and_then(|s| s.as_object()).map(|o| o.values().filter(|v| v.is_object()).count()).unwrap_or(0)
}

/// "HH:MM" → minutes after midnight.
pub fn minutes(hhmm: &str) -> Option<u32> {
    let (h, m) = hhmm.trim().split_once(':')?;
    let (h, m): (u32, u32) = (h.parse().ok()?, m.parse().ok()?);
    (h < 24 && m < 60).then_some(h * 60 + m)
}

/// Is `now` (minutes after midnight) inside from…to (which may cross midnight)?
pub fn in_window(now: u32, from: u32, to: u32) -> bool {
    if from <= to {
        now >= from && now < to
    } else {
        now >= from || now < to
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn merging_keeps_everything_else() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("g15.json");
        std::fs::write(&p, r#"{"set": 2, "keys": {"m1": {"g1": {"kind": "keys", "keys": "Ctrl+C"}}, "m2": {"g4": {"kind": "text", "text": "hi"}}}, "other": 5}"#).unwrap();
        update_at(&p, |m| set_key(m, 1, 2, Some(json!({"kind": "keys", "keys": "Ctrl+V"})))).unwrap();
        let v = load_from(&p);
        assert_eq!(key_action(&v, 1, 1).unwrap()["keys"], "Ctrl+C");
        assert_eq!(key_action(&v, 1, 2).unwrap()["keys"], "Ctrl+V");
        assert_eq!(key_action(&v, 2, 4).unwrap()["text"], "hi");
        assert_eq!(v["other"], 5);
        assert_eq!(set_in_use(&v), 2);
        assert!(d.path().join("g15.json.bak").exists(), "the last good copy is kept");
        update_at(&p, |m| set_key(m, 1, 1, None)).unwrap();
        assert!(key_action(&load_from(&p), 1, 1).is_none());
        assert_eq!(keys_set(&load_from(&p), 1), 1);
    }
    #[test]
    fn a_damaged_file_is_never_replaced_by_defaults() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("g15.json");
        std::fs::write(&p, r#"{"set": 3, "kbdLight": 1}"#).unwrap();
        update_at(&p, |m| {
            m.insert("lcdLight".into(), json!(2));
        })
        .unwrap();
        std::fs::write(&p, "{\"set\": 3, \"kbd").unwrap();
        // Reading puts the last good copy back; nothing of the defaults is written.
        let v = load_from(&p);
        assert_eq!(set_in_use(&v), 3);
        assert_eq!(light(&v, "kbdLight"), Some(1));
    }
    #[test]
    fn night_windows() {
        let (f, t) = (minutes("21:00").unwrap(), minutes("07:00").unwrap());
        assert!(in_window(minutes("23:30").unwrap(), f, t));
        assert!(in_window(minutes("06:59").unwrap(), f, t));
        assert!(!in_window(minutes("12:00").unwrap(), f, t));
        assert!(minutes("25:00").is_none());
        assert_eq!(pages(&json!({})).len(), 4);
    }
}
