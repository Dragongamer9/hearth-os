//! Task Manager's pages: the title bar with search, the rail, each page, the right-click menu and the sheets.

use std::collections::VecDeque;

use hearth_ui::ui::{self, btn, chip, gap, glyph, hline, t, t1, tm, ts, Graph, Kind, TitleMsgs, G};
use hearth_ui::Pal;
use iced::widget::{column, container, mouse_area, opaque, pin, row, scrollable, stack, text, text_input, Column, Row, Space};
use iced::{mouse, Color, Element, Length, Padding};
use serde_json::Value;

use crate::sample::{Group, Sample};
use crate::{App, Msg, Page, SheetKind};

// ------------------------------------------------------------------ numbers in words
pub fn pct1(v: f32) -> String {
    format!("{:.1}%", v.max(0.0))
}
pub fn mb(b: u64) -> String {
    format!("{:.1} MB", b as f64 / 1048576.0)
}
pub fn rate(b: u64) -> String {
    let m = b as f64 / 1048576.0;
    if m < 0.1 { "0 MB/s".into() } else { format!("{m:.1} MB/s") }
}
fn gb(b: u64) -> String {
    format!("{:.1} GB", b as f64 / 1073741824.0)
}
fn kbps(bytes: u64) -> String {
    let bits = bytes as f64 * 8.0;
    if bits >= 1e6 { format!("{:.1} Mbps", bits / 1e6) } else { format!("{:.0} Kbps", bits / 1e3) }
}
fn uptime(s: u64) -> String {
    format!("{}:{:02}:{:02}:{:02}", s / 86400, (s / 3600) % 24, (s / 60) % 60, s % 60)
}
/// Power usage the Windows way, from CPU and graphics use.
pub fn power_of(cpu: f32, gpu: f32, suspended: bool) -> &'static str {
    if suspended {
        return "None";
    }
    let u = cpu + gpu / 2.0;
    if u < 1.0 { "Very low" } else if u < 5.0 { "Low" } else if u < 15.0 { "Moderate" } else if u < 40.0 { "High" } else { "Very high" }
}
/// 0 (barely used) to 5 (very busy), as hearth-tasks' heat map.
pub fn heat(kind: &str, v: f32) -> usize {
    let th: [f32; 5] = match kind {
        "mem" => [0.5, 2.0, 5.0, 10.0, 25.0],
        "disk" => [0.1, 1.0, 10.0, 50.0, 150.0],
        _ => [0.5, 5.0, 15.0, 30.0, 60.0],
    };
    th.iter().filter(|x| v >= **x).count()
}
fn heat_bg(p: &Pal, level: usize) -> Color {
    if level == 0 {
        return Color::TRANSPARENT;
    }
    let warm = if p.dark { Color::from_rgb8(0xf2, 0xc1, 0x4e) } else { Color::from_rgb8(0xff, 0xc8, 0x3d) };
    Pal::mix(p.bg, warm, 0.07 + 0.09 * level as f32)
}

// ------------------------------------------------------------------ the frame
pub fn view(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    let search: Element<Msg> = container(
        row![
            glyph(G::Search, p.muted, 14.0, 2.0),
            text_input("Type a name, publisher or PID to search", &a.query).on_input(Msg::Query).size(13.0).padding(Padding { top: 6.0, right: 6.0, bottom: 6.0, left: 4.0 }).style(move |_t: &iced::Theme, _s: text_input::Status| text_input::Style {
                background: iced::Background::Color(Color::TRANSPARENT),
                border: iced::Border::default(),
                icon: p.muted,
                placeholder: p.muted,
                value: p.text,
                selection: Pal::a(p.accent, 0.35),
            }),
        ]
        .spacing(6.0)
        .align_y(iced::Center)
        .padding(Padding { top: 0.0, right: 6.0, bottom: 0.0, left: 10.0 }),
    )
    .width(Length::Fixed(440.0))
    .style(ui::boxed(p.panel2, p.r(6.0), Some((p.line, 1.0))))
    .into();
    let title = ui::title_bar(&p, G::Tasks, "Task Manager", Some(search), TitleMsgs { drag: Msg::Drag, maximize: Msg::Maximize, minimize: Some(Msg::Minimize), close: Msg::Close }, Pal::mix(p.bg, Color::BLACK, 0.25));

    let body = row![rail(a), ui::vline(p.line), container(page(a)).width(Length::Fill).height(Length::Fill)].height(Length::Fill);
    let base: Element<Msg> = container(column![title, hline(p.line), body]).width(Length::Fill).height(Length::Fill).style(ui::boxed(p.bg, 0.0, None)).into();

    // The cursor's place (where a right-click menu opens): only while the pointer moves over the window.
    let base: Element<Msg> = mouse_area(base).on_move(Msg::Cursor).into();
    let base = match &a.menu {
        Some(m) => {
            let x = m.at.x.min(900.0);
            let y = m.at.y;
            let catcher = mouse_area(Space::new().width(Length::Fill).height(Length::Fill)).on_press(Msg::CloseMenu).on_right_press(Msg::CloseMenu);
            stack![base, catcher, pin(opaque(menu(a, m))).x(x).y(y)].into()
        }
        None => base,
    };
    let base = match &a.sheet {
        Some(_) => ui::dialog(base, sheet(a), None),
        None => base,
    };
    ui::edges(base, Msg::Resize)
}

fn rail(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    let folded = a.pref_b("railFolded");
    let item = |pg: Page, g: G| -> Element<'static, Msg> {
        let on = a.page == pg;
        let fg = if on { p.text } else { p.muted };
        let inner: Element<'static, Msg> = if folded {
            container(glyph(g, if on { p.accent } else { p.muted }, 18.0, 2.0)).center_x(Length::Fixed(40.0)).into()
        } else {
            row![glyph(g, if on { p.accent } else { p.muted }, 18.0, 2.0), gap(12.0, 1.0), if on { ts(pg.label(), 13.0, fg).wrapping(text::Wrapping::None) } else { t1(pg.label(), 13.0, fg) }]
                .align_y(iced::Center)
                .into()
        };
        let mark = container(Space::new()).width(Length::Fixed(3.0)).height(Length::Fixed(16.0)).style(ui::boxed(if on { p.accent } else { Color::TRANSPARENT }, 1.5, None));
        mouse_area(
            container(row![mark, gap(9.0, 1.0), inner].align_y(iced::Center))
                .center_y(Length::Fixed(38.0))
                .width(Length::Fill)
                .style(ui::boxed(if on { p.panel2 } else { Color::TRANSPARENT }, p.r(6.0), None)),
        )
        .on_press(Msg::Page(pg))
        .interaction(mouse::Interaction::Pointer)
        .into()
    };
    let fold = mouse_area(container(glyph(G::List, p.muted, 18.0, 2.0)).padding(Padding { top: 8.0, right: 0.0, bottom: 8.0, left: 14.0 }))
        .on_press(Msg::Pref("railFolded", Value::Bool(!folded)))
        .interaction(mouse::Interaction::Pointer);
    let col = column![
        fold,
        item(Page::Processes, G::Grid),
        item(Page::Performance, G::Pulse),
        item(Page::History, G::Clock),
        item(Page::Startup, G::Rocket),
        item(Page::Users, G::Users),
        item(Page::Details, G::List),
        item(Page::Services, G::Cog),
        item(Page::Tasks, G::Clock),
        Space::new().height(Length::Fill),
        item(Page::Settings, G::Gear),
    ]
    .spacing(2.0)
    .padding(6.0);
    container(col).width(Length::Fixed(if folded { 64.0 } else { 210.0 })).height(Length::Fill).style(ui::boxed(Pal::mix(p.bg, p.panel, 0.4), 0.0, None)).into()
}

