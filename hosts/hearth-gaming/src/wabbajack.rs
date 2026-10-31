//! Wabbajack modlists in Hearth Mods: the gallery of public lists, installing one, and putting it in Hearth's
//! library so it starts from Game Mode like any game.
//!
//! The install itself is done by jackify-engine (Jackify's native Linux build of wabbajack-cli, GPL-3.0, like
//! Wabbajack and Hearth): it downloads every archive the list needs, checks each one's hash against the list, and
//! builds the list's folder (Mod Organizer 2, its mods and profiles). Hearth OS carries the engine
//! (/usr/lib/hearth/jackify-engine); otherwise Hearth Mods downloads it once from its GitHub releases.
//!
//! What Hearth adds around it: the Nexus key, and the LoversLab and MEGA sign-ins (accounts.rs), in the engine's
//! environment. Files the engine can't download itself: a free Nexus account's files go in Hearth's download queue
//! as "needs-link" items, filled by pressing "Mod Manager Download" (downloads.rs); everything else (LoversLab and
//! Vector Plexus files, read from the .wabbajack itself, "manual" downloads, files the engine reported failed) goes in
//! the guided flow (guided.rs): Hearth opens each file's page in Hearth Browser, catches the download, checks name,
//! size and hash, and moves it into the list's downloads; Google Drive files come with your Google account when
//! you're signed in. When the engine's own ones are there it is told to carry on; when it had stopped, Hearth starts
//! it again by itself.
//! Afterwards Hearth points Mod Organizer at the game and the downloads with Wine's Z: paths, makes the list its
//! own Proton prefix with the Windows parts these lists need (umu-run winetricks), and writes
//! mods/modlists/<id>.json, which Hearth's library reads (hearth-core library.rs modlist_games).

use crate::downloads;
use crate::util;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub const ENGINE_REPO: &str = "Omni-guides/dev-jackify-engine";
pub const ENGINE_SOURCE: &str = "https://github.com/Omni-guides/dev-jackify-engine";
const SYSTEM_ENGINE: &str = "/usr/lib/hearth/jackify-engine/jackify-engine";

fn root() -> PathBuf { util::data_dir().join("mods").join("wabbajack") }
fn jobs_dir() -> PathBuf { root().join("jobs") }
fn modlists_dir() -> PathBuf { util::data_dir().join("mods").join("modlists") }
fn own_engine_dir() -> PathBuf { util::data_dir().join("tools").join("jackify-engine") }

// ------------------------------------------------------------------ the engine

/// Where jackify-engine is: $HEARTH_JACKIFY_ENGINE, Hearth OS's copy, or the one Hearth Mods downloaded.
pub fn engine() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("HEARTH_JACKIFY_ENGINE").map(PathBuf::from).filter(|p| p.is_file()) { return Some(p); }
    let own = own_engine_dir().join("jackify-engine");
    // Hearth Mods' own copy is newer when it was updated after the system's.
    [own, PathBuf::from(SYSTEM_ENGINE)].into_iter().find(|p| p.is_file())
}
pub fn engine_info() -> Value {
    let p = engine();
    let ver = p.as_ref().and_then(|p| std::fs::read_to_string(p.with_file_name("hearth-version.txt")).ok()).map(|s| s.trim().to_string()).unwrap_or_default();
    json!({"path": p, "version": ver, "source": ENGINE_SOURCE, "system": p.as_deref() == Some(Path::new(SYSTEM_ENGINE))})
}

/// Downloads the newest jackify-engine for Linux from its GitHub releases into Hearth's tools folder.
pub fn fetch_engine() -> Result<Value, String> {
    let (status, _, body) = crate::nexus::curl_get(&format!("https://api.github.com/repos/{ENGINE_REPO}/releases/latest"), &[("Accept", "application/vnd.github+json")], 30)?;
    if status != 200 { return Err(format!("Couldn't look up jackify-engine's newest release ({status}).")); }
    let rel: Value = serde_json::from_str(&body).unwrap_or(Value::Null);
    let tag = rel["tag_name"].as_str().unwrap_or("").to_string();
    let assets = rel["assets"].as_array().cloned().unwrap_or_default();
    let pick = |want: &dyn Fn(&str) -> bool| assets.iter().find(|a| want(&a["name"].as_str().unwrap_or("").to_lowercase())).cloned();
    let asset = pick(&|n| n.contains("linux") && n.contains("x64") && n.ends_with(".tar.gz"))
        .or_else(|| pick(&|n| n.starts_with("jackify-engine") && n.ends_with(".tar.gz")))
        .or_else(|| pick(&|n| n.starts_with("jackify-engine") && n.ends_with(".zip")))
        .ok_or("jackify-engine's newest release has no Linux download.")?;
    let url = asset["browser_download_url"].as_str().ok_or("jackify-engine's download has no address.")?.to_string();
    if !url.starts_with("https://github.com/") && !url.starts_with("https://objects.githubusercontent.com/") { return Err("jackify-engine's download isn't on GitHub; not using it.".into()); }
    let name = asset["name"].as_str().unwrap_or("engine.tar.gz").to_string();
    let tmp = util::data_dir().join("tools").join(format!(".jackify-engine-{}", util::now_ms()));
    std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    let r = (|| -> Result<(), String> {
        let file = tmp.join(&name);
        let st = std::process::Command::new("curl").args(["-fsSL", "--proto", "=https", "--retry", "2", "-o"]).arg(&file).arg(&url).status().map_err(|e| e.to_string())?;
        if !st.success() { return Err("Couldn't download jackify-engine. Is this PC online?".into()); }
        let out = tmp.join("x");
        std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
        let ok = if name.ends_with(".zip") { std::process::Command::new("bsdtar").arg("-xf").arg(&file).arg("-C").arg(&out).status() }
                 else { std::process::Command::new("tar").arg("-xzf").arg(&file).arg("-C").arg(&out).status() };
        if !ok.map(|s| s.success()).unwrap_or(false) { return Err("Couldn't unpack jackify-engine.".into()); }
        let mut files = Vec::new();
        util::files_under(&out, &mut files);
        let bin = files.iter().find(|f| f.file_name().map(|n| n == "jackify-engine").unwrap_or(false)).ok_or("The download has no jackify-engine program in it.")?;
        let dest = own_engine_dir();
        let _ = std::fs::remove_dir_all(&dest);
        std::fs::rename(bin.parent().unwrap(), &dest).map_err(|e| format!("Couldn't keep jackify-engine ({e})"))?;
        #[cfg(unix)] { use std::os::unix::fs::PermissionsExt; let _ = std::fs::set_permissions(dest.join("jackify-engine"), std::fs::Permissions::from_mode(0o755)); }
        let _ = std::fs::write(dest.join("hearth-version.txt"), &tag);
        Ok(())
    })();
    let _ = std::fs::remove_dir_all(&tmp);
    r?;
    Ok(engine_info())
}

fn engine_cmd(engine: &Path) -> std::process::Command {
    let mut c = std::process::Command::new(engine);
    // The engine finds its extractors and tools next to itself.
    if let Some(d) = engine.parent() { c.current_dir(d); }
    c.env("DOTNET_SYSTEM_GLOBALIZATION_INVARIANT", "1");
    c
}

// ------------------------------------------------------------------ the gallery

