//! hearth-oss-kit: the pieces Hearth's open-source apps share (docs/mockups/open-source-apps; the approved boards'
//! style.css: `.chip`, `.tag`, `.sw`, `.seg`, `.tab`, `.field`, `.card`, `.cap`, `.row`, `.mark`, `.credit`), drawn in the
//! person's look (theme::Pal; the boards are Hearth's own dark look) at the size of a 1280 × 800 window (the boards'
//! 1840 px window, lengths about 0.7 of theirs, text about 0.85).
//!
//! Also: the window every one of these apps opens (its own title bar with the board's lettered mark, Describe it, the
//! credit line naming the project it's built on and its licence), checking that project is on this device and
//! installing it the Hearth way (Flathub, for this person only, `flatpak install --user`; nothing system-wide, nothing
//! that touches starting the device), and the safe writers (settings_file.rs, red flag 9).

pub mod need;
pub mod yaml;

use hearth_ui::describe::Describe;
use hearth_ui::ui::{self, t, tb, tm, TitleMsgs};
use hearth_ui::Pal;
use iced::widget::text::Wrapping;
use iced::widget::{column, container, mouse_area, row, text_input, Column, Row, Space};
use iced::{mouse, Background, Border, Color, Element, Length, Padding, Shadow, Theme};

pub type E<'a, M> = Element<'a, M>;

// The boards' own colours for what isn't the look's (style.css --blue, --green, --pink, --red, --yellow).
pub const BLUE: Color = Color { r: 0.369, g: 0.784, b: 1.0, a: 1.0 };
pub const GREEN: Color = Color { r: 0.608, g: 0.890, b: 0.420, a: 1.0 };
pub const PINK: Color = Color { r: 0.890, g: 0.420, b: 0.827, a: 1.0 };
pub const RED: Color = Color { r: 1.0, g: 0.420, b: 0.420, a: 1.0 };
pub const YELLOW: Color = Color { r: 1.0, g: 0.827, b: 0.420, a: 1.0 };

pub fn rgb(hex: &str) -> Color {
    hearth_ui::theme::hex(hex).unwrap_or(Color::from_rgb8(0x80, 0x80, 0x80))
}

pub fn pad(t: f32, r: f32, b: f32, l: f32) -> Padding {
    Padding { top: t, right: r, bottom: b, left: l }
}

/// Anything pressed with the pointer (nothing when `on` is None).
pub fn click<'a, M: Clone + 'a>(el: impl Into<E<'a, M>>, on: Option<M>) -> E<'a, M> {
    match on {
        Some(m) => mouse_area(el).on_press(m).interaction(mouse::Interaction::Pointer).into(),
        None => el.into(),
    }
}

fn bx(bg: Color, r: f32, line: Option<(Color, f32)>) -> impl Fn(&Theme) -> container::Style {
    ui::boxed(bg, r, line)
}

// ------------------------------------------------------------------ text

/// The page's title (`.h1`, 26 px on the board).
pub fn h1<'a, M: 'a>(p: &Pal, s: impl Into<String>) -> E<'a, M> {
    tb(s.into(), 22.0, p.text).wrapping(Wrapping::None).into()
}
/// A card's title (`.h2`, 18 px).
pub fn h2<'a, M: 'a>(p: &Pal, s: impl Into<String>) -> E<'a, M> {
    tb(s.into(), 15.0, p.text).wrapping(Wrapping::None).into()
}
/// A small heading in capitals (`.cap`).
pub fn cap<'a, M: 'a>(p: &Pal, s: impl Into<String>) -> E<'a, M> {
    tb(s.into().to_uppercase(), 10.0, p.muted).wrapping(Wrapping::None).into()
}
/// Muted text (`.mu`).
pub fn mu<'a, M: 'a>(p: &Pal, s: impl Into<String>, size: f32) -> E<'a, M> {
    t(s.into(), size, p.muted).into()
}
/// Muted text on one line.
pub fn mu1<'a, M: 'a>(p: &Pal, s: impl Into<String>, size: f32) -> E<'a, M> {
    t(s.into(), size, p.muted).wrapping(Wrapping::None).into()
}
/// Text on one line.
pub fn tx<'a, M: 'a>(p: &Pal, s: impl Into<String>, size: f32) -> E<'a, M> {
    t(s.into(), size, p.text).wrapping(Wrapping::None).into()
}
/// Text that wraps.
pub fn para<'a, M: 'a>(c: Color, s: impl Into<String>, size: f32) -> E<'a, M> {
    t(s.into(), size, c).into()
}
pub fn fill<'a, M: 'a>() -> E<'a, M> {
    Space::new().width(Length::Fill).into()
}
pub fn gap<'a, M: 'a>(w: f32, h: f32) -> E<'a, M> {
    ui::gap(w, h)
}

