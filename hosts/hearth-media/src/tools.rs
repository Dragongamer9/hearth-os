//! Finding and running programs. Everything that captures or encodes is an outside program (ffmpeg,
//! wf-recorder, gpu-screen-recorder, gst-launch-1.0...), so this is used everywhere.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// The full path of a program on $PATH (or the path itself when it has a slash and is there).
pub fn which(cmd: &str) -> Option<PathBuf> {
    if cmd.is_empty() { return None; }
    if cmd.contains('/') {
        let p = PathBuf::from(cmd);
        return if is_exec(&p) { Some(p) } else { None };
    }
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path).map(|d| d.join(cmd)).find(|p| is_exec(p))
}

fn is_exec(p: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        p.metadata().map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0).unwrap_or(false)
    }
    #[cfg(not(unix))]
    { p.is_file() }
}

pub fn have(cmd: &str) -> bool { which(cmd).is_some() }

/// What a program printed, and how it ended.
#[derive(Debug, Clone)]
pub struct Out { pub code: Option<i32>, pub stdout: String, pub stderr: String }
impl Out {
    pub fn ok(&self) -> bool { self.code == Some(0) }
}

/// Runs a program to the end (or until `timeout`, then it's killed and this is an error).
pub fn run(prog: &str, args: &[&str], timeout: Duration) -> Result<Out, String> {
    let mut c = Command::new(prog);
    c.args(args);
    run_cmd(c, timeout)
}

/// The same with a Command made by the caller (environment, working folder...).
pub fn run_cmd(mut c: Command, timeout: Duration) -> Result<Out, String> {
    let name = c.get_program().to_string_lossy().to_string();
    let mut child = c.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().map_err(|e| format!("{name}: {e}"))?;
    let so = child.stdout.take();
    let se = child.stderr.take();
    let t1 = std::thread::spawn(move || { let mut v = Vec::new(); if let Some(mut s) = so { let _ = s.read_to_end(&mut v); } v });
    let t2 = std::thread::spawn(move || { let mut v = Vec::new(); if let Some(mut s) = se { let _ = s.read_to_end(&mut v); } v });
    let until = Instant::now() + timeout;
    let code = loop {
        match child.try_wait() {
            Ok(Some(st)) => break st.code(),
            Ok(None) => {}
            Err(e) => return Err(format!("{name}: {e}")),
        }
        if Instant::now() > until {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("{name} took too long"));
        }
        std::thread::sleep(Duration::from_millis(15));
    };
    let stdout = String::from_utf8_lossy(&t1.join().unwrap_or_default()).to_string();
    let stderr = String::from_utf8_lossy(&t2.join().unwrap_or_default()).to_string();
    Ok(Out { code, stdout, stderr })
}

/// Its standard output when it worked, trimmed; None otherwise (or when it isn't installed).
pub fn output(prog: &str, args: &[&str]) -> Option<String> {
    if !have(prog) { return None; }
    let o = run(prog, args, Duration::from_secs(5)).ok()?;
    if o.ok() { Some(o.stdout.trim().to_string()) } else { None }
}

/// Starts a program in a session of its own, so it keeps going when whoever started it goes away (the window
/// closing, the shortcut's shell ending).
pub fn spawn_detached(c: &mut Command) -> std::io::Result<Child> {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        unsafe {
            c.pre_exec(|| {
                libc::setsid();
                Ok(())
            });
        }
    }
    c.spawn()
}

pub const SIGINT: i32 = 2;
pub const SIGKILL: i32 = 9;
pub const SIGUSR1: i32 = 10;
pub const SIGTERM: i32 = 15;

/// Sends a signal (the numbers above). False if there's no such process.
pub fn signal(pid: u32, sig: i32) -> bool {
    if pid == 0 { return false; }
    #[cfg(unix)]
    { unsafe { libc::kill(pid as i32, sig) == 0 } }
    #[cfg(not(unix))]
    { let _ = sig; false }
}

/// Whether a process is still running (a finished one nobody has collected yet doesn't count).
pub fn alive(pid: u32) -> bool {
    if pid == 0 || !signal(pid, 0) { return false; }
    match std::fs::read_to_string(format!("/proc/{pid}/stat")) {
        Ok(s) => s.rsplit_once(')').map(|(_, rest)| !rest.trim_start().starts_with('Z')).unwrap_or(true),
        Err(_) => true,
    }
}

