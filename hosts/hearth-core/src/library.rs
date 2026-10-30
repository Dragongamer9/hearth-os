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
    /// Beacon's games (beacon.rs): what Beacon says about it, its mods.
    pub beacon: Option<crate::beacon::BeaconGame>,
    /// Steam downloads: what Steam is doing now ("Verifying", "Waiting for Steam", "Stopped: Disk write failure").
    pub stage: Option<String>,
    /// Multi-seat (seat_share.rs): another player's Epic or GOG install, played from their folder ("Player 1").
    pub shared_from: Option<String>,
    /// Multi-seat: a Steam game another Steam account installed in the shared library (Steam Family lends it).
    pub borrowed: bool,
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
    /// Its number (appid.rs): what its processes and windows carry, and what gamescope is asked to show.
    pub app_id: u32,
}

/// Quick Resume, like Xbox's: games put aside stay frozen in memory (no processor or graphics use) and carry on
/// where they were. At most this many; putting aside one more closes the oldest, as does running low on memory.
pub const QUICK_RESUME_MAX: usize = 5;
fn quick_file() -> PathBuf { PathBuf::from(std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into())).join("hearth-quick-resume.json") }
fn proc_root() -> &'static std::path::Path { std::path::Path::new("/proc") }
fn pids_of(track: &crate::procs::Track) -> Vec<u32> { crate::procs::game_processes(track, &crate::procs::snapshot(proc_root()), std::process::id()) }
fn rec(r: &Running) -> Value {
    json!({"id": r.id, "pid": r.track.pid, "dirs": r.track.dirs, "steamAppId": r.track.steam_app_id, "appId": r.app_id, "launchedAt": r.launched_at, "startedAt": r.started_at,
           "pausedAt": r.paused_at, "pausedMs": r.paused_ms, "frozenAt": r.frozen_at})
}
fn from_rec(v: &Value) -> Option<Running> {
    let app_id = v["appId"].as_u64().filter(|n| *n > 0 && *n <= u32::MAX as u64).map(|n| n as u32);
    let track = crate::procs::Track { pid: v["pid"].as_u64().map(|p| p as u32), dirs: v["dirs"].as_array().map(|a| a.iter().filter_map(|d| d.as_str().map(str::to_string)).collect()).unwrap_or_default(),
        steam_app_id: v["steamAppId"].as_str().map(str::to_string), app_id };
    Some(Running { id: v["id"].as_str()?.to_string(), track, launched_at: v["launchedAt"].as_u64().unwrap_or(0), started_at: v["startedAt"].as_u64().unwrap_or(0), gone_since: 0,
        paused_at: v["pausedAt"].as_u64().unwrap_or(0), paused_ms: v["pausedMs"].as_u64().unwrap_or(0), closing: false, cloud: false, frozen_at: v["frozenAt"].as_u64().unwrap_or(0),
        app_id: app_id.unwrap_or(0) })
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
    /// Beacon (beacon.rs): what it said last; a newer reading made in the background, waiting to be used;
    /// whether one is being made; when its files were last looked at.
    pub beacon: crate::beacon::Snapshot,
    beacon_loaded: bool,
    beacon_next: Arc<Mutex<Option<crate::beacon::Snapshot>>>,
    beacon_busy: Arc<std::sync::atomic::AtomicBool>,
    beacon_checked: u64,
    /// Multi-seat (seat_share.rs): games running on the other seats (id → "Player 2"), and when that was read;
    /// what this player last offered the others; whether Steam has the shared library yet.
    in_use: HashMap<String, String>,
    in_use_checked: u64,
    share_sig: Option<String>,
    steam_share_done: bool,
}

fn norm(s: &str) -> String { s.to_lowercase().chars().filter(|c| c.is_ascii_alphanumeric()).collect() }

impl Library {
    pub fn new(store: Store, home: PathBuf) -> Library {
        Library { running: None, store, games: BTreeMap::new(), counts: BTreeMap::new(), home, queue: Arc::new(Mutex::new(stores::Queue::new())),
                  accounts: Arc::new(Mutex::new(stores::AccState::default())), queue_restored: false, steam_bytes: HashMap::new(), speeds: vec![], download_sig: String::new(), frozen: vec![], quick_saved: String::new(),
                  beacon: Default::default(), beacon_loaded: false, beacon_next: Arc::new(Mutex::new(None)), beacon_busy: Arc::new(std::sync::atomic::AtomicBool::new(false)), beacon_checked: 0,
                  in_use: HashMap::new(), in_use_checked: 0, share_sig: None, steam_share_done: false }
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

}

/// Wabbajack modlists installed by Hearth Mods: one JSON per list in mods/modlists/ (id, title, launch: a .desktop
/// file that starts its Mod Organizer 2 in its own Proton prefix, folder). Started like a game you added yourself.
pub fn modlist_games() -> Vec<Game> {
    let dir = crate::paths::data_dir().join("mods").join("modlists");
    let Ok(rd) = std::fs::read_dir(&dir) else { return vec![] };
    let mut out = Vec::new();
    for e in rd.flatten() {
        let p = e.path();
        if p.extension().map(|x| x != "json").unwrap_or(true) { continue; }
        let Ok(v) = std::fs::read(&p).map(|b| serde_json::from_slice::<Value>(&b).unwrap_or(Value::Null)) else { continue };
        let (Some(id), Some(launch)) = (v["id"].as_str(), v["launch"].as_str()) else { continue };
        if !std::path::Path::new(launch).is_file() { continue; }
        out.push(Game { id: format!("modlist:{id}"), title: v["title"].as_str().unwrap_or("Modlist").to_string(), source: "manual",
            install_path: v["folder"].as_str().map(PathBuf::from), added: v["added"].as_u64().unwrap_or(0), installed: true, state: "installed",
            exe: Some(launch.to_string()), note: v["note"].as_str().map(str::to_string), ..Default::default() });
    }
    out
}

impl Library {
    /// A scan that starts nothing: no download is put back in the queue (`hearth --list-games`, desktop_games.rs).
    pub fn scan_quietly(&mut self) { self.queue_restored = true; self.scan(); }

