//! Store sign-ins asked for in Game Mode happen in Desktop Mode. gamescope shows one window at a time and has no
//! real keyboard focus, so a store's sign-in page (or Steam's own window) misbehaves there; on the desktop, windows,
//! the mouse and the keyboard work normally.
//!
//! 1. Game Mode writes a request ($XDG_STATE_HOME/hearth/desktop-signin.json: {"store", "at"}) and switches.
//! 2. Desktop Mode's autostart runs desktop/shell/hearth-signin, which takes the request (if it's under 10 minutes
//!    old) and opens Hearth in a window with `--signin <store>`, or hands it to a windowed Hearth that's already
//!    open (through route_file(), read here every couple of seconds).
//! 3. The windowed Hearth starts that sign-in and offers the way back (`session.gameMode` → nimbus-mode game).

use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// A request older than this is left over from before (Desktop Mode didn't start, or was opened much later).
pub const MAX_AGE_SECS: u64 = 600;
/// A request handed to a windowed Hearth that's already open: it looks every couple of seconds.
pub const ROUTE_MAX_AGE_SECS: u64 = 60;

/// The stores that can be signed in to this way: Hearth's own sign-ins, and Steam (its own window).
pub fn valid_store(s: &str) -> bool { s == "steam" || crate::stores::STORES.contains(&s) }

/// $XDG_STATE_HOME/hearth (~/.local/state/hearth).
pub fn state_dir() -> PathBuf {
    std::env::var_os("XDG_STATE_HOME").filter(|v| !v.is_empty()).map(PathBuf::from)
        .unwrap_or_else(|| crate::paths::home().join(".local/state")).join("hearth")
}
pub fn request_file() -> PathBuf { state_dir().join("desktop-signin.json") }
/// A request for a windowed Hearth that's already open in Desktop Mode.
pub fn route_file() -> PathBuf {
    PathBuf::from(std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into())).join("hearth-signin-route.json")
}

pub fn now_secs() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

pub fn request_json(store: &str, at: u64) -> Value { json!({"store": store, "at": at}) }

/// The store a request asks for, if it's a store Hearth knows and the request is fresh (`at` in seconds; in
/// milliseconds also understood). A clock a minute ahead is allowed for.
pub fn parse(text: &str, now: u64, max_age: u64) -> Option<String> {
    let v: Value = serde_json::from_str(text).ok()?;
    let store = v["store"].as_str()?;
    if !valid_store(store) { return None; }
    let mut at = v["at"].as_f64()?;
    if at > 1e11 { at /= 1000.0; }
    let age = now as f64 - at;
    if !(-60.0..(max_age as f64)).contains(&age) { return None; }
    Some(store.to_string())
}

/// Game Mode: asks Desktop Mode to open this store's sign-in when it starts.
pub fn write_request(store: &str) -> Result<(), String> {
    if !valid_store(store) { return Err("Unknown store".into()); }
    crate::paths::write_atomic(&request_file(), request_json(store, now_secs()).to_string().as_bytes())
        .map_err(|e| format!("Couldn't save the sign-in request: {e}"))
}

/// Reads a request and removes it (taken once, even with two readers: it's moved aside first).
pub fn take_file(path: &Path, now: u64, max_age: u64) -> Option<String> {
    let taken = path.with_extension(format!("taken-{}", std::process::id()));
    std::fs::rename(path, &taken).ok()?;
    let text = std::fs::read_to_string(&taken).unwrap_or_default();
    let _ = std::fs::remove_file(&taken);
    parse(&text, now, max_age)
}
/// A windowed Hearth that's open: a sign-in handed to it by Desktop Mode, if any.
pub fn take_route() -> Option<String> { take_file(&route_file(), now_secs(), ROUTE_MAX_AGE_SECS) }

/// `--signin <store>` (or `--signin=<store>`), only for a store Hearth knows.
pub fn store_arg<I: IntoIterator<Item = String>>(args: I) -> Option<String> {
    let args: Vec<String> = args.into_iter().collect();
    for (i, a) in args.iter().enumerate() {
        let v = if a == "--signin" { args.get(i + 1).cloned() } else { a.strip_prefix("--signin=").map(str::to_string) };
        if let Some(v) = v { return Some(v).filter(|s| valid_store(s)); }
    }
    None
}
pub fn signin_arg() -> Option<String> { store_arg(std::env::args()) }

