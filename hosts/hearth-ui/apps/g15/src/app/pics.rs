//! The app's pictures (canvas): the G15 itself (Overview and Lighting: the flip-up screen, M1–M3 and MR, the 18
//! G-keys in 3 columns of 6, the main keys, the round button) and the keyboard's screen at its real 160 x 43 dots in
//! the mockups' amber, with the editor's outline around the picked element.

use hearth_g15::lcd::Frame as Lcd;
use hearth_ui::Pal;
use iced::widget::canvas::{self, Frame, Geometry, LineDash, Path, Stroke, Text};
use iced::{mouse, Color, Pixels, Point, Rectangle, Renderer, Size, Theme};

pub const AMBER: Color = Color { r: 0xe7 as f32 / 255.0, g: 0xa3 as f32 / 255.0, b: 0x3c as f32 / 255.0, a: 1.0 };
pub const INK: Color = Color { r: 0x2a as f32 / 255.0, g: 0x1a as f32 / 255.0, b: 0x08 as f32 / 255.0, a: 1.0 };
pub const ORANGE: Color = Color { r: 1.0, g: 0x8a as f32 / 255.0, b: 0x3d as f32 / 255.0, a: 1.0 };

fn rr(f: &mut Frame, x: f32, y: f32, w: f32, h: f32, r: f32, fill: Option<Color>, line: Option<(Color, f32)>) {
    let p = Path::rounded_rectangle(Point::new(x, y), Size::new(w, h), r.into());
    if let Some(c) = fill {
        f.fill(&p, c);
    }
    if let Some((c, wd)) = line {
        f.stroke(&p, Stroke::default().with_color(c).with_width(wd));
    }
}

fn label(f: &mut Frame, s: &str, x: f32, y: f32, size: f32, c: Color) {
    // Centred by hand (about 0.58 of the size per character in Inter's figures and capitals).
    let w = s.chars().count() as f32 * size * 0.58;
    f.fill_text(Text { content: s.to_string(), position: Point::new(x - w / 2.0, y - size * 0.62), color: c, size: Pixels(size), ..Text::default() });
}

/// The G15 as the mockups draw it (g15-1 Overview, g15-4 Lighting).
pub struct Keyboard {
    pub p: Pal,
    /// The key set lit (1–3), and MR while recording.
    pub set: u8,
    pub recording: bool,
    /// Lighting: every key outlined in the keys' orange light.
    pub lit: bool,
    pub time: String,
}

impl<M> canvas::Program<M> for Keyboard {
    type State = ();
    fn draw(&self, _s: &(), renderer: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        let mut f = Frame::new(renderer, b.size());
        // Drawn on a 1000 x 360 grid, scaled to fit.
        let k = (b.width / 1000.0).min(b.height / 360.0);
        let ox = (b.width - 1000.0 * k) / 2.0;
        let oy = (b.height - 360.0 * k) / 2.0;
        let x = |v: f32| ox + v * k;
        let y = |v: f32| oy + v * k;
        let body = Pal::mix(self.p.panel, Color::BLACK, 0.25);
        let key = Pal::mix(self.p.panel, Color::WHITE, 0.06);
        let edge = Pal::mix(self.p.line, Color::WHITE, 0.06);
        let glow = Pal::a(ORANGE, 0.8);
        // The flip-up screen.
        rr(&mut f, x(370.0), y(0.0), 260.0 * k, 92.0 * k, 8.0 * k, Some(body), Some((edge, 1.5)));
        rr(&mut f, x(390.0), y(14.0), 220.0 * k, 54.0 * k, 3.0 * k, Some(AMBER), None);
        f.fill_text(Text { content: self.time.clone(), position: Point::new(x(500.0) - 58.0 * k, y(20.0)), color: INK, size: Pixels(32.0 * k), font: iced::Font::MONOSPACE, ..Text::default() });
        for i in 0..5 {
            rr(&mut f, x(404.0 + i as f32 * 40.0), y(76.0), 28.0 * k, 7.0 * k, 3.0 * k, Some(edge), None);
        }
        // The body.
        rr(&mut f, x(0.0), y(82.0), 1000.0 * k, 276.0 * k, 16.0 * k, Some(Pal::a(body, 0.85)), Some((edge, 1.5)));
        // M1 M2 M3 MR.
        for (i, m) in ["M1", "M2", "M3", "MR"].iter().enumerate() {
            let on = (i < 3 && self.set as usize == i + 1) || (i == 3 && self.recording);
            let (fill, line) = if on { (Pal::mix(key, ORANGE, 0.2), (ORANGE, 2.0)) } else if self.lit { (key, (glow, 1.2)) } else { (key, (edge, 1.0)) };
            rr(&mut f, x(24.0 + i as f32 * 40.0), y(100.0), 34.0 * k, 20.0 * k, 4.0 * k, Some(fill), Some(line));
            label(&mut f, m, x(41.0 + i as f32 * 40.0), y(110.0), 10.0 * k, if on || self.lit { ORANGE } else { self.p.muted });
        }
        // G1–G18: 3 columns of 6.
        for r in 0..6 {
            for c in 0..3 {
                let n = r * 3 + c + 1;
                let (gx, gy) = (24.0 + c as f32 * 52.0, 132.0 + r as f32 * 37.0);
                rr(&mut f, x(gx), y(gy), 44.0 * k, 30.0 * k, 4.0 * k, Some(key), Some(if self.lit { (glow, 1.4) } else { (Pal::a(ORANGE, 0.55), 1.0) }));
                label(&mut f, &format!("G{n}"), x(gx + 22.0), y(gy + 15.0), 10.0 * k, ORANGE);
            }
        }
        let main = |f: &mut Frame, gx: f32, gy: f32, w: f32| {
            rr(f, x(gx), y(gy), w * k, 30.0 * k, 4.0 * k, Some(key), Some(if self.lit { (glow, 1.2) } else { (edge, 1.0) }));
        };
        // The function row, then five rows of the main keys, the keys in the middle and the number pad.
        for i in 0..13 {
            main(&mut f, 190.0 + i as f32 * 34.0, 100.0, 29.0);
        }
        for r in 0..5 {
            for i in 0..15 {
                main(&mut f, 190.0 + i as f32 * 34.0, 146.0 + r as f32 * 36.0, 29.0);
            }
        }
        for r in 0..4 {
            for c in 0..3 {
                main(&mut f, 712.0 + c as f32 * 34.0, if r == 0 { 112.0 } else { 146.0 + (r - 1) as f32 * 36.0 }, 29.0);
            }
        }
        for r in 0..5 {
            for c in 0..4 {
                main(&mut f, 830.0 + c as f32 * 36.0, 146.0 + r as f32 * 36.0, 31.0);
            }
        }
        // The round button (the screen's applet key).
        let circ = Path::circle(Point::new(x(960.0), y(108.0)), 13.0 * k);
        f.fill(&circ, key);
        f.stroke(&circ, Stroke::default().with_color(edge).with_width(1.2));
        vec![f.into_geometry()]
    }
}

