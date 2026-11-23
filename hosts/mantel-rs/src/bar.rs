//! The bar along the top, drawn as the QML Mantel's Islands.qml (Mantel 2026): floating islands (Hearth, Gaming,
//! Glass, Synthwave, Cabin), one full-width bar (Steam-style; Zen's is thin) or Console's bigger one, each size and gap
//! as there:
//!   start   the flame (opens the app menu), the desktops (pills, the one you're on long and lit), the window in front
//!   centre  the clock pill: the time and the date
//!   end     the status island (Discord, keyboard, network, Bluetooth, volume, battery, notifications), then the
//!           Game Mode button, always there.
//! Desktop layouts (desk.rs, hosts/hearth-layout): the bar on any edge, its blocks in the order bar.json gives (the
//! start, the middle, the end), your apps in it when there's no floating dock, Game Mode first or before the status
//! island, the clock at the end, Show desktop last, and the layouts' own blocks (the desktops pill, Task view, the app
//! in front by name, its menus, Quick Access, Show desktop).

use iced::widget::{button, column, container, mouse_area, row, stack, text, Column, Row, Space};
use iced::{border, Alignment, Background, Border, Color, Element, Length, Padding, Theme};

use crate::look::{self, a, lighter, Look};
use crate::shell::{Message, Shell};

/// Islands.qml's sizes for this look (and a handheld-sized screen: compact).
pub struct Geom {
    pub style: String,
    pub floating: bool,
    pub shelf: bool,
    pub k: f32,
    pub island_h: f32,
    pub gap_out: f32,
    pub gap_in: f32,
    pub along: f32,
    pub spacing: f32,
}

impl Geom {
    pub fn new(l: &Look, compact: bool) -> Geom {
        let style = l.style.clone();
        let floating = style == "floating";
        let shelf = style == "shelf";
        let compact = compact && !shelf;
        let island_h = if shelf {
            72.0
        } else if l.thin {
            32.0
        } else if style == "bar" {
            44.0
        } else if compact {
            48.0
        } else {
            52.0
        };
        Geom {
            k: if shelf { 1.4 } else { 1.0 },
            gap_out: if floating { 10.0 } else if shelf { 18.0 } else { 0.0 },
            gap_in: if floating { 6.0 } else if shelf { 10.0 } else { 0.0 },
            along: if floating { 12.0 } else if shelf { 40.0 } else { 0.0 },
            spacing: if floating { 10.0 } else { 6.0 },
            style,
            floating,
            shelf,
            island_h,
        }
    }
    /// Islands.qml's sizes down a side: a little taller buttons, the same gaps.
    pub fn for_edge(l: &Look, compact: bool, vertical: bool) -> Geom {
        let mut g = Geom::new(l, compact);
        if vertical && !g.shelf {
            g.island_h = if compact { 52.0 } else { 56.0 };
        }
        if vertical && g.shelf {
            g.style = "bar".into();
            g.shelf = false;
            g.k = 1.0;
        }
        g
    }
    pub fn band(&self) -> f32 {
        self.island_h + self.gap_out + self.gap_in
    }
    /// IslandButton's side
    pub fn side(&self) -> f32 {
        if self.shelf {
            60.0
        } else {
            (self.island_h - 12.0).max(26.0)
        }
    }
}

/// The bar's thickness (and the space it keeps for itself on its edge).
pub fn band(l: &Look, compact: bool, vertical: bool) -> u32 {
    Geom::for_edge(l, compact, vertical).band().round() as u32
}

