//! The live graphs (the right-hand column of every tab) and the frame mark.
//!
//! A graph is a canvas with two cached layers: what stands still (the faint lines across) is drawn again only when its
//! size changes, and the line with the colour under it only when a new sample came in (its generation). Scrolling or
//! hovering draws neither again.

use std::cell::Cell;

use hearth_ui::Pal;
use iced::widget::canvas::{self, Cache, Frame, Geometry, LineCap, LineJoin, Path, Stroke};
use iced::{mouse, Color, Point, Rectangle, Renderer, Size, Theme};

pub const HIST: usize = 60;

pub struct Spark {
    pub values: Vec<f32>,
    /// > 0: the top of the scale; 0: bytes a second (at least 1 MB/s, room above); < 0: small numbers (ms).
    pub top: f32,
    pub color: Color,
    pub grid: Color,
    pub gen: u64,
}

#[derive(Default)]
pub struct SparkState {
    still: Cache,
    line: Cache,
    size: Cell<Option<Size>>,
    gen: Cell<u64>,
    color: Cell<Option<Color>>,
}

impl<M> canvas::Program<M> for Spark {
    type State = SparkState;
    fn draw(&self, st: &SparkState, renderer: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        let size = b.size();
        if st.size.get() != Some(size) {
            st.size.set(Some(size));
            st.still.clear();
            st.line.clear();
        }
        if st.gen.get() != self.gen || st.color.get() != Some(self.color) {
            st.gen.set(self.gen);
            st.color.set(Some(self.color));
            st.line.clear();
        }
        let still = st.still.draw(renderer, size, |f: &mut Frame| {
            for k in [0.25f32, 0.5, 0.75] {
                let y = (size.height * k).round() + 0.5;
                f.stroke(&Path::line(Point::new(0.0, y), Point::new(size.width, y)), Stroke::default().with_color(self.grid).with_width(1.0));
            }
        });
        let line = st.line.draw(renderer, size, |f: &mut Frame| self.paint(f, size));
        vec![still, line]
    }
}

impl Spark {
    fn paint(&self, f: &mut Frame, s: Size) {
        let a = &self.values;
        if a.is_empty() {
            return;
        }
        let peak = a.iter().cloned().fold(0.0f32, f32::max);
        let mx = if self.top > 0.0 { self.top } else if self.top < 0.0 { peak.max(1.0) * 1.15 } else { peak.max(1_048_576.0) * 1.15 };
        let step = s.width / (HIST - 1) as f32;
        let xs = |i: usize| s.width - (a.len() - 1 - i) as f32 * step;
        let ys = |v: f32| s.height - (v.max(0.0) / mx).min(1.0) * (s.height - 4.0) - 1.0;
        if a.len() >= 2 {
            let area = Path::new(|p| {
                p.move_to(Point::new(xs(0), s.height));
                for (i, v) in a.iter().enumerate() {
                    p.line_to(Point::new(xs(i), ys(*v)));
                }
                p.line_to(Point::new(xs(a.len() - 1), s.height));
                p.close();
            });
            f.fill(&area, Pal::a(self.color, 0.18));
        }
        let line = Path::new(|p| {
            p.move_to(Point::new(xs(0), ys(a[0])));
            for (i, v) in a.iter().enumerate().skip(1) {
                p.line_to(Point::new(xs(i), ys(*v)));
            }
        });
        f.stroke(&line, Stroke::default().with_color(self.color).with_width(2.0).with_line_cap(LineCap::Round).with_line_join(LineJoin::Round));
    }
}

/// A 1 px canvas in the title bar: every frame the window draws tells the sampler the window is seen (feed::drawn).
pub struct FrameMark;

impl<M> canvas::Program<M> for FrameMark {
    type State = ();
    fn draw(&self, _s: &(), _r: &Renderer, _t: &Theme, _b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        crate::feed::drawn();
        hearth_ui::app::frame_tick();
        vec![]
    }
}
