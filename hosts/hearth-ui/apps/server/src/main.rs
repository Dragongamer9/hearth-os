//! Server mode's dashboard on the Fedora edition, in pure Rust (docs/SERVER-MODE.md, "On the Fedora edition"): Mantel's
//! server-mode.qml and ServerDash.qml ported. The Fedora edition has no labwc and no Quickshell, so nimbus-session's
//! run_server starts this app full screen in cage (a kiosk compositor: one app, nothing else on the screen) through
//! desktop/server/cage/run, which watches it as labwc's autostart watches the Quickshell dashboard.
//!
//! The same backend as Mantel's, so both editions behave alike: everything comes from `hearth-server-mode dash` (every
//! 4 s, as ServerDash.qml) and every action goes through hearth-server-mode (go, service, power, idle-power, cancel,
//! ai-unload, ithil-*), Settings opens with hearth-settings, the screen goes off with hearth-screen-power and the
//! controller comes through hearth-pad (keys mode; Guide is Play, ☰ the menu). Nothing here writes a setting.
//!
//! What it leaves for the session: $XDG_RUNTIME_DIR/hearth-server-dash-up when its window is open (nimbus-session ends
//! Server mode and opens Game Mode, or Desktop Mode while Game Mode is off, when that mark doesn't come), and
//! hearth-server-dash-alive every 10 s while it's still drawing (cage/run restarts a dashboard that stops).
//!
//! For CI's pictures: --show detail:N | switch | remote | back | menu | dim, and --sel N.

mod brain;
mod draw;
mod pages;
mod view;

pub const SITE_ID: &str = "server-site";

use hearth_ui::app;
use hearth_ui::run;
use iced::keyboard::{key::Named, Event as KEvent, Key};
use iced::{Size, Subscription, Task};
use serde_json::Value;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Overlay {
    None,
    Switch,
    Detail,
    Menu,
    Back,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Screen {
    On,
    Dim,
    Off,
}

#[derive(Debug, Clone)]
pub enum Msg {
    /// Describe it and its blocks (hearth_ui::describe, hearth_ui::blocks; every Hearth app has them)
    Describe(hearth_ui::describe::DMsg),
    Dash(run::Output),
    Acted(run::Output, String),
    BackRead(run::Output),
    Tick,
    Key(KEvent),
    Opened(Size),
    Resized(Size),
    Wake,
    Pad(String),
    Pick(usize),
    Play,
    MenuItem(usize),
    SwitchNow,
    CancelSwitch,
    RemoteNow,
    RemoteCancel,
    Close,
    DetailAct(String),
    // 1.16: the animation clock, the window's focus, the feature pages and Ithil's brain
    Anim,
    Focus(bool),
    Back,
    FeatRow(usize),
    FeatKey(String),
    PickChoice(usize),
    Site(String),
    IthilWin(String),
    BrainMove(iced::Point),
    BrainLeave,
    BrainRead(run::Output),
    SmoothAgain,
}

pub struct App {
    pub describe: hearth_ui::describe::Describe,
    /// Hearth's look, for Describe it's box and Arrange (the dashboard keeps its own colours)
    pub pal: hearth_ui::Pal,
    pub helper: String,
    pub rundir: String,
    pub d: Option<Value>,
    pub sel: usize,
    pub overlay: Overlay,
    pub menu_sel: usize,
    pub count: i64,
    pub remote: Option<Value>,
    pub back_info: Option<Value>,
    pub toast: String,
    toast_at: f64,
    last_input: f64,
    last_busy: f64,
    idle_power_on: bool,
    pub screen: Screen,
    pub size: Size,
    pub now: f64,
    reading: bool,
    last_read: f64,
    last_alive: f64,
    back_at: f64,
    pub hist: [Vec<f32>; 4],
    last_sample: f64,
    pub pause_armed: String,
    pause_at: f64,
    pub sleep_armed: bool,
    sleep_at: f64,
    pub qr: Option<draw::Qr>,
    qr_path: String,
    pub plasma: bool,
    show: Option<String>,
    up_written: bool,
    // the animation clock (seconds since start), whether the window has the keyboard (Settings or Ithil's app
    // above it in cage: not), Reduce motion (desktop.json, read only)
    pub anim_t: f32,
    start: f64,
    pub focused: bool,
    seen_focus: bool,
    pub reduce: bool,
    last_anim: f64,
    // Ithil: her log for the brain, the window shown (live | 24 | all), where in it, X-ray, how her brain is turned
    pub ithil_prep: brain::Prepared,
    brain_reading: bool,
    last_brain: f64,
    pub ithil_win: String,
    pub tpos: f32,
    pub xray: bool,
    moved: f64,
    pub yaw: f32,
    pub pitch: f32,
    mouse_last: Option<iced::Point>,
    // the cache's, the ad blocker's and the rules' pages: the picked row, X once (empty), the site box, the new-rule
    // picker (step 0 off, 1 When, 2 Do; the picked of each), its numbers (drive °C, power W, from, to) and the device
    pub fsel: usize,
    pub ask_empty: bool,
    pub typing: bool,
    pub site: String,
    pub step: u8,
    pub wi: usize,
    pub di: usize,
    pub when_vals: (i64, i64, i64, i64),
    pub fan_pct: i64,
    pub device: String,
    // the never-lag guard: 0 smooth, 1 at 30 a second, 2 the light mode (this session only); its note; the window
    pub smooth_level: u8,
    pub guard_note: bool,
    cpu_win: std::collections::VecDeque<(f64, u64)>,
}

/// More than this much of one core over ~10 s while the smooth motion runs, and it's turned down.
const GUARD_PCT: f64 = 3.0;
/// The light mode's own limit: over it too, and the AIs stand still for the rest of the session.
const GUARD_LIGHT_PCT: f64 = 8.0;

/// This process's own CPU time (user + system) in clock ticks, from /proc/self/stat.
fn own_cpu_ticks() -> Option<u64> {
    let st = std::fs::read_to_string("/proc/self/stat").ok()?;
    let rest = &st[st.rfind(')')? + 2..];
    let f: Vec<&str> = rest.split_whitespace().collect();
    // after the name: state is field 3, utime 14 and stime 15 (1-based), so 11 and 12 here
    Some(f.get(11)?.parse::<u64>().ok()? + f.get(12)?.parse::<u64>().ok()?)
}

pub const COLS: usize = 5;

pub fn s(v: &Value, k: &str) -> String {
    match v.get(k) {
        Some(Value::String(x)) => x.clone(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Bool(b)) => b.to_string(),
        _ => String::new(),
    }
}
pub fn f(v: &Value, k: &str) -> Option<f64> {
    v.get(k).and_then(Value::as_f64)
}
pub fn arr(v: &Value, k: &str) -> Vec<Value> {
    v.get(k).and_then(Value::as_array).cloned().unwrap_or_default()
}
pub fn truthy(v: &Value, k: &str) -> bool {
    match v.get(k) {
        Some(Value::Bool(b)) => *b,
        Some(Value::Null) | None => false,
        Some(Value::String(x)) => !x.is_empty(),
        Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0) != 0.0,
        Some(_) => true,
    }
}

fn now() -> f64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0)
}

/// Settings › Accessibility's Reduce motion (desktop.json reduceMotion): the cores and the brain stand still.
fn reduce_motion() -> bool {
    let data = std::env::var("XDG_DATA_HOME").ok().filter(|x| !x.is_empty()).unwrap_or_else(|| format!("{}/.local/share", hearth_ui::paths::home().display()));
    std::fs::read_to_string(format!("{data}/nimbus/desktop.json")).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok()).map(|v| v.get("reduceMotion") == Some(&Value::Bool(true))).unwrap_or(false)
}

