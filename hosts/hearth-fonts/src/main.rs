// Hearth Fonts' window (Tauri: WebKitGTK on Linux, WebView2 on Windows). The page (ui/index.html) lists the font
// families with a preview of each; installed fonts are shown by name (the web view finds them through fontconfig),
// a file you opened is loaded into the page (font_data). `hearth-fonts FILE` opens a font file (Hearth Files does
// this for .ttf/.otf/.ttc), to look at and install.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use base64::Engine;
use serde_json::{json, Value};
use std::path::PathBuf;
use tauri::{WebviewUrl, WebviewWindowBuilder};

struct Startup { file: Option<PathBuf> }

#[tauri::command]
fn init(start: tauri::State<Startup>) -> Value {
    json!({"file": start.file, "userDir": hearth_fonts::user_dir(), "platform": if cfg!(windows) { "windows" } else { "linux" }})
}

#[tauri::command(async)]
fn list() -> Result<Value, String> {
    hearth_fonts::list().map(|f| json!(f))
}

#[tauri::command(async)]
fn inspect(path: String) -> Result<Value, String> {
    hearth_fonts::inspect(std::path::Path::new(&path)).map(|f| json!(f))
}

/// A font file's bytes (base64) for the page to show with it.
#[tauri::command(async)]
fn font_data(path: String) -> Result<String, String> {
    let p = std::path::Path::new(&path);
    hearth_fonts::check_file(p)?;
    let b = std::fs::read(p).map_err(|e| e.to_string())?;
    Ok(base64::engine::general_purpose::STANDARD.encode(b))
}

#[tauri::command(async)]
fn browse(dir: String) -> Result<Value, String> {
    let (dir, parent, entries) = hearth_fonts::browse(&dir)?;
    Ok(json!({"dir": dir, "parent": parent, "entries": entries}))
}

#[tauri::command(async)]
fn install(path: String) -> Result<Value, String> {
    hearth_fonts::install(std::path::Path::new(&path)).map(|p| json!({"file": p}))
}

#[tauri::command(async)]
fn uninstall(file: String) -> Result<(), String> {
    hearth_fonts::uninstall(std::path::Path::new(&file))
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    // `hearth-fonts --list`: the families as JSON, without a window (for checking).
    if args.iter().any(|a| a == "--list") {
        match hearth_fonts::list() {
            Ok(f) => { println!("{}", serde_json::to_string_pretty(&f).unwrap_or_default()); std::process::exit(0); }
            Err(e) => { eprintln!("{e}"); std::process::exit(1); }
        }
    }
    let fullscreen = args.iter().any(|a| a == "--fullscreen");
    let file = args.iter().find(|a| !a.starts_with("--")).map(|a| {
        let a = a.strip_prefix("file://").unwrap_or(a);
        std::fs::canonicalize(a).unwrap_or_else(|_| PathBuf::from(a))
    });
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() { std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1"); }
    tauri::Builder::default()
        .manage(Startup { file })
        .invoke_handler(tauri::generate_handler![init, list, inspect, font_data, browse, install, uninstall])
        .setup(move |app| {
            WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title("Fonts")
                .inner_size(1120.0, 740.0)
                .min_inner_size(600.0, 440.0)
                .fullscreen(fullscreen)
                .background_color(tauri::window::Color(15, 11, 9, 255))
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Fonts couldn't start");
}