// ------------------------------------------------------------------ chips, tags, switches

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Ch {
    /// `.chip`: the panel with a line
    Plain,
    /// `.chip.acc`: the accent, ink text
    Acc,
    /// `.chip.ghost`: no fill
    Ghost,
    /// a red word (Delete…)
    Bad,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Sz {
    Sm,
    Md,
    Lg,
}

/// A button as the boards draw it (`.chip`, `.chip.sm`, `.chip.lg`, `.chip.acc`).
pub fn chip<'a, M: Clone + 'a>(p: &Pal, label: impl Into<String>, kind: Ch, size: Sz, on: Option<M>) -> E<'a, M> {
    let (h, fs, ph, r) = match size {
        Sz::Sm => (22.0, 11.0, 8.0, 7.0),
        Sz::Md => (28.0, 13.0, 11.0, 8.0),
        Sz::Lg => (34.0, 14.0, 16.0, 10.0),
    };
    let enabled = on.is_some();
    let (bg, fg, line) = match kind {
        Ch::Acc => (p.accent, p.ink, p.accent),
        Ch::Plain => (p.panel2, p.text, p.line),
        Ch::Ghost => (Color::TRANSPARENT, p.text, p.line),
        Ch::Bad => (p.panel2, Color::from_rgb8(0xff, 0x8a, 0x8a), p.line),
    };
    let fg = if enabled || kind == Ch::Acc { fg } else { Pal::a(fg, 0.45) };
    let label = label.into();
    let txt = if kind == Ch::Acc { tb(label, fs, fg) } else { t(label, fs, fg) };
    let c = container(txt.wrapping(Wrapping::None)).center_y(Length::Fixed(h)).padding(pad(0.0, ph, 0.0, ph)).style(bx(bg, p.r(r), Some((line, 1.0))));
    click(c, on)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tag {
    Acc,
    Dk,
    Ok,
    Bl,
    Warn,
    Bad,
}

/// A small label in capitals (`.tag`, `.tag.dk`, `.tag.ok`, `.tag.bl`, `.tag.warn`, `.tag.bad`).
pub fn tag<'a, M: 'a>(p: &Pal, label: impl Into<String>, kind: Tag) -> E<'a, M> {
    let (bg, fg, line) = match kind {
        Tag::Acc => (p.accent, p.ink, p.accent),
        Tag::Dk => (p.panel2, p.muted, p.line),
        Tag::Ok => (rgb("#21361b"), GREEN, rgb("#3b5a2e")),
        Tag::Bl => (rgb("#173040"), BLUE, rgb("#2d5670")),
        Tag::Warn => (rgb("#3d2a14"), YELLOW, rgb("#6a4a1e")),
        Tag::Bad => (rgb("#3d1717"), RED, rgb("#6a2a2a")),
    };
    container(tb(label.into().to_uppercase(), 9.0, fg).wrapping(Wrapping::None)).padding(pad(2.0, 6.0, 2.0, 6.0)).style(bx(bg, 5.0, Some((line, 1.0)))).into()
}

/// A switch (`.sw`, `.sw.off`): the accent with the knob on the right when on.
pub fn switch<'a, M: Clone + 'a>(p: &Pal, on: bool, m: Option<M>) -> E<'a, M> {
    let knob = container(Space::new()).width(Length::Fixed(12.0)).height(Length::Fixed(12.0)).style(bx(Color::WHITE, 6.0, None));
    let inner: E<'a, M> = if on { row![fill(), knob].into() } else { row![knob, fill()].into() };
    let track = container(inner).padding(3.0).width(Length::Fixed(30.0)).height(Length::Fixed(18.0)).style(bx(if on { p.accent } else { p.line }, 9.0, None));
    click(track, m)
}