/// Game Mode on this PC (desktop.json's gameModeOff, written only by Settings): read, never written.
fn game_modes_off() -> bool {
    let p = std::env::var("HEARTH_DESKTOP_JSON").ok().filter(|x| !x.is_empty()).unwrap_or_else(|| {
        let data = std::env::var("XDG_DATA_HOME").ok().filter(|x| !x.is_empty()).unwrap_or_else(|| format!("{}/.local/share", hearth_ui::paths::home().display()));
        format!("{data}/nimbus/desktop.json")
    });
    std::fs::read_to_string(p).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok()).map(|v| v.get("gameModeOff") == Some(&Value::Bool(true))).unwrap_or(false)
}

impl App {
    fn new() -> (App, Task<Msg>) {
        let helper = std::env::var("HEARTH_SERVER_HELPER").ok().filter(|x| !x.is_empty()).unwrap_or_else(|| run::shell_helper("hearth-server-mode"));
        let rundir = std::env::var("XDG_RUNTIME_DIR").ok().filter(|x| !x.is_empty()).unwrap_or_else(|| "/tmp".into());
        let plasma = std::fs::read_to_string(std::env::var("HEARTH_DESKTOP_MODE_FILE").unwrap_or_else(|_| "/usr/share/nimbus/desktop-mode".into()))
            .map(|t| t.trim() == "plasma")
            .unwrap_or(false);
        let t = now();
        let mut a = App {
            describe: hearth_ui::describe::Describe::new("server", "Server mode").with_pages(view::BLOCKS),
            pal: hearth_ui::theme::load(),
            helper,
            rundir,
            d: None,
            sel: 0,
            overlay: Overlay::None,
            menu_sel: 0,
            count: 5,
            remote: None,
            back_info: None,
            toast: String::new(),
            toast_at: 0.0,
            last_input: t,
            last_busy: t,
            idle_power_on: false,
            screen: Screen::On,
            size: Size::new(1920.0, 1080.0),
            now: t,
            reading: false,
            last_read: 0.0,
            last_alive: 0.0,
            back_at: 0.0,
            hist: [vec![], vec![], vec![], vec![]],
            last_sample: 0.0,
            pause_armed: String::new(),
            pause_at: 0.0,
            sleep_armed: false,
            sleep_at: 0.0,
            qr: None,
            qr_path: String::new(),
            plasma,
            show: app::arg("--show"),
            up_written: false,
            anim_t: 0.0,
            start: t,
            focused: true,
            seen_focus: false,
            reduce: reduce_motion(),
            last_anim: t,
            ithil_prep: brain::Prepared::default(),
            brain_reading: false,
            last_brain: 0.0,
            ithil_win: "live".into(),
            tpos: 120.0,
            xray: false,
            moved: 0.0,
            yaw: -1.1,
            pitch: 0.22,
            mouse_last: None,
            fsel: 0,
            ask_empty: false,
            typing: false,
            site: String::new(),
            step: 0,
            wi: 0,
            di: 0,
            when_vals: (50, 300, 23 * 60, 7 * 60),
            fan_pct: 40,
            device: String::new(),
            smooth_level: std::env::var("HEARTH_SERVER_SMOOTH_LEVEL").ok().and_then(|v| v.parse().ok()).unwrap_or(0),
            guard_note: false,
            cpu_win: std::collections::VecDeque::new(),
        };
        if let Some(n) = app::arg("--sel").and_then(|x| x.parse().ok()) {
            a.sel = n;
        }
        a.read_remote();
        // 1.13: Ithil keeps living in Server mode: her app starts with the dashboard when it's on this PC (never waited on).
        run::detached(&["timeout", "30", "python3", &a.helper, "ithil-start"]);
        let t1 = a.refresh();
        let t2 = Task::perform(run::spawn(a.cmd(&["back"])), Msg::BackRead);
        (a, Task::batch([t1, t2]))
    }

    /// The helper, with a time limit (it never hangs the dashboard).
    fn cmd(&self, args: &[&str]) -> Vec<String> {
        let mut c = run::args(&["timeout", "60", "python3", &self.helper]);
        c.extend(args.iter().map(|x| x.to_string()));
        c
    }
    fn refresh(&mut self) -> Task<Msg> {
        if self.reading {
            return Task::none();
        }
        self.reading = true;
        self.last_read = now();
        let mut c = self.cmd(&["dash"]);
        c[1] = "25".into();
        Task::perform(run::spawn(c), Msg::Dash)
    }
    fn act(&mut self, args: &[&str], ok: &str) -> Task<Msg> {
        let ok = ok.to_string();
        Task::perform(run::spawn(self.cmd(args)), move |o| Msg::Acted(o, ok.clone()))
    }
    pub fn say(&mut self, t: impl Into<String>) {
        self.toast = t.into();
        self.toast_at = now();
    }
    pub fn tiles(&self) -> Vec<Value> {
        self.d.as_ref().map(|d| arr(d, "tiles")).unwrap_or_default()
    }
    pub fn cur(&self) -> Option<Value> {
        if self.sel == 0 {
            return None;
        }
        self.tiles().get(self.sel - 1).cloned()
    }
    fn cur_id(&self) -> String {
        self.cur().map(|t| s(&t, "id")).unwrap_or_default()
    }

