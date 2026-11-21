//! The pieces every Settings page draws with, as the approved mockups draw them (settings-A1/A2 for the mouse look,
//! settings-B1/B2 for the controller look: `big()` is on while the controller look shows, and every piece here grows
//! for it). Pages use only these (or their own, in their own file) so the two looks stay one set of pages.
//!
//! Every piece that can be changed or opened is also a place for the controller's focus (nav.rs): a row (srow,
//! inner_row, link_row, tile, pick_row) is one place, and the control in it (switch, seg, a slider, btn, pill, field)
//! says what A, left and right do there. The focused row is drawn lifted with the accent outline. For the controller to
//! press a page's buttons, draw them with `btn` from here (the same arguments as hearth_ui's, so `use crate::widgets::*`
//! and leaving `btn` out of the `hearth_ui::ui::{...}` list is all it takes).

use crate::nav::{self, Kind};
use hearth_ui::ui::{self, glyph, t, t1, tb, tm, ts, Kind as BtnKind, G};
use hearth_ui::Pal;
use iced::widget::text::IntoFragment;
use iced::widget::{column, container, mouse_area, row, slider, text_input, Column, Row, Space};
use iced::{mouse, Background, Border, Color, Element, Length, Padding, Shadow, Theme};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

pub type E<'a, M> = Element<'a, M>;

static BIG: AtomicBool = AtomicBool::new(false);
static PAL: Mutex<Option<Pal>> = Mutex::new(None);

/// The controller look (layout B): bigger rows and text. Set by the frame before drawing.
pub fn set_big(on: bool) {
    BIG.store(on, Ordering::Relaxed);
}
pub fn big() -> bool {
    BIG.load(Ordering::Relaxed)
}
/// The look in use, for the pieces that aren't given it (sliders). Set by the frame before drawing.
pub fn set_pal(p: Pal) {
    *PAL.lock().unwrap_or_else(|e| e.into_inner()) = Some(p);
}
fn pal() -> Pal {
    PAL.lock().unwrap_or_else(|e| e.into_inner()).unwrap_or_else(hearth_ui::theme::load)
}
/// A size, grown for the controller look.
pub fn sz(s: f32) -> f32 {
    if big() { (s * 1.2).round() } else { s }
}

fn act<M: Send + 'static>(m: M) -> nav::Act {
    Box::new(m)
}

/// The focused row's look: lifted with the accent outline and a soft glow (B1/B2's "the focused tile lifted with the
/// accent outline"; A2's outlined row), else `bg` with the card's faint edge.
pub fn row_style(p: Pal, bg: Color, r: f32, lit: bool) -> impl Fn(&Theme) -> container::Style {
    let b = big();
    move |_| container::Style {
        text_color: None,
        background: Some(Background::Color(if lit && b { Pal::mix(bg, p.accent, 0.12) } else { bg })),
        border: Border {
            color: if lit { p.accent } else if bg.a > 0.0 && b { Pal::a(p.text, 0.05) } else { Color::TRANSPARENT },
            width: if lit { if b { 3.0 } else { 2.0 } } else if b && bg.a > 0.0 { 1.0 } else { 0.0 },
            radius: r.into(),
        },
        // No shadow: a blur reaches outside the widget, and when only part of the window is drawn again it leaves
        // marks there. The controller look's second ring is `ring` instead.
        shadow: Shadow::default(),
        snap: true,
    }
}

