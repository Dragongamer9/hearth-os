//! The few shapes iced's boxes can't make: Firehold's flame (the logo tile), the dashed "Game art" frame, and the
//! guide's dot. Canvas programs: drawn once and cached by iced until their size changes.

use iced::widget::canvas::{self, Frame, Geometry, LineDash, Path, Stroke};
use iced::{mouse, Color, Point, Rectangle, Renderer, Size, Theme};

/// The flame in the logo tile (mockup 1's top left).
pub struct Flame(pub Color);

impl<Message> canvas::Program<Message> for Flame {
    type State = ();
    fn draw(&self, _state: &(), renderer: &Renderer, _theme: &Theme, bounds: Rectangle, _cursor: mouse::Cursor) -> Vec<Geometry> {
        let mut f = Frame::new(renderer, bounds.size());
        let (w, h) = (bounds.width, bounds.height);
        let p = |x: f32, y: f32| Point::new(x * w, y * h);
        let flame = Path::new(|b| {
            b.move_to(p(0.5, 0.18));
            b.bezier_curve_to(p(0.62, 0.36), p(0.74, 0.46), p(0.72, 0.62));
            b.bezier_curve_to(p(0.70, 0.76), p(0.60, 0.84), p(0.5, 0.84));
            b.bezier_curve_to(p(0.38, 0.84), p(0.29, 0.75), p(0.29, 0.63));
            b.bezier_curve_to(p(0.29, 0.52), p(0.36, 0.46), p(0.40, 0.40));
            b.bezier_curve_to(p(0.42, 0.50), p(0.46, 0.54), p(0.50, 0.55));
            b.bezier_curve_to(p(0.48, 0.42), p(0.47, 0.30), p(0.5, 0.18));
            b.close();
        });
        f.stroke(&flame, Stroke::default().with_color(self.0).with_width((w * 0.07).max(1.5)));
        vec![f.into_geometry()]
    }
}

/// A dashed, rounded frame with nothing in it (the hero's "Game art" place).
pub struct Dashed(pub Color, pub f32);

impl<Message> canvas::Program<Message> for Dashed {
    type State = ();
    fn draw(&self, _state: &(), renderer: &Renderer, _theme: &Theme, bounds: Rectangle, _cursor: mouse::Cursor) -> Vec<Geometry> {
        let mut f = Frame::new(renderer, bounds.size());
        let inset = 1.5;
        let r = Path::rounded_rectangle(Point::new(inset, inset), Size::new(bounds.width - 2.0 * inset, bounds.height - 2.0 * inset), self.1.into());
        let seg = [8.0f32, 6.0];
        f.stroke(&r, Stroke { line_dash: LineDash { segments: &seg, offset: 0 }, ..Stroke::default().with_color(self.0).with_width(2.0) });
        vec![f.into_geometry()]
    }
}
