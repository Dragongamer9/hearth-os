//! A small HTTP/1.1 client (std only) for the services Hearth runs on the person's own PC: ComfyUI, Vaultwarden,
//! Nextcloud, Home Assistant, Voice Studio. They listen on 127.0.0.1 and this PC's Tailscale address only, so the
//! link is either inside the PC or WireGuard's (Tailscale encrypts it); plain http:// only. Each request is one short
//! thread; nothing polls. Bodies up to 64 MB.

use serde_json::Value;
use std::future::Future;
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

#[derive(Debug, Clone, Default)]
pub struct Resp {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Resp {
    pub fn ok(&self) -> bool {
        (200..300).contains(&self.status)
    }
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).to_string()
    }
    pub fn json(&self) -> Option<Value> {
        serde_json::from_slice(&self.body).ok()
    }
    pub fn header(&self, k: &str) -> Option<&str> {
        self.headers.iter().find(|(h, _)| h.eq_ignore_ascii_case(k)).map(|(_, v)| v.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct Req {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub timeout: u64,
}

impl Req {
    pub fn new(method: &str, url: &str) -> Req {
        Req { method: method.into(), url: url.into(), headers: Vec::new(), body: Vec::new(), timeout: 20 }
    }
    pub fn get(url: &str) -> Req {
        Req::new("GET", url)
    }
    pub fn header(mut self, k: &str, v: impl Into<String>) -> Req {
        self.headers.push((k.into(), v.into()));
        self
    }
    pub fn json(mut self, v: &Value) -> Req {
        self.body = v.to_string().into_bytes();
        self.header("Content-Type", "application/json")
    }
    pub fn form(mut self, pairs: &[(&str, &str)]) -> Req {
        self.body = pairs.iter().map(|(k, v)| format!("{}={}", enc(k), enc(v))).collect::<Vec<_>>().join("&").into_bytes();
        self.header("Content-Type", "application/x-www-form-urlencoded")
    }
    pub fn body(mut self, b: Vec<u8>, ctype: &str) -> Req {
        self.body = b;
        self.header("Content-Type", ctype)
    }
    pub fn bearer(self, token: &str) -> Req {
        self.header("Authorization", format!("Bearer {token}"))
    }
    pub fn basic(self, user: &str, pass: &str) -> Req {
        self.header("Authorization", format!("Basic {}", crate::b64::encode(format!("{user}:{pass}").as_bytes())))
    }
    pub fn timeout(mut self, s: u64) -> Req {
        self.timeout = s;
        self
    }
}

/// Percent-encoding for a form value or a path piece.
pub fn enc(s: &str) -> String {
    let mut o = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => o.push(b as char),
            _ => o.push_str(&format!("%{b:02X}")),
        }
    }
    o
}
/// A path with each piece encoded, the slashes kept.
pub fn enc_path(s: &str) -> String {
    s.split('/').map(enc).collect::<Vec<_>>().join("/")
}
pub fn dec(s: &str) -> String {
    let b = s.as_bytes();
    let mut o = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let (Some(h), Some(l)) = ((b[i + 1] as char).to_digit(16), (b[i + 2] as char).to_digit(16)) {
                o.push((h * 16 + l) as u8);
                i += 3;
                continue;
            }
        }
        o.push(if b[i] == b'+' { b' ' } else { b[i] });
        i += 1;
    }
    String::from_utf8_lossy(&o).to_string()
}

/// (host, port, path) of an http:// address.
pub fn split(url: &str) -> Result<(String, u16, String), String> {
    let rest = if let Some(r) = url.strip_prefix("http://") {
        r
    } else if url.starts_with("https://") {
        return Err("Hearth reaches this over Tailscale or on this PC with http:// (Tailscale already encrypts it)".into());
    } else {
        return Err(format!("not an http:// address: {url}"));
    };
    let (hp, path) = match rest.find('/') {
        Some(i) => (&rest[..i], rest[i..].to_string()),
        None => (rest, "/".to_string()),
    };
    let (host, port) = match hp.rsplit_once(':') {
        Some((h, p)) => (h.to_string(), p.parse::<u16>().map_err(|_| format!("bad port in {url}"))?),
        None => (hp.to_string(), 80),
    };
    if host.is_empty() {
        return Err(format!("no host in {url}"));
    }
    Ok((host, port, path))
}

