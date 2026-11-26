//! An app's choices (red flag 9: remembered, never changed by themselves): ~/.config/nimbus/<app>.json, read with
//! settings_file (a damaged file is put back from its .bak) and written only when the person changes one, one key at
//! a time, merged under the lock (settings_file::update_json: atomic, .bak). Defaults live here in the code and are
//! never saved. No secret ever goes in here (tokens are in the keyring: secret.rs).

use hearth_ui::settings_file;
use serde_json::{Map, Value};
use std::path::PathBuf;

pub fn path(app: &str) -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| hearth_ui::paths::home().join(".config"))
        .join("nimbus")
        .join(format!("{app}.json"))
}

#[derive(Debug, Clone, Default)]
pub struct Prefs {
    pub app: String,
    pub m: Map<String, Value>,
    /// the file couldn't be read (and had no good .bak): shown, and defaults are used without being saved
    pub broken: bool,
}

impl Prefs {
    pub fn load(app: &str) -> Prefs {
        let p = path(app);
        match settings_file::read(&p) {
            settings_file::Read::Ok(m) | settings_file::Read::Restored(m) => Prefs { app: app.into(), m, broken: false },
            settings_file::Read::Missing => Prefs { app: app.into(), m: Map::new(), broken: false },
            settings_file::Read::Unreadable => Prefs { app: app.into(), m: Map::new(), broken: true },
        }
    }
    pub fn get(&self, k: &str) -> Option<&Value> {
        self.m.get(k)
    }
    pub fn s(&self, k: &str, d: &str) -> String {
        self.m.get(k).and_then(Value::as_str).unwrap_or(d).to_string()
    }
    pub fn b(&self, k: &str, d: bool) -> bool {
        self.m.get(k).and_then(Value::as_bool).unwrap_or(d)
    }
    pub fn f(&self, k: &str, d: f64) -> f64 {
        self.m.get(k).and_then(Value::as_f64).unwrap_or(d)
    }
    pub fn list(&self, k: &str) -> Vec<String> {
        self.m.get(k).and_then(Value::as_array).map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default()
    }
    /// The person changed one: only this key is written (merged with whatever else is in the file now).
    pub fn set(&mut self, k: &str, v: Value) -> Result<(), String> {
        let p = path(&self.app);
        if let Some(d) = p.parent() {
            std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
        }
        let key = k.to_string();
        let val = v.clone();
        let (m, _) = settings_file::update_json(&p, move |m| {
            m.insert(key, val);
        })
        .map_err(|e| format!("Your choice couldn't be saved: {e}"))?;
        self.m = m;
        self.broken = false;
        Ok(())
    }
}
