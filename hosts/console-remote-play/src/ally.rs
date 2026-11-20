//! Game Mode on the ROG Ally (mockups g1, g2, g3, g5, g6, g7a, g7b and g8, 1280 × 800; on a 1920 × 1080 Ally the same
//! layout at 16:9): the top bar with LB / RB tabs (Consoles · Cloud · Settings) and the gamertag, each page in large
//! rows, the controller hints, the streaming overlay and the quick menu that holding View + Menu for 1 s opens.

use crate::desk::{self, checklist, power_chip, power_color, step_row, steps, tile};
use crate::look::*;
use crate::{App, CloudTab, Dd, Load, Msg, Page};
use crp::data;
use crp::xhome::Power;
use iced::widget::{column, container, row, scrollable, stack, text_input, Column, Row, Space};
use iced::{Color, Length};
use serde_json::json;

const HINT: Color = Color { r: 0xc9 as f32 / 255.0, g: 0xcc as f32 / 255.0, b: 0xd3 as f32 / 255.0, a: 1.0 };

fn top<'a>(a: &'a App, tabs: bool) -> E<'a, Msg> {
    let mut r = Row::new().spacing(14.0).align_y(iced::Center).push(app_icon(36.0, 22.0, 10.0)).push(one(t6("Console Remote Play", 20.0, TX)));
    if tabs && a.signed_in {
        let cur = match a.page {
            Page::Cloud => 1,
            Page::Settings => 2,
            _ => 0,
        };
        let pill = |s: &str| -> E<'a, Msg> { container(one(t7(s.to_string(), 11.5, HINT))).center_x(Length::Fixed(34.0)).center_y(Length::Fixed(26.0)).style(bx(Color::TRANSPARENT, 13.0, Some((MUT, 2.0)))).into() };
        let mut tr = Row::new().spacing(6.0).align_y(iced::Center).push(gap(24.0, 1.0)).push(pill("LB"));
        let mut names = vec![("Consoles", 0, Page::Consoles)];
        if a.cloud {
            names.push(("Cloud", 1, Page::Cloud));
        }
        names.push(("Settings", 2, Page::Settings));
        for (n, i, p) in names {
            let on = i == cur;
            let lbl = container(one(t(n.to_string(), 17.0, if on { Color::WHITE } else { HINT }))).padding(pad(8.0, 16.0, 8.0, 16.0));
            let el: E<'a, Msg> = if on {
                container(column![lbl, container(Space::new()).width(Length::Fill).height(Length::Fixed(3.0)).style(bx(ACC, 0.0, None))]).style(bx(CARD2, 9.0, None)).clip(true).into()
            } else {
                lbl.into()
            };
            tr = tr.push(click(el, Some(Msg::Go(p))));
        }
        tr = tr.push(pill("RB"));
        r = r.push(tr);
    }
    r = r.push(fill()).push(one(t(if a.signed_in { a.gamertag.clone() } else { "Not signed in yet".to_string() }, 16.0, MUT)));
    column![container(r).padding(pad(0.0, 26.0, 0.0, 26.0)).center_y(Length::Fixed(63.0)).width(Length::Fill).style(bx(SIDE, 0.0, None)), hline(LINE)].into()
}

fn frame<'a>(a: &'a App, tabs: bool, body: E<'a, Msg>, left: Vec<E<'a, Msg>>, right: Vec<E<'a, Msg>>) -> E<'a, Msg> {
    container(column![top(a, tabs), container(body).width(Length::Fill).height(Length::Fill), hints(left, right, 56.0, 24.0, 24.0, BG)]).style(bx(page_bg(a.tint), 0.0, None)).width(Length::Fill).height(Length::Fill).into()
}

fn bare<'a>(body: E<'a, Msg>, left: Vec<E<'a, Msg>>, right: Vec<E<'a, Msg>>) -> E<'a, Msg> {
    container(column![container(body).width(Length::Fill).height(Length::Fill), hints(left, right, 56.0, 24.0, 24.0, BG)]).style(bx(BG, 0.0, None)).width(Length::Fill).height(Length::Fill).into()
}

fn h<'a>(k: &[&str], what: &str) -> E<'a, Msg> {
    hint(k, what, 17.0)
}

fn rt<'a>(s: &str) -> E<'a, Msg> {
    t(s.to_string(), 19.0, TX).into()
}

