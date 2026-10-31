//! Hearth Mods' worker: one hearth-mods process at a time holds the download queue, the Wabbajack installs and the
//! sign-ins waiting for a browser, and answers the others over a Unix socket. Game Mode (hearth-core
//! `hearth_mods.rs`) asks it for everything its Mods screens show; the Mods window is the worker when it's the first
//! one open, and hands its calls to the worker otherwise, so two processes never run the same queue.
//!
//! The socket is `$XDG_RUNTIME_DIR/hearth-mods.sock` (else `~/.local/share/nimbus/mods/hearth-mods.sock`), readable
//! by you only. One request a connection: a line of JSON `{"call": name, "args": {...}}`, answered with a line
//! `{"ok": true, "value": ...}` or `{"ok": false, "error": "..."}`. `hearth-mods --serve` is a worker without a window:
//! it stops by itself after IDLE with no calls, downloads or installs.

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// A worker without a window stops after this long with nothing to do.
pub const IDLE: Duration = Duration::from_secs(10 * 60);
/// The longest one request may take (a Nexus search, the Wabbajack gallery the first time).
pub const SLOW: Duration = Duration::from_secs(150);

pub fn socket_path() -> PathBuf {
    match std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).filter(|p| p.is_absolute() && p.is_dir()) {
        Some(d) => d.join("hearth-mods.sock"),
        None => crate::util::data_dir().join("mods").join("hearth-mods.sock"),
    }
}

/// Does a worker answer at `path`?
pub fn alive_at(path: &Path) -> bool { UnixStream::connect(path).is_ok() }
pub fn alive() -> bool { alive_at(&socket_path()) }

/// Becomes the worker: the socket, unless another worker already answers there (None).
pub fn bind_at(path: &Path) -> Option<UnixListener> {
    if alive_at(path) { return None; }
    if let Some(d) = path.parent() { let _ = std::fs::create_dir_all(d); }
    let _ = std::fs::remove_file(path);
    let l = UnixListener::bind(path).ok()?;
    #[cfg(unix)] { use std::os::unix::fs::PermissionsExt; let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)); }
    Some(l)
}
pub fn bind() -> Option<UnixListener> { bind_at(&socket_path()) }

/// The answer line for a result.
pub fn answer(r: Result<Value, String>) -> Value {
    match r { Ok(v) => json!({"ok": true, "value": v}), Err(e) => json!({"ok": false, "error": e}) }
}

/// Answers requests, each on its own thread; `last` is set to the time of the latest one (for the idle stop).
pub fn serve<F>(l: UnixListener, handle: F, last: Arc<AtomicU64>)
where F: Fn(&str, &Value) -> Result<Value, String> + Send + Sync + 'static {
    let handle = Arc::new(handle);
    std::thread::spawn(move || {
        for s in l.incoming().flatten() {
            last.store(crate::util::now_ms(), Ordering::SeqCst);
            let h = handle.clone();
            std::thread::spawn(move || { let _ = one(s, &*h); });
        }
    });
}

fn one(mut s: UnixStream, handle: &dyn Fn(&str, &Value) -> Result<Value, String>) -> std::io::Result<()> {
    s.set_read_timeout(Some(Duration::from_secs(10)))?;
    let mut line = String::new();
    BufReader::new(s.try_clone()?).take(4 << 20).read_line(&mut line)?;
    let req: Value = serde_json::from_str(line.trim()).unwrap_or(Value::Null);
    let r = match req["call"].as_str().filter(|c| !c.is_empty()) {
        Some(c) => {
            let args = if req["args"].is_object() { req["args"].clone() } else { json!({}) };
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| handle(c, &args))).unwrap_or_else(|_| Err("Hearth Mods had a problem with that. Try again.".into()))
        }
        None => Err("Not a request.".into()),
    };
    let mut out = serde_json::to_string(&answer(r)).unwrap_or_default();
    out.push('\n');
    s.write_all(out.as_bytes())
}

/// Asks the worker at `path`. Err: no worker there, or its error.
pub fn request_at(path: &Path, call: &str, args: &Value, timeout: Duration) -> Result<Value, String> {
    let mut s = UnixStream::connect(path).map_err(|_| "Hearth Mods isn't running.".to_string())?;
    s.set_read_timeout(Some(timeout)).map_err(|e| e.to_string())?;
    let mut line = serde_json::to_string(&json!({"call": call, "args": args})).unwrap_or_default();
    line.push('\n');
    s.write_all(line.as_bytes()).map_err(|e| e.to_string())?;
    let mut back = String::new();
    BufReader::new(s).take(64 << 20).read_line(&mut back).map_err(|_| "Hearth Mods took too long to answer.".to_string())?;
    let v: Value = serde_json::from_str(back.trim()).map_err(|_| "Hearth Mods gave no answer.".to_string())?;
    if v["ok"] == true { Ok(v["value"].clone()) } else { Err(v["error"].as_str().unwrap_or("Hearth Mods couldn't do that.").to_string()) }
}
pub fn request(call: &str, args: &Value) -> Result<Value, String> { request_at(&socket_path(), call, args, SLOW) }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn one_worker_answers_requests() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("w.sock");
        assert!(!alive_at(&p));
        let l = bind_at(&p).expect("first worker binds");
        let last = Arc::new(AtomicU64::new(0));
        serve(l, |c, a| match c { "echo" => Ok(a.clone()), "boom" => Err("no".into()), _ => panic!("bad") }, last.clone());
        assert!(alive_at(&p));
        assert!(bind_at(&p).is_none(), "a second worker must not take over");
        assert_eq!(request_at(&p, "echo", &json!({"x": 1}), Duration::from_secs(5)).unwrap(), json!({"x": 1}));
        assert_eq!(request_at(&p, "boom", &json!({}), Duration::from_secs(5)).unwrap_err(), "no");
        assert!(request_at(&p, "other", &json!({}), Duration::from_secs(5)).is_err(), "a panic is an error, not a hang");
        assert!(last.load(Ordering::SeqCst) > 0);
        #[cfg(unix)] { use std::os::unix::fs::PermissionsExt; assert_eq!(std::fs::metadata(&p).unwrap().permissions().mode() & 0o777, 0o600); }
        // A stale socket file (no worker) is replaced.
        let q = dir.path().join("stale.sock");
        drop(UnixListener::bind(&q).unwrap());
        assert!(!alive_at(&q));
        assert!(bind_at(&q).is_some());
    }
}
