//! Hearth Input: controller layouts that make games without controller support playable. While a game with a
//! layout is in front, desktop/shell/hearth-input (Python, started with Game Mode by nimbus-session) grabs the
//! controller and acts as a keyboard and mouse. This side keeps each game's choice, checks layouts, answers the
//! UI (input.* calls), and tells the mapper what's going on over its socket.
//!
//! Storage: ~/.config/nimbus/input/<game id>.json, one file per game (this user's own):
//!   {"version": 1, "layout": "off" | <template id> | "custom", "custom": {layout}, "prefs": {"sensitivity", "gyro"}}
//! No file means Off: the game's own controller support (Beacon's strategy games start with the "Strategy (RTS)"
//! template instead, see default_layout; choosing Off for them is saved like any choice). The custom layout is kept when you switch to Off or a
//! template, so switching back doesn't lose your edits.
//!
//! Layouts are checked strictly, against the same tables the mapper uses (desktop/shell/input-layouts.json,
//! compiled in): known controls, known key names, numbers in range, and only actions that press keys, click,
//! scroll, shift to the second set, toggle gyro or open the radial menu. The mapper checks again on its side.
//!
//! The mapper's socket: <runtime dir>/hearth/input.sock (the folder overlay_link.rs also uses: 0700, this user's).
//! One connection stays open (start_link): Hearth sends the state about every 400 ms when it changes and at least
//! once a second; the mapper lets everything go if it hears nothing for 5 s. The mapper sends events back:
//! {"t": "guide"} (it holds the controller, so Hearth's own reader can't see Guide) and {"t": "osd", "text"}.
//! Quick Access opening asks the mapper to let go first and waits for the answer (overlay()), so the two never
//! hold the controller at the same time.

use serde_json::{json, Map, Value};
use std::collections::HashSet;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

const TABLES_JSON: &str = include_str!("../../linux/desktop/shell/input-layouts.json");

/// The shared tables: keys, controls, limits, templates.
pub fn tables() -> &'static Value {
    static T: OnceLock<Value> = OnceLock::new();
    T.get_or_init(|| serde_json::from_str(TABLES_JSON).unwrap_or(Value::Null))
}

fn keys() -> &'static HashSet<String> {
    static K: OnceLock<HashSet<String>> = OnceLock::new();
    K.get_or_init(|| {
        tables()["keys"].as_array().into_iter().flatten()
            .flat_map(|g| g["keys"].as_array().into_iter().flatten().filter_map(|k| k[0].as_str().map(str::to_string)))
            .collect()
    })
}

/// The first column of one of the tables' lists ("buttons", "sticks", "mouse", "scroll").
fn ids(list: &str) -> Vec<&'static str> {
    tables()[list].as_array().into_iter().flatten().filter_map(|x| x[0].as_str()).collect()
}

fn limit(name: &str) -> (f64, f64) {
    let l = &tables()["limits"][name];
    (l[0].as_f64().unwrap_or(0.0), l[1].as_f64().unwrap_or(0.0))
}
fn limit_n(name: &str) -> usize { tables()["limits"][name].as_u64().unwrap_or(0) as usize }

pub fn template(id: &str) -> Option<&'static Value> {
    tables()["templates"].as_array()?.iter().find(|t| t["id"] == id).map(|t| &t["layout"])
}

// ------------------------------------------------------------------ checking layouts

#[derive(Clone, Copy, PartialEq)]
enum Where { Button, ShiftButton, Simple, Radial }

fn allowed(w: Where, kind: &str) -> bool {
    match w {
        Where::Button => true,
        Where::ShiftButton => !matches!(kind, "shift" | "radial"),
        Where::Simple => matches!(kind, "none" | "keys" | "mouse" | "scroll"),
        Where::Radial => matches!(kind, "keys" | "mouse" | "scroll"),
    }
}

fn short(s: &str, n: usize) -> String { s.chars().take(n).collect() }

/// One action string → its kind (none, keys, mouse, scroll, shift, gyro, radial), if it's allowed here.
fn action_kind(v: &Value, w: Where, path: &str) -> Result<&'static str, String> {
    let s = v.as_str().filter(|s| !s.is_empty() && s.len() <= 80).ok_or_else(|| format!("{path}: should be an action like \"key:SPACE\""))?;
    let kind = match s {
        "none" => "none",
        "shift" => "shift",
        "radial" => "radial",
        "gyro:toggle" | "gyro:hold" => "gyro",
        _ => {
            if let Some(k) = s.strip_prefix("key:") {
                let names: Vec<&str> = k.split('+').collect();
                let max = limit_n("combo_keys");
                if names.is_empty() || names.len() > max { return Err(format!("{path}: a key combination has 1 to {max} keys")); }
                let mut seen = HashSet::new();
                for n in &names {
                    if !seen.insert(*n) { return Err(format!("{path}: the same key twice")); }
                    if !keys().contains(*n) { return Err(format!("{path}: unknown key {:?}", short(n, 20))); }
                }
                "keys"
            } else if s.strip_prefix("mouse:").map(|m| ids("mouse").contains(&m)).unwrap_or(false) {
                "mouse"
            } else if s.strip_prefix("scroll:").map(|m| ids("scroll").contains(&m)).unwrap_or(false) {
                "scroll"
            } else {
                return Err(format!("{path}: unknown action {:?}", short(s, 30)));
            }
        }
    };
    if !allowed(w, kind) { return Err(format!("{path}: {s:?} can't be used here")); }
    Ok(kind)
}

