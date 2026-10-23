//! Hearth Text: Hearth OS's text editor. The window is ui/index.html (CodeMirror 6, drawn by the system's web
//! view: WebKitGTK on Linux, WebView2 on Windows); this side opens and saves files (files.rs, textfile.rs),
//! shows the system's Open and Save dialogs, and saves as administrator on Linux (admin.rs). Opening another file
//! while it runs (from Hearth Files) adds a tab to the window that's already open.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use hearth_text::{admin, cli, files, settings, textfile};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_dialog::DialogExt;

/// Set once the page said it's fine to close (nothing unsaved, or you chose to close anyway).
static MAY_CLOSE: AtomicBool = AtomicBool::new(false);

struct Startup {
    launch: cli::Launch,
    cwd: PathBuf,
}

/// A path from the command line, made absolute against the folder the program was started in.
fn absolute(cwd: &Path, f: &str) -> String {
    let p = Path::new(f);
    files::display_path(&if p.is_absolute() { p.to_path_buf() } else { cwd.join(p) })
}

fn resolve(mut l: cli::Launch, cwd: &Path) -> cli::Launch {
    l.files = l.files.iter().map(|f| absolute(cwd, f)).collect();
    l
}

/// Everything the page needs to start.
#[tauri::command(async)]
fn init(start: tauri::State<Startup>) -> Value {
    json!({
        "launch": resolve(start.launch.clone(), &start.cwd),
        "settings": settings::load(),
        "platform": if cfg!(windows) { "windows" } else { "linux" },
        "home": dirs::home_dir(),
        "cwd": start.cwd,
        "version": env!("CARGO_PKG_VERSION"),
        "admin": admin::available(),
        "eol": textfile::default_eol(),
        "limits": { "edit": files::EDIT_MAX, "view": files::VIEW_MAX, "part": files::PART_SIZE },
    })
}

/// Opens a file: mode "auto" | "end" (the end of a big file) | "view"; `encoding` forces one.
#[tauri::command(async)]
fn open_file(path: String, mode: Option<String>, encoding: Option<String>) -> Result<files::Opened, String> {
    files::open(Path::new(&path), mode.as_deref().unwrap_or("auto"), encoding.as_deref())
}

/// Saves the text as `path` in the encoding and line endings given; `admin` goes through pkexec (Linux).
#[tauri::command(async)]
fn save_file(path: String, text: String, encoding: String, bom: bool, eol: String, admin: Option<bool>) -> Result<files::Stat, String> {
    let bytes = textfile::encode(&text, &encoding, bom, &eol)?;
    let p = Path::new(&path);
    if admin.unwrap_or(false) {
        admin::save(p, &bytes)?;
        return Ok(files::stat(p));
    }
    files::save(p, &bytes)
}

#[tauri::command(async)]
fn stat_file(path: String) -> files::Stat { files::stat(Path::new(&path)) }

/// The system's Open dialog (several files at once).
#[tauri::command(async)]
fn pick_open(app: tauri::AppHandle, dir: Option<String>) -> Vec<String> {
    let mut d = app.dialog().file().set_title("Open");
    if let Some(dir) = dir.filter(|d| Path::new(d).is_dir()) { d = d.set_directory(dir); }
    d.blocking_pick_files().unwrap_or_default().into_iter()
        .filter_map(|f| f.into_path().ok()).map(|p| files::display_path(&p)).collect()
}

/// The system's Save dialog, starting at `path` (its folder and name).
#[tauri::command(async)]
fn pick_save(app: tauri::AppHandle, path: Option<String>, name: Option<String>) -> Option<String> {
    let mut d = app.dialog().file().set_title("Save as");
    if let Some(p) = path.as_deref().map(Path::new) {
        if let Some(dir) = p.parent().filter(|d| d.is_dir()) { d = d.set_directory(dir); }
        if let Some(n) = p.file_name() { d = d.set_file_name(n.to_string_lossy()); }
    } else if let Some(n) = name { d = d.set_file_name(n); }
    if let Some(home) = path.is_none().then(dirs::document_dir).flatten() { d = d.set_directory(home); }
    d.blocking_save_file().and_then(|f| f.into_path().ok()).map(|p| files::display_path(&p))
}

#[tauri::command(async)]
fn settings_save(value: Value) -> Result<(), String> {
    settings::save(&value).map_err(|e| e.to_string())
}

/// The page checked for unsaved changes: close for real.
#[tauri::command]
fn quit(app: tauri::AppHandle) {
    MAY_CLOSE.store(true, Ordering::SeqCst);
    app.exit(0);
}

fn main() {
    let launch = match cli::parse(std::env::args().skip(1)) {
        Ok(cli::Parsed::Run(l)) => l,
        Ok(cli::Parsed::Help) => { print!("{}", cli::USAGE); return; }
        Ok(cli::Parsed::Version) => { println!("hearth-text {}", env!("CARGO_PKG_VERSION")); return; }
        Err(e) => { eprintln!("hearth-text: {e}\n\n{}", cli::USAGE); std::process::exit(2); }
    };
    let cwd = std::env::current_dir().unwrap_or_else(|_| dirs::home_dir().unwrap_or_default());
    // WebKit's newer GPU path draws a black window on some Nvidia drivers; an editor doesn't need its speed.
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() { std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1"); }

    tauri::Builder::default()
        // A second `hearth-text FILE` (Hearth Files opening another file) hands its files to this window.
        .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            let l = match cli::parse(argv.into_iter().skip(1)) { Ok(cli::Parsed::Run(l)) => l, _ => return };
            let l = resolve(l, Path::new(&cwd));
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.unminimize();
                let _ = w.set_focus();
            }
            let _ = app.emit("open-files", l);
        }))
        .plugin(tauri_plugin_dialog::init())
        .manage(Startup { launch, cwd })
        .invoke_handler(tauri::generate_handler![init, open_file, save_file, stat_file, pick_open, pick_save, settings_save, quit])
        .setup(|app| {
            let w = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title("Text")
                .inner_size(1040.0, 720.0)
                .min_inner_size(420.0, 300.0)
                .background_color(tauri::window::Color(15, 11, 9, 255))
                .build()?;
            let _ = w.set_focus();
            Ok(())
        })
        .on_window_event(|w, e| {
            // Closing the window asks the page first: it offers to save anything unsaved.
            if let tauri::WindowEvent::CloseRequested { api, .. } = e {
                if !MAY_CLOSE.load(Ordering::SeqCst) {
                    api.prevent_close();
                    let _ = w.emit("close-requested", ());
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("couldn't start Hearth Text");
}
