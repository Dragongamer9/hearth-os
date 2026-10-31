//! Hearth's download manager for legitimate downloads (docs/HYDRA-FEATURES.md): a web address you paste (a free or
//! open-source game's release, a file from your own store account's page), the free and open-source games in
//! Discover, and your itch.io library. A queue (one or more at a time), pause and resume (carries on where it
//! stopped, also after a restart), a speed limit, and then Add to library or Unpack.
//!
//! Each download is curl writing `<file>.part` with `-C -` (resume) and `--limit-rate`; only http and https, also
//! after redirects. Hearth has no torrent client, no file-hoster unlocking and no debrid services: it only downloads
//! what you're entitled to.
//!
//! Kept in downloads/direct.json: {settings: {limitKBps, parallel, folder}, jobs: [...]} (the queue's order is the
//! list's order).

use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};

pub struct Mgr {
    file: PathBuf,
    pub data: Value,
    kids: HashMap<String, Child>,
    /// Downloads being stopped on purpose: what each becomes once curl has gone ("paused", "queued", "cancel").
    stopping: HashMap<String, &'static str>,
    /// Sizes found by asking the server first (HEAD), filled in from other threads.
    totals: Arc<Mutex<HashMap<String, u64>>>,
    asked: HashSet<String>,
    speed: HashMap<String, (u64, u64, u64)>,   // id → (when ms, bytes then, bytes per second)
}

fn now_ms() -> u64 { crate::library::now_ms() }

/// Only http and https addresses, without spaces or control characters, at most 2000 characters.
pub fn valid_url(u: &str) -> bool {
    let u = u.trim();
    (u.starts_with("https://") || u.starts_with("http://")) && u.len() <= 2000 && u.len() > 8 && !u.chars().any(|c| c.is_whitespace() || c.is_control())
}

/// A safe file name from a name or an address: its last path part, decoded, with only harmless characters.
pub fn file_name(name: Option<&str>, url: &str) -> String {
    let from_url = || {
        let path = url.split(['?', '#']).next().unwrap_or("");
        let last = path.trim_end_matches('/').rsplit('/').next().unwrap_or("");
        crate::http::urldecode(last)
    };
    let raw = name.map(str::to_string).filter(|n| !n.trim().is_empty()).unwrap_or_else(from_url);
    let mut s: String = raw.chars().map(|c| if c.is_alphanumeric() || " ._-+()[]".contains(c) { c } else { '_' }).collect();
    s = s.trim().trim_start_matches('.').to_string();
    if s.is_empty() || s.chars().all(|c| c == '_' || c == '.') { s = "download".into(); }
    s.chars().take(150).collect()
}

/// `dir/name`, or `dir/name (2).ext` and so on when that's taken (also by a download in the list).
pub fn unique(dir: &Path, name: &str, taken: &HashSet<String>) -> PathBuf {
    let (stem, ext) = split_ext(name);
    for n in 1..1000 {
        let cand = if n == 1 { name.to_string() } else { format!("{stem} ({n}){ext}") };
        let p = dir.join(&cand);
        let ps = p.to_string_lossy().to_string();
        if !p.exists() && !Path::new(&format!("{ps}.part")).exists() && !taken.contains(&ps) { return p; }
    }
    dir.join(format!("{}-{name}", now_ms()))
}
/// "game.tar.gz" → ("game", ".tar.gz"); "Game.AppImage" → ("Game", ".AppImage").
pub fn split_ext(name: &str) -> (String, String) {
    let low = name.to_ascii_lowercase();
    for e in [".tar.gz", ".tar.xz", ".tar.bz2", ".tar.zst"] { if low.ends_with(e) { return (name[..name.len() - e.len()].to_string(), name[name.len() - e.len()..].to_string()); } }
    match name.rfind('.') { Some(i) if i > 0 => (name[..i].to_string(), name[i..].to_string()), _ => (name.to_string(), String::new()) }
}

