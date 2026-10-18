//! Hearth's own per-person data in library.json: favorites, hidden games, play time, sources, collections,
//! per-game settings. Kept as JSON so keys this host doesn't know yet survive untouched.

use serde_json::{json, Map, Value};
use std::path::PathBuf;

pub struct Store { pub file: PathBuf, pub data: Value }

fn defaults() -> Value {
    json!({ "favorites": [], "hidden": [], "manual": [], "stats": {}, "steamBackground": true, "autoUpdateCheck": true,
            "sources": { "steam": true, "epic": true, "gog": true, "xbox": true, "emu": true } })
}

impl Store {
    pub fn open(file: PathBuf) -> Store {
        let mut data = defaults();
        // A damaged file starts fresh rather than stop Hearth from starting.
        if let Some(Value::Object(saved)) = std::fs::read(&file).ok().and_then(|b| serde_json::from_slice::<Value>(&b).ok()) {
            let d = data.as_object_mut().unwrap();
            let sources = d["sources"].clone();
            for (k, v) in saved { d.insert(k, v); }
            let mut merged = sources.as_object().cloned().unwrap_or_default();
            if let Some(s) = d.get("sources").and_then(|v| v.as_object()) { for (k, v) in s { merged.insert(k.clone(), v.clone()); } }
            d.insert("sources".into(), Value::Object(merged));
        }
        Store { file, data }
    }
    pub fn save(&self) -> std::io::Result<()> {
        crate::paths::write_atomic(&self.file, serde_json::to_string_pretty(&self.data).unwrap_or_default().as_bytes())
    }
    pub fn update(&mut self, change: impl FnOnce(&mut Map<String, Value>)) -> std::io::Result<()> {
        if let Value::Object(m) = &mut self.data { change(m); }
        self.save()
    }
    pub fn source_on(&self, k: &str) -> bool { self.data["sources"][k] != Value::Bool(false) }
    pub fn list_has(&self, list: &str, id: &str) -> bool {
        self.data[list].as_array().map(|a| a.iter().any(|v| v == id)).unwrap_or(false)
    }
    /// Adds or removes an id in a list (favorites, hidden).
    pub fn set_in(&mut self, list: &str, id: &str, on: bool) -> std::io::Result<()> {
        self.update(|d| {
            let mut items: Vec<Value> = d.get(list).and_then(|v| v.as_array()).cloned().unwrap_or_default();
            items.retain(|v| v != id);
            if on { items.push(Value::String(id.into())); }
            d.insert(list.into(), Value::Array(items));
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn keeps_what_it_does_not_know() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("library.json");
        std::fs::write(&f, r#"{"favorites":["steam:1"],"sources":{"xbox":false},"futureThing":{"a":1}}"#).unwrap();
        let mut s = Store::open(f.clone());
        assert!(s.list_has("favorites", "steam:1"));
        assert!(!s.source_on("xbox"));
        assert!(s.source_on("steam"));
        s.set_in("favorites", "gog:2", true).unwrap();
        s.set_in("favorites", "steam:1", false).unwrap();
        let again = Store::open(f);
        assert_eq!(again.data["favorites"], json!(["gog:2"]));
        assert_eq!(again.data["futureThing"], json!({"a":1}));
    }
}
