//! Resource Monitor's page, as the approved mockups resmon-1 … resmon-6 draw it (Mantel's ResourceMonitorView.qml):
//! the title row (Live · every second, Pause, Open Task Manager), the tabs, then each tab's foldable sections on the
//! left (Programs first, with their chips) and its live graphs on the right.

use std::collections::HashMap;

use hearth_ui::ui::{self, btn, gap, glyph, hline, t, t1, tm, ts, Kind, G};
use hearth_ui::Pal;
use iced::widget::canvas::{self, Frame, Geometry, Path};
use iced::widget::{button, column, container, mouse_area, opaque, pin, row, scrollable, stack, text, text_input, Column, Row, Space};
use iced::{mouse, Background, Border, Color, Element, Length, Padding, Point, Rectangle, Renderer, Shadow, Theme};
use serde_json::Value;

use crate::graph::{FrameMark, Spark};
use crate::{arr, f, fo, gid_of, s, App, Msg, Tab};

pub type E<'a> = Element<'a, Msg>;

// ------------------------------------------------------------------ the mockups' colours for each kind of number
pub fn col(hex: u32) -> Color {
    Color::from_rgb8((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}
pub const PURPLE: u32 = 0x8b80f9;
pub const TEAL: u32 = 0x3fd0e0;
pub const ORANGE: u32 = 0xf2a54e;
pub const GREEN: u32 = 0x5fd38a;
pub const PINK: u32 = 0xff6fb1;
pub const HOT: u32 = 0xff8a4d;
pub const YELLOW: u32 = 0xf2c14e;
/// Each kind of program's tick colour (Mantel's tagColors).
fn tag_color(g: &Value) -> Color {
    col(match s(g, "color") {
        "game" => 0xf2c14e,
        "steam" => 0x66c0f4,
        "hearth" => 0xff9a4d,
        "sys" => 0x8f8aa8,
        "web" => 0x7ccf8a,
        "home" => 0x3fb8c9,
        _ => PURPLE,
    })
}

// ------------------------------------------------------------------ numbers in words
pub fn thousands(n: f64) -> String {
    let s0 = format!("{}", n.round() as i64);
    let (neg, d) = if let Some(x) = s0.strip_prefix('-') { (true, x.to_string()) } else { (false, s0) };
    let mut out = String::new();
    for (i, c) in d.chars().enumerate() {
        if i > 0 && (d.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if neg { format!("-{out}") } else { out }
}
fn trim0(x: String) -> String {
    if x.ends_with(".0") { x[..x.len() - 2].to_string() } else { x }
}
/// 7.5 GB, 12 GB, 55.5 GB, 238 GB, 430 MB.
pub fn size(b: f64) -> String {
    let g = b / 1073741824.0;
    if g >= 100.0 {
        format!("{:.0} GB", g)
    } else if g >= 10.0 {
        format!("{} GB", trim0(format!("{g:.1}")))
    } else if g >= 1.0 {
        format!("{g:.1} GB")
    } else if b >= 1048576.0 {
        format!("{:.0} MB", b / 1048576.0)
    } else if b >= 1024.0 {
        format!("{:.0} KB", b / 1024.0)
    } else if b > 0.0 {
        format!("{:.0} B", b)
    } else {
        "0".into()
    }
}
/// Memory's facts: 0.5 GB from a tenth of a GB up (the mockup's Kept for hardware).
pub fn size_gb(b: f64) -> String {
    let g = b / 1073741824.0;
    if (0.095..1.0).contains(&g) { format!("{g:.1} GB") } else { size(b) }
}
/// In the tables: 1.21 GB.
pub fn size2(b: f64) -> String {
    let g = b / 1073741824.0;
    if (1.0..10.0).contains(&g) { format!("{g:.2} GB") } else { size(b) }
}
/// 2.4 MB/s, 60 KB/s, 0.
pub fn rate(b: f64) -> String {
    if b >= 1048576.0 {
        format!("{} MB/s", trim0(format!("{:.1}", b / 1048576.0)))
    } else if b >= 1024.0 {
        format!("{:.0} KB/s", b / 1024.0)
    } else if b > 0.0 {
        format!("{:.0} B/s", b)
    } else {
        "0".into()
    }
}
/// The Disk tab's way: always MB/s (0.4 MB/s), 0 when nothing.
pub fn rate_mb(b: f64) -> String {
    let m = b / 1048576.0;
    if m < 0.05 { "0".into() } else { format!("{} MB/s", trim0(format!("{m:.1}"))) }
}
pub fn pct(n: f64) -> String {
    if n >= 10.0 || n == 0.0 { format!("{:.0}%", n) } else { format!("{}%", trim0(format!("{n:.1}"))) }
}
pub fn pct_opt(n: Option<f64>) -> String {
    n.map(pct).unwrap_or_else(|| "–".into())
}
pub fn ghz(mhz: f64) -> String {
    if mhz > 0.0 { format!("{:.1} GHz", mhz / 1000.0) } else { "–".into() }
}
pub fn priority(n: Option<f64>) -> &'static str {
    match n {
        None => "Normal",
        Some(n) if n < -10.0 => "High",
        Some(n) if n < 0.0 => "Above normal",
        Some(n) if n == 0.0 => "Normal",
        Some(n) if n < 19.0 => "Below normal",
        _ => "Low",
    }
}
pub fn up_time(sec: f64) -> String {
    let s0 = sec as i64;
    let (d, h, m) = (s0 / 86400, s0 % 86400 / 3600, s0 % 3600 / 60);
    if d > 0 {
        format!("{d} day{} {h} h", if d > 1 { "s" } else { "" })
    } else if h > 0 {
        format!("{h} h {m} min")
    } else {
        format!("{m} min")
    }
}
fn press_word(p: Option<f64>) -> &'static str {
    match p {
        None => "–",
        Some(p) if p < 1.0 => "none",
        Some(p) if p < 10.0 => "light",
        Some(p) if p < 30.0 => "medium",
        _ => "heavy",
    }
}
/// A long path shortened in the middle (G:\SteamLibrary\steamapps\downloading\...\depot_19840.chunk).
fn mid_path(p: &str, n: usize) -> String {
    if p.chars().count() <= n {
        return p.to_string();
    }
    let sep = if p.contains('\\') { '\\' } else { '/' };
    let parts: Vec<&str> = p.split(sep).collect();
    if parts.len() > 3 {
        let last = parts[parts.len() - 1];
        let mut head = parts[0].to_string();
        for x in &parts[1..parts.len() - 1] {
            if head.chars().count() + x.chars().count() + last.chars().count() + 6 > n {
                break;
            }
            head = format!("{head}{sep}{x}");
        }
        return format!("{head}{sep}...{sep}{last}");
    }
    ui::fit(p, n)
}

/// A drive's name as the mockups write it: its letter and what it is (C: Hearth, G: Games), else Linux's.
pub fn drive_name(d: &Value) -> String {
    let letters: Vec<String> = arr(d, "letters").iter().filter_map(Value::as_str).map(String::from).collect();
    let system = d.get("system").and_then(Value::as_bool).unwrap_or(false);
    let what = if system {
        "Hearth".to_string()
    } else {
        arr(d, "mounts").iter().filter_map(Value::as_str).next().and_then(|m| m.rsplit('/').next()).filter(|x| !x.is_empty()).map(String::from).unwrap_or_else(|| s(d, "model").to_string())
    };
    match letters.first() {
        Some(l) => format!("{l}: {what}").trim().to_string(),
        None if !s(d, "label").is_empty() => s(d, "label").to_string(),
        None => s(d, "name").to_string(),
    }
}
fn kind_word(t: &str) -> &str {
    match t {
        "SSD (NVMe)" => "NVMe",
        x => x,
    }
}

// ------------------------------------------------------------------ tables
#[derive(Clone)]
pub struct Col {
    pub title: String,
    pub w: f32,
    pub right: bool,
}
fn c(title: &str, w: f32, right: bool) -> Col {
    Col { title: title.into(), w, right }
}
#[derive(Clone)]
pub struct Cell {
    pub t: String,
    pub c: Option<Color>,
}
fn cell(t: impl Into<String>) -> Cell {
    Cell { t: t.into(), c: None }
}
#[derive(Clone)]
pub struct TRow {
    pub gid: Option<String>,
    pub tick: Option<Color>,
    pub cells: Vec<Cell>,
}
pub struct Table {
    pub key: String,
    pub cols: Vec<Col>,
    pub rows: Vec<TRow>,
    pub prog: bool,
    pub empty: String,
}

/// A sortable value from a cell's words: numbers with their units (KB, MB/s, GB, ms, %), else the words.
fn sort_key(s0: &str) -> (u8, f64, String) {
    let t0 = s0.trim().trim_start_matches("✎ ");
    let num: String = t0.chars().take_while(|ch| ch.is_ascii_digit() || *ch == '.' || *ch == ',').filter(|ch| *ch != ',').collect();
    if let Ok(v) = num.parse::<f64>() {
        let rest = t0[num.len().min(t0.len())..].trim().to_lowercase();
        let k = if rest.starts_with("tb") {
            1099511627776.0
        } else if rest.starts_with("gb") || rest.starts_with("ghz") {
            1073741824.0
        } else if rest.starts_with("mb") {
            1048576.0
        } else if rest.starts_with("kb") {
            1024.0
        } else {
            1.0
        };
        return (0, v * k, String::new());
    }
    let pr = ["low", "below normal", "normal", "above normal", "high"];
    if let Some(i) = pr.iter().position(|x| t0.eq_ignore_ascii_case(x)) {
        return (0, i as f64, String::new());
    }
    (1, 0.0, t0.to_lowercase())
}

fn apply_sort(a: &App, tb: &mut Table) {
    let Some((ci, desc)) = a.sort.get(&tb.key).copied() else { return };
    tb.rows.sort_by(|x, y| {
        let (kx, ky) = (sort_key(x.cells.get(ci).map(|c| c.t.as_str()).unwrap_or("")), sort_key(y.cells.get(ci).map(|c| c.t.as_str()).unwrap_or("")));
        let o = kx.0.cmp(&ky.0).then(kx.1.partial_cmp(&ky.1).unwrap_or(std::cmp::Ordering::Equal)).then(kx.2.cmp(&ky.2));
        if desc { o.reverse() } else { o }
    });
}

fn by(v: &mut Vec<&Value>, k: impl Fn(&Value) -> f64) {
    v.sort_by(|a, b| k(b).partial_cmp(&k(a)).unwrap_or(std::cmp::Ordering::Equal).then(f(b, "mem").partial_cmp(&f(a, "mem")).unwrap_or(std::cmp::Ordering::Equal)));
}

fn num_or(v: &Value, k: &str, alt: &str) -> f64 {
    fo(v, k).unwrap_or_else(|| f(v, alt))
}

/// The programs for a tab (Mantel's rowsFor), as (name cells…). `ov`: Overview's shorter columns.
pub fn prog_rows(a: &App, t: Tab, ov: bool) -> (Vec<Col>, Vec<TRow>) {
    let mut gs: Vec<&Value> = a.groups().iter().collect();
    let d = a.sample.as_deref().cloned().unwrap_or(Value::Null);
    let prow = |g: &Value, cells: Vec<String>| TRow {
        gid: Some(gid_of(g)),
        tick: Some(tag_color(g)),
        cells: [vec![cell(s(g, "name")), cell(format!("{}", f(g, "pid") as i64))], cells.into_iter().map(cell).collect()].concat(),
    };
    let cols = |names: &[&str]| -> Vec<Col> {
        let (nw, pw) = if names.len() > 3 { (3.6, 1.0) } else { (4.6, 1.3) };
        let mut v = vec![c("Program", nw, false), c("PID", pw, true)];
        let rest = if names.len() > 3 { (12.0 - nw - pw) / names.len() as f32 } else { (11.0 - nw - pw) / names.len().max(1) as f32 };
        for n in names {
            v.push(Col { title: n.to_string(), w: rest, right: *n != "Through" });
        }
        v
    };
    match t {
        Tab::Overview | Tab::Cpu | Tab::Pc => {
            by(&mut gs, |g| f(g, "cpu"));
            if ov {
                (cols(&["CPU", "Average"]), gs.iter().map(|g| prow(g, vec![format!("{:.1}", f(g, "cpu")), pct(f(g, "avg").round())])).collect())
            } else {
                (
                    cols(&["CPU", "Threads", "Priority", "Cores"]),
                    gs.iter().map(|g| prow(g, vec![format!("{:.1}", f(g, "cpu")), thousands(f(g, "threads")), priority(fo(g, "nice")).into(), if s(g, "cores").is_empty() { "All".into() } else { s(g, "cores").into() }])).collect(),
                )
            }
        }
        Tab::Memory => {
            by(&mut gs, |g| f(g, "mem"));
            // the swapped memory on the drive: Linux doesn't say per program, so each gets the PC's zram / drive split
            let sw = arr(&d, "swaps");
            let drv: f64 = sw.iter().filter(|x| !s(x, "path").starts_with("/dev/zram")).map(|x| f(x, "used")).sum();
            let all: f64 = sw.iter().map(|x| f(x, "used")).sum();
            let share = if all > 0.0 { drv / all } else { 0.0 };
            if ov {
                (cols(&["In use", "Waits / s"]), gs.iter().map(|g| prow(g, vec![size2(f(g, "mem")), format!("{}", f(g, "waits") as i64)])).collect())
            } else {
                (
                    cols(&["In use", "Private", "Shared", "Compressed", "On drive", "Waits / s"]),
                    gs.iter()
                        .map(|g| {
                            let swp = f(g, "swap");
                            prow(g, vec![
                                size2(f(g, "mem")),
                                fo(g, "private").map(size2).unwrap_or_else(|| "–".into()),
                                fo(g, "shared").map(size2).unwrap_or_else(|| "–".into()),
                                size2((swp * (1.0 - share)).round()),
                                if share > 0.0 { size2((swp * share).round()) } else { "0".into() },
                                format!("{}", f(g, "waits") as i64),
                            ])
                        })
                        .collect(),
                )
            }
        }
        Tab::Disk => {
            by(&mut gs, |g| f(g, "read") + f(g, "write") + f(g, "disk") / 1e9);
            // the programs using the disk (mockup resmon-4); Overview's section shows the top ones whatever they do
            if !ov {
                gs.retain(|g| f(g, "read") + f(g, "write") + f(g, "disk") > 0.0);
            }
            let mut rows: Vec<TRow> = gs
                .iter()
                .map(|g| {
                    let r = num_or(g, "read", "disk");
                    let w = f(g, "write");
                    let tot = if r + w > 0.0 { r + w } else { f(g, "disk") };
                    let mut v = vec![rate_mb(r), rate_mb(w), rate_mb(tot)];
                    if !ov {
                        v.push(priority(fo(g, "nice")).into());
                    }
                    prow(g, v)
                })
                .collect();
            // the file system's own writing (btrfs: committing, compressing): what the drives wrote beyond the programs
            let dw: f64 = arr(&d, "disks").iter().map(|x| f(x, "write")).sum();
            let pw: f64 = gs.iter().map(|g| f(g, "write")).sum();
            if dw - pw > 65536.0 {
                let fs = a.drives.iter().find(|x| x.get("system").and_then(Value::as_bool) == Some(true)).map(|x| s(x, "fs").to_string()).filter(|x| !x.is_empty()).unwrap_or_else(|| "btrfs".into());
                let mut v = vec![cell(format!("{fs} (system)")), cell(""), cell("0"), cell(rate_mb(dw - pw)), cell(rate_mb(dw - pw))];
                if !ov {
                    v.push(cell("Normal"));
                }
                rows.insert(rows.len().min(3), TRow { gid: None, tick: Some(col(0x8f8aa8)), cells: v });
            }
            (if ov { cols(&["Read", "Write", "Total"]) } else { cols(&["Read", "Write", "Total", "Priority"]) }, rows)
        }
        Tab::Network => {
            let apps = a.ports.get("apps").cloned().unwrap_or(Value::Null);
            let nr = |g: &Value| -> (f64, f64) {
                let mut r = (0.0, 0.0);
                for p in arr(g, "pids").iter().filter_map(Value::as_i64) {
                    if let Some(x) = apps.get(p.to_string()) {
                        r.0 += f(x, "down");
                        r.1 += f(x, "up");
                    }
                }
                r
            };
            let mut v: Vec<(&Value, (f64, f64))> = gs.iter().map(|g| (*g, nr(g))).collect();
            v.sort_by(|x, y| ((y.1 .0 + y.1 .1), f(y.0, "conns")).partial_cmp(&((x.1 .0 + x.1 .1), f(x.0, "conns"))).unwrap_or(std::cmp::Ordering::Equal));
            // the programs using the network (mockup resmon-5): traffic or an open connection
            if !ov {
                v.retain(|(g, r)| r.0 + r.1 > 0.0 || f(g, "conns") > 0.0);
            }
            let rows = v
                .iter()
                .map(|(g, r)| {
                    let tot = r.0 + r.1;
                    prow(g, vec![rate(r.0), rate(r.1), if tot > 0.0 { rate(tot) } else if f(g, "conns") > 0.0 { format!("{} conn.", f(g, "conns") as i64) } else { "0".into() }])
                })
                .collect();
            (cols(&["Down", "Up", "Total"]), rows)
        }
        Tab::Gpu => {
            let g3 = |g: &Value| fo(g, "g3d").unwrap_or_else(|| f(g, "gpu"));
            let mut v: Vec<&Value> = gs.into_iter().filter(|g| f(g, "gpu") > 0.0 || f(g, "vram") > 0.0 || f(g, "gdec") > 0.0 || f(g, "genc") > 0.0).collect();
            v.sort_by(|x, y| (g3(y), f(y, "vram")).partial_cmp(&(g3(x), f(x, "vram"))).unwrap_or(std::cmp::Ordering::Equal));
            let api = |g: &Value| if s(g, "api").is_empty() { s(g, "gpuUse").to_string() } else { s(g, "api").to_string() };
            let zero = |x: Option<f64>| x.filter(|v| *v > 0.0).map(pct).unwrap_or_else(|| "0".into());
            if ov {
                (cols(&["3D", "Video decode", "Video memory"]), v.iter().map(|g| prow(g, vec![pct(g3(g)), zero(fo(g, "gdec")), if f(g, "vram") > 0.0 { size(f(g, "vram")) } else { "–".into() }])).collect())
            } else {
                (
                    cols(&["3D", "Video decode", "Video encode", "Video memory", "Through"]),
                    v.iter().map(|g| prow(g, vec![pct(g3(g)), zero(fo(g, "gdec")), zero(fo(g, "genc")), if f(g, "vram") > 0.0 { size(f(g, "vram")) } else { "–".into() }, api(g)])).collect(),
                )
            }
        }
    }
}

/// The table the arrows move in: the tab's programs (Overview: the Processor section's).
pub fn main_table(a: &App) -> Option<Table> {
    match a.tab {
        Tab::Pc => None,
        Tab::Overview => Some(ov_table(a, Tab::Cpu)),
        t => Some(prog_table(a, t)),
    }
}

fn prog_table(a: &App, t: Tab) -> Table {
    let (cols, rows) = prog_rows(a, t, false);
    let mut tb = Table { key: format!("{}:programs", t.id()), cols, rows, prog: true, empty: "Nothing here right now.".into() };
    apply_sort(a, &mut tb);
    tb
}

fn ov_table(a: &App, t: Tab) -> Table {
    let (cols, mut rows) = prog_rows(a, t, true);
    // the programs you ticked come first in every section (5 rows each)
    if !a.ticked.is_empty() {
        rows.sort_by_key(|r| !r.gid.as_ref().map(|g| a.ticked.contains(g)).unwrap_or(false));
    }
    let mut tb = Table { key: format!("ov:{}", t.id()), cols, rows, prog: true, empty: "Nothing here right now.".into() };
    apply_sort(a, &mut tb);
    tb.rows.truncate(5);
    tb
}

// ------------------------------------------------------------------ the frame
fn flame<'a>() -> E<'a> {
    struct Flame;
    impl<M> canvas::Program<M> for Flame {
        type State = canvas::Cache;
        fn draw(&self, st: &canvas::Cache, r: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
            vec![st.draw(r, b.size(), |fr: &mut Frame| {
                let k = b.width / 16.0;
                let p = |x: f32, y: f32| Point::new(x * k, y * k);
                let outer = Path::new(|pb| {
                    pb.move_to(p(8.0, 0.5));
                    pb.bezier_curve_to(p(9.0, 4.0), p(14.0, 6.0), p(13.5, 10.5));
                    pb.bezier_curve_to(p(13.2, 13.8), p(10.8, 15.5), p(8.0, 15.5));
                    pb.bezier_curve_to(p(5.0, 15.5), p(2.5, 13.5), p(2.5, 10.5));
                    pb.bezier_curve_to(p(2.5, 8.0), p(4.0, 6.5), p(5.0, 5.0));
                    pb.bezier_curve_to(p(5.3, 7.0), p(6.0, 8.0), p(7.0, 8.2));
                    pb.bezier_curve_to(p(6.2, 5.0), p(7.0, 2.5), p(8.0, 0.5));
                    pb.close();
                });
                fr.fill(&outer, col(0xff7a2f));
                let inner = Path::new(|pb| {
                    pb.move_to(p(8.2, 8.0));
                    pb.bezier_curve_to(p(9.0, 10.0), p(11.0, 11.0), p(10.6, 12.8));
                    pb.bezier_curve_to(p(10.3, 14.3), p(9.2, 15.0), p(8.0, 15.0));
                    pb.bezier_curve_to(p(6.6, 15.0), p(5.4, 14.0), p(5.5, 12.5));
                    pb.bezier_curve_to(p(5.6, 10.8), p(7.4, 10.0), p(8.2, 8.0));
                    pb.close();
                });
                fr.fill(&inner, col(0xffd35a));
            })]
        }
    }
    canvas::Canvas::new(Flame).width(Length::Fixed(16.0)).height(Length::Fixed(16.0)).into()
}

fn link<'a>(p: &Pal, label: String, msg: Msg) -> E<'a> {
    let p = *p;
    button(t1(label, 13.0, p.muted))
        .padding(Padding { top: 4.0, right: 2.0, bottom: 4.0, left: 2.0 })
        .on_press(msg)
        .style(move |_: &Theme, st: button::Status| button::Style {
            background: None,
            text_color: if matches!(st, button::Status::Hovered | button::Status::Pressed) { p.text } else { p.muted },
            border: Border::default(),
            shadow: Shadow::default(),
            snap: true,
        })
        .into()
}

fn title_row(a: &App) -> E<'_> {
    let p = a.p;
    let status: (String, Color) = if a.paused {
        ("Paused".into(), p.warn)
    } else if !a.err.is_empty() {
        (format!("Paused: {}", a.err), p.warn)
    } else if !a.toast.is_empty() {
        (a.toast.clone(), p.text)
    } else {
        ("Live · every second".into(), p.muted)
    };
    let wbtn = |g: G, msg: Msg, red: bool| -> E<'static> {
        button(container(glyph(g, p.muted, 10.0, 2.2)).center_x(Length::Fixed(44.0)).center_y(Length::Fixed(40.0)))
            .padding(0.0)
            .on_press(msg)
            .style(move |_: &Theme, st: button::Status| button::Style {
                background: Some(Background::Color(match st {
                    button::Status::Hovered | button::Status::Pressed if red => Color::from_rgb8(0xc4, 0x2b, 0x1c),
                    button::Status::Hovered | button::Status::Pressed => p.hover(),
                    _ => Color::TRANSPARENT,
                })),
                text_color: p.text,
                border: Border::default(),
                shadow: Shadow::default(),
                snap: true,
            })
            .into()
    };
    let drag = |e: E<'static>| -> E<'static> { mouse_area(e).on_press(Msg::Drag).on_double_click(Msg::Maximize).into() };
    let left = row![gap(14.0, 1.0), flame(), gap(8.0, 1.0), ts("Resource Monitor", 14.0, p.text).wrapping(text::Wrapping::None), gap(18.0, 1.0), t1(status.0, 13.0, status.1)].align_y(iced::Center);
    let r = row![
        drag(container(left).center_y(Length::Fixed(40.0)).into()),
        drag(container(Space::new()).width(Length::Fill).height(Length::Fixed(40.0)).into()),
        link(&p, if a.paused { "Resume ▶".into() } else { "Pause ⏸".into() }, Msg::Pause),
        t1(" · ", 13.0, p.muted),
        link(&p, "Open Task Manager".into(), Msg::OpenTm),
        gap(12.0, 1.0),
        wbtn(G::Minimize, Msg::Minimize, false),
        wbtn(G::Maximize, Msg::Maximize, false),
        wbtn(G::Close, Msg::Close, true),
        canvas::Canvas::new(FrameMark).width(Length::Fixed(1.0)).height(Length::Fixed(1.0)),
    ]
    .align_y(iced::Center);
    container(r).width(Length::Fill).height(Length::Fixed(40.0)).into()
}

