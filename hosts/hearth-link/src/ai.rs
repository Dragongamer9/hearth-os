//! Local AI over Hearth Link: a Hearth device with a strong graphics card runs a local model server you installed
//! yourself (Ollama, or llama.cpp's llama-server, or anything else that speaks Ollama's or OpenAI's HTTP API on this
//! device), and your other paired devices use it as if it ran on them.
//!
//! - **On the device with the model** (off by default): Settings › Hearth Link › Local AI › Share this device's AI
//!   (`ai.set {serve, backend}`). `backend` is the server's address on that device, only ever this device itself
//!   (`http://127.0.0.1:11434`, Ollama's, by default; llama-server is usually `:8080`). Nothing is opened on the
//!   network: paired devices reach it over Hearth Link's encrypted connection.
//! - **On another device** (`ai.use {server}`): Hearth Link listens on `http://127.0.0.1:11435` (or another free
//!   port, said in the answer) on that device only, and passes each request to the shared device. Apps point at that
//!   address as they would at a local Ollama or OpenAI-compatible server.
//! - Only model use goes through: GET and POST under `/api/` and `/v1/` (and `/`), never the requests that manage
//!   models (`/api/pull`, `/api/push`, `/api/delete`, `/api/create`, `/api/copy`, `/api/blobs`). A request's body is
//!   at most 8 MB, an answer at most 32 MB.
//! - This first version answers each request whole: a streamed answer (Ollama streams by default) arrives all at
//!   once, when the model has finished.
//!
//! Between devices: ai.info, ai.http {method, path, body, ctype}. From the UI: ai.status, ai.set, ai.use, ai.stop.

use crate::node::Node;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

pub const DEFAULT_BACKEND: &str = "http://127.0.0.1:11434";
/// The address apps on a device use (one more than Ollama's own, so both can be on one device).
pub const LOCAL_PORT: u16 = 11435;
pub const MAX_BODY: usize = 8 << 20;
pub const MAX_ANSWER: usize = 32 << 20;
/// A model can take a while to answer.
const ANSWER_WAIT: Duration = Duration::from_secs(600);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct AiConfig {
    pub serve: bool,
    pub backend: String,
}

impl Default for AiConfig { fn default() -> Self { AiConfig { serve: false, backend: DEFAULT_BACKEND.into() } } }

impl AiConfig {
    pub fn load(dir: &Path) -> AiConfig { std::fs::read(dir.join("ai.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default() }
    pub fn save(&self, dir: &Path) -> Result<(), String> { crate::write_atomic(&dir.join("ai.json"), serde_json::to_string_pretty(self).unwrap().as_bytes()) }
}

/// The model server's address: only plain http on this device itself (127.0.0.1, localhost or [::1]) with a port.
pub fn backend_addr(url: &str) -> Option<(String, u16)> {
    let rest = url.trim().strip_prefix("http://")?.trim_end_matches('/');
    if rest.contains('/') || rest.contains('@') { return None; }
    let (host, port) = rest.rsplit_once(':')?;
    let port: u16 = port.parse().ok().filter(|p| *p > 0)?;
    let host = match host { "127.0.0.1" | "localhost" => "127.0.0.1", "[::1]" => "::1", _ => return None };
    Some((host.to_string(), port))
}

/// Requests that use a model (never the ones that download, upload, change or delete models).
pub fn path_ok(method: &str, path: &str) -> bool {
    if !matches!(method, "GET" | "POST") || path.len() > 512 || !path.starts_with('/') { return false; }
    if path.chars().any(|c| c.is_whitespace() || c.is_control()) || path.contains("..") || path.contains('\\') { return false; }
    let p = path.split('?').next().unwrap_or("");
    const NEVER: [&str; 6] = ["/api/pull", "/api/push", "/api/delete", "/api/create", "/api/copy", "/api/blobs"];
    if NEVER.iter().any(|n| p == *n || p.starts_with(&format!("{n}/"))) { return false; }
    p == "/" || p.starts_with("/api/") || p.starts_with("/v1/")
}

/// A request's head: (method, path, headers with lower-case names).
pub fn parse_head(head: &[u8]) -> Option<(String, String, Vec<(String, String)>)> {
    let text = std::str::from_utf8(head).ok()?;
    let mut lines = text.split("\r\n");
    let mut first = lines.next()?.split(' ');
    let (method, path, ver) = (first.next()?.to_string(), first.next()?.to_string(), first.next()?);
    if !ver.starts_with("HTTP/1.") { return None; }
    let headers = lines.filter(|l| !l.is_empty()).filter_map(|l| l.split_once(':').map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_string()))).collect();
    Some((method, path, headers))
}

pub fn header<'a>(h: &'a [(String, String)], name: &str) -> Option<&'a str> { h.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str()) }