    pub fn scan(&mut self) {
        let mut all = BTreeMap::new();
        let mut counts = BTreeMap::new();
        // Multi-seat with shared games: its shared folder (seat_share.rs).
        let share = crate::seat_share::active();
        if self.store.source_on("steam") {
            // A game in the shared folder's Steam library that another Steam account installed is lent by Steam Family.
            let me64 = share.as_ref().and_then(|_| steam::steam_path(&self.home)).and_then(|s| steam::find_account(&self.home, &s, false).ok()).map(|(id, _)| id);
            let list: Vec<Game> = steam::scan(&self.home).into_iter().map(|sg| {
                let lent = share.as_ref().is_some_and(|(_, d)| sg.install_path.starts_with(d)) && sg.owner != 0 && me64.is_some_and(|m| m != sg.owner);
                let mut g = Game::from(sg);
                g.borrowed = lent;
                g
            }).collect();
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
        if let Some((cfg, dir)) = &share { self.add_shared(&mut all, cfg, dir); }
        if self.store.source_on("xbox") {
            let list = xcloud::scan();
            counts.insert("xbox".to_string(), list.len().saturating_sub(1));
            for g in list { all.entry(g.id.clone()).or_insert(g); }
        }
        // Xbox PC games through Xodus Nexus (Hearth OS ships it; elsewhere Settings › Games › Library adds it): Nexus
        // itself and the games you made shortcuts for in it.
        if self.store.source_on("xodus") {
            let list = crate::xodus::games();
            counts.insert("xodus".to_string(), list.len());
            for g in list { all.insert(g.id.clone(), g); }
        }
        // Battle.net, once it's set up (Library › Stores › Battle.net), and the Blizzard games it installed.
        if self.store.source_on("battlenet") {
            let list = crate::battlenet::games();
            counts.insert("battlenet".to_string(), list.len());
            for g in list { all.insert(g.id.clone(), g); }
        }
        // Beacon's games (BFME, Age of the Ring, LOTR: Conquest...), read again when Beacon's files changed.
        if self.store.source_on("beacon") {
            // Never waits for Beacon here (its program can take a while, or not answer): the last answer is used and a
            // new one is read in the background, applied by beacon_tick when it's in.
            if !self.beacon_loaded || crate::beacon::signature(&self.home) != self.beacon.sig { self.beacon_start(); }
            let list = crate::beacon::library_games(&self.beacon);
            counts.insert("beacon".to_string(), list.iter().filter(|g| g.installed).count());
            for g in list { all.entry(g.id.clone()).or_insert(g); }
        }
        let manual = self.manual_games();
        counts.insert("manual".to_string(), manual.len());
        for g in manual { all.entry(g.id.clone()).or_insert(g); }
        // Wabbajack modlists Hearth Mods installed (hosts/hearth-gaming wabbajack.rs): Mod Organizer 2 through Proton.
        for g in modlist_games() { all.entry(g.id.clone()).or_insert(g); }
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
        // Steam games on your account that aren't installed (steam_owned.rs): shown with an Install button.
        if self.store.source_on("steam") {
            if let (Some(steam_dir), Some(o)) = (steam::steam_path(&self.home), crate::steam_owned::load_current(&self.home)) {
                let (grid, _) = steam::user_data(&steam_dir);
                let cache = steam_dir.join("appcache/librarycache");
                let list = o["games"].as_array().cloned().unwrap_or_default();
                counts.insert("steamOwned".to_string(), list.len());
                for x in list {
                    let Some(app) = x["id"].as_str().filter(|a| !a.is_empty() && a.chars().all(|c| c.is_ascii_digit())) else { continue };
                    let id = format!("steam:{app}");
                    if all.contains_key(&id) { continue; }
                    all.insert(id.clone(), Game { id, title: x["title"].as_str().unwrap_or(app).to_string(), source: "steam", installed: false, state: "notInstalled",
                        store_minutes: x["minutes"].as_u64().unwrap_or(0), store_last_played: x["lastPlayed"].as_u64().unwrap_or(0),
                        art: steam::find_art(app, grid.as_deref(), &cache), launch_uri: Some(format!("steam://rungameid/{app}")),
                        install_uri: Some(format!("steam://install/{app}")), ..Default::default() });
                }
            }
        }
        // Cloud games you've played on Xbox: when you last played them.
        if stores::signed_in("xbox") {
            let played: HashMap<String, u64> = Owned::load(stores::xbox::CACHE).games.iter().map(|o| (norm(o["title"].as_str().unwrap_or("")), o["lastPlayed"].as_u64().unwrap_or(0))).collect();
            for g in all.values_mut() { if g.cloud { if let Some(t) = played.get(&norm(&g.title)) { g.store_last_played = *t; } } }
        }
        self.games = all;
        self.counts = counts;
        if let Some((_, dir)) = &share { self.share_mine(dir); }
        self.refresh_in_use();
        self.steam_stages(None);
        if !self.queue_restored { self.queue_restored = true; self.restore_queue(); }
        self.download_sig = self.signature();
    }

    // ------------------------------------------------------------------ multi-seat shared games (seat_share.rs)

    /// The other players' Epic and GOG installs, in this player's library. GOG games start from the other player's
    /// folder (DRM-free); an Epic install only shows for a player whose own Epic account has the game.
    fn add_shared(&self, all: &mut BTreeMap<String, Game>, cfg: &Value, dir: &std::path::Path) {
        let me = crate::seat_share::me();
        let epic_owned = Owned::load(epic::CACHE);
        let gog_owned = Owned::load(gog::CACHE);
        for s in crate::seat_share::shared_games(dir, &me, cfg, &crate::seat_share::uid_of) {
            if !self.store.source_on(&s.store) { continue; }
            if let Some(g) = all.get_mut(&s.id) {
                // Already in this player's library: their own copy stays theirs; the shared folder their Epic
                // registered earlier is still the other player's.
                if g.install_path.as_deref() == Some(s.path.as_path()) { g.shared_from = Some(s.from.clone()); g.can_uninstall = false; }
                continue;
            }
            let mut g = match s.store.as_str() {
                "gog" => {
                    let mut rec = serde_json::Map::new();
                    rec.insert(s.id.trim_start_matches("gog:").to_string(), json!(s.path.to_string_lossy()));
                    let Some(mut g) = gog::installed(&Value::Object(rec), &gog_owned).pop() else { continue };
                    if g.art.is_empty() { g.art = stores::art_of(&s.art); }
                    g
                }
                "epic" => {
                    if !stores::signed_in("epic") { continue; }
                    let Some(o) = epic_owned.find(s.id.trim_start_matches("epic:")) else { continue };
                    stores::game(s.id.clone(), &s.title, "epic", &s.path.to_string_lossy(), None, stores::art_of(&o["art"]), true)
                }
                _ => continue,
            };
            g.shared_from = Some(s.from);
            g.can_uninstall = false;
            all.insert(g.id.clone(), g);
        }
    }

    /// Offers this player's own Epic and GOG installs to the others (hearth-seats share-publish) when they changed,
    /// and adds the shared folder's Steam library to this player's Steam once Steam is closed (Steam rewrites its
    /// list of libraries when it closes, so it isn't changed under a running Steam).
    fn share_mine(&mut self, dir: &std::path::Path) {
        let mut mine: Vec<String> = self.games.values().filter(|g| matches!(g.source, "epic" | "gog") && g.installed && g.shared_from.is_none())
            .map(|g| format!("{}={}", g.id, g.install_path.as_ref().map(|p| p.to_string_lossy().to_string()).unwrap_or_default())).collect();
        mine.sort();
        let sig = mine.join("\n");
        if self.share_sig.as_deref() != Some(sig.as_str()) {
            self.share_sig = Some(sig);
            std::thread::spawn(|| match crate::multiplay::ask(crate::multiplay::SEATS, &["share-publish"]) {
                Ok(v) => crate::bridge::log(&format!("multi-seat: shared {} games with the other players{}", v["shared"].as_u64().unwrap_or(0),
                    v["not_shared"].as_array().filter(|a| !a.is_empty()).map(|a| format!(" (couldn't share {})", a.iter().filter_map(|x| x.as_str()).collect::<Vec<_>>().join(", "))).unwrap_or_default())),
                Err(e) => crate::bridge::log(&format!("multi-seat: couldn't share games: {e}")),
            });
        }
        if !self.steam_share_done && self.store.source_on("steam") && !steam::is_running() {
            self.steam_share_done = true;
            let (home, lib) = (self.home.clone(), dir.join("Steam"));
            std::thread::spawn(move || {
                let Some(s) = steam::steam_path(&home) else { return };
                if steam::libraries(&s).iter().any(|l| *l == lib) { return; }
                match steam::add_library(&home, &lib) {
                    Ok(_) => crate::bridge::log("multi-seat: the shared games folder is one of Steam's libraries now"),
                    Err(e) => crate::bridge::log(&format!("multi-seat: Steam's shared library: {e}")),
                }
            });
        }
    }

    /// What's running on the other seats.
    fn refresh_in_use(&mut self) {
        self.in_use_checked = now_ms();
        self.in_use = match crate::seat_share::active() {
            Some((cfg, _)) => crate::seat_share::in_use(std::path::Path::new(crate::seat_share::IN_USE), &crate::seat_share::me(), &cfg, &crate::seat_share::uid_of, &crate::procs::alive),
            None => HashMap::new(),
        };
    }

    // ------------------------------------------------------------------ installs (Epic, GOG) and downloads

    pub fn install_root(&self) -> PathBuf {
        // Multi-seat with shared games: new installs go in the shared folder, so every player can start them.
        self.store.data["installRoot"].as_str().filter(|s| !s.is_empty()).map(PathBuf::from)
            .or_else(|| crate::seat_share::active().map(|(_, d)| d))
            .unwrap_or_else(|| self.home.join("Games"))
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
                    let job = self.queue.lock().unwrap().get(id);
                    if let Some(j) = job { let mut j = j.lock().unwrap(); j.bytes_done = e["bytesDone"].as_u64(); j.bytes_total = e["bytesTotal"].as_u64(); j.progress = e["progress"].as_f64(); }
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
        if uri.starts_with("nimbus-") {
            // An expired sign-in: say so now (the page offers Sign in again) instead of failing in the queue.
            if stores::expiry::is_expired(g.source) { return Err(stores::expiry::message(g.source)); }
            return self.start_install(id, &g.title, &uri, false);
        }
        // Beacon's games are set up in Beacon (its folder, CD key, community files).
        if uri == "beacon:open" { return self.beacon_open(); }
        steam::spawn(&self.home, &uri)
    }

    /// The Steam library new Steam games go to (steam_ctl::choose_library).
    pub fn steam_library(&self) -> Option<PathBuf> {
        let steam = steam::steam_path(&self.home)?;
        let chosen = self.store.data["steamLibrary"].as_str().map(PathBuf::from);
        crate::steam_ctl::choose_library(&steam::libraries(&steam), chosen.as_deref(), &self.install_root())
    }

    /// Installs a Steam game without Steam's dialog (steam_ctl::install): {silent, waiting}.
    pub fn install_steam(&mut self, id: &str) -> Result<Value, String> {
        let g = self.games.get(id).cloned().ok_or("That game isn't in the library anymore.")?;
        if g.installed { return Err(format!("{} is already installed.", g.title)); }
        let app: u32 = id.strip_prefix("steam:").and_then(|a| a.parse().ok()).ok_or("That isn't a Steam game.")?;
        if !steam::client_installed(&self.home) { return Err("Steam isn't installed on this device.".into()); }
        let lib = self.steam_library().ok_or("Steam isn't set up yet. Open Steam once, then try again.")?;
        let r = crate::steam_ctl::install(&self.home, app, &g.title, &lib)?;
        if let Some(x) = self.games.get_mut(id) { x.state = "queued"; x.stage = Some("Waiting for Steam".into()); }
        Ok(r)
    }

    /// Steam downloads: what Steam is doing (its content log), and installs waiting for Steam to take them up.
    /// `log`: the content log already read (downloads() reads it for the speed too).
    fn steam_stages(&mut self, log: Option<&crate::steam_ctl::ContentLog>) {
        let read;
        let log = match log { Some(l) => l, None => { read = steam::steam_path(&self.home).map(|s| crate::steam_ctl::content_log(&s).0).unwrap_or_default(); &read } };
        let pending = crate::steam_ctl::pending_ids();
        for g in self.games.values_mut().filter(|g| g.source == "steam") {
            g.stage = None;
            let Some(app) = g.id.strip_prefix("steam:").and_then(|a| a.parse::<u32>().ok()) else { continue };
            if pending.contains(&app) && !g.installed { g.state = "queued"; g.stage = Some("Waiting for Steam".into()); continue; }
            if ["installed", "notInstalled"].contains(&g.state) { continue; }
            if let Some((stage, reason)) = log.apps.get(&app) {
                g.stage = match reason { Some(r) => Some(format!("Stopped: {r}")), None if !stage.is_empty() => Some(stage.clone()), None => None };
            }
        }
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
            let _ = self.store.update(|d| { crate::store::object_in(d, "gogInstalls").insert(gid, json!(dir)); });
        }
        self.save_queue();
        let title = self.games.get(id).map(|g| g.title.clone()).unwrap_or_else(|| id.to_string());
        if error.is_none() { self.record_installed(id); self.scan(); }
        // Failed because the store's sign-in expired: the page offers Sign in again with the error.
        let source = id.split_once(':').map(|x| x.0).unwrap_or("");
        let expired = error.filter(|e| *e != "cancelled").and(Some(source)).filter(|s| stores::expiry::is_expired(s));
        json!({"id": id, "title": title, "error": error, "expired": expired})
    }

