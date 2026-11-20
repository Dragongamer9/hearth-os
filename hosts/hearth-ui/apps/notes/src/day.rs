//! Dates in US Central time, as the workspace's daily logs are named (log/YYYY-MM-DD.md, CLAUDE.md), worked out here
//! (the US daylight-saving rules: from the second Sunday in March, 2 AM, to the first Sunday in November, 2 AM), so
//! it doesn't matter which time zone the PC is set to.
//!
//! "Day starts at" (Settings › Today, 6:00 AM by default: the owner works overnight until 6 AM): until that time, Today
//! is still the day before, so a night's work stays in one log.

pub const MONTHS: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
pub const SHORT: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
pub const DAYS: [&str; 7] = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];

/// A day: (year, month 1-12, day 1-31).
pub type Ymd = (i32, u32, u32);

/// Days since 1970-01-01 (Howard Hinnant's days_from_civil).
pub fn days_from_civil(y: i32, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y as i64 - 1 } else { y as i64 };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m as i64 + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

pub fn civil_from_days(z: i64) -> Ymd {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    ((if m <= 2 { y + 1 } else { y }) as i32, m, d)
}

/// 0 = Sunday.
pub fn weekday(d: Ymd) -> usize {
    (days_from_civil(d.0, d.1, d.2) + 4).rem_euclid(7) as usize
}

pub fn days_in_month(y: i32, m: u32) -> u32 {
    let next = if m == 12 { days_from_civil(y + 1, 1, 1) } else { days_from_civil(y, m + 1, 1) };
    (next - days_from_civil(y, m, 1)) as u32
}

/// The day of the month of the n-th Sunday (1 = first).
fn nth_sunday(y: i32, m: u32, n: u32) -> u32 {
    let first = weekday((y, m, 1)) as u32;
    let first_sun = if first == 0 { 1 } else { 8 - first };
    first_sun + 7 * (n - 1)
}

/// US Central's offset from UTC at `utc` (seconds): -5 h in daylight time, else -6 h.
pub fn central_offset(utc: i64) -> i64 {
    let (y, _, _) = civil_from_days((utc - 6 * 3600).div_euclid(86400));
    let start = days_from_civil(y, 3, nth_sunday(y, 3, 2)) * 86400 + 8 * 3600; // 2 AM CST
    let end = days_from_civil(y, 11, nth_sunday(y, 11, 1)) * 86400 + 7 * 3600; // 2 AM CDT
    if utc >= start && utc < end { -5 * 3600 } else { -6 * 3600 }
}

/// A moment in US Central time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Central {
    pub day: Ymd,
    pub hour: u32,
    pub min: u32,
}

pub fn central(utc: i64) -> Central {
    let t = utc + central_offset(utc);
    let day = civil_from_days(t.div_euclid(86400));
    let s = t.rem_euclid(86400);
    Central { day, hour: (s / 3600) as u32, min: ((s / 60) % 60) as u32 }
}

/// Today's log's day: the Central date, still the day before until `day_start` (minutes after midnight).
pub fn log_day(utc: i64, day_start: u32) -> Ymd {
    let t = utc + central_offset(utc) - day_start.min(23 * 60 + 59) as i64 * 60;
    civil_from_days(t.div_euclid(86400))
}

pub fn ymd(d: Ymd) -> String {
    format!("{:04}-{:02}-{:02}", d.0, d.1, d.2)
}

/// "log/2026-11-20.md"
pub fn log_path(d: Ymd) -> String {
    format!("log/{}.md", ymd(d))
}

/// "2026-11-20" → (2026, 11, 20)
pub fn parse_ymd(s: &str) -> Option<Ymd> {
    let s = s.trim();
    if s.len() < 10 {
        return None;
    }
    let y = s[0..4].parse().ok()?;
    let m = s[5..7].parse().ok()?;
    let d = s[8..10].parse().ok()?;
    if &s[4..5] != "-" || &s[7..8] != "-" || !(1..=12).contains(&m) || d < 1 || d > days_in_month(y, m) {
        return None;
    }
    Some((y, m, d))
}

/// A log's day from its path ("log/2026-11-20.md").
pub fn log_of(rel: &str) -> Option<Ymd> {
    let name = rel.strip_prefix("log/")?.strip_suffix(".md")?;
    if name.len() != 10 {
        return None;
    }
    parse_ymd(name)
}

pub fn add_days(d: Ymd, n: i64) -> Ymd {
    civil_from_days(days_from_civil(d.0, d.1, d.2) + n)
}

pub fn between(a: Ymd, b: Ymd) -> i64 {
    days_from_civil(b.0, b.1, b.2) - days_from_civil(a.0, a.1, a.2)
}

/// "Friday, November 20, 2026"
pub fn long(d: Ymd) -> String {
    format!("{}, {} {}, {}", DAYS[weekday(d)], MONTHS[(d.1 - 1) as usize], d.2, d.0)
}

/// "Nov 20"
pub fn short(d: Ymd) -> String {
    format!("{} {}", SHORT[(d.1 - 1) as usize], d.2)
}

/// "3:04 AM"
pub fn clock(hour: u32, min: u32) -> String {
    let h = if hour % 12 == 0 { 12 } else { hour % 12 };
    format!("{}:{:02} {}", h, min, if hour < 12 { "AM" } else { "PM" })
}

