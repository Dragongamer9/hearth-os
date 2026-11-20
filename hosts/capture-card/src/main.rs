//! Capture Card (hearth-capture-card): your Xbox (or anything with HDMI) on this PC's screen through a USB capture
//! card, so the owner can play on the laptop's screen. A Hearth Store app (version 1.0). Built exactly to the
//! approved mockups (docs/mockups/capture-card, checklist docs/mockups/checklists/capture-card.md). On the desktop
//! it's a window that goes full screen (desk.rs); in Game Mode it opens full screen with a quick menu (game.rs).
//! Keyboard, touchpad or a controller paired to this PC all work (the Xbox's controller keeps playing the Xbox).
//!
//! Light at rest: nothing polls. Cards are looked for when the app opens and when /dev changes (watch.rs); the
//! picture runs only on Live view; the recordings' list is read when its page opens. Timers run only while a
//! recording is going (its pill's clock) and for the bar's and hints' hiding. Opening the app or a page writes none
//! of your settings.

mod desk;
mod game;
mod look;
mod pad;

use cc::card::Card;
use cc::library::{self, Item, Kind};
use cc::prefs::{self, Prefs};
use cc::sound::{self, Source};
use cc::video::{self, Event as VEv};
use hearth_ui::app::{self, win};
use iced::keyboard::{key::Named, Event as KEvent, Key};
use iced::widget::image;
use iced::{window, Subscription, Task};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// The app's version (the owner's number, Nov 20, 2026).
pub const VERSION: &str = "1.0";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Live,
    Library,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Rec,
    Shot,
    All,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dd {
    Card,
    Size,
    Sound,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Dlg {
    Rename(PathBuf),
    Delete(PathBuf),
}

/// Where the picture stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sig {
    /// Looking (the app just opened).
    Looking,
    NoCard,
    /// The card is found; the picture is starting.
    Starting,
    Live,
    NoSignal,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Mode {
    pub format: String,
    pub w: u32,
    pub h: u32,
    pub fps: u32,
    pub decoder: String,
}

impl Mode {
    /// "1080p60".
    pub fn short(&self) -> String {
        format!("{}p{}", self.h, self.fps)
    }
    pub fn format_words(&self) -> &'static str {
        if self.format == "YUYV" {
            "YUYV"
        } else {
            "MJPEG"
        }
    }
}

