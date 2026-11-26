//! The Dashboard on the TV, the Ally and the phone (board 04, docs/mockups/server-expansion/04-dashboard-tv.png):
//! the same blocks as the desk view, big, for a controller: A Fix it, X Show me, LB / RB the pages (Health, Services,
//! Game servers, Playing now, Backups), Y Edit layout, B Back. Wide enough, the phone's page is shown beside it ("The
//! same page on the phone"); on a narrow screen (a phone, an Ally on its side) the phone's page is the page.

use super::Page;
use crate::{act, act_v, set, App, Msg, E};
use hearth_oss::w::{self, Sz, Tg};
use hearth_server_parts::draw::{self, pad, st};
use hearth_server_parts::{arr, b, f, nums, s};
use hearth_ui::blocks::{bl, PageDef};
use hearth_ui::ui::{self, t, tb};
use hearth_ui::Pal;
use iced::keyboard::{key::Named, Key};
use iced::widget::text::Wrapping;
use iced::widget::{column, container, row, Column, Row, Space};
use iced::{Color, Length};
use serde_json::{json, Value};

pub const PAGE: Page = Page { id: "tv", title: "Dashboard on the TV", side: "dash", back: "dash", engines: &["dash"], every: 2, bare: true, view, tags: super::no_tags, key };
pub const PHONE: Page = Page { id: "phone", title: "Dashboard on the phone", side: "dash", back: "dash", engines: &["dash"], every: 2, bare: true, view: phone_view, tags: super::no_tags, key: super::no_key };

pub const BLOCKS: PageDef = PageDef {
    id: "tv",
    title: "Dashboard on the TV",
    blocks: &[
        bl("top", "Home server, its pages, the time", 0, 0, 12, 1),
        bl("health", "How it's doing, and the one thing to fix", 0, 1, 6, 3),
        bl("temps", "Temperatures", 6, 1, 3, 3),
        bl("pc", "The PC", 0, 4, 3, 4),
        bl("services", "Services", 3, 4, 3, 4),
        bl("now", "Right now", 6, 4, 3, 4),
        bl("games", "Game servers", 0, 8, 3, 3),
        bl("backups", "Last night's backups", 3, 8, 3, 3),
        bl("power", "Power · today", 6, 8, 3, 3),
        bl("phone", "The same page on the phone", 9, 1, 3, 10),
        bl("foot", "The controller's buttons", 0, 11, 12, 1),
    ],
};
pub const PHONE_BLOCKS: PageDef = PageDef {
    id: "phone",
    title: "Dashboard on the phone",
    blocks: &[bl("ph-need", "What needs a look", 0, 0, 12, 2), bl("ph-nums", "GPU, memory, drives, power", 0, 2, 12, 3), bl("ph-services", "Services", 0, 5, 12, 3), bl("ph-quick", "Quick", 0, 8, 12, 2), bl("ph-alerts", "Alerts", 0, 10, 12, 2)],
};

const TABS: [&str; 5] = ["Health", "Services", "Game servers", "Playing now", "Backups"];
/// The tiles the controller moves between on Health.
const TILES: [&str; 8] = ["health", "temps", "pc", "services", "now", "games", "backups", "power"];

fn key(a: &App, k: Key<&str>, shift: bool) -> Option<Msg> {
    let tab = a.ui_n("tvTab");
    let sel = a.ui_n("tvSel");
    Some(match k {
        Key::Named(Named::Tab) => set("tvTab", json!(if shift { (tab + TABS.len() - 1) % TABS.len() } else { (tab + 1) % TABS.len() })),
        Key::Named(Named::PageDown) => set("tvTab", json!((tab + 1) % TABS.len())),
        Key::Named(Named::PageUp) => set("tvTab", json!((tab + TABS.len() - 1) % TABS.len())),
        Key::Named(Named::ArrowRight) => set("tvSel", json!((sel + 1).min(TILES.len() - 1))),
        Key::Named(Named::ArrowLeft) => set("tvSel", json!(sel.saturating_sub(1))),
        Key::Named(Named::ArrowDown) => set("tvSel", json!(if sel == 0 { 2 } else { (sel + 3).min(TILES.len() - 1) })),
        Key::Named(Named::ArrowUp) => set("tvSel", json!(if sel <= 2 { 0 } else { sel - 3 })),
        Key::Named(Named::Enter) | Key::Named(Named::Space) => press(a, false),
        Key::Character("x") => press(a, true),
        Key::Named(Named::ContextMenu) => press(a, true),
        _ => return None,
    })
}

