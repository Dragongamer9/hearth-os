//! The receiving side: LocalSend's HTTPS API (v2).
//!   POST /api/localsend/v2/register        a device introduces itself; we answer with who we are
//!   GET  /api/localsend/v2/info            who we are
//!   POST /api/localsend/v2/prepare-upload  "may I send you these files?": waits for the person's answer
//!   POST /api/localsend/v2/upload          one accepted file (sessionId, fileId, token)
//!   POST /api/localsend/v2/cancel          the sender gave up
//! Only local addresses get an answer at all (the connection is dropped before TLS otherwise). One transfer at a
//! time (409 to a second sender), a wrong PIN five times in ten minutes locks that address out (429), and uploads
//! must come from the address that asked, with that file's token, and be no bigger than announced.

use crate::proto::{FileMeta, PrepareResponse, PrepareUpload};
use crate::service::{lock, Decision, Receive, Service};
use bytes::Bytes;
use http_body_util::{BodyExt, Full, Limited};
use hyper::body::Incoming;
use hyper::{Method, Request, Response, StatusCode};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::convert::Infallible;
use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::AsyncWriteExt;
use tokio::sync::{oneshot, watch};

pub struct Conn {
    pub ip: IpAddr,
    /// The fingerprint of the certificate the other side showed, if any (proves who it is).
    pub cert_fp: Option<String>,
}

type Resp = Response<Full<Bytes>>;

fn status(code: StatusCode, msg: &str) -> Resp {
    let mut r = Response::new(Full::new(Bytes::from(json!({"message": msg}).to_string())));
    *r.status_mut() = code;
    r.headers_mut().insert(hyper::header::CONTENT_TYPE, hyper::header::HeaderValue::from_static("application/json"));
    r
}

fn json_resp(v: &impl serde::Serialize) -> Resp {
    let mut r = Response::new(Full::new(Bytes::from(serde_json::to_vec(v).unwrap_or_default())));
    r.headers_mut().insert(hyper::header::CONTENT_TYPE, hyper::header::HeaderValue::from_static("application/json"));
    r
}

/// The query string as a map (percent-decoded).
pub fn query(q: Option<&str>) -> HashMap<String, String> {
    let mut m = HashMap::new();
    for pair in q.unwrap_or("").split('&').filter(|p| !p.is_empty()) {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        m.insert(decode(k), decode(v));
    }
    m
}

