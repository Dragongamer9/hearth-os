//! This PC as the phone's keyboard and touchpad (KDE Connect's "Remote keyboard" and the phone's mouse receiver,
//! the other way round from input.rs): Settings › Phone's "Use this PC's keyboard and mouse on the phone" sends
//!   kdeconnect.mousepad.request {"key": "a", "shift"/"ctrl"/"alt": bool, "sendAck": true}  a character
//!   kdeconnect.mousepad.request {"specialKey": 12, ...}                                     Enter, arrows, Backspace…
//!   kdeconnect.mousepad.request {"dx", "dy"} / {"singleclick": true} / {"scroll": true, "dy"} the pointer
//! The phone types into the field it has open when its "KDE Connect Remote Keyboard" is the keyboard in use (it says
//! so with kdeconnect.mousepad.keyboardstate, shown on the page), and moves a pointer with the app's mouse receiver.
//! Only to a paired phone that says it takes these (its incomingCapabilities).

use crate::daemon::Daemon;
use serde_json::{json, Value};
use std::sync::Arc;

/// KDE Connect's special keys by name (the same numbering input.rs reads from phones).
pub fn special(name: &str) -> Option<i64> {
    Some(match name {
        "Backspace" => 1, "Tab" => 2, "Left" => 4, "Up" => 5, "Right" => 6, "Down" => 7, "PageUp" => 8, "PageDown" => 9,
        "Home" => 10, "End" => 11, "Enter" | "Return" => 12, "Delete" => 13, "Escape" => 14,
        f if f.len() >= 2 && f.starts_with('F') => { let n: i64 = f[1..].parse().ok()?; if (1..=12).contains(&n) { 20 + n } else { return None } }
        _ => return None,
    })
}

fn ready(d: &Daemon, id: &str) -> Result<(), String> {
    if d.link_of(id).is_none() { return Err("The phone isn't connected right now".into()); }
    if !d.peer_accepts(id, "kdeconnect.mousepad.request") {
        return Err("Turn on Remote keyboard (and Mouse receiver) in KDE Connect on the phone first".into());
    }
    Ok(())
}

/// One key: {"key": "a"} or {"special": "Enter" | 12}, with "shift", "ctrl", "alt".
pub fn key_body(a: &Value) -> Result<Value, String> {
    let mut b = json!({"sendAck": true});
    for m in ["shift", "ctrl", "alt"] { if a.get(m).and_then(|v| v.as_bool()).unwrap_or(false) { b[m] = json!(true); } }
    let sp = a.get("special").and_then(|v| v.as_i64().or_else(|| v.as_str().and_then(special)));
    if let Some(n) = sp {
        if !(1..=32).contains(&n) { return Err("unknown key".into()); }
        b["specialKey"] = json!(n);
        return Ok(b);
    }
    let k = a.get("key").and_then(|v| v.as_str()).unwrap_or("");
    let mut chars = k.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) if !c.is_control() => { b["key"] = json!(c.to_string()); Ok(b) }
        _ => Err("one character at a time (or a special key)".into()),
    }
}

pub fn key(d: &Arc<Daemon>, id: &str, a: &Value) -> Result<Value, String> {
    ready(d, id)?;
    d.send_body(id, "kdeconnect.mousepad.request", key_body(a)?)?;
    Ok(json!({}))
}

/// Typed text, a key at a time (new lines as Enter). At most 2000 characters a go.
pub fn text(d: &Arc<Daemon>, id: &str, t: &str) -> Result<Value, String> {
    ready(d, id)?;
    let mut n = 0;
    for c in t.chars().take(2000) {
        let body = match c {
            '\n' => key_body(&json!({"special": 12}))?,
            '\t' => key_body(&json!({"special": 2}))?,
            c if c.is_control() => continue,
            c => key_body(&json!({"key": c.to_string()}))?,
        };
        d.send_body(id, "kdeconnect.mousepad.request", body)?;
        n += 1;
    }
    Ok(json!({"sent": n}))
}

/// The pointer: {"dx", "dy"} (a move), {"click": "single" | "double" | "middle" | "right" | "hold" | "release"},
/// {"scroll": dy}.
pub fn mouse_body(a: &Value) -> Result<Value, String> {
    if let Some(c) = a.get("click").and_then(|v| v.as_str()) {
        let k = match c { "single" => "singleclick", "double" => "doubleclick", "middle" => "middleclick", "right" => "rightclick",
            "hold" => "singlehold", "release" => "singlerelease", _ => return Err("unknown click".into()) };
        let mut m = serde_json::Map::new();
        m.insert(k.to_string(), json!(true));
        return Ok(Value::Object(m));
    }
    if let Some(s) = a.get("scroll").and_then(|v| v.as_f64()) {
        return Ok(json!({"scroll": true, "dx": 0.0, "dy": s.clamp(-200.0, 200.0)}));
    }
    let dx = a.get("dx").and_then(|v| v.as_f64()).unwrap_or(0.0).clamp(-500.0, 500.0);
    let dy = a.get("dy").and_then(|v| v.as_f64()).unwrap_or(0.0).clamp(-500.0, 500.0);
    if dx == 0.0 && dy == 0.0 { return Err("nothing to move".into()); }
    Ok(json!({"dx": dx, "dy": dy}))
}

pub fn mouse(d: &Arc<Daemon>, id: &str, a: &Value) -> Result<Value, String> {
    ready(d, id)?;
    d.send_body(id, "kdeconnect.mousepad.request", mouse_body(a)?)?;
    Ok(json!({}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_for_the_phone() {
        assert_eq!(key_body(&json!({"key": "a"})).unwrap(), json!({"key": "a", "sendAck": true}));
        assert_eq!(key_body(&json!({"key": "C", "ctrl": true})).unwrap(), json!({"key": "C", "ctrl": true, "sendAck": true}));
        assert_eq!(key_body(&json!({"special": "Enter"})).unwrap()["specialKey"], 12);
        assert_eq!(key_body(&json!({"special": "Backspace", "shift": true})).unwrap(), json!({"specialKey": 1, "shift": true, "sendAck": true}));
        assert_eq!(key_body(&json!({"special": "F5"})).unwrap()["specialKey"], 25);
        assert_eq!(key_body(&json!({"special": 6})).unwrap()["specialKey"], 6);
        assert!(key_body(&json!({"key": "ab"})).is_err());
        assert!(key_body(&json!({"special": "F13"})).is_err());
        // The same numbers input.rs reads from a phone: Enter is KEY_ENTER (28) there.
        assert_eq!(crate::input::SPECIAL_KEYS[special("Enter").unwrap() as usize], 28);
        assert_eq!(crate::input::SPECIAL_KEYS[special("Escape").unwrap() as usize], 1);
    }

    #[test]
    fn the_pointer_for_the_phone() {
        assert_eq!(mouse_body(&json!({"dx": 3.5, "dy": -2})).unwrap(), json!({"dx": 3.5, "dy": -2.0}));
        assert_eq!(mouse_body(&json!({"click": "right"})).unwrap(), json!({"rightclick": true}));
        assert_eq!(mouse_body(&json!({"scroll": -4})).unwrap()["scroll"], true);
        assert!(mouse_body(&json!({})).is_err());
        assert!(mouse_body(&json!({"click": "triple"})).is_err());
    }
}
