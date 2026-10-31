//! Hearth Input: controller layouts that make games without controller support playable. While a game with a
//! layout is in front, desktop/shell/hearth-input (Python, started with Game Mode by nimbus-session) grabs the
//! controller and acts as a keyboard and mouse. This side keeps each game's choice, checks layouts, answers the
//! UI (input.* calls), and tells the mapper what's going on over its socket.
//!
//! Storage: ~/.config/nimbus/input/<game id>.json, one file per game (this user's own):
//!   {"version": 2, "layout": "off" | <template id> | "custom" | "global", "custom": {layout}, "prefs": {"sensitivity", "gyro"}}
//! and global.json, the layout of your own for every game without one: {"version": 2, "enabled", "layout"}. Files
//! from before (version 1, and version 1 layouts) are read and migrated (migrate).
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
use std::collections::{HashMap, HashSet};
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
fn ids_of(list: &str) -> Vec<&'static str> {
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
//
// Version 2 (docs/HEARTH-INPUT.md): the default action set (buttons, sticks, gyro), more action sets ("sets"),
// layers on top of them ("layers": held = mode shift, or toggled), and per-button activators (press, hold = long
// press, double, release, chords, turbo, toggle). Version 1 files (with the "shift" second set) are migrated
// (migrate). The same rules as hearth-input's validate_layout; both are tested against
// hosts/linux/test/fixtures/input-layout-cases.json.

#[derive(Clone, Copy, PartialEq)]
enum Where { Button, LayerButton, Simple, Radial }

fn allowed(w: Where, kind: &str) -> bool {
    match w {
        Where::Button => true,
        Where::LayerButton => !matches!(kind, "layer" | "radial"),
        Where::Simple => matches!(kind, "none" | "keys" | "mouse" | "scroll"),
        Where::Radial => matches!(kind, "keys" | "mouse" | "scroll"),
    }
}
fn simple(kind: &str) -> bool { allowed(Where::Simple, kind) }

fn short(s: &str, n: usize) -> String { s.chars().take(n).collect() }

/// The layer and action set ids a layout has: what its actions may name.
#[derive(Default)]
struct Ids { layers: HashSet<String>, sets: HashSet<String> }

