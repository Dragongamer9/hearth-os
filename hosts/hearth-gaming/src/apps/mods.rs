// Hearth Mods' window (Tauri: WebKitGTK on Linux). The page (ui/mods.html) shows each game's mods in load order
// and installs, turns on and off, reorders and removes them; the work is in the library (mods.rs, games.rs).
// Beacon's games (Battle for Middle-earth...) are Beacon's: they're shown with a button that opens Beacon.
//
//   hearth-mods [--game <id>]                       the window (--fullscreen in Game Mode), opened on one game
//   hearth-mods --list                              the games, as JSON
//   hearth-mods --mods <game>                       one game's mods, as JSON
//   hearth-mods --install <game> <zip|folder> [--name N] [--target game] [--sub Folder] [--off]
//   hearth-mods --enable|--disable|--remove <game> <mod>
//   hearth-mods --move <game> <mod> <-1|1>
//   hearth-mods --deploy <game> <hardlink|copy|overlay>  how the game's mods are put in place
//   hearth-mods --nxm <nxm://...>                   the site's "Mod Manager Download" (the nxm:// handler): queued in
//                                                   the open window, or the window opens on Downloads
//   hearth-mods --nexus-key <key> | --downloads     sign in to Nexus Mods | the download queue, as JSON
//   hearth-mods --remount                           puts the overlays back (at sign-in)
//   hearth-mods --modlists | --wabbajack-engine     the Wabbajack gallery | download the Wabbajack engine
//
// Nexus Mods and Wabbajack: nexus.rs, downloads.rs and wabbajack.rs. nxm:// links that arrive while the window is
// open are dropped in mods/nxm-inbox/ by the handler and picked up here within a second.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use hearth_gaming::downloads;
use hearth_gaming::games::{self, GameEntry};
use hearth_gaming::nexus;
use hearth_gaming::wabbajack;
use hearth_gaming::mods::{self, GameMods, Staged};
use hearth_gaming::util;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

static GAMES: Mutex<Vec<GameEntry>> = Mutex::new(Vec::new());
static STAGED: Mutex<Option<HashMap<String, (String, Staged)>>> = Mutex::new(None);
/// One change at a time (two at once could both move the same original).
static WORK: Mutex<()> = Mutex::new(());

fn data_root() -> PathBuf { util::data_dir() }
fn inbox() -> PathBuf { data_root().join("mods").join("nxm-inbox") }
fn pid_file() -> PathBuf { data_root().join("mods").join("hearth-mods.pid") }
/// What happened to nxm links taken from the inbox (shown by the page, then cleared).
static NOTICES: Mutex<Vec<Value>> = Mutex::new(Vec::new());

/// Is a Hearth Mods window open (its pid file names a live hearth-mods)?
fn window_open() -> bool {
    let Ok(t) = std::fs::read_to_string(pid_file()) else { return false };
    let Ok(pid) = t.trim().parse::<u32>() else { return false };
    std::fs::read(format!("/proc/{pid}/cmdline")).map(|c| String::from_utf8_lossy(&c).contains("hearth-mods")).unwrap_or(false)
}

/// Takes nxm links out of the inbox and queues them.
fn drain_inbox() {
    let Ok(rd) = std::fs::read_dir(inbox()) else { return };
    let mut files: Vec<PathBuf> = rd.flatten().map(|e| e.path()).filter(|p| p.extension().map(|x| x == "nxm").unwrap_or(false)).collect();
    files.sort();
    for f in files {
        let link = std::fs::read_to_string(&f).unwrap_or_default();
        let _ = std::fs::remove_file(&f);
        let n = match downloads::add_nxm(link.trim(), downloads::Want::default()) {
            Ok(i) => json!({"ok": true, "text": format!("Downloading {}", i.name)}),
            Err(e) => json!({"ok": false, "text": e}),
        };
        NOTICES.lock().unwrap_or_else(|e| e.into_inner()).push(n);
    }
}