/// An island's background in the look's style (Frame.qml): nothing inside a full-width bar.
pub fn frame(l: &Look, g: &Geom, radius: f32, lit: bool) -> impl Fn(&Theme) -> container::Style {
    let fill = l.glass();
    let edge = if lit { l.p.accent } else { l.p.line };
    let floating = g.floating || g.shelf;
    let gaming = l.gaming();
    let accent = l.p.accent;
    let frame = l.frame.clone();
    move |_| {
        if !floating {
            return container::Style::default();
        }
        let (bw, bc) = match frame.as_str() {
            "neon" => (1.5, a(accent, if lit { 1.0 } else { 0.8 })),
            "chamfer" => (1.5, a(accent, if lit { 1.0 } else { 0.55 })),
            _ => (1.0, edge),
        };
        container::Style {
            background: Some(fill.into()),
            border: Border { color: bc, width: bw, radius: if gaming { 0.0.into() } else { radius.into() } },
            ..container::Style::default()
        }
    }
}

/// IslandButton: a square button, lit when the pointer is on it.
fn ibtn<'a>(l: &Look, side: f32, width: f32, content: Element<'a, Message>, msg: Message) -> Element<'a, Message> {
    let p = l.p;
    let r = l.r(10.0);
    button(container(content).center_y(Length::Fixed(side)).center_x(Length::Fixed(width)))
        .padding(0)
        .on_press(msg)
        .style(move |_, st| button::Style {
            background: matches!(st, button::Status::Hovered | button::Status::Pressed).then(|| Background::Color(p.panel3)),
            border: border::rounded(r),
            text_color: p.text,
            ..button::Style::default()
        })
        .into()
}

fn island<'a>(l: &Look, g: &Geom, content: Element<'a, Message>, lit: bool) -> Element<'a, Message> {
    let pad = if g.floating { 8.0 } else { 2.0 };
    container(content)
        .padding(Padding { left: pad, right: pad, ..Padding::ZERO })
        .height(Length::Fixed(g.island_h))
        .align_y(Alignment::Center)
        .style(frame(l, g, if l.gaming() { 0.0 } else { l.r(g.island_h / 2.4) }, lit))
        .into()
}

fn desks<'a>(s: &Shell, l: &Look, g: &Geom) -> Option<Element<'a, Message>> {
    if s.wm.desks.len() < 2 || g.shelf {
        return None;
    }
    let p = l.p;
    let size = if g.island_h > 40.0 { 24.0 } else { 20.0 };
    let mut r = row![].spacing(3).align_y(Alignment::Center);
    for (i, d) in s.wm.desks.iter().enumerate() {
        let on = d.active;
        let label = if on { l.tb((i + 1).to_string(), 12.0, p.ink) } else { l.t((i + 1).to_string(), 12.0, p.muted) };
        let rad = l.r(size / 2.0);
        r = r.push(
            button(container(label).center(Length::Fill))
                .width(Length::Fixed(if on { size * 1.7 } else { size }))
                .height(Length::Fixed(size))
                .padding(0)
                .on_press(Message::Desk(d.id))
                .style(move |_, st| button::Style {
                    background: Some(Background::Color(if on {
                        p.accent
                    } else if matches!(st, button::Status::Hovered | button::Status::Pressed) {
                        p.panel3
                    } else {
                        a(p.text, 0.07)
                    })),
                    border: border::rounded(rad),
                    text_color: p.text,
                    ..button::Style::default()
                }),
        );
    }
    Some(container(r).padding(4).into())
}

