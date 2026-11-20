//! The graph (pages 1, 3–3h): the vault's notes and links in ten modes. Drawn on iced's canvas, which the graphics
//! card draws (wgpu) where it starts and the CPU (tiny-skia) otherwise; with the CPU fewer stars and particles are
//! drawn and it animates at 15 frames a second instead of 30 (the light fallback). The modes animate only while their
//! page is in front, the window is focused and something was done there in the last 30 s; with Reduce motion they're
//! drawn still. At rest nothing is drawn again (0 % CPU). Every count is the vault's own: a project with no notes says
//! "no notes yet".

use crate::look::{self, accent2, bx, click, f, pad, project_color, tone, z, E};
use crate::{App, Msg, Page, FLY_ID};
use hearth_notes::graph::{hash, unit};
use hearth_notes::vault::{EdgeKind, KINDS, PROJECTS};
use hearth_notes::{day, md};
use hearth_ui::ui::{self, gap, t, t1, tb, ts};
use hearth_ui::Pal;
use iced::widget::canvas::{self, Cache, Frame, Geometry, LineCap, Path, Stroke, Text};
use iced::widget::{column, container, image, mouse_area, pin, responsive, row, scrollable, stack, text, text_input, Column, Row, Space};
use iced::{mouse, Color, Length, Pixels, Point, Rectangle, Renderer, Size, Task, Theme};
use serde_json::{json, Value};
use std::f32::consts::{PI, TAU};

/// Every mode: its id, name in the picker, its line.
pub const MODES: [(&str, &str, &str); 10] = [
    ("galaxy", "Galaxy", "3D starfield: notes are stars, links light trails, a nebula per project."),
    ("solar", "Solar system", "Projects are suns, kinds are rings, notes are planets with moons."),
    ("blackhole", "Black hole", "Home at the centre; every project a cloud pulled in by its links."),
    ("ember", "Ember", "Notes rise as embers from the fire; links are heat shimmer."),
    ("railway", "Railway map", "Kinds are lines, notes are stations, the daily logs the Main Line."),
    ("nightsky", "Night sky", "Constellations by tag over a dark landscape."),
    ("neural", "Neural", "Like Ithil's brain: links fire when you open a note."),
    ("city", "City at night", "Projects are districts, notes lit buildings: busier = brighter."),
    ("river", "Timeline river", "Notes float down a river by date, newest at the mouth."),
    ("classic", "Classic", "The plain flat graph, quickest to read."),
];

pub fn name_of(id: &str) -> &'static str {
    MODES.iter().find(|m| m.0 == id).map(|m| m.1).unwrap_or("Classic")
}

/// The mode bar's short names.
const BAR: [(&str, &str); 6] = [("classic", "Classic"), ("galaxy", "Galaxy"), ("solar", "Solar system"), ("blackhole", "Black hole"), ("ember", "Ember"), ("railway", "Railway")];

/// The canvas's last size (for flying the camera to a note from the window's update).
static LAST: std::sync::Mutex<(f32, f32)> = std::sync::Mutex::new((900.0, 700.0));

/// A node where a mode puts it.
#[derive(Debug, Clone, Copy)]
pub struct P {
    pub i: usize,
    pub x: f32,
    pub y: f32,
    pub r: f32,
    pub c: Color,
    pub a: f32,
    pub label: bool,
}

fn bg_of(mode: &str) -> Color {
    match mode {
        "classic" => Color::from_rgb8(0x10, 0x12, 0x16),
        "railway" => Color::from_rgb8(0x10, 0x12, 0x16),
        "ember" => Color::from_rgb8(0x0d, 0x0a, 0x09),
        "nightsky" => Color::from_rgb8(0x07, 0x0b, 0x18),
        "neural" => Color::from_rgb8(0x0b, 0x08, 0x14),
        "city" => Color::from_rgb8(0x08, 0x0a, 0x12),
        "river" => Color::from_rgb8(0x0a, 0x0d, 0x14),
        _ => Color::from_rgb8(0x06, 0x07, 0x0b),
    }
}

fn kind_color(k: &str) -> Color {
    match k {
        "mockups" => Color::from_rgb8(0xff, 0x7a, 0x2f),
        "assets" => Color::from_rgb8(0xf5, 0xc0, 0x3e),
        "videos" => Color::from_rgb8(0xf2, 0x55, 0x55),
        "notes" => Color::from_rgb8(0x45, 0xc8, 0xe8),
        "research" => Color::from_rgb8(0x8b, 0xd3, 0x4a),
        _ => Color::from_rgb8(0xb0, 0xb4, 0xbe),
    }
}

/// A saved look's node colour (per-project looks).
fn look_tint(look: &str, base: Color) -> Color {
    match look {
        "Ember" => Pal::mix(base, Color::from_rgb8(0xff, 0x8a, 0x3d), 0.6),
        "Neon" => Color::from_rgb8(0xff, 0x5c, 0xd6),
        "Galaxy night" => Color::from_rgb8(0xc8, 0xd8, 0xff),
        "Minimal" => Color::from_rgb8(0x9a, 0x9f, 0xab),
        "Solar system" => Color::from_rgb8(0xff, 0xc8, 0x5a),
        _ => base,
    }
}

/// Everything a mode needs to place and draw the notes (taken from the window's state).
pub struct Sky<'a> {
    pub a: &'a App,
    pub mode: &'a str,
    /// A preview tile or the main window's pane: no hover cards, labels only for the busiest.
    pub small: bool,
    /// Only the selected note and its links (Local graph, Show only its links).
    pub local: bool,
    /// A preview tile in Graph modes: no labels.
    pub tile: bool,
}

#[derive(Default)]
pub struct SkyState {
    bg: Cache,
    key: std::cell::Cell<Option<(u64, u32, u32)>>,
    press: Option<Point>,
    last: Option<Point>,
    moved: bool,
    hover: Option<usize>,
}

impl<'a> Sky<'a> {
    fn shown(&self) -> Vec<usize> {
        let a = self.a;
        let n = a.graph.nodes.len();
        if self.local {
            if let Some(s) = a.g.sel.or_else(|| a.open_index()) {
                let mut v = a.graph.neighbours(s);
                v.push(s);
                return v;
            }
        }
        let hide_att = a.gb("hideAttachments", true);
        (0..n).filter(|&i| a.node_shown(i) && !(hide_att && !a.vault.notes[i].rel.ends_with(".md"))).collect()
    }

    fn size_of(&self, i: usize) -> f32 {
        let a = self.a;
        let nd = &a.graph.nodes[i];
        let by = a.gs("sizeBy", "links");
        let v = match by.as_str() {
            "length" => (nd.words as f32 / 120.0).sqrt(),
            "opened" => (*a.opened.get(&a.vault.notes[i].rel).unwrap_or(&0) as f32).sqrt(),
            _ => (nd.links as f32).sqrt(),
        };
        (2.6 + v * 1.6).min(12.0)
    }

    fn color_of(&self, i: usize) -> Color {
        let a = self.a;
        let p = &a.p;
        let nd = &a.graph.nodes[i];
        let by = a.gs("colourBy", "project");
        let base = match by.as_str() {
            "kind" => kind_color(&nd.kind),
            "status" => {
                let st = md::properties(&a.vault.notes[i].text).into_iter().find(|x| x.0 == "Status").map(|x| x.1.to_lowercase()).unwrap_or_default();
                if st.starts_with("approved") { look::GOOD } else if st.starts_with("draft") { look::WARN } else if st.starts_with("posted") || st.starts_with("finished") { look::BLUE } else { Color::from_rgb8(0x9a, 0x9f, 0xab) }
            }
            _ => project_color(p, if nd.project.is_empty() { "?" } else { &nd.project }),
        };
        match a.prefs.project_looks.get(&nd.project).and_then(Value::as_str) {
            Some(l) => look_tint(l, base),
            None => base,
        }
    }

    /// Where the mode puts each shown note in a canvas of this size.
    pub fn place(&self, w: f32, h: f32) -> Vec<P> {
        let a = self.a;
        let g = &a.graph;
        let cam = &a.g.cam;
        let m = w.min(h);
        let (cx, cy) = (w / 2.0 + cam.x * m, h / 2.0 + cam.y * m);
        let zoom = cam.zoom;
        let shown = self.shown();
        let tt = a.g.t;
        let sel = a.g.sel;
        let mut out = Vec::with_capacity(shown.len());
        let busy = |i: usize| g.nodes[i].links >= 3 || Some(i) == sel || a.g.hover == Some(i);
        match self.mode {
            "galaxy" | "neural" => {
                let yaw = cam.yaw + tt * 0.03;
                let (sy, cy_) = (yaw.sin(), yaw.cos());
                let (sp, cp) = (cam.pitch.sin(), cam.pitch.cos());
                for &i in &shown {
                    let nd = &g.nodes[i];
                    let (x, y, zz) = if self.mode == "neural" { (nd.x * 1.15, nd.y * 0.7, nd.z * 0.5) } else { (nd.x, nd.y * 0.75, nd.z * 0.9) };
                    let x2 = x * cy_ - zz * sy;
                    let z2 = x * sy + zz * cy_;
                    let y2 = y * cp - z2 * sp * 0.4;
                    let z3 = y * sp * 0.4 + z2 * cp;
                    let k = 2.2 / (2.2 + z3);
                    let s = m * 0.46 * zoom * k;
                    let r = self.size_of(i) * k * if self.mode == "neural" { 0.7 } else { 0.9 };
                    let c = if self.mode == "neural" { Color::from_rgb8(0xd6, 0xc4, 0xff) } else if nd.project == "log" { Color::from_rgb8(0xff, 0xf4, 0xe0) } else { Pal::mix(self.color_of(i), Color::WHITE, 0.25) };
                    out.push(P { i, x: cx + x2 * s * (w / m).min(1.6), y: cy + y2 * s, r, c, a: (0.55 + 0.45 * k).min(1.0), label: busy(i) });
                }
            }
            "solar" => {
                let slots = [(0.42, 0.55), (0.10, 0.17), (0.34, 0.10), (0.82, 0.60), (0.92, 0.72), (0.80, 0.86), (0.62, 0.93), (0.12, 0.84)];
                let order = solar_order(a);
                for &i in &shown {
                    let nd = &g.nodes[i];
                    if nd.project == "log" {
                        // Daily logs pass by as comets across the top.
                        let logs: Vec<usize> = shown.iter().copied().filter(|j| g.nodes[*j].project == "log").collect();
                        let k = logs.iter().position(|j| *j == i).unwrap_or(0) as f32 / logs.len().max(1) as f32;
                        let x = w * (0.28 + 0.55 * k) + (tt * 3.0 + k * 40.0).sin() * 4.0;
                        let y = h * (0.30 - 0.12 * (k * PI).sin()) + k * h * 0.06;
                        out.push(P { i, x, y, r: 2.2, c: Color::WHITE, a: 0.95, label: true });
                        continue;
                    }
                    let Some(slot) = order.iter().position(|p| *p == nd.project) else { continue };
                    let (sx, sy) = slots[slot.min(7)];
                    let (ox, oy) = (w * sx + cam.x * m, h * sy + cam.y * m);
                    let (rx, ry, ang) = if slot == 0 {
                        let k = KINDS.iter().position(|x| *x == nd.kind).map(|x| x as f32).unwrap_or(0.0);
                        let rx = m * zoom * (0.20 + 0.105 * (4.0 - k));
                        (rx, rx * 0.42, unit(&a.vault.notes[i].rel, 5) * TAU + cam.yaw + tt * 0.06 / (1.0 + k * 0.4))
                    } else {
                        (m * 0.07 * zoom, m * 0.03 * zoom, unit(&a.vault.notes[i].rel, 5) * TAU + tt * 0.1)
                    };
                    let opened = *a.opened.get(&a.vault.notes[i].rel).unwrap_or(&0) as f32;
                    let by = a.gs("sizeBy", "opened");
                    let v = match by.as_str() {
                        "links" => (nd.links as f32).sqrt(),
                        "words" => (nd.words as f32 / 120.0).sqrt(),
                        _ => (opened + nd.links as f32 * 0.5).sqrt(),
                    };
                    out.push(P { i, x: ox + ang.cos() * rx, y: oy + ang.sin() * ry, r: (3.0 + v * 1.8).min(12.0) * zoom.sqrt(), c: Color::from_rgb8(0xff, 0x7a, 0x35), a: 1.0, label: busy(i) || nd.links >= 2 });
                }
            }
            "blackhole" => {
                let rr = m * 0.5 * zoom;
                for &i in &shown {
                    let nd = &g.nodes[i];
                    let rel = &a.vault.notes[i].rel;
                    if nd.project == "home" {
                        continue;
                    }
                    if nd.links == 0 && nd.project != "log" {
                        let ang = unit(rel, 7) * TAU;
                        out.push(P { i, x: cx + ang.cos() * rr * 1.45, y: cy + ang.sin() * rr * 0.92, r: 1.8, c: Color::from_rgb8(0xc8, 0xc8, 0xd0), a: 0.7, label: false });
                        continue;
                    }
                    if nd.project == "log" {
                        let ang = unit(rel, 8) * TAU + tt * 0.2;
                        out.push(P { i, x: cx + ang.cos() * rr * 0.36, y: cy + ang.sin() * rr * 0.07, r: 1.6, c: Color::from_rgb8(0xff, 0xe0, 0xb0), a: 0.9, label: false });
                        continue;
                    }
                    let (ccx, ccy, cr) = cloud_of(a, &nd.project, cx, cy, rr);
                    let (u, v) = (unit(rel, 11), unit(rel, 12));
                    let ang = v * TAU;
                    let d = u.sqrt();
                    out.push(P { i, x: ccx + ang.cos() * cr * 1.3 * d, y: ccy + ang.sin() * cr * 0.95 * d, r: 2.2, c: Pal::mix(project_color(&a.p, &nd.project), Color::WHITE, 0.55), a: 0.95, label: false });
                }
            }
            "ember" => {
                let mut by_age: Vec<usize> = shown.clone();
                let heat_opened = a.gs("heat", "changed") == "opened";
                by_age.sort_by_key(|&i| if heat_opened { -(*a.opened.get(&a.vault.notes[i].rel).unwrap_or(&0) as i64) } else { -g.nodes[i].mtime });
                let n = by_age.len().max(2) as f32;
                let fire_y = h * 0.80 + cam.y * m;
                for (rank, &i) in by_age.iter().enumerate() {
                    let fr = rank as f32 / (n - 1.0);
                    let u = unit(&a.vault.notes[i].rel, 3) - 0.5;
                    let x = cx + u * (w * 0.12 + fr * w * 0.85) * zoom + (tt * 0.7 + u * 9.0).sin() * 3.0 + cam.yaw * 40.0;
                    let y = fire_y - h * 0.06 - fr * h * 0.68 * zoom - (tt * 0.9 + u * 7.0).sin().abs() * 4.0;
                    let hot = 1.0 - fr;
                    let c = Pal::mix(Color::from_rgb8(0xe0, 0x5a, 0x1c), Color::from_rgb8(0xff, 0xf0, 0xc0), hot * hot);
                    out.push(P { i, x, y, r: 2.0 + hot * 3.2, c, a: 0.55 + hot * 0.45, label: busy(i) || hot > 0.75 });
                }
            }
            "railway" => {
                for st in railway(a, w, h) {
                    out.push(P { i: st.0, x: st.1, y: st.2, r: if g.nodes[st.0].project == "log" { 7.0 } else { 4.2 }, c: Color::WHITE, a: 1.0, label: true });
                }
            }
            "nightsky" => {
                let groups = tag_groups(a, &shown);
                let cols = (groups.len() as f32).sqrt().ceil().max(1.0) as usize;
                for (gi, (_, members)) in groups.iter().enumerate() {
                    let (gx, gy) = ((gi % cols) as f32 + 0.5, (gi / cols) as f32 + 0.5);
                    let rows_ = groups.len().div_ceil(cols).max(1) as f32;
                    let (ox, oy) = (w * 0.08 + gx / cols as f32 * w * 0.84 + cam.x * m, h * 0.06 + gy / rows_ * h * 0.62 + cam.y * m);
                    for (k, &i) in members.iter().enumerate() {
                        let ang = k as f32 * 2.4;
                        let d = (k as f32).sqrt() * m * 0.035 * zoom;
                        out.push(P { i, x: ox + ang.cos() * d, y: oy + ang.sin() * d * 0.7, r: 1.8 + (g.nodes[i].links as f32).sqrt() * 0.6, c: Color::from_rgb8(0xee, 0xf2, 0xff), a: 0.95, label: Some(i) == sel });
                    }
                }
            }
            "city" => {
                let order = solar_order(a);
                let total: usize = shown.iter().filter(|i| g.nodes[**i].project != "log").count().max(1);
                let mut x0 = w * 0.04 + cam.x * m;
                let ground = h * 0.86;
                for pj in order {
                    let mem: Vec<usize> = shown.iter().copied().filter(|i| g.nodes[*i].project == pj).collect();
                    if mem.is_empty() {
                        continue;
                    }
                    let span = w * 0.92 * mem.len() as f32 / total as f32;
                    let bw = (span / mem.len() as f32).min(24.0);
                    for (k, &i) in mem.iter().enumerate() {
                        let hgt = (18.0 + ((g.nodes[i].words as f32 + 1.0).ln() * 14.0)) * zoom;
                        out.push(P { i, x: x0 + k as f32 * bw + bw / 2.0, y: ground - hgt, r: bw * 0.42, c: project_color(&a.p, &pj), a: 1.0, label: Some(i) == sel });
                    }
                    x0 += span;
                }
            }
            "river" => {
                let mut days: Vec<day::Ymd> = shown.iter().map(|&i| g.nodes[i].log.unwrap_or_else(|| day::log_day(g.nodes[i].mtime, 0))).collect();
                days.sort();
                days.dedup();
                let nd_ = days.len().max(1) as f32;
                let mut seen: std::collections::BTreeMap<day::Ymd, usize> = Default::default();
                for &i in &shown {
                    let d = g.nodes[i].log.unwrap_or_else(|| day::log_day(g.nodes[i].mtime, 0));
                    let k = days.iter().position(|x| *x == d).unwrap_or(0) as f32;
                    let x = w * 0.06 + (k + 0.5) / nd_ * w * 0.88 + cam.x * m;
                    let yr = river_y(x, w, h) + 6.0;
                    let c = seen.entry(d).or_insert(0);
                    let n_day = shown.iter().filter(|&&j| g.nodes[j].log.unwrap_or_else(|| day::log_day(g.nodes[j].mtime, 0)) == d).count();
                    let cols = ((n_day as f32 * 2.0).sqrt().ceil() as usize).clamp(1, 14);
                    let (row_, col_) = (*c / cols, *c % cols);
                    *c += 1;
                    out.push(P { i, x: x - (cols as f32 - 1.0) * 4.0 + col_ as f32 * 8.0, y: yr - 10.0 - row_ as f32 * 8.0 + (tt + k).sin() * 1.5, r: 3.0, c: Color::from_rgb8(0xff, 0xa8, 0x50), a: 1.0, label: false });
                }
            }
            _ => {
                // Fit the notes into the free part of the canvas (the filter panel takes the right side of the
                // full-size page): the middle 90 % of them by place, so a few far ones don't push the rest aside.
                let spread = a.gf("linkDistance", 0.75).clamp(0.2, 1.0) * 0.4 + 0.7;
                let pct = |v: &mut Vec<f32>| -> (f32, f32) {
                    v.sort_by(|x, y| x.total_cmp(y));
                    if v.is_empty() {
                        return (-1.0, 1.0);
                    }
                    let lo = v[(v.len() as f32 * 0.05) as usize];
                    let hi = v[((v.len() as f32 * 0.95) as usize).min(v.len() - 1)];
                    if hi - lo < 0.05 { (lo - 0.5, hi + 0.5) } else { (lo, hi) }
                };
                let (x0, x1) = pct(&mut shown.iter().map(|&i| g.nodes[i].x).collect());
                let (y0, y1) = pct(&mut shown.iter().map(|&i| g.nodes[i].y).collect());
                let panel = if self.small || a.g.customise { 0.0 } else { z(400.0) + z(56.0) };
                let (left, right, top, bottom) = if self.small { (w * 0.08, w * 0.92, h * 0.10, h * 0.88) } else { (z(60.0), w - panel - z(40.0), z(120.0), h - z(90.0)) };
                let (mx, my) = ((left + right) / 2.0 + cam.x * m, (top + bottom) / 2.0 + cam.y * m);
                let (sx, sy) = ((right - left).max(40.0) / 2.0 * zoom * spread, (bottom - top).max(40.0) / 2.0 * zoom * spread);
                let (cxn, cyn) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
                let (hx, hy) = (((x1 - x0) / 2.0).max(0.05), ((y1 - y0) / 2.0).max(0.05));
                for &i in &shown {
                    let nd = &g.nodes[i];
                    let col = if nd.project == "log" { Color::from_rgb8(0xee, 0xec, 0xe8) } else { self.color_of(i) };
                    // The few beyond the middle 90 % are kept just inside the edges.
                    let x = (mx + (nd.x - cxn) / hx * sx * 0.9).clamp(left - z(20.0), right + z(20.0));
                    let y = (my + (nd.y - cyn) / hy * sy * 0.9).clamp(top - z(20.0), bottom + z(10.0));
                    out.push(P { i, x, y, r: self.size_of(i), c: col, a: 1.0, label: if self.small { nd.links >= 3 || Some(i) == sel || nd.project == "log" } else { busy(i) || nd.project == "log" } });
                }
            }
        }
        out
    }
}

