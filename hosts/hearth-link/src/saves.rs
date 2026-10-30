//! Saves follow you (Hearth Storage): before a game starts, the newest save for it comes from the Hearth Storage
//! server this device backs up to if it's newer than the one here; after the game, this device's save goes back.
//! So you can stop on the handheld and carry on at the desk.
//!
//! - Only games whose store doesn't do this already: Steam games without Steam Cloud, Epic games without Epic's cloud
//!   saves, GOG games without GOG's cloud saves (or with cloud saves turned off for the game in Hearth), Beacon's
//!   games. A game's save is its Windows user folder in its own Proton/Wine prefix (caches left out, like backups).
//! - Never silently overwritten: each device remembers which server version it last had ("base") and what its own
//!   save looked like then (a digest of names, sizes and times). Only the server changed: it's pulled. Only this
//!   device changed: it's pushed after the game. Both changed: both are kept and the person picks (a controller
//!   prompt before the game starts, or Settings › Hearth Storage › Backups & saves).
//! - What's replaced is kept: on the server the old version moves to "Saves/<game>/replaced/<when> <device>" (the
//!   last 5), on the device the save that's pulled over goes to Hearth Link's "save-backups/<game>/<when>" (the
//!   last 3) first.
//! - Never in the way of playing: the check before a game asks the server for at most CHECK (it isn't woken: a
//!   sleeping server would take a minute), then the game starts with the save that's here.
//!
//! On the server: "Hearth Storage/Saves/<game>/current" and save.json {version, at, from, fromName, digest, size,
//! files, title}. A push copies into "incoming" (seeded with current, so only what changed is sent) and only replaces
//! current when it's all there.

use crate::node::Node;
use crate::storage::PREFIX_SKIP;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// How long asking the server may take before a game: after that the game starts with the save that's here.
pub const CHECK: Duration = Duration::from_millis(2500);
const KEEP_REPLACED: usize = 5;
const KEEP_LOCAL: usize = 3;
/// A pull before a game: at most this long, then the game starts with the save that's here (it was backed up).
const PULL_MAX: Duration = Duration::from_secs(120);

static STATE_LOCK: Mutex<()> = Mutex::new(());

// ------------------------------------------------------------------ which games, and where their saves are

/// The folder name of a game's saves on the server (and in this device's save backups).
pub fn save_key(gid: &str) -> String {
    let k: String = gid.chars().map(|c| if c.is_ascii_alphanumeric() || "-_.".contains(c) { c } else { '-' }).take(140).collect();
    k.trim_start_matches('.').to_string()
}

/// A prefix's Windows user folder (Proton: pfx/drive_c, plain Wine: drive_c).
pub fn user_dir(prefix: &Path) -> Option<PathBuf> {
    [prefix.join("pfx/drive_c/users/steamuser"), prefix.join("drive_c/users/steamuser")].into_iter().find(|p| p.is_dir())
}

#[derive(Debug, Clone, PartialEq)]
pub enum Target {
    /// Followed: the save folder here.
    Follow(PathBuf),
    /// Not followed, and why (said in the UI and the log).
    Skip(String),
    /// A GOG game nobody has asked GOG about yet (whether it has cloud saves).
    AskGog { id: String, install: PathBuf },
}

/// The game's own option in Hearth (Library › game › Options › Cloud saves): off means its store doesn't sync it.
fn store_sync_off(lib: &Value, gid: &str) -> bool { lib["gameOptions"][gid]["cloudSaves"] == false }

/// Steam Cloud keeps this game's saves (Steam's own folder for them is there for some user).
pub fn steam_cloud(steam: &Path, appid: &str) -> bool {
    std::fs::read_dir(steam.join("userdata")).into_iter().flatten().flatten().any(|e| e.path().join(appid).join("remote").is_dir())
}

/// Epic's cloud saves keep this game's saves (legendary's metadata names a cloud save folder).
pub fn epic_cloud(legendary: &Path, app: &str) -> bool {
    std::fs::read(legendary.join("metadata").join(format!("{app}.json"))).ok().and_then(|b| serde_json::from_slice::<Value>(&b).ok())
        .map(|m| m["metadata"]["customAttributes"]["CloudSaveFolder"]["value"].is_string()).unwrap_or(false)
}

