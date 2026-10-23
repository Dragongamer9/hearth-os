//! Hearth Photos' window (Tauri: WebKitGTK on Linux, WebView2 on Windows). The page (ui/index.html) shows the
//! pictures; it loads them through the "media" scheme below (the picture itself, a thumbnail, or a PNG of a type
//! the web view can't draw) and calls the commands here for everything else.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use hearth_photos::{folder, formats, imaging, info, save, system};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use tauri::http::{header, Response, StatusCode};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_dialog::DialogExt;

struct Startup { target: Option<PathBuf>, slideshow: bool, selftest: bool }

#[tauri::command]
fn init(start: tauri::State<Startup>) -> Value {
    json!({
        "target": start.target.as_ref().map(|p| p.to_string_lossy().into_owned()),
        "slideshow": start.slideshow,
        "selftest": start.selftest,
        "platform": if cfg!(windows) { "windows" } else { "linux" },
        "version": env!("CARGO_PKG_VERSION"),
        "home": dirs::home_dir(),
        "pictures": dirs::picture_dir(),
        "converters": converters(),
    })
}

/// Which of the system's converters are installed (the info panel says why a HEIC won't open).
fn converters() -> Vec<&'static str> {
    ["heif-dec", "heif-convert", "djxl", "magick", "convert"].into_iter().filter(|c| system::which(c)).collect()
}

#[tauri::command(async)]
fn list(path: String) -> Result<folder::Listing, String> { folder::list(Path::new(&path)) }

#[tauri::command(async)]
fn file_info(path: String) -> Result<info::Info, String> { info::read(Path::new(&path)) }

#[tauri::command(async)]
fn save_turn(path: String, op: u8) -> Result<save::Saved, String> { save::save_turn(Path::new(&path), op) }

/// The editor's picture comes as the request's body (raw bytes); what to do with it in the headers.
#[tauri::command(async)]
fn save_edit(request: tauri::ipc::Request<'_>) -> Result<save::Saved, String> {
    let tauri::ipc::InvokeBody::Raw(bytes) = request.body() else { return Err("No picture was sent".into()) };
    let h = |k: &str| request.headers().get(k).and_then(|v| v.to_str().ok()).map(|s| percent_encoding::percent_decode_str(s).decode_utf8_lossy().into_owned());
    let original = h("x-original").ok_or("No original")?;
    let format = h("x-format").unwrap_or_else(|| "jpeg".into());
    let quality = h("x-quality").and_then(|q| q.parse().ok()).unwrap_or(90);
    let encoded = h("x-encoded").as_deref() == Some("1");
    let overwrite = h("x-overwrite").as_deref() == Some("1");
    save::save_edit(Path::new(&original), bytes, &format, quality, encoded, overwrite)
}

/// A picture's bytes for the editor (loaded as a blob, so the canvas can be read back and saved).
#[tauri::command(async)]
fn read_bytes(path: String) -> Result<tauri::ipc::Response, String> {
    let p = Path::new(&path);
    let bytes = if formats::WEB_NATIVE.contains(&formats::ext(p).as_str()) { std::fs::read(p).map_err(|e| hearth_photos::friendly_io(&e))? } else { imaging::convert(p)? };
    Ok(tauri::ipc::Response::new(bytes))
}

#[tauri::command(async)]
fn trash(path: String) -> Result<(), String> { system::trash(Path::new(&path)) }

#[tauri::command(async)]
fn untrash(path: String) -> Result<(), String> { system::untrash(Path::new(&path)) }

#[tauri::command(async)]
fn copy_image(path: String) -> Result<(), String> { system::copy_image(Path::new(&path)) }

#[tauri::command(async)]
fn set_wallpaper(path: String) -> Result<String, String> { system::set_wallpaper(Path::new(&path)) }

#[tauri::command(async)]
fn show_in_folder(path: String) -> Result<(), String> { system::show_in_folder(Path::new(&path)) }

#[tauri::command(async)]
fn pick(app: tauri::AppHandle, from: Option<String>) -> Option<String> {
    let mut d = app.dialog().file().set_title("Open a picture").add_filter("Pictures", formats::PICTURES);
    if let Some(dir) = from.map(PathBuf::from).and_then(|p| if p.is_dir() { Some(p) } else { p.parent().map(Path::to_path_buf) }) { d = d.set_directory(dir); }
    d.blocking_pick_file().and_then(|f| f.into_path().ok()).map(|p| p.to_string_lossy().into_owned())
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

/// For CI's check (HEARTH_PHOTOS_SELFTEST=1): the page reports which pictures it could show, and how.
#[tauri::command]
fn log(line: String) { println!("{line}"); }

#[tauri::command]
fn quit(app: tauri::AppHandle) { app.exit(0); }

// ------------------------------------------------------------------ the media scheme
// media://localhost/raw/<path>        the file as it is
// media://localhost/view/<path>       a PNG of it (types the web view can't draw)
// media://localhost/thumb/<path>?s=N  a thumbnail N pixels on its longer side
// (Windows: http://media.localhost/...; the page builds these with convertFileSrc.)

fn respond(status: StatusCode, mime: &str, body: Vec<u8>, cache: bool) -> Response<Vec<u8>> {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, mime)
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .header(header::CACHE_CONTROL, if cache { "max-age=3600" } else { "no-store" })
        .body(body)
        .unwrap_or_else(|_| Response::new(Vec::new()))
}