/// What can be done with a finished file: "play" (a program: add it to the library), "unpack" (an archive), or "".
pub fn kind_of(name: &str) -> &'static str {
    let low = name.to_ascii_lowercase();
    if [".appimage", ".exe", ".sh", ".x86_64", ".x86", ".msi"].iter().any(|e| low.ends_with(e)) { return "play"; }
    if [".zip", ".tar.gz", ".tgz", ".tar.xz", ".tar.bz2", ".tar.zst", ".7z", ".tar"].iter().any(|e| low.ends_with(e)) { return "unpack"; }
    ""
}

/// curl's exit codes, in words.
pub fn curl_error(code: Option<i32>) -> String {
    match code {
        Some(6) => "Couldn't find that server. Check the address and your connection.".into(),
        Some(7) => "Couldn't connect to the server.".into(),
        Some(22) => "The server refused the download (the link may have expired, or needs a sign-in).".into(),
        Some(23) => "Couldn't write the file. Is the drive full?".into(),
        Some(28) => "The server stopped answering.".into(),
        Some(33) => "The server can't carry on from where it stopped. Remove it and download it again.".into(),
        Some(1) => "That kind of address isn't allowed (only http and https).".into(),
        Some(18) | Some(56) => "The connection dropped. Resume to carry on.".into(),
        Some(c) => format!("The download stopped (curl {c}). Resume to try again."),
        None => "The download stopped.".into(),
    }
}

/// The last Content-Length in curl -I's headers (after redirects).
pub fn content_length(headers: &str) -> Option<u64> {
    headers.lines().filter_map(|l| { let (k, v) = l.split_once(':')?; if k.trim().eq_ignore_ascii_case("content-length") { v.trim().parse().ok() } else { None } }).last().filter(|n| *n > 0)
}

impl Mgr {
    pub fn open(file: PathBuf, default_folder: PathBuf) -> Mgr {
        let mut data = std::fs::read(&file).ok().and_then(|b| serde_json::from_slice::<Value>(&b).ok()).filter(|v| v.is_object()).unwrap_or(json!({}));
        if !data["settings"].is_object() { data["settings"] = json!({}); }
        if !data["jobs"].is_array() { data["jobs"] = json!([]); }
        if data["settings"]["folder"].as_str().map(|f| f.is_empty()).unwrap_or(true) { data["settings"]["folder"] = json!(default_folder.to_string_lossy()); }
        if data["settings"]["parallel"].as_u64().is_none() { data["settings"]["parallel"] = json!(1); }
        if data["settings"]["limitKBps"].as_u64().is_none() { data["settings"]["limitKBps"] = json!(0); }
        // Hearth stopped while downloading: those carry on when it starts again.
        for j in data["jobs"].as_array_mut().unwrap() { if j["state"] == "downloading" { j["state"] = json!("queued"); } }
        Mgr { file, data, kids: HashMap::new(), stopping: HashMap::new(), totals: Default::default(), asked: HashSet::new(), speed: HashMap::new() }
    }
    fn save(&self) { let _ = crate::paths::write_atomic(&self.file, serde_json::to_string_pretty(&self.data).unwrap_or_default().as_bytes()); }
    pub fn folder(&self) -> PathBuf { PathBuf::from(self.data["settings"]["folder"].as_str().unwrap_or("/tmp")) }
    fn jobs(&mut self) -> &mut Vec<Value> { self.data["jobs"].as_array_mut().unwrap() }
    fn job(&mut self, id: &str) -> Option<&mut Value> { self.jobs().iter_mut().find(|j| j["id"] == id) }
    pub fn get(&self, id: &str) -> Option<Value> { self.data["jobs"].as_array().and_then(|a| a.iter().find(|j| j["id"] == id).cloned()) }
    pub fn active(&self) -> bool { self.data["jobs"].as_array().map(|a| a.iter().any(|j| j["state"] == "queued" || j["state"] == "downloading")).unwrap_or(false) || !self.kids.is_empty() }