/// A page's head: its title, a line under it, tabs and its buttons on the right.
fn head<'a>(a: &'a App, title: &'a str, sub: String, tabs: Option<Element<'a, Msg>>, buttons: Vec<Element<'a, Msg>>) -> Element<'a, Msg> {
    let p = a.p;
    let mut left = Column::new().spacing(2.0).push(ts(title, 20.0, p.text));
    if !sub.is_empty() {
        left = left.push(t1(sub, 12.0, p.muted));
    }
    let mut r = Row::new().spacing(6.0).align_y(iced::Center).push(container(left).width(Length::Fill));
    if let Some(tb) = tabs {
        r = r.push(tb);
        r = r.push(gap(12.0, 1.0));
    }
    for b in buttons {
        r = r.push(b);
    }
    container(r).padding(Padding { top: 14.0, right: 20.0, bottom: 10.0, left: 22.0 }).width(Length::Fill).into()
}

fn tabs<'a>(a: &App, list: &[(&'static str, &'static str)], cur: &str, on: fn(&'static str) -> Msg) -> Element<'a, Msg> {
    let p = a.p;
    let mut r = Row::new().spacing(2.0);
    for (id, label) in list {
        let sel = *id == cur;
        r = r.push(
            mouse_area(
                container(if sel { ts(*label, 13.0, p.text) } else { t(*label, 13.0, p.muted) })
                    .padding(Padding { top: 6.0, right: 12.0, bottom: 6.0, left: 12.0 })
                    .style(ui::boxed(if sel { p.panel2 } else { Color::TRANSPARENT }, p.r(6.0), None)),
            )
            .on_press(on(*id))
            .interaction(mouse::Interaction::Pointer),
        );
    }
    container(r).padding(2.0).style(ui::boxed(p.bg2, p.r(8.0), Some((p.line, 1.0)))).into()
}

fn page(a: &App) -> Element<'_, Msg> {
    match a.page {
        Page::Processes => processes(a),
        Page::Performance => performance(a),
        Page::Details => details(a),
        Page::Settings => settings(a),
        _ => list_page(a),
    }
}

// ------------------------------------------------------------------ tables
#[derive(Clone, Copy)]
pub struct Col {
    pub key: &'static str,
    pub title: &'static str,
    pub width: f32,
    pub right: bool,
    pub total: bool,
}
const fn col(key: &'static str, title: &'static str, width: f32, right: bool) -> Col {
    Col { key, title, width, right, total: false }
}
const fn tot(key: &'static str, title: &'static str, width: f32) -> Col {
    Col { key, title, width, right: true, total: true }
}

pub struct Cell {
    pub text: String,
    pub heat: usize,
    pub muted: bool,
}
fn cell(s: impl Into<String>) -> Cell {
    Cell { text: s.into(), heat: 0, muted: false }
}
fn hcell(s: impl Into<String>, h: usize) -> Cell {
    Cell { text: s.into(), heat: h, muted: false }
}
fn mcell(s: impl Into<String>) -> Cell {
    Cell { text: s.into(), heat: 0, muted: true }
}

fn width_of(c: &Col) -> Length {
    if c.width <= 0.0 { Length::Fill } else { Length::Fixed(c.width) }
}

/// The column heads: click to sort (again: the other way). Totals over CPU, Memory, Disk, Network, GPU (Windows).
fn heads<'a>(a: &'a App, table: &'static str, cols: &[Col], totals: Option<&Sample>) -> Element<'a, Msg> {
    let p = a.p;
    let (sk, sd) = a.sort_of(table);
    let mut r = Row::new().padding(Padding { top: 0.0, right: 20.0, bottom: 0.0, left: 22.0 });
    for c in cols {
        let arrow = if sk == c.key { if sd { " ▾" } else { " ▴" } } else { "" };
        let mut inner = Column::new();
        if c.total {
            let v = totals
                .map(|s| match c.key {
                    "cpu" => format!("{:.0}%", s.cpu.total),
                    "mem" => format!("{:.0}%", 100.0 * s.mem.used as f32 / s.mem.total.max(1) as f32),
                    "disk" => format!("{:.0}%", s.disks.iter().map(|d| d.active).fold(0.0, f32::max)),
                    "net" => format!("{:.0}%", 0.0),
                    "gpu" => format!("{:.0}%", s.gpus.iter().filter_map(|g| g.busy).fold(0.0, f32::max)),
                    _ => String::new(),
                })
                .unwrap_or_default();
            inner = inner.push(container(ts(v, 15.0, p.text)).align_right(Length::Fill));
        }
        let label = format!("{}{arrow}", c.title);
        inner = inner.push(if c.right { container(t1(label, 12.0, p.muted)).align_right(Length::Fill) } else { container(t1(label, 12.0, p.muted)) });
        let desc = !matches!(c.key, "name" | "pid" | "status" | "user" | "publisher" | "desc" | "cmd" | "group" | "starts" | "next" | "last" | "runs" | "ppid");
        r = r.push(
            mouse_area(container(inner).width(width_of(c)).padding(Padding { top: 4.0, right: 8.0, bottom: 4.0, left: 8.0 }).clip(true))
                .on_press(Msg::Sort(table, c.key, desc))
                .interaction(mouse::Interaction::Pointer),
        );
    }
    column![r.align_y(iced::Bottom), hline(p.line)].into()
}

/// One row: its cells (the first can carry a chevron, an icon mark and a tag), selected and heat-shaded.
fn row_view<'a>(a: &'a App, key: String, cols: &[Col], cells: Vec<Cell>, lead: Option<(Option<bool>, usize, String, String)>, h: f32) -> Element<'a, Msg> {
    let p = a.p;
    let selected = a.sel.as_deref() == Some(key.as_str());
    let mut r = Row::new().align_y(iced::Center).padding(Padding { top: 0.0, right: 20.0, bottom: 0.0, left: 22.0 });
    for (i, (c, v)) in cols.iter().zip(cells.into_iter()).enumerate() {
        let fg = if v.muted { p.muted } else { p.text };
        let mut content: Element<'a, Msg> = if c.right { container(t1(v.text, 13.0, fg)).align_right(Length::Fill).into() } else { t1(v.text, 13.0, fg).into() };
        if i == 0 {
            if let Some((exp, depth, tag, icon)) = &lead {
                let mut lr = Row::new().spacing(6.0).align_y(iced::Center).push(gap(*depth as f32 * 22.0, 1.0));
                match exp {
                    Some(open) => {
                        let gid = key.split('/').next().unwrap_or("").to_string();
                        lr = lr.push(mouse_area(glyph(if *open { G::Down } else { G::Chevron }, p.muted, 14.0, 2.2)).on_press(Msg::Toggle(gid)).interaction(mouse::Interaction::Pointer));
                    }
                    None => lr = lr.push(gap(14.0, 1.0)),
                }
                // the app's icon (its .desktop file's, from the icon themes)
                if !icon.is_empty() {
                    lr = lr.push(app_icon(icon));
                }
                lr = lr.push(content);
                if !tag.is_empty() {
                    let (bg, fgc) = match tag.as_str() {
                        "AI" => (Pal::mix(p.panel, Color::from_rgb8(0x9b, 0x7b, 0xff), 0.35), Color::from_rgb8(0xc9, 0xb8, 0xff)),
                        "Game · Proton" | "Steam" | "Windows app" => (Pal::mix(p.panel, p.good, 0.3), p.good),
                        _ => (Pal::mix(p.panel, p.accent, 0.3), Pal::mix(p.accent, p.text, 0.3)),
                    };
                    lr = lr.push(chip(tag.clone(), bg, fgc));
                }
                content = lr.into();
            }
        }
        r = r.push(container(content).width(width_of(c)).height(Length::Fixed(h)).center_y(Length::Fixed(h)).padding(Padding { top: 0.0, right: 8.0, bottom: 0.0, left: 8.0 }).clip(true).style(ui::boxed(heat_bg(&p, v.heat), 0.0, None)));
    }
    let bg = if selected { p.sel() } else { Color::TRANSPARENT };
    mouse_area(container(r).width(Length::Fill).style(ui::boxed(bg, 0.0, if selected { Some((p.accent, 1.0)) } else { None })))
        .on_press(Msg::Select(key.clone()))
        .on_right_press(Msg::RowMenu(key.clone()))
        .on_double_click(Msg::Toggle(key))
        .into()
}