/// The Hearth game a Nexus download is for: the one the Nexus game matches.
fn game_for_domain(domain: &str) -> Option<GameEntry> {
    let list = GAMES.lock().unwrap_or_else(|e| e.into_inner()).clone();
    let key = nexus::account().map(|a| a.key).unwrap_or_default();
    let all = if key.is_empty() { Value::Array(vec![]) } else { nexus::games(&key) };
    list.into_iter().find(|g| {
        let gm = GameMods::open(&data_root(), g);
        if !gm.state.nexus.is_empty() { return gm.state.nexus == domain; }
        nexus::domain_for(g, &all).as_deref() == Some(domain)
    })
}

fn load_games() -> Vec<GameEntry> {
    let list: Vec<GameEntry> = games::list().into_iter().filter(|g| g.kind == "game" && g.install.as_ref().map(|p| p.is_dir()).unwrap_or(false)).collect();
    *GAMES.lock().unwrap_or_else(|e| e.into_inner()) = list.clone();
    list
}
fn game(id: &str) -> Result<GameEntry, String> {
    let found = GAMES.lock().unwrap_or_else(|e| e.into_inner()).iter().find(|g| g.id == id).cloned();
    match found { Some(g) => Ok(g), None => load_games().into_iter().find(|g| g.id == id).ok_or_else(|| "That game isn't installed anymore.".to_string()) }
}

fn games_json(list: &[GameEntry]) -> Value {
    Value::Array(list.iter().map(|g| {
        let gm = GameMods::open(&data_root(), g);
        let mut v = serde_json::to_value(g).unwrap_or(Value::Null);
        v["mods"] = json!(gm.state.mods.len());
        v["modsOn"] = json!(gm.state.mods.iter().filter(|m| m.enabled).count());
        v
    }).collect())
}

fn summary(g: &GameEntry) -> Value {
    let mut s = GameMods::open(&data_root(), g).summary(g);
    s["beacon"] = json!(g.beacon);
    if s["nexus"].as_str().map(|d| d.is_empty()).unwrap_or(true) {
        if let Some(d) = nexus::STEAM_DOMAINS.iter().find(|x| Some(x.0) == g.steam_app) { s["nexus"] = json!(d.1); }
    }
    s
}

fn change(id: &str, f: impl FnOnce(&mut GameMods, &GameEntry) -> Result<mods::Report, String>) -> Result<Value, String> {
    let _w = WORK.lock().unwrap_or_else(|e| e.into_inner());
    let g = game(id)?;
    if g.beacon { return Err("Beacon manages this game's mods. Open Beacon to change them.".into()); }
    let mut gm = GameMods::open(&data_root(), &g);
    let r = f(&mut gm, &g)?;
    Ok(json!({"report": r, "summary": summary(&g)}))
}

// ------------------------------------------------------------------ the page's calls

#[tauri::command]
fn info() -> Value {
    let args: Vec<String> = std::env::args().collect();
    json!({"version": env!("CARGO_PKG_VERSION"), "game": util::arg_value(&args, "--game"), "page": util::arg_value(&args, "--page"), "targets": mods::TARGETS.iter().map(|t| json!({"id": t.0, "label": t.1, "prefix": t.2})).collect::<Vec<_>>()})
}
#[tauri::command]
async fn list() -> Value { tauri::async_runtime::spawn_blocking(|| games_json(&load_games())).await.unwrap_or(Value::Null) }
#[tauri::command]
fn mods_of(id: String) -> Result<Value, String> { game(&id).map(|g| summary(&g)) }
#[tauri::command]
async fn stage(id: String, path: String) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let g = game(&id)?;
        if g.beacon { return Err("Beacon manages this game's mods. Open Beacon to add one.".to_string()); }
        let _w = WORK.lock().unwrap_or_else(|e| e.into_inner());
        let s = mods::stage(&data_root(), &g, Path::new(&path))?;
        let v = serde_json::to_value(&s).unwrap_or(Value::Null);
        STAGED.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(HashMap::new).insert(s.token.clone(), (id, s));
        Ok(v)
    }).await.map_err(|e| e.to_string())?
}
#[tauri::command]
async fn install(id: String, token: String, name: String, target: String, sub: String, enable: bool) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let staged = STAGED.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(HashMap::new).remove(&token);
        let Some((gid, s)) = staged.filter(|(g, _)| g == &id) else { return Err("Choose the mod again (it was put away).".to_string()) };
        change(&gid, |gm, g| gm.install(g, &s, &name, &target, &sub, enable))
    }).await.map_err(|e| e.to_string())?
}
#[tauri::command]
fn cancel_stage(token: String) {
    if let Some((_, s)) = STAGED.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(HashMap::new).remove(&token) { let _ = std::fs::remove_dir_all(&s.dir); }
}
#[tauri::command]
async fn toggle(id: String, m: String, on: bool) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || change(&id, |gm, g| gm.set_enabled(g, &m, on))).await.map_err(|e| e.to_string())?
}
#[tauri::command]
async fn move_mod(id: String, m: String, by: i64) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || change(&id, |gm, g| gm.move_by(g, &m, by))).await.map_err(|e| e.to_string())?
}
#[tauri::command]
async fn remove_mod(id: String, m: String) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || change(&id, |gm, g| gm.remove(g, &m))).await.map_err(|e| e.to_string())?
}
#[tauri::command]
async fn set_deploy(id: String, mode: String) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || change(&id, |gm, g| gm.set_mode(g, &mode))).await.map_err(|e| e.to_string())?
}
#[tauri::command]
fn set_nexus_game(id: String, domain: String) -> Result<Value, String> {
    let g = game(&id)?;
    let mut gm = GameMods::open(&data_root(), &g);
    gm.set_nexus_game(&domain)?;
    Ok(summary(&g))
}

