//! The Home server helper's Game servers, Game cache, Remote play, Media and PCs pages (boards
//! docs/mockups/server-expansion/05, 07, 08, 09 and 10). Each page is its own module (games.rs, cache.rs, play.rs,
//! media.rs, pcs.rs) and is made of blocks (its PageDef in [`BLOCKS`], laid out by hearth_ui::arrange, so Edit layout
//! and Describe it work on every part). The work is Mantel's `hearth-server-hub` (it comes and updates with Mantel's
//! Store part): each page asks it for its view (`view games`) and every button is one of its commands, so this crate
//! only shows and asks.
//!
//! Hooking a page into the Home server helper's window is one line each: its sidebar lists [`SIDE`] (id, name, Part),
//! keeps one [`Hub`], sends [`Msg`] through [`Hub::update`] and draws [`Hub::view`] for the open Part; Describe's
//! DMsg comes back as [`Msg::D`] for the window's own Describe.

#[path = "../../oss-kit/kit.rs"]
pub mod kit;
pub mod cache;
pub mod games;
pub mod media;
pub mod pcs;
pub mod play;

use hearth_ui::blocks::PageDef;
use hearth_ui::describe::{DMsg, Describe};
use hearth_ui::ui::{self, t, tb, ts};
use hearth_ui::Pal;
use iced::widget::canvas::{self, Frame, Geometry, Path};
use iced::widget::{column, container, row, Column, Row, Space};
use iced::{mouse, Color, Element, Length, Point, Rectangle, Renderer, Size, Task, Theme};
use kit::{pad, E};
use serde_json::Value;

/// The five pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    Games,
    Cache,
    Play,
    Media,
    Pcs,
}

impl Part {
    pub const ALL: [Part; 5] = [Part::Games, Part::Cache, Part::Play, Part::Media, Part::Pcs];
    pub fn id(self) -> &'static str {
        match self {
            Part::Games => "games",
            Part::Cache => "cache",
            Part::Play => "play",
            Part::Media => "media",
            Part::Pcs => "pcs",
        }
    }
    pub fn title(self) -> &'static str {
        match self {
            Part::Games => "Game servers",
            Part::Cache => "Game cache",
            Part::Play => "Remote play",
            Part::Media => "Media",
            Part::Pcs => "PCs",
        }
    }
    pub fn of(s: &str) -> Option<Part> {
        Part::ALL.iter().copied().find(|p| p.id() == s)
    }
    fn i(self) -> usize {
        self as usize
    }
}

/// The Home server helper's sidebar lines for these pages, in the board's order: (id, name, Part).
pub const SIDE: &[(&str, &str, Part)] = &[
    ("games", "Game servers", Part::Games),
    ("cache", "Game cache", Part::Cache),
    ("play", "Remote play", Part::Play),
    ("media", "Media", Part::Media),
    ("pcs", "PCs", Part::Pcs),
];