/// An app's icon, 18 px: PNG through iced's image widget, SVG through its svg widget.
fn app_icon<'a>(path: &str) -> Element<'a, Msg> {
    let sz = Length::Fixed(18.0);
    if path.ends_with(".svg") {
        iced::widget::svg(iced::widget::svg::Handle::from_path(path)).width(sz).height(sz).into()
    } else {
        iced::widget::image(iced::widget::image::Handle::from_path(path)).width(sz).height(sz).into()
    }
}

fn section<'a>(p: &Pal, label: String) -> Element<'a, Msg> {
    container(ts(label, 13.0, p.text)).padding(Padding { top: 12.0, right: 0.0, bottom: 4.0, left: 30.0 }).into()
}

fn sort_rows<T>(rows: &mut [T], key: &str, desc: bool, num: impl Fn(&T, &str) -> f64, txt: impl Fn(&T, &str) -> String) {
    if key.is_empty() {
        return;
    }
    let textual = matches!(key, "name" | "status" | "user" | "publisher" | "desc" | "cmd" | "group" | "starts" | "runs" | "next" | "last" | "result" | "impact" | "added");
    rows.sort_by(|x, y| {
        let o = if textual { txt(x, key).to_lowercase().cmp(&txt(y, key).to_lowercase()) } else { num(x, key).partial_cmp(&num(y, key)).unwrap_or(std::cmp::Ordering::Equal) };
        let o = o.then_with(|| txt(x, "name").to_lowercase().cmp(&txt(y, "name").to_lowercase()));
        if desc { o.reverse() } else { o }
    });
}

// ------------------------------------------------------------------ Processes (mockup tm-1)
const PROC_COLS: [Col; 11] = [
    col("name", "Name", 0.0, false),
    col("pid", "PID", 76.0, true),
    col("status", "Status", 150.0, false),
    tot("cpu", "CPU", 80.0),
    tot("mem", "Memory", 100.0),
    col("swap", "Compressed", 104.0, true),
    tot("disk", "Disk", 90.0),
    tot("net", "Network", 86.0),
    tot("gpu", "GPU", 76.0),
    col("power", "Power", 90.0, true),
    col("gpuengine", "", 0.0, false),
];

fn status_of(g: &Group) -> String {
    if g.suspended {
        if g.frozen { "Paused to save memory ⏸".into() } else if g.kind == "game" { "Quick Resume ⏸".into() } else { "Suspended ⏸".into() }
    } else if g.eco {
        "Efficiency mode".into()
    } else {
        String::new()
    }
}

fn matches(g: &Group, q: &str) -> bool {
    if q.is_empty() {
        return true;
    }
    g.name.to_lowercase().contains(q)
        || g.exe.to_lowercase().contains(q)
        || g.desktop.to_lowercase().contains(q)
        || g.tag.to_lowercase().contains(q)
        || g.pids.iter().any(|p| p.to_string().starts_with(q))
        || g.children.iter().any(|c| c.name.to_lowercase().contains(q))
}

fn group_num(g: &Group, k: &str) -> f64 {
    match k {
        "pid" => g.pid as f64,
        "cpu" => g.cpu as f64,
        "mem" => g.mem as f64,
        "swap" => g.swap as f64,
        "disk" => g.disk as f64,
        "gpu" => g.gpu as f64,
        "power" => (g.cpu + g.gpu / 2.0) as f64,
        _ => 0.0,
    }
}

pub struct ProcRow {
    pub key: String,
    pub cells: Vec<Cell>,
    pub lead: (Option<bool>, usize, String, String),
    pub section: Option<String>,
}

pub fn proc_rows(a: &App) -> Vec<ProcRow> {
    let Some(s) = &a.sample else { return vec![] };
    let q = a.query.trim().to_lowercase();
    let (sk, sd) = a.sort_of("procs");
    let total = s.mem.total.max(1) as f32;
    let cells_for = |pid: i32, status: String, cpu: f32, mem: u64, swap: u64, disk: u64, gpu: f32, power: &str, name: String| -> Vec<Cell> {
        vec![
            cell(name),
            mcell(if pid > 0 { pid.to_string() } else { String::new() }),
            cell(status),
            hcell(pct1(cpu), heat("cpu", cpu)),
            hcell(mb(mem), heat("mem", 100.0 * mem as f32 / total)),
            Cell { text: if swap > 0 { mb(swap) } else { String::new() }, heat: 0, muted: swap < 50 * 1048576 },
            hcell(rate(disk), heat("disk", disk as f32 / 1048576.0)),
            mcell("–"),
            hcell(pct1(gpu), heat("gpu", gpu)),
            Cell { text: power.to_string(), heat: 0, muted: power == "Very low" || power.is_empty() },
            mcell(""),
        ]
    };
    let mut out = Vec::new();
    if a.pref_s("procView", "groups") == "all" {
        let mut flat: Vec<(String, crate::sample::Kid, bool)> = Vec::new();
        for g in &s.groups {
            let kids = if g.children.is_empty() {
                vec![crate::sample::Kid { pid: g.pid, name: g.name.clone(), cpu: g.cpu, mem: g.mem, swap: g.swap, disk: g.disk, gpu: g.gpu, state: if g.suspended { 'T' } else { 'S' }, nice: g.nice, ..Default::default() }]
            } else {
                g.children.clone()
            };
            for c in kids {
                if !q.is_empty() && !c.name.to_lowercase().contains(&q) && !c.pid.to_string().starts_with(&q) && !g.name.to_lowercase().contains(&q) {
                    continue;
                }
                flat.push((g.id.clone(), c, g.protected));
            }
        }
        let key = if sk.is_empty() || sk == "status" { "mem".to_string() } else { sk.clone() };
        let desc = if sk.is_empty() { true } else { sd };
        sort_rows(
            &mut flat,
            &key,
            desc,
            |x, k| match k {
                "pid" => x.1.pid as f64,
                "cpu" => x.1.cpu as f64,
                "mem" => x.1.mem as f64,
                "swap" => x.1.swap as f64,
                "disk" => x.1.disk as f64,
                "gpu" => x.1.gpu as f64,
                _ => 0.0,
            },
            |x, _| x.1.name.clone(),
        );
        for (_, c, _) in flat {
            let tag = if c.ai { "AI" } else if c.bridge { "bridge" } else { "" };
            let susp = matches!(c.state, 'T' | 't');
            let status = if susp { "Suspended ⏸".to_string() } else if c.nice >= 19 { "Efficiency mode".into() } else { String::new() };
            out.push(ProcRow {
                key: format!("a/{}", c.pid),
                cells: cells_for(c.pid, status, c.cpu, c.mem, c.swap, c.disk, c.gpu, power_of(c.cpu, c.gpu, susp), c.name.clone()),
                lead: (None, 0, tag.into(), String::new()),
                section: None,
            });
        }
        return out;
    }
    let mut list: Vec<&Group> = s.groups.iter().filter(|g| matches(g, &q)).collect();
    if !sk.is_empty() {
        sort_rows(&mut list, &sk, sd, |g, k| group_num(g, k), |g, k| if k == "status" { status_of(g) } else { g.name.clone() });
    }
    let apps: Vec<&Group> = list.iter().copied().filter(|g| g.kind == "app" || g.kind == "game").collect();
    let hearth: Vec<&Group> = list.iter().copied().filter(|g| g.kind == "system").collect();
    let bg: Vec<&Group> = list.iter().copied().filter(|g| !(g.kind == "app" || g.kind == "game" || g.kind == "system")).collect();
    for (label, gs) in [("Apps", apps), ("Hearth", hearth), ("Background processes", bg)] {
        if gs.is_empty() {
            continue;
        }
        let mut first = true;
        for g in gs.iter() {
            let open = a.expanded.contains(&g.id) && g.children.len() > 1;
            let tag = if !g.tag.is_empty() { g.tag.clone() } else if g.kind == "system" { "Hearth".into() } else { String::new() };
            let tag = if tag == "Part of Hearth" { "Hearth".into() } else { tag };
            let name = if g.children.len() > 1 { format!("{} ({})", g.name, g.children.len()) } else { g.name.clone() };
            out.push(ProcRow {
                key: g.id.clone(),
                cells: cells_for(g.pid, status_of(g), g.cpu, g.mem, g.swap, g.disk, g.gpu, power_of(g.cpu, g.gpu, g.suspended), name),
                lead: (if g.children.len() > 1 { Some(open) } else { None }, 0, tag, g.icon_path.clone()),
                section: if first { Some(format!("{label} ({})", gs.len())) } else { None },
            });
            first = false;
            if open {
                let mut kids = g.children.clone();
                if !sk.is_empty() && sk != "status" {
                    sort_rows(&mut kids, &sk, sd, |c, k| match k { "pid" => c.pid as f64, "cpu" => c.cpu as f64, "mem" => c.mem as f64, "disk" => c.disk as f64, "gpu" => c.gpu as f64, _ => 0.0 }, |c, _| c.name.clone());
                }
                for c in kids {
                    let tag = if c.ai { "AI" } else if c.bridge { "bridge" } else { "" };
                    let st = if matches!(c.state, 'T' | 't') { "Suspended ⏸".to_string() } else { String::new() };
                    out.push(ProcRow {
                        key: format!("{}/{}", g.id, c.pid),
                        cells: cells_for(c.pid, st, c.cpu, c.mem, c.swap, c.disk, c.gpu, "", format!("{}  ·  {}", c.name, c.pid)),
                        lead: (None, 1, tag.into(), String::new()),
                        section: None,
                    });
                }
            }
        }
    }
    out
}

