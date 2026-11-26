//! The pictures Hearth Lights draws (board 09): the PC's case with its fans, cooler, memory and graphics card in
//! their real colours (each light as OpenRGB reports it), the keyboard and mouse, the colour wheel, and the zones
//! (every light of a fan, clicked to pick, dragged to paint). A press on a part picks that part.

use crate::orgb::Rgb;
use crate::Msg;
use iced::widget::canvas::{self, Frame, Geometry, Path, Stroke};
use iced::{mouse, Color, Pixels, Point, Rectangle, Renderer, Size, Theme};

fn col(c: Rgb) -> Color {
    Color::from_rgb8(c[0], c[1], c[2])
}
fn hexc(s: u32) -> Color {
    Color::from_rgb8((s >> 16) as u8, (s >> 8) as u8, s as u8)
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
fn rrect(f: &mut Frame, x: f32, y: f32, w: f32, h: f32, r: f32, fill: Color, line: Option<(Color, f32)>) {
    let p = Path::rounded_rectangle(Point::new(x, y), Size::new(w, h), r.into());
    f.fill(&p, fill);
    if let Some((c, lw)) = line {
        f.stroke(&p, Stroke::default().with_color(c).with_width(lw));
    }
}
/// A board tag ("FANS · 4"): `acc` the accent one, else the dark one.
fn tagged(f: &mut Frame, s: &str, x: f32, y: f32, acc: bool, k: f32) {
    let w = (s.chars().count() as f32 * 7.2 + 16.0) * k;
    let h = 20.0 * k;
    if acc {
        rrect(f, x, y, w, h, 6.0 * k, hexc(0xff8a3d), None);
        text(f, s, Point::new(x + 8.0 * k, y + 4.0 * k), 11.0 * k, hexc(0x1b0f06), true);
    } else {
        rrect(f, x, y, w, h, 6.0 * k, hexc(0x2a2420), Some((hexc(0x3a322c), 1.0)));
        text(f, s, Point::new(x + 8.0 * k, y + 4.0 * k), 11.0 * k, hexc(0xa59a91), true);
    }
}

/// A fan: a ring of its lights (each light its own arc, in its own colour), dark inside, the hub.
fn fan(f: &mut Frame, cx: f32, cy: f32, d: f32, lights: &[Rgb], picked: bool) {
    let r = d / 2.0;
    let n = lights.len().max(1);
    if picked {
        f.fill(&Path::circle(Point::new(cx, cy), r + 6.0), Color { a: 0.18, ..hexc(0xff8a3d) });
    }
    for (i, c) in lights.iter().enumerate() {
        let a0 = std::f32::consts::TAU * i as f32 / n as f32 - std::f32::consts::FRAC_PI_2;
        let a1 = std::f32::consts::TAU * (i + 1) as f32 / n as f32 - std::f32::consts::FRAC_PI_2;
        let arc = Path::new(|b| {
            b.arc(canvas::path::Arc { center: Point::new(cx, cy), radius: r - 6.0, start_angle: iced::Radians(a0), end_angle: iced::Radians(a1 + 0.02) });
        });
        f.stroke(&arc, Stroke::default().with_color(col(*c)).with_width(12.0));
    }
    f.fill(&Path::circle(Point::new(cx, cy), r - 12.0), hexc(0x16110e));
    let hub = Path::circle(Point::new(cx, cy), (r - 52.0).max(d * 0.15));
    f.fill(&hub, hexc(0x2a2420));
    f.stroke(&hub, Stroke::default().with_color(hexc(0x3a322c)).with_width(2.0));
}

fn avg(v: &[Rgb]) -> Rgb {
    if v.is_empty() {
        return [0x40, 0x38, 0x32];
    }
    let n = v.len() as u32;
    let s = v.iter().fold([0u32; 3], |a, c| [a[0] + c[0] as u32, a[1] + c[1] as u32, a[2] + c[2] as u32]);
    [(s[0] / n) as u8, (s[1] / n) as u8, (s[2] / n) as u8]
}

/// One part of the picture, which device group it is, and where (in the board's 600 × 540 units).
#[derive(Debug, Clone)]
pub struct Part {
    pub key: String,
    pub what: PartKind,
    pub lights: Vec<Rgb>,
}
#[derive(Debug, Clone, PartialEq)]
pub enum PartKind {
    /// fan number (0..4)
    Fan(usize),
    Cooler,
    /// memory stick number (0..4)
    Ram(usize),
    Gpu,
}

/// The case (board: its fans, the CPU cooler, the memory and the graphics card).
pub struct Case {
    pub parts: Vec<Part>,
    pub picked: String,
    pub fans: usize,
    pub sticks: usize,
}

const FANS: [(f32, f32, f32); 4] = [(60.0, 30.0, 150.0), (60.0, 200.0, 150.0), (60.0, 370.0, 120.0), (380.0, 30.0, 130.0)];

impl Case {
    fn k(b: Rectangle) -> (f32, f32, f32) {
        let k = (b.width / 600.0).min(b.height / 540.0).max(0.2);
        (k, (b.width - 600.0 * k) / 2.0, (b.height - 540.0 * k) / 2.0)
    }
    fn rect_of(&self, p: &Part) -> (f32, f32, f32, f32) {
        match p.what {
            PartKind::Fan(i) => {
                let (x, y, d) = FANS[i.min(3)];
                (x, y, d, d)
            }
            PartKind::Cooler => (250.0, 50.0, 110.0, 110.0),
            PartKind::Ram(i) => (370.0 + 28.0 * i as f32, 180.0, 20.0, 200.0),
            PartKind::Gpu => (220.0, 400.0, 340.0, 70.0),
        }
    }
}

impl canvas::Program<Msg> for Case {
    type State = ();
    fn update(&self, _s: &mut (), event: &canvas::Event, b: Rectangle, cursor: mouse::Cursor) -> Option<canvas::Action<Msg>> {
        if let canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event {
            let q = cursor.position_in(b)?;
            let (k, ox, oy) = Case::k(b);
            let (x, y) = ((q.x - ox) / k, (q.y - oy) / k);
            for p in &self.parts {
                let (px, py, pw, ph) = self.rect_of(p);
                if x >= px && x <= px + pw && y >= py && y <= py + ph {
                    return Some(canvas::Action::publish(Msg::Pick(p.key.clone())).and_capture());
                }
            }
        }
        None
    }
    fn mouse_interaction(&self, _s: &(), b: Rectangle, cursor: mouse::Cursor) -> mouse::Interaction {
        if cursor.position_in(b).is_some() { mouse::Interaction::Pointer } else { mouse::Interaction::default() }
    }
    fn draw(&self, _s: &(), renderer: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        let mut f = Frame::new(renderer, b.size());
        let (k, ox, oy) = Case::k(b);
        f.translate(iced::Vector::new(ox, oy));
        f.scale(k);
        rrect(&mut f, 0.0, 0.0, 560.0, 520.0, 18.0, hexc(0x120e0c), Some((hexc(0x4a4038), 2.0)));
        let mut ram_tag = false;
        for p in &self.parts {
            let on = !self.picked.is_empty() && p.key == self.picked;
            match p.what {
                PartKind::Fan(i) => {
                    let (x, y, d) = FANS[i.min(3)];
                    fan(&mut f, x - 30.0 + d / 2.0 + 30.0, y + d / 2.0, d, &p.lights, on);
                }
                PartKind::Cooler => {
                    let c = avg(&p.lights);
                    rrect(&mut f, 250.0, 50.0, 110.0, 110.0, 12.0, hexc(0x1c1816), Some((if on { hexc(0xff8a3d) } else { Color { a: 0.4, ..hexc(0xff8a3d) } }, 3.0)));
                    f.fill(&Path::circle(Point::new(305.0, 105.0), 26.0), Color { a: 0.35, ..col(c) });
                    f.fill(&Path::circle(Point::new(305.0, 105.0), 20.0), col(c));
                    tagged(&mut f, "CPU COOLER", 250.0, 26.0, false, 1.0);
                }
                PartKind::Ram(i) => {
                    let x = 370.0 + 28.0 * i as f32;
                    rrect(&mut f, x, 180.0, 20.0, 200.0, 4.0, hexc(0x1c1816), Some((if on { hexc(0xff8a3d) } else { hexc(0x3a322c) }, 1.0)));
                    let n = p.lights.len().max(1);
                    for (j, c) in p.lights.iter().enumerate().take(16) {
                        let h = 34.0 / n.min(16) as f32;
                        f.fill_rectangle(Point::new(x + 1.0, 181.0 + j as f32 * h), Size::new(18.0, h + 0.5), col(*c));
                    }
                    if !ram_tag {
                        ram_tag = true;
                        tagged(&mut f, &format!("MEMORY · {}", self.sticks), 370.0, 156.0, false, 1.0);
                    }
                }
                PartKind::Gpu => {
                    rrect(&mut f, 220.0, 400.0, 340.0, 70.0, 10.0, hexc(0x1c1816), Some((if on { hexc(0xff8a3d) } else { hexc(0x3a322c) }, 1.0)));
                    let n = p.lights.len().max(1);
                    let w = 312.0 / n as f32;
                    for (j, c) in p.lights.iter().enumerate() {
                        f.fill_rectangle(Point::new(234.0 + j as f32 * w, 412.0), Size::new(w + 0.5, 6.0), col(*c));
                    }
                    text(&mut f, "Graphics card", Point::new(234.0, 444.0), 12.0, hexc(0xa59a91), false);
                }
            }
        }
        if self.fans > 0 {
            tagged(&mut f, &format!("FANS · {}", self.fans), 30.0, 6.0, true, 1.0);
        }
        vec![f.into_geometry()]
    }
}

/// The keyboard (each key in its light's colour) and the mouse (its logo and wheel), with their tags.
pub struct Desk {
    pub keys: Option<(String, Vec<Rgb>)>,
    pub mouse: Option<(String, Rgb, Rgb)>,
    pub picked: String,
}

impl canvas::Program<Msg> for Desk {
    type State = ();
    fn update(&self, _s: &mut (), event: &canvas::Event, b: Rectangle, cursor: mouse::Cursor) -> Option<canvas::Action<Msg>> {
        if let canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event {
            let q = cursor.position_in(b)?;
            let k = (b.width / 455.0).min(b.height / 180.0);
            let x = q.x / k;
            if x < 345.0 {
                if let Some((key, _)) = &self.keys {
                    return Some(canvas::Action::publish(Msg::Pick(key.clone())).and_capture());
                }
            } else if let Some((key, _, _)) = &self.mouse {
                return Some(canvas::Action::publish(Msg::Pick(key.clone())).and_capture());
            }
        }
        None
    }
    fn mouse_interaction(&self, _s: &(), b: Rectangle, cursor: mouse::Cursor) -> mouse::Interaction {
        if cursor.position_in(b).is_some() { mouse::Interaction::Pointer } else { mouse::Interaction::default() }
    }
    fn draw(&self, _s: &(), renderer: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        let mut f = Frame::new(renderer, b.size());
        let k = (b.width / 455.0).min(b.height / 180.0).max(0.2);
        f.scale(k);
        if let Some((key, lights)) = &self.keys {
            let on = *key == self.picked;
            tagged(&mut f, "KEYBOARD · PER-KEY", 0.0, 0.0, false, 1.0);
            rrect(&mut f, 0.0, 28.0, 340.0, 150.0, 14.0, hexc(0x16110e), Some((if on { hexc(0xff8a3d) } else { hexc(0x3a322c) }, 1.0)));
            let (cols, rows) = (15usize, 5usize);
            let (kw, kh) = ((340.0 - 24.0 - 4.0 * 14.0) / 15.0, 20.0);
            for r in 0..rows {
                for c in 0..cols {
                    let i = r * cols + c;
                    let li = if lights.is_empty() { 0 } else { i * lights.len() / (cols * rows) };
                    let colr = lights.get(li).copied().unwrap_or([0x40, 0x38, 0x32]);
                    rrect(&mut f, 12.0 + c as f32 * (kw + 4.0), 40.0 + r as f32 * (kh + 6.0), kw, kh, 4.0, col(colr), None);
                }
            }
        }
        if let Some((key, logo, wheel)) = &self.mouse {
            let on = *key == self.picked;
            tagged(&mut f, "MOUSE", 365.0, 0.0, false, 1.0);
            let body = Path::new(|p| {
                p.move_to(Point::new(365.0, 73.0));
                p.arc_to(Point::new(365.0, 28.0), Point::new(410.0, 28.0), 45.0);
                p.arc_to(Point::new(455.0, 28.0), Point::new(455.0, 73.0), 45.0);
                p.line_to(Point::new(455.0, 138.0));
                p.arc_to(Point::new(455.0, 178.0), Point::new(415.0, 178.0), 40.0);
                p.line_to(Point::new(405.0, 178.0));
                p.arc_to(Point::new(365.0, 178.0), Point::new(365.0, 138.0), 40.0);
                p.close();
            });
            f.fill(&body, hexc(0x1c1816));
            f.stroke(&body, Stroke::default().with_color(if on { hexc(0xff8a3d) } else { hexc(0x3a322c) }).with_width(1.0));
            rrect(&mut f, 407.0, 40.0, 6.0, 30.0, 3.0, col(*logo), None);
            f.stroke(&Path::circle(Point::new(410.0, 136.0), 16.0), Stroke::default().with_color(col(*wheel)).with_width(4.0));
        }
        vec![f.into_geometry()]
    }
}

/// The colour wheel: every hue round it, white in the middle, the picked colour's ring.
pub struct Wheel {
    pub at: Option<(f32, f32)>,
    pub colour: Rgb,
}

impl canvas::Program<Msg> for Wheel {
    type State = bool;
    fn update(&self, pressed: &mut bool, event: &canvas::Event, b: Rectangle, cursor: mouse::Cursor) -> Option<canvas::Action<Msg>> {
        let pick = |q: Point| -> Option<canvas::Action<Msg>> {
            let r = b.width.min(b.height) / 2.0;
            let (dx, dy) = (q.x - b.width / 2.0, q.y - b.height / 2.0);
            let d = (dx * dx + dy * dy).sqrt() / r;
            if d > 1.05 {
                return None;
            }
            let h = (dy.atan2(dx) / std::f32::consts::TAU).rem_euclid(1.0);
            Some(canvas::Action::publish(Msg::Wheel(crate::fx::hsv(h, d.min(1.0), 1.0))).and_capture())
        };
        match event {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let q = cursor.position_in(b)?;
                *pressed = true;
                pick(q)
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) if *pressed => pick(cursor.position_in(b)?),
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                *pressed = false;
                None
            }
            _ => None,
        }
    }
    fn draw(&self, _s: &bool, renderer: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        let mut f = Frame::new(renderer, b.size());
        let r = b.width.min(b.height) / 2.0;
        let c = Point::new(b.width / 2.0, b.height / 2.0);
        let (nh, nr) = (72, 8);
        for i in 0..nh {
            let a0 = std::f32::consts::TAU * i as f32 / nh as f32;
            let a1 = std::f32::consts::TAU * (i + 1) as f32 / nh as f32 + 0.01;
            for j in 0..nr {
                let (r0, r1) = (r * j as f32 / nr as f32, r * (j + 1) as f32 / nr as f32);
                let p = Path::new(|p| {
                    p.move_to(Point::new(c.x + r0 * a0.cos(), c.y + r0 * a0.sin()));
                    p.line_to(Point::new(c.x + r1 * a0.cos(), c.y + r1 * a0.sin()));
                    p.line_to(Point::new(c.x + r1 * a1.cos(), c.y + r1 * a1.sin()));
                    p.line_to(Point::new(c.x + r0 * a1.cos(), c.y + r0 * a1.sin()));
                    p.close();
                });
                f.fill(&p, col(crate::fx::hsv(i as f32 / nh as f32, (j as f32 + 0.5) / nr as f32, 1.0)));
            }
        }
        // where the colour is: from its hue and how strong it is
        let (h, s) = self.at.unwrap_or_else(|| hue_sat(self.colour));
        let at = Point::new(c.x + r * s * (h * std::f32::consts::TAU).cos(), c.y + r * s * (h * std::f32::consts::TAU).sin());
        f.fill(&Path::circle(at, 9.0), Color::from_rgba8(0, 0, 0, 0.4));
        f.fill(&Path::circle(at, 7.5), col(self.colour));
        f.stroke(&Path::circle(at, 7.5), Stroke::default().with_color(Color::WHITE).with_width(3.0));
        vec![f.into_geometry()]
    }
}

