//! Hearth Time: four of Hearth OS's everyday apps in one program, sharing one window shell (Tauri: WebKitGTK on
//! Linux, WebView2 on Windows) and one look (ui/hearth.css, ui/hearth.js).
//!   Clock     alarms, timers, a stopwatch and world clocks (clock.rs; ringing: service.rs and ring.rs)
//!   Calendar  events with reminders and repeats, .ics import/export and subscriptions (calendar.rs, ics.rs, rrule.rs)
//!   Notes     notes and checklists as plain Markdown files in ~/Documents/Notes (notes.rs)
//!   Weather   Open-Meteo forecasts for a place you type in (weather.rs)
//! The background part (`hearth-time --service`, a systemd user service on Linux) rings alarms and timers with the
//! apps closed, shows calendar reminders, refreshes subscribed calendars and keeps the weather summary fresh.

pub mod alert;
pub mod calendar;
pub mod clock;
pub mod ics;
pub mod net;
pub mod notes;
pub mod paths;
pub mod ring;
pub mod rrule;
pub mod service;
pub mod store;
pub mod weather;

/// The four apps.
pub const APPS: [&str; 4] = ["clock", "calendar", "notes", "weather"];

/// Which app a program name or `--app` value means: "hearth-clock", "clock", "Clock" all mean the clock.
pub fn app_from_name(name: &str) -> Option<&'static str> {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name).to_ascii_lowercase();
    let base = base.trim_end_matches(".exe");
    let base = base.strip_prefix("hearth-").unwrap_or(base);
    APPS.iter().copied().find(|a| *a == base)
}

/// What Calendar was opened with (it's the default app for .ics files and webcal: links): ("file", path) for an .ics
/// file (a path or a file:// address), ("subscribe", https address) for a webcal:/webcals: link or an https .ics
/// address. Anything else isn't for Calendar.
pub fn calendar_open_target(arg: &str) -> Option<(&'static str, String)> {
    let a = arg.trim();
    let low = a.to_ascii_lowercase();
    let ics = |s: &str| s.split(['?', '#']).next().unwrap_or(s).ends_with(".ics");
    if let Some(rest) = low.strip_prefix("webcals://").or_else(|| low.strip_prefix("webcal://")) {
        if rest.is_empty() { return None; }
        return Some(("subscribe", format!("https://{}", &a[a.len() - rest.len()..])));
    }
    if low.starts_with("https://") || low.starts_with("http://") {
        return ics(&low).then(|| ("subscribe", a.to_string()));
    }
    if low.starts_with("file://") {
        let path = &a["file://".len()..];
        let path = path.strip_prefix("localhost").unwrap_or(path);
        return (path.starts_with('/') && ics(&low)).then(|| ("file", percent_decode(path)));
    }
    (!a.starts_with('-') && !low.contains("://") && low.ends_with(".ics")).then(|| ("file", a.to_string()))
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        let hex = |c: u8| (c as char).to_digit(16);
        if b[i] == b'%' && i + 2 < b.len() {
            if let (Some(h), Some(l)) = (hex(b[i + 1]), hex(b[i + 2])) { out.push((h * 16 + l) as u8); i += 3; continue; }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Milliseconds since 1970 (what the pages and the files use for moments in time).
pub fn now_ms() -> i64 { chrono::Utc::now().timestamp_millis() }

#[cfg(test)]
mod tests {
    #[test]
    fn calendar_open_targets() {
        use super::calendar_open_target as t;
        assert_eq!(t("/home/sam/Downloads/term.ics"), Some(("file", "/home/sam/Downloads/term.ics".into())));
        assert_eq!(t("file:///home/sam/My%20Calendar.ICS"), Some(("file", "/home/sam/My Calendar.ICS".into())));
        assert_eq!(t("webcal://example.com/cal.ics"), Some(("subscribe", "https://example.com/cal.ics".into())));
        assert_eq!(t("webcals://Example.com/Feed?x=1"), Some(("subscribe", "https://Example.com/Feed?x=1".into())));
        assert_eq!(t("https://example.com/holidays.ics?key=2"), Some(("subscribe", "https://example.com/holidays.ics?key=2".into())));
        assert_eq!(t("https://example.com/page"), None);
        assert_eq!(t("--app"), None);
        assert_eq!(t("calendar"), None);
        assert_eq!(t("alarm:1"), None);
        assert_eq!(t("webcal://"), None);
        assert_eq!(t("file:///tmp/notes.txt"), None);
    }
    #[test]
    fn app_names() {
        assert_eq!(super::app_from_name("/usr/bin/hearth-clock"), Some("clock"));
        assert_eq!(super::app_from_name("Weather"), Some("weather"));
        assert_eq!(super::app_from_name("C:\\x\\hearth-notes.exe"), Some("notes"));
        assert_eq!(super::app_from_name("hearth-time"), None);
    }
}