fn tabs(a: &App) -> E<'_> {
    let p = a.p;
    let mut r = Row::new().spacing(2.0).padding(Padding { top: 0.0, right: 0.0, bottom: 0.0, left: 12.0 }).align_y(iced::Bottom);
    for tb in Tab::ALL {
        let on = a.tab == tb;
        let label = if on { ts(tb.label(), 14.0, p.text) } else { t(tb.label(), 14.0, p.muted) };
        let b = button(container(label.wrapping(text::Wrapping::None)).padding(Padding { top: 0.0, right: 22.0, bottom: 0.0, left: 22.0 }).center_y(Length::Fixed(37.0)))
            .padding(0.0)
            .on_press(Msg::Tab(tb))
            .style(move |_: &Theme, st: button::Status| button::Style {
                background: Some(Background::Color(if on {
                    p.panel
                } else if matches!(st, button::Status::Hovered) {
                    p.hover()
                } else {
                    Color::TRANSPARENT
                })),
                text_color: p.text,
                border: Border { color: if on { p.line } else { Color::TRANSPARENT }, width: if on { 1.0 } else { 0.0 }, radius: iced::border::Radius { top_left: 7.0, top_right: 7.0, bottom_right: 0.0, bottom_left: 0.0 } },
                shadow: Shadow::default(),
                snap: true,
            });
        r = r.push(b);
    }
    column![container(r).width(Length::Fill).height(Length::Fixed(41.0)).align_bottom(Length::Fixed(41.0)), hline(p.line)].into()
}