    pub fn uninstall(&mut self, id: &str) -> Result<(), String> {
        let g = self.games.get(id).cloned().ok_or("That game isn't in the library anymore.")?;
        if self.running.as_ref().map(|r| r.id == id).unwrap_or(false) { return Err(format!("Exit {} first.", g.title)); }
        if let Some(who) = &g.shared_from { return Err(format!("This is {who}'s copy of {}. Only {who} can delete it.", g.title)); }
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
        let (clog, clog_fresh) = if self.store.source_on("steam") { steam::steam_path(&self.home).map(|s| crate::steam_ctl::content_log(&s)).unwrap_or_default() } else { Default::default() };
        self.steam_stages(Some(&clog));
        let sig = self.signature();
        let changed = sig != self.download_sig;
        self.download_sig = sig;
        let mut active: Vec<Value> = self.games.values().filter(|g| !["installed", "notInstalled", "cloud"].contains(&g.state)).map(|g| self.dto(g)).collect();
        // The queue's ids first, then the queue let go: dto() asks the queue again (holding it here waited forever).
        let queued = self.queue.lock().unwrap().ids();
        for id in queued {
            if let Some(g) = self.games.get(&id) { if !active.iter().any(|a| a["id"] == id.as_str()) { active.push(self.dto(g)); } }
        }
        let now = now_ms();
        let mut clog_used = false;
        for a in active.iter_mut() {
            if a["source"] != "steam" { continue; }
            if clog_used && a["state"] == "downloading" { a["bytesPerSec"] = json!(0); continue; }
            let (Some(id), Some(bytes)) = (a["id"].as_str().map(str::to_string), a["bytesDone"].as_u64()) else { continue };
            let prev = self.steam_bytes.get(&id).copied();
            let mut bps = 0;
            if let Some((t, b, _)) = prev { if now > t && bytes >= b { bps = ((bytes - b) as f64 / ((now - t) as f64 / 1000.0)).round() as u64; } }
            match prev {
                Some((t, b, old)) if bytes == b && now - t <= 20000 => bps = old,
                _ => { self.steam_bytes.insert(id.clone(), (now, bytes, bps)); }
            }
            // Steam writes its byte counts now and then; its log says the speed as it goes.
            if bps == 0 && clog_fresh && a["state"] == "downloading" { bps = clog.rate.unwrap_or(0); clog_used = true; }
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
            "installKind": match g.install_uri.as_deref() { None => None, Some(u) if u.starts_with("nimbus-") => Some("nimbus"), Some(u) if u.starts_with("steam:") => Some("steam"), Some(u) if u.starts_with("beacon:") => Some("beacon"), Some(_) => Some("store") },
            "system": g.system, "frozen": self.frozen.iter().any(|f| f.id == g.id), "stage": g.stage,
            "sharedFrom": g.shared_from, "borrowed": g.borrowed, "inUse": self.in_use.get(&g.id),
        });
        if let Some(b) = &g.beacon {
            out["beacon"] = json!({"family": b.family, "short": b.short, "ready": b.ready, "blocker": b.blocker(), "edition": b.edition, "viaSteam": b.via_steam, "custom": b.custom,
                "issues": b.issues.iter().map(|(l, m)| json!({"level": l, "message": m})).collect::<Vec<_>>(),
                "mod": b.selected_mod, "mods": b.mods.iter().map(|m| json!({"id": m.id, "name": m.name, "kind": m.kind})).collect::<Vec<_>>()});
        }
        // An install Hearth is running shows as a download.
        if let Some(Value::Object(j)) = self.queue.lock().unwrap().state(&g.id) { for (k, v) in j { out[k] = v; } }
        out
    }