/// A (or X: Show me) on the lit tile.
fn press(a: &App, show: bool) -> Msg {
    let d = a.d("dash");
    let need = arr(d, "needs").into_iter().next();
    match TILES[a.ui_n("tvSel").min(TILES.len() - 1)] {
        "health" => match (need, show) {
            (Some(n), false) if !b(&n, "nofix") => act_v("dash", vec!["fix".into(), if s(&n, "fix").is_empty() { s(&n, "id") } else { s(&n, "fix") }], "Fixed"),
            (Some(n), _) => super::dash::show_me(&s(&n, "show")),
            (None, _) => set("tvTab", json!(1)),
        },
        "services" => set("tvTab", json!(1)),
        "games" => set("tvTab", json!(2)),
        "now" => set("tvTab", json!(3)),
        "backups" => set("tvTab", json!(4)),
        _ => Msg::Go("dash"),
    }
}

fn tile<'a>(p: &Pal, inner: impl Into<E<'a>>, lit: bool) -> E<'a> {
    let line = if lit { (p.accent, 4.0) } else { (p.line, 1.0) };
    container(container(inner).padding(22.0).width(Length::Fill).height(Length::Fill).style(ui::boxed(p.panel, 20.0, Some(line)))).padding(if lit { 0.0 } else { 3.0 }).width(Length::Fill).height(Length::Fill).into()
}

fn k<'a>(p: &Pal, s_: &str) -> E<'a> {
    tb(s_.to_uppercase(), 13.0, p.muted).wrapping(Wrapping::None).into()
}

fn sr<'a>(p: &Pal, left: E<'a>, right: E<'a>, last: bool) -> E<'a> {
    draw::under(p, row![left, Space::new().width(Length::Fill), right].spacing(14.0).align_y(iced::Center), last)
}

/// A controller button as the TV's hints draw it (round A, B, X, Y; LB RB in a box).
pub fn pb<'a>(p: &Pal, l: &str) -> E<'a> {
    let round = l.len() == 1;
    let (bg, fg) = if l == "A" { (p.accent, p.ink) } else { (Color::from_rgba(1.0, 1.0, 1.0, 0.92), Color::from_rgb8(0x1b, 0x0f, 0x06)) };
    container(tb(l.to_string(), 12.0, fg).wrapping(Wrapping::None)).center_x(Length::Fixed(if round { 28.0 } else { 52.0 })).center_y(Length::Fixed(28.0)).style(ui::boxed(bg, if round { 14.0 } else { 7.0 }, None)).into()
}

fn health<'a>(p: &Pal, d: &Value) -> E<'a> {
    let ok = f(d, "healthy");
    let tot = f(d, "total").max(1.0);
    let ring = draw::ring(p, (ok / tot) as f32, w::green(), 200.0, 18.0, column![tb(format!("{ok}"), 50.0, p.text), t(format!("of {tot} healthy"), 16.0, p.muted)].align_x(iced::Center).into());
    let need = arr(d, "needs").into_iter().next();
    let (kk, title, text) = match &need {
        Some(n) => (if f(d, "needsCount") > 1.0 { format!("{} things to fix", f(d, "needsCount")) } else { "All good, one thing to fix".into() }, if s(n, "tvTitle").is_empty() { s(n, "title") } else { s(n, "tvTitle") }, if s(n, "tvText").is_empty() { s(n, "text") } else { s(n, "tvText") }),
        None => ("All good".into(), "Everything's healthy".into(), "Nothing needs a look. The server keeps an eye on it and tells your phone if something comes up.".into()),
    };
    let mut btns = Row::new().spacing(12.0);
    if let Some(n) = &need {
        if !b(n, "nofix") {
            let fx = if s(n, "fix").is_empty() { s(n, "id") } else { s(n, "fix") };
            btns = btns.push(iced::widget::mouse_area(container(row![pb(p, "A"), tb("Fix it", 15.0, p.ink)].spacing(8.0).align_y(iced::Center)).padding(pad(0.0, 20.0, 0.0, 12.0)).center_y(Length::Fixed(44.0)).style(ui::boxed(p.accent, 12.0, None))).on_press(act_v("dash", vec!["fix".into(), fx], "Fixed")));
        }
        btns = btns.push(iced::widget::mouse_area(container(row![pb(p, "X"), t("Show me", 15.0, p.text)].spacing(8.0).align_y(iced::Center)).padding(pad(0.0, 20.0, 0.0, 12.0)).center_y(Length::Fixed(44.0)).style(ui::boxed(p.panel2, 12.0, Some((p.line, 1.0))))).on_press(super::dash::show_me(&s(n, "show"))));
    }
    row![ring, column![k(p, &kk), tb(title, 30.0, p.text), t(text, 17.0, p.muted), container(btns).padding(pad(10.0, 0.0, 0.0, 0.0))].spacing(6.0).width(Length::Fill)].spacing(30.0).align_y(iced::Center).into()
}

