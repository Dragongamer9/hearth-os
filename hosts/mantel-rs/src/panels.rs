//! The extra panels (ExtraPanels.qml; bar.json "p:<id>", made by a desktop layout or Arrange the bar): a bar on any
//! free edge holding what you put in it, full width, floating or centred, each keeping its own room.
//!   menu, apps, windows, pager, spacer, clock, tray (apps' tray icons: not in this test yet), status, notifications,
//!   gamemode, the bar widgets (not in this test yet) and the layouts' own blocks (bar.rs block()).

use iced::widget::{button, column, container, row, Column, Row, Space};
use iced::{border, Alignment, Background, Element, Length, Padding};

use crate::bar::{self, Geom};
use crate::desk::Panel;
use crate::look::{self, a};
use crate::shell::{Message, Shell};

/// The panel's surface thickness: its size, and the gaps a floating or centred one keeps.
pub fn thickness(p: &Panel) -> u32 {
    p.size + if p.style.is_empty() { 0 } else { 16 }
}

fn pbtn<'a>(s: &Shell, side: f32, face: Element<'a, Message>, lit: bool, msg: Message) -> Element<'a, Message> {
    let p = s.look.p;
    let r = s.look.r(9.0);
    button(container(face).center(Length::Fixed(side)))
        .padding(0)
        .on_press(msg)
        .style(move |_, st| button::Style {
            background: if matches!(st, button::Status::Hovered | button::Status::Pressed) {
                Some(Background::Color(p.panel3))
            } else if lit {
                Some(Background::Color(a(p.text, 0.1)))
            } else {
                None
            },
            border: border::rounded(r),
            text_color: p.text,
            ..button::Style::default()
        })
        .into()
}

fn item<'a>(s: &'a Shell, pn: &Panel, id: &str) -> Option<Element<'a, Message>> {
    let l = &s.look;
    let p = l.p;
    let vert = pn.edge == "left" || pn.edge == "right";
    let side = 34.0f32.min(pn.size as f32 - 8.0).max(22.0);
    match id {
        "spacer" => Some(if vert { Space::new().height(Length::Fill).into() } else { Space::new().width(Length::Fill).into() }),
        "menu" => Some(pbtn(s, side, look::flame(side * 0.62), false, Message::Flame)),
        "apps" => Some(bar::apps_strip(s, l, side, vert)),
        "windows" => {
            let v: Vec<Element<'a, Message>> = s
                .wm
                .wins
                .iter()
                .filter(|w| !w.title.is_empty() || !w.app_id.is_empty())
                .take(14)
                .map(|w| {
                    let app = crate::apps::for_window(&s.apps, &w.app_id);
                    pbtn(s, side, look::app_pic(app.and_then(|a| a.icon_path.as_deref()), side * 0.62), w.active, Message::Win(w.id))
                })
                .collect();
            Some(bar::line(vert, v, 2.0))
        }
        "pager" => {
            let v: Vec<Element<'a, Message>> = s.wm.desks.iter().enumerate().map(|(i, d)| pbtn(s, side * 0.8, l.t((i + 1).to_string(), 12.0, if d.active { p.accent } else { p.muted }).into(), d.active, Message::Desk(d.id))).collect();
            Some(bar::line(vert, v, 2.0))
        }
        "clock" => {
            let mut c = column![l.ts(if vert { s.clock.split(' ').next().unwrap_or("").to_string() } else { s.clock.clone() }, 14.0, p.text)].align_x(Alignment::Center);
            c = c.push(l.t(s.date.clone(), if vert { 9.0 } else { 10.0 }, p.muted));
            Some(container(c).padding(Padding { left: 8.0, right: 8.0, ..Padding::ZERO }).into())
        }
        "status" => {
            let mut v: Vec<Element<'a, Message>> = vec![look::icon(match s.status.wifi { Some(-1) => "wired", Some(_) => "wifi", None => "offline" }, 16.0, p.text)];
            if let Some((_, muted)) = s.status.volume {
                v.push(look::icon(if muted { "vol-mute" } else { "vol" }, 16.0, p.text));
            }
            if s.status.battery.is_some() {
                v.push(look::icon("batt", 16.0, p.text));
            }
            Some(container(bar::line(vert, v, 6.0)).padding(8).into())
        }
        "notifications" => Some(pbtn(s, side, look::icon("bell", 17.0, p.text), false, Message::Nothing)),
        "gamemode" => {
            if l.game_mode_off {
                return None;
            }
            let ink = p.ink;
            let face: Element<'a, Message> = if vert { look::icon("pad", 16.0, ink) } else { row![look::icon("pad", 16.0, ink), l.tb("Game Mode", 12.0, ink)].spacing(6).align_y(Alignment::Center).into() };
            let (ac, r) = (p.accent, l.r(15.0));
            Some(
                button(container(face).center_y(Length::Fixed(30.0)).padding(Padding { left: if vert { 9.0 } else { 12.0 }, right: if vert { 9.0 } else { 12.0 }, ..Padding::ZERO }))
                    .padding(0)
                    .on_press(Message::GameMode)
                    .style(move |_, st| button::Style {
                        background: Some(Background::Color(if matches!(st, button::Status::Hovered | button::Status::Pressed) { look::lighter(ac, 1.12) } else { ac })),
                        border: border::rounded(r),
                        text_color: ink,
                        ..button::Style::default()
                    })
                    .into(),
            )
        }
        x if ["overview", "taskview", "appname", "appmenus", "quickaccess", "showdesktop"].contains(&x) => {
            let g = Geom { island_h: pn.size as f32, ..Geom::new(l, false) };
            bar::block(s, l, &g, vert, x)
        }
        _ => None,
    }
}

pub fn view<'a>(s: &'a Shell, i: usize) -> Element<'a, Message> {
    let Some(pn) = s.desk.panels.get(i) else { return Space::new().into() };
    let l = &s.look;
    let p = l.p;
    let vert = pn.edge == "left" || pn.edge == "right";
    let v: Vec<Element<'a, Message>> = pn.items.iter().filter_map(|id| item(s, pn, id)).collect();
    let body: Element<'a, Message> = if vert {
        Column::with_children(v).spacing(6).align_x(Alignment::Center).height(Length::Fill).into()
    } else {
        Row::with_children(v).spacing(6).align_y(Alignment::Center).width(Length::Fill).into()
    };
    let floats = !pn.style.is_empty();
    let centred = pn.style == "centred";
    let glass = l.glass();
    let (line_c, r) = (p.line, if floats { l.r((pn.size as f32 / 2.0).min(14.0)) } else { 0.0 });
    let mut c = container(body)
        .padding(if vert { Padding { top: 8.0, bottom: 8.0, left: 4.0, right: 4.0 } } else { Padding { left: 8.0, right: 8.0, top: 4.0, bottom: 4.0 } })
        .style(move |_| container::Style {
            background: Some(glass.into()),
            border: iced::Border { color: if floats { line_c } else { iced::Color::TRANSPARENT }, width: if floats { 1.0 } else { 0.0 }, radius: r.into() },
            ..container::Style::default()
        });
    let size = Length::Fixed(pn.size as f32);
    c = if vert { c.width(size).height(if centred { Length::Shrink } else { Length::Fill }) } else { c.height(size).width(if centred { Length::Shrink } else { Length::Fill }) };
    let gap = if floats { 8.0 } else { 0.0 };
    let wrap = container(c).padding(gap).width(Length::Fill).height(Length::Fill);
    if vert {
        wrap.center_y(Length::Fill).into()
    } else {
        wrap.center_x(Length::Fill).into()
    }
}
