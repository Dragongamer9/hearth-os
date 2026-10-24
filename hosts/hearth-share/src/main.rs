// Hearth Share's window (Tauri: WebKitGTK on Linux): start or stop showing this screen in a web browser on the
// network (with its link and QR code), and Moonlight through Hearth's Remote play. The work is in lib.rs.
//   hearth-share                        the window (--fullscreen)
//   hearth-share --serve SECONDS [--port P] [--no-sound]   share without a window for a while (prints the links)
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use hearth_share::{Options, Sharing};
use serde_json::{json, Value};
use std::sync::atomic::Ordering;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

struct App { share: Mutex<Option<Sharing>> }

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> Result<T, String> + Send + 'static) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| e.to_string())?
}

fn sharing_json(s: &mut Sharing) -> Value {
    let urls = s.urls();
    let qr = s.qr();
    json!({"urls": urls, "qr": qr, "started": s.started, "viewers": s.viewers.load(Ordering::SeqCst), "listeners": s.listeners.load(Ordering::SeqCst),
           "backend": s.backend, "options": s.options, "hasSound": s.has_sound, "alive": s.alive(), "port": s.port})
}

fn status_of(a: &App) -> Value {
    let mut g = a.share.lock().unwrap_or_else(|e| e.into_inner());
    let sharing = g.as_mut().map(sharing_json);
    let session = hearth_media::env::session();
    json!({"sharing": sharing, "session": session, "sessionLabel": session.label()})
}

#[tauri::command]
fn status(app: tauri::AppHandle) -> Value {
    let a = app.state::<App>();
    let v = status_of(&a);
    v
}

#[tauri::command]
async fn start(app: tauri::AppHandle, options: Options) -> Result<Value, String> {
    blocking(move || {
        let a = app.state::<App>();
        if let Some(mut old) = a.share.lock().unwrap_or_else(|e| e.into_inner()).take() { old.halt(); }
        let s = Sharing::start(options, 0)?;
        *a.share.lock().unwrap_or_else(|e| e.into_inner()) = Some(s);
        Ok(status_of(&a))
    }).await
}

#[tauri::command]
async fn stop(app: tauri::AppHandle) -> Result<Value, String> {
    blocking(move || {
        let a = app.state::<App>();
        let old = a.share.lock().unwrap_or_else(|e| e.into_inner()).take();
        if let Some(mut s) = old { s.halt(); }
        Ok(status_of(&a))
    }).await
}

#[tauri::command]
async fn remote_status() -> Result<Value, String> {
    #[cfg(target_os = "linux")]
    { blocking(|| Ok(hearth_share::moonlight::status())).await }
    #[cfg(not(target_os = "linux"))]
    { Ok(json!({"installed": false})) }
}

#[tauri::command]
async fn remote_on(on: bool) -> Result<Value, String> {
    #[cfg(target_os = "linux")]
    { blocking(move || hearth_share::moonlight::set_on(on)).await }
    #[cfg(not(target_os = "linux"))]
    { let _ = on; Err("Moonlight sharing comes with Hearth OS".into()) }
}

#[tauri::command]
async fn remote_pair(pin: String, name: String) -> Result<bool, String> {
    #[cfg(target_os = "linux")]
    { blocking(move || hearth_share::moonlight::pair(&pin, &name)).await }
    #[cfg(not(target_os = "linux"))]
    { let _ = (pin, name); Err("Moonlight sharing comes with Hearth OS".into()) }
}

#[tauri::command]
async fn remote_unpair(uuid: String) -> Result<bool, String> {
    #[cfg(target_os = "linux")]
    { blocking(move || hearth_share::moonlight::unpair(&uuid)).await }
    #[cfg(not(target_os = "linux"))]
    { let _ = uuid; Err("Moonlight sharing comes with Hearth OS".into()) }
}

fn cli(args: &[String]) -> Option<i32> {
    let i = args.iter().position(|a| a == "--serve")?;
    let secs: u64 = args.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(30);
    let port: u16 = args.iter().position(|a| a == "--port").and_then(|i| args.get(i + 1)).and_then(|s| s.parse().ok()).unwrap_or(0);
    let o = Options { sound: !args.iter().any(|a| a == "--no-sound"), ..Options::default() };
    match Sharing::start(o, port) {
        Ok(mut s) => {
            println!("{}", json!({"ok": true, "urls": s.urls(), "port": s.port, "token": s.token, "backend": s.backend, "hasSound": s.has_sound}));
            let until = std::time::Instant::now() + Duration::from_secs(secs);
            while std::time::Instant::now() < until && s.alive() { std::thread::sleep(Duration::from_millis(200)); }
            let ok = s.alive();
            s.halt();
            if ok { Some(0) } else { eprintln!("The screen capture stopped: {}", hearth_media::tools::tail(&hearth_media::dirs::runtime("hearth-share").join("capture.log"), 3)); Some(1) }
        }
        Err(e) => { println!("{}", json!({"ok": false, "error": e})); Some(1) }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let Some(code) = cli(&args) { std::process::exit(code); }
    let fullscreen = args.iter().any(|a| a == "--fullscreen");
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() { std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1"); }
    tauri::Builder::default()
        .manage(App { share: Mutex::new(None) })
        .invoke_handler(tauri::generate_handler![status, start, stop, remote_status, remote_on, remote_pair, remote_unpair])
        .setup(move |app| {
            WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title("Share")
                .inner_size(1080.0, 720.0)
                .min_inner_size(560.0, 440.0)
                .fullscreen(fullscreen)
                .background_color(tauri::window::Color(15, 11, 9, 255))
                .build()?;
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("Share couldn't start")
        .run(|app, ev| {
            // Sharing stops with the window.
            if let tauri::RunEvent::Exit = ev {
                let a = app.state::<App>();
                let old = a.share.lock().unwrap_or_else(|e| e.into_inner()).take();
                if let Some(mut s) = old { s.halt(); }
            }
        });
}
