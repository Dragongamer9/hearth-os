//! Hearth Clock's file (paths::clock_file) and what in it is due to ring. The page owns the file's layout and
//! saves all of it; the service only reads it, except for the changes ringing makes (snooze, a timer that has
//! finished), which go through `after_ring` here.
//!
//! {
//!   "alarms": [{"id", "time": "07:30", "days": [1,2,3,4,5] (0 = Sunday; empty = once), "date": "2026-10-23" (once:
//!               the day it rings), "label", "enabled", "since": ms (rings only after this: set when it's switched
//!               on or changed, so an alarm made at 7:40 for 7:30 waits for tomorrow), "snooze": minutes,
//!               "snoozedUntil": ms | null}],
//!   "timers": [{"id", "label", "duration": ms, "endsAt": ms | null (running), "remaining": ms (paused)}],
//!   "stopwatch": {"startedAt": ms | null, "elapsed": ms, "laps": [ms]},
//!   "world": [{"tz": "Asia/Tokyo", "name": "Tokyo"}],
//!   "settings": {"ringMinutes": 10, "snooze": 10, "clock24": false}
//! }

use chrono::{Datelike, Duration, Local, NaiveDate, NaiveTime, TimeZone};
use serde_json::{json, Value};

/// How late an alarm may still ring (the PC was asleep or off at the time): 30 minutes.
pub const ALARM_GRACE_MS: i64 = 30 * 60 * 1000;
/// A timer that ran out while the PC was asleep still rings when it wakes, within 6 hours.
pub const TIMER_GRACE_MS: i64 = 6 * 60 * 60 * 1000;

#[derive(Debug, Clone, PartialEq)]
pub struct Due {
    /// "alarm" or "timer".
    pub kind: &'static str,
    pub id: String,
    pub label: String,
    /// When it was meant to ring (ms).
    pub at: i64,
    /// Unique for this ringing, so it rings once: "alarm:<id>:<at>".
    pub key: String,
}

fn parse_time(s: &str) -> Option<NaiveTime> { NaiveTime::parse_from_str(s.trim(), "%H:%M").ok() }

/// Local wall time to ms (the earlier of two in the autumn hour that repeats; a time skipped in spring rings an
/// hour later).
pub fn local_ms(d: NaiveDate, t: NaiveTime) -> Option<i64> {
    let ndt = d.and_time(t);
    Local.from_local_datetime(&ndt).earliest()
        .or_else(|| Local.from_local_datetime(&(ndt + Duration::hours(1))).earliest())
        .map(|x| x.timestamp_millis())
}

/// The last time `alarm` was set to ring at or before `now` (ms), if any.
pub fn last_ring(alarm: &Value, now: i64) -> Option<i64> {
    let t = parse_time(alarm["time"].as_str()?)?;
    let days: Vec<u32> = alarm["days"].as_array().map(|a| a.iter().filter_map(|d| d.as_u64()).map(|d| d as u32 % 7).collect()).unwrap_or_default();
    let today = Local.timestamp_millis_opt(now).single()?.date_naive();
    if days.is_empty() {
        let d = NaiveDate::parse_from_str(alarm["date"].as_str()?, "%Y-%m-%d").ok()?;
        return local_ms(d, t).filter(|ms| *ms <= now);
    }
    for back in 0..8 {
        let d = today - Duration::days(back);
        if !days.contains(&d.weekday().num_days_from_sunday()) { continue; }
        if let Some(ms) = local_ms(d, t) { if ms <= now { return Some(ms); } }
    }
    None
}

/// The next time `alarm` rings after `now` (for "rings in 7 h 20 min" and for the service's sleep).
pub fn next_ring(alarm: &Value, now: i64) -> Option<i64> {
    if alarm["enabled"] == false { return None; }
    let snoozed = alarm["snoozedUntil"].as_i64().filter(|s| *s > now);
    let t = parse_time(alarm["time"].as_str()?)?;
    let days: Vec<u32> = alarm["days"].as_array().map(|a| a.iter().filter_map(|d| d.as_u64()).map(|d| d as u32 % 7).collect()).unwrap_or_default();
    let today = Local.timestamp_millis_opt(now).single()?.date_naive();
    let regular = if days.is_empty() {
        NaiveDate::parse_from_str(alarm["date"].as_str().unwrap_or(""), "%Y-%m-%d").ok().and_then(|d| local_ms(d, t)).filter(|ms| *ms > now)
    } else {
        (0..8).filter_map(|i| {
            let d = today + Duration::days(i);
            if !days.contains(&d.weekday().num_days_from_sunday()) { return None; }
            local_ms(d, t).filter(|ms| *ms > now)
        }).next()
    };
    match (snoozed, regular) { (Some(a), Some(b)) => Some(a.min(b)), (a, b) => a.or(b) }
}