/// Where a game's saves are on this device, or why they aren't followed. `gog_cloud`: what GOG said for this game.
pub fn target(home: &Path, gid: &str, gog_cloud: Option<bool>) -> Target {
    let lib = std::fs::read(hearth_core::paths::data_dir().join("library.json")).ok().and_then(|b| serde_json::from_slice::<Value>(&b).ok()).unwrap_or(Value::Null);
    let (store, id) = gid.split_once(':').unwrap_or(("", gid));
    let first_start = || Target::Skip("Its first start here makes its save folder: its save comes over next time".into());
    match store {
        "steam" => {
            let Some((lib_dir, appid)) = crate::games::steam_installed(home).into_iter().find(|g| g.1 == id).map(|g| (g.0, g.1)) else { return Target::Skip("Not installed here".into()) };
            if let Some(steam) = hearth_core::steam::steam_path(home) { if steam_cloud(&steam, &appid) { return Target::Skip("Steam Cloud keeps its saves".into()); } }
            let prefix = lib_dir.join("steamapps/compatdata").join(&appid);
            match user_dir(&prefix) {
                Some(u) => Target::Follow(u),
                None if prefix.exists() => first_start(),
                None => Target::Skip("No Windows save folder for it here (a Linux game, or not started here yet)".into()),
            }
        }
        "epic" => {
            if epic_cloud(&hearth_core::stores::epic::config(), id) && !store_sync_off(&lib, gid) { return Target::Skip("Epic's cloud saves keep its saves".into()); }
            user_dir(&hearth_core::stores::prefix_of("epic", id)).map(Target::Follow).unwrap_or_else(first_start)
        }
        "gog" => {
            let Some(install) = lib["gogInstalls"][id].as_str().map(PathBuf::from) else { return Target::Skip("Not installed here".into()) };
            if !store_sync_off(&lib, gid) {
                match gog_cloud { Some(true) => return Target::Skip("GOG's cloud saves keep its saves".into()), None => return Target::AskGog { id: id.to_string(), install }, Some(false) => {} }
            }
            user_dir(&hearth_core::stores::prefix_of("gog", id)).map(Target::Follow).unwrap_or_else(first_start)
        }
        "beacon" => {
            // Beacon keeps a Windows prefix per game in its data folder (native or Flatpak).
            let Some(folder) = crate::games::safe_folder(id) else { return Target::Skip("Not a game Hearth can follow".into()) };
            let data = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|| home.join(".local/share"));
            let bases = [data.join("beacon"), home.join(".var/app").join(hearth_core::beacon::FLATPAK_ID).join("data/beacon")];
            bases.iter().flat_map(|b| [b.join("prefixes").join(&folder), b.join("prefix").join(&folder)]).find_map(|p| user_dir(&p))
                .map(Target::Follow).unwrap_or_else(|| Target::Skip("Beacon hasn't set this game up on this device yet".into()))
        }
        _ => Target::Skip("Hearth doesn't follow this game's saves".into()),
    }
}

// ------------------------------------------------------------------ a save's fingerprint

/// What a save folder looks like: (digest of every file's name, size and time; size; files; newest change in ms).
pub fn digest(root: &Path) -> Result<(String, u64, usize, u64), String> {
    let mut files = Vec::new();
    if root.exists() { crate::manifest::walk(root, 0, PREFIX_SKIP, &mut files).map_err(|e| format!("Reading the save: {e}"))?; }
    let mut f: Vec<_> = files.into_iter().filter(|e| e.k == "f").collect();
    f.sort_by(|a, b| a.p.cmp(&b.p));
    let mut h = blake3::Hasher::new();
    let (mut size, mut newest) = (0u64, 0u64);
    for e in &f {
        h.update(format!("{}\0{}\0{}\n", e.p, e.s, e.m).as_bytes());
        size += e.s;
        newest = newest.max(e.m.max(0) as u64 * 1000);
    }
    let d = if f.is_empty() { String::new() } else { h.finalize().to_hex()[..32].to_string() };
    Ok((d, size, f.len(), newest))
}

// ------------------------------------------------------------------ this device's memory of each game

