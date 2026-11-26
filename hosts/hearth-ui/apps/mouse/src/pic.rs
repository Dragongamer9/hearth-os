//! The mouse as the board draws it (board 11): its body, the wheel, the DPI button, the side buttons, the logo and the
//! bottom strip in their lights' colours, and a numbered pin on each button. A press on a pin picks it; pressing the
//! mouse's own buttons over the picture picks them too (left 1, right 2, wheel 3, forward 5, back 6).

use crate::Msg;
use iced::widget::canvas::{self, Frame, Geometry, Path, Stroke};
use iced::{mouse, Color, Pixels, Point, Rectangle, Renderer, Size, Theme};

fn hexc(s: u32) -> Color {
    Color::from_rgb8((s >> 16) as u8, (s >> 8) as u8, s as u8)
}

pub struct Pic {
    pub picked: String,
    pub logo: Option<[u8; 3]>,
    pub strip: Option<[u8; 3]>,
    pub pins: Vec<String>,
}

/// Each pin's place in the board's 640 × 760 picture (its top left there, 30 px round).
fn pin_at(p: &str) -> (f32, f32) {
    match p {
        "1" => (225.0, 170.0),
        "2" => (385.0, 170.0),
        "3" => (350.0, 146.0),
        "4" => (350.0, 250.0),
        "5" => (108.0, 374.0),
        "6" => (108.0, 442.0),
        "L" => (370.0, 510.0),
        _ => (480.0, 300.0),
    }
}

impl Pic {
    fn k(b: Rectangle) -> (f32, f32, f32) {
        let k = (b.width / 640.0).min(b.height / 700.0).max(0.3);
        (k, (b.width - 640.0 * k) / 2.0, (b.height - 700.0 * k) / 2.0)
    }
}

fn text(f: &mut Frame, s: &str, at: Point, size: f32, c: Color) {
    f.fill_text(canvas::Text { content: s.to_string(), position: at, color: c, size: Pixels(size), font: hearth_ui::ui::weight(iced::font::Weight::ExtraBold), ..canvas::Text::default() });
}

