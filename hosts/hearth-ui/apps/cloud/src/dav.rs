//! Nextcloud (AGPL-3, nextcloud.com) through its own open ways in: WebDAV for files (PROPFIND, DAV SEARCH for the
//! newest, PUT, MKCOL, PROPPATCH, the trash bin), CalDAV for the calendar (REPORT calendar-query), CardDAV for contacts
//! (REPORT addressbook-query) and its OCS API (shares, the server's numbers, sign-in with Login Flow v2: you approve
//! this PC on Nextcloud's own page and it hands back an app password, kept in your keyring).

use hearth_oss::http::{self, Req};
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct Acct {
    pub base: String,
    pub user: String,
    pub pass: String,
}

impl Acct {
    fn req(&self, method: &str, path: &str) -> Req {
        Req::new(method, &format!("{}{}", self.base, path)).basic(&self.user, &self.pass).header("OCS-APIRequest", "true").timeout(30)
    }
    pub fn dav(&self, method: &str, path: &str, depth: &str, body: &str) -> Result<String, String> {
        let mut r = self.req(method, path).header("Depth", depth);
        if !body.is_empty() {
            r = r.body(body.as_bytes().to_vec(), "application/xml; charset=utf-8");
        }
        let a = http::send(&r).map_err(|e| format!("Your cloud doesn't answer: {e}"))?;
        if a.status == 401 {
            return Err("Your cloud asks you to sign in again".into());
        }
        if !(a.ok() || a.status == 207) {
            return Err(format!("Your cloud answered {}", a.status));
        }
        Ok(a.text())
    }
    pub fn ocs(&self, path: &str) -> Result<Value, String> {
        let a = http::send(&self.req("GET", &format!("{path}{}format=json", if path.contains('?') { "&" } else { "?" }))).map_err(|e| format!("Your cloud doesn't answer: {e}"))?;
        a.json().and_then(|j| j.pointer("/ocs/data").cloned()).ok_or_else(|| format!("Your cloud answered {}", a.status))
    }
    pub fn send(&self, method: &str, path: &str, body: Vec<u8>, ctype: &str, extra: &[(&str, String)]) -> Result<u16, String> {
        let mut r = self.req(method, path);
        for (k, v) in extra {
            r = r.header(k, v.clone());
        }
        if !body.is_empty() || matches!(method, "PUT" | "POST") {
            r = r.body(body, ctype);
        }
        let a = http::send(&r).map_err(|e| format!("Your cloud doesn't answer: {e}"))?;
        if a.ok() || a.status == 207 {
            Ok(a.status)
        } else {
            Err(format!("Your cloud said no ({})", a.status))
        }
    }
    pub fn files(&self) -> String {
        format!("/remote.php/dav/files/{}", http::enc(&self.user))
    }
}

/// One file or folder from a multistatus answer.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Entry {
    pub href: String,
    pub name: String,
    pub dir: bool,
    pub modified: i64,
    pub size: u64,
    /// the Hearth device that changed it last (Hearth's own property), or who did
    pub from: String,
    pub used: u64,
    pub avail: i64,
    pub ctype: String,
    pub color: String,
    pub data: String,
}

fn local<'a>(n: &roxmltree::Node<'a, 'a>) -> &'a str {
    n.tag_name().name()
}

/// A multistatus (207) answer's entries.
pub fn multistatus(xml: &str) -> Vec<Entry> {
    let Ok(doc) = roxmltree::Document::parse(xml) else { return Vec::new() };
    let mut out = Vec::new();
    for r in doc.descendants().filter(|n| local(n) == "response") {
        let mut e = Entry::default();
        for n in r.descendants() {
            let txt = n.text().unwrap_or("").trim().to_string();
            match local(&n) {
                "href" => e.href = http::dec(&txt),
                "getlastmodified" => e.modified = http_date(&txt),
                "getcontentlength" | "size" => e.size = txt.parse().unwrap_or(e.size),
                "collection" => e.dir = true,
                "device" if n.tag_name().namespace() == Some(NS) => e.from = txt,
                "owner-display-name" if e.from.is_empty() => e.from = txt,
                "quota-used-bytes" => e.used = txt.parse().unwrap_or(0),
                "quota-available-bytes" => e.avail = txt.parse().unwrap_or(-1),
                "getcontenttype" => e.ctype = txt,
                "calendar-color" => e.color = txt,
                "calendar-data" | "address-data" => e.data = n.text().unwrap_or("").to_string(),
                "displayname" if !txt.is_empty() => e.name = txt,
                _ => {}
            }
        }
        if e.name.is_empty() {
            e.name = e.href.trim_end_matches('/').rsplit('/').next().unwrap_or("").to_string();
        }
        out.push(e);
    }
    out
}

