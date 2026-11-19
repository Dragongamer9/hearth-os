//! Talking to hearth-files (hosts/linux/desktop/shell/hearth-files, Mantel's own backend): every change to files,
//! the places, This PC, the settings (files.json, merged and written the safe way by hearth-files prefs-set) go
//! through it, so Debian's Mantel and the Fedora edition's Rust Files do exactly the same things to the same files.
//!
//! * [`call`]: one run, its last line as JSON (or "error: …"), with a time limit (it's stopped when it runs over);
//! * [`lines`]: a run that keeps printing (watch, search, copy / move with progress): each line as it comes, and
//!   the program is stopped (SIGTERM) the moment the window stops listening (a job cancelled, the folder left).
//! Nothing polls: a run is a thread that waits for its program.

use futures::channel::mpsc;
use futures::StreamExt;
use serde_json::Value;
use std::future::Future;
use std::io::{BufRead, BufReader, Read};
use std::pin::Pin;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

pub fn helper() -> String {
    hearth_ui::run::shell_helper("hearth-files")
}

pub fn argv(args: &[String]) -> Vec<String> {
    let mut v = vec!["python3".to_string(), helper()];
    v.extend(args.iter().cloned());
    v
}

pub fn a(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

/// What hearth-files answered.
#[derive(Debug, Clone)]
pub struct Reply {
    pub value: Value,
    pub err: String,
}

impl Reply {
    pub fn ok(&self) -> bool {
        self.err.is_empty()
    }
    pub fn get(&self, k: &str) -> &Value {
        self.value.get(k).unwrap_or(&Value::Null)
    }
}

fn parse(out: &str, code: i32, timed_out: bool) -> Reply {
    if timed_out {
        return Reply { value: Value::Null, err: "That took too long, so it was stopped".into() };
    }
    let last = out.trim().lines().last().unwrap_or("").trim().to_string();
    if let Some(e) = last.strip_prefix("error: ") {
        return Reply { value: Value::Null, err: e.to_string() };
    }
    match serde_json::from_str::<Value>(&last) {
        Ok(v) => Reply { value: v, err: String::new() },
        Err(_) => Reply {
            value: Value::Null,
            err: if code == 0 { "Something went wrong".into() } else { format!("Something went wrong (code {code})") },
        },
    }
}

/// Runs a program to its end (at most `secs`), off the window's thread.
pub fn run_blocking(cmd: &[String], secs: u64, input: Option<String>) -> (String, i32, bool) {
    if cmd.is_empty() {
        return (String::new(), -1, false);
    }
    let mut c = Command::new(&cmd[0]);
    c.args(&cmd[1..]).stdout(Stdio::piped()).stderr(Stdio::null()).stdin(Stdio::null());
    // Text for the helper (a sharing password) goes in HEARTH_INPUT, which only you can read, never on the command line.
    if let Some(i) = &input {
        c.env("HEARTH_INPUT", i);
    }
    let mut child = match c.spawn() {
        Ok(ch) => ch,
        Err(e) => return (format!("error: {}: {e}", cmd[0]), -1, false),
    };
    let mut out = child.stdout.take();
    let reader = std::thread::Builder::new().name("hearth-files-out".into()).stack_size(256 * 1024).spawn(move || {
        let mut s = String::new();
        if let Some(o) = out.as_mut() {
            let _ = o.read_to_string(&mut s);
        }
        s
    });
    let start = Instant::now();
    let limit = Duration::from_secs(secs.max(1));
    let mut timed_out = false;
    let code = loop {
        match child.try_wait() {
            Ok(Some(st)) => break st.code().unwrap_or(-1),
            Ok(None) => {}
            Err(_) => break -1,
        }
        if start.elapsed() > limit {
            let _ = child.kill();
            let _ = child.wait();
            timed_out = true;
            break -1;
        }
        // Waiting for a short-lived program: a few milliseconds between looks, only while it runs.
        std::thread::sleep(Duration::from_millis(if start.elapsed() < Duration::from_millis(200) { 4 } else { 25 }));
    };
    let s = reader.ok().and_then(|r| r.join().ok()).unwrap_or_default();
    (s, code, timed_out)
}

/// hearth-files ARGS…: its answer.
pub fn call(args: Vec<String>, secs: u64) -> impl Future<Output = Reply> + Send + 'static {
    call_cmd(argv(&args), secs, None)
}

/// Any program whose last line is JSON (hearth-briefcase, hearth-tasks…).
pub fn call_cmd(cmd: Vec<String>, secs: u64, input: Option<String>) -> impl Future<Output = Reply> + Send + 'static {
    let (tx, rx) = futures::channel::oneshot::channel();
    let _ = std::thread::Builder::new().name("hearth-files-call".into()).stack_size(512 * 1024).spawn(move || {
        let (out, code, to) = run_blocking(&cmd, secs, input);
        let _ = tx.send(parse(&out, code, to));
    });
    async move { rx.await.unwrap_or(Reply { value: Value::Null, err: "It stopped".into() }) }
}

