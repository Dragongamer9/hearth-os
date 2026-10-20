//! The open-source tools Hearth uses for Epic, GOG and Windows games: legendary, gogdl, umu-launcher.
//! Downloaded from their GitHub releases the first time, checked against a pinned SHA-256, kept in
//! ~/.local/share/nimbus/tools (the same place the Electron host keeps them). Port of stores/tools.js.

use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub struct Tool { pub name: &'static str, pub file: &'static str, pub url: &'static str, pub sha256: &'static str, pub tar: bool, pub system: Option<&'static str> }

pub const LEGENDARY: Tool = Tool { name: "legendary", file: "legendary", url: "https://github.com/legendary-gl/legendary/releases/download/0.21.1/legendary_linux_x64", sha256: "dbed33bbe96031e65858233e4badf0a1d401bb6cb0cc995d1237cf3a0c826a45", tar: false, system: None };
pub const GOGDL: Tool = Tool { name: "gogdl", file: "gogdl", url: "https://github.com/Heroic-Games-Launcher/heroic-gogdl/releases/download/v1.3.0/gogdl_linux_x86_64", sha256: "cba013d42767c808237c437335ab1d56f58405d07e8f37b3324d264ea5c49655", tar: false, system: None };
pub const UMU: Tool = Tool { name: "umu-launcher", file: "umu/umu-run", url: "https://github.com/Open-Wine-Components/umu-launcher/releases/download/1.4.4/umu-launcher-1.4.4-zipapp.tar", sha256: "eb590691841f7fad3fc3ad8fd5db4ccb87849fe7948e62b28ece7a4ee48cc851", tar: true, system: Some("umu-run") };

pub fn dir() -> PathBuf { crate::paths::data_dir().join("tools") }
fn path_of(t: &Tool) -> PathBuf { dir().join(t.file) }
pub fn is_ready(t: &Tool) -> bool { t.system.map(crate::system::has).unwrap_or(false) || path_of(t).exists() }

static DOWNLOADING: Mutex<()> = Mutex::new(());

/// The program to run, downloading it first if needed (one download at a time).
pub fn ensure(t: &Tool) -> Result<String, String> {
    if let Some(s) = t.system { if crate::system::has(s) { return Ok(s.to_string()); } }
    let dst = path_of(t);
    if dst.exists() { return Ok(dst.to_string_lossy().into()); }
    crate::watchdog::long_task();
    let _one = DOWNLOADING.lock().unwrap_or_else(|p| p.into_inner());
    if dst.exists() { return Ok(dst.to_string_lossy().into()); }
    std::fs::create_dir_all(dir()).map_err(|e| e.to_string())?;
    let tmp = dir().join(format!("{}.download", t.url.rsplit('/').next().unwrap_or("tool")));
    let tmps = tmp.to_string_lossy().to_string();
    crate::system::run_timeout("curl", &["-fsSL", "-A", "Hearth", "-o", &tmps, t.url], std::time::Duration::from_secs(600)).map_err(|e| format!("Couldn't download {} ({e})", t.name))?;
    let sum = crate::system::run("sha256sum", &[&tmps]).unwrap_or_default();
    if sum.split_whitespace().next() != Some(t.sha256) {
        let _ = std::fs::remove_file(&tmp);
        return Err(format!("The download of {} didn't match its checksum, so Hearth didn't use it. Try again later.", t.name));
    }
    if t.tar {
        crate::system::run("tar", &["-xf", &tmps, "-C", &dir().to_string_lossy()])?;
        let _ = std::fs::remove_file(&tmp);
        if !dst.exists() { return Err(format!("{}'s download didn't contain {}", t.name, t.file)); }
    } else { std::fs::rename(&tmp, &dst).map_err(|e| e.to_string())?; }
    crate::system::run("chmod", &["755", &dst.to_string_lossy()])?;
    crate::bridge::log(&format!("tools: downloaded {}", t.name));
    Ok(dst.to_string_lossy().into())
}

pub struct Output { pub code: i32, pub out: String, pub err: String }

/// The last meaningful error line of a tool's output ("[cli] ERROR: Login failed" → "Login failed").
pub fn problem(o: &Output) -> String {
    let all = format!("{}\n{}", o.err, o.out);
    let lines: Vec<&str> = all.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    let line = lines.iter().rev().find(|l| l.contains("ERROR") || l.contains("CRITICAL")).or(lines.last()).map(|s| s.to_string()).unwrap_or_else(|| format!("exit code {}", o.code));
    match (line.starts_with('['), line.find(": ")) { (true, Some(i)) => line[i + 2..].to_string(), _ => line }
}

/// Runs a tool and collects its output; `on_line` sees each line as it comes (they log progress to stderr).
/// `cancel` stops it (and everything it started).
pub fn run(t: &Tool, args: &[&str], env: &[(&str, String)], on_line: Option<Arc<dyn Fn(&str) + Send + Sync>>, cancel: Option<Arc<AtomicBool>>, timeout: Option<std::time::Duration>) -> Result<Output, String> {
    let exe = ensure(t)?;
    let mut cmd = Command::new(&exe);
    cmd.args(args).current_dir(dir()).env("PYTHONIOENCODING", "utf-8").env("PYTHONUNBUFFERED", "1")
        .stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    for (k, v) in env { cmd.env(k, v); }
    // Its own process group, so cancelling stops everything it started.
    cmd.process_group(0);
    let mut child = cmd.spawn().map_err(|e| format!("Couldn't start {}: {e}", t.name))?;
    let pid = child.id();
    let collect = |stream: Box<dyn std::io::Read + Send>, cb: Option<Arc<dyn Fn(&str) + Send + Sync>>| {
        std::thread::spawn(move || {
            let mut all = String::new();
            let mut r = BufReader::new(stream);
            let mut buf = Vec::new();
            // Split on \n and \r (progress lines rewrite themselves with \r).
            loop {
                buf.clear();
                let mut byte = [0u8; 1];
                let mut got = false;
                while std::io::Read::read(&mut r, &mut byte).map(|n| n == 1).unwrap_or(false) {
                    got = true;
                    if byte[0] == b'\n' || byte[0] == b'\r' { break; }
                    buf.push(byte[0]);
                }
                if !got { break; }
                let line = String::from_utf8_lossy(&buf).into_owned();
                if !line.is_empty() { if let Some(c) = &cb { c(&line); } all.push_str(&line); all.push('\n'); }
            }
            let _ = r.fill_buf();
            all
        })
    };
    let o = collect(Box::new(child.stdout.take().unwrap()), on_line.clone());
    let e = collect(Box::new(child.stderr.take().unwrap()), on_line);
    let start = std::time::Instant::now();
    let code = loop {
        if let Ok(Some(s)) = child.try_wait() { break s.code().unwrap_or(-1); }
        let cancelled = cancel.as_ref().map(|c| c.load(Ordering::SeqCst)).unwrap_or(false);
        let late = timeout.map(|t| start.elapsed() > t).unwrap_or(false);
        if cancelled || late {
            let _ = Command::new("kill").args(["-TERM", &format!("-{pid}")]).status();
            std::thread::sleep(std::time::Duration::from_secs(3));
            let _ = Command::new("kill").args(["-KILL", &format!("-{pid}")]).status();
            let _ = child.wait();
            return Err(if cancelled { "cancelled".into() } else { format!("{} took too long", t.name) });
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    };
    Ok(Output { code, out: o.join().unwrap_or_default(), err: e.join().unwrap_or_default() })
}

use std::os::unix::process::CommandExt;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn problems_read_like_sentences() {
        assert_eq!(problem(&Output { code: 1, out: String::new(), err: "[cli] INFO: x\n[cli] ERROR: Login failed\n".into() }), "Login failed");
        assert_eq!(problem(&Output { code: 3, out: String::new(), err: String::new() }), "exit code 3");
    }
}
