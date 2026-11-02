//! The one safe way Hearth reads and writes a settings file (docs/SETTINGS-PERSISTENCE.md). The same rules, file
//! names and lock as desktop/shell/hearth_settings_io.py, so Rust, Python, QML and shell writers can share a file:
//!
//! * atomic writes: a temporary file in the same folder, fsync, rename, fsync of the folder;
//! * FILE.bak keeps the last good version (copied before each write that changes the file);
//! * a file that can't be read is put back from FILE.bak; with no good .bak it's kept as FILE.broken-<time>.
//!   Defaults are never saved over it, and the person is told once;
//! * merge on write: `update_json` reads the latest file under an exclusive lock (`.FILE.lock` beside it, flock),
//!   changes only what the caller changes, then writes. Two programs changing different keys both keep theirs.

use serde_json::{Map, Value};
use std::io::Write;
use std::path::{Path, PathBuf};

fn sibling(path: &Path, prefix: &str, suffix: &str) -> PathBuf {
    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    path.with_file_name(format!("{prefix}{name}{suffix}"))
}
pub fn bak_path(path: &Path) -> PathBuf { sibling(path, "", ".bak") }

/// An exclusive lock on `.NAME.lock` beside the file, held until dropped (shared with hearth_settings_io.py).
pub struct Lock { _f: std::fs::File }
pub fn lock(path: &Path) -> std::io::Result<Lock> {
    use std::os::unix::io::AsRawFd;
    if let Some(dir) = path.parent() { if !dir.as_os_str().is_empty() { std::fs::create_dir_all(dir)?; } }
    let f = std::fs::OpenOptions::new().read(true).write(true).create(true).truncate(false).open(sibling(path, ".", ".lock"))?;
    // SAFETY: flock on a file descriptor we own; released when the file is closed.
    let r = unsafe { libc::flock(f.as_raw_fd(), libc::LOCK_EX) };
    if r != 0 { return Err(std::io::Error::last_os_error()); }
    Ok(Lock { _f: f })
}

fn fsync_dir(dir: &Path) { if let Ok(d) = std::fs::File::open(dir) { let _ = d.sync_all(); } }

/// Writes `bytes` all at once and makes sure they're on the drive before the old file goes.
fn atomic(path: &Path, bytes: &[u8], mode: Option<u32>) -> std::io::Result<()> { atomic_opt(path, bytes, mode, true) }
fn atomic_opt(path: &Path, bytes: &[u8], mode: Option<u32>, sync: bool) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let dir = path.parent().filter(|d| !d.as_os_str().is_empty()).map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("."));
    std::fs::create_dir_all(&dir)?;
    let tmp = sibling(path, ".", &format!(".{}-{}.tmp", std::process::id(), SEQ.fetch_add(1, Ordering::SeqCst)));
    let r = (|| {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        if sync { f.sync_all()?; }
        let m = mode.or_else(|| std::fs::metadata(path).ok().map(|m| m.permissions().mode() & 0o7777)).unwrap_or(0o644);
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(m))?;
        std::fs::rename(&tmp, path)
    })();
    if r.is_err() { let _ = std::fs::remove_file(&tmp); }
    if sync { fsync_dir(&dir); }
    r
}

pub fn valid_json_object(b: &[u8]) -> bool { serde_json::from_slice::<Value>(b).map(|v| v.is_object()).unwrap_or(false) }
fn valid_text(b: &[u8]) -> bool { std::str::from_utf8(b).is_ok() && !b.iter().all(u8::is_ascii_whitespace) }

