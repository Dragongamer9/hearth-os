//! The pieces Hearth Notes' pages are drawn with, as the approved mockups draw them (docs/mockups/hearth-notes, 2000 x
//! 1250; here at 1280 x 800, so lengths are 0.64 of theirs and text about 0.8, the Store's rule). Colours follow the
//! person's look (theme::Pal) and its accent; the theme setting (Hearth / Gold / Firehold) can pick another accent.

use hearth_ui::ui::{self, gap, t, tb, tm, ts};
use hearth_ui::Pal;
use iced::widget::{column, container, mouse_area, row, text, text_input, Column, Row, Space};
use iced::{mouse, Background, Border, Color, Element, Font, Length, Padding, Shadow, Theme};

pub type E<'a, M> = Element<'a, M>;

pub const MONO: Font = Font::with_name("DejaVu Sans Mono");
pub const SERIF: Font = Font::with_name("Noto Serif");
pub const GOOD: Color = Color { r: 0.298, g: 0.812, b: 0.478, a: 1.0 }; // #4ccf7a
pub const GOOD_TX: Color = Color { r: 0.498, g: 0.878, b: 0.639, a: 1.0 }; // #7fe0a3
pub const GOOD_LINE: Color = Color { r: 0.184, g: 0.420, b: 0.271, a: 1.0 }; // #2f6b45
pub const BAD: Color = Color { r: 1.0, g: 0.365, b: 0.365, a: 1.0 }; // #ff5d5d
pub const WARN: Color = Color { r: 0.949, g: 0.757, b: 0.306, a: 1.0 }; // #f2c14e
pub const BLUE: Color = Color { r: 0.42, g: 0.66, b: 1.0, a: 1.0 };

/// A length from the mockups (2000 px wide) at this window's size.
pub fn z(v: f32) -> f32 {
    v * 0.64
}
/// A text size from the mockups.
pub fn f(v: f32) -> f32 {
    if v <= 22.0 { (v * 0.8 * 2.0).round() / 2.0 } else { (v * 0.68).round() }
}

pub fn pad(t: f32, r: f32, b: f32, l: f32) -> Padding {
    Padding { top: t, right: r, bottom: b, left: l }
}

pub fn bx(bg: Color, r: f32, line: Option<(Color, f32)>) -> impl Fn(&Theme) -> container::Style {
    ui::boxed(bg, r, line)
}

/// The neutral colours from the look: the page, the sidebar, cards and their lines.
#[derive(Clone, Copy)]
pub struct Tone {
    pub page: Color,
    pub side: Color,
    pub rail: Color,
    pub card: Color,
    pub card2: Color,
    pub line: Color,
    pub text: Color,
    pub muted: Color,
    pub faint: Color,
    pub accent: Color,
    pub ink: Color,
}

pub fn tone(p: &Pal) -> Tone {
    Tone {
        page: p.bg,
        side: Pal::mix(p.bg, p.panel, 0.45),
        rail: Pal::mix(p.bg, p.panel, 0.25),
        card: p.panel,
        card2: p.panel2,
        line: p.line,
        text: p.text,
        muted: p.muted,
        faint: Pal::mix(p.muted, p.bg, 0.35),
        accent: p.accent,
        ink: p.ink,
    }
}

/// The accent, a little lighter (links, "Check again").
pub fn accent2(p: &Pal) -> Color {
    Pal::mix(p.accent, Color::WHITE, 0.18)
}

/// The mockups' gradient (angle in degrees, CSS's).
pub fn grad(deg: f32, a: Color, b: Color, r: f32, line: Option<(Color, f32)>) -> impl Fn(&Theme) -> container::Style {
    move |_| {
        let g = iced::gradient::Linear::new(iced::Radians((deg - 90.0).to_radians())).add_stop(0.0, a).add_stop(1.0, b);
        container::Style {
            text_color: None,
            background: Some(Background::Gradient(g.into())),
            border: match line {
                Some((c, w)) => Border { color: c, width: w, radius: r.into() },
                None => iced::border::rounded(r),
            },
            shadow: Shadow::default(),
            snap: true,
        }
    }
}

