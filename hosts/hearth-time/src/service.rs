//! `hearth-time --service`: the small background part (a systemd user service on Linux: hearth-time.service).
//! Once a second it looks at the clock's and the calendar's files (they're only read again when they change) and:
//!   - rings alarms (always, also in Do not disturb and in Game Mode: ring.rs) and timers (in Do not disturb only a
//!     quiet notification) with the ringing window (`hearth-time --ring alarm:<id>`) and the tone until you stop
//!     or snooze it, or for the clock's "ringMinutes" (then a "missed alarm" notification);
//!   - shows calendar reminders as notifications (Do not disturb keeps them quiet: the taskbar's rule);
//!   - downloads subscribed calendars every 6 hours and the weather every 30 minutes (only with a place chosen).
//! What has rung is remembered in paths::service_dir()/rang.json, so nothing rings twice, also after a restart.
//! The Clock app stops a ring with no window (no display at the time) by creating the file "stop" there.

use crate::{alert, calendar, clock, paths, ring, store};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const REMINDER_GRACE_MS: i64 = 60 * 60 * 1000;
const SUB_EVERY_MS: i64 = 6 * 3600 * 1000;
const KEEP_RANG_MS: i64 = 8 * 24 * 3600 * 1000;

fn rang_file() -> PathBuf { paths::service_dir().join("rang.json") }
pub fn stop_file() -> PathBuf { paths::service_dir().join("stop") }
pub fn ringing_file() -> PathBuf { paths::service_dir().join("ringing.json") }

struct State {
    rang: HashMap<String, i64>,
    clock: Value,
    clock_m: i64,
    cal: Value,
    cal_m: i64,
    /// Reminders coming up (when, key, title, body), worked out again when the calendar changes or every 10 minutes.
    reminders: Vec<(i64, String, String, String)>,
    reminders_at: i64,
    last_sub_check: i64,
    last_weather: i64,
}

impl State {
    fn load() -> State {
        let r = store::load(&rang_file());
        let rang = r.as_object().map(|o| o.iter().filter_map(|(k, v)| v.as_i64().map(|t| (k.clone(), t))).collect()).unwrap_or_default();
        State { rang, clock: Value::Null, clock_m: -1, cal: Value::Null, cal_m: -1, reminders: vec![], reminders_at: 0, last_sub_check: 0, last_weather: 0 }
    }
    fn mark(&mut self, key: &str, now: i64) {
        self.rang.insert(key.to_string(), now);
        self.rang.retain(|_, t| now - *t < KEEP_RANG_MS);
        let v: serde_json::Map<String, Value> = self.rang.iter().map(|(k, t)| (k.clone(), json!(t))).collect();
        let _ = store::save(&rang_file(), &Value::Object(v));
    }
    fn refresh_files(&mut self, now: i64) {
        let cm = store::mtime(&paths::clock_file());
        if cm != self.clock_m { self.clock = clock::load(); self.clock_m = cm; }
        let km = store::mtime(&paths::calendar_file()) + paths::subs_dir().read_dir().into_iter().flatten().flatten().map(|e| store::mtime(&e.path())).sum::<i64>();
        if km != self.cal_m || now - self.reminders_at > 10 * 60 * 1000 {
            if km != self.cal_m { self.cal = calendar::load(); self.cal_m = km; }
            self.reminders = upcoming_reminders(&self.cal, now);
            self.reminders_at = now;
        }
    }
}

/// Calendar reminders from an hour ago to a day ahead: (when, key, title, body).
pub fn upcoming_reminders(cal: &Value, now: i64) -> Vec<(i64, String, String, String)> {
    let occ = calendar::range(cal, now - REMINDER_GRACE_MS, now + 2 * 24 * 3600 * 1000);
    let mut v = Vec::new();
    for o in occ {
        for (at, m) in calendar::reminder_times(&o) {
            if at < now - REMINDER_GRACE_MS || at > now + 24 * 3600 * 1000 { continue; }
            let when = chrono::TimeZone::timestamp_millis_opt(&chrono::Local, o.start).single();
            let body = if o.all_day { "All day".to_string() } else {
                let t = when.map(|w| w.format("%H:%M").to_string()).unwrap_or_default();
                let inm = if m == 0 { "now".to_string() } else if m < 60 { format!("in {m} min") } else if m % 60 == 0 && m < 1440 { format!("in {} h", m / 60) } else { format!("at {t}") };
                format!("{t} ({inm})")
            };
            let body = if o.location.is_empty() { body } else { format!("{body} · {}", o.location) };
            v.push((at, format!("rem:{}:{}:{m}", o.id, o.occ), o.title.clone(), body));
        }
    }
    v.sort_by_key(|x| x.0);
    v
}

/// The ringing window, opened on the session's display.
fn open_ring_window(items: &[clock::Due], env: &[(String, String)]) -> Option<Child> {
    let exe = std::env::current_exe().ok()?;
    let mode = ring::mode(env);
    if mode == "none" { return None; }
    let arg = items.iter().map(|d| format!("{}:{}", d.kind, d.id)).collect::<Vec<_>>().join(",");
    let mut c = Command::new(exe);
    c.args(["--ring", &arg]).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    ring::apply_env(&mut c, env, mode == "game");
    if mode == "game" { c.env("HEARTH_RING_PIN", "1"); }
    c.spawn().ok()
}

fn describe(items: &[clock::Due]) -> (String, String) {
    let first = &items[0];
    let when = chrono::TimeZone::timestamp_millis_opt(&chrono::Local, first.at).single().map(|w| w.format("%H:%M").to_string()).unwrap_or_default();
    let title = if first.kind == "alarm" { if first.label.is_empty() { format!("Alarm {when}") } else { format!("{} ({when})", first.label) } }
        else if first.label.is_empty() { "Timer done".to_string() } else { format!("{} is done", first.label) };
    let more = if items.len() > 1 { format!(" and {} more", items.len() - 1) } else { String::new() };
    (format!("{title}{more}"), if first.kind == "alarm" { "Alarm".into() } else { "Timer".into() })
}