fn game_mode<'a>(l: &Look, g: &Geom) -> Element<'a, Message> {
    let p = l.p;
    let white = l.name == "glass";
    let synth = l.name == "synthwave";
    let ink = if white { Color::from_rgb8(0x17, 0x20, 0x3a) } else { p.ink };
    let h = if g.style == "bar" { g.island_h - 8.0 } else { g.island_h };
    let radius = if g.style == "bar" { l.r(6.0) } else { l.r(h / 2.0) };
    let label = if l.gaming() { "GAME MODE" } else { "Game Mode" };
    let size = ((if l.thin { 12.0 } else { 14.0 }) * g.k).round();
    let content = row![look::icon("pad", 20.0 * g.k, ink), l.tb(label, size, ink)].spacing(8).align_y(Alignment::Center);
    let pad = if g.shelf { 28.0 } else { 18.0 };
    let (accent, flame) = (p.accent, p.flame);
    let gaming = l.gaming();
    button(container(content).center_y(Length::Fixed(h)))
        .padding(Padding { left: pad, right: pad, ..Padding::ZERO })
        .height(Length::Fixed(h))
        .on_press(Message::GameMode)
        .style(move |_, st| {
            let hover = matches!(st, button::Status::Hovered | button::Status::Pressed);
            let bg: Background = if white {
                Color::from_rgb8(if hover { 0xff } else { 0xf2 }, if hover { 0xff } else { 0xf6 }, 0xff).into()
            } else if synth {
                iced::Gradient::Linear(
                    iced::gradient::Linear::new(iced::Radians(std::f32::consts::PI))
                        .add_stop(0.0, Color::from_rgb8(0xff, 0x9b, 0xd5))
                        .add_stop(0.48, Color::from_rgb8(0xff, 0x3f, 0xa4))
                        .add_stop(0.52, Color::from_rgb8(0xc8, 0x1d, 0x7e))
                        .add_stop(1.0, Color::from_rgb8(0xff, 0x5f, 0xb4)),
                )
                .into()
            } else if gaming {
                iced::Gradient::Linear(iced::gradient::Linear::new(iced::Radians(std::f32::consts::FRAC_PI_2)).add_stop(0.0, accent).add_stop(1.0, flame)).into()
            } else if hover {
                lighter(accent, 1.12).into()
            } else {
                accent.into()
            };
            button::Style { background: Some(bg), border: border::rounded(if gaming { 0.0 } else { radius }), text_color: ink, ..button::Style::default() }
        })
        .into()
}


/// Items in a line: across, or down a side.
pub fn line<'a>(vert: bool, items: Vec<Element<'a, Message>>, spacing: f32) -> Element<'a, Message> {
    if vert {
        Column::with_children(items).spacing(spacing).align_x(Alignment::Center).into()
    } else {
        Row::with_children(items).spacing(spacing).align_y(Alignment::Center).into()
    }
}

fn island_v<'a>(l: &Look, g: &Geom, vert: bool, content: Element<'a, Message>, lit: bool) -> Element<'a, Message> {
    if !vert {
        return island(l, g, content, lit);
    }
    let pad = if g.floating { 8.0 } else { 2.0 };
    container(content)
        .padding(Padding { top: pad, bottom: pad, ..Padding::ZERO })
        .width(Length::Fixed(g.island_h))
        .align_x(Alignment::Center)
        .style(frame(l, g, if l.gaming() { 0.0 } else { l.r(g.island_h / 2.4) }, lit))
        .into()
}

/// The app in front's name (macOS-style), as the QML Mantel works it out.
pub fn front_name(s: &Shell) -> Option<String> {
    let w = s.wm.wins.iter().find(|w| w.active)?;
    let app = crate::apps::for_window(&s.apps, &w.app_id);
    Some(app.map(|a| a.name.clone()).unwrap_or_else(|| w.title.split(" — ").next().unwrap_or(&w.title).split(" - ").next().unwrap_or("").to_string()))
}