/// A chunked body, put back together.
pub fn dechunk(mut b: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    loop {
        let end = b.windows(2).position(|w| w == b"\r\n")?;
        let size_text = std::str::from_utf8(&b[..end]).ok()?;
        let size = usize::from_str_radix(size_text.split(';').next()?.trim(), 16).ok()?;
        b = &b[end + 2..];
        if size == 0 { return Some(out); }
        if b.len() < size + 2 { return None; }
        out.extend_from_slice(&b[..size]);
        b = &b[size + 2..];
    }
}

/// A whole answer from the model server: (status, content type, body).
pub fn parse_response(b: &[u8]) -> Result<(u16, String, Vec<u8>), String> {
    let end = b.windows(4).position(|w| w == b"\r\n\r\n").ok_or("The AI server's answer was cut off")?;
    let head = std::str::from_utf8(&b[..end]).map_err(|_| "The AI server's answer wasn't readable")?;
    let mut lines = head.split("\r\n");
    let status: u16 = lines.next().and_then(|l| l.split(' ').nth(1)).and_then(|s| s.parse().ok()).ok_or("The AI server's answer wasn't HTTP")?;
    let headers: Vec<(String, String)> = lines.filter_map(|l| l.split_once(':').map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_string()))).collect();
    let ctype = header(&headers, "content-type").unwrap_or("application/json").to_string();
    let rest = &b[end + 4..];
    let body = if header(&headers, "transfer-encoding").map(|t| t.to_ascii_lowercase().contains("chunked")).unwrap_or(false) {
        dechunk(rest).ok_or("The AI server's answer was cut off")?
    } else if let Some(n) = header(&headers, "content-length").and_then(|n| n.parse::<usize>().ok()) {
        rest.get(..n).ok_or("The AI server's answer was cut off")?.to_vec()
    } else { rest.to_vec() };
    Ok((status, ctype, body))
}

pub fn reason(status: u16) -> &'static str {
    match status { 200 => "OK", 201 => "Created", 400 => "Bad Request", 403 => "Forbidden", 404 => "Not Found", 413 => "Payload Too Large", 500 => "Internal Server Error", 502 => "Bad Gateway", 503 => "Service Unavailable", _ => "Status" }
}

pub fn http_answer(status: u16, ctype: &str, body: &[u8]) -> Vec<u8> {
    let mut out = format!("HTTP/1.1 {status} {}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", reason(status), body.len()).into_bytes();
    out.extend_from_slice(body);
    out
}

/// One request to the model server on this device, its whole answer.
async fn backend_call(backend: &str, method: &str, path: &str, ctype: &str, body: &[u8]) -> Result<(u16, String, Vec<u8>), String> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let (host, port) = backend_addr(backend).ok_or("The AI server's address must be on this device (http://127.0.0.1:port)")?;
    let mut s = tokio::time::timeout(Duration::from_secs(5), tokio::net::TcpStream::connect((host.as_str(), port))).await
        .map_err(|_| "The AI server didn't answer".to_string())?.map_err(|_| "The AI server isn't running on that device (is Ollama or llama-server started?)".to_string())?;
    let hosthdr = if host.contains(':') { format!("[{host}]:{port}") } else { format!("{host}:{port}") };
    let head = format!("{method} {path} HTTP/1.1\r\nHost: {hosthdr}\r\nAccept: */*\r\nConnection: close\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\n\r\n", body.len());
    s.write_all(head.as_bytes()).await.map_err(|e| e.to_string())?;
    s.write_all(body).await.map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    let mut buf = vec![0u8; 64 << 10];
    let read = async {
        loop {
            let n = s.read(&mut buf).await.map_err(|e| e.to_string())?;
            if n == 0 { return Ok::<(), String>(()); }
            out.extend_from_slice(&buf[..n]);
            if out.len() > MAX_ANSWER + 65536 { return Err("The AI server's answer is too big".to_string()); }
        }
    };
    tokio::time::timeout(ANSWER_WAIT, read).await.map_err(|_| "The AI server took too long".to_string())??;
    parse_response(&out)
}

