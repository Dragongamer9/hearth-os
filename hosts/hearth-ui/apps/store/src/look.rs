//! The pieces the Store's pages are drawn with, as the approved mockups draw them (docs/mockups/hearth-store, 2000 x
//! 1250; here at 1280 x 800, so lengths are 0.64 of theirs and text about 0.8): chips, buttons, the apps' icons, the
//! hero's picture, switches, rows, the controller's hints. The mockups' orange and their status colours are fixed (as
//! Logitech G15's app does); the rest follows the person's look (theme::Pal).

use hearth_store::catalog::Entry;
use hearth_ui::ui::{self, gap, t, t1, tb, tm, ts};
use hearth_ui::Pal;
use iced::widget::canvas::{Frame, Geometry, LineCap, Path, Stroke};
use iced::widget::{canvas, column, container, mouse_area, row, text, Column, Row, Space};
use iced::{mouse, Background, Border, Color, Element, Font, Length, Padding, Point, Rectangle, Renderer, Shadow, Theme};

pub const ORANGE: Color = Color { r: 1.0, g: 0.478, b: 0.184, a: 1.0 }; // #ff7a2f
pub const ORANGE2: Color = Color { r: 1.0, g: 0.604, b: 0.322, a: 1.0 }; // #ff9a52
pub const INK: Color = Color { r: 0.106, g: 0.063, b: 0.024, a: 1.0 }; // #1b1006
pub const OK_TX: Color = Color { r: 0.498, g: 0.878, b: 0.639, a: 1.0 }; // #7fe0a3
pub const OK_LINE: Color = Color { r: 0.184, g: 0.420, b: 0.271, a: 1.0 }; // #2f6b45
pub const OK_DOT: Color = Color { r: 0.298, g: 0.812, b: 0.478, a: 1.0 }; // #4ccf7a
pub const WARN: Color = Color { r: 0.949, g: 0.757, b: 0.306, a: 1.0 }; // #f2c14e
pub const BAD: Color = Color { r: 1.0, g: 0.365, b: 0.365, a: 1.0 }; // #ff5d5d
pub const PURPLE: Color = Color { r: 0.788, g: 0.702, b: 1.0, a: 1.0 }; // #c9b3ff

pub type E<'a, M> = Element<'a, M>;

/// A length from the mockups (2000 px wide) at this window's size.
pub fn z(v: f32) -> f32 {
    v * 0.64
}
/// A text size from the mockups.
pub fn f(v: f32) -> f32 {
    if v <= 22.0 { (v * 0.8 * 2.0).round() / 2.0 } else { (v * 0.68).round() }
}

pub fn hex(s: &str, d: Color) -> Color {
    hearth_ui::theme::hex(s).unwrap_or(d)
}

/// The Store's neutral colours from the look: the page, the sidebar, cards and their lines.
#[derive(Clone, Copy)]
pub struct Tone {
    pub page: Color,
    pub side: Color,
    pub card: Color,
    pub card2: Color,
    pub line: Color,
    pub text: Color,
    pub muted: Color,
}

pub fn tone(p: &Pal) -> Tone {
    Tone {
        page: p.bg,
        side: Pal::mix(p.bg, p.panel, 0.45),
        card: p.panel,
        card2: p.panel2,
        line: p.line,
        text: p.text,
        muted: p.muted,
    }
}

pub fn pad(t: f32, r: f32, b: f32, l: f32) -> Padding {
    Padding { top: t, right: r, bottom: b, left: l }
}

pub fn bx(bg: Color, r: f32, line: Option<(Color, f32)>) -> impl Fn(&Theme) -> container::Style {
    ui::boxed(bg, r, line)
}