/// The public modlists (from the engine's list-modlists), kept for six hours.
pub fn gallery(refresh: bool) -> Result<Value, String> {
    let f = root().join("gallery.json");
    let fresh = std::fs::metadata(&f).ok().and_then(|m| m.modified().ok()).and_then(|t| t.elapsed().ok()).map(|d| d.as_secs() < 6 * 3600).unwrap_or(false);
    let cached = util::read_json(&f);
    if fresh && !refresh && cached.is_array() { return Ok(cached); }
    let eng = engine().ok_or("The Wabbajack engine isn't installed yet.")?;
    let out = engine_cmd(&eng).args(["list-modlists", "--json", "--sort-by", "title"]).stdin(std::process::Stdio::null()).output().map_err(|e| format!("Couldn't start the Wabbajack engine ({e})"))?;
    let text = String::from_utf8_lossy(&out.stdout);
    let start = text.find('{').ok_or_else(|| if cached.is_array() { String::new() } else { "The Wabbajack engine couldn't get the list of modlists. Is this PC online?".into() });
    let v: Value = match start { Ok(i) => serde_json::from_str(&text[i..]).unwrap_or(Value::Null), Err(e) if e.is_empty() => return Ok(cached), Err(e) => return Err(e) };
    let lists: Vec<Value> = v["modlists"].as_array().cloned().unwrap_or_default().iter().map(slim).collect();
    if lists.is_empty() { return if cached.is_array() { Ok(cached) } else { Err("The Wabbajack engine gave no modlists.".into()) }; }
    let lists = Value::Array(lists);
    let _ = util::write_json(&f, &lists);
    Ok(lists)
}

/// What the page needs of a modlist.
fn slim(m: &Value) -> Value {
    json!({"title": m["title"], "author": m["author"], "description": m["description"], "id": m["namespacedName"], "machineUrl": m["machineURL"],
           "game": m["game"], "gameName": m["gameHumanFriendly"], "nsfw": m["nsfw"], "official": m["official"], "down": m["forceDown"] == true || m["validation"]["hasFailures"] == true,
           "version": m["version"], "tags": m["tags"], "image": m["images"]["large"].as_str().or(m["links"]["image"].as_str()), "thumb": m["images"]["small"],
           "readme": m["links"]["readme"], "download": m["sizes"]["downloadSize"], "install": m["sizes"]["installSize"], "archives": m["sizes"]["numberOfArchives"],
           "updated": m["dateUpdated"], "steamApp": GAMES.iter().find(|g| g.0.eq_ignore_ascii_case(m["game"].as_str().unwrap_or(""))).map(|g| g.1)})
}

/// Wabbajack's game names → (Steam app, umu game id for Proton's fixes, winetricks set, the game's own folder names
/// in Documents/My Games and AppData/Local).
pub const GAMES: [(&str, u64, &str, &str); 13] = [
    ("SkyrimSpecialEdition", 489830, "modern", "Skyrim Special Edition"),
    ("Skyrim", 72850, "legacy", "Skyrim"),
    ("SkyrimVR", 611670, "modern", "Skyrim VR"),
    ("Fallout4", 377160, "modern", "Fallout4"),
    ("Fallout4VR", 611660, "modern", "Fallout4VR"),
    ("FalloutNewVegas", 22380, "legacy", "FalloutNV"),
    ("Fallout3", 22300, "legacy", "Fallout3"),
    ("Oblivion", 22330, "legacy", "Oblivion"),
    ("OblivionRemastered", 2623190, "modern", "Oblivion Remastered"),
    ("EnderalSpecialEdition", 976620, "modern", "Enderal Special Edition"),
    ("Starfield", 1716740, "modern", "Starfield"),
    ("Cyberpunk2077", 1091500, "modern", "Cyberpunk 2077"),
    ("BaldursGate3", 1086940, "modern", "Baldur's Gate 3"),
];
const WT_BASE: [&str; 4] = ["fontsmooth=rgb", "xact", "xact_x64", "vcrun2022"];
const WT_MODERN: [&str; 7] = ["d3dcompiler_47", "d3dx11_43", "d3dcompiler_43", "dotnet6", "dotnet7", "dotnet8", "dotnetdesktop6"];
const WT_LEGACY: [&str; 4] = ["d3dx9_43", "d3dx9", "d3dcompiler_43", "d3dcompiler_47"];

