//! What Hearth Retro keeps, and only when the person changes it (red flag 9: settings_file's atomic writes with a
//! .bak, merged on write, never defaults over a file that didn't read):
//!   ~/.config/hearth/retro.json          favourites, each game's look (CRT, 4:3 smooth), the ROM folder picked, the
//!                                        RetroAchievements name (and the person's own web API key, typed in by them)
//!   ~/.local/share/hearth/retro/played.json   when each game was last played (written when it starts)

use hearth_ui::settings_file;
use serde_json::{Map, Value};
use std::path::PathBuf;

pub fn path() -> PathBuf {
    std::env::var_os("HEARTH_RETRO_PREFS").map(PathBuf::from).unwrap_or_else(|| crate::home().join(".config/hearth/retro.json"))
}

pub fn played_path() -> PathBuf {
    crate::home().join(".local/share/hearth/retro/played.json")
}

/// The settings as they are on disk ({} when there are none or they couldn't be read: nothing is written then).
pub fn read() -> Map<String, Value> {
    settings_file::read(&path()).map().unwrap_or_default()
}

/// One change, merged into the file as it is now.
pub fn change(f: impl FnOnce(&mut Map<String, Value>)) -> Result<(), String> {
    if let Some(dir) = path().parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    settings_file::update_json(&path(), |m| f(m)).map(|_| ()).map_err(|e| e.to_string())
}

pub fn played() -> Map<String, Value> {
    settings_file::read(&played_path()).map().unwrap_or_default()
}

/// A game started now: its time (seconds) and how many times.
pub fn mark_played(rom: &str, now: i64) {
    if let Some(dir) = played_path().parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let rom = rom.to_string();
    let _ = settings_file::update_json(&played_path(), move |m| {
        let n = m.get(&rom).and_then(|v| v.get("count")).and_then(Value::as_u64).unwrap_or(0) + 1;
        m.insert(rom, serde_json::json!({"last": now, "count": n}));
    });
}

pub fn favourites(m: &Map<String, Value>) -> Vec<String> {
    m.get("favourites").and_then(Value::as_array).map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default()
}

/// A game's own look: (CRT look, 4:3 smooth). Both on unless the person turned one off (RetroDECK's own defaults).
pub fn look(m: &Map<String, Value>, rom: &str) -> (bool, bool) {
    let g = m.get("looks").and_then(|l| l.get(rom));
    let b = |k: &str| g.and_then(|x| x.get(k)).and_then(Value::as_bool).unwrap_or(true);
    (b("crt"), b("smooth"))
}
