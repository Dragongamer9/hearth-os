//! Capture, in pure Rust: Mantel's Capture (Capture.qml, CaptureEditor.qml, CaptureBtn.qml, CaptureGlyph.qml,
//! CaptureIndicator.qml, capture-draw.js) ported for the Fedora edition (KDE Plasma on KWin, Wayland; no Mantel).
//! The checklist of what's in is docs/mockups/checklists/capture-rust.md.
//!   Take: an area (a box drawn on a still picture), a window (the one in front), this screen, all screens, or a
//!   scrolling capture; after a delay or not, with the pointer or not; Record (the screen recorder). Then the markup
//!   editor (editor.rs). Recent: the pictures saved lately.
//! The work is Mantel's hearth-capture-helper's (helper.rs), so both editions keep the same recent list, the same
//! names in Pictures › Screenshots and the same choices. On Plasma the picture itself comes from KWin's
//! org.kde.KWin.ScreenShot2 (kwin.rs), which KWin gives Capture through its menu entry.
//! Your choices (mode, delay, pointer, scrolling by itself) are kept in ~/.config/nimbus/capture.json through
//! settings_file.rs, only when you change one; opening it writes nothing.
//!
//!   hearth-capture-app [area|window|screen|all|scroll]   open Capture, or straight to taking one
//!   hearth-capture-app --page take|recent | --recent     open on that page
//!   hearth-capture-app --edit FILE                       the editor with FILE
//!   hearth-capture-app --indicator                       the capture dot while a recording runs (indicator.rs)
//!   hearth-capture-app shot …                            a picture from KWin, for hearth-screens (kwin.rs)
//! CI's pictures: --sample (the editor's sample of every tool), --show share|saveas|text|qr, --tool crop,
//! --crop X,Y,W,H, --save, --pick FILE [--sel X,Y,W,H], --delay N, --scroll-pill.

mod btn;
mod draw;
mod editor;
mod export;
mod glyph;
mod helper;
mod indicator;
mod kwin;
mod pages;
mod picker;
mod picture;

use draw::R;
use editor::{EMsg, Ed};
use hearth_ui::app;
use hearth_ui::settings_file;
use hearth_ui::Pal;
use iced::keyboard::{key::Named, Event as KEvent, Key};
use iced::widget::image::Handle;
use iced::{window, Element, Size, Subscription, Task};
use serde_json::{Map, Value};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Take,
    Edit,
    Recent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Idle,
    /// The window has gone, counting down.
    Wait,
    Grab,
    /// The area picker.
    Pick,
    Scroll,
}

#[derive(Debug, Clone)]
pub struct Item {
    pub path: String,
    pub time: i64,
    pub w: i64,
    pub h: i64,
}

pub const MODES: [&str; 5] = ["area", "window", "screen", "all", "scroll"];

