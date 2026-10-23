//! iCalendar (.ics) files in and out: the events (VEVENT) with their title, notes, place, times (all-day, local,
//! UTC or in a named time zone), repeats (RRULE, EXDATE) and reminders (VALARM with a TRIGGER before the start).
//! What Hearth Calendar doesn't use (attendees, VTODO, VJOURNAL...) is skipped.

use crate::calendar::Event;

/// Lines as written, with folded lines (a line starting with a space or tab continues the one before) joined.
fn unfold(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for raw in text.split('\n') {
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        if (line.starts_with(' ') || line.starts_with('\t')) && !out.is_empty() {
            out.last_mut().unwrap().push_str(&line[1..]);
        } else {
            out.push(line.to_string());
        }
    }
    out
}

/// NAME;PARAM=x;PARAM2="y":VALUE into (NAME, [(PARAM, x)], VALUE).
fn split_line(line: &str) -> (String, Vec<(String, String)>, String) {
    // The value starts at the first ':' outside quotes.
    let mut in_q = false;
    let mut cut = line.len();
    for (i, c) in line.char_indices() {
        if c == '"' { in_q = !in_q; }
        if c == ':' && !in_q { cut = i; break; }
    }
    let (head, value) = (&line[..cut], if cut < line.len() { &line[cut + 1..] } else { "" });
    let mut parts = head.split(';');
    let name = parts.next().unwrap_or("").trim().to_ascii_uppercase();
    let params = parts.filter_map(|p| p.split_once('=').map(|(k, v)| (k.trim().to_ascii_uppercase(), v.trim().trim_matches('"').to_string()))).collect();
    (name, params, value.to_string())
}

pub fn unescape(s: &str) -> String {
    let mut o = String::new();
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c == '\\' {
            match it.next() { Some('n') | Some('N') => o.push('\n'), Some(x) => o.push(x), None => {} }
        } else { o.push(c); }
    }
    o
}

pub fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace(';', "\\;").replace(',', "\\,").replace("\r\n", "\\n").replace('\n', "\\n")
}

/// A DTSTART/DTEND value in Hearth's form: ("2026-10-22", all-day) or ("2026-10-22T09:30:00", timed), and its zone
/// ("UTC" for ...Z, the TZID, or None for floating local time).
fn when(params: &[(String, String)], value: &str) -> Option<(String, bool, Option<String>)> {
    let v = value.trim();
    let date_only = params.iter().any(|(k, x)| k == "VALUE" && x.eq_ignore_ascii_case("DATE")) || (v.len() == 8 && !v.contains('T'));
    let d = crate::rrule::parse_when(v)?;
    if date_only { return Some((d.format("%Y-%m-%d").to_string(), true, None)); }
    let tz = if v.ends_with('Z') { Some("UTC".to_string()) } else { params.iter().find(|(k, _)| k == "TZID").map(|(_, t)| map_tzid(t)) };
    Some((d.format("%Y-%m-%dT%H:%M:%S").to_string(), false, tz))
}

/// Time zone names as calendars write them: IANA names mostly; a few programs prefix them ("/mozilla.org/.../Europe/Berlin")
/// or use Windows names, of which the common ones are mapped.
pub fn map_tzid(t: &str) -> String {
    let t = t.trim().trim_matches('"');
    if t.parse::<chrono_tz::Tz>().is_ok() { return t.to_string(); }
    // A prefixed IANA name: try the last two parts, then the last three.
    let parts: Vec<&str> = t.split('/').filter(|p| !p.is_empty()).collect();
    for n in [2usize, 3, 1] {
        if parts.len() >= n {
            let cand = parts[parts.len() - n..].join("/");
            if cand.parse::<chrono_tz::Tz>().is_ok() { return cand; }
        }
    }
    let windows = [
        ("Pacific Standard Time", "America/Los_Angeles"), ("Mountain Standard Time", "America/Denver"), ("Central Standard Time", "America/Chicago"),
        ("Eastern Standard Time", "America/New_York"), ("GMT Standard Time", "Europe/London"), ("W. Europe Standard Time", "Europe/Berlin"),
        ("Romance Standard Time", "Europe/Paris"), ("Central Europe Standard Time", "Europe/Budapest"), ("E. South America Standard Time", "America/Sao_Paulo"),
        ("Tokyo Standard Time", "Asia/Tokyo"), ("China Standard Time", "Asia/Shanghai"), ("India Standard Time", "Asia/Kolkata"),
        ("AUS Eastern Standard Time", "Australia/Sydney"), ("UTC", "UTC"), ("Coordinated Universal Time", "UTC"),
    ];
    windows.iter().find(|(w, _)| w.eq_ignore_ascii_case(t)).map(|(_, i)| i.to_string()).unwrap_or_default()
}

