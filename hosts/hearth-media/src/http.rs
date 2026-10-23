//! A very small HTTP server: enough for Hearth Camera's preview (the window shows the camera as a live MJPEG
//! picture from 127.0.0.1) and Hearth Share's viewer page (any browser on the network watches the screen).
//! One thread per connection; only GET.

use crate::hub::{ByteHub, FrameHub};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Request { pub method: String, pub path: String, pub query: String, pub peer: Option<SocketAddr> }

impl Request {
    /// A query parameter (?k=...), not decoded (the apps only use plain letters and digits).
    pub fn param(&self, k: &str) -> Option<String> {
        self.query.split('&').find_map(|kv| { let (a, b) = kv.split_once('=').unwrap_or((kv, "")); if a == k { Some(b.to_string()) } else { None } })
    }
}

/// The request line out of what the browser sent: "GET /x?y HTTP/1.1".
pub fn parse_request(head: &str) -> Option<(String, String, String)> {
    let line = head.lines().next()?;
    let mut w = line.split_whitespace();
    let method = w.next()?.to_string();
    let target = w.next()?;
    if !w.next()?.starts_with("HTTP/") { return None; }
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    Some((method, path.to_string(), query.to_string()))
}

fn read_head(s: &mut TcpStream) -> Option<String> {
    let mut buf = Vec::with_capacity(1024);
    let mut b = [0u8; 1024];
    while !buf.windows(4).any(|w| w == &b"\r\n\r\n"[..]) {
        let n = s.read(&mut b).ok()?;
        if n == 0 { return None; }
        buf.extend_from_slice(&b[..n]);
        if buf.len() > 16 * 1024 { return None; }
    }
    Some(String::from_utf8_lossy(&buf).to_string())
}

pub type Handler = Arc<dyn Fn(Request, TcpStream) + Send + Sync>;

