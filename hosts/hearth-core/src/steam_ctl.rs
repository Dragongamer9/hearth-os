//! Steam run by Hearth, hidden, with the real Steam client doing all the work (docs/STEAM.md):
//! - Installs without Steam's dialog: Hearth writes the small install record Steam itself writes when you press
//!   Install (steamapps/appmanifest_<id>.acf with StateFlags "1026": update required, update started) into the
//!   library you chose, then lets Steam read it (Steam reads its libraries when it starts, so a Steam that's already
//!   running and idle is closed properly and started again, hidden). If Steam doesn't take the install up within two
//!   minutes, Hearth removes its record again and falls back to Steam's own install window, brought in front.
//! - Download progress from the same records (StateFlags, BytesDownloaded, BytesToDownload) and Steam's
//!   logs/content_log.txt (download rate, what it's doing, why it stopped).
//! - Uninstalls without a dialog: only when Steam isn't working on the game and the game isn't running; Steam is
//!   closed properly first (it keeps its own list of installed games), then the game's folder is removed, only
//!   inside the Steam library it's in.
//! - "Start Steam only when needed": Steam starts hidden when you play, install or use its features, and is closed
//!   properly (steam -shutdown, never killed) once nothing has needed it for a while, never during a download, an
//!   update, a cloud sync or a game.
//! - Signing in: Steam's own sign-in window in front; Hearth only watches Steam's files to see when it's done.
//!   Hearth never signs in itself, never sees or keeps a password or token.

use crate::vdf::Kv;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

// Steam's StateFlags bits (EAppState).
pub const UPDATE_REQUIRED: i64 = 2;
pub const FULLY_INSTALLED: i64 = 4;
pub const APP_RUNNING: i64 = 64;
pub const UPDATE_RUNNING: i64 = 256;
pub const UPDATE_PAUSED: i64 = 512;
pub const UPDATE_STARTED: i64 = 1024;
pub const UNINSTALLING: i64 = 2048;
pub const BACKUP_RUNNING: i64 = 4096;
pub const RECONFIGURING: i64 = 65536;
pub const VALIDATING: i64 = 131072;
pub const ADDING_FILES: i64 = 262144;
pub const PREALLOCATING: i64 = 524288;
pub const DOWNLOADING: i64 = 1048576;
pub const STAGING: i64 = 2097152;
pub const COMMITTING: i64 = 4194304;
pub const UPDATE_STOPPING: i64 = 8388608;
/// What Steam writes for an install it has queued: update required + update started.
pub const NEW_INSTALL: i64 = UPDATE_REQUIRED | UPDATE_STARTED;
/// Steam is writing the game's files (or running it) right now.
const WRITING: i64 = APP_RUNNING | UPDATE_RUNNING | UNINSTALLING | BACKUP_RUNNING | RECONFIGURING | VALIDATING | ADDING_FILES | PREALLOCATING | DOWNLOADING | STAGING | COMMITTING | UPDATE_STOPPING;

/// Steam is (or will be, as soon as it can) working on this app: a download, update or check that isn't paused.
pub fn steam_working(flags: i64) -> bool { flags & UPDATE_PAUSED == 0 && flags & (WRITING | UPDATE_STARTED) != 0 }
/// Steam is touching the game's files right now: they must be left alone.
pub fn writing(flags: i64) -> bool { flags & WRITING != 0 }

/// How long Steam may sit unused before Hearth closes it (when it started it, with "only when needed" on).
pub const IDLE_CLOSE_MS: u64 = 10 * 60 * 1000;
/// How long Steam gets to take up an install record before Hearth falls back to Steam's own install window.
pub const PICKUP_MS: u64 = 120_000;

fn now() -> u64 { crate::library::now_ms() }
fn log(t: &str) { crate::bridge::log(&format!("steam: {t}")); }

// ------------------------------------------------------------------ what Hearth knows about the client

#[derive(Debug, Clone)]
pub struct Pending { pub app_id: u32, pub title: String, pub manifest: PathBuf, pub text: String, pub written: u64, pub kicked: u64 }

#[derive(Default)]
struct Ctl {
    started_by_hearth: bool,
    last_use: u64,
    /// When Hearth last saw Steam come up (0: not running).
    up_since: u64,
    /// Hearth is starting or closing Steam right now.
    busy: bool,
    pending: Vec<Pending>,
    last_pending_check: u64,
    last_idle_check: u64,
    /// A sign-in Hearth is waiting for: what Steam's files said when it began.
    signin: Option<(u64, SignState)>,
}
fn ctl() -> std::sync::MutexGuard<'static, Ctl> {
    static C: std::sync::OnceLock<Mutex<Ctl>> = std::sync::OnceLock::new();
    C.get_or_init(Default::default).lock().unwrap_or_else(|e| e.into_inner())
}

/// Something needed Steam just now (a game, an install, friends): it stays open for a while.
pub fn touch() { ctl().last_use = now(); }
/// Hearth started Steam itself (so Hearth may close it again).
pub fn started_by_hearth() { let mut c = ctl(); c.started_by_hearth = true; c.last_use = now(); }
pub fn pending_ids() -> Vec<u32> { ctl().pending.iter().map(|p| p.app_id).collect() }

/// Runs `f` on its own thread unless Hearth is already starting or closing Steam.
fn with_client(f: impl FnOnce() + Send + 'static) {
    { let mut c = ctl(); if c.busy { return; } c.busy = true; }
    std::thread::spawn(move || { f(); ctl().busy = false; });
}

