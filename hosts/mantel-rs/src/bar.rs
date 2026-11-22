//! The bar along the top, drawn as the QML Mantel's Islands.qml (Mantel 2026): floating islands (Hearth, Gaming,
//! Glass, Synthwave, Cabin), one full-width bar (Steam-style; Zen's is thin) or Console's bigger one, each size and gap
//! as there:
//!   start   the flame (opens the app menu), the desktops (pills, the one you're on long and lit), the window in front
//!   centre  the clock pill: the time and the date
//!   end     the status island (Discord, keyboard, network, Bluetooth, volume, battery, notifications), then the
//!           Game Mode button, always there.

use iced::widget::{button, column, container, mouse_area, row, stack, text, Space};
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

/// The height of the bar's surface (and the space it keeps for itself at the top).
pub fn band(l: &Look, compact: bool) -> u32 {
    Geom::new(l, compact).band().round() as u32
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

fn status<'a>(s: &Shell, l: &Look, g: &Geom) -> Element<'a, Message> {
    let p = l.p;
    let side = g.side();
    let k = g.k;
    let compact = s.compact() && !g.shelf;
    let mut r = row![].spacing(if g.shelf { 12.0 } else { 2.0 }).align_y(Alignment::Center);
    // (Apps' tray icons: not in this test yet.)
    if !compact {
        r = r.push(ibtn(l, side, side, look::icon("chat", 20.0 * k, p.text), Message::Nothing)); // Discord
        r = r.push(ibtn(l, side, side, look::icon("kbd", 20.0 * k, p.text), Message::Nothing)); // keyboard
    }
    let net = match s.status.wifi {
        Some(-1) => "wired",
        Some(_) => "wifi",
        None => "offline",
    };
    r = r.push(ibtn(l, side, side, look::icon(net, 19.0 * k, p.text), Message::Nothing));
    if s.bt.present && !compact {
        let c = if !s.bt.powered { p.muted } else { p.text };
        r = r.push(ibtn(l, side, side, look::icon("bt", 18.0 * k, c), Message::Nothing));
    }
    if let Some((pct, muted)) = s.status.volume {
        let name = if muted { "vol-mute" } else if pct > 33 { "vol" } else { "vol-low" };
        let b = ibtn(l, side, side, look::icon(name, 19.0 * k, p.text), Message::Nothing);
        r = r.push(mouse_area(b).on_middle_press(Message::VolMute).on_scroll(|d| {
            let y = match d {
                iced::mouse::ScrollDelta::Lines { y, .. } | iced::mouse::ScrollDelta::Pixels { y, .. } => y,
            };
            Message::VolScroll(y > 0.0)
        }));
    }
    if let Some((pct, charging)) = s.status.battery {
        let name = if charging { "batt-charging" } else if pct > 20 { "batt" } else { "batt-low" };
        let label = l.t(format!("{pct}%"), (13.0 * k).round(), p.text);
        let w = side + 4.0 + 13.0 * 0.6 * format!("{pct}%").len() as f32;
        let inner: Element<'_, Message> = row![Space::new().width(Length::Fixed((side - 19.0 * k) / 2.0)), look::icon(name, 19.0 * k, p.text), Space::new().width(Length::Fixed(((side - 19.0 * k) / 2.0 - 6.0).max(0.0))), label]
            .align_y(Alignment::Center)
            .into();
        r = r.push(ibtn(l, side, w, inner, Message::Nothing));
    }
    if !compact {
        r = r.push(ibtn(l, side, side, look::icon("bell", 19.0 * k, p.text), Message::Nothing));
    }
    island(l, g, r.into(), false)
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

pub fn view(s: &Shell) -> Element<'_, Message> {
    let l = &s.look;
    let p = l.p;
    let g = Geom::new(l, s.compact());
    let side = g.side();

    // start: the flame, the desktops, the window in front
    let mut start = row![ibtn(l, side, side, look::flame(26.0 * g.k), Message::Flame)].spacing(8.0 * g.k).align_y(Alignment::Center);
    if let Some(d) = desks(s, l, &g) {
        start = start.push(d);
    }
    if !s.compact() && !g.shelf {
        if let Some(w) = s.wm.wins.iter().find(|w| w.active && !w.title.is_empty()) {
            let app = crate::apps::for_window(&s.apps, &w.app_id);
            let title = hearth_ui::ui::fit(&w.title, 38);
            let t = if l.gaming() { l.ts(title.to_uppercase(), 13.0, p.text) } else { l.ts(title, 13.0, p.text) };
            start = start.push(row![look::app_pic(app.and_then(|a| a.icon_path.as_deref()), 20.0), t].spacing(8).align_y(Alignment::Center));
        }
    }

    let mut main = row![island(l, &g, start.into(), false), Space::new().width(Length::Fill), status(s, l, &g)];
    if !l.game_mode_off {
        main = main.push(game_mode(l, &g));
    }
    let main = main
        .spacing(g.spacing)
        .align_y(Alignment::Center)
        .height(Length::Fixed(g.island_h));

    // the centre pill: the time and the date
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
    let centre = container(pill).center_x(Length::Fill).height(Length::Fixed(g.island_h));

    let pad = if g.style == "bar" { Padding { left: 8.0, right: 6.0, ..Padding::ZERO } } else { Padding::ZERO };
    let inner = container(stack![container(main).padding(pad), centre]).padding(Padding { top: g.gap_out, bottom: g.gap_in, left: g.along, right: g.along });
    if g.style == "bar" {
        // one full-width bar: a flat strip with a line on the side toward the windows
        let glass = l.glass();
        let line = p.line;
        column![
            container(inner).width(Length::Fill).height(Length::Fixed(g.band() - 1.0)).style(move |_| container::Style { background: Some(glass.into()), ..container::Style::default() }),
            container(Space::new()).width(Length::Fill).height(Length::Fixed(1.0)).style(move |_| container::Style { background: Some(line.into()), ..container::Style::default() }),
        ]
        .into()
    } else {
        container(inner).width(Length::Fill).height(Length::Fill).into()
    }
}
