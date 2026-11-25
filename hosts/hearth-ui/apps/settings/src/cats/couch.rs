//! System › Couch, Desk and Away (Firehold on Plasma, mockup docs/mockups/firehold-plasma/05-settings-modes.png) and
//! its While Away page. For each screen: the mode it starts in, what's on its Couch home, the sound in Couch; then
//! switching by itself (a controller puts the TV in Couch, the mouse puts its own screen in Desk, Away after a time,
//! what keeps running while Away, games following the mode).
//!
//! These are Firehold's settings (~/.config/nimbus/firehold.json, shared with Firehold on Plasma's Couch and Desk),
//! changed only here and only when you change one: hearth_settings_io merges the one key (atomic, .bak, the change
//! history). Opening the page only reads (`hearth-firehold-plasma get` and `modes`). Switching modes is live state
//! (hearth-firehold-plasma's $XDG_RUNTIME_DIR), never a settings change (red flag 9). Its own module, so it merges
//! apart from the other categories (cats/mod.rs has the shape).

use crate::shared::Shared;
use crate::widgets::*;
use crate::App;
use hearth_ui::run;
use hearth_ui::ui::{self, glyph, t, tb, G};
use iced::widget::{column, container, mouse_area, row, Column, Row, Space};
use iced::{mouse, Element, Length, Padding, Subscription, Task};
use serde_json::{json, Map, Value};

pub const PAGES: &[&str] = &["couch-desk-away", "couch-away"];

#[derive(Default)]
pub struct State {
    pub cfg: Option<Value>,
    pub modes: Option<Value>,
    pub sel: usize,
    pub msg: String,
}

#[derive(Debug, Clone)]
pub enum Msg {
    /// Arrange this page's parts (Settings' sections as blocks, widgets.rs Secs; main.rs gives it to Describe it)
    Arrange(hearth_ui::describe::DMsg),
    Cfg(run::Output),
    Modes(run::Output),
    Pick(usize),
    /// A key of the picked screen's own choices (start, sound).
    Screen(&'static str, String),
    Set(&'static str, Value),
    Identify,
    Display,
    Away,
    Did(run::Output),
}

impl From<hearth_ui::describe::DMsg> for Msg {
    fn from(d: hearth_ui::describe::DMsg) -> Msg {
        Msg::Arrange(d)
    }
}

fn path() -> std::path::PathBuf {
    std::env::var_os("XDG_CONFIG_HOME").map(std::path::PathBuf::from).unwrap_or_else(|| hearth_ui::paths::home().join(".config")).join("nimbus/firehold.json")
}

fn helper(args: &[&str]) -> Vec<String> {
    let mut v = vec!["timeout".to_string(), "10".into(), "hearth-firehold-plasma".into()];
    v.extend(args.iter().map(|s| s.to_string()));
    v
}

pub fn open(_sh: &mut Shared, st: &mut State, _page: &'static str) -> Task<Msg> {
    st.msg.clear();
    Task::batch([Task::perform(run::spawn(helper(&["get"])), Msg::Cfg), Task::perform(run::spawn(helper(&["modes"])), Msg::Modes)])
}

fn cfg<'a>(st: &'a State) -> &'a Value {
    static NULL: Value = Value::Null;
    st.cfg.as_ref().unwrap_or(&NULL)
}

fn screens(st: &State) -> Vec<Value> {
    st.modes.as_ref().and_then(|m| m.get("screens")).and_then(Value::as_array).cloned().unwrap_or_default()
}

fn s_of(v: &Value, k: &str) -> String {
    v.get(k).and_then(Value::as_str).unwrap_or("").to_string()
}

