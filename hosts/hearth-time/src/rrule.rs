//! Repeating events: the part of iCalendar's RRULE that calendars really use.
//!   FREQ=DAILY|WEEKLY|MONTHLY|YEARLY, INTERVAL, COUNT, UNTIL, BYDAY (MO,TU... and for monthly/yearly 2MO, -1FR),
//!   BYMONTHDAY (monthly: 1..31, -1 = last day), BYMONTH (yearly)
//! Anything else in a rule is ignored (the event still repeats, maybe more often than the other calendar shows).
//! Times are wall-clock times (NaiveDateTime): the caller turns them into moments in the event's time zone, so a
//! weekly 9:00 meeting stays at 9:00 across daylight saving changes.

use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, Weekday};

#[derive(Debug, Clone, PartialEq)]
pub enum Freq { Daily, Weekly, Monthly, Yearly }

#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    pub freq: Freq,
    pub interval: u32,
    pub count: Option<u32>,
    pub until: Option<NaiveDateTime>,
    /// (nth, weekday): nth 0 = every; 1 = first, -1 = last (monthly and yearly).
    pub byday: Vec<(i32, Weekday)>,
    pub bymonthday: Vec<i32>,
    pub bymonth: Vec<u32>,
}

pub fn weekday(s: &str) -> Option<Weekday> {
    Some(match s { "MO" => Weekday::Mon, "TU" => Weekday::Tue, "WE" => Weekday::Wed, "TH" => Weekday::Thu, "FR" => Weekday::Fri, "SA" => Weekday::Sat, "SU" => Weekday::Sun, _ => return None })
}
pub fn weekday_code(w: Weekday) -> &'static str {
    match w { Weekday::Mon => "MO", Weekday::Tue => "TU", Weekday::Wed => "WE", Weekday::Thu => "TH", Weekday::Fri => "FR", Weekday::Sat => "SA", Weekday::Sun => "SU" }
}

/// A date or date-time as iCalendar writes it (20261022, 20261022T093000, 20261022T093000Z) or as Hearth's files
/// do (2026-10-22, 2026-10-22T09:30).
pub fn parse_when(s: &str) -> Option<NaiveDateTime> {
    let s = s.trim().trim_end_matches('Z');
    for f in ["%Y%m%dT%H%M%S", "%Y-%m-%dT%H:%M:%S", "%Y-%m-%dT%H:%M", "%Y%m%dT%H%M"] {
        if let Ok(d) = NaiveDateTime::parse_from_str(s, f) { return Some(d); }
    }
    for f in ["%Y%m%d", "%Y-%m-%d"] {
        if let Ok(d) = NaiveDate::parse_from_str(s, f) { return d.and_hms_opt(0, 0, 0); }
    }
    None
}

pub fn parse(rule: &str) -> Option<Rule> {
    let mut r = Rule { freq: Freq::Daily, interval: 1, count: None, until: None, byday: vec![], bymonthday: vec![], bymonth: vec![] };
    let mut freq = None;
    for part in rule.trim().trim_start_matches("RRULE:").split(';') {
        let Some((k, v)) = part.split_once('=') else { continue };
        match k.trim().to_ascii_uppercase().as_str() {
            "FREQ" => freq = Some(match v.trim().to_ascii_uppercase().as_str() { "DAILY" => Freq::Daily, "WEEKLY" => Freq::Weekly, "MONTHLY" => Freq::Monthly, "YEARLY" => Freq::Yearly, _ => return None }),
            "INTERVAL" => r.interval = v.trim().parse().unwrap_or(1).max(1),
            "COUNT" => r.count = v.trim().parse().ok(),
            "UNTIL" => r.until = parse_when(v).map(|d| if v.trim().len() == 8 || !v.contains('T') { d + Duration::seconds(86_399) } else { d }),
            "BYDAY" => r.byday = v.split(',').filter_map(|d| {
                let d = d.trim().to_ascii_uppercase();
                if d.len() < 2 { return None; }
                let (n, w) = d.split_at(d.len() - 2);
                Some((if n.is_empty() || n == "+" { 0 } else { n.trim_start_matches('+').parse().ok()? }, weekday(w)?))
            }).collect(),
            "BYMONTHDAY" => r.bymonthday = v.split(',').filter_map(|d| d.trim().parse().ok()).filter(|d: &i32| *d != 0 && d.abs() <= 31).collect(),
            "BYMONTH" => r.bymonth = v.split(',').filter_map(|d| d.trim().parse().ok()).filter(|m: &u32| (1..=12).contains(m)).collect(),
            _ => {}
        }
    }
    r.freq = freq?;
    Some(r)
}

