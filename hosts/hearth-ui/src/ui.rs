//! The pieces every Hearth app draws with, in the person's look (theme::Pal).

use iced::widget::canvas::{self, Frame, Geometry, LineCap, LineJoin, Path, Stroke};
use iced::widget::{button, column, container, mouse_area, opaque, row, stack, text, Space, Text};
use iced::{border, font, mouse, Background, Border, Color, Element, Font, Length, Padding, Point, Rectangle, Renderer, Shadow, Theme};

use crate::app::INTER;
use crate::theme::Pal;

// A block's own font (blocks.rs Look: Font and Text size): set only while that block's view is being made, so the
// text it makes takes it; everywhere else it's Inter at the size asked (no change).
thread_local! {
    static FACE: std::cell::Cell<Option<&'static str>> = const { std::cell::Cell::new(None) };
    static SCALE: std::cell::Cell<f32> = const { std::cell::Cell::new(1.0) };
}

/// `f` made with this font family (None: Inter) and text size (1.0: as asked); what it was is put back after.
pub fn with_face<R>(face: Option<&'static str>, scale: f32, f: impl FnOnce() -> R) -> R {
    let before = (FACE.with(|x| x.replace(face)), SCALE.with(|x| x.replace(scale)));
    let r = f();
    FACE.with(|x| x.set(before.0));
    SCALE.with(|x| x.set(before.1));
    r
}
fn scaled(size: f32) -> f32 {
    size * SCALE.with(|x| x.get())
}

pub fn weight(w: font::Weight) -> Font {
    match FACE.with(|x| x.get()) {
        Some(n) => Font { weight: w, ..Font::with_name(n) },
        None => Font { weight: w, ..INTER },
    }
}

/// Text in Inter at this size and colour.
pub fn t<'a>(s: impl text::IntoFragment<'a>, size: f32, c: Color) -> Text<'a> {
    text(s).size(scaled(size)).color(c).font(weight(font::Weight::Normal))
}
pub fn tm<'a>(s: impl text::IntoFragment<'a>, size: f32, c: Color) -> Text<'a> {
    text(s).size(scaled(size)).color(c).font(weight(font::Weight::Medium))
}
pub fn ts<'a>(s: impl text::IntoFragment<'a>, size: f32, c: Color) -> Text<'a> {
    text(s).size(scaled(size)).color(c).font(weight(font::Weight::Semibold))
}
pub fn tb<'a>(s: impl text::IntoFragment<'a>, size: f32, c: Color) -> Text<'a> {
    text(s).size(scaled(size)).color(c).font(weight(font::Weight::Bold))
}
/// One line, never wrapped (cells, titles): the cell clips what doesn't fit.
pub fn t1<'a>(s: impl text::IntoFragment<'a>, size: f32, c: Color) -> Text<'a> {
    t(s, size, c).wrapping(text::Wrapping::None)
}

/// `s` shortened to `n` characters with an ellipsis (text in a fixed-width cell).
pub fn fit(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        let mut o: String = s.chars().take(n.saturating_sub(1)).collect();
        o.push('…');
        o
    }
}

/// A container style: background, corners, an optional border.
pub fn boxed(bg: Color, radius: f32, line: Option<(Color, f32)>) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        text_color: None,
        background: Some(Background::Color(bg)),
        border: match line {
            Some((c, w)) => Border { color: c, width: w, radius: radius.into() },
            None => border::rounded(radius),
        },
        shadow: Shadow::default(),
        snap: true,
    }
}

pub fn clear() -> impl Fn(&Theme) -> container::Style {
    |_| container::Style::default()
}

/// A line across (or down, with `vline`).
pub fn hline<'a, M: 'a>(c: Color) -> Element<'a, M> {
    container(Space::new()).width(Length::Fill).height(Length::Fixed(1.0)).style(boxed(c, 0.0, None)).into()
}
pub fn vline<'a, M: 'a>(c: Color) -> Element<'a, M> {
    container(Space::new()).width(Length::Fixed(1.0)).height(Length::Fill).style(boxed(c, 0.0, None)).into()
}
pub fn gap<'a, M: 'a>(w: f32, h: f32) -> Element<'a, M> {
    Space::new().width(Length::Fixed(w)).height(Length::Fixed(h)).into()
}
pub fn fill<'a, M: 'a>() -> Element<'a, M> {
    Space::new().width(Length::Fill).into()
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kind {
    Primary,
    Normal,
    Danger,
}