/// One of the bar's blocks (or nothing, when it has nothing to show here, as in the QML Mantel).
pub fn block<'a>(s: &'a Shell, l: &Look, g: &Geom, vert: bool, id: &str) -> Option<Element<'a, Message>> {
    let p = l.p;
    let side = g.side();
    let k = g.k;
    let compact = s.compact() && !g.shelf;
    let simple = |name: &str, size: f32, c: iced::Color, msg: Message| Some(ibtn(l, side, side, look::icon(name, size * k, c), msg));
    match id {
        "menu" => Some(ibtn(l, side, side, look::flame(26.0 * k), Message::Flame)),
        "desktops" => desks(s, l, g),
        "window" => {
            if compact || g.shelf || vert || s.desk.apps_in_bar {
                return None;
            }
            let w = s.wm.wins.iter().find(|w| w.active && !w.title.is_empty())?;
            let app = crate::apps::for_window(&s.apps, &w.app_id);
            let title = hearth_ui::ui::fit(&w.title, 38);
            let t = if l.gaming() { l.ts(title.to_uppercase(), 13.0, p.text) } else { l.ts(title, 13.0, p.text) };
            Some(row![look::app_pic(app.and_then(|a| a.icon_path.as_deref()), 20.0), t].spacing(8).align_y(Alignment::Center).into())
        }
        "search" => {
            if vert || compact || g.shelf || s.desk.layout.is_empty() && s.desk.app_menu != "tiles" {
                return None;
            }
            let (bg, line, muted) = (a(p.bg2, 0.85), p.line, p.muted);
            let r = l.r(8.0);
            let face = row![look::icon("search", 14.0, muted), l.t("Search", 13.0, muted)].spacing(8).align_y(Alignment::Center);
            Some(
                button(container(face).padding(Padding { left: 12.0, right: 12.0, ..Padding::ZERO }).width(Length::Fixed(260.0)).center_y(Length::Fixed((g.island_h - 14.0).max(26.0))))
                    .padding(0)
                    .on_press(Message::Flame)
                    .style(move |_, st| button::Style {
                        background: Some(Background::Color(if matches!(st, button::Status::Hovered | button::Status::Pressed) { p.panel3 } else { bg })),
                        border: Border { color: line, width: 1.0, radius: r.into() },
                        text_color: p.text,
                        ..button::Style::default()
                    })
                    .into(),
            )
        }
        "discord" if !compact => simple("chat", 20.0, p.text, Message::Nothing),
        "keyboard" if !compact => simple("kbd", 20.0, p.text, Message::Nothing),
        "network" => {
            let net = match s.status.wifi {
                Some(-1) => "wired",
                Some(_) => "wifi",
                None => "offline",
            };
            simple(net, 19.0, p.text, Message::Nothing)
        }
        "bluetooth" if s.bt.present && !compact => simple("bt", 18.0, if s.bt.powered { p.text } else { p.muted }, Message::Nothing),
        "volume" => {
            let (pct, muted) = s.status.volume?;
            let name = if muted { "vol-mute" } else if pct > 33 { "vol" } else { "vol-low" };
            let b = ibtn(l, side, side, look::icon(name, 19.0 * k, p.text), Message::Nothing);
            Some(
                mouse_area(b)
                    .on_middle_press(Message::VolMute)
                    .on_scroll(|d| {
                        let y = match d {
                            iced::mouse::ScrollDelta::Lines { y, .. } | iced::mouse::ScrollDelta::Pixels { y, .. } => y,
                        };
                        Message::VolScroll(y > 0.0)
                    })
                    .into(),
            )
        }
        "battery" => {
            let (pct, charging) = s.status.battery?;
            let name = if charging { "batt-charging" } else if pct > 20 { "batt" } else { "batt-low" };
            if vert {
                return simple(name, 19.0, p.text, Message::Nothing);
            }
            let label = l.t(format!("{pct}%"), (13.0 * k).round(), p.text);
            let w = side + 4.0 + 13.0 * 0.6 * format!("{pct}%").len() as f32;
            let inner: Element<'_, Message> = row![Space::new().width(Length::Fixed((side - 19.0 * k) / 2.0)), look::icon(name, 19.0 * k, p.text), Space::new().width(Length::Fixed(((side - 19.0 * k) / 2.0 - 6.0).max(0.0))), label]
                .align_y(Alignment::Center)
                .into();
            Some(ibtn(l, side, w, inner, Message::Nothing))
        }
        "notifications" if !compact => simple("bell", 19.0, p.text, Message::Nothing),
        // the layouts' own blocks
        "overview" => {
            let n = s.wm.desks.len().max(1);
            let at = s.wm.desks.iter().position(|d| d.active).unwrap_or(0);
            let mut dots: Vec<Element<'a, Message>> = Vec::new();
            for i in 0..n {
                let on = i == at;
                let (w, h) = if vert { (6.0, if on { 22.0 } else { 6.0 }) } else { (if on { 22.0 } else { 6.0 }, 6.0) };
                let c = if on { p.text } else { p.muted };
                dots.push(container(Space::new().width(Length::Fixed(w)).height(Length::Fixed(h))).style(move |_| container::Style { background: Some(c.into()), border: border::rounded(3.0), ..container::Style::default() }).into());
            }
            let pill = container(line(vert, dots, 5.0)).padding(8);
            let (t, p3) = (a(p.text, 0.12), p.panel3);
            let r = l.r(side * 0.35);
            Some(
                button(pill)
                    .padding(0)
                    .on_press(Message::Overview)
                    .style(move |_, st| button::Style {
                        background: Some(Background::Color(if matches!(st, button::Status::Hovered | button::Status::Pressed) { p3 } else { t })),
                        border: border::rounded(r),
                        ..button::Style::default()
                    })
                    .into(),
            )
        }
        "taskview" => simple("desktops", 20.0, p.text, Message::Overview),
        "appname" => {
            if vert {
                return None;
            }
            let n = front_name(s)?;
            Some(container(l.tb(hearth_ui::ui::fit(&n, 30), 14.0, p.text)).padding(Padding { left: 8.0, right: 8.0, ..Padding::ZERO }).into())
        }
        "appmenus" => {
            if vert || front_name(s).is_none() {
                return None;
            }
            // Hearth's standard menus (the app's own come with right-click menus, not in this test yet)
            let items: Vec<Element<'a, Message>> = ["Window", "Help"].iter().map(|t| container(l.t(*t, 14.0, p.text)).padding(Padding { left: 9.0, right: 9.0, ..Padding::ZERO }).into()).collect();
            Some(line(false, items, 2.0))
        }
        "quickaccess" => simple("grid", 18.0, p.text, Message::Nothing),
        "showdesktop" => {
            let (txt, p3) = (p.text, p.panel3);
            let (w, h) = if vert { (side, 8.0) } else { (8.0, side) };
            Some(
                button(Space::new().width(Length::Fixed(w)).height(Length::Fixed(h)))
                    .padding(0)
                    .on_press(Message::ShowDesktop)
                    .style(move |_, st| button::Style {
                        background: Some(Background::Color(if matches!(st, button::Status::Hovered | button::Status::Pressed) { p3 } else { a(txt, 0.12) })),
                        border: border::rounded(3.0),
                        ..button::Style::default()
                    })
                    .into(),
            )
        }
        _ => None,
    }
}

