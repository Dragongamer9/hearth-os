//! Ithil's live 3D / 4D brain (IthilBrain.qml and ithil-brain.js, ported as they are): the same seeded point cloud,
//! the same parts and colours, lit by her real event log (`hearth-server-mode ithil-brain HOURS`, read-only). live:
//! the last seconds as they happen; otherwise tpos moves through the window (0..120, the 4th dimension).
//! Brain regions are analogies, not equivalents (her app's own words).

use iced::widget::canvas::{self, Frame, Geometry, Path, Stroke};
use iced::{mouse, Color, Point, Rectangle, Renderer, Size, Theme};
use serde_json::Value;
use std::sync::OnceLock;

pub struct Part {
    pub name: &'static str,
    pub hex: u32,
}
pub const R: [Part; 11] = [
    Part { name: "Senses", hex: 0xe2e8f0 },
    Part { name: "Knowledge", hex: 0x2dd4bf },
    Part { name: "Event log", hex: 0x4ade80 },
    Part { name: "Workspace", hex: 0xfb923c },
    Part { name: "Confidence tags", hex: 0xfacc15 },
    Part { name: "Attention record", hex: 0x60a5fa },
    Part { name: "Self-model", hex: 0xc084fc },
    Part { name: "Prediction", hex: 0x22d3ee },
    Part { name: "Needs & mood", hex: 0xf472b6 },
    Part { name: "Sleep cycle", hex: 0x818cf8 },
    Part { name: "Hard-limit gate", hex: 0xef4444 },
];
const IDS: [&str; 11] = ["senses", "knowledge", "memory", "workspace", "confidence", "attention", "selfmodel", "prediction", "needs", "sleep", "gate"];
fn ri(id: &str) -> i32 {
    IDS.iter().position(|x| *x == id).map(|i| i as i32).unwrap_or(-1)
}
pub fn col(i: usize) -> (f32, f32, f32) {
    let h = R[i].hex;
    (((h >> 16) & 255) as f32, ((h >> 8) & 255) as f32, (h & 255) as f32)
}
pub const TOTAL: f32 = 120.0;

#[derive(Clone, Copy)]
pub enum Kind {
    Cortex,
    Cb,
    Stem,
    Deep,
}
#[derive(Clone, Copy)]
pub struct Pt {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub reg: i32,
    pub kind: Kind,
}

pub struct Cloud {
    pub p: Vec<Pt>,
    pub cent: Vec<Vec<[f64; 3]>>,
}

struct Rng(u32);
impl Rng {
    // ithil-brain.js's rnd() (mulberry32), exactly, so the brain has the same shape.
    fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x6D2B79F5);
        let s = self.0;
        let mut t = (s ^ (s >> 15)).wrapping_mul(1 | s);
        t = (t.wrapping_add((t ^ (t >> 7)).wrapping_mul(61 | t))) ^ t;
        ((t ^ (t >> 14)) as f64) / 4294967296.0
    }
    fn sph(&mut self) -> [f64; 3] {
        let u = self.next() * 2.0 - 1.0;
        let th = self.next() * std::f64::consts::PI * 2.0;
        let s = (1.0 - u * u).sqrt();
        [s * th.cos(), u, s * th.sin()]
    }
}

pub fn cloud() -> &'static Cloud {
    static C: OnceLock<Cloud> = OnceLock::new();
    C.get_or_init(build)
}

