//! Performance › Memory: the Overview (Mantel's MemoryOverview.qml, mockup task-manager-memory) and Swap & settings
//! (MemorySettings.qml, mockup task-manager-swap-settings), in Rust.
//!
//! Both read `hearth-memory status` (the same helper and /etc/hearth/memory.json as Mantel and Settings › Memory, red
//! flag 4). The Overview only reads. On Swap & settings your changes are kept here until you press Save (or Try until
//! restart), then go to `pkexec hearth-memory set` (polkit: 49-hearth-memory.rules). Nothing changes by itself, and
//! opening it writes nothing (red flag 9).

use hearth_ui::run;
use hearth_ui::ui::{self, btn, t, t1, tm, ts, Graph, Kind};
use hearth_ui::Pal;
use iced::widget::{column, container, mouse_area, row, text, text_input, Column, Row, Space};
use iced::{mouse, Color, Length, Padding, Task};
use serde_json::{json, Map, Value};

use crate::devset::{at, f_of, s_of};
use crate::sample::Sample;
use crate::widgets::{self as w, E};
use crate::{App, Msg};

pub const HELPER: &str = "/usr/libexec/nimbus/hearth-memory";
const GIB: f64 = 1073741824.0;

fn c_prog() -> Color {
    Color::from_rgb8(0x8b, 0x80, 0xf9)
}
fn c_zram() -> Color {
    Color::from_rgb8(0x3f, 0xd0, 0xe0)
}
fn c_cache() -> Color {
    Color::from_rgb8(0xcf, 0xc9, 0xd8)
}

/// Sizes as Task Manager shows them (1024s: 3.9 GB, 310 MB).
pub fn size(b: f64) -> String {
    let u = ["B", "KB", "MB", "GB", "TB"];
    let (mut x, mut i) = (b.max(0.0), 0);
    while x >= 1024.0 && i < u.len() - 1 {
        x /= 1024.0;
        i += 1;
    }
    if i == 0 || x >= 100.0 { format!("{} {}", x.round(), u[i]) } else { format!("{:.1} {}", x, u[i]) }
}
/// "3.1/12 GB": both in the unit of the total.
pub fn size_of(used: f64, total: f64) -> String {
    if total <= 0.0 {
        return "–".into();
    }
    let u = ["B", "KB", "MB", "GB", "TB"];
    let (mut x, mut i) = (total, 0);
    while x >= 1024.0 && i < u.len() - 1 {
        x /= 1024.0;
        i += 1;
    }
    let f = |n: f64| {
        let y = n / 1024f64.powi(i as i32);
        if y >= 100.0 { format!("{}", y.round()) } else { format!("{y:.1}") }
    };
    format!("{} / {} {}", f(used), f(total), u[i])
}
fn press_color(p: &Pal, v: f32) -> Color {
    if v < 1.0 { Color::from_rgb8(0x4e, 0xc9, 0x8a) } else if v < 10.0 { p.flame } else if v < 30.0 { p.warn } else { p.danger }
}
fn press_word(v: f32) -> &'static str {
    if v < 1.0 { "None" } else if v < 10.0 { "Light" } else if v < 30.0 { "Medium" } else { "Heavy" }
}

/// "7.5 GB · DDR5 4800 MT/s · 2 of 2 slots": the memory's speed and slots, from the board (hearth-hwset memory-info).
pub fn headline(a: &App, s: &Sample) -> String {
    let b = a.dev.get("board").unwrap_or(&Value::Null);
    let mut out = size(s.mem.total as f64);
    if let Some(sp) = f_of(at(b, "speed")) {
        let typ = s_of(at(b, "type"));
        out += &format!(" · {}{sp} MT/s", if typ.is_empty() { String::new() } else { format!("{typ} ") });
    }
    if let (Some(sl), Some(u)) = (f_of(at(b, "slots")), f_of(at(b, "used"))) {
        if sl > 0.0 {
            out += &format!(" · {u} of {sl} slots");
        }
    }
    out
}

fn kv<'a>(p: &Pal, k: impl Into<String>, v: impl Into<String>, c: Option<Color>) -> E<'a> {
    row![container(t1(k.into(), 13.0, p.muted)).width(Length::Fill).clip(true), t1(v.into(), 13.0, c.unwrap_or(p.text)).wrapping(text::Wrapping::None)].align_y(iced::Center).into()
}
fn meter<'a>(p: &Pal, frac: f32, c: Color) -> E<'a> {
    ui::bar(vec![(frac.clamp(0.0, 1.0).max(if frac > 0.0 { 0.01 } else { 0.0 }), c)], p.panel3, 6.0, 3.0)
}
/// A card with its title in capitals (Swap · Memory pressure · Memory setting · Biggest users), and NEW.
fn ocard<'a>(p: &Pal, title: &str, fresh: bool, body: Vec<E<'a>>) -> E<'a> {
    let p = *p;
    let mut head = Row::new().spacing(10.0).align_y(iced::Center).push(ts(title.to_uppercase(), 11.0, p.muted));
    if fresh {
        head = head.push(w::new_tag(&p));
    }
    let mut col = Column::new().spacing(9.0).push(head);
    for b in body {
        col = col.push(b);
    }
    container(col).padding(Padding { top: 12.0, right: 14.0, bottom: 14.0, left: 14.0 }).width(Length::Fill).style(ui::boxed(Color::TRANSPARENT, p.r(10.0), Some((p.line, 1.0)))).into()
}

