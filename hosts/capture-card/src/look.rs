//! The look, exactly as the approved mockups draw it (docs/mockups/capture-card, viewer.html's CSS): Hearth's dark
//! ember tokens, the same as Console Remote Play's (this file started as its look.rs), so the two apps feel like
//! siblings. The colours are the mockups' own; Inter for text, DejaVu Sans Mono for folders. The icons are the
//! mockups' SVG symbols (g-card, g-live, …), drawn by a small path reader (no Microsoft or Xbox marks anywhere).

use hearth_ui::app::INTER;
use iced::widget::canvas::{self, Frame, Geometry, LineCap, LineJoin, Path, Stroke};
use iced::widget::{column, container, mouse_area, row, text, Column, Row, Space};
use iced::{font, mouse, Background, Border, Color, Element, Font, Length, Padding, Point, Rectangle, Renderer, Shadow, Theme, Vector};

pub type E<'a, M> = Element<'a, M>;

const fn c(r: u8, g: u8, b: u8) -> Color {
    Color { r: r as f32 / 255.0, g: g as f32 / 255.0, b: b as f32 / 255.0, a: 1.0 }
}
pub const BG: Color = c(0x15, 0x17, 0x1c);

pub const SIDE: Color = c(0x1b, 0x1e, 0x24);
pub const CARD: Color = c(0x20, 0x24, 0x2b);
pub const CARD2: Color = c(0x27, 0x2b, 0x33);
pub const LINE: Color = c(0x2f, 0x34, 0x3d);
pub const TX: Color = c(0xe9, 0xeb, 0xef);
pub const MUT: Color = c(0x9a, 0xa1, 0xad);
pub const ACC: Color = c(0xff, 0x7a, 0x2f);
pub const ACC2: Color = c(0xff, 0x9a, 0x52);
pub const OK: Color = c(0x4c, 0xcf, 0x7a);
pub const WARN: Color = c(0xf2, 0xc1, 0x4e);
pub const BAD: Color = c(0xff, 0x5d, 0x5d);
pub const INK: Color = c(0x1b, 0x10, 0x06);
pub const SLEEP: Color = c(0x6a, 0xa8, 0xff);
pub const OFF: Color = c(0x6f, 0x76, 0x82);
pub const EDGE: Color = c(0x3c, 0x41, 0x4c);
pub const SOFT: Color = c(0xdf, 0xe2, 0xe7);
pub const SOFT2: Color = c(0xcf, 0xd3, 0xda);
pub const DEEP: Color = c(0x0d, 0x0e, 0x11);
pub const QUIET: Color = c(0x17, 0x19, 0x1e);
pub const FOCUSROW: Color = c(0x2a, 0x2f, 0x38);
pub const MONO: Font = Font::with_name("DejaVu Sans Mono");

pub fn a(col: Color, alpha: f32) -> Color {
    Color { a: alpha, ..col }
}

pub fn w(wt: font::Weight) -> Font {
    Font { weight: wt, ..INTER }
}

/// Text: size, colour, weight (400 / 500 / 600 / 700).
pub fn t<'a>(s: impl text::IntoFragment<'a>, size: f32, col: Color) -> text::Text<'a> {
    text(s).size(size).color(col).font(INTER)
}
pub fn t6<'a>(s: impl text::IntoFragment<'a>, size: f32, col: Color) -> text::Text<'a> {
    text(s).size(size).color(col).font(w(font::Weight::Semibold))
}
pub fn t7<'a>(s: impl text::IntoFragment<'a>, size: f32, col: Color) -> text::Text<'a> {
    text(s).size(size).color(col).font(w(font::Weight::Bold))
}
pub fn one<'a>(tx: text::Text<'a>) -> text::Text<'a> {
    tx.wrapping(text::Wrapping::None)
}

/// `rich` on one line, never wrapped (a short line that must not be measured narrower than it is).
pub fn rich1<'a, M: 'a>(s: &str, size: f32, col: Color, bold: Color) -> E<'a, M> {
    let mut spans: Vec<iced::widget::text::Span<'a, (), Font>> = Vec::new();
    for (i, part) in s.split("**").enumerate() {
        if !part.is_empty() {
            spans.push(iced::widget::span(part.to_string()).size(size).color(if i % 2 == 1 { bold } else { col }).font(if i % 2 == 1 { w(font::Weight::Semibold) } else { INTER }));
        }
    }
    iced::widget::rich_text(spans).line_height(1.4).wrapping(text::Wrapping::None).into()
}

/// `rich`, as wide as its place (so it wraps there).
pub fn rich_w<'a, M: 'a>(s: &str, size: f32, col: Color, bold: Color) -> E<'a, M> {
    let mut spans: Vec<iced::widget::text::Span<'a, (), Font>> = Vec::new();
    for (i, part) in s.split("**").enumerate() {
        if !part.is_empty() {
            spans.push(iced::widget::span(part.to_string()).size(size).color(if i % 2 == 1 { bold } else { col }).font(if i % 2 == 1 { w(font::Weight::Semibold) } else { INTER }));
        }
    }
    iced::widget::rich_text(spans).line_height(1.4).width(Length::Fill).into()
}

