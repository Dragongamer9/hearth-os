//! The window: ui/index.html (WebKitGTK), packages dropped on it or picked, converted one at a time, with the
//! AppImages made so far listed (open, show in folder, add to or take out of the app menu, delete).
//!
//! Saved: ~/.config/hearth-appimage/settings.json (where AppImages go, fetch libraries, add to menu: the page's
//! settings, merged on write) and made.json (the AppImages made, written only from here). Both through appfile
//! (atomic, .bak, never defaults over a damaged file).

#[path = "../../hearth-apptheme/apptheme.rs"]
mod apptheme;

use crate::appfile;
use crate::convert::{self, appdir::Desktop, menu, Event};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use tauri::{DragDropEvent, Emitter, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use tauri_plugin_dialog::DialogExt;

fn config_dir() -> PathBuf {
    std::env::var_os("HEARTH_APPIMAGE_CONFIG").map(PathBuf::from)
        .unwrap_or_else(|| dirs::config_dir().unwrap_or_else(std::env::temp_dir).join("hearth-appimage"))
}
fn settings_file() -> PathBuf { config_dir().join("settings.json") }
fn made_file() -> PathBuf { config_dir().join("made.json") }

static SEEN: appfile::Seen = std::sync::Mutex::new(None);
static START_FILES: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

#[tauri::command(async)]
fn store_get() -> Value {
    let mut v = appfile::read_seen(&settings_file(), &SEEN).unwrap_or(Value::Null);
    if !v.is_object() { v = json!({}); }
    v["defaultOut"] = json!(crate::default_out().display().to_string());
    v
}

#[tauri::command(async)]
fn store_set(value: Value) -> Result<(), String> {
    let mut value = value;
    if let Some(m) = value.as_object_mut() { m.remove("defaultOut"); }
    appfile::save_changed(&settings_file(), &SEEN, &value).map_err(|e| e.to_string())
}

/// Files given on the command line (hearth-appimage FILE..., or Open with), once.
#[tauri::command]
fn start_files() -> Vec<String> { std::mem::take(&mut *START_FILES.lock().unwrap_or_else(|e| e.into_inner())) }

/// The AppImages made, newest first; each says whether its file and menu entry are still there.
#[tauri::command(async)]
fn made_list() -> Value {
    let list = appfile::read_json(&made_file()).and_then(|v| v.as_array().cloned()).unwrap_or_default();
    let list: Vec<Value> = list.into_iter().map(|mut e| {
        let out = e["output"].as_str().unwrap_or("").to_string();
        e["exists"] = json!(Path::new(&out).is_file());
        let entry = e["menu_entry"].as_str().unwrap_or("").to_string();
        e["in_menu"] = json!(!entry.is_empty() && Path::new(&entry).is_file());
        e
    }).collect();
    Value::Array(list)
}

fn made_update(change: impl FnOnce(&mut Vec<Value>)) -> Result<(), String> {
    appfile::update(&made_file(), |cur| {
        let mut list = cur.and_then(|v| v.as_array().cloned()).unwrap_or_default();
        change(&mut list);
        (Some(Value::Array(list)), ())
    }).map_err(|e| e.to_string())
}

#[tauri::command(async)]
fn pick_files(app: tauri::AppHandle) -> Vec<String> {
    app.dialog().file()
        .set_title("Pick apps to turn into AppImages")
        .add_filter("Apps (.deb, .rpm, .tar, .zip)", &["deb", "rpm", "gz", "xz", "zst", "bz2", "tgz", "txz", "tar", "zip"])
        .blocking_pick_files()
        .unwrap_or_default()
        .into_iter().filter_map(|f| f.into_path().ok()).map(|p| p.display().to_string()).collect()
}

#[tauri::command(async)]
fn pick_folder(app: tauri::AppHandle) -> Option<String> {
    app.dialog().file().set_title("Where AppImages go").blocking_pick_folder()
        .and_then(|f| f.into_path().ok()).map(|p| p.display().to_string())
}

/// One conversion. Progress goes to the page as `progress` events ({job, step|log}); the report comes back and is
/// added to made.json.
#[tauri::command(async)]
fn convert_one(app: tauri::AppHandle, job: u32, path: String, out_dir: String, fetch: bool, menu: bool) -> Result<Value, String> {
    let out = if out_dir.trim().is_empty() { crate::default_out() } else { PathBuf::from(shellexpand_home(&out_dir)) };
    let opt = convert::Options { out_dir: out, fetch, bundle: true, menu };
    let mut on = |e: Event| {
        let p = match e { Event::Step(s) => json!({"job": job, "step": s}), Event::Log(s) => json!({"job": job, "log": s}) };
        let _ = app.emit("progress", p);
    };
    let r = convert::convert(Path::new(&path), &opt, &mut on).map_err(|e| e.to_string())?;
    let mut v = serde_json::to_value(&r).map_err(|e| e.to_string())?;
    v["made"] = json!(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0));
    let entry = v.clone();
    made_update(|list| {
        list.retain(|e| e["output"] != entry["output"]);
        list.insert(0, entry);
        list.truncate(200);
    })?;
    Ok(v)
}

