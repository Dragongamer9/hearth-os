//! Hearth Link: a small service on every Hearth device (`hearth-link serve`, the systemd user unit
//! hearth-link.service). Your devices pair once, with a short code, and then:
//!
//! 1. keep your Hearth data in sync between them (no Hearth server in the middle): favorites, hidden games,
//!    collections, play time (each device's own minutes, added up), per-game settings, your profile and its
//!    pictures, appearance, controller layouts, Beacon's own games list, Quick Resume. Newest change wins per field.
//!    Store sign-ins are never copied (they're bound to the device, or rotate): a device only shows "sign in once on
//!    this device" for the stores you're signed in to elsewhere.
//! 2. copy games from one to another, fast: Steam (with its appmanifest, so Steam sees it installed), GOG, Epic,
//!    Beacon, Xbox games from Xodus Nexus, with the game's Proton/Wine prefix and Hearth's own settings for it.
//!
//! Everything goes over iroh (QUIC, TLS 1.3): each device's identity is its own key (state.rs), a connection is
//! only ever with the device whose key was pinned at pairing, so the data is encrypted end to end even when it goes
//! through a relay. On the same network devices find each other by a UDP announcement (discovery.rs) and connect
//! directly; over the internet iroh punches through NATs and falls back to a relay (n0's public ones by default, or
//! your own: Settings › Hearth Link), and the UI says so when a copy goes through one.
//!
//! - state.rs: this device's key, name and settings; the paired devices
//! - wire.rs: frames on the QUIC streams (JSON messages and the chunks of a copy)
//! Pairing is described for people in docs/HEARTH-LINK.md.
//!
//! - pair.rs: pairing with a code (SPAKE2, bound to both devices' keys): 6 digits on the same network, a
//!   10-character short code from somewhere else
//! - shortcode.rs: the short code (typo-proof alphabet, check character, the keys derived from it)
//! - lookup.rs: finding a device from its short code (a sealed, signed record on public pkarr relays)
//! - discovery.rs: finding Hearth devices on the same network (with their kind and edition)
//! - manifest.rs: a game's files, and checking every path that comes over the network
//! - games.rs: the games this device has (every store), and where a copied one goes
//! - xfer.rs: copying a game: chunked, content-addressed (BLAKE3), parallel streams, compression where it helps,
//!   resumable, verified while it's written
//! - sync.rs: the data that's kept in sync, and merging it
//! - storage.rs: Hearth Storage, a Hearth device as a home server for the others (shared folders, a game archive,
//!   backups of saves and screenshots, media, waking it from the network)
//! - cache.rs: Hearth Storage's caches: games kept once for every device, Hearth OS updates fetched once (with
//!   hold-back until one device has started a new version well)
//! - saves.rs: saves follow you (a game's newest save from the Hearth Storage server before it starts, back after)
//! - health.rs: every paired device's health on the server (battery, space, backups, version, last start), and
//!   removing a device you don't use anymore
//! - handoff.rs: "Continue on this device": a game running (or just quit) on one device carries on on another
//! - fleet.rs: update every device at once (each by its own boot-safety, hold-back, battery and "someone's playing"
//!   rules; never restarted for you)
//! - inputs.rs: controller layouts from your other devices (Hearth Input; pull only, checked here)
//! - ai.rs: local AI over Hearth Link (a device's own model server, shared with the paired devices; off by default)
//! - node.rs: the service: connections, the requests from other devices and from the UI
//! - ipc.rs: the UI's (and `hearth-link call`'s) way in: a Unix socket only this user can open

pub mod ai;
pub mod discovery;
pub mod fleet;
pub mod handoff;
pub mod games;
pub mod health;
pub mod inputs;
pub mod ipc;
pub mod lookup;
pub mod manifest;
pub mod node;
pub mod pair;
pub mod saves;
pub mod shortcode;
pub mod state;
pub mod storage;
pub mod bench;
pub mod cache;
pub mod sync;
pub mod wire;
pub mod xfer;

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