pub fn game_info(game: &str) -> Option<(&'static str, u64, &'static str, &'static str)> { GAMES.iter().find(|g| g.0.eq_ignore_ascii_case(game)).copied() }

// ------------------------------------------------------------------ installing

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manual {
    pub file_name: String, pub url: String, pub size: u64, pub nexus: bool, pub domain: String, pub mod_id: u64, pub file_id: u64, pub mod_name: String, pub here: bool,
    #[serde(default)] pub download: String,
    /// Wabbajack's xxHash64 of the file (guided::hash_text), when known: a file is only taken when it matches.
    #[serde(default)] pub hash: String,
    /// Where it comes from, for the page ("LoversLab", "MEGA", "Google Drive", a web address's host).
    #[serde(default)] pub site: String,
    /// Found by Hearth (from the list, or a download the engine reported failed), not by the engine's own manual
    /// download step: when they're all here the engine is started again rather than told to carry on.
    #[serde(default)] pub from_list: bool,
    /// Skipped for now in the guided flow (comes back after the others).
    #[serde(default)] pub skipped: bool,
    /// The last problem with it (a wrong file in Downloads, Google refusing it).
    #[serde(default)] pub problem: String,
    /// A Google Drive file's id: with a Google sign-in, Hearth downloads it itself.
    #[serde(default)] pub gdrive: String,
    #[serde(default)] pub tried_google: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    pub id: String,
    pub title: String,
    /// A gallery list ("repository/name") or a .wabbajack file.
    pub list: String,
    pub game: String,
    pub game_dir: PathBuf,
    pub install_dir: PathBuf,
    pub downloads_dir: PathBuf,
    /// installing, manual (waiting for files), setting-up, done, failed, cancelled
    pub state: String,
    pub phase: String,
    pub status: String,
    pub step: Option<(u64, u64)>,
    pub count: Option<(u64, u64)>,
    #[serde(default)]
    pub manual: Vec<Manual>,
    #[serde(default)]
    pub error: String,
    #[serde(default)]
    pub notes: Vec<String>,
    pub started: u64,
    #[serde(default)]
    pub finished: u64,
    #[serde(default)]
    pub premium_needed: bool,
    /// The engine is waiting for its manual files (it carries on when told "continue").
    #[serde(default)]
    pub waiting: bool,
    /// Times Hearth started the engine again by itself after fetching files it couldn't (a guard against loops).
    #[serde(default)]
    pub auto_resumes: u32,
}
impl Job {
    fn file(&self) -> PathBuf { jobs_dir().join(format!("{}.json", self.id)) }
    pub fn log(&self) -> PathBuf { jobs_dir().join(format!("{}.log", self.id)) }
    fn save(&self) { let _ = util::write_json(&self.file(), &serde_json::to_value(self).unwrap_or(Value::Null)); }
    pub fn public(&self) -> Value {
        let mut v = serde_json::to_value(self).unwrap_or(Value::Null);
        v["log"] = json!(self.log());
        v["running"] = json!(RUN.lock().unwrap_or_else(|e| e.into_inner()).contains_key(&self.id));
        v["guide"] = guide(&self.manual);
        v
    }
}

/// Running installs: their stdin (to say "continue") and whether to stop.
struct Live { stdin: Option<std::process::ChildStdin>, pid: u32, stop: bool }
static RUN: Mutex<BTreeMap<String, Arc<Mutex<Live>>>> = Mutex::new(BTreeMap::new());
static JOBS: Mutex<BTreeMap<String, Job>> = Mutex::new(BTreeMap::new());

fn load_jobs() {
    let mut j = JOBS.lock().unwrap_or_else(|e| e.into_inner());
    if !j.is_empty() { return; }
    if let Ok(rd) = std::fs::read_dir(jobs_dir()) {
        for e in rd.flatten() {
            if e.path().extension().map(|x| x == "json").unwrap_or(false) {
                if let Ok(mut job) = serde_json::from_value::<Job>(util::read_json(&e.path())) {
                    // An install stopped by closing the app can be started again (the engine skips what's done).
                    if matches!(job.state.as_str(), "installing" | "manual" | "setting-up") { job.state = "stopped".into(); job.status = "Stopped when Hearth Mods closed. Press Resume to carry on.".into(); }
                    j.insert(job.id.clone(), job);
                }
            }
        }
    }
}
fn update(id: &str, f: impl FnOnce(&mut Job)) -> Option<Job> {
    let mut j = JOBS.lock().unwrap_or_else(|e| e.into_inner());
    let job = j.get_mut(id)?;
    f(job);
    job.save();
    Some(job.clone())
}
pub fn jobs() -> Value { load_jobs(); json!(JOBS.lock().unwrap_or_else(|e| e.into_inner()).values().rev().map(Job::public).collect::<Vec<_>>()) }
pub fn job(id: &str) -> Option<Job> { load_jobs(); JOBS.lock().unwrap_or_else(|e| e.into_inner()).get(id).cloned() }

/// Free space where a list goes.
pub fn free_space(p: &Path) -> Option<u64> {
    let mut d = p.to_path_buf();
    while !d.exists() { if !d.pop() { return None; } }
    let out = std::process::Command::new("df").args(["-B1", "--output=avail"]).arg(&d).output().ok()?;
    String::from_utf8_lossy(&out.stdout).lines().nth(1)?.trim().parse().ok()
}

/// Starts installing a modlist. `list` is a gallery id (repository/name) or a .wabbajack file.
pub fn start(title: &str, list: &str, game: &str, game_dir: &Path, install_dir: &Path, downloads_dir: &Path) -> Result<Job, String> {
    start_with(title, list, game, game_dir, install_dir, downloads_dir, 0)
}
fn start_with(title: &str, list: &str, game: &str, game_dir: &Path, install_dir: &Path, downloads_dir: &Path, auto_resumes: u32) -> Result<Job, String> {
    load_jobs();
    let eng = engine().ok_or("The Wabbajack engine isn't installed yet.")?;
    if list.trim().is_empty() { return Err("Choose a modlist.".into()); }
    if list.ends_with(".wabbajack") && !Path::new(list).is_file() { return Err("That .wabbajack file isn't there anymore.".into()); }
    if !game_dir.is_dir() { return Err("Install the game first (Wabbajack lists are built on the game's own files), or choose its folder.".into()); }
    if !install_dir.is_absolute() || !downloads_dir.is_absolute() { return Err("Choose full folders for the modlist and its downloads.".into()); }
    if install_dir.starts_with(game_dir) || game_dir.starts_with(install_dir) { return Err("The modlist can't go inside the game's folder (or the game inside it). Choose another folder.".into()); }
    if std::fs::read_dir(install_dir).map(|mut r| r.next().is_some()).unwrap_or(false) && !install_dir.join("ModOrganizer.ini").is_file() {
        return Err("That folder already has other things in it. Choose an empty folder (or the folder of this list, to update it).".into());
    }
    std::fs::create_dir_all(install_dir).map_err(|e| format!("Couldn't make {} ({e})", install_dir.display()))?;
    std::fs::create_dir_all(downloads_dir).map_err(|e| format!("Couldn't make {} ({e})", downloads_dir.display()))?;
    if JOBS.lock().unwrap_or_else(|e| e.into_inner()).values().any(|j| j.install_dir == install_dir && RUN.lock().unwrap_or_else(|e| e.into_inner()).contains_key(&j.id)) { return Err("That modlist is already being installed.".into()); }
    let existing = JOBS.lock().unwrap_or_else(|e| e.into_inner()).values().find(|j| j.install_dir == install_dir).map(|j| j.id.clone());
    let id = existing.unwrap_or_else(|| format!("w{}", util::now_ms()));
    let job = Job { id: id.clone(), title: if title.trim().is_empty() { list.rsplit('/').next().unwrap_or("Modlist").trim_end_matches(".wabbajack").to_string() } else { title.trim().to_string() },
        list: list.into(), game: game.into(), game_dir: game_dir.into(), install_dir: install_dir.into(), downloads_dir: downloads_dir.into(),
        state: "installing".into(), phase: "Starting".into(), started: util::now_ms(), auto_resumes, ..Default::default() };
    job.save();
    JOBS.lock().unwrap_or_else(|e| e.into_inner()).insert(id.clone(), job.clone());
    let j2 = job.clone();
    std::thread::spawn(move || run(j2, eng));
    Ok(job)
}
pub fn resume(id: &str) -> Result<Job, String> {
    let j = job(id).ok_or("That install isn't in the list anymore.")?;
    start(&j.title, &j.list, &j.game, &j.game_dir, &j.install_dir, &j.downloads_dir)
}
/// Hearth starting the engine again by itself once the files it couldn't fetch are here (at most 3 times in a row).
fn resume_auto(j: &Job) -> bool {
    if j.auto_resumes >= 3 { return false; }
    start_with(&j.title, &j.list, &j.game, &j.game_dir, &j.install_dir, &j.downloads_dir, j.auto_resumes + 1).is_ok()
}
pub fn cancel(id: &str) {
    if let Some(l) = RUN.lock().unwrap_or_else(|e| e.into_inner()).get(id) {
        let mut l = l.lock().unwrap_or_else(|e| e.into_inner());
        l.stop = true;
        #[cfg(unix)] unsafe { libc::kill(-(l.pid as i32), libc::SIGTERM); libc::kill(l.pid as i32, libc::SIGTERM); }
    }
}
/// Takes an install out of the list (and, when asked, deletes the list's folder; downloads stay unless asked).
pub fn forget(id: &str, delete_files: bool, delete_downloads: bool) -> Result<(), String> {
    if RUN.lock().unwrap_or_else(|e| e.into_inner()).contains_key(id) { return Err("Stop the install first.".into()); }
    let j = job(id).ok_or("That install isn't in the list anymore.")?;
    if delete_files && j.install_dir.join("ModOrganizer.ini").is_file() { std::fs::remove_dir_all(&j.install_dir).map_err(|e| format!("Couldn't delete the modlist's folder ({e})"))?; }
    if delete_downloads && j.downloads_dir.is_dir() && j.downloads_dir != util::home() { let _ = std::fs::remove_dir_all(&j.downloads_dir); }
    let _ = std::fs::remove_file(modlists_dir().join(format!("{id}.json")));
    let _ = std::fs::remove_file(modlists_dir().join(format!("{id}.desktop")));
    let _ = std::fs::remove_file(j.file());
    for d in downloads::for_job(id) { if d.state != "done" { downloads::remove(&d.id, false); } }
    JOBS.lock().unwrap_or_else(|e| e.into_inner()).remove(id);
    Ok(())
}

fn run(start_job: Job, eng: PathBuf) {
    let job = start_job;
    let id = job.id.clone();
    let mut cmd = engine_cmd(&eng);
    cmd.arg("install").arg("--show-file-progress");
    if job.list.ends_with(".wabbajack") { cmd.arg("-w").arg(&job.list); } else { cmd.arg("-m").arg(&job.list); }
    cmd.arg("-o").arg(&job.install_dir).arg("-d").arg(&job.downloads_dir).arg("-g").arg(&job.game_dir);
    if let Some(a) = crate::nexus::account() { cmd.env("NEXUS_API_KEY", a.key); }
    // LoversLab and MEGA sign-ins (accounts.rs), in the engine's environment like the Nexus key.
    for (k, v) in crate::accounts::engine_env() { cmd.env(k, v); }
    #[cfg(unix)] { use std::os::unix::process::CommandExt; cmd.process_group(0); }
    let log = std::fs::OpenOptions::new().create(true).append(true).open(job.log()).ok().map(|f| Arc::new(Mutex::new(f)));
    let write_log = |s: &str| if let Some(l) = &log { let _ = writeln!(l.lock().unwrap_or_else(|e| e.into_inner()), "{s}"); };
    write_log(&format!("--- {} install of {} into {}", util::iso_now(), job.list, job.install_dir.display()));
    let child = cmd.stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).spawn();
    let mut child = match child {
        Ok(c) => c,
        Err(e) => { update(&id, |j| { j.state = "failed".into(); j.error = format!("Couldn't start the Wabbajack engine ({e})"); }); return; }
    };
    let live = Arc::new(Mutex::new(Live { stdin: child.stdin.take(), pid: child.id(), stop: false }));
    RUN.lock().unwrap_or_else(|e| e.into_inner()).insert(id.clone(), live.clone());
    // Errors go to the log too.
    if let Some(err) = child.stderr.take() {
        let lg = log.clone();
        std::thread::spawn(move || { for l in std::io::BufReader::new(err).lines().map_while(Result::ok) { if let Some(f) = &lg { let _ = writeln!(f.lock().unwrap_or_else(|e| e.into_inner()), "! {l}"); } } });
    }
    let mut pending: Vec<Manual> = Vec::new();
    let mut last_error = String::new();
    // The list's archives (read from the .wabbajack once it's there), for files the engine can't or didn't fetch.
    let mut archives: Option<Vec<ListArchive>> = None;
    scan_list(&id, &mut archives);
    if let Some(out) = child.stdout.take() {
        // Progress lines end in \r as often as \n.
        let mut r = std::io::BufReader::new(out);
        let mut buf = Vec::new();
        loop {
            buf.clear();
            let n = read_until_any(&mut r, &mut buf);
            if n == 0 { break; }
            let line = String::from_utf8_lossy(&buf).trim().to_string();
            if line.is_empty() || line.contains("[FILE_PROGRESS]") { continue; }
            if line.starts_with('{') {
                if let Ok(ev) = serde_json::from_str::<Value>(&line) {
                    match ev["event"].as_str().unwrap_or("") {
                        "manual_download_required" => { pending.push(manual_of(&ev)); continue; }
                        "manual_download_list_complete" => {
                            let items = std::mem::take(&mut pending);
                            write_log(&format!("needs {} files Hearth has to fetch another way", items.len()));
                            ask_for(&id, items, &live);
                            continue;
                        }
                        "manual_download_phase_complete" => { update(&id, |j| { j.state = "installing".into(); j.waiting = false; j.manual.retain(|m| m.from_list && !m.here); }); continue; }
                        _ => {}
                    }
                }
            }
            write_log(&line);
            if archives.is_none() && line.contains("Missing ") && line.contains(" archives") { scan_list(&id, &mut archives); }
            if line.contains("Failed to download '") { failed_download(&id, &line, &mut archives); }
            let low = line.to_lowercase();
            if low.contains("error") || low.contains("failed") || low.contains("exception") { last_error = line.clone(); }
            let premium = low.contains("premium") && (low.contains("required") || low.contains("403") || low.contains("non-premium"));
            let p = parse_progress(&line);
            update(&id, |j| {
                j.status = line.chars().take(300).collect();
                if let Some((ph, cnt)) = &p { if !ph.is_empty() { j.phase = ph.clone(); } if cnt.is_some() { j.count = *cnt; } }
                if let Some(s) = step_of(&line) { j.step = Some(s); }
                if premium { j.premium_needed = true; }
            });
        }
    }
    let status = child.wait();
    RUN.lock().unwrap_or_else(|e| e.into_inner()).remove(&id);
    let stopped = live.lock().unwrap_or_else(|e| e.into_inner()).stop;
    write_log(&format!("--- engine finished: {status:?}"));
    match status {
        _ if stopped => { update(&id, |j| { j.state = "cancelled".into(); j.status = "Stopped. Resume carries on where it stopped.".into(); }); }
        Ok(s) if s.success() => {
            update(&id, |j| { j.state = "setting-up".into(); j.phase = "Setting up for Hearth".into(); });
            let j = self::job(&id).unwrap_or(job);
            match set_up(&j) {
                Ok(notes) => { update(&id, |j| { j.state = "done".into(); j.phase = "Ready".into(); j.status = "Installed. It's in your library.".into(); j.notes = notes; j.finished = util::now_ms(); j.error.clear(); }); }
                Err(e) => { update(&id, |j| { j.state = "failed".into(); j.error = format!("Installed, but setting it up didn't work: {e}"); }); }
            }
        }
        _ => {
            // Files Hearth found the engine couldn't fetch: wait for them (the guided flow), then start again.
            let j = self::job(&id).unwrap_or(job);
            let outstanding = j.manual.iter().filter(|m| !m.here).count();
            let fetched = j.manual.iter().any(|m| m.from_list && m.here);
            if outstanding > 0 {
                update(&id, |j| { j.state = "manual".into(); j.phase = "Files to fetch".into(); j.waiting = false; j.error.clear();
                                   j.status = format!("{outstanding} file{} to fetch; then the install carries on by itself.", if outstanding == 1 { "" } else { "s" }); });
                watch(&id);
            } else if fetched && resume_auto(&j) {
                write_log("--- the files Hearth fetched are here: starting the engine again");
            } else {
                update(&id, |j| { j.state = "failed".into(); j.error = if last_error.is_empty() { "The Wabbajack engine stopped with a problem. The log has what it said.".into() } else { last_error.chars().take(400).collect() }; });
            }
        }
    }
}

