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
//   hearth-mods --organizer <instance> <command…>   the Hearth Mod Organizer, as JSON (docs/MODS.md); `run` is what
//                                                   the library's Play starts for an organizer instance
//   hearth-mods --serve                             the worker without a window (Game Mode's Mods screens ask it through
//                                                   its socket: service.rs); stops by itself when idle
//
// Accounts (LoversLab, MEGA, Google: accounts.rs) are signed in to from the page only: the browser for LoversLab and
// Google, the on-screen keyboard for MEGA. Files no account fetches go through the guided flow (guided.rs).
// Nexus Mods and Wabbajack: nexus.rs, downloads.rs and wabbajack.rs. nxm:// links that arrive while the window is
// open are dropped in mods/nxm-inbox/ by the handler and picked up here within a second.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Hearth's look (Settings › Appearance), shared by Hearth's apps.
#[path = "../../../hearth-apptheme/apptheme.rs"]
mod apptheme;

use hearth_gaming::mods_app::*;
use hearth_gaming::{mods, util};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

#[tauri::command]
fn info() -> Value {
    let args: Vec<String> = std::env::args().collect();
    json!({"version": env!("CARGO_PKG_VERSION"), "game": util::arg_value(&args, "--game"), "page": util::arg_value(&args, "--page"), "targets": mods::TARGETS.iter().map(|t| json!({"id": t.0, "label": t.1, "prefix": t.2})).collect::<Vec<_>>()})
}
#[tauri::command]
async fn call(name: String, args: Value) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || route(&name, &args)).await.map_err(|e| e.to_string())?
}

/// The standard Linux file chooser (Hearth Files through the portal in Hearth OS): a file (with `filters`, extensions
/// without dots) or a folder. None: closed without a choice. Runs off the window's thread, so the window stays live.
#[tauri::command(async)]
fn pick_path(app: tauri::AppHandle, folder: bool, title: String, start: Option<String>, filters: Option<Vec<String>>) -> Option<String> {
    use tauri_plugin_dialog::DialogExt;
    let mut d = app.dialog().file().set_title(if title.is_empty() { if folder { "Choose a folder" } else { "Choose a file" } } else { title.as_str() });
    if let Some(dir) = start.map(PathBuf::from).map(|p| if p.is_dir() { p } else { p.parent().map(Path::to_path_buf).unwrap_or(p) }).filter(|p| p.is_dir()) { d = d.set_directory(dir); }
    if let Some(f) = filters.filter(|f| !f.is_empty() && !folder) {
        let exts: Vec<&str> = f.iter().map(|s| s.as_str()).collect();
        d = d.add_filter("Mods", &exts);
    }
    let picked = if folder { d.blocking_pick_folder() } else { d.blocking_pick_file() };
    picked.and_then(|f| f.into_path().ok()).map(|p| p.to_string_lossy().into_owned())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if util::has_arg(&args, "--serve") { std::process::exit(serve_headless()); }
    if let Some(code) = cli(&args) { std::process::exit(code); }
    let fullscreen = util::has_arg(&args, "--fullscreen");
    // This window is the worker (the queue, the installs, nxm links) unless one is already running (Game Mode's).
    let _ = std::fs::create_dir_all(data_root().join("mods"));
    let _ = std::fs::write(pid_file(), std::process::id().to_string());
    become_worker();
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() { std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1"); }
    tauri::Builder::default().plugin(apptheme::plugin())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![info, call, pick_path])
        .setup(move |app| {
            tauri::WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::App("mods.html".into()))
                .title("Mods")
                .inner_size(1280.0, 800.0)
                .min_inner_size(640.0, 480.0)
                .fullscreen(fullscreen)
                .background_color(apptheme::bg())
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Mods couldn't start");
}