/// Hearth's own WebDAV property: which Hearth device changed a file (written by Hearth Cloud when it uploads).
pub const NS: &str = "https://hearthos.fun/ns";

/// "Thu, 26 Nov 2026 01:21:00 GMT" → seconds since 1970.
pub fn http_date(s: &str) -> i64 {
    let p: Vec<&str> = s.split_whitespace().collect();
    if p.len() < 5 {
        return 0;
    }
    let months = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let (d, m, y) = (p[1].parse::<i64>().unwrap_or(1), months.iter().position(|x| *x == p[2]).unwrap_or(0) as i64 + 1, p[3].parse::<i64>().unwrap_or(1970));
    let hms: Vec<i64> = p[4].split(':').map(|x| x.parse().unwrap_or(0)).collect();
    days_from_civil(y, m, d) * 86_400 + hms.first().unwrap_or(&0) * 3600 + hms.get(1).unwrap_or(&0) * 60 + hms.get(2).unwrap_or(&0)
}
pub fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The newest files (DAV SEARCH, newest first).
pub fn search_body(user: &str, n: usize, name_like: &str) -> String {
    let filter = if name_like.is_empty() {
        "<d:not><d:is-collection/></d:not>".to_string()
    } else {
        format!("<d:and><d:not><d:is-collection/></d:not><d:like><d:prop><d:displayname/></d:prop><d:literal>%{}%</d:literal></d:like></d:and>", xml_escape(name_like))
    };
    format!(
        r#"<?xml version="1.0"?><d:searchrequest xmlns:d="DAV:" xmlns:oc="http://owncloud.org/ns" xmlns:h="{NS}"><d:basicsearch><d:select><d:prop><d:displayname/><d:getlastmodified/><d:getcontentlength/><d:getcontenttype/><oc:owner-display-name/><h:device/></d:prop></d:select><d:from><d:scope><d:href>/files/{}</d:href><d:depth>infinity</d:depth></d:scope></d:from><d:where>{filter}</d:where><d:orderby><d:order><d:prop><d:getlastmodified/></d:prop><d:descending/></d:order></d:orderby><d:limit><d:nresults>{n}</d:nresults></d:limit></d:basicsearch></d:searchrequest>"#,
        xml_escape(user)
    )
}
pub fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// A file's folder as the board writes it ("Projects/Gravesleep"), from its DAV path.
pub fn folder_of(href: &str, user: &str) -> String {
    let root = format!("/remote.php/dav/files/{user}/");
    let rel = href.split_once(&root).map(|x| x.1).unwrap_or(href);
    rel.trim_end_matches('/').rsplit_once('/').map(|x| x.0.to_string()).unwrap_or_default()
}

// ------------------------------------------------------------------ calendars and contacts

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Event {
    pub title: String,
    pub start: i64,
    pub end: i64,
    pub all_day: bool,
    pub place: String,
    pub note: String,
    pub repeats: String,
    pub color: String,
    pub calendar: String,
}

