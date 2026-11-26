//! Hearth Retro, in pure Rust (hearth-ui, iced): the approved board docs/mockups/open-source-apps/04-retro.png
//! (checklist docs/mockups/checklists/open-source-04-retro.md). In Game Mode, full screen and controller first; in the
//! desktop the same in a window. Built on RetroDECK and EmuDeck (GPL-3): their ROM, BIOS and save folders, the
//! emulators they set up, ES-DE's game lists and scraped covers (lib.rs).
//!
//! The controller (read straight from /dev/input, Hearth Notes' pad.rs, only while Hearth Retro is in front and no
//! game is running): D-pad / stick move, A plays, X the game's saves, Y favourite, ≡ options, B back, LB / RB the
//! tabs. The keyboard does the same (arrows, Enter, Esc, F, S, Tab / Menu, Ctrl+Page Up / Down), as hearth-pad sends.
//!
//! `hearth-retro-app --play FILE` starts that game in its emulator with no window (Retro to Steam's Steam shortcuts).
//! Nothing at rest: no timers but the clock's (once a minute), no polling; the controller thread sleeps in the kernel.

#[path = "../../notes/src/pad.rs"]
#[allow(dead_code)]
mod pad;
mod view;

use hearth_retro::bios::{self, Check};
use hearth_retro::emus::{self, Here};
use hearth_retro::library::{self, Game, System};
use hearth_retro::pads::{self, Pad};
use hearth_retro::prefs;
use hearth_ui::app::{self, win};
use hearth_ui::run;
use hearth_ui::Pal;
use iced::keyboard::{key::Named, Event as KEvent, Key};
use iced::{Element, Subscription, Task};
use serde_json::{Map, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Shelves,
    Setup,
    Bios,
    Pads,
    Search,
}

impl Tab {
    pub const ALL: [Tab; 5] = [Tab::Shelves, Tab::Setup, Tab::Bios, Tab::Pads, Tab::Search];
    pub fn label(self) -> &'static str {
        match self {
            Tab::Shelves => "Shelves",
            Tab::Setup => "Set up",
            Tab::Bios => "BIOS check",
            Tab::Pads => "Controllers",
            Tab::Search => "Search",
        }
    }
    pub fn id(self) -> &'static str {
        match self {
            Tab::Shelves => "shelves",
            Tab::Setup => "setup",
            Tab::Bios => "bios",
            Tab::Pads => "pads",
            Tab::Search => "search",
        }
    }
    fn from_id(s: &str) -> Option<Tab> {
        Tab::ALL.iter().copied().find(|t| t.id() == s)
    }
}

