//! Reliability history, in pure Rust (Mantel's ReliabilityView.qml, the approved mockup, ported): a stability line
//! over 14 days (or weeks), a row of marks per kind (App stopped working, Hearth problems, Warnings, Updates and
//! changes), the days, and the picked day's events with Details and Send logs (problems only).
//!
//! Reading only: the numbers come from hearth_reliability.py (Mantel's shell folder), which reads the journal once
//! when this opens. Send logs is Hearth's own (hearth-send-logs), with the event as the note.
//! Mouse, keyboard and the controller (as keys): Left / Right pick a day, Up goes to Days / Weeks, Down goes into the
//! day's events (Up / Down between them, Left / Right between Details and Send logs), Enter presses, Esc closes.

use hearth_ui::app::{self, win};
use hearth_ui::run;
use hearth_ui::time;
use hearth_ui::ui::{self, btn, gap, hline, t, t1, tb, tm, ts, Graph, Kind, TitleMsgs, G};
use hearth_ui::Pal;
use iced::keyboard::{key::Named, Event as KEvent, Key};
use iced::widget::{column, container, mouse_area, row, scrollable, text, Column, Row, Space};
use iced::{mouse, Color, Element, Length, Padding, Subscription, Task};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Zone {
    Toggle,
    Chart,
    Events,
}

#[derive(Debug, Clone)]
enum Msg {
    /// Describe it (hearth_ui::describe; every Hearth app has it)
    Describe(hearth_ui::describe::DMsg),
    Theme,
    Summary(run::Output),
    Sent(run::Output),
    Weeks(bool),
    PickDay(usize),
    PickEvent(usize),
    Details(usize),
    Send(usize),
    SendOpen,
    CloseDetails,
    Close,
    Key(KEvent),
    Drag,
    Maximize,
    Minimize,
    Resize(iced::window::Direction),
}

struct App {
    pub describe: hearth_ui::describe::Describe,
    p: Pal,
    helper: String,
    summary: Option<Value>,
    weeks: bool,
    loading: bool,
    sel: usize,
    zone: Zone,
    ev: usize,
    ev_btn: usize,
    details: Option<Value>,
    dlg: usize,
    send_msg: String,
    sending: bool,
}

struct K {
    id: &'static str,
    label: &'static str,
    glyph: &'static str,
    round: bool,
}
static KINDS: [K; 4] = [
    K { id: "app", label: "App stopped working", glyph: "×", round: true },
    K { id: "hearth", label: "Hearth problems", glyph: "!", round: false },
    K { id: "warn", label: "Warnings", glyph: "!", round: false },
    K { id: "info", label: "Updates and changes", glyph: "i", round: true },
];
fn kind_of(id: &str) -> &'static K {
    KINDS.iter().find(|k| k.id == id).unwrap_or(&KINDS[3])
}