/// The keys of the rows on show, in order (Up / Down).
pub fn row_keys(a: &App) -> Vec<String> {
    match a.page {
        Page::Processes => proc_rows(a).into_iter().map(|r| r.key).collect(),
        Page::Details => details_rows(a).into_iter().map(|r| r.pid.to_string()).collect(),
        _ => list_rows(a).into_iter().map(|r| r.0).collect(),
    }
}

fn processes(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    let tgt = a.sel.as_ref().and_then(|k| a.group(k.split('/').next().unwrap_or("")).cloned());
    let can_end = tgt.as_ref().map(|g| !g.protected).unwrap_or(false) || a.sel.as_deref().map(|k| k.starts_with("a/")).unwrap_or(false);
    let is_eco = tgt.as_ref().map(|g| g.eco).unwrap_or(false);
    let has_exe = tgt.as_ref().map(|g| !g.exe.is_empty()).unwrap_or(false);
    let view_id = a.pref_s("procView", "groups");
    let buttons = vec![
        btn(&p, "Run new task", Kind::Primary, false, Some(Msg::RunNew)),
        btn(&p, if is_eco { "Efficiency mode ✓" } else { "Efficiency mode" }, Kind::Normal, false, if can_end { Some(Msg::TopEco) } else { None }),
        btn(&p, "Open file location", Kind::Normal, false, if has_exe { Some(Msg::TopWhere) } else { None }),
        btn(&p, "End task", Kind::Normal, false, if can_end { Some(Msg::TopEnd) } else { None }),
    ];
    let tabs_el = tabs(a, &[("groups", "Apps & groups"), ("all", "All processes")], if view_id == "all" { "all" } else { "groups" }, Msg::ProcView);
    let mut list = Column::new();
    let rows = proc_rows(a);
    if a.sample.is_none() {
        list = list.push(container(t("Measuring…", 13.0, p.muted)).padding(22.0));
    }
    for r in rows {
        if let Some(sec) = r.section {
            list = list.push(section(&p, sec));
        }
        list = list.push(row_view(a, r.key, &PROC_COLS[..10], r.cells, Some(r.lead), 34.0));
    }
    column![
        head(a, "Processes", String::new(), Some(tabs_el), buttons),
        heads(a, "procs", &PROC_COLS[..10], a.sample.as_deref()),
        scrollable(list).height(Length::Fill)
    ]
    .into()
}

// ------------------------------------------------------------------ Details (mockup tm-2)
pub const DETAIL_COLS: [Col; 13] = [
    col("name", "Name", 200.0, false),
    col("pid", "PID", 72.0, true),
    col("ppid", "Parent", 72.0, true),
    col("status", "Status", 110.0, false),
    col("user", "User name", 110.0, false),
    col("cpu", "CPU", 60.0, true),
    col("cputime", "CPU time", 86.0, true),
    col("mem", "Memory", 100.0, true),
    col("swap", "Swapped", 90.0, true),
    col("threads", "Threads", 70.0, true),
    col("nice", "Priority", 106.0, true),
    col("cores", "Cores", 64.0, true),
    col("cmd", "Command line", 0.0, false),
];

/// The columns you hid with Columns… (remembered); before that, only Command line could be hidden.
pub fn details_hidden(a: &App) -> Vec<String> {
    match a.prefs.get("detailsHidden").and_then(Value::as_array) {
        Some(l) => l.iter().filter_map(Value::as_str).map(String::from).collect(),
        None if a.pref_b("hide_details") => vec!["cmd".into()],
        None => vec![],
    }
}

fn prio(n: i32) -> &'static str {
    match n {
        i32::MIN..=-11 => "High",
        -10..=-1 => "Above normal",
        0 => "Normal",
        1..=9 => "Below normal",
        _ => "Low",
    }
}

fn details_rows(a: &App) -> Vec<crate::sample::Row> {
    let Some(s) = &a.sample else { return vec![] };
    let q = a.query.trim().to_lowercase();
    let mut rows: Vec<crate::sample::Row> = s
        .details
        .iter()
        .filter(|r| q.is_empty() || r.name.to_lowercase().contains(&q) || r.pid.to_string().starts_with(&q) || r.cmd.to_lowercase().contains(&q) || r.user.to_lowercase().contains(&q))
        .cloned()
        .collect();
    let (sk, sd) = a.sort_of("details");
    sort_rows(
        &mut rows,
        &sk,
        sd,
        |r, k| match k {
            "pid" => r.pid as f64,
            "ppid" => r.ppid as f64,
            "cpu" => r.cpu as f64,
            "cputime" => r.cputime as f64,
            "mem" => r.mem as f64,
            "swap" => r.swap as f64,
            "threads" => r.threads as f64,
            "nice" => r.nice as f64,
            _ => 0.0,
        },
        |r, k| match k {
            "status" => r.status.to_string(),
            "user" => r.user.clone(),
            "cmd" => r.cmd.clone(),
            _ => r.name.clone(),
        },
    );
    rows
}

fn details(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    let sel_mine = a.sel.as_ref().and_then(|k| k.parse::<i32>().ok()).and_then(|pid| a.sample.as_ref()?.details.iter().find(|r| r.pid == pid).map(|r| r.mine)).unwrap_or(false);
    let buttons = vec![
        btn(&p, "Columns…", Kind::Normal, false, Some(Msg::DetailCols)),
        btn(&p, "End task", Kind::Normal, false, if sel_mine { Some(Msg::TopEnd) } else { None }),
    ];
    // "Columns…": any column but Name on or off (remembered).
    let hidden = details_hidden(a);
    let show: Vec<bool> = DETAIL_COLS.iter().map(|c| c.key == "name" || !hidden.iter().any(|h| h == c.key)).collect();
    let cols: Vec<Col> = DETAIL_COLS.iter().zip(show.iter()).filter(|(_, s)| **s).map(|(c, _)| *c).collect();
    let mut list = Column::new();
    for r in details_rows(a) {
        let cells = vec![
            cell(r.name.clone()),
            mcell(r.pid.to_string()),
            mcell(r.ppid.to_string()),
            cell(r.status),
            if r.mine { cell(r.user.clone()) } else { mcell(r.user.clone()) },
            cell(format!("{:.0}", r.cpu)),
            mcell(hearth_ui::time::hms(r.cputime as f64)),
            cell(format!("{} K", r.mem / 1024)),
            Cell { text: if r.swap > 0 { format!("{} K", r.swap / 1024) } else { String::new() }, heat: 0, muted: true },
            mcell(r.threads.to_string()),
            cell(prio(r.nice)),
            mcell(r.cores.clone()),
            mcell(ui::fit(&r.cmd, 160)),
        ];
        let cells = cells.into_iter().zip(show.iter()).filter(|(_, s)| **s).map(|(c, _)| c).collect();
        list = list.push(row_view(a, r.pid.to_string(), &cols, cells, None, 30.0));
    }
    column![
        head(a, "Details", "every process with its number, the Windows way".into(), None, buttons),
        heads(a, "details", &cols, None),
        scrollable(list).height(Length::Fill)
    ]
    .into()
}