/// Your apps in the bar (no floating dock): the dock's buttons, smaller.
pub fn apps_strip<'a>(s: &'a Shell, l: &Look, side: f32, vert: bool) -> Element<'a, Message> {
    let p = l.p;
    let mut v: Vec<Element<'a, Message>> = Vec::new();
    for (i, it) in crate::dock::items(&s.apps, &s.pins, &s.wm.wins).into_iter().enumerate() {
        let active = it.wins.iter().any(|w| w.active);
        let running = !it.wins.is_empty();
        let face = look::app_pic(it.app.as_ref().and_then(|a| a.icon_path.as_deref()), side * 0.62);
        let c = if active { p.accent } else { p.muted };
        let len = if active { 14.0 } else if running { 5.0 } else { 0.0 };
        let mark = container(Space::new().width(Length::Fixed(if vert { 3.0 } else { len })).height(Length::Fixed(if vert { len } else { 3.0 }))).style(move |_| container::Style { background: Some(c.into()), border: border::rounded(1.5), ..container::Style::default() });
        let body: Element<'a, Message> = if vert {
            stack![container(face).center(Length::Fixed(side)), container(mark).height(Length::Fixed(side)).center_y(Length::Fixed(side))].into()
        } else {
            stack![container(face).center(Length::Fixed(side)), column![Space::new().height(Length::Fill), container(mark).center_x(Length::Fixed(side))].height(Length::Fixed(side))].into()
        };
        let r = l.r(9.0);
        let b = button(body)
            .padding(0)
            .width(Length::Fixed(side))
            .height(Length::Fixed(side))
            .on_press(Message::Task(i))
            .style(move |_, st| button::Style {
                background: if matches!(st, button::Status::Hovered | button::Status::Pressed) {
                    Some(Background::Color(p.panel3))
                } else if active {
                    Some(Background::Color(a(p.text, 0.08)))
                } else {
                    None
                },
                border: border::rounded(r),
                ..button::Style::default()
            });
        v.push(mouse_area(b).on_middle_press(Message::TaskMiddle(i)).into());
    }
    line(vert, v, 2.0)
}

