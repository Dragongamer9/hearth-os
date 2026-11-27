//! Remote play (board docs/mockups/server-expansion/08-remote-play.png; checklist server-remote-play.md): play any
//! game on the Nvidia PC from the TV, the phone, the Ally or the truck laptop. Sunshine on the PC (hearth-remote-desktop
//! keeps it to Tailscale and pairs Moonlight), Moonlight on each device, quality for each device, who's playing,
//! waking the PC (Wake-on-LAN) and what the PC does while someone plays.

use crate::kit::{self, pad, Tag, E};
use crate::{act, at, boxed, head, line, line_chart, list, meter, num, on, panel, s, sv, sw, Hub, Msg, Part};
use hearth_ui::blocks::{bl, PageDef};
use hearth_ui::describe::Describe;
use hearth_ui::ui::{self, t, tb, ts};
use hearth_ui::Pal;
use iced::widget::{column, container, row, text_input, Column, Row, Space};
use iced::{Color, Length};
use serde_json::Value;

const P: Part = Part::Play;
const SIZES: [&str; 5] = ["4K", "1440p", "1080p", "900p", "720p"];
const FPS: [&str; 4] = ["30", "60", "90", "120"];
const RATES: [&str; 9] = ["5", "8", "10", "15", "20", "30", "40", "60", "80"];

pub const PAGE: PageDef = PageDef {
    id: "srv-play",
    title: "Remote play",
    blocks: &[
        bl("head", "Remote play and its buttons", 0, 0, 12, 1),
        bl("now", "Playing now", 0, 1, 6, 4),
        bl("pc", "The PC while streaming", 6, 1, 6, 4),
        bl("devices", "Devices, quality for each", 0, 5, 12, 3),
        bl("library", "Any game on the PC", 0, 8, 12, 4),
    ],
};
pub const PANEL: PageDef = PageDef {
    id: "srv-play-side",
    title: "Remote play's right side",
    blocks: &[bl("who", "Who's playing", 0, 0, 12, 3), bl("wake", "Wake the PC", 0, 3, 12, 3), bl("week", "Played this week", 0, 6, 12, 3), bl("store", "Sunshine and Moonlight", 0, 9, 12, 3)],
};

fn now<'a>(h: &'a Hub, p: Pal, n: &'a Value) -> E<'a, Msg> {
    let live = on(n, "on");
    let tags = row![
        container(row![kit::dot(Color::WHITE, 7.0), tb("LIVE", 11.0, Color::WHITE)].spacing(5.0).align_y(iced::Center)).padding(pad(3.0, 8.0, 3.0, 8.0)).style(ui::boxed(kit::rgb(0xe0, 0x3a, 0x3a), 6.0, None)),
        kit::tag(&p, if live { "Playing now" } else { "Nothing playing" }, Tag::Dark)
    ]
    .spacing(8.0);
    let mut btns = Row::new().spacing(8.0);
    if live {
        let typed = h.typed.trim().to_string();
        if h.pick == "message" {
            btns = btns.push(text_input("Say something on the stream", &h.typed).on_input(Msg::Typed).size(12.0).width(Length::Fixed(200.0)).padding(pad(5.0, 8.0, 5.0, 8.0)).style(kit::field_style(&p)));
            btns = btns.push(kit::chip_acc(&p, "Send", if typed.is_empty() { None } else { Some(act(P, &["play-message", &typed], "Sent: it shows on the stream.")) }));
        } else {
            btns = btns.push(kit::chip(&p, "Send a message", Some(Msg::Pick("message"))));
        }
        btns = btns.push(kit::chip(&p, "Stop the stream", Some(act(P, &["play-stop"], "The stream is stopped."))));
    }
    let bottom = column![
        tb(s(n, "title"), 26.0, Color::WHITE),
        row![t(s(n, "line"), 13.0, Color::from_rgba8(255, 255, 255, 0.85)), Space::new().width(Length::Fill), btns].align_y(iced::Center)
    ]
    .spacing(8.0);
    let words = container(column![tags, Space::new().height(Length::Fill), bottom]).padding(pad(16.0, 18.0, 16.0, 18.0)).width(Length::Fill).height(Length::Fixed(300.0));
    // The stream's own picture (a small frame grab of this PC's screen while it streams), darkened under the words.
    let shot = s(n, "shot");
    if live && !shot.is_empty() && std::path::Path::new(&shot).exists() {
        let pic = iced::widget::image(iced::widget::image::Handle::from_path(shot)).content_fit(iced::ContentFit::Cover).width(Length::Fill).height(Length::Fixed(300.0));
        let shade = container(Space::new()).width(Length::Fill).height(Length::Fixed(300.0)).style(kit::gradient_n(vec![Color::from_rgba8(0, 0, 0, 0.15), Color::from_rgba8(0, 0, 0, 0.7)], 14.0));
        return container(iced::widget::stack![pic, shade, words]).clip(true).width(Length::Fill).height(Length::Fixed(300.0)).style(ui::boxed(kit::rgb(0x1a, 0x12, 0x0c), 14.0, None)).into();
    }
    words.style(kit::gradient_n(vec![kit::rgb(0x3a, 0x2a, 0x20), kit::rgb(0x2a, 0x1d, 0x15), kit::rgb(0x55, 0x38, 0x22), kit::rgb(0x1a, 0x12, 0x0c)], 14.0)).into()
}