/// The controller look's focus ring around a row (Mantel's SetBase: a soft accent ring just outside the outline),
/// drawn in a 3 px margin the row always has, so nothing moves when the focus does. The other look: the row alone.
pub fn ring<'a, M: 'a>(p: &Pal, lit: bool, r: f32, el: impl Into<E<'a, M>>) -> E<'a, M> {
    if !big() {
        return el.into();
    }
    let p = *p;
    container(el)
        .padding(3.0)
        .style(move |_: &Theme| container::Style {
            border: Border { color: if lit { Pal::a(p.accent, 0.3) } else { Color::TRANSPARENT }, width: 3.0, radius: (r + 3.0).into() },
            ..container::Style::default()
        })
        .into()
}

/// A control's own box: outlined when it's the focused place and no row took it (a button alone in a card). Asked
/// while drawing, so it knows whether a row took it.
fn own<'a, M: 'a>(p: Pal, slot: usize, r: f32, el: impl Into<E<'a, M>>) -> E<'a, M> {
    container(el)
        .id(nav::slot_id(slot))
        .style(move |_: &Theme| {
            let lit = nav::slot_lit(slot);
            container::Style {
                border: Border { color: if lit { p.accent } else { Color::TRANSPARENT }, width: if lit { 2.0 } else { 0.0 }, radius: r.into() },
                ..container::Style::default()
            }
        })
        .into()
}

/// A switch: the accent track with the knob on the right when on, "On" / "Off" before it (the controller look: the
/// switch alone, as settings-B2 draws it). A turns it on or off.
pub fn switch<'a, M: Clone + Send + 'static>(p: &Pal, on: bool, msg: Option<M>) -> E<'a, M> {
    let p = *p;
    let b = big();
    let slot = nav::control(Kind::Toggle(on), msg.iter().cloned().map(act).collect(), Vec::new(), Vec::new(), String::new(), if on { "On".into() } else { "Off".into() });
    let k = if b { 20.0 } else { 16.0 };
    let knob = container(Space::new()).width(Length::Fixed(k)).height(Length::Fixed(k)).style(ui::boxed(if on { p.ink } else if b { Pal::mix(p.text, p.muted, 0.3) } else { p.text }, k / 2.0, None));
    let inner: E<'a, M> = if on { row![Space::new().width(Length::Fill), knob].into() } else { row![knob, Space::new().width(Length::Fill)].into() };
    let track = container(inner)
        .padding(Padding { top: 4.0, right: 4.0, bottom: 4.0, left: 4.0 })
        .width(Length::Fixed(if b { 52.0 } else { 46.0 }))
        .height(Length::Fixed(if b { 28.0 } else { 24.0 }))
        .style(ui::boxed(if on { p.accent } else { Pal::mix(p.panel2, p.text, 0.12) }, if b { 14.0 } else { 12.0 }, None));
    let r: E<'a, M> = if b { track.into() } else { row![t(if on { "On" } else { "Off" }, sz(13.0), p.muted), track].spacing(12.0).align_y(iced::Center).into() };
    let r = match msg {
        Some(m) => mouse_area(r).on_press(m).interaction(mouse::Interaction::Pointer).into(),
        None => r,
    };
    own(p, slot, 14.0, r)
}

/// What ‹ and › pick (the next pickable choice either way), and what A picks (the next one, round).
fn choice_acts<M: Clone + Send + 'static>(items: &[(String, bool, Option<M>)]) -> (usize, Option<M>, Option<M>, Option<M>) {
    let idx = items.iter().position(|x| x.1).unwrap_or(0);
    let prev = items[..idx.min(items.len())].iter().rev().find_map(|x| x.2.clone());
    let next = items.iter().skip(idx + 1).find_map(|x| x.2.clone());
    let n = items.len();
    let round = (1..n).map(|k| (idx + k) % n).find_map(|i| items[i].2.clone());
    (idx, prev, next, round)
}

fn register_choice<M: Clone + Send + 'static>(items: &[(String, bool, Option<M>)]) -> (usize, usize, Option<M>, Option<M>) {
    let (idx, prev, next, round) = choice_acts(items);
    let label = items.get(idx).map(|x| x.0.clone()).unwrap_or_default();
    let slot = nav::control(Kind::Choice, round.into_iter().map(act).collect(), prev.iter().cloned().map(act).collect(), next.iter().cloned().map(act).collect(), String::new(), label);
    (slot, idx, prev, next)
}

/// "‹ 2560 × 1440 ›": the controller look's choices (settings-B2), changed with left and right (or a click on ‹ ›).
pub fn stepper<'a, M: Clone + 'a>(p: &Pal, label: impl Into<String>, prev: Option<M>, next: Option<M>) -> E<'a, M> {
    let p = *p;
    let arrow = |s: &'static str, m: Option<M>| -> E<'a, M> {
        let on = m.is_some();
        let a = container(tb(s, 22.0, if on { p.accent } else { Pal::a(p.accent, 0.35) })).padding(Padding { top: 0.0, right: 4.0, bottom: 3.0, left: 4.0 });
        match m {
            Some(m) => mouse_area(a).on_press(m).interaction(mouse::Interaction::Pointer).into(),
            None => a.into(),
        }
    };
    row![arrow("‹", prev), ts(label.into(), 17.0, p.accent), arrow("›", next)].spacing(6.0).align_y(iced::Center).into()
}

