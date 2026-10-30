// Hearth Tools' window (Tauri: WebKitGTK on Linux). `hearth-tools --app <name>` opens one tool (lib.rs lists
// them); its page (ui/<name>.html) calls `call` for quick answers and `start` for long jobs, which report back as
// "tools://job" events ({id, kind: "step" | "done", ...}). `cancel` stops a job.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{Emitter, WebviewUrl, WebviewWindowBuilder};

static NEXT: AtomicU64 = AtomicU64::new(1);
static JOBS: Mutex<Option<HashMap<u64, Arc<AtomicBool>>>> = Mutex::new(None);

#[tauri::command]
async fn call(method: String, args: Value) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| hearth_tools::call(&method, &args))).unwrap_or_else(|_| Err("Something went wrong inside the tool.".into()))
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
fn start(app: tauri::AppHandle, job: String, args: Value) -> Result<u64, String> {
    let id = NEXT.fetch_add(1, Ordering::SeqCst);
    let flag = Arc::new(AtomicBool::new(false));
    JOBS.lock().map_err(|_| "busy")?.get_or_insert_with(HashMap::new).insert(id, flag.clone());
    std::thread::spawn(move || {
        let a2 = app.clone();
        let emit = move |v: Value| { let _ = a2.emit("tools://job", json!({"id": id, "kind": "step", "data": v})); };
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| hearth_tools::job(&job, &args, &flag, &emit)))
            .unwrap_or_else(|_| Err("Something went wrong inside the tool.".into()));
        if let Ok(mut j) = JOBS.lock() { if let Some(m) = j.as_mut() { m.remove(&id); } }
        let _ = app.emit("tools://job", match r {
            Ok(v) => json!({"id": id, "kind": "done", "ok": true, "result": v}),
            Err(e) => json!({"id": id, "kind": "done", "ok": false, "error": e}),
        });
    });
    Ok(id)
}

#[tauri::command]
fn cancel(id: u64) {
    if let Ok(j) = JOBS.lock() { if let Some(f) = j.as_ref().and_then(|m| m.get(&id)) { f.store(true, Ordering::SeqCst); } }
}

fn usage() -> String {
    let names: Vec<&str> = hearth_tools::APPS.iter().map(|a| a.0).collect();
    format!("hearth-tools: Hearth OS's system tools\n\nUsage: hearth-tools --app <{}> [--fullscreen] [--page help]\n       hearth-tools --call <method> [json]   (no window: print the answer)\n       hearth-tools --list-apps", names.join("|"))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let opt = |k: &str| args.iter().position(|a| a == k).and_then(|i| args.get(i + 1)).cloned();
    if args.iter().any(|a| a == "-h" || a == "--help") { println!("{}", usage()); return; }
    if args.iter().any(|a| a == "--version" || a == "-V") { println!("hearth-tools {}", env!("CARGO_PKG_VERSION")); return; }
    if args.iter().any(|a| a == "--list-apps") { for a in hearth_tools::APPS { println!("{}\t{}", a.0, a.1); } return; }
    if let Some(m) = opt("--call") {
        let j = args.iter().position(|a| a == "--call").and_then(|i| args.get(i + 2)).cloned().unwrap_or_default();
        std::process::exit(hearth_tools::cli_call(&m, &j));
    }
    let name = opt("--app").unwrap_or_else(|| "welcome".into());
    let Some((id, title, page)) = hearth_tools::app(&name) else { eprintln!("unknown app {name}\n\n{}", usage()); std::process::exit(2); };
    // The first-run tour: once per account.
    if args.iter().any(|a| a == "--first-run") && hearth_tools::welcome::marker().exists() { return; }
    let fullscreen = args.iter().any(|a| a == "--fullscreen");
    let boot = json!({"app": id, "title": title, "page": opt("--page").unwrap_or_default(), "firstRun": args.iter().any(|a| a == "--first-run"), "fullscreen": fullscreen});
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() { std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1"); }
    let mut ctx = tauri::generate_context!();
    // Each tool its own app id (Wayland), so the taskbar tells them apart; the .desktop files' StartupWMClass match.
    ctx.config_mut().identifier = format!("io.github.dragongamer9.hearth.{}", id.replace('-', ""));
    let (w, h) = match id { "system-info" => (1000.0, 760.0), "welcome" => (1060.0, 720.0), "passwords" => (1060.0, 720.0), "benchmark" => (1180.0, 760.0), _ => (1240.0, 800.0) };
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![call, start, cancel])
        .setup(move |app| {
            WebviewWindowBuilder::new(app, "main", WebviewUrl::App(page.into()))
                .title(title)
                .inner_size(w, h)
                .min_inner_size(560.0, 440.0)
                .fullscreen(fullscreen)
                .background_color(tauri::window::Color(15, 11, 9, 255))
                .initialization_script(&format!("window.__HEARTH__ = {boot};"))
                .build()?;
            Ok(())
        })
        .run(ctx)
        .expect("Hearth Tools couldn't start");
}