/// One action string → its kind (none, keys, mouse, scroll, gyro, radial, layer, layer_toggle, set), if it's allowed here.
fn action_kind(v: &Value, w: Where, path: &str, ids: &Ids) -> Result<&'static str, String> {
    let s = v.as_str().filter(|s| !s.is_empty() && s.len() <= 80).ok_or_else(|| format!("{path}: should be an action like \"key:SPACE\""))?;
    let kind = match s {
        "none" => "none",
        "radial" => "radial",
        "gyro:toggle" | "gyro:hold" | "gyro:ratchet" => "gyro",
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
            } else if s.strip_prefix("mouse:").map(|m| ids_of("mouse").contains(&m)).unwrap_or(false) {
                "mouse"
            } else if s.strip_prefix("scroll:").map(|m| ids_of("scroll").contains(&m)).unwrap_or(false) {
                "scroll"
            } else if let Some(l) = s.strip_prefix("layer:").or_else(|| s.strip_prefix("layer_toggle:")) {
                if !ids.layers.contains(l) { return Err(format!("{path}: there's no layer {:?}", short(l, 20))); }
                if s.starts_with("layer:") { "layer" } else { "layer_toggle" }
            } else if let Some(st) = s.strip_prefix("set:") {
                if st != "default" && !ids.sets.contains(st) { return Err(format!("{path}: there's no action set {:?}", short(st, 20))); }
                "set"
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

fn check_name(v: Option<&Value>, path: &str) -> Result<(), String> {
    if let Some(n) = v {
        let ok = n.as_str().map(|s| !s.trim().is_empty() && s.chars().count() <= limit_n("name_len") && !s.chars().any(|c| (c as u32) < 32)).unwrap_or(false);
        if !ok { return Err(format!("{path}: up to {} characters", limit_n("name_len"))); }
    }
    Ok(())
}

const BUTTON_FIELDS: &[&str] = &["press", "hold", "double", "release", "chords", "turbo", "hold_turbo", "double_turbo", "toggle"];

fn check_button(ctl: &str, v: &Value, w: Where, path: &str, ids: &Ids) -> Result<(), String> {
    let owned;
    let v = if v.is_string() { owned = json!({"press": v}); &owned } else { v };
    let mut fields = BUTTON_FIELDS.to_vec();
    if ctl == "lt" || ctl == "rt" { fields.push("threshold"); }
    let m = obj(v, path, &fields)?;
    let pk = match m.get("press") { Some(p) => action_kind(p, w, &format!("{path}.press"), ids)?, None => "none" };
    let mut kinds: HashMap<&str, &str> = HashMap::new();
    for k in ["hold", "double", "release"] {
        if let Some(a) = m.get(k).filter(|a| !a.is_null()) { kinds.insert(k, action_kind(a, Where::Simple, &format!("{path}.{k}"), ids)?); }
    }
    let chords = match m.get("chords") { Some(c) => c.as_array().filter(|c| c.len() <= limit_n("chords")).ok_or_else(|| format!("{path}.chords: up to {}", limit_n("chords")))?.clone(), None => vec![] };
    let mut seen = HashSet::new();
    for (i, c) in chords.iter().enumerate() {
        let cp = format!("{path}.chords.{i}");
        let cm = obj(c, &cp, &["with", "do", "turbo"])?;
        let with = cm.get("with").and_then(Value::as_str).unwrap_or("");
        if !ids_of("buttons").contains(&with) || with == ctl || !seen.insert(with.to_string()) { return Err(format!("{cp}.with: another button (once each)")); }
        let dk = action_kind(cm.get("do").unwrap_or(&json!("none")), Where::Simple, &format!("{cp}.do"), ids)?;
        if let Some(t) = cm.get("turbo") { if boolean(t, &format!("{cp}.turbo"))? && !matches!(dk, "keys" | "mouse") { return Err(format!("{cp}: turbo repeats a key or a click")); } }
    }
    let flag = |k: &str| -> Result<bool, String> { match m.get(k) { Some(b) => boolean(b, &format!("{path}.{k}")), None => Ok(false) } };
    let (turbo, hold_turbo, double_turbo, toggle) = (flag("turbo")?, flag("hold_turbo")?, flag("double_turbo")?, flag("toggle")?);
    let extra = !kinds.is_empty() || !chords.is_empty() || turbo || toggle;
    if extra && !simple(pk) { return Err(format!("{path}: tap and hold, double presses, chords, turbo and toggle work with keys, clicks and scrolling")); }
    let key_or_click = |k: &str| matches!(k, "keys" | "mouse");
    if turbo && (kinds.contains_key("hold") || toggle || !key_or_click(pk)) { return Err(format!("{path}: turbo repeats a key or a click (and can't be used with hold or toggle)")); }
    if toggle && (kinds.contains_key("hold") || kinds.contains_key("double") || !key_or_click(pk)) { return Err(format!("{path}: toggle holds a key or a click (and can't be used with hold or a double press)")); }
    for (k, t, on) in [("hold", "hold_turbo", hold_turbo), ("double", "double_turbo", double_turbo)] {
        if on && !kinds.get(k).map(|x| key_or_click(x)).unwrap_or(false) { return Err(format!("{path}.{t}: turbo repeats a key or a click")); }
    }
    if let Some(t) = m.get("threshold") { num(t, "threshold", &format!("{path}.threshold"), false)?; }
    Ok(())
}

const STICK_MODES: &[&str] = &["none", "keys", "dpad", "mouse", "scroll", "wheel", "flick"];

fn check_stick(v: &Value, path: &str, ids: &Ids) -> Result<(), String> {
    let mode = v.get("mode").and_then(Value::as_str).filter(|m| STICK_MODES.contains(m))
        .ok_or_else(|| format!("{path}: needs a mode: {}", STICK_MODES.join(", ")))?;
    let fields: &[&str] = match mode {
        "none" => &["mode"],
        "keys" | "dpad" => &["mode", "up", "down", "left", "right", "threshold"],
        "mouse" => &["mode", "sensitivity", "deadzone", "outer", "curve", "invert_x", "invert_y"],
        "scroll" => &["mode", "sensitivity", "deadzone"],
        "wheel" => &["mode", "sensitivity", "threshold"],
        _ => &["mode", "turn", "threshold"],
    };
    let m = obj(v, path, fields)?;
    for d in ["up", "down", "left", "right"] { if let Some(a) = m.get(d) { action_kind(a, Where::Simple, &format!("{path}.{d}"), ids)?; } }
    if let Some(t) = m.get("threshold") { num(t, "stick_threshold", &format!("{path}.threshold"), false)?; }
    if let Some(s) = m.get("sensitivity") { num(s, "sensitivity", &format!("{path}.sensitivity"), false)?; }
    if let Some(t) = m.get("turn") { num(t, "turn_px", &format!("{path}.turn"), true)?; }
    for k in ["invert_x", "invert_y"] { if let Some(i) = m.get(k) { boolean(i, &format!("{path}.{k}"))?; } }
    if mode == "mouse" || mode == "scroll" {
        let dz = match m.get("deadzone") { Some(d) => num(d, "deadzone", &format!("{path}.deadzone"), false)?, None => 0.12 };
        if mode == "mouse" {
            let outer = match m.get("outer") { Some(o) => num(o, "outer", &format!("{path}.outer"), false)?, None => 0.95 };
            if let Some(c) = m.get("curve") { num(c, "curve", &format!("{path}.curve"), false)?; }
            if outer < dz + 0.1 { return Err(format!("{path}: the outer dead zone must be well past the dead zone")); }
        }
    }
    Ok(())
}

fn check_gyro(g: &Value, path: &str) -> Result<(), String> {
    let g = obj(g, path, &["sensitivity", "deadzone", "invert_x", "invert_y", "start_on", "when", "axis"])?;
    if let Some(v) = g.get("sensitivity") { num(v, "gyro_sensitivity", &format!("{path}.sensitivity"), false)?; }
    if let Some(v) = g.get("deadzone") { num(v, "gyro_deadzone", &format!("{path}.deadzone"), false)?; }
    for k in ["invert_x", "invert_y", "start_on"] { if let Some(v) = g.get(k) { boolean(v, &format!("{path}.{k}"))?; } }
    if let Some(w) = g.get("when") { if !matches!(w.as_str(), Some("button" | "always" | "stick")) { return Err(format!("{path}.when: one of button, always, stick")); } }
    if let Some(a) = g.get("axis") { if !matches!(a.as_str(), Some("yaw" | "roll")) { return Err(format!("{path}.axis: one of yaw, roll")); } }
    Ok(())
}

/// A set's or a layer's buttons and sticks.
fn check_controls(m: &Map<String, Value>, prefix: &str, w: Where, ids: &Ids) -> Result<(), String> {
    let pre = if prefix.is_empty() { String::new() } else { format!("{prefix}.") };
    let (buttons, sticks) = (ids_of("buttons"), ids_of("sticks"));
    if let Some(b) = m.get("buttons") {
        for (ctl, v) in obj(b, &format!("{pre}buttons"), &buttons)? { check_button(ctl, v, w, &format!("{pre}buttons.{ctl}"), ids)?; }
    }
    if let Some(s) = m.get("sticks") {
        for (st, v) in obj(s, &format!("{pre}sticks"), &sticks)? { check_stick(v, &format!("{pre}sticks.{st}"), ids)?; }
    }
    Ok(())
}

/// A file without a version is version 1 if it uses version 1's second set, else version 2.
pub fn uses_v1_shift(l: &Value) -> bool {
    l.get("shift").is_some() || l["buttons"].as_object().map(|b| b.values().any(|v| v == "shift" || v["press"] == "shift")).unwrap_or(false)
}

fn version_of(l: &Value) -> Result<u64, String> {
    match l.get("version") {
        None => Ok(if uses_v1_shift(l) { 1 } else { 2 }),
        Some(v) => v.as_f64().filter(|x| *x == 1.0 || *x == 2.0).map(|x| x as u64).ok_or_else(|| "layout.version: only versions 1 and 2 are known".to_string()),
    }
}

/// A layout as version 2: version 1's second set ("shift", held by buttons set to "shift") becomes the layer
/// "shift" (held by "layer:shift"). Version 2 comes back as it is. Doesn't check anything else (validate does).
pub fn migrate(l: &Value) -> Result<Value, String> {
    if !l.is_object() { return Err("layout: should be an object".into()); }
    if version_of(l)? == 2 { return Ok(l.clone()); }
    let m = obj(l, "layout", &["version", "name", "base", "buttons", "sticks", "gyro", "shift", "radial", "hold_ms", "double_ms", "turbo_hz"])?;
    let conv = |b: &Value| -> Value {
        let Some(o) = b.as_object() else { return b.clone() };
        Value::Object(o.iter().map(|(k, v)| {
            let v = if v == "shift" { json!("layer:shift") } else if v["press"] == "shift" { let mut x = v.clone(); x["press"] = json!("layer:shift"); x } else { v.clone() };
            (k.clone(), v)
        }).collect())
    };
    let mut n = Map::new();
    n.insert("version".into(), json!(2));
    for k in ["name", "base", "buttons", "sticks", "gyro", "radial", "hold_ms", "double_ms", "turbo_hz"] {
        if let Some(v) = m.get(k) { n.insert(k.into(), if k == "buttons" { conv(v) } else { v.clone() }); }
    }
    if m.contains_key("shift") || uses_v1_shift(l) {
        let empty = json!({});
        let sh = obj(m.get("shift").unwrap_or(&empty), "shift", &["buttons", "sticks"])?;
        let mut layer = json!({"id": "shift", "name": "Second set"});
        if let Some(b) = sh.get("buttons") { layer["buttons"] = conv(b); }
        if let Some(s) = sh.get("sticks") { layer["sticks"] = s.clone(); }
        n.insert("layers".into(), json!([layer]));
    }
    Ok(Value::Object(n))
}

fn id_ok(s: &str) -> bool { !s.is_empty() && s.len() <= 20 && s != "default" && s.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_') }

/// Is this a layout Hearth Input can run? (The same rules as hearth-input's validate_layout.)
pub fn validate(l: &Value) -> Result<(), String> {
    let l = migrate(l)?;
    let m = obj(&l, "layout", &["version", "name", "base", "buttons", "sticks", "gyro", "sets", "layers", "radial", "hold_ms", "double_ms", "turbo_hz"])?;
    check_name(m.get("name"), "layout.name")?;
    if let Some(b) = m.get("base") {
        let ok = b.as_str().map(|s| s.is_empty() || template(s).is_some()).unwrap_or(false);
        if !ok { return Err("layout.base: unknown template".into()); }
    }
    let empty = json!([]);
    let list = |k: &str| -> Result<Vec<Value>, String> {
        m.get(k).unwrap_or(&empty).as_array().filter(|a| a.len() <= limit_n(k)).cloned().ok_or_else(|| format!("{k}: up to {}", limit_n(k)))
    };
    let (layers, sets) = (list("layers")?, list("sets")?);
    let mut ids = Ids::default();
    for (kind, items) in [("layers", &layers), ("sets", &sets)] {
        for (i, it) in items.iter().enumerate() {
            let id = it["id"].as_str().filter(|s| id_ok(s)).ok_or_else(|| format!("{kind}.{i}.id: lower-case letters, digits and _ (not \"default\")"))?;
            let set = if kind == "layers" { &mut ids.layers } else { &mut ids.sets };
            if !set.insert(id.to_string()) { return Err(format!("{kind}.{i}.id: {id:?} twice")); }
        }
    }
    check_controls(m, "", Where::Button, &ids)?;
    if let Some(g) = m.get("gyro") { check_gyro(g, "gyro")?; }
    for (i, it) in sets.iter().enumerate() {
        let p = format!("sets.{i}");
        let sm = obj(it, &p, &["id", "name", "buttons", "sticks", "gyro"])?;
        check_name(sm.get("name"), &format!("{p}.name"))?;
        check_controls(sm, &p, Where::Button, &ids)?;
        if let Some(g) = sm.get("gyro") { check_gyro(g, &format!("{p}.gyro"))?; }
    }
    for (i, it) in layers.iter().enumerate() {
        let p = format!("layers.{i}");
        let lm = obj(it, &p, &["id", "name", "buttons", "sticks"])?;
        check_name(lm.get("name"), &format!("{p}.name"))?;
        check_controls(lm, &p, Where::LayerButton, &ids)?;
    }
    if let Some(r) = m.get("radial") {
        let r = r.as_array().filter(|r| r.len() <= limit_n("radial_items")).ok_or_else(|| format!("radial: up to {} actions", limit_n("radial_items")))?;
        for (i, a) in r.iter().enumerate() { action_kind(a, Where::Radial, &format!("radial.{i}"), &ids)?; }
    }
    if let Some(v) = m.get("hold_ms") { num(v, "hold_ms", "hold_ms", true)?; }
    if let Some(v) = m.get("double_ms") { num(v, "double_ms", "double_ms", true)?; }
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

/// The layout a game has before you choose one: Off, "Strategy (RTS)" for Beacon's strategy games, or "Mouse only"
/// for launchers' own windows (Xodus Nexus, Battle.net), which are made for a mouse. Their games start Off.
pub fn default_layout(id: &str) -> &'static str {
    if crate::beacon::rts_by_default(id) && template("rts").is_some() { "rts" }
    else if (crate::xodus::pointer_by_default(id) || id == crate::battlenet::APP_ID) && template("mouse").is_some() { "mouse" }
    else { "off" }
}

/// The global layout (Hearth Input's "desktop" layout): {"enabled", "layout"}. When it's on, every game that has no
/// choice of its own (and would start Off) uses it; any game can also choose it ("global"). Broken: off, none.
pub const GLOBAL_FILE: &str = "global.json";
pub fn global_in(dir: &Path) -> Value {
    let v: Value = std::fs::read_to_string(dir.join(GLOBAL_FILE)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or(Value::Null);
    let layout = migrate(&v["layout"]).ok().filter(|l| l.is_object() && validate(l).is_ok()).unwrap_or(Value::Null);
    json!({"enabled": v["enabled"] == true && !layout.is_null(), "layout": layout})
}
pub fn save_global_in(dir: &Path, enabled: Option<bool>, layout: Option<&Value>) -> Result<Value, String> {
    let cur = global_in(dir);
    let layout = match layout.filter(|l| !l.is_null()) { Some(l) => { validate(l)?; migrate(l)? } None => cur["layout"].clone() };
    let enabled = enabled.unwrap_or(cur["enabled"] == true);
    if enabled && layout.is_null() { return Err("Make the global layout first".into()); }
    write_private(dir, GLOBAL_FILE, &json!({"version": 2, "enabled": enabled, "layout": layout}))?;
    Ok(global_in(dir))
}

fn write_private(dir: &Path, name: &str, v: &Value) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700));
    }
    crate::paths::write_atomic(&dir.join(name), serde_json::to_string_pretty(v).unwrap_or_default().as_bytes()).map_err(|e| e.to_string())
}

