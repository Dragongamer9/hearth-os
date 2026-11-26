//! The boards' small parts (style.css of hearth-workspace hearth/mockups/2026-11-26-store-apps), drawn in the
//! person's chosen look: .sw (a switch), .seg (choices side by side), .field, .chip (sm / lg / acc / ghost), .tag
//! (ok / bl / warn / bad / dk), .mark (Hearth's own letter marks: no other company's logo), .cap (a section label),
//! .nav (a side row with its count), .row (a label and its control), .bar, .sl (a slider's look), bars (a graph of
//! bars, a waveform), a ring (a countdown) and the credit line every board ends with.

use hearth_ui::ui::{self, glyph, t, tb, tm, G};
use hearth_ui::Pal;
use iced::widget::canvas::{self, Frame, Geometry, Path, Stroke};
use iced::widget::text::Wrapping;
use iced::widget::{container, mouse_area, row, text_input, Column, Row, Space};
use iced::{mouse, Background, Border, Color, Element, Length, Padding, Point, Rectangle, Renderer, Shadow, Theme};

pub type E<'a, M> = Element<'a, M>;

/// The boards' colours beside the look's own: blue, green, pink, red, yellow.
pub fn blue() -> Color {
    Color::from_rgb8(0x5e, 0xc8, 0xff)
}
pub fn green() -> Color {
    Color::from_rgb8(0x9b, 0xe3, 0x6b)
}
pub fn pink() -> Color {
    Color::from_rgb8(0xe3, 0x6b, 0xd3)
}
pub fn red() -> Color {
    Color::from_rgb8(0xff, 0x6b, 0x6b)
}
pub fn yellow() -> Color {
    Color::from_rgb8(0xff, 0xd3, 0x6b)
}
pub fn purple() -> Color {
    Color::from_rgb8(0xb9, 0x9b, 0xff)
}
pub fn lime() -> Color {
    Color::from_rgb8(0x9b, 0xe3, 0x6b)
}
pub fn hex(s: &str) -> Color {
    hearth_ui::theme::hex(s).unwrap_or(Color::from_rgb8(0x9a, 0x9a, 0xa6))
}

/// The letter marks' colours (.g-or … .g-gy): from, to.
pub fn grad(name: &str) -> (Color, Color) {
    let (a, b) = match name {
        "bl" => ("#7fd6ff", "#2c7fd8"),
        "gr" => ("#b8f07f", "#3ba55c"),
        "pk" => ("#ff9be8", "#b04fc8"),
        "pu" => ("#b9a2ff", "#6a4fe0"),
        "rd" => ("#ff9a8a", "#d8413a"),
        "yl" => ("#ffe28a", "#f0a020"),
        "tl" => ("#7ff0d8", "#1f9f8f"),
        "gy" => ("#c9c1ba", "#6d645d"),
        _ => ("#ffb36b", "#ff6a1f"),
    };
    (hex(a), hex(b))
}
/// A colour name for any text (the same text always gets the same one).
pub fn grad_for(s: &str) -> &'static str {
    const N: [&str; 9] = ["or", "bl", "gr", "pk", "pu", "rd", "yl", "tl", "gy"];
    let h = s.bytes().fold(7u32, |h, b| h.wrapping_mul(31).wrapping_add(b as u32));
    N[(h % 9) as usize]
}