/// Choices side by side (`.seg`): the picked one in the accent.
pub fn seg<'a, M: Clone + 'a>(p: &Pal, items: Vec<(String, bool, M)>) -> E<'a, M> {
    let mut r = Row::new();
    for (l, on, m) in items {
        let txt = if on { tb(l, 11.0, p.ink) } else { t(l, 11.0, p.muted) };
        let c = container(txt.wrapping(Wrapping::None)).padding(pad(4.0, 8.0, 4.0, 8.0)).style(bx(if on { p.accent } else { Color::TRANSPARENT }, 0.0, None));
        r = r.push(click(c, Some(m)));
    }
    container(r).clip(true).style(bx(Color::TRANSPARENT, p.r(7.0), Some((p.line, 1.0)))).into()
}

/// Tabs (`.tabs` / `.tab`): each its own outlined chip, the picked one in the accent.
pub fn tabs<'a, M: Clone + 'a>(p: &Pal, items: Vec<(String, bool, M)>) -> E<'a, M> {
    let mut r = Row::new().spacing(5.0);
    for (l, on, m) in items {
        let txt = if on { tb(l, 11.0, p.ink) } else { t(l, 11.0, p.muted) };
        let c = container(txt.wrapping(Wrapping::None)).padding(pad(4.0, 8.0, 4.0, 8.0)).style(bx(if on { p.accent } else { Color::TRANSPARENT }, p.r(7.0), Some((if on { p.accent } else { p.line }, 1.0))));
        r = r.push(click(c, Some(m)));
    }
    r.into()
}

/// A field that opens a list (`.field` with ▾), or shows a value.
pub fn field<'a, M: Clone + 'a>(p: &Pal, label: impl Into<String>, min_w: f32, on: Option<M>) -> E<'a, M> {
    let c = container(t(label.into(), 11.0, p.text).wrapping(Wrapping::None)).center_y(Length::Fixed(26.0)).padding(pad(0.0, 9.0, 0.0, 9.0)).width(Length::Shrink).style(bx(p.bg, p.r(7.0), Some((p.line, 1.0))));
    let c = container(c).width(if min_w > 0.0 { Length::Fixed(min_w) } else { Length::Shrink });
    click(c, on)
}

