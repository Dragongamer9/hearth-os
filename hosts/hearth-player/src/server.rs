//! A small web server on 127.0.0.1 that hands the web view the files it plays. Both web views (WebKitGTK and
//! WebView2) stream <video>/<audio> from it with byte ranges (seeking), the page can draw its frames into a canvas
//! (screenshots, seek-bar thumbnails: it answers with CORS headers), and internet radio and podcast streams go
//! through it too (so an http:// stream plays in a page that isn't http, and no site's CORS rules get in the way).
//! Only files the app registered are served, under a random token, to this computer only.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

enum Entry {
    File(PathBuf),
    Bytes(Arc<Vec<u8>>, String),
}

pub struct Server {
    pub port: u16,
    token: String,
    next: AtomicU64,
    entries: Mutex<HashMap<String, Entry>>,
    by_path: Mutex<HashMap<PathBuf, String>>,
}

fn random_token() -> String {
    // Not for cryptography: enough that another program on this computer can't guess the address.
    let mut seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(1) as u64
        ^ (std::process::id() as u64).rotate_left(32);
    let addr = &seed as *const u64 as u64;
    seed ^= addr;
    let mut s = String::new();
    for _ in 0..24 {
        seed ^= seed << 13; seed ^= seed >> 7; seed ^= seed << 17;
        s.push(b"abcdefghijklmnopqrstuvwxyz0123456789"[(seed % 36) as usize] as char);
    }
    s
}

/// Encodes a path segment or query value.
pub fn url_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) { out.push(b as char); } else { out.push_str(&format!("%{b:02X}")); }
    }
    out
}

/// A byte range from a Range header ("bytes=100-", "bytes=100-199", "bytes=-500") for a file of `len` bytes.
pub fn parse_range(h: &str, len: u64) -> Option<(u64, u64)> {
    let r = h.trim().strip_prefix("bytes=")?;
    let r = r.split(',').next()?.trim();
    let (a, b) = r.split_once('-')?;
    if len == 0 { return None; }
    if a.is_empty() {
        let n: u64 = b.parse().ok()?;
        if n == 0 { return None; }
        return Some((len.saturating_sub(n), len - 1));
    }
    let start: u64 = a.parse().ok()?;
    if start >= len { return None; }
    let end = if b.is_empty() { len - 1 } else { b.parse::<u64>().ok()?.min(len - 1) };
    if end < start { return None; }
    Some((start, end))
}

impl Server {
    pub fn start() -> std::io::Result<Arc<Server>> {
        let listener = TcpListener::bind(("127.0.0.1", 0))?;
        let port = listener.local_addr()?.port();
        let s = Arc::new(Server { port, token: random_token(), next: AtomicU64::new(1), entries: Mutex::new(HashMap::new()), by_path: Mutex::new(HashMap::new()) });
        let s2 = s.clone();
        std::thread::spawn(move || {
            for conn in listener.incoming().flatten() {
                let s = s2.clone();
                std::thread::spawn(move || { let _ = s.handle(conn); });
            }
        });
        Ok(s)
    }

    fn base(&self) -> String { format!("http://127.0.0.1:{}/{}", self.port, self.token) }