// ------------------------------------------------------------------ App history, Startup apps, Users, Services, Scheduled tasks
fn list_cols(pg: Page) -> Vec<Col> {
    match pg {
        Page::History => vec![col("name", "Name", 0.0, false), col("cpu", "CPU time", 120.0, true), col("read", "Disk read", 120.0, true), col("write", "Disk written", 130.0, true), col("last", "Last used", 170.0, false)],
        Page::Startup => vec![
            col("name", "Name", 0.0, false),
            col("publisher", "Publisher", 200.0, false),
            col("status", "Status", 150.0, false),
            col("impact", "Start-up impact", 150.0, false),
            col("added", "Time added to start", 160.0, true),
            col("starts", "Starts", 180.0, false),
        ],
        Page::Users => vec![col("name", "User", 0.0, false), col("status", "Status", 170.0, false), col("cpu", "CPU", 90.0, true), col("mem", "Memory", 110.0, true), col("procs", "Processes", 100.0, true)],
        Page::Services => vec![col("name", "Name", 230.0, false), col("pid", "PID", 72.0, true), col("desc", "Description", 0.0, false), col("status", "Status", 96.0, false), col("starts", "Starts", 120.0, false), col("group", "Group", 80.0, false)],
        _ => vec![col("name", "Task", 0.0, false), col("next", "When (next)", 170.0, false), col("last", "Last run", 170.0, false), col("result", "Last result", 110.0, false), col("runs", "What it runs", 240.0, false)],
    }
}

fn vs(v: &Value, k: &str) -> String {
    match v.get(k) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Bool(b)) => if *b { "Yes".into() } else { "No".into() },
        _ => String::new(),
    }
}
fn vn(v: &Value, k: &str) -> f64 {
    v.get(k).and_then(Value::as_f64).unwrap_or(0.0)
}

/// (key, cells, muted) for each row of the list page on show.
fn list_rows(a: &App) -> Vec<(String, Vec<Cell>, bool)> {
    let Some(l) = a.page.list() else { return vec![] };
    let Some(v) = a.lists.get(l) else { return vec![] };
    let key = match a.page {
        Page::History => "apps",
        Page::Startup => "items",
        Page::Users => "users",
        Page::Services => "services",
        _ => "timers",
    };
    let mut rows: Vec<Value> = v.get(key).and_then(Value::as_array).cloned().unwrap_or_default();
    let q = a.query.trim().to_lowercase();
    if !q.is_empty() {
        rows.retain(|r| ["name", "unit", "desc", "publisher", "full"].iter().any(|k| vs(r, k).to_lowercase().contains(&q)) || vs(r, "pid").starts_with(&q));
    }
    let (sk, sd) = a.sort_of(a.page.id());
    sort_rows(&mut rows, &sk, sd, |r, k| vn(r, k), |r, k| if k == "name" && vs(r, "name").is_empty() { vs(r, "unit") } else { vs(r, k) });
    rows.iter()
        .enumerate()
        .map(|(i, r)| {
            let id = { let x = vs(r, "id"); if x.is_empty() { format!("{}:{i}", vs(r, "name")) } else { x } };
            let cells = match a.page {
                Page::History => {
                    let last = vn(r, "last") as i64;
                    vec![cell(vs(r, "name")), cell(hearth_ui::time::hms(vn(r, "cpu"))), cell(mb(vn(r, "read") as u64)), cell(mb(vn(r, "write") as u64)), mcell(if last > 0 { hearth_ui::time::when(last) } else { String::new() })]
                }
                Page::Startup => {
                    let on = r.get("on").and_then(Value::as_bool).unwrap_or(false);
                    let added = vn(r, "added");
                    vec![
                        cell(vs(r, "name")),
                        mcell(vs(r, "publisher")),
                        cell(if on { "Enabled" } else { "Disabled" }),
                        cell(vs(r, "impact")),
                        mcell(if added > 0.0 { format!("{added:.1} s") } else { String::new() }),
                        mcell(vs(r, "starts")),
                    ]
                }
                Page::Users => {
                    let you = r.get("you").and_then(Value::as_bool).unwrap_or(false);
                    let full = vs(r, "full");
                    let name = format!("{}{}{}", vs(r, "name"), if full.is_empty() { String::new() } else { format!(" ({full})") }, if you { " · you" } else { "" });
                    let signed = r.get("signedIn").and_then(Value::as_bool).unwrap_or(false);
                    vec![cell(name), cell(if signed { "Signed in" } else { "Not signed in" }), cell(pct1(vn(r, "cpu") as f32)), cell(mb(vn(r, "mem") as u64)), cell(vs(r, "procs"))]
                }
                Page::Services => vec![cell(vs(r, "name")), mcell(vs(r, "pid")), cell(vs(r, "desc")), cell(vs(r, "status")), mcell(vs(r, "starts")), mcell(vs(r, "group"))],
                _ => {
                    let desc = vs(r, "desc");
                    vec![cell(if desc.is_empty() { vs(r, "unit") } else { format!("{desc} ({})", vs(r, "unit")) }), cell(vs(r, "next")), mcell(vs(r, "last")), cell(vs(r, "result")), mcell(vs(r, "runs"))]
                }
            };
            let muted = !r.get("canChange").and_then(Value::as_bool).unwrap_or(true);
            (id, cells, muted)
        })
        .collect()
}

fn list_page(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    let l = a.page.list().unwrap_or("");
    let sel_row = a.sel.clone().and_then(|id| {
        let v = a.lists.get(l)?;
        let key = match a.page {
            Page::Startup => "items",
            Page::Services => "services",
            _ => "timers",
        };
        v.get(key)?.as_array()?.iter().find(|r| r.get("id").and_then(Value::as_str) == Some(id.as_str())).cloned()
    });
    let can = sel_row.as_ref().and_then(|r| r.get("canChange").and_then(Value::as_bool)).unwrap_or(false);
    let sid = a.sel.clone().unwrap_or_default();
    let (sub, buttons): (String, Vec<Element<Msg>>) = match a.page {
        Page::History => {
            let since = a.lists.get("history").and_then(|v| v.get("since")).and_then(Value::as_i64).unwrap_or(0);
            (
                if since > 0 { format!("Processor time and disk use per app since {}", hearth_ui::time::when(since)) } else { "Processor time and disk use per app".into() },
                vec![btn(&p, "Delete usage history", Kind::Normal, false, Some(Msg::ClearHistory))],
            )
        }
        Page::Startup => {
            let on = sel_row.as_ref().and_then(|r| r.get("on").and_then(Value::as_bool)).unwrap_or(false);
            let boot = a.lists.get("startup").and_then(|v| v.get("bootSeconds")).and_then(Value::as_f64);
            (
                boot.map(|b| format!("Last start: {} s (desktop ready)", b.round())).unwrap_or_default(),
                vec![
                    btn(&p, "Enable", Kind::Primary, false, if can && !on { Some(Msg::ListAct("startup", sid.clone(), "on".into())) } else { None }),
                    btn(&p, "Disable", Kind::Normal, false, if can && on { Some(Msg::ListAct("startup", sid.clone(), "off".into())) } else { None }),
                ],
            )
        }
        Page::Users => ("Everyone with an account on this PC, and what their programs use".into(), vec![]),
        Page::Services => {
            let running = sel_row.as_ref().map(|r| vs(r, "status") == "Running").unwrap_or(false);
            (
                "Yours can be started and stopped; the system's are shown read only".into(),
                vec![
                    btn(&p, "Start", Kind::Primary, false, if can && !running { Some(Msg::ListAct("services", sid.clone(), "start".into())) } else { None }),
                    btn(&p, "Stop", Kind::Normal, false, if can && running { Some(Msg::ListAct("services", sid.clone(), "stop".into())) } else { None }),
                    btn(&p, "Restart", Kind::Normal, false, if can && running { Some(Msg::ListAct("services", sid.clone(), "restart".into())) } else { None }),
                ],
            )
        }
        _ => {
            let on = sel_row.as_ref().map(|r| !vs(r, "next").is_empty()).unwrap_or(false);
            (
                "What runs by itself at set times, like Windows' Task Scheduler".into(),
                vec![
                    btn(&p, "+ New task", Kind::Primary, false, Some(Msg::NewTask)),
                    btn(&p, "Turn on", Kind::Normal, false, if can && !on { Some(Msg::ListAct("timers", sid.clone(), "on".into())) } else { None }),
                    btn(&p, "Turn off", Kind::Normal, false, if can && on { Some(Msg::ListAct("timers", sid.clone(), "off".into())) } else { None }),
                ],
            )
        }
    };
    let cols = list_cols(a.page);
    let mut list = Column::new();
    let rows = list_rows(a);
    if !a.lists.contains_key(l) {
        list = list.push(container(t("Reading…", 13.0, p.muted)).padding(22.0));
    } else if rows.is_empty() {
        list = list.push(container(t("Nothing here.", 13.0, p.muted)).padding(22.0));
    }
    for (id, cells, muted) in rows {
        let cells = if muted { cells.into_iter().map(|c| Cell { muted: true, ..c }).collect() } else { cells };
        list = list.push(row_view(a, id, &cols, cells, None, 32.0));
    }
    let table: &'static str = a.page.id();
    column![head(a, a.page.label(), sub, None, buttons), heads(a, table, &cols, None), scrollable(list).height(Length::Fill)].into()
}

