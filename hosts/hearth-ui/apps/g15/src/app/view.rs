//! The app's pages, drawn as the approved mockups show them (docs/mockups/g15, 2000 x 1250; here at 1280 x 800,
//! 0.64 of their size): the title bar, the sidebar with the keyboard and its pages, each page, and the controller's
//! buttons at the bottom.

use crate::pics::{self, AMBER, INK, ORANGE};
use crate::{bounds, el_name, value_name, App, Dlg, Msg, Page};
use hearth_g15::actions;
use hearth_g15::config;
use hearth_g15::values::VALUES;
use hearth_ui::ui::{self, btn, gap, glyph, hline, t, t1, tb, tm, ts, Kind, G};
use hearth_ui::Pal;
use iced::widget::{canvas, column, container, mouse_area, row, scrollable, stack, text, text_input, Column, Row, Space};
use iced::{mouse, Color, Element, Length, Padding, Size, Theme};
use serde_json::{json, Value};

pub type E<'a> = Element<'a, Msg>;

/// The editor's screen, in the window (pointer positions are turned into dots with it).
pub const EDITOR_SIZE: Size = Size { width: 580.0, height: 166.0 };

/// Something the controller (or the keyboard) can be on: what Enter, Left and Right do there.
#[derive(Clone)]
pub struct Item {
    pub id: String,
    pub enter: Option<Msg>,
    pub left: Option<Msg>,
    pub right: Option<Msg>,
}

fn it(id: impl Into<String>, enter: Option<Msg>) -> Item {
    Item { id: id.into(), enter, left: None, right: None }
}
fn lr(id: impl Into<String>, enter: Option<Msg>, left: Option<Msg>, right: Option<Msg>) -> Item {
    Item { id: id.into(), enter, left, right }
}

fn flag(a: &App, k: &str, d: bool) -> bool {
    config::flag(&a.cfg, k, d)
}
fn level(a: &App, k: &str, d: u32) -> u32 {
    config::light(&a.cfg, k).unwrap_or(d)
}
fn s(v: &Value, k: &str) -> String {
    v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string()
}

/// The page's rows, in order (the view rings the one the controller is on).
pub fn focusables(a: &App) -> Vec<Item> {
    let mut v: Vec<Item> = Vec::new();
    let three = |id: &str, k: &'static str, d: u32| {
        let cur = level(a, k, d);
        lr(id, None, Some(Msg::Level(k, cur.saturating_sub(1))), Some(Msg::Level(k, (cur + 1).min(2))))
    };
    match a.page {
        Page::Overview => {
            for n in 1..=3u8 {
                v.push(it(format!("set{n}"), Some(Msg::UseSet(n))));
            }
            v.push(lr("switch", Some(Msg::Flag("switchSets", !flag(a, "switchSets", true))), None, Some(Msg::GamesOpen)));
            v.push(it("mr", Some(Msg::Flag("mrRecord", !flag(a, "mrRecord", true)))));
        }
        Page::GKeys => {
            v.push(lr("sets", Some(Msg::RenameOpen), Some(Msg::ViewSet(if a.set > 1 { a.set - 1 } else { 1 })), Some(Msg::ViewSet((a.set + 1).min(3)))));
            for g in 1..=18u8 {
                v.push(lr(format!("g{g}"), Some(Msg::PickG(g)), (g % 3 != 1).then(|| Msg::PickG(g - 1)), (g % 3 != 0).then(|| Msg::PickG(g + 1))));
            }
            let kinds = KINDS.iter().map(|(k, _)| *k).collect::<Vec<_>>();
            let cur = kinds.iter().position(|k| *k == a.kind()).unwrap_or(5);
            v.push(lr("kind", Some(Msg::Kind(kinds[cur])), cur.checked_sub(1).map(|i| Msg::Kind(kinds[i])), kinds.get(cur + 1).map(|k| Msg::Kind(*k))));
            v.push(it("change", Some(Msg::Change)));
            v.push(it("test", Some(Msg::TestIt)));
            v.push(it("clear", Some(Msg::ClearKey)));
            if a.macro_key().is_some() {
                v.push(it("record", Some(Msg::Record)));
                v.push(lr("opts", Some(Msg::MacroOpt("wait")), Some(Msg::MacroOpt("repeat")), Some(Msg::MacroOpt("held"))));
            }
        }
        Page::Screen => {
            v.push(three("bright", "lcdLight", 2));
            let c = level(a, "lcdContrast", 1);
            v.push(lr("contrast", None, Some(Msg::Level("lcdContrast", c.saturating_sub(1))), Some(Msg::Level("lcdContrast", (c + 1).min(2)))));
            v.push(it("idle", Some(pick_idle())));
            for p in hearth_g15::lcd::screen::BUILT_IN {
                v.push(lr(format!("page-{p}"), Some(Msg::TogglePage(p.to_string())), Some(Msg::MovePage(p.to_string(), -1)), Some(Msg::MovePage(p.to_string(), 1))));
            }
            for o in a.own() {
                let id = s(&o, "id");
                v.push(lr(format!("own-{id}"), Some(Msg::Edit(id.clone())), None, Some(Msg::TogglePage(id))));
            }
            v.push(it("new", Some(Msg::NewScreen)));
            v.push(it("gamescreen", Some(Msg::Flag("gameScreens", !flag(a, "gameScreens", true)))));
            v.push(it("autopage", Some(Msg::Flag("autoPage", !flag(a, "autoPage", false)))));
        }
        Page::Lighting => {
            v.push(three("kbd", "kbdLight", 2));
            v.push(three("lcd", "lcdLight", 2));
            v.push(it("locked", Some(Msg::Flag("lightsOffWhenLocked", !flag(a, "lightsOffWhenLocked", false)))));
            v.push(it("night", Some(Msg::Flag("nightDim", !flag(a, "nightDim", false)))));
            v.push(it("mlights", Some(Msg::Flag("mLights", !flag(a, "mLights", true)))));
        }
        Page::Editor => {
            for (k, _, _) in ADD {
                v.push(it(format!("add-{k}"), Some(Msg::Add(*k))));
            }
            let els = a.elements();
            for (i, _) in els.iter().enumerate() {
                v.push(it(format!("el{i}"), Some(Msg::PickEl(i))));
            }
            // The picked element's settings: Left / Right change a choice, A opens a list.
            if let Some(e) = a.el.and_then(|i| els.get(i)) {
                let ty = s(e, "type");
                let cycle = |id: &str, key: &'static str, opts: Vec<Value>, cur: Value| -> Item {
                    let i = opts.iter().position(|o| *o == cur).unwrap_or(0);
                    lr(id, None, i.checked_sub(1).map(|j| Msg::ElSet(key, opts[j].clone())), opts.get(i + 1).map(|o| Msg::ElSet(key, o.clone())))
                };
                let num = |k: &str, d: i64| json!(e.get(k).and_then(|x| x.as_i64()).unwrap_or(d));
                if matches!(ty.as_str(), "value" | "bar" | "graph") {
                    v.push(it("p-value", Some(Msg::Picked("value", "@open".into()))));
                }
                if ty == "graph" {
                    v.push(cycle("p-span", "span", vec![json!(60), json!(300), json!(3600)], num("span", 300)));
                }
                if matches!(ty.as_str(), "bar" | "graph") {
                    v.push(it("p-range", Some(Msg::Picked("range", "@open".into()))));
                }
                if ty == "graph" {
                    v.push(cycle("p-style", "style", vec![json!("line"), json!("filled")], json!(if s(e, "style") == "filled" { "filled" } else { "line" })));
                }
                if ty == "clock" {
                    let f = s(e, "format");
                    let mut c = cycle("p-format", "format", CLOCK.iter().map(|(k, _)| json!(k)).collect(), json!(if f.is_empty() { "24h" } else { f.as_str() }));
                    c.enter = Some(Msg::Picked("format", "@open".into()));
                    v.push(c);
                }
                if matches!(ty.as_str(), "text" | "value" | "clock") {
                    v.push(cycle("p-size", "size", (1..=4).map(|n| json!(n)).collect(), num("size", 1)));
                }
                if ty == "box" {
                    v.push(cycle("p-fill", "fill", vec![json!(false), json!(true)], json!(e.get("fill").and_then(|x| x.as_bool()).unwrap_or(false))));
                }
                if ty == "picture" {
                    v.push(cycle("p-shade", "shade", vec![json!(true), json!(false)], json!(e.get("shade").and_then(|x| x.as_bool()).unwrap_or(true))));
                    v.push(cycle("p-dark", "threshold", DARK.iter().map(|(t, _)| json!(t)).collect(), num("threshold", 128)));
                }
            }
            v.push(it("s-show", Some(Msg::Picked("show", "@open".into()))));
            v.push(it("s-while", Some(Msg::Picked("while", "@open".into()))));
            v.push(it("undo", Some(Msg::Undo)));
            v.push(it("show", Some(Msg::Show)));
            v.push(it("done", Some(Msg::Done)));
        }
    }
    v
}

fn pick_idle() -> Msg {
    Msg::Picked("idle", "@open".into())
}

pub fn x_action(a: &App) -> Msg {
    match a.page {
        Page::GKeys => Msg::TestIt,
        Page::Editor => Msg::Delete,
        _ => Msg::Show,
    }
}
pub fn y_action(a: &App) -> Option<Msg> {
    match a.page {
        Page::GKeys => Some(Msg::Record),
        // Y: to the Add list.
        Page::Editor => Some(Msg::Focus(ix(a, "add-text"))),
        _ => None,
    }
}

