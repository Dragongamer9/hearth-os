//! The window (and `hearth-connect --call`) talking to the service: a Unix socket in the user's runtime folder
//! (folder 0700, socket 0600, and the other end must be this same user). One JSON object per line:
//!   to the service:   {"id": 1, "cmd": "pair", "args": {"id": "<device>"}}
//!   from the service: {"id": 1, "ok": true, "data": ...} or {"id": 1, "ok": false, "error": "..."}
//!                     and events for every connection: {"event": "pair-request", "data": {...}}

use crate::daemon::Daemon;
use serde_json::{json, Value};
use std::path::Path;
use std::sync::Arc;

#[cfg(unix)]
pub async fn serve(d: Arc<Daemon>, path: &Path, mut stop: tokio::sync::watch::Receiver<bool>) -> Result<(), String> {
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    let dir = path.parent().ok_or("no folder for the socket")?;
    if !dir.exists() { std::fs::DirBuilder::new().recursive(true).mode(0o700).create(dir).map_err(|e| format!("{}: {e}", dir.display()))?; }
    let _ = std::fs::remove_file(path);
    let listener = tokio::net::UnixListener::bind(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
    let me = unsafe { libc::getuid() };
    loop {
        let (stream, _) = tokio::select! {
            r = listener.accept() => match r { Ok(x) => x, Err(_) => continue },
            _ = stop.changed() => { let _ = std::fs::remove_file(path); return Ok(()); }
        };
        if stream.peer_cred().map(|c| c.uid()).ok() != Some(me) { continue; }
        let d = d.clone();
        tokio::spawn(async move {
            let (rd, mut wr) = stream.into_split();
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
            crate::lock(&d.clients).push(tx.clone());
            let writer = tokio::spawn(async move {
                while let Some(line) = rx.recv().await { if wr.write_all(line.as_bytes()).await.is_err() { break; } }
            });
            let mut lines = BufReader::new(rd).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if line.len() > 1 << 20 { break; }
                let Ok(req) = serde_json::from_str::<Value>(&line) else { continue };
                let id = req["id"].clone();
                let cmd = req["cmd"].as_str().unwrap_or("").to_string();
                let args = req.get("args").cloned().unwrap_or(json!({}));
                let reply = match d.command(&cmd, &args).await {
                    Ok(v) => json!({"id": id, "ok": true, "data": v}),
                    Err(e) => json!({"id": id, "ok": false, "error": e}),
                };
                if tx.send(reply.to_string() + "\n").is_err() { break; }
            }
            drop(tx);
            writer.abort();
        });
    }
}

/// One request to the running service, and its answer (for the window and --call). Blocking.
#[cfg(unix)]
pub fn call(cmd: &str, args: Value) -> Result<Value, String> {
    use std::io::{BufRead, BufReader, Write};
    let mut s = std::os::unix::net::UnixStream::connect(crate::socket_path()).map_err(|_| "Hearth Connect's service isn't running".to_string())?;
    s.set_read_timeout(Some(std::time::Duration::from_secs(20))).ok();
    let req = json!({"id": 1, "cmd": cmd, "args": args}).to_string() + "\n";
    s.write_all(req.as_bytes()).map_err(|e| e.to_string())?;
    let mut r = BufReader::new(s);
    let mut line = String::new();
    loop {
        line.clear();
        if r.read_line(&mut line).map_err(|e| e.to_string())? == 0 { return Err("The service closed the connection".into()); }
        let Ok(v) = serde_json::from_str::<Value>(&line) else { continue };
        if v.get("event").is_some() { continue; }
        if v["ok"].as_bool() == Some(true) { return Ok(v["data"].clone()); }
        return Err(v["error"].as_str().unwrap_or("error").to_string());
    }
}

/// Is the service there?
#[cfg(unix)]
pub fn running() -> bool { std::os::unix::net::UnixStream::connect(crate::socket_path()).is_ok() }

/// Every event from the service, to `on` (blocking; returns when the service goes away).
#[cfg(unix)]
pub fn events(on: impl Fn(Value)) -> Result<(), String> {
    use std::io::{BufRead, BufReader};
    let s = std::os::unix::net::UnixStream::connect(crate::socket_path()).map_err(|e| e.to_string())?;
    for line in BufReader::new(s).lines() {
        let Ok(line) = line else { break };
        if let Ok(v) = serde_json::from_str::<Value>(&line) { if v.get("event").is_some() { on(v); } }
    }
    Ok(())
}