    fn read_remote(&mut self) {
        let p = format!("{}/hearth-server/countdown.json", self.rundir);
        let v = std::fs::read_to_string(p).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok());
        let was = self.remote.is_some();
        self.remote = v.filter(|c| c.get("action").map(|x| !x.is_null()).unwrap_or(false));
        if self.remote.is_some() && !was {
            self.wake();
        }
    }
    pub fn remote_left(&self) -> i64 {
        self.remote.as_ref().map(|r| (f(r, "deadline").unwrap_or(0.0) - self.now).ceil().max(0.0) as i64).unwrap_or(0)
    }

    fn screen_power(on: bool) {
        let h = run::shell_helper("hearth-screen-power");
        run::detached(&["timeout", "15", "sh", &h, if on { "on" } else { "off" }]);
    }
    fn wake(&mut self) {
        self.last_input = now();
        if self.screen != Screen::On {
            if self.screen == Screen::Off {
                App::screen_power(true);
            }
            self.screen = Screen::On;
        }
    }

    /// Dim, black, lower power limits (ServerDash.qml idleTick).
    fn idle_tick(&mut self) -> Task<Msg> {
        let Some(d) = self.d.clone() else { return Task::none() };
        let Some(idle) = d.get("idle").cloned() else { return Task::none() };
        if self.remote.is_some() || self.overlay == Overlay::Switch {
            return Task::none();
        }
        let idle_min = (now() - self.last_input) / 60.0;
        let off = f(&idle, "off").unwrap_or(0.0);
        let dim = f(&idle, "dim").unwrap_or(0.0);
        if off > 0.0 && idle_min >= off && self.screen != Screen::Off {
            self.screen = Screen::Off;
            App::screen_power(false);
        } else if dim > 0.0 && idle_min >= dim && self.screen == Screen::On {
            self.screen = Screen::Dim;
        }
        let quiet_min = (now() - self.last_busy) / 60.0;
        let busy = truthy(&d, "busy");
        if truthy(&idle, "idlePower") && !self.idle_power_on && quiet_min >= f(&idle, "idleMinutes").unwrap_or(10.0) && !busy {
            self.idle_power_on = true;
            return self.act(&["idle-power", "on"], "");
        } else if self.idle_power_on && busy {
            self.idle_power_on = false;
            return self.act(&["idle-power", "off"], "");
        }
        Task::none()
    }

    fn sample(&mut self) {
        let Some(d) = &self.d else { return };
        let Some(l) = d.get("live") else { return };
        if now() - self.last_sample < 30.0 {
            return;
        }
        self.last_sample = now();
        let vals = [f(l, "vram").unwrap_or(0.0), f(l, "storage").unwrap_or(0.0), f(l, "power").unwrap_or(0.0), f(l, "rx").unwrap_or(0.0) + f(l, "tx").unwrap_or(0.0)];
        for (h, v) in self.hist.iter_mut().zip(vals) {
            h.push(v as f32);
            if h.len() > 120 {
                h.remove(0);
            }
        }
    }

    // ---------------------------------------------------------------- switching
    pub fn play(&mut self) {
        if self.overlay == Overlay::Switch {
            return;
        }
        if game_modes_off() {
            self.say("Game Mode is off on this PC · turn it on in Settings › System › Startup");
            return;
        }
        self.overlay = Overlay::Switch;
        self.count = 5;
    }
    fn switch_now(&mut self) -> Task<Msg> {
        self.overlay = Overlay::None;
        self.say("Switching to Game Mode…");
        self.act(&["go", "game"], "")
    }
    fn go_mode(&mut self, m: &str) -> Task<Msg> {
        self.overlay = Overlay::None;
        self.say(if m == "desktop" { "Opening Desktop Mode…" } else { "Switching…" });
        self.act(&["go", m], "")
    }
    fn remote_now(&mut self) -> Task<Msg> {
        let Some(r) = self.remote.clone() else { return Task::none() };
        let a = r.get("action").cloned().unwrap_or(Value::Null);
        let t1 = self.act(&["cancel"], "");
        let t2 = if s(&a, "kind") == "mode" { self.act(&["go", &s(&a, "to")], "") } else { self.act(&["power", &s(&a, "what")], "") };
        Task::batch([t1, t2])
    }
    fn remote_cancel(&mut self) -> Task<Msg> {
        self.remote = None;
        self.act(&["cancel"], "Cancelled")
    }
    fn settings(&mut self, page: &str) {
        self.overlay = Overlay::None;
        // Hearth Settings (hearth-settings-app on this edition) opens above the dashboard in cage; closing it shows
        // the dashboard again.
        if page.is_empty() {
            run::detached(&["hearth-settings"]);
        } else {
            run::detached(&["hearth-settings", page]);
        }
        self.say("Opening Settings…");
    }

    pub fn menu_items(&self) -> Vec<(&'static str, &'static str, String)> {
        let off = game_modes_off();
        vec![
            ("pad", "Game Mode", if off { "Off on this PC · Settings › System › Startup".into() } else { "Play: Steam on the big screen (5-second countdown)".into() }),
            ("desktops", "Desktop Mode", if self.plasma { "The KDE Plasma desktop with windows, files and apps".into() } else { "The Mantel desktop with windows, files and apps".into() }),
            ("gear", "Server mode settings", "Power when idle, what pauses, the web page".into()),
            ("gear", "Hearth Settings", "Everything else".into()),
            ("moon", "Screen off", "Any button turns it on again".into()),
            ("sleep", "Sleep", "Your devices can't reach the PC while it sleeps".into()),
            ("restart", "Restart the PC", "Server mode comes back if it's your Start in mode".into()),
            ("power", "Turn off", "Everything stops until you turn it on".into()),
        ]
    }
    fn menu_do(&mut self, i: usize) -> Task<Msg> {
        self.overlay = Overlay::None;
        match i {
            0 => {
                self.play();
                Task::none()
            }
            1 => self.go_mode("desktop"),
            2 => {
                self.settings("hs-server");
                Task::none()
            }
            3 => {
                self.settings("");
                Task::none()
            }
            4 => {
                self.screen = Screen::Off;
                App::screen_power(false);
                Task::none()
            }
            5 => self.act(&["power", "sleep"], ""),
            6 => self.act(&["power", "restart"], ""),
            7 => self.act(&["power", "off"], ""),
            _ => Task::none(),
        }
    }

    // ---------------------------------------------------------------- services
    fn ithil_act(&mut self, k: &str) -> Task<Msg> {
        let name = self.d.as_ref().and_then(|d| d.get("ithil")).map(|i| s(i, "name")).filter(|x| !x.is_empty()).unwrap_or_else(|| "Ithil".into());
        match k {
            "A" | "menu" => {
                let mut args = vec!["ithil-talk"];
                if k == "menu" {
                    args.push("settings");
                }
                self.say(format!("Opening {name}… close her window to come back here"));
                self.act(&args, "")
            }
            "X" => self.act(&["ithil-restart"], "Restarting her app"),
            "Y" => {
                if !self.sleep_armed {
                    self.sleep_armed = true;
                    self.sleep_at = now();
                    self.say("Press Y again to put her to sleep");
                    return Task::none();
                }
                self.sleep_armed = false;
                self.act(&["ithil-sleep"], "She's going to sleep")
            }
            _ => Task::none(),
        }
    }
    fn service_act(&mut self, t: Option<Value>, a: &str) -> Task<Msg> {
        let Some(t) = t else {
            self.say(format!("Nothing to {a} here"));
            return Task::none();
        };
        let id = s(&t, "id");
        if id == "ithil" {
            return self.ithil_act(if a == "restart" { "X" } else { "Y" });
        }
        let acts: Vec<String> = arr(&t, "actions").iter().filter_map(|x| x.as_str().map(String::from)).collect();
        if !acts.iter().any(|x| x == if a == "restart" { "restart" } else { "pause" }) {
            self.say(format!("Nothing to {a} here"));
            return Task::none();
        }
        let mut a = a.to_string();
        if a == "pause" && s(&t, "state") == "paused-you" {
            a = "resume".into();
        }
        let title = s(&t, "title");
        // 1.13: pausing takes a second press within 4 seconds (resuming and restarting don't).
        if a == "pause" && self.pause_armed != id {
            self.pause_armed = id.clone();
            self.pause_at = now();
            let extra = if id == "team" || id == "ai" {
                let it = self.d.as_ref().and_then(|d| d.get("ithil")).cloned();
                match it {
                    Some(i) if truthy(&i, "running") => format!(" (AI chat stops, and {}'s brain with it)", s(&i, "name")),
                    _ => " (AI chat stops, and anything that thinks with it)".into(),
                }
            } else {
                String::new()
            };
            self.say(format!("Press again to pause {title}{extra}"));
            return Task::none();
        }
        self.pause_armed.clear();
        let said = format!("{title}: {}", match a.as_str() { "restart" => "restarting", "pause" => "paused", _ => "resuming" });
        self.act(&["service", &a, &id], &said)
    }

    // ---------------------------------------------------------------- the grid (tiles can be two wide)
    pub fn cells(&self) -> Vec<(usize, usize, usize)> {
        let mut out = vec![];
        let (mut c, mut r) = (0usize, 0usize);
        for t in self.tiles() {
            let w = if truthy(&t, "wide") { 2 } else { 1 };
            if c + w > COLS {
                r += 1;
                c = 0;
            }
            out.push((r, c, w));
            c += w;
            if c >= COLS {
                r += 1;
                c = 0;
            }
        }
        out
    }
    fn vertical(&self, from: usize, dir: i64) -> usize {
        let cs = self.cells();
        let Some(me) = cs.get(from - 1).copied() else { return from };
        let want = me.0 as i64 + dir;
        let mut best: Option<usize> = None;
        let mut bd = usize::MAX;
        for (i, c) in cs.iter().enumerate() {
            if c.0 as i64 == want {
                let dist = (c.1 as i64 - me.1 as i64).unsigned_abs() as usize;
                if dist < bd {
                    bd = dist;
                    best = Some(i);
                }
            }
        }
        match best {
            Some(b) => b + 1,
            None => if dir < 0 { 0 } else { from },
        }
    }
    fn last_in_row0(&self) -> usize {
        self.cells().iter().enumerate().filter(|(_, c)| c.0 == 0).map(|(i, _)| i + 1).last().unwrap_or(0)
    }

    /// Nothing moves while Reduce motion is on, the screen is dim or off, or another window (Settings, Ithil's app)
    /// is in front: the dashboard then draws only when its data changes.
    /// Nothing moves while Reduce motion is on, the screen is off (black), or another window (Settings, Ithil's
    /// app) is in front. The dim screen's faint cores move, as on Mantel.
    pub fn still(&self) -> bool {
        self.reduce || !self.focused || self.screen == Screen::Off
    }
    /// Drawn with the CPU (tiny-skia: no graphics card here, wgpu didn't start, or the watcher's fallback after two
    /// quick closes): the light mode, as smooth as the CPU allows without lagging the PC.
    pub fn light(&self) -> bool {
        self.smooth_level >= 2 || hearth_ui::app::renderer("hearth-server") == "cpu" || (now() - self.start > 6.0 && !hearth_ui::app::gpu_started())
    }
    /// Something moves for a reason (a core working, a sender lit, the AI team's or Ithil's page open): the screen's
    /// rate. Otherwise only the "ready" breathing and Ithil's tile, which never need more than 30 a second.
    fn at_work(&self) -> bool {
        let Some(d) = self.d.as_ref() else { return false };
        (self.overlay == Overlay::Detail && matches!(self.cur_id().as_str(), "ithil" | "team"))
            || arr(d, "cores").iter().any(|c| s(c, "state") == "working")
            || d.get("team").map(|tm| arr(tm, "senders").iter().any(|x| truthy(x, "active"))).unwrap_or(false)
    }
    /// The never-lag guard (the owner's rule: an update must never make his PCs lag): while the smooth motion runs,
    /// the app's own CPU time over the last ~10 s. Over 3 % of one core: down to 30 a second; still over: the light
    /// mode. For the rest of this session only (nothing is written anywhere); the dashboard says so, with Try smooth
    /// again.
    fn guard_tick(&mut self) {
        if self.smooth_level >= 3 {
            self.cpu_win.clear();
            return;
        }
        let t = now();
        if let Some(&(last, _)) = self.cpu_win.back() {
            if t - last < 1.0 {
                return;
            }
            if t - last > 3.0 {
                self.cpu_win.clear(); // it stood still in between (off, away): a new window
            }
        }
        let Some(ticks) = own_cpu_ticks() else { return };
        self.cpu_win.push_back((t, ticks));
        while self.cpu_win.len() > 2 && t - self.cpu_win[1].0 >= 10.0 {
            self.cpu_win.pop_front();
        }
        let (t0, c0) = self.cpu_win[0];
        if t - t0 < 9.5 {
            return;
        }
        let hz = unsafe { libc::sysconf(libc::_SC_CLK_TCK) }.max(1) as f64;
        let pct = 100.0 * (ticks - c0) as f64 / hz / (t - t0);
        // smooth (60 or 30 a second): 3 %; the light mode (also when the CPU draws): its own 8 %, then it stands still
        let limit = if self.light() { GUARD_LIGHT_PCT } else { GUARD_PCT };
        if pct > limit {
            self.smooth_level = if self.light() { 3 } else { self.smooth_level + 1 };
            self.guard_note = true;
            self.cpu_win.clear();
            eprintln!("server mode: smooth motion turned down to keep this PC fast ({pct:.1} % of a core over {:.0} s; now {})", t - t0, match self.smooth_level { 1 => "30 a second", 2 => "the light mode", _ => "standing still" });
        }
    }
    /// How often the dashboard moves (ms), or None.
    /// With the graphics card (the owner, Nov 20: "smooth like Mantel"): ServerCore.qml's motions at the screen's own
    /// rate (60 a second), the "ready" breathing too, whenever the dashboard is on the screen and in front.
    /// Light mode (drawn with the CPU): only while something is at work (a core working, a sender lit in the AI
    /// team's mesh, or the AI team's or Ithil's page open), 5 times a second; Ithil's small brain alone twice a second.
    pub fn anim_rate(&self) -> Option<u64> {
        if self.still() || self.smooth_level >= 3 {
            return None;
        }
        let d = self.d.as_ref()?;
        let has = !arr(d, "cores").is_empty() || d.get("ithil").map(|v| v.is_object()).unwrap_or(false) || self.tiles().iter().any(|t| truthy(t, "mesh"));
        if !self.light() {
            if !has {
                return None;
            }
            return Some(if self.smooth_level == 0 && self.at_work() { 16 } else { 33 });
        }
        if self.screen != Screen::On {
            return None;
        }
        if self.overlay == Overlay::Detail && matches!(self.cur_id().as_str(), "ithil" | "team") {
            return Some(200);
        }
        if self.overlay == Overlay::None || self.overlay == Overlay::Back {
            let busy = arr(d, "cores").iter().any(|c| s(c, "state") == "working")
                || d.get("team").map(|tm| arr(tm, "senders").iter().any(|x| truthy(x, "active"))).unwrap_or(false);
            if busy {
                return Some(200);
            }
            if d.get("ithil").map(|v| v.is_object()).unwrap_or(false) {
                return Some(500);
            }
        }
        None
    }
    pub fn start_secs(&self) -> f64 {
        self.start
    }
    fn animating(&self) -> bool {
        self.anim_rate().is_some()
    }
    fn open_detail(&mut self) {
        self.overlay = Overlay::Detail;
        self.fsel = 0;
        self.ask_empty = false;
        self.typing = false;
        self.step = 0;
    }
    fn read_brain(&mut self) -> Task<Msg> {
        let has = self.d.as_ref().and_then(|d| d.get("ithil")).map(|v| v.is_object()).unwrap_or(false);
        if !has || self.brain_reading || self.still() {
            return Task::none();
        }
        self.brain_reading = true;
        self.last_brain = now();
        let h = if self.ithil_win == "all" { "0" } else { "24" };
        Task::perform(run::spawn(self.cmd(&["ithil-brain", h])), Msg::BrainRead)
    }
    fn ithil_step(&mut self, d: f32) -> Task<Msg> {
        self.moved = now();
        let mut t = Task::none();
        if self.ithil_win == "live" {
            self.ithil_win = "24".into();
            self.tpos = 118.0;
            self.brain_reading = false;
            t = self.read_brain();
        }
        self.tpos = (self.tpos + d).clamp(0.0, 120.0);
        t
    }
    fn ithil_cycle(&mut self, d: i32) -> Task<Msg> {
        let w = ["live", "24", "all"];
        let i = w.iter().position(|x| *x == self.ithil_win).unwrap_or(0) as i32;
        self.ithil_win = w[((i + d + 3) % 3) as usize].into();
        self.tpos = 120.0;
        self.brain_reading = false;
        self.read_brain()
    }

    // ---------------------------------------------------------------- the cache, the ad blocker, the rules
    fn feat_rows(&self) -> usize {
        let Some(t) = self.cur() else { return 1 };
        match s(&t, "id").as_str() {
            "cache" => 1 + pages::STORES.len(),
            "ads" => 1 + pages::ads_devices(&t).len(),
            "rules" => {
                let (q, r, _) = pages::rules_of(&t);
                q.len() + r.len() + 1
            }
            _ => 1,
        }
    }
    /// Their changes: hearth-server-mode dns … (the cache, the ad blocker) or rules … (quiet hours and rules).
    fn feature_act(&mut self, args: Vec<String>) -> Task<Msg> {
        if args.first().map(String::as_str) == Some("settings") {
            self.settings("hs-server");
            return Task::none();
        }
        let what = if self.cur_id() == "rules" { "rules" } else { "dns" };
        let said = match args.first().map(String::as_str).unwrap_or("") {
            "cache" => "Game download cache",
            "store" => "Stores",
            "cache-empty" => "The cache is being emptied",
            "ads" | "ads-pause" => "Ad blocker",
            "allow" => "Allowed",
            "device" => "Your devices",
            "quiet-add" => "Quiet hours made (off until you turn it on)",
            "quiet-set" | "quiet-remove" => "Quiet hours",
            "rule-add" => "Rule made (off until you turn it on)",
            "rule-set" | "rule-pause" | "rule-remove" => "Rule",
            "while" => "While quiet",
            _ => "Saved",
        };
        let mut full = vec![what.to_string()];
        full.extend(args);
        let refs: Vec<&str> = full.iter().map(String::as_str).collect();
        self.act(&refs, &format!("{said} ✓"))
    }
    fn feat_pick(&mut self, d: i64) {
        match self.step {
            1 => self.wi = (self.wi as i64 + d).clamp(0, pages::whens(self).len() as i64 - 1) as usize,
            2 => self.di = (self.di as i64 + d).clamp(0, pages::dos(self).len() as i64 - 1) as usize,
            _ => {
                if !self.typing {
                    self.fsel = (self.fsel as i64 + d).clamp(0, self.feat_rows() as i64 - 1) as usize;
                }
                self.ask_empty = false;
            }
        }
    }
    fn feat_adjust(&mut self, d: i64) {
        if self.cur_id() != "rules" {
            return;
        }
        if self.step == 1 {
            let w = pages::whens(self)[self.wi].0;
            let v = &mut self.when_vals;
            match w {
                "drive" => v.0 = (v.0 + 5 * d).clamp(30, 90),
                "power" => v.1 = (v.1 + 50 * d).clamp(50, 1500),
                "time" | "quiet" => v.2 = (v.2 + 30 * d + 1440) % 1440,
                _ => {}
            }
        } else if self.step == 2 && pages::dos(self)[self.di].0 == "fans" {
            self.fan_pct = (self.fan_pct + 10 * d).clamp(20, 100);
        }
    }
    fn new_rule(&mut self) {
        self.step = 1;
        self.wi = 0;
        self.di = 0;
        self.device = self.cur().map(|t| arr(&t, "devices")).and_then(|d| d.first().map(|x| x.as_str().map(String::from).unwrap_or_else(|| x.to_string()))).unwrap_or_default();
    }
    fn feat_enter(&mut self) -> Task<Msg> {
        let Some(t) = self.cur() else { return Task::none() };
        let id = s(&t, "id");
        let sel = self.fsel;
        let st = |b: bool| if b { "off" } else { "on" }.to_string();
        match id.as_str() {
            "cache" => {
                self.ask_empty = false;
                let c = t.get("cache").cloned().unwrap_or(Value::Null);
                if sel == 0 {
                    self.feature_act(vec!["cache".into(), st(truthy(&c, "on"))])
                } else {
                    let s_ = pages::STORES[sel - 1];
                    let on = c.get("stores").map(|x| truthy(x, s_)).unwrap_or(false);
                    self.feature_act(vec!["store".into(), s_.into(), st(on)])
                }
            }
            "ads" => {
                if self.typing {
                    let site = self.site.trim().to_string();
                    self.typing = false;
                    return if site.is_empty() { Task::none() } else { self.feature_act(vec!["allow".into(), site]) };
                }
                let ad = t.get("ads").cloned().unwrap_or(Value::Null);
                if sel == 0 {
                    self.feature_act(vec!["ads".into(), st(truthy(&ad, "on"))])
                } else {
                    let d = pages::ads_devices(&t).get(sel - 1).cloned().unwrap_or(Value::Null);
                    self.feature_act(vec!["device".into(), s(&d, "id"), st(truthy(&d, "on"))])
                }
            }
            "rules" => {
                let (quiet, rules, _) = pages::rules_of(&t);
                let hh = pages::hhmm;
                if self.step == 1 {
                    if pages::whens(self)[self.wi].0 == "quiet" {
                        self.step = 0;
                        let v = self.when_vals;
                        return self.feature_act(vec!["quiet-add".into(), "Quiet hours".into(), hh(v.2), hh(v.3), "mon-fri".into()]);
                    }
                    self.step = 2;
                    return Task::none();
                }
                if self.step == 2 {
                    let v = self.when_vals;
                    let w = match pages::whens(self)[self.wi].0 {
                        "device" => format!("device:{}", if self.device.is_empty() { "phone".into() } else { self.device.clone() }),
                        "time" => format!("time:{}-{}", hh(v.2), hh(v.3)),
                        "drive" => format!("drive:{}", v.0),
                        "power" => format!("power:{}", v.1),
                        x => x.to_string(),
                    };
                    let d = match pages::dos(self)[self.di].0 { "fans" => format!("fans:{}", self.fan_pct), x => x.to_string() };
                    self.step = 0;
                    self.fsel = quiet.len() + rules.len();
                    return self.feature_act(vec!["rule-add".into(), w, d]);
                }
                if sel == quiet.len() + rules.len() {
                    self.new_rule();
                    return Task::none();
                }
                if sel < quiet.len() {
                    let q = &quiet[sel];
                    self.feature_act(vec!["quiet-set".into(), s(q, "id"), st(truthy(q, "on"))])
                } else {
                    let r = &rules[sel - quiet.len()];
                    self.feature_act(vec!["rule-set".into(), s(r, "id"), st(truthy(r, "on"))])
                }
            }
            _ => Task::none(),
        }
    }
    /// X, Y and ☰ on those pages (ServerDash.qml featureKey).
    fn feat_key(&mut self, k: &str) -> Task<Msg> {
        let Some(t) = self.cur() else { return Task::none() };
        let id = s(&t, "id");
        match (id.as_str(), k) {
            (_, "enter") => self.feat_enter(),
            ("cache", "x") => {
                if self.ask_empty {
                    self.ask_empty = false;
                    self.feature_act(vec!["cache-empty".into(), "sure".into()])
                } else {
                    self.ask_empty = true;
                    Task::none()
                }
            }
            ("cache", "y") | ("ads", "menu") => self.feature_act(vec!["settings".into()]),
            ("ads", "x") => {
                self.typing = true;
                self.site.clear();
                iced::widget::operation::focus(SITE_ID)
            }
            ("ads", "y") => {
                let paused = t.get("ads").and_then(|a| a.get("view")).and_then(|v| f(v, "pausedUntil")).unwrap_or(0.0) > 0.0;
                if self.fsel == 0 {
                    self.feature_act(vec!["ads-pause".into(), if paused { "0" } else { "3600" }.into()])
                } else {
                    let d = pages::ads_devices(&t).get(self.fsel - 1).cloned().unwrap_or(Value::Null);
                    self.feature_act(vec!["device".into(), s(&d, "id"), "pause".into()])
                }
            }
            ("rules", "x") => {
                if self.step > 0 {
                    return Task::none();
                }
                let (quiet, rules, _) = pages::rules_of(&t);
                let sel = self.fsel;
                if sel >= quiet.len() + rules.len() {
                    return Task::none();
                }
                // edit = remove it and pick it again (its When and Do picked as they were)
                if sel < quiet.len() {
                    let tk = self.feature_act(vec!["quiet-remove".into(), s(&quiet[sel], "id")]);
                    self.new_rule();
                    self.wi = pages::whens(self).len() - 1;
                    return tk;
                }
                let r = rules[sel - quiet.len()].clone();
                let wk = s(&r, "when").split(':').next().unwrap_or("").to_string();
                let dk = s(&r, "do").split(':').next().unwrap_or("").to_string();
                let tk = self.feature_act(vec!["rule-remove".into(), s(&r, "id")]);
                self.new_rule();
                self.wi = pages::whens(self).iter().position(|x| x.0 == wk).unwrap_or(0);
                self.di = pages::dos(self).iter().position(|x| x.0 == dk).unwrap_or(0);
                tk
            }
            ("rules", "y") => {
                let (quiet, rules, _) = pages::rules_of(&t);
                if self.step > 0 || self.fsel < quiet.len() || self.fsel >= quiet.len() + rules.len() {
                    return Task::none();
                }
                let r = &rules[self.fsel - quiet.len()];
                let paused = truthy(r, "paused");
                self.feature_act(vec!["rule-pause".into(), s(r, "id"), if paused { "0" } else { "3600" }.into()])
            }
            ("rules", "menu") => {
                self.new_rule();
                Task::none()
            }
            _ => Task::none(),
        }
    }
    /// B on those pages: out of the site box or a picker step first.
    fn feat_back(&mut self) {
        if self.typing {
            self.typing = false;
        } else if self.step > 0 {
            self.step -= 1;
        } else {
            self.overlay = Overlay::None;
        }
    }

    fn write_up_mark(&mut self) {
        if self.up_written {
            return;
        }
        self.up_written = true;
        // nimbus-session waits for this mark: without it, Server mode is closed and another mode opens.
        let _ = std::fs::write(format!("{}/hearth-server-dash-up", self.rundir), b"");
        let dir = format!("{}/hearth-server", self.rundir);
        if std::fs::create_dir_all(&dir).is_ok() {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700));
            let cd = format!("{dir}/countdown.json");
            if !std::path::Path::new(&cd).exists() {
                let _ = std::fs::write(cd, b"{}\n");
            }
        }
        self.alive();
    }
    fn alive(&mut self) {
        self.last_alive = now();
        let _ = std::fs::File::create(format!("{}/hearth-server-dash-alive", self.rundir));
    }

    fn apply_show(&mut self) {
        let Some(sh) = self.show.take() else { return };
        match sh.as_str() {
            "switch" => {
                self.overlay = Overlay::Switch;
                self.count = 5;
            }
            "menu" => {
                self.overlay = Overlay::Menu;
                self.menu_sel = 0;
            }
            "back" => {
                self.back_info = Some(serde_json::json!({"back": true, "played": 9660}));
                self.overlay = Overlay::Back;
                self.back_at = now() + 1e9;
            }
            "dim" => self.screen = Screen::Dim,
            "new-rule" => {
                if let Some(i) = self.tiles().iter().position(|t| s(t, "id") == "rules") {
                    self.sel = i + 1;
                    self.open_detail();
                    self.new_rule();
                }
            }
            x if x.starts_with("page:") => {
                let id = &x[5..];
                if let Some(i) = self.tiles().iter().position(|t| s(t, "id") == id) {
                    self.sel = i + 1;
                    self.open_detail();
                }
                if let Some(w) = app::arg("--ithil-win") {
                    self.ithil_win = w;
                    self.tpos = 80.0;
                }
            }
            x if x.starts_with("detail:") => {
                if let Ok(n) = x[7..].parse::<usize>() {
                    self.sel = n;
                    self.open_detail();
                }
            }
            _ => {}
        }
    }
}

