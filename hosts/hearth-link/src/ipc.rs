//! The UI's way in (Hearth's bridge, Mantel's settings page, `hearth-link call`): a Unix socket in the user's
//! runtime folder (folder 0700, socket 0600, and the other end must be this same user). One JSON object per line.
//! Root may also connect, for ROOT_COMMANDS only (hearth-update-mirror: bootc's update pieces from Hearth Storage,
//! and the hold-back check of background updates):
//!   in:  {"id": 1, "cmd": "status", "args": {...}}
//!   out: {"id": 1, "ok": true, "data": ...} or {"id": 1, "ok": false, "error": "..."}

use crate::node::Node;
use serde_json::{json, Value};
use std::path::Path;
use std::sync::Arc;

pub const ROOT_COMMANDS: [&str; 4] = ["storage.updates.mirror", "storage.updates.fetch", "storage.updates.drop", "storage.updates.hold"];

pub async fn serve(node: Arc<Node>, path: &Path) -> Result<(), String> {
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    let dir = path.parent().ok_or("no folder for the socket")?;
    if !dir.exists() { std::fs::DirBuilder::new().recursive(true).mode(0o700).create(dir).map_err(|e| format!("{}: {e}", dir.display()))?; }
    let _ = std::fs::remove_file(path);
    let listener = tokio::net::UnixListener::bind(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
    let me = unsafe { libc::getuid() };
    loop {
        let Ok((stream, _)) = listener.accept().await else { continue };
        let uid = stream.peer_cred().map(|c| c.uid()).ok();
        let root = uid == Some(0) && me != 0;
        if uid != Some(me) && !root { continue; }
        let node = node.clone();
        tokio::spawn(async move {
            let (rd, mut wr) = stream.into_split();
            let mut lines = BufReader::new(rd).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if line.len() > 1 << 20 { break; }
                let Ok(req) = serde_json::from_str::<Value>(&line) else { continue };
                let id = req["id"].clone();
                let cmd = req["cmd"].as_str().unwrap_or("").to_string();
                let args = req.get("args").cloned().unwrap_or(json!({}));
                let r = if root && !ROOT_COMMANDS.contains(&cmd.as_str()) { Err("Not a command for root".to_string()) } else { node.command(&cmd, &args).await };
                let reply = match r {
                    Ok(v) => json!({"id": id, "ok": true, "data": v}),
                    Err(e) => json!({"id": id, "ok": false, "error": e}),
                };
                if wr.write_all((reply.to_string() + "\n").as_bytes()).await.is_err() { break; }
            }
        });
    }
}

/// One request to the running service, and its answer. Blocking.
pub fn call(cmd: &str, args: Value, timeout_s: u64) -> Result<Value, String> {
    use std::io::{BufRead, BufReader, Write};
    let mut s = std::os::unix::net::UnixStream::connect(crate::socket_path()).map_err(|_| "Hearth Link isn't running".to_string())?;
    s.set_read_timeout(Some(std::time::Duration::from_secs(timeout_s))).ok();
    let req = json!({"id": 1, "cmd": cmd, "args": args}).to_string() + "\n";
    s.write_all(req.as_bytes()).map_err(|e| e.to_string())?;
    let mut r = BufReader::new(s);
    let mut line = String::new();
    if r.read_line(&mut line).map_err(|e| e.to_string())? == 0 { return Err("Hearth Link closed the connection".into()); }
    let v: Value = serde_json::from_str(&line).map_err(|e| e.to_string())?;
    if v["ok"] == true { Ok(v["data"].clone()) } else { Err(v["error"].as_str().unwrap_or("error").to_string()) }
}