pub fn update(sh: &mut Shared, st: &mut State, m: Msg) -> Task<Msg> {
    match m {
        Msg::Arrange(_) => return Task::none(),
        Msg::Cfg(o) => st.cfg = Some(o.json().filter(Value::is_object).unwrap_or(json!({}))),
        Msg::Modes(o) => st.modes = o.json(),
        Msg::Pick(i) => st.sel = i,
        Msg::Screen(key, value) => {
            let list = screens(st);
            let Some(s) = list.get(st.sel) else { return Task::none() };
            let name = s_of(s, "name");
            // The whole "screens" map with this one screen's one choice changed (merged: its other keys are kept).
            let mut all = cfg(st).get("screens").and_then(Value::as_object).cloned().unwrap_or_default();
            let mut one = all.get(&name).and_then(Value::as_object).cloned().unwrap_or_else(Map::new);
            one.insert(key.to_string(), Value::String(value));
            all.insert(name, Value::Object(one));
            return set(sh, st, "screens", Value::Object(all));
        }
        Msg::Set(key, v) => return set(sh, st, key, v),
        Msg::Identify => run::detached(&["hearth-firehold-plasma", "do", "identify"]),
        Msg::Display => sh.go("display"),
        Msg::Away => sh.go("couch-away"),
        Msg::Did(o) => {
            sh.result("Couldn't save that", &o);
            return Task::perform(run::spawn(helper(&["get"])), Msg::Cfg);
        }
    }
    Task::none()
}

fn set(sh: &mut Shared, st: &mut State, key: &'static str, v: Value) -> Task<Msg> {
    if cfg(st).get(key) == Some(&v) {
        return Task::none(); // unchanged: nothing written
    }
    if let Some(Value::Object(c)) = st.cfg.as_mut() {
        c.insert(key.to_string(), v.clone());
    }
    let mut ch = Map::new();
    ch.insert(key.to_string(), v.clone());
    // Away after a time: the last time picked is kept too, so switching it on again (Modes) puts back yours.
    if key == "awayAfter" && v.as_i64().unwrap_or(0) > 0 {
        ch.insert("awayAfterOn".into(), v);
    }
    let cmd = sh.json_cmd(&path(), Value::Object(ch));
    Task::perform(run::spawn(cmd), Msg::Did)
}

fn choice<'a>(p: &hearth_ui::Pal, cur: &str, items: &[(&str, &str)], on: impl Fn(String) -> Msg) -> E<'a, Msg> {
    seg_flat(p, items.iter().map(|(v, l)| (l.to_string(), *v == cur, Some(on(v.to_string())))).collect())
}

fn mode_name(m: &str) -> &'static str {
    match m {
        "couch" => "Couch",
        "away" => "Away",
        _ => "Desk",
    }
}