/// The models the server has: Ollama's /api/tags, or the OpenAI-style /v1/models.
async fn models(backend: &str) -> Result<Vec<String>, String> {
    if let Ok((200, _, b)) = backend_call(backend, "GET", "/api/tags", "application/json", b"").await {
        if let Ok(v) = serde_json::from_slice::<Value>(&b) {
            if let Some(m) = v["models"].as_array() { return Ok(m.iter().filter_map(|x| x["name"].as_str().map(str::to_string)).collect()); }
        }
    }
    let (st, _, b) = backend_call(backend, "GET", "/v1/models", "application/json", b"").await?;
    if st != 200 { return Err(format!("The AI server said {st}")); }
    let v: Value = serde_json::from_slice(&b).map_err(|_| "The AI server's model list wasn't readable".to_string())?;
    Ok(v["data"].as_array().map(|d| d.iter().filter_map(|x| x["id"].as_str().map(str::to_string)).collect()).unwrap_or_default())
}

/// The graphics memory of this device's biggest card (AMD and Intel report it in sysfs), for the device list.
fn vram() -> Option<u64> {
    std::fs::read_dir("/sys/class/drm").ok()?.flatten()
        .filter_map(|e| std::fs::read_to_string(e.path().join("device/mem_info_vram_total")).ok()?.trim().parse::<u64>().ok())
        .max()
}

struct Using { server: String, name: String, port: u16, task: tokio::task::JoinHandle<()> }
fn using() -> &'static Mutex<Option<Using>> { static U: OnceLock<Mutex<Option<Using>>> = OnceLock::new(); U.get_or_init(|| Mutex::new(None)) }

impl Node {
    fn ai_cfg(&self) -> AiConfig { AiConfig::load(&self.dir) }

    pub async fn ai_answer(self: Arc<Self>, _peer: &str, op: &str, req: &Value) -> Result<Value, String> {
        let c = self.ai_cfg();
        match op {
            "ai.info" => {
                if !c.serve { return Ok(json!({"serving": false, "name": self.name()})); }
                let m = models(&c.backend).await;
                Ok(json!({"serving": true, "name": self.name(), "kind": self.kind, "vram": vram(), "models": m.as_ref().ok(), "error": m.err()}))
            }
            "ai.http" => {
                if !c.serve { return Err(format!("{} doesn't share its AI (Settings › Hearth Link › Local AI)", self.name())); }
                let method = req["method"].as_str().unwrap_or("GET");
                let path = req["path"].as_str().unwrap_or("");
                if !path_ok(method, path) { return Ok(json!({"status": 403, "ctype": "text/plain", "body": "Not shared over Hearth Link"})); }
                let body = req["body"].as_str().unwrap_or("");
                if body.len() > MAX_BODY { return Ok(json!({"status": 413, "ctype": "text/plain", "body": "Too big"})); }
                let ctype = req["ctype"].as_str().filter(|t| t.len() < 200 && !t.contains('\r') && !t.contains('\n')).unwrap_or("application/json");
                match backend_call(&c.backend, method, path, ctype, body.as_bytes()).await {
                    Ok((st, ct, b)) => Ok(json!({"status": st, "ctype": ct, "body": String::from_utf8_lossy(&b)})),
                    Err(e) => Ok(json!({"status": 502, "ctype": "text/plain", "body": e})),
                }
            }
            _ => Err(format!("Unknown request: {op}")),
        }
    }