/// Closes Steam properly (steam -shutdown; Steam finishes its own cloud sync) and waits up to 45 s. Never kills it.
pub fn shutdown(home: &Path) -> Result<(), String> {
    if !crate::steam::is_running() { return Ok(()); }
    log("closing it (-shutdown)");
    crate::steam::spawn(home, "-shutdown")?;
    for _ in 0..90 { if !crate::steam::is_running() { break; } std::thread::sleep(std::time::Duration::from_millis(500)); }
    if crate::steam::is_running() { return Err("Steam didn't close. It may be finishing something; try again in a minute.".into()); }
    let mut c = ctl(); c.started_by_hearth = false; c.up_since = 0;
    Ok(())
}

/// Closes Steam properly and starts it again hidden, so it reads its libraries again.
pub fn restart_hidden(home: &Path) -> Result<(), String> {
    shutdown(home)?;
    std::thread::sleep(std::time::Duration::from_secs(2));
    crate::steam::ensure_running_silent(home);
    if crate::steam::is_running() { Ok(()) } else { Err("Steam didn't start again.".into()) }
}

/// Starts Steam hidden (in the background) if it isn't running, then opens a steam:// address in it. Blocks
/// while Steam starts: call it off the UI thread.
pub fn open_url(home: &Path, url: &str) -> Result<(), String> {
    crate::steam::ensure_running_silent(home);
    touch();
    crate::steam::spawn(home, url)
}

// ------------------------------------------------------------------ install records (appmanifest_<id>.acf)

fn quote(s: &str) -> String { s.replace('\\', "\\\\").replace('"', "\\\"") }

/// A new install record in Steam's own layout, as Steam writes it when an install is queued.
pub fn manifest_text(app_id: u32, name: &str, installdir: &str, owner: u64) -> String {
    format!("\"AppState\"\n{{\n\t\"appid\"\t\t\"{app_id}\"\n\t\"Universe\"\t\t\"1\"\n\t\"name\"\t\t\"{}\"\n\t\"StateFlags\"\t\t\"{NEW_INSTALL}\"\n\t\"installdir\"\t\t\"{}\"\n\
\t\"LastUpdated\"\t\t\"0\"\n\t\"SizeOnDisk\"\t\t\"0\"\n\t\"StagingSize\"\t\t\"0\"\n\t\"buildid\"\t\t\"0\"\n\t\"LastOwner\"\t\t\"{owner}\"\n\t\"UpdateResult\"\t\t\"0\"\n\
\t\"BytesToDownload\"\t\t\"0\"\n\t\"BytesDownloaded\"\t\t\"0\"\n\t\"BytesToStage\"\t\t\"0\"\n\t\"BytesStaged\"\t\t\"0\"\n\t\"TargetBuildID\"\t\t\"0\"\n\
\t\"AutoUpdateBehavior\"\t\t\"0\"\n\t\"AllowOtherDownloadsWhileRunning\"\t\t\"0\"\n\t\"ScheduledAutoUpdate\"\t\t\"0\"\n\t\"InstalledDepots\"\n\t{{\n\t}}\n\
\t\"UserConfig\"\n\t{{\n\t}}\n\t\"MountedConfig\"\n\t{{\n\t}}\n}}\n", quote(name), quote(installdir))
}