fn num(v: &Value, lim: &str, path: &str, integer: bool) -> Result<f64, String> {
    let (lo, hi) = limit(lim);
    let x = match v { Value::Number(n) => n.as_f64().filter(|f| f.is_finite()), _ => None }.ok_or_else(|| format!("{path}: should be a number"))?;
    if integer && x.fract() != 0.0 { return Err(format!("{path}: should be a whole number")); }
    if x < lo || x > hi { return Err(format!("{path}: should be between {lo} and {hi}")); }
    Ok(x)
}

fn boolean(v: &Value, path: &str) -> Result<bool, String> { v.as_bool().ok_or_else(|| format!("{path}: should be true or false")) }

fn obj<'a>(v: &'a Value, path: &str, allowed: &[&str]) -> Result<&'a Map<String, Value>, String> {
    let m = v.as_object().ok_or_else(|| format!("{path}: should be an object"))?;
    if let Some(k) = m.keys().find(|k| !allowed.contains(&k.as_str())) { return Err(format!("{path}: unknown setting {:?}", short(k, 30))); }
    Ok(m)
}

fn check_button(ctl: &str, v: &Value, w: Where, path: &str) -> Result<(), String> {
    let owned;
    let v = if v.is_string() { owned = json!({"press": v}); &owned } else { v };
    let fields: &[&str] = if ctl == "lt" || ctl == "rt" { &["press", "hold", "turbo", "threshold"] } else { &["press", "hold", "turbo"] };
    let m = obj(v, path, fields)?;
    let press = m.get("press").ok_or_else(|| format!("{path}: needs \"press\""))?;
    let pk = action_kind(press, w, &format!("{path}.press"))?;
    let hold = m.get("hold").filter(|h| !h.is_null());
    if let Some(h) = hold {
        action_kind(h, Where::Simple, &format!("{path}.hold"))?;
        if !allowed(Where::Simple, pk) { return Err(format!("{path}: tap and hold work with keys, clicks and scrolling")); }
    }
    if let Some(t) = m.get("turbo") {
        if boolean(t, &format!("{path}.turbo"))? && (hold.is_some() || !matches!(pk, "keys" | "mouse")) {
            return Err(format!("{path}: turbo repeats a key or a click (and can't be used with hold)"));
        }
    }
    if let Some(t) = m.get("threshold") { num(t, "threshold", &format!("{path}.threshold"), false)?; }
    Ok(())
}

fn check_stick(v: &Value, path: &str) -> Result<(), String> {
    let mode = v.get("mode").and_then(Value::as_str).filter(|m| ["none", "keys", "mouse", "scroll"].contains(m))
        .ok_or_else(|| format!("{path}: needs a mode: none, keys, mouse or scroll"))?;
    let fields: &[&str] = match mode {
        "none" => &["mode"],
        "keys" => &["mode", "up", "down", "left", "right", "threshold"],
        "mouse" => &["mode", "sensitivity", "deadzone", "outer", "curve", "invert_y"],
        _ => &["mode", "sensitivity", "deadzone"],
    };
    let m = obj(v, path, fields)?;
    match mode {
        "keys" => {
            for d in ["up", "down", "left", "right"] { if let Some(a) = m.get(d) { action_kind(a, Where::Simple, &format!("{path}.{d}"))?; } }
            if let Some(t) = m.get("threshold") { num(t, "stick_threshold", &format!("{path}.threshold"), false)?; }
        }
        "mouse" | "scroll" => {
            if let Some(s) = m.get("sensitivity") { num(s, "sensitivity", &format!("{path}.sensitivity"), false)?; }
            let dz = match m.get("deadzone") { Some(d) => num(d, "deadzone", &format!("{path}.deadzone"), false)?, None => 0.12 };
            if mode == "mouse" {
                let outer = match m.get("outer") { Some(o) => num(o, "outer", &format!("{path}.outer"), false)?, None => 0.95 };
                if let Some(c) = m.get("curve") { num(c, "curve", &format!("{path}.curve"), false)?; }
                if let Some(i) = m.get("invert_y") { boolean(i, &format!("{path}.invert_y"))?; }
                if outer < dz + 0.1 { return Err(format!("{path}: the outer dead zone must be well past the dead zone")); }
            }
        }
        _ => {}
    }
    Ok(())
}