pub fn send(r: &Req) -> Result<Resp, String> {
    let (host, port, path) = split(&r.url)?;
    let t = Duration::from_secs(r.timeout.max(1));
    let addr = (host.as_str(), port).to_socket_addrs().map_err(|e| format!("{host}: {e}"))?.next().ok_or_else(|| format!("{host}: no address"))?;
    let mut s = TcpStream::connect_timeout(&addr, Duration::from_secs(r.timeout.clamp(1, 8))).map_err(|e| format!("{host}:{port}: {e}"))?;
    let _ = s.set_read_timeout(Some(t));
    let _ = s.set_write_timeout(Some(t));
    let mut head = format!("{} {} HTTP/1.1\r\nHost: {}:{}\r\nConnection: close\r\nUser-Agent: Hearth\r\nAccept-Encoding: identity\r\n", r.method, path, host, port);
    if !r.body.is_empty() || matches!(r.method.as_str(), "POST" | "PUT" | "PATCH") {
        head.push_str(&format!("Content-Length: {}\r\n", r.body.len()));
    }
    for (k, v) in &r.headers {
        if k.contains(['\r', '\n']) || v.contains(['\r', '\n']) {
            return Err("a header with a line break".into());
        }
        head.push_str(&format!("{k}: {v}\r\n"));
    }
    head.push_str("\r\n");
    s.write_all(head.as_bytes()).and_then(|_| s.write_all(&r.body)).map_err(|e| format!("{host}:{port}: {e}"))?;
    let mut raw = Vec::new();
    let mut buf = [0u8; 16384];
    loop {
        match s.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                raw.extend_from_slice(&buf[..n]);
                if raw.len() > 64 << 20 {
                    return Err("the answer is too big".into());
                }
            }
            Err(e) if raw.is_empty() => return Err(format!("{host}:{port}: {e}")),
            Err(_) => break,
        }
    }
    parse(&raw)
}

pub fn parse(raw: &[u8]) -> Result<Resp, String> {
    let end = raw.windows(4).position(|w| w == b"\r\n\r\n").ok_or("no answer")?;
    let head = String::from_utf8_lossy(&raw[..end]).to_string();
    let mut lines = head.split("\r\n");
    let status = lines.next().and_then(|l| l.split_whitespace().nth(1)).and_then(|c| c.parse().ok()).ok_or("not HTTP")?;
    let headers: Vec<(String, String)> = lines.filter_map(|l| l.split_once(':')).map(|(k, v)| (k.trim().to_string(), v.trim().to_string())).collect();
    let mut body = raw[end + 4..].to_vec();
    if headers.iter().any(|(k, v)| k.eq_ignore_ascii_case("transfer-encoding") && v.to_ascii_lowercase().contains("chunked")) {
        body = unchunk(&body);
    }
    Ok(Resp { status, headers, body })
}

fn unchunk(b: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let Some(nl) = b[i..].windows(2).position(|w| w == b"\r\n") else { break };
        let line = String::from_utf8_lossy(&b[i..i + nl]).to_string();
        let n = usize::from_str_radix(line.split(';').next().unwrap_or("0").trim(), 16).unwrap_or(0);
        i += nl + 2;
        if n == 0 || i + n > b.len() {
            break;
        }
        out.extend_from_slice(&b[i..i + n]);
        i += n + 2;
    }
    out
}

/// `send` on its own thread; ready when it's answered.
pub fn go(r: Req) -> impl Future<Output = Result<Resp, String>> + Send + 'static {
    let (tx, rx) = futures::channel::oneshot::channel();
    let _ = std::thread::Builder::new().name("hearth-oss-http".into()).spawn(move || {
        let _ = tx.send(send(&r));
    });
    async move { rx.await.unwrap_or_else(|_| Err("it stopped".into())) }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn answers_are_read() {
        let r = parse(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nTransfer-Encoding: chunked\r\n\r\n4\r\n{\"a\"\r\n3\r\n:1}\r\n0\r\n\r\n").unwrap();
        assert_eq!(r.status, 200);
        assert_eq!(r.json().unwrap()["a"], 1);
        assert_eq!(split("http://100.64.0.2:8123/api/states").unwrap(), ("100.64.0.2".into(), 8123, "/api/states".into()));
        assert!(split("https://x").is_err());
        assert_eq!(enc("a b&c"), "a%20b%26c");
        assert_eq!(dec("a%20b%26c"), "a b&c");
    }
}