/// The projects in the order their suns (and districts) take: the one with most notes first.
fn solar_order(a: &App) -> Vec<String> {
    let counts = a.vault.project_counts();
    let mut v: Vec<(String, usize)> = PROJECTS.iter().map(|p| (p.to_string(), *counts.get(*p).unwrap_or(&0))).collect();
    v.sort_by(|x, y| y.1.cmp(&x.1).then(PROJECTS.iter().position(|p| *p == x.0).cmp(&PROJECTS.iter().position(|p| *p == y.0))));
    v.into_iter().map(|x| x.0).collect()
}

/// A project's cloud in Black hole: its centre and radius (by how many notes it has).
fn cloud_of(a: &App, project: &str, cx: f32, cy: f32, rr: f32) -> (f32, f32, f32) {
    let angles = [("hearth", 185.0), ("tributary", 228.0), ("hearth-ai", 272.0), ("beacon", 330.0), ("other", 352.0), ("command-center", 25.0), ("quad-mech", 72.0), ("gravesleep", 118.0)];
    let deg = angles.iter().find(|x| x.0 == project).map(|x| x.1).unwrap_or(0.0f32) + a.g.cam.yaw.to_degrees() * 0.3;
    let n = *a.vault.project_counts().get(project).unwrap_or(&0) as f32;
    let cr = 0.08 + (n.sqrt() * 0.045).min(0.30);
    // A bigger cloud sits a little closer in, so all of it stays on the map.
    let dist = if project == "other" { 0.62 } else { 0.86 } - cr * 0.55;
    let ang = deg.to_radians();
    (cx + ang.cos() * rr * dist * 1.25, cy + ang.sin() * rr * dist * 0.88, rr * cr)
}

fn tag_groups(a: &App, shown: &[usize]) -> Vec<(String, Vec<usize>)> {
    let mut groups: Vec<(String, Vec<usize>)> = Vec::new();
    for &i in shown {
        let tg = a.graph.nodes[i].tags.first().cloned().unwrap_or_else(|| "untagged".into());
        match groups.iter_mut().find(|g| g.0 == tg) {
            Some(g) => g.1.push(i),
            None => groups.push((tg, vec![i])),
        }
    }
    groups.sort_by(|x, y| y.1.len().cmp(&x.1.len()));
    groups.truncate(9);
    groups
}

fn river_y(x: f32, w: f32, h: f32) -> f32 {
    h * 0.52 + (x / w * TAU * 1.1 + 0.6).sin() * h * 0.14
}

/// Railway map's lines: (name, colour, the route, its stations (node), the kind or "log"/"other").
pub fn lines(a: &App) -> Vec<(String, Color, Vec<(f32, f32)>, Vec<usize>)> {
    let mut out = Vec::new();
    let mut logs: Vec<usize> = (0..a.vault.notes.len()).filter(|&i| a.vault.notes[i].log.is_some() && a.node_shown(i)).collect();
    logs.sort_by(|x, y| a.vault.notes[*x].rel.cmp(&a.vault.notes[*y].rel));
    out.push(("Main Line · daily logs".to_string(), Color::from_rgb8(0xf2, 0xf2, 0xf2), vec![(0.08, 0.52), (0.90, 0.52)], logs));
    let routes: [(&str, &str, [u8; 3], &[(f32, f32)]); 5] = [
        ("mockups", "Mockups line", [0xff, 0x7a, 0x2f], &[(0.17, 0.20), (0.60, 0.20), (0.66, 0.27), (0.66, 0.80), (0.71, 0.87), (0.92, 0.87)]),
        ("assets", "Assets line", [0xf5, 0xc0, 0x3e], &[(0.42, 0.33), (0.72, 0.33), (0.78, 0.40), (0.78, 0.72), (0.84, 0.78), (0.97, 0.78)]),
        ("videos", "Videos line", [0xf2, 0x55, 0x55], &[(0.10, 0.33), (0.31, 0.33), (0.37, 0.40), (0.37, 0.70), (0.33, 0.75), (0.18, 0.75)]),
        ("notes", "Notes line", [0x45, 0xc8, 0xe8], &[(0.06, 0.38), (0.06, 0.72), (0.12, 0.79), (0.26, 0.79)]),
        ("research", "Research line", [0x8b, 0xd3, 0x4a], &[(0.22, 0.42), (0.22, 0.62), (0.16, 0.69)]),
    ];
    for (k, name, c, pts) in routes {
        let st: Vec<usize> = a.vault.items().into_iter().filter(|it| !it.starts_with("other/") && it.split('/').nth(1) == Some(k)).filter_map(|it| a.vault.item_note(&it)).filter(|i| a.node_shown(*i)).collect();
        if st.is_empty() {
            continue;
        }
        out.push((name.to_string(), Color::from_rgb8(c[0], c[1], c[2]), pts.to_vec(), st));
    }
    let dad: Vec<usize> = a.vault.items().into_iter().filter(|it| it.starts_with("other/")).filter_map(|it| a.vault.item_note(&it)).filter(|i| a.node_shown(*i)).collect();
    if !dad.is_empty() {
        out.push(("Dad's line · other".to_string(), Color::from_rgb8(0xff, 0x8f, 0xc0), vec![(0.48, 0.52), (0.48, 0.64), (0.54, 0.70), (0.72, 0.70)], dad));
    }
    out
}

fn along(pts: &[(f32, f32)], frac: f32) -> (f32, f32) {
    let mut total = 0.0;
    for w in pts.windows(2) {
        total += ((w[1].0 - w[0].0).powi(2) + (w[1].1 - w[0].1).powi(2)).sqrt();
    }
    let mut want = total * frac;
    for w in pts.windows(2) {
        let d = ((w[1].0 - w[0].0).powi(2) + (w[1].1 - w[0].1).powi(2)).sqrt();
        if want <= d || d == 0.0 {
            let k = if d == 0.0 { 0.0 } else { want / d };
            return (w[0].0 + (w[1].0 - w[0].0) * k, w[0].1 + (w[1].1 - w[0].1) * k);
        }
        want -= d;
    }
    *pts.last().unwrap_or(&(0.0, 0.0))
}

/// Every station: (node, x, y) in a canvas of this size.
fn railway(a: &App, w: f32, h: f32) -> Vec<(usize, f32, f32)> {
    let m = w.min(h);
    let cam = &a.g.cam;
    let tr = |(x, y): (f32, f32)| -> (f32, f32) { (w / 2.0 + (x * w - w / 2.0) * cam.zoom + cam.x * m, h / 2.0 + (y * h - h / 2.0) * cam.zoom + cam.y * m) };
    let mut out = Vec::new();
    for (li, (_, _, pts, st)) in lines(a).into_iter().enumerate() {
        let n = st.len();
        for (k, i) in st.into_iter().enumerate() {
            let fr = if li == 0 { (k as f32 + 0.5) / n as f32 } else { (k as f32 + 1.0) / (n as f32 + 1.0) };
            if out.iter().any(|x: &(usize, f32, f32)| x.0 == i) {
                continue;
            }
            let (x, y) = tr(along(&pts, fr));
            out.push((i, x, y));
        }
    }
    out
}

fn label(fr: &mut Frame, s: &str, x: f32, y: f32, size: f32, c: Color, centre: bool) {
    let w = s.chars().count() as f32 * size * 0.55;
    let x = if centre { x - w / 2.0 } else { x };
    fr.fill_text(Text { content: s.to_string(), position: Point::new(x, y), color: c, size: Pixels(size), ..Text::default() });
}

fn glow(fr: &mut Frame, at: Point, r: f32, c: Color, strength: f32, cpu: bool) {
    let rings: &[(f32, f32)] = if cpu { &[(3.2, 0.10)] } else { &[(4.4, 0.05), (3.0, 0.08), (2.0, 0.12)] };
    for (k, al) in rings {
        fr.fill(&Path::circle(at, r * k), Pal::a(c, al * strength));
    }
}

fn ellipse(cx: f32, cy: f32, rx: f32, ry: f32, rot: f32, n: usize) -> Path {
    Path::new(|b| {
        for k in 0..=n {
            let t = k as f32 / n as f32 * TAU;
            let (x, y) = (t.cos() * rx, t.sin() * ry);
            let p = Point::new(cx + x * rot.cos() - y * rot.sin(), cy + x * rot.sin() + y * rot.cos());
            if k == 0 { b.move_to(p) } else { b.line_to(p) }
        }
        b.close();
    })
}

/// A little random number for decorations (stars, sparks), steady for the same seed.
fn rnd(seed: u32) -> f32 {
    let mut h = seed.wrapping_mul(0x9e37_79b9) ^ 0x85eb_ca6b;
    h ^= h >> 13;
    h = h.wrapping_mul(0xc2b2_ae35);
    h ^= h >> 16;
    (h % 100_000) as f32 / 100_000.0
}