/// A Game Mode row (`.ally .row`): the focused one lighter, with the ring.
fn grow<'a>(inner: E<'a, Msg>, focused: bool, bg: Option<Color>) -> E<'a, Msg> {
    ring(focused, 12.0, container(inner).padding(pad(14.0, 18.0, 14.0, 18.0)).width(Length::Fill).style(bx(if focused { FOCUSROW } else { bg.unwrap_or(CARD) }, 12.0, None)))
}

pub fn view(a: &App) -> E<'_, Msg> {
    let page = match a.page {
        Page::Welcome => welcome(a),
        Page::SignIn => sign_in(a),
        Page::SignedIn => signed_in(a),
        Page::Consoles => consoles(a),
        Page::Connecting => connecting(a),
        Page::Stream => stream_view(a),
        Page::Trouble => trouble(a),
        Page::Settings => settings(a),
        Page::Cloud => cloud(a),
    };
    let mut layers = vec![page];
    if let Some(d) = a.dd {
        layers.push(chooser(a, d));
    }
    if layers.len() == 1 {
        return layers.pop().unwrap();
    }
    stack(layers).into()
}

fn welcome(a: &App) -> E<'_, Msg> {
    let mut col = Column::new().align_x(iced::Center).spacing(0.0);
    col = col.push(app_icon(96.0, 56.0, 24.0)).push(gap(1.0, 22.0)).push(one(t7("Play your Xbox from anywhere", 36.0, TX)));
    col = col.push(container(t("Your own Xbox at home, on this screen, over the internet. The games run on the Xbox; you play them here.", 18.0, MUT).line_height(1.45).align_x(iced::Center)).max_width(760.0).padding(pad(10.0, 0.0, 26.0, 0.0)));
    col = col.push(btn("Sign in with Microsoft", Some(G::Lock), B::Primary, true, 19.0, a.ring && a.focus == 0, Some(Msg::SignIn)));
    col = col.push(gap(1.0, 12.0)).push(t("You type your password on Microsoft's page, never into this app.", 15.0, MUT));
    if let Some(n) = &a.note {
        col = col.push(gap(1.0, 10.0)).push(t(n.clone(), 15.0, WARN));
    }
    col = col.push(gap(1.0, 26.0)).push(t("Hearth Console Remote Play is a Hearth app. It isn't made by Microsoft.", 13.5, OFF));
    frame(a, false, container(col).center_x(Length::Fill).center_y(Length::Fill).into(), vec![h(&["A"], "Sign in"), h(&["B"], "Close")], vec![one(t("From the Hearth Store", 17.0, HINT)).into()])
}

/// g1: the sign-in window fills the screen in Game Mode (the helper draws it, with Hearth's keyboard); under it the
/// app shows the same frame, dimmed.
fn sign_in(a: &App) -> E<'_, Msg> {
    let body = container(Space::new()).width(Length::Fill).height(Length::Fill).style(bx(Color { r: 0.0, g: 0.0, b: 0.0, a: 0.66 }, 0.0, None));
    frame(a, false, body.into(), vec![h(&["A"], "Select"), h(&["X"], "Keyboard"), h(&["B"], "Cancel sign-in")], vec![one(t("Hearth's on-screen keyboard comes up for each box", 17.0, HINT)).into()])
}

fn signed_in(a: &App) -> E<'_, Msg> {
    let col = column![
        container(one(t7("✓", 48.0, Color::from_rgb8(0x8f, 0xf0, 0xb3)))).center_x(Length::Fixed(96.0)).center_y(Length::Fixed(96.0)).style(bx(Color::from_rgb8(0x17, 0x35, 0x24), 48.0, None)),
        gap(1.0, 20.0),
        one(t7("You're signed in", 34.0, TX)),
        gap(1.0, 10.0),
        t(format!("Signed in as {}", a.gamertag), 19.0, MUT),
        t("Only your gamertag is shown. Your email and password stay with Microsoft.", 15.0, MUT),
        gap(1.0, 26.0),
        btn("Find my Xbox", None, B::Primary, true, 19.0, a.ring && a.focus == 0, Some(Msg::Go(Page::Consoles)))
    ]
    .align_x(iced::Center)
    .spacing(4.0);
    frame(a, false, container(col).center_x(Length::Fill).center_y(Length::Fill).into(), vec![h(&["A"], "Find my Xbox")], vec![])
}

