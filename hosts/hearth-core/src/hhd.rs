//! Handheld Daemon (HHD): the controller and power service of Hearth OS on Anatase, used when InputPlumber isn't
//! running (Bazzite keeps InputPlumber, unchanged). HHD turns the handheld's buttons into an ordinary virtual
//! controller (Xbox by default, or DualSense / Steam Controller emulation), so the Guide button reaches Hearth
//! through the controller reader (guide.rs) like any pad's. Its local API is a small HTTP server on the socket
//! /run/hhd/api (open to everyone on the device, no token); this reads its state for Settings and the logs.
//! Settings › Controls and the power limit go through desktop/shell/hearth-controller and hearth-power (Python),
//! which use the same API.
//! Settings › Controls › Handheld Daemon is a whole page: every setting HHD has for this device, drawn from the
//! settings tree HHD serves (/api/v1/settings: containers, modes, switches, choices, sliders, buttons, read-only
//! lines), with the values in its state (/api/v1/state). It replaces HHD's own settings app (hhd-ui), which these
//! editions don't ship. Changes are checked here against that tree (the kind of value, its choices, its limits)
//! before they go to HHD, and only for settings the page shows (never HHD's hidden ones).
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

/// One request to HHD's API over its socket: the answer's text, as HHD wrote it.
pub fn request_text(method: &str, path: &str, body: Option<&Value>, timeout: Duration) -> Result<String, String> {
    let mut s = std::os::unix::net::UnixStream::connect(SOCKET).map_err(|e| format!("Handheld Daemon isn't answering ({e})"))?;
    let _ = s.set_read_timeout(Some(timeout));
    let _ = s.set_write_timeout(Some(Duration::from_secs(5)));
    let data = body.map(|b| b.to_string()).unwrap_or_default();
    let req = format!("{method} {path} HTTP/1.0\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{data}", data.len());
    s.write_all(req.as_bytes()).map_err(|e| e.to_string())?;
    let mut raw = Vec::new();
    s.read_to_end(&mut raw).map_err(|e| e.to_string())?;
    let (code, text) = parse_response(&String::from_utf8_lossy(&raw)).ok_or_else(|| "Handheld Daemon sent something unreadable".to_string())?;
    if code != 200 { return Err(format!("Handheld Daemon answered {code}: {}", text.trim().lines().last().unwrap_or(""))); }
    Ok(text)
}

/// One request to HHD's API over its socket, read as JSON (nothing: null).
pub fn request(method: &str, path: &str, body: Option<&Value>) -> Result<Value, String> {
    request_with(method, path, body, Duration::from_secs(5))
}

/// The same, waiting up to `timeout` for the answer (a change is answered after HHD's next loop has applied it).
pub fn request_with(method: &str, path: &str, body: Option<&Value>, timeout: Duration) -> Result<Value, String> {
    let text = request_text(method, path, body, timeout)?;
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

/// A dotted HHD key Hearth could ask about: section, the plugin's entry, then the setting (e.g.
/// controllers.rog_ally.imu, or tdp.unified.tdp.mode for a mode's choice).
pub fn valid_key(k: &str) -> bool {
    !k.is_empty() && k.len() <= 160 && k.split('.').count() >= 3 && k.split('.').all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_alphanumeric() || "_-".contains(c)))
}

// ---------------------------------------------------------------- HHD's settings tree (the whole page)

/// Tags that keep a setting off Hearth's page, and out of its reach: HHD's hidden ones (hhd-ui hides the same), and
/// hhd-ui's own look (its colour theme), which isn't Hearth's.
pub const SKIP_TAGS: &[&str] = &["hidden", "hhd-update-decky", "hhd-version-display-decky", "theme-selector"];

/// Does this entry of HHD's settings tree carry the tag?
pub fn has_tag(node: &Value, tag: &str) -> bool {
    node["tags"].as_array().map(|a| a.iter().any(|t| t.as_str() == Some(tag))).unwrap_or(false)
}

/// Left off Hearth's page (see SKIP_TAGS).
pub fn skipped(node: &Value) -> bool {
    SKIP_TAGS.iter().any(|t| has_tag(node, t))
}