pub struct App {
    pub game: bool,
    pub page: Page,
    pub prefs: Prefs,
    pub cards: Vec<Card>,
    pub card: Option<Card>,
    pub sources: Vec<Source>,
    /// The card's own microphone (or your choice), when found.
    pub sound: Option<Source>,
    pub sig: Sig,
    pub mode: Option<Mode>,
    pub frame: Option<image::Handle>,
    pub delay_ms: Option<u32>,
    delay_acc: (u64, u32, Instant),
    /// The last delay measured (kept in the cache, for the sidebar on other pages).
    pub last_live: Option<(String, u32)>,
    last_saved: Option<Instant>,
    pub bar: bool,
    pub bar_gen: u64,
    pub pill: bool,
    pub pill_gen: u64,
    pub full: bool,
    went_full: bool,
    pub tip: bool,
    pub shot: Option<(PathBuf, image::Handle)>,
    pub shot_gen: u64,
    pub rec: Option<(Instant, PathBuf)>,
    pub rec_size: u64,
    pub rec_free: String,
    pub menu: bool,
    pub menu_focus: usize,
    pub focus: usize,
    pub ring: bool,
    pub dd: Option<Dd>,
    pub dlg: Option<Dlg>,
    pub rename_text: String,
    pub items: Vec<Item>,
    pub tab: Tab,
    pub sel: usize,
    pub thumbs: HashMap<PathBuf, image::Handle>,
    pub preview: Option<(PathBuf, image::Handle)>,
    pub store_chip: (String, bool),
    pub note: Option<String>,
    pub show: Vec<String>,
    inhibit: Option<std::process::Child>,
    pub laptop: bool,
    b_gen: u64,
    b_down: bool,
    retry: u32,
    pub size: (f32, f32),
    pub encoders: (Option<String>, Option<String>),
    note_gen: u64,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Key(KEvent),
    Pad(pad::Button),
    Mouse,
    Ev((u64, VEv)),
    Changed,
    Cards(Vec<Card>, Vec<Source>),
    Retry(u32),
    Drag,
    Max,
    Min,
    Close,
    Resize(window::Direction),
    Go(Page),
    Tab(Tab),
    Sort,
    Select(usize),
    Set(&'static str, Value),
    Dd(Option<Dd>),
    Pick(Dd, Value),
    Volume(u32),
    VolumeDone,
    Mute,
    Fit(&'static str),
    BarHide(u64),
    PillHide(u64),
    Screenshot,
    Shot(Result<PathBuf, String>),
    ShotHide(u64),
    ShowFile(PathBuf),
    CopyFile(PathBuf),
    Copied(bool),
    Play(PathBuf),
    Record,
    Tick,
    RecSize(u64, String),
    FullScreen,
    Tip(bool),
    Menu,
    OpenDir(PathBuf),
    ChangeDir(&'static str),
    PickedDir(&'static str, Option<String>),
    OpenStore,
    Items(Vec<Item>),
    Filled(Vec<Item>),
    Ask(Dlg),
    RenameText(String),
    DoRename,
    DoDelete,
    Done(Result<(), String>),
    CloseDlg,
    BHeld(u64),
    Leave,
    NoteOff,
    NoteHide(u64),
    Size(f32, f32),
    None,
}

// ------------------------------------------------------------------ small helpers
/// Runs `f` on its own thread; its answer becomes a message.
fn work<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static, then: impl Fn(T) -> Msg + Send + 'static) -> Task<Msg> {
    let (tx, rx) = futures::channel::oneshot::channel();
    let _ = std::thread::Builder::new().name("cc-work".into()).spawn(move || {
        let _ = tx.send(f());
    });
    Task::perform(async move { rx.await.ok() }, move |r| match r {
        Some(v) => then(v),
        None => Msg::None,
    })
}

fn after(ms: u64, m: Msg) -> Task<Msg> {
    Task::perform(hearth_ui::run::after(ms), move |_| m.clone())
}

fn has_battery() -> bool {
    std::fs::read_dir("/sys/class/power_supply").map(|d| d.flatten().any(|e| e.file_name().to_string_lossy().starts_with("BAT"))).unwrap_or(false)
}

/// Game Mode (gamescope) or a desktop window.
fn game_mode() -> bool {
    app::flag("--game-mode")
        || std::env::var_os("GAMESCOPE_WAYLAND_DISPLAY").is_some()
        || std::env::var("XDG_CURRENT_DESKTOP").map(|d| d.to_lowercase().contains("gamescope")).unwrap_or(false)
}

/// What the Store knows about this app: Up to date, or an update is ready (its own files; read only).
fn store_chip() -> (String, bool) {
    let data = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|| cc::home().join(".local/share"));
    let cache = std::env::var_os("XDG_CACHE_HOME").map(PathBuf::from).unwrap_or_else(|| cc::home().join(".cache"));
    let read = |p: PathBuf| -> Value { std::fs::read(p).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null) };
    let have = read(data.join("hearth/store/apps/capture-card/store.json"))["version"].as_str().unwrap_or("").to_string();
    let seen = read(cache.join("hearth/hearth-store/releases.json"));
    let newest = seen["apps"]["capture-card"]["version"].as_str().unwrap_or("").to_string();
    if have.is_empty() {
        return ("Not installed from the Store".into(), false);
    }
    if !newest.is_empty() && newest != have {
        return ("Update ready".into(), false);
    }
    ("Up to date".into(), true)
}

/// The cards and the microphones (a worker).
fn scan() -> (Vec<Card>, Vec<Source>) {
    let cards = cc::card::list();
    let sources = if cards.is_empty() { Vec::new() } else { sound::sources() };
    (cards, sources)
}

fn last_file() -> PathBuf {
    cc::cache_dir().join("last.json")
}

/// The last picture's mode and delay (a cache, not a setting: the sidebar shows it on the other pages).
fn read_last() -> Option<(String, u32)> {
    let v: Value = serde_json::from_slice(&std::fs::read(last_file()).ok()?).ok()?;
    Some((v["mode"].as_str()?.to_string(), v["delay"].as_u64()? as u32))
}

/// Asks for a folder (KDE's or GNOME's chooser), with a time limit.
fn pick_folder(start: &Path) -> Option<String> {
    let s = start.to_string_lossy().to_string();
    let out = if cc::have("kdialog") {
        cc::output(&["kdialog", "--getexistingdirectory", &s, "--title", "Choose a folder"], 600)
    } else if cc::have("zenity") {
        cc::output(&["zenity", "--file-selection", "--directory", "--filename", &format!("{s}/"), "--title", "Choose a folder"], 600)
    } else {
        None
    };
    out.map(|o| o.trim().to_string()).filter(|o| !o.is_empty())
}

/// Shows a file in the file manager (selected), else opens its folder.
fn show_file(p: &Path) {
    let uri = format!("file://{}", p.display());
    let arg = format!("['{}']", uri.replace('\'', "%27"));
    let shown = cc::output(&["gdbus", "call", "--session", "--dest", "org.freedesktop.FileManager1", "--object-path", "/org/freedesktop/FileManager1", "--method", "org.freedesktop.FileManager1.ShowItems", &arg, ""], 5).is_some();
    if !shown {
        let dir = p.parent().unwrap_or(p).to_string_lossy().to_string();
        hearth_ui::run::detached(&["xdg-open", &dir]);
    }
}

fn copy_file(p: &Path) -> bool {
    let png = p.extension().map(|e| e.eq_ignore_ascii_case("png")).unwrap_or(false);
    let ty = if png { "image/png" } else { "text/uri-list" };
    if !png {
        let tmp = cc::cache_dir().join("copy.txt");
        if cc::write_atomic(&tmp, format!("file://{}\n", p.display()).as_bytes()).is_err() {
            return false;
        }
        return cc::run_with_input(&["wl-copy", "--type", ty], &tmp, 5);
    }
    cc::run_with_input(&["wl-copy", "--type", ty], p, 5) || cc::run_with_input(&["xclip", "-selection", "clipboard", "-t", ty, "-i"], p, 5)
}

fn play_file(p: &Path) {
    let s = p.to_string_lossy().to_string();
    if cc::have("hearth-player") {
        hearth_ui::run::detached(&["hearth-player", &s]);
    } else {
        hearth_ui::run::detached(&["xdg-open", &s]);
    }
}

/// Saves a screenshot: Pictures › Screenshots › Capture YYYY-MM-DD HH-MM-SS.png.
fn save_shot(dir: PathBuf, px: Option<(u32, u32, Vec<u8>)>) -> Result<PathBuf, String> {
    let (w, h, rgba) = px.ok_or("There's no picture to save yet.")?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("Couldn't make the Screenshots folder: {e}"))?;
    let mut file = dir.join(format!("{}.png", cc::stamp_name(cc::now())));
    let mut n = 2;
    while file.exists() {
        file = dir.join(format!("{} ({n}).png", cc::stamp_name(cc::now())));
        n += 1;
    }
    let img = ::image::RgbaImage::from_raw(w, h, rgba).ok_or("There's no picture to save yet.")?;
    let tmp = file.with_extension("png.part");
    img.save_with_format(&tmp, ::image::ImageFormat::Png).map_err(|e| format!("Couldn't save the screenshot: {e}"))?;
    std::fs::rename(&tmp, &file).map_err(|e| format!("Couldn't save the screenshot: {e}"))?;
    Ok(file)
}

// ------------------------------------------------------------------ start
impl App {
    fn new() -> (App, Task<Msg>) {
        let prefs = prefs::load();
        let game = game_mode();
        let page = match app::arg("--page").as_deref() {
            Some("library") | Some("recordings") | Some("screenshots") => Page::Library,
            Some("settings") => Page::Settings,
            _ => Page::Live,
        };
        let tab = match app::arg("--page").as_deref() {
            Some("screenshots") => Tab::Shot,
            _ => match app::arg("--tab").as_deref() {
                Some("screenshots") => Tab::Shot,
                Some("everything") => Tab::All,
                _ => Tab::Rec,
            },
        };
        let show: Vec<String> = app::arg("--show").map(|s| s.split(',').map(String::from).collect()).unwrap_or_default();
        let a = App {
            game,
            page,
            prefs,
            cards: Vec::new(),
            card: None,
            sources: Vec::new(),
            sound: None,
            sig: Sig::Looking,
            mode: None,
            frame: None,
            delay_ms: None,
            delay_acc: (0, 0, Instant::now()),
            last_live: read_last(),
            last_saved: None,
            bar: false,
            bar_gen: 0,
            pill: true,
            pill_gen: 0,
            full: game,
            went_full: false,
            tip: false,
            shot: None,
            shot_gen: 0,
            rec: None,
            rec_size: 0,
            rec_free: String::new(),
            menu: false,
            menu_focus: 0,
            focus: 0,
            ring: game,
            dd: None,
            dlg: None,
            rename_text: String::new(),
            items: Vec::new(),
            tab,
            sel: 0,
            thumbs: HashMap::new(),
            preview: None,
            store_chip: ("Up to date".into(), true),
            note: None,
            show,
            inhibit: None,
            laptop: has_battery(),
            b_gen: 0,
            b_down: false,
            retry: 0,
            size: (1920.0, 1080.0),
            encoders: (None, None),
            note_gen: 0,
        };
        let mut a = a;
        // The cards, and the list's counts (the sidebar and the quick menu show them).
        let mut t = vec![work(scan, |(c, s)| Msg::Cards(c, s)), load_items(&a)];
        if page == Page::Settings {
            a.store_chip = store_chip();
        }
        if a.shows("menu") {
            a.menu = true;
        }
        (a, Task::batch(t))
    }

