//! Hearth Console Remote Play (hearth-console-remote-play): play your own Xbox at home on the ROG Ally or a laptop, over
//! the internet, the way Greenlight does it. A Hearth Store app. Built exactly to the approved mockups
//! (docs/mockups/console-remote-play, checklist docs/mockups/checklists/console-remote-play.md) and the plan in
//! docs/CONSOLE-REMOTE-PLAY.md. On the desktop (laptop) it's a window (desk.rs); in Game Mode on the Ally it's the
//! Game Mode pages (ally.rs).
//!
//! Light at rest: nothing polls. The console list is asked for when its page opens or on Refresh; the keyring when
//! the app opens; the network only when you stream or something fails. Only a live stream has timers. Opening the
//! app or a page writes none of your settings.

mod ally;
mod desk;
mod look;
mod pad;

use crp::input;
use crp::spad::Nav;
use crp::stream::{self, Event as SEv, Stats, Step};
use crp::xhome::{self, Console, Game, Kind, Power};
use crp::{data, keyring, prefs, xal};
use hearth_ui::app::{self, win};
use iced::keyboard::{key::Named, Event as KEvent, Key};
use iced::widget::image;
use iced::{window, Subscription, Task};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// The signed-in account (in memory only; what's kept is in the keyring).
static ACCOUNT: Mutex<Option<xal::Account>> = Mutex::new(None);
/// The sign-in window while it's open (Cancel closes it).
static SIGNIN: Mutex<Option<u32>> = Mutex::new(None);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Welcome,
    SignIn,
    SignedIn,
    Consoles,
    Connecting,
    Stream,
    Trouble,
    Settings,
    Cloud,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Load<T> {
    Idle,
    Loading,
    Ready(T),
    Failed(String),
}

/// This device's network, in plain words (connection trouble, Data saver on a hotspot).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Net {
    /// "wifi", "ethernet" or "".
    pub kind: String,
    /// "2.4", "5", "6" (GHz).
    pub band: String,
    pub name: String,
    pub metered: bool,
}

