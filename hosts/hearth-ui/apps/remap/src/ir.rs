//! Input Remapper (GPL-3, github.com/sezanzeb/input-remapper) as Hearth Remap drives it: its presets are the one place
//! a button's change lives (~/.config/input-remapper-2/presets/<device>/<profile>.json, its own format, written
//! atomically with a .bak by settings_file and merged: an entry Hearth doesn't understand is kept as it was), its
//! service does the remapping (root, input-remapper.service), and `input-remapper-control` starts and stops a preset
//! on a device. What only Hearth has (when a profile turns on, notes, macros' names, the dead zone and pointer speed as
//! the board shows them) is in Hearth's own remap.json.

use serde_json::{json, Value};
use std::path::PathBuf;

pub const APP: &str = "remap";

// Linux input codes (linux/input-event-codes.h)
pub const EV_KEY: u64 = 1;
pub const EV_REL: u64 = 2;
pub const EV_ABS: u64 = 3;

/// A control on the controller (or a key of a keyboard / a mouse button, by its code).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ctl {
    Btn(u16),
    LeftStick,
    RightStick,
    Dpad,
    Lt,
    Rt,
}

pub const CONTROLS: [(Ctl, &str); 15] = [
    (Ctl::Btn(310), "LB"),
    (Ctl::Btn(311), "RB"),
    (Ctl::Btn(307), "X"),
    (Ctl::Btn(308), "Y"),
    (Ctl::Btn(304), "A"),
    (Ctl::Btn(305), "B"),
    (Ctl::LeftStick, "Left stick"),
    (Ctl::RightStick, "Right stick"),
    (Ctl::Dpad, "D-pad ↑"),
    (Ctl::Btn(314), "View"),
    (Ctl::Btn(315), "≡"),
    (Ctl::Btn(317), "Left stick press"),
    (Ctl::Btn(318), "Right stick press"),
    (Ctl::Lt, "LT"),
    (Ctl::Rt, "RT"),
];

pub fn ctl_name(c: Ctl) -> String {
    CONTROLS.iter().find(|x| x.0 == c).map(|x| x.1.to_string()).unwrap_or_else(|| match c {
        Ctl::Btn(272) => "Left button".into(),
        Ctl::Btn(273) => "Right button".into(),
        Ctl::Btn(274) => "Middle button".into(),
        Ctl::Btn(275) => "Mouse 4".into(),
        Ctl::Btn(276) => "Mouse 5".into(),
        Ctl::Btn(k) => key_label(k),
        _ => "?".into(),
    })
}
pub fn ctl_id(c: Ctl) -> String {
    match c {
        Ctl::Btn(k) => format!("btn:{k}"),
        Ctl::LeftStick => "ls".into(),
        Ctl::RightStick => "rs".into(),
        Ctl::Dpad => "dpad".into(),
        Ctl::Lt => "lt".into(),
        Ctl::Rt => "rt".into(),
    }
}
pub fn ctl_of(id: &str) -> Option<Ctl> {
    Some(match id {
        "ls" => Ctl::LeftStick,
        "rs" => Ctl::RightStick,
        "dpad" => Ctl::Dpad,
        "lt" => Ctl::Lt,
        "rt" => Ctl::Rt,
        _ => Ctl::Btn(id.strip_prefix("btn:")?.parse().ok()?),
    })
}

/// A keyboard key's name from its code (the common ones), for a keyboard's or mouse's own buttons.
pub fn key_label(k: u16) -> String {
    match k {
        1 => "Esc".into(),
        2..=10 => (k - 1).to_string(),
        11 => "0".into(),
        59..=68 => format!("F{}", k - 58),
        87 => "F11".into(),
        88 => "F12".into(),
        _ => format!("Key {k}"),
    }
}

