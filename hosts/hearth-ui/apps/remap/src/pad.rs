//! The controller as the board draws it (board 10): its body, bumpers, sticks, D-pad and face buttons, and a label for
//! every changed control ("LB → Shift (add to group)"), lit in the accent when this profile changes it from Everywhere.
//! A press on a control (or its label) picks it; the controller's own button press picks it too (main.rs).

use crate::ir::Ctl;
use crate::Msg;
use iced::widget::canvas::{self, Frame, Geometry, Path, Stroke};
use iced::{mouse, Color, Pixels, Point, Rectangle, Renderer, Size, Theme};

fn hexc(s: u32) -> Color {
    Color::from_rgb8((s >> 16) as u8, (s >> 8) as u8, s as u8)
}
const ACC: u32 = 0xff8a3d;
const LINE: u32 = 0x4a4038;

/// One label: the control, its words (the bold part after the arrow), lit or not.
#[derive(Debug, Clone)]
pub struct Callout {
    pub ctl: Ctl,
    pub name: String,
    pub what: String,
    pub lit: bool,
}

pub struct Pad {
    pub calls: Vec<Callout>,
    pub picked: Option<Ctl>,
    pub lit: Vec<Ctl>,
}

/// Where each control sits (the board's 1060 × 450 card), and where its label goes and its line starts.
fn spot(c: Ctl) -> Option<((f32, f32), (f32, f32), (f32, f32))> {
    // (control centre), (label's top left), (the line's end at the label)
    Some(match c {
        Ctl::Btn(310) => ((405.0, 117.0), (150.0, 44.0), (300.0, 60.0)),
        Ctl::Btn(311) => ((655.0, 117.0), (700.0, 20.0), (700.0, 36.0)),
        Ctl::Btn(307) => ((620.0, 208.0), (760.0, 76.0), (760.0, 90.0)),
        Ctl::Btn(308) => ((650.0, 178.0), (760.0, 30.0), (760.0, 44.0)),
        Ctl::Btn(304) => ((650.0, 238.0), (760.0, 136.0), (760.0, 150.0)),
        Ctl::Btn(305) => ((680.0, 208.0), (760.0, 196.0), (760.0, 208.0)),
        Ctl::LeftStick => ((410.0, 210.0), (96.0, 216.0), (250.0, 230.0)),
        Ctl::RightStick => ((590.0, 280.0), (760.0, 316.0), (760.0, 328.0)),
        Ctl::Dpad => ((485.0, 271.0), (150.0, 366.0), (300.0, 380.0)),
        Ctl::Lt => ((405.0, 104.0), (150.0, 4.0), (250.0, 20.0)),
        Ctl::Rt => ((655.0, 104.0), (860.0, 256.0), (860.0, 270.0)),
        _ => return None,
    })
}

fn text(f: &mut Frame, s: &str, at: Point, size: f32, c: Color, bold: bool) {
    f.fill_text(canvas::Text {
        content: s.to_string(),
        position: at,
        color: c,
        size: Pixels(size),
        font: hearth_ui::ui::weight(if bold { iced::font::Weight::Bold } else { iced::font::Weight::Normal }),
        ..canvas::Text::default()
    });
}
fn width(s: &str, size: f32) -> f32 {
    s.chars().count() as f32 * size * 0.56
}