fn build() -> Cloud {
    let mut r = Rng(7);
    let mut p: Vec<Pt> = Vec::with_capacity(7000);
    let mut add = |p: &mut Vec<Pt>, x: f64, y: f64, z: f64, reg: i32, kind: Kind| p.push(Pt { x, y, z, reg, kind });
    let zones: [(&str, [f64; 3], f64); 5] = [
        ("workspace", [0.35, 0.30, 0.78], 0.50),
        ("confidence", [0.25, -0.15, 0.90], 0.33),
        ("attention", [0.42, 0.62, -0.30], 0.45),
        ("senses", [0.30, 0.10, -0.95], 0.40),
        ("knowledge", [0.80, -0.22, 0.18], 0.42),
    ];
    for s in [-1.0f64, 1.0] {
        for _ in 0..2700 {
            let [ux, uy, uz] = r.sph();
            let medial = ux * s < 0.0;
            let g = 1.0 + 0.045 * (9.0 * ux + 7.0 * uy).sin() * (8.0 * uz + 5.0 * uy).sin() + 0.03 * (17.0 * uy + 3.0 * uz + ux * 5.0).sin();
            let x = s * 0.33 + ux * (if medial { 0.26 } else { 0.60 }) * g;
            let mut y = 0.1 + uy * 0.58 * g;
            let z = uz * 1.0 * g;
            if y < -0.32 {
                y = -0.32 + (y + 0.32) * 0.35;
            }
            if medial && x.abs() < 0.03 {
                continue;
            }
            let mut reg = -1;
            let mut best = 9.0;
            for (id, c, rad) in &zones {
                let d = ((x.abs() - c[0]).powi(2) + (y - c[1]).powi(2) + (z - c[2]).powi(2)).sqrt();
                if d < *rad && d / rad < best {
                    best = d / rad;
                    reg = ri(id);
                }
            }
            if medial && x.abs() < 0.12 && y > 0.1 && z < -0.2 && z > -0.8 {
                reg = ri("selfmodel");
            }
            add(&mut p, x, y, z, reg, Kind::Cortex);
        }
    }
    for _ in 0..900 {
        let [ux, uy, uz] = r.sph();
        let g = 1.0 + 0.06 * (40.0 * uy).sin();
        add(&mut p, ux * 0.5 * g, -0.40 + uy * 0.2 * g, -0.62 + uz * 0.3 * g, ri("prediction"), Kind::Cb);
    }
    for _ in 0..360 {
        let t = r.next();
        let a = r.next() * std::f64::consts::PI * 2.0;
        let rr = 0.09 + 0.02 * r.next();
        add(&mut p, a.cos() * rr, -0.30 - 0.65 * t, -0.22 - 0.12 * t + a.sin() * rr, ri("gate"), Kind::Stem);
    }
    for s in [-1.0f64, 1.0] {
        for _ in 0..260 {
            let t = r.next();
            let a = r.next() * std::f64::consts::PI * 2.0;
            let rr = 0.055;
            add(&mut p, s * (0.30 + 0.06 * t) + a.cos() * rr, -0.24 + 0.12 * (std::f64::consts::PI * t).sin() + a.sin() * rr, 0.25 - 0.62 * t, ri("memory"), Kind::Deep);
        }
    }
    for s in [-1.0f64, 1.0] {
        for _ in 0..200 {
            let [ux, uy, uz] = r.sph();
            let k = r.next().cbrt();
            add(&mut p, s * 0.11 + ux * 0.10 * k, 0.02 + uy * 0.09 * k, -0.05 + uz * 0.15 * k, ri("sleep"), Kind::Deep);
        }
    }
    for _ in 0..160 {
        let [ux, uy, uz] = r.sph();
        let k = r.next().cbrt();
        add(&mut p, ux * 0.08 * k, -0.17 + uy * 0.07 * k, 0.14 + uz * 0.09 * k, ri("needs"), Kind::Deep);
    }
    for s in [-1.0f64, 1.0] {
        for _ in 0..90 {
            let [ux, uy, uz] = r.sph();
            let k = r.next().cbrt();
            add(&mut p, s * 0.33 + ux * 0.07 * k, -0.30 + uy * 0.06 * k, 0.30 + uz * 0.07 * k, ri("needs"), Kind::Deep);
        }
    }
    for _ in 0..140 {
        let [ux, uy, uz] = r.sph();
        let k = r.next().cbrt();
        add(&mut p, ux * 0.05 * k, 0.34 + uy * 0.12 * k, -0.50 + uz * 0.18 * k, ri("selfmodel"), Kind::Deep);
    }
    // the parts' centres (left and right apart when a part has both)
    let mut cent = vec![];
    for i in 0..R.len() as i32 {
        let (mut l, mut rr) = ([0.0f64; 4], [0.0f64; 4]);
        for q in p.iter().filter(|q| q.reg == i) {
            let b = if q.x < 0.0 { &mut l } else { &mut rr };
            b[0] += q.x;
            b[1] += q.y;
            b[2] += q.z;
            b[3] += 1.0;
        }
        let tot = l[3] + rr[3];
        let mut c = vec![];
        if tot > 0.0 && l[3] > tot * 0.25 && rr[3] > tot * 0.25 && (l[0] / l[3]).abs() > 0.15 {
            c.push([l[0] / l[3], l[1] / l[3], l[2] / l[3]]);
            c.push([rr[0] / rr[3], rr[1] / rr[3], rr[2] / rr[3]]);
        } else if tot > 0.0 {
            c.push([(l[0] + rr[0]) / tot, (l[1] + rr[1]) / tot, (l[2] + rr[2]) / tot]);
        } else {
            c.push([0.0, 0.0, 0.0]);
        }
        cent.push(c);
    }
    Cloud { p, cent }
}