    pub fn running_id(&self) -> Option<String> { self.running.as_ref().filter(|r| r.started_at > 0).map(|r| r.id.clone()) }
    pub fn running_since(&self) -> Option<u64> { self.running.as_ref().filter(|r| r.started_at > 0).map(|r| r.started_at) }

    /// Starts a game. {tracked} says whether Hearth can watch it (it then shows as running).
    pub fn launch(&mut self, id: &str) -> Result<Value, String> { self.launch_with(id, None) }

    /// Starts a game. `beacon_mod` (Beacon's games): Some(mod id) to play with that mod this time, Some("") without
    /// mods, None with the mod chosen in Beacon.
    pub fn launch_with(&mut self, id: &str, beacon_mod: Option<&str>) -> Result<Value, String> {
        let g = self.games.get(id).cloned().ok_or("That game isn't in the library anymore. Try rescanning.")?;
        // Put aside with Quick Resume: carry on where it was.
        if self.frozen.iter().any(|f| f.id == id) { return self.quick_resume(id).map(|_| json!({"tracked": true, "resumed": true})); }
        if self.running.as_ref().map(|r| r.id == id).unwrap_or(false) { return Ok(json!({"tracked": true, "resumed": true})); }
        if self.running.is_some() {
            if !self.quick_resume_on() { return Err("Another game is already running".into()); }
            self.freeze_running()?;
        }
        // Its number: in its processes' environment, on its windows, and what gamescope is asked to show.
        let app_id = self.app_id(id);
        if g.cloud {
            // The host opens the stream window (it gets this number); Hearth counts it as running from now.
            crate::bridge::log(&format!("launch: {id} as id {app_id} (Xbox Cloud Gaming's window)"));
            self.running = Some(Running { id: id.to_string(), track: Default::default(), launched_at: now_ms(), started_at: now_ms(), gone_since: 0, paused_at: 0, paused_ms: 0, closing: false, cloud: true, frozen_at: 0, app_id });
            return Ok(json!({"tracked": true, "stream": g.stream_url, "title": g.title}));
        }
        if !g.installed { return Err(format!("{} isn't installed yet.", g.title)); }
        // Multi-seat: a copy another player has right now waits for them (seat_share.rs).
        self.refresh_in_use();
        if let Some(who) = self.in_use.get(id) {
            if crate::seat_share::must_wait(g.source, g.shared_from.as_deref(), g.borrowed) {
                return Err(format!("{} is in use by {who}. {}", g.title, if g.source == "steam" { "Steam Family lets one player at a time play a lent game." } else { "One player at a time can play this copy." }));
            }
        }
        let track = match g.source {
            "steam" => {
                let uri = g.launch_uri.clone().unwrap_or_default();
                crate::steam_ctl::touch();
                // Started cold by the game's URL, Steam shows its update and main windows over everything.
                // Started hidden first (-silent), it only starts the game.
                if steam::is_running() {
                    steam::spawn(&self.home, &uri)?;
                } else {
                    if !steam::client_installed(&self.home) { return Err("Couldn't start Steam: it isn't installed.".into()); }
                    let home = self.home.clone();
                    std::thread::spawn(move || {
                        steam::ensure_running_silent(&home);
                        if let Err(e) = steam::spawn(&home, &uri) { crate::bridge::log(&format!("steam: {e}")); }
                    });
                }
                crate::procs::Track { steam_app_id: Some(id.trim_start_matches("steam:").to_string()), app_id: Some(app_id), ..Default::default() }
            }
            "manual" => {
                let exe = g.exe.clone().unwrap_or_default();
                let opts = crate::game_options::get(&self.store, id);
                let pid = crate::manual::launch(&exe, &g.args, &opts, app_id)?;
                // Its folder too (games that hand off to another program), unless that folder is too broad to mean anything.
                let dir = std::path::Path::new(&exe).parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
                let home = crate::paths::home().to_string_lossy().to_string();
                let broad = ["/", "/usr/bin", "/usr/local/bin", "/bin", "/opt", home.as_str()].contains(&dir.as_str()) || exe.ends_with(".desktop");
                crate::procs::Track { pid: Some(pid), dirs: if broad { vec![] } else { vec![dir] }, app_id: Some(app_id), ..Default::default() }
            }
            // Games a launcher starts (Battle.net in its Proton prefix, Xodus Nexus): Hearth runs the launcher's script or
            // shortcut with the game's number in its environment, and the game's own folder finds its processes, since
            // the launcher (not Hearth) starts the game itself.
            "battlenet" | "xodus" => {
                let exe = g.exe.clone().unwrap_or_default();
                let opts = crate::game_options::get(&self.store, id);
                let pid = crate::manual::launch(&exe, &g.args, &opts, app_id)?;
                let dirs = g.install_path.as_ref().filter(|p| p.starts_with(crate::battlenet::prefix())).map(|p| vec![p.to_string_lossy().to_string()]).unwrap_or_default();
                crate::procs::Track { pid: Some(pid), dirs, app_id: Some(app_id), ..Default::default() }
            }
            "emu" => {
                let pid = crate::emulators::launch(g.exe.as_deref().unwrap_or(""), g.system_key.unwrap_or(""), &self.home, app_id)?;
                crate::procs::Track { pid: Some(pid), app_id: Some(app_id), ..Default::default() }
            }
            "epic" | "gog" if g.can_uninstall || g.shared_from.is_some() => {
                // Installed by Hearth: started directly with Proton, so Hearth can watch it.
                if g.source == "epic" && g.shared_from.is_some() && !epic::installed(&Owned::default()).iter().any(|x| x.id == id) {
                    // Another player's Epic install: registered once with this player's own Epic sign-in, which then
                    // starts it with this player's copy of the game.
                    let dir = g.install_path.as_ref().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
                    epic::import_game(id.trim_start_matches("epic:"), &dir)?;
                }
                let opts = crate::game_options::get(&self.store, id);
                let pid = if g.source == "epic" { epic::launch(id.trim_start_matches("epic:"), &opts, app_id)? } else { gog::launch(&g, &opts, app_id)? };
                let dirs = g.install_path.as_ref().map(|p| vec![p.to_string_lossy().to_string()]).unwrap_or_default();
                crate::procs::Track { pid: Some(pid), dirs, app_id: Some(app_id), ..Default::default() }
            }
            "beacon" => {
                let b = g.beacon.clone().ok_or("Hearth doesn't know how to start this game.")?;
                if let Some(e) = b.blocker() { return Err(format!("{} can't start yet: {e}. Game setup (on its page) says how to fix it.", g.title)); }
                let cli = self.beacon.cli.clone().ok_or("Beacon isn't installed anymore.")?;
                let pid = crate::beacon::launch(&cli, &b, beacon_mod, app_id)?;
                // Steam copies Beacon starts through Steam (UaW, SWGB): Steam's reaper finds them.
                let steam = if b.via_steam { b.steam_appid.map(|n| n.to_string()) } else { None };
                let home = crate::paths::home().to_string_lossy().to_string();
                let dirs = b.path.clone().filter(|p| p != "/" && p.trim_end_matches('/') != home).map(|p| vec![p]).unwrap_or_default();
                crate::procs::Track { pid: Some(pid), dirs, steam_app_id: steam, app_id: Some(app_id) }
            }
            _ => return Err("Hearth doesn't know how to start this game.".into()),
        };
        crate::bridge::log(&format!("launch: {id} as id {app_id} ({}, {})", g.source, match (track.pid, &track.steam_app_id) {
            (Some(p), _) => format!("started process {p} with HEARTH_GAME_ID={app_id}"),
            (None, Some(s)) => format!("started by Steam, found by its reaper AppId={s}"),
            _ => "started".to_string(),
        }));
        self.running = Some(Running { id: id.to_string(), track, launched_at: now_ms(), started_at: 0, gone_since: 0, paused_at: 0, paused_ms: 0, closing: false, cloud: false, frozen_at: 0, app_id });
        // Multi-seat: the other players see "In use by …" (nothing on a PC without shared games).
        if let Err(e) = crate::seat_share::take(std::path::Path::new(crate::seat_share::IN_USE), id, &crate::seat_share::me(), std::process::id()) { crate::bridge::log(&format!("multi-seat: couldn't note {id} as in use: {e}")); }
        // Started without the store (its sign-in expired): the page says cloud saves and online play wait for Sign in again.
        let expired = matches!(g.source, "epic" | "gog") && stores::expiry::is_expired(g.source);
        Ok(serde_json::json!({"tracked": true, "expired": if expired { Some(g.source) } else { None }}))
    }