/// A text box in the boards' look (`.search`, `.field`).
pub fn input<'a, M: Clone + 'a>(p: &Pal, placeholder: &str, value: &str, on: impl Fn(String) -> M + 'a) -> iced::widget::TextInput<'a, M> {
    let p = *p;
    text_input(placeholder, value).on_input(on).size(12.0).padding(pad(6.0, 10.0, 6.0, 10.0)).style(move |_t: &Theme, st: text_input::Status| text_input::Style {
        background: Background::Color(p.bg),
        border: Border { color: if matches!(st, text_input::Status::Focused { .. }) { p.accent } else { p.line }, width: 1.0, radius: p.r(8.0).into() },
        icon: p.muted,
        placeholder: p.muted,
        value: p.text,
        selection: Pal::a(p.accent, 0.35),
    })
}

/// A search box with its ⌕.
pub fn search<'a, M: Clone + 'a>(p: &Pal, placeholder: &str, value: &str, on: impl Fn(String) -> M + 'a) -> E<'a, M> {
    input(p, &format!("⌕ {placeholder}"), value, on).into()
}

/// A tick box (`.ck`): accent square when on.
pub fn check<'a, M: Clone + 'a>(p: &Pal, on: bool, label: impl Into<String>, m: M) -> E<'a, M> {
    let bx_ = container(if on { container(Space::new()).width(Length::Fixed(6.0)).height(Length::Fixed(6.0)).style(bx(p.accent, 1.0, None)).into() } else { gap::<M>(6.0, 6.0) })
        .center_x(Length::Fixed(12.0))
        .center_y(Length::Fixed(12.0))
        .style(bx(Color::TRANSPARENT, 3.0, Some((if on { p.accent } else { p.line }, 1.5))));
    click(row![bx_, t(label.into(), 12.0, p.text).wrapping(Wrapping::None)].spacing(9.0).align_y(iced::Center).padding(pad(3.0, 0.0, 3.0, 0.0)), Some(m))
}

/// A card (`.card`): the panel, a line, rounded, padding 12.
pub fn card<'a, M: 'a>(p: &Pal, inner: impl Into<E<'a, M>>) -> E<'a, M> {
    container(inner).padding(12.0).width(Length::Fill).height(Length::Fill).style(bx(p.panel, p.r(10.0), Some((p.line, 1.0)))).into()
}
/// A card as tall as what's in it.
pub fn card_fit<'a, M: 'a>(p: &Pal, inner: impl Into<E<'a, M>>) -> E<'a, M> {
    container(inner).padding(12.0).width(Length::Fill).style(bx(p.panel, p.r(10.0), Some((p.line, 1.0)))).into()
}
/// A card in another colour (the boards' notes: "Proton Hearth 9-5 is out").
pub fn note_card<'a, M: 'a>(p: &Pal, inner: impl Into<E<'a, M>>, bg: Color, line: Color, dashed: bool) -> E<'a, M> {
    let _ = dashed;
    container(inner).padding(pad(9.0, 11.0, 9.0, 11.0)).width(Length::Fill).style(bx(bg, p.r(10.0), Some((line, 1.0)))).into()
}

/// A settings row (`.row`): its words on the left, what's on its right.
pub fn kv<'a, M: 'a>(p: &Pal, label: impl Into<String>, right: impl Into<E<'a, M>>) -> E<'a, M> {
    row![t(label.into(), 12.0, p.text).wrapping(Wrapping::None), fill(), right.into()].spacing(8.0).align_y(iced::Center).padding(pad(3.0, 0.0, 3.0, 0.0)).into()
}

/// A line across (`border-bottom: 1px solid`).
pub fn line<'a, M: 'a>(p: &Pal) -> E<'a, M> {
    ui::hline(Pal::mix(p.line, p.panel, 0.35))
}

/// A filled bar (`.bar`): `frac` of it in the accent.
pub fn bar<'a, M: 'a>(p: &Pal, frac: f32, h: f32, w: Length) -> E<'a, M> {
    let f = frac.clamp(0.0, 1.0);
    let on = (f * 1000.0).round() as u16;
    let mut r = Row::new();
    if on > 0 {
        r = r.push(container(Space::new()).width(Length::FillPortion(on)).height(Length::Fixed(h)).style(bx(p.accent, h / 2.0, None)));
    }
    if on < 1000 {
        r = r.push(Space::new().width(Length::FillPortion(1000 - on)).height(Length::Fixed(h)));
    }
    container(r).width(w).height(Length::Fixed(h)).clip(true).style(bx(p.line, h / 2.0, None)).into()
}

/// A dot in a colour (●).
pub fn dot<'a, M: 'a>(c: Color, d: f32) -> E<'a, M> {
    container(Space::new()).width(Length::Fixed(d)).height(Length::Fixed(d)).style(bx(c, d / 2.0, None)).into()
}

/// The boards' lettered mark (`.mark`): two or three letters on the app's colours.
pub fn mark<'a, M: 'a>(letters: &str, from: Color, to: Color, size: f32) -> E<'a, M> {
    let g = iced::gradient::Linear::new(iced::Degrees(135.0)).add_stop(0.0, from).add_stop(1.0, to);
    let fs = (size * 0.42).round().max(7.0);
    // the gear (Settings' own mark) is drawn, not a letter
    let inner: E<'a, M> = if letters == "⚙" { ui::glyph(ui::G::Gear, Color::WHITE, (size * 0.62).round(), 2.0) } else { tb(letters.to_string(), fs, Color::WHITE).wrapping(Wrapping::None).into() };
    container(inner)
        .center_x(Length::Fixed(size))
        .center_y(Length::Fixed(size))
        .style(move |_: &Theme| container::Style {
            text_color: None,
            background: Some(Background::Gradient(iced::Gradient::Linear(g))),
            border: Border { color: Color::TRANSPARENT, width: 0.0, radius: (size * 0.3).into() },
            shadow: Shadow::default(),
            snap: true,
        })
        .into()
}

/// A slider in the accent (`.sl`).
pub fn slider<'a, M: Clone + 'a>(p: &Pal, range: std::ops::RangeInclusive<f32>, v: f32, w: f32, on: impl Fn(f32) -> M + 'a, done: Option<M>) -> E<'a, M> {
    use iced::widget::slider as sl;
    let p = *p;
    let lo = *range.start();
    let hi = *range.end();
    let mut s = sl(range, v.clamp(lo, hi), on).step(1.0).width(Length::Fixed(w)).style(move |_: &Theme, _| sl::Style {
        rail: sl::Rail { backgrounds: (Background::Color(p.accent), Background::Color(p.line)), width: 4.0, border: Border { color: Color::TRANSPARENT, width: 0.0, radius: 2.0.into() } },
        handle: sl::Handle { shape: sl::HandleShape::Circle { radius: 7.0 }, background: Background::Color(Color::from_rgb8(0xf1, 0xf1, 0xf1)), border_width: 0.0, border_color: Color::TRANSPARENT },
    });
    if let Some(d) = done {
        s = s.on_release(d);
    }
    s.into()
}

// ------------------------------------------------------------------ lists to pick from, menus

/// A list to pick from (a field's ▾ or a ⋯ menu): its title, the choices (the picked one ticked), Close.
pub fn chooser<'a, M: Clone + 'a>(p: &Pal, title: impl Into<String>, opts: Vec<(String, bool, M)>, close: M) -> E<'a, M> {
    let mut c = Column::new().spacing(2.0).push(row![h2::<M>(p, title.into()), fill(), chip(p, "Close", Ch::Plain, Sz::Sm, Some(close))].align_y(iced::Center)).push(gap(1.0, 8.0));
    if opts.is_empty() {
        c = c.push(mu(p, "Nothing to pick from here yet.", 12.0));
    }
    for (l, on, m) in opts {
        let mark: E<'a, M> = if on { ui::glyph(ui::G::Check, p.accent, 12.0, 2.2) } else { gap(12.0, 12.0) };
        c = c.push(click(container(row![mark, t(l, 13.0, p.text)].spacing(10.0).align_y(iced::Center)).padding(pad(7.0, 10.0, 7.0, 8.0)).width(Length::Fill).style(bx(if on { p.sel() } else { Color::TRANSPARENT }, p.r(6.0), None)), Some(m)));
    }
    ui::dialog_box(p, 380.0, iced::widget::scrollable(c).height(Length::Shrink))
}

/// Something said at the bottom of a page (what happened, what went wrong).
pub fn said<'a, M: 'a>(p: &Pal, s: &str) -> E<'a, M> {
    if s.is_empty() {
        return gap(1.0, 1.0);
    }
    let bad = s.starts_with("Couldn't") || s.contains("didn't") || s.contains("isn't");
    container(t(s.to_string(), 12.0, if bad { RED } else { p.text })).padding(pad(6.0, 10.0, 6.0, 10.0)).width(Length::Fill).style(bx(p.panel2, p.r(8.0), Some((p.line, 1.0)))).into()
}

