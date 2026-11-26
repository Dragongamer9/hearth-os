//! The viewport: the scene drawn in Rust on iced's canvas (wgpu where the graphics card starts, tiny-skia otherwise).
//! Each object's mesh (as Blender made it, changes on top applied) is moved to where it is at this frame, seen
//! through the camera (orbit, pan, zoom; perspective or flat), its faces lit by one light and drawn far to near; the
//! picked object outlined in the accent with Move's arrows on it; the floor's grid, a soft shadow under the scene and
//! the axes in the corner. Drawn again only when something it shows changes (a cache).

use crate::scene::{EditMesh, Obj, Scene};
use iced::widget::canvas::{self, Cache, Frame, Geometry, Path, Stroke, Text};
use hearth_ui::Pal;
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
        Cam { yaw: -0.785, pitch: 0.5, dist: 15.0, target: [0.0, 0.0, 2.0], persp: true }
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

/// What a press and a drag in the viewport do (the mode and the tool).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VMode {
    /// pick, orbit, move the picked (Select, Move, Turn, Size)
    Object,
    /// Box select: a drawn box picks every object in it
    Boxes,
    /// Tab: the picked mesh's points (0), edges (1) or faces (2)
    Edit(u8),
    /// brushes on the picked mesh: Sculpt or Paint
    Brush,
}

/// The inverse of a matrix's 3 × 3 part (world directions → the object's own).
pub fn inv3(m: &[f32; 16]) -> [[f32; 3]; 3] {
    let a = [[m[0], m[4], m[8]], [m[1], m[5], m[9]], [m[2], m[6], m[10]]];
    let det = a[0][0] * (a[1][1] * a[2][2] - a[1][2] * a[2][1]) - a[0][1] * (a[1][0] * a[2][2] - a[1][2] * a[2][0]) + a[0][2] * (a[1][0] * a[2][1] - a[1][1] * a[2][0]);
    let d = if det.abs() < 1e-12 { 1.0 } else { 1.0 / det };
    [
        [(a[1][1] * a[2][2] - a[1][2] * a[2][1]) * d, (a[0][2] * a[2][1] - a[0][1] * a[2][2]) * d, (a[0][1] * a[1][2] - a[0][2] * a[1][1]) * d],
        [(a[1][2] * a[2][0] - a[1][0] * a[2][2]) * d, (a[0][0] * a[2][2] - a[0][2] * a[2][0]) * d, (a[0][2] * a[1][0] - a[0][0] * a[1][2]) * d],
        [(a[1][0] * a[2][1] - a[1][1] * a[2][0]) * d, (a[0][1] * a[2][0] - a[0][0] * a[2][1]) * d, (a[0][0] * a[1][1] - a[0][1] * a[1][0]) * d],
    ]
}
pub fn mul3(m: &[[f32; 3]; 3], v: [f32; 3]) -> [f32; 3] {
    [m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2], m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2], m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2]]
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
    pub mode: VMode,
    /// the picked object's own mesh while editing, sculpting or painting (drawn instead of Blender's)
    pub edit: Option<&'a EditMesh>,
    /// which of its points are picked (Tab)
    pub vsel: &'a [bool],
    /// the brush's size on the screen (Sculpt, Paint) and its colour (Paint)
    pub brush: f32,
    pub paint: Option<Color>,
    pub egen: u64,
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
    /// Box select: the objects in the drawn box (Shift: added to what's picked)
    BoxPick(Vec<String>, bool),
    /// Tab: points picked by a click or a drawn box (Shift: added; a click on nothing: none)
    EditPick(Vec<u32>, bool, bool),
    /// Tab: the picked points dragged, by this much in the world, and the pointer's move on the screen
    EditMove([f32; 3], f32, f32),
    EditMoveEnd,
    /// Sculpt / Paint: the brush on the mesh at this point (the object's own space), its radius there, and which
    /// way the camera looks (also in the object's own space)
    Brush([f32; 3], f32, [f32; 3]),
    BrushEnd,
}