fn segmented<'a, M: Clone + 'a>(p: Pal, items: Vec<(String, bool, Option<M>)>) -> E<'a, M> {
    let b = big();
    let mut r = Row::new().spacing(2.0);
    for (label, on, msg) in items {
        let fg = if on { p.ink } else if msg.is_some() { if b { p.muted } else { p.text } } else { Pal::a(p.muted, 0.6) };
        let cell = container(t1(label, sz(13.0), fg).font(if on { ui::weight(iced::font::Weight::Semibold) } else { hearth_ui::app::INTER }))
            .padding(Padding { top: sz(6.0), right: if b { 18.0 } else { 14.0 }, bottom: sz(6.0), left: if b { 18.0 } else { 14.0 } })
            .style(ui::boxed(if on { p.accent } else { Color::TRANSPARENT }, p.r(if b { 8.0 } else { 6.0 }), None));
        r = r.push(match msg {
            Some(m) if !on => E::from(mouse_area(cell).on_press(m).interaction(mouse::Interaction::Pointer)),
            _ => cell.into(),
        });
    }
    container(r).padding(3.0).style(ui::boxed(Pal::mix(p.panel, p.bg, 0.4), p.r(if b { 10.0 } else { 8.0 }), None)).into()
}

/// Choices side by side, the picked one in the accent (Scale 100% 125% 150%, Start in Game Mode / Desktop Mode). The
/// controller look shows them as ‹ value › (settings-B2); left and right pick, A the next one.
pub fn seg<'a, M: Clone + Send + 'static>(p: &Pal, items: Vec<(String, bool, Option<M>)>) -> E<'a, M> {
    let p = *p;
    let (slot, idx, prev, next) = register_choice(&items);
    let el = if big() { stepper(&p, items.get(idx).map(|x| x.0.clone()).unwrap_or_default(), prev, next) } else { segmented(p, items) };
    own(p, slot, p.r(8.0), el)
}

/// Choices side by side in both looks (the Quick tab's Start in, settings-B1); left and right pick.
pub fn seg_flat<'a, M: Clone + Send + 'static>(p: &Pal, items: Vec<(String, bool, Option<M>)>) -> E<'a, M> {
    let p = *p;
    let (slot, _, _, _) = register_choice(&items);
    own(p, slot, p.r(8.0), segmented(p, items))
}

pub fn icon_box<'a, M: 'a>(p: &Pal, g: G) -> E<'a, M> {
    let s = sz(40.0);
    container(glyph(g, p.text, sz(18.0), 1.8)).center_x(Length::Fixed(s)).center_y(Length::Fixed(s)).style(ui::boxed(Pal::mix(p.panel, p.text, 0.06), p.r(8.0), None)).into()
}

/// A setting's row: its icon, name, the line under it, and its control on the right. The controller look's rows are
/// bigger and have no icon (settings-B2); a slider's value shows next to it there, not under the name.
pub fn srow<'a, M: 'a>(p: &Pal, g: G, title: impl Into<String>, sub: impl Into<String>, control: E<'a, M>) -> E<'a, M> {
    let (title, sub): (String, String) = (title.into(), sub.into());
    let r = nav::item(&title, &sub, false);
    let b = big();
    let mut words = Column::new().spacing(if b { 4.0 } else { 3.0 }).push(tm(title, if b { 18.0 } else { 15.0 }, p.text));
    if !sub.is_empty() && !(b && r.slider_value.as_deref() == Some(sub.as_str())) {
        words = words.push(t(sub, if b { 14.0 } else { 13.0 }, p.muted));
    }
    let line: E<'a, M> = if b { row![words.width(Length::Fill), control].spacing(18.0).align_y(iced::Center).into() } else { row![icon_box(p, g), words.width(Length::Fill), control].spacing(16.0).align_y(iced::Center).into() };
    let row = container(line)
        .id(r.key)
        .padding(if b { Padding { top: 16.0, right: 24.0, bottom: 16.0, left: 24.0 } } else { Padding { top: 14.0, right: 20.0, bottom: 14.0, left: 20.0 } })
        .width(Length::Fill)
        .style(row_style(*p, p.panel, p.r(if b { 14.0 } else { 10.0 }), r.lit));
    ring(p, r.lit, p.r(14.0), row)
}

/// A row that opens something (another page, an app): › on the right (in the accent in the controller look).
pub fn link_row<'a, M: Clone + Send + 'static>(p: &Pal, g: G, title: impl Into<String>, sub: impl Into<String>, msg: M) -> E<'a, M> {
    nav::control(Kind::Open, vec![act(msg.clone())], Vec::new(), Vec::new(), String::new(), String::new());
    let b = big();
    let r = srow(p, g, title, sub, glyph(G::Chevron, if b { p.accent } else { p.muted }, if b { 18.0 } else { 14.0 }, if b { 2.6 } else { 2.0 }));
    mouse_area(r).on_press(msg).interaction(mouse::Interaction::Pointer).into()
}

