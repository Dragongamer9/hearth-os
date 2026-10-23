// Hearth Maps' window (Tauri: WebKitGTK on Linux). The page (ui/) draws the map with MapLibre GL (vendored in
// ui/vendor); map pieces, search, directions and "Where am I" are asked of the Rust side (lib.rs), so every request
// to the services keeps their rules (services.rs).
//   hearth-maps                    open the map where it was last
//   hearth-maps geo:52.5,13.4?z=15 open at a place (the geo: link handler)
//   hearth-maps --search "Berlin"  open and search
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use hearth_maps::{geo, locate, places, route, search, services::{self, Services}, tiles};
use serde_json::{json, Value};
use std::sync::{Mutex, OnceLock};
use tauri::{Emitter, WebviewUrl, WebviewWindowBuilder};

static SERVICES: OnceLock<Services> = OnceLock::new();
fn svc() -> &'static Services { SERVICES.get_or_init(services::load) }

struct Launch(Mutex<Option<Value>>);

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> Result<T, String> + Send + 'static) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn config(launch: tauri::State<'_, Launch>) -> Result<Value, String> {
    let l = launch.0.lock().ok().and_then(|mut g| g.take());
    blocking(move || Ok(json!({
        "services": svc(), "view": places::view(), "saved": places::saved(), "launch": l,
        "cacheBytes": tiles::usage(), "userAgent": services::USER_AGENT,
    }))).await
}

/// A map piece (hearth://...), as bytes. A copy kept from before, used because the service couldn't be reached,
/// also sends "maps://offline" so the page can say so.
#[tauri::command]
async fn tile(app: tauri::AppHandle, url: String) -> Result<tauri::ipc::Response, String> {
    let f = blocking(move || tiles::fetch(svc(), &url)).await?;
    if f.stale { let _ = app.emit("maps://offline", true); }
    Ok(tauri::ipc::Response::new(f.data))
}

#[tauri::command]
async fn find(query: String, near: Option<[f64; 4]>, lang: String) -> Result<Vec<search::Place>, String> {
    blocking(move || search::search(svc(), &query, near, &lang)).await
}

#[tauri::command]
async fn whats_here(lat: f64, lon: f64, zoom: u8, lang: String) -> Result<Option<search::Place>, String> {
    blocking(move || search::reverse(svc(), lat, lon, zoom, &lang)).await
}

#[tauri::command]
async fn directions(mode: String, from: [f64; 2], to: [f64; 2]) -> Result<route::Route, String> {
    blocking(move || route::route(svc(), &mode, (from[0], from[1]), (to[0], to[1]))).await
}

#[tauri::command]
fn directions_in_browser(mode: String, from: [f64; 2], to: [f64; 2]) -> Result<(), String> {
    open_with_system(&route::browser_url(&mode, (from[0], from[1]), (to[0], to[1])))
}

/// Opens a web page (the map's attribution links, a place on openstreetmap.org) in the browser.
#[tauri::command]
fn open_web(url: String) -> Result<(), String> {
    if !url.starts_with("https://") { return Err("Only web pages open from here.".into()); }
    open_with_system(&url)
}

/// Only when the person pressed "Where am I".
#[tauri::command]
async fn where_am_i() -> Result<locate::Fix, String> { blocking(|| locate::locate(25)).await }

#[tauri::command]
async fn save_place(place: places::Saved) -> Result<Vec<places::Saved>, String> { blocking(move || places::save(place)).await }

#[tauri::command]
async fn forget_place(lat: f64, lon: f64) -> Result<Vec<places::Saved>, String> { blocking(move || places::forget(lat, lon)).await }

#[tauri::command]
async fn set_view(view: places::View) -> Result<(), String> { blocking(move || places::set_view(&view)).await }

#[tauri::command]
async fn clear_cache() -> Result<u64, String> { blocking(|| { tiles::clear(); Ok(tiles::usage()) }).await }

fn open_with_system(uri: &str) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    let r = std::process::Command::new("xdg-open").arg(uri).spawn();
    #[cfg(windows)]
    let r = std::process::Command::new("rundll32").args(["url.dll,FileProtocolHandler", uri]).spawn();
    #[cfg(target_os = "macos")]
    let r = std::process::Command::new("open").arg(uri).spawn();
    r.map(|_| ()).map_err(|e| format!("Couldn't open it: {e}"))
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--version") { println!("Hearth Maps {}", env!("CARGO_PKG_VERSION")); return; }
    let fullscreen = args.iter().any(|a| a == "--fullscreen");
    let launch = if let Some(i) = args.iter().position(|a| a == "--search") {
        args.get(i + 1).map(|q| json!({"search": q}))
    } else {
        args.iter().find_map(|a| geo::parse(a)).map(|g| json!({"geo": g}))
    };
    // Loads the services' switch file in the background while the window opens.
    std::thread::spawn(|| { let _ = svc(); });
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() { std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1"); }
    tauri::Builder::default()
        .manage(Launch(Mutex::new(launch)))
        .invoke_handler(tauri::generate_handler![config, tile, find, whats_here, directions, directions_in_browser, open_web, where_am_i, save_place, forget_place, set_view, clear_cache])
        .setup(move |app| {
            WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title("Maps")
                .inner_size(1280.0, 800.0)
                .min_inner_size(520.0, 400.0)
                .fullscreen(fullscreen)
                .background_color(tauri::window::Color(15, 11, 9, 255))
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Maps couldn't start");
}