    pub fn shows(&self, what: &str) -> bool {
        self.show.iter().any(|s| s == what)
    }

    pub fn recs(&self) -> usize {
        self.items.iter().filter(|i| i.kind == Kind::Rec).count()
    }
    pub fn shots(&self) -> usize {
        self.items.iter().filter(|i| i.kind == Kind::Shot).count()
    }

    /// The tab's files, in the order chosen.
    pub fn shown(&self) -> Vec<&Item> {
        let mut v: Vec<&Item> = self
            .items
            .iter()
            .filter(|i| match self.tab {
                Tab::Rec => i.kind == Kind::Rec,
                Tab::Shot => i.kind == Kind::Shot,
                Tab::All => true,
            })
            .collect();
        if self.prefs.sort == "oldest" {
            v.reverse();
        }
        v
    }

    pub fn picked(&self) -> Option<&Item> {
        self.shown().get(self.sel).copied()
    }

    pub fn rec_dir(&self) -> PathBuf {
        prefs::rec_dir(&self.prefs)
    }
    pub fn shot_dir(&self) -> PathBuf {
        prefs::shot_dir(&self.prefs)
    }

    /// The sound in use: your choice, else the card's own microphone.
    fn pick_sound(&mut self) {
        self.sound = match (&self.card, self.prefs.sound.is_empty()) {
            (_, false) => self.sources.iter().find(|s| s.name == self.prefs.sound).cloned(),
            (Some(c), true) => sound::for_card(c, &self.sources).cloned(),
            (None, true) => None,
        };
    }
}

fn load_items(_a: &App) -> Task<Msg> {
    let (r, s) = (_a.rec_dir(), _a.shot_dir());
    work(move || library::scan(&r, &s), Msg::Items)
}

// ------------------------------------------------------------------ what each control does
/// One thing the focus can be on: A presses it, ◀ ▶ change it.
pub struct T {
    pub press: Option<Msg>,
    pub left: Option<Msg>,
    pub right: Option<Msg>,
}

fn tp(m: Msg) -> T {
    T { press: Some(m), left: None, right: None }
}

fn flip(key: &'static str, on: bool) -> T {
    T { press: Some(Msg::Set(key, json!(!on))), left: Some(Msg::Set(key, json!(!on))), right: Some(Msg::Set(key, json!(!on))) }
}

fn cycle_n(key: &'static str, opts: &[u32], now: u32) -> T {
    let i = opts.iter().position(|o| *o == now).unwrap_or(opts.len() - 1);
    let l = opts[(i + opts.len() - 1) % opts.len()];
    let r = opts[(i + 1) % opts.len()];
    T { press: Some(Msg::Set(key, json!(r))), left: Some(Msg::Set(key, json!(l))), right: Some(Msg::Set(key, json!(r))) }
}

fn cycle_s(key: &'static str, opts: &[&str], now: &str) -> T {
    let i = opts.iter().position(|o| *o == now).unwrap_or(0);
    let l = opts[(i + opts.len() - 1) % opts.len()];
    let r = opts[(i + 1) % opts.len()];
    T { press: Some(Msg::Set(key, json!(r))), left: Some(Msg::Set(key, json!(l))), right: Some(Msg::Set(key, json!(r))) }
}

fn fit_t(now: &str) -> T {
    let opts = ["fit", "fill", "stretch"];
    let i = opts.iter().position(|o| *o == now).unwrap_or(0);
    T { press: Some(Msg::Fit(opts[(i + 1) % 3])), left: Some(Msg::Fit(opts[(i + 2) % 3])), right: Some(Msg::Fit(opts[(i + 1) % 3])) }
}

/// The page's controls in order (the views draw the ring on the same order).
pub fn targets(a: &App) -> Vec<T> {
    let p = &a.prefs;
    match a.page {
        Page::Live => match a.sig {
            Sig::NoSignal => vec![tp(Msg::Go(Page::Settings))],
            _ => vec![],
        },
        Page::Library => (0..a.shown().len()).map(|i| tp(Msg::Select(i))).collect(),
        Page::Settings => vec![
            tp(Msg::Dd(Some(Dd::Card))),
            tp(Msg::Dd(Some(Dd::Size))),
            cycle_n("fps", &[30, 50, 60], p.fps),
            cycle_s("format", &["MJPG", "YUYV"], &p.format),
            flip("lowDelay", p.low_delay),
            tp(Msg::Dd(Some(Dd::Sound))),
            flip("playSound", p.play_sound),
            tp(Msg::OpenDir(a.rec_dir())),
            tp(Msg::ChangeDir("recordingsDir")),
            tp(Msg::OpenDir(a.shot_dir())),
            tp(Msg::ChangeDir("screenshotsDir")),
            flip("startFullScreen", p.full_screen),
            flip("keepAwake", p.keep_awake),
            tp(Msg::OpenStore),
        ],
    }
}

/// The quick menu's rows (5b): Low delay, Volume, Mute, Picture, Screenshot, Record, Recordings & screenshots,
/// Settings, Leave Capture Card.
pub fn menu_targets(a: &App) -> Vec<T> {
    let p = &a.prefs;
    let v = p.volume;
    vec![
        flip("lowDelay", p.low_delay),
        T { press: None, left: Some(Msg::Volume(v.saturating_sub(5))), right: Some(Msg::Volume((v + 5).min(100))) },
        T { press: Some(Msg::Mute), left: Some(Msg::Mute), right: Some(Msg::Mute) },
        fit_t(&p.fit),
        tp(Msg::Screenshot),
        tp(Msg::Record),
        tp(Msg::Go(Page::Library)),
        tp(Msg::Go(Page::Settings)),
        tp(Msg::Leave),
    ]
}

fn move_focus(a: &mut App, d: i32) {
    let n = if a.menu { menu_targets(a).len() } else { targets(a).len() };
    if n == 0 {
        return;
    }
    let cur = if a.menu { a.menu_focus } else if a.page == Page::Library { a.sel } else { a.focus };
    let next = (cur as i32 + d).clamp(0, n as i32 - 1) as usize;
    if a.menu {
        a.menu_focus = next;
    } else if a.page == Page::Library {
        a.sel = next;
        a.preview = None;
    } else {
        a.focus = next;
    }
    a.ring = true;
}