// ------------------------------------------------------------------ Performance (mockups tm-3…tm-7, task-manager-memory)
struct Dev {
    key: String,
    title: String,
    sub: String,
    hist: Vec<f32>,
    color: Color,
    max: f32,
}

fn hv(h: Option<&VecDeque<f32>>) -> Vec<f32> {
    h.map(|x| x.iter().copied().collect()).unwrap_or_default()
}

fn devices(a: &App, s: &Sample) -> Vec<Dev> {
    let p = a.p;
    let mut d = vec![
        Dev { key: "cpu".into(), title: "CPU".into(), sub: format!("{}  {}", pct1(s.cpu.total), s.cpu.mhz.map(|m| format!("{:.2} GHz", m as f32 / 1000.0)).unwrap_or_default()), hist: hv(Some(&a.cpu_hist)), color: p.accent, max: 100.0 },
        Dev {
            key: "memory".into(),
            title: "Memory".into(),
            sub: format!("{}/{} ({:.0}%)", gb(s.mem.used), gb(s.mem.total), 100.0 * s.mem.used as f32 / s.mem.total.max(1) as f32),
            hist: hv(Some(&a.mem_hist)),
            color: Color::from_rgb8(0x9b, 0x7b, 0xff),
            max: 100.0,
        },
    ];
    for (i, k) in s.disks.iter().enumerate() {
        d.push(Dev {
            key: format!("disk:{}", k.name),
            title: format!("Disk {i} ({})", k.name),
            sub: format!("{}\n{:.0}%", if k.ssd { "SSD" } else { "HDD" }, k.active),
            hist: hv(a.disk_hist.get(&k.name)),
            color: p.good,
            max: 100.0,
        });
    }
    for n in s.nets.iter().filter(|n| n.up) {
        let h = a.net_hist.get(&n.name).map(|x| x.0.iter().zip(x.1.iter()).map(|(r, s)| r + s).collect()).unwrap_or_default();
        let peak = a.net_hist.get(&n.name).map(|x| x.0.iter().chain(x.1.iter()).fold(1.0f32, |m, v| m.max(*v) * 1.0)).unwrap_or(1.0);
        // Tailscale: its other devices (mockup tm-5: "Tailscale · 3 devices")
        let sub = if n.kind == "Tailscale" {
            if a.ts_peers >= 0 { format!("{} device{}", a.ts_peers, if a.ts_peers == 1 { "" } else { "s" }) } else { "Connected".into() }
        } else {
            format!("S: {} R: {}", kbps(n.send), kbps(n.recv))
        };
        d.push(Dev { key: format!("net:{}", n.name), title: n.kind.clone(), sub, hist: h, color: p.warn, max: peak.max(1.0) * 1.2 });
    }
    for g in &s.gpus {
        d.push(Dev {
            key: format!("gpu:{}", g.index),
            title: format!("GPU {}", g.index),
            sub: format!("{}\n{}{}", g.name, g.busy.map(|b| format!("{b:.0}%")).unwrap_or_default(), g.temp.map(|t| format!(" ({t:.0} °C)")).unwrap_or_default()),
            hist: hv(a.gpu_hist.get(&g.index)),
            color: Color::from_rgb8(0x5a, 0xb8, 0xf0),
            max: 100.0,
        });
    }
    if let Some(b) = &s.battery {
        let left = if b.hours > 0.0 { format!("{} h {} min", b.hours.floor(), ((b.hours % 1.0) * 60.0).round()) } else { b.status.clone() };
        d.push(Dev { key: "battery".into(), title: "Battery".into(), sub: format!("{}% · {left}", b.percent), hist: vec![], color: p.good, max: 100.0 });
    }
    d
}

fn stat<'a>(p: &Pal, label: &str, value: String) -> Element<'a, Msg> {
    column![t1(label.to_string(), 12.0, p.muted), ts(value, 17.0, p.text).wrapping(text::Wrapping::None)].spacing(2.0).width(Length::Fixed(150.0)).into()
}

fn big_graph<'a>(p: &Pal, values: Vec<f32>, dashed: Vec<f32>, max: f32, color: Color, h: f32, label: &str, right: String) -> Element<'a, Msg> {
    let g = ui::graph(
        Graph { values, dashed, max, min: 0.0, color, grid: Pal::mix(p.panel, p.line, 0.6), fill: true, width: 1.5, centred: false, gridlines: vec![max * 0.25, max * 0.5, max * 0.75] },
        Length::Fill,
        Length::Fixed(h),
    );
    column![
        row![t1(label.to_string(), 12.0, p.muted), Space::new().width(Length::Fill), t1(right, 12.0, p.muted)],
        container(g).width(Length::Fill).style(ui::boxed(Pal::mix(p.bg, p.panel, 0.5), 0.0, Some((Pal::mix(color, p.bg, 0.5), 1.0)))),
        row![t1("60 seconds", 11.0, p.muted), Space::new().width(Length::Fill), t1("0", 11.0, p.muted)],
    ]
    .spacing(4.0)
    .into()
}

