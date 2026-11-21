//! The pieces Performance's "… & settings" tabs draw with, as Mantel's DeviceSettings.qml and MemorySettings.qml
//! draw them (the approved mockups tm-3 … tm-7 and task-manager-swap-settings): cards with a title, a line under it
//! and their rows; a row's title, its line and its control on the right; switches, choices side by side, a choice
//! with ▾ (it opens Task Manager's own menu under the pointer), sliders, the numbers on top, and small tables.

use hearth_ui::ui::{self, t, t1, tm, ts};
use hearth_ui::Pal;
use iced::widget::{column, container, mouse_area, row, text, Column, Row, Space};
use iced::{mouse, Color, Element, Length, Padding};

use crate::Msg;

pub type E<'a> = Element<'a, Msg>;

/// Mantel's green for "good" words (none today, tested · works, ready).
pub fn green() -> Color {
    Color::from_rgb8(0x5f, 0xd0, 0x8a)
}

/// The NEW tag: the flame colour, outlined (as the mockups and Mantel's cards draw it).
pub fn new_tag<'a>(p: &Pal) -> E<'a> {
    let p = *p;
    container(text("NEW").size(10.0).color(p.flame).font(ui::weight(iced::font::Weight::Semibold)).wrapping(text::Wrapping::None))
        .padding(Padding { top: 1.0, right: 5.0, bottom: 1.0, left: 5.0 })
        .style(ui::boxed(Color::TRANSPARENT, 4.0, Some((p.flame, 1.0))))
        .into()
}

/// A card: its title (with NEW), the line under it, then its rows.
pub fn card<'a>(p: &Pal, title: impl Into<String>, sub: impl Into<String>, fresh: bool, body: Vec<E<'a>>) -> E<'a> {
    let p = *p;
    let sub = sub.into();
    let mut head = Row::new().spacing(8.0).align_y(iced::Center).push(ts(title.into(), 16.0, p.text));
    if fresh {
        head = head.push(new_tag(&p));
    }
    let mut col = Column::new().spacing(4.0).push(head);
    if !sub.is_empty() {
        col = col.push(t(sub, 12.0, p.muted));
    }
    let mut inner = Column::new().spacing(10.0);
    for (i, b) in body.into_iter().enumerate() {
        if i > 0 {
            inner = inner.push(ui::hline(Pal::a(p.line, 0.7)));
        }
        inner = inner.push(b);
    }
    ui::card(&p, column![col, container(ui::hline(Pal::a(p.line, 0.7))).padding(Padding { top: 8.0, right: 0.0, bottom: 2.0, left: 0.0 }), inner].spacing(4.0))
        .padding(Padding { top: 16.0, right: 18.0, bottom: 16.0, left: 18.0 })
        .width(Length::Fill)
        .into()
}

/// A row inside a card: its title and line on the left, its control on the right.
pub fn line<'a>(p: &Pal, title: impl Into<String>, sub: impl Into<String>, tail: E<'a>) -> E<'a> {
    let p = *p;
    let sub = sub.into();
    let mut left = Column::new().spacing(2.0).push(t(title.into(), 14.0, p.text));
    if !sub.is_empty() {
        left = left.push(t(sub, 11.0, p.muted));
    }
    row![container(left).width(Length::Fill), tail].spacing(12.0).align_y(iced::Center).into()
}

/// A value on the right of a row (61°C · 100°C, none today).
pub fn value<'a>(p: &Pal, s: impl Into<String>, c: Option<Color>) -> E<'a> {
    tm(s.into(), 14.0, c.unwrap_or(p.text)).wrapping(text::Wrapping::None).into()
}

