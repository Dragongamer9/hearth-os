//! Your PC: the board or device drawn from `hearth-resmon pc`'s view (Mantel's ResmonBoard.qml and the details
//! panel of ResourceMonitorView.qml). Everything about where things go comes from the helper (parts with x, y, w, h,
//! style, the hub each one is wired to and its lanes); this only paints it.
//!
//! One canvas with a cached drawing: the board, its holes and capacitors, the silkscreen, the wires (one line per
//! lane), the parts and their labels, the selection and the lightning bolts on busy links. It's drawn again only when
//! the drawing, the selection or which links are busy changes. The bolts hold still (as Mantel's do with Reduce
//! motion): nothing moves at rest, so the window costs nothing while you read it.

use std::cell::Cell as StdCell;
use std::collections::HashSet;

use hearth_ui::ui::{self, btn, gap, t, t1, ts, Kind};
use hearth_ui::{run, Pal};
use iced::widget::canvas::{self, Cache, Event, Frame, Geometry, LineCap, LineDash, LineJoin, Path, Stroke, Text};
use iced::widget::{column, container, mouse_area, row, scrollable, text, Column, Row, Space};
use iced::{mouse, Color, Element, Length, Padding, Point, Rectangle, Renderer, Size, Theme, Vector};
use serde_json::Value;

use crate::view::{col, pct, rate, size, thousands};
use crate::{arr, f, fo, s, App, Msg, Tab};

type E<'a> = Element<'a, Msg>;

fn hex(h: &str) -> Color {
    hearth_ui::theme::hex(h).unwrap_or(Color::from_rgb8(0x9f, 0xb2, 0xc3))
}

pub fn color_of(kind: &str) -> Color {
    hex(match kind {
        "cpu" | "disp" => "#66c0f4",
        "ram" => "#9a7cff",
        "gpu" => "#ff9a4d",
        "drive" => "#7ccf8a",
        "usb" => "#c9d6e2",
        "net" | "power" => "#f2c14e",
        "audio" => "#e0607e",
        "fan" | "chip" => "#3fb8c9",
        "input" => "#7fe0a4",
        _ => "#9fb2c3",
    })
}

fn round_style(st: &str) -> bool {
    matches!(st, "batt" | "fan" | "stick" | "btn")
}

const LIGHT_TEXT: [&str; 10] = ["cpu", "batt", "ioMetal", "ioUsbc", "pwr", "pciWhite", "ioDvi", "pcieArmor", "pcieGpu", "conn"];

// ------------------------------------------------------------------ what's live on the board
fn gpu_for<'a>(a: &'a App, key: &str) -> Option<&'a Value> {
    let d = a.sample.as_deref()?;
    let gs = arr(d, "gpus");
    let addr = key.trim_start_matches("gpu:");
    let pg = a.pc.as_ref().map(|pc| arr(pc, "gpus")).unwrap_or(&[]).iter().find(|g| s(g, "pci") == addr);
    gs.iter()
        .find(|g| {
            let pci = s(g, "pci");
            (!pci.is_empty() && addr.ends_with(&pci[pci.len().saturating_sub(7)..])) || pg.map(|p| !s(g, "card").is_empty() && s(g, "card") == s(p, "card")).unwrap_or(false)
        })
        .or_else(|| gs.iter().find(|g| pg.map(|p| s(g, "vendor") == s(p, "vendor")).unwrap_or(false)))
}

/// The busy parts (Mantel's updateLive).
pub fn busy_parts(a: &App) -> HashSet<String> {
    let mut out = HashSet::new();
    let (Some(vw), Some(d)) = (a.pc_view.as_ref(), a.sample.as_deref()) else { return out };
    let cpu_t = d.get("cpu").map(|c| f(c, "total")).unwrap_or(0.0);
    let mut chip_busy = false;
    for p in arr(vw, "parts") {
        let k = s(p, "live");
        let b = if k == "cpu" {
            cpu_t > 10.0
        } else if k == "mem" {
            cpu_t > 5.0
        } else if k.starts_with("gpu:") {
            gpu_for(a, k).map(|g| f(g, "busy") > 10.0).unwrap_or(false)
        } else if let Some(n) = k.strip_prefix("disk:") {
            let x = arr(d, "disks").iter().find(|x| s(x, "name") == n).map(|x| f(x, "active") > 3.0 || f(x, "read") + f(x, "write") > 1048576.0).unwrap_or(false);
            if x && s(p, "hub") == "chip" {
                chip_busy = true;
            }
            x
        } else if let Some(n) = k.strip_prefix("net:") {
            let x = arr(d, "nets").iter().find(|x| s(x, "name") == n).map(|x| f(x, "recv") + f(x, "send") > 51200.0).unwrap_or(false);
            if x && s(p, "hub") == "chip" {
                chip_busy = true;
            }
            x
        } else if let Some(n) = k.strip_prefix("fan:") {
            d.get("fans").and_then(|fa| fa.get(n)).and_then(Value::as_f64).unwrap_or(0.0) > 0.0
        } else if k == "battery" {
            d.get("battery").map(|b| f(b, "watts") > 1.0).unwrap_or(false)
        } else {
            false
        };
        if b {
            out.insert(s(p, "id").to_string());
        }
    }
    if chip_busy {
        for p in arr(vw, "parts").iter().filter(|p| s(p, "live") == "chip") {
            out.insert(s(p, "id").to_string());
        }
    }
    out
}

fn part<'a>(a: &'a App, id: &str) -> Option<&'a Value> {
    a.pc_view.as_ref().and_then(|v| arr(v, "parts").iter().find(|p| s(p, "id") == id))
}