/// Everything that should be ringing at `now`, leaving out what `rang` says has rung already.
pub fn due(clock: &Value, now: i64, rang: &dyn Fn(&str) -> bool) -> Vec<Due> {
    let mut out = Vec::new();
    for a in clock["alarms"].as_array().into_iter().flatten() {
        if a["enabled"] == false { continue; }
        let id = a["id"].as_str().unwrap_or("").to_string();
        if id.is_empty() { continue; }
        let since = a["since"].as_i64().unwrap_or(0);
        let label = a["label"].as_str().unwrap_or("").to_string();
        let mut candidates = Vec::new();
        if let Some(s) = a["snoozedUntil"].as_i64() { candidates.push(s); }
        if let Some(at) = last_ring(a, now) { if at >= since { candidates.push(at); } }
        for at in candidates {
            if at > now || now - at > ALARM_GRACE_MS { continue; }
            let key = format!("alarm:{id}:{at}");
            if rang(&key) { continue; }
            out.push(Due { kind: "alarm", id: id.clone(), label: label.clone(), at, key });
        }
    }
    for t in clock["timers"].as_array().into_iter().flatten() {
        let Some(at) = t["endsAt"].as_i64() else { continue };
        let id = t["id"].as_str().unwrap_or("").to_string();
        if id.is_empty() || at > now || now - at > TIMER_GRACE_MS { continue; }
        let key = format!("timer:{id}:{at}");
        if rang(&key) { continue; }
        out.push(Due { kind: "timer", id, label: t["label"].as_str().unwrap_or("").to_string(), at, key });
    }
    out.sort_by_key(|d| d.at);
    out
}

/// The soonest moment anything in the clock rings after `now` (the service sleeps until then, at most a minute).
pub fn next_any(clock: &Value, now: i64) -> Option<i64> {
    let a = clock["alarms"].as_array().into_iter().flatten().filter_map(|a| next_ring(a, now));
    let t = clock["timers"].as_array().into_iter().flatten().filter_map(|t| t["endsAt"].as_i64()).filter(|e| *e > now);
    a.chain(t).min()
}

/// What ringing changes in the file. `action`: "snooze" (the alarm rings again in its snooze minutes) or
/// "dismiss" (a finished timer goes back to its full time; a snoozed alarm stops snoozing; a one-time alarm
/// switches off).
pub fn after_ring(clock: &mut Value, kind: &str, id: &str, action: &str, now: i64) {
    let snooze_default = clock["settings"]["snooze"].as_i64().unwrap_or(10).clamp(1, 60);
    if kind == "alarm" {
        if let Some(a) = clock["alarms"].as_array_mut().and_then(|v| v.iter_mut().find(|a| a["id"] == id)) {
            if action == "snooze" {
                let m = a["snooze"].as_i64().unwrap_or(snooze_default).clamp(1, 60);
                a["snoozedUntil"] = json!(now + m * 60_000);
            } else {
                a["snoozedUntil"] = Value::Null;
                let once = a["days"].as_array().map(|d| d.is_empty()).unwrap_or(true);
                if once { a["enabled"] = json!(false); }
            }
        }
    } else if kind == "timer" {
        if let Some(t) = clock["timers"].as_array_mut().and_then(|v| v.iter_mut().find(|t| t["id"] == id)) {
            if action == "snooze" {
                // "+1 min" on a finished timer.
                t["endsAt"] = json!(now + 60_000);
            } else {
                t["endsAt"] = Value::Null;
                t["remaining"] = t["duration"].clone();
            }
        }
    }
}

