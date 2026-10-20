//! Quick Access over a game on the Handheld Daemon editions (Anatase, Ubuntu, CachyOS, Peppermint): HHD shows its
//! overlay app over the game and holds the controllers for it, and that app is Hearth's own Quick Access page
//! (desktop/shell/hearth-overlay, installed as hhd-ui, runs hosts/linux/src/overlay). It reaches the running Hearth
//! through this Unix socket: <runtime dir>/hearth/overlay.sock (/run/user/<uid>, since HHD starts the app without
//! XDG_RUNTIME_DIR). One JSON message per line each way: up, what the page posts (the same messages Hearth's own
//! windows send) and HHD's open requests; down, the replies and events Hearth sends its overlay window.
//! Only this user: the folder is 0700, the socket 0600, and every connection's other end is checked (SO_PEERCRED) to
//! be this user. Nothing on the network. Bazzite (InputPlumber, no HHD) doesn't install the app as hhd-ui, so none
//! of this runs there and Hearth keeps its own overlay window.

use serde_json::Value;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{sync_channel, SyncSender, TrySendError};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Where HHD finds its overlay app (the editions also point HHD_OVERLAY at the same program).
pub const HHD_UI: &str = "/usr/bin/hhd-ui";
const MAX_CLIENTS: usize = 4;
const MAX_LINE: usize = 16 * 1024 * 1024;
const QUEUE: usize = 1024;

extern "C" {
    fn getuid() -> u32;
    fn getsockopt(fd: i32, level: i32, name: i32, val: *mut std::ffi::c_void, len: *mut u32) -> i32;
}
const SOL_SOCKET: i32 = 1;
const SO_PEERCRED: i32 = 17;
#[repr(C)]
struct UCred { pid: i32, uid: u32, gid: u32 }

pub type OnMessage = Arc<dyn Fn(Value) + Send + Sync>;
pub type OnConnect = Arc<dyn Fn() + Send + Sync>;

/// The overlay app's connections: (id, its outgoing queue).
static CLIENTS: Mutex<Vec<(u64, SyncSender<String>)>> = Mutex::new(Vec::new());

pub fn uid() -> u32 { unsafe { getuid() } }

/// The user id at the other end of a connection.
pub fn peer_uid(s: &UnixStream) -> Option<u32> {
    use std::os::unix::io::AsRawFd;
    let mut c = UCred { pid: 0, uid: u32::MAX, gid: 0 };
    let mut len = std::mem::size_of::<UCred>() as u32;
    let r = unsafe { getsockopt(s.as_raw_fd(), SOL_SOCKET, SO_PEERCRED, &mut c as *mut UCred as *mut std::ffi::c_void, &mut len) };
    if r == 0 && len as usize == std::mem::size_of::<UCred>() { Some(c.uid) } else { None }
}

pub fn runtime_dir() -> PathBuf {
    let run = PathBuf::from(format!("/run/user/{}", uid()));
    if run.is_dir() { return run; }
    std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap_or(run)
}
pub fn socket_path() -> PathBuf { runtime_dir().join("hearth").join("overlay.sock") }

/// Is this program (or what it links to) Hearth's overlay app?
pub fn is_hearth_overlay(p: &Path) -> bool {
    std::fs::canonicalize(p).ok().and_then(|c| c.file_name().map(|n| n == "hearth-overlay")).unwrap_or(false)
}

/// Does Handheld Daemon show Quick Access over games here (its overlay app is Hearth's)? Not for extra multi-seat
/// players (HHD serves the main seat). HEARTH_OVERLAY=webkit / hhd overrides it, for testing.
pub fn hhd_hosts_overlay() -> bool {
    if std::env::var_os("HEARTH_SEAT").is_some() { return false; }
    match std::env::var("HEARTH_OVERLAY").as_deref() {
        Ok("webkit") => return false,
        Ok("hhd") => return true,
        _ => {}
    }
    is_hearth_overlay(Path::new(HHD_UI))
}

/// Is the overlay app connected?
pub fn connected() -> bool { !CLIENTS.lock().unwrap_or_else(|p| p.into_inner()).is_empty() }

/// A message for the overlay page (a reply or an event). Never waits: a message that doesn't fit in a stuck app's
/// queue is dropped.
pub fn send(v: &Value) {
    let mut c = CLIENTS.lock().unwrap_or_else(|p| p.into_inner());
    if c.is_empty() { return; }
    let line = v.to_string();
    c.retain(|(_, tx)| !matches!(tx.try_send(line.clone()), Err(TrySendError::Disconnected(_))));
}

fn remove(id: u64) { CLIENTS.lock().unwrap_or_else(|p| p.into_inner()).retain(|(i, _)| *i != id); }

