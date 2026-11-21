//! Today's optional trackers: the productivity bar (0–10) and the habits (Writing, Workout, Reading). Kept on this PC
//! in ~/.local/share/hearth/notes/trackers.json, apart from the log, so the log's format never changes (and the
//! workspace check, which allows only log/YYYY-MM-DD.md in log/, stays tidy). Written with settings_file.rs
//! (atomic, .bak, merged), only when you change one.

use hearth_ui::settings_file;
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;

pub const HABITS: [&str; 3] = ["Writing", "Workout", "Reading"];

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Day {
    pub productivity: Option<u8>,
    pub habits: BTreeMap<String, bool>,
}

fn key(vault: &str, day: &str) -> String {
    format!("{vault}/{day}")
}

pub fn load(vault: &str, day: &str) -> Day {
    let m = settings_file::read(&crate::paths::trackers()).map().unwrap_or_default();
    let Some(d) = m.get("days").and_then(|d| d.get(key(vault, day))).and_then(Value::as_object) else { return Day::default() };
    let mut out = Day { productivity: d.get("productivity").and_then(Value::as_u64).map(|v| v.min(10) as u8), ..Default::default() };
    if let Some(h) = d.get("habits").and_then(Value::as_object) {
        for (k, v) in h {
            out.habits.insert(k.clone(), v.as_bool().unwrap_or(false));
        }
    }
    out
}

fn change(vault: &str, day: &str, f: impl FnOnce(&mut Map<String, Value>)) -> Result<Day, String> {
    if matches!(settings_file::read(&crate::paths::trackers()), settings_file::Read::Unreadable) {
        return Err("The trackers' file couldn't be read, so nothing was written over it.".into());
    }
    settings_file::update_json(&crate::paths::trackers(), |m| {
        let days = m.entry("days").or_insert_with(|| json!({}));
        if !days.is_object() {
            *days = json!({});
        }
        let d = days.as_object_mut().unwrap().entry(key(vault, day)).or_insert_with(|| json!({}));
        if !d.is_object() {
            *d = json!({});
        }
        f(d.as_object_mut().unwrap());
    })
    .map_err(|e| format!("Couldn't save it: {e}"))?;
    Ok(load(vault, day))
}

pub fn set_productivity(vault: &str, day: &str, v: u8) -> Result<Day, String> {
    change(vault, day, |d| {
        d.insert("productivity".into(), json!(v.min(10)));
    })
}

pub fn set_habit(vault: &str, day: &str, habit: &str, on: bool) -> Result<Day, String> {
    let h = habit.to_string();
    change(vault, day, move |d| {
        let hs = d.entry("habits").or_insert_with(|| json!({}));
        if !hs.is_object() {
            *hs = json!({});
        }
        hs.as_object_mut().unwrap().insert(h, json!(on));
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trackers_are_kept_apart_from_the_log() {
        let _l = crate::env_lock();
        let d = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_DATA_HOME", d.path().join("data"));
        let vault = "home-chris-hearth-workspace";
        assert_eq!(load(vault, "2026-11-20"), Day::default());
        assert!(!crate::paths::trackers().exists(), "reading writes nothing");
        set_productivity(vault, "2026-11-20", 7).unwrap();
        let day = set_habit(vault, "2026-11-20", "Writing", true).unwrap();
        assert_eq!(day.productivity, Some(7));
        assert_eq!(day.habits.get("Writing"), Some(&true));
        assert_eq!(load(vault, "2026-11-19"), Day::default(), "each day apart");
        // A second change keeps the first (merged), and the .bak holds the version before.
        set_productivity(vault, "2026-11-20", 12).unwrap();
        let day = load(vault, "2026-11-20");
        assert_eq!(day.productivity, Some(10), "0 to 10");
        assert_eq!(day.habits.get("Writing"), Some(&true));
        assert!(settings_file::bak_path(&crate::paths::trackers()).exists());
        assert!(crate::paths::trackers().starts_with(d.path().join("data")), "not in the vault");
        std::env::remove_var("XDG_DATA_HOME");
    }
}