impl<'a> canvas::Program<Msg> for Sky<'a> {
    type State = SkyState;

    fn update(&self, st: &mut SkyState, event: &canvas::Event, b: Rectangle, cursor: mouse::Cursor) -> Option<canvas::Action<Msg>> {
        let three_d = matches!(self.mode, "galaxy" | "neural" | "solar" | "blackhole" | "ember");
        match event {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) if cursor.is_over(b) => {
                st.press = cursor.position_in(b);
                st.last = st.press;
                st.moved = false;
                Some(canvas::Action::request_redraw().and_capture())
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let pos = cursor.position_in(b);
                if let (Some(last), Some(p)) = (st.last, pos) {
                    if st.press.is_some() {
                        let (dx, dy) = (p.x - last.x, p.y - last.y);
                        if dx.abs() + dy.abs() > 0.5 {
                            st.last = Some(p);
                            if (p.x - st.press.unwrap_or(p).x).abs() + (p.y - st.press.unwrap_or(p).y).abs() > 4.0 {
                                st.moved = true;
                            }
                            let m = b.width.min(b.height).max(1.0);
                            return Some(canvas::Action::publish(if three_d { Msg::GTurn(dx / m * 2.0, dy / m * 1.2) } else { Msg::GPan(dx / m, dy / m) }).and_capture());
                        }
                        return None;
                    }
                }
                let Some(p) = pos else {
                    if st.hover.take().is_some() {
                        return Some(canvas::Action::publish(Msg::GHover(None)));
                    }
                    return None;
                };
                let near = self.place(b.width, b.height).into_iter().filter(|q| (q.x - p.x).powi(2) + (q.y - p.y).powi(2) < (q.r + 6.0).powi(2)).min_by(|x, y| ((x.x - p.x).powi(2) + (x.y - p.y).powi(2)).total_cmp(&((y.x - p.x).powi(2) + (y.y - p.y).powi(2)))).map(|q| q.i);
                if near != st.hover {
                    st.hover = near;
                    return Some(canvas::Action::publish(Msg::GHover(near)));
                }
                None
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                st.press?;
                st.press = None;
                st.last = None;
                if st.moved {
                    return Some(canvas::Action::request_redraw());
                }
                let hit = st.hover;
                if self.small {
                    return Some(canvas::Action::publish(match hit { Some(i) => Msg::GOpen(i), None => Msg::GClick(None) }).and_capture());
                }
                Some(canvas::Action::publish(Msg::GClick(hit)).and_capture())
            }
            canvas::Event::Mouse(mouse::Event::WheelScrolled { delta }) if cursor.is_over(b) => {
                let y = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => *y,
                    mouse::ScrollDelta::Pixels { y, .. } => *y / 40.0,
                };
                Some(canvas::Action::publish(Msg::GZoom(1.12f32.powf(y))).and_capture())
            }
            _ => None,
        }
    }

    fn draw(&self, st: &SkyState, renderer: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        let (w, h) = (b.width, b.height);
        if !self.small {
            if let Ok(mut l) = LAST.lock() {
                *l = (w, h);
            }
        }
        let a = self.a;
        let cpu = a.cpu;
        let key = Some((hash(self.mode) as u64, w as u32, h as u32 ^ (a.g.cam.zoom * 100.0) as u32));
        if st.key.get() != key {
            st.bg.clear();
            st.key.set(key);
        }
        let bg = st.bg.draw(renderer, b.size(), |fr| self.background(fr, w, h, cpu));
        let mut fr = Frame::new(renderer, b.size());
        let placed = self.place(w, h);
        self.links(&mut fr, &placed, w, h);
        self.decor(&mut fr, &placed, w, h);
        self.nodes(&mut fr, &placed);
        vec![bg, fr.into_geometry()]
    }
}

impl<'a> Sky<'a> {
    fn background(&self, fr: &mut Frame, w: f32, h: f32, cpu: bool) {
        let a = self.a;
        fr.fill_rectangle(Point::ORIGIN, Size::new(w, h), bg_of(self.mode));
        let m = w.min(h);
        let stars = |fr: &mut Frame, n: usize, seed: u32, top: f32| {
            for k in 0..n {
                let (x, y) = (rnd(seed + k as u32 * 3) * w, rnd(seed + k as u32 * 3 + 1) * h * top);
                let s = rnd(seed + k as u32 * 3 + 2);
                fr.fill(&Path::circle(Point::new(x, y), 0.4 + s * 0.9), Color { a: 0.12 + s * 0.45, ..Color::WHITE });
            }
        };
        let dense = if cpu { 0.35 } else { 1.0 };
        let small_k = if self.small { 0.4 } else { 1.0 };
        match self.mode {
            "classic" | "railway" => {
                if self.mode == "railway" {
                    let step = 36.0 * a.g.cam.zoom.max(0.5);
                    let mut x = 0.0;
                    while x < w {
                        let mut y = 0.0;
                        while y < h {
                            fr.fill_rectangle(Point::new(x, y), Size::new(1.5, 1.5), Color { a: 0.035, ..Color::WHITE });
                            y += step;
                        }
                        x += step;
                    }
                } else if a.gb("starfield", false) {
                    stars(fr, (220.0 * dense * small_k) as usize, 7, 1.0);
                }
            }
            "galaxy" => {
                stars(fr, (900.0 * dense * small_k) as usize, 11, 1.0);
                // The galaxy's warm core.
                let c = Point::new(w * 0.52, h * 0.47);
                for (k, al) in [(0.55, 0.05), (0.38, 0.06), (0.22, 0.08), (0.10, 0.10)] {
                    fr.fill(&ellipse(c.x, c.y, m * k * 1.2, m * k * 0.7, -0.15, 48), Color::from_rgba8(0xff, 0xb0, 0x70, al));
                }
            }
            "solar" | "blackhole" => {
                stars(fr, (700.0 * dense * small_k) as usize, 13, 1.0);
            }
            "nightsky" => {
                stars(fr, (600.0 * dense * small_k) as usize, 17, 0.8);
                let land = Path::new(|bb| {
                    bb.move_to(Point::new(0.0, h));
                    let n = 40;
                    for k in 0..=n {
                        let x = k as f32 / n as f32 * w;
                        let y = h * 0.84 - ((x / w * 9.0).sin() * 0.04 + (x / w * 23.0).sin() * 0.015 + 0.03) * h;
                        bb.line_to(Point::new(x, y));
                    }
                    bb.line_to(Point::new(w, h));
                    bb.close();
                });
                fr.fill(&land, Color::from_rgb8(0x05, 0x07, 0x0c));
                fr.fill(&Path::circle(Point::new(w * 0.72, h * 0.80), m * 0.025), Color::from_rgba8(0xff, 0x9a, 0x52, 0.7));
            }
            "neural" => {
                stars(fr, (500.0 * dense * small_k) as usize, 19, 1.0);
                fr.fill(&ellipse(w / 2.0, h / 2.0, w * 0.42, h * 0.36, 0.0, 64), Color::from_rgba8(0x7a, 0x4d, 0xd6, 0.06));
            }
            "city" => {
                stars(fr, (260.0 * dense * small_k) as usize, 23, 0.55);
                let g = canvas::gradient::Linear::new(Point::new(0.0, h * 0.5), Point::new(0.0, h)).add_stop(0.0, Color::TRANSPARENT).add_stop(1.0, Color::from_rgba8(0x3a, 0x4a, 0x80, 0.35));
                fr.fill_rectangle(Point::new(0.0, h * 0.5), Size::new(w, h * 0.5), g);
            }
            "river" => {
                stars(fr, (160.0 * dense * small_k) as usize, 29, 0.4);
                let band = Path::new(|bb| {
                    let n = 80;
                    for k in 0..=n {
                        let x = k as f32 / n as f32 * w;
                        let p = Point::new(x, river_y(x, w, h));
                        if k == 0 { bb.move_to(p) } else { bb.line_to(p) }
                    }
                });
                fr.stroke(&band, Stroke::default().with_color(Color::from_rgba8(0x2a, 0x5e, 0xa8, 0.55)).with_width(m * 0.11 * a.g.cam.zoom).with_line_cap(LineCap::Round));
                fr.stroke(&band, Stroke::default().with_color(Color::from_rgba8(0x5a, 0x9c, 0xe8, 0.35)).with_width(m * 0.035 * a.g.cam.zoom).with_line_cap(LineCap::Round));
            }
            "ember" => {
                let base = Point::new(w / 2.0, h * 0.86);
                for (k, al) in [(0.95, 0.05), (0.7, 0.07), (0.48, 0.09), (0.3, 0.12)] {
                    fr.fill(&ellipse(base.x, base.y, w * k * 0.6, h * k * 0.55, 0.0, 48), Color::from_rgba8(0xd2, 0x5a, 0x1e, al));
                }
            }
            _ => {}
        }
    }

    fn links(&self, fr: &mut Frame, placed: &[P], w: f32, h: f32) {
        let a = self.a;
        let at: std::collections::HashMap<usize, (f32, f32)> = placed.iter().map(|q| (q.i, (q.x, q.y))).collect();
        let sel = a.g.sel;
        let curved = a.gs("linkStyle", "straight") == "curved";
        let by_type = a.gb("colourByType", true);
        let glow_on = a.gb("glow", true);
        let arrows = a.gb("arrows", false);
        let index_links = a.gb("indexLinks", false);
        let home = a.vault.find("INDEX.md");
        let accent = a.p.accent;
        let tt = a.g.t;
        if self.mode == "railway" {
            self.rail_lines(fr, w, h);
            return;
        }
        if matches!(self.mode, "blackhole" | "city" | "river") {
            return;
        }
        for &(x, y, kind) in a.graph.edges.iter() {
            let (Some(&(ax, ay)), Some(&(bx_, by))) = (at.get(&x), at.get(&y)) else { continue };
            if !index_links && (Some(x) == home || Some(y) == home) && self.mode == "classic" {
                continue;
            }
            let hot = sel.map(|s| s == x || s == y).unwrap_or(false);
            let base = match (self.mode, by_type, kind) {
                ("galaxy", _, _) => Color::from_rgba8(0xff, 0xb0, 0x70, 0.12),
                ("neural", _, _) => Color::from_rgba8(0xb0, 0x90, 0xff, 0.14),
                ("ember", _, _) => Color::from_rgba8(0xff, 0x8a, 0x3d, 0.14),
                ("nightsky", _, _) => Color::from_rgba8(0xff, 0xff, 0xff, 0.05),
                ("solar", _, _) => Color::from_rgba8(0xff, 0xff, 0xff, 0.0),
                (_, true, EdgeKind::Mention) => Color::from_rgba8(0xff, 0x9a, 0x52, 0.16),
                (_, true, EdgeKind::Embed) => Color::from_rgba8(0x6a, 0xa8, 0xff, 0.22),
                _ => Color::from_rgba8(0xff, 0xff, 0xff, 0.10),
            };
            let c = if hot { Pal::a(accent, 0.9) } else { base };
            if c.a <= 0.0 {
                continue;
            }
            let wdt = if hot { 1.8 } else { 0.8 };
            let path = if self.mode == "ember" {
                // Heat shimmer: a wavy line.
                Path::new(|bb| {
                    let n = 14;
                    let (dx, dy) = (bx_ - ax, by - ay);
                    let len = (dx * dx + dy * dy).sqrt().max(1.0);
                    let (nx, ny) = (-dy / len, dx / len);
                    for k in 0..=n {
                        let s = k as f32 / n as f32;
                        let wv = (s * 12.0 + tt * 2.0).sin() * 3.0 * (s * PI).sin();
                        let p = Point::new(ax + dx * s + nx * wv, ay + dy * s + ny * wv);
                        if k == 0 { bb.move_to(p) } else { bb.line_to(p) }
                    }
                })
            } else if curved {
                let (mx, my) = ((ax + bx_) / 2.0, (ay + by) / 2.0);
                let (dx, dy) = (bx_ - ax, by - ay);
                Path::new(|bb| {
                    bb.move_to(Point::new(ax, ay));
                    bb.quadratic_curve_to(Point::new(mx - dy * 0.18, my + dx * 0.18), Point::new(bx_, by));
                })
            } else {
                Path::line(Point::new(ax, ay), Point::new(bx_, by))
            };
            if hot && glow_on && !a.cpu {
                fr.stroke(&path, Stroke::default().with_color(Pal::a(accent, 0.25)).with_width(5.0));
            }
            fr.stroke(&path, Stroke::default().with_color(c).with_width(wdt));
            if arrows && self.mode == "classic" {
                let (dx, dy) = (bx_ - ax, by - ay);
                let len = (dx * dx + dy * dy).sqrt().max(1.0);
                let (ux, uy) = (dx / len, dy / len);
                let tip = Point::new(bx_ - ux * 6.0, by - uy * 6.0);
                let ar = Path::new(|bb| {
                    bb.move_to(tip);
                    bb.line_to(Point::new(tip.x - ux * 6.0 - uy * 3.5, tip.y - uy * 6.0 + ux * 3.5));
                    bb.line_to(Point::new(tip.x - ux * 6.0 + uy * 3.5, tip.y - uy * 6.0 - ux * 3.5));
                    bb.close();
                });
                fr.fill(&ar, c);
            }
            // Particles along the open note's links for a few seconds (or links firing, Neural).
            let burst = (a.g.burst_until - tt).max(0.0);
            if hot && (burst > 0.0 || self.mode == "neural") && a.gb("particles", true) && !a.cpu && a.animating() {
                for k in 0..3 {
                    let s = ((tt * 0.6 + k as f32 / 3.0) % 1.0).abs();
                    fr.fill(&Path::circle(Point::new(ax + (bx_ - ax) * s, ay + (by - ay) * s), 2.2), Color::from_rgba8(0xff, 0xd8, 0x9a, 0.9));
                }
            }
        }
    }

    fn rail_lines(&self, fr: &mut Frame, w: f32, h: f32) {
        let a = self.a;
        let m = w.min(h);
        let cam = &a.g.cam;
        let tr = |(x, y): (f32, f32)| Point::new(w / 2.0 + (x * w - w / 2.0) * cam.zoom + cam.x * m, h / 2.0 + (y * h - h / 2.0) * cam.zoom + cam.y * m);
        for (li, (_, c, pts, _)) in lines(a).into_iter().enumerate() {
            let path = Path::new(|bb| {
                for (k, p) in pts.iter().enumerate() {
                    if k == 0 { bb.move_to(tr(*p)) } else { bb.line_to(tr(*p)) }
                }
            });
            fr.stroke(&path, Stroke::default().with_color(c).with_width(if li == 0 { 9.0 } else { 6.5 } * cam.zoom.sqrt()).with_line_cap(LineCap::Round).with_line_join(canvas::LineJoin::Round));
        }
    }

