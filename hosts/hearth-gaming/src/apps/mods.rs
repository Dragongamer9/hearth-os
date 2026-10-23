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
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use hearth_gaming::games::{self, GameEntry};
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
    json!({"version": env!("CARGO_PKG_VERSION"), "game": util::arg_value(&args, "--game"), "targets": mods::TARGETS.iter().map(|t| json!({"id": t.0, "label": t.1, "prefix": t.2})).collect::<Vec<_>>()})
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
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() { std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1"); }
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![info, list, mods_of, stage, install, cancel_stage, toggle, move_mod, remove_mod, open_folder, open_beacon, places, browse, target_root])
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
