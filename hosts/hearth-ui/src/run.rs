//! Running a program off the window's thread: Hearth's helpers (hearth-tasks, hearth_restore.py,
//! hearth_reliability.py) do the work, the apps only show it. Each run is one short-lived thread; nothing polls.

use serde_json::Value;
use std::future::Future;
use std::process::{Command, Stdio};

/// What a program printed (stdout, stderr) and whether it ended well.
#[derive(Debug, Clone)]
pub struct Output {
    pub ok: bool,
    pub code: i32,
    pub out: String,
    pub err: String,
}

impl Output {
    /// Its stdout as JSON (the last JSON line, for helpers that print notes first).
    pub fn json(&self) -> Option<Value> {
        if let Ok(v) = serde_json::from_str::<Value>(self.out.trim()) {
            return Some(v);
        }
        self.out.lines().rev().find_map(|l| serde_json::from_str::<Value>(l.trim()).ok())
    }
    /// A short reason to show when it failed.
    pub fn why(&self) -> String {
        let t = format!("{}\n{}", self.err, self.out);
        t.trim().lines().last().unwrap_or("").trim_start_matches("error: ").to_string()
    }
}

pub fn blocking(cmd: &[String]) -> Output {
    if cmd.is_empty() {
        return Output { ok: false, code: -1, out: String::new(), err: "nothing to run".into() };
    }
    match Command::new(&cmd[0]).args(&cmd[1..]).stdin(Stdio::null()).output() {
        Ok(o) => Output {
            ok: o.status.success(),
            code: o.status.code().unwrap_or(-1),
            out: String::from_utf8_lossy(&o.stdout).to_string(),
            err: String::from_utf8_lossy(&o.stderr).to_string(),
        },
        Err(e) => Output { ok: false, code: -1, out: String::new(), err: format!("{}: {e}", cmd[0]) },
    }
}

/// Runs `cmd` on its own thread; the future is ready when it ended.
pub fn spawn(cmd: Vec<String>) -> impl Future<Output = Output> + Send + 'static {
    let (tx, rx) = futures::channel::oneshot::channel();
    let _ = std::thread::Builder::new().name("hearth-ui-run".into()).spawn(move || {
        let _ = tx.send(blocking(&cmd));
    });
    async move { rx.await.unwrap_or(Output { ok: false, code: -1, out: String::new(), err: "it stopped".into() }) }
}

/// Starts a program and doesn't wait (opening a folder, rebooting).
pub fn detached(cmd: &[&str]) {
    if let Some((p, a)) = cmd.split_first() {
        let _ = Command::new(p).args(a).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn();
    }
}

pub fn args(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

/// Where Mantel's helpers are: $HEARTH_SHELL_DIR, else the image's shell folder.
pub fn shell_dir() -> std::path::PathBuf {
    std::env::var_os("HEARTH_SHELL_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("/usr/share/nimbus/desktop/shell"))
}

pub fn shell_helper(name: &str) -> String {
    shell_dir().join(name).to_string_lossy().to_string()
}

/// Ready after `ms` milliseconds (a short wait before a restart; one sleeping thread).
pub fn after(ms: u64) -> impl Future<Output = ()> + Send + 'static {
    let (tx, rx) = futures::channel::oneshot::channel::<()>();
    let _ = std::thread::Builder::new().name("hearth-ui-wait".into()).stack_size(64 * 1024).spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(ms));
        let _ = tx.send(());
    });
    async move {
        let _ = rx.await;
    }
}