    /// What each mode draws besides its notes and links (suns and rings, the black hole, the fire, the train…).
    fn decor(&self, fr: &mut Frame, placed: &[P], w: f32, h: f32) {
        let a = self.a;
        let m = w.min(h);
        let cam = &a.g.cam;
        let tt = a.g.t;
        let cpu = a.cpu;
        let counts = a.vault.project_counts();
        match self.mode {
            "galaxy" if a.gb("nebulae", true) => {
                for pj in PROJECTS.iter().chain(["log"].iter()) {
                    let mem: Vec<&P> = placed.iter().filter(|q| a.graph.nodes[q.i].project == *pj).collect();
                    if mem.is_empty() {
                        continue;
                    }
                    let (sx, sy) = mem.iter().fold((0.0, 0.0), |s, q| (s.0 + q.x, s.1 + q.y));
                    let (mx, my) = (sx / mem.len() as f32, sy / mem.len() as f32);
                    let col = project_color(&a.p, pj);
                    let r = m * (0.08 + (mem.len() as f32).sqrt() * 0.02).min(0.3);
                    let layers = if cpu { 2 } else { 5 };
                    for k in 0..layers {
                        let s = 1.0 - k as f32 * 0.17;
                        fr.fill(&Path::circle(Point::new(mx + (k as f32 * 1.7).sin() * r * 0.2, my + (k as f32 * 2.3).cos() * r * 0.15), r * s), Pal::a(col, 0.035));
                    }
                }
                if let Some(t_) = a.g.flew.and_then(|i| placed.iter().find(|q| q.i == i)) {
                    // Flying the camera: a dashed arc from the centre to the note, and a ring on it.
                    let (sx, sy) = (w * 0.55, h * 0.42);
                    let arc = Path::new(|bb| {
                        bb.move_to(Point::new(sx, sy));
                        bb.quadratic_curve_to(Point::new((sx + t_.x) / 2.0 + 60.0, sy.min(t_.y) - 80.0), Point::new(t_.x, t_.y));
                    });
                    let mut s = Stroke::default().with_color(Pal::a(a.p.accent, 0.9)).with_width(1.6);
                    s.line_dash = canvas::LineDash { segments: &[5.0, 5.0], offset: 0 };
                    fr.stroke(&arc, s);
                    fr.stroke(&Path::circle(Point::new(t_.x, t_.y), 10.0), Stroke::default().with_color(Color::WHITE).with_width(1.6));
                }
            }
            "solar" => {
                let slots = [(0.42, 0.55), (0.10, 0.17), (0.34, 0.10), (0.82, 0.60), (0.92, 0.72), (0.80, 0.86), (0.62, 0.93), (0.12, 0.84)];
                for (slot, pj) in solar_order(a).iter().enumerate().take(8) {
                    let (sx, sy) = slots[slot];
                    let c = Point::new(w * sx + cam.x * m, h * sy + cam.y * m);
                    let n = *counts.get(pj).unwrap_or(&0);
                    let col = project_color(&a.p, pj);
                    if n == 0 {
                        fr.fill(&Path::circle(c, m * 0.012), Pal::a(col, 0.8));
                        glow(fr, c, m * 0.012, col, 0.7, cpu);
                        if !self.small {
                            label(fr, pj, c.x, c.y + m * 0.018, f(13.0), Pal::a(col, 0.95), true);
                            label(fr, "no notes yet", c.x, c.y + m * 0.018 + f(13.0) * 1.2, f(11.0), Color::from_rgba8(0xa0, 0xa4, 0xae, 0.9), true);
                        }
                        continue;
                    }
                    let big = slot == 0;
                    let r = if big { m * 0.055 * cam.zoom } else { m * 0.022 };
                    if big {
                        for (k, kd) in KINDS.iter().enumerate() {
                            let rx = m * cam.zoom * (0.20 + 0.105 * (4.0 - k as f32));
                            fr.stroke(&ellipse(c.x, c.y, rx, rx * 0.42, 0.0, 96), Stroke::default().with_color(Color::from_rgba8(0xff, 0x9a, 0x52, 0.22)).with_width(1.0));
                            if !self.small {
                                label(fr, kd, c.x, c.y - rx * 0.42 - f(12.0) * 1.2, f(12.0), Color::from_rgba8(0xff, 0x9a, 0x52, 0.75), true);
                            }
                        }
                    } else {
                        fr.stroke(&ellipse(c.x, c.y, m * 0.07, m * 0.03, 0.0, 48), Stroke::default().with_color(Pal::a(col, 0.35)).with_width(1.0));
                    }
                    let halo = if cpu { 2 } else { 6 };
                    for k in 0..halo {
                        fr.fill(&Path::circle(c, r * (1.4 + k as f32 * 0.6)), Color::from_rgba8(0xff, 0x9a, 0x40, 0.05));
                    }
                    let sun = if big { Color::from_rgb8(0xfb, 0xf2, 0xe0) } else { Pal::mix(col, Color::WHITE, 0.6) };
                    fr.fill(&Path::circle(c, r), sun);
                    if !self.small {
                        let tc = if big { Color::from_rgb8(0xff, 0xd2, 0xa8) } else { col };
                        label(fr, pj, c.x, c.y + r + 4.0, f(if big { 17.0 } else { 14.0 }), tc, true);
                    }
                }
                // Comets' tails (the daily logs) and the planets' moons.
                for q in placed {
                    let nd = &a.graph.nodes[q.i];
                    if nd.project == "log" && a.gb("comets", true) {
                        fr.stroke(&Path::line(Point::new(q.x - 22.0, q.y + 9.0), Point::new(q.x, q.y)), Stroke::default().with_color(Color::from_rgba8(0xff, 0xff, 0xff, 0.35)).with_width(1.4).with_line_cap(LineCap::Round));
                        if !self.small && !self.tile {
                            label(fr, &format!("log {}", nd.label), q.x, q.y - f(12.0) * 1.5, f(11.0), Color::from_rgba8(0xe6, 0xe6, 0xea, 0.85), true);
                        }
                    } else if nd.links > 0 && !self.small {
                        let moons = nd.links.min(3);
                        for k in 0..moons {
                            let ang = tt * 0.6 + k as f32 * TAU / moons as f32 + unit(&a.vault.notes[q.i].rel, 9) * TAU;
                            fr.fill(&Path::circle(Point::new(q.x + ang.cos() * (q.r + 5.0), q.y + ang.sin() * (q.r + 5.0) * 0.6), 1.4), Color::from_rgba8(0xff, 0xff, 0xff, 0.8));
                        }
                    }
                }
            }
            "blackhole" => {
                let c = Point::new(w / 2.0 + cam.x * m, h / 2.0 + cam.y * m);
                let rr = m * 0.5 * cam.zoom;
                // Faint orbit rings, the clouds' link trails pulled toward Home, the disk, the dark core.
                for k in [0.72, 1.18, 1.6] {
                    fr.stroke(&ellipse(c.x, c.y, rr * k * 1.25, rr * k * 0.95, 0.0, 120), Stroke::default().with_color(Color::from_rgba8(0xff, 0xff, 0xff, 0.05)).with_width(1.0));
                }
                for pj in PROJECTS {
                    let (ccx, ccy, cr) = cloud_of(a, pj, c.x, c.y, rr);
                    let n = *counts.get(pj).unwrap_or(&0);
                    let col = project_color(&a.p, pj);
                    let dir = ((c.x - ccx), (c.y - ccy));
                    let dl = (dir.0 * dir.0 + dir.1 * dir.1).sqrt().max(1.0);
                    let focal = Point::new(ccx + dir.0 / dl * cr * 1.2, ccy + dir.1 / dl * cr * 1.0);
                    if n > 0 {
                        let mem: Vec<&P> = placed.iter().filter(|q| a.graph.nodes[q.i].project == pj).collect();
                        let step = if cpu { 4 } else { 1 };
                        for q in mem.iter().step_by(step) {
                            fr.stroke(&Path::line(Point::new(q.x, q.y), focal), Stroke::default().with_color(Pal::a(Pal::mix(col, Color::WHITE, 0.5), 0.13)).with_width(0.7));
                        }
                        for k in 0..40u32 {
                            // The fan: trails from all over the cloud to its edge facing Home.
                            let ang = rnd(hash(pj) + 900 + k) * TAU;
                            let d = rnd(hash(pj) + 950 + k).sqrt();
                            let from = Point::new(ccx + ang.cos() * cr * 1.3 * d, ccy + ang.sin() * cr * 0.95 * d);
                            fr.stroke(&Path::line(from, focal), Stroke::default().with_color(Color::from_rgba8(0xff, 0xe8, 0xc8, 0.10)).with_width(0.8));
                        }
                        fr.stroke(&Path::line(focal, Point::new(c.x - dir.0 / dl * rr * 0.12, c.y - dir.1 / dl * rr * 0.09)), Stroke::default().with_color(Color::from_rgba8(0xff, 0xd0, 0xa0, 0.10)).with_width(1.0));
                        // Star dust in the cloud (drawn, not notes): as much as the project has notes, so the cloud's
                        // size and brightness follow its real count; its label says the count itself.
                        let dust = (n as u32 * if cpu { 8 } else { 30 }).min(if cpu { 300 } else { 1400 });
                        for k in 0..dust {
                            let (u, v) = (rnd(hash(pj) + k * 2), rnd(hash(pj) + k * 2 + 1));
                            let ang = v * TAU;
                            let d = u.powf(0.6);
                            let bright = rnd(hash(pj) + k * 2 + 7);
                            fr.fill(&Path::circle(Point::new(ccx + ang.cos() * cr * 1.35 * d, ccy + ang.sin() * cr * 1.0 * d), 0.6 + bright * 1.1), Pal::a(Pal::mix(col, Color::WHITE, 0.7), 0.40 + bright * 0.55));
                        }
                    }
                    if !self.small {
                        let ly = ccy - cr * 1.05 - f(26.0);
                        fr.fill(&Path::circle(Point::new(ccx - (pj.len() as f32 * f(16.0) * 0.3) - 9.0, ly + f(16.0) * 0.55), 3.5), col);
                        label(fr, pj, ccx, ly, f(16.0), Color::WHITE, true);
                        let sub = if n == 0 { "no notes yet".to_string() } else { format!("{n} page{}", crate::plural(n)) };
                        label(fr, &sub, ccx, ly + f(16.0) * 1.2, f(12.0), Color::from_rgba8(0xc0, 0xc4, 0xcc, 0.9), true);
                    }
                }
                let disk = rr * 0.62;
                let layers = if cpu { 4 } else { 12 };
                for k in 0..layers {
                    let s = k as f32 / layers as f32;
                    fr.stroke(&ellipse(c.x, c.y, disk * (1.0 - s * 0.55), disk * 0.09 * (1.0 - s * 0.4), -0.04, 96), Stroke::default().with_color(Pal::mix(Color::from_rgba8(0xff, 0x8a, 0x2a, 0.18), Color::from_rgba8(0xff, 0xe8, 0xb0, 0.35), s)).with_width(3.0 + s * 3.0));
                }
                for (k, al) in [(0.30, 0.10), (0.24, 0.16), (0.20, 0.24)] {
                    fr.fill(&Path::circle(c, rr * k), Color::from_rgba8(0xff, 0xa0, 0x40, al));
                }
                fr.stroke(&Path::circle(c, rr * 0.19), Stroke::default().with_color(Color::from_rgba8(0xff, 0xd8, 0xa0, 0.75)).with_width(3.0));
                fr.fill(&Path::circle(c, rr * 0.17), Color::from_rgb8(0x02, 0x02, 0x03));
                // The near side of the disk crosses the core.
                fr.fill(&ellipse(c.x, c.y + rr * 0.005, rr * 0.20, rr * 0.016, -0.04, 48), Color::from_rgba8(0xff, 0xe0, 0xb0, 0.85));
                if !self.small {
                    let py = c.y + rr * 0.26;
                    fr.fill(&Path::rounded_rectangle(Point::new(c.x - 34.0, py), Size::new(68.0, 24.0), 12.0.into()), Color::from_rgba8(0x2a, 0x2c, 0x32, 0.95));
                    label(fr, "Home", c.x, py + 5.0, f(15.0), Color::WHITE, true);
                }
            }
            "ember" => {
                let base = Point::new(w / 2.0, h * 0.86 + cam.y * m);
                let fw = m * 0.20;
                let flick = |k: f32| -> f32 { if a.animating() { (tt * 6.0 + k).sin() * 0.06 } else { 0.0 } };
                for (k, (hh, col)) in [(1.0f32, Color::from_rgb8(0xd8, 0x4a, 0x14)), (0.82, Color::from_rgb8(0xf2, 0x7a, 0x20)), (0.62, Color::from_rgb8(0xff, 0xb0, 0x40)), (0.40, Color::from_rgb8(0xff, 0xe6, 0x9a))].into_iter().enumerate() {
                    for lobe in [-0.55f32, 0.0, 0.5] {
                        let hgt = fw * 1.25 * hh * (1.0 + flick(k as f32 + lobe * 3.0)) * if lobe == 0.0 { 1.0 } else { 0.78 };
                        let wid = fw * 0.42 * hh;
                        let bx0 = base.x + lobe * fw * 0.75 * hh;
                        let fl = Path::new(|bb| {
                            bb.move_to(Point::new(bx0 - wid, base.y));
                            bb.bezier_curve_to(Point::new(bx0 - wid, base.y - hgt * 0.5), Point::new(bx0 - wid * 0.2, base.y - hgt * 0.7), Point::new(bx0 + wid * 0.1, base.y - hgt));
                            bb.bezier_curve_to(Point::new(bx0 + wid * 0.4, base.y - hgt * 0.6), Point::new(bx0 + wid, base.y - hgt * 0.45), Point::new(bx0 + wid, base.y));
                            bb.close();
                        });
                        fr.fill(&fl, col);
                    }
                }
                let log_c = Color::from_rgb8(0x2a, 0x1a, 0x12);
                fr.stroke(&Path::line(Point::new(base.x - fw * 1.25, base.y - 6.0), Point::new(base.x + fw * 1.25, base.y + 22.0)), Stroke::default().with_color(log_c).with_width(11.0).with_line_cap(LineCap::Round));
                fr.stroke(&Path::line(Point::new(base.x + fw * 1.25, base.y - 6.0), Point::new(base.x - fw * 1.25, base.y + 22.0)), Stroke::default().with_color(log_c).with_width(11.0).with_line_cap(LineCap::Round));
                if a.gb("sparks", true) {
                    let n = if cpu { 12 } else { 40 };
                    for k in 0..n {
                        let s = ((tt * 0.12 + rnd(k as u32 + 400)) % 1.0).abs();
                        let x = base.x + (rnd(k as u32 + 500) - 0.5) * fw * 2.0 * (1.0 + s);
                        let y = base.y - fw * 0.6 - s * h * 0.5;
                        fr.fill(&Path::circle(Point::new(x, y), 1.1), Color::from_rgba8(0xff, 0xc0, 0x70, 0.8 * (1.0 - s)));
                    }
                }
            }
            "railway" => {
                // Station names, the Main Line's days and today's train.
                let logs: Vec<&P> = placed.iter().filter(|q| a.graph.nodes[q.i].project == "log").collect();
                for q in &logs {
                    if let Some(d) = a.graph.nodes[q.i].log {
                        if !self.small {
                            label(fr, &day::short(d), q.x, q.y - 22.0, f(12.0), Color::WHITE, true);
                        }
                    }
                }
                if let Some(q) = logs.iter().find(|q| a.graph.nodes[q.i].log == Some(a.today)).or(logs.last()) {
                    let tw = 30.0;
                    fr.fill(&Path::rounded_rectangle(Point::new(q.x - tw / 2.0, q.y - 25.0), Size::new(tw, 11.0), 3.0.into()), a.p.accent);
                    for k in 0..3 {
                        fr.fill_rectangle(Point::new(q.x - tw / 2.0 + 4.0 + k as f32 * 8.0, q.y - 22.5), Size::new(5.0, 5.0), Color::from_rgb8(0x2a, 0x12, 0x06));
                    }
                    if !self.small {
                        label(fr, "today", q.x, q.y - 40.0, f(11.0), accent2(&a.p), true);
                    }
                }
            }
            "nightsky" => {
                let shown: Vec<usize> = placed.iter().map(|q| q.i).collect();
                for (tg, mem) in tag_groups(a, &shown) {
                    let pts: Vec<&P> = mem.iter().filter_map(|i| placed.iter().find(|q| q.i == *i)).collect();
                    for wnd in pts.windows(2).take(8) {
                        fr.stroke(&Path::line(Point::new(wnd[0].x, wnd[0].y), Point::new(wnd[1].x, wnd[1].y)), Stroke::default().with_color(Color::from_rgba8(0xc8, 0xd4, 0xff, 0.35)).with_width(1.0));
                    }
                    if let (Some(first), false) = (pts.first(), self.small) {
                        label(fr, &format!("#{tg}"), first.x, first.y + 14.0, f(11.0), Color::from_rgba8(0xc8, 0xd4, 0xff, 0.7), true);
                    }
                }
            }
            "city" => {
                for q in placed {
                    let ground = h * 0.86;
                    let bw = q.r * 2.2;
                    let top = q.y;
                    let nd = &a.graph.nodes[q.i];
                    let busy = (nd.links as f32 + *a.opened.get(&a.vault.notes[q.i].rel).unwrap_or(&0) as f32).min(12.0) / 12.0;
                    fr.fill_rectangle(Point::new(q.x - bw / 2.0, top), Size::new(bw, ground - top), Pal::mix(Color::from_rgb8(0x14, 0x18, 0x26), q.c, 0.10));
                    let rows_ = ((ground - top) / 7.0) as i32;
                    let cols_ = ((bw - 3.0) / 5.0).max(1.0) as i32;
                    for ry in 0..rows_ {
                        for cx_ in 0..cols_ {
                            let lit = rnd(hash(&a.vault.notes[q.i].rel) + (ry * 7 + cx_) as u32) < 0.25 + busy * 0.6;
                            if lit {
                                fr.fill_rectangle(Point::new(q.x - bw / 2.0 + 2.0 + cx_ as f32 * 5.0, top + 3.0 + ry as f32 * 7.0), Size::new(2.6, 3.2), Color::from_rgba8(0xff, 0xd2, 0x7a, 0.55 + busy * 0.45));
                            }
                        }
                    }
                }
            }
            "river" => {
                let mut done: Vec<day::Ymd> = Vec::new();
                for q in placed {
                    let d = a.graph.nodes[q.i].log.unwrap_or_else(|| day::log_day(a.graph.nodes[q.i].mtime, 0));
                    if done.contains(&d) || self.small {
                        continue;
                    }
                    done.push(d);
                    label(fr, &format!("{:02}-{:02}", d.1, d.2), q.x, q.y - 30.0, f(11.0), Color::from_rgba8(0xd8, 0xe4, 0xff, 0.85), true);
                }
            }
            _ => {}
        }
    }