/// A / ◀ / ▶ on the focused control.
fn act(a: &mut App, how: u8) -> Task<Msg> {
    let list = if a.menu { menu_targets(a) } else { targets(a) };
    let i = if a.menu { a.menu_focus } else { a.focus };
    let Some(t) = list.into_iter().nth(i) else { return Task::none() };
    let m = match how {
        0 => t.press,
        1 => t.left,
        _ => t.right,
    };
    a.ring = true;
    match m {
        Some(m) => update(a, m),
        None => Task::none(),
    }
}

// ------------------------------------------------------------------ the picture
fn ask_for(a: &App, watching: bool) -> Option<video::Ask> {
    let c = a.card.clone()?;
    let (format, w, h, fps) = c.pick(&a.prefs.format, a.prefs.w, a.prefs.h, a.prefs.fps).unwrap_or(("MJPG".into(), 1920, 1080, 60));
    Some(video::Ask {
        card: c,
        format,
        w,
        h,
        fps,
        low_delay: a.prefs.low_delay,
        sound: a.sound.as_ref().map(|s| s.name.clone()),
        play_sound: a.prefs.play_sound,
        volume: a.prefs.volume,
        muted: a.prefs.muted,
        watching,
    })
}

/// Starts the picture (Live view), or waits for a signal.
fn begin(a: &mut App, watching: bool) -> Task<Msg> {
    match ask_for(a, watching) {
        Some(ask) => {
            if !watching {
                a.sig = Sig::Starting;
            }
            video::start(ask);
            Task::none()
        }
        None => {
            video::stop();
            a.sig = Sig::NoCard;
            awake(a, false);
            Task::none()
        }
    }
}

/// Stops the picture when leaving Live view (unless recording): nothing runs on the other pages.
fn pause(a: &mut App) {
    if a.rec.is_none() && video::running() {
        video::stop();
        a.frame = None;
        if a.sig == Sig::Live || a.sig == Sig::Starting {
            a.sig = Sig::Starting;
        }
    }
    awake(a, false);
}

/// Keep the screen awake while a picture is showing (systemd-inhibit, stopped when it isn't).
fn awake(a: &mut App, on: bool) {
    let want = on && a.prefs.keep_awake;
    if want && a.inhibit.is_none() {
        a.inhibit = std::process::Command::new("systemd-inhibit")
            .args(["--what=idle", "--who=Capture Card", "--why=Showing your capture card's picture", "--mode=block", "sleep", "infinity"])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .ok();
    } else if !want {
        if let Some(mut c) = a.inhibit.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
    }
}

fn set_full(a: &mut App, on: bool) -> Task<Msg> {
    a.full = on;
    let mode = if on { window::Mode::Fullscreen } else { window::Mode::Windowed };
    window::latest().and_then(move |id| window::set_mode(id, mode))
}

fn show_bar(a: &mut App) -> Task<Msg> {
    a.bar = true;
    a.bar_gen += 1;
    after(2000, Msg::BarHide(a.bar_gen))
}

fn show_hints(a: &mut App) -> Task<Msg> {
    a.pill = true;
    a.pill_gen += 1;
    after(if a.game { 3000 } else { 6000 }, Msg::PillHide(a.pill_gen))
}

fn go(a: &mut App, page: Page) -> Task<Msg> {
    let was = a.page;
    a.page = page;
    a.focus = 0;
    a.dd = None;
    a.menu = false;
    a.ring = a.game || page == Page::Library;
    let mut t = Vec::new();
    if was == Page::Live && page != Page::Live {
        pause(a);
        if a.full && !a.game {
            t.push(set_full(a, false));
        }
    }
    match page {
        Page::Live if was != Page::Live => {
            if a.card.is_some() && !video::running() {
                let waiting = a.sig == Sig::NoSignal;
                t.push(begin(a, waiting));
            }
        }
        Page::Settings => {
            a.store_chip = store_chip();
            a.prefs = prefs::load();
            t.push(work(scan, |(c, s)| Msg::Cards(c, s)));
        }
        Page::Library => {
            a.sel = 0;
            a.preview = None;
            t.push(load_items(a));
        }
        _ => {}
    }
    Task::batch(t)
}

fn back(a: &mut App) -> Task<Msg> {
    if a.dlg.is_some() {
        a.dlg = None;
        return Task::none();
    }
    if a.dd.is_some() {
        a.dd = None;
        return Task::none();
    }
    if a.menu {
        a.menu = false;
        return Task::none();
    }
    match a.page {
        Page::Library | Page::Settings => go(a, Page::Live),
        Page::Live if a.full && !a.game => set_full(a, false),
        _ => Task::none(),
    }
}

fn step_page(a: &mut App, d: i32) -> Task<Msg> {
    let pages = [Page::Live, Page::Library, Page::Settings];
    let i = pages.iter().position(|p| *p == a.page).unwrap_or(0) as i32;
    go(a, pages[((i + d).rem_euclid(3)) as usize])
}

fn step_tab(a: &mut App, d: i32) -> Task<Msg> {
    let tabs = [Tab::Rec, Tab::Shot, Tab::All];
    let i = tabs.iter().position(|t| *t == a.tab).unwrap_or(0) as i32;
    update(a, Msg::Tab(tabs[((i + d).rem_euclid(3)) as usize]))
}

/// What a fresh card list means: the card in use, its sound, and the picture.
fn on_cards(a: &mut App, cards: Vec<Card>, sources: Vec<Source>) -> Task<Msg> {
    let before = a.card.clone();
    a.cards = cards;
    a.sources = sources;
    a.card = a.cards.iter().find(|c| !a.prefs.card.is_empty() && c.id == a.prefs.card).or(a.cards.first()).cloned();
    a.pick_sound();
    if a.card.is_none() {
        video::stop();
        a.frame = None;
        a.mode = None;
        a.sig = Sig::NoCard;
        awake(a, false);
        return Task::none();
    }
    let same = before.as_ref().map(|b| b.id == a.card.as_ref().unwrap().id && b.dev == a.card.as_ref().unwrap().dev).unwrap_or(false);
    if a.page == Page::Live && (!same || !video::running()) {
        return begin(a, false);
    }
    if !same && a.sig == Sig::NoCard || a.sig == Sig::Looking {
        a.sig = Sig::Starting;
    }
    Task::none()
}

/// Every message; a new note (a short line about what happened) hides by itself after 6 s.
pub fn update(a: &mut App, m: Msg) -> Task<Msg> {
    let before = a.note.clone();
    let t = handle(a, m);
    if a.note.is_some() && a.note != before {
        a.note_gen += 1;
        return Task::batch([t, after(6000, Msg::NoteHide(a.note_gen))]);
    }
    t
}