/// Rich text: plain and **bold** parts ("Hold **View + Menu** for 1 second").
pub fn rich<'a, M: 'a>(s: &str, size: f32, col: Color, bold: Color, center: bool) -> E<'a, M> {
    let mut spans: Vec<iced::widget::text::Span<'a, (), Font>> = Vec::new();
    for (i, part) in s.split("**").enumerate() {
        if part.is_empty() {
            continue;
        }
        let sp = iced::widget::span(part.to_string()).size(size).color(if i % 2 == 1 { bold } else { col }).font(if i % 2 == 1 { w(font::Weight::Semibold) } else { INTER });
        spans.push(sp);
    }
    let r = iced::widget::rich_text(spans).line_height(1.4);
    if center {
        r.align_x(iced::Center).width(Length::Fill).into()
    } else {
        r.into()
    }
}

pub fn pad(tp: f32, r: f32, b: f32, l: f32) -> Padding {
    Padding { top: tp, right: r, bottom: b, left: l }
}

pub fn bx(bg: Color, r: f32, line: Option<(Color, f32)>) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        text_color: None,
        background: Some(Background::Color(bg)),
        border: match line {
            Some((col, wd)) => Border { color: col, width: wd, radius: r.into() },
            None => iced::border::rounded(r),
        },
        shadow: Shadow::default(),
        snap: true,
    }
}

pub fn shadowed(bg: Color, r: f32, line: Option<(Color, f32)>, blur: f32) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        text_color: None,
        background: Some(Background::Color(bg)),
        border: match line {
            Some((col, wd)) => Border { color: col, width: wd, radius: r.into() },
            None => iced::border::rounded(r),
        },
        shadow: Shadow { color: a(Color::BLACK, 0.7), offset: Vector::new(0.0, blur * 0.3), blur_radius: blur },
        snap: true,
    }
}

pub fn grad(deg: f32, from: Color, to: Color, r: f32) -> impl Fn(&Theme) -> container::Style {
    move |_| {
        let g = iced::gradient::Linear::new(iced::Radians((deg - 90.0).to_radians())).add_stop(0.0, from).add_stop(1.0, to);
        container::Style { text_color: None, background: Some(Background::Gradient(g.into())), border: iced::border::rounded(r), shadow: Shadow::default(), snap: true }
    }
}


pub fn gap<'a, M: 'a>(wd: f32, h: f32) -> E<'a, M> {
    Space::new().width(Length::Fixed(wd)).height(Length::Fixed(h)).into()
}
pub fn fill<'a, M: 'a>() -> E<'a, M> {
    Space::new().width(Length::Fill).into()
}
pub fn vfill<'a, M: 'a>() -> E<'a, M> {
    Space::new().height(Length::Fill).into()
}
pub fn hline<'a, M: 'a>(col: Color) -> E<'a, M> {
    container(Space::new()).width(Length::Fill).height(Length::Fixed(1.0)).style(bx(col, 0.0, None)).into()
}
pub fn vline<'a, M: 'a>(col: Color, h: f32) -> E<'a, M> {
    container(Space::new()).width(Length::Fixed(1.0)).height(Length::Fixed(h)).style(bx(col, 0.0, None)).into()
}

