//! Firehold's look (the approved mockups, docs/mockups/firehold): the warm dark palette, Inter, rounded cards, and the
//! small pieces every screen uses (a card, a pill, a button hint). Shared by every surface.

use iced::widget::{container, text, Text};
use iced::{border, font, Background, Border, Color, Degrees, Font, Gradient, Shadow, Theme, Vector};

pub const fn c(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgb8(r, g, b)
}

pub const BG: Color = c(0x12, 0x0c, 0x09);
pub const BG2: Color = c(0x1c, 0x13, 0x0e);
pub const PANEL: Color = c(0x17, 0x11, 0x0e);
pub const CARD: Color = c(0x26, 0x1d, 0x18);
pub const CARD2: Color = c(0x33, 0x27, 0x1f);
pub const LINE: Color = c(0x3b, 0x2e, 0x26);
pub const TEXT: Color = c(0xf4, 0xec, 0xe3);
pub const MUTED: Color = c(0xa6, 0x94, 0x84);
pub const ACCENT: Color = c(0xff, 0x8a, 0x3d);
pub const FLAME: Color = c(0xff, 0xd1, 0x66);
pub const BLUE: Color = c(0x5a, 0xb8, 0xf0);
pub const GREEN: Color = c(0x6f, 0xd3, 0x9a);
pub const RED: Color = c(0xff, 0x7a, 0x7a);
pub const DARK_INK: Color = c(0x2a, 0x12, 0x06);

pub const INTER: Font = Font::with_name("Inter");
pub fn w(weight: font::Weight) -> Font {
    Font { weight, ..INTER }
}
pub fn bold() -> Font {
    w(font::Weight::Bold)
}
pub fn semi() -> Font {
    w(font::Weight::Semibold)
}
pub fn medium() -> Font {
    w(font::Weight::Medium)
}
pub const MONO: Font = Font::MONOSPACE;

pub fn rgb(t: (u8, u8, u8)) -> Color {
    Color::from_rgb8(t.0, t.1, t.2)
}

pub fn linear(deg: f32, stops: &[(f32, Color)]) -> Background {
    let mut l = iced::gradient::Linear::new(Degrees(deg));
    for (o, col) in stops {
        l = l.add_stop(*o, *col);
    }
    Background::Gradient(Gradient::Linear(l))
}

/// The pages' warm background (darker at the edges, a glow up top).
pub fn page_bg(glow: Color) -> Background {
    linear(180.0, &[(0.0, glow), (0.45, BG2), (1.0, BG)])
}

/// The Couch hero's and the game's own colours (mockups 1 and 3: purple to ember).
pub fn hero_bg() -> Background {
    linear(105.0, &[(0.0, c(0x2b, 0x1a, 0x2e)), (0.5, c(0x5a, 0x2a, 0x1c)), (1.0, c(0xa8, 0x54, 0x1e))])
}

/// A container style: background, rounded corners, optional border.
pub fn boxed(bg: impl Into<Background>, radius: f32, border_c: Option<(Color, f32)>) -> impl Fn(&Theme) -> container::Style {
    let bg = bg.into();
    move |_| container::Style {
        text_color: None,
        background: Some(bg),
        border: match border_c {
            Some((col, width)) => Border { color: col, width, radius: radius.into() },
            None => border::rounded(radius),
        },
        shadow: Shadow::default(),
        snap: true,
    }
}

/// The selected card's accent ring and glow (mockup 4).
pub fn ring(bg: impl Into<Background>, radius: f32) -> impl Fn(&Theme) -> container::Style {
    let bg = bg.into();
    move |_| container::Style {
        text_color: None,
        background: Some(bg),
        border: Border { color: ACCENT, width: 3.0, radius: radius.into() },
        shadow: Shadow { color: Color { a: 0.35, ..ACCENT }, offset: Vector::new(0.0, 0.0), blur_radius: 18.0 },
        snap: true,
    }
}

pub fn clear() -> impl Fn(&Theme) -> container::Style {
    |_| container::Style::default()
}

pub fn t<'a>(s: impl text::IntoFragment<'a>, size: f32, col: Color) -> Text<'a> {
    text(s).size(size).color(col).font(INTER)
}
pub fn tb<'a>(s: impl text::IntoFragment<'a>, size: f32, col: Color) -> Text<'a> {
    text(s).size(size).color(col).font(bold())
}
pub fn ts<'a>(s: impl text::IntoFragment<'a>, size: f32, col: Color) -> Text<'a> {
    text(s).size(size).color(col).font(semi())
}