/// The socket's folder: made if needed, this user's own and nobody else's.
fn prepare_dir(dir: &Path) -> Result<(), String> {
    use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
    if std::fs::symlink_metadata(dir).is_err() {
        std::fs::DirBuilder::new().recursive(true).mode(0o700).create(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let m = std::fs::symlink_metadata(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    if !m.is_dir() || m.uid() != uid() { return Err(format!("{} isn't a folder of this user's", dir.display())); }
    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700)).map_err(|e| format!("{}: {e}", dir.display()))
}

/// Listens at `path`. Every message from the overlay app goes to `on_message`; `on_connect` runs when it connects.
pub fn listen(path: &Path, on_message: OnMessage, on_connect: OnConnect) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let dir = path.parent().ok_or_else(|| "no folder for the overlay socket".to_string())?;
    prepare_dir(dir)?;
    let _ = std::fs::remove_file(path);
    let listener = UnixListener::bind(path).map_err(|e| format!("{}: {e}", path.display()))?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).map_err(|e| format!("{}: {e}", path.display()))?;
    std::thread::spawn(move || {
        let mut next: u64 = 0;
        for conn in listener.incoming() {
            let Ok(s) = conn else { continue };
            if peer_uid(&s) != Some(uid()) { crate::bridge::log("overlay link: refused a connection from another user"); continue; }
            if CLIENTS.lock().unwrap_or_else(|p| p.into_inner()).len() >= MAX_CLIENTS { continue; }
            next += 1;
            serve(next, s, on_message.clone(), on_connect.clone());
        }
    });
    Ok(())
}

/// Listens at the usual place (socket_path()); answers where.
pub fn start(on_message: OnMessage, on_connect: OnConnect) -> Result<PathBuf, String> {
    let path = socket_path();
    listen(&path, on_message, on_connect)?;
    Ok(path)
}

fn serve(id: u64, s: UnixStream, on_message: OnMessage, on_connect: OnConnect) {
    let Ok(mut writer) = s.try_clone() else { return };
    let (tx, rx) = sync_channel::<String>(QUEUE);
    CLIENTS.lock().unwrap_or_else(|p| p.into_inner()).push((id, tx));
    // Writes on their own thread, so whoever sends (maybe holding the bridge) never waits on the app.
    std::thread::spawn(move || {
        let _ = writer.set_write_timeout(Some(Duration::from_secs(5)));
        for line in rx {
            if writer.write_all(line.as_bytes()).and_then(|_| writer.write_all(b"\n")).is_err() { break; }
        }
        let _ = writer.shutdown(std::net::Shutdown::Both);
    });
    std::thread::spawn(move || {
        on_connect();
        let mut r = BufReader::new(s);
        let mut buf = Vec::new();
        loop {
            buf.clear();
            match (&mut r).take(MAX_LINE as u64 + 1).read_until(b'\n', &mut buf) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    if buf.len() > MAX_LINE { break; }
                    let text = String::from_utf8_lossy(&buf);
                    let t = text.trim();
                    if t.is_empty() { continue; }
                    if let Ok(v) = serde_json::from_str::<Value>(t) { on_message(v); }
                }
            }
        }
        remove(id);
        crate::bridge::log("overlay link: Handheld Daemon's overlay app disconnected");
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn knows_hearths_overlay_app() {
        let d = tempfile::tempdir().unwrap();
        let real = d.path().join("hearth-overlay");
        std::fs::write(&real, "#!/bin/sh\n").unwrap();
        let link = d.path().join("hhd-ui");
        std::os::unix::fs::symlink(&real, &link).unwrap();
        assert!(is_hearth_overlay(&link));
        assert!(is_hearth_overlay(&real));
        let other = d.path().join("hhd-ui.AppImage");
        std::fs::write(&other, "x").unwrap();
        assert!(!is_hearth_overlay(&other));
        assert!(!is_hearth_overlay(&d.path().join("missing")));
    }

    #[test]
    fn checks_who_connected() {
        let (a, _b) = UnixStream::pair().unwrap();
        assert_eq!(peer_uid(&a), Some(uid()));
    }

    #[test]
    fn talks_lines_of_json() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("hearth").join("overlay.sock");
        let got: Arc<Mutex<Vec<Value>>> = Arc::default();
        let connects = Arc::new(AtomicUsize::new(0));
        let (g, c) = (got.clone(), connects.clone());
        listen(&path, Arc::new(move |v: Value| g.lock().unwrap().push(v)), Arc::new(move || { c.fetch_add(1, Ordering::SeqCst); })).unwrap();
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(std::fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
        assert_eq!(std::fs::metadata(path.parent().unwrap()).unwrap().permissions().mode() & 0o777, 0o700);
        let mut s = UnixStream::connect(&path).unwrap();
        s.write_all(b"{\"type\":\"hhd\",\"action\":\"open\"}\n\nnot json\n{\"type\":\"call\",\"id\":1}\n").unwrap();
        for _ in 0..100 { if got.lock().unwrap().len() >= 2 && connected() { break; } std::thread::sleep(Duration::from_millis(20)); }
        assert_eq!(got.lock().unwrap().len(), 2);
        assert_eq!(got.lock().unwrap()[0]["type"], "hhd");
        assert_eq!(connects.load(Ordering::SeqCst), 1);
        send(&serde_json::json!({"type": "event", "name": "overlay.shown"}));
        let mut line = String::new();
        s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        BufReader::new(&s).read_line(&mut line).unwrap();
        assert_eq!(serde_json::from_str::<Value>(&line).unwrap()["name"], "overlay.shown");
    }
}