// ------------------------------------------------------------------ Overview (mockup task-manager-memory)
pub fn overview<'a>(a: &'a App, s: &Sample) -> E<'a> {
    let p = a.p;
    let m = &s.mem;
    let total = m.total.max(1) as f64;
    let zmem = m.zram_used as f64;
    let programs = (m.used as f64 - zmem).max(0.0);
    let cached = m.cached as f64;
    let free = m.free as f64;
    let has_zram = m.zram_swap_total > 0;

    // the graph: programs (filled) and compressed memory (dashed)
    let prog = ui::graph(
        Graph { values: a.prog_hist.iter().copied().collect(), dashed: vec![], max: 100.0, min: 0.0, color: c_prog(), grid: Color::TRANSPARENT, fill: true, width: 2.0, centred: false, gridlines: vec![] },
        Length::Fill,
        Length::Fixed(230.0),
    );
    let mut layers = vec![prog];
    if has_zram {
        layers.push(ui::graph(
            Graph { values: vec![], dashed: a.zram_hist.iter().copied().collect(), max: 100.0, min: 0.0, color: c_zram(), grid: Color::TRANSPARENT, fill: false, width: 1.5, centred: false, gridlines: vec![] },
            Length::Fill,
            Length::Fixed(230.0),
        ));
        layers.push(
            container(column![Space::new().height(Length::Fill), row![Space::new().width(Length::Fill), t1("- - compressed (zram)", 12.0, c_zram())]])
                .padding(10.0)
                .width(Length::Fill)
                .height(Length::Fixed(230.0))
                .into(),
        );
    }
    let graph = container(iced::widget::Stack::with_children(layers).width(Length::Fill).height(Length::Fixed(230.0))).style(ui::boxed(p.bg, 0.0, Some((Pal::a(c_prog(), 0.9), 1.0))));

    // what memory is made of
    let sum = (programs + zmem + cached + free).max(1.0);
    let mut bar = Row::new();
    for (i, (v, c, l)) in [(programs, c_prog(), "In use"), (zmem, c_zram(), "Compressed"), (cached, c_cache(), "Cached"), (free, p.panel3, "Free")].iter().enumerate() {
        let por = (1000.0 * v / sum).round() as u16;
        if por == 0 {
            continue;
        }
        let fg = if i == 3 { p.text } else { Color::from_rgb8(0x14, 0x10, 0x1c) };
        let label = format!("{l} {}", size(*v));
        bar = bar.push(
            container(text(label).size(12.0).color(fg).font(ui::weight(iced::font::Weight::Semibold)).wrapping(text::Wrapping::None))
                .center_x(Length::FillPortion(por))
                .center_y(Length::Fixed(26.0))
                .clip(true)
                .style(ui::boxed(*c, 0.0, None)),
        );
    }
    let bar = container(bar).width(Length::Fill).clip(true).style(ui::boxed(p.panel3, 4.0, None));
    let key = |c: Color, l: &str| -> E<'a> { row![container(Space::new()).width(Length::Fixed(11.0)).height(Length::Fixed(11.0)).style(ui::boxed(c, 2.0, None)), t1(l.to_string(), 12.0, p.muted)].spacing(6.0).align_y(iced::Center).into() };
    let legend = row![key(c_prog(), "In use: what programs hold"), key(c_zram(), "Compressed: zram's real size"), key(c_cache(), "Cached: given back when needed"), key(p.panel3, "Free")].spacing(18.0).wrap();

    // the numbers
    let num = |label: &str, big: String, small: String, fresh: bool| -> E<'a> {
        let mut l = Row::new().spacing(8.0).align_y(iced::Center).push(t1(label.to_string(), 12.0, p.muted));
        if fresh {
            l = l.push(w::new_tag(&p));
        }
        let mut v = Row::new().spacing(6.0).align_y(iced::Bottom).push(t1(big, 24.0, p.text));
        if !small.is_empty() {
            v = v.push(container(t1(small, 12.0, p.muted)).padding(Padding { top: 0.0, right: 0.0, bottom: 3.0, left: 0.0 }));
        }
        column![l, v].spacing(1.0).into()
    };
    let mut nums = Row::new().spacing(30.0).push(num("In use", size(programs), String::new(), false)).push(num("Available", size(m.available as f64), String::new(), false)).push(num("Cached", size(cached), String::new(), false)).push(num("Free", size(free), String::new(), false));
    if has_zram {
        let ratio = if zmem > 0.0 && m.zram_data > 0 { format!(" · {}:1", (m.zram_data as f64 / zmem).round().max(1.0)) } else { String::new() };
        nums = nums.push(num("Compressed", size(m.zram_data as f64), format!("in {}{ratio}", size(zmem)), true));
    }
    nums = nums.push(num("Committed", size(m.committed as f64), format!("/ {}", size(m.commit_limit as f64)), false));

    // Swap · Memory pressure · Memory setting
    let st = a.dev.get("mem").unwrap_or(&Value::Null);
    let set = at(st, "settings");
    let drive = m.drive_swap_total > 0;
    let mut swap = vec![kv(&p, "Zram (in memory, fast)", if has_zram { size_of(m.zram_swap_used as f64, m.zram_swap_total as f64) } else { "Off".into() }, None)];
    if has_zram {
        swap.push(meter(&p, m.zram_swap_used as f32 / m.zram_swap_total.max(1) as f32, c_zram()));
    }
    swap.push(kv(&p, "Drive swap file (slow)", if drive { size_of(m.drive_swap_used as f64, m.drive_swap_total as f64) } else { "None".into() }, None));
    if drive {
        swap.push(meter(&p, m.drive_swap_used as f32 / m.drive_swap_total.max(1) as f32, p.flame));
    }
    swap.push(
        t(
            if has_zram {
                format!("Fills only when zram is full.{}", if at(st, "swapfile.hibernate") == &Value::Bool(true) { " Also used for hibernation." } else { "" })
            } else {
                "Memory isn't compressed; Swap & settings can turn zram on.".to_string()
            },
            12.0,
            p.muted,
        )
        .into(),
    );
    let pc = press_color(&p, m.pressure);
    let now_chip: E<'a> = container(t1(press_word(m.pressure), 13.0, pc).font(ui::weight(iced::font::Weight::Semibold))).padding(Padding { top: 3.0, right: 8.0, bottom: 3.0, left: 8.0 }).style(ui::boxed(Pal::a(pc, 0.18), p.r(6.0), None)).into();
    let mut bars = Row::new().spacing(2.0).align_y(iced::Bottom).height(Length::Fixed(34.0));
    for v in a.pressure_hist.iter() {
        bars = bars.push(container(Space::new()).width(Length::FillPortion(1)).height(Length::Fixed((3.0 + v * 1.5).clamp(2.0, 30.0))).style(ui::boxed(press_color(&p, *v), 0.0, None)));
    }
    let pressure = vec![
        row![container(t1("Now", 13.0, p.muted)).width(Length::Fill), now_chip].align_y(iced::Center).into(),
        kv(&p, "Last 10 s · last minute", format!("{:.1}% · {:.1}%", m.pressure, m.pressure60), None),
        bars.into(),
        t("How often programs waited for memory. High = stutter.", 12.0, p.muted).into(),
    ];
    let keep_mode = { let k = s_of(at(set, "keep.mode")); if k.is_empty() { "auto".to_string() } else { k } };
    let keep = match keep_mode.as_str() {
        "auto" => "Auto",
        "off" => "Off",
        "2gb" => "2 GB",
        "50%" => "Half (50%)",
        "25%" => "A quarter (25%)",
        "custom" => "Custom",
        x => x,
    }
    .to_string();
    let dash = st.is_null();
    let zram_txt = if dash { "–".to_string() } else if at(set, "zram.on") == &Value::Bool(false) { "Off".into() } else if at(set, "zram").is_null() { "–".into() } else { format!("{} GB · {}", s_of(at(set, "zram.gb")), s_of(at(set, "zram.algo"))) };
    let swapf = if dash {
        "–".to_string()
    } else if at(st, "swapfile.exists") == &Value::Bool(true) {
        format!("{} GB{}", s_of(at(st, "swapfile.gb")), if at(set, "swapfile.everyday") == &Value::Bool(false) { " · only for hibernating" } else { "" })
    } else {
        "None".into()
    };
    let guard = match s_of(at(set, "guard.level")).as_str() {
        _ if dash || at(set, "guard").is_null() => "–",
        "strong" => "Close something early",
        "off" => "The system decides",
        _ => "Close something gently",
    };
    let setting = vec![
        kv(&p, "Keep free", if dash { "–".into() } else { keep }, None),
        kv(&p, "Zram", zram_txt, None),
        kv(&p, "Drive swap file", swapf, None),
        kv(&p, "When memory runs out", guard, None),
        w::link(&p, "Change in Swap & settings ›", Msg::PerfTab("settings")),
    ];
    let cards = row![
        container(ocard(&p, "Swap", true, swap)).width(Length::FillPortion(1)),
        container(ocard(&p, "Memory pressure", true, pressure)).width(Length::FillPortion(1)),
        container(ocard(&p, "Memory setting", true, setting)).width(Length::FillPortion(1))
    ]
    .spacing(14.0);

    // the programs using the most; swapped memory is in zram or on the drive (Linux doesn't say which per program,
    // so each gets the PC's split)
    let share = if m.drive_swap_used > 0 { m.drive_swap_used as f64 / (m.drive_swap_used + m.zram_swap_used).max(1) as f64 } else { 0.0 };
    let mut top: Vec<&crate::sample::Group> = s.groups.iter().collect();
    top.sort_by(|x, y| y.mem.cmp(&x.mem));
    let rcell = |s: String, wdt: f32, c: Color| -> E<'a> { container(t1(s, 13.0, c)).align_right(Length::Fixed(wdt)).into() };
    let head_new: E<'a> = row![t1("Compressed", 13.0, p.muted), w::new_tag(&p)].spacing(8.0).align_y(iced::Center).into();
    let mut users = Column::new().push(
        container(row![container(t1("Program", 13.0, p.muted)).width(Length::Fill), rcell("In use".into(), 110.0, p.muted), rcell("Shared".into(), 110.0, p.muted), container(head_new).align_right(Length::Fixed(160.0)), rcell("On drive".into(), 100.0, p.muted)].spacing(10.0).align_y(iced::Center))
            .padding(Padding { top: 0.0, right: 8.0, bottom: 0.0, left: 8.0 })
            .center_y(Length::Fixed(30.0)),
    );
    for g in top.iter().take(6) {
        let n = g.children.len().max(1);
        let name = if n > 1 { format!("{} ({n})", g.name) } else { g.name.clone() };
        users = users.push(ui::hline(Pal::a(p.line, 0.6)));
        users = users.push(
            container(
                row![
                    container(t1(name, 13.0, p.text)).width(Length::Fill).clip(true),
                    rcell(size(g.mem as f64), 110.0, p.text),
                    rcell(size(g.shared as f64), 110.0, p.text),
                    rcell(size(g.swap as f64 * (1.0 - share)), 160.0, p.text),
                    rcell(if share > 0.0 { size(g.swap as f64 * share) } else { "0".into() }, 100.0, p.text)
                ]
                .spacing(10.0)
                .align_y(iced::Center),
            )
            .padding(Padding { top: 0.0, right: 8.0, bottom: 0.0, left: 8.0 })
            .center_y(Length::Fixed(30.0)),
        );
    }
    if top.is_empty() {
        users = users.push(t1("Looking at what's running…", 12.0, p.muted));
    }

    let comp_head: E<'a> = row![t1("Memory composition", 12.0, p.muted), Space::new().width(Length::Fill), w::new_tag(&p)].align_y(iced::Center).into();
    column![
        row![t1("Memory in use (programs) · 60 seconds", 12.0, p.muted), Space::new().width(Length::Fill), t1(size(total), 12.0, p.muted)],
        graph,
        container(comp_head).padding(Padding { top: 6.0, right: 0.0, bottom: 0.0, left: 0.0 }),
        bar,
        legend,
        container(nums.wrap()).padding(Padding { top: 6.0, right: 0.0, bottom: 0.0, left: 0.0 }),
        container(cards).padding(Padding { top: 8.0, right: 0.0, bottom: 0.0, left: 0.0 }),
        container(ocard(&p, "Biggest users", false, vec![users.into()])).padding(Padding { top: 8.0, right: 0.0, bottom: 0.0, left: 0.0 }),
    ]
    .spacing(8.0)
    .into()
}

