//! Hearth Storage's caches (docs/HEARTH-LINK.md, "Download once"): the server fetches things once and the other
//! devices get them from it at home-network speed. Both fall back to the normal way on their own, so a server that's
//! off, asleep or gone never stops a device from installing, updating or starting.
//!
//! Games (server: `cache`, off by default; devices: `cacheUse`, on):
//! - each device offers its installed games (`cache.offer`) when the server has no copy or an older build (Steam's
//!   buildid); the server pulls only what changed with xfer.rs into "Hearth Storage/Cache/<store>-<id>" (game files
//!   only: never a prefix or saves). Oldest-used copies make room when the drive gets full.
//! - a device gets a game (or a newer build of one it has) from the server instead of the store's download
//!   (`storage.cache.get`): the copy lands as the store's own game, Steam then fetches only what's left, if anything.
//!   Store accounts, sign-ins and DRM are never touched: it's the same files the store wrote.
//!
//! Hearth OS updates (server: `updates`, off by default; devices: `updatesUse`, on; `holdBack`, on):
//! - the server fetches each new image of the refs its devices run (hearth-update-mirror fetch, as this user) into
//!   "Hearth Storage/Updates" (content-addressed, every piece checked against its sha256).
//! - devices: bootc pulls through a mirror on 127.0.0.1:479 (a root socket-activated hearth-update-mirror serve),
//!   but only while hearth-update-mirror sync has written registries.conf.d/60-hearth-storage.conf: when
//!   `storage.updates.mirror` says `configured` (a server offers updates and this device uses it). A failed update
//!   with it in place removes it for 7 days. Tag manifests always come from the real registry; manifests by
//!   digest and layers come from the server through this service (`storage.updates.fetch`) when it has them,
//!   checked, else from the registry. No server: the mirror answers "unavailable" and bootc uses the registry.
//! - hold-back: the first device to ask for a new image goes first; the others wait (an ExecCondition on the
//!   background update, `hearth-update-mirror may-stage`) until a device reports that hearth-boot-guard marked a
//!   start of that image good (/var/lib/hearth/boot-good.json). Never longer than 3 days, server reachable or not;
//!   "Update now" is never held.
//!
//! Requests (paired devices, through storage_answer): cache.list, cache.offer {game}, updates.stat {digest},
//! updates.read {digest, offset, len} (chunk frames), updates.hold {ref, booted}, updates.report {ref, digest,
//! version, at}.

use crate::node::Node;
use crate::storage::key_ok;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

pub const MIRROR: &str = "/usr/libexec/nimbus/hearth-update-mirror";
const BOOT_GOOD: &str = "/var/lib/hearth/boot-good.json";
/// Hold-back never keeps a device from updating longer than this.
pub const HOLD_MAX_MS: u64 = 3 * 86_400_000;
/// The game cache leaves this much of the drive free.
const CACHE_RESERVE: u64 = 20 << 30;
const FETCH_EVERY_MS: u64 = 6 * 3_600_000;
const SEED_EVERY_MS: u64 = 30 * 60_000;
const STORES: [&str; 3] = ["steam", "epic", "gog"];

// ------------------------------------------------------------------ games

/// Steam's build of an installed game (appmanifest's buildid).
pub fn steam_build(acf: &str) -> Option<u64> {
    for line in acf.lines() {
        let parts: Vec<&str> = line.split('"').collect();
        if parts.len() >= 4 && parts[1].eq_ignore_ascii_case("buildid") { return parts[3].trim().parse().ok(); }
    }
    None
}

/// A game's build when its store says ("" when it doesn't: such a game is only cached once).
pub fn build_of(store: &str, meta: &Value) -> String {
    if store == "steam" { meta["acf"].as_str().and_then(steam_build).map(|b| b.to_string()).unwrap_or_default() } else { String::new() }
}

/// `mine` is a newer build than `theirs` (both known).
pub fn newer_build(mine: &str, theirs: &str) -> bool {
    matches!((mine.parse::<u64>(), theirs.parse::<u64>()), (Ok(a), Ok(b)) if a > b)
}

/// The cache's folder for a game: one copy per game, whichever device it came from.
pub fn cache_key(game_id: &str) -> String {
    let g: String = game_id.chars().map(|c| if c.is_ascii_alphanumeric() || "-_.".contains(c) { c } else { '-' }).take(120).collect();
    g.trim_start_matches('.').to_string()
}

/// A copy was used (for making room: the oldest-used go first). At most once an hour.
pub fn touch_used(dir: &Path, key: &str) {
    if !key_ok(key) || !dir.join(key).join("game.json").is_file() { return; }
    let f = dir.join(key).join("used");
    let old = std::fs::read_to_string(&f).ok().and_then(|s| s.trim().parse::<u64>().ok()).unwrap_or(0);
    let now = crate::now_ms();
    if now.saturating_sub(old) > 3_600_000 { let _ = std::fs::write(&f, now.to_string()); }
}

/// Every complete game in the cache.
pub fn cache_list(dir: &Path) -> Vec<Value> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let key = e.file_name().to_string_lossy().to_string();
        if !key_ok(&key) { continue; }
        let Some(v) = std::fs::read(e.path().join("game.json")).ok().and_then(|b| serde_json::from_slice::<Value>(&b).ok()) else { continue };
        let g = &v["game"];
        let used = std::fs::read_to_string(e.path().join("used")).ok().and_then(|s| s.trim().parse::<u64>().ok()).unwrap_or(0);
        out.push(json!({"id": format!("cache:{key}"), "key": key, "game": g["id"], "title": g["title"], "store": g["store"], "size": v["size"],
                        "files": v["files"], "build": build_of(g["store"].as_str().unwrap_or(""), &g["meta"]), "from": v["from"], "at": v["at"],
                        "used": used.max(v["at"].as_u64().unwrap_or(0))}));
    }
    out.sort_by(|a, b| a["title"].as_str().unwrap_or("").to_lowercase().cmp(&b["title"].as_str().unwrap_or("").to_lowercase()));
    out
}

