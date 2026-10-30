//! Hearth Storage: a spare PC running Hearth as a small home server for your other Hearth devices. It's Hearth Link
//! (same pairing, same encrypted QUIC connections, same copy engine), so there is no second system to trust:
//! only paired devices can ask it anything, and it never opens anything to the internet on its own.
//!
//! On the server (Settings › Hearth Storage › Use this device as Hearth Storage, off by default):
//! - shared folders: paired devices list them and copy files from them (Hearth Files, Game Mode). Over Hearth Link
//!   they're read-only for now; hidden files (a dot at the start) are never shared, so ~/.ssh or Hearth Link's own
//!   key can't leak even when Home is shared.
//! - game archive: a device moves a game here to free space (the server pulls it with xfer.rs, like any copy,
//!   into "Hearth Storage/Archive/<key>"), and copies it back the same way later (the archive is offered as a game
//!   of its original store, so the device puts it back where it belongs and tells the store).
//! - backups: every few hours each device that picked this server has its game saves and screenshots pulled here
//!   ("Hearth Storage/Backups/<device>"). Only files that changed are sent (xfer.rs skips files with the same size
//!   and time); files deleted on the device stay in the backup.
//! - media: a file in a shared folder is played on another device through a small web server on that device that
//!   only listens on 127.0.0.1 and only knows random links made for one file (stream_http below).
//! - away from home: off by default; then it only answers devices on the local network. On, it answers over Hearth
//!   Link's own connection (direct or through the relay, end-to-end encrypted): no port is opened.
//! - Windows sharing (SMB): off by default; hearth-storage-helper (root, through pkexec and a polkit rule) writes
//!   Samba's config for the shared folders, and the firewall opens 445 to the local network only
//!   (hearth-link-firewall, like Hearth Link's own ports).
//! - sleep: optional; after N idle minutes (no copies, no game, the session idle) it suspends. Devices wake it with
//!   a Wake-on-LAN packet when they need it (they remember its network cards' addresses).
//!
//! Requests from a paired device (wire.rs, answered in node.rs):
//!   storage.info                   what this device offers (or {server: false})
//!   storage.ls {share, path}       a folder of a shared folder
//!   storage.stat {share, path}     one file's size
//!   storage.read {share, path, offset, len}   a file's bytes, as chunk frames (wire.rs), then done
//!   archive.offer {game}           "keep this game for me": the server pulls it
//!   archive.list / archive.remove {game}
//!   archive.done {game, size, files}   (server → device) the game is safely in the archive
//!   backup.offer {what}            "back up my saves | shots now": the server pulls them
//! Copies use the usual manifest/hashes/chunks requests with these ids: archive:<key> (from the server),
//! share:<share>:<path> (from the server), backup:saves | backup:shots (from the device, only to its backup server).

use crate::games::{LocalGame, Root};
use crate::node::Node;
use crate::wire;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const HELPER: &str = "/usr/libexec/nimbus/hearth-storage-helper";
const FIREWALL: &str = "/usr/libexec/nimbus/hearth-link-firewall";
/// At most this much of a file per storage.read (the player asks again for the rest).
pub const READ_MAX: u64 = 8 << 20;
/// A media link works this long.
const STREAM_LIFETIME_MS: u64 = 12 * 3600 * 1000;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Share {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    /// Windows sharing (SMB) may change files in it. Over Hearth Link shared folders are read-only for now.
    pub write: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct StorageConfig {
    // ---- this device as a server
    pub server: bool,
    /// Where archived games and backups go; "Hearth Storage" in Home when empty.
    pub folder: String,
    pub shares: Vec<Share>,
    pub archive: bool,
    pub backups: bool,
    pub media: bool,
    /// Answer paired devices that aren't on the local network (over Hearth Link's encrypted connection).
    pub away: bool,
    pub smb: bool,
    /// Suspend after this many idle minutes (0: never).
    pub sleep_minutes: u64,
    // ---- this device using a server
    /// The server this device backs up to ("": no backups).
    pub backup_to: String,
    pub backup_saves: bool,
    pub backup_shots: bool,
    pub backup_hours: u64,
    pub last_backup: u64,
    /// What each server said about itself last (name, network cards for waking it): kept, because a sleeping
    /// server can't say it again.
    pub servers: HashMap<String, Value>,
    /// Games this device moved to a server: id → {server, title, size, at, state: archiving|archived|changed}.
    pub archived: HashMap<String, Value>,
}

impl Default for StorageConfig {
    fn default() -> Self {
        StorageConfig {
            server: false, folder: String::new(), shares: Vec::new(), archive: true, backups: true, media: true, away: false, smb: false,
            sleep_minutes: 0, backup_to: String::new(), backup_saves: true, backup_shots: true, backup_hours: 6, last_backup: 0,
            servers: HashMap::new(), archived: HashMap::new(),
        }
    }
}

impl StorageConfig {
    pub fn load(dir: &Path) -> StorageConfig {
        std::fs::read(dir.join("storage.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
    }
    pub fn save(&self, dir: &Path) -> Result<(), String> {
        crate::write_atomic(&dir.join("storage.json"), serde_json::to_string_pretty(self).unwrap().as_bytes())
    }
    pub fn root(&self, home: &Path) -> PathBuf {
        if self.folder.trim().is_empty() { home.join("Hearth Storage") } else { PathBuf::from(self.folder.trim()) }
    }
    pub fn archive_dir(&self, home: &Path) -> PathBuf { self.root(home).join("Archive") }
    pub fn backups_dir(&self, home: &Path) -> PathBuf { self.root(home).join("Backups") }

    /// Changes from the UI, checked. (The backup server is checked against the paired devices by the caller.)
    pub fn apply(&mut self, a: &Value, home: &Path) -> Result<(), String> {
        let b = |k: &str| a.get(k).and_then(Value::as_bool);
        if let Some(v) = b("server") { self.server = v; }
        if let Some(v) = b("archive") { self.archive = v; }
        if let Some(v) = b("backups") { self.backups = v; }
        if let Some(v) = b("media") { self.media = v; }
        if let Some(v) = b("away") { self.away = v; }
        if let Some(v) = b("smb") { self.smb = v; }
        if let Some(v) = b("backupSaves") { self.backup_saves = v; }
        if let Some(v) = b("backupShots") { self.backup_shots = v; }
        if let Some(m) = a.get("sleepMinutes").and_then(Value::as_u64) { self.sleep_minutes = m.min(720); }
        if let Some(h) = a.get("backupHours").and_then(Value::as_u64) { self.backup_hours = h.clamp(1, 168); }
        if let Some(t) = a.get("backupTo").and_then(Value::as_str) {
            if !t.is_empty() && !t.chars().all(|c| c.is_ascii_hexdigit()) { return Err("That isn't a paired device".into()); }
            if t != self.backup_to { self.last_backup = 0; }
            self.backup_to = t.to_string();
        }
        if let Some(f) = a.get("folder").and_then(Value::as_str) {
            let f = f.trim();
            if f.is_empty() { self.folder = String::new(); }
            else {
                let p = PathBuf::from(f);
                if !p.is_absolute() || !place_ok(&p, home) { return Err("Pick a folder in Home or on a drive".into()); }
                std::fs::create_dir_all(&p).map_err(|e| format!("{}: {e}", p.display()))?;
                self.folder = p.to_string_lossy().to_string();
            }
        }
        Ok(())
    }

    pub fn add_share(&mut self, path: &str, name: &str, write: bool, home: &Path) -> Result<Share, String> {
        if self.shares.len() >= 32 { return Err("That's a lot of shared folders already: remove one first".into()); }
        let p = std::fs::canonicalize(path.trim()).map_err(|_| "That folder isn't there".to_string())?;
        if !p.is_dir() { return Err("Pick a folder".into()); }
        let rel = in_base(&p, home).ok_or("Only folders in Home or on a drive can be shared")?;
        if rel.components().any(|c| c.as_os_str().to_string_lossy().starts_with('.')) { return Err("Hidden folders can't be shared".into()); }
        if self.shares.iter().any(|s| s.path == p) { return Err("That folder is shared already".into()); }
        let mut n: String = name.trim().chars().filter(|c| !c.is_control() && *c != '/').take(40).collect();
        if n.is_empty() { n = p.file_name().map(|x| x.to_string_lossy().to_string()).unwrap_or_else(|| "Shared".into()); }
        let s = Share { id: crate::hex(&crate::random_bytes::<6>()), name: n, path: p, write };
        self.shares.push(s.clone());
        Ok(s)
    }

    /// A path in a shared folder, checked: plain parts only, nothing hidden, and (links followed) still inside it.
    pub fn resolve(&self, share: &str, rel: &str) -> Result<(PathBuf, Share), String> {
        let sh = self.shares.iter().find(|s| s.id == share).cloned().ok_or("That folder isn't shared anymore")?;
        let root = std::fs::canonicalize(&sh.path).map_err(|_| format!("{} isn't there right now (a drive that's unplugged?)", sh.name))?;
        let rel = rel.trim_matches('/');
        if rel.is_empty() { return Ok((root, sh)); }
        let r = crate::manifest::safe_rel(rel).ok_or("That isn't a path in the shared folder")?;
        if rel.split('/').any(|p| p.starts_with('.')) { return Err("Hidden files aren't shared".into()); }
        let full = std::fs::canonicalize(root.join(r)).map_err(|_| "That isn't there anymore".to_string())?;
        if !full.starts_with(&root) { return Err("That's outside the shared folder".into()); }
        Ok((full, sh))
    }

    pub fn to_json(&self, home: &Path) -> Value {
        let root = self.root(home);
        json!({"server": self.server, "folder": root, "customFolder": self.folder, "free": crate::free_space(&root).unwrap_or(0),
               "shares": self.shares.iter().map(|s| json!({"id": s.id, "name": s.name, "path": s.path, "write": s.write, "there": s.path.is_dir()})).collect::<Vec<_>>(),
               "archive": self.archive, "backups": self.backups, "media": self.media, "away": self.away, "smb": self.smb, "sleepMinutes": self.sleep_minutes,
               "backupTo": self.backup_to, "backupSaves": self.backup_saves, "backupShots": self.backup_shots, "backupHours": self.backup_hours,
               "lastBackup": self.last_backup, "archived": self.archived})
    }
}

/// A folder this device may use for Hearth Storage: in Home or on a drive (never system folders).
pub fn place_ok(p: &Path, home: &Path) -> bool { in_base(p, home).is_some() }

/// Where `p` is inside Home or a drive's folder (relative to it), or None when it's elsewhere.
fn in_base(p: &Path, home: &Path) -> Option<PathBuf> {
    let user = std::env::var("USER").unwrap_or_default();
    let mut bases = vec![home.to_path_buf(), PathBuf::from("/mnt")];
    if !user.is_empty() { bases.push(PathBuf::from("/run/media").join(&user)); bases.push(PathBuf::from("/media").join(&user)); }
    let p = std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    bases.iter().find_map(|b| {
        let b = std::fs::canonicalize(b).unwrap_or_else(|_| b.clone());
        p.strip_prefix(&b).ok().map(Path::to_path_buf)
    })
}

/// Folders worth sharing, for the UI's list: the usual ones in Home and each drive.
pub fn candidates(home: &Path) -> Vec<Value> {
    let mut out = Vec::new();
    for n in ["Videos", "Music", "Pictures", "Documents", "Downloads", "Games"] {
        let p = home.join(n);
        if p.is_dir() { out.push(json!({"label": n, "path": p})); }
    }
    let user = std::env::var("USER").unwrap_or_default();
    for base in [PathBuf::from("/run/media").join(&user), PathBuf::from("/media").join(&user)] {
        for d in std::fs::read_dir(&base).into_iter().flatten().flatten() {
            if d.path().is_dir() { out.push(json!({"label": format!("Drive {}", d.file_name().to_string_lossy()), "path": d.path()})); }
        }
    }
    out
}

/// A folder of a shared folder: [{name, dir, size, mtime}], folders first, nothing hidden.
pub fn list_dir(dir: &Path) -> Result<Vec<Value>, String> {
    let rd = std::fs::read_dir(dir).map_err(|_| "That folder can't be opened".to_string())?;
    let mut out: Vec<(bool, String, Value)> = Vec::new();
    for e in rd.flatten() {
        let Some(name) = e.file_name().to_str().map(str::to_string) else { continue };
        if name.starts_with('.') { continue; }
        let Ok(md) = std::fs::metadata(e.path()) else { continue };
        let mtime = md.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0);
        let dir = md.is_dir();
        out.push((dir, name.to_lowercase(), json!({"name": name, "dir": dir, "size": if dir { 0 } else { md.len() }, "mtime": mtime})));
        if out.len() >= 5000 { break; }
    }
    out.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    Ok(out.into_iter().map(|x| x.2).collect())
}

// ------------------------------------------------------------------ the games and folders copied with xfer.rs

/// Ids that aren't a store's game but something of Hearth Storage.
pub fn is_special(id: &str) -> bool { id.starts_with("archive:") || id.starts_with("share:") || id.starts_with("backup:") }

fn static_store(s: &str) -> Option<&'static str> {
    match s { "steam" => Some("steam"), "epic" => Some("epic"), "gog" => Some("gog"), "beacon" => Some("beacon"), "xodus" => Some("xodus"), _ => None }
}

pub fn key_ok(k: &str) -> bool {
    !k.is_empty() && k.len() <= 160 && !k.starts_with('.') && k.chars().all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
}

/// The archive's folder name for a game from a device: the device's id (start of it) and the game's id.
pub fn archive_key(peer: &str, game_id: &str) -> String {
    let dev: String = peer.chars().filter(|c| c.is_ascii_alphanumeric()).take(8).collect();
    let g: String = game_id.chars().map(|c| if c.is_ascii_alphanumeric() || "-_.".contains(c) { c } else { '-' }).take(120).collect();
    format!("{dev}-{}", g.trim_start_matches('.'))
}

/// A device's folder in Backups: its name and the start of its id (two devices can have the same name).
pub fn device_folder(peer: &str, name: &str) -> String {
    let n: String = name.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ' ' { c } else { '-' }).take(40).collect();
    let n = n.trim().trim_start_matches('.').to_string();
    let dev: String = peer.chars().filter(|c| c.is_ascii_alphanumeric()).take(8).collect();
    if n.is_empty() { dev } else { format!("{n} ({dev})") }
}