/// The mockups' gradient (angle in degrees, CSS's: 90 is left to right, 135 top left to bottom right).
pub fn grad(deg: f32, a: Color, b: Color, r: f32, line: Option<(Color, f32)>) -> impl Fn(&Theme) -> container::Style {
    move |_| {
        // CSS's angle points "to" a side, clockwise from up; iced's starts at the right (y grows downward).
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

/// The controller's (and the keyboard's) focus: a 3 px orange ring just outside, as the mockups' `.focus`. A margin is
/// always kept, so nothing moves when the focus does.
pub fn ring<'a, M: 'a>(on: bool, r: f32, inner: impl Into<E<'a, M>>) -> E<'a, M> {
    container(inner)
        .padding(3.0)
        .style(move |_: &Theme| container::Style { border: Border { color: if on { ORANGE } else { Color::TRANSPARENT }, width: 2.0, radius: (r + 3.0).into() }, ..container::Style::default() })
        .into()
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Chip {
    /// Up to date, Installed (green outline).
    Ok,
    /// Update ready, Update (orange, filled).
    Up,
    /// Built in (grey, filled).
    Built,
    /// Coming (grey outline).
    Coming,
    /// Updates itself, Stable, Checked… (outline).
    Plain,
    /// TEST (yellow).
    Test,
    /// What's wrong: Changed (red), Not allowed to run / Missing (yellow).
    Bad,
    Warn,
}

pub fn chip<'a, M: 'a>(p: &Pal, label: impl Into<String>, k: Chip) -> E<'a, M> {
    let tn = tone(p);
    let (bg, line, fg, bold) = match k {
        Chip::Ok => (Color::TRANSPARENT, OK_LINE, OK_TX, false),
        Chip::Up => (ORANGE, ORANGE, INK, true),
        Chip::Built => (Pal::mix(tn.card, tn.text, 0.04), Pal::mix(tn.line, tn.text, 0.12), Pal::mix(tn.text, tn.muted, 0.35), false),
        Chip::Coming => (Color::TRANSPARENT, tn.line, tn.muted, false),
        Chip::Plain => (Color::TRANSPARENT, Pal::mix(tn.line, tn.text, 0.08), Pal::mix(tn.text, tn.muted, 0.25), false),
        Chip::Test => (Color::TRANSPARENT, Color::from_rgb8(0x8a, 0x6a, 0x1f), WARN, false),
        Chip::Bad => (Color::TRANSPARENT, Color::from_rgb8(0x7a, 0x33, 0x33), Color::from_rgb8(0xff, 0x9d, 0x9d), false),
        Chip::Warn => (Color::TRANSPARENT, Color::from_rgb8(0x8a, 0x6a, 0x1f), WARN, false),
    };
    let lbl: String = label.into();
    let tx = if bold { tb(lbl, f(14.0), fg) } else { t(lbl, f(14.0), fg) };
    container(tx.wrapping(text::Wrapping::None)).padding(pad(z(5.0), z(13.0), z(5.0), z(13.0))).style(bx(bg, 13.0, Some((line, 1.0)))).into()
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum B {
    Primary,
    Normal,
    Danger,
}

/// A button as the mockups draw them (`.btn`, `.btn.p`, `.btn.d`; `big` is `.btn.big`), with the focus ring.
pub fn btn<'a, M: Clone + 'a>(p: &Pal, label: impl Into<String>, k: B, big: bool, focused: bool, on: Option<M>) -> E<'a, M> {
    let tn = tone(p);
    let enabled = on.is_some();
    let (bg, line, fg) = match k {
        B::Primary => (ORANGE, ORANGE, INK),
        B::Normal => (Color::TRANSPARENT, Pal::mix(tn.line, tn.text, 0.1), Pal::mix(tn.text, tn.muted, 0.1)),
        B::Danger => (Color::from_rgb8(0x3a, 0x1d, 0x1d), Color::from_rgb8(0x7a, 0x33, 0x33), Color::from_rgb8(0xff, 0xb3, 0xb3)),
    };
    let (fg, bg) = if enabled { (fg, bg) } else { (Pal::a(fg, 0.4), Pal::a(bg, if k == B::Primary { 0.4 } else { bg.a })) };
    let size = if big { f(19.0) } else { f(16.0) };
    let lbl: String = label.into();
    let tx = if k == B::Normal { t(lbl, size, fg) } else { tb(lbl, size, fg) };
    let (pv, ph) = if big { (z(14.0), z(30.0)) } else { (z(10.0), z(20.0)) };
    let r = if big { 8.0 } else { 6.0 };
    let c = container(tx.wrapping(text::Wrapping::None)).padding(pad(pv, ph, pv, ph)).style(bx(bg, r, Some((if enabled { line } else { Pal::a(line, 0.4) }, 1.0))));
    ring(focused, r, click(c, on))
}

/// "See all ›", "Settings › Updates & recovery ›": a text link in the accent.
pub fn link<'a, M: Clone + 'a>(label: impl Into<String>, size: f32, m: M) -> E<'a, M> {
    click(tm(label.into(), size, ORANGE2), Some(m))
}

/// A switch as the mockups draw it (`.tog`): the orange track with the dark knob on the right when on.
pub fn switch<'a, M: Clone + 'a>(p: &Pal, on: bool, focused: bool, m: Option<M>) -> E<'a, M> {
    let tn = tone(p);
    let k = z(20.0).max(13.0);
    let knob = container(Space::new()).width(Length::Fixed(k)).height(Length::Fixed(k)).style(bx(if on { INK } else { Color::from_rgb8(0xe9, 0xeb, 0xef) }, k / 2.0, None));
    let inner: E<'a, M> = if on { row![Space::new().width(Length::Fill), knob].into() } else { row![knob, Space::new().width(Length::Fill)].into() };
    let track = container(inner)
        .padding(z(5.0))
        .width(Length::Fixed(z(56.0)))
        .height(Length::Fixed(z(30.0)))
        .style(bx(if on { ORANGE } else { Pal::mix(tn.card2, tn.text, 0.12) }, z(15.0), None));
    ring(focused, z(15.0), click(track, m))
}

/// Two or more choices side by side (`.seg`): the picked one orange.
pub fn seg<'a, M: Clone + 'a>(p: &Pal, items: Vec<(String, bool, M)>, focused: bool) -> E<'a, M> {
    let tn = tone(p);
    let mut r = Row::new().spacing(2.0);
    for (l, on, m) in items {
        let c = container(if on { tb(l, f(15.0), INK) } else { t(l, f(15.0), Pal::mix(tn.text, tn.muted, 0.3)) }.wrapping(text::Wrapping::None))
            .padding(pad(z(8.0), z(16.0), z(8.0), z(16.0)))
            .style(bx(if on { ORANGE } else { Color::TRANSPARENT }, 5.0, None));
        r = r.push(click(c, Some(m)));
    }
    ring(focused, 7.0, container(r).padding(z(4.0)).style(bx(Pal::mix(tn.page, tn.card, 0.4), 7.0, None)))
}

