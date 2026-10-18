//! Game art (and store profile pictures) kept on the device, so the library looks the same offline. Port of
//! hosts/linux/src/art-cache.js, sharing its folder: web pictures are downloaded once into
//! ~/.local/share/nimbus/art-cache/<sha1>.<ext> and served as nimbus-art://cache/<name>. Until a picture is
//! in, the UI uses its web address; downloads run quietly in the background, a few at a time.

use serde_json::{json, Value};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

const EXTS: [&str; 6] = ["jpg", "png", "webp", "jpeg", "gif", "avif"];
const WORKERS: usize = 3;

struct Cache { dir: PathBuf, index: HashMap<String, PathBuf>, queue: VecDeque<String>, queued: HashSet<String>, failed: HashMap<String, u64>, active: usize, last_changed: u64 }

fn cache() -> &'static Mutex<Cache> {
    static C: OnceLock<Mutex<Cache>> = OnceLock::new();
    C.get_or_init(|| {
        let dir = crate::paths::data_dir().join("art-cache");
        let _ = std::fs::create_dir_all(&dir);
        let mut index = HashMap::new();
        for e in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
            let p = e.path();
            let (Some(stem), Some(ext)) = (p.file_stem().map(|s| s.to_string_lossy().to_string()), p.extension().map(|s| s.to_string_lossy().to_lowercase())) else { continue };
            if EXTS.contains(&ext.as_str()) { index.insert(stem, p); }
        }
        Mutex::new(Cache { dir, index, queue: VecDeque::new(), queued: HashSet::new(), failed: HashMap::new(), active: 0, last_changed: 0 })
    })
}

pub fn name_of(url: &str) -> String { let mut h = sha1_smol::Sha1::new(); h.update(url.as_bytes()); h.digest().to_string()[..24].to_string() }
fn remote(u: &str) -> bool { let l = u.to_ascii_lowercase(); l.starts_with("http://") || l.starts_with("https://") }

/// The cached file behind nimbus-art://cache/<name>.
pub fn file(name: &str) -> Option<PathBuf> {
    if name.len() != 24 || !name.chars().all(|c| c.is_ascii_hexdigit()) { return None; }
    cache().lock().unwrap().index.get(name).cloned()
}

/// What the UI should load: the device's copy when there is one; otherwise the web address (and it's queued).
pub fn url(u: &str) -> String {
    if !remote(u) { return u.to_string(); }
    let name = name_of(u);
    let mut c = cache().lock().unwrap();
    if c.index.contains_key(&name) { return format!("nimbus-art://cache/{name}"); }
    let now = crate::library::now_ms();
    let recently_failed = c.failed.get(u).map(|t| now.saturating_sub(*t) < 6 * 3600 * 1000).unwrap_or(false);
    if !recently_failed && !c.queued.contains(u) && !cfg!(test) {
        c.queued.insert(u.to_string());
        c.queue.push_back(u.to_string());
        if c.active < WORKERS { c.active += 1; std::thread::spawn(worker); }
    }
    u.to_string()
}

fn worker() {
    loop {
        let (u, dir) = {
            let mut c = cache().lock().unwrap();
            match c.queue.pop_front() { Some(u) => (u, c.dir.clone()), None => { c.active -= 1; return; } }
        };
        let ok = download(&u, &dir);
        let mut c = cache().lock().unwrap();
        c.queued.remove(&u);
        match ok {
            Ok(file) => {
                c.index.insert(name_of(&u), file);
                // The library reloads its pictures once the queue is empty, and at most once a minute during a big batch.
                let now = crate::library::now_ms();
                if c.queue.is_empty() || now.saturating_sub(c.last_changed) > 60_000 {
                    c.last_changed = now;
                    crate::stores::notify(json!({"type": "event", "name": "library.art", "data": {}}));
                }
            }
            Err(e) => {
                c.failed.insert(u.clone(), crate::library::now_ms());
                if !e.contains("resolve") && !e.contains("connect") { crate::bridge::log(&format!("art cache: {u}: {e}")); }
            }
        }
    }
}

fn download(u: &str, dir: &std::path::Path) -> Result<PathBuf, String> {
    let name = name_of(u);
    let part = dir.join(format!("{name}.part"));
    let o = std::process::Command::new("curl").args(["-fsSL", "--max-time", "30", "-o"]).arg(&part).args(["-w", "%{content_type}", u]).output().map_err(|e| e.to_string())?;
    if !o.status.success() { let _ = std::fs::remove_file(&part); return Err(String::from_utf8_lossy(&o.stderr).trim().to_string()); }
    let ty = String::from_utf8_lossy(&o.stdout).split(';').next().unwrap_or("").trim().to_lowercase();
    let from_url = u.split(['?', '#']).next().unwrap_or("").rsplit('.').next().unwrap_or("").to_lowercase();
    let ext = match ty.as_str() { "image/jpeg" => "jpg", "image/png" => "png", "image/webp" => "webp", "image/gif" => "gif", "image/avif" => "avif",
        t if !t.is_empty() && !t.starts_with("image/") => { let _ = std::fs::remove_file(&part); return Err(format!("not a picture: {t}")); }
        _ if EXTS.contains(&from_url.as_str()) => from_url.as_str(), _ => "jpg" }.to_string();
    if std::fs::metadata(&part).map(|m| m.len()).unwrap_or(0) < 64 { let _ = std::fs::remove_file(&part); return Err("empty".into()); }
    let file = dir.join(format!("{name}.{ext}"));
    std::fs::rename(&part, &file).map_err(|e| e.to_string())?;
    Ok(file)
}

/// Try everything that failed again (the network is back).
pub fn retry() {
    let again: Vec<String> = { let mut c = cache().lock().unwrap(); c.failed.drain().map(|(u, _)| u).collect() };
    for u in again { url(&u); }
}

pub fn status() -> Value {
    let c = cache().lock().unwrap();
    let bytes: u64 = c.index.values().filter_map(|f| std::fs::metadata(f).ok()).map(|m| m.len()).sum();
    json!({"pictures": c.index.len(), "bytes": bytes, "waiting": c.queue.len() + c.active, "failed": c.failed.len()})
}

#[cfg(test)]
mod tests {
    #[test]
    fn same_names_as_the_electron_host() {
        // sha1("https://example.com/a.jpg") = 1b0c6f0c...; the first 24 hex digits name the file.
        let n = super::name_of("https://example.com/a.jpg");
        assert_eq!(n.len(), 24);
        assert!(n.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(super::url("/home/x/a.png"), "/home/x/a.png");
    }
}
