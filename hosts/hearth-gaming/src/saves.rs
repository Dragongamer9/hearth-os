//! Hearth Saves: find a game's saves (locate.rs with the manifest), back them up as one zip per game, restore a
//! backup (after a safety copy of what's there now), and back up on a schedule.
//!
//! A backup is `<destination>/Hearth Saves/<Game>/<Game> <date time>.zip` holding `hearth-saves.json` (what it is)
//! and `files/<place>/<path>` (each file under its portable name, locate.rs), so it restores on another Hearth PC.

use crate::games::GameEntry;
use crate::locate::{self, Place};
use crate::manifest::Manifest;
use crate::util;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub const FOLDER: &str = "Hearth Saves";
const META: &str = "hearth-saves.json";

// ------------------------------------------------------------------ finding

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Scan {
    pub id: String,
    pub files: Vec<PathBuf>,
    pub count: usize,
    pub size: u64,
    /// The newest file's time (ms).
    pub newest: u64,
    /// The manifest's name for the game (None: it isn't in the manifest).
    pub known_as: Option<String>,
    /// The folders the files are in, with how many and how big: what the page shows.
    pub folders: Vec<Folder>,
    /// Folders you added for this game.
    pub custom: Vec<PathBuf>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Folder { pub path: PathBuf, pub count: usize, pub size: u64 }

/// One game's saves on this PC.
pub fn scan(g: &GameEntry, m: Option<&Manifest>, custom: &[PathBuf]) -> Scan {
    let place = g.place();
    let entry = if g.kind == "emulator" { None } else { m.and_then(|m| m.find(g.steam_app, g.gog_id, &g.title, g.folder_name().as_deref())) };
    let mut extra: Vec<PathBuf> = g.save_dirs.clone();
    extra.extend(custom.iter().cloned());
    let files = locate::find_files(&place, entry.map(|e| e.1), &extra);
    let mut folders: BTreeMap<PathBuf, (usize, u64)> = BTreeMap::new();
    let (mut size, mut newest) = (0u64, 0u64);
    for f in &files {
        let md = std::fs::metadata(f).ok();
        let s = md.as_ref().map(|m| m.len()).unwrap_or(0);
        newest = newest.max(md.as_ref().map(util::mtime_ms).unwrap_or(0));
        size += s;
        let d = folders.entry(f.parent().map(Path::to_path_buf).unwrap_or_default()).or_default();
        d.0 += 1;
        d.1 += s;
    }
    Scan { id: g.id.clone(), count: files.len(), files, size, newest, known_as: entry.map(|e| e.0.to_string()),
           folders: folders.into_iter().map(|(path, (count, size))| Folder { path, count, size }).collect(), custom: custom.to_vec() }
}

/// What changes when the saves change (for skipping unchanged games in scheduled backups).
pub fn fingerprint(s: &Scan) -> String { format!("{}:{}:{}", s.count, s.size, s.newest) }

// ------------------------------------------------------------------ backing up

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Backup {
    pub file: PathBuf,
    pub id: String,
    pub title: String,
    pub created: u64,
    pub host: String,
    pub count: usize,
    pub bytes: u64,
    pub auto: bool,
    pub safety: bool,
}

fn backup_from_meta(file: &Path, meta: &Value) -> Backup {
    Backup {
        file: file.to_path_buf(), id: meta["id"].as_str().unwrap_or("").into(), title: meta["title"].as_str().unwrap_or("Game").into(),
        created: meta["created"].as_u64().unwrap_or(0), host: meta["host"].as_str().unwrap_or("").into(),
        count: meta["files"].as_array().map(|a| a.len()).unwrap_or(0), bytes: std::fs::metadata(file).map(|m| m.len()).unwrap_or(0),
        auto: meta["auto"] == true, safety: meta["safety"] == true,
    }
}

/// Zips one game's saves into `<dest>/Hearth Saves/<Game>/`. `kind`: "" (by you), "auto" or "safety".
pub fn backup(g: &GameEntry, files: &[PathBuf], dest: &Path, kind: &str) -> Result<Backup, String> {
    if files.is_empty() { return Err(format!("No saves found for {} on this PC.", g.title)); }
    let place = g.place();
    let dir = dest.join(FOLDER).join(util::safe_name(&g.title));
    std::fs::create_dir_all(&dir).map_err(|e| format!("Couldn't make a folder on {} ({e}). Is the drive full or read-only?", dest.display()))?;
    let base = format!("{} {}{}", util::safe_name(&g.title), util::stamp(), match kind { "auto" => " (automatic)", "safety" => " (before restore)", _ => "" });
    let mut file = dir.join(format!("{base}.zip"));
    for n in 2..100 { if !file.exists() { break; } file = dir.join(format!("{base} ({n}).zip")); }
    let part = file.with_extension("zip.partial");
    let mut listed = Vec::new();
    let r = (|| -> Result<(), String> {
        let out = std::fs::File::create(&part).map_err(|e| format!("Couldn't write the backup ({e})"))?;
        let mut z = zip::ZipWriter::new(std::io::BufWriter::new(out));
        let mut used = std::collections::HashSet::new();
        for f in files {
            let Some((root, rel)) = place.portable(f) else { continue };
            let name = format!("files/{root}/{}", rel.to_string_lossy());
            if !used.insert(name.clone()) { continue; }
            let md = std::fs::metadata(f).map_err(|e| format!("Couldn't read {} ({e})", f.display()))?;
            let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated).large_file(md.len() >= 0xFFFF_0000);
            z.start_file(name.clone(), opts).map_err(|e| e.to_string())?;
            let mut src = std::fs::File::open(f).map_err(|e| format!("Couldn't read {} ({e})", f.display()))?;
            std::io::copy(&mut src, &mut z).map_err(|e| format!("Couldn't copy {} ({e})", f.display()))?;
            listed.push(json!({"path": name, "size": md.len(), "mtime": util::mtime_ms(&md), "from": f}));
        }
        let meta = json!({"format": 1, "app": "Hearth Saves", "id": g.id, "title": g.title, "source": g.source, "steamApp": g.steam_app,
            "created": util::now_ms(), "createdText": util::iso_now(), "host": util::host_name(), "auto": kind == "auto", "safety": kind == "safety", "files": listed});
        z.start_file(META, zip::write::SimpleFileOptions::default()).map_err(|e| e.to_string())?;
        z.write_all(serde_json::to_string_pretty(&meta).unwrap_or_default().as_bytes()).map_err(|e| e.to_string())?;
        let mut w = z.finish().map_err(|e| format!("Couldn't finish the backup ({e})"))?;
        w.flush().map_err(|e| e.to_string())?;
        w.get_ref().sync_all().map_err(|e| e.to_string())?;
        Ok(())
    })();
    if let Err(e) = r { let _ = std::fs::remove_file(&part); return Err(e); }
    std::fs::rename(&part, &file).map_err(|e| e.to_string())?;
    read_meta(&file).map(|m| backup_from_meta(&file, &m))
}

