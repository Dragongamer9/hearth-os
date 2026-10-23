//! Hearth Calendar's events (paths::calendar_file) and subscribed calendars (an .ics address, refreshed every few
//! hours into paths::subs_dir; read-only here).
//!
//! { "events": [Event], "subs": [{"id", "name", "url", "color", "updated": ms, "error": ""}],
//!   "settings": {"weekStart": 1, "view": "month", "defaultReminder": 10} }
//!
//! Times are kept as the calendar wrote them: wall-clock "2026-10-22T09:30:00" plus a zone (an IANA name, "UTC",
//! or none: your own local time, so an event stays at 9:30 when you travel or the clocks change). All-day events
//! have dates ("2026-12-25", the end date not included). `occurrences` turns them, repeats included, into moments.

use crate::rrule;
use chrono::{Duration, Local, NaiveDate, NaiveDateTime, TimeZone};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const LOCAL: &str = "local";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Event {
    pub id: String,
    pub calendar: String,
    pub title: String,
    pub notes: String,
    pub location: String,
    pub all_day: bool,
    pub start: String,
    pub end: String,
    pub tz: Option<String>,
    pub rrule: Option<String>,
    pub exdates: Vec<String>,
    /// Minutes before the start (all-day events: before 9:00 on the day).
    pub reminders: Vec<i64>,
    pub color: String,
}

impl Event {
    pub fn blank(calendar: &str) -> Event { Event { calendar: calendar.to_string(), ..Default::default() } }
}

/// One showing of an event, as the page and the taskbar's calendar get it.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Occurrence {
    pub id: String,
    pub calendar: String,
    pub title: String,
    pub location: String,
    pub notes: String,
    pub all_day: bool,
    /// ms
    pub start: i64,
    pub end: i64,
    /// Local date of the start, and of the last day it covers ("2026-10-22").
    pub date: String,
    pub last_date: String,
    /// The occurrence's own start as the event writes it (to skip just this one).
    pub occ: String,
    pub recurring: bool,
    pub read_only: bool,
    pub color: String,
    pub reminders: Vec<i64>,
}

/// Wall-clock time in a zone to ms.
pub fn to_ms(t: NaiveDateTime, tz: Option<&str>) -> Option<i64> {
    match tz {
        Some("UTC") => Some(t.and_utc().timestamp_millis()),
        Some(z) if !z.is_empty() => {
            let zone: chrono_tz::Tz = z.parse().ok()?;
            zone.from_local_datetime(&t).earliest().or_else(|| zone.from_local_datetime(&(t + Duration::hours(1))).earliest()).map(|x| x.timestamp_millis())
        }
        _ => Local.from_local_datetime(&t).earliest().or_else(|| Local.from_local_datetime(&(t + Duration::hours(1))).earliest()).map(|x| x.timestamp_millis()),
    }
}

fn local_date(ms: i64) -> String {
    Local.timestamp_millis_opt(ms).single().map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default()
}

/// The showings of `events` that overlap [from, to) (ms), sorted by start.
pub fn occurrences(events: &[Event], from: i64, to: i64, read_only: &dyn Fn(&Event) -> bool, colors: &dyn Fn(&Event) -> String) -> Vec<Occurrence> {
    let mut out = Vec::new();
    // Windows in wall time, padded by a day and a half each side for time zones.
    let pad = 36 * 3600 * 1000;
    let (Some(wf), Some(wt)) = (chrono::DateTime::from_timestamp_millis(from - pad), chrono::DateTime::from_timestamp_millis(to + pad)) else { return out };
    let (wf, wt) = (wf.naive_utc(), wt.naive_utc());
    for e in events {
        let Some(start) = rrule::parse_when(&e.start) else { continue };
        let end = rrule::parse_when(&e.end).filter(|x| *x > start).unwrap_or(if e.all_day { start + Duration::days(1) } else { start });
        let length = end - start;
        let zone = if e.all_day { None } else { e.tz.as_deref() };
        let rule = e.rrule.as_deref().and_then(rrule::parse);
        let starts: Vec<NaiveDateTime> = match &rule {
            Some(r) => {
                let ex: Vec<NaiveDateTime> = e.exdates.iter().filter_map(|x| rrule::parse_when(x)).collect();
                // Look back by the event's length so a long one that started before the window still shows.
                rrule::expand(r, start, wf - length, wt, &ex, 2000)
            }
            None => vec![start],
        };
        for s in starts {
            let (Some(a), Some(b)) = (to_ms(s, zone), to_ms(s + length, zone)) else { continue };
            let b = b.max(a);
            let overlaps = if b == a { a >= from && a < to } else { a < to && b > from };
            if !overlaps { continue; }
            let last = if e.all_day { (s + length - Duration::days(1)).format("%Y-%m-%d").to_string() } else { local_date((b - 1).max(a)) };
            out.push(Occurrence {
                id: e.id.clone(),
                calendar: e.calendar.clone(),
                title: if e.title.trim().is_empty() { "(No title)".into() } else { e.title.clone() },
                location: e.location.clone(),
                notes: e.notes.clone(),
                all_day: e.all_day,
                start: a,
                end: b,
                date: if e.all_day { s.format("%Y-%m-%d").to_string() } else { local_date(a) },
                last_date: last,
                occ: if e.all_day { s.format("%Y-%m-%d").to_string() } else { s.format("%Y-%m-%dT%H:%M:%S").to_string() },
                recurring: rule.is_some(),
                read_only: read_only(e),
                color: colors(e),
                reminders: e.reminders.clone(),
            });
        }
    }
    out.sort_by(|x, y| (x.start, !x.all_day, &x.title).cmp(&(y.start, !y.all_day, &y.title)));
    out
}

