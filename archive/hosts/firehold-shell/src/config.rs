//! The settings this shell reads and writes, only through settings_file.rs (red flag 9: atomic, .bak, merge on write,
//! never defaults over a damaged file). Opening anything writes nothing; a key is written only when the person picks a
//! different value.
//!
//! * `~/.config/nimbus/firehold.json`: Couch and Desk (mockup 6), the same keys as the QML shell's (Firehold.qml):
//!   posture (auto, couch, desk), perScreen ({screen: couch|desk}), startIn (couch, desk, last), guide (quick, open),
//!   games (follow, fullscreen, window), bigPicture (bool), overlay (off, basic, full).
//! * `~/.config/nimbus/firehold-rust.json`: {"desktop": "firehold"} when "Firehold (Rust, test)" is Desktop Mode on
//!   Anatase; anything else (or no file) is KDE Plasma, the default. Only `firehold-shell choose` writes it.
//!
//! State, not settings (never in those files): the last posture, for "Start in: Last" (~/.local/state/hearth).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use crate::settings_file;

pub fn home() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/"))
}
pub fn config_dir() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| home().join(".config"))
}
pub fn state_dir() -> PathBuf {
    std::env::var_os("XDG_STATE_HOME").map(PathBuf::from).unwrap_or_else(|| home().join(".local/state")).join("hearth")
}
pub fn firehold_path() -> PathBuf {
    config_dir().join("nimbus/firehold.json")
}
pub fn choice_path() -> PathBuf {
    config_dir().join("nimbus/firehold-rust.json")
}

/// Couch and Desk, as read (defaults filled in memory only).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Couch {
    pub posture: String,
    pub start_in: String,
    pub guide: String,
    pub games: String,
    pub big_picture: bool,
    pub overlay: String,
    pub per_screen: Map<String, Value>,
}

impl Default for Couch {
    fn default() -> Self {
        Couch {
            posture: "auto".into(),
            start_in: "desk".into(),
            guide: "quick".into(),
            games: "follow".into(),
            big_picture: false,
            overlay: "off".into(),
            per_screen: Map::new(),
        }
    }
}

fn pick(m: &Map<String, Value>, k: &str, allowed: &[&str], dflt: &str) -> String {
    m.get(k).and_then(|v| v.as_str()).filter(|v| allowed.contains(v)).unwrap_or(dflt).to_string()
}

impl Couch {
    pub fn from_map(m: &Map<String, Value>) -> Couch {
        Couch {
            posture: pick(m, "posture", &["auto", "couch", "desk"], "auto"),
            start_in: pick(m, "startIn", &["couch", "desk", "last"], "desk"),
            guide: pick(m, "guide", &["quick", "open"], "quick"),
            games: pick(m, "games", &["follow", "fullscreen", "window"], "follow"),
            big_picture: m.get("bigPicture").and_then(|v| v.as_bool()) == Some(true),
            overlay: pick(m, "overlay", &["off", "basic", "full"], "off"),
            per_screen: m.get("perScreen").and_then(|v| v.as_object()).cloned().unwrap_or_default(),
        }
    }
    pub fn read() -> Couch {
        Couch::from_map(&settings_file::read(&firehold_path()).map().unwrap_or_default())
    }
}

/// The keys the shell may set, and the values each takes (anything else is refused, never written).
pub fn valid_setting(key: &str, value: &Value) -> bool {
    let s = value.as_str().unwrap_or("");
    match key {
        "posture" => ["auto", "couch", "desk"].contains(&s),
        "startIn" => ["couch", "desk", "last"].contains(&s),
        "guide" => ["quick", "open"].contains(&s),
        "games" => ["follow", "fullscreen", "window"].contains(&s),
        "overlay" => ["off", "basic", "full"].contains(&s),
        "bigPicture" => value.is_boolean(),
        "perScreen" => value.as_object().is_some_and(|o| o.values().all(|v| matches!(v.as_str(), Some("couch" | "desk")))),
        _ => false,
    }
}