/// Reads up to and including the next \n or \r.
fn read_until_any(r: &mut impl BufRead, buf: &mut Vec<u8>) -> usize {
    let mut n = 0;
    loop {
        let (done, used) = {
            let Ok(avail) = r.fill_buf() else { return n };
            if avail.is_empty() { return n; }
            match avail.iter().position(|&b| b == b'\n' || b == b'\r') {
                Some(i) => { buf.extend_from_slice(&avail[..=i]); (true, i + 1) }
                None => { buf.extend_from_slice(avail); (false, avail.len()) }
            }
        };
        r.consume(used);
        n += used;
        if done { return n; }
    }
}

/// "[00:00:10] Downloading Mod Archives (17/214) - 6.8MB/s" → ("Downloading Mod Archives", (17, 214)).
pub fn parse_progress(line: &str) -> Option<(String, Option<(u64, u64)>)> {
    let mut s = line.trim();
    if s.starts_with('[') { if let Some(i) = s.find(']') { s = s[i + 1..].trim(); } }
    let open = s.find('(')?;
    let close = s[open..].find(')')? + open;
    let inside = &s[open + 1..close];
    let (a, b) = inside.split_once('/')?;
    let count = match (a.trim().parse::<u64>(), b.trim().parse::<u64>()) { (Ok(a), Ok(b)) if b > 0 => Some((a, b)), _ => None };
    let phase = s[..open].trim().trim_end_matches(':').to_string();
    if phase.is_empty() || phase.len() > 80 { return None; }
    Some((phase, count))
}
/// "[12/14] Installing files" → (12, 14).
pub fn step_of(line: &str) -> Option<(u64, u64)> {
    let s = line.trim().strip_prefix('[')?;
    let (inside, _) = s.split_once(']')?;
    let (a, b) = inside.split_once('/')?;
    Some((a.trim().parse().ok()?, b.trim().parse().ok()?))
}

fn manual_of(ev: &Value) -> Manual {
    let url = ["nexus_url", "download_url", "manual_url", "url"].iter().find_map(|k| ev[*k].as_str()).unwrap_or("").to_string();
    let (mut domain, mut mod_id) = (String::new(), ev["mod_id"].as_u64().unwrap_or(0));
    // https://www.nexusmods.com/<game>/mods/<id>?tab=files&file_id=<f>
    let nexus_url = url.contains("nexusmods.com/");
    if nexus_url {
        let path = url.split("nexusmods.com/").nth(1).unwrap_or("").split('?').next().unwrap_or("");
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() >= 3 && parts[1] == "mods" { domain = parts[0].to_ascii_lowercase(); if mod_id == 0 { mod_id = parts[2].parse().unwrap_or(0); } }
    }
    if let Some(g) = ev["game_domain"].as_str().or(ev["game"].as_str()) { if domain.is_empty() { domain = g.to_ascii_lowercase(); } }
    let file_id = ev["file_id"].as_u64().or_else(|| crate::nexus::urldecode(&url).split("file_id=").nth(1).and_then(|s| s.split('&').next()).and_then(|s| s.parse().ok())).unwrap_or(0);
    let hash = ev["expected_hash"].as_str().and_then(crate::guided::parse_hash).map(crate::guided::hash_text).unwrap_or_default();
    let site = if nexus_url { "Nexus Mods".to_string() } else { site_of_url(&url) };
    Manual { file_name: ev["file_name"].as_str().unwrap_or("").to_string(), size: ev["expected_size"].as_u64().unwrap_or(0), nexus: nexus_url && file_id > 0 && !domain.is_empty(),
             domain, mod_id, file_id, mod_name: ev["mod_name"].as_str().unwrap_or("").to_string(), hash, site, gdrive: gdrive_id(&url), url, ..Default::default() }
}