fn s(v: &Value, k: &str) -> String {
    match v.get(k) {
        Some(Value::String(x)) => x.clone(),
        Some(Value::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}
fn n(v: &Value, k: &str) -> i64 {
    v.get(k).and_then(|x| x.as_i64().or_else(|| x.as_f64().map(|f| f as i64))).unwrap_or(0)
}
fn problem(e: &Value) -> bool {
    matches!(s(e, "cat").as_str(), "app" | "hearth")
}

impl App {
    fn new() -> (App, Task<Msg>) {
        let helper = std::env::var("HEARTH_RELIABILITY_HELPER").unwrap_or_else(|_| run::shell_helper("hearth_reliability.py"));
        let a = App {
            describe: hearth_ui::describe::Describe::new("reliability", "Reliability history").with_pages(BLOCKS),
            p: hearth_ui::theme::load(),
            helper,
            summary: None,
            weeks: app::flag("--weeks"),
            loading: true,
            sel: 13,
            zone: Zone::Chart,
            ev: 0,
            ev_btn: 0,
            details: None,
            dlg: 0,
            send_msg: String::new(),
            sending: false,
        };
        let t = a.load();
        (a, t)
    }
    fn load(&self) -> Task<Msg> {
        let mut cmd = run::args(&["python3", &self.helper, "summary"]);
        if self.weeks {
            cmd.push("--weeks".into());
        }
        Task::perform(run::spawn(cmd), Msg::Summary)
    }
    fn rows(&self) -> Vec<Value> {
        self.summary.as_ref().and_then(|x| x.get("rows")).and_then(Value::as_array).cloned().unwrap_or_default()
    }
    fn cur(&self) -> Option<Value> {
        self.rows().get(self.sel).cloned()
    }
    fn events(&self) -> Vec<Value> {
        self.cur().and_then(|r| r.get("events").and_then(Value::as_array).cloned()).unwrap_or_default()
    }
    fn count(r: &Value, id: &str) -> i64 {
        r.get("counts").map(|c| n(c, id)).unwrap_or(0)
    }
    fn pick_default(&mut self) {
        let rows = self.rows();
        let mut sel = rows.len().saturating_sub(1);
        for (i, r) in rows.iter().enumerate().rev() {
            if App::count(r, "app") + App::count(r, "hearth") + App::count(r, "warn") > 0 {
                sel = i;
                break;
            }
        }
        self.sel = sel;
    }
    fn day_label(&self, i: usize, rows: &[Value]) -> String {
        let Some(r) = rows.get(i) else { return String::new() };
        if !self.weeks && i + 1 == rows.len() {
            return "Today".into();
        }
        let Some((_, m, d)) = time::parse_date(&s(r, "date")) else { return s(r, "date") };
        if self.weeks || i == 0 || d == 1 { format!("{} {d}", time::month(m)) } else { d.to_string() }
    }
    fn long_day(&self, r: &Value) -> String {
        let date = s(r, "date");
        let Some((_, m, d)) = time::parse_date(&date) else { return date };
        if self.weeks {
            return format!("Week of {} {d}", time::month(m));
        }
        if self.summary.as_ref().map(|x| s(x, "today")) == Some(date) { "Today".into() } else { format!("{} {d}", time::month(m)) }
    }
    fn time_of(&self, ts: i64) -> String {
        if self.weeks { format!("{}, {}", time::day(ts), time::clock(ts)) } else { time::clock(ts) }
    }
    fn open_details(&mut self, e: Value) {
        self.details = Some(e);
        self.dlg = 0;
        self.send_msg.clear();
    }
    fn send(&mut self) -> Task<Msg> {
        let Some(e) = self.details.clone() else { return Task::none() };
        if self.sending {
            return Task::none();
        }
        self.sending = true;
        self.send_msg.clear();
        let sub = s(&e, "s");
        let note = format!("Reliability history: {} · {}{}", s(&e, "t"), time::when(n(&e, "time")), if sub.is_empty() { String::new() } else { format!(" · {sub}") });
        let note: String = note.chars().take(400).collect();
        Task::perform(run::spawn(vec!["hearth-send-logs".into(), note]), Msg::Sent)
    }
    fn dlg_buttons(&self) -> Vec<&'static str> {
        match &self.details {
            Some(d) if problem(d) => vec!["send", "close"],
            Some(_) => vec!["close"],
            None => vec![],
        }
    }
    fn set_weeks(&mut self, w: bool) -> Task<Msg> {
        if self.weeks == w {
            return Task::none();
        }
        self.weeks = w;
        self.summary = None;
        self.loading = true;
        self.load()
    }
}

fn update(a: &mut App, m: Msg) -> Task<Msg> {
    match m {
        Msg::Describe(dm) => return a.describe.update(dm).map(Msg::Describe),
        Msg::Theme => a.p = hearth_ui::theme::load(),
        Msg::Summary(o) => {
            let d = o.json().filter(|d| d.get("rows").map(Value::is_array).unwrap_or(false));
            a.summary = Some(d.unwrap_or_else(|| serde_json::json!({"rows": []})));
            a.loading = false;
            a.pick_default();
            a.ev = 0;
            // CI's pictures: --show details opens the picked day's first event.
            if app::arg("--show").as_deref() == Some("details") {
                if let Some(e) = a.events().into_iter().next() {
                    a.open_details(e);
                }
            }
        }
        Msg::Sent(o) => {
            a.sending = false;
            let code = o.out.split("code: ").nth(1).and_then(|x| x.split_whitespace().next()).map(String::from);
            a.send_msg = match (o.ok, code) {
                (true, Some(c)) => format!("Sent. Share this code with the developer: {c}"),
                _ => {
                    let why = o.why();
                    format!("Couldn't send them: {}", if why.is_empty() { "Hearth's Send logs isn't on this system".into() } else { why })
                }
            };
        }
        Msg::Weeks(w) => return a.set_weeks(w),
        Msg::PickDay(i) => {
            a.sel = i.min(a.rows().len().saturating_sub(1));
            a.ev = 0;
            a.zone = Zone::Chart;
        }
        Msg::PickEvent(i) => {
            a.zone = Zone::Events;
            a.ev = i;
        }
        Msg::Details(i) => {
            if let Some(e) = a.events().get(i).cloned() {
                a.open_details(e);
            }
        }
        Msg::Send(i) => {
            if let Some(e) = a.events().get(i).cloned() {
                a.open_details(e);
                return a.send();
            }
        }
        Msg::SendOpen => return a.send(),
        Msg::CloseDetails => a.details = None,
        Msg::Close => return win::close(),
        Msg::Drag => return win::drag(),
        Msg::Maximize => return win::toggle_maximize(),
        Msg::Minimize => return win::minimize(),
        Msg::Resize(d) => return win::resize(d),
        Msg::Key(ev) => return key(a, ev),
    }
    Task::none()
}

fn key(a: &mut App, ev: KEvent) -> Task<Msg> {
    let KEvent::KeyPressed { key, .. } = ev else { return Task::none() };
    let Key::Named(k) = key.as_ref() else { return Task::none() };
    let enter = matches!(k, Named::Enter | Named::Space);
    if a.details.is_some() {
        let bs = a.dlg_buttons();
        match k {
            Named::Escape => a.details = None,
            Named::ArrowLeft => a.dlg = a.dlg.saturating_sub(1),
            Named::ArrowRight | Named::Tab => a.dlg = (a.dlg + 1).min(bs.len().saturating_sub(1)),
            _ if enter => {
                if bs.get(a.dlg) == Some(&"send") {
                    return a.send();
                }
                a.details = None;
            }
            _ => {}
        }
        return Task::none();
    }
    if k == Named::Escape {
        return win::close();
    }
    let evs = a.events();
    match a.zone {
        Zone::Toggle => match k {
            Named::ArrowLeft => return a.set_weeks(false),
            Named::ArrowRight => return a.set_weeks(true),
            Named::ArrowDown | Named::Tab => a.zone = Zone::Chart,
            Named::Enter => {
                let w = !a.weeks;
                return a.set_weeks(w);
            }
            _ => {}
        },
        Zone::Chart => match k {
            Named::ArrowLeft => {
                a.sel = a.sel.saturating_sub(1);
                a.ev = 0
            }
            Named::ArrowRight => {
                a.sel = (a.sel + 1).min(a.rows().len().saturating_sub(1));
                a.ev = 0
            }
            Named::ArrowUp => a.zone = Zone::Toggle,
            Named::ArrowDown | Named::Tab | Named::Enter if !evs.is_empty() => {
                a.zone = Zone::Events;
                a.ev = 0;
                a.ev_btn = 0
            }
            _ => {}
        },
        Zone::Events => match k {
            Named::ArrowUp => {
                if a.ev > 0 {
                    a.ev -= 1
                } else {
                    a.zone = Zone::Chart
                }
            }
            Named::ArrowDown => a.ev = (a.ev + 1).min(evs.len().saturating_sub(1)),
            Named::ArrowLeft => a.ev_btn = 0,
            Named::ArrowRight => {
                if evs.get(a.ev).map(problem).unwrap_or(false) {
                    a.ev_btn = 1
                }
            }
            Named::Tab => a.zone = Zone::Toggle,
            Named::Enter => {
                if let Some(e) = evs.get(a.ev).cloned() {
                    a.open_details(e);
                    if a.ev_btn == 1 {
                        return a.send();
                    }
                }
            }
            _ => {}
        },
    }
    Task::none()
}

fn mark<'a>(p: &Pal, k: &K, size: f32) -> Element<'a, Msg> {
    let c = match k.id {
        "app" | "hearth" => p.danger,
        "warn" => p.warn,
        _ => p.accent,
    };
    let ink = if p.dark { Color::from_rgb8(0x06, 0x12, 0x1c) } else { Color::WHITE };
    container(text(k.glyph).size(11.0).color(ink).font(ui::weight(iced::font::Weight::Black)))
        .center_x(Length::Fixed(size))
        .center_y(Length::Fixed(size))
        .style(ui::boxed(c, if k.round { size / 2.0 } else { 4.0 }, None))
        .into()
}

const LABEL_W: f32 = 150.0;

/// Reliability history's blocks (hearth_ui::blocks).
const BLOCKS: &[hearth_ui::blocks::PageDef] = &[hearth_ui::blocks::PageDef {
    id: "main",
    title: "Reliability history",
    blocks: &[
        hearth_ui::blocks::bl("header", "Heading", 0, 0, 12, 2),
        hearth_ui::blocks::bl("chart", "The chart", 0, 2, 12, 5),
        hearth_ui::blocks::bl("events", "The day's events", 0, 7, 12, 5),
    ],
}];

fn view(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    let rows = a.rows();
    let title = hearth_ui::describe::title_bar(&p, &a.describe, G::Reliability, "Reliability history", None, TitleMsgs { drag: Msg::Drag, maximize: Msg::Maximize, minimize: None, close: Msg::Close }, Pal::mix(p.bg, Color::BLACK, 0.3), Msg::Describe(hearth_ui::describe::DMsg::Open));

    // ---- header with Days / Weeks
    let toggle_btn = |label: &'static str, w: bool| -> Element<'static, Msg> {
        let on = a.weeks == w;
        mouse_area(
            container(if on { ts(label, 13.0, p.text) } else { t(label, 13.0, p.muted) })
                .padding(Padding { top: 5.0, right: 12.0, bottom: 5.0, left: 12.0 })
                .style(ui::boxed(if on { p.panel } else { Color::TRANSPARENT }, p.r(6.0), None)),
        )
        .on_press(Msg::Weeks(w))
        .interaction(mouse::Interaction::Pointer)
        .into()
    };
    let toggle = ui::ring(p, a.zone == Zone::Toggle, 6.0, row![toggle_btn("Days", false), toggle_btn("Weeks", true)].spacing(4.0).into());
    let what = if a.weeks { ("week by week", "week") } else { ("day by day", "day") };
    let header = container(
        row![
            column![
                ts("How Hearth has been running", 16.0, p.text),
                t1(format!("Problems and changes on this PC, {}. Click a {} to see what happened.", what.0, what.1), 13.0, p.muted)
            ]
            .spacing(2.0)
            .width(Length::Fill),
            toggle
        ]
        .align_y(iced::Center)
        .spacing(16.0),
    )
    .padding(Padding { top: 0.0, right: 20.0, bottom: 0.0, left: 20.0 })
    .center_y(Length::Fixed(64.0))
    .width(Length::Fill)
    .style(ui::boxed(p.bg2, 0.0, None));

    // ---- the chart: stability line, the four rows of marks, the days. Each column is one day (or week).
    let grid = Pal::mix(p.panel, p.line, 0.5);
    let scores: Vec<f32> = rows.iter().map(|r| r.get("score").and_then(Value::as_f64).unwrap_or(10.0) as f32).collect();
    let line: Element<'static, Msg> = ui::graph(
        Graph { values: scores, dashed: vec![], max: 10.0, min: 1.0, color: p.accent, grid, fill: false, width: 3.0, centred: true, gridlines: vec![10.0, 5.5, 1.0] },
        Length::Fill,
        Length::Fixed(110.0),
    );
    let ylabels = container(
        column![
            container(t1("10 · steady", 11.0, p.muted)).align_right(Length::Fill),
            Space::new().height(Length::Fill),
            container(t1("5", 11.0, p.muted)).align_right(Length::Fill),
            Space::new().height(Length::Fill),
            container(t1("1 · rocky", 11.0, p.muted)).align_right(Length::Fill),
        ]
        .height(Length::Fixed(110.0)),
    )
    .width(Length::Fixed(LABEL_W - 10.0));
    let line_row = row![ylabels, gap(10.0, 1.0), line];

    let col_cell = |i: usize, inner: Element<'static, Msg>, h: f32| -> Element<'static, Msg> {
        let picked = i == a.sel;
        let bg = if picked { Pal::mix(p.panel, p.accent, 0.18) } else { Color::TRANSPARENT };
        // The picked column, ringed in the accent while the chart has the keyboard (as Mantel's).
        let line = if picked && a.zone == Zone::Chart { Some((p.accent, 1.0)) } else { None };
        mouse_area(container(inner).center_x(Length::FillPortion(1)).center_y(Length::Fixed(h)).style(ui::boxed(bg, 0.0, line)))
            .on_press(Msg::PickDay(i))
            .interaction(mouse::Interaction::Pointer)
            .into()
    };
    let mut marks = Column::new();
    for k in KINDS.iter() {
        let mut r = Row::new().push(container(t1(k.label, 12.0, Pal::mix(p.text, p.muted, 0.3))).width(Length::Fixed(LABEL_W)).center_y(Length::Fixed(30.0)));
        for (i, day) in rows.iter().enumerate() {
            let inner: Element<'static, Msg> = if App::count(day, k.id) > 0 { mark(&p, k, 18.0) } else { gap(18.0, 18.0) };
            r = r.push(col_cell(i, inner, 30.0));
        }
        marks = marks.push(hline(grid)).push(r);
    }
    let mut days = Row::new().push(Space::new().width(Length::Fixed(LABEL_W)));
    for i in 0..rows.len() {
        let l = a.day_label(i, &rows);
        let txt: Element<'static, Msg> = if i == a.sel { ts(l, 11.0, p.text).wrapping(text::Wrapping::None).into() } else { t1(l, 11.0, p.muted).into() };
        days = days.push(col_cell(i, txt, 26.0));
    }
    let chart = container(column![line_row, gap(1.0, 10.0), marks, hline(grid), days]).padding(Padding { top: 12.0, right: 20.0, bottom: 12.0, left: 20.0 });

    // ---- the day's events
    let evs = a.events();
    let heading = if a.loading {
        "Reading the system's log…".to_string()
    } else if let Some(r) = a.cur() {
        format!("{} · {} {}", a.long_day(&r), evs.len(), if evs.len() == 1 { "event" } else { "events" })
    } else {
        "Nothing to show yet".into()
    };
    let mut list = Column::new();
    for (i, e) in evs.iter().enumerate() {
        let here = a.zone == Zone::Events && a.ev == i;
        let k = kind_of(&s(e, "cat"));
        let mut btns = Row::new().spacing(6.0).push(btn(&p, "Details", Kind::Normal, here && a.ev_btn == 0, Some(Msg::Details(i))));
        if problem(e) {
            btns = btns.push(btn(&p, "Send logs", Kind::Primary, here && a.ev_btn == 1, Some(Msg::Send(i))));
        }
        let line = row![
            container(mark(&p, k, 18.0)).width(Length::Fixed(28.0)),
            column![tm(ui::fit(&s(e, "t"), 90), 13.0, p.text).wrapping(text::Wrapping::None), t1(ui::fit(&s(e, "s"), 110), 12.0, p.muted)].spacing(2.0).width(Length::Fill),
            container(t1(a.time_of(n(e, "time")), 13.0, p.muted)).width(Length::Fixed(if a.weeks { 130.0 } else { 90.0 })),
            container(btns).align_right(Length::Fixed(200.0)),
        ]
        .spacing(10.0)
        .align_y(iced::Center)
        .padding(Padding { top: 0.0, right: 14.0, bottom: 0.0, left: 14.0 });
        let bg = if here { p.sel() } else { Color::TRANSPARENT };
        list = list.push(mouse_area(container(line).center_y(Length::Fixed(52.0)).width(Length::Fill).style(ui::boxed(bg, 0.0, None))).on_press(Msg::PickEvent(i)));
        list = list.push(hline(grid));
    }
    if !a.loading && a.cur().is_some() && evs.is_empty() {
        list = list.push(container(t(format!("Nothing happened {}. Hearth ran steadily.", if a.weeks { "this week" } else { "on this day" }), 13.0, p.muted)).padding(14.0));
    }
    let events = container(
        column![
            container(ts(heading, 13.0, p.text)).padding(Padding { top: 0.0, right: 14.0, bottom: 0.0, left: 14.0 }).center_y(Length::Fixed(40.0)),
            hline(p.line),
            scrollable(list).height(Length::Fill)
        ],
    )
    .padding(1.0)
    .width(Length::Fill)
    .height(Length::Fill)
    .style(ui::boxed(p.panel, p.r(10.0), Some((p.line, 1.0))));

    // its blocks (hearth_ui::blocks): moved, sized, hidden or restyled; as it was when nothing is
    let blocks = hearth_ui::arrange::parts(
        &a.describe,
        &p,
        "main",
        vec![("header", header.into()), ("chart", chart.into()), ("events", events.into())],
        |ps| column![ps.take("header"), hline(p.line), ps.take("chart"), container(ps.take("events")).padding(Padding { top: 0.0, right: 20.0, bottom: 16.0, left: 20.0 }).height(Length::Fill)].into(),
        Msg::Describe,
    );
    let page = column![title, blocks];
    let base: Element<Msg> = container(page).width(Length::Fill).height(Length::Fill).style(ui::boxed(p.bg, 0.0, None)).into();
    let base = match &a.details {
        Some(d) => ui::dialog(base, details(a, d), Some(Msg::CloseDetails)),
        None => base,
    };
    ui::edges(base, Msg::Resize)
}