fn consoles(a: &App) -> E<'_, Msg> {
    let mut left = Column::new();
    match &a.consoles {
        Load::Ready(list) if !list.is_empty() => {
            for (i, c) in list.iter().enumerate() {
                let waking = a.waking.as_deref() == Some(c.id.as_str());
                let info = column![
                    one(t6(c.name.clone(), 28.0, TX)),
                    one(t(c.model.clone(), 15.5, MUT)),
                    container(row![if waking { chip("Waking…", Ch::On, Some(ACC2), 15.0) } else { power_chip(c.power, 15.0) }, one(t(desk::ago(c.seen).replace(" minutes", " min").replace(" minute", " min"), 15.0, MUT))].spacing(8.0).align_y(iced::Center)).padding(pad(12.0, 0.0, 16.0, 0.0)),
                    row![
                        btn(if waking { "Waking…" } else { "Wake" }, Some(G::Power), B::Normal, false, 17.0, false, (c.power == Power::Sleeping && !waking).then_some(Msg::Wake(i))),
                        btn("Stream", Some(G::Play), B::Primary, false, 17.0, false, (c.power != Power::Off).then_some(Msg::Stream(i)))
                    ]
                    .spacing(10.0)
                ];
                left = left.push(ring0(a.ring && a.focus == i, 16.0, CARD, container(row![console_box(110.0, 128.0, 16.0, power_color(c.power)), info.width(Length::Fill)].spacing(22.0).align_y(iced::Center)).padding(22.0))).push(gap(1.0, 14.0));
            }
        }
        Load::Failed(e) => left = left.push(grow(column![rt("Couldn't get your consoles"), t(e.clone(), 15.0, MUT)].spacing(4.0).into(), false, None)),
        Load::Ready(_) => left = left.push(grow(column![rt("No Xbox on this account yet"), t("Check the list on the right, then press Y to look again.", 15.0, MUT)].spacing(4.0).into(), false, None)),
        _ => left = left.push(grow(rt("Looking for your Xbox…"), false, None)),
    }
    let legend = row![
        chip("On", Ch::Ok, None, 13.5),
        one(t(" ready · ", 15.5, MUT)),
        chip("Sleeping", Ch::Sleep, None, 13.5),
        one(t(" Wake works · ", 15.5, MUT)),
        chip("Offline", Ch::Plain, None, 13.5),
        one(t(" can't be woken", 15.5, MUT))
    ]
    .spacing(6.0)
    .align_y(iced::Center);
    left = left.push(grow(legend.into(), false, Some(QUIET)));
    let right = container(column![one(t6("Can't see your Xbox?", 20.0, TX)), gap(1.0, 4.0), checklist(&a.gamertag, true)]).padding(pad(18.0, 20.0, 18.0, 20.0)).width(Length::Fixed(460.0)).style(bx(SIDE, 14.0, None));
    let body = container(row![left.width(Length::Fill), column![right]].spacing(22.0)).padding(pad(26.0, 30.0, 26.0, 30.0));
    frame(a, true, body.into(), vec![h(&["A"], "Stream"), h(&["X"], "Wake"), h(&["Y"], "Refresh"), h(&["B"], "Back")], vec![one(t(a.gamertag.clone(), 17.0, HINT)).into()])
}

fn connecting(a: &App) -> E<'_, Msg> {
    let name = a.conn.as_ref().map(|c| c.name.clone()).unwrap_or_else(|| "your Xbox".into());
    let cloud = a.conn.as_ref().map(|c| c.kind == crp::xhome::Kind::Cloud).unwrap_or(false);
    let mut col = Column::new().width(Length::Fixed(760.0));
    col = col.push(one(t6(format!("Connecting to {name}"), 30.0, TX))).push(gap(1.0, 6.0));
    col = col.push(t(if cloud { "Over the internet to Microsoft's servers." } else { "Over the internet to your Xbox at home." }, 17.0, MUT)).push(gap(1.0, 22.0));
    for (i, (k, title, line, timer)) in steps(a, true).into_iter().enumerate() {
        col = col.push(step_row(k, &(i + 1).to_string(), &title, line, timer, (38.0, 19.0, 15.0))).push(gap(1.0, 10.0));
    }
    col = col.push(gap(1.0, 10.0)).push(row![btn("Cancel", Some(G::X), B::Normal, false, 18.0, a.ring && a.focus == 0, Some(Msg::Cancel)), t("Usually 10 to 30 seconds.", 15.5, MUT)].spacing(16.0).align_y(iced::Center));
    bare(container(col).center_x(Length::Fill).center_y(Length::Fill).style(bx(DEEP, 0.0, None)).into(), vec![h(&["B"], "Cancel")], vec![one(t(name, 17.0, HINT)).into()])
}

