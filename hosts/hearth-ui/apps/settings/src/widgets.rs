//! The pieces every Settings page draws with, as the approved mockups draw them (settings-A1/A2 for the mouse look,
//! settings-B1/B2 for the controller look: `big()` is on while the controller look shows, and every piece here grows
//! for it). Pages use only these (or their own, in their own file) so the two looks stay one set of pages.

use hearth_ui::ui::{self, glyph, t, t1, tb, tm, ts, G};
use hearth_ui::Pal;
use iced::widget::{column, container, mouse_area, row, text_input, Column, Row, Space};
use iced::{mouse, Color, Element, Length, Padding};
use std::sync::atomic::{AtomicBool, Ordering};

pub type E<'a, M> = Element<'a, M>;

static BIG: AtomicBool = AtomicBool::new(false);

/// The controller look (layout B): bigger rows and text. Set by the frame before drawing.
pub fn set_big(on: bool) {
    BIG.store(on, Ordering::Relaxed);
}
pub fn big() -> bool {
    BIG.load(Ordering::Relaxed)
}
/// A size, grown for the controller look.
pub fn sz(s: f32) -> f32 {
    if big() { (s * 1.25).round() } else { s }
}

/// A switch: the accent track with the knob on the right when on, "On" / "Off" before it.
pub fn switch<'a, M: Clone + 'a>(p: &Pal, on: bool, msg: Option<M>) -> E<'a, M> {
    let p = *p;
    let k = sz(16.0);
    let knob = container(Space::new()).width(Length::Fixed(k)).height(Length::Fixed(k)).style(ui::boxed(if on { p.ink } else { p.text }, k / 2.0, None));
    let inner: E<'a, M> = if on { row![Space::new().width(Length::Fill), knob].into() } else { row![knob, Space::new().width(Length::Fill)].into() };
    let track = container(inner)
        .padding(Padding { top: 3.0, right: 4.0, bottom: 3.0, left: 4.0 })
        .width(Length::Fixed(sz(46.0)))
        .height(Length::Fixed(sz(24.0)))
        .style(ui::boxed(if on { p.accent } else { Pal::mix(p.panel2, p.text, 0.12) }, sz(12.0), None));
    let r = row![t(if on { "On" } else { "Off" }, sz(13.0), p.muted), track].spacing(12.0).align_y(iced::Center);
    match msg {
        Some(m) => mouse_area(r).on_press(m).interaction(mouse::Interaction::Pointer).into(),
        None => r.into(),
    }
}

/// Choices side by side, the picked one in the accent (Scale 100% 125% 150%, Start in Game Mode / Desktop Mode).
pub fn seg<'a, M: Clone + 'a>(p: &Pal, items: Vec<(String, bool, Option<M>)>) -> E<'a, M> {
    let p = *p;
    let mut r = Row::new().spacing(2.0);
    for (label, on, msg) in items {
        let fg = if on { p.ink } else if msg.is_some() { p.text } else { p.muted };
        let cell = container(t1(label, sz(13.0), fg).font(if on { ui::weight(iced::font::Weight::Semibold) } else { hearth_ui::app::INTER }))
            .padding(Padding { top: sz(6.0), right: 14.0, bottom: sz(6.0), left: 14.0 })
            .style(ui::boxed(if on { p.accent } else { Color::TRANSPARENT }, p.r(6.0), None));
        r = r.push(match msg {
            Some(m) if !on => E::from(mouse_area(cell).on_press(m).interaction(mouse::Interaction::Pointer)),
            _ => cell.into(),
        });
    }
    container(r).padding(3.0).style(ui::boxed(Pal::mix(p.panel, p.bg, 0.4), p.r(8.0), None)).into()
}

pub fn icon_box<'a, M: 'a>(p: &Pal, g: G) -> E<'a, M> {
    let s = sz(40.0);
    container(glyph(g, p.text, sz(18.0), 1.8)).center_x(Length::Fixed(s)).center_y(Length::Fixed(s)).style(ui::boxed(Pal::mix(p.panel, p.text, 0.06), p.r(8.0), None)).into()
}

