// Hearth Saves' window (Tauri: WebKitGTK on Linux). The page (ui/saves.html) asks for the games and their saves,
// backs them up, restores and schedules; the work is in the library (saves.rs, locate.rs, manifest.rs, games.rs).
//
//   hearth-saves [--game <id>]               the window (--fullscreen in Game Mode), opened on one game
//   hearth-saves --auto                       the scheduled backup (the hourly timer runs this)
//   hearth-saves --list                       every game and its saves, as JSON
//   hearth-saves --backup <id|all> --to <dir> back up now
//   hearth-saves --backups <dir>              the backups in a folder, as JSON
//   hearth-saves --restore <zip> [--game <id>] [--yes]   restore (asks unless --yes)
//   hearth-saves --refresh-manifest           download the save-location list again
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use hearth_gaming::games::{self, GameEntry};
use hearth_gaming::manifest::{self, Manifest};
use hearth_gaming::saves::{self, Scan};
use hearth_gaming::util;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::Emitter;

#[derive(Default)]
struct Data {
    manifest: Option<Arc<Manifest>>,
    info: Value,
    games: Vec<GameEntry>,
    scans: HashMap<String, Scan>,
}
static DATA: Mutex<Option<Data>> = Mutex::new(None);
static BUSY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn with<T>(f: impl FnOnce(&mut Data) -> T) -> T {
    let mut g = DATA.lock().unwrap_or_else(|e| e.into_inner());
    f(g.get_or_insert_with(Data::default))
}

fn load_manifest(refresh: bool) -> (Option<Arc<Manifest>>, Value) {
    match manifest::load(refresh) {
        Ok((m, i)) => (Some(Arc::new(m)), json!({"ok": true, "games": i.games, "fetched": i.fetched, "source": i.source, "note": i.note})),
        Err(e) => (None, json!({"ok": false, "error": e})),
    }
}

fn game_json(g: &GameEntry, s: Option<&Scan>) -> Value {
    let mut v = serde_json::to_value(g).unwrap_or(Value::Null);
    if let Some(s) = s { v["saves"] = json!({"count": s.count, "size": s.size, "newest": s.newest, "knownAs": s.known_as, "folders": s.folders, "custom": s.custom}); }
    v
}

/// Reads everything: the games, the save-location list, and every game's saves.
fn load_all(refresh: bool, progress: &dyn Fn(&str)) -> Value {
    progress("Reading your library");
    let gl = std::thread::spawn(games::list);
    progress("Reading the list of save locations");
    let (m, info) = load_manifest(refresh);
    let list = gl.join().unwrap_or_default();
    progress("Looking for saves");
    let cfg = saves::load_config();
    let mut scans = HashMap::new();
    for g in &list { scans.insert(g.id.clone(), saves::scan(g, m.as_deref(), &saves::custom_dirs(&cfg, &g.id))); }
    let out = json!({
        "games": list.iter().map(|g| game_json(g, scans.get(&g.id))).collect::<Vec<_>>(),
        "manifest": info, "config": public_config(&cfg), "destinations": saves::destinations(&cfg),
        "timer": Path::new(saves::SYSTEM_TIMER).exists(),
    });
    with(|d| { d.manifest = m; d.info = info; d.games = list; d.scans = scans; });
    out
}

fn public_config(cfg: &Value) -> Value {
    json!({"schedule": cfg["schedule"], "dest": cfg["dest"], "keep": cfg["keep"], "lastAuto": cfg["lastAuto"], "lastAutoText": cfg["lastAutoText"], "games": cfg["games"]})
}

fn game(id: &str) -> Result<GameEntry, String> {
    with(|d| d.games.iter().find(|g| g.id == id).cloned()).ok_or_else(|| "That game isn't in the list anymore. Look again.".to_string())
}

fn rescan_one(id: &str) -> Result<Scan, String> {
    let g = game(id)?;
    let m = with(|d| d.manifest.clone());
    let s = saves::scan(&g, m.as_deref(), &saves::custom_dirs(&saves::load_config(), id));
    with(|d| { d.scans.insert(id.to_string(), s.clone()); });
    Ok(s)
}

fn back_up(ids: &[String], dest: &Path, progress: &dyn Fn(&str, Option<f64>)) -> Value {
    let (mut done, mut failed, mut none) = (Vec::new(), Vec::new(), 0);
    let total = ids.len().max(1);
    for (i, id) in ids.iter().enumerate() {
        let Ok(g) = game(id) else { continue };
        progress(&format!("Backing up {}", g.title), Some(i as f64 / total as f64));
        let s = match rescan_one(id) { Ok(s) => s, Err(e) => { failed.push(json!({"title": g.title, "error": e})); continue } };
        if s.files.is_empty() { none += 1; continue; }
        match saves::backup(&g, &s.files, dest, "") {
            Ok(b) => done.push(serde_json::to_value(b).unwrap_or(Value::Null)),
            Err(e) => failed.push(json!({"title": g.title, "error": e})),
        }
    }
    json!({"done": done, "failed": failed, "noSaves": none, "dest": dest})
}