fn performance(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    let Some(s) = a.sample.clone() else {
        let why = if a.pref_s("speed", "normal") == "paused" { "Updates are paused (Settings › Real-time update speed)." } else { "Measuring…" };
        return column![head(a, "Performance", String::new(), None, vec![]), container(t(why, 13.0, p.muted)).padding(22.0)].into();
    };
    let devs = devices(a, &s);
    let cur = devs.iter().position(|d| d.key == a.perf_dev).or_else(|| devs.iter().position(|d| d.key.starts_with(&format!("{}:", a.perf_dev)))).unwrap_or(0);
    let mut tiles = Column::new().spacing(4.0).padding(6.0);
    for (i, d) in devs.iter().enumerate() {
        let lit = i == cur;
        let mini = ui::graph(
            Graph { values: d.hist.clone(), dashed: vec![], max: d.max, min: 0.0, color: d.color, grid: Color::TRANSPARENT, fill: true, width: 1.0, centred: false, gridlines: vec![] },
            Length::Fixed(64.0),
            Length::Fixed(42.0),
        );
        let tile = row![
            container(mini).style(ui::boxed(Color::TRANSPARENT, 0.0, Some((d.color, 1.0)))),
            column![t1(d.title.clone(), 13.0, p.text), t(d.sub.clone(), 11.0, p.muted)].spacing(1.0).width(Length::Fill)
        ]
        .spacing(10.0)
        .align_y(iced::Center);
        tiles = tiles.push(
            mouse_area(container(tile).padding(Padding { top: 8.0, right: 8.0, bottom: 8.0, left: 10.0 }).width(Length::Fill).style(ui::boxed(if lit { p.panel2 } else { Color::TRANSPARENT }, p.r(6.0), None)))
                .on_press(Msg::PerfDevice(d.key.clone()))
                .interaction(mouse::Interaction::Pointer),
        );
    }
    let dev = &devs[cur];
    let kind = crate::devset::kind_of(&dev.key).to_string();
    // Overview | <device> & settings (Mantel: Memory's Swap & settings, CPU's Power & settings, …)
    let settings_label = match kind.as_str() {
        "memory" => "Swap & settings",
        "cpu" => "Power & settings",
        "disk" => "Health & settings",
        "net" => "Apps & settings",
        "gpu" => "Tuning",
        "battery" => "Health & settings",
        _ => "",
    };
    let on_settings = a.tab == "settings" && !settings_label.is_empty();
    let tab = |id: &'static str, label: &'static str, fresh: bool| -> Element<'static, Msg> {
        let on = (a.tab == "settings") == (id == "settings");
        let mut r = Row::new().spacing(8.0).align_y(iced::Center).push(t1(label, 15.0, if on { p.text } else { p.muted }));
        if fresh {
            r = r.push(crate::widgets::new_tag(&p));
        }
        mouse_area(container(r).padding(Padding { top: 7.0, right: 18.0, bottom: 7.0, left: 18.0 }).style(ui::boxed(if on { p.panel2 } else { Color::TRANSPARENT }, p.r(8.0), if on { Some((p.line, 1.0)) } else { None })))
            .on_press(Msg::PerfTab(id))
            .interaction(mouse::Interaction::Pointer)
            .into()
    };
    let right: Element<Msg> = if on_settings && kind == "memory" {
        crate::memset::pills(a)
    } else {
        let line = if on_settings {
            crate::devset::headline(a, &s)
        } else {
            match kind.as_str() {
                "cpu" => format!("{} · {} cores", s.cpu.name, s.cpu.cores_physical),
                "memory" => crate::memset::headline(a, &s),
                "disk" => {
                    let k = s.disks.iter().find(|d| Some(d.name.as_str()) == dev.key.strip_prefix("disk:")).cloned().unwrap_or_default();
                    format!("{} · {}", if k.model.is_empty() { k.name.clone() } else { k.model.clone() }, gb(k.size))
                }
                "net" => dev.key.strip_prefix("net:").unwrap_or("").to_string(),
                "gpu" => s.gpus.iter().find(|g| Some(g.index.to_string().as_str()) == dev.key.strip_prefix("gpu:")).map(|g| g.name.clone()).unwrap_or_default(),
                _ => String::new(),
            }
        };
        t1(line, 15.0, p.text).into()
    };
    let mut head = Row::new().spacing(10.0).align_y(iced::Center).push(container(ts(dev.title.clone(), 28.0, p.text).wrapping(text::Wrapping::None)).padding(Padding { top: 0.0, right: 30.0, bottom: 0.0, left: 0.0 }));
    if !settings_label.is_empty() {
        head = head.push(tab("overview", "Overview", false)).push(tab("settings", settings_label, true));
    }
    let head = head.push(Space::new().width(Length::Fill)).push(right);
    let detail: Element<Msg> = if on_settings {
        if kind == "memory" { crate::memset::settings(a) } else { crate::devset::view(a, &s) }
    } else if kind == "cpu" {
        cpu_detail(a, &s)
    } else if kind == "memory" {
        crate::memset::overview(a, &s)
    } else if let Some(name) = dev.key.strip_prefix("disk:") {
        let k = s.disks.iter().find(|d| d.name == name).cloned().unwrap_or_default();
        column![
            big_graph(&p, dev.hist.clone(), vec![], 100.0, dev.color, 220.0, "Active time", "100%".into()),
            row![stat(&p, "Active time", format!("{:.0}%", k.active)), stat(&p, "Average response time", format!("{:.1} ms", k.response)), stat(&p, "Read speed", rate(k.read)), stat(&p, "Write speed", rate(k.write))].spacing(12.0),
            row![stat(&p, "Capacity", gb(k.size)), stat(&p, "Type", if k.ssd { "SSD".into() } else { "HDD".into() })].spacing(12.0),
        ]
        .spacing(14.0)
        .into()
    } else if let Some(name) = dev.key.strip_prefix("net:") {
        let n = s.nets.iter().find(|x| x.name == name).cloned().unwrap_or_default();
        let (rh, sh) = a.net_hist.get(name).map(|x| (x.0.iter().copied().collect::<Vec<f32>>(), x.1.iter().copied().collect::<Vec<f32>>())).unwrap_or_default();
        column![
            big_graph(&p, rh, sh, dev.max, dev.color, 220.0, "Throughput", format!("{:.1} Mbps", dev.max)),
            row![stat(&p, "Send", kbps(n.send)), stat(&p, "Receive", kbps(n.recv)), stat(&p, "Link speed", if n.speed > 0 { format!("{} Mbps", n.speed) } else { "–".into() })].spacing(12.0),
        ]
        .spacing(14.0)
        .into()
    } else if let Some(idx) = dev.key.strip_prefix("gpu:") {
        let g = s.gpus.iter().find(|g| g.index.to_string() == idx).cloned().unwrap_or_default();
        column![
            big_graph(&p, dev.hist.clone(), vec![], 100.0, dev.color, 220.0, "Busy", "100%".into()),
            row![
                stat(&p, "Busy", g.busy.map(|b| format!("{b:.0}%")).unwrap_or_else(|| "–".into())),
                stat(&p, "Video memory", if g.vram_total > 0 { format!("{}/{}", gb(g.vram_used), gb(g.vram_total)) } else { "–".into() }),
                stat(&p, "Temperature", g.temp.map(|t| format!("{t:.0} °C")).unwrap_or_else(|| "–".into())),
            ]
            .spacing(12.0),
        ]
        .spacing(14.0)
        .into()
    } else {
        let b = s.battery.clone().unwrap_or_default();
        column![
            row![
                stat(&p, "Charge", format!("{}% ({})", b.percent, b.status.to_lowercase())),
                stat(&p, "Using now", if b.watts > 0.0 { format!("{:.1} W", b.watts) } else { "–".into() }),
                stat(&p, "Health", if b.design_wh > 0.0 { format!("{:.0} of {:.0} Wh", b.full_wh, b.design_wh) } else { "–".into() }),
                stat(&p, "Charge cycles", if b.cycles > 0 { b.cycles.to_string() } else { "–".into() }),
            ]
            .spacing(12.0),
        ]
        .spacing(14.0)
        .into()
    };
    let right = scrollable(container(column![head, detail].spacing(14.0)).padding(Padding { top: 12.0, right: 22.0, bottom: 22.0, left: 22.0 })).height(Length::Fill);
    column![
        head_bar(a),
        row![container(scrollable(tiles)).width(Length::Fixed(240.0)).height(Length::Fill), ui::vline(p.line), right].height(Length::Fill)
    ]
    .into()
}

fn head_bar(a: &App) -> Element<'_, Msg> {
    head(a, "Performance", String::new(), None, vec![])
}

fn cpu_detail<'a>(a: &'a App, s: &Sample) -> Element<'a, Msg> {
    let p = a.p;
    let mut grid = Row::new().spacing(6.0);
    let n = a.core_hist.len().min(16);
    for h in a.core_hist.iter().take(n) {
        grid = grid.push(ui::graph(
            Graph { values: h.iter().copied().collect(), dashed: vec![], max: 100.0, min: 0.0, color: p.accent, grid: Color::TRANSPARENT, fill: true, width: 1.0, centred: false, gridlines: vec![] },
            Length::FillPortion(1),
            Length::Fixed(46.0),
        ));
    }
    column![
        crate::devset::cores_strip(a, s),
        big_graph(&p, a.cpu_hist.iter().copied().collect(), vec![], 100.0, p.accent, 200.0, "% Utilisation", "100%".into()),
        grid,
        row![
            stat(&p, "Utilisation", format!("{:.0}%", s.cpu.total)),
            stat(&p, "Speed", s.cpu.mhz.map(|m| format!("{:.2} GHz", m as f32 / 1000.0)).unwrap_or_else(|| "–".into())),
            stat(&p, "Processes", s.cpu.processes.to_string()),
            stat(&p, "Threads", s.cpu.threads.to_string()),
        ]
        .spacing(12.0),
        row![
            stat(&p, "Up time", uptime(s.uptime)),
            stat(&p, "Temperature", s.cpu.temp.map(|t| format!("{t:.0} °C")).unwrap_or_else(|| "–".into())),
            stat(&p, "Base speed", s.cpu.max_mhz.map(|m| format!("{:.2} GHz", m as f32 / 1000.0)).unwrap_or_else(|| "–".into())),
            stat(&p, "Logical processors", s.cpu.logical.to_string()),
        ]
        .spacing(12.0),
    ]
    .spacing(14.0)
    .into()
}