#[derive(Debug, Clone)]
pub enum Msg {
    /// Describe it (hearth_ui::describe; every Hearth app has it)
    Describe(hearth_ui::describe::DMsg),
    Theme,
    Opened(window::Id),
    Closed(window::Id),
    Resized(window::Id, Size),
    Drag,
    Maximize,
    Minimize,
    Close,
    Resize(window::Direction),
    Key(KEvent),
    Page(Page),
    Focus(pages::Fid),
    SetMode(&'static str),
    TakeMode(&'static str),
    SetDelay(i64),
    TogglePointer,
    ToggleScrollAuto,
    Take,
    Record,
    Recorded(Value),
    Tools(Value),
    Recent(Value),
    Thumbs(Vec<(String, Option<(Handle, u32, u32)>)>),
    Hover(Option<String>),
    RecentEdit(String),
    RecentCopy(String),
    RecentFolder(String),
    RecentForget(String),
    Forgotten,
    HideWaitDone,
    Tick,
    AfterPill,
    CancelTake,
    Grabbed(Value),
    Still(Value),
    /// The screens (name, where it is on the desktop) for one picker on each, as Mantel's; empty: only the one
    /// you're on.
    ScreensListed(Vec<(String, R)>),
    StillEach(Vec<(String, R)>, Value),
    StillsLoaded(Vec<(String, R)>, Vec<Result<picture::Pic, String>>),
    GrabTimeout(u64),
    PickPress(usize, f32, f32, f32, f32),
    PickMove(usize, f32, f32, f32, f32),
    PickRelease(usize),
    PickDone,
    PickCancel,
    ScrollGo(String),
    ScrollDone(Value),
    ScrollPoll,
    ScrollStop(bool),
    Loaded(String, Result<picture::Pic, String>),
    Ed(EMsg),
    Noop,
}

pub struct App {
    pub describe: hearth_ui::describe::Describe,
    pub p: Pal,
    pub main: Option<window::Id>,
    /// The area picker: one window on each screen (Mantel's), each with its still picture.
    pub screens: Vec<PickScreen>,
    /// The screen the box is on.
    pub sel_screen: usize,
    pub pill: Option<window::Id>,
    pub main_size: Size,
    pub page: Page,
    pub phase: Phase,
    pub prefs: Map<String, Value>,
    pub tools: Value,
    pub recent: Vec<Item>,
    pub thumbs: HashMap<String, Handle>,
    pub hover: Option<String>,
    pub focus: pages::Fid,
    pub recent_cur: Option<usize>,
    pub error: String,
    pub countdown: i64,
    pub pending: &'static str,
    pub grab_seq: u64,
    pub pick_hint: String,
    pub sel: Option<R>,
    pub sel_sizing: bool,
    pub pick_drag: Option<(f32, f32, Option<R>, bool)>,
    pub scroll_geo: Option<R>,
    pub scroll_info: Value,
    pub scroll_note: String,
    pub shot: String,
    pub ed: Ed,
    /// Only for this run (CI's --delay, the menu's "take one now").
    pub delay_once: Option<i64>,
    pub ci_scroll_pill: bool,
}

/// One screen's picker: its name (KWin's), where it is on the desktop (logical pixels), its still picture, its
/// window and that window's size.
pub struct PickScreen {
    pub name: String,
    pub geo: R,
    pub still: picture::Pic,
    pub win: Option<window::Id>,
    pub size: (f32, f32),
}

fn prefs_path() -> std::path::PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME").map(std::path::PathBuf::from).unwrap_or_else(|| hearth_ui::paths::home().join(".config"));
    base.join("nimbus").join("capture.json")
}

impl App {
    pub fn mode(&self) -> &'static str {
        let m = self.prefs.get("mode").and_then(Value::as_str).unwrap_or("area");
        MODES.iter().copied().find(|x| *x == m).unwrap_or("area")
    }
    pub fn delay(&self) -> i64 {
        let d = self.prefs.get("delay").and_then(Value::as_i64).unwrap_or(0);
        if [0, 3, 5, 10].contains(&d) {
            d
        } else {
            0
        }
    }
    pub fn pointer(&self) -> bool {
        self.prefs.get("pointer").and_then(Value::as_bool) == Some(true)
    }
    pub fn scroll_auto(&self) -> bool {
        self.prefs.get("scrollAuto").and_then(Value::as_bool) != Some(false)
    }
    pub fn has_recorder(&self) -> bool {
        !helper::s(&self.tools, "record").is_empty()
    }
    /// The size of the picker the box is on (logical pixels).
    pub fn pick_size(&self) -> (f32, f32) {
        self.screens.get(self.sel_screen).map(|s| s.size).unwrap_or((1280.0, 800.0))
    }
    pub fn picker_of(&self, id: window::Id) -> Option<usize> {
        self.screens.iter().position(|s| s.win == Some(id))
    }
    fn close_pickers(&mut self) -> Task<Msg> {
        let t: Vec<Task<Msg>> = self.screens.iter_mut().filter_map(|s| s.win.take()).map(window::close).collect();
        Task::batch(t)
    }
    /// Your choice, kept (only when it changed; merged into the file, red flag 9).
    fn set_pref(&mut self, k: &str, v: Value) {
        if self.prefs.get(k) == Some(&v) {
            return;
        }
        self.prefs.insert(k.to_string(), v.clone());
        let mut c = Map::new();
        c.insert(k.to_string(), v);
        if let Err(e) = settings_file::merge_json(&prefs_path(), &c) {
            self.error = format!("Couldn't keep that choice: {e}");
        }
    }

    fn open_main(&mut self) -> Task<Msg> {
        if self.main.is_some() {
            return Task::none();
        }
        let (id, t) = window::open(app::window("hearth-capture", (1120.0, 720.0), (760.0, 520.0)));
        self.main = Some(id);
        t.map(Msg::Opened)
    }
    fn close_main(&mut self) -> Task<Msg> {
        match self.main.take() {
            Some(id) => window::close(id),
            None => Task::none(),
        }
    }
    fn close_pill(&mut self) -> Task<Msg> {
        match self.pill.take() {
            Some(id) => window::close(id),
            None => Task::none(),
        }
    }
    fn open_pill(&mut self, size: (f32, f32), place: kwin::Place) -> Task<Msg> {
        self.open_pill_on(size, place, String::new())
    }
    fn open_pill_on(&mut self, size: (f32, f32), place: kwin::Place, screen: String) -> Task<Msg> {
        if self.pill.is_some() {
            return Task::none();
        }
        let mut s = app::window("hearth-capture-pill", size, size);
        s.resizable = false;
        // See-through around the pill, as Mantel's (its window gets a clear background: the theme below).
        s.transparent = true;
        s.level = window::Level::AlwaysOnTop;
        let (id, t) = window::open(s);
        self.pill = Some(id);
        let (w, h) = size;
        t.map(Msg::Opened).chain(Task::perform(kwin::place_async("hearth-capture-pill", w, h, place, screen), |_| Msg::Noop))
    }

    fn load_tools(&self) -> Task<Msg> {
        Task::perform(helper::call(helper::a(&["tools"]), None, 15), Msg::Tools)
    }
    fn load_recent(&self) -> Task<Msg> {
        Task::perform(helper::call(helper::a(&["history"]), None, 15), Msg::Recent)
    }
    fn run(&self, args: &[&str]) -> Task<Msg> {
        Task::perform(helper::call(helper::a(args), None, 30), |_| Msg::Noop)
    }

