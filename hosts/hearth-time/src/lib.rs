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

/// Milliseconds since 1970 (what the pages and the files use for moments in time).
pub fn now_ms() -> i64 { chrono::Utc::now().timestamp_millis() }

#[cfg(test)]
mod tests {
    #[test]
    fn app_names() {
        assert_eq!(super::app_from_name("/usr/bin/hearth-clock"), Some("clock"));
        assert_eq!(super::app_from_name("Weather"), Some("weather"));
        assert_eq!(super::app_from_name("C:\\x\\hearth-notes.exe"), Some("notes"));
        assert_eq!(super::app_from_name("hearth-time"), None);
    }
}