fn shellexpand_home(p: &str) -> String {
    match (p.strip_prefix("~/"), dirs::home_dir()) { (Some(rest), Some(h)) => h.join(rest).display().to_string(), _ => p.to_string() }
}

fn fuse_ok() -> bool {
    ["/usr/bin/fusermount3", "/bin/fusermount3", "/usr/bin/fusermount", "/bin/fusermount"].iter().any(|p| Path::new(p).exists())
        && Path::new("/dev/fuse").exists()
}

#[tauri::command(async)]
fn open_app(path: String) -> Result<(), String> {
    let mut c = std::process::Command::new(&path);
    // Without FUSE the AppImage unpacks itself to run (slower to start, works the same).
    if !fuse_ok() { c.env("APPIMAGE_EXTRACT_AND_RUN", "1"); }
    c.stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
    c.spawn().map(|_| ()).map_err(|e| format!("couldn't start it: {e}"))
}

#[tauri::command(async)]
fn show_in_folder(path: String) -> Result<(), String> {
    let dir = Path::new(&path).parent().map(Path::to_path_buf).unwrap_or_else(crate::default_out);
    std::process::Command::new("xdg-open").arg(dir).spawn().map(|_| ()).map_err(|e| e.to_string())
}

#[tauri::command(async)]
fn add_menu(output: String) -> Result<String, String> {
    let list = appfile::read_json(&made_file()).and_then(|v| v.as_array().cloned()).unwrap_or_default();
    let e = list.iter().find(|e| e["output"] == json!(output)).ok_or("it isn't in the list")?;
    let d = Desktop::parse(e["desktop"].as_str().unwrap_or(""));
    let id = e["id"].as_str().filter(|s| !s.is_empty()).map(String::from).unwrap_or_else(|| convert::appdir::slug(e["name"].as_str().unwrap_or("app")));
    let icon = PathBuf::from(e["icon"].as_str().unwrap_or(""));
    let p = menu::add(Path::new(&output), &d, &id, &icon).map_err(|e| e.to_string())?;
    let ps = p.display().to_string();
    made_update(|list| for e in list.iter_mut() { if e["output"] == json!(output) { e["menu_entry"] = json!(ps.clone()); } })?;
    Ok(ps)
}

#[tauri::command(async)]
fn remove_menu(output: String) -> Result<(), String> {
    let list = appfile::read_json(&made_file()).and_then(|v| v.as_array().cloned()).unwrap_or_default();
    if let Some(e) = list.iter().find(|e| e["output"] == json!(output)) {
        let entry = e["menu_entry"].as_str().unwrap_or("");
        if !entry.is_empty() { menu::remove(Path::new(entry)).map_err(|e| e.to_string())?; }
    }
    made_update(|list| for e in list.iter_mut() { if e["output"] == json!(output) { e["menu_entry"] = json!(""); } })
}

