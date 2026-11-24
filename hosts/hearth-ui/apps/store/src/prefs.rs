//! The Store's own settings (~/.config/nimbus/hearth-store.json), Store settings' switches (store-9). Red flag 9:
//! written only when the person changes one, with settings_file.rs (atomic, a .bak, merged with what's there, never
//! the defaults over a file that couldn't be read); opening the Store writes nothing. Nothing changes by itself.

use hearth_ui::settings_file;
use serde_json::{Map, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Prefs {
    /// "When the Store opens and once a day" (true, the default) or "Only when the Store opens".
    pub daily: bool,
    /// "Tell me when an update is ready" (the owner, Nov 19: on by default).
    pub notify: bool,
    /// "Download on metered connections" (off).
    pub metered: bool,
    /// "Get test versions" (off; on only on the owner's devices, by him).
    pub test: bool,
    /// "Install updates by themselves" (store-w11-6): off unless the person turns it on; only Hearth parts (apps that
    /// update themselves do it anyway), never while a game runs.
    pub auto: bool,
    /// The file couldn't be read (and its .bak neither): nothing is written over it.
    pub unreadable: bool,
}

impl Default for Prefs {
    fn default() -> Self {
        Prefs { daily: true, notify: true, metered: false, test: false, auto: false, unreadable: false }
    }
}

fn b(m: &Map<String, Value>, k: &str, d: bool) -> bool {
    m.get(k).and_then(Value::as_bool).unwrap_or(d)
}

pub fn load() -> Prefs {
    let d = Prefs::default();
    match settings_file::read(&crate::paths::prefs()) {
        settings_file::Read::Unreadable => Prefs { unreadable: true, ..d },
        r => {
            let m = r.map().unwrap_or_default();
            // Where apps go (Settings › Where apps go › Change…): only a folder in the home folder that's there.
            let at = m.get("appsDir").and_then(Value::as_str).map(std::path::PathBuf::from).filter(|p| p.is_absolute() && p.starts_with(hearth_ui::paths::home()) && p.is_dir());
            crate::paths::set_apps(at);
            Prefs { daily: b(&m, "daily", d.daily), notify: b(&m, "notify", d.notify), metered: b(&m, "metered", d.metered), test: b(&m, "testVersions", d.test), auto: b(&m, "autoInstall", d.auto), unreadable: false }
        }
    }
}

/// One switch changed by the person: merged into the file (only that key).
pub fn set(key: &str, on: bool) -> Result<Prefs, String> {
    let p = load();
    if p.unreadable {
        return Err("The Store's settings file couldn't be read, so nothing was written over it.".into());
    }
    let k = match key {
        "test" => "testVersions",
        "auto" => "autoInstall",
        k => k,
    };
    let mut m = Map::new();
    m.insert(k.to_string(), Value::Bool(on));
    settings_file::merge_json(&crate::paths::prefs(), &m).map_err(|e| format!("Couldn't save it: {e}"))?;
    Ok(load())
}

/// Where apps go, changed by the person (files::move_apps, once every app is in its new folder): merged into the file.
pub fn set_apps_dir(dir: &std::path::Path) -> Result<(), String> {
    if load().unreadable {
        return Err("The Store's settings file couldn't be read, so nothing was written over it.".into());
    }
    let mut m = Map::new();
    m.insert("appsDir".to_string(), Value::String(dir.display().to_string()));
    settings_file::merge_json(&crate::paths::prefs(), &m).map_err(|e| format!("Couldn't save it: {e}"))?;
    crate::paths::set_apps(Some(dir.to_path_buf()));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_one_key_at_a_time() {
        let d = tempfile::tempdir().unwrap();
        std::env::set_var("HOME", d.path());
        std::env::set_var("XDG_CONFIG_HOME", d.path().join(".config"));
        let p = load();
        assert!(p.daily && p.notify && !p.metered && !p.test && !p.auto, "update notice on by default (the owner); installing by itself off");
        assert!(!crate::paths::prefs().exists(), "reading writes nothing");
        std::fs::create_dir_all(crate::paths::prefs().parent().unwrap()).unwrap();
        std::fs::write(crate::paths::prefs(), r#"{"notify": false, "other": 1}"#).unwrap();
        let p = set("test", true).unwrap();
        assert!(p.test && !p.notify, "merged: the other keys stay");
        let v: Value = serde_json::from_slice(&std::fs::read(crate::paths::prefs()).unwrap()).unwrap();
        assert_eq!(v["other"], 1);
        std::env::remove_var("XDG_CONFIG_HOME");
    }
}
