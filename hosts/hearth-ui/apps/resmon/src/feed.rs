//! The numbers: Mantel's own sampler, `hearth-resmon serve` (desktop/shell/hearth-resmon, the same helper and the same
//! files as Debian's Resource Monitor), started when the window opens and ended when it closes. It reads /proc and
//! /sys (and nvidia-smi / DRM fdinfo for the graphics card) once a second, only what the tab on show needs; told on
//! its stdin ({"interval": 1, "want": [...]}, {"interval": 0} to stand still).
//!
//! Nothing heavy at rest (the owner's red flag): the sampler is live only while the window is seen. When the window
//! is minimized or hidden the compositor stops asking it to draw (no frame callbacks), so two samples arrive with no
//! frame drawn after them: the sampler is told to stand still (interval 0, its select() sleeps for real) and the app's
//! own timers stop. The first frame drawn after the window shows again starts it. Pause does the same on purpose.

use std::io::{BufRead, BufReader, Write};
use std::process::{ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use serde_json::Value;

#[derive(Debug, Clone)]
pub enum Ev {
    Sample(std::sync::Arc<Value>),
    /// The window isn't seen (true) or is again (false: the next sample also says so).
    Hidden(bool),
    Error(String),
}

static STDIN: Mutex<Option<ChildStdin>> = Mutex::new(None);
static WANT: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());
static PAUSED: AtomicBool = AtomicBool::new(false);
static HIDDEN: AtomicBool = AtomicBool::new(false);
/// When the last frame was drawn (ms since START), and samples come in since then.
static DRAWN_MS: AtomicU64 = AtomicU64::new(0);
static SINCE_DRAWN: AtomicU64 = AtomicU64::new(0);
static START: OnceLock<Instant> = OnceLock::new();

fn now_ms() -> u64 {
    START.get_or_init(Instant::now).elapsed().as_millis() as u64
}

/// The helper: $HEARTH_RESMON_HELPER (CI's test answers, never set on a PC), else Mantel's hearth-resmon.
pub fn helper() -> String {
    std::env::var("HEARTH_RESMON_HELPER").ok().filter(|s| !s.is_empty()).unwrap_or_else(|| hearth_ui::run::shell_helper("hearth-resmon"))
}

pub fn fixture() -> bool {
    std::env::var_os("HEARTH_RESMON_HELPER").is_some()
}

fn send(line: &str) {
    if let Ok(mut g) = STDIN.lock() {
        if let Some(s) = g.as_mut() {
            let _ = s.write_all(line.as_bytes());
            let _ = s.write_all(b"\n");
            let _ = s.flush();
        }
    }
}

fn config_line() -> String {
    let want = WANT.lock().map(|w| w.clone()).unwrap_or_default();
    let live = !PAUSED.load(Ordering::Relaxed) && !HIDDEN.load(Ordering::Relaxed);
    serde_json::json!({ "interval": if live { 1 } else { 0 }, "want": want }).to_string()
}

