//! CaptureIndicator.qml in Rust (`hearth-capture-app --indicator`): the capture dot, a small pill at the top right
//! whenever this PC's screen is being taken somewhere, so it's never done without you seeing it. "Recording 1:23"
//! with Stop while the recorder records (hearth-record), and "<device> is connected" with Disconnect while someone
//! uses this PC with Remote desktop (hearth-vnc). It only reads the two helpers' runtime files
//! ($XDG_RUNTIME_DIR/hearth-record/state.json and hearth-vnc/clients.json), as Mantel's does, and writes nothing.
//! On Plasma there's no Mantel bar to hold it, so Capture starts it with a recording; it goes by itself a few
//! seconds after nothing is being taken any more (or after a minute when nothing started).

use crate::glyph::glyph;
use crate::kwin;
use hearth_ui::app;
use hearth_ui::ui::{boxed, weight};
use hearth_ui::Pal;
use iced::widget::{button, column, container, mouse_area, row, text, Space};
use iced::{font, window, Background, Border, Color, Element, Length, Padding, Shadow, Size, Subscription, Task, Theme};
use serde_json::Value;
use std::time::{Duration, Instant};

const RED: Color = Color { r: 0xe5 as f32 / 255.0, g: 0x48 as f32 / 255.0, b: 0x4d as f32 / 255.0, a: 1.0 };
const APP_ID: &str = "hearth-capture-dot";

#[derive(Debug, Clone)]
enum M {
    Tick,
    Theme,
    Stop,
    Disconnect,
    OpenRecorder,
    OpenRemote,
    Noop,
}

struct Ind {
    p: Pal,
    rec: Value,
    vnc: Value,
    start: Instant,
    seen: bool,
    idle_since: Option<Instant>,
    blink: bool,
    rows: usize,
}

fn runtime() -> std::path::PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR").map(std::path::PathBuf::from).unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
}

fn read(p: std::path::PathBuf) -> Value {
    std::fs::read_to_string(p).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or(Value::Null)
}

impl Ind {
    fn recording(&self) -> bool {
        matches!(self.rec.get("state").and_then(Value::as_str), Some("recording") | Some("stopping"))
    }
    fn viewers(&self) -> Vec<Value> {
        self.vnc.get("clients").and_then(Value::as_array).cloned().unwrap_or_default()
    }
    fn elapsed(&self) -> String {
        let since = self.rec.get("since").and_then(Value::as_f64).unwrap_or(0.0);
        if since <= 0.0 {
            return "0:00".into();
        }
        let s = ((hearth_ui::time::now() as f64 - since).max(0.0)) as i64;
        let (m, h) = (s / 60, s / 3600);
        if h > 0 {
            format!("{h}:{:02}:{:02}", m % 60, s % 60)
        } else {
            format!("{m}:{:02}", s % 60)
        }
    }
    fn who(&self) -> String {
        let v = self.viewers();
        let Some(first) = v.first() else { return String::new() };
        let n = first.get("name").and_then(Value::as_str).filter(|s| !s.is_empty()).or_else(|| first.get("ip").and_then(Value::as_str)).unwrap_or("Someone").to_string();
        if v.len() > 1 {
            format!("{n} and {} more", v.len() - 1)
        } else {
            n
        }
    }
    fn read_files(&mut self) {
        let r = runtime();
        self.rec = read(r.join("hearth-record").join("state.json"));
        self.vnc = read(r.join("hearth-vnc").join("clients.json"));
    }
}

fn boot() -> (Ind, Task<M>) {
    let mut i = Ind { p: hearth_ui::theme::load(), rec: Value::Null, vnc: Value::Null, start: Instant::now(), seen: false, idle_since: None, blink: true, rows: 1 };
    i.read_files();
    i.rows = (usize::from(i.recording()) + usize::from(!i.viewers().is_empty())).max(1);
    let h = height(i.rows);
    (i, Task::perform(kwin::place_async(APP_ID, 300.0, h, kwin::Place::TopRight(56.0, 12.0), String::new()), |_| M::Noop))
}

fn shell(name: &str) -> String {
    hearth_ui::run::shell_helper(name)
}