/// Atomic write with FILE.bak: the current file is kept as .bak first when `valid` says it's good. Unchanged: no write.
pub fn write_bytes(path: &Path, bytes: &[u8], valid: fn(&[u8]) -> bool) -> std::io::Result<bool> { write_bytes_opt(path, bytes, valid, true) }
/// `sync` false: no fsync (still atomic, with .bak). Only for library.json, written with the bridge held, where an
/// fsync could wait seconds behind a big download (ext4 and btrfs put a file's data on the drive before a rename
/// that replaces it, and the .bak covers the rest).
pub fn write_bytes_opt(path: &Path, bytes: &[u8], valid: fn(&[u8]) -> bool, sync: bool) -> std::io::Result<bool> {
    let cur = std::fs::read(path).ok();
    if cur.as_deref() == Some(bytes) { return Ok(false); }
    if let Some(c) = cur.as_deref().filter(|c| valid(c)) { atomic_opt(&bak_path(path), c, Some(0o600), sync)?; }
    atomic_opt(path, bytes, None, sync)?;
    Ok(true)
}
pub fn write_text(path: &Path, text: &str) -> std::io::Result<bool> { write_bytes(path, text.as_bytes(), valid_text) }
pub fn write_json(path: &Path, v: &Value) -> std::io::Result<bool> { write_json_opt(path, v, true) }
fn write_json_opt(path: &Path, v: &Value, sync: bool) -> std::io::Result<bool> {
    write_bytes_opt(path, format!("{}\n", serde_json::to_string_pretty(v).unwrap_or_default()).as_bytes(), valid_json_object, sync)
}

#[cfg(test)]
fn state_dir() -> PathBuf { std::env::temp_dir().join("hearth-settings-test-state") }
#[cfg(not(test))]
fn state_dir() -> PathBuf {
    std::env::var_os("XDG_STATE_HOME").map(PathBuf::from).unwrap_or_else(|| crate::paths::home().join(".local/state")).join("nimbus")
}
fn stamp() -> String { chrono::Local::now().format("%Y%m%d-%H%M%S").to_string() }

/// A line in ~/.local/state/nimbus/settings-events.log (Send logs includes it) and the app's own log.
pub fn log_event(text: &str) {
    eprintln!("hearth-settings: {text}");
    let d = state_dir();
    let _ = std::fs::create_dir_all(&d);
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(d.join("settings-events.log")) {
        let _ = writeln!(f, "{} {text}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"));
    }
}

/// Tells the person once per id: the events log, a desktop notification where one can be shown.
pub fn notice(id: &str, text: &str) -> bool {
    let d = state_dir().join("settings-notified");
    let mark = d.join(&sha1_smol::Sha1::from(id.as_bytes()).digest().to_string()[..16]);
    if mark.exists() { return false; }
    log_event(&format!("notice: {text}"));
    let _ = std::fs::create_dir_all(&d);
    let _ = std::fs::write(&mark, format!("{text}\n"));
    if !cfg!(test) && (std::env::var_os("DBUS_SESSION_BUS_ADDRESS").is_some() || std::env::var_os("XDG_RUNTIME_DIR").is_some()) {
        let text = text.to_string();
        std::thread::spawn(move || { let _ = std::process::Command::new("notify-send").args(["-a", "Hearth", "Hearth settings", &text])
            .stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).status(); });
    }
    true
}

fn name_of(path: &Path) -> String { path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default() }

/// With the lock held: `raw` (PATH's damaged contents) put right. Some(object) from .bak (written back, the damaged
/// copy kept), or None after keeping the damaged file as PATH.broken-<time>. Never writes defaults.
fn repair_locked(path: &Path, raw: &[u8]) -> Option<Map<String, Value>> {
    let raw = std::fs::read(path).unwrap_or_else(|_| raw.to_vec());
    if let Ok(Value::Object(m)) = serde_json::from_slice::<Value>(&raw) { return Some(m); }
    let hash = sha1_smol::Sha1::from(&raw).digest().to_string();
    let keep = |raw: &[u8]| { let p = sibling(path, "", &format!(".broken-{}", stamp())); let _ = atomic(&p, raw, Some(0o600)); p };
    if let Ok(bak) = std::fs::read(bak_path(path)) {
        if let Ok(Value::Object(m)) = serde_json::from_slice::<Value>(&bak) {
            let kept = if raw.iter().all(u8::is_ascii_whitespace) { None } else { Some(keep(&raw)) };
            let _ = atomic(path, &bak, None);
            notice(&format!("restored:{}:{hash}", path.display()),
                   &format!("Your {} settings were damaged (probably a power cut) and have been put back from the last good copy.", name_of(path)));
            log_event(&format!("restored {} from .bak{}", path.display(), kept.map(|k| format!(" (damaged copy kept as {})", k.display())).unwrap_or_default()));
            return Some(m);
        }
    }
    let kept = keep(&raw);
    notice(&format!("broken:{}:{hash}", path.display()),
           &format!("Your {} settings couldn't be read and there was no good copy to put back. The file is kept as {}; Hearth uses its defaults until you change something.",
                    name_of(path), name_of(&kept)));
    None
}

