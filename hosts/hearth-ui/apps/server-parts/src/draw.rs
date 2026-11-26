//! The server boards' own small parts (server.css: .kpi, .st status dots, .mini bars, .box, .alert; the boards' svg
//! data-line and data-bars graphs), drawn in the person's look.

use hearth_oss::w;
use hearth_ui::ui::{self, t, tb};
use hearth_ui::Pal;
use iced::widget::canvas::{self, Frame, Geometry, LineCap, LineJoin, Path, Stroke};
use iced::widget::text::Wrapping;
use iced::widget::{column, container, row, Space};
use iced::{mouse, Color, Element, Length, Padding, Point, Rectangle, Renderer, Theme};

pub type E<'a, M> = Element<'a, M>;

pub fn pad(t: f32, r: f32, b: f32, l: f32) -> Padding {
    Padding { top: t, right: r, bottom: b, left: l }
}

/// A health: "ok" green, "warn" yellow, "bad" red, "off" grey.
pub fn tone(state: &str) -> Color {
    match state {
        "warn" => w::yellow(),
        "bad" => w::red(),
        "off" | "" => Color::from_rgb8(0x6d, 0x64, 0x5d),
        _ => w::green(),
    }
}

/// .st: a status dot (10 px; the TV's 14).
pub fn st<'a, M: 'a>(state: &str, d: f32) -> E<'a, M> {
    w::dot(tone(state), d)
}

/// A line graph of 0..1 values (the boards' data-line svgs): the line, the area under it faintly, a grid if asked.
pub struct Spark {
    pub vals: Vec<f32>,
    pub color: Color,
    pub fill: bool,
    pub grid: bool,
}

impl<M> canvas::Program<M> for Spark {
    type State = ();
    fn draw(&self, _s: &(), r: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        let mut f = Frame::new(r, b.size());
        if self.grid {
            for i in 1..4 {
                let y = b.height * i as f32 / 4.0;
                f.stroke(&Path::line(Point::new(0.0, y), Point::new(b.width, y)), Stroke::default().with_color(Color::from_rgba(1.0, 1.0, 1.0, 0.06)).with_width(1.0));
            }
        }
        let n = self.vals.len();
        if n >= 2 {
            let x = |i: usize| i as f32 * b.width / (n - 1) as f32;
            let y = |v: f32| b.height - 2.0 - v.clamp(0.0, 1.0) * (b.height - 4.0);
            if self.fill {
                let area = Path::new(|p| {
                    p.move_to(Point::new(0.0, b.height));
                    for (i, v) in self.vals.iter().enumerate() {
                        p.line_to(Point::new(x(i), y(*v)));
                    }
                    p.line_to(Point::new(b.width, b.height));
                    p.close();
                });
                f.fill(&area, Pal::a(self.color, 0.22));
            }
            let line = Path::new(|p| {
                p.move_to(Point::new(0.0, y(self.vals[0])));
                for (i, v) in self.vals.iter().enumerate().skip(1) {
                    p.line_to(Point::new(x(i), y(*v)));
                }
            });
            f.stroke(&line, Stroke::default().with_color(self.color).with_width(2.0).with_line_join(LineJoin::Round).with_line_cap(LineCap::Round));
        }
        vec![f.into_geometry()]
    }
}