// ------------------------------------------------------------------ the window

/// What the title bar shows and asks for.
pub struct Bar<'a, M> {
    pub letters: &'a str,
    pub from: Color,
    pub to: Color,
    pub title: &'a str,
    /// the muted line after the name ("· any Windows game, with the community's fixes")
    pub line: String,
    /// what's in the middle of the bar (Hearth 3D's menus and modes)
    pub middle: Option<E<'a, M>>,
    /// the buttons on its right, before Minimize (+ Install a game)
    pub right: Vec<E<'a, M>>,
    pub msgs: TitleMsgs<M>,
    /// a right-click on the bar: Describe it
    pub open: M,
}

/// The whole window: its bar, the body, the credit line ("Hearth Game Fixer · Built on Lutris (GPL-3)"), the edges
/// that resize it, and Describe it's box under it while it's open.
pub fn window<'a, M: Clone + 'a>(
    p: &Pal,
    d: &'a Describe,
    bar: Bar<'a, M>,
    body: E<'a, M>,
    credit: (&'a str, &'a str),
    resize: impl Fn(iced::window::Direction) -> M,
    map: impl Fn(hearth_ui::describe::DMsg) -> M + Clone + 'a,
) -> E<'a, M> {
    let mut mid = Row::new().spacing(8.0).align_y(iced::Center);
    if !bar.line.is_empty() {
        mid = mid.push(t(bar.line, 12.0, p.muted).wrapping(Wrapping::None));
    }
    if let Some(m) = bar.middle {
        mid = mid.push(m);
    }
    mid = mid.push(Space::new().width(Length::Fill).height(Length::Fixed(1.0)));
    for r in bar.right {
        mid = mid.push(r);
    }
    let mid = mouse_area(container(mid).width(Length::FillPortion(40)).center_y(Length::Fixed(40.0))).on_press(bar.msgs.drag.clone()).on_double_click(bar.msgs.maximize.clone()).on_right_press(bar.open.clone());
    let top = hearth_ui::describe::title_bar_with(p, d, mark(bar.letters, bar.from, bar.to, 18.0), bar.title, Some(mid.into()), bar.msgs, Pal::mix(p.bg, Color::BLACK, 0.2), bar.open);
    let foot = container(row![fill(), t(format!("{} · ", credit.0), 10.0, p.muted), tb(credit.1.to_string(), 10.0, p.muted)].align_y(iced::Center)).padding(pad(3.0, 14.0, 4.0, 14.0)).width(Length::Fill);
    let all = container(column![top, ui::hline(p.line), container(body).width(Length::Fill).height(Length::Fill), foot]).width(Length::Fill).height(Length::Fill).style(bx(p.bg, p.window_corner, Some((p.line, 1.0))));
    let framed = ui::edges(all.into(), resize);
    hearth_ui::describe::with_bar(framed, p, d, map)
}