// ------------------------------------------------------------------ Swap & settings (mockup task-manager-swap-settings)
fn cur<'v>(a: &'v App) -> &'v Value {
    a.dev.get("mem").map(|s| at(s, "settings")).unwrap_or(&Value::Null)
}
/// What you changed (not saved yet), else what's saved, else `d`.
fn val(a: &App, group: &str, key: &str, d: Value) -> Value {
    if let Some(v) = a.mem_draft.get(group).and_then(|g| g.get(key)) {
        return v.clone();
    }
    let c = at(cur(a), &format!("{group}.{key}"));
    if c.is_null() { d } else { c.clone() }
}
fn put(a: &mut App, group: &str, key: &str, v: Value) {
    let e = a.mem_draft.entry(group.to_string()).or_insert_with(|| Value::Object(Map::new()));
    if !e.is_object() {
        *e = Value::Object(Map::new());
    }
    if let Some(o) = e.as_object_mut() {
        o.insert(key.to_string(), v);
    }
}
fn keep_mode(a: &App) -> String {
    a.mem_draft.get("keep").map(|k| s_of(at(k, "mode"))).filter(|x| !x.is_empty()).unwrap_or_else(|| { let m = s_of(at(cur(a), "keep.mode")); if m.is_empty() { "auto".into() } else { m } })
}
fn keep_factor(a: &App) -> f64 {
    let total = a.dev.get("mem").and_then(|s| f_of(at(s, "memory.total"))).unwrap_or(0.0);
    let from_mb = |c: &Value| f_of(at(c, "keep.mb")).filter(|_| total > 0.0).map(|mb| (mb * 1048576.0 / (2.0 * total) * 10000.0).round());
    a.mem_draft.get("keep").and_then(|k| f_of(at(k, "factor"))).or_else(|| f_of(at(cur(a), "keep.factor"))).or_else(|| from_mb(cur(a))).unwrap_or(1000.0)
}
fn swappiness(a: &App) -> f64 {
    a.mem_draft.get("swappiness").and_then(Value::as_f64).or_else(|| f_of(at(cur(a), "swappiness"))).unwrap_or(200.0)
}
fn swap_gb(a: &App) -> f64 {
    a.mem_draft.get("swapGb").and_then(Value::as_f64).or_else(|| a.dev.get("mem").and_then(|s| f_of(at(s, "swapfile.gb")))).unwrap_or(0.0)
}
fn gb(b: f64) -> String {
    if b >= 10.0 * GIB { format!("{}", (b / GIB).round()) } else { format!("{:.1}", b / GIB) }
}

