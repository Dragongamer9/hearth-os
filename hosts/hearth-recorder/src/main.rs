// Hearth Recorder: its window (Tauri: WebKitGTK on Linux), and the commands Mantel's shortcuts and Hearth's Quick
// Access use. The work is in the library (lib.rs and the files it names).
//   hearth-recorder                       the window (--sound opens it on the sound recorder, --fullscreen)
//   hearth-recorder --toggle [--area]     start recording the screen (or an area: drag a box), or stop and save
//   hearth-recorder --start [--area[=X,Y WxH]] | --stop
//   hearth-recorder --replay              instant replay on / off
//   hearth-recorder --save-replay         save the last stretch (turns instant replay on when it's off)
//   hearth-recorder --sound-toggle        the sound recorder: start, or stop and save
//   hearth-recorder --status | --probe [--fresh]
// --json prints the answer as JSON (Quick Access reads it) and leaves out the "Recording" notification.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use hearth_recorder::plan::{Kind, Rect};
use hearth_recorder::{control, probe, session, settings};
use serde_json::{json, Value};
use tauri::{WebviewUrl, WebviewWindowBuilder};

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> Result<T, String> + Send + 'static) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| e.to_string())?
}

#[tauri::command]
fn status() -> Value { hearth_recorder::status() }

#[tauri::command]
async fn files() -> Result<Value, String> { blocking(|| Ok(hearth_recorder::files())).await }

#[tauri::command]
async fn environment(fresh: bool) -> Result<Value, String> {
    blocking(move || Ok(serde_json::to_value(probe::Env::detect(fresh)).unwrap_or(Value::Null))).await
}

#[tauri::command]
async fn start(window: tauri::WebviewWindow, kind: String, area: bool) -> Result<Value, String> {
    let k = match kind.as_str() { "replay" => Kind::Replay, "sound" => Kind::Sound, _ => Kind::Record };
    // Out of the way before the screen is recorded (and before choosing an area).
    let hide = k != Kind::Sound;
    if hide { let _ = window.minimize(); }
    let w2 = window.clone();
    let r = blocking(move || {
        if hide { std::thread::sleep(std::time::Duration::from_millis(450)); }
        let rect = if area { match hearth_recorder::choose_area()? { Some(r) => Some(r), None => return Ok(Value::Null) } } else { None };
        hearth_recorder::start(k, rect, false).map(|s| serde_json::to_value(s).unwrap_or(Value::Null))
    }).await;
    // Back if it didn't start (or the area was cancelled): there's something to read.
    if hide && !matches!(&r, Ok(v) if !v.is_null()) { let _ = w2.unminimize(); let _ = w2.set_focus(); }
    r
}

#[tauri::command]
async fn stop() -> Result<Value, String> { blocking(hearth_recorder::stop).await }

#[tauri::command]
async fn save_replay() -> Result<String, String> { blocking(hearth_recorder::save_replay).await }

#[tauri::command]
fn set_settings(s: settings::Settings) -> Result<settings::Settings, String> { settings::save(&s) }

#[tauri::command]
fn open(path: String) -> Result<(), String> {
    let p = hearth_media::files::inside(&path, &hearth_recorder::dirs()).ok_or("That file isn't one of Recorder's")?;
    hearth_media::notify::open(&p)
}

#[tauri::command]
fn show(path: String) -> Result<(), String> {
    let p = hearth_media::files::inside(&path, &hearth_recorder::dirs()).ok_or("That file isn't one of Recorder's")?;
    hearth_media::notify::show(&p)
}

#[tauri::command]
fn delete(path: String) -> Result<(), String> { hearth_media::files::delete(&path, &hearth_recorder::dirs()) }

#[tauri::command]
fn rename(path: String, name: String) -> Result<String, String> {
    hearth_media::files::rename(&path, &name, &hearth_recorder::dirs()).map(|p| p.to_string_lossy().to_string())
}

/// The command-line side: prints the answer and exits.
fn cli(args: &[String]) -> Option<i32> {
    let has = |f: &str| args.iter().any(|a| a == f);
    let json_out = has("--json");
    if let Some(i) = args.iter().position(|a| a == "--session") {
        return Some(match args.get(i + 1).and_then(|s| serde_json::from_str(s).ok()) {
            Some(req) => session::run(req),
            None => { eprintln!("--session needs a recording request"); 2 }
        });
    }
    // --area (drag a box with slurp) or --area=X,Y WxH
    let area = || -> Result<Option<Rect>, String> {
        match args.iter().find(|a| a.starts_with("--area")) {
            None => Ok(None),
            Some(a) => match a.strip_prefix("--area=") {
                Some(g) => Rect::parse(g).map(Some).ok_or_else(|| format!("{g} isn't an area (X,Y WxH)")),
                None => hearth_recorder::choose_area()?.map(Some).ok_or_else(|| "No area chosen".to_string()),
            },
        }
    };
    let announce = !json_out;
    let r: Result<Value, String> = if has("--toggle") {
        // Stopping never asks for an area first.
        if control::current().map(|s| s.kind == "record").unwrap_or(false) { hearth_recorder::stop() }
        else { area().and_then(|a| hearth_recorder::toggle(a, announce)) }
    } else if has("--start") {
        area().and_then(|a| hearth_recorder::start(Kind::Record, a, announce).map(|s| json!({"ok": true, "started": s})))
    } else if has("--stop") {
        hearth_recorder::stop()
    } else if has("--replay") {
        match control::current() {
            Some(s) if s.kind == "replay" => hearth_recorder::stop(),
            _ => hearth_recorder::start(Kind::Replay, None, announce).map(|s| json!({"ok": true, "started": s})),
        }
    } else if has("--save-replay") {
        hearth_recorder::replay_save_or_start(announce)
    } else if has("--sound-toggle") {
        match control::current() {
            Some(s) if s.kind == "sound" => hearth_recorder::stop(),
            _ => hearth_recorder::start(Kind::Sound, None, announce).map(|s| json!({"ok": true, "started": s})),
        }
    } else if has("--status") {
        Ok(hearth_recorder::status())
    } else if has("--probe") {
        Ok(serde_json::to_value(probe::Env::detect(has("--fresh"))).unwrap_or(Value::Null))
    } else {
        return None;
    };
    match r {
        Ok(v) => {
            if json_out || has("--status") || has("--probe") { println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default()); }
            else if let Some(f) = v["file"].as_str().or(v["saved"].as_str()) { println!("Saved {f}"); }
            else if v["started"].is_object() { println!("Recording ({})", v["started"]["encoder"].as_str().unwrap_or("")); }
            Some(0)
        }
        Err(e) => {
            if json_out { println!("{}", json!({"ok": false, "error": e})); } else { eprintln!("{e}"); }
            Some(1)
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let Some(code) = cli(&args) { std::process::exit(code); }
    let fullscreen = args.iter().any(|a| a == "--fullscreen");
    let tab = if args.iter().any(|a| a == "--sound") { "sound" } else { "screen" };
    // WebKit's newer GPU path draws a black window on some Nvidia drivers; Recorder doesn't need its speed.
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() { std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1"); }
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![status, files, environment, start, stop, save_replay, set_settings, open, show, delete, rename])
        .setup(move |app| {
            WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .initialization_script(&format!("window.HEARTH_TAB = '{tab}';"))
                .title("Recorder")
                .inner_size(1040.0, 720.0)
                .min_inner_size(560.0, 460.0)
                .fullscreen(fullscreen)
                .background_color(tauri::window::Color(15, 11, 9, 255))
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Recorder couldn't start");
}