/// A game's saved choice, checked: {"layout", "custom", "prefs"}. No file: the game's default (default_layout).
/// Broken files are Off.
pub fn load_in(dir: &Path, id: &str) -> Value {
    let off = json!({"layout": "off", "custom": null, "prefs": check_prefs(&Value::Null).unwrap_or(Value::Null)});
    let Ok(name) = file_name(id) else { return off };
    let Ok(text) = std::fs::read_to_string(dir.join(name)) else {
        let d = default_layout(id);
        let d = if d == "off" && global_in(dir)["enabled"] == true { "global" } else { d };
        return json!({"layout": d, "custom": null, "prefs": check_prefs(&Value::Null).unwrap_or(Value::Null)});
    };
    let Ok(v) = serde_json::from_str::<Value>(&text) else { return off };
    let custom = if v["custom"].is_object() && validate(&v["custom"]).is_ok() { migrate(&v["custom"]).unwrap_or(Value::Null) } else { Value::Null };
    let prefs = check_prefs(&v["prefs"]).unwrap_or_else(|_| check_prefs(&Value::Null).unwrap_or(Value::Null));
    let layout = match v["layout"].as_str() {
        Some("custom") if !custom.is_null() => "custom".to_string(),
        Some("global") if !global_in(dir)["layout"].is_null() => "global".to_string(),
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
        Some(c) => { validate(c)?; migrate(c)? }
        None => cur["custom"].clone(),
    };
    let layout = layout.unwrap_or(cur["layout"].as_str().unwrap_or("off"));
    match layout {
        "off" => {}
        "custom" => if custom.is_null() { return Err("There's no custom layout to use yet".into()); },
        "global" => if global_in(dir)["layout"].is_null() { return Err("There's no global layout yet".into()); },
        t if template(t).is_some() => {}
        _ => return Err("Unknown layout".into()),
    }
    let mut p = cur["prefs"].as_object().cloned().unwrap_or_default();
    if let Some(np) = prefs.filter(|x| !x.is_null()) {
        let np = obj(np, "prefs", &["sensitivity", "gyro"])?;
        for (k, v) in np { p.insert(k.clone(), v.clone()); }
    }
    let prefs = check_prefs(&Value::Object(p))?;
    let v = json!({"version": 2, "layout": layout, "custom": custom, "prefs": prefs});
    write_private(dir, &name, &v)?;
    Ok(load_in(dir, id))
}