/// The pills on the right of the title (Pressure: none · Zram 3.1 / 12 GB · 3:1 · Drive swap 0.2 / 36 GB).
pub fn pills<'a>(a: &'a App) -> E<'a> {
    let p = a.p;
    let Some(st) = a.dev.get("mem") else { return Space::new().into() };
    let word = s_of(at(st, "pressure.word"));
    let mut r = Row::new().spacing(8.0).push(w::pill(&p, format!("Pressure: {word}"), Some(if word != "none" && word != "light" { p.warn } else { w::green() })));
    if at(st, "zram.active") == &Value::Bool(true) {
        let ratio = f_of(at(st, "zram.ratio")).map(|r| format!(" · {r}:1")).unwrap_or_default();
        r = r.push(w::pill(&p, format!("Zram {} / {} GB{ratio}", gb(f_of(at(st, "zram.used")).unwrap_or(0.0)), gb(f_of(at(st, "zram.disksize")).unwrap_or(0.0))), None));
    }
    if at(st, "swapfile.exists") == &Value::Bool(true) {
        let path = s_of(at(st, "swapfile.path"));
        let used = at(st, "swaps").as_array().and_then(|l| l.iter().find(|x| s_of(at(x, "path")) == path)).and_then(|x| f_of(at(x, "used"))).unwrap_or(0.0);
        r = r.push(w::pill(&p, format!("Drive swap {} / {} GB", gb(used), s_of(at(st, "swapfile.gb"))), None));
    }
    r.into()
}

fn mbtn<'a>(a: &'a App, label: &'a str, kind: Kind, key: &str) -> E<'a> {
    btn(&a.p, label, kind, false, if a.mem_busy { None } else { Some(Msg::Pick(key.to_string(), String::new())) })
}
fn chip_list<'a>(p: &Pal, items: Vec<(String, Color, Msg)>) -> E<'a> {
    let mut r = Row::new().spacing(6.0);
    for (l, c, m) in items {
        r = r.push(mouse_area(container(t1(l, 12.0, c)).padding(Padding { top: 5.0, right: 10.0, bottom: 5.0, left: 10.0 }).style(ui::boxed(Color::TRANSPARENT, p.r(6.0), Some((c, 1.0))))).on_press(m).interaction(mouse::Interaction::Pointer));
    }
    container(r.wrap()).max_width(320.0).into()
}