pub fn click<'a, M: Clone + 'a>(e: impl Into<E<'a, M>>, m: Option<M>) -> E<'a, M> {
    match m {
        Some(m) => mouse_area(e).on_press(m).interaction(mouse::Interaction::Pointer).into(),
        None => e.into(),
    }
}

/// The focus ring (`.focus`: a 3 px orange outline, 3 px outside the control), drawn around the control without
/// taking any room, so rows keep the height the mockups give them and nothing moves with the focus.
pub fn ring<'a, M: 'a>(on: bool, r: f32, inner: impl Into<E<'a, M>>) -> E<'a, M> {
    if !on {
        return inner.into();
    }
    Element::new(Outline { content: inner.into(), radius: r })
}

/// The ring itself: the control as it is, and the outline drawn above it, outside its bounds.
pub struct Outline<'a, M> {
    content: E<'a, M>,
    radius: f32,
}

mod outline {
    use super::{Outline, ACC};
    use iced::advanced::layout::{self, Layout};
    use iced::advanced::renderer::{self, Renderer as _};
    use iced::advanced::widget::operation::Operation;
    use iced::advanced::widget::{tree, Tree};
    use iced::advanced::{overlay, Clipboard, Shell, Widget};
    use iced::{mouse, Border, Color, Event, Length, Rectangle, Size, Vector};

    /// outline-offset 3 px, outline 3 px.
    const OFFSET: f32 = 3.0;
    const WIDTH: f32 = 3.0;

    impl<'a, M: 'a> Widget<M, iced::Theme, iced::Renderer> for Outline<'a, M> {
        fn size(&self) -> Size<Length> {
            self.content.as_widget().size()
        }
        fn size_hint(&self) -> Size<Length> {
            self.content.as_widget().size_hint()
        }
        fn layout(&mut self, tree: &mut Tree, renderer: &iced::Renderer, limits: &layout::Limits) -> layout::Node {
            self.content.as_widget_mut().layout(&mut tree.children[0], renderer, limits)
        }
        fn draw(&self, tree: &Tree, renderer: &mut iced::Renderer, theme: &iced::Theme, style: &renderer::Style, layout: Layout<'_>, cursor: mouse::Cursor, viewport: &Rectangle) {
            self.content.as_widget().draw(&tree.children[0], renderer, theme, style, layout, cursor, viewport);
            let b = layout.bounds();
            let e = OFFSET + WIDTH;
            let ring = Rectangle { x: b.x - e, y: b.y - e, width: b.width + 2.0 * e, height: b.height + 2.0 * e };
            let r = self.radius + OFFSET + WIDTH / 2.0;
            renderer.with_layer(ring, |rd| {
                rd.fill_quad(renderer::Quad { bounds: ring, border: Border { color: ACC, width: WIDTH, radius: r.into() }, shadow: iced::Shadow::default(), snap: true }, Color::TRANSPARENT);
            });
        }
        fn tag(&self) -> tree::Tag {
            tree::Tag::stateless()
        }
        fn children(&self) -> Vec<Tree> {
            vec![Tree::new(&self.content)]
        }
        fn diff(&self, tree: &mut Tree) {
            tree.diff_children(std::slice::from_ref(&self.content));
        }
        fn operate(&mut self, tree: &mut Tree, layout: Layout<'_>, renderer: &iced::Renderer, operation: &mut dyn Operation) {
            self.content.as_widget_mut().operate(&mut tree.children[0], layout, renderer, operation)
        }
        fn update(&mut self, tree: &mut Tree, event: &Event, layout: Layout<'_>, cursor: mouse::Cursor, renderer: &iced::Renderer, clipboard: &mut dyn Clipboard, shell: &mut Shell<'_, M>, viewport: &Rectangle) {
            self.content.as_widget_mut().update(&mut tree.children[0], event, layout, cursor, renderer, clipboard, shell, viewport)
        }
        fn mouse_interaction(&self, tree: &Tree, layout: Layout<'_>, cursor: mouse::Cursor, viewport: &Rectangle, renderer: &iced::Renderer) -> mouse::Interaction {
            self.content.as_widget().mouse_interaction(&tree.children[0], layout, cursor, viewport, renderer)
        }
        fn overlay<'b>(&'b mut self, tree: &'b mut Tree, layout: Layout<'b>, renderer: &iced::Renderer, viewport: &Rectangle, translation: Vector) -> Option<overlay::Element<'b, M, iced::Theme, iced::Renderer>> {
            self.content.as_widget_mut().overlay(&mut tree.children[0], layout, renderer, viewport, translation)
        }
    }
}

/// A ring drawn on the element's own edge (`.focus` with outline-offset 0: the console card, tiles).
pub fn ring0<'a, M: 'a>(on: bool, r: f32, bg: Color, inner: impl Into<E<'a, M>>) -> container::Container<'a, M> {
    container(inner).style(move |_: &Theme| container::Style {
        background: Some(Background::Color(bg)),
        border: Border { color: if on { ACC } else { Color::TRANSPARENT }, width: 3.0, radius: r.into() },
        ..container::Style::default()
    })
}

// ------------------------------------------------------------------ buttons, chips, switches
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum B {
    Normal,
    Primary,
    Danger,
}

/// `.btn`, `.btn.p`, `.btn.d`; big = `.btn.big`; an icon before the label.
pub fn btn<'a, M: Clone + 'a>(label: &str, icon: Option<G>, k: B, big: bool, size: f32, focused: bool, on: Option<M>) -> E<'a, M> {
    let (bg, line, fg) = match k {
        B::Normal => (Color::TRANSPARENT, EDGE, SOFT),
        B::Primary => (ACC, ACC, INK),
        B::Danger => (c(0x3a, 0x1d, 0x1d), c(0x7a, 0x33, 0x33), c(0xff, 0xb3, 0xb3)),
    };
    let lbl = if k == B::Normal { one(t(label.to_string(), size, fg)) } else { one(t7(label.to_string(), size, fg)) };
    let mut r = Row::new().spacing(8.0).align_y(iced::Center);
    if let Some(g) = icon {
        r = r.push(glyph(g, fg, if big { 20.0 } else { 18.0 }));
    }
    r = r.push(lbl);
    let (pv, ph, rad) = if big { (14.0, 26.0, 12.0) } else { (9.0, 18.0, 9.0) };
    let b = container(r).padding(pad(pv, ph, pv, ph)).style(bx(bg, rad, Some((line, 1.0))));
    ring(focused, rad, click(b, on))
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Ch {
    Plain,
    Ok,
    On,
    Bad,
    Sleep,
    Warn,
}

/// `.chip` and its kinds, with an optional status dot.
pub fn chip<'a, M: 'a>(label: &str, k: Ch, dot: Option<Color>, size: f32) -> E<'a, M> {
    let (line, fg) = match k {
        Ch::Plain => (EDGE, SOFT2),
        Ch::Ok => (c(0x2f, 0x6b, 0x45), c(0x7f, 0xe0, 0xa3)),
        Ch::On => (ACC, ACC2),
        Ch::Bad => (c(0x7a, 0x33, 0x33), c(0xff, 0xb3, 0xb3)),
        Ch::Sleep => (c(0x2f, 0x4f, 0x7a), c(0xa9, 0xcb, 0xff)),
        Ch::Warn => (c(0x7a, 0x66, 0x33), c(0xff, 0xe0, 0x8a)),
    };
    let mut r = Row::new().align_y(iced::Center);
    if let Some(d) = dot {
        r = r.push(dotted(d, 9.0)).push(gap(7.0, 1.0));
    }
    r = r.push(one(t(label.to_string(), size, fg)));
    container(r).padding(pad(4.0, 12.0, 4.0, 12.0)).style(bx(Color::TRANSPARENT, 20.0, Some((line, 1.0)))).into()
}