/// Folders you add for a game must be your own: in your home folder or the game's folders.
fn allowed_folder(g: &GameEntry, p: &Path) -> bool {
    let p = std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    let home = util::home();
    p != home && (p.starts_with(&home) || g.install.as_ref().map(|i| p.starts_with(i)).unwrap_or(false) || g.prefix.as_ref().map(|x| p.starts_with(x)).unwrap_or(false))
}

// ------------------------------------------------------------------ the page's calls

#[tauri::command]
fn info() -> Value {
    let args: Vec<String> = std::env::args().collect();
    json!({"version": env!("CARGO_PKG_VERSION"), "game": util::arg_value(&args, "--game"), "home": util::home()})
}

#[tauri::command]
async fn load(app: tauri::AppHandle, refresh: bool) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let a = app.clone();
        load_all(refresh, &move |t: &str| { let _ = a.emit("saves://progress", json!({"text": t})); })
    }).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn rescan(id: String) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let s = rescan_one(&id)?;
        let g = game(&id)?;
        Ok(game_json(&g, Some(&s)))
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
fn backup(app: tauri::AppHandle, ids: Vec<String>, dest: String) -> Result<Value, String> {
    let dest = PathBuf::from(dest);
    if !dest.is_dir() && dest != saves::default_dest() { return Err("That place isn't there anymore (was the drive unplugged?).".into()); }
    if BUSY.swap(true, std::sync::atomic::Ordering::SeqCst) { return Err("Saves is busy with something else. Wait for it to finish.".into()); }
    std::thread::spawn(move || {
        let a = app.clone();
        let r = back_up(&ids, &dest, &move |t: &str, p: Option<f64>| { let _ = a.emit("saves://progress", json!({"text": t, "pct": p})); });
        BUSY.store(false, std::sync::atomic::Ordering::SeqCst);
        let _ = app.emit("saves://done", json!({"what": "backup", "result": r}));
    });
    Ok(json!({"started": true}))
}

#[tauri::command]
async fn backups() -> Value {
    tauri::async_runtime::spawn_blocking(|| {
        let cfg = saves::load_config();
        let dests: Vec<PathBuf> = saves::destinations(&cfg).iter().filter_map(|d| d["path"].as_str().map(PathBuf::from)).collect();
        let mut list = saves::list_backups(&dests);
        list.extend(saves::list_backups(&[saves::safety_dir()]));
        list.sort_by(|a, b| b.created.cmp(&a.created));
        json!({"backups": list, "safetyDir": saves::safety_dir()})
    }).await.unwrap_or(Value::Null)
}

#[tauri::command]
async fn restore(file: String, id: String) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        if BUSY.swap(true, std::sync::atomic::Ordering::SeqCst) { return Err("Saves is busy with something else. Wait for it to finish.".to_string()); }
        let r = (|| -> Result<Value, String> {
            let g = game(&id)?;
            let current = rescan_one(&id)?;
            let r = saves::restore(Path::new(&file), &g, &current.files)?;
            let _ = rescan_one(&id);
            Ok(serde_json::to_value(r).unwrap_or(Value::Null))
        })();
        BUSY.store(false, std::sync::atomic::Ordering::SeqCst);
        r
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
fn delete_backup(file: String) -> Result<(), String> {
    let p = PathBuf::from(&file);
    // Backups are <place>/Hearth Saves/<game>/<name>.zip (safety copies too, in their own place).
    let inside = p.parent().and_then(|g| g.parent()).and_then(|f| f.file_name()).map(|n| n == saves::FOLDER).unwrap_or(false);
    if !inside || p.extension().map(|e| e != "zip").unwrap_or(true) { return Err("That isn't one of Saves' backups.".into()); }
    saves::read_meta(&p)?;
    std::fs::remove_file(&p).map_err(|e| format!("Couldn't delete it ({e})"))
}

#[tauri::command]
fn set_schedule(schedule: String, dest: String, keep: u64) -> Result<Value, String> {
    if !["off", "daily", "weekly"].contains(&schedule.as_str()) { return Err("Choose daily, weekly or off.".into()); }
    let mut cfg = saves::load_config();
    cfg["schedule"] = json!(schedule);
    cfg["dest"] = json!(dest);
    cfg["keep"] = json!(keep.clamp(1, 50));
    saves::save_config(&cfg)?;
    saves::ensure_timer(schedule != "off")?;
    Ok(public_config(&cfg))
}

#[tauri::command]
fn run_auto(app: tauri::AppHandle) -> Result<Value, String> {
    if BUSY.swap(true, std::sync::atomic::Ordering::SeqCst) { return Err("Saves is busy with something else. Wait for it to finish.".into()); }
    std::thread::spawn(move || {
        let _ = app.emit("saves://progress", json!({"text": "Backing up every game whose saves changed"}));
        let (list, m) = with(|d| (d.games.clone(), d.manifest.clone()));
        let r = saves::auto_run(&list, m.as_deref(), true);
        BUSY.store(false, std::sync::atomic::Ordering::SeqCst);
        let _ = app.emit("saves://done", json!({"what": "auto", "ok": r.is_ok(), "message": r.unwrap_or_else(|e| e), "config": public_config(&saves::load_config())}));
    });
    Ok(json!({"started": true}))
}