/// The layout a choice runs (None: Off), as version 2.
pub fn resolve_in(dir: &Path, choice: &Value) -> Option<Value> {
    match choice["layout"].as_str()? {
        "off" => None,
        "custom" => Some(choice["custom"].clone()).filter(|c| c.is_object()),
        "global" => Some(global_in(dir)["layout"].clone()).filter(|c| c.is_object()),
        t => template(t).and_then(|l| migrate(l).ok()),
    }
}
pub fn resolve(choice: &Value) -> Option<Value> { resolve_in(&dir(), choice) }

/// This device's own layouts (custom ones and the global one), for your other devices (Hearth Link's
/// input.layouts): [{game, name, layout}]; the global one has game "_global".
pub fn own_layouts_in(dir: &Path) -> Vec<Value> {
    let mut out = Vec::new();
    let g = global_in(dir);
    if g["layout"].is_object() { out.push(json!({"game": "_global", "name": g["layout"]["name"].as_str().unwrap_or("Global"), "layout": g["layout"]})); }
    let mut names: Vec<String> = std::fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect();
    names.sort();
    for n in names {
        let Some(stem) = n.strip_suffix(".json") else { continue };
        if n == GLOBAL_FILE { continue; }
        let id = decode_name(stem);
        if file_name(&id).ok().as_deref() != Some(n.as_str()) { continue; }
        let c = load_in(dir, &id);
        if c["custom"].is_object() { out.push(json!({"game": id, "name": c["custom"]["name"].as_str().unwrap_or("Custom"), "layout": c["custom"]})); }
        if out.len() >= 200 { break; }
    }
    out
}

