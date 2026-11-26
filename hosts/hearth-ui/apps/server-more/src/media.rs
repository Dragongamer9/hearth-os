//! Media (board docs/mockups/server-expansion/09-media.png; checklist server-media.md): TV, movies, music and photos
//! in one place, sorted, named and given their art by themselves, then into Jellyfin. Only from things you own or
//! that are free: your discs, the antenna (Jellyfin's Live TV), your own videos and phone, public-domain films (the
//! Internet Archive) and a folder you drop files in. Nothing else, ever.

use crate::kit::{self, pad, Tag, E};
use crate::{act, at, boxed, head, line, list, num, on, panel, s, sv, sw, Hub, Msg, Part};
use hearth_ui::blocks::{bl, PageDef};
use hearth_ui::describe::Describe;
use hearth_ui::ui::{self, t, tb, ts};
use hearth_ui::Pal;
use iced::widget::{column, container, row, text_input, Column, Row, Space};
use iced::Length;
use serde_json::Value;

const P: Part = Part::Media;

pub const PAGE: PageDef = PageDef {
    id: "srv-media",
    title: "Media",
    blocks: &[
        bl("head", "Media, its tabs and Want something", 0, 0, 12, 1),
        bl("board", "Wanted, getting it, ready, in Jellyfin", 0, 1, 12, 5),
        bl("music", "Music", 0, 6, 5, 4),
        bl("photos", "Photos", 5, 6, 7, 4),
        bl("coming", "Watching now and coming up", 0, 10, 12, 2),
    ],
};
pub const PANEL: PageDef = PageDef {
    id: "srv-media-side",
    title: "Media's right side",
    blocks: &[bl("from", "Where it comes from", 0, 0, 12, 4), bl("sorting", "Sorting", 0, 4, 12, 3), bl("space", "Space", 0, 7, 12, 2), bl("store", "Its own Store app", 0, 9, 12, 3)],
};

fn poster<'a>(a: &Value) -> E<'a, Msg> {
    container(Space::new())
        .width(Length::Fixed(42.0))
        .height(Length::Fixed(58.0))
        .style(kit::gradient(160.0, crate::colour(at(a, "/0"), kit::rgb(0x2a, 0x3a, 0x5a)), crate::colour(at(a, "/1"), kit::rgb(0x10, 0x14, 0x1e)), 6.0, None))
        .into()
}

fn item<'a>(p: Pal, x: &'a Value, kind: &str) -> E<'a, Msg> {
    let mut c = Column::new().spacing(2.0).push(ts(s(x, "title"), 13.0, p.text));
    let what = s(x, "what");
    if !what.is_empty() {
        c = c.push(t(what, 11.0, p.muted));
    }
    c = c.push(t(s(x, "line"), 11.0, p.muted));
    if kind == "getting" {
        let live = on(x, "live");
        c = c.push(container(ui::bar(vec![(num(at(x, "/progress")).clamp(0.0, 1.0), if live { kit::RED } else { p.accent })], p.line, 4.0, 2.0)).padding(pad(4.0, 0.0, 2.0, 0.0)));
        c = c.push(t(s(x, "left"), 11.0, p.muted));
    }
    if kind == "ready" {
        let ask = s(x, "ask");
        let id = s(x, "id");
        if !ask.is_empty() {
            let guess = s(x, "guess");
            c = c.push(t(ask, 11.0, kit::YELLOW));
            c = c.push(row![kit::chip_acc(&p, "Yes", Some(act(P, &["media-yes", &id], "Going into Jellyfin."))), kit::chip(&p, "Pick", Some(act(P, &["media-pick", &id, &guess], "Going into Jellyfin.")))].spacing(6.0));
        }
        let l2 = s(x, "line2");
        if !l2.is_empty() {
            c = c.push(t(l2, 11.0, p.muted));
        }
    }
    container(row![poster(at(x, "/art")), c].spacing(10.0)).padding(8.0).width(Length::Fill).style(ui::boxed(p.panel2, 10.0, Some((p.line, 1.0)))).into()
}

fn column_of<'a>(p: Pal, title: &'a str, n: String, tag: Tag, inner: Vec<E<'a, Msg>>) -> E<'a, Msg> {
    let mut c = Column::new().spacing(10.0).push(row![ts(title, 13.0, p.text), kit::tag(&p, n, tag)].spacing(8.0).align_y(iced::Center));
    for x in inner {
        c = c.push(x);
    }
    container(c).padding(12.0).width(Length::Fill).height(Length::Fixed(320.0)).style(ui::boxed(p.panel, 12.0, Some((p.line, 1.0)))).into()
}

