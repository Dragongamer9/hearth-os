// Hearth Camera's window (Tauri: WebKitGTK on Linux). The page shows the camera as a live MJPEG picture from
// 127.0.0.1 (a link with a token only this window knows) and asks the Rust side for photos and videos.
// The camera work is in the library (lib.rs).
//   hearth-camera                       the window (--fullscreen)
//   hearth-camera --list                the cameras as JSON
//   hearth-camera --photo [--device D]  one photo, without a window (prints where it went)
//   hearth-camera --video SECONDS [--device D] [--no-sound]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use hearth_camera::{Camera, Engine};
use hearth_media::http::{self, Request};
use serde_json::{json, Value};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

struct App { engine: Arc<Mutex<Engine>>, cams: Mutex<Vec<Camera>>, port: u16, token: String }

fn lock(a: &App) -> std::sync::MutexGuard<'_, Engine> { a.engine.lock().unwrap_or_else(|e| e.into_inner()) }

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> Result<T, String> + Send + 'static) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| e.to_string())?
}

fn state_json(a: &App) -> Value {
    let mut e = lock(a);
    let alive = e.alive();
    json!({"cameras": a.cams.lock().unwrap().clone(), "port": a.port, "token": a.token, "camera": e.cam, "mode": e.mode, "mirror": e.mirror, "alive": alive,
           "recording": e.recording.as_ref().map(|(f, t)| json!({"file": f.to_string_lossy(), "started": t}))})
}

#[tauri::command]
async fn init(app: tauri::AppHandle) -> Result<Value, String> {
    blocking(move || {
        let a = app.state::<App>();
        let cams = hearth_camera::cameras();
        *a.cams.lock().unwrap() = cams.clone();
        let mut err = None;
        {
            let mut e = lock(&a);
            let keep = cams.iter().any(|c| c.id == e.cam) && e.alive();
            if !keep {
                if let Some(c) = cams.first() { if let Err(x) = e.open(&cams, &c.id) { err = Some(x); } }
            }
        }
        let mut v = state_json(&a);
        v["error"] = json!(err);
        Ok(v)
    }).await
}

#[tauri::command]
async fn select(app: tauri::AppHandle, id: String) -> Result<Value, String> {
    blocking(move || {
        let a = app.state::<App>();
        let cams = a.cams.lock().unwrap().clone();
        lock(&a).open(&cams, &id)?;
        Ok(state_json(&a))
    }).await
}

#[tauri::command]
async fn mirror(app: tauri::AppHandle, on: bool) -> Result<Value, String> {
    blocking(move || { let a = app.state::<App>(); lock(&a).set_mirror(on)?; Ok(state_json(&a)) }).await
}

#[tauri::command]
fn status(app: tauri::AppHandle) -> Value {
    let a = app.state::<App>();
    let v = state_json(&a);
    v
}

#[tauri::command]
fn photo(app: tauri::AppHandle) -> Result<String, String> {
    let a = app.state::<App>();
    let r = lock(&a).photo().map(|p| p.to_string_lossy().to_string());
    r
}

#[tauri::command]
async fn record(app: tauri::AppHandle, on: bool, sound: bool) -> Result<String, String> {
    blocking(move || {
        let a = app.state::<App>();
        let mut e = lock(&a);
        let r = if on { e.record_start(sound) } else { e.record_stop() };
        drop(e);
        r.map(|p| p.to_string_lossy().to_string())
    }).await
}

#[tauri::command]
async fn files() -> Result<Value, String> {
    blocking(|| {
        let mut v = hearth_media::files::list(&hearth_camera::photos_dir(), hearth_media::files::PHOTO, 60);
        v.extend(hearth_media::files::list(&hearth_camera::videos_dir(), hearth_media::files::VIDEO, 60));
        v.sort_by(|a, b| b["time"].as_u64().cmp(&a["time"].as_u64()));
        v.truncate(60);
        Ok(json!({"files": v, "photoDir": hearth_camera::photos_dir().to_string_lossy(), "videoDir": hearth_camera::videos_dir().to_string_lossy()}))
    }).await
}

#[tauri::command]
fn open(path: String) -> Result<(), String> {
    let p = hearth_media::files::inside(&path, &hearth_camera::dirs()).ok_or("That file isn't one of Camera's")?;
    hearth_media::notify::open(&p)
}

#[tauri::command]
fn show(path: String) -> Result<(), String> {
    let p = hearth_media::files::inside(&path, &hearth_camera::dirs()).ok_or("That file isn't one of Camera's")?;
    hearth_media::notify::show(&p)
}