    /// The address the page plays `path` from.
    pub fn file_url(&self, path: &Path) -> String {
        let path = path.to_path_buf();
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "file".into());
        let mut by = self.by_path.lock().unwrap();
        let id = by.entry(path.clone()).or_insert_with(|| {
            let id = self.next.fetch_add(1, Ordering::SeqCst).to_string();
            self.entries.lock().unwrap().insert(id.clone(), Entry::File(path.clone()));
            id
        }).clone();
        format!("{}/f/{}/{}", self.base(), id, url_encode(&name))
    }

    /// The address of some bytes (subtitles made into WebVTT, a song's cover picture).
    pub fn bytes_url(&self, bytes: Vec<u8>, mime: &str, name: &str) -> String {
        let id = self.next.fetch_add(1, Ordering::SeqCst).to_string();
        self.entries.lock().unwrap().insert(id.clone(), Entry::Bytes(Arc::new(bytes), mime.to_string()));
        format!("{}/f/{}/{}", self.base(), id, url_encode(name))
    }

    /// The address that streams an internet address through this server.
    pub fn proxy_url(&self, url: &str) -> String { format!("{}/p?u={}", self.base(), url_encode(url)) }

    fn handle(&self, conn: TcpStream) -> std::io::Result<()> {
        conn.set_read_timeout(Some(std::time::Duration::from_secs(20)))?;
        let mut reader = BufReader::new(conn.try_clone()?);
        let mut line = String::new();
        reader.read_line(&mut line)?;
        let mut parts = line.split_whitespace();
        let method = parts.next().unwrap_or("").to_string();
        let target = parts.next().unwrap_or("").to_string();
        let mut headers = HashMap::new();
        loop {
            let mut h = String::new();
            if reader.read_line(&mut h)? == 0 { break; }
            let h = h.trim_end();
            if h.is_empty() { break; }
            if let Some((k, v)) = h.split_once(':') { headers.insert(k.trim().to_ascii_lowercase(), v.trim().to_string()); }
        }
        let mut out = conn;
        if method == "OPTIONS" {
            return write!(out, "HTTP/1.1 204 No Content\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Headers: Range\r\nAccess-Control-Allow-Methods: GET, HEAD\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        }
        let head = method == "HEAD";
        if method != "GET" && !head { return simple(&mut out, 405, "Not allowed"); }
        let prefix = format!("/{}/", self.token);
        let Some(rest) = target.strip_prefix(&prefix) else { return simple(&mut out, 404, "Not here") };
        if let Some(q) = rest.strip_prefix("p?u=") {
            let url = crate::media::percent_decode(q);
            return proxy(&mut out, &url, headers.get("range").map(String::as_str), head);
        }
        let Some(rest) = rest.strip_prefix("f/") else { return simple(&mut out, 404, "Not here") };
        let id = rest.split('/').next().unwrap_or("");
        let entry = {
            let e = self.entries.lock().unwrap();
            match e.get(id) {
                Some(Entry::File(p)) => Some(Entry::File(p.clone())),
                Some(Entry::Bytes(b, m)) => Some(Entry::Bytes(b.clone(), m.clone())),
                None => None,
            }
        };
        match entry {
            None => simple(&mut out, 404, "Not here"),
            Some(Entry::Bytes(b, mime)) => send(&mut out, &mut std::io::Cursor::new(&b[..]), b.len() as u64, &mime, headers.get("range"), head),
            Some(Entry::File(p)) => {
                let Ok(mut f) = std::fs::File::open(&p) else { return simple(&mut out, 404, "That file isn't there any more") };
                let len = f.metadata().map(|m| m.len()).unwrap_or(0);
                send(&mut out, &mut f, len, crate::media::mime_for(&p), headers.get("range"), head)
            }
        }
    }
}

fn simple(out: &mut TcpStream, code: u16, text: &str) -> std::io::Result<()> {
    write!(out, "HTTP/1.1 {code} {text}\r\nAccess-Control-Allow-Origin: *\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{text}", text.len())
}

fn send<R: Read + Seek>(out: &mut TcpStream, src: &mut R, len: u64, mime: &str, range: Option<&String>, head: bool) -> std::io::Result<()> {
    let common = format!("Access-Control-Allow-Origin: *\r\nAccept-Ranges: bytes\r\nContent-Type: {mime}\r\nCache-Control: no-store\r\nConnection: close\r\n");
    let (start, end, status) = match range {
        Some(r) => match parse_range(r, len) {
            Some((a, b)) => (a, b, 206),
            None => return write!(out, "HTTP/1.1 416 Range Not Satisfiable\r\n{common}Content-Range: bytes */{len}\r\nContent-Length: 0\r\n\r\n"),
        },
        None => (0, len.saturating_sub(1), 200),
    };
    let n = if len == 0 { 0 } else { end - start + 1 };
    if status == 206 {
        write!(out, "HTTP/1.1 206 Partial Content\r\n{common}Content-Range: bytes {start}-{end}/{len}\r\nContent-Length: {n}\r\n\r\n")?;
    } else {
        write!(out, "HTTP/1.1 200 OK\r\n{common}Content-Length: {n}\r\n\r\n")?;
    }
    if head || n == 0 { return Ok(()); }
    src.seek(SeekFrom::Start(start))?;
    let mut left = n;
    let mut buf = vec![0u8; 256 * 1024];
    while left > 0 {
        let want = (buf.len() as u64).min(left) as usize;
        let got = src.read(&mut buf[..want])?;
        if got == 0 { break; }
        // The web view drops connections when it seeks: that's fine.
        if out.write_all(&buf[..got]).is_err() { break; }
        left -= got as u64;
    }
    Ok(())
}

fn proxy(out: &mut TcpStream, url: &str, range: Option<&str>, head: bool) -> std::io::Result<()> {
    if !(url.starts_with("http://") || url.starts_with("https://")) { return simple(out, 400, "Only web addresses"); }
    let mut req = crate::net::agent().get(url);
    if let Some(r) = range { req = req.set("Range", r); }
    let resp = match req.call() {
        Ok(r) => r,
        Err(ureq::Error::Status(code, r)) => { let _ = r; return simple(out, code, "The station or feed said no"); }
        Err(_) => return simple(out, 502, "Couldn't reach it"),
    };
    let status = resp.status();
    let mut h = format!("HTTP/1.1 {status} {}\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\nCache-Control: no-store\r\n", resp.status_text());
    for k in ["content-type", "content-length", "content-range", "accept-ranges"] {
        if let Some(v) = resp.header(k) { h.push_str(&format!("{k}: {v}\r\n")); }
    }
    h.push_str("\r\n");
    out.write_all(h.as_bytes())?;
    if head { return Ok(()); }
    let mut r = resp.into_reader();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = match r.read(&mut buf) { Ok(0) | Err(_) => break, Ok(n) => n };
        if out.write_all(&buf[..n]).is_err() { break; }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges() {
        assert_eq!(parse_range("bytes=0-", 100), Some((0, 99)));
        assert_eq!(parse_range("bytes=10-19", 100), Some((10, 19)));
        assert_eq!(parse_range("bytes=90-500", 100), Some((90, 99)));
        assert_eq!(parse_range("bytes=-30", 100), Some((70, 99)));
        assert_eq!(parse_range("bytes=100-", 100), None);
        assert_eq!(parse_range("bytes=5-2", 100), None);
        assert_eq!(parse_range("items=0-1", 100), None);
    }

    fn get(url: &str, range: Option<&str>) -> (u16, Vec<(String, String)>, Vec<u8>) {
        let rest = url.strip_prefix("http://").unwrap();
        let (host, path) = rest.split_once('/').unwrap();
        let mut s = TcpStream::connect(host).unwrap();
        let r = range.map(|r| format!("Range: {r}\r\n")).unwrap_or_default();
        write!(s, "GET /{path} HTTP/1.1\r\nHost: {host}\r\n{r}\r\n").unwrap();
        let mut all = Vec::new();
        s.read_to_end(&mut all).unwrap();
        let split = all.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
        let head = String::from_utf8_lossy(&all[..split]).into_owned();
        let code = head.split_whitespace().nth(1).unwrap().parse().unwrap();
        let hs = head.lines().skip(1).filter_map(|l| l.split_once(':')).map(|(a, b)| (a.to_ascii_lowercase(), b.trim().to_string())).collect();
        (code, hs, all[split + 4..].to_vec())
    }

    #[test]
    fn serves_files_with_ranges() {
        let s = Server::start().unwrap();
        let p = std::env::temp_dir().join(format!("hp-srv-{}.mp4", std::process::id()));
        std::fs::write(&p, (0..=255u8).cycle().take(1000).collect::<Vec<_>>()).unwrap();
        let url = s.file_url(&p);
        assert_eq!(url, s.file_url(&p), "the same file keeps its address");
        let (c, h, b) = get(&url, None);
        assert_eq!(c, 200);
        assert_eq!(b.len(), 1000);
        assert!(h.iter().any(|(k, v)| k == "access-control-allow-origin" && v == "*"));
        assert!(h.iter().any(|(k, v)| k == "content-type" && v == "video/mp4"));
        let (c, h, b) = get(&url, Some("bytes=256-259"));
        assert_eq!(c, 206);
        assert_eq!(b, vec![0, 1, 2, 3]);
        assert!(h.iter().any(|(k, v)| k == "content-range" && v == "bytes 256-259/1000"));
        let bad = url.replace(&s.token, "nottoken");
        assert_eq!(get(&bad, None).0, 404);
        let vtt = s.bytes_url(b"WEBVTT\n\n".to_vec(), "text/vtt", "a.vtt");
        assert_eq!(get(&vtt, None).2, b"WEBVTT\n\n");
        let _ = std::fs::remove_file(&p);
    }
}