fn board<'a>(p: Pal, v: &'a Value) -> E<'a, Msg> {
    let w = at(v, "/wanted");
    let mut wanted: Vec<E<'a, Msg>> = list(w, "items").iter().map(|x| item(p, x, "wanted")).collect();
    let more = at(w, "/more").as_u64().unwrap_or(0);
    if more > 0 {
        wanted.push(row![t(format!("+ {more} more · "), 12.0, p.muted), kit::click(t("All wanted", 12.0, p.accent), Some(Msg::Pick("wanted")))].into());
    }
    let g = at(v, "/getting");
    let getting: Vec<E<'a, Msg>> = list(g, "items").iter().map(|x| item(p, x, "getting")).collect();
    let r = at(v, "/ready");
    let ready: Vec<E<'a, Msg>> = list(r, "items").iter().map(|x| item(p, x, "ready")).collect();
    let lib = at(v, "/library");
    let mut l: Vec<E<'a, Msg>> = list(lib, "rows").iter().map(|x| line(t(sv(at(x, "/0")), 13.0, p.text).into(), t(sv(at(x, "/1")), 12.0, p.muted).into())).collect();
    l.push(kit::caps(&p, "Added this week"));
    let mut posters = Row::new().spacing(8.0);
    let tones = [("#2a3a5a", "#10141e"), ("#6a3a2a", "#1e100c"), ("#2a5a5a", "#0c1a1a"), ("#5a5a2a", "#1a1a0c")];
    for (i, _n) in list(lib, "latest").iter().enumerate() {
        let (a, b) = tones[i % 4];
        posters = posters.push(container(Space::new()).width(Length::Fixed(44.0)).height(Length::Fixed(64.0)).style(kit::gradient(160.0, kit::hex(a).unwrap_or(p.panel2), kit::hex(b).unwrap_or(p.bg), 6.0, None)));
    }
    l.push(posters.into());
    l.push(t("Watched on the TV, Ally and phone · picks up where you stopped", 12.0, p.muted).into());
    crate::cols(
        vec![
            column_of(p, "Wanted", sv(at(w, "/n")), Tag::Dark, wanted),
            column_of(p, "Getting it", sv(at(g, "/n")), Tag::Blue, getting),
            column_of(p, "Ready", sv(at(r, "/n")), Tag::Warn, ready),
            column_of(p, "In Jellyfin", s(lib, "n"), Tag::Ok, l),
        ],
        12.0,
    )
}

fn cover<'a>(a: &Value) -> E<'a, Msg> {
    container(Space::new()).width(Length::Fixed(36.0)).height(Length::Fixed(36.0)).style(kit::gradient(135.0, crate::colour(at(a, "/0"), kit::rgb(0xff, 0x9a, 0x4a)), crate::colour(at(a, "/1"), kit::rgb(0xc0, 0x50, 0x1a)), 6.0, None)).into()
}

fn music<'a>(p: Pal, v: &'a Value) -> E<'a, Msg> {
    let m = at(v, "/music");
    let mut c = Column::new().spacing(0.0).push(row![ts("Music", 18.0, p.text), Space::new().width(Length::Fill), t(s(m, "line"), 12.0, p.muted)].align_y(iced::Center));
    for r in list(m, "rows") {
        c = c.push(
            container(row![cover(at(r, "/3")), column![t(sv(at(r, "/0")), 13.0, p.text), t(sv(at(r, "/1")), 11.0, p.muted)].spacing(1.0).width(Length::Fill), t(sv(at(r, "/2")), 13.0, p.muted)].spacing(10.0).align_y(iced::Center))
                .padding(pad(8.0, 0.0, 8.0, 0.0)),
        )
        .push(ui::hline(crate::ROWLINE));
    }
    kit::card(&p, c).into()
}