/// Serves connections on `l` until `stop` is set (checked between connections; a closed listener ends it too).
pub fn serve(l: TcpListener, stop: Arc<AtomicBool>, h: Handler) {
    std::thread::spawn(move || {
        let _ = l.set_nonblocking(true);
        while !stop.load(Ordering::SeqCst) {
            match l.accept() {
                Ok((mut s, peer)) => {
                    let h = h.clone();
                    std::thread::spawn(move || {
                        let _ = s.set_nonblocking(false);
                        let _ = s.set_read_timeout(Some(Duration::from_secs(10)));
                        let _ = s.set_write_timeout(Some(Duration::from_secs(10)));
                        let _ = s.set_nodelay(true);
                        let Some(head) = read_head(&mut s) else { return };
                        let Some((method, path, query)) = parse_request(&head) else { let _ = respond(&mut s, "400 Bad Request", "text/plain", b"bad request"); return };
                        if method != "GET" && method != "HEAD" { let _ = respond(&mut s, "405 Method Not Allowed", "text/plain", b"GET only"); return; }
                        h(Request { method, path, query, peer: Some(peer) }, s);
                    });
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => std::thread::sleep(Duration::from_millis(40)),
                Err(_) => std::thread::sleep(Duration::from_millis(200)),
            }
        }
    });
}

pub fn respond(s: &mut TcpStream, status: &str, ctype: &str, body: &[u8]) -> std::io::Result<()> {
    let head = format!("HTTP/1.1 {status}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n", body.len());
    s.write_all(head.as_bytes())?;
    s.write_all(body)?;
    s.flush()
}

pub const BOUNDARY: &str = "hearthframe";

/// Streams pictures as multipart MJPEG (what an <img> shows live) until the viewer goes, `stop` is set, or no
/// picture comes for `idle`. `viewers` counts who's watching.
pub fn stream_mjpeg(s: &mut TcpStream, hub: &FrameHub, stop: &AtomicBool, viewers: &AtomicUsize, idle: Duration) {
    let head = format!("HTTP/1.1 200 OK\r\nContent-Type: multipart/x-mixed-replace; boundary={BOUNDARY}\r\nCache-Control: no-store\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n");
    if s.write_all(head.as_bytes()).is_err() { return; }
    viewers.fetch_add(1, Ordering::SeqCst);
    let mut seq = 0;
    // The newest picture straight away, so the viewer sees something before the next one.
    if let Some((n, f)) = hub.latest() { seq = n; if write_part(s, &f).is_err() { viewers.fetch_sub(1, Ordering::SeqCst); return; } }
    let mut waited = Duration::ZERO;
    while !stop.load(Ordering::SeqCst) {
        match hub.wait_newer(seq, Duration::from_millis(500)) {
            Some((n, f)) => { seq = n; waited = Duration::ZERO; if write_part(s, &f).is_err() { break; } }
            None => { waited += Duration::from_millis(500); if waited >= idle { break; } }
        }
    }
    viewers.fetch_sub(1, Ordering::SeqCst);
}

fn write_part(s: &mut TcpStream, f: &[u8]) -> std::io::Result<()> {
    s.write_all(format!("--{BOUNDARY}\r\nContent-Type: image/jpeg\r\nContent-Length: {}\r\n\r\n", f.len()).as_bytes())?;
    s.write_all(f)?;
    s.write_all(b"\r\n")?;
    s.flush()
}

/// Streams a live byte stream (MP3) as one long response.
pub fn stream_bytes(s: &mut TcpStream, hub: &ByteHub, ctype: &str, stop: &AtomicBool, viewers: &AtomicUsize) {
    let head = format!("HTTP/1.1 200 OK\r\nContent-Type: {ctype}\r\nCache-Control: no-store\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n");
    if s.write_all(head.as_bytes()).is_err() { return; }
    viewers.fetch_add(1, Ordering::SeqCst);
    let mut n = hub.next();
    let mut quiet = 0;
    while !stop.load(Ordering::SeqCst) && quiet < 20 {
        match hub.get(n, Duration::from_millis(500)) {
            Some((b, next)) => { n = next; quiet = 0; if s.write_all(&b).is_err() { break; } }
            None => quiet += 1,
        }
    }
    viewers.fetch_sub(1, Ordering::SeqCst);
}

/// Whether an address is on this PC or a home network (10/8, 172.16/12, 192.168/16, link-local, IPv6 ULA and
/// link-local): Hearth Share only answers those.
pub fn is_local(a: &std::net::IpAddr) -> bool {
    match a {
        std::net::IpAddr::V4(v) => v.is_loopback() || v.is_private() || v.is_link_local(),
        std::net::IpAddr::V6(v) => {
            if let Some(m) = v.to_ipv4_mapped() { return m.is_loopback() || m.is_private() || m.is_link_local(); }
            let s = v.segments()[0];
            v.is_loopback() || (s & 0xfe00) == 0xfc00 || (s & 0xffc0) == 0xfe80
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn request_lines() {
        assert_eq!(parse_request("GET /live.mjpg?k=ab HTTP/1.1\r\nHost: x\r\n\r\n"), Some(("GET".into(), "/live.mjpg".into(), "k=ab".into())));
        assert_eq!(parse_request("GET / HTTP/1.0\r\n\r\n"), Some(("GET".into(), "/".into(), "".into())));
        assert_eq!(parse_request("nonsense"), None);
        let r = Request { method: "GET".into(), path: "/".into(), query: "a=1&k=xyz".into(), peer: None };
        assert_eq!(r.param("k").as_deref(), Some("xyz"));
        assert_eq!(r.param("z"), None);
    }
    #[test]
    fn local_addresses() {
        for a in ["127.0.0.1", "192.168.1.20", "10.0.0.5", "172.20.1.1", "169.254.3.3", "::1", "fd00::1", "fe80::1", "::ffff:192.168.0.2"] {
            assert!(is_local(&a.parse().unwrap()), "{a}");
        }
        for a in ["8.8.8.8", "172.32.0.1", "2001:db8::1"] { assert!(!is_local(&a.parse().unwrap()), "{a}"); }
    }
    #[test]
    fn serves_a_stream() {
        let hub = FrameHub::new();
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        let stop = Arc::new(AtomicBool::new(false));
        let viewers = Arc::new(AtomicUsize::new(0));
        let (h2, s2, v2) = (hub.clone(), stop.clone(), viewers.clone());
        serve(l, stop.clone(), Arc::new(move |r: Request, mut s: TcpStream| {
            if r.path == "/live.mjpg" { stream_mjpeg(&mut s, &h2, &s2, &v2, Duration::from_secs(2)); } else { let _ = respond(&mut s, "404 Not Found", "text/plain", b"no"); }
        }));
        hub.publish(crate::jpeg::tests::frame(1));
        let mut c = TcpStream::connect(("127.0.0.1", port)).unwrap();
        c.write_all(b"GET /live.mjpg HTTP/1.1\r\n\r\n").unwrap();
        c.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        let mut got = vec![];
        let mut b = [0u8; 4096];
        let mut s = crate::jpeg::Splitter::new();
        let mut frames = 0;
        for _ in 0..50 {
            let n = c.read(&mut b).unwrap_or(0);
            if n == 0 { break; }
            got.extend_from_slice(&b[..n]);
            frames += s.push(&b[..n]).len();
            if frames > 0 { break; }
        }
        let text = String::from_utf8_lossy(&got);
        assert!(text.starts_with("HTTP/1.1 200 OK"), "{text}");
        assert!(text.contains("--hearthframe"));
        assert_eq!(frames, 1);
        stop.store(true, Ordering::SeqCst);
    }
}