/// ISO 8601 durations as used by TRIGGER and DURATION (-PT15M, P1D, PT1H30M, -P1W) in minutes.
pub fn duration_minutes(s: &str) -> Option<i64> {
    let s = s.trim();
    let (neg, rest) = if let Some(r) = s.strip_prefix('-') { (true, r) } else { (false, s.trim_start_matches('+')) };
    let rest = rest.strip_prefix('P')?;
    let mut total = 0i64;
    let mut num = String::new();
    let mut in_time = false;
    for c in rest.chars() {
        match c {
            'T' => in_time = true,
            '0'..='9' => num.push(c),
            'W' => { total += num.parse::<i64>().ok()? * 7 * 1440; num.clear(); }
            'D' => { total += num.parse::<i64>().ok()? * 1440; num.clear(); }
            'H' if in_time => { total += num.parse::<i64>().ok()? * 60; num.clear(); }
            'M' if in_time => { total += num.parse::<i64>().ok()?; num.clear(); }
            'S' if in_time => { num.clear(); }
            _ => return None,
        }
    }
    Some(if neg { -total } else { total })
}

/// The events in an .ics file, for calendar `calendar` (their ids are the file's UIDs; repeats of the same UID with a
/// RECURRENCE-ID, changed single occurrences, are skipped and the main event kept).
pub fn parse(text: &str, calendar: &str) -> Vec<Event> {
    let mut out: Vec<Event> = Vec::new();
    let mut cur: Option<Event> = None;
    let mut depth_alarm = false;
    let mut is_override = false;
    let mut duration: Option<i64> = None;
    for line in unfold(text) {
        if line.trim().is_empty() { continue; }
        let (name, params, value) = split_line(&line);
        match (name.as_str(), value.trim().to_ascii_uppercase().as_str()) {
            ("BEGIN", "VEVENT") => { cur = Some(Event::blank(calendar)); is_override = false; duration = None; continue; }
            ("BEGIN", "VALARM") => { depth_alarm = true; continue; }
            ("END", "VALARM") => { depth_alarm = false; continue; }
            ("END", "VEVENT") => {
                if let Some(mut e) = cur.take() {
                    if e.end.is_empty() {
                        e.end = match (duration, crate::rrule::parse_when(&e.start)) {
                            (Some(m), Some(s)) => { let x = s + chrono::Duration::minutes(m); if e.all_day { x.format("%Y-%m-%d").to_string() } else { x.format("%Y-%m-%dT%H:%M:%S").to_string() } }
                            (None, Some(s)) if e.all_day => (s + chrono::Duration::days(1)).format("%Y-%m-%d").to_string(),
                            _ => e.start.clone(),
                        };
                    }
                    if e.id.is_empty() { e.id = crate::store::new_id("ics"); }
                    if !is_override && !e.start.is_empty() && !out.iter().any(|o| o.id == e.id) { out.push(e); }
                }
                continue;
            }
            _ => {}
        }
        let Some(e) = cur.as_mut() else { continue };
        if depth_alarm {
            if name == "TRIGGER" && !params.iter().any(|(k, v)| k == "VALUE" && v.eq_ignore_ascii_case("DATE-TIME")) {
                if let Some(m) = duration_minutes(&value) { if m <= 0 && !e.reminders.contains(&-m) { e.reminders.push(-m); } }
            }
            continue;
        }
        match name.as_str() {
            "UID" => e.id = value.trim().to_string(),
            "SUMMARY" => e.title = unescape(&value),
            "DESCRIPTION" => e.notes = unescape(&value),
            "LOCATION" => e.location = unescape(&value),
            "DTSTART" => if let Some((s, all, tz)) = when(&params, &value) { e.start = s; e.all_day = all; e.tz = tz.filter(|t| !t.is_empty()); },
            "DTEND" => if let Some((s, _, _)) = when(&params, &value) { e.end = s; },
            "DURATION" => duration = duration_minutes(&value),
            "RRULE" => e.rrule = Some(value.trim().to_string()).filter(|r| crate::rrule::parse(r).is_some()),
            "EXDATE" => for v in value.split(',') { if let Some((s, _, _)) = when(&params, v) { e.exdates.push(s); } },
            "RECURRENCE-ID" => is_override = true,
            _ => {}
        }
    }
    out
}

