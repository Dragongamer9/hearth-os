// Hearth Disks' window (Tauri: WebKitGTK on Linux, WebView2 on Windows). The page (ui/index.html) asks for the
// drives, for a plan before each action, and starts actions; long ones report back as "disks://progress" events
// and finish with "disks://done". The work itself is in the library (lib.rs and the files it names). On Linux the page
// also has Make a bootable USB (usb_* calls, "usb://progress" and "usb://done"; library: src/usb).
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

// ---------------------------------------------------------------- Make a bootable USB (Linux)
// The page's USB maker screen calls these; a job reports on "usb://progress" and ends with "usb://done".

#[cfg(target_os = "linux")]
mod usb_cmds {
    use super::BUSY;
    use hearth_disks::usb;
    use serde_json::{json, Value};
    use std::sync::atomic::Ordering;
    use std::sync::Arc;
    use tauri::Emitter;

    fn blocking<T: Send + 'static>(f: impl FnOnce() -> Result<T, String> + Send + 'static) -> impl std::future::Future<Output = Result<T, String>> {
        async move { tauri::async_runtime::spawn_blocking(f).await.map_err(|e| e.to_string())? }
    }

    #[tauri::command]
    pub fn usb_catalog() -> Value { usb::catalog_for_page() }

    #[tauri::command]
    pub async fn usb_resolve(id: String) -> Result<Value, String> { blocking(move || usb::resolve_for_page(&id)).await }

    #[tauri::command]
    pub async fn usb_drives(show_all: bool) -> Result<Value, String> {
        blocking(move || usb::drives(show_all).map(|d| json!({"drives": d, "busy": BUSY.load(Ordering::SeqCst)}))).await
    }

    #[tauri::command]
    pub async fn usb_check_file(app: tauri::AppHandle, path: String) -> Result<Value, String> {
        blocking(move || {
            let a = app.clone();
            let emit: usb::Emit = Arc::new(move |v: Value| { let _ = a.emit("usb://progress", v); });
            usb::check_file(&path, &emit)
        }).await
    }

    /// Hearth's file picker (Hearth Files through the desktop portal) for an ISO or disk image.
    #[tauri::command(async)]
    pub fn usb_pick_file(app: tauri::AppHandle) -> Option<String> {
        use tauri_plugin_dialog::DialogExt;
        let mut d = app.dialog().file().set_title("Choose an ISO or disk image").add_filter("ISO and disk images", &["iso", "img", "xz", "zst", "gz", "raw"]);
        let dl = usb::downloads_dir();
        if dl.is_dir() { d = d.set_directory(dl); }
        d.blocking_pick_file().and_then(|f| f.into_path().ok()).map(|p| p.to_string_lossy().into_owned())
    }

    #[tauri::command]
    pub fn usb_start(app: tauri::AppHandle, job: usb::Job) -> Result<Value, String> {
        if BUSY.swap(true, Ordering::SeqCst) { return Err("Disks is already busy with something. Wait for it to finish.".into()); }
        std::thread::spawn(move || {
            let a = app.clone();
            let emit: usb::Emit = Arc::new(move |v: Value| { let _ = a.emit("usb://progress", v); });
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| usb::run(&job, emit)))
                .unwrap_or_else(|_| Err("Something went wrong inside Disks. Look at the drive again before using it.".into()));
            BUSY.store(false, Ordering::SeqCst);
            if let Err(e) = &r { usb::log(&format!("failed: {e}")); }
            let _ = app.emit("usb://done", match r {
                Ok(v) => json!({"ok": true, "result": v}),
                Err(error) => json!({"ok": false, "error": error, "cancelled": error == usb::CANCELLED}),
            });
        });
        Ok(json!({"started": true}))
    }

    #[tauri::command]
    pub fn usb_cancel() { usb::cancel(); }

    #[tauri::command]
    pub fn usb_open_page(id: String) -> Result<(), String> { usb::open_page(&id) }

    #[tauri::command]
    pub fn usb_find_windows(since: u64) -> Option<Value> { usb::find_windows_iso(since) }

    #[tauri::command]
    pub fn usb_cache() -> Value { json!({"dir": usb::cache_dir(), "files": usb::cache_list()}) }

    #[tauri::command]
    pub fn usb_cache_remove(path: String) -> Result<(), String> { usb::cache_remove(&path) }

    #[tauri::command]
    pub fn usb_log() -> String { usb::log_tail() }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    // `hearth-disks --list`: the drives as JSON, without a window (for checking and for logs).
    // `hearth-disks --usb-catalog-check`: every catalog entry found, its signature checked and its server asked (CI).
    #[cfg(target_os = "linux")]
    if args.iter().any(|a| a == "--usb-catalog-check") {
        let (lines, bad) = hearth_disks::usb::catalog_check();
        for l in &lines { println!("{l}"); }
        for b in &bad { eprintln!("PROBLEM {b}"); }
        std::process::exit(if bad.is_empty() { 0 } else { 1 });
    }
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
    // `hearth-disks --usb` opens straight on Make a bootable USB (its own menu entry).
    // `hearth-disks --usb FILE` (Open with… on an ISO in Files) opens it there, ready to check and write.
    let usb_first = args.iter().any(|a| a == "--usb");
    let usb_file = args.iter().position(|a| a == "--usb").and_then(|i| args.get(i + 1)).filter(|f| !f.starts_with('-'))
        .map(|f| f.strip_prefix("file://").unwrap_or(f).to_string()).filter(|f| std::path::Path::new(f).is_file());
    let init = if usb_first { format!("window.__DISKS_USB_FIRST = true; window.__DISKS_USB_FILE = {};", serde_json::to_string(&usb_file).unwrap_or_else(|_| "null".into())) } else { String::new() };
    let builder = tauri::Builder::default();
    #[cfg(target_os = "linux")]
    let builder = builder.plugin(tauri_plugin_dialog::init()).invoke_handler(tauri::generate_handler![list, plan, run,
        usb_cmds::usb_catalog, usb_cmds::usb_resolve, usb_cmds::usb_drives, usb_cmds::usb_check_file, usb_cmds::usb_pick_file, usb_cmds::usb_start,
        usb_cmds::usb_cancel, usb_cmds::usb_open_page, usb_cmds::usb_find_windows, usb_cmds::usb_cache, usb_cmds::usb_cache_remove, usb_cmds::usb_log]);
    #[cfg(not(target_os = "linux"))]
    let builder = builder.invoke_handler(tauri::generate_handler![list, plan, run]);
    builder
        .setup(move |app| {
            WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .initialization_script(&init)
                .title(if usb_first { "Make a bootable USB" } else { "Disks" })
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