/// When each reminder of an occurrence goes off (ms), with its minutes: before the start, or for all-day events
/// before 9:00 on the day.
pub fn reminder_times(o: &Occurrence) -> Vec<(i64, i64)> {
    let base = if o.all_day {
        NaiveDate::parse_from_str(&o.date, "%Y-%m-%d").ok().and_then(|d| d.and_hms_opt(9, 0, 0)).and_then(|t| to_ms(t, None)).unwrap_or(o.start)
    } else { o.start };
    o.reminders.iter().map(|m| (base - m * 60_000, *m)).collect()
}

// ---------------------------------------------------------------------------------------------- the file

pub fn load() -> Value {
    let mut v = crate::store::load(&crate::paths::calendar_file());
    if !v["events"].is_array() { v["events"] = json!([]); }
    if !v["subs"].is_array() { v["subs"] = json!([]); }
    if !v["settings"].is_object() { v["settings"] = json!({"weekStart": 1, "view": "month", "defaultReminder": 10}); }
    v
}

pub fn save(v: &Value) -> Result<(), String> { crate::store::save(&crate::paths::calendar_file(), v) }

pub fn own_events(v: &Value) -> Vec<Event> {
    v["events"].as_array().into_iter().flatten().filter_map(|e| serde_json::from_value(e.clone()).ok()).collect()
}

/// A subscribed calendar's events, from its last download.
pub fn sub_events(sub: &Value) -> Vec<Event> {
    let id = sub["id"].as_str().unwrap_or("");
    if id.is_empty() || id.contains(['/', '\\', '.']) { return vec![]; }
    let text = std::fs::read_to_string(crate::paths::subs_dir().join(format!("{id}.ics"))).unwrap_or_default();
    let color = sub["color"].as_str().unwrap_or("").to_string();
    crate::ics::parse(&text, id).into_iter().map(|mut e| { e.color = color.clone(); e }).collect()
}

/// Every event: yours, then each subscribed calendar's.
pub fn all_events(v: &Value) -> Vec<Event> {
    let mut ev = own_events(v);
    for s in v["subs"].as_array().into_iter().flatten() { if s["hidden"] != true { ev.extend(sub_events(s)); } }
    ev
}

/// Showings in [from, to) of every event.
pub fn range(v: &Value, from: i64, to: i64) -> Vec<Occurrence> {
    let ev = all_events(v);
    occurrences(&ev, from, to, &|e| e.calendar != LOCAL, &|e| e.color.clone())
}

/// Adds or replaces one of your events (an empty id makes a new one). Returns it.
pub fn put_event(v: &mut Value, mut e: Event) -> Result<Event, String> {
    if rrule::parse_when(&e.start).is_none() { return Err("The event needs a start date.".into()); }
    if let (Some(s), Some(en)) = (rrule::parse_when(&e.start), rrule::parse_when(&e.end)) {
        if en < s { return Err("The event ends before it starts.".into()); }
    }
    if let Some(r) = &e.rrule { if r.trim().is_empty() { e.rrule = None; } else if rrule::parse(r).is_none() { return Err("That repeat rule isn't one Calendar knows.".into()); } }
    if let Some(z) = &e.tz { if !z.is_empty() && z != "UTC" && z.parse::<chrono_tz::Tz>().is_err() { e.tz = None; } }
    e.calendar = LOCAL.into();
    e.title = e.title.trim().chars().take(500).collect();
    e.reminders.retain(|m| (0..=40_320).contains(m));
    e.reminders.sort();
    e.reminders.dedup();
    if e.id.is_empty() { e.id = format!("{}@hearth", crate::store::new_id("ev")); }
    let list = v["events"].as_array_mut().ok_or("damaged calendar file")?;
    let val = serde_json::to_value(&e).map_err(|x| x.to_string())?;
    match list.iter_mut().find(|x| x["id"] == e.id.as_str()) { Some(x) => *x = val, None => list.push(val) }
    Ok(e)
}