    /// Opens Capture's window (Mantel's open()).
    fn open(&mut self, page: Option<Page>) -> Task<Msg> {
        if self.phase != Phase::Idle {
            return Task::none();
        }
        self.page = page.unwrap_or(if !self.shot.is_empty() && self.page == Page::Edit { Page::Edit } else { Page::Take });
        self.focus = pages::TAKE_BUTTON;
        Task::batch([self.open_main(), self.load_tools(), self.load_recent(), self.run(&["clean"])])
    }

    // ------------------------------------------------ taking a picture
    fn take(&mut self, m: &str) -> Task<Msg> {
        if self.phase != Phase::Idle {
            return Task::none();
        }
        self.error.clear();
        self.pending = MODES.iter().copied().find(|x| *x == m).unwrap_or(self.mode());
        self.phase = Phase::Wait;
        self.countdown = self.delay_once.take().unwrap_or(self.delay());
        self.ed.commit_pending();
        // The window goes away first.
        let mut t = vec![self.close_main(), Task::perform(hearth_ui::run::after(450), |_| Msg::HideWaitDone)];
        if self.tools.as_object().map(|o| o.is_empty()).unwrap_or(true) {
            t.push(self.load_tools());
        }
        Task::batch(t)
    }

    fn grab_now(&mut self) -> Task<Msg> {
        self.phase = Phase::Grab;
        self.grab_seq += 1;
        let seq = self.grab_seq;
        // The helper never answered (it didn't start, or hung): Capture can be opened again rather than stay hidden.
        let guard = Task::perform(hearth_ui::run::after(45000), move |_| Msg::GrabTimeout(seq));
        let m = self.pending;
        if m == "area" {
            return Task::batch([self.pick_area(""), guard]);
        }
        if m == "scroll" {
            return Task::batch([self.pick_area("Drag around the part that scrolls, then Take"), guard]);
        }
        let mut args = vec!["grab".to_string(), m.to_string()];
        if self.pointer() {
            args.push("--pointer".into());
        }
        Task::batch([Task::perform(helper::call(args, None, 40), Msg::Grabbed), guard])
    }

    /// The area picker over a still picture of each screen, one picker on every screen at once (Mantel's). On KDE
    /// Plasma the screens come from KDE's screen settings (Mantel's hearth-screens get, kscreen-doctor) and a KWin
    /// script puts each picker full screen on its own screen (kwin::place_pickers); elsewhere, or when the screens
    /// can't be read, it's the one screen you're on.
    fn pick_area(&mut self, hint: &str) -> Task<Msg> {
        self.phase = Phase::Grab;
        self.pick_hint = hint.to_string();
        if helper::on_kwin() {
            return Task::perform(helper::screens(), Msg::ScreensListed);
        }
        self.grab_one_still()
    }
    fn grab_one_still(&mut self) -> Task<Msg> {
        let mut args = vec!["grab".to_string(), "screen".to_string()];
        if self.pointer() {
            args.push("--pointer".into());
        }
        Task::perform(helper::call(args, None, 40), Msg::Still)
    }

    fn failed(&mut self, msg: &str) -> Task<Msg> {
        self.phase = Phase::Idle;
        self.error = msg.to_string();
        self.page = Page::Take;
        let close_picker = self.close_pickers();
        self.screens.clear();
        Task::batch([close_picker, self.close_pill(), self.open_main()])
    }

    pub fn edit(&mut self, path: &str, crop: Option<R>) -> Task<Msg> {
        self.shot = path.to_string();
        self.ed.start(crop);
        self.phase = Phase::Idle;
        self.page = Page::Edit;
        let p = path.to_string();
        let close_picker = self.close_pickers();
        self.screens.clear();
        Task::batch([close_picker, self.close_pill(), self.open_main(), self.load_tools(), Task::perform(picture::load_async(p.clone()), move |r| Msg::Loaded(p.clone(), r))])
    }

    fn pick_done(&mut self) -> Task<Msg> {
        let (Some(sel), Some(scr)) = (self.sel, self.screens.get(self.sel_screen)) else { return Task::none() };
        if sel.w < 4.0 || sel.h < 4.0 {
            return Task::none();
        }
        let (sw, sh) = scr.size;
        if self.pending == "scroll" {
            // Where it is on the whole desktop (the box is on its own screen's picker).
            let g = R::new((scr.geo.x + sel.x).round(), (scr.geo.y + sel.y).round(), sel.w.round(), sel.h.round());
            let below = draw::pill_below(sel, sh);
            let name = scr.name.clone();
            return self.start_scroll(g, below, name);
        }
        let c = draw::picker_to_crop(sel, sw, sh, scr.still.w as f32, scr.still.h as f32);
        let path = scr.still.path.clone();
        self.edit(&path, Some(c))
    }
    fn pick_cancel(&mut self) -> Task<Msg> {
        self.phase = Phase::Idle;
        self.sel = None;
        self.page = Page::Take;
        let close_picker = self.close_pickers();
        self.screens.clear();
        Task::batch([close_picker, self.open_main()])
    }