/// "03:04" (the log's lines)
pub fn hhmm(c: Central) -> String {
    format!("{:02}:{:02}", c.hour, c.min)
}

/// "06:00" or "6:00 AM" → minutes after midnight.
pub fn parse_time(s: &str) -> Option<u32> {
    let t = s.trim().to_uppercase();
    let (body, pm) = if let Some(b) = t.strip_suffix("PM") { (b.trim().to_string(), Some(true)) } else if let Some(b) = t.strip_suffix("AM") { (b.trim().to_string(), Some(false)) } else { (t.clone(), None) };
    let (h, m) = body.split_once(':')?;
    let (mut h, m): (u32, u32) = (h.trim().parse().ok()?, m.trim().parse().ok()?);
    if m > 59 {
        return None;
    }
    match pm {
        Some(true) if h < 12 => h += 12,
        Some(false) if h == 12 => h = 0,
        _ => {}
    }
    if h > 23 {
        return None;
    }
    Some(h * 60 + m)
}

/// ISO week number (the calendar's W column).
pub fn iso_week(d: Ymd) -> u32 {
    let wd = (weekday(d) + 6) % 7; // Monday 0
    let thursday = add_days(d, 3 - wd as i64);
    let jan1 = days_from_civil(thursday.0, 1, 1);
    ((days_from_civil(thursday.0, thursday.1, thursday.2) - jan1) / 7 + 1) as u32
}

/// The calendar's six weeks (Monday first) for a month: each day and whether it's in the month.
pub fn month_grid(y: i32, m: u32) -> Vec<Vec<(Ymd, bool)>> {
    let first = (y, m, 1);
    let back = ((weekday(first) + 6) % 7) as i64;
    let start = add_days(first, -back);
    (0..6).map(|w| (0..7).map(|i| { let d = add_days(start, w * 7 + i); (d, d.1 == m) }).collect()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utc(y: i32, m: u32, d: u32, h: i64, mi: i64) -> i64 {
        days_from_civil(y, m, d) * 86400 + h * 3600 + mi * 60
    }

    #[test]
    fn central_time_and_daylight_saving() {
        assert_eq!(civil_from_days(days_from_civil(2026, 11, 20)), (2026, 11, 20));
        assert_eq!(weekday((2026, 11, 20)), 5, "Friday");
        // 3:04 AM CST on Nov 20, 2026 is 09:04 UTC (standard time since Nov 1).
        let t = utc(2026, 11, 20, 9, 4);
        assert_eq!(central(t), Central { day: (2026, 11, 20), hour: 3, min: 4 });
        // In July it's daylight time (-5 h).
        assert_eq!(central(utc(2026, 7, 4, 17, 0)).hour, 12);
        // The changes: 2026-03-08 and 2026-11-01.
        assert_eq!(central_offset(utc(2026, 3, 8, 7, 59)), -6 * 3600);
        assert_eq!(central_offset(utc(2026, 3, 8, 8, 0)), -5 * 3600);
        assert_eq!(central_offset(utc(2026, 11, 1, 6, 59)), -5 * 3600);
        assert_eq!(central_offset(utc(2026, 11, 1, 7, 0)), -6 * 3600);
    }

    #[test]
    fn the_log_day_starts_at_6_am() {
        let six = 6 * 60;
        // 3:04 AM on Nov 21 (Central): still Nov 20's log with the day starting at 6 AM, Nov 21's at midnight.
        let t = utc(2026, 11, 21, 9, 4);
        assert_eq!(log_day(t, six), (2026, 11, 20));
        assert_eq!(log_day(t, 0), (2026, 11, 21));
        // 6:00 AM sharp starts the new day; 5:59 doesn't.
        assert_eq!(log_day(utc(2026, 11, 21, 12, 0), six), (2026, 11, 21));
        assert_eq!(log_day(utc(2026, 11, 21, 11, 59), six), (2026, 11, 20));
        // Over a month's and a year's end.
        assert_eq!(log_day(utc(2027, 1, 1, 10, 0), six), (2026, 12, 31));
        assert_eq!(log_path(log_day(t, six)), "log/2026-11-20.md");
        assert_eq!(long((2026, 11, 20)), "Friday, November 20, 2026");
        assert_eq!(clock(3, 4), "3:04 AM");
        assert_eq!(clock(0, 30), "12:30 AM");
        assert_eq!(parse_time("06:00"), Some(360));
        assert_eq!(parse_time("6:30 PM"), Some(18 * 60 + 30));
        assert_eq!(parse_time("25:00"), None);
        assert_eq!(log_of("log/2026-11-19.md"), Some((2026, 11, 19)));
        assert_eq!(log_of("log/notes.md"), None);
    }

    #[test]
    fn the_calendar() {
        assert_eq!(iso_week((2026, 11, 20)), 47);
        assert_eq!(iso_week((2026, 10, 26)), 44);
        let g = month_grid(2026, 11);
        assert_eq!(g[0][0], ((2026, 10, 26), false));
        assert_eq!(g[0][6], ((2026, 11, 1), true));
        assert_eq!(g[5][0], ((2026, 11, 30), true));
        assert_eq!(days_in_month(2028, 2), 29);
    }
}