/// The engine needs files it can't download itself: they go in Hearth's queue (Nexus) or the guided flow (other
/// sites). When the engine's ones are all in the list's downloads folder, the engine carries on.
fn ask_for(id: &str, items: Vec<Manual>, _live: &Arc<Mutex<Live>>) {
    let Some(j) = job(id) else { return };
    let mut items = items;
    for m in items.iter_mut() {
        m.here = present(&j.downloads_dir, m);
        if !m.here && m.nexus {
            let premium = crate::nexus::account().map(|a| a.premium).unwrap_or(false);
            let want = downloads::Want { dir: Some(j.downloads_dir.clone()), file_name: Some(m.file_name.clone()), game: String::new(), job: id.to_string() };
            let d = if premium { downloads::add_file(&m.domain, m.mod_id, m.file_id, want).ok() } else { None };
            let d = d.unwrap_or_else(|| downloads::add_needs_link(&m.domain, m.mod_id, m.file_id, if m.mod_name.is_empty() { &m.file_name } else { &m.mod_name }, &m.file_name, m.size, &j.downloads_dir, id));
            m.download = d.id;
        }
    }
    update(id, |jj| {
        // Files Hearth already found from the list stay, once each.
        let keep: Vec<Manual> = jj.manual.iter().filter(|o| o.from_list && !items.iter().any(|m| m.file_name == o.file_name)).cloned().collect();
        jj.state = "manual".into(); jj.phase = "Files to fetch".into(); jj.waiting = true;
        jj.manual = items.clone();
        jj.manual.extend(keep);
    });
    watch(id);
}

static WATCHING: Mutex<std::collections::BTreeSet<String>> = Mutex::new(std::collections::BTreeSet::new());
static SEEN: Mutex<Option<std::collections::HashSet<(PathBuf, u64, u64)>>> = Mutex::new(None);

/// Looks every two seconds for the files a job still needs, until they're all there or the install stops.
fn watch(id: &str) {
    if !WATCHING.lock().unwrap_or_else(|e| e.into_inner()).insert(id.to_string()) { return; }
    let id = id.to_string();
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_secs(2));
            if !tick(&id) { break; }
        }
        WATCHING.lock().unwrap_or_else(|e| e.into_inner()).remove(&id);
    });
}

pub fn want_of(m: &Manual) -> crate::guided::Want {
    crate::guided::Want { name: m.file_name.clone(), size: m.size, hash: crate::guided::hash_of_text(&m.hash) }
}

/// One look: picks up downloaded files (checked against the list), starts Google Drive downloads with your Google
/// account, tells a waiting engine to carry on, or starts it again. Whether to keep looking.
fn tick(id: &str) -> bool {
    use crate::guided::{self, PickUp};
    let Some(j) = job(id) else { return false };
    if !matches!(j.state.as_str(), "installing" | "manual") { return false; }
    let running = RUN.lock().unwrap_or_else(|e| e.into_inner()).contains_key(id);
    let dirs = guided::browser_download_dirs();
    let google = crate::accounts::signed_in("google");
    let mut list = j.manual.clone();
    for m in list.iter_mut().filter(|m| !m.here) {
        if present(&j.downloads_dir, m) { m.here = true; continue; }
        if m.nexus { continue; }
        if google && !m.gdrive.is_empty() && !m.tried_google { m.tried_google = true; fetch_google(id, &j.downloads_dir, m.clone()); continue; }
        let mut seen = SEEN.lock().unwrap_or_else(|e| e.into_inner());
        match guided::pick_up(&dirs, &j.downloads_dir, &want_of(m), seen.get_or_insert_with(Default::default)) {
            PickUp::Moved(_) => { m.here = true; m.problem.clear(); }
            PickUp::Wrong(e) => m.problem = e,
            PickUp::Nothing => {}
        }
    }
    // Merged by name: the engine's reader may have added files meanwhile.
    let j = update(id, |jj| for m in jj.manual.iter_mut() {
        if let Some(n) = list.iter().find(|n| n.file_name == m.file_name) { m.here |= n.here; m.problem = n.problem.clone(); m.tried_google |= n.tried_google; }
    }).unwrap_or(j);
    let all = j.manual.iter().all(|m| m.here);
    if j.waiting && running && j.manual.iter().filter(|m| !m.from_list).all(|m| m.here) {
        if let Some(l) = RUN.lock().unwrap_or_else(|e| e.into_inner()).get(id).cloned() {
            if let Some(si) = l.lock().unwrap_or_else(|e| e.into_inner()).stdin.as_mut() { let _ = si.write_all(b"{\"command\":\"continue\"}\n"); let _ = si.flush(); }
        }
        update(id, |jj| { jj.waiting = false; if jj.state == "manual" { jj.state = "installing".into(); jj.phase = "Carrying on".into(); } });
    }
    if !running && all && j.state == "manual" {
        if !resume_auto(&j) { update(id, |jj| { jj.status = "All the files are here. Press Resume to carry on.".into(); }); }
        return false;
    }
    !all
}

fn fetch_google(id: &str, dir: &Path, m: Manual) {
    let (id, dir) = (id.to_string(), dir.to_path_buf());
    std::thread::spawn(move || {
        let dest = dir.join(&m.file_name);
        let r = crate::accounts::google_download(&m.gdrive, &dest).and_then(|_| match crate::guided::check(&dest, &want_of(&m)) {
            crate::guided::Check::Good => Ok(()),
            _ => { let _ = std::fs::remove_file(&dest); Err("Google Drive gave a different file than the modlist wants.".to_string()) }
        });
        update(&id, |jj| if let Some(x) = jj.manual.iter_mut().find(|x| x.file_name == m.file_name) {
            match &r { Ok(_) => { x.here = true; x.problem.clear(); } Err(e) => x.problem = format!("{e} Fetch it in the browser instead."), }
        });
    });
}

/// The file is in the list's downloads folder at its full size.
fn present(dir: &Path, m: &Manual) -> bool {
    if m.file_name.is_empty() { return false; }
    std::fs::metadata(dir.join(&m.file_name)).map(|md| md.is_file() && (m.size == 0 || md.len() == m.size)).unwrap_or(false)
}

// ------------------------------------------------------------------ the list's own archive list

/// An archive of the list, from the .wabbajack's "modlist" (its Archives).
#[derive(Debug, Clone, Default)]
pub struct ListArchive { pub name: String, pub size: u64, pub hash: Option<u64>, pub site: String, pub page: String, pub gdrive: String, pub browser_only: bool }

fn host_of(url: &str) -> String { url.split("://").nth(1).unwrap_or("").split(['/', '?', '#']).next().unwrap_or("").trim_start_matches("www.").to_string() }
fn site_of_url(url: &str) -> String {
    let h = host_of(url);
    match h.as_str() { "loverslab.com" | "api.loverslab.com" => "LoversLab".into(), "mega.nz" | "mega.co.nz" => "MEGA".into(), "drive.google.com" | "drive.usercontent.google.com" => "Google Drive".into(), _ => h }
}
/// A Google Drive file id from a drive address (…/file/d/<id>/…, ?id=<id>).
fn gdrive_id(url: &str) -> String {
    if !site_of_url(url).eq("Google Drive") { return String::new(); }
    let id = url.split("/file/d/").nth(1).map(|r| r.split(['/', '?']).next().unwrap_or("").to_string())
        .or_else(|| url.split(['?', '&']).find_map(|kv| kv.strip_prefix("id=")).map(|s| s.to_string())).unwrap_or_default();
    if id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') { id } else { String::new() }
}

