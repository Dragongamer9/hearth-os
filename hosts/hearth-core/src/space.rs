//! Make Hearth bigger (Settings › Import, Beta Experimental): space from the other systems on Hearth's drive
//! (Windows, SteamOS, another Linux) after games were moved off them. The work is done by hearth-space, as root
//! through hearth-admin (pkexec): `space-status`, `space-grow --from DEV|free --gb N [--dry-run]`,
//! `space-clear-journal`. See docs/MOVE-AND-GROW.md.
//! Growing reports as events: space.grow.progress {pct, text}, then space.grow.done {result} or
//! space.grow.error {text}.

use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};

static BUSY: AtomicBool = AtomicBool::new(false);

/// --from: a partition (/dev/nvme0n1p3) or "free" (unpartitioned space). Checked here, again in hearth-admin
/// and again in hearth-space.
pub fn valid_from(from: &str) -> bool {
    if from == "free" { return true; }
    let Some(rest) = from.strip_prefix("/dev/") else { return false };
    !rest.is_empty() && rest.len() <= 64 && !rest.contains("..") && !rest.starts_with('/')
        && rest.chars().all(|c| c.is_ascii_alphanumeric() || c == '/' || c == '_' || c == '-')
}

/// hearth-admin's arguments for growing.
pub fn grow_args(from: &str, gb: u64, dry_run: bool) -> Result<Vec<String>, String> {
    if !valid_from(from) { return Err("Choose which system gives the space.".into()); }
    if !(1..=65536).contains(&gb) { return Err("Choose how many GB to give Hearth.".into()); }
    let mut a = vec!["space-grow".to_string(), "--from".into(), from.to_string(), "--gb".into(), gb.to_string()];
    if dry_run { a.push("--dry-run".into()); }
    Ok(a)
}

/// Hearth's space and what each other system on its drive could give (blocking: ntfsresize reads the drive).
pub fn status() -> Result<Value, String> {
    let mut v = crate::admin::run(&["space-status"], None, &mut |_| {})?;
    if v.is_object() { v["busy"] = json!(BUSY.load(Ordering::SeqCst)); }
    Ok(v)
}

/// What growing would do, without doing it (blocking).
pub fn plan(from: &str, gb: u64) -> Result<Value, String> {
    let args = grow_args(from, gb, true)?;
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    crate::admin::run(&refs, None, &mut |_| {})
}

/// After an interrupted grow was looked at (the other system checked its drive): allows growing again.
pub fn clear_journal() -> Result<Value, String> { crate::admin::run(&["space-clear-journal"], None, &mut |_| {}) }

/// Starts growing in the background. Once started it isn't cancelled: hearth-space finishes or puts things back.
pub fn grow(from: &str, gb: u64, emit: crate::system::bt_scan::Emit) -> Result<Value, String> {
    let args = grow_args(from, gb, false)?;
    if BUSY.swap(true, Ordering::SeqCst) { return Err("Hearth is already being made bigger.".into()); }
    crate::bridge::log(&format!("space.grow: {gb} GB from {from}"));
    std::thread::spawn(move || {
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let e2 = emit.clone();
        let r = crate::admin::run(&refs, None, &mut |line: &str| {
            if let Some((pct, text)) = crate::multiplay::parse_progress(line) {
                e2(json!({"type": "event", "name": "space.grow.progress", "data": {"pct": pct, "text": text}}));
            }
        });
        BUSY.store(false, Ordering::SeqCst);
        match r {
            Ok(v) => {
                crate::bridge::log(&format!("space.grow: done {v}"));
                emit(json!({"type": "event", "name": "space.grow.done", "data": v}));
            }
            Err(e) => {
                crate::bridge::log(&format!("space.grow: {e}"));
                emit(json!({"type": "event", "name": "space.grow.error", "data": {"text": e}}));
            }
        }
    });
    Ok(json!({"started": true}))
}

/// Answers a call from another thread, so a slow look at the drives doesn't hold up the rest of the page.
pub fn answer_later(id: Value, f: impl FnOnce() -> Result<Value, String> + Send + 'static) { answer_later_to(id, None, f) }
/// The same, to one window only ("to": its label; the host sends it there).
pub fn answer_later_to(id: Value, to: Option<String>, f: impl FnOnce() -> Result<Value, String> + Send + 'static) {
    std::thread::spawn(move || {
        let mut r = match f() {
            Ok(v) => json!({"type": "result", "id": id, "ok": true, "value": v}),
            Err(e) => json!({"type": "result", "id": id, "ok": false, "error": e}),
        };
        if let Some(t) = to { r["to"] = json!(t); }
        crate::stores::notify(r);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn checks_what_goes_to_root() {
        assert!(valid_from("free"));
        assert!(valid_from("/dev/nvme0n1p3"));
        assert!(valid_from("/dev/mapper/x-y_z"));
        for bad in ["", "/dev/", "/dev/../etc/passwd", "/etc/passwd", "/dev//x", "/dev/sda1;rm", "/dev/sda 1", "free "] {
            assert!(!valid_from(bad), "{bad}");
        }
        assert_eq!(grow_args("/dev/sda3", 64, false).unwrap(), vec!["space-grow", "--from", "/dev/sda3", "--gb", "64"]);
        let dry = grow_args("free", 8, true).unwrap();
        assert_eq!(dry.last().map(String::as_str), Some("--dry-run"));
        assert!(grow_args("/dev/sda3", 0, false).is_err());
        assert!(grow_args("/dev/sda3", 70000, false).is_err());
    }
}