#[derive(Debug, Clone)]
pub enum Msg {
    /// for the window's Describe it (Edit layout, the blocks)
    D(DMsg),
    Load(Part),
    Loaded(Part, Result<Value, String>),
    /// a hearth-server-hub command for that page, and what to say when it worked
    Act(Part, Vec<String>, String),
    Done(Part, Result<Value, String>, String),
    /// one of the page's choices (set PART KEY JSON)
    Set(Part, &'static str, String),
    /// the game's side: Players, Mods, Worlds, Settings, Console
    GTab(&'static str),
    /// Remote play's library: Played lately, Steam, Your games, Retro, The whole desktop
    PTab(&'static str),
    Typed(String),
    Open(String),
    Copy(String),
    /// a list opened on a page (the cache's drives), or closed with ""
    Pick(&'static str),
    /// Remote play: the game whose "on the … ▾" list is open (pick a device to start it on)
    PGame(String),
    Tick,
}

/// Everything the five pages show: each page's view from hearth-server-hub, and the little the window itself
/// remembers while it's open (which tab, what's being typed).
pub struct Hub {
    pub v: [Value; 5],
    pub loaded: [bool; 5],
    pub problem: [String; 5],
    pub said: String,
    pub busy: bool,
    pub gtab: &'static str,
    pub ptab: &'static str,
    pub console: Vec<String>,
    pub typed: String,
    pub pick: &'static str,
    pub pgame: String,
}

impl Default for Hub {
    fn default() -> Self {
        Hub {
            v: Default::default(),
            loaded: [false; 5],
            problem: Default::default(),
            said: String::new(),
            busy: false,
            gtab: "Players",
            ptab: "Played lately",
            console: vec![],
            typed: String::new(),
            pick: "",
            pgame: String::new(),
        }
    }
}

/// `python3 <Mantel's folder>/hearth-server-hub ARGS`, stopped after `secs`.
pub fn hub_cmd(args: Vec<String>, secs: u32) -> Vec<String> {
    let mut v = vec!["timeout".to_string(), secs.to_string(), "python3".into(), hearth_ui::run::shell_helper("hearth-server-hub")];
    v.extend(args);
    v
}

fn answer(o: hearth_ui::run::Output) -> Result<Value, String> {
    if o.ok {
        if let Some(v) = o.json() {
            return Ok(v);
        }
    }
    let why = o.why();
    Err(if why.is_empty() { "the Home server helper didn't answer".into() } else { why })
}

/// Its page's view, read off the window's thread.
pub fn load(part: Part) -> Task<Msg> {
    let c = hub_cmd(vec!["view".into(), part.id().into()], 60);
    Task::perform(hearth_ui::run::spawn(c), move |o| Msg::Loaded(part, answer(o)))
}

impl Hub {
    pub fn new() -> Hub {
        Hub::default()
    }
    /// The page's view as hearth-server-hub made it.
    pub fn view_of(&self, part: Part) -> &Value {
        &self.v[part.i()]
    }
    /// The window's status tag for the open page ("3 running · 7 players").
    pub fn tag(&self, part: Part) -> String {
        s(&self.v[part.i()], "tag")
    }
    /// The sidebar's count for a page ("3 on", "612 GB").
    pub fn count(&self, part: Part) -> String {
        s(&self.v[part.i()], "count")
    }
    pub fn update(&mut self, m: Msg) -> Task<Msg> {
        match m {
            Msg::D(_) => {}
            Msg::Load(part) => return load(part),
            Msg::Loaded(part, r) => {
                self.loaded[part.i()] = true;
                match r {
                    Ok(v) => {
                        self.v[part.i()] = v;
                        self.problem[part.i()].clear();
                    }
                    Err(e) => self.problem[part.i()] = e,
                }
            }
            Msg::Act(part, args, ok) => {
                self.busy = true;
                let c = hub_cmd(args, 900);
                return Task::perform(hearth_ui::run::spawn(c), move |o| Msg::Done(part, answer(o), ok.clone()));
            }
            Msg::Done(part, r, ok) => {
                self.busy = false;
                match r {
                    Ok(v) => {
                        if let Some(lines) = v.get("lines").and_then(Value::as_array) {
                            self.console = lines.iter().filter_map(Value::as_str).map(str::to_string).collect();
                            self.gtab = "Console";
                        }
                        if let Some(u) = v.get("open").and_then(Value::as_str) {
                            open_url(u);
                        }
                        if let Some(x) = v.get("say").and_then(Value::as_str) {
                            self.said = x.to_string();
                        } else if let Some(a) = v.get("address").and_then(Value::as_str) {
                            self.said = format!("{ok} {a}");
                        } else {
                            self.said = ok;
                        }
                    }
                    Err(e) => self.said = format!("That didn't work: {e}"),
                }
                return load(part);
            }
            Msg::Set(part, key, json) => {
                return self.update(Msg::Act(part, vec!["set".into(), part.id().into(), key.into(), json], "Kept.".into()));
            }
            Msg::GTab(x) => self.gtab = x,
            Msg::Pick(x) => self.pick = if self.pick == x { "" } else { x },
            Msg::PTab(x) => self.ptab = x,
            Msg::PGame(g) => self.pgame = if self.pgame == g { String::new() } else { g },
            Msg::Typed(x) => self.typed = x.chars().take(200).collect(),
            Msg::Open(u) => open_url(&u),
            Msg::Copy(x) => {
                self.said = "Copied.".into();
                return iced::clipboard::write(x);
            }
            Msg::Tick => {}
        }
        Task::none()
    }
    /// The open page: everything right of the sidebar (its page and its right side).
    pub fn view<'a>(&'a self, part: Part, d: &'a Describe, p: Pal) -> E<'a, Msg> {
        if !self.loaded[part.i()] {
            return container(kit::missing(&p, format!("Reading {}…", part.title()), "Asking the home server.", None, true)).padding(pad(22.0, 26.0, 22.0, 26.0)).into();
        }
        if !self.problem[part.i()].is_empty() && self.v[part.i()].is_null() {
            return container(kit::missing(&p, format!("{} isn't answering", part.title()), self.problem[part.i()].clone(), Some(("Try again".into(), Msg::Load(part))), false))
                .padding(pad(22.0, 26.0, 22.0, 26.0))
                .into();
        }
        match part {
            Part::Games => games::view(self, d, p),
            Part::Cache => cache::view(self, d, p),
            Part::Play => play::view(self, d, p),
            Part::Media => media::view(self, d, p),
            Part::Pcs => pcs::view(self, d, p),
        }
    }
}

fn open_url(u: &str) {
    if u.starts_with("https://") || u.starts_with("hearth-settings://") {
        hearth_ui::run::detached(&["xdg-open", u]);
    }
}

/// Every page's blocks, for the window's Describe (`.with_pages`): its own pages first, then these.
pub fn blocks() -> Vec<PageDef> {
    BLOCKS.to_vec()
}

pub const BLOCKS: &[PageDef] = &[games::PAGE, games::PANEL, cache::PAGE, cache::PANEL, play::PAGE, play::PANEL, media::PAGE, media::PANEL, pcs::PAGE, pcs::PANEL];

// ------------------------------------------------------------------ reading a view

pub(crate) fn s(v: &Value, k: &str) -> String {
    match v.get(k) {
        Some(Value::String(x)) => x.clone(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Bool(b)) => b.to_string(),
        _ => String::new(),
    }
}
pub(crate) fn at<'a>(v: &'a Value, path: &str) -> &'a Value {
    v.pointer(path).unwrap_or(&Value::Null)
}
pub(crate) fn sv(v: &Value) -> String {
    match v {
        Value::String(x) => x.clone(),
        Value::Number(n) => n.to_string(),
        _ => String::new(),
    }
}
pub(crate) fn on(v: &Value, k: &str) -> bool {
    v.get(k).and_then(Value::as_bool).unwrap_or(false)
}
pub(crate) fn num(v: &Value) -> f32 {
    v.as_f64().unwrap_or(0.0) as f32
}
pub(crate) fn list<'a>(v: &'a Value, k: &str) -> &'a [Value] {
    v.get(k).and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
}
pub(crate) fn colour(v: &Value, fallback: Color) -> Color {
    v.as_str().and_then(kit::hex).unwrap_or(fallback)
}