impl Pad {
    fn k(b: Rectangle) -> (f32, f32, f32) {
        let k = (b.width / 1060.0).min(b.height / 450.0).max(0.3);
        (k, (b.width - 1060.0 * k) / 2.0, (b.height - 450.0 * k) / 2.0)
    }
    fn label_rect(c: &Callout) -> Option<(f32, f32, f32, f32)> {
        let (_, at, _) = spot(c.ctl)?;
        let w = width(&format!("{} → ", c.name), 12.0) + width(&c.what, 12.0) + 22.0;
        Some((at.0, at.1, w, 26.0))
    }
    fn hit(&self, x: f32, y: f32) -> Option<Ctl> {
        for c in &self.calls {
            if let Some((lx, ly, w, h)) = Pad::label_rect(c) {
                if x >= lx && x <= lx + w && y >= ly && y <= ly + h {
                    return Some(c.ctl);
                }
            }
        }
        let all = [Ctl::Btn(310), Ctl::Btn(311), Ctl::Btn(307), Ctl::Btn(308), Ctl::Btn(304), Ctl::Btn(305), Ctl::LeftStick, Ctl::RightStick, Ctl::Dpad];
        for c in all {
            if let Some(((cx, cy), _, _)) = spot(c) {
                let r = match c {
                    Ctl::LeftStick | Ctl::RightStick => 34.0,
                    Ctl::Dpad => 28.0,
                    Ctl::Btn(310) | Ctl::Btn(311) => 40.0,
                    _ => 16.0,
                };
                if (x - cx).powi(2) + (y - cy).powi(2) <= r * r {
                    return Some(c);
                }
            }
        }
        // View, ≡ and the guide button
        if (484.0..=500.0).contains(&x) && (200.0..=214.0).contains(&y) {
            return Some(Ctl::Btn(314));
        }
        if (560.0..=576.0).contains(&x) && (200.0..=214.0).contains(&y) {
            return Some(Ctl::Btn(315));
        }
        None
    }
}

