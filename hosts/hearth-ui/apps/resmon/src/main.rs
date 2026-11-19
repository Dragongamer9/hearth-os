//! Resource Monitor, in pure Rust: the Fedora edition's Resource Monitor (on Debian it's Mantel's
//! ResourceMonitorView.qml, which runs only inside Mantel). Mantel's page ported, to the approved mockups
//! resmon-1 … resmon-6 (docs/mockups/checklists/resource-monitor.md): the tabs Your PC, Overview, Processor, Memory,
//! Disk, Network and Graphics; per-program tables you can tick, sort and right-click (End task asks first, Efficiency
//! mode, Open file location, Search online); foldable sections; the live graphs on the right.
//!
//! The same helpers and files as Mantel's: `hearth-resmon serve` for the numbers (feed.rs), `hearth-resmon ports |
//! drives | writing | services | libs | files | pc` for the rest, `hearth-resmon end | eco` (Task Manager's own
//! hearth-tasks, with its checks: your own programs only, never a part of Hearth). It writes no setting at all, like
//! Mantel's (the helper keeps only caches in ~/.cache/hearth). Live only while the window is seen (feed.rs).
//!
//! Mouse, keyboard and the controller (hearth-pad's keys: D-pad arrows, A Enter, B Esc, X the Menu key, LB / RB
//! Ctrl+Page Up / Down): LB / RB switch tabs; Up / Down pick a program, Space ticks it, X (or a right-click) opens
//! its actions, Delete ends it (asks first); on Your PC the D-pad moves between parts, A shows a part, X runs its
//! button; B closes the menu or the dialog, then the window.

mod board;
mod feed;
mod graph;
mod view;

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use hearth_ui::app::{self, win};
use hearth_ui::run;
use hearth_ui::Pal;
use iced::keyboard::{key::Named, Event as KEvent, Key};
use iced::{Element, Point, Subscription, Task};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tab {
    Pc,
    Overview,
    Cpu,
    Memory,
    Disk,
    Network,
    Gpu,
}

impl Tab {
    pub const ALL: [Tab; 7] = [Tab::Pc, Tab::Overview, Tab::Cpu, Tab::Memory, Tab::Disk, Tab::Network, Tab::Gpu];
    pub fn id(self) -> &'static str {
        match self {
            Tab::Pc => "pc",
            Tab::Overview => "overview",
            Tab::Cpu => "cpu",
            Tab::Memory => "memory",
            Tab::Disk => "disk",
            Tab::Network => "network",
            Tab::Gpu => "gpu",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Tab::Pc => "Your PC",
            Tab::Overview => "Overview",
            Tab::Cpu => "Processor",
            Tab::Memory => "Memory",
            Tab::Disk => "Disk",
            Tab::Network => "Network",
            Tab::Gpu => "Graphics",
        }
    }
    pub fn from_id(s: &str) -> Option<Tab> {
        let s = match s {
            "processor" => "cpu",
            "graphics" => "gpu",
            "net" => "network",
            x => x,
        };
        Tab::ALL.iter().copied().find(|t| t.id() == s)
    }
}

/// A program row's own facts (for its menu and its tick).
#[derive(Debug, Clone, Default)]
pub struct Prog {
    pub gid: String,
    pub name: String,
    pub pid: i64,
    pub pids: Vec<i64>,
    pub exe: String,
    pub eco: bool,
    pub protected: bool,
    pub why: String,
}

