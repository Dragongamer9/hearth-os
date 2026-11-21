//! The lights' automations, as the service uses them (service.rs apply_lights / update_night / on_login1):
//!
//! * **Dim the keys with Night light** follows Hearth's own Night light (Settings › Display): desktop.json's
//!   nightLight, nightMode ("times", "location", "always"), nightFrom / nightTo and nightLat / nightLon, read the way
//!   Mantel's hearth-screens does (NIGHT_DEFAULTS there). On KDE Plasma without Hearth's desktop.json (the Fedora
//!   edition), KWin's own Night Light in ~/.config/kwinrc [NightColor]. [`state_at`] says whether it's on now and how
//!   long until it next changes, so the service sets one timer to that moment: no loop, no checking every minute.
//! * **Lights off when the PC sleeps or locks**: logind's signals, read from one `gdbus monitor` on the system bus
//!   ([`parse_login1`]): PrepareForSleep (going to sleep / waking up), the session's LockedHint (Mantel's lock screen
//!   and KDE's both set it) and IdleHint. Nothing is asked again and again.
//!
//! Nothing here writes g15.json: dimming, sleeping and locking change the lights live and put back the levels you set.

use serde_json::Value;
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub enum Night {
    /// Night light is off (or never turned on).
    Off,
    /// "Warmer all the time".
    Always,
    /// Your times, minutes after midnight (local): from `from` to `to`, which may cross midnight.
    Times { from: u32, to: u32 },
    /// Sunset to sunrise at your place.
    Sun { lat: f64, lon: f64 },
}

/// "19:30" → minutes after midnight.
fn hhmm(s: &str) -> Option<u32> {
    let s = s.trim();
    let (h, m) = match s.split_once(':') {
        Some((h, m)) => (h, m),
        // kwinrc's "1930" / "700"
        None if s.len() >= 3 && s.len() <= 4 && s.bytes().all(|b| b.is_ascii_digit()) => s.split_at(s.len() - 2),
        None => return None,
    };
    let (h, m): (u32, u32) = (h.parse().ok()?, m.parse().ok()?);
    (h < 24 && m < 60).then_some(h * 60 + m)
}