fn press<'a, M: Clone + 'a>(el: E<'a, M>, msg: Option<M>) -> E<'a, M> {
    match msg {
        Some(m) => mouse_area(el).on_press(m).interaction(mouse::Interaction::Pointer).into(),
        None => el,
    }
}

/// .sw: the accent track with the white knob on the right when on (34 × 20; `k` scales it: the TV's 1.4).
pub fn sw<'a, M: Clone + 'a>(p: &Pal, on: bool, msg: Option<M>) -> E<'a, M> {
    sw_k(p, on, msg, 1.0)
}
pub fn sw_k<'a, M: Clone + 'a>(p: &Pal, on: bool, msg: Option<M>, k: f32) -> E<'a, M> {
    let knob = container(Space::new()).width(Length::Fixed(14.0 * k)).height(Length::Fixed(14.0 * k)).style(ui::boxed(Color::WHITE, 7.0 * k, None));
    let inner: E<'a, M> = if on { row![Space::new().width(Length::Fill), knob].into() } else { row![knob, Space::new().width(Length::Fill)].into() };
    let track = container(inner).padding(3.0 * k).width(Length::Fixed(34.0 * k)).height(Length::Fixed(20.0 * k)).style(ui::boxed(if on { p.accent } else { p.line }, 10.0 * k, None));
    press(track.into(), msg)
}

/// .seg: choices side by side in a line; the picked one in the accent.
pub fn seg<'a, M: Clone + 'a>(p: &Pal, items: Vec<(String, bool, Option<M>)>) -> E<'a, M> {
    seg_k(p, items, 12.0)
}
pub fn seg_k<'a, M: Clone + 'a>(p: &Pal, items: Vec<(String, bool, Option<M>)>, size: f32) -> E<'a, M> {
    let mut r = Row::new();
    let pad = size * 0.75;
    for (label, on, msg) in items {
        let txt = if on { tb(label, size, p.ink) } else { t(label, size, p.muted) };
        let cell = container(txt.wrapping(Wrapping::None)).padding(Padding { top: size * 0.42, right: pad, bottom: size * 0.42, left: pad }).style(ui::boxed(if on { p.accent } else { Color::TRANSPARENT }, 0.0, None));
        r = r.push(press(cell.into(), if on { None } else { msg }));
    }
    container(r).clip(true).style(ui::boxed(Color::TRANSPARENT, p.r(8.0), Some((p.line, 1.0)))).into()
}

/// .tabs / .tab: separate little choices (the picked one in the accent).
pub fn tabs<'a, M: Clone + 'a>(p: &Pal, items: Vec<(String, bool, Option<M>)>) -> E<'a, M> {
    let mut r = Row::new().spacing(6.0);
    for (label, on, msg) in items {
        let txt = if on { tb(label, 12.0, p.ink) } else { t(label, 12.0, p.muted) };
        let cell = container(txt.wrapping(Wrapping::None))
            .padding(Padding { top: 6.0, right: 10.0, bottom: 6.0, left: 10.0 })
            .style(ui::boxed(if on { p.accent } else { Color::TRANSPARENT }, p.r(8.0), Some((if on { p.accent } else { p.line }, 1.0))));
        r = r.push(press(cell.into(), msg));
    }
    r.into()
}

/// .field: a value in a box (a choice with ▾, a number, a size); `w` 0 is as wide as its words.
pub fn field<'a, M: Clone + 'a>(p: &Pal, label: impl Into<String>, w: f32, msg: Option<M>) -> E<'a, M> {
    let c = container(t(label.into(), 12.0, p.text).wrapping(Wrapping::None))
        .padding(Padding { top: 0.0, right: 10.0, bottom: 0.0, left: 10.0 })
        .center_y(Length::Fixed(30.0))
        .width(if w > 0.0 { Length::Fixed(w) } else { Length::Shrink })
        .style(ui::boxed(p.bg, p.r(8.0), Some((p.line, 1.0))));
    press(c.into(), msg)
}

/// A field's choices, open under it (one of them picked).
pub fn menu<'a, M: Clone + 'a>(p: &Pal, items: Vec<(String, bool, M)>, w: f32) -> E<'a, M> {
    let mut c = Column::new().spacing(2.0);
    for (label, on, m) in items {
        let r = container(t(label, 12.0, if on { p.ink } else { p.text }).wrapping(Wrapping::None))
            .padding(Padding { top: 6.0, right: 10.0, bottom: 6.0, left: 10.0 })
            .width(Length::Fill)
            .style(ui::boxed(if on { p.accent } else { Color::TRANSPARENT }, p.r(6.0), None));
        c = c.push(mouse_area(r).on_press(m).interaction(mouse::Interaction::Pointer));
    }
    container(c).padding(4.0).width(if w > 0.0 { Length::Fixed(w) } else { Length::Shrink }).style(ui::boxed(p.panel2, p.r(8.0), Some((p.line, 1.0)))).into()
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Sz {
    Sm,
    Md,
    Lg,
}