fn pc<'a>(p: Pal, v: &'a Value) -> E<'a, Msg> {
    let c = at(v, "/pc");
    let r = at(v, "/rules");
    let pct = |x: &Value| -> (f32, String) {
        if x.is_null() { (0.0, "—".into()) } else { (num(x) / 100.0, format!("{}%", sv(x))) }
    };
    let (e, es) = pct(at(c, "/enc"));
    let (g, gs) = pct(at(c, "/gpu"));
    let (o, os) = pct(at(c, "/ontime"));
    let pts: Vec<f32> = list(c, "delays").iter().map(num).collect();
    kit::card(
        &p,
        column![
            kit::caps(&p, "The Nvidia PC while streaming"),
            meter(p, "Video encoder (NVENC)".into(), e, p.accent, es, 200.0),
            meter(p, "GPU".into(), g, p.accent, gs, 160.0),
            meter(p, if s(c, "ontimeWords").is_empty() { "Frames sent on time".into() } else { format!("Frames sent on time · {}", s(c, "ontimeWords")) }, o, kit::GREEN, os, 200.0),
            line_chart(pts, kit::GREEN, p.line, 60.0),
            t(s(c, "delayWords"), 12.0, p.muted),
            container(kit::caps(&p, "While someone's playing")).padding(pad(8.0, 0.0, 0.0, 0.0)),
            line(t("Stop Colibri and Imagine to free the GPU", 13.0, p.text).into(), sw(p, P, "freeGpu", on(r, "freeGpu"))),
            line(t("Keep the game servers running", 13.0, p.text).into(), sw(p, P, "keepGames", on(r, "keepGames"))),
            line(t("Pause the Model vault upload", 13.0, p.text).into(), sw(p, P, "pauseVault", on(r, "pauseVault"))),
        ]
        .spacing(4.0),
    )
    .into()
}

fn next(list_: &[&str], now: &str) -> String {
    let base = now.split(' ').next().unwrap_or(now);
    let i = list_.iter().position(|x| *x == base).map(|i| (i + 1) % list_.len()).unwrap_or(0);
    list_[i].to_string()
}

fn device<'a>(p: Pal, d: &'a Value) -> E<'a, Msg> {
    let id = s(d, "id");
    let state = s(d, "state");
    let playing = state == "playing";
    let tag = if playing { kit::tag(&p, state.clone(), Tag::Ok) } else { kit::tag(&p, state.clone(), Tag::Dark) };
    let mut c = Column::new()
        .spacing(0.0)
        .push(row![crate::st(if playing { "" } else { "o" }), ts(s(d, "name"), 15.0, p.text), Space::new().width(Length::Fill), tag].spacing(8.0).align_y(iced::Center))
        .push(container(t(s(d, "via"), 12.0, p.muted)).padding(pad(2.0, 0.0, 8.0, 0.0)));
    for r in list(d, "rows") {
        let k = sv(at(r, "/0"));
        let val = sv(at(r, "/1"));
        let m = match k.as_str() {
            "Size" => Some(act(P, &["play-device", &id, "size", &next(&SIZES, &val)], "Kept for that device.")),
            "Frames" => Some(act(P, &["play-device", &id, "fps", &next(&FPS, &val)], "Kept for that device.")),
            "Bit rate" => Some(act(P, &["play-device", &id, "bitrate", &next(&RATES, &val)], "Kept for that device.")),
            _ => None,
        };
        let right = kit::click(t(val, 12.0, p.text), m);
        c = c.push(ui::hline(crate::ROWLINE)).push(container(row![t(k, 12.0, p.muted), Space::new().width(Length::Fill), right].align_y(iced::Center)).padding(pad(5.0, 0.0, 5.0, 0.0)));
    }
    let (bg, ln, w) = if playing { (kit::rgb(0x2e, 0x22, 0x18), p.accent, 2.0) } else { (p.panel, p.line, 1.0) };
    container(c).padding(14.0).width(Length::Fill).style(ui::boxed(bg, 12.0, Some((ln, w)))).into()
}

