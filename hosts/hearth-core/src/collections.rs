//! Collections: your own groups of games, shown as Library tabs; kept in library.json, so each profile has its
//! own. Port of library/collections.js.

use crate::store::Store;
use serde_json::{json, Value};

const MAX: usize = 30;

fn list(store: &Store) -> Vec<Value> { store.data["collections"].as_array().cloned().unwrap_or_default() }
fn save(store: &mut Store, cols: Vec<Value>) -> Result<(), String> { store.update(|d| { d.insert("collections".into(), Value::Array(cols)); }).map_err(|e| e.to_string()) }

pub fn create(store: &mut Store, name: &str) -> Result<Value, String> {
    let clean: String = name.trim().chars().take(40).collect();
    if clean.is_empty() { return Err("Give the collection a name".into()); }
    let mut cols = list(store);
    if cols.len() >= MAX { return Err(format!("Hearth has room for {MAX} collections")); }
    if cols.iter().any(|c| c["name"].as_str().map(|n| n.to_lowercase() == clean.to_lowercase()).unwrap_or(false)) { return Err(format!("There's already a collection called {clean}")); }
    let id = format!("c{:x}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0) % 0xffff_ffff_ffff);
    let c = json!({"id": id, "name": clean, "games": []});
    cols.push(c.clone());
    save(store, cols)?;
    Ok(c)
}
pub fn rename(store: &mut Store, id: &str, name: &str) -> Result<(), String> {
    let clean: String = name.trim().chars().take(40).collect();
    if clean.is_empty() { return Err("Give the collection a name".into()); }
    let mut cols = list(store);
    let c = cols.iter_mut().find(|c| c["id"] == id).ok_or("No such collection")?;
    c["name"] = json!(clean);
    save(store, cols)
}
pub fn remove(store: &mut Store, id: &str) -> Result<(), String> {
    let cols = list(store).into_iter().filter(|c| c["id"] != id).collect();
    save(store, cols)
}
pub fn set_game(store: &mut Store, id: &str, game: &str, on: bool) -> Result<(), String> {
    let mut cols = list(store);
    let c = cols.iter_mut().find(|c| c["id"] == id).ok_or("No such collection")?;
    let mut games: Vec<Value> = c["games"].as_array().cloned().unwrap_or_default();
    games.retain(|g| g != game);
    if on { games.push(json!(game)); }
    c["games"] = Value::Array(games);
    save(store, cols)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn make_fill_rename_remove() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = Store::open(dir.path().join("l.json"));
        let c = create(&mut s, "  Couch co-op ").unwrap();
        let id = c["id"].as_str().unwrap().to_string();
        assert!(create(&mut s, "couch CO-OP").unwrap_err().contains("already"));
        set_game(&mut s, &id, "steam:1", true).unwrap();
        set_game(&mut s, &id, "steam:1", true).unwrap();
        assert_eq!(s.data["collections"][0]["games"], json!(["steam:1"]));
        rename(&mut s, &id, "Party").unwrap();
        assert_eq!(s.data["collections"][0]["name"], "Party");
        remove(&mut s, &id).unwrap();
        assert_eq!(s.data["collections"], json!([]));
    }
}
