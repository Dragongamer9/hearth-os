//! Hearth Documents' window (Tauri: WebKitGTK on Linux, WebView2 on Windows). The page (ui/index.html) reads PDFs
//! with pdf.js and e-books with epub.js (both in ui/vendor); it gets a document's bytes from read_file, comic book
//! pages through the "media" scheme, and remembers where you were through the state commands.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use hearth_documents::{cbz, kind, state};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use tauri::http::{header, Response, StatusCode};
use tauri::{WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

struct Startup { target: Option<PathBuf>, selftest: bool }

#[tauri::command]
fn init(start: tauri::State<Startup>) -> Value {
    json!({
        "target": start.target.as_ref().map(|p| p.to_string_lossy().into_owned()),
        "selftest": start.selftest,
        "platform": if cfg!(windows) { "windows" } else { "linux" },
        "version": env!("CARGO_PKG_VERSION"),
        "state": state::load(),
    })
}

/// A document's bytes (PDF and EPUB are read by the page).
#[tauri::command(async)]
fn read_file(path: String) -> Result<tauri::ipc::Response, String> {
    let p = Path::new(&path);
    let meta = std::fs::metadata(p).map_err(|e| hearth_documents::friendly_io(&e))?;
    if meta.len() > 2_000_000_000 { return Err("That file is too big to open".into()); }
    Ok(tauri::ipc::Response::new(std::fs::read(p).map_err(|e| hearth_documents::friendly_io(&e))?))
}

#[tauri::command(async)]
fn file_meta(path: String) -> Result<Value, String> {
    let m = std::fs::metadata(&path).map_err(|e| hearth_documents::friendly_io(&e))?;
    let ms = |t: std::io::Result<std::time::SystemTime>| t.ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_millis() as u64);
    Ok(json!({ "size": m.len(), "modified": ms(m.modified()), "kind": kind(Path::new(&path)), "folder": Path::new(&path).parent() }))
}

#[tauri::command(async)]
fn cbz_pages(path: String) -> Result<Vec<String>, String> { cbz::pages(Path::new(&path)) }

#[tauri::command(async)]
fn remember(path: String, fields: Value) -> Result<(), String> { state::remember(&path, &fields).map(|_| ()) }

#[tauri::command(async)]
fn forget(path: String) -> Result<(), String> { state::forget(&path) }

#[tauri::command(async)]
fn settings(value: Value) -> Result<(), String> { state::set_settings(&value) }

#[tauri::command(async)]
fn recent() -> Value { state::load() }

#[tauri::command(async)]
fn pick(app: tauri::AppHandle, from: Option<String>) -> Option<String> {
    let mut d = app.dialog().file().set_title("Open a document")
        .add_filter("Documents", &["pdf", "epub", "cbz"]).add_filter("PDF", &["pdf"]).add_filter("E-books", &["epub"]).add_filter("Comic books", &["cbz"]);
    let start = from.map(PathBuf::from).and_then(|p| p.parent().map(Path::to_path_buf)).or_else(dirs::document_dir);
    if let Some(dir) = start { d = d.set_directory(dir); }
    d.blocking_pick_file().and_then(|f| f.into_path().ok()).map(|p| p.to_string_lossy().into_owned())
}

/// A link in a document: web and mail links open in their app; nothing else is opened.
#[tauri::command(async)]
fn open_url(app: tauri::AppHandle, url: String) -> Result<(), String> {
    let lower = url.to_ascii_lowercase();
    if !(lower.starts_with("http://") || lower.starts_with("https://") || lower.starts_with("mailto:")) {
        return Err("Only web and mail links open from here".into());
    }
    app.opener().open_url(url, None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command(async)]
fn show_in_folder(path: String) -> Result<(), String> {
    let p = PathBuf::from(&path);
    #[cfg(target_os = "linux")]
    {
        if std::process::Command::new("hearth-filemanager").arg(&p).spawn().is_ok() { return Ok(()); }
        let dir = p.parent().map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("/"));
        std::process::Command::new("xdg-open").arg(dir).spawn().map(|_| ()).map_err(|e| e.to_string())
    }
    #[cfg(windows)]
    { std::process::Command::new("explorer.exe").arg(format!("/select,{}", p.display())).spawn().map(|_| ()).map_err(|e| e.to_string()) }
    #[cfg(not(any(target_os = "linux", windows)))]
    { let _ = p; Err("Not on this system".into()) }
}

#[tauri::command]
fn set_title(window: tauri::WebviewWindow, title: String) { let _ = window.set_title(&title); }

