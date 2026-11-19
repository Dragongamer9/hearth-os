//! CaptureGlyph.qml in Rust: Capture's tool pictures, drawn (one colour, rounded 2 px lines in a 24 × 24 box, like
//! the rest of Mantel's icons), plus the few Hearth icons Capture's buttons use (Icon.qml's cam, copy, doc, folder,
//! check, undo, close, kbd, open, image, eye).
//! Names: arrow box pen highlighter text blur pixelate steps crop area window screen all qr scan record timer pointer
//! redo scroll share phone, and the icons.

use iced::widget::canvas::{self, Frame, Geometry, LineCap, LineDash, LineJoin, Path, Stroke};
use iced::{mouse, Color, Element, Length, Point, Radians, Rectangle, Renderer, Size, Theme};

pub struct Glyph {
    pub name: &'static str,
    pub c: Color,
    /// The colour behind it (Numbered steps' number is cut out of its disc in this colour).
    pub cut: Color,
}

impl<M> canvas::Program<M> for Glyph {
    type State = ();
    fn draw(&self, _s: &(), renderer: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        let mut f = Frame::new(renderer, b.size());
        paint(&mut f, self.name, self.c, self.cut, b.width.min(b.height) / 24.0);
        vec![f.into_geometry()]
    }
}

pub fn glyph<'a, M: 'a>(name: &'static str, c: Color, size: f32) -> Element<'a, M> {
    glyph_on(name, c, Color::TRANSPARENT, size)
}

pub fn glyph_on<'a, M: 'a>(name: &'static str, c: Color, cut: Color, size: f32) -> Element<'a, M> {
    canvas::Canvas::new(Glyph { name, c, cut }).width(Length::Fixed(size)).height(Length::Fixed(size)).into()
}

/// Draws glyph NAME at scale K (24 units = K × 24 px) with its top left at the frame's origin.
pub fn paint(f: &mut Frame, name: &str, c: Color, cut: Color, k: f32) {
    let p = |x: f32, y: f32| Point::new(x * k, y * k);
    let st = |w: f32| Stroke::default().with_color(c).with_width(w * k).with_line_cap(LineCap::Round).with_line_join(LineJoin::Round);
    let line = |f: &mut Frame, pts: &[(f32, f32)]| {
        if pts.len() < 2 {
            return;
        }
        let path = Path::new(|b| {
            b.move_to(p(pts[0].0, pts[0].1));
            for q in &pts[1..] {
                b.line_to(p(q.0, q.1));
            }
        });
        f.stroke(&path, st(2.0));
    };
    let closed = |f: &mut Frame, pts: &[(f32, f32)]| {
        let path = Path::new(|b| {
            b.move_to(p(pts[0].0, pts[0].1));
            for q in &pts[1..] {
                b.line_to(p(q.0, q.1));
            }
            b.close();
        });
        f.stroke(&path, st(2.0));
    };
    let rect = |f: &mut Frame, x: f32, y: f32, w: f32, h: f32| f.stroke(&Path::rectangle(p(x, y), Size::new(w * k, h * k)), st(2.0));
    let fill_rect = |f: &mut Frame, x: f32, y: f32, w: f32, h: f32, col: Color| f.fill_rectangle(p(x, y), Size::new(w * k, h * k), col);
    let ring = |f: &mut Frame, x: f32, y: f32, r: f32, col: Color| f.stroke(&Path::circle(p(x, y), r * k), st(2.0).with_color(col));
    let disc = |f: &mut Frame, x: f32, y: f32, r: f32, col: Color| f.fill(&Path::circle(p(x, y), r * k), col);
    let arc = |f: &mut Frame, x: f32, y: f32, r: f32, a0: f32, a1: f32| {
        let path = Path::new(|b| {
            b.arc(canvas::path::Arc { center: p(x, y), radius: r * k, start_angle: Radians(a0), end_angle: Radians(a1) });
        });
        f.stroke(&path, st(2.0));
    };
    let faded = |a: f32| Color { a: c.a * a, ..c };
    let pi = std::f32::consts::PI;
    match name {
        "arrow" => {
            line(f, &[(5.0, 19.0), (18.0, 6.0)]);
            line(f, &[(10.0, 6.0), (18.0, 6.0), (18.0, 14.0)]);
        }
        "box" => rect(f, 4.0, 6.0, 16.0, 12.0),
        "pen" => {
            line(f, &[(5.0, 19.0), (6.0, 15.0), (16.0, 5.0), (19.0, 8.0), (9.0, 18.0), (5.0, 19.0)]);
            line(f, &[(14.0, 7.0), (17.0, 10.0)]);
        }
        "highlighter" => {
            line(f, &[(7.0, 14.0), (14.0, 5.0), (19.0, 10.0), (12.0, 19.0)]);
            line(f, &[(7.0, 14.0), (12.0, 19.0)]);
            fill_rect(f, 3.0, 19.0, 18.0, 3.0, faded(0.45));
        }
        "text" => {
            line(f, &[(6.0, 6.0), (18.0, 6.0)]);
            line(f, &[(12.0, 6.0), (12.0, 19.0)]);
            line(f, &[(9.0, 19.0), (15.0, 19.0)]);
        }
        "blur" => {
            for i in 0..3 {
                ring(f, 12.0, 12.0, 3.0 + i as f32 * 3.5, faded(1.0 - i as f32 * 0.3));
            }
        }
        "pixelate" => {
            for y in 0..4 {
                for x in 0..4 {
                    if (x + y) % 2 == 0 {
                        fill_rect(f, 4.0 + x as f32 * 4.0, 4.0 + y as f32 * 4.0, 4.0, 4.0, c);
                    }
                }
            }
            rect(f, 4.0, 4.0, 16.0, 16.0);
        }
        "steps" => {
            disc(f, 12.0, 12.0, 8.0, c);
            // The "1" cut out of the disc (destination-out in Mantel): drawn in the colour behind it.
            let back = if cut.a > 0.0 { cut } else { Color::BLACK };
            let path = Path::new(|b| {
                b.move_to(p(10.5, 9.0));
                b.line_to(p(12.5, 7.5));
                b.line_to(p(12.5, 16.5));
            });
            f.stroke(&path, st(2.0).with_color(back));
        }
        "crop" => {
            line(f, &[(7.0, 3.0), (7.0, 17.0), (21.0, 17.0)]);
            line(f, &[(3.0, 7.0), (17.0, 7.0), (17.0, 21.0)]);
        }
        "area" => {
            let segs = [2.5 * k, 2.5 * k];
            let mut s = Stroke::default().with_color(c).with_width(2.0 * k).with_line_cap(LineCap::Butt);
            s.line_dash = LineDash { segments: &segs, offset: 0 };
            f.stroke(&Path::rectangle(p(4.0, 5.0), Size::new(16.0 * k, 14.0 * k)), s);
            fill_rect(f, 2.5, 3.5, 3.0, 3.0, c);
            fill_rect(f, 18.5, 17.5, 3.0, 3.0, c);
        }
        "window" => {
            rect(f, 3.0, 5.0, 18.0, 14.0);
            line(f, &[(3.0, 9.0), (21.0, 9.0)]);
            fill_rect(f, 5.0, 6.4, 1.4, 1.4, c);
            fill_rect(f, 7.6, 6.4, 1.4, 1.4, c);
        }
        "screen" => {
            rect(f, 3.0, 4.0, 18.0, 12.0);
            line(f, &[(9.0, 20.0), (15.0, 20.0)]);
            line(f, &[(12.0, 16.0), (12.0, 20.0)]);
        }
        "all" => {
            rect(f, 2.0, 5.0, 11.0, 8.0);
            rect(f, 11.0, 9.0, 11.0, 8.0);
            line(f, &[(5.0, 19.0), (9.0, 19.0)]);
            line(f, &[(14.0, 20.0), (19.0, 20.0)]);
        }
        "qr" => {
            rect(f, 3.0, 3.0, 7.0, 7.0);
            rect(f, 14.0, 3.0, 7.0, 7.0);
            rect(f, 3.0, 14.0, 7.0, 7.0);
            for (x, y, w, h) in [(5.5, 5.5, 2.0, 2.0), (16.5, 5.5, 2.0, 2.0), (5.5, 16.5, 2.0, 2.0), (14.0, 14.0, 3.0, 3.0), (18.0, 18.0, 3.0, 3.0), (18.0, 14.0, 3.0, 2.0), (14.0, 19.0, 2.0, 2.0)] {
                fill_rect(f, x, y, w, h, c);
            }
        }
        "scan" => {
            line(f, &[(3.0, 8.0), (3.0, 3.0), (8.0, 3.0)]);
            line(f, &[(16.0, 3.0), (21.0, 3.0), (21.0, 8.0)]);
            line(f, &[(21.0, 16.0), (21.0, 21.0), (16.0, 21.0)]);
            line(f, &[(8.0, 21.0), (3.0, 21.0), (3.0, 16.0)]);
            line(f, &[(7.0, 9.0), (17.0, 9.0)]);
            line(f, &[(7.0, 12.0), (17.0, 12.0)]);
            line(f, &[(7.0, 15.0), (13.0, 15.0)]);
        }
        "record" => {
            ring(f, 12.0, 12.0, 8.5, c);
            disc(f, 12.0, 12.0, 4.5, c);
        }
        "timer" => {
            ring(f, 12.0, 13.0, 8.0, c);
            line(f, &[(12.0, 13.0), (12.0, 9.0)]);
            line(f, &[(10.0, 3.0), (14.0, 3.0)]);
        }
        "pointer" => closed(f, &[(6.0, 3.0), (18.0, 14.0), (12.5, 14.5), (15.5, 20.5), (13.0, 21.5), (10.0, 15.5), (6.0, 19.0)]),
        "scroll" => {
            rect(f, 6.0, 2.0, 12.0, 20.0);
            line(f, &[(12.0, 7.0), (12.0, 15.0)]);
            line(f, &[(9.0, 12.0), (12.0, 15.0), (15.0, 12.0)]);
        }
        "share" => {
            ring(f, 6.0, 12.0, 2.4, c);
            ring(f, 18.0, 6.0, 2.4, c);
            ring(f, 18.0, 18.0, 2.4, c);
            line(f, &[(8.2, 10.9), (15.8, 7.1)]);
            line(f, &[(8.2, 13.1), (15.8, 16.9)]);
        }
        "phone" => {
            let path = Path::new(|b| b.rounded_rectangle(p(7.0, 2.5), Size::new(10.0 * k, 19.0 * k), (2.0 * k).into()));
            f.stroke(&path, st(2.0));
            line(f, &[(10.5, 18.5), (13.5, 18.5)]);
        }
        "redo" => {
            arc(f, 11.0, 13.0, 6.0, pi * 1.1, pi * 2.1);
            line(f, &[(17.0, 7.0), (17.0, 12.0), (12.0, 12.0)]);
        }
        // ---- Hearth icons (Icon.qml) used by Capture's buttons
        "undo" => {
            arc(f, 13.0, 13.0, 6.0, pi * 0.9, pi * 1.9);
            line(f, &[(7.0, 7.0), (7.0, 12.0), (12.0, 12.0)]);
        }
        "cam" => {
            let path = Path::new(|b| b.rounded_rectangle(p(3.0, 7.0), Size::new(18.0 * k, 13.0 * k), (2.5 * k).into()));
            f.stroke(&path, st(2.0));
            line(f, &[(8.0, 7.0), (9.5, 4.5), (14.5, 4.5), (16.0, 7.0)]);
            ring(f, 12.0, 13.5, 3.5, c);
        }
        "copy" => {
            let a = Path::new(|b| b.rounded_rectangle(p(8.0, 8.0), Size::new(12.0 * k, 12.0 * k), (2.0 * k).into()));
            f.stroke(&a, st(2.0));
            line(f, &[(5.0, 15.0), (4.0, 15.0), (4.0, 4.0), (15.0, 4.0), (15.0, 5.0)]);
        }
        "doc" => {
            closed(f, &[(6.0, 3.0), (14.0, 3.0), (19.0, 8.0), (19.0, 21.0), (6.0, 21.0)]);
            line(f, &[(14.0, 3.0), (14.0, 8.0), (19.0, 8.0)]);
            line(f, &[(9.0, 13.0), (16.0, 13.0)]);
            line(f, &[(9.0, 17.0), (16.0, 17.0)]);
        }
        "folder" => closed(f, &[(3.0, 6.0), (9.0, 6.0), (11.0, 8.0), (21.0, 8.0), (21.0, 19.0), (3.0, 19.0)]),
        "check" => line(f, &[(5.0, 12.0), (10.0, 17.0), (19.0, 7.0)]),
        "close" => {
            line(f, &[(6.0, 6.0), (18.0, 18.0)]);
            line(f, &[(18.0, 6.0), (6.0, 18.0)]);
        }
        "kbd" => {
            let path = Path::new(|b| b.rounded_rectangle(p(2.0, 6.0), Size::new(20.0 * k, 12.0 * k), (2.0 * k).into()));
            f.stroke(&path, st(2.0));
            for (x, y) in [(6.0, 10.0), (10.0, 10.0), (14.0, 10.0), (18.0, 10.0)] {
                fill_rect(f, x - 0.8, y - 0.8, 1.6, 1.6, c);
            }
            line(f, &[(8.0, 14.5), (16.0, 14.5)]);
        }
        "open" => {
            line(f, &[(13.0, 4.0), (20.0, 4.0), (20.0, 11.0)]);
            line(f, &[(20.0, 4.0), (11.0, 13.0)]);
            line(f, &[(17.0, 14.0), (17.0, 20.0), (4.0, 20.0), (4.0, 7.0), (10.0, 7.0)]);
        }
        "image" => {
            rect(f, 3.0, 4.0, 18.0, 16.0);
            line(f, &[(3.0, 17.0), (9.0, 11.0), (14.0, 16.0), (17.0, 13.0), (21.0, 17.0)]);
            ring(f, 15.5, 8.5, 1.6, c);
        }
        "eye" => {
            let path = Path::new(|b| {
                b.move_to(p(2.0, 12.0));
                b.quadratic_curve_to(p(12.0, 2.0), p(22.0, 12.0));
                b.quadratic_curve_to(p(12.0, 22.0), p(2.0, 12.0));
            });
            f.stroke(&path, st(2.0));
            ring(f, 12.0, 12.0, 3.0, c);
        }
        _ => disc(f, 12.0, 12.0, 3.0, c),
    }
}