/// An archived game, offered for copying back: the original store's game, its files in root0, root1...
pub fn archive_game(dir: &Path, key: &str) -> Result<LocalGame, String> {
    if !key_ok(key) { return Err("Not an archived game".into()); }
    let base = dir.join(key);
    let v: Value = std::fs::read(base.join("game.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).ok_or("That game isn't in the archive (anymore)")?;
    let game = &v["game"];
    let store = static_store(game["store"].as_str().unwrap_or("")).ok_or("The archived game's store isn't known here")?;
    let roots: Vec<Root> = v["roots"].as_array().cloned().unwrap_or_default().iter().enumerate().map(|(i, r)| Root {
        kind: if r["kind"] == "prefix" { "prefix" } else { "install" }, path: base.join(format!("root{i}")), skip: vec![],
    }).collect();
    let mut meta = game["meta"].clone();
    if !meta.is_object() { meta = json!({}); }
    meta["archivedId"] = game["id"].clone();
    Ok(LocalGame { id: format!("archive:{key}"), title: game["title"].as_str().unwrap_or(key).to_string(), store, size: v["size"].as_u64().unwrap_or(0), roots, meta })
}

/// Every game in the archive.
pub fn archive_list(dir: &Path) -> Vec<Value> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let key = e.file_name().to_string_lossy().to_string();
        if !key_ok(&key) { continue; }
        let Some(v) = std::fs::read(e.path().join("game.json")).ok().and_then(|b| serde_json::from_slice::<Value>(&b).ok()) else { continue };
        let g = &v["game"];
        out.push(json!({"id": format!("archive:{key}"), "key": key, "game": g["id"], "title": g["title"], "store": g["store"], "size": v["size"], "files": v["files"],
                        "hasPrefix": v["roots"].as_array().map(|r| r.iter().any(|x| x["kind"] == "prefix")).unwrap_or(false), "from": v["from"], "at": v["at"]}));
    }
    out.sort_by(|a, b| a["title"].as_str().unwrap_or("").to_lowercase().cmp(&b["title"].as_str().unwrap_or("").to_lowercase()));
    out
}

/// A file or folder of a shared folder, as something to copy.
pub fn share_game(full: &Path, share: &str, rel: &str, share_name: &str) -> LocalGame {
    let name = full.file_name().map(|n| n.to_string_lossy().to_string()).filter(|n| !n.is_empty()).unwrap_or_else(|| share_name.to_string());
    LocalGame {
        id: format!("share:{share}:{rel}"), title: name.clone(), store: "share", size: 0,
        roots: vec![Root { kind: "install", path: full.to_path_buf(), skip: vec![] }],
        meta: json!({"folder": name, "file": full.is_file()}),
    }
}

/// Inside a user folder of a Wine/Proton prefix: what isn't a save (caches, temporary files).
const PREFIX_SKIP: &[&str] = &["Temp", "AppData/Local/Temp", "AppData/Local/Microsoft", "AppData/Local/D3DSCache", "AppData/Local/NVIDIA",
    "AppData/Local/AMD", "AppData/Local/CEF", "AppData/LocalLow/Microsoft", "AppData/Roaming/Microsoft"];