/// A row's icon box (`.ri`): a symbol on the second card colour.
pub fn icon_box<'a, M: 'a>(p: &Pal, sym: &'a str) -> E<'a, M> {
    let tn = tone(p);
    container(t(sym, f(18.0), Pal::mix(tn.text, tn.muted, 0.2))).center_x(Length::Fixed(z(42.0))).center_y(Length::Fixed(z(42.0))).style(bx(tn.card2, 7.0, None)).into()
}

/// The section title (`.sec`): muted, semibold, with something on the right (See all ›).
pub fn sec<'a, M: 'a>(p: &Pal, s: impl Into<String>, right: Option<E<'a, M>>) -> E<'a, M> {
    let mut r = row![ts(s.into(), f(17.0), tone(p).muted)].align_y(iced::Center);
    if let Some(x) = right {
        r = r.push(Space::new().width(Length::Fill)).push(x);
    }
    container(r).padding(pad(z(22.0), 0.0, z(12.0), 0.0)).width(Length::Fill).into()
}

/// A card (`.card`).
pub fn card<'a, M: 'a>(p: &Pal, inner: impl Into<E<'a, M>>, focused: bool) -> E<'a, M> {
    let tn = tone(p);
    ring(focused, 10.0, container(inner).padding(pad(z(22.0), z(24.0), z(22.0), z(24.0))).width(Length::Fill).style(bx(tn.card, 10.0, None)))
}

/// A row (`.row`): its icon, title and line, and what's on its right.
pub fn srow<'a, M: Clone + 'a>(p: &Pal, icon: E<'a, M>, title: impl Into<String>, sub: impl Into<String>, end: E<'a, M>, focused: bool, m: Option<M>) -> E<'a, M> {
    let tn = tone(p);
    let sub: String = sub.into();
    let mut txt = Column::new().spacing(z(4.0)).push(t(title.into(), f(19.0), tn.text));
    if !sub.is_empty() {
        txt = txt.push(t(sub, f(15.0), tn.muted));
    }
    let r = row![icon, txt.width(Length::Fill), end].spacing(z(18.0)).align_y(iced::Center);
    ring(focused, 9.0, click(container(r).padding(pad(z(18.0), z(22.0), z(18.0), z(22.0))).width(Length::Fill).style(bx(tn.card, 9.0, None)), m))
}

/// One line of a facts table (`.kv`): the name muted on the left, the value on the right (and a second, smaller line).
pub fn kv<'a, M: 'a>(p: &Pal, k: impl Into<String>, v: impl Into<String>, sub: Option<String>, last: bool) -> E<'a, M> {
    let tn = tone(p);
    let mut right = Column::new().align_x(iced::Right).push(t(v.into(), f(16.0), tn.text));
    if let Some(s) = sub {
        right = right.push(t(s, f(13.0), tn.muted));
    }
    let r = row![t1(k.into(), f(16.0), tn.muted), Space::new().width(Length::Fill), right].align_y(iced::Center).spacing(z(16.0));
    let mut c = Column::new().push(container(r).padding(pad(z(10.0), 0.0, z(10.0), 0.0)));
    if !last {
        c = c.push(ui::hline(tn.line));
    }
    c.into()
}

/// What's new as the mockups' list (`ul.new`): an orange dot before each line.
pub fn bullets<'a, M: 'a>(p: &Pal, lines: &[String], size: f32) -> E<'a, M> {
    let tn = tone(p);
    let mut c = Column::new().spacing(z(8.0));
    for l in lines {
        let dot = container(Space::new()).width(Length::Fixed(z(7.0).max(4.5))).height(Length::Fixed(z(7.0).max(4.5))).style(bx(ORANGE, 4.0, None));
        c = c.push(row![container(dot).padding(pad(size * 0.45, 0.0, 0.0, z(4.0))), gap(z(11.0), 1.0), t(l.clone(), size, Pal::mix(tn.text, tn.muted, 0.12))].align_y(iced::Top));
    }
    c.into()
}

