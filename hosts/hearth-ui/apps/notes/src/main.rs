//! Hearth Notes (hearth-notes-app): the owner's own Obsidian, connected to his hearth-workspace repository. Built to
//! the approved mockups (docs/mockups/hearth-notes, checklist docs/mockups/checklists/hearth-notes.md) and the plan in
//! docs/HEARTH-NOTES.md.
//!
//! Light at rest: nothing polls. The vault is read when the app opens (and after a save or a sync), the graph modes
//! animate only while their page is in front, the window is focused and something was done there in the last 30 s
//! (Reduce motion: never), and Sync by itself is one timer every N minutes. Opening it writes none of your settings.
//!
//! Mouse, keyboard and controller: Ctrl+O the quick switcher, Ctrl+Shift+F search, Ctrl+Shift+L today's log, Esc back;
//! with a controller the layout of page 9 (LB / RB pages, A / B / X / Y, the on-screen keyboard).

mod boardv;
mod editor;
mod gview;
mod look;
mod newitemv;
mod pad;
mod padview;
mod searchv;
mod settingsv;
mod side;
mod syncv;
mod today;

use hearth_notes::board::{self, Status};
use hearth_notes::check::{self, Check};
use hearth_notes::conflict::{self, Pick};
use hearth_notes::day::{self, Ymd};
use hearth_notes::git::{self, Auth};
use hearth_notes::graph::{self, Graph};
use hearth_notes::prefs::{self, Prefs};
use hearth_notes::search::{self, Query};
use hearth_notes::sync::{self, Step};
use hearth_notes::trackers;
use hearth_notes::vault::Vault;
use hearth_notes::{md, newitem, paths, save};
use hearth_ui::app::{self, win};
use hearth_ui::Pal;
use iced::keyboard::{key::Named, Key};
use iced::widget::{operation, text_editor};
use iced::{Subscription, Task};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub const LINE_ID: &str = "notes-line";
pub const SEARCH_ID: &str = "notes-search";
pub const QS_ID: &str = "notes-quick-switcher";
pub const FLY_ID: &str = "notes-fly";
pub const QUICK_ID: &str = "notes-quick-add";
pub const MSG_ID: &str = "notes-commit-message";
pub const NAME_ID: &str = "notes-new-name";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Files,
    Search,
    Today,
    Board,
    Graph,
    Modes,
    Sync,
    Conflict,
    Settings,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tab {
    Note(String),
    Board,
    Graph,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Live,
    Source,
    Read,
}

impl Mode {
    fn of(s: &str) -> Mode {
        match s {
            "source" => Mode::Source,
            "read" => Mode::Read,
            _ => Mode::Live,
        }
    }
}

/// Where the vault is at.
#[derive(Debug, Clone, PartialEq)]
pub enum VaultState {
    Reading,
    Ready,
    /// The folder isn't there: the first-run step offers to clone hearth-workspace.
    Missing,
    Cloning,
    CloneFailed(String),
}

/// The open note.
pub struct Doc {
    pub rel: String,
    pub lines: Vec<String>,
    pub trailing_nl: bool,
    /// The line shown raw and typed in (live preview).
    pub cur: Option<usize>,
    pub ac: usize,
    pub gen: u64,
    pub dirty: bool,
    pub saved: bool,
    pub error: String,
    pub source: text_editor::Content,
    /// A generated page (Atlas maps): read only.
    pub generated: bool,
    /// The line was just emptied by Backspace (the next Backspace removes the line, not this one).
    pub fresh_empty: bool,
}

impl Doc {
    pub fn text(&self) -> String {
        let mut t = self.lines.join("\n");
        if self.trailing_nl {
            t.push('\n');
        }
        t
    }
    pub fn from_text(rel: &str, text: &str) -> Doc {
        Doc::from(rel, text, false)
    }
    fn from(rel: &str, text: &str, generated: bool) -> Doc {
        let trailing_nl = text.ends_with('\n');
        let mut lines: Vec<String> = text.split('\n').map(|l| l.trim_end_matches('\r').to_string()).collect();
        if trailing_nl {
            lines.pop();
        }
        if lines.is_empty() {
            lines.push(String::new());
        }
        Doc { rel: rel.to_string(), lines, trailing_nl, cur: None, ac: 0, gen: 0, dirty: false, saved: true, error: String::new(), source: text_editor::Content::with_text(text), generated, fresh_empty: false }
    }
}

/// Today's quick add.
#[derive(Debug, Clone, Default)]
pub struct Quick {
    pub kind: usize,
    pub what: String,
    pub at: String,
    pub waiting: String,
}

pub const QUICK_KINDS: [&str; 5] = ["Made", "Changed", "Decided", "Stored at", "Waiting"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    Status,
    Project,
    Tag,
    Date,
}

/// The graph's camera and what's picked.
#[derive(Debug, Clone)]
pub struct Cam {
    pub x: f32,
    pub y: f32,
    pub zoom: f32,
    pub yaw: f32,
    pub pitch: f32,
}

impl Default for Cam {
    fn default() -> Self {
        Cam { x: 0.0, y: 0.0, zoom: 1.0, yaw: 0.0, pitch: 0.35 }
    }
}

#[derive(Debug, Clone)]
pub struct GraphUi {
    pub mode: String,
    pub cam: Cam,
    pub hover: Option<usize>,
    pub sel: Option<usize>,
    pub fly: String,
    pub fly_sel: usize,
    /// Projects shown (empty: the default, those with notes).
    pub projects: BTreeSet<String>,
    pub tags: Vec<String>,
    pub tag_typing: Option<String>,
    pub customise: bool,
    pub local: bool,
    pub paused: bool,
    pub full: bool,
    /// The animation's clock (seconds), moved on only while it's awake.
    pub t: f32,
    pub awake_until: Option<Instant>,
    pub last_frame: Option<Instant>,
    /// "Particles along links when a note opens": until then.
    pub burst_until: f32,
    pub picker_focus: usize,
    pub look_menu: Option<String>,
    /// The note the camera last flew to (Galaxy draws the arc to it).
    pub flew: Option<usize>,
    /// Fly to's list is hidden (the name typed, the camera already there).
    pub fly_closed: bool,
}

#[derive(Debug, Clone, Default)]
pub struct SyncUi {
    pub changes: Vec<git::Change>,
    pub sel: usize,
    pub diff: Vec<git::DiffLine>,
    pub msg: String,
    pub check: Option<Check>,
    pub checked_at: i64,
    pub busy: bool,
    pub said: String,
    pub bad: bool,
    pub repo: git::Repo,
    pub who: git::Identity,
    pub auth: Option<Auth>,
    pub last_sync: i64,
    pub secrets: Vec<(String, hearth_notes::secrets::Hit)>,
    pub is_repo: bool,
    pub conflicts: Vec<String>,
    pub next_auto: i64,
}

#[derive(Debug, Clone, Default)]
pub struct ConflictUi {
    pub files: Vec<String>,
    pub i: usize,
    pub mine: String,
    pub theirs: String,
    pub commit: String,
    pub theirs_at: i64,
    pub pick: Option<Pick>,
}

#[derive(Debug, Clone)]
pub struct NewItem {
    pub project: usize,
    pub kind: usize,
    pub name: String,
    pub status: usize,
    pub release: String,
    pub error: String,
}