pub fn settings<'a>(a: &'a App) -> E<'a> {
    let p = a.p;
    let Some(st) = a.dev.get("mem") else {
        return t(if a.dev_busy.contains("mem") { "Reading memory settings…" } else { "Your memory settings couldn't be read here." }, 13.0, p.muted).into();
    };
    let mut col = Column::new().spacing(14.0);
    // your test files: handed over only when you press it
    if let Some(tf) = at(st, "testFiles").as_object().filter(|o| !o.is_empty()) {
        let files = tf.values().map(s_of).collect::<Vec<_>>().join(", ");
        col = col.push(
            container(row![
                container(t(format!("Your test settings are in use ({files}). Hearth shows their values here and leaves them as they are. Use Hearth's settings to hand them over: the same values, kept by Hearth, with the files kept as .bak."), 12.0, p.text)).width(Length::Fill),
                mbtn(a, "Use Hearth's settings", Kind::Normal, "mem.adopt")
            ].spacing(12.0).align_y(iced::Center))
            .padding(14.0)
            .style(ui::boxed(p.panel2, p.r(8.0), Some((p.flame, 1.0)))),
        );
    }
    let total = f_of(at(st, "memory.total")).unwrap_or(0.0);

    // Compressed memory (zram)
    let zon = val(a, "zram", "on", Value::Bool(true)) == Value::Bool(true);
    let zgb = val(a, "zram", "gb", json!(8)).as_f64().unwrap_or(8.0);
    let zmax = f_of(at(st, "limits.zramMaxGb")).unwrap_or(16.0) as f32;
    let algos: Vec<(String, String)> = at(st, "algos").as_array().map(|l| l.iter().take(4).map(|x| (s_of(x), s_of(x))).collect()).unwrap_or_default();
    let ratio = f_of(at(st, "zram.ratio"));
    let r = ratio.unwrap_or(3.0);
    let rec = f_of(at(st, "recommended.zram.gb"));
    let mut zt = Row::new().spacing(4.0).align_y(iced::Center).push(t1("When full, holds ", 12.0, p.text)).push(tm(format!("{zgb} GB"), 12.0, p.text)).push(t1(" in about ", 12.0, p.text)).push(tm(format!("{} GB", (zgb / r).round().max(1.0)), 12.0, p.text)).push(t1(format!(" of real memory ({}{r}:1).", if ratio.is_some() { "your usual " } else { "about " }), 12.0, p.text));
    if rec == Some(zgb) {
        zt = zt.push(container(t1(format!("recommended for {} GB", (total / GIB).round()), 12.0, w::green())).padding(Padding { top: 1.0, right: 4.0, bottom: 1.0, left: 4.0 }).style(ui::boxed(Pal::a(w::green(), 0.15), 3.0, None)));
    }
    let zram = w::card(
        &p,
        "Compressed memory (zram)",
        "Squeezes idle memory inside RAM instead of writing it to the drive. Fast, and it uses nothing until it's needed.",
        false,
        vec![
            w::line(&p, "On", "", w::toggle(&p, zon, Some(Msg::Pick("mem.zram.on".into(), if zon { "off".into() } else { "on".into() })))),
            w::line(&p, "Size", "how much overflow it can hold (up to twice your memory; the green mark is recommended)", w::slide("mem.zram.gb", 1.0, zmax.max(2.0), 1.0, zgb as f32, format!("{zgb} GB"), &p)),
            w::line(&p, "Compression", "zstd squeezes most · lz4 is quickest (only what this kernel offers)", w::chips(&p, "mem.zram.algo", &algos, &s_of(&val(a, "zram", "algo", json!("zstd"))))),
            container(zt.wrap()).padding(Padding { top: 9.0, right: 12.0, bottom: 9.0, left: 12.0 }).width(Length::Fill).style(ui::boxed(p.panel2, p.r(6.0), None)).into(),
        ],
    );

    // Keep memory free
    let km = keep_mode(a);
    let mut kf = vec![w::line(&p, "Target", "", w::chips(&p, "mem.keep", &[("auto", "Auto"), ("off", "Off"), ("2gb", "2 GB"), ("25%", "25%"), ("50%", "50%"), ("custom", "Custom")].iter().map(|(x, y)| (x.to_string(), y.to_string())).collect::<Vec<_>>(), &km))];
    if km == "custom" {
        let pct = (keep_factor(a) / 50.0).round();
        kf.push(w::line(&p, "Keep free", "a share of your memory", w::slide("mem.keep.factor", 1.0, 60.0, 1.0, pct as f32, format!("{pct}%"), &p)));
    }
    if let Some(low) = f_of(at(st, "watermarks.low")).filter(|l| *l > 0.0) {
        kf.push(w::line(&p, "Starts freeing below", "then frees up to the target", w::value(&p, format!("{}% free", (100.0 * low / total.max(1.0)).round().max(1.0)), None)));
    }
    let sw = swappiness(a);
    kf.push(w::line(&p, "Swappiness", "how readily idle memory is compressed instead of dropping cached files", w::slide("mem.swappiness", 0.0, 200.0, 1.0, sw as f32, format!("{sw}"), &p)));
    if let Some(over) = at(st, "overridden").as_object().filter(|o| !o.is_empty()) {
        let vm = at(st, "vm");
        let what = over.keys().map(|k| format!("vm.{k} = {}", { let v = s_of(at(vm, k)); if v.is_empty() { "?".into() } else { v } })).collect::<Vec<_>>().join(", ");
        let mut by: Vec<String> = over.values().map(s_of).collect();
        by.sort();
        by.dedup();
        kf.push(
            container(row![container(t(format!("Not in use: {what} is set by {}, so these settings don't apply there. Use these settings keeps that file as .bak (rename it back to undo).", by.join(", ")), 12.0, p.warn)).width(Length::Fill), mbtn(a, "Use these settings", Kind::Normal, "mem.handover")].spacing(12.0).align_y(iced::Center))
                .padding(12.0)
                .style(ui::boxed(Color::TRANSPARENT, p.r(6.0), Some((p.warn, 1.0))))
                .into(),
        );
    }
    if km != "auto" && !a.mem_draft.contains_key("keep") {
        let mb = if total > 0.0 { ((total * 0.025 / 1048576.0 / 10.0).round() * 10.0).max(1.0) } else { 200.0 };
        kf.push(
            container(row![container(t(format!("Recommended: Auto. It keeps a small share of memory free (about {mb} MB here) instead of a fixed amount, so the PC isn't busy freeing memory all the time. Your choice stays until you change it."), 12.0, p.muted)).width(Length::Fill), mbtn(a, "Use Auto", Kind::Normal, "mem.useAuto")].spacing(12.0).align_y(iced::Center))
                .padding(12.0)
                .style(ui::boxed(Color::TRANSPARENT, p.r(6.0), Some((p.line, 1.0))))
                .into(),
        );
    }
    if km == "50%" || km == "25%" {
        kf.push(
            container(t(format!("{km} works your memory harder: more compressing, a little more battery. Fine for browsing and AI work; if a game stutters, try Auto."), 12.0, p.warn))
                .padding(12.0)
                .width(Length::Fill)
                .style(ui::boxed(Pal::a(p.warn, 0.08), p.r(6.0), Some((p.warn, 1.0))))
                .into(),
        );
    }
    let keep = w::card(&p, "Keep memory free", "How much the system keeps empty, ready for the next heavy moment. It moves idle memory into zram to keep it.", false, kf);

    // Drive swap file
    let max = f_of(at(st, "limits.swapfileMaxGb")).unwrap_or(0.0);
    let have = f_of(at(st, "swapfile.gb")).unwrap_or(0.0);
    let mem_gb = (total / GIB).round();
    let mut sizes: Vec<f64> = vec![0.0, 4.0, 8.0, 16.0, 32.0, mem_gb, 64.0, have].into_iter().filter(|v| *v <= max.max(have)).collect();
    sizes.sort_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal));
    sizes.dedup();
    let mut size_opts: Vec<(String, String)> = sizes.iter().map(|v| (format!("{v}"), if *v == 0.0 { "Off".into() } else { format!("{v} GB") })).collect();
    size_opts.push(("custom".into(), "Custom".into()));
    let sgb = swap_gb(a);
    let everyday = val(a, "swapfile", "everyday", Value::Bool(true)) == Value::Bool(true);
    let mut df = vec![
        w::line(&p, "Use for everyday overflow", "off = only for hibernation", w::toggle(&p, everyday, Some(Msg::Pick("mem.swapfile.everyday".into(), if everyday { "off".into() } else { "on".into() })))),
        w::line(
            &p,
            "Size",
            format!("on C: (/var/swap) · hibernation needs at least your memory size · C: always keeps 20 GB free · at most {max} GB here"),
            // a list with ▾, as the mockup draws it (Mantel: the same sizes side by side)
            w::dropdown(&p, "mem.swapGb", size_opts, &if a.swap_custom { "custom".to_string() } else { format!("{sgb}") }),
        ),
    ];
    if a.swap_custom {
        let v = sgb.max(1.0);
        df.push(w::line(&p, "Custom size", format!("1 GB up to {max} GB"), w::slide("mem.swapGb.custom", 1.0, max.max(have).max(2.0) as f32, 1.0, v as f32, format!("{v} GB"), &p)));
    }
    df.push(t(if at(st, "swapfile.hibernate") == &Value::Bool(true) { "Hibernation has its own space (/var/swap/hearth-hibernate), left as it is." } else { "Changing the size makes the file again when you press Save (it can take a minute)." }, 11.0, p.muted).into());
    let checked = f_of(at(st, "hibernateChecked")).filter(|x| *x > 0.0);
    let hib_sub = match checked {
        Some(t0) => format!("works on this laptop · last checked {}", hearth_ui::time::when(t0 as i64)),
        None if at(st, "swapfile.hibernate") == &Value::Bool(true) => "set up, not tested on this PC yet".into(),
        None => "not set up".into(),
    };
    df.push(w::line(&p, "Hibernation", hib_sub, if checked.is_some() { container(t1("ready", 12.0, w::green())).padding(Padding { top: 2.0, right: 6.0, bottom: 2.0, left: 6.0 }).style(ui::boxed(Pal::a(w::green(), 0.15), 3.0, None)).into() } else { Space::new().into() }));
    let drive = w::card(&p, "Drive swap file", "The slow overflow on your drive (C:, /var/swap). Used only when zram is full, and for hibernation.", false, df);

    // When memory runs out
    let never: Vec<String> = val(a, "guard", "never", json!([])).as_array().map(|l| l.iter().map(s_of).collect()).unwrap_or_default();
    let labels = at(st, "neverLabels").as_object().cloned().unwrap_or_default();
    let mut chips_n: Vec<(String, Color, Msg)> = Vec::new();
    for (id, label) in labels.iter() {
        let on = never.contains(id);
        chips_n.push((s_of(label), if on { w::green() } else { p.muted }, Msg::Pick(format!("mem.never:{id}"), String::new())));
    }
    for x in val(a, "guard", "extra", json!([])).as_array().cloned().unwrap_or_default() {
        let n = s_of(&x);
        chips_n.push((format!("{n}  ✕"), w::green(), Msg::Pick(format!("mem.extra:{n}"), String::new())));
    }
    let never_box: E<'a> = {
        let chips = chip_list(&p, chips_n);
        let add: E<'a> = match &a.never_typing {
            Some(txt) => text_input("program name, Enter", txt).on_input(Msg::NeverInput).on_submit(Msg::NeverAdd).size(12.0).padding(5.0).width(Length::Fixed(170.0)).into(),
            None => mouse_area(container(t1("+ Add", 12.0, p.muted)).padding(Padding { top: 5.0, right: 10.0, bottom: 5.0, left: 10.0 }).style(ui::boxed(Color::TRANSPARENT, p.r(6.0), Some((p.line, 1.0))))).on_press(Msg::NeverInput(String::new())).interaction(mouse::Interaction::Pointer).into(),
        };
        column![chips, add].spacing(6.0).into()
    };
    let mut gd = vec![
        w::line(&p, "Protection", "", w::chips(&p, "mem.guard.level", &[("off", "Off"), ("gentle", "Gentle"), ("strong", "Strong")].iter().map(|(x, y)| (x.to_string(), y.to_string())).collect::<Vec<_>>(), &s_of(&val(a, "guard", "level", json!("gentle"))))),
        w::line(&p, "Never close", "always kept running (press one to take it off the list)", never_box),
    ];
    let cps: Vec<String> = val(a, "guard", "checkpoint", json!([])).as_array().map(|l| l.iter().map(s_of).collect()).unwrap_or_default();
    if !cps.is_empty() {
        gd.push(w::line(&p, "Pause instead of closing", "frozen, its memory moved to the drive; Resume in Task Manager", chip_list(&p, cps.iter().map(|n| (format!("{n}  ✕"), p.accent, Msg::Pick(format!("mem.checkpoint:{n}"), String::new()))).collect())));
    }
    gd.push(w::line(&p, "Close first", "the program using the most memory and swap", container(t1("automatic", 12.0, w::green())).padding(Padding { top: 2.0, right: 6.0, bottom: 2.0, left: 6.0 }).style(ui::boxed(Pal::a(w::green(), 0.15), 3.0, None)).into()));
    if let Some(lc) = at(st, "lastClosed").as_array().filter(|l| !l.is_empty()) {
        let list = lc
            .iter()
            .rev()
            .take(3)
            .map(|c| format!("{} ({}{:.1} GB, {})", s_of(at(c, "name")), if at(c, "paused") == &Value::Bool(true) { "paused, " } else { "" }, f_of(at(c, "mb")).unwrap_or(0.0) / 1024.0, hearth_ui::time::when(f_of(at(c, "at")).unwrap_or(0.0) as i64)))
            .collect::<Vec<_>>()
            .join(", ");
        gd.push(t(format!("Last closed: {list}"), 11.0, p.muted).into());
    }
    let guard = w::card(&p, "When memory runs out", "Instead of the whole PC freezing, Hearth closes the one program eating memory. It tells you which, and you can open it again.", true, gd);

    col = col.push(w::pair(zram, keep)).push(w::pair(drive, guard));

    // save
    let dirty = !a.mem_draft.is_empty();
    let (note, c) = if !a.mem_note.0.is_empty() {
        (a.mem_note.0.clone(), if a.mem_note.1 { p.danger } else { p.muted })
    } else if dirty {
        ("Not saved yet.".to_string(), p.flame)
    } else {
        ("Changes are saved and kept until you change them. Nothing here changes by itself.".to_string(), p.muted)
    };
    let mut bar = Row::new().spacing(10.0).align_y(iced::Center).push(container(t1(note, 13.0, c)).width(Length::Fill).clip(true)).push(mbtn(a, "Back to recommended", Kind::Normal, "mem.recommended"));
    if dirty {
        bar = bar.push(btn(&p, "Try until restart", Kind::Normal, false, if a.mem_busy { None } else { Some(Msg::Pick("mem.try".into(), String::new())) }));
    }
    bar = bar.push(mbtn(a, "Save", Kind::Primary, "mem.save"));
    col = col.push(ui::card(&p, bar).padding(Padding { top: 12.0, right: 12.0, bottom: 12.0, left: 18.0 }).width(Length::Fill));
    col.into()
}