// Nexus Mods
#[tauri::command]
fn nexus_account() -> Value { nexus::account().map(|a| a.public()).unwrap_or(Value::Null) }
#[tauri::command]
async fn nexus_sign_in(key: String) -> Result<Value, String> { tauri::async_runtime::spawn_blocking(move || nexus::sign_in(&key).map(|a| a.public())).await.map_err(|e| e.to_string())? }
#[tauri::command]
async fn nexus_refresh() -> Result<Value, String> { tauri::async_runtime::spawn_blocking(|| nexus::refresh().map(|a| a.public())).await.map_err(|e| e.to_string())? }
#[tauri::command]
fn nexus_sign_out() { nexus::sign_out(); }
#[tauri::command]
fn nexus_pages() -> Value { json!({"keys": nexus::KEY_PAGE, "site": nexus::SITE}) }
/// A mod's files, for Premium accounts downloading from inside Hearth: from a mod page's address or game + id.
#[tauri::command]
async fn nexus_files(url: String) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let acct = nexus::account().ok_or("Sign in to Nexus Mods first.")?;
        let rest = url.trim().split("nexusmods.com/").nth(1).unwrap_or(url.trim());
        let parts: Vec<&str> = rest.split(['?', '#']).next().unwrap_or("").split('/').collect();
        if parts.len() < 3 || parts[1] != "mods" { return Err("Paste a mod's address from Nexus Mods (nexusmods.com/<game>/mods/<number>).".to_string()); }
        let (domain, mod_id) = (parts[0].to_ascii_lowercase(), parts[2].parse::<u64>().map_err(|_| "That address has no mod number.".to_string())?);
        let info = nexus::mod_info(&acct.key, &domain, mod_id)?;
        let files = nexus::files(&acct.key, &domain, mod_id)?;
        let list: Vec<Value> = files["files"].as_array().cloned().unwrap_or_default().into_iter().rev()
            .filter(|f| f["category_name"].as_str().map(|c| c != "ARCHIVED" && c != "DELETED").unwrap_or(true))
            .map(|f| json!({"fileId": f["file_id"], "name": f["name"], "version": f["version"], "category": f["category_name"], "size": f["size_in_bytes"].as_u64().or_else(|| f["size_kb"].as_u64().map(|k| k * 1024)), "fileName": f["file_name"], "primary": f["is_primary"]}))
            .collect();
        Ok(json!({"domain": domain, "modId": mod_id, "name": info["name"], "summary": info["summary"], "author": info["author"], "version": info["version"], "picture": info["picture_url"],
                  "page": nexus::files_page(&domain, mod_id, None), "premium": acct.premium, "files": list, "adult": info["contains_adult_content"]}))
    }).await.map_err(|e| e.to_string())?
}