/// The keyboard's screen: its dots on the amber screen, filling the space it's given (an outline around `pick` in
/// the editor: x, y, w, h in dots, and its name on a tag above).
pub struct Screen {
    pub frame: Lcd,
    pub pick: Option<(i32, i32, i32, i32, String)>,
    pub glow: bool,
}

impl<M> canvas::Program<M> for Screen {
    type State = ();
    fn draw(&self, _s: &(), renderer: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        let mut f = Frame::new(renderer, b.size());
        let pad = if self.glow { 6.0 } else { 3.0 };
        let s = ((b.width - pad * 2.0) / 160.0).min((b.height - pad * 2.0) / 43.0).max(0.5);
        let (w, h) = (160.0 * s, 43.0 * s);
        let (ox, oy) = ((b.width - w) / 2.0, (b.height - h) / 2.0);
        if self.glow {
            rr(&mut f, ox - 6.0, oy - 6.0, w + 12.0, h + 12.0, 6.0, Some(Pal::a(ORANGE, 0.18)), None);
        }
        rr(&mut f, ox - pad, oy - pad, w + pad * 2.0, h + pad * 2.0, 3.0, Some(AMBER), Some((Pal::a(ORANGE, 0.6), 1.0)));
        let dots = Path::new(|p| {
            for yy in 0..43 {
                for xx in 0..160 {
                    if self.frame.get(xx, yy) {
                        p.rectangle(Point::new(ox + xx as f32 * s, oy + yy as f32 * s), Size::new(s, s));
                    }
                }
            }
        });
        f.fill(&dots, INK);
        if let Some((px, py, pw, ph, name)) = &self.pick {
            let r = Path::rectangle(Point::new(ox + *px as f32 * s - 1.0, oy + *py as f32 * s - 1.0), Size::new(*pw as f32 * s + 2.0, *ph as f32 * s + 2.0));
            f.stroke(&r, Stroke { line_dash: LineDash { segments: &[6.0, 4.0], offset: 0 }, ..Stroke::default().with_color(Color::WHITE).with_width(2.0) });
            // The corner to drag.
            f.fill(&Path::rectangle(Point::new(ox + (*px + *pw) as f32 * s - 4.0, oy + (*py + *ph) as f32 * s - 4.0), Size::new(8.0, 8.0)), Color::WHITE);
            let tw = name.chars().count() as f32 * 7.4 + 14.0;
            let ty = (oy + *py as f32 * s - 26.0).max(0.0);
            rr(&mut f, ox + *px as f32 * s, ty, tw, 20.0, 4.0, Some(Color::from_rgb8(0xff, 0x6a, 0x1a)), None);
            f.fill_text(Text { content: name.clone(), position: Point::new(ox + *px as f32 * s + 7.0, ty + 3.0), color: Color::from_rgb8(0x1b, 0x10, 0x06), size: Pixels(13.0), font: hearth_ui::ui::weight(iced::font::Weight::Semibold), ..Text::default() });
        }
        vec![f.into_geometry()]
    }
}

/// The screen's scale and origin in a box of `size` (the editor turns pointer positions into dots with it).
pub fn screen_map(size: Size, glow: bool) -> (f32, f32, f32) {
    let pad = if glow { 6.0 } else { 3.0 };
    let s = ((size.width - pad * 2.0) / 160.0).min((size.height - pad * 2.0) / 43.0).max(0.5);
    ((size.width - 160.0 * s) / 2.0, (size.height - 43.0 * s) / 2.0, s)
}
