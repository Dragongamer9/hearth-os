//! The dashboard's drawings (iced canvas), each kept between frames and drawn again only when what it shows changes
//! (nothing redraws at rest): the dotted grid behind everything, a panel's gold corner marks, the last-hour graphs,
//! the AIs' cores (ServerCore.qml's shapes and colours, the Hearth AI Command Center's), the countdown's ring, the
//! web page's QR code and the line icons (Mantel's Icon names the helper sends: chat, image, film, ...).
//!
//! The cores stand still here (ServerCore.qml moves them while one works): one that's working shows its two rings
//! instead. Moving them would mean drawing the screen 30 times a second at rest, and Server mode must stay light.

use iced::widget::canvas::{self, Frame, Geometry, LineCap, LineJoin, Path, Stroke};
use iced::{mouse, Color, Element, Length, Point, Rectangle, Renderer, Size, Theme};
use std::cell::RefCell;

pub fn rgb(h: u32) -> Color {
    Color::from_rgb8((h >> 16) as u8, (h >> 8) as u8, h as u8)
}
pub fn a(c: Color, k: f32) -> Color {
    Color { a: c.a * k, ..c }
}

#[derive(Default)]
pub struct Memo {
    cache: canvas::Cache,
    key: RefCell<String>,
}

fn memo(st: &Memo, renderer: &Renderer, bounds: Rectangle, key: String, f: impl FnOnce(&mut Frame)) -> Vec<Geometry> {
    let k = format!("{key}|{:?}", bounds.size());
    if *st.key.borrow() != k {
        st.cache.clear();
        *st.key.borrow_mut() = k;
    }
    vec![st.cache.draw(renderer, bounds.size(), f)]
}

fn stroke(c: Color, w: f32) -> Stroke<'static> {
    Stroke::default().with_color(c).with_width(w).with_line_cap(LineCap::Round).with_line_join(LineJoin::Round)
}

fn ellipse(cx: f32, cy: f32, rx: f32, ry: f32) -> Path {
    Path::new(|b| {
        for i in 0..=48 {
            let t = std::f32::consts::TAU * i as f32 / 48.0;
            let p = Point::new(cx + rx * t.cos(), cy + ry * t.sin());
            if i == 0 {
                b.move_to(p)
            } else {
                b.line_to(p)
            }
        }
    })
}

pub fn canvas<'a, P: canvas::Program<M> + 'a, M: 'a>(p: P, w: Length, h: Length) -> Element<'a, M> {
    canvas::Canvas::new(p).width(w).height(h).into()
}

// ------------------------------------------------------------------ the dotted grid
pub struct Dots {
    pub step: f32,
    pub color: Color,
}
impl<M> canvas::Program<M> for Dots {
    type State = Memo;
    fn draw(&self, st: &Memo, r: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        memo(st, r, b, format!("{} {:?}", self.step, self.color), |f| {
            let step = self.step.max(6.0);
            let mut y = step / 2.0;
            while y < b.height {
                let mut x = step / 2.0;
                while x < b.width {
                    f.fill_rectangle(Point::new(x, y), Size::new(1.2, 1.2), self.color);
                    x += step;
                }
                y += step;
            }
        })
    }
}

// ------------------------------------------------------------------ a panel's corner marks (GPanel's)
pub struct Corners {
    pub color: Color,
    pub len: f32,
    pub w: f32,
}
impl<M> canvas::Program<M> for Corners {
    type State = Memo;
    fn draw(&self, st: &Memo, r: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        memo(st, r, b, format!("{:?} {} {}", self.color, self.len, self.w), |f| {
            let (l, w) = (self.len, self.w);
            f.fill_rectangle(Point::new(0.0, 0.0), Size::new(l, w), self.color);
            f.fill_rectangle(Point::new(0.0, 0.0), Size::new(w, l), self.color);
            f.fill_rectangle(Point::new(b.width - l, b.height - w), Size::new(l, w), self.color);
            f.fill_rectangle(Point::new(b.width - w, b.height - l), Size::new(w, l), self.color);
        })
    }
}

