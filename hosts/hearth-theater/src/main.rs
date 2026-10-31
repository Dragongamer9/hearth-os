// Hearth Theater's windows (Tauri: WebKitGTK on Linux, WebView2 on Windows). The main window (ui/index.html) is your
// wall of saved sites: paste a link, and Theater reads the page's name, preview picture and icon to make a tile.
// Choosing a tile opens the site itself in the viewer: a second, full-screen window with a controller pointer laid
// over the page (viewer.js). Theater never takes videos out of a site; what plays, plays on the site's own page.
// Each site's catalog (its shows, movies and videos) is read in a hidden window of the same web view, with your own
// sign-ins, by reader.js, and shown as Theater's own rows; choosing a title opens its page in the viewer.
// `hearth-theater --fullscreen` starts full screen (how Game Mode opens it); `--list` prints the saved sites as JSON;
// `--read-catalogs` reads every site's catalog (no main window), saves it and quits (CI uses it).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use base64::Engine;
use hearth_theater as th;
use serde_json::{json, Value};
use std::io::Read;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

const VIEWER_JS: &str = include_str!("viewer.js");
/// The viewer's way of saying "back to Theater": it goes to this address, and the viewer window closes instead.
const CLOSE_URL: &str = "hearth-theater://close";
const READER_JS: &str = include_str!("reader.js");
const MAX_POSTER: u64 = 1536 * 1024;
const MAX_POSTERS: usize = 160;

/// The reader's answer as it comes in, piece by piece (reader.js's send()).
struct Pending {
    parts: Vec<Option<String>>,
    tx: std::sync::mpsc::Sender<String>,
}

