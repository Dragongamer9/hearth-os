// Hearth Theater's windows (Tauri: WebKitGTK on Linux, WebView2 on Windows). The main window (ui/index.html) is your
// wall of saved sites: paste a link, and Theater reads the page's name, preview picture and icon to make a tile.
// Choosing a tile opens the site itself in the viewer: a second, full-screen window with a controller pointer laid
// over the page (viewer.js). Theater never takes videos out of a site; what plays, plays on the site's own page.
// `hearth-theater --fullscreen` starts full screen (how Game Mode opens it); `--list` prints the saved sites as JSON.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use base64::Engine;
use hearth_theater as th;
use serde_json::{json, Value};
use std::io::Read;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

const VIEWER_JS: &str = include_str!("viewer.js");
/// The viewer's way of saying "back to Theater": it goes to this address, and the viewer window closes instead.
const CLOSE_URL: &str = "hearth-theater://close";

struct State {
    dir: PathBuf,
    sites: Mutex<Vec<th::Site>>,
    fullscreen: bool,
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(8))
        .timeout(Duration::from_secs(15))
        .redirects(6)
        .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/605.1.15 (KHTML, like Gecko) HearthTheater/1.0")
        .build()
}

/// A page's head (at most MAX_PAGE_BYTES), and the address it ended up at after redirects.
fn fetch_page(url: &str) -> Result<(String, String), String> {
    let r = agent().get(url).call().map_err(|e| match e {
        ureq::Error::Status(code, _) => format!("The site answered with an error ({code})"),
        e => format!("Couldn't reach the site ({})", e.kind()),
    })?;
    let final_url = r.get_url().to_string();
    let mut body = Vec::new();
    r.into_reader().take(th::MAX_PAGE_BYTES).read_to_end(&mut body).map_err(|e| e.to_string())?;
    Ok((String::from_utf8_lossy(&body).into_owned(), final_url))
}

/// Saves a picture into the art folder; returns its file name ("" when it couldn't be had).
fn fetch_art(dir: &std::path::Path, id: &str, kind: &str, url: &str) -> String {
    if url.is_empty() { return String::new(); }
    let Ok(r) = agent().get(url).call() else { return String::new() };
    let ct = r.content_type().to_string();
    if !ct.starts_with("image/") && !url.ends_with(".ico") { return String::new(); }
    let mut b = Vec::new();
    if r.into_reader().take(th::MAX_ART_BYTES + 1).read_to_end(&mut b).is_err() || b.is_empty() || b.len() as u64 > th::MAX_ART_BYTES {
        return String::new();
    }
    let name = th::art_name(id, kind, &ct, url);
    let art = th::art_dir(dir);
    if std::fs::create_dir_all(&art).is_err() { return String::new(); }
    // An older picture of this site with another extension goes.
    if let Ok(rd) = std::fs::read_dir(&art) {
        for e in rd.flatten() {
            let n = e.file_name().to_string_lossy().to_string();
            if n.starts_with(&format!("{id}-{kind}.")) && n != name { let _ = std::fs::remove_file(e.path()); }
        }
    }
    if std::fs::write(art.join(&name), &b).is_ok() { name } else { String::new() }
}

#[tauri::command]
fn init(st: tauri::State<State>) -> Value {
    let sites = st.sites.lock().unwrap().clone();
    json!({"sites": sites, "categories": th::CATEGORIES, "fullscreen": st.fullscreen,
           "platform": if cfg!(windows) { "windows" } else { "linux" }})
}

/// What a link's page is called and looks like, for the Add dialog to fill in the name.
#[tauri::command(async)]
fn preview(url: String) -> Result<Value, String> {
    let url = th::normalize_url(&url)?;
    let (html, at) = fetch_page(&url)?;
    let m = th::parse_meta(&html, &at);
    Ok(json!({"url": url, "title": m.title, "image": m.image, "host": th::host_name(&url)}))
}

#[tauri::command]
fn add(url: String, name: String, category: String, st: tauri::State<State>) -> Result<Value, String> {
    let mut sites = st.sites.lock().unwrap();
    let s = th::add(&mut sites, &url, &name, &category)?;
    th::save(&st.dir, &sites)?;
    Ok(json!(s))
}

/// Reads the site's page again for its picture and icon (and its name, if you didn't give it one).
#[tauri::command(async)]
fn refresh(id: String, app: AppHandle) -> Result<Value, String> {
    let st = app.state::<State>();
    let (url, named) = {
        let sites = st.sites.lock().unwrap();
        let s = sites.iter().find(|s| s.id == id).ok_or("That site isn't in Theater")?;
        (s.url.clone(), s.name != th::host_name(&s.url))
    };
    let (html, at) = fetch_page(&url)?;
    let m = th::parse_meta(&html, &at);
    let art = fetch_art(&st.dir, &id, "art", &m.image);
    let icon = fetch_art(&st.dir, &id, "icon", &m.icon);
    let mut sites = st.sites.lock().unwrap();
    let s = sites.iter_mut().find(|s| s.id == id).ok_or("That site was removed")?;
    if !art.is_empty() { s.art = art; }
    if !icon.is_empty() { s.icon = icon; }
    if !named && !m.title.is_empty() { s.name = m.title; }
    let out = json!(s.clone());
    th::save(&st.dir, &sites)?;
    Ok(out)
}