pub fn view<'a>(a: &'a App, page: &'static str) -> Option<Element<'a, Msg>> {
    if !PAGES.contains(&page) {
        return None;
    }
    let p = a.sh.p;
    let st = &a.couch;
    let c = cfg(st);
    if page == "couch-away" {
        return Some(away_page(a));
    }
    let list = screens(st);
    let mut col = crate::widgets::Secs::on(a, 0).spacing(10.0);
    col = col.push(t("Couch is for the controller and the TV, Desk for the mouse and keyboard, Away for when you're not there. Each screen has its own.", sz(16.0), p.muted));
    if !a.sh.error.is_empty() {
        col = col.push(note(&p, a.sh.error.clone(), true));
    }
    // ---- the screens, as KDE has them, each with its mode now
    let sel = st.sel.min(list.len().saturating_sub(1));
    let mut strip = Row::new().spacing(20.0).align_y(iced::Center);
    for (i, s) in list.iter().enumerate() {
        let on = i == sel;
        let label = format!("{} · {}", i + 1, if s.get("tv").and_then(Value::as_bool).unwrap_or(false) { "TV" } else { "Monitor" });
        let inner = column![tb(label, sz(if on { 24.0 } else { 21.0 }), p.text), t(mode_name(&s_of(s, "mode")), sz(16.0), p.muted)].spacing(4.0).align_x(iced::Center);
        let (w, h) = if on { (300.0, 168.0) } else { (230.0, 130.0) };
        let bg = if on { hearth_ui::Pal::mix(p.panel, p.accent, 0.14) } else { p.panel };
        let card = container(inner)
            .center_x(Length::Fixed(sz(w)))
            .center_y(Length::Fixed(sz(h)))
            .style(ui::boxed(bg, p.r(10.0), Some((if on { p.accent } else { hearth_ui::Pal::mix(p.panel, p.text, 0.12) }, if on { 3.0 } else { 1.5 }))));
        strip = strip.push(mouse_area(card).on_press(Msg::Pick(i)).interaction(mouse::Interaction::Pointer));
    }
    if let Some(s) = list.get(sel) {
        let name = s_of(s, "label");
        let spec = s_of(s, "spec");
        strip = strip.push(
            column![
                tb(if name.is_empty() { s_of(s, "name") } else { name }, sz(22.0), p.text),
                t(spec, sz(16.0), p.muted),
                row![pill(&p, "Identify", true, Some(Msg::Identify)), pill(&p, "Display settings ›", false, Some(Msg::Display))].spacing(10.0)
            ]
            .spacing(8.0),
        );
    }
    if list.is_empty() {
        col = col.push(card(&p, t("Reading your screens…", sz(15.0), p.muted)));
    } else {
        col = col.push(card(&p, strip.wrap()));
    }
    // ---- this screen's own choices
    if let Some(s) = list.get(sel) {
        let name = s_of(s, "name");
        let label = s_of(s, "label");
        let mine = c.get("screens").and_then(|x| x.get(&name)).cloned().unwrap_or(Value::Null);
        let tv = s.get("tv").and_then(Value::as_bool).unwrap_or(false);
        let start = mine.get("start").and_then(Value::as_str).unwrap_or(if tv { "couch" } else { "desk" }).to_string();
        let sound = mine.get("sound").and_then(Value::as_str).unwrap_or(if tv { "tv" } else { "keep" }).to_string();
        col = col.push(head(&p, format!("This screen: {}", if label.is_empty() { name.clone() } else { label })));
        col = col.push(srow(&p, G::Power, "Starts in", if tv { "When the PC starts or the TV turns on" } else { "When the PC starts or this screen turns on" },
            choice(&p, &start, &[("couch", "Couch"), ("desk", "Desk"), ("away", "Away"), ("last", "Where I left it")], |v| Msg::Screen("start", v))));
        let home: Vec<String> = c.get("couchHome").and_then(Value::as_array).map(|l| l.iter().filter_map(|x| x.as_str().map(String::from)).collect())
            .unwrap_or_else(|| ["Hero", "Continue", "Recently added", "Watch and listen", "Downloads", "Controllers"].iter().map(|s| s.to_string()).collect());
        col = col.push(srow(&p, G::Home, "Couch home", "What's on Home and in which order",
            row![t(home.join(", "), sz(15.0), p.muted).width(Length::Fixed(sz(300.0))), glyph(G::Chevron, p.muted, 14.0, 2.0)].spacing(10.0).align_y(iced::Center).into()));
        col = col.push(srow(&p, G::Sound, "Sound in Couch", "Switches the output when this screen goes Couch",
            choice(&p, &sound, &[("tv", if tv { "TV speakers" } else { "This screen's speakers" }), ("keep", "Keep what I picked")], |v| Msg::Screen("sound", v))));
    }
    // ---- switching by itself
    col = col.push(head(&p, "Switching by itself"));
    let b = |k: &str, d: bool| c.get(k).and_then(Value::as_bool).unwrap_or(d);
    let scope = c.get("autoPadScope").and_then(Value::as_str).unwrap_or("tv").to_string();
    let auto_pad = b("autoPad", true);
    col = col.push(srow(&p, G::Pad, "Controller → Couch", "Pressing the Guide button or moving a stick puts the TV in Couch",
        row![choice(&p, &scope, &[("tv", "The TV"), ("every", "Every screen")], |v| Msg::Set("autoPadScope", Value::String(v))),
             switch(&p, auto_pad, Some(Msg::Set("autoPad", Value::Bool(!auto_pad))))].spacing(16.0).align_y(iced::Center).into()));
    let auto_mouse = b("autoMouse", true);
    col = col.push(srow(&p, G::Mouse, "Mouse or keyboard → Desk", "Only the screen the mouse is on goes to Desk",
        switch(&p, auto_mouse, Some(Msg::Set("autoMouse", Value::Bool(!auto_mouse))))));
    let after = c.get("awayAfter").and_then(Value::as_i64).unwrap_or(20);
    col = col.push(srow(&p, G::Moon, "Away when nobody's here", "No controller, mouse or keyboard for this long, and nothing playing",
        seg_flat(&p, [(10, "10 min"), (20, "20 min"), (60, "1 h"), (0, "Never")].iter().map(|(v, l)| (l.to_string(), *v == after, Some(Msg::Set("awayAfter", json!(v))))).collect())));
    let off = c.get("awayScreenOff").and_then(Value::as_i64).unwrap_or(10);
    let lock = c.get("awayLock").and_then(Value::as_i64).unwrap_or(10);
    col = col.push(link_row(&p, G::Clock, "While Away", format!("{} · downloads, Hearth Link, backups and the home server keep going{}",
        if off > 0 { format!("Screens off after {off} min") } else { "Screens stay on".to_string() },
        if lock > 0 { " · locks with your PIN" } else { "" }), Msg::Away));
    let games = c.get("games").and_then(Value::as_str).unwrap_or("follow");
    let games = if games == "stay" || games == "window" { "stay" } else { "follow" };
    col = col.push(srow(&p, G::Display, "Games when you switch", "Full screen on Couch, back in their window on Desk",
        choice(&p, games, &[("follow", "Follow the mode"), ("stay", "Stay as they are")], |v| Msg::Set("games", Value::String(v)))));
    col = col.push(container(t("These choices only change when you change them. Switching modes is never a settings change.", sz(14.0), p.muted)).padding(Padding { top: 4.0, right: 0.0, bottom: 0.0, left: 2.0 }));
    Some(col.into())
}

