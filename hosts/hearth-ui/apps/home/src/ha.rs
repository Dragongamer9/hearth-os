//! Home Assistant (Apache-2.0, home-assistant.io) through its own REST API on the Nvidia PC (hearth-oss-server, port
//! 8123, 127.0.0.1 and Tailscale only), with a long-lived access token you make in Home Assistant (kept in your
//! keyring): /api/states, /api/services/<domain>/<service>, /api/template (rooms), /api/history (today's energy),
//! /api/camera_proxy (the cameras' pictures) and /api/config/automation/config (when routines run next).

use hearth_oss::http::{self, Req};
use serde_json::{json, Value};

#[derive(Debug, Clone)]
pub struct Ha {
    pub base: String,
    pub token: String,
}

/// Every room and what's in it, in one answer (Home Assistant's own template functions).
pub const AREAS: &str = "{% set ns = namespace(o=[]) %}{% for a in areas() %}{% set ns.o = ns.o + [{'id': a, 'name': area_name(a), 'entities': area_entities(a)}] %}{% endfor %}{{ ns.o | tojson }}";

impl Ha {
    fn req(&self, m: &str, p: &str) -> Req {
        Req::new(m, &format!("{}{}", self.base, p)).bearer(&self.token).timeout(20)
    }
    pub fn get(&self, p: &str) -> Result<Value, String> {
        let r = http::send(&self.req("GET", p)).map_err(|e| format!("Home Assistant doesn't answer: {e}"))?;
        if r.status == 401 {
            return Err("Home Assistant didn't take the token: make a new one (your profile › Security › Long-lived tokens)".into());
        }
        if !r.ok() {
            return Err(format!("Home Assistant answered {}", r.status));
        }
        r.json().ok_or_else(|| "Home Assistant's answer couldn't be read".into())
    }
    pub fn bytes(&self, p: &str) -> Result<Vec<u8>, String> {
        let r = http::send(&self.req("GET", p)).map_err(|e| e.to_string())?;
        if r.ok() {
            Ok(r.body)
        } else {
            Err(format!("{}", r.status))
        }
    }
    pub fn call(&self, domain: &str, service: &str, data: &Value) -> Result<(), String> {
        let r = http::send(&self.req("POST", &format!("/api/services/{domain}/{service}")).json(data)).map_err(|e| format!("Home Assistant doesn't answer: {e}"))?;
        if r.ok() {
            Ok(())
        } else {
            Err(format!("Home Assistant said no ({})", r.status))
        }
    }
    pub fn template(&self, t: &str) -> Result<String, String> {
        let r = http::send(&self.req("POST", "/api/template").json(&json!({"template": t}))).map_err(|e| e.to_string())?;
        if r.ok() {
            Ok(r.text())
        } else {
            Err(format!("{}", r.status))
        }
    }
}

pub fn attr<'a>(s: &'a Value, k: &str) -> Option<&'a Value> {
    s.pointer(&format!("/attributes/{k}"))
}
pub fn name(s: &Value) -> String {
    attr(s, "friendly_name").and_then(Value::as_str).map(str::to_string).unwrap_or_else(|| id(s).split('.').nth(1).unwrap_or("").replace('_', " "))
}
pub fn id(s: &Value) -> &str {
    s.get("entity_id").and_then(Value::as_str).unwrap_or("")
}
pub fn state(s: &Value) -> &str {
    s.get("state").and_then(Value::as_str).unwrap_or("")
}
pub fn num(s: &Value) -> Option<f64> {
    state(s).parse::<f64>().ok()
}
pub fn domain(s: &Value) -> &str {
    id(s).split('.').next().unwrap_or("")
}

/// A light's words under its name: "warm · 60%", "follows the picture", "off · on at sunset", "on · 2 h".
pub fn light_words(s: &Value, now: i64) -> String {
    if state(s) != "on" {
        let note = attr(s, "hearth_note").and_then(Value::as_str).unwrap_or("");
        return if note.is_empty() { "off".into() } else { format!("off · {note}") };
    }
    if let Some(n) = attr(s, "hearth_note").and_then(Value::as_str).filter(|n| !n.is_empty()) {
        return n.to_string();
    }
    let pct = attr(s, "brightness").and_then(Value::as_f64).map(|b| (b / 255.0 * 100.0).round() as i64);
    let warm = attr(s, "color_temp_kelvin").and_then(Value::as_f64).map(|k| k < 3500.0).unwrap_or(false) || attr(s, "color_mode").and_then(Value::as_str) == Some("color_temp");
    match pct {
        Some(p) if warm => format!("warm · {p}%"),
        Some(p) => format!("{p}%"),
        None => {
            let since = s.get("last_changed").and_then(Value::as_str).map(iso_secs).unwrap_or(0);
            let h = (now - since) / 3600;
            if since > 0 && h >= 1 {
                format!("on · {h} h")
            } else {
                "on".into()
            }
        }
    }
}

