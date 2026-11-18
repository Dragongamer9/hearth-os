//! Firehold's look (the approved mockups, docs/mockups/firehold): the warm dark palette, Inter, rounded cards, and the
//! small pieces every screen uses (a card, a pill, a button hint). Shared by every surface.

use iced::widget::{container, text, Text};
use std::sync::RwLock;

use iced::{border, font, Background, Border, Color, Degrees, Font, Gradient, Shadow, Theme, Vector};

pub const fn c(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgb8(r, g, b)
}

// ---- The colours: hearth-ui's (hosts/hearth-ui/src/theme.rs, shared with Hearth's pure-Rust apps), so Firehold
// follows the look picked in Settings (desktop.json, Mantel's and Plasma's looks alike) and changes with it at once.
// With no look picked it's Hearth's own warm look, the one the mockups are drawn in (the mockups' colours are that
// look's: bg #0f0b09, panel2 #261d18, panel3 #33271f, line #3b2e26, text #f4ece3, muted #a69484, ember #ff8a3d).

pub use hearth_ui::theme::Pal;

static PAL: RwLock<Option<Pal>> = RwLock::new(None);

fn load() -> Pal {
    hearth_ui::theme::set_default_look("hearth");
    hearth_ui::theme::load()
}

/// The look in use (read once, then again only when it changes: `reload`).
pub fn pal() -> Pal {
    if let Some(p) = PAL.read().ok().and_then(|g| *g) {
        return p;
    }
    let p = load();
    if let Ok(mut g) = PAL.write() {
        *g = Some(p);
    }
    p
}

/// desktop.json or palettes.json changed (link.rs's watch): read the look again; true when it's different.
pub fn reload() -> bool {
    let p = load();
    match PAL.write() {
        Ok(mut g) => {
            let changed = *g != Some(p);
            *g = Some(p);
            changed
        }
        Err(_) => false,
    }
}

pub fn c_bg() -> Color {
    pal().bg
}
pub fn c_bg2() -> Color {
    pal().bg2
}
pub fn c_panel() -> Color {
    pal().panel
}
pub fn c_card() -> Color {
    pal().panel2
}
pub fn c_card2() -> Color {
    pal().panel3
}
pub fn c_line() -> Color {
    pal().line
}
pub fn c_text() -> Color {
    pal().text
}
pub fn c_muted() -> Color {
    pal().muted
}
pub fn c_accent() -> Color {
    pal().accent
}
pub fn c_flame() -> Color {
    pal().flame
}
pub fn c_good() -> Color {
    pal().good
}
pub fn c_danger() -> Color {
    pal().danger
}
/// Text on the accent.
pub fn c_ink() -> Color {
    pal().ink
}
/// A surface between the page and the raised cards (0 = the page's colour, 1 = panel3).
pub fn shade(t: f32) -> Color {
    let p = pal();
    Pal::mix(p.bg, p.panel3, t)
}
/// Text between muted (0) and the text colour (1).
pub fn soft(t: f32) -> Color {
    let p = pal();
    Pal::mix(p.muted, p.text, t)
}
/// The page's warm glow: the page's colour with a little of the accent.
pub fn glow(t: f32) -> Color {
    let p = pal();
    Pal::mix(p.bg, p.accent, t)
}
/// The X button's blue (the controller's own colours, not the look's).
pub const BLUE: Color = c(0x5a, 0xb8, 0xf0);
/// Text over a game's own colours or art (the hero, the tiles): light whatever the look.
pub const ART_TEXT: Color = c(0xf4, 0xec, 0xe3);
pub const ART_SOFT: Color = c(0xe0, 0xd6, 0xcc);
/// A tag over a picture (a tile's store, GAME or WINDOW).
pub const ART_TAG: Color = c(0x1c, 0x14, 0x10);

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
    linear(180.0, &[(0.0, glow), (0.45, c_panel()), (1.0, c_bg())])
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
    let accent = c_accent();
    move |_| container::Style {
        text_color: None,
        background: Some(bg),
        border: Border { color: accent, width: 3.0, radius: radius.into() },
        shadow: Shadow { color: Color { a: 0.35, ..accent }, offset: Vector::new(0.0, 0.0), blur_radius: 18.0 },
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
