//! Dates and times in words, in local time (libc's localtime_r; no time-zone database is carried).

const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
const LONG_MONTHS: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
const DAYS: [&str; 7] = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tm {
    pub year: i32,
    /// 0-11
    pub mon: usize,
    pub day: i32,
    pub hour: i32,
    pub min: i32,
    pub sec: i32,
    /// 0 = Sunday
    pub wday: usize,
}

pub fn now() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

pub fn local(ts: i64) -> Tm {
    unsafe {
        let t: libc::time_t = ts as libc::time_t;
        let mut tm: libc::tm = std::mem::zeroed();
        libc::localtime_r(&t, &mut tm);
        Tm {
            year: tm.tm_year + 1900,
            mon: tm.tm_mon.clamp(0, 11) as usize,
            day: tm.tm_mday,
            hour: tm.tm_hour,
            min: tm.tm_min,
            sec: tm.tm_sec,
            wday: tm.tm_wday.clamp(0, 6) as usize,
        }
    }
}

fn same_day(a: Tm, b: Tm) -> bool {
    a.year == b.year && a.mon == b.mon && a.day == b.day
}

/// 3:42 PM
pub fn clock(ts: i64) -> String {
    let t = local(ts);
    let h = if t.hour % 12 == 0 { 12 } else { t.hour % 12 };
    format!("{}:{:02} {}", h, t.min, if t.hour < 12 { "AM" } else { "PM" })
}

/// Nov 12
pub fn day(ts: i64) -> String {
    let t = local(ts);
    format!("{} {}", MONTHS[t.mon], t.day)
}

/// Today, 3:42 PM · Yesterday, 9:10 AM · Nov 12, 3:42 PM
pub fn when(ts: i64) -> String {
    if ts <= 0 {
        return String::new();
    }
    let t = local(ts);
    let n = local(now());
    let y = local(now() - 86_400);
    let d = if same_day(t, n) {
        "Today".to_string()
    } else if same_day(t, y) {
        "Yesterday".to_string()
    } else {
        day(ts)
    };
    format!("{d}, {}", clock(ts))
}

/// Wednesday, November 18, 2026 at 3:42 PM
pub fn long(ts: i64) -> String {
    let t = local(ts);
    format!("{}, {} {}, {} at {}", DAYS[t.wday], LONG_MONTHS[t.mon], t.day, t.year, clock(ts))
}

/// "2026-11-18" → (2026, 10, 18)
pub fn parse_date(s: &str) -> Option<(i32, usize, i32)> {
    let mut it = s.split('-');
    let y = it.next()?.parse().ok()?;
    let m: usize = it.next()?.parse().ok()?;
    let d = it.next()?.parse().ok()?;
    if !(1..=12).contains(&m) {
        return None;
    }
    Some((y, m - 1, d))
}

pub fn month(m: usize) -> &'static str {
    MONTHS[m.min(11)]
}

/// A duration in words: 1:02:03 (CPU time), 12 s, 3 min.
pub fn hms(secs: f64) -> String {
    let s = secs.max(0.0) as u64;
    format!("{}:{:02}:{:02}", s / 3600, (s / 60) % 60, s % 60)
}
