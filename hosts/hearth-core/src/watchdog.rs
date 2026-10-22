//! Hearth's fail-safe: notices when the page's requests stop being answered and does something about it.
//!
//! Every request from the page runs with the bridge held (see hearth-rs page_msg); `enter`/`leave` mark which one
//! holds it and since when. A thread (`start`) looks every two seconds:
//! - stuck 15 s: the helper programs Hearth started and is waiting on (Handheld Daemon's helper, Beacon, xprop...)
//!   are stopped, which usually frees it; never games, never a password prompt;
//! - stuck 20 s: the logs go to the developer by themselves (once every 10 minutes at most, across restarts), with
//!   what it was stuck on; the page is told the code;
//! - stuck 60 s: Hearth starts again (in the Hearth session; at most twice in 10 minutes, so a Hearth that gets stuck
//!   right as it starts doesn't go round forever).
//! Settings › System › "Send a report when Hearth gets stuck" turns the report off; the rest always runs.

use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

static BUSY: Mutex<Option<(String, u64)>> = Mutex::new(None);
static AUTO_SEND: AtomicBool = AtomicBool::new(true);
static SEQ: AtomicU64 = AtomicU64::new(0);
/// Set while the request in hand is known to take long (a download it waits for): the fail-safe leaves it be.
static LONG: AtomicBool = AtomicBool::new(false);

/// Programs Hearth starts and waits on that are safe to stop when Hearth is stuck on them.
pub const HELPERS: &[&str] = &["hearth-hhd", "hearth-power", "hearth-controller", "beacon", "xprop", "xdotool", "gamescopectl",
    "nmcli", "bluetoothctl", "busctl", "gdbus", "wpctl", "pactl", "journalctl", "flatpak remote-ls", "flatpak info"];

fn now_ms() -> u64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0) }

/// A request starts being worked on (the bridge is held). Returns its number, for `leave`.
pub fn enter(method: &str) -> u64 {
    let n = SEQ.fetch_add(1, Ordering::SeqCst) + 1;
    if let Ok(mut b) = BUSY.lock() { *b = Some((method.to_string(), now_ms())); }
    n
}
/// That request is done.
pub fn leave(_n: u64) { LONG.store(false, Ordering::SeqCst); if let Ok(mut b) = BUSY.lock() { *b = None; } }
/// The request in hand is going to take a while on purpose (downloading a tool, sending logs): not "stuck".
pub fn long_task() { LONG.store(true, Ordering::SeqCst); }
/// What's being worked on now and for how long (ms).
pub fn busy() -> Option<(String, u64)> { BUSY.lock().ok().and_then(|b| b.clone()).map(|(m, t)| (m, now_ms().saturating_sub(t))) }

/// Whether a report goes out by itself (Settings).
pub fn set_auto_send(on: bool) { AUTO_SEND.store(on, Ordering::SeqCst); }

/// Whether a command line is one of the helpers above (and not something it mustn't touch).
pub fn is_helper(cmdline: &str) -> bool {
    let c = cmdline.trim();
    if c.contains("--command=beacon ") { return true; }
    if c.is_empty() || c.contains("pkexec") || c.contains("polkit") || c.contains("steam") || c.contains("rungame") { return false; }
    let first = c.split_whitespace().next().unwrap_or("");
    let prog = first.rsplit('/').next().unwrap_or(first);
    // Python helpers: the script's name is the second word.
    let script = c.split_whitespace().nth(1).map(|s| s.rsplit('/').next().unwrap_or(s)).unwrap_or("");
    // "gdbus monitor" runs all along (sleep and wake, InputPlumber's presses) and is never what a request waits on:
    // stopping it left lock-after-sleep dead until Hearth restarted.
    if prog == "gdbus" && script == "monitor" { return false; }
    HELPERS.iter().any(|h| {
        if h.contains(' ') { c.contains(h) && prog == h.split(' ').next().unwrap() }
        else { prog == *h || (prog.starts_with("python") && script == *h) }
    })
}

/// Hearth's own child processes that are helpers: (pid, command line).
fn stuck_helpers() -> Vec<(u32, String)> {
    let me = std::process::id().to_string();
    let mut out = vec![];
    for e in std::fs::read_dir("/proc").into_iter().flatten().flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        let Ok(pid) = name.parse::<u32>() else { continue };
        let stat = std::fs::read_to_string(e.path().join("stat")).unwrap_or_default();
        // ppid is the 2nd field after the ")" that ends the name.
        let ppid = stat.rsplit_once(')').map(|(_, r)| r.split_whitespace().nth(1).unwrap_or("").to_string()).unwrap_or_default();
        if ppid != me { continue; }
        // Long Beacon jobs (an installer, setting a game up) run while the page is answered: not what it's stuck on.
        if crate::beacon::job_pids().contains(&pid) { continue; }
        let cmd = std::fs::read(e.path().join("cmdline")).map(|b| String::from_utf8_lossy(&b).replace('\0', " ")).unwrap_or_default();
        if is_helper(&cmd) { out.push((pid, cmd.trim().to_string())); }
    }
    out
}

/// Kept across restarts: when a report last went out, and the restarts lately.
fn state_file() -> std::path::PathBuf { crate::paths::data_dir().join("watchdog.json") }
fn state() -> Value { std::fs::read_to_string(state_file()).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or(json!({})) }
fn save_state(v: &Value) { let _ = std::fs::write(state_file(), v.to_string()); }