// ------------------------------------------------------------------ the last hour (Fact's Spark)
pub struct Spark {
    pub pts: Vec<f32>,
    pub color: Color,
    pub w: f32,
}
impl<M> canvas::Program<M> for Spark {
    type State = Memo;
    fn draw(&self, st: &Memo, r: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        memo(st, r, b, format!("{:?} {:?} {}", self.pts, self.color, self.w), |f| {
            let p = &self.pts;
            if p.len() < 2 {
                return;
            }
            let mx = p.iter().cloned().fold(1.0f32, f32::max);
            let mn = p.iter().cloned().fold(f32::INFINITY, f32::min);
            let span = (mx - mn).max(mx * 0.1).max(0.001);
            let path = Path::new(|pb| {
                for (i, v) in p.iter().enumerate() {
                    let x = i as f32 * b.width / (p.len() - 1) as f32;
                    let y = b.height - 2.0 - (v - mn) / span * (b.height - 4.0);
                    if i == 0 {
                        pb.move_to(Point::new(x, y))
                    } else {
                        pb.line_to(Point::new(x, y))
                    }
                }
            });
            f.stroke(&path, stroke(self.color, self.w));
        })
    }
}

// ------------------------------------------------------------------ the countdown's ring
pub struct Ring {
    pub p: f32,
    pub track: Color,
    pub fill: Color,
    pub w: f32,
}
impl<M> canvas::Program<M> for Ring {
    type State = Memo;
    fn draw(&self, st: &Memo, r: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        memo(st, r, b, format!("{} {:?} {:?} {}", self.p, self.track, self.fill, self.w), |f| {
            let c = Point::new(b.width / 2.0, b.height / 2.0);
            let rad = b.width.min(b.height) / 2.0 - self.w;
            f.stroke(&Path::circle(c, rad), stroke(self.track, self.w));
            let p = self.p.clamp(0.0, 1.0);
            if p > 0.0 {
                let arc = Path::new(|pb| {
                    let n = (64.0 * p).ceil().max(2.0) as usize;
                    for i in 0..=n {
                        let t = -std::f32::consts::FRAC_PI_2 + std::f32::consts::TAU * p * i as f32 / n as f32;
                        let q = Point::new(c.x + rad * t.cos(), c.y + rad * t.sin());
                        if i == 0 {
                            pb.move_to(q)
                        } else {
                            pb.line_to(q)
                        }
                    }
                });
                f.stroke(&arc, stroke(self.fill, self.w));
            }
        })
    }
}

// ------------------------------------------------------------------ an AI's core (ServerCore.qml)
/// The Command Center's colours (live.mjs CORES), exactly as ServerCore.qml has them.
pub fn core_color(kind: &str) -> Color {
    match kind {
        "gpu" => Color::from_rgb8(232, 214, 110),
        "comfy" => Color::from_rgb8(255, 95, 210),
        "voice" => Color::from_rgb8(255, 154, 168),
        "listen" => Color::from_rgb8(156, 245, 214),
        "hearth" => Color::from_rgb8(255, 138, 76),
        "ollama" => Color::from_rgb8(242, 239, 232),
        _ => Color::from_rgb8(201, 163, 114),
    }
}

