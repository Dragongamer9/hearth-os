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
/// Pictures the web says aren't there (404) aren't asked for again for a week, across restarts (every start asked
/// again for the same missing pictures).
const MISSING_FOR_MS: u64 = 7 * 24 * 3600 * 1000;

struct Cache { dir: PathBuf, index: HashMap<String, PathBuf>, queue: VecDeque<String>, queued: HashSet<String>, failed: HashMap<String, u64>, missing: HashMap<String, u64>, active: usize, last_changed: u64 }

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
        let saved: HashMap<String, u64> = std::fs::read(dir.join("missing.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        let missing = still_missing(saved, crate::library::now_ms());
        Mutex::new(Cache { dir, index, queue: VecDeque::new(), queued: HashSet::new(), failed: HashMap::new(), missing, active: 0, last_changed: 0 })
    })
}

pub fn name_of(url: &str) -> String { let mut h = sha1_smol::Sha1::new(); h.update(url.as_bytes()); h.digest().to_string()[..24].to_string() }
/// The pictures found missing in the last week (older ones are tried again).
fn still_missing(saved: HashMap<String, u64>, now: u64) -> HashMap<String, u64> { saved.into_iter().filter(|(_, t)| now.saturating_sub(*t) < MISSING_FOR_MS).collect() }
/// curl -f's answer for a picture that isn't there.
fn not_there(err: &str) -> bool { err.contains("error: 404") || err.contains("error: 410") }
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
    let recently_failed = c.failed.get(u).map(|t| now.saturating_sub(*t) < 6 * 3600 * 1000).unwrap_or(false)
        || c.missing.get(u).map(|t| now.saturating_sub(*t) < MISSING_FOR_MS).unwrap_or(false);
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
                if not_there(&e) {
                    c.missing.insert(u.clone(), crate::library::now_ms());
                    let _ = crate::paths::write_atomic(&c.dir.join("missing.json"), serde_json::to_string(&c.missing).unwrap_or_default().as_bytes());
                }
                if !e.contains("resolve") && !e.contains("connect") { crate::bridge::log(&format!("art cache: {u}: {e}")); }
            }
        }
    }
}

/// An address with the characters a URL can't carry as they are (space, quotes, brackets, non-ASCII) percent-encoded;
/// anything already encoded or legal is left alone.
pub fn encode_url(u: &str) -> String {
    let mut out = String::with_capacity(u.len());
    for b in u.bytes() {
        if b > 0x20 && b < 0x7f && !b"\"<>\\^`{|}".contains(&b) { out.push(b as char); } else { out.push_str(&format!("%{b:02X}")); }
    }
    out
}

fn download(u: &str, dir: &std::path::Path) -> Result<PathBuf, String> {
    let name = name_of(u);
    let part = dir.join(format!("{name}.part"));
    // Epic's art addresses can have spaces and other characters curl refuses ("URL rejected"): sent encoded.
    let wire = encode_url(u);
    let o = std::process::Command::new("curl").args(["-fsSL", "--max-time", "30", "-o"]).arg(&part).args(["-w", "%{content_type}", &wire]).output().map_err(|e| e.to_string())?;
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
    #[test]
    fn missing_pictures_are_remembered_for_a_week() {
        assert!(super::not_there("curl: (22) The requested URL returned error: 404"));
        assert!(!super::not_there("curl: (6) Could not resolve host: x"));
        let now = 30 * 24 * 3600 * 1000;
        let saved = std::collections::HashMap::from([("a".to_string(), now - 1000), ("b".to_string(), now - super::MISSING_FOR_MS - 1)]);
        let kept = super::still_missing(saved, now);
        assert!(kept.contains_key("a") && !kept.contains_key("b"));
    }
}

#[cfg(test)]
mod encode_tests {
    #[test]
    fn spaces_and_odd_characters_are_encoded() {
        assert_eq!(super::encode_url("https://cdn1.epicgames.com/a/EGS_TOEM_S1 (1)_2560x1440-8b"), "https://cdn1.epicgames.com/a/EGS_TOEM_S1%20(1)_2560x1440-8b");
        assert_eq!(super::encode_url("https://x/a%20b?c=1&d=2"), "https://x/a%20b?c=1&d=2");
        assert_eq!(super::encode_url("https://x/é"), "https://x/%C3%A9");
    }
}