/// What a control does now.
#[derive(Debug, Clone, PartialEq)]
pub enum Out {
    /// a key or a combination (input-remapper's symbols: "Shift_L", "Control_L + 1", "F12")
    Key(String),
    Click(&'static str),
    /// the stick moves the pointer
    Pointer,
    /// the stick presses the arrow keys
    Arrows,
    /// one of Hearth's macros (its id)
    Macro(String),
    /// the D-pad's four directions, each a key (up, right, down, left)
    Four([String; 4]),
}

/// "Shift_L" → "Shift", "Control_L + 1" → "Ctrl + 1".
pub fn words(sym: &str) -> String {
    sym.split('+')
        .map(|p| {
            let p = p.trim();
            match p {
                "Shift_L" | "Shift_R" => "Shift".to_string(),
                "Control_L" | "Control_R" => "Ctrl".to_string(),
                "Alt_L" | "Alt_R" => "Alt".to_string(),
                "Super_L" => "Super".to_string(),
                "Escape" => "Esc".to_string(),
                "Return" => "Enter".to_string(),
                "space" => "Space".to_string(),
                x if x.len() == 1 => x.to_uppercase(),
                x => x.to_string(),
            }
        })
        .collect::<Vec<_>>()
        .join(" + ")
}
/// What some keys are for, said after them ("F12 (screenshot)": Steam's screenshot key).
fn hint(sym: &str) -> Option<&'static str> {
    match sym {
        "F12" => Some("screenshot"),
        "Print" => Some("screenshot"),
        "F5" => Some("refresh"),
        _ => None,
    }
}
pub fn out_words(o: &Out, macro_name: impl Fn(&str) -> String) -> String {
    match o {
        Out::Key(s) => match hint(s) {
            Some(h) => format!("{} ({h})", words(s)),
            None => words(s),
        },
        Out::Click("left") => "Left click".into(),
        Out::Click("right") => "Right click".into(),
        Out::Click(_) => "Middle click".into(),
        Out::Pointer => "Mouse pointer".into(),
        Out::Arrows => "Arrow keys".into(),
        Out::Macro(id) => format!("Macro: {}", macro_name(id)),
        Out::Four(k) => {
            let ks: Vec<String> = k.iter().map(|x| words(x)).collect();
            let common = ks.iter().map(|x| x.rsplit_once(" + ").map(|y| y.0.to_string()).unwrap_or_default()).collect::<Vec<_>>();
            if !common[0].is_empty() && common.iter().all(|c| *c == common[0]) {
                format!("{} + {}", common[0], ks.iter().map(|x| x.rsplit_once(" + ").map(|y| y.1.to_string()).unwrap_or_default()).collect::<Vec<_>>().join(" · "))
            } else {
                ks.join(" · ")
            }
        }
    }
}

/// One change, with what only Hearth keeps about it.
#[derive(Debug, Clone, PartialEq)]
pub struct Change {
    pub ctl: Ctl,
    pub out: Out,
}