fn trouble(a: &App) -> E<'_, Msg> {
    let name = a.conn.as_ref().map(|c| c.name.clone()).unwrap_or_else(|| "Your Xbox".into());
    let what = if a.trouble.as_ref().map(|t| t.1).unwrap_or(false) { format!("{name} is on, but the picture couldn't get through.") } else { desk::trouble_words(a) };
    let on = format!("You're on **{}**.", a.net.words());
    let tip = |g: G, title: &str, l: &str| -> E<'static, Msg> { srow(Some(ri(g, 40.0)), title, Some(t(l.to_string(), 15.0, MUT).into()), None, 19.0, CARD, pad(14.0, 18.0, 14.0, 18.0), false) };
    // The line under the title is its own layer over the header (laid out across the page's whole width), so it's
    // never measured to the title's width (the CPU renderer clipped it there).
    let head = row![
        container(glyph(G::Console, Color::from_rgb8(0xff, 0x8a, 0x8a), 36.0)).center_x(Length::Fixed(64.0)).center_y(Length::Fixed(64.0)).style(bx(Color::from_rgb8(0x2a, 0x16, 0x16), 18.0, Some((Color::from_rgb8(0x6a, 0x2d, 0x2d), 1.0)))),
        column![one(t6("Couldn't reach your Xbox", 27.0, TX)), gap(1.0, 26.0)]
    ]
    .spacing(18.0)
    .width(Length::Fill)
    .align_y(iced::Center);
    let under = container(rich1(&format!("{what} {on}"), 16.0, MUT, SOFT)).padding(pad(38.0, 0.0, 0.0, 82.0)).width(Length::Fill);
    let col = column![
        stack![head, under],
        gap(1.0, 16.0),
        tip(G::Up, "Home upload speed: about 10 Mbps or more", "Run a speed test at home; the upload number is the one that counts."),
        gap(1.0, 8.0),
        tip(G::Wifi, "Use 5 GHz Wi-Fi or your phone's hotspot", "2.4 GHz is slower and busier."),
        gap(1.0, 8.0),
        tip(G::Building, "Work Wi-Fi may block game streaming", "If your hotspot works but this network doesn't, that's why."),
        gap(1.0, 14.0),
        row![
            btn("Try again", None, B::Primary, false, 17.0, a.ring && a.focus == 0, Some(Msg::TryAgain(true))),
            btn("Try at 720p", None, B::Normal, false, 17.0, a.ring && a.focus == 1, Some(Msg::TryAgain(false))),
            btn("Back to consoles", None, B::Normal, false, 17.0, a.ring && a.focus == 2, Some(Msg::Go(Page::Consoles)))
        ]
        .spacing(12.0)
    ]
    .width(Length::Fill);
    frame(a, true, container(col).padding(pad(22.0, 34.0, 22.0, 34.0)).into(), vec![h(&["A"], "Try again"), h(&["X"], "Try at 720p"), h(&["B"], "Back")], vec![])
}