struct State {
    dir: PathBuf,
    sites: Mutex<Vec<th::Site>>,
    titles: Mutex<Vec<th::Title>>,
    fullscreen: bool,
    pending: Mutex<Option<Pending>>,
    /// One reading at a time (one hidden reader window).
    read_lock: Mutex<()>,
    /// The sites whose catalog is being read now.
    reading: Mutex<Vec<String>>,
    /// The newest search (older ones still waiting to be read are dropped).
    search_gen: AtomicU64,
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> { m.lock().unwrap_or_else(|e| e.into_inner()) }

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

// ------------------------------------------------------------------------------------------- reading catalogs

/// Opens `url` in the hidden reader window with reader.js (mode "catalog" or "search") and waits for what it finds.
/// The page loads as it would in a browser, with Theater's own cookies and sign-ins. Never call it on the main thread.
fn read_page(app: &AppHandle, url: &str, mode: &str, wait: Duration, still_wanted: &dyn Fn() -> bool) -> Result<String, String> {
    let st = app.state::<State>();
    let _one = lock(&st.read_lock);
    if !still_wanted() { return Err("no longer wanted".into()); }
    let target: tauri::Url = url.parse().map_err(|_| "That link can't be read".to_string())?;
    let (tx, rx) = std::sync::mpsc::channel();
    *lock(&st.pending) = Some(Pending { parts: Vec::new(), tx });
    if let Some(w) = app.get_webview_window("reader") { let _ = w.destroy(); }
    let handle = app.clone();
    let script = format!("window.__hearthReaderMode={};\n{}", json!(mode), READER_JS);
    let w = WebviewWindowBuilder::new(app, "reader", WebviewUrl::External(target))
        .title("Hearth Theater reader")
        .inner_size(1280.0, 800.0)
        .visible(false)
        .focused(false)
        .skip_taskbar(true)
        .initialization_script(&script)
        .on_navigation(move |u| {
            if u.scheme() == "hearth-theater" { reader_piece(&handle, u); return false; }
            matches!(u.scheme(), "http" | "https" | "about" | "blob" | "data")
        })
        .build()
        .map_err(|e| { *lock(&st.pending) = None; format!("Couldn't read the site: {e}") })?;
    #[cfg(target_os = "linux")]
    let _ = w.with_webview(|wv| {
        use webkit2gtk::{SettingsExt, WebViewExt};
        if let Some(s) = wv.inner().settings() {
            s.set_media_playback_requires_user_gesture(true);
            s.set_javascript_can_open_windows_automatically(false);
        }
    });
    let got = rx.recv_timeout(wait);
    *lock(&st.pending) = None;
    let _ = w.destroy();
    got.map_err(|_| "The site didn't finish loading in time".to_string())
}

/// One piece of the reader's answer: hearth-theater://read/<n>/<count>?d=<piece>.
fn reader_piece(app: &AppHandle, u: &tauri::Url) {
    if u.host_str() != Some("read") { return; }
    let mut segs = u.path().trim_start_matches('/').split('/');
    let n = segs.next().and_then(|x| x.parse::<usize>().ok());
    let count = segs.next().and_then(|x| x.parse::<usize>().ok());
    let (Some(n), Some(count)) = (n, count) else { return };
    if count == 0 || count > 400 || n >= count { return; }
    let d = u.query_pairs().find(|(k, _)| k == "d").map(|(_, v)| v.into_owned()).unwrap_or_default();
    let st = app.state::<State>();
    let mut p = lock(&st.pending);
    let Some(pend) = p.as_mut() else { return };
    if pend.parts.len() != count { pend.parts = vec![None; count]; }
    pend.parts[n] = Some(d);
    if pend.parts.iter().all(Option::is_some) {
        let all: String = pend.parts.iter().filter_map(|x| x.as_deref()).collect();
        let _ = pend.tx.send(all);
    } else {
        let h = app.clone();
        let _ = app.run_on_main_thread(move || {
            if let Some(w) = h.get_webview_window("reader") { let _ = w.eval("window.__hearthReaderAck && window.__hearthReaderAck()"); }
        });
    }
}

/// Saves a poster into art/c-<site>/; its file name, or None.
fn fetch_poster(dir: &std::path::Path, id: &str, url: &str) -> Option<String> {
    let r = agent().get(url).call().ok()?;
    let ct = r.content_type().to_string();
    if !ct.starts_with("image/") { return None; }
    let mut b = Vec::new();
    r.into_reader().take(MAX_POSTER + 1).read_to_end(&mut b).ok()?;
    if b.is_empty() || b.len() as u64 > MAX_POSTER { return None; }
    let name = th::poster_name(id, url, &ct);
    std::fs::write(th::art_dir(dir).join(&name), &b).ok()?;
    Some(name)
}

/// Saves the catalog's posters that aren't saved yet (four at a time) and drops the site's old ones nothing uses.
fn fetch_posters(dir: &std::path::Path, id: &str, cat: &mut th::Catalog, keep: &[String]) -> usize {
    let mut want: Vec<String> = Vec::new();
    for i in cat.rows.iter().flat_map(|r| &r.items) {
        if i.art.is_empty() && !i.image.is_empty() && !want.contains(&i.image) { want.push(i.image.clone()); }
    }
    want.truncate(MAX_POSTERS);
    let folder = th::art_dir(dir).join(format!("c-{id}"));
    let _ = std::fs::create_dir_all(&folder);
    let got = Mutex::new(std::collections::HashMap::new());
    let next = AtomicUsize::new(0);
    std::thread::scope(|s| {
        for _ in 0..4 {
            s.spawn(|| loop {
                let k = next.fetch_add(1, Ordering::SeqCst);
                let Some(u) = want.get(k) else { break };
                if let Some(f) = fetch_poster(dir, id, u) { lock(&got).insert(u.clone(), f); }
            });
        }
    });
    let got = got.into_inner().unwrap_or_default();
    for i in cat.rows.iter_mut().flat_map(|r| r.items.iter_mut()) {
        if let Some(f) = got.get(&i.image) { i.art = f.clone(); }
    }
    let used: std::collections::HashSet<String> = cat.rows.iter().flat_map(|r| &r.items).map(|i| i.art.clone()).chain(keep.iter().cloned()).collect();
    if let Ok(rd) = std::fs::read_dir(&folder) {
        for e in rd.flatten() {
            let n = format!("c-{id}/{}", e.file_name().to_string_lossy());
            if !used.contains(&n) { let _ = std::fs::remove_file(e.path()); }
        }
    }
    got.len()
}

/// Reads a site's catalog again (in the hidden reader), saves it with its posters and tells the main window.
fn read_catalog(app: &AppHandle, id: &str) -> Result<th::Catalog, String> {
    let st = app.state::<State>();
    let (url, name) = {
        let sites = lock(&st.sites);
        let s = sites.iter().find(|s| s.id == id).ok_or("That site isn't in Theater")?;
        (s.url.clone(), s.name.clone())
    };
    lock(&st.reading).push(id.to_string());
    let _ = app.emit("catalog-reading", json!({"id": id}));
    let got = read_page(app, &url, "catalog", Duration::from_secs(75), &|| true)
        .and_then(|raw| serde_json::from_str::<Value>(&raw).map_err(|e| format!("The reader's answer didn't make sense ({e})")));
    lock(&st.reading).retain(|x| x != id);
    let old = th::load_catalog(&st.dir, id);
    let mut cat = match got {
        Ok(v) => { let mut c = th::sanitize(&v, &url, &name); th::carry_art(&old, &mut c, &st.dir); c }
        Err(e) => { let mut c = old; c.error = e; c.updated = th::now_ms(); c }
    };
    let gone = || !lock(&st.sites).iter().any(|s| s.id == id);
    if gone() { return Err("That site was removed".into()); }
    th::save_catalog(&st.dir, id, &cat)?;
    let _ = app.emit("catalog", json!({"id": id, "catalog": cat}));
    let keep: Vec<String> = lock(&st.titles).iter().filter(|t| t.site == id).map(|t| t.item.art.clone()).collect();
    if fetch_posters(&st.dir, id, &mut cat, &keep) > 0 && !gone() {
        th::save_catalog(&st.dir, id, &cat)?;
        let _ = app.emit("catalog", json!({"id": id, "catalog": cat}));
    }
    Ok(cat)
}

#[tauri::command]
fn init(st: tauri::State<State>) -> Value {
    let sites = lock(&st.sites).clone();
    let catalogs: serde_json::Map<String, Value> = sites.iter().map(|s| (s.id.clone(), json!(th::load_catalog(&st.dir, &s.id)))).collect();
    json!({"sites": sites, "categories": th::CATEGORIES, "fullscreen": st.fullscreen, "titles": *lock(&st.titles),
           "catalogs": catalogs, "reading": *lock(&st.reading),
           "platform": if cfg!(windows) { "windows" } else { "linux" }})
}

/// Reads a site's catalog now (Refresh now, and right after a site is added).
#[tauri::command(async)]
fn catalog_refresh(id: String, app: AppHandle) -> Result<Value, String> {
    read_catalog(&app, &id).map(|c| json!(c))
}

/// Searches one site with its own search page (`gen`: the search's number; older ones are dropped).
#[tauri::command(async)]
fn search_site(id: String, query: String, gen: u64, app: AppHandle) -> Result<Value, String> {
    let st = app.state::<State>();
    st.search_gen.fetch_max(gen, Ordering::SeqCst);
    let q = query.trim().to_string();
    if q.is_empty() { return Ok(json!({"items": []})); }
    let (site_url, template) = {
        let sites = lock(&st.sites);
        let s = sites.iter().find(|s| s.id == id).ok_or("That site isn't in Theater")?;
        (s.url.clone(), th::load_catalog(&st.dir, &id).search)
    };
    if template.is_empty() { return Ok(json!({"items": [], "nosearch": true})); }
    let page = th::search_url(&template, &q);
    let raw = read_page(&app, &page, "search", Duration::from_secs(35), &|| st.search_gen.load(Ordering::SeqCst) <= gen)?;
    let v: Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    Ok(json!({"items": th::sanitize_items(&v, &page, &site_url), "page": page}))
}

fn item_from(v: &Value) -> Result<th::Item, String> {
    let mut i = th::sanitize_item(v, None).ok_or("That title can't be opened")?;
    let art = v.get("art").and_then(|x| x.as_str()).unwrap_or("");
    if th::safe_art(art) { i.art = art.to_string(); }
    Ok(i)
}

/// Opens a title's page in the viewer and keeps it for Continue.
#[tauri::command]
fn open_title(site: String, item: Value, app: AppHandle) -> Result<Value, String> {
    let st = app.state::<State>();
    let item = item_from(&item)?;
    let t = {
        let mut titles = lock(&st.titles);
        let t = th::keep_title(&mut titles, &site, &item);
        t.opened = th::now_ms();
        t.opens = t.opens.saturating_add(1);
        let out = t.clone();
        th::tidy_titles(&mut titles);
        th::save_titles(&st.dir, &titles)?;
        out
    };
    if let Some(s) = lock(&st.sites).iter_mut().find(|s| s.id == site) { s.opened = th::now_ms(); }
    let _ = th::save(&st.dir, &lock(&st.sites));
    open_viewer(&app, &item.url, &item.title)?;
    Ok(json!(t))
}

#[tauri::command]
fn fav_title(site: String, item: Value, on: bool, st: tauri::State<State>) -> Result<Value, String> {
    let item = item_from(&item)?;
    let mut titles = lock(&st.titles);
    let t = th::keep_title(&mut titles, &site, &item);
    t.favorite = on;
    let out = t.clone();
    th::tidy_titles(&mut titles);
    th::save_titles(&st.dir, &titles)?;
    Ok(json!(out))
}

/// Hides (or shows again) one of a site's catalog rows.
#[tauri::command]
fn hide_row(id: String, row: String, hidden: bool, st: tauri::State<State>) -> Result<Value, String> {
    let mut sites = lock(&st.sites);
    let s = sites.iter_mut().find(|s| s.id == id).ok_or("That site isn't in Theater")?;
    s.hidden.retain(|r| r != &row);
    if hidden && !row.is_empty() { s.hidden.push(row.chars().take(80).collect()); }
    let out = json!(s.clone());
    th::save(&st.dir, &sites)?;
    Ok(out)
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
    if !th::safe_art(&file) { return Err("no picture".into()); }
    let b = std::fs::read(th::art_dir(&st.dir).join(&file)).map_err(|e| e.to_string())?;
    Ok(format!("data:{};base64,{}", th::mime_for(&file), base64::engine::general_purpose::STANDARD.encode(b)))
}

/// Opens a site in the viewer (one at a time: another tile replaces what's playing).
#[tauri::command]
fn open(id: String, app: AppHandle) -> Result<(), String> {
    let st = app.state::<State>();
    let (url, name) = {
        let mut sites = lock(&st.sites);
        let s = sites.iter_mut().find(|s| s.id == id).ok_or("That site isn't in Theater")?;
        s.opened = th::now_ms();
        s.opens = s.opens.saturating_add(1);
        let out = (s.url.clone(), s.name.clone());
        th::save(&st.dir, &sites)?;
        out
    };
    open_viewer(&app, &url, &name)?;
    let _ = app.emit("opened", json!({"id": id}));
    Ok(())
}

/// The viewer: a page full screen with the controller pointer (viewer.js).
fn open_viewer(app: &AppHandle, url: &str, name: &str) -> Result<(), String> {
    let target: tauri::Url = url.parse().map_err(|_| "That link can't be opened".to_string())?;
    if let Some(w) = app.get_webview_window("viewer") { let _ = w.destroy(); }
    let handle = app.clone();
    let w = WebviewWindowBuilder::new(app, "viewer", WebviewUrl::External(target))
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
    let read_all = args.iter().any(|a| a == "--read-catalogs");
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() { std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1"); }
    let sites = Mutex::new(th::load(&dir));
    let titles = Mutex::new(th::load_titles(&dir));
    let app = tauri::Builder::default()
        .manage(State { dir, sites, titles, fullscreen, pending: Mutex::new(None), read_lock: Mutex::new(()), reading: Mutex::new(Vec::new()), search_gen: AtomicU64::new(0) })
        .invoke_handler(tauri::generate_handler![init, preview, add, refresh, edit, favorite, remove, art, open, close, quit,
            catalog_refresh, search_site, open_title, fav_title, hide_row])
        .setup(move |app| {
            let handle = app.handle().clone();
            // Catalogs older than six hours are read again in the background, one site at a time (--read-catalogs:
            // every site, then Theater quits).
            std::thread::spawn(move || {
                if !read_all { std::thread::sleep(Duration::from_secs(3)); }
                let st = handle.state::<State>();
                let sites = lock(&st.sites).clone();
                let ids: Vec<String> = if read_all { sites.iter().map(|s| s.id.clone()).collect() }
                    else { th::stale(&sites, |id| th::load_catalog(&st.dir, id).updated, th::now_ms()) };
                for id in ids {
                    let r = read_catalog(&handle, &id);
                    if read_all {
                        match r {
                            Ok(c) => println!("{id}: {} rows, {} titles, search {:?}, sources {:?}, error {:?}", c.rows.len(),
                                c.rows.iter().map(|r| r.items.len()).sum::<usize>(), c.search, c.sources, c.error),
                            Err(e) => println!("{id}: {e}"),
                        }
                    }
                }
                if read_all { handle.exit(0); }
            });
            if read_all { return Ok(()); }
            WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title("Hearth Theater")
                .inner_size(1280.0, 800.0)
                .min_inner_size(640.0, 440.0)
                .fullscreen(fullscreen)
                .background_color(tauri::window::Color(15, 11, 9, 255))
                .build()?;
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("Hearth Theater couldn't start");
    app.run(move |_, e| {
        // Reading catalogs without a window: the reader window closing between sites isn't the end.
        if let tauri::RunEvent::ExitRequested { code, api, .. } = e { if read_all && code.is_none() { api.prevent_exit(); } }
    });
}