/// Makes anything clickable.
pub fn click<'a, M: Clone + 'a>(e: impl Into<E<'a, M>>, m: Option<M>) -> E<'a, M> {
    match m {
        Some(m) => mouse_area(e).on_press(m).interaction(mouse::Interaction::Pointer).into(),
        None => e.into(),
    }
}

/// The controller's (and the keyboard's) focus: a 2 px ring in the accent just outside. A margin is always kept, so
/// nothing moves when the focus does.
pub fn ring<'a, M: 'a>(p: &Pal, on: bool, r: f32, inner: impl Into<E<'a, M>>) -> E<'a, M> {
    let c = if on { p.accent } else { Color::TRANSPARENT };
    container(inner).padding(3.0).style(move |_: &Theme| container::Style { border: Border { color: c, width: 2.0, radius: (r + 3.0).into() }, ..container::Style::default() }).into()
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Chip {
    /// Filled accent ("3 changes to push", "A · Use").
    Accent,
    /// Accent text and outline ("A · Use", "approved"…).
    Outline,
    /// A tag: accent text on a dim accent ("#mockup").
    Tag,
    Ok,
    Bad,
    Warn,
    Plain,
}

pub fn chip<'a, M: 'a>(p: &Pal, label: impl Into<String>, k: Chip) -> E<'a, M> {
    let tn = tone(p);
    let (bg, line, fg, bold) = match k {
        Chip::Accent => (tn.accent, tn.accent, tn.ink, true),
        Chip::Outline => (Color::TRANSPARENT, Pal::a(tn.accent, 0.8), accent2(p), false),
        Chip::Tag => (Pal::a(tn.accent, 0.16), Color::TRANSPARENT, accent2(p), false),
        Chip::Ok => (Pal::a(GOOD, 0.08), GOOD_LINE, GOOD_TX, false),
        Chip::Bad => (Color::TRANSPARENT, Color::from_rgb8(0x7a, 0x33, 0x33), Color::from_rgb8(0xff, 0x9d, 0x9d), false),
        Chip::Warn => (Color::TRANSPARENT, Color::from_rgb8(0x8a, 0x6a, 0x1f), WARN, false),
        Chip::Plain => (Color::TRANSPARENT, Pal::mix(tn.line, tn.text, 0.08), Pal::mix(tn.text, tn.muted, 0.25), false),
    };
    let lbl: String = label.into();
    let tx = if bold { tb(lbl, f(14.0), fg) } else { t(lbl, f(14.0), fg) };
    container(tx.wrapping(text::Wrapping::None)).padding(pad(z(4.0), z(12.0), z(4.0), z(12.0))).style(bx(bg, z(12.0), Some((line, 1.0)))).into()
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum B {
    Primary,
    Normal,
    Danger,
    /// Drawn but not to be pressed yet (Sync now while the check fails): the accent, dimmed.
    Dim,
}

/// A button as the mockups draw them, with the focus ring.
pub fn btn<'a, M: Clone + 'a>(p: &Pal, label: impl Into<String>, k: B, big: bool, focused: bool, on: Option<M>) -> E<'a, M> {
    let tn = tone(p);
    let enabled = on.is_some();
    let (bg, line, fg) = match k {
        B::Primary => (tn.accent, tn.accent, tn.ink),
        B::Dim => (Pal::mix(tn.accent, tn.page, 0.55), Pal::mix(tn.accent, tn.page, 0.55), Pal::mix(tn.ink, tn.page, 0.2)),
        B::Normal => (Color::TRANSPARENT, Pal::mix(tn.line, tn.text, 0.1), Pal::mix(tn.text, tn.muted, 0.1)),
        B::Danger => (Color::from_rgb8(0x3a, 0x1d, 0x1d), Color::from_rgb8(0x7a, 0x33, 0x33), Color::from_rgb8(0xff, 0xb3, 0xb3)),
    };
    let (fg, bg) = if enabled || k == B::Dim { (fg, bg) } else { (Pal::a(fg, 0.4), Pal::a(bg, if k == B::Primary { 0.4 } else { bg.a })) };
    let size = if big { f(19.0) } else { f(15.0) };
    let lbl: String = label.into();
    let tx = if k == B::Normal { t(lbl, size, fg) } else { tb(lbl, size, fg) };
    let (pv, ph) = if big { (z(14.0), z(30.0)) } else { (z(9.0), z(18.0)) };
    let r = if big { 8.0 } else { 6.0 };
    let c = container(tx.wrapping(text::Wrapping::None)).padding(pad(pv, ph, pv, ph)).style(bx(bg, r, Some((line, 1.0))));
    ring(p, focused, r, click(c, on))
}

/// A text link in the accent.
pub fn link<'a, M: Clone + 'a>(p: &Pal, label: impl Into<String>, size: f32, m: M) -> E<'a, M> {
    click(tm(label.into(), size, accent2(p)), Some(m))
}

/// A switch (`.tog`): the accent track with the dark knob on the right when on.
pub fn switch<'a, M: Clone + 'a>(p: &Pal, on: bool, focused: bool, m: Option<M>) -> E<'a, M> {
    let tn = tone(p);
    let k = z(20.0).max(12.0);
    let knob = container(Space::new()).width(Length::Fixed(k)).height(Length::Fixed(k)).style(bx(if on { tn.ink } else { Color::from_rgb8(0xe9, 0xeb, 0xef) }, k / 2.0, None));
    let inner: E<'a, M> = if on { row![Space::new().width(Length::Fill), knob].into() } else { row![knob, Space::new().width(Length::Fill)].into() };
    let track = container(inner)
        .padding(z(4.0))
        .width(Length::Fixed(z(54.0)))
        .height(Length::Fixed(z(28.0)))
        .style(bx(if on { tn.accent } else { Pal::mix(tn.card2, tn.text, 0.12) }, z(14.0), None));
    ring(p, focused, z(14.0), click(track, m))
}