// Downloads
#[tauri::command]
fn downloads_list() -> Value {
    let mut v = downloads::list();
    v["notices"] = json!(std::mem::take(&mut *NOTICES.lock().unwrap_or_else(|e| e.into_inner())));
    v
}
#[tauri::command]
async fn download_nxm(link: String) -> Result<Value, String> { tauri::async_runtime::spawn_blocking(move || downloads::add_nxm(&link, downloads::Want::default()).map(|i| i.public())).await.map_err(|e| e.to_string())? }
#[tauri::command]
async fn download_file(domain: String, mod_id: u64, file_id: u64, game: String) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || downloads::add_file(&domain, mod_id, file_id, downloads::Want { game, ..Default::default() }).map(|i| i.public())).await.map_err(|e| e.to_string())?
}
#[tauri::command]
fn download_pause(id: String) { downloads::pause(&id); }
#[tauri::command]
fn download_resume(id: String) -> Result<(), String> { downloads::resume(&id) }
#[tauri::command]
fn download_remove(id: String, delete_file: bool) { downloads::remove(&id, delete_file); }
#[tauri::command]
fn downloads_clear() { downloads::clear_finished(); }
/// A finished download, unpacked and waiting for "Install" into a game (the game it was for, or the one asked).
#[tauri::command]
async fn stage_download(id: String, game_id: String) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let d = downloads::get(&id).ok_or("That download isn't in the list anymore.")?;
        if d.state != "done" { return Err("It hasn't finished downloading.".to_string()); }
        let g = if !game_id.is_empty() { game(&game_id)? } else if !d.game.is_empty() { game(&d.game)? } else {
            if GAMES.lock().unwrap_or_else(|e| e.into_inner()).is_empty() { load_games(); }
            game_for_domain(&d.domain).ok_or("Choose which game it's for.")?
        };
        if g.beacon { return Err("Beacon manages this game's mods. Open Beacon to add one.".to_string()); }
        let _w = WORK.lock().unwrap_or_else(|e| e.into_inner());
        let mut s = mods::stage(&data_root(), &g, &d.path())?;
        s.name = d.name.clone();
        s.nexus = Some(mods::NexusRef { domain: d.domain.clone(), mod_id: d.mod_id, file_id: d.file_id, version: String::new() });
        let mut v = serde_json::to_value(&s).unwrap_or(Value::Null);
        v["game"] = json!(g.id);
        STAGED.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(HashMap::new).insert(s.token.clone(), (g.id.clone(), s));
        Ok(v)
    }).await.map_err(|e| e.to_string())?
}

// Wabbajack
#[tauri::command]
fn wj_engine() -> Value { wabbajack::engine_info() }
#[tauri::command]
async fn wj_fetch_engine() -> Result<Value, String> { tauri::async_runtime::spawn_blocking(wabbajack::fetch_engine).await.map_err(|e| e.to_string())? }
#[tauri::command]
async fn wj_gallery(refresh: bool) -> Result<Value, String> { tauri::async_runtime::spawn_blocking(move || wabbajack::gallery(refresh)).await.map_err(|e| e.to_string())? }
#[tauri::command]
fn wj_jobs() -> Value { wabbajack::jobs() }
/// Where a list would go, and the game's folder from the library.
#[tauri::command]
async fn wj_defaults(title: String, game: String) -> Value {
    tauri::async_runtime::spawn_blocking(move || {
        let info = wabbajack::game_info(&game);
        let list = if GAMES.lock().unwrap_or_else(|e| e.into_inner()).is_empty() { load_games() } else { GAMES.lock().unwrap_or_else(|e| e.into_inner()).clone() };
        let found = info.and_then(|i| list.iter().find(|g| g.steam_app == Some(i.1) || (i.1 == 22380 && g.steam_app == Some(22490))).cloned());
        let base = util::home().join("Modlists");
        let dir = base.join(util::safe_name(&title));
        json!({"gameDir": found.as_ref().and_then(|g| g.install.clone()), "gameTitle": found.as_ref().map(|g| g.title.clone()), "known": info.is_some(),
               "installDir": dir, "downloadsDir": base.join("Downloads").join(util::safe_name(&title)), "free": wabbajack::free_space(&dir)})
    }).await.unwrap_or(Value::Null)
}
#[tauri::command]
fn wj_start(title: String, list: String, game: String, game_dir: String, install_dir: String, downloads_dir: String) -> Result<Value, String> {
    wabbajack::start(&title, &list, &game, Path::new(&game_dir), Path::new(&install_dir), Path::new(&downloads_dir)).map(|j| j.public())
}
#[tauri::command]
fn wj_resume(id: String) -> Result<Value, String> { wabbajack::resume(&id).map(|j| j.public()) }
#[tauri::command]
fn wj_cancel(id: String) { wabbajack::cancel(&id); }
#[tauri::command]
fn wj_forget(id: String, delete_files: bool, delete_downloads: bool) -> Result<(), String> { wabbajack::forget(&id, delete_files, delete_downloads) }
#[tauri::command]
async fn wj_repair(id: String) -> Result<Value, String> { tauri::async_runtime::spawn_blocking(move || wabbajack::repair(&id).map(|n| json!(n))).await.map_err(|e| e.to_string())? }
#[tauri::command]
fn wj_log(id: String) -> Result<String, String> {
    let j = wabbajack::job(&id).ok_or("That install isn't in the list anymore.")?;
    let t = std::fs::read_to_string(j.log()).unwrap_or_default();
    let lines: Vec<&str> = t.lines().collect();
    Ok(lines[lines.len().saturating_sub(400)..].join("\n"))
}
#[tauri::command]
fn free_space(path: String) -> Value { json!(wabbajack::free_space(Path::new(&path))) }