    // ------------------------------------------------ scrolling capture
    fn start_scroll(&mut self, g: R, below: bool, screen: String) -> Task<Msg> {
        self.scroll_geo = Some(g);
        self.scroll_info = Value::Object(Map::new());
        self.scroll_note.clear();
        self.sel = None;
        self.phase = Phase::Scroll;
        let mut args = vec!["scroll".to_string(), "--geometry".to_string(), format!("{},{},{},{}", g.x, g.y, g.w, g.h)];
        if self.scroll_auto() {
            args.push("--auto".into());
        }
        if self.pointer() {
            args.push("--pointer".into());
        }
        let close_picker = self.close_pickers();
        self.screens.clear();
        // The pill goes outside the area when it fits: below it when the area starts near the top (draw::pill_below),
        // on the screen the area is on.
        let pill = self.open_pill_on((pages::scroll_pill_width(self), 56.0), if below { kwin::Place::Bottom(24.0) } else { kwin::Place::Top(24.0) }, screen);
        // The picker's windows go first (they'd be in the pictures), then the pictures start.
        let go = Task::perform(hearth_ui::run::after(500), move |_| Msg::ScrollGo(args.join("\u{1f}")));
        Task::batch([close_picker, pill, go])
    }

    fn scroll_file() -> std::path::PathBuf {
        helper::runtime_dir().join("scroll.json")
    }
}

fn parse_geo(s: &str) -> Option<R> {
    let v: Vec<f32> = s.split(',').filter_map(|x| x.trim().parse().ok()).collect();
    if v.len() == 4 && v[2] > 0.0 && v[3] > 0.0 {
        Some(R::new(v[0], v[1], v[2], v[3]))
    } else {
        None
    }
}

impl Ed {
    /// A text being typed is kept before the window goes (a new screenshot from the editor).
    fn commit_pending(&mut self) {
        if self.editing_text {
            let _ = editor::update(self, EMsg::TextCommit, &Value::Null, &mut Vec::new());
        }
    }
}

fn boot() -> (App, Task<Msg>) {
    let prefs = settings_file::read(&prefs_path()).map().unwrap_or_default();
    let mut a = App {
            describe: hearth_ui::describe::Describe::new("capture", "Capture"),
        p: hearth_ui::theme::load(),
        main: None,
        screens: Vec::new(),
        sel_screen: 0,
        pill: None,
        main_size: Size::new(1120.0, 720.0),
        page: Page::Take,
        phase: Phase::Idle,
        prefs,
        tools: Value::Object(Map::new()),
        recent: Vec::new(),
        thumbs: HashMap::new(),
        hover: None,
        focus: pages::TAKE_BUTTON,
        recent_cur: None,
        error: String::new(),
        countdown: 0,
        pending: "area",
        grab_seq: 0,
        pick_hint: String::new(),
        sel: None,
        sel_sizing: false,
        pick_drag: None,
        scroll_geo: None,
        scroll_info: Value::Object(Map::new()),
        scroll_note: String::new(),
        shot: String::new(),
        ed: Ed::new(),
        delay_once: app::arg("--delay").and_then(|d| d.parse().ok()),
        ci_scroll_pill: app::flag("--scroll-pill"),
    };
    // What CI and the menu's actions ask for.
    if app::flag("--sample") {
        a.ed.after_load.push(EMsg::Sample);
    }
    if let Some(t) = app::arg("--tool") {
        if let Some((tt, _, _)) = draw::TOOLS.iter().find(|x| x.1 == t || format!("{:?}", x.0).to_lowercase() == t) {
            a.ed.after_load.push(EMsg::Tool(*tt));
        }
    }
    match app::arg("--show").as_deref() {
        Some("share") => a.ed.after_load.push(EMsg::Share),
        Some("saveas") => a.ed.after_load.push(EMsg::SaveAsOpen),
        Some("text") => a.ed.after_load.push(EMsg::ReadText),
        Some("qr") => a.ed.after_load.push(EMsg::ReadQr),
        _ => {}
    }
    if app::flag("--save") {
        a.ed.after_load.push(EMsg::SaveCopy);
    }
    let crop = app::arg("--crop").and_then(|c| parse_geo(&c));
    let first = std::env::args().nth(1).unwrap_or_default();
    let task = if let Some(f) = app::arg("--edit") {
        Task::batch([a.edit(&f, crop), a.load_recent(), a.run(&["clean"])])
    } else if let Some(f) = app::arg("--pick") {
        // CI: the picker over FILE (a still picture), with a box when --sel is given.
        a.phase = Phase::Grab;
        a.pending = if app::flag("--scroll") { "scroll" } else { "area" };
        a.sel = app::arg("--sel").and_then(|s| parse_geo(&s));
        Task::batch([a.load_tools(), Task::done(Msg::Still(serde_json::json!({"ok": true, "path": f})))])
    } else if a.ci_scroll_pill {
        // CI: the pill as a scrolling capture of this area shows it (--scroll-pill-area X,Y,W,H; the pill goes below
        // an area that starts near the top, as Take does).
        a.phase = Phase::Scroll;
        let g = app::arg("--scroll-pill-area").and_then(|s| parse_geo(&s)).unwrap_or(R::new(200.0, 300.0, 700.0, 400.0));
        a.scroll_geo = Some(g);
        a.scroll_info = serde_json::json!({"frames": 6, "height": 2140, "width": 700, "auto": true});
        let below = draw::pill_below(g, app::arg("--screen-height").and_then(|v| v.parse().ok()).unwrap_or(1080.0));
        a.open_pill((pages::scroll_pill_width(&a), 56.0), if below { kwin::Place::Bottom(24.0) } else { kwin::Place::Top(24.0) })
    } else if MODES.contains(&first.as_str()) || app::arg("--take").is_some() {
        let m = app::arg("--take").unwrap_or(first);
        Task::batch([a.load_tools(), a.take(&m)])
    } else {
        let page = match app::arg("--page").as_deref() {
            Some("recent") => Some(Page::Recent),
            _ if app::flag("--recent") => Some(Page::Recent),
            _ => None,
        };
        a.open(page)
    };
    (a, task)
}

