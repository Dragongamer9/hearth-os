//! Desktop layout (SetDesktopLayout.qml; docs/mockups/desktop-layouts/layouts-6-layout-editor.png): where the bars, the
//! dock, the app menu and the window buttons go, for Mantel in Rust and wherever Mantel's own Settings isn't running.
//! The ready-made layouts with their pictures (the one in use marked), the screen with its main bar and panels (Add a
//! panel on a free edge, the selected one lit), blocks dragged from Blocks into a panel or along it (a lit line where
//! it lands; right-click takes one out), This panel and The whole desktop, Share…, Open a layout file…, Back to (the
//! layout), Save as my own….
//!
//! Read with hosts/hearth-layout's library; every change is put in place by `hearth-layout apply-json` (checked,
//! Mantel's own settings merged, atomic, .bak, with Undo; red flag 9), off the window's thread. Opening the page writes
//! nothing.

use super::kit::{self, irow, Ic};
use super::Msg;
use crate::widgets::*;
use crate::App;
use hearth_layout::layout::{Bar, Layout, Panel, WindowButtons};
use hearth_ui::run;
use hearth_ui::ui::{self, glyph, t, t1, ts, G};
use hearth_ui::Pal;
use iced::widget::canvas::{self, Frame, Geometry, Path};
use iced::widget::{column, container, mouse_area, row, stack, Canvas, Column, Row, Space};
use iced::{mouse, Color, Element, Length, Padding, Point, Rectangle, Renderer, Size, Task, Theme};
use serde_json::Value;

type E<'a> = Element<'a, Msg>;