    fn nodes(&self, fr: &mut Frame, placed: &[P]) {
        let a = self.a;
        let cpu = a.cpu;
        let sel = a.g.sel;
        let bloom = a.gf("bloom", 0.7);
        let lsize = a.gf("labelSize", 0.5) * 0.6 + 0.7;
        let fade = a.gb("labelsFade", true);
        let shapes = a.gs("shapesBy", "none");
        if matches!(self.mode, "city") {
            return;
        }
        for q in placed {
            let at = Point::new(q.x, q.y);
            let nd = &a.graph.nodes[q.i];
            let c = Pal::a(q.c, q.a);
            match self.mode {
                "blackhole" => fr.fill(&Path::circle(at, q.r), c),
                "railway" => {
                    fr.fill(&Path::circle(at, q.r), Color::WHITE);
                    fr.stroke(&Path::circle(at, q.r), Stroke::default().with_color(Color::from_rgb8(0x10, 0x12, 0x16)).with_width(if nd.project == "log" { 2.5 } else { 1.5 }));
                    if nd.project == "log" {
                        fr.fill(&Path::circle(at, q.r * 0.45), Color::from_rgb8(0x10, 0x12, 0x16));
                    }
                }
                "river" => {
                    glow(fr, at, q.r, q.c, 0.8, cpu);
                    fr.fill(&Path::rounded_rectangle(Point::new(q.x - 3.0, q.y - 2.5), Size::new(6.0, 5.0), 1.0.into()), c);
                }
                "solar" if nd.project != "log" => {
                    glow(fr, at, q.r, q.c, 0.5, cpu);
                    fr.fill(&Path::circle(at, q.r), c);
                    fr.fill(&Path::circle(Point::new(q.x - q.r * 0.35, q.y - q.r * 0.35), q.r * 0.45), Color::from_rgba8(0xff, 0xd8, 0xb0, 0.45));
                }
                _ => {
                    if a.gb("glow", true) || self.mode != "classic" {
                        glow(fr, at, q.r, q.c, bloom * if self.mode == "classic" { 0.9 } else { 1.2 }, cpu);
                    }
                    let shape = match shapes.as_str() {
                        "kind" => nd.kind.as_str(),
                        _ => "",
                    };
                    let path = match shape {
                        "mockups" => Path::rectangle(Point::new(q.x - q.r, q.y - q.r * 0.8), Size::new(q.r * 2.0, q.r * 1.6)),
                        "videos" => Path::new(|bb| {
                            bb.move_to(Point::new(q.x, q.y - q.r * 1.1));
                            bb.line_to(Point::new(q.x + q.r, q.y + q.r * 0.8));
                            bb.line_to(Point::new(q.x - q.r, q.y + q.r * 0.8));
                            bb.close();
                        }),
                        "notes" => Path::new(|bb| {
                            bb.move_to(Point::new(q.x, q.y - q.r * 1.15));
                            bb.line_to(Point::new(q.x + q.r * 1.15, q.y));
                            bb.line_to(Point::new(q.x, q.y + q.r * 1.15));
                            bb.line_to(Point::new(q.x - q.r * 1.15, q.y));
                            bb.close();
                        }),
                        "research" => Path::new(|bb| {
                            for k in 0..6 {
                                let ang = k as f32 * TAU / 6.0;
                                let p = Point::new(q.x + ang.cos() * q.r, q.y + ang.sin() * q.r);
                                if k == 0 { bb.move_to(p) } else { bb.line_to(p) }
                            }
                            bb.close();
                        }),
                        _ => Path::circle(at, q.r),
                    };
                    fr.fill(&path, c);
                    if self.mode == "galaxy" && q.r > 3.4 && !cpu {
                        let s = Stroke::default().with_color(Color::from_rgba8(0xff, 0xff, 0xff, 0.45)).with_width(0.8);
                        fr.stroke(&Path::line(Point::new(q.x - q.r * 3.0, q.y), Point::new(q.x + q.r * 3.0, q.y)), s);
                        fr.stroke(&Path::line(Point::new(q.x, q.y - q.r * 3.0), Point::new(q.x, q.y + q.r * 3.0)), s);
                    }
                }
            }
            if Some(q.i) == sel && !self.small {
                fr.stroke(&Path::circle(at, q.r + 5.0), Stroke::default().with_color(Color::WHITE).with_width(2.0));
            } else if Some(q.i) == sel {
                fr.stroke(&Path::circle(at, q.r + 4.0), Stroke::default().with_color(Color::WHITE).with_width(1.6));
            }
            if q.label && self.mode != "blackhole" && !self.tile {
                let z_ = a.g.cam.zoom;
                let alpha = if fade && z_ < 0.8 { (z_ / 0.8).powi(2) } else { 1.0 };
                let size = f(12.0) * lsize * if self.small { 0.9 } else { 1.0 };
                let text_c = Color { a: 0.85 * alpha, ..Color::from_rgb8(0xe8, 0xe6, 0xe2) };
                let name = if nd.project == "log" && self.mode == "ember" { format!("log {}", nd.label) } else { nd.label.clone() };
                if self.mode == "railway" {
                    label(fr, &name, q.x + 7.0, q.y + 4.0, size * 0.92, text_c, false);
                } else {
                    label(fr, &name, q.x, q.y + q.r + 3.0, size, text_c, true);
                }
            }
        }
    }
}

/// The canvas for a mode at a size.
pub fn sky<'a>(a: &'a App, mode: &'a str, small: bool, local: bool) -> E<'a, Msg> {
    canvas::Canvas::new(Sky { a, mode, small, local, tile: false }).width(Length::Fill).height(Length::Fill).into()
}

/// A mode's preview tile (Graph modes, saved looks): no labels.
pub fn tile<'a>(a: &'a App, mode: &'a str) -> E<'a, Msg> {
    canvas::Canvas::new(Sky { a, mode, small: true, local: false, tile: true }).width(Length::Fill).height(Length::Fill).into()
}

/// Where a note is on screen in the mode in use (for its hover card).
fn screen_of(a: &App, i: usize, w: f32, h: f32) -> Option<(f32, f32, f32)> {
    let s = Sky { a, mode: &a.g.mode, small: false, local: a.g.local, tile: false };
    s.place(w, h).into_iter().find(|q| q.i == i).map(|q| (q.x, q.y, q.r))
}

/// Flies the camera to a note: it's put in the middle and picked.
pub fn fly_to(a: &mut App, i: usize) {
    let (w, h) = LAST.lock().map(|l| *l).unwrap_or((900.0, 700.0));
    a.g.cam.x = 0.0;
    a.g.cam.y = 0.0;
    if let Some((x, y, _)) = screen_of(a, i, w, h) {
        let m = w.min(h);
        a.g.cam.x = (w / 2.0 - x) / m;
        a.g.cam.y = (h / 2.0 - y) / m;
    }
    a.g.sel = Some(i);
    a.g.hover = Some(i);
    a.g.flew = Some(i);
    a.g.burst_until = a.g.t + 3.0;
    a.wake();
}

// ------------------------------------------------------------------ the main window's pane (page 1)
pub fn pane(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let tabs = row![
        click(container(t1("✳ Graph view", f(15.0), if a.g.local { tn.muted } else { tn.text })).padding(pad(z(9.0), z(18.0), z(9.0), z(18.0))).style(bx(if a.g.local { Color::TRANSPARENT } else { tn.page }, 6.0, if a.g.local { None } else { Some((Pal::a(p.accent, 0.75), 1.0)) })), Some(Msg::GLocal(false))),
        click(container(t1("✳ Local graph", f(15.0), if a.g.local { tn.text } else { tn.muted })).padding(pad(z(9.0), z(18.0), z(9.0), z(18.0))).style(bx(if a.g.local { tn.page } else { Color::TRANSPARENT }, 6.0, if a.g.local { Some((Pal::a(p.accent, 0.75), 1.0)) } else { None })), Some(Msg::GLocal(true))),
        click(container(t("+", f(17.0), tn.muted)).padding(pad(z(6.0), z(14.0), z(8.0), z(14.0))), Some(Msg::Rail(Page::Modes))),
    ]
    .spacing(z(6.0))
    .align_y(iced::Bottom);
    let crumb = row![t1(format!("{} · {}", if a.g.local { "Local graph" } else { "Graph view" }, "Classic"), f(15.0), tn.muted), Space::new().width(Length::Fill), click(t("⚙", f(15.0), tn.muted), Some(Msg::GCustomise(true))), gap(z(18.0), 1.0), click(t("✦", f(15.0), tn.muted), Some(Msg::Rail(Page::Modes)))].align_y(iced::Center);
    let n = a.vault.notes.len();
    let foot = container(t1(format!("Follows the open note · {n} note{}", crate::plural(n)), f(14.0), Pal::mix(tn.text, tn.muted, 0.2))).padding(pad(z(6.0), z(14.0), z(6.0), z(14.0))).style(bx(Pal::mix(tn.page, tn.card, 0.5), 6.0, Some((tn.line, 1.0))));
    let canvas = stack![sky(a, "classic", true, a.g.local), container(foot).align_bottom(Length::Fill).padding(pad(0.0, 0.0, z(18.0), z(18.0)))];
    column![
        container(tabs).padding(pad(z(8.0), z(12.0), 0.0, z(12.0))).height(Length::Fixed(z(56.0))).align_bottom(Length::Fixed(z(56.0))),
        ui::hline(tn.line),
        container(crumb).padding(pad(z(6.0), z(22.0), z(6.0), z(26.0))).height(Length::Fixed(z(52.0))).center_y(Length::Fixed(z(52.0))),
        ui::hline(tn.line),
        container(canvas).height(Length::Fill).style(bx(bg_of("classic"), 0.0, None)),
    ]
    .into()
}