    /// The number for a library id (appid.rs): Steam games keep their Steam app id; everything else gets one of
    /// Hearth's, handed out once and kept in library.json "appIds".
    pub fn app_id(&mut self, id: &str) -> u32 {
        let known = self.store.data["appIds"].as_object().cloned().unwrap_or_default();
        let (n, new) = crate::appid::assign(&known, id);
        if new {
            let key = id.to_string();
            let _ = self.store.update(|d| {
                let m = d.entry("appIds").or_insert(json!({}));
                if !m.is_object() { *m = json!({}); }
                if let Some(o) = m.as_object_mut() { o.insert(key, json!(n)); }
            });
            crate::bridge::log(&format!("ids: {id} gets id {n}"));
        }
        n
    }
    /// The running game's number.
    pub fn running_app_id(&self) -> Option<u32> { self.running.as_ref().map(|r| r.app_id).filter(|n| *n != 0) }
    /// The numbers of the games put aside (their windows must never be shown for another game).
    pub fn frozen_ids(&self) -> Vec<u32> { self.frozen.iter().map(|f| f.app_id).filter(|n| *n != 0).collect() }
    /// Every game that's open, running or put aside: (number, library id, its processes), from one look at /proc.
    pub fn open_games(&self) -> Vec<(u32, String, Vec<u32>)> {
        let snap = crate::procs::snapshot(proc_root());
        let me = std::process::id();
        self.running.iter().chain(self.frozen.iter()).filter(|r| !r.cloud).map(|r| (r.app_id, r.id.clone(), crate::procs::game_processes(&r.track, &snap, me))).collect()
    }