/// Choices side by side (`.seg`): the picked one in the accent.
pub fn seg<'a, M: Clone + 'a>(p: &Pal, items: Vec<(String, bool, M)>, focused: bool) -> E<'a, M> {
    let tn = tone(p);
    let mut r = Row::new().spacing(2.0);
    for (l, on, m) in items {
        let c = container(if on { tb(l, f(14.0), tn.ink) } else { t(l, f(14.0), Pal::mix(tn.text, tn.muted, 0.3)) }.wrapping(text::Wrapping::None))
            .padding(pad(z(6.0), z(14.0), z(6.0), z(14.0)))
            .style(bx(if on { tn.accent } else { Color::TRANSPARENT }, 5.0, None));
        r = r.push(click(c, Some(m)));
    }
    ring(p, focused, 7.0, container(r).padding(z(4.0)).style(bx(Pal::mix(tn.page, tn.card, 0.4), 7.0, None)))
}

/// The small section title (`FILES`, `PROJECTS`, `NODES`): muted capitals.
pub fn caps<'a, M: 'a>(p: &Pal, s: &str) -> E<'a, M> {
    ts(s.to_uppercase(), f(13.0), tone(p).muted).wrapping(text::Wrapping::None).into()
}

/// The accent section title (`NODES`, `LINKS` in Customise).
pub fn caps_accent<'a, M: 'a>(p: &Pal, s: &str) -> E<'a, M> {
    ts(s.to_uppercase(), f(13.0), accent2(p)).wrapping(text::Wrapping::None).into()
}