// ------------------------------------------------------------------ the graph page (3, 3c–3h)
pub fn page(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let mode = a.g.mode.clone();
    let body = responsive(move |size: Size| {
        let (w, h) = (size.width, size.height);
        let mut layers: Vec<E<Msg>> = vec![sky(a, &a.g.mode, false, a.g.local)];
        if a.g.mode == "classic" && a.gb("pictures", false) {
            for q in (Sky { a, mode: &a.g.mode, small: false, local: a.g.local, tile: false }).place(w, h) {
                if let Some(pic) = a.graph.nodes[q.i].picture.as_ref().and_then(|x| a.vault.resolve_file(x, &a.vault.notes[q.i].rel)) {
                    layers.push(pin(container(image(image::Handle::from_path(pic)).width(Length::Fixed(40.0)).height(Length::Fixed(25.0))).style(bx(Color::TRANSPARENT, 3.0, Some((Color::WHITE, 1.0))))).x(q.x - 20.0).y(q.y - 12.5).into());
                }
            }
        }
        layers.push(top_bar(a));
        if let Some(panel) = filters(a) {
            layers.push(container(panel).align_right(Length::Fill).padding(pad(z(92.0), z(28.0), 0.0, 0.0)).into());
        }
        if let Some(i) = a.g.hover.or(a.g.sel) {
            if let Some((x, y, r)) = screen_of(a, i, w, h) {
                let cw = z(420.0);
                let (mut cx, mut cy) = (x - cw - r - 20.0, y - z(30.0));
                if a.g.mode == "galaxy" || a.g.mode == "ember" || a.g.mode == "railway" || a.g.mode == "solar" {
                    cx = x - cw / 2.0;
                    cy = y + r + 14.0;
                }
                if cx < 10.0 {
                    cx = x + r + 20.0;
                }
                if cy + z(180.0) > h {
                    cy = y - z(200.0);
                }
                layers.push(pin(hover_card(a, i)).x(cx.clamp(10.0, (w - cw - 10.0).max(10.0))).y(cy.clamp(z(100.0), (h - z(170.0)).max(z(100.0)))).into());
            }
        }
        layers.push(container(legend(a)).align_bottom(Length::Fill).padding(pad(0.0, 0.0, z(24.0), z(26.0))).into());
        match a.g.mode.as_str() {
            "galaxy" | "solar" => layers.push(container(orbit(a)).align_right(Length::Fill).align_bottom(Length::Fill).padding(pad(0.0, z(28.0), z(24.0), 0.0)).into()),
            "blackhole" => {
                layers.push(container(controls(a)).center_x(Length::Fill).align_bottom(Length::Fill).padding(pad(0.0, 0.0, z(24.0), 0.0)).into());
                let back = a.open_index().map(|i| a.vault.backlinks(i).len()).unwrap_or(0);
                layers.push(container(container(t(format!("↩ {back} backlinks"), f(14.0), tn.text)).padding(pad(z(6.0), z(14.0), z(6.0), z(14.0))).style(bx(Color::from_rgba8(0x1a, 0x1c, 0x22, 0.9), 6.0, Some((tn.line, 1.0))))).align_right(Length::Fill).align_bottom(Length::Fill).padding(pad(0.0, z(28.0), z(28.0), 0.0)).into());
            }
            _ => {}
        }
        if !a.g.fly.is_empty() && !a.g.fly_closed {
            layers.push(pin(fly_list(a)).x(z(26.0)).y(z(80.0)).into());
        }
        let e: E<Msg> = iced::widget::Stack::with_children(layers).width(Length::Fill).height(Length::Fill).into();
        e
    });
    let _ = mode;
    let main: E<Msg> = container(body).width(Length::Fill).height(Length::Fill).style(bx(bg_of(&a.g.mode), 0.0, None)).into();
    if a.g.customise {
        row![main, ui::vline(tn.line), customise(a)].into()
    } else {
        main
    }
}

fn top_bar(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let ph = if a.g.mode == "classic" && !a.g.customise { "Find a note…" } else { "Fly to a note…" };
    let input = text_input(ph, &a.g.fly).id(FLY_ID).on_input(Msg::FlyText).size(f(17.0)).padding(pad(z(10.0), z(50.0), z(10.0), z(36.0))).style(look::field_style(&p, true));
    let field = container(stack![input, container(t("⌕", f(14.0), tn.muted)).center_y(Length::Fill).padding(pad(0.0, 0.0, 0.0, z(12.0))), container(container(t("/", f(13.0), tn.muted)).padding(pad(0.0, z(6.0), 0.0, z(6.0))).style(bx(Color::TRANSPARENT, 3.0, Some((tn.line, 1.0))))).align_right(Length::Fill).center_y(Length::Fill).padding(pad(0.0, z(12.0), 0.0, 0.0))]).width(Length::Fixed(if a.g.customise { z(350.0) } else { z(410.0) }));
    let mut bar = Row::new().spacing(2.0);
    let names: Vec<(&str, &str)> = if a.g.customise { vec![("classic", "Classic"), ("galaxy", "Galaxy"), ("solar", "Solar")] } else { BAR.to_vec() };
    for (id, nm) in names {
        let on = a.g.mode == id;
        bar = bar.push(click(container(if on { tb(nm, f(15.0), p.ink) } else { t(nm, f(15.0), Pal::mix(tn.text, tn.muted, 0.2)) }.wrapping(text::Wrapping::None)).padding(pad(z(7.0), z(15.0), z(7.0), z(15.0))).style(bx(if on { p.accent } else { Color::TRANSPARENT }, 5.0, None)), Some(Msg::GMode(id.to_string()))));
    }
    let more_on = !BAR.iter().any(|b| b.0 == a.g.mode) && !a.g.customise;
    let more = if more_on { format!("{} ▾", name_of(&a.g.mode)) } else { "More ▾".to_string() };
    bar = bar.push(click(container(if more_on { tb(more, f(15.0), p.ink) } else { t(more, f(15.0), Pal::mix(tn.text, tn.muted, 0.2)) }).padding(pad(z(7.0), z(15.0), z(7.0), z(15.0))).style(bx(if more_on { p.accent } else { Color::TRANSPARENT }, 5.0, None)), Some(Msg::Rail(Page::Modes))));
    let modes = container(bar).padding(z(4.0)).style(bx(Color::from_rgba8(0x1a, 0x1c, 0x22, 0.92), 8.0, Some((tn.line, 1.0))));
    let mut r = row![field, gap(z(16.0), 1.0), modes, Space::new().width(Length::Fill)].align_y(iced::Center);
    if a.g.mode == "classic" && !a.g.customise {
        let links = a.graph.edges.len();
        let n = a.graph.nodes.len();
        r = r.push(t1(format!("{n} notes · {links} links"), f(15.0), tn.muted));
    }
    if a.g.mode == "blackhole" {
        r = r.push(click(container(t1("⚲ Filters · projects, tags", f(15.0), tn.text)).padding(pad(z(9.0), z(16.0), z(9.0), z(16.0))).style(bx(Color::from_rgba8(0x1a, 0x1c, 0x22, 0.92), 6.0, Some((tn.line, 1.0)))), Some(Msg::GOpt("panel".into(), json!(!a.gb("panel", false))))));
    }
    container(r).padding(pad(z(22.0), z(28.0), 0.0, z(26.0))).width(Length::Fill).into()
}

fn fly_list(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let mut c = Column::new().spacing(z(4.0));
    for (k, (i, by_name)) in a.fly_list().into_iter().enumerate() {
        let on = k == a.g.fly_sel;
        let n = &a.vault.notes[i];
        let name = if n.rel.ends_with("/README.md") { n.rel.trim_end_matches("/README.md").to_string() } else { n.rel.clone() };
        let right = if by_name && k == 0 { "fly there" } else if by_name { "" } else { "mentions it" };
        let r = container(row![t1(if on { format!("✦ {name}") } else { name }, f(15.0), tn.text), Space::new().width(Length::Fill), t1(right, f(12.0), tn.muted)].align_y(iced::Center)).padding(pad(z(7.0), z(14.0), z(7.0), z(14.0))).width(Length::Fill).style(bx(if on { Pal::mix(tn.card, tn.text, 0.04) } else { Color::TRANSPARENT }, 6.0, if on { Some((p.accent, 2.0)) } else { None }));
        c = c.push(click(r, Some(Msg::FlyGo(i))));
    }
    container(c).padding(z(6.0)).width(Length::Fixed(z(410.0))).style(bx(Color::from_rgba8(0x1a, 0x1c, 0x22, 0.96), 8.0, Some((tn.line, 1.0)))).into()
}

/// The hover card: the note, a line about it in this mode, Open note and the mode's second button.
fn hover_card(a: &App, i: usize) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let n = &a.vault.notes[i];
    let nd = &a.graph.nodes[i];
    let name = if n.rel.ends_with("/README.md") { n.rel.trim_end_matches("/README.md").to_string() } else { n.rel.trim_end_matches(".md").to_string() };
    let nb: Vec<String> = a.graph.neighbours(i).into_iter().take(3).map(|j| { let x = &a.graph.nodes[j]; if x.project == "log" { format!("log {}", x.label) } else { x.label.clone() } }).collect();
    let opened = *a.opened.get(&n.rel).unwrap_or(&0);
    let status = md::properties(&n.text).into_iter().find(|x| x.0 == "Status").map(|x| x.1).unwrap_or_default();
    let status = status.split(['(', '·']).next().unwrap_or("").trim().to_string();
    let first = |s: &str| -> String { let mut c = s.chars(); c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default() };
    let list = if nb.is_empty() { String::new() } else { format!(": {}", nb.join(", ")) };
    let (glyph_, line, second): (&str, String, Option<(String, Msg)>) = match a.g.mode.as_str() {
        "solar" => ("◉", format!("A planet on the {} ring · opened {opened} time{} · its {} moon{}{list}", if nd.kind.is_empty() { "inner" } else { &nd.kind }, crate::plural(opened as usize), nd.links.min(3), crate::plural(nd.links.min(3))), Some(("Follow".into(), Msg::FlyGo(i)))),
        "blackhole" => ("✦", format!("In the {} cluster · pulled in by {} link{} · opened {opened} time{}", if nd.project.is_empty() { "top" } else { &nd.project }, nd.links, crate::plural(nd.links), crate::plural(opened as usize)), Some(("Fly here".into(), Msg::FlyGo(i)))),
        "ember" => {
            let ct = day::central(nd.mtime);
            let when = if day::log_day(nd.mtime, a.prefs.day_start) == a.today { format!("Made today, {}", day::hhmm(ct)) } else { format!("Changed {}", day::short(ct.day)) };
            ("✹", format!("{when} · {} link{} shimmer{}", nd.links, crate::plural(nd.links), if nb.is_empty() { String::new() } else { format!(" to {}", nb.join(", ")) }), None)
        }
        "railway" => {
            let line_name = lines(a).into_iter().find(|l| l.3.contains(&i)).map(|l| l.0).unwrap_or_else(|| "map".into());
            let change = a.graph.neighbours(i).into_iter().filter_map(|j| a.graph.nodes[j].log).max().map(|d| format!(" · change here for the Main Line at {}", day::short(d))).unwrap_or_default();
            ("●", format!("A station on {} · {}{change}", if line_name.starts_with("Main") { "the Main Line".to_string() } else { format!("{line_name}") }, n.rel.rsplit_once('/').map(|x| x.0).unwrap_or("").trim_end_matches(&format!("/{}", nd.label)).to_string()), Some(("Ride the line".into(), Msg::GClick(next_station(a, i)))))
        }
        "galaxy" => ("✦", format!("{}{} · {} link{}{}", if status.is_empty() { String::new() } else { format!("{} · ", first(&status)) }, n.title, nd.links, crate::plural(nd.links), if nb.is_empty() { String::new() } else { format!(" ({})", nb.join(", ")) }), None),
        _ => ("", format!("{}{} link{}{list}", if status.is_empty() { String::new() } else { format!("{} · ", first(&status)) }, nd.links, crate::plural(nd.links)), Some(("Show only its links".into(), Msg::GOnly(i)))),
    };
    let title = if glyph_.is_empty() { name } else { format!("{glyph_} {name}") };
    let mut btns = row![look::btn(&p, "Open note", look::B::Primary, false, false, Some(Msg::GOpen(i)))].spacing(z(8.0));
    if let Some((l, m)) = second {
        btns = btns.push(look::btn(&p, l, look::B::Normal, false, false, Some(m)));
    }
    container(column![tb(title, f(16.0), tn.text), t(line, f(14.0), tn.muted), btns].spacing(z(8.0))).padding(pad(z(14.0), z(18.0), z(14.0), z(18.0))).width(Length::Fixed(z(420.0))).style(bx(Color::from_rgba8(0x1b, 0x1d, 0x23, 0.97), 8.0, Some((tn.line, 1.0)))).into()
}

fn next_station(a: &App, i: usize) -> Option<usize> {
    let l = lines(a).into_iter().find(|l| l.3.contains(&i))?;
    let k = l.3.iter().position(|x| *x == i)?;
    l.3.get((k + 1) % l.3.len()).copied()
}

/// PROJECTS (checkboxes, with "no notes yet"), TAGS, and the mode's own section.
fn filters(a: &App) -> Option<E<'_, Msg>> {
    let p = a.p;
    let tn = tone(&p);
    if a.g.customise || (a.g.mode == "blackhole" && !a.gb("panel", false)) {
        return None;
    }
    let shown = a.shown_projects();
    let counts = a.vault.project_counts();
    let mut c = Column::new().spacing(z(6.0));
    if a.g.mode == "railway" {
        c = c.push(look::caps(&p, "Lines"));
        for (name, col, _, st) in lines(a) {
            c = c.push(row![container(Space::new()).width(Length::Fixed(z(26.0))).height(Length::Fixed(z(8.0))).style(bx(col, 3.0, None)), gap(z(12.0), 1.0), t1(name, f(15.0), tn.text), Space::new().width(Length::Fill), t1(st.len().to_string(), f(13.0), tn.muted)].align_y(iced::Center));
        }
        c = c.push(gap(1.0, z(6.0))).push(look::caps(&p, "Filter"));
        c = c.push(row![look::chip(&p, "All projects ▾", look::Chip::Tag), gap(z(8.0), 1.0), tag_add(a)].align_y(iced::Center));
    } else {
        c = c.push(look::caps(&p, "Projects"));
        for pj in PROJECTS.iter().copied().chain(["log"]) {
            let n = *counts.get(pj).unwrap_or(&0);
            let on = shown.contains(pj);
            let col = project_color(&p, pj);
            let name = if pj == "log" { "daily logs" } else { pj };
            let right = if n == 0 { "no notes yet".to_string() } else { n.to_string() };
            let r = row![ui::check(&p, on, "", false, Msg::GProject(pj.to_string())), look::dot(col, z(11.0)), gap(z(10.0), 1.0), t1(name, f(15.0), if n == 0 { tn.muted } else { tn.text }), Space::new().width(Length::Fill), t1(right, f(13.0), tn.muted)].align_y(iced::Center);
            c = c.push(r);
        }
        c = c.push(gap(1.0, z(4.0))).push(look::caps(&p, "Tags"));
        let mut tr = Row::new().spacing(z(8.0)).align_y(iced::Center);
        for tg in &a.g.tags {
            tr = tr.push(container(row![t1(format!("#{tg}"), f(13.0), accent2(&p)), gap(z(6.0), 1.0), click(t("✕", f(11.0), accent2(&p)), Some(Msg::GTagDrop(tg.clone())))].align_y(iced::Center)).padding(pad(z(3.0), z(10.0), z(3.0), z(10.0))).style(bx(Pal::a(p.accent, 0.16), 4.0, None)));
        }
        tr = tr.push(tag_add(a));
        c = c.push(tr);
        match a.g.mode.as_str() {
            "classic" => {
                c = c.push(gap(1.0, z(4.0))).push(look::caps(&p, "Show"));
                c = c.push(ui::check(&p, a.gb("orphans", true), "Orphans", false, Msg::GOpt("orphans".into(), json!(!a.gb("orphans", true)))));
                c = c.push(ui::check(&p, a.gb("indexLinks", false), "INDEX.md's links", false, Msg::GOpt("indexLinks".into(), json!(!a.gb("indexLinks", false)))));
                c = c.push(gap(1.0, z(4.0))).push(look::caps(&p, "Colour by"));
                let cb = a.gs("colourBy", "project");
                c = c.push(look::seg(&p, vec![("Project".into(), cb == "project", Msg::GOpt("colourBy".into(), json!("project"))), ("Kind".into(), cb == "kind", Msg::GOpt("colourBy".into(), json!("kind"))), ("Status".into(), cb == "status", Msg::GOpt("colourBy".into(), json!("status")))], false));
                c = c.push(gap(1.0, z(4.0))).push(look::caps(&p, "Link distance"));
                c = c.push(iced::widget::slider(0.2..=1.0, a.gf("linkDistance", 0.75), |v| Msg::GOpt("linkDistance".into(), json!((v * 100.0).round() / 100.0))).step(0.01).style(look::slider_style(&p)));
            }
            "galaxy" => {
                c = c.push(gap(1.0, z(4.0))).push(look::caps(&p, "Nebulae"));
                c = c.push(ui::check(&p, a.gb("nebulae", true), "Colour clouds per project", false, Msg::GOpt("nebulae".into(), json!(!a.gb("nebulae", true)))));
                c = c.push(ui::check(&p, a.gb("trails", true), "Light trails for links", false, Msg::GOpt("trails".into(), json!(!a.gb("trails", true)))));
            }
            "solar" => {
                c = c.push(gap(1.0, z(4.0))).push(look::caps(&p, "Sizes"));
                let sb = a.gs("sizeBy", "opened");
                c = c.push(look::seg(&p, vec![("Times opened".into(), sb == "opened", Msg::GOpt("sizeBy".into(), json!("opened"))), ("Links".into(), sb == "links", Msg::GOpt("sizeBy".into(), json!("links"))), ("Words".into(), sb == "words", Msg::GOpt("sizeBy".into(), json!("words")))], false));
                c = c.push(ui::check(&p, a.gb("comets", true), "Logs as comets", false, Msg::GOpt("comets".into(), json!(!a.gb("comets", true)))));
            }
            "ember" => {
                c = c.push(gap(1.0, z(4.0))).push(look::caps(&p, "Heat means"));
                let hm = a.gs("heat", "changed");
                c = c.push(look::seg(&p, vec![("Changed lately".into(), hm == "changed", Msg::GOpt("heat".into(), json!("changed"))), ("Opened lately".into(), hm == "opened", Msg::GOpt("heat".into(), json!("opened")))], false));
                c = c.push(ui::check(&p, a.gb("sparks", true), "Sparks when a note opens", false, Msg::GOpt("sparks".into(), json!(!a.gb("sparks", true)))));
            }
            _ => {}
        }
    }
    Some(container(c).padding(pad(z(16.0), z(20.0), z(16.0), z(20.0))).width(Length::Fixed(z(400.0))).style(bx(Color::from_rgba8(0x17, 0x19, 0x1e, 0.94), 10.0, Some((tn.line, 1.0)))).into())
}