fn update(a: &mut App, m: Msg) -> Task<Msg> {
    a.now = now();
    match m {
        Msg::Describe(dm) => return a.describe.update(dm).map(Msg::Describe),
        Msg::Dash(o) => {
            a.reading = false;
            match o.json().filter(|v| v.get("tiles").map(Value::is_array).unwrap_or(false)) {
                Some(v) => {
                    if truthy(&v, "busy") {
                        a.last_busy = now();
                    }
                    let qr = s(&v, "qr");
                    if qr != a.qr_path {
                        a.qr = std::fs::read_to_string(&qr).ok().and_then(|t| draw::parse_qr(&t));
                        a.qr_path = qr;
                    }
                    a.d = Some(v);
                    a.sample();
                    a.apply_show();
                    if a.last_brain == 0.0 {
                        return a.read_brain();
                    }
                }
                None => eprintln!("server mode: dash: {}", o.why()),
            }
        }
        Msg::Acted(o, ok) => {
            let t = o.out.trim();
            if let Some(e) = t.strip_prefix("error: ") {
                a.say(e.to_string());
            } else if !o.ok && o.code == 124 {
                a.say("That took too long; try again");
            } else if let Some(e) = o.err.trim().lines().last().and_then(|l| l.strip_prefix("error: ")) {
                a.say(e.to_string());
            } else if !ok.is_empty() {
                a.say(ok);
            }
            return a.refresh();
        }
        Msg::BackRead(o) => {
            if let Some(v) = o.json() {
                if truthy(&v, "back") {
                    a.back_info = Some(v);
                    a.overlay = Overlay::Back;
                    a.back_at = now();
                    // Light mode resumes now, not in a minute.
                    run::detached(&["timeout", "200", "python3", &a.helper, "nudge"]);
                }
            }
        }
        Msg::Tick => {
            let t = now();
            a.read_remote();
            if a.overlay == Overlay::Switch {
                a.count -= 1;
                if a.count <= 0 {
                    return a.switch_now();
                }
            }
            if !a.toast.is_empty() && t - a.toast_at > 4.5 {
                a.toast.clear();
            }
            if !a.pause_armed.is_empty() && t - a.pause_at > 4.0 {
                a.pause_armed.clear();
            }
            if a.sleep_armed && t - a.sleep_at > 4.0 {
                a.sleep_armed = false;
            }
            if a.overlay == Overlay::Back && t - a.back_at > 90.0 {
                a.overlay = Overlay::None;
            }
            if a.up_written && t - a.last_alive >= 10.0 {
                a.alive();
            }
            let mut it = a.idle_tick();
            if t - a.last_brain >= 3.0 {
                it = Task::batch([it, a.read_brain()]);
            }
            if t - a.last_read >= 4.0 {
                return Task::batch([it, a.refresh()]);
            }
            return it;
        }
        Msg::Opened(sz) => {
            a.size = sz;
            a.write_up_mark();
        }
        Msg::Resized(sz) => {
            a.size = sz;
            a.write_up_mark();
        }
        Msg::Wake => a.wake(),
        Msg::Pad(ev) => return pad(a, &ev),
        Msg::Pick(i) => {
            a.wake();
            a.sel = i;
            if i == 0 {
                a.overlay = Overlay::None;
                a.play();
            } else {
                a.open_detail();
            }
        }
        Msg::Play => {
            a.wake();
            a.sel = 0;
            a.play();
        }
        Msg::MenuItem(i) => return a.menu_do(i),
        Msg::SwitchNow => return a.switch_now(),
        Msg::CancelSwitch => a.overlay = Overlay::None,
        Msg::RemoteNow => return a.remote_now(),
        Msg::RemoteCancel => return a.remote_cancel(),
        Msg::Close => a.overlay = Overlay::None,
        Msg::DetailAct(x) => return detail_act(a, &x),
        Msg::Key(ev) => return key(a, ev),
        Msg::Anim => {
            let t = now();
            let dt = (t - a.last_anim).min(0.2) as f32;
            a.last_anim = t;
            a.anim_t = (t - a.start) as f32;
            a.guard_tick();
            // not live: the window plays through slowly by itself (120 s), unless you just moved it
            if a.overlay == Overlay::Detail && a.cur_id() == "ithil" && a.ithil_win != "live" && t - a.moved > 4.0 {
                a.tpos = if a.tpos >= 120.0 { 0.0 } else { a.tpos + dt };
            }
        }
        Msg::Focus(fo) => {
            // Only once the window has had the keyboard (a screen without a keyboard, like CI's, never gives it):
            // losing it then means another window is in front.
            if fo {
                a.seen_focus = true;
            }
            if !a.seen_focus {
                return Task::none();
            }
            a.focused = fo;
            if fo {
                a.wake();
                a.last_anim = now();
            }
        }
        Msg::Back => a.feat_back(),
        Msg::SmoothAgain => {
            a.smooth_level = 0;
            a.guard_note = false;
            a.cpu_win.clear();
        }
        Msg::FeatRow(i) => {
            a.fsel = i;
            return a.feat_enter();
        }
        Msg::FeatKey(k) => return a.feat_key(&k),
        Msg::PickChoice(i) => {
            if a.step == 1 {
                a.wi = i;
            } else {
                a.di = i;
            }
            return a.feat_enter();
        }
        Msg::Site(v) => a.site = v.chars().take(253).collect(),
        Msg::IthilWin(w) => {
            a.ithil_win = w;
            a.tpos = 120.0;
            a.brain_reading = false;
            return a.read_brain();
        }
        Msg::BrainMove(p) => {
            // the pointer over her brain turns it (the right stick moves the pointer)
            if let Some(l) = a.mouse_last {
                a.yaw += (p.x - l.x) * 0.01;
                a.pitch = (a.pitch + (p.y - l.y) * 0.01).clamp(-1.4, 1.4);
            }
            a.mouse_last = Some(p);
        }
        Msg::BrainLeave => a.mouse_last = None,
        Msg::BrainRead(o) => {
            a.brain_reading = false;
            if let Some(v) = o.json() {
                a.ithil_prep = brain::prepare(Some(&v));
            }
        }
    }
    Task::none()
}