/// A chip of a fixed width with its label centred (the status legend).
pub fn chip_w<'a, M: 'a>(label: &str, k: Ch, dot: Option<Color>, width: f32) -> E<'a, M> {
    container(chip(label, k, dot, 13.5)).center_x(Length::Fixed(width)).into()
}

pub fn dotted<'a, M: 'a>(col: Color, size: f32) -> E<'a, M> {
    container(Space::new()).width(Length::Fixed(size)).height(Length::Fixed(size)).style(bx(col, size / 2.0, None)).into()
}

/// `.tog`: 54 × 30, the knob on the right in ink when on.
pub fn toggle<'a, M: Clone + 'a>(on: bool, scale: f32, focused: bool, m: Option<M>) -> E<'a, M> {
    let k = 20.0 * scale;
    let knob = container(Space::new()).width(Length::Fixed(k)).height(Length::Fixed(k)).style(bx(if on { INK } else { TX }, k / 2.0, None));
    let inner: E<'a, M> = if on { row![fill(), knob].into() } else { row![knob, fill()].into() };
    let track = container(inner).padding(5.0 * scale).width(Length::Fixed(54.0 * scale)).height(Length::Fixed(30.0 * scale)).style(bx(if on { ACC } else { c(0x35, 0x3a, 0x44) }, 15.0 * scale, None));
    ring(focused, 15.0 * scale, click(track, m))
}

/// `.seg`: the choices side by side, the picked one orange.
pub fn seg<'a, M: Clone + 'a>(items: Vec<(String, bool, M)>, size: f32, padv: f32, padh: f32, focused: bool) -> E<'a, M> {
    let mut r = Row::new();
    for (l, on, m) in items {
        let lbl = if on { one(t7(l, size, INK)) } else { one(t(l, size, c(0xc3, 0xc7, 0xcf))) };
        let cell = container(lbl).padding(pad(padv, padh, padv, padh)).style(bx(if on { ACC } else { Color::TRANSPARENT }, 8.0, None));
        r = r.push(click(cell, Some(m)));
    }
    ring(focused, 10.0, container(r).padding(4.0).style(bx(c(0x1a, 0x1d, 0x23), 10.0, None)))
}

/// `.dd`: a drop-down's face (it opens a chooser).
pub fn dd<'a, M: Clone + 'a>(label: &str, width: f32, focused: bool, m: M) -> E<'a, M> {
    let r = row![one(t(label.to_string(), 15.0, TX)), fill(), one(t("▾", 15.0, MUT))].spacing(10.0).align_y(iced::Center);
    ring(focused, 9.0, click(container(r).padding(pad(9.0, 14.0, 9.0, 14.0)).width(Length::Fixed(width)).style(bx(BG, 9.0, Some((EDGE, 1.0)))), Some(m)))
}

/// `.ri`: a row's icon in its box.
pub fn ri<'a, M: 'a>(g: G, size: f32) -> E<'a, M> {
    container(glyph(g, SOFT2, 20.0)).center_x(Length::Fixed(size)).center_y(Length::Fixed(size)).style(bx(CARD2, 10.0, None)).into()
}