// ------------------------------------------------------------------ the boards' small parts

pub(crate) const ROWLINE: Color = Color::from_rgb(0x2c as f32 / 255.0, 0x26 as f32 / 255.0, 0x22 as f32 / 255.0);

/// A page's title, its line, its buttons on the right, and what just happened.
pub(crate) fn head<'a>(h: &'a Hub, p: Pal, title: &'a str, line: String, right: Vec<E<'a, Msg>>) -> E<'a, Msg> {
    let mut c = column![tb(title, 26.0, p.text), t(line, 12.0, p.muted)].spacing(4.0);
    if !h.said.is_empty() {
        c = c.push(t(h.said.clone(), 12.0, kit::YELLOW));
    }
    let mut r = Row::new().push(container(c).width(Length::Fill)).spacing(10.0).align_y(iced::Bottom);
    for x in right {
        r = r.push(x);
    }
    r.into()
}

/// `.kpi`: its name in small capitals, its number, its line (and a bar under it when `bar` is given).
pub(crate) fn kpi<'a>(p: Pal, k: String, v: E<'a, Msg>, sub: Option<String>, bar: Option<f32>) -> E<'a, Msg> {
    let mut c = column![kit::caps(&p, k), v].spacing(4.0);
    if let Some(x) = sub {
        c = c.push(t(x, 12.0, p.muted));
    }
    if let Some(fr) = bar {
        c = c.push(container(ui::bar(vec![(fr.clamp(0.0, 1.0), p.accent)], p.line, 4.0, 2.0)).padding(pad(6.0, 0.0, 0.0, 0.0)));
    }
    container(c).padding(pad(12.0, 16.0, 12.0, 16.0)).width(Length::Fill).style(ui::boxed(p.panel, 12.0, Some((p.line, 1.0)))).into()
}