/// Opens a web page (Hearth Browser, the default browser on Hearth OS). Only Nexus Mods, Wabbajack and GitHub pages,
/// and the addresses Wabbajack lists give for their manual downloads.
#[tauri::command]
fn open_url(url: String) -> Result<(), String> {
    if !url.starts_with("https://") { return Err("Only https pages open from here.".into()); }
    std::process::Command::new("xdg-open").arg(&url).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null())
        .spawn().map(|mut c| { std::thread::spawn(move || { let _ = c.wait(); }); }).map_err(|e| format!("Couldn't open the page ({e})"))
}

#[tauri::command]
fn open_folder(path: String) -> Result<(), String> { util::open_folder(Path::new(&path)) }
#[tauri::command]
fn open_beacon() -> Result<(), String> { games::open_beacon() }
#[tauri::command]
fn places() -> Value { hearth_gaming::browse::places() }
#[tauri::command]
fn browse(path: String, files: bool, archives: bool) -> Result<Value, String> { hearth_gaming::browse::list(Path::new(&path), files, archives, false) }
#[tauri::command]
fn target_root(id: String, target: String) -> Result<Value, String> {
    let g = game(&id)?;
    mods::target_root(&g, &target).map(|p| json!(p)).ok_or_else(|| "This game has no folder for that.".to_string())
}

// ------------------------------------------------------------------ without a window