/// This device's game saves or screenshots, as something its backup server copies: (label, folder, skip).
pub fn backup_roots(home: &Path, what: &str) -> Vec<(String, PathBuf, Vec<&'static str>)> {
    let mut out: Vec<(String, PathBuf, Vec<&'static str>)> = Vec::new();
    let steam = hearth_core::steam::steam_path(home);
    let userdata: Vec<(String, PathBuf)> = steam.as_ref().map(|s| s.join("userdata")).into_iter()
        .flat_map(|d| std::fs::read_dir(d).into_iter().flatten().flatten())
        .filter_map(|e| { let n = e.file_name().to_string_lossy().to_string(); (n.chars().all(|c| c.is_ascii_digit()) && n != "0" && e.path().is_dir()).then(|| (n, e.path())) })
        .collect();
    if what == "shots" {
        let hearth = hearth_core::gamescope::screenshot_dir();
        if hearth.is_dir() { out.push(("Hearth".into(), hearth, vec![])); }
        for (uid, p) in &userdata {
            let shots = p.join("760/remote");
            if shots.is_dir() { out.push((format!("Steam {uid}"), shots, vec![])); }
        }
        return out;
    }
    // Saves: Steam Cloud's local copies (userdata, without screenshots and Steam's own settings), the Windows user
    // folder of every Proton prefix (Steam's) and Wine prefix (Hearth's, for Epic and GOG).
    for (uid, p) in &userdata { out.push((format!("Steam userdata {uid}"), p.clone(), vec!["760", "config", "7"])); }
    if let Some(s) = &steam {
        for lib in hearth_core::steam::libraries(s) {
            for e in std::fs::read_dir(lib.join("steamapps/compatdata")).into_iter().flatten().flatten() {
                let appid = e.file_name().to_string_lossy().to_string();
                if !appid.chars().all(|c| c.is_ascii_digit()) || appid == "0" { continue; }
                let u = e.path().join("pfx/drive_c/users/steamuser");
                if u.is_dir() && !out.iter().any(|o| o.0 == format!("Steam {appid}")) { out.push((format!("Steam {appid}"), u, PREFIX_SKIP.to_vec())); }
            }
        }
    }
    for e in std::fs::read_dir(hearth_core::paths::data_dir().join("prefixes")).into_iter().flatten().flatten() {
        let n = e.file_name().to_string_lossy().to_string();
        if crate::games::safe_folder(&n).is_none() { continue; }
        for u in [e.path().join("pfx/drive_c/users/steamuser"), e.path().join("drive_c/users/steamuser")] {
            if u.is_dir() { out.push((n.clone(), u, PREFIX_SKIP.to_vec())); break; }
        }
    }
    out
}

pub fn backup_game(home: &Path, what: &str) -> Result<LocalGame, String> {
    let roots = backup_roots(home, what);
    if roots.is_empty() { return Err(if what == "shots" { "No screenshots on this device yet" } else { "No game saves found on this device" }.into()); }
    let labels: Vec<String> = roots.iter().map(|r| r.0.clone()).collect();
    Ok(LocalGame {
        id: format!("backup:{what}"), title: if what == "shots" { "Screenshots".into() } else { "Game saves".into() }, store: "backup", size: 0,
        roots: roots.into_iter().map(|(_, path, skip)| Root { kind: "install", path, skip }).collect(),
        meta: json!({"labels": labels}),
    })
}

/// Where each root of a Hearth Storage copy goes (None: it's a store's game; games.rs decides).
pub fn destinations(kind: &str, game: &Value, roots: &[Value], place: &Path) -> Option<Result<Vec<Option<PathBuf>>, String>> {
    match kind {
        "archive" => Some(Ok((0..roots.len()).map(|i| Some(place.join(format!("root{i}")))).collect())),
        "backup" => Some((|| {
            let labels = game["meta"]["labels"].as_array().ok_or("The backup's folder names didn't come with it")?;
            if labels.len() != roots.len() { return Err("The backup's folder names don't match its folders".to_string()); }
            let mut out = Vec::new();
            for l in labels {
                let f = l.as_str().and_then(crate::games::safe_folder).ok_or("A backup folder name can't be used here")?;
                out.push(Some(place.join(f)));
            }
            Ok(out)
        })()),
        "share" => Some((|| {
            if roots.len() != 1 { return Err("Not a shared file or folder".to_string()); }
            let f = game["meta"]["folder"].as_str().and_then(crate::games::safe_folder).ok_or("That name can't be used here")?;
            Ok(vec![Some(if game["meta"]["file"] == true { place.to_path_buf() } else { place.join(f) })])
        })()),
        _ => None,
    }
}

/// After a Hearth Storage copy is complete (None: a store's game; games.rs finalize).
pub fn finalize(kind: &str, peer: &str, manifest: &Value, place: &Path) -> Option<Result<String, String>> {
    let files = manifest["files"].as_array().map(|f| f.iter().filter(|e| e["k"] == "f").count()).unwrap_or(0);
    let size = manifest["size"].as_u64().unwrap_or(0);
    match kind {
        // Written last: an archive without game.json (a copy that didn't finish) is never offered back.
        "archive" => Some(crate::write_atomic(&place.join("game.json"), json!({"game": manifest["game"], "roots": manifest["roots"], "size": size,
            "files": files, "from": peer, "at": crate::now_ms()}).to_string().as_bytes()).map(|_| "In the archive".to_string())),
        "backup" => Some(crate::write_atomic(&place.join("backup.json"), json!({"size": size, "files": files, "at": crate::now_ms()}).to_string().as_bytes())
            .map(|_| "Backed up".to_string())),
        "share" => Some(Ok(format!("Copied to {}", place.display()))),
        _ => None,
    }
}

/// The devices backed up here: [{device, saves: {at, size, files}, shots: {...}}].
pub fn backups_list(dir: &Path) -> Vec<Value> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        if !e.path().is_dir() { continue; }
        let read = |w: &str| std::fs::read(e.path().join(w).join("backup.json")).ok().and_then(|b| serde_json::from_slice::<Value>(&b).ok()).unwrap_or(Value::Null);
        out.push(json!({"device": e.file_name().to_string_lossy(), "saves": read("Saves"), "shots": read("Screenshots"), "path": e.path()}));
    }
    out
}

// ------------------------------------------------------------------ waking a server

/// This device's network cards (real ones, not virtual), for Wake-on-LAN: [(name, "aa:bb:...")].
pub fn nics() -> Vec<(String, String)> {
    let mut out = Vec::new();
    for e in std::fs::read_dir("/sys/class/net").into_iter().flatten().flatten() {
        let n = e.file_name().to_string_lossy().to_string();
        if n == "lo" || !e.path().join("device").exists() { continue; }
        let mac = std::fs::read_to_string(e.path().join("address")).unwrap_or_default().trim().to_lowercase();
        if valid_mac(&mac) && mac != "00:00:00:00:00:00" { out.push((n, mac)); }
    }
    out.sort();
    out
}

pub fn valid_mac(s: &str) -> bool {
    s.len() == 17 && s.split(':').count() == 6 && s.split(':').all(|p| p.len() == 2 && p.chars().all(|c| c.is_ascii_hexdigit()))
}

/// The Wake-on-LAN "magic packet": 6 × 0xff, then the card's address 16 times.
pub fn magic_packet(mac: &str) -> Option<Vec<u8>> {
    if !valid_mac(mac) { return None; }
    let b: Vec<u8> = mac.split(':').filter_map(|p| u8::from_str_radix(p, 16).ok()).collect();
    let mut out = vec![0xffu8; 6];
    for _ in 0..16 { out.extend_from_slice(&b); }
    Some(out)
}

/// Sends the packet for each address to every local network's broadcast address (ports 9 and 7). Returns how many
/// went out. It only works on the same network: a router doesn't pass broadcasts on.
pub fn wake(macs: &[String]) -> usize {
    let Ok(sock) = std::net::UdpSocket::bind("0.0.0.0:0") else { return 0 };
    let _ = sock.set_broadcast(true);
    let mut n = 0;
    for m in macs {
        let Some(p) = magic_packet(m) else { continue };
        for t in crate::discovery::targets() {
            for port in [9u16, 7] { if sock.send_to(&p, (t, port)).is_ok() { n += 1; } }
        }
    }
    n
}

// ------------------------------------------------------------------ the media links (stream_http)

#[derive(Debug, Clone)]
pub struct Stream { pub server: String, pub share: String, pub path: String, pub size: u64, pub name: String, pub until: u64 }

/// A "Range: bytes=..." header for a file of `size` bytes: Ok(None) the whole file, Ok(Some((first, last)))
/// inclusive, Err(()) a range that can't be served (416).
pub fn parse_range(h: Option<&str>, size: u64) -> Result<Option<(u64, u64)>, ()> {
    let Some(h) = h.map(str::trim) else { return Ok(None) };
    let spec = h.strip_prefix("bytes=").ok_or(())?;
    // Several ranges at once: only the first (players never ask for more than one).
    let spec = spec.split(',').next().unwrap_or("").trim();
    let (a, b) = spec.split_once('-').ok_or(())?;
    if size == 0 { return Err(()); }
    let (first, last) = if a.is_empty() {
        let n: u64 = b.parse().map_err(|_| ())?;
        if n == 0 { return Err(()); }
        (size.saturating_sub(n), size - 1)
    } else {
        let first: u64 = a.parse().map_err(|_| ())?;
        let last: u64 = if b.is_empty() { size - 1 } else { b.parse::<u64>().map_err(|_| ())?.min(size - 1) };
        (first, last)
    };
    if first > last || first >= size { return Err(()); }
    Ok(Some((first, last)))
}

/// The request line and the Range header of an HTTP request's head: (method, path, range).
pub fn parse_head(head: &str) -> Option<(String, String, Option<String>)> {
    let mut lines = head.split("\r\n");
    let mut first = lines.next()?.split(' ');
    let method = first.next()?.to_string();
    let path = first.next()?.to_string();
    let mut range = None;
    for l in lines {
        if let Some((k, v)) = l.split_once(':') { if k.trim().eq_ignore_ascii_case("range") { range = Some(v.trim().to_string()); } }
    }
    Some((method, path, range))
}

pub fn mime(name: &str) -> &'static str {
    let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "mp4" | "m4v" => "video/mp4", "mkv" => "video/x-matroska", "webm" => "video/webm", "avi" => "video/x-msvideo", "mov" => "video/quicktime",
        "ts" => "video/mp2t", "mp3" => "audio/mpeg", "flac" => "audio/flac", "ogg" | "oga" => "audio/ogg", "opus" => "audio/opus", "m4a" => "audio/mp4",
        "wav" => "audio/wav", "aac" => "audio/aac", "jpg" | "jpeg" => "image/jpeg", "png" => "image/png", "gif" => "image/gif", "webp" => "image/webp",
        _ => "application/octet-stream",
    }
}

/// What's playable from a shared folder.
pub fn is_media(name: &str) -> bool { let m = mime(name); m.starts_with("video/") || m.starts_with("audio/") }