// ------------------------------------------------------------------ the apps' icons
/// An app's icon (`.app`): its two colours, rounded (22 %), with its letters or its picture (☾ moon, ☺ buddy, ⬡ CAD).
pub fn app_icon<'a, M: 'a>(e: &Entry, size: f32) -> E<'a, M> {
    let from = hex(&e.from, ORANGE);
    let to = hex(&e.to, Color::from_rgb8(0xd9, 0x53, 0x1e));
    let r = (size * 0.22).round();
    let inner: E<'a, M> = match e.icon.as_str() {
        "moon" | "smile" | "hex" => canvas(Art { kind: e.icon.clone() }).width(Length::Fixed(size * 0.5)).height(Length::Fixed(size * 0.5)).into(),
        letters => {
            let n = letters.chars().count().max(1) as f32;
            let s = if n <= 1.0 { size * 0.42 } else if n <= 2.0 { size * 0.36 } else { size * 0.28 };
            text(letters.to_string()).size(s).color(Color::WHITE).font(Font { weight: iced::font::Weight::ExtraBold, ..hearth_ui::app::INTER }).wrapping(text::Wrapping::None).into()
        }
    };
    container(inner).center_x(Length::Fixed(size)).center_y(Length::Fixed(size)).style(grad(135.0, from, to, r, None)).into()
}

/// Small pictures drawn in place of symbols a font may not have: ☾ ☺ ⬡ (apps), ⚠ (offline), the bell, the bin.
pub struct Art {
    pub kind: String,
}