/// A button on an opened tile's page.
fn detail_act(a: &mut App, x: &str) -> Task<Msg> {
    let cur = a.cur();
    match x {
        "unload" => a.act(&["ai-unload"], "Model unloaded"),
        "settings-team" => {
            a.settings("hs-aidevices");
            Task::none()
        }
        "settings" => {
            a.settings("hs-server");
            Task::none()
        }
        "talk" => a.ithil_act("A"),
        "ithil-restart" => a.ithil_act("X"),
        "ithil-sleep" => a.ithil_act("Y"),
        "ithil-settings" => a.ithil_act("menu"),
        other => a.service_act(cur, other),
    }
}

/// From hearth-pad: Guide is Play, ☰ the menu, Y pause / resume (server-mode.qml's).
fn pad(a: &mut App, ev: &str) -> Task<Msg> {
    // Settings or Ithil's app is in front of the dashboard in cage (it doesn't have the keyboard): Guide or ☰ bring
    // the dashboard back. cage shows the window that opened last, so the dashboard opens its window again: it ends
    // with 42 and its watcher (desktop/server/cage/run) starts it again at once, not counted as a failure.
    if !a.focused && (ev == "guide" || ev == "menu") && std::env::var("HEARTH_WM").as_deref() == Ok("cage") {
        eprintln!("server mode: Guide while another window is in front: the dashboard comes back to the front");
        std::process::exit(42);
    }
    let asleep = a.screen != Screen::On;
    a.wake();
    if asleep {
        return Task::none();
    }
    let id = a.cur_id();
    if a.overlay == Overlay::Detail && matches!(id.as_str(), "cache" | "ads" | "rules") && (ev == "menu" || ev == "y") {
        return a.feat_key(ev);
    }
    if a.overlay == Overlay::Detail && id == "ithil" {
        if ev == "menu" {
            return a.ithil_act("menu");
        }
        if ev == "y" {
            return a.ithil_act("Y");
        }
        if ev == "view" {
            a.xray = !a.xray;
            return Task::none();
        }
    }
    match ev {
        "guide" => {
            if a.remote.is_some() {
                return a.remote_cancel();
            } else if a.overlay == Overlay::Switch {
                a.overlay = Overlay::None;
            } else {
                a.play();
            }
        }
        "menu" => {
            a.overlay = if a.overlay == Overlay::Menu { Overlay::None } else { Overlay::Menu };
            a.menu_sel = 0;
        }
        "y" => {
            if a.overlay == Overlay::Detail && id == "team" {
                return detail_act(a, "settings-team");
            } else if a.overlay == Overlay::None || a.overlay == Overlay::Detail {
                let c = a.cur();
                return a.service_act(c, "pause");
            }
        }
        _ => {}
    }
    Task::none()
}