/// Reads `len` bytes from `off` as chunk frames (item 0, chunk = piece number), then done (or an error frame).
fn read_range(path: PathBuf, off: u64, len: u64, tx: tokio::sync::mpsc::Sender<bytes::Bytes>) {
    use std::os::unix::fs::FileExt;
    let fail = |m: &str| { let _ = tx.blocking_send(bytes::Bytes::from(wire::control_frame(wire::T_ERROR, m))); };
    let f = match std::fs::File::open(&path) { Ok(f) => f, Err(_) => return fail("That file can't be read") };
    let size = f.metadata().map(|m| m.len()).unwrap_or(0);
    let end = off.saturating_add(len).min(size);
    let mut pos = off;
    let mut i = 0u32;
    while pos < end {
        let n = wire::CHUNK.min(end - pos) as usize;
        let mut buf = vec![0u8; n];
        if f.read_exact_at(&mut buf, pos).is_err() { return fail("That file can't be read"); }
        if tx.blocking_send(bytes::Bytes::from(wire::chunk_frame(0, i, &buf, false))).is_err() { return; }
        pos += n as u64;
        i += 1;
    }
    let _ = tx.blocking_send(bytes::Bytes::from(wire::control_frame(wire::T_DONE, "")));
}

// ------------------------------------------------------------------ the service's side

pub struct Storage {
    pub cfg: Mutex<StorageConfig>,
    /// Paired devices' answers to storage.info: id → (when, answer).
    pub servers: Mutex<HashMap<String, (u64, Value)>>,
    /// Whether a paired device's latest request came over the local network.
    pub local: Mutex<HashMap<String, bool>>,
    pub streams: Mutex<HashMap<String, Stream>>,
    pub http_port: tokio::sync::Mutex<Option<u16>>,
    /// The last time another device used this server (for sleeping when idle).
    pub activity: AtomicU64,
    pub backup_busy: AtomicBool,
    /// What happened when the network cards were set to wake from the network.
    pub wol_note: Mutex<Option<String>>,
    /// The last time a backup was tried (it waits half an hour after one that didn't reach the server).
    pub backup_try: AtomicU64,
}

impl Storage {
    pub fn load(dir: &Path) -> Storage {
        Storage {
            cfg: Mutex::new(StorageConfig::load(dir)), servers: Mutex::new(HashMap::new()), local: Mutex::new(HashMap::new()),
            streams: Mutex::new(HashMap::new()), http_port: tokio::sync::Mutex::new(None), activity: AtomicU64::new(crate::now_ms()),
            backup_busy: AtomicBool::new(false), wol_note: Mutex::new(None), backup_try: AtomicU64::new(0),
        }
    }
    pub fn touch(&self) { self.activity.store(crate::now_ms(), Ordering::Relaxed); }
}

/// The root helper (pkexec, allowed without a prompt by 49-hearth-storage.rules), with `input` on stdin (never on
/// the command line, never logged: it can be a password).
fn helper(args: &[&str], input: &str) -> Result<Value, String> {
    use std::io::Write;
    if !Path::new(HELPER).exists() { return Err("This comes with the next Hearth OS update".into()); }
    let mut c = std::process::Command::new("pkexec").arg(HELPER).args(args)
        .stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::null())
        .spawn().map_err(|e| format!("Couldn't run the helper: {e}"))?;
    if let Some(mut i) = c.stdin.take() { let _ = i.write_all(input.as_bytes()); }
    let o = c.wait_with_output().map_err(|e| e.to_string())?;
    let line = String::from_utf8_lossy(&o.stdout).lines().last().unwrap_or("").to_string();
    let v: Value = serde_json::from_str(&line).unwrap_or(json!({"ok": false, "error": "The helper didn't answer"}));
    if v["ok"] == true { Ok(v) } else { Err(v["error"].as_str().unwrap_or("That didn't work").to_string()) }
}

fn firewall(action: &str, app: &str) {
    if !Path::new(FIREWALL).exists() || std::env::var_os("HEARTH_LINK_NO_FIREWALL").is_some() { return; }
    let (a, p) = (action.to_string(), app.to_string());
    std::thread::spawn(move || {
        if let Ok(o) = std::process::Command::new("pkexec").args([FIREWALL, a.as_str(), p.as_str()]).stdin(std::process::Stdio::null()).output() {
            eprintln!("hearth-link: storage: firewall: {}", String::from_utf8_lossy(&o.stdout).trim());
        }
    });
}

fn smb_apply(c: &StorageConfig) -> Result<Value, String> {
    if !c.smb || !c.server { return helper(&["smb", "off"], ""); }
    let shares: Vec<Value> = c.shares.iter().map(|s| json!({"name": s.name, "path": s.path, "write": s.write})).collect();
    helper(&["smb", "apply"], &json!({"shares": shares}).to_string())
}

/// The person's session has been idle (logind's IdleHint): a server only sleeps then, never under someone using it.
fn session_idle() -> bool {
    let uid = unsafe { libc::getuid() }.to_string();
    std::process::Command::new("loginctl").args(["show-user", uid.as_str(), "-p", "IdleHint", "--value"]).output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "yes").unwrap_or(false)
}

impl Node {
    fn st(&self) -> StorageConfig { crate::lock(&self.storage.cfg).clone() }

    fn st_save(&self, c: StorageConfig) -> Result<(), String> {
        c.save(&self.dir)?;
        *crate::lock(&self.storage.cfg) = c;
        Ok(())
    }

    /// A request from `peer` for this device's Hearth Storage: on, and from home unless Away from home is on.
    fn storage_allowed(&self, peer: &str) -> Result<StorageConfig, String> {
        let c = self.st();
        if !c.server { return Err("Hearth Storage is off on that device".into()); }
        let local = crate::lock(&self.storage.local).get(peer).copied().unwrap_or(false);
        if !local && !c.away { return Err("That device's Hearth Storage only answers at home (Away from home is off on it)".into()); }
        self.storage.touch();
        Ok(c)
    }

    /// Something of Hearth Storage that a copy asks for (archive:, share:, backup:), checked for `peer`.
    pub(crate) fn storage_find(&self, peer: &str, id: &str) -> Result<LocalGame, String> {
        if let Some(key) = id.strip_prefix("archive:") {
            let c = self.storage_allowed(peer)?;
            if !c.archive { return Err("The game archive is off on that device".into()); }
            return archive_game(&c.archive_dir(&self.home), key);
        }
        if let Some(rest) = id.strip_prefix("share:") {
            let c = self.storage_allowed(peer)?;
            let (share, rel) = rest.split_once(':').unwrap_or((rest, ""));
            let (full, sh) = c.resolve(share, rel)?;
            return Ok(share_game(&full, share, rel.trim_matches('/'), &sh.name));
        }
        if let Some(what) = id.strip_prefix("backup:") {
            // Only to the server this device backs up to.
            let c = self.st();
            if c.backup_to.is_empty() || c.backup_to != peer { return Err("This device doesn't back up to that device".into()); }
            if !((what == "saves" && c.backup_saves) || (what == "shots" && c.backup_shots)) { return Err("That backup is off on this device".into()); }
            return backup_game(&self.home, what);
        }
        Err("Not something of Hearth Storage".into())
    }

    pub fn storage_local(&self, peer: &str, local: bool) { crate::lock(&self.storage.local).insert(peer.to_string(), local); }