/// What reading a settings file found.
#[derive(Debug, PartialEq)]
pub enum Read { Missing, Ok(Map<String, Value>), Restored(Map<String, Value>), Unreadable }
impl Read {
    pub fn map(self) -> Option<Map<String, Value>> { match self { Read::Ok(m) | Read::Restored(m) => Some(m), _ => None } }
}

/// The JSON object in PATH, restoring a damaged file from PATH.bak. Never writes defaults into it.
pub fn read(path: &Path) -> Read {
    let raw = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Read::Missing,
        Err(e) => { log_event(&format!("couldn't read {}: {e} (nothing changed)", path.display())); return Read::Unreadable; }
    };
    if let Ok(Value::Object(m)) = serde_json::from_slice::<Value>(&raw) { return Read::Ok(m); }
    let Ok(_l) = lock(path) else { return Read::Unreadable };
    match repair_locked(path, &raw) { Some(m) => Read::Restored(m), None => Read::Unreadable }
}
/// `read`, as a Value: {} when missing or unreadable (for showing; saving goes through `update_json`).
pub fn read_json(path: &Path) -> Value { Value::Object(read(path).map().unwrap_or_default()) }

/// Reads the latest PATH under the lock, lets `change` change it, writes it when it changed. A damaged file is put
/// back from .bak first; with no good .bak the damaged one is kept aside and `change` starts from {}.
pub fn update_json<T>(path: &Path, change: impl FnOnce(&mut Map<String, Value>) -> T) -> std::io::Result<(Map<String, Value>, T)> {
    update_json_opt(path, true, change)
}
pub fn update_json_opt<T>(path: &Path, sync: bool, change: impl FnOnce(&mut Map<String, Value>) -> T) -> std::io::Result<(Map<String, Value>, T)> {
    let _l = lock(path)?;
    let raw = std::fs::read(path).ok();
    let mut m = match &raw {
        None => Map::new(),
        Some(b) => match serde_json::from_slice::<Value>(b) { Ok(Value::Object(m)) => m, _ => repair_locked(path, b).unwrap_or_default() },
    };
    let before = Value::Object(m.clone());
    let out = change(&mut m);
    let after = Value::Object(m);
    if raw.is_none() || after != before { write_json_opt(path, &after, sync)?; }
    let Value::Object(m) = after else { unreachable!() };
    Ok((m, out))
}
/// Sets these top-level keys (Null removes one); every other key stays as it is on disk.
pub fn merge_json(path: &Path, changes: &Map<String, Value>) -> std::io::Result<Map<String, Value>> {
    update_json(path, |m| for (k, v) in changes { if v.is_null() { m.remove(k); } else { m.insert(k.clone(), v.clone()); } }).map(|(m, _)| m)
}

/// A text settings file (session.conf) changed under the same lock, with .bak; unchanged text isn't written.
pub fn update_text(path: &Path, change: impl FnOnce(&str) -> String) -> std::io::Result<String> {
    let _l = lock(path)?;
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let new = change(&text);
    if new != text { write_text(path, &new)?; }
    Ok(new)
}