/// Text a program printed (the versions search), at most `secs`.
pub fn output(cmd: Vec<String>, secs: u64) -> impl Future<Output = String> + Send + 'static {
    let (tx, rx) = futures::channel::oneshot::channel();
    let _ = std::thread::Builder::new().name("hearth-files-out".into()).stack_size(256 * 1024).spawn(move || {
        let (out, _, _) = run_blocking(&cmd, secs, None);
        let _ = tx.send(out);
    });
    async move { rx.await.unwrap_or_default() }
}

/// Starts a program and doesn't wait (opening a file in its app is hearth-files' own job; this is for wl-copy,
/// hearth-wallpaper…). Its stdin gets `input` when given.
pub fn detached(cmd: &[String], input: Option<&str>) {
    if cmd.is_empty() {
        return;
    }
    let mut c = Command::new(&cmd[0]);
    c.args(&cmd[1..]).stdout(Stdio::null()).stderr(Stdio::null()).stdin(if input.is_some() { Stdio::piped() } else { Stdio::null() });
    if let Ok(mut ch) = c.spawn() {
        if let (Some(i), Some(mut sin)) = (input, ch.stdin.take()) {
            use std::io::Write;
            let _ = sin.write_all(i.as_bytes());
        }
        // Reaped on its own thread, so no program is left a zombie.
        let _ = std::thread::Builder::new().stack_size(64 * 1024).spawn(move || {
            let _ = ch.wait();
        });
    }
}

// ------------------------------------------------------------------ programs that keep printing
/// What a running program said: a line, or that it ended (with its exit code).
#[derive(Debug, Clone)]
pub enum Line {
    Text(u64, String),
    End(u64, i32),
}

/// A program and the number that tells its runs apart (a subscription per run).
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Run {
    pub seq: u64,
    pub cmd: Vec<String>,
}

pub struct LineStream {
    rx: mpsc::UnboundedReceiver<Line>,
    pid: i32,
    done: Arc<AtomicBool>,
}

impl futures::Stream for LineStream {
    type Item = Line;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Line>> {
        self.rx.poll_next_unpin(cx)
    }
}

impl Drop for LineStream {
    fn drop(&mut self) {
        // The window stopped listening (a job cancelled, the folder left, the search cleared): the program stops too
        // (hearth-files removes a half-done copy on SIGTERM).
        if self.pid > 0 && !self.done.load(Ordering::SeqCst) {
            unsafe {
                libc::kill(self.pid, libc::SIGTERM);
            }
        }
    }
}

pub fn lines(r: &Run) -> LineStream {
    let (tx, rx) = mpsc::unbounded();
    let done = Arc::new(AtomicBool::new(false));
    let seq = r.seq;
    let cmd = r.cmd.clone();
    if cmd.is_empty() {
        let _ = tx.unbounded_send(Line::End(seq, -1));
        return LineStream { rx, pid: 0, done };
    }
    let child = Command::new(&cmd[0]).args(&cmd[1..]).stdout(Stdio::piped()).stderr(Stdio::null()).stdin(Stdio::null()).spawn();
    let mut child = match child {
        Ok(c) => c,
        Err(e) => {
            let _ = tx.unbounded_send(Line::Text(seq, format!("error: {}: {e}", cmd[0])));
            let _ = tx.unbounded_send(Line::End(seq, -1));
            done.store(true, Ordering::SeqCst);
            return LineStream { rx, pid: 0, done };
        }
    };
    let pid = child.id() as i32;
    let d2 = done.clone();
    let _ = std::thread::Builder::new().name("hearth-files-lines".into()).stack_size(256 * 1024).spawn(move || {
        if let Some(out) = child.stdout.take() {
            for l in BufReader::new(out).lines() {
                let Ok(l) = l else { break };
                if tx.unbounded_send(Line::Text(seq, l)).is_err() {
                    break;
                }
            }
        }
        let code = child.wait().ok().and_then(|s| s.code()).unwrap_or(-1);
        d2.store(true, Ordering::SeqCst);
        let _ = tx.unbounded_send(Line::End(seq, code));
    });
    LineStream { rx, pid, done }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn answers() {
        let r = parse("progress: {}\n{\"ok\": true}\n", 0, false);
        assert!(r.ok());
        assert_eq!(r.get("ok"), &Value::Bool(true));
        let r = parse("error: That name can't be used\n", 1, false);
        assert_eq!(r.err, "That name can't be used");
        let (o, c, t) = run_blocking(&a(&["sh", "-c", "echo hi"]), 5, None);
        assert_eq!((o.trim(), c, t), ("hi", 0, false));
        let (_, _, t) = run_blocking(&a(&["sleep", "5"]), 1, None);
        assert!(t);
    }
}
