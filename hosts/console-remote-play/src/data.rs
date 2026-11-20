//! The monthly streaming data (the owner, Nov 20: counted on every network, home, work Wi-Fi and the phone's hotspot;
//! 20 GB, a warning at 90 %). Kept in ~/.local/share/hearth/console-remote-play/data.json through settings_file.rs:
//! {"period": "2026-11", "bytes": N, "warned": "2026-11"}. Written once a minute while streaming and when a stream
//! ends, never at rest. A new period starts on the day of the month in Settings (the 1st by default). At the limit it
//! asks before streaming more; it never stops a game by itself.

use hearth_ui::settings_file;
use serde_json::{json, Map, Value};
use std::path::PathBuf;

pub fn path() -> PathBuf {
    crate::home().join(".local/share/hearth/console-remote-play/data.json")
}

/// A local date: year, month (1-12), day (1-31).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Day {
    pub y: i32,
    pub m: u32,
    pub d: u32,
}

pub fn today() -> Day {
    let t = crate::now() as libc::time_t;
    // SAFETY: localtime_r fills the struct we give it.
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    unsafe { libc::localtime_r(&t, &mut tm) };
    Day { y: tm.tm_year + 1900, m: (tm.tm_mon + 1) as u32, d: tm.tm_mday as u32 }
}

pub fn days_in(y: i32, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 => 29,
        _ => 28,
    }
}

/// The period a day is in ("2026-11": the one that started in November), its day number and its length in days.
pub fn period(day: Day, reset: u32) -> (String, u32, u32) {
    let reset = reset.clamp(1, 28);
    let (py, pm) = if day.d >= reset { (day.y, day.m) } else if day.m == 1 { (day.y - 1, 12) } else { (day.y, day.m - 1) };
    let len = days_in(py, pm);
    let start_day = reset;
    let n = if day.d >= reset { day.d - start_day + 1 } else { days_in(py, pm) - start_day + 1 + day.d };
    (format!("{py:04}-{pm:02}"), n, len)
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Used {
    pub period: String,
    pub bytes: u64,
    pub warned: bool,
}

fn used_from(m: &Map<String, Value>, now_period: &str) -> Used {
    let p = m.get("period").and_then(Value::as_str).unwrap_or("");
    if p != now_period {
        return Used { period: now_period.into(), bytes: 0, warned: false };
    }
    Used { period: p.into(), bytes: m.get("bytes").and_then(Value::as_u64).unwrap_or(0), warned: m.get("warned").and_then(Value::as_str) == Some(p) }
}

pub fn load(reset: u32) -> Used {
    load_from(&path(), today(), reset)
}

pub fn load_from(f: &std::path::Path, day: Day, reset: u32) -> Used {
    let (p, _, _) = period(day, reset);
    used_from(&settings_file::read(f).map().unwrap_or_default(), &p)
}

/// Adds what was used since the last write (and marks the warning shown, once a period).
pub fn add(bytes: u64, warned: bool, reset: u32) -> Used {
    add_in(&path(), today(), reset, bytes, warned)
}

pub fn add_in(f: &std::path::Path, day: Day, reset: u32, bytes: u64, warned: bool) -> Used {
    let (p, _, _) = period(day, reset);
    if let Some(dir) = f.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let r = settings_file::update_json(f, |m| {
        let mut u = used_from(m, &p);
        u.bytes += bytes;
        u.warned |= warned;
        m.insert("period".into(), json!(u.period));
        m.insert("bytes".into(), json!(u.bytes));
        if u.warned {
            m.insert("warned".into(), json!(u.period));
        }
        u
    });
    r.map(|(_, u)| u).unwrap_or_default()
}

/// Where the count stands against the limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Fine,
    /// At the warning level: one toast a period.
    Warn,
    /// At the limit: it asks before a new stream (never stops one).
    Limit,
}

pub fn level(bytes: u64, limit_gb: u32, warn_pct: u32) -> Level {
    let limit = limit_gb as f64 * 1e9;
    let b = bytes as f64;
    if b >= limit {
        Level::Limit
    } else if b >= limit * warn_pct as f64 / 100.0 {
        Level::Warn
    } else {
        Level::Fine
    }
}

/// "14.6 GB" (decimal, as phone plans count).
pub fn gb(bytes: u64) -> String {
    let g = bytes as f64 / 1e9;
    if g >= 100.0 {
        format!("{g:.0} GB")
    } else {
        format!("{g:.1} GB")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn periods() {
        assert_eq!(period(Day { y: 2026, m: 11, d: 20 }, 1), ("2026-11".into(), 20, 30));
        assert_eq!(period(Day { y: 2026, m: 1, d: 3 }, 15), ("2025-12".into(), 20, 31));
        assert_eq!(period(Day { y: 2024, m: 2, d: 29 }, 1).2, 29);
    }

    #[test]
    fn counting_on_every_network_and_the_warning_once() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("data.json");
        let day = Day { y: 2026, m: 11, d: 20 };
        assert_eq!(load_from(&f, day, 1).bytes, 0);
        assert!(!f.exists(), "reading writes nothing");
        add_in(&f, day, 1, 14_600_000_000, false);
        let u = add_in(&f, day, 1, 3_500_000_000, true);
        assert_eq!(u.bytes, 18_100_000_000);
        assert!(u.warned);
        assert_eq!(level(u.bytes, 20, 90), Level::Warn);
        assert_eq!(gb(u.bytes), "18.1 GB");
        // A new month starts again at 0, and the warning can show again.
        let dec = load_from(&f, Day { y: 2026, m: 12, d: 1 }, 1);
        assert_eq!((dec.bytes, dec.warned), (0, false));
        assert_eq!(level(20_000_000_000, 20, 90), Level::Limit);
        assert_eq!(level(1, 20, 90), Level::Fine);
    }
}