/// What Hearth needs of one archive: its page for the browser, and whether only a browser can fetch it (LoversLab and
/// Vector Plexus: Invision sites whose files need a signed-in account; their page addresses as Wabbajack's own
/// ManualDownloadUrls makes them).
pub fn archive_of(a: &Value) -> ListArchive {
    let st = &a["State"];
    let ty = st["$type"].as_str().unwrap_or("");
    let url = st["Url"].as_str().or(st["URL"].as_str()).unwrap_or("").to_string();
    let ips4 = |base: &str| {
        let id = st["IPS4Mod"].as_i64().unwrap_or(0);
        if st["IsAttachment"] == true { format!("{base}/applications/core/interface/file/attachment.php?id={id}") } else { format!("{base}/files/file/{id}/") }
    };
    let (site, page, gdrive, browser_only) = if ty.contains("LoversLab") { ("LoversLab".to_string(), ips4("https://www.loverslab.com"), String::new(), true) }
        else if ty.contains("VectorPlexus") { ("Vector Plexus".to_string(), ips4("https://vectorplexis.com"), String::new(), true) }
        else if ty.contains("GoogleDrive") { let id = st["Id"].as_str().unwrap_or("").to_string(); ("Google Drive".to_string(), format!("https://drive.google.com/file/d/{id}/view"), id, false) }
        else if ty.contains("Nexus") || ty.contains("WabbajackCDN") || ty.contains("GameFile") || ty.contains("Bethesda") { (String::new(), String::new(), String::new(), false) }
        else if url.starts_with("https://") { (site_of_url(&url), url.clone(), gdrive_id(&url), false) }
        else { (String::new(), String::new(), String::new(), false) };
    ListArchive { name: a["Name"].as_str().unwrap_or("").to_string(), size: a["Size"].as_u64().unwrap_or(0), hash: a["Hash"].as_str().and_then(crate::guided::parse_hash), site, page, gdrive, browser_only }
}

/// The archives of a .wabbajack file (a zip with the list as JSON in "modlist").
pub fn read_modlist(path: &Path) -> Result<Vec<ListArchive>, String> {
    let f = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut z = zip::ZipArchive::new(f).map_err(|e| format!("not a .wabbajack file ({e})"))?;
    let mut text = String::new();
    std::io::Read::read_to_string(&mut z.by_name("modlist").map_err(|e| e.to_string())?, &mut text).map_err(|e| e.to_string())?;
    let v: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    Ok(v["Archives"].as_array().map(|a| a.iter().map(archive_of).collect()).unwrap_or_default())
}

/// Where jackify-engine keeps its data (~/Jackify, or jackify_data_dir in ~/.config/jackify/config.json).
fn jackify_data_dir() -> PathBuf {
    util::read_json(&util::home().join(".config/jackify/config.json"))["jackify_data_dir"].as_str().filter(|s| s.starts_with('/')).map(PathBuf::from)
        .unwrap_or_else(|| util::home().join("Jackify"))
}
/// The job's .wabbajack: the file chosen, or the one the engine downloaded for a gallery list.
pub fn list_file(list: &str) -> PathBuf {
    if list.ends_with(".wabbajack") { return PathBuf::from(list); }
    jackify_data_dir().join("downloaded_mod_lists").join(format!("{}.wabbajack", list.replace('/', "_@@_")))
}

fn add_list_item(id: &str, a: &ListArchive) -> bool {
    let mut added = false;
    update(id, |j| if !a.name.is_empty() && !j.manual.iter().any(|m| m.file_name == a.name) {
        j.manual.push(Manual { file_name: a.name.clone(), url: a.page.clone(), size: a.size, hash: a.hash.map(crate::guided::hash_text).unwrap_or_default(), site: a.site.clone(),
            mod_name: a.name.clone(), gdrive: a.gdrive.clone(), from_list: true, ..Default::default() });
        added = true;
    });
    added
}

/// Reads the list once its .wabbajack is there; files only a browser can fetch (LoversLab without a LoversLab
/// sign-in, Vector Plexus) are put in the guided flow straight away, while the engine fetches the rest.
fn scan_list(id: &str, archives: &mut Option<Vec<ListArchive>>) {
    if archives.is_some() { return; }
    let Some(j) = job(id) else { return };
    let f = list_file(&j.list);
    if !f.is_file() { return; }
    let Ok(list) = read_modlist(&f) else { return };
    let ll = crate::accounts::signed_in("loverslab");
    let mut any = false;
    for a in list.iter().filter(|a| a.browser_only && !(ll && a.site == "LoversLab") && !a.page.is_empty()) {
        if !present(&j.downloads_dir, &Manual { file_name: a.name.clone(), size: a.size, ..Default::default() }) { any |= add_list_item(id, a); }
    }
    *archives = Some(list);
    if any { watch(id); }
}

/// "Failed to download 'name'…": a file the engine couldn't get goes in the guided flow when it has a page to fetch
/// it from (Nexus files have their own way: the engine's manual step).
fn failed_download(id: &str, line: &str, archives: &mut Option<Vec<ListArchive>>) {
    scan_list(id, archives);
    let Some(list) = archives.as_ref() else { return };
    if let Some(a) = list.iter().find(|a| !a.name.is_empty() && !a.page.is_empty() && line.contains(&format!("'{}'", a.name))) {
        if add_list_item(id, a) { watch(id); }
    }
}

// ------------------------------------------------------------------ the guided flow, for the page

/// The next file to fetch in the browser: the first one not here and not skipped, then the skipped ones again.
pub fn guide_next(list: &[Manual]) -> Option<usize> {
    let open = |skip_ok: bool| list.iter().position(|m| !m.nexus && !m.here && (skip_ok || !m.skipped));
    open(false).or_else(|| open(true))
}
/// Progress for the page: "3 of 12 LoversLab files" and the file to fetch now.
pub fn guide(list: &[Manual]) -> Value {
    let g: Vec<&Manual> = list.iter().filter(|m| !m.nexus).collect();
    if g.is_empty() { return Value::Null; }
    let (total, done) = (g.len(), g.iter().filter(|m| m.here).count());
    let sites: std::collections::BTreeSet<&str> = g.iter().map(|m| m.site.as_str()).collect();
    let site = if sites.len() == 1 { sites.iter().next().copied().unwrap_or("") } else { "" };
    let current = (done + 1).min(total);
    let next = guide_next(list);
    json!({"total": total, "done": done, "current": current, "site": site,
           "label": format!("{current} of {total} {}file{}", if site.is_empty() { String::new() } else { format!("{site} ") }, if total == 1 { "" } else { "s" }),
           "item": next.map(|i| &list[i]), "index": next})
}
/// Skips a file for now (it comes back after the others).
pub fn guide_skip(id: &str, file_name: &str) -> Option<Value> {
    update(id, |j| if let Some(m) = j.manual.iter_mut().find(|m| m.file_name == file_name) { m.skipped = true; }).map(|j| guide(&j.manual))
}
/// A file you chose yourself for one the list needs: checked, then moved into the list's downloads.
pub fn manual_file(id: &str, file_name: &str, path: &Path) -> Result<Value, String> {
    use crate::guided::Check;
    let j = job(id).ok_or("That install isn't in the list anymore.")?;
    let m = j.manual.iter().find(|m| m.file_name == file_name).ok_or("The modlist doesn't need that file anymore.")?;
    match crate::guided::check(path, &want_of(m)) {
        Check::Good => {}
        Check::Missing => return Err("That file isn't there.".into()),
        Check::WrongSize(n) => return Err(format!("That file is {n} bytes; the modlist wants {} bytes. It's probably another version.", m.size)),
        Check::WrongHash => return Err("That isn't the exact file the modlist wants (its hash differs). It's probably another version.".into()),
    }
    crate::mods::move_file(path, &j.downloads_dir.join(&m.file_name)).map_err(|e| format!("Couldn't move it ({e})"))?;
    let j = update(id, |jj| if let Some(x) = jj.manual.iter_mut().find(|x| x.file_name == file_name) { x.here = true; x.problem.clear(); }).ok_or("gone")?;
    watch(id);
    Ok(guide(&j.manual))
}

// ------------------------------------------------------------------ after the install

/// A Linux path as Wine's Z: drive in MO2's ini (backslashes doubled): /home/a/G → Z:\\home\\a\\G
pub fn z_path(p: &Path) -> String {
    let s = p.to_string_lossy().replace('/', "\\");
    format!("Z:{}", s.replace('\\', "\\\\"))
}