/// Deletes one of your events, or (with `occ`) only that showing of a repeating one.
pub fn delete_event(v: &mut Value, id: &str, occ: Option<&str>) -> bool {
    let Some(list) = v["events"].as_array_mut() else { return false };
    if let Some(o) = occ.filter(|o| !o.is_empty()) {
        if let Some(e) = list.iter_mut().find(|x| x["id"] == id) {
            if e["rrule"].is_string() {
                if !e["exdates"].is_array() { e["exdates"] = json!([]); }
                e["exdates"].as_array_mut().unwrap().push(json!(o));
                return true;
            }
        }
    }
    let n = list.len();
    list.retain(|x| x["id"] != id);
    list.len() != n
}

/// Adds the events of an .ics file to your calendar. Returns how many were new and how many replaced older copies.
pub fn import(v: &mut Value, text: &str) -> Result<(usize, usize), String> {
    let ev = crate::ics::parse(text, LOCAL);
    if ev.is_empty() { return Err("No events in that file.".into()); }
    let (mut added, mut updated) = (0, 0);
    let list = v["events"].as_array_mut().ok_or("damaged calendar file")?;
    for e in ev {
        let val = serde_json::to_value(&e).map_err(|x| x.to_string())?;
        match list.iter_mut().find(|x| x["id"] == e.id.as_str()) { Some(x) => { *x = val; updated += 1; } None => { list.push(val); added += 1; } }
    }
    Ok((added, updated))
}

/// Downloads a subscribed calendar. Returns how many events it has.
pub fn refresh_sub(sub: &mut Value) -> Result<usize, String> {
    let id = sub["id"].as_str().unwrap_or("").to_string();
    let mut url = sub["url"].as_str().unwrap_or("").trim().to_string();
    if let Some(rest) = url.strip_prefix("webcal://") { url = format!("https://{rest}"); }
    let r = crate::net::get(&url, 30).and_then(|text| {
        if !text.contains("BEGIN:VCALENDAR") { return Err("That address isn't a calendar (.ics) file.".into()); }
        let n = crate::ics::parse(&text, &id).len();
        crate::store::write_bytes(&crate::paths::subs_dir().join(format!("{id}.ics")), text.as_bytes())?;
        Ok(n)
    });
    match &r {
        Ok(_) => { sub["updated"] = json!(crate::now_ms()); sub["error"] = json!(""); }
        Err(e) => sub["error"] = json!(e),
    }
    r
}

pub const SUB_COLORS: [&str; 6] = ["#6fb7ff", "#9bd36a", "#c79bff", "#ffd166", "#ff7ab6", "#5fd3c6"];

/// Subscribes to a calendar at a web address (downloaded once now, then every few hours by the service).
pub fn subscribe(v: &mut Value, url: &str, name: &str) -> Result<Value, String> {
    let url = url.trim();
    let low = url.to_ascii_lowercase();
    if !(low.starts_with("https://") || low.starts_with("http://") || low.starts_with("webcal://")) { return Err("Type the calendar's web address (https:// or webcal://).".into()); }
    let subs = v["subs"].as_array().cloned().unwrap_or_default();
    if subs.iter().any(|s| s["url"] == url) { return Err("You're already subscribed to that calendar.".into()); }
    let id = crate::store::new_id("sub");
    let color = SUB_COLORS[subs.len() % SUB_COLORS.len()];
    let name = if name.trim().is_empty() { "Subscribed calendar" } else { name.trim() };
    let mut sub = json!({"id": id, "name": name, "url": url, "color": color, "updated": 0, "error": ""});
    refresh_sub(&mut sub)?;
    v["subs"].as_array_mut().ok_or("damaged calendar file")?.push(sub.clone());
    Ok(sub)
}

pub fn unsubscribe(v: &mut Value, id: &str) {
    if let Some(list) = v["subs"].as_array_mut() { list.retain(|s| s["id"] != id); }
    if !id.is_empty() && !id.contains(['/', '\\', '.']) { let _ = std::fs::remove_file(crate::paths::subs_dir().join(format!("{id}.ics"))); }
}

