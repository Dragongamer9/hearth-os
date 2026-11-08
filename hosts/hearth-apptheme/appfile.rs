//! Safe settings and data files for Hearth's own apps (docs/SETTINGS-PERSISTENCE.md): the same rules and file names
//! as hearth-core's settings_file.rs and desktop/shell/hearth_settings_io.py, with nothing but std and serde_json, so
//! every app can include this one file (no crate, so no Cargo.lock changes):
//!     #[path = "../../hearth-apptheme/appfile.rs"] mod appfile;
//!
//! * atomic writes: a temporary file with its own name in the same folder, fsync, rename, fsync of the folder;
//! * FILE.bak keeps the last good version (copied before each write that changes the file);
//! * a file that can't be read (empty or cut short by a power cut, not JSON) is put back from FILE.bak; with no good
//!   .bak it's kept as FILE.broken-<time> before anything is written. Defaults are never saved over it, and the
//!   person is told (~/.local/state/nimbus/settings-events.log, which Send logs includes, and a notification);
//! * `update` reads the latest file under an exclusive lock (`.FILE.lock` beside it, the same flock the other
//!   helpers take), lets the caller change it, then writes, so two windows or the app and its service don't undo
//!   each other's changes.
#![allow(dead_code)]

use serde_json::Value;
use std::io::Write;
use std::path::{Path, PathBuf};

fn sibling(path: &Path, prefix: &str, suffix: &str) -> PathBuf {
    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    path.with_file_name(format!("{prefix}{name}{suffix}"))
}
pub fn bak_path(path: &Path) -> PathBuf { sibling(path, "", ".bak") }

/// An exclusive lock on `.NAME.lock` beside the file, held until dropped (shared with hearth_settings_io.py and
/// settings_file.rs: std's File::lock is flock on Linux).
pub struct Lock { _f: Option<std::fs::File> }
pub fn lock(path: &Path) -> Lock {
    if let Some(dir) = path.parent() { if !dir.as_os_str().is_empty() { let _ = std::fs::create_dir_all(dir); } }
    let f = std::fs::OpenOptions::new().read(true).write(true).create(true).truncate(false).open(sibling(path, ".", ".lock")).ok();
    if let Some(f) = &f { let _ = f.lock(); }
    Lock { _f: f }
}

#[cfg(unix)]
fn fsync_dir(dir: &Path) { if let Ok(d) = std::fs::File::open(dir) { let _ = d.sync_all(); } }
#[cfg(not(unix))]
fn fsync_dir(_dir: &Path) {}

/// Writes `bytes` all at once and makes sure they're on the drive before the old file goes.
fn atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let dir = path.parent().filter(|d| !d.as_os_str().is_empty()).map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("."));
    std::fs::create_dir_all(&dir)?;
    let tmp = sibling(path, ".", &format!(".{}-{}.tmp", std::process::id(), SEQ.fetch_add(1, Ordering::SeqCst)));
    let r = (|| {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        drop(f);
        if let Ok(m) = std::fs::metadata(path) { let _ = std::fs::set_permissions(&tmp, m.permissions()); }
        std::fs::rename(&tmp, path)
    })();
    if r.is_err() { let _ = std::fs::remove_file(&tmp); }
    fsync_dir(&dir);
    r
}

/// Good JSON (any kind: an object, a list...). Empty, cut short or zeros from a power cut is not.
fn parse(b: &[u8]) -> Option<Value> {
    if b.iter().all(u8::is_ascii_whitespace) { return None; }
    serde_json::from_slice::<Value>(b).ok()
}

fn state_dir() -> PathBuf {
    if cfg!(test) { return std::env::temp_dir().join(format!("hearth-appfile-test-state-{}", std::process::id())); }
    std::env::var_os("XDG_STATE_HOME").map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/state")))
        .or_else(|| std::env::var_os("LOCALAPPDATA").map(PathBuf::from))
        .unwrap_or_else(std::env::temp_dir).join("nimbus")
}
fn stamp() -> u64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) }
fn name_of(path: &Path) -> String { path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default() }

/// A line in ~/.local/state/nimbus/settings-events.log (Send logs includes it) and the app's own log.
pub fn log_event(text: &str) {
    eprintln!("hearth-settings: {text}");
    let d = state_dir();
    let _ = std::fs::create_dir_all(&d);
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(d.join("settings-events.log")) {
        let _ = writeln!(f, "{} {text}", stamp());
    }
}

/// Tells the person: the events log, and a desktop notification where one can be shown.
fn tell(text: &str) {
    log_event(&format!("notice: {text}"));
    if cfg!(unix) && !cfg!(test) && std::env::var_os("HEARTH_APPFILE_QUIET").is_none()
        && (std::env::var_os("DBUS_SESSION_BUS_ADDRESS").is_some() || std::env::var_os("XDG_RUNTIME_DIR").is_some()) {
        let text = text.to_string();
        std::thread::spawn(move || { let _ = std::process::Command::new("notify-send").args(["-a", "Hearth", "Hearth settings", &text])
            .stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).status(); });
    }
}