fn on(a: &App, id: &str) -> bool {
    focusables(a).get(a.focus).map(|i| i.id == id).unwrap_or(false)
}

pub fn ix(a: &App, id: &str) -> usize {
    focusables(a).iter().position(|i| i.id == id).unwrap_or(0)
}

// ------------------------------------------------------------------ small pieces
fn sz(v: f32) -> f32 {
    v * 0.64
}

fn card<'a>(p: &Pal, inner: impl Into<E<'a>>) -> container::Container<'a, Msg> {
    container(inner).style(ui::boxed(p.panel, p.r(12.0), None))
}

/// A bordered card that's picked (orange) or ringed by the controller.
fn card_on<'a>(p: &Pal, inner: impl Into<E<'a>>, picked: bool, focused: bool) -> E<'a> {
    let pp = *p;
    let line = if picked || focused { Some((ORANGE, if picked { 2.0 } else { 2.0 })) } else { None };
    container(inner).style(ui::boxed(pp.panel, pp.r(12.0), line)).into()
}

fn switch<'a>(p: &Pal, on: bool, msg: Msg) -> E<'a> {
    let p = *p;
    let w = 46.0;
    let knob = container(Space::new()).width(Length::Fixed(16.0)).height(Length::Fixed(16.0)).style(ui::boxed(if on { Color::from_rgb8(0x14, 0x10, 0x0c) } else { Color::from_rgb8(0xe8, 0xe8, 0xea) }, 8.0, None));
    let track = container(row![if on { Space::new().width(Length::Fill).into() } else { gap(0.0, 0.0) }, knob, if on { gap(0.0, 0.0) } else { Space::new().width(Length::Fill).into() }].align_y(iced::Center))
        .padding(4.0)
        .width(Length::Fixed(w))
        .height(Length::Fixed(24.0))
        .style(ui::boxed(if on { ORANGE } else { Pal::mix(p.panel, Color::WHITE, 0.12) }, 12.0, None));
    mouse_area(track).on_press(msg).interaction(mouse::Interaction::Pointer).into()
}

/// Off / Low / High as the mockups' segmented control.
fn seg<'a>(p: &Pal, items: Vec<(String, bool, Msg)>) -> E<'a> {
    let p = *p;
    let mut r = Row::new().spacing(2.0);
    for (l, on, m) in items {
        let c = container(if on { tb(l, 13.0, Color::from_rgb8(0x1b, 0x10, 0x06)) } else { t(l, 13.0, p.muted) })
            .padding(Padding { top: 6.0, right: 14.0, bottom: 6.0, left: 14.0 })
            .style(ui::boxed(if on { ORANGE } else { Color::TRANSPARENT }, 6.0, None));
        r = r.push(mouse_area(c).on_press(m).interaction(mouse::Interaction::Pointer));
    }
    container(r).padding(3.0).style(ui::boxed(Pal::mix(p.bg, Color::BLACK, 0.15), 8.0, None)).into()
}

fn three<'a>(p: &Pal, k: &'static str, cur: u32) -> E<'a> {
    seg(p, ["Off", "Low", "High"].iter().enumerate().map(|(i, l)| (l.to_string(), cur == i as u32, Msg::Level(k, i as u32))).collect())
}

/// A settings row: icon box, title and line, the control on the right.
fn srow<'a>(p: &Pal, icon: &'a str, title: impl Into<String>, sub: impl Into<String>, right: E<'a>, focused: bool, msg: Option<Msg>) -> E<'a> {
    let p = *p;
    let ib = container(t(icon, 15.0, p.muted)).center_x(Length::Fixed(28.0)).center_y(Length::Fixed(28.0)).style(ui::boxed(Pal::mix(p.panel, Color::WHITE, 0.05), 6.0, None));
    let sub: String = sub.into();
    let mut txt = Column::new().spacing(3.0).push(t(title.into(), 14.5, p.text));
    if !sub.is_empty() {
        txt = txt.push(t1(sub, 12.0, p.muted));
    }
    let r = row![ib, txt.width(Length::Fill), right].spacing(14.0).align_y(iced::Center);
    let c = container(r).padding(Padding { top: 12.0, right: 16.0, bottom: 12.0, left: 14.0 }).width(Length::Fill).style(ui::boxed(p.panel, p.r(10.0), if focused { Some((ORANGE, 2.0)) } else { None }));
    match msg {
        Some(m) => mouse_area(c).on_press(m).interaction(mouse::Interaction::Pointer).into(),
        None => c.into(),
    }
}

fn head<'a>(p: &Pal, s: impl Into<String>) -> E<'a> {
    container(tm(s.into(), 13.5, Pal::mix(p.text, p.muted, 0.4))).padding(Padding { top: 10.0, right: 0.0, bottom: 2.0, left: 0.0 }).into()
}

fn big_title<'a>(p: &Pal, parts: Vec<(String, bool)>) -> E<'a> {
    let mut r = Row::new().spacing(10.0).align_y(iced::Center);
    let n = parts.len();
    for (i, (s, strong)) in parts.into_iter().enumerate() {
        r = r.push(text(s).size(sz(52.0)).color(if strong { p.text } else { p.muted }).font(ui::weight(iced::font::Weight::Semibold)));
        if i + 1 < n {
            r = r.push(text("›").size(sz(46.0)).color(p.muted));
        }
    }
    r.into()
}

fn lcd<'a>(f: hearth_g15::lcd::Frame, w: f32, h: f32, glow: bool) -> E<'a> {
    canvas(pics::Screen { frame: f, pick: None, glow }).width(Length::Fixed(w)).height(Length::Fixed(h)).into()
}

fn chip_btn<'a>(p: &Pal, label: impl Into<String>, picked: bool, focused: bool, msg: Msg) -> E<'a> {
    let p = *p;
    let c = container(t(label.into(), 13.0, if picked { ORANGE } else { p.text }))
        .padding(Padding { top: 6.0, right: 13.0, bottom: 6.0, left: 13.0 })
        .style(ui::boxed(if picked { Pal::a(ORANGE, 0.08) } else { Color::TRANSPARENT }, 16.0, Some((if picked || focused { ORANGE } else { p.line }, 1.0))));
    mouse_area(c).on_press(msg).interaction(mouse::Interaction::Pointer).into()
}

// ------------------------------------------------------------------ the frame
pub fn view(a: &App) -> E<'_> {
    let p = a.p;
    let bg = Pal::mix(p.bg, Color::BLACK, 0.15);
    let body: E = match a.page {
        Page::Overview => overview(a),
        Page::GKeys => gkeys(a),
        Page::Screen => screen_page(a),
        Page::Lighting => lighting(a),
        Page::Editor => editor(a),
    };
    let page = container(scrollable(container(body).padding(Padding { top: 22.0, right: 30.0, bottom: 20.0, left: 32.0 })).height(Length::Fill)).width(Length::Fill).height(Length::Fill).style(ui::boxed(bg, 0.0, None));
    let main = row![sidebar(a), page].height(Length::Fill);
    let all = column![title_bar(a), hline(p.line), main, hline(p.line), bottom_bar(a)];
    let base: E = container(all).width(Length::Fill).height(Length::Fill).style(ui::boxed(bg, 0.0, None)).into();
    let base = match &a.dlg {
        Some(d) => ui::dialog(base, dialog(a, d), Some(Msg::CloseDlg)),
        None => base,
    };
    ui::edges(base, Msg::Resize)
}

fn title_bar(a: &App) -> E<'_> {
    let p = a.p;
    let badge = container(text("G").size(12.0).color(Color::WHITE).font(ui::weight(iced::font::Weight::Bold))).center_x(Length::Fixed(20.0)).center_y(Length::Fixed(20.0)).style(ui::boxed(Color::from_rgb8(0xff, 0x6a, 0x1a), 5.0, None));
    let name = row![gap(14.0, 1.0), badge, gap(10.0, 1.0), tm("Logitech G15", 14.0, p.text), gap(8.0, 1.0), t("· Hearth", 12.0, p.muted)].align_y(iced::Center);
    let drag = mouse_area(container(name).width(Length::Fill).center_y(Length::Fixed(36.0))).on_press(Msg::Drag).on_double_click(Msg::Maximize);
    let wbtn = |g: G, m: Msg| -> E<'static> {
        mouse_area(container(glyph(g, p.muted, 10.0, 2.0)).center_x(Length::Fixed(40.0)).center_y(Length::Fixed(36.0))).on_press(m).interaction(mouse::Interaction::Pointer).into()
    };
    container(row![drag, wbtn(G::Minimize, Msg::Minimize), wbtn(G::Maximize, Msg::Maximize), wbtn(G::Close, Msg::Close)].align_y(iced::Center))
        .width(Length::Fill)
        .style(ui::boxed(Pal::mix(p.bg, Color::BLACK, 0.25), 0.0, None))
        .into()
}