/// Out of the list; with `delete_file`, the AppImage (only one this app made, in the list) and its menu entry too.
#[tauri::command(async)]
fn forget(output: String, delete_file: bool) -> Result<(), String> {
    let list = appfile::read_json(&made_file()).and_then(|v| v.as_array().cloned()).unwrap_or_default();
    let Some(e) = list.iter().find(|e| e["output"] == json!(output)) else { return Ok(()) };
    if delete_file {
        let entry = e["menu_entry"].as_str().unwrap_or("");
        if !entry.is_empty() { let _ = menu::remove(Path::new(entry)); }
        if output.ends_with(".AppImage") && Path::new(&output).is_file() { std::fs::remove_file(&output).map_err(|e| e.to_string())?; }
    }
    made_update(|list| list.retain(|e| e["output"] != json!(output)))
}

/// An icon AppImage Maker kept (data/hearth-appimage/icons), as a data: URL for the page.
#[tauri::command(async)]
fn icon_data(path: String) -> Option<String> {
    let base = dirs::data_dir()?.join("hearth-appimage/icons").canonicalize().ok()?;
    let real = Path::new(&path).canonicalize().ok()?;
    if !real.starts_with(&base) { return None; }
    let bytes = std::fs::read(&real).ok()?;
    if bytes.len() > 4 << 20 { return None; }
    let mime = if path.ends_with(".svg") { "image/svg+xml" } else { "image/png" };
    Some(format!("data:{mime};base64,{}", b64(&bytes)))
}

fn b64(b: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::with_capacity(b.len() / 3 * 4 + 4);
    for c in b.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        s.push(T[(n >> 18) as usize & 63] as char);
        s.push(T[(n >> 12) as usize & 63] as char);
        s.push(if c.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        s.push(if c.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    s
}

/// WebKit's newer drawing path shows a black window on Nvidia and basic graphics (the rule Hearth's apps share).
fn pick_drawing() {
    const VAR: &str = "WEBKIT_DISABLE_DMABUF_RENDERER";
    if std::env::var_os(VAR).is_some() { return; }
    let fallback = dirs::config_dir().map(|d| d.join("nimbus").join("drawing-fallback").exists()).unwrap_or(false)
        || std::fs::read_to_string("/proc/cmdline").map(|c| c.split_whitespace().any(|w| w == "nomodeset")).unwrap_or(false);
    let nvidia = std::fs::read_dir("/sys/class/drm").into_iter().flatten().flatten()
        .any(|e| std::fs::read_to_string(e.path().join("device/vendor")).map(|v| v.trim() == "0x10de").unwrap_or(false));
    if fallback || nvidia { std::env::set_var(VAR, "1"); }
}

pub fn run(files: Vec<String>, full: bool) {
    *START_FILES.lock().unwrap_or_else(|e| e.into_inner()) = files.into_iter()
        .map(|f| std::fs::canonicalize(&f).map(|p| p.display().to_string()).unwrap_or(f)).collect();
    pick_drawing();
    tauri::Builder::default()
        .plugin(apptheme::plugin())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![store_get, store_set, start_files, made_list, pick_files, pick_folder,
            convert_one, open_app, show_in_folder, add_menu, remove_menu, forget, icon_data])
        .setup(move |app| {
            let w = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title("AppImage Maker")
                .inner_size(980.0, 680.0)
                .min_inner_size(520.0, 460.0)
                .fullscreen(full)
                .background_color(apptheme::bg())
                .build()?;
            let _ = w.set_focus();
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::DragDrop(d) = event {
                let msg = match d {
                    DragDropEvent::Enter { .. } | DragDropEvent::Over { .. } => json!({"over": true}),
                    DragDropEvent::Leave => json!({"over": false}),
                    DragDropEvent::Drop { paths, .. } => json!({"files": paths.iter().map(|p| p.display().to_string()).collect::<Vec<_>>()}),
                    _ => return,
                };
                let _ = window.app_handle().emit("drop", msg);
            }
        })
        .run(tauri::generate_context!())
        .expect("couldn't start AppImage Maker");
}