pub fn read_meta(file: &Path) -> Result<Value, String> {
    let f = std::fs::File::open(file).map_err(|e| format!("Couldn't open {} ({e})", file.display()))?;
    let mut a = zip::ZipArchive::new(f).map_err(|_| "That isn't a Hearth Saves backup (not a zip file).".to_string())?;
    let mut m = a.by_name(META).map_err(|_| "That zip isn't a Hearth Saves backup.".to_string())?;
    let mut s = String::new();
    m.read_to_string(&mut s).map_err(|e| e.to_string())?;
    serde_json::from_str(&s).map_err(|_| "That backup's description is damaged.".into())
}

/// Every backup in these destinations, newest first.
pub fn list_backups(dests: &[PathBuf]) -> Vec<Backup> {
    let mut out = Vec::new();
    for d in dests {
        let Ok(games) = std::fs::read_dir(d.join(FOLDER)) else { continue };
        for g in games.flatten() {
            let Ok(rd) = std::fs::read_dir(g.path()) else { continue };
            for e in rd.flatten() {
                let p = e.path();
                if p.extension().map(|x| x == "zip").unwrap_or(false) {
                    if let Ok(m) = read_meta(&p) { out.push(backup_from_meta(&p, &m)); }
                }
            }
        }
    }
    out.sort_by(|a, b| b.created.cmp(&a.created));
    out
}

