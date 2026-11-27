//! Game cache (board docs/mockups/server-expansion/07-game-cache.png; checklist server-game-cache.md): a game is
//! downloaded from the internet once and every other device at home gets it from the server (LANCache, run by
//! hearth-server-dns, with Hearth Shield's redirects for every device at home; port 80 installed with the admin
//! password once). Its numbers are hearth-server-dns' own (per game, per device this month, since it started).

use crate::kit::{self, pad, Tag, E};
use crate::{act, at, bars, head, kpi, list, meter, num, on, panel, s, sv, sw, Hub, Msg, Part, ROWLINE};
use hearth_ui::blocks::{bl, PageDef};
use hearth_ui::describe::Describe;
use hearth_ui::ui::{self, t, tb, ts};
use hearth_ui::Pal;
use iced::widget::{column, container, row, Column, Row, Space};
use iced::Length;
use serde_json::Value;

const P: Part = Part::Cache;

pub const PAGE: PageDef = PageDef {
    id: "srv-cache",
    title: "Game cache",
    blocks: &[
        bl("head", "Game cache and its buttons", 0, 0, 12, 1),
        bl("kpis", "Saved, cache size, this month, right now", 0, 1, 12, 2),
        bl("flow", "How it works, right now", 0, 3, 12, 3),
        bl("games", "Each game", 0, 6, 7, 4),
        bl("devices", "Use by device", 7, 6, 5, 2),
        bl("stores", "Stores it caches", 7, 8, 5, 2),
        bl("month", "Game downloads at home this month", 0, 10, 12, 2),
    ],
};
pub const PANEL: PageDef = PageDef {
    id: "srv-cache-side",
    title: "Game cache's settings",
    blocks: &[bl("settings", "Settings", 0, 0, 12, 4), bl("preload", "Pre-load overnight", 0, 4, 12, 2), bl("find", "How devices find it", 0, 6, 12, 2), bl("store", "Its own Store app", 0, 8, 12, 4)],
};

fn node<'a>(p: Pal, title: String, sub: String, hot: bool) -> E<'a, Msg> {
    let (bg, line) = if hot { (kit::rgb(0x2a, 0x1e, 0x14), p.accent) } else { (p.panel2, p.line) };
    container(column![ts(title, 15.0, p.text), t(sub, 12.0, p.muted)].spacing(2.0).align_x(iced::Center))
        .padding(pad(14.0, 16.0, 14.0, 16.0))
        .width(Length::Fixed(if hot { 230.0 } else { 200.0 }))
        .style(ui::boxed(bg, 14.0, Some((line, if hot { 1.5 } else { 1.0 }))))
        .into()
}

fn pipe<'a>(p: Pal, words: String, thick: bool) -> E<'a, Msg> {
    let bar: E<'a, Msg> = if thick {
        container(Space::new()).width(Length::Fill).height(Length::Fixed(6.0)).style(kit::gradient(90.0, kit::rgb(0xff, 0x8a, 0x3d), kit::rgb(0xff, 0xb3, 0x6b), 3.0, None)).into()
    } else {
        container(Space::new()).width(Length::Fill).height(Length::Fixed(3.0)).style(ui::boxed(kit::rgb(0x5a, 0x4a, 0x3e), 2.0, None)).into()
    };
    column![container(t(words, 12.0, p.muted)).center_x(Length::Fill), bar].spacing(10.0).width(Length::Fill).into()
}