/// Where a Steam game installed here lives (its library), for putting a newer build over it.
pub fn steam_library_of(install: &Path) -> Option<PathBuf> {
    let common = install.parent()?;
    let apps = common.parent()?;
    if common.file_name()? != "common" || apps.file_name()? != "steamapps" { return None; }
    apps.parent().map(Path::to_path_buf)
}

// ------------------------------------------------------------------ updates

pub fn digest_ok(d: &str) -> bool {
    d.len() == 71 && d.starts_with("sha256:") && d[7..].chars().all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
}

/// A piece of an image in the server's update cache.
pub fn blob_path(dir: &Path, digest: &str) -> Result<PathBuf, String> {
    if !digest_ok(digest) { return Err("Not a piece of an image".into()); }
    Ok(dir.join("blobs").join("sha256").join(&digest[7..]))
}

/// Hearth OS images only (the owner's on ghcr.io), by tag.
pub fn image_ref_ok(r: &str) -> bool {
    let Some(rest) = r.strip_prefix("ghcr.io/dragongamer9/") else { return false };
    let (name, tag) = rest.split_once(':').unwrap_or((rest, "latest"));
    !name.is_empty() && name.len() <= 64 && name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "-_.".contains(c))
        && !tag.is_empty() && tag.len() <= 128 && tag.chars().all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
}

/// Server: whether `peer` (running `booted`) should wait before staging the newest image of a ref (`e`, its entry
/// in updates.json). Returns (hold, why, `peer` becomes the device that goes first).
pub fn hold_decision(e: &Value, peer: &str, booted: &str, now: u64) -> (bool, &'static str, bool) {
    let latest = e["latest"].as_str().unwrap_or("");
    if latest.is_empty() { return (false, "unknown", false); }
    if booted == latest || (!booted.is_empty() && e["manifest"].as_str() == Some(booted)) { return (false, "current", false); }
    if e["good"].as_array().map(|g| !g.is_empty()).unwrap_or(false) { return (false, "verified", false); }
    if now.saturating_sub(e["seen"].as_u64().unwrap_or(0)) >= HOLD_MAX_MS { return (false, "waited", false); }
    match e["canary"].as_str().filter(|c| !c.is_empty()) {
        None => (false, "first", true),
        Some(c) if c == peer => (false, "first", false),
        Some(_) => (true, "waiting", false),
    }
}

/// Device: the answer when the server can't be asked: only a hold it already said, and never past 3 days.
pub fn hold_offline(last: &Value, now: u64) -> bool {
    last["hold"] == true && now.saturating_sub(last["since"].as_u64().unwrap_or(0)) < HOLD_MAX_MS
        && now.saturating_sub(last["at"].as_u64().unwrap_or(0)) < HOLD_MAX_MS
}

/// Device: the hold answer from the server, with how long this device has been held for that image (`last`: the
/// previous answer). Past 3 days it's let go whatever the server says.
pub fn hold_record(last: &Value, v: &Value, now: u64) -> Value {
    let same = last["hold"] == true && last["latest"] == v["latest"];
    let since = if same { last["since"].as_u64().unwrap_or(now) } else { now };
    let mut hold = v["hold"] == true;
    let mut reason = v["reason"].clone();
    if hold && now.saturating_sub(since) >= HOLD_MAX_MS { hold = false; reason = json!("waited"); }
    json!({"hold": hold, "reason": reason, "since": since, "until": since + HOLD_MAX_MS, "latest": v["latest"], "version": v["version"],
           "waitingFor": v["waitingFor"], "verifiedBy": v["verifiedBy"], "at": now})
}

/// The running Hearth OS image and when hearth-boot-guard last said a start of it was good (root writes it).
pub fn boot_good() -> Value {
    let p = std::env::var_os("HEARTH_BOOT_GOOD").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(BOOT_GOOD));
    std::fs::read(p).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null)
}

fn read_json(p: &Path) -> Value { std::fs::read(p).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null) }

impl Node {
    fn peer_name(&self, id: &str) -> String {
        if id.is_empty() { return String::new(); }
        if id == self.me { return self.name(); }
        crate::lock(&self.peers).get(id).map(|p| p.name.clone()).unwrap_or_else(|| "another device".into())
    }

    /// A paired Hearth Storage server that offers `what` ("cache" or "updates"); one that answered lately first.
    pub(crate) fn storage_server_for(&self, what: &str) -> Option<String> {
        let c = self.st();
        let live = crate::lock(&self.storage.servers).clone();
        let ps = crate::lock(&self.peers);
        let mut ids: Vec<String> = live.iter().filter(|(id, (_, v))| v["server"] == true && v[what] == true && v["usable"] == true && ps.is_paired(id))
            .map(|(id, _)| id.clone()).collect();
        if ids.is_empty() {
            ids = c.servers.iter().filter(|(id, v)| v[what] == true && ps.is_paired(id) && live.get(*id).map(|(_, l)| l["server"] == true).unwrap_or(true))
                .map(|(id, _)| id.clone()).collect();
        }
        ids.sort();
        ids.into_iter().next()
    }

    // ---------------------------------------------------------------- updates.json (server)

    fn upd_load(&self) -> Value {
        let mut v = read_json(&self.dir.join("updates.json"));
        if !v.is_object() { v = json!({}); }
        if !v["refs"].is_object() { v["refs"] = json!({}); }
        if !v["devices"].is_object() { v["devices"] = json!({}); }
        v
    }
    fn upd_save(&self, v: &Value) { let _ = crate::write_atomic(&self.dir.join("updates.json"), v.to_string().as_bytes()); }