/// The two QUIC protocols: pairing (open to devices that aren't paired yet, only while a code is shown) and
/// everything else (paired devices only).
pub const ALPN_PAIR: &[u8] = b"hearth/link-pair/1";
pub const ALPN_LINK: &[u8] = b"hearth/link/1";
/// The UDP port of the QUIC endpoint (another port when it's taken) and of the announcements on the network.
pub const QUIC_PORT: u16 = 47740;
pub const DISCOVERY_PORT: u16 = 47741;

/// A lock that keeps working after a panic elsewhere.
pub fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> { m.lock().unwrap_or_else(|p| p.into_inner()) }

pub fn now_ms() -> u64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0) }

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

pub fn random_bytes<const N: usize>() -> [u8; N] {
    let mut b = [0u8; N];
    getrandom::getrandom(&mut b).expect("the system's random numbers");
    b
}

/// Hearth Link's own folder: $HEARTH_LINK_HOME, or link/ in Hearth's data folder (~/.local/share/nimbus/link).
pub fn home_dir() -> PathBuf {
    std::env::var_os("HEARTH_LINK_HOME").map(PathBuf::from).unwrap_or_else(|| hearth_core::paths::data_dir().join("link"))
}

/// The service's socket: $HEARTH_LINK_SOCKET, or hearth-link/link.sock in the user's runtime folder.
pub fn socket_path() -> PathBuf {
    if let Some(p) = std::env::var_os("HEARTH_LINK_SOCKET") { return PathBuf::from(p); }
    let run = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join(format!("hearth-link-{}", unsafe { libc::getuid() })));
    run.join("hearth-link").join("link.sock")
}

/// Writes a file all at once (temporary file, then rename).
pub fn write_atomic(path: &std::path::Path, bytes: &[u8]) -> Result<(), String> {
    hearth_core::paths::write_atomic(path, bytes).map_err(|e| format!("{}: {e}", path.display()))
}

/// Free bytes on the file system holding `path` (or its nearest existing parent).
pub fn free_space(path: &std::path::Path) -> Option<u64> {
    use std::os::unix::ffi::OsStrExt;
    let mut p = path.to_path_buf();
    while !p.exists() { if !p.pop() { return None; } }
    let c = std::ffi::CString::new(p.as_os_str().as_bytes()).ok()?;
    let mut s: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statvfs(c.as_ptr(), &mut s) } != 0 { return None; }
    Some(s.f_bavail as u64 * s.f_frsize as u64)
}

/// A game is running on this device (Hearth's bridge keeps this file while one is): sync waits, so nothing of
/// Hearth Link's competes with the game for the disk or the CPU.
pub fn game_running() -> bool {
    let f = std::env::var_os("HEARTH_LINK_PLAYING_FILE").map(PathBuf::from).unwrap_or_else(hearth_core::link::playing_file);
    // Left behind by a Hearth that didn't get to say the game ended: not believed after 12 hours.
    std::fs::metadata(&f).and_then(|m| m.modified()).ok().and_then(|t| t.elapsed().ok()).map(|age| age.as_secs() < 12 * 3600).unwrap_or(false)
}

/// Bytes as people read them: "1.4 GB".
pub fn human_bytes(n: u64) -> String {
    let units = ["B", "KB", "MB", "GB", "TB"];
    let mut v = n as f64;
    let mut i = 0;
    while v >= 1000.0 && i < units.len() - 1 { v /= 1000.0; i += 1; }
    if i == 0 { format!("{n} B") } else { format!("{v:.1} {}", units[i]) }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hex_both_ways() {
        assert_eq!(hex(&[0, 15, 255]), "000fff");
        assert_eq!(unhex("000fff"), Some(vec![0, 15, 255]));
        assert_eq!(unhex("0g"), None);
        assert_eq!(unhex("abc"), None);
    }
    #[test]
    fn sizes() {
        assert_eq!(human_bytes(12), "12 B");
        assert_eq!(human_bytes(1_400_000_000), "1.4 GB");
    }
    #[test]
    fn free_space_of_a_folder_that_isnt_there_yet() {
        let d = tempfile::tempdir().unwrap();
        assert!(free_space(&d.path().join("a/b/c")).unwrap() > 0);
    }
}