/// A dialog over the page.
#[derive(Debug, Clone, PartialEq)]
pub enum Dlg {
    /// the picked game's save states (file, slot, when)
    Saves(Vec<(String, i32, i64)>),
    /// where a missing BIOS goes (the check's index)
    Where(usize),
    /// why a BIOS file is the wrong one
    Why(usize),
    /// the options (≡, ⋯)
    Options,
    /// RetroAchievements: your name and your own web API key
    Ra,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Describe(hearth_ui::describe::DMsg),
    Theme,
    Scanned(Vec<(&'static System, Vec<Game>)>),
    Looked(Here),
    Bios(Vec<Check>),
    Pads(Vec<Pad>),
    Ra(Value),
    Tab(Tab),
    Pick(usize, usize),
    Play,
    PlayGame(usize, usize),
    PlaySlot(i32),
    Ended(i32),
    Saves,
    Fav,
    Options,
    MenuPick(usize),
    Back,
    SetUp,
    SetUpDone(run::Output),
    Where(usize),
    Why(usize),
    CloseDlg,
    Search(String),
    SeeAll(&'static str),
    OpenDir(String),
    ToggleLook(bool),
    RaName(String),
    RaKey(String),
    RaSave,
    ReadAgain,
    Pad(pad::Button),
    Key(KEvent),
    Focus(bool),
    Clock,
    Drag,
    Max,
    Min,
    Close,
    Resize(iced::window::Direction),
}

pub struct App {
    pub describe: hearth_ui::describe::Describe,
    pub p: Pal,
    pub game_mode: bool,
    pub tab: Tab,
    pub shelves: Vec<(&'static System, Vec<Game>)>,
    pub scanned: bool,
    pub here: Option<Here>,
    pub bios: Vec<Check>,
    pub pads: Vec<Pad>,
    pub prefs: Map<String, Value>,
    pub played: Map<String, Value>,
    /// RetroAchievements progress by game title (lowercase): (won, of)
    pub ra: std::collections::HashMap<String, (u64, u64)>,
    /// the picked tile: (shelf, game)
    pub sel: (usize, usize),
    /// the first shelf shown (the controller keeps the picked one in view)
    pub top: usize,
    pub dlg: Option<Dlg>,
    pub menu_sel: usize,
    pub search: String,
    pub search_sys: Option<&'static str>,
    pub search_sel: usize,
    pub bios_sel: usize,
    pub busy: String,
    pub said: String,
    pub running: bool,
    pub focused: bool,
    pub now: i64,
    pub ra_name: String,
    pub ra_key: String,
    pub battery: Option<u8>,
}

fn now() -> i64 {
    hearth_ui::time::now()
}

impl App {
    fn new() -> (App, Task<Msg>) {
        let game_mode = app::flag("--game-mode")
            || std::env::var_os("GAMESCOPE_WAYLAND_DISPLAY").is_some()
            || std::env::var("XDG_CURRENT_DESKTOP").map(|d| d.to_lowercase().contains("gamescope")).unwrap_or(false);
        let prefs = prefs::read();
        let a = App {
            describe: hearth_ui::describe::Describe::new("retro", "Hearth Retro").with_pages(view::BLOCKS).with_kit(&view::KIT),
            p: hearth_ui::theme::load(),
            game_mode,
            tab: app::arg("--page").and_then(|t| Tab::from_id(&t)).unwrap_or(Tab::Shelves),
            shelves: Vec::new(),
            scanned: false,
            here: None,
            bios: Vec::new(),
            pads: Vec::new(),
            ra_name: prefs.get("raUser").and_then(Value::as_str).unwrap_or("").to_string(),
            ra_key: String::new(),
            prefs,
            played: prefs::played(),
            ra: Default::default(),
            sel: (0, 0),
            top: 0,
            dlg: None,
            menu_sel: 0,
            search: app::arg("--search").unwrap_or_default(),
            search_sys: None,
            search_sel: 0,
            bios_sel: 0,
            busy: String::new(),
            said: String::new(),
            running: false,
            focused: true,
            now: now(),
            battery: pads::own_battery(),
        };
        (a, Task::batch([read_all(), read_ra()]))
    }

    pub fn picked(&self) -> Option<(&'static System, &Game)> {
        let (s, g) = self.sel;
        self.shelves.get(s).and_then(|(sys, games)| games.get(g).map(|x| (*sys, x)))
    }

    pub fn total(&self) -> usize {
        self.shelves.iter().map(|x| x.1.len()).sum()
    }

    /// The search's results (every console, or one: "see all ›").
    pub fn results(&self) -> Vec<(usize, usize)> {
        let q = self.search.to_lowercase();
        let mut v = Vec::new();
        for (si, (sys, games)) in self.shelves.iter().enumerate() {
            if self.search_sys.map(|s| s != sys.id).unwrap_or(false) {
                continue;
            }
            for (gi, g) in games.iter().enumerate() {
                if q.is_empty() || g.title.to_lowercase().contains(&q) {
                    v.push((si, gi));
                }
            }
        }
        v
    }

    pub fn is_fav(&self, g: &Game) -> bool {
        prefs::favourites(&self.prefs).iter().any(|f| *f == g.path.display().to_string())
    }

    /// The picked game's save states: (file name, slot, when), newest first.
    pub fn states(&self, g: &Game) -> Vec<(String, i32, i64)> {
        let h = hearth_retro::home();
        let r = hearth_retro::retro_dir();
        let mut dirs = vec![r.join("states"), h.join("retrodeck/states").join(g.system)];
        if let Some(sys) = library::system(g.system) {
            for f in sys.folders {
                dirs.push(h.join("retrodeck/states").join(f));
                dirs.push(h.join("Emulation/saves/retroarch/states").join(f));
            }
        }
        let mut out = Vec::new();
        for d in dirs {
            let Ok(rd) = std::fs::read_dir(&d) else { continue };
            for e in rd.flatten() {
                let n = e.file_name().to_string_lossy().to_string();
                let Some(rest) = n.strip_prefix(&format!("{}.state", g.stem)) else { continue };
                if rest.ends_with(".png") {
                    continue;
                }
                let slot = if rest.is_empty() { 0 } else if rest == ".auto" { -1 } else { rest.parse::<i32>().unwrap_or(0) };
                let when = e.metadata().ok().and_then(|m| m.modified().ok()).and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs() as i64).unwrap_or(0);
                if !out.iter().any(|x: &(String, i32, i64)| x.0 == n) {
                    out.push((n, slot, when));
                }
            }
        }
        out.sort_by(|a, b| b.2.cmp(&a.2));
        out
    }

    pub fn last_played(&self, g: &Game) -> Option<i64> {
        self.played.get(&g.path.display().to_string()).and_then(|v| v.get("last")).and_then(Value::as_i64)
    }

    /// Menu items for ≡ / ⋯: (label, what it does).
    pub fn menu_items(&self) -> Vec<(String, Msg)> {
        let mut v: Vec<(String, Msg)> = Vec::new();
        if let Some((_, g)) = self.picked() {
            let (crt, smooth) = prefs::look(&self.prefs, &g.path.display().to_string());
            v.push((if self.is_fav(g) { "Not a favourite".into() } else { "Favourite".into() }, Msg::Fav));
            v.push((format!("CRT look: {}", if crt { "on" } else { "off" }), Msg::ToggleLook(true)));
            v.push((format!("4:3 smooth: {}", if smooth { "on" } else { "off" }), Msg::ToggleLook(false)));
            v.push(("Open the game's folder".into(), Msg::OpenDir(g.path.parent().map(|p| p.display().to_string()).unwrap_or_default())));
        }
        v.push(("RetroAchievements…".into(), Msg::MenuPick(901)));
        v.push(("Read the games again".into(), Msg::ReadAgain));
        v.push(("Edit layout".into(), Msg::Describe(hearth_ui::arrange::edit_layout())));
        v.push(("Describe it…".into(), Msg::Describe(hearth_ui::describe::DMsg::Open)));
        v.push(("Quit Hearth Retro".into(), Msg::Close));
        v
    }
}

/// Everything read again, off the window's thread: the games, the emulators, the BIOS files, the controllers.
fn read_all() -> Task<Msg> {
    Task::batch([
        Task::perform(off(library::scan), Msg::Scanned),
        Task::perform(off(emus::look), Msg::Looked),
        Task::perform(off(bios::check), Msg::Bios),
        Task::perform(off(pads::list), Msg::Pads),
    ])
}

/// RetroAchievements: your progress, with your own name and web API key (only when you gave them; CI's fixture
/// says it instead). One request for every game.
fn read_ra() -> Task<Msg> {
    let fx = hearth_retro::fixture();
    if let Some(r) = fx.get("ra") {
        let r = r.clone();
        return Task::done(Msg::Ra(r));
    }
    let m = prefs::read();
    let (u, k) = (m.get("raUser").and_then(Value::as_str).unwrap_or(""), m.get("raKey").and_then(Value::as_str).unwrap_or(""));
    if u.is_empty() || k.is_empty() {
        return Task::none();
    }
    let url = format!("https://retroachievements.org/API/API_GetUserCompletionProgress.php?u={}&y={}&c=500", enc(u), enc(k));
    Task::perform(run::spawn(vec!["curl".into(), "-fsS".into(), "--max-time".into(), "20".into(), url]), |o| {
        let mut out = serde_json::Map::new();
        if let Some(v) = o.json() {
            for r in v.get("Results").and_then(Value::as_array).cloned().unwrap_or_default() {
                let t = r.get("Title").and_then(Value::as_str).unwrap_or("").to_lowercase();
                let won = r.get("NumAwarded").and_then(Value::as_u64).unwrap_or(0);
                let of = r.get("MaxPossible").and_then(Value::as_u64).unwrap_or(0);
                if !t.is_empty() && of > 0 {
                    out.insert(t, serde_json::json!([won, of]));
                }
            }
        }
        Msg::Ra(Value::Object(out))
    })
}

fn enc(s: &str) -> String {
    s.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect()
}

/// `f` on a thread of its own; the future is ready with its answer.
fn off<T: Send + 'static>(f: fn() -> T) -> impl std::future::Future<Output = T> + Send + 'static {
    let (tx, rx) = futures::channel::oneshot::channel();
    let _ = std::thread::Builder::new().name("hearth-retro-read".into()).spawn(move || {
        let _ = tx.send(f());
    });
    async move { rx.await.expect("the reading thread ended") }
}

/// The emulator, started and waited for on a thread of its own (its output goes nowhere); ready with its exit code.
fn start(cmd: Vec<String>) -> impl std::future::Future<Output = i32> + Send + 'static {
    let (tx, rx) = futures::channel::oneshot::channel();
    let _ = std::thread::Builder::new().name("hearth-retro-game".into()).spawn(move || {
        let code = std::process::Command::new(&cmd[0])
            .args(&cmd[1..])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.code().unwrap_or(0))
            .unwrap_or(-1);
        let _ = tx.send(code);
    });
    async move { rx.await.unwrap_or(-1) }
}

fn play(a: &mut App, si: usize, gi: usize, slot: Option<i32>) -> Task<Msg> {
    let Some((sys, g)) = a.shelves.get(si).and_then(|(s, gs)| gs.get(gi).map(|g| (*s, g.clone()))) else { return Task::none() };
    let Some(here) = a.here.clone() else {
        a.said = "Still looking for the emulators…".into();
        return Task::none();
    };
    let look = prefs::look(&a.prefs, &g.path.display().to_string());
    match emus::command(&g, sys, &here, slot, look) {
        Ok(cmd) => {
            if hearth_retro::fixture().get("installed").is_some() {
                a.said = format!("Would start: {}", cmd.join(" "));
                return Task::none();
            }
            prefs::mark_played(&g.path.display().to_string(), now());
            a.played = prefs::played();
            a.running = true;
            a.said = format!("Playing {} · {}", g.title, sys.emu);
            Task::perform(start(cmd), Msg::Ended)
        }
        Err(e) => {
            a.said = e;
            Task::none()
        }
    }
}

fn move_sel(a: &mut App, dx: i32, dy: i32) {
    match a.tab {
        Tab::Shelves => {
            if a.shelves.is_empty() {
                return;
            }
            let (mut s, mut g) = (a.sel.0 as i32, a.sel.1 as i32);
            if dy != 0 {
                s = (s + dy).clamp(0, a.shelves.len() as i32 - 1);
                g = g.min(a.shelves[s as usize].1.len() as i32 - 1);
            }
            g = (g + dx).clamp(0, a.shelves[s as usize].1.len() as i32 - 1);
            a.sel = (s as usize, g as usize);
            // the picked shelf stays among the three in view
            if a.sel.0 < a.top {
                a.top = a.sel.0;
            } else if a.sel.0 >= a.top + 3 {
                a.top = a.sel.0 - 2;
            }
        }
        Tab::Search => {
            let n = a.results().len() as i32;
            if n == 0 {
                return;
            }
            let i = (a.search_sel as i32 + dx + dy * 8).clamp(0, n - 1);
            a.search_sel = i as usize;
            if let Some(&(s, g)) = a.results().get(a.search_sel) {
                a.sel = (s, g);
            }
        }
        Tab::Bios => {
            let n = a.bios.len() as i32;
            if n > 0 {
                a.bios_sel = (a.bios_sel as i32 + dy).clamp(0, n - 1) as usize;
            }
        }
        _ => {}
    }
}

fn set_tab(a: &mut App, t: Tab) {
    a.tab = t;
    a.dlg = None;
    if t == Tab::Search {
        a.search_sel = 0;
    }
}

fn next_tab(a: &mut App, d: i32) {
    let i = Tab::ALL.iter().position(|x| *x == a.tab).unwrap_or(0) as i32;
    set_tab(a, Tab::ALL[((i + d).rem_euclid(Tab::ALL.len() as i32)) as usize]);
}

fn menu_len(a: &App) -> usize {
    match &a.dlg {
        Some(Dlg::Options) => a.menu_items().len(),
        Some(Dlg::Saves(s)) => s.len(),
        _ => 0,
    }
}

/// A controller button or its key, the same everywhere.
fn press(a: &mut App, b: pad::Button) -> Task<Msg> {
    use pad::Button as B;
    // Edit layout takes the controller while it's open (hearth-ui's own: move, size, keep)
    if a.describe.studio_editing() {
        let name = match b {
            B::A => "a",
            B::B => "b",
            B::X => "x",
            B::Y => "y",
            B::Up => "up",
            B::Down => "down",
            B::Left => "left",
            B::Right => "right",
            B::Lb => "lb",
            B::Rb => "rb",
            B::Menu => "menu",
            _ => "",
        };
        if let Some(t) = hearth_ui::arrange::pad(&mut a.describe, name) {
            return t.map(Msg::Describe);
        }
        return Task::none();
    }
    if a.dlg.is_some() {
        let n = menu_len(a);
        match b {
            B::Up if n > 0 => a.menu_sel = a.menu_sel.saturating_sub(1),
            B::Down if n > 0 => a.menu_sel = (a.menu_sel + 1).min(n - 1),
            B::A => return update(a, Msg::MenuPick(a.menu_sel)),
            B::B | B::Menu => a.dlg = None,
            _ => {}
        }
        return Task::none();
    }
    match b {
        B::Up => move_sel(a, 0, -1),
        B::Down => move_sel(a, 0, 1),
        B::Left => move_sel(a, -1, 0),
        B::Right => move_sel(a, 1, 0),
        B::Lb => next_tab(a, -1),
        B::Rb => next_tab(a, 1),
        B::A => {
            return match a.tab {
                Tab::Shelves | Tab::Search => update(a, Msg::Play),
                Tab::Setup => update(a, Msg::SetUp),
                Tab::Bios => {
                    let i = a.bios_sel;
                    match a.bios.get(i).map(|c| c.state.clone()) {
                        Some(bios::State::Missing) => update(a, Msg::Where(i)),
                        Some(bios::State::Bad(..)) => update(a, Msg::Why(i)),
                        _ => Task::none(),
                    }
                }
                Tab::Pads => Task::none(),
            }
        }
        B::X => return update(a, Msg::Saves),
        B::Y => return update(a, Msg::Fav),
        B::Menu => return update(a, Msg::Options),
        B::B => return update(a, Msg::Back),
        _ => {}
    }
    Task::none()
}

fn update(a: &mut App, m: Msg) -> Task<Msg> {
    match m {
        Msg::Describe(dm) => {
            if let Some(m) = hearth_ui::studio::act(&dm).and_then(view::block_action) {
                return update(a, m);
            }
            return a.describe.update(dm).map(Msg::Describe);
        }
        Msg::Theme => a.p = hearth_ui::theme::load(),
        Msg::Scanned(mut s) => {
            // each shelf: the games played most recently first, then by name
            for (_, games) in s.iter_mut() {
                games.sort_by_key(|g| (std::cmp::Reverse(a.played.get(&g.path.display().to_string()).and_then(|v| v.get("last")).and_then(Value::as_i64).unwrap_or(0)), g.title.to_lowercase()));
            }
            a.shelves = s;
            a.scanned = true;
            a.sel = (a.sel.0.min(a.shelves.len().saturating_sub(1)), 0);
            if let Some(t) = app::arg("--pick") {
                // CI's pictures and a shortcut: --pick "Chrono Trigger"
                for (si, (_, gs)) in a.shelves.iter().enumerate() {
                    if let Some(gi) = gs.iter().position(|g| g.title == t) {
                        a.sel = (si, gi);
                    }
                }
            }
            match app::arg("--show").as_deref() {
                Some("saves") => return update(a, Msg::Saves),
                Some("options") => a.dlg = Some(Dlg::Options),
                _ => {}
            }
        }
        Msg::Looked(h) => a.here = Some(h),
        Msg::Bios(b) => {
            a.bios = b;
            if let Some(w) = app::arg("--show") {
                let find = |st: fn(&bios::State) -> bool| a.bios.iter().position(|c| st(&c.state));
                if w == "where" {
                    if let Some(i) = find(|s| *s == bios::State::Missing) {
                        a.dlg = Some(Dlg::Where(i));
                    }
                } else if w == "why" {
                    if let Some(i) = find(|s| matches!(s, bios::State::Bad(..))) {
                        a.dlg = Some(Dlg::Why(i));
                    }
                }
            }
        }
        Msg::Pads(p) => a.pads = p,
        Msg::Ra(v) => {
            a.ra.clear();
            if let Some(o) = v.as_object() {
                for (k, x) in o {
                    let won = x.get(0).and_then(Value::as_u64).unwrap_or(0);
                    let of = x.get(1).and_then(Value::as_u64).unwrap_or(0);
                    a.ra.insert(k.to_lowercase(), (won, of));
                }
            }
        }
        Msg::Tab(t) => set_tab(a, t),
        Msg::Pick(s, g) => {
            a.sel = (s, g);
            a.said.clear();
        }
        Msg::Play => {
            let (s, g) = a.sel;
            return play(a, s, g, None);
        }
        Msg::PlayGame(s, g) => {
            a.sel = (s, g);
            return play(a, s, g, None);
        }
        Msg::PlaySlot(slot) => {
            a.dlg = None;
            let (s, g) = a.sel;
            return play(a, s, g, Some(slot));
        }
        Msg::Ended(code) => {
            a.running = false;
            a.said = if code == 0 || code == -1 { String::new() } else { format!("The emulator ended with code {code}") };
            a.played = prefs::played();
        }
        Msg::Saves => {
            if let Some((_, g)) = a.picked() {
                let s = a.states(g);
                a.menu_sel = 0;
                a.dlg = Some(Dlg::Saves(s));
            }
        }
        Msg::Fav => {
            if let Some((_, g)) = a.picked() {
                let rom = g.path.display().to_string();
                let on = !a.is_fav(g);
                let r = prefs::change(move |m| {
                    let mut favs: Vec<Value> = m.get("favourites").and_then(Value::as_array).cloned().unwrap_or_default();
                    favs.retain(|x| x.as_str() != Some(rom.as_str()));
                    if on {
                        favs.push(Value::String(rom.clone()));
                    }
                    m.insert("favourites".into(), Value::Array(favs));
                });
                if let Err(e) = r {
                    a.said = format!("It couldn't be saved: {e}");
                }
                a.prefs = prefs::read();
                if matches!(a.dlg, Some(Dlg::Options)) {
                    a.dlg = None;
                }
            }
        }
        Msg::Options => {
            a.menu_sel = 0;
            a.dlg = if a.dlg == Some(Dlg::Options) { None } else { Some(Dlg::Options) };
        }
        Msg::MenuPick(i) => match a.dlg.clone() {
            Some(Dlg::Options) => {
                if i == 901 {
                    a.dlg = Some(Dlg::Ra);
                    return Task::none();
                }
                let items = a.menu_items();
                if let Some((_, msg)) = items.into_iter().nth(i) {
                    if !matches!(msg, Msg::ToggleLook(_)) {
                        a.dlg = None;
                    }
                    return update(a, msg);
                }
            }
            Some(Dlg::Saves(s)) => {
                if let Some(x) = s.get(i) {
                    return update(a, Msg::PlaySlot(x.1));
                }
            }
            Some(Dlg::Where(_)) | Some(Dlg::Why(_)) => a.dlg = None,
            _ => {}
        },
        Msg::Back => {
            if a.dlg.is_some() {
                a.dlg = None;
            } else if a.tab != Tab::Shelves {
                set_tab(a, Tab::Shelves);
                a.search_sys = None;
            } else if !a.game_mode {
                return win::close();
            }
        }
        Msg::SetUp => {
            if !a.busy.is_empty() {
                return Task::none();
            }
            if let Err(e) = emus::write_cfg() {
                a.said = format!("Hearth Retro's folders couldn't be made: {e}");
                return Task::none();
            }
            let missing = a.here.as_ref().map(|h| h.missing().len()).unwrap_or(0);
            if missing == 0 {
                a.said = "Every emulator is set up the same way.".into();
                return Task::perform(off(emus::look), Msg::Looked);
            }
            a.busy = "Installing RetroDECK from Flathub (every emulator, for you only)…".into();
            return Task::perform(run::spawn(emus::install_cmd()), Msg::SetUpDone);
        }
        Msg::SetUpDone(o) => {
            a.busy.clear();
            a.said = if o.ok { "Set up: every emulator installed and set the same way.".into() } else { format!("It couldn't be installed: {}", o.why()) };
            return read_all();
        }
        Msg::Where(i) => {
            a.dlg = Some(Dlg::Where(i));
            a.menu_sel = 0;
        }
        Msg::Why(i) => {
            a.dlg = Some(Dlg::Why(i));
            a.menu_sel = 0;
        }
        Msg::CloseDlg => a.dlg = None,
        Msg::Search(s) => {
            a.search = s;
            a.search_sel = 0;
        }
        Msg::SeeAll(sys) => {
            a.search_sys = Some(sys);
            a.search.clear();
            set_tab(a, Tab::Search);
        }
        Msg::OpenDir(d) => {
            let _ = std::fs::create_dir_all(&d);
            run::detached(&["xdg-open", &d]);
        }
        Msg::ToggleLook(crt) => {
            if let Some((_, g)) = a.picked() {
                let rom = g.path.display().to_string();
                let (c, s) = prefs::look(&a.prefs, &rom);
                let (c, s) = if crt { (!c, s) } else { (c, !s) };
                if let Err(e) = prefs::change(move |m| {
                    let mut looks = m.get("looks").and_then(Value::as_object).cloned().unwrap_or_default();
                    looks.insert(rom, serde_json::json!({"crt": c, "smooth": s}));
                    m.insert("looks".into(), Value::Object(looks));
                }) {
                    a.said = format!("It couldn't be saved: {e}");
                }
                a.prefs = prefs::read();
            }
        }
        Msg::RaName(s) => a.ra_name = s,
        Msg::RaKey(s) => a.ra_key = s,
        Msg::RaSave => {
            let (u, k) = (a.ra_name.trim().to_string(), a.ra_key.trim().to_string());
            let r = prefs::change(move |m| {
                m.insert("raUser".into(), Value::String(u));
                if !k.is_empty() {
                    m.insert("raKey".into(), Value::String(k));
                }
            });
            // your key stays yours: only you can read the file
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(prefs::path(), std::fs::Permissions::from_mode(0o600));
            }
            a.ra_key.clear();
            a.dlg = None;
            if let Err(e) = r {
                a.said = format!("It couldn't be saved: {e}");
            }
            a.prefs = prefs::read();
            return read_ra();
        }
        Msg::ReadAgain => {
            a.said = "Reading your games again…".into();
            return read_all();
        }
        Msg::Pad(b) => {
            if a.focused && !a.running {
                return press(a, b);
            }
        }
        Msg::Key(ev) => {
            if let KEvent::KeyPressed { key, modifiers, .. } = ev {
                if let Some(dm) = hearth_ui::arrange::arrange_key(&a.describe, &key, modifiers) {
                    return update(a, Msg::Describe(dm));
                }
                use pad::Button as B;
                let typing = a.tab == Tab::Search && a.dlg.is_none();
                let b = match key.as_ref() {
                    Key::Named(Named::ArrowUp) => Some(B::Up),
                    Key::Named(Named::ArrowDown) => Some(B::Down),
                    Key::Named(Named::ArrowLeft) if !typing => Some(B::Left),
                    Key::Named(Named::ArrowRight) if !typing => Some(B::Right),
                    Key::Named(Named::Enter) => Some(B::A),
                    Key::Named(Named::Escape) => Some(B::B),
                    Key::Named(Named::PageUp) if modifiers.control() => Some(B::Lb),
                    Key::Named(Named::PageDown) if modifiers.control() => Some(B::Rb),
                    Key::Named(Named::ContextMenu) | Key::Named(Named::Tab) => Some(B::Menu),
                    Key::Character("s") if !typing => Some(B::X),
                    Key::Character("f") if !typing => Some(B::Y),
                    _ => None,
                };
                if let Some(b) = b {
                    return press(a, b);
                }
            }
        }
        Msg::Focus(f) => a.focused = f,
        Msg::Clock => {
            a.now = now();
            a.battery = pads::own_battery();
        }
        Msg::Drag => return win::drag(),
        Msg::Max => return win::toggle_maximize(),
        Msg::Min => return win::minimize(),
        Msg::Close => return win::close(),
        Msg::Resize(d) => return win::resize(d),
    }
    Task::none()
}