#[tauri::command]
fn delete(path: String) -> Result<(), String> { hearth_media::files::delete(&path, &hearth_camera::dirs()) }

/// The window's own little server: the live picture, and photos for the thumbnails. Only on 127.0.0.1, and only
/// with the token.
fn serve(engine: Arc<Mutex<Engine>>, token: String) -> Result<u16, String> {
    let l = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    let port = l.local_addr().map_err(|e| e.to_string())?.port();
    let hub = engine.lock().unwrap_or_else(|e| e.into_inner()).hub.clone();
    let stop = Arc::new(AtomicBool::new(false));
    let viewers = Arc::new(AtomicUsize::new(0));
    let s2 = stop.clone();
    http::serve(l, stop, Arc::new(move |r: Request, mut s: TcpStream| {
        if r.param("k").as_deref() != Some(token.as_str()) { let _ = http::respond(&mut s, "403 Forbidden", "text/plain", b"no"); return; }
        match r.path.as_str() {
            "/live.mjpg" => http::stream_mjpeg(&mut s, &hub, &s2, &viewers, Duration::from_secs(3600)),
            "/frame.jpg" => match hub.latest() { Some((_, f)) => { let _ = http::respond(&mut s, "200 OK", "image/jpeg", &f); } None => { let _ = http::respond(&mut s, "404 Not Found", "text/plain", b"no picture yet"); } },
            "/file" => {
                let p = r.param("p").map(|p| http::decode(&p)).unwrap_or_default();
                match hearth_media::files::inside(&p, &hearth_camera::dirs()).and_then(|f| std::fs::read(f).ok()) {
                    Some(b) => { let _ = http::respond(&mut s, "200 OK", "image/jpeg", &b); }
                    None => { let _ = http::respond(&mut s, "404 Not Found", "text/plain", b"no"); }
                }
            }
            _ => { let _ = http::respond(&mut s, "404 Not Found", "text/plain", b"no"); }
        }
    }));
    Ok(port)
}

fn cli(args: &[String]) -> Option<i32> {
    let has = |f: &str| args.iter().any(|a| a == f);
    let val = |f: &str| args.iter().position(|a| a == f).and_then(|i| args.get(i + 1)).cloned();
    if has("--list") { println!("{}", serde_json::to_string_pretty(&hearth_camera::cameras()).unwrap_or_default()); return Some(0); }
    if !has("--photo") && !has("--video") { return None; }
    let cams = hearth_camera::cameras();
    let Some(id) = val("--device").or_else(|| cams.first().map(|c| c.id.clone())) else { eprintln!("There's no camera"); return Some(1) };
    let mut e = Engine::new();
    if let Err(x) = e.open(&cams, &id) { eprintln!("{x}"); return Some(1); }
    let r = if has("--photo") {
        std::thread::sleep(Duration::from_millis(800));
        e.photo()
    } else {
        let secs: u64 = val("--video").and_then(|s| s.parse().ok()).unwrap_or(5);
        e.record_start(!has("--no-sound")).and_then(|_| { std::thread::sleep(Duration::from_secs(secs)); e.record_stop() })
    };
    match r { Ok(p) => { println!("{}", p.display()); Some(0) } Err(x) => { eprintln!("{x}"); Some(1) } }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let Some(code) = cli(&args) { std::process::exit(code); }
    let fullscreen = args.iter().any(|a| a == "--fullscreen");
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() { std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1"); }
    let engine = Arc::new(Mutex::new(Engine::new()));
    let token = hearth_media::tools::token(12);
    let port = serve(engine.clone(), token.clone()).expect("Camera couldn't start its picture server");
    tauri::Builder::default()
        .manage(App { engine, cams: Mutex::new(vec![]), port, token })
        .invoke_handler(tauri::generate_handler![init, select, mirror, status, photo, record, files, open, show, delete])
        .setup(move |app| {
            WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title("Camera")
                .inner_size(1180.0, 760.0)
                .min_inner_size(600.0, 440.0)
                .fullscreen(fullscreen)
                .background_color(tauri::window::Color(15, 11, 9, 255))
                .build()?;
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("Camera couldn't start")
        .run(|app, ev| {
            // The camera's light goes off with the window (a recording is finished first).
            if let tauri::RunEvent::Exit = ev {
                let a = app.state::<App>();
                let mut e = lock(&a);
                if e.recording.is_some() { let _ = e.record_stop(); }
                e.halt();
            }
        });
}