fn photos<'a>(p: Pal, v: &'a Value) -> E<'a, Msg> {
    let ph = at(v, "/photos");
    let tag = if on(ph, "done") { kit::tag(&p, "iCloud move done", Tag::Ok) } else { kit::tag(&p, "iCloud move", Tag::Dark) };
    let tones = [
        ("#4a3a30", "#2a201a"), ("#2a4a6a", "#14202e"), ("#6a5a3a", "#2a2414"), ("#3a5a3a", "#142014"), ("#5a2a2a", "#201010"), ("#2a2a5a", "#10101e"),
        ("#5a5a2a", "#20200c"), ("#1a3a4a", "#0c1a20"), ("#4a2a5a", "#1a1020"), ("#3a3a2a", "#14140c"), ("#6a3a1a", "#2a140a"), ("#2a5a4a", "#0c201a"),
    ];
    let mut grid = Column::new().spacing(6.0);
    for rw in 0..2 {
        let mut r = Row::new().spacing(6.0);
        for i in 0..6 {
            let (a, b) = tones[rw * 6 + i];
            r = r.push(container(Space::new()).width(Length::FillPortion(1)).height(Length::Fixed(64.0)).style(kit::gradient(135.0, kit::hex(a).unwrap_or(p.panel2), kit::hex(b).unwrap_or(p.bg), 4.0, None)));
        }
        grid = grid.push(r);
    }
    let mut c = Column::new().spacing(8.0).push(row![ts("Photos", 18.0, p.text), Space::new().width(Length::Fill), tag, t(s(ph, "line"), 12.0, p.muted)].spacing(10.0).align_y(iced::Center)).push(grid);
    let mut rows = Column::new().spacing(0.0);
    for r in list(ph, "rows") {
        let link = sv(at(r, "/2"));
        let right: E<'a, Msg> = if link.is_empty() {
            t(sv(at(r, "/1")), 12.0, p.muted).into()
        } else {
            row![t(sv(at(r, "/1")), 12.0, p.muted), kit::click(t(link, 12.0, p.accent), Some(Msg::Open("hearth-settings://photos".into())))].into()
        };
        rows = rows.push(line(t(sv(at(r, "/0")), 13.0, p.text).into(), right));
    }
    c = c.push(rows);
    kit::card(&p, c).into()
}

fn coming<'a>(p: Pal, v: &'a Value) -> E<'a, Msg> {
    let mut boxes: Vec<E<'a, Msg>> = Vec::new();
    for w in list(v, "watching") {
        boxes.push(boxed(p, column![row![crate::st(""), tb(s(w, "title"), 12.0, p.text)].spacing(6.0).align_y(iced::Center), t(format!("{} · {}", s(w, "who"), s(w, "at")), 12.0, p.muted)].spacing(4.0)).into());
    }
    for x in list(v, "coming") {
        boxes.push(boxed(p, column![row![tb(s(x, "when"), 12.0, p.text), t(format!(" · {}", s(x, "what")), 12.0, p.text)], t(s(x, "line"), 12.0, p.muted)].spacing(4.0)).into());
    }
    let n = at(v, "/nightly");
    if !n.is_null() {
        boxes.push(boxed(p, column![row![tb("Every night", 12.0, p.text), t(format!(" · {}", s(n, "title")), 12.0, p.text)], t(s(n, "line"), 12.0, p.muted)].spacing(4.0)).into());
    }
    kit::card(&p, column![row![kit::caps(&p, "Watching now and coming up"), Space::new().width(Length::Fill), t("from the antenna and Jellyfin", 12.0, p.muted)].align_y(iced::Center), crate::cols(boxes, 10.0)].spacing(10.0)).into()
}

fn want_form<'a>(h: &'a Hub, p: Pal) -> E<'a, Msg> {
    let title = h.typed.trim().to_string();
    let mk = |kind: &str, from: &str| -> Msg {
        let j = serde_json::json!({"title": title, "kind": kind, "from": from}).to_string();
        act(P, &["media-want", &j], "Wanted: it's sorted in by itself.")
    };
    let ok = !title.is_empty();
    boxed(
        p,
        column![
            text_input("What do you want? A show, a film, an album", &h.typed).on_input(Msg::Typed).size(13.0).padding(pad(7.0, 10.0, 7.0, 10.0)).style(kit::field_style(&p)),
            row![
                kit::chip(&p, "A show from the antenna", if ok { Some(mk("tv", "antenna")) } else { None }),
                kit::chip(&p, "One of my discs", if ok { Some(mk("movie", "disc")) } else { None }),
                kit::chip(&p, "A free film (public domain)", if ok { Some(mk("movie", "free")) } else { None }),
                kit::chip(&p, "My own video", if ok { Some(mk("movie", "own")) } else { None }),
            ]
            .spacing(6.0)
        ]
        .spacing(8.0),
    )
    .into()
}