fn tiles_of<'a>(a: &'a App, d: &'a Value) -> Vec<(&'static str, E<'a>)> {
    let p = a.p;
    let tv = hearth_server_parts::at(d, "tv");
    let sel = TILES[a.ui_n("tvSel").min(TILES.len() - 1)];
    let lit = |id: &str| id == sel;
    let temps = column![k(&p, "Temperatures"), row![tb(s(tv, "temp.big"), 44.0, p.text), container(t(" GPU", 20.0, p.muted)).padding(pad(0.0, 0.0, 8.0, 0.0))].align_y(iced::alignment::Vertical::Bottom), t(s(tv, "temp.line"), 17.0, p.muted), container(draw::spark(nums(d, "temps24"), w::yellow(), true, Length::Fill, 90.0)).padding(pad(14.0, 0.0, 0.0, 0.0))].spacing(4.0);
    let mut pc = Column::new().push(k(&p, &format!("The {}", s(d, "pc"))));
    let bars = arr(tv, "bars");
    for (i, x) in bars.iter().enumerate() {
        let name = x.get(0).and_then(Value::as_str).unwrap_or("").to_string();
        let v = x.get(1).and_then(Value::as_str).unwrap_or("").to_string();
        let fr = x.get(2).and_then(Value::as_f64).unwrap_or(0.0) as f32;
        let c = if name == "Drives" { Some(w::green()) } else { None };
        pc = pc.push(sr(&p, t(name, 19.0, p.text).into(), row![w::meter(&p, fr, c, 170.0, 6.0), container(t(v, 15.0, p.muted)).width(Length::Fixed(64.0)).align_x(iced::alignment::Horizontal::Right)].spacing(10.0).align_y(iced::Center).into(), i + 1 == bars.len()));
    }
    let mut sv = Column::new().push(k(&p, "Services"));
    for x in arr(tv, "services") {
        let warn = s(&x, "state") != "ok";
        sv = sv.push(sr(&p, row![st(&s(&x, "state"), 14.0), t(s(&x, "name"), 19.0, p.text)].spacing(14.0).align_y(iced::Center).into(), t(s(&x, "line"), 15.0, if warn { w::yellow() } else { p.muted }).into(), false));
    }
    let more = f(tv, "more");
    if more > 0.0 {
        sv = sv.push(sr(&p, t(format!("+ {more} more{}", if b(tv, "moreOk") { ", all healthy" } else { "" }), 15.0, p.muted).into(), t("›", 15.0, p.muted).into(), true));
    }
    let mut now = Column::new().push(k(&p, "Right now"));
    let nl = arr(tv, "now");
    for (i, x) in nl.iter().enumerate() {
        now = now.push(sr(&p, row![w::mark(&s(x, "mark"), &s(x, "grad"), 34.0), column![t(s(x, "title"), 19.0, p.text), t(s(x, "line"), 15.0, p.muted)]].spacing(14.0).align_y(iced::Center).into(), Space::new().into(), i + 1 == nl.len()));
    }
    if nl.is_empty() {
        now = now.push(container(t("Nothing's playing, streaming or downloading", 15.0, p.muted)).padding(pad(10.0, 0.0, 0.0, 0.0)));
    }
    let mut games = Column::new().push(k(&p, "Game servers"));
    let gl = arr(tv, "games");
    for (i, g) in gl.iter().enumerate() {
        let on = b(g, "on");
        let btn = w::chip_x(&p, if on { "Stop" } else { "Start" }, !on, false, Sz::Md, Some(act_v("games", vec![if on { "stop".into() } else { "start".into() }, s(g, "id")], "")));
        games = games.push(sr(&p, row![st(if on { "ok" } else { "off" }, 14.0), column![t(s(g, "name"), 19.0, p.text), t(s(g, "line"), 15.0, p.muted)]].spacing(14.0).align_y(iced::Center).into(), btn, i + 1 == gl.len()));
    }
    if gl.is_empty() {
        games = games.push(container(t("Game servers come with their own part from the Hearth Store", 15.0, p.muted)).padding(pad(10.0, 0.0, 0.0, 0.0)));
    }
    let mut bk = Column::new().push(k(&p, "Last night's backups"));
    for x in arr(tv, "backups") {
        bk = bk.push(sr(&p, row![st("ok", 14.0), t(s(&x, "what"), 19.0, p.text)].spacing(14.0).align_y(iced::Center).into(), t(s(&x, "when"), 15.0, p.muted).into(), false));
    }
    bk = bk.push(sr(&p, t(s(tv, "backupsLine"), 15.0, p.muted).into(), w::chip(&p, "Restore", false, Sz::Md, Some(Msg::Go("backup"))), true));
    let power = column![k(&p, "Power · today"), row![tb(s(d, "power.now"), 34.0, p.text), Space::new().width(Length::Fill), t(s(d, "power.kwh"), 16.0, p.muted)].align_y(iced::Center), container(draw::bars(&p, nums(d, "power.bars"), Length::Fill, 110.0)).padding(pad(12.0, 0.0, 0.0, 0.0)), row![t("12 AM", 13.0, p.muted), Space::new().width(Length::Fill), t("noon", 13.0, p.muted), Space::new().width(Length::Fill), t("now", 13.0, p.muted)]].spacing(4.0);
    vec![
        ("health", tile(&p, health(&p, d), lit("health"))),
        ("temps", tile(&p, temps, lit("temps"))),
        ("pc", tile(&p, pc, lit("pc"))),
        ("services", tile(&p, sv, lit("services"))),
        ("now", tile(&p, now, lit("now"))),
        ("games", tile(&p, games, lit("games"))),
        ("backups", tile(&p, bk, lit("backups"))),
        ("power", tile(&p, power, lit("power"))),
    ]
}

