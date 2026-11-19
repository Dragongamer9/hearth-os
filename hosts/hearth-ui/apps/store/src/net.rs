//! The network, through curl (in every Hearth OS image): https only, time limits on everything, nothing secret sent
//! (no account, no token; GitHub's public release pages and hearthos.fun). Downloads can be cancelled. Each call runs
//! on its own short-lived thread (`off`); nothing polls.

use std::future::Future;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq)]
pub enum Fail {
    /// No connection (the name couldn't be looked up, nothing answered, it timed out).
    Offline,
    /// It answered, but not with the file (404 and the like).
    Missing,
    Other(String),
}

impl Fail {
    pub fn words(&self) -> String {
        match self {
            Fail::Offline => "You're offline (or GitHub and hearthos.fun can't be reached).".into(),
            Fail::Missing => "The file isn't there (no release yet?).".into(),
            Fail::Other(s) => s.clone(),
        }
    }
}

fn allowed(url: &str) -> bool {
    url.starts_with("https://") || (url.starts_with("file://") && crate::paths::fixture().is_some())
}

fn fail_of(code: i32, err: &str) -> Fail {
    match code {
        5 | 6 | 7 | 28 | 35 | 52 | 55 | 56 => Fail::Offline,
        22 => Fail::Missing,
        _ => Fail::Other(err.trim().lines().last().unwrap_or("it didn't work").trim_start_matches("curl: ").to_string()),
    }
}

/// A small file (a release's description, SHA256SUMS.txt, the list of apps): at most `max` bytes, `secs` seconds.
pub fn fetch(url: &str, max: u64, secs: u64) -> Result<Vec<u8>, Fail> {
    if !allowed(url) {
        return Err(Fail::Other("only https links are used".into()));
    }
    let out = Command::new("curl")
        .args(["-sS", "-L", "--fail", "--proto", "=https,file", "--connect-timeout", "10"])
        .args(["--max-time", &secs.to_string(), "--max-filesize", &max.to_string()])
        .args(["-H", "Accept: application/vnd.github+json", "-A", "Hearth-Store"])
        .arg(url)
        .stdin(Stdio::null())
        .output();
    match out {
        Ok(o) if o.status.success() => Ok(o.stdout),
        Ok(o) => Err(fail_of(o.status.code().unwrap_or(-1), &String::from_utf8_lossy(&o.stderr))),
        Err(e) => Err(Fail::Other(format!("curl: {e}"))),
    }
}

/// A download that's running: Cancel stops it.
#[derive(Clone, Default)]
pub struct Cancel {
    child: Arc<Mutex<Option<Child>>>,
    stop: Arc<AtomicBool>,
}

impl std::fmt::Debug for Cancel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Cancel({})", self.stop.load(Ordering::Relaxed))
    }
}

impl Cancel {
    pub fn cancel(&self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Ok(mut c) = self.child.lock() {
            if let Some(ch) = c.as_mut() {
                let _ = ch.kill();
            }
        }
    }
    pub fn cancelled(&self) -> bool {
        self.stop.load(Ordering::Relaxed)
    }
}

/// Downloads `url` to `dest` (blocking; run it with `off`). Stalls (under 1 KB/s for a minute) and anything over an
/// hour stop it. A cancelled or failed download leaves nothing behind.
pub fn download(url: &str, dest: &Path, c: &Cancel) -> Result<(), Fail> {
    if !allowed(url) {
        return Err(Fail::Other("only https links are used".into()));
    }
    if let Some(d) = dest.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let _ = std::fs::remove_file(dest);
    let child = Command::new("curl")
        .args(["-sS", "-L", "--fail", "--proto", "=https,file", "--connect-timeout", "20", "--speed-time", "60", "--speed-limit", "1024", "--max-time", "3600", "-A", "Hearth-Store", "-o"])
        .arg(dest)
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| Fail::Other(format!("curl: {e}")))?;
    if let Ok(mut g) = c.child.lock() {
        *g = Some(child);
    }
    if c.cancelled() {
        c.cancel();
    }
    // Wait without holding the lock for long (Cancel takes it to kill curl).
    let status = loop {
        let done = match c.child.lock() {
            Ok(mut g) => match g.as_mut() {
                Some(ch) => ch.try_wait().ok().flatten(),
                None => None,
            },
            Err(_) => None,
        };
        if let Some(s) = done {
            break s;
        }
        std::thread::sleep(std::time::Duration::from_millis(150));
    };
    let mut err = String::new();
    if let Ok(mut g) = c.child.lock() {
        if let Some(mut ch) = g.take() {
            if let Some(mut e) = ch.stderr.take() {
                use std::io::Read;
                let _ = e.read_to_string(&mut err);
            }
        }
    }
    if c.cancelled() {
        let _ = std::fs::remove_file(dest);
        return Err(Fail::Other("Cancelled".into()));
    }
    if !status.success() {
        let _ = std::fs::remove_file(dest);
        return Err(fail_of(status.code().unwrap_or(-1), &err));
    }
    Ok(())
}