// ------------------------------------------------------------------ changing
fn run_mem(a: &mut App, args: Vec<String>, stage: &str) -> Task<Msg> {
    if std::env::var_os("HEARTH_TM_FIXTURES").is_some() {
        a.mem_note = ("Saved ✓".into(), false);
        return Task::none();
    }
    a.mem_busy = true;
    let mut cmd = vec!["timeout".to_string(), "300".to_string(), "pkexec".to_string(), HELPER.to_string()];
    cmd.extend(args);
    let stage = stage.to_string();
    Task::perform(run::spawn(cmd), move |o| Msg::MemActed(stage.clone(), o))
}

fn changes(a: &App) -> Map<String, Value> {
    let mut c = Map::new();
    for k in ["zram", "keep", "swapfile", "guard"] {
        if let Some(v) = a.mem_draft.get(k) {
            c.insert(k.to_string(), v.clone());
        }
    }
    if let Some(v) = a.mem_draft.get("swappiness") {
        c.insert("swappiness".into(), v.clone());
    }
    c
}

fn save(a: &mut App, try_only: bool) -> Task<Msg> {
    let c = changes(a);
    let have = a.dev.get("mem").and_then(|s| f_of(at(s, "swapfile.gb"))).unwrap_or(0.0);
    let sw = a.mem_draft.get("swapGb").and_then(Value::as_f64).filter(|g| *g != have);
    a.mem_draft.clear();
    a.swap_custom = false;
    let mut set = vec!["set".to_string()];
    if try_only {
        set.push("--try".into());
    }
    set.push(Value::Object(c.clone()).to_string());
    a.mem_pending = if c.is_empty() { None } else { Some(set.clone()) };
    match sw {
        Some(g) if !try_only => run_mem(a, vec!["swapfile".into(), format!("{}", g.round())], "swapfile"),
        _ => {
            if c.is_empty() {
                a.mem_note = ("Saved ✓".into(), false);
                a.mem_pending = None;
                return Task::none();
            }
            a.mem_pending = None;
            run_mem(a, set, "set")
        }
    }
}

