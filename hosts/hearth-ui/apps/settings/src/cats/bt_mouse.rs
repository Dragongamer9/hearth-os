//! Bluetooth & devices › Mouse & touchpad and Mouse buttons (Mantel's SetMouse and SetMouseButtons). Mouse & touchpad:
//! pointer speed, acceleration, natural scrolling, left-handed buttons, scroll speed, middle-click, middle-click paste;
//! for a touchpad tap to click, scrolling with two fingers or the edge, tap-and-drag, how it clicks, and ignoring it
//! while you type; KWin's touchpad gestures. Saved by hearth-system pointer-set to ~/.config/nimbus/pointer.json, as
//! Mantel's page does; on KDE Plasma the helper is told KWin is the window manager, so the same file is put on KWin's
//! devices (hearth-kwin-settings pointer-apply, KWin's libinput settings in kcminputrc). Middle-click paste is KWin's
//! own (kwinrc [Wayland] EnablePrimarySelection). Mouse buttons: what the middle, back, forward and other extra buttons
//! do, KWin's button rebinds (kcminputrc [ButtonRebinds][Mouse]) through hearth-kwin-settings mouse-rebind; left and
//! right are never changed. Opening either page writes nothing.

use super::kw::{self, Kw};
use crate::cats::net::kit::{self, *};
use crate::shared::Shared;
use crate::widgets::*;
use hearth_ui::ui::{self, glyph, t, tm, G};
use iced::keyboard::{key::Named, Event as KEvent, Key};
use iced::widget::canvas::{self, Action, Frame, Geometry};
use iced::widget::{column, container, Column};
use iced::{mouse, Length, Rectangle, Renderer, Subscription, Task, Theme};
use serde_json::{json, Map, Value};

const HS: &str = "hearth-system";

#[derive(Default)]
pub struct S {
    pub st: Option<Value>,
    pub mouse: Map<String, Value>,
    pub pad: Map<String, Value>,
    pub saving: bool,
    pub say: Say,
    pub kw: Kw,
    // buttons
    pub added: Vec<String>,
    pub pressing: bool,
    pub key_for: String,
    pub open: String,
    pub note: Say,
}