fn clock<'a>(s: &'a Shell, l: &Look, g: &Geom, vert: bool) -> Element<'a, Message> {
    let p = l.p;
    let one = s.desk.clock_at == "end-one" && !vert;
    let time = if vert { s.clock.split(' ').next().unwrap_or("").to_string() } else { s.clock.clone() };
    let mut c = column![l.ts(if one { format!("{}  {}", s.date, time) } else { time }, if vert { 14.0 } else if g.island_h > 40.0 { 14.0 } else { 13.0 }, p.text)].align_x(Alignment::Center);
    if !one && (vert || g.island_h > 40.0) {
        let second = if vert { s.clock.split(' ').nth(1).unwrap_or("").to_string() } else { s.date.clone() };
        if !second.is_empty() {
            c = c.push(l.t(second, 11.0, p.muted));
        }
    }
    island_v(l, g, vert, container(c).padding(Padding { left: if vert { 0.0 } else { 12.0 }, right: if vert { 0.0 } else { 12.0 }, top: if vert { 8.0 } else { 0.0 }, bottom: if vert { 8.0 } else { 0.0 } }).into(), false)
}

pub fn view(s: &Shell) -> Element<'_, Message> {
    let l = &s.look;
    let p = l.p;
    let d = &s.desk;
    let vert = d.vertical();
    let g = Geom::for_edge(l, s.compact(), vert);
    let centred = !vert && !g.shelf && !d.centre.is_empty();
    let pill_centred = !vert && !d.apps_in_bar && !g.shelf && !centred && d.clock_at.is_empty();
    let clock_last = d.clock_at == "end" || d.clock_at == "end-one";
    let show_last = d.end.last().map(String::as_str) == Some("showdesktop");
    let side = g.side();

    let mut start_ids: Vec<String> = d.start.clone();
    if vert || g.shelf {
        start_ids.extend(d.centre.iter().cloned());
    }
    let start: Vec<Element<'_, Message>> = start_ids.iter().filter_map(|id| block(s, l, &g, vert, id)).collect();
    let end_ids: Vec<&String> = if show_last { d.end[..d.end.len() - 1].iter().collect() } else { d.end.iter().collect() };
    let end: Vec<Element<'_, Message>> = end_ids.iter().filter_map(|id| block(s, l, &g, vert, id)).collect();

    let mut main: Vec<Element<'_, Message>> = Vec::new();
    if d.game_at == "start" && !l.game_mode_off {
        main.push(game_mode(l, &g));
    }
    if !start.is_empty() {
        main.push(island_v(l, &g, vert, line(vert, start, 8.0 * g.k), false));
    }
    if d.apps_in_bar && !centred {
        main.push(island_v(l, &g, vert, apps_strip(s, l, (g.island_h - if g.floating { 10.0 } else { 8.0 }).max(30.0), vert), false));
    }
    main.push(if vert { Space::new().height(Length::Fill).into() } else { Space::new().width(Length::Fill).into() });
    if !pill_centred && !g.shelf && !clock_last {
        main.push(clock(s, l, &g, vert));
    }
    if d.game_at == "before-status" && !l.game_mode_off {
        main.push(game_mode(l, &g));
    }
    if !end.is_empty() {
        main.push(island_v(l, &g, vert, line(vert, end, if g.shelf { 12.0 } else { 2.0 }), false));
    }
    if !pill_centred && !g.shelf && clock_last {
        main.push(clock(s, l, &g, vert));
    }
    if d.game_at != "start" && d.game_at != "before-status" && !l.game_mode_off {
        main.push(game_mode(l, &g));
    }
    if show_last {
        if let Some(b) = block(s, l, &g, vert, "showdesktop") {
            main.push(b);
        }
    }
    let main: Element<'_, Message> = if vert {
        Column::with_children(main).spacing(g.spacing).align_x(Alignment::Center).width(Length::Fixed(g.island_h)).into()
    } else {
        Row::with_children(main).spacing(g.spacing).align_y(Alignment::Center).height(Length::Fixed(g.island_h)).into()
    };

    // the middle: the clock pill, or a layout's middle blocks (and your apps)
    let middle: Element<'_, Message> = if pill_centred {
        let time_font = if l.gaming() || l.name == "synthwave" { look::mono() } else { l.semi() };
        let time = text(s.clock.clone()).size(if l.thin { 13.0 } else { 15.0 }).color(p.text).font(time_font);
        let mut pill_row = row![time].spacing(12).align_y(Alignment::Center);
        if !s.compact() {
            pill_row = pill_row.push(l.t(s.date.clone(), 13.0, p.muted));
        }
        let pill = container(pill_row)
            .padding(Padding { left: 14.0, right: 14.0, ..Padding::ZERO })
            .height(Length::Fixed(g.island_h))
            .align_y(Alignment::Center)
            .style(frame(l, &g, l.r(g.island_h / 2.0), false));
        container(pill).center_x(Length::Fill).height(Length::Fixed(g.island_h)).into()
    } else if centred {
        let mut v: Vec<Element<'_, Message>> = d.centre.iter().filter_map(|id| block(s, l, &g, false, id)).collect();
        if d.apps_in_bar {
            v.push(apps_strip(s, l, (g.island_h - if g.floating { 10.0 } else { 8.0 }).max(30.0), false));
        }
        container(island(l, &g, line(false, v, 8.0 * g.k), false)).center_x(Length::Fill).height(Length::Fixed(g.island_h)).into()
    } else {
        Space::new().into()
    };

    let pad = if g.style == "bar" {
        if vert {
            Padding { top: 8.0, bottom: 8.0, ..Padding::ZERO }
        } else {
            Padding { left: 8.0, right: 6.0, ..Padding::ZERO }
        }
    } else {
        Padding::ZERO
    };
    let edge = d.edge.as_str();
    let outer = Padding {
        top: if edge == "top" { g.gap_out } else if edge == "bottom" { g.gap_in } else { g.along },
        bottom: if edge == "bottom" { g.gap_out } else if edge == "top" { g.gap_in } else { g.along },
        left: if edge == "left" { g.gap_out } else if edge == "right" { g.gap_in } else { g.along },
        right: if edge == "right" { g.gap_out } else if edge == "left" { g.gap_in } else { g.along },
    };
    let inner = container(stack![container(main).padding(pad), middle]).padding(outer);
    if g.style == "bar" {
        // one full-width bar: a flat strip with a line on the side toward the windows
        let glass = l.glass();
        let line_c = p.line;
        let strip = container(inner).width(Length::Fill).height(Length::Fill).style(move |_| container::Style { background: Some(glass.into()), ..container::Style::default() });
        let rule = |w: Length, h: Length| container(Space::new()).width(w).height(h).style(move |_| container::Style { background: Some(line_c.into()), ..container::Style::default() });
        match edge {
            "bottom" => column![rule(Length::Fill, Length::Fixed(1.0)), strip].into(),
            "left" => row![strip, rule(Length::Fixed(1.0), Length::Fill)].into(),
            "right" => row![rule(Length::Fixed(1.0), Length::Fill), strip].into(),
            _ => column![strip, rule(Length::Fill, Length::Fixed(1.0))].into(),
        }
    } else {
        container(inner).width(Length::Fill).height(Length::Fill).into()
    }
}