/// Is this a layout Hearth Input can run? (The same rules as hearth-input's validate_layout.)
pub fn validate(l: &Value) -> Result<(), String> {
    let m = obj(l, "layout", &["version", "name", "base", "buttons", "sticks", "gyro", "shift", "radial", "hold_ms", "turbo_hz"])?;
    if let Some(v) = m.get("version") { if v.as_f64() != Some(1.0) { return Err("layout.version: only version 1 is known".into()); } }
    if let Some(n) = m.get("name") {
        let ok = n.as_str().map(|s| !s.trim().is_empty() && s.chars().count() <= limit_n("name_len") && !s.chars().any(|c| (c as u32) < 32)).unwrap_or(false);
        if !ok { return Err(format!("layout.name: up to {} characters", limit_n("name_len"))); }
    }
    if let Some(b) = m.get("base") {
        let ok = b.as_str().map(|s| s.is_empty() || template(s).is_some()).unwrap_or(false);
        if !ok { return Err("layout.base: unknown template".into()); }
    }
    let buttons = ids("buttons");
    let sticks = ids("sticks");
    if let Some(b) = m.get("buttons") {
        for (ctl, v) in obj(b, "buttons", &buttons)? { check_button(ctl, v, Where::Button, &format!("buttons.{ctl}"))?; }
    }
    if let Some(s) = m.get("sticks") {
        for (st, v) in obj(s, "sticks", &sticks)? { check_stick(v, &format!("sticks.{st}"))?; }
    }
    if let Some(sh) = m.get("shift") {
        let sh = obj(sh, "shift", &["buttons", "sticks"])?;
        if let Some(b) = sh.get("buttons") {
            for (ctl, v) in obj(b, "shift.buttons", &buttons)? { check_button(ctl, v, Where::ShiftButton, &format!("shift.buttons.{ctl}"))?; }
        }
        if let Some(s) = sh.get("sticks") {
            for (st, v) in obj(s, "shift.sticks", &sticks)? { check_stick(v, &format!("shift.sticks.{st}"))?; }
        }
    }
    if let Some(g) = m.get("gyro") {
        let g = obj(g, "gyro", &["sensitivity", "deadzone", "invert_x", "invert_y", "start_on"])?;
        if let Some(v) = g.get("sensitivity") { num(v, "gyro_sensitivity", "gyro.sensitivity", false)?; }
        if let Some(v) = g.get("deadzone") { num(v, "gyro_deadzone", "gyro.deadzone", false)?; }
        for k in ["invert_x", "invert_y", "start_on"] { if let Some(v) = g.get(k) { boolean(v, &format!("gyro.{k}"))?; } }
    }
    if let Some(r) = m.get("radial") {
        let r = r.as_array().filter(|r| r.len() <= limit_n("radial_items")).ok_or_else(|| format!("radial: up to {} actions", limit_n("radial_items")))?;
        for (i, a) in r.iter().enumerate() { action_kind(a, Where::Radial, &format!("radial.{i}"))?; }
    }
    if let Some(v) = m.get("hold_ms") { num(v, "hold_ms", "hold_ms", true)?; }
    if let Some(v) = m.get("turbo_hz") { num(v, "turbo_hz", "turbo_hz", false)?; }
    Ok(())
}

/// Quick Access's per-game choices: {sensitivity, gyro}, with defaults.
pub fn check_prefs(p: &Value) -> Result<Value, String> {
    if p.is_null() { return Ok(json!({"sensitivity": 1.0, "gyro": false})); }
    let m = obj(p, "prefs", &["sensitivity", "gyro"])?;
    let s = match m.get("sensitivity") { Some(v) => num(v, "pref_sensitivity", "prefs.sensitivity", false)?, None => 1.0 };
    let g = match m.get("gyro") { Some(v) => boolean(v, "prefs.gyro")?, None => false };
    Ok(json!({"sensitivity": s, "gyro": g}))
}

// ------------------------------------------------------------------ each game's choice

pub fn dir() -> PathBuf { crate::paths::config_dir().join("input") }

/// A game id as a file name: letters, digits, - and _ as they are (and dots, but not first); anything else
/// %XX, so no id can name another folder.
pub fn file_name(id: &str) -> Result<String, String> {
    if id.is_empty() || id.len() > 200 { return Err("Unknown game".into()); }
    let mut out = String::new();
    for (i, b) in id.bytes().enumerate() {
        if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || (b == b'.' && i > 0) { out.push(b as char); } else { out.push_str(&format!("%{b:02X}")); }
    }
    Ok(out + ".json")
}