    /// Requests from paired devices.
    pub async fn storage_answer(self: Arc<Self>, peer: &str, op: &str, req: &Value) -> Result<Value, String> {
        let home = self.home.clone();
        match op {
            "storage.info" => {
                let c = self.st();
                if !c.server { return Ok(json!({"server": false})); }
                let local = crate::lock(&self.storage.local).get(peer).copied().unwrap_or(false);
                Ok(json!({"server": true, "name": self.name(), "away": c.away, "local": local, "archive": c.archive, "backups": c.backups, "media": c.media,
                          "shares": c.shares.iter().filter(|s| s.path.is_dir()).map(|s| json!({"id": s.id, "name": s.name})).collect::<Vec<_>>(),
                          "macs": nics().into_iter().map(|n| n.1).collect::<Vec<_>>(), "free": crate::free_space(&c.root(&home)).unwrap_or(0),
                          "usable": local || c.away}))
            }
            "storage.ls" => {
                let c = self.storage_allowed(peer)?;
                let (share, rel) = (req["share"].as_str().unwrap_or("").to_string(), req["path"].as_str().unwrap_or("").to_string());
                tokio::task::spawn_blocking(move || {
                    let (full, sh) = c.resolve(&share, &rel)?;
                    if !full.is_dir() { return Err("That's a file, not a folder".to_string()); }
                    Ok(json!({"share": sh.name, "path": rel.trim_matches('/'), "entries": list_dir(&full)?}))
                }).await.map_err(|e| e.to_string())?
            }
            "storage.stat" => {
                let c = self.storage_allowed(peer)?;
                let (full, _) = c.resolve(req["share"].as_str().unwrap_or(""), req["path"].as_str().unwrap_or(""))?;
                let md = std::fs::metadata(&full).map_err(|_| "That isn't there anymore".to_string())?;
                Ok(json!({"size": md.len(), "dir": md.is_dir(), "name": full.file_name().map(|n| n.to_string_lossy().to_string())}))
            }
            "archive.offer" => {
                let c = self.storage_allowed(peer)?;
                if !c.archive { return Err("The game archive is off on that device".into()); }
                let mut game = req["game"].clone();
                let gid = game["id"].as_str().filter(|g| !is_special(g) && g.contains(':')).ok_or("No game")?.to_string();
                let key = archive_key(peer, &gid);
                let place = c.archive_dir(&home).join(&key);
                let size = game["size"].as_u64().unwrap_or(0);
                if let Some(free) = crate::free_space(&place) { if size > free { return Err(format!("Not enough space on {}: it needs {} and {} is free", self.name(), crate::human_bytes(size), crate::human_bytes(free))); } }
                // Archived again (an update, new saves): it isn't offered back until the new copy is complete.
                let _ = std::fs::remove_file(place.join("game.json"));
                std::fs::create_dir_all(&place).map_err(|e| format!("{}: {e}", place.display()))?;
                let with_prefix = game["hasPrefix"] == true;
                game["store"] = json!("archive");
                let job = self.start_job(peer, &game, place, with_prefix)?;
                eprintln!("hearth-link: storage: archiving {} from {peer}", job.title);
                Ok(json!({"accepted": true, "key": key}))
            }
            "archive.list" => {
                let c = self.storage_allowed(peer)?;
                if !c.archive { return Err("The game archive is off on that device".into()); }
                let dir = c.archive_dir(&home);
                let mut list = tokio::task::spawn_blocking(move || archive_list(&dir)).await.map_err(|e| e.to_string())?;
                let ps = crate::lock(&self.peers);
                for g in list.iter_mut() {
                    let from = g["from"].as_str().unwrap_or("").to_string();
                    g["fromName"] = json!(if from == self.me { self.name() } else { ps.get(&from).map(|p| p.name.clone()).unwrap_or_else(|| "another device".into()) });
                }
                Ok(json!({"games": list}))
            }
            "archive.remove" => {
                let c = self.storage_allowed(peer)?;
                let key = req["game"].as_str().unwrap_or("").trim_start_matches("archive:").to_string();
                self.archive_remove(&c, &key)
            }
            "backup.offer" => {
                let c = self.storage_allowed(peer)?;
                if !c.backups { return Err("Backups are off on that device".into()); }
                let what = req["what"].as_str().unwrap_or("");
                let (folder, title) = match what { "saves" => ("Saves", "Game saves"), "shots" => ("Screenshots", "Screenshots"), _ => return Err("Back up what?".into()) };
                let gid = format!("backup:{what}");
                if crate::lock(&self.jobs).iter().any(|j| j.peer == peer && j.game_id == gid && !matches!(j.state().as_str(), "done" | "cancelled" | "error")) {
                    return Ok(json!({"accepted": true, "already": true}));
                }
                let name = crate::lock(&self.peers).get(peer).map(|p| p.name.clone()).unwrap_or_default();
                let place = c.backups_dir(&home).join(device_folder(peer, &name)).join(folder);
                std::fs::create_dir_all(&place).map_err(|e| format!("{}: {e}", place.display()))?;
                // Failed earlier: cleared, so the new one can start.
                crate::lock(&self.jobs).retain(|j| !(j.peer == peer && j.game_id == gid && j.state() == "error"));
                self.start_job(peer, &json!({"id": gid, "title": format!("{title} from {name}"), "store": "backup"}), place, false)?;
                Ok(json!({"accepted": true}))
            }
            "archive.done" => {
                // From the server this device moved a game to: checked against the game here before it's called
                // safe to uninstall (same files, same size: not played since, or it says to archive it again).
                let gid = req["game"].as_str().unwrap_or("").to_string();
                let pending = self.st().archived.get(&gid).map(|v| v["server"] == peer).unwrap_or(false);
                if !pending { return Err("This device didn't ask for that".into()); }
                let (h, g2) = (home.clone(), gid.clone());
                let mine = tokio::task::spawn_blocking(move || {
                    let g = crate::games::find(&h, &g2)?;
                    crate::xfer::manifest_for(&g, true).ok()
                }).await.map_err(|e| e.to_string())?;
                let (size, files) = (req["size"].as_u64().unwrap_or(0), req["files"].as_u64().unwrap_or(0));
                let same = mine.as_ref().map(|m| {
                    let n = m["files"].as_array().map(|f| f.iter().filter(|e| e["k"] == "f").count() as u64).unwrap_or(0);
                    m["size"].as_u64() == Some(size) && n == files
                });
                let mut c = self.st();
                if let Some(e) = c.archived.get_mut(&gid) {
                    e["state"] = json!(match same { Some(true) => "archived", Some(false) => "changed", None => "archived" });
                    e["size"] = json!(size);
                    e["at"] = json!(crate::now_ms());
                }
                self.st_save(c)?;
                eprintln!("hearth-link: storage: {gid} is in the archive on {peer} ({})", match same { Some(true) => "matches this device", Some(false) => "changed here since", None => "not on this device anymore" });
                Ok(json!({"ok": true}))
            }
            _ => Err(format!("Unknown request: {op}")),
        }
    }

    fn archive_remove(&self, c: &StorageConfig, key: &str) -> Result<Value, String> {
        if !key_ok(key) { return Err("Not an archived game".into()); }
        let place = c.archive_dir(&self.home).join(key);
        if crate::lock(&self.jobs).iter().any(|j| j.place == place && j.running.load(Ordering::SeqCst)) { return Err("It's being archived right now".into()); }
        if !place.join("game.json").exists() && !place.exists() { return Err("That game isn't in the archive".into()); }
        std::fs::remove_dir_all(&place).map_err(|e| format!("Couldn't remove it: {e}"))?;
        eprintln!("hearth-link: storage: removed {key} from the archive");
        Ok(json!({"ok": true}))
    }

    /// storage.read: a file's bytes as chunk frames.
    pub async fn storage_serve_read(self: &Arc<Self>, peer: &str, req: &Value, s: &mut iroh::endpoint::SendStream) -> Result<(), String> {
        let c = self.storage_allowed(peer)?;
        if !c.media { return Err("Playing from that device is off".into()); }
        let (full, _) = c.resolve(req["share"].as_str().unwrap_or(""), req["path"].as_str().unwrap_or(""))?;
        if !full.is_file() { return Err("That isn't a file".into()); }
        let off = req["offset"].as_u64().unwrap_or(0);
        let len = req["len"].as_u64().unwrap_or(wire::CHUNK).min(READ_MAX);
        let (tx, mut rx) = tokio::sync::mpsc::channel::<bytes::Bytes>(4);
        tokio::task::spawn_blocking(move || read_range(full, off, len, tx));
        while let Some(b) = rx.recv().await {
            s.write_chunk(b).await.map_err(|e| e.to_string())?;
            self.storage.touch();
        }
        Ok(())
    }

    /// A server, woken first when it doesn't answer and this device knows its network cards.
    pub async fn storage_reach(self: &Arc<Self>, id: &str) -> Result<(), String> {
        let first = match self.connect(id).await { Ok(_) => return Ok(()), Err(e) => e };
        let macs: Vec<String> = self.st().servers.get(id).and_then(|v| v["macs"].as_array().cloned()).unwrap_or_default()
            .iter().filter_map(|m| m.as_str().map(str::to_string)).collect();
        if macs.is_empty() { return Err(first); }
        let n = wake(&macs);
        eprintln!("hearth-link: storage: {id} didn't answer: sent it a wake-up ({n} packets)");
        let t = Instant::now();
        while t.elapsed() < Duration::from_secs(75) {
            tokio::time::sleep(Duration::from_secs(4)).await;
            if t.elapsed().as_secs() % 20 < 4 { wake(&macs); }
            if let Ok(Ok(_)) = tokio::time::timeout(Duration::from_secs(8), self.connect(id)).await { return Ok(()); }
        }
        Err(format!("{first}. A wake-up was sent: if it's set to wake from the network, try again in a minute"))
    }

    async fn storage_request(self: &Arc<Self>, id: &str, req: &Value) -> Result<Value, String> {
        if !crate::lock(&self.peers).is_paired(id) { return Err("Not a paired device".into()); }
        self.storage_reach(id).await?;
        self.request(id, req).await.map_err(|e| e.trim_start_matches("connection: ").to_string())
    }

    /// Asks every paired device (or one) whether it's a Hearth Storage server; remembers how to wake the ones that are.
    pub async fn storage_refresh(self: &Arc<Self>, only: Option<&str>) -> Value {
        let ids: Vec<String> = match only { Some(i) => vec![i.to_string()], None => crate::lock(&self.peers).list.iter().map(|p| p.id.clone()).collect() };
        let mut set = tokio::task::JoinSet::new();
        for id in ids {
            let n = self.clone();
            set.spawn(async move {
                let r = tokio::time::timeout(Duration::from_secs(10), n.request(&id, &json!({"op": "storage.info"}))).await;
                (id, r)
            });
        }
        while let Some(r) = set.join_next().await {
            let Ok((id, r)) = r else { continue };
            match r {
                Ok(Ok(v)) => {
                    if v["server"] == true {
                        let mut c = self.st();
                        let macs: Vec<Value> = v["macs"].as_array().cloned().unwrap_or_default().into_iter().filter(|m| m.as_str().map(valid_mac).unwrap_or(false)).collect();
                        c.servers.insert(id.clone(), json!({"name": v["name"], "macs": macs, "at": crate::now_ms()}));
                        let _ = self.st_save(c);
                    } else if self.st().servers.contains_key(&id) {
                        let mut c = self.st(); c.servers.remove(&id); let _ = self.st_save(c);
                    }
                    crate::lock(&self.storage.servers).insert(id, (crate::now_ms(), v));
                }
                // Unknown request: an older Hearth Link, not a server.
                Ok(Err(e)) if e.starts_with("Unknown request") => { crate::lock(&self.storage.servers).insert(id, (crate::now_ms(), json!({"server": false}))); }
                _ => {}
            }
        }
        self.storage_status()
    }