impl Net {
    pub fn words(&self) -> String {
        match self.kind.as_str() {
            "wifi" if !self.band.is_empty() => format!("Wi-Fi {} GHz", self.band),
            "wifi" => "Wi-Fi".into(),
            "ethernet" => "Ethernet".into(),
            "" => "no network".into(),
            other => other.to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dd {
    Limit,
    Warn,
    Sound,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloudTab {
    Recent,
    All,
    Search,
}

/// A stream being set up (the Connecting page).
#[derive(Debug, Clone)]
pub struct Conn {
    pub name: String,
    pub model: String,
    pub kind: Kind,
    pub target: String,
    /// The console was asleep (step 1 shows), and how long it took to wake.
    pub waking: bool,
    pub woke: Option<u64>,
    pub step: u8,
    pub since: Instant,
    pub queue: Option<u64>,
    pub hd: bool,
}

pub struct App {
    pub ally: bool,
    pub page: Page,
    pub back: Page,
    pub prefs: prefs::Prefs,
    pub used: data::Used,
    pub gamertag: String,
    pub signed_in: bool,
    pub consoles: Load<Vec<Console>>,
    pub waking: Option<String>,
    pub cloud: bool,
    pub games: Load<Vec<Game>>,
    pub all_games: Load<Vec<Game>>,
    pub pics: HashMap<String, image::Handle>,
    /// The same pictures cut to the smaller tiles' shape (wide), so nothing is drawn outside them.
    pub wide: HashMap<String, image::Handle>,
    pub tab: CloudTab,
    pub search: String,
    pub focus: usize,
    pub ring: bool,
    pub conn: Option<Conn>,
    pub frame: Option<image::Handle>,
    pub still: Option<image::Handle>,
    pub stats: Stats,
    pub bar: bool,
    pub bar_gen: u64,
    pub pill: bool,
    pub pill_gen: u64,
    pub menu: bool,
    pub menu_focus: usize,
    pub tip: bool,
    pub warn_toast: bool,
    pub shot: Option<(PathBuf, image::Handle)>,
    pub shot_gen: u64,
    pub trouble: Option<(String, bool)>,
    pub net: Net,
    pub dd: Option<Dd>,
    pub sounds: Vec<(String, String)>,
    pub note: Option<String>,
    pub ask_limit: Option<usize>,
    pub full: bool,
    pub store_chip: (String, bool),
    pub show: Vec<String>,
    pub counted: u64,
    pub restart: bool,
    pub last: Option<usize>,
    pub last_game: Option<usize>,
    inhibit: Option<std::process::Child>,
    pub laptop: bool,
    /// The window's size (the tooltip over Disconnect is placed from it).
    pub size: (f32, f32),
    /// Flipped to draw the whole window again (see `update`).
    pub tint: bool,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Key(KEvent),
    Pad(pad::Button),
    Mouse,
    Ev((u64, SEv)),
    Drag,
    Max,
    Min,
    Close,
    Resize(window::Direction),
    Go(Page),
    Tab(i32),
    Focus(usize),
    SignIn,
    SignedIn(Result<String, String>),
    CancelSignIn,
    Booted(Option<String>, String),
    Refresh,
    Consoles(Result<(Vec<Console>, bool), String>),
    Wake(usize),
    Woke(Result<Power, String>),
    Stream(usize),
    StreamAnyway,
    NotNow,
    Begin(Result<stream::Ask, String>),
    Cancel,
    Tick,
    Set(&'static str, Value),
    Dd(Option<Dd>),
    Pick(Dd, Value),
    Sounds(Vec<(String, String)>),
    Volume(u32),
    VolumeDone,
    BarHide(u64),
    PillHide(u64),
    Screenshot,
    Shot(Result<PathBuf, String>),
    ShotHide(u64),
    ShowShot,
    CopyShot,
    Disconnect,
    FullScreen,
    Menu,
    Tip(bool),
    SaverNow,
    KeepGoing,
    TryAgain(bool),
    ShowList,
    SignOut,
    OpenStore,
    CloudTab(CloudTab),
    Search(String),
    Games(bool, Result<Vec<Game>, String>),
    Pic(String, Option<Vec<u8>>),
    Play(usize),
    Net(Net),
    Used(data::Used),
    Size(f32, f32),
    Repaint,
    None,
}

// ------------------------------------------------------------------ small helpers
fn fixture() -> Option<PathBuf> {
    crp::fixture()
}

fn work<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static, then: impl Fn(T) -> Msg + Send + 'static) -> Task<Msg> {
    Task::perform(crp::http::spawn(f), move |r| match r {
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

/// Runs a program with a time limit (seconds); its stdout, or "" when it failed.
fn output(cmd: &[&str], secs: u64) -> String {
    use std::process::{Command, Stdio};
    let Some((p, args)) = cmd.split_first() else { return String::new() };
    let Ok(mut child) = Command::new(p).args(args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn() else { return String::new() };
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                let mut s = String::new();
                if let Some(mut o) = child.stdout.take() {
                    use std::io::Read;
                    let _ = o.read_to_string(&mut s);
                }
                return s;
            }
            Ok(None) if start.elapsed() < Duration::from_secs(secs) => std::thread::sleep(Duration::from_millis(20)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return String::new();
            }
        }
    }
}

/// This device's network (NetworkManager), asked once when it matters. CI: HEARTH_CRP_NET="wifi|2.4|OfficeNet|metered".
fn read_net() -> Net {
    if let Ok(v) = std::env::var("HEARTH_CRP_NET") {
        let p: Vec<&str> = v.split('|').collect();
        return Net { kind: p.first().unwrap_or(&"").to_string(), band: p.get(1).unwrap_or(&"").to_string(), name: p.get(2).unwrap_or(&"").to_string(), metered: p.get(3) == Some(&"metered") };
    }
    let mut n = Net::default();
    let devs = output(&["nmcli", "-t", "-f", "DEVICE,TYPE,STATE,CONNECTION", "device"], 5);
    let Some(line) = devs.lines().find(|l| l.contains(":connected:")) else { return n };
    let f: Vec<&str> = line.split(':').collect();
    let dev = f.first().copied().unwrap_or("");
    n.kind = f.get(1).copied().unwrap_or("").to_string();
    n.name = f.get(3).copied().unwrap_or("").to_string();
    if n.kind == "wifi" {
        let list = output(&["nmcli", "-t", "-f", "ACTIVE,SSID,FREQ", "device", "wifi", "list", "--rescan", "no"], 5);
        if let Some(l) = list.lines().find(|l| l.starts_with("yes:")) {
            let mhz: u32 = l.rsplit(':').next().unwrap_or("").split_whitespace().next().unwrap_or("0").parse().unwrap_or(0);
            n.band = if mhz >= 5925 { "6".into() } else if mhz >= 4900 { "5".into() } else if mhz > 0 { "2.4".into() } else { String::new() };
        }
    }
    n.metered = output(&["nmcli", "-t", "-g", "GENERAL.METERED", "device", "show", dev], 5).trim().starts_with("yes");
    n
}

/// The sound outputs (PipeWire, through pactl): (node name, description).
fn read_sounds() -> Vec<(String, String)> {
    let v: Value = serde_json::from_str(&output(&["pactl", "--format=json", "list", "sinks"], 5)).unwrap_or(Value::Null);
    v.as_array().cloned().unwrap_or_default().iter().filter_map(|s| Some((s["name"].as_str()?.to_string(), s["description"].as_str().unwrap_or("").to_string()))).collect()
}

/// What the Store knows about this app: Up to date, or an update is ready (its own files; read only).
fn store_chip() -> (String, bool) {
    let data = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|| crp::home().join(".local/share"));
    let cache = std::env::var_os("XDG_CACHE_HOME").map(PathBuf::from).unwrap_or_else(|| crp::home().join(".cache"));
    let read = |p: PathBuf| -> Value { std::fs::read(p).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null) };
    let have = read(data.join("hearth/store/apps/console-remote-play/store.json"))["version"].as_str().unwrap_or("").to_string();
    let seen = read(cache.join("hearth/hearth-store/releases.json"));
    let newest = seen["apps"]["console-remote-play"]["version"].as_str().unwrap_or("").to_string();
    if have.is_empty() {
        return ("Not installed from the Store".into(), false);
    }
    if !newest.is_empty() && newest != have {
        return ("Update ready".into(), false);
    }
    ("Up to date".into(), true)
}

fn signin_helper() -> PathBuf {
    if let Ok(p) = std::env::var("HEARTH_CRP_SIGNIN") {
        return PathBuf::from(p);
    }
    if let Some(d) = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.to_path_buf())) {
        let p = d.join("hearth-console-remote-play-signin");
        if p.exists() {
            return p;
        }
    }
    PathBuf::from("hearth-console-remote-play-signin")
}

/// The whole sign-in (a worker): Microsoft's page in the sign-in window, then the tokens, then the keyring.
fn sign_in(ally: bool) -> Result<String, String> {
    use std::io::BufRead;
    use std::process::{Command, Stdio};
    let b = xal::begin().map_err(|e| e.words())?;
    let url = match fixture() {
        Some(f) => format!("file://{}?state={}{}", f.join("sign-in-page.html").display(), crp::http::enc(&b.state), if std::env::var_os("HEARTH_CRP_AUTO_REDIRECT").is_some() { "&auto=1" } else { "" }),
        None => b.page.clone(),
    };
    let mut c = Command::new(signin_helper());
    c.arg("--url").arg(&url).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null());
    if ally {
        c.arg("--game-mode");
    }
    let mut child = c.spawn().map_err(|_| "The sign-in window couldn't open (WebKitGTK is missing).".to_string())?;
    *SIGNIN.lock().unwrap() = Some(child.id());
    let mut line = String::new();
    if let Some(o) = child.stdout.take() {
        let _ = std::io::BufReader::new(o).read_line(&mut line);
    }
    let _ = child.wait();
    *SIGNIN.lock().unwrap() = None;
    let Some(redirect) = line.trim().strip_prefix("REDIRECT ").map(String::from) else { return Err(String::new()) };
    let code = xal::code_from(&redirect, &b.state)?;
    let acc = xal::Account::finish(b, &code).map_err(|e| e.words())?;
    let gt = acc.gamertag.clone();
    let kept = keyring::save(&acc.kept());
    *ACCOUNT.lock().unwrap() = Some(acc);
    match kept {
        Ok(()) => Ok(gt),
        Err(e) => Err(format!("KEPT-NOT:{gt}:{e}")),
    }
}

/// At start (a worker): the kept sign-in from the keyring. CI's stand-in signs in by itself for the pages after it.
fn boot(stand_in: bool) -> (Option<String>, String) {
    if stand_in {
        if let Ok(b) = xal::begin() {
            if let Ok(acc) = xal::Account::finish(b, "STANDIN") {
                let gt = acc.gamertag.clone();
                *ACCOUNT.lock().unwrap() = Some(acc);
                return (Some(gt), String::new());
            }
        }
    }
    match keyring::load().and_then(|s| xal::Account::from_kept(&s)) {
        Some(acc) => {
            let gt = acc.gamertag.clone();
            *ACCOUNT.lock().unwrap() = Some(acc);
            (Some(gt), String::new())
        }
        None => (None, String::new()),
    }
}

/// Keeps the sign-in again when Microsoft gave a new refresh token.
fn keep_if_changed(acc: &xal::Account, before: &str) {
    let now = acc.kept();
    if now != before {
        let _ = keyring::save(&now);
    }
}

fn list_consoles() -> Result<(Vec<Console>, bool), String> {
    let mut g = ACCOUNT.lock().unwrap();
    let acc = g.as_mut().ok_or("Not signed in.")?;
    let before = acc.kept();
    let home = acc.home().map_err(|e| e.words())?;
    let web = acc.web().ok();
    let list = xhome::consoles(&home, web.as_ref()).map_err(|e| e.words())?;
    let cloud = acc.cloud().is_some();
    keep_if_changed(acc, &before);
    Ok((list, cloud))
}