/// A section's heading (Brightness & colour, Related).
pub fn head<'a, M: 'a>(p: &Pal, s: impl Into<String>) -> E<'a, M> {
    nav::flush();
    container(ts(s.into(), sz(14.0), p.muted)).padding(Padding { top: 14.0, right: 0.0, bottom: 2.0, left: 2.0 }).into()
}

pub fn card<'a, M: 'a>(p: &Pal, inner: impl Into<E<'a, M>>) -> E<'a, M> {
    nav::flush();
    container(inner).padding(22.0).width(Length::Fill).style(row_style(*p, p.panel, p.r(if big() { 14.0 } else { 12.0 }), false)).into()
}

/// A card with a title and a link on the right (Storage · Manage).
pub fn titled_card<'a, M: Clone + Send + 'static>(p: &Pal, title: impl Into<String>, link: Option<(&'a str, M)>, inner: impl Into<E<'a, M>>) -> E<'a, M> {
    let title: String = title.into();
    let mut top = Row::new().push(ts(title.clone(), sz(17.0), p.text)).push(Space::new().width(Length::Fill));
    if let Some((l, m)) = link {
        let slot = nav::control(Kind::Open, vec![act(m.clone())], Vec::new(), Vec::new(), format!("{title} · {l}"), String::new());
        top = top.push(own(*p, slot, 6.0, mouse_area(container(tm(l, sz(14.0), p.accent)).padding(Padding { top: 1.0, right: 4.0, bottom: 1.0, left: 4.0 })).on_press(m).interaction(mouse::Interaction::Pointer)));
    }
    card(p, column![top, inner.into()].spacing(14.0))
}

/// A small outlined button (Rename, About this PC, Identify).
pub fn pill<'a, M: Clone + Send + 'static>(p: &Pal, label: impl Into<String>, accent: bool, msg: Option<M>) -> E<'a, M> {
    let label: String = label.into();
    let slot = nav::control(Kind::Press, msg.iter().cloned().map(act).collect(), Vec::new(), Vec::new(), label.clone(), String::new());
    let c = container(t1(label, sz(13.0), if accent { p.accent } else { p.muted }))
        .padding(Padding { top: 4.0, right: 12.0, bottom: 4.0, left: 12.0 })
        .style(ui::boxed(Color::TRANSPARENT, 14.0, Some((if accent { p.accent } else { p.line }, 1.0))));
    let el: E<'a, M> = match msg {
        Some(m) => mouse_area(c).on_press(m).interaction(mouse::Interaction::Pointer).into(),
        None => c.into(),
    };
    own(*p, slot, 16.0, el)
}