impl<M> canvas::Program<M> for Art {
    type State = canvas::Cache;
    fn draw(&self, cache: &canvas::Cache, renderer: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        vec![cache.draw(renderer, b.size(), |fr: &mut Frame| {
            let w = b.width.min(b.height);
            let c = Point::new(b.width / 2.0, b.height / 2.0);
            let st = Stroke::default().with_color(Color::WHITE).with_width((w * 0.09).max(1.5)).with_line_cap(LineCap::Round);
            match self.kind.as_str() {
                "moon" => {
                    // A crescent: a circle with a second one taken out of it, as the mockups' ☾.
                    let outer: Vec<Point> = (0..=40).map(|i| { let a = std::f32::consts::PI * (0.62 + 1.5 * i as f32 / 40.0); Point::new(c.x + w * 0.42 * a.cos(), c.y + w * 0.42 * a.sin()) }).collect();
                    let inner: Vec<Point> = (0..=40).rev().map(|i| { let a = std::f32::consts::PI * (0.75 + 1.25 * i as f32 / 40.0); Point::new(c.x + w * 0.16 + w * 0.34 * a.cos(), c.y - w * 0.04 + w * 0.34 * a.sin()) }).collect();
                    let path = Path::new(|p| {
                        p.move_to(outer[0]);
                        for q in &outer[1..] {
                            p.line_to(*q);
                        }
                        for q in &inner {
                            p.line_to(*q);
                        }
                        p.close();
                    });
                    fr.fill(&path, Color::WHITE);
                }
                "smile" => {
                    fr.stroke(&Path::circle(c, w * 0.4), st);
                    fr.fill(&Path::circle(Point::new(c.x - w * 0.14, c.y - w * 0.1), w * 0.05), Color::WHITE);
                    fr.fill(&Path::circle(Point::new(c.x + w * 0.14, c.y - w * 0.1), w * 0.05), Color::WHITE);
                    let m = Path::new(|p| {
                        for i in 0..=12 {
                            let a = std::f32::consts::PI * (0.2 + 0.6 * i as f32 / 12.0);
                            let q = Point::new(c.x + w * 0.2 * a.cos(), c.y + w * 0.02 + w * 0.2 * a.sin());
                            if i == 0 { p.move_to(q) } else { p.line_to(q) }
                        }
                    });
                    fr.stroke(&m, st);
                }
                "warn" => {
                    let tri = Path::new(|p| {
                        p.move_to(Point::new(c.x, c.y - w * 0.44));
                        p.line_to(Point::new(c.x + w * 0.48, c.y + w * 0.4));
                        p.line_to(Point::new(c.x - w * 0.48, c.y + w * 0.4));
                        p.close();
                    });
                    fr.fill(&tri, Color::from_rgb8(0xf5, 0xdc, 0x9a));
                    fr.stroke(&Path::line(Point::new(c.x, c.y - w * 0.14), Point::new(c.x, c.y + w * 0.12)), Stroke::default().with_color(Color::from_rgb8(0x2a, 0x25, 0x16)).with_width((w * 0.1).max(1.5)).with_line_cap(LineCap::Round));
                    fr.fill(&Path::circle(Point::new(c.x, c.y + w * 0.27), (w * 0.06).max(1.0)), Color::from_rgb8(0x2a, 0x25, 0x16));
                }
                "bell" => {
                    let gold = Color::from_rgb8(0xf5, 0xb7, 0x2a);
                    let body = Path::new(|p| {
                        p.move_to(Point::new(c.x - w * 0.36, c.y + w * 0.24));
                        p.line_to(Point::new(c.x - w * 0.28, c.y + w * 0.12));
                        p.line_to(Point::new(c.x - w * 0.28, c.y - w * 0.08));
                        for i in 0..=12 {
                            let a = std::f32::consts::PI * (1.0 + i as f32 / 12.0);
                            p.line_to(Point::new(c.x + w * 0.28 * a.cos(), c.y - w * 0.08 + w * 0.3 * a.sin()));
                        }
                        p.line_to(Point::new(c.x + w * 0.28, c.y + w * 0.12));
                        p.line_to(Point::new(c.x + w * 0.36, c.y + w * 0.24));
                        p.close();
                    });
                    fr.fill(&body, gold);
                    fr.fill(&Path::circle(Point::new(c.x, c.y + w * 0.34), w * 0.09), gold);
                }
                "trash" => {
                    let red = Color::from_rgb8(0xff, 0x9d, 0x9d);
                    let st = Stroke::default().with_color(red).with_width((w * 0.08).max(1.2)).with_line_cap(LineCap::Round);
                    fr.stroke(&Path::line(Point::new(c.x - w * 0.36, c.y - w * 0.26), Point::new(c.x + w * 0.36, c.y - w * 0.26)), st);
                    fr.stroke(&Path::line(Point::new(c.x - w * 0.1, c.y - w * 0.38), Point::new(c.x + w * 0.1, c.y - w * 0.38)), st);
                    let can = Path::new(|p| {
                        p.move_to(Point::new(c.x - w * 0.28, c.y - w * 0.18));
                        p.line_to(Point::new(c.x - w * 0.22, c.y + w * 0.4));
                        p.line_to(Point::new(c.x + w * 0.22, c.y + w * 0.4));
                        p.line_to(Point::new(c.x + w * 0.28, c.y - w * 0.18));
                    });
                    fr.stroke(&can, st);
                }
                _ => {
                    let h = Path::new(|p| {
                        for i in 0..=6 {
                            let a = std::f32::consts::PI / 3.0 * i as f32 - std::f32::consts::FRAC_PI_2;
                            let q = Point::new(c.x + w * 0.4 * a.cos(), c.y + w * 0.4 * a.sin());
                            if i == 0 { p.move_to(q) } else { p.line_to(q) }
                        }
                    });
                    fr.stroke(&h, st);
                }
            }
        })]
    }
}