/// The settings tree's version (HHD adds it to the tree as hhd.version; its state carries the same as "version"):
/// when the state's differs, HHD has changed the tree (a controller mode with other options, a plugin that
/// started), and the page reads the tree again.
pub fn schema_version(sets: &Value) -> Value {
    sets["hhd"]["version"]["value"].clone()
}

/// Where a dotted state key sits in HHD's settings tree: the entry, and whether the key is a mode's choice
/// ("<mode>.mode", HHD's own spelling). The first two parts are the section and the plugin's entry; below that,
/// containers have "children" and modes have "modes". None for keys the tree doesn't have, for containers and modes
/// themselves (only their settings have values), and for anything Hearth's page leaves out, there or above it.
pub fn find<'a>(sets: &'a Value, key: &str) -> Option<(&'a Value, bool)> {
    let parts: Vec<&str> = key.split('.').collect();
    if parts.len() < 3 { return None; }
    let mut node = sets.get(parts[0])?.get(parts[1])?;
    if !node.is_object() || skipped(node) { return None; }
    let rest = &parts[2..];
    for (i, part) in rest.iter().enumerate() {
        node = match node["type"].as_str()? {
            "mode" if *part == "mode" && i == rest.len() - 1 => return Some((node, true)),
            "mode" => node["modes"].get(*part)?,
            "container" => node["children"].get(*part)?,
            _ => return None,
        };
        if !node.is_object() || skipped(node) { return None; }
    }
    match node["type"].as_str() {
        None | Some("container") | Some("mode") => None,
        Some(_) => Some((node, false)),
    }
}

/// A value somewhere in HHD's (nested) state, by dotted key; null when there's none.
pub fn lookup<'a>(state: &'a Value, key: &str) -> &'a Value {
    key.split('.').fold(state, |v, p| &v[p])
}

/// A setting's name for messages: its title, else its key's last part.
fn name_of(node: &Value, key: &str) -> String {
    node["title"].as_str().map(str::trim).filter(|t| !t.is_empty()).map(str::to_string)
        .unwrap_or_else(|| key.rsplit('.').next().unwrap_or(key).replace('_', " "))
}

/// Two JSON numbers that are the same number (15 and 15.0).
fn same_number(a: &Value, b: &Value) -> bool {
    matches!((a.as_f64(), b.as_f64()), (Some(x), Some(y)) if (x - y).abs() < 1e-9)
}

/// A number the way the page shows it: 15, or 0.5.
fn show_number(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 { format!("{}", n as i64) } else { format!("{n}") }
}

/// Checks a number against the setting's limits (HHD's min and max; either may be missing).
fn in_range(node: &Value, n: f64, name: &str) -> Result<(), String> {
    if let Some(min) = node["min"].as_f64() { if n < min { return Err(format!("{name} can't be lower than {}", show_number(min))); } }
    if let Some(max) = node["max"].as_f64() { if n > max { return Err(format!("{name} can't be higher than {}", show_number(max))); } }
    Ok(())
}

/// The value to send HHD for a setting (an entry found with `find`), or why it isn't one the setting takes:
///   a mode's choice: one of its modes;  bool: on or off;  action (a button): true, which runs it (HHD sets it back);
///   multiple: one of its option ids;  discrete: one of its numbers;  int: a whole number within min..max;
///   float: a number within min..max;  custom with the "dropdown" tag: one of the choices its value lists now
///   (`current`; HHD wants the choices back with the pick).
/// Read-only kinds (display, progress, version) and anything else can't be changed.
pub fn check_value(node: &Value, is_mode: bool, value: &Value, current: &Value, key: &str) -> Result<Value, String> {
    let name = name_of(node, key);
    let not_a_choice = || format!("That isn't one of the choices for {name}");
    if is_mode {
        let id = value.as_str().ok_or_else(not_a_choice)?;
        return match node["modes"].get(id) {
            Some(m) if m.is_object() && !skipped(m) => Ok(json!(id)),
            _ => Err(not_a_choice()),
        };
    }
    match node["type"].as_str().unwrap_or("") {
        "bool" => value.as_bool().map(Value::Bool).ok_or_else(|| format!("{name} is either on or off")),
        "action" => if value == &Value::Bool(true) { Ok(Value::Bool(true)) } else { Err(format!("{name} is a button: it only runs")) },
        "multiple" => match value.as_str() {
            Some(s) if node["options"].get(s).is_some() => Ok(json!(s)),
            _ => Err(not_a_choice()),
        },
        "discrete" => node["options"].as_array().and_then(|o| o.iter().find(|o| same_number(o, value) || (o.is_string() && *o == value)))
            .cloned().ok_or_else(not_a_choice),
        "int" => {
            let n = value.as_f64().filter(|n| n.is_finite() && n.fract() == 0.0).ok_or_else(|| format!("{name} takes a whole number"))?;
            in_range(node, n, &name)?;
            Ok(json!(n as i64))
        }
        "float" => {
            let n = value.as_f64().filter(|n| n.is_finite()).ok_or_else(|| format!("{name} takes a number"))?;
            in_range(node, n, &name)?;
            Ok(json!(n))
        }
        "custom" if has_tag(node, "dropdown") => match value.as_str() {
            Some(s) if current["options"].get(s).is_some() => Ok(json!({"options": current["options"].clone(), "value": s})),
            _ => Err(not_a_choice()),
        },
        _ => Err(format!("{name} can't be changed here")),
    }
}

