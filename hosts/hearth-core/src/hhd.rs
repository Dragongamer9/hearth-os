//! Handheld Daemon (HHD): the controller and power service of Hearth OS on Anatase, used when InputPlumber isn't
//! running (Bazzite keeps InputPlumber, unchanged). HHD turns the handheld's buttons into an ordinary virtual
//! controller (Xbox by default, or DualSense / Steam Controller emulation), so the Guide button reaches Hearth
//! through the controller reader (guide.rs) like any pad's. Its local API is a small HTTP server on the socket
//! /run/hhd/api (open to everyone on the device, no token); this reads its state for Settings and the logs.
//! Settings › Controls and the power limit go through desktop/shell/hearth-controller and hearth-power (Python),
//! which use the same API; Settings › Controls › Handheld Daemon (gyro, back buttons, fan, lights) through
//! desktop/shell/hearth-hhd, which only changes the few settings it lists, to values HHD offers for the device.
//! Quick Access over games is HHD's overlay app, which on these editions is Hearth's own (overlay_link.rs): HHD's QAM
//! button opens it, HHD shows it over the game and holds the controllers while it's open.

use serde_json::{json, Value};
use std::io::{Read, Write};
use std::time::Duration;

pub const SOCKET: &str = "/run/hhd/api";

/// An HTTP answer as read off the socket: (status code, body). HHD's server answers HTTP/1.0 and closes.
pub fn parse_response(raw: &str) -> Option<(u16, String)> {
    let (head, body) = raw.split_once("\r\n\r\n").or_else(|| raw.split_once("\n\n"))?;
    let status = head.lines().next()?;
    if !status.starts_with("HTTP/") { return None; }
    let code: u16 = status.split_whitespace().nth(1)?.parse().ok()?;
    Some((code, body.to_string()))
}

/// One request to HHD's API over its socket.
pub fn request(method: &str, path: &str, body: Option<&Value>) -> Result<Value, String> {
    let mut s = std::os::unix::net::UnixStream::connect(SOCKET).map_err(|e| format!("Handheld Daemon isn't answering ({e})"))?;
    let _ = s.set_read_timeout(Some(Duration::from_secs(5)));
    let _ = s.set_write_timeout(Some(Duration::from_secs(5)));
    let data = body.map(|b| b.to_string()).unwrap_or_default();
    let req = format!("{method} {path} HTTP/1.0\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{data}", data.len());
    s.write_all(req.as_bytes()).map_err(|e| e.to_string())?;
    let mut raw = Vec::new();
    s.read_to_end(&mut raw).map_err(|e| e.to_string())?;
    let (code, text) = parse_response(&String::from_utf8_lossy(&raw)).ok_or_else(|| "Handheld Daemon sent something unreadable".to_string())?;
    if code != 200 { return Err(format!("Handheld Daemon answered {code}: {}", text.trim().lines().last().unwrap_or(""))); }
    if text.trim().is_empty() { return Ok(Value::Null); }
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

/// Is Handheld Daemon running here? Extra multi-seat players leave the shared service alone.
pub fn available() -> bool {
    std::env::var_os("HEARTH_SEAT").is_none() && std::path::Path::new(SOCKET).exists() && request("GET", "/api/v1/version", None).is_ok()
}

/// The handheld controllers in HHD's state: (device key, emulation mode), e.g. ("rog_ally", "uinput").
pub fn controllers(state: &Value) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = state["controllers"].as_object().map(|o| o.iter()
        .filter_map(|(k, v)| v["controller_mode"]["mode"].as_str().map(|m| (k.clone(), m.to_string()))).collect()).unwrap_or_default();
    out.sort();
    out
}

/// HHD's power limit: whether it manages it, its range, and its mode (watts only in its custom mode).
pub fn tdp(state: &Value) -> Value {
    let s = &state["hhd"]["steamos"];
    let (driver, t) = state["tdp"].as_object().and_then(|o| o.iter().find(|(_, v)| v["tdp"]["mode"].is_string()).map(|(k, v)| (k.clone(), v["tdp"].clone())))
        .unwrap_or((String::new(), Value::Null));
    let mode = t["mode"].as_str().unwrap_or("").to_string();
    json!({"status": s["tdp_status"], "min": s["tdp_min"], "max": s["tdp_max"], "driver": driver, "mode": mode,
           "watts": if mode == "custom" { t["custom"]["tdp"].clone() } else { Value::Null }})
}

/// One line for the log: what HHD runs.
pub fn describe(state: &Value) -> String {
    let c: Vec<String> = controllers(state).into_iter().map(|(k, m)| format!("{k}: {m}")).collect();
    let t = tdp(state);
    format!("controllers [{}], power limit {}", c.join(", "), t["status"].as_str().unwrap_or("unknown"))
}