#[derive(Debug, Clone)]
pub enum LMsg {
    Use(String),
    Did(run::Output),
    Empty,
    Sel(String),
    Edge(String),
    Style(String),
    Size(f32),
    SizeDone,
    Hide(bool),
    AddPanel(String),
    RemovePanel,
    AppMenu(String),
    Dock(&'static str),
    WinSide(&'static str),
    WinSet(&'static str),
    Drag(String, String, i32),
    Over(String, usize),
    Drop,
    Remove(String, usize),
    Share,
    OpenFile,
    Picked(run::Output),
    Naming(bool),
    Name(String),
    SaveAs,
}

#[derive(Default)]
pub struct Lay {
    pub all: Vec<(Layout, bool)>,
    pub cur: String,
    pub draft: Option<Layout>,
    pub base: String,
    pub sel: String,
    pub drag: Option<(String, String, i32)>,
    pub over: Option<(String, usize)>,
    pub msg: String,
    pub bad: bool,
    pub naming: bool,
    pub name: String,
    /// keep the copy being changed when the list is read again (after a change), not after picking a layout
    pub keep: bool,
}

pub const BLOCKS_ORDER: &[&str] = &["search", "desktops", "window", "appmenus", "notifications", "quickaccess", "showdesktop", "keyboard", "layout", "spacer",
    "menu", "apps", "windows", "pager", "clock", "tray", "status", "network", "volume", "battery", "bluetooth", "overview", "taskview", "appname", "activity",
    "jobs", "background", "privacy", "printers", "disks", "discord", "widget:weather", "widget:monitor", "widget:battery", "widget:timer"];

pub fn name_of(id: &str) -> &'static str {
    match id {
        "menu" => "App menu", "apps" => "Apps (icons)", "windows" => "Window list", "pager" => "Desktops", "spacer" => "Space", "clock" => "Clock",
        "tray" => "Tray", "status" => "Status", "notifications" => "Notifications", "layout" => "Keyboard layout", "gamemode" => "Game Mode",
        "search" => "Search", "desktops" => "Desktops", "activity" => "Activity", "window" => "Window title", "jobs" => "Copies",
        "background" => "Background apps", "privacy" => "Camera & mic", "printers" => "Printing", "disks" => "Disks", "discord" => "Discord",
        "keyboard" => "Keyboard", "network" => "Wi-Fi", "bluetooth" => "Bluetooth", "volume" => "Volume", "battery" => "Battery",
        "overview" => "Desktops pill", "taskview" => "Task view", "appname" => "App name", "appmenus" => "App menus (File, Edit…)",
        "quickaccess" => "Quick Access", "showdesktop" => "Show desktop", "widget:weather" => "Weather", "widget:monitor" => "System monitor",
        "widget:battery" => "Battery widget", "widget:timer" => "Timer", _ => "Block",
    }
}

/// Reads the layouts (and today's desktop when none is in place).
pub fn open(st: &mut Lay) -> Task<Msg> {
    reload(st, false);
    st.sel = "main".into();
    st.msg.clear();
    Task::none()
}
fn reload(st: &mut Lay, keep: bool) {
    let (all, _) = hearth_layout::apply::all();
    st.cur = hearth_layout::apply::current();
    st.all = all;
    if keep && st.draft.is_some() {
        return;
    }
    if let Some((l, _)) = st.all.iter().find(|(l, _)| l.id == st.cur) {
        st.base = l.id.trim_end_matches("-mine").to_string();
        st.draft = Some(l.clone());
    } else if st.cur.ends_with("-mine") {
        st.base = st.cur.trim_end_matches("-mine").to_string();
        st.draft = Some(hearth_layout::apply::today(&st.cur, &format!("{} (your copy)", base_name(st))));
    } else {
        st.base.clear();
        st.draft = Some(hearth_layout::apply::today("mine", "Your desktop"));
    }
}
fn base_name(st: &Lay) -> String {
    st.all.iter().find(|(l, _)| l.id == st.base).map(|(l, _)| l.name.clone()).unwrap_or_default()
}
fn cmd(args: &[&str]) -> Vec<String> {
    std::iter::once("hearth-layout").chain(args.iter().copied()).map(String::from).collect()
}
fn spawn(c: Vec<String>) -> Task<Msg> {
    Task::perform(run::spawn(c), |o| Msg::Lay(LMsg::Did(o)))
}
fn commit(st: &Lay) -> Task<Msg> {
    match &st.draft {
        Some(d) => spawn(cmd(&["apply-json", &serde_json::to_string(d).unwrap_or_default()])),
        None => Task::none(),
    }
}
fn bar_ok(id: &str) -> bool {
    hearth_layout::blocks::in_bar(id)
}
fn panel_ok(id: &str) -> bool {
    hearth_layout::blocks::in_panel(id)
}
fn dstr<'a>(d: &'a Layout, k: &str) -> &'a str {
    d.desktop.get(k).and_then(Value::as_str).unwrap_or("")
}
fn set(d: &mut Layout, k: &str, v: &str) {
    if v.is_empty() {
        d.desktop.remove(k);
    } else {
        d.desktop.insert(k.into(), Value::String(v.into()));
    }
}
/// The main bar's blocks with its middle and end marked ("|centre", "|end"), as the editor shows them.
fn main_tokens(d: &Layout) -> Vec<String> {
    let b = d.bar.clone().unwrap_or(Bar { start: vec!["menu".into()], ..Default::default() });
    let mut v = b.start.clone();
    v.push("|centre".into());
    v.extend(b.centre.clone());
    v.push("|end".into());
    v.extend(b.end.clone());
    v
}
fn set_main_tokens(d: &mut Layout, t: Vec<String>) {
    let i = t.iter().position(|x| x == "|centre").unwrap_or(t.len());
    let j = t.iter().position(|x| x == "|end").unwrap_or(t.len());
    let (i, j) = (i.min(j), i.max(j));
    let mut start: Vec<String> = t[..i].to_vec();
    let centre: Vec<String> = t[(i + 1).min(t.len())..j].iter().filter(|x| !x.starts_with('|')).cloned().collect();
    let end: Vec<String> = t[(j + 1).min(t.len())..].iter().filter(|x| !x.starts_with('|')).cloned().collect();
    start.retain(|x| !x.starts_with('|'));
    let placed: Vec<&String> = start.iter().chain(&centre).chain(&end).collect();
    if !placed.iter().any(|x| *x == "menu") {
        start.insert(0, "menu".into()); // the flame always stays
    }
    let placed: Vec<String> = start.iter().chain(&centre).chain(&end).cloned().collect();
    let hidden = hearth_layout::blocks::bar_items().map(|b| b.id.to_string()).filter(|x| !placed.contains(x) && x != "menu").collect();
    d.bar = Some(Bar { start, centre, end, hidden });
}
fn tokens(d: &Layout, key: &str) -> Vec<String> {
    if key == "main" {
        main_tokens(d)
    } else {
        key[1..].parse::<usize>().ok().and_then(|i| d.panels.get(i)).map(|p| p.items.clone()).unwrap_or_default()
    }
}
fn put(d: &mut Layout, key: &str, t: Vec<String>) {
    if key == "main" {
        set_main_tokens(d, t);
    } else if let Some(p) = key[1..].parse::<usize>().ok().and_then(|i| d.panels.get_mut(i)) {
        p.items = t;
    }
}
fn main_edge(d: &Layout) -> String {
    d.main_edge().to_string()
}
fn free_edges(d: &Layout) -> Vec<&'static str> {
    let m = main_edge(d);
    ["top", "left", "right", "bottom"].into_iter().filter(|e| *e != m && !(*e == "bottom" && d.dock_on()) && !d.panels.iter().any(|p| p.edge == *e)).collect()
}
fn panel_ix(st: &Lay) -> Option<usize> {
    st.sel.strip_prefix('p').and_then(|n| n.parse().ok())
}