/// Runs `f` on its own short-lived thread; the future is ready when it's done.
pub fn off<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> impl Future<Output = Option<T>> + Send + 'static {
    let (tx, rx) = futures::channel::oneshot::channel();
    let _ = std::thread::Builder::new().name("hearth-store-work".into()).spawn(move || {
        let _ = tx.send(f());
    });
    async move { rx.await.ok() }
}

/// A program with a time limit (`timeout`), its output if it ended well.
pub fn run_short(secs: u32, cmd: &[&str]) -> Option<String> {
    let o = Command::new("timeout").arg(secs.to_string()).args(cmd).stdin(Stdio::null()).stderr(Stdio::null()).output().ok()?;
    o.status.success().then(|| String::from_utf8_lossy(&o.stdout).to_string())
}

/// The connection is marked metered (NetworkManager: yes, or guessed yes: phone hotspots).
pub fn metered() -> bool {
    if let Some(f) = crate::paths::fixture() {
        return f.join("metered").exists();
    }
    run_short(3, &["busctl", "--system", "get-property", "org.freedesktop.NetworkManager", "/org/freedesktop/NetworkManager", "org.freedesktop.NetworkManager", "Metered"])
        .map(|o| matches!(o.split_whitespace().nth(1), Some("1") | Some("3")))
        .unwrap_or(false)
}

/// A game (or Game Mode) is running: gamescope, or Hearth's hearth-playing mark (as Hearth's other helpers ask).
pub fn game_running() -> bool {
    if let Ok(rd) = std::fs::read_dir("/proc") {
        for x in rd.flatten() {
            if x.file_name().to_str().map(|s| s.bytes().all(|b| b.is_ascii_digit())).unwrap_or(false) {
                if let Ok(c) = std::fs::read_to_string(x.path().join("comm")) {
                    if matches!(c.trim(), "gamescope" | "gamescope-wl") {
                        return true;
                    }
                }
            }
        }
    }
    std::env::var_os("XDG_RUNTIME_DIR").map(|r| Path::new(&r).join("hearth-playing").exists()).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curls_answers_in_words() {
        assert_eq!(fail_of(6, ""), Fail::Offline);
        assert_eq!(fail_of(28, ""), Fail::Offline);
        assert_eq!(fail_of(22, "curl: (22) The requested URL returned error: 404"), Fail::Missing);
        assert!(matches!(fail_of(3, "curl: (3) URL rejected"), Fail::Other(_)));
        assert!(!allowed("http://example.com/x"));
        assert!(allowed("https://github.com/x"));
    }

    #[test]
    fn a_download_from_a_local_file_in_ci_and_its_cancel() {
        let d = tempfile::tempdir().unwrap();
        std::env::set_var("HEARTH_STORE_FIXTURE", d.path());
        let src = d.path().join("app.AppImage");
        std::fs::write(&src, b"an app").unwrap();
        let dest = d.path().join("out/.app.download");
        let c = Cancel::default();
        if download(&format!("file://{}", src.display()), &dest, &c).is_ok() {
            assert_eq!(std::fs::read(&dest).unwrap(), b"an app");
        }
        let c = Cancel::default();
        c.cancel();
        assert!(download(&format!("file://{}", src.display()), &dest, &c).is_err());
        assert!(!dest.exists(), "a cancelled download leaves nothing");
        std::env::remove_var("HEARTH_STORE_FIXTURE");
    }
}