pub fn spark<'a, M: 'a>(vals: Vec<f32>, color: Color, fill: bool, w: Length, h: f32) -> E<'a, M> {
    canvas::Canvas::new(Spark { vals, color, fill, grid: false }).width(w).height(Length::Fixed(h)).into()
}
pub fn spark_grid<'a, M: 'a>(vals: Vec<f32>, color: Color, w: Length, h: f32) -> E<'a, M> {
    canvas::Canvas::new(Spark { vals, color, fill: true, grid: true }).width(w).height(Length::Fixed(h)).into()
}
/// The boards' data-bars (the power graphs).
pub fn bars<'a, M: 'a>(p: &Pal, vals: Vec<f32>, w: Length, h: f32) -> E<'a, M> {
    canvas::Canvas::new(w::Bars { vals, color: p.accent, head: None, head_color: p.accent, bright_over: 0.55, centred: false }).width(w).height(Length::Fixed(h)).into()
}
/// A ring (the TV's "12 of 13 healthy").
pub fn ring<'a, M: 'a>(p: &Pal, frac: f32, color: Color, size: f32, width: f32, inner: E<'a, M>) -> E<'a, M> {
    let c = canvas::Canvas::new(w::Ring { frac, color, track: p.line, width }).width(Length::Fixed(size)).height(Length::Fixed(size));
    iced::widget::stack![c, container(inner).center_x(Length::Fixed(size)).center_y(Length::Fixed(size))].into()
}

/// .kpi: its name in small capitals, its value, then whatever's under it.
pub fn kpi<'a, M: 'a>(p: &Pal, k: &str, v: String, of: String, under: E<'a, M>) -> E<'a, M> {
    let mut val = row![tb(v, 22.0, p.text).wrapping(Wrapping::None)].spacing(5.0).align_y(iced::alignment::Vertical::Bottom);
    if !of.is_empty() {
        val = val.push(container(t(of, 13.0, p.muted).wrapping(Wrapping::None)).padding(pad(0.0, 0.0, 3.0, 0.0)));
    }
    container(column![tb(k.to_uppercase(), 11.0, p.muted).wrapping(Wrapping::None), val, under].spacing(4.0))
        .padding(pad(12.0, 16.0, 12.0, 16.0))
        .width(Length::Fill)
        .style(ui::boxed(p.panel, p.r(12.0), Some((p.line, 1.0))))
        .into()
}

/// .box: a panel2 box.
pub fn bx<'a, M: 'a>(p: &Pal, inner: impl Into<E<'a, M>>) -> container::Container<'a, M> {
    container(inner).padding(pad(12.0, 14.0, 12.0, 14.0)).width(Length::Fill).style(ui::boxed(p.panel2, p.r(10.0), Some((p.line, 1.0))))
}

/// .alert (warn), .alert.bad, .alert.ok.
pub fn alert<'a, M: 'a>(p: &Pal, kind: &str, inner: impl Into<E<'a, M>>) -> container::Container<'a, M> {
    let (bg, line) = match kind {
        "bad" => (Color::from_rgb8(0x2a, 0x14, 0x14), Color::from_rgb8(0x6a, 0x2a, 0x2a)),
        "ok" => (Color::from_rgb8(0x18, 0x23, 0x1a), Color::from_rgb8(0x3b, 0x5a, 0x2e)),
        _ => (Color::from_rgb8(0x2a, 0x1e, 0x12), Color::from_rgb8(0x6a, 0x4a, 0x1e)),
    };
    let bg = if p.dark { bg } else { Pal::mix(p.panel, bg, 0.25) };
    container(inner).padding(pad(14.0, 16.0, 14.0, 16.0)).width(Length::Fill).style(ui::boxed(bg, p.r(12.0), Some((line, 1.0))))
}

/// .mini: a small bar 70 px wide.
pub fn mini<'a, M: 'a>(p: &Pal, frac: f32) -> E<'a, M> {
    w::meter(p, frac, None, 70.0, 4.0)
}

/// A row: words on the left, something on the right (.row).
pub fn lr<'a, M: 'a>(left: impl Into<E<'a, M>>, right: impl Into<E<'a, M>>) -> E<'a, M> {
    row![left.into(), Space::new().width(Length::Fill), right.into()].spacing(10.0).align_y(iced::Center).into()
}

/// A thin line under a row.
pub fn under<'a, M: 'a>(p: &Pal, inner: impl Into<E<'a, M>>, last: bool) -> E<'a, M> {
    let line = if last { Color::TRANSPARENT } else { Pal::mix(p.panel, p.line, 0.6) };
    column![container(inner).padding(pad(8.0, 0.0, 8.0, 0.0)), container(Space::new()).width(Length::Fill).height(Length::Fixed(1.0)).style(ui::boxed(line, 0.0, None))].into()
}

/// A colour from "#rrggbb".
pub fn hex(s: &str) -> Color {
    w::hex(s)
}
