//! Sending to another device (LocalSend's upload API): prepare-upload with the list of files (the other side asks its
//! person; that can take a while), then one upload per accepted file, with progress. A text message is a single
//! text "file" with the text as its preview, as LocalSend sends it.

use crate::discovery::{base_url, client};
use crate::proto::{FileMeta, PrepareResponse, PrepareUpload};
use crate::service::{lock, Service};
use futures_util::StreamExt;
use serde_json::json;
use std::collections::HashMap;
use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

#[derive(Clone, Debug)]
pub struct Target {
    pub ip: IpAddr,
    pub port: u16,
    pub https: bool,
    pub fingerprint: Option<String>,
    pub alias: String,
}

pub enum What {
    Files(Vec<PathBuf>),
    Text(String),
}

#[derive(Debug, PartialEq)]
pub enum Outcome {
    /// Everything accepted was sent (count of files).
    Sent(usize),
    /// The other side said no.
    Declined,
    /// It wants a PIN (or the one given was wrong).
    NeedPin,
    Busy,
    Cancelled,
}

fn mime(name: &str) -> String { mime_guess::from_path(name).first_or_octet_stream().essence_str().to_string() }

pub async fn send(svc: Arc<Service>, target: Target, what: What, pin: Option<String>, cancel: CancellationToken) -> Result<Outcome, String> {
    let emit = |ev: &str, v: serde_json::Value| (svc.emit)(ev, v);
    // What's going: (id, path or text, name, size).
    let mut files: HashMap<String, FileMeta> = HashMap::new();
    let mut sources: HashMap<String, (Option<PathBuf>, Vec<u8>)> = HashMap::new();
    match &what {
        What::Files(paths) => {
            let all = tokio::task::spawn_blocking({ let p = paths.clone(); move || crate::files::collect(&p) }).await.map_err(|e| e.to_string())??;
            if all.is_empty() { return Err("There's nothing to send in that".into()); }
            for (path, name, size) in all {
                let id = crate::random_id(8);
                files.insert(id.clone(), FileMeta { id: id.clone(), file_type: mime(&name), file_name: name, size, ..Default::default() });
                sources.insert(id, (Some(path), vec![]));
            }
        }
        What::Text(t) => {
            let id = crate::random_id(8);
            files.insert(id.clone(), FileMeta { id: id.clone(), file_name: format!("{id}.txt"), size: t.len() as u64, file_type: "text/plain".into(), preview: Some(t.clone()), ..Default::default() });
            sources.insert(id, (None, t.as_bytes().to_vec()));
        }
    }
    let total: u64 = files.values().map(|f| f.size).sum();
    let count = files.len();
    let c = client(&svc, if target.https { target.fingerprint.clone() } else { None }, Duration::from_secs(5))?;
    let base = base_url(target.ip, target.port, target.https);
    emit("send-state", json!({"to": target.alias, "state": "asking", "count": count, "total": total}));
    let mut req = c.post(format!("{base}/api/localsend/v2/prepare-upload")).timeout(Duration::from_secs(600))
        .json(&PrepareUpload { info: svc.info(None), files: files.clone() });
    if let Some(p) = &pin { req = req.query(&[("pin", p)]); }
    let resp = tokio::select! {
        r = req.send() => r.map_err(|e| friendly(&e))?,
        _ = cancel.cancelled() => return Ok(Outcome::Cancelled),
    };
    let st = resp.status().as_u16();
    let accepted: PrepareResponse = match st {
        200 => resp.json().await.map_err(|e| format!("{} answered strangely: {e}", target.alias))?,
        204 => { emit("send-state", json!({"to": target.alias, "state": "done", "count": 0, "total": 0})); return Ok(Outcome::Sent(0)); }
        401 => return Ok(Outcome::NeedPin),
        403 => return Ok(Outcome::Declined),
        409 => return Ok(Outcome::Busy),
        429 => return Err(format!("{} is refusing for now (too many tries)", target.alias)),
        _ => return Err(format!("{} answered with an error ({st})", target.alias)),
    };
    let session = accepted.session_id.clone();
    let mut order: Vec<(String, String)> = accepted.files.into_iter().filter(|(id, _)| files.contains_key(id)).collect();
    order.sort_by(|a, b| files[&a.0].file_name.cmp(&files[&b.0].file_name));
    let sending_total: u64 = order.iter().map(|(id, _)| files[id].size).sum();
    let sent = Arc::new(AtomicU64::new(0));
    // Progress, five times a second, while the uploads run.
    let ticker = {
        let (svc, sent, alias, done) = (svc.clone(), sent.clone(), target.alias.clone(), cancel.child_token());
        let t = done.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! { _ = tokio::time::sleep(Duration::from_millis(200)) => {}, _ = t.cancelled() => return }
                (svc.emit)("send-progress", json!({"to": alias, "sent": sent.load(Ordering::Relaxed), "total": sending_total}));
            }
        });
        done
    };
    emit("send-state", json!({"to": target.alias, "state": "sending", "count": order.len(), "total": sending_total}));
    let mut n = 0;
    for (id, token) in &order {
        let meta = &files[id];
        let (path, text) = sources.remove(id).unwrap_or((None, vec![]));
        let body = match path {
            Some(p) => {
                let f = tokio::fs::File::open(&p).await.map_err(|e| format!("Can't read {}: {e}", p.display()))?;
                let counter = sent.clone();
                let stream = tokio_util::io::ReaderStream::with_capacity(f, 256 * 1024).map(move |chunk| {
                    if let Ok(c) = &chunk { counter.fetch_add(c.len() as u64, Ordering::Relaxed); }
                    chunk
                });
                reqwest::Body::wrap_stream(stream)
            }
            None => { sent.fetch_add(text.len() as u64, Ordering::Relaxed); reqwest::Body::from(text) }
        };
        let up = c.post(format!("{base}/api/localsend/v2/upload")).query(&[("sessionId", session.as_str()), ("fileId", id.as_str()), ("token", token.as_str())])
            .header(reqwest::header::CONTENT_LENGTH, meta.size).header(reqwest::header::CONTENT_TYPE, "application/octet-stream")
            .timeout(Duration::from_secs(24 * 3600)).body(body).send();
        let r = tokio::select! {
            r = up => r,
            _ = cancel.cancelled() => {
                let _ = c.post(format!("{base}/api/localsend/v2/cancel")).query(&[("sessionId", session.as_str())]).timeout(Duration::from_secs(3)).send().await;
                ticker.cancel();
                return Ok(Outcome::Cancelled);
            }
        };
        match r {
            Ok(r) if r.status().is_success() => n += 1,
            Ok(r) => { ticker.cancel(); return Err(format!("{} didn't take \"{}\" ({})", target.alias, meta.file_name, r.status())); }
            Err(e) => { ticker.cancel(); return Err(friendly(&e)); }
        }
    }
    ticker.cancel();
    emit("send-progress", json!({"to": target.alias, "sent": sending_total, "total": sending_total}));
    Ok(Outcome::Sent(n))
}

