//! Hearth Calculator: Hearth OS's own calculator. The window is ui/index.html (drawn by WebKitGTK on Linux,
//! WebView2 on Windows); all the arithmetic is in ui/engine.js (decimal numbers, tested by tests/engine.test.js).
//! This side keeps the history, memory and chosen mode (~/.config/hearth-calculator/state.json) and reaches the
//! clipboard.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde_json::Value;
use std::path::PathBuf;
use tauri::{WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_clipboard_manager::ClipboardExt;

fn state_file() -> PathBuf {
    std::env::var_os("HEARTH_CALCULATOR_CONFIG").map(PathBuf::from)
        .unwrap_or_else(|| dirs::config_dir().unwrap_or_else(std::env::temp_dir).join("hearth-calculator"))
        .join("state.json")
}

#[tauri::command(async)]
fn store_get() -> Value {
    std::fs::read(state_file()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null)
}

/// Saved through a temporary file, so a crash never leaves half a file.
#[tauri::command(async)]
fn store_set(value: Value) -> Result<(), String> {
    let f = state_file();
    if let Some(d) = f.parent() { std::fs::create_dir_all(d).map_err(|e| e.to_string())?; }
    let tmp = f.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec(&value).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &f).map_err(|e| e.to_string())
}

#[tauri::command(async)]
fn clipboard_read(app: tauri::AppHandle) -> String { app.clipboard().read_text().unwrap_or_default() }

#[tauri::command(async)]
fn clipboard_write(app: tauri::AppHandle, text: String) -> Result<(), String> { app.clipboard().write_text(text).map_err(|e| e.to_string()) }

/// WebKit's newer drawing path shows a black window on older Nvidia drivers and on basic graphics (the same rule
/// as Hearth Terminal and the Hearth app).
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

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--version") { println!("hearth-calculator {}", env!("CARGO_PKG_VERSION")); return; }
    if args.iter().any(|a| a == "-h" || a == "--help") { println!("Hearth Calculator\n\nUsage: hearth-calculator [--fullscreen] [--version]"); return; }
    let full = args.iter().any(|a| a == "--fullscreen");
    #[cfg(target_os = "linux")]
    pick_drawing();
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .invoke_handler(tauri::generate_handler![store_get, store_set, clipboard_read, clipboard_write])
        .setup(move |app| {
            let w = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title("Calculator")
                .inner_size(820.0, 640.0)
                .min_inner_size(340.0, 480.0)
                .fullscreen(full)
                .background_color(tauri::window::Color(15, 11, 9, 255))
                .build()?;
            let _ = w.set_focus();
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("couldn't start Hearth Calculator");
}