fn sidebar(a: &App) -> E<'_> {
    let p = a.p;
    let kb = container(container(Space::new()).width(Length::Fixed(14.0)).height(Length::Fixed(4.0)).style(ui::boxed(ORANGE, 2.0, None)))
        .padding(Padding { top: 0.0, right: 0.0, bottom: 0.0, left: 12.0 })
        .width(Length::Fixed(36.0))
        .height(Length::Fixed(26.0))
        .style(ui::boxed(Pal::mix(p.panel, Color::WHITE, 0.04), 5.0, Some((p.line, 1.0))));
    let plugged = a.plugged();
    let model = a.status.get("model").and_then(|v| v.as_str()).unwrap_or("G15").to_string();
    let dot = container(Space::new()).width(Length::Fixed(7.0)).height(Length::Fixed(7.0)).style(ui::boxed(if plugged { Color::from_rgb8(0x3f, 0xc4, 0x6a) } else { p.muted }, 4.0, None));
    let who = row![kb, column![tb(model, 14.0, p.text), row![dot, t(if plugged { "Plugged in" } else { "Not plugged in" }, 12.0, p.muted)].spacing(6.0).align_y(iced::Center)].spacing(2.0)].spacing(12.0).align_y(iced::Center);
    let mut nav = Column::new().spacing(4.0);
    for (pg, icon, label) in [(Page::Overview, "◉", "Overview"), (Page::GKeys, "G", "G-keys"), (Page::Screen, "▭", "Keyboard screen"), (Page::Lighting, "☀", "Lighting")] {
        let here = a.page == pg || (pg == Page::Screen && a.page == Page::Editor);
        let mark = container(Space::new()).width(Length::Fixed(3.0)).height(Length::Fixed(18.0)).style(ui::boxed(if here { ORANGE } else { Color::TRANSPARENT }, 2.0, None));
        let r = row![mark, gap(10.0, 1.0), container(t(icon, 13.0, p.muted)).width(Length::Fixed(22.0)), t(label, 14.5, p.text)].align_y(iced::Center);
        let c = container(r).padding(Padding { top: 8.0, right: 10.0, bottom: 8.0, left: 0.0 }).width(Length::Fill).style(ui::boxed(if here { Pal::mix(p.panel, Color::WHITE, 0.04) } else { Color::TRANSPARENT }, 8.0, None));
        nav = nav.push(mouse_area(c).on_press(Msg::Go(pg)).interaction(mouse::Interaction::Pointer));
    }
    let foot = t1("Settings are saved as you change them.\nAlso in Settings › Bluetooth & devices.", 11.5, p.muted);
    container(column![container(who).padding(Padding { top: 16.0, right: 0.0, bottom: 18.0, left: 18.0 }), container(nav).padding(Padding { top: 0.0, right: 12.0, bottom: 0.0, left: 10.0 }), Space::new().height(Length::Fill), container(foot).padding(18.0)])
        .width(Length::Fixed(sz(330.0)))
        .height(Length::Fill)
        .style(ui::boxed(Pal::mix(p.bg, Color::BLACK, 0.05), 0.0, None))
        .into()
}

fn hint<'a>(p: &Pal, b: &'a str, what: &'a str) -> E<'a> {
    let c = container(t(b, 9.5, p.text)).center_x(Length::Fixed(20.0)).center_y(Length::Fixed(20.0)).style(ui::boxed(Color::TRANSPARENT, 10.0, Some((p.muted, 1.0))));
    row![c, t(what, 13.0, p.text)].spacing(7.0).align_y(iced::Center).into()
}

fn bottom_bar(a: &App) -> E<'_> {
    let p = a.p;
    let mut l = Row::new().spacing(22.0).align_y(iced::Center);
    let right: E = match a.page {
        Page::Editor => t("Arrows: move one dot", 13.0, p.text).into(),
        _ => row![hint(&p, "LB", "Previous / next page")].into(),
    };
    match a.page {
        Page::Editor => {
            l = l.push(hint(&p, "A", "Pick up / put down")).push(hint(&p, "B", "Back")).push(hint(&p, "X", "Delete")).push(hint(&p, "Y", "Add"));
        }
        Page::GKeys => {
            l = l.push(hint(&p, "A", "Select")).push(hint(&p, "B", "Back")).push(hint(&p, "X", "Test it")).push(hint(&p, "Y", "Record"));
        }
        Page::Screen => {
            l = l.push(hint(&p, "A", "Select")).push(hint(&p, "B", "Back")).push(hint(&p, "X", "Show on the keyboard"));
        }
        _ => {
            l = l.push(hint(&p, "A", "Select")).push(hint(&p, "B", "Back"));
        }
    }
    container(row![l.width(Length::Fill), right].align_y(iced::Center)).padding(Padding { top: 0.0, right: 20.0, bottom: 0.0, left: 20.0 }).center_y(Length::Fixed(36.0)).width(Length::Fill).style(ui::boxed(Pal::mix(p.bg, Color::BLACK, 0.25), 0.0, None)).into()
}

fn note<'a>(a: &'a App) -> E<'a> {
    if a.note.is_empty() {
        gap(1.0, 1.0)
    } else {
        container(t(a.note.clone(), 13.0, ORANGE)).padding(Padding { top: 6.0, right: 0.0, bottom: 0.0, left: 2.0 }).into()
    }
}

fn clock_now() -> String {
    let t = hearth_ui::time::local(hearth_ui::time::now());
    format!("{:02}:{:02}", t.hour, t.min)
}

// ------------------------------------------------------------------ 1. Overview (g15-1)
fn overview(a: &App) -> E<'_> {
    let p = a.p;
    let set = config::set_in_use(&a.cfg);
    let pic = canvas(pics::Keyboard { p, set, recording: a.recording, lit: false, time: clock_now() }).width(Length::Fill).height(Length::Fixed(sz(360.0)));
    let gk = a.status.get("gkeys").and_then(|v| v.as_u64()).unwrap_or(18);
    let model = a.status.get("model").and_then(|v| v.as_str()).unwrap_or("G15");
    let line = if a.plugged() { format!("Plugged in · {}, {gk} G-keys", if model == "G15" { "first model" } else { model }) } else { "Not plugged in · plug it in to use it".into() };
    let dot = container(Space::new()).width(Length::Fixed(7.0)).height(Length::Fixed(7.0)).style(ui::boxed(if a.plugged() { Color::from_rgb8(0x3f, 0xc4, 0x6a) } else { p.muted }, 4.0, None));
    let fact = |k: &str, v: String| -> E<'static> {
        column![row![t(k.to_string(), 14.0, p.muted).width(Length::Fill), t(v, 14.0, p.text)], hline(p.line)].spacing(8.0).into()
    };
    let names: Vec<String> = a.pages_on().iter().map(|x| page_title(a, x)).collect();
    let keys_light = ["Off", "Low", "High"][level(a, "kbdLight", 2).min(2) as usize].to_string();
    let info = column![
        tb("Logitech G15 keyboard", 20.0, p.text),
        row![dot, t(line, 13.0, p.muted)].spacing(7.0).align_y(iced::Center),
        gap(1.0, 8.0),
        fact("Key set", format!("M{set} · {}", a.set_name(set))),
        fact("G-keys set up", format!("{} of {gk}", config::keys_set(&a.cfg, set))),
        fact("Screen", names.join(", ")),
        fact("Keys light", keys_light),
    ]
    .spacing(8.0)
    .width(Length::Fixed(380.0));
    let top = card(&p, row![pic, info].spacing(24.0).align_y(iced::Center)).padding(18.0).width(Length::Fill);
    let mut sets = Row::new().spacing(10.0);
    for n in 1..=3u8 {
        let k = config::keys_set(&a.cfg, n);
        let what: Vec<String> = (1..=18u8).filter_map(|g| a.action(n, g).map(|_| a.says(n, g).to_lowercase())).take(4).collect();
        let sub = if k == 0 { "Nothing set up yet".to_string() } else { format!("{k} keys · {}", what.join(", ")) };
        let inuse = n == set;
        let foot: E = if inuse { container(t("In use", 13.0, ORANGE)).padding(Padding { top: 5.0, right: 12.0, bottom: 5.0, left: 12.0 }).style(ui::boxed(Color::TRANSPARENT, 14.0, Some((ORANGE, 1.0)))).into() } else { container(t(format!("Press M{n} to use"), 13.0, p.text)).padding(Padding { top: 5.0, right: 12.0, bottom: 5.0, left: 12.0 }).style(ui::boxed(Color::TRANSPARENT, 14.0, Some((p.line, 1.0)))).into() };
        let inner = column![t(format!("M{n} · {}", a.set_name(n)), 16.0, p.text), t1(sub, 12.5, p.muted), gap(1.0, 6.0), foot].spacing(10.0);
        let c = card_on(&p, container(inner).padding(16.0).width(Length::Fill), inuse, on(a, &format!("set{n}")));
        sets = sets.push(mouse_area(container(c).width(Length::FillPortion(1))).on_press(Msg::UseSet(n)).interaction(mouse::Interaction::Pointer));
    }
    let g = a.games();
    let rule = g.iter().enumerate().find(|(_, x)| !x.is_empty());
    let sw_sub = match rule {
        Some((i, game)) => format!("Starts M{} when {game} opens, back to M{set} when it closes", i + 1),
        None => "Pick a game for a key set: press → here".into(),
    };
    column![
        big_title(&p, vec![("Your G15".into(), true)]),
        gap(1.0, 14.0),
        top,
        head(&p, "Key sets"),
        sets,
        gap(1.0, 2.0),
        srow(&p, "⟳", "Switch key sets for each game", sw_sub, switch(&p, flag(a, "switchSets", true), Msg::Flag("switchSets", !flag(a, "switchSets", true))), on(a, "switch"), Some(Msg::GamesOpen)),
        srow(&p, "●", "Record a macro with MR", "Press MR, type, then press the G-key to keep it on", switch(&p, flag(a, "mrRecord", true), Msg::Flag("mrRecord", !flag(a, "mrRecord", true))), on(a, "mr"), None),
        note(a),
    ]
    .spacing(10.0)
    .into()
}