fn top<'a>(a: &'a App, d: &Value) -> E<'a> {
    let p = a.p;
    let tab = a.ui_n("tvTab");
    let mut tabs = Row::new().spacing(10.0);
    for (i, l) in TABS.iter().enumerate() {
        tabs = tabs.push(if i == tab { w::chip(&p, *l, true, Sz::Lg, Some(set("tvTab", json!(i)))) } else { w::ghost(&p, *l, Sz::Lg, Some(set("tvTab", json!(i)))) });
    }
    row![
        w::mark("HS", "or", 44.0),
        tb("Home server", 34.0, p.text),
        container(tabs).padding(pad(0.0, 0.0, 0.0, 36.0)),
        Space::new().width(Length::Fill),
        column![t(s(hearth_server_parts::at(d, "tv"), "clock"), 30.0, p.text).font(iced::Font { weight: iced::font::Weight::Light, ..hearth_ui::app::INTER }), t("on the TV · the same page fits the Ally and the phone", 15.0, p.muted)].align_x(iced::Right),
    ]
    .spacing(18.0)
    .align_y(iced::Center)
    .into()
}

fn foot<'a>(p: &Pal) -> E<'a> {
    let hint = |b_: &str, l: &str| -> E<'a> { row![pb(p, b_), t(l.to_string(), 15.0, p.text)].spacing(8.0).align_y(iced::Center).into() };
    container(row![t("Home server · TV view · the same blocks as the desk dashboard · updates in the Hearth Store", 15.0, p.muted), Space::new().width(Length::Fill), hint("A", "Fix it"), hint("X", "Show me"), hint("LB RB", "Pages"), hint("Y", "Edit layout"), hint("B", "Back")].spacing(22.0).align_y(iced::Center))
        .padding(pad(16.0, 40.0, 16.0, 40.0))
        .width(Length::Fill)
        .style(ui::boxed(Pal::mix(p.bg, Color::BLACK, 0.35), 0.0, None))
        .into()
}