/// A macro step (board: press, hold, let go, type words, click, move the pointer, scroll, wait).
#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    Press(String),
    Hold(String),
    LetGo(String),
    Type(String),
    Click(&'static str),
    Move(&'static str, u16),
    Scroll(&'static str),
    Wait(u32),
    /// run a command (its slot 1–12 and the command): Input Remapper presses Super + Ctrl + Alt + F12+slot, which
    /// Hearth's own keybind in labwc (rc.xml) turns into `hearth-remap-app --run-step SLOT`, run as you (never root).
    Run(u8, String),
}

pub const STEP_KINDS: [&str; 9] = ["press", "hold", "let go", "type words", "click", "move the pointer", "scroll", "wait", "run a command"];
/// How many commands macros can run (Super + Ctrl + Alt + F13 … F24).
pub const RUN_SLOTS: u8 = 12;

/// The key Input Remapper presses for a run slot (1 → F13).
pub fn run_key(slot: u8) -> String {
    format!("modify(Super_L, modify(Control_L, modify(Alt_L, key(F{}))))", 12 + slot.clamp(1, RUN_SLOTS))
}

/// The first run slot no macro uses yet (one slot a command; the one a step had is kept when its command changes).
pub fn free_slot(ms: &[Macro]) -> Option<u8> {
    (1..=RUN_SLOTS).find(|n| !ms.iter().flat_map(|m| m.steps.iter()).any(|s| matches!(s, Step::Run(k, _) if k == n)))
}

/// The command a run slot runs, from Hearth's own remap.json (the one place macros live).
pub fn command_of(settings: &Value, slot: u8) -> Option<String> {
    macros_of(settings).into_iter().flat_map(|m| m.steps).find_map(|s| match s {
        Step::Run(k, c) if k == slot && !c.trim().is_empty() => Some(c),
        _ => None,
    })
}

impl Step {
    pub fn kind(&self) -> &'static str {
        match self {
            Step::Press(_) => "press",
            Step::Hold(_) => "hold",
            Step::LetGo(_) => "let go",
            Step::Type(_) => "type words",
            Step::Click(_) => "click",
            Step::Move(..) => "move the pointer",
            Step::Scroll(_) => "scroll",
            Step::Wait(_) => "wait",
            Step::Run(..) => "run a command",
        }
    }
    /// The board's words ("Press B", "Wait 50 ms", "Click where the pointer is").
    pub fn words(&self) -> (String, Option<String>) {
        match self {
            Step::Press(k) => ("Press".into(), Some(words(k))),
            Step::Hold(k) => ("Hold".into(), Some(words(k))),
            Step::LetGo(k) => ("Let go of".into(), Some(words(k))),
            Step::Type(s) => (format!("Type \"{s}\""), None),
            Step::Click("right") => ("Right-click where the pointer is".into(), None),
            Step::Click(_) => ("Click where the pointer is".into(), None),
            Step::Move(d, n) => (format!("Move the pointer {d} {n}"), None),
            Step::Scroll(d) => (format!("Scroll {d}"), None),
            Step::Wait(ms) => (format!("Wait {ms} ms"), None),
            Step::Run(_, c) => ("Run".into(), Some(if c.chars().count() > 28 { format!("{}…", c.chars().take(27).collect::<String>()) } else { c.clone() })),
        }
    }
    /// How long it takes: its wait, or a key's press and release (input-remapper's keystroke_sleep_ms each).
    pub fn ms(&self, ks: u32) -> u32 {
        match self {
            Step::Wait(ms) => *ms,
            Step::Type(s) => 2 * ks * s.chars().count() as u32,
            _ => 2 * ks,
        }
    }
    fn to_json(&self) -> Value {
        match self {
            Step::Press(k) => json!(["press", k]),
            Step::Hold(k) => json!(["hold", k]),
            Step::LetGo(k) => json!(["letgo", k]),
            Step::Type(s) => json!(["type", s]),
            Step::Click(b) => json!(["click", b]),
            Step::Move(d, n) => json!(["move", d, n]),
            Step::Scroll(d) => json!(["scroll", d]),
            Step::Wait(ms) => json!(["wait", ms]),
            Step::Run(slot, c) => json!(["run", c, slot]),
        }
    }
    fn from_json(v: &Value) -> Option<Step> {
        let a = v.as_array()?;
        let s = |i: usize| a.get(i).and_then(Value::as_str).unwrap_or("").to_string();
        Some(match a.first()?.as_str()? {
            "press" => Step::Press(s(1)),
            "hold" => Step::Hold(s(1)),
            "letgo" => Step::LetGo(s(1)),
            "type" => Step::Type(s(1)),
            "click" => Step::Click(if s(1) == "right" { "right" } else { "left" }),
            "move" => Step::Move(dir(&s(1)), a.get(2).and_then(Value::as_u64).unwrap_or(10).min(500) as u16),
            "scroll" => Step::Scroll(if s(1) == "up" { "up" } else { "down" }),
            "wait" => Step::Wait(a.get(1).and_then(Value::as_u64).unwrap_or(50).min(60_000) as u32),
            "run" => Step::Run(a.get(2).and_then(Value::as_u64).unwrap_or(1).clamp(1, RUN_SLOTS as u64) as u8, s(1)),
            _ => return None,
        })
    }
}
fn dir(s: &str) -> &'static str {
    match s {
        "up" => "up",
        "down" => "down",
        "left" => "left",
        _ => "right",
    }
}

/// One of Hearth's macros: its steps, its name and colour, how it runs (once | held | toggle).
#[derive(Debug, Clone, PartialEq)]
pub struct Macro {
    pub id: String,
    pub name: String,
    pub colour: String,
    pub mode: String,
    pub steps: Vec<Step>,
}

impl Macro {
    /// How long it runs, in seconds, rounded up to a tenth.
    pub fn seconds(&self, ks: u32) -> f32 {
        let ms = self.steps.iter().map(|s| s.ms(ks)).sum::<u32>();
        (ms as f32 / 100.0).ceil() / 10.0
    }
    /// Input Remapper's macro language for it (its own: key, key_down, key_up, wait, mouse, wheel, modify, hold,
    /// if_eq/set for Turn on/off).
    pub fn compile(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        for s in &self.steps {
            match s {
                Step::Press(k) => parts.push(press(k)),
                Step::Hold(k) => parts.push(format!("key_down({})", first_key(k))),
                Step::LetGo(k) => parts.push(format!("key_up({})", first_key(k))),
                Step::Type(t) => {
                    for c in t.chars().take(200) {
                        if let Some(k) = char_key(c) {
                            parts.push(k);
                        }
                    }
                }
                Step::Click(b) => parts.push(format!("key({})", if *b == "right" { "BTN_RIGHT" } else { "BTN_LEFT" })),
                Step::Move(d, n) => parts.push(format!("mouse({d}, {n})")),
                Step::Scroll(d) => parts.push(format!("wheel({d}, 1)")),
                Step::Wait(ms) => parts.push(format!("wait({ms})")),
                Step::Run(slot, _) => parts.push(run_key(*slot)),
            }
        }
        if parts.is_empty() {
            parts.push("wait(1)".into());
        }
        let seq = parts.join(".");
        match self.mode.as_str() {
            "held" => format!("hold({seq})"),
            // Turn on/off: the first press does the steps and keeps held keys down, the next press lets them go
            "toggle" => {
                let ups: Vec<String> = self.steps.iter().filter_map(|s| if let Step::Hold(k) = s { Some(format!("key_up({})", first_key(k))) } else { None }).collect();
                let off = if ups.is_empty() { "wait(1)".to_string() } else { ups.join(".") };
                let var = format!("hearth_{}", self.id.replace(|c: char| !c.is_ascii_alphanumeric(), "_"));
                format!("if_eq(${var}, 1, set({var}, 0).{off}, set({var}, 1).{seq})")
            }
            _ => seq,
        }
    }
}