pub struct Core {
    pub kind: String,
    pub working: bool,
    pub off: bool,
    /// The dim screen's faint copy (opacity 0.32 there).
    pub fade: f32,
}
impl<M> canvas::Program<M> for Core {
    type State = Memo;
    fn draw(&self, st: &Memo, r: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        memo(st, r, b, format!("{} {} {} {}", self.kind, self.working, self.off, self.fade), |f| {
            let s = b.width.min(b.height) / 1.5; // room around it for the rings it sends out
            let c = Point::new(b.width / 2.0, b.height / 2.0);
            let o = if self.off { 0.38 } else { 1.0 } * self.fade;
            let base = core_color(&self.kind);
            let col = |k: f32| a(base, k * o);
            let line = |f: &mut Frame, p: &Path, k: f32, w: f32| f.stroke(p, stroke(col(k), w));
            if self.working {
                line(f, &Path::circle(c, s * 0.5 * 1.18), 0.55, 2.0);
                line(f, &Path::circle(c, s * 0.5 * 1.42), 0.28, 1.0);
            }
            match self.kind.as_str() {
                "gpu" => {
                    line(f, &Path::circle(c, s * 0.46), 0.85, 1.0);
                    line(f, &ellipse(c.x, c.y, s * 0.46, s * 0.24), 0.5, 1.0);
                    line(f, &ellipse(c.x, c.y, s * 0.46, s * 0.08), 0.38, 1.0);
                    line(f, &ellipse(c.x, c.y, s * 0.23, s * 0.46), 0.7, 1.0);
                    line(f, &ellipse(c.x, c.y, s * 0.36, s * 0.46), 0.45, 1.0);
                }
                "comfy" => {
                    let cell = s * 0.14;
                    let gap = s * 0.04;
                    let w = cell * 4.0 + gap * 3.0;
                    let ks = [0.85, 0.38, 0.7, 0.5];
                    for i in 0..16 {
                        let (x, y) = ((i % 4) as f32, (i / 4) as f32);
                        f.fill_rectangle(Point::new(c.x - w / 2.0 + x * (cell + gap), c.y - w / 2.0 + y * (cell + gap)), Size::new(cell, cell), col(ks[i % 4]));
                    }
                }
                "voice" => {
                    line(f, &Path::circle(c, s * 0.44), 0.38, 1.0);
                    let hs = [0.5, 0.9, 0.7, 1.0, 0.6, 0.8];
                    let bw = s * 0.05;
                    let gap = s * 0.04;
                    let total = bw * 6.0 + gap * 5.0;
                    for (i, h) in hs.iter().enumerate() {
                        let hh = s * 0.52 * h;
                        f.fill_rectangle(Point::new(c.x - total / 2.0 + i as f32 * (bw + gap), c.y - hh / 2.0), Size::new(bw, hh), col(1.0));
                    }
                }
                "listen" => {
                    for (i, sc) in [0.45f32, 0.75, 1.05].iter().enumerate() {
                        line(f, &Path::circle(c, s * 0.5 * sc), [0.85, 0.55, 0.25][i], 2.0);
                    }
                }
                "hearth" => {
                    for i in 0..3 {
                        let fi = i as f32;
                        let w = s * (0.56 - fi * 0.1);
                        f.fill(&ellipse(b.width / 2.0 - s / 2.0 + s * (0.12 + fi * 0.06) + w / 2.0, b.height / 2.0 - s / 2.0 + s * (0.26 + fi * 0.02) + w * 0.4, w / 2.0, w * 0.4), col([0.32, 0.22, 0.4][i]));
                        let w2 = s * (0.5 - fi * 0.1);
                        f.fill(&ellipse(b.width / 2.0 - s / 2.0 + s * (0.38 + fi * 0.05) + w2 / 2.0, b.height / 2.0 - s / 2.0 + s * (0.38 + fi * 0.03) + w2 * 0.4, w2 / 2.0, w2 * 0.4), col([0.18, 0.14, 0.24][i]));
                    }
                    line(f, &Path::circle(c, s * 0.4), 0.6, 1.0);
                }
                _ => {
                    for (i, sc) in [0.92f32, 0.68, 0.44].iter().enumerate() {
                        line(f, &Path::circle(c, s * sc / 2.0), [0.5, 0.7, 0.85][i], 1.0);
                    }
                }
            }
            // The iris in the middle.
            f.fill(&Path::circle(c, s * 0.216), col(0.12));
            f.fill(&Path::circle(c, s * 0.12), a(Color::from_rgb8(5, 3, 2), o.max(0.6)));
            line(f, &Path::circle(c, s * 0.12), 1.0, 2.0);
        })
    }
}

// ------------------------------------------------------------------ the web page's QR code (qrencode's SVG)
#[derive(Debug, Clone, PartialEq)]
pub struct Qr {
    pub w: f32,
    pub h: f32,
    pub rects: Vec<(f32, f32, f32, f32)>,
}

/// The modules of the SVG qrencode made (hearth-server-mode web_qr): its viewBox and its rects (the first, the
/// background, left out). None when it isn't one.
pub fn parse_qr(svg: &str) -> Option<Qr> {
    let num = |tag: &str, k: &str| -> Option<f32> {
        let i = tag.find(&format!(" {k}=\""))? + k.len() + 3;
        tag[i..].split('"').next()?.trim().parse().ok()
    };
    let vb = svg.find("viewBox=\"").map(|i| &svg[i + 9..]).and_then(|s| s.split('"').next())?;
    let v: Vec<f32> = vb.split_whitespace().filter_map(|x| x.parse().ok()).collect();
    if v.len() != 4 || v[2] <= 0.0 || v[3] <= 0.0 {
        return None;
    }
    // qrencode puts the modules in a group moved by the margin (transform="translate(1,1)").
    let (tx, ty) = svg
        .find("translate(")
        .and_then(|i| svg[i + 10..].split(')').next())
        .map(|s| {
            let n: Vec<f32> = s.split([',', ' ']).filter_map(|x| x.trim().parse().ok()).collect();
            (n.first().copied().unwrap_or(0.0), n.get(1).copied().unwrap_or(0.0))
        })
        .unwrap_or((0.0, 0.0));
    let mut rects = Vec::new();
    for part in svg.split("<rect").skip(1) {
        let tag = part.split('>').next().unwrap_or("");
        let (x, y) = (num(tag, "x").unwrap_or(0.0), num(tag, "y").unwrap_or(0.0));
        let (w, h) = (num(tag, "width")?, num(tag, "height")?);
        if w >= v[2] * 0.9 && h >= v[3] * 0.9 {
            continue; // the background
        }
        rects.push((x + tx - v[0], y + ty - v[1], w, h));
    }
    if rects.is_empty() {
        return None;
    }
    Some(Qr { w: v[2], h: v[3], rects })
}