pub fn acted(a: &mut App, stage: String, o: run::Output) -> Task<Msg> {
    a.mem_busy = false;
    let last = o.out.trim().lines().last().unwrap_or("").to_string();
    if let Some(e) = last.strip_prefix("error: ") {
        a.mem_note = (e.to_string(), true);
        a.mem_pending = None;
        return Task::none();
    }
    if !o.ok && last.is_empty() {
        a.mem_note = (format!("That didn't work{}", if o.why().is_empty() { ".".to_string() } else { format!(": {}", o.why()) }), true);
        a.mem_pending = None;
        return Task::none();
    }
    let r: Value = serde_json::from_str(&last).unwrap_or(Value::Null);
    if stage == "swapfile" {
        if r.get("ok") == Some(&Value::Bool(true)) {
            return match a.mem_pending.take() {
                Some(set) => run_mem(a, set, "set"),
                None => {
                    a.mem_note = ("Saved ✓".into(), false);
                    refresh(a)
                }
            };
        }
        a.mem_pending = None;
        a.mem_note = ("The swap file didn't change".into(), true);
        return refresh(a);
    }
    let notes: Vec<String> = r.get("notes").and_then(Value::as_array).map(|l| l.iter().map(s_of).collect()).unwrap_or_default();
    let blocked = notes.iter().any(|n| n.contains("yours wins"));
    let joined = notes.join(" · ");
    if let Some(h) = r.get("handedOver").and_then(Value::as_array) {
        a.mem_note = (if h.is_empty() { "Nothing was in the way ✓".into() } else { format!("Your settings are in use now ✓ · kept as {}", h.iter().map(|x| s_of(at(x, "bak"))).collect::<Vec<_>>().join(", ")) }, false);
    } else {
        let head = if r.get("tried") == Some(&Value::Bool(true)) { "Tried until the next restart" } else if blocked { "Saved, but not all in use" } else { "Saved ✓" };
        a.mem_note = (format!("{head}{}", if joined.is_empty() { String::new() } else { format!(" · {joined}") }), blocked);
    }
    refresh(a)
}