pub fn page_title(a: &App, id: &str) -> String {
    match id {
        "clock" => "Clock".into(),
        "system" => "System".into(),
        "music" => "Now playing".into(),
        "alerts" => "Alerts".into(),
        _ => a.own().iter().find(|x| s(x, "id") == id).map(|x| s(x, "name")).unwrap_or_else(|| id.to_string()),
    }
}

// ------------------------------------------------------------------ 2. G-keys (g15-2)
pub const KINDS: &[(&str, &str)] = &[("keys", "Keys"), ("text", "Type text"), ("macro", "Macro"), ("app", "Open an app"), ("hearth", "Hearth action"), ("none", "Nothing")];

fn gkeys(a: &App) -> E<'_> {
    let p = a.p;
    let set = a.set;
    let mut chips = Row::new().spacing(8.0).align_y(iced::Center);
    for n in 1..=3u8 {
        chips = chips.push(chip_btn(&p, format!("M{n} · {}", a.set_name(n)), n == set, on(a, "sets") && n == set, Msg::ViewSet(n)));
    }
    chips = chips.push(chip_btn(&p, "+ Rename", false, false, Msg::RenameOpen));
    // The keys drawn big.
    let mk = |l: String, lit: bool| -> E<'static> {
        container(t(l, 14.0, if lit { ORANGE } else { p.muted })).center_x(Length::Fixed(50.0)).center_y(Length::Fixed(26.0)).style(ui::boxed(Pal::mix(p.panel, Color::WHITE, 0.05), 5.0, Some((if lit { ORANGE } else { p.line }, if lit { 1.5 } else { 1.0 })))).into()
    };
    let inuse = config::set_in_use(&a.cfg);
    let mrow = row![mk("M1".into(), set == 1), mk("M2".into(), set == 2), mk("M3".into(), set == 3), mk("MR".into(), a.recording)].spacing(8.0);
    let _ = inuse;
    let mut grid = Column::new().spacing(8.0);
    for r in 0..6u8 {
        let mut rw = Row::new().spacing(8.0);
        for c in 0..3u8 {
            let g = r * 3 + c + 1;
            let set_up = a.action(set, g).is_some();
            let picked = g == a.g;
            let focused = on(a, &format!("g{g}"));
            let (bg, fg, line) = if picked { (Color::from_rgb8(0xff, 0x7a, 0x2f), Color::from_rgb8(0x1b, 0x10, 0x06), (Color::from_rgb8(0xff, 0xb0, 0x7a), 2.0)) } else if set_up { (Pal::mix(p.panel, Color::WHITE, 0.05), ORANGE, (ORANGE, 1.5)) } else { (Pal::mix(p.panel, Color::WHITE, 0.05), ORANGE, (p.line, 1.0)) };
            let line = if focused && !picked { (Color::WHITE, 2.0) } else { line };
            let k = container(text(format!("G{g}")).size(if picked { 19.0 } else { 14.0 }).color(fg).font(ui::weight(if picked { iced::font::Weight::Bold } else { iced::font::Weight::Semibold })))
                .center_x(Length::Fixed(sz(110.0)))
                .center_y(Length::Fixed(sz(110.0)))
                .style(ui::boxed(bg, 6.0, Some(line)));
            rw = rw.push(mouse_area(k).on_press(Msg::PickG(g)).interaction(mouse::Interaction::Pointer));
        }
        grid = grid.push(rw);
    }
    let left = card(&p, column![mrow, gap(1.0, 4.0), grid, gap(1.0, 20.0), t1("Click a G-key, or press it on the keyboard, to set what it does.", 13.0, p.muted)].spacing(10.0))
        .padding(Padding { top: 22.0, right: 24.0, bottom: 22.0, left: 40.0 })
        .width(Length::Fixed(sz(560.0)));

    // The picked key.
    let g = a.g;
    let badge = container(tb(format!("G{g}"), 15.0, Color::from_rgb8(0x1b, 0x10, 0x06))).center_x(Length::Fixed(30.0)).center_y(Length::Fixed(24.0)).style(ui::boxed(Color::from_rgb8(0xff, 0x7a, 0x2f), 4.0, None));
    let says = a.says(set, g);
    let headr = row![badge, gap(6.0, 1.0), tb(says.clone(), 18.0, p.text).width(Length::Fill), btn(&p, "Test it", Kind::Normal, on(a, "test"), a.action(set, g).map(|_| Msg::TestIt)), btn(&p, "Clear", Kind::Normal, on(a, "clear"), a.action(set, g).map(|_| Msg::ClearKey))]
        .spacing(8.0)
        .align_y(iced::Center);
    let kind = a.kind();
    let kseg = seg(&p, KINDS.iter().map(|(k, l)| (l.to_string(), kind == *k, Msg::Kind(*k))).collect());
    let kseg: E = if on(a, "kind") { ui::ring(p, true, 8.0, kseg) } else { kseg };
    let act = a.action(set, g);
    let what: E = match kind.as_str() {
        "text" => {
            let field = text_input("What it types", &a.text).on_input(Msg::Text).on_submit(Msg::TextSave).padding(8.0).size(14.0);
            row![container(t("⌨", 16.0, p.muted)).width(Length::Fixed(30.0)), field, btn(&p, "Keep", Kind::Primary, false, Some(Msg::TextSave))].spacing(10.0).align_y(iced::Center).into()
        }
        "none" => row![t1("This key does nothing in this set.", 13.0, p.muted)].into(),
        _ => {
            let (icon, title, sub) = match kind.as_str() {
                "keys" => ("⌨", if a.capturing { "Press the keys now…".to_string() } else { act.as_ref().map(|v| s(v, "keys")).unwrap_or_default() }, "Pressed together, like on the keyboard".to_string()),
                "macro" => ("●", format!("A recorded macro · {} keys", act.as_ref().and_then(|v| v.get("steps")).and_then(|x| x.as_array()).map(|x| x.iter().filter(|s| s.get("down").and_then(|d| d.as_bool()).unwrap_or(true)).count()).unwrap_or(0)), "Below: its keys, record it again".to_string()),
                "app" => {
                    let d = act.as_ref().map(|v| s(v, "desktop")).unwrap_or_default();
                    ("▢", crate::app_name(&d).map(|n| format!("Open {n}")).unwrap_or_else(|| says.clone()), d)
                }
                _ => {
                    let h = act.as_ref().map(|v| s(v, "action")).unwrap_or_default();
                    let (title, sub) = hearth_words(&h);
                    ("◉", title, sub)
                }
            };
            row![container(t(icon, 16.0, p.muted)).width(Length::Fixed(30.0)), column![t(title, 15.0, p.text), t1(sub, 12.0, p.muted)].spacing(3.0).width(Length::Fill), btn(&p, "Change…", Kind::Normal, on(a, "change"), Some(Msg::Change))].spacing(10.0).align_y(iced::Center).into()
        }
    };
    let what = container(what).padding(14.0).width(Length::Fill).style(ui::boxed(Pal::mix(p.panel, Color::WHITE, 0.03), 8.0, None));
    let key_card = card(&p, column![headr, gap(1.0, 4.0), t("What it does", 14.0, p.muted), kseg, what].spacing(10.0)).padding(18.0).width(Length::Fill);

    // A macro of this set: its keys as chips, with the waits between them.
    let macro_card: E = match a.macro_key().and_then(|mg| a.action(set, mg).map(|v| (mg, v))) {
        Some((mg, v)) => {
            let steps = v.get("steps").and_then(|x| x.as_array()).cloned().unwrap_or_default();
            let mut chips = Row::new().spacing(6.0).align_y(iced::Center);
            for (i, (label, wait)) in macro_chips(&steps).into_iter().enumerate() {
                if i >= 11 {
                    chips = chips.push(t("…", 13.0, p.muted));
                    break;
                }
                if let Some(w) = wait {
                    chips = chips.push(t(w, 11.5, p.muted));
                }
                chips = chips.push(container(text(label).size(13.0).color(p.text).font(iced::Font::MONOSPACE)).padding(Padding { top: 5.0, right: 9.0, bottom: 5.0, left: 9.0 }).style(ui::boxed(Pal::mix(p.panel, Color::WHITE, 0.04), 4.0, Some((p.line, 1.0)))));
            }
            let o = actions::macro_opts(&v);
            let wait = match o.wait {
                None => "As recorded".to_string(),
                Some(0) => "None".to_string(),
                Some(ms) => format!("{:.1} s", ms as f32 / 1000.0),
            };
            let rep = if o.repeat <= 1 { "Once".to_string() } else { format!("{} times", o.repeat) };
            let held = if o.while_held { "Repeat" } else { "Play once" };
            let opt = |k: &'static str, l: &'static str, v: String| -> E<'static> {
                mouse_area(row![t(format!("{l}: "), 13.0, p.muted), tb(v, 13.0, p.text), t(" ▾", 11.0, p.text)]).on_press(Msg::MacroOpt(k)).interaction(mouse::Interaction::Pointer).into()
            };
            let name = s(&v, "name");
            let rec: E = if a.recording { btn(&p, "■ Stop", Kind::Primary, on(a, "record"), Some(Msg::StopRecord)) } else { btn(&p, "● Record again", Kind::Primary, on(a, "record"), Some(Msg::Record)) };
            let opts: E = row![opt("wait", "Wait between keys", wait), opt("repeat", "Repeat", rep), opt("held", "While held", held.to_string())].spacing(24.0).into();
            let opts = if on(a, "opts") { ui::ring(p, true, 6.0, opts) } else { opts };
            card(&p, column![row![tb(format!("G{mg} · Macro: \"{}\"", if name.is_empty() { "recorded".into() } else { name }), 16.0, p.text).width(Length::Fill), rec].align_y(iced::Center), chips, opts].spacing(12.0)).padding(18.0).width(Length::Fill).into()
        }
        None => card(&p, row![column![tb("Macros", 16.0, p.text), t1("Press MR on the keyboard, type, then press the G-key to keep it on. Or pick Macro above.", 12.5, p.muted)].spacing(4.0).width(Length::Fill), btn(&p, if a.recording { "■ Stop" } else { "● Record" }, Kind::Primary, false, Some(if a.recording { Msg::StopRecord } else { Msg::Record }))].align_y(iced::Center)).padding(18.0).width(Length::Fill).into(),
    };
    // All 18 keys.
    let mut all = Column::new().spacing(8.0);
    for r in 0..6u8 {
        let mut rw = Row::new().spacing(8.0);
        for c in 0..3u8 {
            let g = r * 3 + c + 1;
            let w = a.says(set, g);
            let unset = w == "Not set";
            let cell = container(row![container(tb(format!("G{g}"), 14.0, ORANGE)).width(Length::Fixed(44.0)), t(w, 14.0, if unset { p.muted } else { p.text })].align_y(iced::Center))
                .padding(Padding { top: 8.0, right: 12.0, bottom: 8.0, left: 12.0 })
                .width(Length::FillPortion(1))
                .style(ui::boxed(p.panel, 6.0, if g == a.g { Some((ORANGE, 1.5)) } else { None }));
            rw = rw.push(mouse_area(cell).on_press(Msg::PickG(g)).interaction(mouse::Interaction::Pointer));
        }
        all = all.push(rw);
    }
    let right = column![key_card, macro_card, head(&p, format!("All keys in M{set}")), all, note(a)].spacing(10.0).width(Length::Fill);
    column![big_title(&p, vec![("G-keys".into(), false), (format!("M{set} · {}", a.set_name(set)), true)]), gap(1.0, 6.0), chips, gap(1.0, 8.0), row![left, right].spacing(16.0)].spacing(6.0).into()
}