fn tag_add(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    match &a.g.tag_typing {
        Some(s) => text_input("tag", s).id("notes-graph-tag").on_input(Msg::GTagText).on_submit(Msg::GTagDone).size(f(13.0)).padding(pad(z(3.0), z(8.0), z(3.0), z(8.0))).width(Length::Fixed(z(110.0))).style(look::field_style(&p, true)).into(),
        None => click(container(t1("+ tag", f(13.0), tn.muted)).padding(pad(z(3.0), z(10.0), z(3.0), z(10.0))).style(bx(Color::TRANSPARENT, 4.0, Some((tn.line, 1.0)))), Some(Msg::GTagAdd)),
    }
}

fn legend(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let counts = a.vault.project_counts();
    let empty: Vec<&str> = PROJECTS.iter().copied().filter(|pj| *counts.get(*pj).unwrap_or(&0) == 0).collect();
    let mut r = Row::new().spacing(z(14.0)).align_y(iced::Center);
    match a.g.mode.as_str() {
        "classic" | "galaxy" => {
            for pj in PROJECTS.iter().copied().filter(|pj| *counts.get(*pj).unwrap_or(&0) > 0).chain(["log"]) {
                r = r.push(row![look::dot(project_color(&p, pj), z(10.0)), gap(z(8.0), 1.0), t1(if pj == "log" { "daily logs" } else { pj }, f(14.0), tn.text)].align_y(iced::Center));
            }
            if !empty.is_empty() {
                r = r.push(t1(format!("{}: no notes yet", empty.join(", ")), f(14.0), tn.muted));
            }
        }
        "solar" => {
            for s in ["☀ a project", "◯ a kind's ring", "● a note: bigger = opened more", "· its moons: notes it links to", "⁄ daily logs pass by as comets"] {
                r = r.push(t1(s, f(14.0), tn.text));
            }
        }
        "ember" => {
            r = r.push(row![look::dot(Color::from_rgb8(0xff, 0xd2, 0x60), z(10.0)), gap(z(8.0), 1.0), t1("low and bright = made or changed lately", f(14.0), tn.text)].align_y(iced::Center));
            r = r.push(row![look::dot(Color::from_rgb8(0xd8, 0x5a, 0x1c), z(10.0)), gap(z(8.0), 1.0), t1("higher up = older, cooling", f(14.0), tn.text)].align_y(iced::Center));
            r = r.push(t1("〰 heat shimmer = a link", f(14.0), tn.text));
        }
        "blackhole" => {
            return column![t1("Home = INDEX.md · each cloud is a project, its links pulled toward Home", f(13.0), tn.muted), t1("Orphans (no links) drift at the edge", f(13.0), tn.muted)].spacing(2.0).into();
        }
        "railway" => return gap(1.0, 1.0),
        m => {
            r = r.push(t1(MODES.iter().find(|x| x.0 == m).map(|x| x.2).unwrap_or(""), f(14.0), tn.text));
        }
    }
    container(r).padding(pad(z(8.0), z(16.0), z(8.0), z(16.0))).style(bx(Color::from_rgba8(0x17, 0x19, 0x1e, 0.92), 6.0, Some((tn.line, 1.0)))).into()
}

/// Galaxy's and Solar system's camera control: the turn dial, the zoom, Reset view.
fn orbit(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    struct Dial(f32, Color);
    impl canvas::Program<Msg> for Dial {
        type State = ();
        fn draw(&self, _s: &(), r: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
            let mut fr = Frame::new(r, b.size());
            let c = Point::new(b.width / 2.0, b.height / 2.0);
            let rad = b.width.min(b.height) / 2.0 - 2.0;
            fr.stroke(&Path::circle(c, rad), Stroke::default().with_color(Color::from_rgba8(0xff, 0xff, 0xff, 0.35)).with_width(1.5));
            fr.fill(&Path::circle(Point::new(c.x + (self.0 - PI / 2.0).cos() * rad, c.y + (self.0 - PI / 2.0).sin() * rad), 3.5), self.1);
            vec![fr.into_geometry()]
        }
    }
    let dial = canvas::Canvas::new(Dial(a.g.cam.yaw, p.accent)).width(Length::Fixed(z(58.0))).height(Length::Fixed(z(58.0)));
    let words = column![row![t("Turn: drag or ", f(14.0), tn.text), tb("LS", f(14.0), tn.text)], row![t("Zoom: scroll or ", f(14.0), tn.text), tb("RS", f(14.0), tn.text)]].spacing(2.0);
    let mut r = row![dial, gap(z(14.0), 1.0), words].align_y(iced::Center);
    if a.g.mode == "galaxy" {
        let zoom = column![iced::widget::slider(0.3..=4.0, a.g.cam.zoom.clamp(0.3, 4.0), Msg::GZoomTo).step(0.05).width(Length::Fixed(z(130.0))).style(look::slider_style(&p)), t(format!("Zoom {:.1}×", a.g.cam.zoom), f(12.0), tn.muted)].spacing(2.0);
        r = r.push(gap(z(16.0), 1.0)).push(zoom);
    }
    r = r.push(gap(z(16.0), 1.0)).push(look::btn(&p, "Reset view", look::B::Normal, false, false, Some(Msg::GReset)));
    container(r).padding(pad(z(12.0), z(16.0), z(12.0), z(16.0))).style(bx(Color::from_rgba8(0x17, 0x19, 0x1e, 0.94), 10.0, Some((tn.line, 1.0)))).into()
}

/// Black hole's floating pill: zoom out, zoom in, reset, pause, fullscreen.
fn controls(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let b = |s: &'static str, m: Msg| -> E<'static, Msg> { click(container(t(s, f(16.0), tn.text)).center_x(Length::Fixed(z(50.0))).center_y(Length::Fixed(z(40.0))), Some(m)) };
    let r = row![b("−", Msg::GZoom(1.0 / 1.2)), b("+", Msg::GZoom(1.2)), b("⟲", Msg::GReset), b(if a.g.paused { "▶" } else { "⏸" }, Msg::GPause), b("⛶", Msg::GFull)].align_y(iced::Center);
    container(r).padding(pad(z(4.0), z(12.0), z(4.0), z(12.0))).style(bx(Color::from_rgba8(0x1a, 0x1c, 0x22, 0.95), z(24.0), Some((tn.line, 1.0)))).into()
}

pub fn hints(a: &App) -> (Vec<E<'_, Msg>>, Vec<E<'_, Msg>>) {
    let p = a.p;
    use look::{hint, hint2, hint_words};
    if a.g.customise {
        return (vec![hint(&p, "A", "Change"), hint(&p, "B", "Close panel"), hint(&p, "Y", "Switch mode")], vec![hint2(&p, "LB", "RB", "Previous / next section"), hint_words(&p, "Saved for this mode · nothing changes by itself")]);
    }
    let x = if a.g.mode == "classic" || a.g.mode == "galaxy" { "Find" } else { "Fly to" };
    let left = vec![hint(&p, "A", "Open note"), hint(&p, "Y", "Switch mode"), hint(&p, "X", x), hint(&p, "B", "Back")];
    let right = match a.g.mode.as_str() {
        "classic" => vec![hint(&p, "LS", "Move"), hint(&p, "RS", "Zoom")],
        "galaxy" => vec![hint(&p, "LS", "Orbit"), hint(&p, "RS", "Zoom"), hint2(&p, "LB", "RB", "Next / previous star")],
        "solar" => vec![hint(&p, "LS", "Orbit"), hint(&p, "RS", "Zoom"), hint2(&p, "LB", "RB", "Next / previous sun")],
        "blackhole" => vec![hint(&p, "LS", "Orbit"), hint(&p, "RS", "Zoom"), hint2(&p, "LB", "RB", "Next / previous cluster")],
        "railway" => vec![hint(&p, "LS", "Move"), hint(&p, "RS", "Zoom"), hint2(&p, "LB", "RB", "Next / previous station")],
        _ => vec![hint(&p, "LS", "Orbit"), hint(&p, "RS", "Zoom")],
    };
    (left, right)
}

// ------------------------------------------------------------------ Graph modes (3b)
pub fn modes(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let head = row![tb("Graph modes", f(40.0), tn.text), gap(z(20.0), 1.0), t("Same notes and links in every mode, with labels, Fly to, filters and Open note.", f(19.0), tn.muted)].align_y(iced::Center);
    let mut grid = Column::new().spacing(z(16.0));
    for rowi in 0..2 {
        let mut r = Row::new().spacing(z(16.0));
        for k in 0..5 {
            let idx = rowi * 5 + k;
            let (id, name, desc) = MODES[idx];
            let focused = idx == a.g.picker_focus;
            let in_use = a.g.mode == id;
            let mut title = row![ts(name, f(19.0), tn.text)].align_y(iced::Center).spacing(z(10.0));
            if focused {
                title = title.push(Space::new().width(Length::Fill)).push(look::chip(&p, "A · Use", look::Chip::Outline));
            } else if in_use {
                title = title.push(look::chip(&p, "In use", look::Chip::Plain));
            }
            let card = column![container(tile(a, id)).height(Length::Fixed(z(190.0))).width(Length::Fill).clip(true), container(column![title, t(desc, f(14.0), tn.muted)].spacing(z(6.0))).padding(pad(z(12.0), z(16.0), z(14.0), z(16.0)))];
            let boxed = container(card).width(Length::Fill).style(bx(tn.card, 10.0, None)).clip(true);
            let el = mouse_area(look::ring(&p, focused, 10.0, boxed)).on_enter(Msg::GPickFocus(idx)).on_press(Msg::GMode(id.to_string())).interaction(mouse::Interaction::Pointer);
            r = r.push(container(el).width(Length::FillPortion(1)));
        }
        grid = grid.push(r);
    }
    let rm = if a.reduce_motion { "On · from Hearth" } else { "Off · from Hearth" };
    let info = |icon: &'static str, title: &'static str, line: &'static str, end: E<'static, Msg>| -> E<'static, Msg> {
        container(row![look::sym(&p, icon), gap(z(16.0), 1.0), column![t(title, f(19.0), tn.text), t(line, f(14.0), tn.muted)].spacing(z(4.0)).width(Length::Fill), end].align_y(iced::Center)).padding(pad(z(16.0), z(22.0), z(16.0), z(22.0))).width(Length::Fill).style(bx(tn.card, 10.0, None)).into()
    };
    let bottom = row![
        info("◎", "Reduce motion", "Follows Hearth's setting: every mode is drawn still", look::chip(&p, rm, look::Chip::Plain)),
        info("✎", "Customise", "Node pictures and shapes, links, effects, saved looks", look::btn(&p, "Customise ›", look::B::Normal, false, false, Some(Msg::GCustomise(true)))),
        info("⏸", "Stays light", "Stops drawing when the window isn't visible or a game starts", look::chip(&p, "Always", look::Chip::Ok)),
    ]
    .spacing(z(16.0));
    let page_: E<Msg> = scrollable(container(column![head, grid, bottom].spacing(z(24.0))).padding(pad(z(28.0), z(40.0), z(28.0), z(40.0)))).into();
    if a.g.customise {
        row![container(page_).width(Length::Fill), ui::vline(tn.line), customise(a)].into()
    } else {
        page_
    }
}

// ------------------------------------------------------------------ Customise the graph (3f)
fn opt_row<'a>(a: &'a App, label: &str, end: E<'a, Msg>) -> E<'a, Msg> {
    let tn = tone(&a.p);
    row![t1(label.to_string(), f(15.5), tn.text), Space::new().width(Length::Fill), end].align_y(iced::Center).height(Length::Fixed(z(44.0))).into()
}

fn toggle<'a>(a: &'a App, k: &str, d: bool) -> E<'a, Msg> {
    let on = a.gb(k, d);
    look::switch(&a.p, on, false, Some(Msg::GOpt(k.to_string(), json!(!on))))
}

fn segk<'a>(a: &'a App, k: &str, d: &str, items: &[(&str, &str)]) -> E<'a, Msg> {
    let cur = a.gs(k, d);
    look::seg(&a.p, items.iter().map(|(id, l)| (l.to_string(), cur == *id, Msg::GOpt(k.to_string(), json!(id)))).collect(), false)
}

fn slide<'a>(a: &'a App, k: &str, d: f32) -> E<'a, Msg> {
    let key = k.to_string();
    iced::widget::slider(0.0..=1.0, a.gf(k, d), move |v| Msg::GOpt(key.clone(), json!((v * 100.0).round() / 100.0))).step(0.01).width(Length::Fixed(z(170.0))).style(look::slider_style(&a.p)).into()
}

pub const LOOKS: [&str; 5] = ["Galaxy night", "Solar system", "Ember", "Minimal", "Neon"];