/// A game folder name Steam would accept, from the title (when appinfo.vdf doesn't say): no path characters.
pub fn folder_name(title: &str, app_id: u32) -> String {
    let s: String = title.chars().filter(|c| !matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') && !c.is_control()).collect();
    let s = s.trim().trim_matches('.').trim().to_string();
    if s.is_empty() { format!("app_{app_id}") } else { s }
}

/// A game folder name from a record that's safe to delete inside steamapps/common: one plain path part.
pub fn safe_dir_name(d: &str) -> bool {
    !d.is_empty() && d != "." && d != ".." && !d.contains('/') && !d.contains('\\') && !d.contains('\0') && d.trim() == d
}

/// The record for an app in any library: (library, record file, its contents).
pub fn find_manifest(steam: &Path, app_id: u32) -> Option<(PathBuf, PathBuf, Kv)> {
    for lib in crate::steam::libraries(steam) {
        let f = lib.join("steamapps").join(format!("appmanifest_{app_id}.acf"));
        if let Some(kv) = crate::vdf::load(&f) { return Some((lib, f, kv)); }
    }
    None
}

/// Free space on the drive a folder is on (df), in bytes.
pub fn free_bytes(p: &Path) -> Option<u64> {
    let out = crate::system::run_timeout("df", &["-B1", "--output=avail", &p.to_string_lossy()], std::time::Duration::from_secs(5)).ok()?;
    out.lines().nth(1)?.trim().parse().ok()
}

/// Steam's libraries for the Stores page: [{path, free, main, chosen}]
pub fn libraries_dto(steam: &Path, chosen: &Path) -> Value {
    json!(crate::steam::libraries(steam).iter().enumerate().map(|(i, l)| json!({"path": l.to_string_lossy(), "free": free_bytes(l), "main": i == 0, "chosen": l == chosen})).collect::<Vec<_>>())
}

/// Where Steam games go: the library you chose (if it's still a Steam library), else one on the drive of
/// Hearth's install folder (the drive you picked for games), else Steam's own folder.
pub fn choose_library(libs: &[PathBuf], chosen: Option<&Path>, install_root: &Path) -> Option<PathBuf> {
    if let Some(c) = chosen { if let Some(l) = libs.iter().find(|l| l.as_path() == c) { return Some(l.clone()); } }
    use std::os::unix::fs::MetadataExt;
    let root_dev = install_root.ancestors().find_map(|a| std::fs::metadata(a).ok()).map(|m| m.dev());
    if let Some(dev) = root_dev {
        if let Some(l) = libs.iter().skip(1).find(|l| std::fs::metadata(l).map(|m| m.dev() == dev).unwrap_or(false)) { return Some(l.clone()); }
    }
    libs.first().cloned()
}

/// Starts a Steam install without Steam's dialog: writes the record, then makes Steam read it (starting it, or
/// restarting it when it's idle; when it's busy, once it's done). Returns {silent, waiting}.
pub fn install(home: &Path, app_id: u32, title: &str, library: &Path) -> Result<Value, String> {
    let steam = crate::steam::steam_path(home).ok_or("Steam isn't set up yet. Open Steam once, then try again.")?;
    if let Some((_, _, kv)) = find_manifest(&steam, app_id) {
        let flags = kv.num(&["AppState", "StateFlags"]);
        if flags & FULLY_INSTALLED != 0 { return Err(format!("{title} is already installed.")); }
        return Ok(json!({"silent": true, "waiting": false, "already": true}));
    }
    let owner = crate::steam_owned::current_id64(&steam).ok_or("Sign in to Steam first (Library › Stores).")?;
    if !crate::steam::libraries(&steam).iter().any(|l| l == library) { return Err("That folder isn't a Steam library anymore. Pick another one in Library › Stores.".into()); }
    let wanted = [app_id].into_iter().collect();
    let info = crate::steam_owned::appinfo(&steam.join("appcache/appinfo.vdf"), &wanted).remove(&app_id).unwrap_or_default();
    let name = if info.name.is_empty() { title.to_string() } else { info.name.clone() };
    let dir = if safe_dir_name(&info.installdir) { info.installdir.clone() } else { folder_name(&name, app_id) };
    let apps = library.join("steamapps");
    std::fs::create_dir_all(apps.join("common")).map_err(|e| format!("Couldn't use the Steam library at {} ({e}).", library.display()))?;
    let file = apps.join(format!("appmanifest_{app_id}.acf"));
    let text = manifest_text(app_id, &name, &dir, owner);
    crate::paths::write_atomic(&file, text.as_bytes()).map_err(|e| format!("Couldn't write to the Steam library ({e})."))?;
    log(&format!("install {app_id} ({name}): record written to {}", file.display()));
    ctl().pending.push(Pending { app_id, title: name, manifest: file, text, written: now(), kicked: 0 });
    touch();
    let waiting = kick(home);
    Ok(json!({"silent": true, "waiting": waiting}))
}

/// Gets Steam to read new install records: starts it if it isn't running, restarts it if it's idle. Returns true
/// when Steam is busy (a download, a game) and the records wait until it's done.
fn kick(home: &Path) -> bool {
    let running = crate::steam::is_running();
    if running && !idle_for_restart(home) { return true; }
    for p in ctl().pending.iter_mut() { if p.kicked == 0 { p.kicked = now(); } }
    let home = home.to_path_buf();
    with_client(move || {
        let r = if crate::steam::is_running() { restart_hidden(&home) } else { crate::steam::ensure_running_silent(&home); Ok(()) };
        if let Err(e) = r { log(&format!("couldn't restart for an install: {e}")); }
        let mut c = ctl(); let t = now(); for p in c.pending.iter_mut() { p.kicked = t; }
    });
    false
}

/// Nothing is using Steam right now: no Steam game, no download, update or check, no cloud sync.
fn idle_for_restart(home: &Path) -> bool {
    let Some(steam) = crate::steam::steam_path(home) else { return true };
    !steam_game_running(None) && !any_working(&steam, &pending_ids()) && !recently_written(&steam.join("logs/cloud_log.txt"), 120_000)
}

/// A game Steam started is running (any, or this app): Steam's reaper process carries its AppId.
pub fn steam_game_running(app_id: Option<u32>) -> bool {
    crate::procs::snapshot(Path::new("/proc")).iter().any(|p| match (p.reaper_app, app_id) { (Some(_), None) => true, (Some(a), Some(b)) => a == b, _ => false })
}

/// Any record Steam is working on (downloads, updates, checks), leaving out the install records Hearth just
/// wrote and Steam hasn't taken up yet.
fn any_working(steam: &Path, except: &[u32]) -> bool {
    for lib in crate::steam::libraries(steam) {
        let Ok(rd) = std::fs::read_dir(lib.join("steamapps")) else { continue };
        for e in rd.flatten() {
            let n = e.file_name().to_string_lossy().to_string();
            let Some(id) = n.strip_prefix("appmanifest_").and_then(|r| r.strip_suffix(".acf")).and_then(|r| r.parse::<u32>().ok()) else { continue };
            if except.contains(&id) { continue; }
            if let Some(kv) = crate::vdf::load(&e.path()) { if steam_working(kv.num(&["AppState", "StateFlags"])) { return true; } }
        }
    }
    false
}

fn recently_written(f: &Path, ms: u64) -> bool {
    std::fs::metadata(f).and_then(|m| m.modified()).ok().and_then(|t| t.elapsed().ok()).map(|d| (d.as_millis() as u64) < ms).unwrap_or(false)
}

/// What became of an install record Hearth wrote.
#[derive(Debug, PartialEq)]
pub enum Pickup { Waiting, Taken, Gone, TimedOut }

/// `text_now`: the record as it is now (None: gone). `kicked`: when Steam was started or restarted to read it
/// (0: not yet). Steam rewrites the record as soon as it takes the install up.
pub fn pickup(written: &str, text_now: Option<&str>, steam_up: bool, kicked: u64, now: u64) -> Pickup {
    match text_now {
        None if kicked > 0 && steam_up => Pickup::Gone,
        None => Pickup::Waiting,
        Some(t) if t != written => Pickup::Taken,
        Some(_) if kicked > 0 && steam_up && now.saturating_sub(kicked) > PICKUP_MS => Pickup::TimedOut,
        Some(_) => Pickup::Waiting,
    }
}

/// Removes an install record Hearth wrote that Steam hasn't taken up (cancelling it). False when Steam has it.
pub fn cancel_pending(app_id: u32) -> bool {
    let mut c = ctl();
    let Some(i) = c.pending.iter().position(|p| p.app_id == app_id) else { return false };
    let p = c.pending.remove(i);
    if std::fs::read_to_string(&p.manifest).ok().as_deref() == Some(p.text.as_str()) { let _ = std::fs::remove_file(&p.manifest); log(&format!("install {app_id}: cancelled before Steam took it up")); true } else { false }
}

// ------------------------------------------------------------------ Steam's content_log.txt

#[derive(Debug, Default, PartialEq)]
pub struct ContentLog {
    /// The last "Current download rate" in bytes a second.
    pub rate: Option<u64>,
    /// Per app: (what Steam is doing, why it stopped or waits), from its last "update changed"/"state changed" lines.
    pub apps: std::collections::HashMap<u32, (String, Option<String>)>,
}

/// Reads the end of content_log.txt: "[date] Current download rate: 64.821 Mbps",
/// "[date] AppID 620 update changed : Running Update,Downloading,", "[date] AppID 620 state changed : Update
/// Required,Update Queued, (Manifest unavailable)".
pub fn parse_content_log(text: &str) -> ContentLog {
    let mut out = ContentLog::default();
    for line in text.lines() {
        let body = line.split_once("] ").map(|x| x.1).unwrap_or(line).trim();
        if let Some(r) = body.strip_prefix("Current download rate:") {
            let mut it = r.split_whitespace();
            if let Some(n) = it.next().and_then(|n| n.parse::<f64>().ok()) {
                let mult = match it.next().unwrap_or("Mbps") { "Kbps" => 1e3, "Gbps" => 1e9, _ => 1e6 };
                out.rate = Some((n * mult / 8.0).round() as u64);
            }
            continue;
        }
        let Some(rest) = body.strip_prefix("AppID ") else { continue };
        let Some((id, what)) = rest.split_once(' ') else { continue };
        let Ok(id) = id.parse::<u32>() else { continue };
        let (kind, val) = match what.split_once(" : ") { Some(x) => x, None => continue };
        let reason = val.rfind('(').and_then(|i| val[i + 1..].strip_suffix(')').map(|r| r.trim().to_string())).filter(|r| !r.is_empty());
        let words: Vec<&str> = val.split('(').next().unwrap_or("").split(',').map(str::trim).filter(|w| !w.is_empty()).collect();
        match kind.trim() {
            "update changed" => {
                let stage = stage_of(&words);
                let e = out.apps.entry(id).or_insert((String::new(), None));
                e.0 = stage.to_string();
            }
            "state changed" => {
                let e = out.apps.entry(id).or_insert((String::new(), None));
                // A reason that isn't a problem (a short delay, a schedule) doesn't stop anything.
                e.1 = reason.filter(|r| !r.to_lowercase().starts_with("update delayed") && !r.to_lowercase().contains("scheduled"));
                if words.contains(&"Fully Installed") && !words.iter().any(|w| w.starts_with("Update")) { e.0 = String::new(); }
            }
            _ => {}
        }
    }
    out
}

/// A short word for what Steam's "update changed" says it's doing.
fn stage_of(words: &[&str]) -> &'static str {
    let has = |w: &str| words.iter().any(|x| x.eq_ignore_ascii_case(w));
    if has("None") { "" }
    else if has("Validating") || has("Verifying") { "Verifying" }
    else if has("Committing") || has("Staging") { "Installing" }
    else if has("Preallocating") { "Making room" }
    else if has("Downloading") { "Downloading" }
    else if has("Reconfiguring") { "Getting ready" }
    else if has("Stopping") { "Stopping" }
    else { "" }
}

