//! PCs (board docs/mockups/server-expansion/10-more-pcs.png; checklist server-more-pcs.md): more Hearth PCs as one
//! server over Hearth Link and Tailscale: what each is doing, each PC's share of the work, what moves if one goes off,
//! the last 7 days and what each PC may do. Each PC runs whole jobs (whole AI models): one model is never split over
//! the internet.

use crate::kit::{self, pad, Tag, E};
use crate::{at, boxed, head, line, list, num, on, panel, s, sv, sw, Hub, Msg, Part};
use hearth_ui::blocks::{bl, PageDef};
use hearth_ui::describe::Describe;
use hearth_ui::ui::{self, t, tb, ts};
use hearth_ui::Pal;
use iced::widget::{column, container, row, Column, Row, Space};
use iced::{Color, Length};
use serde_json::Value;

const P: Part = Part::Pcs;

pub const PAGE: PageDef = PageDef {
    id: "srv-pcs",
    title: "PCs",
    blocks: &[
        bl("head", "PCs and Add a PC", 0, 0, 12, 1),
        bl("cards", "Each PC", 0, 1, 12, 4),
        bl("share", "Each PC's share", 0, 5, 7, 4),
        bl("off", "If a PC goes off", 7, 5, 5, 4),
        bl("week", "The last 7 days", 0, 9, 12, 3),
    ],
};
pub const PANEL: PageDef = PageDef {
    id: "srv-pcs-side",
    title: "PCs' right side",
    blocks: &[bl("add", "Add a PC", 0, 0, 12, 4), bl("limits", "What each PC may do", 0, 4, 12, 3), bl("moving", "Moving things", 0, 7, 12, 3), bl("store", "Hearth Server", 0, 10, 12, 2)],
};

fn mood(m: &str) -> Color {
    match m {
        "blue" => kit::BLUE,
        "green" => kit::GREEN,
        _ => kit::rgb(0xff, 0x8a, 0x3d),
    }
}

fn mark<'a>(c: &Value) -> E<'a, Msg> {
    let (a, b) = match s(c, "mood").as_str() {
        "blue" => (kit::rgb(0x7f, 0xd6, 0xff), kit::rgb(0x2c, 0x7f, 0xd8)),
        "green" => (kit::rgb(0xc9, 0xc1, 0xba), kit::rgb(0x6d, 0x64, 0x5d)),
        _ => (kit::rgb(0xff, 0xb3, 0x6b), kit::rgb(0xff, 0x6a, 0x1f)),
    };
    container(tb(s(c, "mark"), 15.0, Color::WHITE)).center_x(Length::Fixed(44.0)).center_y(Length::Fixed(44.0)).style(kit::gradient(135.0, a, b, 10.0, None)).into()
}

fn card<'a>(p: Pal, c: &'a Value) -> E<'a, Msg> {
    let main = on(c, "main");
    let tg = s(c, "tag");
    let tag = match tg.as_str() {
        "main" => kit::tag(&p, tg.clone(), Tag::Acc),
        "over Tailscale" => kit::tag(&p, tg.clone(), Tag::Blue),
        _ => kit::tag(&p, tg.clone(), Tag::Dark),
    };
    let mut col = Column::new()
        .spacing(6.0)
        .push(row![mark(c), column![ts(s(c, "name"), 18.0, p.text), t(s(c, "spec"), 12.0, p.muted)].spacing(2.0).width(Length::Fill), tag].spacing(12.0).align_y(iced::Center))
        .push(line(row![crate::st(if on(c, "online") { "" } else { "o" }), t(s(c, "health"), 13.0, p.text)].spacing(6.0).align_y(iced::Center).into(), t(s(c, "right"), 12.0, p.muted).into()));
    for b in list(c, "bars") {
        let label = sv(at(b, "/0"));
        let v = num(at(b, "/1"));
        let right = if at(b, "/2").is_null() { format!("{}%", v.round()) } else { sv(at(b, "/2")) };
        let colour = if label == "Battery" { kit::GREEN } else { p.accent };
        col = col.push(row![container(t(label, 12.0, p.muted)).width(Length::Fixed(70.0)), container(ui::bar(vec![((v / 100.0).clamp(0.0, 1.0), colour)], p.line, 6.0, 3.0)).width(Length::Fill), container(t(right, 12.0, p.muted)).width(Length::Fixed(44.0)).align_x(iced::alignment::Horizontal::Right)].spacing(10.0).align_y(iced::Center));
    }
    col = col.push(container(kit::caps(&p, "Doing")).padding(pad(6.0, 0.0, 0.0, 0.0)));
    for j in list(c, "jobs") {
        let on_ = at(j, "/on").as_bool().unwrap_or(true);
        col = col.push(
            container(row![crate::st(if on_ { "" } else { "o" }), t(s(j, "name"), 13.0, if on_ { p.text } else { p.muted }).width(Length::Fill), t(s(j, "kind"), 12.0, p.muted)].spacing(8.0).align_y(iced::Center))
                .padding(pad(7.0, 10.0, 7.0, 10.0))
                .width(Length::Fill)
                .style(ui::boxed(p.panel2, 8.0, Some((p.line, 1.0)))),
        );
    }
    let name = s(c, "name");
    let (bg, ln, w) = if main { (kit::rgb(0x2e, 0x22, 0x18), p.accent, 2.0) } else { (p.panel, p.line, 1.0) };
    kit::click(container(col).padding(16.0).width(Length::Fill).style(ui::boxed(bg, 14.0, Some((ln, w)))), Some(Msg::Set(P, "sel", serde_json::Value::String(name).to_string())))
}

