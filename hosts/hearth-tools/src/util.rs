//! Small things every tool uses: running programs (with a time limit, or line by line for long jobs), Hearth's
//! paths and edition, the root helper (hearth-admin through pkexec), and argument checks.

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// Hearth's root helper: each command does one narrow job and checks its input (os/system_files/usr/libexec/nimbus).
pub const ADMIN: &str = "/usr/libexec/nimbus/hearth-admin";

pub struct Out {
    pub code: i32,
    pub out: String,
    pub err: String,
}

impl Out {
    pub fn ok(&self) -> bool { self.code == 0 }
    /// The last line that says something (stderr first), for a short error message.
    pub fn why(&self) -> String {
        let pick = |s: &str| s.lines().rev().find(|l| !l.trim().is_empty()).map(|l| l.trim().to_string());
        pick(&self.err).or_else(|| pick(&self.out)).unwrap_or_else(|| format!("exit code {}", self.code))
    }
}

pub fn home() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).filter(|p| p.is_absolute()).unwrap_or_else(|| PathBuf::from("/"))
}

/// ~/.local/share/nimbus: where Hearth keeps its own files and logs.
pub fn data_dir() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).filter(|p| p.is_absolute()).unwrap_or_else(|| home().join(".local/share")).join("nimbus")
}

/// Is `prog` on PATH (or an absolute path that exists)?
pub fn which(prog: &str) -> Option<PathBuf> {
    if prog.contains('/') { return Some(PathBuf::from(prog)).filter(|p| p.is_file()); }
    let path = std::env::var_os("PATH").unwrap_or_else(|| "/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin".into());
    std::env::split_paths(&path).map(|d| d.join(prog)).find(|p| p.is_file())
}

fn command(prog: &str, args: &[&str]) -> Command {
    let mut c = Command::new(prog);
    c.args(args);
    // Plain English and dots for decimals in what we parse, but UTF-8 kept (journal messages, file names).
    c.env("LC_ALL", "C.UTF-8").env("LANG", "C.UTF-8");
    c
}

/// Runs a program and waits at most `timeout`. `input` goes to its standard input.
pub fn run_in(prog: &str, args: &[&str], input: Option<&[u8]>, timeout: Duration) -> Result<Out, String> {
    let mut cmd = command(prog, args);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).stdin(if input.is_some() { Stdio::piped() } else { Stdio::null() });
    let mut child = cmd.spawn().map_err(|e| if e.kind() == std::io::ErrorKind::NotFound { format!("{prog} isn't installed") } else { format!("{prog}: {e}") })?;
    if let (Some(data), Some(mut si)) = (input, child.stdin.take()) {
        let d = data.to_vec();
        std::thread::spawn(move || { let _ = si.write_all(&d); });
    }
    let mut so = child.stdout.take();
    let mut se = child.stderr.take();
    let t1 = std::thread::spawn(move || { let mut b = Vec::new(); if let Some(s) = so.as_mut() { let _ = s.read_to_end(&mut b); } b });
    let t2 = std::thread::spawn(move || { let mut b = Vec::new(); if let Some(s) = se.as_mut() { let _ = s.read_to_end(&mut b); } b });
    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break Some(s),
            Ok(None) => {}
            Err(e) => return Err(e.to_string()),
        }
        if start.elapsed() > timeout { let _ = child.kill(); let _ = child.wait(); break None; }
        std::thread::sleep(Duration::from_millis(15));
    };
    let out = t1.join().unwrap_or_default();
    let err = t2.join().unwrap_or_default();
    match status {
        None => Err(format!("{prog} took too long and was stopped")),
        Some(s) => Ok(Out { code: s.code().unwrap_or(-1), out: String::from_utf8_lossy(&out).into_owned(), err: String::from_utf8_lossy(&err).into_owned() }),
    }
}

pub fn run(prog: &str, args: &[&str], timeout_secs: u64) -> Result<Out, String> { run_in(prog, args, None, Duration::from_secs(timeout_secs)) }