/// With the lock held: PATH's damaged contents put right. Some(value) from .bak (written back, the damaged copy
/// kept), or None after keeping the damaged file as PATH.broken-<time>. Never writes defaults.
fn repair_locked(path: &Path) -> Option<Value> {
    let raw = std::fs::read(path).ok()?;
    if let Some(v) = parse(&raw) { return Some(v); }     // another program put it right meanwhile
    let keep = |raw: &[u8]| { let p = sibling(path, "", &format!(".broken-{}", stamp())); let _ = atomic(&p, raw); p };
    if let Some((bak, v)) = std::fs::read(bak_path(path)).ok().and_then(|b| parse(&b).map(|v| (b, v))) {
        let kept = if raw.iter().all(u8::is_ascii_whitespace) { None } else { Some(keep(&raw[..])) };
        let _ = atomic(path, &bak);
        tell(&format!("Your {} was damaged (probably a power cut) and has been put back from the last good copy.", name_of(path)));
        log_event(&format!("restored {} from .bak{}", path.display(), kept.map(|k| format!(" (damaged copy kept as {})", k.display())).unwrap_or_default()));
        return Some(v);
    }
    let kept = keep(&raw[..]);
    let _ = std::fs::remove_file(path);
    tell(&format!("Your {} couldn't be read and there was no good copy to put back. It's kept as {}.", name_of(path), name_of(&kept)));
    None
}

/// What reading a file found.
#[derive(Debug, PartialEq)]
pub enum Got {
    /// Not there yet (nothing saved so far).
    Missing,
    /// Read fine, or put back from FILE.bak.
    Good(Value),
    /// Couldn't be read (no permission, a drive error): don't save over it.
    Unreadable,
}
impl Got {
    pub fn value(self) -> Option<Value> { match self { Got::Good(v) => Some(v), _ => None } }
}

/// The JSON in PATH, putting a damaged file back from PATH.bak. Never writes defaults into it.
pub fn read(path: &Path) -> Got {
    let raw = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Got::Missing,
        Err(e) => { log_event(&format!("couldn't read {}: {e} (nothing changed)", path.display())); return Got::Unreadable; }
    };
    if let Some(v) = parse(&raw) { return Got::Good(v); }
    let _l = lock(path);
    match repair_locked(path) { Some(v) => Got::Good(v), None => Got::Missing }
}

/// `read`, or None when there's nothing usable (missing, or kept aside as .broken).
pub fn read_json(path: &Path) -> Option<Value> { read(path).value() }

fn write_locked(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let cur = std::fs::read(path).ok();
    if cur.as_deref() == Some(bytes) { return Ok(()); }
    if let Some(c) = cur.as_deref().filter(|c| parse(c).is_some()) { atomic(&bak_path(path), c)?; }
    atomic(path, bytes)
}

/// Writes PATH atomically, keeping the old good version as PATH.bak.
pub fn write_bytes(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let _l = lock(path);
    write_locked(path, bytes)
}
pub fn write_json(path: &Path, v: &Value) -> std::io::Result<()> {
    write_bytes(path, format!("{}\n", serde_json::to_string_pretty(v).unwrap_or_default()).as_bytes())
}

/// Reads the latest PATH under the lock (None when there's none yet), lets `change` make the new value, and writes
/// it (`change` gives None to write nothing). A file that can't be read at all (no permission) is left alone.
pub fn update<T>(path: &Path, change: impl FnOnce(Option<Value>) -> (Option<Value>, T)) -> std::io::Result<T> {
    let _l = lock(path);
    let cur = match std::fs::read(path) {
        Ok(b) => match parse(&b) { Some(v) => Some(v), None => repair_locked(path) },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => { log_event(&format!("couldn't read {}: {e} (nothing changed)", path.display())); return Err(e); }
    };
    let (new, out) = change(cur);
    if let Some(new) = new { write_locked(path, format!("{}\n", serde_json::to_string_pretty(&new).unwrap_or_default()).as_bytes())?; }
    Ok(out)
}

/// For a page that keeps its whole settings object and sends all of it on each change: `read_seen` remembers what
/// this program read, and `save_changed` merges only the top-level keys the page changed since then into the latest
/// file. Another window (a second Terminal, say) that changed a different setting meanwhile keeps its change.
pub type Seen = std::sync::Mutex<Option<serde_json::Map<String, Value>>>;

pub fn read_seen(path: &Path, seen: &Seen) -> Option<Value> {
    let v = read_json(path);
    *seen.lock().unwrap_or_else(|e| e.into_inner()) = v.as_ref().and_then(Value::as_object).cloned();
    v
}