/// A button as Hearth's mockups draw them: accent (primary), outlined, or red; a ring when the controller is on it.
pub fn btn<'a, M: Clone + 'a>(p: &Pal, label: impl text::IntoFragment<'a>, kind: Kind, focused: bool, on: Option<M>) -> Element<'a, M> {
    let p = *p;
    let enabled = on.is_some();
    let fg = match kind {
        Kind::Primary if enabled => p.ink,
        Kind::Primary => p.muted,
        Kind::Danger => Color::WHITE,
        Kind::Normal => if enabled { p.text } else { p.muted },
    };
    let semi = kind != Kind::Normal;
    let lbl = text(label).size(scaled(13.0)).color(fg).font(weight(if semi { font::Weight::Semibold } else { font::Weight::Normal })).wrapping(text::Wrapping::None);
    let r = p.r(6.0);
    let b = button(container(lbl).center_y(Length::Fixed(20.0)))
        .padding(Padding { top: 7.0, right: if semi { 18.0 } else { 14.0 }, bottom: 7.0, left: if semi { 18.0 } else { 14.0 } })
        .on_press_maybe(on)
        .style(move |_: &Theme, st: button::Status| {
            let hover = matches!(st, button::Status::Hovered | button::Status::Pressed);
            let bg = match kind {
                Kind::Primary if enabled => if hover { Pal::mix(p.accent, Color::WHITE, 0.12) } else { p.accent },
                Kind::Primary => p.line,
                Kind::Danger => if hover { Pal::mix(p.danger, Color::WHITE, 0.12) } else { p.danger },
                Kind::Normal => if hover && enabled { p.hover() } else { Color::TRANSPARENT },
            };
            button::Style {
                background: Some(Background::Color(bg)),
                text_color: fg,
                border: Border { color: if kind == Kind::Normal { p.line } else { Color::TRANSPARENT }, width: if kind == Kind::Normal { 1.0 } else { 0.0 }, radius: r.into() },
                shadow: Shadow::default(),
                snap: true,
            }
        });
    ring(p, focused, r, b.into())
}

/// The controller's focus ring around anything (2 px in the accent), or nothing.
pub fn ring<'a, M: 'a>(p: Pal, on: bool, r: f32, inner: Element<'a, M>) -> Element<'a, M> {
    container(inner)
        .padding(3.0)
        .style(move |_: &Theme| container::Style {
            border: Border { color: if on { p.accent } else { Color::TRANSPARENT }, width: 2.0, radius: (r + 3.0).into() },
            ..container::Style::default()
        })
        .into()
}

/// A small rounded label (Running now, NEW, AI, Game · Proton…).
pub fn chip<'a, M: 'a>(label: impl text::IntoFragment<'a>, bg: Color, fg: Color) -> Element<'a, M> {
    container(text(label).size(11.0).color(fg).font(weight(font::Weight::Semibold)).wrapping(text::Wrapping::None))
        .padding(Padding { top: 1.0, right: 7.0, bottom: 1.0, left: 7.0 })
        .style(boxed(bg, 9.0, None))
        .into()
}

/// A tick box with its label.
pub fn check<'a, M: Clone + 'a>(p: &Pal, on: bool, label: impl text::IntoFragment<'a>, focused: bool, msg: M) -> Element<'a, M> {
    let p = *p;
    let bx: Element<'a, M> = container(if on { glyph(G::Check, p.ink, 12.0, 3.0) } else { gap(12.0, 12.0) })
        .center_x(Length::Fixed(18.0))
        .center_y(Length::Fixed(18.0))
        .style(move |_: &Theme| container::Style {
            background: Some(Background::Color(if on { p.accent } else { Color::TRANSPARENT })),
            border: Border { color: if on { p.accent } else { p.muted }, width: if on { 0.0 } else { 1.0 }, radius: 4.0.into() },
            ..container::Style::default()
        })
        .into();
    let inner = row![bx, t(label, 13.0, p.text)].spacing(10.0).align_y(iced::Center);
    ring(p, focused, 6.0, mouse_area(container(inner).padding(Padding { top: 3.0, right: 6.0, bottom: 3.0, left: 3.0 })).on_press(msg).interaction(mouse::Interaction::Pointer).into())
}

/// A card: the panel colour, a border, rounded.
pub fn card<'a, M: 'a>(p: &Pal, inner: impl Into<Element<'a, M>>) -> container::Container<'a, M> {
    container(inner).style(boxed(p.panel, p.r(10.0), Some((p.line, 1.0))))
}

/// What the window's own title bar can ask for.
#[derive(Debug, Clone)]
pub struct TitleMsgs<M> {
    pub drag: M,
    pub maximize: M,
    pub minimize: Option<M>,
    pub close: M,
}

/// The window's title bar as the mockups draw it: the app's mark, its name, anything in the middle (Task Manager's
/// search), and the window buttons. Dragging it moves the window; a double click maximizes.
pub fn title_bar<'a, M: Clone + 'a>(p: &Pal, mark: G, title: &'a str, middle: Option<Element<'a, M>>, m: TitleMsgs<M>, bg: Color) -> Element<'a, M> {
    let p = *p;
    let wbtn = |g: G, msg: M, hover_red: bool| -> Element<'a, M> {
        button(container(glyph(g, p.muted, 10.0, 2.2)).center_x(Length::Fixed(46.0)).center_y(Length::Fixed(40.0)))
            .padding(0.0)
            .on_press(msg)
            .style(move |_: &Theme, st: button::Status| button::Style {
                background: Some(Background::Color(match st {
                    button::Status::Hovered | button::Status::Pressed if hover_red => Color::from_rgb8(0xc4, 0x2b, 0x1c),
                    button::Status::Hovered | button::Status::Pressed => p.hover(),
                    _ => Color::TRANSPARENT,
                })),
                text_color: p.text,
                border: Border::default(),
                shadow: Shadow::default(),
                snap: true,
            })
            .into()
    };
    let name = row![gap(14.0, 1.0), glyph(mark, p.accent, 16.0, 2.0), gap(10.0, 1.0), tm(title, 13.0, p.text).wrapping(text::Wrapping::None)]
        .align_y(iced::Center);
    let draggable = |e: Element<'a, M>| -> Element<'a, M> {
        mouse_area(e).on_press(m.drag.clone()).on_double_click(m.maximize.clone()).into()
    };
    let mut r = row![draggable(container(name).center_y(Length::Fixed(40.0)).into())].align_y(iced::Center);
    match middle {
        Some(mid) => {
            r = r.push(draggable(container(Space::new()).width(Length::Fill).height(Length::Fixed(40.0)).into()));
            r = r.push(mid);
            r = r.push(draggable(container(Space::new()).width(Length::Fill).height(Length::Fixed(40.0)).into()));
        }
        None => r = r.push(draggable(container(Space::new()).width(Length::Fill).height(Length::Fixed(40.0)).into())),
    }
    if let Some(min) = m.minimize.clone() {
        r = r.push(wbtn(G::Minimize, min, false));
        r = r.push(wbtn(G::Maximize, m.maximize.clone(), false));
    }
    r = r.push(wbtn(G::Close, m.close.clone(), true));
    if let Some(fm) = frame_meter() {
        r = r.push(fm);
    }
    container(r).width(Length::Fill).height(Length::Fixed(40.0)).style(boxed(bg, 0.0, None)).into()
}

/// `base` with a dimmed layer over it and `dialog` in the middle. A click outside sends `outside` (if any).
pub fn dialog<'a, M: Clone + 'a>(base: Element<'a, M>, dlg: Element<'a, M>, outside: Option<M>) -> Element<'a, M> {
    let dim = container(container(opaque(dlg)).center_x(Length::Fill).center_y(Length::Fill))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(boxed(Color { r: 0.0, g: 0.0, b: 0.0, a: 0.7 }, 0.0, None));
    let layer: Element<'a, M> = match outside {
        Some(o) => mouse_area(dim).on_press(o).into(),
        None => mouse_area(dim).into(),
    };
    stack![base, opaque(layer)].into()
}

/// A dialog's box: the panel, a border, padding 20.
pub fn dialog_box<'a, M: 'a>(p: &Pal, width: f32, inner: impl Into<Element<'a, M>>) -> Element<'a, M> {
    container(inner).padding(20.0).max_width(width).width(Length::Fixed(width)).style(boxed(p.panel, p.r(10.0), Some((p.line, 1.0)))).into()
}

/// The window's edges: a 6 px strip on each side that resizes it (the window has no frame of its own).
pub fn edges<'a, M: Clone + 'a>(inner: Element<'a, M>, on: impl Fn(iced::window::Direction) -> M) -> Element<'a, M> {
    use iced::window::Direction as D;
    let strip = |w: Length, h: Length, d: D, i: mouse::Interaction| -> Element<'a, M> {
        mouse_area(Space::new().width(w).height(h)).on_press(on(d)).interaction(i).into()
    };
    let e = 5.0;
    let top = row![
        strip(Length::Fixed(e), Length::Fixed(e), D::NorthWest, mouse::Interaction::ResizingDiagonallyDown),
        strip(Length::Fill, Length::Fixed(e), D::North, mouse::Interaction::ResizingVertically),
        strip(Length::Fixed(e), Length::Fixed(e), D::NorthEast, mouse::Interaction::ResizingDiagonallyUp),
    ];
    let bottom = row![
        strip(Length::Fixed(e), Length::Fixed(e), D::SouthWest, mouse::Interaction::ResizingDiagonallyUp),
        strip(Length::Fill, Length::Fixed(e), D::South, mouse::Interaction::ResizingVertically),
        strip(Length::Fixed(e), Length::Fixed(e), D::SouthEast, mouse::Interaction::ResizingDiagonallyDown),
    ];
    let middle = row![
        strip(Length::Fixed(e), Length::Fill, D::West, mouse::Interaction::ResizingHorizontally),
        Space::new().width(Length::Fill).height(Length::Fill),
        strip(Length::Fixed(e), Length::Fill, D::East, mouse::Interaction::ResizingHorizontally),
    ];
    let frame = column![top, middle, bottom].width(Length::Fill).height(Length::Fill);
    stack![inner, frame].into()
}

// ------------------------------------------------------------------ glyphs
/// Line icons, drawn in a 24 × 24 box (the QML apps' Glyph paths).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum G {
    Check,
    Arrow,
    Plus,
    Close,
    Minimize,
    Maximize,
    Restore,
    Reliability,
    Tasks,
    Search,
    Gear,
    Chevron,
    Dots,
    Folder,
    Leaf,
    Stop,
    Play,
    Grid,
    Pulse,
    List,
    Clock,
    Users,
    Rocket,
    Cog,
    Down,
    Home,
    Wifi,
    Bt,
    Display,
    Palette,
    Power,
    Shield,
    User,
    Pad,
    Server,
    Link,
    Update,
    Access,
    Store,
    Sound,
    Drive,
    Moon,
    Info,
    Camera,
    // hearth-pad-host (apps/pad): the Hearth menu's items.
    Keyboard,
    Mouse,
    Menu,
    // hearth-notes-app (apps/notes): its rail (the mockups' ▤ ▥ ✳ ⇅ ✎ ?).
    Doc,
    Columns,
    Asterisk,
    Sync,
    Pencil,
    Help,
    // hearth-ideas-app (apps/ideas): its mark.
    Bell,
}

struct Glyph {
    g: G,
    c: Color,
    w: f32,
}

fn lines(g: G) -> (Vec<Vec<(f32, f32)>>, Vec<(f32, f32, f32)>) {
    let v = |pts: &[(f32, f32)]| pts.to_vec();
    match g {
        G::Check => (vec![v(&[(5.0, 12.0), (10.0, 17.0), (19.0, 7.0)])], vec![]),
        G::Arrow => (vec![v(&[(5.0, 12.0), (19.0, 12.0)]), v(&[(13.0, 6.0), (19.0, 12.0), (13.0, 18.0)])], vec![]),
        G::Plus => (vec![v(&[(12.0, 5.0), (12.0, 19.0)]), v(&[(5.0, 12.0), (19.0, 12.0)])], vec![]),
        G::Close => (vec![v(&[(2.0, 2.0), (22.0, 22.0)]), v(&[(22.0, 2.0), (2.0, 22.0)])], vec![]),
        G::Minimize => (vec![v(&[(2.0, 12.0), (22.0, 12.0)])], vec![]),
        G::Maximize => (vec![v(&[(2.0, 2.0), (22.0, 2.0), (22.0, 22.0), (2.0, 22.0), (2.0, 2.0)])], vec![]),
        G::Restore => {
            let mut arc = Vec::new();
            for i in 0..=20 {
                let a = std::f32::consts::PI * (1.0 + 1.5 * i as f32 / 20.0);
                arc.push((12.0 + 9.0 * a.cos(), 12.0 + 9.0 * a.sin()));
            }
            (vec![arc, v(&[(3.0, 4.0), (3.0, 9.0), (8.0, 9.0)]), v(&[(12.0, 7.0), (12.0, 12.0), (15.0, 14.0)])], vec![])
        }
        G::Reliability | G::Pulse => (vec![v(&[(3.0, 17.0), (8.0, 11.0), (12.0, 14.0), (17.0, 6.0), (21.0, 11.0)])], vec![]),
        G::Tasks => (vec![v(&[(3.0, 4.0), (21.0, 4.0), (21.0, 20.0), (3.0, 20.0), (3.0, 4.0)]), v(&[(6.0, 15.0), (10.0, 10.0), (13.0, 13.0), (18.0, 8.0)])], vec![]),
        G::Search => (vec![v(&[(15.0, 15.0), (21.0, 21.0)])], vec![(10.0, 10.0, 6.5)]),
        G::Gear | G::Cog => (vec![v(&[(12.0, 2.0), (12.0, 5.0)]), v(&[(12.0, 19.0), (12.0, 22.0)]), v(&[(2.0, 12.0), (5.0, 12.0)]), v(&[(19.0, 12.0), (22.0, 12.0)]),
                                  v(&[(5.0, 5.0), (7.0, 7.0)]), v(&[(17.0, 17.0), (19.0, 19.0)]), v(&[(19.0, 5.0), (17.0, 7.0)]), v(&[(5.0, 19.0), (7.0, 17.0)])], vec![(12.0, 12.0, 5.0)]),
        G::Chevron => (vec![v(&[(9.0, 6.0), (15.0, 12.0), (9.0, 18.0)])], vec![]),
        G::Down => (vec![v(&[(6.0, 9.0), (12.0, 15.0), (18.0, 9.0)])], vec![]),
        G::Dots => (vec![], vec![(5.0, 12.0, 1.2), (12.0, 12.0, 1.2), (19.0, 12.0, 1.2)]),
        G::Folder => (vec![v(&[(3.0, 6.0), (9.0, 6.0), (11.0, 8.0), (21.0, 8.0), (21.0, 19.0), (3.0, 19.0), (3.0, 6.0)])], vec![]),
        G::Leaf => (vec![v(&[(5.0, 19.0), (5.0, 12.0), (8.0, 7.0), (19.0, 5.0), (17.0, 16.0), (12.0, 19.0), (5.0, 19.0)]), v(&[(5.0, 19.0), (14.0, 10.0)])], vec![]),
        G::Stop => (vec![v(&[(6.0, 6.0), (18.0, 6.0), (18.0, 18.0), (6.0, 18.0), (6.0, 6.0)])], vec![]),
        G::Play => (vec![v(&[(8.0, 5.0), (19.0, 12.0), (8.0, 19.0), (8.0, 5.0)])], vec![]),
        G::Grid => (vec![v(&[(4.0, 4.0), (10.0, 4.0), (10.0, 10.0), (4.0, 10.0), (4.0, 4.0)]), v(&[(14.0, 4.0), (20.0, 4.0), (20.0, 10.0), (14.0, 10.0), (14.0, 4.0)]),
                         v(&[(4.0, 14.0), (10.0, 14.0), (10.0, 20.0), (4.0, 20.0), (4.0, 14.0)]), v(&[(14.0, 14.0), (20.0, 14.0), (20.0, 20.0), (14.0, 20.0), (14.0, 14.0)])], vec![]),
        G::List => (vec![v(&[(8.0, 6.0), (20.0, 6.0)]), v(&[(8.0, 12.0), (20.0, 12.0)]), v(&[(8.0, 18.0), (20.0, 18.0)])], vec![(4.0, 6.0, 0.8), (4.0, 12.0, 0.8), (4.0, 18.0, 0.8)]),
        G::Clock => (vec![v(&[(12.0, 7.0), (12.0, 12.0), (16.0, 14.0)])], vec![(12.0, 12.0, 9.0)]),
        G::Users => (vec![v(&[(3.0, 20.0), (4.0, 16.0), (9.0, 14.0), (14.0, 16.0), (15.0, 20.0)]), v(&[(16.0, 13.0), (20.0, 15.0), (21.0, 19.0)])], vec![(9.0, 8.0, 3.5), (16.0, 8.0, 2.8)]),
        G::Home => (vec![v(&[(3.0, 11.0), (12.0, 3.0), (21.0, 11.0)]), v(&[(5.0, 9.5), (5.0, 21.0), (19.0, 21.0), (19.0, 9.5)]), v(&[(10.0, 21.0), (10.0, 15.0), (14.0, 15.0), (14.0, 21.0)])], vec![]),
        G::Wifi => {
            let arc = |r: f32| -> Vec<(f32, f32)> { (0..=12).map(|i| { let a = std::f32::consts::PI * (1.25 + 0.5 * i as f32 / 12.0); (12.0 + r * a.cos(), 19.0 + r * a.sin()) }).collect() };
            (vec![arc(15.0), arc(10.0), arc(5.0)], vec![(12.0, 19.0, 1.4)])
        }
        G::Bt => (vec![v(&[(7.0, 7.0), (17.0, 16.0), (12.0, 21.0), (12.0, 3.0), (17.0, 8.0), (7.0, 17.0)])], vec![]),
        G::Display => (vec![v(&[(3.0, 4.0), (21.0, 4.0), (21.0, 16.0), (3.0, 16.0), (3.0, 4.0)]), v(&[(12.0, 16.0), (12.0, 20.0)]), v(&[(8.0, 20.0), (16.0, 20.0)])], vec![]),
        G::Palette => (vec![], vec![(12.0, 12.0, 9.0), (8.0, 9.0, 1.4), (12.0, 7.0, 1.4), (16.0, 9.0, 1.4), (15.0, 15.0, 2.2)]),
        G::Power => {
            // A ring open at the top (from -60° round the bottom to 240°), and the line through the gap.
            let arc: Vec<(f32, f32)> = (0..=20).map(|i| { let a = (-60.0 + 300.0 * i as f32 / 20.0f32).to_radians(); (12.0 + 8.0 * a.cos(), 13.0 + 8.0 * a.sin()) }).collect();
            (vec![arc, v(&[(12.0, 3.0), (12.0, 11.0)])], vec![])
        }
        G::Shield => (vec![v(&[(12.0, 3.0), (20.0, 6.0), (19.0, 13.0), (12.0, 21.0), (5.0, 13.0), (4.0, 6.0), (12.0, 3.0)])], vec![]),
        G::User => (vec![v(&[(4.0, 21.0), (5.0, 16.0), (12.0, 13.5), (19.0, 16.0), (20.0, 21.0)])], vec![(12.0, 8.0, 4.0)]),
        G::Pad => (vec![v(&[(4.0, 8.0), (20.0, 8.0), (22.0, 17.0), (18.0, 18.0), (15.0, 14.0), (9.0, 14.0), (6.0, 18.0), (2.0, 17.0), (4.0, 8.0)]), v(&[(8.0, 9.5), (8.0, 12.5)]), v(&[(6.5, 11.0), (9.5, 11.0)])], vec![(16.0, 10.5, 0.9), (17.5, 12.0, 0.9)]),
        G::Server => (vec![v(&[(4.0, 4.0), (20.0, 4.0), (20.0, 10.0), (4.0, 10.0), (4.0, 4.0)]), v(&[(4.0, 14.0), (20.0, 14.0), (20.0, 20.0), (4.0, 20.0), (4.0, 14.0)])], vec![(7.5, 7.0, 0.9), (7.5, 17.0, 0.9)]),
        G::Link => (vec![v(&[(10.0, 14.0), (14.0, 10.0)]), v(&[(11.0, 6.0), (14.0, 3.5), (20.5, 10.0), (18.0, 13.0)]), v(&[(13.0, 18.0), (10.0, 20.5), (3.5, 14.0), (6.0, 11.0)])], vec![]),
        G::Update => {
            let arc: Vec<(f32, f32)> = (0..=20).map(|i| { let a = std::f32::consts::PI * (0.2 + 1.6 * i as f32 / 20.0); (12.0 + 8.0 * a.cos(), 12.0 + 8.0 * a.sin()) }).collect();
            (vec![arc, v(&[(18.0, 2.5), (19.5, 7.0), (15.0, 8.0)])], vec![])
        }
        G::Access => (vec![v(&[(5.0, 8.0), (12.0, 9.5), (19.0, 8.0)]), v(&[(12.0, 9.5), (12.0, 14.0)]), v(&[(8.5, 21.0), (12.0, 14.0), (15.5, 21.0)])], vec![(12.0, 4.5, 2.0)]),
        G::Store => (vec![v(&[(4.0, 8.0), (20.0, 8.0), (19.0, 21.0), (5.0, 21.0), (4.0, 8.0)]), v(&[(8.5, 8.0), (8.5, 6.0), (10.0, 3.5), (14.0, 3.5), (15.5, 6.0), (15.5, 8.0)])], vec![]),
        G::Sound => (vec![v(&[(3.0, 9.0), (7.0, 9.0), (12.0, 4.0), (12.0, 20.0), (7.0, 15.0), (3.0, 15.0), (3.0, 9.0)]), v(&[(16.0, 9.0), (17.5, 12.0), (16.0, 15.0)]), v(&[(19.0, 6.0), (21.5, 12.0), (19.0, 18.0)])], vec![]),
        G::Drive => (vec![v(&[(3.0, 6.0), (21.0, 6.0), (21.0, 18.0), (3.0, 18.0), (3.0, 6.0)]), v(&[(3.0, 13.0), (21.0, 13.0)])], vec![(17.5, 15.5, 0.9)]),
        G::Moon => {
            let mut a1: Vec<(f32, f32)> = (0..=20).map(|i| { let a = std::f32::consts::PI * (0.35 + 1.55 * i as f32 / 20.0); (12.0 + 9.0 * a.cos(), 12.0 + 9.0 * a.sin()) }).collect();
            let a2: Vec<(f32, f32)> = (0..=16).rev().map(|i| { let a = std::f32::consts::PI * (0.55 + 1.15 * i as f32 / 16.0); (16.0 + 7.0 * a.cos(), 9.0 + 7.0 * a.sin()) }).collect();
            a1.extend(a2);
            (vec![a1], vec![])
        }
        G::Info => (vec![v(&[(12.0, 11.0), (12.0, 17.0)])], vec![(12.0, 12.0, 9.0), (12.0, 7.5, 1.1)]),
        G::Rocket => (vec![v(&[(12.0, 3.0), (17.0, 9.0), (16.0, 16.0), (8.0, 16.0), (7.0, 9.0), (12.0, 3.0)]), v(&[(9.0, 19.0), (12.0, 22.0), (15.0, 19.0)])], vec![(12.0, 10.0, 1.6)]),
        G::Camera => (vec![v(&[(3.0, 8.0), (8.0, 8.0), (9.5, 5.0), (14.5, 5.0), (16.0, 8.0), (21.0, 8.0), (21.0, 19.0), (3.0, 19.0), (3.0, 8.0)])], vec![(12.0, 13.5, 3.5)]),
        G::Keyboard => (vec![v(&[(2.0, 6.0), (22.0, 6.0), (22.0, 18.0), (2.0, 18.0), (2.0, 6.0)]), v(&[(7.5, 14.5), (16.5, 14.5)])],
                        vec![(6.0, 10.0, 0.9), (10.0, 10.0, 0.9), (14.0, 10.0, 0.9), (18.0, 10.0, 0.9)]),
        G::Mouse => {
            let body: Vec<(f32, f32)> = (0..=28).map(|i| { let a = std::f32::consts::TAU * i as f32 / 28.0; (12.0 + 6.0 * a.cos(), 12.0 + 8.5 * a.sin()) }).collect();
            (vec![body, v(&[(12.0, 3.5), (12.0, 9.0)])], vec![])
        }
        G::Menu => (vec![v(&[(4.0, 7.0), (20.0, 7.0)]), v(&[(4.0, 12.0), (20.0, 12.0)]), v(&[(4.0, 17.0), (20.0, 17.0)])], vec![]),
        G::Doc => (vec![v(&[(5.0, 3.0), (19.0, 3.0), (19.0, 21.0), (5.0, 21.0), (5.0, 3.0)]), v(&[(8.0, 8.0), (16.0, 8.0)]), v(&[(8.0, 12.0), (16.0, 12.0)]), v(&[(8.0, 16.0), (13.0, 16.0)])], vec![]),
        G::Columns => (vec![v(&[(3.0, 4.0), (21.0, 4.0), (21.0, 20.0), (3.0, 20.0), (3.0, 4.0)]), v(&[(9.0, 4.0), (9.0, 20.0)]), v(&[(15.0, 4.0), (15.0, 20.0)])], vec![]),
        G::Asterisk => (vec![v(&[(12.0, 3.0), (12.0, 21.0)]), v(&[(4.2, 7.5), (19.8, 16.5)]), v(&[(19.8, 7.5), (4.2, 16.5)])], vec![]),
        G::Sync => (vec![v(&[(8.0, 20.0), (8.0, 4.0)]), v(&[(4.0, 8.0), (8.0, 4.0), (12.0, 8.0)]), v(&[(16.0, 4.0), (16.0, 20.0)]), v(&[(12.0, 16.0), (16.0, 20.0), (20.0, 16.0)])], vec![]),
        G::Pencil => (vec![v(&[(4.0, 20.0), (5.0, 15.0), (16.0, 4.0), (20.0, 8.0), (9.0, 19.0), (4.0, 20.0)]), v(&[(14.0, 6.0), (18.0, 10.0)])], vec![]),
        G::Bell => {
            let dome: Vec<(f32, f32)> = (0..=12).map(|i| { let a = std::f32::consts::PI * (1.0 + i as f32 / 12.0); (12.0 + 6.0 * a.cos(), 11.0 + 6.0 * a.sin()) }).collect();
            let mut b = vec![(4.5, 17.0), (6.0, 15.0)];
            b.extend(std::iter::once((6.0, 11.0)).chain(dome).chain(std::iter::once((18.0, 11.0))));
            b.extend([(18.0, 15.0), (19.5, 17.0), (4.5, 17.0)]);
            (vec![b], vec![(12.0, 19.5, 1.6)])
        }
        G::Help => {
            let arc: Vec<(f32, f32)> = (0..=14).map(|i| { let a = std::f32::consts::PI * (1.0 + 1.25 * i as f32 / 14.0); (12.0 + 4.5 * a.cos(), 9.0 + 4.5 * a.sin()) }).collect();
            let mut q = arc;
            q.push((12.0, 15.0));
            (vec![q], vec![(12.0, 19.5, 1.2)])
        }
    }
}

/// A glyph's drawing, kept between frames (scrolling a long page redraws every frame: without this each icon's paths
/// were built, and on the graphics card tessellated, again every frame). Drawn again only when the glyph, its colour,
/// its line width or its size changes.
#[derive(Default)]
pub(crate) struct GlyphCache {
    cache: canvas::Cache,
    key: std::cell::Cell<Option<(G, Color, f32, iced::Size)>>,
}

impl<M> canvas::Program<M> for Glyph {
    type State = GlyphCache;
    fn draw(&self, st: &GlyphCache, renderer: &Renderer, _t: &Theme, bounds: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        let key = Some((self.g, self.c, self.w, bounds.size()));
        if st.key.get() != key {
            st.cache.clear();
            st.key.set(key);
        }
        vec![st.cache.draw(renderer, bounds.size(), |f| self.paint(f, bounds))]
    }
}

impl Glyph {
    fn paint(&self, f: &mut Frame, bounds: Rectangle) {
        let k = bounds.width.min(bounds.height) / 24.0;
        let (ls, cs) = lines(self.g);
        let stroke = || Stroke::default().with_color(self.c).with_width(self.w * k.max(0.5)).with_line_cap(LineCap::Round).with_line_join(LineJoin::Round);
        for l in ls {
            if l.len() < 2 {
                continue;
            }
            let path = Path::new(|b| {
                b.move_to(Point::new(l[0].0 * k, l[0].1 * k));
                for pt in &l[1..] {
                    b.line_to(Point::new(pt.0 * k, pt.1 * k));
                }
            });
            f.stroke(&path, stroke());
        }
        for (x, y, r) in cs {
            let path = Path::circle(Point::new(x * k, y * k), r * k);
            if r < 2.0 {
                f.fill(&path, self.c);
            } else {
                f.stroke(&path, stroke());
            }
        }
    }
}

struct Meter;

impl<M> canvas::Program<M> for Meter {
    type State = ();
    fn draw(&self, _s: &(), _r: &Renderer, _t: &Theme, _b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        crate::app::frame_tick();
        vec![]
    }
}

/// CI's frame counter (app::bench, HEARTH_UI_FRAMES): a 1 px canvas that counts the frames drawn. Nothing without it.
pub fn frame_meter<'a, M: 'a>() -> Option<Element<'a, M>> {
    crate::app::bench()?;
    Some(cv(Meter).width(Length::Fixed(1.0)).height(Length::Fixed(1.0)).into())
}