#[derive(Debug, Clone)]
pub enum M {
    Pointer(Res),
    KwStatus(Res),
    KwGot(Res),
    Set(bool, &'static str, Value),
    Speed(bool, &'static str, f32),
    Release,
    Saved(Res),
    Paste(bool),
    KwSaved(Res),
    Go(&'static str),
    Press,
    Caught(mouse::Button),
    Open(String),
    Action(String, String),
    Rebound(String, String, Res),
    Key(KEvent),
}

pub fn open(_sh: &mut Shared, s: &mut S, page: &'static str, part: &str) -> Task<M> {
    s.say.clear();
    s.note.clear();
    s.kw.say.clear();
    s.open.clear();
    s.key_for.clear();
    s.pressing = page == "mouse-buttons" && part == "press";
    let mut t = vec![Task::perform(kw::status(), M::KwStatus)];
    if page == "mouse" {
        t.push(Task::perform(kit::job(HS, kit::a(&["pointer"]), String::new()), M::Pointer));
    }
    Task::batch(t)
}

fn save(s: &mut S) -> Task<M> {
    s.saving = true;
    let req = json!({ "mouse": Value::Object(s.mouse.clone()), "touchpad": Value::Object(s.pad.clone()) });
    Task::perform(kit::job(HS, kit::a(&["pointer-set"]), req.to_string()), M::Saved)
}

/// Qt's other names for the same buttons (hearth-kwin-settings MOUSE_ALIASES).
fn norm(k: &str) -> String {
    match k {
        "BackButton" | "XButton1" => "ExtraButton1".into(),
        "ForwardButton" | "XButton2" => "ExtraButton2".into(),
        "TaskButton" => "ExtraButton3".into(),
        "MidButton" => "MiddleButton".into(),
        o => o.into(),
    }
}
fn valid(b: &str) -> bool {
    b == "MiddleButton" || b.strip_prefix("ExtraButton").and_then(|n| n.parse::<u32>().ok()).map(|n| (1..=24).contains(&n)).unwrap_or(false)
}
fn order(b: &str) -> u32 {
    if b == "MiddleButton" {
        0
    } else {
        b.strip_prefix("ExtraButton").and_then(|n| n.parse().ok()).unwrap_or(99)
    }
}
pub fn label(b: &str) -> String {
    match b {
        "MiddleButton" => "Middle button (the wheel)".into(),
        "ExtraButton1" => "Back button (on the side, nearer you)".into(),
        "ExtraButton2" => "Forward button (on the side, further forward)".into(),
        o => format!("Extra button {}", o.trim_start_matches("ExtraButton")),
    }
}
/// A pressed button as KWin's name for it; "" for left and right. Back and forward are KWin's ExtraButton1 and 2;
/// other buttons come as their Linux code (BTN_TASK 0x117 is ExtraButton3, 0x118 on ExtraButton4…) or, on X11, as
/// button 8 on.
pub fn name_of(b: mouse::Button) -> String {
    match b {
        mouse::Button::Middle => "MiddleButton".into(),
        mouse::Button::Back => "ExtraButton1".into(),
        mouse::Button::Forward => "ExtraButton2".into(),
        mouse::Button::Other(code) => {
            let n: u32 = match code {
                0x113 | 0x116 => 1,
                0x114 | 0x115 => 2,
                0x117 => 3,
                c if c >= 0x118 => c as u32 - 0x118 + 4,
                c if c >= 8 => c as u32 - 7,
                _ => 0,
            };
            if (1..=24).contains(&n) {
                format!("ExtraButton{n}")
            } else {
                String::new()
            }
        }
        _ => String::new(),
    }
}
/// MouseButton,N is the Linux button code (linux/input-event-codes.h), as KWin sends it.
pub const ACTIONS: &[(&str, &str)] = &[
    ("", "As it is"), ("MouseButton,275", "Back"), ("MouseButton,276", "Forward"), ("MouseButton,274", "Middle-click"), ("MouseButton,273", "Right-click"),
    ("Key,Ctrl+C", "Copy (Ctrl+C)"), ("Key,Ctrl+V", "Paste (Ctrl+V)"), ("Key,Ctrl+Z", "Undo (Ctrl+Z)"), ("Key,Ctrl+Shift+Z", "Redo (Ctrl+Shift+Z)"),
    ("Key,Ctrl+Tab", "Next tab"), ("Key,Ctrl+Shift+Tab", "Previous tab"), ("Key,Ctrl+W", "Close the tab (Ctrl+W)"), ("Key,Meta+W", "Overview of windows"),
    ("Key,Volume Up", "Volume up"), ("Key,Volume Down", "Volume down"), ("Key,Media Play", "Play / pause"), ("Key,Print", "Screenshot"),
    ("Disabled", "Nothing (the button does nothing)"),
];
pub fn describe(v: &str) -> String {
    if let Some(a) = ACTIONS.iter().find(|a| a.0 == v) {
        return a.1.to_string();
    }
    if let Some(k) = v.strip_prefix("Key,") {
        return format!("The key {k}");
    }
    if let Some(k) = v.strip_prefix("MouseButton,") {
        return format!("Mouse button {k}");
    }
    v.to_string()
}

/// A key or key combination pressed (Ctrl+Shift+Z), as KWin writes it; "" for a modifier on its own.
pub fn key_text(key: &Key, mods: iced::keyboard::Modifiers) -> String {
    let k: String = match key {
        Key::Character(c) => {
            let c = c.as_str();
            if c.chars().count() == 1 && c.chars().all(|x| x.is_ascii_alphanumeric()) {
                c.to_uppercase()
            } else if c == "-" || c == "." {
                c.to_string()
            } else {
                String::new()
            }
        }
        Key::Named(n) => match n {
            Named::ArrowLeft => "Left".into(),
            Named::ArrowRight => "Right".into(),
            Named::ArrowUp => "Up".into(),
            Named::ArrowDown => "Down".into(),
            Named::Tab => "Tab".into(),
            Named::Enter => "Return".into(),
            Named::Space => "Space".into(),
            Named::Backspace => "Backspace".into(),
            Named::Delete => "Del".into(),
            Named::Insert => "Ins".into(),
            Named::Home => "Home".into(),
            Named::End => "End".into(),
            Named::PageUp => "PgUp".into(),
            Named::PageDown => "PgDown".into(),
            Named::PrintScreen => "Print".into(),
            Named::AudioVolumeUp => "Volume Up".into(),
            Named::AudioVolumeDown => "Volume Down".into(),
            Named::AudioVolumeMute => "Volume Mute".into(),
            Named::MediaPlay | Named::MediaPlayPause => "Media Play".into(),
            Named::MediaTrackNext => "Media Next".into(),
            Named::MediaTrackPrevious => "Media Previous".into(),
            Named::F1 => "F1".into(),
            Named::F2 => "F2".into(),
            Named::F3 => "F3".into(),
            Named::F4 => "F4".into(),
            Named::F5 => "F5".into(),
            Named::F6 => "F6".into(),
            Named::F7 => "F7".into(),
            Named::F8 => "F8".into(),
            Named::F9 => "F9".into(),
            Named::F10 => "F10".into(),
            Named::F11 => "F11".into(),
            Named::F12 => "F12".into(),
            _ => String::new(),
        },
        _ => String::new(),
    };
    if k.is_empty() {
        return k;
    }
    let mut m: Vec<&str> = Vec::new();
    if mods.control() {
        m.push("Ctrl");
    }
    if mods.alt() {
        m.push("Alt");
    }
    if mods.shift() {
        m.push("Shift");
    }
    if mods.logo() {
        m.push("Meta");
    }
    m.push(&k);
    m.join("+")
}

impl S {
    fn saved(&self) -> Vec<(String, String)> {
        self.kw.group("kcminputrc", "ButtonRebinds][Mouse").map(|g| g.iter().map(|(k, v)| (norm(k), v.as_str().unwrap_or("").to_string())).collect()).unwrap_or_default()
    }
    fn action_of(&self, b: &str) -> String {
        self.saved().into_iter().find(|x| x.0 == b).map(|x| x.1).unwrap_or_default()
    }
    /// Back and forward always; any other button once it's changed or pressed in the box.
    fn buttons(&self) -> Vec<String> {
        let mut v = vec!["ExtraButton1".to_string(), "ExtraButton2".to_string()];
        for b in self.saved().into_iter().map(|x| x.0).chain(self.added.iter().cloned()) {
            if valid(&b) && !v.contains(&b) {
                v.push(b);
            }
        }
        v.sort_by_key(|b| order(b));
        v
    }
    fn mn(&self, k: &str) -> f64 {
        self.mouse.get(k).and_then(|v| v.as_f64()).unwrap_or(0.0)
    }
    fn pn(&self, k: &str) -> f64 {
        self.pad.get(k).and_then(|v| v.as_f64()).unwrap_or(0.0)
    }
    fn mb(&self, k: &str) -> Option<bool> {
        self.mouse.get(k).and_then(|v| v.as_bool())
    }
    fn pb(&self, k: &str) -> Option<bool> {
        self.pad.get(k).and_then(|v| v.as_bool())
    }
    fn ms(&self, k: &str) -> String {
        self.mouse.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string()
    }
    fn ps(&self, k: &str) -> String {
        self.pad.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string()
    }
}

pub fn update(sh: &mut Shared, s: &mut S, m: M) -> Task<M> {
    match m {
        M::Pointer(r) => match r {
            Res::Ok(v) => {
                s.mouse = kit::at(&v, "mouse").as_object().cloned().unwrap_or_default();
                s.pad = kit::at(&v, "touchpad").as_object().cloned().unwrap_or_default();
                s.st = Some(v);
            }
            Res::Err(e) => s.say.bad(e),
        },
        M::KwStatus(r) => match r {
            Res::Ok(v) => {
                let live = kit::bo(&v, "live");
                s.kw.st = Some(v);
                if live {
                    return Task::perform(kw::get(), M::KwGot);
                }
            }
            Res::Err(e) => {
                s.kw.st = Some(json!({ "live": false, "error": e }));
                s.kw.say.bad(e);
            }
        },
        M::KwGot(r) => s.kw.take(&r),
        M::Set(pad, k, v) => {
            if pad { s.pad.insert(k.into(), v) } else { s.mouse.insert(k.into(), v) };
            // Left-handed swaps the buttons of both, as Mantel's page does.
            if !pad && k == "leftHanded" {
                let v = s.mouse.get(k).cloned().unwrap_or(Value::Bool(false));
                s.pad.insert(k.into(), v);
            }
            return save(s);
        }
        M::Speed(pad, k, v) => {
            let v = json!((v * 100.0).round() / 100.0);
            if pad { s.pad.insert(k.into(), v) } else { s.mouse.insert(k.into(), v) };
        }
        M::Release => return save(s),
        M::Saved(r) => {
            s.saving = false;
            match r {
                Res::Ok(v) => {
                    if kit::bo(&v, "reloaded") {
                        s.say.clear();
                    } else {
                        s.say.ok("Saved. It's used from the next time you sign in to Desktop Mode.");
                    }
                }
                Res::Err(e) => s.say.bad(e),
            }
        }
        M::Paste(on) => {
            s.kw.put("kwinrc", "Wayland", "EnablePrimarySelection", Some(if on { "true" } else { "false" }));
            s.say.ok(format!("Saved. Middle-click paste is {} from the next time you sign in to Desktop Mode.", if on { "on" } else { "off" }));
            return Task::perform(kw::set("kwinrc", json!({ "Wayland": { "EnablePrimarySelection": on } })), M::KwSaved);
        }
        M::KwSaved(r) => {
            if let Res::Err(e) = &r {
                s.say.bad(e.clone());
            }
            s.kw.take(&r);
        }
        M::Go(id) => sh.go(id),
        M::Press => {
            s.pressing = !s.pressing;
            s.key_for.clear();
            s.note.clear();
        }
        M::Caught(b) => {
            if !s.pressing {
                return Task::none();
            }
            let n = name_of(b);
            if n.is_empty() {
                s.note.bad(format!("That's the {} button: it always stays as it is. Press another one.", if b == mouse::Button::Right { "right" } else { "left" }));
                return Task::none();
            }
            s.pressing = false;
            if !s.added.contains(&n) && !s.buttons().contains(&n) {
                s.added.push(n.clone());
            }
            s.open = n.clone();
            s.note.ok(format!("{} found: pick what it does below.", label(&n)));
        }
        M::Open(b) => s.open = if s.open == b { String::new() } else { b },
        M::Action(b, v) => {
            s.open.clear();
            if v == "@key" {
                s.key_for = b;
                s.pressing = false;
                return Task::none();
            }
            s.note.clear();
            let input = if v.is_empty() { " ".to_string() } else { v.clone() };
            return Task::perform(kit::job(kw::H, vec!["mouse-rebind".into(), b.clone()], input), move |r| M::Rebound(b.clone(), v.clone(), r));
        }
        M::Rebound(b, v, r) => match r {
            Res::Ok(_) => {
                s.added.retain(|x| x != &b);
                s.note.ok(format!("{}: {}.", label(&b), if v.is_empty() { "as it is".to_string() } else { describe(&v) }));
                return Task::perform(kw::get(), M::KwGot);
            }
            Res::Err(e) => s.note.bad(e),
        },
        M::Key(e) => {
            if s.key_for.is_empty() {
                return Task::none();
            }
            if let KEvent::KeyPressed { key, modifiers, .. } = e {
                if key == Key::Named(Named::Escape) {
                    s.key_for.clear();
                    s.note.clear();
                    return Task::none();
                }
                let k = key_text(&key, modifiers);
                if k.is_empty() {
                    return Task::none();
                }
                let b = std::mem::take(&mut s.key_for);
                return update(sh, s, M::Action(b, format!("Key,{k}")));
            }
        }
    }
    Task::none()
}

/// The box you press a mouse button in: it hears every button, the extra ones too.
pub struct Catch;

impl canvas::Program<M> for Catch {
    type State = ();
    fn update(&self, _s: &mut (), event: &canvas::Event, bounds: Rectangle, cursor: mouse::Cursor) -> Option<Action<M>> {
        if let canvas::Event::Mouse(mouse::Event::ButtonPressed(b)) = event {
            if cursor.is_over(bounds) {
                return Some(Action::publish(M::Caught(*b)).and_capture());
            }
        }
        None
    }
    fn draw(&self, _s: &(), renderer: &Renderer, _t: &Theme, bounds: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        vec![Frame::new(renderer, bounds.size()).into_geometry()]
    }
    fn mouse_interaction(&self, _s: &(), bounds: Rectangle, cursor: mouse::Cursor) -> mouse::Interaction {
        if cursor.is_over(bounds) {
            mouse::Interaction::Crosshair
        } else {
            mouse::Interaction::default()
        }
    }
}

pub fn view<'a>(sh: &'a Shared, s: &'a S, page: &'static str) -> E<'a, M> {
    if page == "mouse-buttons" {
        return buttons(sh, s);
    }
    let p = sh.p;
    let live = s.kw.live();
    let note = if live { "" } else { ". Needs KWin (KDE Plasma)" };
    let mut c = crate::secs::Plain::new(0).spacing(10.0);
    if s.st.is_none() {
        c = c.push(words(&p, "Reading your mouse and touchpad…"));
    }
    c = c.push(head(&p, if s.saving { "Mouse · saving…" } else { "Mouse" }));
    let sp = s.mn("speed");
    c = c.push(srow(&p, G::Grid, "Pointer speed", format!("{}%", (((sp + 1.0) / 2.0) * 100.0).round()), slider_f32(sp as f32, -1.0..=1.0, 0.01, |v| M::Speed(false, "speed", v), M::Release)));
    let accel = s.ms("accel") != "flat";
    c = c.push(srow(&p, G::Grid, "Mouse acceleration", "The pointer goes further when you move the mouse quickly. Off is steadier for games", switch(&p, accel, Some(M::Set(false, "accel", json!(if accel { "flat" } else { "adaptive" }))))));
    let nat = s.mb("natural").unwrap_or(false);
    c = c.push(srow(&p, G::Grid, "Natural scrolling", "The page moves the way the wheel turns, like on a phone", switch(&p, nat, Some(M::Set(false, "natural", Value::Bool(!nat))))));
    let lh = s.mb("leftHanded").unwrap_or(false);
    c = c.push(srow(&p, G::Grid, "Left-handed", "Swaps the left and right buttons", switch(&p, lh, Some(M::Set(false, "leftHanded", Value::Bool(!lh))))));
    let ss = s.mouse.get("scrollSpeed").and_then(|v| v.as_f64()).filter(|v| *v > 0.0).unwrap_or(1.0);
    c = c.push(slider_row(&p, live, "Scroll speed", format!("How far each turn of the wheel scrolls{note}"), ss, false));
    let mid = s.mb("middle").unwrap_or(false);
    c = c.push(switch_row(&p, G::Grid, "Middle-click with both buttons", format!("Pressing left and right together is a middle click{note}"), mid, live, M::Set(false, "middle", Value::Bool(!mid))));
    let paste = s.kw.b("kwinrc", "Wayland", "EnablePrimarySelection", true);
    c = c.push(switch_row(&p, G::Grid, "Paste with the middle button", format!("Select text, then click the middle button (the wheel) to paste it. Used from the next time you sign in{note}"), paste, live, M::Paste(!paste)));
    c = c.push(go_row(&p, G::Grid, "Mouse buttons", "What your mouse's side and extra buttons do: press one to change it", M::Go("mouse-buttons")));
    if s.st.is_some() {
        c = c.push(head(&p, "Touchpad"));
        if !s.st.as_ref().map(|v| kit::bo(v, "hasTouchpad")).unwrap_or(false) {
            c = c.push(words(&p, "No touchpad is connected. These are used when one is."));
        }
        let ps = s.pn("speed");
        c = c.push(srow(&p, G::Grid, "Touchpad speed", format!("{}%", (((ps + 1.0) / 2.0) * 100.0).round()), slider_f32(ps as f32, -1.0..=1.0, 0.01, |v| M::Speed(true, "speed", v), M::Release)));
        let tap = s.pb("tap") != Some(false);
        c = c.push(srow(&p, G::Grid, "Tap to click", "Tap with one finger to click, two fingers to right-click", switch(&p, tap, Some(M::Set(true, "tap", Value::Bool(!tap))))));
        let pnat = s.pb("natural").unwrap_or(false);
        c = c.push(srow(&p, G::Grid, "Natural scrolling", "Content moves with your fingers", switch(&p, pnat, Some(M::Set(true, "natural", Value::Bool(!pnat))))));
        let scroll = { let x = s.ps("scroll"); if x.is_empty() { "twofinger".to_string() } else { x } };
        c = c.push(choice(&p, G::Grid, "Scroll with", "", [("twofinger", "Two fingers"), ("edge", "The right edge")].iter().map(|&(v, l)| (l.to_string(), scroll == v, Some(M::Set(true, "scroll", json!(v))))).collect()));
        let pss = s.pad.get("scrollSpeed").and_then(|v| v.as_f64()).filter(|v| *v > 0.0).unwrap_or(1.0);
        c = c.push(slider_row(&p, live, "Touchpad scroll speed", format!("How far two fingers scroll{note}"), pss, true));
        let pacc = s.ps("accel") != "flat";
        c = c.push(switch_row(&p, G::Grid, "Touchpad acceleration", format!("The pointer goes further when you move quickly{note}"), pacc, live, M::Set(true, "accel", json!(if pacc { "flat" } else { "adaptive" }))));
        let td = s.pb("tapDrag") != Some(false);
        c = c.push(switch_row(&p, G::Grid, "Tap and drag", format!("Tap, then keep the finger down to drag{note}"), td, live, M::Set(true, "tapDrag", Value::Bool(!td))));
        let click = { let x = s.ps("click"); if x.is_empty() { "areas".to_string() } else { x } };
        let items: Vec<(String, bool)> = [("areas", "Corners"), ("fingers", "Fingers")].iter().map(|&(v, l)| (l.to_string(), click == v)).collect();
        c = c.push(if live {
            choice(&p, G::Grid, "Pressing the touchpad down", format!("Right-click with the bottom-right corner, or with two fingers anywhere{note}"), [("areas", "Corners"), ("fingers", "Fingers")].iter().map(|&(v, l)| (l.to_string(), click == v, Some(M::Set(true, "click", json!(v))))).collect())
        } else {
            choice_dim(&p, G::Grid, "Pressing the touchpad down", format!("Right-click with the bottom-right corner, or with two fingers anywhere{note}"), items)
        });
        let dwt = s.pb("dwt") != Some(false);
        c = c.push(srow(&p, G::Grid, "Ignore the touchpad while typing", "", switch(&p, dwt, Some(M::Set(true, "dwt", Value::Bool(!dwt))))));
        c = c.push(said(&p, &s.say));
        // KWin's touchpad gestures (built into KWin), so shown, not changed; only on KWin (the owner, Nov 28).
        if live {
        c = c.push(head(&p, "Touchpad gestures"));
        let q = if live { p } else { dim(&p) };
        c = c.push(info_row(&q, G::Grid, "Four fingers up: overview · down: desktop grid", if live { "Turning the overview off in Window effects turns these off" } else { "Needs KWin (KDE Plasma)" }));
        c = c.push(info_row(&q, G::Display, "Three or four fingers sideways: next or previous desktop", if live { "Three fingers up or down move between rows of desktops" } else { "Needs KWin (KDE Plasma)" }));
        }
        // a KWin page: off KWin it could never work (the owner, Nov 28)
        if crate::shared::kwin_desktop() {
        c = c.push(go_row(&p, G::Grid, "Drawing tablet & touchscreen", "Pen tablet's screen and area, pressure, pen buttons; touchscreen screen and calibration", M::Go("kwin-tablet")));
        }
    } else {
        c = c.push(said(&p, &s.say));
    }
    c.into()
}

/// A scroll speed (0.2× to 3×), greyed when KWin isn't here.
fn slider_row<'a>(p: &hearth_ui::Pal, live: bool, title: &'a str, sub: String, v: f64, pad: bool) -> E<'a, M> {
    let shown = format!("{sub} · {:.1}×", v);
    if live {
        srow(p, G::Grid, title, shown, slider_f32(v as f32, 0.2..=3.0, 0.1, move |x| M::Speed(pad, "scrollSpeed", x), M::Release))
    } else {
        let q = dim(p);
        srow(&q, G::Grid, title, shown, container(level_bar(&q, ((v - 0.2) / 2.8) as f32, q.accent)).width(Length::Fixed(sz(260.0))).into())
    }
}