/// "2026-11-26T05:24:00.123+00:00" → seconds since 1970.
pub fn iso_secs(s: &str) -> i64 {
    let date = s.get(..10).unwrap_or("");
    let Some((y, m, d)) = hearth_ui::time::parse_date(date) else { return 0 };
    let t = s.get(11..19).unwrap_or("00:00:00");
    let p: Vec<i64> = t.split(':').map(|x| x.parse().unwrap_or(0)).collect();
    let mut secs = days(y as i64, m as i64 + 1, d as i64) * 86_400 + p.first().unwrap_or(&0) * 3600 + p.get(1).unwrap_or(&0) * 60 + p.get(2).unwrap_or(&0);
    // a +hh:mm / -hh:mm after the time
    if let Some(off) = s.get(19..).and_then(|r| r.rfind(['+', '-']).map(|i| &r[i..])) {
        let sign = if off.starts_with('-') { -1 } else { 1 };
        let hm: Vec<i64> = off[1..].split(':').map(|x| x.parse().unwrap_or(0)).collect();
        secs -= sign * (hm.first().unwrap_or(&0) * 3600 + hm.get(1).unwrap_or(&0) * 60);
    }
    secs
}
fn days(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    era * 146_097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719_468
}

/// Energy used each hour today from a total_increasing sensor's history (kWh per hour, 24 of them).
pub fn hourly(history: &Value, day_start: i64) -> Vec<f64> {
    let mut pts: Vec<(i64, f64)> = history.get(0).and_then(Value::as_array).map(|a| a.iter().filter_map(|p| Some((iso_secs(p.get("last_changed")?.as_str()?), p.get("state")?.as_str()?.parse::<f64>().ok()?))).collect()).unwrap_or_default();
    pts.sort_by_key(|p| p.0);
    let mut out = vec![0.0; 24];
    for w in pts.windows(2) {
        let d = w[1].1 - w[0].1;
        if d > 0.0 {
            let h = ((w[1].0 - day_start) / 3600).clamp(0, 23) as usize;
            out[h] += d;
        }
    }
    out
}

/// When a routine runs next: its first time trigger ("05:45:00") after now (seconds since midnight, local).
pub fn next_time(cfg: &Value) -> Option<i64> {
    let trig = cfg.get("triggers").or_else(|| cfg.get("trigger"))?;
    let list = trig.as_array().cloned().unwrap_or_else(|| vec![trig.clone()]);
    for t in list {
        let kind = t.get("trigger").or_else(|| t.get("platform")).and_then(Value::as_str).unwrap_or("");
        if kind == "time" {
            if let Some(at) = t.get("at").and_then(Value::as_str) {
                let p: Vec<i64> = at.split(':').map(|x| x.parse().unwrap_or(0)).collect();
                return Some(p.first().unwrap_or(&0) * 3600 + p.get(1).unwrap_or(&0) * 60);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn home_assistants_states_in_words() {
        let l = json!({"entity_id": "light.ceiling", "state": "on", "attributes": {"friendly_name": "Ceiling", "brightness": 153, "color_mode": "color_temp"}});
        assert_eq!((name(&l), light_words(&l, 0)), ("Ceiling".to_string(), "warm · 60%".to_string()));
        let off = json!({"entity_id": "light.porch", "state": "off", "attributes": {"hearth_note": "on at sunset"}});
        assert_eq!(light_words(&off, 0), "off · on at sunset");
        assert_eq!(iso_secs("1970-01-02T01:00:00+01:00"), 86_400);
        let h = json!([[{"state": "1.0", "last_changed": "1970-01-01T00:10:00+00:00"}, {"state": "1.5", "last_changed": "1970-01-01T01:10:00+00:00"}, {"state": "2.5", "last_changed": "1970-01-01T05:30:00+00:00"}]]);
        let v = hourly(&h, 0);
        assert_eq!((v[1], v[5]), (0.5, 1.0));
        assert_eq!(next_time(&json!({"triggers": [{"trigger": "time", "at": "05:45:00"}]})), Some(5 * 3600 + 45 * 60));
    }
}