fn decode(s: &str) -> String {
    let hexval = |c: u8| (c as char).to_digit(16).map(|d| d as u8);
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => { out.push(b' '); i += 1; }
            b'%' if i + 2 < b.len() => match (hexval(b[i + 1]), hexval(b[i + 2])) {
                (Some(h), Some(l)) => { out.push(h * 16 + l); i += 3; }
                _ => { out.push(b'%'); i += 1; }
            },
            c => { out.push(c); i += 1; }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

pub async fn run(svc: Arc<Service>, listener: tokio::net::TcpListener, acceptor: tokio_rustls::TlsAcceptor, mut stop: watch::Receiver<bool>) {
    loop {
        let (tcp, addr) = tokio::select! {
            r = listener.accept() => match r { Ok(x) => x, Err(_) => { tokio::time::sleep(Duration::from_millis(200)).await; continue; } },
            _ = stop.changed() => return,
        };
        let ip = crate::canonical(addr.ip());
        if !crate::is_local_ip(ip) { drop(tcp); continue; }
        let (svc, acceptor) = (svc.clone(), acceptor.clone());
        tokio::spawn(async move {
            let tls = match tokio::time::timeout(Duration::from_secs(15), acceptor.accept(tcp)).await { Ok(Ok(t)) => t, _ => return };
            let cert_fp = tls.get_ref().1.peer_certificates().and_then(|c| c.first()).map(|c| crate::tls::fingerprint(c.as_ref()));
            let conn = Arc::new(Conn { ip, cert_fp });
            let service = hyper::service::service_fn(move |req: Request<Incoming>| {
                let (svc, conn) = (svc.clone(), conn.clone());
                async move { Ok::<_, Infallible>(handle(svc, conn, req).await) }
            });
            let _ = hyper::server::conn::http1::Builder::new().keep_alive(true).serve_connection(hyper_util::rt::TokioIo::new(tls), service).await;
        });
    }
}

pub async fn handle(svc: Arc<Service>, conn: Arc<Conn>, req: Request<Incoming>) -> Resp {
    let path = req.uri().path().trim_end_matches('/').to_string();
    let q = query(req.uri().query());
    match (req.method().clone(), path.as_str()) {
        (Method::POST, "/api/localsend/v2/register") | (Method::POST, "/api/localsend/v1/register") => register(&svc, &conn, req).await,
        (Method::GET, "/api/localsend/v2/info") | (Method::GET, "/api/localsend/v1/info") => json_resp(&svc.info(None)),
        (Method::POST, "/api/localsend/v2/prepare-upload") => prepare(svc, conn, q, req).await,
        (Method::POST, "/api/localsend/v2/upload") => upload(svc, conn, q, req).await,
        (Method::POST, "/api/localsend/v2/cancel") => cancel(&svc, &conn, &q),
        (Method::POST, "/api/localsend/v2/prepare-download") | (Method::GET, "/api/localsend/v2/download") =>
            status(StatusCode::FORBIDDEN, "Hearth doesn't share files by link"),
        _ => status(StatusCode::NOT_FOUND, "Not found"),
    }
}

async fn read_json<T: serde::de::DeserializeOwned>(req: Request<Incoming>, max: usize) -> Result<T, Resp> {
    let body = Limited::new(req.into_body(), max).collect().await.map_err(|_| status(StatusCode::BAD_REQUEST, "Invalid body"))?.to_bytes();
    serde_json::from_slice(&body).map_err(|_| status(StatusCode::BAD_REQUEST, "Invalid body"))
}

async fn register(svc: &Arc<Service>, conn: &Conn, req: Request<Incoming>) -> Resp {
    let info: crate::proto::Info = match read_json(req, 64 * 1024).await { Ok(i) => i, Err(r) => return r };
    // A device that proves its certificate is remembered by it; one that doesn't, by what it says.
    let mut seen = info.clone();
    if let Some(fp) = &conn.cert_fp { seen.fingerprint = fp.clone(); }
    svc.saw(&seen, conn.ip);
    json_resp(&svc.info(None))
}

/// Clears the waiting request if the sender gives up while we're still asking (its connection closes, and this
/// handler is dropped with it).
struct AskGuard { svc: Arc<Service>, session: String, armed: bool }

impl Drop for AskGuard {
    fn drop(&mut self) {
        if !self.armed { return; }
        let gone = {
            let mut g = lock(&self.svc.receive);
            let still = g.as_ref().map(|r| r.session == self.session && r.decide.is_some()).unwrap_or(false);
            if still { g.take() } else { None }
        };
        if let Some(r) = gone {
            (self.svc.emit)("request-gone", json!({"sessionId": r.session, "from": r.info.alias}));
            self.svc.changed();
        }
    }
}

async fn prepare(svc: Arc<Service>, conn: Arc<Conn>, q: HashMap<String, String>, req: Request<Incoming>) -> Resp {
    // PIN, if one is set; too many wrong ones from an address and it has to wait.
    if let Some(pin) = lock(&svc.settings).pin.clone() {
        if lock(&svc.pin_fail).get(&conn.ip).map(|(n, _)| *n >= 5).unwrap_or(false) { return status(StatusCode::TOO_MANY_REQUESTS, "Too many wrong PINs"); }
        if q.get("pin").map(|p| p.as_str()) != Some(pin.as_str()) {
            if q.contains_key("pin") {
                let mut f = lock(&svc.pin_fail);
                let e = f.entry(conn.ip).or_insert((0, Instant::now()));
                e.0 += 1;
            }
            return status(StatusCode::UNAUTHORIZED, "PIN required");
        }
        lock(&svc.pin_fail).remove(&conn.ip);
    }
    let body: PrepareUpload = match read_json(req, 8 * 1024 * 1024).await { Ok(b) => b, Err(r) => return r };
    if body.files.is_empty() || body.files.len() > 20000 { return status(StatusCode::BAD_REQUEST, "Invalid body"); }
    let mut files: HashMap<String, FileMeta> = HashMap::new();
    for (k, mut f) in body.files {
        if f.id.is_empty() { f.id = k.clone(); }
        if f.id != k { return status(StatusCode::BAD_REQUEST, "Invalid body"); }
        files.insert(k, f);
    }
    // A text message: shown, nothing saved, no transfer needed (what LocalSend does).
    if files.len() == 1 {
        let f = files.values().next().cloned().unwrap_or_default();
        if f.is_message() {
            let text: String = f.preview.unwrap_or_default().chars().take(100_000).collect();
            let mut seen = body.info.clone();
            if let Some(fp) = &conn.cert_fp { seen.fingerprint = fp.clone(); }
            svc.saw(&seen, conn.ip);
            (svc.emit)("message", json!({"from": body.info.alias, "ip": conn.ip.to_string(), "verified": conn.cert_fp.as_deref().map(|c| c.eq_ignore_ascii_case(&body.info.fingerprint)).unwrap_or(false), "text": text}));
            let mut r = Response::new(Full::new(Bytes::new()));
            *r.status_mut() = StatusCode::NO_CONTENT;
            return r;
        }
    }
    let session = crate::random_id(16);
    let (tx, rx) = oneshot::channel();
    let total: u64 = files.values().map(|f| f.size).fold(0u64, |a, b| a.saturating_add(b));
    {
        let mut g = lock(&svc.receive);
        if g.is_some() { return status(StatusCode::CONFLICT, "Blocked by another session"); }
        *g = Some(Receive {
            session: session.clone(), ip: conn.ip, info: body.info.clone(), cert_fp: conn.cert_fp.clone(), files, decide: Some(tx),
            tokens: HashMap::new(), done: HashSet::new(), uploading: HashSet::new(), saved: vec![], received: 0, total,
            last: Instant::now(), cancelled: Arc::new(AtomicBool::new(false)),
        });
    }
    let mut guard = AskGuard { svc: svc.clone(), session: session.clone(), armed: true };
    let auto = svc.opts.answer.or_else(|| if svc.is_trusted(conn.cert_fp.as_deref()) { Some(true) } else { None });
    let decision = match auto {
        Some(true) => { lock(&svc.receive).as_mut().map(|r| r.decide.take()); Decision::Accept { ids: None, trust: false } }
        Some(false) => { lock(&svc.receive).as_mut().map(|r| r.decide.take()); Decision::Decline }
        None => {
            let ask = lock(&svc.receive).as_ref().map(|r| svc.describe(r)).unwrap_or_default();
            (svc.emit)("request", ask);
            svc.changed();
            match tokio::time::timeout(Duration::from_secs(300), rx).await {
                Ok(Ok(d)) => d,
                _ => Decision::Decline,
            }
        }
    };
    guard.armed = false;
    match decision {
        Decision::Decline => {
            let r = { let mut g = lock(&svc.receive); if g.as_ref().map(|r| r.session == session).unwrap_or(false) { g.take() } else { None } };
            if let Some(r) = r { (svc.emit)("declined", json!({"sessionId": r.session, "from": r.info.alias})); }
            svc.changed();
            status(StatusCode::FORBIDDEN, "Rejected")
        }
        Decision::Accept { ids, trust } => {
            let mut tokens = HashMap::new();
            let mut trust_who = None;
            {
                let mut g = lock(&svc.receive);
                let Some(r) = g.as_mut().filter(|r| r.session == session) else { return status(StatusCode::FORBIDDEN, "Rejected") };
                let chosen: Vec<String> = match ids { Some(v) => v.into_iter().filter(|i| r.files.contains_key(i)).collect(), None => r.files.keys().cloned().collect() };
                for id in chosen { tokens.insert(id, crate::random_id(16)); }
                if tokens.is_empty() { g.take(); return status(StatusCode::FORBIDDEN, "Rejected"); }
                r.total = r.files.iter().filter(|(k, _)| tokens.contains_key(*k)).map(|(_, f)| f.size).fold(0u64, |a, b| a.saturating_add(b));
                r.tokens = tokens.clone();
                r.last = Instant::now();
                if trust { if let Some(fp) = &r.cert_fp { if r.verified() { trust_who = Some((fp.clone(), r.info.alias.clone())); } } }
                let v = svc.describe(r);
                drop(g);
                (svc.emit)("receiving", v);
            }
            if let Some((fp, alias)) = trust_who { let _ = svc.trust(&fp, &alias, true); }
            svc.changed();
            json_resp(&PrepareResponse { session_id: session, files: tokens })
        }
    }
}

/// Removes a half-written file if the upload stops early.
struct PartGuard(Option<PathBuf>);
impl Drop for PartGuard {
    fn drop(&mut self) { if let Some(p) = self.0.take() { let _ = std::fs::remove_file(p); } }
}

async fn upload(svc: Arc<Service>, conn: Arc<Conn>, q: HashMap<String, String>, req: Request<Incoming>) -> Resp {
    let (Some(sid), Some(fid), Some(tok)) = (q.get("sessionId"), q.get("fileId"), q.get("token")) else { return status(StatusCode::BAD_REQUEST, "Missing parameters") };
    // Checked and claimed in one go: right session, address and token, not already being received.
    let (meta, cancelled) = {
        let mut g = lock(&svc.receive);
        let Some(r) = g.as_mut() else { return status(StatusCode::FORBIDDEN, "Invalid token or IP address") };
        if &r.session != sid || r.ip != conn.ip || r.tokens.get(fid) != Some(tok) || r.done.contains(fid) || r.uploading.contains(fid) {
            return status(StatusCode::FORBIDDEN, "Invalid token or IP address");
        }
        r.uploading.insert(fid.clone());
        r.last = Instant::now();
        (r.files.get(fid).cloned().unwrap_or_default(), r.cancelled.clone())
    };
    let result = save(&svc, &meta, &cancelled, sid, req).await;
    let mut finished = None;
    {
        let mut g = lock(&svc.receive);
        if let Some(r) = g.as_mut().filter(|r| &r.session == sid) {
            r.uploading.remove(fid);
            r.last = Instant::now();
            if let Ok(p) = &result {
                r.done.insert(fid.clone());
                r.saved.push(p.to_string_lossy().to_string());
                if r.done.len() >= r.tokens.len() { finished = g.take(); }
            }
        }
    }
    if let Some(r) = finished {
        (svc.emit)("received", json!({"sessionId": r.session, "from": r.info.alias, "saved": r.saved, "folder": svc.opts.dest.to_string_lossy(), "total": r.total}));
        svc.changed();
    }
    match result {
        Ok(_) => { let mut r = Response::new(Full::new(Bytes::new())); *r.status_mut() = StatusCode::OK; r }
        Err((code, msg)) => {
            (svc.emit)("file-failed", json!({"sessionId": sid, "file": meta.file_name, "error": msg}));
            status(code, &msg)
        }
    }
}

/// Writes one file into the receive folder: under a temporary name first, renamed when it's all there (and its
/// SHA-256 matches, when the sender gave one).
async fn save(svc: &Arc<Service>, meta: &FileMeta, cancelled: &Arc<AtomicBool>, sid: &str, req: Request<Incoming>) -> Result<PathBuf, (StatusCode, String)> {
    let internal = |m: String| (StatusCode::INTERNAL_SERVER_ERROR, m);
    let rel = crate::files::safe_relative(&meta.file_name).unwrap_or_else(|| PathBuf::from(format!("file-{}", &meta.id.chars().take(8).collect::<String>())));
    let root = svc.opts.dest.clone();
    let target = root.join(&rel);
    let dir = target.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| root.clone());
    tokio::fs::create_dir_all(&dir).await.map_err(|e| internal(format!("Can't make {}: {e}", dir.display())))?;
    if !crate::files::inside(&root, &dir) { return Err((StatusCode::BAD_REQUEST, "Bad file name".into())); }
    let part = dir.join(format!(".{}.nearby-part", crate::random_id(6)));
    let mut guard = PartGuard(Some(part.clone()));
    let mut file = tokio::fs::File::create(&part).await.map_err(|e| internal(format!("Can't write {}: {e}", part.display())))?;
    let mut hasher = meta.sha256.as_ref().filter(|s| s.len() == 64).map(|_| Sha256::new());
    let mut body = req.into_body();
    let mut written: u64 = 0;
    let mut last_emit = Instant::now();
    while let Some(frame) = body.frame().await {
        if cancelled.load(Ordering::SeqCst) { return Err((StatusCode::CONFLICT, "Cancelled".into())); }
        let frame = frame.map_err(|e| internal(format!("The connection broke: {e}")))?;
        let Ok(data) = frame.into_data() else { continue };
        written += data.len() as u64;
        if written > meta.size { return Err((StatusCode::BAD_REQUEST, "The file is bigger than announced".into())); }
        if let Some(h) = hasher.as_mut() { h.update(&data); }
        file.write_all(&data).await.map_err(|e| internal(format!("Can't write: {e}")))?;
        let n = data.len() as u64;
        let progress = {
            let mut g = lock(&svc.receive);
            g.as_mut().filter(|r| r.session == sid).map(|r| { r.received = r.received.saturating_add(n); r.last = Instant::now(); (r.received, r.total) })
        };
        if last_emit.elapsed() > Duration::from_millis(200) {
            last_emit = Instant::now();
            if let Some((received, total)) = progress { (svc.emit)("progress", json!({"sessionId": sid, "received": received, "total": total, "file": meta.file_name})); }
        }
    }
    file.flush().await.map_err(|e| internal(e.to_string()))?;
    drop(file);
    if written != meta.size { return Err((StatusCode::BAD_REQUEST, format!("Only {written} of {} bytes arrived", meta.size))); }
    if let (Some(h), Some(want)) = (hasher, meta.sha256.as_ref()) {
        if !crate::hex(&h.finalize()).eq_ignore_ascii_case(want) { return Err((StatusCode::UNPROCESSABLE_ENTITY, "Checksum mismatch".into())); }
    }
    let final_path = crate::files::unique(&target);
    tokio::fs::rename(&part, &final_path).await.map_err(|e| internal(format!("Can't save {}: {e}", final_path.display())))?;
    guard.0 = None;
    Ok(final_path)
}

fn cancel(svc: &Arc<Service>, conn: &Conn, q: &HashMap<String, String>) -> Resp {
    let ours = lock(&svc.receive).as_ref().map(|r| Some(&r.session) == q.get("sessionId") && r.ip == conn.ip).unwrap_or(false);
    if ours { svc.cancel_receive("The sender cancelled"); }
    let mut r = Response::new(Full::new(Bytes::new()));
    *r.status_mut() = StatusCode::OK;
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_query_strings() {
        let q = query(Some("sessionId=ab%20c&fileId=x+y&token=1%2F2&pin="));
        assert_eq!(q["sessionId"], "ab c");
        assert_eq!(q["fileId"], "x y");
        assert_eq!(q["token"], "1/2");
        assert_eq!(q["pin"], "");
        assert_eq!(query(Some("a=%zz&b=%4"))["a"], "%zz");
        assert_eq!(query(Some("b=%4"))["b"], "%4");
        assert!(query(None).is_empty());
    }
}