/// The layout a game has before you choose one: Off, or "Strategy (RTS)" for Beacon's strategy games.
pub fn default_layout(id: &str) -> &'static str { if crate::beacon::rts_by_default(id) && template("rts").is_some() { "rts" } else { "off" } }

/// A game's saved choice, checked: {"layout", "custom", "prefs"}. No file: the game's default (default_layout).
/// Broken files are Off.
pub fn load_in(dir: &Path, id: &str) -> Value {
    let off = json!({"layout": "off", "custom": null, "prefs": check_prefs(&Value::Null).unwrap_or(Value::Null)});
    let Ok(name) = file_name(id) else { return off };
    let Ok(text) = std::fs::read_to_string(dir.join(name)) else { return json!({"layout": default_layout(id), "custom": null, "prefs": check_prefs(&Value::Null).unwrap_or(Value::Null)}) };
    let Ok(v) = serde_json::from_str::<Value>(&text) else { return off };
    let custom = if v["custom"].is_object() && validate(&v["custom"]).is_ok() { v["custom"].clone() } else { Value::Null };
    let prefs = check_prefs(&v["prefs"]).unwrap_or_else(|_| check_prefs(&Value::Null).unwrap_or(Value::Null));
    let layout = match v["layout"].as_str() {
        Some("custom") if !custom.is_null() => "custom".to_string(),
        Some(t) if template(t).is_some() => t.to_string(),
        _ => "off".to_string(),
    };
    json!({"layout": layout, "custom": custom, "prefs": prefs})
}

/// Saves a game's choice. `layout`: "off", a template id or "custom" (then `custom` is the layout, or the one
/// already saved). `prefs` are merged into what's saved. Returns the saved choice.
pub fn save_in(dir: &Path, id: &str, layout: Option<&str>, custom: Option<&Value>, prefs: Option<&Value>) -> Result<Value, String> {
    let name = file_name(id)?;
    let cur = load_in(dir, id);
    let custom = match custom.filter(|c| !c.is_null()) {
        Some(c) => { validate(c)?; c.clone() }
        None => cur["custom"].clone(),
    };
    let layout = layout.unwrap_or(cur["layout"].as_str().unwrap_or("off"));
    match layout {
        "off" => {}
        "custom" => if custom.is_null() { return Err("There's no custom layout to use yet".into()); },
        t if template(t).is_some() => {}
        _ => return Err("Unknown layout".into()),
    }
    let mut p = cur["prefs"].as_object().cloned().unwrap_or_default();
    if let Some(np) = prefs.filter(|x| !x.is_null()) {
        let np = obj(np, "prefs", &["sensitivity", "gyro"])?;
        for (k, v) in np { p.insert(k.clone(), v.clone()); }
    }
    let prefs = check_prefs(&Value::Object(p))?;
    let v = json!({"version": 1, "layout": layout, "custom": custom, "prefs": prefs});
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700));
    }
    crate::paths::write_atomic(&dir.join(name), serde_json::to_string_pretty(&v).unwrap_or_default().as_bytes()).map_err(|e| e.to_string())?;
    Ok(load_in(dir, id))
}

/// The layout a choice runs (None: Off).
pub fn resolve(choice: &Value) -> Option<Value> {
    match choice["layout"].as_str()? {
        "off" => None,
        "custom" => Some(choice["custom"].clone()).filter(|c| c.is_object()),
        t => template(t).cloned(),
    }
}

/// What the mapper is told: the game, whether it's in front, whether Quick Access is open, and its layout.
pub fn state_in(dir: &Path, game: Option<&str>, front: bool, overlay: bool) -> Value {
    let choice = game.map(|g| load_in(dir, g));
    let layout = choice.as_ref().and_then(resolve);
    let prefs = match (&layout, &choice) { (Some(_), Some(c)) => c["prefs"].clone(), _ => Value::Null };
    json!({"cmd": "state", "game": game, "front": front, "overlay": overlay, "layout": layout, "prefs": prefs})
}
pub fn state(game: Option<&str>, front: bool, overlay: bool) -> Value { state_in(&dir(), game, front, overlay) }

// ------------------------------------------------------------------ the mapper's socket

pub fn socket_path() -> PathBuf { crate::overlay_link::runtime_dir().join("hearth").join("input.sock") }

/// The socket's folder is this user's own (nobody else could have put a socket there).
fn safe(path: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    path.parent().and_then(|d| std::fs::symlink_metadata(d).ok()).map(|m| m.is_dir() && m.uid() == crate::overlay_link::uid()).unwrap_or(false)
        && std::fs::symlink_metadata(path).map(|m| m.uid() == crate::overlay_link::uid()).unwrap_or(false)
}