/// Is Handheld Daemon's socket here, for this player? (Extra multi-seat players leave the shared service alone.)
fn reachable() -> bool {
    std::env::var_os("HEARTH_SEAT").is_none() && std::path::Path::new(SOCKET).exists()
}

/// For the bridge (hhd.settings): everything the Handheld Daemon page draws. The tree goes to the page as HHD wrote
/// it ("schema", JSON text: its order is the order HHD shows things in, which a parsed map here wouldn't keep),
/// with the section names (/api/v1/sections), the state (nested, as HHD has it) and the tree's version.
/// Not running: {"running": false}.
pub fn settings() -> Result<Value, String> {
    if !reachable() { return Ok(json!({"running": false})); }
    let raw = request_text("GET", "/api/v1/settings", None, Duration::from_secs(8))?;
    let sets: Value = serde_json::from_str(&raw).map_err(|_| "Handheld Daemon sent settings Hearth can't read".to_string())?;
    let state = request("GET", "/api/v1/state", None)?;
    let sections = request("GET", "/api/v1/sections", None).unwrap_or(Value::Null);
    Ok(json!({"running": true, "version": schema_version(&sets), "schema": raw, "sections": sections, "state": state}))
}

/// For the bridge (hhd.state): the state alone, for the page to stay current while it's open (the Quick Access
/// power limit, HHD's own shortcuts and its controller modes change things too).
pub fn state() -> Result<Value, String> {
    if !reachable() { return Ok(json!({"running": false})); }
    let st = request("GET", "/api/v1/state", None)?;
    Ok(json!({"running": true, "version": st["version"].clone(), "state": st}))
}

/// Changes any setting the page shows, after checking the value against HHD's settings tree (see check_value).
/// HHD applies it in its next loop and answers with its new state: {running, version, state}.
pub fn write(key: &str, value: &Value) -> Result<Value, String> {
    if !valid_key(key) { return Err("Unknown setting".into()); }
    if !reachable() { return Err("Handheld Daemon isn't running".into()); }
    let sets = request_with("GET", "/api/v1/settings", None, Duration::from_secs(8))?;
    let (node, is_mode) = find(&sets, key).ok_or("Handheld Daemon doesn't have that setting on this device")?;
    let current = if node["type"] == "custom" { lookup(&request("GET", "/api/v1/state", None)?, key).clone() } else { Value::Null };
    let v = check_value(node, is_mode, value, &current, key)?;
    let mut change = serde_json::Map::new();
    change.insert(key.to_string(), v);
    let st = request_with("POST", "/api/v1/state", Some(&Value::Object(change)), Duration::from_secs(15))?;
    Ok(json!({"running": true, "version": st["version"].clone(), "state": st}))
}