fn window_events(ev: iced::Event, _s: iced::event::Status, _id: iced::window::Id) -> Option<Msg> {
    match ev {
        iced::Event::Window(iced::window::Event::Focused) => Some(Msg::Focus(true)),
        iced::Event::Window(iced::window::Event::Unfocused) => Some(Msg::Focus(false)),
        _ => None,
    }
}

fn subscription(_a: &App) -> Subscription<Msg> {
    Subscription::batch([
        Subscription::run(hearth_ui::theme::watch).map(|_| Msg::Theme),
        iced::keyboard::listen().map(Msg::Key),
        iced::event::listen_with(window_events),
        Subscription::run(pad::presses).map(Msg::Pad),
        iced::time::every(std::time::Duration::from_secs(60)).map(|_| Msg::Clock),
    ])
}

fn shown(a: &App) -> Element<'_, Msg> {
    hearth_ui::describe::with_bar(view::view(a), &a.p, &a.describe, Msg::Describe)
}

/// `--play FILE`: the game started in its emulator (as Hearth Retro would), with no window: Steam's shortcuts use it.
fn play_cli(file: &str) -> i32 {
    let path = std::path::PathBuf::from(file);
    let folder = path.parent().and_then(|p| p.file_name()).map(|f| f.to_string_lossy().to_lowercase()).unwrap_or_default();
    let Some(sys) = library::SYSTEMS.iter().find(|s| s.folders.contains(&folder.as_str())) else {
        eprintln!("hearth-retro: which console is {file} for? (its folder should be a console's: snes, psx, n64…)");
        return 2;
    };
    let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let g = Game { path: path.clone(), system: sys.id, title: library::clean_title(&stem), stem, year: String::new() };
    let here = emus::look();
    let m = prefs::read();
    match emus::command(&g, sys, &here, None, prefs::look(&m, file)) {
        Ok(cmd) => {
            prefs::mark_played(file, now());
            use std::os::unix::process::CommandExt;
            let e = std::process::Command::new(&cmd[0]).args(&cmd[1..]).exec();
            eprintln!("hearth-retro: {}: {e}", cmd[0]);
            1
        }
        Err(e) => {
            eprintln!("hearth-retro: {e}");
            3
        }
    }
}

fn main() -> iced::Result {
    if app::handle_cli(
        "hearth-retro-app",
        "Hearth Retro: your emulator games as shelves (RetroDECK and EmuDeck).\n  --page shelves|setup|bios|pads|search   --game-mode   --pick TITLE   --show saves|options|where|why\n  --play FILE   start that game in its emulator, no window",
    ) {
        return Ok(());
    }
    if let Some(f) = app::arg("--play") {
        std::process::exit(play_cli(&f));
    }
    let game_mode = app::flag("--game-mode") || std::env::var_os("GAMESCOPE_WAYLAND_DISPLAY").is_some();
    let mut w = app::window("hearth-retro", (1600.0, 900.0), (1000.0, 640.0));
    if game_mode {
        w.fullscreen = true;
    }
    iced::application(App::new, update, shown)
        .title(|_: &App| "Hearth Retro".to_string())
        .subscription(subscription)
        .theme(|a: &App| app::iced_theme(&a.p))
        .window(w)
        .default_font(app::INTER)
        .run()
}