/// The end of Steam's content log (the last 64 KB), and whether it was written in the last 30 seconds.
pub fn content_log(steam: &Path) -> (ContentLog, bool) {
    let f = steam.join("logs/content_log.txt");
    let fresh = recently_written(&f, 30_000);
    let Ok(mut file) = std::fs::File::open(&f) else { return (ContentLog::default(), false) };
    use std::io::{Read, Seek, SeekFrom};
    let len = file.metadata().map(|m| m.len()).unwrap_or(0);
    let _ = file.seek(SeekFrom::Start(len.saturating_sub(65536)));
    let mut buf = Vec::new();
    let _ = file.read_to_end(&mut buf);
    (parse_content_log(&String::from_utf8_lossy(&buf)), fresh)
}

// ------------------------------------------------------------------ uninstalling

/// Removes a Steam game without Steam's dialog. Refuses while Steam works on it or it runs; closes an idle Steam
/// first (Steam keeps its own list of installed games and would write the record back). Only ever deletes inside
/// the Steam library the game is in. Returns whether Steam was running (and is now closed). Blocks: off the UI thread.
pub fn uninstall(home: &Path, app_id: u32, title: &str) -> Result<bool, String> {
    let steam = crate::steam::steam_path(home).ok_or("Steam isn't set up on this device.")?;
    let (_, _, kv) = find_manifest(&steam, app_id).ok_or(format!("Steam doesn't have {title} installed anymore."))?;
    if writing(kv.num(&["AppState", "StateFlags"])) { return Err(format!("Steam is working on {title} (downloading, updating or checking it). Try again when it's done.")); }
    if steam_game_running(Some(app_id)) { return Err(format!("Exit {title} first.")); }
    let was_running = crate::steam::is_running();
    if was_running {
        if steam_game_running(None) { return Err("Exit the Steam game that's running first: Steam has to close to delete a game.".into()); }
        if any_working(&steam, &[app_id]) { return Err("Steam is downloading or updating another game. Try again when it's done.".into()); }
        { let mut c = ctl(); if c.busy { return Err("Steam is starting or closing. Try again in a moment.".into()); } c.busy = true; }
        let r = shutdown(home);
        ctl().busy = false;
        r?;
    }
    // Read again: Steam writes its records as it closes.
    let (lib, file, kv) = find_manifest(&steam, app_id).ok_or(format!("Steam doesn't have {title} installed anymore."))?;
    if writing(kv.num(&["AppState", "StateFlags"])) { return Err(format!("Steam is working on {title}. Try again when it's done.")); }
    let dir = kv.str(&["AppState", "installdir"]).unwrap_or("").to_string();
    remove_files(&lib, &file, app_id, &dir)?;
    log(&format!("uninstalled {app_id} ({title}) from {}", lib.display()));
    Ok(was_running)
}