/// For the bridge (hhd.set): changes one setting (any the page shows, checked as in `write`), then answers with
/// the short status hhd.status gives (what Settings › Controls has always got back).
pub fn set(key: &str, value: &Value) -> Result<Value, String> {
    write(key, value)?;
    Ok(status())
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
    fn checks_keys() {
        assert!(valid_key("controllers.rog_ally.imu"));
        assert!(valid_key("rgb.handheld.mode.mode"));
        assert!(valid_key("tdp.unified.tdp.low-power.x"));
        assert!(!valid_key("controllers..imu"));
        assert!(!valid_key("hhd.x"));
        assert!(!valid_key("controllers.rog ally.imu"));
        assert!(!valid_key("controllers.rog_ally.imu\nx"));
        assert!(!valid_key(&format!("a.b.{}", "c".repeat(200))));
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

    /// A small tree in HHD's shape (what merge_settings serves: every entry has type, tags, title; containers have
    /// children, modes have modes), from the ROG Ally's, the unified power driver's and HHD's own settings.
    fn tree() -> Value {
        // Parsed from text: a json! literal this big can run past the compiler's macro depth.
        serde_json::from_str(r#"{
            "controllers": {"rog_ally": {"type": "container", "tags": ["lgc"], "title": "Ally Controller", "children": {
                "controller_mode": {"type": "mode", "tags": ["rog_controller_mode"], "title": "Controller Emulation", "default": "uinput", "modes": {
                    "uinput": {"type": "container", "tags": [], "title": "Xbox", "children": {
                        "paddles_as": {"type": "multiple", "tags": [], "title": "Back buttons", "options": {"steam_input": "Steam Input", "noob": "Keyboard", "disabled": "Disabled"}}}},
                    "dualsense": {"type": "container", "tags": [], "title": "Dualsense", "children": {}},
                    "secret": {"type": "container", "tags": ["hidden"], "title": "Secret", "children": {}}}},
                "imu": {"type": "bool", "tags": [], "title": "Motion Support", "default": true},
                "imu_hz": {"type": "discrete", "tags": ["non-essential"], "title": "Motion Hz", "options": [100, 200, 400, 800], "default": 200},
                "limits": {"type": "mode", "tags": [], "title": "Stick limits", "modes": {"manual": {"type": "container", "tags": [], "title": "Manual", "children": {
                    "ls_min": {"type": "int", "tags": ["slim"], "title": "LS Min", "min": 0, "max": 95, "step": 1},
                    "reset": {"type": "action", "tags": [], "title": "Reset"}}}}},
                "rgb_boot": {"type": "bool", "tags": ["advanced"], "title": "RGB During Boot"},
                "inner": {"type": "container", "tags": ["hidden"], "title": "Hidden box", "children": {
                    "x": {"type": "bool", "tags": [], "title": "X"}}}}}},
            "tdp": {"unified": {"type": "container", "tags": ["hide-title"], "title": "Power Controls", "children": {
                "tdp": {"type": "mode", "tags": [], "title": "Power Mode", "default": "balanced", "modes": {
                    "balanced": {"type": "container", "tags": [], "title": "Performance", "unit": "17W", "children": {}},
                    "custom": {"type": "container", "tags": [], "title": "Custom", "children": {
                        "tdp": {"type": "int", "tags": [], "title": "TDP", "min": 5, "max": 30, "step": 1, "unit": "W"},
                        "boost": {"type": "bool", "tags": [], "title": "TDP Boost"}}}}},
                "sys_tdp": {"type": "display", "tags": [], "title": " "},
                "scale": {"type": "float", "tags": [], "title": "Scale", "min": 0.5, "max": 2.0},
                "open": {"type": "float", "tags": [], "title": "Open ended"},
                "fan_full_speed": {"type": "bool", "tags": [], "title": "Set Fan to Full Speed"}}}},
            "hhd": {
                "settings": {"type": "container", "tags": [], "title": "Core Settings", "children": {
                    "theme": {"type": "multiple", "tags": ["theme-selector", "advanced"], "title": "Theme", "options": {"default": "Default"}},
                    "reset": {"type": "action", "tags": ["reset-button", "verify"], "title": "Reset Settings"},
                    "boot": {"type": "custom", "tags": ["dropdown"], "title": "Logs from"},
                    "progress": {"type": "custom", "tags": ["progress"]}}},
                "steamos": {"type": "container", "tags": ["hidden"], "children": {
                    "tdp_status": {"type": "multiple", "tags": ["hidden"], "options": {"enabled": ""}}}},
                "version": {"type": "version", "tags": ["non-essential", "advanced", "expert", "hide"], "value": "a1b2c3d4"}}
        }"#).unwrap()
    }
    #[test]
    fn finds_settings_in_the_tree() {
        let t = tree();
        let (n, m) = find(&t, "controllers.rog_ally.imu").unwrap();
        assert_eq!(n["type"], "bool"); assert!(!m);
        let (n, m) = find(&t, "controllers.rog_ally.controller_mode.mode").unwrap();
        assert_eq!(n["type"], "mode"); assert!(m);
        assert_eq!(find(&t, "controllers.rog_ally.controller_mode.uinput.paddles_as").unwrap().0["type"], "multiple");
        assert_eq!(find(&t, "controllers.rog_ally.limits.manual.reset").unwrap().0["type"], "action");
        assert_eq!(find(&t, "tdp.unified.tdp.custom.tdp").unwrap().0["max"], 30);
        assert!(find(&t, "tdp.unified.tdp.mode").unwrap().1);
        // Not settings: containers, modes themselves, keys the tree doesn't have, too short.
        assert!(find(&t, "controllers.rog_ally.controller_mode").is_none());
        assert!(find(&t, "controllers.rog_ally.controller_mode.uinput").is_none());
        assert!(find(&t, "controllers.rog_ally.nope").is_none());
        assert!(find(&t, "controllers.legion_go.imu").is_none());
        assert!(find(&t, "controllers.rog_ally").is_none());
        assert!(find(&t, "controllers.rog_ally.imu.more").is_none());
        // "mode" is the choice only as the last part.
        assert!(find(&t, "controllers.rog_ally.controller_mode.mode.x").is_none());
        // Hidden ones, and everything under a hidden entry, are out of reach; so is hhd-ui's theme.
        assert!(find(&t, "hhd.steamos.tdp_status").is_none());
        assert!(find(&t, "controllers.rog_ally.inner.x").is_none());
        assert!(find(&t, "controllers.rog_ally.controller_mode.secret.anything").is_none());
        assert!(find(&t, "hhd.settings.theme").is_none());
        // HHD's version entry is no setting.
        assert!(find(&t, "hhd.version.value").is_none());
        assert_eq!(schema_version(&t), "a1b2c3d4");
    }
    #[test]
    fn checks_values_against_the_tree() {
        let t = tree();
        let chk = |k: &str, v: Value| { let (n, m) = find(&t, k).unwrap(); check_value(n, m, &v, &Value::Null, k) };
        // Switches.
        assert_eq!(chk("controllers.rog_ally.imu", json!(false)), Ok(json!(false)));
        assert!(chk("controllers.rog_ally.imu", json!("yes")).unwrap_err().contains("on or off"));
        assert!(chk("controllers.rog_ally.imu", json!(1)).is_err());
        // Mode choices: only the modes it has, not hidden ones.
        assert_eq!(chk("controllers.rog_ally.controller_mode.mode", json!("dualsense")), Ok(json!("dualsense")));
        assert!(chk("controllers.rog_ally.controller_mode.mode", json!("hori_steam")).is_err());
        assert!(chk("controllers.rog_ally.controller_mode.mode", json!("secret")).is_err());
        assert!(chk("controllers.rog_ally.controller_mode.mode", json!(3)).is_err());
        // Choices: the option ids, not their labels.
        assert_eq!(chk("controllers.rog_ally.controller_mode.uinput.paddles_as", json!("noob")), Ok(json!("noob")));
        assert!(chk("controllers.rog_ally.controller_mode.uinput.paddles_as", json!("Keyboard")).unwrap_err().contains("Back buttons"));
        // Discrete numbers: one of them (15 and 15.0 are the same), sent as HHD lists it.
        assert_eq!(chk("controllers.rog_ally.imu_hz", json!(400)), Ok(json!(400)));
        assert_eq!(chk("controllers.rog_ally.imu_hz", json!(400.0)), Ok(json!(400)));
        assert!(chk("controllers.rog_ally.imu_hz", json!(300)).is_err());
        assert!(chk("controllers.rog_ally.imu_hz", json!("400")).is_err());
        // Whole numbers within HHD's limits.
        assert_eq!(chk("tdp.unified.tdp.custom.tdp", json!(17)), Ok(json!(17)));
        assert_eq!(chk("tdp.unified.tdp.custom.tdp", json!(17.0)), Ok(json!(17)));
        assert_eq!(chk("tdp.unified.tdp.custom.tdp", json!(5)), Ok(json!(5)));
        assert_eq!(chk("tdp.unified.tdp.custom.tdp", json!(30)), Ok(json!(30)));
        assert_eq!(chk("tdp.unified.tdp.custom.tdp", json!(31)).unwrap_err(), "TDP can't be higher than 30");
        assert_eq!(chk("tdp.unified.tdp.custom.tdp", json!(4)).unwrap_err(), "TDP can't be lower than 5");
        assert!(chk("tdp.unified.tdp.custom.tdp", json!(17.5)).unwrap_err().contains("whole number"));
        assert!(chk("tdp.unified.tdp.custom.tdp", json!("17")).is_err());
        assert_eq!(chk("controllers.rog_ally.limits.manual.ls_min", json!(0)), Ok(json!(0)));
        // Numbers within limits; without limits, any number.
        assert_eq!(chk("tdp.unified.scale", json!(1.25)), Ok(json!(1.25)));
        assert_eq!(chk("tdp.unified.scale", json!(2.5)).unwrap_err(), "Scale can't be higher than 2");
        assert_eq!(chk("tdp.unified.scale", json!(0.25)).unwrap_err(), "Scale can't be lower than 0.5");
        assert_eq!(chk("tdp.unified.open", json!(-3)), Ok(json!(-3.0)));
        assert!(chk("tdp.unified.open", json!(null)).is_err());
        // Buttons: run with true, nothing else.
        assert_eq!(chk("hhd.settings.reset", json!(true)), Ok(json!(true)));
        assert!(chk("hhd.settings.reset", json!(false)).is_err());
        assert!(chk("controllers.rog_ally.limits.manual.reset", json!("go")).is_err());
        // Read-only lines and progress can't be changed.
        assert!(chk("tdp.unified.sys_tdp", json!("x")).unwrap_err().contains("can't be changed"));
        assert!(chk("hhd.settings.progress", json!({"value": 1})).is_err());
        // A titleless entry is named by its key in messages.
        assert!(chk("tdp.unified.sys_tdp", json!("x")).unwrap_err().starts_with("sys tdp"));
    }
    #[test]
    fn checks_a_dropdown_against_its_current_choices() {
        let t = tree();
        let (n, m) = find(&t, "hhd.settings.boot").unwrap();
        let cur = json!({"options": {"current": "Current Boot", "previous": "Previous Boot"}, "value": "current"});
        assert_eq!(check_value(n, m, &json!("previous"), &cur, "hhd.settings.boot"),
                   Ok(json!({"options": {"current": "Current Boot", "previous": "Previous Boot"}, "value": "previous"})));
        assert!(check_value(n, m, &json!("m9"), &cur, "hhd.settings.boot").is_err());
        assert!(check_value(n, m, &json!("current"), &Value::Null, "hhd.settings.boot").is_err());
    }
    #[test]
    fn reads_tags_and_values() {
        let t = tree();
        assert!(has_tag(&t["hhd"]["settings"]["children"]["reset"], "verify"));
        assert!(!has_tag(&t["hhd"]["settings"]["children"]["reset"], "hidden"));
        assert!(!has_tag(&json!({"title": "no tags"}), "hidden"));
        assert!(skipped(&t["hhd"]["steamos"]));
        assert!(skipped(&t["hhd"]["settings"]["children"]["theme"]));
        assert!(!skipped(&t["controllers"]["rog_ally"]["children"]["rgb_boot"]));   // advanced: shown on request, not skipped
        let st = json!({"tdp": {"unified": {"tdp": {"mode": "custom", "custom": {"tdp": 17}}}}, "version": "a1b2c3d4"});
        assert_eq!(lookup(&st, "tdp.unified.tdp.custom.tdp"), &json!(17));
        assert_eq!(lookup(&st, "tdp.unified.tdp.mode"), &json!("custom"));
        assert!(lookup(&st, "tdp.unified.nope.x").is_null());
        assert!(lookup(&st, "version.x").is_null());
    }
}