fn settings(a: &App) -> E<'_, Msg> {
    let p = &a.prefs;
    let f = |i: usize| a.ring && a.focus == i;
    let seg3 = |key: &'static str, opts: &[(&str, &str)], now: &str| -> E<'static, Msg> { seg(opts.iter().map(|(v, l)| (l.to_string(), *v == now, Msg::Set(key, json!(v)))).collect(), 16.0, 7.0, 15.0, false) };
    let line = |title: &str, end: E<'static, Msg>| -> E<'static, Msg> { row![rt(title), fill(), end].align_y(iced::Center).into() };
    let used = a.month_bytes();
    let limit = p.limit_gb as f64 * 1e9;
    let mut limit_box = Column::new().spacing(8.0).push(line("Monthly data limit", row![dd(&format!("{} GB", p.limit_gb), 100.0, f(3), Msg::Dd(Some(Dd::Limit))), toggle(p.limit_on, 1.0, false, Some(Msg::Set("limitOn", json!(!p.limit_on))))].spacing(10.0).align_y(iced::Center).into()));
    if p.limit_on {
        limit_box = limit_box.push(meter((used as f64 / limit) as f32, p.warn_pct as f32 / 100.0)).push(t(format!("{} used on every network · warns at {} % ({})", data::gb(used), p.warn_pct, data::gb((limit * p.warn_pct as f64 / 100.0) as u64).replace(".0 GB", " GB")), 15.0, MUT));
    }
    let left = column![
        grow(line("Default quality", seg3("quality", &[("auto", "Auto"), ("720p", "720p"), ("1080p", "1080p")], &p.quality)), f(0), None),
        gap(1.0, 8.0),
        grow(line("Data saver", seg3("dataSaver", &[("off", "Off"), ("hotspot", "Hotspot"), ("always", "Always")], &p.saver)), f(1), None),
        gap(1.0, 8.0),
        grow(limit_box.into(), f(2), None),
        gap(1.0, 8.0),
        grow(line("Play with", seg3("playWith", &[("builtin", "Ally's buttons"), ("paired", "Paired pad")], &p.play_with)), f(4), None),
        gap(1.0, 8.0),
        grow(column![rt("Menu while streaming"), rich("Hold **View + Menu** for 1 second", 15.0, MUT, SOFT, false)].spacing(3.0).into(), false, None)
    ];
    let (chip_l, chip_ok) = &a.store_chip;
    let right = column![
        grow(line("Vibration", toggle(p.vibration, 1.0, false, Some(Msg::Set("vibration", json!(!p.vibration))))), f(5), None),
        gap(1.0, 8.0),
        grow(line("Start full screen", toggle(p.full_screen, 1.0, false, Some(Msg::Set("startFullScreen", json!(!p.full_screen))))), f(6), None),
        gap(1.0, 8.0),
        grow(line("Keep the screen awake", toggle(p.keep_awake, 1.0, false, Some(Msg::Set("keepAwake", json!(!p.keep_awake))))), f(7), None),
        gap(1.0, 8.0),
        grow(line("Sound plays on", dd(&desk::sound_words(a), 210.0, false, Msg::Dd(Some(Dd::Sound)))), f(8), None),
        gap(1.0, 8.0),
        grow(line(&a.gamertag, btn("Sign out", None, B::Normal, false, 16.0, false, Some(Msg::SignOut))), f(9), None),
        gap(1.0, 8.0),
        grow(row![column![rt("Hearth Store app"), t("Updates apart from Hearth OS", 15.0, MUT)].spacing(3.0), fill(), chip(chip_l, if *chip_ok { Ch::Ok } else { Ch::On }, None, 13.5)].align_y(iced::Center).into(), f(10), None)
    ];
    let body = container(scrollable(row![left.width(Length::Fill), right.width(Length::Fixed(500.0))].spacing(20.0))).padding(pad(18.0, 30.0, 18.0, 30.0));
    frame(a, true, body.into(), vec![h(&["A"], "Select"), h(&["◀▶"], "Change"), h(&["B"], "Back")], vec![one(t("Saved as you change them", 17.0, HINT)).into()])
}