fn details<'a>(a: &'a App, d: &'a Value) -> Element<'a, Msg> {
    let p = a.p;
    let k = kind_of(&s(d, "cat"));
    let mut col = Column::new().spacing(10.0);
    col = col.push(row![mark(&p, k, 18.0), tb(s(d, "t"), 16.0, p.text).width(Length::Fill)].spacing(10.0).align_y(iced::Center));
    col = col.push(t(time::long(n(d, "time")), 12.0, p.muted));
    let sub = s(d, "s");
    if !sub.is_empty() {
        col = col.push(t(sub, 13.0, p.text));
    }
    let detail = s(d, "detail");
    if !detail.is_empty() {
        col = col.push(
            container(text(detail).size(12.0).color(Pal::mix(p.text, p.muted, 0.3)).font(iced::Font::MONOSPACE).wrapping(text::Wrapping::Glyph))
                .padding(10.0)
                .width(Length::Fill)
                .style(ui::boxed(p.bg2, 6.0, None)),
        );
    }
    if a.sending || !a.send_msg.is_empty() {
        col = col.push(t(if a.sending { "Sending Hearth's logs…".to_string() } else { a.send_msg.clone() }, 13.0, p.muted));
    }
    let mut bs = Row::new().spacing(8.0);
    for (i, b) in a.dlg_buttons().into_iter().enumerate() {
        bs = bs.push(if b == "send" {
            btn(&p, if a.sending { "Sending…" } else { "Send logs" }, Kind::Primary, a.dlg == i, Some(Msg::SendOpen))
        } else {
            btn(&p, "Close", Kind::Normal, a.dlg == i, Some(Msg::CloseDetails))
        });
    }
    col = col.push(container(bs).align_right(Length::Fill));
    ui::dialog_box(&p, 600.0, col)
}

fn subscription(_a: &App) -> Subscription<Msg> {
    Subscription::batch([Subscription::run(hearth_ui::theme::watch).map(|_| Msg::Theme), iced::keyboard::listen().map(Msg::Key)])
}

/// The app with Describe it's box under it when it's open (every Hearth app has it).
fn shown(a: &App) -> Element<'_, Msg> {
    hearth_ui::describe::with_bar(view(a), &a.p, &a.describe, Msg::Describe)
}

fn main() -> iced::Result {
    if app::handle_cli("hearth-reliability-history", "Reliability history: how Hearth has been running.\n  --weeks         open week by week\n  --show details  open the picked day's first event (pictures)") {
        return Ok(());
    }
    iced::application(App::new, update, shown)
        .title("Reliability history")
        .subscription(subscription)
        .theme(|a: &App| app::iced_theme(&a.p))
        .window(app::window("hearth-reliability", (1000.0, 680.0), (760.0, 520.0)))
        .default_font(app::INTER)
        .run()
}
