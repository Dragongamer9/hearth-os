//! The library: every store's games merged with Hearth's own data (favorites, hidden, play time,
//! collections), in the shape the UI reads (the Electron host's Library.dto). Steam for now; the other
//! stores come over from the Electron host one by one.

use crate::steam;
use crate::store::Store;
use serde_json::{json, Value};
use crate::owned::Owned;
use crate::stores::{self, epic, gog, xcloud};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Default)]
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
    /// Xbox Cloud Gaming: streamed, not installed; opens this page.
    pub cloud: bool,
    pub stream_url: Option<String>,
    /// GOG games: the folder the game starts in.
    pub work_dir: Option<PathBuf>,
    /// Why Hearth can't install it ("Installs through EA app").
    pub note: Option<String>,
    /// How to install it: nimbus-epic:<app>, nimbus-gog:<id>, steam://install/<id>.
    pub install_uri: Option<String>,
    pub can_uninstall: bool,
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
    /// Xbox Cloud Gaming: it runs in the stream window, not here.
    pub cloud: bool,
    /// Quick Resume: when it was put aside (frozen, kept in memory), or 0.
    pub frozen_at: u64,
}

/// Quick Resume, like Xbox's: games put aside stay frozen in memory (no processor or graphics use) and carry on
/// where they were. At most this many; putting aside one more closes the oldest, as does running low on memory.
pub const QUICK_RESUME_MAX: usize = 5;
fn quick_file() -> PathBuf { PathBuf::from(std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into())).join("hearth-quick-resume.json") }
fn pids_of(track: &crate::procs::Track) -> Vec<u32> { crate::procs::game_processes(track, &crate::procs::snapshot(std::path::Path::new("/proc")), std::process::id()) }
fn rec(r: &Running) -> Value {
    json!({"id": r.id, "pid": r.track.pid, "dirs": r.track.dirs, "steamAppId": r.track.steam_app_id, "launchedAt": r.launched_at, "startedAt": r.started_at,
           "pausedAt": r.paused_at, "pausedMs": r.paused_ms, "frozenAt": r.frozen_at})
}
fn from_rec(v: &Value) -> Option<Running> {
    let track = crate::procs::Track { pid: v["pid"].as_u64().map(|p| p as u32), dirs: v["dirs"].as_array().map(|a| a.iter().filter_map(|d| d.as_str().map(str::to_string)).collect()).unwrap_or_default(),
        steam_app_id: v["steamAppId"].as_str().map(str::to_string) };
    Some(Running { id: v["id"].as_str()?.to_string(), track, launched_at: v["launchedAt"].as_u64().unwrap_or(0), started_at: v["startedAt"].as_u64().unwrap_or(0), gone_since: 0,
        paused_at: v["pausedAt"].as_u64().unwrap_or(0), paused_ms: v["pausedMs"].as_u64().unwrap_or(0), closing: false, cloud: false, frozen_at: v["frozenAt"].as_u64().unwrap_or(0) })
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
            bytes_done: g.bytes_done, bytes_total: g.bytes_total, art: g.art, exe: None, args: String::new(), system: None, system_key: None, ..Default::default()
        }
    }
}

/// A local picture goes to the UI through Hearth's own address (nimbus-art://file/<path>), so the page can
/// show it without reading files itself; web pictures pass straight through.
pub fn art_url(v: &str) -> String {
    if v.starts_with("http://") || v.starts_with("https://") { return crate::artcache::url(v); }
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
    /// Epic and GOG downloads Hearth runs itself.
    pub queue: Arc<Mutex<stores::Queue>>,
    pub accounts: Arc<Mutex<stores::AccState>>,
    queue_restored: bool,
    /// Steam download speeds, from how fast its byte count grows: id → (time, bytes, bytes a second)
    steam_bytes: HashMap<String, (u64, u64, u64)>,
    /// (time, bytes a second) for the Downloads page's graph: the last few minutes.
    speeds: Vec<(u64, u64)>,
    download_sig: String,
    /// Quick Resume: games put aside, newest first.
    pub frozen: Vec<Running>,
    quick_saved: String,
}

fn norm(s: &str) -> String { s.to_lowercase().chars().filter(|c| c.is_ascii_alphanumeric()).collect() }

impl Library {
    pub fn new(store: Store, home: PathBuf) -> Library {
        Library { running: None, store, games: BTreeMap::new(), counts: BTreeMap::new(), home, queue: Arc::new(Mutex::new(stores::Queue::new())),
                  accounts: Arc::new(Mutex::new(stores::AccState::default())), queue_restored: false, steam_bytes: HashMap::new(), speeds: vec![], download_sig: String::new(), frozen: vec![], quick_saved: String::new() }
    }