pub struct QrCode {
    pub qr: Qr,
    pub fg: Color,
    pub bg: Color,
}
impl<M> canvas::Program<M> for QrCode {
    type State = Memo;
    fn draw(&self, st: &Memo, r: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        memo(st, r, b, format!("{} {} {}", self.qr.w, self.qr.rects.len(), self.qr.rects.first().map(|x| x.0 + x.1).unwrap_or(0.0)), |f| {
            let k = (b.width / self.qr.w).min(b.height / self.qr.h);
            f.fill_rectangle(Point::ORIGIN, Size::new(self.qr.w * k, self.qr.h * k), self.bg);
            for (x, y, w, h) in &self.qr.rects {
                f.fill_rectangle(Point::new(x * k, y * k), Size::new(w * k, h * k), self.fg);
            }
        })
    }
}

// ------------------------------------------------------------------ line icons (the names Mantel's Icon has)
pub struct Icon {
    pub name: String,
    pub color: Color,
}
impl<M> canvas::Program<M> for Icon {
    type State = Memo;
    fn draw(&self, st: &Memo, r: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        memo(st, r, b, format!("{} {:?}", self.name, self.color), |f| {
            let k = b.width.min(b.height) / 24.0;
            let (ls, cs) = icon_lines(&self.name);
            let st = stroke(self.color, (1.8 * k).max(1.0));
            for l in ls {
                if l.len() < 2 {
                    continue;
                }
                let p = Path::new(|pb| {
                    pb.move_to(Point::new(l[0].0 * k, l[0].1 * k));
                    for q in &l[1..] {
                        pb.line_to(Point::new(q.0 * k, q.1 * k));
                    }
                });
                f.stroke(&p, st.clone());
            }
            for (x, y, rr) in cs {
                let p = Path::circle(Point::new(x * k, y * k), rr * k);
                if rr < 2.0 {
                    f.fill(&p, self.color);
                } else {
                    f.stroke(&p, st.clone());
                }
            }
        })
    }
}

pub fn icon<'a, M: 'a>(name: &str, color: Color, size: f32) -> Element<'a, M> {
    canvas(Icon { name: name.to_string(), color }, Length::Fixed(size), Length::Fixed(size))
}

type Lines = (Vec<Vec<(f32, f32)>>, Vec<(f32, f32, f32)>);

fn arc(cx: f32, cy: f32, r: f32, from: f32, to: f32) -> Vec<(f32, f32)> {
    (0..=20).map(|i| {
        let t = (from + (to - from) * i as f32 / 20.0).to_radians();
        (cx + r * t.cos(), cy + r * t.sin())
    }).collect()
}

