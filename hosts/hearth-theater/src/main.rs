// Hearth Theater. Its windows are drawn by Chromium: Hearth's own Electron build (/usr/lib/hearth-electron, the
// same one that draws Game Mode on Nvidia graphics), running Theater's window app (electron/main.js). This program
// is what Game Mode and the app menu start, and Theater's core:
//
//   hearth-theater [--fullscreen]   starts the window app (`hearth-electron --hearth-theater`), with its output going
//                                   to Theater's log (~/.local/share/hearth-theater/theater.log, in Send logs);
//   hearth-theater --core-stdio     the core, started by the window app: your sites, their catalogs and posters, and
//                                   the controller pointer, talking one JSON object a line over stdin/stdout;
//   hearth-theater --list           prints the saved sites as JSON;
//   hearth-theater --read-catalogs  reads every site's catalog (no main window), saves it and quits (CI uses it);
//   hearth-theater --open <url>     opens one page in the viewer (tests).
//
// Why Chromium and not the system's WebKitGTK (what Theater used before): WebKitGTK's drawing fails on RTX 50
// graphics (the window stays empty; docs/RENDERERS.md), bot checks (Cloudflare, hCaptcha, reCAPTCHA) often loop on
// it, and its pages could only get pretend clicks. See docs/THEATER.md.
//
// The window app → core lines: {"t":"call","id","cmd","args"} (the main window's calls, answered with
// {"t":"reply","id","ok","value"|"error"}), {"t":"read-done","id","data"|"error"} (a page the reader read),
// {"t":"ptr",...} (the pointer: lib.rs ptr_cmd). Core → window app: {"t":"event","name","payload"},
// {"t":"read","id","url","mode"} / {"t":"read-stop","id"} (read a page in the hidden reader), {"t":"do","op":
// "viewer"|"close-viewer"|"quit"}, {"t":"log","text"}.

mod pointer;

use base64::Engine;
use hearth_theater as th;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{BufRead, Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

const MAX_POSTER: u64 = 1536 * 1024;
const MAX_POSTERS: usize = 160;

struct State {
    dir: PathBuf,
    sites: Mutex<Vec<th::Site>>,
    titles: Mutex<Vec<th::Title>>,
    fullscreen: bool,
    /// Pages being read by the window app's hidden reader: id → where the answer goes.
    pending: Mutex<HashMap<u64, std::sync::mpsc::Sender<Result<String, String>>>>,
    next_read: AtomicU64,
    /// One reading at a time (one hidden reader window).
    read_lock: Mutex<()>,
    /// The sites whose catalog is being read now.
    reading: Mutex<Vec<String>>,
    /// The newest search (older ones still waiting to be read are dropped).
    search_gen: AtomicU64,
    out: Mutex<std::io::Stdout>,
}

static ST: OnceLock<State> = OnceLock::new();
fn st() -> &'static State { ST.get().expect("the core's state") }

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> { m.lock().unwrap_or_else(|e| e.into_inner()) }

/// A line in Theater's log (its standard error, which the launcher points at theater.log).
fn log(msg: &str) { eprintln!("{} [theater core] {msg}", th::now_ms()); }

/// One line to the window app.
fn send(v: Value) {
    let mut o = lock(&st().out);
    let _ = writeln!(o, "{v}");
    let _ = o.flush();
}

fn emit(name: &str, payload: Value) { send(json!({"t": "event", "name": name, "payload": payload})); }

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(8))
        .timeout(Duration::from_secs(15))
        .redirects(6)
        .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36")
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