/// The side's "Edit layout" (the owner, Nov 24: always findable).
pub fn edit_layout<'a, M: Clone + 'a>(p: &Pal, m: M) -> E<'a, M> {
    click(container(row![ui::glyph(ui::G::Pencil, p.muted, 12.0, 1.8), tm("Edit layout", 12.0, p.muted)].spacing(8.0).align_y(iced::Center)).padding(pad(6.0, 8.0, 6.0, 8.0)), Some(m))
}

/// A side column (`.side`): darker, a line on its right.
pub fn side<'a, M: 'a>(p: &Pal, inner: impl Into<E<'a, M>>, w: f32) -> E<'a, M> {
    container(inner).width(Length::Fixed(w)).height(Length::Fill).padding(pad(10.0, 9.0, 10.0, 9.0)).style(bx(Pal::mix(p.bg, Color::BLACK, 0.15), 0.0, None)).into()
}

/// A row of the side's list (`.nav`, `.gl`, `.bt`): picked, it's lit with the accent's bar on its left.
pub fn side_row<'a, M: Clone + 'a>(p: &Pal, inner: impl Into<E<'a, M>>, on: bool, h: f32, m: M) -> E<'a, M> {
    let lit = if on { Pal::mix(p.panel2, p.accent, 0.18) } else { Color::TRANSPARENT };
    let edge: E<'a, M> = container(Space::new()).width(Length::Fixed(3.0)).height(Length::Fixed(h)).style(bx(if on { p.accent } else { Color::TRANSPARENT }, 1.5, None)).into();
    let body = container(row![edge, container(inner).padding(pad(6.0, 8.0, 6.0, 6.0)).width(Length::Fill)].align_y(iced::Center)).width(Length::Fill).style(bx(lit, p.r(8.0), None));
    click(body, Some(m))
}

// ------------------------------------------------------------------ picking a file

/// Hearth's Open window for one file (hearth-file-chooser; zenity where it isn't), off the window's
/// thread: the path picked, or None.
pub fn pick_file(title: &str, patterns: &[&str]) -> impl std::future::Future<Output = Option<String>> + Send + 'static {
    let title = title.to_string();
    let pats: Vec<String> = patterns.iter().map(|s| s.to_string()).collect();
    let (tx, rx) = futures::channel::oneshot::channel();
    let _ = std::thread::Builder::new().name("hearth-pick".into()).spawn(move || {
        let dir = std::env::var_os("XDG_RUNTIME_DIR").map(std::path::PathBuf::from).unwrap_or_else(std::env::temp_dir);
        let req = dir.join(format!("hearth-pick-{}.json", std::process::id()));
        let res = dir.join(format!("hearth-pick-{}-answer.json", std::process::id()));
        let _ = std::fs::remove_file(&res);
        let body = serde_json::json!({"kind": "open", "title": title, "multiple": false, "filters": [{"name": title, "patterns": pats}]});
        let mut got: Option<String> = None;
        if std::fs::write(&req, body.to_string()).is_ok() {
            let ok = std::process::Command::new("hearth-file-chooser").env("HEARTH_PICK_REQUEST", &req).env("HEARTH_PICK_RESULT", &res).status().map(|s| s.success()).unwrap_or(false);
            if ok {
                got = std::fs::read_to_string(&res).ok().and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok()).and_then(|v| v.get("paths").and_then(|p| p.get(0)).and_then(|p| p.as_str()).map(str::to_string));
            }
            let _ = std::fs::remove_file(&req);
            let _ = std::fs::remove_file(&res);
        }
        if got.is_none() {
            let filter = pats.join(" ");
            for cmd in [vec!["zenity".to_string(), "--file-selection".into(), format!("--file-filter={filter}")]] {
                if let Ok(o) = std::process::Command::new(&cmd[0]).args(&cmd[1..]).output() {
                    let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
                    if o.status.success() && !s.is_empty() {
                        got = Some(s);
                        break;
                    }
                }
            }
        }
        let _ = tx.send(got);
    });
    async move { rx.await.ok().flatten() }
}