/// A dashed outline over `inner` (the mockups' dashed cards and chips: "Looking for other apps?", "Nothing installed
/// from the Store yet", Coming).
pub fn dashed<'a, M: 'a>(inner: impl Into<E<'a, M>>, r: f32, c: Color) -> E<'a, M> {
    iced::widget::stack![inner.into(), canvas(Dash { r, c }).width(Length::Fill).height(Length::Fill)].into()
}

struct Dash {
    r: f32,
    c: Color,
}

impl<M> canvas::Program<M> for Dash {
    type State = ();
    fn draw(&self, _s: &(), renderer: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        if self.c.a == 0.0 {
            return vec![];
        }
        let mut fr = Frame::new(renderer, b.size());
        let (w, h, r) = (b.width - 1.0, b.height - 1.0, self.r.min(b.width / 2.0).min(b.height / 2.0));
        let path = Path::new(|p| {
            p.move_to(Point::new(0.5 + r, 0.5));
            p.line_to(Point::new(w + 0.5 - r, 0.5));
            p.arc_to(Point::new(w + 0.5, 0.5), Point::new(w + 0.5, 0.5 + r), r);
            p.line_to(Point::new(w + 0.5, h + 0.5 - r));
            p.arc_to(Point::new(w + 0.5, h + 0.5), Point::new(w + 0.5 - r, h + 0.5), r);
            p.line_to(Point::new(0.5 + r, h + 0.5));
            p.arc_to(Point::new(0.5, h + 0.5), Point::new(0.5, h + 0.5 - r), r);
            p.line_to(Point::new(0.5, 0.5 + r));
            p.arc_to(Point::new(0.5, 0.5), Point::new(0.5 + r, 0.5), r);
        });
        let mut st = Stroke::default().with_color(self.c).with_width(1.0);
        st.line_dash = canvas::LineDash { segments: &[5.0, 4.0], offset: 0 };
        fr.stroke(&path, st);
        vec![fr.into_geometry()]
    }
}

/// The home page's picture for a featured app: points of light joined by faint lines in an oval (Hearth AI's brain in
/// the mockup), drawn once and kept.
pub struct Brain;

impl<M> canvas::Program<M> for Brain {
    type State = canvas::Cache;
    fn draw(&self, cache: &canvas::Cache, renderer: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        vec![cache.draw(renderer, b.size(), |fr: &mut Frame| {
            let mut seed: u32 = 0x9e37_79b9;
            let mut rnd = || {
                seed ^= seed << 13;
                seed ^= seed >> 17;
                seed ^= seed << 5;
                (seed % 10_000) as f32 / 10_000.0
            };
            let (cx, cy) = (b.width / 2.0, b.height / 2.0);
            let (rx, ry) = (b.width * 0.38, b.height * 0.40);
            let mut pts = Vec::new();
            while pts.len() < 300 {
                let (x, y) = (rnd() * 2.0 - 1.0, rnd() * 2.0 - 1.0);
                if x * x + y * y <= 1.0 {
                    pts.push(Point::new(cx + x * rx, cy + y * ry));
                }
            }
            let line = Stroke::default().with_color(Color::from_rgba8(0x8b, 0x6c, 0xf0, 0.22)).with_width(0.7);
            for i in 0..pts.len() {
                for j in 1..3 {
                    let k = (i * 7 + j * 13) % pts.len();
                    let (a, c) = (pts[i], pts[k]);
                    if (a.x - c.x).abs() < rx * 0.35 && (a.y - c.y).abs() < ry * 0.35 {
                        fr.stroke(&Path::line(a, c), line);
                    }
                }
            }
            for (i, q) in pts.iter().enumerate() {
                let col = if i % 6 == 0 { Color::from_rgba8(0xff, 0x9a, 0x52, 0.95) } else { Color::from_rgba8(0xa7, 0x8b, 0xfa, 0.85) };
                fr.fill(&Path::circle(*q, if i % 9 == 0 { 2.2 } else { 1.4 }), col);
            }
        })]
    }
}

// ------------------------------------------------------------------ hints and the window
/// A controller button in a circle (`.hints b`) and what it does.
pub fn hint<'a, M: 'a>(p: &Pal, b: &'a str, what: impl Into<String>) -> E<'a, M> {
    let tn = tone(p);
    let w = if b.chars().count() > 1 { z(32.0) } else { z(28.0) };
    let c = container(tb(b, f(12.0), Pal::mix(tn.text, tn.muted, 0.2)).wrapping(text::Wrapping::None)).center_x(Length::Fixed(w)).center_y(Length::Fixed(z(28.0))).style(bx(Color::TRANSPARENT, z(14.0), Some((tn.muted, 1.5))));
    row![c, gap(z(8.0), 1.0), t(what.into(), f(17.0), Pal::mix(tn.text, tn.muted, 0.2)).wrapping(text::Wrapping::None)].align_y(iced::Center).into()
}

/// Two buttons in circles and one meaning (LB RB Previous / next page).
pub fn hint2<'a, M: 'a>(p: &Pal, a: &'a str, b: &'a str, what: impl Into<String>) -> E<'a, M> {
    let tn = tone(p);
    let c = |s: &'a str| -> E<'a, M> {
        container(tb(s, f(11.0), Pal::mix(tn.text, tn.muted, 0.2)).wrapping(text::Wrapping::None)).center_x(Length::Fixed(z(32.0))).center_y(Length::Fixed(z(28.0))).style(bx(Color::TRANSPARENT, z(14.0), Some((tn.muted, 1.5)))).into()
    };
    row![c(a), gap(z(6.0), 1.0), c(b), gap(z(8.0), 1.0), t(what.into(), f(17.0), Pal::mix(tn.text, tn.muted, 0.2)).wrapping(text::Wrapping::None)].align_y(iced::Center).into()
}

/// The bottom bar (`.hints`): buttons on the left, others on the right.
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

/// The Store's mark: the orange square with its S (title bar, controller look).
pub fn mark<'a, M: 'a>(size: f32) -> E<'a, M> {
    container(text("S").size(size * 0.5).color(INK).font(Font { weight: iced::font::Weight::ExtraBold, ..hearth_ui::app::INTER }))
        .center_x(Length::Fixed(size))
        .center_y(Length::Fixed(size))
        .style(grad(135.0, Color::from_rgb8(0xff, 0x8a, 0x3d), Color::from_rgb8(0xd9, 0x53, 0x1e), (size * 0.27).round(), None))
        .into()
}