pub fn view(a: &App) -> E<'_> {
    let p = a.p;
    let page: E = match a.tab {
        Tab::Pc => crate::board::page(a),
        _ => {
            let cw = 330.0;
            row![
                scrollable(container(left(a)).padding(Padding { top: 0.0, right: 6.0, bottom: 14.0, left: 0.0 })).height(Length::Fill).width(Length::Fill),
                container(graphs(a)).width(Length::Fixed(cw)),
            ]
            .spacing(14.0)
            .padding(Padding { top: 14.0, right: 16.0, bottom: 10.0, left: 16.0 })
            .height(Length::Fill)
            .into()
        }
    };
    let base: E = container(column![title_row(a), hline(Pal::a(p.line, 0.6)), tabs(a), page]).width(Length::Fill).height(Length::Fill).style(ui::boxed(p.bg, 0.0, None)).into();
    let base: E = mouse_area(base).on_move(Msg::Cursor).into();
    let base = match &a.menu {
        Some(m) => {
            let catcher = mouse_area(Space::new().width(Length::Fill).height(Length::Fill)).on_press(Msg::CloseMenu).on_right_press(Msg::CloseMenu);
            stack![base, catcher, pin(opaque(menu(a, m))).x(m.at.x.min(1000.0)).y(m.at.y.min(600.0))].into()
        }
        None => base,
    };
    let base = match &a.ask_end {
        Some(g) => ui::dialog(base, end_dialog(a, g), Some(Msg::EndNo)),
        None => base,
    };
    ui::edges(base, Msg::Resize)
}