fn num(v: Option<&Value>) -> Option<f64> {
    match v? {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

/// Hearth's Night light from desktop.json (None when it has no "nightLight" at all: then KWin's is asked).
/// Defaults as hearth-screens' NIGHT_DEFAULTS: times 19:30 to 07:00.
pub fn from_desktop(d: &Value) -> Option<Night> {
    let on = d.get("nightLight")?;
    if on.as_bool() != Some(true) {
        return Some(Night::Off);
    }
    let mode = d.get("nightMode").and_then(|v| v.as_str()).unwrap_or("times");
    let place = d.get("nightPlace").and_then(|v| v.as_str()).unwrap_or("");
    let (lat, lon) = (num(d.get("nightLat")), num(d.get("nightLon")));
    Some(match mode {
        "always" => Night::Always,
        "location" if !place.is_empty() => match (lat, lon) {
            (Some(lat), Some(lon)) if lat.abs() <= 90.0 && lon.abs() <= 180.0 => Night::Sun { lat, lon },
            _ => times(d.get("nightFrom").and_then(|v| v.as_str()), d.get("nightTo").and_then(|v| v.as_str()), 19 * 60 + 30, 7 * 60),
        },
        _ => times(d.get("nightFrom").and_then(|v| v.as_str()), d.get("nightTo").and_then(|v| v.as_str()), 19 * 60 + 30, 7 * 60),
    })
}

fn times(from: Option<&str>, to: Option<&str>, df: u32, dt: u32) -> Night {
    Night::Times { from: from.and_then(hhmm).unwrap_or(df), to: to.and_then(hhmm).unwrap_or(dt) }
}

/// KWin's Night Light ([NightColor] in kwinrc), for KDE Plasma without Hearth's desktop.json.
pub fn from_kwinrc(text: &str) -> Night {
    let mut inside = false;
    let mut kv = std::collections::HashMap::new();
    for l in text.lines() {
        let l = l.trim();
        if l.starts_with('[') {
            inside = l == "[NightColor]";
            continue;
        }
        if inside {
            if let Some((k, v)) = l.split_once('=') {
                kv.insert(k.trim().to_string(), v.trim().to_string());
            }
        }
    }
    if kv.get("Active").map(|s| s.as_str()) != Some("true") {
        return Night::Off;
    }
    let f = |k: &str| kv.get(k).and_then(|v| v.parse::<f64>().ok());
    match kv.get("Mode").map(|s| s.as_str()).unwrap_or("Automatic") {
        "Constant" => Night::Always,
        "Times" => times(kv.get("EveningBeginFixed").map(|s| s.as_str()), kv.get("MorningBeginFixed").map(|s| s.as_str()), 18 * 60, 6 * 60),
        "Location" => match (f("LatitudeFixed"), f("LongitudeFixed")) {
            (Some(lat), Some(lon)) => Night::Sun { lat, lon },
            _ => times(None, None, 18 * 60, 6 * 60),
        },
        _ => match (f("LatitudeAuto"), f("LongitudeAuto")) {
            (Some(lat), Some(lon)) => Night::Sun { lat, lon },
            _ => times(None, None, 18 * 60, 6 * 60),
        },
    }
}

/// Hearth's desktop.json first (Mantel's Settings › Display), else KWin's kwinrc.
pub fn load(desktop: &Path, kwinrc: &Path) -> Night {
    if let Some(n) = std::fs::read_to_string(desktop).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok()).and_then(|v| from_desktop(&v)) {
        return n;
    }
    std::fs::read_to_string(kwinrc).map(|t| from_kwinrc(&t)).unwrap_or(Night::Off)
}

/// ~/.local/share/nimbus (XDG_DATA_HOME respected): Hearth's desktop.json is there.
pub fn data_dir() -> std::path::PathBuf {
    std::env::var_os("XDG_DATA_HOME").map(std::path::PathBuf::from).unwrap_or_else(|| crate::root::home().join(".local/share")).join("nimbus")
}

/// Hearth's desktop.json (the pretend keyboard's own, if it has one).
pub fn desktop_json(r: &crate::root::Root) -> std::path::PathBuf {
    r.fixture_file("desktop.json").unwrap_or_else(|| data_dir().join("desktop.json"))
}

/// Night light's schedule as this PC has it (the service and the app's Lighting page).
pub fn for_root(r: &crate::root::Root) -> Night {
    let kwinrc = r.fixture_file("kwinrc").unwrap_or_else(|| crate::root::config_dir().parent().map(Path::to_path_buf).unwrap_or_else(crate::root::home).join("kwinrc"));
    load(&desktop_json(r), &kwinrc)
}

/// Sunrise and sunset in minutes after midnight UTC for day `yday` (1–366) at a place (NOAA's approximation, good
/// to a few minutes). The sunset can be past 1440 (after midnight UTC). None while the sun doesn't rise or set.
pub fn sun_utc(yday: u32, lat: f64, lon: f64) -> Option<(f64, f64)> {
    use std::f64::consts::PI;
    let g = 2.0 * PI / 365.0 * (yday as f64 - 1.0 + 0.5);
    let eq = 229.18 * (0.000075 + 0.001868 * g.cos() - 0.032077 * g.sin() - 0.014615 * (2.0 * g).cos() - 0.040849 * (2.0 * g).sin());
    let d = 0.006918 - 0.399912 * g.cos() + 0.070257 * g.sin() - 0.006758 * (2.0 * g).cos() + 0.000907 * (2.0 * g).sin() - 0.002697 * (3.0 * g).cos() + 0.00148 * (3.0 * g).sin();
    let la = lat.to_radians();
    let c = 90.833f64.to_radians().cos() / (la.cos() * d.cos()) - la.tan() * d.tan();
    if !c.is_finite() || c.abs() > 1.0 {
        return None;
    }
    let ha = c.acos().to_degrees();
    Some((720.0 - 4.0 * (lon + ha) - eq, 720.0 - 4.0 * (lon - ha) - eq))
}

/// Local time for a moment: (minutes after midnight, seconds, day of the year 1–366, offset from UTC in minutes).
pub fn local_parts(ts: i64) -> (u32, u32, u32, i64) {
    unsafe {
        let t: libc::time_t = ts as libc::time_t;
        let mut tm: libc::tm = std::mem::zeroed();
        libc::localtime_r(&t, &mut tm);
        ((tm.tm_hour * 60 + tm.tm_min).clamp(0, 1439) as u32, tm.tm_sec.clamp(0, 60) as u32, (tm.tm_yday + 1).clamp(1, 366) as u32, tm.tm_gmtoff as i64 / 60)
    }
}

fn in_window(now: u32, from: u32, to: u32) -> bool {
    if from <= to {
        now >= from && now < to
    } else {
        now >= from || now < to
    }
}

/// Minutes from `now` to `at` going forward around the clock (a whole day when they're the same minute).
fn ahead(now: u32, at: u32) -> u32 {
    let d = (at + 1440 - now % 1440) % 1440;
    if d == 0 {
        1440
    } else {
        d
    }
}

/// The night window in local minutes for a schedule on a day (None: Off / Always / no sunset there today).
pub fn window(n: &Night, yday: u32, offset_min: i64) -> Option<(u32, u32)> {
    match n {
        Night::Times { from, to } => Some((*from, *to)),
        Night::Sun { lat, lon } => {
            let (rise, set) = sun_utc(yday, *lat, *lon)?;
            let loc = |m: f64| ((m.round() as i64 + offset_min).rem_euclid(1440)) as u32;
            Some((loc(set), loc(rise)))
        }
        _ => None,
    }
}

/// Is Night light on at local minute `now` (with `sec` seconds into it), and how many seconds until it next changes
/// (None: it never changes by itself). The service sets its one timer to that; at the timer it asks again.
pub fn state_parts(n: &Night, now: u32, sec: u32, yday: u32, offset_min: i64) -> (bool, Option<u64>) {
    match n {
        Night::Off => (false, None),
        Night::Always => (true, None),
        Night::Sun { lat, .. } if window(n, yday, offset_min).is_none() => {
            // The midnight sun or the polar night: on all day in winter, off in summer; looked at again in 6 hours.
            let winter = (*lat >= 0.0) == !(80..=264).contains(&yday);
            (winter, Some(6 * 3600))
        }
        _ => {
            let Some((from, to)) = window(n, yday, offset_min) else { return (false, None) };
            if from == to {
                return (false, None);
            }
            let mins = ahead(now, from).min(ahead(now, to)) as u64;
            (in_window(now, from, to), Some((mins * 60).saturating_sub(sec as u64).max(1)))
        }
    }
}

pub fn state_at(n: &Night, ts: i64) -> (bool, Option<u64>) {
    let (now, sec, yday, off) = local_parts(ts);
    state_parts(n, now, sec, yday, off)
}

fn ampm(m: u32) -> String {
    let (h, mm) = (m / 60 % 24, m % 60);
    let h12 = if h % 12 == 0 { 12 } else { h % 12 };
    format!("{h12}:{mm:02} {}", if h < 12 { "AM" } else { "PM" })
}

/// For the Lighting page's line: "Key light goes to Low from 9:00 PM to 7:00 AM" and the like.
pub fn describe(n: &Night) -> String {
    match n {
        Night::Off => "Key light goes to Low while Night light is on (it's off in Settings › Display)".into(),
        Night::Always => "Key light goes to Low while Night light is on (all the time)".into(),
        Night::Times { from, to } => format!("Key light goes to Low from {} to {}", ampm(*from), ampm(*to)),
        Night::Sun { .. } => "Key light goes to Low from sunset to sunrise, with Night light".into(),
    }
}

// ------------------------------------------------------------------ logind, through `gdbus monitor --system`
#[derive(Debug, Clone, PartialEq)]
pub enum Login1 {
    /// PrepareForSleep: true going to sleep, false woken up.
    Sleep(bool),
    /// Our session's LockedHint.
    Locked(bool),
    /// Our session's IdleHint, and IdleSinceHintMonotonic (µs) when it came with it.
    Idle(bool, Option<u64>),
    /// A session came or went (our session id may be known now).
    Sessions,
}

/// logind's object path for a session id, as sd_bus_path_encode writes it ("2" → …/session/_32).
pub fn session_path(id: &str) -> String {
    let mut out = String::from("/org/freedesktop/login1/session/");
    if id.is_empty() {
        out.push('_');
        return out;
    }
    for (i, b) in id.bytes().enumerate() {
        if b.is_ascii_alphabetic() || (i > 0 && b.is_ascii_digit()) {
            out.push(b as char);
        } else {
            out.push_str(&format!("_{b:02x}"));
        }
    }
    out
}

fn prop<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let at = line.find(&format!("'{name}': <"))? + name.len() + 5;
    let rest = &line[at..];
    Some(rest[..rest.find('>')?].trim())
}