/// A button, as hearth_ui's (same arguments), that the controller can press too (A, when it's the focused place or
/// the control of the focused row).
pub fn btn<'a, M: Clone + Send + 'static>(p: &Pal, label: impl IntoFragment<'a>, kind: BtnKind, focused: bool, on: Option<M>) -> E<'a, M> {
    let label = label.into_fragment();
    let slot = nav::control(Kind::Press, on.iter().cloned().map(act).collect(), Vec::new(), Vec::new(), label.to_string(), String::new());
    own(*p, slot, p.r(6.0) + 3.0, ui::btn(p, label, kind, focused, on))
}

/// The same button, without the focus argument.
pub fn button<'a, M: Clone + Send + 'static>(p: &Pal, label: impl IntoFragment<'a>, kind: BtnKind, on: Option<M>) -> E<'a, M> {
    btn(p, label, kind, false, on)
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

/// A text field in Settings' look. With the controller, A on it puts the typing there (and the on-screen keyboard
/// comes with it, as hearth-pad gives it for a text field).
pub fn field<'a, M: Clone + 'a>(p: &Pal, placeholder: &str, value: &str, on: impl Fn(String) -> M + 'a) -> text_input::TextInput<'a, M> {
    let p = *p;
    let slot = nav::control(Kind::Field, Vec::new(), Vec::new(), Vec::new(), placeholder.to_string(), value.to_string());
    let base = input_style(p);
    text_input(placeholder, value).id(nav::field_id(slot)).on_input(on).size(sz(14.0)).padding(sz(9.0)).style(move |th, st| {
        let mut s = base(th, st);
        if nav::slot_lit(slot) || matches!(st, text_input::Status::Focused { .. }) {
            s.border = iced::Border { color: p.accent, width: 2.0, radius: p.r(8.0).into() };
        }
        s
    })
}

/// A plain line in the page (in red when something went wrong).
pub fn note<'a, M: 'a>(p: &Pal, s: impl Into<String>, bad: bool) -> E<'a, M> {
    container(t(s.into(), sz(13.0), if bad { p.danger } else { p.muted })).padding(Padding { top: 4.0, right: 0.0, bottom: 4.0, left: 2.0 }).into()
}

/// A home tile (Wi-Fi · Home-5G · strong).
pub fn tile<'a, M: Clone + Send + 'static>(p: &Pal, g: G, title: &'a str, sub: String, msg: M) -> E<'a, M> {
    nav::control(Kind::Open, vec![act(msg.clone())], Vec::new(), Vec::new(), String::new(), String::new());
    let r = nav::item(title, &sub, true);
    mouse_area(ring(
        p,
        r.lit,
        p.r(12.0),
        container(row![icon_box(p, g), column![tm(title, sz(15.0), p.text), t1(sub, sz(13.0), p.muted)].spacing(2.0)].spacing(14.0).align_y(iced::Center))
            .id(r.key)
            .padding(16.0)
            .width(Length::Fill)
            .style(row_style(*p, p.panel, p.r(12.0), r.lit)),
    ))
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
    let (title, sub): (String, String) = (title.into(), sub.into());
    let r = nav::item(&title, &sub, false);
    let words = column![tm(title, sz(15.0), p.text), t(sub, sz(13.0), p.muted)].spacing(2.0).width(Length::Fill);
    let line: E<'a, M> = if big() { row![words, control].spacing(16.0).align_y(iced::Center).into() } else { row![icon_box(p, g), words, control].spacing(16.0).align_y(iced::Center).into() };
    ring(p, r.lit, p.r(10.0), container(line).id(r.key).padding(Padding { top: 6.0, right: 8.0, bottom: 6.0, left: 8.0 }).width(Length::Fill).style(row_style(*p, Color::TRANSPARENT, p.r(10.0), r.lit)))
}

/// A big heading inside a page (the PC's name).
pub fn big_title<'a, M: 'a>(p: &Pal, s: impl Into<String>) -> E<'a, M> {
    tb(s.into(), sz(30.0), p.text).into()
}

/// The controller look's slider: the accent rail and a round handle (settings-B1/B2).
fn big_slider_style(p: Pal) -> impl Fn(&Theme, slider::Status) -> slider::Style {
    move |_, _| slider::Style {
        rail: slider::Rail { backgrounds: (Background::Color(p.accent), Background::Color(Pal::mix(p.panel2, p.text, 0.12))), width: 6.0, border: Border { color: Color::TRANSPARENT, width: 0.0, radius: 3.0.into() } },
        handle: slider::Handle { shape: slider::HandleShape::Circle { radius: 10.0 }, background: Background::Color(p.text), border_width: 0.0, border_color: Color::TRANSPARENT },
    }
}

fn percent(v: f32, lo: f32, hi: f32) -> Option<String> {
    if (lo <= 1.0 && hi == 100.0) || (lo == 0.0 && hi == 1.0) {
        Some(format!("{}%", if hi == 1.0 { (v * 100.0).round() } else { v.round() }))
    } else {
        None
    }
}

/// A slider row's control: the slider and its value. Left and right move it a step (5 %), as Mantel's SetSlider.
pub fn slider_u32<'a, M: Clone + Send + 'static>(v: u32, range: std::ops::RangeInclusive<u32>, on: impl Fn(u32) -> M + 'a, release: M) -> E<'a, M> {
    let (lo, hi) = (*range.start(), *range.end());
    let step = if hi.saturating_sub(lo) >= 50 { 5 } else { 1 };
    let shown = percent(v as f32, lo as f32, hi as f32).unwrap_or_else(|| v.to_string());
    let down = vec![act(on(v.saturating_sub(step).max(lo))), act(release.clone())];
    let up = vec![act(on((v + step).min(hi))), act(release.clone())];
    nav::control(Kind::Slider, Vec::new(), down, up, String::new(), shown.clone());
    let s = slider(range, v, on).on_release(release);
    if big() {
        let p = pal();
        row![s.width(Length::Fixed(300.0)).style(big_slider_style(p)), container(ts(shown, 17.0, p.accent)).width(Length::Fixed(58.0)).align_right(Length::Fixed(58.0))].spacing(16.0).align_y(iced::Center).into()
    } else {
        s.width(Length::Fixed(sz(280.0))).into()
    }
}

pub fn slider_f32<'a, M: Clone + Send + 'static>(v: f32, range: std::ops::RangeInclusive<f32>, step: f32, on: impl Fn(f32) -> M + 'a, release: M) -> E<'a, M> {
    let (lo, hi) = (*range.start(), *range.end());
    let k = step.max((hi - lo) * 0.05);
    let shown = percent(v, lo, hi).unwrap_or_else(|| format!("{v:.1}"));
    let down = vec![act(on((v - k).max(lo))), act(release.clone())];
    let up = vec![act(on((v + k).min(hi))), act(release.clone())];
    nav::control(Kind::Slider, Vec::new(), down, up, String::new(), shown.clone());
    let s = slider(range, v, on).step(step).on_release(release);
    if big() {
        let p = pal();
        row![s.width(Length::Fixed(280.0)).style(big_slider_style(p)), container(ts(shown, 17.0, p.accent)).width(Length::Fixed(58.0)).align_right(Length::Fixed(58.0))].spacing(16.0).align_y(iced::Center).into()
    } else {
        s.width(Length::Fixed(sz(260.0))).into()
    }
}

/// A list row you click (a time zone, a search hit).
pub fn pick_row<'a, M: Clone + Send + 'static>(p: &Pal, label: impl Into<String>, msg: M) -> E<'a, M> {
    let label: String = label.into();
    nav::control(Kind::Press, vec![act(msg.clone())], Vec::new(), Vec::new(), String::new(), String::new());
    let r = nav::item(&label, "", false);
    mouse_area(ring(p, r.lit, p.r(8.0), container(t(label, sz(14.0), p.text)).id(r.key).padding(Padding { top: 8.0, right: 14.0, bottom: 8.0, left: 14.0 }).width(Length::Fill).style(row_style(*p, p.panel, p.r(8.0), r.lit))))
        .on_press(msg)
        .interaction(mouse::Interaction::Pointer)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn choices_step_both_ways_and_round() {
        let items = vec![("100%".to_string(), false, Some(1)), ("125%".to_string(), true, Some(2)), ("150%".to_string(), false, Some(3))];
        let (idx, prev, next, round) = choice_acts(&items);
        assert_eq!((idx, prev, next, round), (1, Some(1), Some(3), Some(3)));
        let last = vec![("Game Mode".to_string(), false, Some(1)), ("Desktop Mode".to_string(), true, None)];
        assert_eq!(choice_acts(&last), (1, Some(1), None, Some(1)));
        assert_eq!(percent(70.0, 1.0, 100.0).as_deref(), Some("70%"));
        assert_eq!(percent(0.45, 0.0, 1.0).as_deref(), Some("45%"));
        assert_eq!(percent(3.0, 0.0, 10.0), None);
    }
}
