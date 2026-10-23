// Map pieces (the style, tiles, fonts and icons) come through here: the page asks for hearth://<host>/<path>,
// this fetches https://<host>/<path> from the tile service only (it isn't a way to fetch anything else), at most 4
// at a time, with Hearth Maps' User-Agent, and keeps a copy of what was viewed so recently seen places still show
// offline. Copies are used while the service says they're fresh (Cache-Control / Expires), and after that only
// when the service can't be reached. The copies are kept under the size the services file allows (50 MB at most,
// OpenFreeMap's limit for offline copies); the least recently used go first.
use crate::services::{agent, cache_dir, Services};
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::PathBuf;
use std::sync::{Condvar, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

pub const SCHEME: &str = "hearth://";

#[derive(Debug, Serialize, Deserialize)]
struct Meta {
    url: String,
    ctype: String,
    expires: u64,
}

pub struct Fetched {
    pub data: Vec<u8>,
    pub ctype: String,
    /// From the copy kept here because the service couldn't be reached.
    pub stale: bool,
}

static SLOTS: (Mutex<usize>, Condvar) = (Mutex::new(0), Condvar::new());
const MAX_AT_ONCE: usize = 4;

struct Slot;
impl Slot {
    fn take() -> Slot {
        let (m, cv) = &SLOTS;
        let mut n = m.lock().unwrap_or_else(|e| e.into_inner());
        while *n >= MAX_AT_ONCE { n = cv.wait(n).unwrap_or_else(|e| e.into_inner()); }
        *n += 1;
        Slot
    }
}
impl Drop for Slot {
    fn drop(&mut self) {
        let (m, cv) = &SLOTS;
        let mut n = m.lock().unwrap_or_else(|e| e.into_inner());
        *n = n.saturating_sub(1);
        cv.notify_one();
    }
}

fn now() -> u64 { SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) }

pub fn fnv(s: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() { h ^= *b as u64; h = h.wrapping_mul(0x100000001b3); }
    format!("{h:016x}")
}

fn tiles_dir() -> PathBuf { cache_dir().join("tiles") }
fn paths(url: &str) -> (PathBuf, PathBuf) {
    let h = fnv(url);
    let d = tiles_dir().join(&h[..2]);
    (d.join(format!("{h}.bin")), d.join(format!("{h}.json")))
}

/// hearth://host/path → https://host/path, only for the tile service's own host.
pub fn real_url(s: &Services, asked: &str) -> Result<String, String> {
    let rest = asked.strip_prefix(SCHEME).ok_or("Not a map piece.")?;
    let host = rest.split(['/', '?']).next().unwrap_or("");
    if host.is_empty() || host != s.tiles_host() { return Err(format!("Map pieces only come from {}.", s.tiles_host())); }
    Ok(format!("https://{rest}"))
}

/// In JSON from the tile service (the style, TileJSON), its addresses point back through hearth://.
pub fn rewrite(s: &Services, text: &str) -> String {
    let host = s.tiles_host();
    text.replace(&format!("https://{host}"), &format!("{SCHEME}{host}"))
}

/// How long the service says a response stays fresh.
pub fn max_age(cache_control: &str) -> Option<u64> {
    let cc = cache_control.to_ascii_lowercase();
    if cc.contains("no-store") || cc.contains("no-cache") { return Some(0); }
    cc.split(',').find_map(|p| p.trim().strip_prefix("max-age=").and_then(|v| v.trim().parse().ok()))
}

pub fn fetch(s: &Services, asked: &str) -> Result<Fetched, String> {
    if !s.tiles_enabled { return Err(if s.notice.is_empty() { "The map service is switched off for now.".into() } else { s.notice.clone() }); }
    let url = real_url(s, asked)?;
    let (bin, meta) = paths(&url);
    let kept: Option<Meta> = std::fs::read_to_string(&meta).ok().and_then(|t| serde_json::from_str(&t).ok()).filter(|m: &Meta| m.url == url);
    if let Some(m) = &kept {
        if m.expires > now() {
            if let Ok(data) = std::fs::read(&bin) { touch(&bin); return Ok(Fetched { data: post(s, &m.ctype, data), ctype: m.ctype.clone(), stale: false }); }
        }
    }
    let got = {
        let _slot = Slot::take();
        agent().get(&url).call()
    };
    match got {
        Ok(r) => {
            let ctype = r.content_type().to_string();
            let age = max_age(r.header("Cache-Control").unwrap_or("")).unwrap_or(3600);
            let mut data = Vec::new();
            r.into_reader().take(32 << 20).read_to_end(&mut data).map_err(|e| e.to_string())?;
            if age > 0 && s.cache_mb > 0 {
                if let Some(d) = bin.parent() { let _ = std::fs::create_dir_all(d); }
                if std::fs::write(&bin, &data).is_ok() {
                    let _ = std::fs::write(&meta, serde_json::to_string(&Meta { url: url.clone(), ctype: ctype.clone(), expires: now() + age }).unwrap_or_default());
                    note_written(s, data.len() as u64);
                }
            }
            Ok(Fetched { data: post(s, &ctype, data), ctype, stale: false })
        }
        Err(ureq::Error::Status(code, _)) => Err(format!("The map service answered {code}.")),
        Err(e) => {
            // Offline: an old copy is better than nothing.
            if let (Some(m), Ok(data)) = (kept, std::fs::read(&bin)) { touch(&bin); return Ok(Fetched { data: post(s, &m.ctype, data), ctype: m.ctype, stale: true }); }
            Err(format!("Offline, and this part of the map wasn't seen before ({e})."))
        }
    }
}