/// A card.
pub fn card<'a, M: 'a>(p: &Pal, inner: impl Into<E<'a, M>>, focused: bool) -> E<'a, M> {
    let tn = tone(p);
    ring(p, focused, 10.0, container(inner).padding(pad(z(18.0), z(20.0), z(18.0), z(20.0))).width(Length::Fill).style(bx(tn.card, 10.0, None)))
}

/// A settings row (`.row`): its icon, title and line, and what's on its right.
pub fn srow<'a, M: Clone + 'a>(p: &Pal, icon: E<'a, M>, title: impl Into<String>, sub: E<'a, M>, end: E<'a, M>, focused: bool, m: Option<M>) -> E<'a, M> {
    let tn = tone(p);
    let txt = Column::new().spacing(z(4.0)).push(t(title.into(), f(19.0), tn.text)).push(sub);
    let r = row![icon, txt.width(Length::Fill), end].spacing(z(18.0)).align_y(iced::Center);
    ring(p, focused, 9.0, click(container(r).padding(pad(z(16.0), z(22.0), z(16.0), z(22.0))).width(Length::Fill).style(bx(tn.card, 9.0, None)), m))
}

/// A row's icon box: a symbol on the second card colour.
pub fn icon_box<'a, M: 'a>(p: &Pal, inner: E<'a, M>) -> E<'a, M> {
    let tn = tone(p);
    container(inner).center_x(Length::Fixed(z(42.0))).center_y(Length::Fixed(z(42.0))).style(bx(tn.card2, 7.0, None)).into()
}

pub fn sym<'a, M: 'a>(p: &Pal, s: &'a str) -> E<'a, M> {
    icon_box(p, t(s, f(18.0), Pal::mix(tone(p).text, tone(p).muted, 0.2)).into())
}

// ------------------------------------------------------------------ hints and the window
/// A controller button in a circle and what it does.
pub fn hint<'a, M: 'a>(p: &Pal, b: &'a str, what: impl Into<String>) -> E<'a, M> {
    let tn = tone(p);
    let w = if b.chars().count() > 1 { z(32.0) } else { z(28.0) };
    let c = container(tb(b, f(12.0), Pal::mix(tn.text, tn.muted, 0.2)).wrapping(text::Wrapping::None)).center_x(Length::Fixed(w)).center_y(Length::Fixed(z(28.0))).style(bx(Color::TRANSPARENT, z(14.0), Some((tn.muted, 1.5))));
    row![c, gap(z(8.0), 1.0), t(what.into(), f(17.0), Pal::mix(tn.text, tn.muted, 0.2)).wrapping(text::Wrapping::None)].align_y(iced::Center).into()
}

pub fn hint2<'a, M: 'a>(p: &Pal, a: &'a str, b: &'a str, what: impl Into<String>) -> E<'a, M> {
    let tn = tone(p);
    let c = |s: &'a str| -> E<'a, M> {
        container(tb(s, f(11.0), Pal::mix(tn.text, tn.muted, 0.2)).wrapping(text::Wrapping::None)).center_x(Length::Fixed(z(32.0))).center_y(Length::Fixed(z(28.0))).style(bx(Color::TRANSPARENT, z(14.0), Some((tn.muted, 1.5)))).into()
    };
    row![c(a), gap(z(6.0), 1.0), c(b), gap(z(8.0), 1.0), t(what.into(), f(17.0), Pal::mix(tn.text, tn.muted, 0.2)).wrapping(text::Wrapping::None)].align_y(iced::Center).into()
}

/// A plain words hint on the right ("Ctrl+O Quick switcher").
pub fn hint_words<'a, M: 'a>(p: &Pal, s: impl Into<String>) -> E<'a, M> {
    t(s.into(), f(17.0), Pal::mix(tone(p).text, tone(p).muted, 0.2)).wrapping(text::Wrapping::None).into()
}

/// The bottom bar: buttons on the left, others on the right.
pub fn hints<'a, M: 'a>(p: &Pal, left: Vec<E<'a, M>>, right: Vec<E<'a, M>>) -> E<'a, M> {
    let tn = tone(p);
    let mut l = Row::new().spacing(z(30.0)).align_y(iced::Center);
    for x in left {
        l = l.push(x);
    }
    let mut r = Row::new().spacing(z(30.0)).align_y(iced::Center);
    for x in right {
        r = r.push(x);
    }
    column![
        ui::hline(tn.line),
        container(row![l, Space::new().width(Length::Fill), r].align_y(iced::Center)).padding(pad(0.0, z(30.0), 0.0, z(30.0))).center_y(Length::Fixed(z(56.0))).width(Length::Fill).style(bx(tn.page, 0.0, None))
    ]
    .into()
}

/// Hearth Notes' mark: the accent square with its N.
pub fn mark<'a, M: 'a>(p: &Pal, size: f32) -> E<'a, M> {
    let a = p.accent;
    container(text("N").size(size * 0.5).color(p.ink).font(Font { weight: iced::font::Weight::ExtraBold, ..hearth_ui::app::INTER }))
        .center_x(Length::Fixed(size))
        .center_y(Length::Fixed(size))
        .style(grad(135.0, Pal::mix(a, Color::WHITE, 0.08), Pal::mix(a, Color::BLACK, 0.18), (size * 0.22).round(), None))
        .into()
}

pub fn dot<'a, M: 'a>(c: Color, size: f32) -> E<'a, M> {
    container(Space::new()).width(Length::Fixed(size)).height(Length::Fixed(size)).style(bx(c, size / 2.0, None)).into()
}