fn menu<'a>(a: &'a App, m: &'a crate::Menu) -> E<'a> {
    let p = a.p;
    let mut col0 = Column::new().spacing(1.0).push(container(t1(ui::fit(&m.prog.name, 34), 11.0, p.muted)).padding(Padding { top: 2.0, right: 8.0, bottom: 6.0, left: 8.0 }));
    for (i, (_, label, on)) in a.menu_items().into_iter().enumerate() {
        let sel = i == m.sel && on;
        let b = button(container(t1(label, 13.0, if on { p.text } else { p.muted })).padding(Padding { top: 0.0, right: 10.0, bottom: 0.0, left: 10.0 }).center_y(Length::Fixed(32.0)))
            .width(Length::Fill)
            .padding(0.0)
            .on_press_maybe(if on { Some(Msg::MenuPick(i)) } else { None })
            .style(move |_: &Theme, st: button::Status| button::Style {
                background: Some(Background::Color(if sel { p.sel() } else if on && matches!(st, button::Status::Hovered) { p.hover() } else { Color::TRANSPARENT })),
                text_color: p.text,
                border: iced::border::rounded(5.0),
                shadow: Shadow::default(),
                snap: true,
            });
        col0 = col0.push(b);
    }
    container(col0).padding(6.0).width(Length::Fixed(260.0)).style(ui::boxed(p.panel2, p.r(8.0), Some((p.line, 1.0)))).into()
}

fn end_dialog<'a>(a: &'a App, g: &'a crate::Prog) -> E<'a> {
    let p = a.p;
    let text0 = if g.protected {
        format!("{} keeps Hearth running, so it can't be ended here.", if g.why.is_empty() { "This" } else { &g.why })
    } else {
        "It closes now. Anything you haven't saved in it may be lost.".to_string()
    };
    let buttons = if g.protected {
        row![ui::fill(), btn(&p, "OK", Kind::Primary, true, Some(Msg::EndNo))]
    } else {
        row![ui::fill(), btn(&p, "Cancel", Kind::Normal, false, Some(Msg::EndNo)), btn(&p, "End task", Kind::Danger, true, Some(Msg::EndYes))].spacing(8.0)
    };
    ui::dialog_box(
        &p,
        420.0,
        column![ts(format!("End {}?", ui::fit(&g.name, 40)), 17.0, p.text), t(text0, 13.0, p.muted), t1(format!("PID {}", g.pid), 12.0, p.muted), gap(1.0, 6.0), buttons].spacing(10.0),
    )
}

// ------------------------------------------------------------------ sections
pub fn chips<'a>(p: &Pal, items: Vec<(String, &'static str)>) -> E<'a> {
    let p = *p;
    let mut r = Row::new().spacing(8.0).align_y(iced::Center);
    for (label, tone) in items {
        let (bd, fg, bg) = match tone {
            "main" => (p.accent, Pal::mix(p.accent, p.text, 0.35), Pal::a(p.accent, 0.10)),
            "good" => (col(GREEN), col(GREEN), Color::TRANSPARENT),
            "warn" => (col(0xf2a54e), col(0xf2a54e), Color::TRANSPARENT),
            _ => (p.line, p.text, Color::TRANSPARENT),
        };
        r = r.push(container(t1(label, 13.0, fg)).padding(Padding { top: 2.0, right: 10.0, bottom: 2.0, left: 10.0 }).style(ui::boxed(bg, 11.0, Some((bd, 1.0)))));
    }
    r.into()
}

/// A section: its heading (▾ / ▸ and the title; click to fold) with chips on the right, then its body.
fn section<'a>(a: &'a App, key: &str, title: String, chip_row: E<'a>, body: Option<E<'a>>) -> E<'a> {
    let p = a.p;
    let open = !a.folded.contains(key);
    let head = mouse_area(
        container(row![tb_arrow(&p, open), gap(4.0, 1.0), ts(title, 14.0, p.text).wrapping(text::Wrapping::None), ui::fill(), chip_row].align_y(iced::Center))
            .padding(Padding { top: 0.0, right: 12.0, bottom: 0.0, left: 12.0 })
            .center_y(Length::Fixed(34.0))
            .width(Length::Fill)
            .style(ui::boxed(p.panel, 0.0, None)),
    )
    .on_press(Msg::Fold(key.to_string()))
    .interaction(mouse::Interaction::Pointer);
    let mut c0 = Column::new().push(head);
    if open {
        if let Some(b) = body {
            c0 = c0.push(hline(Pal::a(p.line, 0.6))).push(b);
        }
    }
    container(c0).width(Length::Fill).clip(true).style(ui::boxed(p.bg, p.r(8.0), Some((p.line, 1.0)))).into()
}

fn tb_arrow<'a>(p: &Pal, open: bool) -> E<'a> {
    t1(if open { "▾" } else { "▸" }, 13.0, p.text).into()
}

/// A table's rows: the head (click a column to sort by it, again for the other way), the rows with a faint line
/// between them; programs have their tick, and a picked row is lit; a right-click opens the program's actions.
fn table_view<'a>(a: &'a App, tb: Table) -> E<'a> {
    let p = a.p;
    let total: f32 = tb.cols.iter().map(|c| c.w).sum::<f32>().max(0.01);
    let portion = |w: f32| Length::FillPortion(((w / total) * 1000.0).round().max(1.0) as u16);
    let sorted = a.sort.get(&tb.key).copied();
    let mut head = Row::new().padding(Padding { top: 0.0, right: 14.0, bottom: 0.0, left: 14.0 }).spacing(10.0).align_y(iced::Center);
    for (i, cl) in tb.cols.iter().enumerate() {
        let mark = match sorted {
            Some((ci, desc)) if ci == i => if desc { " ▾" } else { " ▴" },
            _ => "",
        };
        let lbl = t1(format!("{}{}", cl.title, mark), 13.0, p.muted);
        let cellc = if cl.right { container(lbl).align_right(portion(cl.w)) } else { container(lbl).width(portion(cl.w)) };
        head = head.push(mouse_area(cellc).on_press(Msg::Sort(tb.key.clone(), i)).interaction(mouse::Interaction::Pointer));
    }
    let mut col0 = Column::new().push(container(head).center_y(Length::Fixed(28.0)).width(Length::Fill));
    if tb.rows.is_empty() {
        col0 = col0.push(hline(Pal::a(p.line, 0.5))).push(container(t1(tb.empty.clone(), 13.0, p.muted)).padding(Padding { top: 7.0, right: 14.0, bottom: 8.0, left: 14.0 }));
    }
    let sub = Pal::mix(p.text, p.muted, 0.25);
    for (ri, r) in tb.rows.into_iter().enumerate() {
        let gid = r.gid.clone().unwrap_or_default();
        let picked = tb.prog && !gid.is_empty() && a.pick.0 == tb.key && a.pick.2 == gid;
        let ticked = !gid.is_empty() && a.ticked.contains(&gid);
        let mut line = Row::new().padding(Padding { top: 0.0, right: 14.0, bottom: 0.0, left: 14.0 }).spacing(10.0).align_y(iced::Center);
        for (ci, (cl, ce)) in tb.cols.iter().zip(r.cells.into_iter()).enumerate() {
            let color = ce.c.unwrap_or(if ci == 0 { p.text } else if tb.prog && ci == 1 { p.muted } else if !tb.prog && cl.right && ci > 0 && cl.title == "PID" { p.muted } else { sub });
            let words: E = if ci == 0 && tb.prog {
                let bx_c = r.tick.unwrap_or(p.accent);
                let bx: E = container(if ticked { glyph(G::Check, Color::from_rgb8(0x10, 0x10, 0x18), 10.0, 3.0) } else { gap(10.0, 10.0) })
                    .center_x(Length::Fixed(15.0))
                    .center_y(Length::Fixed(15.0))
                    .style(ui::boxed(if ticked { bx_c } else { Color::TRANSPARENT }, 3.0, if ticked { None } else { Some((p.muted, 1.0)) }))
                    .into();
                let bx: E = if gid.is_empty() { bx } else { mouse_area(bx).on_press(Msg::Tick(gid.clone())).interaction(mouse::Interaction::Pointer).into() };
                row![bx, gap(10.0, 1.0), t1(ce.t, 14.0, color)].align_y(iced::Center).into()
            } else {
                t1(ce.t, 14.0, color).into()
            };
            line = line.push(if cl.right { container(words).align_right(portion(cl.w)).clip(true) } else { container(words).width(portion(cl.w)).clip(true) });
        }
        let bg = if picked { Pal::mix(p.bg, p.accent, 0.20) } else { Color::TRANSPARENT };
        let rowc: E = container(line).center_y(Length::Fixed(24.0)).width(Length::Fill).style(ui::boxed(bg, 0.0, None)).into();
        let rowc: E = if tb.prog && !gid.is_empty() {
            mouse_area(rowc).on_press(Msg::Pick(tb.key.clone(), ri, gid.clone())).on_right_press(Msg::RowMenu(tb.key.clone(), ri, gid.clone())).into()
        } else {
            rowc
        };
        col0 = col0.push(hline(Pal::a(p.line, 0.45))).push(rowc);
    }
    col0.push(gap(1.0, 4.0)).into()
}

