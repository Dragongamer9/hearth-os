//! The model turning in the viewport, drawn in Rust on iced's canvas: the preview the service made of it (at most
//! 14 000 faces with their colours), lit by the light you picked, seen as its shape, as clay, real, or its wires, on
//! a floor. Drag to turn it, scroll to zoom, right-drag to move. Drawn again only when something it shows changes.

use iced::widget::canvas::{self, Cache, Frame, Geometry, Path, Stroke};
use iced::{mouse, Color, Point, Rectangle, Renderer, Size, Theme};

#[derive(Debug, Clone, Default)]
pub struct Mesh {
    pub v: Vec<[f32; 3]>,
    pub f: Vec<[u32; 3]>,
    pub c: Vec<[u8; 3]>,
}

impl Mesh {
    /// From preview.json: centred on its floor, one unit tall, Y up.
    pub fn from_preview(j: &serde_json::Value) -> Option<Mesh> {
        let num = |k: &str| j.get(k).and_then(|x| x.as_array()).map(|a| a.iter().filter_map(|x| x.as_f64()).collect::<Vec<f64>>()).unwrap_or_default();
        let (v, f, c) = (num("v"), num("f"), num("c"));
        if v.len() < 9 || f.len() < 3 {
            return None;
        }
        let mut verts: Vec<[f32; 3]> = v.chunks(3).map(|p| [p[0] as f32, p[1] as f32, p[2] as f32]).collect();
        let lo = (0..3).map(|i| verts.iter().map(|p| p[i]).fold(f32::MAX, f32::min)).collect::<Vec<_>>();
        let hi = (0..3).map(|i| verts.iter().map(|p| p[i]).fold(f32::MIN, f32::max)).collect::<Vec<_>>();
        let h = (hi[1] - lo[1]).max(1e-6);
        let (cx, cz) = ((lo[0] + hi[0]) / 2.0, (lo[2] + hi[2]) / 2.0);
        for p in verts.iter_mut() {
            *p = [(p[0] - cx) / h, (p[1] - lo[1]) / h, (p[2] - cz) / h];
        }
        let n = verts.len() as u32;
        let faces: Vec<[u32; 3]> = f.chunks(3).filter(|t| t.len() == 3).map(|t| [t[0] as u32, t[1] as u32, t[2] as u32]).filter(|t| t.iter().all(|i| *i < n)).collect();
        let cols: Vec<[u8; 3]> = c.chunks(3).filter(|t| t.len() == 3).map(|t| [t[0] as u8, t[1] as u8, t[2] as u8]).collect();
        Some(Mesh { v: verts, f: faces, c: cols })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cam {
    pub yaw: f32,
    pub pitch: f32,
    pub zoom: f32,
    pub pan: (f32, f32),
}

impl Default for Cam {
    fn default() -> Self {
        Cam { yaw: 0.35, pitch: 0.16, zoom: 1.0, pan: (0.0, 0.0) }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Look {
    Shape,
    Clay,
    Real,
    Wire,
}

/// The lights to pick from: (name, the light's colour, the glow behind, the background, how strong).
pub const LIGHTS: [(&str, [u8; 3], [u8; 3], [u8; 3], f32); 4] = [
    ("workshop", [255, 214, 160], [255, 160, 70], [43, 35, 32], 1.0),
    ("daylight", [255, 255, 250], [200, 215, 235], [40, 42, 46], 1.0),
    ("studio", [235, 240, 255], [120, 130, 160], [26, 27, 32], 1.1),
    ("candle", [255, 170, 90], [255, 120, 40], [24, 16, 12], 0.85),
];

pub struct Viewport<'a, M> {
    pub mesh: Option<&'a Mesh>,
    pub cam: Cam,
    pub look: Look,
    pub light: usize,
    pub floor: bool,
    pub cache: &'a Cache,
    pub on_cam: fn(Cam) -> M,
}

#[derive(Default)]
pub struct St {
    drag: Option<(mouse::Button, Point)>,
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn norm(a: [f32; 3]) -> [f32; 3] {
    let l = dot(a, a).sqrt().max(1e-9);
    [a[0] / l, a[1] / l, a[2] / l]
}
fn ellipse(c: Point, rx: f32, ry: f32) -> Path {
    Path::new(|p| {
        for i in 0..=36 {
            let a = i as f32 / 36.0 * std::f32::consts::TAU;
            let q = Point::new(c.x + rx * a.cos(), c.y + ry * a.sin());
            if i == 0 {
                p.move_to(q);
            } else {
                p.line_to(q);
            }
        }
        p.close();
    })
}
fn rgb(c: [u8; 3], k: f32) -> Color {
    Color::from_rgb((c[0] as f32 / 255.0 * k).min(1.0), (c[1] as f32 / 255.0 * k).min(1.0), (c[2] as f32 / 255.0 * k).min(1.0))
}

impl<'a, M> Viewport<'a, M> {
    /// The camera's view of a point (model space, one unit tall): screen x, y and its depth.
    fn project(&self, p: [f32; 3], b: Size) -> (f32, f32, f32) {
        let (sy, cy) = self.cam.yaw.sin_cos();
        let (sp, cp) = self.cam.pitch.sin_cos();
        let q = [p[0], p[1] - 0.5, p[2]];
        let x = q[0] * cy + q[2] * sy;
        let z0 = -q[0] * sy + q[2] * cy;
        let y = q[1] * cp - z0 * sp;
        let z = q[1] * sp + z0 * cp;
        let dist = 3.2;
        let f = b.height * 0.42 * self.cam.zoom * dist / (dist - z).max(0.3);
        (b.width / 2.0 + x * f + self.cam.pan.0, b.height * 0.52 - y * f + self.cam.pan.1, z)
    }
}

impl<'a, M: Clone> canvas::Program<M> for Viewport<'a, M> {
    type State = St;

    fn update(&self, st: &mut St, event: &canvas::Event, b: Rectangle, cursor: mouse::Cursor) -> Option<canvas::Action<M>> {
        match event {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(btn @ (mouse::Button::Left | mouse::Button::Right))) => {
                let pt = cursor.position_in(b)?;
                st.drag = Some((*btn, pt));
                Some(canvas::Action::request_redraw().and_capture())
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let (btn, last) = st.drag?;
                let pt = cursor.position_in(b).unwrap_or(last);
                let (dx, dy) = (pt.x - last.x, pt.y - last.y);
                if dx.abs() + dy.abs() < 0.5 {
                    return None;
                }
                st.drag = Some((btn, pt));
                let mut c = self.cam;
                if btn == mouse::Button::Left {
                    c.yaw += dx * 0.01;
                    c.pitch = (c.pitch + dy * 0.01).clamp(-1.2, 1.2);
                } else {
                    c.pan = (c.pan.0 + dx, c.pan.1 + dy);
                }
                Some(canvas::Action::publish((self.on_cam)(c)).and_capture())
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(_)) => {
                st.drag.take()?;
                Some(canvas::Action::request_redraw())
            }
            canvas::Event::Mouse(mouse::Event::WheelScrolled { delta }) if cursor.is_over(b) => {
                let y = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => *y,
                    mouse::ScrollDelta::Pixels { y, .. } => *y / 40.0,
                };
                let mut c = self.cam;
                c.zoom = (c.zoom * 1.1f32.powf(y)).clamp(0.3, 4.0);
                Some(canvas::Action::publish((self.on_cam)(c)).and_capture())
            }
            _ => None,
        }
    }

    fn draw(&self, _st: &St, r: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        let g = self.cache.draw(r, b.size(), |f: &mut Frame| {
            let sz = b.size();
            let (_, lc, glow, bgc, power) = LIGHTS[self.light.min(LIGHTS.len() - 1)];
            // the room: its dark, the glow behind the model (soft rings), the floor
            f.fill(&Path::rectangle(Point::ORIGIN, sz), rgb(bgc, 0.62));
            let c = Point::new(sz.width / 2.0, sz.height * 0.45);
            for i in 0..14 {
                let k = i as f32 / 14.0;
                let rad = sz.height * (0.75 - 0.5 * k);
                f.fill(&Path::circle(c, rad), Color { a: 0.05 + 0.02 * k, ..rgb(bgc, 1.0 + 0.4 * k) });
            }
            if self.mesh.is_some() {
                for i in 0..10 {
                    let k = i as f32 / 10.0;
                    f.fill(&Path::circle(c, sz.height * (0.3 - 0.24 * k)), Color { a: 0.06 * power, ..rgb(glow, 1.0) });
                }
            }
            if self.floor {
                let lines = Color::from_rgba8(0x3a, 0x32, 0x2c, 0.35);
                for k in -10..=10 {
                    let x = k as f32 * 0.12;
                    let (a, b2) = (self.project([x, 0.0, -1.2], sz), self.project([x, 0.0, 1.2], sz));
                    f.stroke(&Path::line(Point::new(a.0, a.1), Point::new(b2.0, b2.1)), Stroke::default().with_color(lines).with_width(1.0));
                    let (a, b2) = (self.project([-1.2, 0.0, x], sz), self.project([1.2, 0.0, x], sz));
                    f.stroke(&Path::line(Point::new(a.0, a.1), Point::new(b2.0, b2.1)), Stroke::default().with_color(lines).with_width(1.0));
                }
            }
            let Some(m) = self.mesh else { return };
            // its shadow and the light's pool on the floor
            let foot = self.project([0.0, 0.0, 0.0], sz);
            let w = sz.height * 0.42 * self.cam.zoom * 0.9;
            for i in 0..8 {
                let k = i as f32 / 8.0;
                let e = ellipse(Point::new(foot.0, foot.1), w * (1.3 - k), w * (0.2 - 0.15 * k));
                f.fill(&e, Color { a: 0.04 * power, ..rgb(glow, 1.0) });
            }
            let sh = ellipse(Point::new(foot.0, foot.1), w * 0.45, w * 0.07);
            f.fill(&sh, Color::from_rgba(0.0, 0.0, 0.0, 0.45));
            // the faces, far to near, each lit by the light (from the front left, a little above) and the room
            let pts: Vec<(f32, f32, f32)> = m.v.iter().map(|p| self.project(*p, sz)).collect();
            let (sy, cy) = self.cam.yaw.sin_cos();
            let light = norm([-0.5 * cy + 0.8 * sy, 0.7, 0.5 * sy + 0.8 * cy]);
            let eye = norm([sy, self.cam.pitch.sin(), cy]);
            let mut order: Vec<(f32, usize)> = Vec::with_capacity(m.f.len());
            for (i, t) in m.f.iter().enumerate() {
                let (a, b2, c2) = (pts[t[0] as usize], pts[t[1] as usize], pts[t[2] as usize]);
                // every face, far to near (models come with either winding, so none is skipped but the empty ones)
                let area = (b2.0 - a.0) * (c2.1 - a.1) - (b2.1 - a.1) * (c2.0 - a.0);
                if area.abs() < 0.01 {
                    continue;
                }
                order.push(((a.2 + b2.2 + c2.2) / 3.0, i));
            }
            order.sort_by(|x, y| x.0.partial_cmp(&y.0).unwrap_or(std::cmp::Ordering::Equal));
            let base = match self.look {
                Look::Shape => [196, 192, 188],
                Look::Clay => [196, 132, 96],
                _ => [180, 180, 180],
            };
            for (_, i) in order {
                let t = m.f[i];
                let (p0, p1, p2) = (m.v[t[0] as usize], m.v[t[1] as usize], m.v[t[2] as usize]);
                let n = norm(cross(sub(p1, p0), sub(p2, p0)));
                let n = if dot(n, eye) < 0.0 { [-n[0], -n[1], -n[2]] } else { n };
                let lit = dot(n, light).max(0.0) * 0.75 * power + 0.22 + dot(n, eye).abs().powf(8.0) * 0.12;
                let col = match self.look {
                    Look::Real => m.c.get(i).copied().unwrap_or(base),
                    _ => base,
                };
                let shade = [((col[0] as f32) * (lc[0] as f32 / 255.0)) as u8, ((col[1] as f32) * (lc[1] as f32 / 255.0)) as u8, ((col[2] as f32) * (lc[2] as f32 / 255.0)) as u8];
                let (a, b2, c2) = (pts[t[0] as usize], pts[t[1] as usize], pts[t[2] as usize]);
                let tri = Path::new(|p| {
                    p.move_to(Point::new(a.0, a.1));
                    p.line_to(Point::new(b2.0, b2.1));
                    p.line_to(Point::new(c2.0, c2.1));
                    p.close();
                });
                if self.look == Look::Wire {
                    f.stroke(&tri, Stroke::default().with_color(Color::from_rgba8(0xff, 0x8a, 0x3d, 0.55)).with_width(0.6));
                } else {
                    let fill = rgb(shade, lit);
                    f.fill(&tri, fill);
                    // the seams between faces closed with the face's own colour
                    f.stroke(&tri, Stroke::default().with_color(fill).with_width(0.6));
                }
            }
        });
        vec![g]
    }

    fn mouse_interaction(&self, st: &St, b: Rectangle, c: mouse::Cursor) -> mouse::Interaction {
        if st.drag.is_some() {
            mouse::Interaction::Grabbing
        } else if c.is_over(b) {
            mouse::Interaction::Grab
        } else {
            mouse::Interaction::default()
        }
    }
}