/// One line of `gdbus monitor --system --dest org.freedesktop.login1`; `session` is our session's object path.
pub fn parse_login1(line: &str, session: &str) -> Vec<Login1> {
    let mut out = Vec::new();
    if line.contains(".Manager.PrepareForSleep") {
        if line.contains("(true") {
            out.push(Login1::Sleep(true));
        } else if line.contains("(false") {
            out.push(Login1::Sleep(false));
        }
    }
    if line.contains(".Manager.SessionNew") || line.contains(".Manager.SessionRemoved") {
        out.push(Login1::Sessions);
    }
    let ours = !session.ends_with('/') && !session.ends_with("/_") && line.starts_with(session) && line[session.len()..].starts_with(':');
    if ours && line.contains("PropertiesChanged") {
        match prop(line, "LockedHint") {
            Some("true") => out.push(Login1::Locked(true)),
            Some("false") => out.push(Login1::Locked(false)),
            _ => {}
        }
        let since = prop(line, "IdleSinceHintMonotonic").and_then(|v| v.trim_start_matches("uint64").trim().parse().ok());
        match prop(line, "IdleHint") {
            Some("true") => out.push(Login1::Idle(true, since)),
            Some("false") => out.push(Login1::Idle(false, since)),
            _ => {}
        }
    }
    // `loginctl unlock-session`: unlocked (Lock alone isn't trusted: the lock screen sets LockedHint when it's up).
    if ours && line.contains(".Session.Unlock") {
        out.push(Login1::Locked(false));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn hearth_night_light_settings() {
        assert_eq!(from_desktop(&json!({})), None, "no Night light keys: KWin's is asked");
        assert_eq!(from_desktop(&json!({"nightLight": false, "nightFrom": "21:00"})), Some(Night::Off));
        assert_eq!(from_desktop(&json!({"nightLight": true})), Some(Night::Times { from: 19 * 60 + 30, to: 7 * 60 }), "hearth-screens' defaults");
        assert_eq!(from_desktop(&json!({"nightLight": true, "nightMode": "times", "nightFrom": "21:00", "nightTo": "07:00"})), Some(Night::Times { from: 1260, to: 420 }));
        assert_eq!(from_desktop(&json!({"nightLight": true, "nightMode": "always"})), Some(Night::Always));
        assert_eq!(from_desktop(&json!({"nightLight": true, "nightMode": "location", "nightPlace": "Chicago", "nightLat": 41.88, "nightLon": -87.63})), Some(Night::Sun { lat: 41.88, lon: -87.63 }));
        // A place not picked yet: your times.
        assert!(matches!(from_desktop(&json!({"nightLight": true, "nightMode": "location", "nightPlace": ""})), Some(Night::Times { .. })));
        let rc = "[General]\nx=1\n[NightColor]\nActive=true\nMode=Times\nEveningBeginFixed=2100\nMorningBeginFixed=0630\n[Other]\nActive=false\n";
        assert_eq!(from_kwinrc(rc), Night::Times { from: 1260, to: 390 });
        assert_eq!(from_kwinrc("[NightColor]\nMode=Constant\n"), Night::Off, "not Active");
        assert_eq!(from_kwinrc("[NightColor]\nActive=true\nMode=Constant\n"), Night::Always);
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("kwinrc"), rc).unwrap();
        std::fs::write(d.path().join("desktop.json"), r#"{"look": "x"}"#).unwrap();
        assert_eq!(load(&d.path().join("desktop.json"), &d.path().join("kwinrc")), Night::Times { from: 1260, to: 390 });
        std::fs::write(d.path().join("desktop.json"), r#"{"nightLight": true, "nightMode": "always"}"#).unwrap();
        assert_eq!(load(&d.path().join("desktop.json"), &d.path().join("kwinrc")), Night::Always);
    }

    #[test]
    fn one_timer_to_the_next_change() {
        let n = Night::Times { from: 21 * 60, to: 7 * 60 };
        // 8:59:30 PM: off, on in 30 s.
        assert_eq!(state_parts(&n, 20 * 60 + 59, 30, 100, 0), (false, Some(30)));
        // 9:00 PM: on until 7 AM (10 hours).
        assert_eq!(state_parts(&n, 21 * 60, 0, 100, 0), (true, Some(10 * 3600)));
        // 11:30 PM: on, 7.5 hours left, across midnight.
        assert_eq!(state_parts(&n, 23 * 60 + 30, 0, 100, 0), (true, Some(7 * 3600 + 1800)));
        // 6:59 AM: on, a minute left; 7:00 AM: off until 9 PM.
        assert_eq!(state_parts(&n, 6 * 60 + 59, 0, 100, 0), (true, Some(60)));
        assert_eq!(state_parts(&n, 7 * 60, 0, 100, 0), (false, Some(14 * 3600)));
        // A daytime window (from before to).
        let d = Night::Times { from: 600, to: 660 };
        assert_eq!(state_parts(&d, 630, 0, 1, 0), (true, Some(1800)));
        assert_eq!(state_parts(&d, 700, 0, 1, 0), (false, Some((1440 - 700 + 600) * 60)));
        // The same time twice: never on, no timer; Off and Always: no timer.
        assert_eq!(state_parts(&Night::Times { from: 5, to: 5 }, 5, 0, 1, 0), (false, None));
        assert_eq!(state_parts(&Night::Off, 0, 0, 1, 0), (false, None));
        assert_eq!(state_parts(&Night::Always, 0, 0, 1, 0), (true, None));
    }

    #[test]
    fn sunset_to_sunrise() {
        // Chicago, June 21: sunrise about 5:15 AM CDT (10:15 UTC), sunset about 8:29 PM CDT (01:29 UTC).
        let (rise, set) = sun_utc(172, 41.88, -87.63).unwrap();
        assert!((rise - 615.0).abs() < 10.0, "{rise}");
        assert!((set - 1529.0).abs() < 10.0, "{set}");
        let n = Night::Sun { lat: 41.88, lon: -87.63 };
        let (from, to) = window(&n, 172, -300).unwrap();
        assert!((from as i64 - (20 * 60 + 29)).abs() < 10 && (to as i64 - (5 * 60 + 15)).abs() < 10, "{from} {to}");
        // Noon: off, on at sunset; midnight: on.
        let (on, secs) = state_parts(&n, 12 * 60, 0, 172, -300);
        assert!(!on && (secs.unwrap() as i64 - (from as i64 - 720) * 60).abs() <= 60);
        assert!(state_parts(&n, 0, 0, 172, -300).0);
        // Svalbard in June: the sun never sets: off, asked again in 6 hours.
        assert_eq!(state_parts(&Night::Sun { lat: 78.0, lon: 15.0 }, 0, 0, 172, 120), (false, Some(6 * 3600)));
        assert_eq!(state_parts(&Night::Sun { lat: 78.0, lon: 15.0 }, 0, 0, 355, 60), (true, Some(6 * 3600)));
    }

    #[test]
    fn logind_lines() {
        let me = session_path("2");
        assert_eq!(me, "/org/freedesktop/login1/session/_32");
        assert_eq!(session_path("c1"), "/org/freedesktop/login1/session/c1");
        assert_eq!(parse_login1("/org/freedesktop/login1: org.freedesktop.login1.Manager.PrepareForSleep (true,)", &me), vec![Login1::Sleep(true)]);
        assert_eq!(parse_login1("/org/freedesktop/login1: org.freedesktop.login1.Manager.PrepareForSleep (false,)", &me), vec![Login1::Sleep(false)]);
        let l = "/org/freedesktop/login1/session/_32: org.freedesktop.DBus.Properties.PropertiesChanged ('org.freedesktop.login1.Session', {'LockedHint': <true>}, @as [])";
        assert_eq!(parse_login1(l, &me), vec![Login1::Locked(true)]);
        // Someone else's session: not ours.
        assert!(parse_login1(&l.replace("_32", "_33"), &me).is_empty());
        assert!(parse_login1(&l.replace("_32", "_320"), &me).is_empty());
        let i = "/org/freedesktop/login1/session/_32: org.freedesktop.DBus.Properties.PropertiesChanged ('org.freedesktop.login1.Session', {'IdleHint': <true>, 'IdleSinceHint': <uint64 1731966000000000>, 'IdleSinceHintMonotonic': <uint64 987654321>}, @as [])";
        assert_eq!(parse_login1(i, &me), vec![Login1::Idle(true, Some(987654321))]);
        assert_eq!(parse_login1("/org/freedesktop/login1/session/_32: org.freedesktop.login1.Session.Unlock ()", &me), vec![Login1::Locked(false)]);
        assert!(parse_login1("/org/freedesktop/login1/session/_32: org.freedesktop.login1.Session.Lock ()", &me).is_empty());
        assert_eq!(parse_login1("/org/freedesktop/login1: org.freedesktop.login1.Manager.SessionNew ('3', objectpath '/org/freedesktop/login1/session/_33')", &me), vec![Login1::Sessions]);
        // No session known: nothing is taken as ours.
        assert!(parse_login1(l, &session_path("")).is_empty());
    }

    #[test]
    fn the_lighting_line() {
        assert_eq!(describe(&Night::Times { from: 21 * 60, to: 7 * 60 }), "Key light goes to Low from 9:00 PM to 7:00 AM");
        assert_eq!(describe(&Night::Times { from: 0, to: 12 * 60 + 5 }), "Key light goes to Low from 12:00 AM to 12:05 PM");
    }
}