/// A bar filled to `frac` (Productivity).
pub fn progress<'a, M: 'a>(p: &Pal, frac: f32, h: f32, width: Length) -> E<'a, M> {
    let tn = tone(p);
    let fr = (frac.clamp(0.0, 1.0) * 1000.0).round() as u16;
    let mut r = Row::new();
    if fr > 0 {
        r = r.push(container(Space::new()).width(Length::FillPortion(fr)).height(Length::Fixed(h)).style(grad(90.0, tn.accent, accent2(p), h / 2.0, None)));
    }
    if fr < 1000 {
        r = r.push(Space::new().width(Length::FillPortion(1000 - fr)).height(Length::Fixed(h)));
    }
    container(r).width(width).height(Length::Fixed(h)).clip(true).style(bx(Pal::mix(tn.card2, tn.text, 0.08), h / 2.0, None)).into()
}

pub fn mono<'a, M: 'a>(s: impl Into<String>, size: f32, c: Color) -> E<'a, M> {
    text(s.into()).size(size).color(c).font(MONO).wrapping(text::Wrapping::None).into()
}

/// A text field in the mockups' look: dark inside, a line around, the accent ring when it's the one typed in.
pub fn field_style(p: &Pal, focus_ring: bool) -> impl Fn(&Theme, text_input::Status) -> text_input::Style {
    let p = *p;
    move |_t, st| {
        let focused = matches!(st, text_input::Status::Focused { .. });
        text_input::Style {
            background: Background::Color(Pal::mix(p.bg, p.panel, 0.35)),
            border: Border { color: if focus_ring && focused { p.accent } else { p.line }, width: if focus_ring && focused { 2.0 } else { 1.0 }, radius: 7.0.into() },
            icon: p.muted,
            placeholder: p.muted,
            value: p.text,
            selection: Pal::a(p.accent, 0.35),
        }
    }
}

/// A field without its own box (the live editor's line, a field inside a drawn box).
pub fn bare_style(p: &Pal) -> impl Fn(&Theme, text_input::Status) -> text_input::Style {
    let p = *p;
    move |_t, _st| text_input::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border { color: Color::TRANSPARENT, width: 0.0, radius: 0.0.into() },
        icon: p.muted,
        placeholder: p.muted,
        value: p.text,
        selection: Pal::a(p.accent, 0.35),
    }
}

pub fn slider_style(p: &Pal) -> impl Fn(&Theme, iced::widget::slider::Status) -> iced::widget::slider::Style {
    use iced::widget::slider;
    let p = *p;
    move |_, _| slider::Style {
        rail: slider::Rail { backgrounds: (Background::Color(p.accent), Background::Color(Pal::mix(p.panel2, p.text, 0.12))), width: 4.0, border: Border { color: Color::TRANSPARENT, width: 0.0, radius: 2.0.into() } },
        handle: slider::Handle { shape: slider::HandleShape::Circle { radius: 7.0 }, background: Background::Color(Color::from_rgb8(0xf1, 0xf1, 0xf1)), border_width: 0.0, border_color: Color::TRANSPARENT },
    }
}

/// A project's colour (the mockups' legend): hearth the accent, the others their own.
pub fn project_color(p: &Pal, project: &str) -> Color {
    match project {
        "hearth" => p.accent,
        "beacon" => Color::from_rgb8(0x2f, 0xbf, 0x9b),
        "quad-mech" => Color::from_rgb8(0xc9, 0xa2, 0x4a),
        "gravesleep" => Color::from_rgb8(0x8b, 0xc3, 0x4a),
        "tributary" => Color::from_rgb8(0x3f, 0xb6, 0xd8),
        "hearth-ai" => Color::from_rgb8(0xa7, 0x8b, 0xfa),
        "command-center" => Color::from_rgb8(0x4a, 0x90, 0xff),
        "other" => Color::from_rgb8(0xf4, 0x72, 0xb6),
        "log" => Color::from_rgb8(0xf2, 0xee, 0xe8),
        _ => Color::from_rgb8(0x9a, 0x9a, 0xa6),
    }
}

pub fn divider<'a, M: 'a>(p: &Pal) -> E<'a, M> {
    ui::hline(tone(p).line)
}

pub fn label_value<'a, M: 'a>(p: &Pal, k: impl Into<String>, v: impl Into<String>) -> E<'a, M> {
    let tn = tone(p);
    row![t(k.into(), f(15.0), tn.muted), Space::new().width(Length::Fill), t(v.into(), f(15.0), tn.text)].align_y(iced::Center).into()
}
