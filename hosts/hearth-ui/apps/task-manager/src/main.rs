//! Task Manager, in pure Rust (Mantel's TaskManagerView.qml and its approved mockups ported; the checklist of what's
//! in is docs/mockups/checklists/task-manager-rust.md): the Windows 11 way, with a title bar and search, the left
//! rail (Processes, Performance, App history, Startup apps, Users, Details, Services, Scheduled tasks, Settings at
//! the bottom), the process list with heat-map columns and the right-click menu, the Performance graphs, and the
//! lists of start-up apps, users, services and scheduled tasks.
//!
//! The numbers come from its own sampler (sample.rs, apps.rs: /proc and /sys, only while it's open, only what the
//! page on show needs). What changes something goes through Hearth's `hearth-tasks` (the same checks as Mantel's
//! Task Manager: only your own processes, never a part of Hearth). Your choices (start page, update speed, the rail
//! folded, sort, the last page…) are kept in ~/.config/nimbus/taskmgr.json through settings_file.rs, only what you
//! change (red flag 9); opening it writes nothing.

mod apps;
mod pages;
mod sample;

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::Ordering;
use std::sync::Arc;

use hearth_ui::app::{self, win};
use hearth_ui::run;
use hearth_ui::settings_file;
use hearth_ui::Pal;
use iced::keyboard::{key::Named, Event as KEvent, Key};
use iced::{Element, Point, Subscription, Task};
use serde_json::{Map, Value};

use sample::{Sample, W_DETAILS, W_PROCS, W_SENSORS};

pub const HISTORY: usize = 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Page {
    Processes,
    Performance,
    History,
    Startup,
    Users,
    Details,
    Services,
    Tasks,
    Settings,
}

impl Page {
    pub const ALL: [Page; 9] = [Page::Processes, Page::Performance, Page::History, Page::Startup, Page::Users, Page::Details, Page::Services, Page::Tasks, Page::Settings];
    pub fn id(self) -> &'static str {
        match self {
            Page::Processes => "processes",
            Page::Performance => "performance",
            Page::History => "history",
            Page::Startup => "startup",
            Page::Users => "users",
            Page::Details => "details",
            Page::Services => "services",
            Page::Tasks => "tasks",
            Page::Settings => "settings",
        }
    }
    pub fn from_id(s: &str) -> Option<Page> {
        let s = match s {
            "apps" => "processes",
            "perf" => "performance",
            "startups" => "startup",
            x => x,
        };
        let base = s.split(':').next().unwrap_or(s);
        Page::ALL.iter().copied().find(|p| p.id() == base)
    }
    pub fn label(self) -> &'static str {
        match self {
            Page::Processes => "Processes",
            Page::Performance => "Performance",
            Page::History => "App history",
            Page::Startup => "Startup apps",
            Page::Users => "Users",
            Page::Details => "Details",
            Page::Services => "Services",
            Page::Tasks => "Scheduled tasks",
            Page::Settings => "Settings",
        }
    }
    /// The hearth-tasks list this page shows (one-shot, when it opens and every few seconds while open).
    pub fn list(self) -> Option<&'static str> {
        match self {
            Page::History => Some("history"),
            Page::Startup => Some("startup"),
            Page::Users => Some("users"),
            Page::Services => Some("services"),
            Page::Tasks => Some("timers"),
            _ => None,
        }
    }
    fn want(self) -> u8 {
        match self {
            Page::Processes => W_PROCS,
            Page::Details => W_DETAILS,
            Page::Performance => W_SENSORS,
            _ => 0,
        }
    }
}