    /// The hold answer for `peer` (this device too, when it's the server).
    fn server_hold(&self, peer: &str, r: &str, booted: &str) -> Value {
        let _g = crate::lock(&self.storage.upd);
        let mut st = self.upd_load();
        let now = crate::now_ms();
        let e = st["refs"][r].clone();
        let (hold, reason, first) = hold_decision(&e, peer, booted, now);
        if first {
            st["refs"][r]["canary"] = json!(peer);
            st["refs"][r]["canaryAt"] = json!(now);
            self.upd_save(&st);
            eprintln!("hearth-link: updates: {} goes first with {}", self.peer_name(peer), e["version"].as_str().unwrap_or("the new version"));
        }
        let canary = if first { peer.to_string() } else { e["canary"].as_str().unwrap_or("").to_string() };
        let verified = e["good"].as_array().and_then(|g| g.first()).map(|g| json!({"device": g["device"], "name": self.peer_name(g["device"].as_str().unwrap_or("")), "at": g["at"]}));
        json!({"hold": hold, "reason": reason, "latest": e["latest"], "version": e["version"], "seen": e["seen"],
               "waitingFor": if hold { json!(self.peer_name(&canary)) } else { Value::Null }, "verifiedBy": verified})
    }

    /// A device (or this one) says which image it runs and that a start of it was good.
    fn updates_record(&self, peer: &str, r: &str, digest: &str, version: &str, good_at: u64) {
        let _g = crate::lock(&self.storage.upd);
        let mut st = self.upd_load();
        let new_ref = !st["refs"].as_object().map(|m| m.contains_key(r)).unwrap_or(false);
        st["devices"][peer] = json!({"ref": r, "digest": digest, "version": version, "at": crate::now_ms(), "goodAt": good_at});
        let e = &mut st["refs"][r];
        if good_at > 0 && !digest.is_empty() && (e["latest"].as_str() == Some(digest) || e["manifest"].as_str() == Some(digest)) {
            if !e["good"].is_array() { e["good"] = json!([]); }
            let g = e["good"].as_array_mut().unwrap();
            if !g.iter().any(|x| x["device"] == peer) {
                g.push(json!({"device": peer, "at": good_at}));
                eprintln!("hearth-link: updates: {} started {} well: the other devices may update", self.peer_name(peer), if version.is_empty() { digest } else { version });
            }
        }
        if new_ref {
            // Only made by the line above when the server didn't know this ref yet: fetched soon.
            if let Some(m) = st["refs"].as_object_mut() { m.remove(r); }
            self.storage.fetch_at.store(0, Ordering::Relaxed);
        }
        self.upd_save(&st);
    }

    /// What hearth-update-mirror fetch said for a ref.
    fn updates_fetched(&self, r: &str, v: &Value) {
        let _g = crate::lock(&self.storage.upd);
        let mut st = self.upd_load();
        let now = crate::now_ms();
        if v["ok"] != true {
            if st["refs"].as_object().map(|m| m.contains_key(r)).unwrap_or(false) { st["refs"][r]["error"] = v["error"].clone(); }
            self.upd_save(&st);
            return;
        }
        let digest = v["digest"].as_str().unwrap_or("").to_string();
        let manifest = v["manifest"].as_str().unwrap_or("").to_string();
        if st["refs"][r]["latest"].as_str() != Some(digest.as_str()) {
            // A new image: devices already running it that said it started well count.
            let mut good = Vec::new();
            for (dev, d) in st["devices"].as_object().cloned().unwrap_or_default() {
                let dd = d["digest"].as_str().unwrap_or("");
                if d["ref"] == r && !dd.is_empty() && (dd == digest || dd == manifest) && d["goodAt"].as_u64().unwrap_or(0) > 0 { good.push(json!({"device": dev, "at": d["goodAt"]})); }
            }
            eprintln!("hearth-link: updates: new image for {r}: {}", v["version"].as_str().unwrap_or(&digest));
            st["refs"][r] = json!({"latest": digest, "manifest": manifest, "version": v["version"], "seen": now, "at": now, "bytes": v["bytes"],
                                   "canary": "", "canaryAt": 0, "good": good, "error": null});
        } else {
            let e = &mut st["refs"][r];
            e["at"] = json!(now);
            e["bytes"] = v["bytes"].clone();
            e["error"] = Value::Null;
        }
        self.upd_save(&st);
    }

    /// Server: fetch the newest image of every ref its devices (and itself) run, one after another, in the background.
    fn updates_fetch_all(self: &Arc<Self>) {
        if !Path::new(MIRROR).exists() || self.storage.fetching.swap(true, Ordering::SeqCst) { return; }
        self.storage.fetch_at.store(crate::now_ms(), Ordering::Relaxed);
        let mut refs: Vec<String> = { let _g = crate::lock(&self.storage.upd); self.upd_load()["devices"].as_object().cloned().unwrap_or_default()
            .values().filter_map(|d| d["ref"].as_str().map(str::to_string)).collect() };
        if let Some(r) = boot_good()["image"].as_str() { refs.push(r.to_string()); }
        refs.retain(|r| image_ref_ok(r));
        refs.sort();
        refs.dedup();
        refs.truncate(4);
        let dir = self.st().updates_dir(&self.home);
        let n = self.clone();
        std::thread::spawn(move || {
            for r in refs {
                let out = std::process::Command::new(MIRROR).args(["fetch", r.as_str()]).arg(&dir)
                    .stdin(std::process::Stdio::null()).stderr(std::process::Stdio::inherit()).output();
                let v = match out {
                    Ok(o) => String::from_utf8_lossy(&o.stdout).lines().last().and_then(|l| serde_json::from_str::<Value>(l).ok())
                        .unwrap_or(json!({"ok": false, "error": "hearth-update-mirror didn't answer"})),
                    Err(e) => json!({"ok": false, "error": e.to_string()}),
                };
                if v["ok"] != true { eprintln!("hearth-link: updates: fetching {r}: {}", v["error"].as_str().unwrap_or("failed")); }
                n.updates_fetched(&r, &v);
            }
            n.storage.fetching.store(false, Ordering::SeqCst);
        });
    }