/// A switch (Turbo boost, Metered connection…).
pub fn toggle<'a>(p: &Pal, on: bool, msg: Option<Msg>) -> E<'a> {
    let p = *p;
    let knob = container(Space::new()).width(Length::Fixed(14.0)).height(Length::Fixed(14.0)).style(ui::boxed(if on { p.ink } else { p.muted }, 7.0, None));
    let inner: E<'a> = if on { row![Space::new().width(Length::Fill), knob].into() } else { row![knob, Space::new().width(Length::Fill)].into() };
    let track = container(inner)
        .padding(Padding { top: 4.0, right: 4.0, bottom: 4.0, left: 4.0 })
        .width(Length::Fixed(42.0))
        .height(Length::Fixed(22.0))
        .style(ui::boxed(if on { p.accent } else { Color::TRANSPARENT }, 11.0, if on { None } else { Some((p.muted, 1.0)) }));
    match msg {
        Some(m) => mouse_area(track).on_press(m).interaction(mouse::Interaction::Pointer).into(),
        None => track.into(),
    }
}

/// Choices side by side, the picked one in the accent (Battery saver · Balanced · Performance).
pub fn chips<'a>(p: &Pal, key: &str, opts: &[(String, String)], cur: &str) -> E<'a> {
    let p = *p;
    let mut r = Row::new().spacing(0.0);
    for (v, label) in opts {
        let on = v == cur;
        let cell = container(t1(label.clone(), 12.0, if on { p.ink } else { p.text }).font(if on { ui::weight(iced::font::Weight::Semibold) } else { hearth_ui::app::INTER }))
            .padding(Padding { top: 6.0, right: 11.0, bottom: 6.0, left: 11.0 })
            .style(ui::boxed(if on { p.accent } else { p.panel2 }, p.r(6.0), Some((p.line, 1.0))));
        r = r.push(mouse_area(cell).on_press(Msg::Pick(key.to_string(), v.clone())).interaction(mouse::Interaction::Pointer));
    }
    r.into()
}

/// A choice with ▾: pressing it opens the list (Task Manager's menu, under the pointer), the current one ticked.
pub fn dropdown<'a>(p: &Pal, key: &str, opts: Vec<(String, String)>, cur: &str) -> E<'a> {
    let p = *p;
    let label = opts.iter().find(|o| o.0 == cur).or_else(|| opts.first()).map(|o| o.1.clone()).unwrap_or_else(|| "–".into());
    let bx = container(t1(format!("{label}  ▾"), 13.0, p.text))
        .padding(Padding { top: 7.0, right: 12.0, bottom: 7.0, left: 12.0 })
        .style(ui::boxed(p.panel2, p.r(6.0), Some((p.line, 1.0))));
    mouse_area(bx).on_press(Msg::DropOpen(key.to_string(), opts, cur.to_string())).interaction(mouse::Interaction::Pointer).into()
}

/// A slider with its value on the right; the change is sent when you let go (Max processor speed, Power limit).
pub fn slide<'a>(key: &str, from: f32, to: f32, step: f32, v: f32, shown: String, p: &Pal) -> E<'a> {
    let k1 = key.to_string();
    let k2 = key.to_string();
    let s = iced::widget::slider(from..=to, v.clamp(from, to), move |x| Msg::Slide(k1.clone(), x)).step(step).on_release(Msg::SlideDone(k2)).width(Length::Fixed(170.0));
    row![s, container(tm(shown, 14.0, p.text).wrapping(text::Wrapping::None)).align_right(Length::Fixed(58.0))].spacing(12.0).align_y(iced::Center).into()
}

/// The numbers on top of a page (Health · Written so far · …): a label, a big value, a small word after it.
pub fn stats<'a>(p: &Pal, items: Vec<(String, String, String, Option<Color>)>) -> E<'a> {
    let p = *p;
    let mut r = Row::new().spacing(34.0);
    for (label, big, small, c) in items {
        let mut v = Row::new().spacing(6.0).align_y(iced::Bottom).push(t1(big, 26.0, c.unwrap_or(p.text)));
        if !small.is_empty() {
            v = v.push(container(t1(small, 13.0, p.muted)).padding(Padding { top: 0.0, right: 0.0, bottom: 4.0, left: 0.0 }));
        }
        r = r.push(column![t1(label, 12.0, p.muted), v].spacing(2.0));
    }
    r.wrap().into()
}