/// saves.json: {games: {id: {base, digest, pushing: {version, digest}, pending, conflict, title, at}}, gogCloud: {id: bool}}.
pub fn load_state(dir: &Path) -> Value {
    let v: Value = std::fs::read(dir.join("saves.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null);
    if v.is_object() { v } else { json!({"games": {}, "gogCloud": {}}) }
}
fn save_state(dir: &Path, v: &Value) { let _ = crate::write_atomic(&dir.join("saves.json"), serde_json::to_string_pretty(v).unwrap().as_bytes()); }
fn with_state<T>(dir: &Path, f: impl FnOnce(&mut Value) -> T) -> T {
    let _g = crate::lock(&STATE_LOCK);
    let mut v = load_state(dir);
    if !v["games"].is_object() { v["games"] = json!({}); }
    let r = f(&mut v);
    save_state(dir, &v);
    r
}

/// What to do before a game, from this device's memory of it, the save here and the server's newest.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Plan { Nothing, Pull, Conflict, Adopt }

pub fn plan(st: &Value, local_digest: &str, server: &Value) -> Plan {
    let base = st["base"].as_str().unwrap_or("");
    let known = st["digest"].as_str().unwrap_or("");
    let local_changed = local_digest != known;
    let srv = server["version"].as_str().unwrap_or("");
    let server_changed = !srv.is_empty() && srv != base;
    match (local_changed, server_changed) {
        (_, false) => Plan::Nothing,
        (false, true) => Plan::Pull,
        // Both, but they're the same save (copied by hand, or a push whose answer was lost): nothing to choose.
        (true, true) if !local_digest.is_empty() && server["digest"].as_str() == Some(local_digest) => Plan::Adopt,
        (true, true) => Plan::Conflict,
    }
}

// ------------------------------------------------------------------ the server's side (blocking file work)

pub fn info(dir: &Path) -> Value {
    std::fs::read(dir.join("save.json")).ok().and_then(|b| serde_json::from_slice::<Value>(&b).ok()).filter(|v| v["version"].is_string() && dir.join("current").is_dir()).unwrap_or(Value::Null)
}

/// `cp -a` (keeps times, so a copy into it only sends what changed).
fn copy_tree(from: &Path, to: &Path) -> Result<(), String> {
    if let Some(p) = to.parent() { std::fs::create_dir_all(p).map_err(|e| e.to_string())?; }
    let st = std::process::Command::new("cp").arg("-a").arg("--reflink=auto").arg("--").arg(from).arg(to).status().map_err(|e| e.to_string())?;
    if st.success() { Ok(()) } else { Err(format!("Couldn't copy {}", from.display())) }
}

/// Keeps the newest `keep` folders in `dir` (their names start with the time).
fn prune(dir: &Path, keep: usize) {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
    v.sort();
    while v.len() > keep { let _ = std::fs::remove_dir_all(v.remove(0)); }
}

fn stamp() -> String {
    let s = crate::now_ms() / 1000;
    let (d, t) = (s / 86400, s % 86400);
    // Days since 1970 → y-m-d (civil from days, Howard Hinnant's algorithm).
    let z = d as i64 + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + if month <= 2 { 1 } else { 0 };
    format!("{year:04}-{month:02}-{day:02} {:02}.{:02}.{:02}", t / 3600, t % 3600 / 60, t % 60)
}

/// A push is coming: "incoming" seeded with current, and what it'll be called once it's there.
pub fn prepare_push(dir: &Path, pending: &Value) -> Result<PathBuf, String> {
    let inc = dir.join("incoming");
    let _ = std::fs::remove_dir_all(&inc);
    if dir.join("current").is_dir() { copy_tree(&dir.join("current"), &inc)?; } else { std::fs::create_dir_all(&inc).map_err(|e| e.to_string())?; }
    crate::write_atomic(&dir.join("pending.json"), pending.to_string().as_bytes())?;
    Ok(inc)
}

/// A push arrived complete (in `place`, the incoming folder): files the device doesn't have anymore removed, the old
/// version kept in replaced/, the new one made current.
pub fn finalize_push(place: &Path, manifest: &Value) -> Result<String, String> {
    let dir = place.parent().ok_or("No save folder")?;
    let pending: Value = std::fs::read(dir.join("pending.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).ok_or("The save's details are missing")?;
    let keep: std::collections::HashSet<String> = manifest["files"].as_array().into_iter().flatten().filter_map(|e| e["p"].as_str().map(str::to_string)).collect();
    let mut have = Vec::new();
    crate::manifest::walk(place, 0, &[], &mut have).map_err(|e| e.to_string())?;
    for e in have.iter().filter(|e| e.k != "d" && !keep.contains(&e.p)) { let _ = std::fs::remove_file(place.join(&e.p)); }
    let (files, size) = (keep.len(), manifest["size"].as_u64().unwrap_or(0));
    if dir.join("current").is_dir() {
        let from = info(dir)["fromName"].as_str().unwrap_or("").chars().filter(|c| c.is_ascii_alphanumeric() || *c == ' ' || *c == '-').take(30).collect::<String>();
        let rep = dir.join("replaced");
        std::fs::create_dir_all(&rep).map_err(|e| e.to_string())?;
        std::fs::rename(dir.join("current"), rep.join(format!("{} {}", stamp(), from).trim())).map_err(|e| e.to_string())?;
        prune(&rep, KEEP_REPLACED);
    }
    std::fs::rename(place, dir.join("current")).map_err(|e| e.to_string())?;
    let mut s = pending.clone();
    s["at"] = json!(crate::now_ms()); s["size"] = json!(size); s["files"] = json!(files);
    crate::write_atomic(&dir.join("save.json"), s.to_string().as_bytes())?;
    let _ = std::fs::remove_file(dir.join("pending.json"));
    Ok("Save kept".into())
}

/// The saves kept on this server: [{key, title, from, at, size, replaced}].
pub fn list(root: &Path) -> Vec<Value> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(root).into_iter().flatten().flatten() {
        let i = info(&e.path());
        if i.is_null() { continue; }
        let replaced = std::fs::read_dir(e.path().join("replaced")).map(|r| r.count()).unwrap_or(0);
        out.push(json!({"key": e.file_name().to_string_lossy(), "title": i["title"], "fromName": i["fromName"], "at": i["at"], "size": i["size"], "replaced": replaced}));
    }
    out
}

// ------------------------------------------------------------------ the service

impl Node {
    fn saves_root(&self, c: &crate::storage::StorageConfig) -> PathBuf { c.root(&self.home).join("Saves") }

    /// The server this device's saves follow (the one it backs up to), when on.
    fn save_server(&self) -> Option<String> {
        let c = self.st();
        (c.saves_follow && !c.backup_to.is_empty() && crate::lock(&self.peers).is_paired(&c.backup_to)).then_some(c.backup_to)
    }

    /// Requests from paired devices: saves.info, saves.offer (server), saves.done (the device that pushed).
    pub async fn saves_answer(self: Arc<Self>, peer: &str, op: &str, req: &Value) -> Result<Value, String> {
        match op {
            "saves.info" | "saves.offer" => {
                let c = self.storage_allowed(peer)?;
                if !c.backups { return Err("Backups and saves are off on that device".into()); }
                let key = save_key(req["game"].as_str().unwrap_or(""));
                if !crate::storage::key_ok(&key) || !req["game"].as_str().unwrap_or("").contains(':') { return Err("No game".into()); }
                let dir = self.saves_root(&c).join(&key);
                let cur = info(&dir);
                if op == "saves.info" { return Ok(json!({"save": cur})); }
                let base = req["base"].as_str().unwrap_or("");
                if !cur.is_null() && cur["version"].as_str() != Some(base) && req["force"] != true {
                    return Ok(json!({"conflict": true, "save": cur}));
                }
                let gid = format!("savedev:{}", req["game"].as_str().unwrap_or(""));
                if crate::lock(&self.jobs).iter().any(|j| j.peer == peer && j.game_id == gid && !matches!(j.state().as_str(), "done" | "cancelled" | "error")) {
                    let p: Value = std::fs::read(dir.join("pending.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null);
                    return Ok(json!({"accepted": true, "already": true, "version": p["version"]}));
                }
                crate::lock(&self.jobs).retain(|j| !(j.peer == peer && j.game_id == gid && j.state() == "error"));
                let version = crate::hex(&crate::random_bytes::<8>());
                let from_name = crate::lock(&self.peers).get(peer).map(|p| p.name.clone()).unwrap_or_default();
                let title: String = req["title"].as_str().unwrap_or("Game").chars().filter(|c| !c.is_control()).take(80).collect();
                let pending = json!({"version": version, "from": peer, "fromName": from_name, "digest": req["digest"], "title": title, "game": req["game"]});
                let d2 = dir.clone();
                let place = tokio::task::spawn_blocking(move || prepare_push(&d2, &pending)).await.map_err(|e| e.to_string())??;
                self.start_job(peer, &json!({"id": gid, "title": format!("{title} (save from {from_name})"), "store": "savedev"}), place, false)?;
                eprintln!("hearth-link: saves: taking {title}'s save from {from_name}");
                Ok(json!({"accepted": true, "version": version}))
            }
            "saves.done" => {
                // From the server: a push of ours is in.
                let gid = req["game"].as_str().unwrap_or("").to_string();
                let version = req["version"].as_str().unwrap_or("").to_string();
                if self.save_server().as_deref() != Some(peer) { return Err("This device doesn't keep its saves there".into()); }
                with_state(&self.dir, |s| {
                    let g = &mut s["games"][&gid];
                    if g["pushing"]["version"].as_str() == Some(version.as_str()) {
                        g["base"] = json!(version); g["digest"] = g["pushing"]["digest"].clone(); g["pushing"] = Value::Null; g["at"] = json!(crate::now_ms());
                    }
                });
                Ok(json!({"ok": true}))
            }
            _ => Err(format!("Unknown request: {op}")),
        }
    }

    /// For a copy the server asks for: this device's save of a game (savedev:<game>, only to its save server), or
    /// the server's kept save (savesrv:<key>).
    pub(crate) fn saves_find(&self, peer: &str, id: &str) -> Result<crate::games::LocalGame, String> {
        use crate::games::{LocalGame, Root};
        if let Some(gid) = id.strip_prefix("savedev:") {
            if self.save_server().as_deref() != Some(peer) { return Err("This device doesn't keep its saves there".into()); }
            let cloud = load_state(&self.dir)["gogCloud"][gid.trim_start_matches("gog:")].as_bool().or(Some(false));
            let Target::Follow(root) = target(&self.home, gid, cloud) else { return Err("That game's save isn't followed".into()) };
            return Ok(LocalGame { id: id.to_string(), title: "Save".into(), store: "savedev", size: 0, roots: vec![Root { kind: "install", path: root, skip: PREFIX_SKIP.to_vec() }], meta: json!({}) });
        }
        if let Some(key) = id.strip_prefix("savesrv:") {
            let c = self.storage_allowed(peer)?;
            if !c.backups || !crate::storage::key_ok(key) { return Err("Saves are off on that device".into()); }
            let dir = self.saves_root(&c).join(key);
            if info(&dir).is_null() { return Err("No save for that game there".into()); }
            return Ok(LocalGame { id: id.to_string(), title: "Save".into(), store: "savesrv", size: 0, roots: vec![Root { kind: "install", path: dir.join("current"), skip: vec![] }], meta: json!({}) });
        }
        Err("Not a save".into())
    }

    /// Where this game's save is here, asking GOG once (briefly) whether it has cloud saves.
    async fn saves_target(self: &Arc<Self>, gid: &str) -> Target {
        let gog = gid.strip_prefix("gog:").map(str::to_string);
        let cached = gog.as_ref().and_then(|g| load_state(&self.dir)["gogCloud"][g].as_bool());
        let (h, g2) = (self.home.clone(), gid.to_string());
        let t = tokio::task::spawn_blocking(move || target(&h, &g2, cached)).await.unwrap_or(Target::Skip("Couldn't look".into()));
        let Target::AskGog { id, install } = t else { return t };
        let (dir, id2) = (self.dir.clone(), id.clone());
        let ask = tokio::task::spawn_blocking(move || {
            let r = hearth_core::stores::gog::save_locations(&id2, &install).map(|l| !l.is_empty());
            if let Ok(b) = r { with_state(&dir, |s| { if !s["gogCloud"].is_object() { s["gogCloud"] = json!({}); } s["gogCloud"][&id2] = json!(b); }); }
            r
        });
        match tokio::time::timeout(CHECK, ask).await {
            Ok(Ok(Ok(_))) => {
                let (h, g2) = (self.home.clone(), gid.to_string());
                let cached = load_state(&self.dir)["gogCloud"][&id].as_bool();
                tokio::task::spawn_blocking(move || target(&h, &g2, cached)).await.unwrap_or(Target::Skip("Couldn't look".into()))
            }
            // GOG didn't answer in time: left alone this time (it may sync it itself); asked again next time.
            _ => Target::Skip("Couldn't ask GOG whether it keeps this game's saves".into()),
        }
    }

    /// The server's newest save of a game, asked quickly (never woken: the game shouldn't wait for that).
    async fn saves_server_info(self: &Arc<Self>, server: &str, gid: &str, wait: Duration) -> Result<Value, String> {
        match tokio::time::timeout(wait, self.request(server, &json!({"op": "saves.info", "game": gid}))).await {
            Ok(Ok(v)) => Ok(v["save"].clone()),
            Ok(Err(e)) => Err(e.trim_start_matches("connection: ").to_string()),
            Err(_) => Err("offline".into()),
        }
    }

    fn server_name(&self, id: &str) -> String { crate::lock(&self.peers).get(id).map(|p| p.name.clone()).unwrap_or_else(|| "Hearth Storage".into()) }

    /// Before a game: what to do with its save. Answers within about CHECK when the server isn't there.
    pub async fn saves_before(self: &Arc<Self>, gid: &str, title: &str) -> Result<Value, String> {
        let Some(server) = self.save_server() else { return Ok(json!({"action": "none", "why": "off"})) };
        let root = match self.saves_target(gid).await { Target::Follow(r) => r, Target::Skip(why) => return Ok(json!({"action": "skip", "why": why})), Target::AskGog { .. } => return Ok(json!({"action": "skip"})) };
        let r2 = root.clone();
        let (d, size, files, newest) = tokio::task::spawn_blocking(move || digest(&r2)).await.map_err(|e| e.to_string())??;
        let srv = match self.saves_server_info(&server, gid, CHECK).await {
            Ok(v) => v,
            Err(e) => { eprintln!("hearth-link: saves: {gid}: the server didn't answer in time ({}), starting with the save here", if e == "offline" { "offline" } else { "error" }); return Ok(json!({"action": "offline"})); }
        };
        let name = self.server_name(&server);
        let (gid_s, title_s) = (gid.to_string(), title.to_string());
        let p = with_state(&self.dir, |s| {
            let g = &mut s["games"][&gid_s];
            if !g.is_object() { *g = json!({}); }
            if !title_s.is_empty() { g["title"] = json!(title_s); }
            // A push of ours that the server has: it's the base now.
            if let Some(v) = g["pushing"]["version"].as_str().map(str::to_string) {
                if srv["version"].as_str() == Some(v.as_str()) { g["base"] = json!(v); g["digest"] = g["pushing"]["digest"].clone(); }
                g["pushing"] = Value::Null;
            }
            let p = plan(g, &d, &srv);
            match p {
                Plan::Adopt => { g["base"] = srv["version"].clone(); g["digest"] = json!(d); g["conflict"] = Value::Null; }
                Plan::Conflict => { g["conflict"] = json!({"server": srv, "local": {"at": newest, "size": size, "files": files}, "at": crate::now_ms()}); }
                Plan::Nothing if g["conflict"].is_object() && srv["version"] == g["base"] => { g["conflict"] = Value::Null; }
                _ => {}
            }
            p
        });
        Ok(match p {
            Plan::Nothing | Plan::Adopt => json!({"action": "none"}),
            Plan::Pull => json!({"action": "pull", "server": server, "serverName": name, "save": srv}),
            Plan::Conflict => {
                eprintln!("hearth-link: saves: {gid} changed here and on {name}: asking which to keep");
                json!({"action": "conflict", "server": server, "serverName": name, "save": srv, "local": {"at": newest, "size": size, "files": files}})
            }
        })
    }

    /// The server's save of a game copied over this device's (which is backed up first). Waits for it, up to PULL_MAX.
    pub async fn saves_pull(self: &Arc<Self>, gid: &str) -> Result<Value, String> {
        let server = self.save_server().ok_or("Saves don't follow you on this device: pick a Hearth Storage device to back up to first")?;
        let Target::Follow(root) = self.saves_target(gid).await else { return Err("This game's save isn't followed".into()) };
        let srv = self.saves_server_info(&server, gid, Duration::from_secs(8)).await.map_err(|e| if e == "offline" { format!("{} isn't answering", self.server_name(&server)) } else { e })?;
        let version = srv["version"].as_str().ok_or("There's no save for it there yet")?.to_string();
        let key = save_key(gid);
        // What's here now is kept first.
        let (r2, bk) = (root.clone(), self.dir.join("save-backups").join(&key));
        tokio::task::spawn_blocking(move || -> Result<(), String> {
            if digest(&r2)?.2 == 0 { return Ok(()); }
            copy_tree(&r2, &bk.join(stamp()))?;
            prune(&bk, KEEP_LOCAL);
            Ok(())
        }).await.map_err(|e| e.to_string())??;
        let title = load_state(&self.dir)["games"][gid]["title"].as_str().unwrap_or("Game").to_string();
        let gid_job = format!("savesrv:{key}");
        crate::lock(&self.jobs).retain(|j| !(j.game_id == gid_job && matches!(j.state().as_str(), "done" | "cancelled" | "error")));
        let job = self.start_job(&server, &json!({"id": gid_job, "title": format!("{title} (save)"), "store": "savesrv"}), root.clone(), false)?;
        let t = Instant::now();
        loop {
            tokio::time::sleep(Duration::from_millis(300)).await;
            match job.state().as_str() {
                "done" => break,
                "error" | "cancelled" => {
                    let e = crate::lock(&job.error).clone().unwrap_or_else(|| "It didn't arrive".into());
                    crate::lock(&self.jobs).retain(|j| j.id != job.id);
                    return Err(format!("The save didn't come over ({e}): the one here is used"));
                }
                _ if t.elapsed() > PULL_MAX => {
                    job.cancel.store(true, std::sync::atomic::Ordering::SeqCst); job.stop.store(true, std::sync::atomic::Ordering::SeqCst);
                    return Err("The save took too long to come over: the one here is used (the one that was here is backed up)".into());
                }
                _ => {}
            }
        }
        crate::lock(&self.jobs).retain(|j| j.id != job.id);
        let r2 = root.clone();
        let (d, ..) = tokio::task::spawn_blocking(move || digest(&r2)).await.map_err(|e| e.to_string())??;
        let gid_s = gid.to_string();
        with_state(&self.dir, |s| {
            let g = &mut s["games"][&gid_s];
            if !g.is_object() { *g = json!({}); }
            g["base"] = json!(version); g["digest"] = json!(d); g["conflict"] = Value::Null; g["pushing"] = Value::Null; g["at"] = json!(crate::now_ms());
        });
        eprintln!("hearth-link: saves: {gid}: took the newer save from {}", self.server_name(&server));
        Ok(json!({"pulled": true, "serverName": self.server_name(&server), "save": srv}))
    }

    /// After a game (and again later while the server didn't answer): this device's save goes to the server when it
    /// changed, unless the server's changed too (then it's kept here and the person picks).
    pub async fn saves_after(self: &Arc<Self>, gid: &str) -> Result<Value, String> {
        let Some(server) = self.save_server() else { return Ok(json!({"pushed": false})) };
        let Target::Follow(root) = self.saves_target(gid).await else { return Ok(json!({"pushed": false})) };
        let r2 = root.clone();
        let (d, ..) = tokio::task::spawn_blocking(move || digest(&r2)).await.map_err(|e| e.to_string())??;
        let st = load_state(&self.dir)["games"][gid].clone();
        if d.is_empty() || (st["digest"].as_str() == Some(d.as_str()) && st["pushing"].is_null()) {
            let g = gid.to_string();
            with_state(&self.dir, |s| { if s["games"][&g].is_object() { s["games"][&g]["pending"] = Value::Null; } });
            return Ok(json!({"pushed": false}));
        }
        let mark_pending = |on: bool| { let g = gid.to_string(); with_state(&self.dir, |s| { if !s["games"][&g].is_object() { s["games"][&g] = json!({}); } s["games"][&g]["pending"] = if on { json!(true) } else { Value::Null }; }); };
        let title = st["title"].as_str().unwrap_or("Game").to_string();
        let req = json!({"op": "saves.offer", "game": gid, "title": title, "base": st["base"].as_str().unwrap_or(""), "digest": d});
        let v = match tokio::time::timeout(Duration::from_secs(10), self.request(&server, &req)).await {
            Ok(Ok(v)) => v,
            Ok(Err(e)) => { mark_pending(true); return Err(e); }
            Err(_) => { mark_pending(true); return Ok(json!({"pushed": false, "later": true})); }
        };
        let gid_s = gid.to_string();
        if v["conflict"] == true {
            let (r3, srv) = (root.clone(), v["save"].clone());
            let (_, size, files, newest) = tokio::task::spawn_blocking(move || digest(&r3)).await.map_err(|e| e.to_string())??;
            with_state(&self.dir, |s| { let g = &mut s["games"][&gid_s]; g["pending"] = Value::Null; g["conflict"] = json!({"server": srv, "local": {"at": newest, "size": size, "files": files}, "at": crate::now_ms()}); });
            eprintln!("hearth-link: saves: {gid} changed here and on the server: kept here until you pick one");
            return Ok(json!({"pushed": false, "conflict": true}));
        }
        let version = v["version"].clone();
        with_state(&self.dir, |s| { let g = &mut s["games"][&gid_s]; g["pending"] = Value::Null; g["pushing"] = json!({"version": version, "digest": d}); });
        eprintln!("hearth-link: saves: {gid}: sending the save to {}", self.server_name(&server));
        Ok(json!({"pushed": true}))
    }

    /// Saves that didn't reach the server after their game (it was off): tried again now and then.
    pub async fn saves_retry(self: &Arc<Self>) {
        if self.save_server().is_none() || crate::game_running() { return; }
        let games: Vec<String> = load_state(&self.dir)["games"].as_object().map(|o| o.iter().filter(|(_, v)| v["pending"] == true).map(|(k, _)| k.clone()).collect()).unwrap_or_default();
        for g in games { if let Err(e) = self.saves_after(&g).await { eprintln!("hearth-link: saves: {g}: {e}"); } }
    }

    /// The server finished taking a device's save: that device is told (so it knows the server has it).
    pub async fn saves_job_done(self: &Arc<Self>, job: &crate::xfer::Job) {
        if job.store == "savedev" {
            let p: Value = std::fs::read(job.place.parent().map(|d| d.join("save.json")).unwrap_or_default()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null);
            let req = json!({"op": "saves.done", "game": p["game"], "version": p["version"]});
            if let Err(e) = tokio::time::timeout(Duration::from_secs(15), self.request(&job.peer, &req)).await.unwrap_or(Err("no answer".into())) {
                eprintln!("hearth-link: saves: telling the device its save is in: {e}");
            }
        }
        crate::lock(&self.jobs).retain(|j| j.id != job.id);
    }

    /// The UI's storage.saves.* commands.
    pub async fn saves_command(self: &Arc<Self>, cmd: &str, a: &Value) -> Result<Value, String> {
        let gid = || a["game"].as_str().filter(|g| g.contains(':') && g.len() < 200).map(str::to_string).ok_or("Missing 'game'".to_string());
        match cmd {
            "storage.saves.before" => self.saves_before(&gid()?, a["title"].as_str().unwrap_or("")).await,
            "storage.saves.pull" => self.saves_pull(&gid()?).await,
            "storage.saves.after" => self.saves_after(&gid()?).await,
            "storage.saves.resolve" => {
                let g = gid()?;
                match a["keep"].as_str().unwrap_or("") {
                    "server" => self.saves_pull(&g).await,
                    "local" => {
                        // This device's save wins: the server's version counts as seen, so it's sent (and the server's
                        // kept in replaced/) after the game, or now when no game is starting.
                        let st = load_state(&self.dir)["games"][&g].clone();
                        let base = st["conflict"]["server"]["version"].clone();
                        let g2 = g.clone();
                        with_state(&self.dir, |s| { let x = &mut s["games"][&g2]; x["base"] = base; x["conflict"] = Value::Null; });
                        if a["now"] == true { return self.saves_after(&g).await; }
                        Ok(json!({"kept": "local"}))
                    }
                    _ => Ok(json!({"kept": "both"})),
                }
            }
            "storage.saves.status" => {
                let c = self.st();
                let st = load_state(&self.dir);
                let games: Vec<Value> = st["games"].as_object().into_iter().flatten().map(|(k, v)| json!({"id": k, "title": v["title"], "at": v["at"], "conflict": v["conflict"], "pending": v["pending"] == true})).collect();
                let mut out = json!({"follow": c.saves_follow, "server": self.save_server(), "games": games});
                if c.server {
                    let root = self.saves_root(&c);
                    out["here"] = json!(tokio::task::spawn_blocking(move || list(&root)).await.unwrap_or_default());
                }
                Ok(out)
            }
            _ => Err(format!("Unknown command: {cmd}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_plain_folder_names() {
        assert_eq!(save_key("steam:620"), "steam-620");
        assert_eq!(save_key("beacon:custom:the witcher"), "beacon-custom-the-witcher");
        assert!(crate::storage::key_ok(&save_key("gog:1207658924")));
        assert!(!save_key("../x").starts_with('.'));
    }

    #[test]
    fn plans_never_overwrite_both() {
        let srv = json!({"version": "v2", "digest": "s"});
        // Nothing known yet, nothing here: the server's comes over.
        assert_eq!(plan(&json!({}), "", &srv), Plan::Pull);
        // Nothing known yet, something here that's different: asked.
        assert_eq!(plan(&json!({}), "l", &srv), Plan::Conflict);
        // The same save on both: nothing to ask.
        assert_eq!(plan(&json!({}), "s", &srv), Plan::Adopt);
        // Only the server changed.
        assert_eq!(plan(&json!({"base": "v1", "digest": "l"}), "l", &srv), Plan::Pull);
        // Only here changed: pushed after the game, nothing before.
        assert_eq!(plan(&json!({"base": "v2", "digest": "l"}), "l2", &srv), Plan::Nothing);
        // Both changed.
        assert_eq!(plan(&json!({"base": "v1", "digest": "l"}), "l2", &srv), Plan::Conflict);
        // Nothing on the server.
        assert_eq!(plan(&json!({}), "l", &json!(null)), Plan::Nothing);
    }

    #[test]
    fn digest_sees_changes_but_not_caches() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("AppData/Roaming/Game")).unwrap();
        std::fs::create_dir_all(d.path().join("Temp")).unwrap();
        std::fs::write(d.path().join("AppData/Roaming/Game/save1"), b"one").unwrap();
        let a = digest(d.path()).unwrap();
        assert_eq!(a.2, 1);
        std::fs::write(d.path().join("Temp/junk"), b"x").unwrap();
        assert_eq!(digest(d.path()).unwrap().0, a.0);
        std::fs::write(d.path().join("AppData/Roaming/Game/save1"), b"one more").unwrap();
        assert_ne!(digest(d.path()).unwrap().0, a.0);
        assert_eq!(digest(&d.path().join("nothing")).unwrap().0, "");
    }

    #[test]
    fn a_push_replaces_current_and_keeps_the_old_one() {
        let d = tempfile::tempdir().unwrap();
        let dir = d.path().join("steam-620");
        std::fs::create_dir_all(dir.join("current")).unwrap();
        std::fs::write(dir.join("current/old"), b"old").unwrap();
        std::fs::write(dir.join("current/kept"), b"k").unwrap();
        std::fs::write(dir.join("save.json"), json!({"version": "v1", "fromName": "Deck"}).to_string()).unwrap();
        let inc = prepare_push(&dir, &json!({"version": "v2", "fromName": "Desk", "game": "steam:620"})).unwrap();
        assert!(inc.join("old").exists());
        std::fs::write(inc.join("new"), b"new").unwrap();
        let m = json!({"files": [{"p": "kept", "k": "f"}, {"p": "new", "k": "f"}], "size": 4});
        finalize_push(&inc, &m).unwrap();
        assert!(dir.join("current/new").exists() && dir.join("current/kept").exists() && !dir.join("current/old").exists());
        assert_eq!(info(&dir)["version"], "v2");
        let rep: Vec<_> = std::fs::read_dir(dir.join("replaced")).unwrap().flatten().collect();
        assert_eq!(rep.len(), 1);
        assert!(rep[0].path().join("old").exists());
        assert!(rep[0].file_name().to_string_lossy().ends_with("Deck"));
        assert_eq!(list(d.path()).len(), 1);
    }

    #[test]
    fn stores_that_sync_are_left_alone() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("userdata/123/620/remote")).unwrap();
        assert!(steam_cloud(d.path(), "620"));
        assert!(!steam_cloud(d.path(), "400"));
        std::fs::create_dir_all(d.path().join("metadata")).unwrap();
        std::fs::write(d.path().join("metadata/A.json"), r#"{"metadata":{"customAttributes":{"CloudSaveFolder":{"value":"{AppData}/A"}}}}"#).unwrap();
        std::fs::write(d.path().join("metadata/B.json"), r#"{"metadata":{"customAttributes":{}}}"#).unwrap();
        assert!(epic_cloud(d.path(), "A"));
        assert!(!epic_cloud(d.path(), "B"));
        assert_eq!(target(d.path(), "xodus:Halo", None), Target::Skip("Hearth doesn't follow this game's saves".into()));
    }

    #[test]
    fn stamps_sort_by_time() {
        let s = stamp();
        assert_eq!(s.len(), 19);
        assert!(s.starts_with("20"));
    }
}
