//! Mantel's look in Rust: the same palette the QML Mantel draws with (theme/palettes.json and desktop.json, read by
//! hearth-ui's theme, shared with Hearth's Rust apps), the look's bar style ("islandStyle": floating islands, one
//! full-width bar, Zen's thin one) and corners. Read once, and again only when one of those files is written.

use iced::widget::{container, text, Text};
use iced::{border, font, Border, Color, Font, Shadow, Theme, Vector};
use serde_json::Value;
use std::sync::RwLock;

pub use hearth_ui::theme::Pal;

static PAL: RwLock<Option<(Pal, String, String)>> = RwLock::new(None);

fn read() -> (Pal, String, String) {
    let p = hearth_ui::theme::load();
    let d: Value = std::fs::read(hearth_ui::theme::desktop_json()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null);
    let map = hearth_ui::theme::palette_map();
    let mut name = d.get("look").and_then(Value::as_str).unwrap_or("").to_string();
    if !map.get(&name).map(Value::is_object).unwrap_or(false) || name.starts_with('_') || name == "accents" || name == "blackSet" {
        name = "steam".into();
    }
    let style = map.get(&name).and_then(|l| l.get("islandStyle")).and_then(Value::as_str).unwrap_or("floating").to_string();
    (p, name, style)
}

fn get() -> (Pal, String, String) {
    if let Some(v) = PAL.read().ok().and_then(|g| g.clone()) {
        return v;
    }
    let v = read();
    if let Ok(mut g) = PAL.write() {
        *g = Some(v.clone());
    }
    v
}

/// The look in use.
pub fn pal() -> Pal {
    get().0
}
/// Its name in palettes.json ("steam", "hearth", "zen"…).
pub fn look_name() -> String {
    get().1
}
/// "floating" (islands), "bar" (one full-width bar), "thin" (Zen) or "console".
pub fn bar_style() -> String {
    get().2
}

/// desktop.json or palettes.json was written: read the look again; true when it changed.
pub fn reload() -> bool {
    let v = read();
    match PAL.write() {
        Ok(mut g) => {
            let changed = g.as_ref().map(|o| o.0 != v.0 || o.1 != v.1 || o.2 != v.2).unwrap_or(true);
            *g = Some(v);
            changed
        }
        Err(_) => false,
    }
}

/// A stream that wakes the surface when the look may have changed (hearth-ui's inotify watch: asleep otherwise).
pub fn changes() -> impl futures::Stream<Item = ()> {
    use futures::StreamExt;
    hearth_ui::theme::watch().filter(|_| futures::future::ready(reload()))
}

pub const INTER: Font = Font::with_name("Inter");
pub fn weight(w: font::Weight) -> Font {
    Font { weight: w, ..INTER }
}
pub fn bold() -> Font {
    weight(font::Weight::Bold)
}
pub fn semi() -> Font {
    weight(font::Weight::Semibold)
}

pub fn t<'a>(s: impl text::IntoFragment<'a>, size: f32, c: Color) -> Text<'a> {
    text(s).size(size).color(c).font(INTER)
}
pub fn tb<'a>(s: impl text::IntoFragment<'a>, size: f32, c: Color) -> Text<'a> {
    text(s).size(size).color(c).font(semi())
}

pub fn a(c: Color, alpha: f32) -> Color {
    Color { a: alpha, ..c }
}

/// An island (the floating rounded pill of Mantel 2026's bar) or the full-width bar's panel.
pub fn island(p: Pal, radius: f32, lit: bool) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: Some((if lit { Pal::mix(p.panel, p.accent, 0.18) } else { a(p.panel, if p.dark { 0.92 } else { 0.95 }) }).into()),
        border: Border { color: a(p.line, 0.9), width: 1.0, radius: p.r(radius).into() },
        shadow: Shadow { color: Color { a: 0.35, ..Color::BLACK }, offset: Vector::new(0.0, 4.0), blur_radius: 14.0 },
        ..container::Style::default()
    }
}

/// A plain rounded box.
pub fn boxed(bg: Color, radius: f32) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style { background: Some(bg.into()), border: border::rounded(radius), ..container::Style::default() }
}

/// Nothing drawn (the surface's see-through parts).
pub fn clear() -> impl Fn(&Theme) -> container::Style {
    |_| container::Style::default()
}

/// Where Hearth's desktop files are (the QML Mantel's folder: its icons, pictures and wallpapers).
pub fn share() -> std::path::PathBuf {
    if let Some(s) = std::env::var_os("NIMBUS_SHARE") {
        return std::path::PathBuf::from(s);
    }
    for p in ["/usr/share/nimbus", "/usr/local/share/nimbus"] {
        if std::path::Path::new(p).join("desktop").exists() {
            return p.into();
        }
    }
    "/usr/share/nimbus".into()
}

/// The tone of one of Mantel's own icons (shell/icons/<name>-<tone>.svg): the same pictures the QML Mantel draws.
#[derive(Clone, Copy)]
pub enum Tone {
    Light,
    Muted,
    Accent,
    Ink,
}

pub fn icon<'a, M: 'a>(name: &str, tone: Tone, size: f32) -> iced::Element<'a, M> {
    let p = pal();
    // Light looks draw the "light" (white) set dark: muted then, as the QML Mantel does.
    let tone = match (tone, p.dark) {
        (Tone::Light, false) => Tone::Ink,
        (t, _) => t,
    };
    let t = match tone {
        Tone::Light => "light",
        Tone::Muted => "muted",
        Tone::Accent => "accent",
        Tone::Ink => "ink",
    };
    let f = share().join("desktop/shell/icons").join(format!("{name}-{t}.svg"));
    iced::widget::svg(iced::widget::svg::Handle::from_path(f)).width(size).height(size).into()
}