    /// The local address for apps: each request passed to `server` over Hearth Link.
    async fn ai_listen(self: &Arc<Self>, server: &str) -> Result<u16, String> {
        let want = std::env::var("HEARTH_AI_PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(LOCAL_PORT);
        let l = match tokio::net::TcpListener::bind(("127.0.0.1", want)).await { Ok(l) => l, Err(_) => tokio::net::TcpListener::bind(("127.0.0.1", 0)).await.map_err(|e| e.to_string())? };
        let port = l.local_addr().map_err(|e| e.to_string())?.port();
        let (n, srv) = (self.clone(), server.to_string());
        let task = tokio::spawn(async move {
            loop {
                let Ok((sock, from)) = l.accept().await else { continue };
                if !from.ip().is_loopback() { continue; }
                let (n, srv) = (n.clone(), srv.clone());
                tokio::spawn(async move { if let Err(e) = n.ai_proxy(sock, &srv).await { eprintln!("hearth-link: ai: {e}"); } });
            }
        });
        let name = crate::lock(&self.peers).get(server).map(|p| p.name.clone()).unwrap_or_default();
        if let Some(old) = crate::lock(using()).replace(Using { server: server.to_string(), name, port, task }) { old.task.abort(); }
        Ok(port)
    }

    /// One app request: read, sent to the shared device, answered.
    async fn ai_proxy(self: Arc<Self>, mut sock: tokio::net::TcpStream, server: &str) -> Result<(), String> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let mut buf = Vec::new();
        let mut tmp = vec![0u8; 64 << 10];
        let head_end = loop {
            let n = tokio::time::timeout(Duration::from_secs(30), sock.read(&mut tmp)).await.map_err(|_| "closed".to_string())?.map_err(|e| e.to_string())?;
            if n == 0 { return Ok(()); }
            buf.extend_from_slice(&tmp[..n]);
            if let Some(p) = buf.windows(4).position(|w| w == b"\r\n\r\n") { break p; }
            if buf.len() > 64 << 10 { return sock.write_all(&http_answer(400, "text/plain", b"Bad request")).await.map_err(|e| e.to_string()); }
        };
        let Some((method, path, headers)) = parse_head(&buf[..head_end]) else { return sock.write_all(&http_answer(400, "text/plain", b"Bad request")).await.map_err(|e| e.to_string()) };
        if !path_ok(&method, &path) { return sock.write_all(&http_answer(403, "text/plain", b"Not shared over Hearth Link")).await.map_err(|e| e.to_string()); }
        let len = header(&headers, "content-length").and_then(|n| n.parse::<usize>().ok()).unwrap_or(0);
        if len > MAX_BODY || header(&headers, "transfer-encoding").is_some() { return sock.write_all(&http_answer(413, "text/plain", b"Send a body of at most 8 MB with a Content-Length")).await.map_err(|e| e.to_string()); }
        let mut body = buf[head_end + 4..].to_vec();
        while body.len() < len {
            let n = tokio::time::timeout(Duration::from_secs(30), sock.read(&mut tmp)).await.map_err(|_| "closed".to_string())?.map_err(|e| e.to_string())?;
            if n == 0 { return Ok(()); }
            body.extend_from_slice(&tmp[..n]);
        }
        body.truncate(len);
        let req = json!({"op": "ai.http", "method": method, "path": path, "ctype": header(&headers, "content-type").unwrap_or("application/json"), "body": String::from_utf8_lossy(&body)});
        let out = match tokio::time::timeout(ANSWER_WAIT, self.request(server, &req)).await {
            Ok(Ok(v)) => http_answer(v["status"].as_u64().unwrap_or(502) as u16, v["ctype"].as_str().filter(|t| !t.contains('\r') && !t.contains('\n')).unwrap_or("application/json"), v["body"].as_str().unwrap_or("").as_bytes()),
            Ok(Err(e)) => http_answer(502, "text/plain", e.trim_start_matches("connection: ").as_bytes()),
            Err(_) => http_answer(504, "text/plain", b"The shared AI took too long"),
        };
        sock.write_all(&out).await.map_err(|e| e.to_string())?;
        let _ = sock.shutdown().await;
        Ok(())
    }

