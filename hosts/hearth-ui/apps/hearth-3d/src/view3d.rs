//! The viewport: the scene drawn in Rust on iced's canvas (wgpu where the graphics card starts, tiny-skia otherwise).
//! Each object's mesh (as Blender made it, changes on top applied) is moved to where it is at this frame, seen
//! through the camera (orbit, pan, zoom; perspective or flat), its faces lit by one light and drawn far to near; the
//! picked object outlined in the accent with Move's arrows on it; the floor's grid, a soft shadow under the scene and
//! the axes in the corner. Drawn again only when something it shows changes (a cache).

use crate::scene::{Obj, Scene};
use iced::widget::canvas::{self, Cache, Frame, Geometry, Path, Stroke, Text};
use iced::{mouse, Color, Point, Rectangle, Renderer, Size, Theme};
use std::cell::RefCell;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cam {
    pub yaw: f32,
    pub pitch: f32,
    pub dist: f32,
    pub target: [f32; 3],
    pub persp: bool,
}

impl Default for Cam {
    fn default() -> Self {
        Cam { yaw: -0.785, pitch: 0.55, dist: 26.0, target: [0.0, 0.0, 3.0], persp: true }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Shade {
    Wire,
    Solid,
    Material,
    Rendered,
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn norm(a: [f32; 3]) -> [f32; 3] {
    let l = dot(a, a).sqrt().max(1e-6);
    [a[0] / l, a[1] / l, a[2] / l]
}
pub fn apply(m: &[f32; 16], v: [f32; 3]) -> [f32; 3] {
    [m[0] * v[0] + m[4] * v[1] + m[8] * v[2] + m[12], m[1] * v[0] + m[5] * v[1] + m[9] * v[2] + m[13], m[2] * v[0] + m[6] * v[1] + m[10] * v[2] + m[14]]
}

/// The camera's eye and its three directions (Z is up, as in Blender).
pub struct View {
    eye: [f32; 3],
    right: [f32; 3],
    up: [f32; 3],
    fwd: [f32; 3],
    f: f32,
    c: Point,
    persp: bool,
    ortho: f32,
}

impl View {
    pub fn new(cam: &Cam, size: Size) -> View {
        let dir = [cam.pitch.cos() * cam.yaw.cos(), cam.pitch.cos() * cam.yaw.sin(), cam.pitch.sin()];
        let eye = [cam.target[0] + dir[0] * cam.dist, cam.target[1] + dir[1] * cam.dist, cam.target[2] + dir[2] * cam.dist];
        let fwd = norm(sub(cam.target, eye));
        let right = norm(cross(fwd, [0.0, 0.0, 1.0]));
        let up = cross(right, fwd);
        let f = size.height / (2.0 * (0.4f32).tan());
        View { eye, right, up, fwd, f, c: Point::new(size.width / 2.0, size.height / 2.0), persp: cam.persp, ortho: size.height / (cam.dist * 0.8) }
    }
    /// A point on the screen and its depth (None behind the eye).
    pub fn project(&self, p: [f32; 3]) -> Option<(Point, f32)> {
        let v = sub(p, self.eye);
        let z = dot(v, self.fwd);
        if self.persp && z < 0.1 {
            return None;
        }
        let (x, y) = (dot(v, self.right), dot(v, self.up));
        Some(if self.persp { (Point::new(self.c.x + self.f * x / z, self.c.y - self.f * y / z), z) } else { (Point::new(self.c.x + self.ortho * x, self.c.y - self.ortho * y), z) })
    }
    /// How far a pointer move of (dx, dy) pixels goes in the world, at the distance of `at` (Move, G).
    pub fn screen_to_world(&self, at: [f32; 3], dx: f32, dy: f32) -> [f32; 3] {
        let z = dot(sub(at, self.eye), self.fwd).max(0.1);
        let k = if self.persp { z / self.f } else { 1.0 / self.ortho };
        [self.right[0] * dx * k - self.up[0] * dy * k, self.right[1] * dx * k - self.up[1] * dy * k, self.right[2] * dx * k - self.up[2] * dy * k]
    }
}

/// Where an object is at a frame (its own frames when it moves, else as it is now).
pub fn mat_at<'a>(o: &'a Obj, s: &Scene, frame: i64) -> &'a [f32; 16] {
    let i = (frame - s.start).max(0) as usize;
    o.anim.get(i).or(o.anim.last()).unwrap_or(&o.mat)
}

pub struct Viewport<'a> {
    pub scene: &'a Scene,
    pub cam: Cam,
    pub shade: Shade,
    pub picked: &'a [String],
    pub frame: i64,
    pub accent: Color,
    pub gen: u64,
    pub overlays: [bool; 3],
    /// what each pointer gesture asks the app for
    pub on: fn(VEvent) -> crate::Msg,
}