fn side<'a>(h: &'a Hub, d: &'a Describe, p: Pal) -> E<'a, Msg> {
    let v = h.view_of(P);
    let f = at(v, "/sources");
    let from = column![
        ts("Where it comes from", 18.0, p.text),
        t("Only from things you own or that are free", 12.0, p.muted),
        line(t("Discs · the Blu-ray drive", 13.0, p.text).into(), sw(p, P, "discs", on(f, "discs"))),
        line(crate::two(p, "The antenna · TV tuner".into(), "records shows you want, cuts the ads".into()), sw(p, P, "antenna", on(f, "antenna"))),
        line(t("Your own videos and phone", 13.0, p.text).into(), sw(p, P, "own", on(f, "own"))),
        line(t("Free and public domain", 13.0, p.text).into(), sw(p, P, "free", on(f, "free"))),
        line(t("A folder you drop files in", 13.0, p.text).into(), sw(p, P, "folder", on(f, "folder"))),
        row![kit::chip(&p, "Copy the disc in the drive", Some(act(P, &["media-rip"], "Copying your disc: it shows under Getting it.")))],
    ]
    .spacing(2.0);
    let so = at(v, "/sorting");
    let sorting = column![
        kit::caps(&p, "Sorting"),
        line(t("Name, art and subtitles by themselves", 13.0, p.text).into(), sw(p, P, "autoName", on(so, "autoName"))),
        line(t("Ask when it isn't sure", 13.0, p.text).into(), sw(p, P, "ask", on(so, "ask"))),
        line(t("Make smaller copies for the Ally", 13.0, p.text).into(), sw(p, P, "smaller", on(so, "smaller"))),
    ]
    .spacing(2.0);
    let sp = at(v, "/space");
    let space = column![
        kit::caps(&p, "Space"),
        boxed(p, column![tb(s(sp, "title"), 12.0, p.text), ui::bar(vec![(num(at(sp, "/used")).clamp(0.0, 1.0), p.accent)], p.line, 4.0, 2.0), t(s(sp, "line"), 12.0, p.muted)].spacing(8.0))
    ]
    .spacing(8.0);
    let store = crate::store_box(p, row![tb("Media", 12.0, p.text), t(" · its own Store app", 12.0, p.text)].into(), "Updates in the Hearth Store. The TV tuner and disc ripping need system parts.", on(v, "installed"), act(P, &["media-setup"], "Media's system parts are in."));
    let items: Vec<(&'static str, E<'a, Msg>)> = vec![("from", from.into()), ("sorting", sorting.into()), ("space", space.into()), ("store", store)];
    panel(p, hearth_ui::arrange::parts(d, &p, "srv-media-side", items, |ps| column![ps.take("from"), ps.take("sorting"), ps.take("space"), ps.take("store")].spacing(18.0).into(), Msg::D))
}

pub fn view<'a>(h: &'a Hub, d: &'a Describe, p: Pal) -> E<'a, Msg> {
    let v = h.view_of(P);
    let tab = s(v, "tab");
    let tabs = kit::seg(&p, vec![
        ("TV and movies".into(), tab == "tv" || tab.is_empty(), Some(Msg::Set(P, "tab", "\"tv\"".into()))),
        ("Music".into(), tab == "music", Some(Msg::Set(P, "tab", "\"music\"".into()))),
        ("Photos".into(), tab == "photos", Some(Msg::Set(P, "tab", "\"photos\"".into()))),
    ]);
    let hd = head(h, p, "Media", "TV, movies, music and photos in one place · sorted, named and given their art by themselves, then into Jellyfin".into(), vec![tabs, kit::chip_acc(&p, "＋ Want something", Some(Msg::Pick("want")))]);
    let top: E<'a, Msg> = match tab.as_str() {
        "music" => music(p, v),
        "photos" => photos(p, v),
        _ => board(p, v),
    };
    let top = if h.pick == "want" { column![want_form(h, p), top].spacing(12.0).into() } else { top };
    let items: Vec<(&'static str, E<'a, Msg>)> = vec![("head", hd), ("board", top), ("music", music(p, v)), ("photos", photos(p, v)), ("coming", coming(p, v))];
    let main = hearth_ui::arrange::parts(d, &p, "srv-media", items, |ps| {
        iced::widget::scrollable(
            column![
                ps.take("head"),
                ps.take("board"),
                row![container(ps.take("music")).width(Length::FillPortion(10)), container(ps.take("photos")).width(Length::FillPortion(12))].spacing(16.0),
                ps.take("coming"),
            ]
            .spacing(16.0),
        )
        .height(Length::Fill)
        .into()
    }, Msg::D);
    row![container(main).width(Length::Fill).height(Length::Fill).padding(pad(18.0, 22.0, 18.0, 22.0)), ui::vline(p.line), side(h, d, p)].height(Length::Fill).into()
}
