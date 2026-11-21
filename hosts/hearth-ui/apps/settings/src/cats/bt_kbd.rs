//! Bluetooth & devices › Keyboard (Mantel's SetKeyboard): layouts (add, remove, put in order; the first is the one you
//! start with), the shortcut that switches between them, how fast a held key repeats, Num Lock at start, the compose
//! key and one layout for everything or per app / per window, the on-screen keyboard (Hearth's shared one,
//! docs/OSK.md: ~/.config/nimbus/osk.json, the same file as Game Mode's Settings › Keyboard) and the keyboard
//! shortcuts. Saved by hearth-system keyboard-set to ~/.config/nimbus/keyboard.json, as Mantel's page does; on KDE
//! Plasma (Fedora) the helper is told KWin is the window manager, so the same settings go into KWin's kxkbrc [Layout]
//! and kcminputrc [Keyboard] (hearth-kwin-settings keyboard-apply) and KWin takes them at once.

use crate::cats::net::kit::{self, *};
use crate::shared::Shared;
use crate::widgets::*;
use hearth_ui::run;
use hearth_ui::ui::{self, glyph, t, t1, tm, G};
use hearth_ui::Pal;
use iced::widget::{column, container, mouse_area, row, Column, Row, Space};
use iced::{mouse, Color, Length, Padding, Subscription, Task};
use serde_json::{json, Map, Value};

#[derive(Default)]
pub struct S {
    pub st: Option<Value>,
    pub layouts: Vec<(String, String, String)>,
    pub all: Vec<Value>,
    pub shortcuts: Vec<Value>,
    pub picked: Option<usize>,
    pub switch: String,
    pub delay: u32,
    pub rate: u32,
    pub numlock: Option<String>,
    pub compose: Option<String>,
    pub switch_mode: Option<String>,
    pub compose_open: bool,
    pub query: String,
    pub try_text: String,
    pub saving: bool,
    pub say: Say,
    pub osk: Map<String, Value>,
    pub osk_open: bool,
    pub osk_top: bool,
    pub kde_keys: u64,
    pub kde_busy: bool,
    pub kde_say: Say,
}