/// Whether another automatic report may go out now (one every 10 minutes).
pub fn may_report(st: &Value, now: u64) -> bool { now.saturating_sub(st["lastReport"].as_u64().unwrap_or(0)) > 10 * 60_000 }
/// Whether Hearth may restart itself now (at most twice in 10 minutes).
pub fn may_restart(st: &Value, now: u64) -> bool {
    st["restarts"].as_array().map(|a| a.iter().filter(|t| now.saturating_sub(t.as_u64().unwrap_or(0)) < 10 * 60_000).count()).unwrap_or(0) < 2
}

/// Starts the watching. `emit` tells the page (it doesn't need the bridge); `restart` starts Hearth again.
pub fn start(version: String, edition: &'static str, emit: Arc<dyn Fn(Value) + Send + Sync>, restart: Arc<dyn Fn() + Send + Sync>) {
    std::thread::spawn(move || {
        let (mut killed, mut reported, mut restarted): (u64, u64, bool) = (0, 0, false);
        let mut stall_start: u64 = 0;
        loop {
            std::thread::sleep(Duration::from_secs(2));
            let Some((method, ms)) = busy() else { killed = 0; reported = 0; restarted = false; stall_start = 0; continue };
            if ms < 15_000 || LONG.load(Ordering::SeqCst) { continue; }
            let now = now_ms();
            let began = now - ms;
            if stall_start != began { stall_start = began; killed = 0; reported = 0; restarted = false; }
            if killed == 0 || (ms >= 30_000 && killed == 1) {
                killed += 1;
                let list = stuck_helpers();
                crate::bridge::log(&format!("watchdog: stuck in {method} for {} s; stopping {} helper(s): {}", ms / 1000, list.len(),
                    list.iter().map(|(p, c)| format!("{p} {}", c.chars().take(80).collect::<String>())).collect::<Vec<_>>().join("; ")));
                for (pid, _) in list { unsafe { libc_kill(pid as i32, 9); } }
            }
            if ms >= 20_000 && reported == 0 {
                reported = 1;
                let mut st = state();
                if AUTO_SEND.load(Ordering::SeqCst) && may_report(&st, now) {
                    st["lastReport"] = json!(now); save_state(&st);
                    let (version, emit, method) = (version.clone(), emit.clone(), method.clone());
                    std::thread::spawn(move || {
                        let note = format!("Automatic report: Hearth stopped answering (stuck in {method} for 20 s).");
                        match crate::logs::send_report(&version, edition, &note) {
                            Ok(r) => { crate::bridge::log(&format!("watchdog: report sent ({})", r["code"].as_str().unwrap_or("?")));
                                emit(json!({"type": "event", "name": "watchdog", "data": {"stuck": method, "code": r["code"]}})); }
                            Err(e) => { crate::bridge::log(&format!("watchdog: report not sent: {e}"));
                                emit(json!({"type": "event", "name": "watchdog", "data": {"stuck": method, "error": e}})); }
                        }
                    });
                } else {
                    emit(json!({"type": "event", "name": "watchdog", "data": {"stuck": method}}));
                }
            }
            if ms >= 60_000 && !restarted {
                restarted = true;
                let mut st = state();
                if may_restart(&st, now) {
                    let mut list: Vec<Value> = st["restarts"].as_array().cloned().unwrap_or_default();
                    list.retain(|t| now.saturating_sub(t.as_u64().unwrap_or(0)) < 10 * 60_000);
                    list.push(json!(now));
                    st["restarts"] = json!(list); save_state(&st);
                    crate::bridge::log(&format!("watchdog: still stuck in {method} after 60 s; starting Hearth again"));
                    restart();
                } else {
                    crate::bridge::log(&format!("watchdog: still stuck in {method}; restarted twice lately, so not again"));
                }
            }
        }
    });
}

extern "C" { #[link_name = "kill"] fn libc_kill(pid: i32, sig: i32) -> i32; }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn helpers_only() {
        assert!(is_helper("python3 /usr/share/nimbus/desktop/shell/hearth-hhd status"));
        assert!(is_helper("/usr/bin/beacon status --json"));
        assert!(is_helper("xprop -root GAMESCOPE_FOCUSABLE_APPS"));
        assert!(is_helper("/usr/bin/flatpak info com.example"));
        assert!(!is_helper("/usr/bin/flatpak run com.valvesoftware.Steam"));
        assert!(!is_helper("pkexec /usr/libexec/nimbus/hearth-admin update"));
        assert!(!is_helper("/home/me/Games/game.exe"));
        assert!(!is_helper("steam steam://rungameid/10"));
        assert!(!is_helper(""));
        assert!(is_helper("/usr/bin/flatpak run --command=beacon io.github.dragongamer9.Beacon status --json"));
        assert!(!is_helper("curl -sS https://example.com"));
        assert!(is_helper("gdbus call --system --dest org.freedesktop.login1"));
        assert!(!is_helper("gdbus monitor --system --dest org.freedesktop.login1 --object-path /org/freedesktop/login1"));
    }
    #[test]
    fn limits() {
        let now = 100 * 60_000;
        assert!(may_report(&json!({}), now));
        assert!(!may_report(&json!({"lastReport": now - 60_000}), now));
        assert!(may_restart(&json!({"restarts": [now - 20 * 60_000, now - 60_000]}), now));
        assert!(!may_restart(&json!({"restarts": [now - 120_000, now - 60_000]}), now));
    }
    #[test]
    fn marks_what_is_busy() {
        let n = enter("library.list");
        assert_eq!(busy().map(|b| b.0), Some("library.list".to_string()));
        leave(n);
        assert!(busy().is_none());
    }
}