/// The file, with defaults filled in.
pub fn load() -> Value {
    let mut v = crate::store::load(&crate::paths::clock_file());
    for k in ["alarms", "timers", "world"] { if !v[k].is_array() { v[k] = json!([]); } }
    if !v["stopwatch"].is_object() { v["stopwatch"] = json!({"startedAt": null, "elapsed": 0, "laps": []}); }
    if !v["settings"].is_object() { v["settings"] = json!({"ringMinutes": 10, "snooze": 10}); }
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    fn at(y: i32, m: u32, d: u32, h: u32, min: u32) -> i64 { local_ms(NaiveDate::from_ymd_opt(y, m, d).unwrap(), NaiveTime::from_hms_opt(h, min, 0).unwrap()).unwrap() }

    #[test]
    fn weekday_alarm() {
        // 2026-10-22 is a Thursday.
        let a = json!({"id": "a", "time": "07:30", "days": [1, 2, 3, 4, 5], "enabled": true});
        assert_eq!(last_ring(&a, at(2026, 10, 22, 7, 31)), Some(at(2026, 10, 22, 7, 30)));
        assert_eq!(last_ring(&a, at(2026, 10, 22, 7, 29)), Some(at(2026, 10, 21, 7, 30)));
        // Saturday morning: Friday's was the last; Monday's is next.
        assert_eq!(last_ring(&a, at(2026, 10, 24, 9, 0)), Some(at(2026, 10, 23, 7, 30)));
        assert_eq!(next_ring(&a, at(2026, 10, 24, 9, 0)), Some(at(2026, 10, 26, 7, 30)));
    }

    #[test]
    fn rings_once_and_not_for_the_past() {
        let clock = json!({"alarms": [{"id": "a", "time": "07:30", "days": [4], "enabled": true, "since": at(2026, 10, 22, 7, 0)}]});
        let none = |_: &str| false;
        assert!(due(&clock, at(2026, 10, 22, 7, 29), &none).is_empty());
        let d = due(&clock, at(2026, 10, 22, 7, 30), &none);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].kind, "alarm");
        let key = d[0].key.clone();
        assert!(due(&clock, at(2026, 10, 22, 7, 31), &|k: &str| k == key).is_empty());
        // Far too late (the PC was off): it doesn't ring an hour later.
        assert!(due(&clock, at(2026, 10, 22, 8, 31), &none).is_empty());
        // Made at 7:40 for 7:30: waits for next week.
        let late = json!({"alarms": [{"id": "a", "time": "07:30", "days": [4], "enabled": true, "since": at(2026, 10, 22, 7, 40)}]});
        assert!(due(&late, at(2026, 10, 22, 7, 41), &none).is_empty());
        // Switched off: never.
        let off = json!({"alarms": [{"id": "a", "time": "07:30", "days": [4], "enabled": false}]});
        assert!(due(&off, at(2026, 10, 22, 7, 30), &none).is_empty());
    }

    #[test]
    fn one_time_alarm_and_snooze() {
        let mut clock = json!({"alarms": [{"id": "o", "time": "06:00", "days": [], "date": "2026-10-23", "enabled": true, "since": 0}], "settings": {"snooze": 9}});
        let none = |_: &str| false;
        assert!(due(&clock, at(2026, 10, 22, 6, 0), &none).is_empty());
        assert_eq!(due(&clock, at(2026, 10, 23, 6, 0), &none).len(), 1);
        after_ring(&mut clock, "alarm", "o", "snooze", at(2026, 10, 23, 6, 0));
        assert_eq!(clock["alarms"][0]["snoozedUntil"], json!(at(2026, 10, 23, 6, 9)));
        let first = format!("alarm:o:{}", at(2026, 10, 23, 6, 0));
        let d = due(&clock, at(2026, 10, 23, 6, 9), &|k: &str| k == first);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].at, at(2026, 10, 23, 6, 9));
        after_ring(&mut clock, "alarm", "o", "dismiss", at(2026, 10, 23, 6, 10));
        assert_eq!(clock["alarms"][0]["enabled"], false);
        assert_eq!(next_ring(&clock["alarms"][0], at(2026, 10, 23, 6, 10)), None);
    }

    #[test]
    fn timers() {
        let mut clock = json!({"timers": [
            {"id": "t1", "label": "Pasta", "duration": 600000, "endsAt": 1_000_000, "remaining": 0},
            {"id": "t2", "label": "Tea", "duration": 180000, "endsAt": 1_500_000},
            {"id": "t3", "label": "Paused", "duration": 60000, "endsAt": null, "remaining": 30000}]});
        let d = due(&clock, 1_600_000, &|_: &str| false);
        assert_eq!(d.iter().map(|x| x.id.as_str()).collect::<Vec<_>>(), vec!["t1", "t2"]);
        assert_eq!(next_any(&clock, 1_200_000), Some(1_500_000));
        after_ring(&mut clock, "timer", "t1", "dismiss", 1_600_000);
        assert!(clock["timers"][0]["endsAt"].is_null());
        assert_eq!(clock["timers"][0]["remaining"], 600000);
        after_ring(&mut clock, "timer", "t2", "snooze", 1_600_000);
        assert_eq!(clock["timers"][1]["endsAt"], 1_660_000);
    }
}