fn key(a: &mut App, ev: KEvent) -> Task<Msg> {
    let KEvent::KeyPressed { key, modifiers, .. } = ev else { return Task::none() };
    let shift = modifiers.shift();
    let asleep = a.screen != Screen::On;
    a.wake();
    if asleep {
        return Task::none();
    }
    let ch = match key.as_ref() {
        Key::Character(c) => c.to_lowercase(),
        _ => String::new(),
    };
    let named = match key.as_ref() {
        Key::Named(n) => Some(n),
        _ => None,
    };
    let is = |n: Named| named == Some(n);
    let enter = is(Named::Enter) || is(Named::Space) || ch == " ";
    let back = is(Named::Escape) || is(Named::Backspace);
    let x = is(Named::ContextMenu) || ch == "x" || ch == "r";
    let y = ch == "y" || ch == "p";
    let menu = ch == "m" || is(Named::F1);
    let play_key = ch == "g" || is(Named::Home) || is(Named::F12);
    let (up, down, left, right) = (is(Named::ArrowUp), is(Named::ArrowDown), is(Named::ArrowLeft), is(Named::ArrowRight));

    if a.remote.is_some() {
        if enter {
            return a.remote_now();
        } else if back {
            return a.remote_cancel();
        }
        return Task::none();
    }
    match a.overlay {
        Overlay::Switch => {
            if enter {
                return a.switch_now();
            } else if back {
                a.overlay = Overlay::None;
            }
            return Task::none();
        }
        Overlay::Back => {
            a.overlay = Overlay::None;
            return Task::none();
        }
        Overlay::Menu => {
            let n = a.menu_items().len();
            if down {
                a.menu_sel = (a.menu_sel + 1) % n;
            } else if up {
                a.menu_sel = (a.menu_sel + n - 1) % n;
            } else if enter {
                return a.menu_do(a.menu_sel);
            } else if back || menu {
                a.overlay = Overlay::None;
            }
            return Task::none();
        }
        Overlay::Detail => {
            let id = a.cur_id();
            if matches!(id.as_str(), "cache" | "ads" | "rules") {
                // ▲ ▼ pick a row, A turns it on or off, X / Y / ☰ as their pages say, ◀ ▶ a new rule's number, B back
                if back {
                    a.feat_back();
                } else if up || down {
                    a.feat_pick(if down { 1 } else { -1 });
                } else if left || right {
                    a.feat_adjust(if right { 1 } else { -1 });
                } else if enter {
                    return a.feat_enter();
                } else if x {
                    return a.feat_key("x");
                } else if y {
                    return a.feat_key("y");
                } else if menu {
                    return a.feat_key("menu");
                }
                return Task::none();
            }
            if back {
                a.overlay = Overlay::None;
                return Task::none();
            }
            if id == "ithil" {
                // LB / RB (Alt+Tab from the pad) or the arrows move through time, ▲ ▼ the window, V X-ray
                if is(Named::Tab) || left || right {
                    let backw = left || (is(Named::Tab) && shift);
                    return a.ithil_step(if backw { -4.0 } else { 4.0 });
                }
                if up || down {
                    return a.ithil_cycle(if down { 1 } else { -1 });
                }
                if ch == "v" {
                    a.xray = !a.xray;
                    return Task::none();
                }
            }
            return match id.as_str() {
                "ithil" if enter => a.ithil_act("A"),
                "ithil" if x => a.ithil_act("X"),
                "ithil" if y => a.ithil_act("Y"),
                "team" if enter => detail_act(a, "pause"),
                "team" if x => detail_act(a, "restart"),
                "team" if y => detail_act(a, "settings-team"),
                "ai" if enter => detail_act(a, "unload"),
                "ithil" | "team" => Task::none(),
                _ if x => detail_act(a, "restart"),
                _ if y => detail_act(a, "pause"),
                _ => Task::none(),
            };
        }
        Overlay::None => {}
    }
    if play_key {
        a.play();
        return Task::none();
    }
    if menu {
        a.overlay = Overlay::Menu;
        a.menu_sel = 0;
        return Task::none();
    }
    if enter {
        if a.sel == 0 {
            a.play();
        } else if a.cur().is_some() {
            a.open_detail();
        }
        return Task::none();
    }
    if x {
        let c = a.cur();
        return a.service_act(c, "restart");
    }
    if y {
        let c = a.cur();
        return a.service_act(c, "pause");
    }
    let n = a.tiles().len();
    let cs = a.cells();
    // Play sits to the right of the tiles: Right from a row's last tile goes to it, Left from Play to the top row's last.
    if right {
        a.sel = if a.sel == 0 {
            0
        } else {
            let me = cs[a.sel - 1];
            if me.1 + me.2 >= COLS || a.sel == n { 0 } else { (a.sel + 1).min(n) }
        };
    } else if left {
        a.sel = if a.sel == 0 { a.last_in_row0() } else { (a.sel - 1).max(1) };
    } else if down {
        a.sel = if a.sel == 0 { a.last_in_row0() } else { a.vertical(a.sel, 1) };
    } else if up {
        a.sel = if a.sel == 0 { 0 } else { a.vertical(a.sel, -1) };
    }
    Task::none()
}