/// A Hearth action's row ("Take a screenshot · pick an area", "Capture").
fn hearth_words(h: &str) -> (String, String) {
    match h {
        "screenshot-area" => ("Take a screenshot · pick an area".into(), "Capture".into()),
        "screenshot" => ("Take a screenshot · the whole screen".into(), "Capture".into()),
        "apps" => ("Open the app menu".into(), "Hearth".into()),
        "quick" => ("Open Quick Access".into(), "Hearth".into()),
        "settings" => ("Open Settings".into(), "Hearth Settings".into()),
        "files" => ("Open Hearth Files".into(), "Hearth Files".into()),
        "tasks" => ("Open Task Manager".into(), "Task Manager".into()),
        "lock" => ("Lock the screen".into(), "Hearth".into()),
        "power" => ("Power menu".into(), "Sleep, restart, shut down".into()),
        "volume-up" | "volume-down" | "mute" => (actions::HEARTH_ACTIONS.iter().find(|(k, _)| *k == h).map(|(_, l)| l.to_string()).unwrap_or_default(), "Sound".into()),
        "next-page" => ("Next page on the keyboard's screen".into(), "Logitech G15".into()),
        _ => ("Pick a Hearth action".into(), String::new()),
    }
}

/// A macro's steps as the chips the mockup shows: keys pressed together ("Ctrl+End"), each with the wait before it.
pub fn macro_chips(steps: &[Value]) -> Vec<(String, Option<String>)> {
    let mut out = Vec::new();
    let mut held: Vec<String> = Vec::new();
    let mut wait = 0u64;
    for st in steps {
        let k = s(st, "key");
        let down = st.get("down").and_then(|d| d.as_bool()).unwrap_or(true);
        wait += st.get("ms").and_then(|m| m.as_u64()).unwrap_or(0);
        let modifier = matches!(k.as_str(), "Ctrl" | "Shift" | "Alt" | "Super" | "RightCtrl" | "RightShift" | "AltGr");
        if down && modifier {
            held.push(k);
        } else if down {
            let shown = if held.iter().all(|h| h == "Shift") && k.chars().count() == 1 {
                if held.is_empty() { k.to_lowercase() } else { k.clone() }
            } else if held.is_empty() {
                if k.chars().count() == 1 { k.to_lowercase() } else { k.clone() }
            } else {
                format!("{}+{}", held.join("+"), k)
            };
            let w = if out.is_empty() || wait < 100 { None } else { Some(format!("{:.1} s", wait as f32 / 1000.0)) };
            out.push((shown, w));
            wait = 0;
        } else if modifier {
            held.retain(|h| *h != k);
        }
    }
    out
}

// ------------------------------------------------------------------ 3. Keyboard screen (g15-3)
fn screen_page(a: &App) -> E<'_> {
    let p = a.p;
    let now = a.now.clone().unwrap_or_default();
    let big = column![lcd(now, sz(660.0), sz(195.0), true), t1("What's on the screen now · the keys under it switch pages", 12.5, p.muted)].spacing(10.0);
    let idle = a.cfg.get("lcdIdleOff").and_then(|v| v.as_u64()).unwrap_or(0);
    let idle_txt = if idle == 0 { "Never".to_string() } else { format!("{idle} minutes idle") };
    let ctl = |l: &str, w: E<'static>, id: &str| -> E<'static> {
        let r: E = row![t(l.to_string(), 14.5, p.text).width(Length::Fill), w].align_y(iced::Center).into();
        column![if on(a, id) { ui::ring(p, true, 6.0, r) } else { r }, hline(p.line)].spacing(10.0).into()
    };
    let idle_pick: E = mouse_area(row![tb(idle_txt, 14.0, p.text), t(" ▾", 12.0, p.text)]).on_press(Msg::Picked("idle", "@open".into())).interaction(mouse::Interaction::Pointer).into();
    let c = level(a, "lcdContrast", 1);
    let controls = column![
        ctl("Brightness", three(&p, "lcdLight", level(a, "lcdLight", 2)), "bright"),
        ctl("Contrast", seg(&p, ["Low", "Medium", "High"].iter().enumerate().map(|(i, l)| (l.to_string(), c == i as u32, Msg::Level("lcdContrast", i as u32))).collect()), "contrast"),
        ctl("Turn the light off after", idle_pick, "idle"),
    ]
    .spacing(14.0)
    .width(Length::Fill);
    let top = card(&p, row![big, controls].spacing(28.0).align_y(iced::Center)).padding(20.0).width(Length::Fill);

    // The pages, in their order: on ones first (as saved), then the off ones.
    let on_pages = a.pages_on();
    let mut cards = Row::new().spacing(12.0);
    for id in a.all_pages().into_iter().filter(|x| hearth_g15::lcd::screen::BUILT_IN.contains(&x.as_str())) {
        let enabled = on_pages.contains(&id);
        let sub = match id.as_str() {
            "clock" => "Time, date, the weather",
            "system" => "CPU, graphics card, memory, temperatures",
            "music" => "Any music or video app",
            _ => "Hearth's messages pop up for 5 s",
        };
        let focused = on(a, &format!("page-{id}"));
        let over = a.over.as_deref() == Some(id.as_str()) && a.dragging.is_some();
        let inner = column![lcd(a.shot(&id), sz(330.0), sz(102.0), false), row![tb(page_title(a, &id), 14.5, p.text).width(Length::Fill), switch(&p, enabled, Msg::TogglePage(id.clone()))].align_y(iced::Center), t1(sub, 12.5, p.muted)].spacing(10.0);
        let c = card_on(&p, container(inner).padding(14.0).width(Length::Fill), focused || over, false);
        cards = cards.push(
            mouse_area(container(c).width(Length::FillPortion(1)))
                .on_press(Msg::DragPage(id.clone()))
                .on_enter(Msg::OverPage(id.clone()))
                .on_release(Msg::DropPage)
                .interaction(if a.dragging.is_some() { mouse::Interaction::Grabbing } else { mouse::Interaction::Grab }),
        );
    }
    // Your own screens.
    let mut own = Row::new().spacing(12.0);
    for o in a.own() {
        let id = s(&o, "id");
        let enabled = on_pages.contains(&id);
        let inner = column![lcd(a.shot(&id), sz(340.0), sz(105.0), false), row![tb(s(&o, "name"), 14.5, p.text).width(Length::Fill), btn(&p, "Edit", Kind::Normal, false, Some(Msg::Edit(id.clone()))), switch(&p, enabled, Msg::TogglePage(id.clone()))].spacing(10.0).align_y(iced::Center)].spacing(12.0);
        own = own.push(container(card_on(&p, container(inner).padding(14.0), false, on(a, &format!("own-{id}")))).width(Length::Fixed(sz(390.0))));
    }
    let dashed = container(column![text("+").size(26.0).color(ORANGE), tb("Make your own screen", 14.5, p.text), t1("Put anything on it: values, graphs, text, a picture", 12.5, p.muted)].spacing(6.0).align_x(iced::Center))
        .center_x(Length::Fixed(sz(390.0)))
        .center_y(Length::Fixed(sz(200.0)))
        .style(ui::boxed(Color::TRANSPARENT, 12.0, Some((if on(a, "new") { ORANGE } else { Pal::mix(p.line, Color::WHITE, 0.15) }, 1.5))));
    own = own.push(mouse_area(dashed).on_press(Msg::NewScreen).interaction(mouse::Interaction::Pointer));
    column![
        big_title(&p, vec![("Keyboard screen".into(), true)]),
        gap(1.0, 14.0),
        top,
        head(&p, "Pages · drag to reorder"),
        cards,
        head(&p, "Your own screens"),
        own,
        gap(1.0, 4.0),
        srow(&p, "◈", "Show the game's own screen when it has one", "Games made for the G15's screen take it over while they run", switch(&p, flag(a, "gameScreens", true), Msg::Flag("gameScreens", !flag(a, "gameScreens", true))), on(a, "gamescreen"), None),
        srow(&p, "◷", "Change page by itself", format!("Every {} seconds", a.cfg.get("autoPageSecs").and_then(|v| v.as_u64()).unwrap_or(10)), switch(&p, flag(a, "autoPage", false), Msg::Flag("autoPage", !flag(a, "autoPage", false))), on(a, "autopage"), None),
        note(a),
    ]
    .spacing(10.0)
    .into()
}