    /// Puts a download in the queue. `source`: "url", "foss" or "itch" (shown on the page).
    pub fn add(&mut self, url: &str, name: Option<&str>, source: &str, total: Option<u64>) -> Result<Value, String> {
        let url = url.trim();
        if !valid_url(url) { return Err("That isn't a web address Hearth can download (it starts with https://).".into()); }
        if self.data["jobs"].as_array().unwrap().iter().any(|j| j["url"] == url && j["state"] != "done" && j["state"] != "failed") { return Err("That's in your downloads already.".into()); }
        let dir = self.folder();
        std::fs::create_dir_all(&dir).map_err(|e| format!("Can't use the download folder: {e}"))?;
        let taken: HashSet<String> = self.data["jobs"].as_array().unwrap().iter().filter_map(|j| j["file"].as_str().map(str::to_string)).collect();
        let name = file_name(name, url);
        let file = unique(&dir, &name, &taken);
        let id = format!("d{:x}", now_ms() * 1000 + (self.data["jobs"].as_array().unwrap().len() as u64 % 1000));
        let j = json!({"id": id, "url": url, "name": file.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or(name), "file": file.to_string_lossy(),
                       "state": "queued", "total": total, "done": 0, "added": now_ms(), "source": source, "kind": kind_of(&file.to_string_lossy())});
        self.jobs().push(j.clone());
        self.save();
        self.tick();
        Ok(j)
    }

    fn part(j: &Value) -> PathBuf { PathBuf::from(format!("{}.part", j["file"].as_str().unwrap_or("/tmp/x"))) }