// ------------------------------------------------------------------ safe writes (red flag 9)

pub use hearth_ui::settings_file;

/// This app's own settings file: ~/.config/nimbus/<app>.json (read; a damaged one comes back from its .bak).
pub fn settings_path(app: &str) -> std::path::PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME").map(std::path::PathBuf::from).unwrap_or_else(|| hearth_ui::paths::home().join(".config"));
    base.join("nimbus").join(format!("{app}.json"))
}
pub fn read_settings(app: &str) -> serde_json::Value {
    settings_file::read_json(&settings_path(app))
}
/// Sets these keys only (merge-on-write, atomic, a .bak); every other key stays as it is on disk.
pub fn save_settings(app: &str, keys: serde_json::Map<String, serde_json::Value>) -> Result<(), String> {
    let p = settings_path(app);
    if let Some(d) = p.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    settings_file::merge_json(&p, &keys).map(|_| ()).map_err(|e| e.to_string())
}

/// Where an app keeps what it makes: ~/.local/share/hearth/<app>.
pub fn data_dir(app: &str) -> std::path::PathBuf {
    let base = std::env::var_os("XDG_DATA_HOME").map(std::path::PathBuf::from).unwrap_or_else(|| hearth_ui::paths::home().join(".local/share"));
    base.join("hearth").join(app)
}

pub fn home() -> std::path::PathBuf {
    hearth_ui::paths::home()
}

/// "~/…" for a path under the home folder.
pub fn tilde(p: &std::path::Path) -> String {
    let h = home();
    match p.strip_prefix(&h) {
        Ok(r) => format!("~/{}", r.display()),
        Err(_) => p.display().to_string(),
    }
}

/// A size in words (1.2 GB, 340 MB).
pub fn size_words(b: u64) -> String {
    let gb = b as f64 / 1_073_741_824.0;
    if (1.0..100.0).contains(&gb) { format!("{gb:.1} GB") } else { ui::bytes(b as f64) }
}

/// The total size of a folder (what's in it, not following links).
pub fn folder_size(p: &std::path::Path) -> u64 {
    let mut n = 0u64;
    let mut stack = vec![p.to_path_buf()];
    let mut seen = 0usize;
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            seen += 1;
            if seen > 400_000 {
                return n;
            }
            let Ok(m) = e.metadata() else { continue };
            if m.is_dir() {
                stack.push(e.path());
            } else {
                n += m.len();
            }
        }
    }
    n
}

/// Month and day in words ("Nov 9") from an ISO date ("2026-11-09…").
pub fn day_words(iso: &str) -> String {
    const M: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let mut it = iso.get(..10).unwrap_or("").split('-');
    let (_, m, d) = (it.next(), it.next().and_then(|x| x.parse::<usize>().ok()), it.next().and_then(|x| x.parse::<u32>().ok()));
    match (m, d) {
        (Some(m), Some(d)) if (1..=12).contains(&m) => format!("{} {}", M[m - 1], d),
        _ => String::new(),
    }
}

/// Thousands with commas (2,314).
pub fn commas(n: u64) -> String {
    let s = n.to_string();
    let mut o = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            o.push(',');
        }
        o.push(c);
    }
    o
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn words() {
        assert_eq!(commas(2314), "2,314");
        assert_eq!(commas(18904), "18,904");
        assert_eq!(commas(12), "12");
        assert_eq!(day_words("2026-11-09T10:00:00Z"), "Nov 9");
        assert_eq!(day_words("x"), "");
    }
}