fn cli(args: &[String]) -> Option<i32> {
    let print = |v: &Value| println!("{}", serde_json::to_string_pretty(v).unwrap_or_default());
    let pos = |flag: &str, n: usize| -> Option<Vec<String>> { let i = args.iter().position(|a| a == flag)?; let v: Vec<String> = args.iter().skip(i + 1).take(n).cloned().collect(); if v.len() == n { Some(v) } else { None } };
    let done = |r: Result<Value, String>| -> i32 { match r { Ok(v) => { print(&v); if v["report"]["problems"].as_array().map(|a| a.is_empty()).unwrap_or(true) { 0 } else { 2 } } Err(e) => { eprintln!("{e}"); 1 } } };
    if util::has_arg(args, "--list") { print(&games_json(&load_games())); return Some(0); }
    if let Some(v) = pos("--mods", 1) { load_games(); return Some(match game(&v[0]) { Ok(g) => { print(&summary(&g)); 0 } Err(e) => { eprintln!("{e}"); 1 } }); }
    if let Some(v) = pos("--install", 2) {
        load_games();
        let target = util::arg_value(args, "--target").unwrap_or_else(|| "game".into());
        let name = util::arg_value(args, "--name").unwrap_or_default();
        let r = (|| -> Result<Value, String> {
            let g = game(&v[0])?;
            let s = mods::stage(&data_root(), &g, Path::new(&v[1]))?;
            let sub = util::arg_value(args, "--sub").unwrap_or_else(|| s.suggest_sub.clone());
            change(&v[0], |gm, g| gm.install(g, &s, &name, &target, &sub, !util::has_arg(args, "--off")))
        })();
        return Some(done(r));
    }
    for (flag, what) in [("--enable", 1), ("--disable", 0), ("--remove", 2)] {
        if let Some(v) = pos(flag, 2) {
            load_games();
            return Some(done(change(&v[0], |gm, g| match what { 1 => gm.set_enabled(g, &v[1], true), 0 => gm.set_enabled(g, &v[1], false), _ => gm.remove(g, &v[1]) })));
        }
    }
    if let Some(v) = pos("--nxm", 1) {
        // The nxm:// handler: into the inbox; the open window takes it, or a window opens on Downloads.
        let _ = std::fs::create_dir_all(inbox());
        let f = inbox().join(format!("{}-{}.nxm", util::now_ms(), std::process::id()));
        if let Err(e) = std::fs::write(&f, &v[0]) { eprintln!("{e}"); return Some(1); }
        if !window_open() {
            let me = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("hearth-mods"));
            let _ = std::process::Command::new(me).arg("--page").arg("downloads").stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn();
        }
        return Some(0);
    }
    if let Some(v) = pos("--nexus-key", 1) { return Some(match nexus::sign_in(&v[0]) { Ok(a) => { print(&a.public()); 0 } Err(e) => { eprintln!("{e}"); 1 } }); }
    if util::has_arg(args, "--downloads") { print(&downloads::list()); return Some(0); }
    if util::has_arg(args, "--modlists") { return Some(match wabbajack::gallery(true) { Ok(v) => { print(&v); 0 } Err(e) => { eprintln!("{e}"); 1 } }); }
    if util::has_arg(args, "--wabbajack-engine") { return Some(match wabbajack::fetch_engine() { Ok(v) => { print(&v); 0 } Err(e) => { eprintln!("{e}"); 1 } }); }
    if util::has_arg(args, "--remount") {
        let mut code = 0;
        for g in load_games() { if let Err(e) = GameMods::open(&data_root(), &g).remount(&g) { eprintln!("{}: {e}", g.title); code = 2; } }
        return Some(code);
    }
    if let Some(v) = pos("--deploy", 2) { load_games(); return Some(done(change(&v[0], |gm, g| gm.set_mode(g, &v[1])))); }
    if let Some(v) = pos("--move", 3) {
        load_games();
        let by: i64 = v[2].parse().unwrap_or(0);
        return Some(done(change(&v[0], |gm, g| gm.move_by(g, &v[1], by))));
    }
    None
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let Some(code) = cli(&args) { std::process::exit(code); }
    let fullscreen = util::has_arg(&args, "--fullscreen");
    // This window takes nxm links (and finishes the downloads queued before it closed).
    let _ = std::fs::create_dir_all(data_root().join("mods"));
    let _ = std::fs::write(pid_file(), std::process::id().to_string());
    std::thread::spawn(|| { downloads::kick(); loop { drain_inbox(); std::thread::sleep(std::time::Duration::from_millis(800)); } });
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() { std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1"); }
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![info, list, mods_of, stage, install, cancel_stage, toggle, move_mod, remove_mod, open_folder, open_beacon, places, browse, target_root,
            set_deploy, set_nexus_game, nexus_account, nexus_sign_in, nexus_refresh, nexus_sign_out, nexus_pages, nexus_files,
            downloads_list, download_nxm, download_file, download_pause, download_resume, download_remove, downloads_clear, stage_download,
            wj_engine, wj_fetch_engine, wj_gallery, wj_jobs, wj_defaults, wj_start, wj_resume, wj_cancel, wj_forget, wj_repair, wj_log, free_space, open_url])
        .setup(move |app| {
            tauri::WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::App("mods.html".into()))
                .title("Mods")
                .inner_size(1280.0, 800.0)
                .min_inner_size(640.0, 480.0)
                .fullscreen(fullscreen)
                .background_color(tauri::window::Color(15, 11, 9, 255))
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Mods couldn't start");
}