pub fn update(st: &mut Lay, m: LMsg) -> Task<Msg> {
    // a change to the copy: made "your copy", then put in place
    fn change(st: &mut Lay, f: impl FnOnce(&mut Layout)) -> Task<Msg> {
        let base = base_name(st);
        st.keep = true;
        let Some(d) = st.draft.as_mut() else { return Task::none() };
        f(d);
        if !d.id.ends_with("-mine") && d.id != "mine" {
            d.id = format!("{}-mine", d.id);
            d.name = format!("{} (your copy)", if base.is_empty() { d.name.clone() } else { base });
        }
        commit(st)
    }
    match m {
        LMsg::Use(id) => {
            st.keep = false;
            return spawn(cmd(&["apply", &id])).chain(Task::done(Msg::Lay(LMsg::Sel("main".into()))));
        }
        LMsg::Did(o) => {
            if o.ok {
                st.msg.clear();
                st.bad = false;
                reload(st, st.keep);
                if o.out.starts_with("Saved") {
                    st.msg = "Saved with your layouts".into();
                }
            } else {
                st.msg = if o.err.trim().is_empty() { o.out.trim().to_string() } else { o.err.trim().to_string() };
                st.bad = true;
            }
        }
        LMsg::Empty => {
            st.base.clear();
            let mut d = Layout { format: 1, id: "empty-mine".into(), name: "My own layout".into(), about: String::new(), desktop: Default::default(), bar: None, panels: vec![], window_buttons: None };
            set(&mut d, "barLayout", "one");
            set(&mut d, "barEdge", "bottom");
            set(&mut d, "dockFloat", "off");
            set(&mut d, "barStyle", "bar");
            set_main_tokens(&mut d, vec!["menu".into(), "|centre".into(), "|end".into()]);
            st.draft = Some(d);
            st.keep = true;
            return commit(st);
        }
        LMsg::Sel(k) => st.sel = k,
        LMsg::Edge(e) => {
            let ix = panel_ix(st);
            return change(st, |d| match ix {
                None => {
                    set(d, "barLayout", "one");
                    set(d, "barEdge", &e);
                }
                Some(i) => {
                    if let Some(p) = d.panels.get_mut(i) {
                        p.edge = e.clone();
                    }
                }
            });
        }
        LMsg::Style(s) => {
            let ix = panel_ix(st);
            return change(st, |d| match ix {
                None => set(d, "barStyle", if s.is_empty() { "bar" } else { &s }),
                Some(i) => {
                    if let Some(p) = d.panels.get_mut(i) {
                        p.style = s.clone();
                    }
                }
            });
        }
        LMsg::Size(v) => {
            if let (Some(i), Some(d)) = (panel_ix(st), st.draft.as_mut()) {
                if let Some(p) = d.panels.get_mut(i) {
                    p.size = v.round() as u32;
                }
            }
        }
        LMsg::SizeDone => return change(st, |_| {}),
        LMsg::Hide(on) => {
            let ix = panel_ix(st);
            return change(st, |d| match ix {
                None => set(d, "dockMode", if on { "autohide" } else { "" }),
                Some(i) => {
                    if let Some(p) = d.panels.get_mut(i) {
                        p.auto_hide = on;
                    }
                }
            });
        }
        LMsg::AddPanel(e) => {
            let n = st.draft.as_ref().map(|d| d.panels.len()).unwrap_or(0);
            st.sel = format!("p{n}");
            return change(st, |d| d.panels.push(Panel { edge: e, items: vec!["menu".into(), "windows".into(), "spacer".into(), "clock".into()], size: 44, auto_hide: false, style: String::new() }));
        }
        LMsg::RemovePanel => {
            let ix = panel_ix(st);
            st.sel = "main".into();
            return change(st, |d| {
                if let Some(i) = ix {
                    if i < d.panels.len() {
                        d.panels.remove(i);
                    }
                }
            });
        }
        LMsg::AppMenu(v) => {
            kit_close(st);
            return change(st, |d| set(d, "appMenu", &v));
        }
        LMsg::Dock(v) => {
            return change(st, |d| {
                if v == "none" {
                    set(d, "barLayout", "one");
                    if dstr(d, "barEdge").is_empty() {
                        set(d, "barEdge", "bottom");
                    }
                    set(d, "dockFloat", "off");
                } else {
                    set(d, "dockFloat", "on");
                    set(d, "dockMode", if v == "hide" { "autohide" } else { "" });
                }
            })
        }
        LMsg::WinSide(s) => {
            return change(st, |d| {
                let all = d.window_buttons.as_ref().map(|w| w.buttons.len() > 1).unwrap_or(true);
                d.window_buttons = Some(WindowButtons { side: s.into(), buttons: buttons(s, all) });
            })
        }
        LMsg::WinSet(v) => {
            return change(st, |d| {
                let side = d.window_buttons.as_ref().map(|w| w.side.clone()).unwrap_or_else(|| "right".into());
                d.window_buttons = Some(WindowButtons { buttons: buttons(&side, v == "all"), side });
            })
        }
        LMsg::Drag(id, from, at) => {
            st.drag = Some((id, from, at));
            st.over = None;
        }
        LMsg::Over(k, i) => {
            if st.drag.is_some() {
                st.over = Some((k, i));
            }
        }
        LMsg::Drop => {
            let (Some((id, from, at)), Some((key, to))) = (st.drag.take(), st.over.take()) else {
                st.drag = None;
                st.over = None;
                return Task::none();
            };
            if from == key && at >= 0 && (to as i32 == at || to as i32 == at + 1) {
                return Task::none();
            }
            let ok = if key == "main" { bar_ok(&id) } else { panel_ok(&id) };
            if !ok {
                st.msg = format!("{} can't go {}.", name_of(&id), if key == "main" { "in the main bar" } else { "in a panel" });
                st.bad = true;
                return Task::none();
            }
            st.sel = key.clone();
            return change(st, |d| {
                let mut to = to;
                if from == key && at >= 0 {
                    let mut t = tokens(d, &key);
                    let x = t.remove(at as usize);
                    if to > at as usize {
                        to -= 1;
                    }
                    t.insert(to.min(t.len()), x);
                    put(d, &key, t);
                } else {
                    let mut t = tokens(d, &key);
                    if key == "main" {
                        if let Some(p) = t.iter().position(|x| *x == id) {
                            t.remove(p);
                            if p < to {
                                to -= 1;
                            }
                        }
                    }
                    t.insert(to.min(t.len()), id.clone());
                    put(d, &key, t);
                }
            });
        }
        LMsg::Remove(key, at) => {
            let Some(d) = &st.draft else { return Task::none() };
            let t = tokens(d, &key);
            if key == "main" && t.get(at).map(|x| x == "menu").unwrap_or(false) {
                st.msg = "The app menu always stays in the main bar.".into();
                st.bad = true;
                return Task::none();
            }
            if key != "main" && t.len() <= 1 {
                st.msg = "A panel needs at least one block: remove the panel instead.".into();
                st.bad = true;
                return Task::none();
            }
            return change(st, |d| {
                let mut t = tokens(d, &key);
                if at < t.len() {
                    t.remove(at);
                }
                put(d, &key, t);
            });
        }
        LMsg::Share => {
            let Some(d) = &st.draft else { return Task::none() };
            let home = std::env::var("HOME").unwrap_or_default();
            let f = format!("{home}/Documents/{}.hearth-layout.json", d.id);
            let json = serde_json::to_string_pretty(d).unwrap_or_default();
            let c = vec!["sh".into(), "-c".into(), r#"mkdir -p "$(dirname "$1")" && printf %s "$2" > "$1.tmp" && mv "$1.tmp" "$1" && xdg-open "$(dirname "$1")" >/dev/null 2>&1 & echo "Saved to share: $1""#.into(), "sh".into(), f, json];
            return Task::perform(run::spawn(c), |o| Msg::Lay(LMsg::Picked(run::Output { out: format!("#share {}", o.out), ..o })));
        }
        LMsg::OpenFile => {
            let c = kit::sh_t("if command -v kdialog >/dev/null; then kdialog --title 'Open a layout file' --getopenfilename \"$HOME\" '*.json'; elif command -v zenity >/dev/null; then zenity --file-selection --title='Open a layout file' --filename=\"$HOME/\" --file-filter='Layouts | *.json'; else exit 3; fi", 600);
            return Task::perform(run::spawn(c), |o| Msg::Lay(LMsg::Picked(o)));
        }
        LMsg::Picked(o) => {
            if let Some(s) = o.out.strip_prefix("#share ") {
                st.msg = if o.ok { format!("{} (anyone with Hearth OS can open it here)", s.trim()) } else { "Couldn't save it to share".into() };
                st.bad = !o.ok;
                return Task::none();
            }
            let f = o.out.trim().to_string();
            if o.ok && !f.is_empty() {
                return update(st, LMsg::Use(f));
            }
        }
        LMsg::Naming(on) => st.naming = on,
        LMsg::Name(s) => st.name = s,
        LMsg::SaveAs => {
            if st.name.trim().is_empty() {
                return Task::none();
            }
            st.naming = false;
            st.keep = true;
            return spawn(cmd(&["save", st.name.trim()]));
        }
    }
    Task::none()
}
fn kit_close(_st: &mut Lay) {}
fn buttons(side: &str, all: bool) -> Vec<String> {
    let v: &[&str] = match (side, all) {
        (_, false) => &["close"],
        ("left", true) => &["close", "iconify", "max"],
        _ => &["iconify", "max", "close"],
    };
    v.iter().map(|s| s.to_string()).collect()
}

// ---------------------------------------------------------------- the page
pub fn page(a: &App) -> E<'_> {
    let p = a.sh.p;
    let st = &a.pers.lay;
    let Some(d) = &st.draft else { return t("Reading your layouts…", sz(14.0), p.muted).into() };
    let yours = d.id.ends_with("-mine");
    let mut top = Row::new().spacing(10.0).align_y(iced::Center).push(
        container(t("Where the bars, the dock, the app menu and the window buttons go. Your look's colours stay as they are.", sz(14.0), p.muted)).width(Length::Fill),
    );
    top = top.push(btn(&p, "Share…", ui::Kind::Normal, false, Some(Msg::Lay(LMsg::Share))));
    top = top.push(btn(&p, "Open a layout file…", ui::Kind::Normal, false, Some(Msg::Lay(LMsg::OpenFile))));
    if yours && !st.base.is_empty() {
        top = top.push(btn(&p, format!("Back to {}", base_name(st)), ui::Kind::Normal, false, Some(Msg::Lay(LMsg::Use(st.base.clone())))));
    }
    top = top.push(btn(&p, "Save as my own…", ui::Kind::Primary, false, Some(Msg::Lay(LMsg::Naming(true)))));
    let mut c = Column::new().spacing(14.0).push(top);
    if st.naming {
        c = c.push(row![field(&p, "A name for this layout", &st.name, |s| Msg::Lay(LMsg::Name(s))).on_submit(Msg::Lay(LMsg::SaveAs)), btn(&p, "Save", ui::Kind::Primary, false, Some(Msg::Lay(LMsg::SaveAs))), btn(&p, "Cancel", ui::Kind::Normal, false, Some(Msg::Lay(LMsg::Naming(false))))].spacing(10.0));
    }
    if !st.msg.is_empty() {
        c = c.push(kit::said(&p, &st.msg, st.bad));
    }
    // the ready-made layouts and yours, with their pictures
    let mut g = Row::new().spacing(14.0);
    for (l, mine) in &st.all {
        let on = l.id == st.cur || d.id == format!("{}-mine", l.id);
        let label = format!("{}{}{}", l.name, if on { if d.id == format!("{}-mine", l.id) { " · your copy" } else { " · in use" } } else { "" }, if *mine { " (yours)" } else { "" });
        let pic = container(Canvas::new(Mini { lay: l.clone(), p }).width(Length::Fixed(170.0)).height(Length::Fixed(96.0)))
            .padding(3.0)
            .style(ui::boxed(Color::TRANSPARENT, p.r(10.0), Some((if on { p.accent } else { p.line }, if on { 2.0 } else { 1.0 }))));
        g = g.push(mouse_area(column![pic, if on { ts(label, sz(14.0), p.text) } else { t(label, sz(14.0), p.text) }].spacing(8.0).width(Length::Fixed(176.0))).on_press(Msg::Lay(LMsg::Use(l.id.clone()))).interaction(mouse::Interaction::Pointer));
    }
    let empty = container(t1("+", 30.0, p.muted)).center(Length::Fixed(176.0)).height(Length::Fixed(102.0)).style(ui::boxed(Color::TRANSPARENT, p.r(10.0), Some((p.line, 1.0))));
    g = g.push(mouse_area(column![empty, t("Start from empty", sz(14.0), p.text)].spacing(8.0)).on_press(Msg::Lay(LMsg::Empty)).interaction(mouse::Interaction::Pointer));
    c = c.push(g.wrap());

    // the screen with its bars, and the right-hand column
    let title = row![
        ts(format!("{}{}", d.name.trim_end_matches(" (your copy)"), if yours { " · your copy" } else { "" }), sz(17.0), p.text),
        Space::new().width(Length::Fill),
        t("Drag blocks into a panel, or along it · right-click a block to take it out", sz(13.0), p.muted)
    ]
    .align_y(iced::Center);
    let left = column![title, screen(&p, st, d)].spacing(8.0).width(Length::Fill);
    let right = column![blocks_card(&p, st, d), panel_card(&p, st, d), whole_card(&p, st, d)].spacing(14.0).width(Length::Fixed(430.0));
    c = c.push(row![left, right].spacing(18.0));
    // Letting go anywhere on the page drops a dragged block where the lit line is.
    mouse_area(c).on_release(Msg::Lay(LMsg::Drop)).into()
}