fn icon_lines(name: &str) -> Lines {
    let v = |p: &[(f32, f32)]| p.to_vec();
    match name {
        "server" => (vec![v(&[(4.0, 4.0), (20.0, 4.0), (20.0, 10.0), (4.0, 10.0), (4.0, 4.0)]), v(&[(4.0, 14.0), (20.0, 14.0), (20.0, 20.0), (4.0, 20.0), (4.0, 14.0)])], vec![(7.5, 7.0, 0.9), (7.5, 17.0, 0.9)]),
        "pad" => (vec![v(&[(4.0, 8.0), (20.0, 8.0), (22.0, 17.0), (18.0, 18.0), (15.0, 14.0), (9.0, 14.0), (6.0, 18.0), (2.0, 17.0), (4.0, 8.0)]), v(&[(8.0, 9.5), (8.0, 12.5)]), v(&[(6.5, 11.0), (9.5, 11.0)])], vec![(16.0, 10.5, 0.9), (17.5, 12.0, 0.9)]),
        "gear" => (vec![v(&[(12.0, 2.0), (12.0, 5.0)]), v(&[(12.0, 19.0), (12.0, 22.0)]), v(&[(2.0, 12.0), (5.0, 12.0)]), v(&[(19.0, 12.0), (22.0, 12.0)]),
                        v(&[(5.0, 5.0), (7.0, 7.0)]), v(&[(17.0, 17.0), (19.0, 19.0)]), v(&[(19.0, 5.0), (17.0, 7.0)]), v(&[(5.0, 19.0), (7.0, 17.0)])], vec![(12.0, 12.0, 5.0)]),
        "moon" | "sleep" => {
            let mut a1 = arc(12.0, 12.0, 9.0, 63.0, 342.0);
            a1.extend(arc(16.0, 9.0, 7.0, 306.0, 99.0).into_iter().rev().collect::<Vec<_>>().into_iter().rev());
            (vec![a1], if name == "sleep" { vec![(18.0, 5.0, 0.9)] } else { vec![] })
        }
        "restart" => (vec![arc(12.0, 12.0, 8.0, 36.0, 324.0), v(&[(18.0, 2.5), (19.5, 7.0), (15.0, 8.0)])], vec![]),
        "power" => (vec![arc(12.0, 13.0, 8.0, -60.0, 240.0), v(&[(12.0, 3.0), (12.0, 11.0)])], vec![]),
        "desktops" => (vec![v(&[(3.0, 4.0), (21.0, 4.0), (21.0, 16.0), (3.0, 16.0), (3.0, 4.0)]), v(&[(12.0, 16.0), (12.0, 20.0)]), v(&[(8.0, 20.0), (16.0, 20.0)])], vec![]),
        "chat" => (vec![v(&[(4.0, 5.0), (20.0, 5.0), (20.0, 16.0), (11.0, 16.0), (6.0, 20.0), (7.0, 16.0), (4.0, 16.0), (4.0, 5.0)])], vec![(8.5, 10.5, 0.9), (12.0, 10.5, 0.9), (15.5, 10.5, 0.9)]),
        "image" => (vec![v(&[(3.0, 5.0), (21.0, 5.0), (21.0, 19.0), (3.0, 19.0), (3.0, 5.0)]), v(&[(3.0, 17.0), (9.0, 11.0), (13.0, 15.0), (16.0, 12.0), (21.0, 17.0)])], vec![(15.5, 8.5, 1.4)]),
        "film" => (vec![v(&[(3.0, 4.0), (21.0, 4.0), (21.0, 20.0), (3.0, 20.0), (3.0, 4.0)]), v(&[(7.0, 4.0), (7.0, 20.0)]), v(&[(17.0, 4.0), (17.0, 20.0)]), v(&[(3.0, 12.0), (21.0, 12.0)])], vec![]),
        "drive" => (vec![v(&[(3.0, 6.0), (21.0, 6.0), (21.0, 18.0), (3.0, 18.0), (3.0, 6.0)]), v(&[(3.0, 13.0), (21.0, 13.0)])], vec![(17.5, 15.5, 0.9)]),
        "cloud" => {
            let mut c = arc(8.0, 15.0, 4.0, 90.0, 270.0);
            c.extend(arc(12.5, 10.5, 5.0, 200.0, 340.0));
            c.extend(arc(17.0, 14.5, 4.5, 270.0, 450.0));
            c.push((8.0, 19.0));
            (vec![c], vec![])
        }
        "cpu" => (vec![v(&[(6.0, 6.0), (18.0, 6.0), (18.0, 18.0), (6.0, 18.0), (6.0, 6.0)]), v(&[(9.0, 9.0), (15.0, 9.0), (15.0, 15.0), (9.0, 15.0), (9.0, 9.0)]),
                       v(&[(9.0, 2.5), (9.0, 6.0)]), v(&[(15.0, 2.5), (15.0, 6.0)]), v(&[(9.0, 18.0), (9.0, 21.5)]), v(&[(15.0, 18.0), (15.0, 21.5)]),
                       v(&[(2.5, 9.0), (6.0, 9.0)]), v(&[(2.5, 15.0), (6.0, 15.0)]), v(&[(18.0, 9.0), (21.5, 9.0)]), v(&[(18.0, 15.0), (21.5, 15.0)])], vec![]),
        "tailscale" => (vec![], vec![(5.0, 5.0, 1.6), (12.0, 5.0, 1.6), (19.0, 5.0, 1.6), (5.0, 12.0, 1.6), (12.0, 12.0, 1.6), (19.0, 12.0, 1.6), (12.0, 19.0, 1.6)]),
        "grid" => (vec![v(&[(4.0, 4.0), (10.0, 4.0), (10.0, 10.0), (4.0, 10.0), (4.0, 4.0)]), v(&[(14.0, 4.0), (20.0, 4.0), (20.0, 10.0), (14.0, 10.0), (14.0, 4.0)]),
                        v(&[(4.0, 14.0), (10.0, 14.0), (10.0, 20.0), (4.0, 20.0), (4.0, 14.0)]), v(&[(14.0, 14.0), (20.0, 14.0), (20.0, 20.0), (14.0, 20.0), (14.0, 14.0)])], vec![]),
        "wave" | "vol" => (vec![v(&[(3.0, 12.0), (5.0, 12.0)]), v(&[(7.0, 8.0), (7.0, 16.0)]), v(&[(10.0, 5.0), (10.0, 19.0)]), v(&[(13.0, 9.0), (13.0, 15.0)]), v(&[(16.0, 6.0), (16.0, 18.0)]), v(&[(19.0, 10.0), (19.0, 14.0)])], vec![]),
        "mic" => (vec![v(&[(9.0, 4.0), (15.0, 4.0), (15.0, 13.0), (9.0, 13.0), (9.0, 4.0)]), arc(12.0, 11.0, 6.5, 0.0, 180.0), v(&[(12.0, 17.5), (12.0, 21.0)])], vec![]),
        "folder" => (vec![v(&[(3.0, 6.0), (9.0, 6.0), (11.0, 8.0), (21.0, 8.0), (21.0, 19.0), (3.0, 19.0), (3.0, 6.0)])], vec![]),
        "sparkle" => (vec![v(&[(12.0, 3.0), (13.8, 10.2), (21.0, 12.0), (13.8, 13.8), (12.0, 21.0), (10.2, 13.8), (3.0, 12.0), (10.2, 10.2), (12.0, 3.0)])], vec![(19.0, 5.0, 1.2)]),
        "check" => (vec![v(&[(5.0, 12.0), (10.0, 17.0), (19.0, 7.0)])], vec![]),
        "globe" => (vec![v(&[(3.0, 12.0), (21.0, 12.0)]), arc(12.0, 12.0, 9.0, 0.0, 360.0), {
            let mut m = arc(12.0, 12.0, 9.0, -90.0, 90.0);
            for p in m.iter_mut() { p.0 = 12.0 + (p.0 - 12.0) * 0.45; }
            m
        }, {
            let mut m = arc(12.0, 12.0, 9.0, 90.0, 270.0);
            for p in m.iter_mut() { p.0 = 12.0 + (p.0 - 12.0) * 0.45; }
            m
        }], vec![]),
        "shield" => (vec![v(&[(12.0, 3.0), (20.0, 6.0), (19.0, 13.0), (12.0, 21.0), (5.0, 13.0), (4.0, 6.0), (12.0, 3.0)])], vec![]),
        "clock" | "rules" => (vec![v(&[(12.0, 7.0), (12.0, 12.0), (16.0, 14.0)])], vec![(12.0, 12.0, 9.0)]),
        "download" | "cache" => (vec![v(&[(12.0, 3.0), (12.0, 15.0)]), v(&[(7.0, 10.0), (12.0, 15.0), (17.0, 10.0)]), v(&[(4.0, 19.0), (20.0, 19.0)])], vec![]),
        "heart" => (vec![v(&[(12.0, 20.0), (4.0, 12.0), (4.0, 7.5), (7.5, 4.5), (12.0, 7.5), (16.5, 4.5), (20.0, 7.5), (20.0, 12.0), (12.0, 20.0)])], vec![]),
        _ => (vec![], vec![(12.0, 12.0, 1.8)]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qrencode_svg_is_read() {
        let svg = r##"<?xml version="1.0"?><svg width="1.0cm" height="1.0cm" viewBox="0 0 23 23"><g id="QRcode"><rect x="0" y="0" width="23" height="23" fill="#0C0702"/><g id="Pattern" transform="translate(1,1)"><rect x="0" y="0" width="7" height="1" fill="#FFE9C7"/><rect x="8" y="0" width="1" height="1" fill="#FFE9C7"/></g></g></svg>"##;
        let q = parse_qr(svg).expect("a QR code");
        assert_eq!(q.w, 23.0);
        assert_eq!(q.rects.len(), 2);
        assert!(parse_qr("<svg></svg>").is_none());
    }
}