/// Folds a line at 75 bytes (without splitting a character), as the format asks.
fn fold(line: &str) -> String {
    let mut out = String::new();
    let mut n = 0;
    for c in line.chars() {
        let l = c.len_utf8();
        if n + l > 75 { out.push_str("\r\n "); n = 1; }
        out.push(c);
        n += l;
    }
    out.push_str("\r\n");
    out
}

fn ics_when(s: &str, all_day: bool, tz: &Option<String>) -> (String, String) {
    let d = crate::rrule::parse_when(s);
    match (d, all_day) {
        (Some(d), true) => (";VALUE=DATE".into(), d.format("%Y%m%d").to_string()),
        (Some(d), false) => match tz.as_deref() {
            Some("UTC") => (String::new(), d.format("%Y%m%dT%H%M%SZ").to_string()),
            Some(z) if !z.is_empty() => (format!(";TZID={z}"), d.format("%Y%m%dT%H%M%S").to_string()),
            _ => (String::new(), d.format("%Y%m%dT%H%M%S").to_string()),
        },
        _ => (String::new(), s.to_string()),
    }
}

/// An .ics file with these events.
pub fn export(events: &[Event]) -> String {
    let stamp = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
    let mut s = String::from("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//Hearth OS//Hearth Calendar//EN\r\nCALSCALE:GREGORIAN\r\n");
    for e in events {
        s.push_str("BEGIN:VEVENT\r\n");
        s.push_str(&fold(&format!("UID:{}", e.id)));
        s.push_str(&format!("DTSTAMP:{stamp}\r\n"));
        let (p, v) = ics_when(&e.start, e.all_day, &e.tz);
        s.push_str(&fold(&format!("DTSTART{p}:{v}")));
        if !e.end.is_empty() { let (p, v) = ics_when(&e.end, e.all_day, &e.tz); s.push_str(&fold(&format!("DTEND{p}:{v}"))); }
        s.push_str(&fold(&format!("SUMMARY:{}", escape(&e.title))));
        if !e.location.is_empty() { s.push_str(&fold(&format!("LOCATION:{}", escape(&e.location)))); }
        if !e.notes.is_empty() { s.push_str(&fold(&format!("DESCRIPTION:{}", escape(&e.notes)))); }
        if let Some(r) = &e.rrule { s.push_str(&fold(&format!("RRULE:{r}"))); }
        for x in &e.exdates { let (p, v) = ics_when(x, e.all_day, &e.tz); s.push_str(&fold(&format!("EXDATE{p}:{v}"))); }
        for m in &e.reminders {
            s.push_str(&format!("BEGIN:VALARM\r\nACTION:DISPLAY\r\nDESCRIPTION:{}\r\nTRIGGER:-PT{}M\r\nEND:VALARM\r\n", escape(&e.title), m));
        }
        s.push_str("END:VEVENT\r\n");
    }
    s.push_str("END:VCALENDAR\r\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    const SAMPLE: &str = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VTIMEZONE\r\nTZID:Europe/Berlin\r\nEND:VTIMEZONE\r\n\
BEGIN:VEVENT\r\nUID:abc@x\r\nDTSTART;TZID=Europe/Berlin:20261022T093000\r\nDTEND;TZID=Europe/Berlin:20261022T103000\r\n\
SUMMARY:Team\\, weekly\r\nDESCRIPTION:Line one\\nLine two that is quite long and folds over\r\n  here\r\nRRULE:FREQ=WEEKLY;BYDAY=TH\r\n\
EXDATE;TZID=Europe/Berlin:20261029T093000\r\nBEGIN:VALARM\r\nTRIGGER:-PT15M\r\nACTION:DISPLAY\r\nEND:VALARM\r\nEND:VEVENT\r\n\
BEGIN:VEVENT\r\nUID:abc@x\r\nRECURRENCE-ID;TZID=Europe/Berlin:20261105T093000\r\nDTSTART;TZID=Europe/Berlin:20261105T110000\r\nSUMMARY:Moved\r\nEND:VEVENT\r\n\
BEGIN:VEVENT\r\nUID:holiday\r\nDTSTART;VALUE=DATE:20261225\r\nSUMMARY:Christmas\r\nEND:VEVENT\r\n\
BEGIN:VEVENT\r\nUID:utc\r\nDTSTART:20261023T120000Z\r\nDURATION:PT45M\r\nSUMMARY:Call\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";

    #[test]
    fn reads_events() {
        let ev = parse(SAMPLE, "local");
        assert_eq!(ev.len(), 3);
        let t = &ev[0];
        assert_eq!(t.title, "Team, weekly");
        assert_eq!(t.notes, "Line one\nLine two that is quite long and folds over here");
        assert_eq!(t.start, "2026-10-22T09:30:00");
        assert_eq!(t.tz.as_deref(), Some("Europe/Berlin"));
        assert_eq!(t.rrule.as_deref(), Some("FREQ=WEEKLY;BYDAY=TH"));
        assert_eq!(t.exdates, vec!["2026-10-29T09:30:00".to_string()]);
        assert_eq!(t.reminders, vec![15]);
        assert!(ev[1].all_day);
        assert_eq!(ev[1].end, "2026-12-26");
        assert_eq!(ev[2].tz.as_deref(), Some("UTC"));
        assert_eq!(ev[2].end, "2026-10-23T12:45:00");
    }

    #[test]
    fn round_trip() {
        let ev = parse(SAMPLE, "local");
        let text = export(&ev);
        assert!(text.lines().all(|l| l.len() <= 76));
        let back = parse(&text, "local");
        assert_eq!(back.len(), 3);
        for (a, b) in ev.iter().zip(back.iter()) {
            assert_eq!(a.title, b.title);
            assert_eq!(a.notes, b.notes);
            assert_eq!(a.start, b.start);
            assert_eq!(a.end, b.end);
            assert_eq!(a.tz, b.tz);
            assert_eq!(a.rrule, b.rrule);
            assert_eq!(a.exdates, b.exdates);
            assert_eq!(a.reminders, b.reminders);
            assert_eq!(a.all_day, b.all_day);
        }
    }

    #[test]
    fn durations_and_zones() {
        assert_eq!(duration_minutes("-PT15M"), Some(-15));
        assert_eq!(duration_minutes("P1DT2H"), Some(1560));
        assert_eq!(duration_minutes("-P1W"), Some(-10080));
        assert_eq!(map_tzid("/mozilla.org/20050126_1/Europe/Berlin"), "Europe/Berlin");
        assert_eq!(map_tzid("W. Europe Standard Time"), "Europe/Berlin");
        assert_eq!(map_tzid("Nowhere/Land"), "");
    }
}