fn flow<'a>(p: Pal, f: &'a Value) -> E<'a, Msg> {
    let mut devs = Column::new().spacing(5.0);
    for d in list(f, "devices") {
        let live = on(d, "live");
        devs = devs.push(
            container(row![crate::st(if live { "" } else { "o" }), tb(s(d, "name"), 15.0, p.text), Space::new().width(Length::Fill), t(s(d, "what"), 12.0, p.muted)].spacing(10.0).align_y(iced::Center))
                .padding(pad(6.0, 14.0, 6.0, 14.0))
                .width(Length::Fixed(330.0))
                .style(ui::boxed(p.panel2, 14.0, Some((p.line, 1.0)))),
        );
    }
    kit::card(
        &p,
        column![
            kit::caps(&p, "How it works, right now"),
            row![node(p, "The internet".into(), s(f, "internet"), false), pipe(p, s(f, "first"), false), node(p, "Game cache".into(), s(f, "cache"), true), pipe(p, s(f, "home"), true), devs].spacing(18.0).align_y(iced::Center)
        ]
        .spacing(12.0),
    )
    .padding(pad(14.0, 22.0, 14.0, 22.0))
    .into()
}

fn table<'a>(p: Pal, v: &'a Value) -> E<'a, Msg> {
    let w = [Length::FillPortion(9), Length::FillPortion(3), Length::FillPortion(3), Length::FillPortion(6), Length::FillPortion(3), Length::FillPortion(2)];
    let hdr = ["Game", "Store", "Cached", "Got by", "Saved", "Used"];
    let mut r = Row::new().spacing(8.0);
    for (i, x) in hdr.iter().enumerate() {
        r = r.push(container(kit::caps(&p, *x)).width(w[i]));
    }
    let mut c = Column::new().push(container(r).padding(pad(8.0, 10.0, 8.0, 10.0))).push(ui::hline(ROWLINE));
    for (n, g) in list(v, "games").iter().enumerate() {
        let more = on(g, "more");
        let cells = [s(g, "game"), s(g, "store"), s(g, "cached"), s(g, "by"), s(g, "saved"), s(g, "used")];
        let mut r = Row::new().spacing(8.0);
        for (i, x) in cells.into_iter().enumerate() {
            let c_ = if more && i == 0 || i == 1 || i == 5 { p.muted } else { p.text };
            r = r.push(container(t(x, 13.0, c_)).width(w[i]));
        }
        let row_ = container(r.align_y(iced::Center)).padding(pad(7.0, 10.0, 7.0, 10.0)).width(Length::Fill);
        c = c.push(if n == 0 && s(g, "used") == "now" { row_.style(ui::boxed(kit::rgb(0x2e, 0x22, 0x18), 0.0, None)) } else { row_ }).push(ui::hline(ROWLINE));
    }
    kit::card(&p, c).padding(pad(12.0, 14.0, 12.0, 14.0)).into()
}

fn devices<'a>(p: Pal, v: &'a Value) -> E<'a, Msg> {
    let mut c = Column::new().spacing(2.0).push(kit::caps(&p, "Use by device · this month"));
    for d in list(v, "devices") {
        let b = sv(at(d, "/bytes"));
        let note = s(d, "note");
        let right = if b == "0" || b.is_empty() { note } else if note.is_empty() { b } else { format!("{b} · {note}") };
        c = c.push(meter(p, s(d, "name"), num(at(d, "/share")), p.accent, right, 110.0));
    }
    kit::card(&p, c).padding(pad(14.0, 16.0, 14.0, 16.0)).into()
}

fn stores<'a>(p: Pal, v: &'a Value) -> E<'a, Msg> {
    let mut c = Column::new().spacing(0.0).push(container(kit::caps(&p, "Stores it caches")).padding(pad(0.0, 0.0, 6.0, 0.0)));
    for x in list(v, "stores") {
        let cached = at(x, "/1").as_bool().unwrap_or(false);
        c = c.push(
            container(row![t(sv(at(x, "/0")), 13.0, p.text).width(Length::Fill), if cached { kit::tag(&p, "Cached", Tag::Ok) } else { kit::tag(&p, "Not cached", Tag::Dark) }].align_y(iced::Center))
                .padding(pad(7.0, 0.0, 7.0, 0.0)),
        )
        .push(ui::hline(ROWLINE));
    }
    kit::card(&p, c).padding(pad(14.0, 16.0, 14.0, 16.0)).into()
}