/// .chip: a button as the boards draw them (sm 28 px, 36 px, lg 44 px; acc in the accent).
pub fn chip<'a, M: Clone + 'a>(p: &Pal, label: impl Into<String>, acc: bool, sz: Sz, msg: Option<M>) -> E<'a, M> {
    chip_x(p, label, acc, false, sz, msg)
}
/// .chip.ghost: the same with no fill.
pub fn ghost<'a, M: Clone + 'a>(p: &Pal, label: impl Into<String>, sz: Sz, msg: Option<M>) -> E<'a, M> {
    chip_x(p, label, false, true, sz, msg)
}
pub fn chip_x<'a, M: Clone + 'a>(p: &Pal, label: impl Into<String>, acc: bool, ghost: bool, sz: Sz, msg: Option<M>) -> E<'a, M> {
    let (h, f, pad, r) = match sz {
        Sz::Sm => (28.0, 12.0, 10.0, 8.0),
        Sz::Md => (36.0, 14.0, 14.0, 10.0),
        Sz::Lg => (44.0, 15.0, 20.0, 12.0),
    };
    let off = msg.is_none();
    let fg = if acc { p.ink } else if off { p.muted } else { p.text };
    let txt = if acc { tb(label.into(), f, fg) } else { t(label.into(), f, fg) };
    let bg = if acc { p.accent } else if ghost { Color::TRANSPARENT } else { p.panel2 };
    let c = container(txt.wrapping(Wrapping::None)).padding(Padding { top: 0.0, right: pad, bottom: 0.0, left: pad }).center_y(Length::Fixed(h)).style(ui::boxed(bg, p.r(r), Some((if acc { p.accent } else { p.line }, 1.0))));
    press(c.into(), msg)
}
/// A chip of the full width (Make 4 pictures).
pub fn wide<'a, M: Clone + 'a>(p: &Pal, label: impl Into<String>, msg: Option<M>) -> E<'a, M> {
    let on = msg.is_some();
    let c = container(tb(label.into(), 15.0, if on { p.ink } else { p.muted }).wrapping(Wrapping::None))
        .center_x(Length::Fill)
        .center_y(Length::Fixed(44.0))
        .style(ui::boxed(if on { p.accent } else { p.line }, p.r(12.0), None));
    press(c.into(), msg)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tg {
    Acc,
    Ok,
    Bl,
    Warn,
    Bad,
    Dk,
    Live,
}

/// .tag: a small label in capitals (SYNCED, RUNNING, ALL QUIET, ● LIVE).
pub fn tag<'a, M: 'a>(p: &Pal, label: impl Into<String>, k: Tg) -> E<'a, M> {
    let (bg, fg, line) = match k {
        Tg::Acc => (p.accent, p.ink, p.accent),
        Tg::Ok => (Pal::mix(p.panel, green(), 0.16), green(), Pal::mix(p.panel, green(), 0.35)),
        Tg::Bl => (Pal::mix(p.panel, blue(), 0.16), blue(), Pal::mix(p.panel, blue(), 0.35)),
        Tg::Warn => (Pal::mix(p.panel, yellow(), 0.14), yellow(), Pal::mix(p.panel, yellow(), 0.32)),
        Tg::Bad => (Pal::mix(p.panel, red(), 0.16), red(), Pal::mix(p.panel, red(), 0.35)),
        Tg::Dk => (p.panel2, p.muted, p.line),
        Tg::Live => (Color::from_rgb8(0xff, 0x4a, 0x4a), Color::WHITE, Color::from_rgb8(0xff, 0x4a, 0x4a)),
    };
    container(tb(label.into().to_uppercase(), 11.0, fg).wrapping(Wrapping::None))
        .padding(Padding { top: 3.0, right: 8.0, bottom: 3.0, left: 8.0 })
        .style(ui::boxed(bg, 6.0, Some((line, 1.0))))
        .into()
}