// ------------------------------------------------------------------ the board's drawing
struct Board<'a> {
    view: &'a Value,
    busy: HashSet<String>,
    sel: String,
    cursor: String,
    scale: f32,
    key: u64,
}

#[derive(Default)]
struct BoardState {
    cache: Cache,
    key: StdCell<u64>,
    size: StdCell<Option<Size>>,
}

fn center(p: &Value) -> (f32, f32) {
    ((f(p, "x") + f(p, "w") / 2.0) as f32, (f(p, "y") + f(p, "h") / 2.0) as f32)
}

impl<'a> Board<'a> {
    fn hub_pt(&self, h: &str) -> (f32, f32) {
        if let Some(pt) = self.view.get("hubs").and_then(|x| x.get(h)).and_then(Value::as_array) {
            return (pt.first().and_then(Value::as_f64).unwrap_or(435.0) as f32, pt.get(1).and_then(Value::as_f64).unwrap_or(330.0) as f32);
        }
        arr(self.view, "parts").iter().find(|p| s(p, "id") == h).map(center).unwrap_or((435.0, 330.0))
    }

    fn paint(&self, fr: &mut Frame) {
        let v = self.view;
        let (w, h) = (f(v, "w").max(1.0) as f32, f(v, "h").max(1.0) as f32);
        let board = s(v, "stage") == "board";
        fr.fill_rectangle(Point::ORIGIN, Size::new(w, h), hex(if board { "#15181c" } else if s(v, "stage") == "dark" { "#0f1216" } else { "#101317" }));
        for sh in arr(v, "shapes") {
            let path = svg_path(s(sh, "d"));
            if s(sh, "fill") == "board" {
                fr.fill(&path, hex("#1d2126"));
                for i in (0..(w as i32)).step_by(24) {
                    fr.stroke(&Path::line(Point::new(i as f32 + 0.5, 0.0), Point::new(i as f32 + 0.5, h)), Stroke::default().with_color(Color { a: 0.018, ..Color::WHITE }).with_width(1.0));
                }
                for i in (0..(h as i32)).step_by(24) {
                    fr.stroke(&Path::line(Point::new(0.0, i as f32 + 0.5), Point::new(w, i as f32 + 0.5)), Stroke::default().with_color(Color { a: 0.018, ..Color::WHITE }).with_width(1.0));
                }
                fr.stroke(&path, Stroke::default().with_color(hex("#2a2f36")).with_width(2.0));
            } else {
                let fill = s(sh, "fill");
                if !fill.is_empty() && fill != "none" {
                    fr.fill(&path, hex(fill));
                }
                if !s(sh, "stroke").is_empty() {
                    fr.stroke(&path, Stroke::default().with_color(hex(s(sh, "stroke"))).with_width(f(sh, "sw").max(1.0) as f32));
                }
            }
        }
        if board {
            let io = Path::new(|b| {
                b.move_to(Point::new(8.0, h - 164.0));
                b.line_to(Point::new(186.0, h - 164.0));
                b.line_to(Point::new(186.0, h - 8.0));
            });
            fr.stroke(&io, Stroke { line_dash: LineDash { segments: &[3.0, 3.0], offset: 0 }, ..Stroke::default().with_color(hex("#3a3f46")).with_width(1.5) });
        }
        for c in arr(v, "caps") {
            let fill = s(c, "fill");
            let pth = Path::circle(Point::new(f(c, "x") as f32, f(c, "y") as f32), f(c, "r") as f32);
            fr.fill(&pth, hex(fill));
            fr.stroke(&pth, Stroke::default().with_color(hex(if fill == "#0d0f12" { "#8a8f96" } else { "#0d0f12" })).with_width(if fill == "#0d0f12" { 2.0 } else { 1.0 }));
        }
        // the wires, under the parts: one line per lane, an L from the part to what it talks to
        let parts = arr(v, "parts");
        for p in parts {
            let hub = s(p, "hub");
            let lanes = f(p, "lanes") as i32;
            if hub.is_empty() || lanes <= 0 {
                continue;
            }
            let (ax, ay) = center(p);
            let (hx, hy) = self.hub_pt(hub);
            let n = lanes.min(10);
            let power = s(p, "kind") == "power";
            let (sp, lw) = if power { (4.5, 3.0) } else { (3.0, 1.6) };
            let on = self.sel == s(p, "id") || self.sel == hub;
            let empty = s(p, "status") == "empty";
            let busy = self.busy.contains(s(p, "id"));
            let base_c = if empty { hex("#3a4048") } else { hex("#8a6d2e") };
            let alpha = if empty { if on { 0.6 } else { 0.25 } } else if on { 0.7 } else { 0.35 };
            for k in 0..n {
                let o = (k as f32 - (n - 1) as f32 / 2.0) * sp;
                let line = Path::new(|b| {
                    b.move_to(Point::new(ax + o, ay + o));
                    b.line_to(Point::new(hx + o, ay + o));
                    b.line_to(Point::new(hx + o, hy + o));
                });
                fr.stroke(&line, Stroke::default().with_color(Pal::a(base_c, alpha)).with_width(if on { lw + 0.6 } else { lw }).with_line_cap(LineCap::Round).with_line_join(LineJoin::Round));
                if busy {
                    // a busy link: its own colour in dashes (still; Mantel's flow with Reduce motion)
                    fr.stroke(
                        &line,
                        Stroke { line_dash: LineDash { segments: &[4.0, 9.0], offset: (k as usize) * 2 }, ..Stroke::default().with_color(Pal::a(color_of(s(p, "kind")), if on { 1.0 } else { 0.55 })).with_width(lw).with_line_cap(LineCap::Round) },
                    );
                }
            }
        }
        for sk in arr(v, "silk") {
            fr.fill_text(Text { content: s(sk, "t").to_string(), position: Point::new(f(sk, "x") as f32, f(sk, "y") as f32 - 8.0), color: Color::from_rgba8(200, 204, 209, 0.55), size: 8.0.into(), font: iced::Font::MONOSPACE, ..Text::default() });
        }
        for p in parts {
            self.paint_part(fr, p);
        }
        // bolts on busy links (and the selected part's), at the middle of the wire
        for p in parts {
            let hub = s(p, "hub");
            if hub.is_empty() || f(p, "lanes") <= 0.0 {
                continue;
            }
            let on = self.sel == s(p, "id") || self.sel == hub;
            if !(self.busy.contains(s(p, "id")) || (on && s(p, "status") != "empty")) {
                continue;
            }
            let (ax, ay) = center(p);
            let (hx, hy) = self.hub_pt(hub);
            let (l1, l2) = ((hx - ax).abs(), (hy - ay).abs());
            let half = (l1 + l2) / 2.0;
            let m = if half <= l1 { (ax + (hx - ax).signum() * half, ay) } else { (hx, ay + (hy - ay).signum() * (half - l1)) };
            fr.fill(&Path::circle(Point::new(m.0, m.1), 9.0), Color::from_rgba8(12, 14, 17, 0.85));
            fr.fill(&bolt(m.0 - 6.0, m.1 - 6.0, 0.5), color_of(s(p, "kind")));
        }
    }