/// Facts in three columns (Physical memory, Graphics card).
fn facts<'a>(p: &Pal, items: Vec<(String, String)>) -> E<'a> {
    let p = *p;
    let mut rows = Column::new().spacing(5.0).padding(Padding { top: 10.0, right: 18.0, bottom: 12.0, left: 18.0 });
    for chunk in items.chunks(3) {
        let mut r = Row::new().spacing(28.0);
        for (k, v) in chunk {
            r = r.push(container(row![t1(k.clone(), 14.0, Pal::mix(p.text, p.muted, 0.35)), ui::fill(), tm(v.clone(), 14.0, p.text).wrapping(text::Wrapping::None)].align_y(iced::Center)).width(Length::FillPortion(1)));
        }
        for _ in chunk.len()..3 {
            r = r.push(Space::new().width(Length::FillPortion(1)));
        }
        rows = rows.push(r);
    }
    rows.into()
}

fn mem_bar<'a>(p: &Pal, parts: Vec<(String, f64, Color, Color)>, total: f64) -> E<'a> {
    let mut r = Row::new();
    for (label, v, bg, fg) in parts {
        let frac = if total > 0.0 { (v / total).clamp(0.0, 1.0) } else { 0.0 };
        let w = (frac * 1000.0).round() as u16;
        if w == 0 {
            continue;
        }
        let lbl: E = if frac > 0.045 { ts(label, 13.0, fg).wrapping(text::Wrapping::None).into() } else { Space::new().into() };
        r = r.push(container(lbl).center_x(Length::FillPortion(w)).center_y(Length::Fixed(26.0)).clip(true).style(ui::boxed(bg, 0.0, None)));
    }
    container(container(r).width(Length::Fill).height(Length::Fixed(26.0)).clip(true).style(ui::boxed(p.line, 5.0, None)))
        .padding(Padding { top: 10.0, right: 18.0, bottom: 2.0, left: 18.0 })
        .into()
}

fn tbl(key: &str, cols: Vec<Col>, rows: Vec<Vec<Cell>>, empty: &str) -> Table {
    Table { key: key.into(), cols, rows: rows.into_iter().map(|c0| TRow { gid: None, tick: None, cells: c0 }).collect(), prog: false, empty: empty.into() }
}

fn sorted(a: &App, mut tb: Table) -> Table {
    apply_sort(a, &mut tb);
    tb
}