/// A line with words on the left and something on the right (`.row`).
pub(crate) fn line<'a>(left: E<'a, Msg>, right: E<'a, Msg>) -> E<'a, Msg> {
    row![container(left).width(Length::Fill), right].spacing(10.0).align_y(iced::Center).padding(pad(5.0, 0.0, 5.0, 0.0)).into()
}

/// Words with a smaller muted line under them.
pub(crate) fn two<'a>(p: Pal, a: String, b: String) -> E<'a, Msg> {
    if b.is_empty() {
        return t(a, 13.0, p.text).into();
    }
    column![t(a, 13.0, p.text), t(b, 11.0, p.muted)].spacing(1.0).into()
}

/// A status dot (`.st`): green, yellow, red or off.
pub(crate) fn st<'a>(kind: &str) -> E<'a, Msg> {
    let c = match kind {
        "w" | "wait" => kit::YELLOW,
        "b" | "bad" => kit::RED,
        "o" | "off" => kit::rgb(0x6d, 0x64, 0x5d),
        _ => kit::GREEN,
    };
    kit::dot(c, 10.0)
}

/// `.box`: the second panel colour, a border, 10 px corners.
pub(crate) fn boxed<'a>(p: Pal, inner: impl Into<E<'a, Msg>>) -> container::Container<'a, Msg> {
    container(inner).padding(pad(12.0, 14.0, 12.0, 14.0)).width(Length::Fill).style(ui::boxed(p.panel2, 10.0, Some((p.line, 1.0))))
}

/// The right side's "its own Store app" box: what it is, what it adds, and Install it (the password once).
pub(crate) fn store_box<'a>(p: Pal, name: E<'a, Msg>, words: &'a str, installed: bool, act: Msg) -> E<'a, Msg> {
    let b: E<'a, Msg> = if installed { kit::tag(&p, "Installed · updates in the Store", kit::Tag::Ok) } else { kit::chip_acc(&p, "Install it · asks for your password once", Some(act)) };
    boxed(p, column![name, t(words, 12.0, p.muted).line_height(1.5), container(b).padding(pad(6.0, 0.0, 0.0, 0.0))].spacing(4.0)).into()
}

/// A poster or game art: the art's colours as the boards' gradients.
pub(crate) fn art<'a>(cs: &[Value], w: Length, h: f32, r: f32) -> container::Container<'a, Msg> {
    let a = colour(cs.first().unwrap_or(&Value::Null), kit::rgb(0x3a, 0x2a, 0x20));
    let b = colour(cs.get(1).unwrap_or(&Value::Null), kit::rgb(0x12, 0x0d, 0x0a));
    container(Space::new()).width(w).height(Length::Fixed(h)).style(kit::gradient(135.0, a, b, r, None))
}

/// The board's bars (`data-bars`): each value as a bar from the bottom, in the accent (and a second part over it in grey).
pub(crate) struct Bars {
    pub bars: Vec<(f32, f32)>,
    pub a: Color,
    pub b: Color,
}
impl<M> canvas::Program<M> for Bars {
    type State = ();
    fn draw(&self, _s: &(), r: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        let mut f = Frame::new(r, b.size());
        let n = self.bars.len().max(1) as f32;
        let mx = self.bars.iter().map(|x| x.0 + x.1).fold(0.0f32, f32::max).max(1.0);
        let w = b.width / n;
        for (i, (x, y)) in self.bars.iter().enumerate() {
            let left = i as f32 * w + w * 0.1;
            let bw = (w * 0.8).max(1.0);
            let h1 = x / mx * b.height;
            let h2 = y / mx * b.height;
            f.fill(&Path::rectangle(Point::new(left, b.height - h1), Size::new(bw, h1)), self.a);
            if h2 > 0.0 {
                f.fill(&Path::rectangle(Point::new(left, b.height - h1 - h2), Size::new(bw, h2)), self.b);
            }
        }
        vec![f.into_geometry()]
    }
}
pub(crate) fn bars<'a>(vals: Vec<(f32, f32)>, a: Color, b: Color, h: f32) -> E<'a, Msg> {
    canvas::Canvas::new(Bars { bars: vals, a, b }).width(Length::Fill).height(Length::Fixed(h)).into()
}