    fn paint_part(&self, fr: &mut Frame, p: &Value) {
        let (x, y, w, h) = (f(p, "x") as f32, f(p, "y") as f32, f(p, "w") as f32, f(p, "h") as f32);
        let st = s(p, "style");
        let round = round_style(st);
        let r = if round { w.min(h) / 2.0 } else if st == "pack" { 10.0 } else if st.starts_with("dimm") || st == "ddr3" { 3.0 } else { 4.0 };
        let shape = if round { Path::new(|b| b.ellipse(canvas::path::arc::Elliptical { center: Point::new(x + w / 2.0, y + h / 2.0), radii: Vector::new(w / 2.0, h / 2.0), rotation: iced::Radians(0.0), start_angle: iced::Radians(0.0), end_angle: iced::Radians(std::f32::consts::TAU) })) } else { Path::rounded_rectangle(Point::new(x, y), Size::new(w, h), r.into()) };
        // a soft shadow under it
        fr.fill(&Path::rounded_rectangle(Point::new(x, y + 1.5), Size::new(w, h), r.into()), Color::from_rgba8(0, 0, 0, 0.35));
        let fill = match st {
            "ioUsbc" => "#7c838a",
            "ioMetal" => "#90969d",
            "ioVga" => "#244ea0",
            "ioDvi" => "#dcdee1",
            "ioOpt" | "sd" => "#2a2e33",
            "apu" => "#2c3037",
            "chip" => "#1a1e22",
            "lpddr" => "#1e2125",
            "ddr3" => "#275430",
            "dimmFull" => "#54487a",
            "dimmEmpty" => "#262a2f",
            "pcieArmor" => "#a3aab1",
            "pciWhite" => "#dcdde0",
            "pcie1" => "#202328",
            "wifi" => "#1b4e2e",
            "shroud" => "#343a40",
            "dpad" => "#23272c",
            "trig" => "#22262b",
            "conn" => "#ccae56",
            "batt" => "#c6cbd0",
            "stick" => "#2a2e34",
            "btn" => "#22262b",
            "heatRed" => "#4a1818",
            "heat" | "heatV" => "#33383e",
            "hdr" | "spk" => "#23272c",
            "ioAudio" | "ioAnt" => "#2b2f35",
            "pwr" => "#e6e7e9",
            "pack" => "#2c3238",
            "m2heat" => "#444a52",
            "m2bare" => "#1d2126",
            "ssd" => "#173f27",
            "ioUsb3" | "ioUsb2" => "#1b1e22",
            "ioPs2" => "#7d3fb5",
            "fan" => "#2a2f35",
            "pcieGpu" => "#c7cdd3",
            _ => "#2b2f35",
        };
        fr.fill(&shape, hex(fill));
        // textures inside
        match st {
            "ioUsb3" | "ioUsb2" => fr.fill_rectangle(Point::new(x + 3.0, y + h * 0.3), Size::new((w - 6.0).max(0.0), h * 0.3), hex(if st == "ioUsb3" { "#2d6cdf" } else { "#22262b" })),
            "ioPs2" => fr.fill_rectangle(Point::new(x + w / 2.0, y), Size::new(w / 2.0, h), hex("#3d9a4a")),
            "heat" | "heatRed" => {
                let mut i = x;
                while i < x + w {
                    fr.fill_rectangle(Point::new(i, y), Size::new(3.0f32.min(x + w - i), h), hex(if st == "heat" { "#545b64" } else { "#8a2a2a" }));
                    i += 7.0;
                }
            }
            "heatV" => {
                let mut i = y;
                while i < y + h {
                    fr.fill_rectangle(Point::new(x, i), Size::new(w, 3.0f32.min(y + h - i)), hex("#545b64"));
                    i += 7.0;
                }
            }
            "cpu" => fr.fill(&Path::rounded_rectangle(Point::new(x + 9.0, y + 9.0), Size::new((w - 18.0).max(0.0), (h - 18.0).max(0.0)), 3.0.into()), hex("#b8bdc3")),
            "pcieGpu" => fr.fill_rectangle(Point::new(x, y + h * 0.25), Size::new(w, h * 0.5), hex("#ff9a4d")),
            "m2bare" => fr.fill_rectangle(Point::new(x + w * 0.92, y), Size::new(w * 0.04, h), hex("#8a8f96")),
            "ssd" => fr.fill_rectangle(Point::new(x + w * 0.7, y), Size::new(w * 0.04, h), hex("#c9a227")),
            "sata" | "sataUsed" => fr.fill_rectangle(Point::new(x + w * 0.18, y), Size::new(w * 0.64, h), hex(if st == "sata" { "#d2702a" } else { "#e8833a" })),
            "hdr" | "spk" | "pwr" => {
                let (step, rad) = match st {
                    "hdr" => (6.0, 1.4),
                    "spk" => (5.0, 1.2),
                    _ => (8.0, 2.0),
                };
                let c0 = hex(if st == "hdr" { "#d1aa3a" } else { "#0f1114" });
                let mut yy = y + step / 2.0;
                while yy < y + h {
                    let mut xx = x + step / 2.0;
                    while xx < x + w {
                        fr.fill(&Path::circle(Point::new(xx, yy), rad), c0);
                        xx += step;
                    }
                    yy += step;
                }
            }
            "ioAudio" => {
                let cols = ["#66c26a", "#e8c34a", "#5a8ee0", "#e07aa8", "#9aa1a8", "#e0904a"];
                let mut n = 0;
                let mut yy = y + 9.0;
                while yy < y + h - 4.0 {
                    let mut xx = x + 11.0;
                    while xx < x + w - 4.0 {
                        fr.fill(&Path::circle(Point::new(xx, yy), 4.0), hex(cols[n % cols.len()]));
                        n += 1;
                        xx += 20.0;
                    }
                    yy += 14.0;
                }
            }
            "ioAnt" => {
                for cx in [x + w * 0.25, x + w * 0.75] {
                    fr.fill(&Path::circle(Point::new(cx, y + h / 2.0), (h / 2.0 - 1.0).min(5.0).max(1.0)), hex("#d7b45a"));
                }
            }
            "pack" => {
                let mut i = x + 2.0;
                while i < x + w {
                    fr.fill_rectangle(Point::new(i, y), Size::new(66.0f32.min(x + w - i), h), hex("#3a414a"));
                    i += 70.0;
                }
            }
            "fan" => {
                let (cx, cy, rad) = (x + w / 2.0, y + h / 2.0, w / 2.0);
                for k in 0..10 {
                    let a0 = (k as f32 * 36.0).to_radians();
                    let a1 = (k as f32 * 36.0 + 18.0).to_radians();
                    let blade = Path::new(|b| {
                        b.move_to(Point::new(cx, cy));
                        b.line_to(Point::new(cx + rad * a0.cos(), cy + rad * a0.sin()));
                        b.line_to(Point::new(cx + rad * a1.cos(), cy + rad * a1.sin()));
                        b.close();
                    });
                    fr.fill(&blade, hex("#4a525c"));
                }
                fr.fill(&Path::circle(Point::new(cx, cy), rad * 0.22), hex("#1b1e22"));
            }
            _ => {}
        }
        // the edge: dashed for an empty slot or port
        if s(p, "status") == "empty" {
            fr.stroke(&shape, Stroke { line_dash: LineDash { segments: &[3.0, 3.0], offset: 0 }, ..Stroke::default().with_color(hex("#59616b")).with_width(1.0) });
        } else {
            fr.stroke(&shape, Stroke::default().with_color(hex("#0d0f12")).with_width(1.0));
        }
        let id = s(p, "id");
        let pc = color_of(s(p, "kind"));
        if self.sel == id {
            let glow = Path::rounded_rectangle(Point::new(x - 5.0, y - 5.0), Size::new(w + 10.0, h + 10.0), (if round { (w + 10.0) / 2.0 } else { 7.0 }).into());
            fr.stroke(&glow, Stroke::default().with_color(Pal::a(pc, 0.4)).with_width(3.0));
        }
        if self.sel == id || self.cursor == id {
            let ring = Path::rounded_rectangle(Point::new(x - 1.0, y - 1.0), Size::new(w + 2.0, h + 2.0), (if round { (w + 2.0) / 2.0 } else { 4.0 }).into());
            fr.stroke(&ring, Stroke::default().with_color(if self.sel == id { pc } else { Pal::a(hex("#66c0f4"), 0.8) }).with_width(2.0));
        }
        let label = s(p, "text");
        if !label.is_empty() {
            let vertical = p.get("vertical").and_then(Value::as_bool).unwrap_or(false);
            let small = (h < 22.0 || w < 46.0) && !vertical;
            let size0 = if id == "cpu" { 13.0 } else if small { 8.5 } else { 10.5 };
            let light = LIGHT_TEXT.contains(&st);
            let room = if vertical { h - 4.0 } else { w - 4.0 };
            let max_chars = ((room / (size0 * 0.56)).floor() as usize).max(1);
            let content = ui::fit(label, max_chars);
            let (cx, cy) = (x + w / 2.0, y + h / 2.0);
            fr.with_save(|fr| {
                fr.translate(Vector::new(cx, cy));
                if vertical {
                    fr.rotate(iced::Radians(std::f32::consts::FRAC_PI_2));
                }
                let mk = |dx: f32, dy: f32, c: Color| Text {
                    content: content.clone(),
                    position: Point::new(dx, dy),
                    color: c,
                    size: size0.into(),
                    font: hearth_ui::ui::weight(iced::font::Weight::Semibold),
                    align_x: iced::alignment::Horizontal::Center.into(),
                    align_y: iced::alignment::Vertical::Center,
                    ..Text::default()
                };
                if !light {
                    fr.fill_text(mk(0.0, 1.0, Color::from_rgba8(0, 0, 0, 0.5)));
                }
                fr.fill_text(mk(0.0, 0.0, if light { hex("#14171a") } else { hex("#e6eef5") }));
            });
        }
    }
}

