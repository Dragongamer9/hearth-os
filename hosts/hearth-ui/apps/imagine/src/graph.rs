//! The node graph (board 06's middle): what ComfyUI runs, drawn as its steps (Load model, Describe it, Leave out,
//! Empty picture, Look, Make, Turn into pictures, and Bigger when it's added) with the wires between them in the colour
//! of what flows (the model purple, the text reader yellow, the picture maker blue, words orange, the picture pink, the
//! look green). The step ComfyUI is on is outlined in the accent with RUNNING and how far it is. Steps are dragged to
//! move them, the dotted ground to look around, the wheel (or − and +) zooms.

use hearth_ui::Pal;
use iced::widget::canvas::{self, Frame, Geometry, Path, Stroke, Text};
use iced::{mouse, Color, Font, Pixels, Point, Rectangle, Renderer, Size, Theme, Vector};

#[derive(Debug, Clone)]
pub struct Node {
    pub id: &'static str,
    pub title: String,
    pub color: Color,
    /// where it is, in the board's units (as drawn at 86 %)
    pub x: f32,
    pub y: f32,
    pub w: f32,
    /// its words: (text, in the text colour)
    pub lines: Vec<(String, bool)>,
    /// what comes out of it, each with its dot's colour
    pub ports: Vec<(String, Color)>,
    /// (step, of) while ComfyUI runs it
    pub running: Option<(u32, u32)>,
}

impl Node {
    pub fn h(&self) -> f32 {
        30.0 + 16.0 + (self.lines.len() + self.ports.len()).max(1) as f32 * 21.6 + if self.running.is_some() { 30.0 } else { 0.0 }
    }
}

/// A wire: from a node (at this height in it) to another (at this height), in a colour.
#[derive(Debug, Clone)]
pub struct Wire {
    pub from: &'static str,
    pub fy: f32,
    pub to: &'static str,
    pub ty: f32,
    pub color: Color,
}

pub struct Graph<'a, M> {
    pub nodes: &'a [Node],
    pub wires: &'a [Wire],
    pub zoom: f32,
    pub pan: Vector,
    pub p: Pal,
    pub on_move: fn(&'static str, f32, f32) -> M,
    pub on_pan: fn(f32, f32) -> M,
    pub on_zoom: fn(f32) -> M,
}

#[derive(Default)]
pub struct St {
    drag: Option<(Option<&'static str>, Point)>,
}

fn txt(f: &mut Frame, s: &str, at: Point, size: f32, c: Color, bold: bool) {
    let font = if bold { Font { weight: iced::font::Weight::Semibold, ..hearth_ui::app::INTER } } else { hearth_ui::app::INTER };
    f.fill_text(Text { content: s.to_string(), position: at, color: c, size: Pixels(size), font, ..Text::default() });
}

impl<'a, M> Graph<'a, M> {
    /// How big it's drawn: the board's own size at 86 %.
    fn s(&self) -> f32 {
        self.zoom / 0.86
    }
    fn at(&self, x: f32, y: f32) -> Point {
        Point::new(x * self.s() + self.pan.x, y * self.s() + self.pan.y)
    }
    fn hit(&self, pt: Point) -> Option<&'static str> {
        self.nodes.iter().rev().find(|n| {
            let a = self.at(n.x, n.y);
            pt.x >= a.x && pt.y >= a.y && pt.x <= a.x + n.w * self.s() && pt.y <= a.y + n.h() * self.s()
        }).map(|n| n.id)
    }
}

impl<'a, M: Clone> canvas::Program<M> for Graph<'a, M> {
    type State = St;