/// The other TV pages (LB / RB): one big list each.
fn list_page<'a>(a: &'a App, d: &'a Value, tab: usize) -> E<'a> {
    let p = a.p;
    let tv = hearth_server_parts::at(d, "tv");
    let mut c = Column::new().spacing(0.0);
    match tab {
        1 => {
            c = c.push(k(&p, &format!("Services · {} of {} healthy", f(d, "healthy"), f(d, "total"))));
            let svcs = arr(d, "services");
            for (i, x) in svcs.iter().enumerate() {
                c = c.push(sr(&p, row![st(&s(x, "state"), 14.0), w::mark(&s(x, "mark"), &s(x, "grad"), 34.0), column![t(s(x, "name"), 19.0, p.text), t(s(x, "what"), 15.0, p.muted)]].spacing(14.0).align_y(iced::Center).into(), t(s(x, "foot"), 15.0, p.muted).into(), i + 1 == svcs.len()));
            }
        }
        2 => {
            c = c.push(k(&p, "Game servers"));
            for g in arr(tv, "games") {
                c = c.push(sr(&p, row![st(if b(&g, "on") { "ok" } else { "off" }, 14.0), column![t(s(&g, "name"), 19.0, p.text), t(s(&g, "line"), 15.0, p.muted)]].spacing(14.0).align_y(iced::Center).into(), Space::new().into(), false));
            }
            if arr(tv, "games").is_empty() {
                c = c.push(t("Game servers come with their own part from the Hearth Store", 17.0, p.muted));
            }
        }
        3 => {
            c = c.push(k(&p, "Playing now"));
            for x in arr(tv, "now") {
                c = c.push(sr(&p, row![w::mark(&s(&x, "mark"), &s(&x, "grad"), 34.0), column![t(s(&x, "title"), 19.0, p.text), t(s(&x, "line"), 15.0, p.muted)]].spacing(14.0).align_y(iced::Center).into(), Space::new().into(), false));
            }
            if arr(tv, "now").is_empty() {
                c = c.push(t("Nothing's playing or streaming right now", 17.0, p.muted));
            }
        }
        _ => {
            c = c.push(k(&p, "Last night's backups"));
            for x in arr(tv, "backups") {
                c = c.push(sr(&p, row![st("ok", 14.0), t(s(&x, "what"), 19.0, p.text)].spacing(14.0).align_y(iced::Center).into(), t(s(&x, "when"), 15.0, p.muted).into(), false));
            }
            c = c.push(sr(&p, t(s(tv, "backupsLine"), 15.0, p.muted).into(), w::chip(&p, "Restore", false, Sz::Md, Some(Msg::Go("backup"))), true));
        }
    }
    tile(&p, iced::widget::scrollable(c), false)
}

fn view(a: &App) -> E<'_> {
    let p = a.p;
    let d = a.d("dash");
    if d.is_null() || !b(d, "ok") {
        return container(column![tb("Home server", 34.0, p.text), t(if d.is_null() { "Looking at the server…".to_string() } else { s(d, "error") }, 17.0, p.muted)].spacing(10.0)).padding(56.0).into();
    }
    // narrow (a phone, an Ally on its side): the phone's page is the page
    if a.size.width < 760.0 {
        return phone_view(a);
    }
    let tab = a.ui_n("tvTab");
    let wide = a.size.width >= 1700.0;
    let mut items = tiles_of(a, d);
    items.push(("top", top(a, d)));
    items.push(("foot", foot(&p)));
    if wide {
        items.push(("phone", column![t("The same page on the phone", 13.0, p.muted), phone_frame(a, d)].spacing(10.0).into()));
    }
    let body: E<'_> = if tab == 0 {
        hearth_ui::arrange::parts(
            &a.describe,
            &p,
            "tv",
            items,
            move |ps| {
                let gap = 22.0;
                let r1 = row![container(ps.take("health")).width(Length::FillPortion(2)), container(ps.take("temps")).width(Length::FillPortion(1))].spacing(gap).height(Length::FillPortion(26));
                let r2 = row![container(ps.take("pc")).width(Length::FillPortion(1)), container(ps.take("services")).width(Length::FillPortion(1)), container(ps.take("now")).width(Length::FillPortion(1))].spacing(gap).height(Length::FillPortion(33));
                let r3 = row![container(ps.take("games")).width(Length::FillPortion(1)), container(ps.take("backups")).width(Length::FillPortion(1)), container(ps.take("power")).width(Length::FillPortion(1))].spacing(gap).height(Length::FillPortion(26));
                let grid = column![r1, r2, r3].spacing(gap).width(Length::Fill).height(Length::Fill);
                let mut mid = Row::new().spacing(54.0).push(grid);
                if wide {
                    mid = mid.push(container(ps.take("phone")).width(Length::Fixed(420.0)));
                }
                column![container(ps.take("top")).padding(pad(36.0, 56.0, 20.0, 56.0)), container(mid).padding(pad(0.0, 56.0, 24.0, 56.0)).height(Length::Fill), ps.take("foot")].into()
            },
            Msg::Describe,
        )
    } else {
        column![container(top(a, d)).padding(pad(36.0, 56.0, 20.0, 56.0)), container(list_page(a, d, tab)).padding(pad(0.0, 56.0, 24.0, 56.0)).height(Length::Fill), foot(&p)].into()
    };
    container(body).width(Length::Fill).height(Length::Fill).style(ui::boxed(p.bg, 0.0, None)).into()
}