/// A setting's row: its icon, name, the line under it, and its control on the right.
pub fn srow<'a, M: 'a>(p: &Pal, g: G, title: impl Into<String>, sub: impl Into<String>, control: E<'a, M>) -> E<'a, M> {
    let sub: String = sub.into();
    let mut words = Column::new().spacing(3.0).push(tm(title.into(), sz(15.0), p.text));
    if !sub.is_empty() {
        words = words.push(t(sub, sz(13.0), p.muted));
    }
    container(row![icon_box(p, g), words.width(Length::Fill), control].spacing(16.0).align_y(iced::Center))
        .padding(Padding { top: sz(14.0), right: 20.0, bottom: sz(14.0), left: 20.0 })
        .width(Length::Fill)
        .style(ui::boxed(p.panel, p.r(10.0), None))
        .into()
}

/// A row that opens something (another page, an app): › on the right.
pub fn link_row<'a, M: Clone + 'a>(p: &Pal, g: G, title: impl Into<String>, sub: impl Into<String>, msg: M) -> E<'a, M> {
    let r = srow(p, g, title, sub, glyph(G::Chevron, p.muted, 14.0, 2.0));
    mouse_area(r).on_press(msg).interaction(mouse::Interaction::Pointer).into()
}

/// A section's heading (Brightness & colour, Related).
pub fn head<'a, M: 'a>(p: &Pal, s: impl Into<String>) -> E<'a, M> {
    container(ts(s.into(), sz(14.0), p.muted)).padding(Padding { top: 14.0, right: 0.0, bottom: 2.0, left: 2.0 }).into()
}

pub fn card<'a, M: 'a>(p: &Pal, inner: impl Into<E<'a, M>>) -> E<'a, M> {
    container(inner).padding(22.0).width(Length::Fill).style(ui::boxed(p.panel, p.r(12.0), None)).into()
}

/// A card with a title and a link on the right (Storage · Manage).
pub fn titled_card<'a, M: Clone + 'a>(p: &Pal, title: impl Into<String>, link: Option<(&'a str, M)>, inner: impl Into<E<'a, M>>) -> E<'a, M> {
    let mut top = Row::new().push(ts(title.into(), sz(17.0), p.text)).push(Space::new().width(Length::Fill));
    if let Some((l, m)) = link {
        top = top.push(mouse_area(tm(l, sz(14.0), p.accent)).on_press(m).interaction(mouse::Interaction::Pointer));
    }
    card(p, column![top, inner.into()].spacing(14.0))
}

/// A small outlined button (Rename, About this PC, Identify).
pub fn pill<'a, M: Clone + 'a>(p: &Pal, label: impl Into<String>, accent: bool, msg: Option<M>) -> E<'a, M> {
    let c = container(t1(label.into(), sz(13.0), if accent { p.accent } else { p.muted }))
        .padding(Padding { top: 4.0, right: 12.0, bottom: 4.0, left: 12.0 })
        .style(ui::boxed(Color::TRANSPARENT, 14.0, Some((if accent { p.accent } else { p.line }, 1.0))));
    match msg {
        Some(m) => mouse_area(c).on_press(m).interaction(mouse::Interaction::Pointer).into(),
        None => c.into(),
    }
}

/// How full something is (a drive, the battery).
pub fn bar_of<'a, M: 'a>(p: &Pal, used: f64, size: f64) -> E<'a, M> {
    let f = if size > 0.0 { (used / size).clamp(0.0, 1.0) as f32 } else { 0.0 };
    ui::bar(vec![(f, p.accent)], Pal::mix(p.panel2, p.text, 0.08), 8.0, 4.0)
}

pub fn drive_rows<'a, M: 'a>(p: &Pal, drives: &[crate::data::Drive]) -> Column<'a, M> {
    let mut c = Column::new().spacing(14.0);
    for d in drives {
        c = c.push(column![
            row![t(d.name.clone(), sz(14.0), p.text), Space::new().width(Length::Fill), t(format!("{} of {}", ui::bytes(d.used as f64), ui::bytes(d.size as f64)), sz(13.0), p.muted)],
            bar_of(p, d.used as f64, d.size as f64)
        ].spacing(8.0));
    }
    c
}