/// Points ModOrganizer.ini at the game and the downloads (every copy of those lines: MO2 reads the last one).
pub fn fix_ini(ini: &Path, game_dir: &Path, downloads: &Path) -> Result<(), String> {
    let text = std::fs::read_to_string(ini).map_err(|e| format!("Couldn't read ModOrganizer.ini ({e})"))?;
    let (gp, dl) = (format!("gamePath=@ByteArray({})", z_path(game_dir)), format!("download_directory={}", z_path(downloads)));
    let mut saw_gp = false;
    let mut out: Vec<String> = text.lines().map(|l| {
        let k = l.trim_start().to_ascii_lowercase();
        if k.starts_with("gamepath") && k.trim_start_matches("gamepath").trim_start().starts_with('=') { saw_gp = true; gp.clone() }
        else if k.starts_with("download_directory") && k.trim_start_matches("download_directory").trim_start().starts_with('=') { dl.clone() }
        else { l.to_string() }
    }).collect();
    if !saw_gp {
        let at = out.iter().position(|l| l.trim().eq_ignore_ascii_case("[General]")).map(|i| i + 1).unwrap_or(0);
        out.insert(at, gp);
    }
    if !out.iter().any(|l| l.starts_with("download_directory=")) {
        let at = out.iter().position(|l| l.trim().eq_ignore_ascii_case("[Settings]")).map(|i| i + 1);
        match at { Some(i) => out.insert(i, dl), None => { out.push("[Settings]".into()); out.push(dl); } }
    }
    let _ = std::fs::copy(ini, ini.with_extension("ini.hearth-backup"));
    util::write_atomic(ini, (out.join("\r\n") + "\r\n").as_bytes()).map_err(|e| format!("Couldn't write ModOrganizer.ini ({e})"))
}

fn prefix_of(id: &str) -> PathBuf { util::data_dir().join("prefixes").join(format!("wabbajack-{id}")) }

/// Mod Organizer, its prefix and the library entry.
fn set_up(j: &Job) -> Result<Vec<String>, String> {
    let mut notes = Vec::new();
    let ini = j.install_dir.join("ModOrganizer.ini");
    let mo2 = j.install_dir.join("ModOrganizer.exe");
    if !mo2.is_file() { return Err("this list has no ModOrganizer.exe (lists that don't use Mod Organizer 2 aren't supported yet)".into()); }
    if ini.is_file() { fix_ini(&ini, &j.game_dir, &j.downloads_dir)?; }
    let info = game_info(&j.game);
    let gameid = info.map(|g| format!("umu-{}", g.1)).unwrap_or_else(|| "umu-default".into());
    let prefix = prefix_of(&j.id);
    std::fs::create_dir_all(&prefix).map_err(|e| e.to_string())?;
    // The folders MO2's virtual file system expects to find in the prefix.
    if let Some((_, _, _, folder)) = info {
        let user = prefix.join("pfx/drive_c/users/steamuser");
        let _ = std::fs::create_dir_all(user.join("AppData/Local").join(folder));
        let _ = std::fs::create_dir_all(user.join("Documents/My Games").join(folder));
        let plugins = user.join("AppData/Local").join(folder).join("Plugins.txt");
        if !plugins.exists() { let _ = std::fs::write(&plugins, ""); }
    }
    // The Windows parts these lists need, once per prefix.
    let verbs: Vec<&str> = WT_BASE.iter().copied().chain(match info.map(|g| g.2) { Some("legacy") => WT_LEGACY.to_vec(), _ => WT_MODERN.to_vec() }).collect();
    let mark = prefix.join("hearth-winetricks.txt");
    if std::fs::read_to_string(&mark).map(|s| s.trim() != verbs.join(" ")).unwrap_or(true) {
        if has("umu-run") {
            let st = std::process::Command::new("umu-run").arg("winetricks").arg("-q").args(&verbs)
                .env("WINEPREFIX", prefix.join("pfx")).env("GAMEID", &gameid).env("PROTONPATH", "GE-Proton")
                .stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).status();
            if st.map(|s| s.success()).unwrap_or(false) { let _ = std::fs::write(&mark, verbs.join(" ")); }
            else { notes.push(format!("Some Windows parts didn't install into its Proton folder ({}); if Mod Organizer won't start, run Hearth Mods' Repair on it.", verbs.join(", "))); }
        } else { notes.push("umu-launcher isn't installed, so its Proton folder couldn't be set up.".into()); }
    }
    // The library entry: a .desktop file that starts MO2 with this prefix.
    std::fs::create_dir_all(modlists_dir()).map_err(|e| e.to_string())?;
    let desktop = modlists_dir().join(format!("{}.desktop", j.id));
    let q = |p: &Path| format!("\"{}\"", p.to_string_lossy().replace('\\', "\\\\").replace('"', "\\\""));
    let text = format!("[Desktop Entry]\nType=Application\nName={}\nExec=env WINEPREFIX={} STEAM_COMPAT_DATA_PATH={} GAMEID={gameid} PROTONPATH=GE-Proton umu-run {}\nPath={}\n",
        j.title.replace('\n', " "), q(&prefix.join("pfx")), q(&prefix), q(&mo2), j.install_dir.to_string_lossy());
    util::write_atomic(&desktop, text.as_bytes()).map_err(|e| e.to_string())?;
    util::write_json(&modlists_dir().join(format!("{}.json", j.id)), &json!({"id": j.id, "title": j.title, "launch": desktop, "folder": j.install_dir, "game": j.game,
        "added": util::now_ms(), "note": "Wabbajack modlist (Mod Organizer 2)"}))?;
    Ok(notes)
}

fn has(cmd: &str) -> bool { std::env::var_os("PATH").map(|p| std::env::split_paths(&p).any(|d| d.join(cmd).is_file())).unwrap_or(false) }