fn chip<'a>(p: &Pal, id: &str, from: &str, at: i32, lit: bool) -> E<'a> {
    let label: E<'a> = if id == "menu" { row![container(Space::new()).width(Length::Fixed(12.0)).height(Length::Fixed(12.0)).style(ui::boxed(p.flame, 6.0, None)), t1(name_of(id), sz(13.0), p.text)].spacing(7.0).align_y(iced::Center).into() } else { t1(name_of(id), sz(13.0), p.text).into() };
    let inner = if from.is_empty() { row![t1("⠿", sz(13.0), p.muted), label].spacing(7.0).align_y(iced::Center) } else { row![label].align_y(iced::Center) };
    let mut m = mouse_area(
        container(inner)
            .padding(Padding { top: if from.is_empty() { 6.0 } else { 10.0 }, right: 12.0, bottom: if from.is_empty() { 6.0 } else { 10.0 }, left: 12.0 })
            .style(ui::boxed(if lit { p.panel3 } else { p.bg2 }, p.r(8.0), Some((p.line, 1.0)))),
    )
    .on_press(Msg::Lay(LMsg::Drag(id.to_string(), from.to_string(), at)))
    .interaction(mouse::Interaction::Grab);
    if !from.is_empty() {
        m = m.on_right_press(Msg::Lay(LMsg::Remove(from.to_string(), at.max(0) as usize))).on_enter(Msg::Lay(LMsg::Over(from.to_string(), at.max(0) as usize)));
    }
    m.into()
}
fn line<'a>(p: &Pal, vert: bool) -> E<'a> {
    container(Space::new()).width(Length::Fixed(if vert { 110.0 } else { 3.0 })).height(Length::Fixed(if vert { 3.0 } else { 40.0 })).style(ui::boxed(p.accent, 1.0, None)).into()
}