pub fn windowed() -> bool { std::env::args().any(|a| a == "--windowed") }
/// Game Mode: Hearth full screen in the Hearth session (gamescope). A windowed Hearth in Desktop Mode is also in
/// the session, but isn't Game Mode.
pub fn game_mode() -> bool { crate::gamescope::in_session() && !windowed() }

/// From Desktop Mode back to Game Mode (the same as Mantel's Game Mode button).
pub fn back_to_game_mode() -> Result<(), String> {
    if !crate::gamescope::in_session() { return Err("Game Mode is part of the Hearth session, which isn't running".into()); }
    let mut last = String::new();
    for cmd in ["nimbus-mode", "/usr/bin/nimbus-mode", "/usr/local/bin/nimbus-mode"] {
        match std::process::Command::new(cmd).arg("game")
            .stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn() {
            Ok(mut child) => { std::thread::spawn(move || { let _ = child.wait(); }); return Ok(()); }
            Err(e) => last = e.to_string(),
        }
    }
    Err(format!("Couldn't switch to Game Mode: {last}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_that_can_be_handed_over() {
        for s in crate::stores::STORES { assert!(valid_store(s), "{s}"); }
        assert!(valid_store("steam"));
        for s in ["", "amazon", "Steam", "epic ", "../epic", "steam;rm"] { assert!(!valid_store(s), "{s:?}"); }
    }

    #[test]
    fn only_fresh_requests_for_known_stores_count() {
        let now = 1_800_000_000;
        let req = |s: &str, at: u64| request_json(s, at).to_string();
        assert_eq!(parse(&req("epic", now), now, MAX_AGE_SECS).as_deref(), Some("epic"));
        assert_eq!(parse(&req("steam", now - 599), now, MAX_AGE_SECS).as_deref(), Some("steam"));
        assert_eq!(parse(&req("gog", now - 600), now, MAX_AGE_SECS), None);           // 10 minutes: too old
        assert_eq!(parse(&req("gog", now - 86_400), now, MAX_AGE_SECS), None);
        assert_eq!(parse(&req("xbox", now + 30), now, MAX_AGE_SECS).as_deref(), Some("xbox"));   // clock a bit ahead
        assert_eq!(parse(&req("xbox", now + 3600), now, MAX_AGE_SECS), None);
        assert_eq!(parse(&format!("{{\"store\":\"epic\",\"at\":{}}}", (now - 5) * 1000), now, MAX_AGE_SECS).as_deref(), Some("epic"));   // milliseconds
        assert_eq!(parse(&req("amazon", now), now, MAX_AGE_SECS), None);
        assert_eq!(parse("{\"store\":\"epic\"}", now, MAX_AGE_SECS), None);
        assert_eq!(parse("{\"store\":\"epic\",\"at\":\"soon\"}", now, MAX_AGE_SECS), None);
        assert_eq!(parse("not json", now, MAX_AGE_SECS), None);
        assert_eq!(parse("", now, MAX_AGE_SECS), None);
        assert_eq!(parse(&req("epic", now - 61), now, ROUTE_MAX_AGE_SECS), None);
    }

    #[test]
    fn a_request_is_taken_once() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("desktop-signin.json");
        let now = now_secs();
        std::fs::write(&f, request_json("gog", now).to_string()).unwrap();
        assert_eq!(take_file(&f, now, MAX_AGE_SECS).as_deref(), Some("gog"));
        assert!(!f.exists());
        assert_eq!(take_file(&f, now, MAX_AGE_SECS), None);
        // A stale one is removed too, so it can't open a sign-in some later day.
        std::fs::write(&f, request_json("gog", now - 3600).to_string()).unwrap();
        assert_eq!(take_file(&f, now, MAX_AGE_SECS), None);
        assert!(!f.exists());
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    }

    #[test]
    fn signin_option() {
        let a = |v: &[&str]| store_arg(v.iter().map(|s| s.to_string()));
        assert_eq!(a(&["nimbus", "--windowed", "--signin", "epic"]).as_deref(), Some("epic"));
        assert_eq!(a(&["nimbus", "--signin=steam", "--windowed"]).as_deref(), Some("steam"));
        assert_eq!(a(&["nimbus", "--windowed", "--signin", "amazon"]), None);
        assert_eq!(a(&["nimbus", "--windowed", "--signin"]), None);
        assert_eq!(a(&["nimbus", "--windowed"]), None);
    }
}