/// Sets the list's Proton folder up again (after a failed winetricks run).
pub fn repair(id: &str) -> Result<Vec<String>, String> {
    let j = job(id).ok_or("That modlist isn't in the list anymore.")?;
    let _ = std::fs::remove_file(prefix_of(id).join("hearth-winetricks.txt"));
    set_up(&j)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_lines() {
        assert_eq!(parse_progress("[00:00:10] Downloading Mod Archives (17/214) - 6.8MB/s"), Some(("Downloading Mod Archives".into(), Some((17, 214)))));
        assert_eq!(parse_progress("Installing files (1.1GB/56.3GB)"), Some(("Installing files".into(), None)));
        assert_eq!(parse_progress("nothing here"), None);
        assert_eq!(step_of("[12/14] Installing files (1.1GB/56.3GB)"), Some((12, 14)));
        assert_eq!(step_of("[00:00:10] x"), None);
    }

    #[test]
    fn manual_events() {
        let ev = json!({"event": "manual_download_required", "nexus_url": "https://www.nexusmods.com/skyrimspecialedition/mods/266?tab=files&file_id=1000", "file_name": "SKSE.7z", "expected_size": 123, "mod_name": "SKSE"});
        let m = manual_of(&ev);
        assert!(m.nexus);
        assert_eq!((m.domain.as_str(), m.mod_id, m.file_id, m.size), ("skyrimspecialedition", 266, 1000, 123));
        let other = manual_of(&json!({"event": "manual_download_required", "manual_url": "https://www.moddb.com/x", "file_name": "x.zip"}));
        assert!(!other.nexus);
    }

    #[test]
    fn mo2_paths() {
        assert_eq!(z_path(Path::new("/home/a/Skyrim Special Edition")), "Z:\\\\home\\\\a\\\\Skyrim Special Edition");
        let t = tempfile::tempdir().unwrap();
        let ini = t.path().join("ModOrganizer.ini");
        std::fs::write(&ini, "[General]\r\ngamePath=@ByteArray(C:\\\\Games\\\\Skyrim)\r\ngameName=Skyrim Special Edition\r\n[Settings]\r\ndownload_directory=C:\\\\dl\r\n[General]\r\ngamePath = @ByteArray(D:\\\\x)\r\n").unwrap();
        fix_ini(&ini, Path::new("/g/Skyrim"), Path::new("/d/dl")).unwrap();
        let s = std::fs::read_to_string(&ini).unwrap();
        assert_eq!(s.matches("gamePath=@ByteArray(Z:\\\\g\\\\Skyrim)").count(), 2, "{s}");
        assert!(s.contains("download_directory=Z:\\\\d\\\\dl"));
        assert!(s.contains("gameName=Skyrim Special Edition"));
        assert!(ini.with_extension("ini.hearth-backup").is_file());
    }

    #[test]
    fn files_from_a_browser_are_picked_up() {
        let t = tempfile::tempdir().unwrap();
        let dl = t.path().join("dl");
        std::fs::create_dir_all(&dl).unwrap();
        let m = Manual { file_name: "a.zip".into(), size: 3, ..Default::default() };
        assert!(!present(&dl, &m));
        std::fs::write(dl.join("a.zip"), "abc").unwrap();
        assert!(present(&dl, &m));
        let wrong = Manual { file_name: "a.zip".into(), size: 4, ..Default::default() };
        assert!(!present(&dl, &wrong));
    }

    #[test]
    fn manual_events_carry_the_hash() {
        let h = crate::guided::xxh64(b"hello");
        let hex: String = h.to_le_bytes().iter().map(|b| format!("{b:02x}")).collect();
        let m = manual_of(&json!({"event": "manual_download_required", "nexus_url": "https://drive.google.com/file/d/AbC_12-x/view", "file_name": "x.7z", "expected_hash": hex, "expected_size": 5}));
        assert_eq!(crate::guided::hash_of_text(&m.hash), Some(h));
        assert_eq!((m.site.as_str(), m.gdrive.as_str(), m.nexus), ("Google Drive", "AbC_12-x", false));
    }

    #[test]
    fn archives_of_a_list() {
        let ll = archive_of(&json!({"Name": "Body.7z", "Size": 10, "Hash": "menYUTfbRu8=", "State": {"$type": "LoversLabOAuthDownloader, Wabbajack.Lib", "IPS4Mod": 1234, "IPS4File": "Body.7z"}}));
        assert!(ll.browser_only);
        assert_eq!((ll.site.as_str(), ll.page.as_str()), ("LoversLab", "https://www.loverslab.com/files/file/1234/"));
        assert_eq!(ll.hash, Some(0xEF46DB3751D8E999), "the .wabbajack's base64 is the little-endian bytes");
        let att = archive_of(&json!({"Name": "a.zip", "State": {"$type": "LoversLab", "IPS4Mod": 9, "IsAttachment": true}}));
        assert_eq!(att.page, "https://www.loverslab.com/applications/core/interface/file/attachment.php?id=9");
        let vp = archive_of(&json!({"Name": "v.zip", "State": {"$type": "VectorPlexusOAuthDownloader+State, Wabbajack.Lib", "IPS4Mod": 5}}));
        assert!(vp.browser_only && vp.page == "https://vectorplexis.com/files/file/5/");
        let gd = archive_of(&json!({"Name": "g.zip", "State": {"$type": "GoogleDriveDownloader, Wabbajack.Lib", "Id": "1abc"}}));
        assert!(!gd.browser_only && gd.gdrive == "1abc" && gd.page.contains("/file/d/1abc/"));
        let mega = archive_of(&json!({"Name": "m.zip", "State": {"$type": "MegaDownloader, Wabbajack.Lib", "Url": "https://mega.nz/file/x#y"}}));
        assert_eq!(mega.site, "MEGA");
        let cdn = archive_of(&json!({"Name": "c.zip", "State": {"$type": "WabbajackCDNDownloader+State, Wabbajack.Lib", "Url": "https://authored-files.wabbajack.org/c"}}));
        assert!(cdn.page.is_empty(), "nothing a browser can hand back");
        assert_eq!(list_file("repo/List Name").file_name().unwrap().to_string_lossy(), "repo_@@_List Name.wabbajack");
        // A .wabbajack is a zip with the list in "modlist".
        let t = tempfile::tempdir().unwrap();
        let f = t.path().join("l.wabbajack");
        let mut z = zip::ZipWriter::new(std::fs::File::create(&f).unwrap());
        z.start_file("modlist", zip::write::SimpleFileOptions::default()).unwrap();
        z.write_all(json!({"Archives": [{"Name": "Body.7z", "Size": 10, "State": {"$type": "LoversLab", "IPS4Mod": 1}}]}).to_string().as_bytes()).unwrap();
        z.finish().unwrap();
        let l = read_modlist(&f).unwrap();
        assert_eq!(l.len(), 1);
        assert!(l[0].browser_only);
    }

    #[test]
    fn the_guided_queue_moves_on() {
        let m = |n: &str, here: bool| Manual { file_name: n.into(), site: "LoversLab".into(), here, ..Default::default() };
        let mut list = vec![m("a", true), m("b", false), m("c", false), Manual { nexus: true, ..m("n", false) }];
        let g = guide(&list);
        assert_eq!(g["label"], "2 of 3 LoversLab files");
        assert_eq!(g["item"]["fileName"], "b");
        list[1].skipped = true;
        assert_eq!(guide_next(&list), Some(2));
        list[2].here = true;
        assert_eq!(guide_next(&list), Some(1), "skipped files come back when the rest are done");
        list[1].here = true;
        let g = guide(&list);
        assert!(g["item"].is_null());
        assert_eq!(g["label"], "3 of 3 LoversLab files");
        list.push(Manual { file_name: "z".into(), site: "MEGA".into(), ..Default::default() });
        assert_eq!(guide(&list)["label"], "4 of 4 files");
    }

    #[test]
    fn a_waiting_install_picks_up_the_browsers_file() {
        let t = tempfile::tempdir().unwrap();
        std::env::set_var("HOME", t.path());
        std::env::set_var("XDG_DATA_HOME", t.path().join("data"));
        std::env::set_var("XDG_CONFIG_HOME", t.path().join("config"));
        let (dl, list_dl) = (t.path().join("Downloads"), t.path().join("list-downloads"));
        std::fs::create_dir_all(&dl).unwrap();
        std::fs::create_dir_all(&list_dl).unwrap();
        let want = |n: &str, body: &[u8]| Manual { file_name: n.into(), size: body.len() as u64, hash: crate::guided::hash_text(crate::guided::xxh64(body)), site: "LoversLab".into(), from_list: true, ..Default::default() };
        let jb = Job { id: "wtest".into(), state: "manual".into(), downloads_dir: list_dl.clone(), manual: vec![want("one.7z", b"first"), want("two.7z", b"second")], ..Default::default() };
        JOBS.lock().unwrap().insert(jb.id.clone(), jb);
        assert!(tick("wtest"), "keeps looking while files are missing");
        std::fs::write(dl.join("one.7z"), b"first").unwrap();
        std::fs::write(dl.join("two.7z"), b"wrong!").unwrap();
        assert!(tick("wtest"));
        let j = job("wtest").unwrap();
        assert!(j.manual[0].here && list_dl.join("one.7z").is_file());
        assert!(!j.manual[1].here && !j.manual[1].problem.is_empty(), "a wrong file is reported, not taken");
        assert_eq!(guide(&j.manual)["label"], "2 of 2 LoversLab files");
        std::fs::write(dl.join("two (1).7z"), b"second").unwrap();
        // All here and no engine running: Hearth would start it again (no engine in the test, so it asks for Resume).
        assert!(!tick("wtest"));
        let j = job("wtest").unwrap();
        assert!(j.manual.iter().all(|m| m.here) && list_dl.join("two.7z").is_file());
        assert!(j.status.contains("Resume"));
        JOBS.lock().unwrap().remove("wtest");
    }

    #[test]
    fn games_known() {
        assert_eq!(game_info("falloutnewvegas").map(|g| g.1), Some(22380));
        assert_eq!(game_info("SkyrimSpecialEdition").map(|g| g.2), Some("modern"));
        assert!(game_info("Nope").is_none());
    }
}