fn refresh(a: &mut App) -> Task<Msg> {
    if a.dev_busy.insert("mem".into()) {
        crate::devset::read_task("mem", vec!["python3".into(), HELPER.into(), "status".into()], 20)
    } else {
        Task::none()
    }
}

/// A choice on Swap & settings: kept in the draft until Save (the buttons save, try or go back to recommended).
pub fn pick(a: &mut App, key: &str, v: &str) -> Task<Msg> {
    a.mem_note = (String::new(), false);
    if let Some(id) = key.strip_prefix("mem.never:") {
        let mut l: Vec<Value> = val(a, "guard", "never", json!([])).as_array().cloned().unwrap_or_default();
        if let Some(i) = l.iter().position(|x| s_of(x) == id) {
            l.remove(i);
        } else {
            l.push(json!(id));
        }
        put(a, "guard", "never", Value::Array(l));
        return Task::none();
    }
    for (prefix, field) in [("mem.extra:", "extra"), ("mem.checkpoint:", "checkpoint")] {
        if let Some(n) = key.strip_prefix(prefix) {
            let l: Vec<Value> = val(a, "guard", field, json!([])).as_array().cloned().unwrap_or_default().into_iter().filter(|x| s_of(x) != n).collect();
            put(a, "guard", field, Value::Array(l));
            return Task::none();
        }
    }
    match key {
        "mem.zram.on" => put(a, "zram", "on", Value::Bool(v == "on")),
        "mem.zram.algo" => put(a, "zram", "algo", json!(v)),
        "mem.swapfile.everyday" => put(a, "swapfile", "everyday", Value::Bool(v == "on")),
        "mem.guard.level" => put(a, "guard", "level", json!(v)),
        "mem.keep" | "mem.useAuto" => {
            let mode = if key == "mem.useAuto" { "auto" } else { v };
            if mode == "custom" {
                let f = keep_factor(a);
                a.mem_draft.insert("keep".into(), json!({"mode": "custom", "factor": f}));
            } else {
                a.mem_draft.insert("keep".into(), json!({ "mode": mode }));
            }
        }
        "mem.swapGb" => {
            if v == "custom" {
                a.swap_custom = true;
                let g = swap_gb(a);
                a.mem_draft.insert("swapGb".into(), json!(g));
            } else {
                a.swap_custom = false;
                a.mem_draft.insert("swapGb".into(), json!(v.parse::<f64>().unwrap_or(0.0)));
            }
        }
        "mem.save" => return save(a, false),
        "mem.try" => return save(a, true),
        "mem.recommended" => {
            a.mem_draft.clear();
            a.swap_custom = false;
            return run_mem(a, vec!["recommended".into()], "set");
        }
        "mem.adopt" => return run_mem(a, vec!["adopt".into()], "set"),
        "mem.handover" => return run_mem(a, vec!["handover".into()], "set"),
        _ => {}
    }
    Task::none()
}

/// A slider on Swap & settings moved: into the draft at once (as Mantel).
pub fn slide(a: &mut App, key: &str, v: f32) {
    let v = v.round() as f64;
    match key {
        "mem.zram.gb" => put(a, "zram", "gb", json!(v)),
        "mem.keep.factor" => {
            a.mem_draft.insert("keep".into(), json!({"mode": "custom", "factor": (v * 50.0).clamp(10.0, 3000.0)}));
        }
        "mem.swappiness" => {
            a.mem_draft.insert("swappiness".into(), json!(v));
        }
        "mem.swapGb.custom" => {
            a.mem_draft.insert("swapGb".into(), json!(v));
        }
        _ => {}
    }
}

/// + Add: any program, by its name (letters, digits and . + - _, up to 64).
pub fn never_add(a: &mut App) {
    let n = a.never_typing.take().unwrap_or_default().trim().to_string();
    let ok = !n.is_empty() && n.len() <= 64 && n.chars().all(|c| c.is_ascii_alphanumeric() || "._+-".contains(c));
    let mut l: Vec<Value> = val(a, "guard", "extra", json!([])).as_array().cloned().unwrap_or_default();
    if ok && !l.iter().any(|x| s_of(x) == n) {
        l.push(json!(n));
        put(a, "guard", "extra", Value::Array(l));
    }
}