fn first_key(k: &str) -> String {
    k.split('+').next().unwrap_or("").trim().to_string()
}
/// A key or a combination as a step: "Control_L + 1" → modify(Control_L, key(1)).
fn press(k: &str) -> String {
    let ks: Vec<&str> = k.split('+').map(str::trim).filter(|x| !x.is_empty()).collect();
    match ks.split_last() {
        Some((last, mods)) if !mods.is_empty() => {
            let mut s = format!("key({last})");
            for m in mods.iter().rev() {
                s = format!("modify({m}, {s})");
            }
            s
        }
        Some((last, _)) => format!("key({last})"),
        None => "wait(1)".into(),
    }
}
fn char_key(c: char) -> Option<String> {
    let sym = match c {
        'a'..='z' | '0'..='9' => c.to_string(),
        'A'..='Z' => return Some(format!("modify(Shift_L, key({}))", c.to_ascii_lowercase())),
        ' ' => "space".into(),
        '.' => "period".into(),
        ',' => "comma".into(),
        '-' => "minus".into(),
        '\n' => "Return".into(),
        '!' => return Some("modify(Shift_L, key(1))".into()),
        '?' => return Some("modify(Shift_L, key(slash))".into()),
        '\'' => "apostrophe".into(),
        _ => return None,
    };
    Some(format!("key({sym})"))
}

// ------------------------------------------------------------------ presets (input-remapper's files)
fn ir_dir() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| hearth_ui::paths::home().join(".config")).join("input-remapper-2")
}
pub fn preset_path(device: &str, preset: &str) -> PathBuf {
    ir_dir().join("presets").join(safe(device)).join(format!("{}.json", safe(preset)))
}
fn safe(s: &str) -> String {
    s.replace('/', "-")
}
/// The profiles (presets) a device has, by name.
pub fn presets(device: &str) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(ir_dir().join("presets").join(safe(device)))
        .map(|rd| rd.flatten().filter_map(|e| e.file_name().to_str().and_then(|n| n.strip_suffix(".json")).map(str::to_string)).collect())
        .unwrap_or_default();
    v.sort();
    v
}

fn mapping_ctl(m: &Value) -> Option<(Ctl, i64)> {
    let inp = m.get("input_combination")?.as_array()?;
    if inp.len() != 1 {
        return None;
    }
    let i = &inp[0];
    let ty = i.get("type")?.as_u64()?;
    let code = i.get("code")?.as_u64()? as u16;
    let th = i.get("analog_threshold").and_then(Value::as_i64).unwrap_or(0);
    let c = match (ty, code) {
        (EV_KEY, k) => Ctl::Btn(k),
        (EV_ABS, 0) | (EV_ABS, 1) => Ctl::LeftStick,
        (EV_ABS, 3) | (EV_ABS, 4) => Ctl::RightStick,
        (EV_ABS, 2) => Ctl::Lt,
        (EV_ABS, 5) => Ctl::Rt,
        (EV_ABS, 16) | (EV_ABS, 17) => Ctl::Dpad,
        _ => return None,
    };
    Some((c, (code as i64) * 1000 + th.signum()))
}