fn window_event(e: iced::Event, _s: iced::event::Status, _w: iced::window::Id) -> Option<Msg> {
    use iced::{mouse, window, Event};
    match e {
        Event::Window(window::Event::Opened { size, .. }) => Some(Msg::Opened(size)),
        Event::Window(window::Event::Resized(size)) => Some(Msg::Resized(size)),
        Event::Window(window::Event::Focused) => Some(Msg::Focus(true)),
        Event::Window(window::Event::Unfocused) => Some(Msg::Focus(false)),
        Event::Mouse(mouse::Event::ButtonPressed(_)) | Event::Mouse(mouse::Event::CursorMoved { .. }) => Some(Msg::Wake),
        Event::Touch(_) => Some(Msg::Wake),
        _ => None,
    }
}

// ---------------------------------------------------------------- the controller (hearth-pad, keys mode)
/// hearth-pad (desktop/shell) in keys mode the whole time, as server-mode.qml runs it: its keys arrive as key
/// presses; Guide, ☰, Y and View come as its JSON lines. Without python3-evdev or /dev/uinput it ends with 2 or 3
/// and isn't tried again (the keyboard still works); otherwise it's started again after 3 seconds. One thread,
/// blocked on its output; nothing polls.
fn pad_stream() -> futures::channel::mpsc::UnboundedReceiver<String> {
    let (tx, rx) = futures::channel::mpsc::unbounded();
    if std::env::var("HEARTH_SERVER_NO_PAD").as_deref() == Ok("1") {
        return rx;
    }
    let _ = std::thread::Builder::new().name("hearth-server-pad".into()).spawn(move || {
        use std::io::{BufRead, BufReader, Write};
        use std::process::{Command, Stdio};
        let helper = run::shell_helper("hearth-pad");
        loop {
            let Ok(mut child) = Command::new("python3").arg(&helper).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn() else { return };
            let mut stdin = child.stdin.take();
            if let Some(out) = child.stdout.take() {
                for line in BufReader::new(out).lines().map_while(Result::ok) {
                    let Ok(m) = serde_json::from_str::<Value>(&line) else { continue };
                    let mut send = |o: Value| {
                        if let Some(i) = stdin.as_mut() {
                            let _ = writeln!(i, "{o}");
                            let _ = i.flush();
                        }
                    };
                    match s(&m, "t").as_str() {
                        // "mouse off" is hearth-pad's whole-controller off switch: never sent here.
                        "ready" => {
                            send(serde_json::json!({"cmd": "keys", "on": true, "why": "server mode"}));
                            send(serde_json::json!({"cmd": "mouse", "on": true}));
                        }
                        "radial" if truthy(&m, "open") => {
                            send(serde_json::json!({"cmd": "radial", "open": false}));
                            let _ = tx.unbounded_send("guide".into());
                        }
                        "launcher" => {
                            let _ = tx.unbounded_send("menu".into());
                        }
                        "osk" => {
                            let _ = tx.unbounded_send("y".into());
                        }
                        "help" => {
                            let _ = tx.unbounded_send("view".into());
                        }
                        _ => {}
                    }
                }
            }
            drop(stdin);
            let code = child.wait().ok().and_then(|s| s.code()).unwrap_or(1);
            if code == 2 || code == 3 || tx.is_closed() {
                return;
            }
            std::thread::sleep(Duration::from_secs(3));
        }
    });
    rx
}

