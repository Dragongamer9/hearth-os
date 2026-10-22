// Hearth Disks' window (Tauri: WebKitGTK on Linux, WebView2 on Windows). The page (ui/index.html) asks for the
// drives, for a plan before each action, and starts actions; long ones report back as "disks://progress" events
// and finish with "disks://done". The work itself is in the library (lib.rs and the files it names).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{Emitter, WebviewUrl, WebviewWindowBuilder};

static BUSY: AtomicBool = AtomicBool::new(false);

#[tauri::command]
async fn list() -> Result<Value, String> {
    // Off the window's thread: Windows' listing runs PowerShell, which takes a moment.
    tauri::async_runtime::spawn_blocking(|| {
        let drives = hearth_disks::list()?;
        Ok(json!({"host": hearth_disks::HOST, "busy": BUSY.load(Ordering::SeqCst), "drives": drives}))
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn plan(action: String, id: String) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        hearth_disks::plan(&action, &id).map(|p| serde_json::to_value(p).unwrap_or(Value::Null))
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
fn run(app: tauri::AppHandle, action: String, id: String, opts: hearth_disks::safety::Opts) -> Result<Value, String> {
    if BUSY.swap(true, Ordering::SeqCst) { return Err("Disks is already busy with something. Wait for it to finish.".into()); }
    std::thread::spawn(move || {
        let a2 = app.clone();
        let progress = move |text: &str, pct: Option<f64>| { let _ = a2.emit("disks://progress", json!({"text": text, "pct": pct})); };
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| hearth_disks::run(&action, &id, &opts, &progress)))
            .unwrap_or_else(|_| Err("Something went wrong inside Disks. Look at the drive again before trying more.".into()));
        BUSY.store(false, Ordering::SeqCst);
        let _ = app.emit("disks://done", match r {
            Ok(message) => json!({"ok": true, "action": action, "message": message}),
            Err(error) => json!({"ok": false, "action": action, "error": error}),
        });
    });
    Ok(json!({"started": true}))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    // `hearth-disks --list`: the drives as JSON, without a window (for checking and for logs).
    if args.iter().any(|a| a == "--list") {
        match hearth_disks::list() {
            Ok(d) => { println!("{}", serde_json::to_string_pretty(&d).unwrap_or_default()); std::process::exit(0); }
            Err(e) => { eprintln!("{e}"); std::process::exit(1); }
        }
    }
    let fullscreen = args.iter().any(|a| a == "--fullscreen");
    // WebKit's newer GPU path draws a black window on some Nvidia drivers; Disks doesn't need its speed.
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() { std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1"); }
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![list, plan, run])
        .setup(move |app| {
            WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title("Disks")
                .inner_size(1180.0, 760.0)
                .min_inner_size(640.0, 480.0)
                .fullscreen(fullscreen)
                .background_color(tauri::window::Color(15, 11, 9, 255))
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Disks couldn't start");
}