#[derive(Debug, Clone)]
pub struct Menu {
    pub prog: Prog,
    pub at: Point,
    pub sel: usize,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Theme,
    Feed(feed::Ev),
    Tab(Tab),
    Fold(String),
    Tick(String),
    Pick(String, usize, String),
    RowMenu(String, usize, String),
    Cursor(Point),
    MenuPick(usize),
    CloseMenu,
    EndYes,
    EndNo,
    Sort(String, usize),
    Pause,
    OpenTm,
    Out(&'static str, run::Output),
    Acted(String, run::Output),
    Every(&'static str),
    FilesQuery(String),
    Part(String),
    PartAction,
    ReadPcAgain,
    ToastDone(u64),
    Key(KEvent),
    Drag,
    Maximize,
    Minimize,
    Close,
    Resize(iced::window::Direction),
}

pub struct App {
    pub p: Pal,
    pub tab: Tab,
    pub sample: Option<Arc<Value>>,
    pub samples: u64,
    pub hist: HashMap<String, VecDeque<f32>>,
    pub paused: bool,
    pub hidden: bool,
    pub err: String,
    pub folded: HashSet<String>,
    pub ticked: HashSet<String>,
    /// The picked row: (table, index, group id).
    pub pick: (String, usize, String),
    pub sort: HashMap<String, (usize, bool)>,
    pub menu: Option<Menu>,
    pub ask_end: Option<Prog>,
    pub toast: String,
    pub toast_n: u64,
    pub cursor: Point,
    pub busy: HashSet<&'static str>,
    // what the tabs read besides the sampler
    pub ports: Value,
    pub drives: Vec<Value>,
    pub files_busy: Vec<Value>,
    pub busy_names: HashMap<i64, String>,
    pub services: Vec<Value>,
    pub libs: Vec<Value>,
    pub open_files: Vec<Value>,
    pub files_query: String,
    pub health: Value,
    pub temp_limit: Option<f64>,
    pub ts_peers: i64,
    pub ssid: HashMap<String, String>,
    pub focus_seen: String,
    // Your PC
    pub pc: Option<Value>,
    pub pc_view: Option<Value>,
    pub pc_err: String,
    pub part: String,
    pub part_cursor: String,
    pub have_disks: String,
}

pub fn push(h: &mut HashMap<String, VecDeque<f32>>, k: &str, v: f64) {
    let q = h.entry(k.to_string()).or_default();
    if q.len() >= graph::HIST {
        q.pop_front();
    }
    q.push_back(v.max(0.0) as f32);
}

// ------------------------------------------------------------------ JSON in words
pub fn f(v: &Value, k: &str) -> f64 {
    v.get(k).and_then(Value::as_f64).unwrap_or(0.0)
}
pub fn fo(v: &Value, k: &str) -> Option<f64> {
    v.get(k).and_then(Value::as_f64)
}
pub fn s<'a>(v: &'a Value, k: &str) -> &'a str {
    v.get(k).and_then(Value::as_str).unwrap_or("")
}
pub fn arr<'a>(v: &'a Value, k: &str) -> &'a [Value] {
    v.get(k).and_then(Value::as_array).map(|a| a.as_slice()).unwrap_or(&[])
}

impl App {
    fn new() -> (App, Task<Msg>) {
        let tab = app::arg("--page").and_then(|t| Tab::from_id(&t)).or_else(|| std::env::args().nth(1).and_then(|t| Tab::from_id(&t))).unwrap_or(Tab::Pc);
        let mut folded: HashSet<String> = ["ov:disk", "ov:network", "ov:gpu", "cpu:libs"].iter().map(|s| s.to_string()).collect();
        if let Some(open) = app::arg("--unfold") {
            for k in open.split(',') {
                folded.remove(k);
            }
        }
        let mut a = App {
            p: hearth_ui::theme::load(),
            tab,
            sample: None,
            samples: 0,
            hist: HashMap::new(),
            paused: false,
            hidden: false,
            err: String::new(),
            folded,
            ticked: HashSet::new(),
            pick: (String::new(), 0, String::new()),
            sort: HashMap::new(),
            menu: None,
            ask_end: None,
            toast: String::new(),
            toast_n: 0,
            cursor: Point::ORIGIN,
            busy: HashSet::new(),
            ports: Value::Null,
            drives: Vec::new(),
            files_busy: Vec::new(),
            busy_names: HashMap::new(),
            services: Vec::new(),
            libs: Vec::new(),
            open_files: Vec::new(),
            files_query: String::new(),
            health: Value::Null,
            temp_limit: None,
            ts_peers: -1,
            ssid: HashMap::new(),
            focus_seen: String::new(),
            pc: None,
            pc_view: None,
            pc_err: String::new(),
            part: String::new(),
            part_cursor: String::new(),
            have_disks: String::new(),
        };
        feed::set_want(a.wants());
        let mut tasks = vec![a.read("hwset", feed::other_cmd("hwset", "")), a.read_pc(false), a.read(
            "have",
            run::args(&["sh", "-c", "for c in hearth-disks gnome-disks kdepartitionmanager kinfocenter pavucontrol systemsettings hearth-settings-app; do command -v \"$c\" >/dev/null 2>&1 && echo \"$c\"; done"]),
        )];
        tasks.push(a.tab_reads());
        (a, Task::batch(tasks))
    }