fn update(a: &mut App, m: Msg) -> Task<Msg> {
    match m {
        Msg::Describe(dm) => return a.describe.update(dm).map(Msg::Describe),
        Msg::Theme => a.p = hearth_ui::theme::load(),
        Msg::Opened(_) | Msg::Noop => {}
        Msg::Closed(id) => {
            if a.main == Some(id) {
                a.main = None;
            }
            if let Some(i) = a.picker_of(id) {
                a.screens[i].win = None;
            }
            if a.pill == Some(id) {
                a.pill = None;
            }
            // The window closed by you, with nothing being taken: Capture ends.
            if a.main.is_none() && a.screens.iter().all(|s| s.win.is_none()) && a.pill.is_none() && a.phase == Phase::Idle {
                return iced::exit();
            }
        }
        Msg::Resized(id, s) => {
            if a.main == Some(id) {
                a.main_size = s;
            }
            if let Some(i) = a.picker_of(id) {
                a.screens[i].size = (s.width, s.height);
            }
        }
        Msg::Drag => return a.main.map(window::drag).unwrap_or_else(Task::none),
        Msg::Maximize => return a.main.map(window::toggle_maximize).unwrap_or_else(Task::none),
        Msg::Minimize => return a.main.map(|id| window::minimize(id, true)).unwrap_or_else(Task::none),
        Msg::Resize(d) => return a.main.map(|id| window::drag_resize(id, d)).unwrap_or_else(Task::none),
        Msg::Close => {
            if a.phase == Phase::Idle {
                return Task::batch([a.run(&["clean"]), iced::exit()]);
            }
        }
        Msg::Key(ev) => return key(a, ev),
        Msg::Page(p) => {
            a.page = p;
            if p == Page::Recent {
                return a.load_recent();
            }
        }
        Msg::Focus(i) => a.focus = i,
        Msg::SetMode(m) => a.set_pref("mode", Value::from(m)),
        Msg::TakeMode(m) => {
            a.set_pref("mode", Value::from(m));
            return a.take(m);
        }
        Msg::SetDelay(d) => a.set_pref("delay", Value::from(d)),
        Msg::TogglePointer => {
            let v = !a.pointer();
            a.set_pref("pointer", Value::from(v));
        }
        Msg::ToggleScrollAuto => {
            let v = !a.scroll_auto();
            a.set_pref("scrollAuto", Value::from(v));
        }
        Msg::Take => {
            let m = a.mode();
            return a.take(m);
        }
        Msg::Record => {
            if !a.has_recorder() {
                a.error = "No screen recorder is installed (Hearth Recorder).".into();
                return Task::none();
            }
            // The capture dot comes up while it records (it goes by itself when the recording ends).
            if let Ok(me) = std::env::current_exe() {
                helper::detached(&me.to_string_lossy(), &["--indicator".to_string()]);
            }
            return Task::batch([a.close_main(), Task::perform(helper::call(helper::a(&["record"]), None, 30), Msg::Recorded)]);
        }
        Msg::Recorded(v) => {
            if !helper::b(&v, "ok") {
                a.error = helper::s(&v, "error").to_string();
                return a.open(Some(Page::Take));
            }
            if a.main.is_none() && a.phase == Phase::Idle {
                return iced::exit();
            }
        }
        Msg::Tools(v) => {
            if v.is_object() {
                a.tools = v;
            }
        }
        Msg::Recent(v) => {
            a.recent = v
                .get("items")
                .and_then(Value::as_array)
                .map(|l| {
                    l.iter()
                        .map(|i| Item { path: helper::s(i, "path").to_string(), time: helper::n(i, "time"), w: helper::n(i, "width"), h: helper::n(i, "height") })
                        .filter(|i| !i.path.is_empty())
                        .collect()
                })
                .unwrap_or_default();
            let want: Vec<String> = a.recent.iter().map(|i| i.path.clone()).filter(|p| !a.thumbs.contains_key(p)).collect();
            a.thumbs.retain(|k, _| a.recent.iter().any(|i| &i.path == k));
            if let Some(c) = a.recent_cur {
                if c >= a.recent.len() {
                    a.recent_cur = if a.recent.is_empty() { None } else { Some(a.recent.len() - 1) };
                }
            }
            if !want.is_empty() {
                return Task::perform(picture::thumbs_async(want), Msg::Thumbs);
            }
        }
        Msg::Thumbs(list) => {
            for (p, t) in list {
                if let Some((h, _, _)) = t {
                    a.thumbs.insert(p, h);
                }
            }
        }
        Msg::Hover(h) => a.hover = h,
        Msg::RecentEdit(p) => return a.edit(&p, None),
        Msg::RecentCopy(p) => return a.run(&["copy", &p]),
        Msg::RecentFolder(p) => return a.run(&["folder", &p]),
        Msg::RecentForget(p) => return Task::perform(helper::call(helper::a(&["forget", &p]), None, 15), |_| Msg::Forgotten),
        Msg::Forgotten => return a.load_recent(),
        Msg::HideWaitDone => {
            if a.phase != Phase::Wait {
                return Task::none();
            }
            if a.countdown > 0 {
                return Task::batch([a.open_pill((260.0, 64.0), kwin::Place::Top(40.0)), Task::perform(hearth_ui::run::after(1000), |_| Msg::Tick)]);
            }
            return a.grab_now();
        }
        Msg::Tick => {
            if a.phase != Phase::Wait {
                return Task::none();
            }
            a.countdown -= 1;
            if a.countdown > 0 {
                return Task::perform(hearth_ui::run::after(1000), |_| Msg::Tick);
            }
            // The countdown's pill is gone before the picture is taken.
            return Task::batch([a.close_pill(), Task::perform(hearth_ui::run::after(300), |_| Msg::AfterPill)]);
        }
        Msg::AfterPill => {
            if a.phase == Phase::Wait {
                return a.grab_now();
            }
        }
        Msg::CancelTake => {
            if a.phase == Phase::Wait {
                a.countdown = 0;
                a.phase = Phase::Idle;
                return Task::batch([a.close_pill(), a.open(Some(Page::Take))]);
            }
        }
        Msg::GrabTimeout(seq) => {
            if a.phase == Phase::Grab && seq == a.grab_seq {
                return a.failed("Taking the picture didn't finish. Try again.");
            }
        }
        Msg::Grabbed(v) => {
            if a.phase != Phase::Grab {
                return Task::none();
            }
            if helper::b(&v, "ok") {
                let p = helper::s(&v, "path").to_string();
                return a.edit(&p, None);
            }
            if helper::b(&v, "pick") {
                return a.pick_area("Draw around the window you want (this desktop can't find it by itself)");
            }
            let e = helper::s(&v, "error");
            return a.failed(if e.is_empty() { "Couldn't take the picture" } else { e });
        }
        Msg::ScreensListed(list) => {
            if a.phase != Phase::Grab {
                return Task::none();
            }
            // Two or more screens: a still picture of each (Mantel's grab-each); one, or none read: as before.
            if list.len() < 2 {
                return a.grab_one_still();
            }
            let mut args = vec!["grab-each".to_string()];
            if a.pointer() {
                args.push("--pointer".into());
            }
            args.extend(list.iter().map(|(n, _)| n.clone()));
            return Task::perform(helper::call(args, None, 60), move |v| Msg::StillEach(list.clone(), v));
        }
        Msg::StillEach(list, v) => {
            if a.phase != Phase::Grab {
                return Task::none();
            }
            if !helper::b(&v, "ok") {
                let e = helper::s(&v, "error");
                return a.failed(if e.is_empty() { "Couldn't take the picture" } else { e });
            }
            let paths: Vec<String> = list.iter().map(|(n, _)| v.get("shots").and_then(|s| s.get(n)).map(|s| helper::s(s, "path").to_string()).unwrap_or_default()).collect();
            return Task::perform(picture::load_many_async(paths), move |r| Msg::StillsLoaded(list.clone(), r));
        }
        Msg::Still(v) => {
            if !helper::b(&v, "ok") {
                let e = helper::s(&v, "error");
                return a.failed(if e.is_empty() { "Couldn't take the picture" } else { e });
            }
            let p = helper::s(&v, "path").to_string();
            return Task::perform(picture::load_many_async(vec![p]), |r| Msg::StillsLoaded(Vec::new(), r));
        }
        Msg::StillsLoaded(list, results) => {
            let mut screens = Vec::new();
            for (i, r) in results.into_iter().enumerate() {
                match r {
                    Err(e) => return a.failed(&format!("Couldn't take the picture: {e}")),
                    Ok(pic) => {
                        let (name, geo) = list.get(i).cloned().unwrap_or_else(|| (String::new(), R::new(0.0, 0.0, 1280.0, 800.0)));
                        screens.push(PickScreen { name, geo, still: pic, win: None, size: (geo.w, geo.h) });
                    }
                }
            }
            if screens.is_empty() {
                return a.failed("Couldn't take the picture");
            }
            a.sel_screen = 0;
            a.sel_sizing = false;
            a.pick_drag = None;
            a.phase = Phase::Pick;
            let many = screens.len() > 1;
            let mut tasks = vec![a.close_main()];
            let mut places = Vec::new();
            for (i, scr) in screens.iter_mut().enumerate() {
                // One picker: Capture's own window name (as before); one on each screen: a name each, so KWin's
                // script can put each on its screen.
                let app_id = if many { format!("hearth-capture-pick-{i}") } else { "hearth-capture".to_string() };
                let mut s = app::window(&app_id, (scr.geo.w.max(320.0), scr.geo.h.max(240.0)), (320.0, 240.0));
                s.fullscreen = true;
                s.resizable = false;
                let (id, t) = window::open(s);
                scr.win = Some(id);
                tasks.push(t.map(Msg::Opened).chain(window::size(id).map(move |sz| Msg::Resized(id, sz))));
                places.push((app_id, scr.name.clone()));
            }
            a.screens = screens;
            if many {
                tasks.push(Task::perform(kwin::place_pickers_async(places), |_| Msg::Noop));
            }
            return Task::batch(tasks);
        }
        Msg::PickPress(i, x, y, w, h) => {
            if i >= a.screens.len() {
                return Task::none();
            }
            // A box on another screen starts there (the one before goes).
            if i != a.sel_screen {
                a.sel_screen = i;
                a.sel = None;
                a.sel_sizing = false;
            }
            a.screens[i].size = (w, h);
            let moving = a.sel.map(|c| c.contains(x, y)).unwrap_or(false);
            a.pick_drag = Some((x, y, a.sel, moving));
            if !moving {
                a.sel = Some(R::new(x, y, 0.0, 0.0));
            }
        }
        Msg::PickMove(i, x, y, w, h) => {
            if i != a.sel_screen || i >= a.screens.len() {
                return Task::none();
            }
            a.screens[i].size = (w, h);
            if let Some((sx, sy, start, moving)) = a.pick_drag {
                if moving {
                    if let Some(s) = start {
                        a.sel = Some(draw::move_sel(s, x - sx, y - sy, w, h));
                    }
                } else {
                    a.sel = Some(draw::norm_rect(sx, sy, x.clamp(0.0, w), y.clamp(0.0, h)));
                }
            }
        }
        Msg::PickRelease(i) => {
            if i != a.sel_screen {
                return Task::none();
            }
            if let Some((_, _, _, moving)) = a.pick_drag.take() {
                if !moving && a.sel.map(|s| s.w < 4.0 || s.h < 4.0).unwrap_or(false) {
                    a.sel = None;
                }
            }
        }
        Msg::PickDone => return a.pick_done(),
        Msg::PickCancel => return a.pick_cancel(),
        Msg::ScrollGo(joined) => {
            if a.phase != Phase::Scroll {
                return Task::none();
            }
            let args: Vec<String> = joined.split('\u{1f}').map(String::from).collect();
            return Task::batch([
                Task::perform(helper::call(args, None, 600), Msg::ScrollDone),
                Task::perform(hearth_ui::run::after(700), |_| Msg::ScrollPoll),
            ]);
        }
        Msg::ScrollPoll => {
            if a.phase != Phase::Scroll || a.ci_scroll_pill {
                return Task::none();
            }
            if let Ok(t) = std::fs::read_to_string(App::scroll_file()) {
                if let Ok(v) = serde_json::from_str::<Value>(&t) {
                    a.scroll_info = v;
                }
            }
            return Task::perform(hearth_ui::run::after(700), |_| Msg::ScrollPoll);
        }
        Msg::ScrollStop(cancel) => {
            if a.ci_scroll_pill {
                a.phase = Phase::Idle;
                return Task::batch([a.close_pill(), iced::exit()]);
            }
            let mut args = vec!["scroll-stop"];
            if cancel {
                args.push("--cancel");
            }
            return a.run(&args);
        }
        Msg::ScrollDone(v) => {
            if helper::b(&v, "cancelled") {
                a.phase = Phase::Idle;
                return Task::batch([a.close_pill(), a.open(Some(Page::Take))]);
            }
            if !helper::b(&v, "ok") {
                let e = helper::s(&v, "error");
                return a.failed(if e.is_empty() { "The scrolling capture didn't work" } else { e });
            }
            let frames = helper::n(&v, "frames");
            let head = if frames > 1 {
                format!("{frames} pictures joined, {} × {}. ", helper::n(&v, "width"), helper::n(&v, "height"))
            } else {
                "It didn't scroll, so this is one picture. ".to_string()
            };
            a.scroll_note = format!("{head}{}", helper::s(&v, "why"));
            let p = helper::s(&v, "path").to_string();
            return a.edit(&p, None);
        }
        Msg::Loaded(path, r) => {
            if path != a.shot {
                return Task::none();
            }
            match r {
                Ok(pic) => {
                    a.ed.pic = Some(pic);
                    a.ed.sync_view();
                    let after = std::mem::take(&mut a.ed.after_load);
                    let mut t = Vec::new();
                    for m in after {
                        t.push(Task::done(Msg::Ed(m)));
                    }
                    return Task::batch(t);
                }
                Err(e) => a.ed.load_error = e,
            }
        }
        Msg::Ed(m) => {
            let mut fx = Vec::new();
            let t = editor::update(&mut a.ed, m, &a.tools, &mut fx);
            let mut tasks = vec![t];
            for f in fx {
                match f {
                    editor::Fx::Saved(_) => tasks.push(a.load_recent()),
                    editor::Fx::NewShot => {
                        let m = a.mode();
                        tasks.push(a.take(m));
                    }
                }
            }
            return Task::batch(tasks);
        }
    }
    Task::none()
}