fn bolt(x: f32, y: f32, k: f32) -> Path {
    let pts = [(13.0, 2.0), (4.0, 14.0), (11.0, 14.0), (10.0, 22.0), (19.0, 10.0), (12.0, 10.0)];
    Path::new(|b| {
        b.move_to(Point::new(x + pts[0].0 * k, y + pts[0].1 * k));
        for (px, py) in &pts[1..] {
            b.line_to(Point::new(x + px * k, y + py * k));
        }
        b.close();
    })
}

/// The helper's outlines: SVG path words (M, L, H, V, Q, C, Z; absolute), as ResmonBoard.qml's tracePath.
fn svg_path(d: &str) -> Path {
    let mut toks: Vec<String> = Vec::new();
    let mut cur = String::new();
    for ch in d.chars() {
        if "MLHVQCZ".contains(ch) {
            if !cur.is_empty() {
                toks.push(std::mem::take(&mut cur));
            }
            toks.push(ch.to_string());
        } else if ch == '-' && !cur.is_empty() && !cur.ends_with('e') {
            toks.push(std::mem::take(&mut cur));
            cur.push(ch);
        } else if ch.is_ascii_digit() || ch == '.' || ch == '-' {
            cur.push(ch);
        } else if !cur.is_empty() {
            toks.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        toks.push(cur);
    }
    Path::new(|b| {
        let mut i = 0;
        let (mut x, mut y) = (0.0f32, 0.0f32);
        let mut cmd = String::new();
        let num = |i: &mut usize| -> f32 {
            let v = toks.get(*i).and_then(|t| t.parse::<f32>().ok()).unwrap_or(0.0);
            *i += 1;
            v
        };
        while i < toks.len() {
            if "MLHVQCZ".contains(toks[i].as_str()) && toks[i].len() == 1 {
                cmd = toks[i].clone();
                i += 1;
            }
            match cmd.as_str() {
                "M" => {
                    x = num(&mut i);
                    y = num(&mut i);
                    b.move_to(Point::new(x, y));
                    cmd = "L".into();
                }
                "L" => {
                    x = num(&mut i);
                    y = num(&mut i);
                    b.line_to(Point::new(x, y));
                }
                "H" => {
                    x = num(&mut i);
                    b.line_to(Point::new(x, y));
                }
                "V" => {
                    y = num(&mut i);
                    b.line_to(Point::new(x, y));
                }
                "Q" => {
                    let (cx, cy) = (num(&mut i), num(&mut i));
                    x = num(&mut i);
                    y = num(&mut i);
                    b.quadratic_curve_to(Point::new(cx, cy), Point::new(x, y));
                }
                "C" => {
                    let (a1, a2, c1, c2) = (num(&mut i), num(&mut i), num(&mut i), num(&mut i));
                    x = num(&mut i);
                    y = num(&mut i);
                    b.bezier_curve_to(Point::new(a1, a2), Point::new(c1, c2), Point::new(x, y));
                }
                "Z" => {
                    b.close();
                    if i < toks.len() && !("MLHVQCZ".contains(toks[i].as_str()) && toks[i].len() == 1) {
                        i += 1;
                    }
                }
                _ => i += 1,
            }
        }
    })
}

impl<'a> canvas::Program<Msg> for Board<'a> {
    type State = BoardState;
    fn update(&self, _st: &mut BoardState, ev: &Event, b: Rectangle, c: mouse::Cursor) -> Option<canvas::Action<Msg>> {
        if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = ev {
            let pos = c.position_in(b)?;
            let (x, y) = (pos.x / self.scale, pos.y / self.scale);
            // the smallest part under the pointer (ports sit on the shroud)
            let hit = arr(self.view, "parts")
                .iter()
                .filter(|p| x >= f(p, "x") as f32 && x <= (f(p, "x") + f(p, "w")) as f32 && y >= f(p, "y") as f32 && y <= (f(p, "y") + f(p, "h")) as f32)
                .min_by(|p, q| (f(p, "w") * f(p, "h")).partial_cmp(&(f(q, "w") * f(q, "h"))).unwrap_or(std::cmp::Ordering::Equal))?;
            return Some(canvas::Action::publish(Msg::Part(s(hit, "id").to_string())).and_capture());
        }
        None
    }
    fn draw(&self, st: &BoardState, r: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        if st.key.get() != self.key || st.size.get() != Some(b.size()) {
            st.key.set(self.key);
            st.size.set(Some(b.size()));
            st.cache.clear();
        }
        vec![st.cache.draw(r, b.size(), |fr| {
            fr.scale(self.scale);
            self.paint(fr);
        })]
    }
    fn mouse_interaction(&self, _s: &BoardState, b: Rectangle, c: mouse::Cursor) -> mouse::Interaction {
        if c.is_over(b) { mouse::Interaction::Pointer } else { mouse::Interaction::default() }
    }
}

// ------------------------------------------------------------------ the details of the part you picked
fn part_live(a: &App, p: &Value) -> (Vec<(String, String)>, Vec<(String, String)>) {
    let Some(d) = a.sample.as_deref() else { return (vec![], vec![]) };
    let k = s(p, "live");
    let top = |key: &str, fmt: &dyn Fn(f64) -> String| -> Vec<(String, String)> {
        let mut g: Vec<&Value> = a.groups().iter().filter(|g| f(g, key) > 0.0).collect();
        g.sort_by(|x, y| f(y, key).partial_cmp(&f(x, key)).unwrap_or(std::cmp::Ordering::Equal));
        g.into_iter().take(3).map(|g| (s(g, "name").to_string(), fmt(f(g, key)))).collect()
    };
    let tmp = |t: Option<f64>| t.filter(|x| *x > 0.0).map(|x| format!("{} °C", x.round() as i64)).unwrap_or_else(|| "–".into());
    if k == "cpu" {
        let c = d.get("cpu").cloned().unwrap_or(Value::Null);
        return (
            vec![("Busy".into(), pct(f(&c, "total"))), ("Temperature".into(), tmp(fo(&c, "temp"))), ("Speed".into(), if f(&c, "mhz") > 0.0 { format!("{:.2} GHz", f(&c, "mhz") / 1000.0) } else { "–".into() }), ("Threads".into(), thousands(f(&c, "threads")))],
            top("cpu", &pct),
        );
    }
    if k == "mem" {
        let m = d.get("memBar").cloned().unwrap_or(Value::Null);
        return (vec![("In use".into(), size(f(&m, "inUse"))), ("Cache".into(), size(f(&m, "cache")))], top("mem", &size));
    }
    if k.starts_with("gpu:") {
        if let Some(g) = gpu_for(a, k) {
            return (
                vec![
                    ("Busy".into(), fo(g, "busy").map(pct).unwrap_or_else(|| "–".into())),
                    ("Video memory".into(), if f(g, "vramTotal") > 0.0 { format!("{} / {}", size(f(g, "vramUsed")), size(f(g, "vramTotal"))) } else { "–".into() }),
                    ("Temperature".into(), tmp(fo(g, "temp"))),
                    ("Power".into(), fo(g, "watts").map(|w| format!("{} W", w.round() as i64)).unwrap_or_else(|| "–".into())),
                ],
                top("gpu", &pct),
            );
        }
    }
    if let Some(n) = k.strip_prefix("disk:") {
        if let Some(x) = arr(d, "disks").iter().find(|x| s(x, "name") == n) {
            return (
                vec![("Reading".into(), rate(f(x, "read"))), ("Writing".into(), rate(f(x, "write"))), ("Active".into(), pct(f(x, "active"))), ("Response".into(), fo(x, "response").map(|r| format!("{r} ms")).unwrap_or_else(|| "–".into()))],
                top("disk", &rate),
            );
        }
    }
    if let Some(n) = k.strip_prefix("net:") {
        if let Some(x) = arr(d, "nets").iter().find(|x| s(x, "name") == n) {
            return (vec![("Down".into(), rate(f(x, "recv"))), ("Up".into(), rate(f(x, "send")))], vec![]);
        }
    }
    if let Some(n) = k.strip_prefix("fan:") {
        if let Some(r) = d.get("fans").and_then(|x| x.get(n)).and_then(Value::as_f64) {
            return (vec![("Speed".into(), format!("{} rpm", thousands(r)))], vec![]);
        }
    }
    if k == "battery" {
        if let Some(b) = d.get("battery").filter(|b| b.is_object()) {
            return (vec![("Charge".into(), format!("{}%", f(b, "percent") as i64)), ("Power".into(), fo(b, "watts").map(|w| format!("{w} W")).unwrap_or_else(|| "–".into()))], vec![]);
        }
    }
    (vec![], vec![])
}

/// Whether a part's button has somewhere to go on this PC (only ones that exist here are shown).
fn action_ok(a: &App, act: &Value) -> bool {
    let have = |c: &str| a.have_disks.lines().any(|l| l.trim() == c);
    match s(act, "kind") {
        "tab" | "files" => true,
        "settings" => have("hearth-settings-app") || have("systemsettings"),
        "disks" => have("hearth-disks") || have("gnome-disks") || have("kdepartitionmanager"),
        // Device Manager and the volume mixer: KDE's own on the Fedora edition (Info Center, Plasma's audio settings)
        "devices" => have("kinfocenter"),
        "mixer" => have("pavucontrol") || have("systemsettings"),
        _ => false,
    }
}

pub fn run_action(a: &mut App) -> iced::Task<Msg> {
    let Some(p) = part(a, &a.part.clone()).cloned() else { return iced::Task::none() };
    let Some(act) = p.get("action").filter(|x| x.is_object()).cloned() else { return iced::Task::none() };
    if !action_ok(a, &act) {
        return iced::Task::none();
    }
    let have = |c: &str| a.have_disks.lines().any(|l| l.trim() == c);
    match s(&act, "kind") {
        "tab" => {
            if let Some(t) = Tab::from_id(s(&act, "tab")) {
                return iced::Task::done(Msg::Tab(t));
            }
        }
        "settings" => {
            if have("hearth-settings-app") {
                run::detached(&["hearth-settings-app", "--page", s(&act, "page")]);
            } else {
                run::detached(&["systemsettings"]);
            }
        }
        "files" => run::detached(&["xdg-open", if s(&act, "path").is_empty() { "/" } else { s(&act, "path") }]),
        "disks" => {
            let c = ["hearth-disks", "gnome-disks", "kdepartitionmanager"].into_iter().find(|c| have(c)).unwrap_or("hearth-disks");
            run::detached(&[c]);
        }
        "devices" => run::detached(&["kinfocenter"]),
        "mixer" => {
            if have("pavucontrol") {
                run::detached(&["pavucontrol"]);
            } else {
                run::detached(&["systemsettings", "kcm_pulseaudio"]);
            }
        }
        _ => {}
    }
    iced::Task::none()
}

/// The D-pad on the board: the nearest part that way (Mantel's moveCursor).
pub fn move_cursor(a: &mut App, dx: f32, dy: f32) {
    let Some(vw) = a.pc_view.as_ref() else { return };
    let parts = arr(vw, "parts");
    let Some(cur) = parts.iter().find(|p| s(p, "id") == a.part_cursor).or_else(|| parts.first()) else { return };
    let (cx, cy) = center(cur);
    let mut best: Option<(&Value, f32)> = None;
    for p in parts {
        if s(p, "id") == s(cur, "id") {
            continue;
        }
        let (px, py) = center(p);
        let (px, py) = (px - cx, py - cy);
        let along = px * dx + py * dy;
        if along <= 2.0 {
            continue;
        }
        let across = (px * dy - py * dx).abs();
        let score = along + across * 2.2;
        if best.map(|b| score < b.1).unwrap_or(true) {
            best = Some((p, score));
        }
    }
    if let Some((p, _)) = best {
        a.part_cursor = s(p, "id").to_string();
    }
}

fn status_chip<'a>(label: &str, bg: &str, fg: &str) -> E<'a> {
    container(text(label.to_string()).size(11.0).color(hex(fg)).font(ui::weight(iced::font::Weight::Semibold)))
        .padding(Padding { top: 2.0, right: 9.0, bottom: 2.0, left: 9.0 })
        .style(ui::boxed(hex(bg), 10.0, None))
        .into()
}

fn caps<'a>(p: &Pal, s0: &str) -> E<'a> {
    text(s0.to_string()).size(11.0).color(p.muted).font(ui::weight(iced::font::Weight::Semibold)).into()
}

fn details<'a>(a: &'a App, busy: &HashSet<String>) -> E<'a> {
    let p = a.p;
    let Some(part0) = part(a, &a.part) else {
        return container(t(if a.pc_err.is_empty() { "Reading what this PC has inside…" } else { a.pc_err.as_str() }, 13.0, p.muted)).padding(14.0).into();
    };
    let pc = color_of(s(part0, "kind"));
    let id = s(part0, "id");
    let (st_l, st_bg, st_fg) = if busy.contains(id) { ("Busy now", "#1f4a32", "#7fe0a4") } else if s(part0, "status") == "empty" { ("Empty", "#2a3038", "#c3ccd5") } else { ("Connected", "#22405a", "#9fd6f7") };
    let icon = container(canvas::Canvas::new(BoltIcon(pc)).width(Length::Fixed(20.0)).height(Length::Fixed(20.0))).center_x(Length::Fixed(38.0)).center_y(Length::Fixed(38.0)).style(ui::boxed(Pal::a(pc, 0.2), 9.0, None));
    let mut c0 = Column::new().spacing(11.0).push(
        row![icon, column![ts(s(part0, "title").to_string(), 15.0, p.text), t(s(part0, "dev").to_string(), 12.0, Pal::mix(p.text, p.muted, 0.3))].spacing(3.0).width(Length::Fill)].spacing(10.0).align_y(iced::Center),
    );
    c0 = c0.push(status_chip(st_l, st_bg, st_fg));
    let (stats, users) = part_live(a, part0);
    if !stats.is_empty() {
        let mut grid = Column::new().spacing(6.0);
        for pair in stats.chunks(2) {
            let mut r = Row::new().spacing(6.0);
            for (k, v) in pair {
                r = r.push(container(column![t1(k.clone(), 11.0, p.muted), ts(v.clone(), 16.0, p.text).wrapping(text::Wrapping::None)].spacing(1.0)).padding(Padding { top: 6.0, right: 9.0, bottom: 6.0, left: 9.0 }).width(Length::FillPortion(1)).style(ui::boxed(p.panel2, 7.0, None)));
            }
            if pair.len() == 1 {
                r = r.push(Space::new().width(Length::FillPortion(1)));
            }
            grid = grid.push(r);
        }
        c0 = c0.push(grid);
    }
    let specs = arr(part0, "specs");
    if !specs.is_empty() {
        let mut sc = Column::new();
        for (i, sp) in specs.iter().enumerate() {
            let k = sp.get(0).and_then(Value::as_str).unwrap_or("");
            let v = sp.get(1).and_then(Value::as_str).unwrap_or("");
            sc = sc.push(container(row![container(t(k.to_string(), 12.0, p.muted)).width(Length::Fixed(104.0)), t(v.to_string(), 12.0, p.text).width(Length::Fill)].spacing(8.0)).padding(Padding { top: 5.0, right: 9.0, bottom: 5.0, left: 9.0 }).width(Length::Fill).style(ui::boxed(if i % 2 == 1 { p.panel } else { p.panel2 }, 0.0, None)));
        }
        c0 = c0.push(container(sc).clip(true).style(ui::boxed(Color::TRANSPARENT, 7.0, Some((p.line, 1.0)))));
    }
    let vw = a.pc_view.as_ref().cloned().unwrap_or(Value::Null);
    let hub = s(part0, "hub");
    let mut talks = String::new();
    if !hub.is_empty() {
        let name = vw.get("hubNames").and_then(|h| h.get(hub)).and_then(Value::as_str).map(String::from).or_else(|| part(a, hub).map(|x| s(x, "title").to_lowercase())).unwrap_or_else(|| hub.to_string());
        let lanes = f(part0, "lanes") as i64;
        talks = format!("Wired to {name}{}. ", if lanes > 0 { format!(" · {lanes} {}", if lanes == 1 { "line" } else { "lines" }) } else { String::new() });
    }
    talks.push_str(s(part0, "link"));
    c0 = c0.push(column![caps(&p, "TALKS TO"), t(talks, 12.0, Pal::mix(p.text, p.muted, 0.3))].spacing(3.0));
    if !users.is_empty() {
        let mut u = Column::new().spacing(2.0).push(caps(&p, "RIGHT NOW"));
        for (n, v) in users {
            u = u.push(row![t1(n, 12.0, p.text).width(Length::Fill), t1(v, 12.0, Pal::mix(p.text, p.muted, 0.3))]);
        }
        c0 = c0.push(u);
    }
    let tips = arr(&vw, "tips");
    if !tips.is_empty() {
        let mut tc = Column::new().spacing(6.0).push(caps(&p, "TIPS FOR THIS PC"));
        for tip in tips {
            tc = tc.push(container(t(s(tip, "text").to_string(), 12.0, hex("#d8f0dd"))).padding(Padding { top: 7.0, right: 9.0, bottom: 7.0, left: 9.0 }).width(Length::Fill).style(ui::boxed(hex("#2a3a2e"), 7.0, Some((hex("#3d5a43"), 1.0)))));
        }
        c0 = c0.push(tc);
    }
    if let Some(act) = part0.get("action").filter(|x| x.is_object()) {
        if action_ok(a, act) {
            c0 = c0.push(container(btn(&p, s(act, "label").to_string(), Kind::Primary, false, Some(Msg::PartAction))).width(Length::Fill));
        }
    }
    let mem_src = a.pc.as_ref().and_then(|x| x.get("memory")).map(|m| s(m, "source").to_string()).unwrap_or_default();
    if mem_src == "none" && a.pc.as_ref().map(|x| s(x, "family") == "desktop").unwrap_or(false) {
        c0 = c0.push(column![
            t("Each memory stick's maker, speed and slot come from the board, which Hearth reads once with the system's permission.", 11.0, p.muted),
            mouse_area(t1("Read them now", 11.0, p.accent)).on_press(Msg::ReadPcAgain).interaction(mouse::Interaction::Pointer),
        ]);
    }
    scrollable(container(c0).padding(14.0)).height(Length::Fill).into()
}