/// A preset read back as Hearth's changes; what Hearth doesn't understand is kept aside, untouched.
pub fn read_preset(device: &str, preset: &str, macro_of: impl Fn(&str) -> Option<String>) -> (Vec<Change>, Vec<Value>) {
    let v: Value = std::fs::read(preset_path(device, preset)).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null);
    let maps: Vec<Value> = match &v {
        Value::Array(a) => a.clone(),
        // input-remapper 1.x's "mapping" object: kept as it is, shown as nothing Hearth can edit
        _ => Vec::new(),
    };
    let mut out: Vec<Change> = Vec::new();
    let mut kept: Vec<Value> = Vec::new();
    let mut dpad: [String; 4] = Default::default();
    let mut dpad_seen = false;
    let mut sticks: Vec<(Ctl, &'static str)> = Vec::new();
    for m in &maps {
        let Some((ctl, k)) = mapping_ctl(m) else {
            kept.push(m.clone());
            continue;
        };
        let sym = m.get("output_symbol").and_then(Value::as_str).unwrap_or("").to_string();
        let analog_out = m.get("output_type").and_then(Value::as_u64) == Some(EV_REL);
        match ctl {
            Ctl::Dpad if !sym.is_empty() => {
                dpad_seen = true;
                let i = match k {
                    16_999 => 0, // up: ABS_HAT0Y below zero
                    16_001 => 1,
                    17_001 => 2,
                    _ => 3,
                };
                dpad[i] = unmacro(&sym);
            }
            Ctl::LeftStick | Ctl::RightStick if analog_out => {
                if !sticks.iter().any(|s| s.0 == ctl) {
                    sticks.push((ctl, "pointer"));
                }
            }
            Ctl::LeftStick | Ctl::RightStick if matches!(sym.as_str(), "Left" | "Right" | "Up" | "Down") => {
                if !sticks.iter().any(|s| s.0 == ctl) {
                    sticks.push((ctl, "arrows"));
                }
            }
            Ctl::Btn(_) | Ctl::Lt | Ctl::Rt if !sym.is_empty() => {
                let o = match sym.as_str() {
                    "BTN_LEFT" => Out::Click("left"),
                    "BTN_RIGHT" => Out::Click("right"),
                    "BTN_MIDDLE" => Out::Click("middle"),
                    s => match macro_of(s) {
                        Some(id) => Out::Macro(id),
                        None => Out::Key(unmacro(s)),
                    },
                };
                out.push(Change { ctl, out: o });
            }
            _ => kept.push(m.clone()),
        }
    }
    for (c, how) in sticks {
        out.push(Change { ctl: c, out: if how == "pointer" { Out::Pointer } else { Out::Arrows } });
    }
    if dpad_seen {
        out.push(Change { ctl: Ctl::Dpad, out: Out::Four(dpad) });
    }
    (out, kept)
}
/// modify(Control_L, key(1)) → "Control_L + 1"; key(F12) → "F12".
fn unmacro(s: &str) -> String {
    let mut mods = Vec::new();
    let mut rest = s.trim();
    while let Some(r) = rest.strip_prefix("modify(") {
        let Some((m, more)) = r.split_once(',') else { break };
        mods.push(m.trim().to_string());
        rest = more.trim().strip_suffix(')').unwrap_or(more.trim());
    }
    let key = rest.strip_prefix("key(").and_then(|r| r.strip_suffix(')')).unwrap_or(rest).to_string();
    mods.push(key);
    mods.join(" + ")
}

fn one(ty: u64, code: u64, th: Option<i64>) -> Value {
    let mut i = json!({"type": ty, "code": code});
    if let Some(t) = th {
        i["analog_threshold"] = json!(t);
    }
    json!([i])
}
fn key_map(inp: Value, sym: &str, target: &str) -> Value {
    json!({"input_combination": inp, "target_uinput": target, "output_symbol": sym, "mapping_type": "key_macro"})
}