// ------------------------------------------------------------------ the phone
fn ph_card<'a>(p: &Pal, inner: impl Into<E<'a>>) -> E<'a> {
    container(inner).padding(12.0).width(Length::Fill).style(ui::boxed(p.panel, 14.0, Some((p.line, 1.0)))).into()
}

fn phone_parts<'a>(a: &'a App, d: &'a Value) -> Vec<(&'static str, E<'a>)> {
    let p = a.p;
    let tv = hearth_server_parts::at(d, "tv");
    let need = arr(d, "needs").into_iter().next();
    let need_el: E<'a> = match need {
        Some(n) => {
            let fx = if s(&n, "fix").is_empty() { s(&n, "id") } else { s(&n, "fix") };
            let title = s(&n, "title").replace("Hearth Shield's", "Shield's");
            container(column![row![st("warn", 10.0), tb(title, 12.0, p.text)].spacing(6.0).align_y(iced::Center), row![w::chip(&p, "Fix it", true, Sz::Sm, if b(&n, "nofix") { None } else { Some(act_v("dash", vec!["fix".into(), fx], "Fixed")) }), w::chip(&p, "Show me", false, Sz::Sm, Some(super::dash::show_me(&s(&n, "show"))))].spacing(8.0)].spacing(10.0))
                .padding(12.0)
                .width(Length::Fill)
                .style(ui::boxed(Color::from_rgb8(0x2a, 0x1e, 0x12), 14.0, Some((Color::from_rgb8(0x6a, 0x4a, 0x1e), 1.0))))
                .into()
        }
        None => ph_card(&p, row![st("ok", 10.0), tb("All good", 12.0, p.text)].spacing(6.0)),
    };
    let kp = arr(d, "kpis");
    let get = |name: &str| kp.iter().find(|x| s(x, "k").starts_with(name)).cloned().unwrap_or(Value::Null);
    let gpu = get("GPU ·");
    let mem = get("Memory");
    let drv = get("Drives");
    let temp = get("Temperatures");
    let num = |label: &str, v: String| -> E<'a> { ph_card(&p, column![tb(label.to_uppercase(), 11.0, p.muted), tb(v, 22.0, p.text)].spacing(2.0)) };
    let nums_el = column![
        row![container(num("GPU", format!("{} · {}", s(&gpu, "v"), s(&temp, "v")))).width(Length::FillPortion(1)), container(num("Memory", format!("{} / {}", s(&mem, "v").split('.').next().unwrap_or(""), s(&mem, "of").trim_start_matches("/ ").trim_end_matches(" GB")))).width(Length::FillPortion(1))].spacing(10.0),
        row![container(num("Drives", format!("{}%", (f(&drv, "bar") * 100.0).round()))).width(Length::FillPortion(1)), container(num("Power", s(d, "power.now"))).width(Length::FillPortion(1))].spacing(10.0),
    ]
    .spacing(10.0);
    let mut svc = Column::new().spacing(6.0).push(tb("SERVICES", 11.0, p.muted));
    for x in arr(tv, "services") {
        let warn = s(&x, "state") != "ok";
        svc = svc.push(draw::lr(row![st(&s(&x, "state"), 10.0), t(s(&x, "short"), 13.0, p.text)].spacing(8.0).align_y(iced::Center), t(s(&x, "line"), 13.0, if warn { w::yellow() } else { p.muted })));
    }
    svc = svc.push(draw::lr(t(format!("+ {} more", f(tv, "more")), 13.0, p.muted), t("›", 13.0, p.muted)));
    let quick = column![
        tb("QUICK", 11.0, p.muted),
        row![container(w::chip(&p, "Wake the PC", false, Sz::Sm, Some(crate::open(&["hearth-settings-app", "--page", "hs-alerts"])))).width(Length::FillPortion(1)), container(w::chip(&p, "Restart a service", false, Sz::Sm, Some(set("tvTab", json!(1))))).width(Length::FillPortion(1))].spacing(8.0),
        row![container(w::chip(&p, "Start a server", false, Sz::Sm, Some(set("tvTab", json!(2))))).width(Length::FillPortion(1)), container(w::chip(&p, "Back up now", false, Sz::Sm, Some(act("backups", &["now", "all"], "Backing up now")))).width(Length::FillPortion(1))].spacing(8.0),
    ]
    .spacing(8.0);
    let mut al = Column::new().spacing(6.0).push(tb("ALERTS", 11.0, p.muted));
    for n in arr(d, "needs").iter().take(1) {
        al = al.push(draw::lr(t(s(n, "title").replace("Hearth Shield's block lists are old", "Shield lists old"), 13.0, p.text), t(hearth_ui::time::day(hearth_ui::time::now()), 13.0, p.muted)));
    }
    for e in arr(d, "earlier").iter().take(2) {
        al = al.push(draw::lr(t(s(e, "text"), 13.0, p.text), t(s(e, "when"), 13.0, p.muted)));
    }
    vec![("ph-need", need_el), ("ph-nums", nums_el.into()), ("ph-services", ph_card(&p, svc)), ("ph-quick", ph_card(&p, quick)), ("ph-alerts", ph_card(&p, al))]
}