/// The left column: each tab's sections (Mantel's sectionsFor), Overview's five.
fn left(a: &App) -> E<'_> {
    let p = a.p;
    let d = a.sample.as_deref().cloned().unwrap_or(Value::Null);
    let cpu = d.get("cpu").cloned().unwrap_or(Value::Null);
    let mb = d.get("memBar").cloned().unwrap_or(Value::Null);
    let mut out = Column::new().spacing(12.0);
    let ticked_pids: Vec<i64> = a.groups().iter().filter(|g| a.ticked.contains(&gid_of(g))).flat_map(|g| arr(g, "pids").iter().filter_map(Value::as_i64).collect::<Vec<_>>()).collect();
    let pick_pid = |pid: i64| ticked_pids.is_empty() || ticked_pids.contains(&pid);
    match a.tab {
        Tab::Overview => {
            for (k, title) in [(Tab::Cpu, "Processor"), (Tab::Memory, "Memory"), (Tab::Disk, "Disk"), (Tab::Network, "Network"), (Tab::Gpu, "Graphics")] {
                let key = format!("ov:{}", k.id());
                let ch = chips(&p, overview_chips(a, k, &d));
                let body = if a.folded.contains(&key) { None } else { Some(table_view(a, ov_table(a, k))) };
                out = out.push(section(a, &key, title.into(), ch, body));
            }
            let names: Vec<String> = a.groups().iter().filter(|g| a.ticked.contains(&gid_of(g))).map(|g| s(g, "name").to_string()).collect();
            let hint = if names.is_empty() {
                "Tick programs to filter every section to just them. Click a heading to fold it.".to_string()
            } else {
                format!("Tick programs to filter every section to just them (here: {}). Click a heading to fold it.", names.join(", "))
            };
            out = out.push(container(t(hint, 13.0, p.muted)).padding(Padding { top: 6.0, right: 4.0, bottom: 0.0, left: 16.0 }));
        }
        Tab::Cpu => {
            let chip_v = if cpu.is_object() { vec![(format!("{} used", pct(f(&cpu, "total"))), "main"), (format!("{} threads", thousands(f(&cpu, "threads"))), ""), (format!("Up {}", up_time(f(&d, "uptime"))), "")] } else { vec![] };
            out = out.push(section(a, "cpu:programs", "Programs".into(), chips(&p, chip_v), Some(table_view(a, prog_table(a, Tab::Cpu)))));
            if let Some(fc) = a.focus() {
                let banner = row![
                    t1("Filtered to ", 14.0, Pal::mix(p.text, p.muted, 0.25)),
                    ts(format!("{} ({})", fc.name, fc.pid), 14.0, col(PURPLE)).wrapping(text::Wrapping::None),
                    t1(" · showing its services, open files and libraries", 14.0, Pal::mix(p.text, p.muted, 0.25)),
                ]
                .align_y(iced::Center);
                out = out.push(container(banner).padding(Padding { top: 0.0, right: 16.0, bottom: 0.0, left: 16.0 }).center_y(Length::Fixed(36.0)).width(Length::Fill).style(ui::boxed(Pal::a(col(PURPLE), 0.12), p.r(6.0), None)));
                let running = a.services.iter().filter(|x| s(x, "status") == "Running").count();
                let groups = a.groups();
                let svc_rows: Vec<Vec<Cell>> = a
                    .services
                    .iter()
                    .map(|x| {
                        let pid = f(x, "pid") as i64;
                        let gcpu = groups.iter().find(|g| arr(g, "pids").iter().any(|q| q.as_i64() == Some(pid))).map(|g| f(g, "cpu")).unwrap_or(0.0);
                        vec![cell(s(x, "name")), cell(if pid > 0 { pid.to_string() } else { "—".into() }), cell(s(x, "status")), cell(pct(gcpu)), cell(s(x, "what"))]
                    })
                    .collect();
                let st = sorted(a, tbl("cpu:services", vec![c("Service", 3.0, false), c("PID", 1.0, true), c("Status", 1.4, false), c("CPU", 1.0, true), c("What it is", 4.0, false)], svc_rows, "It doesn't run as a service."));
                out = out.push(section(a, "cpu:services", "Services".into(), chips(&p, vec![(format!("{running} running"), "")]), Some(table_view(a, st))));
                let q = a.files_query.to_lowercase();
                let file_rows: Vec<Vec<Cell>> = a
                    .open_files
                    .iter()
                    .filter(|x| q.is_empty() || s(x, "path").to_lowercase().contains(&q) || s(x, "win").to_lowercase().contains(&q))
                    .take(40)
                    .map(|x| vec![cell(fc.name.clone()), cell(format!("{}", f(x, "pid") as i64)), cell(if s(x, "win").is_empty() { s(x, "path") } else { s(x, "win") })])
                    .collect();
                let ft = sorted(a, tbl("cpu:files", vec![c("Program", 3.0, false), c("PID", 1.0, true), c("File", 8.0, false)], file_rows, "No files open."));
                let search: E = container(
                    row![
                        t1("search:", 13.0, p.text),
                        text_input("", &a.files_query).on_input(Msg::FilesQuery).size(13.0).width(Length::Fixed(110.0)).padding(Padding { top: 1.0, right: 2.0, bottom: 1.0, left: 4.0 }).style(move |_t: &Theme, _s| text_input::Style {
                            background: Background::Color(Color::TRANSPARENT),
                            border: Border::default(),
                            icon: p.muted,
                            placeholder: p.muted,
                            value: p.text,
                            selection: Pal::a(p.accent, 0.35),
                        }),
                    ]
                    .align_y(iced::Center),
                )
                .padding(Padding { top: 1.0, right: 8.0, bottom: 1.0, left: 10.0 })
                .style(ui::boxed(Color::TRANSPARENT, 11.0, Some((p.line, 1.0))))
                .into();
                out = out.push(section(a, "cpu:files", "Open files".into(), search, Some(table_view(a, ft))));
                let lib_rows: Vec<Vec<Cell>> = a.libs.iter().map(|x| vec![cell(s(x, "name")), cell(s(x, "path"))]).collect();
                let n_libs = lib_rows.len();
                let lt = sorted(a, tbl("cpu:libs", vec![c("Library", 3.0, false), c("Where", 8.0, false)], lib_rows, "None it shares with others."));
                let lib_chips = if a.folded.contains("cpu:libs") { vec![] } else { vec![(format!("{n_libs} loaded"), "")] };
                out = out.push(section(a, "cpu:libs", "Libraries (modules)".into(), chips(&p, lib_chips), Some(table_view(a, lt))));
            }
        }
        Tab::Memory => {
            let pr = fo(&d, "pressure");
            let chip_v = vec![(format!("{} in use", size(f(&mb, "inUse"))), "main"), (format!("pressure: {}", press_word(pr)), if pr.unwrap_or(0.0) >= 10.0 { "warn" } else { "good" })];
            out = out.push(section(a, "memory:programs", "Programs".into(), chips(&p, chip_v), Some(table_view(a, prog_table(a, Tab::Memory)))));
            let zr = d.get("zram").cloned();
            let zmem = zr.as_ref().map(|z| f(z, "mem")).unwrap_or(0.0);
            let programs = (f(&mb, "inUse") - zmem).max(0.0);
            let cached = f(&mb, "cache") + f(&mb, "saving");
            let total = if f(&mb, "installed") > 0.0 { f(&mb, "installed") } else { f(&mb, "total").max(1.0) };
            let g1 = |b: f64| trim0(format!("{:.1}", b / 1073741824.0));
            let parts = vec![
                ("Kept".to_string(), f(&mb, "reserved"), col(0x6c6a7c), p.text),
                (format!("In use {}", size(programs)), programs, col(PURPLE), col(0x15131f)),
                (format!("Compr. {:.1}", zmem / 1073741824.0), zmem, col(TEAL), col(0x0f1a1d)),
                (format!("Cached {:.1}", cached / 1073741824.0), cached, col(0xcfc9d8), col(0x1a1822)),
                (format!("Free {}", size(f(&mb, "free"))), f(&mb, "free"), Pal::mix(p.panel, p.line, 0.6), Pal::mix(p.text, p.muted, 0.3)),
            ];
            let drive: Vec<&Value> = arr(&d, "swaps").iter().filter(|x| !s(x, "path").starts_with("/dev/zram")).collect();
            let mem = d.get("mem").cloned().unwrap_or(Value::Null);
            let zr_holds = match &zr {
                Some(z) if f(z, "size") > 0.0 => {
                    let ratio = if f(z, "mem") > 0.0 { format!(" ({}:1)", (f(z, "orig") / f(z, "mem")).round().max(1.0)) } else { String::new() };
                    format!("{} of {}{}", g1(f(z, "orig")), size(f(z, "size")), ratio)
                }
                _ => "off".into(),
            };
            let fx = vec![
                ("Kept for hardware".to_string(), size_gb(f(&mb, "reserved"))),
                ("In use".into(), size_gb(programs)),
                ("Compressed (zram real size)".into(), if zr.is_some() { size_gb(zmem) } else { "off".into() }),
                ("Cached".into(), size_gb(cached)),
                ("Free".into(), size_gb(f(&mb, "free"))),
                ("Available".into(), size_gb(f(&mem, "available"))),
                ("Zram holds".into(), zr_holds),
                ("Drive swap".into(), if drive.is_empty() { "none".into() } else { format!("{} of {}", g1(drive.iter().map(|x| f(x, "used")).sum()), size_gb(drive.iter().map(|x| f(x, "size")).sum())) }),
                ("Committed".into(), {
                    let lim = f(&mem, "commitLimit");
                    if lim > 0.0 { format!("{} of {}", g1(f(&mb, "committed")), size_gb(lim)) } else { size_gb(f(&mb, "committed")) }
                }),
            ];
            let body = column![mem_bar(&p, parts, total), facts(&p, fx)];
            out = out.push(section(a, "memory:physical", "Physical memory".into(), chips(&p, vec![(size(total), "")]), Some(body.into())));
        }
        Tab::Disk => {
            let ds = arr(&d, "disks");
            let chip_v = vec![(format!("{} write", rate(ds.iter().map(|x| f(x, "write")).sum())), ""), (format!("{} read", rate(ds.iter().map(|x| f(x, "read")).sum())), "")];
            out = out.push(section(a, "disk:programs", "Programs using the disk".into(), chips(&p, chip_v), Some(table_view(a, prog_table(a, Tab::Disk)))));
            let frows: Vec<Vec<Cell>> = a
                .files_busy
                .iter()
                .filter(|x| pick_pid(f(x, "pid") as i64))
                .take(20)
                .map(|x| {
                    let pid = f(x, "pid") as i64;
                    let path = if s(x, "win").is_empty() { s(x, "path") } else { s(x, "win") };
                    vec![cell(mid_path(path, 62)), cell(a.busy_names.get(&pid).cloned().unwrap_or_default()), cell(pid.to_string()), cell(rate_mb(f(x, "read"))), cell(rate_mb(f(x, "write")))]
                })
                .collect();
            let ft = sorted(a, tbl("disk:files", vec![c("File", 6.0, false), c("Program", 1.8, false), c("PID", 0.9, true), c("Read", 1.2, true), c("Write", 1.2, true)], frows, "Nothing is reading or writing files right now."));
            out = out.push(section(a, "disk:files", "Files being read and written".into(), chips(&p, vec![]), Some(table_view(a, ft))));
            let health = a.health.get("drives").cloned().unwrap_or(Value::Null);
            let mut drows = Vec::new();
            for dv in &a.drives {
                let base = s(dv, "name").trim_end_matches(|ch: char| ch.is_ascii_digit()).trim_end_matches('p').to_string();
                let h = health.get(s(dv, "name")).or_else(|| health.get(&base)).cloned().unwrap_or(Value::Null);
                let live = ds.iter().find(|x| s(x, "name") == s(dv, "name")).cloned().unwrap_or(Value::Null);
                let life = fo(&h, "lifeLeft");
                let hc = if h.get("ok").and_then(Value::as_bool) == Some(false) { Some(col(0xf2a54e)) } else if life.unwrap_or(0.0) >= 80.0 { Some(col(GREEN)) } else { None };
                let ht = match life {
                    Some(l) => format!("{}%", l as i64),
                    None if h.get("ok").and_then(Value::as_bool) == Some(false) => "Failing".into(),
                    None => "–".into(),
                };
                let typ = [kind_word(s(dv, "type")), s(dv, "fs")].iter().filter(|x| !x.is_empty()).cloned().collect::<Vec<_>>().join(" · ");
                drows.push(vec![
                    cell(drive_name(dv)),
                    cell(typ),
                    cell(fo(&live, "active").map(pct).unwrap_or_else(|| "–".into())),
                    cell(fo(&live, "response").map(|r| format!("{} ms", trim0(format!("{r:.1}")))).unwrap_or_else(|| "–".into())),
                    cell(fo(dv, "free").map(size).unwrap_or_else(|| "–".into())),
                    cell(fo(dv, "total").map(size).unwrap_or_else(|| "–".into())),
                    Cell { t: ht, c: hc },
                    cell(fo(dv, "temp").or_else(|| fo(&h, "temp")).map(|t0| format!("{}°C", t0.round() as i64)).unwrap_or_else(|| "–".into())),
                ]);
            }
            let dt = sorted(a, tbl("disk:drives", vec![c("Drive", 2.0, false), c("Type", 2.0, false), c("Busy", 1.0, true), c("Response", 1.2, true), c("Free", 1.2, true), c("Total", 1.2, true), c("Health", 1.0, true), c("Temp", 1.0, true)], drows, "Reading the drives…"));
            out = out.push(section(a, "disk:drives", "Drives".into(), chips(&p, vec![]), Some(table_view(a, dt))));
        }
        Tab::Network => {
            let ns: Vec<&Value> = arr(&d, "nets").iter().filter(|x| x.get("up").and_then(Value::as_bool).unwrap_or(false)).collect();
            let chip_v = vec![(format!("{} down", rate(ns.iter().map(|x| f(x, "recv")).sum())), ""), (format!("{} up", rate(ns.iter().map(|x| f(x, "send")).sum())), "")];
            out = out.push(section(a, "network:programs", "Programs using the network".into(), chips(&p, chip_v), Some(table_view(a, prog_table(a, Tab::Network)))));
            let conns: Vec<Vec<Cell>> = arr(&a.ports, "connections")
                .iter()
                .filter(|x| pick_pid(f(x, "pid") as i64))
                .take(30)
                .map(|x| {
                    let pid = f(x, "pid") as i64;
                    let name = a.groups().iter().find(|g| arr(g, "pids").iter().any(|q| q.as_i64() == Some(pid))).map(|g| s(g, "name").to_string()).unwrap_or_else(|| s(x, "name").to_string());
                    vec![
                        cell(name),
                        cell(if pid > 0 { pid.to_string() } else { "—".into() }),
                        cell(if s(x, "host").is_empty() { s(x, "remote") } else { s(x, "host") }),
                        cell(format!("{}", f(x, "port") as i64)),
                        cell(fo(x, "ping").map(|v| format!("{} ms", v as i64)).unwrap_or_else(|| "–".into())),
                        cell(if s(x, "state").is_empty() { "Open" } else { s(x, "state") }),
                    ]
                })
                .collect();
            let ct = sorted(a, tbl("net:conns", vec![c("Program", 2.6, false), c("PID", 1.0, true), c("Talking to", 4.5, false), c("Port", 1.0, true), c("Ping", 1.0, true), c("State", 1.0, false)], conns, "No open connections."));
            out = out.push(section(a, "net:conns", "Connections (TCP)".into(), chips(&p, vec![]), Some(table_view(a, ct))));
            let listen: Vec<Vec<Cell>> = arr(&a.ports, "listening")
                .iter()
                .filter(|x| pick_pid(f(x, "pid") as i64))
                .take(30)
                .map(|x| {
                    let pid = f(x, "pid") as i64;
                    vec![cell(s(x, "name")), cell(if pid > 0 { pid.to_string() } else { "—".into() }), cell(format!("{}", f(x, "port") as i64)), cell(s(x, "openTo")), cell(s(x, "what"))]
                })
                .collect();
            let lt = sorted(a, tbl("net:listen", vec![c("Program", 2.2, false), c("PID", 0.9, true), c("Port", 1.0, true), c("Open to", 2.4, false), c("What it is", 4.0, false)], listen, "Nothing is listening."));
            let fw = s(&a.ports, "firewall");
            let fw = if fw.is_empty() { "allowed only on your network" } else { fw };
            out = out.push(section(a, "net:listen", "Listening ports".into(), chips(&p, vec![(format!("firewall: {fw}"), "warn")]), Some(table_view(a, lt))));
        }
        Tab::Gpu => {
            let g = arr(&d, "gpus").first().cloned().unwrap_or(Value::Null);
            let card = d.get("card").cloned().unwrap_or(Value::Null);
            let mut chip_v = vec![];
            if !s(&g, "name").is_empty() {
                let who = if s(&g, "vendor") == "Intel" { "Intel graphics".to_string() } else if s(&g, "vendor").is_empty() { "GPU".into() } else { s(&g, "vendor").to_string() };
                chip_v.push((format!("{who} {}", fo(&g, "busy").map(pct).unwrap_or_default()).trim().to_string(), "main"));
                if f(&g, "temp") > 0.0 {
                    chip_v.push((format!("{}°C", f(&g, "temp").round() as i64), ""));
                }
            }
            out = out.push(section(a, "gpu:programs", "Programs using the graphics card".into(), chips(&p, chip_v), Some(table_view(a, prog_table(a, Tab::Gpu)))));
            let games: Vec<String> = a.groups().iter().filter(|x| s(x, "kind") == "game").map(|x| s(x, "name").to_string()).collect();
            let drv = [if s(&card, "mesa").is_empty() { String::new() } else { format!("Mesa {}", s(&card, "mesa")) }, if s(&card, "driver").is_empty() { s(&g, "driver").to_string() } else { s(&card, "driver").to_string() }]
                .into_iter()
                .filter(|x| !x.is_empty())
                .collect::<Vec<_>>()
                .join(" · ");
            let screens: Vec<String> = arr(&card, "screens").iter().filter_map(Value::as_str).map(String::from).collect();
            let fx = vec![
                ("Name".to_string(), if s(&g, "name").is_empty() { "–".into() } else { s(&g, "name").to_string() }),
                ("Driver".into(), if drv.is_empty() { "–".into() } else { drv }),
                ("Shared memory".into(), if f(&g, "gttTotal") > 0.0 { format!("{} of {}", size(f(&g, "gttUsed")), size(f(&g, "gttTotal"))) } else if f(&g, "vramTotal") > 0.0 { format!("{} of {}", size(f(&g, "vramUsed")), size(f(&g, "vramTotal"))) } else { "–".into() }),
                ("Clock".into(), if f(&g, "mhz") > 0.0 { format!("{}{} MHz", f(&g, "mhz").round() as i64, fo(&g, "mhzMax").map(|m| format!(" / {}", m.round() as i64)).unwrap_or_default()) } else { "–".into() }),
                ("Temperature".into(), if f(&g, "temp") > 0.0 { format!("{}°C", f(&g, "temp").round() as i64) } else { "–".into() }),
                ("Power".into(), match fo(&g, "watts") { Some(w) if w > 0.0 => if w < 10.0 { format!("{w:.1} W") } else { format!("{} W", w.round() as i64) }, _ => "–".into() }),
                ("Screens".into(), if screens.is_empty() { "–".into() } else { format!("{} · {}", screens.len(), screens.join(", ")) }),
                ("Vulkan · OpenGL".into(), format!("{} · {}", if s(&card, "vulkan").is_empty() { "–" } else { s(&card, "vulkan") }, if s(&card, "opengl").is_empty() { "–" } else { s(&card, "opengl") })),
                ("Games running".into(), if games.is_empty() { "none".into() } else { games.join(", ") }),
            ];
            out = out.push(section(a, "gpu:card", "Graphics card".into(), chips(&p, vec![]), Some(facts(&p, fx))));
        }
        Tab::Pc => {}
    }
    out.into()
}