fn handle(a: &mut App, m: Msg) -> Task<Msg> {
    match m {
        Msg::None => Task::none(),
        Msg::Drag => win::drag(),
        Msg::Max => win::toggle_maximize(),
        Msg::Min => win::minimize(),
        Msg::Close | Msg::Leave => {
            video::stop();
            awake(a, false);
            // A recording being closed gets its moment (stop() finishes it on its own thread).
            if a.rec.take().is_some() {
                return after(1500, Msg::Close);
            }
            win::close()
        }
        Msg::Resize(d) => win::resize(d),
        Msg::Size(w, h) => {
            a.size = (w, h);
            Task::none()
        }
        Msg::Changed => work(scan, |(c, s)| Msg::Cards(c, s)),
        Msg::Cards(c, s) => on_cards(a, c, s),
        Msg::Retry(n) => {
            if n == a.retry && a.page == Page::Live && a.card.is_some() && !video::running() {
                return begin(a, false);
            }
            Task::none()
        }
        Msg::Ev((gen, e)) => {
            // A recording's end is heard whichever pipeline it was in.
            if let VEv::Recorded(r) = &e {
                a.rec = None;
                if let Err(e) = r {
                    a.note = Some(e.clone());
                }
                return if a.page == Page::Library { load_items(a) } else { Task::none() };
            }
            if gen != video::current() {
                if e == VEv::Frame {
                    // An old pipeline's frame: taken and dropped, so it never holds up the new one's.
                    let _ = video::frame();
                }
                return Task::none();
            }
            on_video(a, e)
        }
        Msg::Go(p) => go(a, p),
        Msg::Tab(t) => {
            a.tab = t;
            a.sel = 0;
            a.preview = None;
            Task::none()
        }
        Msg::Sort => {
            let v = if a.prefs.sort == "oldest" { "newest" } else { "oldest" };
            update(a, Msg::Set("sort", json!(v)))
        }
        Msg::Select(i) => {
            a.sel = i;
            a.preview = None;
            a.ring = true;
            Task::none()
        }
        Msg::Set(key, v) => {
            if let Err(e) = prefs::set(&a.prefs, key, v) {
                a.note = Some(e);
                return Task::none();
            }
            let before = a.prefs.clone();
            a.prefs = prefs::load();
            if key == "sound" {
                a.pick_sound();
            }
            if key == "keepAwake" {
                awake(a, a.sig == Sig::Live && a.prefs.keep_awake);
            }
            // The picture's choices: the picture starts again with them (a moment).
            let restart = matches!(key, "card" | "format" | "size" | "fps" | "lowDelay" | "sound" | "playSound");
            if key == "card" {
                return on_cards(a, a.cards.clone(), a.sources.clone());
            }
            if restart && before != a.prefs && video::running() && a.rec.is_none() {
                return begin(a, a.sig == Sig::NoSignal);
            }
            Task::none()
        }
        Msg::Dd(d) => {
            a.dd = d;
            Task::none()
        }
        Msg::Pick(d, v) => {
            a.dd = None;
            match d {
                Dd::Card => update(a, Msg::Set("card", v)),
                Dd::Sound => update(a, Msg::Set("sound", v)),
                Dd::Size => update(a, Msg::Set("size", v)),
            }
        }
        Msg::Volume(v) => {
            a.prefs.volume = v;
            video::set_volume(v, a.prefs.muted);
            if a.menu {
                return update(a, Msg::VolumeDone);
            }
            Task::none()
        }
        Msg::VolumeDone => {
            let v = a.prefs.volume;
            let mut p = prefs::load();
            p.volume = v;
            if let Err(e) = prefs::set(&p, "volume", json!(v)) {
                a.note = Some(e);
            }
            a.prefs = prefs::load();
            Task::none()
        }
        Msg::Mute => {
            let m = !a.prefs.muted;
            video::set_volume(a.prefs.volume, m);
            update(a, Msg::Set("muted", json!(m)))
        }
        Msg::Fit(f) => update(a, Msg::Set("fit", json!(f))),
        Msg::BarHide(g) => {
            if g == a.bar_gen && !a.shows("bar") {
                a.bar = false;
                a.tip = false;
            }
            Task::none()
        }
        Msg::PillHide(g) => {
            if g == a.pill_gen && !a.shows("pill") {
                a.pill = false;
            }
            Task::none()
        }
        Msg::Mouse => {
            if a.page == Page::Live && !a.game && !a.bar {
                return show_bar(a);
            }
            if a.page == Page::Live && !a.game {
                a.bar_gen += 1;
                return after(2000, Msg::BarHide(a.bar_gen));
            }
            Task::none()
        }
        Msg::Screenshot => {
            a.menu = false;
            let px = match &a.frame {
                Some(image::Handle::Rgba { width, height, pixels, .. }) => Some((*width, *height, pixels.to_vec())),
                _ => None,
            };
            let dir = a.shot_dir();
            work(move || save_shot(dir, px), Msg::Shot)
        }
        Msg::Shot(r) => match r {
            Ok(p) => {
                a.shot = Some((p.clone(), image::Handle::from_path(&p)));
                a.shot_gen += 1;
                if a.shows("shot") {
                    return Task::none();
                }
                after(6000, Msg::ShotHide(a.shot_gen))
            }
            Err(e) => {
                a.note = Some(e);
                Task::none()
            }
        },
        Msg::ShotHide(g) => {
            if g == a.shot_gen {
                a.shot = None;
            }
            Task::none()
        }
        Msg::ShowFile(p) => {
            std::thread::spawn(move || show_file(&p));
            Task::none()
        }
        Msg::CopyFile(p) => work(move || copy_file(&p), Msg::Copied),
        Msg::Copied(ok) => {
            a.note = Some(if ok { "Copied.".into() } else { "Couldn't copy it (wl-copy isn't there).".into() });
            Task::none()
        }
        Msg::Play(p) => {
            play_file(&p);
            Task::none()
        }
        Msg::Record => {
            a.menu = false;
            if a.rec.is_some() {
                video::record_stop();
                return Task::none();
            }
            if a.sig != Sig::Live {
                a.note = Some("There's no picture to record yet.".into());
                return Task::none();
            }
            let file = a.rec_dir().join(format!("{}.mp4", cc::stamp_name(cc::now())));
            video::record_start(file);
            Task::none()
        }
        Msg::Tick => {
            let Some((_, f)) = a.rec.clone() else { return Task::none() };
            let dir = a.rec_dir();
            work(move || (std::fs::metadata(&f).map(|m| m.len()).unwrap_or(0), cc::free_words(&dir)), |(s, fr)| Msg::RecSize(s, fr))
        }
        Msg::RecSize(s, f) => {
            a.rec_size = s;
            a.rec_free = f;
            Task::none()
        }
        Msg::FullScreen => {
            let on = !a.full;
            set_full(a, on)
        }
        Msg::Tip(on) => {
            a.tip = on || a.shows("tip");
            Task::none()
        }
        Msg::Menu => {
            a.menu = !a.menu;
            a.menu_focus = 0;
            a.ring = true;
            Task::none()
        }
        Msg::OpenDir(d) => {
            let _ = std::fs::create_dir_all(&d);
            hearth_ui::run::detached(&["xdg-open", &d.to_string_lossy()]);
            Task::none()
        }
        Msg::ChangeDir(key) => {
            let start = if key == "recordingsDir" { a.rec_dir() } else { a.shot_dir() };
            work(move || pick_folder(&start), move |r| Msg::PickedDir(key, r))
        }
        Msg::PickedDir(key, r) => match r {
            Some(d) => update(a, Msg::Set(key, json!(d))),
            None => Task::none(),
        },
        Msg::OpenStore => {
            hearth_ui::run::detached(&["hearth-store-app", "--page", "app:capture-card"]);
            Task::none()
        }
        Msg::Items(v) => {
            let todo = v.iter().any(|i| i.info.as_ref().and_then(|x| x.thumb.as_ref()).is_none());
            let all = v.clone();
            set_items(a, v);
            if !todo {
                return Task::none();
            }
            let (r, s) = (a.rec_dir(), a.shot_dir());
            work(
                move || {
                    library::fill(&all);
                    library::scan(&r, &s)
                },
                Msg::Filled,
            )
        }
        Msg::Filled(v) => {
            set_items(a, v);
            Task::none()
        }
        Msg::Ask(d) => {
            if let Dlg::Rename(p) = &d {
                a.rename_text = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            }
            a.dlg = Some(d);
            Task::batch([iced::widget::operation::focus(desk::RENAME_FIELD), iced::widget::operation::select_all(desk::RENAME_FIELD)])
        }
        Msg::RenameText(s) => {
            a.rename_text = s;
            Task::none()
        }
        Msg::DoRename => {
            let Some(Dlg::Rename(p)) = a.dlg.clone() else { return Task::none() };
            let name = a.rename_text.clone();
            a.dlg = None;
            work(move || library::rename(&p, &name).map(|_| ()), Msg::Done)
        }
        Msg::DoDelete => {
            let Some(Dlg::Delete(p)) = a.dlg.clone() else { return Task::none() };
            a.dlg = None;
            work(move || library::trash(&p), Msg::Done)
        }
        Msg::Done(r) => {
            if let Err(e) = r {
                a.note = Some(e);
            }
            a.preview = None;
            load_items(a)
        }
        Msg::CloseDlg => {
            a.dlg = None;
            Task::none()
        }
        Msg::BHeld(g) => {
            if g == a.b_gen && a.b_down {
                return update(a, Msg::Leave);
            }
            Task::none()
        }
        Msg::NoteOff => {
            a.note = None;
            Task::none()
        }
        Msg::NoteHide(g) => {
            if g == a.note_gen {
                a.note = None;
            }
            Task::none()
        }
        Msg::Key(ev) => on_key(a, ev),
        Msg::Pad(b) => on_pad(a, b),
    }
}