#[tauri::command]
fn edit(id: String, name: String, category: String, url: String, st: tauri::State<State>) -> Result<Value, String> {
    let url = th::normalize_url(&url)?;
    let mut sites = st.sites.lock().unwrap();
    if sites.iter().any(|s| s.id != id && th::same_site(&s.url, &url)) { return Err("Another tile already has that link".into()); }
    let s = sites.iter_mut().find(|s| s.id == id).ok_or("That site isn't in Theater")?;
    let name = th::clean_title(&name);
    s.name = if name.is_empty() { th::host_name(&url) } else { name };
    s.category = th::fix_category(&category);
    s.url = url;
    let out = json!(s.clone());
    th::save(&st.dir, &sites)?;
    Ok(out)
}

#[tauri::command]
fn favorite(id: String, on: bool, st: tauri::State<State>) -> Result<(), String> {
    let mut sites = st.sites.lock().unwrap();
    sites.iter_mut().find(|s| s.id == id).ok_or("That site isn't in Theater")?.favorite = on;
    th::save(&st.dir, &sites)
}

#[tauri::command]
fn remove(id: String, st: tauri::State<State>) -> Result<(), String> {
    let mut sites = st.sites.lock().unwrap();
    th::remove(&st.dir, &mut sites, &id)?;
    th::save(&st.dir, &sites)
}

/// A saved picture as a data: address for an <img>.
#[tauri::command(async)]
fn art(file: String, st: tauri::State<'_, State>) -> Result<String, String> {
    if file.is_empty() || file.contains('/') || file.contains('\\') || file.starts_with('.') { return Err("no picture".into()); }
    let b = std::fs::read(th::art_dir(&st.dir).join(&file)).map_err(|e| e.to_string())?;
    Ok(format!("data:{};base64,{}", th::mime_for(&file), base64::engine::general_purpose::STANDARD.encode(b)))
}

/// Opens a site in the viewer (one at a time: another tile replaces what's playing).
#[tauri::command]
fn open(id: String, app: AppHandle) -> Result<(), String> {
    let st = app.state::<State>();
    let (url, name) = {
        let mut sites = st.sites.lock().unwrap();
        let s = sites.iter_mut().find(|s| s.id == id).ok_or("That site isn't in Theater")?;
        s.opened = th::now_ms();
        s.opens = s.opens.saturating_add(1);
        let out = (s.url.clone(), s.name.clone());
        th::save(&st.dir, &sites)?;
        out
    };
    let target: tauri::Url = url.parse().map_err(|_| "That link can't be opened".to_string())?;
    if let Some(w) = app.get_webview_window("viewer") { let _ = w.destroy(); }
    let handle = app.clone();
    let w = WebviewWindowBuilder::new(&app, "viewer", WebviewUrl::External(target))
        .title(format!("{name} – Hearth Theater"))
        .inner_size(1280.0, 800.0)
        .fullscreen(true)
        .focused(true)
        .background_color(tauri::window::Color(0, 0, 0, 255))
        .initialization_script_for_all_frames(VIEWER_JS)
        .on_navigation(move |u| {
            if u.as_str().starts_with(CLOSE_URL) {
                let h = handle.clone();
                let _ = handle.run_on_main_thread(move || close_viewer(&h));
                return false;
            }
            // Only web pages open in the viewer (no file: or other programs' links).
            matches!(u.scheme(), "http" | "https" | "about" | "blob" | "data")
        })
        .build()
        .map_err(|e| format!("Couldn't open the viewer: {e}"))?;
    #[cfg(target_os = "linux")]
    let _ = w.with_webview(|wv| {
        use webkit2gtk::{SettingsExt, WebViewExt};
        if let Some(s) = wv.inner().settings() {
            // The controller's A is not a "real" click to WebKit, so videos may play without one, and the page may
            // go full screen.
            s.set_media_playback_requires_user_gesture(false);
            s.set_enable_fullscreen(true);
            s.set_javascript_can_open_windows_automatically(false);
        }
    });
    #[cfg(not(target_os = "linux"))]
    let _ = &w;
    let _ = app.emit("opened", json!({"id": id}));
    Ok(())
}

fn close_viewer(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("viewer") { let _ = w.destroy(); }
    if let Some(m) = app.get_webview_window("main") { let _ = m.show(); let _ = m.set_focus(); }
    let _ = app.emit("closed", json!({}));
}

#[tauri::command]
fn close(app: AppHandle) { close_viewer(&app); }

#[tauri::command]
fn quit(app: AppHandle) { app.exit(0); }

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let dir = th::data_dir();
    if args.iter().any(|a| a == "--list") {
        println!("{}", serde_json::to_string_pretty(&th::load(&dir)).unwrap_or_default());
        std::process::exit(0);
    }
    let fullscreen = args.iter().any(|a| a == "--fullscreen");
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() { std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1"); }
    let sites = Mutex::new(th::load(&dir));
    tauri::Builder::default()
        .manage(State { dir, sites, fullscreen })
        .invoke_handler(tauri::generate_handler![init, preview, add, refresh, edit, favorite, remove, art, open, close, quit])
        .setup(move |app| {
            WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title("Hearth Theater")
                .inner_size(1280.0, 800.0)
                .min_inner_size(640.0, 440.0)
                .fullscreen(fullscreen)
                .background_color(tauri::window::Color(15, 11, 9, 255))
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Hearth Theater couldn't start");
}