/// Keeps the newest `keep` automatic backups of each game in a destination; older automatic ones are deleted.
/// Backups you made yourself are never deleted here.
pub fn prune(dest: &Path, keep: usize, safety: bool) -> usize {
    let mut by_game: BTreeMap<String, Vec<Backup>> = BTreeMap::new();
    for b in list_backups(&[dest.to_path_buf()]) {
        if (safety && b.safety) || (!safety && b.auto) { by_game.entry(b.id.clone()).or_default().push(b); }
    }
    let mut gone = 0;
    for (_, list) in by_game { for b in list.into_iter().skip(keep.max(1)) { if std::fs::remove_file(&b.file).is_ok() { gone += 1; } } }
    gone
}

// ------------------------------------------------------------------ restoring

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Restored { pub restored: usize, pub skipped: Vec<String>, pub safety: Option<PathBuf> }

/// Where safety copies go: ~/.local/share/nimbus/saves-safety.
pub fn safety_dir() -> PathBuf { util::data_dir().join("saves-safety") }

/// Puts a backup's files back for this game on this PC. What's there now is zipped first (the safety copy, kept
/// in safety_dir), so a restore can be undone by restoring that.
pub fn restore(zip_file: &Path, g: &GameEntry, current: &[PathBuf]) -> Result<Restored, String> {
    let meta = read_meta(zip_file)?;
    let place = g.place();
    let mut report = Restored::default();
    if !current.is_empty() {
        let sd = safety_dir();
        let b = backup(g, current, &sd, "safety").map_err(|e| format!("Couldn't make the safety copy first, so nothing was changed: {e}"))?;
        report.safety = Some(b.file);
        prune(&sd, 5, true);
    }
    let times: BTreeMap<String, u64> = meta["files"].as_array().map(|a| a.iter().filter_map(|f| Some((f["path"].as_str()?.to_string(), f["mtime"].as_u64().unwrap_or(0)))).collect()).unwrap_or_default();
    let f = std::fs::File::open(zip_file).map_err(|e| e.to_string())?;
    let mut a = zip::ZipArchive::new(f).map_err(|e| e.to_string())?;
    let mut missing: BTreeMap<String, usize> = BTreeMap::new();
    for i in 0..a.len() {
        let mut entry = a.by_index(i).map_err(|e| e.to_string())?;
        if entry.is_dir() { continue; }
        let name = entry.name().to_string();
        let Some(rest) = name.strip_prefix("files/") else { continue };
        let Some((root, rel)) = rest.split_once('/') else { continue };
        let Some(rel) = util::clean_rel(rel) else { report.skipped.push(format!("{name}: not a safe path")); continue };
        let rel = remap_steam_user(&place, root, rel);
        let Some(target) = place.resolve(root, &rel) else { *missing.entry(root.to_string()).or_default() += 1; continue };
        if let Some(dir) = target.parent() { std::fs::create_dir_all(dir).map_err(|e| format!("Couldn't make {} ({e})", dir.display()))?; }
        let tmp = target.with_file_name(format!(".{}.hearth-restore", target.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()));
        {
            let mut out = std::fs::File::create(&tmp).map_err(|e| format!("Couldn't write {} ({e})", target.display()))?;
            std::io::copy(&mut entry, &mut out).map_err(|e| { let _ = std::fs::remove_file(&tmp); format!("Couldn't write {} ({e})", target.display()) })?;
            out.sync_all().ok();
            if let Some(ms) = times.get(&name).filter(|m| **m > 0) { let _ = out.set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_millis(*ms)); }
        }
        std::fs::rename(&tmp, &target).map_err(|e| { let _ = std::fs::remove_file(&tmp); format!("Couldn't put {} in place ({e})", target.display()) })?;
        report.restored += 1;
    }
    for (root, n) in missing {
        report.skipped.push(match root.as_str() {
            "game" => format!("{n} file(s) from the game's own folder: the game isn't installed on this PC"),
            r if r.starts_with("prefix") => format!("{n} file(s) from the game's Proton/Wine folder: start the game once on this PC so that folder exists, then restore again"),
            "steam-userdata" => format!("{n} Steam Cloud file(s): Steam isn't set up on this PC"),
            _ => format!("{n} file(s) from a place this PC doesn't have ({root})"),
        });
    }
    Ok(report)
}