    // ---------------------------------------------------------------- requests from paired devices

    pub async fn cache_answer(self: Arc<Self>, peer: &str, op: &str, req: &Value) -> Result<Value, String> {
        let home = self.home.clone();
        match op {
            "cache.list" => {
                let c = self.storage_allowed(peer)?;
                if !c.cache { return Err("The game cache is off on that device".into()); }
                let dir = c.cache_dir(&home);
                let (games, own) = tokio::task::spawn_blocking(move || {
                    let own: Vec<Value> = crate::games::list(&home).iter().filter(|g| STORES.contains(&g.store))
                        .map(|g| json!({"id": g.id, "title": g.title, "store": g.store, "size": g.size, "build": build_of(g.store, &g.meta)})).collect();
                    (cache_list(&dir), own)
                }).await.map_err(|e| e.to_string())?;
                Ok(json!({"games": games, "own": own}))
            }
            "cache.offer" => {
                let c = self.storage_allowed(peer)?;
                if !c.cache { return Err("The game cache is off on that device".into()); }
                let g = &req["game"];
                let gid = g["id"].as_str().filter(|i| !crate::storage::is_special(i) && i.contains(':')).ok_or("No game")?.to_string();
                let store = g["store"].as_str().filter(|s| STORES.contains(s)).ok_or("That store's games aren't kept")?.to_string();
                let build = g["build"].as_str().unwrap_or("").to_string();
                let key = cache_key(&gid);
                if !key_ok(&key) { return Err("No game".into()); }
                let dir = c.cache_dir(&home);
                let busy = crate::lock(&self.jobs).iter().any(|j| j.store == "cache" && !matches!(j.state().as_str(), "done" | "cancelled" | "error"));
                if busy { return Ok(json!({"accepted": false, "busy": true})); }
                let have = cache_list(&dir).into_iter().find(|e| e["key"] == key.as_str());
                if let Some(h) = &have { if !newer_build(&build, h["build"].as_str().unwrap_or("")) { return Ok(json!({"accepted": false, "have": true})); } }
                let size = g["size"].as_u64().unwrap_or(0);
                std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
                self.cache_make_room(&dir, &key, size)?;
                let place = dir.join(&key);
                // Being updated: not offered until the new copy is complete.
                let _ = std::fs::remove_file(place.join("game.json"));
                std::fs::create_dir_all(&place).map_err(|e| format!("{}: {e}", place.display()))?;
                crate::lock(&self.jobs).retain(|j| !(j.store == "cache" && j.state() == "error"));
                let job = self.start_job(peer, &json!({"id": gid, "title": g["title"], "store": "cache", "size": size}), place, false)?;
                eprintln!("hearth-link: storage: keeping {} ({store}{}) from {} for the other devices", job.title, if build.is_empty() { String::new() } else { format!(" build {build}") }, self.peer_name(peer));
                Ok(json!({"accepted": true, "key": key}))
            }
            "updates.stat" => {
                let c = self.storage_allowed(peer)?;
                if !c.updates { return Err("The update cache is off on that device".into()); }
                let p = blob_path(&c.updates_dir(&home), req["digest"].as_str().unwrap_or(""))?;
                let md = std::fs::metadata(&p).map_err(|_| "Not in the update cache".to_string())?;
                if !md.is_file() { return Err("Not in the update cache".into()); }
                Ok(json!({"size": md.len()}))
            }
            "updates.hold" => {
                let c = self.storage_allowed(peer)?;
                if !c.updates || !c.hold_back { return Ok(json!({"hold": false, "reason": "off"})); }
                let r = req["ref"].as_str().filter(|r| image_ref_ok(r)).ok_or("Not a Hearth OS image")?;
                Ok(self.server_hold(peer, r, req["booted"].as_str().unwrap_or("")))
            }
            "updates.report" => {
                let c = self.storage_allowed(peer)?;
                if !c.updates { return Ok(json!({"ok": false})); }
                let r = req["ref"].as_str().filter(|r| image_ref_ok(r)).ok_or("Not a Hearth OS image")?;
                let d = req["digest"].as_str().filter(|d| digest_ok(d)).unwrap_or("");
                let v: String = req["version"].as_str().unwrap_or("").chars().take(64).collect();
                self.updates_record(peer, r, d, &v, req["goodAt"].as_u64().unwrap_or(0));
                Ok(json!({"ok": true}))
            }
            _ => Err(format!("Unknown request: {op}")),
        }
    }

    /// Room for a game in the cache: the oldest-used copies go (never archives or backups, never one being copied).
    fn cache_make_room(&self, dir: &Path, key: &str, size: u64) -> Result<(), String> {
        let Some(mut free) = crate::free_space(dir) else { return Ok(()) };
        if size.saturating_add(CACHE_RESERVE) <= free { return Ok(()); }
        let mut list = cache_list(dir);
        list.sort_by_key(|g| g["used"].as_u64().unwrap_or(0));
        for g in list {
            if size.saturating_add(CACHE_RESERVE) <= free { break; }
            let k = g["key"].as_str().unwrap_or("");
            if k == key || !key_ok(k) { continue; }
            let p = dir.join(k);
            if crate::lock(&self.jobs).iter().any(|j| j.place == p && j.running.load(Ordering::SeqCst)) { continue; }
            if std::fs::remove_dir_all(&p).is_ok() { eprintln!("hearth-link: storage: made room in the game cache: removed {}", g["title"].as_str().unwrap_or(k)); }
            free = crate::free_space(dir).unwrap_or(free);
        }
        if size.saturating_add(CACHE_RESERVE) > free {
            return Err(format!("Not enough space on {} for the game cache: it needs {} and {} is free", self.name(), crate::human_bytes(size), crate::human_bytes(free)));
        }
        Ok(())
    }

    // ---------------------------------------------------------------- this device's side, every minute