/// Runs a program and hands each line of its output (stdout and stderr) to `line` as it comes; stops it when
/// `cancel` is set. Returns its exit code (None when it was stopped).
pub fn run_lines(prog: &str, args: &[&str], cancel: &AtomicBool, mut line: impl FnMut(&str)) -> Result<Option<i32>, String> {
    let mut cmd = command(prog, args);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).stdin(Stdio::null());
    let mut child = cmd.spawn().map_err(|e| if e.kind() == std::io::ErrorKind::NotFound { format!("{prog} isn't installed") } else { format!("{prog}: {e}") })?;
    let (tx, rx) = std::sync::mpsc::channel::<String>();
    for r in [child.stdout.take().map(|s| Box::new(s) as Box<dyn Read + Send>), child.stderr.take().map(|s| Box::new(s) as Box<dyn Read + Send>)].into_iter().flatten() {
        let tx = tx.clone();
        std::thread::spawn(move || { for l in BufReader::new(r).lines().map_while(Result::ok) { if tx.send(l).is_err() { break; } } });
    }
    drop(tx);
    loop {
        if cancel.load(Ordering::SeqCst) { let _ = child.kill(); let _ = child.wait(); return Ok(None); }
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(l) => line(&l),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    Ok(Some(status.code().unwrap_or(-1)))
}

/// Starts a program without waiting for it (opening a folder, another app).
pub fn spawn(prog: &str, args: &[&str]) -> Result<(), String> {
    command(prog, args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().map(|_| ()).map_err(|e| format!("Couldn't open it ({prog}: {e})"))
}

/// hearth-admin's answer: "error: ..." lines mean it refused or failed; the last JSON line is the result.
pub fn parse_admin(o: &Out) -> Result<Value, String> {
    if let Some(e) = o.out.lines().chain(o.err.lines()).find_map(|l| l.strip_prefix("error: ")) {
        return Err(e.trim().to_string());
    }
    // pkexec: 126 = the password prompt was dismissed, 127 = not allowed (or no agent to ask).
    if o.code == 126 { return Err("Cancelled.".into()); }
    if o.code == 127 && o.out.trim().is_empty() { return Err("You aren't allowed to do that here (Hearth's system helper refused).".into()); }
    if let Some(v) = o.out.lines().rev().find_map(|l| serde_json::from_str::<Value>(l.trim()).ok().filter(|v| v.is_object())) { return Ok(v); }
    if o.ok() { Ok(json!({"ok": true})) } else { Err(o.why()) }
}

/// Runs a hearth-admin command as root (pkexec; Hearth's polkit rule lets the person at the device do it).
pub fn admin(args: &[&str], input: Option<&[u8]>, timeout_secs: u64) -> Result<Value, String> {
    if !Path::new(ADMIN).exists() { return Err("Hearth's system helper (hearth-admin) isn't installed here.".into()); }
    let mut full = vec![ADMIN];
    full.extend_from_slice(args);
    let o = run_in("pkexec", &full, input, Duration::from_secs(timeout_secs))?;
    parse_admin(&o)
}

/// The same, keeping its raw output (for commands that print many lines, like journal-read).
pub fn admin_raw(args: &[&str], timeout_secs: u64) -> Result<String, String> {
    if !Path::new(ADMIN).exists() { return Err("Hearth's system helper (hearth-admin) isn't installed here.".into()); }
    let mut full = vec![ADMIN];
    full.extend_from_slice(args);
    let o = run_in("pkexec", &full, None, Duration::from_secs(timeout_secs))?;
    if let Some(e) = o.out.lines().chain(o.err.lines()).find_map(|l| l.strip_prefix("error: ")) { return Err(e.trim().to_string()); }
    if o.code == 126 { return Err("Cancelled.".into()); }
    if !o.ok() { return Err(o.why()); }
    Ok(o.out)
}

/// /usr/share/nimbus/image-info.json: which edition this is ("anatase", "peppermint").
pub fn edition() -> String {
    std::fs::read_to_string("/usr/share/nimbus/image-info.json").ok()
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .and_then(|v| v["base"].as_str().map(str::to_string))
        .unwrap_or_default()
}

/// Hearth's version (the image's), or this program's.
pub fn version() -> String {
    std::fs::read_to_string("/usr/share/nimbus/version").ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string())
}