/// The rule written back as RRULE text (for export).
pub fn to_string(r: &Rule) -> String {
    let mut s = format!("FREQ={}", match r.freq { Freq::Daily => "DAILY", Freq::Weekly => "WEEKLY", Freq::Monthly => "MONTHLY", Freq::Yearly => "YEARLY" });
    if r.interval > 1 { s += &format!(";INTERVAL={}", r.interval); }
    if let Some(c) = r.count { s += &format!(";COUNT={c}"); }
    if let Some(u) = r.until { s += &format!(";UNTIL={}", u.format("%Y%m%dT%H%M%S")); }
    if !r.byday.is_empty() {
        s += ";BYDAY=";
        s += &r.byday.iter().map(|(n, w)| if *n == 0 { weekday_code(*w).to_string() } else { format!("{n}{}", weekday_code(*w)) }).collect::<Vec<_>>().join(",");
    }
    if !r.bymonthday.is_empty() { s += &format!(";BYMONTHDAY={}", r.bymonthday.iter().map(|d| d.to_string()).collect::<Vec<_>>().join(",")); }
    if !r.bymonth.is_empty() { s += &format!(";BYMONTH={}", r.bymonth.iter().map(|d| d.to_string()).collect::<Vec<_>>().join(",")); }
    s
}

fn days_in_month(y: i32, m: u32) -> u32 {
    let (ny, nm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
    NaiveDate::from_ymd_opt(ny, nm, 1).and_then(|d| d.pred_opt()).map(|d| d.day()).unwrap_or(28)
}

/// The nth (1.., or -1 = last) `w` in a month.
fn nth_weekday(y: i32, m: u32, n: i32, w: Weekday) -> Option<NaiveDate> {
    if n > 0 {
        let first = NaiveDate::from_ymd_opt(y, m, 1)?;
        let off = (7 + w.num_days_from_monday() as i64 - first.weekday().num_days_from_monday() as i64) % 7;
        let d = first + Duration::days(off + 7 * (n as i64 - 1));
        (d.month() == m).then_some(d)
    } else {
        let last = NaiveDate::from_ymd_opt(y, m, days_in_month(y, m))?;
        let off = (7 + last.weekday().num_days_from_monday() as i64 - w.num_days_from_monday() as i64) % 7;
        let d = last - Duration::days(off + 7 * (-n as i64 - 1));
        (d.month() == m).then_some(d)
    }
}

/// The days a month period gives (sorted).
fn month_days(r: &Rule, y: i32, m: u32, start_day: u32) -> Vec<NaiveDate> {
    let mut v: Vec<NaiveDate> = Vec::new();
    if !r.bymonthday.is_empty() {
        let dim = days_in_month(y, m) as i32;
        for d in &r.bymonthday {
            let day = if *d > 0 { *d } else { dim + 1 + *d };
            if day >= 1 && day <= dim { if let Some(x) = NaiveDate::from_ymd_opt(y, m, day as u32) { v.push(x); } }
        }
    } else if !r.byday.is_empty() {
        for (n, w) in &r.byday {
            if *n != 0 { if let Some(d) = nth_weekday(y, m, *n, *w) { v.push(d); } }
            else {
                let mut d = NaiveDate::from_ymd_opt(y, m, 1);
                while let Some(x) = d { if x.month() != m { break; } if x.weekday() == *w { v.push(x); } d = x.succ_opt(); }
            }
        }
    } else if let Some(x) = NaiveDate::from_ymd_opt(y, m, start_day) {
        // The 31st repeats only in months that have one (as other calendars do).
        v.push(x);
    }
    v.sort();
    v.dedup();
    v
}

/// The starts of a repeating event from `start` (wall time) that fall before `before`, skipping `exdates`.
/// Occurrences before `after` are counted (for COUNT) but not returned. At most `limit` are returned.
pub fn expand(r: &Rule, start: NaiveDateTime, after: NaiveDateTime, before: NaiveDateTime, exdates: &[NaiveDateTime], limit: usize) -> Vec<NaiveDateTime> {
    let mut out = Vec::new();
    let mut n = 0u32;
    let time = start.time();
    let step = r.interval.max(1) as i64;
    let mut push = |d: NaiveDateTime, out: &mut Vec<NaiveDateTime>| -> bool {
        // false: stop.
        if d < start { return true; }
        if let Some(u) = r.until { if d > u { return false; } }
        if d >= before { return false; }
        if let Some(c) = r.count { if n >= c { return false; } }
        n += 1;
        if d >= after && !exdates.contains(&d) { out.push(d); }
        out.len() < limit
    };
    // Periods are walked in order; one far beyond `before` ends it. A cap keeps a broken rule from running forever.
    for k in 0..200_000i64 {
        let days: Vec<NaiveDate> = match r.freq {
            Freq::Daily => {
                let d = start.date() + Duration::days(k * step);
                let ok = (r.byday.is_empty() || r.byday.iter().any(|(_, w)| *w == d.weekday()))
                    && (r.bymonth.is_empty() || r.bymonth.contains(&d.month()))
                    && (r.bymonthday.is_empty() || r.bymonthday.contains(&(d.day() as i32)));
                if ok { vec![d] } else { vec![] }
            }
            Freq::Weekly => {
                let week_start = start.date() - Duration::days(start.date().weekday().num_days_from_monday() as i64) + Duration::weeks(k * step);
                let mut wd: Vec<Weekday> = r.byday.iter().map(|(_, w)| *w).collect();
                if wd.is_empty() { wd.push(start.date().weekday()); }
                let mut v: Vec<NaiveDate> = wd.iter().map(|w| week_start + Duration::days(w.num_days_from_monday() as i64)).collect();
                v.sort();
                v.dedup();
                v
            }
            Freq::Monthly => {
                let total = start.date().year() as i64 * 12 + start.date().month0() as i64 + k * step;
                let (y, m) = ((total.div_euclid(12)) as i32, (total.rem_euclid(12)) as u32 + 1);
                month_days(r, y, m, start.date().day())
            }
            Freq::Yearly => {
                let y = start.date().year() + (k * step) as i32;
                let months: Vec<u32> = if r.bymonth.is_empty() { vec![start.date().month()] } else { r.bymonth.clone() };
                let mut v = Vec::new();
                for m in months {
                    if r.byday.is_empty() && r.bymonthday.is_empty() {
                        if let Some(d) = NaiveDate::from_ymd_opt(y, m, start.date().day()) { v.push(d); }
                    } else {
                        v.extend(month_days(r, y, m, start.date().day()));
                    }
                }
                v.sort();
                v
            }
        };
        // Is this whole period past the end?
        let period_floor = match r.freq {
            Freq::Daily => start.date() + Duration::days(k * step),
            Freq::Weekly => start.date() - Duration::days(start.date().weekday().num_days_from_monday() as i64) + Duration::weeks(k * step),
            Freq::Monthly => {
                let total = start.date().year() as i64 * 12 + start.date().month0() as i64 + k * step;
                NaiveDate::from_ymd_opt(total.div_euclid(12) as i32, total.rem_euclid(12) as u32 + 1, 1).unwrap_or(start.date())
            }
            Freq::Yearly => NaiveDate::from_ymd_opt(start.date().year() + (k * step) as i32, 1, 1).unwrap_or(start.date()),
        };
        if period_floor.and_time(time) >= before && period_floor > start.date() { break; }
        if let Some(u) = r.until { if period_floor.and_hms_opt(0, 0, 0).map(|p| p > u).unwrap_or(false) { break; } }
        for d in days {
            if !push(d.and_time(time), &mut out) { return out; }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    fn dt(s: &str) -> NaiveDateTime { parse_when(s).unwrap() }

    #[test]
    fn parses_and_writes() {
        let r = parse("FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,WE;COUNT=10").unwrap();
        assert_eq!(r.freq, Freq::Weekly);
        assert_eq!(r.interval, 2);
        assert_eq!(r.byday, vec![(0, Weekday::Mon), (0, Weekday::Wed)]);
        assert_eq!(to_string(&r), "FREQ=WEEKLY;INTERVAL=2;COUNT=10;BYDAY=MO,WE");
        assert_eq!(parse("FREQ=MONTHLY;BYDAY=-1FR").unwrap().byday, vec![(-1, Weekday::Fri)]);
        assert!(parse("INTERVAL=2").is_none());
        assert_eq!(parse("FREQ=DAILY;UNTIL=20261031").unwrap().until, Some(dt("20261031T235959")));
    }

    #[test]
    fn daily_and_weekdays() {
        let r = parse("FREQ=DAILY;COUNT=3").unwrap();
        let v = expand(&r, dt("20261022T090000"), dt("20260101"), dt("20270101"), &[], 100);
        assert_eq!(v, vec![dt("20261022T090000"), dt("20261023T090000"), dt("20261024T090000")]);
        let wk = parse("FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR").unwrap();
        let v = expand(&wk, dt("20261022T080000"), dt("20261022"), dt("20261029"), &[], 100);
        // Thu 22, Fri 23, Mon 26, Tue 27, Wed 28.
        assert_eq!(v.len(), 5);
        assert_eq!(v[2], dt("20261026T080000"));
    }

    #[test]
    fn window_count_and_exdates() {
        let r = parse("FREQ=WEEKLY;COUNT=4").unwrap();
        // Counted from the start even when the window starts later.
        let v = expand(&r, dt("20261001T100000"), dt("20261010"), dt("20271010"), &[dt("20261022T100000")], 100);
        assert_eq!(v, vec![dt("20261015T100000")]);
        let every2 = parse("FREQ=WEEKLY;INTERVAL=2").unwrap();
        let v = expand(&every2, dt("20261001T100000"), dt("20261001"), dt("20261101"), &[], 100);
        assert_eq!(v, vec![dt("20261001T100000"), dt("20261015T100000"), dt("20261029T100000")]);
    }

    #[test]
    fn monthly_and_yearly() {
        let last_fri = parse("FREQ=MONTHLY;BYDAY=-1FR").unwrap();
        let v = expand(&last_fri, dt("20261030T170000"), dt("20261001"), dt("20270101"), &[], 100);
        assert_eq!(v, vec![dt("20261030T170000"), dt("20261127T170000"), dt("20261225T170000")]);
        let on31 = parse("FREQ=MONTHLY").unwrap();
        let v = expand(&on31, dt("20260131"), dt("20260101"), dt("20260601"), &[], 100);
        assert_eq!(v, vec![dt("20260131"), dt("20260331"), dt("20260531")]);
        let bday = parse("FREQ=YEARLY").unwrap();
        let v = expand(&bday, dt("20240229"), dt("20240101"), dt("20290101"), &[], 100);
        assert_eq!(v, vec![dt("20240229"), dt("20280229")]);
        let until = parse("FREQ=DAILY;UNTIL=20261024T090000Z").unwrap();
        assert_eq!(expand(&until, dt("20261022T090000"), dt("20200101"), dt("20300101"), &[], 100).len(), 3);
        // A rule with no end and a far window stops at the limit.
        assert_eq!(expand(&parse("FREQ=DAILY").unwrap(), dt("20000101"), dt("20000101"), dt("21000101"), &[], 50).len(), 50);
    }
}
