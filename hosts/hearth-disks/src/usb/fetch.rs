//! Fetching with curl (as Hearth Mods does): small pages and checksum files into memory (https only, redirects
//! too), and images into a .part file that picks up where it stopped (curl -C -). An image may be redirected to an
//! http mirror: it's checked against the signed checksum before it's used, so that's safe.
//! Tests serve files from 127.0.0.1 over http: allowed only when HEARTH_USB_TEST_HTTP=1.

use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

pub const UA: &str = concat!("HearthDisks/", env!("CARGO_PKG_VERSION"), " (+https://hearthos.fun)");

fn test_http(url: &str) -> bool {
    std::env::var("HEARTH_USB_TEST_HTTP").map(|v| v == "1").unwrap_or(false) && (url.starts_with("http://127.0.0.1:") || url.starts_with("http://localhost:"))
}

/// Only https addresses (and the tests' own local server) are fetched.
pub fn allowed(url: &str) -> bool { url.starts_with("https://") || test_http(url) }

fn protos(url: &str, redirects_may_be_http: bool) -> [String; 4] {
    let p = if test_http(url) { "=http,https" } else { "=https" };
    let r = if redirects_may_be_http || test_http(url) { "=http,https" } else { "=https" };
    ["--proto".into(), p.into(), "--proto-redir".into(), r.into()]
}

/// A page or small file (at most 16 MB).
pub fn get(url: &str) -> Result<Vec<u8>, String> {
    if !allowed(url) { return Err(format!("Only https addresses are fetched ({url})")); }
    let out = Command::new("curl")
        .args(["-fsSL", "--max-time", "60", "--connect-timeout", "20", "--retry", "2", "--max-filesize", "16777216", "-A", UA])
        .args(protos(url, false))
        .arg("--").arg(url)
        .stdin(Stdio::null()).output().map_err(|e| format!("curl isn't installed ({e})"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(plain_curl(out.status.code().unwrap_or(-1), &err, url));
    }
    Ok(out.stdout)
}

/// HEAD: (status, length) after redirects. For the catalog check.
pub fn head(url: &str) -> Result<(u32, Option<u64>), String> {
    if !allowed(url) { return Err("not https".into()); }
    let out = Command::new("curl").args(["-sSIL", "--max-time", "60", "-A", UA, "-o", "/dev/null", "-w", "%{http_code} %header{content-length}"])
        .args(protos(url, true)).arg("--").arg(url).output().map_err(|e| e.to_string())?;
    let s = String::from_utf8_lossy(&out.stdout).to_string();
    let mut it = s.split_whitespace();
    let code = it.next().and_then(|c| c.parse().ok()).unwrap_or(0);
    let len = it.next().and_then(|l| l.trim().parse().ok());
    Ok((code, len))
}

fn plain_curl(code: i32, err: &str, url: &str) -> String {
    match code {
        6 | 7 => "Couldn't reach the download server. Check the internet connection.".into(),
        22 => format!("The server said no ({}) for {url}", err.rsplit(':').next().unwrap_or(err).trim()),
        23 => "Couldn't save the download: is the drive full?".into(),
        28 => "The download server stopped answering.".into(),
        35 | 51 | 58 | 60 => "The download server's certificate couldn't be checked, so nothing was downloaded.".into(),
        1 => "The download was sent somewhere that isn't https, so it was stopped.".into(),
        _ => format!("The download didn't work ({err})"),
    }
}

/// How a download is going.
#[derive(Clone, Copy, Debug, Default)]
pub struct Got { pub done: u64, pub total: Option<u64>, pub speed: f64, pub eta: Option<u64> }

/// Downloads `url` into `part`, continuing what's already there. `progress` about twice a second.
pub fn download(url: &str, part: &Path, total: Option<u64>, cancel: &AtomicBool, progress: &dyn Fn(Got)) -> Result<(), String> {
    if !allowed(url) { return Err(format!("Only https addresses are downloaded ({url})")); }
    if let Some(d) = part.parent() { std::fs::create_dir_all(d).map_err(|e| format!("Couldn't make {}: {e}", d.display()))?; }
    let mut tries = 0;
    loop {
        tries += 1;
        let resume = part.exists() && std::fs::metadata(part).map(|m| m.len() > 0).unwrap_or(false);
        if let (true, Some(t)) = (resume, total) {
            if std::fs::metadata(part).map(|m| m.len()).unwrap_or(0) == t { return Ok(()); }
        }
        let mut c = Command::new("curl");
        c.args(["-fsSL", "--connect-timeout", "20", "--speed-time", "60", "--speed-limit", "1024", "-A", UA]).args(protos(url, true));
        if resume { c.args(["-C", "-"]); }
        c.arg("-o").arg(part).arg("--").arg(url).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::piped());
        let mut child = c.spawn().map_err(|e| format!("curl isn't installed ({e})"))?;
        let start_size = std::fs::metadata(part).map(|m| m.len()).unwrap_or(0);
        let started = Instant::now();
        let mut speed = 0.0f64;
        let mut last = (Instant::now(), start_size);
        let status = loop {
            if cancel.load(Ordering::SeqCst) { let _ = child.kill(); let _ = child.wait(); return Err(super::CANCELLED.into()); }
            match child.try_wait() {
                Ok(Some(s)) => break s,
                Ok(None) => {}
                Err(e) => return Err(e.to_string()),
            }
            std::thread::sleep(Duration::from_millis(500));
            let now = std::fs::metadata(part).map(|m| m.len()).unwrap_or(0);
            let dt = last.0.elapsed().as_secs_f64();
            if dt > 0.0 {
                let inst = now.saturating_sub(last.1) as f64 / dt;
                speed = if speed == 0.0 { inst } else { speed * 0.8 + inst * 0.2 };
                last = (Instant::now(), now);
            }
            let eta = total.filter(|_| speed > 1.0).map(|t| (t.saturating_sub(now) as f64 / speed) as u64);
            if started.elapsed() > Duration::from_millis(400) { progress(Got { done: now, total, speed, eta }); }
        };
        let mut err = String::new();
        if let Some(mut e) = child.stderr.take() { use std::io::Read; let _ = e.read_to_string(&mut err); }
        if status.success() {
            let got = std::fs::metadata(part).map(|m| m.len()).unwrap_or(0);
            if let Some(t) = total { if got != t && tries < 3 { continue; } }
            progress(Got { done: got, total, speed, eta: Some(0) });
            return Ok(());
        }
        let code = status.code().unwrap_or(-1);
        // 33: the server can't continue a download; 22 with 416: it's already complete or the file changed.
        if code == 33 || (code == 22 && err.contains("416")) {
            if let Some(t) = total { if std::fs::metadata(part).map(|m| m.len()).unwrap_or(0) == t { return Ok(()); } }
            let _ = std::fs::remove_file(part);
            if tries < 3 { continue; }
        }
        if matches!(code, 18 | 28 | 56 | 92) && tries < 4 { std::thread::sleep(Duration::from_secs(2)); continue; }
        return Err(plain_curl(code, err.trim(), url));
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn only_https() {
        assert!(super::allowed("https://releases.ubuntu.com/"));
        assert!(!super::allowed("http://releases.ubuntu.com/"));
        assert!(!super::allowed("ftp://x/"));
        assert!(!super::allowed("file:///etc/shadow"));
        assert!(super::get("file:///etc/passwd").is_err());
    }
}