/// What the tab on show needs (Mantel's wants()).
pub fn set_want(w: &[&'static str]) {
    if let Ok(mut g) = WANT.lock() {
        if g.as_slice() == w {
            return;
        }
        *g = w.to_vec();
    }
    send(&config_line());
}

pub fn set_paused(p: bool) {
    PAUSED.store(p, Ordering::Relaxed);
    send(&config_line());
}

pub fn hidden() -> bool {
    HIDDEN.load(Ordering::Relaxed)
}

/// Called for every frame the window draws (the title bar's 1 px canvas): the window is seen.
pub fn drawn() {
    DRAWN_MS.store(now_ms(), Ordering::Relaxed);
    SINCE_DRAWN.store(0, Ordering::Relaxed);
    if HIDDEN.swap(false, Ordering::Relaxed) {
        send(&config_line());
        send("{\"now\": true}");
    }
}

/// The sampler's stream: one thread reads the helper's lines (it blocks on the pipe: no polling) and restarts the
/// helper if it ends (at most a few times, two seconds apart).
pub fn stream() -> futures::channel::mpsc::UnboundedReceiver<Ev> {
    let (tx, rx) = futures::channel::mpsc::unbounded();
    let _ = START.get_or_init(Instant::now);
    let _ = std::thread::Builder::new().name("hearth-resmon-feed".into()).spawn(move || {
        let mut tries = 0;
        loop {
            let helper = helper();
            let child = Command::new("python3").arg(&helper).arg("serve").stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn();
            let mut child = match child {
                Ok(c) => c,
                Err(e) => {
                    let _ = tx.unbounded_send(Ev::Error(format!("couldn't start the sampler ({e})")));
                    return;
                }
            };
            if let Ok(mut g) = STDIN.lock() {
                *g = child.stdin.take();
            }
            send(&config_line());
            let started = Instant::now();
            if let Some(out) = child.stdout.take() {
                for line in BufReader::new(out).lines() {
                    let Ok(line) = line else { break };
                    let Ok(v) = serde_json::from_str::<Value>(&line) else { continue };
                    if let Some(e) = v.get("error").and_then(Value::as_str) {
                        if tx.unbounded_send(Ev::Error(e.to_string())).is_err() {
                            return;
                        }
                        continue;
                    }
                    // Two samples with no frame drawn for 2.5 s: the window isn't seen. Stand still until it is.
                    let n = SINCE_DRAWN.fetch_add(1, Ordering::Relaxed) + 1;
                    if n >= 2 && now_ms().saturating_sub(DRAWN_MS.load(Ordering::Relaxed)) > 2500 && !HIDDEN.swap(true, Ordering::Relaxed) {
                        send(&config_line());
                        if tx.unbounded_send(Ev::Hidden(true)).is_err() {
                            return;
                        }
                    }
                    if tx.unbounded_send(Ev::Sample(std::sync::Arc::new(v))).is_err() {
                        return;
                    }
                }
            }
            if let Ok(mut g) = STDIN.lock() {
                *g = None;
            }
            let _ = child.kill();
            let _ = child.wait();
            if started.elapsed().as_secs() > 60 {
                tries = 0;
            }
            tries += 1;
            if tries > 4 {
                let _ = tx.unbounded_send(Ev::Error("the sampler stopped".into()));
                return;
            }
            if tx.unbounded_send(Ev::Error("the sampler stopped; starting it again".into())).is_err() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_secs(2));
        }
    });
    rx
}

/// A one-shot reading through the helper, with a time limit (timeout(1)).
pub fn helper_cmd(secs: u32, args: &[&str]) -> Vec<String> {
    let mut v = vec!["timeout".to_string(), secs.to_string(), "python3".to_string(), helper()];
    v.extend(args.iter().map(|s| s.to_string()));
    v
}

/// The other readings Mantel's page makes (Tailscale's devices, the Wi-Fi's name, the temperature limit, the drives'
/// health); with CI's test answers (HEARTH_RESMON_HELPER) they come from that helper's own `x-…` answers.
pub fn other_cmd(kind: &str, arg: &str) -> Vec<String> {
    if fixture() {
        return helper_cmd(10, &[&format!("x-{kind}"), arg]);
    }
    let sh = |s: &str| vec!["timeout".to_string(), "8".into(), "sh".into(), "-c".into(), s.to_string(), "sh".into(), arg.to_string()];
    match kind {
        "tailscale" => sh("tailscale status --json 2>/dev/null || true"),
        // The Wi-Fi's name (NetworkManager on both editions)
        "ssid" => sh("nmcli -t -g GENERAL.CONNECTION device show \"$1\" 2>/dev/null || true"),
        "hwset" => vec!["timeout".into(), "20".into(), "python3".into(), "/usr/libexec/nimbus/hearth-hwset".into(), "status".into()],
        // smartctl needs root: hearth-hwset through pkexec (49-hearth-memory.rules: no password at the device)
        "health" => vec!["timeout".into(), "60".into(), "pkexec".into(), "/usr/libexec/nimbus/hearth-hwset".into(), "disk-health".into()],
        _ => vec!["true".into()],
    }
}