/// `.row`: an icon, a title and a line, and what's on its right.
pub fn srow<'a, M: Clone + 'a>(icon: Option<E<'a, M>>, title: &str, sub: Option<E<'a, M>>, end: Option<E<'a, M>>, title_size: f32, bg: Color, padding: Padding, focused: bool) -> E<'a, M> {
    let mut txt = Column::new().spacing(3.0).push(t(title.to_string(), title_size, TX));
    if let Some(s) = sub {
        txt = txt.push(s);
    }
    let mut r = Row::new().spacing(16.0).align_y(iced::Center);
    if let Some(i) = icon {
        r = r.push(i);
    }
    r = r.push(txt.width(Length::Fill));
    if let Some(e) = end {
        r = r.push(e);
    }
    ring(focused, 12.0, container(r).padding(padding).width(Length::Fill).style(bx(bg, 12.0, None)))
}

/// `.sec`: the section's title, in capitals.
pub fn sec<'a, M: 'a>(s: &str, top: f32) -> E<'a, M> {
    container(one(t6(s.to_uppercase(), 14.0, MUT))).padding(pad(top, 0.0, 10.0, 0.0)).into()
}

/// The app's tile: Hearth's ember gradient with the capture card.
pub fn app_icon<'a, M: 'a>(size: f32, glyph_size: f32, radius: f32) -> E<'a, M> {
    container(glyph(G::Card, INK, glyph_size)).center_x(Length::Fixed(size)).center_y(Length::Fixed(size)).style(grad(135.0, c(0xff, 0x8a, 0x3d), c(0xd9, 0x53, 0x1e), radius)).into()
}

/// A drawing kept between frames, drawn again only when what it shows (or its size) changes.
pub struct Kept<K> {
    pub cache: canvas::Cache,
    key: std::cell::Cell<Option<K>>,
}

impl<K> Default for Kept<K> {
    fn default() -> Self {
        Kept { cache: canvas::Cache::new(), key: std::cell::Cell::new(None) }
    }
}

impl<K: Copy + PartialEq> Kept<K> {
    pub fn check(&self, k: K) {
        if self.key.get() != Some(k) {
            self.cache.clear();
            self.key.set(Some(k));
        }
    }
}

// ------------------------------------------------------------------ the controller hints
/// A button in a circle (`.hints b`) and what it does.
pub fn hint<'a, M: 'a>(keys: &[&str], what: &str, size: f32) -> E<'a, M> {
    let mut r = Row::new().align_y(iced::Center);
    for (i, k) in keys.iter().enumerate() {
        let wd = if k.chars().count() > 1 { (k.chars().count() as f32 * 8.0 + 14.0).max(30.0) } else { 28.0 };
        r = r.push(container(one(t7(k.to_string(), 12.0, c(0xc9, 0xcc, 0xd3)))).center_x(Length::Fixed(wd)).center_y(Length::Fixed(28.0)).style(bx(Color::TRANSPARENT, 14.0, Some((MUT, 2.0)))));
        r = r.push(gap(if i + 1 < keys.len() { 6.0 } else { 8.0 }, 1.0));
    }
    if !what.is_empty() {
        r = r.push(one(t(what.to_string(), size, c(0xc9, 0xcc, 0xd3))));
    }
    r.into()
}

/// The hints strip along the bottom (`.hints`).
pub fn hints<'a, M: 'a>(left: Vec<E<'a, M>>, right: Vec<E<'a, M>>, height: f32, spacing: f32, side: f32, bg: Color) -> E<'a, M> {
    let mut l = Row::new().spacing(spacing).align_y(iced::Center);
    for x in left {
        l = l.push(x);
    }
    let mut r = Row::new().spacing(spacing).align_y(iced::Center);
    for x in right {
        r = r.push(x);
    }
    column![hline(LINE), container(row![l, fill(), r].align_y(iced::Center)).padding(pad(0.0, side, 0.0, side)).center_y(Length::Fixed(height - 1.0)).width(Length::Fill).style(bx(bg, 0.0, None))].into()
}

// ------------------------------------------------------------------ icons (the mockups' SVG paths)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum G {
    Card,
    Live,
    List,
    Gear,
    Cam,
    Full,
    Unfull,
    Vol,
    Mute,
    Bolt,
    Usb,
    Shield,
    Tv,
    Folder,
    Mic,
    Sun,
    Film,
    Fit,
    Store,
    Exit,
    Play,
    X,
    Min,
    Max,
}