/// DATA with DEFAULTS filled in where keys are missing only (nested objects too); present values stay.
pub fn with_defaults(data: &mut Map<String, Value>, defaults: &Map<String, Value>) {
    for (k, d) in defaults {
        match data.get_mut(k) {
            None => { data.insert(k.clone(), d.clone()); }
            Some(Value::Object(have)) => if let Value::Object(dd) = d { with_defaults(have, dd); },
            Some(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn obj(v: Value) -> Map<String, Value> { v.as_object().unwrap().clone() }

    #[test]
    fn writes_keep_the_last_good_copy_and_no_temporary_files() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("desktop.json");
        write_json(&f, &json!({"look": "hearth"})).unwrap();
        write_json(&f, &json!({"look": "steam"})).unwrap();
        assert_eq!(read_json(&f), json!({"look": "steam"}));
        assert_eq!(serde_json::from_slice::<Value>(&std::fs::read(bak_path(&f)).unwrap()).unwrap(), json!({"look": "hearth"}));
        let names: Vec<String> = std::fs::read_dir(d.path()).unwrap().flatten().map(|e| e.file_name().to_string_lossy().to_string()).filter(|n| n.ends_with(".tmp")).collect();
        assert!(names.is_empty(), "{names:?}");
    }

    #[test]
    fn a_cut_short_file_comes_back_from_bak_and_defaults_are_never_saved_over_it() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("library.json");
        write_json(&f, &json!({"favorites": ["steam:1"], "setupDone": true})).unwrap();
        write_json(&f, &json!({"favorites": ["steam:1", "gog:2"], "setupDone": true})).unwrap();
        std::fs::write(&f, b"{\"favorites\": [\"ste").unwrap();      // power cut mid-write (an old, non-atomic writer)
        let r = read(&f);
        assert!(matches!(r, Read::Restored(_)));
        assert_eq!(r.map().unwrap()["favorites"], json!(["steam:1"]));
        assert_eq!(read_json(&f)["setupDone"], json!(true));          // written back
        // An empty file (truncated) is the same.
        std::fs::write(&f, b"").unwrap();
        let (m, _) = update_json(&f, |m| { m.insert("hidden".into(), json!(["x"])); }).unwrap();
        assert_eq!(m["setupDone"], json!(true));
        assert_eq!(m["hidden"], json!(["x"]));
    }

    #[test]
    fn with_no_good_copy_the_damaged_file_is_kept() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("osk.json");
        std::fs::write(&f, b"{broken").unwrap();
        assert_eq!(read(&f), Read::Unreadable);
        let kept: Vec<_> = std::fs::read_dir(d.path()).unwrap().flatten().filter(|e| e.file_name().to_string_lossy().starts_with("osk.json.broken-")).collect();
        assert_eq!(kept.len(), 1);
        assert_eq!(std::fs::read(kept[0].path()).unwrap(), b"{broken");
        assert_eq!(std::fs::read(&f).unwrap(), b"{broken");           // nothing written over it by reading
    }

    #[test]
    fn two_writers_changing_different_keys_both_keep_theirs() {
        let d = tempfile::tempdir().unwrap();
        let f = std::sync::Arc::new(d.path().join("desktop.json"));
        write_json(&f, &json!({"look": "steam"})).unwrap();
        let hs: Vec<_> = (0..8).map(|i| { let f = f.clone(); std::thread::spawn(move || {
            for j in 0..20 { merge_json(&f, &obj(json!({format!("k{i}"): j}))).unwrap(); }
        }) }).collect();
        for h in hs { h.join().unwrap(); }
        let v = read_json(&f);
        for i in 0..8 { assert_eq!(v[format!("k{i}")], json!(19)); }
        assert_eq!(v["look"], json!("steam"));
    }

    #[test]
    fn defaults_fill_only_missing_keys() {
        let mut m = obj(json!({"steamBackground": false, "sources": {"xbox": false}}));
        with_defaults(&mut m, &obj(json!({"steamBackground": true, "autoUpdateCheck": true, "sources": {"xbox": true, "steam": true}})));
        assert_eq!(Value::Object(m), json!({"steamBackground": false, "autoUpdateCheck": true, "sources": {"xbox": false, "steam": true}}));
    }

    #[test]
    fn text_edits_keep_the_other_lines() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("session.conf");
        std::fs::write(&f, "# mine\nHEARTH_STEAM_DEFAULT=1\nNIMBUS_START_MODE=game\n").unwrap();
        update_text(&f, |t| crate::admin::write_start_mode(t, "desktop")).unwrap();
        let t = std::fs::read_to_string(&f).unwrap();
        assert!(t.contains("# mine\nHEARTH_STEAM_DEFAULT=1\nNIMBUS_START_MODE=desktop"), "{t}");
        assert!(std::fs::read_to_string(bak_path(&f)).unwrap().contains("NIMBUS_START_MODE=game"));
    }
}