/// What a row in the right-click menu does.
#[derive(Debug, Clone, PartialEq)]
pub enum Act {
    Expand(String),
    End,
    Tree,
    Eco(bool),
    PriorityMenu,
    Priority(&'static str),
    Cores,
    Limit,
    Resume,
    Suspend(bool),
    Keep(bool),
    Never,
    Details,
    Where,
    Web,
    Props,
    Dump,
    Service(&'static str),
    StartupSet(bool),
    TimerSet(bool),
}

#[derive(Debug, Clone)]
pub struct MenuItem {
    pub label: String,
    pub act: Option<Act>,
    pub enabled: bool,
    pub checked: bool,
    pub fresh: bool,
    pub key: String,
}

impl MenuItem {
    pub fn new(label: &str, act: Act) -> MenuItem {
        MenuItem { label: label.into(), act: Some(act), enabled: true, checked: false, fresh: false, key: String::new() }
    }
    pub fn sep() -> MenuItem {
        MenuItem { label: String::new(), act: None, enabled: false, checked: false, fresh: false, key: String::new() }
    }
    pub fn on(mut self, e: bool) -> Self {
        self.enabled = e;
        self
    }
    pub fn checked(mut self, c: bool) -> Self {
        self.checked = c;
        self
    }
    pub fn fresh(mut self) -> Self {
        self.fresh = true;
        self
    }
    pub fn key(mut self, k: String) -> Self {
        self.key = k;
        self
    }
}

/// What a menu or sheet acts on.
#[derive(Debug, Clone, Default)]
pub struct Target {
    pub gid: String,
    pub pid: i32,
    pub name: String,
    pub path: String,
    pub child: bool,
    pub protected: bool,
    pub unit: String,
    pub list_id: String,
}

#[derive(Debug, Clone)]
pub struct Menu {
    pub title: String,
    pub items: Vec<MenuItem>,
    pub at: Point,
    pub target: Target,
    pub sel: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SheetKind {
    Notice,
    Ending,
    Stuck,
    Ask(&'static str),
    Run,
}

#[derive(Debug, Clone)]
pub struct Sheet {
    pub kind: SheetKind,
    pub title: String,
    pub text: String,
    pub input: String,
    pub target: Target,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Theme,
    Sample(Arc<Sample>),
    List(&'static str, run::Output),
    Acted(String, run::Output),
    Page(Page),
    PerfDevice(String),
    Query(String),
    ProcView(&'static str),
    Sort(&'static str, &'static str, bool),
    Select(String),
    Toggle(String),
    RowMenu(String),
    Cursor(Point),
    MenuPick(usize),
    CloseMenu,
    TopEnd,
    TopEco,
    TopWhere,
    SheetInput(String),
    SheetOk,
    SheetForce,
    SheetWait,
    CloseSheet,
    EndCheck,
    RunNew,
    ListAct(&'static str, String, String),
    Pref(&'static str, Value),
    ClearHistory,
    Key(KEvent),
    Drag,
    Maximize,
    Minimize,
    Close,
    Resize(iced::window::Direction),
    Tick,
}

pub struct App {
    pub p: Pal,
    pub page: Page,
    pub sample: Option<Arc<Sample>>,
    pub cpu_hist: VecDeque<f32>,
    pub core_hist: Vec<VecDeque<f32>>,
    pub mem_hist: VecDeque<f32>,
    pub zram_hist: VecDeque<f32>,
    pub disk_hist: HashMap<String, VecDeque<f32>>,
    pub net_hist: HashMap<String, (VecDeque<f32>, VecDeque<f32>)>,
    pub gpu_hist: HashMap<usize, VecDeque<f32>>,
    pub pressure_hist: VecDeque<f32>,
    pub perf_dev: String,
    pub query: String,
    pub sel: Option<String>,
    pub expanded: HashSet<String>,
    pub menu: Option<Menu>,
    pub sheet: Option<Sheet>,
    pub cursor: Point,
    pub lists: HashMap<&'static str, Value>,
    pub prefs: Map<String, Value>,
    pub ending: Vec<i32>,
    pub note: String,
}

pub fn prefs_path() -> std::path::PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| hearth_ui::paths::home().join(".config"))
        .join("nimbus/taskmgr.json")
}

pub fn tasks_helper() -> String {
    std::env::var("HEARTH_TASKS_HELPER").unwrap_or_else(|_| run::shell_helper("hearth-tasks"))
}

fn push(h: &mut VecDeque<f32>, v: f32) {
    if h.len() >= HISTORY {
        h.pop_front();
    }
    h.push_back(v);
}

impl App {
    fn new() -> (App, Task<Msg>) {
        let prefs = settings_file::read(&prefs_path()).map().unwrap_or_default();
        let pref_s = |k: &str| prefs.get(k).and_then(Value::as_str).unwrap_or("").to_string();
        let start = pref_s("startPage");
        let mut page = if start == "last" { Page::from_id(&pref_s("lastPage")) } else { Page::from_id(&start) }.unwrap_or(Page::Processes);
        let mut perf_dev = { let d = pref_s("perfDevice"); if d.is_empty() { "cpu".to_string() } else { d } };
        if let Some(arg) = app::arg("--page") {
            if let Some(pg) = Page::from_id(&arg) {
                page = pg;
                if let Some((_, dev)) = arg.split_once(':') {
                    perf_dev = dev.to_string();
                }
            }
        }
        let a = App {
            p: hearth_ui::theme::load(),
            page,
            sample: None,
            cpu_hist: VecDeque::new(),
            core_hist: Vec::new(),
            mem_hist: VecDeque::new(),
            zram_hist: VecDeque::new(),
            disk_hist: HashMap::new(),
            net_hist: HashMap::new(),
            gpu_hist: HashMap::new(),
            pressure_hist: VecDeque::new(),
            perf_dev,
            query: app::arg("--search").unwrap_or_default(),
            sel: None,
            expanded: HashSet::new(),
            menu: None,
            sheet: None,
            cursor: Point::ORIGIN,
            lists: HashMap::new(),
            prefs,
            ending: Vec::new(),
            note: String::new(),
        };
        sample::INTERVAL_MS.store(a.interval(), Ordering::Relaxed);
        sample::WANT.store(a.page.want(), Ordering::Relaxed);
        let t = a.load_list();
        (a, t)
    }

    pub fn pref_s(&self, k: &str, d: &str) -> String {
        self.prefs.get(k).and_then(Value::as_str).map(String::from).unwrap_or_else(|| d.to_string())
    }
    pub fn pref_b(&self, k: &str) -> bool {
        self.prefs.get(k).and_then(Value::as_bool).unwrap_or(false)
    }
    fn interval(&self) -> u64 {
        match self.pref_s("speed", "normal").as_str() {
            "high" => 500,
            "low" => 4000,
            "paused" => 0,
            _ => 1000,
        }
    }
    /// Only a change the person made is written, and only that key, merged into the latest file (red flag 9).
    fn set_pref(&mut self, k: &str, v: Value) {
        if self.prefs.get(k) == Some(&v) {
            return;
        }
        self.prefs.insert(k.to_string(), v.clone());
        let key = k.to_string();
        let _ = settings_file::update_json(&prefs_path(), move |m| {
            m.insert(key, v);
        });
    }
    pub fn sort_of(&self, table: &str) -> (String, bool) {
        match self.prefs.get(&format!("sort_{table}")) {
            Some(Value::Object(o)) => (o.get("key").and_then(Value::as_str).unwrap_or("").to_string(), o.get("desc").and_then(Value::as_bool).unwrap_or(false)),
            _ => (String::new(), false),
        }
    }
    fn load_list(&self) -> Task<Msg> {
        match self.page.list() {
            Some(l) => Task::perform(run::spawn(vec![tasks_helper(), l.to_string()]), move |o| Msg::List(l, o)),
            None => Task::none(),
        }
    }
    fn act(&mut self, what: &str, args: Vec<String>) -> Task<Msg> {
        let mut cmd = vec![tasks_helper()];
        cmd.extend(args);
        let w = what.to_string();
        Task::perform(run::spawn(cmd), move |o| Msg::Acted(w.clone(), o))
    }
    fn notice(&mut self, title: &str, text: &str) {
        self.sheet = Some(Sheet { kind: SheetKind::Notice, title: title.into(), text: text.into(), input: String::new(), target: Target::default() });
    }
    pub fn group(&self, gid: &str) -> Option<&sample::Group> {
        self.sample.as_ref().and_then(|s| s.groups.iter().find(|g| g.id == gid))
    }

    fn take_sample(&mut self, s: Arc<Sample>) {
        push(&mut self.cpu_hist, s.cpu.total);
        if self.core_hist.len() != s.cpu.cores.len() {
            self.core_hist = vec![VecDeque::new(); s.cpu.cores.len()];
        }
        for (i, c) in s.cpu.cores.iter().enumerate() {
            push(&mut self.core_hist[i], *c);
        }
        let total = s.mem.total.max(1) as f32;
        push(&mut self.mem_hist, 100.0 * s.mem.used as f32 / total);
        push(&mut self.zram_hist, 100.0 * s.mem.zram_used as f32 / total);
        push(&mut self.pressure_hist, s.mem.pressure);
        for d in &s.disks {
            push(self.disk_hist.entry(d.name.clone()).or_default(), d.active);
        }
        for n in &s.nets {
            let e = self.net_hist.entry(n.name.clone()).or_default();
            push(&mut e.0, n.recv as f32 * 8.0 / 1e6);
            push(&mut e.1, n.send as f32 * 8.0 / 1e6);
        }
        for g in &s.gpus {
            push(self.gpu_hist.entry(g.index).or_default(), g.busy.unwrap_or(0.0));
        }
        self.sample = Some(s);
    }

    fn set_page(&mut self, p: Page) -> Task<Msg> {
        self.page = p;
        self.menu = None;
        self.sel = None;
        sample::WANT.store(p.want(), Ordering::Relaxed);
        sample::wake();
        self.set_pref("lastPage", Value::String(p.id().into()));
        self.load_list()
    }

    // ---------------------------------------------------------------- the right-click menus
    fn proc_menu(&mut self, key: &str) {
        let Some(s) = self.sample.clone() else { return };
        let (gid, child_pid) = match key.split_once('/') {
            Some((g, c)) => (g.to_string(), c.parse::<i32>().ok()),
            None => (key.to_string(), None),
        };
        let all_view = key.starts_with("a/");
        let (gid, child_pid) = if all_view {
            let pid = key[2..].parse::<i32>().unwrap_or(0);
            (s.groups.iter().find(|g| g.pids.contains(&pid)).map(|g| g.id.clone()).unwrap_or_default(), Some(pid))
        } else {
            (gid, child_pid)
        };
        let Some(g) = s.groups.iter().find(|g| g.id == gid) else { return };
        let child = child_pid.is_some();
        let pid = child_pid.unwrap_or(g.pid);
        let cname = g.children.iter().find(|k| Some(k.pid) == child_pid).map(|k| k.name.clone()).unwrap_or_default();
        let proc_name = if child { cname.clone() } else if !g.exe.is_empty() { apps::basename(&g.exe) } else { g.name.clone() };
        let ok = !g.protected;
        let mut items = Vec::new();
        if !child && g.children.len() > 1 {
            items.push(MenuItem::new(if self.expanded.contains(&g.id) { "Collapse" } else { "Expand" }, Act::Expand(g.id.clone())));
        }
        items.push(MenuItem::new("End task", Act::End).on(ok).key("Del".into()));
        items.push(MenuItem::new("End process tree", Act::Tree).on(ok));
        items.push(MenuItem::new("Efficiency mode", Act::Eco(!g.eco)).on(ok).checked(g.eco));
        items.push(MenuItem::new("Set priority ▸", Act::PriorityMenu).on(ok));
        items.push(MenuItem::new("Run on these cores…", Act::Cores).on(ok).fresh());
        items.push(MenuItem::new("Limit CPU / memory…", Act::Limit).on(ok && !child).fresh());
        items.push(MenuItem::sep());
        if g.suspended && !child {
            items.push(MenuItem::new("Resume", Act::Resume).on(ok));
        }
        items.push(MenuItem::new("Keep this running (checkpoint)", Act::Keep(true)).on(ok && !proc_name.is_empty()).fresh());
        items.push(MenuItem::new("Never close when memory runs out", Act::Never).on(ok && !proc_name.is_empty()).fresh());
        items.push(MenuItem::sep());
        items.push(MenuItem::new("Go to details", Act::Details).key(format!("PID {pid}")));
        items.push(MenuItem::new("Open file location", Act::Where).on(!g.exe.is_empty()));
        items.push(MenuItem::new("Search online", Act::Web));
        items.push(MenuItem::sep());
        items.push(MenuItem::new("Properties", Act::Props).on(!g.exe.is_empty()));
        let target = Target { gid: g.id.clone(), pid, name: if child { cname } else { g.name.clone() }, path: g.exe.clone(), child, protected: g.protected, unit: proc_name, list_id: String::new() };
        self.menu = Some(Menu { title: target.name.clone(), items, at: self.cursor, target, sel: 0 });
    }

    fn details_menu(&mut self, pid: i32) {
        let Some(s) = self.sample.clone() else { return };
        let Some(r) = s.details.iter().find(|r| r.pid == pid) else { return };
        let ok = r.mine;
        let suspended = r.status == "Suspended";
        let items = vec![
            MenuItem::new("End task", Act::End).on(ok).key("Del".into()),
            MenuItem::new("End process tree", Act::Tree).on(ok),
            MenuItem::new("Set priority ▸", Act::PriorityMenu).on(ok),
            MenuItem::new("Set affinity", Act::Cores).on(ok),
            MenuItem::new(if suspended { "Resume" } else { "Suspend" }, Act::Suspend(!suspended)).on(ok),
            MenuItem::sep(),
            MenuItem::new("Create dump file", Act::Dump).on(ok),
            MenuItem::new("Open file location", Act::Where).on(!r.path.is_empty()),
            MenuItem::new("Search online", Act::Web),
            MenuItem::new("Properties", Act::Props).on(!r.path.is_empty()),
        ];
        let target = Target { pid, name: r.name.clone(), path: r.path.clone(), child: true, protected: !ok, ..Target::default() };
        self.menu = Some(Menu { title: format!("{} · {}", r.name, r.pid), items, at: self.cursor, target, sel: 0 });
    }

    fn list_menu(&mut self, list: &'static str, id: &str) {
        let Some(v) = self.lists.get(list) else { return };
        let key = match list {
            "startup" => "items",
            "services" => "services",
            "timers" => "timers",
            _ => return,
        };
        let Some(row) = v.get(key).and_then(Value::as_array).and_then(|a| a.iter().find(|x| x.get("id").and_then(Value::as_str) == Some(id))) else { return };
        let can = row.get("canChange").and_then(Value::as_bool).unwrap_or(false);
        let name = row.get("name").and_then(Value::as_str).or_else(|| row.get("unit").and_then(Value::as_str)).unwrap_or("").to_string();
        let unit = row.get("unit").and_then(Value::as_str).unwrap_or("").to_string();
        let items = match list {
            "startup" => {
                let on = row.get("on").and_then(Value::as_bool).unwrap_or(false);
                vec![MenuItem::new(if on { "Disable" } else { "Enable" }, Act::StartupSet(!on)).on(can), MenuItem::new("Search online", Act::Web)]
            }
            "services" => {
                let running = row.get("status").and_then(Value::as_str) == Some("Running");
                vec![
                    MenuItem::new("Start", Act::Service("start")).on(can && !running),
                    MenuItem::new("Stop", Act::Service("stop")).on(can && running),
                    MenuItem::new("Restart", Act::Service("restart")).on(can && running),
                    MenuItem::sep(),
                    MenuItem::new("Search online", Act::Web),
                ]
            }
            _ => {
                let on = !row.get("next").map(|x| x.is_null() || x.as_str() == Some("")).unwrap_or(true);
                vec![MenuItem::new(if on { "Turn off" } else { "Turn on" }, Act::TimerSet(!on)).on(can), MenuItem::new("Search online", Act::Web)]
            }
        };
        let target = Target { name: name.clone(), unit, list_id: id.to_string(), ..Target::default() };
        self.menu = Some(Menu { title: name, items, at: self.cursor, target, sel: 0 });
    }

    fn do_act(&mut self, act: Act, t: Target) -> Task<Msg> {
        self.menu = None;
        let pid = t.pid.to_string();
        match act {
            Act::Expand(gid) => {
                if !self.expanded.remove(&gid) {
                    self.expanded.insert(gid);
                }
                Task::none()
            }
            Act::End => {
                if t.protected {
                    let why = self.group(&t.gid).map(|g| g.why.clone()).unwrap_or_else(|| "This".into());
                    self.notice(&t.name, &format!("{why} keeps Hearth running, so it can't be ended here."));
                    return Task::none();
                }
                let (cmd, pids) = if t.child || self.page == Page::Details {
                    ("end-pid", vec![t.pid])
                } else {
                    ("end", self.group(&t.gid).map(|g| g.pids.clone()).unwrap_or_else(|| vec![t.pid]))
                };
                self.ending = pids;
                self.sheet = Some(Sheet { kind: SheetKind::Ending, title: t.name.clone(), text: format!("Asking {} to close…", t.name), input: String::new(), target: t.clone() });
                Task::batch([self.act("end", vec![cmd.into(), pid]), Task::perform(run::after(3000), |_| Msg::EndCheck)])
            }
            Act::Tree => self.act("tree", vec!["end-tree".into(), pid]),
            Act::Eco(on) => self.act("eco", vec!["eco".into(), pid, if on { "on" } else { "off" }.into()]),
            Act::PriorityMenu => {
                let items = vec![
                    MenuItem::new("Realtime", Act::Priority("realtime")).on(false),
                    MenuItem::new("High", Act::Priority("high")).on(false),
                    MenuItem::new("Above normal", Act::Priority("above")).on(false),
                    MenuItem::new("Normal", Act::Priority("normal")),
                    MenuItem::new("Below normal", Act::Priority("below")),
                    MenuItem::new("Low", Act::Priority("low")),
                ];
                self.menu = Some(Menu { title: format!("Set priority · {}", t.name), items, at: Point::new(self.cursor.x + 24.0, self.cursor.y), target: t, sel: 3 });
                Task::none()
            }
            Act::Priority(level) => self.act("priority", vec!["priority".into(), pid, level.into()]),
            Act::Cores => {
                let n = self.sample.as_ref().map(|s| s.cpu.logical).unwrap_or(1);
                self.sheet = Some(Sheet {
                    kind: SheetKind::Ask("cores"),
                    title: "Run on these cores".into(),
                    text: format!("Which of this PC's cores {} may use, until it closes (cores 0 to {}). For example 0-3 or 0,2,4.", t.name, n - 1),
                    input: format!("0-{}", n - 1),
                    target: t,
                });
                Task::none()
            }
            Act::Limit => {
                self.sheet = Some(Sheet {
                    kind: SheetKind::Ask("limit"),
                    title: "Limit CPU / memory".into(),
                    text: format!("A cap for {} until it closes: CPU as a percent of one core, then memory in MB (0 = no limit). It's slowed down, never closed. For example: 200 4096", t.name),
                    input: "200 0".into(),
                    target: t,
                });
                Task::none()
            }
            Act::Resume => self.act("resume", vec!["resume-group".into(), pid]),
            Act::Suspend(on) => self.act("suspend", vec![if on { "suspend" } else { "resume" }.into(), pid]),
            Act::Keep(_) => {
                run::detached(&["pkexec", "/usr/libexec/nimbus/hearth-memory", "checkpoint", &t.unit, "on"]);
                self.notice(&t.name, &format!("When memory runs out and {} is the one using it, Hearth pauses it instead of closing it: it's frozen and its memory is moved to the drive, so nothing is lost. Right-click it and choose Resume to carry on where you were. Press this again to take it off the list.", t.name));
                Task::none()
            }
            Act::Never => {
                run::detached(&["pkexec", "/usr/libexec/nimbus/hearth-memory", "never", &t.unit, "on"]);
                self.notice(&t.name, &format!("{} is on your Never close list: when memory runs out, Hearth closes something else. Change the list in Memory › Swap & settings.", t.unit));
                Task::none()
            }
            Act::Details => {
                self.query = t.pid.to_string();
                self.set_page(Page::Details)
            }
            Act::Where => {
                let dir = std::path::Path::new(&t.path).parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|| "/".into());
                run::detached(&["xdg-open", &dir]);
                Task::none()
            }
            Act::Web => {
                let q: String = t.name.chars().map(|c| if c.is_ascii_alphanumeric() { c.to_string() } else { format!("%{:02X}", c as u32 & 0xff) }).collect();
                run::detached(&["xdg-open", &format!("https://duckduckgo.com/?q={q}")]);
                Task::none()
            }
            Act::Props => {
                run::detached(&["xdg-open", &std::path::Path::new(&t.path).parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default()]);
                Task::none()
            }
            Act::Dump => {
                let home = hearth_ui::paths::home();
                let out = home.join(format!("{}-{}.core", t.name, t.pid)).to_string_lossy().to_string();
                self.notice(&t.name, &format!("A dump of {} is being written to {out} (gcore).", t.name));
                Task::perform(run::spawn(vec!["gcore".into(), "-o".into(), out.trim_end_matches(&format!(".{}", t.pid)).to_string(), pid]), |o| Msg::Acted("dump".into(), o))
            }
            Act::Service(what) => self.act("service", vec!["service".into(), t.unit.clone(), what.into()]),
            Act::StartupSet(on) => self.act("startup-set", vec!["startup-set".into(), t.list_id.clone(), if on { "on" } else { "off" }.into()]),
            Act::TimerSet(on) => self.act("timer", vec!["timer".into(), t.unit.clone(), if on { "on" } else { "off" }.into()]),
        }
    }

    /// The selected row's target (the buttons at the top of Processes and Details).
    fn selected_target(&self) -> Option<Target> {
        let key = self.sel.clone()?;
        let s = self.sample.as_ref()?;
        if self.page == Page::Details {
            let pid: i32 = key.parse().ok()?;
            let r = s.details.iter().find(|r| r.pid == pid)?;
            return Some(Target { pid, name: r.name.clone(), path: r.path.clone(), child: true, protected: !r.mine, ..Target::default() });
        }
        if let Some(rest) = key.strip_prefix("a/") {
            let pid: i32 = rest.parse().ok()?;
            let g = s.groups.iter().find(|g| g.pids.contains(&pid))?;
            return Some(Target { gid: g.id.clone(), pid, name: g.name.clone(), path: g.exe.clone(), child: true, protected: g.protected, ..Target::default() });
        }
        let gid = key.split('/').next()?;
        let g = s.groups.iter().find(|g| g.id == gid)?;
        Some(Target { gid: g.id.clone(), pid: g.pid, name: g.name.clone(), path: g.exe.clone(), child: false, protected: g.protected, ..Target::default() })
    }
}

fn update(a: &mut App, m: Msg) -> Task<Msg> {
    match m {
        Msg::Theme => a.p = hearth_ui::theme::load(),
        Msg::Sample(s) => {
            a.take_sample(s);
            // CI's pictures: --show menu opens the first app's right-click menu once the list is in.
            if a.menu.is_none() && a.cpu_hist.len() == 2 && app::arg("--show").as_deref() == Some("menu") {
                if let Some(g) = a.sample.as_ref().and_then(|s| s.groups.first().map(|g| g.id.clone())) {
                    a.cursor = Point::new(520.0, 150.0);
                    a.sel = Some(g.clone());
                    a.proc_menu(&g);
                }
            }
            if let Some(sh) = &a.sheet {
                if sh.kind == SheetKind::Ending && !a.ending.is_empty() {
                    let alive = a.sample.as_ref().map(|s| s.groups.iter().any(|g| g.pids.iter().any(|p| a.ending.contains(p))) || s.details.iter().any(|r| a.ending.contains(&r.pid))).unwrap_or(false);
                    if !alive && a.sample.as_ref().map(|s| s.has_procs || s.has_details).unwrap_or(false) {
                        a.sheet = None;
                        a.ending.clear();
                    }
                }
            }
        }
        Msg::Tick => return a.load_list(),
        Msg::List(l, o) => {
            if let Some(v) = o.json() {
                a.lists.insert(l, v);
            } else if !o.ok {
                a.note = format!("Couldn't read the list: {}", o.why());
            }
        }
        Msg::Acted(what, o) => {
            if !o.ok {
                let why = o.why();
                a.notice("Task Manager", &format!("That didn't work{}", if why.is_empty() { ".".to_string() } else { format!(": {why}") }));
            } else if what != "end" {
                sample::wake();
            }
            if matches!(what.as_str(), "service" | "startup-set" | "timer") {
                return a.load_list();
            }
        }
        Msg::Page(p) => return a.set_page(p),
        Msg::PerfDevice(d) => {
            a.perf_dev = d.clone();
            a.set_pref("perfDevice", Value::String(d));
        }
        Msg::Query(q) => a.query = q,
        Msg::ProcView(v) => a.set_pref("procView", Value::String(v.into())),
        Msg::Sort(table, key, def_desc) => {
            let (k, d) = a.sort_of(table);
            let desc = if k == key { !d } else { def_desc };
            a.set_pref(&format!("sort_{table}"), serde_json::json!({"key": key, "desc": desc}));
        }
        Msg::Select(k) => {
            a.sel = Some(k);
            a.menu = None;
        }
        Msg::Toggle(gid) => {
            if !a.expanded.remove(&gid) {
                a.expanded.insert(gid);
            }
        }
        Msg::RowMenu(k) => {
            a.sel = Some(k.clone());
            match a.page {
                Page::Processes => a.proc_menu(&k),
                Page::Details => {
                    if let Ok(pid) = k.parse() {
                        a.details_menu(pid)
                    }
                }
                Page::Startup => a.list_menu("startup", &k),
                Page::Services => a.list_menu("services", &k),
                Page::Tasks => a.list_menu("timers", &k),
                _ => {}
            }
        }
        Msg::Cursor(pt) => a.cursor = pt,
        Msg::MenuPick(i) => {
            if let Some(menu) = a.menu.clone() {
                if let Some(it) = menu.items.get(i) {
                    if let (Some(act), true) = (it.act.clone(), it.enabled) {
                        return a.do_act(act, menu.target.clone());
                    }
                }
            }
        }
        Msg::CloseMenu => a.menu = None,
        Msg::TopEnd => {
            if let Some(t) = a.selected_target() {
                return a.do_act(Act::End, t);
            }
        }
        Msg::TopEco => {
            if let Some(t) = a.selected_target() {
                let on = !a.group(&t.gid).map(|g| g.eco).unwrap_or(false);
                return a.do_act(Act::Eco(on), t);
            }
        }
        Msg::TopWhere => {
            if let Some(t) = a.selected_target() {
                return a.do_act(Act::Where, t);
            }
        }
        Msg::SheetInput(s) => {
            if let Some(sh) = a.sheet.as_mut() {
                sh.input = s;
            }
        }
        Msg::SheetOk => {
            if let Some(sh) = a.sheet.take() {
                let pid = sh.target.pid.to_string();
                match sh.kind {
                    SheetKind::Ask("cores") => return a.act("affinity", vec!["affinity".into(), pid, sh.input.split_whitespace().collect::<String>()]),
                    SheetKind::Ask("limit") => {
                        let mut it = sh.input.split(|c: char| c.is_whitespace() || c == ',').filter(|x| !x.is_empty());
                        let cpu = it.next().unwrap_or("0").to_string();
                        let mem = it.next().unwrap_or("0").to_string();
                        return a.act("limit", vec!["limit".into(), pid, cpu, mem]);
                    }
                    SheetKind::Run => {
                        let words: Vec<String> = sh.input.split_whitespace().map(String::from).collect();
                        if let Some((p, rest)) = words.split_first() {
                            let rest: Vec<&str> = rest.iter().map(String::as_str).collect();
                            let mut all = vec![p.as_str()];
                            all.extend(rest);
                            run::detached(&all);
                        }
                    }
                    _ => {}
                }
            }
        }
        Msg::SheetForce => {
            if let Some(sh) = a.sheet.take() {
                let pids = std::mem::take(&mut a.ending);
                let tasks: Vec<Task<Msg>> = if sh.target.child || pids.len() <= 1 {
                    vec![a.act("kill", vec!["kill-pid".into(), sh.target.pid.to_string()])]
                } else {
                    vec![a.act("kill", vec!["kill".into(), sh.target.pid.to_string()])]
                };
                return Task::batch(tasks);
            }
        }
        Msg::SheetWait => {
            if let Some(sh) = a.sheet.as_mut() {
                sh.kind = SheetKind::Ending;
            }
            return Task::perform(run::after(3000), |_| Msg::EndCheck);
        }
        Msg::EndCheck => {
            if let Some(sh) = a.sheet.as_mut() {
                if sh.kind == SheetKind::Ending {
                    sh.kind = SheetKind::Stuck;
                    sh.text = format!("{} hasn't closed yet. You can wait a little longer, or force it to close (anything not saved will be lost).", sh.title);
                }
            }
        }
        Msg::CloseSheet => a.sheet = None,
        Msg::RunNew => {
            a.sheet = Some(Sheet { kind: SheetKind::Run, title: "Run new task".into(), text: "Type the name of a program, and Hearth will start it for you.".into(), input: String::new(), target: Target::default() })
        }
        Msg::ListAct(list, id, what) => {
            let Some(v) = a.lists.get(list).cloned() else { return Task::none() };
            let key = match list {
                "startup" => "items",
                "services" => "services",
                _ => "timers",
            };
            let row = v.get(key).and_then(Value::as_array).and_then(|x| x.iter().find(|r| r.get("id").and_then(Value::as_str) == Some(id.as_str())).cloned());
            let Some(row) = row else { return Task::none() };
            let unit = row.get("unit").and_then(Value::as_str).unwrap_or("").to_string();
            return match (list, what.as_str()) {
                ("startup", w) => a.act("startup-set", vec!["startup-set".into(), id, if w == "on" { "on" } else { "off" }.into()]),
                ("services", w) => a.act("service", vec!["service".into(), unit, w.to_string()]),
                (_, w) => a.act("timer", vec!["timer".into(), unit, if w == "on" { "on" } else { "off" }.into()]),
            };
        }
        Msg::Pref(k, v) => {
            a.set_pref(k, v);
            if k == "speed" {
                sample::INTERVAL_MS.store(a.interval(), Ordering::Relaxed);
                sample::wake();
            }
            if k == "onTop" {
                let on = a.pref_b("onTop");
                return iced::window::latest().and_then(move |id| iced::window::set_level(id, if on { iced::window::Level::AlwaysOnTop } else { iced::window::Level::Normal }));
            }
        }
        Msg::ClearHistory => {
            return Task::perform(run::spawn(vec![tasks_helper(), "history-clear".into()]), |_| Msg::Tick);
        }
        Msg::Key(ev) => return key(a, ev),
        Msg::Drag => return win::drag(),
        Msg::Maximize => return win::toggle_maximize(),
        Msg::Minimize => return win::minimize(),
        Msg::Close => return win::close(),
        Msg::Resize(d) => return win::resize(d),
    }
    Task::none()
}

fn key(a: &mut App, ev: KEvent) -> Task<Msg> {
    let KEvent::KeyPressed { key, modifiers, .. } = ev else { return Task::none() };
    if let Some(m) = a.menu.as_mut() {
        if let Key::Named(k) = key.as_ref() {
            let n = m.items.len();
            match k {
                Named::Escape => a.menu = None,
                Named::ArrowDown => {
                    let mut i = m.sel;
                    for _ in 0..n {
                        i = (i + 1) % n.max(1);
                        if m.items[i].act.is_some() {
                            break;
                        }
                    }
                    m.sel = i;
                }
                Named::ArrowUp => {
                    let mut i = m.sel;
                    for _ in 0..n {
                        i = (i + n - 1) % n.max(1);
                        if m.items[i].act.is_some() {
                            break;
                        }
                    }
                    m.sel = i;
                }
                Named::Enter | Named::Space => {
                    let i = m.sel;
                    return update(a, Msg::MenuPick(i));
                }
                _ => {}
            }
        }
        return Task::none();
    }
    if a.sheet.is_some() {
        if let Key::Named(Named::Escape) = key.as_ref() {
            a.sheet = None;
        }
        return Task::none();
    }
    match key.as_ref() {
        Key::Named(Named::Escape) => {
            if a.sel.is_some() {
                a.sel = None;
                Task::none()
            } else {
                win::close()
            }
        }
        Key::Named(Named::Delete) => update(a, Msg::TopEnd),
        Key::Named(Named::ContextMenu) => {
            if let Some(k) = a.sel.clone() {
                return update(a, Msg::RowMenu(k));
            }
            Task::none()
        }
        Key::Named(Named::ArrowDown) | Key::Named(Named::ArrowUp) if modifiers.control() => {
            let i = Page::ALL.iter().position(|p| *p == a.page).unwrap_or(0);
            let down = matches!(key.as_ref(), Key::Named(Named::ArrowDown));
            let j = if down { (i + 1).min(Page::ALL.len() - 1) } else { i.saturating_sub(1) };
            a.set_page(Page::ALL[j])
        }
        Key::Named(Named::ArrowDown) | Key::Named(Named::ArrowUp) => {
            let down = matches!(key.as_ref(), Key::Named(Named::ArrowDown));
            let keys = pages::row_keys(a);
            if keys.is_empty() {
                return Task::none();
            }
            let cur = a.sel.as_ref().and_then(|k| keys.iter().position(|x| x == k));
            let j = match cur {
                Some(i) if down => (i + 1).min(keys.len() - 1),
                Some(i) => i.saturating_sub(1),
                None => 0,
            };
            a.sel = Some(keys[j].clone());
            Task::none()
        }
        Key::Named(Named::ArrowRight) | Key::Named(Named::ArrowLeft) if a.page == Page::Processes => {
            if let Some(k) = a.sel.clone() {
                if !k.contains('/') {
                    if matches!(key.as_ref(), Key::Named(Named::ArrowRight)) {
                        a.expanded.insert(k);
                    } else {
                        a.expanded.remove(&k);
                    }
                }
            }
            Task::none()
        }
        _ => Task::none(),
    }
}

fn view(a: &App) -> Element<'_, Msg> {
    pages::view(a)
}

fn subscription(a: &App) -> Subscription<Msg> {
    let mut subs = vec![
        Subscription::run(hearth_ui::theme::watch).map(|_| Msg::Theme),
        Subscription::run(sample::stream).map(Msg::Sample),
        iced::keyboard::listen().map(Msg::Key),
    ];
    // The lists from hearth-tasks (services, start-up apps…) are read again every 5 s while their page is open.
    if a.page.list().is_some() && a.interval() > 0 {
        subs.push(iced::time::every(std::time::Duration::from_secs(5)).map(|_| Msg::Tick));
    }
    Subscription::batch(subs)
}

fn main() -> iced::Result {
    if app::handle_cli(
        "hearth-task-manager",
        "Task Manager: what's running, performance, start-up apps, users, services and scheduled tasks.\n  --page processes|performance[:cpu|memory|disk:NAME|net:NAME|gpu:N|battery]|history|startup|users|details|services|tasks|settings\n  --search TEXT",
    ) {
        return Ok(());
    }
    iced::application(App::new, update, view)
        .title("Task Manager")
        .subscription(subscription)
        .theme(|a: &App| app::iced_theme(&a.p))
        .window(app::window("hearth-tasks", (1180.0, 760.0), (820.0, 520.0)))
        .default_font(app::INTER)
        .run()
}