/// Deletes a game's files inside one Steam library: its folder in steamapps/common (a link there is removed,
/// never followed), its shader cache, unfinished download and Workshop items, and its record (set aside first,
/// so a half-finished removal never looks installed). Proton prefixes (compatdata, with saves) stay.
pub fn remove_files(lib: &Path, manifest: &Path, app_id: u32, installdir: &str) -> Result<(), String> {
    let apps = lib.join("steamapps");
    if manifest.parent() != Some(apps.as_path()) { return Err("That record isn't in a Steam library.".into()); }
    let aside = manifest.with_extension("acf.hearth-removing");
    std::fs::rename(manifest, &aside).map_err(|e| format!("Couldn't change Steam's record ({e})."))?;
    let mut targets = vec![apps.join("shadercache").join(app_id.to_string()), apps.join("downloading").join(app_id.to_string()), apps.join("temp").join(app_id.to_string()),
                           apps.join("workshop/content").join(app_id.to_string()), apps.join("workshop").join(format!("appworkshop_{app_id}.acf"))];
    if safe_dir_name(installdir) { targets.insert(0, apps.join("common").join(installdir)); }
    let common = std::fs::canonicalize(apps.join("common")).ok();
    for t in targets {
        let Ok(meta) = std::fs::symlink_metadata(&t) else { continue };
        // Never outside the library: the folder's real parent must be the library's steamapps folder (or common).
        let parent = t.parent().and_then(|p| std::fs::canonicalize(p).ok());
        let real_apps = std::fs::canonicalize(&apps).ok();
        let inside = parent.as_ref().map(|p| Some(p) == common.as_ref() || real_apps.as_ref().map(|a| p.starts_with(a)).unwrap_or(false)).unwrap_or(false);
        if !inside { log(&format!("left {} alone: not inside the library", t.display())); continue; }
        let r = if meta.file_type().is_symlink() || meta.is_file() { std::fs::remove_file(&t) } else { std::fs::remove_dir_all(&t) };
        if let Err(e) = r {
            let _ = std::fs::rename(&aside, manifest);
            return Err(format!("Couldn't delete {} ({e}).", t.display()));
        }
    }
    let _ = std::fs::remove_file(&aside);
    Ok(())
}

// ------------------------------------------------------------------ closing Steam when nothing needs it

#[derive(Debug, Default, Clone)]
pub struct IdleCheck {
    pub on_demand: bool, pub running: bool, pub started_by_hearth: bool, pub busy: bool,
    pub game_running: bool, pub working: bool, pub content_recent: bool, pub cloud_recent: bool,
    pub pending: bool, pub since_use: u64, pub since_up: u64,
}

/// Close Steam now? Only one Hearth started, with "only when needed" on, when nothing has used it for a while and
/// it isn't downloading, updating, syncing or running a game.
pub fn should_close(i: &IdleCheck) -> bool {
    i.on_demand && i.running && i.started_by_hearth && !i.busy && !i.game_running && !i.working && !i.content_recent && !i.cloud_recent && !i.pending
        && i.since_use >= IDLE_CLOSE_MS && i.since_up >= IDLE_CLOSE_MS
}