#[tauri::command]
fn set_folder(id: String, path: String, add: bool) -> Result<Value, String> {
    let g = game(&id)?;
    let p = PathBuf::from(&path);
    if add && (!p.is_dir() || !allowed_folder(&g, &p)) { return Err("Choose a folder in your home folder or the game's own folders.".into()); }
    let mut cfg = saves::load_config();
    let mut list: Vec<String> = saves::custom_dirs(&cfg, &id).into_iter().map(|x| x.to_string_lossy().to_string()).collect();
    list.retain(|x| x != &path);
    if add { list.push(path); }
    cfg["custom"][&id] = json!(list);
    saves::save_config(&cfg)?;
    let s = rescan_one(&id)?;
    Ok(game_json(&g, Some(&s)))
}

#[tauri::command]
fn destinations() -> Value { Value::Array(saves::destinations(&saves::load_config())) }
#[tauri::command]
fn places() -> Value { hearth_gaming::browse::places() }
#[tauri::command]
fn browse(path: String, files: bool) -> Result<Value, String> { hearth_gaming::browse::list(Path::new(&path), files, false, false) }
#[tauri::command]
fn open_folder(path: String) -> Result<(), String> { util::open_folder(Path::new(&path)) }
#[tauri::command]
fn make_folder(path: String) -> Result<(), String> {
    let p = PathBuf::from(&path);
    if !p.starts_with(util::home()) && !p.starts_with("/run/media") && !p.starts_with("/media") { return Err("Folders can be made in your home folder or on a USB drive.".into()); }
    std::fs::create_dir_all(&p).map_err(|e| e.to_string())
}

// ------------------------------------------------------------------ without a window

fn cli(args: &[String]) -> Option<i32> {
    let print = |v: &Value| println!("{}", serde_json::to_string_pretty(v).unwrap_or_default());
    if util::has_arg(args, "--auto") {
        let list = games::list();
        let (m, _) = load_manifest(false);
        return Some(match saves::auto_run(&list, m.as_deref(), util::has_arg(args, "--force")) { Ok(t) => { println!("{t}"); 0 } Err(e) => { eprintln!("{e}"); 1 } });
    }
    if util::has_arg(args, "--refresh-manifest") {
        let (_, info) = load_manifest(true);
        print(&info);
        return Some(if info["ok"] == true { 0 } else { 1 });
    }
    if util::has_arg(args, "--list") { print(&load_all(false, &|_| {})); return Some(0); }
    if let Some(dir) = util::arg_value(args, "--backups") { print(&json!(saves::list_backups(&[PathBuf::from(dir)]))); return Some(0); }
    if let Some(which) = util::arg_value(args, "--backup") {
        let to = util::arg_value(args, "--to").map(PathBuf::from).unwrap_or_else(saves::default_dest);
        load_all(false, &|_| {});
        let ids: Vec<String> = if which == "all" { with(|d| d.games.iter().map(|g| g.id.clone()).collect()) } else { vec![which] };
        let r = back_up(&ids, &to, &|t, _| eprintln!("{t}"));
        print(&r);
        return Some(if r["failed"].as_array().map(|a| a.is_empty()).unwrap_or(true) && !r["done"].as_array().map(|a| a.is_empty()).unwrap_or(true) { 0 } else { 1 });
    }
    if let Some(zip) = util::arg_value(args, "--restore") {
        load_all(false, &|_| {});
        let meta = match saves::read_meta(Path::new(&zip)) { Ok(m) => m, Err(e) => { eprintln!("{e}"); return Some(1); } };
        let id = util::arg_value(args, "--game").unwrap_or_else(|| meta["id"].as_str().unwrap_or("").to_string());
        let g = match game(&id) { Ok(g) => g, Err(e) => { eprintln!("{e} ({id})"); return Some(1); } };
        if !util::has_arg(args, "--yes") {
            eprint!("Restore {} from {}? What's there now is kept in a safety copy first. Type yes: ", g.title, zip);
            let mut s = String::new();
            let _ = std::io::stdin().read_line(&mut s);
            if s.trim() != "yes" { eprintln!("Nothing changed."); return Some(1); }
        }
        let cur = rescan_one(&id).map(|s| s.files).unwrap_or_default();
        return Some(match saves::restore(Path::new(&zip), &g, &cur) { Ok(r) => { print(&json!(r)); 0 } Err(e) => { eprintln!("{e}"); 1 } });
    }
    None
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let Some(code) = cli(&args) { std::process::exit(code); }
    let fullscreen = util::has_arg(&args, "--fullscreen");
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() { std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1"); }
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![info, load, rescan, backup, backups, restore, delete_backup, set_schedule, run_auto, set_folder, destinations, places, browse, open_folder, make_folder])
        .setup(move |app| {
            tauri::WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::App("saves.html".into()))
                .title("Saves")
                .inner_size(1280.0, 800.0)
                .min_inner_size(640.0, 480.0)
                .fullscreen(fullscreen)
                .background_color(tauri::window::Color(15, 11, 9, 255))
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Saves couldn't start");
}