fn buttons<'a>(sh: &'a Shared, s: &'a S) -> E<'a, M> {
    let p = sh.p;
    let mut c = crate::secs::Plain::new(0).spacing(10.0);
    c = c.push(kw::gate(&p, &s.kw, "Mouse button settings", M::Go("your-desktop")));
    let on = s.kw.live();
    let q = if on { p } else { dim(&p) };
    c = c.push(head(&q, "Your mouse's buttons"));
    c = c.push(words(&q, "Give the side buttons and any extra buttons of your mouse another job: a different click, a key or a shortcut. The left and right buttons always stay as they are. It works at once, in every app."));
    for b in s.buttons() {
        let cur = s.action_of(&b);
        let mut opts: Vec<(String, bool, M)> = ACTIONS.iter().map(|&(v, l)| (l.to_string(), cur == v, M::Action(b.clone(), v.to_string()))).collect();
        if !cur.is_empty() && !ACTIONS.iter().any(|a| a.0 == cur) {
            opts.push((describe(&cur), true, M::Action(b.clone(), cur.clone())));
        }
        opts.push(("A key you press…".into(), false, M::Action(b.clone(), "@key".into())));
        let sub = if cur.is_empty() { "Does what the mouse's maker set".to_string() } else { format!("Now: {}", describe(&cur)) };
        let shown = if cur.is_empty() { "As it is".to_string() } else { describe(&cur) };
        if on {
            c = c.push(dropdown(&p, G::Grid, label(&b), sub, shown, s.open == b, M::Open(b.clone()), opts));
        } else {
            c = c.push(srow(&q, G::Grid, label(&b), sub, t(shown, sz(13.0), q.muted).into()));
        }
    }
    c = c.push(press(&q, G::Plus, if s.pressing { "Cancel" } else { "Change another button" }, if s.pressing { "Close the box without adding a button" } else { "Press it in the box that opens, so Hearth knows which one it is" }, if on { Some(M::Press) } else { None }));
    // ---- "press the button" box
    if s.pressing {
        let inner = column![
            glyph(G::Grid, p.accent, 44.0, 2.0),
            tm("Point here and press the mouse button you want to change", sz(17.0), p.text),
            t("The middle button (press the wheel), back, forward or any other extra button", sz(13.0), p.muted),
        ]
        .spacing(8.0)
        .align_x(iced::Center);
        let boxed = container(inner).center_x(Length::Fill).center_y(Length::Fixed(180.0)).style(ui::boxed(p.panel, p.r(14.0), Some((p.accent, 2.0))));
        c = c.push(iced::widget::stack![boxed, canvas::Canvas::new(Catch).width(Length::Fill).height(Length::Fixed(180.0))]);
    }
    // ---- the key box: the next key or key combination pressed is what the button sends
    if !s.key_for.is_empty() {
        let what = label(&s.key_for);
        let short = what.split(" (").next().unwrap_or(&what).to_lowercase();
        let inner = column![
            glyph(G::Grid, p.accent, 40.0, 2.0),
            tm(format!("Press the key or key combination for {short}"), sz(17.0), p.text),
            t("Hold Ctrl, Alt, Shift or Super with it for a shortcut · Esc cancels", sz(13.0), p.muted),
        ]
        .spacing(8.0)
        .align_x(iced::Center);
        c = c.push(container(inner).center_x(Length::Fill).center_y(Length::Fixed(150.0)).style(ui::boxed(p.panel, p.r(14.0), Some((p.accent, 2.0)))));
    }
    c = c.push(said(&p, &s.note));
    c = c.push(said(&p, &s.kw.say));
    c = c.push(words(&p, "A button already changed here is caught as what it now does: set it back to \"As it is\" first to change it again. Pen buttons are on Drawing tablet & touchscreen."));
    c.into()
}

