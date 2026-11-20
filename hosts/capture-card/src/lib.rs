//! Capture Card's workings, apart from its window (so `cargo test` checks them without a screen):
//!
//! * [`card`]: the capture cards plugged in (V4L2 through /sys and its ioctls: name, chip, USB speed, picture modes);
//! * [`watch`]: waits for a card to be plugged in or out (inotify on /dev, sleeping in the kernel: nothing polls);
//! * [`sound`]: the card's own USB microphone among PipeWire's sources (pactl), picked by itself;
//! * [`video`]: the picture, sound, screenshots and recording (GStreamer), the delay, no signal;
//! * [`library`]: Recordings & screenshots (the folders, names, dates, thumbnails kept in a cache, free space);
//! * [`prefs`]: your choices (red flag 9: only through settings_file.rs, opening writes nothing).
//!
//! CI has no capture card: HEARTH_CAPTURE_FAKE_CARD names a small JSON file that stands in for one (its name, chip,
//! speed and modes, and a JPEG the stand-in card sends as its MJPEG picture). Only CI sets it.

pub mod card;
pub mod library;
pub mod prefs;
pub mod sound;
pub mod video;
pub mod watch;

use std::path::PathBuf;
use std::time::{Duration, Instant};

/// The home folder.
pub fn home() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/tmp"))
}

/// "~/Videos/Capture Card" for a path in the home folder.
pub fn tilde(p: &std::path::Path) -> String {
    let h = home();
    match p.strip_prefix(&h) {
        Ok(r) if r.as_os_str().is_empty() => "~".into(),
        Ok(r) => format!("~/{}", r.display()),
        Err(_) => p.display().to_string(),
    }
}

/// Seconds since 1970.
pub fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// A local date and time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Local {
    pub y: i32,
    pub mo: u32,
    pub d: u32,
    pub h: u32,
    pub mi: u32,
    pub s: u32,
    /// 0 = Sunday.
    pub wd: u32,
    /// Days since 1970 in local time (for "Today", "Yesterday").
    pub day: i64,
}

pub fn local(t: u64) -> Local {
    let tt = t as libc::time_t;
    // SAFETY: localtime_r fills the struct we give it.
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    unsafe { libc::localtime_r(&tt, &mut tm) };
    let off = tm.tm_gmtoff;
    Local { y: tm.tm_year + 1900, mo: tm.tm_mon as u32 + 1, d: tm.tm_mday as u32, h: tm.tm_hour as u32, mi: tm.tm_min as u32, s: tm.tm_sec as u32, wd: tm.tm_wday as u32, day: (t as i64 + off).div_euclid(86400) }
}

/// "Capture 2026-11-20 03-58-21": a new screenshot's or recording's name.
pub fn stamp_name(t: u64) -> String {
    let l = local(t);
    format!("Capture {:04}-{:02}-{:02} {:02}-{:02}-{:02}", l.y, l.mo, l.d, l.h, l.mi, l.s)
}

/// "3:41 AM".
pub fn clock12(l: &Local) -> String {
    let h = if l.h % 12 == 0 { 12 } else { l.h % 12 };
    format!("{h}:{:02} {}", l.mi, if l.h < 12 { "AM" } else { "PM" })
}

/// "Today, 3:41 AM", "Yesterday, 5:22 AM", "Wed, 4:47 AM", "Nov 2, 4:47 AM".
pub fn when(t: u64, now: u64) -> String {
    let (l, n) = (local(t), local(now));
    let days = n.day - l.day;
    const WD: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    const MO: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let d = match days {
        0 => "Today".to_string(),
        1 => "Yesterday".to_string(),
        2..=6 => WD[l.wd as usize % 7].to_string(),
        _ => format!("{} {}", MO[(l.mo as usize + 11) % 12], l.d),
    };
    format!("{d}, {}", clock12(&l))
}

/// "12:48", "1:02:03".
pub fn length(secs: u64) -> String {
    if secs >= 3600 {
        format!("{}:{:02}:{:02}", secs / 3600, secs / 60 % 60, secs % 60)
    } else {
        format!("{}:{:02}", secs / 60, secs % 60)
    }
}

/// "3.4 GB", "3.1 MB".
pub fn size(b: u64) -> String {
    let f = b as f64;
    if f >= 1e9 {
        format!("{:.1} GB", f / 1e9)
    } else if f >= 1e6 {
        format!("{:.1} MB", f / 1e6)
    } else {
        format!("{} KB", (f / 1e3).round() as u64)
    }
}

