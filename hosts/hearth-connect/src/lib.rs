//! Hearth Connect: a phone linked to Hearth OS. It speaks KDE Connect's open protocol (version 8, the one the
//! KDE Connect apps for Android and iOS and the KDE Connect desktop use), implemented here rather than by shipping
//! KDE's daemon (kdeconnectd): see docs/HEARTH-CONNECT.md for why (in short: the phone-as-touchpad needs an input
//! path that works in Game Mode's gamescope and Desktop Mode's labwc, which kdeconnectd doesn't have there).
//!
//! It runs as a small background service for the signed-in person (`hearth-connect --daemon`, the systemd user unit
//! hearth-connect.service), so the phone stays linked whether or not the window is open and in both modes. The
//! window (`hearth-connect`) talks to it over a Unix socket only this user can open (ipc.rs).
//!
//! - identity.rs: this PC's device id and certificate; the pairing verification key
//! - packet.rs: the packets (one JSON object per line) and what this PC says it can do
//! - lan.rs: finding phones (UDP broadcast on 1716), the TCP + TLS links, files (payloads) both ways
//! - daemon.rs: devices, pairing, and what each kind of packet does (the "plugins")
//! - input.rs: the phone as touchpad and keyboard (a virtual mouse and keyboard through uinput)
//! - desktop.rs: notifications, media players (MPRIS), clipboard, battery, sounds
//! - ipc.rs: the window's connection to the service
//!
//! Safety: only local addresses are answered; nothing but pairing happens with a device until the person accepts
//! the pairing on this PC (and checks the verification key matches the phone's); files from a paired phone wait in a
//! hidden folder until accepted, unless the person chose "accept files without asking" for that phone.

pub mod daemon;
pub mod desktop;
pub mod identity;
pub mod input;
pub mod ipc;
pub mod lan;
pub mod packet;

use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

/// A lock that keeps working after a panic elsewhere.
pub fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> { m.lock().unwrap_or_else(|p| p.into_inner()) }

pub fn hex(b: &[u8]) -> String {
    const H: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(b.len() * 2);
    for x in b { s.push(H[(x >> 4) as usize] as char); s.push(H[(x & 15) as usize] as char); }
    s
}

pub fn unhex(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 { return None; }
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(s.get(i..i + 2)?, 16).ok()).collect()
}

pub fn random_hex(n: usize) -> String {
    use rand::RngCore;
    let mut b = vec![0u8; n];
    rand::thread_rng().fill_bytes(&mut b);
    hex(&b)
}

pub fn canonical(ip: IpAddr) -> IpAddr {
    match ip { IpAddr::V6(v6) => v6.to_ipv4_mapped().map(IpAddr::V4).unwrap_or(IpAddr::V6(v6)), v4 => v4 }
}

/// Private ranges, link-local and this machine: the only addresses Hearth Connect talks to.
pub fn is_local_ip(ip: IpAddr) -> bool {
    match canonical(ip) {
        IpAddr::V4(a) => a.is_private() || a.is_link_local() || a.is_loopback(),
        IpAddr::V6(a) => { let s = a.segments()[0]; a.is_loopback() || (s & 0xffc0) == 0xfe80 || (s & 0xfe00) == 0xfc00 }
    }
}

fn home() -> PathBuf { std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/tmp")) }

/// Identity, paired phones and settings. HEARTH_CONNECT_HOME overrides it (tests).
pub fn config_dir() -> PathBuf {
    if let Some(d) = std::env::var_os("HEARTH_CONNECT_HOME") { return PathBuf::from(d); }
    let base = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).filter(|p| p.is_absolute()).unwrap_or_else(|| home().join(".config"));
    base.join("hearth").join("connect")
}

/// Where accepted files go: ~/Downloads/Hearth Connect. HEARTH_CONNECT_DEST overrides it (tests).
pub fn receive_dir() -> PathBuf {
    if let Some(d) = std::env::var_os("HEARTH_CONNECT_DEST") { return PathBuf::from(d); }
    let h = home();
    let cfg = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| h.join(".config"));
    let mut dl = h.join("Downloads");
    if let Ok(text) = std::fs::read_to_string(cfg.join("user-dirs.dirs")) {
        for line in text.lines() {
            if let Some(v) = line.trim().strip_prefix("XDG_DOWNLOAD_DIR=") {
                let p = PathBuf::from(v.trim().trim_matches('"').replace("$HOME", &h.to_string_lossy()));
                if p.is_absolute() && p != h { dl = p; }
            }
        }
    }
    dl.join("Hearth Connect")
}

/// The service's socket: <runtime dir>/hearth/connect.sock (HEARTH_CONNECT_SOCKET overrides it).
pub fn socket_path() -> PathBuf {
    if let Some(p) = std::env::var_os("HEARTH_CONNECT_SOCKET") { return PathBuf::from(p); }
    #[cfg(unix)]
    {
        let uid = unsafe { libc_getuid() };
        let run = PathBuf::from(format!("/run/user/{uid}"));
        let base = if run.is_dir() { run } else { std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap_or_else(std::env::temp_dir) };
        base.join("hearth").join("connect.sock")
    }
    #[cfg(not(unix))]
    { std::env::temp_dir().join("hearth-connect.sock") }
}

#[cfg(unix)]
extern "C" { #[link_name = "getuid"] fn libc_getuid() -> u32; }

pub fn host_name() -> String {
    std::fs::read_to_string("/proc/sys/kernel/hostname").or_else(|_| std::fs::read_to_string("/etc/hostname"))
        .ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).unwrap_or_else(|| "Hearth".into())
}

pub fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn helpers() {
        assert_eq!(unhex(&hex(&[1, 2, 250])), Some(vec![1, 2, 250]));
        assert!(is_local_ip("192.168.0.9".parse().unwrap()));
        assert!(!is_local_ip("100.64.1.1".parse().unwrap()));
        assert!(!is_local_ip("9.9.9.9".parse().unwrap()));
    }
}