pub fn subscription(s: &S) -> Subscription<M> {
    if !s.key_for.is_empty() {
        iced::keyboard::listen().map(M::Key)
    } else {
        Subscription::none()
    }
}

pub fn back(s: &mut S) -> bool {
    if s.pressing || !s.key_for.is_empty() || !s.open.is_empty() {
        s.pressing = false;
        s.key_for.clear();
        s.open.clear();
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn names_buttons_and_keys() {
        assert_eq!(name_of(mouse::Button::Back), "ExtraButton1");
        assert_eq!(name_of(mouse::Button::Other(0x117)), "ExtraButton3");
        assert_eq!(name_of(mouse::Button::Left), "");
        assert_eq!(norm("BackButton"), "ExtraButton1");
        assert!(valid("ExtraButton24") && !valid("ExtraButton25"));
        assert_eq!(describe("Key,Ctrl+C"), "Copy (Ctrl+C)");
        assert_eq!(describe("Key,F7"), "The key F7");
        let m = iced::keyboard::Modifiers::CTRL | iced::keyboard::Modifiers::SHIFT;
        assert_eq!(key_text(&Key::Character("z".into()), m), "Ctrl+Shift+Z");
        assert_eq!(key_text(&Key::Named(Named::Shift), m), "");
    }
}