/// Each icon: paths (SVG d; a leading * fills it), rectangles (x, y, w, h, rx), circles (cx, cy, r, filled), as
/// viewer.html's symbols have them.
fn def(g: G) -> (&'static [&'static str], &'static [(f32, f32, f32, f32, f32)], &'static [(f32, f32, f32, bool)]) {
    match g {
        G::Card => (&["M18 10h3v4h-3M7 7V5M11 7V5"], &[(3.0, 7.0, 15.0, 10.0, 2.0)], &[]),
        G::Live => (&["M8 21h8M12 17v4", "*M10 9l4 2-4 2z"], &[(3.0, 5.0, 18.0, 12.0, 2.0)], &[]),
        G::List => (&[], &[(3.0, 4.0, 8.0, 7.0, 1.5), (13.0, 4.0, 8.0, 7.0, 1.5), (3.0, 13.0, 8.0, 7.0, 1.5), (13.0, 13.0, 8.0, 7.0, 1.5)], &[]),
        G::Gear => (&["M12 2.5v3M12 18.5v3M2.5 12h3M18.5 12h3M5.3 5.3l2.1 2.1M16.6 16.6l2.1 2.1M5.3 18.7l2.1-2.1M16.6 7.4l2.1-2.1"], &[], &[(12.0, 12.0, 3.0, false)]),
        G::Cam => (&["M4 8h3l2-3h6l2 3h3v11H4z"], &[], &[(12.0, 13.0, 3.5, false)]),
        G::Full => (&["M4 9V4h5M20 9V4h-5M4 15v5h5M20 15v5h-5"], &[], &[]),
        G::Unfull => (&["M9 4v5H4M15 4v5h5M9 20v-5H4M15 20v-5h5"], &[], &[]),
        G::Vol => (&["M4 9h4l5-4v14l-5-4H4z", "M16 9a4 4 0 010 6M18.5 6.5a7.5 7.5 0 010 11"], &[], &[]),
        G::Mute => (&["M4 9h4l5-4v14l-5-4H4z", "M16.5 9.5l5 5M21.5 9.5l-5 5"], &[], &[]),
        G::Bolt => (&["*M13 2L4 14h7l-1 8 9-12h-7z"], &[], &[]),
        G::Usb => (&["M12 3v15M9 6l3-3 3 3M12 13l-4-3V8M12 15l4-3v-2"], &[(6.8, 6.5, 2.4, 2.4, 0.0)], &[(12.0, 19.5, 1.8, false), (16.0, 8.8, 1.3, false)]),
        G::Shield => (&["M12 3l8 3v6c0 4.5-3.5 8-8 9-4.5-1-8-4.5-8-9V6z", "M8.5 12l2.5 2.5 4.5-5"], &[], &[]),
        G::Tv => (&["M8 21h8"], &[(3.0, 5.0, 18.0, 12.0, 2.0)], &[]),
        G::Folder => (&["M3 6.5A1.5 1.5 0 014.5 5H9l2 2h8.5A1.5 1.5 0 0121 8.5v9a1.5 1.5 0 01-1.5 1.5h-15A1.5 1.5 0 013 17.5z"], &[], &[]),
        G::Mic => (&["M5.5 11a6.5 6.5 0 0013 0M12 17.5V21"], &[(9.0, 3.0, 6.0, 11.0, 3.0)], &[]),
        G::Sun => (&["M12 2v2.5M12 19.5V22M2 12h2.5M19.5 12H22M4.9 4.9l1.8 1.8M17.3 17.3l1.8 1.8M4.9 19.1l1.8-1.8M17.3 6.7l1.8-1.8"], &[], &[(12.0, 12.0, 4.0, false)]),
        G::Film => (&["M7 5v14M17 5v14M3 9h4M3 15h4M17 9h4M17 15h4"], &[(3.0, 5.0, 18.0, 14.0, 2.0)], &[]),
        G::Fit => (&[], &[(3.0, 5.0, 18.0, 14.0, 2.0), (7.0, 8.5, 10.0, 7.0, 1.0)], &[]),
        G::Store => (&["M4 9l1.5-5h13L20 9M4 9h16v11H4zM4 9a2.7 2.7 0 005.3 0 2.7 2.7 0 005.4 0A2.7 2.7 0 0020 9"], &[], &[]),
        G::Exit => (&["M14 4H5v16h9M10 12h11M17 8l4 4-4 4"], &[], &[]),
        G::Play => (&["*M8 5l11 7-11 7z"], &[], &[]),
        G::X => (&["M6 6l12 12M18 6L6 18"], &[], &[]),
        G::Min => (&["M5 12h14"], &[], &[]),
        G::Max => (&[], &[(5.0, 5.0, 14.0, 14.0, 1.0)], &[]),
    }
}