/// Steam keeps Steam Cloud's copies per account (userdata/<account>/<game>). From another PC with another
/// account, the files go to the only account here.
fn remap_steam_user(place: &Place, root: &str, rel: PathBuf) -> PathBuf {
    if root != "steam-userdata" { return rel; }
    let Some(ud) = place.steam_dir.as_ref().map(|s| s.join("userdata")) else { return rel };
    let mut parts = rel.components();
    let Some(first) = parts.next() else { return rel };
    if ud.join(first.as_os_str()).is_dir() { return rel; }
    let accounts: Vec<PathBuf> = std::fs::read_dir(&ud).map(|rd| rd.flatten().map(|e| e.path()).filter(|p| p.is_dir() && p.file_name().map(|n| n.to_string_lossy().chars().all(|c| c.is_ascii_digit()) && n != "0").unwrap_or(false)).collect()).unwrap_or_default();
    if accounts.len() == 1 { return PathBuf::from(accounts[0].file_name().unwrap()).join(parts.as_path()); }
    rel
}

// ------------------------------------------------------------------ settings, destinations, schedule

pub fn config_file() -> PathBuf { util::config_dir().join("saves.json") }
pub fn load_config() -> Value {
    let mut v = util::read_json(&config_file());
    if !v.is_object() { v = json!({}); }
    for (k, d) in [("schedule", json!("off")), ("keep", json!(5)), ("custom", json!({})), ("prints", json!({})), ("lastAuto", json!(0))] {
        if v.get(k).is_none() { v[k] = d; }
    }
    v
}
pub fn save_config(v: &Value) -> Result<(), String> { util::write_json(&config_file(), v) }
pub fn custom_dirs(cfg: &Value, id: &str) -> Vec<PathBuf> {
    cfg["custom"][id].as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(PathBuf::from)).collect()).unwrap_or_default()
}

/// Your Backups folder (on this PC).
pub fn default_dest() -> PathBuf { util::home().join("Backups") }

/// Where backups can go: your Backups folder, every USB drive and SD card that's plugged in, and the folder you
/// picked for automatic backups.
pub fn destinations(cfg: &Value) -> Vec<Value> {
    let mut out = vec![json!({"path": default_dest(), "label": "Backups folder on this PC", "removable": false, "free": free_space(&util::home())})];
    let user = std::env::var("USER").unwrap_or_default();
    for root in [format!("/run/media/{user}"), format!("/media/{user}"), "/var/run/media".to_string()] {
        let Ok(rd) = std::fs::read_dir(&root) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() && !out.iter().any(|o| o["path"] == json!(p)) {
                out.push(json!({"path": p, "label": e.file_name().to_string_lossy(), "removable": true, "free": free_space(&p)}));
            }
        }
    }
    if let Some(d) = cfg["dest"].as_str().filter(|d| !d.is_empty()) {
        if !out.iter().any(|o| o["path"] == json!(d)) {
            let p = PathBuf::from(d);
            let here = p.is_dir();
            out.push(json!({"path": d, "label": p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| d.to_string()), "removable": false, "missing": !here, "free": if here { free_space(&p) } else { None }}));
        }
    }
    out
}

#[cfg(target_os = "linux")]
pub fn free_space(p: &Path) -> Option<u64> {
    use std::os::unix::ffi::OsStrExt;
    let c = std::ffi::CString::new(p.as_os_str().as_bytes()).ok()?;
    let mut s: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statvfs(c.as_ptr(), &mut s) } != 0 { return None; }
    Some(s.f_bavail as u64 * s.f_frsize as u64)
}
#[cfg(not(target_os = "linux"))]
pub fn free_space(_: &Path) -> Option<u64> { None }