fn away_page(a: &App) -> Element<'_, Msg> {
    let p = a.sh.p;
    let c = cfg(&a.couch);
    let off = c.get("awayScreenOff").and_then(Value::as_i64).unwrap_or(10);
    let lock = c.get("awayLock").and_then(Value::as_i64).unwrap_or(10);
    let mut col = crate::widgets::Secs::on(a, 0).spacing(10.0);
    col = col.push(t("Away shows a big clock, what keeps working and how many notifications wait (not what they say).", sz(16.0), p.muted));
    col = col.push(srow(&p, G::Display, "Screens off", "After this long Away", steps(&p, off, "awayScreenOff")));
    col = col.push(srow(&p, G::Shield, "Lock with your PIN", "After this long Away, coming back asks for it", steps(&p, lock, "awayLock")));
    col = col.push(head(&p, "Keeps going while Away"));
    for (g, n, s) in [(G::Down, "Downloads and updates", "Steam's, Heroic's and Hearth OS's"), (G::Link, "Hearth Link", "Your other devices' saves and copies"),
                      (G::Drive, "Backups", "Hearth Storage and Saved Games"), (G::Server, "The home server", "Media, AI chat and its apps")] {
        col = col.push(srow(&p, g, n, s, Space::new().width(Length::Shrink).into()));
    }
    col = col.push(note(&p, "Away also turns on Power saver until you're back (the profile you had comes back with you).", false));
    col.into()
}

fn steps<'a>(p: &hearth_ui::Pal, cur: i64, key: &'static str) -> E<'a, Msg> {
    seg_flat(p, [(5i64, "5 min"), (10, "10 min"), (30, "30 min"), (0, "Never")].iter().map(|(v, l)| (l.to_string(), *v == cur, Some(Msg::Set(key, json!(v))))).collect())
}

pub fn subscription(_st: &State) -> Subscription<Msg> {
    Subscription::none()
}

pub fn dialog<'a>(_a: &'a App) -> Option<(Element<'a, Msg>, Option<Msg>)> {
    None
}

pub fn back(_st: &mut State) -> bool {
    false
}