/// A cell of a small table: words, words muted (PIDs), a name with a tag, or a choice with ▾.
pub enum Cell {
    T(String),
    M(String),
    Tag(String, String),
    Drop(String, Vec<(String, String)>, String),
}

/// A small table inside a card (Busiest programs, Which programs use the network…): columns (title, width, 0 = fills;
/// right-aligned), the head in the muted colour, a faint line between rows.
pub fn table<'a>(p: &Pal, cols: &[(&str, f32, bool)], rows: Vec<Vec<Cell>>) -> E<'a> {
    let p = *p;
    // 0 fills, a negative width is a share of what's left (-2: twice a 0's), else pixels
    let w = |x: f32| if x == 0.0 { Length::Fill } else if x < 0.0 { Length::FillPortion((-x) as u16) } else { Length::Fixed(x) };
    let mut head = Row::new().spacing(12.0).padding(Padding { top: 0.0, right: 8.0, bottom: 0.0, left: 8.0 });
    for (title, width, right) in cols {
        let c = t1(title.to_string(), 13.0, p.muted);
        head = head.push(if *right { container(c).align_right(w(*width)) } else { container(c).width(w(*width)) });
    }
    let mut col = Column::new().push(container(head).height(Length::Fixed(32.0)).center_y(Length::Fixed(32.0)));
    for r in rows {
        let mut line = Row::new().spacing(12.0).align_y(iced::Center).padding(Padding { top: 0.0, right: 8.0, bottom: 0.0, left: 8.0 });
        for ((_, width, right), cell) in cols.iter().zip(r.into_iter()) {
            let el: E<'a> = match cell {
                Cell::T(s) => t1(s, 13.0, p.text).into(),
                Cell::M(s) => t1(s, 13.0, p.muted).into(),
                Cell::Tag(s, tag) => {
                    let mut rr = Row::new().spacing(8.0).align_y(iced::Center).push(t1(s, 13.0, p.text));
                    if !tag.is_empty() {
                        rr = rr.push(
                            container(t1(tag, 11.0, p.accent))
                                .padding(Padding { top: 2.0, right: 6.0, bottom: 2.0, left: 6.0 })
                                .style(ui::boxed(Pal::a(p.accent, 0.18), p.r(4.0), Some((p.accent, 1.0)))),
                        );
                    }
                    rr.into()
                }
                Cell::Drop(key, opts, cur) => dropdown(&p, &key, opts, &cur),
            };
            line = line.push(if *right { container(el).align_right(w(*width)).clip(true) } else { container(el).width(w(*width)).clip(true) });
        }
        col = col.push(ui::hline(Pal::a(p.line, 0.6)));
        col = col.push(container(line).height(Length::Fixed(38.0)).center_y(Length::Fixed(38.0)));
    }
    col.into()
}

/// Two cards side by side (one under the other when the window is narrow is left to the window: Task Manager's
/// detail pane is always wide enough at its smallest size for two).
pub fn pair<'a>(a: E<'a>, b: E<'a>) -> E<'a> {
    row![container(a).width(Length::FillPortion(1)), container(b).width(Length::FillPortion(1))].spacing(14.0).into()
}

/// A rounded outline with words (the Pressure / Zram / Drive swap pills on Swap & settings).
pub fn pill<'a>(p: &Pal, s: impl Into<String>, tone: Option<Color>) -> E<'a> {
    let p = *p;
    let c = tone.unwrap_or(p.line);
    container(t1(s.into(), 12.0, tone.unwrap_or(p.text)))
        .padding(Padding { top: 5.0, right: 11.0, bottom: 5.0, left: 11.0 })
        .style(ui::boxed(Color::TRANSPARENT, 14.0, Some((c, 1.0))))
        .into()
}

/// A link-like word in the accent (+ Add a program, Change in Swap & settings ›).
pub fn link<'a>(p: &Pal, s: impl Into<String>, msg: Msg) -> E<'a> {
    mouse_area(t1(s.into(), 14.0, p.accent)).on_press(msg).interaction(mouse::Interaction::Pointer).into()
}

pub fn spacer<'a>() -> E<'a> {
    Space::new().width(Length::Fill).into()
}