/// Whole lines out of what's been read so far; the unfinished rest stays in `buf`.
pub fn take_lines(buf: &mut Vec<u8>) -> Vec<String> {
    let mut out = Vec::new();
    while let Some(i) = buf.iter().position(|b| *b == b'\n') {
        let line: Vec<u8> = buf.drain(..=i).collect();
        out.push(String::from_utf8_lossy(&line[..line.len() - 1]).to_string());
    }
    out
}

/// One request to the mapper, one answer.
pub fn request(cmd: &Value, timeout: Duration) -> Result<Value, String> {
    let path = socket_path();
    if !safe(&path) { return Err("Hearth Input isn't running (it runs in Game Mode on Hearth, the OS)".into()); }
    let mut s = UnixStream::connect(&path).map_err(|_| "Hearth Input isn't running".to_string())?;
    let _ = s.set_read_timeout(Some(timeout));
    let _ = s.set_write_timeout(Some(timeout));
    s.write_all(format!("{cmd}\n").as_bytes()).map_err(|e| e.to_string())?;
    let mut buf = Vec::new();
    let mut chunk = [0u8; 8192];
    let start = Instant::now();
    loop {
        if let Some(line) = take_lines(&mut buf).into_iter().next() { return serde_json::from_str(&line).map_err(|e| e.to_string()); }
        if start.elapsed() > timeout || buf.len() > 1 << 20 { return Err("Hearth Input didn't answer".into()); }
        match s.read(&mut chunk) { Ok(0) | Err(_) => return Err("Hearth Input didn't answer".into()), Ok(n) => buf.extend_from_slice(&chunk[..n]) }
    }
}

/// Quick Access opening (on) or closed (off). Waits until the mapper has let go of the controller, so Hearth (or
/// Handheld Daemon) can take it. Does nothing quickly when the mapper isn't running.
pub fn overlay(on: bool) {
    if !socket_path().exists() { return; }
    if let Err(e) = request(&json!({"cmd": "overlay", "on": on}), Duration::from_millis(400)) { crate::bridge::log(&format!("input: overlay {on}: {e}")); }
}

pub type GetState = Arc<dyn Fn() -> (Option<String>, bool, bool) + Send + Sync>;
pub type OnEvent = Arc<dyn Fn(Value) + Send + Sync>;

/// Keeps Hearth Input told what's going on (Game Mode): `get` says (running game, game in front, Quick Access
/// open); `on_event` gets the mapper's events (guide, osd). Reconnects when the mapper restarts.
pub fn start_link(get: GetState, on_event: OnEvent) {
    std::thread::spawn(move || {
        let mut told = false;
        loop {
            let path = socket_path();
            let conn = if safe(&path) { UnixStream::connect(&path).ok() } else { None };
            let Some(mut s) = conn else { std::thread::sleep(Duration::from_secs(3)); continue };
            if !told { crate::bridge::log("input: talking to Hearth Input (controller layouts)"); told = true; }
            let _ = s.set_read_timeout(Some(Duration::from_millis(400)));
            let _ = s.set_write_timeout(Some(Duration::from_secs(2)));
            let mut buf = Vec::new();
            let mut last = String::new();
            let mut sent: Option<Instant> = None;
            let mut chunk = [0u8; 4096];
            loop {
                let (game, front, overlay) = get();
                let msg = state(game.as_deref(), front, overlay).to_string();
                if msg != last || sent.map(|t| t.elapsed() >= Duration::from_secs(1)).unwrap_or(true) {
                    if s.write_all(format!("{msg}\n").as_bytes()).is_err() { break; }
                    last = msg;
                    sent = Some(Instant::now());
                }
                match s.read(&mut chunk) {
                    Ok(0) => break,
                    Ok(n) => {
                        buf.extend_from_slice(&chunk[..n]);
                        if buf.len() > 1 << 20 { break; }
                        for line in take_lines(&mut buf) {
                            if let Ok(v) = serde_json::from_str::<Value>(&line) { if v["t"].is_string() { on_event(v); } }
                        }
                    }
                    Err(e) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut | std::io::ErrorKind::Interrupted) => {}
                    Err(_) => break,
                }
            }
            crate::bridge::log("input: Hearth Input went away; looking for it again");
            std::thread::sleep(Duration::from_secs(2));
        }
    });
}

// ------------------------------------------------------------------ the UI's calls

fn game_arg(a: &Value, running: &Option<String>) -> Result<String, String> {
    a["game"].as_str().filter(|g| !g.is_empty()).map(str::to_string).or_else(|| running.clone()).ok_or_else(|| "No game".to_string())
}

/// input.tables, input.get, input.set, input.test, input.status. `running` is the game playing now (the default
/// game for Quick Access).
pub fn call(method: &str, a: &Value, running: Option<String>) -> Result<Value, String> {
    call_in(&dir(), method, a, running)
}