/// The phone's page in its frame (the TV view's "The same page on the phone").
fn phone_frame<'a>(a: &'a App, d: &'a Value) -> E<'a> {
    let p = a.p;
    let parts = phone_parts(a, d);
    let mut c = Column::new().spacing(10.0).push(phone_head(a, d));
    for (_, e) in parts {
        c = c.push(e);
    }
    c = c.push(container(row![t("Made of blocks · ", 12.0, p.muted), iced::widget::mouse_area(tb("Edit layout", 12.0, p.accent)).on_press(Msg::Describe(hearth_ui::arrange::edit_layout()))]).center_x(Length::Fill));
    container(c).padding(16.0).width(Length::Fixed(420.0)).height(Length::Fill).style(ui::boxed(Color::from_rgb8(0x0e, 0x0b, 0x0a), 34.0, Some((p.line, 2.0)))).into()
}

fn phone_head<'a>(a: &'a App, d: &'a Value) -> E<'a> {
    let p = a.p;
    let clock = s(hearth_server_parts::at(d, "tv"), "clock").replace(" AM", "").replace(" PM", "");
    column![
        row![t(clock, 12.0, p.muted), Space::new().width(Length::Fill), t("Tailscale", 12.0, p.muted)].padding(pad(0.0, 6.0, 0.0, 6.0)),
        row![w::mark("HS", "or", 22.0), tb("Home server", 18.0, p.text), Space::new().width(Length::Fill), w::tag(&p, format!("{} of {}", f(d, "healthy"), f(d, "total")), Tg::Ok)].spacing(10.0).align_y(iced::Center).padding(pad(0.0, 4.0, 4.0, 4.0)),
    ]
    .spacing(12.0)
    .into()
}

/// On a phone (or a narrow window): the phone's page itself, its blocks.
fn phone_view(a: &App) -> E<'_> {
    let p = a.p;
    let d = a.d("dash");
    if d.is_null() || !b(d, "ok") {
        return container(t("Looking at the server…", 14.0, p.muted)).padding(20.0).into();
    }
    let items = phone_parts(a, d);
    let body = hearth_ui::arrange::parts(
        &a.describe,
        &p,
        "phone",
        items,
        move |ps| {
            let c = column![phone_head(a, d), ps.take("ph-need"), ps.take("ph-nums"), ps.take("ph-services"), ps.take("ph-quick"), ps.take("ph-alerts"), container(row![t("Made of blocks · ", 12.0, p.muted), iced::widget::mouse_area(tb("Edit layout", 12.0, p.accent)).on_press(Msg::Describe(hearth_ui::arrange::edit_layout()))]).center_x(Length::Fill)].spacing(10.0);
            container(iced::widget::scrollable(c)).padding(16.0).width(Length::Fixed(420.0_f32.min(a.size.width))).into()
        },
        Msg::Describe,
    );
    container(body).center_x(Length::Fill).height(Length::Fill).style(ui::boxed(p.bg, 0.0, None)).into()
}
