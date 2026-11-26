//! libratbag (MIT) through its daemon ratbagd's D-Bus API (org.freedesktop.ratbag1 on the system bus, the same API
//! Piper (GPL-2) uses), asked with busctl: the mice it knows, each profile kept on the mouse, its DPI steps, buttons,
//! report rate and lights. A change is set on ratbagd's object, then Commit writes it to the mouse itself.
//!
//! $HEARTH_MOUSE_FIXTURE: a JSON file of the objects' properties by path (CI's pictures); writes only change it in
//! memory then (nothing on the PC is touched).

use serde_json::{json, Map, Value};
use std::sync::Mutex;

const BUS: &str = "org.freedesktop.ratbag1";
const IF: &str = "org.freedesktop.ratbag1";

static FIX: Mutex<Option<Value>> = Mutex::new(None);

fn fixture() -> Option<Value> {
    let f = std::env::var("HEARTH_MOUSE_FIXTURE").ok()?;
    let mut g = FIX.lock().ok()?;
    if g.is_none() {
        *g = std::fs::read(&f).ok().and_then(|b| serde_json::from_slice(&b).ok());
    }
    g.clone()
}

/// busctl's JSON (`{"type": "u", "data": 5}`, variants inside) as plain values.
fn plain(v: &Value) -> Value {
    match v {
        Value::Object(o) if o.contains_key("type") && o.contains_key("data") && o.len() == 2 => plain(&o["data"]),
        Value::Object(o) => Value::Object(o.iter().map(|(k, x)| (k.clone(), plain(x))).collect()),
        Value::Array(a) => Value::Array(a.iter().map(plain).collect()),
        x => x.clone(),
    }
}

fn busctl(args: &[&str]) -> Result<String, String> {
    let mut v = vec!["busctl".to_string(), "--system".into()];
    v.extend(args.iter().map(|s| s.to_string()));
    let o = hearth_ui::run::blocking(&v);
    if o.ok { Ok(o.out) } else { Err(o.why()) }
}

/// Every property of one of ratbagd's objects (iface: Manager, Device, Profile, Resolution, Button, Led).
pub fn props(path: &str, iface: &str) -> Result<Map<String, Value>, String> {
    if let Some(f) = fixture() {
        return f.get(path).and_then(Value::as_object).cloned().ok_or_else(|| format!("{path}: not in the fixture"));
    }
    let out = busctl(&["--json=short", "call", BUS, path, "org.freedesktop.DBus.Properties", "GetAll", "s", &format!("{IF}.{iface}")])?;
    let v: Value = serde_json::from_str(out.trim()).map_err(|e| e.to_string())?;
    let p = plain(&v);
    p.as_array().and_then(|a| a.first()).and_then(Value::as_object).cloned().ok_or_else(|| "ratbagd's answer couldn't be read".to_string())
}

/// Sets one property: `sig` is its D-Bus type, `args` busctl's words for the value.
pub fn set(path: &str, iface: &str, prop: &str, sig: &str, args: &[String], fixture_value: Value) -> Result<(), String> {
    if fixture().is_some() {
        if let Ok(mut g) = FIX.lock() {
            if let Some(o) = g.as_mut().and_then(|f| f.get_mut(path)).and_then(Value::as_object_mut) {
                o.insert(prop.to_string(), fixture_value);
            }
        }
        return Ok(());
    }
    let mut a: Vec<String> = vec!["set-property".into(), BUS.into(), path.into(), format!("{IF}.{iface}"), prop.into(), sig.into()];
    a.extend(args.iter().cloned());
    let r: Vec<&str> = a.iter().map(String::as_str).collect();
    busctl(&r).map(|_| ())
}

pub fn call(path: &str, iface: &str, method: &str) -> Result<(), String> {
    if fixture().is_some() {
        if method == "SetActive" {
            if let Ok(mut g) = FIX.lock() {
                if let Some(f) = g.as_mut().and_then(Value::as_object_mut) {
                    // the others of its kind aren't active any more; this one is
                    let parent = path.rsplit_once('/').map(|x| x.0.to_string()).unwrap_or_default();
                    for (k, v) in f.iter_mut() {
                        if k.rsplit_once('/').map(|x| x.0) == Some(parent.as_str()) && k.as_str() != path {
                            if let Some(o) = v.as_object_mut() {
                                if o.contains_key("IsActive") {
                                    o.insert("IsActive".into(), json!(false));
                                }
                            }
                        }
                    }
                    if let Some(o) = f.get_mut(path).and_then(Value::as_object_mut) {
                        o.insert("IsActive".into(), json!(true));
                    }
                }
            }
        }
        return Ok(());
    }
    busctl(&["call", BUS, path, &format!("{IF}.{iface}"), method]).map(|_| ())
}