fn share<'a>(p: Pal, v: &'a Value) -> E<'a, Msg> {
    let legend: Vec<E<'a, Msg>> = list(v, "legend").iter().map(|l| row![t("■", 12.0, mood(&sv(at(l, "/1")))), t(sv(at(l, "/0")), 12.0, p.muted)].spacing(4.0).into()).collect();
    let moods: Vec<String> = list(v, "cards").iter().map(|c| s(c, "mood")).collect();
    let mut c = Column::new().spacing(8.0).push(kit::caps(&p, "Each PC's share")).push(Row::with_children(legend).spacing(12.0));
    for r in list(v, "share") {
        let mut bar = Row::new();
        let cells = at(r, "/1").as_array().map(Vec::as_slice).unwrap_or(&[]);
        let most = cells.iter().map(|c| num(at(c, "/2"))).fold(1.0f32, f32::max);
        for cell in cells {
            let i = at(cell, "/0").as_u64().unwrap_or(0) as usize;
            let w = (num(at(cell, "/2")).max(0.0) / most * 1000.0).max(40.0) as u16;
            let col = mood(moods.get(i).map(String::as_str).unwrap_or(""));
            bar = bar.push(container(tb(sv(at(cell, "/1")), 11.0, kit::rgb(0x1a, 0x12, 0x0c))).center_x(Length::FillPortion(w.max(1))).center_y(Length::Fixed(24.0)).style(ui::boxed(col, 0.0, None)));
        }
        c = c.push(row![container(t(sv(at(r, "/0")), 13.0, p.text)).width(Length::Fixed(150.0)), container(bar).width(Length::Fill).clip(true).style(ui::boxed(p.line, 4.0, None))].spacing(8.0).align_y(iced::Center));
    }
    c = c.push(t("Each PC runs whole models: one model isn't split over the internet (too slow). Dad's PC takes the small ones, so the Nvidia PC keeps its 12 GB for Colibri.", 12.0, p.muted).line_height(1.5));
    kit::card(&p, c).into()
}

fn off<'a>(p: Pal, v: &'a Value) -> E<'a, Msg> {
    let sel = s(v, "sel");
    let names: Vec<(String, bool, Option<Msg>)> = list(v, "names")
        .iter()
        .map(|n| {
            let x = sv(n);
            (x.clone(), x == sel, Some(Msg::Set(P, "sel", serde_json::Value::String(x.clone()).to_string())))
        })
        .collect();
    let mut c = Column::new().spacing(0.0).push(row![kit::caps(&p, "If a PC goes off"), Space::new().width(Length::Fill), kit::seg(&p, names)].align_y(iced::Center));
    for m in list(v, "moves") {
        let icon: E<'a, Msg> = match sv(at(m, "/0")).as_str() {
            "arrow" => t("→", 14.0, p.accent).into(),
            "wait" => crate::st("w"),
            "off" => crate::st("o"),
            _ => crate::st(""),
        };
        c = c.push(container(row![container(icon).width(Length::Fixed(16.0)), t(sv(at(m, "/1")), 13.0, p.text).width(Length::Fill), t(sv(at(m, "/2")), 12.0, p.muted)].spacing(8.0).align_y(iced::Center)).padding(pad(8.0, 0.0, 8.0, 0.0))).push(ui::hline(crate::ROWLINE));
    }
    // What runs here now for a PC that went off, with Move it back (when you chose Ask).
    for w in list(v, "standin") {
        let id = s(w, "id");
        c = c.push(line(t(format!("{} · since {}", s(w, "line"), s(w, "since")), 12.0, p.text).into(), kit::click(t("Move it back", 12.0, p.accent), Some(crate::act(P, &["pcs-back", &id], "")))));
    }
    let tried = s(v, "tried");
    if !tried.is_empty() {
        c = c.push(
            container(t(format!("Tried {}: {}", s(v, "triedWhen"), tried), 12.0, p.text).line_height(1.5))
                .padding(pad(12.0, 16.0, 12.0, 16.0))
                .width(Length::Fill)
                .style(ui::boxed(kit::rgb(0x18, 0x23, 0x1a), 12.0, Some((kit::rgb(0x3b, 0x5a, 0x2e), 1.0)))),
        );
    }
    kit::card(&p, c.spacing(4.0)).into()
}