fn cloud(a: &App) -> E<'_, Msg> {
    let head = row![one(t6("Cloud", 24.0, TX)), chip("Optional", Ch::On, None, 13.5), chip("Game Pass Ultimate", Ch::Ok, None, 13.5), fill(), one(t("Your Xbox can stay asleep", 15.0, MUT))].spacing(12.0).align_y(iced::Center);
    let tabs = seg(vec![("Recently played".into(), a.tab == CloudTab::Recent, Msg::CloudTab(CloudTab::Recent)), ("All cloud games".into(), a.tab == CloudTab::All, Msg::CloudTab(CloudTab::All)), ("Search".into(), a.tab == CloudTab::Search, Msg::CloudTab(CloudTab::Search))], 15.0, 7.0, 15.0, a.ring && a.focus == 0 && a.tab != CloudTab::Recent);
    let mut trow = Row::new().spacing(12.0).align_y(iced::Center).push(tabs);
    if a.tab == CloudTab::Search {
        trow = trow.push(text_input("Search cloud games", &a.search).on_input(Msg::Search).size(16.0).padding(pad(9.0, 14.0, 9.0, 14.0)).width(Length::Fixed(320.0)));
    }
    let list = a.shown_games();
    let mut grid = Column::new().spacing(14.0);
    if list.is_empty() {
        grid = grid.push(t(
            match if a.tab == CloudTab::Recent { &a.games } else { &a.all_games } {
                Load::Failed(e) => e.clone(),
                Load::Ready(_) => "Nothing here yet.".to_string(),
                _ => "Getting the cloud games…".to_string(),
            },
            16.0,
            MUT,
        ));
    }
    let mut i = 0;
    while i < list.len() {
        let big = i < 4;
        let mut r = Row::new().spacing(14.0);
        for j in i..(i + 4) {
            let cell: E<'_, Msg> = match list.get(j) {
                Some(g) => tile(a, j, g, if big { 190.0 } else { 110.0 }, !big, a.ring && a.focus == j + 1, if big { 16.0 } else { 15.0 }, big),
                None => Space::new().width(Length::Fill).into(),
            };
            r = r.push(container(cell).width(Length::FillPortion(1)));
        }
        grid = grid.push(r);
        i += 4;
    }
    let body = column![
        head,
        gap(1.0, 6.0),
        t("Games that run on Microsoft's servers. This tab shows only when your account has Game Pass Ultimate.", 15.0, MUT),
        gap(1.0, 14.0),
        trow,
        gap(1.0, 14.0),
        grid
    ];
    frame(a, true, container(scrollable(body)).padding(pad(18.0, 28.0, 18.0, 28.0)).into(), vec![h(&["A"], "Play"), h(&["Y"], "Search"), h(&["B"], "Back")], vec![one(t("Same quality and data settings as your Xbox", 17.0, HINT)).into()])
}

fn chooser(a: &App, d: Dd) -> E<'_, Msg> {
    let now = desk::dd_now(a, d);
    let mut c = Column::new().spacing(8.0);
    for (v, words) in desk::dd_choices(a, d) {
        let on = v == now;
        c = c.push(click(container(one(t(words, 18.0, if on { INK } else { TX }))).padding(pad(12.0, 16.0, 12.0, 16.0)).width(Length::Fill).style(bx(if on { ACC } else { CARD }, 10.0, None)), Some(Msg::Pick(d, v))));
    }
    c = c.push(t("A picks the next one · B closes", 15.0, MUT));
    hearth_ui::ui::dialog(Space::new().into(), container(c).padding(20.0).width(Length::Fixed(420.0)).style(shadowed(SIDE, 14.0, Some((LINE, 1.0)), 40.0)).into(), Some(Msg::Dd(None)))
}