#[tauri::command]
fn fullscreen(window: tauri::WebviewWindow, on: Option<bool>) -> bool {
    let now = window.is_fullscreen().unwrap_or(false);
    let want = on.unwrap_or(!now);
    if want != now { let _ = window.set_fullscreen(want); }
    want
}

#[tauri::command]
fn print(window: tauri::WebviewWindow) -> Result<(), String> { window.print().map_err(|e| e.to_string()) }

#[tauri::command]
fn log(line: String) { println!("{line}"); }

#[tauri::command]
fn quit(app: tauri::AppHandle) { app.exit(0); }

// media://localhost/cbz/<path>?i=N   page N of a comic book (Windows: http://media.localhost/...)
fn media(uri: &tauri::http::Uri) -> Response<Vec<u8>> {
    let raw = uri.path().trim_start_matches('/');
    let (route, rest) = raw.split_once('/').unwrap_or((raw, ""));
    let path = PathBuf::from(percent_encoding::percent_decode_str(rest).decode_utf8_lossy().into_owned());
    let index = uri.query().unwrap_or("").split('&').find_map(|kv| kv.strip_prefix("i=")).and_then(|s| s.parse().ok()).unwrap_or(0);
    let reply = |status: StatusCode, mime: &str, body: Vec<u8>| Response::builder().status(status).header(header::CONTENT_TYPE, mime)
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*").header(header::CACHE_CONTROL, "max-age=600").body(body).unwrap_or_else(|_| Response::new(Vec::new()));
    match (route, path.is_absolute()) {
        ("cbz", true) => match cbz::page(&path, index) {
            Ok((b, mime)) => reply(StatusCode::OK, mime, b),
            Err(e) => reply(StatusCode::NOT_FOUND, "text/plain; charset=utf-8", e.into_bytes()),
        },
        _ => reply(StatusCode::NOT_FOUND, "text/plain", b"unknown".to_vec()),
    }
}

#[cfg(target_os = "linux")]
fn pick_drawing() {
    const VAR: &str = "WEBKIT_DISABLE_DMABUF_RENDERER";
    if std::env::var_os(VAR).is_some() { return; }
    let fallback = dirs::config_dir().map(|d| d.join("nimbus").join("drawing-fallback").exists()).unwrap_or(false)
        || std::fs::read_to_string("/proc/cmdline").map(|c| c.split_whitespace().any(|w| w == "nomodeset")).unwrap_or(false);
    let nvidia = std::fs::read_dir("/sys/class/drm").into_iter().flatten().flatten()
        .any(|e| std::fs::read_to_string(e.path().join("device/vendor")).map(|v| v.trim() == "0x10de").unwrap_or(false));
    let new_driver = std::fs::read_to_string("/sys/module/nvidia/version").unwrap_or_default()
        .trim().split('.').next().and_then(|m| m.parse::<u32>().ok()).map(|m| m >= 560).unwrap_or(false);
    if fallback || (nvidia && !new_driver) { std::env::set_var(VAR, "1"); }
}

const USAGE: &str = "Hearth Documents: read PDFs, e-books (EPUB) and comic books (CBZ).

Usage: hearth-documents [--fullscreen] [DOCUMENT]
";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") { print!("{USAGE}"); return; }
    if args.iter().any(|a| a == "--version") { println!("hearth-documents {}", env!("CARGO_PKG_VERSION")); return; }
    let start_fullscreen = args.iter().any(|a| a == "--fullscreen");
    let target = args.iter().find(|a| !a.starts_with("--")).map(|a| {
        let p = hearth_documents::path_from_arg(a);
        std::fs::canonicalize(&p).unwrap_or(p)
    });
    #[cfg(target_os = "linux")]
    pick_drawing();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(Startup { target, selftest: std::env::var_os("HEARTH_DOCUMENTS_SELFTEST").is_some() })
        .register_asynchronous_uri_scheme_protocol("media", |_ctx, request, responder| {
            let uri = request.uri().clone();
            tauri::async_runtime::spawn_blocking(move || responder.respond(media(&uri)));
        })
        .invoke_handler(tauri::generate_handler![
            init, read_file, file_meta, cbz_pages, remember, forget, settings, recent, pick, open_url, show_in_folder,
            set_title, fullscreen, print, log, quit
        ])
        .setup(move |app| {
            let w = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title("Documents")
                .inner_size(1200.0, 860.0)
                .min_inner_size(420.0, 320.0)
                .fullscreen(start_fullscreen)
                .background_color(tauri::window::Color(15, 11, 9, 255))
                .build()?;
            let _ = w.set_focus();
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("couldn't start Hearth Documents");
}