#[derive(Debug, Clone)]
pub enum M {
    Status(Res),
    All(Res),
    Shortcuts(Res),
    KdeKeys(Res),
    Pick(usize),
    Move(usize, bool),
    Remove(usize),
    Query(String),
    Add(String, String, String),
    Switch(&'static str),
    Mode(&'static str),
    Delay(u32),
    Rate(u32),
    Release,
    NumLock(&'static str),
    ComposeOpen,
    Compose(&'static str),
    Try(String),
    Saved(Res),
    OskOpen,
    TryOsk,
    OskLayout(&'static str),
    Osk(&'static str, Value),
    OskSaved(run::Output),
    ClearWords,
    Cleared(run::Output),
    KdeRestore,
    KdeRestored(Res),
}

const HS: &str = "hearth-system";

pub const SWITCHES: &[(&str, &str)] = &[("", "None"), ("grp:alt_shift_toggle", "Alt+Shift"), ("grp:ctrl_shift_toggle", "Ctrl+Shift"), ("grp:alt_space_toggle", "Alt+Space"), ("grp:caps_toggle", "Caps Lock")];
pub const COMPOSE: &[(&str, &str)] = &[("", "None"), ("compose:ralt", "Right Alt"), ("compose:rwin", "Right Super"), ("compose:menu", "Menu"), ("compose:rctrl", "Right Ctrl"), ("compose:caps", "Caps Lock")];
/// Hearth's on-screen keyboard's own layouts (osk/hearth-osk.js LAYOUTS) and settings it starts with (DEFAULTS).
pub const OSK_LAYOUTS: &[(&str, &str)] = &[
    ("auto", "Automatic"), ("en-us", "English (QWERTY)"), ("fr", "Français (AZERTY)"), ("de", "Deutsch (QWERTZ)"), ("es", "Español (QWERTY)"), ("sv", "Svenska (QWERTY)"),
    ("fi", "Suomi (QWERTY)"), ("da", "Dansk (QWERTY)"), ("nb", "Norsk (QWERTY)"), ("pt", "Português (QWERTY)"), ("it", "Italiano (QWERTY)"),
];
/// The on-screen keyboard's looks in Desktop Mode (OskThemes.qml): only colours and shapes.
pub const OSK_LOOKS: &[(&str, &str, &str)] = &[
    ("steam", "Steam-style", "Follows Appearance: your accent, dark or light"),
    ("warm", "Hearth warm", "Firelit browns and an ember glow; dark or light"),
    ("contrast", "High contrast", "Bold outlines, easiest to read"),
    ("frost", "Frosted", "See-through glass, thin outlines"),
    ("pocket", "Pocket", "Round keys, big letters for thumbs"),
    ("hearth", "Classic dark", "The keyboard's first look"),
    ("ember", "Ember glow", "Dark, with a warm glow"),
    ("slate", "Slate", "Dark blue-grey"),
    ("daylight", "Daylight", "Light"),
];

fn osk_default(k: &str) -> Value {
    match k {
        "layouts" => json!(["auto"]),
        "suggestions" | "learn" | "autocap" | "doubleSpace" | "trail" => Value::Bool(true),
        "fast" => json!("wheel"),
        "shape" => json!("full"),
        "size" => json!("m"),
        "position" => json!("bottom"),
        "theme" => json!("hearth"),
        "haptics" => json!("light"),
        _ => Value::Null,
    }
}

fn osk_path() -> std::path::PathBuf {
    let base = std::env::var("XDG_CONFIG_HOME").ok().filter(|x| !x.is_empty()).unwrap_or_else(|| format!("{}/.config", std::env::var("HOME").unwrap_or_default()));
    std::path::PathBuf::from(format!("{base}/nimbus/osk.json"))
}
fn words_path() -> String {
    let base = std::env::var("XDG_DATA_HOME").ok().filter(|x| !x.is_empty()).unwrap_or_else(|| format!("{}/.local/share", std::env::var("HOME").unwrap_or_default()));
    format!("{base}/nimbus/osk-words.json")
}

impl S {
    fn osk_get(&self, k: &str) -> Value {
        self.osk.get(k).cloned().filter(|v| !v.is_null()).unwrap_or_else(|| osk_default(k))
    }
    fn osk_str(&self, k: &str) -> String {
        self.osk_get(k).as_str().unwrap_or("").to_string()
    }
    fn osk_bool(&self, k: &str) -> bool {
        self.osk_get(k).as_bool().unwrap_or(false)
    }
    fn osk_layouts(&self) -> Vec<String> {
        let l: Vec<String> = self.osk_get("layouts").as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
        if l.is_empty() {
            vec!["auto".into()]
        } else {
            l
        }
    }
    /// The look in use: the pick from Settings › Keyboard; else the engine's older pick (not its default); else Steam-style.
    fn osk_look(&self) -> &'static str {
        let chosen = self.osk.get("deskTheme").and_then(|v| v.as_str()).unwrap_or("");
        if let Some(l) = OSK_LOOKS.iter().find(|l| l.0 == chosen) {
            return l.0;
        }
        let th = self.osk_str("theme");
        if th != "hearth" {
            if let Some(l) = OSK_LOOKS.iter().find(|l| l.0 == th) {
                return l.0;
            }
        }
        "steam"
    }
    fn kwin(&self) -> bool {
        self.st.as_ref().map(|v| kit::bo(v, "kwin")).unwrap_or(false)
    }
}

pub fn open(_sh: &mut Shared, s: &mut S, part: &str) -> Task<M> {
    s.say.clear();
    s.kde_say.clear();
    s.picked = None;
    s.query.clear();
    s.osk_open = part == "osk";
    s.osk_top = part == "osk";
    s.osk = match hearth_ui::settings_file::read_json(&osk_path()) {
        Value::Object(m) => m,
        _ => Map::new(),
    };
    Task::batch([
        Task::perform(kit::job(HS, kit::a(&["keyboard"]), String::new()), M::Status),
        Task::perform(kit::job(HS, kit::a(&["keyboard-layouts"]), String::new()), M::All),
        Task::perform(kit::job(HS, kit::a(&["shortcuts"]), String::new()), M::Shortcuts),
        // Reads only: which of KDE's own shortcuts Hearth moved once so Mantel's work on KWin.
        Task::perform(kit::job("hearth-kwin-bridge", kit::a(&["keys", "changed"]), String::new()), M::KdeKeys),
    ])
}

fn apply(s: &mut S) -> Task<M> {
    let mut req = json!({
        "layouts": s.layouts.iter().map(|(l, v, _)| json!({ "layout": l, "variant": v })).collect::<Vec<_>>(),
        "switch": s.switch, "delay": s.delay, "rate": s.rate,
    });
    if let Some(n) = &s.numlock {
        req["numlock"] = json!(n);
    }
    if let Some(c) = &s.compose {
        req["compose"] = json!(c);
    }
    if let Some(m) = &s.switch_mode {
        req["switchMode"] = json!(m);
    }
    s.saving = true;
    Task::perform(kit::job(HS, kit::a(&["keyboard-set"]), req.to_string()), M::Saved)
}

fn opt_str(v: &Value, k: &str) -> Option<String> {
    kit::at(v, k).as_str().map(String::from)
}

pub fn update(sh: &mut Shared, s: &mut S, m: M) -> Task<M> {
    match m {
        M::Status(r) => match r {
            Res::Ok(v) => {
                s.layouts = kit::ar(&v, "layouts").iter().map(|l| (kit::st(l, "layout"), kit::st(l, "variant"), kit::st(l, "name"))).collect();
                s.switch = kit::st(&v, "switch");
                s.delay = kit::nu(&v, "delay").map(|x| x as u32).filter(|x| *x > 0).unwrap_or(600);
                s.rate = kit::nu(&v, "rate").map(|x| x as u32).filter(|x| *x > 0).unwrap_or(25);
                s.numlock = opt_str(&v, "numlock");
                s.compose = opt_str(&v, "compose");
                s.switch_mode = opt_str(&v, "switchMode");
                s.st = Some(v);
            }
            Res::Err(e) => s.say.bad(e),
        },
        M::All(r) => {
            if let Res::Ok(v) = r {
                s.all = kit::ar(&v, "layouts").to_vec();
            }
        }
        M::Shortcuts(r) => {
            if let Res::Ok(v) = r {
                s.shortcuts = kit::ar(&v, "shortcuts").to_vec();
            }
        }
        M::KdeKeys(r) => {
            if let Res::Ok(v) = r {
                s.kde_keys = kit::nu(&v, "count").unwrap_or(0.0) as u64;
            }
        }
        M::Pick(i) => s.picked = if s.picked == Some(i) { None } else { Some(i) },
        M::Move(i, up) => {
            let j = if up { i.checked_sub(1) } else { Some(i + 1) };
            if let Some(j) = j.filter(|j| *j < s.layouts.len()) {
                s.layouts.swap(i, j);
                s.picked = Some(j);
                return apply(s);
            }
        }
        M::Remove(i) => {
            if s.layouts.len() > 1 && i < s.layouts.len() {
                s.layouts.remove(i);
                s.picked = None;
                return apply(s);
            }
        }
        M::Query(q) => s.query = q,
        M::Add(l, v, n) => {
            if s.layouts.len() >= 4 {
                s.say.bad("Up to 4 layouts.");
                return Task::none();
            }
            s.layouts.push((l, v, n));
            s.query.clear();
            s.picked = None;
            return apply(s);
        }
        M::Switch(k) => {
            s.switch = k.to_string();
            return apply(s);
        }
        M::Mode(k) => {
            s.switch_mode = Some(k.to_string());
            return apply(s);
        }
        M::Delay(d) => s.delay = ((d + 5) / 10 * 10).clamp(150, 1000),
        M::Rate(r) => s.rate = r.clamp(5, 60),
        M::Release => return apply(s),
        M::NumLock(k) => {
            s.numlock = Some(k.to_string());
            return apply(s);
        }
        M::ComposeOpen => s.compose_open = !s.compose_open,
        M::Compose(k) => {
            s.compose = Some(k.to_string());
            s.compose_open = false;
            return apply(s);
        }
        M::Try(t) => s.try_text = t,
        M::Saved(r) => {
            s.saving = false;
            match r {
                Res::Ok(v) => s.say.ok(if kit::bo(&v, "reloaded") { "Saved and in use." } else { "Saved. It's used from the next time you sign in to Desktop Mode." }),
                Res::Err(e) => s.say.bad(e),
            }
        }
        M::OskOpen => s.osk_open = !s.osk_open,
        M::TryOsk => {
            // KDE Plasma: Hearth Plasma's on-screen keyboard (osk-plasma.qml, started by hearth-companion) through its
            // IPC; Mantel's own pad opens it there.
            let qml = run::shell_helper("osk-plasma.qml");
            run::detached(&["quickshell", "ipc", "-p", qml.as_str(), "call", "controller", "openKeyboard"]);
        }
        M::OskLayout(id) => {
            let mut l = s.osk_layouts();
            if l.iter().any(|x| x == id) {
                if l.len() > 1 {
                    l.retain(|x| x != id);
                }
            } else {
                l.push(id.to_string());
            }
            return osk_set(sh, s, "layouts", json!(l));
        }
        M::Osk(k, v) => return osk_set(sh, s, k, v),
        M::OskSaved(o) => sh.result("Couldn't save the on-screen keyboard's settings", &o),
        M::ClearWords => {
            let cmd = vec!["python3".to_string(), crate::shared::io_helper(), "write".into(), words_path()];
            return Task::perform(kit::exec(cmd, "{}".into(), 20), M::Cleared);
        }
        M::Cleared(o) => {
            if o.ok {
                s.say.ok("Learned words cleared.");
            } else {
                s.say.bad(o.why());
            }
        }
        M::KdeRestore => {
            s.kde_busy = true;
            return Task::perform(kit::job("hearth-kwin-bridge", kit::a(&["keys", "restore", "--json"]), String::new()), M::KdeRestored);
        }
        M::KdeRestored(r) => {
            s.kde_busy = false;
            match r {
                Res::Ok(v) => {
                    s.kde_keys = kit::nu(&v, "count").unwrap_or(0.0) as u64;
                    let n = kit::ar(&v, "back").len();
                    if n == 0 {
                        s.kde_say.bad("None could be put back right now. Try again in Desktop Mode on KWin.");
                    } else if s.kde_keys > 0 {
                        s.kde_say.bad(format!("{n} put back; {} couldn't be yet. Try again in Desktop Mode on KWin.", s.kde_keys));
                    } else {
                        s.kde_say.ok("KDE's shortcuts are back. Hearth won't change them again.");
                    }
                }
                Res::Err(e) => s.kde_say.bad(e),
            }
        }
    }
    Task::none()
}

/// Only the changed key is merged into osk.json (Game Mode's Settings › Keyboard writes it too).
fn osk_set(sh: &mut Shared, s: &mut S, k: &'static str, v: Value) -> Task<M> {
    s.osk.insert(k.to_string(), v.clone());
    let mut ch = Map::new();
    ch.insert(k.to_string(), v);
    let cmd = sh.json_cmd(&osk_path(), Value::Object(ch));
    Task::perform(run::spawn(cmd), M::OskSaved)
}

fn hexc(s: &str) -> Color {
    let h = s.trim_start_matches('#');
    let n = u32::from_str_radix(h, 16).unwrap_or(0x222222);
    Color::from_rgb8(((n >> 16) & 255) as u8, ((n >> 8) & 255) as u8, (n & 255) as u8)
}

/// A look's colours for its small picture: background, keys, side keys, the picked key, its outline, key corner.
fn look_colours(p: &Pal, id: &str) -> (Color, Color, Color, Color, Color, f32) {
    let dark = p.dark;
    match id {
        "steam" => (if dark { hexc("171a21") } else { hexc("dbe2e9") }, if dark { hexc("24303f") } else { hexc("f4f7fa") }, if dark { hexc("1e2633") } else { hexc("d2dce6") }, Pal::mix(if dark { hexc("24303f") } else { Color::WHITE }, p.accent, 0.45), p.accent, 4.0),
        "warm" => (hexc("0f0b09"), hexc("261d18"), hexc("1c1512"), Pal::mix(hexc("261d18"), hexc("ff8a3d"), 0.5), hexc("ff8a3d"), 6.0),
        "contrast" => if dark { (hexc("000000"), hexc("000000"), hexc("000000"), hexc("ffd400"), hexc("ffffff"), 6.0) } else { (hexc("ffffff"), hexc("ffffff"), hexc("ffffff"), hexc("0047d6"), hexc("000000"), 6.0) },
        "frost" => (Pal::a(p.panel, 0.55), Pal::a(Color::WHITE, 0.12), Pal::a(Color::WHITE, 0.06), Pal::a(p.accent, 0.9), Pal::a(Color::WHITE, 0.2), 6.0),
        "pocket" => (if dark { hexc("0d0f14") } else { hexc("eef1f5") }, if dark { hexc("1d212b") } else { hexc("ffffff") }, if dark { hexc("14171e") } else { hexc("d7dde6") }, p.accent, p.accent, 7.0),
        "ember" => (hexc("120c0a"), hexc("2a1a14"), hexc("1f1410"), hexc("ffd9bd"), hexc("ff7a2f"), 4.0),
        "slate" => (hexc("1b2028"), hexc("2d3542"), hexc("252c37"), hexc("f2f6fb"), p.accent, 4.0),
        "daylight" => (hexc("e9e6e1"), hexc("ffffff"), hexc("d4d0c8"), hexc("1d1b20"), p.accent, 4.0),
        _ => (hexc("17171c"), hexc("2b2b32"), hexc("222228"), hexc("f6f2ea"), p.accent, 4.0),
    }
}

/// A small keyboard in a look: three rows of keys, one picked, a wide space bar.
fn mini<'a>(p: &Pal, id: &str) -> E<'a, M> {
    let (bg, key, modk, sel, edge, kr) = look_colours(p, id);
    let k = |c: Color, w: u16, lit: bool| -> E<'a, M> {
        container(Space::new()).width(Length::FillPortion(w)).height(Length::Fixed(14.0)).style(ui::boxed(c, kr, if lit { Some((edge, 1.5)) } else if id == "contrast" { Some((edge, 1.0)) } else { None })).into()
    };
    let mut rows = Column::new().spacing(3.0);
    for r in 0..3 {
        let mut line = Row::new().spacing(3.0);
        if r < 2 {
            for i in 0..8 {
                let lit = r == 1 && i == 3;
                line = line.push(k(if lit { sel } else { key }, 1, lit));
            }
        } else {
            for i in 0..5 {
                line = line.push(if i == 2 { k(key, 4, false) } else { k(modk, 1, false) });
            }
        }
        rows = rows.push(line);
    }
    container(rows).padding(6.0).width(Length::Fill).style(ui::boxed(bg, 8.0, Some((Pal::a(edge, 0.25), 1.0)))).into()
}

pub fn view<'a>(sh: &'a Shared, s: &'a S) -> E<'a, M> {
    let p = sh.p;
    let mut c = Column::new().spacing(10.0);
    c = c.push(head(&p, if s.saving { "Layouts · saving…" } else { "Layouts" }));
    let sw_label = SWITCHES.iter().find(|x| x.0 == s.switch).map(|x| x.1).unwrap_or("the shortcut below");
    for (i, (lay, _v, name)) in s.layouts.iter().enumerate() {
        let picked = s.picked == Some(i);
        c = c.push(val_row(&p, G::Grid, if name.is_empty() { lay.clone() } else { name.clone() }, if i == 0 { "Used when Desktop Mode starts".to_string() } else { format!("Switch to it with {sw_label}") }, if !picked && i == 0 { "First" } else { "" }, Some(M::Pick(i))));
        if picked {
            let mut b = Row::new().spacing(8.0);
            if i > 0 {
                b = b.push(crate::widgets::btn(&p, "Move up", hearth_ui::ui::Kind::Normal, false, Some(M::Move(i, true))));
            }
            if i + 1 < s.layouts.len() {
                b = b.push(crate::widgets::btn(&p, "Move down", hearth_ui::ui::Kind::Normal, false, Some(M::Move(i, false))));
            }
            if s.layouts.len() > 1 {
                b = b.push(crate::widgets::btn(&p, "Remove", hearth_ui::ui::Kind::Danger, false, Some(M::Remove(i))));
            }
            c = c.push(indent(b));
        }
    }
    if s.st.is_none() {
        c = c.push(words(&p, "Reading your keyboard…"));
    }
    c = c.push(input(&p, "Add a layout: type a language or country", &s.query, false, M::Query, None));
    let q = s.query.trim().to_lowercase();
    if !q.is_empty() {
        let mut n = 0;
        for l in &s.all {
            let (lay, var, name) = (kit::st(l, "layout"), kit::st(l, "variant"), kit::st(l, "name"));
            if !format!("{name} {lay} {var}").to_lowercase().contains(&q) || s.layouts.iter().any(|x| x.0 == lay && x.1 == var) {
                continue;
            }
            c = c.push(val_row(&p, G::Grid, name.clone(), format!("{lay}{}", if var.is_empty() { String::new() } else { format!(" · {var}") }), "Add", Some(M::Add(lay, var, name))));
            n += 1;
            if n >= 30 {
                break;
            }
        }
    }
    if s.layouts.len() > 1 {
        c = c.push(choice(&p, G::Grid, "Switch layouts with", "Press it to go to the next layout", SWITCHES.iter().map(|&(v, l)| (l.to_string(), s.switch == v, Some(M::Switch(v)))).collect()));
        if s.switch == "grp:alt_space_toggle" {
            c = c.push(words(&p, "Alt+Space switches layouts now, so the search bar opens with Alt+F2."));
        }
        if s.kwin() {
            let cur = s.switch_mode.as_deref().unwrap_or("Global");
            c = c.push(choice(&p, G::Grid, "Remember the layout", "One layout for everything, or each app or window keeps the one you switched it to", [("Global", "Everywhere"), ("WinClass", "Per app"), ("Window", "Per window")].iter().map(|&(v, l)| (l.to_string(), cur == v, Some(M::Mode(v)))).collect()));
        }
    }
    c = c.push(head(&p, "Typing"));
    c = c.push(srow(&p, G::Clock, "Wait before a held key repeats", format!("{} ms", s.delay), slider_u32(s.delay.clamp(150, 1000), 150..=1000, M::Delay, M::Release)));
    c = c.push(srow(&p, G::Pulse, "Repeat speed", format!("{} per second", s.rate), slider_u32(s.rate.clamp(5, 60), 5..=60, M::Rate, M::Release)));
    if s.kwin() {
        let nl = s.numlock.as_deref().unwrap_or("keep");
        c = c.push(choice(&p, G::Grid, "Num Lock when Desktop Mode starts", "Used from the next time Desktop Mode starts", [("on", "On"), ("off", "Off"), ("keep", "As it was")].iter().map(|&(v, l)| (l.to_string(), nl == v, Some(M::NumLock(v)))).collect()));
        let cp = s.compose.as_deref().unwrap_or("");
        let cur = COMPOSE.iter().find(|x| x.0 == cp).map(|x| x.1).unwrap_or("None");
        c = c.push(dropdown(&p, G::Grid, "Compose key", "Press it, then two keys, to type a letter that isn't on the keyboard: Compose, e, ' types é", cur, s.compose_open, M::ComposeOpen, COMPOSE.iter().map(|&(v, l)| (l.to_string(), cp == v, M::Compose(v))).collect()));
    }
    c = c.push(input(&p, "Try typing here", &s.try_text, false, M::Try, None));
    c = c.push(said(&p, &s.say));

    // ---- the on-screen keyboard
    let mut o = Column::new().spacing(10.0).push(head(&p, "On-screen keyboard"));
    let fast = s.osk_str("fast");
    let fast_name = match fast.as_str() {
        "wheel" => "Ember Wheel",
        "split" => "Split sticks",
        _ => "",
    };
    let body = if s.osk_open {
        let mut b = Column::new().spacing(8.0);
        b = b.push(press(&p, G::Grid, "Try the on-screen keyboard", "Y on the controller, the keyboard button on the taskbar, or a swipe in from the left edge", Some(M::TryOsk)));
        b = b.push(words(&p, "Its layouts (the globe key switches; Automatic follows the layouts above)"));
        let on = s.osk_layouts();
        for &(id, name) in OSK_LAYOUTS {
            let has = on.iter().any(|x| x == id);
            b = b.push(srow(&p, G::Grid, name, "", switch(&p, has, Some(M::OskLayout(id)))));
        }
        for (k, title, sub) in [("suggestions", "Word suggestions", "Words to pick above the keys, from a word list on this device"), ("learn", "Learn new words", "Remembers words you type (never passwords), only on this device"), ("autocap", "Capital letters", "At the start of a sentence"), ("doubleSpace", "Double space types a period", "")] {
            let v = s.osk_bool(k);
            b = b.push(srow(&p, G::Check, title, sub, switch(&p, v, Some(M::Osk(k, Value::Bool(!v))))));
        }
        b = b.push(choice(&p, G::Pad, "Fast typing", "Ember Wheel: tilt the left stick at a group of letters, A B X Y type one of its four. Split sticks: each stick points into its half, LT and RT type.", [("wheel", "Ember Wheel"), ("split", "Split sticks"), ("off", "Off")].iter().map(|&(v, l)| (l.to_string(), fast == v, Some(M::Osk("fast", json!(v))))).collect()));
        // Its look in Desktop Mode: only colours and shapes, the keys work the same in each.
        b = b.push(container(tm("Look", sz(15.0), p.text)).padding(Padding { top: 10.0, right: 0.0, bottom: 2.0, left: 4.0 }));
        let look = s.osk_look();
        let mut grid = Column::new().spacing(8.0);
        for chunk in OSK_LOOKS.chunks(3) {
            let mut r = Row::new().spacing(8.0);
            for &(id, label, sub) in chunk {
                let in_use = look == id;
                let tile = container(column![
                    mini(&p, id),
                    row![t1(label, sz(13.0), p.text).width(Length::Fill), if in_use { glyph(G::Check, p.accent, 14.0, 2.4) } else { Space::new().width(Length::Fixed(14.0)).into() }].align_y(iced::Center),
                    t1(sub, sz(11.0), p.muted),
                ].spacing(6.0))
                .padding(10.0)
                .width(Length::FillPortion(1))
                .style(ui::boxed(p.panel, p.r(10.0), Some((if in_use { Pal::a(p.accent, 0.7) } else { Color::TRANSPARENT }, 2.0))));
                r = r.push(mouse_area(tile).on_press(M::Osk("deskTheme", json!(id))).interaction(mouse::Interaction::Pointer));
            }
            grid = grid.push(r);
        }
        b = b.push(grid);
        let size = s.osk_str("size");
        b = b.push(choice(&p, G::Grid, "Size", "", [("s", "Small"), ("m", "Medium"), ("l", "Large")].iter().map(|&(v, l)| (l.to_string(), size == v, Some(M::Osk("size", json!(v))))).collect()));
        let shape = s.osk_str("shape");
        b = b.push(choice(&p, G::Grid, "Shape", "Split puts the halves at the edges, under your thumbs on a handheld's touchscreen", [("full", "Full"), ("split", "Split")].iter().map(|&(v, l)| (l.to_string(), shape == v, Some(M::Osk("shape", json!(v))))).collect()));
        let pos = s.osk_str("position");
        b = b.push(choice(&p, G::Grid, "Position", "", [("bottom", "Bottom"), ("top", "Top")].iter().map(|&(v, l)| (l.to_string(), pos == v, Some(M::Osk("position", json!(v))))).collect()));
        let trail = s.osk_bool("trail");
        b = b.push(srow(&p, G::Leaf, "Ember trail", "A warm glow follows the selection", switch(&p, trail, Some(M::Osk("trail", Value::Bool(!trail))))));
        let hap = s.osk_str("haptics");
        b = b.push(choice(&p, G::Pad, "Rumble on key presses", "", [("off", "Off"), ("light", "Light"), ("strong", "Strong")].iter().map(|&(v, l)| (l.to_string(), hap == v, Some(M::Osk("haptics", json!(v))))).collect()));
        b = b.push(danger_row(&p, G::Close, "Clear learned words", "Word suggestions go back to the word list (Game Mode's keyboard forgets them too)", Some(M::ClearWords)));
        Some(b.into())
    } else {
        None
    };
    o = o.push(expander(&p, G::Grid, "On-screen keyboard", "For the controller and touch. Opens with Y, the keyboard button on the taskbar, or a swipe in from the left edge", fast_name, None, s.osk_open, M::OskOpen, body));

    // ---- keyboard shortcuts
    let mut k = Column::new().spacing(10.0);
    if !s.shortcuts.is_empty() || s.kde_keys > 0 {
        k = k.push(head(&p, "Keyboard shortcuts"));
    }
    for sc in &s.shortcuts {
        k = k.push(info_row(&p, G::Grid, kit::st(sc, "keys"), kit::st(sc, "action")));
    }
    // KDE's own shortcuts that hearth-kwin-bridge changed once so Mantel's keys work on KWin: put back with one
    // button (red flag 9: anything Hearth changes by itself is reversible from Settings).
    if s.kde_keys > 0 {
        k = k.push(press(&p, G::Restore, "Bring back KDE's own shortcuts", format!("{}Hearth moved {} of KDE's shortcuts (Super+arrows, Super+D, Super+G, Print…) so Mantel's own work on KWin. This gives them back to KDE; Mantel's shortcuts on those keys then stop working.", if s.kde_busy { "Putting them back… " } else { "" }, s.kde_keys), Some(M::KdeRestore)));
    }
    k = k.push(said(&p, &s.kde_say));
    // `hearth-settings keyboard:osk` (Personalization's "On-screen keyboard style", the pictures): that part first.
    if s.osk_top {
        column![o, c, k].spacing(10.0).into()
    } else {
        column![c, o, k].spacing(10.0).into()
    }
}

pub fn subscription(_s: &S) -> Subscription<M> {
    Subscription::none()
}

pub fn back(s: &mut S) -> bool {
    if s.picked.is_some() || s.compose_open {
        s.picked = None;
        s.compose_open = false;
        return true;
    }
    false
}