    /// Games you added yourself (kept in library.json "manual").
    pub fn manual_games(&self) -> Vec<Game> {
        self.store.data["manual"].as_array().cloned().unwrap_or_default().iter().filter_map(|m| {
            let id = m["id"].as_str()?.to_string();
            let path = m["path"].as_str()?.to_string();
            Some(Game { id, title: m["title"].as_str().unwrap_or("Game").to_string(), source: "manual", install_path: Some(path.clone().into()),
                size_bytes: None, added: m["added"].as_u64().unwrap_or(0), store_minutes: 0, store_last_played: 0, installed: true, state: "installed",
                bytes_done: None, bytes_total: None, art: vec![], launch_uri: None, exe: Some(path), args: m["args"].as_str().unwrap_or("").to_string(), system: None, system_key: None, ..Default::default() })
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
        // Games Hearth installed itself (legendary, gogdl), and Xbox Cloud Gaming's catalog.
        if self.store.source_on("epic") {
            let list = epic::installed(&Owned::load(epic::CACHE));
            counts.insert("epic".to_string(), list.len());
            for g in list { all.entry(g.id.clone()).or_insert(g); }
        }
        if self.store.source_on("gog") {
            let list = gog::installed(&self.store.data["gogInstalls"], &Owned::load(gog::CACHE));
            counts.insert("gog".to_string(), list.len());
            for g in list { all.entry(g.id.clone()).or_insert(g); }
        }
        if self.store.source_on("xbox") {
            let list = xcloud::scan();
            counts.insert("xbox".to_string(), list.len().saturating_sub(1));
            for g in list { all.entry(g.id.clone()).or_insert(g); }
        }
        // Xbox PC games through Xodus Nexus, once you've added it (Settings › Games › Library).
        if let Some(g) = crate::xodus::game() { counts.insert("xodus".to_string(), 1); all.insert(g.id.clone(), g); }
        let manual = self.manual_games();
        counts.insert("manual".to_string(), manual.len());
        for g in manual { all.entry(g.id.clone()).or_insert(g); }
        // Games on your Epic and GOG accounts that aren't installed: shown with an Install button.
        for s in ["epic", "gog"] {
            if !self.store.source_on(s) || !stores::signed_in(s) { continue; }
            for o in Owned::load(stores::cache_of(s)).games {
                let Some(oid) = o["id"].as_str() else { continue };
                let id = format!("{s}:{oid}");
                if all.contains_key(&id) { continue; }
                let mut g = stores::game(id.clone(), o["title"].as_str().unwrap_or(oid), s, "", None, stores::art_of(&o["art"]), false);
                g.added = o["added"].as_u64().unwrap_or(0);
                g.store_last_played = o["lastPlayed"].as_u64().unwrap_or(0);
                g.note = o["note"].as_str().map(str::to_string);
                g.install_uri = if g.note.is_some() { None } else { Some(format!("nimbus-{s}:{oid}")) };
                all.insert(id, g);
            }
        }
        // Cloud games you've played on Xbox: when you last played them.
        if stores::signed_in("xbox") {
            let played: HashMap<String, u64> = Owned::load(stores::xbox::CACHE).games.iter().map(|o| (norm(o["title"].as_str().unwrap_or("")), o["lastPlayed"].as_u64().unwrap_or(0))).collect();
            for g in all.values_mut() { if g.cloud { if let Some(t) = played.get(&norm(&g.title)) { g.store_last_played = *t; } } }
        }
        self.games = all;
        self.counts = counts;
        if !self.queue_restored { self.queue_restored = true; self.restore_queue(); }
        self.download_sig = self.signature();
    }

    // ------------------------------------------------------------------ installs (Epic, GOG) and downloads

    pub fn install_root(&self) -> PathBuf {
        self.store.data["installRoot"].as_str().filter(|s| !s.is_empty()).map(PathBuf::from).unwrap_or_else(|| self.home.join("Games"))
    }

    fn start_install(&mut self, id: &str, title: &str, uri: &str, paused: bool) -> Result<(), String> {
        let store = if uri.starts_with("nimbus-epic:") { "epic" } else if uri.starts_with("nimbus-gog:") { "gog" } else { return Err("Hearth can't install this game itself.".into()) };
        if !stores::signed_in(store) { return Err(format!("Sign in to {} first (Library › Stores).", if store == "epic" { "Epic Games" } else { "GOG" })); }
        let job = stores::Job { game_id: id.into(), title: title.into(), uri: uri.into(), root: self.install_root(), paused,
            state: if paused { "paused".into() } else { "queued".into() }, ..Default::default() };
        self.queue.lock().unwrap().add(job)?;
        self.save_queue();
        stores::pump(self.queue.clone());
        Ok(())
    }

    /// The download queue is kept in library.json, so a restart doesn't lose it.
    pub fn save_queue(&mut self) {
        let q: Vec<Value> = self.queue.lock().unwrap().jobs.iter().map(|(j, _)| { let j = j.lock().unwrap();
            json!({"gameId": j.game_id, "title": j.title, "uri": j.uri, "paused": j.paused, "bytesDone": j.bytes_done, "bytesTotal": j.bytes_total, "progress": j.progress}) }).collect();
        let _ = self.store.update(|d| { d.insert("installQueue".into(), json!(q)); });
    }

    /// After a restart: the saved downloads go back in the queue (the tools carry on from the files already there).
    fn restore_queue(&mut self) {
        for e in self.store.data["installQueue"].as_array().cloned().unwrap_or_default() {
            let Some(id) = e["gameId"].as_str() else { continue };
            if self.queue.lock().unwrap().get(id).is_some() { continue; }
            let Some(g) = self.games.get(id).cloned() else { continue };
            if g.installed { continue; }
            let uri = g.install_uri.clone().or(e["uri"].as_str().map(str::to_string)).unwrap_or_default();
            match self.start_install(id, &g.title, &uri, e["paused"] == true) {
                Ok(()) => {
                    if let Some(j) = self.queue.lock().unwrap().get(id) { let mut j = j.lock().unwrap(); j.bytes_done = e["bytesDone"].as_u64(); j.bytes_total = e["bytesTotal"].as_u64(); j.progress = e["progress"].as_f64(); }
                    crate::bridge::log(&format!("install {id}: back in the queue after a restart"));
                }
                Err(err) => crate::bridge::log(&format!("install {id}: couldn't restore: {err}")),
            }
        }
    }

    pub fn install(&mut self, id: &str) -> Result<(), String> {
        let g = self.games.get(id).cloned().ok_or("That game isn't in the library anymore.")?;
        let Some(uri) = g.install_uri.clone() else {
            return Err(match g.note { Some(n) => format!("{n}. Hearth can't install it."), None => "Hearth can't install this game. Install it from its store app.".into() });
        };
        if uri.starts_with("nimbus-") { return self.start_install(id, &g.title, &uri, false); }
        std::process::Command::new("steam").arg(&uri).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null())
            .spawn().map(|_| ()).map_err(|e| format!("Couldn't open Steam: {e}"))
    }

    /// (found, removed now): a waiting download is removed at once.
    pub fn cancel_install(&mut self, id: &str) -> (bool, bool) {
        let r = self.queue.lock().unwrap().cancel(id);
        if r.1 { self.save_queue(); }
        r
    }
    pub fn pause_install(&mut self, id: &str) -> bool { let ok = self.queue.lock().unwrap().pause(id); if ok { self.save_queue(); } ok }
    pub fn resume_install(&mut self, id: &str) -> bool {
        let ok = self.queue.lock().unwrap().resume(id);
        if ok { self.save_queue(); stores::pump(self.queue.clone()); }
        ok
    }
    pub fn install_first(&mut self, id: &str) -> bool { let ok = self.queue.lock().unwrap().first(id); if ok { self.save_queue(); } ok }

    /// A download finished (or failed, or was cancelled).
    pub fn install_done(&mut self, id: &str, error: Option<&str>, gog_dir: Option<&str>) -> Value {
        if let (Some(dir), None) = (gog_dir, error) {
            let gid = id.trim_start_matches("gog:").to_string();
            let _ = self.store.update(|d| { let m = d.entry("gogInstalls").or_insert(json!({})); if let Some(o) = m.as_object_mut() { o.insert(gid, json!(dir)); } });
        }
        self.save_queue();
        let title = self.games.get(id).map(|g| g.title.clone()).unwrap_or_else(|| id.to_string());
        if error.is_none() { self.record_installed(id); self.scan(); }
        json!({"id": id, "title": title, "error": error})
    }

    pub fn uninstall(&mut self, id: &str) -> Result<(), String> {
        let g = self.games.get(id).cloned().ok_or("That game isn't in the library anymore.")?;
        if self.running.as_ref().map(|r| r.id == id).unwrap_or(false) { return Err(format!("Exit {} first.", g.title)); }
        if !g.can_uninstall { return Err(format!("Uninstall {} from its store app.", g.title)); }
        match g.source {
            "epic" => epic::uninstall(id.trim_start_matches("epic:"))?,
            "gog" => {
                let gid = id.trim_start_matches("gog:").to_string();
                // Only ever delete a folder that is clearly this game's (it holds GOG's goggame-<id>.info).
                if let Some(dir) = self.store.data["gogInstalls"][&gid].as_str() {
                    let dir = std::path::Path::new(dir);
                    if dir.join(format!("goggame-{gid}.info")).exists() { std::fs::remove_dir_all(dir).map_err(|e| format!("Couldn't delete the game's folder: {e}"))?; }
                }
                let _ = self.store.update(|d| { if let Some(o) = d.get_mut("gogInstalls").and_then(|v| v.as_object_mut()) { o.remove(&gid); } });
                stores::remove_prefix("gog", &gid);
            }
            _ => return Err(format!("Uninstall {} from its store app.", g.title)),
        }
        self.scan();
        Ok(())
    }

    fn signature(&self) -> String {
        let mut parts: Vec<String> = self.games.values().filter(|g| !["installed", "notInstalled", "cloud"].contains(&g.state)).map(|g| format!("{}={}", g.id, g.state)).collect();
        for (j, _) in self.queue.lock().unwrap().jobs.iter() { let j = j.lock().unwrap(); parts.push(format!("{}={}", j.game_id, if j.state == "queued" { "queued" } else { "active" })); }
        parts.join(";")
    }

    /// What's downloading (Steam's own downloads and Hearth's), whether that changed, the speed graph and
    /// what finished lately. The Downloads page asks every couple of seconds.
    pub fn downloads(&mut self) -> Value {
        if self.store.source_on("steam") {
            let manifests: Vec<Game> = steam::scan(&self.home).into_iter().map(Game::from).collect();
            let fresh: std::collections::HashSet<String> = manifests.iter().map(|m| m.id.clone()).collect();
            // A cancelled Steam download removes its manifest: gone until the next scan.
            self.games.retain(|id, g| g.source != "steam" || ["installed", "notInstalled"].contains(&g.state) || fresh.contains(id));
            for m in manifests { self.games.insert(m.id.clone(), m); }
        }
        let sig = self.signature();
        let changed = sig != self.download_sig;
        self.download_sig = sig;
        let mut active: Vec<Value> = self.games.values().filter(|g| !["installed", "notInstalled", "cloud"].contains(&g.state)).map(|g| self.dto(g)).collect();
        for id in self.queue.lock().unwrap().ids() {
            if let Some(g) = self.games.get(&id) { if !active.iter().any(|a| a["id"] == id.as_str()) { active.push(self.dto(g)); } }
        }
        let now = now_ms();
        for a in active.iter_mut() {
            if a["source"] != "steam" { continue; }
            let (Some(id), Some(bytes)) = (a["id"].as_str().map(str::to_string), a["bytesDone"].as_u64()) else { continue };
            let prev = self.steam_bytes.get(&id).copied();
            let mut bps = 0;
            if let Some((t, b, _)) = prev { if now > t && bytes >= b { bps = ((bytes - b) as f64 / ((now - t) as f64 / 1000.0)).round() as u64; } }
            match prev {
                Some((t, b, old)) if bytes == b && now - t <= 20000 => bps = old,
                _ => { self.steam_bytes.insert(id.clone(), (now, bytes, bps)); }
            }
            a["bytesPerSec"] = json!(bps);
        }
        let gone: Vec<String> = self.steam_bytes.keys().filter(|id| !active.iter().any(|a| a["id"] == id.as_str())).cloned().collect();
        for id in gone {
            self.steam_bytes.remove(&id);
            if self.games.get(&id).map(|g| g.installed).unwrap_or(false) { self.record_installed(&id); }
        }
        let bps: u64 = active.iter().filter(|a| a["state"] == "downloading" || a["state"] == "updating").map(|a| a["bytesPerSec"].as_u64().unwrap_or(0)).sum();
        self.speeds.push((now, bps));
        self.speeds.retain(|(t, _)| now - t < 5 * 60 * 1000);
        json!({"active": active, "changed": changed, "speeds": self.speeds.iter().map(|(t, b)| json!([t, b])).collect::<Vec<_>>(), "recent": self.recent_installs()})
    }

    /// Remembered across restarts: what finished installing lately (the Downloads page lists them).
    fn record_installed(&mut self, id: &str) {
        let _ = self.store.update(|d| {
            let mut h: Vec<Value> = d.get("installHistory").and_then(|v| v.as_array()).cloned().unwrap_or_default();
            h.retain(|e| e["id"] != id);
            h.insert(0, json!({"id": id, "at": now_ms()}));
            h.truncate(20);
            d.insert("installHistory".into(), json!(h));
        });
    }
    fn recent_installs(&self) -> Vec<Value> {
        let since = now_ms().saturating_sub(14 * 24 * 3600 * 1000);
        self.store.data["installHistory"].as_array().cloned().unwrap_or_default().into_iter()
            .filter(|e| e["at"].as_u64().unwrap_or(0) > since && e["id"].as_str().and_then(|id| self.games.get(id)).map(|g| g.installed).unwrap_or(false)).collect()
    }
    pub fn clear_recent(&mut self) { let _ = self.store.update(|d| { d.insert("installHistory".into(), json!([])); }); }

    pub fn dto(&self, g: &Game) -> Value {
        let d = &self.store.data;
        let stats = &d["stats"][&g.id];
        let art: serde_json::Map<String, Value> = g.art.iter().map(|(k, v)| (k.to_string(), Value::String(art_url(v)))).collect();
        let cols: Vec<Value> = d["collections"].as_array().map(|a| a.iter()
            .filter(|c| c["games"].as_array().map(|g2| g2.iter().any(|x| x == g.id.as_str())).unwrap_or(false))
            .map(|c| c["id"].clone()).collect()).unwrap_or_default();
        let mut out = json!({
            "id": g.id, "title": g.title, "source": g.source,
            "fav": self.store.list_has("favorites", &g.id), "hidden": self.store.list_has("hidden", &g.id), "cols": cols,
            "minutes": g.store_minutes.max(stats["minutes"].as_u64().unwrap_or(0)),
            "lastPlayed": g.store_last_played.max(stats["lastPlayed"].as_u64().unwrap_or(0)),
            "added": g.added, "sizeBytes": g.size_bytes, "art": art,
            "path": if g.installed { g.install_path.as_ref().map(|p| p.to_string_lossy().to_string()) } else { None },
            "canRemove": g.source == "manual", "installed": g.installed, "state": g.state, "cloud": g.cloud,
            "progress": match (g.bytes_done, g.bytes_total) { (Some(a), Some(b)) if b > 0 => Some((a as f64 / b as f64).min(1.0)), _ => None },
            "bytesDone": g.bytes_done, "bytesTotal": g.bytes_total, "canUninstall": g.can_uninstall, "note": g.note,
            "installKind": match g.install_uri.as_deref() { None => None, Some(u) if u.starts_with("nimbus-") => Some("nimbus"), Some(u) if u.starts_with("steam:") => Some("steam"), Some(_) => Some("store") },
            "system": g.system, "frozen": self.frozen.iter().any(|f| f.id == g.id),
        });
        // An install Hearth is running shows as a download.
        if let Some(Value::Object(j)) = self.queue.lock().unwrap().state(&g.id) { for (k, v) in j { out[k] = v; } }
        out
    }

    pub fn running_id(&self) -> Option<String> { self.running.as_ref().filter(|r| r.started_at > 0).map(|r| r.id.clone()) }
    pub fn running_since(&self) -> Option<u64> { self.running.as_ref().filter(|r| r.started_at > 0).map(|r| r.started_at) }

    /// Starts a game. {tracked} says whether Hearth can watch it (it then shows as running).
    pub fn launch(&mut self, id: &str) -> Result<Value, String> {
        let g = self.games.get(id).cloned().ok_or("That game isn't in the library anymore. Try rescanning.")?;
        // Put aside with Quick Resume: carry on where it was.
        if self.frozen.iter().any(|f| f.id == id) { return self.quick_resume(id).map(|_| json!({"tracked": true, "resumed": true})); }
        if self.running.as_ref().map(|r| r.id == id).unwrap_or(false) { return Ok(json!({"tracked": true, "resumed": true})); }
        if self.running.is_some() {
            if !self.quick_resume_on() { return Err("Another game is already running".into()); }
            self.freeze_running()?;
        }
        if g.cloud {
            // The host opens the stream window; Hearth counts it as running from now.
            self.running = Some(Running { id: id.to_string(), track: Default::default(), launched_at: now_ms(), started_at: now_ms(), gone_since: 0, paused_at: 0, paused_ms: 0, closing: false, cloud: true, frozen_at: 0 });
            return Ok(json!({"tracked": true, "stream": g.stream_url, "title": g.title}));
        }
        if !g.installed { return Err(format!("{} isn't installed yet.", g.title)); }
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
            "epic" | "gog" if g.can_uninstall => {
                // Installed by Hearth: started directly with Proton, so Hearth can watch it.
                let opts = crate::game_options::get(&self.store, id);
                let pid = if g.source == "epic" { epic::launch(id.trim_start_matches("epic:"), &opts)? } else { gog::launch(&g, &opts)? };
                let dirs = g.install_path.as_ref().map(|p| vec![p.to_string_lossy().to_string()]).unwrap_or_default();
                crate::procs::Track { pid: Some(pid), dirs, ..Default::default() }
            }
            _ => return Err("Hearth doesn't know how to start this game.".into()),
        };
        self.running = Some(Running { id: id.to_string(), track, launched_at: now_ms(), started_at: 0, gone_since: 0, paused_at: 0, paused_ms: 0, closing: false, cloud: false, frozen_at: 0 });
        Ok(serde_json::json!({"tracked": true}))
    }

