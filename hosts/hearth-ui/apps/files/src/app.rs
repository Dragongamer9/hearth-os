//! Hearth Files' state and what each click, key and answer does (FileManager.qml's logic, ported). The window
//! is drawn by view.rs; the menus are built and run in menus.rs.

use crate::backend::{self, a, Line, Reply, Run};
use crate::drive::{self, CData, Ctx};
use crate::model::{self, base, parent_of, Entry, Listing, APPS_PAGE, HOME_PAGE, THIS_PC};
use crate::thumbs::Thumb;
use hearth_ui::app::{self as happ, win};
use hearth_ui::Pal;
use iced::keyboard::{key::Named, Event as KEvent, Key, Modifiers};
use iced::widget::image::Handle;
use iced::widget::operation;
use iced::{Point, Subscription, Task};
use serde_json::{json, Map, Value};
use std::cell::Cell;
use std::collections::{HashMap, HashSet};

pub const ROW_H: f32 = 26.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum View {
    XLarge,
    Large,
    Grid,
    Small,
    Tiles,
    Compact,
    Details,
}

impl View {
    pub fn id(self) -> &'static str {
        match self {
            View::XLarge => "xlarge",
            View::Large => "large",
            View::Grid => "grid",
            View::Small => "small",
            View::Tiles => "tiles",
            View::Compact => "compact",
            View::Details => "details",
        }
    }
    pub fn from(s: &str) -> Option<View> {
        Some(match s {
            "xlarge" => View::XLarge,
            "large" => View::Large,
            "grid" => View::Grid,
            "small" => View::Small,
            "tiles" => View::Tiles,
            "compact" => View::Compact,
            "details" | "list" => View::Details,
            _ => return None,
        })
    }
    /// A cell's size in the icon views (width, height).
    pub fn cell(self) -> (f32, f32) {
        match self {
            View::XLarge => (250.0, 214.0),
            View::Large => (143.0, 122.0),
            View::Grid => (104.0, 100.0),
            View::Small => (190.0, 30.0),
            View::Tiles => (260.0, 58.0),
            View::Compact => (230.0, 26.0),
            View::Details => (0.0, ROW_H),
        }
    }
}

/// What starts the window: the file manager at a place, or the Open / Save window for a request.
#[derive(Debug, Clone)]
pub enum Mode {
    Files { open: String, props: bool },
    Pick(Value),
}

#[derive(Debug, Clone, Default)]
pub struct Tab {
    pub path: String,
    pub back: Vec<String>,
    pub fwd: Vec<String>,
    pub sel: String,
}

