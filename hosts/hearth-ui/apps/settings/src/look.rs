//! Settings' own look choice (Accessibility › Controller look): Automatic (the controller look while you use a
//! controller, the keyboard-and-mouse look when you go back to them; the default), Always or Never. Kept in
//! ~/.config/nimbus/settings-app.json ("controllerLook") through settings_file.rs (red flag 9: atomic, .bak, merged;
//! opening Settings writes nothing; only choosing writes). `--layout` / HEARTH_SETTINGS_LAYOUT still win (pictures).

use serde_json::{json, Map, Value};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU8, Ordering};

/// The choice in force (0 auto, 1 always, 2 never): read once at start, changed only by choosing.
static PREF: AtomicU8 = AtomicU8::new(0);

pub fn current() -> &'static str {
    match PREF.load(Ordering::Relaxed) {
        1 => "always",
        2 => "never",
        _ => "auto",
    }
}
/// The look follows what you use (Automatic).
pub fn auto() -> bool {
    PREF.load(Ordering::Relaxed) == 0
}
pub fn set_current(v: &str) {
    PREF.store(match v { "always" => 1, "never" => 2, _ => 0 }, Ordering::Relaxed);
}

pub fn path() -> PathBuf {
    let cfg = std::env::var("XDG_CONFIG_HOME").ok().filter(|s| !s.is_empty()).map(PathBuf::from).unwrap_or_else(|| hearth_ui::paths::home().join(".config"));
    cfg.join("nimbus").join("settings-app.json")
}

/// "auto", "always" or "never" (anything else, or no file: "auto").
pub fn read() -> &'static str {
    let v = hearth_ui::settings_file::read_json(&path());
    match v.get("controllerLook").and_then(Value::as_str) {
        Some("always") => "always",
        Some("never") => "never",
        _ => "auto",
    }
}

/// None: follows what you use; Some(true): always the controller look; Some(false): never.
pub fn fixed(v: &str) -> Option<bool> {
    match v {
        "always" => Some(true),
        "never" => Some(false),
        _ => None,
    }
}

pub fn save(v: &str) -> Result<(), String> {
    let mut m = Map::new();
    m.insert("controllerLook".into(), json!(v));
    hearth_ui::settings_file::merge_json(&path(), &m).map(|_| ()).map_err(|e| format!("Couldn't save it ({e})"))
}