    /// What the sampler reads for the tab on show (Mantel's wants()).
    pub fn wants(&self) -> &'static [&'static str] {
        match self.tab {
            Tab::Pc => &["procs", "gpu", "temps", "disk"],
            Tab::Overview if !self.folded.contains("ov:gpu") => &["procs", "disk", "net", "temps", "mem", "gpuprocs"],
            Tab::Overview => &["procs", "disk", "net", "temps", "mem"],
            Tab::Cpu => &["procs", "temps"],
            Tab::Memory => &["procs", "mem"],
            Tab::Disk => &["procs", "disk"],
            Tab::Network => &["procs", "net"],
            Tab::Gpu => &["procs", "gpu", "gpuprocs", "temps"],
        }
    }

    fn read(&mut self, kind: &'static str, cmd: Vec<String>) -> Task<Msg> {
        if !self.busy.insert(kind) {
            return Task::none();
        }
        Task::perform(run::spawn(cmd), move |o| Msg::Out(kind, o))
    }

    fn read_pc(&mut self, refresh: bool) -> Task<Msg> {
        let mut args: Vec<String> = vec!["pc".into()];
        if refresh {
            args.push("--refresh".into());
        }
        // CI's pictures: a test machine (hosts/linux/test/fixtures/resmon), never set on a PC
        let fx = std::env::var("HEARTH_RESMON_PC_FIXTURE").unwrap_or_default();
        if !fx.is_empty() {
            args.extend(["--fixture".to_string(), fx, "--no-helper".to_string()]);
        }
        let refs: Vec<&str> = args.iter().map(|x| x.as_str()).collect();
        let cmd = feed::helper_cmd(40, &refs);
        self.read("pc", cmd)
    }

    /// The live one: the sampler isn't standing still (not paused, the window is seen).
    pub fn live(&self) -> bool {
        !self.paused && !self.hidden
    }

    pub fn groups(&self) -> &[Value] {
        self.sample.as_deref().map(|s| arr(s, "procs")).unwrap_or(&[])
    }

    pub fn group(&self, gid: &str) -> Option<&Value> {
        self.groups().iter().find(|g| s(g, "id") == gid || g.get("id").map(|x| x.to_string()) == Some(gid.to_string()))
    }

    pub fn prog_of(&self, gid: &str) -> Option<Prog> {
        let g = self.group(gid)?;
        Some(Prog {
            gid: gid.to_string(),
            name: s(g, "name").to_string(),
            pid: f(g, "pid") as i64,
            pids: arr(g, "pids").iter().filter_map(Value::as_i64).collect(),
            exe: s(g, "exe").to_string(),
            eco: g.get("eco").and_then(Value::as_bool).unwrap_or(false),
            protected: g.get("protected").and_then(Value::as_bool).unwrap_or(false),
            why: s(g, "why").to_string(),
        })
    }

    /// The program the Processor tab looks into: the first ticked, else the picked row.
    pub fn focus(&self) -> Option<Prog> {
        if let Some(g) = self.groups().iter().find(|g| self.ticked.contains(&gid_of(g))) {
            return self.prog_of(&gid_of(g));
        }
        if !self.pick.2.is_empty() {
            return self.prog_of(&self.pick.2);
        }
        None
    }

    fn take(&mut self, d: Arc<Value>) {
        self.hidden = false;
        if self.paused {
            return;
        }
        self.err.clear();
        let h = &mut self.hist;
        let cpu = d.get("cpu").cloned().unwrap_or(Value::Null);
        push(h, "cpu", f(&cpu, "total"));
        let disks = arr(&d, "disks");
        let nets = arr(&d, "nets");
        let gpus = arr(&d, "gpus");
        push(h, "disk", disks.iter().map(|x| f(x, "read") + f(x, "write")).sum());
        push(h, "net", nets.iter().map(|x| f(x, "recv") + f(x, "send")).sum());
        let mb = d.get("memBar").cloned().unwrap_or(Value::Null);
        push(h, "mem", f(&mb, "inUse"));
        push(h, "gpu", gpus.iter().map(|g| f(g, "busy")).fold(0.0, f64::max));
        let gp = arr(&d, "procs");
        let g3d: f64 = gp.iter().map(|g| f(g, "g3d")).sum::<f64>().min(100.0);
        push(h, "g3d", if g3d > 0.0 { g3d } else { gpus.iter().map(|g| f(g, "busy")).fold(0.0, f64::max) });
        push(h, "gdec", gp.iter().map(|g| f(g, "gdec")).sum::<f64>().min(100.0));
        let g0 = gpus.first().cloned().unwrap_or(Value::Null);
        push(h, "vram", if f(&g0, "vramUsed") > 0.0 { f(&g0, "vramUsed") } else { f(&g0, "gttUsed") });
        push(h, "gtemp", f(&g0, "temp"));
        push(h, "resp", disks.iter().map(|x| f(x, "response")).fold(0.0, f64::max));
        for (i, x) in arr(&cpu, "cores").iter().enumerate() {
            push(h, &format!("core:{i}"), x.as_f64().unwrap_or(0.0));
        }
        push(h, "ctemp", f(&cpu, "temp"));
        push(h, "memz", d.get("zram").map(|z| f(z, "orig")).unwrap_or(0.0));
        push(h, "mempsi", f(&d, "pressure"));
        push(h, "swapio", d.get("swapIO").map(|x| f(x, "in") + f(x, "out")).unwrap_or(0.0));
        for x in disks {
            push(h, &format!("disk:{}", s(x, "name")), f(x, "active"));
            push(h, &format!("diskio:{}", s(x, "name")), f(x, "read") + f(x, "write"));
        }
        for x in nets {
            push(h, &format!("net:{}", s(x, "name")), f(x, "recv") + f(x, "send"));
        }
        self.sample = Some(d);
        self.samples += 1;
        // the picked row follows its program when the order changes
        if !self.pick.2.is_empty() && self.group(&self.pick.2.clone()).is_none() {
            self.pick = (String::new(), 0, String::new());
        }
    }

    fn set_tab(&mut self, t: Tab) -> Task<Msg> {
        if self.tab == t {
            return Task::none();
        }
        self.tab = t;
        self.menu = None;
        self.pick = (String::new(), 0, String::new());
        feed::set_want(self.wants());
        self.tab_reads()
    }

    /// What the tab on show reads now (it opened), then on its timers (subscription).
    fn tab_reads(&mut self) -> Task<Msg> {
        if !self.live() && self.samples > 0 {
            return Task::none();
        }
        match self.tab {
            Tab::Network => Task::batch([self.every("ports"), self.every("tailscale"), self.every("ssid")]),
            Tab::Overview => self.every("ports"),
            Tab::Disk => Task::batch([self.every("drives"), self.every("writing"), self.every("health")]),
            Tab::Cpu => self.every("focus"),
            _ => Task::none(),
        }
    }

    fn every(&mut self, what: &'static str) -> Task<Msg> {
        match what {
            "ports" => self.read("ports", feed::helper_cmd(15, &["ports"])),
            "drives" => self.read("drives", feed::helper_cmd(15, &["drives"])),
            "health" => self.read("health", feed::other_cmd("health", "")),
            "tailscale" => self.read("tailscale", feed::other_cmd("tailscale", "")),
            "ssid" => {
                let wifi: Vec<String> = self.sample.as_deref().map(|d| arr(d, "nets").iter().filter(|n| s(n, "kind") == "Wi-Fi").map(|n| s(n, "name").to_string()).collect()).unwrap_or_default();
                match wifi.first() {
                    Some(n) => self.read("ssid", feed::other_cmd("ssid", n)),
                    None => Task::none(),
                }
            }
            "writing" => {
                let mut busy: Vec<&Value> = self.groups().iter().filter(|g| f(g, "read") + f(g, "write") > 0.0).collect();
                busy.sort_by(|a, b| (f(b, "read") + f(b, "write")).partial_cmp(&(f(a, "read") + f(a, "write"))).unwrap_or(std::cmp::Ordering::Equal));
                busy.truncate(3);
                let mut names = HashMap::new();
                let mut pids: Vec<String> = Vec::new();
                for g in &busy {
                    for p in arr(g, "pids").iter().filter_map(Value::as_i64) {
                        names.insert(p, s(g, "name").to_string());
                        if pids.len() < 60 {
                            pids.push(p.to_string());
                        }
                    }
                }
                if pids.is_empty() {
                    self.files_busy.clear();
                    return Task::none();
                }
                self.busy_names = names;
                let mut args: Vec<&str> = vec!["writing"];
                args.extend(pids.iter().map(|x| x.as_str()));
                let cmd = feed::helper_cmd(15, &args);
                self.read("writing", cmd)
            }
            "focus" => {
                let Some(fc) = self.focus() else { return Task::none() };
                self.focus_seen = fc.gid.clone();
                let pids: Vec<String> = fc.pids.iter().take(64).map(|p| p.to_string()).collect();
                let mut svc: Vec<&str> = vec!["services"];
                svc.extend(pids.iter().map(|x| x.as_str()));
                let mut fl: Vec<&str> = vec!["files"];
                fl.extend(pids.iter().map(|x| x.as_str()));
                let pid = fc.pid.to_string();
                let (c1, c2, c3) = (feed::helper_cmd(20, &svc), feed::helper_cmd(15, &["libs", &pid]), feed::helper_cmd(15, &fl));
                Task::batch([self.read("services", c1), self.read("libs", c2), self.read("files", c3)])
            }
            _ => Task::none(),
        }
    }

    fn act(&mut self, args: Vec<String>, ok: String) -> Task<Msg> {
        let mut cmd = vec!["timeout".to_string(), "20".into(), "python3".into(), feed::helper()];
        cmd.extend(args);
        Task::perform(run::spawn(cmd), move |o| Msg::Acted(ok.clone(), o))
    }

    fn say(&mut self, t: String) -> Task<Msg> {
        self.toast = t;
        self.toast_n += 1;
        let n = self.toast_n;
        Task::perform(run::after(5000), move |_| Msg::ToastDone(n))
    }

    pub fn menu_items(&self) -> Vec<(&'static str, String, bool)> {
        let Some(m) = &self.menu else { return vec![] };
        let g = &m.prog;
        vec![
            ("end", "End task".into(), !g.protected),
            ("eco", if g.eco { "Efficiency mode: on (turn off)".into() } else { "Efficiency mode".into() }, !g.protected),
            ("where", "Open file location".into(), !g.exe.is_empty()),
            ("web", "Search online".into(), true),
        ]
    }

    fn pick_menu(&mut self, i: usize) -> Task<Msg> {
        let items = self.menu_items();
        let Some(m) = self.menu.take() else { return Task::none() };
        let Some((id, _, on)) = items.get(i).cloned() else { return Task::none() };
        if !on {
            self.menu = Some(m);
            return Task::none();
        }
        let g = m.prog;
        match id {
            "end" => {
                self.ask_end = Some(g);
                Task::none()
            }
            "eco" => {
                let ok = if g.eco { format!("Efficiency mode is off for {}.", g.name) } else { format!("{} runs in Efficiency mode now.", g.name) };
                self.act(vec!["eco".into(), g.pid.to_string(), if g.eco { "off" } else { "on" }.into()], ok)
            }
            "where" => {
                open_location(&g.exe);
                Task::none()
            }
            _ => {
                let q: String = g.name.chars().map(|c| if c.is_ascii_alphanumeric() { c.to_string() } else { format!("%{:02X}", c as u32 & 0xff) }).collect();
                run::detached(&["xdg-open", &format!("https://duckduckgo.com/?q={q}")]);
                Task::none()
            }
        }
    }

    fn open_menu_for(&mut self, gid: &str, at: Option<Point>) {
        let Some(prog) = self.prog_of(gid) else { return };
        let at = at.unwrap_or(Point::new(self.cursor.x.max(300.0), self.cursor.y.max(200.0)));
        let mut m = Menu { prog, at, sel: 0 };
        self.menu = Some(m.clone());
        m.sel = self.menu_items().iter().position(|x| x.2).unwrap_or(0);
        self.menu = Some(m);
    }

    /// The rows of the table the arrows move in (the tab's programs), as the page shows them.
    pub fn nav_rows(&self) -> (String, Vec<String>) {
        match view::main_table(self) {
            Some(t) => (t.key.clone(), t.rows.iter().map(|r| r.gid.clone().unwrap_or_default()).collect()),
            None => (String::new(), vec![]),
        }
    }
}