// ------------------------------------------------------------------ streaming (g7a, g7b)
fn stream_view(a: &App) -> E<'_, Msg> {
    let s = &a.stats;
    let (q, nbars) = s.quality();
    let name = a.conn.as_ref().map(|c| c.name.clone()).unwrap_or_else(|| "Your Xbox".into());
    let mut layers: Vec<E<'_, Msg>> = vec![desk::picture(a)];
    if a.menu {
        layers.push(container(Space::new()).width(Length::Fill).height(Length::Fill).style(bx(Color { r: 0.0, g: 0.0, b: 0.0, a: 0.5 }, 0.0, None)).into());
    }
    if (a.pill || a.prefs.show_stats) && !a.menu {
        let delay = s.delay_ms.map(|d| format!("{d} ms")).unwrap_or_else(|| "– ms".into());
        let pill = container(
            row![bars(nbars, OK), column![one(t7(format!("{name} · {q}"), 15.0, TX)), one(t(format!("{delay} · {:.1} Mbps · {} · {}", s.mbps, s.resolution(), data::gb(s.session_bytes)), 13.5, MUT))].spacing(2.0)].spacing(10.0).align_y(iced::Center),
        )
        .padding(pad(10.0, 14.0, 10.0, 14.0))
        .style(shadowed(Color { a: 0.91, ..Color::from_rgb8(0x12, 0x14, 0x19) }, 12.0, Some((Color { r: 1.0, g: 1.0, b: 1.0, a: 0.11 }, 1.0)), 40.0));
        layers.push(container(pill).padding(18.0).into());
    }
    if a.pill && !a.menu {
        let strip = container(row![hint(&["View", "Menu"], "Hold 1 s: Console Remote Play menu", 17.0), fill(), one(t("Every other button plays your Xbox", 17.0, Color::from_rgb8(0xe3, 0xe6, 0xeb)))].align_y(iced::Center))
            .padding(pad(0.0, 22.0, 0.0, 22.0))
            .center_y(Length::Fixed(60.0))
            .width(Length::Fill)
            .style(grad(180.0, Color::TRANSPARENT, Color { r: 0.0, g: 0.0, b: 0.0, a: 0.85 }, 0.0));
        layers.push(container(strip).align_bottom(Length::Fill).width(Length::Fill).into());
    }
    if a.menu {
        layers.push(quick_menu(a, true));
    }
    if let Some((p, hd)) = &a.shot {
        let name = p.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
        let toast = container(row![container(iced::widget::image(hd.clone()).content_fit(iced::ContentFit::Cover).width(Length::Fixed(96.0)).height(Length::Fixed(54.0))).clip(true), column![one(t6("Screenshot saved", 15.0, TX)), one(t(format!("Pictures › Screenshots › {name}"), 13.0, MUT))].spacing(2.0)].spacing(12.0).align_y(iced::Center))
            .padding(pad(10.0, 14.0, 10.0, 14.0))
            .style(shadowed(Color { a: 0.92, ..Color::from_rgb8(0x12, 0x14, 0x19) }, 12.0, None, 30.0));
        layers.push(container(toast).center_x(Length::Fill).align_bottom(Length::Fill).padding(pad(0.0, 0.0, 70.0, 0.0)).into());
    }
    if a.warn_toast {
        let toast = container(column![one(t6(format!("{} of your {} GB monthly streaming data used", data::gb(a.month_bytes()), a.prefs.limit_gb), 15.0, TX)), row![btn("Turn on Data saver", None, B::Primary, false, 15.0, false, Some(Msg::SaverNow)), btn("Keep going", None, B::Normal, false, 15.0, false, Some(Msg::KeepGoing))].spacing(10.0)].spacing(8.0))
            .padding(14.0)
            .style(shadowed(Color { a: 0.96, ..Color::from_rgb8(0x12, 0x14, 0x19) }, 12.0, Some((Color::from_rgb8(0x7a, 0x66, 0x33), 1.0)), 30.0));
        layers.push(container(toast).center_x(Length::Fill).padding(pad(18.0, 0.0, 0.0, 0.0)).into());
    }
    stack(layers).into()
}