// ------------------------------------------------------------------ 4. Lighting (g15-4)
fn lighting(a: &App) -> E<'_> {
    let p = a.p;
    let pic = canvas(pics::Keyboard { p, set: config::set_in_use(&a.cfg), recording: a.recording, lit: true, time: clock_now() }).width(Length::Fill).height(Length::Fixed(sz(400.0)));
    let from = config::text(&a.cfg, "nightFrom", "21:00").to_string();
    let to = config::text(&a.cfg, "nightTo", "07:00").to_string();
    let ampm = |hm: &str| -> String {
        let m = config::minutes(hm).unwrap_or(0);
        let (h, mm) = (m / 60, m % 60);
        format!("{}:{:02} {}", if h % 12 == 0 { 12 } else { h % 12 }, mm, if h < 12 { "AM" } else { "PM" })
    };
    column![
        big_title(&p, vec![("Lighting".into(), true)]),
        gap(1.0, 14.0),
        card(&p, container(pic).padding(Padding { top: 14.0, right: 120.0, bottom: 14.0, left: 120.0 })).width(Length::Fill),
        head(&p, "Keys"),
        srow(&p, "☀", "Key light", "The orange light under the keys · the light button on the keyboard changes it too", three(&p, "kbdLight", level(a, "kbdLight", 2)), on(a, "kbd"), None),
        srow(&p, "▭", "Screen light", "Also on the Keyboard screen page", three(&p, "lcdLight", level(a, "lcdLight", 2)), on(a, "lcd"), None),
        head(&p, "When"),
        srow(&p, "☾", "Turn the lights off when the PC sleeps or locks", "They come back the way you had them", switch(&p, flag(a, "lightsOffWhenLocked", false), Msg::Flag("lightsOffWhenLocked", !flag(a, "lightsOffWhenLocked", false))), on(a, "locked"), None),
        srow(&p, "◖", "Dim the keys with Night light", format!("Key light goes to Low from {} to {}", ampm(&from), ampm(&to)), switch(&p, flag(a, "nightDim", false), Msg::Flag("nightDim", !flag(a, "nightDim", false))), on(a, "night"), None),
        srow(&p, "⊞", "Light up the key set's M-key", "M1, M2 or M3 shows which set is in use · MR lights while recording", switch(&p, flag(a, "mLights", true), Msg::Flag("mLights", !flag(a, "mLights", true))), on(a, "mlights"), None),
        note(a),
    ]
    .spacing(10.0)
    .into()
}

// ------------------------------------------------------------------ 6. Make your own screen (g15-6)
pub const ADD: &[(&str, &str, &str)] = &[("text", "T", "Text"), ("value", "#", "A value"), ("bar", "▭", "Bar"), ("graph", "~", "Graph"), ("clock", "◷", "Clock"), ("picture", "▣", "Picture"), ("box", "◻", "Line or box")];
/// A clock's choices, and a picture's darkness (how dark a dot has to be: lcd/picture.rs).
pub const CLOCK: &[(&str, &str)] = &[("24h", "24:00"), ("12h", "12:00"), ("seconds", "Seconds"), ("date", "Date"), ("weekday", "Day")];
pub const DARK: &[(u64, &str)] = &[(96, "Light"), (128, "Normal"), (160, "Dark")];

/// The editor's own small pieces (g15-6's right panel is set smaller than the other pages' rows).
fn eseg<'a>(p: &Pal, items: Vec<(String, bool, Msg)>) -> E<'a> {
    let p = *p;
    let mut r = Row::new().spacing(2.0);
    for (l, on, m) in items {
        let c = container(if on { tb(l, 11.5, Color::from_rgb8(0x1b, 0x10, 0x06)) } else { t(l, 11.5, p.muted) })
            .padding(Padding { top: 4.0, right: 7.0, bottom: 4.0, left: 7.0 })
            .style(ui::boxed(if on { ORANGE } else { Color::TRANSPARENT }, 6.0, None));
        r = r.push(mouse_area(c).on_press(m).interaction(mouse::Interaction::Pointer));
    }
    container(r).padding(3.0).style(ui::boxed(Pal::mix(p.bg, Color::BLACK, 0.15), 8.0, None)).into()
}

/// The orange button (Done), as the mockup draws it.
fn orange_btn<'a>(label: &'a str, focused: bool, msg: Msg) -> E<'a> {
    let c = container(tb(label, 14.0, Color::from_rgb8(0x1b, 0x10, 0x06)))
        .padding(Padding { top: 8.0, right: 18.0, bottom: 8.0, left: 18.0 })
        .style(ui::boxed(ORANGE, 8.0, if focused { Some((Color::WHITE, 2.0)) } else { None }));
    mouse_area(c).on_press(msg).interaction(mouse::Interaction::Pointer).into()
}

