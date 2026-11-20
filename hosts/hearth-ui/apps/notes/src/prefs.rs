//! Hearth Notes' settings (~/.config/nimbus/hearth-notes.json), everything on Settings (page 8) and the graph's
//! customising. Red flag 9: written only when you change one, with settings_file.rs (atomic, a .bak, merged with what's
//! there, never the defaults over a file that couldn't be read); opening the app or a page writes nothing, and nothing
//! here changes by itself.

use hearth_ui::settings_file;
use serde_json::{json, Map, Value};

#[derive(Debug, Clone, PartialEq)]
pub struct Prefs {
    /// The vault's folder (None: ~/hearth-workspace).
    pub vault: Option<String>,
    /// Sync by itself every N minutes (0: off). 5, as drawn.
    pub sync_every: u32,
    pub warn_secrets: bool,
    /// "inter", "serif" (Noto Serif) or "mono".
    pub font: String,
    pub font_size: f32,
    /// "live", "source" or "read".
    pub open_in: String,
    /// "hearth" (follows Hearth's looks), "gold" or "firehold".
    pub theme: String,
    /// Today's log starts at (minutes after midnight, US Central): 6:00 AM.
    pub day_start: u32,
    /// Today's productivity and habits shown.
    pub trackers: bool,
    pub graph_mode: String,
    /// The graph's options (Customise), by key.
    pub graph: Map<String, Value>,
    /// Saved looks: name → options.
    pub looks: Map<String, Value>,
    /// Per-project looks: project → look name.
    pub project_looks: Map<String, Value>,
    /// The file couldn't be read (nor its .bak): nothing is written over it.
    pub unreadable: bool,
}

pub const SYNC_CHOICES: [u32; 5] = [0, 2, 5, 10, 30];

impl Default for Prefs {
    fn default() -> Self {
        Prefs {
            vault: None,
            sync_every: 5,
            warn_secrets: true,
            font: "inter".into(),
            font_size: 15.0,
            open_in: "live".into(),
            theme: "hearth".into(),
            day_start: 6 * 60,
            trackers: true,
            graph_mode: "classic".into(),
            graph: Map::new(),
            looks: Map::new(),
            project_looks: Map::new(),
            unreadable: false,
        }
    }
}

fn s(m: &Map<String, Value>, k: &str, d: &str, ok: &[&str]) -> String {
    m.get(k).and_then(Value::as_str).filter(|v| ok.is_empty() || ok.contains(v)).unwrap_or(d).to_string()
}

fn obj(m: &Map<String, Value>, k: &str) -> Map<String, Value> {
    m.get(k).and_then(Value::as_object).cloned().unwrap_or_default()
}

pub fn from_map(m: &Map<String, Value>) -> Prefs {
    let d = Prefs::default();
    Prefs {
        vault: m.get("vault").and_then(Value::as_str).filter(|v| !v.is_empty()).map(str::to_string),
        sync_every: m.get("syncEvery").and_then(Value::as_u64).map(|v| v as u32).filter(|v| SYNC_CHOICES.contains(v)).unwrap_or(d.sync_every),
        warn_secrets: m.get("warnSecrets").and_then(Value::as_bool).unwrap_or(d.warn_secrets),
        font: s(m, "font", &d.font, &["inter", "serif", "mono"]),
        font_size: m.get("fontSize").and_then(Value::as_f64).map(|v| (v as f32).clamp(11.0, 26.0)).unwrap_or(d.font_size),
        open_in: s(m, "openIn", &d.open_in, &["live", "source", "read"]),
        theme: s(m, "theme", &d.theme, &["hearth", "gold", "firehold"]),
        day_start: m.get("dayStart").and_then(Value::as_str).and_then(crate::day::parse_time).unwrap_or(d.day_start),
        trackers: m.get("trackers").and_then(Value::as_bool).unwrap_or(d.trackers),
        graph_mode: s(m, "graphMode", &d.graph_mode, &[]),
        graph: obj(m, "graph"),
        looks: obj(m, "looks"),
        project_looks: obj(m, "projectLooks"),
        unreadable: false,
    }
}

pub fn load() -> Prefs {
    match settings_file::read(&crate::paths::prefs()) {
        settings_file::Read::Unreadable => Prefs { unreadable: true, ..Prefs::default() },
        r => from_map(&r.map().unwrap_or_default()),
    }
}

fn guard() -> Result<(), String> {
    if matches!(settings_file::read(&crate::paths::prefs()), settings_file::Read::Unreadable) {
        return Err("Hearth Notes' settings file couldn't be read, so nothing was written over it.".into());
    }
    Ok(())
}

/// One setting changed by the person: merged into the file (only that key).
pub fn set(key: &str, v: Value) -> Result<Prefs, String> {
    guard()?;
    let mut m = Map::new();
    m.insert(key.to_string(), v);
    settings_file::merge_json(&crate::paths::prefs(), &m).map_err(|e| format!("Couldn't save it: {e}"))?;
    Ok(load())
}

/// One key inside an object setting ("graph", "looks", "projectLooks"), the rest of it kept.
pub fn set_in(object: &str, key: &str, v: Value) -> Result<Prefs, String> {
    guard()?;
    let (o, k) = (object.to_string(), key.to_string());
    settings_file::update_json(&crate::paths::prefs(), move |m| {
        let e = m.entry(o).or_insert_with(|| json!({}));
        if !e.is_object() {
            *e = json!({});
        }
        let inner = e.as_object_mut().unwrap();
        if v.is_null() {
            inner.remove(&k);
        } else {
            inner.insert(k, v);
        }
    })
    .map_err(|e| format!("Couldn't save it: {e}"))?;
    Ok(load())
}

/// "6:00 AM"
pub fn day_start_words(m: u32) -> String {
    crate::day::clock(m / 60, m % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_one_key_at_a_time_and_never_over_a_broken_file() {
        let _l = crate::env_lock();
        let d = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_CONFIG_HOME", d.path().join("config"));
        let p = load();
        assert_eq!(p.day_start, 360, "the day starts at 6:00 AM (the owner works overnight)");
        assert_eq!(p.sync_every, 5, "sync by itself every 5 minutes, as drawn");
        assert!(p.warn_secrets && p.trackers);
        assert!(!crate::paths::prefs().exists(), "reading writes nothing");
        std::fs::create_dir_all(crate::paths::prefs().parent().unwrap()).unwrap();
        std::fs::write(crate::paths::prefs(), r#"{"other": 1, "syncEvery": 10}"#).unwrap();
        let p = set("syncEvery", json!(0)).unwrap();
        assert_eq!(p.sync_every, 0, "off");
        let p = set("dayStart", json!("05:30")).unwrap();
        assert_eq!(p.day_start, 330);
        let p = set_in("graph", "bloom", json!(0.6)).unwrap();
        assert_eq!(p.graph.get("bloom"), Some(&json!(0.6)));
        let p = set_in("graph", "labels", json!(1.2)).unwrap();
        assert_eq!(p.graph.len(), 2, "the other graph options stay");
        let v: Value = serde_json::from_slice(&std::fs::read(crate::paths::prefs()).unwrap()).unwrap();
        assert_eq!(v["other"], 1, "merged: other keys stay");
        // A broken file (and no good .bak) is never written over.
        std::fs::write(crate::paths::prefs(), "{broken").unwrap();
        std::fs::write(settings_file::bak_path(&crate::paths::prefs()), "{also broken").unwrap();
        assert!(load().unreadable);
        assert!(set("syncEvery", json!(5)).is_err());
        std::env::remove_var("XDG_CONFIG_HOME");
    }
}