/// One key, merged (only when it's different from what the file has now). Ok(true) when something was written.
pub fn set(key: &str, value: Value) -> Result<bool, String> {
    if !valid_setting(key, &value) {
        return Err(format!("{key} can't be {value}"));
    }
    settings_file::update_json(&firehold_path(), |m| {
        if m.get(key) == Some(&value) {
            false
        } else {
            m.insert(key.to_string(), value.clone());
            true
        }
    })
    .map(|(_, changed)| changed)
    .map_err(|e| e.to_string())
}

/// Desktop Mode on Anatase: "firehold" (the Rust test) or "plasma" (the default, also for no file or a damaged one).
pub fn desktop_choice() -> String {
    match settings_file::read(&choice_path()).map() {
        Some(m) if m.get("desktop").and_then(|d| d.as_str()) == Some("firehold") => "firehold".into(),
        _ => "plasma".into(),
    }
}

/// `firehold-shell choose firehold|plasma`: only the person's pick writes it, and only when it changes.
pub fn choose(what: &str) -> Result<bool, String> {
    if !["firehold", "plasma"].contains(&what) {
        return Err("choose firehold or plasma".into());
    }
    let p = choice_path();
    if what == "plasma" && !p.exists() {
        return Ok(false);
    }
    settings_file::update_json(&p, |m| {
        if m.get("desktop").and_then(|d| d.as_str()) == Some(what) {
            false
        } else {
            m.insert("desktop".into(), json!(what));
            true
        }
    })
    .map(|(_, c)| c)
    .map_err(|e| e.to_string())
}

/// The posture Firehold was in last (for "Start in: Last"): state, written whenever it changes.
pub fn last_posture() -> Option<String> {
    let t = std::fs::read_to_string(state_dir().join("firehold-rust-posture")).ok()?;
    let t = t.trim();
    ["couch", "desk"].contains(&t).then(|| t.to_string())
}
pub fn save_last_posture(p: &str) {
    let d = state_dir();
    let _ = std::fs::create_dir_all(&d);
    let tmp = d.join(format!(".firehold-rust-posture.{}", std::process::id()));
    if std::fs::write(&tmp, format!("{p}\n")).is_ok() {
        let _ = std::fs::rename(&tmp, d.join("firehold-rust-posture"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_never_written_and_bad_values_are_refused() {
        let d = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_CONFIG_HOME", d.path());
        std::env::set_var("XDG_STATE_HOME", d.path().join("state"));
        assert_eq!(Couch::read(), Couch::default());
        assert!(!firehold_path().exists(), "reading writes nothing");
        assert_eq!(desktop_choice(), "plasma");
        assert!(!choose("plasma").unwrap(), "the default isn't written");
        assert!(!choice_path().exists());
        assert!(choose("firehold").unwrap());
        assert_eq!(desktop_choice(), "firehold");
        assert!(!choose("firehold").unwrap(), "the same again writes nothing");
        assert!(choose("nope").is_err());
        // Mantel's keys stay when this shell sets one of its own.
        std::fs::write(firehold_path(), r#"{"firehold": {"unlocked": true}, "posture": "desk"}"#).unwrap();
        assert!(set("posture", json!("couch")).unwrap());
        assert!(!set("posture", json!("couch")).unwrap());
        assert!(set("posture", json!("sideways")).is_err());
        assert!(set("unlocked", json!(true)).is_err());
        let v: Value = serde_json::from_str(&std::fs::read_to_string(firehold_path()).unwrap()).unwrap();
        assert_eq!(v["firehold"]["unlocked"], true);
        assert_eq!(Couch::read().posture, "couch");
        // A damaged file is never replaced by defaults: put back from .bak.
        std::fs::write(firehold_path(), "{nope").unwrap();
        assert_eq!(Couch::read().posture, "desk", "put back from the .bak (the last good copy before the change)");
        assert!(std::fs::read_to_string(firehold_path()).unwrap().contains("unlocked"));
        save_last_posture("couch");
        assert_eq!(last_posture().as_deref(), Some("couch"));
    }
}