/// desktop/shell/hearth-hhd with these arguments: its last line is the JSON answer, "error: ..." a failure.
pub fn helper(args: &[&str]) -> Result<Value, String> {
    let helper = crate::paths::desktop_dir().join("shell/hearth-hhd");
    if !helper.exists() { return Err("Handheld Daemon settings need Hearth, the OS.".into()); }
    let out = std::process::Command::new("python3").arg(&helper).args(args).output().map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&out.stdout);
    let last = text.trim().lines().last().unwrap_or("").to_string();
    if let Some(e) = last.strip_prefix("error: ") { return Err(e.to_string()); }
    if !out.status.success() { return Err(format!("hearth-hhd failed ({})", out.status)); }
    serde_json::from_str(&last).map_err(|_| "Unexpected answer from hearth-hhd".to_string())
}

/// For the bridge (hhd.status): what Settings › Controls › Handheld Daemon shows. Not running: {"running": false}.
pub fn status() -> Value {
    if !std::path::Path::new(SOCKET).exists() || std::env::var_os("HEARTH_SEAT").is_some() { return json!({"running": false}); }
    helper(&["status"]).unwrap_or_else(|e| json!({"running": false, "error": e}))
}

/// A setting's value as hearth-hhd's command line takes it: on/off, or a choice's id. None for anything else.
pub fn value_arg(v: &Value) -> Option<String> {
    match v {
        Value::Bool(b) => Some(if *b { "true".into() } else { "false".into() }),
        Value::String(s) if !s.is_empty() && s.len() <= 64 && s.chars().all(|c| c.is_ascii_alphanumeric() || "_-".contains(c)) => Some(s.clone()),
        _ => None,
    }
}

/// A dotted HHD key Hearth could ask about (hearth-hhd decides whether it may be changed).
pub fn valid_key(k: &str) -> bool {
    !k.is_empty() && k.len() <= 160 && k.split('.').count() >= 3 && k.split('.').all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_alphanumeric() || "_-".contains(c)))
}

/// For the bridge (hhd.set): changes one of the extra controls, then answers with the new status.
pub fn set(key: &str, value: &Value) -> Result<Value, String> {
    if !valid_key(key) { return Err("Unknown setting".into()); }
    let v = value_arg(value).ok_or("That isn't a value this setting takes")?;
    helper(&["set", key, v.as_str()])
}

/// For the logs report.
pub fn report() -> String {
    if !std::path::Path::new(SOCKET).exists() { return "not running (no /run/hhd/api)".into(); }
    match request("GET", "/api/v1/state", None) {
        Ok(st) => format!("{}\ncontrollers: {}\npower limit: {}", describe(&st), json!(controllers(&st)), tdp(&st)),
        Err(e) => e,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reads_answers() {
        assert_eq!(parse_response("HTTP/1.0 200 OK\r\nContent-type: application/json\r\n\r\n{\"version\": 5}"), Some((200, "{\"version\": 5}".into())));
        assert_eq!(parse_response("HTTP/1.0 401 Unauthorized\r\n\r\nno"), Some((401, "no".into())));
        assert_eq!(parse_response("HTTP/1.0 200 OK\r\n\r\n"), Some((200, String::new())));
        assert_eq!(parse_response("garbage"), None);
        assert_eq!(parse_response("SSH-2.0\r\n\r\nx"), None);
    }
    #[test]
    fn checks_what_goes_to_the_helper() {
        assert_eq!(value_arg(&json!(true)).as_deref(), Some("true"));
        assert_eq!(value_arg(&json!(false)).as_deref(), Some("false"));
        assert_eq!(value_arg(&json!("steam_input")).as_deref(), Some("steam_input"));
        assert_eq!(value_arg(&json!("rm -rf")), None);
        assert_eq!(value_arg(&json!("")), None);
        assert_eq!(value_arg(&json!(5)), None);
        assert!(valid_key("controllers.rog_ally.imu"));
        assert!(valid_key("rgb.handheld.mode.mode"));
        assert!(!valid_key("controllers..imu"));
        assert!(!valid_key("hhd.x"));
        assert!(!valid_key("controllers.rog ally.imu"));
    }
    #[test]
    fn reads_state() {
        let st = json!({
            "controllers": {"rog_ally": {"controller_mode": {"mode": "uinput", "uinput": {"paddles_as": "noob"}}, "nintendo_mode": false}, "other": 3},
            "hhd": {"steamos": {"tdp_status": "enabled", "tdp_min": 5, "tdp_max": 30}},
            "tdp": {"unified": {"tdp": {"mode": "custom", "custom": {"tdp": 17}}}}
        });
        assert_eq!(controllers(&st), vec![("rog_ally".to_string(), "uinput".to_string())]);
        let t = tdp(&st);
        assert_eq!(t["watts"], 17);
        assert_eq!(t["driver"], "unified");
        assert_eq!(t["max"], 30);
        let st2 = json!({"tdp": {"unified": {"tdp": {"mode": "balanced", "custom": {"tdp": 17}}}}});
        assert!(tdp(&st2)["watts"].is_null());
        assert!(controllers(&json!({})).is_empty());
        assert!(describe(&st).contains("rog_ally: uinput"));
    }
}