    /// Processes of the games put aside (Quick Resume): their windows stay around, frozen on their last picture,
    /// and must never be the one put in front for another game.
    pub fn frozen_pids(&self) -> Vec<u32> {
        if self.frozen.is_empty() { return vec![]; }
        let snap = crate::procs::snapshot(std::path::Path::new("/proc"));
        let running = self.game_pids();
        self.frozen.iter().flat_map(|f| crate::procs::game_processes(&f.track, &snap, std::process::id())).filter(|p| !running.contains(p)).collect()
    }
    pub fn game_pids(&self) -> Vec<u32> {
        let Some(r) = &self.running else { return vec![] };
        crate::procs::game_processes(&r.track, &crate::procs::snapshot(std::path::Path::new("/proc")), std::process::id())
    }

    /// Looks at what's running (every 2 seconds). Returns events for the UI: started, gap, handoff, exited, timeout.
    pub fn tick(&mut self) -> Vec<Value> {
        let before = self.frozen.len();
        let mut events = self.tick_running();
        let mut q = self.quick_tick();
        // The list only goes to the page when it changed.
        if self.frozen.len() == before && q.last().map(|e| e["name"] == "quickResume").unwrap_or(false) { q.pop(); }
        events.extend(q);
        events
    }

    fn tick_running(&mut self) -> Vec<Value> {
        let mut events = Vec::new();
        let pids = self.game_pids();
        let now = now_ms();
        let Some(r) = self.running.as_mut() else { return events };
        if r.cloud { return events; }
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
    pub fn exited(&mut self) -> Value {
        let r = self.running.take().unwrap();
        let dto = self.finish(r);
        json!({"type": "event", "name": "game.exited", "data": dto})
    }

    /// A game is over (or a Quick Resume game was closed): play time saved, Epic saves uploaded. Returns its dto.
    fn finish(&mut self, r: Running) -> Value {
        // Epic cloud saves go up once the game has closed.
        // GOG cloud saves go up once the game has closed.
        if let Some(gid) = r.id.strip_prefix("gog:") {
            if crate::game_options::get(&self.store, &r.id)["cloudSaves"] != false {
                if let Some(dir) = self.store.data["gogInstalls"][gid].as_str().map(PathBuf::from) {
                    let gid = gid.to_string();
                    std::thread::spawn(move || { if let Err(e) = gog::sync_saves(&gid, &dir, "up") { crate::bridge::log(&format!("gog saves: {e}")); } });
                }
            }
        }
        if let Some(app) = r.id.strip_prefix("epic:") {
            if crate::game_options::get(&self.store, &r.id)["cloudSaves"] != false {
                let app = app.to_string();
                std::thread::spawn(move || { if let Err(e) = epic::sync_saves(&app, "up") { crate::bridge::log(&format!("epic saves: {e}")); } });
            }
        }
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
        self.games.get(&r.id).map(|g| self.dto(g)).unwrap_or(json!({"id": r.id}))
    }

    // ------------------------------------------------------------------ Quick Resume

    pub fn quick_resume_on(&self) -> bool { self.store.data["quickResume"] != false }

    /// The running game goes aside: frozen where it is, kept in memory.
    pub fn freeze_running(&mut self) -> Result<(), String> {
        let r = self.running.as_ref().ok_or("No game is running.")?;
        if r.cloud { return Err("Close Xbox Cloud Gaming first: cloud games can't be put aside.".into()); }
        if r.started_at == 0 { return Err(format!("{} is still starting. Wait a moment, then try again.", self.games.get(&r.id).map(|g| g.title.as_str()).unwrap_or("The game"))); }
        let pids = self.game_pids();
        if pids.is_empty() || crate::procs::signal(&pids, "STOP") == 0 { return Err("Couldn't put the game aside.".into()); }
        let mut r = self.running.take().unwrap();
        let now = now_ms();
        if r.paused_at == 0 { r.paused_at = now; }
        r.frozen_at = now; r.gone_since = 0; r.closing = false;
        crate::bridge::log(&format!("quick resume: {} put aside", r.id));
        self.frozen.insert(0, r);
        while self.frozen.len() > QUICK_RESUME_MAX { let old = self.frozen.pop().unwrap(); self.close_frozen_entry(old); }
        self.save_quick();
        Ok(())
    }

    /// Back to a game that was put aside (the running one, if any, goes aside in its place).
    pub fn quick_resume(&mut self, id: &str) -> Result<(), String> {
        let pos = self.frozen.iter().position(|f| f.id == id).ok_or("That game isn't in Quick Resume anymore.")?;
        if self.running.is_some() { self.freeze_running()?; }
        let pos = self.frozen.iter().position(|f| f.id == id).unwrap_or(pos);
        let mut r = self.frozen.remove(pos);
        crate::procs::signal(&pids_of(&r.track), "CONT");
        let now = now_ms();
        if r.paused_at > 0 { r.paused_ms += now.saturating_sub(r.paused_at); r.paused_at = 0; }
        r.frozen_at = 0;
        crate::bridge::log(&format!("quick resume: back to {id}"));
        self.running = Some(r);
        self.save_quick();
        Ok(())
    }

    /// Closes a game that was put aside. Returns its dto.
    pub fn close_frozen(&mut self, id: &str) -> Result<Value, String> {
        let pos = self.frozen.iter().position(|f| f.id == id).ok_or("That game isn't in Quick Resume anymore.")?;
        let r = self.frozen.remove(pos);
        let dto = self.close_frozen_entry(r);
        self.save_quick();
        Ok(dto)
    }
    fn close_frozen_entry(&mut self, r: Running) -> Value {
        let pids = pids_of(&r.track);
        crate::procs::signal(&pids, "CONT");   // stopped processes only act on "please close" once they run again
        crate::procs::stop(&pids, std::time::Duration::from_secs(5));
        crate::bridge::log(&format!("quick resume: closed {}", r.id));
        self.finish(r)
    }

    /// What Quick Resume holds, newest first: [{id, title, frozenAt}]
    pub fn quick_list(&self) -> Value {
        json!(self.frozen.iter().map(|f| json!({"id": f.id, "title": self.games.get(&f.id).map(|g| g.title.clone()).unwrap_or_else(|| f.id.clone()), "frozenAt": f.frozen_at})).collect::<Vec<_>>())
    }
    pub fn quick_state(&self) -> Value { json!({"runningId": self.running_id(), "list": self.quick_list(), "on": self.quick_resume_on()}) }

    /// Kept while the device is on (in the runtime folder), so Quick Resume games are still there after Hearth
    /// restarts (an update, a crash): the games keep running without Hearth.
    pub fn save_quick(&mut self) {
        let v = json!({"running": self.running.as_ref().filter(|r| !r.cloud && r.started_at > 0).map(rec), "frozen": self.frozen.iter().map(rec).collect::<Vec<_>>()});
        let t = v.to_string();
        if t != self.quick_saved { let _ = std::fs::write(quick_file(), &t); self.quick_saved = t; }
    }
    /// After Hearth starts again: the games that are still there.
    pub fn restore_quick(&mut self) {
        let Some(v) = std::fs::read(quick_file()).ok().and_then(|b| serde_json::from_slice::<Value>(&b).ok()) else { return };
        if self.running.is_none() {
            if let Some(r) = from_rec(&v["running"]).filter(|r| !pids_of(&r.track).is_empty()) { crate::bridge::log(&format!("quick resume: {} still running after a restart", r.id)); self.running = Some(r); }
        }
        for f in v["frozen"].as_array().cloned().unwrap_or_default() {
            if let Some(r) = from_rec(&f).filter(|r| !pids_of(&r.track).is_empty() && !self.frozen.iter().any(|x| x.id == r.id)) { self.frozen.push(r); }
        }
        self.save_quick();
    }

    /// Every couple of seconds: games put aside that were closed some other way leave the list, and when memory
    /// runs low the oldest one is closed to make room (like Xbox).
    pub fn quick_tick(&mut self) -> Vec<Value> {
        let mut events = Vec::new();
        if self.frozen.is_empty() { self.save_quick(); return events; }
        let snap = crate::procs::snapshot(std::path::Path::new("/proc"));
        let me = std::process::id();
        let gone: Vec<usize> = self.frozen.iter().enumerate().filter(|(_, f)| crate::procs::game_processes(&f.track, &snap, me).is_empty()).map(|(i, _)| i).collect();
        for i in gone.into_iter().rev() { let r = self.frozen.remove(i); crate::bridge::log(&format!("quick resume: {} was closed", r.id)); self.finish(r); }
        let mem = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
        let kb = |k: &str| mem.lines().find(|l| l.starts_with(&format!("{k}:"))).and_then(|l| l.split_whitespace().nth(1)).and_then(|n| n.parse::<u64>().ok()).unwrap_or(0);
        let (avail, total) = (kb("MemAvailable"), kb("MemTotal"));
        if total > 0 && avail < (total / 14).max(1_000_000) {
            if let Some(old) = self.frozen.pop() {
                let title = self.games.get(&old.id).map(|g| g.title.clone()).unwrap_or_else(|| old.id.clone());
                crate::bridge::log(&format!("quick resume: low on memory ({} MB left), closing {}", avail / 1024, old.id));
                self.close_frozen_entry(old);
                events.push(json!({"type": "event", "name": "osd", "data": {"kind": "msg", "icon": "clock", "text": format!("Closed {title} to make room in memory (Quick Resume)"), "ms": 5000}}));
            }
        }
        self.save_quick();
        events.push(json!({"type": "event", "name": "quickResume", "data": self.quick_state()}));
        events
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
                "runningId": self.running_id(), "steamBackground": self.store.data["steamBackground"], "quickResume": self.quick_state() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn local_pictures_get_hearths_address() {
        assert_eq!(art_url("/home/sam/a b/620p.png"), "nimbus-art://file//home/sam/a%20b/620p.png");
        assert_eq!(art_url("https://cdn.x/1.jpg"), "https://cdn.x/1.jpg");   // until it's downloaded
    }
    #[test]
    fn quick_resume_puts_games_aside_and_back() {
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_RUNTIME_DIR", dir.path());
        let mut lib = Library::new(Store::open(dir.path().join("l.json")), dir.path().to_path_buf());
        let state = |pid: u32| std::fs::read_to_string(format!("/proc/{pid}/stat")).unwrap_or_default().split(") ").nth(1).and_then(|r| r.chars().next()).unwrap_or('?');
        let mut a = std::process::Command::new("sleep").arg("60").spawn().unwrap();
        let mut b = std::process::Command::new("sleep").arg("60").spawn().unwrap();
        let run = |id: &str, pid: u32| Running { id: id.into(), track: crate::procs::Track { pid: Some(pid), ..Default::default() }, launched_at: 1, started_at: now_ms(), gone_since: 0, paused_at: 0, paused_ms: 0, closing: false, cloud: false, frozen_at: 0 };
        lib.running = Some(run("manual:a", a.id()));
        lib.freeze_running().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));
        assert!(lib.running.is_none());
        assert_eq!(lib.quick_list().as_array().unwrap().len(), 1);
        assert_eq!(state(a.id()), 'T');
        // Another game runs; going back to the first puts this one aside in its place.
        lib.running = Some(run("manual:b", b.id()));
        lib.quick_resume("manual:a").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));
        assert_eq!(lib.running.as_ref().unwrap().id, "manual:a");
        assert_ne!(state(a.id()), 'T');
        assert_eq!(state(b.id()), 'T');
        // Saved for after a restart of Hearth, and read back.
        let mut again = Library::new(Store::open(dir.path().join("l2.json")), dir.path().to_path_buf());
        again.restore_quick();
        assert_eq!(again.running.as_ref().map(|r| r.id.as_str()), Some("manual:a"));
        assert_eq!(again.frozen.len(), 1);
        // Closing a put-aside game ends it.
        lib.close_frozen("manual:b").unwrap();
        assert!(lib.frozen.is_empty());
        let _ = b.wait();
        let _ = a.kill(); let _ = a.wait();
    }
}