/// A line over a grid, filled under it (`data-line ... data-grid data-fill`).
pub(crate) struct Line {
    pub pts: Vec<f32>,
    pub c: Color,
    pub grid: Color,
}
impl<M> canvas::Program<M> for Line {
    type State = ();
    fn draw(&self, _s: &(), r: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        let mut f = Frame::new(r, b.size());
        for i in 1..4 {
            let y = b.height * i as f32 / 4.0;
            f.stroke(&Path::line(Point::new(0.0, y), Point::new(b.width, y)), canvas::Stroke::default().with_color(self.grid).with_width(1.0));
        }
        if self.pts.len() >= 2 {
            let lo = self.pts.iter().cloned().fold(f32::MAX, f32::min) - 2.0;
            let hi = self.pts.iter().cloned().fold(f32::MIN, f32::max) + 2.0;
            let xy = |i: usize, v: f32| Point::new(i as f32 / (self.pts.len() - 1) as f32 * b.width, b.height - (v - lo) / (hi - lo).max(1.0) * b.height);
            let line = Path::new(|pb| {
                for (i, v) in self.pts.iter().enumerate() {
                    if i == 0 { pb.move_to(xy(i, *v)) } else { pb.line_to(xy(i, *v)) }
                }
            });
            let fill = Path::new(|pb| {
                pb.move_to(Point::new(0.0, b.height));
                for (i, v) in self.pts.iter().enumerate() {
                    pb.line_to(xy(i, *v));
                }
                pb.line_to(Point::new(b.width, b.height));
                pb.close();
            });
            f.fill(&fill, Color { a: 0.18, ..self.c });
            f.stroke(&line, canvas::Stroke::default().with_color(self.c).with_width(2.0));
        }
        vec![f.into_geometry()]
    }
}
pub(crate) fn line_chart<'a>(pts: Vec<f32>, c: Color, grid: Color, h: f32) -> E<'a, Msg> {
    canvas::Canvas::new(Line { pts, c, grid }).width(Length::Fill).height(Length::Fixed(h)).into()
}

/// A labelled bar (`row` + `bar` + its number): Use by device, Played this week, the PC while streaming.
pub(crate) fn meter<'a>(p: Pal, label: String, fr: f32, c: Color, right: String, label_w: f32) -> E<'a, Msg> {
    row![
        container(t(label, 12.0, p.text)).width(Length::Fixed(label_w)),
        container(ui::bar(vec![(fr.clamp(0.0, 1.0), c)], p.line, 6.0, 3.0)).width(Length::Fill),
        container(t(right, 12.0, p.muted)).width(Length::Fixed(96.0)).align_x(iced::alignment::Horizontal::Right)
    ]
    .spacing(12.0)
    .align_y(iced::Center)
    .padding(pad(3.0, 0.0, 3.0, 0.0))
    .into()
}

/// A switch for one of the page's choices (true or false).
pub(crate) fn sw<'a>(p: Pal, part: Part, key: &'static str, now: bool) -> E<'a, Msg> {
    kit::switch(&p, now, Some(Msg::Set(part, key, if now { "false".into() } else { "true".into() })))
}

/// A hearth-server-hub command for this page.
pub(crate) fn act(part: Part, args: &[&str], ok: &str) -> Msg {
    Msg::Act(part, args.iter().map(|x| x.to_string()).collect(), ok.to_string())
}

/// A few columns of the same width.
pub(crate) fn cols<'a>(items: Vec<E<'a, Msg>>, gap: f32) -> E<'a, Msg> {
    let mut r = Row::new().spacing(gap);
    for x in items {
        r = r.push(container(x).width(Length::FillPortion(1)));
    }
    r.into()
}

/// The right side of a page: 330 px, the line on its left.
pub(crate) fn panel<'a>(p: Pal, inner: E<'a, Msg>) -> E<'a, Msg> {
    let _ = p;
    container(iced::widget::scrollable(inner).height(Length::Fill)).width(Length::Fixed(330.0)).height(Length::Fill).padding(pad(18.0, 18.0, 18.0, 18.0)).into()
}

pub(crate) fn col<'a>(v: Vec<E<'a, Msg>>, gap: f32) -> E<'a, Msg> {
    Column::with_children(v).spacing(gap).into()
}

pub(crate) fn title2<'a>(p: Pal, s: impl Into<String>) -> E<'a, Msg> {
    ts(s.into(), 18.0, p.text).into()
}