/// One bar on the screen: its blocks (the main bar's middle and end as Space), Game Mode last in the main bar.
fn bar_edit<'a>(p: &Pal, st: &Lay, d: &Layout, key: &str, edge: &str, style: &str) -> E<'a> {
    let vert = edge == "left" || edge == "right";
    let ts_ = tokens(d, key);
    let mut items: Vec<E<'a>> = Vec::new();
    for (i, id) in ts_.iter().enumerate() {
        if st.drag.is_some() && st.over.as_ref() == Some(&(key.to_string(), i)) {
            items.push(line(p, vert));
        }
        if id.starts_with('|') {
            let sp = container(t1("space", sz(12.0), p.muted)).center_x(Length::Fixed(if vert { 110.0 } else if key == "main" { 150.0 } else { 90.0 })).height(Length::Fixed(if vert { 30.0 } else { 40.0 })).align_y(iced::Center).style(ui::boxed(Color::TRANSPARENT, p.r(8.0), Some((p.line, 1.0))));
            items.push(mouse_area(sp).on_enter(Msg::Lay(LMsg::Over(key.to_string(), i))).into());
        } else {
            items.push(chip(p, id, key, i as i32, false));
        }
    }
    if st.drag.is_some() && st.over.as_ref() == Some(&(key.to_string(), ts_.len())) {
        items.push(line(p, vert));
    }
    // the end of the bar takes a dropped block too
    items.push(mouse_area(container(Space::new()).width(Length::Fixed(if vert { 110.0 } else { 24.0 })).height(Length::Fixed(if vert { 24.0 } else { 40.0 }))).on_enter(Msg::Lay(LMsg::Over(key.to_string(), ts_.len()))).into());
    if key == "main" {
        items.push(container(row![glyph(G::Pad, p.ink, 16.0, 2.0), t1("Game Mode", sz(13.0), p.ink)].spacing(8.0).align_y(iced::Center)).padding(Padding { top: 10.0, right: 16.0, bottom: 10.0, left: 16.0 }).style(ui::boxed(p.accent, p.r(8.0), None)).into());
    }
    let body: E<'a> = if vert { Column::with_children(items).spacing(8.0).align_x(iced::Center).into() } else { Row::with_children(items).spacing(8.0).align_y(iced::Center).into() };
    let picked = st.sel == key;
    let floats = style == "floating" || style == "centred";
    let mut b = container(body).padding(10.0).style(ui::boxed(Pal::a(p.panel, 0.92), if floats { p.r(14.0) } else { 0.0 }, if picked { Some((p.accent, 2.0)) } else { None }));
    b = if vert { b.height(if style == "centred" { Length::Shrink } else { Length::Fill }).center_y(Length::Fill) } else { b.width(if style == "centred" { Length::Shrink } else { Length::Fill }).center_x(Length::Fill) };
    let b = mouse_area(b).on_press(Msg::Lay(LMsg::Sel(key.to_string())));
    let gap = if floats { 12.0 } else { 0.0 };
    let w = container(b).padding(gap).width(Length::Fill).height(Length::Fill);
    match edge {
        "top" => w.align_top(Length::Fill).into(),
        "bottom" => w.align_bottom(Length::Fill).into(),
        "left" => w.align_left(Length::Fill).into(),
        _ => w.align_right(Length::Fill).into(),
    }
}