pub fn gid_of(g: &Value) -> String {
    match g.get("id") {
        Some(Value::String(x)) => x.clone(),
        Some(x) => x.to_string(),
        None => String::new(),
    }
}

/// Open file location: the file picked in the file manager (Dolphin on Plasma, through the FileManager1 interface every
/// file manager offers), else its folder.
fn open_location(path: &str) {
    if path.is_empty() {
        return;
    }
    let dir = std::path::Path::new(path).parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|| "/".into());
    let uri = format!("file://{path}");
    run::detached(&[
        "sh",
        "-c",
        "timeout 5 gdbus call --session --dest org.freedesktop.FileManager1 --object-path /org/freedesktop/FileManager1 --method org.freedesktop.FileManager1.ShowItems \"['$1']\" '' >/dev/null 2>&1 || xdg-open \"$2\"",
        "sh",
        &uri,
        &dir,
    ]);
}

fn update(a: &mut App, m: Msg) -> Task<Msg> {
    match m {
        Msg::Theme => a.p = hearth_ui::theme::load(),
        Msg::Feed(ev) => match ev {
            feed::Ev::Sample(d) => {
                let was_hidden = a.hidden;
                a.take(d);
                let mut t = vec![];
                if was_hidden {
                    t.push(a.tab_reads());
                }
                // CI's pictures: --show menu / --show end open the first program's menu or the End task question.
                let show = app::arg("--show").unwrap_or_default();
                if a.samples == 3 && !show.is_empty() {
                    let (key, rows) = a.nav_rows();
                    if let Some(g) = rows.first().cloned() {
                        a.pick = (key, 0, g.clone());
                        if show == "menu" {
                            a.open_menu_for(&g, Some(Point::new(330.0, 190.0)));
                        } else if show == "end" {
                            a.ask_end = a.prog_of(&g);
                        }
                    }
                }
                if a.samples == 2 {
                    if let Some(t0) = app::arg("--tick") {
                        let ids: Vec<String> = a.groups().iter().filter(|g| t0.split(',').any(|n| s(g, "name") == n)).map(gid_of).collect();
                        for id in ids {
                            a.ticked.insert(id.clone());
                            let (key, rows) = a.nav_rows();
                            if let Some(i) = rows.iter().position(|x| *x == id) {
                                a.pick = (key, i, id);
                            }
                        }
                    }
                }
                // the Processor tab looks into the program you picked or ticked as soon as it changes
                if a.tab == Tab::Cpu {
                    if let Some(fc) = a.focus() {
                        if fc.gid != a.focus_seen {
                            t.push(a.every("focus"));
                        }
                    }
                }
                return Task::batch(t);
            }
            feed::Ev::Hidden(h) => a.hidden = h,
            feed::Ev::Error(e) => a.err = e,
        },
        Msg::Tab(t) => return a.set_tab(t),
        Msg::Fold(k) => {
            if !a.folded.remove(&k) {
                a.folded.insert(k);
            }
            feed::set_want(a.wants());
        }
        Msg::Tick(gid) => {
            if !a.ticked.remove(&gid) {
                a.ticked.insert(gid);
            }
            if a.tab == Tab::Cpu {
                return a.every("focus");
            }
        }
        Msg::Pick(table, i, gid) => {
            a.pick = (table, i, gid);
            if a.tab == Tab::Cpu {
                return a.every("focus");
            }
        }
        Msg::RowMenu(table, i, gid) => {
            a.pick = (table, i, gid.clone());
            a.open_menu_for(&gid, Some(a.cursor));
        }
        Msg::Cursor(p) => a.cursor = p,
        Msg::MenuPick(i) => return a.pick_menu(i),
        Msg::CloseMenu => a.menu = None,
        Msg::EndYes => {
            if let Some(g) = a.ask_end.take() {
                if g.protected {
                    let why = if g.why.is_empty() { "This".to_string() } else { g.why.clone() };
                    return a.say(format!("{why} keeps Hearth running, so it can't be ended here."));
                }
                let ok = format!("{} was asked to close.", g.name);
                return a.act(vec!["end".into(), g.pid.to_string()], ok);
            }
        }
        Msg::EndNo => a.ask_end = None,
        Msg::Sort(table, col) => {
            let cur = a.sort.get(&table).copied();
            let next = match cur {
                Some((c, desc)) if c == col => (col, !desc),
                _ => (col, col >= 2),
            };
            a.sort.insert(table, next);
        }
        Msg::Pause => {
            a.paused = !a.paused;
            feed::set_paused(a.paused);
        }
        Msg::OpenTm => {
            // The Fedora edition's Task Manager is hearth-task-manager (pure Rust); Mantel's is hearth-tasks open.
            run::detached(&["sh", "-c", "command -v hearth-task-manager >/dev/null 2>&1 && exec hearth-task-manager; exec quickshell ipc -p /usr/share/nimbus/desktop/shell call tasks open"]);
        }
        Msg::Out(kind, o) => {
            a.busy.remove(kind);
            let j = o.json();
            match kind {
                "ports" => {
                    if let Some(v) = j {
                        let pings: Vec<f64> = arr(&v, "connections").iter().filter_map(|c| fo(c, "ping")).collect();
                        let avg = if pings.is_empty() { 0.0 } else { pings.iter().sum::<f64>() / pings.len() as f64 };
                        push(&mut a.hist, "ping", avg);
                        a.ports = v;
                    }
                }
                "drives" => a.drives = j.map(|v| arr(&v, "drives").to_vec()).unwrap_or_default(),
                "writing" => a.files_busy = j.map(|v| arr(&v, "files").to_vec()).unwrap_or_default(),
                "services" => a.services = j.map(|v| arr(&v, "services").to_vec()).unwrap_or_default(),
                "libs" => a.libs = j.map(|v| arr(&v, "libs").to_vec()).unwrap_or_default(),
                "files" => a.open_files = j.map(|v| arr(&v, "files").to_vec()).unwrap_or_default(),
                "health" => {
                    if let Some(v) = j {
                        a.health = v;
                    }
                }
                "hwset" => a.temp_limit = j.as_ref().and_then(|v| fo(v, "tempLimit")),
                "tailscale" => a.ts_peers = j.as_ref().and_then(|v| v.get("Peer")).and_then(Value::as_object).map(|m| m.len() as i64).unwrap_or(-1),
                "ssid" => {
                    let name = o.out.trim().to_string();
                    let iface = a.sample.as_deref().and_then(|d| arr(d, "nets").iter().find(|n| s(n, "kind") == "Wi-Fi").map(|n| s(n, "name").to_string())).unwrap_or_default();
                    if !iface.is_empty() && !name.is_empty() {
                        a.ssid.insert(iface, name);
                    }
                }
                "have" => a.have_disks = o.out.trim().to_string(),
                "pc" => match j {
                    Some(v) if v.get("view").is_some() => {
                        a.pc = v.get("pc").cloned();
                        let vw = v.get("view").cloned().unwrap_or(Value::Null);
                        let parts = arr(&vw, "parts");
                        if a.part.is_empty() || !parts.iter().any(|p| s(p, "id") == a.part) {
                            a.part = vw.get("def").and_then(Value::as_str).map(String::from).unwrap_or_else(|| parts.first().map(|p| s(p, "id").to_string()).unwrap_or_default());
                        }
                        if let Some(sel) = app::arg("--part") {
                            if parts.iter().any(|p| s(p, "id") == sel) {
                                a.part = sel;
                            }
                        }
                        a.part_cursor = a.part.clone();
                        a.pc_view = Some(vw);
                        a.pc_err.clear();
                    }
                    _ => {
                        let why = o.why();
                        a.pc_err = if why.is_empty() { "Couldn't read this PC's parts.".into() } else { format!("Couldn't read this PC's parts ({why}).") };
                    }
                },
                _ => {}
            }
        }
        Msg::Acted(ok, o) => {
            let line = o.out.trim().lines().last().unwrap_or("").to_string();
            let t = if !o.ok || line.starts_with("error: ") {
                let w = if line.starts_with("error: ") { line[7..].to_string() } else { o.why() };
                let mut c = w.chars();
                match c.next() {
                    Some(f0) => format!("{}{}.", f0.to_uppercase(), c.as_str().trim_end_matches('.')),
                    None => "That didn't work.".into(),
                }
            } else {
                ok
            };
            return a.say(t);
        }
        Msg::Every(what) => {
            if a.live() {
                return a.every(what);
            }
        }
        Msg::FilesQuery(q) => a.files_query = q,
        Msg::Part(id) => {
            a.part = id.clone();
            a.part_cursor = id;
        }
        Msg::PartAction => return board::run_action(a),
        Msg::ReadPcAgain => return a.read_pc(true),
        Msg::ToastDone(n) => {
            if n == a.toast_n {
                a.toast.clear();
            }
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
    let named = match key.as_ref() {
        Key::Named(k) => Some(k),
        _ => None,
    };
    let step = |a: &mut App, d: i32| -> Task<Msg> {
        let i = Tab::ALL.iter().position(|t| *t == a.tab).unwrap_or(0) as i32;
        let n = Tab::ALL.len() as i32;
        a.set_tab(Tab::ALL[((i + d + n) % n) as usize])
    };
    if modifiers.control() {
        match named {
            Some(Named::PageDown) => return step(a, 1),
            Some(Named::PageUp) => return step(a, -1),
            Some(Named::Tab) => return step(a, if modifiers.shift() { -1 } else { 1 }),
            _ => {}
        }
    }
    if a.ask_end.is_some() {
        match named {
            Some(Named::Enter) => return update(a, Msg::EndYes),
            Some(Named::Escape) => a.ask_end = None,
            _ => {}
        }
        return Task::none();
    }
    if a.menu.is_some() {
        let n = a.menu_items().len();
        let items = a.menu_items();
        let m = a.menu.as_mut().unwrap();
        match named {
            Some(Named::ArrowDown) | Some(Named::Tab) => {
                let mut i = m.sel;
                for _ in 0..n {
                    i = (i + 1) % n.max(1);
                    if items[i].2 {
                        break;
                    }
                }
                m.sel = i;
            }
            Some(Named::ArrowUp) => {
                let mut i = m.sel;
                for _ in 0..n {
                    i = (i + n - 1) % n.max(1);
                    if items[i].2 {
                        break;
                    }
                }
                m.sel = i;
            }
            Some(Named::Enter) | Some(Named::Space) => {
                let i = m.sel;
                return a.pick_menu(i);
            }
            Some(Named::Escape) | Some(Named::ContextMenu) | Some(Named::ArrowLeft) => a.menu = None,
            _ => {}
        }
        return Task::none();
    }
    if a.tab == Tab::Pc {
        match named {
            Some(Named::ArrowLeft) => board::move_cursor(a, -1.0, 0.0),
            Some(Named::ArrowRight) => board::move_cursor(a, 1.0, 0.0),
            Some(Named::ArrowUp) => board::move_cursor(a, 0.0, -1.0),
            Some(Named::ArrowDown) => board::move_cursor(a, 0.0, 1.0),
            Some(Named::Enter) | Some(Named::Space) => a.part = a.part_cursor.clone(),
            Some(Named::ContextMenu) => return board::run_action(a),
            Some(Named::Escape) => return win::close(),
            _ => {}
        }
        return Task::none();
    }
    let (table, rows) = a.nav_rows();
    let cur = if a.pick.0 == table { rows.iter().position(|g| *g == a.pick.2).unwrap_or(a.pick.1.min(rows.len().saturating_sub(1))) } else { usize::MAX };
    match named {
        Some(Named::ArrowDown) if !rows.is_empty() => {
            let i = if cur == usize::MAX { 0 } else { (cur + 1).min(rows.len() - 1) };
            return update(a, Msg::Pick(table, i, rows[i].clone()));
        }
        Some(Named::ArrowUp) if !rows.is_empty() => {
            let i = if cur == usize::MAX { 0 } else { cur.saturating_sub(1) };
            return update(a, Msg::Pick(table, i, rows[i].clone()));
        }
        Some(Named::Space) if cur != usize::MAX => return update(a, Msg::Tick(rows[cur].clone())),
        Some(Named::ContextMenu) if cur != usize::MAX => {
            let g = rows[cur].clone();
            a.open_menu_for(&g, None);
        }
        Some(Named::Delete) if cur != usize::MAX => a.ask_end = a.prog_of(&rows[cur]),
        Some(Named::Escape) => return win::close(),
        _ => {}
    }
    Task::none()
}

fn subscription(a: &App) -> Subscription<Msg> {
    let mut subs = vec![
        Subscription::run(hearth_ui::theme::watch).map(|_| Msg::Theme),
        Subscription::run(feed::stream).map(Msg::Feed),
        iced::keyboard::listen().map(Msg::Key),
    ];
    // The tab's other readings, Mantel's timers, only while the window is seen and not paused.
    if a.live() {
        let ev = |secs: u64, w: &'static str| iced::time::every(std::time::Duration::from_secs(secs)).with(w).map(|(w, _)| Msg::Every(w));
        match a.tab {
            Tab::Network => {
                subs.push(ev(3, "ports"));
                subs.push(ev(30, "tailscale"));
                subs.push(ev(30, "ssid"));
            }
            Tab::Overview => subs.push(ev(5, "ports")),
            Tab::Disk => {
                subs.push(ev(5, "drives"));
                subs.push(ev(3, "writing"));
                subs.push(ev(60, "health"));
            }
            Tab::Cpu => subs.push(ev(5, "focus")),
            _ => {}
        }
    }
    Subscription::batch(subs)
}

fn view(a: &App) -> Element<'_, Msg> {
    view::view(a)
}

fn main() -> iced::Result {
    if app::handle_cli(
        "hearth-resmon-app",
        "Resource Monitor: this PC's parts and what each program uses, live.\n  --page pc|overview|cpu|memory|disk|network|gpu   (or the tab's name as the first word)\n  --show menu|end   --tick NAME,…   --unfold KEY,…   --part ID",
    ) {
        return Ok(());
    }
    iced::application(App::new, update, view)
        .title("Resource Monitor")
        .subscription(subscription)
        .theme(|a: &App| app::iced_theme(&a.p))
        .window(app::window("hearth-resmon", (1280.0, 800.0), (900.0, 560.0)))
        .default_font(app::INTER)
        .run()
}