    pub async fn ai_command(self: &Arc<Self>, cmd: &str, a: &Value) -> Result<Value, String> {
        match cmd {
            "ai.status" => {
                let c = self.ai_cfg();
                let peers: Vec<String> = crate::lock(&self.peers).list.iter().map(|p| p.id.clone()).collect();
                let mut set = tokio::task::JoinSet::new();
                for id in peers {
                    let n = self.clone();
                    set.spawn(async move { let r = tokio::time::timeout(Duration::from_secs(6), n.request(&id, &json!({"op": "ai.info"}))).await; (id, r) });
                }
                let mut servers = Vec::new();
                while let Some(r) = set.join_next().await {
                    if let Ok((id, Ok(Ok(mut v)))) = r { if v["serving"] == true { v["id"] = json!(id); servers.push(v); } }
                }
                let here = if c.serve { let m = models(&c.backend).await; json!({"models": m.as_ref().ok(), "error": m.err(), "vram": vram()}) } else { Value::Null };
                let u = crate::lock(using()).as_ref().map(|u| json!({"server": u.server, "name": u.name, "port": u.port, "url": format!("http://127.0.0.1:{}", u.port)}));
                Ok(json!({"serve": c.serve, "backend": c.backend, "here": here, "servers": servers, "using": u}))
            }
            "ai.set" => {
                let mut c = self.ai_cfg();
                if let Some(b) = a["backend"].as_str() {
                    let b = b.trim();
                    let b = if b.is_empty() { DEFAULT_BACKEND.to_string() } else { b.to_string() };
                    if backend_addr(&b).is_none() { return Err("Use the AI server's address on this device, like http://127.0.0.1:11434".into()); }
                    c.backend = b;
                }
                if let Some(s) = a["serve"].as_bool() { c.serve = s; }
                c.save(&self.dir)?;
                eprintln!("hearth-link: ai: sharing this device's AI {}", if c.serve { "on" } else { "off" });
                Ok(json!({"serve": c.serve, "backend": c.backend}))
            }
            "ai.use" => {
                let server = a["server"].as_str().filter(|s| !s.is_empty()).ok_or("Missing 'server'")?.to_string();
                if !crate::lock(&self.peers).is_paired(&server) { return Err("Not a paired device".into()); }
                let port = self.ai_listen(&server).await?;
                eprintln!("hearth-link: ai: using {server}'s AI at 127.0.0.1:{port}");
                Ok(json!({"port": port, "url": format!("http://127.0.0.1:{port}")}))
            }
            "ai.stop" => { if let Some(u) = crate::lock(using()).take() { u.task.abort(); } Ok(json!({"ok": true})) }
            _ => Err(format!("Unknown command: {cmd}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_this_devices_own_server() {
        assert_eq!(backend_addr("http://127.0.0.1:11434"), Some(("127.0.0.1".into(), 11434)));
        assert_eq!(backend_addr("http://localhost:8080/"), Some(("127.0.0.1".into(), 8080)));
        assert_eq!(backend_addr("http://[::1]:8080"), Some(("::1".into(), 8080)));
        for bad in ["http://192.168.1.5:11434", "https://127.0.0.1:11434", "http://127.0.0.1", "http://127.0.0.1:0", "http://127.0.0.1:11434/x",
                    "http://user@127.0.0.1:1", "http://evil.com:80", "ftp://127.0.0.1:21", "http://127.0.0.1.evil.com:80"] {
            assert!(backend_addr(bad).is_none(), "{bad}");
        }
    }

    #[test]
    fn only_using_models_goes_through() {
        assert!(path_ok("POST", "/api/generate") && path_ok("POST", "/api/chat") && path_ok("GET", "/api/tags"));
        assert!(path_ok("POST", "/v1/chat/completions") && path_ok("GET", "/v1/models") && path_ok("GET", "/"));
        for (m, p) in [("POST", "/api/pull"), ("POST", "/api/delete"), ("DELETE", "/api/delete"), ("POST", "/api/push"), ("POST", "/api/create"),
                       ("POST", "/api/copy"), ("POST", "/api/blobs/sha256:x"), ("PUT", "/api/generate"), ("GET", "/etc/passwd"), ("GET", "/api/../x"),
                       ("GET", "api/tags"), ("GET", "/api/ tags"), ("GET", "/metrics")] {
            assert!(!path_ok(m, p), "{m} {p}");
        }
        assert!(path_ok("GET", "/api/tags?x=1"));
    }

    #[test]
    fn http_pieces() {
        let (m, p, h) = parse_head(b"POST /api/chat HTTP/1.1\r\nHost: x\r\nContent-Length: 12\r\nContent-Type: application/json").unwrap();
        assert_eq!((m.as_str(), p.as_str()), ("POST", "/api/chat"));
        assert_eq!(header(&h, "content-length"), Some("12"));
        assert!(parse_head(b"garbage").is_none());
        assert_eq!(dechunk(b"5\r\nhello\r\n6;x=y\r\n world\r\n0\r\n\r\n").unwrap(), b"hello world");
        assert!(dechunk(b"5\r\nhel").is_none(), "cut off");
        let (st, ct, b) = parse_response(b"HTTP/1.1 200 OK\r\nContent-Type: application/x-ndjson\r\nTransfer-Encoding: chunked\r\n\r\n3\r\n{}\n\r\n0\r\n\r\n").unwrap();
        assert_eq!((st, ct.as_str(), b.as_slice()), (200, "application/x-ndjson", &b"{}\n"[..]));
        let (st, _, b) = parse_response(b"HTTP/1.0 404 Not Found\r\nContent-Length: 2\r\n\r\nnoEXTRA").unwrap();
        assert_eq!((st, b.as_slice()), (404, &b"no"[..]));
        let a = http_answer(200, "application/json", b"{}");
        assert!(a.starts_with(b"HTTP/1.1 200 OK\r\n") && a.ends_with(b"\r\n\r\n{}"));
        assert!(String::from_utf8_lossy(&a).contains("Content-Length: 2\r\n"));
    }

    #[test]
    fn settings_off_by_default() {
        let d = tempfile::tempdir().unwrap();
        let c = AiConfig::load(d.path());
        assert!(!c.serve, "off by default");
        assert_eq!(c.backend, DEFAULT_BACKEND);
        AiConfig { serve: true, backend: "http://127.0.0.1:8080".into() }.save(d.path()).unwrap();
        assert!(AiConfig::load(d.path()).serve);
    }
}