/// The chips on each Overview heading (Mantel's overviewChips, with the mockup's ping on Network).
fn overview_chips(a: &App, k: Tab, d: &Value) -> Vec<(String, &'static str)> {
    let cpu = d.get("cpu").cloned().unwrap_or(Value::Null);
    match k {
        Tab::Cpu if cpu.is_object() => {
            let mut v = vec![(format!("{} used", pct(f(&cpu, "total"))), "main"), (ghz(f(&cpu, "mhz")), "")];
            if f(&cpu, "temp") > 0.0 {
                v.push((format!("{}°C", f(&cpu, "temp").round() as i64), if f(&cpu, "temp") > 85.0 { "warn" } else { "" }));
            }
            v
        }
        Tab::Memory => {
            let mb = d.get("memBar").cloned().unwrap_or(Value::Null);
            let mut v = vec![(format!("{} in use", size(f(&mb, "inUse"))), "main")];
            if let Some(z) = d.get("zram").filter(|z| f(z, "mem") > 0.0) {
                v.push((format!("{} compressed", size(f(z, "mem"))), ""));
            } else if f(&mb, "swapUsed") > 0.0 {
                v.push((format!("{} in swap", size(f(&mb, "swapUsed"))), ""));
            }
            if let Some(pr) = fo(d, "pressure") {
                v.push((format!("pressure: {}", press_word(Some(pr))), if pr < 10.0 { "good" } else { "warn" }));
            }
            v
        }
        Tab::Disk => {
            let ds = arr(d, "disks");
            let busiest = ds.iter().max_by(|x, y| f(x, "active").partial_cmp(&f(y, "active")).unwrap_or(std::cmp::Ordering::Equal));
            match busiest {
                Some(b) => {
                    let letter = arr(b, "letters").iter().filter_map(Value::as_str).next().map(|l| format!("{l}:")).unwrap_or_else(|| s(b, "label").to_string());
                    vec![(format!("{letter} {} busy", pct(f(b, "active"))), ""), (format!("{} write", rate(ds.iter().map(|x| f(x, "write")).sum())), "")]
                }
                None => vec![],
            }
        }
        Tab::Network => {
            let ns: Vec<&Value> = arr(d, "nets").iter().filter(|x| x.get("up").and_then(Value::as_bool).unwrap_or(false)).collect();
            if ns.is_empty() {
                return vec![("Not connected".into(), "")];
            }
            let mut v = vec![(format!("{} down", rate(ns.iter().map(|x| f(x, "recv")).sum())), ""), (format!("{} up", rate(ns.iter().map(|x| f(x, "send")).sum())), "")];
            if let Some(pg) = a.hist.get("ping").and_then(|h| h.back()).filter(|x| **x > 0.0) {
                v.push((format!("{} ms", pg.round() as i64), ""));
            }
            v
        }
        Tab::Gpu => match arr(d, "gpus").first() {
            Some(g) => {
                let mut v = vec![(format!("{} {}", if s(g, "vendor").is_empty() { "GPU" } else { s(g, "vendor") }, fo(g, "busy").map(pct).unwrap_or_default()).trim().to_string(), "")];
                if f(g, "temp") > 0.0 {
                    v.push((format!("{}°C", f(g, "temp").round() as i64), ""));
                }
                v
            }
            None => vec![("No graphics card seen".into(), "")],
        },
        _ => vec![],
    }
}

// ------------------------------------------------------------------ the graphs
struct GSpec {
    title: String,
    value: String,
    color: u32,
    key: String,
    top: f32,
    span: bool,
}
fn gs(title: impl Into<String>, value: impl Into<String>, color: u32, key: impl Into<String>, top: f32) -> GSpec {
    GSpec { title: title.into(), value: value.into(), color, key: key.into(), top, span: false }
}