fn friendly(e: &reqwest::Error) -> String {
    if e.is_timeout() { return "The other device took too long to answer".into(); }
    if e.is_connect() { return format!("Couldn't reach the other device ({e})"); }
    e.to_string()
}

/// Sends from the window: one at a time; the window hears how it went ("send-done").
pub fn start(svc: Arc<Service>, target: Target, what: What, pin: Option<String>) -> Result<(), String> {
    let token = CancellationToken::new();
    {
        let mut g = lock(&svc.sending);
        if g.is_some() { return Err("Already sending something. Wait for it to finish, or cancel it.".into()); }
        *g = Some(token.clone());
    }
    svc.changed();
    tokio::spawn(async move {
        let alias = target.alias.clone();
        let r = send(svc.clone(), target, what, pin, token).await;
        lock(&svc.sending).take();
        let v = match r {
            Ok(Outcome::Sent(n)) => json!({"to": alias, "ok": true, "count": n}),
            Ok(Outcome::Declined) => json!({"to": alias, "ok": false, "declined": true, "error": format!("{alias} said no")}),
            Ok(Outcome::NeedPin) => json!({"to": alias, "ok": false, "needPin": true, "error": format!("{alias} wants a PIN")}),
            Ok(Outcome::Busy) => json!({"to": alias, "ok": false, "error": format!("{alias} is busy receiving something else")}),
            Ok(Outcome::Cancelled) => json!({"to": alias, "ok": false, "cancelled": true, "error": "Cancelled"}),
            Err(e) => json!({"to": alias, "ok": false, "error": e}),
        };
        (svc.emit)("send-done", v);
        svc.changed();
    });
    Ok(())
}

pub fn cancel(svc: &Service) {
    if let Some(t) = lock(&svc.sending).as_ref() { t.cancel(); }
}