/// The controller layout's on-screen keyboard and quick note.
#[derive(Debug, Clone, Default)]
pub struct Osk {
    pub open: bool,
    pub row: usize,
    pub col: usize,
    pub shift: bool,
    pub text: String,
    pub kind: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PadPage {
    Today,
    Notes,
    Graph,
    Sync,
}

#[derive(Debug)]
pub struct Loaded {
    pub vault: Vault,
    pub graph: Graph,
    pub opened: BTreeMap<String, u64>,
}

#[derive(Debug, Clone)]
pub struct SyncInfo {
    pub is_repo: bool,
    pub changes: Vec<git::Change>,
    pub repo: git::Repo,
    pub who: git::Identity,
    pub last_sync: i64,
    pub conflicts: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum Msg {
    /// Describe it and its blocks (hearth_ui::describe, hearth_ui::blocks; every Hearth app has them)
    Describe(hearth_ui::describe::DMsg),
    Theme,
    Event(iced::Event),
    Pad(pad::Button),
    Drag,
    Maximize,
    Minimize,
    Close,
    Resize(iced::window::Direction),
    Loaded(std::sync::Arc<std::sync::Mutex<Option<Loaded>>>),
    GraphReady(std::sync::Arc<Graph>),
    Rail(Page),
    Toggle(String),
    OpenNote(usize),
    OpenRel(String),
    OpenAtlas(String),
    TabPick(usize),
    TabClose(usize),
    SetMode(Mode),
    LineClick(usize),
    LineEdit(String),
    LineEnter,
    LineAway,
    TaskToggle(usize),
    AcPick(String),
    Source(text_editor::Action),
    SaveTick(u64),
    Saved(String, Result<(String, i64), String>),
    Cal(i32),
    CalDay(Ymd),
    Outline(usize),
    // Today
    QuickKind(usize),
    QuickWhat(String),
    QuickAt(String),
    QuickWait(String),
    QuickAdd,
    DayStep(i64),
    Productivity(u8),
    Habit(String),
    TodayAction(&'static str),
    // Board
    BoardGroup(Group),
    BoardProject(Option<String>),
    BoardTag(Option<String>),
    BoardMenu(u8),
    CardMove(usize, Status),
    CardOpen(usize),
    CardTick(usize),
    CardFocus(usize),
    CardDrag(usize),
    CardDrop(usize),
    AddCard(Status),
    // Search
    SearchText(String),
    SearchCase,
    SearchExact,
    SearchFolder(Option<String>),
    SearchTag(Option<String>),
    SearchDays(Option<u32>),
    SearchFilterMenu(u8),
    SearchOpen(usize),
    MatchStep(i32),
    QsOpen,
    QsText(String),
    QsPick(usize, bool),
    QsMake,
    QsClose,
    // Graph
    GMode(String),
    GHover(Option<usize>),
    GClick(Option<usize>),
    GPan(f32, f32),
    GZoom(f32),
    GZoomTo(f32),
    GTurn(f32, f32),
    GReset,
    GPause,
    GFull,
    FlyText(String),
    FlyGo(usize),
    GOpen(usize),
    GOnly(usize),
    GProject(String),
    GTagDrop(String),
    GTagAdd,
    GTagText(String),
    GTagDone,
    GOpt(String, Value),
    GCustomise(bool),
    GLocal(bool),
    GPickFocus(usize),
    Frame,
    LookSave,
    LookShare,
    LookOpen,
    LookFile(Option<String>),
    LookApply(String),
    LookMenu(Option<String>),
    ProjectLook(String, String),
    // Sync
    SyncSel(usize),
    SyncMsg(String),
    SyncNow,
    SyncPull,
    SyncDone(Box<Step>),
    SyncInfo(Box<SyncInfo>),
    CheckAgain,
    Checked(Check),
    Fix(usize),
    Fixed(Result<(), String>),
    SyncEvery(u32),
    AutoSync,
    SecretsGo,
    SecretsCancel,
    Diff(Vec<git::DiffLine>),
    // Conflict
    ConflictLoaded(Box<ConflictUi>),
    ConflictPick(Pick),
    ConflictLater,
    ConflictStep(i32),
    OpenKept,
    Settled(Result<bool, String>),
    // New item
    NewOpen,
    NewProject(usize),
    NewKind(usize),
    NewName(String),
    NewSuggested,
    NewStatus(usize),
    NewRelease(String),
    NewMake,
    NewCancel,
    NewMade(Result<newitem::Made, String>),
    // Settings
    SetSec(usize),
    Set(&'static str, Value),
    CheckSignIn,
    Authed(Auth),
    ChangeVault,
    VaultPicked(Option<String>),
    CloneNow,
    Cloned(Result<(), String>),
    DayStartStep(i32),
    // Controller layout
    PadPage(PadPage),
    PadNote(usize),
    OskKey(usize, usize),
    OskKind(usize),
    OskToggle,
    Nothing,
}

pub struct App {
    pub describe: hearth_ui::describe::Describe,
    pub base: Pal,
    pub p: Pal,
    pub prefs: Prefs,
    pub root: PathBuf,
    pub vs: VaultState,
    pub vault: Vault,
    pub graph: std::sync::Arc<Graph>,
    pub opened: BTreeMap<String, u64>,
    pub page: Page,
    pub tabs: Vec<Tab>,
    pub tab: usize,
    pub doc: Option<Doc>,
    pub mode: Mode,
    pub tree_open: BTreeSet<String>,
    pub cal: (i32, u32),
    pub today: Ymd,
    pub day: Ymd,
    pub now: i64,
    pub quick: Quick,
    pub track: trackers::Day,
    pub group: Group,
    pub board_project: Option<String>,
    pub board_tag: Option<String>,
    pub board_menu: u8,
    pub card_focus: usize,
    pub card_drag: Option<usize>,
    pub query: Query,
    pub hits: Vec<search::Hit>,
    pub hit_focus: usize,
    pub match_at: usize,
    pub filter_menu: u8,
    pub qs: Option<(String, usize)>,
    pub g: GraphUi,
    pub sync: SyncUi,
    pub conflict: Option<ConflictUi>,
    pub new_item: Option<NewItem>,
    pub settings_sec: usize,
    pub pad: bool,
    pub pad_fixed: bool,
    pub pad_page: PadPage,
    pub pad_note: usize,
    pub osk: Osk,
    pub focus: String,
    pub note: String,
    pub note_bad: bool,
    pub focused: bool,
    pub this_pc: String,
    pub reduce_motion: bool,
    pub cpu: bool,
    pub show: String,
    pub look_name: String,
}

/// Work off the window's thread (a short-lived thread; the future is ready when it ends).
pub fn off<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> impl std::future::Future<Output = Option<T>> + Send + 'static {
    let (tx, rx) = futures::channel::oneshot::channel();
    let _ = std::thread::Builder::new().name("hearth-notes-work".into()).spawn(move || {
        let _ = tx.send(f());
    });
    async move { rx.await.ok() }
}

fn this_pc() -> String {
    if let Ok(s) = std::env::var("HEARTH_NOTES_THIS_PC") {
        return s;
    }
    let ver = std::fs::read_to_string("/usr/share/nimbus/version").map(|s| s.trim().to_string()).unwrap_or_default();
    let info: Value = std::fs::read("/usr/share/nimbus/image-info.json").ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
    let ver = if ver.is_empty() { info.get("version").and_then(|x| x.as_str()).unwrap_or("").to_string() } else { ver };
    let ver: String = {
        let p: Vec<&str> = ver.split('.').collect();
        if p.len() >= 3 && p[2..].iter().all(|x| *x == "0") { p[..2].join(".") } else { ver.clone() }
    };
    let base = info.get("base").and_then(|x| x.as_str()).unwrap_or("");
    let fedora = base == "anatase" || std::path::Path::new("/usr/bin/rpm-ostree").exists();
    match (ver.is_empty(), fedora) {
        (true, _) => "Hearth OS".into(),
        (false, true) => format!("Hearth OS {ver} · Fedora edition"),
        (false, false) => format!("Hearth OS {ver}"),
    }
}

/// Hearth's Reduce motion (desktop.json "reduceMotion", Mantel's Settings.qml).
fn reduce_motion() -> bool {
    if std::env::var("HEARTH_NOTES_REDUCE_MOTION").map(|v| v == "1").unwrap_or(false) {
        return true;
    }
    let v: Value = std::fs::read(hearth_ui::theme::desktop_json()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
    v.get("reduceMotion").and_then(Value::as_bool).unwrap_or(false)
}

fn look_name() -> String {
    let v: Value = std::fs::read(hearth_ui::theme::desktop_json()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
    let look = v.get("look").and_then(Value::as_str).unwrap_or("steam").to_string();
    let pals = hearth_ui::theme::palette_map();
    let label = pals.get(&look).and_then(|l| l.get("label")).and_then(Value::as_str).unwrap_or("");
    if label.is_empty() { "Classic".into() } else { label.to_string() }
}

/// The app's own theme on top of Hearth's look: Hearth (as it is), Gold, Firehold.
fn themed(base: Pal, theme: &str) -> Pal {
    match theme {
        "gold" => Pal { accent: iced::Color::from_rgb8(0xd9, 0xa6, 0x3f), ink: iced::Color::from_rgb8(0x1f, 0x15, 0x05), ..base },
        "firehold" => Pal { accent: iced::Color::from_rgb8(0xff, 0x6a, 0x2a), ink: iced::Color::from_rgb8(0x1b, 0x0c, 0x04), bg: Pal::mix(base.bg, iced::Color::BLACK, 0.25), ..base },
        _ => base,
    }
}

/// A game is running (gamescope, or Hearth's hearth-playing mark): no syncing and no animation then.
pub fn game_running() -> bool {
    if let Ok(rt) = std::env::var("XDG_RUNTIME_DIR") {
        if std::path::Path::new(&rt).join("hearth-playing").exists() {
            return true;
        }
    }
    let Ok(rd) = std::fs::read_dir("/proc") else { return false };
    for e in rd.flatten() {
        let n = e.file_name();
        let n = n.to_string_lossy();
        if !n.bytes().all(|b| b.is_ascii_digit()) {
            continue;
        }
        if let Ok(c) = std::fs::read_to_string(e.path().join("comm")) {
            let c = c.trim();
            if c == "gamescope" || c == "gamescope-wl" {
                return true;
            }
        }
    }
    false
}

fn page_of(s: &str) -> Page {
    match s {
        "search" => Page::Search,
        "today" => Page::Today,
        "board" => Page::Board,
        "graph" => Page::Graph,
        "modes" | "graph-modes" => Page::Modes,
        "sync" => Page::Sync,
        "conflict" => Page::Conflict,
        "settings" => Page::Settings,
        _ => Page::Files,
    }
}

pub fn load_opened() -> BTreeMap<String, u64> {
    let v: Value = std::fs::read(paths::opened()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
    v.as_object().map(|m| m.iter().filter_map(|(k, v)| v.as_u64().map(|n| (k.clone(), n))).collect()).unwrap_or_default()
}

impl App {
    fn new() -> (App, Task<Msg>) {
        let prefs = prefs::load();
        let root = prefs.vault.as_ref().map(|v| PathBuf::from(shell_expand(v))).unwrap_or_else(paths::default_vault);
        let base = hearth_ui::theme::load();
        let now = paths::now();
        let today = day::log_day(now, prefs.day_start);
        let cpu = hearth_ui::app::renderer("hearth-notes") == "cpu";
        let mut a = App {
            describe: hearth_ui::describe::Describe::new("notes", "Hearth Notes").with_pages(side::BLOCKS).with_kit(&side::KIT),
            p: themed(base, &prefs.theme),
            base,
            mode: Mode::of(&prefs.open_in),
            g: GraphUi {
                mode: prefs.graph_mode.clone(),
                cam: Cam::default(),
                hover: None,
                sel: None,
                fly: String::new(),
                fly_sel: 0,
                projects: BTreeSet::new(),
                tags: Vec::new(),
                tag_typing: None,
                customise: false,
                local: false,
                paused: false,
                full: false,
                t: 4.0,
                awake_until: None,
                last_frame: None,
                burst_until: 0.0,
                picker_focus: 0,
                look_menu: None,
                flew: None,
                fly_closed: false,
            },
            prefs,
            vs: if root.is_dir() { VaultState::Reading } else { VaultState::Missing },
            root,
            vault: Vault::default(),
            graph: std::sync::Arc::new(Graph::default()),
            opened: BTreeMap::new(),
            page: Page::Files,
            tabs: Vec::new(),
            tab: 0,
            doc: None,
            tree_open: ["atlas".to_string()].into_iter().collect(),
            cal: (today.0, today.1),
            today,
            day: today,
            now,
            quick: Quick::default(),
            track: trackers::Day::default(),
            group: Group::Status,
            board_project: None,
            board_tag: None,
            board_menu: 0,
            card_focus: 0,
            card_drag: None,
            query: Query::default(),
            hits: Vec::new(),
            hit_focus: 0,
            match_at: 0,
            filter_menu: 0,
            qs: None,
            sync: SyncUi::default(),
            conflict: None,
            new_item: None,
            settings_sec: 0,
            pad: app::flag("--controller"),
            pad_fixed: app::flag("--controller") || app::flag("--mouse"),
            pad_page: PadPage::Notes,
            pad_note: 0,
            osk: Osk { open: true, row: 2, col: 3, ..Default::default() },
            focus: String::new(),
            note: String::new(),
            note_bad: false,
            focused: true,
            this_pc: this_pc(),
            reduce_motion: reduce_motion(),
            cpu,
            show: app::arg("--show").unwrap_or_default(),
            look_name: look_name(),
        };
        pad::PAD_LOOK.store(a.pad, std::sync::atomic::Ordering::Relaxed);
        a.track = trackers::load(&paths::vault_key(&a.root), &day::ymd(a.day));
        if let Some(pg) = app::arg("--page") {
            a.page = page_of(&pg);
        }
        if let Some(m) = app::arg("--graph-mode") {
            a.g.mode = m;
        }
        let mut tasks = vec![a.load_vault()];
        tasks.push(a.refresh_sync());
        (a, Task::batch(tasks))
    }

    pub fn load_vault(&mut self) -> Task<Msg> {
        if !self.root.is_dir() {
            self.vs = VaultState::Missing;
            return Task::none();
        }
        let root = self.root.clone();
        Task::perform(
            off(move || {
                let vault = Vault::scan(&root);
                let graph = graph::build(&vault);
                Loaded { vault, graph, opened: load_opened() }
            }),
            |r| Msg::Loaded(std::sync::Arc::new(std::sync::Mutex::new(r))),
        )
    }

    pub fn refresh_sync(&self) -> Task<Msg> {
        let root = self.root.clone();
        Task::perform(
            off(move || {
                let is_repo = git::is_repo(&root);
                if !is_repo {
                    return SyncInfo { is_repo, changes: vec![], repo: Default::default(), who: Default::default(), last_sync: 0, conflicts: vec![] };
                }
                SyncInfo { is_repo, changes: git::changes(&root), repo: git::repo(&root), who: git::identity(&root), last_sync: git::last_sync(&root), conflicts: git::conflicts(&root) }
            }),
            |r| r.map(|i| Msg::SyncInfo(Box::new(i))).unwrap_or(Msg::Nothing),
        )
    }

    pub fn say(&mut self, s: impl Into<String>, bad: bool) {
        self.note = s.into();
        self.note_bad = bad;
    }

    pub fn vault_key(&self) -> String {
        paths::vault_key(&self.root)
    }

    pub fn today_rel(&self) -> String {
        day::log_path(self.today)
    }

    pub fn open_index(&self) -> Option<usize> {
        self.doc.as_ref().and_then(|d| self.vault.find(&d.rel))
    }

    /// The vault's name in the title bar and its header.
    pub fn vault_name(&self) -> String {
        self.root.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "Notes".into())
    }

    pub fn central_now(&self) -> day::Central {
        day::central(self.now)
    }

    /// The tab strip's tabs for the page in front.
    pub fn ensure_tab(&mut self, t: Tab) {
        if let Some(i) = self.tabs.iter().position(|x| *x == t) {
            self.tab = i;
        } else {
            self.tabs.push(t);
            self.tab = self.tabs.len() - 1;
        }
    }

    pub fn open_rel(&mut self, rel: &str) -> Task<Msg> {
        self.open_rel_as(rel, true)
    }

    /// Opens a note; `count`: the person opened it (counted for Solar system's sizes and the recent notes). The note
    /// the app opens with by itself isn't counted, so opening the app writes nothing.
    pub fn open_rel_as(&mut self, rel: &str, count: bool) -> Task<Msg> {
        let flush = self.flush_doc();
        let text = match self.vault.find(rel) {
            Some(i) => self.vault.notes[i].text.clone(),
            None => std::fs::read_to_string(self.root.join(rel)).unwrap_or_default(),
        };
        self.doc = Some(Doc::from(rel, &text, false));
        self.mode = Mode::of(&self.prefs.open_in);
        self.ensure_tab(Tab::Note(rel.to_string()));
        if rel == self.today_rel() {
            self.page = Page::Today;
        } else if !matches!(self.page, Page::Search) {
            self.page = Page::Files;
        }
        // Open the folders it's in.
        let parts: Vec<&str> = rel.split('/').collect();
        for i in 1..parts.len() {
            self.tree_open.insert(parts[..i].join("/"));
        }
        if let Some(i) = self.vault.find(rel) {
            self.g.sel = Some(i);
            self.g.burst_until = self.g.t + 3.0;
        }
        if !count {
            return flush;
        }
        *self.opened.entry(rel.to_string()).or_insert(0) += 1;
        let opened = self.opened.clone();
        let count = Task::perform(
            off(move || {
                let m: serde_json::Map<String, Value> = opened.into_iter().map(|(k, v)| (k, json!(v))).collect();
                let _ = save::atomic(&paths::opened(), format!("{}\n", Value::Object(m)).as_bytes());
            }),
            |_| Msg::Nothing,
        );
        Task::batch([flush, count])
    }

    pub fn open_atlas(&mut self, which: &str) -> Task<Msg> {
        if which == "home" {
            return self.open_rel("INDEX.md");
        }
        let flush = self.flush_doc();
        let text = editor::atlas_page(self, which);
        let rel = format!("atlas:{which}");
        self.doc = Some(Doc::from(&rel, &text, true));
        self.mode = Mode::Read;
        self.ensure_tab(Tab::Note(rel));
        self.page = Page::Files;
        flush
    }

    /// Saves what's typed now (before another note opens).
    pub fn flush_doc(&mut self) -> Task<Msg> {
        let Some(d) = self.doc.as_mut() else { return Task::none() };
        if !d.dirty || d.generated {
            return Task::none();
        }
        d.dirty = false;
        let (rel, text, root) = (d.rel.clone(), d.text(), self.root.clone());
        save_task(root, rel, text)
    }

    /// Saves what's typed now, before the window closes (here, so it's on the drive before the app ends).
    pub fn flush_now(&mut self) {
        let Some(d) = self.doc.as_mut() else { return };
        if !d.dirty || d.generated {
            return;
        }
        d.dirty = false;
        if let Err(e) = save::note(&self.root, &d.rel, &d.text()) {
            eprintln!("hearth-notes: couldn't save {}: {e}", d.rel);
        }
    }

    /// A change typed: saved a moment after typing stops.
    pub fn edited(&mut self) -> Task<Msg> {
        let Some(d) = self.doc.as_mut() else { return Task::none() };
        if d.generated {
            return Task::none();
        }
        d.dirty = true;
        d.saved = false;
        d.gen += 1;
        let g = d.gen;
        Task::perform(hearth_ui::run::after(350), move |_| Msg::SaveTick(g))
    }

    pub fn rail(&mut self, pg: Page) -> Task<Msg> {
        let mut t = self.flush_doc();
        self.qs = None;
        match pg {
            Page::Today => {
                self.day = self.today;
                let rel = self.today_rel();
                let text = self.vault.find(&rel).map(|i| self.vault.notes[i].text.clone()).unwrap_or_default();
                self.doc = Some(Doc::from(&rel, &text, false));
                self.ensure_tab(Tab::Note(rel));
                self.track = trackers::load(&self.vault_key(), &day::ymd(self.day));
            }
            Page::Board => self.ensure_tab(Tab::Board),
            Page::Graph | Page::Modes => {
                self.ensure_tab(Tab::Graph);
                self.wake();
            }
            Page::Sync => {
                t = Task::batch([t, self.refresh_sync()]);
                if self.sync.check.is_none() {
                    t = Task::batch([t, self.run_check()]);
                }
            }
            Page::Search => {
                t = Task::batch([t, operation::focus(SEARCH_ID)]);
            }
            _ => {}
        }
        self.page = pg;
        t
    }

    pub fn run_check(&mut self) -> Task<Msg> {
        let root = self.root.clone();
        Task::perform(off(move || check::run(&root)), |r| Msg::Checked(r.unwrap_or(Check::Failed("it stopped".into()))))
    }

    /// The graph's animation wakes for 30 s (something was done on its page).
    pub fn wake(&mut self) {
        if self.reduce_motion || app::flag("--still") {
            return;
        }
        // 30 s after the last thing done on the graph's page (CI measures the rest after a shorter one).
        let secs = std::env::var("HEARTH_NOTES_AWAKE_SECS").ok().and_then(|s| s.parse::<u64>().ok()).unwrap_or(30);
        self.g.awake_until = Some(Instant::now() + Duration::from_secs(secs));
    }

    pub fn animating(&self) -> bool {
        let page = matches!(self.page, Page::Graph | Page::Modes) || (self.pad && self.pad_page == PadPage::Graph);
        page && self.focused && !self.reduce_motion && !self.g.paused && self.g.awake_until.map(|t| t > Instant::now()).unwrap_or(false)
    }

    pub fn cards(&self) -> Vec<board::Card> {
        board::cards(&self.vault, self.today)
            .into_iter()
            .filter(|c| self.board_project.as_ref().map(|p| &c.project == p).unwrap_or(true))
            .filter(|c| self.board_tag.as_ref().map(|t| c.tag.eq_ignore_ascii_case(t)).unwrap_or(true))
            .collect()
    }

    fn run_search(&mut self) {
        self.hits = search::search(&self.vault, &self.query, self.today);
        self.hit_focus = 0;
        self.match_at = 0;
    }

    pub fn sync_line(&self) -> (iced::Color, String) {
        let s = &self.sync;
        if !s.is_repo {
            return (self.p.muted, "Not a Git repository: notes stay on this PC".into());
        }
        if !s.conflicts.is_empty() || self.conflict.is_some() {
            let n = s.conflicts.len().max(1);
            return (look::WARN, format!("{n} note{} changed on two PCs", if n == 1 { "" } else { "s" }));
        }
        if let Some(Check::Untidy(p)) = &s.check {
            if !s.changes.is_empty() {
                return (look::BAD, format!("{} change{} · the check found {} problem{}", s.changes.len(), plural(s.changes.len()), p.len(), plural(p.len())));
            }
        }
        if !s.changes.is_empty() {
            let mins = if self.prefs.sync_every == 0 { String::new() } else { format!(" · syncs in {} min", ((s.next_auto - self.now).max(60) / 60).max(1)) };
            return (self.p.accent, format!("{} change{} to push{mins}", s.changes.len(), plural(s.changes.len())));
        }
        let when = if s.last_sync > 0 { format!(" · synced {}", { let c = day::central(s.last_sync); day::clock(c.hour, c.min) }) } else { String::new() };
        (look::GOOD, format!("Up to date{when}"))
    }

    fn start_sync(&mut self, allow_secrets: bool, push: bool) -> Task<Msg> {
        if self.sync.busy {
            return Task::none();
        }
        let t = self.flush_doc();
        self.sync.busy = true;
        self.sync.said = if push { "Syncing…".into() } else { "Getting GitHub's changes…".into() };
        let (root, msg) = (self.root.clone(), self.sync.msg.clone());
        let o = sync::Options { warn_secrets: self.prefs.warn_secrets, allow_secrets, push };
        Task::batch([
            t,
            Task::perform(off(move || if push { sync::sync(&root, &msg, o) } else { sync::pull_only(&root) }), |r| Msg::SyncDone(Box::new(r.unwrap_or(Step::Failed("it stopped".into()))))),
        ])
    }

    fn load_conflict(&self, files: Vec<String>, i: usize) -> Task<Msg> {
        let root = self.root.clone();
        Task::perform(
            off(move || {
                let rel = files.get(i).cloned().unwrap_or_default();
                let mine = git::stage(&root, 2, &rel).unwrap_or_default();
                let theirs = git::stage(&root, 3, &rel).unwrap_or_default();
                let (commit, at) = git::theirs_commit(&root);
                ConflictUi { files, i, mine, theirs, commit, theirs_at: at, pick: None }
            }),
            |r| r.map(|c| Msg::ConflictLoaded(Box::new(c))).unwrap_or(Msg::Nothing),
        )
    }

    pub fn set_pref(&mut self, k: &'static str, v: Value) -> Task<Msg> {
        match prefs::set(k, v) {
            Ok(p) => {
                self.prefs = p;
                self.p = themed(self.base, &self.prefs.theme);
                if k == "dayStart" {
                    self.today = day::log_day(self.now, self.prefs.day_start);
                    self.day = self.today;
                    self.cal = (self.today.0, self.today.1);
                }
                if k == "syncEvery" {
                    self.sync.next_auto = self.now + self.prefs.sync_every as i64 * 60;
                }
            }
            Err(e) => self.say(e, true),
        }
        Task::none()
    }

    pub fn set_graph(&mut self, k: &str, v: Value) {
        match prefs::set_in("graph", &format!("{}.{k}", self.g.mode), v) {
            Ok(p) => self.prefs = p,
            Err(e) => self.say(e, true),
        }
    }

    /// A graph option for the mode in use (Customise), else its default.
    pub fn gopt(&self, k: &str) -> Option<&Value> {
        self.prefs.graph.get(&format!("{}.{k}", self.g.mode))
    }
    pub fn gb(&self, k: &str, d: bool) -> bool {
        self.gopt(k).and_then(Value::as_bool).unwrap_or(d)
    }
    pub fn gf(&self, k: &str, d: f32) -> f32 {
        self.gopt(k).and_then(Value::as_f64).map(|v| v as f32).unwrap_or(d)
    }
    pub fn gs(&self, k: &str, d: &str) -> String {
        self.gopt(k).and_then(Value::as_str).unwrap_or(d).to_string()
    }

    /// The projects the graph shows (by default those with notes, and the daily logs).
    pub fn shown_projects(&self) -> BTreeSet<String> {
        if !self.g.projects.is_empty() {
            return self.g.projects.clone();
        }
        let mut s: BTreeSet<String> = self.vault.project_counts().into_iter().filter(|(_, n)| *n > 0).map(|(p, _)| p).collect();
        s.insert("log".into());
        s.insert("home".into());
        s.insert("tools".into());
        s.insert(String::new());
        s
    }

    pub fn node_shown(&self, i: usize) -> bool {
        let Some(n) = self.graph.nodes.get(i) else { return false };
        if !self.shown_projects().contains(&n.project) {
            return false;
        }
        if !self.g.tags.is_empty() && n.project != "log" && !self.g.tags.iter().any(|t| n.tags.iter().any(|x| x.eq_ignore_ascii_case(t))) {
            return false;
        }
        if !self.gb("orphans", true) && n.links == 0 {
            return false;
        }
        true
    }

    fn fly_matches(&self) -> Vec<usize> {
        let q = self.g.fly.trim().to_lowercase();
        if q.is_empty() {
            return Vec::new();
        }
        let mut v: Vec<(i32, usize)> = self.vault.notes.iter().enumerate().filter_map(|(i, n)| search::fuzzy(&q, &n.rel).map(|(s, _)| (s, i))).collect();
        v.sort_by(|a, b| b.0.cmp(&a.0));
        let mut out: Vec<usize> = v.into_iter().take(4).map(|x| x.1).collect();
        for (i, n) in self.vault.notes.iter().enumerate() {
            if out.len() >= 5 {
                break;
            }
            if !out.contains(&i) && n.text.to_lowercase().contains(&q) {
                out.push(i);
            }
        }
        out
    }

    pub fn fly_list(&self) -> Vec<(usize, bool)> {
        let q = self.g.fly.trim().to_lowercase();
        self.fly_matches().into_iter().map(|i| (i, search::fuzzy(&q, &self.vault.notes[i].rel).is_some())).collect()
    }
}

pub fn plural(n: usize) -> &'static str {
    if n == 1 { "" } else { "s" }
}

fn shell_expand(p: &str) -> String {
    match p.strip_prefix("~/") {
        Some(r) => paths::home().join(r).to_string_lossy().to_string(),
        None => p.to_string(),
    }
}

pub fn save_task_pub(root: PathBuf, rel: String, text: String) -> Task<Msg> {
    save_task(root, rel, text)
}

/// CI's pictures ask for the keyboard focus to be drawn (--focus).
pub fn app_flag_focus() -> bool {
    app::arg("--focus").is_some()
}

fn save_task(root: PathBuf, rel: String, text: String) -> Task<Msg> {
    let r2 = rel.clone();
    Task::perform(
        off(move || {
            save::note(&root, &rel, &text).map(|_| {
                let mt = std::fs::metadata(root.join(&rel)).ok().and_then(|m| m.modified().ok()).and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs() as i64).unwrap_or(0);
                (text, mt)
            })
        }),
        move |r| {
            Msg::Saved(
                r2.clone(),
                match r {
                    Some(Ok(x)) => Ok(x),
                    Some(Err(e)) => Err(e.to_string()),
                    None => Err("it stopped".into()),
                },
            )
        },
    )
}

/// Continues a list on Enter: "- [ ] " after a task, "- " after a bullet, "1. " → "2. ".
fn continuation(line: &str) -> (String, bool) {
    if let Some((indent, _, text)) = md::task(line) {
        let pad = " ".repeat(indent);
        return (format!("{pad}- [ ] "), text.is_empty());
    }
    if let Some((indent, mark, text)) = md::bullet(line) {
        let pad = " ".repeat(indent);
        let next = match mark.trim_end_matches(['.', ')']).parse::<u32>() {
            Ok(n) => format!("{}{}", n + 1, &mark[mark.len() - 1..]),
            Err(_) => mark.clone(),
        };
        return (format!("{pad}{next} "), text.is_empty());
    }
    (String::new(), false)
}

pub fn update(a: &mut App, m: Msg) -> Task<Msg> {
    match m {
        Msg::Describe(dm) => {
            // one of Hearth Notes' buttons dragged in (drag and drop), pressed
            if let Some(m) = hearth_ui::studio::act(&dm).and_then(side::block_action) {
                return update(a, m);
            }
            return a.describe.update(dm).map(Msg::Describe);
        }
        Msg::Nothing => {}
        Msg::Theme => {
            a.base = hearth_ui::theme::load();
            a.p = themed(a.base, &a.prefs.theme);
            a.reduce_motion = reduce_motion();
            a.look_name = look_name();
        }
        Msg::Event(ev) => return event(a, ev),
        Msg::Pad(b) => return padview::press(a, b),
        Msg::Drag => return win::drag(),
        Msg::Maximize => return win::toggle_maximize(),
        Msg::Minimize => return win::minimize(),
        Msg::Close => {
            a.flush_now();
            return win::close();
        }
        Msg::Resize(d) => return win::resize(d),
        Msg::Loaded(l) => {
            let Some(l) = l.lock().ok().and_then(|mut x| x.take()) else { return Task::none() };
            a.vault = l.vault;
            a.graph = std::sync::Arc::new(l.graph);
            a.opened = l.opened;
            a.vs = VaultState::Ready;
            return first_open(a);
        }
        Msg::GraphReady(g) => a.graph = g,
        Msg::Rail(pg) => return a.rail(pg),
        Msg::Toggle(d) => {
            if !a.tree_open.remove(&d) {
                a.tree_open.insert(d);
            }
        }
        Msg::OpenNote(i) => {
            if let Some(n) = a.vault.notes.get(i) {
                let rel = n.rel.clone();
                return a.open_rel(&rel);
            }
        }
        Msg::OpenRel(r) => return a.open_rel(&r),
        Msg::OpenAtlas(w) => return a.open_atlas(&w),
        Msg::TabPick(i) => {
            let Some(t) = a.tabs.get(i).cloned() else { return Task::none() };
            a.tab = i;
            return match t {
                Tab::Note(r) if r.starts_with("atlas:") => a.open_atlas(&r[6..].to_string()),
                Tab::Note(r) if r == a.today_rel() => a.rail(Page::Today),
                Tab::Note(r) => a.open_rel(&r),
                Tab::Board => a.rail(Page::Board),
                Tab::Graph => a.rail(Page::Graph),
            };
        }
        Msg::TabClose(i) => {
            if i < a.tabs.len() {
                a.tabs.remove(i);
                if a.tab >= a.tabs.len() {
                    a.tab = a.tabs.len().saturating_sub(1);
                }
            }
        }
        Msg::SetMode(md_) => {
            let t = a.flush_doc();
            a.mode = md_;
            if let Some(d) = a.doc.as_mut() {
                d.cur = None;
                d.source = text_editor::Content::with_text(&d.text());
            }
            return t;
        }
        Msg::LineClick(i) => {
            if a.mode != Mode::Live {
                return Task::none();
            }
            let Some(d) = a.doc.as_mut() else { return Task::none() };
            if d.generated || i >= d.lines.len() {
                return Task::none();
            }
            d.cur = Some(i);
            d.ac = 0;
            return Task::batch([operation::focus(LINE_ID), operation::move_cursor_to_end(LINE_ID)]);
        }
        Msg::LineEdit(s) => {
            let Some(d) = a.doc.as_mut() else { return Task::none() };
            if let Some(c) = d.cur {
                d.fresh_empty = s.is_empty();
                if let Some(l) = d.lines.get_mut(c) {
                    *l = s;
                }
                d.ac = 0;
                return a.edited();
            }
        }
        Msg::LineEnter => {
            let ac = editor::autocomplete(a);
            let Some(d) = a.doc.as_mut() else { return Task::none() };
            let Some(c) = d.cur else { return Task::none() };
            if let Some(list) = ac {
                if let Some(target) = list.get(d.ac.min(list.len().saturating_sub(1))).cloned() {
                    d.lines[c] = md::complete_link(&d.lines[c], &target);
                    let t = a.edited();
                    return Task::batch([t, operation::move_cursor_to_end(LINE_ID)]);
                }
            }
            let (next, empty_item) = continuation(&d.lines[c]);
            if empty_item {
                // Enter on an empty list item ends the list.
                d.lines[c] = String::new();
            } else {
                d.lines.insert(c + 1, next);
                d.cur = Some(c + 1);
            }
            let t = a.edited();
            return Task::batch([t, operation::focus(LINE_ID), operation::move_cursor_to_end(LINE_ID)]);
        }
        Msg::LineAway => {
            if let Some(d) = a.doc.as_mut() {
                d.cur = None;
            }
        }
        Msg::TaskToggle(li) => {
            let Some(d) = a.doc.as_mut() else { return Task::none() };
            if d.generated {
                return Task::none();
            }
            if let Some(l) = d.lines.get(li).cloned() {
                if let Some((_, st, _)) = md::task(&l) {
                    let to = if st == 'x' { Status::Todo } else { Status::Done };
                    d.lines[li] = board::moved(&l, true, to);
                    return a.edited();
                }
            }
        }
        Msg::AcPick(target) => {
            let Some(d) = a.doc.as_mut() else { return Task::none() };
            if let Some(c) = d.cur {
                d.lines[c] = md::complete_link(&d.lines[c], &target);
                let t = a.edited();
                return Task::batch([t, operation::focus(LINE_ID), operation::move_cursor_to_end(LINE_ID)]);
            }
        }
        Msg::Source(act) => {
            let Some(d) = a.doc.as_mut() else { return Task::none() };
            let edit = act.is_edit();
            if edit && d.generated {
                return Task::none();
            }
            d.source.perform(act);
            if edit {
                let text = d.source.text();
                let nd = Doc::from(&d.rel, &text, false);
                d.lines = nd.lines;
                d.trailing_nl = nd.trailing_nl;
                return a.edited();
            }
        }
        Msg::SaveTick(g) => {
            let Some(d) = a.doc.as_mut() else { return Task::none() };
            if d.gen != g || !d.dirty {
                return Task::none();
            }
            d.dirty = false;
            let (rel, text) = (d.rel.clone(), d.text());
            return save_task(a.root.clone(), rel, text);
        }
        Msg::Saved(rel, r) => match r {
            Ok((text, mt)) => {
                let before = a.vault.find(&rel).map(|i| (a.vault.notes[i].links.clone(), a.vault.notes[i].embeds.clone()));
                a.vault.update(&rel, text, mt);
                if let Some(d) = a.doc.as_mut() {
                    if d.rel == rel && !d.dirty {
                        d.saved = true;
                        d.error.clear();
                    }
                }
                let after = a.vault.find(&rel).map(|i| (a.vault.notes[i].links.clone(), a.vault.notes[i].embeds.clone()));
                let mut tasks = vec![a.refresh_sync()];
                if before != after || a.graph.nodes.len() != a.vault.notes.len() {
                    let v = a.vault.clone();
                    tasks.push(Task::perform(off(move || graph::build(&v)), |g| g.map(|g| Msg::GraphReady(std::sync::Arc::new(g))).unwrap_or(Msg::Nothing)));
                }
                return Task::batch(tasks);
            }
            Err(e) => {
                if let Some(d) = a.doc.as_mut() {
                    if d.rel == rel {
                        d.dirty = true;
                        d.error = e.clone();
                    }
                }
                a.say(format!("Couldn't save {rel}: {e}. What you typed is still here."), true);
            }
        },
        Msg::Cal(step) => {
            let (y, mth) = a.cal;
            let n = (y * 12 + mth as i32 - 1) + step;
            a.cal = (n.div_euclid(12), (n.rem_euclid(12) + 1) as u32);
            if step == 0 {
                a.cal = (a.today.0, a.today.1);
            }
        }
        Msg::CalDay(d) => {
            let rel = day::log_path(d);
            if a.vault.find(&rel).is_some() || d == a.today {
                a.day = d;
                let text = a.vault.find(&rel).map(|i| a.vault.notes[i].text.clone()).unwrap_or_default();
                let t = a.flush_doc();
                a.doc = Some(Doc::from(&rel, &text, false));
                a.ensure_tab(Tab::Note(rel));
                a.page = Page::Today;
                a.track = trackers::load(&a.vault_key(), &day::ymd(d));
                return t;
            }
        }
        Msg::Outline(line) => {
            if a.mode == Mode::Live {
                return update(a, Msg::LineClick(line));
            }
        }
        Msg::QuickKind(k) => a.quick.kind = k,
        Msg::QuickWhat(s) => a.quick.what = s,
        Msg::QuickAt(s) => a.quick.at = s,
        Msg::QuickWait(s) => a.quick.waiting = s,
        Msg::QuickAdd => return today::add(a),
        Msg::DayStep(n) => {
            let d = day::add_days(a.day, n);
            return update(a, Msg::CalDay(d));
        }
        Msg::Productivity(v) => match trackers::set_productivity(&a.vault_key(), &day::ymd(a.day), v) {
            Ok(d) => a.track = d,
            Err(e) => a.say(e, true),
        },
        Msg::Habit(h) => {
            let on = !a.track.habits.get(&h).copied().unwrap_or(false);
            match trackers::set_habit(&a.vault_key(), &day::ymd(a.day), &h, on) {
                Ok(d) => a.track = d,
                Err(e) => a.say(e, true),
            }
        }
        Msg::TodayAction(w) => return today::action(a, w),
        Msg::BoardGroup(g) => a.group = g,
        Msg::BoardProject(p) => {
            a.board_project = p;
            a.board_menu = 0;
        }
        Msg::BoardTag(t) => {
            a.board_tag = t;
            a.board_menu = 0;
        }
        Msg::BoardMenu(n) => a.board_menu = if a.board_menu == n { 0 } else { n },
        Msg::CardMove(i, to) => return boardv::move_card(a, i, to),
        Msg::CardOpen(i) => {
            let cards = a.cards();
            if let Some(c) = cards.get(i) {
                let rel = a.vault.notes[c.note].rel.clone();
                let line = c.line;
                let t = a.open_rel(&rel);
                a.page = if rel == a.today_rel() { Page::Today } else { Page::Files };
                if let Some(d) = a.doc.as_mut() {
                    if a.mode == Mode::Live {
                        d.cur = Some(line);
                    }
                }
                return t;
            }
        }
        Msg::CardTick(i) => {
            let cards = a.cards();
            if let Some(c) = cards.get(i) {
                let to = if c.status == Status::Done { Status::Todo } else { Status::Done };
                return boardv::move_card(a, i, to);
            }
        }
        Msg::CardFocus(i) => a.card_focus = i,
        Msg::CardDrag(i) => a.card_drag = Some(i),
        Msg::CardDrop(col) => {
            if let Some(i) = a.card_drag.take() {
                if let Some(to) = board::STATUSES.get(col) {
                    return boardv::move_card(a, i, *to);
                }
            }
        }
        Msg::AddCard(st) => return boardv::add_card(a, st),
        Msg::SearchText(s) => {
            a.query.text = s;
            a.run_search();
        }
        Msg::SearchCase => {
            a.query.case = !a.query.case;
            a.run_search();
        }
        Msg::SearchExact => {
            a.query.exact = !a.query.exact;
            a.run_search();
        }
        Msg::SearchFolder(f) => {
            match f {
                Some(f) => {
                    if !a.query.folders.contains(&f) {
                        a.query.folders.push(f);
                    }
                }
                None => a.query.folders.clear(),
            }
            a.filter_menu = 0;
            a.run_search();
        }
        Msg::SearchTag(t) => {
            a.query.tag = t;
            a.filter_menu = 0;
            a.run_search();
        }
        Msg::SearchDays(d) => {
            a.query.days = d;
            a.filter_menu = 0;
            a.run_search();
        }
        Msg::SearchFilterMenu(n) => a.filter_menu = if a.filter_menu == n { 0 } else { n },
        Msg::SearchOpen(i) => {
            a.hit_focus = i;
            a.match_at = 0;
            if let Some(h) = a.hits.get(i) {
                let rel = a.vault.notes[h.note].rel.clone();
                let t = a.open_rel(&rel);
                a.page = Page::Search;
                a.mode = Mode::Read;
                return t;
            }
        }
        Msg::MatchStep(d) => {
            let n = searchv::matches_in_open(a);
            if n > 0 {
                a.match_at = ((a.match_at as i64 + d as i64).rem_euclid(n as i64)) as usize;
            }
        }
        Msg::QsOpen => {
            a.qs = Some((String::new(), 0));
            return operation::focus(QS_ID);
        }
        Msg::QsText(s) => a.qs = Some((s, 0)),
        Msg::QsPick(i, _new_tab) => {
            let Some((q, _)) = a.qs.take() else { return Task::none() };
            let list = search::switcher(&a.vault, &q, 4);
            if let Some(s) = list.get(i) {
                let rel = a.vault.notes[s.note].rel.clone();
                return a.open_rel(&rel);
            }
        }
        Msg::QsMake => {
            let Some((q, _)) = a.qs.take() else { return Task::none() };
            let name = newitem::suggest(&q);
            if name.is_empty() {
                return Task::none();
            }
            let rel = format!("{name}.md");
            if a.vault.find(&rel).is_none() {
                let text = format!("# {}\n\n", q.trim());
                a.vault.update(&rel, text.clone(), a.now);
                let t = save_task(a.root.clone(), rel.clone(), text);
                let o = a.open_rel(&rel);
                return Task::batch([t, o]);
            }
            return a.open_rel(&rel);
        }
        Msg::QsClose => a.qs = None,
        Msg::GMode(mo) => {
            a.g.mode = mo.clone();
            a.g.cam = Cam::default();
            a.page = Page::Graph;
            a.wake();
            if let Err(e) = prefs::set("graphMode", json!(mo)).map(|p| a.prefs = p) {
                a.say(e, true);
            }
        }
        Msg::GHover(h) => {
            if a.g.hover != h {
                a.g.hover = h;
                a.wake();
            }
        }
        Msg::GClick(c) => {
            a.g.sel = c;
            a.g.burst_until = a.g.t + 3.0;
            a.wake();
        }
        Msg::GPan(dx, dy) => {
            a.g.cam.x += dx;
            a.g.cam.y += dy;
            a.wake();
        }
        Msg::GZoom(f) => {
            a.g.cam.zoom = (a.g.cam.zoom * f).clamp(0.3, 6.0);
            a.wake();
        }
        Msg::GZoomTo(z_) => {
            a.g.cam.zoom = z_.clamp(0.3, 6.0);
            a.wake();
        }
        Msg::GTurn(dy, dp) => {
            a.g.cam.yaw += dy;
            a.g.cam.pitch = (a.g.cam.pitch + dp).clamp(-1.2, 1.2);
            a.wake();
        }
        Msg::GReset => {
            a.g.cam = Cam::default();
            a.wake();
        }
        Msg::GPause => a.g.paused = !a.g.paused,
        Msg::GFull => a.g.full = !a.g.full,
        Msg::FlyText(s) => {
            a.g.fly = s;
            a.g.fly_sel = 0;
            a.g.fly_closed = false;
        }
        Msg::FlyGo(i) => {
            gview::fly_to(a, i);
            a.g.fly_closed = true;
        }
        Msg::GOpen(i) => {
            if let Some(n) = a.vault.notes.get(i) {
                let rel = n.rel.clone();
                return a.open_rel(&rel);
            }
        }
        Msg::GOnly(i) => {
            a.g.sel = Some(i);
            a.g.local = true;
        }
        Msg::GProject(p) => {
            let mut s = a.shown_projects();
            if !s.remove(&p) {
                s.insert(p);
            }
            a.g.projects = s;
        }
        Msg::GTagDrop(t) => a.g.tags.retain(|x| x != &t),
        Msg::GTagAdd => {
            a.g.tag_typing = Some(String::new());
            return operation::focus("notes-graph-tag");
        }
        Msg::GTagText(s) => a.g.tag_typing = Some(s),
        Msg::GTagDone => {
            if let Some(t) = a.g.tag_typing.take() {
                let t = t.trim().trim_start_matches('#').to_string();
                if !t.is_empty() && !a.g.tags.contains(&t) {
                    a.g.tags.push(t);
                }
            }
        }
        Msg::GOpt(k, v) => {
            a.set_graph(&k, v);
            a.wake();
        }
        Msg::GCustomise(on) => a.g.customise = on,
        Msg::GLocal(on) => a.g.local = on,
        Msg::GPickFocus(i) => a.g.picker_focus = i,
        Msg::Frame => {
            let now = Instant::now();
            let dt = a.g.last_frame.map(|l| now.duration_since(l).as_secs_f32()).unwrap_or(0.0).min(0.1);
            a.g.last_frame = Some(now);
            a.g.t += dt;
            if !a.animating() {
                a.g.last_frame = None;
            }
        }
        Msg::LookSave => return gview::look_save(a),
        Msg::LookShare => return gview::look_share(a),
        Msg::LookOpen => return gview::look_open(a),
        Msg::LookFile(p) => return gview::look_file(a, p),
        Msg::LookApply(n) => return gview::look_apply(a, &n),
        Msg::LookMenu(p) => a.g.look_menu = p,
        Msg::ProjectLook(p, l) => {
            a.g.look_menu = None;
            match prefs::set_in("projectLooks", &p, json!(l)) {
                Ok(pr) => a.prefs = pr,
                Err(e) => a.say(e, true),
            }
        }
        Msg::SyncSel(i) => {
            a.sync.sel = i;
            if let Some(c) = a.sync.changes.get(i) {
                let (root, path) = (a.root.clone(), c.path.clone());
                return Task::perform(off(move || git::diff(&root, &path)), |d| Msg::Diff(d.unwrap_or_default()));
            }
        }
        Msg::Diff(d) => a.sync.diff = d,
        Msg::SyncMsg(s) => a.sync.msg = s,
        Msg::SyncNow => return a.start_sync(false, true),
        Msg::SyncPull => return a.start_sync(false, false),
        Msg::SecretsGo => {
            a.sync.secrets.clear();
            return a.start_sync(true, true);
        }
        Msg::SecretsCancel => {
            a.sync.secrets.clear();
            a.sync.said = "Nothing was committed. Change the note, then Sync now.".into();
        }
        Msg::SyncDone(step) => {
            a.sync.busy = false;
            a.now = paths::now();
            a.sync.next_auto = a.now + a.prefs.sync_every as i64 * 60;
            let mut tasks = vec![a.refresh_sync()];
            match *step {
                Step::Done { committed, pulled, pushed } => {
                    a.sync.said = match (committed, pulled, pushed) {
                        (_, _, true) => "Pushed to GitHub".into(),
                        (_, true, false) => "GitHub's changes are here".into(),
                        _ => "Up to date".into(),
                    };
                    a.sync.bad = false;
                    if committed {
                        a.sync.msg.clear();
                    }
                    if pulled {
                        tasks.push(a.load_vault());
                    }
                    a.sync.check = None;
                    tasks.push(a.run_check());
                }
                Step::Untidy(p) => {
                    a.sync.said = format!("Fix the {} problem{} first", p.len(), plural(p.len()));
                    a.sync.bad = true;
                    a.sync.check = Some(Check::Untidy(p));
                    a.sync.checked_at = a.now;
                    a.page = if a.page == Page::Sync || !a.pad { Page::Sync } else { a.page };
                }
                Step::CheckFailed(e) => {
                    a.sync.said = format!("The workspace check couldn't run, so nothing was pushed: {e}");
                    a.sync.bad = true;
                    a.sync.check = Some(Check::Failed(e));
                }
                Step::Secrets(h) => {
                    a.sync.said = "Something looks like a secret: nothing was committed".into();
                    a.sync.bad = true;
                    a.sync.secrets = h;
                    a.page = Page::Sync;
                }
                Step::Conflict(files) => {
                    a.sync.said = String::new();
                    a.page = Page::Conflict;
                    tasks.push(a.load_conflict(files, 0));
                }
                Step::NotRepo => {
                    a.sync.said = "This folder isn't a Git repository, so it stays on this PC.".into();
                    a.sync.bad = true;
                }
                Step::Failed(e) => {
                    let signed = matches!(a.sync.auth, Some(Auth::None));
                    a.sync.said = if signed { format!("{e}. {}", git::SIGN_IN_HELP) } else { e };
                    a.sync.bad = true;
                }
            }
            return Task::batch(tasks);
        }
        Msg::SyncInfo(i) => {
            let first = a.sync.changes.is_empty() && !i.changes.is_empty();
            a.sync.is_repo = i.is_repo;
            a.sync.changes = i.changes;
            a.sync.repo = i.repo;
            a.sync.who = i.who;
            a.sync.last_sync = i.last_sync;
            a.sync.conflicts = i.conflicts.clone();
            if a.sync.next_auto == 0 {
                a.sync.next_auto = a.now + a.prefs.sync_every as i64 * 60;
            }
            if a.sync.sel >= a.sync.changes.len() {
                a.sync.sel = 0;
            }
            let mut tasks = Vec::new();
            if (first || a.sync.diff.is_empty()) && !a.sync.changes.is_empty() {
                let s = a.sync.sel;
                tasks.push(update(a, Msg::SyncSel(s)));
            }
            if !i.conflicts.is_empty() && a.conflict.is_none() && (a.page == Page::Conflict || a.show == "conflict") {
                tasks.push(a.load_conflict(i.conflicts, 0));
            }
            if a.sync.auth.is_none() {
                let root = a.root.clone();
                tasks.push(Task::perform(off(move || git::auth(&root)), |r| Msg::Authed(r.unwrap_or(Auth::None))));
            }
            if a.page == Page::Sync && a.sync.check.is_none() {
                tasks.push(a.run_check());
            }
            return Task::batch(tasks);
        }
        Msg::CheckAgain => {
            a.sync.check = None;
            return a.run_check();
        }
        Msg::Checked(c) => {
            a.sync.check = Some(c);
            a.sync.checked_at = paths::now();
        }
        Msg::Fix(i) => {
            if let Some(Check::Untidy(p)) = &a.sync.check {
                if let Some(fix) = p.get(i).and_then(|x| x.fix.clone()) {
                    let root = a.root.clone();
                    return Task::perform(off(move || check::apply(&root, &fix, "draft")), |r| Msg::Fixed(r.unwrap_or(Err("it stopped".into()))));
                }
            }
        }
        Msg::Fixed(r) => {
            if let Err(e) = r {
                a.say(e, true);
            }
            a.sync.check = None;
            return Task::batch([a.run_check(), a.refresh_sync(), a.load_vault()]);
        }
        Msg::SyncEvery(n) => return a.set_pref("syncEvery", json!(n)),
        Msg::AutoSync => {
            a.now = paths::now();
            if a.prefs.sync_every == 0 || a.sync.busy || !a.sync.is_repo || game_running() {
                return Task::none();
            }
            if let Some(Check::Untidy(_)) = a.sync.check {
                return a.run_check();
            }
            if a.sync.msg.trim().is_empty() {
                let c = a.central_now();
                a.sync.msg = format!("Notes, {} {}", day::short(c.day), day::clock(c.hour, c.min));
            }
            return a.start_sync(false, true);
        }
        Msg::ConflictLoaded(c) => {
            a.conflict = Some(*c);
            a.page = Page::Conflict;
        }
        Msg::ConflictPick(p) => {
            let Some(c) = a.conflict.as_mut() else { return Task::none() };
            c.pick = Some(p);
            let Some(rel) = c.files.get(c.i).cloned() else { return Task::none() };
            let root = a.root.clone();
            let ct = a.central_now();
            let stamp = format!("{}-{:02}{:02}", day::ymd(ct.day), ct.hour, ct.min);
            return Task::perform(off(move || conflict::settle(&root, &rel, p, &stamp)), |r| Msg::Settled(r.unwrap_or(Err("it stopped".into()))));
        }
        Msg::Settled(r) => match r {
            Ok(done) => {
                let left: Vec<String> = a.conflict.as_ref().map(|c| c.files.iter().enumerate().filter(|(i, _)| *i != c.i).map(|(_, f)| f.clone()).collect()).unwrap_or_default();
                if done || left.is_empty() {
                    a.conflict = None;
                    a.page = Page::Sync;
                    a.sync.said = "Both versions are kept as copies. Sync now to push it.".into();
                    a.sync.bad = false;
                    return Task::batch([a.refresh_sync(), a.load_vault()]);
                }
                return a.load_conflict(left, 0);
            }
            Err(e) => a.say(e, true),
        },
        Msg::ConflictLater => {
            a.page = Page::Files;
        }
        Msg::ConflictStep(d) => {
            if let Some(c) = &a.conflict {
                let n = c.files.len().max(1);
                let i = ((c.i as i64 + d as i64).rem_euclid(n as i64)) as usize;
                return a.load_conflict(c.files.clone(), i);
            }
        }
        Msg::OpenKept => {
            let k = paths::kept();
            let _ = std::fs::create_dir_all(&k);
            hearth_ui::run::detached(&["xdg-open", &k.to_string_lossy()]);
        }
        Msg::NewOpen => {
            a.new_item = Some(NewItem { project: 0, kind: 1, name: String::new(), status: 0, release: "not yet assigned".into(), error: String::new() });
            return operation::focus(NAME_ID);
        }
        Msg::NewProject(i) => {
            if let Some(n) = a.new_item.as_mut() {
                n.project = i;
            }
        }
        Msg::NewKind(i) => {
            if let Some(n) = a.new_item.as_mut() {
                n.kind = i;
            }
        }
        Msg::NewName(s) => {
            if let Some(n) = a.new_item.as_mut() {
                n.name = s;
                n.error.clear();
            }
        }
        Msg::NewSuggested => {
            if let Some(n) = a.new_item.as_mut() {
                n.name = newitem::suggest(&n.name);
            }
        }
        Msg::NewStatus(i) => {
            if let Some(n) = a.new_item.as_mut() {
                n.status = i;
            }
        }
        Msg::NewRelease(r) => {
            if let Some(n) = a.new_item.as_mut() {
                n.release = r;
            }
        }
        Msg::NewMake => {
            let Some(n) = a.new_item.clone() else { return Task::none() };
            let root = a.root.clone();
            let c = a.central_now();
            let (d, hm) = (day::ymd(a.today), day::hhmm(c));
            let (pj, kd) = (newitem::PROJECTS[n.project].to_string(), newitem::KINDS[n.kind].to_string());
            let st = newitem::STATUSES[n.status].to_string();
            return Task::perform(off(move || newitem::make(&root, &pj, &kd, &n.name, &st, &n.release, &d, &hm)), |r| Msg::NewMade(r.unwrap_or(Err("it stopped".into()))));
        }
        Msg::NewMade(r) => match r {
            Ok(m) => {
                a.new_item = None;
                a.say(format!("Made {}", m.item), false);
                let readme = m.readme.clone();
                let root = a.root.clone();
                return Task::batch([
                    Task::perform(
                        off(move || {
                            let v = Vault::scan(&root);
                            let g = graph::build(&v);
                            Loaded { vault: v, graph: g, opened: load_opened() }
                        }),
                        |r| Msg::Loaded(std::sync::Arc::new(std::sync::Mutex::new(r))),
                    ),
                    Task::perform(hearth_ui::run::after(300), move |_| Msg::OpenRel(readme.clone())),
                    a.refresh_sync(),
                ]);
            }
            Err(e) => {
                if let Some(n) = a.new_item.as_mut() {
                    n.error = e;
                }
            }
        },
        Msg::NewCancel => a.new_item = None,
        Msg::SetSec(i) => {
            a.settings_sec = i;
            return settingsv::scroll_to(i);
        }
        Msg::Set(k, v) => return a.set_pref(k, v),
        Msg::DayStartStep(d) => {
            let m = (a.prefs.day_start as i32 + d * 30).rem_euclid(24 * 60) as u32;
            return a.set_pref("dayStart", json!(format!("{:02}:{:02}", m / 60, m % 60)));
        }
        Msg::CheckSignIn => {
            a.sync.auth = None;
            let root = a.root.clone();
            return Task::perform(off(move || git::auth(&root)), |r| Msg::Authed(r.unwrap_or(Auth::None)));
        }
        Msg::Authed(au) => a.sync.auth = Some(au),
        Msg::ChangeVault => {
            return Task::perform(off(pick_folder), |r| Msg::VaultPicked(r.flatten()));
        }
        Msg::VaultPicked(p) => {
            if let Some(p) = p {
                let t = a.set_pref("vault", json!(p));
                a.root = PathBuf::from(&p);
                a.vs = VaultState::Reading;
                a.doc = None;
                a.tabs.clear();
                a.sync = SyncUi::default();
                return Task::batch([t, a.load_vault(), a.refresh_sync()]);
            }
        }
        Msg::CloneNow => {
            a.vs = VaultState::Cloning;
            let dest = a.root.clone();
            return Task::perform(
                off(move || {
                    let au = git::auth(&dest);
                    if !au.signed_in() {
                        return Err(git::SIGN_IN_HELP.to_string());
                    }
                    let slug = std::env::var("HEARTH_NOTES_CLONE_FROM").unwrap_or_else(|_| "Hearth-Organization/hearth-workspace".into());
                    git::clone(&slug, &dest, &au)
                }),
                |r| Msg::Cloned(r.unwrap_or(Err("it stopped".into()))),
            );
        }
        Msg::Cloned(r) => match r {
            Ok(()) => {
                a.vs = VaultState::Reading;
                return Task::batch([a.load_vault(), a.refresh_sync()]);
            }
            Err(e) => a.vs = VaultState::CloneFailed(e),
        },
        Msg::PadPage(p) => a.pad_page = p,
        Msg::PadNote(i) => a.pad_note = i,
        Msg::OskKey(r, c) => return padview::osk_key(a, r, c),
        Msg::OskKind(k) => a.osk.kind = k,
        Msg::OskToggle => a.osk.open = !a.osk.open,
    }
    Task::none()
}

/// The folder picker (Settings › Repository › Change…): Hearth's file chooser, else KDE's or GTK's.
fn pick_folder() -> Option<String> {
    for cmd in [vec!["hearth-file-chooser", "--folder"], vec!["kdialog", "--getexistingdirectory"], vec!["zenity", "--file-selection", "--directory"]] {
        let o = git::run(std::process::Command::new(cmd[0]).args(&cmd[1..]), 600);
        if o.ok {
            let p = o.out.trim().to_string();
            if !p.is_empty() && std::path::Path::new(&p).is_dir() {
                return Some(p);
            }
        }
    }
    None
}

/// What opens first: --note, else the newest README the person opened, else Home.
fn first_open(a: &mut App) -> Task<Msg> {
    let mut tasks = Vec::new();
    let first = app::arg("--note").or_else(|| a.opened.iter().max_by_key(|x| x.1).map(|x| x.0.clone()).filter(|r| a.vault.find(r).is_some())).or_else(|| a.vault.find("INDEX.md").map(|_| "INDEX.md".to_string()));
    let page = a.page;
    if let Some(r) = first {
        tasks.push(a.open_rel_as(&r, false));
    }
    if let Some(f) = app::arg("--open-folders") {
        a.tree_open = f.split(',').filter(|x| !x.is_empty()).map(str::to_string).collect();
    }
    if let Some(t) = app::arg("--osk-text") {
        a.osk.text = t;
    }
    a.g.fly_closed = app::flag("--fly-closed");
    if app::flag("--local") {
        a.g.local = true;
    }
    let tr = a.today_rel();
    if !a.tabs.contains(&Tab::Note(tr.clone())) {
        a.tabs.push(Tab::Note(tr));
    }
    if !a.tabs.contains(&Tab::Board) {
        a.tabs.push(Tab::Board);
    }
    a.page = page;
    match page {
        Page::Files => {}
        p => tasks.push(a.rail(p)),
    }
    if let Some(m) = app::arg("--mode") {
        a.mode = Mode::of(&m);
    }
    if let Some(l) = app::arg("--line").and_then(|s| s.parse::<usize>().ok()) {
        if let Some(d) = a.doc.as_mut() {
            d.cur = Some(l);
        }
    }
    if let Some(q) = app::arg("--search") {
        a.query.text = q;
        if let Some(f) = app::arg("--search-folder") {
            a.query.folders = f.split(',').map(str::to_string).collect();
        }
        if app::arg("--search-days").is_some() {
            a.query.days = Some(7);
        }
        a.run_search();
        if app::arg("--search-open").is_some() {
            if let Some(h) = a.hits.first() {
                let rel = a.vault.notes[h.note].rel.clone();
                tasks.push(a.open_rel_as(&rel, false));
                a.page = Page::Search;
                a.mode = Mode::Read;
            }
            a.match_at = 1;
        }
    }
    if let Some(q) = app::arg("--quick-switcher") {
        a.qs = Some((q, 0));
    }
    if let Some(f) = app::arg("--fly") {
        a.g.fly = f;
    }
    if let Some(s) = app::arg("--select") {
        if let Some(i) = a.vault.find(&s) {
            a.g.sel = Some(i);
            a.g.hover = Some(i);
            if app::flag("--flew") {
                a.g.flew = Some(i);
            }
        }
    }
    if let Some(z_) = app::arg("--zoom").and_then(|s| s.parse::<f32>().ok()) {
        a.g.cam.zoom = z_;
    }
    match a.show.as_str() {
        "new-item" => {
            tasks.push(update(a, Msg::NewOpen));
            if let Some(n) = app::arg("--name") {
                tasks.push(update(a, Msg::NewName(n)));
            }
        }
        "customise" => a.g.customise = true,
        "sync" => tasks.push(a.run_check()),
        _ => {}
    }
    if let Some(q) = app::arg("--quick") {
        let mut it = q.split('|');
        a.quick.what = it.next().unwrap_or("").to_string();
        a.quick.at = it.next().unwrap_or("").to_string();
        a.quick.waiting = it.next().unwrap_or("").to_string();
    }
    if let Some(m) = app::arg("--commit-message") {
        a.sync.msg = m;
    }
    if let Some(n) = app::arg("--focus") {
        a.focus = n;
    }
    if a.page == Page::Today {
        tasks.push(operation::focus(QUICK_ID));
    }
    a.wake();
    Task::batch(tasks)
}

fn event(a: &mut App, ev: iced::Event) -> Task<Msg> {
    use iced::keyboard::Event as K;
    match ev {
        iced::Event::Window(iced::window::Event::Focused) => {
            a.focused = true;
            a.now = paths::now();
        }
        iced::Event::Window(iced::window::Event::Unfocused) => a.focused = false,
        iced::Event::Window(iced::window::Event::CloseRequested) => {
            a.flush_now();
            return win::close();
        }
        iced::Event::Keyboard(K::KeyPressed { key, modifiers, .. }) => {
            if a.pad && !a.pad_fixed && pad::desk_in_use() {
                a.pad = false;
                pad::PAD_LOOK.store(false, std::sync::atomic::Ordering::Relaxed);
            }
            let ctrl = modifiers.control();
            let shift = modifiers.shift();
            match key.as_ref() {
                Key::Character("o") if ctrl => return update(a, Msg::QsOpen),
                Key::Character("f") | Key::Character("F") if ctrl && shift => return a.rail(Page::Search),
                Key::Character("l") | Key::Character("L") if ctrl && shift => {
                    let t = a.rail(Page::Today);
                    return Task::batch([t, operation::focus(QUICK_ID)]);
                }
                Key::Character("f") if ctrl => {
                    if matches!(a.page, Page::Graph) {
                        return operation::focus(FLY_ID);
                    }
                    return a.rail(Page::Search);
                }
                Key::Character("e") if ctrl => {
                    let next = match a.mode {
                        Mode::Live => Mode::Source,
                        Mode::Source => Mode::Read,
                        Mode::Read => Mode::Live,
                    };
                    return update(a, Msg::SetMode(next));
                }
                Key::Character("s") if ctrl => return a.flush_doc(),
                Key::Named(Named::Escape) => {
                    if a.qs.is_some() {
                        a.qs = None;
                    } else if a.new_item.is_some() {
                        a.new_item = None;
                    } else if a.g.customise {
                        a.g.customise = false;
                    } else if let Some(d) = a.doc.as_mut().filter(|d| d.cur.is_some()) {
                        d.cur = None;
                    } else if a.page != Page::Files {
                        a.page = Page::Files;
                    }
                }
                Key::Named(Named::ArrowUp) | Key::Named(Named::ArrowDown) => {
                    let down = matches!(key.as_ref(), Key::Named(Named::ArrowDown));
                    if let Some((q, sel)) = a.qs.as_mut() {
                        let n = search::switcher(&a.vault, q, 4).len();
                        if n > 0 {
                            *sel = if down { (*sel + 1) % n } else { (*sel + n - 1) % n };
                        }
                        return Task::none();
                    }
                    if a.page == Page::Graph && !a.g.fly.is_empty() {
                        let n = a.fly_matches().len().max(1);
                        a.g.fly_sel = if down { (a.g.fly_sel + 1) % n } else { (a.g.fly_sel + n - 1) % n };
                        return Task::none();
                    }
                    let ac = editor::autocomplete(a).map(|l| l.len()).unwrap_or(0);
                    if let Some(d) = a.doc.as_mut() {
                        if let Some(c) = d.cur {
                            if ac > 0 {
                                d.ac = if down { (d.ac + 1) % ac } else { (d.ac + ac - 1) % ac };
                                return Task::none();
                            }
                            let n = if down { (c + 1).min(d.lines.len().saturating_sub(1)) } else { c.saturating_sub(1) };
                            d.cur = Some(n);
                            return Task::batch([operation::focus(LINE_ID), operation::move_cursor_to_end(LINE_ID)]);
                        }
                    }
                }
                Key::Named(Named::Backspace) => {
                    if let Some(d) = a.doc.as_mut() {
                        if let Some(c) = d.cur {
                            if std::mem::take(&mut d.fresh_empty) {
                                return Task::none();
                            }
                            if d.lines.get(c).map(|l| l.is_empty()).unwrap_or(false) && d.lines.len() > 1 && c > 0 {
                                d.lines.remove(c);
                                d.cur = Some(c - 1);
                                let t = a.edited();
                                return Task::batch([t, operation::focus(LINE_ID), operation::move_cursor_to_end(LINE_ID)]);
                            }
                        }
                    }
                }
                Key::Named(Named::Enter) => {
                    if let Some((q, sel)) = a.qs.clone() {
                        if shift {
                            return update(a, Msg::QsMake);
                        }
                        let _ = q;
                        return update(a, Msg::QsPick(sel, ctrl));
                    }
                    if a.page == Page::Graph && !a.g.fly.is_empty() {
                        if let Some(&i) = a.fly_matches().get(a.g.fly_sel) {
                            return update(a, Msg::FlyGo(i));
                        }
                    }
                }
                Key::Named(Named::Tab) => {
                    if let Some(d) = a.doc.as_ref() {
                        if d.cur.is_some() {
                            if let Some(list) = editor::autocomplete(a) {
                                if let Some(t) = list.get(d.ac).cloned() {
                                    return update(a, Msg::AcPick(t));
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        iced::Event::Mouse(_) => {
            if a.pad && !a.pad_fixed && pad::desk_in_use() {
                a.pad = false;
                pad::PAD_LOOK.store(false, std::sync::atomic::Ordering::Relaxed);
            }
        }
        _ => {}
    }
    Task::none()
}

fn events(ev: iced::Event, _status: iced::event::Status, _id: iced::window::Id) -> Option<Msg> {
    match &ev {
        iced::Event::Keyboard(iced::keyboard::Event::KeyPressed { .. }) => Some(Msg::Event(ev)),
        iced::Event::Window(iced::window::Event::Focused | iced::window::Event::Unfocused | iced::window::Event::CloseRequested) => Some(Msg::Event(ev)),
        iced::Event::Mouse(iced::mouse::Event::ButtonPressed(_)) => Some(Msg::Event(ev)),
        _ => None,
    }
}

fn subscription(a: &App) -> Subscription<Msg> {
    let mut subs = vec![Subscription::run(hearth_ui::theme::watch).map(|_| Msg::Theme), iced::event::listen_with(events), Subscription::run(pad::presses).map(Msg::Pad)];
    if a.animating() {
        // 30 frames a second on the graphics card, 15 with the CPU (the light fallback).
        let ms = if a.cpu { 66 } else { 33 };
        subs.push(iced::time::every(Duration::from_millis(ms)).map(|_| Msg::Frame));
    }
    if a.prefs.sync_every > 0 && a.sync.is_repo && !app::flag("--no-auto-sync") {
        subs.push(iced::time::every(Duration::from_secs(a.prefs.sync_every as u64 * 60)).map(|_| Msg::AutoSync));
    }
    Subscription::batch(subs)
}

fn view(a: &App) -> iced::Element<'_, Msg> {
    if a.pad {
        return padview::view(a);
    }
    hearth_ui::describe::with_bar(side::window(a), &a.p, &a.describe, Msg::Describe)
}

fn main() -> iced::Result {
    if app::handle_cli(
        "hearth-notes-app",
        "Hearth Notes: Markdown notes in a folder (the hearth-workspace repository by default), synced with Git.\n  --page files|search|today|board|graph|modes|sync|conflict|settings\n  --note PATH             open this note first\n  --graph-mode MODE       classic, galaxy, solar, blackhole, ember, railway, nightsky, neural, city, river\n  --controller / --mouse  the controller layout, or the other, whatever is used\n  --still                 the graph drawn still (as with Reduce motion)",
    ) {
        return Ok(());
    }
    iced::application(App::new, update, view)
        .title("Hearth Notes")
        .subscription(subscription)
        .theme(|a: &App| app::iced_theme(&a.p))
        .window(app::window("hearth-notes", (1280.0, 800.0), (960.0, 620.0)))
        .default_font(app::INTER)
        .run()
}