/// How often automatic backups run.
pub fn period_ms(schedule: &str) -> Option<u64> {
    match schedule { "daily" => Some(24 * 3600 * 1000), "weekly" => Some(7 * 24 * 3600 * 1000), _ => None }
}

/// Whether an automatic backup is due (a little early is fine: the timer looks every hour).
pub fn due(cfg: &Value, now: u64) -> bool {
    let Some(p) = period_ms(cfg["schedule"].as_str().unwrap_or("off")) else { return false };
    now.saturating_sub(cfg["lastAuto"].as_u64().unwrap_or(0)) + 30 * 60 * 1000 >= p
}

/// The scheduled backup (`hearth-saves --auto`, from the hourly timer): when it's due and the destination is
/// there, backs up every game whose saves changed since its last automatic backup and keeps the newest few.
pub fn auto_run(games: &[GameEntry], m: Option<&Manifest>, force: bool) -> Result<String, String> {
    let mut cfg = load_config();
    let now = util::now_ms();
    if !force && !due(&cfg, now) { return Ok("Not due yet.".into()); }
    let dest = cfg["dest"].as_str().filter(|d| !d.is_empty()).map(PathBuf::from).unwrap_or_else(default_dest);
    if dest != default_dest() && !dest.is_dir() { return Ok(format!("{} isn't connected; trying again later.", dest.display())); }
    let keep = cfg["keep"].as_u64().unwrap_or(5) as usize;
    let wanted: Option<Vec<String>> = cfg["games"].as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect());
    let (mut done, mut same, mut failed) = (0, 0, Vec::new());
    for g in games {
        if let Some(w) = &wanted { if !w.contains(&g.id) { continue; } }
        let s = scan(g, m, &custom_dirs(&cfg, &g.id));
        if s.files.is_empty() { continue; }
        let print = fingerprint(&s);
        if cfg["prints"][&g.id] == json!(print) { same += 1; continue; }
        match backup(g, &s.files, &dest, "auto") {
            Ok(_) => { cfg["prints"][&g.id] = json!(print); done += 1; }
            Err(e) => failed.push(format!("{}: {e}", g.title)),
        }
    }
    prune(&dest, keep, false);
    cfg["lastAuto"] = json!(now);
    cfg["lastAutoText"] = json!(format!("{done} backed up, {same} unchanged{}", if failed.is_empty() { String::new() } else { format!(", {} failed", failed.len()) }));
    save_config(&cfg)?;
    if failed.is_empty() { Ok(cfg["lastAutoText"].as_str().unwrap_or("").to_string()) } else { Err(failed.join("; ")) }
}