fn post(s: &Services, ctype: &str, data: Vec<u8>) -> Vec<u8> {
    if ctype.contains("json") { if let Ok(t) = String::from_utf8(data.clone()) { return rewrite(s, &t).into_bytes(); } }
    data
}

fn touch(p: &std::path::Path) {
    if let Ok(f) = std::fs::File::options().append(true).open(p) { let _ = f.set_modified(SystemTime::now()); }
}

static WRITTEN: Mutex<u64> = Mutex::new(0);

/// Every 2 MB written, the copies are trimmed to the allowed size.
fn note_written(s: &Services, n: u64) {
    let mut w = WRITTEN.lock().unwrap_or_else(|e| e.into_inner());
    *w += n;
    if *w < 2 << 20 { return; }
    *w = 0;
    drop(w);
    let limit = s.cache_mb * 1024 * 1024;
    std::thread::spawn(move || { let _ = trim(limit); });
}

/// Deletes the least recently used copies until they fit in `limit` bytes. Returns the size left.
pub fn trim(limit: u64) -> std::io::Result<u64> {
    let mut files = Vec::new();
    let Ok(rd) = std::fs::read_dir(tiles_dir()) else { return Ok(0) };
    for d in rd.flatten() {
        let Ok(inner) = std::fs::read_dir(d.path()) else { continue };
        for f in inner.flatten() {
            let p = f.path();
            if p.extension().is_none_or(|x| x != "bin") { continue; }
            if let Ok(m) = f.metadata() { files.push((m.modified().unwrap_or(UNIX_EPOCH), m.len(), p)); }
        }
    }
    let mut total: u64 = files.iter().map(|f| f.1).sum();
    files.sort_by_key(|f| f.0);
    for (_, len, p) in files {
        if total <= limit { break; }
        let _ = std::fs::remove_file(&p);
        let _ = std::fs::remove_file(p.with_extension("json"));
        total = total.saturating_sub(len);
    }
    Ok(total)
}

/// How much the copies take now, in bytes.
pub fn usage() -> u64 {
    let mut total = 0;
    if let Ok(rd) = std::fs::read_dir(tiles_dir()) {
        for d in rd.flatten() { if let Ok(inner) = std::fs::read_dir(d.path()) { for f in inner.flatten() { total += f.metadata().map(|m| m.len()).unwrap_or(0); } } }
    }
    total
}

pub fn clear() { let _ = std::fs::remove_dir_all(tiles_dir()); }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_tile_service() {
        let s = Services::default();
        assert_eq!(real_url(&s, "hearth://tiles.openfreemap.org/planet/1/2/3.pbf").unwrap(), "https://tiles.openfreemap.org/planet/1/2/3.pbf");
        assert!(real_url(&s, "hearth://example.com/x").is_err());
        assert!(real_url(&s, "hearth://tiles.openfreemap.org.example.com/x").is_err());
        assert!(real_url(&s, "https://tiles.openfreemap.org/x").is_err());
    }

    #[test]
    fn json_points_back_here() {
        let s = Services::default();
        let t = r#"{"sprite":"https://tiles.openfreemap.org/sprites/ofm_f384/ofm","sources":{"openmaptiles":{"url":"https://tiles.openfreemap.org/planet"}},"x":"https://example.com/a"}"#;
        let r = rewrite(&s, t);
        assert!(r.contains("\"hearth://tiles.openfreemap.org/sprites/ofm_f384/ofm\""));
        assert!(r.contains("\"hearth://tiles.openfreemap.org/planet\""));
        assert!(r.contains("https://example.com/a"));
    }

    #[test]
    fn cache_control() {
        assert_eq!(max_age("public, max-age=86400"), Some(86400));
        assert_eq!(max_age("no-store"), Some(0));
        assert_eq!(max_age(""), None);
    }

    #[test]
    fn trimming_keeps_the_newest() {
        let dir = std::env::temp_dir().join(format!("hearth-maps-trim-{}", now()));
        std::env::set_var("HEARTH_MAPS_CACHE", &dir);
        let sub = tiles_dir().join("ab");
        std::fs::create_dir_all(&sub).unwrap();
        for i in 0..5u64 {
            let p = sub.join(format!("{i}.bin"));
            std::fs::write(&p, vec![0u8; 1000]).unwrap();
            let f = std::fs::File::options().append(true).open(&p).unwrap();
            f.set_modified(UNIX_EPOCH + std::time::Duration::from_secs(1_000_000 + i)).unwrap();
        }
        assert_eq!(trim(2500).unwrap(), 2000);
        assert!(!sub.join("0.bin").exists() && !sub.join("2.bin").exists() && sub.join("3.bin").exists() && sub.join("4.bin").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