fn update(i: &mut Ind, m: M) -> Task<M> {
    match m {
        M::Tick => {
            i.read_files();
            i.blink = !i.blink;
            let n = usize::from(i.recording()) + usize::from(!i.viewers().is_empty());
            if n > 0 {
                i.seen = true;
                i.idle_since = None;
            } else if i.idle_since.is_none() {
                i.idle_since = Some(Instant::now());
            }
            let done = match i.idle_since {
                Some(t) if i.seen => t.elapsed() > Duration::from_secs(3),
                _ => !i.seen && i.start.elapsed() > Duration::from_secs(60),
            };
            if done {
                return iced::exit();
            }
            if n.max(1) != i.rows {
                i.rows = n.max(1);
                let h = height(i.rows);
                return window::latest().and_then(move |id| window::resize(id, Size::new(300.0, h)));
            }
        }
        M::Theme => i.p = hearth_ui::theme::load(),
        M::Stop => crate::helper::detached("python3", &[shell("hearth-record"), "stop".to_string()]),
        M::Disconnect => crate::helper::detached("python3", &[shell("hearth-vnc"), "disconnect".to_string(), "all".to_string()]),
        M::OpenRecorder => {
            if std::path::Path::new("/usr/bin/hearth-recorder").exists() {
                crate::helper::detached("/usr/bin/hearth-recorder", &[]);
            }
        }
        M::OpenRemote => crate::helper::detached("hearth-settings", &["remote-desktop".to_string()]),
        M::Noop => {}
    }
    Task::none()
}

fn pill<'a>(p: &Pal, rec: bool, words: String, act: &'static str, blink: bool, on_text: M, on_act: M) -> Element<'a, M> {
    let p = *p;
    let lead: Element<'a, M> = if rec {
        container(Space::new()).width(Length::Fixed(10.0)).height(Length::Fixed(10.0)).style(boxed(Color { a: if blink { 1.0 } else { 0.3 }, ..RED }, 5.0, None)).into()
    } else {
        glyph("eye", p.text, 16.0)
    };
    let hot = if rec { RED } else { p.accent };
    let act_b = button(text(act).size(12.0).color(p.text).font(weight(font::Weight::Semibold)))
        .padding(Padding { top: 4.0, right: 8.0, bottom: 4.0, left: 8.0 })
        .on_press(on_act)
        .style(move |_: &Theme, st: button::Status| {
            let hov = matches!(st, button::Status::Hovered | button::Status::Pressed);
            button::Style {
                background: Some(Background::Color(if hov { hot } else { p.panel3 })),
                text_color: if hov { Color::WHITE } else { p.text },
                border: Border { color: Color::TRANSPARENT, width: 0.0, radius: 12.0.into() },
                shadow: Shadow::default(),
                snap: true,
            }
        });
    let words = mouse_area(text(words).size(13.0).color(p.text).font(app::INTER)).on_press(on_text).interaction(iced::mouse::Interaction::Pointer);
    container(row![lead, words, act_b].spacing(8.0).align_y(iced::Center))
        .padding(Padding { top: 0.0, right: 6.0, bottom: 0.0, left: 12.0 })
        .center_y(Length::Fixed(34.0))
        .style(boxed(Pal::a(p.panel, 0.92), 17.0, Some((hot, 1.5))))
        .into()
}

fn view(i: &Ind) -> Element<'_, M> {
    let p = i.p;
    let mut col = column![].spacing(6.0).align_x(iced::Right);
    if i.recording() {
        col = col.push(pill(&p, true, format!("Recording {}", i.elapsed()), "Stop", i.blink, M::OpenRecorder, M::Stop));
    }
    let v = i.viewers();
    if !v.is_empty() {
        let words = format!("{} {} connected", i.who(), if v.len() > 1 { "are" } else { "is" });
        col = col.push(pill(&p, false, words, "Disconnect", i.blink, M::OpenRemote, M::Disconnect));
    }
    container(col).width(Length::Fill).height(Length::Fill).align_right(Length::Fill).style(boxed(p.bg, 0.0, None)).into()
}

fn subscription(_i: &Ind) -> Subscription<M> {
    // Mantel's dot reads the two files again every 2 seconds (a file that isn't there yet can't be watched); here
    // the 0.7 s tick also blinks the dot. Only while this runs: during a recording or a connection.
    Subscription::batch([iced::time::every(Duration::from_millis(700)).map(|_| M::Tick), Subscription::run(hearth_ui::theme::watch).map(|_| M::Theme)])
}

fn height(rows: usize) -> f32 {
    34.0 * rows as f32 + 6.0 * (rows as f32 - 1.0)
}

pub fn run() -> iced::Result {
    // Its size is set before it opens (a fixed-size window can't be resized afterwards everywhere).
    let mut probe = Ind { p: hearth_ui::theme::load(), rec: Value::Null, vnc: Value::Null, start: Instant::now(), seen: false, idle_since: None, blink: true, rows: 1 };
    probe.read_files();
    let rows = (usize::from(probe.recording()) + usize::from(!probe.viewers().is_empty())).max(1);
    let mut s = app::window(APP_ID, (300.0, height(rows)), (200.0, 34.0));
    s.resizable = false;
    s.level = window::Level::AlwaysOnTop;
    iced::application(boot, update, view)
        .title("Capture")
        .subscription(subscription)
        .theme(|i: &Ind| app::iced_theme(&i.p))
        .window(s)
        .default_font(app::INTER)
        .run()
}
