//! Clipboard between your devices (Settings › Hearth Link › Share between them › Clipboard, "Copy here, paste
//! there"; approved mockup settings-cleanup/settings-05). Off by default: the `clipboard` setting (config.json),
//! changed only by the person (`settings.set {"clipboard": true}`).
//!
//! - **Text only**, at most MAX_TEXT bytes: what you copy on one device can be pasted on your others. Pictures and
//!   files never go (wl-paste is asked for text/plain only).
//! - **Only your own devices**: sent only to paired devices that are yours (never a friend's or a family member's),
//!   over Hearth Link's encrypted connections; a device takes it only from one of yours (family.rs `allowed`: a
//!   friend or family member's "clip.put" is refused) and only while its own switch is on.
//! - On Wayland with wl-clipboard (wl-paste / wl-copy, the data-control protocol labwc has). Light at rest: while the
//!   switch is off nothing runs; while it's on, wl-paste is asked every POLL_SECS (a small process, no window), and a
//!   copy goes to each device with a short time limit, side by side, so a device that's away holds nothing up.
//! - What came from another device isn't sent back (no ping-pong between devices).

use crate::node::Node;
use serde_json::{json, Value};
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

/// The most text that's shared (64 KiB): a long note or a page of code, never a whole file.
pub const MAX_TEXT: usize = 64 * 1024;
pub const POLL_SECS: u64 = 2;

/// The last text seen on (or put on) this device's clipboard.
fn last() -> &'static Mutex<String> {
    static L: OnceLock<Mutex<String>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(String::new()))
}

/// Text that may be shared: not empty, not too big, no NUL.
pub fn shareable(t: &str) -> bool {
    !t.trim().is_empty() && t.len() <= MAX_TEXT && !t.contains('\0')
}

/// The Wayland display for wl-paste / wl-copy: the service's own, or the first wayland-N socket in the runtime dir
/// (a user service may start before the session's environment is imported).
fn wayland_env() -> Option<(String, String)> {
    if let Ok(d) = std::env::var("WAYLAND_DISPLAY") {
        if !d.is_empty() {
            return Some(("WAYLAND_DISPLAY".into(), d));
        }
    }
    let rt = std::env::var("XDG_RUNTIME_DIR").ok()?;
    let mut names: Vec<String> = std::fs::read_dir(rt).ok()?.flatten().map(|e| e.file_name().to_string_lossy().to_string()).filter(|n| n.starts_with("wayland-") && !n.ends_with(".lock")).collect();
    names.sort();
    names.into_iter().next().map(|n| ("WAYLAND_DISPLAY".into(), n))
}

fn tool(name: &str) -> Option<Command> {
    let (k, v) = wayland_env()?;
    let mut c = Command::new(name);
    c.env(k, v).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null());
    Some(c)
}

/// The clipboard's text now (None: no text on it, no Wayland, or no wl-paste).
pub fn read_text() -> Option<String> {
    let mut c = tool("wl-paste")?;
    c.args(["--no-newline", "--type", "text/plain"]);
    let child = c.spawn().ok()?;
    let out = wait_with_limit(child, Duration::from_secs(3))?;
    if !out.status.success() || out.stdout.len() > MAX_TEXT {
        return None;
    }
    String::from_utf8(out.stdout).ok()
}

/// Puts `text` on this device's clipboard (wl-copy keeps serving it in the background).
pub fn write_text(text: &str) -> Result<(), String> {
    let mut c = tool("wl-copy").ok_or("No clipboard here (no Wayland session or wl-clipboard)")?;
    c.args(["--type", "text/plain"]).stdin(Stdio::piped()).stdout(Stdio::null());
    let mut child = c.spawn().map_err(|e| format!("wl-copy: {e}"))?;
    if let Some(mut i) = child.stdin.take() {
        i.write_all(text.as_bytes()).map_err(|e| format!("wl-copy: {e}"))?;
    }
    // wl-copy forks to serve the text; its first process ends at once.
    let st = wait_with_limit(child, Duration::from_secs(3)).ok_or("wl-copy didn't answer")?;
    if st.status.success() { Ok(()) } else { Err("wl-copy couldn't take it".into()) }
}

fn wait_with_limit(mut child: std::process::Child, limit: Duration) -> Option<std::process::Output> {
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return child.wait_with_output().ok(),
            Ok(None) if start.elapsed() < limit => std::thread::sleep(Duration::from_millis(30)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
}

impl Node {
    fn clipboard_on(&self) -> bool {
        crate::lock(&self.cfg).clipboard
    }

    /// The background watch (spawn_all): while the switch is on, new text copied here goes to your other devices.
    pub async fn clipboard_loop(self: Arc<Self>) {
        let mut was_on = false;
        loop {
            tokio::time::sleep(Duration::from_secs(POLL_SECS)).await;
            let on = self.clipboard_on();
            if !on {
                was_on = false;
                continue;
            }
            let Ok(Some(text)) = tokio::task::spawn_blocking(read_text).await else { continue };
            {
                let mut l = crate::lock(last());
                if !was_on {
                    // Just turned on: what's already on the clipboard isn't sent, only what's copied from now on.
                    *l = text;
                    was_on = true;
                    continue;
                }
                if *l == text {
                    continue;
                }
                *l = text.clone();
            }
            if !shareable(&text) {
                continue;
            }
            let ids: Vec<String> = crate::lock(&self.peers).mine().map(|p| p.id.clone()).collect();
            for id in ids {
                let n = self.clone();
                let t = text.clone();
                tokio::spawn(async move {
                    let _ = tokio::time::timeout(Duration::from_secs(10), n.request(&id, &json!({"op": "clip.put", "text": t}))).await;
                });
            }
        }
    }

    /// From another device: clip.put {text}. Only from your own devices (family.rs) and only while the switch is on.
    pub async fn clipboard_answer(self: Arc<Self>, peer: &str, op: &str, req: &Value) -> Result<Value, String> {
        match op {
            "clip.put" => {
                if !crate::lock(&self.peers).is_mine(peer) {
                    return Err("Only your own devices share the clipboard".into());
                }
                if !self.clipboard_on() {
                    return Err(format!("Clipboard sharing is off on {}", self.name()));
                }
                let text = req["text"].as_str().unwrap_or("").to_string();
                if !shareable(&text) {
                    return Err("Only text, up to 64 KB, is shared".into());
                }
                *crate::lock(last()) = text.clone();
                tokio::task::spawn_blocking(move || write_text(&text)).await.map_err(|e| e.to_string())??;
                Ok(json!({"ok": true}))
            }
            _ => Err(format!("Unknown request: {op}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_text_of_a_sensible_size() {
        assert!(shareable("hello"));
        assert!(!shareable("   "));
        assert!(!shareable("a\0b"));
        assert!(shareable(&"x".repeat(MAX_TEXT)));
        assert!(!shareable(&"x".repeat(MAX_TEXT + 1)));
    }

    #[test]
    fn off_by_default_and_changed_only_by_the_person() {
        let c = crate::state::Config::default();
        assert!(!c.clipboard);
        let mut c2 = c.clone();
        c2.apply(&json!({"clipboard": true})).unwrap();
        assert!(c2.clipboard);
        assert_eq!(c2.to_json()["clipboard"], true);
        let mut c3 = c2.clone();
        c3.apply(&json!({"name": "PC"})).unwrap();
        assert!(c3.clipboard, "another change leaves it as it was");
    }

    #[test]
    fn friends_and_family_never_get_or_send_it() {
        assert!(!crate::family::friend_may("clip.put"));
    }
}