#[derive(Debug, Clone)]
pub enum VEvent {
    Orbit(f32, f32),
    Pan(f32, f32),
    Zoom(f32),
    /// a click: the object under the pointer (or none), with Shift held (add to what's picked)
    Pick(Option<String>, bool),
    /// a drag on the picked object's arrows / with G, R or S: how far
    Drag(f32, f32),
    DragEnd,
    Size(f32, f32),
}

#[derive(Default)]
pub struct VState {
    cache: Cache,
    key: RefCell<Option<(Cam, Shade, u64, i64, usize, Size)>>,
    tris: RefCell<Vec<([Point; 3], f32, String)>>,
    down: Option<(Point, mouse::Button, bool)>,
    moved: bool,
    shift: bool,
}

fn shade_of(c: [f32; 3], n: [f32; 3], light: [f32; 3]) -> Color {
    let k = 0.38 + 0.62 * dot(n, light).max(0.0);
    Color::from_rgb((c[0] * k).min(1.0), (c[1] * k).min(1.0), (c[2] * k).min(1.0))
}

impl<'a> canvas::Program<crate::Msg> for Viewport<'a> {
    type State = VState;

    fn update(&self, st: &mut VState, ev: &iced::Event, b: Rectangle, cursor: mouse::Cursor) -> Option<canvas::Action<crate::Msg>> {
        use iced::Event;
        let pos = cursor.position_in(b);
        match ev {
            Event::Keyboard(iced::keyboard::Event::ModifiersChanged(m)) => {
                st.shift = m.shift();
                None
            }
            Event::Mouse(mouse::Event::ButtonPressed(btn)) => {
                let p = pos?;
                // on the picked object (its Move arrows): dragging moves it
                let on_picked = st.tris.borrow().iter().rev().find(|t| inside(p, &t.0)).map(|t| self.picked.contains(&t.2)).unwrap_or(false);
                st.down = Some((p, *btn, on_picked && *btn == mouse::Button::Left));
                st.moved = false;
                Some(canvas::Action::capture())
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let (start, btn, drag) = st.down?;
                let p = cursor.position_in(b).or_else(|| cursor.position().map(|q| Point::new(q.x - b.x, q.y - b.y)))?;
                let (dx, dy) = (p.x - start.x, p.y - start.y);
                if !st.moved && dx.abs() + dy.abs() < 3.0 {
                    return None;
                }
                st.moved = true;
                st.down = Some((p, btn, drag));
                let m = if drag {
                    VEvent::Drag(dx, dy)
                } else if btn == mouse::Button::Middle && st.shift || btn == mouse::Button::Right {
                    VEvent::Pan(dx, dy)
                } else {
                    VEvent::Orbit(dx, dy)
                };
                Some(canvas::Action::publish((self.on)(m)).and_capture())
            }
            Event::Mouse(mouse::Event::ButtonReleased(_)) => {
                let (p, _, drag) = st.down.take()?;
                if st.moved {
                    return drag.then(|| canvas::Action::publish((self.on)(VEvent::DragEnd)));
                }
                let hit = st.tris.borrow().iter().rev().find(|t| inside(p, &t.0)).map(|t| t.2.clone());
                Some(canvas::Action::publish((self.on)(VEvent::Pick(hit, st.shift))))
            }
            Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                pos?;
                let d = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => *y,
                    mouse::ScrollDelta::Pixels { y, .. } => *y / 40.0,
                };
                Some(canvas::Action::publish((self.on)(VEvent::Zoom(d))).and_capture())
            }
            Event::Window(iced::window::Event::Resized(_)) => Some(canvas::Action::publish((self.on)(VEvent::Size(b.width, b.height)))),
            _ => None,
        }
    }

    fn draw(&self, st: &VState, r: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        let key = (self.cam, self.shade, self.gen, self.frame, self.picked.len(), b.size());
        if *st.key.borrow() != Some(key) {
            st.cache.clear();
            *st.key.borrow_mut() = Some(key);
        }
        let g = st.cache.draw(r, b.size(), |f: &mut Frame| self.paint(f, b.size(), st));
        vec![g]
    }

    fn mouse_interaction(&self, st: &VState, _b: Rectangle, _c: mouse::Cursor) -> mouse::Interaction {
        if st.down.map(|d| d.2).unwrap_or(false) { mouse::Interaction::Grabbing } else { mouse::Interaction::Crosshair }
    }
}