// ------------------------------------------------------------------ her log → the timeline
#[derive(Clone, Debug, Default)]
pub struct Ev {
    pub ms: f64,
    pub t: f32,
    pub reg: usize,
    pub bc: bool,
    pub block: bool,
    pub quiet: bool,
}
#[derive(Clone, Debug, Default)]
pub struct Prepared {
    pub from: f64,
    pub to: f64,
    pub ev: Vec<Ev>,
    pub sleeps: Vec<(f32, f32)>,
}

pub fn prepare(d: Option<&Value>) -> Prepared {
    let Some(d) = d else { return Prepared::default() };
    let num = |v: &Value, k: &str| v.get(k).and_then(Value::as_f64).unwrap_or(0.0);
    let (from, to) = (num(d, "from"), num(d, "to"));
    let span = (to - from).max(1.0);
    let tp = |ms: f64| ((ms - from) / span * TOTAL as f64) as f32;
    let mut out = Prepared { from, to, ..Default::default() };
    for e in d.get("events").and_then(Value::as_array).into_iter().flatten() {
        let r = ri(e.get("reg").and_then(Value::as_str).unwrap_or(""));
        if r < 0 {
            continue;
        }
        let b = |k: &str| e.get(k).and_then(Value::as_bool).unwrap_or(false);
        let ms = num(e, "ms");
        out.ev.push(Ev { ms, t: tp(ms), reg: r as usize, bc: b("bc"), block: b("block"), quiet: b("quiet") });
    }
    for s in d.get("sleeps").and_then(Value::as_array).into_iter().flatten() {
        out.sleeps.push((tp(num(s, "from")), tp(num(s, "to"))));
    }
    out
}

pub fn sleepiness(d: &Prepared, t: f32) -> f32 {
    for &(a, b) in &d.sleeps {
        let b = b.max(a + 0.8);
        if t >= a - 0.5 && t <= b + 1.0 {
            return (t - a + 0.5).min(b + 1.0 - t).min(1.0);
        }
    }
    0.0
}

/// How lit each part is (0..1): live, real seconds since each entry; otherwise timeline seconds from `t`.
pub fn activity(d: &Prepared, t: f32, now_ms: f64, live: bool, asleep: bool) -> Vec<f32> {
    let n = R.len();
    let mut a: Vec<f32> = (0..n).map(|i| 0.06 + 0.03 * ((now_ms / 1000.0 * 1.7 + i as f64 * 2.3).sin() as f32)).collect();
    let sl = if live { if asleep { 1.0 } else { 0.0 } } else { sleepiness(d, t) };
    let (gate, sleep, memory, knowledge) = (10, 9, 2, 1);
    for e in &d.ev {
        let dd = if live { ((now_ms - e.ms) / 1000.0 / 1.5) as f32 } else { t - e.t };
        if !(-0.6..=2.5).contains(&dd) {
            continue;
        }
        let g = if dd < 0.0 { (-(dd * dd) / 0.04).exp() } else { (-dd / 0.7).exp() };
        a[e.reg] += 0.95 * g;
        if e.bc {
            for (k, v) in a.iter_mut().enumerate() {
                if k != e.reg && k != gate {
                    *v += 0.35 * g * (1.0 - (dd - 0.6).abs()).max(0.0);
                }
            }
        }
    }
    for (i, v) in a.iter_mut().enumerate() {
        if i != sleep && i != memory && i != knowledge {
            *v *= 1.0 - 0.6 * sl;
        }
    }
    a[sleep] += 0.55 * sl;
    a[gate] += 0.12;
    a.iter().map(|v| v.clamp(0.0, 1.0)).collect()
}

