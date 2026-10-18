//! The library: every store's games merged with Hearth's own data (favorites, hidden, play time,
//! collections), in the shape the UI reads (the Electron host's Library.dto). Steam for now; the other
//! stores come over from the Electron host one by one.

use crate::steam;
use crate::store::Store;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Game {
    pub id: String,
    pub title: String,
    pub source: &'static str,
    pub install_path: Option<PathBuf>,
    pub size_bytes: Option<u64>,
    pub added: u64,
    pub store_minutes: u64,
    pub store_last_played: u64,
    pub installed: bool,
    pub state: &'static str,
    pub bytes_done: Option<u64>,
    pub bytes_total: Option<u64>,
    pub art: Vec<(&'static str, String)>,
    pub launch_uri: Option<String>,
}

impl From<steam::SteamGame> for Game {
    fn from(g: steam::SteamGame) -> Game {
        Game {
            id: format!("steam:{}", g.app_id), launch_uri: Some(format!("steam://rungameid/{}", g.app_id)),
            title: g.title, source: "steam", install_path: Some(g.install_path), size_bytes: g.size_bytes, added: g.added,
            store_minutes: g.minutes, store_last_played: g.last_played, installed: g.installed, state: g.state,
            bytes_done: g.bytes_done, bytes_total: g.bytes_total, art: g.art,
        }
    }
}

/// A local picture goes to the UI through Hearth's own address (nimbus-art://file/<path>), so the page can
/// show it without reading files itself; web pictures pass straight through.
pub fn art_url(v: &str) -> String {
    if v.starts_with("http://") || v.starts_with("https://") { return v.to_string(); }
    let mut out = String::from("nimbus-art://file/");
    for b in v.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.~/".contains(&b) { out.push(b as char); } else { out.push_str(&format!("%{b:02X}")); }
    }
    out
}

pub struct Library {
    pub store: Store,
    pub games: BTreeMap<String, Game>,
    pub counts: BTreeMap<String, usize>,
    pub home: PathBuf,
}

impl Library {
    pub fn new(store: Store, home: PathBuf) -> Library { Library { store, games: BTreeMap::new(), counts: BTreeMap::new(), home } }

    pub fn scan(&mut self) {
        let mut all = BTreeMap::new();
        let mut counts = BTreeMap::new();
        if self.store.source_on("steam") {
            let list: Vec<Game> = steam::scan(&self.home).into_iter().map(Game::from).collect();
            counts.insert("steam".to_string(), list.iter().filter(|g| g.installed).count());
            for g in list { all.entry(g.id.clone()).or_insert(g); }
        }
        self.games = all;
        self.counts = counts;
    }

    pub fn dto(&self, g: &Game) -> Value {
        let d = &self.store.data;
        let stats = &d["stats"][&g.id];
        let art: serde_json::Map<String, Value> = g.art.iter().map(|(k, v)| (k.to_string(), Value::String(art_url(v)))).collect();
        let cols: Vec<Value> = d["collections"].as_array().map(|a| a.iter()
            .filter(|c| c["games"].as_array().map(|g2| g2.iter().any(|x| x == g.id.as_str())).unwrap_or(false))
            .map(|c| c["id"].clone()).collect()).unwrap_or_default();
        json!({
            "id": g.id, "title": g.title, "source": g.source,
            "fav": self.store.list_has("favorites", &g.id), "hidden": self.store.list_has("hidden", &g.id), "cols": cols,
            "minutes": g.store_minutes.max(stats["minutes"].as_u64().unwrap_or(0)),
            "lastPlayed": g.store_last_played.max(stats["lastPlayed"].as_u64().unwrap_or(0)),
            "added": g.added, "sizeBytes": g.size_bytes, "art": art,
            "path": if g.installed { g.install_path.as_ref().map(|p| p.to_string_lossy().to_string()) } else { None },
            "canRemove": g.source == "manual", "installed": g.installed, "state": g.state, "cloud": false,
            "progress": match (g.bytes_done, g.bytes_total) { (Some(a), Some(b)) if b > 0 => Some((a as f64 / b as f64).min(1.0)), _ => None },
            "bytesDone": g.bytes_done, "bytesTotal": g.bytes_total, "canUninstall": false, "note": Value::Null,
            "installKind": if g.source == "steam" { Some("steam") } else { None }, "system": Value::Null,
        })
    }

    pub fn get(&mut self, rescan: bool) -> Value {
        if rescan || self.games.is_empty() { self.scan(); }
        let mut games: Vec<Value> = self.games.values().map(|g| self.dto(g)).collect();
        games.sort_by_key(|g| g["title"].as_str().unwrap_or("").to_lowercase());
        let collections: Vec<Value> = self.store.data["collections"].as_array().map(|a| a.iter()
            .map(|c| json!({"id": c["id"], "name": c["name"], "count": c["games"].as_array().map(|g| g.len()).unwrap_or(0)})).collect()).unwrap_or_default();
        json!({ "games": games, "sources": self.store.data["sources"], "collections": collections, "counts": self.counts,
                "runningId": Value::Null, "steamBackground": self.store.data["steamBackground"] })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn local_pictures_get_hearths_address() {
        assert_eq!(art_url("/home/sam/a b/620p.png"), "nimbus-art://file//home/sam/a%20b/620p.png");
        assert_eq!(art_url("https://cdn.x/1.jpg"), "https://cdn.x/1.jpg");
    }
}
