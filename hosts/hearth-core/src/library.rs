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
    /// Non-library games: the file to start and its extra arguments.
    pub exe: Option<String>,
    pub args: String,
    /// Emulated games: the system's name ("Super Nintendo") and folder name ("snes").
    pub system: Option<&'static str>,
    pub system_key: Option<&'static str>,
}

/// The game that's running: what to watch, and its times (ms since 1970).
pub struct Running {
    pub id: String,
    pub track: crate::procs::Track,
    pub launched_at: u64,
    pub started_at: u64,
    pub gone_since: u64,
    pub paused_at: u64,
    pub paused_ms: u64,
    pub closing: bool,
}

const EXIT_GRACE_MS: u64 = 10_000;     // a launcher closing and the game starting a moment later is still one game
const START_TIMEOUT_MS: u64 = 180_000; // Steam may update the game first

pub fn now_ms() -> u64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0) }

impl From<steam::SteamGame> for Game {
    fn from(g: steam::SteamGame) -> Game {
        Game {
            id: format!("steam:{}", g.app_id), launch_uri: Some(format!("steam://rungameid/{}", g.app_id)),
            title: g.title, source: "steam", install_path: Some(g.install_path), size_bytes: g.size_bytes, added: g.added,
            store_minutes: g.minutes, store_last_played: g.last_played, installed: g.installed, state: g.state,
            bytes_done: g.bytes_done, bytes_total: g.bytes_total, art: g.art, exe: None, args: String::new(), system: None, system_key: None,
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
    pub running: Option<Running>,
    pub store: Store,
    pub games: BTreeMap<String, Game>,
    pub counts: BTreeMap<String, usize>,
    pub home: PathBuf,
}

impl Library {
    pub fn new(store: Store, home: PathBuf) -> Library { Library { running: None, store, games: BTreeMap::new(), counts: BTreeMap::new(), home } }

    /// Games you added yourself (kept in library.json "manual").
    pub fn manual_games(&self) -> Vec<Game> {
        self.store.data["manual"].as_array().cloned().unwrap_or_default().iter().filter_map(|m| {
            let id = m["id"].as_str()?.to_string();
            let path = m["path"].as_str()?.to_string();
            Some(Game { id, title: m["title"].as_str().unwrap_or("Game").to_string(), source: "manual", install_path: Some(path.clone().into()),
                size_bytes: None, added: m["added"].as_u64().unwrap_or(0), store_minutes: 0, store_last_played: 0, installed: true, state: "installed",
                bytes_done: None, bytes_total: None, art: vec![], launch_uri: None, exe: Some(path), args: m["args"].as_str().unwrap_or("").to_string(), system: None, system_key: None })
        }).collect()
    }

    pub fn scan(&mut self) {
        let mut all = BTreeMap::new();
        let mut counts = BTreeMap::new();
        if self.store.source_on("steam") {
            let list: Vec<Game> = steam::scan(&self.home).into_iter().map(Game::from).collect();
            counts.insert("steam".to_string(), list.iter().filter(|g| g.installed).count());
            for g in list { all.entry(g.id.clone()).or_insert(g); }
        }
        if self.store.source_on("emu") {
            let emu = crate::emulators::scan(&crate::emulators::rom_roots(&self.home));
            counts.insert("emu".to_string(), emu.len());
            for g in emu { all.entry(g.id.clone()).or_insert(g); }
        }
        let manual = self.manual_games();
        counts.insert("manual".to_string(), manual.len());
        for g in manual { all.entry(g.id.clone()).or_insert(g); }
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
            "installKind": if g.source == "steam" { Some("steam") } else { None }, "system": g.system,
        })
    }

    pub fn running_id(&self) -> Option<String> { self.running.as_ref().filter(|r| r.started_at > 0).map(|r| r.id.clone()) }
    pub fn running_since(&self) -> Option<u64> { self.running.as_ref().filter(|r| r.started_at > 0).map(|r| r.started_at) }

