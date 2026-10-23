// Hearth Paint's window (Tauri: WebKitGTK on Linux, WebView2 on Windows). The drawing is all in the page
// (ui/index.html, a canvas); this side reads and writes the picture files, lists folders for the page's own file
// picker, and reaches the clipboard and the screen. `hearth-paint FILE` opens a picture.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use base64::Engine;
use serde_json::{json, Value};
use std::path::PathBuf;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

struct Startup { file: Option<PathBuf> }

fn b64(b: &[u8]) -> String { base64::engine::general_purpose::STANDARD.encode(b) }
fn unb64(s: &str) -> Result<Vec<u8>, String> { base64::engine::general_purpose::STANDARD.decode(s).map_err(|_| "That picture didn't come through.".to_string()) }

#[tauri::command]
fn init(start: tauri::State<Startup>) -> Value {
    json!({"file": start.file, "pictures": hearth_paint::pictures(), "home": hearth_paint::home(), "platform": if cfg!(windows) { "windows" } else { "linux" }})
}

#[tauri::command(async)]
fn open_image(path: String) -> Result<Value, String> {
    let (b, mime) = hearth_paint::read_image(std::path::Path::new(&path))?;
    Ok(json!({"data": b64(&b), "mime": mime, "path": path}))
}

#[tauri::command(async)]
fn save_image(path: String, data: String) -> Result<(), String> {
    hearth_paint::write_image(std::path::Path::new(&path), &unb64(&data)?)
}

#[tauri::command(async)]
fn browse(dir: String) -> Result<Value, String> {
    let (dir, parent, entries) = hearth_paint::browse(&dir)?;
    Ok(json!({"dir": dir, "parent": parent, "entries": entries}))
}

#[tauri::command(async)]
fn paste_image() -> Result<Value, String> {
    Ok(match hearth_paint::clipboard_image()? { Some(b) => json!({"data": b64(&b), "mime": hearth_paint::sniff(&b)}), None => Value::Null })
}

#[tauri::command(async)]
fn copy_image(data: String) -> Result<(), String> {
    hearth_paint::copy_image(&unb64(&data)?)
}

/// The whole screen without Paint on it: the window steps aside for the picture, then comes back.
#[tauri::command(async)]
fn screenshot(app: tauri::AppHandle) -> Result<Value, String> {
    let w = app.get_webview_window("main");
    if let Some(w) = &w { let _ = w.hide(); }
    std::thread::sleep(std::time::Duration::from_millis(450));
    let r = hearth_paint::screenshot();
    if let Some(w) = &w { let _ = w.show(); let _ = w.set_focus(); }
    r.map(|b| json!({"data": b64(&b), "mime": "image/png"}))
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let fullscreen = args.iter().any(|a| a == "--fullscreen");
    let file = args.iter().find(|a| !a.starts_with("--")).map(|a| {
        let a = a.strip_prefix("file://").unwrap_or(a);
        std::fs::canonicalize(a).unwrap_or_else(|_| PathBuf::from(a))
    });
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() { std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1"); }
    tauri::Builder::default()
        .manage(Startup { file })
        .invoke_handler(tauri::generate_handler![init, open_image, save_image, browse, paste_image, copy_image, screenshot])
        .setup(move |app| {
            WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title("Paint")
                .inner_size(1280.0, 820.0)
                .min_inner_size(640.0, 480.0)
                .fullscreen(fullscreen)
                .background_color(tauri::window::Color(15, 11, 9, 255))
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Paint couldn't start");
}