// ------------------------------------------------------------------ drawing it
pub struct Brain {
    pub data: Prepared,
    pub yaw: f32,
    pub pitch: f32,
    pub zoom: f32,
    pub xray: bool,
    pub live: bool,
    pub asleep: bool,
    pub t: f32,
    pub now_ms: f64,
    /// The tile's small one: every third point, bigger.
    pub step: usize,
}

/// Her brain's drawing, kept until what it shows changes (the tile's turns twice a second; the dashboard's other
/// frames reuse it).
#[derive(Default)]
pub struct BrainMemo {
    cache: canvas::Cache,
    key: std::cell::RefCell<String>,
}

impl<M> canvas::Program<M> for Brain {
    type State = BrainMemo;
    fn draw(&self, st: &BrainMemo, renderer: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        let key = format!("{} {} {} {} {} {} {} {} {} {:?}", self.yaw, self.pitch, self.zoom, self.xray, self.live, self.asleep, self.t, self.now_ms, self.data.ev.len(), b.size());
        if *st.key.borrow() != key {
            st.cache.clear();
            *st.key.borrow_mut() = key;
        }
        vec![st.cache.draw(renderer, b.size(), |f| self.paint(f, b.width, b.height))]
    }
}

impl Brain {
    fn paint(&self, f: &mut Frame, w: f32, h: f32) {
        if w < 2.0 || h < 2.0 {
            return;
        }
        let c = cloud();
        let act = activity(&self.data, self.t, self.now_ms, self.live, self.asleep);
        let (cy, sy, cp, sp) = (self.yaw.cos() as f64, self.yaw.sin() as f64, self.pitch.cos() as f64, self.pitch.sin() as f64);
        let sc0 = (w.min(h) * 0.36 * self.zoom) as f64;
        let (wd, hd) = (w as f64, h as f64);
        let proj = |x: f64, y: f64, z: f64| -> (f32, f32, f32, f32) {
            let x1 = x * cy + z * sy;
            let z1 = -x * sy + z * cy;
            let y1 = y * cp - z1 * sp;
            let z2 = y * sp + z1 * cp;
            let per = 3.2 / (3.2 - z2);
            ((wd / 2.0 + x1 * sc0 * per) as f32, (hd * 0.5 - (y1 - 0.05) * sc0 * per) as f32, z2 as f32, per as f32)
        };
        let step = self.step.max(1);
        let mut ord: Vec<(usize, f32, f32, f32, f32)> = (0..c.p.len()).step_by(step).map(|i| {
            let q = c.p[i];
            let (x, y, z, per) = proj(q.x, q.y, q.z);
            (i, x, y, z, per)
        }).collect();
        ord.sort_by(|a, b| a.3.partial_cmp(&b.3).unwrap_or(std::cmp::Ordering::Equal));
        let big = if step > 1 { 1.5 } else { 1.0 };
        for (i, x, y, z, per) in ord {
            let q = c.p[i];
            let shade = 0.5 + 0.5 * ((z + 1.0) / 2.0).clamp(0.0, 1.0);
            let (mut r, mut g, mut bl, mut al, mut a) = (150.0, 135.0, 110.0, 0.28f32, 0.0f32);
            if q.reg >= 0 {
                a = act[q.reg as usize];
                let (cr, cg, cb) = col(q.reg as usize);
                let ww = a * 0.45;
                r = cr + (255.0 - cr) * ww;
                g = cg + (255.0 - cg) * ww;
                bl = cb + (255.0 - cb) * ww;
                al = 0.22 + 0.78 * a;
            }
            if self.xray {
                if matches!(q.kind, Kind::Cortex) {
                    al *= if q.reg < 0 { 0.10 } else { 0.22 };
                } else {
                    al = (al * 1.6 + 0.2).min(1.0);
                }
            } else if matches!(q.kind, Kind::Deep) {
                al *= 0.55;
            }
            let s = (1.1 + 1.8 * a) * per * big;
            f.fill_rectangle(Point::new(x - s / 2.0, y - s / 2.0), Size::new(s, s), Color::from_rgba(r * shade / 255.0, g * shade / 255.0, bl * shade / 255.0, al));
        }
        // the glow of the parts that are lit
        for (ri_, cs) in c.cent.iter().enumerate() {
            let aa = act[ri_];
            if aa < 0.18 {
                continue;
            }
            let (cr, cg, cb) = col(ri_);
            for cc in cs {
                let (x, y, _, per) = proj(cc[0], cc[1], cc[2]);
                let rad = (25.0 + 55.0 * aa) * per * self.zoom * if w / 900.0 > 0.5 { 1.0 } else { 0.6 };
                for (k, fr) in [(0.12f32, 1.0f32), (0.10, 0.7), (0.10, 0.4)] {
                    f.fill(&Path::circle(Point::new(x, y), rad * fr), Color::from_rgba(cr / 255.0, cg / 255.0, cb / 255.0, k * aa));
                }
            }
        }
        // a broadcast: lines from the workspace to every part, with a dot running down them
        let ws = c.cent[3][0];
        let o = proj(ws[0], ws[1], ws[2]);
        for e in self.data.ev.iter().filter(|e| e.bc) {
            let dd = if self.live { ((self.now_ms - e.ms) / 1000.0 / 1.5) as f32 } else { self.t - e.t };
            if !(0.0..=1.4).contains(&dd) {
                continue;
            }
            let pr = (dd / 0.9).min(1.0);
            let fade = 1.0 - dd / 1.4;
            for (rj, cs) in c.cent.iter().enumerate() {
                if rj == 3 || rj == 10 {
                    continue;
                }
                for c2 in cs {
                    let q2 = proj(c2[0], c2[1], c2[2]);
                    f.stroke(&Path::line(Point::new(o.0, o.1), Point::new(q2.0, q2.1)), Stroke::default().with_color(Color::from_rgba(251.0 / 255.0, 146.0 / 255.0, 60.0 / 255.0, 0.35 * fade)).with_width(1.2));
                    f.fill(&Path::circle(Point::new(o.0 + (q2.0 - o.0) * pr, o.1 + (q2.1 - o.1) * pr), 3.0), Color::from_rgba(1.0, 220.0 / 255.0, 180.0 / 255.0, fade));
                }
            }
        }
    }
}

