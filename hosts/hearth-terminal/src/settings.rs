//! Settings (font size, theme, default shell, opacity, cursor...) as one JSON file in your config folder:
//! ~/.config/hearth-terminal/settings.json on Linux, %APPDATA%\hearth-terminal\settings.json on Windows.
//! The page owns what's inside; this side only reads and writes it.

use serde_json::Value;
use std::path::PathBuf;

pub fn path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("hearth-terminal").join("settings.json"))
}

static SEEN: crate::appfile::Seen = std::sync::Mutex::new(None);

/// The saved settings, or an empty object when there are none yet. A file a power cut damaged is put back from
/// settings.json.bak (it used to read as empty, and the next change saved the defaults over all your settings).
pub fn load() -> Value {
    path()
        .and_then(|p| crate::appfile::read_seen(&p, &SEEN))
        .filter(Value::is_object)
        .unwrap_or_else(|| Value::Object(Default::default()))
}

/// Saves the settings the page changed since this window read them (atomic, with .bak): another window that changed
/// a different setting meanwhile keeps its change instead of getting this window's older copy.
pub fn save(v: &Value) -> std::io::Result<()> {
    let p = path().ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "no config folder"))?;
    crate::appfile::save_changed(&p, &SEEN, v)
}