/// The hourly timer that runs `hearth-saves --auto`: Hearth OS has it for everyone (it does nothing until you
/// turn on a schedule); elsewhere it's made for you here.
pub const SYSTEM_TIMER: &str = "/usr/lib/systemd/user/hearth-saves-auto.timer";
#[cfg(target_os = "linux")]
pub fn ensure_timer(on: bool) -> Result<(), String> {
    if Path::new(SYSTEM_TIMER).exists() { return Ok(()); }
    let dir = util::xdg_config().join("systemd/user");
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let unit = |n: &str| dir.join(n);
    if on {
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        util::write_atomic(&unit("hearth-saves-auto.service"), format!("[Unit]\nDescription=Hearth Saves: scheduled game save backup\n\n[Service]\nType=oneshot\nExecStart={} --auto\nNice=10\nIOSchedulingClass=idle\n", exe.display()).as_bytes()).map_err(|e| e.to_string())?;
        util::write_atomic(&unit("hearth-saves-auto.timer"), b"[Unit]\nDescription=Hearth Saves: look every hour whether a scheduled backup is due\n\n[Timer]\nOnBootSec=10min\nOnUnitActiveSec=1h\nPersistent=true\n\n[Install]\nWantedBy=timers.target\n").map_err(|e| e.to_string())?;
    }
    let _ = std::process::Command::new("systemctl").args(["--user", "daemon-reload"]).status();
    let st = std::process::Command::new("systemctl").args(["--user", if on { "enable" } else { "disable" }, "--now", "hearth-saves-auto.timer"]).status();
    if on && !st.map(|s| s.success()).unwrap_or(false) { return Err("Couldn't start the backup timer (systemctl --user).".into()); }
    Ok(())
}
#[cfg(not(target_os = "linux"))]
pub fn ensure_timer(_: bool) -> Result<(), String> { Ok(()) }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{parse_yaml, Manifest};

    fn put(p: &Path, text: &str) { std::fs::create_dir_all(p.parent().unwrap()).unwrap(); std::fs::write(p, text).unwrap(); }

    fn hk(t: &Path) -> GameEntry {
        let steam = t.join(".local/share/Steam");
        GameEntry { id: "steam:367520".into(), title: "Hollow Knight".into(), source: "steam".into(), store: "steam".into(),
            install: Some(steam.join("steamapps/common/Hollow Knight")), prefix: Some(steam.join("steamapps/compatdata/367520/pfx")),
            steam_app: Some(367520), library_root: Some(steam.clone()), steam_dir: Some(steam), kind: "game".into(), ..Default::default() }
    }

    #[test]
    fn backs_up_and_restores_with_a_safety_copy() {
        let t = tempfile::tempdir().unwrap();
        // The functions read $HOME/$XDG_*: this test owns them (cargo runs it in its own process with --test-threads=1 in CI).
        std::env::set_var("HOME", t.path());
        std::env::set_var("XDG_DATA_HOME", t.path().join(".local/share"));
        std::env::set_var("XDG_CONFIG_HOME", t.path().join(".config"));
        let g = hk(t.path());
        let save = g.prefix.clone().unwrap().join("drive_c/users/steamuser/AppData/LocalLow/Team Cherry/Hollow Knight/user1.dat");
        put(&save, "progress 1");
        put(&g.install.clone().unwrap().join("hollow_knight_Data/Config.ini"), "cfg");
        let m = Manifest::from_games(parse_yaml(crate::manifest::tests::SAMPLE).unwrap());
        let s = scan(&g, Some(&m), &[]);
        assert_eq!((s.count, s.known_as.as_deref()), (2, Some("Hollow Knight")));
        let usb = t.path().join("usb");
        std::fs::create_dir_all(&usb).unwrap();
        let b = backup(&g, &s.files, &usb, "").unwrap();
        assert_eq!((b.count, b.title.as_str(), b.auto), (2, "Hollow Knight", false));
        assert!(b.file.starts_with(usb.join("Hearth Saves/Hollow Knight")));
        assert_eq!(list_backups(&[usb.clone()]).len(), 1);
        // Play on: the save changes. Restoring puts the old one back and keeps the new one in a safety copy.
        put(&save, "progress 2");
        let now = scan(&g, Some(&m), &[]);
        let r = restore(&b.file, &g, &now.files).unwrap();
        assert_eq!(r.restored, 2);
        assert!(r.skipped.is_empty(), "{:?}", r.skipped);
        assert_eq!(std::fs::read_to_string(&save).unwrap(), "progress 1");
        let safety = r.safety.unwrap();
        assert!(safety.starts_with(safety_dir()));
        assert!(read_meta(&safety).unwrap()["safety"] == true);
        // "Another PC": the game's prefix doesn't exist yet: the prefix files are skipped with a reason.
        let mut other = g.clone();
        other.prefix = None;
        let r2 = restore(&b.file, &other, &[]).unwrap();
        assert_eq!(r2.restored, 1);
        assert!(r2.skipped[0].contains("start the game once"), "{:?}", r2.skipped);
    }

    #[test]
    fn schedules() {
        let mut c = json!({"schedule": "daily", "lastAuto": 0});
        assert!(due(&c, 100_000_000));
        c["lastAuto"] = json!(100_000_000u64);
        assert!(!due(&c, 100_000_000 + 3600 * 1000));
        assert!(due(&c, 100_000_000 + 24 * 3600 * 1000));
        c["schedule"] = json!("off");
        assert!(!due(&c, u64::MAX / 2));
    }
}
