//! Couch Co-op (several players on one screen, each with their own copy of a game: hearth-coop) and
//! Multi-seat (a separate Game Mode per screen: hearth-seats). Both helpers are part of Hearth OS; they print
//! "progress: N text" lines and one JSON answer ("error: ..." on failure). See docs/COUCH-COOP.md and
//! docs/MULTI-SEAT.md.

use serde_json::{json, Value};
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};

pub const COOP: &str = "/usr/libexec/nimbus/hearth-coop";
pub const SEATS: &str = "/usr/libexec/nimbus/hearth-seats";

pub fn coop_available() -> bool { Path::new(COOP).exists() && std::env::var_os("HEARTH_SEAT").is_none() }
/// Multi-seat, a window of the split-screen host (player 1's, or another player's on the same card): the host
/// gives only one window input at a time, and the web view only reads the controller in that one, so Hearth reads
/// this player's controllers itself (hearth-seats sets HEARTH_SEAT_HOSTED).
pub fn seat_hosted() -> bool { std::env::var_os("HEARTH_SEAT_HOSTED").is_some() }
pub fn seats_available() -> bool { Path::new(SEATS).exists() && std::env::var_os("HEARTH_SEAT").is_none() }

/// "progress: 40 Starting player 2" → (40, "Starting player 2")
pub fn parse_progress(line: &str) -> Option<(u64, String)> {
    let rest = line.trim().strip_prefix("progress: ")?;
    let (n, text) = rest.split_once(' ').unwrap_or((rest, ""));
    Some((n.parse().ok()?, text.to_string()))
}

/// Runs a helper; each progress line goes to `on_progress`. Its JSON answer, or its error in plain words.
pub fn run(helper: &str, args: &[&str], on_progress: &mut dyn FnMut(u64, &str)) -> Result<Value, String> {
    if !Path::new(helper).exists() { return Err("This needs Hearth OS.".into()); }
    let mut child = Command::new("python3").arg(helper).args(args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped())
        .spawn().map_err(|e| e.to_string())?;
    // Its error output is read at the same time: read only at the end, a helper that wrote more of it than a pipe
    // holds (64 KiB: a long traceback, chatty tools it runs) waited for us forever, and we for it, with the page's
    // requests held up behind.
    let err_reader = child.stderr.take().map(|mut e| std::thread::spawn(move || { let mut b = Vec::new(); let _ = std::io::Read::read_to_end(&mut e, &mut b); b }));
    let mut out = String::new();
    if let Some(stdout) = child.stdout.take() {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if let Some((n, t)) = parse_progress(&line) { on_progress(n, &t); }
            out.push_str(&line); out.push('\n');
        }
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    let stderr = err_reader.and_then(|t| t.join().ok()).unwrap_or_default();
    let parsed = crate::admin::parse_helper(&out)?;
    if !status.success() && parsed.is_null() {
        return Err(String::from_utf8_lossy(&stderr).lines().last().unwrap_or("It didn't work.").to_string());
    }
    Ok(parsed)
}

/// A quick answer (no progress needed).
pub fn ask(helper: &str, args: &[&str]) -> Result<Value, String> { run(helper, args, &mut |_, _| {}) }

/// Long jobs (starting a co-op game, setting up seats, waiting for a button) run on their own thread and
/// report as events: {name}.progress {pct, text}, then {name}.done {result} or {name}.error {text}.
pub fn spawn_job(name: &'static str, helper: &'static str, args: Vec<String>, emit: crate::system::bt_scan::Emit,
                 then: Option<Box<dyn FnOnce(&Value) + Send>>) {
    std::thread::spawn(move || {
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let e2 = emit.clone();
        let r = run(helper, &refs, &mut |pct, text| e2(json!({"type": "event", "name": format!("{name}.progress"), "data": {"pct": pct, "text": text}})));
        match r {
            Ok(v) => {
                crate::bridge::log(&format!("{name}: done"));
                if let Some(f) = then { f(&v); }
                emit(json!({"type": "event", "name": format!("{name}.done"), "data": v}));
            }
            Err(e) => {
                crate::bridge::log(&format!("{name}: {e}"));
                emit(json!({"type": "event", "name": format!("{name}.error"), "data": {"text": e}}));
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reads_progress_lines() {
        assert_eq!(parse_progress("progress: 40 Starting player 2"), Some((40, "Starting player 2".into())));
        assert_eq!(parse_progress("progress: 100"), Some((100, String::new())));
        assert_eq!(parse_progress("{\"ok\":true}"), None);
        // A helper with a lot to say on its error output still answers.
        let d = tempfile::tempdir().unwrap();
        let h = d.path().join("helper");
        std::fs::write(&h, "import sys\nsys.stderr.write('x' * 300000)\nprint('progress: 50 half')\nprint('{\"ok\": true}')\n").unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        let hs = h.to_string_lossy().to_string();
        std::thread::spawn(move || { let mut seen = 0; let r = run(&hs, &[], &mut |n, _| seen = n); let _ = tx.send((r, seen)); });
        let (r, seen) = rx.recv_timeout(std::time::Duration::from_secs(20)).expect("the helper and Hearth waited on each other");
        assert_eq!(r.unwrap()["ok"], true);
        assert_eq!(seen, 50);
    }
}