fn editor(a: &App) -> E<'_> {
    let p = a.p;
    let scr = a.cur_screen().unwrap_or(json!({}));
    let els = a.elements();
    let pick = a.el.and_then(|i| els.get(i));

    // Add: press one and let go on the screen to put it there (or just click it).
    let mut add = Column::new().spacing(8.0).push(t("Add", 14.0, p.muted));
    for (k, icon, l) in ADD {
        let focused = on(a, &format!("add-{k}"));
        let picked = pick.map(|e| s(e, "type") == *k || (*k == "box" && s(e, "type") == "line")).unwrap_or(false);
        let ic: E = match *k {
            // Drawn: a wave, the red alarm clock and a framed picture, as the mockup has them.
            "graph" | "clock" | "picture" => canvas(AddIcon(*k)).width(Length::Fixed(14.0)).height(Length::Fixed(14.0)).into(),
            _ => t(*icon, 13.0, p.muted).into(),
        };
        let c = container(row![container(ic).width(Length::Fixed(24.0)), t(*l, 13.5, p.text)].align_y(iced::Center))
            .padding(Padding { top: 10.0, right: 8.0, bottom: 10.0, left: 10.0 })
            .width(Length::Fill)
            .style(ui::boxed(Pal::mix(p.panel, Color::WHITE, 0.04), 8.0, if focused || picked { Some((ORANGE, 2.0)) } else { None }));
        add = add.push(mouse_area(c).on_press(Msg::AddGrab(*k)).interaction(if a.adding == Some(*k) { mouse::Interaction::Grabbing } else { mouse::Interaction::Grab }));
    }
    add = add.push(gap(1.0, 4.0)).push(t("Drag onto the screen. Pictures are turned into the screen's dots for you.", 12.0, p.muted));
    let add = card(&p, add).padding(14.0).width(Length::Fixed(sz(184.0)));

    let cv = editor_canvas(a);
    let centre = column![container(cv).center_x(Length::Fill), container(t1("160 × 43 dots, exactly as the keyboard shows it · arrows nudge one dot · drag a corner to resize", 12.0, p.muted)).center_x(Length::Fill)].spacing(10.0);
    let centre = card(&p, centre).padding(Padding { top: 14.0, right: 12.0, bottom: 14.0, left: 12.0 }).width(Length::Fill);
    let mut chips = Row::new().spacing(8.0);
    for (i, e) in els.iter().enumerate() {
        chips = chips.push(chip_btn(&p, el_name(e), Some(i) == a.el, on(a, &format!("el{i}")), Msg::PickEl(i)));
    }
    let middle = column![centre, head(&p, "On this screen"), chips.wrap(), note(a)].spacing(10.0).width(Length::Fill);

    // The picked element's settings, then the screen's: a label on the left, its choice on the right, a line under.
    let mut props = Column::new().spacing(10.0);
    let prow = |id: &str, l: &str, w: E<'static>| -> E<'static> {
        let r: E = row![t1(l.to_string(), 12.0, p.muted).width(Length::Fill), w].spacing(6.0).align_y(iced::Center).into();
        let r = if on(a, id) { ui::ring(p, true, 6.0, r) } else { r };
        column![r, hline(p.line)].spacing(8.0).into()
    };
    let dd = |what: &'static str, label: String| -> E<'static> {
        mouse_area(row![tb(label, 12.5, p.text), t(" ▾", 10.5, p.text)].align_y(iced::Center)).on_press(Msg::Picked(what, String::from("@open"))).interaction(mouse::Interaction::Pointer).into()
    };
    if let Some(e) = pick {
        let ty = s(e, "type");
        props = props.push(tb(match ty.as_str() {
            "text" => "Text",
            "value" => "A value",
            "bar" => "Bar",
            "graph" => "Graph",
            "clock" => "Clock",
            "picture" => "Picture",
            "line" => "Line",
            "box" => "Box",
            "tabs" => "Page names",
            _ => "Element",
        }, 14.0, p.text));
        if matches!(ty.as_str(), "value" | "bar" | "graph") {
            props = props.push(prow("p-value", "Shows", dd("value", value_name(&s(e, "value")))));
        }
        if ty == "text" {
            props = props.push(column![t("Text (a value goes in { }: {cpu.temp}°)", 12.5, p.muted), text_input("Text", &s(e, "text")).on_input(Msg::ElText).padding(7.0).size(13.0)].spacing(6.0));
        }
        if ty == "graph" {
            let span = e.get("span").and_then(|x| x.as_i64()).unwrap_or(300);
            props = props.push(prow("p-span", "Time across", eseg(&p, [("1 min", 60), ("5 min", 300), ("1 hr", 3600)].iter().map(|(l, v)| (l.to_string(), span == *v, Msg::ElSet("span", json!(v)))).collect())));
        }
        if matches!(ty.as_str(), "bar" | "graph") {
            let unit = hearth_g15::lcd::screen::unit_of(&s(e, "value")).trim().to_string();
            let r = format!("{}{u} – {}{u}", e.get("min").and_then(|x| x.as_i64()).unwrap_or(0), e.get("max").and_then(|x| x.as_i64()).unwrap_or(100), u = unit);
            props = props.push(prow("p-range", "Range", dd("range", r)));
        }
        if ty == "graph" {
            let filled = s(e, "style") == "filled";
            props = props.push(prow("p-style", "Style", eseg(&p, vec![("Line".into(), !filled, Msg::ElSet("style", json!("line"))), ("Filled".into(), filled, Msg::ElSet("style", json!("filled")))])));
        }
        if ty == "clock" {
            let f = s(e, "format");
            let l = CLOCK.iter().find(|(k, _)| *k == f.as_str()).map(|(_, l)| *l).unwrap_or("24:00");
            props = props.push(prow("p-format", "Shows", dd("format", l.to_string())));
        }
        if matches!(ty.as_str(), "text" | "value" | "clock") {
            let z = e.get("size").and_then(|x| x.as_i64()).unwrap_or(1);
            props = props.push(prow("p-size", "Size", eseg(&p, (1..=4).map(|n| (format!("{n}×"), z == n, Msg::ElSet("size", json!(n)))).collect())));
        }
        if ty == "box" || ty == "line" {
            let fill = e.get("fill").and_then(|x| x.as_bool()).unwrap_or(false);
            let line = ty == "line";
            props = props.push(prow("p-fill", "Style", eseg(&p, vec![("Line".into(), line, Msg::ElSet("type", json!("line"))), ("Box".into(), !line && !fill, Msg::ElSet("fill", json!(false))), ("Filled".into(), !line && fill, Msg::ElSet("fill", json!(true)))])));
        }
        if ty == "picture" {
            let shade = e.get("shade").and_then(|x| x.as_bool()).unwrap_or(true);
            let th = e.get("threshold").and_then(|x| x.as_u64()).unwrap_or(128);
            props = props.push(prow("p-shade", "Dots", eseg(&p, vec![("Shaded".into(), shade, Msg::ElSet("shade", json!(true))), ("Sharp".into(), !shade, Msg::ElSet("shade", json!(false)))])));
            props = props.push(prow("p-dark", "Darkness", eseg(&p, DARK.iter().map(|(v, l)| (l.to_string(), th == *v, Msg::ElSet("threshold", json!(v)))).collect())));
        }
        let (x, y, w, h) = bounds(e);
        props = props.push(prow("p-place", "Place", tb(format!("x {x} · y {y} · {w} × {h}"), 12.5, p.text).into()));
    } else {
        props = props.push(t("Pick something on the screen, or add something from the left.", 13.0, p.muted));
    }
    let show = if s(&scr, "show") == "hidden" { "Not in the page list" } else { "In the page list" };
    let whilerun = s(&scr, "whileRunning");
    props = props.push(gap(1.0, 8.0)).push(tb("This screen", 14.0, p.text));
    // The name reads as plain text, typed into where it stands.
    let name_style = move |_t: &Theme, st: text_input::Status| text_input::Style {
        background: iced::Background::Color(if matches!(st, text_input::Status::Focused { .. }) { Pal::mix(p.panel, p.bg, 0.5) } else { Color::TRANSPARENT }),
        border: iced::Border { color: if matches!(st, text_input::Status::Focused { .. }) { p.line } else { Color::TRANSPARENT }, width: 1.0, radius: 6.0.into() },
        icon: p.muted,
        placeholder: p.muted,
        value: p.text,
        selection: Pal::a(ORANGE, 0.35),
    };
    let name = text_input("Name", &s(&scr, "name")).on_input(Msg::ScreenName).padding(Padding { top: 2.0, right: 0.0, bottom: 2.0, left: 4.0 }).size(12.5).font(ui::weight(iced::font::Weight::Bold)).align_x(iced::alignment::Horizontal::Right).style(name_style).width(Length::Fixed(150.0));
    props = props.push(prow("s-name", "Name", name.into()));
    props = props.push(prow("s-show", "Show it", dd("show", show.into())));
    let also: E = if a.while_typing {
        text_input("A game's name", &whilerun).on_input(Msg::WhileText).on_submit(Msg::WhileDone).padding(5.0).size(13.0).width(Length::Fixed(150.0)).into()
    } else {
        dd("while", if whilerun.is_empty() { "Never".to_string() } else { format!("While {whilerun} runs") })
    };
    props = props.push(prow("s-while", "Also show it", also));
    props = props.push(t("Values you can show: CPU and graphics card use and heat, memory, the drive, network speed, fan speeds, frames per second, the time, the weather, the song playing, the battery of a controller.", 12.0, p.muted));
    // A little wider than 0.64 of the mockup's, so "Also show it  While Trains 2010 runs" fits on one line.
    let props = card(&p, props).padding(Padding { top: 16.0, right: 12.0, bottom: 16.0, left: 12.0 }).width(Length::Fixed(sz(380.0) + 12.0));

    let top = row![
        big_title(&p, vec![("Keyboard screen".into(), false), (s(&scr, "name"), true)]),
        Space::new().width(Length::Fill),
        btn(&p, "Undo", Kind::Normal, on(a, "undo"), Some(Msg::Undo)),
        btn(&p, "Show on the keyboard", Kind::Normal, on(a, "show"), Some(Msg::Show)),
        orange_btn("Done", on(a, "done"), Msg::Done),
    ]
    .spacing(8.0)
    .align_y(iced::Center);
    column![top, gap(1.0, 12.0), row![add, middle, props].spacing(14.0)].spacing(6.0).into()
}

fn editor_canvas(a: &App) -> E<'_> {
    let els = a.elements();
    let pk = a.el.and_then(|i| els.get(i)).map(|e| {
        let (x, y, w, h) = bounds(e);
        (x, y, w, h, el_name(e))
    });
    let frame = a.editor_frame.clone().unwrap_or_default();
    let cv = canvas(pics::Screen { frame, pick: pk, glow: true }).width(Length::Fixed(EDITOR_SIZE.width)).height(Length::Fixed(EDITOR_SIZE.height));
    let layer = canvas(Pointer { adding: a.adding.is_some() }).width(Length::Fixed(EDITOR_SIZE.width)).height(Length::Fixed(EDITOR_SIZE.height));
    stack![cv, layer].into()
}