fn screen<'a>(p: &Pal, st: &Lay, d: &Layout) -> E<'a> {
    let mut layers: Vec<E<'a>> = vec![container(Space::new()).width(Length::Fill).height(Length::Fill).style(ui::boxed(Pal::a(p.bg, 0.9), p.r(12.0), Some((p.line, 1.0)))).into()];
    for e in free_edges(d) {
        let vert = e == "left" || e == "right";
        let label = match e { "top" => "+ Add a panel at the top", "bottom" => "+ Add a panel at the bottom", "left" => "+ Add a panel\non the left", _ => "+ Add a panel\non the right" };
        let add = mouse_area(container(t1(label, sz(13.0), p.muted)).center_x(if vert { Length::Fixed(220.0) } else { Length::Fill }).center_y(Length::Fixed(if vert { 200.0 } else { 44.0 })).style(ui::boxed(Color::TRANSPARENT, p.r(8.0), Some((p.line, 1.5)))))
            .on_press(Msg::Lay(LMsg::AddPanel(e.to_string())))
            .interaction(mouse::Interaction::Pointer);
        let pad = Padding { top: if e == "top" { 22.0 } else if vert { 110.0 } else { 0.0 }, bottom: if e == "bottom" { 22.0 } else { 0.0 }, left: 24.0, right: 24.0 };
        let w = container(add).padding(pad).width(Length::Fill).height(Length::Fill);
        layers.push(match e { "top" => w.align_top(Length::Fill).into(), "bottom" => w.align_bottom(Length::Fill).into(), "left" => w.align_left(Length::Fill).align_top(Length::Fill).into(), _ => w.align_right(Length::Fill).align_top(Length::Fill).into() });
    }
    if d.dock_on() {
        let dock = container(t1("Dock: the flame and your apps", sz(12.0), p.muted)).padding(Padding { top: 16.0, right: 24.0, bottom: 16.0, left: 24.0 }).style(ui::boxed(Pal::a(p.panel, 0.8), p.r(16.0), Some((p.line, 1.0))));
        layers.push(container(dock).padding(Padding { bottom: 16.0, ..Padding::ZERO }).center_x(Length::Fill).align_bottom(Length::Fill).into());
    }
    let ms = if dstr(d, "barStyle") == "floating" { "floating" } else { "" };
    layers.push(bar_edit(p, st, d, "main", &main_edge(d), ms));
    for (i, pn) in d.panels.iter().enumerate() {
        layers.push(bar_edit(p, st, d, &format!("p{i}"), &pn.edge, &pn.style));
    }
    container(stack(layers)).width(Length::Fill).height(Length::Fixed(560.0)).into()
}