/// A colour's hue and strength (0..1 each).
pub fn hue_sat(c: Rgb) -> (f32, f32) {
    let (r, g, b) = (c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0);
    let mx = r.max(g).max(b);
    let mn = r.min(g).min(b);
    let d = mx - mn;
    let s = if mx > 0.0 { d / mx } else { 0.0 };
    let h = if d == 0.0 {
        0.0
    } else if mx == r {
        ((g - b) / d).rem_euclid(6.0) / 6.0
    } else if mx == g {
        ((b - r) / d + 2.0) / 6.0
    } else {
        ((r - g) / d + 4.0) / 6.0
    };
    (h, s)
}

/// The zones: each fan's (zone's) lights in a row, its name on the left; a press picks a light, a drag paints.
pub struct Zones {
    pub rows: Vec<(String, Vec<Rgb>)>,
}

impl Zones {
    fn hit(&self, q: Point) -> Option<(usize, usize)> {
        let z = (q.y / 26.0).floor() as usize;
        let x = q.x - 76.0;
        if x < 0.0 || z >= self.rows.len() {
            return None;
        }
        let j = (x / 22.0).floor() as usize;
        (j < self.rows[z].1.len()).then_some((z, j))
    }
}

impl canvas::Program<Msg> for Zones {
    type State = Option<(usize, usize)>;
    fn update(&self, st: &mut Option<(usize, usize)>, event: &canvas::Event, b: Rectangle, cursor: mouse::Cursor) -> Option<canvas::Action<Msg>> {
        match event {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let h = self.hit(cursor.position_in(b)?)?;
                *st = Some(h);
                Some(canvas::Action::publish(Msg::Paint(h.0, h.1)).and_capture())
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) if st.is_some() => {
                let h = self.hit(cursor.position_in(b)?)?;
                if *st == Some(h) {
                    return None;
                }
                *st = Some(h);
                Some(canvas::Action::publish(Msg::Paint(h.0, h.1)).and_capture())
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                *st = None;
                None
            }
            _ => None,
        }
    }
    fn mouse_interaction(&self, _s: &Option<(usize, usize)>, b: Rectangle, cursor: mouse::Cursor) -> mouse::Interaction {
        match cursor.position_in(b) {
            Some(q) if self.hit(q).is_some() => mouse::Interaction::Crosshair,
            _ => mouse::Interaction::default(),
        }
    }
    fn draw(&self, _s: &Option<(usize, usize)>, renderer: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        let mut f = Frame::new(renderer, b.size());
        for (z, (name, lights)) in self.rows.iter().enumerate() {
            let y = z as f32 * 26.0;
            text(&mut f, name, Point::new(0.0, y + 2.0), 12.0, hexc(0xa59a91), false);
            for (j, c) in lights.iter().enumerate() {
                let cx = 76.0 + j as f32 * 22.0 + 8.0;
                f.fill(&Path::circle(Point::new(cx, y + 9.0), 10.0), Color { a: 0.25, ..col(*c) });
                f.fill(&Path::circle(Point::new(cx, y + 9.0), 8.0), col(*c));
            }
        }
        vec![f.into_geometry()]
    }
}