impl canvas::Program<Msg> for Pad {
    type State = ();
    fn update(&self, _s: &mut (), event: &canvas::Event, b: Rectangle, cursor: mouse::Cursor) -> Option<canvas::Action<Msg>> {
        if let canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event {
            let q = cursor.position_in(b)?;
            let (k, ox, oy) = Pad::k(b);
            let c = self.hit((q.x - ox) / k, (q.y - oy) / k)?;
            return Some(canvas::Action::publish(Msg::PickCtl(crate::ir::ctl_id(c))).and_capture());
        }
        None
    }
    fn mouse_interaction(&self, _s: &(), b: Rectangle, cursor: mouse::Cursor) -> mouse::Interaction {
        let Some(q) = cursor.position_in(b) else { return mouse::Interaction::default() };
        let (k, ox, oy) = Pad::k(b);
        if self.hit((q.x - ox) / k, (q.y - oy) / k).is_some() { mouse::Interaction::Pointer } else { mouse::Interaction::default() }
    }
    fn draw(&self, _s: &(), renderer: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        let mut f = Frame::new(renderer, b.size());
        let (k, ox, oy) = Pad::k(b);
        f.translate(iced::Vector::new(ox, oy));
        f.scale(k);
        let lit = |c: Ctl| self.lit.contains(&c) || self.picked == Some(c);
        let stroke = |c: Ctl| Stroke::default().with_color(if lit(c) { hexc(ACC) } else { hexc(LINE) }).with_width(if lit(c) { 2.0 } else { 1.5 });
        // the lines from each control to its label (under everything)
        for c in &self.calls {
            if let Some((from, _, to)) = spot(c.ctl) {
                let col = if c.lit { Color { a: 0.8, ..hexc(ACC) } } else { hexc(0x5a4c42) };
                f.stroke(&Path::line(Point::new(from.0, from.1), Point::new(to.0, to.1)), Stroke::default().with_color(col).with_width(1.5));
            }
        }
        // the body (the board's path, moved by 280, 60)
        let body = Path::new(|p| {
            let q = |x: f32, y: f32| Point::new(280.0 + x, 60.0 + y);
            p.move_to(q(120.0, 70.0));
            p.bezier_curve_to(q(60.0, 70.0), q(34.0, 130.0), q(22.0, 210.0));
            p.bezier_curve_to(q(10.0, 290.0), q(40.0, 340.0), q(84.0, 340.0));
            p.bezier_curve_to(q(124.0, 340.0), q(144.0, 292.0), q(172.0, 268.0));
            p.line_to(q(328.0, 268.0));
            p.bezier_curve_to(q(356.0, 292.0), q(376.0, 340.0), q(416.0, 340.0));
            p.bezier_curve_to(q(460.0, 340.0), q(490.0, 290.0), q(478.0, 210.0));
            p.bezier_curve_to(q(466.0, 130.0), q(440.0, 70.0), q(380.0, 70.0));
            p.close();
        });
        f.fill(&body, hexc(0x1c1816));
        f.stroke(&body, Stroke::default().with_color(hexc(LINE)).with_width(2.0));
        for (c, x) in [(Ctl::Btn(310), 360.0), (Ctl::Btn(311), 610.0)] {
            let r = Path::rounded_rectangle(Point::new(x, 104.0), Size::new(90.0, 26.0), 13.0.into());
            f.fill(&r, hexc(0x2a2420));
            f.stroke(&r, stroke(c));
        }
        for (c, x, y) in [(Ctl::LeftStick, 410.0, 210.0), (Ctl::RightStick, 590.0, 280.0)] {
            let o = Path::circle(Point::new(x, y), 34.0);
            f.fill(&o, hexc(0x141110));
            f.stroke(&o, stroke(c));
            f.fill(&Path::circle(Point::new(x, y), 20.0), hexc(0x2a2420));
        }
        let d = Path::new(|p| {
            let q = |x: f32, y: f32| Point::new(280.0 + x, 60.0 + y);
            p.move_to(q(178.0, 202.0));
            for (dx, dy) in [(18.0, 0.0), (0.0, -18.0), (18.0, 0.0), (0.0, 18.0), (18.0, 0.0), (0.0, 18.0), (-18.0, 0.0), (0.0, 18.0), (-18.0, 0.0), (0.0, -18.0), (-18.0, 0.0)] {
                let _ = (dx, dy);
            }
            let pts = [(196.0, 202.0), (196.0, 184.0), (214.0, 184.0), (214.0, 202.0), (232.0, 202.0), (232.0, 220.0), (214.0, 220.0), (214.0, 238.0), (196.0, 238.0), (196.0, 220.0), (178.0, 220.0)];
            for (x, y) in pts {
                p.line_to(q(x, y));
            }
            p.close();
        });
        f.fill(&d, hexc(0x2a2420));
        f.stroke(&d, stroke(Ctl::Dpad));
        for (c, x, y, l) in [(Ctl::Btn(308), 650.0, 178.0, "Y"), (Ctl::Btn(307), 620.0, 208.0, "X"), (Ctl::Btn(305), 680.0, 208.0, "B"), (Ctl::Btn(304), 650.0, 238.0, "A")] {
            let o = Path::circle(Point::new(x, y), 15.0);
            f.fill(&o, hexc(0x2a2420));
            f.stroke(&o, stroke(c));
            text(&mut f, l, Point::new(x - 4.5, y - 8.0), 13.0, if lit(c) { hexc(ACC) } else { hexc(0xa59a91) }, true);
        }
        let g = Path::circle(Point::new(530.0, 180.0), 14.0);
        f.fill(&g, hexc(0x2a2420));
        f.stroke(&g, Stroke::default().with_color(hexc(LINE)).with_width(1.0));
        for (c, x) in [(Ctl::Btn(314), 484.0), (Ctl::Btn(315), 560.0)] {
            let r = Path::rounded_rectangle(Point::new(x, 202.0), Size::new(16.0, 10.0), 4.0.into());
            f.fill(&r, if lit(c) { hexc(ACC) } else { hexc(0x2a2420) });
        }
        // the labels
        for c in &self.calls {
            let Some((x, y, w, h)) = Pad::label_rect(c) else { continue };
            let r = Path::rounded_rectangle(Point::new(x, y), Size::new(w, h), 8.0.into());
            let on = c.lit || self.picked == Some(c.ctl);
            f.fill(&r, if on { hexc(0x3b2a1d) } else { hexc(0x221d1a) });
            f.stroke(&r, Stroke::default().with_color(if on { hexc(ACC) } else { hexc(0x3a322c) }).with_width(1.0));
            let head = format!("{} → ", c.name);
            text(&mut f, &head, Point::new(x + 9.0, y + 6.0), 12.0, hexc(0xf1e9e2), false);
            text(&mut f, &c.what, Point::new(x + 9.0 + width(&head, 12.0), y + 6.0), 12.0, hexc(0xf1e9e2), on);
        }
        vec![f.into_geometry()]
    }
}