fn card_<'a>(p: &Pal, title: &str, rows: Vec<E<'a>>) -> E<'a> {
    let mut c = Column::new().spacing(8.0).push(ts(title.to_string(), sz(16.0), p.text));
    for r in rows {
        c = c.push(r);
    }
    container(c).padding(14.0).width(Length::Fill).style(ui::boxed(Pal::a(p.panel, 0.7), p.r(12.0), Some((p.line, 1.0)))).into()
}

fn blocks_card<'a>(p: &Pal, st: &Lay, d: &Layout) -> E<'a> {
    let main = st.sel == "main";
    let have = main_tokens(d);
    let mut r = Row::new().spacing(8.0);
    for id in BLOCKS_ORDER {
        let ok = if main { bar_ok(id) && !have.iter().any(|x| x == id) } else { panel_ok(id) };
        if ok {
            r = r.push(chip(p, id, "", -1, st.drag.as_ref().map(|x| x.0 == *id).unwrap_or(false)));
        }
    }
    card_(p, "Blocks", vec![r.wrap().vertical_spacing(8.0).into()])
}

fn panel_card<'a>(p: &Pal, st: &Lay, d: &Layout) -> E<'a> {
    let pn = panel_ix(st).and_then(|i| d.panels.get(i));
    let edge = pn.map(|x| x.edge.clone()).unwrap_or_else(|| main_edge(d));
    let style = pn.map(|x| x.style.clone()).unwrap_or_else(|| if dstr(d, "barStyle") == "floating" { "floating".into() } else { String::new() });
    let mut rows: Vec<E<'a>> = vec![
        irow(p, Ic::G(G::Display), "Edge", "", seg(p, ["top", "bottom", "left", "right"].iter().map(|e| (titlecase(e), edge == *e, Some(Msg::Lay(LMsg::Edge(e.to_string()))))).collect())),
        irow(p, Ic::Window, "Style", "", seg(p, vec![
            ("Full width".into(), style.is_empty(), Some(Msg::Lay(LMsg::Style(String::new())))),
            ("Floating".into(), style == "floating", Some(Msg::Lay(LMsg::Style("floating".into())))),
            ("Centred".into(), style == "centred", if pn.is_some() { Some(Msg::Lay(LMsg::Style("centred".into()))) } else { None }),
        ])),
    ];
    match pn {
        Some(x) => rows.push(irow(p, Ic::Window, "Height", "", row![
            iced::widget::slider(28.0..=96.0, x.size as f32, |v| Msg::Lay(LMsg::Size(v))).step(1.0).on_release(Msg::Lay(LMsg::SizeDone)).width(Length::Fixed(sz(170.0))),
            t1(format!("{} px", x.size), sz(13.0), p.muted)
        ].spacing(10.0).align_y(iced::Center).into())),
        None => rows.push(kit::small(p, "The main bar's height comes with your look.")),
    }
    let hide = pn.map(|x| x.auto_hide).unwrap_or(dstr(d, "dockMode") == "autohide");
    rows.push(irow(p, Ic::Eye, "Hide until the pointer touches the edge", "", switch(p, hide, Some(Msg::Lay(LMsg::Hide(!hide))))));
    if pn.is_some() {
        rows.push(btn(p, "Remove this panel", ui::Kind::Danger, false, Some(Msg::Lay(LMsg::RemovePanel))));
    }
    card_(p, if pn.is_some() { "This panel" } else { "The main bar" }, rows)
}

fn whole_card<'a>(p: &Pal, _st: &Lay, d: &Layout) -> E<'a> {
    let am = dstr(d, "appMenu");
    let menus = [("", "Hearth"), ("start", "Start menu"), ("centred", "Centred"), ("tiles", "Tiles"), ("classic", "Classic")];
    let dock = if !d.dock_on() { "none" } else if dstr(d, "dockMode") == "autohide" { "hide" } else { "floating" };
    let wb = d.window_buttons.clone().unwrap_or(WindowButtons { side: "right".into(), buttons: buttons("right", true) });
    let all = wb.buttons.len() > 1;
    let rows: Vec<E<'a>> = vec![
        irow(p, Ic::Menu, "App menu", "", seg(p, menus.iter().map(|(v, l)| (l.to_string(), am == *v, Some(Msg::Lay(LMsg::AppMenu(v.to_string()))))).collect())),
        irow(p, Ic::G(G::Grid), "Dock", "", seg(p, vec![
            ("None".into(), dock == "none", Some(Msg::Lay(LMsg::Dock("none")))),
            ("Floating".into(), dock == "floating", Some(Msg::Lay(LMsg::Dock("floating")))),
            ("Hide".into(), dock == "hide", Some(Msg::Lay(LMsg::Dock("hide")))),
        ])),
        irow(p, Ic::Window, "Window buttons", "", row![
            seg(p, vec![("Left".into(), wb.side == "left", Some(Msg::Lay(LMsg::WinSide("left")))), ("Right".into(), wb.side != "left", Some(Msg::Lay(LMsg::WinSide("right"))))]),
            seg(p, vec![("Close".into(), !all, Some(Msg::Lay(LMsg::WinSet("close")))), ("All three".into(), all, Some(Msg::Lay(LMsg::WinSet("all"))))]),
        ].spacing(8.0).into()),
        kit::small(p, "Game Mode always stays in a panel, in every layout."),
    ];
    card_(p, "The whole desktop", rows)
}

