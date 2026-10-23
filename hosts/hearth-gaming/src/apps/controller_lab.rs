// Controller Lab's window (Tauri: WebKitGTK on Linux). The page (ui/controller-lab.html) asks for every
// controller's state about 30 times a second; the reading is in the library (pads.rs).
//
//   hearth-controller-lab               the window (--fullscreen in Game Mode)
//   hearth-controller-lab --dump [ms]   what it sees after [ms] (default 1000), as JSON, without a window
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde_json::{json, Value};

#[cfg(target_os = "linux")]
mod lab {
    use super::*;
    use hearth_gaming::pads::Lab;
    use std::sync::OnceLock;

    static LAB: OnceLock<Lab> = OnceLock::new();
    pub fn lab() -> &'static Lab { LAB.get_or_init(Lab::start) }

    pub fn state() -> Value { lab().snapshot() }
    pub fn rumble(id: &str, strong: f64, weak: f64, ms: u32) -> Result<(), String> { lab().rumble(id, strong, weak, ms) }
    pub fn set_axis(id: &str, code: u16, min: i32, max: i32, flat: i32) -> Result<Value, String> {
        lab().set_axis(id, code, min, max, flat).map(|a| json!({"min": a.min, "max": a.max, "flat": a.flat}))
    }
    pub fn reset(id: &str) -> Result<usize, String> { lab().reset(id) }

    // ---- Hearth Input's dead zones: the sticks in games' custom controller layouts (~/.config/nimbus/input).
    fn decode(name: &str) -> String {
        let b = name.as_bytes();
        let mut out = Vec::new();
        let mut i = 0;
        while i < b.len() {
            if b[i] == b'%' && i + 2 < b.len() {
                if let Some(v) = std::str::from_utf8(&b[i + 1..i + 3]).ok().and_then(|h| u8::from_str_radix(h, 16).ok()) { out.push(v); i += 3; continue; }
            }
            out.push(b[i]);
            i += 1;
        }
        String::from_utf8_lossy(&out).to_string()
    }
    fn stick_dzs(custom: &Value) -> Vec<f64> {
        let mut v = Vec::new();
        for path in [&custom["sticks"], &custom["shift"]["sticks"]] {
            if let Some(m) = path.as_object() { for s in m.values() { if let Some(d) = s["deadzone"].as_f64() { v.push(d); } } }
        }
        v
    }
    pub fn layouts() -> Value {
        let dir = hearth_core::input_map::dir();
        let mut out = Vec::new();
        if let Ok(rd) = std::fs::read_dir(&dir) {
            for e in rd.flatten() {
                let n = e.file_name().to_string_lossy().to_string();
                let Some(stem) = n.strip_suffix(".json") else { continue };
                let id = decode(stem);
                let c = hearth_core::input_map::load_in(&dir, &id);
                let custom = c["layout"] == "custom";
                out.push(json!({"id": id, "layout": c["layout"], "custom": custom, "deadzones": if custom { stick_dzs(&c["custom"]) } else { vec![] }}));
            }
        }
        json!({"dir": dir, "games": out})
    }
    /// Sets the dead zone of every stick that has one (mouse and scrolling sticks) in games' custom layouts.
    pub fn apply_deadzone(dz: f64) -> Result<Value, String> {
        if !(0.0..=0.5).contains(&dz) { return Err("A dead zone is between 0% and 50%.".into()); }
        let dz = (dz * 100.0).round() / 100.0;
        let dir = hearth_core::input_map::dir();
        let (mut changed, mut failed) = (Vec::new(), Vec::new());
        for g in layouts()["games"].as_array().cloned().unwrap_or_default() {
            if g["custom"] != true { continue; }
            let id = g["id"].as_str().unwrap_or("").to_string();
            let mut c = hearth_core::input_map::load_in(&dir, &id);
            let mut touched = false;
            for path in ["/custom/sticks", "/custom/shift/sticks"] {
                if let Some(m) = c.pointer_mut(path).and_then(|n| n.as_object_mut()) {
                    for s in m.values_mut() {
                        if s.get("deadzone").is_some() {
                            s["deadzone"] = json!(dz);
                            if let Some(o) = s["outer"].as_f64() { if o < dz + 0.1 { s["outer"] = json!(((dz + 0.1) * 100.0).round() / 100.0); } }
                            touched = true;
                        }
                    }
                }
            }
            if !touched { continue; }
            match hearth_core::input_map::save_in(&dir, &id, Some("custom"), Some(&c["custom"]), None) {
                Ok(_) => changed.push(id),
                Err(e) => failed.push(format!("{id}: {e}")),
            }
        }
        Ok(json!({"changed": changed, "failed": failed, "deadzone": dz}))
    }
}

#[cfg(not(target_os = "linux"))]
mod lab {
    use super::*;
    pub fn state() -> Value { json!({"pads": [], "denied": [], "unsupported": true}) }
    pub fn rumble(_: &str, _: f64, _: f64, _: u32) -> Result<(), String> { Err("Not on this system yet.".into()) }
    pub fn set_axis(_: &str, _: u16, _: i32, _: i32, _: i32) -> Result<Value, String> { Err("Not on this system yet.".into()) }
    pub fn reset(_: &str) -> Result<usize, String> { Ok(0) }
    pub fn layouts() -> Value { json!({"games": []}) }
    pub fn apply_deadzone(_: f64) -> Result<Value, String> { Err("Not on this system yet.".into()) }
}

#[tauri::command]
fn state() -> Value { lab::state() }
#[tauri::command]
fn rumble(id: String, strong: f64, weak: f64, ms: u32) -> Result<(), String> { lab::rumble(&id, strong, weak, ms) }
#[tauri::command]
fn set_axis(id: String, code: u16, min: i32, max: i32, flat: i32) -> Result<Value, String> { lab::set_axis(&id, code, min, max, flat) }
#[tauri::command]
fn reset(id: String) -> Result<usize, String> { lab::reset(&id) }
#[tauri::command]
async fn layouts() -> Value { tauri::async_runtime::spawn_blocking(lab::layouts).await.unwrap_or(Value::Null) }
#[tauri::command]
async fn apply_deadzone(dz: f64) -> Result<Value, String> { tauri::async_runtime::spawn_blocking(move || lab::apply_deadzone(dz)).await.map_err(|e| e.to_string())? }
#[tauri::command]
fn info() -> Value { json!({"version": env!("CARGO_PKG_VERSION"), "host": if cfg!(windows) { "windows" } else { "linux" }}) }

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let Some(i) = args.iter().position(|a| a == "--dump") {
        let ms: u64 = args.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(1000);
        let _ = lab::state();
        std::thread::sleep(std::time::Duration::from_millis(ms));
        println!("{}", serde_json::to_string_pretty(&lab::state()).unwrap_or_default());
        return;
    }
    let fullscreen = args.iter().any(|a| a == "--fullscreen");
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() { std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1"); }
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![state, rumble, set_axis, reset, layouts, apply_deadzone, info])
        .setup(move |app| {
            tauri::WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::App("controller-lab.html".into()))
                .title("Controller Lab")
                .inner_size(1280.0, 800.0)
                .min_inner_size(640.0, 480.0)
                .fullscreen(fullscreen)
                .background_color(tauri::window::Color(15, 11, 9, 255))
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Controller Lab couldn't start");
}