/// A line icon at `size` px.
pub fn glyph<'a, M: 'a>(g: G, c: Color, size: f32, w: f32) -> Element<'a, M> {
    cv(Glyph { g, c, w }).width(Length::Fixed(size)).height(Length::Fixed(size)).into()
}

fn cv<P, M>(p: P) -> canvas::Canvas<P, M>
where
    P: canvas::Program<M>,
{
    canvas::Canvas::new(p)
}

// ------------------------------------------------------------------ graphs
/// A line graph of values 0..=max (Task Manager's Performance, Reliability's stability line), with the area under it
/// filled faintly and a dashed second line (`dashed`, e.g. compressed memory). Draws only when its values change.
pub struct Graph {
    pub values: Vec<f32>,
    pub dashed: Vec<f32>,
    pub max: f32,
    pub min: f32,
    pub color: Color,
    pub grid: Color,
    pub fill: bool,
    pub width: f32,
    /// Points sit in the middle of their column (Reliability) instead of on the edges (Performance).
    pub centred: bool,
    pub gridlines: Vec<f32>,
}

impl<M> canvas::Program<M> for Graph {
    type State = ();
    fn draw(&self, _s: &(), renderer: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        let mut f = Frame::new(renderer, b.size());
        let span = (self.max - self.min).max(0.0001);
        let y_of = |v: f32| b.height - 3.0 - ((v - self.min) / span).clamp(0.0, 1.0) * (b.height - 6.0);
        for g in &self.gridlines {
            let y = y_of(*g);
            f.stroke(&Path::line(Point::new(0.0, y), Point::new(b.width, y)), Stroke::default().with_color(self.grid).with_width(1.0));
        }
        let draw = |f: &mut Frame, vals: &[f32], dashed: bool| {
            let n = vals.len();
            if n < 2 && !(self.centred && n == 1) {
                return;
            }
            let x_of = |i: usize| if self.centred { (i as f32 + 0.5) * b.width / n as f32 } else { i as f32 * b.width / (n - 1).max(1) as f32 };
            let line = Path::new(|p| {
                p.move_to(Point::new(x_of(0), y_of(vals[0])));
                for (i, v) in vals.iter().enumerate().skip(1) {
                    p.line_to(Point::new(x_of(i), y_of(*v)));
                }
            });
            if self.fill && !dashed {
                let area = Path::new(|p| {
                    p.move_to(Point::new(x_of(0), b.height));
                    for (i, v) in vals.iter().enumerate() {
                        p.line_to(Point::new(x_of(i), y_of(*v)));
                    }
                    p.line_to(Point::new(x_of(n - 1), b.height));
                    p.close();
                });
                f.fill(&area, Pal::a(self.color, 0.16));
            }
            let mut st = Stroke::default().with_color(self.color).with_width(self.width).with_line_join(LineJoin::Round).with_line_cap(LineCap::Round);
            if dashed {
                st.line_dash = canvas::LineDash { segments: &[5.0, 4.0], offset: 0 };
            }
            f.stroke(&line, st);
        };
        draw(&mut f, &self.values, false);
        if !self.dashed.is_empty() {
            draw(&mut f, &self.dashed, true);
        }
        vec![f.into_geometry()]
    }
}