/// An item in the left menu.
#[derive(Debug, Clone, Default)]
pub struct NavItem {
    pub id: String,
    pub label: String,
    pub path: String,
    pub kind: String,
    pub section: String,
    pub depth: u8,
    pub tree: String,
    pub open: bool,
    pub empty: bool,
    pub sub: String,
    pub pinned: bool,
    pub raw: Value,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Zone {
    Main,
    Places,
    Toolbar,
    Tabs,
    Pick,
    /// The terminal panel's line (F4) and the split view (F9).
    Term,
    Split,
}

/// One item of a menu (Hearth Files' right-click menu, New, Sort, View, a place's menu…).
#[derive(Debug, Clone, Default)]
pub struct MenuItem {
    pub id: String,
    pub label: String,
    pub key: String,
    pub sub: String,
    pub icon: Option<crate::icons::Ic>,
    pub danger: bool,
    /// Windows 11's short menu: shown as an icon in the row at the top.
    pub quick: bool,
    pub check: Option<bool>,
    pub sep: bool,
    pub heading: bool,
    /// A tag's colour dot (Tags ›).
    pub dot: Option<iced::Color>,
}

impl MenuItem {
    pub fn new(id: &str, label: &str) -> MenuItem {
        MenuItem { id: id.into(), label: label.into(), ..Default::default() }
    }
    pub fn sep() -> MenuItem {
        MenuItem { sep: true, ..Default::default() }
    }
    pub fn heading(label: &str) -> MenuItem {
        MenuItem { heading: true, label: label.into(), ..Default::default() }
    }
    pub fn key(mut self, k: &str) -> Self {
        self.key = k.into();
        self
    }
    pub fn sub(mut self, k: &str) -> Self {
        self.sub = k.into();
        self
    }
    pub fn icon(mut self, i: crate::icons::Ic) -> Self {
        self.icon = Some(i);
        self
    }
    pub fn danger(mut self) -> Self {
        self.danger = true;
        self
    }
    pub fn quick(mut self) -> Self {
        self.quick = true;
        self
    }
    pub fn check(mut self, on: bool) -> Self {
        self.check = Some(on);
        self
    }
    pub fn dot(mut self, c: iced::Color) -> Self {
        self.dot = Some(c);
        self
    }
}

#[derive(Debug, Clone)]
pub struct Menu {
    pub title: String,
    pub items: Vec<MenuItem>,
    pub at: Point,
    pub sel: usize,
    /// Windows 11's short right-click menu (the icon row on top, then the rest).
    pub short: bool,
    pub width: f32,
}

/// What the text box of a question is for.
#[derive(Debug, Clone)]
pub enum Ask {
    Rename(Entry),
    BatchRename(Vec<String>),
    NewFolder(String),
    NewFile(String),
    FromTemplate(String, String),
    Compress(Vec<String>, String),
    ExtractTo(String),
    /// Tags, signing in to a shared folder (extras.rs).
    Fx(crate::extras::AskFx),
}

#[derive(Debug, Clone)]
pub struct Input {
    pub title: String,
    pub ok: String,
    pub text: String,
    pub ask: Ask,
    /// A password: what's typed isn't shown.
    pub secret: bool,
}

/// What a yes/no question does when you say yes.
#[derive(Debug, Clone)]
pub enum Yes {
    Delete(Vec<String>),
    Empty,
    PickFinish(Vec<String>),
    Wallpaper(String),
    RunAppImage(String),
    Note,
    /// Anything else: the message to act on.
    Then(Msg),
}

#[derive(Debug, Clone)]
pub struct Confirm {
    pub title: String,
    pub text: String,
    pub ok: String,
    pub danger: bool,
    pub yes: Yes,
    pub button: usize,
    /// A third button between Cancel and OK (Just this time).
    pub alt: Option<(String, Yes)>,
}

#[derive(Debug, Clone)]
pub struct AppsDlg {
    pub title: String,
    pub path: String,
    pub all: bool,
    pub apps: Vec<Value>,
    pub loaded: bool,
    pub sel: usize,
}

#[derive(Debug, Clone)]
pub struct Cf {
    pub verb: String,
    pub paths: Vec<String>,
    pub dest: String,
    pub list: Vec<Value>,
    pub i: usize,
    pub decide: Map<String, Value>,
    pub all: bool,
    pub button: usize,
}

#[derive(Debug, Clone)]
pub struct Pv {
    pub e: Entry,
    pub text: String,
    pub error: String,
}

/// Properties (FileProperties.qml): General, Permissions, Checksums.
#[derive(Debug, Clone)]
pub struct Props {
    pub paths: Vec<String>,
    pub info: Value,
    pub loaded: bool,
    pub tab: usize,
    pub name: String,
    pub mode: u32,
    pub mode0: u32,
    pub sums: Map<String, Value>,
    pub compare: String,
    pub count: Value,
    pub error: String,
    /// Hearth (C:)'s own folders: their rows, no Linux file behind them.
    pub cview: Option<(Entry, String)>,
    /// Sharing (folders): hearth-files sharing's answer (Null until it's asked), and a change on its way.
    pub share: Value,
    pub share_busy: bool,
}

#[derive(Debug, Clone)]
pub enum Overlay {
    Menu(Menu),
    Input(Input),
    Confirm(Confirm),
    Apps(AppsDlg),
    Conflict(Cf),
    Preview(Pv),
    Props(Props),
    /// The Briefcase's dialogs (briefcase.rs keeps what they show).
    Bc,
    /// Sort my Downloads' preview (sortdl.rs).
    Sort,
}

/// A change Hearth Files made that Undo can take back (the last 50, this session).
#[derive(Debug, Clone)]
pub enum Op {
    Rename { from: String, to: String },
    Create { items: Vec<String> },
    Trash { paths: Vec<String> },
    Copy { items: Vec<String> },
    Move { items: Vec<String>, srcs: Vec<String>, dest: String, back: String },
    /// Sort my Downloads: (from, to) of everything it moved.
    Sort { moves: Vec<(String, String)> },
}

#[derive(Debug, Clone)]
pub struct JobReq {
    pub verb: String,
    pub paths: Vec<String>,
    pub dest: String,
    pub decide: Map<String, Value>,
    pub fmt: String,
    pub archive: String,
    pub here: bool,
    pub undo: Option<(Op, bool)>,
}

#[derive(Debug, Clone)]
pub struct Job {
    pub seq: u64,
    pub req: JobReq,
    pub cmd: Vec<String>,
    pub name: String,
    pub dest_name: String,
    pub done: f64,
    pub total: f64,
    pub speed: f64,
    pub file: String,
    pub cancelling: bool,
    pub last: String,
}

#[derive(Debug, Clone)]
pub struct Search {
    pub seq: u64,
    pub cmd: Vec<String>,
}

/// The Open / Save window's request (hearth-portal's, see its docstring).
#[derive(Debug, Clone, Default)]
pub struct Pick {
    pub kind: String,
    pub title: String,
    pub accept: String,
    pub multiple: bool,
    pub directory: bool,
    pub filters: Vec<(String, Vec<String>, Vec<String>)>,
    pub folder: String,
    pub name: String,
    pub files: Vec<String>,
    pub choices: Vec<(String, String, Vec<(String, String)>, String)>,
}

impl Pick {
    pub fn from_json(v: &Value) -> Pick {
        let s = |k: &str| v.get(k).and_then(Value::as_str).unwrap_or("").to_string();
        let strs = |x: Option<&Value>| -> Vec<String> { x.and_then(Value::as_array).map(|a| a.iter().filter_map(|i| i.as_str().map(String::from)).collect()).unwrap_or_default() };
        let filters = v.get("filters").and_then(Value::as_array).map(|a| {
            a.iter().map(|f| (f.get("name").and_then(Value::as_str).unwrap_or("").to_string(), strs(f.get("globs")), strs(f.get("mimes")))).collect()
        }).unwrap_or_default();
        let choices = v.get("choices").and_then(Value::as_array).map(|a| {
            a.iter().map(|c| {
                let opts = c.get("options").and_then(Value::as_array).map(|o| {
                    o.iter().filter_map(|p| p.as_array()).map(|p| (p.first().and_then(Value::as_str).unwrap_or("").to_string(), p.get(1).and_then(Value::as_str).unwrap_or("").to_string())).collect()
                }).unwrap_or_default();
                (c.get("id").and_then(Value::as_str).unwrap_or("").to_string(), c.get("label").and_then(Value::as_str).unwrap_or("").to_string(), opts, c.get("value").and_then(Value::as_str).unwrap_or("").to_string())
            }).collect()
        }).unwrap_or_default();
        let kind = { let k = s("kind"); if k.is_empty() { "open".into() } else { k } };
        let accept = { let a = s("accept"); if a.is_empty() { if kind == "open" { "Open".into() } else { "Save".into() } } else { a } };
        Pick {
            title: { let t = s("title"); if t.is_empty() { if kind == "open" { "Open".into() } else { "Save As".into() } } else { t } },
            kind,
            accept,
            multiple: v.get("multiple").and_then(Value::as_bool).unwrap_or(false),
            directory: v.get("directory").and_then(Value::as_bool).unwrap_or(false),
            filters,
            folder: s("folder"),
            name: s("name"),
            files: strs(v.get("files")),
            choices,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Msg {
    Theme,
    Drag,
    Maximize,
    Minimize,
    Close,
    Resize(iced::window::Direction),
    Key(KEvent),
    Cursor(Point),
    FileDropped(std::path::PathBuf),
    NoteClear(u64),
    // answers
    Prefs(Reply, bool),
    Places(Reply),
    Listed(u64, String, String, bool, Reply),
    Recent(Reply),
    ThisPc(Reply),
    CDrive(Reply),
    Apps(Reply),
    Templates(Reply),
    Did(Then, Reply),
    Watch(Line),
    Mounts(Line),
    Found(Line),
    Job(Line),
    Thumbs(Vec<Thumb>),
    Versions(String, bool, String),
    // moving around
    Go(String),
    Back,
    Forward,
    Up,
    Refresh,
    EditPath,
    PathText(String),
    PathGo,
    SearchText(String),
    SearchGo,
    SearchClear,
    Tab(usize),
    CloseTab(usize),
    NewTab,
    Place(usize),
    PlaceRight(usize),
    PlaceMiddle(usize),
    Fold(String),
    Quick(String),
    QuickMiddle(String),
    Press(usize),
    Double(usize),
    Right(usize),
    Middle(usize),
    BgPress,
    /// Sort my Downloads (sortdl.rs), and its note going after a while.
    Sort(crate::sortdl::SMsg),
    /// Describe it (board 11) and what a feature made with it does when pressed
    Describe(hearth_ui::describe::DMsg),
    DescAct(String),
    SortNoteGone(u64),
    /// The pointer moved over the file view (its own coordinates): drag to select.
    BandMove(Point),
    BgRight,
    Scrolled(f32),
    Cmd(&'static str),
    MenuPick(usize),
    MenuClose,
    // dialogs
    DlgText(String),
    DlgOk,
    DlgCancel,
    DlgButton(usize),
    ConflictPick(&'static str),
    ConflictAll,
    OpenWith(usize),
    OtherApps,
    RenameText(String),
    RenameDone,
    Wallpaper,
    OpenWithMenu,
    PropsTab(usize),
    PropsName(String),
    PropsBit(u32),
    PropsApply(bool),
    PropsCopy(bool),
    Checksum(&'static str),
    CompareText(String),
    // the Open / Save window
    PickName(String),
    PickAccept,
    PickCancel,
    PickFilter,
    PickChoice(usize),
    StopJob,
    DlgAlt,
    // the terminal panel, split view, extras, the Briefcase, the controller in the Open / Save window, drag and drop
    Term(crate::terminal::TMsg),
    Split(crate::split::SMsg),
    Fx(crate::extras::XMsg),
    Bc(crate::briefcase::BMsg),
    Pad(crate::pad::PMsg),
    Dnd(crate::dnd::DMsg),
    WinOpened(iced::window::Id, f32, f32),
    WinSize(f32, f32),
    DragOutReady(Option<(usize, usize)>),
    DragOut(crate::dragout::Out),
}

/// What to do once hearth-files answered.
#[derive(Debug, Clone)]
pub enum Then {
    Nothing,
    Say(String),
    Reload(String),
    Renamed(String),
    Made(String, String),
    Trashed(Vec<String>),
    Deleted(usize),
    Restored(usize),
    Emptied,
    PlacesToo(String),
    Conflicts(String, Vec<String>, String),
    Clip(bool),
    OpenPlan(Entry),
    OpenApps(String, bool),
    Peek(String),
    Props(Vec<String>),
    Count(String),
    Sum(String),
    Chmod,
    Undone(bool, Op),
    Mounted(bool, String),
    Unmounted(bool, String, String),
    PickList(String, String),
    Batch(usize),
    Compare(Vec<String>),
    ThisPcLetter,
    Linked(String),
}

pub struct App {
    pub p: Pal,
    pub home: String,
    pub user: String,
    pub pick: Option<Pick>,
    pub pick_name: String,
    pub pick_filter: i32,
    pub pick_choice: Map<String, Value>,
    pub pick_done: bool,
    pub props_only: bool,
    pub prefs: Map<String, Value>,
    pub prefs_loaded: bool,
    pub start_target: String,
    pub path: String,
    pub listing: Option<Listing>,
    pub load_seq: u64,
    pub loading: bool,
    pub view: View,
    pub sort: String,
    pub desc: bool,
    pub group: String,
    pub hidden: bool,
    pub show_ext: bool,
    pub details_pane: bool,
    /// Describe it (board 11): the box, what's being tried and what was kept (hearth_ui::describe; hearth-layout app.rs)
    pub describe: hearth_ui::describe::Describe,
    pub pc_view: String,
    pub nav_w: f32,
    pub raw_places: Vec<Value>,
    pub places: Vec<NavItem>,
    pub nav_open: HashMap<String, bool>,
    pub pc: Value,
    pub cdata: Value,
    pub app_list: Vec<Value>,
    pub apps_loading: bool,
    pub recent: Vec<Entry>,
    pub templates: Vec<Value>,
    pub query: String,
    pub found: Option<Vec<Entry>>,
    pub found_query: String,
    pub search: Option<Search>,
    pub entries: Vec<Entry>,
    pub marked: HashSet<String>,
    pub anchor: Option<usize>,
    pub cur: usize,
    pub clip_mode: String,
    pub clip: Vec<String>,
    pub back: Vec<String>,
    pub fwd: Vec<String>,
    pub tabs: Vec<Tab>,
    pub tab_index: usize,
    pub editing_path: bool,
    pub path_text: String,
    pub search_text: String,
    pub zone: Zone,
    pub place_index: usize,
    pub overlay: Option<Overlay>,
    pub menu_on_folder: bool,
    pub menu_full: bool,
    pub job: Option<Job>,
    pub job_queue: Vec<JobReq>,
    pub seq: u64,
    pub note: String,
    pub note_err: bool,
    pub note_seq: u64,
    pub undo: Vec<Op>,
    pub redo: Vec<Op>,
    pub renaming: Option<String>,
    pub rename_text: String,
    pub thumbs: HashMap<String, (i64, Option<Handle>, u32, u32, String)>,
    pub thumb_pending: HashSet<String>,
    pub versions: Vec<String>,
    pub versions_of: String,
    pub cursor: Point,
    /// Drag to select (the rubber band): where it started and is now, in the file view's own coordinates, and what
    /// was picked before it (kept with Ctrl or Shift).
    pub band: Option<(Point, Point, HashSet<String>)>,
    /// The last place the pointer was over the file view's empty space (the band starts there).
    pub band_at: Point,
    /// Sort my Downloads: what it would move (the preview), Show every file, and the note after it sorted
    /// (moved, folders, left where they were).
    pub sort_plan: Option<crate::sortdl::Plan>,
    pub sort_every: bool,
    pub sorted: Option<(usize, usize, usize)>,
    pub sorted_seq: u64,
    pub mods: Modifiers,
    pub scroll_y: f32,
    pub area: Cell<(f32, f32)>,
    pub cols: Cell<usize>,
    pub last_click: (usize, std::time::Instant),
    pub show: String,
    /// You clicked or moved in this folder (the row you're on is lit from then on).
    pub touched: bool,
    /// Saved Games' tile on the Home page: how many games have saves there.
    pub saved_games: usize,
    pub term: crate::terminal::Term,
    pub split: Option<crate::split::Split>,
    pub fx: crate::extras::Fx,
    pub bc: crate::briefcase::Bc,
    pub pad: crate::pad::Pad,
    pub drag: Option<crate::dnd::Drag>,
    pub drag_pending: Option<crate::dnd::Pending>,
    /// What a submenu (Tags ›) is about.
    pub menu_targets: Vec<String>,
    /// The window's size (for dragging out: when the pointer leaves it).
    pub win: (f32, f32),
}

pub const SCROLL_ID: &str = "files-main";
pub const PATH_ID: &str = "files-path";
pub const SEARCH_ID: &str = "files-search";
pub const DLG_ID: &str = "files-dlg";
pub const RENAME_ID: &str = "files-rename";
pub const PICKNAME_ID: &str = "files-pickname";

fn sv(v: &Value, k: &str) -> String {
    v.get(k).and_then(Value::as_str).unwrap_or("").to_string()
}

impl App {
    pub fn new(mode: Mode) -> (App, Task<Msg>) {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/".into());
        let user = std::env::var("USER").unwrap_or_else(|_| base(&home));
        let (pick, start, props_only) = match &mode {
            Mode::Files { open, props } => (None, open.clone(), *props),
            Mode::Pick(v) => (Some(Pick::from_json(v)), String::new(), false),
        };
        let mut s = App {
            p: hearth_ui::theme::load(),
            describe: hearth_ui::describe::Describe::new("files", "Hearth Files").with_pages(crate::view::BLOCKS),
            home: home.clone(),
            user,
            pick_name: pick.as_ref().map(|p| p.name.clone()).unwrap_or_default(),
            pick_filter: pick.as_ref().map(|p| if p.filters.is_empty() { -1 } else { 0 }).unwrap_or(-1),
            pick_choice: pick.as_ref().map(|p| p.choices.iter().map(|c| (c.0.clone(), Value::String(c.3.clone()))).collect()).unwrap_or_default(),
            pick,
            pick_done: false,
            props_only,
            prefs: Map::new(),
            prefs_loaded: false,
            start_target: start,
            path: String::new(),
            listing: None,
            load_seq: 0,
            loading: false,
            view: View::Details,
            sort: "name".into(),
            desc: false,
            group: "none".into(),
            hidden: false,
            show_ext: true,
            details_pane: false,
            pc_view: "grid".into(),
            nav_w: 158.0,
            raw_places: vec![],
            places: vec![],
            nav_open: HashMap::new(),
            pc: Value::Null,
            cdata: Value::Null,
            app_list: vec![],
            apps_loading: false,
            recent: vec![],
            templates: vec![],
            query: String::new(),
            found: None,
            found_query: String::new(),
            search: None,
            entries: vec![],
            marked: HashSet::new(),
            anchor: None,
            cur: 0,
            clip_mode: String::new(),
            clip: vec![],
            back: vec![],
            fwd: vec![],
            tabs: vec![Tab::default()],
            tab_index: 0,
            editing_path: false,
            path_text: String::new(),
            search_text: String::new(),
            zone: Zone::Main,
            place_index: 0,
            overlay: None,
            menu_on_folder: false,
            menu_full: false,
            job: None,
            job_queue: vec![],
            seq: 1,
            note: String::new(),
            note_err: false,
            note_seq: 0,
            undo: vec![],
            redo: vec![],
            renaming: None,
            rename_text: String::new(),
            thumbs: HashMap::new(),
            thumb_pending: HashSet::new(),
            versions: vec![],
            versions_of: String::new(),
            cursor: Point::new(400.0, 300.0),
            band: None,
            band_at: Point::ORIGIN,
            sort_plan: None,
            sort_every: false,
            sorted: None,
            sorted_seq: 0,
            mods: Modifiers::default(),
            scroll_y: 0.0,
            area: Cell::new((900.0, 560.0)),
            cols: Cell::new(1),
            last_click: (usize::MAX, std::time::Instant::now()),
            show: happ::arg("--show").unwrap_or_default(),
            touched: false,
            saved_games: 0,
            term: Default::default(),
            split: None,
            fx: Default::default(),
            bc: Default::default(),
            pad: Default::default(),
            drag: None,
            drag_pending: None,
            menu_targets: vec![],
            win: (1180.0, 740.0),
        };
        // CI's pictures: the wider left menu (as dragged past 200 px), shown, never saved.
        if s.show.split(',').any(|x| x == "widenav") {
            s.nav_w = 240.0;
        }
        if let Some(v) = happ::arg("--view").and_then(|v| View::from(&v)) {
            s.view = v;
        }
        let first = Task::batch(vec![
            Task::perform(backend::call(a(&["prefs"]), 10), |r| Msg::Prefs(r, true)),
            s.refresh_places(),
            Task::perform(backend::call(a(&["templates"]), 10), Msg::Templates),
            crate::extras::refresh_tags(),
            crate::briefcase::refresh_known(),
        ]);
        // --describe WORDS (CI's pictures; Describe it opened and asked at once)
        let first = match hearth_ui::app::arg("--describe") {
            Some(w) if !s.picking() => {
                s.describe.open = true;
                s.describe.text = w;
                Task::batch(vec![first, Task::done(Msg::Describe(hearth_ui::describe::DMsg::Make))])
            }
            _ => first,
        };
        (s, first)
    }

    // ------------------------------------------------------------------ the words for where you are
    pub fn picking(&self) -> bool {
        self.pick.is_some()
    }
    pub fn on_home(&self) -> bool {
        self.path == HOME_PAGE
    }
    pub fn on_this_pc(&self) -> bool {
        self.path == THIS_PC
    }
    pub fn on_apps(&self) -> bool {
        self.path == APPS_PAGE
    }
    pub fn cdrive_on(&self) -> bool {
        self.prefs.get("cdrive") != Some(&Value::Bool(false))
    }
    pub fn c_linux(&self) -> bool {
        self.prefs.get("linuxFolders") == Some(&Value::Bool(true))
    }
    pub fn nav_shown(&self) -> bool {
        self.prefs.get("navPane") != Some(&Value::Bool(false))
    }
    pub fn addr_linux(&self) -> bool {
        self.prefs.get("addressBar").and_then(Value::as_str) == Some("linux")
    }
    pub fn copy_linux(&self) -> bool {
        self.prefs.get("copyPath").and_then(Value::as_str) == Some("linux")
    }
    pub fn c_page(&self) -> &'static str {
        if !drive::is_c_path(&self.path) {
            return "";
        }
        drive::c_page(&self.path, &self.ctx())
    }
    pub fn on_cview(&self) -> bool {
        !self.c_page().is_empty()
    }
    pub fn on_special(&self) -> bool {
        self.on_this_pc() || self.on_apps() || self.on_cview()
    }
    /// Where Files opens (View › Open Files to, files.json "startPage", shared with Mantel's FileManager.qml): your
    /// pick; without one, This PC while the C: view is on (like Windows' File Explorer: the owner, Nov 19), Home
    /// otherwise. A folder given on the command line (or by xdg-open) still opens that folder; the Open / Save window
    /// opens where the app asked.
    pub fn start_page(&self) -> String {
        if self.picking() {
            return HOME_PAGE.into();
        }
        match self.prefs.get("startPage").and_then(Value::as_str) {
            Some("thispc") => THIS_PC.into(),
            Some("home") => HOME_PAGE.into(),
            _ if self.cdrive_on() => THIS_PC.into(),
            _ => HOME_PAGE.into(),
        }
    }
    pub fn in_trash(&self) -> bool {
        self.listing.as_ref().map(|l| l.trash).unwrap_or(false)
    }
    pub fn writable(&self) -> bool {
        (self.listing.as_ref().map(|l| l.writable).unwrap_or(false) || crate::extras::admin_here(self)) && self.found.is_none()
    }
    pub fn current(&self) -> Option<&Entry> {
        self.entries.get(self.cur)
    }
    pub fn shown_view(&self) -> View {
        // The Open / Save window is a Details list (mockup cdrive-1).
        if self.picking() {
            return View::Details;
        }
        if self.on_home() || self.on_cview() || self.on_apps() {
            return View::Details;
        }
        // Describe it's "how files are shown" (what's tried, else kept), over the View menu's pick
        match self.describe.knob("view").as_str() {
            "icons" => View::Large,
            "list" => View::Compact,
            "details" => View::Details,
            _ => self.view,
        }
    }

    pub fn cdata_struct(&self) -> CData {
        let strs = |k: &str| -> Vec<String> { self.cdata.get(k).and_then(Value::as_array).map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default() };
        CData {
            apps: self.app_list.clone(),
            hearth: sv(&self.cdata, "hearth"),
            steam: sv(&self.cdata, "steam"),
            linux: strs("linux"),
            flatpak: strs("flatpak"),
            show_linux: self.c_linux(),
        }
    }

    /// drive-paths.js's ctx: your home, the lettered drives, your folders and the view's names.
    pub fn ctx(&self) -> Ctx {
        let mut drives: Vec<drive::Drive> = self
            .raw_places
            .iter()
            .filter(|p| !sv(p, "letter").is_empty() && !sv(p, "path").is_empty() && p.get("mounted") != Some(&Value::Bool(false)))
            .map(|p| drive::Drive { letter: sv(p, "letter"), path: sv(p, "path"), label: sv(p, "label") })
            .collect();
        if let Some(net) = self.pc.get("network").and_then(Value::as_array) {
            for n in net {
                if !sv(n, "letter").is_empty() && !sv(n, "path").is_empty() {
                    drives.push(drive::Drive { letter: sv(n, "letter"), path: sv(n, "path"), label: sv(n, "label") });
                }
            }
        }
        let kinds = ["desktop", "documents", "downloads", "pictures", "music", "videos"];
        let folders = self
            .raw_places
            .iter()
            .filter(|p| sv(p, "section") == "places" && p.get("child") != Some(&Value::Bool(true)) && kinds.contains(&sv(p, "kind").as_str()))
            .map(|p| drive::Folder { label: sv(p, "label"), path: sv(p, "path") })
            .collect();
        let mut user = sv(&self.pc, "user");
        if user.is_empty() {
            user = sv(&self.cdata, "user");
        }
        if user.is_empty() {
            user = base(&self.home);
        }
        let mut c = Ctx { home: self.home.clone(), user, drives, folders, c_on: self.cdrive_on(), users: sv(&self.cdata, "users"), apps: vec![] };
        if c.c_on {
            c.apps = drive::c_app_list(&self.cdata_struct());
        }
        c
    }
    /// The address bar's ctx: the view's names unless you chose Linux paths.
    pub fn crumb_ctx(&self) -> Ctx {
        let mut c = self.ctx();
        if self.addr_linux() {
            c.c_on = false;
        }
        c
    }
    pub fn win_path(&self, p: &str) -> String {
        drive::to_windows(p, &self.ctx())
    }
    pub fn copy_form(&self, p: &str, linux: bool) -> String {
        if drive::is_c_path(p) {
            return self.win_path(p);
        }
        if linux { p.to_string() } else { let w = self.win_path(p); if w.is_empty() { p.to_string() } else { w } }
    }
    pub fn address_text(&self, p: &str) -> String {
        if self.on_this_pc() {
            return "This PC".into();
        }
        if self.on_apps() {
            return "This PC\\Installed apps".into();
        }
        if self.on_home() {
            return "Home".into();
        }
        let ctx = self.ctx();
        if drive::is_c_path(p) {
            return drive::to_windows(p, &ctx);
        }
        if self.cdrive_on() && !self.addr_linux() && p.starts_with('/') && !self.in_trash() {
            return drive::to_windows(p, &ctx);
        }
        if drive::drive_for(p, &ctx).is_some() && !(self.cdrive_on() && self.addr_linux()) { drive::to_windows(p, &ctx) } else { p.to_string() }
    }

    pub fn title_of(&self, p: &str) -> String {
        if p.is_empty() || p == HOME_PAGE {
            return "Home".into();
        }
        if p == THIS_PC {
            return "This PC".into();
        }
        if p == APPS_PAGE {
            return "Installed apps".into();
        }
        if drive::is_c_path(p) {
            return if drive::c_page(p, &self.ctx()) == "root" { self.system_label() } else { base(p) };
        }
        if let Some(d) = self.raw_places.iter().find(|q| !sv(q, "letter").is_empty() && sv(q, "path") == p) {
            return sv(d, "label");
        }
        if p == "/" { "This device".into() } else { base(p) }
    }
    pub fn system_label(&self) -> String {
        self.raw_places.iter().find(|q| sv(q, "key") == "system").map(|q| sv(q, "label")).unwrap_or_else(|| "Hearth (C:)".into())
    }

    pub fn trash_path(&self) -> String {
        self.raw_places.iter().find(|q| sv(q, "kind") == "trash").map(|q| sv(q, "path")).unwrap_or_else(|| format!("{}/.local/share/Trash/files", self.home))
    }

    /// Where a path starts from, for the path bar and for Up: the Trash, a drive, Home, or the whole computer.
    pub fn root_of(&self, p: &str) -> (String, String) {
        let mut roots = vec![(self.trash_path(), "Trash".to_string())];
        for q in &self.raw_places {
            if sv(q, "section") == "drives" && !sv(q, "path").is_empty() && sv(q, "path") != "/" {
                roots.push((sv(q, "path"), sv(q, "label")));
            }
        }
        roots.push((self.home.clone(), "Home".into()));
        let mut best = ("/".to_string(), "Computer".to_string());
        for r in roots {
            if model::under(p, &r.0) && r.0.len() > if best.0 == "/" { 0 } else { best.0.len() } {
                best = r;
            }
        }
        best
    }

    pub fn crumbs(&self) -> Vec<drive::Crumb> {
        use drive::Crumb;
        let p = &self.path;
        if p.is_empty() {
            return vec![];
        }
        if p == HOME_PAGE {
            return vec![Crumb { label: "Home".into(), path: HOME_PAGE.into() }];
        }
        if p == THIS_PC {
            return vec![Crumb { label: "This PC".into(), path: THIS_PC.into() }];
        }
        if p == APPS_PAGE {
            return vec![Crumb { label: "This PC".into(), path: THIS_PC.into() }, Crumb { label: "Installed apps".into(), path: APPS_PAGE.into() }];
        }
        let (rp, rl) = self.root_of(p);
        let mapped = [".config", ".local/share", ".cache"].iter().any(|d| model::under(p, &format!("{}/{d}", self.home)));
        let home_way = rp == self.home && !self.picking_linux_paths() && !mapped;
        if !self.in_trash() && !home_way {
            if let Some(c) = drive::crumbs_for(p, &self.crumb_ctx(), THIS_PC) {
                // Your home folder itself, the Windows way: This PC › Hearth (C:) › Users › you.
                return c;
            }
        }
        // The home folder the Windows way (mockups files-1, -6, cdrive-1): This PC › Hearth (C:) › Users › chris › …
        if home_way {
            let mut out = vec![Crumb { label: "This PC".into(), path: THIS_PC.into() }, Crumb { label: self.system_label(), path: if self.cdrive_on() { drive::C_ROOT.into() } else { "/".into() } }];
            out.push(Crumb { label: "Users".into(), path: drive::users_dir(&self.ctx()) });
            out.push(Crumb { label: base(&self.home), path: self.home.clone() });
            let rest = p.get(self.home.len()..).unwrap_or("");
            let mut at = self.home.clone();
            for part in rest.split('/').filter(|s| !s.is_empty()) {
                at = format!("{at}/{part}");
                out.push(Crumb { label: part.into(), path: at.clone() });
            }
            return out;
        }
        let mut out = vec![Crumb { label: rl, path: rp.clone() }];
        let rest = if rp == "/" { p.get(1..).unwrap_or("") } else { p.get(rp.len() + 1..).unwrap_or("") };
        let mut at = if rp == "/" { String::new() } else { rp.clone() };
        for part in rest.split('/').filter(|s| !s.is_empty()) {
            at = format!("{at}/{part}");
            out.push(Crumb { label: part.into(), path: at.clone() });
        }
        out
    }
    fn picking_linux_paths(&self) -> bool {
        self.addr_linux() || self.in_trash()
    }

    // ------------------------------------------------------------------ the entries shown
    pub fn rebuild(&mut self) {
        let keep = self.entries.get(self.cur).map(|e| e.path.clone());
        self.entries = self.compute_entries();
        if let Some(k) = keep {
            if let Some(i) = self.entries.iter().position(|e| e.path == k) {
                self.cur = i;
            }
        }
        if self.cur >= self.entries.len() {
            self.cur = self.entries.len().saturating_sub(1);
        }
    }

    fn compute_entries(&self) -> Vec<Entry> {
        let q = self.query.trim().to_lowercase();
        let filt = |v: Vec<Entry>| -> Vec<Entry> { if q.is_empty() { v } else { v.into_iter().filter(|e| e.name.to_lowercase().contains(&q)).collect() } };
        let ctx = self.ctx();
        let pg = drive::c_page(&self.path, &ctx);
        if !pg.is_empty() {
            let list = drive::c_entries(pg, &ctx, &self.cdata_struct());
            return filt(list.into_iter().map(|c| Entry { name: c.name, path: c.path, dir: true, size: c.cv.size, cv: Some(c.cv), ..Default::default() }).collect());
        }
        if self.path == THIS_PC {
            let mut out = vec![];
            for d in self.pc.get("drives").and_then(Value::as_array).cloned().unwrap_or_default() {
                let path = { let p = sv(&d, "path"); if p.is_empty() { sv(&d, "id") } else { p } };
                out.push(Entry { name: sv(&d, "label"), path, dir: true, pc: "drive".into(), item: d, ..Default::default() });
            }
            for n in self.pc.get("network").and_then(Value::as_array).cloned().unwrap_or_default() {
                let path = { let p = sv(&n, "path"); if p.is_empty() { sv(&n, "id") } else { p } };
                out.push(Entry { name: sv(&n, "label"), path, dir: true, pc: "net".into(), item: n, ..Default::default() });
            }
            return filt(out);
        }
        if self.path == APPS_PAGE {
            let mut out: Vec<Entry> = self
                .app_list
                .iter()
                .map(|a| Entry { name: sv(a, "name"), path: format!("app:{}", sv(a, "id")), size: a.get("size").and_then(Value::as_u64).unwrap_or(0), mtime: a.get("date").and_then(Value::as_i64).unwrap_or(0), app: Some(a.clone()), ..Default::default() })
                .collect();
            out.sort_by(|x, y| model::natural(&x.name, &y.name));
            return filt(out);
        }
        if self.path == HOME_PAGE {
            return filt(self.recent.clone());
        }
        let mut all: Vec<Entry> = match &self.found {
            Some(f) => f.clone(),
            None => {
                let mut v = self.listing.as_ref().map(|l| l.entries.clone()).unwrap_or_default();
                if let Some(l) = &self.listing {
                    if self.cdrive_on() {
                        let names: Vec<String> = l.entries.iter().map(|e| e.name.clone()).collect();
                        for c in drive::c_extras(&l.path, &ctx, &names) {
                            v.push(Entry { name: c.name, path: c.path, dir: true, mime: "inode/directory".into(), cv: Some(c.cv), ..Default::default() });
                        }
                    }
                }
                v
            }
        };
        if self.found.is_none() && !q.is_empty() {
            all.retain(|e| e.name.to_lowercase().contains(&q));
        }
        if self.picking() {
            all.retain(|e| self.pick_shows(e));
        }
        let by = self.sort.as_str();
        let desc = self.desc;
        let g = self.group.as_str();
        let mut keyed: Vec<(i64, Vec<(bool, String)>, String, Entry)> =
            all.into_iter().map(|e| (if g == "none" { 0 } else { self.group_rank(&e, g) }, model::natural_key(&e.name), if by == "type" { model::ext_of(&e) } else { String::new() }, e)).collect();
        keyed.sort_by(|x, y| {
            use std::cmp::Ordering;
            if x.0 != y.0 {
                return x.0.cmp(&y.0);
            }
            if x.3.dir != y.3.dir {
                return if x.3.dir { Ordering::Less } else { Ordering::Greater };
            }
            let mut r = match by {
                "date" => x.3.mtime.cmp(&y.3.mtime),
                "size" => x.3.size.cmp(&y.3.size),
                "type" => x.2.cmp(&y.2),
                _ => Ordering::Equal,
            };
            if r == Ordering::Equal {
                r = model::natural_cmp(&x.1, &y.1).then_with(|| x.3.name.cmp(&y.3.name));
            }
            if desc { r.reverse() } else { r }
        });
        keyed.into_iter().map(|k| k.3).collect()
    }

    pub fn group_rank(&self, e: &Entry, g: &str) -> i64 {
        let kinds = ["folder", "doc", "text", "image", "video", "audio", "archive", "app", "disc", "file"];
        match g {
            "type" => kinds.iter().position(|k| *k == model::kind_of(e)).unwrap_or(9) as i64,
            "size" => size_rank(e),
            "tags" => {
                if e.tags.is_empty() {
                    100000
                } else {
                    1
                }
            }
            _ => {
                let now = hearth_ui::time::now();
                let n = hearth_ui::time::local(now);
                let day = now - (n.hour as i64 * 3600 + n.min as i64 * 60 + n.sec as i64);
                let t = e.mtime;
                if t >= day {
                    0
                } else if t >= day - 86400 {
                    1
                } else if t >= day - 6 * 86400 {
                    2
                } else if t >= day - 30 * 86400 {
                    3
                } else if hearth_ui::time::local(t).year == n.year {
                    4
                } else {
                    5 + (n.year - hearth_ui::time::local(t).year) as i64
                }
            }
        }
    }

    /// Where each file sits in the file view (its own coordinates), worked out the way view.rs lays them out:
    /// Details' rows (group headings take a row), Compact's columns, and the icon views' grid.
    pub fn item_rects(&self) -> Vec<(usize, iced::Rectangle)> {
        let (w, h) = self.area.get();
        let mut out = Vec::with_capacity(self.entries.len());
        match self.view {
            View::Details => {
                let rh = ROW_H + 2.0;
                let (mut k, mut last) = (0usize, String::new());
                for (i, e) in self.entries.iter().enumerate() {
                    if self.group != "none" {
                        let g = self.group_name(e);
                        if g != last {
                            k += 1;
                            last = g;
                        }
                    }
                    out.push((i, iced::Rectangle { x: 12.0, y: 4.0 + k as f32 * rh, width: (w - 24.0).max(1.0), height: ROW_H }));
                    k += 1;
                }
            }
            View::Compact => {
                let per = ((h - 12.0) / ROW_H).floor().max(1.0) as usize;
                for i in 0..self.entries.len() {
                    let (c, j) = (i / per, i % per);
                    out.push((i, iced::Rectangle { x: 6.0 + c as f32 * 236.0, y: 6.0 + j as f32 * ROW_H, width: 230.0, height: ROW_H }));
                }
            }
            v => {
                let (cw, ch) = v.cell();
                let cols = self.cols.get().max(1);
                for i in 0..self.entries.len() {
                    let (r, c) = (i / cols, i % cols);
                    out.push((i, iced::Rectangle { x: 12.0 + c as f32 * (cw + 8.0), y: 10.0 + r as f32 * (ch + 8.0), width: cw, height: ch }));
                }
            }
        }
        out
    }

    /// The band's rectangle, once it's been dragged a few pixels (a plain click on empty space stays a click).
    pub fn band_rect(&self) -> Option<iced::Rectangle> {
        let (a, b, _) = self.band.as_ref()?;
        let r = iced::Rectangle { x: a.x.min(b.x), y: a.y.min(b.y), width: (a.x - b.x).abs(), height: (a.y - b.y).abs() };
        (r.width > 4.0 || r.height > 4.0).then_some(r)
    }

    /// What the band covers is picked, with what was picked before it when Ctrl or Shift was held.
    pub fn band_select(&mut self) {
        let Some(r) = self.band_rect() else { return };
        let mut m = self.band.as_ref().map(|b| b.2.clone()).unwrap_or_default();
        let mut first = None;
        for (i, ir) in self.item_rects() {
            if ir.intersects(&r) {
                if let Some(e) = self.entries.get(i) {
                    m.insert(e.path.clone());
                    first.get_or_insert(i);
                }
            }
        }
        self.marked = m;
        self.touched = true;
        if let Some(i) = first {
            self.cur = i;
            self.anchor = Some(i);
        }
    }

    pub fn group_name(&self, e: &Entry) -> String {
        match self.group.as_str() {
            "none" => String::new(),
            "type" => match model::kind_of(e) {
                "folder" => "Folders",
                "doc" => "Documents",
                "text" => "Text",
                "image" => "Pictures",
                "video" => "Videos",
                "audio" => "Music and audio",
                "archive" => "Archives",
                "app" => "Apps and programs",
                "disc" => "Disc images",
                _ => "Other files",
            }
            .into(),
            "size" => ["Folders", "Tiny (under 16 KB)", "Small (under 1 MB)", "Medium (under 128 MB)", "Large (under 1 GB)", "Huge (1 GB or more)"][size_rank(e) as usize].into(),
            "tags" => e.tags.first().cloned().unwrap_or_else(|| "No tag".into()),
            _ => {
                let r = self.group_rank(e, "date");
                match r {
                    0 => "Today".into(),
                    1 => "Yesterday".into(),
                    2 => "Earlier this week".into(),
                    3 => "Earlier this month".into(),
                    4 => "Earlier this year".into(),
                    _ => hearth_ui::time::local(e.mtime).year.to_string(),
                }
            }
        }
    }

    // ------------------------------------------------------------------ the Open / Save window
    fn pick_shows(&self, e: &Entry) -> bool {
        let Some(pk) = &self.pick else { return true };
        if e.dir {
            return true;
        }
        if pk.directory {
            return false;
        }
        let Some(f) = (if self.pick_filter >= 0 { pk.filters.get(self.pick_filter as usize) } else { None }) else { return true };
        let any = f.1.iter().any(|g| g == "*") || f.2.iter().any(|m| m == "*/*" || m == "application/octet-stream");
        if any || (f.1.is_empty() && f.2.is_empty()) {
            return true;
        }
        if f.1.iter().any(|g| model::glob_match(g, &e.name)) {
            return true;
        }
        f.2.iter().any(|x| *x == e.mime || (x.ends_with("/*") && e.mime.starts_with(&x[..x.len() - 1])))
    }

    pub fn pick_targets(&self) -> Vec<String> {
        let Some(pk) = &self.pick else { return vec![] };
        let sel: Vec<String> = self
            .entries
            .iter()
            .filter(|e| self.marked.contains(&e.path) && (if pk.directory { e.dir } else { !e.dir }) && !drive::is_c_path(&e.path))
            .map(|e| e.path.clone())
            .collect();
        if pk.directory {
            if !sel.is_empty() {
                return if pk.multiple { sel } else { sel[..1].to_vec() };
            }
            return match &self.listing {
                Some(l) if l.error.is_empty() && self.found.is_none() && !drive::is_c_path(&l.path) && l.path.starts_with('/') => vec![l.path.clone()],
                _ => vec![],
            };
        }
        if !sel.is_empty() {
            return if pk.multiple { sel } else { sel[..1].to_vec() };
        }
        match self.current() {
            Some(c) if !c.dir && !drive::is_c_path(&c.path) && c.path.starts_with('/') => vec![c.path.clone()],
            _ => vec![],
        }
    }

    pub fn pick_accept_label(&self) -> String {
        let Some(pk) = &self.pick else { return String::new() };
        if pk.kind != "open" {
            return pk.accept.clone();
        }
        let t = self.pick_targets();
        if pk.directory {
            return format!("{}{}", pk.accept, if t.len() == 1 { format!(" “{}”", base(&t[0])) } else if t.len() > 1 { format!(" ({})", t.len()) } else { String::new() });
        }
        format!("{}{}", pk.accept, if t.len() > 1 { format!(" ({})", t.len()) } else { String::new() })
    }

    fn pick_finish(&mut self, paths: Vec<String>) -> Task<Msg> {
        if self.pick_done {
            return Task::none();
        }
        self.pick_done = true;
        let res = json!({ "paths": paths, "filter": self.pick_filter, "choices": Value::Object(self.pick_choice.clone()) });
        write_result(&res);
        iced::exit()
    }

    fn pick_cancel(&mut self) -> Task<Msg> {
        if self.picking() && !self.pick_done {
            self.pick_done = true;
            write_result(&json!({ "cancel": true }));
        }
        iced::exit()
    }

    fn pick_accept(&mut self) -> Task<Msg> {
        let Some(pk) = self.pick.clone() else { return Task::none() };
        if self.pick_done {
            return Task::none();
        }
        if pk.kind == "save" {
            return self.pick_save();
        }
        if pk.kind == "savefiles" {
            let Some(l) = self.listing.clone() else { return self.say("Open the folder to save in first", true) };
            if !l.error.is_empty() || self.found.is_some() || drive::is_c_path(&self.path) || !l.path.starts_with('/') {
                return self.say("Open the folder to save in first", true);
            }
            let there: Vec<String> = l.entries.iter().filter(|e| pk.files.contains(&e.name)).map(|e| e.name.clone()).collect();
            if there.is_empty() {
                return self.pick_finish(vec![l.path.clone()]);
            }
            let title = if there.len() == 1 { format!("Replace “{}”?", there[0]) } else { format!("Replace {} files?", there.len()) };
            let text = format!("{} already in “{}”. Replacing overwrites what's there.", if there.len() == 1 { "A file with this name is" } else { "Files with these names are" }, l.name);
            self.confirm(&title, &text, "Replace", Yes::PickFinish(vec![l.path.clone()]));
            return Task::none();
        }
        let t = self.pick_targets();
        if t.is_empty() {
            return self.say(if pk.directory { "Pick a folder first" } else { "Pick a file first" }, true);
        }
        self.pick_finish(t)
    }

    fn pick_save(&mut self) -> Task<Msg> {
        let mut n = self.pick_name.trim().to_string();
        if n.is_empty() {
            return Task::batch(vec![operation::focus(PICKNAME_ID), self.say("Type a name first", true)]);
        }
        if let Some(r) = n.strip_prefix('~') {
            n = format!("{}{}", self.home, r);
        }
        if n.len() >= 2 && n.as_bytes()[1] == b':' && n.as_bytes()[0].is_ascii_alphabetic() {
            match drive::from_windows(&n, &self.ctx()) {
                Some(drive::Res::Path(p)) if !drive::is_c_path(&p) => n = p,
                Some(drive::Res::Error(e)) => return self.say(&e, true),
                _ => return self.say("That's not a folder you can save in", true),
            }
        }
        if !n.starts_with('/') && (drive::is_c_path(&self.path) || !self.path.starts_with('/')) {
            return self.say("Open a folder to save in first (your own folders are in Users)", true);
        }
        let full = if n.starts_with('/') { n } else { format!("{}/{}", if self.path == "/" { "" } else { &self.path }, n) };
        let dir = parent_of(&full);
        let name = base(&full);
        if let Some(l) = &self.listing {
            if l.path == dir && (self.hidden || !name.starts_with('.')) {
                let hit = l.entries.iter().find(|e| e.name == name).cloned();
                return self.pick_decide(full, dir, name, hit);
            }
        }
        Task::perform(backend::call(a(&["list", &dir, "--hidden"]), 20), move |r| Msg::Did(Then::PickList(full.clone(), dir.clone()), r))
    }

    fn pick_decide(&mut self, full: String, dir: String, name: String, hit: Option<Entry>) -> Task<Msg> {
        match hit {
            Some(h) if h.dir => {
                self.pick_name.clear();
                self.navigate(&h.path, "")
            }
            Some(_) => {
                self.confirm(&format!("Replace “{name}”?"), &format!("A file with this name is already in “{}”. Replacing it overwrites what's there.", base(&dir)), "Replace", Yes::PickFinish(vec![full]));
                Task::none()
            }
            None => self.pick_finish(vec![full]),
        }
    }

    // ------------------------------------------------------------------ messages at the bottom
    pub fn say(&mut self, t: &str, err: bool) -> Task<Msg> {
        self.note = t.to_string();
        self.note_err = err;
        self.note_seq += 1;
        let s = self.note_seq;
        Task::perform(hearth_ui::run::after(4500), move |_| Msg::NoteClear(s))
    }

    pub fn confirm(&mut self, title: &str, text: &str, ok: &str, yes: Yes) {
        let danger = matches!(yes, Yes::Delete(_) | Yes::Empty | Yes::PickFinish(_));
        self.overlay = Some(Overlay::Confirm(Confirm { title: title.into(), text: text.into(), ok: ok.into(), danger, yes, button: 0, alt: None }));
    }

    pub fn ask(&mut self, title: &str, initial: &str, ok: &str, select_to: usize, ask: Ask) -> Task<Msg> {
        self.overlay = Some(Overlay::Input(Input { title: title.into(), ok: ok.into(), text: initial.into(), ask, secret: false }));
        Task::batch(vec![operation::focus(DLG_ID), operation::select_range(DLG_ID, 0, select_to.min(initial.chars().count()))])
    }

    // ------------------------------------------------------------------ settings (files.json, through hearth-files)
    fn apply_prefs(&mut self, r: &Value) {
        self.prefs = r.as_object().cloned().unwrap_or_default();
        self.prefs_loaded = true;
        self.hidden = self.prefs.get("hidden").and_then(Value::as_bool).unwrap_or(false);
        self.show_ext = self.prefs.get("showExt").and_then(Value::as_bool) != Some(false);
        self.details_pane = self.prefs.get("detailsPane").and_then(Value::as_bool).unwrap_or(false);
        self.pc_view = if self.prefs.get("thisPcView").and_then(Value::as_str) == Some("details") { "details".into() } else { "grid".into() };
        if let Some(w) = self.prefs.get("navWidth").and_then(Value::as_f64) {
            if (120.0..=360.0).contains(&w) {
                self.nav_w = w as f32;
            }
        }
        self.apply_folder_view("");
    }

    fn apply_folder_view(&mut self, p: &str) {
        let f = self.prefs.get("folders").and_then(|f| f.get(p)).and_then(Value::as_object).cloned().unwrap_or_else(|| self.prefs.clone());
        self.view = f.get("view").and_then(Value::as_str).and_then(View::from).unwrap_or(View::Grid);
        // CI's pictures ask for a view (--view): shown, never saved.
        if let Some(v) = happ::arg("--view").and_then(|v| View::from(&v)) {
            self.view = v;
        }
        let s = f.get("sort").and_then(Value::as_str).unwrap_or("name");
        self.sort = if ["name", "date", "size", "type"].contains(&s) { s.into() } else { "name".into() };
        self.desc = f.get("desc").and_then(Value::as_bool).unwrap_or(false);
        let g = f.get("group").and_then(Value::as_str).unwrap_or("none");
        self.group = if ["none", "type", "date", "size", "tags"].contains(&g) { g.into() } else { "none".into() };
    }

    /// Only what you changed goes to the file (hearth-files prefs-set merges it, one folder's view at a time).
    pub fn remember_view(&mut self) -> Task<Msg> {
        let v = json!({ "view": self.view.id(), "sort": self.sort, "desc": self.desc, "group": self.group });
        let mut ch = Map::new();
        if !self.path.is_empty() && !self.on_home() && !self.on_special() && self.path.starts_with('/') {
            let mut fo = Map::new();
            fo.insert(self.path.clone(), v.clone());
            ch.insert("folders".into(), Value::Object(fo.clone()));
            let mut all = self.prefs.get("folders").and_then(Value::as_object).cloned().unwrap_or_default();
            all.remove(&self.path);
            all.insert(self.path.clone(), v);
            self.prefs.insert("folders".into(), Value::Object(all));
        }
        for (k, val) in [
            ("view", json!(self.view.id())),
            ("sort", json!(self.sort)),
            ("desc", json!(self.desc)),
            ("group", json!(self.group)),
            ("hidden", json!(self.hidden)),
            ("showExt", json!(self.show_ext)),
            ("detailsPane", json!(self.details_pane)),
        ] {
            ch.insert(k.into(), val.clone());
            self.prefs.insert(k.into(), val);
        }
        self.prefs_set(Value::Object(ch))
    }

    fn prefs_set(&mut self, ch: Value) -> Task<Msg> {
        Task::perform(backend::call(vec!["prefs-set".into(), ch.to_string()], 15), |r| Msg::Did(Then::Nothing, r))
    }

    /// The view's switches (the same files.json keys as Settings › System › Drives & folders), one key at a time.
    pub fn set_files_pref(&mut self, k: &str, v: Value) -> Task<Msg> {
        let now = match k {
            "cdrive" => json!(self.cdrive_on()),
            "linuxFolders" => json!(self.c_linux()),
            "addressBar" => json!(if self.addr_linux() { "linux" } else { "windows" }),
            "copyPath" => json!(if self.copy_linux() { "linux" } else { "windows" }),
            "navPane" => json!(self.nav_shown()),
            "startPage" => json!(self.prefs.get("startPage").and_then(Value::as_str).unwrap_or(if self.cdrive_on() { "thispc" } else { "home" })),
            _ => Value::Null,
        };
        if now == v {
            return Task::none();
        }
        self.prefs.insert(k.into(), v.clone());
        let mut o = Map::new();
        o.insert(k.into(), v.clone());
        let t = self.prefs_set(Value::Object(o));
        self.places = self.build_nav();
        let on = v.as_bool().unwrap_or(false);
        let vs = v.as_str().unwrap_or("");
        let msg = match k {
            "cdrive" => if on { "Hearth (C:) shows Windows folder names" } else { "Hearth (C:) shows its real folders" },
            "linuxFolders" => if on { "Showing the Linux folders on Hearth (C:) too" } else { "The Linux folders are hidden on Hearth (C:)" },
            "addressBar" => if vs == "linux" { "The address bar shows Linux paths" } else { "The address bar shows Windows names" },
            "copyPath" => if vs == "linux" { "Copy path gives the Linux path" } else { "Copy path gives the C:\\ form" },
            "navPane" => if on { "Navigation pane shown" } else { "Navigation pane hidden" },
            "startPage" => if vs == "thispc" { "Files opens to This PC from now on" } else { "Files opens to Home from now on" },
            _ => "Saved",
        };
        let mut tasks = vec![t, self.say(msg, false)];
        if k == "cdrive" && !on && self.on_cview() {
            tasks.push(self.navigate(THIS_PC, ""));
        }
        if k == "cdrive" && on && self.cdata.is_null() {
            tasks.push(Task::perform(backend::call(a(&["cdrive"]), 20), Msg::CDrive));
        }
        self.rebuild();
        Task::batch(tasks)
    }

    // ------------------------------------------------------------------ the left menu
    pub fn refresh_places(&self) -> Task<Msg> {
        Task::perform(backend::call(a(&["places"]), 20), Msg::Places)
    }

    fn sorted_places(&self) -> Vec<Value> {
        let rank = |s: &str| -> f32 {
            match s {
                "bookmarks" => 1.0,
                "places" => 2.0,
                "briefcases" => 2.5,
                "games" => 3.0,
                "drives" => 4.0,
                "network" => 5.0,
                "devices" => 6.0,
                "trash" => 7.0,
                _ => 5.0,
            }
        };
        // Your folders the Windows way (mockups files-1, -4): Desktop, Downloads, Documents, Pictures, Music, Videos.
        let order = |k: &str| -> i32 {
            match k {
                "desktop" => 0,
                "downloads" => 1,
                "documents" => 2,
                "pictures" => 3,
                "music" => 4,
                "videos" => 5,
                "games" => 6,
                _ => 7,
            }
        };
        let mut v: Vec<(usize, Value)> = self.raw_places.iter().cloned().enumerate().collect();
        v.sort_by(|x, y| {
            let (a, b) = (&x.1, &y.1);
            rank(&sv(a, "section"))
                .partial_cmp(&rank(&sv(b, "section")))
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| if sv(a, "section") == "places" && sv(b, "section") == "places" && a.get("child").is_none() && b.get("child").is_none() { order(&sv(a, "kind")).cmp(&order(&sv(b, "kind"))) } else { std::cmp::Ordering::Equal })
                .then_with(|| x.0.cmp(&y.0))
        });
        v.into_iter().map(|x| x.1).collect()
    }

    /// The left menu the Windows 11 way (the owner's approved option A, files-4-A-narrow): Home (no Gallery or Home server:
    /// the owner, Nov 19),
    /// Trash; your folders and pinned ones; This PC (Hearth (C:) open with Users, Program Files and Windows apps, then
    /// the other drives); Network; the Linux folders folded under "Linux" (only with Show the Linux folders).
    pub fn build_nav(&self) -> Vec<NavItem> {
        let mut out: Vec<NavItem> = Vec::new();
        let sorted = self.sorted_places();
        let item = |q: &Value, section: &str, depth: u8| -> NavItem {
            NavItem {
                id: sv(q, "id"),
                label: sv(q, "label"),
                path: sv(q, "path"),
                kind: sv(q, "kind"),
                section: section.into(),
                depth,
                sub: sv(q, "sub"),
                raw: q.clone(),
                ..Default::default()
            }
        };
        out.push(NavItem { id: "start".into(), label: "Home".into(), path: HOME_PAGE.into(), kind: "start".into(), section: "start".into(), ..Default::default() });
        // (Gallery and a separate Home server entry were taken off by the owner on Nov 19: "there must be only one" (Home
        // server and Network): the home server's shared folders are first inside Network; Pictures stays a folder, and
        // the Photos app is the gallery.)
        let devs: Vec<&Value> = sorted.iter().filter(|q| sv(q, "section") == "network" || sv(q, "section") == "devices").collect();
        if !self.picking() {
            let mut t = sorted.iter().find(|q| sv(q, "kind") == "trash").map(|q| item(q, "start", 0)).unwrap_or_default();
            t.id = "trash".into();
            t.label = "Trash".into();
            t.kind = "trash".into();
            if t.path.is_empty() {
                t.path = format!("{}/.local/share/Trash/files", self.home);
            }
            out.push(t);
        }
        // Your folders, then what you pinned: each real folder once (the owner, Nov 19: the favourite folders showed
        // twice; a GTK bookmark of Documents is the same folder as Documents). Nothing is removed from any file.
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        for q in &sorted {
            if sv(q, "section") == "places" && sv(q, "id") != "home" && q.get("child") != Some(&Value::Bool(true)) {
                if !seen.insert(place_key(&sv(q, "path"))) {
                    continue;
                }
                let mut it = item(q, "folders", 0);
                it.pinned = true;
                out.push(it);
            }
        }
        for q in &sorted {
            let s = sv(q, "section");
            if s == "bookmarks" || s == "briefcases" || s == "games" {
                let k = place_key(&sv(q, "path"));
                if !k.is_empty() && !seen.insert(k) {
                    continue;
                }
                let mut it = item(q, "folders", 0);
                it.pinned = true;
                out.push(it);
            }
        }
        // Tags (FilesExtras.qml): after your folders, each with how many carry it.
        if !self.picking() {
            out.extend(crate::extras::tag_places(self));
        }
        let pc_open = *self.nav_open.get("thispc").unwrap_or(&true);
        out.push(NavItem { id: "thispc".into(), label: "This PC".into(), path: THIS_PC.into(), kind: "thispc".into(), section: "pc".into(), tree: "thispc".into(), open: pc_open, ..Default::default() });
        if pc_open {
            let ctx = self.ctx();
            for d in sorted.iter().filter(|q| sv(q, "section") == "drives") {
                let sys = sv(d, "key") == "system" && self.cdrive_on();
                let c_open = *self.nav_open.get("cdrive").unwrap_or(&true);
                let mut it = item(d, "pc", 1);
                it.sub.clear();
                if sv(d, "kind") == "games" {
                    it.kind = "drive".into();
                }
                if sys {
                    it.path = drive::C_ROOT.into();
                    it.tree = if self.picking() { String::new() } else { "cdrive".into() };
                    it.open = c_open;
                } else if self.picking() {
                    it.tree.clear();
                } else {
                    it.tree = format!("drive:{}", it.id);
                    it.open = false;
                }
                out.push(it);
                if sys && c_open && !self.picking() {
                    out.push(NavItem { id: "c-users".into(), label: "Users".into(), path: drive::users_dir(&ctx), kind: "folder".into(), section: "pc".into(), depth: 2, tree: "c-users".into(), ..Default::default() });
                    out.push(NavItem { id: "c-pf".into(), label: "Program Files".into(), path: format!("{}/Program Files", drive::C_ROOT), kind: "folder".into(), section: "pc".into(), depth: 2, tree: "c-pf".into(), ..Default::default() });
                    out.push(NavItem { id: "c-apps".into(), label: "Windows apps".into(), path: APPS_PAGE.into(), kind: "folder".into(), section: "pc".into(), depth: 2, tree: "c-apps".into(), ..Default::default() });
                }
            }
        }
        let net_open = *self.nav_open.get("network").unwrap_or(&false);
        out.push(NavItem { id: "netroot".into(), label: "Network".into(), kind: "network".into(), section: "pc".into(), tree: "network".into(), open: net_open, empty: devs.is_empty(), ..Default::default() });
        if net_open {
            for q in devs.iter().copied() {
                out.push(item(q, "pc", if q.get("child") == Some(&Value::Bool(true)) { 2 } else { 1 }));
            }
        }
        // Linux (mockup files-1): folded at the bottom, "hidden" unless View › Linux folders is on (not in the Open /
        // Save window: "no Linux folders", mockup cdrive-1).
        if !self.picking() {
            let lx_open = *self.nav_open.get("linux").unwrap_or(&false) && self.c_linux();
            out.push(NavItem { id: "linux".into(), label: "Linux".into(), kind: "linux".into(), section: "pc".into(), tree: "linux".into(), open: lx_open, sub: if self.c_linux() { String::new() } else { "hidden".into() }, ..Default::default() });
            if lx_open {
                for (id, p) in [("lx-root", "/"), ("lx-home", "/home"), ("lx-usr", "/usr"), ("lx-etc", "/etc"), ("lx-var", "/var")] {
                    out.push(NavItem { id: id.into(), label: p.into(), path: p.into(), kind: "folder".into(), section: "pc".into(), depth: 1, ..Default::default() });
                }
            }
        }
        out
    }

    /// Quick access on the Home page: your folders and what you pinned.
    pub fn quick_access(&self) -> Vec<&NavItem> {
        self.places
            .iter()
            .filter(|q| !q.path.is_empty() && q.path != HOME_PAGE && q.depth == 0 && q.section == "folders" && q.kind != "trash")
            .take(15)
            .collect()
    }

    fn toggle_nav(&mut self, k: &str) {
        let def = matches!(k, "thispc" | "cdrive");
        let was = *self.nav_open.get(k).unwrap_or(&def);
        self.nav_open.insert(k.into(), !was);
        let at = self.places.get(self.place_index).map(|q| q.id.clone()).unwrap_or_default();
        self.places = self.build_nav();
        self.place_index = self.places.iter().position(|q| q.id == at).unwrap_or(0);
    }

    // ------------------------------------------------------------------ moving between folders
    pub fn navigate(&mut self, p: &str, sel: &str) -> Task<Msg> {
        self.zone = Zone::Main;
        self.editing_path = false;
        self.load(p, sel, true)
    }

    pub fn reload(&mut self) -> Task<Msg> {
        let sel = self.current().map(|e| e.name.clone()).unwrap_or_default();
        let p = self.path.clone();
        if p.is_empty() {
            return Task::none();
        }
        self.load(&p, &sel, false)
    }

    fn enter(&mut self, p: &str, push: bool) {
        let prev = self.path.clone();
        if prev != p {
            if push && !prev.is_empty() {
                self.back.push(prev);
                if self.back.len() > 100 {
                    self.back.remove(0);
                }
                self.fwd.clear();
            }
            self.query.clear();
            self.search_text.clear();
            self.search = None;
            self.found = None;
            self.marked.clear();
            self.anchor = None;
            self.cur = 0;
            self.scroll_y = 0.0;
            self.renaming = None;
            self.touched = false;
        }
    }

    pub fn load(&mut self, p: &str, sel: &str, push: bool) -> Task<Msg> {
        let ctx = self.ctx();
        if p == HOME_PAGE || p == THIS_PC || p == APPS_PAGE || !drive::c_page(p, &ctx).is_empty() {
            self.load_seq += 1;
            self.loading = false;
            let was = self.path.clone();
            self.enter(p, push);
            let pg = drive::c_page(p, &ctx);
            let name = if p == HOME_PAGE { "Home".to_string() } else if p == THIS_PC { "This PC".into() } else if p == APPS_PAGE { "Installed apps".into() } else {
                match pg { "root" => self.system_label(), "pf" => "Program Files".into(), "x86" => "Program Files (x86)".into(), _ => "AppData".into() }
            };
            self.listing = Some(Listing::page(p, &name));
            self.path = p.to_string();
            let mut tasks = vec![];
            if p == HOME_PAGE {
                tasks.push(Task::perform(backend::call(a(&["recent"]), 15), Msg::Recent));
            } else if p == THIS_PC {
                tasks.push(Task::perform(backend::call(a(&["this-pc"]), 20), Msg::ThisPc));
            } else if p == APPS_PAGE {
                tasks.push(self.load_apps());
            } else {
                tasks.push(Task::perform(backend::call(a(&["cdrive"]), 20), Msg::CDrive));
                if !self.apps_loading && (pg == "pf" || pg == "x86" || (pg == "root" && self.app_list.is_empty())) {
                    tasks.push(self.load_apps());
                }
            }
            self.rebuild();
            if !sel.is_empty() {
                if let Some(i) = self.entries.iter().position(|e| e.name == sel) {
                    self.cur = i;
                }
            }
            if was != p {
                tasks.push(operation::scroll_to(SCROLL_ID, operation::AbsoluteOffset { x: None, y: Some(0.0) }));
            }
            return Task::batch(tasks);
        }
        if drive::is_c_path(p) {
            return match drive::c_resolve(p, &ctx) {
                drive::Res::Path(r) if !drive::is_c_path(&r) => self.load(&r, sel, push),
                drive::Res::Error(e) => self.say(&e, true),
                _ => self.say("There's nothing called that on Hearth (C:)", true),
            };
        }
        self.load_seq += 1;
        self.loading = true;
        let seq = self.load_seq;
        let mut args = a(&["list", p]);
        if self.hidden {
            args.push("--hidden".into());
        }
        let (pp, ss) = (p.to_string(), sel.to_string());
        Task::perform(backend::call(args, 30), move |r| Msg::Listed(seq, pp.clone(), ss.clone(), push, r))
    }

    fn load_apps(&mut self) -> Task<Msg> {
        self.apps_loading = true;
        Task::perform(backend::call(a(&["installed-apps"]), 60), Msg::Apps)
    }

    fn listed(&mut self, seq: u64, p: String, sel: String, push: bool, r: Reply) -> Task<Msg> {
        if seq != self.load_seq {
            return Task::none();
        }
        self.loading = false;
        let mut l = if r.ok() { Listing::from_json(&r.value) } else { Listing { path: p.clone(), name: base(&p), error: r.err.clone(), ..Default::default() } };
        if l.path.is_empty() {
            l.path = p.clone();
        }
        let prev = self.path.clone();
        let changed = l.path != prev;
        self.enter(&l.path, push);
        if changed {
            let lp = l.path.clone();
            self.apply_folder_view(&lp);
            // A Briefcase's own default view and sort (Customize), unless you chose one for this folder.
            let mine = self.prefs.get("folders").and_then(|f| f.get(&lp)).is_some();
            if l.briefcase.is_object() && !mine && happ::arg("--view").is_none() {
                if let Some(v) = l.briefcase.get("view").and_then(Value::as_str).and_then(View::from) {
                    self.view = v;
                }
                let so = sv(&l.briefcase, "sort");
                if ["name", "date", "size", "type"].contains(&so.as_str()) {
                    self.sort = so;
                }
            }
        }
        let want = if !l.select.is_empty() { l.select.clone() } else { sel };
        self.path = l.path.clone();
        self.listing = Some(l);
        self.rebuild();
        if !want.is_empty() {
            if let Some(i) = self.entries.iter().position(|e| e.name == want) {
                self.cur = i;
            }
        }
        let mut tasks = vec![self.want_thumbs(), crate::extras::folder_loaded(self), crate::briefcase::opened(self)];
        if changed {
            tasks.push(operation::scroll_to(SCROLL_ID, operation::AbsoluteOffset { x: None, y: Some(0.0) }));
        } else {
            tasks.push(self.show_current());
        }
        // CI's pictures (--show menu,view,select,props,rename; any of them, comma-separated).
        let has = |k: &str| self.show.split(',').any(|x| x == k);
        if has("menu") && self.overlay.is_none() && !self.entries.is_empty() {
            // The right-click menu (mockup files-7): on the second item, where the mockup has it.
            self.show.clear();
            self.cur = 1.min(self.entries.len() - 1);
            self.touched = true;
            self.cursor = Point::new(382.0, 236.0);
            self.open_menu(false);
        } else if has("sort") && self.overlay.is_none() {
            // Sort my Downloads' preview (mockup sort-downloads-1), or with sorted: done, with its note (mockup 2).
            let then = has("sorted");
            self.show.clear();
            tasks.push(crate::sortdl::update(self, crate::sortdl::SMsg::Open));
            if then {
                tasks.push(crate::sortdl::update(self, crate::sortdl::SMsg::Go));
            }
        } else if has("view") && self.overlay.is_none() {
            self.show.clear();
            self.cursor = Point::new(422.0, 116.0);
            self.view_menu();
        } else if has("props") && !self.entries.is_empty() {
            self.show.clear();
            self.cur = 0;
            let p = self.entries[0].path.clone();
            tasks.push(self.properties(vec![p]));
        } else if has("rename") && !self.entries.is_empty() {
            self.show.clear();
            self.cur = 0;
            self.touched = true;
            tasks.push(crate::menus::run_menu(self, "rename"));
        } else if has("select") && !self.entries.is_empty() {
            self.cur = 2.min(self.entries.len() - 1);
            self.touched = true;
            self.marked.insert(self.entries[self.cur].path.clone());
            tasks.push(self.details_lookups());
            self.show = self.show.split(',').filter(|x| *x != "select").collect::<Vec<_>>().join(",");
            tasks.push(self.show_more());
        } else {
            tasks.push(self.show_more());
        }
        Task::batch(tasks)
    }

    // ------------------------------------------------------------------ the selection
    pub fn targets(&self) -> Vec<String> {
        if self.on_special() {
            return vec![];
        }
        if !self.marked.is_empty() {
            return self.entries.iter().filter(|e| self.marked.contains(&e.path) && !drive::is_c_path(&e.path)).map(|e| e.path.clone()).collect();
        }
        match self.current() {
            Some(c) if !drive::is_c_path(&c.path) => vec![c.path.clone()],
            _ => vec![],
        }
    }

    pub fn select_range(&mut self, from: usize, to: usize) {
        self.marked.clear();
        for i in from.min(to)..=from.max(to) {
            if let Some(e) = self.entries.get(i) {
                self.marked.insert(e.path.clone());
            }
        }
    }

    pub fn show_current(&self) -> Task<Msg> {
        let (_w, h) = self.area.get();
        let cols = self.cols.get().max(1);
        let v = self.shown_view();
        let (row, rh, top) = match v {
            View::Details => (self.cur, ROW_H, if self.on_home() { self.home_top() } else { 30.0 }),
            View::Compact => (self.cur % self.rows_per_col(), ROW_H, 8.0),
            _ => (self.cur / cols, v.cell().1 + 8.0, 8.0),
        };
        if self.on_this_pc() || v == View::Compact {
            return Task::none();
        }
        let y0 = top + row as f32 * rh;
        let y1 = y0 + rh;
        let off = self.scroll_y;
        if y0 < off {
            return operation::scroll_to(SCROLL_ID, operation::AbsoluteOffset { x: None, y: Some((y0 - 4.0).max(0.0)) });
        }
        if y1 > off + h {
            return operation::scroll_to(SCROLL_ID, operation::AbsoluteOffset { x: None, y: Some(y1 - h + 4.0) });
        }
        Task::none()
    }

    /// The Home page's list starts under Quick access.
    pub fn home_top(&self) -> f32 {
        let cols = 3usize;
        let n = self.quick_access().len();
        let rows = n.div_ceil(cols).max(1);
        14.0 + 24.0 + rows as f32 * 62.0 + 20.0 + 24.0
    }

    pub fn rows_per_col(&self) -> usize {
        ((self.area.get().1 - 12.0) / ROW_H).floor().max(1.0) as usize
    }

    // ------------------------------------------------------------------ pictures
    pub fn want_thumbs(&mut self) -> Task<Msg> {
        let v = self.shown_view();
        let mut want: Vec<(String, i64)> = Vec::new();
        let mut add = |e: &Entry, want: &mut Vec<(String, i64)>, thumbs: &HashMap<String, (i64, Option<Handle>, u32, u32, String)>, pending: &HashSet<String>| {
            if model::kind_of(e) == "image" && e.path.starts_with('/') && !pending.contains(&e.path) && thumbs.get(&e.path).map(|t| t.0 != e.mtime).unwrap_or(true) {
                want.push((e.path.clone(), e.mtime));
            }
        };
        if matches!(v, View::XLarge | View::Large | View::Grid | View::Tiles) && !self.on_special() && !self.on_home() {
            let cols = self.cols.get().max(1);
            let (_, h) = self.area.get();
            let rh = v.cell().1 + 8.0;
            let first = ((self.scroll_y / rh).floor() as usize).saturating_mul(cols);
            let last = (((self.scroll_y + h) / rh).ceil() as usize + 1).saturating_mul(cols);
            for e in self.entries.iter().skip(first).take(last.saturating_sub(first)) {
                add(e, &mut want, &self.thumbs, &self.thumb_pending);
            }
        }
        if self.details_pane {
            if let Some(c) = self.entries.get(self.cur) {
                add(c, &mut want, &self.thumbs, &self.thumb_pending);
            }
        }
        if want.is_empty() {
            return Task::none();
        }
        // A few hundred pictures at most stay in memory.
        if self.thumbs.len() > 400 {
            self.thumbs.clear();
        }
        for w in &want {
            self.thumb_pending.insert(w.0.clone());
        }
        let max = if v == View::XLarge || self.details_pane { 420 } else { 260 };
        let mut tasks = vec![];
        for chunk in want.chunks(8) {
            tasks.push(Task::perform(crate::thumbs::load(chunk.to_vec(), max), Msg::Thumbs));
        }
        Task::batch(tasks)
    }

    /// What the details pane looks up for the item picked: its earlier versions (snapshots of your files).
    pub fn details_lookups(&mut self) -> Task<Msg> {
        let mut t = vec![self.want_thumbs()];
        if self.details_pane {
            let want = self.current().filter(|c| c.path.starts_with('/') && c.path != self.versions_of).map(|c| c.path.clone());
            if let Some(p) = want {
                t.push(self.look_for_versions(p, false));
            }
        }
        Task::batch(t)
    }

    pub fn look_for_versions(&mut self, path: String, menu: bool) -> Task<Msg> {
        self.versions_of = path.clone();
        self.versions.clear();
        let cmd = vec![
            "sh".to_string(),
            "-c".into(),
            "r=$(realpath -- \"$1\" 2>/dev/null) || exit 0; for base in /var/home /home; do case \"$r\" in \"$base\"/*) rel=${r#\"$base\"/}; for d in \"$base\"/.hearth-snapshots/*/; do [ -e \"$d$rel\" ] && echo \"$d$rel\"; done; break;; esac; done; true".into(),
            "sh".into(),
            path.clone(),
        ];
        Task::perform(backend::output(cmd, 10), move |o| Msg::Versions(path.clone(), menu, o))
    }

    // ------------------------------------------------------------------ opening
    pub fn activate(&mut self, i: usize) -> Task<Msg> {
        let Some(e) = self.entries.get(i).cloned() else { return Task::none() };
        if e.cv.is_some() {
            return self.navigate(&e.path, "");
        }
        if !e.pc.is_empty() {
            return self.open_pc_item(&e);
        }
        if e.app.is_some() {
            self.cur = i;
            self.open_menu(false);
            return Task::none();
        }
        if let Some(pk) = self.pick.clone() {
            if !e.dir {
                if pk.directory {
                    return Task::none();
                }
                if pk.kind == "open" {
                    if self.marked.contains(&e.path) && pk.multiple {
                        return self.pick_accept();
                    }
                    return self.pick_finish(vec![e.path.clone()]);
                }
                if pk.kind == "save" {
                    self.pick_name = e.name.clone();
                    self.zone = Zone::Pick;
                    return operation::focus(PICKNAME_ID);
                }
                return Task::none();
            }
        }
        if self.in_trash() && !self.picking() {
            self.cur = i;
            self.open_menu(false);
            return Task::none();
        }
        if e.dir {
            if self.found.is_some() {
                self.found = None;
                self.search = None;
            }
            return self.navigate(&e.path, "");
        }
        let low = e.name.to_lowercase();
        if low.ends_with(".appimage") {
            if e.exec {
                return self.run_appimage(&e.path, false);
            }
            self.confirm("Run this app?", &format!("“{}” comes from {}. Only run apps you trust: an app can do anything you can. Hearth marks it as runnable, so it won't ask again.", e.name, parent_of(&e.path).replacen(&self.home, "~", 1)), "Run", Yes::RunAppImage(e.path.clone()));
            if let Some(Overlay::Confirm(c)) = &mut self.overlay {
                c.danger = false;
            }
            return Task::none();
        }
        if low.ends_with(".exe") || low.ends_with(".msi") {
            backend::detached(&[String::from("hearth-winapps"), "open".into(), e.path.clone()], None);
            return self.say(&format!("Opening “{}” with Proton…", e.name), false);
        }
        if low.ends_with(".deb") || low.ends_with(".rpm") {
            backend::detached(&[String::from("hearth-install-package"), e.path.clone()], None);
            return Task::none();
        }
        Task::perform(backend::call(a(&["open-plan", &e.path]), 15), move |r| Msg::Did(Then::OpenPlan(e.clone()), r))
    }

    pub fn run_appimage(&mut self, path: &str, trust: bool) -> Task<Msg> {
        let mut args = a(&["appimage-run", path]);
        if trust {
            args.push("--trust".into());
        }
        let name = base(path);
        let t = self.say(&format!("Starting “{name}”…"), false);
        Task::batch(vec![t, Task::perform(backend::call(args, 30), move |r| Msg::Did(Then::Say(format!("Started “{name}”")), r))])
    }

    fn open_pc_item(&mut self, e: &Entry) -> Task<Msg> {
        let d = &e.item;
        if e.pc == "net" {
            if let Some(net) = d.get("net").filter(|n| n.is_object()).cloned() {
                return crate::extras::net_open(self, net, String::new(), String::new(), false);
            }
            if !sv(d, "path").is_empty() {
                return self.navigate(&sv(d, "path"), "");
            }
            return self.say(&format!("{}: open it from the Network in the left menu", e.name), false);
        }
        if d.get("mounted") == Some(&Value::Bool(false)) && !sv(d, "device").is_empty() {
            let label = e.name.clone();
            let t = self.say(&format!("Mounting “{label}”…"), false);
            return Task::batch(vec![t, Task::perform(backend::call(a(&["mount", &sv(d, "device")]), 60), move |r| Msg::Did(Then::Mounted(true, label.clone()), r))]);
        }
        if sv(d, "key") == "system" && self.cdrive_on() {
            return self.navigate(drive::C_ROOT, "");
        }
        if !sv(d, "path").is_empty() {
            return self.navigate(&sv(d, "path"), "");
        }
        Task::none()
    }

    pub fn open_place(&mut self, i: usize) -> Task<Msg> {
        let Some(p) = self.places.get(i).cloned() else { return Task::none() };
        self.place_index = i;
        if p.kind == "tag" {
            let t = sv(&p.raw, "tag");
            return crate::extras::open_tag(self, &t);
        }
        // Your devices' shared folders: one click opens one (signing in the first time).
        if let Some(net) = p.raw.get("net").filter(|n| n.is_object()).cloned() {
            return crate::extras::net_open(self, net, String::new(), String::new(), false);
        }
        if !p.tree.is_empty() && p.path.is_empty() {
            if p.empty && (p.tree == "homeserver" || p.tree == "network") {
                return self.say("No home server or shared folders yet: Settings › Home server sets one up", false);
            }
            if p.tree == "linux" && !self.c_linux() {
                return self.say("The Linux folders are hidden: turn on View › Linux folders to see them", false);
            }
            self.toggle_nav(&p.tree);
            return Task::none();
        }
        if p.path.is_empty() && !sv(&p.raw, "device").is_empty() {
            let label = p.label.clone();
            return Task::perform(backend::call(a(&["mount", &sv(&p.raw, "device")]), 60), move |r| Msg::Did(Then::Mounted(true, label.clone()), r));
        }
        if p.path.is_empty() {
            return Task::none();
        }
        self.navigate(&p.path, "")
    }

    pub fn go_up(&mut self, close_at_top: bool) -> Task<Msg> {
        if self.found.is_some() {
            self.found = None;
            self.search = None;
            self.rebuild();
            return Task::none();
        }
        if self.listing.is_none() {
            return Task::none();
        }
        if self.on_home() || self.on_this_pc() {
            return if close_at_top { self.close() } else { Task::none() };
        }
        if self.on_apps() {
            return self.navigate(THIS_PC, "");
        }
        let cc = self.crumbs();
        if self.on_cview() || (self.cdrive_on() && !self.addr_linux() && cc.len() >= 2 && cc[1].path == drive::C_ROOT) {
            if cc.len() >= 2 {
                let (to, lab) = (cc[cc.len() - 2].path.clone(), cc[cc.len() - 1].label.clone());
                return self.navigate(&to, &lab);
            }
            return self.navigate(THIS_PC, "");
        }
        let p = self.path.clone();
        if p == "/" || self.raw_places.iter().any(|q| !sv(q, "letter").is_empty() && sv(q, "path") == p) {
            return self.navigate(THIS_PC, "");
        }
        if p == self.home {
            return if close_at_top { self.close() } else { Task::none() };
        }
        let up = parent_of(&p);
        self.navigate(&up, &base(&p))
    }

    pub fn go_back(&mut self) -> Task<Msg> {
        let Some(to) = self.back.pop() else { return Task::none() };
        let from = self.path.clone();
        self.fwd.push(from.clone());
        self.zone = Zone::Main;
        let sel = if model::under(&from, &to) && from != to { from[if to == "/" { 1 } else { to.len() + 1 }..].split('/').next().unwrap_or("").to_string() } else { String::new() };
        self.load(&to, &sel, false)
    }

    pub fn go_forward(&mut self) -> Task<Msg> {
        let Some(to) = self.fwd.pop() else { return Task::none() };
        self.back.push(self.path.clone());
        self.zone = Zone::Main;
        self.load(&to, "", false)
    }

    pub fn close(&mut self) -> Task<Msg> {
        if crate::pad::wanted() {
            crate::pad::quit();
        }
        if self.picking() {
            return self.pick_cancel();
        }
        iced::exit()
    }

    /// CI's pictures of the terminal panel, split view, Act as administrator, Tags ›, Sharing, the Briefcase's
    /// dialogs, signing in to a shared folder, a drag and the Open / Save window's keyboard (--show, comma-separated):
    /// each opens what it names once the folder is listed, the way a click would. Shown, never saved.
    fn show_more(&mut self) -> Task<Msg> {
        let list: Vec<String> = self.show.split(',').map(String::from).collect();
        if list.iter().all(|x| x.is_empty() || x == "widenav" || x == "pane" || x == "tabs") {
            return Task::none();
        }
        let has = |k: &str| list.iter().any(|x| x == k);
        self.show.clear();
        let mut t = vec![];
        if has("term") {
            t.push(crate::terminal::show(self, true));
            self.term.input = "ls -l".into();
            t.push(crate::terminal::update(self, crate::terminal::TMsg::Submit));
        }
        if has("split") {
            t.push(crate::split::update(self, crate::split::SMsg::Toggle));
            let d = format!("{}/Pictures", self.home);
            t.push(crate::split::load(self, &d));
        }
        if has("admin") {
            t.push(crate::extras::update(self, crate::extras::XMsg::AdminOn));
        }
        if has("tagmenu") && !self.entries.is_empty() {
            self.cur = 0;
            self.touched = true;
            let p = self.entries[0].path.clone();
            self.cursor = Point::new(430.0, 210.0);
            crate::extras::tag_menu(self, vec![p]);
        }
        if has("sharing") {
            if let Some(e) = self.entries.iter().find(|e| e.dir && e.cv.is_none() && e.path.starts_with('/')).cloned() {
                t.push(crate::extras::run(self, "x-share", vec![e.path.clone()], Some(e)));
            }
        }
        if has("menu-bc") && !self.entries.is_empty() {
            if self.bc.st.is_some() {
                self.cur = 0;
                self.touched = true;
                self.cursor = Point::new(420.0, 150.0);
                self.open_menu(false);
            } else {
                // Its status isn't in yet: when it comes (briefcase.rs).
                self.show = "menu-bc".into();
            }
        }
        for k in ["bc-update-all", "bc-setlock", "bc-customize", "bc-addto"] {
            if has(k) {
                let tg = if k == "bc-addto" { self.entries.iter().find(|e| !e.dir).map(|e| vec![e.path.clone()]).unwrap_or_default() } else { vec![] };
                t.push(crate::briefcase::run(self, k, tg));
            }
        }
        if has("bc-password") {
            let p = self.path.clone();
            t.push(crate::briefcase::run(self, &format!("bc-unlock:{p}"), vec![]));
        }
        if has("netsign") {
            match self.raw_places.iter().find_map(|q| q.get("net").filter(|n| n.is_object()).cloned()) {
                Some(net) => t.push(crate::extras::net_open(self, net, String::new(), String::new(), false)),
                // The left menu isn't in yet: when it comes (Msg::Places).
                None => self.show = "netsign".into(),
            }
        }
        if has("drag") {
            if let (Some(src), Some(dst)) = (self.entries.iter().find(|e| !e.dir).cloned(), self.entries.iter().find(|e| e.dir && e.cv.is_none() && e.path.starts_with('/')).cloned()) {
                self.touched = true;
                self.cur = self.entries.iter().position(|e| e.path == src.path).unwrap_or(0);
                self.drag = Some(crate::dnd::Drag { paths: vec![src.path.clone()], label: src.name.clone(), at: Point::new(470.0, 205.0), from_split: false, item: Some(dst.path.clone()), area: None });
            }
        }
        if has("osk") {
            t.push(crate::pad::open_kb(self));
        }
        Task::batch(t)
    }

    // ------------------------------------------------------------------ tabs
    fn save_tab(&mut self) {
        let sel = self.current().map(|e| e.name.clone()).unwrap_or_default();
        if let Some(t) = self.tabs.get_mut(self.tab_index) {
            *t = Tab { path: self.path.clone(), back: self.back.clone(), fwd: self.fwd.clone(), sel };
        }
    }
    pub fn new_tab(&mut self, p: &str) -> Task<Msg> {
        self.save_tab();
        let path = if p.is_empty() { self.start_page() } else { p.to_string() };
        self.tabs.push(Tab { path: path.clone(), ..Default::default() });
        self.tab_index = self.tabs.len() - 1;
        self.back.clear();
        self.fwd.clear();
        self.zone = Zone::Main;
        self.load(&path, "", false)
    }
    pub fn switch_tab(&mut self, i: usize) -> Task<Msg> {
        if i == self.tab_index || i >= self.tabs.len() {
            return Task::none();
        }
        self.save_tab();
        self.tab_index = i;
        let t = self.tabs[i].clone();
        self.back = t.back;
        self.fwd = t.fwd;
        self.load(&t.path, &t.sel, false)
    }
    pub fn close_tab(&mut self, i: usize) -> Task<Msg> {
        if self.tabs.len() <= 1 {
            return self.close();
        }
        self.save_tab();
        self.tabs.remove(i);
        if i == self.tab_index {
            self.tab_index = i.min(self.tabs.len() - 1);
            let n = self.tabs[self.tab_index].clone();
            self.back = n.back;
            self.fwd = n.fwd;
            return self.load(&n.path, &n.sel, false);
        }
        if i < self.tab_index {
            self.tab_index -= 1;
        }
        Task::none()
    }

    // ------------------------------------------------------------------ changing files
    pub fn set_clip(&mut self, mode: &str, t: Vec<String>) -> Task<Msg> {
        if t.is_empty() {
            return self.say("Pick something first", false);
        }
        if mode == "cut" && !self.writable() {
            return Task::none();
        }
        self.clip_mode = mode.into();
        self.clip = t.clone();
        let mut args = vec!["clip-set".to_string(), mode.into()];
        args.extend(t.iter().cloned());
        let call = Task::perform(backend::call(args, 10), |r| Msg::Did(Then::Nothing, r));
        let msg = if mode == "cut" {
            "Open the folder they should go to and choose Move here.".to_string()
        } else if t.len() == 1 {
            format!("Copied “{}”. Open a folder and choose Paste.", base(&t[0]))
        } else {
            format!("Copied {} items. Open a folder and choose Paste.", t.len())
        };
        Task::batch(vec![call, self.say(&msg, false)])
    }

    pub fn c_refusal_here(&self) -> String {
        let p = &self.path;
        let ctx = self.ctx();
        if drive::is_c_path(p) {
            return drive::c_refusal(p, &ctx);
        }
        if self.cdrive_on() { drive::c_refusal(p, &ctx) } else { String::new() }
    }

    pub fn paste(&mut self) -> Task<Msg> {
        if !self.writable() {
            let why = self.c_refusal_here();
            return self.say(if why.is_empty() { "You can't paste here" } else { &why }, !why.is_empty());
        }
        let cut = self.clip_mode == "cut";
        Task::perform(backend::call(a(&["clip-get"]), 10), move |r| Msg::Did(Then::Clip(cut), r))
    }

    pub fn transfer_to(&mut self, verb: &str, paths: Vec<String>, dest: String) -> Task<Msg> {
        if paths.is_empty() {
            return Task::none();
        }
        let mut args = vec!["conflicts".to_string()];
        args.extend(paths.iter().cloned());
        args.push(dest.clone());
        let v = verb.to_string();
        Task::perform(backend::call(args, 30), move |r| Msg::Did(Then::Conflicts(v.clone(), paths.clone(), dest.clone()), r))
    }

    pub fn trash_paths(&mut self, t: Vec<String>) -> Task<Msg> {
        if t.is_empty() {
            return Task::none();
        }
        let mut args = vec!["trash".to_string()];
        args.extend(t.iter().cloned());
        Task::perform(backend::call(args, 60), move |r| Msg::Did(Then::Trashed(t.clone()), r))
    }

    pub fn push_undo(&mut self, op: Op) {
        self.undo.push(op);
        if self.undo.len() > 50 {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    pub fn op_name(op: &Op) -> String {
        let n: Vec<String> = match op {
            Op::Rename { to, .. } => vec![to.clone()],
            Op::Create { items } | Op::Copy { items } | Op::Move { items, .. } => items.clone(),
            Op::Trash { paths } => paths.clone(),
            Op::Sort { moves } => moves.iter().map(|m| m.1.clone()).collect(),
        };
        let what = if n.len() == 1 { format!("“{}”", base(&n[0])) } else { format!("{} items", n.len()) };
        match op {
            Op::Rename { from, .. } => format!("Rename “{}”", base(from)),
            Op::Create { .. } => format!("New {what}"),
            Op::Trash { .. } => format!("Trash {what}"),
            Op::Copy { .. } => format!("Copy {what}"),
            Op::Move { .. } => format!("Move {what}"),
            Op::Sort { .. } => format!("Sort my Downloads ({what})"),
        }
    }

    pub fn run_undo(&mut self, is_undo: bool) -> Task<Msg> {
        let stack = if is_undo { &mut self.undo } else { &mut self.redo };
        let Some(op) = stack.pop() else { return self.say(if is_undo { "Nothing to undo" } else { "Nothing to redo" }, false) };
        if self.job.is_some() {
            if is_undo { self.undo.push(op) } else { self.redo.push(op) }
            return self.say("Wait for the copy or move to finish first", false);
        }
        let then = Then::Undone(is_undo, op.clone());
        match &op {
            Op::Rename { from, to } => {
                let (f, t) = if is_undo { (to.clone(), from.clone()) } else { (from.clone(), to.clone()) };
                Task::perform(backend::call(vec!["rename".into(), f, base(&t)], 30), move |r| Msg::Did(then.clone(), r))
            }
            Op::Trash { paths } => {
                let mut args = vec![if is_undo { "untrash" } else { "trash" }.to_string()];
                args.extend(paths.iter().cloned());
                Task::perform(backend::call(args, 60), move |r| Msg::Did(then.clone(), r))
            }
            Op::Create { items } | Op::Copy { items } => {
                let mut args = vec![if is_undo { "trash" } else { "untrash" }.to_string()];
                args.extend(items.iter().cloned());
                Task::perform(backend::call(args, 60), move |r| Msg::Did(then.clone(), r))
            }
            Op::Move { items, srcs, dest, back } => {
                let req = if is_undo {
                    JobReq { verb: "move".into(), paths: items.clone(), dest: back.clone(), decide: Map::new(), fmt: String::new(), archive: String::new(), here: false, undo: Some((op.clone(), true)) }
                } else {
                    JobReq { verb: "move".into(), paths: srcs.clone(), dest: dest.clone(), decide: Map::new(), fmt: String::new(), archive: String::new(), here: false, undo: Some((op.clone(), false)) }
                };
                self.start_job(req)
            }
            Op::Sort { moves } => {
                // Quick renames inside Downloads, done here; each only when nothing of that name is in its way.
                let n = crate::sortdl::swap(moves, is_undo);
                if is_undo { self.redo.push(op.clone()) } else { self.undo.push(op.clone()) }
                self.sorted = None;
                let msg = format!("{}{} ({} of {})", if is_undo { "Undone: " } else { "Redone: " }, App::op_name(&op), n, moves.len());
                let say = self.say(&msg, false);
                Task::batch(vec![self.reload(), say])
            }
        }
    }

    pub fn start_job(&mut self, j: JobReq) -> Task<Msg> {
        if j.paths.is_empty() {
            return Task::none();
        }
        if self.job.is_some() {
            self.job_queue.push(j);
            return self.say("That starts as soon as the current one is done", false);
        }
        let mut cmd: Vec<String> = match j.verb.as_str() {
            "compress" => {
                let mut c = vec!["compress".to_string(), j.fmt.clone(), j.archive.clone()];
                c.extend(j.paths.iter().cloned());
                c
            }
            "extract" => vec!["extract".into(), j.archive.clone(), if j.here { "--here".into() } else { j.dest.clone() }],
            v => {
                let mut c = vec![v.to_string()];
                c.extend(j.paths.iter().cloned());
                c.push(j.dest.clone());
                if !j.decide.is_empty() {
                    c.push("--decide".into());
                    c.push(Value::Object(j.decide.clone()).to_string());
                }
                c
            }
        };
        cmd = backend::argv(&cmd);
        let dest = if j.verb == "compress" { j.archive.clone() } else { j.dest.clone() };
        let (rp, rl) = self.root_of(&dest);
        let dest_name = if j.verb == "compress" { base(&j.archive) } else if rp == dest { rl } else { base(&dest) };
        self.seq += 1;
        self.job = Some(Job {
            seq: self.seq,
            name: if j.paths.len() == 1 { base(&j.paths[0]) } else { String::new() },
            dest_name,
            req: j,
            cmd,
            done: 0.0,
            total: 0.0,
            speed: 0.0,
            file: String::new(),
            cancelling: false,
            last: String::new(),
        });
        Task::none()
    }

    fn job_ended(&mut self) -> Task<Msg> {
        let Some(j) = self.job.take() else { return Task::none() };
        let last = j.last.clone();
        let mut sel = String::new();
        let mut tasks = vec![];
        let what = if !j.name.is_empty() { format!("“{}”", j.name) } else { format!("{} items", j.req.paths.len()) };
        if let Some(e) = last.strip_prefix("error: ") {
            tasks.push(self.say(e, true));
        } else {
            let r: Value = serde_json::from_str(&last).unwrap_or(Value::Null);
            if r.is_null() || r.get("cancelled").and_then(Value::as_bool) == Some(true) {
                tasks.push(self.say(if j.cancelling { "Stopped. Nothing half-done was left behind." } else { "It stopped before finishing" }, !j.cancelling));
            } else if j.req.verb == "compress" {
                tasks.push(self.say(&format!("Made “{}”", j.dest_name), false));
                sel = j.dest_name.clone();
            } else if j.req.verb == "extract" {
                let p = sv(&r, "path");
                tasks.push(self.say(&format!("Unpacked {what}{}", if p.is_empty() { String::new() } else { format!(" into “{}”", base(&p)) }), false));
                sel = if p.is_empty() { String::new() } else { base(&p) };
            } else if let Some((op, undoing)) = j.req.undo.clone() {
                let items: Vec<String> = r.get("items").and_then(Value::as_array).map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
                let op2 = match (&op, undoing) {
                    (Op::Move { srcs, dest, back, .. }, false) => Op::Move { items, srcs: srcs.clone(), dest: dest.clone(), back: back.clone() },
                    _ => op.clone(),
                };
                if undoing { self.redo.push(op2.clone()) } else { self.undo.push(op2.clone()) }
                tasks.push(self.say(&format!("{}{}", if undoing { "Undone: " } else { "Redone: " }, App::op_name(&op2)), false));
            } else {
                let skipped = r.get("skipped").and_then(Value::as_array).map(|s| s.len()).unwrap_or(0);
                tasks.push(self.say(&format!("{} {what} to “{}”{}", if j.req.verb == "move" { "Moved" } else { "Copied" }, j.dest_name, if skipped > 0 { format!(" (skipped {skipped})") } else { String::new() }), false));
                let items: Vec<String> = r.get("items").and_then(Value::as_array).map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
                if !items.is_empty() {
                    let parents: Vec<String> = j.req.paths.iter().map(|p| parent_of(p)).collect();
                    if j.req.verb == "copy" {
                        self.push_undo(Op::Copy { items });
                    } else if parents.iter().all(|x| *x == parents[0]) {
                        self.push_undo(Op::Move { items, srcs: j.req.paths.clone(), dest: j.req.dest.clone(), back: parents[0].clone() });
                    }
                }
            }
        }
        if self.listing.is_some() {
            let s = if sel.is_empty() { self.current().map(|e| e.name.clone()).unwrap_or_default() } else { sel };
            let p = self.path.clone();
            tasks.push(self.load(&p, &s, false));
        }
        tasks.push(self.refresh_places());
        tasks.push(crate::split::reload(self));
        if !self.job_queue.is_empty() {
            let n = self.job_queue.remove(0);
            tasks.push(self.start_job(n));
        }
        Task::batch(tasks)
    }

    // ------------------------------------------------------------------ Properties
    pub fn properties(&mut self, paths: Vec<String>) -> Task<Msg> {
        if paths.is_empty() {
            return Task::none();
        }
        self.overlay = Some(Overlay::Props(Props {
            paths: paths.clone(),
            info: Value::Null,
            loaded: false,
            tab: 0,
            name: if paths.len() == 1 { base(&paths[0]) } else { String::new() },
            mode: 0,
            mode0: 0,
            sums: Map::new(),
            compare: String::new(),
            count: Value::Null,
            error: String::new(),
            cview: None,
            share: Value::Null,
            share_busy: false,
        }));
        let mut args = vec!["props".to_string()];
        args.extend(paths.iter().cloned());
        Task::perform(backend::call(args, 30), move |r| Msg::Did(Then::Props(paths.clone()), r))
    }

    /// Properties of one of Hearth (C:)'s own folders (FilesCProps.qml): what it is and where it really is.
    pub fn c_props(&mut self, e: Option<Entry>) -> Task<Msg> {
        if let Some(e) = &e {
            if e.path.starts_with('/') {
                return self.properties(vec![e.path.clone()]);
            }
        }
        let e = e.unwrap_or_else(|| Entry { name: self.listing.as_ref().map(|l| l.name.clone()).unwrap_or_default(), path: self.path.clone(), dir: true, ..Default::default() });
        let win = e.cv.as_ref().map(|c| c.win.clone()).unwrap_or_else(|| self.win_path(&e.path));
        self.overlay = Some(Overlay::Props(Props {
            paths: vec![e.path.clone()],
            info: Value::Null,
            loaded: true,
            tab: 0,
            name: e.name.clone(),
            mode: 0,
            mode0: 0,
            sums: Map::new(),
            compare: String::new(),
            count: Value::Null,
            error: String::new(),
            cview: Some((e, win)),
            share: Value::Null,
            share_busy: false,
        }));
        Task::none()
    }
}

fn size_rank(e: &Entry) -> i64 {
    if e.dir {
        return 0;
    }
    let s = e.size;
    if s < 16384 {
        1
    } else if s < 1_048_576 {
        2
    } else if s < 134_217_728 {
        3
    } else if s < 1_073_741_824 {
        4
    } else {
        5
    }
}

/// The Open / Save window's answer, written whole (then renamed) so hearth-portal never reads half of it.
fn write_result(v: &Value) {
    let Ok(p) = std::env::var("HEARTH_PICK_RESULT") else {
        println!("{v}");
        return;
    };
    let part = format!("{p}.part");
    if std::fs::write(&part, v.to_string()).is_ok() {
        let _ = std::fs::rename(&part, &p);
    }
}

// ------------------------------------------------------------------ update
pub fn update(a: &mut App, m: Msg) -> Task<Msg> {
    let t = update_inner(a, m);
    t
}

fn update_inner(app: &mut App, m: Msg) -> Task<Msg> {
    match m {
        Msg::Theme => app.p = hearth_ui::theme::load(),
        Msg::Drag => return win::drag(),
        Msg::Maximize => return win::toggle_maximize(),
        Msg::Minimize => return win::minimize(),
        Msg::Close => return app.close(),
        Msg::Resize(d) => return win::resize(d),
        Msg::Cursor(p) => {
            app.cursor = p;
            crate::dnd::moved(app, p);
            // The window opened before its events were listened to: its handles are asked for now.
            if !app.picking() && crate::dragout::ask_once() {
                return iced::window::latest()
                    .then(|id| match id {
                        Some(id) => iced::window::run(id, |w| crate::dragout::handles_of(w)),
                        None => Task::done(None),
                    })
                    .map(Msg::DragOutReady);
            }
        }
        Msg::Key(ev) => return crate::menus::key(app, ev),
        Msg::FileDropped(p) => {
            let dest = app.path.clone();
            if !app.writable() {
                let why = app.c_refusal_here();
                return app.say(if why.is_empty() { "You can't put files here" } else { &why }, true);
            }
            return app.transfer_to("copy", vec![p.to_string_lossy().to_string()], dest);
        }
        Msg::NoteClear(s) => {
            if s == app.note_seq {
                app.note.clear();
            }
        }
        Msg::Prefs(r, first) => {
            if !r.ok() && first {
                // files.json couldn't be read: nothing is saved over it (red flag 9); the defaults are only shown.
                app.prefs_loaded = true;
            } else {
                app.apply_prefs(&r.value);
            }
            app.places = app.build_nav();
            let mut tasks = vec![];
            if first {
                if app.cdrive_on() {
                    tasks.push(Task::perform(backend::call(a(&["cdrive"]), 20), Msg::CDrive));
                }
                let target = if let Some(pk) = &app.pick {
                    if pk.folder.is_empty() { app.home.clone() } else { pk.folder.clone() }
                } else if !app.start_target.is_empty() {
                    app.start_target.clone()
                } else if let Some(pg) = happ::arg("--page") {
                    match pg.as_str() {
                        "home" => HOME_PAGE.into(),
                        "thispc" => THIS_PC.into(),
                        "apps" => APPS_PAGE.into(),
                        "c" => drive::C_ROOT.into(),
                        p if p.starts_with('~') => format!("{}{}", app.home, &p[1..]),
                        p => p.to_string(),
                    }
                } else {
                    app.start_page()
                };
                let mut target = target;
                if let Some(t) = target.strip_prefix("file://") {
                    target = t.to_string();
                }
                if let Some(t) = target.strip_prefix('~') {
                    target = format!("{}{}", app.home, t);
                }
                if !target.starts_with('/') && !target.starts_with("hearth:") {
                    if let Some(drive::Res::Path(p)) = drive::from_windows(&target, &app.ctx()) {
                        target = p;
                    } else {
                        target = std::env::current_dir().map(|d| d.join(&target).to_string_lossy().to_string()).unwrap_or(target);
                    }
                }
                if app.props_only {
                    tasks.push(app.properties(vec![target.clone()]));
                }
                tasks.push(app.navigate(&target, ""));
                if app.show.split(',').any(|x| x == "pane") {
                    app.details_pane = true;
                }
                if app.show.split(',').any(|x| x == "tabs") && !app.picking() {
                    for t in ["Downloads", "Saved Games"] {
                        app.tabs.push(Tab { path: format!("{}/{t}", app.home), ..Default::default() });
                    }
                }
                if app.picking() && app.pick.as_ref().map(|p| p.kind == "save").unwrap_or(false) {
                    app.zone = Zone::Pick;
                    let n = app.pick_name.clone();
                    let dot = n.rfind('.').filter(|d| *d > 0).unwrap_or(n.len());
                    tasks.push(operation::focus(PICKNAME_ID));
                    tasks.push(operation::select_range(PICKNAME_ID, 0, n[..dot].chars().count()));
                }
            }
            app.rebuild();
            return Task::batch(tasks);
        }
        Msg::Places(r) => {
            if let Some(list) = r.get("places").as_array() {
                let old = std::mem::take(&mut app.raw_places);
                app.raw_places = list.clone();
                app.places = app.build_nav();
                app.saved_games = app
                    .raw_places
                    .iter()
                    .find(|q| sv(q, "id") == "savedgames")
                    .and_then(|q| std::fs::read_dir(sv(q, "path")).ok())
                    .map(|d| d.filter_map(|x| x.ok()).filter(|x| x.file_type().map(|t| t.is_dir()).unwrap_or(false)).count())
                    .unwrap_or(0);
                app.place_index = app.place_index.min(app.places.len().saturating_sub(1));
                let mut tasks = vec![];
                let sig = |l: &[Value]| -> String { l.iter().filter(|q| sv(q, "section") == "drives").map(|q| format!("{}{}", sv(q, "id"), if q.get("mounted") == Some(&Value::Bool(false)) { "-" } else { "+" })).collect::<Vec<_>>().join("|") };
                if app.on_this_pc() && sig(&old) != sig(&app.raw_places) {
                    tasks.push(Task::perform(backend::call(a(&["this-pc"]), 20), Msg::ThisPc));
                }
                for o in &old {
                    let op = sv(o, "path");
                    if sv(o, "section") != "drives" || op.is_empty() || app.raw_places.iter().any(|n| sv(n, "path") == op) {
                        continue;
                    }
                    if model::under(&app.path, &op) {
                        tasks.push(app.say(&format!("“{}” was removed", sv(o, "label")), false));
                        let h = app.home.clone();
                        tasks.push(app.navigate(&h, ""));
                    }
                }
                app.rebuild();
                if app.show == "netsign" && app.listing.is_some() {
                    if let Some(net) = app.raw_places.iter().find_map(|q| q.get("net").filter(|n| n.is_object()).cloned()) {
                        app.show.clear();
                        tasks.push(crate::extras::net_open(app, net, String::new(), String::new(), false));
                    }
                }
                return Task::batch(tasks);
            }
        }
        Msg::Listed(seq, p, sel, push, r) => return app.listed(seq, p, sel, push, r),
        Msg::Recent(r) => {
            if app.on_home() {
                app.recent = r.get("recent").as_array().map(|a| a.iter().map(Entry::from_json).collect()).unwrap_or_default();
                for e in app.recent.iter_mut() {
                    if e.mtime == 0 || !e.opened.is_empty() {
                        let t = model::parse_iso(&e.opened);
                        if t > 0 {
                            e.mtime = t;
                        }
                    }
                }
                app.rebuild();
            }
        }
        Msg::ThisPc(r) => {
            if r.ok() {
                app.pc = r.value;
                app.rebuild();
            } else {
                return app.say(&r.err, true);
            }
        }
        Msg::CDrive(r) => {
            if r.ok() {
                app.cdata = r.value;
                app.rebuild();
            }
        }
        Msg::Apps(r) => {
            app.apps_loading = false;
            if r.ok() {
                app.app_list = r.get("apps").as_array().cloned().unwrap_or_default();
                app.rebuild();
            } else {
                return app.say(&r.err, true);
            }
        }
        Msg::Templates(r) => {
            if let Some(t) = r.get("templates").as_array() {
                app.templates = t.clone();
            }
        }
        Msg::Did(then, r) => return crate::menus::did(app, then, r),
        Msg::Watch(l) => {
            if let Line::Text(_, w) = l {
                match w.trim() {
                    "gone" => {
                        let up = parent_of(&app.path);
                        let t = app.say("This folder was moved or deleted", false);
                        return Task::batch(vec![t, app.navigate(&up, "")]);
                    }
                    "changed" => {
                        if app.renaming.is_none() && app.found.is_none() && !app.loading {
                            return app.reload();
                        }
                    }
                    _ => {}
                }
            }
        }
        Msg::Mounts(l) => {
            if let Line::Text(_, w) = l {
                if w.trim() == "mounts" {
                    let mut t = vec![app.refresh_places()];
                    if app.on_this_pc() {
                        t.push(Task::perform(backend::call(a(&["this-pc"]), 20), Msg::ThisPc));
                    }
                    return Task::batch(t);
                }
            }
        }
        Msg::Found(l) => match l {
            Line::Text(seq, s) => {
                if app.search.as_ref().map(|x| x.seq) != Some(seq) {
                    return Task::none();
                }
                if let Some(j) = s.strip_prefix("found: ") {
                    if let Ok(v) = serde_json::from_str::<Value>(j) {
                        let n = {
                            let f = app.found.get_or_insert_with(Vec::new);
                            f.push(Entry::from_json(&v));
                            f.len()
                        };
                        if n < 200 || n % 100 == 0 {
                            app.rebuild();
                        }
                    }
                } else if let Some(e) = s.strip_prefix("error: ") {
                    return app.say(e, true);
                } else if let Ok(v) = serde_json::from_str::<Value>(&s) {
                    if v.get("truncated").and_then(Value::as_bool) == Some(true) {
                        return app.say(&format!("Showing the first {} matches", v.get("count").and_then(Value::as_u64).unwrap_or(0)), false);
                    }
                }
            }
            Line::End(seq, _) => {
                if app.search.as_ref().map(|x| x.seq) == Some(seq) {
                    app.search = None;
                    app.rebuild();
                }
            }
        },
        Msg::Job(l) => match l {
            Line::Text(seq, s) => {
                if let Some(j) = app.job.as_mut().filter(|j| j.seq == seq) {
                    if let Some(p) = s.strip_prefix("progress: ") {
                        if let Ok(v) = serde_json::from_str::<Value>(p) {
                            j.done = v.get("done").and_then(Value::as_f64).unwrap_or(j.done);
                            j.total = v.get("total").and_then(Value::as_f64).unwrap_or(j.total);
                            j.speed = v.get("speed").and_then(Value::as_f64).unwrap_or(0.0);
                            j.file = v.get("file").and_then(Value::as_str).unwrap_or("").to_string();
                        }
                    } else if !s.trim().is_empty() {
                        j.last = s.trim().to_string();
                    }
                }
            }
            Line::End(seq, _) => {
                if app.job.as_ref().map(|j| j.seq) == Some(seq) {
                    return app.job_ended();
                }
            }
        },
        Msg::StopJob => {
            if let Some(j) = app.job.as_mut() {
                j.cancelling = true;
                app.job_queue.clear();
                // Dropping its subscription stops it (SIGTERM: hearth-files removes the half-done item); the end is
                // then reported here.
                return app.job_ended();
            }
        }
        Msg::Thumbs(list) => {
            for t in list {
                app.thumb_pending.remove(&t.path);
                app.thumbs.insert(t.path.clone(), (t.mtime, t.handle, t.w, t.h, t.taken));
            }
            // The first look happened before the window knew how many fit in a row: the rest on screen now.
            if app.thumb_pending.is_empty() {
                return app.want_thumbs();
            }
        }
        Msg::Versions(p, menu, out) => {
            if p == app.versions_of {
                let mut v: Vec<String> = out.lines().filter(|l| !l.trim().is_empty()).map(String::from).collect();
                v.sort();
                v.reverse();
                app.versions = v;
                if menu {
                    crate::menus::versions_menu(app);
                    if app.overlay.is_none() {
                        return app.say(&format!("No earlier versions of “{}” yet. Turn on Snapshots of your files in Task Manager › Disk › Health & settings.", base(&p)), false);
                    }
                }
            }
        }
        Msg::Go(p) => return app.navigate(&p, ""),
        Msg::Back => return app.go_back(),
        Msg::Forward => return app.go_forward(),
        Msg::Up => return app.go_up(false),
        Msg::Refresh => return Task::batch(vec![app.reload(), app.refresh_places()]),
        Msg::EditPath => {
            app.editing_path = true;
            app.zone = Zone::Toolbar;
            app.path_text = app.address_text(&app.path.clone());
            return Task::batch(vec![operation::focus(PATH_ID), operation::select_all(PATH_ID)]);
        }
        Msg::PathText(t) => app.path_text = t,
        Msg::PathGo => return crate::menus::end_edit_path(app, true),
        Msg::SearchText(t) => {
            if app.found.is_some() && !t.is_empty() {
                app.found = None;
                app.search = None;
            }
            app.search_text = t.clone();
            app.query = t;
            app.cur = 0;
            app.rebuild();
        }
        Msg::SearchGo => {
            let q = app.search_text.trim().to_string();
            if q.is_empty() || app.listing.is_none() || app.in_trash() || app.on_cview() || !app.path.starts_with('/') {
                return Task::none();
            }
            app.found = Some(vec![]);
            app.found_query = q.clone();
            app.query.clear();
            app.search_text.clear();
            app.marked.clear();
            app.cur = 0;
            app.zone = Zone::Main;
            app.seq += 1;
            let mut args = a(&["search", &app.path.clone(), &q]);
            if app.hidden {
                args.push("--hidden".into());
            }
            app.search = Some(Search { seq: app.seq, cmd: backend::argv(&args) });
            app.rebuild();
        }
        Msg::SearchClear => {
            app.found = None;
            app.search = None;
            app.query.clear();
            app.search_text.clear();
            app.rebuild();
        }
        Msg::Tab(i) => return app.switch_tab(i),
        Msg::CloseTab(i) => return app.close_tab(i),
        Msg::NewTab => {
            let sp = if app.picking() { app.path.clone() } else { app.start_page() };
            return app.new_tab(&sp);
        }
        Msg::Place(i) => return app.open_place(i),
        Msg::PlaceRight(i) => {
            app.place_index = i;
            app.zone = Zone::Places;
            crate::menus::place_menu(app);
        }
        Msg::PlaceMiddle(i) => {
            if let Some(p) = app.places.get(i).map(|p| p.path.clone()).filter(|p| !p.is_empty()) {
                return app.new_tab(&p);
            }
        }
        Msg::Fold(k) => app.toggle_nav(&k),
        Msg::Quick(p) => return app.navigate(&p, ""),
        Msg::QuickMiddle(p) => return app.new_tab(&p),
        Msg::Press(i) => {
            app.zone = Zone::Main;
            app.touched = true;
            let now = std::time::Instant::now();
            let double = app.last_click.0 == i && now.duration_since(app.last_click.1).as_millis() < 450;
            app.last_click = (i, now);
            if double {
                app.last_click = (usize::MAX, now);
                return app.activate(i);
            }
            if app.mods.control() {
                if let Some(e) = app.entries.get(i) {
                    let p = e.path.clone();
                    if !app.marked.remove(&p) {
                        app.marked.insert(p);
                    }
                }
                app.anchor = Some(i);
            } else if app.mods.shift() {
                let from = app.anchor.unwrap_or(app.cur);
                app.select_range(from, i);
            } else {
                app.marked.clear();
                app.anchor = Some(i);
            }
            app.cur = i;
            if app.renaming.is_some() {
                return crate::menus::finish_rename(app);
            }
            // A drag starts from here once the pointer moves (dnd.rs): what's picked, or this item.
            if !app.on_special() && !app.on_home() {
                let t = app.targets();
                let name = app.entries.get(i).map(|e| e.name.clone()).unwrap_or_default();
                crate::dnd::pending(app, t, &name, false);
            }
            return app.details_lookups();
        }
        Msg::Double(i) => return app.activate(i),
        Msg::Right(i) => {
            app.touched = true;
            if let Some(e) = app.entries.get(i) {
                if !app.marked.contains(&e.path) {
                    app.marked.clear();
                }
            }
            app.cur = i;
            app.open_menu(false);
        }
        Msg::Middle(i) => {
            if let Some(e) = app.entries.get(i).cloned() {
                if e.dir && !e.pc.is_empty() || e.dir && e.path.starts_with('/') {
                    return app.new_tab(&e.path);
                }
            }
        }
        Msg::BgPress => {
            app.zone = Zone::Main;
            if !app.mods.control() && !app.mods.shift() {
                app.marked.clear();
            }
            if app.renaming.is_none() {
                app.band = Some((app.band_at, app.band_at, app.marked.clone()));
            }
            if app.renaming.is_some() {
                return crate::menus::finish_rename(app);
            }
        }
        Msg::BgRight => {
            app.marked.clear();
            app.open_menu(true);
        }
        Msg::Scrolled(y) => {
            app.scroll_y = y;
            return app.want_thumbs();
        }
        Msg::Cmd(c) => return crate::menus::command(app, c),
        Msg::MenuPick(i) => {
            let id = match &app.overlay {
                Some(Overlay::Menu(m)) => m.items.get(i).filter(|it| !it.sep && !it.heading).map(|it| it.id.clone()),
                _ => None,
            };
            if let Some(id) = id {
                return crate::menus::run_menu(app, &id);
            }
        }
        Msg::MenuClose => {
            if matches!(app.overlay, Some(Overlay::Menu(_))) {
                app.overlay = None;
                app.menu_on_folder = false;
            }
        }
        Msg::DlgText(t) => match &mut app.overlay {
            Some(Overlay::Input(i)) => i.text = t,
            _ => {}
        },
        Msg::DlgOk => return crate::menus::dialog_ok(app),
        Msg::DlgCancel => {
            if matches!(app.overlay, Some(Overlay::Conflict(_))) {
                app.overlay = None;
                return app.say("Nothing was copied or moved", false);
            }
            app.overlay = None;
            if app.props_only {
                return app.close();
            }
        }
        Msg::DlgButton(b) => {
            if let Some(Overlay::Confirm(c)) = &mut app.overlay {
                c.button = b;
            }
        }
        Msg::ConflictPick(c) => return crate::menus::conflict_choose(app, c),
        Msg::ConflictAll => {
            if let Some(Overlay::Conflict(c)) = &mut app.overlay {
                c.all = !c.all;
            }
        }
        Msg::OpenWith(i) => return crate::menus::open_with(app, Some(i)),
        Msg::OtherApps => return crate::menus::open_with(app, None),
        Msg::RenameText(t) => app.rename_text = t,
        Msg::RenameDone => return crate::menus::finish_rename(app),
        Msg::Wallpaper => {
            if let Some(e) = app.current().cloned() {
                backend::detached(&[String::from("hearth-wallpaper"), e.path.clone()], None);
                return app.say(&format!("“{}” is your wallpaper now", e.name), false);
            }
        }
        Msg::OpenWithMenu => {
            if let Some(e) = app.current().cloned() {
                return crate::menus::show_apps(app, &e, false);
            }
        }
        Msg::PropsTab(i) => {
            if let Some(Overlay::Props(p)) = &mut app.overlay {
                p.tab = i;
                let tabs = crate::view::props_tabs(p);
                if tabs.get(i) == Some(&"Sharing") && p.share.is_null() && p.paths.len() == 1 {
                    return crate::extras::load_sharing(p.paths[0].clone());
                }
            }
        }
        Msg::PropsName(n) => {
            if let Some(Overlay::Props(p)) = &mut app.overlay {
                p.name = n;
            }
        }
        Msg::PropsBit(b) => {
            if let Some(Overlay::Props(p)) = &mut app.overlay {
                p.mode ^= b;
            }
        }
        Msg::PropsApply(close) => return crate::menus::props_apply(app, close),
        Msg::PropsCopy(linux) => {
            if let Some(Overlay::Props(p)) = &app.overlay {
                let path = p.paths.first().cloned().unwrap_or_default();
                let t = if let Some((_, win)) = &p.cview { win.clone() } else { app.copy_form(&path, linux) };
                return crate::menus::copy_text(app, &t);
            }
        }
        Msg::Checksum(algo) => {
            if let Some(Overlay::Props(p)) = &mut app.overlay {
                let path = p.paths.first().cloned().unwrap_or_default();
                p.sums.insert(algo.into(), Value::String("…".into()));
                let al = algo.to_string();
                return Task::perform(backend::call(vec!["checksum".into(), algo.into(), path], 600), move |r| Msg::Did(Then::Sum(al.clone()), r));
            }
        }
        Msg::CompareText(t) => {
            if let Some(Overlay::Props(p)) = &mut app.overlay {
                p.compare = t;
            }
        }
        Msg::PickName(n) => app.pick_name = n,
        Msg::PickAccept => return app.pick_accept(),
        Msg::PickCancel => return app.pick_cancel(),
        Msg::PickFilter => crate::menus::filter_menu(app),
        Msg::PickChoice(i) => return crate::menus::choice_run(app, i),
        Msg::DlgAlt => {
            if let Some(Overlay::Confirm(c)) = app.overlay.clone() {
                if let Some((_, y)) = c.alt {
                    app.overlay = None;
                    return crate::menus::run_yes(app, y);
                }
            }
        }
        Msg::Term(m) => return crate::terminal::update(app, m),
        Msg::Split(m) => return crate::split::update(app, m),
        Msg::Fx(m) => return crate::extras::update(app, m),
        Msg::Bc(m) => return crate::briefcase::update(app, m),
        Msg::Pad(m) => return crate::pad::update(app, m),
        Msg::Sort(m) => return crate::sortdl::update(app, m),
        Msg::Describe(m) => return app.describe.update(m).map(Msg::Describe),
        Msg::DescAct(act) => return crate::ldescribe::act(app, &act),
        Msg::SortNoteGone(n) => {
            if app.sorted_seq == n {
                app.sorted = None;
            }
        }
        Msg::BandMove(p) => {
            app.band_at = p;
            if let Some(b) = app.band.as_mut() {
                b.1 = p;
                app.band_select();
            }
        }
        Msg::Dnd(m) => {
            if matches!(m, crate::dnd::DMsg::Release) {
                app.band = None;
            }
            return crate::dnd::update(app, m);
        }
        Msg::WinOpened(id, w, h) => {
            app.win = (w, h);
            // Dragging out to other apps: on the window's own Wayland connection (dragout.rs).
            if !app.picking() && crate::dragout::ask_once() {
                return iced::window::run(id, |w| crate::dragout::handles_of(w)).map(Msg::DragOutReady);
            }
        }
        Msg::WinSize(w, h) => app.win = (w, h),
        Msg::DragOutReady(h) => {
            if let Some((d, s)) = h {
                crate::dragout::start(d, s);
            }
        }
        Msg::DragOut(o) => match o {
            crate::dragout::Out::Started(_) => {
                // The system has the drag now: Files' own ends.
                app.drag = None;
                app.drag_pending = None;
            }
            crate::dragout::Out::Ended(_) => {}
        },
    }
    Task::none()
}

// ------------------------------------------------------------------ what the window listens to
fn watch_target(a: &App) -> Option<String> {
    if a.listing.as_ref().map(|l| l.error.is_empty()).unwrap_or(false) && a.found.is_none() && a.path.starts_with('/') && !a.loading {
        Some(a.path.clone())
    } else {
        None
    }
}

pub fn subscription(a: &App) -> Subscription<Msg> {
    let mut subs = vec![
        Subscription::run(hearth_ui::theme::watch).map(|_| Msg::Theme),
        iced::keyboard::listen().map(Msg::Key),
        iced::event::listen_with(|ev, _status, _id| match ev {
            iced::Event::Window(iced::window::Event::FileDropped(p)) => Some(Msg::FileDropped(p)),
            _ => None,
        }),
        // Drives plugged in or taken out (hearth-files watch --mounts: poll() on /proc/self/mounts, no cost at rest).
        Subscription::run_with(Run { seq: 0, cmd: backend::argv(&a_(&["watch", "--mounts"])) }, backend::lines).map(Msg::Mounts),
    ];
    // The open folder updates by itself (inotify in hearth-files watch: nothing runs while nothing changes).
    if let Some(p) = watch_target(a) {
        subs.push(Subscription::run_with(Run { seq: 1, cmd: backend::argv(&a_(&["watch", &p])) }, backend::lines).map(Msg::Watch));
    }
    if let Some(s) = &a.search {
        subs.push(Subscription::run_with(Run { seq: s.seq, cmd: s.cmd.clone() }, backend::lines).map(Msg::Found));
    }
    if let Some(j) = &a.job {
        if !j.cancelling {
            subs.push(Subscription::run_with(Run { seq: j.seq, cmd: j.cmd.clone() }, backend::lines).map(Msg::Job));
        }
    }
    // The window's size, and the news of a drag out to another app (dragout.rs).
    subs.push(iced::event::listen_with(window_events));
    subs.push(Subscription::run(crate::dragout::events).map(Msg::DragOut));
    // The terminal panel's command, while it runs; Esc and Ctrl+C on its line (its text box keeps them otherwise).
    if let Some(r) = &a.term.run {
        subs.push(Subscription::run_with(r.clone(), crate::terminal::lines).map(|l| Msg::Term(crate::terminal::TMsg::Out(l))));
    }
    if a.term.on && a.zone == Zone::Term {
        subs.push(iced::event::listen_with(crate::terminal::captured_keys));
    }
    // An unlocked, locked Briefcase counts down to locking by itself: its status once a minute while it's shown.
    if crate::briefcase::in_briefcase(a) && crate::briefcase::locked(a) && crate::briefcase::unlocked(a) {
        subs.push(iced::time::every(std::time::Duration::from_secs(60)).map(|_| Msg::Bc(crate::briefcase::BMsg::Tick)));
    }
    // Game Mode's Open / Save window: its own hearth-pad, and the keyboard's frames while a button is held.
    if a.picking() && crate::pad::wanted() && happ::arg("--show").is_none() {
        subs.push(Subscription::run(crate::pad::stream).map(|v| Msg::Pad(crate::pad::PMsg::Ev(v))));
    }
    if a.pad.kb && a.pad.busy {
        subs.push(iced::time::every(std::time::Duration::from_millis(16)).map(|_| Msg::Pad(crate::pad::PMsg::Tick)));
    }
    Subscription::batch(subs)
}

fn window_events(e: iced::Event, _s: iced::event::Status, id: iced::window::Id) -> Option<Msg> {
    match e {
        iced::Event::Window(iced::window::Event::Opened { size, .. }) => Some(Msg::WinOpened(id, size.width, size.height)),
        iced::Event::Window(iced::window::Event::Resized(size)) => Some(Msg::WinSize(size.width, size.height)),
        _ => None,
    }
}

fn a_(v: &[&str]) -> Vec<String> {
    a(v)
}

pub fn title(a: &App) -> String {
    if let Some(p) = &a.pick {
        return p.title.clone();
    }
    if a.path.is_empty() { "Hearth Files".into() } else { format!("{} – Hearth Files", a.title_of(&a.path)) }
}

pub fn run(mode: Mode) -> iced::Result {
    let picking = matches!(mode, Mode::Pick(_));
    let (id, size, min) = if picking { ("hearth-file-chooser", (1000.0, 640.0), (560.0, 380.0)) } else { ("hearth-files", (1180.0, 740.0), (640.0, 420.0)) };
    iced::application(move || App::new(mode.clone()), update, crate::view::view)
        .title(title)
        .subscription(subscription)
        .theme(|a: &App| happ::iced_theme(&a.p))
        .window(happ::window(id, size, min))
        .default_font(happ::INTER)
        .run()
}

/// One key per real folder (the left menu's favourites shown once): a file:// URI, a trailing slash or a link to the
/// same folder don't make two. Empty for an item without a path (it's never merged with another).
pub fn place_key(p: &str) -> String {
    let mut p = p.trim().to_string();
    if p.is_empty() {
        return p;
    }
    if let Some(rest) = p.strip_prefix("file://") {
        p = percent_decode(rest);
    }
    while p.len() > 1 && p.ends_with('/') {
        p.pop();
    }
    std::fs::canonicalize(&p).map(|c| c.to_string_lossy().into_owned()).unwrap_or(p)
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            let hex = |c: u8| (c as char).to_digit(16);
            if let (Some(h), Some(l)) = (hex(b[i + 1]), hex(b[i + 2])) {
                out.push((h * 16 + l) as u8);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod place_key_tests {
    use super::place_key;

    #[test]
    fn same_folder_one_key() {
        let d = std::env::temp_dir().join(format!("hearth-files-place-key-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        let docs = d.join("My Docs");
        std::fs::create_dir(&docs).unwrap();
        let link = d.join("link");
        std::os::unix::fs::symlink(&docs, &link).unwrap();
        let k = place_key(docs.to_str().unwrap());
        assert_eq!(place_key(&format!("{}/", docs.display())), k);
        assert_eq!(place_key(&format!("file://{}", docs.display()).replace(' ', "%20")), k);
        assert_eq!(place_key(link.to_str().unwrap()), k);
        assert_eq!(place_key(""), "");
        let _ = std::fs::remove_dir_all(&d);
    }
}
