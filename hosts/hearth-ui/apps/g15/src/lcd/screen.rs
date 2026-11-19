//! Screens made of elements, each with a place and a size (the approved mockup g15-6, "Make your own screen"). The
//! built-in pages (g15-5: Clock, System, Now playing, Alerts) are ready-made screens of the same elements, and so are
//! the messages (recording, key set changed, a Hearth alert). Your own screens are saved in g15.json "screens":
//!
//!   {"id": "dads-screen", "name": "Dad's screen", "show": "list" | "hidden", "whileRunning": "Trainz", "elements": [
//!     {"type": "text", "x": 1, "y": 1, "size": 1, "text": "GPU {gpu.temp}°", "w": 0, "invert": false},
//!     {"type": "value", "x": 1, "y": 10, "size": 3, "value": "gpu.temp"},
//!     {"type": "bar", "x": 92, "y": 35, "w": 66, "h": 6, "value": "cpu.use", "min": 0, "max": 100},
//!     {"type": "graph", "x": 86, "y": 2, "w": 74, "h": 30, "value": "gpu.temp", "span": 300, "min": 30, "max": 90,
//!      "style": "line" | "filled"},
//!     {"type": "clock", "x": 1, "y": 34, "size": 1, "format": "24h" | "12h" | "date" | "weekday" | "seconds"},
//!     {"type": "picture", "x": 0, "y": 0, "w": 40, "h": 43, "bits": "<base64: rows of w dots, high bit first>"},
//!     {"type": "line", "x": 0, "y": 34, "x2": 159, "y2": 34}, {"type": "box", "x": 0, "y": 0, "w": 160, "h": 11, "fill": true},
//!     {"type": "tabs"}                                                    the page names over the keys under the screen
//!   ]}
//!
//! "{value}" in a text is replaced by the value now (values.rs VALUES, and song.title, song.artist, song.time,
//! cpu.name, gpu.name, set.name, alert.app, alert.title, alert.body, rec.keys, rec.secs…); "—" when it isn't known.

use super::{fit, font, Frame};
use crate::values::Reading;
use serde_json::{json, Value};
use std::collections::{HashMap, VecDeque};

pub const BUILT_IN: &[&str] = &["clock", "system", "music", "alerts"];

/// What a screen is drawn from.
pub struct Ctx<'a> {
    pub r: &'a Reading,
    pub history: &'a HashMap<String, VecDeque<(u64, f64)>>,
    pub now: i64,
    /// The page names on the tabs, and which is showing.
    pub tabs: Vec<String>,
    pub tab: usize,
}

fn n(v: &Value, k: &str, d: i64) -> i32 {
    v.get(k).and_then(|x| x.as_i64()).unwrap_or(d).clamp(-200, 400) as i32
}
fn f(v: &Value, k: &str, d: f64) -> f64 {
    v.get(k).and_then(|x| x.as_f64()).unwrap_or(d)
}
fn s<'a>(v: &'a Value, k: &str) -> &'a str {
    v.get(k).and_then(|x| x.as_str()).unwrap_or("")
}

/// The unit a value is shown with.
pub fn unit_of(id: &str) -> &'static str {
    crate::values::VALUES.iter().find(|(k, _, _)| *k == id).map(|(_, _, u)| match *u {
        "°" => "°",
        "%" => "%",
        "GB" => " GB",
        "KB/s" => " KB/s",
        "rpm" => " rpm",
        "fps" => " fps",
        _ => "",
    }).unwrap_or("")
}

/// A value as text: whole numbers, one decimal for small ones (GB), "—" when unknown.
pub fn show(r: &Reading, id: &str) -> String {
    if let Some(t) = r.texts.get(id) {
        return t.clone();
    }
    match r.num(id) {
        Some(x) if id.starts_with("mem.") && id != "mem.use" => format!("{:.0}", x),
        Some(x) if x.abs() < 10.0 && x.fract().abs() > 0.05 && (id.ends_with(".free") || id.starts_with("net.")) => format!("{:.1}", x),
        Some(x) => format!("{:.0}", x),
        None => "—".into(),
    }
}

