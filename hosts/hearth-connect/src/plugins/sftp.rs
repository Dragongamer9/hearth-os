//! Browse the phone's files (KDE Connect's "Filesystem expose"): Hearth asks the phone to start its SFTP server, the
//! phone answers (over the TLS link, so only the paired phone can) with a port, a user name and a one-time password,
//! and Hearth mounts it with sshfs at <runtime folder>/hearth/phones/<device> for Hearth Files. The password goes to
//! sshfs on its stdin, never on a command line, and only to the phone's own address. Unmounted when you press
//! Disconnect, when you forget the phone, or when you sign out (the runtime folder goes).

use crate::daemon::Daemon;
use crate::packet::Packet;
use serde_json::{json, Value};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Default)]
pub struct State {
    pub mounted: Option<String>,
    pub waiting: bool,
    pub error: Option<String>,
    /// The phone's storage as folders in the mount ({name, path}).
    pub folders: Vec<Value>,
}

impl State {
    pub fn json(&self) -> Value { json!({"mounted": self.mounted, "waiting": self.waiting, "error": self.error, "folders": self.folders}) }
}

pub fn mount_dir(id: &str) -> PathBuf {
    let base = crate::socket_path().parent().map(Path::to_path_buf).unwrap_or_else(std::env::temp_dir);
    base.join("phones").join(crate::daemon::safe_name(id))
}

pub fn browse(d: &Daemon, id: &str) -> Result<Value, String> {
    if let Some(m) = super::with(id, |s| s.sftp.mounted.clone()) { return Ok(json!({"path": m})); }
    if !crate::desktop::has("sshfs") { return Err("Browsing the phone needs sshfs, which isn't installed".into()); }
    d.send_body(id, "kdeconnect.sftp.request", json!({"startBrowsing": true}))?;
    super::with(id, |s| { s.sftp.waiting = true; s.sftp.error = None; });
    d.changed();
    Ok(json!({"waiting": true}))
}

/// A user name or folder the phone sent: nothing sshfs could read as an option or a second argument.
fn safe(s: &str) -> bool { !s.is_empty() && !s.starts_with('-') && s.len() < 512 && !s.chars().any(|c| c.is_control()) }

fn fail(d: &Arc<Daemon>, id: &str, e: String) {
    super::with(id, |s| { s.sftp.waiting = false; s.sftp.error = Some(e.clone()); });
    d.emit("browse-failed", json!({"device": id, "error": e}));
    d.changed();
}

pub fn packet(d: &Arc<Daemon>, id: &str, from: &str, p: &Packet) {
    if let Some(e) = p.str("errorMessage") { fail(d, id, format!("{from}: {e}")); return; }
    let Some((ip, _)) = d.link_of(id) else { return };
    let port = p.int("port").unwrap_or(0);
    let user = p.str("user").unwrap_or("").to_string();
    let pass = p.str("password").unwrap_or("").to_string();
    let root = p.str("path").unwrap_or("/").to_string();
    if !(1..=65535).contains(&port) || !safe(&user) || !user.chars().all(|c| c.is_ascii_alphanumeric() || "_-.".contains(c)) || !safe(&root) || !root.starts_with('/') || pass.contains('\n') {
        fail(d, id, "The phone sent something Hearth can't use to browse it".into());
        return;
    }
    let list = |k: &str| p.body.get(k).and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect::<Vec<_>>()).unwrap_or_default();
    let (paths, names) = (list("multiPaths"), list("pathNames"));
    let dir = mount_dir(id);
    let (d, id) = (d.clone(), id.to_string());
    std::thread::spawn(move || {
        let _ = std::fs::create_dir_all(&dir);
        unmount_dir(&dir);
        let host = if ip.is_ipv6() { format!("{user}@[{ip}]:{root}") } else { format!("{user}@{ip}:{root}") };
        let mut c = crate::desktop::cmd("sshfs");
        let port_s = port.to_string();
        c.arg(&host).arg(&dir).args(["-p", port_s.as_str(), "-o", "password_stdin", "-o", "StrictHostKeyChecking=no", "-o", "UserKnownHostsFile=/dev/null",
            "-o", "reconnect", "-o", "ServerAliveInterval=15", "-o", "HostKeyAlgorithms=+ssh-rsa", "-o", "PubkeyAcceptedKeyTypes=+ssh-rsa"])
            .stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::piped());
        let mut ch = match c.spawn() { Ok(ch) => ch, Err(e) => return fail(&d, &id, format!("Couldn't start sshfs: {e}")) };
        if let Some(mut i) = ch.stdin.take() { let _ = i.write_all(format!("{pass}\n").as_bytes()); }
        // sshfs stays in the background once it has mounted; the first one ends then.
        let t = Instant::now();
        let code = loop {
            match ch.try_wait() {
                Ok(Some(s)) => break s.code(),
                Ok(None) if t.elapsed() < Duration::from_secs(25) => std::thread::sleep(Duration::from_millis(100)),
                _ => { let _ = ch.kill(); break None; }
            }
        };
        if code != Some(0) {
            let mut err = String::new();
            if let Some(mut e) = ch.stderr.take() { use std::io::Read; let _ = e.read_to_string(&mut err); }
            return fail(&d, &id, format!("Couldn't open the phone's files{}", err.lines().last().map(|l| format!(": {l}")).unwrap_or_default()));
        }
        let shown = dir.to_string_lossy().to_string();
        let folders: Vec<Value> = paths.iter().enumerate().map(|(i, p)| {
            let rel = p.strip_prefix(root.as_str()).unwrap_or(p.as_str()).trim_start_matches('/');
            json!({"name": names.get(i).cloned().unwrap_or_else(|| p.clone()), "path": dir.join(rel).to_string_lossy()})
        }).collect();
        super::with(&id, |s| { s.sftp.mounted = Some(shown.clone()); s.sftp.waiting = false; s.sftp.error = None; s.sftp.folders = folders; });
        d.emit("browse-ready", json!({"device": id, "path": shown}));
        d.changed();
    });
}

fn unmount_dir(dir: &Path) {
    for prog in ["fusermount3", "fusermount"] {
        if crate::desktop::has(prog) { let _ = crate::desktop::cmd(prog).args(["-u", "-z"]).arg(dir).stdout(Stdio::null()).status(); return; }
    }
}

pub fn unmount(id: &str) {
    let was = super::with(id, |s| { s.sftp.waiting = false; s.sftp.folders.clear(); s.sftp.mounted.take() });
    if was.is_some() { unmount_dir(&mount_dir(id)); }
}
