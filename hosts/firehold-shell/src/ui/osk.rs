//! `firehold-shell osk`: Firehold's own on-screen keyboard (it's its own desktop, so it has its own), Hearth's
//! keyboard (ui/osk/hearth-osk.js, docs/OSK.md) drawn the Firehold way: the number row and ⌫, the letters with ↵ and
//! ⇧, &123 for symbols, space, the caret keys and ⌨ to hide it. Docked at the bottom of the screen; windows make room.
//! It never takes the keyboard, so the window you're typing into keeps it: each key goes to that window through
//! firehold-comp (`key`, as if typed on a US keyboard).
//!
//! The controller (docs/OSK.md): D-pad to move, A types, X ⌫, Y space, LB / RB move the caret, LT shift, RT symbols,
//! ☰ Enter, B hides it. The mouse and touch tap the keys.

use iced::widget::{column, container, mouse_area, row};
use iced::{Color, Element, Length, Padding, Subscription, Task};
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, Settings};
use iced_layershell::to_layer_message;
use serde_json::json;

use super::style::*;
use crate::link::{self, HubMsg};
use crate::proto::State;

#[to_layer_message]
#[derive(Debug, Clone)]
pub enum Message {
    Hub(HubMsg),
    Tap(usize, usize),
    Hover(usize, usize),
}

#[derive(Default)]
struct K {
    st: State,
    shift: bool,
    caps: bool,
    sym: bool,
    r: usize,
    c: usize,
}

pub const H: u32 = 330;

/// A key: what it types or does, its label, how wide (in keys).
#[derive(Clone)]
struct Key {
    act: String,
    label: String,
    w: f32,
}
fn k(act: &str, label: &str, w: f32) -> Key {
    Key { act: act.into(), label: label.into(), w }
}
fn chars(s: &str, upper: bool) -> Vec<Key> {
    s.chars()
        .map(|ch| {
            let c = if upper { ch.to_ascii_uppercase() } else { ch };
            k(&format!("ch:{c}"), &c.to_string(), 1.0)
        })
        .collect()
}

/// The rows (hearth-osk.js letterRows / symRows, English QWERTY; the symbols of its first page).
fn rows(s: &K) -> Vec<Vec<Key>> {
    let up = s.shift || s.caps;
    let mut v = Vec::new();
    let mut r0 = chars("1234567890", false);
    r0.push(k("bksp", "⌫", 1.5));
    v.push(r0);
    if s.sym {
        v.push(chars("!@#$%^&*()", false));
        let mut r2 = chars("-_=+[]{};", false);
        r2.push(k("enter", "↵", 1.6));
        v.push(r2);
        v.push(chars("'\"/\\|~`:?", false));
        v.push(vec![k("abc", "ABC", 1.5), k("ch: ", "", 5.4), k("left", "‹", 1.0), k("right", "›", 1.0), k("hide", "⌨", 1.3)]);
    } else {
        v.push(chars("qwertyuiop", up));
        let mut r2 = chars("asdfghjkl", up);
        r2.push(k("enter", "↵", 1.6));
        v.push(r2);
        let mut r3 = vec![k("shift", "⇧", 1.4)];
        r3.extend(chars("zxcvbnm,.", up));
        r3.push(k("shift", "⇧", 1.2));
        v.push(r3);
        v.push(vec![k("sym", "&123", 1.5), k("ch: ", "English (QWERTY)", 5.4), k("left", "‹", 1.0), k("right", "›", 1.0), k("hide", "⌨", 1.3)]);
    }
    v
}

fn act(s: &mut K, a: &str) {
    if let Some(ch) = a.strip_prefix("ch:") {
        link::send(json!({"op": "oskType", "text": ch}));
        if s.shift && !s.caps {
            s.shift = false;
        }
        return;
    }
    match a {
        "bksp" => link::send(json!({"op": "oskType", "keys": ["backspace"]})),
        "enter" => link::send(json!({"op": "oskType", "keys": ["enter"]})),
        "left" | "right" => link::send(json!({"op": "oskType", "keys": [a]})),
        "shift" => {
            // Once: one capital; again while it's on: caps lock; again: off.
            if s.caps {
                s.caps = false;
                s.shift = false;
            } else if s.shift {
                s.caps = true;
            } else {
                s.shift = true;
            }
        }
        "sym" => s.sym = true,
        "abc" => s.sym = false,
        "hide" => link::send(json!({"op": "osk", "arg": "off"})),
        _ => {}
    }
}

fn fix(s: &mut K) {
    let rs = rows(s);
    s.r = s.r.min(rs.len() - 1);
    s.c = s.c.min(rs[s.r].len() - 1);
}

