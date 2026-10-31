//! Game Mode's Mods screens (ui/index.html, the "Hearth Mods in Game Mode" block): a game's mods, Nexus Mods,
//! downloads, Wabbajack modlists and accounts, without opening the Mods window. Everything is done by Hearth Mods
//! itself (hosts/hearth-gaming): Hearth asks its worker over a Unix socket (hearth-gaming `service.rs`), starting
//! `hearth-mods --serve` when no worker runs yet. The Mods window, when open, is the worker, so both show the same
//! queue and installs. hearth-gaming already depends on hearth-core, so this side only speaks the socket's protocol:
//! a line `{"call": name, "args": {...}}`, answered by a line `{"ok": true, "value": ...}` / `{"ok": false, "error"}`.
//!
//! Only the calls in CALLS go through (no folder browsing or file paths from the page: those stay in the window).
//! Arguments may hold a password (MEGA's sign-in): they're never logged here.

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// What Game Mode may ask Hearth Mods.
pub const CALLS: &[&str] = &[
    "ping", "status", "list", "mods_of", "toggle", "move_mod", "remove_mod", "set_deploy", "set_nexus_game", "check_updates",
    "nexus_account", "nexus_sign_in", "nexus_refresh", "nexus_sign_out", "nexus_pages", "nexus_files", "nexus_browse", "nexus_search",
    "accounts_list", "account_begin", "account_sign_out", "mega_sign_in",
    "downloads_list", "download_nxm", "download_file", "download_pause", "download_resume", "download_remove", "downloads_clear", "install_download",
    "wj_engine", "wj_fetch_engine", "wj_gallery", "wj_jobs", "wj_defaults", "wj_start", "wj_resume", "wj_cancel", "wj_forget", "wj_repair", "wj_guide_skip",
    "open_page", "open_url", "free_space",
    // The Hearth Mod Organizer (hearth-gaming organizer/gamemode.rs).
    "organizer_game", "organizer_set_profile", "organizer_plugin_toggle", "organizer_plugin_move", "organizer_fomod_finish",
];
/// Calls that can take a while (Nexus, the gallery, downloading the engine, signing in to MEGA, setting up a prefix).
const SLOW: &[&str] = &["nexus_files", "nexus_browse", "nexus_search", "nexus_sign_in", "nexus_refresh", "check_updates", "wj_gallery", "wj_fetch_engine", "mega_sign_in", "wj_repair", "download_file", "download_nxm", "install_download", "list", "organizer_game", "organizer_fomod_finish"];

pub fn allowed(name: &str) -> bool { CALLS.contains(&name) }
pub fn timeout_for(name: &str) -> Duration { if SLOW.contains(&name) { Duration::from_secs(300) } else { Duration::from_secs(30) } }

/// Whether Hearth Mods is installed (host.info feature "mods": Game Mode shows its Mods screens).
pub fn available() -> bool { crate::gaming_apps::find("hearth-mods").is_some() }

/// The worker's socket (the same rule as hearth-gaming's service::socket_path).
pub fn socket_path() -> PathBuf {
    match std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).filter(|p| p.is_absolute() && p.is_dir()) {
        Some(d) => d.join("hearth-mods.sock"),
        None => crate::paths::data_dir().join("mods").join("hearth-mods.sock"),
    }
}

const NOT_RUNNING: &str = "Hearth Mods isn't running.";

/// One request to the worker at `path`.
pub fn request_at(path: &Path, name: &str, args: &Value, timeout: Duration) -> Result<Value, String> {
    let mut s = UnixStream::connect(path).map_err(|_| NOT_RUNNING.to_string())?;
    s.set_read_timeout(Some(timeout)).map_err(|e| e.to_string())?;
    let mut line = serde_json::to_string(&json!({"call": name, "args": args})).unwrap_or_default();
    line.push('\n');
    s.write_all(line.as_bytes()).map_err(|e| e.to_string())?;
    let mut back = String::new();
    BufReader::new(s).take(64 << 20).read_line(&mut back).map_err(|_| "Hearth Mods took too long to answer.".to_string())?;
    let v: Value = serde_json::from_str(back.trim()).map_err(|_| "Hearth Mods gave no answer.".to_string())?;
    if v["ok"] == true { Ok(v["value"].clone()) } else { Err(v["error"].as_str().unwrap_or("Hearth Mods couldn't do that.").to_string()) }
}

/// Starts the worker without a window and waits (5 s at most) for it to answer.
fn start_worker(path: &Path) -> Result<(), String> {
    let exe = crate::gaming_apps::find("hearth-mods").ok_or("Hearth Mods isn't installed. It comes with Hearth OS.")?;
    crate::bridge::log("hearth mods: starting its worker (hearth-mods --serve)");
    std::process::Command::new(exe).arg("--serve").current_dir(crate::paths::home())
        .stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null())
        .spawn().map(crate::procs::detach).map_err(|e| format!("Couldn't start Hearth Mods ({e})"))?;
    for _ in 0..50 {
        if UnixStream::connect(path).is_ok() { return Ok(()); }
        std::thread::sleep(Duration::from_millis(100));
    }
    Err("Hearth Mods didn't start. Try again, or open Mods from the game's menu.".into())
}