fn titlecase(s: &str) -> String {
    let mut c = s.chars();
    c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
}

// ---------------------------------------------------------------- a layout drawn small (the gallery's pictures)
struct Mini {
    lay: Layout,
    p: Pal,
}
impl canvas::Program<Msg> for Mini {
    type State = ();
    fn draw(&self, _s: &(), r: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        let mut f = Frame::new(r, b.size());
        let (w, h) = (b.width, b.height);
        let p = &self.p;
        let th = (h * 0.065).max(5.0);
        f.fill(&Path::rounded_rectangle(Point::ORIGIN, b.size(), p.r(8.0).into()), p.bg2);
        let dim = Pal::a(p.text, 0.13);
        let dot = Pal::a(p.text, 0.35);
        let mut strip = |f: &mut Frame, edge: &str, dots: Vec<String>, floats: bool| {
            let vert = edge == "left" || edge == "right";
            let g = if floats { 3.0 } else { 0.0 };
            let (x, y, sw, sh) = match edge {
                "top" => (g, g, w - g * 2.0, th),
                "bottom" => (g, h - th - g, w - g * 2.0, th),
                "left" => (g, g, th, h - g * 2.0),
                _ => (w - th - g, g, th, h - g * 2.0),
            };
            f.fill(&Path::rounded_rectangle(Point::new(x, y), Size::new(sw, sh), (if floats { th / 2.0 } else { 0.0 }).into()), dim);
            let mut at = 2.0;
            for d in dots.iter().take(18) {
                let len = if d == "spacer" { th * 2.0 } else { th - 4.0 };
                if at + len > if vert { sh } else { sw } {
                    break;
                }
                let c = if d == "menu" || d == "gamemode" { p.accent } else if d == "spacer" { Color::TRANSPARENT } else { dot };
                let (dx, dy) = if vert { (x + 2.0, y + at) } else { (x + at, y + 2.0) };
                f.fill(&Path::rounded_rectangle(Point::new(dx, dy), if vert { Size::new(th - 4.0, len) } else { Size::new(len, th - 4.0) }, 1.5.into()), c);
                at += len + 2.0;
            }
        };
        let l = &self.lay;
        let bar = l.bar.clone().unwrap_or(Bar { start: vec!["menu".into()], end: vec!["network".into(), "volume".into(), "battery".into()], ..Default::default() });
        let mut dots = bar.start.clone();
        dots.push("spacer".into());
        dots.extend(bar.centre.clone());
        dots.push("spacer".into());
        dots.extend(bar.end.clone());
        dots.push("gamemode".into());
        strip(&mut f, l.main_edge(), dots, dstr(l, "barStyle") == "floating");
        for pn in &l.panels {
            strip(&mut f, &pn.edge, pn.items.clone(), !pn.style.is_empty());
        }
        if l.dock_on() {
            let dw = w * 0.3;
            f.fill(&Path::rounded_rectangle(Point::new((w - dw) / 2.0, h - th * 1.2 - 4.0), Size::new(dw, th * 1.2), (th * 0.6).into()), Pal::a(p.text, 0.18));
        }
        vec![f.into_geometry()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lay() -> Layout {
        serde_json::from_str(r#"{"format":1,"id":"plasma","name":"Plasma-style","desktop":{"barLayout":"one","barEdge":"bottom","dockFloat":"off"},"bar":{"start":["menu"],"end":["tray","network"]}}"#).unwrap()
    }

    #[test]
    fn the_main_bar_keeps_its_flame_and_marks() {
        let mut d = lay();
        let t = main_tokens(&d);
        assert_eq!(t, ["menu", "|centre", "|end", "tray", "network"]);
        set_main_tokens(&mut d, vec!["|centre".into(), "search".into(), "|end".into(), "tray".into()]);
        let b = d.bar.clone().unwrap();
        assert_eq!(b.start, ["menu"], "the flame comes back by itself");
        assert_eq!(b.centre, ["search"]);
        assert!(b.hidden.contains(&"network".to_string()));
        assert!(hearth_layout::layout::check(&d).is_empty(), "{:?}", hearth_layout::layout::check(&d));
    }

    #[test]
    fn a_dropped_block_lands_where_the_line_is() {
        let mut st = Lay { draft: Some(lay()), sel: "main".into(), ..Default::default() };
        let _ = update(&mut st, LMsg::Drag("search".into(), String::new(), -1));
        let _ = update(&mut st, LMsg::Over("main".into(), 2));
        let _ = update(&mut st, LMsg::Drop);
        let d = st.draft.unwrap();
        assert_eq!(d.bar.unwrap().centre, ["search"]);
        assert!(d.id.ends_with("-mine"), "a change makes it your copy");
    }

    #[test]
    fn free_edges_leave_the_bar_and_the_dock() {
        let d = lay();
        assert_eq!(free_edges(&d), ["top", "left", "right"]);
        let split: Layout = serde_json::from_str(r#"{"format":1,"id":"x","name":"X"}"#).unwrap();
        assert_eq!(free_edges(&split), ["left", "right"]);
    }
}
