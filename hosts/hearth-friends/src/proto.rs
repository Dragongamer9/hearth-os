//! The local socket's protocol: one JSON object per line.
//!
//!   → {"id": 1, "method": "friends", "args": {}}
//!   ← {"id": 1, "ok": true, "value": ...}        or {"id": 1, "ok": false, "error": "In plain words"}
//!   → {"id": 2, "method": "watch", "args": {"client": "gamemode"}}
//!   ← {"event": "message", "data": {...}}       (as things happen, until the connection closes)
//!
//! The socket is in $XDG_RUNTIME_DIR (only its user can open it) and is made 0600; the server also checks the
//! caller's user id. Hearth's core (hosts/hearth-core/src/matrix_friends.rs) and Mantel (`hearth-friends call`)
//! are its clients.

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

pub fn request(id: u64, method: &str, args: &Value) -> String {
    let mut s = json!({"id": id, "method": method, "args": args}).to_string();
    s.push('\n');
    s
}

pub fn reply(id: &Value, r: Result<Value, String>) -> String {
    let mut s = match r { Ok(v) => json!({"id": id, "ok": true, "value": v}), Err(e) => json!({"id": id, "ok": false, "error": crate::redact(&e)}) }.to_string();
    s.push('\n');
    s
}

pub fn event(name: &str, data: &Value) -> String {
    let mut s = json!({"event": name, "data": data}).to_string();
    s.push('\n');
    s
}

/// One call, blocking: connect, ask, read the answer. For `hearth-friends call` and the tests.
pub fn call(socket: &Path, method: &str, args: &Value, timeout: Duration) -> Result<Value, String> {
    let mut s = UnixStream::connect(socket).map_err(|e| format!("Hearth Friends isn't running ({e})"))?;
    s.set_read_timeout(Some(timeout)).ok();
    s.write_all(request(1, method, args).as_bytes()).map_err(|e| e.to_string())?;
    let mut line = String::new();
    BufReader::new(s).read_line(&mut line).map_err(|e| format!("No answer from Hearth Friends ({e})"))?;
    let v: Value = serde_json::from_str(&line).map_err(|_| "A garbled answer from Hearth Friends".to_string())?;
    if v["ok"] == true { Ok(v["value"].clone()) } else { Err(v["error"].as_str().unwrap_or("Failed").to_string()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lines() {
        let r = request(3, "send", &json!({"text": "hi"}));
        assert!(r.ends_with('\n') && !r[..r.len() - 1].contains('\n'));
        let v: Value = serde_json::from_str(&reply(&json!(3), Err("bad access_token=syt_abc".into()))).unwrap();
        assert_eq!(v["ok"], false);
        assert!(!v["error"].as_str().unwrap().contains("syt_abc"));
        let e: Value = serde_json::from_str(&event("typing", &json!({"roomId": "!a:b"}))).unwrap();
        assert_eq!(e["event"], "typing");
    }
}