/// The timeline under her brain: her sleeps shaded, her events in their part's colour.
pub struct Strip {
    pub data: Prepared,
}
impl<M> canvas::Program<M> for Strip {
    type State = ();
    fn draw(&self, _s: &(), renderer: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        let mut f = Frame::new(renderer, b.size());
        let (w, h) = (b.width, b.height);
        for &(a, z) in &self.data.sleeps {
            f.fill_rectangle(Point::new(a / TOTAL * w, 0.0), Size::new(((z - a) / TOTAL * w).max(3.0), h), Color::from_rgba(129.0 / 255.0, 140.0 / 255.0, 248.0 / 255.0, 0.25));
        }
        for e in self.data.ev.iter().filter(|e| !e.quiet) {
            let (r, g, bl) = col(e.reg);
            let y = if e.block { 0.0 } else { h * 0.3 };
            f.fill_rectangle(Point::new(e.t / TOTAL * w, y), Size::new(3.0, h - y), Color::from_rgba(r / 255.0, g / 255.0, bl / 255.0, if e.block { 1.0 } else { 0.75 }));
        }
        vec![f.into_geometry()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_brain_has_its_shape_and_parts() {
        let c = cloud();
        assert!(c.p.len() > 6000 && c.p.len() < 7200, "{}", c.p.len());
        assert_eq!(c.cent.len(), R.len());
        // the first numbers of ithil-brain.js's seeded rnd() (seed 7)
        let mut r = Rng(7);
        let a = r.next();
        assert!((0.0..1.0).contains(&a));
        let d = serde_json::json!({"from": 0, "to": 120000, "events": [{"reg": "gate", "ms": 60000, "block": true}], "sleeps": [{"from": 10000, "to": 20000}]});
        let p = prepare(Some(&d));
        assert_eq!(p.ev.len(), 1);
        assert!((p.ev[0].t - 60.0).abs() < 0.01);
        assert!(sleepiness(&p, 15.0) > 0.5);
        assert!(activity(&p, 60.0, 0.0, false, false)[10] > 0.9);
    }
}