fn last(h: &HashMap<String, std::collections::VecDeque<f32>>, k: &str) -> f32 {
    h.get(k).and_then(|q| q.back().copied()).unwrap_or(0.0)
}

fn graph_specs(a: &App) -> Vec<GSpec> {
    let d = a.sample.as_deref().cloned().unwrap_or(Value::Null);
    let cpu = d.get("cpu").cloned().unwrap_or(Value::Null);
    let mb = d.get("memBar").cloned().unwrap_or(Value::Null);
    let h = &a.hist;
    let mem_pct = if f(&mb, "total") > 0.0 { 100.0 * f(&mb, "inUse") / f(&mb, "total") } else { 0.0 };
    let g0 = arr(&d, "gpus").first().cloned().unwrap_or(Value::Null);
    match a.tab {
        Tab::Cpu => {
            let mhz = arr(&d, "coreMhz");
            let mut v: Vec<GSpec> = arr(&cpu, "cores")
                .iter()
                .take(16)
                .enumerate()
                .map(|(i, x)| {
                    let sp = mhz.get(i).and_then(Value::as_f64).map(|m| format!(" · {:.1} GHz", m / 1000.0)).unwrap_or_default();
                    gs(format!("Core {i}"), format!("{}{sp}", pct(x.as_f64().unwrap_or(0.0).round())), PURPLE, format!("core:{i}"), 100.0)
                })
                .collect();
            let lim = a.temp_limit.unwrap_or(100.0);
            let tv = if f(&cpu, "temp") > 0.0 { format!("{}°C{}", f(&cpu, "temp").round() as i64, a.temp_limit.map(|l| format!(" · limit {}°C", l as i64)).unwrap_or_default()) } else { "–".into() };
            let mut t0 = gs("Temperature", tv, HOT, "ctemp", lim as f32);
            t0.span = true;
            v.push(t0);
            v
        }
        Tab::Memory => vec![
            gs("Used memory", pct(mem_pct.round()), PURPLE, "mem", f(&mb, "total").max(1.0) as f32),
            gs("Compressed (zram)", d.get("zram").map(|z| size(f(z, "orig"))).unwrap_or_else(|| "off".into()), TEAL, "memz", 0.0),
            gs("Memory pressure", pct(f(&d, "pressure")), GREEN, "mempsi", 100.0),
            gs(
                "Swapping in / out",
                d.get("swapIO").map(|x| format!("{:.0} / {:.0} MB/s", f(x, "in") / 1048576.0, f(x, "out") / 1048576.0)).unwrap_or_else(|| "0 / 0 MB/s".into()),
                YELLOW,
                "swapio",
                0.0,
            ),
        ],
        Tab::Disk => {
            let mut v: Vec<GSpec> = arr(&d, "disks").iter().take(3).map(|x| gs(drive_name(&merge_drive(a, x)), format!("{} busy", pct(f(x, "active"))), ORANGE, format!("disk:{}", s(x, "name")), 100.0)).collect();
            let r = last(h, "resp");
            v.push(gs("Response time", if r > 0.0 { format!("{} ms", trim0(format!("{r:.1}"))) } else { "–".into() }, GREEN, "resp", -1.0));
            v
        }
        Tab::Network => {
            let mut v: Vec<GSpec> = arr(&d, "nets")
                .iter()
                .filter(|n| n.get("up").and_then(Value::as_bool).unwrap_or(false))
                .take(2)
                .map(|n| {
                    let ts0 = s(n, "kind") == "Tailscale";
                    let title = if ts0 { "Tailscale".to_string() } else { format!("{}{}", s(n, "kind"), a.ssid.get(s(n, "name")).map(|x| format!(" {x}")).unwrap_or_default()) };
                    let tail = if ts0 { if a.ts_peers >= 0 { format!(" · {} devices", a.ts_peers) } else { String::new() } } else { fo(n, "speed").map(|sp| format!(" · {} Mbps link", sp as i64)).unwrap_or_default() };
                    gs(title, format!("{}{tail}", rate(f(n, "recv") + f(n, "send"))), if ts0 { TEAL } else { GREEN }, format!("net:{}", s(n, "name")), 0.0)
                })
                .collect();
            let pg = last(h, "ping");
            v.push(gs("Ping", if pg > 0.0 { format!("{} ms", pg.round() as i64) } else { "–".into() }, YELLOW, "ping", -1.0));
            v
        }
        Tab::Gpu => vec![
            gs("3D", pct(last(h, "g3d") as f64), PINK, "g3d", 100.0),
            gs("Video decode", pct(last(h, "gdec").round() as f64), TEAL, "gdec", 100.0),
            gs("Video memory", size(last(h, "vram") as f64), PURPLE, "vram", { let t0 = if f(&g0, "vramTotal") > 0.0 { f(&g0, "vramTotal") } else { f(&g0, "gttTotal") }; t0 as f32 }),
            gs("Temperature", if last(h, "gtemp") > 0.0 { format!("{}°C", last(h, "gtemp").round() as i64) } else { "–".into() }, HOT, "gtemp", 100.0),
        ],
        _ => {
            // Overview: Processor, Memory, the busiest drive, Network, Graphics (mockup resmon-1)
            let base = fo(&cpu, "base").unwrap_or(0.0);
            let freq = if base > 0.0 && f(&cpu, "mhz") > 0.0 { format!(" · {}", pct((100.0 * f(&cpu, "mhz") / base).round())) } else { String::new() };
            let ds = arr(&d, "disks");
            let sys = ds.iter().find(|x| x.get("system").and_then(Value::as_bool) == Some(true)).or_else(|| ds.first()).cloned().unwrap_or(Value::Null);
            let letter = arr(&sys, "letters").iter().filter_map(Value::as_str).next().map(|l| format!("Disk {l}:")).unwrap_or_else(|| "Disk".into());
            let pr = fo(&d, "pressure").unwrap_or(0.0);
            vec![
                gs("Processor", format!("{}{freq}", pct(f(&cpu, "total").round())), PURPLE, "cpu", 100.0),
                gs("Memory", format!("{} · pressure {}", pct(mem_pct.round()), pct(pr.round())), TEAL, "mem", f(&mb, "total").max(1.0) as f32),
                gs(letter, rate(f(&sys, "read") + f(&sys, "write")), ORANGE, format!("diskio:{}", s(&sys, "name")), 0.0),
                gs("Network", rate(last(h, "net") as f64), GREEN, "net", 0.0),
                gs("Graphics", pct(last(h, "gpu").round() as f64), PINK, "gpu", 100.0),
            ]
        }
    }
}

/// A sampler disk row with the drive's letters from Drives (when read) for its name.
fn merge_drive(a: &App, x: &Value) -> Value {
    let mut v = x.clone();
    if let Some(dv) = a.drives.iter().find(|d0| s(d0, "name") == s(x, "name")) {
        if let (Some(o), Some(m)) = (v.as_object_mut(), dv.get("mounts")) {
            o.entry("mounts").or_insert(m.clone());
        }
    }
    v
}

fn graph_card<'a>(a: &'a App, g: GSpec) -> E<'a> {
    let p = a.p;
    let vals: Vec<f32> = a.hist.get(&g.key).map(|q| q.iter().copied().collect()).unwrap_or_default();
    let sp = Spark { values: vals, top: g.top, color: col(g.color), grid: Pal::a(p.line, 0.22), gen: a.samples };
    let vs = if a.tab == Tab::Cpu && !g.span { 13.0 } else { 14.0 };
    let head = row![ts(g.title, 14.0, p.text).wrapping(text::Wrapping::None), ui::fill(), t1(g.value, vs, Pal::mix(p.text, p.muted, 0.35))].spacing(8.0).align_y(iced::Center);
    container(column![head, canvas::Canvas::new(sp).width(Length::Fill).height(Length::Fill)].spacing(8.0))
        .padding(Padding { top: 10.0, right: 12.0, bottom: 10.0, left: 12.0 })
        .width(Length::Fill)
        .height(Length::Fixed(106.0))
        .style(ui::boxed(Pal::mix(p.bg, p.panel, 0.45), p.r(8.0), Some((p.line, 1.0))))
        .into()
}

fn graphs(a: &App) -> E<'_> {
    let specs = graph_specs(a);
    let mut c0 = Column::new().spacing(12.0);
    if a.tab == Tab::Cpu {
        let mut pair: Vec<E> = Vec::new();
        for g in specs {
            if g.span {
                c0 = c0.push(graph_card(a, g));
                continue;
            }
            pair.push(graph_card(a, g));
            if pair.len() == 2 {
                let b = pair.pop().unwrap();
                let a0 = pair.pop().unwrap();
                c0 = c0.push(row![a0, b].spacing(12.0));
            }
        }
        if let Some(x) = pair.pop() {
            c0 = c0.push(row![x, Space::new().width(Length::Fill)].spacing(12.0));
        }
        // more than eight cores: the graphs scroll
        return scrollable(c0).height(Length::Fill).into();
    }
    for g in specs {
        c0 = c0.push(graph_card(a, g));
    }
    c0.into()
}