/// Rings `items` until they're stopped. Blocks.
fn ring_now(st: &mut State, items: Vec<clock::Due>, now: i64) {
    for d in &items { st.mark(&d.key, now); }
    let alarms = items.iter().any(|d| d.kind == "alarm");
    let dnd = paths::dnd_on();
    let (title, kind) = describe(&items);
    // Do not disturb: a timer is only a quiet note in the clock panel.
    if dnd && !alarms {
        alert::notify("Clock", "hearth-clock", &title, "Your timer finished (quietly: Do not disturb is on).", alert::Urgency::Normal);
        return;
    }
    let env = ring::session_env();
    ring::wake_screen(&env);
    let _ = std::fs::remove_file(stop_file());
    let _ = store::save(&ringing_file(), &json!({"since": now, "items": items.iter().map(|d| json!({"kind": d.kind, "id": d.id, "label": d.label, "at": d.at})).collect::<Vec<_>>()}));
    let mut window = open_ring_window(&items, &env);
    if window.is_none() {
        alert::notify("Clock", "hearth-clock", &title, &format!("{kind} ringing. Open Clock to stop it."), if alarms { alert::Urgency::Critical } else { alert::Urgency::Normal });
    }
    let mut ringer = alert::Ringer::new();
    let limit = Duration::from_secs(60 * st.clock["settings"]["ringMinutes"].as_u64().unwrap_or(10).clamp(1, 60));
    let start = Instant::now();
    let mut stopped = false;
    while start.elapsed() < limit {
        ringer.tick(&env);
        if let Some(w) = window.as_mut() {
            if !matches!(w.try_wait(), Ok(None)) { stopped = true; break; }
        }
        if stop_file().exists() { stopped = true; break; }
        std::thread::sleep(Duration::from_millis(200));
    }
    ringer.stop();
    if let Some(mut w) = window { let _ = w.kill(); let _ = w.wait(); }
    let _ = std::fs::remove_file(stop_file());
    let _ = std::fs::remove_file(ringing_file());
    if !stopped {
        alert::notify("Clock", "hearth-clock", &format!("Missed: {title}"), "It rang and nobody stopped it.", alert::Urgency::Normal);
        // Nobody answered: what ringing would have changed happens anyway (a one-time alarm switches off, a timer resets).
        let mut c = clock::load();
        for d in &items { clock::after_ring(&mut c, d.kind, &d.id, "dismiss", crate::now_ms()); }
        let _ = store::save(&paths::clock_file(), &c);
    }
}

fn background_jobs(st: &mut State, now: i64) {
    // Subscribed calendars: every 6 hours (checked every 10 minutes; a failed download is tried again then).
    if now - st.last_sub_check > 10 * 60 * 1000 {
        st.last_sub_check = now;
        let mut cal = calendar::load();
        let mut changed = false;
        if let Some(subs) = cal["subs"].as_array_mut() {
            for s in subs.iter_mut() {
                let last = s["updated"].as_i64().unwrap_or(0);
                if now - last > SUB_EVERY_MS { let _ = calendar::refresh_sub(s); changed = true; }
            }
        }
        if changed { let _ = calendar::save(&cal); }
    }
    // The weather summary for the taskbar.
    if now - st.last_weather > crate::weather::FRESH_MS {
        st.last_weather = now;
        if !crate::weather::settings()["place"].is_null() { let _ = crate::weather::refresh(); }
    }
}

pub fn run() -> i32 {
    eprintln!("hearth-time service: started (clock {}, calendar {})", paths::clock_file().display(), paths::calendar_file().display());
    let mut st = State::load();
    // Downloads wait a little after sign-in (the network may not be up yet).
    let started = crate::now_ms();
    st.last_sub_check = started - 9 * 60 * 1000;
    st.last_weather = started - crate::weather::FRESH_MS + 60 * 1000;
    loop {
        let now = crate::now_ms();
        st.refresh_files(now);
        let rang = st.rang.clone();
        let due = clock::due(&st.clock, now, &|k: &str| rang.contains_key(k));
        if !due.is_empty() {
            ring_now(&mut st, due, now);
            continue;
        }
        let rems: Vec<(i64, String, String, String)> = st.reminders.iter().filter(|r| r.0 <= now && !st.rang.contains_key(&r.1)).cloned().collect();
        for (_, key, title, body) in rems {
            st.mark(&key, now);
            alert::notify("Calendar", "hearth-calendar", &title, &body, alert::Urgency::Normal);
        }
        background_jobs(&mut st, now);
        std::thread::sleep(Duration::from_millis(1000));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reminder_list() {
        let now = calendar::to_ms(crate::rrule::parse_when("2026-10-22T08:50:00").unwrap(), None).unwrap();
        let cal = json!({"events": [
            {"id": "a", "calendar": "local", "title": "Standup", "start": "2026-10-22T09:00:00", "end": "2026-10-22T09:15:00", "reminders": [15, 5], "location": "Room 2"},
            {"id": "b", "calendar": "local", "title": "Old", "start": "2026-10-20T09:00:00", "end": "2026-10-20T09:15:00", "reminders": [5]}
        ], "subs": []});
        let r = upcoming_reminders(&cal, now);
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].2, "Standup");
        assert!(r[0].3.contains("in 15 min") && r[0].3.contains("Room 2"));
        assert!(r[1].1.ends_with(":5"));
    }
}