    pub fn storage_status(&self) -> Value {
        let c = self.st();
        let live = crate::lock(&self.storage.servers).clone();
        let peers: Vec<(String, String)> = crate::lock(&self.peers).list.iter().map(|p| (p.id.clone(), p.name.clone())).collect();
        let mut servers = Vec::new();
        for (id, name) in &peers {
            let l = live.get(id);
            let known = c.servers.get(id);
            let is_server = l.map(|(_, v)| v["server"] == true).unwrap_or(known.is_some());
            if !is_server { continue; }
            let v = l.map(|(_, v)| v.clone()).unwrap_or(Value::Null);
            servers.push(json!({"id": id, "name": name, "shares": v["shares"], "archive": v["archive"], "backups": v["backups"], "media": v["media"],
                                "free": v["free"], "usable": v["usable"], "at": l.map(|x| x.0), "canWake": known.map(|k| k["macs"].as_array().map(|m| !m.is_empty()).unwrap_or(false)).unwrap_or(false)}));
        }
        let jobs: Vec<Value> = crate::lock(&self.jobs).iter().filter(|j| j.store == "archive" || j.store == "backup").map(|j| {
            let mut v = j.to_json();
            v["from"] = json!(peers.iter().find(|p| p.0 == j.peer).map(|p| p.1.clone()).unwrap_or_default());
            v
        }).collect();
        json!({"config": c.to_json(&self.home), "servers": servers, "jobs": jobs, "busy": self.storage.backup_busy.load(Ordering::Relaxed),
               "nics": if c.server { nics().into_iter().map(|n| json!({"name": n.0, "mac": n.1})).collect::<Vec<_>>() } else { vec![] },
               "wol": crate::lock(&self.storage.wol_note).clone(), "user": std::env::var("USER").unwrap_or_default()})
    }

    /// A copy of something of Hearth Storage finished.
    pub async fn storage_job_done(self: &Arc<Self>, job: &crate::xfer::Job) {
        match job.store.as_str() {
            "archive" => {
                let v: Value = std::fs::read(job.place.join("game.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null);
                let req = json!({"op": "archive.done", "game": v["game"]["id"], "size": v["size"], "files": v["files"]});
                if let Err(e) = tokio::time::timeout(Duration::from_secs(30), self.request(&job.peer, &req)).await.unwrap_or(Err("no answer".into())) {
                    eprintln!("hearth-link: storage: telling {} that {} is archived: {e}", job.peer, job.title);
                }
                crate::lock(&self.jobs).retain(|j| j.id != job.id);
            }
            "backup" => {
                eprintln!("hearth-link: storage: backed up {}", job.title);
                crate::lock(&self.jobs).retain(|j| j.id != job.id);
            }
            _ => {}
        }
    }

    /// This device's backup, now: the server is woken if needed and pulls what changed.
    pub async fn storage_backup_now(self: &Arc<Self>) -> Result<Value, String> {
        let c = self.st();
        if c.backup_to.is_empty() { return Err("Pick a Hearth Storage device to back up to first".into()); }
        if self.storage.backup_busy.swap(true, Ordering::SeqCst) { return Ok(json!({"busy": true})); }
        self.storage.backup_try.store(crate::now_ms(), Ordering::Relaxed);
        let r = async {
            let mut out = serde_json::Map::new();
            for (what, on) in [("saves", c.backup_saves), ("shots", c.backup_shots)] {
                if !on { continue; }
                let v = self.storage_request(&c.backup_to, &json!({"op": "backup.offer", "what": what})).await?;
                out.insert(what.to_string(), v);
            }
            Ok::<Value, String>(Value::Object(out))
        }.await;
        self.storage.backup_busy.store(false, Ordering::SeqCst);
        if r.is_ok() {
            let mut c = self.st();
            c.last_backup = crate::now_ms();
            let _ = self.st_save(c);
        }
        r
    }

    /// Backups when they're due, servers' details now and then, and a server's sleep when it's idle.
    pub async fn storage_loop(self: Arc<Self>) {
        let mut last_tick = Instant::now();
        let mut last_refresh: Option<Instant> = None;
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
            // Back from sleep (the minute took much longer): count that as activity, or it would sleep again at once.
            if last_tick.elapsed() > Duration::from_secs(180) { self.storage.touch(); }
            last_tick = Instant::now();
            if last_refresh.map(|t| t.elapsed() > Duration::from_secs(600)).unwrap_or(true) && !crate::lock(&self.peers).list.is_empty() {
                last_refresh = Some(Instant::now());
                let _ = self.storage_refresh(None).await;
            }
            let c = self.st();
            let now = crate::now_ms();
            let due = !c.backup_to.is_empty() && (c.backup_saves || c.backup_shots) && now.saturating_sub(c.last_backup) > c.backup_hours * 3_600_000
                && now.saturating_sub(self.storage.backup_try.load(Ordering::Relaxed)) > 30 * 60_000 && !crate::game_running();
            if due && crate::lock(&self.peers).is_paired(&c.backup_to) {
                match self.storage_backup_now().await {
                    Ok(_) => eprintln!("hearth-link: storage: backup started"),
                    Err(e) => eprintln!("hearth-link: storage: backup: {e}"),
                }
            }
            if c.server && c.sleep_minutes > 0 {
                let idle = now.saturating_sub(self.storage.activity.load(Ordering::Relaxed)) > c.sleep_minutes * 60_000;
                let copying = crate::lock(&self.jobs).iter().any(|j| j.running.load(Ordering::SeqCst));
                let sending = crate::lock(&self.sending).values().any(|s| now.saturating_sub(s.at) < 120_000);
                if idle && !copying && !sending && !crate::game_running() && tokio::task::spawn_blocking(session_idle).await.unwrap_or(false) {
                    eprintln!("hearth-link: storage: nothing to do for {} minutes: going to sleep (devices wake it when they need it)", c.sleep_minutes);
                    self.storage.touch();
                    let _ = tokio::task::spawn_blocking(|| std::process::Command::new("systemctl").arg("suspend").status()).await;
                }
            }
        }
    }

    /// At start: Windows sharing's firewall rule again (firewalld's rules are only until the next start).
    pub fn storage_start(self: &Arc<Self>) {
        let c = self.st();
        if c.server && c.smb { firewall("open", "smb"); }
        let n = self.clone();
        tokio::spawn(async move { n.storage_loop().await });
    }

    fn storage_setup_wake(self: &Arc<Self>) {
        let n = self.clone();
        std::thread::spawn(move || {
            let mut notes = Vec::new();
            for (name, _) in nics() {
                match helper(&["wol", name.as_str()], "") { Ok(v) => notes.push(v["note"].as_str().unwrap_or("ready").to_string()), Err(e) => notes.push(format!("{name}: {e}")) }
            }
            *crate::lock(&n.storage.wol_note) = Some(if notes.is_empty() { "No network card that can wake it".into() } else { notes.join("; ") });
        });
    }

    /// The local web server for media links (127.0.0.1 only), started the first time one is made.
    async fn storage_http(self: &Arc<Self>) -> Result<u16, String> {
        let mut p = self.storage.http_port.lock().await;
        if let Some(port) = *p { return Ok(port); }
        let l = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await.map_err(|e| e.to_string())?;
        let port = l.local_addr().map_err(|e| e.to_string())?.port();
        let n = self.clone();
        tokio::spawn(async move {
            loop {
                let Ok((sock, from)) = l.accept().await else { continue };
                if !from.ip().is_loopback() { continue; }
                let n = n.clone();
                tokio::spawn(async move { if let Err(e) = n.stream_http(sock).await { if e != "closed" { eprintln!("hearth-link: storage: playing: {e}"); } } });
            }
        });
        *p = Some(port);
        Ok(port)
    }

    /// One request from a player: GET or HEAD /<link>/<name>, with or without a range. One request per connection.
    async fn stream_http(self: Arc<Self>, mut sock: tokio::net::TcpStream) -> Result<(), String> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let mut buf = Vec::new();
        let mut tmp = [0u8; 2048];
        loop {
            let n = tokio::time::timeout(Duration::from_secs(30), sock.read(&mut tmp)).await.map_err(|_| "closed".to_string())?.map_err(|e| e.to_string())?;
            if n == 0 { return Err("closed".into()); }
            buf.extend_from_slice(&tmp[..n]);
            if buf.windows(4).any(|w| w == b"\r\n\r\n") { break; }
            if buf.len() > 16384 { return Err("closed".into()); }
        }
        let head = String::from_utf8_lossy(&buf).to_string();
        let (method, path, range) = parse_head(&head).ok_or("closed")?;
        let token = path.trim_start_matches('/').split('/').next().unwrap_or("").to_string();
        let st = crate::lock(&self.storage.streams).get(&token).cloned().filter(|s| s.until > crate::now_ms());
        let Some(st) = st else {
            let _ = sock.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await;
            return Ok(());
        };
        let (status, first, last) = match parse_range(range.as_deref(), st.size) {
            Ok(None) => ("200 OK", 0, st.size.saturating_sub(1)),
            Ok(Some((a, b))) => ("206 Partial Content", a, b),
            Err(()) => {
                let _ = sock.write_all(format!("HTTP/1.1 416 Range Not Satisfiable\r\nContent-Range: bytes */{}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n", st.size).as_bytes()).await;
                return Ok(());
            }
        };
        let len = if st.size == 0 { 0 } else { last - first + 1 };
        let mut h = format!("HTTP/1.1 {status}\r\nContent-Type: {}\r\nContent-Length: {len}\r\nAccept-Ranges: bytes\r\nConnection: close\r\n", mime(&st.name));
        if status.starts_with("206") { h.push_str(&format!("Content-Range: bytes {first}-{last}/{}\r\n", st.size)); }
        h.push_str("\r\n");
        sock.write_all(h.as_bytes()).await.map_err(|_| "closed".to_string())?;
        if method == "HEAD" || len == 0 { return Ok(()); }
        // The next piece is asked for while this one is written, so the player isn't kept waiting between pieces.
        let mut pos = first;
        let end = last + 1;
        let piece = |n: Arc<Node>, st: Stream, off: u64, len: u64| tokio::spawn(async move { n.read_remote(&st.server, &st.share, &st.path, off, len).await });
        let mut next = Some(piece(self.clone(), st.clone(), pos, READ_MAX.min(end - pos)));
        while let Some(h) = next.take() {
            let data = h.await.map_err(|e| e.to_string())??;
            if data.is_empty() { return Err("The file ended early".into()); }
            pos += data.len() as u64;
            if pos < end { next = Some(piece(self.clone(), st.clone(), pos, READ_MAX.min(end - pos))); }
            sock.write_all(&data).await.map_err(|_| "closed".to_string())?;
        }
        Ok(())
    }