fn set_items(a: &mut App, v: Vec<Item>) {
    a.items = v;
    for it in &a.items {
        if let Some(t) = it.info.as_ref().and_then(|x| x.thumb.as_ref()) {
            a.thumbs.entry(it.path.clone()).or_insert_with(|| image::Handle::from_path(t));
        }
    }
    let n = a.shown().len();
    if a.sel >= n {
        a.sel = n.saturating_sub(1);
    }
}

fn on_video(a: &mut App, e: VEv) -> Task<Msg> {
    match e {
        VEv::Started { format, w, h, fps, decoder, watching } => {
            if !watching {
                a.mode = Some(Mode { format, w, h, fps, decoder });
                a.retry = 0;
            }
            Task::none()
        }
        VEv::Signal(true) => {
            if a.sig == Sig::NoSignal {
                // The signal came back while waiting: the picture at your mode.
                a.sig = Sig::Starting;
                return begin(a, false);
            }
            let first = a.sig != Sig::Live;
            a.sig = Sig::Live;
            awake(a, true);
            let mut t = Vec::new();
            if first {
                t.push(show_hints(a));
                if !a.game && !a.shows("pill-only") {
                    t.push(show_bar(a));
                }
                if !a.game && a.prefs.full_screen && !a.went_full && a.page == Page::Live {
                    a.went_full = true;
                    t.push(set_full(a, true));
                }
                if a.shows("shot") && a.shot.is_none() {
                    t.push(after(1500, Msg::Screenshot));
                }
                if a.shows("record") && a.rec.is_none() {
                    t.push(after(800, Msg::Record));
                }
            }
            Task::batch(t)
        }
        VEv::Signal(false) => {
            if a.rec.is_some() {
                // Recording: it goes on (a black stretch) rather than cutting the file.
                return Task::none();
            }
            a.sig = Sig::NoSignal;
            a.frame = None;
            awake(a, false);
            let mut t = vec![begin(a, true)];
            if a.full && !a.game {
                t.push(set_full(a, false));
            }
            Task::batch(t)
        }
        VEv::Frame => {
            if let Some(f) = video::frame() {
                let total = f.delay_ns / 1_000_000 + f.at.elapsed().as_millis() as u64 + 16;
                a.delay_acc.0 += total;
                a.delay_acc.1 += 1;
                if a.delay_ms.is_none() || a.delay_acc.2.elapsed() >= Duration::from_secs(1) {
                    a.delay_ms = Some((a.delay_acc.0 / a.delay_acc.1.max(1) as u64) as u32);
                    a.delay_acc = (0, 0, Instant::now());
                    remember_live(a);
                }
                a.frame = Some(image::Handle::from_rgba(f.w, f.h, f.rgba));
            }
            Task::none()
        }
        VEv::Failed(why) => {
            a.frame = None;
            awake(a, false);
            a.note = Some(why);
            a.retry += 1;
            let n = a.retry;
            // Was it unplugged? Look again; if the card's still there, try again a few times, further apart.
            let mut t = vec![work(scan, |(c, s)| Msg::Cards(c, s))];
            if a.card.is_some() {
                a.sig = Sig::NoSignal;
                if n <= 3 {
                    t.push(after([3000, 10000, 30000][(n - 1) as usize], Msg::Retry(n)));
                }
            }
            Task::batch(t)
        }
        VEv::Recording(r) => {
            match r {
                Ok(f) => {
                    if a.shows("stop-record") {
                        // CI: a short recording, ended by the app itself (so its file is finished properly).
                        a.rec = Some((Instant::now(), f));
                        a.rec_size = 0;
                        a.rec_free = cc::free_words(&a.rec_dir());
                        return after(5000, Msg::Record);
                    }
                    a.rec = Some((Instant::now(), f));
                    a.rec_size = 0;
                    a.rec_free = cc::free_words(&a.rec_dir());
                    if !a.game && !a.shows("bar") {
                        a.bar = false;
                    }
                }
                Err(e) => {
                    cc::log(&format!("recording: {e}"));
                    a.note = Some(e);
                }
            }
            Task::none()
        }
        VEv::Recorded(_) => Task::none(),
    }
}