fn inside(p: Point, t: &[Point; 3]) -> bool {
    let s = |a: Point, b: Point, c: Point| (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
    let (d1, d2, d3) = (s(t[0], t[1], p), s(t[1], t[2], p), s(t[2], t[0], p));
    let neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
    let pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
    !(neg && pos)
}

impl<'a> Viewport<'a> {
    fn paint(&self, f: &mut Frame, size: Size, st: &VState) {
        let v = View::new(&self.cam, size);
        let grid = Color { r: 0.24, g: 0.21, b: 0.19, a: 1.0 };
        // the floor's grid (1 m), fading out
        if self.overlays[0] {
            for i in -24..=24 {
                let a = i as f32;
                for (p0, p1) in [([a, -24.0, 0.0], [a, 24.0, 0.0]), ([-24.0, a, 0.0], [24.0, a, 0.0])] {
                    if let (Some((x, _)), Some((y, _))) = (v.project(p0), v.project(p1)) {
                        f.stroke(&Path::line(x, y), Stroke::default().with_color(Color { a: if i % 4 == 0 { 0.8 } else { 0.45 }, ..grid }).with_width(1.0));
                    }
                }
            }
        }
        // a soft shadow under what's in the scene
        let (mut lo, mut hi) = ([f32::MAX; 2], [f32::MIN; 2]);
        for o in self.scene.objects.iter().filter(|o| !o.hidden && o.kind == "MESH") {
            let m = mat_at(o, self.scene, self.frame);
            for p in o.verts.iter().step_by(7) {
                let w = apply(m, *p);
                lo = [lo[0].min(w[0]), lo[1].min(w[1])];
                hi = [hi[0].max(w[0]), hi[1].max(w[1])];
            }
        }
        if lo[0] < hi[0] {
            let (cx, cy, rx, ry) = ((lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0, (hi[0] - lo[0]) * 0.75, (hi[1] - lo[1]) * 0.75);
            let pts: Vec<Point> = (0..48).filter_map(|i| {
                let t = i as f32 / 48.0 * std::f32::consts::TAU;
                v.project([cx + rx * t.cos(), cy + ry * t.sin(), 0.0]).map(|x| x.0)
            }).collect();
            if pts.len() > 3 {
                f.fill(&Path::new(|p| {
                    p.move_to(pts[0]);
                    for q in &pts[1..] {
                        p.line_to(*q);
                    }
                    p.close();
                }), Color { r: 0.0, g: 0.0, b: 0.0, a: 0.38 });
            }
        }
        // every face, far to near
        let light = norm([0.35, -0.55, 0.75]);
        let mut faces: Vec<([Point; 3], f32, Color, bool, String)> = Vec::new();
        for o in self.scene.objects.iter().filter(|o| !o.hidden) {
            let m = mat_at(o, self.scene, self.frame);
            let w: Vec<[f32; 3]> = o.verts.iter().map(|p| apply(m, *p)).collect();
            let sel = self.picked.contains(&o.name);
            for t in &o.tris {
                let (a, b2, c) = (w[t[0] as usize], w[t[1] as usize], w[t[2] as usize]);
                let n = norm(cross(sub(b2, a), sub(c, a)));
                let mid = [(a[0] + b2[0] + c[0]) / 3.0, (a[1] + b2[1] + c[1]) / 3.0, (a[2] + b2[2] + c[2]) / 3.0];
                if self.shade != Shade::Wire && dot(n, sub(mid, v.eye)) > 0.0 {
                    continue;
                }
                let (Some(pa), Some(pb), Some(pc)) = (v.project(a), v.project(b2), v.project(c)) else { continue };
                let base = if self.shade == Shade::Solid { [o.color[0] * 0.8 + 0.1, o.color[1] * 0.8 + 0.1, o.color[2] * 0.8 + 0.1] } else { o.color };
                faces.push(([pa.0, pb.0, pc.0], (pa.1 + pb.1 + pc.1) / 3.0, shade_of(base, n, light), sel, o.name.clone()));
            }
        }
        faces.sort_by(|x, y| y.1.partial_cmp(&x.1).unwrap_or(std::cmp::Ordering::Equal));
        let edge = Color::from_rgb8(0x1a, 0x16, 0x14);
        for (t, _, c, sel, _) in &faces {
            let path = Path::new(|p| {
                p.move_to(t[0]);
                p.line_to(t[1]);
                p.line_to(t[2]);
                p.close();
            });
            if self.shade == Shade::Wire {
                f.stroke(&path, Stroke::default().with_color(if *sel { self.accent } else { Color::from_rgb8(0x9a, 0x90, 0x88) }).with_width(1.0));
            } else {
                f.fill(&path, *c);
                f.stroke(&path, Stroke::default().with_color(if *sel { Color { a: 0.55, ..self.accent } } else { Color { a: 0.35, ..edge } }).with_width(if *sel { 1.4 } else { 0.6 }));
            }
        }
        *st.tris.borrow_mut() = faces.into_iter().map(|x| (x.0, x.1, x.4)).collect();
        // Move's arrows on the picked object (X red, Y green, Z blue) and its middle
        if self.overlays[1] {
            if let Some(o) = self.picked.first().and_then(|n| self.scene.objects.iter().find(|o| &o.name == n)) {
                let m = mat_at(o, self.scene, self.frame);
                let c = [m[12], m[13], m[14] + 0.0];
                let top = o.verts.iter().map(|p| apply(m, *p)[2]).fold(c[2], f32::max);
                let c = [c[0], c[1], top];
                if let Some((pc, _)) = v.project(c) {
                    for (d, col) in [([3.0, 0.0, 0.0], Color::from_rgb8(0xff, 0x6b, 0x6b)), ([0.0, 3.0, 0.0], Color::from_rgb8(0x9b, 0xe3, 0x6b)), ([0.0, 0.0, 3.0], Color::from_rgb8(0x5e, 0xc8, 0xff))] {
                        if let Some((pe, _)) = v.project([c[0] + d[0], c[1] + d[1], c[2] + d[2]]) {
                            f.stroke(&Path::line(pc, pe), Stroke::default().with_color(col).with_width(3.0));
                            let (dx, dy) = (pe.x - pc.x, pe.y - pc.y);
                            let l = (dx * dx + dy * dy).sqrt().max(1.0);
                            let (ux, uy) = (dx / l, dy / l);
                            f.fill(&Path::new(|p| {
                                p.move_to(Point::new(pe.x + ux * 9.0, pe.y + uy * 9.0));
                                p.line_to(Point::new(pe.x - uy * 6.0, pe.y + ux * 6.0));
                                p.line_to(Point::new(pe.x + uy * 6.0, pe.y - ux * 6.0));
                                p.close();
                            }), col);
                        }
                    }
                    f.fill(&Path::circle(pc, 5.0), Color::WHITE);
                }
            }
        }
        // the axes in the corner
        if self.overlays[2] {
            let o = Point::new(size.width - 70.0, 70.0);
            f.fill(&Path::circle(o, 42.0), Color { r: 0.0, g: 0.0, b: 0.0, a: 0.38 });
            let mut axes = [("X", [1.0f32, 0.0, 0.0], Color::from_rgb8(0xff, 0x6b, 0x6b)), ("Y", [0.0, 1.0, 0.0], Color::from_rgb8(0x9b, 0xe3, 0x6b)), ("Z", [0.0, 0.0, 1.0], Color::from_rgb8(0x5e, 0xc8, 0xff))];
            axes.sort_by(|a, b| dot(b.1, v.fwd).partial_cmp(&dot(a.1, v.fwd)).unwrap_or(std::cmp::Ordering::Equal));
            for (l, d, col) in axes {
                let e = Point::new(o.x + dot(d, v.right) * 32.0, o.y - dot(d, v.up) * 32.0);
                f.stroke(&Path::line(o, e), Stroke::default().with_color(col).with_width(2.5));
                f.fill(&Path::circle(e, 8.5), col);
                f.fill_text(Text { content: l.to_string(), position: Point::new(e.x - 3.5, e.y - 6.5), color: Color::from_rgb8(0x1b, 0x0f, 0x06), size: iced::Pixels(10.0), font: iced::Font { weight: iced::font::Weight::Bold, ..iced::Font::with_name("Inter") }, ..Text::default() });
            }
        }
    }
}