/// Waits up to `timeout` for a child to end. True when it did.
pub fn wait_child(c: &mut Child, timeout: Duration) -> bool {
    let until = Instant::now() + timeout;
    loop {
        if let Ok(Some(_)) = c.try_wait() { return true; }
        if Instant::now() > until { return false; }
        std::thread::sleep(Duration::from_millis(30));
    }
}

/// Asks a child to finish (SIGINT, which makes ffmpeg, wf-recorder, gpu-screen-recorder and gst-launch -e close
/// their file properly), then kills it if it doesn't within `grace`.
pub fn stop_child(c: &mut Child, grace: Duration) -> bool {
    if let Ok(Some(_)) = c.try_wait() { return true; }
    signal(c.id(), SIGINT);
    if wait_child(c, grace) { return true; }
    signal(c.id(), SIGTERM);
    if wait_child(c, Duration::from_secs(2)) { return false; }
    let _ = c.kill();
    let _ = c.wait();
    false
}

/// Makes a named pipe (for one program's output to be another's input).
pub fn mkfifo(p: &Path) -> Result<(), String> {
    let _ = std::fs::remove_file(p);
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        let c = std::ffi::CString::new(p.as_os_str().as_bytes()).map_err(|e| e.to_string())?;
        if unsafe { libc::mkfifo(c.as_ptr(), 0o600) } != 0 { return Err(format!("Couldn't make {}: {}", p.display(), std::io::Error::last_os_error())); }
        Ok(())
    }
    #[cfg(not(unix))]
    { Err("named pipes need Linux".into()) }
}

/// The last `n` lines of a log file (to say why something stopped).
pub fn tail(p: &Path, n: usize) -> String {
    let s = std::fs::read_to_string(p).unwrap_or_default();
    let lines: Vec<&str> = s.lines().filter(|l| !l.trim().is_empty()).collect();
    lines[lines.len().saturating_sub(n)..].join("\n")
}

/// This user's id.
pub fn uid() -> u32 {
    #[cfg(unix)]
    { unsafe { libc::getuid() } }
    #[cfg(not(unix))]
    { 0 }
}

/// A random token of `n` bytes as hex (from the kernel's random source), for links only the app knows.
pub fn token(n: usize) -> String {
    use std::io::Read as _;
    let mut b = vec![0u8; n];
    let ok = std::fs::File::open("/dev/urandom").and_then(|mut f| f.read_exact(&mut b)).is_ok();
    if !ok {
        // Without it: the time and the process, stirred (never used for anything that matters more than a link).
        let mut x = crate::dirs::now_ms() ^ ((std::process::id() as u64) << 32);
        for v in b.iter_mut() { x ^= x << 13; x ^= x >> 7; x ^= x << 17; *v = x as u8; }
    }
    b.iter().map(|x| format!("{x:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn runs_things() {
        let o = run("sh", &["-c", "echo hi; echo err >&2; exit 3"], Duration::from_secs(5)).unwrap();
        assert_eq!(o.stdout.trim(), "hi");
        assert_eq!(o.stderr.trim(), "err");
        assert_eq!(o.code, Some(3));
        assert!(run("sh", &["-c", "sleep 5"], Duration::from_millis(200)).is_err());
        assert!(have("sh"));
        assert!(!have("surely-not-a-program-hearth"));
        assert_eq!(output("sh", &["-c", "printf ' x \n'"]).as_deref(), Some("x"));
    }
    #[test]
    fn processes() {
        assert!(alive(std::process::id()));
        let mut c = Command::new("sleep").arg("30").spawn().unwrap();
        let pid = c.id();
        assert!(alive(pid));
        assert!(stop_child(&mut c, Duration::from_secs(3)));
        assert!(!alive(pid));
    }
    #[test]
    fn tokens() {
        let a = token(8);
        assert_eq!(a.len(), 16);
        assert_ne!(a, token(8));
    }
    #[test]
    fn fifo() {
        let d = std::env::temp_dir().join(format!("hm-fifo-{}", std::process::id()));
        mkfifo(&d).unwrap();
        assert!(d.exists());
        let _ = std::fs::remove_file(&d);
    }
}