fn library<'a>(h: &'a Hub, p: Pal, v: &'a Value) -> E<'a, Msg> {
    let tabs = ["Played lately", "Steam", "Your games", "Retro", "The whole desktop"];
    let mut tr = Row::new().spacing(6.0);
    for x in tabs {
        tr = tr.push(if h.ptab == x { kit::chip_acc(&p, x, Some(Msg::PTab(x))) } else { kit::chip(&p, x, Some(Msg::PTab(x))) });
    }
    let want = match h.ptab {
        "Steam" => "steam",
        "Your games" => "own",
        "Retro" => "retro",
        "The whole desktop" => "desktop",
        _ => "",
    };
    let games: Vec<&Value> = list(v, "games").iter().filter(|g| want.is_empty() || s(g, "kind") == want).take(6).collect();
    let mut grid = Row::new().spacing(10.0);
    for g in &games {
        let gid = s(g, "id");
        let cover = container(column![Space::new().height(Length::Fill), tb(s(g, "name"), 17.0, Color::WHITE)])
            .padding(pad(12.0, 14.0, 12.0, 14.0))
            .width(Length::Fill)
            .height(Length::Fixed(150.0))
            .style(kit::gradient(160.0, crate::colour(at(g, "/art/0"), kit::rgb(0x4a, 0x3a, 0x30)), crate::colour(at(g, "/art/1"), kit::rgb(0x14, 0x10, 0x0c)), 10.0, None));
        let target = at(v, "/target");
        let tname = if target.is_null() { "a device".to_string() } else { s(target, "name") };
        let tid = s(target, "id");
        let play_m = if tid.is_empty() { act(P, &["play-launch", &gid], "It's in Moonlight's list on every device: pick it there.") } else { act(P, &["play-open", &tid, &gid], "It's in Moonlight's list.") };
        let picks = row![kit::chip_acc(&p, "Play", Some(play_m)), kit::chip(&p, format!("on the {tname} ▾"), Some(Msg::PGame(gid.clone())))].spacing(6.0);
        let mut cell = column![cover, picks].spacing(8.0);
        if h.pgame == gid {
            // Start it on a picked device: a Hearth device's Moonlight opens it by itself; another opens it from its list.
            let mut pick = Column::new().spacing(4.0);
            for d in list(v, "devices") {
                let did = s(d, "id");
                pick = pick.push(kit::chip(&p, format!("{} · {}", s(d, "name"), s(d, "state")), Some(act(P, &["play-open", &did, &gid], "It's in Moonlight's list."))));
            }
            cell = cell.push(pick);
        }
        grid = grid.push(container(cell).width(Length::FillPortion(1)));
    }
    for _ in games.len()..6 {
        grid = grid.push(Space::new().width(Length::FillPortion(1)));
    }
    column![row![kit::caps(&p, format!("Any game on the PC · {}", sv(at(v, "/gamesCount")))), Space::new().width(Length::Fill), tr].align_y(iced::Center), grid].spacing(10.0).into()
}

