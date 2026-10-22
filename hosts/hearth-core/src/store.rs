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

/// `d[k]` as an object: made one when it's missing, or when a damaged or foreign file has something else there
/// (adding play time or a game's settings panicked on that). Saved repaired with the change.
pub fn object_in<'a>(d: &'a mut Map<String, Value>, k: &str) -> &'a mut Map<String, Value> {
    let v = d.entry(k).or_insert(json!({}));
    if !v.is_object() { *v = json!({}); }
    v.as_object_mut().expect("just made an object")
}
/// `d[k]` as a list, the same way.
pub fn array_in<'a>(d: &'a mut Map<String, Value>, k: &str) -> &'a mut Vec<Value> {
    let v = d.entry(k).or_insert(json!([]));
    if !v.is_array() { *v = json!([]); }
    v.as_array_mut().expect("just made a list")
}
/// `m[k]` as an object (one game's entry in a map of games), the same way.
pub fn object_at<'a>(m: &'a mut Map<String, Value>, k: String, empty: Value) -> &'a mut Value {
    let v = m.entry(k).or_insert(empty.clone());
    if !v.is_object() { *v = empty; }
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn repairs_what_isnt_the_right_kind() {
        let mut d = Map::new();
        d.insert("stats".into(), json!(null)); d.insert("manual".into(), json!(5));
        object_in(&mut d, "stats").insert("a".into(), json!(1));
        array_in(&mut d, "manual").push(json!("x"));
        object_in(&mut d, "new");
        assert_eq!(Value::Object(d.clone()), json!({"stats": {"a": 1}, "manual": ["x"], "new": {}}));
        let s = object_in(&mut d, "stats");
        s.insert("g".into(), json!(7));
        object_at(s, "g".into(), json!({"minutes": 0}))["minutes"] = json!(3);
        assert_eq!(d["stats"]["g"], json!({"minutes": 3}));
    }
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