/// "20261126T220000" / "20261126" / "...Z" → seconds (local times taken as local).
pub fn ical_time(s: &str, utc_offset: i64) -> (i64, bool) {
    let s = s.trim();
    let date = &s[..s.len().min(8)];
    if date.len() < 8 {
        return (0, false);
    }
    let (y, m, d) = (date[..4].parse().unwrap_or(1970), date[4..6].parse().unwrap_or(1), date[6..8].parse().unwrap_or(1));
    let base = days_from_civil(y, m, d) * 86_400;
    if s.len() < 15 {
        return (base - utc_offset, true);
    }
    let t = &s[9..15];
    let secs = t[..2].parse::<i64>().unwrap_or(0) * 3600 + t[2..4].parse::<i64>().unwrap_or(0) * 60 + t[4..6].parse::<i64>().unwrap_or(0);
    (base + secs - if s.ends_with('Z') { 0 } else { utc_offset }, false)
}

/// The lines of an iCalendar or vCard, folded lines joined.
pub fn unfold(s: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for l in s.replace("\r\n", "\n").split('\n') {
        if (l.starts_with(' ') || l.starts_with('\t')) && !out.is_empty() {
            let last = out.len() - 1;
            out[last].push_str(&l[1..]);
        } else if !l.is_empty() {
            out.push(l.to_string());
        }
    }
    out
}
fn val(l: &str) -> (String, String) {
    let (k, v) = l.split_once(':').unwrap_or((l, ""));
    (k.split(';').next().unwrap_or(k).to_uppercase(), v.replace("\\,", ",").replace("\\n", "\n").replace("\\;", ";"))
}

pub fn events(ics: &str, utc_offset: i64) -> Vec<Event> {
    let mut out = Vec::new();
    let mut cur: Option<Event> = None;
    for l in unfold(ics) {
        let (k, v) = val(&l);
        match (k.as_str(), v.as_str()) {
            ("BEGIN", "VEVENT") => cur = Some(Event::default()),
            ("END", "VEVENT") => {
                if let Some(e) = cur.take() {
                    out.push(e);
                }
            }
            _ => {
                if let Some(e) = cur.as_mut() {
                    match k.as_str() {
                        "SUMMARY" => e.title = v,
                        "DTSTART" => {
                            let (t, a) = ical_time(&v, utc_offset);
                            e.start = t;
                            e.all_day = a;
                        }
                        "DTEND" => e.end = ical_time(&v, utc_offset).0,
                        "LOCATION" => e.place = v,
                        "DESCRIPTION" => e.note = v,
                        "RRULE" => e.repeats = v,
                        _ => {}
                    }
                }
            }
        }
    }
    out
}

/// "FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR" → "repeats Mon–Fri"
pub fn repeats_words(r: &str) -> String {
    if r.is_empty() {
        return String::new();
    }
    if r.contains("BYDAY=MO,TU,WE,TH,FR") && !r.contains("SA") {
        return "repeats Mon–Fri".into();
    }
    if r.contains("FREQ=DAILY") {
        return "repeats every day".into();
    }
    if r.contains("FREQ=WEEKLY") {
        return "repeats every week".into();
    }
    if r.contains("FREQ=YEARLY") {
        return "repeats every year".into();
    }
    "repeats".into()
}