    fn spawn(&mut self, id: &str) -> Result<(), String> {
        let j = self.get(id).ok_or("gone")?;
        let url = j["url"].as_str().unwrap_or("").to_string();
        if !valid_url(&url) { return Err("bad address".into()); }
        let part = Self::part(&j);
        if let Some(d) = part.parent() { let _ = std::fs::create_dir_all(d); }
        let mut args: Vec<String> = ["-sS", "-L", "--fail", "--proto", "=http,https", "--proto-redir", "=http,https", "-C", "-", "--retry", "3", "--retry-delay", "2",
            "--connect-timeout", "20", "--speed-time", "60", "--speed-limit", "1", "-A", "Hearth"].iter().map(|s| s.to_string()).collect();
        let limit = self.data["settings"]["limitKBps"].as_u64().unwrap_or(0);
        if limit > 0 { args.push("--limit-rate".into()); args.push(format!("{limit}k")); }
        args.push("-o".into()); args.push(part.to_string_lossy().to_string());
        args.push(url.clone());
        let child = Command::new("curl").args(&args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().map_err(|e| format!("curl: {e}"))?;
        self.kids.insert(id.to_string(), child);
        // The size, asked once, from another thread (the page shows a percentage once it's known).
        if j["total"].as_u64().is_none() && self.asked.insert(id.to_string()) {
            let (totals, id2) = (self.totals.clone(), id.to_string());
            std::thread::spawn(move || {
                if let Ok(h) = crate::system::run_timeout("curl", &["-sSIL", "--proto", "=http,https", "--proto-redir", "=http,https", "-m", "20", "-A", "Hearth", &url], std::time::Duration::from_secs(25)) {
                    if let Some(n) = content_length(&h) { totals.lock().unwrap().insert(id2, n); }
                }
            });
        }
        Ok(())
    }

    /// Checks on the running downloads and starts queued ones. Called every second while anything is active.
    pub fn tick(&mut self) {
        let mut changed = false;
        let found: Vec<(String, u64)> = self.totals.lock().unwrap().drain().collect();
        for (id, n) in found { if let Some(j) = self.job(&id) { j["total"] = json!(n); changed = true; } }
        // Finished or stopped curls.
        let ids: Vec<String> = self.kids.keys().cloned().collect();
        for id in ids {
            let st = match self.kids.get_mut(&id).map(|c| c.try_wait()) { Some(Ok(Some(s))) => Some(s), Some(Ok(None)) => None, _ => None };
            let Some(status) = st else { continue };
            self.kids.remove(&id);
            self.speed.remove(&id);
            changed = true;
            match self.stopping.remove(&id) {
                Some("cancel") => { if let Some(j) = self.get(&id) { let _ = std::fs::remove_file(Self::part(&j)); } self.jobs().retain(|j| j["id"] != id.as_str()); }
                Some(next) => { if let Some(j) = self.job(&id) { j["state"] = json!(next); } }
                None if status.success() => self.finish(&id),
                None => { let msg = curl_error(status.code()); if let Some(j) = self.job(&id) { j["state"] = json!("failed"); j["error"] = json!(msg); } }
            }
        }
        // Progress of the running ones.
        let now = now_ms();
        let running: Vec<String> = self.kids.keys().cloned().collect();
        for id in running {
            let Some(j) = self.get(&id) else { continue };
            let bytes = std::fs::metadata(Self::part(&j)).map(|m| m.len()).unwrap_or(0);
            // Bytes per second over the last second or so (kept between ticks closer together than that).
            let (bps, fresh) = match self.speed.get(&id).copied() {
                Some((t, b, _)) if now > t + 900 => (if bytes >= b { ((bytes - b) as f64 / ((now - t) as f64 / 1000.0)) as u64 } else { 0 }, true),
                Some((_, _, old)) => (old, false),
                None => (0, true),
            };
            if fresh { self.speed.insert(id.clone(), (now, bytes, bps)); }
            if let Some(j) = self.job(&id) { j["done"] = json!(bytes); j["bytesPerSec"] = json!(bps); }
        }
        // Start what's next, in the queue's order.
        let parallel = self.data["settings"]["parallel"].as_u64().unwrap_or(1).clamp(1, 3) as usize;
        let queued: Vec<String> = self.data["jobs"].as_array().unwrap().iter().filter(|j| j["state"] == "queued").filter_map(|j| j["id"].as_str().map(str::to_string)).collect();
        for id in queued {
            if self.kids.len() >= parallel { break; }
            match self.spawn(&id) {
                Ok(()) => { if let Some(j) = self.job(&id) { j["state"] = json!("downloading"); j["error"] = Value::Null; } }
                Err(e) => { if let Some(j) = self.job(&id) { j["state"] = json!("failed"); j["error"] = json!(e); } }
            }
            changed = true;
        }
        if changed { self.save(); }
    }

    fn finish(&mut self, id: &str) {
        let Some(j) = self.get(id) else { return };
        let part = Self::part(&j);
        let mut file = PathBuf::from(j["file"].as_str().unwrap_or(""));
        if file.exists() { let n = file.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(); file = unique(&self.folder(), &n, &HashSet::new()); }
        let size = std::fs::metadata(&part).map(|m| m.len()).unwrap_or(0);
        let r = std::fs::rename(&part, &file);
        if let Some(j) = self.job(id) {
            match r {
                Ok(()) => { j["state"] = json!("done"); j["done"] = json!(size); j["total"] = json!(size); j["file"] = json!(file.to_string_lossy()); j["at"] = json!(now_ms()); j["bytesPerSec"] = json!(0); }
                Err(e) => { j["state"] = json!("failed"); j["error"] = json!(format!("Couldn't finish the file: {e}")); }
            }
        }
    }

    fn stop(&mut self, id: &str, next: &'static str) -> bool {
        if let Some(c) = self.kids.get_mut(id) { let _ = c.kill(); self.stopping.insert(id.to_string(), next); return true; }
        false
    }
    pub fn pause(&mut self, id: &str) -> Result<(), String> {
        let st = self.get(id).ok_or("That download isn't in the list.")?["state"].as_str().unwrap_or("").to_string();
        match st.as_str() {
            "downloading" => { self.stop(id, "paused"); }
            "queued" => { if let Some(j) = self.job(id) { j["state"] = json!("paused"); } }
            _ => {}
        }
        self.save();
        self.tick();
        Ok(())
    }
    pub fn resume(&mut self, id: &str) -> Result<(), String> {
        let j = self.job(id).ok_or("That download isn't in the list.")?;
        if j["state"] == "paused" || j["state"] == "failed" { j["state"] = json!("queued"); j["error"] = Value::Null; }
        self.save();
        self.tick();
        Ok(())
    }
    /// Stops it and deletes what was downloaded so far (a finished file stays; it just leaves the list).
    pub fn cancel(&mut self, id: &str) -> Result<(), String> {
        let j = self.get(id).ok_or("That download isn't in the list.")?;
        if !self.stop(id, "cancel") {
            if j["state"] != "done" { let _ = std::fs::remove_file(Self::part(&j)); }
            self.jobs().retain(|x| x["id"] != id);
        }
        self.save();
        self.tick();
        Ok(())
    }
    /// Moves it to the front of the queue.
    pub fn first(&mut self, id: &str) -> Result<(), String> {
        let jobs = self.jobs();
        let i = jobs.iter().position(|j| j["id"] == id).ok_or("That download isn't in the list.")?;
        let j = jobs.remove(i);
        jobs.insert(0, j);
        self.save();
        Ok(())
    }
    /// The speed limit (KB/s, 0 = none) and how many at once (1 to 3). Running downloads restart with it (they carry on).
    pub fn set_settings(&mut self, limit_kbps: Option<u64>, parallel: Option<u64>, folder: Option<&str>) -> Result<(), String> {
        if let Some(l) = limit_kbps { self.data["settings"]["limitKBps"] = json!(l.min(10_000_000)); }
        if let Some(p) = parallel { self.data["settings"]["parallel"] = json!(p.clamp(1, 3)); }
        if let Some(f) = folder.filter(|f| !f.trim().is_empty()) {
            let p = PathBuf::from(f.trim());
            if !p.is_absolute() { return Err("Pick a folder.".into()); }
            std::fs::create_dir_all(&p).map_err(|e| format!("Can't use that folder: {e}"))?;
            self.data["settings"]["folder"] = json!(p.to_string_lossy());
        }
        let running: Vec<String> = self.kids.keys().cloned().collect();
        for id in running { self.stop(&id, "queued"); }
        self.save();
        Ok(())
    }
    /// Takes finished and failed ones off the list (their files stay).
    pub fn clear_done(&mut self) {
        self.jobs().retain(|j| j["state"] != "done");
        self.save();
    }

    /// The page's view: {settings, jobs: [... with pct], active, folder}.
    pub fn view(&self) -> Value {
        let jobs: Vec<Value> = self.data["jobs"].as_array().unwrap().iter().map(|j| {
            let mut j = j.clone();
            if let (Some(d), Some(t)) = (j["done"].as_u64(), j["total"].as_u64()) { if t > 0 { j["pct"] = json!(((d as f64 / t as f64) * 100.0).min(100.0).round() as u64); } }
            j
        }).collect();
        json!({"settings": self.data["settings"], "jobs": jobs, "active": self.active()})
    }
}

// ------------------------------------------------------------------ the one manager, ticking while busy

static MGR: std::sync::OnceLock<Mutex<Mgr>> = std::sync::OnceLock::new();
static TICKING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn state_file() -> PathBuf { crate::paths::data_dir().join("downloads/direct.json") }
pub fn default_folder() -> PathBuf { crate::paths::home().join("Downloads/Hearth") }

pub fn with<T>(f: impl FnOnce(&mut Mgr) -> T) -> T {
    let m = MGR.get_or_init(|| Mutex::new(Mgr::open(state_file(), default_folder())));
    let mut g = m.lock().unwrap_or_else(|e| e.into_inner());
    let out = f(&mut g);
    let busy = g.active();
    drop(g);
    if busy { ensure_ticking(); }
    out
}

fn ensure_ticking() {
    use std::sync::atomic::Ordering;
    if TICKING.swap(true, Ordering::SeqCst) { return; }
    std::thread::spawn(|| loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
        let m = MGR.get().unwrap();
        let mut g = m.lock().unwrap_or_else(|e| e.into_inner());
        g.tick();
        if !g.active() { TICKING.store(false, Ordering::SeqCst); return; }
    });
}