// ------------------------------------------------------------------ start
impl App {
    fn new() -> (App, Task<Msg>) {
        let prefs = prefs::load();
        let used = data::load(prefs.reset_day);
        let ally = game_mode();
        let page = match app::arg("--page").as_deref() {
            Some("signin") => Page::SignIn,
            Some("signed-in") => Page::SignedIn,
            Some("consoles") => Page::Consoles,
            Some("connecting") => Page::Connecting,
            Some("stream") => Page::Stream,
            Some("trouble") => Page::Trouble,
            Some("settings") => Page::Settings,
            Some("cloud") => Page::Cloud,
            _ => Page::Welcome,
        };
        let show: Vec<String> = app::arg("--show").map(|s| s.split(',').map(String::from).collect()).unwrap_or_default();
        let still = fixture().map(|f| image::Handle::from_path(f.join("stream-frame.jpg")));
        let a = App {
            ally,
            page: if page == Page::SignIn { Page::Welcome } else { page },
            back: Page::Consoles,
            prefs,
            used,
            gamertag: String::new(),
            signed_in: false,
            consoles: Load::Idle,
            waking: None,
            cloud: false,
            games: Load::Idle,
            all_games: Load::Idle,
            pics: HashMap::new(),
            wide: HashMap::new(),
            tab: CloudTab::Recent,
            search: String::new(),
            focus: 0,
            ring: true,
            conn: None,
            frame: None,
            still,
            stats: Stats::default(),
            bar: true,
            bar_gen: 0,
            pill: true,
            pill_gen: 0,
            menu: false,
            menu_focus: 0,
            tip: false,
            warn_toast: false,
            shot: None,
            shot_gen: 0,
            trouble: None,
            net: Net::default(),
            dd: None,
            sounds: Vec::new(),
            note: None,
            ask_limit: None,
            full: false,
            store_chip: ("Up to date".into(), true),
            show,
            counted: 0,
            restart: false,
            last: None,
            last_game: None,
            inhibit: None,
            laptop: has_battery(),
            size: (1920.0, 1080.0),
            tint: false,
        };
        let stand_in = fixture().is_some() && !matches!(page, Page::Welcome | Page::SignIn);
        let first = page;
        let boot_task = work(move || boot(stand_in), |(g, n)| Msg::Booted(g, n));
        let mut tasks = vec![boot_task];
        if first == Page::SignIn {
            tasks.push(Task::done(Msg::SignIn));
        }
        (a, Task::batch(tasks))
    }

    pub fn shows(&self, what: &str) -> bool {
        self.show.iter().any(|s| s == what)
    }

    /// The total this period: what's counted plus this stream's bytes not yet written.
    pub fn month_bytes(&self) -> u64 {
        self.used.bytes + self.stats.session_bytes.saturating_sub(self.counted)
    }

    pub fn console(&self) -> Option<&Console> {
        match &self.consoles {
            Load::Ready(v) => v.get(self.last.unwrap_or(0)),
            _ => None,
        }
    }

    pub fn console_status(&self) -> Option<(String, Power)> {
        self.console().map(|c| (c.name.clone(), c.power))
    }

    fn tabs(&self) -> Vec<Page> {
        if self.cloud {
            vec![Page::Consoles, Page::Cloud, Page::Settings]
        } else {
            vec![Page::Consoles, Page::Settings]
        }
    }