// ------------------------------------------------------------------ Settings
fn settings(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    let choice = |key: &'static str, opts: &[(&'static str, &'static str)], cur: String| -> Element<'static, Msg> {
        let mut r = Row::new().spacing(4.0);
        for (id, label) in opts {
            let on = cur == *id;
            r = r.push(
                mouse_area(
                    container(if on { ts(*label, 12.0, p.ink) } else { t(*label, 12.0, p.text) })
                        .padding(Padding { top: 5.0, right: 10.0, bottom: 5.0, left: 10.0 })
                        .style(ui::boxed(if on { p.accent } else { p.panel2 }, p.r(6.0), None)),
                )
                .on_press(Msg::Pref(key, Value::String(id.to_string())))
                .interaction(mouse::Interaction::Pointer),
            );
        }
        r.wrap().into()
    };
    let switch = |key: &'static str, on: bool| -> Element<'static, Msg> {
        let knob = container(Space::new()).width(Length::Fixed(16.0)).height(Length::Fixed(16.0)).style(ui::boxed(if on { p.ink } else { p.muted }, 8.0, None));
        let inner = if on { row![Space::new().width(Length::Fill), knob] } else { row![knob, Space::new().width(Length::Fill)] };
        mouse_area(container(inner).padding(3.0).width(Length::Fixed(42.0)).style(ui::boxed(if on { p.accent } else { p.panel2 }, 11.0, Some((p.line, 1.0)))))
            .on_press(Msg::Pref(key, Value::Bool(!on)))
            .interaction(mouse::Interaction::Pointer)
            .into()
    };
    let line = |title: &'static str, sub: &'static str, ctl: Element<'static, Msg>| -> Element<'static, Msg> {
        container(row![column![tm(title, 14.0, p.text), t(sub, 12.0, p.muted)].spacing(2.0).width(Length::Fill), ctl].spacing(16.0).align_y(iced::Center))
            .padding(Padding { top: 12.0, right: 16.0, bottom: 12.0, left: 16.0 })
            .into()
    };
    let card = ui::card(
        &p,
        column![
            line(
                "Default start page",
                "The page Task Manager opens on",
                choice("startPage", &[("processes", "Processes"), ("performance", "Performance"), ("startup", "Startup apps"), ("details", "Details"), ("services", "Services"), ("last", "Last page used")], a.pref_s("startPage", "processes"))
            ),
            hline(p.line),
            line("Real-time update speed", "How often the numbers change", choice("speed", &[("high", "High (0.5 s)"), ("normal", "Normal (1 s)"), ("low", "Low (4 s)"), ("paused", "Paused")], a.pref_s("speed", "normal"))),
            hline(p.line),
            line("Always on top", "Task Manager stays over other windows", switch("onTop", a.pref_b("onTop"))),
            hline(p.line),
            line("Fold the navigation rail", "Only the icons on the left", switch("railFolded", a.pref_b("railFolded"))),
            hline(p.line),
            line("Theme", "Task Manager follows your Hearth look and accent (Hearth Settings › Appearance)", t1("Hearth's look", 13.0, p.muted).into()),
        ],
    );
    column![head(a, "Settings", String::new(), None, vec![]), scrollable(container(card).padding(Padding { top: 4.0, right: 22.0, bottom: 22.0, left: 22.0 })).height(Length::Fill)].into()
}

// ------------------------------------------------------------------ the menu and the sheets
fn menu<'a>(a: &'a App, m: &'a crate::Menu) -> Element<'a, Msg> {
    let p = a.p;
    let mut col = Column::new().spacing(0.0);
    if !m.title.is_empty() {
        col = col.push(container(t1(ui::fit(&m.title, 40), 12.0, p.muted)).padding(Padding { top: 6.0, right: 12.0, bottom: 6.0, left: 12.0 }));
    }
    for (i, it) in m.items.iter().enumerate() {
        if it.act.is_none() {
            col = col.push(container(hline(p.line)).padding(Padding { top: 4.0, right: 6.0, bottom: 4.0, left: 6.0 }));
            continue;
        }
        let fg = if it.enabled { p.text } else { p.muted };
        let mut r = Row::new().spacing(8.0).align_y(iced::Center).push(container(if it.checked { glyph(G::Check, p.accent, 12.0, 2.5) } else { gap(12.0, 12.0) }).width(Length::Fixed(14.0)));
        r = r.push(t1(it.label.clone(), 13.0, fg));
        if it.fresh {
            r = r.push(chip("NEW", Pal::mix(p.panel, p.accent, 0.3), p.accent));
        }
        r = r.push(Space::new().width(Length::Fill));
        if !it.key.is_empty() {
            r = r.push(t1(it.key.clone(), 12.0, p.muted));
        }
        let lit = i == m.sel;
        let bx = container(r).padding(Padding { top: 6.0, right: 12.0, bottom: 6.0, left: 8.0 }).width(Length::Fill).style(ui::boxed(if lit { p.sel() } else { Color::TRANSPARENT }, p.r(4.0), None));
        col = col.push(if it.enabled { mouse_area(bx).on_press(Msg::MenuPick(i)).interaction(mouse::Interaction::Pointer) } else { mouse_area(bx) });
    }
    container(col)
        .padding(4.0)
        .width(Length::Fixed(330.0))
        .style(move |_t: &iced::Theme| container::Style {
            background: Some(iced::Background::Color(p.panel)),
            border: iced::Border { color: p.line, width: 1.0, radius: p.r(8.0).into() },
            shadow: iced::Shadow { color: Color { a: 0.45, ..Color::BLACK }, offset: iced::Vector::new(0.0, 6.0), blur_radius: 18.0 },
            ..container::Style::default()
        })
        .into()
}

fn sheet(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    let Some(sh) = &a.sheet else { return Space::new().into() };
    let mut col = Column::new().spacing(12.0).push(ts(sh.title.clone(), 17.0, p.text)).push(t(sh.text.clone(), 13.0, p.muted));
    let ask = matches!(sh.kind, SheetKind::Ask(_) | SheetKind::Run);
    if ask {
        col = col.push(
            text_input(if sh.kind == SheetKind::Run { "Program" } else { "" }, &sh.input)
                .on_input(Msg::SheetInput)
                .on_submit(Msg::SheetOk)
                .size(14.0)
                .padding(8.0),
        );
    }
    let buttons: Vec<Element<Msg>> = match sh.kind {
        SheetKind::Ending => vec![btn(&p, "Hide", Kind::Normal, true, Some(Msg::CloseSheet))],
        SheetKind::Stuck => vec![btn(&p, "Force end", Kind::Danger, true, Some(Msg::SheetForce)), btn(&p, "Wait", Kind::Normal, false, Some(Msg::SheetWait))],
        SheetKind::Notice => vec![btn(&p, "OK", Kind::Primary, true, Some(Msg::CloseSheet))],
        _ => vec![btn(&p, if sh.kind == SheetKind::Run { "Run" } else { "OK" }, Kind::Primary, false, Some(Msg::SheetOk)), btn(&p, "Cancel", Kind::Normal, false, Some(Msg::CloseSheet))],
    };
    let mut r = Row::new().spacing(8.0);
    for b in buttons {
        r = r.push(b);
    }
    col = col.push(container(r).align_right(Length::Fill));
    ui::dialog_box(&p, 520.0, col)
}