pub fn save_changed(path: &Path, seen: &Seen, new: &Value) -> std::io::Result<()> {
    let Some(newm) = new.as_object() else { return write_json(path, new) };
    let mut seen = seen.lock().unwrap_or_else(|e| e.into_inner());
    let base = seen.clone();
    update(path, |cur| {
        let mut m = match (cur.and_then(|v| v.as_object().cloned()), &base) {
            (Some(m), Some(_)) => m,
            _ => serde_json::Map::new(),        // nothing read before (or nothing there): the page's copy is the file
        };
        let base = base.unwrap_or_default();
        for (k, v) in newm { if base.get(k) != Some(v) || !m.contains_key(k) { m.insert(k.clone(), v.clone()); } }
        for k in base.keys() { if !newm.contains_key(k) { m.remove(k); } }
        (Some(Value::Object(m)), ())
    })?;
    *seen = Some(newm.clone());
    Ok(())
}

#[cfg(test)]
mod appfile_tests {
    use super::*;
    use serde_json::json;

    fn dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("hearth-appfile-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn appfile_atomic_with_bak_and_no_leftovers() {
        let d = dir("bak");
        let f = d.join("state.json");
        assert_eq!(read(&f), Got::Missing);
        write_json(&f, &json!({"a": 1})).unwrap();
        write_json(&f, &json!({"a": 2})).unwrap();
        assert_eq!(read_json(&f), Some(json!({"a": 2})));
        assert_eq!(read_json(&bak_path(&f)), Some(json!({"a": 1})));
        let left: Vec<String> = std::fs::read_dir(&d).unwrap().flatten().map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|n| n.ends_with(".tmp")).collect();
        assert!(left.is_empty(), "{left:?}");
    }

    #[test]
    fn appfile_damaged_file_comes_back_from_bak_never_defaults() {
        let d = dir("damaged");
        let f = d.join("calendar.json");
        write_json(&f, &json!({"events": [1, 2]})).unwrap();
        write_json(&f, &json!({"events": [1, 2, 3]})).unwrap();
        for damage in [&b""[..], b"{\"events\": [1, 2", b"\0\0\0\0"] {
            std::fs::write(&f, damage).unwrap();
            assert_eq!(read_json(&f), Some(json!({"events": [1, 2]})), "{damage:?}");
            assert_eq!(parse(&std::fs::read(&f).unwrap()), Some(json!({"events": [1, 2]})));   // put back on disk
        }
        // A change made while the file is damaged starts from the good copy, not from nothing.
        std::fs::write(&f, b"").unwrap();
        update(&f, |cur| { let mut v = cur.unwrap(); v["events"].as_array_mut().unwrap().push(json!(9)); (Some(v), ()) }).unwrap();
        assert_eq!(read_json(&f), Some(json!({"events": [1, 2, 9]})));
    }

    #[test]
    fn appfile_damaged_without_bak_is_kept_aside() {
        let d = dir("aside");
        let f = d.join("places.json");
        std::fs::write(&f, b"[{\"name\": \"Ho").unwrap();
        assert_eq!(read(&f), Got::Missing);
        let kept: Vec<String> = std::fs::read_dir(&d).unwrap().flatten().map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|n| n.starts_with("places.json.broken-")).collect();
        assert_eq!(kept.len(), 1, "the damaged file is kept");
        assert_eq!(std::fs::read(d.join(&kept[0])).unwrap(), b"[{\"name\": \"Ho");
    }

    #[test]
    fn appfile_two_windows_keep_each_others_settings() {
        let d = dir("seen");
        let f = d.join("settings.json");
        write_json(&f, &json!({"fontSize": 14, "theme": "ember"})).unwrap();
        let (a, b): (Seen, Seen) = Default::default();
        let mut sa = read_seen(&f, &a).unwrap();
        let mut sb = read_seen(&f, &b).unwrap();
        sa["fontSize"] = json!(18);
        save_changed(&f, &a, &sa).unwrap();
        sb["theme"] = json!("light");           // window B still thinks the font is 14
        save_changed(&f, &b, &sb).unwrap();
        assert_eq!(read_json(&f), Some(json!({"fontSize": 18, "theme": "light"})));
        // A key the page dropped is dropped; a window that never read the file writes what it has.
        sb.as_object_mut().unwrap().remove("theme");
        save_changed(&f, &b, &sb).unwrap();
        assert_eq!(read_json(&f), Some(json!({"fontSize": 18})));
        let c: Seen = Default::default();
        save_changed(&f, &c, &json!({"fontSize": 12, "x": 1})).unwrap();
        assert_eq!(read_json(&f), Some(json!({"fontSize": 12, "x": 1})));
    }

    #[test]
    fn appfile_update_keeps_both_writers_changes() {
        let d = dir("update");
        let f = d.join("clock.json");
        write_json(&f, &json!({"alarms": [], "settings": {"snooze": 5}})).unwrap();
        let hs: Vec<_> = (0..8).map(|i| { let f = f.clone(); std::thread::spawn(move || {
            update(&f, |cur| { let mut v = cur.unwrap(); v["alarms"].as_array_mut().unwrap().push(json!(i)); (Some(v), ()) }).unwrap();
        }) }).collect();
        for h in hs { h.join().unwrap(); }
        let v = read_json(&f).unwrap();
        assert_eq!(v["alarms"].as_array().unwrap().len(), 8);
        assert_eq!(v["settings"]["snooze"], 5);
    }
}