fn week<'a>(h: &'a Hub, p: Pal, v: &'a Value) -> E<'a, Msg> {
    let mut c = Column::new().spacing(6.0).push(row![kit::caps(&p, "The last 7 days"), Space::new().width(Length::Fill), t("on · off · moves", 12.0, p.muted)].align_y(iced::Center));
    for w in list(v, "week") {
        let col = mood(&s(w, "mood"));
        let mut bar = Row::new();
        for seg in list(w, "segments") {
            let fp = (num(at(seg, "/w")) * 1000.0).max(1.0) as u16;
            let c_ = if on(seg, "on") { col } else { kit::rgb(0x3a, 0x32, 0x2c) };
            bar = bar.push(container(Space::new()).width(Length::FillPortion(fp)).height(Length::Fixed(12.0)).style(ui::boxed(c_, 0.0, None)));
        }
        c = c.push(row![container(t(s(w, "name"), 12.0, p.text)).width(Length::Fixed(130.0)), container(bar).width(Length::Fill).clip(true).style(ui::boxed(p.line, 6.0, None)), container(t(s(w, "note"), 12.0, p.muted)).width(Length::Fixed(210.0)).align_x(iced::alignment::Horizontal::Right)].spacing(12.0).align_y(iced::Center));
    }
    c = c.push(row![t(format!("{} · ", s(v, "weekNote")), 12.0, p.muted), kit::click(t("See every move", 12.0, p.accent), Some(Msg::Pick("moves")))]);
    if h.pick == "moves" {
        let mut ml = Column::new().spacing(0.0);
        for m in list(v, "moveList") {
            ml = ml.push(line(t(s(m, "what"), 12.0, p.text).into(), t(format!("{} · {}", s(m, "when"), s(m, "why")), 12.0, p.muted).into()));
        }
        if list(v, "moveList").is_empty() {
            ml = ml.push(t("Nothing moved in the last 7 days.", 12.0, p.muted));
        }
        c = c.push(boxed(p, ml));
    }
    kit::card(&p, c).into()
}