fn key(a: &mut App, ev: KEvent) -> Task<Msg> {
    let KEvent::KeyPressed { key, modifiers, .. } = ev else { return Task::none() };
    if a.phase == Phase::Pick {
        return picker::key(a, &key, modifiers);
    }
    if a.phase == Phase::Wait {
        if let Key::Named(Named::Escape) = key.as_ref() {
            return update(a, Msg::CancelTake);
        }
        return Task::none();
    }
    if a.main.is_none() {
        return Task::none();
    }
    // Ctrl+Page Up / Down: the previous / next page (Take, Edit when there's a shot, Recent), as in Settings and the
    // other apps with tabs; the controller's LB / RB send these (hearth-pad --plasma, PLASMA_TAB_APPS).
    if modifiers.control() && matches!(key.as_ref(), Key::Named(Named::PageUp) | Key::Named(Named::PageDown)) {
        let pages: Vec<Page> = if a.shot.is_empty() { vec![Page::Take, Page::Recent] } else { vec![Page::Take, Page::Edit, Page::Recent] };
        let i = pages.iter().position(|p| *p == a.page).unwrap_or(0);
        let j = if matches!(key.as_ref(), Key::Named(Named::PageDown)) { (i + 1) % pages.len() } else { (i + pages.len() - 1) % pages.len() };
        return update(a, Msg::Page(pages[j]));
    }
    match a.page {
        Page::Edit => {
            let ctrl = modifiers.control();
            let shift = modifiers.shift();
            let ch = match key.as_ref() {
                Key::Character(c) => c.to_lowercase(),
                _ => String::new(),
            };
            let m = if ctrl && ch == "z" && !shift {
                Some(EMsg::Undo)
            } else if ctrl && (ch == "y" || (ch == "z" && shift)) {
                Some(EMsg::Redo)
            } else if ctrl && ch == "c" && a.ed.panel != editor::Panel::Text {
                Some(EMsg::Copy)
            } else if ctrl && ch == "s" && shift {
                Some(EMsg::SaveAsOpen)
            } else if ctrl && ch == "s" {
                Some(EMsg::SaveCopy)
            } else if let Key::Named(Named::Escape) = key.as_ref() {
                Some(EMsg::Esc)
            } else {
                None
            };
            match m {
                Some(m) => update(a, Msg::Ed(m)),
                None => Task::none(),
            }
        }
        Page::Take => pages::take_key(a, &key),
        Page::Recent => pages::recent_key(a, &key),
    }
}