pub fn input_style(p: Pal) -> impl Fn(&iced::Theme, text_input::Status) -> text_input::Style {
    move |_t, _s| text_input::Style {
        background: iced::Background::Color(Pal::mix(p.panel, p.bg, 0.5)),
        border: iced::Border { color: p.line, width: 1.0, radius: p.r(8.0).into() },
        icon: p.muted,
        placeholder: p.muted,
        value: p.text,
        selection: Pal::a(p.accent, 0.35),
    }
}

/// A text field in Settings' look.
pub fn field<'a, M: Clone + 'a>(p: &Pal, placeholder: &str, value: &str, on: impl Fn(String) -> M + 'a) -> text_input::TextInput<'a, M> {
    text_input(placeholder, value).on_input(on).size(sz(14.0)).padding(sz(9.0)).style(input_style(*p))
}

/// A plain line in the page (in red when something went wrong).
pub fn note<'a, M: 'a>(p: &Pal, s: impl Into<String>, bad: bool) -> E<'a, M> {
    container(t(s.into(), sz(13.0), if bad { p.danger } else { p.muted })).padding(Padding { top: 4.0, right: 0.0, bottom: 4.0, left: 2.0 }).into()
}

/// A home tile (Wi-Fi · Home-5G · strong).
pub fn tile<'a, M: Clone + 'a>(p: &Pal, g: G, title: &'a str, sub: String, msg: M) -> E<'a, M> {
    mouse_area(
        container(row![icon_box(p, g), column![tm(title, sz(15.0), p.text), t1(sub, sz(13.0), p.muted)].spacing(2.0)].spacing(14.0).align_y(iced::Center))
            .padding(16.0)
            .width(Length::Fill)
            .style(ui::boxed(p.panel, p.r(12.0), None)),
    )
    .on_press(msg)
    .interaction(mouse::Interaction::Pointer)
    .into()
}

/// "Name    value" (About).
pub fn fact<'a, M: 'a>(p: &Pal, k: impl Into<String>, v: impl Into<String>) -> E<'a, M> {
    row![container(t(k.into(), sz(14.0), p.muted)).width(Length::Fixed(sz(190.0))), t(v.into(), sz(14.0), p.text)].into()
}

/// A row inside a card (Recommended settings): icon, name, line, control, without a box of its own.
pub fn inner_row<'a, M: 'a>(p: &Pal, g: G, title: impl Into<String>, sub: impl Into<String>, control: E<'a, M>) -> E<'a, M> {
    row![icon_box(p, g), column![tm(title.into(), sz(15.0), p.text), t(sub.into(), sz(13.0), p.muted)].spacing(2.0).width(Length::Fill), control].spacing(16.0).align_y(iced::Center).into()
}

/// A big heading inside a page (the PC's name).
pub fn big_title<'a, M: 'a>(p: &Pal, s: impl Into<String>) -> E<'a, M> {
    tb(s.into(), sz(30.0), p.text).into()
}

/// A slider row's control: the slider and its value.
pub fn slider_u32<'a, M: Clone + 'a>(v: u32, range: std::ops::RangeInclusive<u32>, on: impl Fn(u32) -> M + 'a, release: M) -> E<'a, M> {
    iced::widget::slider(range, v, on).on_release(release).width(Length::Fixed(sz(280.0))).into()
}

pub fn slider_f32<'a, M: Clone + 'a>(v: f32, range: std::ops::RangeInclusive<f32>, step: f32, on: impl Fn(f32) -> M + 'a, release: M) -> E<'a, M> {
    iced::widget::slider(range, v, on).step(step).on_release(release).width(Length::Fixed(sz(260.0))).into()
}

/// A list row you click (a time zone, a search hit).
pub fn pick_row<'a, M: Clone + 'a>(p: &Pal, label: impl Into<String>, msg: M) -> E<'a, M> {
    mouse_area(container(t(label.into(), sz(14.0), p.text)).padding(Padding { top: 8.0, right: 14.0, bottom: 8.0, left: 14.0 }).width(Length::Fill).style(ui::boxed(p.panel, p.r(8.0), None)))
        .on_press(msg)
        .interaction(mouse::Interaction::Pointer)
        .into()
}
