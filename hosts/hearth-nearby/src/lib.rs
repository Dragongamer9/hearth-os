//! Nearby Share: send and receive files with phones and computers on the same network. It speaks LocalSend's open
//! protocol (v2: https://github.com/localsend/protocol), so the LocalSend apps for Android, iOS, Windows, macOS and
//! Linux see Hearth and can send to it, and Hearth can send to them.
//!
//! - discovery.rs: multicast announcements (224.0.0.167:53317) and answers, plus the HTTP scan LocalSend falls back to
//! - server.rs: the HTTPS server (self-signed certificate, tls.rs) that receives: prepare-upload, upload, cancel
//! - send.rs: the client that sends to another device
//! - service.rs: what ties it together (devices seen, the incoming request, settings) for the window or the tests
//! - files.rs: safe names for received files, and the folder browser the window picks files with
//! - firewall.rs: opens the port only while Nearby Share listens, and only to the local network
//!
//! Safety: only local addresses are served or answered (is_local_ip); nothing is saved without the person saying
//! yes in the window unless they marked that device trusted, and "trusted" needs the device to prove who it is with
//! its certificate. Received files go to ~/Downloads/Nearby.

pub mod discovery;
pub mod files;
pub mod firewall;
pub mod proto;
pub mod send;
pub mod server;
pub mod service;
pub mod tls;

use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::Arc;

/// Something happened (a device was seen, a request came in, progress...): the window shows it.
pub type Emit = Arc<dyn Fn(&str, serde_json::Value) + Send + Sync>;

pub const DEFAULT_PORT: u16 = 53317;
pub const PROTOCOL_VERSION: &str = "2.1";

/// Lowercase hex.
pub fn hex(b: &[u8]) -> String {
    const H: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(b.len() * 2);
    for x in b {
        s.push(H[(x >> 4) as usize] as char);
        s.push(H[(x & 15) as usize] as char);
    }
    s
}

pub fn unhex(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 { return None; }
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(s.get(i..i + 2)?, 16).ok()).collect()
}

/// A random id of `n` bytes, as hex.
pub fn random_id(n: usize) -> String {
    use rand::RngCore;
    let mut b = vec![0u8; n];
    rand::thread_rng().fill_bytes(&mut b);
    hex(&b)
}

/// IPv4-mapped IPv6 addresses (::ffff:a.b.c.d) as the IPv4 address they are.
pub fn canonical(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(v6) => v6.to_ipv4_mapped().map(IpAddr::V4).unwrap_or(IpAddr::V6(v6)),
        v4 => v4,
    }
}

/// Is this address on a local network (private ranges, link-local, this machine)? Nothing else is ever served.
pub fn is_local_ip(ip: IpAddr) -> bool {
    match canonical(ip) {
        IpAddr::V4(a) => a.is_private() || a.is_link_local() || a.is_loopback(),
        IpAddr::V6(a) => {
            let s = a.segments()[0];
            a.is_loopback() || (s & 0xffc0) == 0xfe80 || (s & 0xfe00) == 0xfc00
        }
    }
}

fn home() -> PathBuf { std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/tmp")) }

/// Where Nearby Share keeps its identity (certificate) and settings. HEARTH_NEARBY_HOME overrides it (tests run two).
pub fn config_dir() -> PathBuf {
    if let Some(d) = std::env::var_os("HEARTH_NEARBY_HOME") { return PathBuf::from(d); }
    let base = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).filter(|p| p.is_absolute()).unwrap_or_else(|| home().join(".config"));
    base.join("hearth").join("nearby")
}

/// The Downloads folder (xdg-user-dirs' DOWNLOAD, else ~/Downloads).
pub fn downloads_dir() -> PathBuf {
    let h = home();
    let cfg = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| h.join(".config"));
    if let Ok(text) = std::fs::read_to_string(cfg.join("user-dirs.dirs")) {
        for line in text.lines() {
            if let Some(v) = line.trim().strip_prefix("XDG_DOWNLOAD_DIR=") {
                let v = v.trim().trim_matches('"').replace("$HOME", &h.to_string_lossy());
                let p = PathBuf::from(v);
                if p.is_absolute() && p != h { return p; }
            }
        }
    }
    h.join("Downloads")
}

/// Where received files go: ~/Downloads/Nearby. HEARTH_NEARBY_DEST overrides it (tests).
pub fn receive_dir() -> PathBuf {
    if let Some(d) = std::env::var_os("HEARTH_NEARBY_DEST") { return PathBuf::from(d); }
    downloads_dir().join("Nearby")
}

/// This computer's name, for the name other devices see.
pub fn host_name() -> String {
    let n = std::fs::read_to_string("/proc/sys/kernel/hostname").or_else(|_| std::fs::read_to_string("/etc/hostname"))
        .ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
        .or_else(|| std::env::var("COMPUTERNAME").ok())
        .unwrap_or_else(|| "Hearth".into());
    n
}

pub fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_round_trip() {
        assert_eq!(hex(&[0, 1, 0xab, 0xff]), "0001abff");
        assert_eq!(unhex("0001abff"), Some(vec![0, 1, 0xab, 0xff]));
        assert_eq!(unhex("0g"), None);
        assert_eq!(unhex("abc"), None);
        assert_eq!(random_id(16).len(), 32);
    }

    #[test]
    fn only_local_addresses() {
        for ok in ["192.168.1.20", "10.0.0.5", "172.16.3.4", "172.31.255.1", "169.254.10.1", "127.0.0.1", "fe80::1", "fd12:3456::1", "::1", "::ffff:192.168.0.2"] {
            assert!(is_local_ip(ok.parse().unwrap()), "{ok}");
        }
        for bad in ["8.8.8.8", "172.32.0.1", "100.64.0.1", "1.1.1.1", "2001:4860::8888", "::ffff:8.8.8.8"] {
            assert!(!is_local_ip(bad.parse().unwrap()), "{bad}");
        }
    }
}