    /// Bytes of a shared file on a server (storage.read).
    pub async fn read_remote(self: &Arc<Self>, server: &str, share: &str, path: &str, off: u64, len: u64) -> Result<Vec<u8>, String> {
        let conn = self.connect(server).await?;
        let (mut s, mut r) = conn.open_bi().await.map_err(|e| format!("The connection broke: {e}"))?;
        wire::write_msg(&mut s, &json!({"op": "storage.read", "share": share, "path": path, "offset": off, "len": len})).await?;
        let _ = s.finish();
        let mut out = Vec::with_capacity(len as usize);
        let mut hb = [0u8; wire::HEADER];
        loop {
            r.read_exact(&mut hb).await.map_err(|e| format!("The connection broke: {e}"))?;
            let h = wire::Header::decode(&hb)?;
            if h.kind == wire::T_DONE { return Ok(out); }
            let mut payload = vec![0u8; h.wire as usize];
            r.read_exact(&mut payload).await.map_err(|e| format!("The connection broke: {e}"))?;
            if h.kind == wire::T_ERROR { return Err(String::from_utf8_lossy(&payload).to_string()); }
            out.extend_from_slice(&wire::open_chunk(&h, payload)?);
            if out.len() as u64 > len { return Err("More came than was asked for".into()); }
        }
    }