/// A path's points, from SVG path data (M L H V A C Z, absolute and relative), arcs and curves as short lines.
pub fn parse(d: &str) -> Vec<(Vec<(f32, f32)>, bool)> {
    let mut sc = Scan { s: d.as_bytes(), i: 0 };
    let mut out: Vec<(Vec<(f32, f32)>, bool)> = Vec::new();
    let mut pts: Vec<(f32, f32)> = Vec::new();
    let (mut x, mut y, mut sx, mut sy) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
    let mut cmd = b'M';
    loop {
        sc.skip();
        if sc.i >= sc.s.len() {
            break;
        }
        let ch = sc.s[sc.i];
        if ch.is_ascii_alphabetic() {
            cmd = ch;
            sc.i += 1;
            if cmd == b'z' || cmd == b'Z' {
                if !pts.is_empty() {
                    pts.push((sx, sy));
                    out.push((std::mem::take(&mut pts), true));
                }
                x = sx;
                y = sy;
                continue;
            }
        }
        let rel = cmd.is_ascii_lowercase();
        let (ox, oy) = if rel { (x, y) } else { (0.0, 0.0) };
        let before = sc.i;
        match cmd.to_ascii_uppercase() {
            b'M' => {
                let (Some(nx), Some(ny)) = (sc.num(), sc.num()) else { break };
                if pts.len() > 1 {
                    out.push((std::mem::take(&mut pts), false));
                } else {
                    pts.clear();
                }
                x = ox + nx;
                y = oy + ny;
                sx = x;
                sy = y;
                pts.push((x, y));
                cmd = if rel { b'l' } else { b'L' };
            }
            b'L' => {
                let (Some(nx), Some(ny)) = (sc.num(), sc.num()) else { break };
                x = ox + nx;
                y = oy + ny;
                pts.push((x, y));
            }
            b'H' => {
                let Some(n) = sc.num() else { break };
                x = if rel { x + n } else { n };
                pts.push((x, y));
            }
            b'V' => {
                let Some(n) = sc.num() else { break };
                y = if rel { y + n } else { n };
                pts.push((x, y));
            }
            b'C' => {
                let v: Vec<f32> = (0..6).filter_map(|_| sc.num()).collect();
                if v.len() < 6 {
                    break;
                }
                let (x1, y1, x2, y2, ex, ey) = (ox + v[0], oy + v[1], ox + v[2], oy + v[3], ox + v[4], oy + v[5]);
                for k in 1..=12 {
                    let tt = k as f32 / 12.0;
                    let u = 1.0 - tt;
                    pts.push((
                        u * u * u * x + 3.0 * u * u * tt * x1 + 3.0 * u * tt * tt * x2 + tt * tt * tt * ex,
                        u * u * u * y + 3.0 * u * u * tt * y1 + 3.0 * u * tt * tt * y2 + tt * tt * tt * ey,
                    ));
                }
                x = ex;
                y = ey;
            }
            b'A' => {
                let (Some(rx), Some(ry), Some(_rot)) = (sc.num(), sc.num(), sc.num()) else { break };
                let (Some(large), Some(sweep)) = (sc.flag(), sc.flag()) else { break };
                let (Some(nx), Some(ny)) = (sc.num(), sc.num()) else { break };
                let (ex, ey) = (ox + nx, oy + ny);
                arc_points(&mut pts, (x, y), (ex, ey), rx.abs(), ry.abs(), large, sweep);
                x = ex;
                y = ey;
            }
            _ => break,
        }
        if sc.i == before {
            break;
        }
    }
    if pts.len() > 1 {
        out.push((pts, false));
    }
    out
}

struct Scan<'a> {
    s: &'a [u8],
    i: usize,
}

impl Scan<'_> {
    fn skip(&mut self) {
        while self.i < self.s.len() && (self.s[self.i] == b',' || self.s[self.i].is_ascii_whitespace()) {
            self.i += 1;
        }
    }
    fn num(&mut self) -> Option<f32> {
        self.skip();
        let st = self.i;
        let s = self.s;
        if self.i < s.len() && (s[self.i] == b'-' || s[self.i] == b'+') {
            self.i += 1;
        }
        let mut dot = false;
        while self.i < s.len() && (s[self.i].is_ascii_digit() || (s[self.i] == b'.' && !dot)) {
            if s[self.i] == b'.' {
                dot = true;
            }
            self.i += 1;
        }
        if self.i < s.len() && (s[self.i] == b'e' || s[self.i] == b'E') {
            self.i += 1;
            if self.i < s.len() && (s[self.i] == b'-' || s[self.i] == b'+') {
                self.i += 1;
            }
            while self.i < s.len() && s[self.i].is_ascii_digit() {
                self.i += 1;
            }
        }
        if self.i == st {
            return None;
        }
        std::str::from_utf8(&s[st..self.i]).ok()?.parse().ok()
    }
    /// An arc's flag: one character, 0 or 1 ("010 6" is 0, 1, 0 6).
    fn flag(&mut self) -> Option<bool> {
        self.skip();
        let ch = *self.s.get(self.i)?;
        if ch == b'0' || ch == b'1' {
            self.i += 1;
            Some(ch == b'1')
        } else {
            None
        }
    }
}