pub fn graph<'a, M: 'a>(g: Graph, w: Length, h: Length) -> Element<'a, M> {
    cv(g).width(w).height(h).into()
}

/// A bar: `frac` of it in `c`, the rest in `bg` (memory composition, disk space).
pub fn bar<'a, M: 'a>(parts: Vec<(f32, Color)>, bg: Color, h: f32, r: f32) -> Element<'a, M> {
    let mut rw = row![];
    let total: f32 = parts.iter().map(|x| x.0.max(0.0)).sum::<f32>().min(1.0);
    for (fr, c) in parts {
        let fp = (fr.max(0.0) * 1000.0).round() as u16;
        if fp == 0 {
            continue;
        }
        rw = rw.push(container(Space::new()).width(Length::FillPortion(fp)).height(Length::Fixed(h)).style(boxed(c, 0.0, None)));
    }
    let rest = ((1.0 - total) * 1000.0).round() as u16;
    if rest > 0 {
        rw = rw.push(Space::new().width(Length::FillPortion(rest)).height(Length::Fixed(h)));
    }
    container(rw).width(Length::Fill).height(Length::Fixed(h)).clip(true).style(boxed(bg, r, None)).into()
}

/// A size in words: 1.2 GB, 340 MB.
pub fn bytes(b: f64) -> String {
    let u = ["B", "KB", "MB", "GB", "TB"];
    let mut v = b.max(0.0);
    let mut i = 0;
    while v >= 1024.0 && i < u.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{} {}", v as u64, u[i])
    } else if v < 10.0 {
        format!("{:.1} {}", v, u[i])
    } else {
        format!("{:.0} {}", v, u[i])
    }
}