    fn update(&self, st: &mut St, event: &canvas::Event, b: Rectangle, cursor: mouse::Cursor) -> Option<canvas::Action<M>> {
        match event {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let pt = cursor.position_in(b)?;
                st.drag = Some((self.hit(pt), pt));
                Some(canvas::Action::request_redraw().and_capture())
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let (which, last) = st.drag?;
                let pt = cursor.position_in(b).unwrap_or(last);
                let (dx, dy) = (pt.x - last.x, pt.y - last.y);
                if dx.abs() + dy.abs() < 0.5 {
                    return None;
                }
                st.drag = Some((which, pt));
                Some(canvas::Action::publish(match which {
                    Some(id) => (self.on_move)(id, dx / self.s(), dy / self.s()),
                    None => (self.on_pan)(dx, dy),
                }).and_capture())
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                st.drag.take()?;
                Some(canvas::Action::request_redraw())
            }
            canvas::Event::Mouse(mouse::Event::WheelScrolled { delta }) if cursor.is_over(b) => {
                let y = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => *y,
                    mouse::ScrollDelta::Pixels { y, .. } => *y / 40.0,
                };
                Some(canvas::Action::publish((self.on_zoom)((self.zoom * 1.1f32.powf(y)).clamp(0.3, 2.0))).and_capture())
            }
            _ => None,
        }
    }

    fn draw(&self, _st: &St, r: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        let p = self.p;
        let mut f = Frame::new(r, b.size());
        // the dotted ground
        f.fill(&Path::rectangle(Point::ORIGIN, b.size()), Pal::mix(p.bg, Color::BLACK, 0.12));
        let step = (22.0 * self.s()).max(8.0);
        let dot = Pal::mix(p.bg, p.line, 0.75);
        let ox = self.pan.x.rem_euclid(step);
        let oy = self.pan.y.rem_euclid(step);
        let mut y = oy;
        while y < b.height {
            let mut x = ox;
            while x < b.width {
                f.fill(&Path::rectangle(Point::new(x, y), Size::new(1.2, 1.2)), dot);
                x += step;
            }
            y += step;
        }
        // the wires
        for wr in self.wires {
            let (Some(a), Some(c)) = (self.nodes.iter().find(|n| n.id == wr.from), self.nodes.iter().find(|n| n.id == wr.to)) else { continue };
            let p0 = self.at(a.x + a.w, a.y + wr.fy);
            let p3 = self.at(c.x, c.y + wr.ty);
            let dx = if p3.x > p0.x { ((p3.x - p0.x) * 0.5).max(60.0 * self.s()) } else { 90.0 * self.s() };
            let path = Path::new(|pb| {
                pb.move_to(p0);
                pb.bezier_curve_to(Point::new(p0.x + dx, p0.y), Point::new(p3.x - dx, p3.y), p3);
            });
            f.stroke(&path, Stroke::default().with_color(wr.color).with_width(2.5 * self.s().max(0.6)));
        }
        // the steps
        for n in self.nodes {
            let o = self.at(n.x, n.y);
            let (w, h) = (n.w * self.s(), n.h() * self.s());
            let z = self.s();
            if n.running.is_some() {
                f.fill(&Path::rounded_rectangle(Point::new(o.x - 3.0, o.y - 3.0), Size::new(w + 6.0, h + 6.0), iced::border::Radius::from(12.0 * z + 3.0)), Pal::a(p.accent, 0.33));
            }
            let body = Path::rounded_rectangle(o, Size::new(w, h), iced::border::Radius::from(10.0 * z));
            f.fill(&body, p.panel);
            f.stroke(&body, Stroke::default().with_color(if n.running.is_some() { p.accent } else { p.line }).with_width(1.0));
            f.fill(&Path::rectangle(Point::new(o.x, o.y + 30.0 * z), Size::new(w, 1.0)), p.line);
            f.fill(&Path::rounded_rectangle(Point::new(o.x + 10.0 * z, o.y + 7.0 * z), Size::new(16.0 * z, 16.0 * z), iced::border::Radius::from(4.0 * z)), Pal::a(n.color, 0.85));
            txt(&mut f, &n.title, Point::new(o.x + 34.0 * z, o.y + 8.0 * z), 12.0 * z, p.text, true);
            if n.running.is_some() {
                let tw = 62.0 * z;
                let tag = Path::rounded_rectangle(Point::new(o.x + w - tw - 10.0 * z, o.y + 8.0 * z), Size::new(tw, 15.0 * z), iced::border::Radius::from(4.0 * z));
                f.fill(&tag, p.accent);
                txt(&mut f, "RUNNING", Point::new(o.x + w - tw - 3.0 * z, o.y + 10.0 * z), 9.0 * z, p.ink, true);
            }
            let mut ly = o.y + 38.0 * z;
            for (s, strong) in &n.lines {
                txt(&mut f, s, Point::new(o.x + 10.0 * z, ly), 12.0 * z, if *strong { p.text } else { p.muted }, false);
                ly += 21.6 * z;
            }
            if let Some((at, of)) = n.running {
                let bw = w - 20.0 * z;
                f.fill(&Path::rounded_rectangle(Point::new(o.x + 10.0 * z, ly + 4.0 * z), Size::new(bw, 6.0 * z), iced::border::Radius::from(3.0 * z)), p.line);
                f.fill(&Path::rounded_rectangle(Point::new(o.x + 10.0 * z, ly + 4.0 * z), Size::new(bw * at as f32 / of.max(1) as f32, 6.0 * z), iced::border::Radius::from(3.0 * z)), p.accent);
                ly += 18.0 * z;
                txt(&mut f, &format!("step {at} of {of}"), Point::new(o.x + 10.0 * z, ly), 12.0 * z, p.muted, false);
                ly += 21.6 * z;
            }
            for (s, c) in &n.ports {
                txt(&mut f, s, Point::new(o.x + 10.0 * z, ly), 12.0 * z, p.muted, false);
                f.fill(&Path::circle(Point::new(o.x + w - 14.0 * z, ly + 7.0 * z), 4.5 * z), *c);
                ly += 21.6 * z;
            }
        }
        vec![f.into_geometry()]
    }

    fn mouse_interaction(&self, st: &St, b: Rectangle, c: mouse::Cursor) -> mouse::Interaction {
        if st.drag.is_some() {
            return mouse::Interaction::Grabbing;
        }
        match c.position_in(b) {
            Some(pt) if self.hit(pt).is_some() => mouse::Interaction::Grab,
            _ => mouse::Interaction::default(),
        }
    }
}