/// A file name back to its game id (file_name's %XX undone).
pub fn decode_name(stem: &str) -> String {
    let b = stem.as_bytes();
    let hex = |c: u8| (c as char).to_digit(16).map(|d| d as u8);
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let (Some(h), Some(l)) = (hex(b[i + 1]), hex(b[i + 2])) { out.push(h * 16 + l); i += 3; continue; }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

/// What the mapper is told: the game, whether it's in front, whether Quick Access is open, and its layout.
pub fn state_in(dir: &Path, game: Option<&str>, front: bool, overlay: bool) -> Value {
    let choice = game.map(|g| load_in(dir, g));
    let layout = choice.as_ref().and_then(|c| resolve_in(dir, c));
    let prefs = match (&layout, &choice) { (Some(_), Some(c)) => c["prefs"].clone(), _ => Value::Null };
    json!({"cmd": "state", "game": game, "front": front, "overlay": overlay, "layout": layout, "prefs": prefs})
}
pub fn state(game: Option<&str>, front: bool, overlay: bool) -> Value { state_in(&dir(), game, front, overlay) }

/// One line for the log: "no game", or "gog:123 in front (Quick Access open), layout Strategy (RTS)" / "layout off".
pub fn describe(st: &Value) -> String {
    let Some(game) = st["game"].as_str() else { return "no game running".into() };
    let place = if st["overlay"] == true { "Quick Access open" } else if st["front"] == true { "in front" } else { "not in front" };
    let layout = match st["layout"]["name"].as_str() { Some(n) => format!("layout {n}"), None if st["layout"].is_object() => "custom layout".into(), None => "layout off (the game gets the controller)".into() };
    format!("{game} {place}, {layout}")
}

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
            let mut said = String::new();
            let mut sent: Option<Instant> = None;
            let mut chunk = [0u8; 4096];
            loop {
                let (game, front, overlay) = get();
                let st = state(game.as_deref(), front, overlay);
                // In Hearth's log whenever it changes (Settings › Send logs): which game, whether it's in front, which layout.
                let summary = describe(&st);
                if summary != said { crate::bridge::log(&format!("input: {summary}")); said = summary; }
                let msg = st.to_string();
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

// ------------------------------------------------------------------ typing into the game (the on-screen keyboard)

/// The on-screen keyboard over a game (Guide + X) types through Hearth Input's virtual keyboard: characters become
/// key presses for gamescope's keyboard layout (XKB_DEFAULT_LAYOUT, as gamescope reads it), so it works in native,
/// Proton/Wine and emulator games alike. Characters that layout can't type go through xdotool on gamescope's X
/// display when it's there; the rest come back as `skipped`.
pub fn type_text(text: &str, mods: &Value) -> Result<Value, String> {
    if text.is_empty() { return Ok(json!({"ok": true, "skipped": ""})); }
    let mods: Vec<&str> = mods.as_array().map(|m| m.iter().filter_map(Value::as_str).filter(|m| matches!(*m, "ctrl" | "alt")).collect()).unwrap_or_default();
    let mut cmd = json!({"cmd": "type", "text": text.chars().take(2000).collect::<String>(), "mods": mods});
    if let Ok(l) = std::env::var("XKB_DEFAULT_LAYOUT") { cmd["layout"] = json!(l); }
    if let Ok(d) = std::env::var("DISPLAY") { cmd["display"] = json!(d); }
    let r = request(&cmd, Duration::from_secs(2)).map_err(|_| "Can't type into the game: Hearth Input isn't running".to_string())?;
    if r["ok"] == false { return Err(r["error"].as_str().unwrap_or("Couldn't type that").to_string()); }
    Ok(r)
}
/// A key for the keyboard over a game: bksp, enter, tab, esc, arrows, home, end, delete, paste (n times).
pub fn type_key(key: &str, n: u64, mods: &Value) -> Result<Value, String> {
    let mods: Vec<&str> = mods.as_array().map(|m| m.iter().filter_map(Value::as_str).filter(|m| matches!(*m, "ctrl" | "alt" | "shift")).collect()).unwrap_or_default();
    let r = request(&json!({"cmd": "key", "key": key, "n": n.clamp(1, 200), "mods": mods}), Duration::from_secs(2))
        .map_err(|_| "Can't type into the game: Hearth Input isn't running".to_string())?;
    if r["ok"] == false { return Err(r["error"].as_str().unwrap_or("Couldn't press that").to_string()); }
    Ok(r)
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
            c["resolved"] = resolve_in(dir, &c).unwrap_or(Value::Null);
            c["global"] = global_in(dir);
            Ok(c)
        }
        "input.global.get" => Ok(global_in(dir)),
        "input.global.set" => save_global_in(dir, a["enabled"].as_bool(), a.get("layout")),
        "input.validate" => {
            let l = a.get("layout").ok_or("No layout")?;
            validate(l)?;
            Ok(json!({"ok": true, "layout": migrate(l)?}))
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
        "osk.type" => type_text(a["text"].as_str().unwrap_or(""), &a["mods"]),
        "osk.key" => type_key(a["key"].as_str().unwrap_or(""), a["n"].as_u64().unwrap_or(1), &a["mods"]),
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
        assert_eq!(ids, vec!["shooter", "pointclick", "retro", "mouse", "rts", "browser"]);
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
        bad(|l| { l["version"] = json!(1); l["buttons"] = json!({"a": {"press": "shift", "hold": "key:B"}}); }, "tap and hold");
        bad(|l| l["buttons"]["rt"] = json!({"press": "mouse:left", "threshold": 2}), "between");
        bad(|l| l["buttons"]["rt"] = json!({"press": "mouse:left", "threshold": true}), "number");
        bad(|l| l["sticks"]["rs"]["sensitivity"] = json!(50), "between");
        bad(|l| l["sticks"]["rs"]["up"] = json!("key:W"), "unknown setting");
        bad(|l| { l["sticks"]["rs"]["deadzone"] = json!(0.5); l["sticks"]["rs"]["outer"] = json!(0.55); }, "outer");
        bad(|l| l["sticks"]["ls"] = json!({"mode": "teleport"}), "mode");
        bad(|l| l["sticks"]["ls"]["up"] = json!("radial"), "can't be used here");
        bad(|l| { l["version"] = json!(1); l["shift"] = json!({"buttons": {"a": "shift"}}); }, "can't be used here");
        bad(|l| l["shift"] = json!({"buttons": {}}), "unknown setting");
        bad(|l| l["radial"] = json!(vec!["key:A"; 9]), "up to 8");
        bad(|l| l["radial"] = json!(["gyro:toggle"]), "can't be used here");
        bad(|l| l["name"] = json!("x".repeat(61)), "name");
        bad(|l| l["name"] = json!("bad\nname"), "name");
        bad(|l| l["base"] = json!("nope"), "template");
        bad(|l| l["hold_ms"] = json!(20), "between");
        bad(|l| l["hold_ms"] = json!(300.5), "whole");
        bad(|l| l["version"] = json!(3), "version");
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
        assert_eq!(r["buttons"]["lb"], "layer:shift");
        assert_eq!(r["buttons"]["menu"], "key:ESC");
        assert_eq!(r["buttons"]["a"], "key:SPACE");
        assert_eq!(r["buttons"]["rb"], "key:LEFTCTRL");
        assert_eq!(r["buttons"]["view"], "radial");
        assert_eq!(r["layers"][0]["id"], "shift");
        assert_eq!(r["layers"][0]["buttons"]["up"], "key:1");
        assert_eq!(r["layers"][0]["buttons"]["left"], "key:4");
        assert_eq!(r["layers"][0]["buttons"]["y"], "key:LEFTCTRL+1");
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
    fn launcher_windows_start_with_the_mouse() {
        let d = tempfile::tempdir().unwrap();
        let dir = d.path().to_path_buf();
        assert_eq!(load_in(&dir, "xodus:nexus")["layout"], "mouse");
        assert_eq!(load_in(&dir, "battlenet:app")["layout"], "mouse");
        assert_eq!(resolve(&load_in(&dir, "battlenet:app")).unwrap()["name"], "Mouse only");
        // Their games have controller support of their own.
        assert_eq!(load_in(&dir, "battlenet:overwatch")["layout"], "off");
        assert_eq!(load_in(&dir, "xodus:game:forza-horizon-5")["layout"], "off");
        save_in(&dir, "xodus:nexus", Some("off"), None, None).unwrap();
        assert_eq!(load_in(&dir, "xodus:nexus")["layout"], "off");
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
    fn log_line() {
        assert_eq!(describe(&json!({"game": null})), "no game running");
        assert_eq!(describe(&json!({"game": "gog:1", "front": true, "overlay": false, "layout": {"name": "Strategy (RTS)"}})), "gog:1 in front, layout Strategy (RTS)");
        assert_eq!(describe(&json!({"game": "gog:1", "front": false, "overlay": true, "layout": null})), "gog:1 Quick Access open, layout off (the game gets the controller)");
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

    /// The cases hearth-input's tests check too: both sides accept and refuse the same layouts.
    #[test]
    fn shared_cases() {
        let cases: Value = serde_json::from_str(include_str!("../../linux/test/fixtures/input-layout-cases.json")).unwrap();
        for l in cases["good"].as_array().unwrap() { validate(l).unwrap_or_else(|e| panic!("{l}: {e}")); }
        for c in cases["bad"].as_array().unwrap() {
            let e = validate(&c[0]).expect_err(&format!("{} should be refused", c[0]));
            assert!(e.contains(c[1].as_str().unwrap()), "{}: {e:?} should mention {}", c[0], c[1]);
        }
    }

    #[test]
    fn version_1_files_are_read_and_migrated() {
        let d = tempfile::tempdir().unwrap();
        let dir = d.path().to_path_buf();
        let old = json!({"version": 1, "name": "Mine", "buttons": {"lb": "shift", "a": "key:J"}, "shift": {"buttons": {"a": "key:K"}}});
        std::fs::write(dir.join(file_name("steam:7").unwrap()), json!({"version": 1, "layout": "custom", "custom": old, "prefs": {"sensitivity": 1.5}}).to_string()).unwrap();
        let c = load_in(&dir, "steam:7");
        assert_eq!(c["layout"], "custom");
        assert_eq!(c["custom"]["version"], 2);
        assert_eq!(c["custom"]["buttons"]["lb"], "layer:shift");
        assert_eq!(c["custom"]["layers"][0]["buttons"]["a"], "key:K");
        assert!(c["custom"].get("shift").is_none());
        assert_eq!(c["prefs"]["sensitivity"], 1.5);
        // Saved again as version 2.
        save_in(&dir, "steam:7", None, None, None).unwrap();
        let raw: Value = serde_json::from_str(&std::fs::read_to_string(dir.join(file_name("steam:7").unwrap())).unwrap()).unwrap();
        assert_eq!(raw["version"], 2);
        assert_eq!(raw["custom"]["layers"][0]["id"], "shift");
        // A layout with no version that uses "shift" is version 1; one without is version 2.
        assert!(validate(&json!({"buttons": {"lb": "shift"}})).is_ok());
        assert!(validate(&json!({"buttons": {"lb": "layer:x"}, "layers": [{"id": "x"}]})).is_ok());
        assert_eq!(migrate(&json!({"buttons": {"lb": "shift"}})).unwrap()["layers"][0]["id"], "shift");
    }

    #[test]
    fn global_layout() {
        let d = tempfile::tempdir().unwrap();
        let dir = d.path().to_path_buf();
        assert_eq!(global_in(&dir), json!({"enabled": false, "layout": null}));
        assert!(save_global_in(&dir, Some(true), None).is_err());           // nothing to turn on yet
        assert!(save_in(&dir, "steam:1", Some("global"), None, None).is_err());
        let g = save_global_in(&dir, None, Some(template("mouse").unwrap())).unwrap();
        assert_eq!((g["enabled"].as_bool(), g["layout"]["name"].as_str()), (Some(false), Some("Mouse only")));
        assert_eq!(load_in(&dir, "steam:1")["layout"], "off");             // not on: games start Off as before
        save_global_in(&dir, Some(true), None).unwrap();
        assert_eq!(load_in(&dir, "steam:1")["layout"], "global");          // on: games without a choice use it
        assert_eq!(state_in(&dir, Some("steam:1"), true, false)["layout"]["name"], "Mouse only");
        assert_eq!(load_in(&dir, "beacon:rotwk")["layout"], "rts");         // their own default still wins
        save_in(&dir, "steam:2", Some("off"), None, None).unwrap();
        assert_eq!(load_in(&dir, "steam:2")["layout"], "off");             // and so does a choice
        save_in(&dir, "steam:3", Some("global"), None, None).unwrap();
        assert_eq!(load_in(&dir, "steam:3")["layout"], "global");
        assert!(save_global_in(&dir, None, Some(&json!({"buttons": {"a": "key:NOPE"}}))).is_err());
        let own = own_layouts_in(&dir);
        assert_eq!(own[0]["game"], "_global");
        assert_eq!(decode_name("steam%3A440"), "steam:440");
        assert_eq!(decode_name("bad%zz%4"), "bad%zz%4");
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