fn nav(s: &mut K, key: &str) {
    let rs = rows(s);
    match key {
        "left" if s.c > 0 => s.c -= 1,
        "right" => s.c += 1,
        "up" if s.r > 0 => {
            // Keep about the same place across rows of different lengths.
            let frac = (s.c as f32 + 0.5) / rs[s.r].len() as f32;
            s.r -= 1;
            s.c = (frac * rs[s.r].len() as f32) as usize;
        }
        "down" if s.r + 1 < rs.len() => {
            let frac = (s.c as f32 + 0.5) / rs[s.r].len() as f32;
            s.r += 1;
            s.c = (frac * rs[s.r].len() as f32) as usize;
        }
        "a" => {
            fix(s);
            let a = rows(s)[s.r][s.c].act.clone();
            act(s, &a);
        }
        "x" => act(s, "bksp"),
        "y" => act(s, "ch: "),
        "lb" => act(s, "left"),
        "rb" => act(s, "right"),
        "lt" => act(s, "shift"),
        "rt" => s.sym = !s.sym,
        "start" => act(s, "enter"),
        "b" => act(s, "hide"),
        _ => {}
    }
    fix(s);
}

fn update(s: &mut K, m: Message) -> Task<Message> {
    match m {
        Message::Hub(HubMsg::State(st)) => {
            s.st = *st;
            if !s.st.osk {
                return iced::exit();
            }
        }
        Message::Hub(HubMsg::Nav(key)) => nav(s, &key),
        Message::Hub(HubMsg::Quit) | Message::Hub(HubMsg::Gone) => return iced::exit(),
        Message::Hub(_) => {}
        Message::Tap(r, c) => {
            s.r = r;
            s.c = c;
            fix(s);
            if let Some(key) = rows(s).get(r).and_then(|row| row.get(c)).cloned() {
                act(s, &key.act);
            }
        }
        Message::Hover(r, c) => {
            s.r = r;
            s.c = c;
        }
        _ => {}
    }
    Task::none()
}

fn view(s: &K) -> Element<'_, Message> {
    let rs = rows(s);
    let mut col = column![].spacing(8.0);
    for (r, line) in rs.iter().enumerate() {
        let mut rr = row![].spacing(8.0);
        for (c, key) in line.iter().enumerate() {
            let on = s.r == r && s.c == c;
            let special = !key.act.starts_with("ch:") || key.act == "ch: ";
            let lit = (key.act == "shift" && (s.shift || s.caps)) || key.act == "enter";
            let (bg, fg) = if lit { (c_accent(), c_ink()) } else if special { (shade(0.55), c_text()) } else { (shade(0.85), c_text()) };
            let label = if key.act == "ch: " { t(key.label.clone(), 14.0, c_muted()) } else { tb(key.label.clone(), 20.0, fg) };
            let cell = container(label)
                .center_x(Length::FillPortion((key.w * 10.0) as u16))
                .center_y(Length::Fixed(52.0))
                .style(boxed(bg, 10.0, on.then_some((c_text(), 2.0))));
            rr = rr.push(mouse_area(cell).on_press(Message::Tap(r, c)).on_enter(Message::Hover(r, c)));
        }
        col = col.push(rr);
    }
    let hints = row![t("A type · X ⌫ · Y space · LB RB move · LT shift · RT symbols · ☰ enter · B hide", 13.0, c_muted())];
    container(column![col, hints].spacing(10.0))
        .padding(Padding { top: 16.0, right: 120.0, bottom: 12.0, left: 120.0 })
        .width(Length::Fill)
        .height(Length::Fill)
        .style(boxed(shade(0.30), 0.0, Some((c_line(), 1.0))))
        .into()
}

fn subscription(_s: &K) -> Subscription<Message> {
    Subscription::run(link::stream).map(Message::Hub)
}

pub fn run() -> i32 {
    link::set_surface("osk");
    iced_layershell::disable_clipboard();
    let layer_settings = LayerShellSettings {
        anchor: Anchor::Bottom | Anchor::Left | Anchor::Right,
        layer: Layer::Overlay,
        // Windows make room above it, so the field you're typing in stays in sight.
        exclusive_zone: H as i32,
        size: Some((0, H)),
        keyboard_interactivity: KeyboardInteractivity::None,
        start_mode: link::start_mode(),
        ..Default::default()
    };
    let r = iced_layershell::application(K::default, || "hearth-firehold-osk".to_string(), update, view)
        .style(|_s: &K, _t: &iced::Theme| iced::theme::Style { background_color: Color::TRANSPARENT, text_color: c_text() })
        .subscription(subscription)
        .settings(Settings { layer_settings, default_font: INTER, ..Default::default() })
        .run();
    super::ended("osk", r)
}