fn media(uri: &tauri::http::Uri) -> Response<Vec<u8>> {
    let raw = uri.path().trim_start_matches('/');
    let (route, rest) = raw.split_once('/').unwrap_or((raw, ""));
    let path = PathBuf::from(percent_encoding::percent_decode_str(rest).decode_utf8_lossy().into_owned());
    let query = uri.query().unwrap_or("");
    let size = query.split('&').find_map(|kv| kv.strip_prefix("s=")).and_then(|s| s.parse().ok()).unwrap_or(256);
    let fail = |status: StatusCode, msg: String| respond(status, "text/plain; charset=utf-8", msg.into_bytes(), false);
    if !path.is_absolute() { return fail(StatusCode::BAD_REQUEST, "not a full path".into()); }
    match route {
        "raw" => match std::fs::read(&path) {
            Ok(b) => respond(StatusCode::OK, formats::mime(&formats::ext(&path)), b, false),
            Err(e) => fail(StatusCode::NOT_FOUND, hearth_photos::friendly_io(&e)),
        },
        "view" => match imaging::convert(&path) {
            Ok(b) => respond(StatusCode::OK, "image/png", b, false),
            Err(e) => fail(StatusCode::UNSUPPORTED_MEDIA_TYPE, e),
        },
        "thumb" => match imaging::thumbnail(&path, size) {
            Ok((b, mime)) => respond(StatusCode::OK, mime, b, true),
            Err(e) => fail(StatusCode::UNSUPPORTED_MEDIA_TYPE, e),
        },
        _ => fail(StatusCode::NOT_FOUND, "unknown".into()),
    }
}

/// WebKit's newer drawing path shows a black window on older Nvidia drivers and on basic graphics: there, use
/// the older one (the same rule as Hearth Terminal).
#[cfg(target_os = "linux")]
fn pick_drawing() {
    const VAR: &str = "WEBKIT_DISABLE_DMABUF_RENDERER";
    if std::env::var_os(VAR).is_some() { return; }
    let fallback = dirs::config_dir().map(|d| d.join("nimbus").join("drawing-fallback").exists()).unwrap_or(false)
        || std::fs::read_to_string("/proc/cmdline").map(|c| c.split_whitespace().any(|w| w == "nomodeset")).unwrap_or(false);
    let nvidia = std::fs::read_dir("/sys/class/drm").into_iter().flatten().flatten()
        .any(|e| std::fs::read_to_string(e.path().join("device/vendor")).map(|v| v.trim() == "0x10de").unwrap_or(false));
    // Nvidia: WebKit's newer drawing path (DMA-BUF) crashes these windows on 615.x too, so the older path is used
    // on every Nvidia driver, as Hearth Text, Clock and the other apps already do.
    let _new_driver = std::fs::read_to_string("/sys/module/nvidia/version").unwrap_or_default()
        .trim().split('.').next().and_then(|m| m.parse::<u32>().ok()).map(|m| m >= 560).unwrap_or(false);
    if fallback || nvidia { std::env::set_var(VAR, "1"); }
}

const USAGE: &str = "Hearth Photos: look at pictures, and make simple edits.

Usage: hearth-photos [options] [PICTURE or FOLDER]

  --fullscreen     start full screen
  --slideshow      start a slideshow of the folder
  --info PICTURE   print what Photos knows about a picture (JSON) and quit
  --thumb PICTURE OUT.png   write a thumbnail and quit (for checking)
  --version, --help
";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") { print!("{USAGE}"); return; }
    if args.iter().any(|a| a == "--version") { println!("hearth-photos {}", env!("CARGO_PKG_VERSION")); return; }
    if let Some(i) = args.iter().position(|a| a == "--info") {
        let p = hearth_photos::path_from_arg(args.get(i + 1).map(String::as_str).unwrap_or(""));
        match info::read(&p) {
            Ok(v) => { println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default()); return; }
            Err(e) => { eprintln!("{e}"); std::process::exit(1); }
        }
    }
    if let Some(i) = args.iter().position(|a| a == "--thumb") {
        let (Some(p), Some(out)) = (args.get(i + 1), args.get(i + 2)) else { eprint!("{USAGE}"); std::process::exit(2) };
        match imaging::thumbnail(&hearth_photos::path_from_arg(p), 256) {
            Ok((b, _)) => { std::fs::write(out, b).expect("couldn't write"); return; }
            Err(e) => { eprintln!("{e}"); std::process::exit(1); }
        }
    }
    let start_fullscreen = args.iter().any(|a| a == "--fullscreen");
    let slideshow = args.iter().any(|a| a == "--slideshow");
    let target = args.iter().find(|a| !a.starts_with("--")).map(|a| {
        let p = hearth_photos::path_from_arg(a);
        std::fs::canonicalize(&p).unwrap_or(p)
    });
    #[cfg(target_os = "linux")]
    pick_drawing();
    std::thread::spawn(imaging::tidy_cache);

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(Startup { target, slideshow, selftest: std::env::var_os("HEARTH_PHOTOS_SELFTEST").is_some() })
        .register_asynchronous_uri_scheme_protocol("media", |_ctx, request, responder| {
            let uri = request.uri().clone();
            tauri::async_runtime::spawn_blocking(move || responder.respond(media(&uri)));
        })
        .invoke_handler(tauri::generate_handler![
            init, list, file_info, save_turn, save_edit, read_bytes, trash, untrash, copy_image, set_wallpaper,
            show_in_folder, pick, set_title, fullscreen, print, log, quit
        ])
        .setup(move |app| {
            let w = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title("Photos")
                .inner_size(1280.0, 820.0)
                .min_inner_size(420.0, 320.0)
                .fullscreen(start_fullscreen)
                .background_color(tauri::window::Color(15, 11, 9, 255))
                .build()?;
            let _ = w.set_focus();
            let _ = app.get_webview_window("main");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("couldn't start Hearth Photos");
}