/// A string argument from the page.
pub fn arg<'a>(args: &'a Value, key: &str) -> &'a str { args.get(key).and_then(Value::as_str).unwrap_or("") }
pub fn arg_u64(args: &Value, key: &str, default: u64) -> u64 { args.get(key).and_then(Value::as_u64).unwrap_or(default) }
pub fn arg_i64(args: &Value, key: &str, default: i64) -> i64 { args.get(key).and_then(Value::as_i64).unwrap_or(default) }

/// A host name or IP address to ping, trace or look up: letters, digits, dots, colons and dashes only, never
/// starting with a dash (so it can't be taken for an option).
pub fn valid_host(h: &str) -> bool {
    !h.is_empty() && h.len() <= 253 && !h.starts_with('-') && !h.starts_with('.') && h.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | ':' | '-' | '_'))
}

/// Base64 (for images handed to the page as data: URLs).
pub fn base64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len() * 4 / 3 + 4);
    for c in data.chunks(3) {
        let b = [c[0], *c.get(1).unwrap_or(&0), *c.get(2).unwrap_or(&0)];
        let n = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if c.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if c.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    out
}

/// The local date and time as "2026-10-22 14.05" (for file names), from `date` so the time zone is right.
pub fn stamp() -> String {
    run("date", &["+%Y-%m-%d %H.%M"], 5).ok().map(|o| o.out.trim().to_string()).filter(|s| !s.is_empty())
        .unwrap_or_else(|| format!("{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)))
}

/// An XDG folder (DOWNLOAD, DOCUMENTS, PICTURES...) from user-dirs.dirs, or the usual English name.
pub fn user_dir(kind: &str, fallback: &str) -> PathBuf {
    let h = home();
    let cfg = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| h.join(".config"));
    if let Ok(t) = std::fs::read_to_string(cfg.join("user-dirs.dirs")) {
        let key = format!("XDG_{kind}_DIR=");
        for l in t.lines() {
            if let Some(v) = l.trim().strip_prefix(&key) {
                let v = v.trim().trim_matches('"').replace("$HOME", &h.to_string_lossy());
                if v.starts_with('/') { return PathBuf::from(v); }
            }
        }
    }
    h.join(fallback)
}

/// Where a file gets saved: `name` in `dir`, with " (2)", " (3)"... if that's taken.
pub fn unique_path(dir: &Path, name: &str) -> PathBuf {
    let p = dir.join(name);
    if !p.exists() { return p; }
    let (stem, ext) = match name.rsplit_once('.') { Some((s, e)) if !s.is_empty() => (s.to_string(), format!(".{e}")), _ => (name.to_string(), String::new()) };
    (2..1000).map(|n| dir.join(format!("{stem} ({n}){ext}"))).find(|p| !p.exists()).unwrap_or(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hosts() {
        assert!(valid_host("example.com") && valid_host("1.1.1.1") && valid_host("2606:4700::1111") && valid_host("my-pc.local"));
        assert!(!valid_host("-f") && !valid_host("a b") && !valid_host("x;rm") && !valid_host("") && !valid_host("$(id)"));
    }

    #[test]
    fn b64() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn admin_answers() {
        let o = |code, out: &str| Out { code, out: out.into(), err: String::new() };
        assert_eq!(parse_admin(&o(1, "error: the password is incorrect\n")).unwrap_err(), "the password is incorrect");
        assert_eq!(parse_admin(&o(0, "log: x\n{\"ok\": true, \"psk\": \"x\"}\n")).unwrap()["psk"], "x");
        assert_eq!(parse_admin(&o(126, "")).unwrap_err(), "Cancelled.");
    }

    #[test]
    fn runs_programs() {
        let o = run("sh", &["-c", "echo hi; echo oops >&2; exit 3"], 5).unwrap();
        assert_eq!((o.code, o.out.trim(), o.err.trim()), (3, "hi", "oops"));
        assert!(run("sleep", &["5"], 1).is_err());
        let mut got = vec![];
        let stop = AtomicBool::new(false);
        let code = run_lines("sh", &["-c", "echo a; echo b"], &stop, |l| got.push(l.to_string())).unwrap();
        got.sort();
        assert_eq!((code, got), (Some(0), vec!["a".to_string(), "b".to_string()]));
    }
}