    /// The UI's storage.* commands.
    pub async fn storage_command(self: &Arc<Self>, cmd: &str, a: &Value) -> Result<Value, String> {
        let s = |k: &str| a[k].as_str().filter(|x| !x.is_empty()).map(str::to_string).ok_or(format!("Missing '{k}'"));
        let home = self.home.clone();
        match cmd {
            "storage.status" => Ok(self.storage_status()),
            "storage.servers" => Ok(self.storage_refresh(a["peer"].as_str()).await),
            "storage.set" => {
                let before = self.st();
                let mut c = before.clone();
                c.apply(a, &home)?;
                if !c.backup_to.is_empty() && !crate::lock(&self.peers).is_paired(&c.backup_to) { return Err("That isn't a paired device".into()); }
                self.st_save(c.clone())?;
                if c.server != before.server { eprintln!("hearth-link: storage: this device is {} Hearth Storage", if c.server { "now" } else { "no longer" }); }
                if c.server && !before.server { self.storage_setup_wake(); }
                let smb_on = c.server && c.smb;
                if smb_on != (before.server && before.smb) {
                    let c2 = c.clone();
                    let r = tokio::task::spawn_blocking(move || smb_apply(&c2)).await.map_err(|e| e.to_string())?;
                    if let Err(e) = r {
                        // Not left saying it's on when it isn't.
                        let mut back = self.st(); back.smb = false; let _ = self.st_save(back);
                        return Err(format!("Windows sharing: {e}"));
                    }
                    firewall(if smb_on { "open" } else { "close" }, "smb");
                }
                Ok(self.storage_status())
            }
            "storage.candidates" => Ok(json!({"folders": candidates(&home)})),
            "storage.share.add" => {
                let mut c = self.st();
                let sh = c.add_share(&s("path")?, a["name"].as_str().unwrap_or(""), a["write"] == true, &home)?;
                self.st_save(c.clone())?;
                if c.server && c.smb { let _ = tokio::task::spawn_blocking(move || smb_apply(&c)).await; }
                Ok(json!({"share": sh}))
            }
            "storage.share.remove" | "storage.share.set" => {
                let id = s("id")?;
                let mut c = self.st();
                if !c.shares.iter().any(|x| x.id == id) { return Err("That folder isn't shared".into()); }
                if cmd == "storage.share.remove" { c.shares.retain(|x| x.id != id); }
                else if let Some(sh) = c.shares.iter_mut().find(|x| x.id == id) {
                    if let Some(w) = a["write"].as_bool() { sh.write = w; }
                    if let Some(n) = a["name"].as_str() { let n: String = n.trim().chars().filter(|c| !c.is_control() && *c != '/').take(40).collect(); if !n.is_empty() { sh.name = n; } }
                }
                self.st_save(c.clone())?;
                if c.server && c.smb { let _ = tokio::task::spawn_blocking(move || smb_apply(&c)).await; }
                Ok(self.storage_status())
            }
            "storage.smb.password" => {
                // Straight to the helper's stdin; never logged, never kept here.
                let pw = a["password"].as_str().unwrap_or("").to_string();
                if pw.chars().count() < 8 || pw.contains('\n') { return Err("Use at least 8 characters".into()); }
                tokio::task::spawn_blocking(move || helper(&["smb-password"], &(pw + "\n"))).await.map_err(|e| e.to_string())??;
                Ok(json!({"ok": true}))
            }
            "storage.ls" => {
                let server = s("server")?;
                self.storage_request(&server, &json!({"op": "storage.ls", "share": s("share")?, "path": a["path"].as_str().unwrap_or("")})).await
            }
            "storage.get" => {
                // A file or folder of a shared folder copied here (Downloads unless a folder is given).
                let server = s("server")?;
                if !crate::lock(&self.peers).is_paired(&server) { return Err("Not a paired device".into()); }
                let share = s("share")?;
                let rel = a["path"].as_str().unwrap_or("").trim_matches('/').to_string();
                if share.contains(':') { return Err("That isn't a shared folder".into()); }
                let place = match a["place"].as_str().filter(|p| !p.is_empty()) {
                    Some(p) => { let p = PathBuf::from(p); if !p.is_absolute() || !p.is_dir() || !place_ok(&p, &home) { return Err("Pick a folder in Home or on a drive".into()); } p }
                    None => { let d = home.join("Downloads"); std::fs::create_dir_all(&d).map_err(|e| e.to_string())?; d }
                };
                self.storage_reach(&server).await?;
                let title = rel.rsplit('/').next().filter(|x| !x.is_empty()).unwrap_or("Shared folder").to_string();
                let job = self.start_job(&server, &json!({"id": format!("share:{share}:{rel}"), "title": title, "store": "share"}), place, false)?;
                Ok(job.to_json())
            }
            "storage.stream" => {
                let server = s("server")?;
                let (share, path) = (s("share")?, s("path")?);
                let st = self.storage_request(&server, &json!({"op": "storage.stat", "share": share, "path": path})).await?;
                if st["dir"] == true { return Err("That's a folder".into()); }
                let name = st["name"].as_str().unwrap_or("file").to_string();
                let port = self.storage_http().await?;
                let token = crate::hex(&crate::random_bytes::<16>());
                let mut m = crate::lock(&self.storage.streams);
                m.retain(|_, v| v.until > crate::now_ms());
                m.insert(token.clone(), Stream { server, share, path, size: st["size"].as_u64().unwrap_or(0), name: name.clone(), until: crate::now_ms() + STREAM_LIFETIME_MS });
                let safe: String = name.chars().map(|c| if c.is_ascii_alphanumeric() || ".-_".contains(c) { c } else { '_' }).collect();
                Ok(json!({"url": format!("http://127.0.0.1:{port}/{token}/{safe}"), "name": name, "media": is_media(&name)}))
            }
            "storage.archive" => {
                let server = s("server")?;
                let gid = s("game")?;
                if is_special(&gid) { return Err("That isn't a game on this device".into()); }
                let game = self.local_game(&self.me, &gid).await?;
                let v = self.storage_request(&server, &json!({"op": "archive.offer", "game": game.to_json()})).await?;
                let mut c = self.st();
                c.archived.insert(gid.clone(), json!({"server": server, "title": game.title, "size": game.size, "at": crate::now_ms(), "state": "archiving", "key": v["key"]}));
                self.st_save(c)?;
                Ok(v)
            }
            "storage.archived" => {
                let server = s("server")?;
                self.storage_request(&server, &json!({"op": "archive.list"})).await
            }
            "storage.archive.remove" => {
                let server = s("server")?;
                let gid = s("game")?;
                let v = if server == self.me { let c = self.st(); self.archive_remove(&c, gid.trim_start_matches("archive:"))? }
                        else { self.storage_request(&server, &json!({"op": "archive.remove", "game": gid})).await? };
                Ok(v)
            }
            "storage.archive.forget" => {
                // This device's note that a game is archived (after it's back, or removed from the archive).
                let gid = s("game")?;
                let mut c = self.st(); c.archived.remove(&gid); self.st_save(c)?;
                Ok(Value::Null)
            }
            "storage.restore" => {
                let server = s("server")?;
                let gid = s("game")?;
                let list = self.storage_request(&server, &json!({"op": "archive.list"})).await?;
                let g = list["games"].as_array().and_then(|l| l.iter().find(|g| g["id"] == gid.as_str()).cloned()).ok_or("That game isn't in the archive anymore")?;
                let store = g["store"].as_str().unwrap_or("").to_string();
                let place = crate::games::check_place(&home, &store, a["place"].as_str())?;
                let size = g["size"].as_u64().unwrap_or(0);
                if size > 0 { if let Some(free) = crate::free_space(&place) { if size > free { return Err(format!("Not enough space there: it needs {} and {} is free", crate::human_bytes(size), crate::human_bytes(free))); } } }
                let job = self.start_job(&server, &json!({"id": gid, "title": g["title"], "store": store, "size": size}), place, a["prefix"] != false && g["hasPrefix"] == true)?;
                Ok(job.to_json())
            }
            "storage.backup.now" => self.storage_backup_now().await,
            "storage.backups" => {
                let c = self.st();
                let dir = c.backups_dir(&home);
                Ok(json!({"backups": tokio::task::spawn_blocking(move || backups_list(&dir)).await.map_err(|e| e.to_string())?}))
            }
            "storage.archive.here" => {
                // This server's own archive, for its own Settings page.
                let c = self.st();
                let dir = c.archive_dir(&home);
                let mut list = tokio::task::spawn_blocking(move || archive_list(&dir)).await.map_err(|e| e.to_string())?;
                let ps = crate::lock(&self.peers);
                for g in list.iter_mut() { let from = g["from"].as_str().unwrap_or("").to_string(); g["fromName"] = json!(ps.get(&from).map(|p| p.name.clone()).unwrap_or_else(|| "another device".into())); }
                Ok(json!({"games": list}))
            }
            "storage.wake" => {
                let server = s("server")?;
                let macs: Vec<String> = self.st().servers.get(&server).and_then(|v| v["macs"].as_array().cloned()).unwrap_or_default()
                    .iter().filter_map(|m| m.as_str().map(str::to_string)).collect();
                if macs.is_empty() { return Err("This device doesn't know how to wake it yet: it learns that the next time both are on".into()); }
                Ok(json!({"sent": wake(&macs)}))
            }
            _ => Err(format!("Unknown command: {cmd}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_paths_stay_inside_and_hide_dot_files() {
        let d = tempfile::tempdir().unwrap();
        let home = d.path();
        std::fs::create_dir_all(home.join("Videos/Films")).unwrap();
        std::fs::write(home.join("Videos/Films/a.mkv"), b"x").unwrap();
        std::fs::create_dir_all(home.join("Videos/.secret")).unwrap();
        std::fs::create_dir_all(home.join(".ssh")).unwrap();
        std::fs::write(home.join(".ssh/id"), b"k").unwrap();
        std::os::unix::fs::symlink(home.join(".ssh"), home.join("Videos/out")).unwrap();
        let mut c = StorageConfig::default();
        let sh = c.add_share(home.join("Videos").to_str().unwrap(), "", false, home).unwrap();
        assert_eq!(sh.name, "Videos");
        assert!(c.add_share(home.join("Videos").to_str().unwrap(), "", false, home).is_err(), "twice");
        assert!(c.add_share(home.join(".ssh").to_str().unwrap(), "", false, home).is_err(), "hidden");
        assert!(c.add_share("/etc", "", false, home).is_err(), "outside Home and drives");
        let (p, _) = c.resolve(&sh.id, "Films/a.mkv").unwrap();
        assert!(p.ends_with("Films/a.mkv"));
        assert!(c.resolve(&sh.id, "../.ssh/id").is_err());
        assert!(c.resolve(&sh.id, ".secret").is_err());
        assert!(c.resolve(&sh.id, "out/id").is_err(), "a link out of the shared folder");
        assert!(c.resolve("nope", "").is_err());
        let names: Vec<String> = list_dir(&home.join("Videos")).unwrap().iter().map(|e| e["name"].as_str().unwrap().to_string()).collect();
        assert_eq!(names[0], "Films", "folders first");
        assert!(!names.iter().any(|n| n.starts_with('.')));
    }

    #[test]
    fn settings_are_checked() {
        let d = tempfile::tempdir().unwrap();
        let mut c = StorageConfig::default();
        assert!(!c.server && !c.smb && !c.away, "off by default, nothing opened");
        c.apply(&json!({"server": true, "sleepMinutes": 99999, "backupHours": 0, "backupTo": "abc123"}), d.path()).unwrap();
        assert_eq!(c.sleep_minutes, 720);
        assert_eq!(c.backup_hours, 1);
        assert!(c.apply(&json!({"backupTo": "not hex!"}), d.path()).is_err());
        assert!(c.apply(&json!({"folder": "/etc/x"}), d.path()).is_err());
        assert!(c.apply(&json!({"folder": "relative"}), d.path()).is_err());
        c.apply(&json!({"folder": d.path().join("Store").to_str().unwrap()}), d.path()).unwrap();
        assert_eq!(c.archive_dir(d.path()), d.path().join("Store/Archive"));
        c.save(d.path()).unwrap();
        assert!(StorageConfig::load(d.path()).server);
    }

    #[test]
    fn archived_games_come_back_as_their_store() {
        let d = tempfile::tempdir().unwrap();
        let key = archive_key("abcdef0123456789", "steam:620");
        assert_eq!(key, "abcdef01-steam-620");
        assert!(key_ok(&key) && !key_ok("../x") && !key_ok(".x") && !key_ok(""));
        let place = d.path().join(&key);
        let manifest = json!({"game": {"id": "steam:620", "title": "Portal 2", "store": "steam", "meta": {"appid": "620", "installdir": "Portal 2"}},
                              "roots": [{"kind": "install"}, {"kind": "prefix"}], "size": 10, "files": [{"k": "f", "p": "a", "r": 0, "s": 10}]});
        let dests = destinations("archive", &manifest["game"], manifest["roots"].as_array().unwrap(), &place).unwrap().unwrap();
        assert_eq!(dests, vec![Some(place.join("root0")), Some(place.join("root1"))]);
        assert!(archive_list(d.path()).is_empty(), "not offered before it's complete");
        std::fs::create_dir_all(&place).unwrap();
        finalize("archive", "abcdef0123456789", &manifest, &place).unwrap().unwrap();
        let l = archive_list(d.path());
        assert_eq!(l.len(), 1);
        assert_eq!(l[0]["store"], "steam");
        assert_eq!(l[0]["files"], 1);
        assert_eq!(l[0]["hasPrefix"], true);
        let g = archive_game(d.path(), &key).unwrap();
        assert_eq!(g.store, "steam");
        assert_eq!(g.meta["archivedId"], "steam:620");
        assert_eq!(g.roots[1].kind, "prefix");
        assert!(archive_game(d.path(), "../../etc").is_err());
        // A store's game: games.rs decides.
        assert!(destinations("steam", &manifest["game"], &[], &place).is_none());
    }

    #[test]
    fn backups_and_shared_copies_land_in_their_folders() {
        let place = Path::new("/srv/b");
        let g = json!({"meta": {"labels": ["Steam 620", "Hearth"]}});
        let d = destinations("backup", &g, &[json!({}), json!({})], place).unwrap().unwrap();
        assert_eq!(d[1], Some(place.join("Hearth")));
        assert!(destinations("backup", &json!({"meta": {"labels": ["../x"]}}), &[json!({})], place).unwrap().is_err());
        assert!(destinations("backup", &g, &[json!({})], place).unwrap().is_err(), "labels must match the roots");
        let f = destinations("share", &json!({"meta": {"folder": "a.mkv", "file": true}}), &[json!({})], place).unwrap().unwrap();
        assert_eq!(f[0], Some(place.to_path_buf()));
        let f = destinations("share", &json!({"meta": {"folder": "Films"}}), &[json!({})], place).unwrap().unwrap();
        assert_eq!(f[0], Some(place.join("Films")));
        assert_eq!(device_folder("abcdef0123", "Steam Deck/.."), "Steam Deck--- (abcdef01)");
    }

    #[test]
    fn wake_on_lan_packets() {
        assert!(valid_mac("aa:bb:cc:00:11:22") && !valid_mac("aa:bb:cc:00:11") && !valid_mac("zz:bb:cc:00:11:22"));
        let p = magic_packet("aa:bb:cc:00:11:22").unwrap();
        assert_eq!(p.len(), 102);
        assert_eq!(&p[..6], &[0xff; 6]);
        assert_eq!(&p[6..12], &[0xaa, 0xbb, 0xcc, 0x00, 0x11, 0x22]);
        assert_eq!(&p[96..102], &[0xaa, 0xbb, 0xcc, 0x00, 0x11, 0x22]);
        assert!(magic_packet("nope").is_none());
    }

    #[test]
    fn ranges_for_players() {
        assert_eq!(parse_range(None, 100), Ok(None));
        assert_eq!(parse_range(Some("bytes=0-"), 100), Ok(Some((0, 99))));
        assert_eq!(parse_range(Some("bytes=10-19"), 100), Ok(Some((10, 19))));
        assert_eq!(parse_range(Some("bytes=90-500"), 100), Ok(Some((90, 99))));
        assert_eq!(parse_range(Some("bytes=-10"), 100), Ok(Some((90, 99))));
        assert_eq!(parse_range(Some("bytes=100-"), 100), Err(()));
        assert_eq!(parse_range(Some("bytes=5-2"), 100), Err(()));
        assert_eq!(parse_range(Some("items=0-1"), 100), Err(()));
        let (m, p, r) = parse_head("GET /abc/film.mkv HTTP/1.1\r\nHost: 127.0.0.1\r\nrange: bytes=5-\r\n\r\n").unwrap();
        assert_eq!((m.as_str(), p.as_str(), r.as_deref()), ("GET", "/abc/film.mkv", Some("bytes=5-")));
        assert!(is_media("Film.MKV") && is_media("song.flac") && !is_media("notes.txt"));
    }

    #[test]
    fn read_range_sends_checked_pieces() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("f.bin");
        let data: Vec<u8> = (0..(3 * wire::CHUNK as usize / 2)).map(|i| (i % 251) as u8).collect();
        std::fs::write(&f, &data).unwrap();
        let (tx, mut rx) = tokio::sync::mpsc::channel(16);
        std::thread::spawn(move || read_range(f, 100, wire::CHUNK + 10, tx));
        let mut out = Vec::new();
        while let Some(b) = rx.blocking_recv() {
            let h = wire::Header::decode(b[..wire::HEADER].try_into().unwrap()).unwrap();
            if h.kind == wire::T_DONE { break; }
            out.extend_from_slice(&wire::open_chunk(&h, b[wire::HEADER..].to_vec()).unwrap());
        }
        assert_eq!(out, data[100..100 + wire::CHUNK as usize + 10].to_vec());
    }
}