/// Has the window app's hidden reader open `url` with reader.js (mode "catalog" or "search") and waits for what it
/// finds. The page loads as it would in a browser, with Theater's own cookies and sign-ins.
fn read_page(url: &str, mode: &str, wait: Duration, still_wanted: &dyn Fn() -> bool) -> Result<String, String> {
    let s = st();
    let _one = lock(&s.read_lock);
    if !still_wanted() { return Err("no longer wanted".into()); }
    let target = url::Url::parse(url).map_err(|_| "That link can't be read".to_string())?;
    if !matches!(target.scheme(), "http" | "https") { return Err("That link can't be read".into()); }
    let id = s.next_read.fetch_add(1, Ordering::SeqCst) + 1;
    let (tx, rx) = std::sync::mpsc::channel();
    lock(&s.pending).insert(id, tx);
    send(json!({"t": "read", "id": id, "url": target.as_str(), "mode": mode, "wait": wait.as_millis() as u64}));
    let got = rx.recv_timeout(wait);
    lock(&s.pending).remove(&id);
    send(json!({"t": "read-stop", "id": id}));
    match got {
        Ok(r) => r,
        Err(_) => Err("The site didn't finish loading in time".to_string()),
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
    let got = Mutex::new(HashMap::new());
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
fn read_catalog(id: &str) -> Result<th::Catalog, String> {
    let s = st();
    let (url, name) = {
        let sites = lock(&s.sites);
        let x = sites.iter().find(|x| x.id == id).ok_or("That site isn't in Theater")?;
        (x.url.clone(), x.name.clone())
    };
    lock(&s.reading).push(id.to_string());
    emit("catalog-reading", json!({"id": id}));
    let got = read_page(&url, "catalog", Duration::from_secs(75), &|| true)
        .and_then(|raw| serde_json::from_str::<Value>(&raw).map_err(|e| format!("The reader's answer didn't make sense ({e})")));
    lock(&s.reading).retain(|x| x != id);
    let old = th::load_catalog(&s.dir, id);
    let mut cat = match got {
        Ok(v) => { let mut c = th::sanitize(&v, &url, &name); th::carry_art(&old, &mut c, &s.dir); c }
        Err(e) => { log(&format!("catalog of {}: {e}", th::log_url(&url))); let mut c = old; c.error = e; c.updated = th::now_ms(); c }
    };
    let gone = || !lock(&s.sites).iter().any(|x| x.id == id);
    if gone() { return Err("That site was removed".into()); }
    th::save_catalog(&s.dir, id, &cat)?;
    emit("catalog", json!({"id": id, "catalog": cat}));
    let keep: Vec<String> = lock(&s.titles).iter().filter(|t| t.site == id).map(|t| t.item.art.clone()).collect();
    if fetch_posters(&s.dir, id, &mut cat, &keep) > 0 && !gone() {
        th::save_catalog(&s.dir, id, &cat)?;
        emit("catalog", json!({"id": id, "catalog": cat}));
    }
    Ok(cat)
}

// ------------------------------------------------------------------------------------------- the main window's calls

fn arg(a: &Value, k: &str) -> String { a.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string() }

fn item_from(v: &Value) -> Result<th::Item, String> {
    let mut i = th::sanitize_item(v, None).ok_or("That title can't be opened")?;
    let art = v.get("art").and_then(|x| x.as_str()).unwrap_or("");
    if th::safe_art(art) { i.art = art.to_string(); }
    Ok(i)
}

/// Opens a page in the viewer (one at a time: another tile replaces what's playing). Web pages only.
fn open_viewer(url: &str, name: &str) -> Result<(), String> {
    let u = url::Url::parse(url).map_err(|_| "That link can't be opened".to_string())?;
    if !matches!(u.scheme(), "http" | "https") { return Err("That link can't be opened".into()); }
    log(&format!("viewer: opening {}", th::log_url(url)));
    send(json!({"t": "do", "op": "viewer", "url": u.as_str(), "title": format!("{name} – Hearth Theater")}));
    Ok(())
}

fn call(cmd: &str, a: &Value) -> Result<Value, String> {
    let s = st();
    match cmd {
        "init" => {
            let sites = lock(&s.sites).clone();
            let catalogs: serde_json::Map<String, Value> = sites.iter().map(|x| (x.id.clone(), json!(th::load_catalog(&s.dir, &x.id)))).collect();
            Ok(json!({"sites": sites, "categories": th::CATEGORIES, "fullscreen": s.fullscreen, "titles": *lock(&s.titles),
                      "catalogs": catalogs, "reading": *lock(&s.reading), "platform": "linux", "pointer": pointer::status()}))
        }
        // Reads a site's catalog now (Refresh now, and right after a site is added).
        "catalog_refresh" => read_catalog(&arg(a, "id")).map(|c| json!(c)),
        // Searches one site with its own search page (`gen`: the search's number; older ones are dropped).
        "search_site" => {
            let (id, gen) = (arg(a, "id"), a.get("gen").and_then(|x| x.as_u64()).unwrap_or(0));
            s.search_gen.fetch_max(gen, Ordering::SeqCst);
            let q = arg(a, "query").trim().to_string();
            if q.is_empty() { return Ok(json!({"items": []})); }
            let (site_url, template) = {
                let sites = lock(&s.sites);
                let x = sites.iter().find(|x| x.id == id).ok_or("That site isn't in Theater")?;
                (x.url.clone(), th::load_catalog(&s.dir, &id).search)
            };
            if template.is_empty() { return Ok(json!({"items": [], "nosearch": true})); }
            let page = th::search_url(&template, &q);
            let raw = read_page(&page, "search", Duration::from_secs(35), &|| s.search_gen.load(Ordering::SeqCst) <= gen)?;
            let v: Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
            Ok(json!({"items": th::sanitize_items(&v, &page, &site_url), "page": page}))
        }
        // Opens a title's page in the viewer and keeps it for Continue.
        "open_title" => {
            let site = arg(a, "site");
            let item = item_from(a.get("item").unwrap_or(&Value::Null))?;
            let t = {
                let mut titles = lock(&s.titles);
                let t = th::keep_title(&mut titles, &site, &item);
                t.opened = th::now_ms();
                t.opens = t.opens.saturating_add(1);
                let out = t.clone();
                th::tidy_titles(&mut titles);
                th::save_titles(&s.dir, &titles)?;
                out
            };
            if let Some(x) = lock(&s.sites).iter_mut().find(|x| x.id == site) { x.opened = th::now_ms(); }
            let _ = th::save(&s.dir, &lock(&s.sites));
            open_viewer(&item.url, &item.title)?;
            Ok(json!(t))
        }
        "fav_title" => {
            let item = item_from(a.get("item").unwrap_or(&Value::Null))?;
            let mut titles = lock(&s.titles);
            let t = th::keep_title(&mut titles, &arg(a, "site"), &item);
            t.favorite = a.get("on").and_then(|x| x.as_bool()).unwrap_or(false);
            let out = t.clone();
            th::tidy_titles(&mut titles);
            th::save_titles(&s.dir, &titles)?;
            Ok(json!(out))
        }
        // Hides (or shows again) one of a site's catalog rows.
        "hide_row" => {
            let (id, row, hidden) = (arg(a, "id"), arg(a, "row"), a.get("hidden").and_then(|x| x.as_bool()).unwrap_or(false));
            let mut sites = lock(&s.sites);
            let x = sites.iter_mut().find(|x| x.id == id).ok_or("That site isn't in Theater")?;
            x.hidden.retain(|r| r != &row);
            if hidden && !row.is_empty() { x.hidden.push(row.chars().take(80).collect()); }
            let out = json!(x.clone());
            th::save(&s.dir, &sites)?;
            Ok(out)
        }
        // What a link's page is called and looks like, for the Add dialog to fill in the name.
        "preview" => {
            let url = th::normalize_url(&arg(a, "url"))?;
            let (html, at) = fetch_page(&url)?;
            let m = th::parse_meta(&html, &at);
            Ok(json!({"url": url, "title": m.title, "image": m.image, "host": th::host_name(&url)}))
        }
        "add" => {
            let mut sites = lock(&s.sites);
            let x = th::add(&mut sites, &arg(a, "url"), &arg(a, "name"), &arg(a, "category"))?;
            th::save(&s.dir, &sites)?;
            Ok(json!(x))
        }
        // Reads the site's page again for its picture and icon (and its name, if you didn't give it one).
        "refresh" => {
            let id = arg(a, "id");
            let (url, named) = {
                let sites = lock(&s.sites);
                let x = sites.iter().find(|x| x.id == id).ok_or("That site isn't in Theater")?;
                (x.url.clone(), x.name != th::host_name(&x.url))
            };
            let (html, at) = fetch_page(&url)?;
            let m = th::parse_meta(&html, &at);
            let art = fetch_art(&s.dir, &id, "art", &m.image);
            let icon = fetch_art(&s.dir, &id, "icon", &m.icon);
            let mut sites = lock(&s.sites);
            let x = sites.iter_mut().find(|x| x.id == id).ok_or("That site was removed")?;
            if !art.is_empty() { x.art = art; }
            if !icon.is_empty() { x.icon = icon; }
            if !named && !m.title.is_empty() { x.name = m.title; }
            let out = json!(x.clone());
            th::save(&s.dir, &sites)?;
            Ok(out)
        }
        "edit" => {
            let (id, url) = (arg(a, "id"), th::normalize_url(&arg(a, "url"))?);
            let mut sites = lock(&s.sites);
            if sites.iter().any(|x| x.id != id && th::same_site(&x.url, &url)) { return Err("Another tile already has that link".into()); }
            let x = sites.iter_mut().find(|x| x.id == id).ok_or("That site isn't in Theater")?;
            let name = th::clean_title(&arg(a, "name"));
            x.name = if name.is_empty() { th::host_name(&url) } else { name };
            x.category = th::fix_category(&arg(a, "category"));
            x.url = url;
            let out = json!(x.clone());
            th::save(&s.dir, &sites)?;
            Ok(out)
        }
        "favorite" => {
            let mut sites = lock(&s.sites);
            sites.iter_mut().find(|x| x.id == arg(a, "id")).ok_or("That site isn't in Theater")?.favorite = a.get("on").and_then(|x| x.as_bool()).unwrap_or(false);
            th::save(&s.dir, &sites).map(|_| Value::Null)
        }
        "remove" => {
            let mut sites = lock(&s.sites);
            th::remove(&s.dir, &mut sites, &arg(a, "id"))?;
            th::save(&s.dir, &sites).map(|_| Value::Null)
        }
        // A saved picture as a data: address for an <img>.
        "art" => {
            let file = arg(a, "file");
            if !th::safe_art(&file) { return Err("no picture".into()); }
            let b = std::fs::read(th::art_dir(&s.dir).join(&file)).map_err(|e| e.to_string())?;
            Ok(json!(format!("data:{};base64,{}", th::mime_for(&file), base64::engine::general_purpose::STANDARD.encode(b))))
        }
        // Opens a site in the viewer.
        "open" => {
            let id = arg(a, "id");
            let (url, name) = {
                let mut sites = lock(&s.sites);
                let x = sites.iter_mut().find(|x| x.id == id).ok_or("That site isn't in Theater")?;
                x.opened = th::now_ms();
                x.opens = x.opens.saturating_add(1);
                let out = (x.url.clone(), x.name.clone());
                th::save(&s.dir, &sites)?;
                out
            };
            open_viewer(&url, &name)?;
            emit("opened", json!({"id": id}));
            Ok(Value::Null)
        }
        "close" => { send(json!({"t": "do", "op": "close-viewer"})); Ok(Value::Null) }
        "quit" => { send(json!({"t": "do", "op": "quit"})); Ok(Value::Null) }
        _ => Err(format!("unknown call {cmd}")),
    }
}

// ------------------------------------------------------------------------------------------- the core

fn core(args: &[String]) {
    let dir = th::data_dir();
    let fullscreen = args.iter().any(|a| a == "--fullscreen");
    let read_all = args.iter().any(|a| a == "--read-catalogs");
    let open_url = args.iter().position(|a| a == "--open").and_then(|i| args.get(i + 1)).cloned();
    let sites = th::load(&dir);
    log(&format!("core: started ({} sites, data in {})", sites.len(), dir.display()));
    let _ = ST.set(State {
        dir, sites: Mutex::new(sites), titles: Mutex::new(Vec::new()), fullscreen, pending: Mutex::new(HashMap::new()), next_read: AtomicU64::new(0),
        read_lock: Mutex::new(()), reading: Mutex::new(Vec::new()), search_gen: AtomicU64::new(0), out: Mutex::new(std::io::stdout()),
    });
    *lock(&st().titles) = th::load_titles(&st().dir);
    log(&pointer::init());
    // Catalogs older than six hours are read again in the background, one site at a time (--read-catalogs: every
    // site, then Theater quits).
    std::thread::spawn(move || {
        if !read_all { std::thread::sleep(Duration::from_secs(3)); }
        let s = st();
        let sites = lock(&s.sites).clone();
        let ids: Vec<String> = if read_all { sites.iter().map(|x| x.id.clone()).collect() }
            else { th::stale(&sites, |id| th::load_catalog(&s.dir, id).updated, th::now_ms()) };
        for id in ids {
            let r = read_catalog(&id);
            if read_all {
                let line = match r {
                    Ok(c) => format!("{id}: {} rows, {} titles, search {:?}, sources {:?}, error {:?}", c.rows.len(),
                        c.rows.iter().map(|r| r.items.len()).sum::<usize>(), c.search, c.sources, c.error),
                    Err(e) => format!("{id}: {e}"),
                };
                log(&line);
            }
        }
        if read_all { send(json!({"t": "do", "op": "quit"})); }
    });
    if let Some(u) = open_url { if let Err(e) = open_viewer(&u, "Test page") { log(&format!("--open: {e}")); } }
    // The window app's lines. Calls that wait (the network, the reader) each get a thread.
    let stdin = std::io::stdin();
    let mut ptr_errors = 0u32;
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        let Ok(v) = serde_json::from_str::<Value>(line.trim()) else { continue };
        match v.get("t").and_then(|x| x.as_str()).unwrap_or("") {
            "call" => {
                let id = v.get("id").cloned().unwrap_or(Value::Null);
                let cmd = v.get("cmd").and_then(|x| x.as_str()).unwrap_or("").to_string();
                let a = v.get("args").cloned().unwrap_or(Value::Null);
                std::thread::spawn(move || {
                    let r = std::panic::catch_unwind(|| call(&cmd, &a)).unwrap_or_else(|_| Err("Theater ran into a problem".into()));
                    match r {
                        Ok(value) => send(json!({"t": "reply", "id": id, "ok": true, "value": value})),
                        Err(e) => send(json!({"t": "reply", "id": id, "ok": false, "error": e})),
                    }
                });
            }
            "read-done" => {
                let id = v.get("id").and_then(|x| x.as_u64()).unwrap_or(0);
                let r = match v.get("error").and_then(|x| x.as_str()) {
                    Some(e) => Err(e.to_string()),
                    None => Ok(v.get("data").and_then(|x| x.as_str()).unwrap_or("").to_string()),
                };
                if let Some(tx) = lock(&st().pending).get(&id) { let _ = tx.send(r); }
            }
            "ptr" => {
                if let Some(p) = th::ptr_cmd(&v) {
                    if let Err(e) = pointer::apply(&p) {
                        ptr_errors += 1;
                        if ptr_errors <= 5 || ptr_errors % 500 == 0 { log(&format!("pointer: {e} ({ptr_errors} so far)")); }
                    }
                }
            }
            _ => {}
        }
    }
    log("core: the window app closed; leaving");
}

// ------------------------------------------------------------------------------------------- starting the window app

#[cfg(unix)]
fn launch(args: &[String]) -> i32 {
    use std::os::unix::process::CommandExt;
    let dir = th::data_dir();
    let _ = std::fs::create_dir_all(&dir);
    let logf = th::log_file(&dir);
    th::trim_log(&logf, 2 * 1024 * 1024, 384 * 1024);
    // Everything Theater and Chromium say goes to its log (HEARTH_THEATER_LOG=- keeps it on the terminal).
    if std::env::var("HEARTH_THEATER_LOG").as_deref() != Ok("-") {
        if let Ok(f) = std::fs::OpenOptions::new().create(true).append(true).open(&logf) {
            use std::os::unix::io::AsRawFd;
            unsafe { libc::dup2(f.as_raw_fd(), 1); libc::dup2(f.as_raw_fd(), 2); }
        }
    }
    let env = |k: &str| std::env::var(k).unwrap_or_else(|_| "-".into());
    log(&format!("===== Hearth Theater {} starting ({}) =====", env!("CARGO_PKG_VERSION"), if args.is_empty() { "windowed".to_string() } else { args.join(" ") }));
    log(&format!("session: DISPLAY={} WAYLAND_DISPLAY={} XDG_SESSION_TYPE={} gamescope={} renderer of Hearth={}",
        env("DISPLAY"), env("WAYLAND_DISPLAY"), env("XDG_SESSION_TYPE"), std::env::var_os("GAMESCOPE_WAYLAND_DISPLAY").is_some(), env("HEARTH_RENDERER_RUNNING")));
    let Some(electron) = th::find_electron_in(std::path::Path::new("/"), std::env::var("HEARTH_THEATER_ELECTRON").ok().as_deref()) else {
        log("can't start: Hearth's Chromium (/usr/lib/hearth-electron/hearth-electron) isn't on this system, and Theater's windows need it");
        return 1;
    };
    let me = std::env::current_exe().map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|_| "hearth-theater".into());
    let mut cmd = std::process::Command::new(&electron);
    // A development Electron (tests) is given Theater's window app folder; Hearth's own build carries it.
    if let Ok(app) = std::env::var("HEARTH_THEATER_APP") { if !app.is_empty() { cmd.arg(app); } }
    cmd.arg("--hearth-theater");
    for f in std::env::var("HEARTH_THEATER_ELECTRON_FLAGS").unwrap_or_default().split_whitespace() { cmd.arg(f); }
    cmd.args(args).env("HEARTH_THEATER_CORE", &me);
    log(&format!("renderer: Chromium ({} --hearth-theater); the window app logs the graphics it got", electron.display()));
    let e = cmd.exec();
    log(&format!("can't start {}: {e}", electron.display()));
    1
}

#[cfg(not(unix))]
fn launch(_args: &[String]) -> i32 { eprintln!("Hearth Theater runs on Linux (Hearth OS)"); 1 }

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--list") {
        println!("{}", serde_json::to_string_pretty(&th::load(&th::data_dir())).unwrap_or_default());
        return;
    }
    if args.iter().any(|a| a == "--core-stdio") { core(&args); return; }
    // Hearth's own arguments only go to the window app.
    let pass: Vec<String> = {
        let mut out = Vec::new();
        let mut it = args.iter().peekable();
        while let Some(a) = it.next() {
            if a == "--fullscreen" || a == "--read-catalogs" { out.push(a.clone()); }
            else if a == "--open" { if let Some(u) = it.next() { out.push(a.clone()); out.push(u.clone()); } }
        }
        out
    };
    std::process::exit(launch(&pass));
}