fn view(a: &App, id: window::Id) -> Element<'_, Msg> {
    if let Some(i) = a.picker_of(id) {
        return picker::view(a, i);
    }
    if Some(id) == a.pill {
        return pages::pill_view(a);
    }
    hearth_ui::describe::with_bar(pages::main_view(a), &a.p, &a.describe, Msg::Describe)
}

fn subscription(_a: &App) -> Subscription<Msg> {
    Subscription::batch([
        Subscription::run(hearth_ui::theme::watch).map(|_| Msg::Theme),
        iced::keyboard::listen().map(Msg::Key),
        window::close_events().map(Msg::Closed),
        window::resize_events().map(|(id, s)| Msg::Resized(id, s)),
    ])
}

fn main() -> iced::Result {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("shot") {
        std::process::exit(kwin::shot_main(&args[1..]));
    }
    if app::handle_cli(
        "hearth-capture-app",
        "Capture: screenshots and screen recording.\n  [area|window|screen|all|scroll]  take one now\n  --page take|recent, --recent, --edit FILE, --indicator\n  shot …  (for hearth-screens: a picture from KWin)",
    ) {
        return Ok(());
    }
    if app::flag("--indicator") {
        return indicator::run();
    }
    // How it draws (the graphics card where it starts), before iced starts: its windows open later (boot).
    app::renderer("hearth-capture");
    // The pills (the countdown, the scrolling capture's) are see-through around their round shape: drawing with the
    // processor, Capture's buffers carry alpha (hearth-ui's softbuffer, as hearth-pad's overlays); every other window
    // of Capture paints its own solid background, so it stays opaque.
    std::env::set_var("HEARTH_SOFTBUFFER_ALPHA", "1");
    iced::daemon(boot, update, view)
        .title(|_a: &App, _id: window::Id| "Capture".to_string())
        .subscription(subscription)
        .theme(|a: &App, id: window::Id| {
            if Some(id) == a.pill {
                // Only the pill's window: a clear background, so nothing shows around the pill.
                iced::Theme::custom(
                    "Hearth pill",
                    iced::theme::Palette { background: iced::Color::TRANSPARENT, text: a.p.text, primary: a.p.accent, success: a.p.good, warning: a.p.warn, danger: a.p.danger },
                )
            } else {
                app::iced_theme(&a.p)
            }
        })
        .default_font(app::INTER)
        .run()
}