fn subscription(a: &App) -> Subscription<Msg> {
    // Once a second while a countdown shows, else every 2 s (the clock, the countdown file, idle): never faster.
    let fast = a.overlay == Overlay::Switch || a.remote.is_some();
    let tick = iced::time::every(Duration::from_millis(if fast { 1000 } else { 2000 })).map(|_| Msg::Tick);
    // The cores, the AI team's mesh and Ithil's brain move only while something is at work (anim_rate) and the
    // dashboard is on the screen, in front, with Reduce motion off (still()); otherwise only the 2-second clock ticks.
    let anim = match a.anim_rate() { Some(ms) => iced::time::every(Duration::from_millis(ms)).map(|_| Msg::Anim), None => Subscription::none() };
    Subscription::batch([
        anim,
        tick,
        iced::keyboard::listen().map(Msg::Key),
        iced::event::listen_with(window_event),
        Subscription::run(pad_stream).map(Msg::Pad),
    ])
}

fn main() -> iced::Result {
    if app::handle_cli(
        "hearth-server-app",
        "Server mode's dashboard (full screen; nimbus-session starts it in cage on the Fedora edition).\n  --show detail:N | switch | menu | back | dim   open with that showing (pictures)\n  --sel N   the picked tile",
    ) {
        return Ok(());
    }
    let mut w = app::window("hearth-server", (1920.0, 1080.0), (640.0, 400.0));
    w.fullscreen = std::env::var("HEARTH_SERVER_WINDOWED").as_deref() != Ok("1");
    iced::application(App::new, update, view::shown)
        .title("Server mode")
        .subscription(subscription)
        .theme(|_a: &App| iced::Theme::custom("Hearth server", iced::theme::Palette {
            background: draw::rgb(0x030201),
            text: draw::rgb(0xffe9c7),
            primary: draw::rgb(0xffb347),
            success: draw::rgb(0x6fe39a),
            warning: draw::rgb(0xff8a4c),
            danger: draw::rgb(0xff8a4c),
        }))
        .window(w)
        .default_font(app::INTER)
        .run()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_tiles(n: usize, wide_at: Option<usize>) -> App {
        let (mut a, _) = (App::new().0, ());
        let tiles: Vec<Value> = (0..n).map(|i| serde_json::json!({"id": format!("t{i}"), "wide": Some(i) == wide_at})).collect();
        a.d = Some(serde_json::json!({"tiles": tiles}));
        a
    }

    #[test]
    fn a_wide_tile_takes_two_cells() {
        std::env::set_var("HEARTH_SERVER_NO_PAD", "1");
        let a = with_tiles(6, Some(1));
        let c = a.cells();
        assert_eq!(c[0], (0, 0, 1));
        assert_eq!(c[1], (0, 1, 2));
        assert_eq!(c[3], (0, 4, 1));
        assert_eq!(c[4], (1, 0, 1));
        assert_eq!(a.last_in_row0(), 4);
        assert_eq!(a.vertical(5, -1), 1);
        assert_eq!(a.vertical(1, -1), 0, "up from the top row goes to Play");
    }
}