/// SVG's endpoint arc as points (the standard centre conversion).
fn arc_points(pts: &mut Vec<(f32, f32)>, p0: (f32, f32), p1: (f32, f32), mut rx: f32, mut ry: f32, large: bool, sweep: bool) {
    if rx == 0.0 || ry == 0.0 || (p0.0 - p1.0).abs() + (p0.1 - p1.1).abs() < 1e-4 {
        pts.push(p1);
        return;
    }
    let dx = (p0.0 - p1.0) / 2.0;
    let dy = (p0.1 - p1.1) / 2.0;
    let lam = (dx * dx) / (rx * rx) + (dy * dy) / (ry * ry);
    if lam > 1.0 {
        rx *= lam.sqrt();
        ry *= lam.sqrt();
    }
    let num = rx * rx * ry * ry - rx * rx * dy * dy - ry * ry * dx * dx;
    let den = rx * rx * dy * dy + ry * ry * dx * dx;
    let mut co = (num / den).max(0.0).sqrt();
    if large == sweep {
        co = -co;
    }
    let cxp = co * rx * dy / ry;
    let cyp = -co * ry * dx / rx;
    let cx = cxp + (p0.0 + p1.0) / 2.0;
    let cy = cyp + (p0.1 + p1.1) / 2.0;
    let ang = |ux: f32, uy: f32, vx: f32, vy: f32| {
        let d = (ux * vx + uy * vy) / ((ux * ux + uy * uy).sqrt() * (vx * vx + vy * vy).sqrt());
        let s = if ux * vy - uy * vx < 0.0 { -1.0 } else { 1.0 };
        s * d.clamp(-1.0, 1.0).acos()
    };
    let t1 = ang(1.0, 0.0, (dx - cxp) / rx, (dy - cyp) / ry);
    let mut dt = ang((dx - cxp) / rx, (dy - cyp) / ry, (-dx - cxp) / rx, (-dy - cyp) / ry);
    if !sweep && dt > 0.0 {
        dt -= std::f32::consts::TAU;
    } else if sweep && dt < 0.0 {
        dt += std::f32::consts::TAU;
    }
    let n = ((dt.abs() / 0.2).ceil() as usize).max(2);
    for k in 1..=n {
        let th = t1 + dt * k as f32 / n as f32;
        pts.push((cx + rx * th.cos(), cy + ry * th.sin()));
    }
}

struct Glyph {
    g: G,
    col: Color,
}

impl<M> canvas::Program<M> for Glyph {
    type State = Kept<(G, Color, iced::Size)>;
    fn draw(&self, st: &Self::State, r: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        st.check((self.g, self.col, b.size()));
        let cache = &st.cache;
        vec![cache.draw(r, b.size(), |f| {
            let k = b.width.min(b.height) / 24.0;
            let stroke = Stroke::default().with_color(self.col).with_width(1.85 * k).with_line_cap(LineCap::Round).with_line_join(LineJoin::Round);
            let (paths, rects, circles) = def(self.g);
            for d0 in paths {
                let fill_it = d0.starts_with('*');
                let d = d0.trim_start_matches('*');
                for (pts, closed) in parse(d) {
                    if pts.len() < 2 {
                        continue;
                    }
                    let p = Path::new(|pb| {
                        pb.move_to(Point::new(pts[0].0 * k, pts[0].1 * k));
                        for q in &pts[1..] {
                            pb.line_to(Point::new(q.0 * k, q.1 * k));
                        }
                        if closed {
                            pb.close();
                        }
                    });
                    if fill_it {
                        f.fill(&p, self.col);
                    } else {
                        f.stroke(&p, stroke);
                    }
                }
            }
            for (x, y, wd, h, rx) in rects {
                f.stroke(&Path::rounded_rectangle(Point::new(x * k, y * k), iced::Size::new(wd * k, h * k), (rx * k).into()), stroke);
            }
            if self.g == G::Tv {
                // The symbol's "60" (Inter 800, 6.5 px in its 24 px box).
                f.fill_text(canvas::Text { content: "60".into(), position: Point::new(8.3 * k, 7.9 * k), color: self.col, size: (6.5 * k).into(), font: w(font::Weight::ExtraBold), ..canvas::Text::default() });
            }
            for (x, y, rr, fl) in circles {
                let p = Path::circle(Point::new(x * k, y * k), rr * k);
                if *fl {
                    f.fill(&p, self.col);
                } else {
                    f.stroke(&p, stroke);
                }
            }
        })]
    }
}

pub fn glyph<'a, M: 'a>(g: G, col: Color, size: f32) -> E<'a, M> {
    iced::widget::canvas(Glyph { g, col }).width(Length::Fixed(size)).height(Length::Fixed(size)).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_path_reader_reads_the_mockups_icons() {
        let p = parse("M12 3v8");
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].0, vec![(12.0, 3.0), (12.0, 11.0)]);
        let p = parse("M4 9h4l5-4v14l-5-4H4z");
        assert!(p[0].1);
        assert_eq!(p[0].0[1], (8.0, 9.0));
        assert_eq!(p[0].0[2], (13.0, 5.0));
        // "a4 4 0 010 6": flags written together.
        let p = parse("M16 9a4 4 0 010 6");
        let last = *p[0].0.last().unwrap();
        assert!((last.0 - 16.0).abs() < 0.01 && (last.1 - 15.0).abs() < 0.01);
        // Two sub-paths.
        assert_eq!(parse("M8 13.5v2M7 14.5h2").len(), 2);
        // "-.7-2.9": numbers run together.
        let p = parse("M0 0l-.7-2.9");
        assert!((p[0].0[1].0 + 0.7).abs() < 0.001 && (p[0].0[1].1 + 2.9).abs() < 0.001);
        for g in [G::Card, G::Live, G::Shield, G::Folder, G::Store, G::Bolt, G::Usb, G::Mic] {
            for d in def(g).0 {
                assert!(!parse(d.trim_start_matches('*')).is_empty(), "{g:?}");
            }
        }
    }
}