/// .mark: Hearth's own letter mark in one of the boards' colours (22 px, mid 44, big 64).
pub fn mark<'a, M: 'a>(letters: &str, g: &str, size: f32) -> E<'a, M> {
    let (a, b) = grad(g);
    let lin = iced::gradient::Linear::new(iced::Radians(2.356)).add_stop(0.0, a).add_stop(1.0, b);
    let r = if size >= 60.0 { 16.0 } else if size >= 40.0 { 12.0 } else { size * 0.32 };
    container(tb(letters.to_string(), (size * 0.42).max(8.0), Color::WHITE).wrapping(Wrapping::None))
        .center_x(Length::Fixed(size))
        .center_y(Length::Fixed(size))
        .style(move |_: &Theme| container::Style {
            background: Some(Background::Gradient(iced::Gradient::Linear(lin.clone()))),
            border: Border { color: Color::from_rgba(1.0, 1.0, 1.0, 0.13), width: 1.0, radius: r.into() },
            ..container::Style::default()
        })
        .into()
}
/// A coloured square (.fi, a light's swatch, .ic).
pub fn swatch<'a, M: 'a>(c: Color, w: f32, h: f32, r: f32) -> E<'a, M> {
    container(Space::new()).width(Length::Fixed(w)).height(Length::Fixed(h)).style(ui::boxed(c, r, None)).into()
}
pub fn swatch_g<'a, M: 'a>(g: &str, w: f32, h: f32, r: f32) -> E<'a, M> {
    let (a, b) = grad(g);
    let lin = iced::gradient::Linear::new(iced::Radians(2.356)).add_stop(0.0, a).add_stop(1.0, b);
    container(Space::new())
        .width(Length::Fixed(w))
        .height(Length::Fixed(h))
        .style(move |_: &Theme| container::Style { background: Some(Background::Gradient(iced::Gradient::Linear(lin.clone()))), border: Border { radius: r.into(), ..Border::default() }, ..container::Style::default() })
        .into()
}
/// A dot (.ic.r).
pub fn dot<'a, M: 'a>(c: Color, d: f32) -> E<'a, M> {
    swatch(c, d, d, d / 2.0)
}

/// .cap: a section label in small capitals.
pub fn cap<'a, M: 'a>(p: &Pal, s: impl Into<String>) -> E<'a, M> {
    container(tb(s.into().to_uppercase(), 11.0, p.muted)).padding(Padding { top: 14.0, right: 0.0, bottom: 8.0, left: 0.0 }).into()
}
pub fn h1<'a, M: 'a>(p: &Pal, s: impl Into<String>) -> E<'a, M> {
    tb(s.into(), 26.0, p.text).into()
}
pub fn h2<'a, M: 'a>(p: &Pal, s: impl Into<String>) -> E<'a, M> {
    tb(s.into(), 18.0, p.text).into()
}
pub fn mu<'a, M: 'a>(p: &Pal, s: impl Into<String>, size: f32) -> E<'a, M> {
    t(s.into(), size, p.muted).into()
}