struct BoltIcon(Color);
impl canvas::Program<Msg> for BoltIcon {
    type State = Cache;
    fn draw(&self, st: &Cache, r: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        vec![st.draw(r, b.size(), |fr| fr.fill(&bolt(0.0, 0.0, b.width / 24.0), self.0))]
    }
}

pub fn page(a: &App) -> E<'_> {
    let p = a.p;
    let busy = busy_parts(a);
    let legend = row![
        t1("● ", 11.0, hex("#7fe0a4")),
        t1("busy   ", 11.0, p.muted),
        t1("● ", 11.0, hex("#66c0f4")),
        t1("connected   ", 11.0, p.muted),
        t1("○ ", 11.0, hex("#5d7488")),
        t1("empty", 11.0, p.muted)
    ]
    .align_y(iced::Center);
    let (name, sub) = match &a.pc_view {
        Some(v) => (s(v, "name").to_string(), s(v, "sub").to_string()),
        None => (if a.pc_err.is_empty() { "Reading this PC…".to_string() } else { "Your PC".to_string() }, String::new()),
    };
    // the stage keeps the drawing's shape (870 × 664), as big as the window allows
    let stage: E = match &a.pc_view {
        Some(v) => {
            let (w, h) = (f(v, "w").max(1.0) as f32, f(v, "h").max(1.0) as f32);
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            use std::hash::{Hash, Hasher};
            a.part.hash(&mut hasher);
            a.part_cursor.hash(&mut hasher);
            let mut b: Vec<&String> = busy.iter().collect();
            b.sort();
            b.hash(&mut hasher);
            (v as *const Value as usize).hash(&mut hasher);
            let key = hasher.finish();
            iced::widget::responsive(move |sz| {
                let scale = ((sz.width / w).min(sz.height / h)).clamp(0.4, 1.3);
                let brd = Board { view: v, busy: busy_parts(a), sel: a.part.clone(), cursor: a.part_cursor.clone(), scale, key: key ^ (scale.to_bits() as u64) };
                container(canvas::Canvas::new(brd).width(Length::Fixed(w * scale)).height(Length::Fixed(h * scale))).style(ui::boxed(hex("#15181c"), 10.0, Some((hex("#2a2f36"), 2.0)))).clip(true).into()
            })
            .into()
        }
        None => container(t(if a.pc_err.is_empty() { "Reading what this PC has inside…" } else { a.pc_err.as_str() }, 14.0, p.muted)).center_x(Length::Fill).center_y(Length::Fill).style(ui::boxed(hex("#15181c"), 10.0, Some((hex("#2a2f36"), 2.0)))).into(),
    };
    let left = column![
        row![ts(ui::fit(&name, 44), 15.0, p.text).wrapping(text::Wrapping::None), gap(10.0, 1.0), container(t1(sub, 13.0, p.muted)).width(Length::Fill).clip(true), legend].spacing(6.0).align_y(iced::Center),
        stage,
    ]
    .spacing(8.0)
    .width(Length::FillPortion(7));
    let right = container(details(a, &busy)).width(Length::Fixed(340.0)).height(Length::Fill).style(ui::boxed(p.panel, 10.0, Some((p.line, 1.0))));
    row![left, right].spacing(12.0).padding(12.0).height(Length::Fill).into()
}