/// Add's drawn icons.
struct AddIcon(&'static str);
impl canvas::Program<Msg> for AddIcon {
    type State = ();
    fn draw(&self, _s: &(), renderer: &iced::Renderer, _t: &Theme, b: iced::Rectangle, _c: mouse::Cursor) -> Vec<canvas::Geometry> {
        use canvas::{Path, Stroke};
        let mut f = canvas::Frame::new(renderer, b.size());
        let (w, h) = (b.width, b.height);
        let grey = Color::from_rgb8(0xa8, 0xab, 0xb4);
        let st = |c: Color| Stroke::default().with_color(c).with_width(1.4);
        match self.0 {
            "graph" => {
                let p = Path::new(|p| {
                    p.move_to(iced::Point::new(0.5, h * 0.55));
                    for i in 1..=12 {
                        let x = w * i as f32 / 12.0;
                        p.line_to(iced::Point::new(x, h * 0.55 - (x / w * std::f32::consts::TAU * 1.5).sin() * h * 0.2));
                    }
                });
                f.stroke(&p, st(grey));
            }
            "clock" => {
                let red = Color::from_rgb8(0xe5, 0x48, 0x3c);
                f.fill(&Path::circle(iced::Point::new(w / 2.0, h * 0.56), w * 0.4), red);
                f.fill(&Path::circle(iced::Point::new(w / 2.0, h * 0.56), w * 0.28), Color::WHITE);
                f.fill(&Path::circle(iced::Point::new(w * 0.18, h * 0.16), w * 0.13), red);
                f.fill(&Path::circle(iced::Point::new(w * 0.82, h * 0.16), w * 0.13), red);
                f.stroke(&Path::new(|p| {
                    p.move_to(iced::Point::new(w / 2.0, h * 0.36));
                    p.line_to(iced::Point::new(w / 2.0, h * 0.56));
                    p.line_to(iced::Point::new(w * 0.64, h * 0.64));
                }), Stroke::default().with_color(Color::from_rgb8(0x20, 0x20, 0x20)).with_width(1.2));
            }
            _ => {
                f.stroke(&Path::rectangle(iced::Point::new(1.0, 2.0), iced::Size::new(w - 2.0, h - 4.0)), st(grey));
                f.fill(&Path::new(|p| {
                    p.move_to(iced::Point::new(2.5, h - 3.0));
                    p.line_to(iced::Point::new(w * 0.4, h * 0.45));
                    p.line_to(iced::Point::new(w * 0.62, h * 0.7));
                    p.line_to(iced::Point::new(w * 0.75, h * 0.55));
                    p.line_to(iced::Point::new(w - 2.5, h - 3.0));
                    p.close();
                }), grey);
                f.fill(&Path::circle(iced::Point::new(w * 0.72, h * 0.32), 1.6), grey);
            }
        }
        vec![f.into_geometry()]
    }
}

/// The pointer on the editor's screen: press to pick up (or on the picked one's corner to resize), drag, let go; and
/// letting go of one of Add's buttons (dragged here, or let go anywhere else: added at its usual place).
struct Pointer {
    adding: bool,
}
impl canvas::Program<Msg> for Pointer {
    type State = bool;
    fn update(&self, pressed: &mut bool, event: &iced::Event, bounds: iced::Rectangle, cursor: mouse::Cursor) -> Option<canvas::Action<Msg>> {
        let pos = cursor.position_in(bounds);
        match event {
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let p = pos?;
                *pressed = true;
                Some(canvas::Action::publish(Msg::Press(p)).and_capture())
            }
            iced::Event::Mouse(mouse::Event::CursorMoved { .. }) if *pressed => {
                let p = cursor.position().map(|c| iced::Point::new(c.x - bounds.x, c.y - bounds.y))?;
                Some(canvas::Action::publish(Msg::Move(p)).and_capture())
            }
            iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) if *pressed => {
                *pressed = false;
                Some(canvas::Action::publish(Msg::Release).and_capture())
            }
            iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) if self.adding => Some(canvas::Action::publish(match pos {
                Some(p) => Msg::AddDrop(p),
                None => Msg::AddEnd,
            })),
            _ => None,
        }
    }
    fn draw(&self, _s: &bool, renderer: &iced::Renderer, _t: &Theme, b: iced::Rectangle, _c: mouse::Cursor) -> Vec<canvas::Geometry> {
        vec![canvas::Frame::new(renderer, b.size()).into_geometry()]
    }
    fn mouse_interaction(&self, pressed: &bool, bounds: iced::Rectangle, cursor: mouse::Cursor) -> mouse::Interaction {
        if *pressed || (self.adding && cursor.is_over(bounds)) {
            mouse::Interaction::Grabbing
        } else if cursor.is_over(bounds) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }
}

// ------------------------------------------------------------------ dialogs
fn dialog<'a>(a: &'a App, d: &'a Dlg) -> E<'a> {
    let p = a.p;
    let list = |title: &str, rows: Vec<(String, String, Msg)>| -> E<'a> {
        let mut c = Column::new().spacing(2.0);
        for (i, (l, sub, m)) in rows.into_iter().enumerate() {
            let here = i == a.dlg_focus;
            let r = container(column![t(l, 14.0, p.text), t1(sub, 12.0, p.muted)].spacing(2.0)).padding(Padding { top: 8.0, right: 12.0, bottom: 8.0, left: 12.0 }).width(Length::Fill).style(ui::boxed(if here { p.sel() } else { Color::TRANSPARENT }, 6.0, None));
            c = c.push(mouse_area(r).on_press(m).interaction(mouse::Interaction::Pointer));
        }
        ui::dialog_box(&p, 520.0, column![tb(title.to_string(), 17.0, p.text), container(scrollable(c)).height(Length::Fixed(420.0)), row![Space::new().width(Length::Fill), btn(&p, "Cancel", Kind::Normal, false, Some(Msg::CloseDlg))]].spacing(12.0))
    };
    match d {
        Dlg::Hearth => list("What should the key do?", actions::HEARTH_ACTIONS.iter().map(|(k, l)| (l.to_string(), hearth_words(k).1, Msg::HearthAction(*k))).collect()),
        Dlg::Apps(v) => list("Open an app", v.iter().map(|(id, n)| (n.clone(), id.clone(), Msg::AppPicked(id.clone(), n.clone()))).collect()),
        Dlg::Pick(w, v) => list(match *w {
            "value" => "What it shows",
            "range" => "Range",
            "show" => "Show it",
            "while" => "Also show it",
            "format" => "The clock shows",
            "idle" => "Turn the light off after",
            _ => "Pick one",
        }, v.iter().map(|(k, l)| (l.clone(), String::new(), Msg::Picked(*w, k.clone()))).collect()),
        Dlg::Rename(x) => ui::dialog_box(&p, 460.0, column![
            tb(format!("Name M{}", a.set), 17.0, p.text),
            t1("The screen shows it when you switch to this set.", 13.0, p.muted),
            text_input("Everyday, Trainz…", x).on_input(Msg::RenameText).on_submit(Msg::RenameSave).padding(8.0).size(14.0),
            row![Space::new().width(Length::Fill), btn(&p, "Cancel", Kind::Normal, false, Some(Msg::CloseDlg)), btn(&p, "Keep", Kind::Primary, false, Some(Msg::RenameSave))].spacing(8.0)
        ].spacing(12.0)),
        Dlg::Games(v) => {
            let mut c = Column::new().spacing(10.0).push(tb("A key set for each game", 17.0, p.text)).push(t1("While a game runs, its key set is used; the one you picked comes back when it closes. Type part of the game's name (as its program is called).", 13.0, p.muted));
            for (i, g) in v.iter().enumerate() {
                c = c.push(row![container(t(format!("M{} · {}", i + 1, a.set_name(i as u8 + 1)), 14.0, p.text)).width(Length::Fixed(150.0)), text_input("No game", g).on_input(move |x| Msg::GameText(i, x)).padding(7.0).size(14.0)].spacing(10.0).align_y(iced::Center));
            }
            c = c.push(row![Space::new().width(Length::Fill), btn(&p, "Cancel", Kind::Normal, false, Some(Msg::CloseDlg)), btn(&p, "Keep", Kind::Primary, false, Some(Msg::GamesSave))].spacing(8.0));
            ui::dialog_box(&p, 560.0, c)
        }
    }
}

/// The choices a ▾ opens (what a value shows, a range, where a screen shows, the light's idle time).
pub fn choices(a: &App, what: &str) -> Vec<(String, String)> {
    match what {
        "value" => VALUES.iter().map(|(k, _, _)| (k.to_string(), value_name(k))).collect(),
        "format" => CLOCK.iter().map(|(k, l)| (k.to_string(), l.to_string())).collect(),
        // "Also show it": never, or while one of the games named for the key sets runs, or another one typed in.
        "while" => {
            let mut v = vec![(String::new(), "Never".to_string())];
            let cur = a.cur_screen().map(|x| s(&x, "whileRunning")).unwrap_or_default();
            for g in a.games().into_iter().chain(std::iter::once(cur)) {
                if !g.trim().is_empty() && !v.iter().any(|(k, _)| *k == g) {
                    v.push((g.clone(), format!("While {g} runs")));
                }
            }
            v.push(("@other".into(), "Another game…".into()));
            v
        }
        "range" => {
            let id = a.el.and_then(|i| a.elements().get(i).cloned()).map(|e| s(&e, "value")).unwrap_or_default();
            let u = hearth_g15::lcd::screen::unit_of(&id).trim().to_string();
            [(0, 100), (30, 90), (20, 100), (0, 50), (0, 10000)].iter().map(|(lo, hi)| (format!("{lo}-{hi}"), format!("{lo}{u} – {hi}{u}"))).collect()
        }
        "show" => vec![("list".into(), "In the page list".into()), ("hidden".into(), "Not in the page list".into())],
        "idle" => [(0, "Never"), (5, "5 minutes idle"), (10, "10 minutes idle"), (30, "30 minutes idle"), (60, "1 hour idle")].iter().map(|(k, l)| (k.to_string(), l.to_string())).collect(),
        _ => Vec::new(),
    }
}