impl canvas::Program<Msg> for Pic {
    type State = ();
    fn update(&self, _s: &mut (), event: &canvas::Event, b: Rectangle, cursor: mouse::Cursor) -> Option<canvas::Action<Msg>> {
        let canvas::Event::Mouse(mouse::Event::ButtonPressed(btn)) = event else { return None };
        let q = cursor.position_in(b)?;
        let (k, ox, oy) = Pic::k(b);
        let (x, y) = ((q.x - ox) / k, (q.y - oy) / k);
        let on_pin = self.pins.iter().find(|p| {
            let (px, py) = pin_at(p);
            (x - px - 15.0).powi(2) + (y - py - 15.0).powi(2) <= 18.0 * 18.0
        });
        let pick = match (btn, on_pin) {
            (mouse::Button::Left, Some(p)) => p.clone(),
            (mouse::Button::Left, None) => "1".into(),
            (mouse::Button::Right, _) => "2".into(),
            (mouse::Button::Middle, _) => "3".into(),
            (mouse::Button::Forward, _) => "5".into(),
            (mouse::Button::Back, _) => "6".into(),
            _ => return None,
        };
        Some(canvas::Action::publish(Msg::Pin(pick)).and_capture())
    }
    fn mouse_interaction(&self, _s: &(), b: Rectangle, cursor: mouse::Cursor) -> mouse::Interaction {
        if cursor.position_in(b).is_some() { mouse::Interaction::Pointer } else { mouse::Interaction::default() }
    }
    fn draw(&self, _s: &(), renderer: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        let mut f = Frame::new(renderer, b.size());
        let (k, ox, oy) = Pic::k(b);
        f.translate(iced::Vector::new(ox, oy));
        f.scale(k);
        let line = |w: f32| Stroke::default().with_color(hexc(0x4a4038)).with_width(w);
        // the body (the board's path, 90 px higher: here the picture starts at its top)
        let pt = |x: f32, y: f32| Point::new(x, y);
        let body = Path::new(|p| {
            p.move_to(pt(320.0, 0.0));
            p.bezier_curve_to(pt(210.0, 0.0), pt(170.0, 80.0), pt(165.0, 190.0));
            p.bezier_curve_to(pt(160.0, 310.0), pt(165.0, 430.0), pt(200.0, 510.0));
            p.bezier_curve_to(pt(230.0, 570.0), pt(280.0, 600.0), pt(320.0, 600.0));
            p.bezier_curve_to(pt(360.0, 600.0), pt(410.0, 570.0), pt(440.0, 510.0));
            p.bezier_curve_to(pt(475.0, 430.0), pt(480.0, 310.0), pt(475.0, 190.0));
            p.bezier_curve_to(pt(470.0, 80.0), pt(430.0, 0.0), pt(320.0, 0.0));
            p.close();
        });
        let g = canvas::gradient::Linear::new(pt(320.0, 0.0), pt(320.0, 600.0)).add_stop(0.0, hexc(0x2e2723)).add_stop(1.0, hexc(0x181412));
        f.fill(&body, g);
        f.stroke(&body, line(2.0));
        f.stroke(&Path::line(pt(320.0, 2.0), pt(320.0, 230.0)), line(2.0));
        let split = Path::new(|p| {
            p.move_to(pt(170.0, 210.0));
            p.bezier_curve_to(pt(230.0, 240.0), pt(410.0, 240.0), pt(470.0, 210.0));
        });
        f.stroke(&split, line(2.0));
        // the wheel, the DPI button, the side buttons
        let wheel = Path::rounded_rectangle(pt(304.0, 60.0), Size::new(32.0, 74.0), 16.0.into());
        f.fill(&wheel, hexc(0x141110));
        f.stroke(&wheel, Stroke::default().with_color(if self.picked == "3" { hexc(0xff8a3d) } else { hexc(0x4a4038) }).with_width(2.5));
        for y in [76.0, 88.0, 100.0, 112.0] {
            f.stroke(&Path::line(pt(310.0, y), pt(330.0, y)), Stroke::default().with_color(hexc(0x3a322c)).with_width(2.0));
        }
        for (r, id) in [((308.0, 156.0, 24.0, 34.0), "4"), ((150.0, 270.0, 22.0, 58.0), "5"), ((150.0, 338.0, 22.0, 58.0), "6")] {
            let p = Path::rounded_rectangle(pt(r.0, r.1), Size::new(r.2, r.3), 8.0.into());
            f.fill(&p, hexc(0x2a2420));
            f.stroke(&p, Stroke::default().with_color(if self.picked == id { hexc(0xff8a3d) } else { hexc(0x4a4038) }).with_width(2.0));
        }
        // the lights: the logo and the bottom strip
        if let Some(c) = self.logo {
            let col = Color::from_rgb8(c[0], c[1], c[2]);
            let logo = Path::new(|p| {
                p.move_to(pt(296.0, 430.0));
                p.line_to(pt(320.0, 390.0));
                p.line_to(pt(344.0, 430.0));
                p.line_to(pt(320.0, 470.0));
                p.close();
            });
            f.fill(&Path::circle(pt(320.0, 430.0), 34.0), Color { a: 0.10, ..col });
            f.fill(&logo, Color { a: 0.92, ..col });
        }
        if let Some(c) = self.strip {
            let col = Color::from_rgb8(c[0], c[1], c[2]);
            let s = Path::new(|p| {
                p.move_to(pt(200.0, 510.0));
                p.bezier_curve_to(pt(230.0, 570.0), pt(280.0, 600.0), pt(320.0, 600.0));
                p.bezier_curve_to(pt(360.0, 600.0), pt(410.0, 570.0), pt(440.0, 510.0));
            });
            f.stroke(&s, Stroke::default().with_color(Color { a: 0.25, ..col }).with_width(12.0));
            f.stroke(&s, Stroke::default().with_color(col).with_width(5.0));
        }
        // the pins
        for p in &self.pins {
            let (x, y) = pin_at(p);
            let on = *p == self.picked;
            let c = Path::circle(pt(x + 15.0, y + 15.0), 15.0);
            f.fill(&c, if on { hexc(0xff8a3d) } else { hexc(0x221d1a) });
            f.stroke(&c, Stroke::default().with_color(if on { Color::WHITE } else { hexc(0x3a322c) }).with_width(2.0));
            let w = if p.len() > 1 { 8.0 } else { 4.0 };
            text(&mut f, p, pt(x + 15.0 - w, y + 7.0), 13.0, if on { hexc(0x1b0f06) } else { hexc(0xf1e9e2) });
        }
        vec![f.into_geometry()]
    }
}