    pub async fn cache_tick(self: &Arc<Self>) {
        let c = self.st();
        let now = crate::now_ms();
        if c.server && c.updates && now.saturating_sub(self.storage.fetch_at.load(Ordering::Relaxed)) > FETCH_EVERY_MS { self.updates_fetch_all(); }
        // The image this device runs, and that it started well: told to the server once per start.
        let bg = boot_good();
        let at = bg["at"].as_u64().unwrap_or(0);
        let image = bg["image"].as_str().unwrap_or("").to_string();
        if (c.updates || c.updates_use) && at != 0 && at != c.reported_good && image_ref_ok(&image) {
            let (digest, version) = (bg["digest"].as_str().unwrap_or("").to_string(), bg["version"].as_str().unwrap_or("").to_string());
            let done = if c.server && c.updates { self.updates_record(&self.me, &image, &digest, &version, at); true }
                else if let Some(s) = self.storage_server_for("updates") {
                    let req = json!({"op": "updates.report", "ref": image, "digest": digest, "version": version, "goodAt": at});
                    matches!(tokio::time::timeout(Duration::from_secs(20), self.request(&s, &req)).await, Ok(Ok(_)))
                } else { false };
            if done { let mut c2 = self.st(); c2.reported_good = at; let _ = self.st_save(c2); }
        }
        if c.cache_use && now.saturating_sub(self.storage.seed_at.load(Ordering::Relaxed)) > SEED_EVERY_MS {
            self.storage.seed_at.store(now, Ordering::Relaxed);
            self.cache_seed().await;
        }
    }

    /// Offers one of this device's games to the server's cache when the server has no copy or an older build.
    async fn cache_seed(self: &Arc<Self>) {
        if crate::game_running() { return; }
        let Some(server) = self.storage_server_for("cache") else { return };
        if server == self.me { return; }
        let Ok(Ok(list)) = tokio::time::timeout(Duration::from_secs(20), self.request(&server, &json!({"op": "cache.list"}))).await else { return };
        let home = self.home.clone();
        let mine = tokio::task::spawn_blocking(move || crate::games::list(&home)).await.unwrap_or_default();
        let theirs: Vec<Value> = list["games"].as_array().cloned().unwrap_or_default().into_iter().chain(list["own"].as_array().cloned().unwrap_or_default()).collect();
        let now = crate::now_ms();
        for g in mine.iter().filter(|g| STORES.contains(&g.store)) {
            let build = build_of(g.store, &g.meta);
            let best = theirs.iter().filter(|e| e["game"] == g.id.as_str() || e["id"] == g.id.as_str()).map(|e| e["build"].as_str().unwrap_or("").to_string())
                .fold(None::<String>, |a, b| match a { None => Some(b), Some(a) => Some(if newer_build(&b, &a) { b } else { a }) });
            let want = match &best { None => true, Some(b) => newer_build(&build, b) };
            if !want { continue; }
            if crate::lock(&self.storage.offered).get(&g.id).map(|(b, t)| *b == build && now.saturating_sub(*t) < 12 * 3_600_000).unwrap_or(false) { continue; }
            crate::lock(&self.storage.offered).insert(g.id.clone(), (build.clone(), now));
            let mut game = g.to_json();
            game["build"] = json!(build);
            match tokio::time::timeout(Duration::from_secs(30), self.request(&server, &json!({"op": "cache.offer", "game": game}))).await {
                Ok(Ok(v)) if v["accepted"] == true => { eprintln!("hearth-link: storage: {} goes to the game cache", g.title); return; }
                Ok(Ok(v)) if v["busy"] == true => { crate::lock(&self.storage.offered).remove(&g.id); return; }
                Ok(Ok(_)) => continue,
                Ok(Err(e)) => { eprintln!("hearth-link: storage: offering {} to the game cache: {e}", g.title); continue; }
                Err(_) => return,
            }
        }
    }

    // ---------------------------------------------------------------- the UI's and hearth-update-mirror's commands

    fn cache_local_status(&self) -> Value {
        let c = self.st();
        let using = self.storage_server_for("cache").filter(|s| *s != self.me).map(|s| json!({"id": s, "name": self.peer_name(&s)}));
        let jobs: Vec<Value> = crate::lock(&self.jobs).iter().filter(|j| j.store == "cache" || j.game_id.starts_with("cache:")).map(|j| j.to_json()).collect();
        json!({"enabled": if c.server { c.cache } else { c.cache_use }, "keep": c.cache, "use": c.cache_use, "server": c.server, "using": using, "jobs": jobs})
    }