    /// Starts a game. {tracked} says whether Hearth can watch it (it then shows as running).
    pub fn launch(&mut self, id: &str) -> Result<Value, String> {
        let g = self.games.get(id).cloned().ok_or("That game isn't in the library anymore. Try rescanning.")?;
        if !g.installed { return Err(format!("{} isn't installed yet.", g.title)); }
        if self.running.is_some() { return Err("Another game is already running".into()); }
        let track = match g.source {
            "steam" => {
                let uri = g.launch_uri.clone().unwrap_or_default();
                std::process::Command::new("steam").arg(&uri).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null())
                    .spawn().map_err(|e| format!("Couldn't start Steam: {e}"))?;
                crate::procs::Track { steam_app_id: Some(id.trim_start_matches("steam:").to_string()), ..Default::default() }
            }
            "manual" => {
                let exe = g.exe.clone().unwrap_or_default();
                let opts = crate::game_options::get(&self.store, id);
                let pid = crate::manual::launch(&exe, &g.args, &opts)?;
                // Its folder too (games that hand off to another program), unless that folder is too broad to mean anything.
                let dir = std::path::Path::new(&exe).parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
                let home = crate::paths::home().to_string_lossy().to_string();
                let broad = ["/", "/usr/bin", "/usr/local/bin", "/bin", "/opt", home.as_str()].contains(&dir.as_str()) || exe.ends_with(".desktop");
                crate::procs::Track { pid: Some(pid), dirs: if broad { vec![] } else { vec![dir] }, ..Default::default() }
            }
            "emu" => {
                let pid = crate::emulators::launch(g.exe.as_deref().unwrap_or(""), g.system_key.unwrap_or(""), &self.home)?;
                crate::procs::Track { pid: Some(pid), ..Default::default() }
            }
            _ => return Err("The Rust version of Hearth can't start this kind of game yet.".into()),
        };
        self.running = Some(Running { id: id.to_string(), track, launched_at: now_ms(), started_at: 0, gone_since: 0, paused_at: 0, paused_ms: 0, closing: false });
        Ok(serde_json::json!({"tracked": true}))
    }

    pub fn game_pids(&self) -> Vec<u32> {
        let Some(r) = &self.running else { return vec![] };
        crate::procs::game_processes(&r.track, &crate::procs::snapshot(std::path::Path::new("/proc")), std::process::id())
    }

    /// Looks at what's running (every 2 seconds). Returns events for the UI: started, gap, handoff, exited, timeout.
    pub fn tick(&mut self) -> Vec<Value> {
        let mut events = Vec::new();
        let pids = self.game_pids();
        let now = now_ms();
        let Some(r) = self.running.as_mut() else { return events };
        if r.started_at == 0 {
            if !pids.is_empty() { r.started_at = now; events.push(json!({"type": "event", "name": "game.started", "data": {"id": r.id}})); }
            else if now - r.launched_at > START_TIMEOUT_MS { events.push(json!({"type": "event", "name": "game.timeout", "data": {"id": r.id}})); self.running = None; }
        } else if pids.is_empty() {
            if r.closing || (r.gone_since > 0 && now - r.gone_since > EXIT_GRACE_MS) { events.push(self.exited()); }
            else if r.gone_since == 0 { r.gone_since = now; events.push(json!({"type": "event", "name": "game.gap", "data": {"id": r.id}})); }
        } else if r.gone_since > 0 {
            r.gone_since = 0;
            events.push(json!({"type": "event", "name": "game.handoff", "data": {"id": r.id}}));
        }
        events
    }

    /// The game is over: play time saved (paused time doesn't count), and the UI told.
    fn exited(&mut self) -> Value {
        let r = self.running.take().unwrap();
        if r.started_at > 0 {
            let paused = r.paused_ms + if r.paused_at > 0 { now_ms() - r.paused_at } else { 0 };
            let minutes = ((now_ms().saturating_sub(r.started_at).saturating_sub(paused)) as f64 / 60000.0).round() as u64;
            let id = r.id.clone();
            let _ = self.store.update(|d| {
                let stats = d.entry("stats").or_insert(json!({}));
                let s = stats.as_object_mut().unwrap().entry(id).or_insert(json!({"minutes": 0, "lastPlayed": 0}));
                s["minutes"] = json!(s["minutes"].as_u64().unwrap_or(0) + minutes);
                s["lastPlayed"] = json!(now_ms());
            });
        }
        let dto = self.games.get(&r.id).map(|g| self.dto(g)).unwrap_or(json!({"id": r.id}));
        json!({"type": "event", "name": "game.exited", "data": dto})
    }

    pub fn close_running(&mut self) -> bool {
        let pids = self.game_pids();
        match self.running.as_mut() { Some(r) => r.closing = true, None => return true }
        crate::procs::stop(&pids, std::time::Duration::from_secs(5))
    }
    pub fn pause(&mut self) -> Result<bool, String> {
        let pids = self.game_pids();
        let r = self.running.as_mut().filter(|r| r.started_at > 0).ok_or("No game is running.")?;
        if r.paused_at > 0 { return Ok(true); }
        if pids.is_empty() || crate::procs::signal(&pids, "STOP") == 0 { return Err("Couldn't pause the game.".into()); }
        r.paused_at = now_ms();
        Ok(true)
    }
    pub fn resume(&mut self) -> bool {
        let pids = self.game_pids();
        let Some(r) = self.running.as_mut().filter(|r| r.paused_at > 0) else { return false };
        crate::procs::signal(&pids, "CONT");
        r.paused_ms += now_ms() - r.paused_at;
        r.paused_at = 0;
        true
    }

    pub fn get(&mut self, rescan: bool) -> Value {
        if rescan || self.games.is_empty() { self.scan(); }
        let mut games: Vec<Value> = self.games.values().map(|g| self.dto(g)).collect();
        games.sort_by_key(|g| g["title"].as_str().unwrap_or("").to_lowercase());
        let collections: Vec<Value> = self.store.data["collections"].as_array().map(|a| a.iter()
            .map(|c| json!({"id": c["id"], "name": c["name"], "count": c["games"].as_array().map(|g| g.len()).unwrap_or(0)})).collect()).unwrap_or_default();
        json!({ "games": games, "sources": self.store.data["sources"], "collections": collections, "counts": self.counts,
                "runningId": self.running_id(), "steamBackground": self.store.data["steamBackground"] })
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