/// .row: a label on the left and its control on the right.
pub fn kv<'a, M: 'a>(p: &Pal, label: impl Into<String>, control: E<'a, M>) -> E<'a, M> {
    container(row![t(label.into(), 13.0, p.text), Space::new().width(Length::Fill), control].spacing(10.0).align_y(iced::Center)).padding(Padding { top: 4.0, right: 0.0, bottom: 4.0, left: 0.0 }).into()
}

/// .nav: a side row (its colour square, its name, its count), lit when it's the one open.
pub fn nav<'a, M: Clone + 'a>(p: &Pal, ic: Option<Color>, label: impl Into<String>, count: impl Into<String>, on: bool, msg: M) -> E<'a, M> {
    let icc = ic.unwrap_or(if on { p.text } else { p.muted });
    let r = row![swatch(Pal::a(icc, 0.85), 16.0, 16.0, 4.0), t(label.into(), 13.0, if on { p.text } else { p.muted }).wrapping(Wrapping::None), Space::new().width(Length::Fill), t(count.into(), 11.0, p.muted).wrapping(Wrapping::None)]
        .spacing(10.0)
        .align_y(iced::Center);
    let inner = container(r).padding(Padding { top: 0.0, right: 10.0, bottom: 0.0, left: 10.0 }).center_y(Length::Fixed(36.0)).width(Length::Fill);
    let lit = lit_row(p, inner.into(), on, 9.0, 36.0);
    mouse_area(lit).on_press(msg).interaction(mouse::Interaction::Pointer).into()
}
/// The boards' lit row: a warm fill and the accent line on its left (.nav.on, .rc.on, .en.on).
pub fn lit_row<'a, M: 'a>(p: &Pal, inner: E<'a, M>, on: bool, r: f32, h: f32) -> E<'a, M> {
    let bg = if on { Pal::mix(p.panel, p.accent, 0.16) } else { Color::TRANSPARENT };
    let bar = container(Space::new()).width(Length::Fixed(3.0)).height(Length::Fixed(h)).style(ui::boxed(if on { p.accent } else { Color::TRANSPARENT }, 0.0, None));
    container(row![bar, inner].align_y(iced::Center)).clip(true).width(Length::Fill).style(ui::boxed(bg, p.r(r), None)).into()
}

/// .search: a box to type in, ⌕ before it.
pub fn search<'a, M: Clone + 'a>(p: &Pal, placeholder: &str, value: &str, on: impl Fn(String) -> M + 'a) -> E<'a, M> {
    let p = *p;
    let inp = text_input(placeholder, value).on_input(on).size(13.0).padding(Padding { top: 9.0, right: 12.0, bottom: 9.0, left: 4.0 }).style(input_style(p, false));
    container(row![t("⌕", 13.0, p.muted), inp].spacing(4.0).align_y(iced::Center))
        .padding(Padding { top: 0.0, right: 0.0, bottom: 0.0, left: 12.0 })
        .style(ui::boxed(p.bg, p.r(10.0), Some((p.line, 1.0))))
        .into()
}
/// A text box's look: borderless inside a .search, or the boards' field.
pub fn input_style(p: Pal, framed: bool) -> impl Fn(&Theme, text_input::Status) -> text_input::Style {
    move |_t, s| text_input::Style {
        background: Background::Color(if framed { p.bg } else { Color::TRANSPARENT }),
        border: Border { color: if framed { if matches!(s, text_input::Status::Focused { .. }) { p.accent } else { p.line } } else { Color::TRANSPARENT }, width: if framed { 1.0 } else { 0.0 }, radius: p.r(8.0).into() },
        icon: p.muted,
        placeholder: p.muted,
        value: p.text,
        selection: Pal::a(p.accent, 0.35),
    }
}
/// A text box in the boards' field look.
pub fn input<'a, M: Clone + 'a>(p: &Pal, placeholder: &str, value: &str, size: f32, on: impl Fn(String) -> M + 'a) -> text_input::TextInput<'a, M> {
    text_input(placeholder, value).on_input(on).size(size).padding(Padding { top: 8.0, right: 10.0, bottom: 8.0, left: 10.0 }).style(input_style(*p, true))
}
/// A password box: dots, never shown, never kept by the app beyond what it needs.
pub fn secret_input<'a, M: Clone + 'a>(p: &Pal, placeholder: &str, value: &str, on: impl Fn(String) -> M + 'a, submit: M) -> text_input::TextInput<'a, M> {
    input(p, placeholder, value, 14.0, on).secure(true).on_submit(submit)
}

/// .card: the panel, its line, rounded 12.
pub fn card<'a, M: 'a>(p: &Pal, inner: impl Into<E<'a, M>>) -> container::Container<'a, M> {
    container(inner).padding(16.0).style(ui::boxed(p.panel, p.r(12.0), Some((p.line, 1.0))))
}
/// A card inside a card (panel2).
pub fn card2<'a, M: 'a>(p: &Pal, inner: impl Into<E<'a, M>>) -> container::Container<'a, M> {
    container(inner).padding(Padding { top: 12.0, right: 14.0, bottom: 12.0, left: 14.0 }).style(ui::boxed(p.panel2, p.r(12.0), Some((p.line, 1.0))))
}
/// The side column's tint (.side).
pub fn side_bg(p: &Pal) -> impl Fn(&Theme) -> container::Style {
    ui::boxed(Pal::mix(p.bg, Color::BLACK, 0.18), 0.0, None)
}

/// .bar: how much of something (0..1), in the accent or a colour of its own.
pub fn meter<'a, M: 'a>(p: &Pal, frac: f32, c: Option<Color>, w: f32, h: f32) -> E<'a, M> {
    container(ui::bar(vec![(frac.clamp(0.0, 1.0), c.unwrap_or(p.accent))], p.line, h, h / 2.0)).width(if w > 0.0 { Length::Fixed(w) } else { Length::Fill }).into()
}

/// .sl's look for iced's slider: the accent rail up to the white knob.
pub fn slider_style(p: &Pal, k: f32) -> impl Fn(&Theme, iced::widget::slider::Status) -> iced::widget::slider::Style {
    use iced::widget::slider;
    let p = *p;
    move |_, _| slider::Style {
        rail: slider::Rail { backgrounds: (Background::Color(p.accent), Background::Color(p.line)), width: 4.0 * k, border: Border { color: Color::TRANSPARENT, width: 0.0, radius: (2.0 * k).into() } },
        handle: slider::Handle { shape: slider::HandleShape::Circle { radius: 8.0 * k }, background: Background::Color(Color::WHITE), border_width: 0.0, border_color: Color::TRANSPARENT },
    }
}

/// The credit every board ends with: "Hearth X · Built on PROJECT (LICENCE)".
pub fn credit<'a, M: 'a>(p: &Pal, app: &str, built_on: &str) -> E<'a, M> {
    container(row![t(format!("{app} · "), 11.0, p.muted), tb(built_on.to_string(), 11.0, Pal::mix(p.muted, p.text, 0.4))].align_y(iced::Center))
        .padding(Padding { top: 6.0, right: 18.0, bottom: 6.0, left: 18.0 })
        .width(Length::Fill)
        .align_x(iced::alignment::Horizontal::Right)
        .into()
}

/// A small square button with one glyph or sign (.icb: ⧉ copy, ◉ show, ↗ open).
pub fn icb<'a, M: Clone + 'a>(p: &Pal, s: &str, msg: Option<M>) -> E<'a, M> {
    let c = container(t(s.to_string(), 13.0, p.muted)).center_x(Length::Fixed(30.0)).center_y(Length::Fixed(30.0)).style(ui::boxed(p.panel2, p.r(8.0), Some((p.line, 1.0))));
    press(c.into(), msg)
}
pub fn icb_g<'a, M: Clone + 'a>(p: &Pal, g: G, msg: Option<M>) -> E<'a, M> {
    let c = container(glyph(g, p.muted, 13.0, 1.8)).center_x(Length::Fixed(30.0)).center_y(Length::Fixed(30.0)).style(ui::boxed(p.panel2, p.r(8.0), Some((p.line, 1.0))));
    press(c.into(), msg)
}

/// A line of muted words with a strong part first (Ties to Voice Studio, Your server).
pub fn titled<'a, M: 'a>(p: &Pal, title: impl Into<String>, sub: impl Into<String>) -> E<'a, M> {
    iced::widget::column![tm(title.into(), 13.0, p.text), t(sub.into(), 11.0, p.muted)].spacing(3.0).into()
}

// ------------------------------------------------------------------ graphs drawn on a canvas

/// Bars: a waveform (Listen) or the day's energy (Home); the playhead (0..1) in the accent when there is one.
pub struct Bars {
    /// each 0..1
    pub vals: Vec<f32>,
    pub color: Color,
    pub head: Option<f32>,
    pub head_color: Color,
    /// bars over this are drawn full, the rest at 75 % (the boards' look)
    pub bright_over: f32,
    /// the bars stand on the bottom (energy), or are centred (a waveform)
    pub centred: bool,
}

impl<M> canvas::Program<M> for Bars {
    type State = ();
    fn draw(&self, _s: &(), r: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        let mut f = Frame::new(r, b.size());
        let n = self.vals.len().max(1) as f32;
        let bw = b.width / n;
        for (i, v) in self.vals.iter().enumerate() {
            let v = v.clamp(0.02, 1.0);
            let h = v * b.height;
            let x = i as f32 * bw + 1.0;
            let y = if self.centred { (b.height - h) / 2.0 } else { b.height - h };
            let c = if v > self.bright_over { self.color } else { Pal::a(self.color, 0.75) };
            f.fill(&Path::rounded_rectangle(Point::new(x, y), iced::Size::new((bw - 2.0).max(1.0), h), iced::border::Radius::from(1.5f32)), c);
        }
        if let Some(hd) = self.head {
            let x = (hd.clamp(0.0, 1.0) * b.width).round();
            f.fill(&Path::rectangle(Point::new(x - 4.0, 0.0), iced::Size::new(8.0, b.height)), Pal::a(self.head_color, 0.18));
            f.fill(&Path::rectangle(Point::new(x - 1.0, 0.0), iced::Size::new(2.0, b.height)), self.head_color);
        }
        vec![f.into_geometry()]
    }
}

/// A ring: how much of a countdown is left (a 2-step code's 30 s), or a dial's arc.
pub struct Ring {
    pub frac: f32,
    pub color: Color,
    pub track: Color,
    pub width: f32,
}

impl<M> canvas::Program<M> for Ring {
    type State = ();
    fn draw(&self, _s: &(), r: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        let mut f = Frame::new(r, b.size());
        let c = Point::new(b.width / 2.0, b.height / 2.0);
        let rad = b.width.min(b.height) / 2.0 - self.width;
        f.stroke(&Path::circle(c, rad), Stroke::default().with_color(self.track).with_width(self.width));
        let a0 = -std::f32::consts::FRAC_PI_2;
        let a1 = a0 + std::f32::consts::TAU * self.frac.clamp(0.0, 1.0);
        let arc = Path::new(|pb| {
            pb.arc(canvas::path::Arc { center: c, radius: rad, start_angle: iced::Radians(a0), end_angle: iced::Radians(a1) });
        });
        f.stroke(&arc, Stroke::default().with_color(self.color).with_width(self.width).with_line_cap(canvas::LineCap::Round));
        vec![f.into_geometry()]
    }
}

/// A shadow under a floating card (the password maker, the captions).
pub fn floating(p: &Pal, border: Color) -> impl Fn(&Theme) -> container::Style {
    let p = *p;
    move |_| container::Style {
        background: Some(Background::Color(Pal::mix(p.bg, p.panel, 0.6))),
        border: Border { color: border, width: 1.0, radius: p.r(14.0).into() },
        shadow: Shadow { color: Color::from_rgba(0.0, 0.0, 0.0, 0.6), offset: iced::Vector::new(0.0, 18.0), blur_radius: 50.0 },
        ..container::Style::default()
    }
}