/// Keeps the last mode and delay in the cache (at most once a minute), for the sidebar on the other pages.
fn remember_live(a: &mut App) {
    let (Some(m), Some(d)) = (&a.mode, a.delay_ms) else { return };
    a.last_live = Some((m.short(), d));
    if a.last_saved.map(|t| t.elapsed() < Duration::from_secs(60)).unwrap_or(false) {
        return;
    }
    a.last_saved = Some(Instant::now());
    let _ = cc::write_atomic(&last_file(), json!({"mode": m.short(), "delay": d}).to_string().as_bytes());
}

fn on_key(a: &mut App, ev: KEvent) -> Task<Msg> {
    let KEvent::KeyPressed { key, modifiers, .. } = ev else { return Task::none() };
    let ctrl = modifiers.control();
    if a.dlg.is_some() {
        return match key {
            Key::Named(Named::Escape) => update(a, Msg::CloseDlg),
            Key::Named(Named::Enter) => match a.dlg {
                Some(Dlg::Rename(_)) => update(a, Msg::DoRename),
                Some(Dlg::Delete(_)) => update(a, Msg::DoDelete),
                None => Task::none(),
            },
            _ => Task::none(),
        };
    }
    if a.dd.is_some() {
        if let Key::Named(Named::Escape) = key {
            a.dd = None;
        }
        return Task::none();
    }
    // The viewer: any key shows the bar; its own keys.
    if a.page == Page::Live && a.sig == Sig::Live && !a.menu {
        let mut t = vec![if a.game { show_hints(a) } else { show_bar(a) }];
        let k = match &key {
            Key::Character(c) => c.to_lowercase(),
            _ => String::new(),
        };
        t.push(match key {
            Key::Named(Named::F11) => update(a, Msg::FullScreen),
            Key::Named(Named::Escape) if a.full && !a.game => set_full(a, false),
            Key::Named(Named::Escape) if a.game => update(a, Msg::Menu),
            Key::Named(Named::ArrowUp) => update(a, Msg::Volume((a.prefs.volume + 5).min(100))).chain(Task::done(Msg::VolumeDone)),
            Key::Named(Named::ArrowDown) => update(a, Msg::Volume(a.prefs.volume.saturating_sub(5))).chain(Task::done(Msg::VolumeDone)),
            _ if k == "m" => update(a, Msg::Mute),
            _ if k == "s" => update(a, Msg::Screenshot),
            _ if k == "r" => update(a, Msg::Record),
            _ if k == "f" => {
                let f = match a.prefs.fit.as_str() {
                    "fit" => "fill",
                    "fill" => "stretch",
                    _ => "fit",
                };
                update(a, Msg::Fit(f))
            }
            _ => Task::none(),
        });
        return Task::batch(t);
    }
    let lib_grid = a.page == Page::Library && !a.menu;
    let cols = if a.tab == Tab::Shot { 4 } else { 3 };
    match key {
        Key::Named(Named::Escape) => back(a),
        Key::Named(Named::F11) if a.page == Page::Live => update(a, Msg::FullScreen),
        Key::Named(Named::ArrowUp) => {
            move_focus(a, if lib_grid { -cols } else { -1 });
            Task::none()
        }
        Key::Named(Named::ArrowDown) => {
            move_focus(a, if lib_grid { cols } else { 1 });
            Task::none()
        }
        Key::Named(Named::ArrowLeft) if lib_grid => {
            move_focus(a, -1);
            Task::none()
        }
        Key::Named(Named::ArrowRight) if lib_grid => {
            move_focus(a, 1);
            Task::none()
        }
        Key::Named(Named::ArrowLeft) => act(a, 1),
        Key::Named(Named::ArrowRight) => act(a, 2),
        Key::Named(Named::Enter) if lib_grid => lib_primary(a),
        Key::Named(Named::Enter) => act(a, 0),
        Key::Named(Named::Delete) if lib_grid => lib_ask(a, false),
        Key::Named(Named::F2) if lib_grid => lib_ask(a, true),
        Key::Named(Named::PageUp) if ctrl => step_page(a, -1),
        Key::Named(Named::PageDown) if ctrl => step_page(a, 1),
        _ => Task::none(),
    }
}

/// A on a file: Play a recording, Copy a screenshot.
fn lib_primary(a: &mut App) -> Task<Msg> {
    match a.picked() {
        Some(i) if i.kind == Kind::Rec => update(a, Msg::Play(i.path.clone())),
        Some(i) => update(a, Msg::CopyFile(i.path.clone())),
        None => Task::none(),
    }
}

fn lib_ask(a: &mut App, rename: bool) -> Task<Msg> {
    match a.picked().map(|i| i.path.clone()) {
        Some(p) => update(a, Msg::Ask(if rename { Dlg::Rename(p) } else { Dlg::Delete(p) })),
        None => Task::none(),
    }
}

