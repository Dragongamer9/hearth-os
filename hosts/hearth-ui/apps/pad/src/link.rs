//! The link to hearth-pad --plasma (desktop/shell/hearth-pad, shared with Debian): one JSON object per line each way,
//! events on its stdout, commands on its stdin (its docstring lists both). It's started here, read on a thread of its
//! own (blocked on the pipe: nothing polls), and started again 3 s after it ends, like osk-plasma.qml's Process does,
//! except after exit 2 or 3 (python3-evdev or /dev/uinput missing: trying again can't help).

use futures::channel::mpsc;
use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::process::{ChildStdin, Command, Stdio};
use std::sync::{Mutex, OnceLock};

static STDIN: OnceLock<Mutex<Option<ChildStdin>>> = OnceLock::new();

fn slot() -> &'static Mutex<Option<ChildStdin>> {
    STDIN.get_or_init(|| Mutex::new(None))
}

/// What hearth-pad said, or that it ended (its exit code; -1: it couldn't be started or was killed).
#[derive(Debug, Clone)]
pub enum Ev {
    Line(Value),
    Ended(i32),
}

/// The helper: $HEARTH_PAD_HELPER (checks), else the image's desktop/shell/hearth-pad.
pub fn helper() -> String {
    std::env::var("HEARTH_PAD_HELPER").ok().filter(|s| !s.is_empty()).unwrap_or_else(|| hearth_ui::run::shell_helper("hearth-pad"))
}

/// A command to hearth-pad (dropped when it isn't running, as the QML hosts do).
pub fn send(v: &Value) {
    let Ok(mut g) = slot().lock() else { return };
    if let Some(w) = g.as_mut() {
        let mut s = v.to_string();
        s.push('\n');
        if w.write_all(s.as_bytes()).and_then(|_| w.flush()).is_err() {
            *g = None;
        }
    }
}

pub fn running() -> bool {
    slot().lock().map(|g| g.is_some()).unwrap_or(false)
}

/// hearth-pad --plasma's events, for as long as this program runs.
pub fn stream() -> mpsc::UnboundedReceiver<Ev> {
    let (tx, rx) = mpsc::unbounded();
    let _ = std::thread::Builder::new().name("hearth-pad-link".into()).stack_size(256 * 1024).spawn(move || loop {
        let path = helper();
        let child = Command::new("python3")
            .arg(&path)
            .arg("--plasma")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn();
        let mut child = match child {
            Ok(c) => c,
            Err(e) => {
                eprintln!("hearth-pad-host: couldn't start {path}: {e}");
                let _ = tx.unbounded_send(Ev::Ended(-1));
                return;
            }
        };
        if let Ok(mut g) = slot().lock() {
            *g = child.stdin.take();
        }
        if let Some(out) = child.stdout.take() {
            for line in BufReader::new(out).lines() {
                let Ok(line) = line else { break };
                let Ok(v) = serde_json::from_str::<Value>(line.trim()) else { continue };
                if tx.unbounded_send(Ev::Line(v)).is_err() {
                    let _ = child.kill();
                    let _ = child.wait();
                    return;
                }
            }
        }
        if let Ok(mut g) = slot().lock() {
            *g = None;
        }
        let code = child.wait().ok().and_then(|s| s.code()).unwrap_or(-1);
        if tx.unbounded_send(Ev::Ended(code)).is_err() || code == 2 || code == 3 {
            return;
        }
        std::thread::sleep(std::time::Duration::from_secs(3));
    });
    rx
}