fn side<'a>(h: &'a Hub, d: &'a Describe, p: Pal) -> E<'a, Msg> {
    let v = h.view_of(P);
    let w = at(v, "/who");
    let mut who = Column::new().spacing(8.0).push(ts("Who's playing", 18.0, p.text));
    who = who.push(boxed(p, column![row![crate::st(if on(w, "on") { "" } else { "o" }), tb(s(w, "title"), 12.0, p.text), Space::new().width(Length::Fill), t(s(w, "time"), 12.0, p.muted)].spacing(8.0).align_y(iced::Center), t(s(w, "line"), 12.0, p.muted)].spacing(4.0)));
    who = who.push(t("One stream at a time on this PC. Another device asks first: \"Take over from the TV?\"", 12.0, p.muted).line_height(1.5));
    if h.pick == "pair" {
        let pin = h.typed.trim().to_string();
        who = who.push(row![text_input("Moonlight's 4 numbers", &h.typed).on_input(Msg::Typed).size(12.0).padding(pad(5.0, 8.0, 5.0, 8.0)).style(kit::field_style(&p)), kit::chip_acc(&p, "Pair", Some(act(P, &["play-pin", &pin], "Paired.")))].spacing(6.0));
    }
    let k = at(v, "/wake");
    let wake = column![
        kit::caps(&p, "Wake the PC"),
        line(crate::two(p, "Wake it from any device".into(), "Moonlight's wake button · or the phone's Home server page".into()), sw(p, P, "wakeAny", on(k, "any"))),
        line(t("Wake it from away, over Tailscale", 13.0, p.text).into(), sw(p, P, "wakeAway", on(k, "away"))),
        line(t("Let it sleep after 30 min with nobody", 13.0, p.text).into(), sw(p, P, "sleepAfter", on(k, "sleep"))),
        t(s(k, "sleepNote"), 11.0, p.muted),
    ]
    .spacing(2.0);
    let mut week = Column::new().spacing(0.0).push(container(kit::caps(&p, "Played this week")).padding(pad(0.0, 0.0, 4.0, 0.0)));
    for x in list(v, "week") {
        week = week.push(
            row![container(t(s(x, "name"), 12.0, p.text)).width(Length::Fill), container(ui::bar(vec![(num(at(x, "/share")).clamp(0.0, 1.0), p.accent)], p.line, 4.0, 2.0)).width(Length::Fixed(120.0)), container(t(s(x, "words"), 12.0, p.muted)).width(Length::Fixed(56.0)).align_x(iced::alignment::Horizontal::Right)]
                .spacing(10.0)
                .align_y(iced::Center)
                .padding(pad(3.0, 0.0, 3.0, 0.0)),
        );
    }
    let store = column![
        crate::store_box(p, row![tb("Sunshine", 12.0, p.text), t(" on the PC · ", 12.0, p.text), tb("Moonlight", 12.0, p.text), t(" on each device", 12.0, p.text)].into(), "Both from the Hearth Store, with their updates there. Sunshine needs a system part to capture the screen.", on(v, "installed"), act(P, &["play-setup"], "Sunshine is ready.")),
        t("For the Xbox itself, Hearth Console Remote Play is its own app.", 12.0, p.muted).line_height(1.5)
    ]
    .spacing(14.0);
    let items: Vec<(&'static str, E<'a, Msg>)> = vec![("who", who.into()), ("wake", wake.into()), ("week", week.into()), ("store", store.into())];
    panel(p, hearth_ui::arrange::parts(d, &p, "srv-play-side", items, |ps| column![ps.take("who"), ps.take("wake"), ps.take("week"), ps.take("store")].spacing(18.0).into(), Msg::D))
}

pub fn view<'a>(h: &'a Hub, d: &'a Describe, p: Pal) -> E<'a, Msg> {
    let v = h.view_of(P);
    let hd = head(h, p, "Remote play", "Play any game on the Nvidia PC from the TV, the phone, the Ally or the truck laptop · Sunshine on the PC, Moonlight on each device".into(), vec![
        kit::chip(&p, "Pair a device", Some(Msg::Pick("pair"))),
        kit::chip_acc(&p, "Wake the PC", Some(act(P, &["play-wake"], "Woken: it's up in a few seconds."))),
    ]);
    let devs: Vec<E<'a, Msg>> = list(v, "devices").iter().map(|x| device(p, x)).collect();
    let items: Vec<(&'static str, E<'a, Msg>)> = vec![
        ("head", hd),
        ("now", now(h, p, at(v, "/now"))),
        ("pc", pc(p, v)),
        ("devices", column![kit::caps(&p, "Devices · quality for each"), crate::cols(devs, 12.0)].spacing(10.0).into()),
        ("library", library(h, p, v)),
    ];
    let main = hearth_ui::arrange::parts(d, &p, "srv-play", items, |ps| {
        iced::widget::scrollable(
            column![
                ps.take("head"),
                row![container(ps.take("now")).width(Length::FillPortion(11)), container(ps.take("pc")).width(Length::FillPortion(10))].spacing(16.0),
                ps.take("devices"),
                ps.take("library"),
                kit::credit(&p, "Remote play", "Sunshine (GPL-3) and Moonlight (GPL-3)"),
            ]
            .spacing(16.0),
        )
        .height(Length::Fill)
        .into()
    }, Msg::D);
    row![container(main).width(Length::Fill).height(Length::Fill).padding(pad(18.0, 22.0, 18.0, 22.0)), ui::vline(p.line), side(h, d, p)].height(Length::Fill).into()
}