/// "{cpu.use}% {cpu.temp}°" → "23% 48°".
pub fn fill_in(t: &str, r: &Reading) -> String {
    let mut out = String::new();
    let mut rest = t;
    while let Some(i) = rest.find('{') {
        out.push_str(&rest[..i]);
        let after = &rest[i + 1..];
        match after.find('}') {
            Some(j) if j <= 40 => {
                out.push_str(&show(r, &after[..j]));
                rest = &after[j + 1..];
            }
            _ => {
                out.push('{');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

const WDAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

pub fn clock_text(now: i64, format: &str) -> String {
    let t = hearth_ui::time::local(now);
    match format {
        "12h" => format!("{}:{:02}", if t.hour % 12 == 0 { 12 } else { t.hour % 12 }, t.min),
        "seconds" => format!("{:02}:{:02}:{:02}", t.hour, t.min, t.sec),
        "date" => format!("{} {}", hearth_ui::time::month(t.mon), t.day),
        "weekday" => WDAYS[t.wday.min(6)].to_string(),
        "ampm" => (if t.hour < 12 { "AM" } else { "PM" }).to_string(),
        _ => format!("{:02}:{:02}", t.hour, t.min),
    }
}

fn b64(s: &str) -> Vec<u8> {
    let val = |c: u8| -> Option<u32> {
        Some(match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            _ => return None,
        } as u32)
    };
    let mut out = Vec::new();
    let (mut acc, mut bits) = (0u32, 0u32);
    for c in s.bytes() {
        let Some(v) = val(c) else { continue };
        acc = (acc << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    out
}

/// The values a screen uses (so only those are read).
pub fn wanted(elements: &[Value]) -> Vec<String> {
    let mut w: Vec<String> = Vec::new();
    for e in elements {
        if let Some(v) = e.get("value").and_then(|x| x.as_str()) {
            w.push(v.to_string());
        }
        let t = s(e, "text");
        let mut rest = t;
        while let Some(i) = rest.find('{') {
            let after = &rest[i + 1..];
            if let Some(j) = after.find('}') {
                w.push(after[..j].to_string());
                rest = &after[j + 1..];
            } else {
                break;
            }
        }
        if s(e, "type") == "clock" {
            w.push("time".into());
        }
    }
    let mut out: Vec<String> = Vec::new();
    for x in w {
        let k = if x.ends_with(".name") { "names".to_string() } else if x.starts_with("weather") { "weather".to_string() } else { x };
        if !out.contains(&k) {
            out.push(k);
        }
    }
    out
}

/// Does the screen change every second (a graph, seconds, a live value), or only each minute?
pub fn live(elements: &[Value]) -> bool {
    elements.iter().any(|e| {
        let t = s(e, "type");
        t == "graph" || t == "bar" || t == "value" || (t == "clock" && s(e, "format") == "seconds") || (t == "text" && s(e, "text").contains('{') && !s(e, "text").contains("{song.") && !s(e, "text").contains("{alert.") && !s(e, "text").contains("{set.") && !s(e, "text").contains(".name}") && !s(e, "text").contains("{weather."))
    })
}

pub fn draw(fr: &mut Frame, elements: &[Value], c: &Ctx) {
    for e in elements {
        let (x, y) = (n(e, "x", 0), n(e, "y", 0));
        let z = n(e, "size", 1).clamp(1, 6);
        let on = !e.get("clear").and_then(|v| v.as_bool()).unwrap_or(false);
        match s(e, "type") {
            "text" | "value" => {
                let t = if s(e, "type") == "value" {
                    let id = s(e, "value");
                    format!("{}{}", show(c.r, id), if show(c.r, id) == "—" { "" } else { unit_of(id) })
                } else {
                    fill_in(s(e, "text"), c.r)
                };
                let w = n(e, "w", 0);
                let w = if w > 0 { w } else { super::W - x };
                let align = s(e, "align");
                let t = fit(&t, z, w);
                let tx = match align {
                    "center" => x + (w - font::width(&t, z)) / 2,
                    "right" => x + w - font::width(&t, z),
                    _ => x,
                };
                fr.text(&t, tx, y, z, on);
                if e.get("invert").and_then(|v| v.as_bool()).unwrap_or(false) {
                    fr.invert(x, y - 1, w, 7 * z + 2);
                }
            }
            "clock" => {
                fr.text(&clock_text(c.now, s(e, "format")), x, y, z, on);
            }
            "bar" => {
                let (lo, hi) = (f(e, "min", 0.0), f(e, "max", 100.0));
                let v = c.r.num(s(e, "value")).unwrap_or(0.0);
                let frac = if hi > lo { ((v - lo) / (hi - lo)) as f32 } else { 0.0 };
                fr.bar(x, y, n(e, "w", 60), n(e, "h", 6), frac, on);
            }
            "graph" => {
                let (w, h) = (n(e, "w", 74).max(4), n(e, "h", 30).max(4));
                let (lo, hi) = (f(e, "min", 0.0), f(e, "max", 100.0));
                let span = f(e, "span", 300.0).clamp(10.0, 3600.0);
                // The axes, as in the mockup: up the left, along the bottom.
                fr.fill(x, y, 1, h, on);
                fr.fill(x, y + h - 1, w, 1, on);
                let Some(hist) = c.history.get(s(e, "value")) else { continue };
                let now = hist.back().map(|b| b.0).unwrap_or(0);
                let start = now.saturating_sub(span as u64);
                let (gx, gw, gh) = (x + 2, w - 3, h - 3);
                let mut prev: Option<(i32, i32)> = None;
                for col in 0..gw {
                    // The newest value in this column's slice of time.
                    let t0 = start as f64 + span * col as f64 / gw as f64;
                    let t1 = start as f64 + span * (col + 1) as f64 / gw as f64;
                    let v = hist.iter().filter(|(t, _)| (*t as f64) >= t0 && (*t as f64) < t1.max(t0 + 1.0)).map(|p| p.1).last();
                    let Some(v) = v else { continue };
                    let fy = if hi > lo { ((v - lo) / (hi - lo)).clamp(0.0, 1.0) } else { 0.0 };
                    let py = y + 1 + gh - 1 - (fy * (gh - 1) as f64).round() as i32;
                    let px = gx + col;
                    if s(e, "style") == "filled" {
                        fr.fill(px, py, 1, y + h - 1 - py, on);
                    } else if let Some((lx, ly)) = prev {
                        fr.line(lx, ly, px, py, on);
                    } else {
                        fr.dot(px, py, on);
                    }
                    prev = Some((px, py));
                }
            }
            "picture" => {
                let bits = b64(s(e, "bits"));
                fr.picture(x, y, n(e, "w", 0), n(e, "h", 0), &bits);
            }
            "line" => fr.line(x, y, n(e, "x2", x as i64), n(e, "y2", y as i64), on),
            "box" => {
                let (w, h) = (n(e, "w", 10), n(e, "h", 10));
                if e.get("fill").and_then(|v| v.as_bool()).unwrap_or(false) {
                    fr.fill(x, y, w, h, on);
                } else {
                    fr.rect(x, y, w, h, on);
                }
            }
            "disc" => fr.disc(x, y, n(e, "r", 5), on),
            "tabs" => tabs(fr, &c.tabs, c.tab),
            _ => {}
        }
    }
}

/// The page names over the four keys under the screen (the one showing on a dark bar), as in the mockups.
pub fn tabs(fr: &mut Frame, names: &[String], on: usize) {
    for (i, t) in names.iter().take(4).enumerate() {
        let x = i as i32 * 40;
        let t: String = t.chars().take(6).collect();
        let tx = x + 20 - font::width(&t, 1) / 2 - 1;
        if i == on {
            fr.fill(x + 1, 35, 38, 8, true);
            fr.text(&t, tx, 36, 1, false);
        } else {
            fr.fill(x + 1, 34, 38, 1, true);
            fr.text(&t, tx, 36, 1, true);
        }
    }
}

/// The short name on a page's tab.
pub fn tab_name(id: &str, own: &[Value]) -> String {
    match id {
        "clock" => "CLK".into(),
        "system" => "SYS".into(),
        "music" => "MUS".into(),
        "alerts" => "ALR".into(),
        _ => own
            .iter()
            .find(|s| s.get("id").and_then(|v| v.as_str()) == Some(id))
            .and_then(|s| s.get("name").and_then(|v| v.as_str()))
            .map(|n| n.chars().filter(|c| !c.is_whitespace()).take(3).collect::<String>().to_uppercase())
            .unwrap_or_else(|| "?".into()),
    }
}

/// The elements of a built-in page or message (the approved mockup g15-5, dot for dot).
pub fn built_in(id: &str) -> Option<Vec<Value>> {
    let v = match id {
        "clock" => json!([
            {"type": "clock", "x": 1, "y": 2, "size": 4, "format": "24h"},
            {"type": "clock", "x": 123, "y": 2, "format": "weekday"},
            {"type": "clock", "x": 123, "y": 11, "format": "date"},
            {"type": "text", "x": 123, "y": 20, "text": "{weather.temp}{weather.unit} {weather.sky}"},
            {"type": "tabs"}
        ]),
        "system" => json!([
            {"type": "text", "x": 1, "y": 1, "text": "CPU"}, {"type": "bar", "x": 21, "y": 1, "w": 70, "h": 6, "value": "cpu.use"},
            {"type": "text", "x": 95, "y": 1, "text": "{cpu.use}% {cpu.temp}°"},
            {"type": "text", "x": 1, "y": 9, "text": "GPU"}, {"type": "bar", "x": 21, "y": 9, "w": 70, "h": 6, "value": "gpu.use"},
            {"type": "text", "x": 95, "y": 9, "text": "{gpu.use}% {gpu.temp}°"},
            {"type": "text", "x": 1, "y": 17, "text": "RAM"}, {"type": "bar", "x": 21, "y": 17, "w": 70, "h": 6, "value": "mem.use"},
            {"type": "text", "x": 95, "y": 17, "text": "{mem.used}/{mem.total} GB"},
            {"type": "text", "x": 1, "y": 25, "text": "{cpu.name} · {gpu.name}"},
            {"type": "tabs"}
        ]),
        "music" => json!([
            {"type": "text", "x": 1, "y": 1, "size": 2, "text": "♪"},
            {"type": "text", "x": 16, "y": 1, "size": 2, "text": "{song.title}"},
            {"type": "text", "x": 1, "y": 17, "text": "{song.artist}"},
            {"type": "bar", "x": 1, "y": 26, "w": 120, "h": 6, "value": "song.pos"},
            {"type": "text", "x": 127, "y": 26, "text": "{song.time}"},
            {"type": "box", "x": 0, "y": 34, "w": 160, "h": 1, "fill": true},
            {"type": "text", "x": 14, "y": 36, "text": "|◀"}, {"type": "text", "x": 74, "y": 36, "text": "{song.button}"},
            {"type": "text", "x": 134, "y": 36, "text": "▶|"}
        ]),
        // The Alerts page: the last Hearth message, as the pop-up shows it, with the page names under it.
        "alerts" => json!([
            {"type": "box", "x": 0, "y": 0, "w": 160, "h": 11, "fill": true},
            {"type": "text", "x": 3, "y": 2, "w": 150, "text": "{alert.app}", "clear": true},
            {"type": "text", "x": 3, "y": 14, "w": 155, "text": "{alert.title}"},
            {"type": "text", "x": 3, "y": 23, "w": 155, "text": "{alert.body}"},
            {"type": "tabs"}
        ]),
        // A Hearth message popping up for 5 seconds.
        "alert" => json!([
            {"type": "box", "x": 0, "y": 0, "w": 160, "h": 11, "fill": true},
            {"type": "text", "x": 3, "y": 2, "w": 150, "text": "{alert.app}", "clear": true},
            {"type": "text", "x": 3, "y": 15, "w": 155, "text": "{alert.title}"},
            {"type": "text", "x": 3, "y": 25, "w": 155, "text": "{alert.body}"},
            {"type": "text", "x": 146, "y": 36, "text": "{alert.left}s"}
        ]),
        "rec" => json!([
            {"type": "disc", "x": 6, "y": 8, "r": 5},
            {"type": "text", "x": 16, "y": 1, "size": 2, "text": "RECORDING"},
            {"type": "text", "x": 1, "y": 20, "text": "Press a G-key to keep it"},
            {"type": "text", "x": 1, "y": 31, "text": "{rec.keys} keys · {rec.secs} s"}
        ]),
        "set" => json!([
            {"type": "text", "x": 2, "y": 4, "size": 4, "text": "M{set.n}"},
            {"type": "text", "x": 60, "y": 4, "size": 2, "w": 100, "text": "{set.name}"},
            {"type": "text", "x": 60, "y": 24, "w": 100, "text": "{set.keys} G-keys set up"}
        ]),
        _ => return None,
    };
    v.as_array().cloned()
}

/// The elements of a page: built in, or one of your own screens.
pub fn elements_of(id: &str, own: &[Value]) -> Vec<Value> {
    if let Some(b) = built_in(id) {
        return b;
    }
    own.iter()
        .find(|s| s.get("id").and_then(|v| v.as_str()) == Some(id))
        .and_then(|s| s.get("elements").and_then(|e| e.as_array()).cloned())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fills_in_values() {
        let mut r = Reading::default();
        r.nums.insert("cpu.use".into(), 23.4);
        r.nums.insert("cpu.temp".into(), 48.0);
        r.texts.insert("song.title".into(), "Hearthfire".into());
        assert_eq!(fill_in("{cpu.use}% {cpu.temp}°", &r), "23% 48°");
        assert_eq!(fill_in("{gpu.temp}°", &r), "—°");
        assert_eq!(fill_in("a {b", &r), "a {b");
        let w = wanted(&built_in("system").unwrap());
        assert!(w.contains(&"cpu.use".to_string()) && w.contains(&"names".to_string()) && w.contains(&"mem.used".to_string()));
        assert!(live(&built_in("system").unwrap()));
        assert!(!live(&built_in("clock").unwrap()), "the clock page changes once a minute");
        assert_eq!(b64("/w=="), vec![0xff]);
    }
    #[test]
    fn every_built_in_screen_draws() {
        let mut r = Reading::default();
        r.texts.insert("alert.app".into(), "HEARTH".into());
        let h = HashMap::new();
        let c = Ctx { r: &r, history: &h, now: 1_763_500_000, tabs: vec!["CLK".into(), "SYS".into(), "MUS".into(), "ALR".into()], tab: 0 };
        for id in ["clock", "system", "music", "alerts", "alert", "rec", "set"] {
            let mut fr = Frame::new();
            draw(&mut fr, &built_in(id).unwrap(), &c);
            assert!(fr.lit() > 20, "{id} drew nothing");
        }
    }
}