/// The saved looks: (mode, options).
pub fn look_def(name: &str) -> (&'static str, Value) {
    match name {
        "Galaxy night" => ("galaxy", json!({"nebulae": true, "trails": true, "bloom": 0.8})),
        "Solar system" => ("solar", json!({"sizeBy": "opened", "comets": true})),
        "Ember" => ("ember", json!({"heat": "changed", "sparks": true})),
        "Minimal" => ("classic", json!({"glow": false, "bloom": 0.0, "colourByType": false, "starfield": false})),
        _ => ("classic", json!({"glow": true, "bloom": 1.0, "starfield": true, "particles": true})),
    }
}

fn customise(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let head = row![tb("Customise the graph", f(24.0), tn.text), gap(z(20.0), 1.0), look::chip(&p, name_of(&a.g.mode), look::Chip::Plain), Space::new().width(Length::Fill), click(t("✕", f(16.0), tn.muted), Some(Msg::GCustomise(false)))].align_y(iced::Center);
    let shapes_line = row![t("▭ mockups ● assets ▲ videos ◆ notes ⬡ research", f(13.0), tn.muted)];
    let left = column![
        look::caps_accent(&p, "Nodes"),
        opt_row(a, "A note's own picture or icon as its node", toggle(a, "pictures", false)),
        opt_row(a, "Shapes by", segk(a, "shapesBy", "kind", &[("kind", "Kind"), ("tag", "Tag"), ("folder", "Folder")])),
        shapes_line,
        opt_row(a, "Colour groups", segk(a, "colourBy", "project", &[("project", "Project"), ("tag", "Tag"), ("folder", "Folder")])),
        opt_row(a, "Size by", segk(a, "sizeBy", "links", &[("links", "Links"), ("length", "Length"), ("opened", "Opened")])),
        look::caps_accent(&p, "Links"),
        opt_row(a, "Style", segk(a, "linkStyle", "straight", &[("curved", "Curved"), ("straight", "Straight")])),
        opt_row(a, "Arrows", toggle(a, "arrows", false)),
        opt_row(a, "Glow", toggle(a, "glow", true)),
        opt_row(a, "Colour by type", row![look::dot(p.accent, z(9.0)), t1(" log mentions ", f(13.0), accent2(&p)), look::dot(Color::WHITE, z(9.0)), t1(" [[links]] ", f(13.0), tn.text), look::dot(look::BLUE, z(9.0)), t1(" embeds", f(13.0), look::BLUE)].align_y(iced::Center).into()),
        opt_row(a, "Hide orphans", look::switch(&p, !a.gb("orphans", true), false, Some(Msg::GOpt("orphans".into(), json!(!a.gb("orphans", true)))))),
        opt_row(a, "Hide attachments (.png, .py, .html)", toggle(a, "hideAttachments", true)),
        look::caps_accent(&p, "Effects"),
        opt_row(a, "Bloom", slide(a, "bloom", 0.7)),
        opt_row(a, "Starfield behind", toggle(a, "starfield", false)),
        opt_row(a, "Particles along links when a note opens", toggle(a, "particles", true)),
        opt_row(a, "Label size", slide(a, "labelSize", 0.5)),
        opt_row(a, "Labels fade out when zoomed out", toggle(a, "labelsFade", true)),
    ]
    .spacing(z(4.0))
    .width(Length::FillPortion(1));
    // Insight: the vault's own.
    let ml: Vec<String> = a.graph.most_linked(3).into_iter().map(|(i, n)| format!("{} ({n})", a.graph.nodes[i].label)).collect();
    let orph = a.graph.orphans();
    let orph_names: Vec<String> = orph.iter().take(2).map(|i| a.graph.nodes[*i].label.clone()).collect();
    let cl: Vec<String> = a.graph.clusters().into_iter().take(3).map(|(n, c)| format!("{n} ({c})")).collect();
    let insight = column![
        row![tb("Most linked: ", f(14.0), tn.text), t(ml.join(", "), f(14.0), tn.muted)].wrap(),
        row![tb(format!("Orphans ({}): ", orph.len()), f(14.0), tn.text), t(orph_names.join(", "), f(14.0), tn.muted)].wrap(),
        row![tb("Clusters: ", f(14.0), tn.text), t(cl.join(" · "), f(14.0), tn.muted)].wrap(),
    ]
    .spacing(z(4.0));
    let mut looks = Row::new().spacing(z(8.0));
    let current = a.gs("look", "");
    for nm in LOOKS {
        let (mode, _) = look_def(nm);
        let on = current == nm || (current.is_empty() && nm == "Ember" && a.g.mode == "ember");
        let prev = container(tile(a, mode)).width(Length::Fill).height(Length::Fixed(z(54.0))).clip(true);
        let tile = container(column![prev, container(t(nm, f(12.0), tn.text)).center_x(Length::Fill)].spacing(z(4.0))).padding(z(4.0)).width(Length::FillPortion(1)).style(bx(tn.card2, 6.0, Some((if on { p.accent } else { tn.line }, if on { 2.0 } else { 1.0 }))));
        looks = looks.push(click(tile, Some(Msg::LookApply(nm.to_string()))));
    }
    for (nm, _) in a.prefs.looks.iter().take(2) {
        looks = looks.push(click(container(t(nm.clone(), f(12.0), tn.text)).padding(z(8.0)).style(bx(tn.card2, 6.0, Some((tn.line, 1.0)))), Some(Msg::LookApply(nm.clone()))));
    }
    let look_btns = row![look::btn(&p, "+ Save this look", look::B::Normal, false, false, Some(Msg::LookSave)), look::btn(&p, "Share as a file…", look::B::Normal, false, false, Some(Msg::LookShare)), look::btn(&p, "Open…", look::B::Normal, false, false, Some(Msg::LookOpen))].spacing(z(8.0));
    let open = a.g.sel.or_else(|| a.open_index());
    let nmap: E<Msg> = match open {
        Some(i) => neighbourhood(a, i),
        None => t("Open a note to see its neighbourhood", f(14.0), tn.muted).into(),
    };
    let mut per = Column::new().spacing(z(6.0));
    for pj in ["hearth", "other", "beacon", "quad-mech"] {
        let cur = a.prefs.project_looks.get(pj).and_then(Value::as_str).unwrap_or(match pj { "hearth" => "Ember", "other" => "Neon", "beacon" => "Galaxy night", _ => "Minimal" }).to_string();
        let open_menu = a.g.look_menu.as_deref() == Some(pj);
        let mut r = Column::new().push(row![look::dot(project_color(&p, pj), z(10.0)), gap(z(10.0), 1.0), t1(pj, f(15.0), tn.text), Space::new().width(Length::Fill), click(container(t1(format!("{cur} ▾"), f(13.0), Pal::mix(tn.text, tn.muted, 0.15))).padding(pad(z(3.0), z(10.0), z(3.0), z(10.0))).style(bx(Color::TRANSPARENT, z(12.0), Some((tn.line, 1.0)))), Some(Msg::LookMenu(if open_menu { None } else { Some(pj.to_string()) })))].align_y(iced::Center));
        if open_menu {
            let mut m = Row::new().spacing(z(6.0));
            for nm in LOOKS {
                m = m.push(click(look::chip(&p, nm, if nm == cur { look::Chip::Outline } else { look::Chip::Plain }), Some(Msg::ProjectLook(pj.to_string(), nm.to_string()))));
            }
            r = r.push(m.wrap());
        }
        per = per.push(r);
    }
    let right = column![
        look::caps_accent(&p, "Insight"),
        insight,
        look::caps_accent(&p, "Saved looks"),
        looks,
        look_btns,
        look::caps_accent(&p, "Neighbourhood view"),
        nmap,
        row![t("Around the open note, as a mind map", f(14.0), tn.text).width(Length::Fill), look::btn(&p, "Open as a pane", look::B::Normal, false, false, Some(Msg::GLocal(true)))].align_y(iced::Center),
        look::caps_accent(&p, "Per-project looks"),
        per,
        t("Stays light: stops drawing when hidden. Reduce motion: everything still.", f(12.5), tn.faint),
    ]
    .spacing(z(10.0))
    .width(Length::FillPortion(1));
    let body = column![head, row![left, gap(z(24.0), 1.0), right]].spacing(z(14.0));
    container(scrollable(container(body).padding(pad(z(20.0), z(26.0), z(20.0), z(26.0))))).width(Length::Fixed(z(860.0))).height(Length::Fill).style(bx(tn.side, 0.0, None)).into()
}

/// The open note's neighbourhood as a small mind map: parents left, children right, siblings below.
fn neighbourhood(a: &App, i: usize) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    struct Map<'b> {
        a: &'b App,
        i: usize,
    }
    impl<'b> canvas::Program<Msg> for Map<'b> {
        type State = ();
        fn draw(&self, _s: &(), r: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
            let a = self.a;
            let mut fr = Frame::new(r, b.size());
            let (parents, children, siblings) = a.graph.neighbourhood(&a.vault, self.i);
            let c = Point::new(b.width * 0.5, b.height * 0.42);
            let pill = |fr: &mut Frame, at: Point, s: &str, main: bool| {
                let size = if main { 12.0 } else { 10.5 };
                let w = s.chars().count() as f32 * size * 0.56 + 16.0;
                fr.fill(&Path::rounded_rectangle(Point::new(at.x - w / 2.0, at.y - 10.0), Size::new(w, 20.0), 10.0.into()), if main { a.p.accent } else { Color::from_rgb8(0x2a, 0x2d, 0x34) });
                label(fr, s, at.x, at.y - size * 0.62, size, if main { a.p.ink } else { Color::from_rgb8(0xe0, 0xe0, 0xe4) }, true);
            };
            let name = |j: usize| -> String { let n = &a.graph.nodes[j]; if n.project == "log" { format!("log {}", n.label) } else { n.label.clone() } };
            let place = |k: usize, n: usize, x: f32, spread: f32| -> Point { Point::new(x, c.y + (k as f32 - (n as f32 - 1.0) / 2.0) * spread) };
            let ps: Vec<usize> = parents.into_iter().take(3).collect();
            let cs: Vec<usize> = children.into_iter().take(3).collect();
            for (k, j) in ps.iter().enumerate() {
                let at = place(k, ps.len(), b.width * 0.17, 30.0);
                fr.stroke(&Path::line(at, c), Stroke::default().with_color(Pal::a(a.p.accent, 0.8)).with_width(1.4));
                pill(&mut fr, at, &name(*j), false);
            }
            for (k, j) in cs.iter().enumerate() {
                let at = place(k, cs.len(), b.width * 0.83, 30.0);
                fr.stroke(&Path::line(c, at), Stroke::default().with_color(Color::from_rgba8(0x8f, 0xd0, 0x8a, 0.8)).with_width(1.4));
                pill(&mut fr, at, &name(*j), false);
            }
            for (k, j) in siblings.into_iter().take(2).enumerate() {
                let at = Point::new(b.width * (0.30 + k as f32 * 0.32), b.height * 0.80);
                fr.stroke(&Path::line(c, at), Stroke::default().with_color(Color::from_rgba8(0xff, 0xff, 0xff, 0.25)).with_width(1.0));
                pill(&mut fr, at, &name(j), false);
            }
            pill(&mut fr, c, &name(self.i), true);
            label(&mut fr, "parents · children · siblings", 8.0, b.height - 14.0, 9.5, Color::from_rgba8(0xa0, 0xa4, 0xae, 0.9), false);
            vec![fr.into_geometry()]
        }
    }
    container(canvas::Canvas::new(Map { a, i }).width(Length::Fill).height(Length::Fixed(z(200.0)))).style(bx(tn.card, 8.0, Some((tn.line, 1.0)))).into()
}

pub fn look_apply(a: &mut App, name: &str) -> Task<Msg> {
    let (mode, opts) = match a.prefs.looks.get(name) {
        Some(v) => (v.get("mode").and_then(Value::as_str).unwrap_or("classic").to_string(), v.get("options").cloned().unwrap_or(json!({}))),
        None => {
            let (m, o) = look_def(name);
            (m.to_string(), o)
        }
    };
    a.g.mode = mode.clone();
    if let Some(o) = opts.as_object() {
        for (k, v) in o {
            a.set_graph(k, v.clone());
        }
    }
    a.set_graph("look", json!(name));
    if let Err(e) = hearth_notes::prefs::set("graphMode", json!(mode)).map(|p| a.prefs = p) {
        a.say(e, true);
    }
    a.wake();
    Task::none()
}

fn current_options(a: &App) -> Value {
    let pre = format!("{}.", a.g.mode);
    let m: serde_json::Map<String, Value> = a.prefs.graph.iter().filter_map(|(k, v)| k.strip_prefix(&pre).map(|k2| (k2.to_string(), v.clone()))).collect();
    json!({"mode": a.g.mode, "options": Value::Object(m)})
}

pub fn look_save(a: &mut App) -> Task<Msg> {
    let n = a.prefs.looks.len() + 1;
    let name = format!("My look {n}");
    match hearth_notes::prefs::set_in("looks", &name, current_options(a)) {
        Ok(p) => {
            a.prefs = p;
            a.say(format!("Saved as \"{name}\""), false);
        }
        Err(e) => a.say(e, true),
    }
    Task::none()
}

/// "Share as a file…": the look as a small JSON file in Documents (or the home folder).
pub fn look_share(a: &mut App) -> Task<Msg> {
    let home = hearth_notes::paths::home();
    let dir = if home.join("Documents").is_dir() { home.join("Documents") } else { home };
    let path = dir.join(format!("hearth-notes-look-{}.json", a.g.mode));
    let v = current_options(a);
    match hearth_notes::save::atomic(&path, format!("{}\n", serde_json::to_string_pretty(&v).unwrap_or_default()).as_bytes()) {
        Ok(()) => a.say(format!("Saved {}", hearth_notes::paths::tilde(&path)), false),
        Err(e) => a.say(format!("Couldn't save the look: {e}"), true),
    }
    Task::none()
}

pub fn look_open(_a: &mut App) -> Task<Msg> {
    Task::perform(
        crate::off(|| {
            for cmd in [vec!["hearth-file-chooser", "--open"], vec!["kdialog", "--getopenfilename", "", "*.json"], vec!["zenity", "--file-selection"]] {
                let o = hearth_notes::git::run(std::process::Command::new(cmd[0]).args(&cmd[1..]), 600);
                if o.ok && !o.out.trim().is_empty() {
                    return Some(o.out.trim().to_string());
                }
            }
            None
        }),
        |r| Msg::LookFile(r.flatten()),
    )
}

pub fn look_file(a: &mut App, p: Option<String>) -> Task<Msg> {
    let Some(p) = p else { return Task::none() };
    let v: Option<Value> = std::fs::read(&p).ok().and_then(|b| serde_json::from_slice(&b).ok());
    match v.filter(|v| v.get("mode").is_some()) {
        Some(v) => {
            let name = std::path::Path::new(&p).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "Opened look".into());
            match hearth_notes::prefs::set_in("looks", &name, v) {
                Ok(pr) => {
                    a.prefs = pr;
                    return look_apply(a, &name);
                }
                Err(e) => a.say(e, true),
            }
        }
        None => a.say("That file isn't a Hearth Notes look.", true),
    }
    Task::none()
}