/// A call from Game Mode. `start`: start the worker when none runs (false for the quiet checks Game Mode makes for
/// its notifications: then Null when no worker runs).
pub fn call_at(path: &Path, name: &str, args: &Value, start: bool, starter: &dyn Fn(&Path) -> Result<(), String>) -> Result<Value, String> {
    if !allowed(name) { return Err(format!("Game Mode can't ask Hearth Mods for \"{name}\".")); }
    let args = if args.is_object() { args.clone() } else { json!({}) };
    match request_at(path, name, &args, timeout_for(name)) {
        Err(e) if e == NOT_RUNNING => {
            if !start { return Ok(Value::Null); }
            starter(path)?;
            request_at(path, name, &args, timeout_for(name))
        }
        r => r,
    }
}
pub fn call(name: &str, args: &Value, start: bool) -> Result<Value, String> { call_at(&socket_path(), name, args, start, &start_worker) }

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::net::UnixListener;

    /// A stand-in worker: answers each request with what it was asked (or an error for "fail").
    fn fake_worker(path: &Path) {
        let l = UnixListener::bind(path).unwrap();
        std::thread::spawn(move || {
            for mut s in l.incoming().flatten() {
                let mut line = String::new();
                BufReader::new(s.try_clone().unwrap()).read_line(&mut line).unwrap();
                let req: Value = serde_json::from_str(line.trim()).unwrap();
                let out = if req["call"] == "wj_cancel" { json!({"ok": false, "error": "Stop the install first."}) } else { json!({"ok": true, "value": {"call": req["call"], "args": req["args"]}}) };
                let _ = s.write_all(format!("{out}\n").as_bytes());
            }
        });
    }

    #[test]
    fn asks_the_worker_and_starts_it_when_needed() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("hm.sock");
        let never = |_: &Path| -> Result<(), String> { panic!("must not start a worker") };
        // Not allowed: never reaches the worker.
        assert!(call_at(&p, "open_folder", &json!({}), true, &never).is_err());
        assert!(call_at(&p, "pick_path", &json!({}), true, &never).is_err());
        // A quiet check with no worker: Null, nothing started.
        assert_eq!(call_at(&p, "status", &json!({}), false, &never).unwrap(), Value::Null);
        // With start: the starter runs (here it brings up the stand-in), then the call goes through.
        let p2 = p.clone();
        let started = std::sync::atomic::AtomicBool::new(false);
        let starter = |path: &Path| -> Result<(), String> { assert_eq!(path, p2.as_path()); fake_worker(path); started.store(true, std::sync::atomic::Ordering::SeqCst); Ok(()) };
        let v = call_at(&p, "toggle", &json!({"id": "steam:489830", "m": "skyui", "on": true}), true, &starter).unwrap();
        assert!(started.load(std::sync::atomic::Ordering::SeqCst));
        assert_eq!(v["call"], "toggle");
        assert_eq!(v["args"]["m"], "skyui");
        // Running now: no second start; bad args become {}; errors come back as they are.
        assert_eq!(call_at(&p, "status", &json!("x"), true, &never).unwrap()["args"], json!({}));
        assert_eq!(call_at(&p, "wj_cancel", &json!({"id": "w1"}), true, &never).unwrap_err(), "Stop the install first.");
        // A starter that fails says why.
        let q = dir.path().join("none.sock");
        assert_eq!(call_at(&q, "status", &json!({}), true, &|_: &Path| -> Result<(), String> { Err("nope".to_string()) }).unwrap_err(), "nope");
    }

    #[test]
    fn the_list_of_calls() {
        for c in ["mods_of", "toggle", "move_mod", "remove_mod", "check_updates", "nexus_search", "nexus_browse", "nexus_files", "download_file", "install_download",
                  "download_pause", "download_resume", "wj_gallery", "wj_start", "wj_resume", "wj_guide_skip", "accounts_list", "account_begin", "mega_sign_in", "status"] {
            assert!(allowed(c), "{c}");
        }
        for c in ["organizer_game", "organizer_set_profile", "organizer_plugin_toggle", "organizer_plugin_move", "organizer_fomod_finish"] { assert!(allowed(c), "{c}"); }
        // The organizer's full command set takes folders and starts programs: the Mods window's only.
        for c in ["open_folder", "browse", "places", "stage", "install", "wj_manual_file", "pick_path", "organizer", "organizer_run", "", "../x"] { assert!(!allowed(c), "{c}"); }
        assert!(timeout_for("nexus_search") > timeout_for("toggle"));
        let s = socket_path();
        assert!(s.ends_with("hearth-mods.sock"));
    }
}