    /// Processes of the games put aside (Quick Resume): their windows stay around, frozen on their last picture,
    /// and must never be the one put in front for another game.
    pub fn frozen_pids(&self) -> Vec<u32> {
        if self.frozen.is_empty() { return vec![]; }
        let snap = crate::procs::snapshot(proc_root());
        let me = std::process::id();
        let running = self.running.as_ref().map(|r| crate::procs::game_processes(&r.track, &snap, me)).unwrap_or_default();
        self.frozen.iter().flat_map(|f| crate::procs::game_processes(&f.track, &snap, me)).filter(|p| !running.contains(p)).collect()
    }
    pub fn game_pids(&self) -> Vec<u32> {
        let Some(r) = &self.running else { return vec![] };
        crate::procs::game_processes(&r.track, &crate::procs::snapshot(proc_root()), std::process::id())
    }

    /// Looks at what's running (every 2 seconds). Returns events for the UI: started, gap, handoff, exited, timeout.
    pub fn tick(&mut self) -> Vec<Value> {
        let before = self.frozen.len();
        let mut events = self.beacon_tick();
        // Steam: installs it should take up, and closing it when nothing needs it ("Start Steam only when needed").
        if self.store.source_on("steam") {
            let steam_game = self.running.iter().chain(self.frozen.iter()).any(|r| r.id.starts_with("steam:") || r.track.steam_app_id.is_some());
            events.extend(crate::steam_ctl::tick(&self.home, self.steam_on_demand(), steam_game));
        }
        events.extend(self.tick_running());
        if now_ms().saturating_sub(self.in_use_checked) > 10_000 { self.refresh_in_use(); }
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

    /// Beacon's files changed (a game set up, a mod added in Beacon's window): read its games again in the
    /// background, then use them on a later tick and tell the page.
    fn beacon_tick(&mut self) -> Vec<Value> {
        let next = self.beacon_next.lock().unwrap().take();
        if let Some(s) = next { self.beacon = s; self.beacon_loaded = true; self.apply_beacon(); return vec![json!({"type": "event", "name": "library.changed", "data": {}})]; }
        let now = now_ms();
        if !self.beacon_loaded || !self.store.source_on("beacon") || now.saturating_sub(self.beacon_checked) < 4000 { return vec![]; }
        self.beacon_checked = now;
        use std::sync::atomic::Ordering;
        if self.beacon_busy.load(Ordering::SeqCst) || crate::beacon::signature(&self.home) == self.beacon.sig { return vec![]; }
        self.beacon_start();
        vec![]
    }
    /// Reads Beacon's games on a thread of its own (once at a time); beacon_tick picks the answer up.
    fn beacon_start(&mut self) {
        use std::sync::atomic::Ordering;
        if self.beacon_busy.swap(true, Ordering::SeqCst) { return; }
        let (home, slot, busy) = (self.home.clone(), self.beacon_next.clone(), self.beacon_busy.clone());
        std::thread::spawn(move || {
            let s = crate::beacon::load(&home);
            *slot.lock().unwrap() = Some(s);
            busy.store(false, Ordering::SeqCst);
        });
    }
    /// Beacon's games in the library replaced by what Beacon says now.
    pub fn apply_beacon(&mut self) {
        self.games.retain(|_, g| g.source != "beacon");
        let list = crate::beacon::library_games(&self.beacon);
        self.counts.insert("beacon".to_string(), list.iter().filter(|g| g.installed).count());
        for g in list { self.games.entry(g.id.clone()).or_insert(g); }
    }
    /// Reads Beacon again now (after Hearth changed something in it).
    pub fn reload_beacon(&mut self) { self.beacon_start(); }
    /// Where a reading of Beacon made elsewhere goes (beacon::refresh, after Game setup changed something): used by
    /// the next beacon_tick, like the background reading.
    pub fn beacon_slot(&self) -> Arc<Mutex<Option<crate::beacon::Snapshot>>> { self.beacon_next.clone() }
    /// Opens Beacon's window (full screen in Game Mode).
    pub fn beacon_open(&self) -> Result<(), String> {
        let app = self.beacon.app.clone().ok_or("Beacon's window (beacon-app) isn't installed.")?;
        crate::beacon::open_app(&app, crate::signin_handoff::game_mode())
    }
    /// The mod a Beacon game starts with (Some(id), or None for no mod), saved in Beacon.
    pub fn beacon_select_mod(&mut self, id: &str, choice: Option<&str>) -> Result<Value, String> {
        let g = self.games.get(id).cloned().filter(|g| g.source == "beacon").ok_or("That game isn't in the library anymore.")?;
        let b = g.beacon.as_ref().ok_or("That game isn't Beacon's.")?;
        if let Some(m) = choice { if !b.mods.iter().any(|x| x.id == m) { return Err("That mod isn't in Beacon anymore.".into()); } }
        let cli = self.beacon.cli.clone().ok_or("Beacon isn't installed anymore.")?;
        crate::beacon::select_mod(&cli, &b.id, choice)?;
        // Shown at once; Beacon's own answer follows in the background.
        if let Some(bg) = self.beacon.games.iter_mut().find(|x| x.id == b.id) { bg.selected_mod = choice.map(str::to_string); }
        self.apply_beacon();
        self.reload_beacon();
        self.games.get(id).map(|g| self.dto(g)).ok_or("That game isn't in the library anymore.".into())
    }

    /// The game is over: play time saved (paused time doesn't count), and the UI told.
    pub fn exited(&mut self) -> Value {
        let r = self.running.take().unwrap();
        let dto = self.finish(r);
        json!({"type": "event", "name": "game.exited", "data": dto})
    }

    /// A game is over (or a Quick Resume game was closed): play time saved, Epic saves uploaded. Returns its dto.
    fn finish(&mut self, r: Running) -> Value {
        crate::seat_share::release(std::path::Path::new(crate::seat_share::IN_USE), &r.id, &crate::seat_share::me());
        // Saves follow you (Hearth Storage): this game's save goes to the server, if it's one Hearth Link follows.
        crate::link::game_closed(&r.id);
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
            let paused = r.paused_ms.saturating_add(if r.paused_at > 0 { now_ms().saturating_sub(r.paused_at) } else { 0 });
            let minutes = ((now_ms().saturating_sub(r.started_at).saturating_sub(paused)) as f64 / 60000.0).round() as u64;
            let id = r.id.clone();
            let _ = self.store.update(|d| {
                let s = crate::store::object_at(crate::store::object_in(d, "stats"), id, json!({"minutes": 0, "lastPlayed": 0}));
                s["minutes"] = json!(s["minutes"].as_u64().unwrap_or(0) + minutes);
                s["lastPlayed"] = json!(now_ms());
            });
        }
        self.games.get(&r.id).map(|g| self.dto(g)).unwrap_or(json!({"id": r.id}))
    }