/// For the taskbar's calendar (`hearth-time --events FROM TO`, dates as YYYY-MM-DD, TO not included).
pub fn events_json(from: &str, to: &str) -> Result<Value, String> {
    let f = NaiveDate::parse_from_str(from, "%Y-%m-%d").map_err(|_| "FROM must be YYYY-MM-DD")?;
    let t = NaiveDate::parse_from_str(to, "%Y-%m-%d").map_err(|_| "TO must be YYYY-MM-DD")?;
    let (Some(a), Some(b)) = (f.and_hms_opt(0, 0, 0).and_then(|d| to_ms(d, None)), t.and_hms_opt(0, 0, 0).and_then(|d| to_ms(d, None))) else { return Err("bad dates".into()) };
    let occ = range(&load(), a, b.max(a));
    Ok(json!({"from": from, "to": to, "events": occ.iter().take(500).map(|o| json!({
        "title": o.title, "date": o.date, "lastDate": o.last_date, "allDay": o.all_day, "start": o.start, "end": o.end,
        "location": o.location, "color": o.color,
    })).collect::<Vec<_>>()}))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn ev(start: &str, end: &str) -> Event { Event { id: "x".into(), calendar: LOCAL.into(), title: "T".into(), start: start.into(), end: end.into(), ..Default::default() } }
    fn ms(s: &str, tz: Option<&str>) -> i64 { to_ms(rrule::parse_when(s).unwrap(), tz).unwrap() }

    #[test]
    fn single_and_all_day() {
        let e = ev("2026-10-22T09:00:00", "2026-10-22T10:00:00");
        let o = occurrences(&[e], ms("2026-10-22", None), ms("2026-10-23", None), &|_| false, &|_| String::new());
        assert_eq!(o.len(), 1);
        assert_eq!(o[0].end - o[0].start, 3_600_000);
        assert_eq!(o[0].date, "2026-10-22");
        let mut h = ev("2026-12-24", "2026-12-27");
        h.all_day = true;
        let o = occurrences(&[h], ms("2026-12-25", None), ms("2026-12-26", None), &|_| false, &|_| String::new());
        assert_eq!(o.len(), 1);
        assert_eq!((o[0].date.as_str(), o[0].last_date.as_str()), ("2026-12-24", "2026-12-26"));
    }

    #[test]
    fn repeating_in_a_zone() {
        // Weekly 9:00 in New York: stays 9:00 there across the change on 1 November 2026.
        let mut e = ev("2026-10-22T09:00:00", "2026-10-22T09:30:00");
        e.tz = Some("America/New_York".into());
        e.rrule = Some("FREQ=WEEKLY".into());
        e.exdates = vec!["2026-10-29T09:00:00".into()];
        let o = occurrences(&[e], ms("2026-10-20T00:00:00", Some("UTC")), ms("2026-11-10T00:00:00", Some("UTC")), &|_| false, &|_| String::new());
        let starts: Vec<i64> = o.iter().map(|x| x.start).collect();
        assert_eq!(starts, vec![ms("2026-10-22T13:00:00", Some("UTC")), ms("2026-11-05T14:00:00", Some("UTC"))]);
        assert!(o[0].recurring);
    }

    #[test]
    fn put_delete_import() {
        let mut v = json!({"events": [], "subs": []});
        let e = put_event(&mut v, Event { title: " Lunch ".into(), start: "2026-10-22T12:00:00".into(), end: "2026-10-22T13:00:00".into(), reminders: vec![10, 10, 5], ..Default::default() }).unwrap();
        assert_eq!(e.title, "Lunch");
        assert_eq!(e.reminders, vec![5, 10]);
        assert!(put_event(&mut v, Event { start: "2026-10-22T12:00:00".into(), end: "2026-10-22T11:00:00".into(), ..Default::default() }).is_err());
        assert_eq!(own_events(&v).len(), 1);
        let mut r = e.clone();
        r.rrule = Some("FREQ=DAILY".into());
        put_event(&mut v, r).unwrap();
        assert!(delete_event(&mut v, &e.id, Some("2026-10-23T12:00:00")));
        assert_eq!(own_events(&v)[0].exdates.len(), 1);
        assert!(delete_event(&mut v, &e.id, None));
        assert!(own_events(&v).is_empty());
        let (a, u) = import(&mut v, "BEGIN:VCALENDAR\nBEGIN:VEVENT\nUID:1\nDTSTART:20261022T100000\nSUMMARY:A\nEND:VEVENT\nEND:VCALENDAR\n").unwrap();
        assert_eq!((a, u), (1, 0));
        let (a, u) = import(&mut v, "BEGIN:VCALENDAR\nBEGIN:VEVENT\nUID:1\nDTSTART:20261022T110000\nSUMMARY:A2\nEND:VEVENT\nEND:VCALENDAR\n").unwrap();
        assert_eq!((a, u), (0, 1));
        assert!(import(&mut v, "nothing").is_err());
    }

    #[test]
    fn reminders() {
        let mut e = ev("2026-10-22T09:00:00", "2026-10-22T10:00:00");
        e.reminders = vec![15];
        let o = occurrences(&[e], ms("2026-10-22", None), ms("2026-10-23", None), &|_| false, &|_| String::new());
        assert_eq!(reminder_times(&o[0]), vec![(ms("2026-10-22T08:45:00", None), 15)]);
    }
}
