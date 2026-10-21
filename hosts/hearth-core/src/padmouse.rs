//! Controller mouse in Game Mode (Quick Access › Controller mouse, or View + Menu together): the right stick moves
//! the pointer, A / RT click, X / LT right-click, the left stick scrolls, B is Esc, the D-pad the arrow keys. For the
//! windows a controller can't drive: a game's launcher, the All-in-One launcher, an installer.
//!
//! It's the Desktop Mode helper (hearth-pad, a virtual mouse through /dev/uinput) started with `--game`: there it only
//! does the mouse, and Guide, Menu, LB/RB and Y stay the game's and Hearth's. It pauses while Hearth or its Quick
//! Access is in front (the controller is Hearth's then: A must not click as well), set by the host through
//! `set_pause_probe`. View + Menu together turns it off again from inside the helper. Its standard input is Hearth's:
//! when Hearth goes, the helper goes too.

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

pub const HELPER: &str = "/usr/share/nimbus/desktop/shell/hearth-pad";

struct Running { child: Child, stdin: ChildStdin, paused: bool }

static RUNNING: Mutex<Option<Running>> = Mutex::new(None);
static PROBE: OnceLock<Arc<dyn Fn() -> bool + Send + Sync>> = OnceLock::new();

/// The host says how to tell that Hearth (or its Quick Access) is in front, when the helper pauses.
pub fn set_pause_probe(f: Arc<dyn Fn() -> bool + Send + Sync>) { let _ = PROBE.set(f); }

/// The helper's arguments: Game Mode, and the pointer speed from Settings when there is one.
pub fn helper_args() -> Vec<String> { vec![HELPER.to_string(), "--game".to_string()] }

/// Is it on?
pub fn is_on() -> bool {
    let mut r = RUNNING.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(run) = r.as_mut() {
        if matches!(run.child.try_wait(), Ok(None)) { return true; }
        *r = None;
    }
    false
}

fn send(run: &mut Running, v: &Value) -> bool { writeln!(run.stdin, "{v}").and_then(|_| run.stdin.flush()).is_ok() }

fn tell_page(on: bool) { crate::stores::notify(json!({"type": "event", "name": "padMouse", "data": {"on": on}})); }

/// Turns it off.
pub fn stop() {
    let taken = RUNNING.lock().unwrap_or_else(|e| e.into_inner()).take();
    if let Some(mut run) = taken {
        drop(run.stdin);
        let _ = run.child.kill();
        let _ = run.child.wait();
        crate::bridge::log("pad mouse: off");
    }
}

/// Turns it on (Game Mode only).
pub fn start() -> Result<(), String> {
    if is_on() { return Ok(()); }
    if !crate::gamescope::inside() { return Err("The controller mouse is for Game Mode; the desktop has its own (Quick Access there).".into()); }
    if !std::path::Path::new(HELPER).is_file() { return Err("The controller helper (hearth-pad) isn't installed.".into()); }
    let args = helper_args();
    let mut child = Command::new("python3").args(&args).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn()
        .map_err(|e| format!("Couldn't start the controller mouse: {e}"))?;
    let stdin = child.stdin.take().ok_or("Couldn't start the controller mouse.")?;
    let out = child.stdout.take();
    let pid = child.id();
    *RUNNING.lock().unwrap_or_else(|e| e.into_inner()) = Some(Running { child, stdin, paused: false });
    crate::bridge::log(&format!("pad mouse: on (process {pid})"));
    // What it says: errors (no uinput, no evdev), and "mouse off" (View + Menu).
    std::thread::spawn(move || {
        if let Some(o) = out {
            for line in BufReader::new(o).lines().map_while(Result::ok) {
                let Ok(v) = serde_json::from_str::<Value>(&line) else { continue };
                match v["t"].as_str() {
                    Some("error") => { crate::bridge::log(&format!("pad mouse: {}", v["text"].as_str().unwrap_or("?"))); }
                    Some("mouse") if v["on"] == false => { crate::bridge::log("pad mouse: turned off with View + Menu"); break; }
                    _ => {}
                }
            }
        }
        // Ended (or turned itself off): only if it's still the same helper.
        let mut r = RUNNING.lock().unwrap_or_else(|e| e.into_inner());
        if r.as_ref().map(|x| x.child.id() == pid).unwrap_or(false) {
            if let Some(mut run) = r.take() { let _ = run.child.kill(); let _ = run.child.wait(); }
            drop(r);
            tell_page(false);
        }
    });
    // Pauses while Hearth is in front.
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(300));
        let front = PROBE.get().map(|p| p()).unwrap_or(false);
        let mut r = RUNNING.lock().unwrap_or_else(|e| e.into_inner());
        let Some(run) = r.as_mut().filter(|x| x.child.id() == pid) else { break };
        if run.paused != front {
            run.paused = front;
            if !send(run, &json!({"cmd": "pause", "on": front})) { break; }
        }
    });
    Ok(())
}

/// Quick Access's button: on ↔ off. Returns whether it's on now.
pub fn toggle() -> Result<bool, String> {
    if is_on() { stop(); tell_page(false); Ok(false) } else { start()?; tell_page(true); Ok(true) }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn game_mode_helper() {
        assert_eq!(helper_args(), vec![HELPER.to_string(), "--game".to_string()]);
        assert!(!is_on());
    }
}
