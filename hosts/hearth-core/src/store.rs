//! Hearth's own per-person data in library.json: favorites, hidden games, play time, sources, collections,
//! per-game settings. Kept as JSON so keys this host doesn't know yet survive untouched.

use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};

pub struct Store { pub file: PathBuf, pub data: Value, seen: Option<(std::time::SystemTime, u64)> }

fn defaults() -> Value {
    json!({ "favorites": [], "hidden": [], "manual": [], "stats": {}, "steamBackground": true, "autoUpdateCheck": true,
            "sources": { "steam": true, "epic": true, "gog": true, "xbox": true, "emu": true } })
}

/// The saved object with the defaults filled in where keys are missing (never over a saved value).
fn with_defaults(saved: Map<String, Value>) -> Value {
    let mut m = saved;
    if let Value::Object(d) = defaults() { crate::settings_file::with_defaults(&mut m, &d); }
    Value::Object(m)
}
fn stamp_of(file: &Path) -> Option<(std::time::SystemTime, u64)> {
    std::fs::metadata(file).ok().and_then(|m| Some((m.modified().ok()?, m.len())))
}

impl Store {
    /// A damaged file is put back from library.json.bak (settings_file); with no good copy it's kept as
    /// library.json.broken-<time> and Hearth starts with the defaults, but never saves them over it unasked.
    pub fn open(file: PathBuf) -> Store {
        let saved = crate::settings_file::read(&file).map().unwrap_or_default();
        let seen = stamp_of(&file);
        Store { file, data: with_defaults(saved), seen }
    }
    /// Writes what's in memory, merged over the latest file: see `update`.
    pub fn save(&self) -> std::io::Result<()> {
        let mine = self.data.as_object().cloned().unwrap_or_default();
        crate::settings_file::update_json_opt(&self.file, false, |m| { for (k, v) in mine { m.insert(k, v); } }).map(|_| ())
    }
    /// Changes the library file. Merge on write: the latest file is read under the lock (nimbus --play-now, Hearth's
    /// core service, Mantel's Task Manager and Hearth Link write it too), `change` is applied to that, and it's
    /// written; a copy held in memory since this process started never puts back what another one changed.
    pub fn update(&mut self, change: impl FnOnce(&mut Map<String, Value>)) -> std::io::Result<()> {
        let (file, seen, mine) = (self.file.clone(), self.seen, self.data.clone());
        let (m, _) = crate::settings_file::update_json_opt(&self.file, false, |m| {
            // The file changed since we read it (checked with the lock held): start from it, with defaults for missing
            // keys only; else from ours (which may hold changes made in memory).
            let fresh = stamp_of(&file) != seen;
            let mut base = if fresh { with_defaults(std::mem::take(m)) } else { mine };
            if let Value::Object(b) = &mut base { change(b); }
            if let Value::Object(b) = base { *m = b; }
        })?;
        self.data = Value::Object(m);
        self.seen = stamp_of(&self.file);
        Ok(())
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
    fn another_writer_s_change_is_never_put_back() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("library.json");
        std::fs::write(&f, r#"{"favorites":["steam:1"],"steamBackground":true}"#).unwrap();
        let mut hearth = Store::open(f.clone());                 // Hearth keeps its copy in memory
        std::thread::sleep(std::time::Duration::from_millis(20));
        // Mantel's Task Manager (hearth-tasks) turns Steam-in-the-background off meanwhile.
        crate::settings_file::merge_json(&f, &json!({"steamBackground": false}).as_object().unwrap().clone()).unwrap();
        hearth.set_in("favorites", "gog:2", true).unwrap();
        let again = Store::open(f);
        assert_eq!(again.data["steamBackground"], json!(false));
        assert_eq!(again.data["favorites"], json!(["steam:1", "gog:2"]));
        assert_eq!(hearth.data["steamBackground"], json!(false));
    }
    #[test]
    fn a_damaged_file_comes_back_from_its_copy_not_as_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("library.json");
        let mut s = Store::open(f.clone());
        s.update(|d| { d.insert("setupDone".into(), json!(true)); d.insert("steamBackground".into(), json!(false)); }).unwrap();
        s.set_in("favorites", "steam:1", true).unwrap();
        std::fs::write(&f, b"").unwrap();                         // a power cut on an older version
        let again = Store::open(f.clone());
        assert_eq!(again.data["setupDone"], json!(true));
        assert_eq!(again.data["steamBackground"], json!(false));
        assert!(serde_json::from_slice::<Value>(&std::fs::read(&f).unwrap()).unwrap()["setupDone"] == json!(true));
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
