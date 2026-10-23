//! Where each app keeps its things. HEARTH_TIME_HOME (tests, CI) puts all of them under one folder instead.
//!   Clock     ~/.config/hearth-clock/clock.json
//!   Calendar  ~/.local/share/hearth-calendar/calendar.json, subscribed calendars in subs/<id>.ics next to it
//!   Notes     ~/Documents/Notes (*.md; pasted pictures in images/; deleted notes in .trash/; pins in .pinned)
//!   Weather   ~/.config/hearth-weather/weather.json; the forecast and the summary in ~/.cache/hearth-weather
//!   Service   ~/.local/state/hearth-time (what has rung already, the "stop ringing" flag)

use std::path::PathBuf;

fn test_home() -> Option<PathBuf> { std::env::var_os("HEARTH_TIME_HOME").filter(|v| !v.is_empty()).map(PathBuf::from) }

fn home() -> PathBuf { dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")) }

fn config() -> PathBuf { test_home().map(|h| h.join("config")).or_else(dirs::config_dir).unwrap_or_else(|| home().join(".config")) }
fn data() -> PathBuf { test_home().map(|h| h.join("data")).or_else(dirs::data_dir).unwrap_or_else(|| home().join(".local/share")) }
fn cache() -> PathBuf { test_home().map(|h| h.join("cache")).or_else(dirs::cache_dir).unwrap_or_else(|| home().join(".cache")) }
fn state() -> PathBuf {
    test_home().map(|h| h.join("state")).or_else(dirs::state_dir).or_else(dirs::data_local_dir).unwrap_or_else(|| home().join(".local/state"))
}

pub fn clock_file() -> PathBuf { config().join("hearth-clock").join("clock.json") }
pub fn calendar_dir() -> PathBuf { data().join("hearth-calendar") }
pub fn calendar_file() -> PathBuf { calendar_dir().join("calendar.json") }
pub fn subs_dir() -> PathBuf { calendar_dir().join("subs") }
pub fn weather_file() -> PathBuf { config().join("hearth-weather").join("weather.json") }
pub fn weather_cache() -> PathBuf { cache().join("hearth-weather") }
pub fn service_dir() -> PathBuf { state().join("hearth-time") }
pub fn sound_file() -> PathBuf { cache().join("hearth-time").join("alarm.wav") }

/// ~/Documents/Notes.
pub fn notes_dir() -> PathBuf {
    if let Some(h) = test_home() { return h.join("Documents").join("Notes"); }
    dirs::document_dir().unwrap_or_else(|| home().join("Documents")).join("Notes")
}

/// Do not disturb as Hearth's taskbar keeps it (~/.config/nimbus/desk.json, "dnd").
pub fn dnd_on() -> bool {
    let f = config().join("nimbus").join("desk.json");
    std::fs::read_to_string(f).ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .and_then(|v| v.get("dnd").and_then(|d| d.as_bool()))
        .unwrap_or(false)
}

/// Folders that may hold .ics files to import (shown as a list, for picking with a controller).
pub fn ics_places() -> Vec<PathBuf> {
    if let Some(h) = test_home() { return vec![h.join("Downloads"), h.join("Documents")]; }
    let mut v = Vec::new();
    if let Some(d) = dirs::download_dir() { v.push(d); }
    if let Some(d) = dirs::document_dir() { v.push(d); }
    v.push(home().join("Desktop"));
    v
}
