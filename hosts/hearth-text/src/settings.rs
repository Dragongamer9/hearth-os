//! Settings (font size, theme, word wrap, recent files...) as one JSON file in your config folder:
//! ~/.config/hearth-text/settings.json on Linux, %APPDATA%\hearth-text\settings.json on Windows.
//! The page owns what's inside; this side only reads and writes it.

use serde_json::Value;
use std::path::PathBuf;

pub fn path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("hearth-text").join("settings.json"))
}

/// The saved settings, or an empty object when there are none yet (or the file is damaged).
pub fn load() -> Value {
    path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .filter(Value::is_object)
        .unwrap_or_else(|| Value::Object(Default::default()))
}

/// Saves the settings: written to a temporary file first, so a crash never leaves half a file.
pub fn save(v: &Value) -> std::io::Result<()> {
    let p = path().ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "no config folder"))?;
    if let Some(dir) = p.parent() { std::fs::create_dir_all(dir)?; }
    let tmp = p.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(v)?)?;
    std::fs::rename(&tmp, &p)
}