/// Is the event on that day (its days, or its weekly days)?
pub fn on_day(e: &Event, day_start: i64, wday: usize) -> bool {
    let day_end = day_start + 86_400;
    if e.start < day_end && (e.end.max(e.start + 1)) > day_start {
        return true;
    }
    if e.start < day_end && !e.repeats.is_empty() {
        const D: [&str; 7] = ["SU", "MO", "TU", "WE", "TH", "FR", "SA"];
        if e.repeats.contains("FREQ=DAILY") {
            return true;
        }
        if e.repeats.contains("FREQ=WEEKLY") {
            return e.repeats.split(';').find(|p| p.starts_with("BYDAY=")).map(|p| p[6..].split(',').any(|d| d.ends_with(D[wday]))).unwrap_or(false);
        }
    }
    false
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Contact {
    pub name: String,
    pub phone: bool,
    pub email: bool,
    pub address: bool,
    pub other: Vec<String>,
    pub groups: Vec<String>,
}

pub fn contacts(vcf: &str) -> Vec<Contact> {
    let mut out = Vec::new();
    let mut cur: Option<Contact> = None;
    for l in unfold(vcf) {
        let (k, v) = val(&l);
        match (k.as_str(), v.as_str()) {
            ("BEGIN", "VCARD") => cur = Some(Contact::default()),
            ("END", "VCARD") => {
                if let Some(c) = cur.take() {
                    out.push(c);
                }
            }
            _ => {
                if let Some(c) = cur.as_mut() {
                    match k.as_str() {
                        "FN" => c.name = v,
                        "TEL" => c.phone = true,
                        "EMAIL" => c.email = true,
                        "ADR" => c.address = true,
                        "CATEGORIES" => c.groups = v.split(',').map(|x| x.trim().to_lowercase()).filter(|x| !x.is_empty()).collect(),
                        "X-SOCIALPROFILE" | "IMPP" => {
                            let t = l.to_lowercase();
                            if t.contains("discord") {
                                c.other.push("Discord".into());
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    out
}

impl Contact {
    /// "Phone, email · family"
    pub fn sub(&self) -> String {
        let mut has: Vec<String> = self.other.clone();
        if self.phone {
            has.insert(0, "Phone".into());
        }
        if self.email {
            has.push("email".into());
        }
        if self.address {
            has.push("address".into());
        }
        let mut s = has.join(", ");
        if let Some(f) = s.get(..1) {
            s = f.to_uppercase() + &s[1..];
        }
        if let Some(g) = self.groups.first() {
            s = format!("{s} · {g}");
        }
        s
    }
}

pub fn calendar_query(from: i64, to: i64) -> String {
    let f = |t: i64| {
        let d = t.div_euclid(86_400);
        let (y, m, dd) = civil(d);
        let s = t.rem_euclid(86_400);
        format!("{y:04}{m:02}{dd:02}T{:02}{:02}{:02}Z", s / 3600, s / 60 % 60, s % 60)
    };
    format!(
        r#"<?xml version="1.0"?><c:calendar-query xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav"><d:prop><d:getetag/><c:calendar-data/></d:prop><c:filter><c:comp-filter name="VCALENDAR"><c:comp-filter name="VEVENT"><c:time-range start="{}" end="{}"/></c:comp-filter></c:comp-filter></c:filter></c:calendar-query>"#,
        f(from),
        f(to)
    )
}
pub fn civil(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + if m <= 2 { 1 } else { 0 }, m, d)
}

pub const CARD_QUERY: &str = r#"<?xml version="1.0"?><card:addressbook-query xmlns:d="DAV:" xmlns:card="urn:ietf:params:xml:ns:carddav"><d:prop><d:getetag/><card:address-data/></d:prop></card:addressbook-query>"#;
pub const QUOTA: &str = r#"<?xml version="1.0"?><d:propfind xmlns:d="DAV:"><d:prop><d:quota-used-bytes/><d:quota-available-bytes/></d:prop></d:propfind>"#;
pub const CALS: &str = r#"<?xml version="1.0"?><d:propfind xmlns:d="DAV:" xmlns:x1="http://apple.com/ns/ical/" xmlns:c="urn:ietf:params:xml:ns:caldav"><d:prop><d:displayname/><x1:calendar-color/><d:resourcetype/></d:prop></d:propfind>"#;

/// An event for "＋ Event", as iCalendar.
pub fn new_event(uid: &str, title: &str, start: i64, mins: i64) -> String {
    let f = |t: i64| {
        let (y, m, d) = civil(t.div_euclid(86_400));
        let s = t.rem_euclid(86_400);
        format!("{y:04}{m:02}{d:02}T{:02}{:02}{:02}Z", s / 3600, s / 60 % 60, s % 60)
    };
    format!("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//Hearth//Hearth Cloud//EN\r\nBEGIN:VEVENT\r\nUID:{uid}\r\nDTSTAMP:{}\r\nDTSTART:{}\r\nDTEND:{}\r\nSUMMARY:{}\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n", f(hearth_oss::now()), f(start), f(start + mins * 60), title.replace(',', "\\,").replace(';', "\\;"))
}

/// Login Flow v2: (the page where you approve this PC, the poll token, the poll address).
pub fn login_start(base: &str) -> Result<(String, String, String), String> {
    let a = http::send(&Req::new("POST", &format!("{base}/index.php/login/v2")).header("User-Agent", "Hearth Cloud").timeout(15)).map_err(|e| format!("Your cloud doesn't answer: {e}"))?;
    let j = a.json().ok_or_else(|| format!("Your cloud answered {}", a.status))?;
    let s = |p: &str| j.pointer(p).and_then(Value::as_str).unwrap_or("").to_string();
    Ok((s("/login"), s("/poll/token"), s("/poll/endpoint")))
}
/// Polled until you approve: (user, app password).
pub fn login_poll(endpoint: &str, token: &str) -> Option<(String, String)> {
    let a = http::send(&Req::new("POST", endpoint).form(&[("token", token)]).timeout(15)).ok()?;
    let j = a.json()?;
    Some((j.get("loginName")?.as_str()?.to_string(), j.get("appPassword")?.as_str()?.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn webdav_answers_are_read() {
        let x = r#"<?xml version="1.0"?><d:multistatus xmlns:d="DAV:" xmlns:oc="http://owncloud.org/ns" xmlns:h="https://hearthos.fun/ns">
          <d:response><d:href>/remote.php/dav/files/me/Projects/Gravesleep/Gravesleep%20story%20outline.odt</d:href><d:propstat><d:prop>
          <d:getlastmodified>Thu, 26 Nov 2026 01:21:00 GMT</d:getlastmodified><d:getcontentlength>2048</d:getcontentlength><h:device>Laptop</h:device>
          <d:resourcetype/></d:prop></d:propstat></d:response>
          <d:response><d:href>/remote.php/dav/files/me/Documents/</d:href><d:propstat><d:prop><d:resourcetype><d:collection/></d:resourcetype><d:quota-used-bytes>5</d:quota-used-bytes></d:prop></d:propstat></d:response></d:multistatus>"#;
        let v = multistatus(x);
        assert_eq!(v.len(), 2);
        assert_eq!((v[0].name.as_str(), v[0].from.as_str(), v[0].size, v[0].dir), ("Gravesleep story outline.odt", "Laptop", 2048, false));
        assert_eq!(folder_of(&v[0].href, "me"), "Projects/Gravesleep");
        assert!(v[1].dir && v[1].used == 5);
        assert_eq!(v[0].modified, days_from_civil(2026, 11, 26) * 86_400 + 21 * 60 + 3600);
        assert!(search_body("me", 7, "").contains("<d:nresults>7</d:nresults>"));
    }

    #[test]
    fn events_and_contacts_are_read() {
        let ics = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nSUMMARY:Work\\, overnight shift\r\nDTSTART:20261126T220000\r\nDTEND:20261127T060000\r\nRRULE:FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nSUMMARY:Thanksgiving\r\nDTSTART;VALUE=DATE:20261126\r\nLOCATION:at Dad's\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
        let e = events(ics, 0);
        assert_eq!(e.len(), 2);
        assert_eq!(e[0].title, "Work, overnight shift");
        assert_eq!(repeats_words(&e[0].repeats), "repeats Mon–Fri");
        assert!(e[1].all_day);
        let day = days_from_civil(2026, 11, 26) * 86_400;
        assert!(on_day(&e[1], day, 4));
        assert!(on_day(&e[0], day + 86_400 * 7, 4), "a weekly event on its day next week");
        let c = contacts("BEGIN:VCARD\nFN:Dad\nTEL:1\nEMAIL:d@x\nCATEGORIES:Family\nEND:VCARD\nBEGIN:VCARD\nFN:Pizza place\nTEL:2\nADR:;;1 Main\nEND:VCARD\n");
        assert_eq!(c[0].sub(), "Phone, email · family");
        assert_eq!(c[1].sub(), "Phone, address");
        assert_eq!(civil(days_from_civil(2026, 2, 28)), (2026, 2, 28));
    }
}
