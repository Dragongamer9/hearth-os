// Hearth's emoji and symbol picker (Tauri: WebKitGTK on Linux). Super+. in Desktop Mode opens it over the app you're
// in (labwc's rc.xml); the page (ui/index.html, the list in ui/emoji-data.js) searches and shows them, and a pick is
// copied and typed into that app once this window has closed (lib.rs). Works with the keyboard, mouse, touch and a
// controller.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde_json::{json, Value};
use tauri::{WebviewUrl, WebviewWindowBuilder};

#[tauri::command]
fn init() -> Value {
    let s = hearth_emoji::load();
    json!({"recent": s.recent, "tone": s.tone, "platform": if cfg!(windows) { "windows" } else { "linux" }})
}

#[tauri::command]
fn set_tone(tone: u8) -> Result<(), String> {
    let mut s = hearth_emoji::load();
    s.tone = tone.min(5);
    hearth_emoji::save(&s)
}

/// The pick: remembered, copied, and (unless `copy_only`) typed into the app you were in after the window closes.
#[tauri::command]
fn pick(app: tauri::AppHandle, text: String, copy_only: bool) -> Result<Value, String> {
    if !hearth_emoji::valid_pick(&text) { return Err("That can't be typed.".into()); }
    let s = hearth_emoji::remember(hearth_emoji::load(), &text);
    let _ = hearth_emoji::save(&s);
    let copied = hearth_emoji::copy(&text).is_ok();
    let typing = !copy_only && hearth_emoji::type_later(&text);
    if !copied && !typing { return Err("Neither typing (wtype) nor copying (wl-copy) works here.".into()); }
    // Closing hands the keyboard back to the app you were in; the typing waits for that.
    let a = app.clone();
    std::thread::spawn(move || { std::thread::sleep(std::time::Duration::from_millis(60)); a.exit(0); });
    Ok(json!({"copied": copied, "typed": typing}))
}

#[tauri::command]
fn close(app: tauri::AppHandle) { app.exit(0); }

fn main() {
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() { std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1"); }
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![init, set_tone, pick, close])
        .setup(|app| {
            WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title("Emoji")
                .inner_size(760.0, 540.0)
                .min_inner_size(420.0, 360.0)
                .center()
                .always_on_top(true)
                .skip_taskbar(true)
                .background_color(tauri::window::Color(15, 11, 9, 255))
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("The emoji picker couldn't start");
}