/// The Console Remote Play menu (g7b): opened by holding View + Menu for 1 s (Ctrl+Shift+M on a keyboard). While it's
/// open the Xbox gets no buttons; B goes back to the game. On the laptop the same panel opens over the picture.
pub fn quick_menu<'a>(a: &'a App, _ally: bool) -> E<'a, Msg> {
    let s = &a.stats;
    let p = &a.prefs;
    let (q, _) = s.quality();
    let name = a.conn.as_ref().map(|c| c.name.clone()).unwrap_or_else(|| "Your Xbox".into());
    let fo = |i: usize| a.menu_focus == i;
    let mrow = |inner: E<'a, Msg>, focused: bool, bg: Color| -> E<'a, Msg> { ring(focused, 12.0, container(inner).padding(pad(11.0, 16.0, 11.0, 16.0)).width(Length::Fill).style(bx(if focused { FOCUSROW } else { bg }, 12.0, None))) };
    let title = |s: &str| -> E<'a, Msg> { t(s.to_string(), 18.0, TX).into() };
    let delay = s.delay_ms.map(|d| format!("{d} ms")).unwrap_or_else(|| "– ms".into());
    let saver_words = match p.saver.as_str() {
        "off" => "Off",
        "hotspot" => "On (hotspot)",
        _ => "On",
    };
    let small_ri = |g: G, col: Color, bg: Color| -> E<'a, Msg> { container(glyph(g, col, 20.0)).center_x(Length::Fixed(34.0)).center_y(Length::Fixed(34.0)).style(bx(bg, 10.0, None)).into() };
    let pick = |label: &str, play: &str| if play == "builtin" { format!("{label} (Ally)") } else { "A paired controller".to_string() };
    let col = column![
        row![app_icon(36.0, 22.0, 10.0), one(t6("Console Remote Play", 22.0, TX)), fill(), chip("Game still running", Ch::Ok, None, 14.0)].spacing(12.0).align_y(iced::Center),
        gap(1.0, 4.0),
        row![dotted(OK, 9.0), gap(7.0, 1.0), one(t(format!("{name} · {q} · {delay} · {:.1} Mbps · {}", s.mbps, s.resolution()), 15.0, MUT))].align_y(iced::Center),
        gap(1.0, 6.0),
        row![glyph(G::Data, MUT, 15.0), gap(6.0, 1.0), one(t(format!("Data: {} now · {} of {} GB this month", data::gb(s.session_bytes), data::gb(a.month_bytes()).replace(" GB", ""), p.limit_gb), 15.0, MUT))].align_y(iced::Center),
        gap(1.0, 12.0),
        mrow(row![title("Quality"), fill(), seg(vec![("Auto".into(), p.quality == "auto", Msg::Set("quality", json!("auto"))), ("720p".into(), p.quality == "720p", Msg::Set("quality", json!("720p"))), ("1080p".into(), p.quality == "1080p", Msg::Set("quality", json!("1080p")))], 15.0, 7.0, 15.0, false)].align_y(iced::Center).into(), fo(0), CARD),
        gap(1.0, 8.0),
        mrow(row![title("Data saver"), fill(), one(t(saver_words, 14.0, MUT)), gap(10.0, 1.0), toggle(p.saver != "off", 1.0, false, Some(Msg::Set("dataSaver", json!(if p.saver == "off" { "hotspot" } else { "off" }))))].align_y(iced::Center).into(), fo(1), CARD),
        gap(1.0, 8.0),
        mrow(row![title("Volume"), fill(), desk::volume_slider(p.volume, 150.0), gap(10.0, 1.0), container(one(t(p.volume.to_string(), 14.0, MUT))).width(Length::Fixed(24.0))].align_y(iced::Center).into(), fo(2), CARD),
        gap(1.0, 8.0),
        mrow(row![title("Show stats"), fill(), toggle(p.show_stats, 1.0, false, Some(Msg::Set("showStats", json!(!p.show_stats))))].align_y(iced::Center).into(), fo(3), CARD),
        gap(1.0, 8.0),
        row![
            click(container(mrow(row![small_ri(G::Cam, SOFT2, CARD2), t("Screenshot", 17.0, TX)].spacing(12.0).align_y(iced::Center).into(), fo(4), CARD)).width(Length::FillPortion(1)), Some(Msg::Screenshot)),
            click(container(mrow(row![small_ri(G::Gear, SOFT2, CARD2), t("Settings", 17.0, TX)].spacing(12.0).align_y(iced::Center).into(), fo(5), CARD)).width(Length::FillPortion(1)), Some(Msg::Go(Page::Settings)))
        ]
        .spacing(8.0),
        gap(1.0, 8.0),
        mrow(row![column![title("Vibration"), t(pick("Built-in controls", &p.play_with), 13.5, MUT)].spacing(2.0), fill(), toggle(p.vibration, 1.0, false, Some(Msg::Set("vibration", json!(!p.vibration))))].align_y(iced::Center).into(), fo(6), CARD),
        gap(1.0, 8.0),
        click(mrow(row![small_ri(G::Exit, Color::from_rgb8(0xff, 0xb3, 0xb3), Color::from_rgb8(0x4a, 0x20, 0x20)), column![t("Disconnect", 18.0, Color::from_rgb8(0xff, 0xd0, 0xd0)), t("Your Xbox stays on and sleeps by itself later", 13.5, MUT)].spacing(2.0)].spacing(16.0).align_y(iced::Center).into(), fo(7), Color::from_rgb8(0x3a, 0x1d, 0x1d)), Some(Msg::Disconnect)),
        gap(1.0, 4.0),
        rich("Hold **View + Menu** any time to open this menu.", 14.0, MUT, SOFT, false),
        vfill(),
        hints(vec![h(&["A"], "Select"), hint(&["◀▶"], "Change", 16.0), hint(&["B"], "Back to game", 16.0)], vec![], 52.0, 20.0, 20.0, BG)
    ];
    let panel = container(container(col).padding(pad(20.0, 22.0, 0.0, 22.0)).height(Length::Fill))
        .width(Length::Fixed(500.0))
        .height(Length::Fill)
        .style(shadowed(BG, 0.0, Some((LINE, 1.0)), 60.0));
    container(panel).align_right(Length::Fill).height(Length::Fill).into()
}