    pub fn shown_games(&self) -> Vec<Game> {
        match self.tab {
            // Recently played first (large), then the rest of the cloud library (smaller), as the mockup.
            CloudTab::Recent => {
                let mut v = match &self.games {
                    Load::Ready(g) => g.clone(),
                    _ => Vec::new(),
                };
                if let Load::Ready(all) = &self.all_games {
                    for g in all {
                        if !v.iter().any(|x| x.title == g.title) {
                            v.push(g.clone());
                        }
                    }
                }
                v
            }
            CloudTab::All | CloudTab::Search => {
                let all = match &self.all_games {
                    Load::Ready(g) => g.clone(),
                    _ => Vec::new(),
                };
                if self.tab == CloudTab::Search && !self.search.trim().is_empty() {
                    let q = self.search.to_lowercase();
                    all.into_iter().filter(|g| g.name.to_lowercase().contains(&q)).collect()
                } else {
                    all
                }
            }
        }
    }
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

fn cycle(key: &'static str, opts: &[&str], now: &str) -> T {
    let i = opts.iter().position(|o| *o == now).unwrap_or(0);
    let l = opts[(i + opts.len() - 1) % opts.len()];
    let r = opts[(i + 1) % opts.len()];
    T { press: Some(Msg::Set(key, json!(r))), left: Some(Msg::Set(key, json!(l))), right: Some(Msg::Set(key, json!(r))) }
}

fn flip(key: &'static str, on: bool) -> T {
    T { press: Some(Msg::Set(key, json!(!on))), left: Some(Msg::Set(key, json!(!on))), right: Some(Msg::Set(key, json!(!on))) }
}

/// The page's controls in order (the views draw the ring on the same order).
pub fn targets(a: &App) -> Vec<T> {
    let p = &a.prefs;
    match a.page {
        Page::Welcome => vec![tp(Msg::SignIn)],
        Page::SignedIn => vec![tp(Msg::Go(Page::Consoles))],
        Page::Consoles => match &a.consoles {
            Load::Ready(v) => (0..v.len()).map(|i| T { press: Some(Msg::Stream(i)), left: None, right: None }).collect(),
            _ => vec![tp(Msg::Refresh)],
        },
        Page::Connecting => vec![tp(Msg::Cancel)],
        Page::Trouble => vec![tp(Msg::TryAgain(true)), tp(Msg::TryAgain(false)), tp(if a.ally { Msg::Go(Page::Consoles) } else { Msg::ShowList })],
        Page::Settings => {
            let mut v = vec![
                cycle("quality", &["auto", "720p", "1080p"], &p.quality),
                cycle("dataSaver", &["off", "hotspot", "always"], &p.saver),
                flip("limitOn", p.limit_on),
                T { press: Some(Msg::Dd(Some(Dd::Limit))), left: None, right: None },
            ];
            if !a.ally {
                v.push(T { press: Some(Msg::Dd(Some(Dd::Warn))), left: None, right: None });
            }
            v.push(cycle("playWith", &["builtin", "paired"], &p.play_with));
            v.push(flip("vibration", p.vibration));
            v.push(flip("startFullScreen", p.full_screen));
            v.push(flip("keepAwake", p.keep_awake));
            v.push(T { press: Some(Msg::Dd(Some(Dd::Sound))), left: None, right: None });
            v.push(tp(Msg::SignOut));
            v.push(tp(Msg::OpenStore));
            v
        }
        Page::Cloud => {
            let mut v = vec![T {
                press: None,
                left: Some(Msg::CloudTab(match a.tab {
                    CloudTab::Recent => CloudTab::Search,
                    CloudTab::All => CloudTab::Recent,
                    CloudTab::Search => CloudTab::All,
                })),
                right: Some(Msg::CloudTab(match a.tab {
                    CloudTab::Recent => CloudTab::All,
                    CloudTab::All => CloudTab::Search,
                    CloudTab::Search => CloudTab::Recent,
                })),
            }];
            for i in 0..a.shown_games().len() {
                v.push(tp(Msg::Play(i)));
            }
            v
        }
        Page::SignIn | Page::Stream => vec![],
    }
}

/// The quick menu's rows (g7b): Quality, Data saver, Volume, Show stats, Screenshot, Settings, Vibration, Disconnect.
pub fn menu_targets(a: &App) -> Vec<T> {
    let p = &a.prefs;
    let v = p.volume;
    vec![
        cycle("quality", &["auto", "720p", "1080p"], &p.quality),
        T { press: Some(Msg::Set("dataSaver", json!(if p.saver == "off" { "hotspot" } else { "off" }))), left: Some(Msg::Set("dataSaver", json!("off"))), right: Some(Msg::Set("dataSaver", json!("hotspot"))) },
        T { press: None, left: Some(Msg::Volume(v.saturating_sub(10))), right: Some(Msg::Volume((v + 10).min(100))) },
        flip("showStats", p.show_stats),
        tp(Msg::Screenshot),
        tp(Msg::Go(Page::Settings)),
        flip("vibration", p.vibration),
        tp(Msg::Disconnect),
    ]
}

// ------------------------------------------------------------------ update
fn go(a: &mut App, page: Page) -> Task<Msg> {
    if a.page == Page::Stream && page == Page::Settings {
        a.back = Page::Stream;
    } else if page == Page::Settings || page == Page::Cloud {
        a.back = if a.page == Page::Settings || a.page == Page::Cloud { a.back } else { a.page };
    }
    a.page = page;
    a.focus = 0;
    a.dd = None;
    a.ring = !(matches!(page, Page::Settings | Page::Trouble) && !a.ally);
    if page == Page::Stream {
        a.menu = false;
        stream::set_menu_open(false);
    }
    match page {
        Page::Consoles if a.consoles == Load::Idle => update(a, Msg::Refresh),
        Page::Settings => {
            a.store_chip = store_chip();
            a.prefs = prefs::load();
            work(read_sounds, Msg::Sounds)
        }
        Page::Cloud if a.games == Load::Idle => {
            a.games = Load::Loading;
            a.all_games = Load::Loading;
            Task::batch([work(|| load_games(true), |r| Msg::Games(true, r)), work(|| load_games(false), |r| Msg::Games(false, r))])
        }
        _ => Task::none(),
    }
}

fn load_games(recent: bool) -> Result<Vec<Game>, String> {
    let tok = {
        let mut g = ACCOUNT.lock().unwrap();
        g.as_mut().and_then(|a| a.cloud())
    };
    let tok = tok.ok_or("Cloud needs Game Pass Ultimate on this account.")?;
    xhome::games(&tok, recent).map_err(|e| e.words())
}

fn fetch_pic(url: String) -> Option<Vec<u8>> {
    if let Some(f) = fixture() {
        if !url.starts_with("https://") {
            return std::fs::read(f.join(&url)).ok();
        }
    }
    let r = crp::http::Req::new("GET", &url);
    crp::http::send(&r).ok().map(|x| x.body.into_bytes())
}

fn pics_for(a: &mut App, list: &[Game]) -> Task<Msg> {
    let mut t = Vec::new();
    for g in list {
        if g.picture.is_empty() || a.pics.contains_key(&g.picture) {
            continue;
        }
        let u = g.picture.clone();
        let u2 = u.clone();
        t.push(work(move || fetch_pic(u), move |b| Msg::Pic(u2.clone(), b)));
    }
    Task::batch(t)
}

fn move_focus(a: &mut App, d: i32) {
    let n = if a.menu { menu_targets(a).len() } else { targets(a).len() };
    if n == 0 {
        return;
    }
    let cur = if a.menu { a.menu_focus } else { a.focus };
    let next = (cur as i32 + d).clamp(0, n as i32 - 1) as usize;
    if a.menu {
        a.menu_focus = next;
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

fn step_tab(a: &mut App, d: i32) -> Task<Msg> {
    let tabs = a.tabs();
    let cur = match a.page {
        Page::Cloud => Page::Cloud,
        Page::Settings => Page::Settings,
        _ => Page::Consoles,
    };
    let i = tabs.iter().position(|p| *p == cur).unwrap_or(0) as i32;
    let n = tabs.len() as i32;
    let next = tabs[((i + d).rem_euclid(n)) as usize];
    go(a, next)
}

fn back(a: &mut App) -> Task<Msg> {
    if a.dd.is_some() {
        a.dd = None;
        return Task::none();
    }
    if a.menu {
        return update(a, Msg::Menu);
    }
    match a.page {
        Page::Connecting => update(a, Msg::Cancel),
        Page::SignIn => update(a, Msg::CancelSignIn),
        Page::Settings | Page::Cloud => {
            let b = a.back;
            go(a, if b == Page::Stream && stream::running() { Page::Stream } else if b == Page::Stream { Page::Consoles } else { b })
        }
        Page::Trouble => go(a, Page::Consoles),
        Page::Welcome => win::close(),
        _ => Task::none(),
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

fn stream_ended(a: &mut App) -> Task<Msg> {
    if let Some(mut c) = a.inhibit.take() {
        let _ = c.kill();
        let _ = c.wait();
    }
    a.frame = None;
    a.menu = false;
    let mut t = vec![];
    if a.full && !a.ally {
        t.push(set_full(a, false));
    }
    if a.restart {
        a.restart = false;
        if let Some(i) = a.last_game {
            t.push(update(a, Msg::Play(i)));
        } else if let Some(i) = a.last {
            t.push(update(a, Msg::Stream(i)));
        }
    } else if a.trouble.is_none() && matches!(a.page, Page::Stream | Page::Connecting) {
        t.push(go(a, Page::Consoles));
    }
    Task::batch(t)
}

/// The picture size and bitrate cap for the next stream, from Quality and Data saver.
fn picture_for(a: &App, metered: bool) -> (bool, u32) {
    let saver = a.prefs.saver == "always" || (a.prefs.saver == "hotspot" && metered);
    let hd = match a.prefs.quality.as_str() {
        "720p" => false,
        "1080p" => true,
        _ => !saver,
    };
    (hd && !saver, if saver { 5000 } else { 0 })
}

/// Every change goes through here. When the page changes, the whole window is drawn again a moment later (`tint`): with
/// the CPU (tiny-skia) only the parts iced thinks changed are redrawn, and a line that had been measured on the page
/// before could stay cut short (the Ally's trouble page after Connecting). The two tints look the same.
pub fn update(a: &mut App, m: Msg) -> Task<Msg> {
    let before = a.page;
    let t = step(a, m);
    if a.page != before {
        return Task::batch([t, after(80, Msg::Repaint)]);
    }
    t
}

fn step(a: &mut App, m: Msg) -> Task<Msg> {
    match m {
        Msg::Repaint => {
            a.tint = !a.tint;
            Task::none()
        }
        Msg::None => Task::none(),
        Msg::Drag => win::drag(),
        Msg::Max => win::toggle_maximize(),
        Msg::Min => win::minimize(),
        Msg::Close => {
            stream::stop();
            win::close()
        }
        Msg::Resize(d) => win::resize(d),
        Msg::Go(p) => go(a, p),
        Msg::Tab(d) => step_tab(a, d),
        Msg::Focus(i) => {
            a.focus = i;
            Task::none()
        }
        Msg::Booted(gt, _) => {
            match gt {
                Some(g) => {
                    a.gamertag = g;
                    a.signed_in = true;
                    let p = a.page;
                    // The console list once, at the start (the sidebar's status, whether Cloud shows); asked for again
                    // only by My consoles' Refresh.
                    let mut t = Vec::new();
                    if a.consoles == Load::Idle {
                        a.consoles = Load::Loading;
                        t.push(work(list_consoles, Msg::Consoles));
                    }
                    if p == Page::Welcome && app::arg("--page").is_none() {
                        t.push(go(a, Page::Consoles));
                    } else {
                        t.push(go(a, p));
                    }
                    Task::batch(t)
                }
                None => {
                    a.signed_in = false;
                    if a.page != Page::Welcome && fixture().is_none() {
                        a.page = Page::Welcome;
                    }
                    Task::none()
                }
            }
        }
        Msg::SignIn => {
            a.page = Page::SignIn;
            a.note = None;
            let ally = a.ally;
            work(move || sign_in(ally), Msg::SignedIn)
        }
        Msg::SignedIn(r) => match r {
            Ok(g) => {
                a.gamertag = g;
                a.signed_in = true;
                go(a, Page::SignedIn)
            }
            Err(e) if e.starts_with("KEPT-NOT:") => {
                let mut parts = e.splitn(3, ':').skip(1);
                a.gamertag = parts.next().unwrap_or("").to_string();
                a.signed_in = true;
                a.note = Some(format!("Signed in for now, but {} You'll sign in again next time.", parts.next().unwrap_or("").to_lowercase()));
                go(a, Page::SignedIn)
            }
            Err(e) => {
                a.page = Page::Welcome;
                a.note = if e.is_empty() { None } else { Some(e) };
                Task::none()
            }
        },
        Msg::CancelSignIn => {
            if let Some(pid) = SIGNIN.lock().unwrap().take() {
                // SAFETY: ending the sign-in window this app started.
                unsafe { libc::kill(pid as i32, libc::SIGTERM) };
            }
            a.page = Page::Welcome;
            Task::none()
        }
        Msg::Refresh => {
            a.consoles = Load::Loading;
            work(list_consoles, Msg::Consoles)
        }
        Msg::Consoles(r) => {
            match r {
                Ok((v, cloud)) => {
                    a.consoles = Load::Ready(v);
                    a.cloud = cloud;
                }
                Err(e) => a.consoles = Load::Failed(e),
            }
            // CI's stand-in stream for the pages past My consoles.
            if fixture().is_some() && matches!(a.page, Page::Connecting | Page::Stream | Page::Trouble) && !stream::running() && a.conn.is_none() {
                return start_stand_in(a);
            }
            Task::none()
        }
        Msg::Wake(i) => {
            let Some(c) = (match &a.consoles {
                Load::Ready(v) => v.get(i).cloned(),
                _ => None,
            }) else {
                return Task::none();
            };
            a.waking = Some(c.id.clone());
            work(
                move || {
                    let w = {
                        let mut g = ACCOUNT.lock().unwrap();
                        g.as_mut().ok_or("Not signed in.".to_string())?.web().map_err(|e| e.words())?
                    };
                    xhome::wake(&w, &c.id).map_err(|e| e.words())?;
                    for _ in 0..3 {
                        std::thread::sleep(Duration::from_secs(5));
                        if let Ok(Power::On) = xhome::power(&w, &c.id) {
                            return Ok(Power::On);
                        }
                    }
                    xhome::power(&w, &c.id).map_err(|e| e.words())
                },
                Msg::Woke,
            )
        }
        Msg::Woke(r) => {
            let id = a.waking.take();
            if let (Ok(pw), Load::Ready(v), Some(id)) = (&r, &mut a.consoles, id) {
                if let Some(c) = v.iter_mut().find(|c| c.id == id) {
                    c.power = *pw;
                    c.seen = crp::now();
                }
            }
            if let Err(e) = r {
                a.note = Some(e);
            }
            Task::none()
        }
        Msg::Stream(i) => {
            a.last = Some(i);
            a.last_game = None;
            if a.prefs.limit_on && data::level(a.month_bytes(), a.prefs.limit_gb, a.prefs.warn_pct) == data::Level::Limit && a.ask_limit.is_none() {
                a.ask_limit = Some(i);
                return Task::none();
            }
            begin_home(a, i)
        }
        Msg::StreamAnyway => {
            let i = a.ask_limit.take();
            match i {
                Some(usize::MAX) => match a.last_game {
                    Some(g) => begin_cloud(a, g),
                    None => Task::none(),
                },
                Some(i) => begin_home(a, i),
                None => Task::none(),
            }
        }
        Msg::NotNow => {
            a.ask_limit = None;
            Task::none()
        }
        Msg::Begin(r) => match r {
            Ok(ask) => {
                if let Some(c) = a.conn.as_mut() {
                    c.waking = ask.wake.is_some();
                    c.hd = ask.hd;
                }
                stream::start(ask);
                Task::none()
            }
            Err(e) => {
                a.trouble = Some((e, false));
                a.conn = None;
                a.page = Page::Trouble;
                work(read_net, Msg::Net)
            }
        },
        Msg::Cancel => {
            stream::stop();
            a.conn = None;
            go(a, if a.last_game.is_some() { Page::Cloud } else { Page::Consoles })
        }
        Msg::Tick => Task::none(),
        Msg::Ev((gen, e)) => {
            if gen != stream::current() {
                return Task::none();
            }
            on_stream(a, e)
        }
        Msg::Set(key, v) => {
            if let Err(e) = prefs::set(&a.prefs, key, v.clone()) {
                a.note = Some(e);
                return Task::none();
            }
            let before = a.prefs.clone();
            a.prefs = prefs::load();
            if key == "vibration" {
                stream::set_vibration(a.prefs.vibration);
            }
            // Quality and Data saver change the picture: while streaming it connects again with them (about 10 s).
            if (key == "quality" || key == "dataSaver") && stream::running() && a.page == Page::Stream && before != a.prefs {
                a.restart = true;
                stream::stop();
            }
            Task::none()
        }
        Msg::Dd(d) => {
            a.dd = d;
            Task::none()
        }
        Msg::Pick(d, v) => {
            a.dd = None;
            let key = match d {
                Dd::Limit => "limitGb",
                Dd::Warn => "warnPct",
                Dd::Sound => "soundOutput",
            };
            update(a, Msg::Set(key, v))
        }
        Msg::Sounds(s) => {
            a.sounds = s;
            Task::none()
        }
        Msg::Volume(v) => {
            a.prefs.volume = v;
            stream::set_volume(v);
            if a.menu {
                return update(a, Msg::VolumeDone);
            }
            Task::none()
        }
        Msg::VolumeDone => {
            let v = a.prefs.volume;
            let mut p = prefs::load();
            p.volume = a.prefs.volume;
            if let Err(e) = prefs::set(&p, "volume", json!(v)) {
                a.note = Some(e);
            }
            Task::none()
        }
        Msg::Mouse => {
            if a.page == Page::Stream && !a.ally {
                return show_bar(a);
            }
            Task::none()
        }
        Msg::BarHide(g) => {
            if g == a.bar_gen && !a.tip && !a.shows("bar") && !a.menu {
                a.bar = false;
            }
            Task::none()
        }
        Msg::PillHide(g) => {
            if g == a.pill_gen && !a.shows("pill") {
                a.pill = false;
            }
            Task::none()
        }
        Msg::Screenshot => {
            a.menu = false;
            stream::set_menu_open(false);
            let px = match &a.frame {
                Some(image::Handle::Rgba { width, height, pixels, .. }) => Some((*width, *height, pixels.to_vec())),
                _ => None,
            };
            let still = fixture().map(|f| f.join("stream-frame.jpg"));
            work(move || save_shot(px, still), Msg::Shot)
        }
        Msg::Shot(r) => match r {
            Ok(p) => {
                a.shot = Some((p.clone(), image::Handle::from_path(&p)));
                a.shot_gen += 1;
                after(6000, Msg::ShotHide(a.shot_gen))
            }
            Err(e) => {
                a.note = Some(e);
                Task::none()
            }
        },
        Msg::ShotHide(g) => {
            if g == a.shot_gen && !a.shows("shot") {
                a.shot = None;
            }
            Task::none()
        }
        Msg::ShowShot => {
            if let Some((p, _)) = &a.shot {
                if let Some(d) = p.parent() {
                    hearth_ui::run::detached(&["xdg-open", &d.to_string_lossy()]);
                }
            }
            Task::none()
        }
        Msg::CopyShot => {
            if let Some((p, _)) = &a.shot {
                if let Ok(f) = std::fs::File::open(p) {
                    let _ = std::process::Command::new("wl-copy").args(["--type", "image/png"]).stdin(f).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn();
                }
            }
            Task::none()
        }
        Msg::Disconnect => {
            a.menu = false;
            stream::stop();
            Task::none()
        }
        Msg::FullScreen => {
            let on = !a.full;
            set_full(a, on)
        }
        Msg::Menu => {
            if a.page != Page::Stream {
                return Task::none();
            }
            a.menu = !a.menu;
            a.menu_focus = 0;
            stream::set_menu_open(a.menu);
            if a.menu {
                a.bar = true;
            }
            Task::none()
        }
        Msg::Tip(on) => {
            a.tip = on;
            if !on {
                return show_bar(a);
            }
            Task::none()
        }
        Msg::SaverNow => {
            a.warn_toast = false;
            update(a, Msg::Set("dataSaver", json!("always")))
        }
        Msg::KeepGoing => {
            a.warn_toast = false;
            Task::none()
        }
        Msg::TryAgain(hd) => {
            a.trouble = None;
            if !hd {
                let _ = prefs::set(&a.prefs, "quality", json!("720p"));
                a.prefs = prefs::load();
            }
            if fixture().is_some() {
                return Task::none();
            }
            match (a.last_game, a.last) {
                (Some(g), _) => update(a, Msg::Play(g)),
                (None, Some(i)) => update(a, Msg::Stream(i)),
                _ => go(a, Page::Consoles),
            }
        }
        Msg::ShowList => go(a, Page::Consoles),
        Msg::SignOut => {
            stream::stop();
            let _ = keyring::clear();
            *ACCOUNT.lock().unwrap() = None;
            a.signed_in = false;
            a.gamertag.clear();
            a.consoles = Load::Idle;
            a.games = Load::Idle;
            a.all_games = Load::Idle;
            a.cloud = false;
            go(a, Page::Welcome)
        }
        Msg::OpenStore => {
            hearth_ui::run::detached(&["hearth-store-app", "--page", "app:console-remote-play"]);
            Task::none()
        }
        Msg::CloudTab(t) => {
            a.tab = t;
            a.focus = 0;
            if t != CloudTab::Recent && a.all_games == Load::Idle {
                a.all_games = Load::Loading;
                return work(|| load_games(false), |r| Msg::Games(false, r));
            }
            Task::none()
        }
        Msg::Search(s) => {
            a.search = s;
            Task::none()
        }
        Msg::Games(recent, r) => {
            let t = match &r {
                Ok(list) => {
                    let l = list.clone();
                    pics_for(a, &l)
                }
                Err(_) => Task::none(),
            };
            let v = match r {
                Ok(l) => Load::Ready(l),
                Err(e) => Load::Failed(e),
            };
            if recent {
                a.games = v;
            } else {
                a.all_games = v;
            }
            t
        }
        Msg::Pic(u, b) => {
            match b {
                Some(b) if !b.is_empty() => {
                    if let Some((w, h, px)) = wide_cut(&b, 2.8) {
                        a.wide.insert(u.clone(), image::Handle::from_rgba(w, h, px));
                    }
                    a.pics.insert(u, image::Handle::from_bytes(b));
                }
                _ => {
                    a.pics.remove(&u);
                }
            }
            Task::none()
        }
        Msg::Play(i) => {
            a.last_game = Some(i);
            if a.prefs.limit_on && data::level(a.month_bytes(), a.prefs.limit_gb, a.prefs.warn_pct) == data::Level::Limit && a.ask_limit.is_none() {
                a.ask_limit = Some(usize::MAX);
                return Task::none();
            }
            begin_cloud(a, i)
        }
        Msg::Net(n) => {
            a.net = n;
            Task::none()
        }
        Msg::Size(w, h) => {
            a.size = (w, h);
            Task::none()
        }
        Msg::Used(u) => {
            a.used = u;
            check_warning(a)
        }
        Msg::Key(ev) => on_key(a, ev),
        Msg::Pad(b) => on_pad(a, b),
    }
}

fn start_stand_in(a: &mut App) -> Task<Msg> {
    let (name, model) = a.console().map(|c| (c.name.clone(), c.model.clone())).unwrap_or(("Living room".into(), "Xbox Series X".into()));
    a.last = Some(0);
    a.conn = Some(Conn { name, model, kind: Kind::Home, target: String::new(), waking: true, woke: None, step: 1, since: Instant::now(), queue: None, hd: !a.ally });
    let step = match a.page {
        Page::Connecting => Some("connecting"),
        Page::Trouble => Some("trouble"),
        _ => None,
    };
    if a.page == Page::Trouble {
        a.page = Page::Connecting;
    }
    stream::start_stand_in(if a.ally { (1280, 720) } else { (1920, 1080) }, step);
    Task::none()
}

fn begin_home(a: &mut App, i: usize) -> Task<Msg> {
    let Some(c) = (match &a.consoles {
        Load::Ready(v) => v.get(i).cloned(),
        _ => None,
    }) else {
        return Task::none();
    };
    if fixture().is_some() {
        a.page = Page::Connecting;
        return start_stand_in(a);
    }
    a.trouble = None;
    a.conn = Some(Conn { name: c.name.clone(), model: c.model.clone(), kind: Kind::Home, target: c.id.clone(), waking: c.power == Power::Sleeping, woke: None, step: if c.power == Power::Sleeping { 1 } else { 2 }, since: Instant::now(), queue: None, hd: true });
    a.page = Page::Connecting;
    a.focus = 0;
    let prefs = a.prefs.clone();
    let ally = a.ally;
    let saver_needs_net = prefs.saver == "hotspot";
    let quality = (prefs.quality.clone(), prefs.saver.clone());
    work(
        move || {
            let metered = saver_needs_net && read_net().metered;
            let saver = quality.1 == "always" || (quality.1 == "hotspot" && metered);
            let hd = match quality.0.as_str() {
                "720p" => false,
                "1080p" => !saver,
                _ => !saver,
            };
            let mut g = ACCOUNT.lock().unwrap();
            let acc = g.as_mut().ok_or("Not signed in.".to_string())?;
            let token = acc.home().map_err(|e| e.words())?;
            let wake = if c.power != Power::On { Some(acc.web().map_err(|e| e.words())?) } else { None };
            Ok(stream::Ask {
                kind: Kind::Home,
                token,
                target: c.id.clone(),
                wake,
                hd,
                cap_kbps: if saver { 5000 } else { 0 },
                out: if ally { (1280, 720) } else if hd { (1920, 1080) } else { (1280, 720) },
                volume: prefs.volume,
                sound: prefs.sound.clone(),
                vibration: prefs.vibration,
                prefer_builtin: prefs.play_with == "builtin",
                transfer: None,
            })
        },
        Msg::Begin,
    )
}

fn begin_cloud(a: &mut App, i: usize) -> Task<Msg> {
    let Some(g) = a.shown_games().get(i).cloned() else { return Task::none() };
    if fixture().is_some() {
        a.page = Page::Connecting;
        return start_stand_in(a);
    }
    a.trouble = None;
    a.conn = Some(Conn { name: g.name.clone(), model: "Cloud".into(), kind: Kind::Cloud, target: g.title.clone(), waking: false, woke: None, step: 2, since: Instant::now(), queue: None, hd: true });
    a.page = Page::Connecting;
    let prefs = a.prefs.clone();
    let ally = a.ally;
    let (hd0, _) = picture_for(a, false);
    work(
        move || {
            let metered = prefs.saver == "hotspot" && read_net().metered;
            let saver = prefs.saver == "always" || metered;
            let hd = hd0 && !saver;
            let mut gg = ACCOUNT.lock().unwrap();
            let acc = gg.as_mut().ok_or("Not signed in.".to_string())?;
            let token = acc.cloud().ok_or("Cloud needs Game Pass Ultimate on this account.".to_string())?;
            let transfer = xal::transfer_token(&acc.refresh).ok();
            Ok(stream::Ask {
                kind: Kind::Cloud,
                token,
                target: g.title.clone(),
                wake: None,
                hd,
                cap_kbps: if saver { 5000 } else { 0 },
                out: if ally { (1280, 720) } else if hd { (1920, 1080) } else { (1280, 720) },
                volume: prefs.volume,
                sound: prefs.sound.clone(),
                vibration: prefs.vibration,
                prefer_builtin: prefs.play_with == "builtin",
                transfer,
            })
        },
        Msg::Begin,
    )
}

fn check_warning(a: &mut App) -> Task<Msg> {
    if !a.prefs.limit_on || a.used.warned || a.warn_toast {
        return Task::none();
    }
    let lvl = data::level(a.month_bytes(), a.prefs.limit_gb, a.prefs.warn_pct);
    if lvl != data::Level::Fine {
        a.warn_toast = true;
        let reset = a.prefs.reset_day;
        return work(move || data::add(0, true, reset), Msg::Used);
    }
    Task::none()
}

fn on_stream(a: &mut App, e: SEv) -> Task<Msg> {
    match e {
        SEv::Step(s) => {
            if let Some(c) = a.conn.as_mut() {
                match s {
                    Step::Waking => {
                        c.waking = true;
                        c.step = 1;
                    }
                    Step::Woke(t) => {
                        c.woke = Some(t);
                        c.step = 2;
                        c.since = Instant::now();
                    }
                    Step::Connecting => {
                        if c.step < 2 && c.woke.is_none() && !c.waking {
                            c.since = Instant::now();
                        }
                        c.step = c.step.max(2);
                        if c.woke.is_some() || !c.waking {
                            c.step = 2;
                        }
                    }
                    Step::Picture => c.step = 3,
                    Step::Queue(q) => c.queue = Some(q),
                }
            }
            Task::none()
        }
        SEv::Started => {
            a.page = Page::Stream;
            a.trouble = None;
            a.counted = 0;
            a.stats = Stats::default();
            a.menu = a.shows("menu");
            stream::set_menu_open(a.menu);
            a.tip = a.shows("disconnect-tip");
            if a.shows("warning") {
                a.bar = false;
                a.pill = false;
            } else {
                a.pill = true;
            }
            let mut t = Vec::new();
            if !a.shows("warning") {
                t.push(show_bar(a));
            }
            a.pill_gen += 1;
            t.push(after(if a.ally { 3000 } else { 6000 }, Msg::PillHide(a.pill_gen)));
            if !a.ally && a.prefs.full_screen {
                t.push(set_full(a, true));
            }
            if a.prefs.keep_awake {
                a.inhibit = std::process::Command::new("systemd-inhibit")
                    .args(["--what=idle:sleep", "--who=Hearth Console Remote Play", "--why=Streaming your Xbox", "--mode=block", "sleep", "infinity"])
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn()
                    .ok();
            }
            if a.shows("shot") {
                t.push(update(a, Msg::Screenshot));
            }
            t.push(check_warning(a));
            Task::batch(t)
        }
        SEv::Frame => {
            if let Some(f) = stream::frame() {
                a.frame = Some(image::Handle::from_rgba(f.w, f.h, f.rgba));
            } else if a.frame.is_none() {
                a.frame = a.still.clone();
            }
            Task::none()
        }
        SEv::Stats(s) => {
            a.stats = s;
            check_warning(a)
        }
        SEv::Data(b) => {
            a.counted = a.stats.session_bytes;
            let reset = a.prefs.reset_day;
            work(move || data::add(b, false, reset), Msg::Used)
        }
        SEv::Failed { why, woke } => {
            a.trouble = Some((why, woke));
            a.page = Page::Trouble;
            a.focus = 0;
            a.ring = a.ally;
            work(read_net, Msg::Net)
        }
        SEv::Nav(n) => on_nav(a, n),
        SEv::Ended => stream_ended(a),
    }
}

fn on_nav(a: &mut App, n: Nav) -> Task<Msg> {
    match n {
        Nav::Menu => update(a, Msg::Menu),
        Nav::A => act(a, 0),
        Nav::B => update(a, Msg::Menu),
        Nav::Up => {
            move_focus(a, -1);
            Task::none()
        }
        Nav::Down => {
            move_focus(a, 1);
            Task::none()
        }
        Nav::Left => act(a, 1),
        Nav::Right => act(a, 2),
    }
}

fn on_key(a: &mut App, ev: KEvent) -> Task<Msg> {
    let KEvent::KeyPressed { key, modifiers, .. } = ev else { return Task::none() };
    let ctrl = modifiers.control();
    if ctrl && modifiers.shift() && matches!(&key, Key::Character(c) if c.as_str().eq_ignore_ascii_case("m")) {
        return update(a, Msg::Menu);
    }
    if a.page == Page::Stream && !a.menu {
        return match key {
            Key::Named(Named::F11) => update(a, Msg::FullScreen),
            Key::Named(Named::Escape) if a.full => update(a, Msg::FullScreen),
            _ => Task::none(),
        };
    }
    if a.dd.is_some() {
        if let Key::Named(Named::Escape) = key {
            a.dd = None;
        }
        return Task::none();
    }
    let in_search = a.page == Page::Cloud && a.tab == CloudTab::Search;
    match key {
        Key::Named(Named::Escape) => back(a),
        Key::Named(Named::ArrowUp) => {
            move_focus(a, if a.page == Page::Cloud && a.focus > 0 && !a.menu { -4 } else { -1 });
            Task::none()
        }
        Key::Named(Named::ArrowDown) => {
            move_focus(a, if a.page == Page::Cloud && a.focus > 0 && !a.menu { 4 } else { 1 });
            Task::none()
        }
        Key::Named(Named::ArrowLeft) if !in_search => {
            if a.page == Page::Cloud && a.focus > 0 && !a.menu {
                move_focus(a, -1);
                return Task::none();
            }
            act(a, 1)
        }
        Key::Named(Named::ArrowRight) if !in_search => {
            if a.page == Page::Cloud && a.focus > 0 && !a.menu {
                move_focus(a, 1);
                return Task::none();
            }
            act(a, 2)
        }
        Key::Named(Named::Enter) => act(a, 0),
        Key::Named(Named::PageUp) if ctrl => step_tab(a, -1),
        Key::Named(Named::PageDown) if ctrl => step_tab(a, 1),
        Key::Named(Named::F5) if a.page == Page::Consoles => update(a, Msg::Refresh),
        _ => Task::none(),
    }
}

fn on_pad(a: &mut App, b: pad::Button) -> Task<Msg> {
    use pad::Button as P;
    // While streaming the stream's own reader has the controller (spad.rs).
    if a.page == Page::Stream || a.page == Page::SignIn {
        return Task::none();
    }
    if let Some(d) = a.dd {
        return match b {
            P::B => {
                a.dd = None;
                Task::none()
            }
            P::A => {
                // A picks the next choice in the open chooser (the chooser shows them all).
                let v = desk::dd_choices(a, d);
                let now = desk::dd_now(a, d);
                let i = v.iter().position(|(val, _)| *val == now).map(|i| (i + 1) % v.len().max(1)).unwrap_or(0);
                match v.get(i) {
                    Some((val, _)) => update(a, Msg::Pick(d, val.clone())),
                    None => Task::none(),
                }
            }
            _ => Task::none(),
        };
    }
    let cloud_tiles = a.page == Page::Cloud && a.focus > 0;
    match b {
        P::Up => {
            move_focus(a, if cloud_tiles { -4 } else { -1 });
            Task::none()
        }
        P::Down => {
            move_focus(a, if cloud_tiles { 4 } else { 1 });
            Task::none()
        }
        P::Left if cloud_tiles => {
            move_focus(a, -1);
            Task::none()
        }
        P::Right if cloud_tiles => {
            move_focus(a, 1);
            Task::none()
        }
        P::Left => act(a, 1),
        P::Right => act(a, 2),
        P::A => act(a, 0),
        P::B => back(a),
        P::X => match a.page {
            Page::Consoles => update(a, Msg::Wake(a.focus)),
            Page::Trouble => update(a, Msg::TryAgain(false)),
            _ => Task::none(),
        },
        P::Y => match a.page {
            Page::Consoles => update(a, Msg::Refresh),
            Page::Cloud => update(a, Msg::CloudTab(CloudTab::Search)),
            _ => Task::none(),
        },
        P::Lb => step_tab(a, -1),
        P::Rb => step_tab(a, 1),
        P::Menu | P::View => Task::none(),
    }
}

/// A picture cut to this shape (width / height), from its middle: the Cloud library's smaller tiles.
fn wide_cut(bytes: &[u8], ratio: f32) -> Option<(u32, u32, Vec<u8>)> {
    let img = ::image::load_from_memory(bytes).ok()?.to_rgba8();
    let (w, h) = img.dimensions();
    let nh = ((w as f32 / ratio).round() as u32).clamp(1, h);
    let y = (h - nh) / 2;
    let cut = ::image::imageops::crop_imm(&img, 0, y, w, nh).to_image();
    Some((w, nh, cut.into_raw()))
}

/// Saves a screenshot: Pictures › Screenshots › Console Remote Play YYYY-MM-DD HH-MM-SS.png.
fn save_shot(px: Option<(u32, u32, Vec<u8>)>, still: Option<PathBuf>) -> Result<PathBuf, String> {
    let pics = std::env::var_os("XDG_PICTURES_DIR").map(PathBuf::from).or_else(|| {
        let o = output(&["xdg-user-dir", "PICTURES"], 3);
        let o = o.trim();
        (!o.is_empty() && o != crp::home().to_string_lossy()).then(|| PathBuf::from(o))
    });
    let dir = pics.unwrap_or_else(|| crp::home().join("Pictures")).join("Screenshots");
    std::fs::create_dir_all(&dir).map_err(|e| format!("Couldn't make the Screenshots folder: {e}"))?;
    let d = data::today();
    let t = crp::now() as libc::time_t;
    // SAFETY: localtime_r fills the struct we give it.
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    unsafe { libc::localtime_r(&t, &mut tm) };
    let file = dir.join(format!("Console Remote Play {:04}-{:02}-{:02} {:02}-{:02}-{:02}.png", d.y, d.m, d.d, tm.tm_hour, tm.tm_min, tm.tm_sec));
    let img = match (px, still) {
        (Some((w, h, rgba)), _) => ::image::RgbaImage::from_raw(w, h, rgba).map(::image::DynamicImage::ImageRgba8),
        (None, Some(s)) => ::image::open(s).ok(),
        _ => None,
    };
    let img = img.ok_or("There's no picture to save yet.")?;
    img.save_with_format(&file, ::image::ImageFormat::Png).map_err(|e| format!("Couldn't save the screenshot: {e}"))?;
    Ok(file)
}

// ------------------------------------------------------------------ the window
fn view(a: &App) -> iced::Element<'_, Msg> {
    if a.ally {
        ally::view(a)
    } else {
        desk::view(a)
    }
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
    let mut s = vec![iced::keyboard::listen().map(Msg::Key), Subscription::run(pad::presses).map(Msg::Pad), Subscription::run(stream::events).map(Msg::Ev), iced::event::listen_with(sized)];
    if a.page == Page::Connecting {
        // The timer on Connecting (0:04): only while connecting.
        s.push(iced::time::every(Duration::from_secs(1)).map(|_| Msg::Tick));
    }
    if a.page == Page::Stream && !a.ally {
        s.push(iced::event::listen_with(mouse_moved));
    }
    Subscription::batch(s)
}

fn theme(_a: &App) -> iced::Theme {
    iced::Theme::custom("Hearth Console Remote Play", iced::theme::Palette { background: look::BG, text: look::TX, primary: look::ACC, success: look::OK, warning: look::WARN, danger: look::BAD })
}

fn main() -> iced::Result {
    if app::handle_cli(
        "hearth-console-remote-play",
        "Hearth Console Remote Play: play your own Xbox from anywhere (a Hearth Store app; not made by Microsoft).\n  --page welcome|signin|signed-in|consoles|connecting|stream|trouble|settings|cloud\n  --game-mode             the Ally's Game Mode pages (also when started in gamescope)\n  --show menu,bar,pill,disconnect-tip,warning,shot   (pictures)",
    ) {
        return Ok(());
    }
    let _ = input::HOLD_MS;
    let ally = game_mode();
    let mut w = app::window("hearth-console-remote-play", (1600.0, 900.0), (1100.0, 680.0));
    if ally {
        w.fullscreen = true;
    }
    iced::application(App::new, update, view)
        .title("Hearth Console Remote Play")
        .subscription(subscription)
        .theme(theme)
        .window(w)
        .default_font(app::INTER)
        .run()
}