    // ------------------------------------------------------------------ Quick Resume

    pub fn quick_resume_on(&self) -> bool { self.store.data["quickResume"] != false }
    /// Games Quick Resume would keep through turning off or restarting (when the PC can): the ones put aside and
    /// the one running (not a cloud game), with Quick Resume on.
    pub fn games_to_keep(&self) -> bool {
        self.quick_resume_on() && (!self.frozen.is_empty() || self.running.as_ref().is_some_and(|r| !r.cloud && r.started_at > 0))
    }

    /// The running game goes aside: frozen where it is, kept in memory.
    pub fn freeze_running(&mut self) -> Result<(), String> {
        let r = self.running.as_ref().ok_or("No game is running.")?;
        if r.cloud { return Err("Close Xbox Cloud Gaming first: cloud games can't be put aside.".into()); }
        if r.started_at == 0 { return Err(format!("{} is still starting. Wait a moment, then try again.", self.games.get(&r.id).map(|g| g.title.as_str()).unwrap_or("The game"))); }
        let (pids, how) = crate::procs::game_processes_how(&r.track, &crate::procs::snapshot(proc_root()), std::process::id());
        let stopped = if pids.is_empty() { 0 } else { crate::procs::signal(&pids, "STOP") };
        if stopped == 0 {
            crate::bridge::log(&format!("quick resume: couldn't put {} (id {}) aside: {} found by {how}, none paused", r.id, r.app_id, crate::procs::describe(&pids)));
            return Err("Couldn't put the game aside.".into());
        }
        let mut r = self.running.take().unwrap();
        let now = now_ms();
        if r.paused_at == 0 { r.paused_at = now; }
        r.frozen_at = now; r.gone_since = 0; r.closing = false;
        crate::bridge::log(&format!("quick resume: {} (id {}) put aside: froze {} found by {how} ({stopped} paused)", r.id, r.app_id, crate::procs::describe(&pids)));
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
        let snap = crate::procs::snapshot(proc_root());
        let me = std::process::id();
        let (mut pids, how) = crate::procs::game_processes_how(&r.track, &snap, me);
        // Never another game's: whatever the other games put aside (or running) own stays as it is.
        let others: std::collections::HashSet<u32> = self.frozen.iter().chain(self.running.iter()).flat_map(|o| crate::procs::game_processes(&o.track, &snap, me)).collect();
        let before = pids.len();
        pids.retain(|p| !others.contains(p));
        let thawed = crate::procs::signal(&pids, "CONT");
        let now = now_ms();
        if r.paused_at > 0 { r.paused_ms += now.saturating_sub(r.paused_at); r.paused_at = 0; }
        r.frozen_at = 0;
        crate::bridge::log(&format!("quick resume: back to {id} (id {}): thawed {} found by {how} ({thawed} resumed){}", r.app_id, crate::procs::describe(&pids),
            if before > pids.len() { format!(", {} left alone (other games')", before - pids.len()) } else { String::new() }));
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
        crate::bridge::log(&format!("quick resume: closing {} (id {}): {}", r.id, r.app_id, crate::procs::describe(&pids)));
        crate::procs::signal(&pids, "CONT");   // stopped processes only act on "please close" once they run again
        crate::procs::stop(&pids, std::time::Duration::from_secs(5));
        crate::bridge::log(&format!("quick resume: closed {}", r.id));
        self.finish(r)
    }