fn month<'a>(p: Pal, v: &'a Value) -> E<'a, Msg> {
    let m = at(v, "/month");
    let vals: Vec<(f32, f32)> = list(m, "bars").iter().map(|x| (num(at(x, "/0")), 0.0)).collect();
    let legend = row![t("■", 12.0, p.accent), t(format!("from the cache {} · ", s(m, "hit")), 12.0, p.muted), t("■", 12.0, kit::rgb(0x5a, 0x4a, 0x3e)), t(format!("from the internet {}", s(m, "miss")), 12.0, p.muted)].spacing(4.0);
    kit::card(&p, column![row![kit::caps(&p, format!("Game downloads at home · {}", s(m, "name"))), Space::new().width(Length::Fill), legend].align_y(iced::Center), bars(vals, kit::rgb(0xd0, 0x6e, 0x30), p.accent, 40.0)].spacing(8.0))
        .padding(pad(12.0, 16.0, 12.0, 16.0))
        .into()
}

fn side<'a>(h: &'a Hub, d: &'a Describe, p: Pal) -> E<'a, Msg> {
    let v = h.view_of(P);
    let st = at(v, "/settings");
    let size = at(st, "/size").as_i64().unwrap_or(1000);
    let sizes: Vec<(String, bool, Option<Msg>)> = [(500, "500 GB"), (1000, "1 TB"), (2000, "2 TB")]
        .iter()
        .map(|(n, l)| (l.to_string(), size == *n, if size == *n { None } else { Some(act(P, &["cache-size", &n.to_string()], "Kept: the cache is that size now.")) }))
        .collect();
    let drive = kit::click(container(t(format!("{} ▾", s(st, "drive")), 12.0, p.text)).padding(pad(6.0, 10.0, 6.0, 10.0)).style(ui::boxed(p.bg2, 8.0, Some((p.line, 1.0)))), Some(Msg::Pick("drives")));
    let mut settings = Column::new().spacing(2.0).push(ts("Settings", 18.0, p.text)).push(crate::line(t("Cache size", 13.0, p.text).into(), kit::seg(&p, sizes))).push(crate::line(t("On", 13.0, p.text).into(), drive));
    if h.pick == "drives" {
        for dr in list(v, "drives") {
            let path = s(dr, "path");
            settings = settings.push(container(kit::chip(&p, format!("{} · {}", s(dr, "label"), s(dr, "size")), Some(act(P, &["cache-drive", &path], "The cache moves there.")))).padding(pad(0.0, 0.0, 0.0, 20.0)));
        }
    }
    settings = settings
        .push(crate::line(t("When full, drop the least used", 13.0, p.text).into(), sw(p, P, "dropOld", on(st, "dropOld"))))
        .push(crate::line(crate::two(p, "Keep a game forever".into(), s(st, "keep")), kit::click(t("Pick", 12.0, p.accent), Some(Msg::Pick("keep")))));
    if on(st, "full") {
        // "Drop the least used" off and the cache is full: nothing is dropped, new downloads go straight through.
        settings = settings.push(t("Full: new downloads come straight from the internet, nothing kept is dropped", 11.0, kit::YELLOW));
    }
    if h.pick == "keep" {
        for g in list(v, "games").iter().filter(|g| !on(g, "more")).take(7) {
            let name = s(g, "game");
            let kept = s(st, "keep").split(", ").any(|x| x == name);
            settings = settings.push(container(kit::chip(&p, format!("{}{}", if kept { "✓ " } else { "" }, name), Some(act(P, &["cache-keep", &name, if kept { "off" } else { "on" }], "Kept.")))).padding(pad(0.0, 0.0, 0.0, 20.0)));
        }
    }
    let preload = column![
        kit::caps(&p, "Pre-load overnight"),
        crate::line(crate::two(p, "Get updates before you play".into(), "for games installed on 2 or more devices".into()), sw(p, P, "preload", on(st, "preload"))),
        crate::line(t("Between", 13.0, p.text).into(), container(t(s(st, "window"), 13.0, p.text)).padding(pad(6.0, 10.0, 6.0, 10.0)).style(ui::boxed(p.bg2, 8.0, Some((p.line, 1.0)))).into()),
    ]
    .spacing(2.0);
    let find = column![
        kit::caps(&p, "How devices find it"),
        t("Hearth Shield points the stores at the cache for every device at home, the Xbox and the TV too. Nothing to set on each device. Turn the cache off and everything downloads as before.", 12.0, p.muted).line_height(1.6)
    ]
    .spacing(6.0);
    let store = column![
        crate::store_box(p, row![tb("Game cache", 12.0, p.text), t(" · its own Store app", 12.0, p.text)].into(), "Updates in the Hearth Store. It adds a system service and Hearth Shield's redirect.", on(v, "installed"), act(P, &["cache-install"], "The game cache is on.")),
        t("Encrypted downloads can't be cached by any store's rules; those go straight through.", 12.0, p.muted).line_height(1.5)
    ]
    .spacing(14.0);
    let items: Vec<(&'static str, E<'a, Msg>)> = vec![("settings", settings.into()), ("preload", preload.into()), ("find", find.into()), ("store", store.into())];
    panel(p, hearth_ui::arrange::parts(d, &p, "srv-cache-side", items, |ps| column![ps.take("settings"), ps.take("preload"), ps.take("find"), ps.take("store")].spacing(18.0).into(), Msg::D))
}