/// A dot (offline banner's list time, What's wrong's lines).
pub fn dot<'a, M: 'a>(c: Color, size: f32) -> E<'a, M> {
    container(Space::new()).width(Length::Fixed(size)).height(Length::Fixed(size)).style(bx(c, size / 2.0, None)).into()
}

/// A progress bar (`.bar`): orange to light orange.
pub fn progress<'a, M: 'a>(p: &Pal, frac: f32, h: f32, width: Length) -> E<'a, M> {
    let tn = tone(p);
    let fr = (frac.clamp(0.0, 1.0) * 1000.0).round() as u16;
    let mut r = Row::new();
    if fr > 0 {
        r = r.push(container(Space::new()).width(Length::FillPortion(fr)).height(Length::Fixed(h)).style(grad(90.0, ORANGE, ORANGE2, h / 2.0, None)));
    }
    if fr < 1000 {
        r = r.push(Space::new().width(Length::FillPortion(1000 - fr)).height(Length::Fixed(h)));
    }
    container(r).width(width).height(Length::Fixed(h)).clip(true).style(bx(Pal::mix(tn.card2, tn.text, 0.05), h / 2.0, None)).into()
}

/// A mono line (file names in What's wrong).
pub fn mono<'a, M: 'a>(s: impl Into<String>, size: f32, c: Color) -> E<'a, M> {
    text(s.into()).size(size).color(c).font(Font::MONOSPACE).wrapping(text::Wrapping::None).into()
}