/// After a restart: queued downloads carry on (the page doesn't need to be opened first).
pub fn resume_at_start() { with(|m| m.tick()); }

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    /// A tiny web server for one file, with Range support (resume) and HEAD, sending in small pieces.
    fn serve(body: Vec<u8>) -> String {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        let body = Arc::new(body);
        std::thread::spawn(move || for s in l.incoming() {
            let Ok(mut s) = s else { continue };
            let body = body.clone();
            std::thread::spawn(move || {
                let mut req = Vec::new();
                let mut buf = [0u8; 1024];
                while !req.windows(4).any(|w| w == b"\r\n\r\n") { match s.read(&mut buf) { Ok(0) | Err(_) => return, Ok(n) => req.extend_from_slice(&buf[..n]) } }
                let text = String::from_utf8_lossy(&req).to_string();
                let head = text.starts_with("HEAD");
                if text.starts_with("GET /missing") { let _ = s.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"); return; }
                let from: usize = text.lines().find_map(|l| l.to_ascii_lowercase().strip_prefix("range: bytes=").map(|r| r.trim_end_matches('-').trim().parse().unwrap_or(0))).unwrap_or(0).min(body.len());
                let rest = &body[from..];
                let status = if from > 0 { format!("HTTP/1.1 206 Partial Content\r\nContent-Range: bytes {from}-{}/{}\r\n", body.len() - 1, body.len()) } else { "HTTP/1.1 200 OK\r\n".to_string() };
                let _ = s.write_all(format!("{status}Content-Length: {}\r\nAccept-Ranges: bytes\r\nConnection: close\r\n\r\n", rest.len()).as_bytes());
                if head { return; }
                for c in rest.chunks(16 * 1024) { if s.write_all(c).is_err() { return; } std::thread::sleep(std::time::Duration::from_millis(15)); }
            });
        });
        format!("http://127.0.0.1:{port}")
    }

    fn wait(m: &mut Mgr, id: &str, want: &str, secs: u64) -> Value {
        let t = std::time::Instant::now();
        loop {
            m.tick();
            let j = m.get(id).unwrap_or(json!({"state": "gone"}));
            if j["state"] == want || t.elapsed().as_secs() > secs { return j; }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }

    #[test]
    fn names_and_addresses() {
        assert!(valid_url("https://example.com/a.zip"));
        assert!(!valid_url("file:///etc/passwd"));
        assert!(!valid_url("ftp://x/y"));
        assert!(!valid_url("https://x/a b"));
        assert_eq!(file_name(None, "https://x.org/dl/Super%20Tux%20Kart-1.4.tar.xz?token=1#x"), "Super Tux Kart-1.4.tar.xz");
        assert_eq!(file_name(None, "https://x.org/"), "x.org");
        assert_eq!(file_name(Some("../../etc/passwd"), "https://x"), "_.._etc_passwd");
        assert_eq!(file_name(Some("..."), "https://x"), "download");
        assert_eq!(split_ext("game.tar.gz"), ("game".to_string(), ".tar.gz".to_string()));
        assert_eq!(split_ext("Game.AppImage"), ("Game".to_string(), ".AppImage".to_string()));
        assert_eq!(kind_of("OpenRA.AppImage"), "play");
        assert_eq!(kind_of("setup_witcher.exe"), "play");
        assert_eq!(kind_of("stk.tar.xz"), "unpack");
        assert_eq!(kind_of("readme.txt"), "");
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("a.zip"), b"x").unwrap();
        let mut taken = HashSet::new();
        assert_eq!(unique(d.path(), "a.zip", &taken), d.path().join("a (2).zip"));
        taken.insert(d.path().join("a (2).zip").to_string_lossy().to_string());
        assert_eq!(unique(d.path(), "a.zip", &taken), d.path().join("a (3).zip"));
        assert_eq!(content_length("HTTP/1.1 302 Found\r\nContent-Length: 0\r\n\r\nHTTP/1.1 200 OK\r\ncontent-length: 1234\r\n"), Some(1234));
        assert_eq!(content_length("HTTP/1.1 200 OK\r\n"), None);
        assert!(curl_error(Some(22)).contains("refused"));
    }

    #[test]
    fn downloads_pauses_resumes_and_finishes() {
        let body: Vec<u8> = (0..600_000u32).map(|i| (i * 7 % 251) as u8).collect();
        let base = serve(body.clone());
        let d = tempfile::tempdir().unwrap();
        let mut m = Mgr::open(d.path().join("direct.json"), d.path().join("dl"));
        // Slow, so it can be paused part way.
        m.set_settings(Some(60), Some(1), None).unwrap();
        let j = m.add(&format!("{base}/files/Free%20Game.tar.gz"), None, "url", None).unwrap();
        let id = j["id"].as_str().unwrap().to_string();
        assert_eq!(j["name"], "Free Game.tar.gz");
        assert_eq!(j["kind"], "unpack");
        assert!(m.add(&format!("{base}/files/Free%20Game.tar.gz"), None, "url", None).is_err(), "the same address twice");
        let j = wait(&mut m, &id, "downloading", 5);
        assert_eq!(j["state"], "downloading");
        std::thread::sleep(std::time::Duration::from_millis(2500));
        m.tick();
        m.pause(&id).unwrap();
        let j = wait(&mut m, &id, "paused", 10);
        assert_eq!(j["state"], "paused", "{j}");
        let part = PathBuf::from(format!("{}.part", j["file"].as_str().unwrap()));
        let got = std::fs::metadata(&part).map(|m| m.len()).unwrap_or(0);
        assert!(got > 0 && (got as usize) < body.len(), "paused part way: {got} bytes");
        assert_eq!(m.view()["jobs"][0]["total"], 600_000, "the size came from asking first");
        // A restart in between: the list is read back, then resumed without the limit.
        drop(m);
        let mut m = Mgr::open(d.path().join("direct.json"), d.path().join("dl"));
        assert_eq!(m.get(&id).unwrap()["state"], "paused");
        m.set_settings(Some(0), None, None).unwrap();
        m.resume(&id).unwrap();
        let j = wait(&mut m, &id, "done", 30);
        assert_eq!(j["state"], "done", "{j}");
        assert_eq!(std::fs::read(j["file"].as_str().unwrap()).unwrap(), body, "resumed, not started again: the file is whole and right");
        assert!(!part.exists());
        assert_eq!(m.view()["jobs"][0]["pct"], 100);
        // A link that doesn't work fails with words, and can be cancelled (taken off the list).
        let bad = m.add(&format!("{base}/missing/x.zip"), None, "url", None).unwrap();
        let bid = bad["id"].as_str().unwrap().to_string();
        let j = wait(&mut m, &bid, "failed", 15);
        assert_eq!(j["state"], "failed");
        assert!(j["error"].as_str().unwrap().contains("refused"), "{j}");
        m.cancel(&bid).unwrap();
        assert!(m.get(&bid).is_none());
        // Front of the queue, and clearing finished ones (the file stays).
        m.clear_done();
        assert!(m.get(&id).is_none());
        assert!(PathBuf::from(j["file"].as_str().unwrap()).parent().unwrap().exists());
    }

    #[test]
    fn queue_order_and_limits() {
        let d = tempfile::tempdir().unwrap();
        let mut m = Mgr::open(d.path().join("direct.json"), d.path().join("dl"));
        m.data["jobs"] = json!([{"id": "a", "state": "paused", "url": "https://x/a", "file": "/nope/a"}, {"id": "b", "state": "paused", "url": "https://x/b", "file": "/nope/b"}]);
        m.first("b").unwrap();
        assert_eq!(m.data["jobs"][0]["id"], "b");
        m.set_settings(Some(500), Some(9), None).unwrap();
        assert_eq!(m.data["settings"]["parallel"], 3);
        assert_eq!(m.data["settings"]["limitKBps"], 500);
        assert!(m.set_settings(None, None, Some("relative/dir")).is_err());
        assert!(m.first("zzz").is_err());
    }
}