pub fn call_in(dir: &Path, method: &str, a: &Value, running: Option<String>) -> Result<Value, String> {
    match method {
        "input.tables" => {
            let mut t = tables().clone();
            if let Some(o) = t.as_object_mut() { o.remove("_about"); }
            Ok(t)
        }
        "input.get" => {
            let id = game_arg(a, &running)?;
            let mut c = load_in(dir, &id);
            c["game"] = Value::String(id.clone());
            c["running"] = json!(running.as_deref() == Some(id.as_str()));
            c["resolved"] = resolve(&c).unwrap_or(Value::Null);
            Ok(c)
        }
        "input.set" => {
            let id = game_arg(a, &running)?;
            save_in(dir, &id, a["layout"].as_str(), a.get("custom"), a.get("prefs"))?;
            call_in(dir, "input.get", &json!({"game": id}), running)
        }
        "input.test" => {
            if a["stop"] == true { return request(&json!({"cmd": "test", "stop": true}), Duration::from_secs(2)); }
            let layout = a.get("layout").filter(|l| l.is_object()).ok_or("No layout to try")?;
            validate(layout)?;
            let secs = a["seconds"].as_u64().unwrap_or(20).clamp(1, 60);
            let r = request(&json!({"cmd": "test", "layout": layout, "seconds": secs}), Duration::from_secs(2))?;
            if r["ok"] == false { return Err(r["error"].as_str().unwrap_or("Couldn't try it").to_string()); }
            Ok(r)
        }
        "input.status" => Ok(request(&json!({"cmd": "status"}), Duration::from_secs(2)).unwrap_or_else(|e| json!({"ok": false, "error": e}))),
        _ => Err(format!("{method} isn't known")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shooter() -> Value { template("shooter").unwrap().clone() }
    fn bad(change: impl Fn(&mut Value), word: &str) {
        let mut l = shooter();
        change(&mut l);
        let e = validate(&l).expect_err("should be refused");
        assert!(e.contains(word), "{e:?} should mention {word:?}");
    }

    #[test]
    fn tables_load_and_templates_are_valid() {
        assert!(tables().is_object());
        assert!(keys().contains("SPACE") && keys().contains("LEFTCTRL") && keys().contains("F12") && keys().contains("KP0"));
        assert!(!keys().contains("POWER"));
        let ids: Vec<&str> = tables()["templates"].as_array().unwrap().iter().map(|t| t["id"].as_str().unwrap()).collect();
        assert_eq!(ids, vec!["shooter", "pointclick", "retro", "mouse", "rts"]);
        for id in ids { validate(template(id).unwrap()).unwrap_or_else(|e| panic!("{id}: {e}")); }
        assert_eq!(template("shooter").unwrap()["buttons"]["a"], "key:SPACE");
        assert!(template("nope").is_none());
    }

    #[test]
    fn strict_checks() {
        bad(|l| l["buttons"]["a"] = json!("key:NOPE"), "unknown key");
        bad(|l| l["buttons"]["a"] = json!("exec:rm -rf /"), "unknown action");
        bad(|l| l["buttons"]["a"] = json!("key:"), "unknown key");
        bad(|l| l["buttons"]["a"] = json!("key:A+B+C+D+E"), "1 to 4");
        bad(|l| l["buttons"]["a"] = json!("key:A+A"), "same key");
        bad(|l| l["buttons"]["guide"] = json!("key:A"), "unknown setting");
        bad(|l| l["buttons"]["a"] = json!({"press": "key:A", "command": "x"}), "unknown setting");
        bad(|l| l["buttons"]["a"] = json!({"press": "key:A", "threshold": 0.5}), "unknown setting");
        bad(|l| l["buttons"]["a"] = json!({"press": "key:A", "hold": "key:B", "turbo": true}), "turbo");
        bad(|l| l["buttons"]["a"] = json!({"press": "shift", "hold": "key:B"}), "tap and hold");
        bad(|l| l["buttons"]["rt"] = json!({"press": "mouse:left", "threshold": 2}), "between");
        bad(|l| l["buttons"]["rt"] = json!({"press": "mouse:left", "threshold": true}), "number");
        bad(|l| l["sticks"]["rs"]["sensitivity"] = json!(50), "between");
        bad(|l| l["sticks"]["rs"]["up"] = json!("key:W"), "unknown setting");
        bad(|l| { l["sticks"]["rs"]["deadzone"] = json!(0.5); l["sticks"]["rs"]["outer"] = json!(0.55); }, "outer");
        bad(|l| l["sticks"]["ls"] = json!({"mode": "teleport"}), "mode");
        bad(|l| l["sticks"]["ls"]["up"] = json!("radial"), "can't be used here");
        bad(|l| l["shift"] = json!({"buttons": {"a": "shift"}}), "can't be used here");
        bad(|l| l["radial"] = json!(vec!["key:A"; 9]), "up to 8");
        bad(|l| l["radial"] = json!(["gyro:toggle"]), "can't be used here");
        bad(|l| l["name"] = json!("x".repeat(61)), "name");
        bad(|l| l["name"] = json!("bad\nname"), "name");
        bad(|l| l["base"] = json!("nope"), "template");
        bad(|l| l["hold_ms"] = json!(20), "between");
        bad(|l| l["hold_ms"] = json!(300.5), "whole");
        bad(|l| l["version"] = json!(2), "version");
        bad(|l| l["script"] = json!("x"), "unknown setting");
        assert!(validate(&json!({"buttons": {"a": "key:LEFTCTRL+C"}})).is_ok());
        assert!(validate(&json!({})).is_ok());
        assert!(validate(&json!("key:A")).is_err());
    }

    #[test]
    fn strategy_template_and_beacon_default() {
        let r = template("rts").unwrap();
        assert_eq!(r["name"], "Strategy (RTS)");
        assert_eq!(r["sticks"]["rs"]["mode"], "mouse");
        assert_eq!(r["buttons"]["rt"]["press"], "mouse:left");
        assert_eq!(r["buttons"]["lt"]["press"], "mouse:right");
        assert_eq!([&r["buttons"]["up"], &r["buttons"]["down"], &r["buttons"]["left"], &r["buttons"]["right"]], [&json!("key:UP"), &json!("key:DOWN"), &json!("key:LEFT"), &json!("key:RIGHT")]);
        assert_eq!(r["buttons"]["lb"], "shift");
        assert_eq!(r["buttons"]["menu"], "key:ESC");
        assert_eq!(r["buttons"]["a"], "key:SPACE");
        assert_eq!(r["buttons"]["rb"], "key:LEFTCTRL");
        assert_eq!(r["buttons"]["view"], "radial");
        assert_eq!(r["shift"]["buttons"]["up"], "key:1");
        assert_eq!(r["shift"]["buttons"]["left"], "key:4");
        assert_eq!(r["shift"]["buttons"]["y"], "key:LEFTCTRL+1");
        assert_eq!(r["radial"].as_array().unwrap().len(), 8);
        // Beacon's strategy games start with it; Off stays possible (and is kept once chosen); others start Off.
        let d = tempfile::tempdir().unwrap();
        let dir = d.path().to_path_buf();
        assert_eq!(load_in(&dir, "beacon:rotwk")["layout"], "rts");
        assert_eq!(resolve(&load_in(&dir, "beacon:aotr")).unwrap()["name"], "Strategy (RTS)");
        assert_eq!(load_in(&dir, "beacon:conquest")["layout"], "off");
        assert_eq!(load_in(&dir, "steam:1")["layout"], "off");
        save_in(&dir, "beacon:rotwk", Some("off"), None, None).unwrap();
        assert_eq!(load_in(&dir, "beacon:rotwk")["layout"], "off");
        assert!(state_in(&dir, Some("beacon:rotwk"), true, false)["layout"].is_null());
        assert_eq!(state_in(&dir, Some("beacon:bfme2"), true, false)["layout"]["name"], "Strategy (RTS)");
        // Changing only the Quick Access settings keeps the default layout.
        let c = save_in(&dir, "beacon:bfme1", None, None, Some(&json!({"sensitivity": 1.5}))).unwrap();
        assert_eq!(c["layout"], "rts");
    }

    #[test]
    fn prefs() {
        assert_eq!(check_prefs(&Value::Null).unwrap(), json!({"sensitivity": 1.0, "gyro": false}));
        assert_eq!(check_prefs(&json!({"sensitivity": 2, "gyro": true})).unwrap(), json!({"sensitivity": 2.0, "gyro": true}));
        assert!(check_prefs(&json!({"sensitivity": 9})).is_err());
        assert!(check_prefs(&json!({"run": "x"})).is_err());
    }

    #[test]
    fn file_names_stay_in_the_folder() {
        assert_eq!(file_name("steam:440").unwrap(), "steam%3A440.json");
        assert_eq!(file_name("manual:my-game_2.0").unwrap(), "manual%3Amy-game_2.0.json");
        assert_eq!(file_name("../x").unwrap(), "%2E.%2Fx.json");
        assert!(!file_name("a/b").unwrap().contains('/'));
        assert!(file_name("").is_err());
        assert!(file_name(&"x".repeat(201)).is_err());
    }

    #[test]
    fn saves_and_resolves_choices() {
        let d = tempfile::tempdir().unwrap();
        let dir = d.path().join("input");
        let c = load_in(&dir, "steam:1");
        assert_eq!(c["layout"], "off");
        assert!(resolve(&c).is_none());
        let c = save_in(&dir, "steam:1", Some("shooter"), None, None).unwrap();
        assert_eq!(resolve(&c).unwrap()["name"], "Shooter / action");
        assert!(save_in(&dir, "steam:1", Some("custom"), None, None).is_err());   // no custom layout yet
        assert!(save_in(&dir, "steam:1", Some("hack"), None, None).is_err());
        let mut mine = shooter();
        mine["name"] = json!("Mine");
        mine["buttons"]["a"] = json!("key:J");
        let c = save_in(&dir, "steam:1", Some("custom"), Some(&mine), Some(&json!({"gyro": true}))).unwrap();
        assert_eq!(resolve(&c).unwrap()["buttons"]["a"], "key:J");
        assert_eq!(c["prefs"]["gyro"], true);
        // Off keeps the custom layout (and the prefs) for later.
        let c = save_in(&dir, "steam:1", Some("off"), None, Some(&json!({"sensitivity": 1.5}))).unwrap();
        assert!(resolve(&c).is_none());
        assert_eq!(c["custom"]["name"], "Mine");
        assert_eq!(c["prefs"], json!({"sensitivity": 1.5, "gyro": true}));
        let c = save_in(&dir, "steam:1", Some("custom"), None, None).unwrap();
        assert_eq!(resolve(&c).unwrap()["name"], "Mine");
        // Bad input is refused and nothing changes.
        let mut evil = shooter();
        evil["buttons"]["a"] = json!("key:A; reboot");
        assert!(save_in(&dir, "steam:1", Some("custom"), Some(&evil), None).is_err());
        assert!(save_in(&dir, "steam:1", None, None, Some(&json!({"sensitivity": 99}))).is_err());
        assert_eq!(load_in(&dir, "steam:1")["custom"]["name"], "Mine");
        // A file broken by hand is Off, not an error.
        std::fs::write(dir.join(file_name("steam:2").unwrap()), "{\"layout\": \"custom\", \"custom\": {\"buttons\": {\"a\": \"key:NOPE\"}}}").unwrap();
        assert_eq!(load_in(&dir, "steam:2")["layout"], "off");
        std::fs::write(dir.join(file_name("steam:3").unwrap()), "not json").unwrap();
        assert_eq!(load_in(&dir, "steam:3")["layout"], "off");
    }

    #[test]
    fn state_for_the_mapper() {
        let d = tempfile::tempdir().unwrap();
        let dir = d.path().to_path_buf();
        save_in(&dir, "steam:1", Some("retro"), None, Some(&json!({"sensitivity": 2.0}))).unwrap();
        let s = state_in(&dir, Some("steam:1"), true, false);
        assert_eq!(s["cmd"], "state");
        assert_eq!(s["layout"]["name"], "Retro / arrow keys");
        assert_eq!(s["prefs"]["sensitivity"], 2.0);
        assert_eq!(s["front"], true);
        let s = state_in(&dir, Some("steam:9"), true, false);
        assert!(s["layout"].is_null() && s["prefs"].is_null());
        let s = state_in(&dir, None, false, true);
        assert!(s["game"].is_null() && s["layout"].is_null());
        assert_eq!(s["overlay"], true);
    }

    #[test]
    fn calls() {
        let d = tempfile::tempdir().unwrap();
        let dir = d.path().to_path_buf();
        let t = call_in(&dir, "input.tables", &json!({}), None).unwrap();
        assert!(t["_about"].is_null() && t["templates"].is_array() && t["keys"].is_array());
        assert!(call_in(&dir, "input.get", &json!({}), None).is_err());   // no game, none running
        let g = call_in(&dir, "input.get", &json!({}), Some("steam:5".into())).unwrap();
        assert_eq!((g["game"].as_str(), g["running"].as_bool(), g["layout"].as_str()), (Some("steam:5"), Some(true), Some("off")));
        let g = call_in(&dir, "input.set", &json!({"game": "steam:6", "layout": "mouse"}), Some("steam:5".into())).unwrap();
        assert_eq!(g["resolved"]["name"], "Mouse only");
        assert_eq!(g["running"], false);
        assert!(call_in(&dir, "input.test", &json!({"layout": {"buttons": {"a": "key:NOPE"}}}), None).is_err());
        assert!(call_in(&dir, "input.nope", &json!({}), None).is_err());
    }

    #[test]
    fn reads_lines() {
        let mut b = b"{\"t\":\"guide\"}\n{\"t\":\"os".to_vec();
        assert_eq!(take_lines(&mut b), vec!["{\"t\":\"guide\"}".to_string()]);
        assert_eq!(b, b"{\"t\":\"os".to_vec());
        b.extend_from_slice(b"d\"}\n\n");
        assert_eq!(take_lines(&mut b), vec!["{\"t\":\"osd\"}".to_string(), String::new()]);
        assert!(b.is_empty());
    }
}