    pub async fn cache_command(self: &Arc<Self>, cmd: &str, a: &Value) -> Result<Value, String> {
        let home = self.home.clone();
        match cmd {
            "storage.cache.status" => {
                let c = self.st();
                let mut out = self.cache_local_status();
                if c.server {
                    let dir = c.cache_dir(&home);
                    let mut list = tokio::task::spawn_blocking(move || cache_list(&dir)).await.map_err(|e| e.to_string())?;
                    for g in list.iter_mut() { let f = g["from"].as_str().unwrap_or("").to_string(); g["fromName"] = json!(self.peer_name(&f)); }
                    out["games"] = json!(list);
                }
                // What the server has that this device hasn't, or has an older build of (not waking it for this).
                if let Some(s) = self.storage_server_for("cache").filter(|s| *s != self.me) {
                    if let Ok(Ok(list)) = tokio::time::timeout(Duration::from_secs(10), self.request(&s, &json!({"op": "cache.list"}))).await {
                        let h = home.clone();
                        let mine: Vec<(String, String)> = tokio::task::spawn_blocking(move || crate::games::list(&h).iter().map(|g| (g.id.clone(), build_of(g.store, &g.meta))).collect::<Vec<(String, String)>>())
                            .await.unwrap_or_default();
                        let mut available = Vec::new();
                        let mut updates = Vec::new();
                        for e in list["games"].as_array().into_iter().flatten().chain(list["own"].as_array().into_iter().flatten()) {
                            let gid = e["game"].as_str().or(e["id"].as_str()).unwrap_or("").to_string();
                            let mut e = e.clone();
                            e["game"] = json!(gid);
                            match mine.iter().find(|m| m.0 == gid) {
                                None => if !available.iter().any(|x: &Value| x["game"] == gid.as_str()) { available.push(e) },
                                Some(m) if newer_build(e["build"].as_str().unwrap_or(""), &m.1) => { e["installedBuild"] = json!(m.1); updates.push(e) }
                                _ => {}
                            }
                        }
                        out["available"] = json!(available);
                        out["updates"] = json!(updates);
                    }
                }
                Ok(out)
            }
            "storage.cache.set" => {
                let mut c = self.st();
                if let Some(e) = a["enabled"].as_bool() { if c.server { c.cache = e; } else { c.cache_use = e; } }
                if let Some(e) = a["keep"].as_bool() { c.cache = e; }
                if let Some(e) = a["use"].as_bool() { c.cache_use = e; }
                self.st_save(c)?;
                Ok(self.cache_local_status())
            }
            "storage.cache.get" => {
                // A game (or a newer build of one installed here) from the server instead of the store's download.
                let gid = a["game"].as_str().filter(|g| !g.is_empty()).ok_or("Missing 'game'")?.to_string();
                let server = match a["server"].as_str().filter(|s| !s.is_empty()) { Some(s) => s.to_string(), None => self.storage_server_for("cache").ok_or("No Hearth Storage keeps games for this device")? };
                let list = self.storage_request(&server, &json!({"op": "cache.list"})).await?;
                let pick = list["games"].as_array().into_iter().flatten().filter(|e| e["game"] == gid.as_str())
                    .chain(list["own"].as_array().into_iter().flatten().filter(|e| e["id"] == gid.as_str()))
                    .fold(None::<Value>, |best, e| match best { Some(b) if !newer_build(e["build"].as_str().unwrap_or(""), b["build"].as_str().unwrap_or("")) => Some(b), _ => Some(e.clone()) })
                    .ok_or("That game isn't kept on Hearth Storage")?;
                let store = pick["store"].as_str().unwrap_or("").to_string();
                let (h, g2) = (home.clone(), gid.clone());
                let here = tokio::task::spawn_blocking(move || crate::games::find(&h, &g2)).await.map_err(|e| e.to_string())?;
                let place = match &here {
                    Some(g) => {
                        if crate::game_running() { return Err("Close the game first".into()); }
                        if g.store != "steam" { return Err("It's installed here already: its store updates it".into()); }
                        if !newer_build(pick["build"].as_str().unwrap_or(""), &build_of(g.store, &g.meta)) { return Err("This device has that version already".into()); }
                        g.roots.first().and_then(|r| steam_library_of(&r.path)).ok_or("Couldn't tell which Steam library it's in")?
                    }
                    None => crate::games::check_place(&home, &store, a["place"].as_str())?,
                };
                let size = pick["size"].as_u64().unwrap_or(0);
                if here.is_none() && size > 0 { if let Some(free) = crate::free_space(&place) { if size > free { return Err(format!("Not enough space there: it needs {} and {} is free", crate::human_bytes(size), crate::human_bytes(free))); } } }
                let job = self.start_job(&server, &json!({"id": pick["id"], "title": pick["title"], "store": store, "size": size}), place, false)?;
                Ok(job.to_json())
            }
            "storage.cache.remove" => {
                let c = self.st();
                let key = a["key"].as_str().unwrap_or("").trim_start_matches("cache:").to_string();
                if !key_ok(&key) { return Err("Not a kept game".into()); }
                let place = c.cache_dir(&home).join(&key);
                if crate::lock(&self.jobs).iter().any(|j| j.place == place && j.running.load(Ordering::SeqCst)) { return Err("It's being copied right now".into()); }
                if !place.exists() { return Err("That game isn't kept here".into()); }
                std::fs::remove_dir_all(&place).map_err(|e| format!("Couldn't remove it: {e}"))?;
                Ok(json!({"ok": true}))
            }
            "storage.updates.status" => {
                let c = self.st();
                let mut cached = Vec::new();
                if c.server {
                    let st = { let _g = crate::lock(&self.storage.upd); self.upd_load() };
                    for (r, e) in st["refs"].as_object().cloned().unwrap_or_default() {
                        let verified: Vec<Value> = e["good"].as_array().cloned().unwrap_or_default().iter()
                            .map(|g| json!({"device": g["device"], "name": self.peer_name(g["device"].as_str().unwrap_or("")), "at": g["at"]})).collect();
                        cached.push(json!({"ref": r, "digest": e["latest"], "manifest": e["manifest"], "version": e["version"], "seen": e["seen"], "at": e["at"],
                                           "bytes": e["bytes"], "error": e["error"], "firstDevice": self.peer_name(e["canary"].as_str().unwrap_or("")),
                                           "verifiedBy": verified, "holdUntil": e["seen"].as_u64().map(|s| s + HOLD_MAX_MS)}));
                    }
                }
                let using = if c.server && c.updates { Some(json!({"id": self.me, "name": self.name()})) }
                            else if !c.updates_use { None }
                            else { self.storage_server_for("updates").map(|s| json!({"id": s, "name": self.peer_name(&s)})) };
                Ok(json!({"enabled": if c.server { c.updates } else { c.updates_use }, "fetch": c.updates, "use": c.updates_use, "holdBack": c.hold_back, "server": c.server, "cached": cached, "fetching": self.storage.fetching.load(Ordering::Relaxed),
                          "using": using, "booted": boot_good(), "hold": c.update_hold, "mirror": Path::new(MIRROR).exists()}))
            }
            "storage.updates.set" => {
                let mut c = self.st();
                if let Some(e) = a["enabled"].as_bool() { if c.server { c.updates = e; } else { c.updates_use = e; } }
                if let Some(e) = a["fetch"].as_bool() { c.updates = e; }
                if let Some(e) = a["use"].as_bool() { c.updates_use = e; }
                if let Some(e) = a["holdBack"].as_bool() { c.hold_back = e; if !e { c.update_hold = Value::Null; } }
                let fetch_now = c.server && c.updates;
                self.st_save(c.clone())?;
                if fetch_now { self.storage.fetch_at.store(0, Ordering::Relaxed); }
                Ok(json!({"enabled": if c.server { c.updates } else { c.updates_use }, "fetch": c.updates, "use": c.updates_use, "holdBack": c.hold_back}))
            }
            // ---- hearth-update-mirror (root) on this device
            "storage.updates.mirror" => {
                // `configured`: hearth-update-mirror sync writes the registries.conf.d entry only then (a server offers
                // updates and this device uses it); `use`: the server answers right now.
                let c = self.st();
                if c.server && c.updates { return Ok(json!({"use": true, "configured": true, "server": self.me})); }
                if !c.updates_use { return Ok(json!({"use": false, "configured": false})); }
                let Some(s) = self.storage_server_for("updates") else { return Ok(json!({"use": false, "configured": false})) };
                // Not woken for this: a server that doesn't answer at once is skipped and bootc uses the registry.
                let ok = matches!(tokio::time::timeout(Duration::from_secs(5), self.connect(&s)).await, Ok(Ok(_)));
                Ok(json!({"use": ok, "configured": true, "server": s}))
            }
            "storage.updates.fetch" => {
                let c = self.st();
                let digest = a["digest"].as_str().unwrap_or("").to_string();
                if !digest_ok(&digest) { return Err("Not a piece of an image".into()); }
                if c.server && c.updates {
                    let p = blob_path(&c.updates_dir(&home), &digest)?;
                    let md = std::fs::metadata(&p).map_err(|_| "Not in the update cache".to_string())?;
                    return Ok(json!({"path": p, "size": md.len()}));
                }
                let server = self.storage_server_for("updates").ok_or("No Hearth Storage with updates")?;
                let st = tokio::time::timeout(Duration::from_secs(15), self.request(&server, &json!({"op": "updates.stat", "digest": digest}))).await
                    .map_err(|_| "Hearth Storage didn't answer".to_string())??;
                let size = st["size"].as_u64().ok_or("Not in the update cache")?;
                let dir = home.join(".cache/hearth-link/update-blobs");
                std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
                // Pieces left behind by a mirror that stopped half way.
                for e in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
                    let old = e.metadata().ok().and_then(|m| m.modified().ok()).and_then(|t| t.elapsed().ok()).map(|d| d > Duration::from_secs(7200)).unwrap_or(false);
                    if old { let _ = std::fs::remove_file(e.path()); }
                }
                let fin = dir.join(&digest[7..]);
                if std::fs::metadata(&fin).map(|m| m.len() == size).unwrap_or(false) { return Ok(json!({"path": fin, "size": size})); }
                let part = dir.join(format!("{}.part", &digest[7..]));
                let r = async {
                    use std::io::Write;
                    let mut f = std::fs::File::create(&part).map_err(|e| e.to_string())?;
                    let mut off = 0u64;
                    while off < size {
                        let len = crate::storage::READ_MAX.min(size - off);
                        let req = json!({"op": "updates.read", "digest": digest, "offset": off, "len": len});
                        let data = tokio::time::timeout(Duration::from_secs(120), self.read_remote_req(&server, &req, len)).await
                            .map_err(|_| "Hearth Storage stopped sending".to_string())??;
                        if data.is_empty() { return Err("The piece ended early".to_string()); }
                        f.write_all(&data).map_err(|e| e.to_string())?;
                        off += data.len() as u64;
                    }
                    f.flush().map_err(|e| e.to_string())?;
                    std::fs::rename(&part, &fin).map_err(|e| e.to_string())
                }.await;
                if let Err(e) = r { let _ = std::fs::remove_file(&part); return Err(e); }
                Ok(json!({"path": fin, "size": size}))
            }
            "storage.updates.drop" => {
                let digest = a["digest"].as_str().unwrap_or("");
                if digest_ok(digest) && !(self.st().server && self.st().updates) { let _ = std::fs::remove_file(home.join(".cache/hearth-link/update-blobs").join(&digest[7..])); }
                Ok(Value::Null)
            }
            "storage.updates.hold" => {
                let c = self.st();
                if !c.hold_back || !((c.server && c.updates) || c.updates_use) { return Ok(json!({"hold": false, "reason": "off"})); }
                let bg = boot_good();
                let image = bg["image"].as_str().unwrap_or("").to_string();
                let booted = bg["digest"].as_str().unwrap_or("").to_string();
                if !image_ref_ok(&image) { return Ok(json!({"hold": false, "reason": "unknown"})); }
                let now = crate::now_ms();
                let v = if c.server && c.updates { self.server_hold(&self.me, &image, &booted) } else {
                    let Some(s) = self.storage_server_for("updates") else { return Ok(json!({"hold": false, "reason": "no server"})) };
                    match tokio::time::timeout(Duration::from_secs(100), self.storage_request(&s, &json!({"op": "updates.hold", "ref": image, "booted": booted}))).await {
                        Ok(Ok(v)) => v,
                        _ => {
                            let hold = hold_offline(&c.update_hold, now);
                            return Ok(json!({"hold": hold, "reason": "unreachable", "since": c.update_hold["since"], "until": c.update_hold["until"]}));
                        }
                    }
                };
                let rec = hold_record(&c.update_hold, &v, now);
                let mut c2 = self.st();
                c2.update_hold = rec.clone();
                self.st_save(c2)?;
                Ok(rec)
            }
            _ => Err(format!("Unknown command: {cmd}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::stored_game;

    #[test]
    fn update_fetching_is_opt_in() {
        let c = crate::storage::StorageConfig::default();
        assert!(!c.updates && c.updates_use && c.hold_back && !c.cache && c.cache_use);
    }

    #[test]
    fn steam_builds_compare() {
        let acf = "\"AppState\"\n{\n\t\"appid\"\t\t\"620\"\n\t\"buildid\"\t\t\"1234567\"\n}\n";
        assert_eq!(steam_build(acf), Some(1234567));
        assert_eq!(build_of("steam", &json!({"acf": acf})), "1234567");
        assert_eq!(build_of("epic", &json!({"acf": acf})), "");
        assert!(newer_build("11", "9") && !newer_build("9", "11") && !newer_build("", "1") && !newer_build("5", "") && !newer_build("5", "5"));
        assert_eq!(cache_key("steam:620"), "steam-620");
        assert_eq!(cache_key(".x/../y"), "x-..-y");
    }

    #[test]
    fn steam_library_from_install() {
        assert_eq!(steam_library_of(Path::new("/home/a/.local/share/Steam/steamapps/common/Portal 2")), Some(PathBuf::from("/home/a/.local/share/Steam")));
        assert_eq!(steam_library_of(Path::new("/games/Portal 2")), None);
    }

    #[test]
    fn digests_and_refs() {
        let d = format!("sha256:{}", "a".repeat(64));
        assert!(digest_ok(&d));
        assert!(!digest_ok(&format!("sha256:{}", "A".repeat(64))) && !digest_ok("sha256:../../etc") && !digest_ok(&format!("sha512:{}", "a".repeat(64))));
        assert_eq!(blob_path(Path::new("/u"), &d).unwrap(), PathBuf::from(format!("/u/blobs/sha256/{}", "a".repeat(64))));
        assert!(blob_path(Path::new("/u"), "sha256:..").is_err());
        assert!(image_ref_ok("ghcr.io/dragongamer9/hearth-os:latest") && image_ref_ok("ghcr.io/dragongamer9/hearth-peppermint:testing") && image_ref_ok("ghcr.io/dragongamer9/hearth-os"));
        assert!(!image_ref_ok("ghcr.io/someone/hearth-os:latest") && !image_ref_ok("ghcr.io/dragongamer9/../x:1") && !image_ref_ok("ghcr.io/dragongamer9/hearth-os:a b"));
    }

    #[test]
    fn hold_back_waits_for_one_good_start() {
        let now = 10 * 86_400_000;
        let mut e = json!({"latest": "sha256:new", "manifest": "sha256:newm", "seen": now - 1000, "canary": "", "good": []});
        // The first device to ask goes first; the others wait for it.
        assert_eq!(hold_decision(&e, "A", "sha256:old", now), (false, "first", true));
        e["canary"] = json!("A");
        assert_eq!(hold_decision(&e, "A", "sha256:old", now), (false, "first", false));
        assert_eq!(hold_decision(&e, "B", "sha256:old", now), (true, "waiting", false));
        // Already on it (by the tag's digest or the manifest's).
        assert_eq!(hold_decision(&e, "B", "sha256:newm", now).0, false);
        // A start of it was good somewhere: everyone may go.
        e["good"] = json!([{"device": "A", "at": now}]);
        assert_eq!(hold_decision(&e, "B", "sha256:old", now), (false, "verified", false));
        // Never past 3 days, and nothing known: no hold.
        e["good"] = json!([]);
        assert_eq!(hold_decision(&e, "B", "sha256:old", now - 1000 + HOLD_MAX_MS), (false, "waited", false));
        assert_eq!(hold_decision(&json!(null), "B", "sha256:old", now).0, false);
    }

    #[test]
    fn device_hold_is_capped() {
        let t0 = 100 * 86_400_000u64;
        let v = json!({"hold": true, "reason": "waiting", "latest": "sha256:new"});
        let r1 = hold_record(&Value::Null, &v, t0);
        assert_eq!(r1["hold"], true);
        assert_eq!(r1["since"], t0);
        let r2 = hold_record(&r1, &v, t0 + 86_400_000);
        assert_eq!(r2["since"], t0);
        assert_eq!(hold_record(&r2, &v, t0 + HOLD_MAX_MS)["hold"], false);
        // A newer image starts its own wait.
        let v2 = json!({"hold": true, "latest": "sha256:newer"});
        assert_eq!(hold_record(&r2, &v2, t0 + 2 * 86_400_000)["since"], t0 + 2 * 86_400_000);
        // Server unreachable: only a hold it already said, only within 3 days.
        assert!(hold_offline(&r2, t0 + 2 * 86_400_000));
        assert!(!hold_offline(&r2, t0 + HOLD_MAX_MS));
        assert!(!hold_offline(&Value::Null, t0));
    }

    #[test]
    fn cache_lists_complete_copies() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("steam-620/root0")).unwrap();
        std::fs::create_dir_all(d.path().join("steam-400")).unwrap();
        let acf = "\"AppState\"\n{\n\t\"buildid\"\t\t\"77\"\n}\n";
        std::fs::write(d.path().join("steam-620/game.json"), json!({"game": {"id": "steam:620", "title": "Portal 2", "store": "steam", "meta": {"appid": "620", "installdir": "Portal 2", "acf": acf}},
            "roots": [{"kind": "install"}], "size": 5, "files": 1, "from": "dev", "at": 3}).to_string()).unwrap();
        let l = cache_list(d.path());
        assert_eq!(l.len(), 1);
        assert_eq!(l[0]["id"], "cache:steam-620");
        assert_eq!(l[0]["build"], "77");
        let g = stored_game(d.path(), "steam-620", "cache").unwrap();
        assert_eq!(g.id, "cache:steam-620");
        assert_eq!(g.meta["archivedId"], "steam:620");
        assert_eq!(g.roots[0].path, d.path().join("steam-620/root0"));
        touch_used(d.path(), "steam-620");
        assert!(cache_list(d.path())[0]["used"].as_u64().unwrap() > 3);
    }
}