fn side<'a>(h: &'a Hub, d: &'a Describe, p: Pal) -> E<'a, Msg> {
    let v = h.view_of(P);
    let mut found = Column::new().spacing(0.0).push(tb("Found on your network", 12.0, p.text));
    for f in list(v, "found") {
        let name = sv(at(f, "/0"));
        let what = sv(at(f, "/1"));
        let right: E<'a, Msg> = if what == "add" { kit::chip(&p, "Add", Some(crate::act(P, &["pcs-add", &name], "Pair it in Hearth Link, then it shows here:"))) } else { t(what, 12.0, p.muted).into() };
        found = found.push(line(t(name, 12.0, p.text).into(), right));
    }
    let add = column![ts("Add a PC", 18.0, p.text), t("Any PC with Hearth OS, at home or over Tailscale. Install Hearth Server on it from the Hearth Store, then pick it here.", 12.0, p.muted).line_height(1.5), boxed(p, found)].spacing(8.0);
    let l = at(v, "/limits");
    let dad = at(l, "/dadMax").as_i64().unwrap_or(40);
    let segs: Vec<(String, bool, Option<Msg>)> = [20, 40, 60].iter().map(|n| (format!("{n}%"), dad == *n, Some(Msg::Set(P, "dadMax", n.to_string())))).collect();
    let limits = column![
        kit::caps(&p, "What each PC may do"),
        line(t("Dad's PC gives at most", 13.0, p.text).into(), kit::seg(&p, segs)),
        line(t("Only when Dad isn't playing", 13.0, p.text).into(), sw(p, P, "onlyIdle", on(l, "onlyIdle"))),
        line(t("Never touches Dad's files", 13.0, p.text).into(), kit::tag(&p, "Always", Tag::Ok)),
        line(t("Old laptop: always on", 13.0, p.text).into(), sw(p, P, "laptopOn", on(l, "laptopOn"))),
    ]
    .spacing(2.0);
    let back = s(l, "moveBack");
    let moving = column![
        kit::caps(&p, "Moving things"),
        line(t("Move work by itself when one's busy", 13.0, p.text).into(), sw(p, P, "autoMove", on(l, "autoMove"))),
        line(t("Tell me each time something moves", 13.0, p.text).into(), sw(p, P, "tell", on(l, "tell"))),
        line(t("Move it back when it's free", 13.0, p.text).into(), kit::seg(&p, vec![("Ask".into(), back == "ask", Some(Msg::Set(P, "moveBack", "\"ask\"".into()))), ("Always".into(), back == "always", Some(Msg::Set(P, "moveBack", "\"always\"".into())))])),
        // On a PC that helps (Dad's PC, the old laptop): it takes whole jobs from your other PCs only with this on.
        line(crate::two(p, "Let this PC take jobs".into(), "game servers from your other PCs, whole".into()), sw(p, P, "helper", on(v, "helper"))),
    ]
    .spacing(2.0);
    let store = crate::store_box(p, row![tb("Hearth Server", 12.0, p.text), t(" on each PC · its own Store app", 12.0, p.text)].into(), "Updates in the Hearth Store, on every PC at once or one at a time.", false, Msg::Pick("add"));
    let items: Vec<(&'static str, E<'a, Msg>)> = vec![("add", add.into()), ("limits", limits.into()), ("moving", moving.into()), ("store", store)];
    panel(p, hearth_ui::arrange::parts(d, &p, "srv-pcs-side", items, |ps| column![ps.take("add"), ps.take("limits"), ps.take("moving"), ps.take("store")].spacing(18.0).into(), Msg::D))
}

pub fn view<'a>(h: &'a Hub, d: &'a Describe, p: Pal) -> E<'a, Msg> {
    let v = h.view_of(P);
    let hd = head(h, p, "PCs", "Add another PC and Hearth spreads the work across them: big AI models, storage, game servers and backups. You still see one server.".into(), vec![
        kit::chip(&p, "How it spreads the work", Some(Msg::Pick("how"))),
        kit::chip_acc(&p, "＋ Add a PC", Some(Msg::Pick("add"))),
    ]);
    let cards: Vec<E<'a, Msg>> = list(v, "cards").iter().map(|c| card(p, c)).collect();
    let mut hd_col = Column::new().spacing(10.0).push(hd);
    if h.pick == "how" {
        hd_col = hd_col.push(boxed(p, t("Each kind of work goes, whole, to the PC that suits it: the big AI model to the PC with the most graphics memory, small models and voices to the next, storage by free space, game servers where the players are, a second backup copy away from home, and the always-on services to the PC that's never off. When a PC goes off, its work moves to the main PC (a game's world saved first) and comes back when it's on again. Nothing is split over the internet.", 12.0, p.muted).line_height(1.5)));
    }
    let items: Vec<(&'static str, E<'a, Msg>)> = vec![("head", hd_col.into()), ("cards", crate::cols(cards, 14.0)), ("share", share(p, v)), ("off", off(p, v)), ("week", week(h, p, v))];
    let main = hearth_ui::arrange::parts(d, &p, "srv-pcs", items, |ps| {
        iced::widget::scrollable(
            column![
                ps.take("head"),
                ps.take("cards"),
                row![container(ps.take("share")).width(Length::FillPortion(13)), container(ps.take("off")).width(Length::FillPortion(10))].spacing(16.0),
                ps.take("week"),
            ]
            .spacing(16.0),
        )
        .height(Length::Fill)
        .into()
    }, Msg::D);
    row![container(main).width(Length::Fill).height(Length::Fill).padding(pad(18.0, 22.0, 18.0, 22.0)), ui::vline(p.line), side(h, d, p)].height(Length::Fill).into()
}