#[derive(Default)]
pub struct VState {
    cache: Cache,
    key: RefCell<Option<(Cam, Shade, u64, i64, usize, Size)>>,
    tris: RefCell<Vec<([Point; 3], f32, String)>>,
    down: Option<(Point, mouse::Button, bool)>,
    moved: bool,
    shift: bool,
    /// the drawn box (from, to) while it's being drawn
    boxing: Option<(Point, Point)>,
    /// the edit mesh on the screen: each point's place and the triangles (points, depth)
    epts: RefCell<Vec<Option<Point>>>,
    etris: RefCell<Vec<([u32; 3], [Point; 3], f32)>>,
    /// Tab: the drag started on a picked point (moves them) and where the picked points' middle is
    emove: Option<[f32; 3]>,
    brushing: bool,
    at: Option<Point>,
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
                st.moved = false;
                if *btn == mouse::Button::Left {
                    match self.mode {
                        VMode::Boxes => {
                            st.boxing = Some((p, p));
                            st.down = Some((p, *btn, false));
                            return Some(canvas::Action::request_redraw().and_capture());
                        }
                        VMode::Edit(_) => {
                            // on a picked point: dragging moves the picked points; elsewhere: a box picks points
                            let near = self.near_point(st, p, 10.0);
                            if near.map(|i| self.vsel.get(i as usize).copied().unwrap_or(false)).unwrap_or(false) {
                                st.emove = Some(self.sel_middle());
                            } else {
                                st.boxing = Some((p, p));
                            }
                            st.down = Some((p, *btn, false));
                            return Some(canvas::Action::capture());
                        }
                        VMode::Brush => {
                            st.down = Some((p, *btn, false));
                            st.brushing = true;
                            return self.brush_at(st, p, b.size()).map(|m| canvas::Action::publish((self.on)(m)).and_capture()).or(Some(canvas::Action::capture()));
                        }
                        VMode::Object => {}
                    }
                }
                // on the picked object (its Move arrows): dragging moves it
                let on_picked = st.tris.borrow().iter().rev().find(|t| inside(p, &t.0)).map(|t| self.picked.contains(&t.2)).unwrap_or(false);
                st.down = Some((p, *btn, on_picked && *btn == mouse::Button::Left));
                Some(canvas::Action::capture())
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                st.at = pos;
                if st.down.is_none() {
                    // the brush's ring follows the pointer
                    return (self.mode == VMode::Brush).then(canvas::Action::request_redraw);
                }
                let (start, btn, drag) = st.down?;
                let p = cursor.position_in(b).or_else(|| cursor.position().map(|q| Point::new(q.x - b.x, q.y - b.y)))?;
                if let Some((from, _)) = st.boxing {
                    st.boxing = Some((from, p));
                    st.moved = true;
                    return Some(canvas::Action::request_redraw().and_capture());
                }
                if st.brushing {
                    st.moved = true;
                    return self.brush_at(st, p, b.size()).map(|m| canvas::Action::publish((self.on)(m)).and_capture()).or(Some(canvas::Action::request_redraw().and_capture()));
                }
                if let Some(mid) = st.emove {
                    let (dx, dy) = (p.x - start.x, p.y - start.y);
                    if !st.moved && dx.abs() + dy.abs() < 3.0 {
                        return None;
                    }
                    st.moved = true;
                    let v = View::new(&self.cam, b.size());
                    return Some(canvas::Action::publish((self.on)(VEvent::EditMove(v.screen_to_world(mid, dx, dy), dx, dy))).and_capture());
                }
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
                if let Some((a, z)) = st.boxing.take() {
                    st.down = None;
                    let r = Rectangle::new(Point::new(a.x.min(z.x), a.y.min(z.y)), Size::new((a.x - z.x).abs(), (a.y - z.y).abs()));
                    let small = r.width < 4.0 && r.height < 4.0;
                    return Some(canvas::Action::publish((self.on)(match self.mode {
                        VMode::Edit(k) if small => VEvent::EditPick(self.pick_at(st, a, k), st.shift, true),
                        VMode::Edit(_) => VEvent::EditPick(st.epts.borrow().iter().enumerate().filter(|(_, q)| q.map(|q| r.contains(q)).unwrap_or(false)).map(|(i, _)| i as u32).collect(), st.shift, false),
                        _ if small => VEvent::Pick(st.tris.borrow().iter().rev().find(|t| inside(a, &t.0)).map(|t| t.2.clone()), st.shift),
                        _ => VEvent::BoxPick(self.objects_in(r, b.size()), st.shift),
                    })));
                }
                if st.brushing {
                    st.brushing = false;
                    st.down = None;
                    return Some(canvas::Action::publish((self.on)(VEvent::BrushEnd)));
                }
                if st.emove.take().is_some() {
                    st.down = None;
                    return st.moved.then(|| canvas::Action::publish((self.on)(VEvent::EditMoveEnd)));
                }
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
        let key = (self.cam, self.shade, self.gen ^ (self.egen << 20), self.frame, self.picked.len(), b.size());
        if *st.key.borrow() != Some(key) {
            st.cache.clear();
            *st.key.borrow_mut() = Some(key);
        }
        let g = st.cache.draw(r, b.size(), |f: &mut Frame| self.paint(f, b.size(), st));
        // what moves with the pointer, drawn over it each time: the drawn box, the brush's ring
        let mut f = Frame::new(r, b.size());
        if let Some((a, z)) = st.boxing {
            let tl = Point::new(a.x.min(z.x), a.y.min(z.y));
            let sz = Size::new((a.x - z.x).abs(), (a.y - z.y).abs());
            f.fill_rectangle(tl, sz, Color { a: 0.12, ..self.accent });
            f.stroke(&Path::rectangle(tl, sz), Stroke { line_dash: canvas::LineDash { segments: &[6.0, 4.0], offset: 0 }, ..Stroke::default().with_color(self.accent).with_width(1.5) });
        }
        if self.mode == VMode::Brush {
            if let Some(c) = st.at.or_else(|| _c.position_in(b)) {
                f.stroke(&Path::circle(c, self.brush), Stroke::default().with_color(Color::WHITE).with_width(1.5));
                if let Some(pc) = self.paint {
                    f.fill(&Path::circle(c, 4.0), pc);
                }
            }
        }
        vec![g, f.into_geometry()]
    }

    fn mouse_interaction(&self, st: &VState, _b: Rectangle, _c: mouse::Cursor) -> mouse::Interaction {
        if self.mode == VMode::Brush {
            return mouse::Interaction::Crosshair;
        }
        if st.down.map(|d| d.2).unwrap_or(false) || st.emove.is_some() { mouse::Interaction::Grabbing } else { mouse::Interaction::Crosshair }
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
    /// The point of the edit mesh nearest `p` on the screen, within `r` pixels.
    fn near_point(&self, st: &VState, p: Point, r: f32) -> Option<u32> {
        let pts = st.epts.borrow();
        pts.iter().enumerate().filter_map(|(i, q)| q.map(|q| (i, (q.x - p.x).hypot(q.y - p.y)))).filter(|x| x.1 <= r).min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)).map(|x| x.0 as u32)
    }
    /// What a click picks in Tab: a point, an edge's two points, or a face's points.
    fn pick_at(&self, st: &VState, p: Point, kind: u8) -> Vec<u32> {
        let Some(e) = self.edit else { return Vec::new() };
        match kind {
            0 => self.near_point(st, p, 10.0).into_iter().collect(),
            1 => {
                let pts = st.epts.borrow();
                let d = |a: Point, b: Point| {
                    let (vx, vy) = (b.x - a.x, b.y - a.y);
                    let t = (((p.x - a.x) * vx + (p.y - a.y) * vy) / (vx * vx + vy * vy).max(1e-6)).clamp(0.0, 1.0);
                    (a.x + vx * t - p.x).hypot(a.y + vy * t - p.y)
                };
                e.edges.iter().filter_map(|ed| Some((ed, d(pts.get(ed[0] as usize).copied().flatten()?, pts.get(ed[1] as usize).copied().flatten()?)))).filter(|x| x.1 <= 8.0).min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)).map(|x| vec![x.0[0], x.0[1]]).unwrap_or_default()
            }
            _ => {
                let tr = st.etris.borrow();
                let hit = tr.iter().find(|t| inside(p, &t.1)).map(|t| t.0);
                hit.and_then(|t| e.faces.iter().find(|f| t.iter().all(|i| f.contains(i)))).cloned().unwrap_or_default()
            }
        }
    }
    /// The middle of the picked points, in the world.
    fn sel_middle(&self) -> [f32; 3] {
        let (Some(e), Some(o)) = (self.edit, self.edit.and_then(|e| self.scene.objects.iter().find(|o| o.name == e.obj))) else { return [0.0; 3] };
        let m = mat_at(o, self.scene, self.frame);
        let mut s = [0.0f32; 3];
        let mut n = 0.0;
        for (i, p) in e.verts.iter().enumerate() {
            if self.vsel.get(i).copied().unwrap_or(false) {
                let w = apply(m, *p);
                s = [s[0] + w[0], s[1] + w[1], s[2] + w[2]];
                n += 1.0;
            }
        }
        if n > 0.0 { [s[0] / n, s[1] / n, s[2] / n] } else { [m[12], m[13], m[14]] }
    }
    /// The objects with any of their points inside the drawn box.
    fn objects_in(&self, r: Rectangle, size: Size) -> Vec<String> {
        let v = View::new(&self.cam, size);
        self.scene.objects.iter().filter(|o| !o.hidden).filter(|o| {
            let m = mat_at(o, self.scene, self.frame);
            if o.verts.is_empty() {
                return v.project([m[12], m[13], m[14]]).map(|x| r.contains(x.0)).unwrap_or(false);
            }
            o.verts.iter().any(|p| v.project(apply(m, *p)).map(|x| r.contains(x.0)).unwrap_or(false))
        }).map(|o| o.name.clone()).collect()
    }
    /// Where the brush touches the mesh under `p` (the nearest triangle there), in the object's own space.
    fn brush_at(&self, st: &VState, p: Point, size: Size) -> Option<VEvent> {
        let e = self.edit?;
        let o = self.scene.objects.iter().find(|o| o.name == e.obj)?;
        let tr = st.etris.borrow();
        let (t, q, _) = tr.iter().find(|t| inside(p, &t.1))?;
        // barycentric on the screen, then the same blend of the three points
        let (a, b, c) = (q[0], q[1], q[2]);
        let den = (b.y - c.y) * (a.x - c.x) + (c.x - b.x) * (a.y - c.y);
        if den.abs() < 1e-6 {
            return None;
        }
        let l1 = ((b.y - c.y) * (p.x - c.x) + (c.x - b.x) * (p.y - c.y)) / den;
        let l2 = ((c.y - a.y) * (p.x - c.x) + (a.x - c.x) * (p.y - c.y)) / den;
        let l3 = 1.0 - l1 - l2;
        let (va, vb, vc) = (e.verts[t[0] as usize], e.verts[t[1] as usize], e.verts[t[2] as usize]);
        let at = [va[0] * l1 + vb[0] * l2 + vc[0] * l3, va[1] * l1 + vb[1] * l2 + vc[1] * l3, va[2] * l1 + vb[2] * l2 + vc[2] * l3];
        let m = mat_at(o, self.scene, self.frame);
        let v = View::new(&self.cam, size);
        let wat = apply(m, at);
        let edge = v.screen_to_world(wat, self.brush, 0.0);
        let inv = inv3(m);
        let r_local = { let l = mul3(&inv, edge); (l[0] * l[0] + l[1] * l[1] + l[2] * l[2]).sqrt() };
        let look = norm(mul3(&inv, v.fwd));
        Some(VEvent::Brush(at, r_local.max(1e-4), look))
    }
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
        let editing = self.edit.map(|e| e.obj.clone());
        for o in self.scene.objects.iter().filter(|o| !o.hidden && Some(&o.name) != editing.as_ref()) {
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
        // the mesh being edited, sculpted or painted: its own mesh (as Blender keeps it), with the paint on its points
        let mut etris: Vec<([u32; 3], [Point; 3], f32)> = Vec::new();
        let mut epts: Vec<Option<Point>> = Vec::new();
        if let (Some(e), Some(o)) = (self.edit, self.edit.and_then(|e| self.scene.objects.iter().find(|o| o.name == e.obj))) {
            let m = mat_at(o, self.scene, self.frame);
            let w: Vec<[f32; 3]> = e.verts.iter().map(|p| apply(m, *p)).collect();
            epts = w.iter().map(|p| v.project(*p).map(|x| x.0)).collect();
            for (t, fi) in e.tris() {
                let (a, b2, c) = (w[t[0] as usize], w[t[1] as usize], w[t[2] as usize]);
                let n = norm(cross(sub(b2, a), sub(c, a)));
                let mid = [(a[0] + b2[0] + c[0]) / 3.0, (a[1] + b2[1] + c[1]) / 3.0, (a[2] + b2[2] + c[2]) / 3.0];
                if dot(n, sub(mid, v.eye)) > 0.0 {
                    continue;
                }
                let (Some(pa), Some(pb), Some(pc)) = (v.project(a), v.project(b2), v.project(c)) else { continue };
                let base = if e.colors.len() == e.verts.len() {
                    let cs = [e.colors[t[0] as usize], e.colors[t[1] as usize], e.colors[t[2] as usize]];
                    [(cs[0][0] + cs[1][0] + cs[2][0]) / 3.0, (cs[0][1] + cs[1][1] + cs[2][1]) / 3.0, (cs[0][2] + cs[1][2] + cs[2][2]) / 3.0]
                } else {
                    [o.color[0] * 0.8 + 0.1, o.color[1] * 0.8 + 0.1, o.color[2] * 0.8 + 0.1]
                };
                let picked_face = matches!(self.mode, VMode::Edit(2)) && e.faces[fi].iter().all(|i| self.vsel.get(*i as usize).copied().unwrap_or(false));
                let col = if picked_face { Pal::mix(shade_of(base, n, light), self.accent, 0.45) } else { shade_of(base, n, light) };
                let d = (pa.1 + pb.1 + pc.1) / 3.0;
                faces.push(([pa.0, pb.0, pc.0], d, col, false, e.obj.clone()));
                etris.push((t, [pa.0, pb.0, pc.0], d));
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
        // Tab: the mesh's edges and points over it (the picked ones in the accent)
        if let (VMode::Edit(_), Some(e)) = (self.mode, self.edit) {
            let on = |i: u32| self.vsel.get(i as usize).copied().unwrap_or(false);
            for ed in &e.edges {
                if let (Some(a), Some(b2)) = (epts.get(ed[0] as usize).copied().flatten(), epts.get(ed[1] as usize).copied().flatten()) {
                    let lit = on(ed[0]) && on(ed[1]);
                    f.stroke(&Path::line(a, b2), Stroke::default().with_color(if lit { self.accent } else { Color::from_rgba8(0x10, 0x0c, 0x0a, 0.85) }).with_width(if lit { 2.0 } else { 1.0 }));
                }
            }
            for (i, q) in epts.iter().enumerate() {
                if let Some(q) = q {
                    let lit = on(i as u32);
                    f.fill(&Path::circle(*q, if lit { 3.5 } else { 2.5 }), if lit { self.accent } else { Color::from_rgb8(0x11, 0x0d, 0x0b) });
                }
            }
        }
        etris.sort_by(|x, y| x.2.partial_cmp(&y.2).unwrap_or(std::cmp::Ordering::Equal));
        *st.etris.borrow_mut() = etris;
        *st.epts.borrow_mut() = epts;
        // Move's arrows on the picked object (X red, Y green, Z blue) and its middle
        if self.overlays[1] && self.edit.is_none() {
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