/// Every couple of seconds (Library::tick): install records Steam should take up, Steam's start time, and closing
/// an unneeded Steam. Returns events for the page (and the host: "steam.window" brings Steam's window in front).
pub fn tick(home: &Path, on_demand: bool, hearth_steam_game: bool) -> Vec<Value> {
    let mut events = Vec::new();
    let t = now();
    let (check_pending, check_idle) = {
        let c = ctl();
        (!c.pending.is_empty() && t.saturating_sub(c.last_pending_check) >= 4000, t.saturating_sub(c.last_idle_check) >= 30_000)
    };
    if !check_pending && !check_idle { return events; }
    let running = crate::steam::is_running();
    {
        let mut c = ctl();
        if running && c.up_since == 0 { c.up_since = t; }
        // Closed some other way: no longer Hearth's to close (unless Hearth is starting it right now).
        if !running { c.up_since = 0; if !c.busy && t.saturating_sub(c.last_use) > 60_000 { c.started_by_hearth = false; } }
    }
    if check_pending {
        ctl().last_pending_check = t;
        let list = ctl().pending.clone();
        let mut need_kick = false;
        for p in list {
            let now_text = std::fs::read_to_string(&p.manifest).ok();
            let up = running && !ctl().busy;
            match pickup(&p.text, now_text.as_deref(), up, p.kicked, t) {
                Pickup::Waiting => { if p.kicked == 0 { need_kick = true; } }
                Pickup::Taken => {
                    log(&format!("install {}: Steam took it up", p.app_id));
                    ctl().pending.retain(|x| x.app_id != p.app_id);
                    events.push(json!({"type": "event", "name": "library.changed", "data": {}}));
                }
                Pickup::Gone | Pickup::TimedOut => {
                    let gone = now_text.is_none();
                    if !gone { let _ = std::fs::remove_file(&p.manifest); }
                    ctl().pending.retain(|x| x.app_id != p.app_id);
                    log(&format!("install {}: Steam {} the record; opening Steam's own install window", p.app_id, if gone { "removed" } else { "didn't take up" }));
                    let _ = crate::steam::spawn(home, &format!("steam://install/{}", p.app_id));
                    events.push(json!({"type": "event", "name": "steam.window", "data": {"why": "install", "id": format!("steam:{}", p.app_id),
                        "text": format!("Steam didn't start {} by itself: confirm the install in Steam's window.", p.title)}}));
                    events.push(json!({"type": "event", "name": "library.changed", "data": {}}));
                }
            }
        }
        let busy = ctl().busy;
        if need_kick && !busy { kick(home); }
    }
    if check_idle {
        ctl().last_idle_check = t;
        // Steam's own window in front (its chat, its store): it's being used.
        if running && crate::gamescope::inside() {
            if let Some(f) = crate::gamescope::focused() {
                if crate::gamescope::mapped_windows().iter().any(|w| w.win == f && crate::procs::is_steam_client(crate::procs::comm(Path::new("/proc"), w.pid as u32).trim())) { touch(); }
            }
        }
        if running && on_demand {
            if let Some(steam) = crate::steam::steam_path(home) {
                let c = ctl();
                let i = IdleCheck { on_demand, running, started_by_hearth: c.started_by_hearth, busy: c.busy, pending: !c.pending.is_empty() || c.signin.is_some(),
                    since_use: t.saturating_sub(c.last_use), since_up: t.saturating_sub(c.up_since), ..Default::default() };
                drop(c);
                let quick = should_close(&IdleCheck { game_running: false, working: false, content_recent: false, cloud_recent: false, ..i.clone() });
                if quick {
                    let full = IdleCheck { game_running: hearth_steam_game || steam_game_running(None), working: any_working(&steam, &[]),
                        content_recent: recently_written(&steam.join("logs/content_log.txt"), 90_000), cloud_recent: recently_written(&steam.join("logs/cloud_log.txt"), 120_000), ..i };
                    if should_close(&full) {
                        log("nothing has needed it for a while: closing it");
                        let home = home.to_path_buf();
                        with_client(move || { if let Err(e) = shutdown(&home) { log(&format!("couldn't close it: {e}")); } });
                    }
                }
            }
        }
    }
    events
}

// ------------------------------------------------------------------ signing in (Steam's own window)

/// What Steam's files say about who's signed in: the most recent account in loginusers.vdf, when that file last
/// changed, and registry.vdf's AutoLoginUser.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SignState { pub recent: Option<u64>, pub users_changed: u64, pub auto: Option<String> }

pub fn sign_state(home: &Path) -> SignState {
    let Some(steam) = crate::steam::steam_path(home) else { return SignState::default() };
    let users = steam.join("config/loginusers.vdf");
    let changed = std::fs::metadata(&users).and_then(|m| m.modified()).ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_millis() as u64).unwrap_or(0);
    let reg = [crate::paths::home().join(".steam/registry.vdf"), home.join(".var/app/com.valvesoftware.Steam/.steam/registry.vdf")]
        .into_iter().find_map(|f| std::fs::read_to_string(f).ok()).unwrap_or_default();
    let auto = crate::vdf::parse(&reg).str(&["Registry", "HKCU", "Software", "Valve", "Steam", "AutoLoginUser"]).map(str::to_string).filter(|s| !s.is_empty());
    SignState { recent: crate::steam_owned::current_id64(&steam), users_changed: changed, auto }
}