/// Hearth's changes as input-remapper's mappings (dead zone 0..1, pointer speed 0..1 as its gain).
pub fn to_mappings(ch: &[Change], macro_code: impl Fn(&str) -> Option<String>, deadzone: f32, speed: f32) -> Vec<Value> {
    let th = ((deadzone.clamp(0.0, 0.9) * 100.0).round() as i64).max(10);
    let mut v = Vec::new();
    for c in ch {
        match (&c.ctl, &c.out) {
            (Ctl::LeftStick | Ctl::RightStick, Out::Pointer) => {
                let (x, y) = if c.ctl == Ctl::LeftStick { (0, 1) } else { (3, 4) };
                for (code, rel) in [(x, 0), (y, 1)] {
                    v.push(json!({"input_combination": one(EV_ABS, code, Some(0)), "target_uinput": "mouse", "output_type": EV_REL, "output_code": rel,
                                  "mapping_type": "analog", "deadzone": deadzone.clamp(0.0, 0.9), "gain": 0.2 + speed.clamp(0.0, 1.0) * 1.8, "expo": 0.0}));
                }
            }
            (Ctl::LeftStick | Ctl::RightStick, Out::Arrows) => {
                let (x, y) = if c.ctl == Ctl::LeftStick { (0, 1) } else { (3, 4) };
                for (code, t, sym) in [(x, -th, "Left"), (x, th, "Right"), (y, -th, "Up"), (y, th, "Down")] {
                    v.push(key_map(one(EV_ABS, code, Some(t)), sym, "keyboard"));
                }
            }
            (Ctl::Dpad, Out::Four(k)) => {
                for (i, (code, t)) in [(17u64, -1i64), (16, 1), (17, 1), (16, -1)].into_iter().enumerate() {
                    if !k[i].is_empty() {
                        v.push(key_map(one(EV_ABS, code, Some(t)), &press(&k[i]), "keyboard"));
                    }
                }
            }
            (ctl, out) => {
                let inp = match ctl {
                    Ctl::Btn(k) => one(EV_KEY, *k as u64, None),
                    Ctl::Lt => one(EV_ABS, 2, Some(50)),
                    Ctl::Rt => one(EV_ABS, 5, Some(50)),
                    Ctl::Dpad => one(EV_ABS, 17, Some(-1)),
                    Ctl::LeftStick => one(EV_KEY, 317, None),
                    Ctl::RightStick => one(EV_KEY, 318, None),
                };
                let (sym, target) = match out {
                    Out::Click(b) => (format!("BTN_{}", b.to_uppercase()), "mouse"),
                    Out::Macro(id) => (macro_code(id).unwrap_or_else(|| "wait(1)".into()), "keyboard + mouse"),
                    Out::Key(k) if k.contains('+') => (press(k), "keyboard"),
                    Out::Key(k) => (k.clone(), "keyboard"),
                    Out::Four(k) => (press(&k[0]), "keyboard"),
                    Out::Pointer | Out::Arrows => continue,
                };
                v.push(key_map(inp, &sym, target));
            }
        }
    }
    v
}

fn is_array(b: &[u8]) -> bool {
    serde_json::from_slice::<Value>(b).map(|v| v.is_array()).unwrap_or(false)
}
/// The preset written: Hearth's changes and everything it kept aside (atomic, a .bak first; red flag 9).
pub fn write_preset(device: &str, preset: &str, maps: Vec<Value>, kept: &[Value]) -> Result<(), String> {
    let p = preset_path(device, preset);
    if let Some(d) = p.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    let mut all = maps;
    all.extend(kept.iter().cloned());
    let bytes = serde_json::to_vec_pretty(&Value::Array(all)).map_err(|e| e.to_string())?;
    hearth_ui::settings_file::write_bytes(&p, &bytes, is_array).map(|_| ()).map_err(|e| e.to_string())
}