pub fn view<'a>(h: &'a Hub, d: &'a Describe, p: Pal) -> E<'a, Msg> {
    let v = h.view_of(P);
    let hd = head(h, p, "Game cache", "A game is downloaded from the internet once. Every other device at home gets it from the server at home-network speed.".into(), vec![
        kit::chip(&p, "Pre-load a game", Some(Msg::Pick("keep"))),
        kit::chip_acc(&p, "Settings", Some(Msg::Pick("drives"))),
    ]);
    let ks = list(v, "kpi");
    let mut kp = Vec::new();
    for (i, k) in ks.iter().enumerate() {
        let big = sv(at(k, "/1"));
        let value: E<'a, Msg> = if i == 1 {
            row![tb(big, 24.0, p.text), t(format!(" {}", sv(at(k, "/2"))), 14.0, p.muted)].align_y(iced::Bottom).into()
        } else {
            tb(big, 24.0, p.text).into()
        };
        if i == 1 {
            kp.push(kpi(p, sv(at(k, "/0")), value, None, Some(num(at(k, "/3")))));
        } else {
            kp.push(kpi(p, sv(at(k, "/0")), value, Some(sv(at(k, "/2"))), None));
        }
    }
    let items: Vec<(&'static str, E<'a, Msg>)> = vec![
        ("head", hd),
        ("kpis", crate::cols(kp, 12.0)),
        ("flow", flow(p, at(v, "/flow"))),
        ("games", table(p, v)),
        ("devices", devices(p, v)),
        ("stores", stores(p, v)),
        ("month", month(p, v)),
    ];
    let main = hearth_ui::arrange::parts(d, &p, "srv-cache", items, |ps| {
        iced::widget::scrollable(
            column![
                ps.take("head"),
                ps.take("kpis"),
                ps.take("flow"),
                row![container(ps.take("games")).width(Length::FillPortion(135)), container(column![ps.take("devices"), ps.take("stores")].spacing(16.0)).width(Length::FillPortion(100))].spacing(16.0),
                ps.take("month"),
            ]
            .spacing(16.0),
        )
        .height(Length::Fill)
        .into()
    }, Msg::D);
    row![container(main).width(Length::Fill).height(Length::Fill).padding(pad(18.0, 22.0, 18.0, 22.0)), ui::vline(p.line), side(h, d, p)].height(Length::Fill).into()
}