/// Signed in since `before`: an account is marked most recent and Steam wrote its account list (or switched the
/// account it signs in as) after the sign-in began.
pub fn signed_in_since(before: &SignState, now: &SignState) -> bool {
    now.recent.is_some() && (now.users_changed > before.users_changed || now.recent != before.recent || (now.auto.is_some() && now.auto != before.auto))
}

/// Starts waiting for a sign-in in Steam's window (up to 15 minutes); the host hears {what:'steamSignedIn'} when
/// it's done. Opening the window is the caller's (open_url + the host bringing it in front).
pub fn watch_sign_in(home: &Path) {
    let start = sign_state(home);
    let t = now();
    ctl().signin = Some((t, start.clone()));
    touch();
    let home = home.to_path_buf();
    std::thread::spawn(move || {
        while now().saturating_sub(t) < 15 * 60 * 1000 {
            std::thread::sleep(std::time::Duration::from_secs(2));
            if ctl().signin.as_ref().map(|s| s.0) != Some(t) { return; }   // cancelled or started again
            let s = sign_state(&home);
            if signed_in_since(&start, &s) {
                ctl().signin = None;
                log("signed in (Steam wrote its account list)");
                crate::stores::notify(json!({"type": "internal", "what": "steamSignedIn"}));
                return;
            }
        }
        let mut c = ctl();
        if c.signin.as_ref().map(|s| s.0) == Some(t) { c.signin = None; }
    });
}
pub fn cancel_sign_in() { ctl().signin = None; }
pub fn signing_in() -> bool { ctl().signin.is_some() }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_flags() {
        assert!(steam_working(NEW_INSTALL), "a queued install");
        assert!(!steam_working(NEW_INSTALL | UPDATE_PAUSED), "a paused one");
        assert!(steam_working(FULLY_INSTALLED | UPDATE_RUNNING | DOWNLOADING));
        assert!(!steam_working(FULLY_INSTALLED) && !steam_working(FULLY_INSTALLED | UPDATE_REQUIRED));
        assert!(!writing(FULLY_INSTALLED) && !writing(NEW_INSTALL | UPDATE_PAUSED));
        assert!(writing(FULLY_INSTALLED | APP_RUNNING) && writing(4 | VALIDATING) && writing(1026 | DOWNLOADING));
        assert_eq!(crate::steam::app_state(NEW_INSTALL), (false, "downloading"));
        assert_eq!(crate::steam::app_state(NEW_INSTALL | UPDATE_PAUSED), (false, "paused"));
        assert_eq!(crate::steam::app_state(FULLY_INSTALLED), (true, "installed"));
        assert_eq!(crate::steam::app_state(FULLY_INSTALLED | UPDATE_RUNNING), (true, "updating"));
        assert_eq!(crate::steam::app_state(UPDATE_REQUIRED), (false, "queued"));
    }

    #[test]
    fn install_records() {
        let t = manifest_text(620, "Portal \"2\"", "Portal 2", 76561197960265729);
        let kv = crate::vdf::parse(&t);
        assert_eq!(kv.str(&["AppState", "appid"]), Some("620"));
        assert_eq!(kv.str(&["AppState", "name"]), Some("Portal \"2\""));
        assert_eq!(kv.num(&["AppState", "StateFlags"]), 1026);
        assert_eq!(kv.str(&["AppState", "installdir"]), Some("Portal 2"));
        assert_eq!(kv.str(&["AppState", "LastOwner"]), Some("76561197960265729"));
        assert!(kv.obj(&["AppState", "InstalledDepots"]).map(|d| d.is_empty()).unwrap_or(false));
        assert_eq!(folder_name("Half-Life: Source / GOTY?", 1), "Half-Life Source  GOTY");
        assert_eq!(folder_name(" ../ ", 7), "app_7");
        assert!(safe_dir_name("Portal 2") && !safe_dir_name("..") && !safe_dir_name("a/b") && !safe_dir_name("") && !safe_dir_name(" x"));
    }

    #[test]
    fn picking_up_records() {
        let w = "record";
        assert_eq!(pickup(w, Some(w), false, 0, 10), Pickup::Waiting);
        assert_eq!(pickup(w, Some(w), true, 1000, 1000 + PICKUP_MS), Pickup::Waiting);
        assert_eq!(pickup(w, Some(w), true, 1000, 2000 + PICKUP_MS), Pickup::TimedOut);
        assert_eq!(pickup(w, Some(w), false, 1000, 2000 + PICKUP_MS), Pickup::Waiting, "Steam is restarting");
        assert_eq!(pickup(w, Some("rewritten by Steam"), true, 1000, 1500), Pickup::Taken);
        assert_eq!(pickup(w, None, true, 1000, 1500), Pickup::Gone);
        assert_eq!(pickup(w, None, false, 0, 1500), Pickup::Waiting);
    }

    #[test]
    fn libraries_by_drive() {
        let a = tempfile::tempdir().unwrap();
        let main = a.path().join("Steam");
        let other = a.path().join("SteamLibrary");
        std::fs::create_dir_all(&main).unwrap(); std::fs::create_dir_all(&other).unwrap();
        let libs = vec![main.clone(), other.clone()];
        assert_eq!(choose_library(&libs, Some(&main), &a.path().join("Games")), Some(main.clone()), "your choice");
        // Same drive as the install folder (a temp folder): the second library wins over Steam's own.
        assert_eq!(choose_library(&libs, None, &a.path().join("Games/not/yet")), Some(other.clone()));
        assert_eq!(choose_library(&libs, Some(Path::new("/gone")), &a.path().join("Games")), Some(other), "a library that's gone isn't used");
        assert_eq!(choose_library(&[], None, a.path()), None);
    }

    #[test]
    fn content_log_lines() {
        let log = "[2025-03-13 22:01:00] AppID 620 state changed : Update Required,Update Queued, (Manifest unavailable)\n\
                   [2025-03-13 22:01:05] AppID 620 update changed : Running Update,Downloading,\n\
                   [2025-03-13 22:01:10] Current download rate: 64.821 Mbps\n\
                   [2025-03-13 22:01:11] AppID 400 update changed : Running Update,Validating,\n\
                   [2025-03-13 22:01:12] AppID 400 state changed : Fully Installed,Update Running, (Update delayed for 300 secs)\n\
                   [2025-03-13 22:01:13] AppID 70 update changed : None\n";
        let c = parse_content_log(log);
        assert_eq!(c.rate, Some(8_102_625));
        assert_eq!(c.apps[&620], ("Downloading".to_string(), Some("Manifest unavailable".to_string())));
        assert_eq!(c.apps[&400], ("Verifying".to_string(), None), "a delay isn't a problem");
        assert_eq!(c.apps[&70].0, "");
        assert_eq!(parse_content_log("garbage\n[x] AppID y z").apps.len(), 0);
    }

    #[test]
    fn removes_only_inside_the_library() {
        let d = tempfile::tempdir().unwrap();
        let lib = d.path().join("lib");
        let apps = lib.join("steamapps");
        let outside = d.path().join("precious");
        std::fs::create_dir_all(apps.join("common/Portal 2/bin")).unwrap();
        std::fs::write(apps.join("common/Portal 2/bin/game"), "x").unwrap();
        std::fs::create_dir_all(apps.join("shadercache/620")).unwrap();
        std::fs::create_dir_all(apps.join("compatdata/620/pfx")).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(outside.join("keep"), "x").unwrap();
        std::os::unix::fs::symlink(&outside, apps.join("common/Linked")).unwrap();
        let m = apps.join("appmanifest_620.acf");
        std::fs::write(&m, manifest_text(620, "Portal 2", "Portal 2", 1)).unwrap();
        remove_files(&lib, &m, 620, "Portal 2").unwrap();
        assert!(!apps.join("common/Portal 2").exists() && !apps.join("shadercache/620").exists() && !m.exists());
        assert!(apps.join("compatdata/620/pfx").exists(), "saves in the Proton prefix stay");
        // A game folder that's a link: the link goes, what it points to stays.
        let m2 = apps.join("appmanifest_9.acf");
        std::fs::write(&m2, manifest_text(9, "Linked", "Linked", 1)).unwrap();
        remove_files(&lib, &m2, 9, "Linked").unwrap();
        assert!(!apps.join("common/Linked").exists() && outside.join("keep").exists());
        // A record that names a path outside: nothing outside is touched.
        let m3 = apps.join("appmanifest_10.acf");
        std::fs::write(&m3, manifest_text(10, "Bad", "../../precious", 1)).unwrap();
        remove_files(&lib, &m3, 10, "../../precious").unwrap();
        assert!(outside.join("keep").exists());
        // A record that isn't in a library's steamapps folder is refused.
        assert!(remove_files(&lib, &outside.join("keep"), 1, "x").is_err());
        assert!(outside.join("keep").exists());
    }

    #[test]
    fn closing_an_unneeded_steam() {
        let idle = IdleCheck { on_demand: true, running: true, started_by_hearth: true, since_use: IDLE_CLOSE_MS, since_up: IDLE_CLOSE_MS, ..Default::default() };
        assert!(should_close(&idle));
        for (what, i) in [
            ("setting off", IdleCheck { on_demand: false, ..idle.clone() }),
            ("you started Steam", IdleCheck { started_by_hearth: false, ..idle.clone() }),
            ("a game", IdleCheck { game_running: true, ..idle.clone() }),
            ("a download", IdleCheck { working: true, ..idle.clone() }),
            ("content activity", IdleCheck { content_recent: true, ..idle.clone() }),
            ("cloud sync", IdleCheck { cloud_recent: true, ..idle.clone() }),
            ("an install waiting", IdleCheck { pending: true, ..idle.clone() }),
            ("used lately", IdleCheck { since_use: IDLE_CLOSE_MS - 1, ..idle.clone() }),
            ("just started", IdleCheck { since_up: 1000, ..idle.clone() }),
            ("starting or closing", IdleCheck { busy: true, ..idle.clone() }),
        ] { assert!(!should_close(&i), "{what}"); }
    }

    #[test]
    fn sign_in_detection() {
        let before = SignState { recent: None, users_changed: 100, auto: None };
        assert!(!signed_in_since(&before, &before));
        assert!(signed_in_since(&before, &SignState { recent: Some(76561197960265729), users_changed: 200, auto: Some("sam".into()) }));
        let signed = SignState { recent: Some(1), users_changed: 100, auto: Some("sam".into()) };
        assert!(!signed_in_since(&signed, &signed), "nothing changed");
        assert!(signed_in_since(&signed, &SignState { recent: Some(2), users_changed: 100, auto: Some("jo".into()) }), "switched accounts");
        assert!(!signed_in_since(&signed, &SignState { recent: None, users_changed: 300, auto: None }), "signed out");
    }
}