/// Free space where a folder is (statvfs), in bytes.
pub fn free_space(p: &std::path::Path) -> Option<u64> {
    let mut q = p.to_path_buf();
    while !q.exists() {
        q = q.parent()?.to_path_buf();
    }
    let c = std::ffi::CString::new(q.as_os_str().as_encoded_bytes()).ok()?;
    // SAFETY: statvfs fills the struct we give it.
    let mut s: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statvfs(c.as_ptr(), &mut s) } != 0 {
        return None;
    }
    Some(s.f_bavail as u64 * s.f_frsize as u64)
}

/// "84 GB free".
pub fn free_words(p: &std::path::Path) -> String {
    match free_space(p) {
        Some(b) => format!("{} GB free", (b as f64 / 1e9).round() as u64),
        None => "free space unknown".into(),
    }
}

/// Runs a program with a time limit (seconds); its stdout, or None when it failed or took too long.
pub fn output(cmd: &[&str], secs: u64) -> Option<String> {
    use std::process::{Command, Stdio};
    let (p, args) = cmd.split_first()?;
    let mut child = Command::new(p).args(args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().ok()?;
    let mut out = child.stdout.take()?;
    // Read on a thread so a full pipe can't hold the program up.
    let reader = std::thread::spawn(move || {
        use std::io::Read;
        let mut s = String::new();
        let _ = out.read_to_string(&mut s);
        s
    });
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(st)) => {
                let s = reader.join().unwrap_or_default();
                return st.success().then_some(s);
            }
            Ok(None) if start.elapsed() < Duration::from_secs(secs) => std::thread::sleep(Duration::from_millis(15)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
}

/// Runs a program with a time limit and stdin from a file (wl-copy < picture.png); whether it worked.
pub fn run_with_input(cmd: &[&str], input: &std::path::Path, secs: u64) -> bool {
    use std::process::{Command, Stdio};
    let Some((p, args)) = cmd.split_first() else { return false };
    let Ok(f) = std::fs::File::open(input) else { return false };
    let Ok(mut child) = Command::new(p).args(args).stdin(Stdio::from(f)).stdout(Stdio::null()).stderr(Stdio::null()).spawn() else { return false };
    let start = Instant::now();
    loop {
        match child.try_wait() {
            // wl-copy forks to serve the clipboard and its first process ends: that's success.
            Ok(Some(st)) => return st.success(),
            Ok(None) if start.elapsed() < Duration::from_secs(secs) => std::thread::sleep(Duration::from_millis(15)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return false;
            }
        }
    }
}

/// Is a program on PATH?
pub fn have(prog: &str) -> bool {
    std::env::var_os("PATH").map(|p| std::env::split_paths(&p).any(|d| d.join(prog).is_file())).unwrap_or(false)
}

/// Writes a file atomically (a cache: thumbnails' list). Settings never go through here: they use settings_file.rs.
pub fn write_atomic(p: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(d) = p.parent() {
        std::fs::create_dir_all(d)?;
    }
    let tmp = p.with_extension(format!("tmp{}", std::process::id()));
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, p)
}

/// The cache folder (~/.cache/hearth/capture-card).
pub fn cache_dir() -> PathBuf {
    std::env::var_os("XDG_CACHE_HOME").filter(|s| !s.is_empty()).map(PathBuf::from).unwrap_or_else(|| home().join(".cache")).join("hearth/capture-card")
}

/// A line for the log (stderr).
pub fn log(s: &str) {
    eprintln!("hearth-capture-card: {s}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_for_sizes_lengths_and_times() {
        assert_eq!(length(768), "12:48");
        assert_eq!(length(91), "1:31");
        assert_eq!(length(3723), "1:02:03");
        assert_eq!(size(3_400_000_000), "3.4 GB");
        assert_eq!(size(3_100_000), "3.1 MB");
        let l = Local { y: 2026, mo: 11, d: 20, h: 3, mi: 41, s: 7, wd: 5, day: 0 };
        assert_eq!(clock12(&l), "3:41 AM");
        assert_eq!(clock12(&Local { h: 15, ..l }), "3:41 PM");
        assert_eq!(clock12(&Local { h: 0, ..l }), "12:41 AM");
        let t = now();
        assert!(when(t, t).starts_with("Today, "));
        assert!(when(t - 86400, t).starts_with("Yesterday, "));
        assert!(when(t - 40 * 86400, t).contains(' '));
        assert!(stamp_name(t).starts_with("Capture 20"));
        assert_eq!(tilde(&home().join("Videos/Capture Card")), "~/Videos/Capture Card");
    }

    #[test]
    fn programs_run_with_a_time_limit() {
        assert_eq!(output(&["echo", "hi"], 5).as_deref(), Some("hi\n"));
        assert!(output(&["sleep", "5"], 1).is_none());
        assert!(output(&["no-such-program-here"], 1).is_none());
    }
}