/// Input Remapper's own keystroke time (its config.json, macros.keystroke_sleep_ms; 10 ms when it isn't set).
pub fn keystroke_ms() -> u32 {
    let v: Value = std::fs::read(ir_dir().join("config.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null);
    v.pointer("/macros/keystroke_sleep_ms").and_then(Value::as_u64).map(|x| x.clamp(1, 1000) as u32).unwrap_or(10)
}

// ------------------------------------------------------------------ the service and its control
pub fn installed() -> bool {
    if std::env::var("HEARTH_REMAP_FIXTURE").is_ok() {
        return std::env::var("HEARTH_REMAP_INSTALLED").map(|v| v != "0").unwrap_or(true);
    }
    crate::kit::have("input-remapper-control")
}
/// Is its service answering (D-Bus inputremapper.Control on the system bus)?
pub fn running() -> bool {
    if std::env::var("HEARTH_REMAP_FIXTURE").is_ok() {
        return std::env::var("HEARTH_REMAP_RUNNING").map(|v| v != "0").unwrap_or(true);
    }
    std::process::Command::new("systemctl").args(["is-active", "--quiet", "input-remapper.service"]).stdin(std::process::Stdio::null()).status().map(|s| s.success()).unwrap_or(false)
}
/// What a device is doing: RUNNING (a preset on), or anything else.
pub fn state(device: &str) -> String {
    if let Ok(f) = std::env::var("HEARTH_REMAP_FIXTURE") {
        let v: Value = std::fs::read(f).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null);
        return v.pointer("/state").and_then(|s| s.get(device)).and_then(Value::as_str).unwrap_or("STOPPED").to_string();
    }
    let o = std::process::Command::new("busctl")
        .args(["--system", "call", "inputremapper.Control", "/inputremapper/Control", "inputremapper.Control", "get_state", "s", device])
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output();
    o.ok().map(|o| String::from_utf8_lossy(&o.stdout).trim().trim_start_matches("s ").trim_matches('"').to_string()).unwrap_or_default()
}
/// `input-remapper-control` (never with a fixture).
pub fn control(args: &[&str]) -> Result<(), String> {
    if std::env::var("HEARTH_REMAP_FIXTURE").is_ok() {
        return Ok(());
    }
    let mut v = vec!["input-remapper-control".to_string()];
    v.extend(args.iter().map(|s| s.to_string()));
    let o = hearth_ui::run::blocking(&v);
    if o.ok { Ok(()) } else { Err(o.why()) }
}
pub fn start(device: &str, preset: &str) -> Result<(), String> {
    control(&["--command", "start", "--device", device, "--preset", preset])
}
pub fn stop(device: &str) -> Result<(), String> {
    control(&["--command", "stop", "--device", device])
}
pub fn stop_all() -> Result<(), String> {
    control(&["--command", "stop-all"])
}

// ------------------------------------------------------------------ the devices
#[derive(Debug, Clone, PartialEq)]
pub struct Dev {
    pub name: String,
    /// controller | keyboard | mouse
    pub kind: String,
    pub sub: String,
    pub event: String,
}

/// The input devices worth remapping, from the kernel's list (/proc/bus/input/devices), named as the board names them.
pub fn devices() -> Vec<Dev> {
    if let Ok(f) = std::env::var("HEARTH_REMAP_FIXTURE") {
        let v: Value = std::fs::read(f).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null);
        return v.get("devices").and_then(Value::as_array).map(|a| a.iter().map(|d| Dev { name: s(d, "name"), kind: s(d, "kind"), sub: s(d, "sub"), event: String::new() }).collect()).unwrap_or_default();
    }
    let text = std::fs::read_to_string("/proc/bus/input/devices").unwrap_or_default();
    let mut out: Vec<Dev> = Vec::new();
    let mut players = 0;
    for block in text.split("\n\n") {
        let mut name = String::new();
        let mut bus = 0u32;
        let mut handlers = String::new();
        let mut keys = 0u32;
        let mut has_abs = false;
        let mut phys = String::new();
        for l in block.lines() {
            if let Some(r) = l.strip_prefix("N: Name=") {
                name = r.trim_matches('"').to_string();
            } else if let Some(r) = l.strip_prefix("I: Bus=") {
                bus = u32::from_str_radix(r.split_whitespace().next().unwrap_or("0"), 16).unwrap_or(0);
            } else if let Some(r) = l.strip_prefix("H: Handlers=") {
                handlers = r.to_string();
            } else if let Some(r) = l.strip_prefix("B: KEY=") {
                keys = r.split_whitespace().map(|w| u64::from_str_radix(w, 16).unwrap_or(0).count_ones()).sum();
            } else if l.starts_with("B: ABS=") {
                has_abs = true;
            } else if let Some(r) = l.strip_prefix("P: Phys=") {
                phys = r.to_string();
            }
        }
        let event = handlers.split_whitespace().find(|h| h.starts_with("event")).unwrap_or("").to_string();
        if name.is_empty() || event.is_empty() || name.starts_with("input-remapper") || keys < 3 {
            continue;
        }
        let joystick = handlers.contains("js");
        let mouse = handlers.contains("mouse") && !joystick;
        let kind = if joystick && has_abs { "controller" } else if mouse { "mouse" } else if keys >= 60 { "keyboard" } else { continue };
        let via = match bus {
            0x05 => "Bluetooth",
            0x03 => "USB",
            0x19 | 0x11 => "built in",
            _ if phys.is_empty() => "built in",
            _ => "USB",
        };
        let sub = match kind {
            "controller" => {
                players += 1;
                format!("{via} · player {players}")
            }
            "keyboard" => format!("{via} · {} keys", keys.min(110)),
            _ => format!("{via} · {} buttons", keys.clamp(2, 20)),
        };
        if out.iter().any(|d| d.name == name) {
            continue;
        }
        out.push(Dev { name, kind: kind.into(), sub, event });
    }
    out
}
fn s(v: &Value, k: &str) -> String {
    v.get(k).and_then(Value::as_str).unwrap_or("").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changes_go_to_input_remapper_and_come_back() {
        let ch = vec![
            Change { ctl: Ctl::Btn(310), out: Out::Key("Shift_L".into()) },
            Change { ctl: Ctl::Btn(304), out: Out::Click("left") },
            Change { ctl: Ctl::RightStick, out: Out::Pointer },
            Change { ctl: Ctl::LeftStick, out: Out::Arrows },
            Change { ctl: Ctl::Dpad, out: Out::Four(["Control_L + 1".into(), "Control_L + 2".into(), "Control_L + 3".into(), "Control_L + 4".into()]) },
            Change { ctl: Ctl::Btn(314), out: Out::Key("F12".into()) },
        ];
        let maps = to_mappings(&ch, |_| None, 0.08, 0.6);
        let dir = std::env::temp_dir().join(format!("hearth-remap-test-{}", std::process::id()));
        std::env::set_var("XDG_CONFIG_HOME", &dir);
        write_preset("Pad", "Game", maps, &[json!({"input_combination": [{"type": 1, "code": 304}, {"type": 1, "code": 305}], "output_symbol": "a"})]).unwrap();
        let (back, kept) = read_preset("Pad", "Game", |_| None);
        assert_eq!(kept.len(), 1, "what Hearth doesn't understand is kept");
        for c in &ch {
            assert!(back.contains(c), "{c:?} came back: {back:?}");
        }
        assert_eq!(out_words(&ch[4].out, |_| String::new()), "Ctrl + 1 · 2 · 3 · 4");
        assert_eq!(out_words(&ch[5].out, |_| String::new()), "F12 (screenshot)");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn macros_in_input_remappers_words() {
        let m = Macro { id: "farm".into(), name: "Build farm".into(), colour: String::new(), mode: "once".into(), steps: vec![Step::Press("b".into()), Step::Wait(50), Step::Press("f".into()), Step::Click("left"), Step::Wait(100), Step::Press("Escape".into())] };
        assert_eq!(m.compile(), "key(b).wait(50).key(f).key(BTN_LEFT).wait(100).key(Escape)");
        assert!((m.seconds(10) - 0.3).abs() < 0.01);
        assert!((m.seconds(25) - 0.4).abs() < 0.01);
        assert_eq!(press("Control_L + 1"), "modify(Control_L, key(1))");
        assert_eq!(unmacro("modify(Control_L, key(1))"), "Control_L + 1");
        let t = Macro { mode: "toggle".into(), steps: vec![Step::Hold("Shift_L".into())], ..m.clone() };
        assert_eq!(t.compile(), "if_eq($hearth_farm, 1, set(hearth_farm, 0).key_up(Shift_L), set(hearth_farm, 1).key_down(Shift_L))");
    }
}

/// Hearth's macros and profiles' extras (remap.json), read.
#[cfg(test)]
mod run_tests {
    use super::*;
    #[test]
    fn a_run_step_is_hearths_key_and_its_command_is_found() {
        let m = Macro { id: "m".into(), name: "Open notes".into(), colour: String::new(), mode: "once".into(), steps: vec![Step::Press("b".into()), Step::Run(3, "hearth-notes-app".into())] };
        assert_eq!(m.compile(), "key(b).modify(Super_L, modify(Control_L, modify(Alt_L, key(F15))))");
        let v = json!({"macros": macros_json(&[m.clone()])});
        assert_eq!(command_of(&v, 3).as_deref(), Some("hearth-notes-app"));
        assert_eq!(command_of(&v, 4), None);
        assert_eq!(free_slot(&[m]), Some(1));
        assert_eq!(Step::Run(3, "x".into()).words().0, "Run");
    }
}

pub fn macros_of(v: &Value) -> Vec<Macro> {
    v.get("macros")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|m| {
                    Some(Macro {
                        id: m.get("id")?.as_str()?.to_string(),
                        name: m.get("name").and_then(Value::as_str).unwrap_or("Macro").to_string(),
                        colour: m.get("colour").and_then(Value::as_str).unwrap_or("#ff8a3d").to_string(),
                        mode: m.get("mode").and_then(Value::as_str).filter(|x| matches!(*x, "once" | "held" | "toggle")).unwrap_or("once").to_string(),
                        steps: m.get("steps").and_then(Value::as_array).map(|s| s.iter().filter_map(Step::from_json).collect()).unwrap_or_default(),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}
pub fn macros_json(ms: &[Macro]) -> Value {
    Value::Array(ms.iter().map(|m| json!({"id": m.id, "name": m.name, "colour": m.colour, "mode": m.mode, "steps": m.steps.iter().map(Step::to_json).collect::<Vec<_>>()})).collect())
}