fn on_pad(a: &mut App, b: pad::Button) -> Task<Msg> {
    use pad::Button as P;
    if b == P::BUp {
        a.b_down = false;
        return Task::none();
    }
    if a.dlg.is_some() {
        return match b {
            P::B => update(a, Msg::CloseDlg),
            P::A => match a.dlg {
                Some(Dlg::Rename(_)) => update(a, Msg::DoRename),
                Some(Dlg::Delete(_)) => update(a, Msg::DoDelete),
                None => Task::none(),
            },
            _ => Task::none(),
        };
    }
    if let Some(d) = a.dd {
        return match b {
            P::B => {
                a.dd = None;
                Task::none()
            }
            P::A | P::Down | P::Right => {
                // The next choice in the open chooser (the chooser shows them all).
                let v = desk::dd_choices(a, d);
                let now = desk::dd_now(a, d);
                let i = v.iter().position(|(val, _)| *val == now).map(|i| (i + 1) % v.len().max(1)).unwrap_or(0);
                match v.get(i) {
                    Some((val, _)) if b == P::A => update(a, Msg::Pick(d, val.clone())),
                    Some((val, _)) => {
                        let val = val.clone();
                        update(a, Msg::Pick(d, val)).chain(Task::done(Msg::Dd(Some(d))))
                    }
                    None => Task::none(),
                }
            }
            _ => Task::none(),
        };
    }
    // The viewer.
    if a.page == Page::Live && !a.menu {
        let mut t = vec![if a.game { show_hints(a) } else { show_bar(a) }];
        t.push(match b {
            P::Menu => update(a, Msg::Menu),
            P::X if a.sig == Sig::Live => update(a, Msg::Screenshot),
            P::Y if a.sig == Sig::Live => update(a, Msg::Record),
            P::Left => update(a, Msg::Volume(a.prefs.volume.saturating_sub(5))).chain(Task::done(Msg::VolumeDone)),
            P::Right => update(a, Msg::Volume((a.prefs.volume + 5).min(100))).chain(Task::done(Msg::VolumeDone)),
            P::B if a.game => {
                // Hold B to leave (0.8 s).
                a.b_down = true;
                a.b_gen += 1;
                after(800, Msg::BHeld(a.b_gen))
            }
            P::B => back(a),
            P::A if a.sig == Sig::NoSignal => update(a, Msg::Go(Page::Settings)),
            P::Lb => step_page(a, -1),
            P::Rb => step_page(a, 1),
            _ => Task::none(),
        });
        return Task::batch(t);
    }
    if a.menu {
        return match b {
            P::Up => {
                move_focus(a, if a.menu_focus == 5 { -2 } else { -1 });
                Task::none()
            }
            P::Down => {
                move_focus(a, if a.menu_focus == 4 { 2 } else { 1 });
                Task::none()
            }
            P::Left if a.menu_focus == 5 => {
                a.menu_focus = 4;
                Task::none()
            }
            P::Right if a.menu_focus == 4 => {
                a.menu_focus = 5;
                Task::none()
            }
            P::Left => act(a, 1),
            P::Right => act(a, 2),
            P::A => act(a, 0),
            P::B | P::Menu => update(a, Msg::Menu),
            _ => Task::none(),
        };
    }
    let lib = a.page == Page::Library;
    let cols = if a.tab == Tab::Shot { 4 } else { 3 };
    match b {
        P::Up => {
            move_focus(a, if lib { -cols } else { -1 });
            Task::none()
        }
        P::Down => {
            move_focus(a, if lib { cols } else { 1 });
            Task::none()
        }
        P::Left if lib => {
            move_focus(a, -1);
            Task::none()
        }
        P::Right if lib => {
            move_focus(a, 1);
            Task::none()
        }
        P::Left => act(a, 1),
        P::Right => act(a, 2),
        P::A if lib => lib_primary(a),
        P::A => act(a, 0),
        P::X if lib => lib_ask(a, false),
        P::Y if lib => lib_ask(a, true),
        P::B => back(a),
        P::Lb if lib => step_tab(a, -1),
        P::Rb if lib => step_tab(a, 1),
        P::Lb => step_page(a, -1),
        P::Rb => step_page(a, 1),
        _ => Task::none(),
    }
}

// ------------------------------------------------------------------ the window
fn view(a: &App) -> iced::Element<'_, Msg> {
    if a.page == Page::Live && matches!(a.sig, Sig::Live | Sig::Starting) && a.card.is_some() {
        if a.game {
            return game::viewer(a);
        }
        return desk::viewer(a);
    }
    desk::view(a)
}

fn mouse_moved(e: iced::Event, _s: iced::event::Status, _w: window::Id) -> Option<Msg> {
    match e {
        iced::Event::Mouse(iced::mouse::Event::CursorMoved { .. }) => Some(Msg::Mouse),
        _ => None,
    }
}

/// The window's size, when it opens or changes (nothing at rest).
fn sized(e: iced::Event, _s: iced::event::Status, _w: window::Id) -> Option<Msg> {
    match e {
        iced::Event::Window(window::Event::Opened { size, .. }) | iced::Event::Window(window::Event::Resized(size)) => Some(Msg::Size(size.width, size.height)),
        _ => None,
    }
}

fn subscription(a: &App) -> Subscription<Msg> {
    let mut s = vec![
        iced::keyboard::listen().map(Msg::Key),
        Subscription::run(pad::presses).map(Msg::Pad),
        Subscription::run(video::events).map(Msg::Ev),
        Subscription::run(cc::watch::changes).map(|_| Msg::Changed),
        iced::event::listen_with(sized),
    ];
    if a.rec.is_some() {
        // The REC pill's clock: only while recording.
        s.push(iced::time::every(Duration::from_secs(1)).map(|_| Msg::Tick));
    }
    if a.page == Page::Live && a.sig == Sig::Live && !a.game {
        s.push(iced::event::listen_with(mouse_moved));
    }
    Subscription::batch(s)
}

fn theme(_a: &App) -> iced::Theme {
    iced::Theme::custom("Capture Card", iced::theme::Palette { background: look::BG, text: look::TX, primary: look::ACC, success: look::OK, warning: look::WARN, danger: look::BAD })
}

fn main() -> iced::Result {
    if app::flag("--version") {
        println!("Capture Card {VERSION}, a Hearth Store app.");
        return Ok(());
    }
    if app::handle_cli(
        "hearth-capture-card",
        "Capture Card: your Xbox on this PC's screen through a USB HDMI capture card (a Hearth Store app).\n  --page live|recordings|screenshots|settings\n  --tab recordings|screenshots|everything\n  --game-mode             Game Mode's viewer and quick menu (also when started in gamescope)\n  --show bar,pill,tip,menu,shot,record   (pictures)\n  --probe                 list the capture cards found and their modes, then end",
    ) {
        return Ok(());
    }
    if app::flag("--probe") {
        let (cards, sources) = scan();
        for c in &cards {
            println!("{} · {} · {} · {} · {}", c.name, c.chip, c.usb_words(), c.dev.display(), c.id);
            for m in &c.modes {
                println!("  {} {}x{} {:?}", m.format, m.w, m.h, m.fps);
            }
            println!("  sound: {}", sound::for_card(c, &sources).map(|s| s.name.as_str()).unwrap_or("(none found)"));
        }
        if cards.is_empty() {
            println!("No capture card found.");
        }
        let (v, au) = video::encoders();
        println!("recording: video {} · sound {}", v.as_deref().unwrap_or("(none)"), au.as_deref().unwrap_or("(none)"));
        return Ok(());
    }
    let game = game_mode();
    let mut w = app::window("hearth-capture-card", (1600.0, 900.0), (1100.0, 680.0));
    if game {
        w.fullscreen = true;
    }
    iced::application(App::new, update, view)
        .title("Capture Card")
        .subscription(subscription)
        .theme(theme)
        .window(w)
        .default_font(app::INTER)
        .run()
}