    /// Before Desktop Mode: the games Hearth stopped (put aside with Quick Resume, or paused) are ended (each process
    /// woken up first, then asked to close, killed after 5 s: procs::stop). A stopped game left behind froze the
    /// desktop session. Returns their titles (for the log and the page).
    pub fn end_stopped(&mut self) -> Vec<String> {
        let title = |me: &Self, id: &str| me.games.get(id).map(|g| g.title.clone()).unwrap_or_else(|| id.to_string());
        let mut ended = Vec::new();
        for r in std::mem::take(&mut self.frozen) {
            ended.push(title(self, &r.id));
            self.close_frozen_entry(r);
        }
        if let Some(id) = self.running.as_ref().filter(|r| r.paused_at > 0).map(|r| r.id.clone()) {
            ended.push(title(self, &id));
            self.resume();
            self.close_running();
        }
        self.save_quick();
        ended
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
    /// Saved by an older Hearth: no number yet (its processes don't carry one either, so the guesses find them).
    fn numbered(&mut self, mut r: Running) -> Running {
        if r.app_id == 0 { r.app_id = self.app_id(&r.id); }
        if r.track.app_id.is_none() && !r.cloud { r.track.app_id = Some(r.app_id); }
        r
    }
    /// After Hearth starts again: the games that are still there.
    pub fn restore_quick(&mut self) {
        let Some(v) = std::fs::read(quick_file()).ok().and_then(|b| serde_json::from_slice::<Value>(&b).ok()) else { return };
        if self.running.is_none() {
            if let Some(r) = from_rec(&v["running"]) {
                let r = self.numbered(r);
                if !pids_of(&r.track).is_empty() { crate::bridge::log(&format!("quick resume: {} (id {}) still running after a restart", r.id, r.app_id)); self.running = Some(r); }
            }
        }
        for f in v["frozen"].as_array().cloned().unwrap_or_default() {
            let Some(r) = from_rec(&f) else { continue };
            let r = self.numbered(r);
            if pids_of(&r.track).is_empty() || self.frozen.iter().any(|x| x.id == r.id) { continue; }
            crate::bridge::log(&format!("quick resume: {} (id {}) still put aside after a restart", r.id, r.app_id));
            self.frozen.push(r);
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
        match self.running.as_mut() { Some(r) => { r.closing = true; crate::bridge::log(&format!("close: {} (id {}): {}", r.id, r.app_id, crate::procs::describe(&pids))); } None => return true }
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
        // The clock set back while paused (time sync after waking) wrapped this to millions of years: the session's play
        // time was lost.
        r.paused_ms = r.paused_ms.saturating_add(now_ms().saturating_sub(r.paused_at));
        r.paused_at = 0;
        true
    }

    /// "Start Steam only when needed" (Settings › Library, on by default).
    pub fn steam_on_demand(&self) -> bool { self.store.data["steamOnDemand"] != false }

    pub fn get(&mut self, rescan: bool) -> Value {
        if rescan || self.games.is_empty() { self.scan(); }
        let mut games: Vec<Value> = self.games.values().map(|g| self.dto(g)).collect();
        games.sort_by_key(|g| g["title"].as_str().unwrap_or("").to_lowercase());
        let collections: Vec<Value> = self.store.data["collections"].as_array().map(|a| a.iter()
            .map(|c| json!({"id": c["id"], "name": c["name"], "count": c["games"].as_array().map(|g| g.len()).unwrap_or(0)})).collect()).unwrap_or_default();
        json!({ "games": games, "sources": self.store.data["sources"], "collections": collections, "counts": self.counts,
                "runningId": self.running_id(), "steamBackground": self.store.data["steamBackground"], "steamOnDemand": self.steam_on_demand(), "quickResume": self.quick_state() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn downloads_with_a_queued_store_game_answers() {
        let dir = tempfile::tempdir().unwrap();
        let mut lib = Library::new(Store::open(dir.path().join("l.json")), dir.path().to_path_buf());
        let g = stores::game("gog:1".into(), "A GOG game", "gog", "", None, vec![], false);
        lib.games.insert(g.id.clone(), g);
        lib.queue.lock().unwrap().add(stores::Job { game_id: "gog:1".into(), title: "A GOG game".into(), uri: "nimbus-gog:1".into(), state: "queued".into(), ..Default::default() }).unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || { let d = lib.downloads(); let _ = tx.send(d); });
        let d = rx.recv_timeout(std::time::Duration::from_secs(10)).expect("downloads() waited on itself");
        assert!(d.to_string().contains("gog:1"));
    }
    #[test]
    fn a_clock_set_back_while_paused_doesnt_lose_the_play_time() {
        let dir = tempfile::tempdir().unwrap();
        let mut lib = Library::new(Store::open(dir.path().join("l.json")), dir.path().to_path_buf());
        // Paused "an hour from now": the clock went back an hour since (a time sync after waking).
        lib.running = Some(Running { id: "manual:x".into(), track: Default::default(), launched_at: 1, started_at: 1, gone_since: 0, paused_at: now_ms() + 3_600_000,
                                     paused_ms: 5, closing: false, cloud: true, frozen_at: 0, app_id: 0 });
        assert!(lib.resume());
        assert_eq!(lib.running.as_ref().unwrap().paused_ms, 5);
    }
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
        let run = |id: &str, pid: u32| Running { id: id.into(), track: crate::procs::Track { pid: Some(pid), ..Default::default() }, launched_at: 1, started_at: now_ms(), gone_since: 0, paused_at: 0, paused_ms: 0, closing: false, cloud: false, frozen_at: 0, app_id: 0 };
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

        // By number: two games whose processes carry HEARTH_GAME_ID (one with children, like a launcher or Wine),
        // neither tracked by process id. Putting one aside and going back never touches the other's processes.
        let (ida, idb) = (lib.app_id("manual:launcher-game"), lib.app_id("emu:snes-game"));
        assert!(ida != idb && !crate::appid::is_steam(ida) && !crate::appid::is_steam(idb));
        assert_eq!(lib.app_id("manual:launcher-game"), ida);   // stable
        let mut ga = std::process::Command::new("sh").args(["-c", "sleep 60 & sleep 60 & wait"]).env(crate::appid::ENV, ida.to_string()).spawn().unwrap();
        let mut gb = std::process::Command::new("sleep").arg("60").env(crate::appid::ENV, idb.to_string()).spawn().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(300));
        let me = std::process::id();
        let by_id = |id: &str, n: u32| Running { id: id.into(), track: crate::procs::Track { app_id: Some(n), ..Default::default() }, launched_at: 1, started_at: now_ms(), gone_since: 0, paused_at: 0, paused_ms: 0, closing: false, cloud: false, frozen_at: 0, app_id: n };
        let snap = crate::procs::snapshot(std::path::Path::new("/proc"));
        let pa = crate::procs::game_processes(&crate::procs::Track { app_id: Some(ida), ..Default::default() }, &snap, me);
        let pb = crate::procs::game_processes(&crate::procs::Track { app_id: Some(idb), ..Default::default() }, &snap, me);
        assert_eq!(pa.len(), 3, "{pa:?}");
        assert!(pa.contains(&ga.id()));
        assert_eq!(pb, vec![gb.id()]);
        lib.running = Some(by_id("manual:launcher-game", ida));
        lib.freeze_running().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));
        for p in &pa { assert_eq!(state(*p), 'T'); }
        assert_ne!(state(gb.id()), 'T');
        lib.running = Some(by_id("emu:snes-game", idb));
        lib.quick_resume("manual:launcher-game").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));
        for p in &pa { assert_ne!(state(*p), 'T'); }
        assert_eq!(state(gb.id()), 'T');
        assert_eq!(lib.running_app_id(), Some(ida));
        assert_eq!(lib.frozen_ids(), vec![idb]);
        assert_eq!(lib.frozen_pids(), vec![gb.id()]);
        assert_eq!(lib.game_pids(), pa);
        crate::procs::signal(&pa, "KILL");
        crate::procs::signal(&[gb.id()], "KILL");
        let _ = ga.wait(); let _ = gb.wait();
        let _ = a.kill(); let _ = a.wait();
    }
}