fn strs(v: Option<&Value>) -> Vec<String> {
    v.and_then(Value::as_array).map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default()
}
fn u(v: Option<&Value>) -> u32 {
    v.and_then(Value::as_u64).unwrap_or(0) as u32
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Res {
    pub path: String,
    pub x: u32,
    pub y: u32,
    /// the value is (x, y): up and down can differ
    pub pair: bool,
    pub active: bool,
    pub disabled: bool,
    pub choices: Vec<u32>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Button {
    pub path: String,
    pub index: u32,
    /// 0 none, 1 a button, 2 special, 3 key, 4 macro
    pub kind: u32,
    pub value: u32,
    pub kinds: Vec<u32>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Led {
    pub path: String,
    pub index: u32,
    /// 0 off, 1 still, 2 cycle (wave), 3 breathing
    pub mode: u32,
    pub modes: Vec<u32>,
    pub colour: [u8; 3],
    /// libratbag's LED type when it says (1 logo, 2 side, 3 battery, 4 dpi, 5 wheel)
    pub kind: u32,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Profile {
    pub path: String,
    pub index: u32,
    pub name: String,
    pub enabled: bool,
    pub active: bool,
    pub rate: u32,
    pub rates: Vec<u32>,
    /// -1: the mouse can't
    pub snapping: i64,
    pub res: Vec<Res>,
    pub buttons: Vec<Button>,
    pub leds: Vec<Led>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Mouse {
    pub path: String,
    pub name: String,
    pub model: String,
    pub profiles: Vec<Profile>,
}

/// Every mouse ratbagd knows, read whole.
pub fn mice() -> Result<Vec<Mouse>, String> {
    let m = props("/org/freedesktop/ratbag1", "Manager")?;
    let mut out = Vec::new();
    for dp in strs(m.get("Devices")) {
        let d = props(&dp, "Device")?;
        let mut mouse = Mouse { path: dp.clone(), name: d.get("Name").and_then(Value::as_str).unwrap_or("Mouse").to_string(), model: d.get("Model").and_then(Value::as_str).unwrap_or("").to_string(), profiles: Vec::new() };
        for pp in strs(d.get("Profiles")) {
            let p = props(&pp, "Profile")?;
            let mut pr = Profile {
                path: pp.clone(),
                index: u(p.get("Index")),
                name: p.get("Name").and_then(Value::as_str).unwrap_or("").to_string(),
                enabled: p.get("Enabled").and_then(Value::as_bool).unwrap_or(true),
                active: p.get("IsActive").and_then(Value::as_bool).unwrap_or(false),
                rate: u(p.get("ReportRate")),
                rates: p.get("ReportRates").and_then(Value::as_array).map(|a| a.iter().filter_map(|x| x.as_u64().map(|x| x as u32)).collect()).unwrap_or_default(),
                snapping: p.get("AngleSnapping").and_then(Value::as_i64).unwrap_or(-1),
                ..Default::default()
            };
            for rp in strs(p.get("Resolutions")) {
                let r = props(&rp, "Resolution")?;
                let (x, y, pair) = match r.get("Resolution") {
                    Some(Value::Array(a)) if a.len() >= 2 => (a[0].as_u64().unwrap_or(0) as u32, a[1].as_u64().unwrap_or(0) as u32, true),
                    Some(v) => (v.as_u64().unwrap_or(0) as u32, v.as_u64().unwrap_or(0) as u32, false),
                    None => (0, 0, false),
                };
                pr.res.push(Res {
                    path: rp,
                    x,
                    y,
                    pair,
                    active: r.get("IsActive").and_then(Value::as_bool).unwrap_or(false),
                    disabled: r.get("IsDisabled").and_then(Value::as_bool).unwrap_or(false),
                    choices: r.get("Resolutions").and_then(Value::as_array).map(|a| a.iter().filter_map(|x| x.as_u64().map(|x| x as u32)).collect()).unwrap_or_default(),
                });
            }
            for bp in strs(p.get("Buttons")) {
                let b = props(&bp, "Button")?;
                let (kind, value) = match b.get("Mapping") {
                    Some(Value::Array(a)) if a.len() == 2 => (a[0].as_u64().unwrap_or(0) as u32, a[1].as_u64().unwrap_or(0) as u32),
                    _ => (0, 0),
                };
                pr.buttons.push(Button { path: bp, index: u(b.get("Index")), kind, value, kinds: b.get("ActionTypes").and_then(Value::as_array).map(|a| a.iter().filter_map(|x| x.as_u64().map(|x| x as u32)).collect()).unwrap_or_default() });
            }
            for lp in strs(p.get("Leds")) {
                let l = props(&lp, "Led")?;
                let c = l.get("Color").and_then(Value::as_array).map(|a| [a.first().and_then(Value::as_u64).unwrap_or(0) as u8, a.get(1).and_then(Value::as_u64).unwrap_or(0) as u8, a.get(2).and_then(Value::as_u64).unwrap_or(0) as u8]).unwrap_or([0, 0, 0]);
                pr.leds.push(Led { path: lp, index: u(l.get("Index")), mode: u(l.get("Mode")), modes: l.get("Modes").and_then(Value::as_array).map(|a| a.iter().filter_map(|x| x.as_u64().map(|x| x as u32)).collect()).unwrap_or_default(), colour: c, kind: u(l.get("Type")) });
            }
            pr.buttons.sort_by_key(|b| b.index);
            mouse.profiles.push(pr);
        }
        mouse.profiles.sort_by_key(|p| p.index);
        out.push(mouse);
    }
    Ok(out)
}

// ------------------------------------------------------------------ what buttons can do
pub const SPECIAL: u32 = 1 << 30;
/// libratbag's special actions (ratbag_button_action_special, after UNKNOWN = 1 << 30)
pub const DOUBLECLICK: u32 = SPECIAL + 1;
pub const WHEEL_LEFT: u32 = SPECIAL + 2;
pub const WHEEL_RIGHT: u32 = SPECIAL + 3;
pub const DPI_CYCLE_UP: u32 = SPECIAL + 7;
pub const DPI_UP: u32 = SPECIAL + 9;
pub const DPI_DOWN: u32 = SPECIAL + 10;
pub const DPI_ALTERNATE: u32 = SPECIAL + 11;
pub const PROFILE_CYCLE_UP: u32 = SPECIAL + 13;

/// The choices a button's list offers (kind, value, words).
pub const ACTIONS: [(u32, u32, &str); 13] = [
    (1, 1, "Left click"),
    (1, 2, "Right click"),
    (1, 3, "Middle click"),
    (1, 4, "Back"),
    (1, 5, "Forward"),
    (2, DPI_CYCLE_UP, "Next DPI step"),
    (2, DPI_UP, "DPI up"),
    (2, DPI_DOWN, "DPI down"),
    (2, DPI_ALTERNATE, "Sniper (slow while held)"),
    (2, PROFILE_CYCLE_UP, "Next profile"),
    (2, DOUBLECLICK, "Double click"),
    (2, WHEEL_LEFT, "Scroll left"),
    (0, 0, "Nothing"),
];

pub fn action_words(kind: u32, value: u32) -> String {
    if let Some(a) = ACTIONS.iter().find(|a| a.0 == kind && a.1 == value) {
        return a.2.to_string();
    }
    match kind {
        1 => format!("Button {value}"),
        2 if value == WHEEL_RIGHT => "Scroll right".into(),
        2 => "A special action".into(),
        3 => "A key".into(),
        4 => "A macro".into(),
        _ => "Nothing".into(),
    }
}

/// The board's names for a mouse's buttons by their place (libratbag numbers them 0…).
pub fn button_name(i: u32) -> &'static str {
    match i {
        0 => "Left button",
        1 => "Right button",
        2 => "Wheel press",
        3 => "Side, back",
        4 => "Side, front",
        5 => "DPI button",
        6 => "Wheel left",
        7 => "Wheel right",
        _ => "Extra button",
    }
}

// ------------------------------------------------------------------ changes, then Commit
pub fn commit(mouse: &str) -> Result<(), String> {
    call(mouse, "Device", "Commit")
}
pub fn set_resolution(r: &Res, x: u32, y: u32) -> Result<(), String> {
    if r.pair {
        set(&r.path, "Resolution", "Resolution", "v", &["(uu)".into(), x.to_string(), y.to_string()], json!([x, y]))
    } else {
        set(&r.path, "Resolution", "Resolution", "v", &["u".into(), x.to_string()], json!(x))
    }
}
pub fn set_disabled(r: &Res, off: bool) -> Result<(), String> {
    set(&r.path, "Resolution", "IsDisabled", "b", &[off.to_string()], json!(off))
}
pub fn set_mapping(b: &Button, kind: u32, value: u32) -> Result<(), String> {
    set(&b.path, "Button", "Mapping", "(uv)", &[kind.to_string(), "u".into(), value.to_string()], json!([kind, value]))
}
pub fn set_rate(p: &Profile, hz: u32) -> Result<(), String> {
    set(&p.path, "Profile", "ReportRate", "u", &[hz.to_string()], json!(hz))
}
pub fn set_snapping(p: &Profile, on: bool) -> Result<(), String> {
    set(&p.path, "Profile", "AngleSnapping", "i", &[(on as i32).to_string()], json!(on as i32))
}
pub fn set_enabled(p: &Profile, on: bool) -> Result<(), String> {
    set(&p.path, "Profile", "Enabled", "b", &[on.to_string()], json!(on))
}
pub fn set_name(p: &Profile, name: &str) -> Result<(), String> {
    set(&p.path, "Profile", "Name", "s", &[name.to_string()], json!(name))
}
pub fn set_led(l: &Led, mode: u32, c: [u8; 3]) -> Result<(), String> {
    set(&l.path, "Led", "Mode", "u", &[mode.to_string()], json!(mode))?;
    set(&l.path, "Led", "Color", "(uuu)", &[c[0].to_string(), c[1].to_string(), c[2].to_string()], json!([c[0], c[1], c[2]]))
}

/// Is ratbagd here (its program or its D-Bus service file)?
pub fn installed() -> bool {
    if std::env::var("HEARTH_MOUSE_FIXTURE").is_ok() {
        return std::env::var("HEARTH_MOUSE_INSTALLED").map(|v| v != "0").unwrap_or(true);
    }
    std::path::Path::new("/usr/share/dbus-1/system-services/org.freedesktop.ratbag1.service").exists() || crate::kit::have("ratbagd") || std::path::Path::new("/usr/libexec/ratbagd").exists()
}

/// The mouse's battery (UPower knows wireless mice): percent, if it says.
pub fn battery(name: &str) -> Option<u32> {
    if std::env::var("HEARTH_MOUSE_FIXTURE").is_ok() {
        return fixture()?.pointer("/battery").and_then(Value::as_u64).map(|x| x as u32);
    }
    let list = hearth_ui::run::blocking(&hearth_ui::run::args(&["upower", "-e"]));
    for dev in list.out.lines().filter(|l| l.contains("mouse")) {
        let i = hearth_ui::run::blocking(&hearth_ui::run::args(&["upower", "-i", dev.trim()]));
        let model = i.out.lines().find(|l| l.trim_start().starts_with("model:")).map(|l| l.split(':').nth(1).unwrap_or("").trim().to_lowercase()).unwrap_or_default();
        let first = name.to_lowercase().split_whitespace().next().unwrap_or("").to_string();
        if model.is_empty() || first.is_empty() || model.contains(&first) || list.out.lines().filter(|l| l.contains("mouse")).count() == 1 {
            if let Some(p) = i.out.lines().find(|l| l.trim_start().starts_with("percentage:")) {
                return p.split(':').nth(1).and_then(|x| x.trim().trim_end_matches('%').parse::<f32>().ok()).map(|x| x.round() as u32);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn busctls_variants_come_out_plain() {
        let v: Value = serde_json::from_str(r#"{"type":"a{sv}","data":[{"Name":{"type":"s","data":"G502"},"Resolution":{"type":"(uu)","data":[1600,1600]},"Mapping":{"type":"(uv)","data":[1,{"type":"u","data":3}]}}]}"#).unwrap();
        let p = plain(&v);
        assert_eq!(p[0]["Name"], json!("G502"));
        assert_eq!(p[0]["Resolution"], json!([1600, 1600]));
        assert_eq!(p[0]["Mapping"], json!([1, 3]));
    }

    #[test]
    fn button_words() {
        assert_eq!(action_words(1, 3), "Middle click");
        assert_eq!(action_words(2, DPI_CYCLE_UP), "Next DPI step");
        assert_eq!(action_words(0, 0), "Nothing");
    }
}
