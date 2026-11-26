//! Retro to Steam, in pure Rust (hearth-ui, iced): the approved board docs/mockups/open-source-apps/05-retro-to-steam.png
//! (checklist docs/mockups/checklists/open-source-05-retro-to-steam.md). Built on Steam ROM Manager (GPL-3): it does
//! what Steam ROM Manager does, the same way: finds the emulator games (Hearth Retro's consoles and folders), finds art
//! for each from community art sites (art.rs), and writes them into the Steam library on this PC as non-Steam games
//! (userdata/<you>/config/shortcuts.vdf, with their art in config/grid), each starting in the emulator Hearth Retro set
//! up (hearth-retro-app --play).
//!
//! Steam's own files only: it never signs in to Steam and never uses an account or a token (red flag 6). Steam has to
//! be closed while the file is written: Hearth asks first, closes it, adds them and opens it again. The old
//! shortcuts.vdf is kept as shortcuts.vdf.bak (settings_file), and a file that can't be read is never written over.
//! What it added is remembered, so Take out what I added takes out only those. Nothing at rest: no timers.

mod art;
mod job;
mod steam;
mod vdf;
mod view;

use art::{Cand, Index, Kind};
use hearth_retro::library::{self, Game, System};
use hearth_ui::app::{self, win};
use hearth_ui::{run, settings_file, Pal};
use iced::{Element, Subscription, Task};
use serde_json::{json, Map, Value};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// What CI's pictures pretend (HEARTH_R2S_FIXTURE: each game's art counts, whether Steam is running); never on a PC.
pub fn fixture() -> Value {
    std::env::var("HEARTH_R2S_FIXTURE").ok().filter(|s| !s.is_empty()).and_then(|p| std::fs::read_to_string(p).ok()).and_then(|t| serde_json::from_str(&t).ok()).unwrap_or(Value::Null)
}

pub fn prefs_path() -> PathBuf {
    hearth_ui::paths::home().join(".config/hearth/retro-to-steam.json")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    All,
    Needs,
    New,
    InSteam,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Dlg {
    /// Steam is open: close it and add (or take out)?
    CloseSteam(bool),
    Preview(String),
    Sources,
    Said(String, String),
    Users,
    Collection,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Describe(hearth_ui::describe::DMsg),
    Theme,
    Scanned(Vec<(&'static System, Vec<Game>)>),
    Indexed(Arc<Index>),
    Sgdb(String, Vec<(Kind, Vec<String>)>),
    Fetched(String, bool),
    Console(usize),
    ToggleConsole(&'static str),
    Kind(Kind),
    Filter(Filter),
    Search(String),
    Pick(String),
    Cycle(String, Kind, i32),
    Name(String),
    Collection(String),
    Shelf(bool),
    Choose,
    Chosen(run::Output),
    RomsFolder(String),
    RomsFolderDone,
    User(String),
    Add,
    Remove,
    Preview,
    Go(bool),
    Tick,
    Done(Result<String, String>),
    Open(Dlg),
    Close,
    SgdbKey(String),
    SgdbSave,
    Drag,
    Max,
    Min,
    Quit,
    Resize(iced::window::Direction),
}

pub struct App {
    pub describe: hearth_ui::describe::Describe,
    pub p: Pal,
    pub shelves: Vec<(&'static System, Vec<Game>)>,
    pub scanned: bool,
    pub index: Option<Arc<Index>>,
    pub sgdb: HashMap<String, Vec<(Kind, Vec<String>)>>,
    pub prefs: Map<String, Value>,
    pub users: Vec<steam::User>,
    pub user: String,
    /// what's in this person's Steam already: (name lowercase, launch options)
    pub existing: Vec<(String, String)>,
    pub existing_err: String,
    pub console: usize,
    pub kind: Kind,
    pub filter: Filter,
    pub search: String,
    pub picked: String,
    pub name_edit: String,
    pub roms_edit: String,
    pub have: HashSet<PathBuf>,
    pub fetching: HashSet<String>,
    pub dlg: Option<Dlg>,
    pub busy: Option<Arc<Mutex<String>>>,
    pub busy_text: String,
    pub said: String,
    pub sgdb_key: String,
}

pub fn rom(g: &Game) -> String {
    g.path.display().to_string()
}

fn obj<'a>(m: &'a Map<String, Value>, k: &str) -> Option<&'a Map<String, Value>> {
    m.get(k).and_then(Value::as_object)
}

impl App {
    fn new() -> (App, Task<Msg>) {
        let prefs = settings_file::read(&prefs_path()).map().unwrap_or_default();
        let users = steam::users();
        let user = prefs.get("user").and_then(Value::as_str).map(String::from).filter(|u| users.iter().any(|x| &x.id == u)).or_else(|| users.first().map(|u| u.id.clone())).unwrap_or_default();
        let roms_edit = hearth_retro::prefs::read().get("romsFolder").and_then(Value::as_str).map(String::from).unwrap_or_else(|| library::tilde(&hearth_retro::retro_dir().join("roms")));
        let mut a = App {
            describe: hearth_ui::describe::Describe::new("retro-to-steam", "Retro to Steam").with_pages(view::BLOCKS).with_kit(&view::KIT),
            p: hearth_ui::theme::load(),
            shelves: Vec::new(),
            scanned: false,
            index: None,
            sgdb: HashMap::new(),
            prefs,
            users,
            user,
            existing: Vec::new(),
            existing_err: String::new(),
            console: 0,
            kind: Kind::Grid,
            filter: Filter::All,
            search: String::new(),
            picked: String::new(),
            name_edit: String::new(),
            roms_edit,
            have: HashSet::new(),
            fetching: HashSet::new(),
            dlg: None,
            busy: None,
            busy_text: String::new(),
            said: String::new(),
            sgdb_key: String::new(),
        };
        a.read_existing();
        (a, Task::perform(off(library::scan), Msg::Scanned))
    }

    pub fn read_existing(&mut self) {
        self.existing.clear();
        self.existing_err.clear();
        if self.user.is_empty() {
            return;
        }
        match steam::read_shortcuts(&self.user) {
            Ok(root) => {
                if let Some(list) = root.get("shortcuts") {
                    for (_, s) in list.entries() {
                        self.existing.push((s.str("AppName").to_lowercase(), s.str("LaunchOptions").to_string()));
                    }
                }
            }
            Err(e) => self.existing_err = e,
        }
    }

    pub fn off(&self, sys: &str) -> bool {
        self.prefs.get("off").and_then(Value::as_array).map(|a| a.iter().any(|x| x.as_str() == Some(sys))).unwrap_or(false)
    }

    pub fn name_of(&self, g: &Game) -> String {
        obj(&self.prefs, "names").and_then(|m| m.get(&rom(g))).and_then(Value::as_str).map(String::from).unwrap_or_else(|| g.title.clone())
    }

    pub fn collection_of(&self, g: &Game) -> String {
        let own = library::system(g.system).map(|s| format!("Retro · {}", s.name)).unwrap_or_else(|| "Retro".into());
        obj(&self.prefs, "collections").and_then(|m| m.get(&rom(g))).and_then(Value::as_str).map(String::from).unwrap_or(own)
    }

    pub fn shelf_of(&self, g: &Game) -> bool {
        obj(&self.prefs, "shelf").and_then(|m| m.get(&rom(g))).and_then(Value::as_bool).unwrap_or(true)
    }

    pub fn in_steam(&self, g: &Game) -> bool {
        let r = rom(g);
        let n = self.name_of(g).to_lowercase();
        self.existing.iter().any(|(name, opts)| *name == n || (!r.is_empty() && opts.contains(&r)))
    }

    pub fn cands(&self, g: &Game, k: Kind) -> Vec<Cand> {
        match &self.index {
            Some(ix) => ix.cands(g, k, self.sgdb.get(&rom(g))),
            None => vec![],
        }
    }

    /// The person's own picture for this kind, if they chose one.
    pub fn own(&self, g: &Game, k: Kind) -> Option<PathBuf> {
        obj(&self.prefs, "own").and_then(|m| m.get(&rom(g))).and_then(|x| x.get(k.id())).and_then(Value::as_str).map(PathBuf::from).filter(|p| p.is_file())
    }

    /// Which choice is picked (0 unless the person stepped through them).
    pub fn pick_of(&self, g: &Game, k: Kind) -> usize {
        obj(&self.prefs, "picks").and_then(|m| m.get(&rom(g))).and_then(|x| x.get(k.id())).and_then(Value::as_u64).unwrap_or(0) as usize
    }

    /// The picture shown for a game and kind: their own, else the picked choice once it's downloaded.
    pub fn shown(&self, g: &Game, k: Kind) -> Option<PathBuf> {
        if let Some(p) = self.own(g, k) {
            return Some(p);
        }
        let c = self.cands(g, k);
        let c = c.get(self.pick_of(g, k).min(c.len().saturating_sub(1)))?;
        let f = art::file_of(c);
        if self.have.contains(&f) || f.is_file() {
            Some(f)
        } else {
            None
        }
    }

    pub fn needs_pick(&self, g: &Game) -> bool {
        self.index.is_some() && self.own(g, Kind::Grid).is_none() && self.cands(g, Kind::Grid).is_empty()
    }

    /// Every game of the consoles that are on.
    pub fn included(&self) -> Vec<&Game> {
        self.shelves.iter().filter(|(s, _)| !self.off(s.id)).flat_map(|(_, g)| g.iter()).collect()
    }

    pub fn counts(&self) -> (usize, usize, usize, usize) {
        let inc = self.included();
        let needs = inc.iter().filter(|g| self.needs_pick(g)).count();
        let in_steam = inc.iter().filter(|g| self.in_steam(g)).count();
        (inc.len(), needs, inc.len() - in_steam, in_steam)
    }

    /// The games shown: the console's, by the filter and the search.
    pub fn shown_games(&self) -> Vec<&Game> {
        let Some((_, games)) = self.shelves.get(self.console) else { return vec![] };
        let q = self.search.to_lowercase();
        games
            .iter()
            .filter(|g| q.is_empty() || g.title.to_lowercase().contains(&q))
            .filter(|g| match self.filter {
                Filter::All => true,
                Filter::Needs => self.needs_pick(g),
                Filter::New => !self.in_steam(g),
                Filter::InSteam => self.in_steam(g),
            })
            .collect()
    }

    pub fn picked_game(&self) -> Option<&Game> {
        self.shelves.iter().flat_map(|(_, g)| g.iter()).find(|g| rom(g) == self.picked)
    }

    fn save(&mut self, f: impl FnOnce(&mut Map<String, Value>)) {
        if let Some(d) = prefs_path().parent() {
            let _ = std::fs::create_dir_all(d);
        }
        match settings_file::update_json(&prefs_path(), |m| f(m)) {
            Ok((m, _)) => self.prefs = m,
            Err(e) => self.said = format!("It couldn't be saved: {e}"),
        }
    }

    /// A game's own setting (names, collections, shelf, picks, own) changed.
    fn set_game(&mut self, key: &'static str, rom: String, v: Value) {
        self.save(move |m| {
            let mut o = m.get(key).and_then(Value::as_object).cloned().unwrap_or_default();
            o.insert(rom, v);
            m.insert(key.into(), Value::Object(o));
        });
    }
}

/// `f` on a thread of its own.
fn off<T: Send + 'static>(f: fn() -> T) -> impl std::future::Future<Output = T> + Send + 'static {
    let (tx, rx) = futures::channel::oneshot::channel();
    let _ = std::thread::Builder::new().name("hearth-r2s-read".into()).spawn(move || {
        let _ = tx.send(f());
    });
    async move { rx.await.expect("the reading thread ended") }
}

fn thread<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> impl std::future::Future<Output = T> + Send + 'static {
    let (tx, rx) = futures::channel::oneshot::channel();
    let _ = std::thread::Builder::new().name("hearth-r2s-work".into()).spawn(move || {
        let _ = tx.send(f());
    });
    async move { rx.await.expect("the working thread ended") }
}

/// The pictures in view downloaded (a few at a time): the shown games' current kind, and the picked one's every kind.
fn want_art(a: &mut App) -> Task<Msg> {
    if a.index.is_none() {
        return Task::none();
    }
    let mut want: Vec<Cand> = Vec::new();
    let games: Vec<Game> = a.shown_games().into_iter().take(18).cloned().collect();
    for g in &games {
        let c = a.cands(g, a.kind);
        if let Some(x) = c.get(a.pick_of(g, a.kind).min(c.len().saturating_sub(1))) {
            want.push(x.clone());
        }
    }
    if let Some(g) = a.picked_game().cloned() {
        for k in Kind::ALL {
            let c = a.cands(&g, k);
            if let Some(x) = c.get(a.pick_of(&g, k).min(c.len().saturating_sub(1))) {
                want.push(x.clone());
            }
        }
    }
    let mut tasks = Vec::new();
    for c in want {
        if c.site == "fixture" || a.fetching.len() >= 6 {
            continue;
        }
        let f = art::file_of(&c);
        if a.have.contains(&f) || a.fetching.contains(&c.url) {
            continue;
        }
        if f.is_file() {
            a.have.insert(f);
            continue;
        }
        a.fetching.insert(c.url.clone());
        let url = c.url.clone();
        tasks.push(Task::perform(thread(move || art::fetch(&c).is_ok()), move |ok| Msg::Fetched(url.clone(), ok)));
    }
    Task::batch(tasks)
}

fn pick_game(a: &mut App, r: String) -> Task<Msg> {
    a.picked = r;
    a.name_edit = a.picked_game().map(|g| a.name_of(g)).unwrap_or_default();
    a.dlg = None;
    let mut t = vec![want_art(a)];
    // SteamGridDB's choices for it, with your own key
    let key = a.prefs.get("sgdbKey").and_then(Value::as_str).unwrap_or("").to_string();
    if let Some(g) = a.picked_game().cloned() {
        if !key.is_empty() && !a.sgdb.contains_key(&rom(&g)) && fixture().is_null() {
            let (title, r) = (g.title.clone(), rom(&g));
            t.push(Task::perform(thread(move || art::sgdb(&title, &key)), move |v| Msg::Sgdb(r.clone(), v)));
        }
    }
    Task::batch(t)
}

fn start_job(a: &mut App, remove: bool) -> Task<Msg> {
    if a.user.is_empty() {
        a.dlg = Some(Dlg::Said("No Steam here yet".into(), "Retro to Steam writes into Steam's own files on this PC: open Steam once (sign in there, not here), then come back.".into()));
        return Task::none();
    }
    if !a.existing_err.is_empty() {
        a.dlg = Some(Dlg::Said("Steam's list couldn't be read".into(), a.existing_err.clone()));
        return Task::none();
    }
    let plan = job::plan(a, remove);
    let progress = Arc::new(Mutex::new(String::from("Starting…")));
    a.busy = Some(progress.clone());
    a.busy_text = "Starting…".into();
    a.dlg = None;
    Task::perform(thread(move || job::run(plan, progress)), Msg::Done)
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
            sort_shelves(&mut s);
            a.shelves = s;
            a.scanned = true;
            a.console = a.console.min(a.shelves.len().saturating_sub(1));
            let want = app::arg("--pick");
            let first = a.shelves.first().and_then(|(_, g)| g.first()).map(rom).unwrap_or_default();
            let r = want.and_then(|t| a.shelves.iter().flat_map(|(_, g)| g.iter()).find(|g| g.title == t).map(rom)).unwrap_or(first);
            if let Some(i) = a.shelves.iter().position(|(_, g)| g.iter().any(|x| rom(x) == r)) {
                a.console = i;
            }
            let systems: Vec<String> = a.shelves.iter().map(|(s, _)| s.id.to_string()).collect();
            let t = pick_game(a, r);
            match app::arg("--show").as_deref() {
                Some("close-steam") => a.dlg = Some(Dlg::CloseSteam(false)),
                Some("preview") => {
                    let plan = job::plan(a, false);
                    a.dlg = Some(Dlg::Preview(job::describe(&plan)));
                }
                Some("sources") => a.dlg = Some(Dlg::Sources),
                _ => {}
            }
            return Task::batch([t, Task::perform(thread(move || art::index(systems)), |ix| Msg::Indexed(Arc::new(ix)))]);
        }
        Msg::Indexed(ix) => {
            a.index = Some(ix);
            if let Some(Dlg::Preview(_)) = a.dlg {
                let plan = job::plan(a, false);
                a.dlg = Some(Dlg::Preview(job::describe(&plan)));
            }
            return want_art(a);
        }
        Msg::Sgdb(r, v) => {
            a.sgdb.insert(r, v);
            return want_art(a);
        }
        Msg::Fetched(url, ok) => {
            a.fetching.remove(&url);
            if ok {
                a.have.insert(art::file_of(&Cand { url, site: "" }));
            }
            return want_art(a);
        }
        Msg::Console(i) => {
            a.console = i;
            if let Some(g) = a.shelves.get(i).and_then(|(_, g)| g.first()).map(rom) {
                return pick_game(a, g);
            }
        }
        Msg::ToggleConsole(id) => {
            let on = a.off(id);
            a.save(move |m| {
                let mut v: Vec<Value> = m.get("off").and_then(Value::as_array).cloned().unwrap_or_default();
                v.retain(|x| x.as_str() != Some(id));
                if !on {
                    v.push(Value::String(id.into()));
                }
                m.insert("off".into(), Value::Array(v));
            });
        }
        Msg::Kind(k) => {
            a.kind = k;
            return want_art(a);
        }
        Msg::Filter(f) => {
            a.filter = f;
            return want_art(a);
        }
        Msg::Search(s) => {
            a.search = s;
            return want_art(a);
        }
        Msg::Pick(r) => return pick_game(a, r),
        Msg::Cycle(r, k, d) => {
            if let Some(g) = a.shelves.iter().flat_map(|(_, g)| g.iter()).find(|g| rom(g) == r).cloned() {
                let n = a.cands(&g, k).len();
                if n > 0 {
                    let i = (a.pick_of(&g, k) as i32 + d).rem_euclid(n as i32) as u64;
                    let rr = r.clone();
                    a.save(move |m| {
                        let mut o = m.get("picks").and_then(Value::as_object).cloned().unwrap_or_default();
                        let mut g = o.get(&rr).and_then(Value::as_object).cloned().unwrap_or_default();
                        g.insert(k.id().into(), json!(i));
                        o.insert(rr, Value::Object(g));
                        m.insert("picks".into(), Value::Object(o));
                    });
                }
                a.picked = r;
                a.name_edit = a.name_of(&g);
                return want_art(a);
            }
        }
        Msg::Name(s) => {
            a.name_edit = s.clone();
            if !a.picked.is_empty() {
                let r = a.picked.clone();
                a.set_game("names", r, Value::String(s));
            }
        }
        Msg::Collection(c) => {
            a.dlg = None;
            if !a.picked.is_empty() {
                let r = a.picked.clone();
                a.set_game("collections", r, Value::String(c));
            }
        }
        Msg::Shelf(on) => {
            if !a.picked.is_empty() {
                let r = a.picked.clone();
                a.set_game("shelf", r, Value::Bool(on));
            }
        }
        Msg::Choose => {
            let cmd = "for c in 'hearth-file-chooser --open' 'kdialog --getopenfilename . image/png image/jpeg image/webp' 'zenity --file-selection'; do set -- $c; command -v \"$1\" >/dev/null 2>&1 && exec \"$@\"; done; exit 3";
            return Task::perform(run::spawn(run::args(&["sh", "-c", cmd])), Msg::Chosen);
        }
        Msg::Chosen(o) => {
            let f = o.out.lines().last().unwrap_or("").trim().to_string();
            if o.ok && !f.is_empty() && std::path::Path::new(&f).is_file() && !a.picked.is_empty() {
                let (r, k) = (a.picked.clone(), a.kind);
                a.save(move |m| {
                    let mut o = m.get("own").and_then(Value::as_object).cloned().unwrap_or_default();
                    let mut g = o.get(&r).and_then(Value::as_object).cloned().unwrap_or_default();
                    g.insert(k.id().into(), Value::String(f));
                    o.insert(r, Value::Object(g));
                    m.insert("own".into(), Value::Object(o));
                });
            } else if o.code == 3 {
                a.said = "No file chooser here to pick a picture with.".into();
            }
        }
        Msg::RomsFolder(s) => a.roms_edit = s,
        Msg::RomsFolderDone => {
            let v = a.roms_edit.trim().to_string();
            if let Err(e) = hearth_retro::prefs::change(move |m| {
                m.insert("romsFolder".into(), Value::String(v));
            }) {
                a.said = format!("It couldn't be saved: {e}");
            }
            a.said = "Reading the games again…".into();
            return Task::perform(off(library::scan), Msg::Scanned);
        }
        Msg::User(u) => {
            a.user = u.clone();
            a.dlg = None;
            a.save(move |m| {
                m.insert("user".into(), Value::String(u));
            });
            a.read_existing();
        }
        Msg::Add | Msg::Remove => {
            let remove = matches!(m, Msg::Remove);
            if a.busy.is_some() {
                return Task::none();
            }
            if steam::running() {
                a.dlg = Some(Dlg::CloseSteam(remove));
                return Task::none();
            }
            return start_job(a, remove);
        }
        Msg::Go(remove) => return start_job(a, remove),
        Msg::Preview => {
            let plan = job::plan(a, false);
            a.dlg = Some(Dlg::Preview(job::describe(&plan)));
        }
        Msg::Tick => {
            if let Some(b) = &a.busy {
                if let Ok(s) = b.lock() {
                    a.busy_text = s.clone();
                }
            }
        }
        Msg::Done(r) => {
            a.busy = None;
            a.busy_text.clear();
            a.prefs = settings_file::read(&prefs_path()).map().unwrap_or_default();
            a.read_existing();
            a.dlg = Some(match r {
                Ok(s) => Dlg::Said("Done".into(), s),
                Err(e) => Dlg::Said("Nothing was changed".into(), e),
            });
        }
        Msg::Open(d) => a.dlg = Some(d),
        Msg::Close => a.dlg = None,
        Msg::SgdbKey(s) => a.sgdb_key = s,
        Msg::SgdbSave => {
            let k = a.sgdb_key.trim().to_string();
            a.sgdb_key.clear();
            a.save(move |m| {
                if k.is_empty() {
                    m.remove("sgdbKey");
                } else {
                    m.insert("sgdbKey".into(), Value::String(k));
                }
            });
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(prefs_path(), std::fs::Permissions::from_mode(0o600));
            }
            a.sgdb.clear();
            a.dlg = None;
            let r = a.picked.clone();
            return pick_game(a, r);
        }
        Msg::Drag => return win::drag(),
        Msg::Max => return win::toggle_maximize(),
        Msg::Min => return win::minimize(),
        Msg::Quit => return win::close(),
        Msg::Resize(d) => return win::resize(d),
    }
    Task::none()
}

/// Each console's games as Hearth Retro's shelves have them: the most recently played first, then by name.
fn sort_shelves(s: &mut [(&'static System, Vec<Game>)]) {
    let played = hearth_retro::prefs::played();
    for (_, games) in s.iter_mut() {
        games.sort_by_key(|g| (std::cmp::Reverse(played.get(&rom(g)).and_then(|v| v.get("last")).and_then(Value::as_i64).unwrap_or(0)), g.title.to_lowercase()));
    }
}

/// `--add`: every new game added to Steam without the window (Steam must be closed; it isn't closed for you here).
fn cli_add() -> i32 {
    if steam::running() {
        eprintln!("retro-to-steam: Steam is running: close it first");
        return 4;
    }
    let (mut a, _) = App::new();
    let mut s = library::scan();
    sort_shelves(&mut s);
    let systems: Vec<String> = s.iter().map(|(x, _)| x.id.to_string()).collect();
    a.shelves = s;
    a.index = Some(Arc::new(art::index(systems)));
    let plan = job::plan(&a, app::flag("--remove"));
    match job::run(plan, Arc::new(Mutex::new(String::new()))) {
        Ok(s) => {
            println!("{s}");
            0
        }
        Err(e) => {
            eprintln!("retro-to-steam: {e}");
            1
        }
    }
}

fn subscription(a: &App) -> Subscription<Msg> {
    let mut s = vec![Subscription::run(hearth_ui::theme::watch).map(|_| Msg::Theme)];
    // only while adding: what it's doing, twice a second
    if a.busy.is_some() {
        s.push(iced::time::every(std::time::Duration::from_millis(500)).map(|_| Msg::Tick));
    }
    Subscription::batch(s)
}

fn shown(a: &App) -> Element<'_, Msg> {
    hearth_ui::describe::with_bar(view::view(a), &a.p, &a.describe, Msg::Describe)
}

fn main() -> iced::Result {
    if app::handle_cli("hearth-retro-to-steam-app", "Retro to Steam: your emulator games in the Steam library, with art (Steam ROM Manager).\n  --pick TITLE   --show close-steam|preview|sources\n  --add [--remove]   add every new game (or take out what was added) with no window; Steam must be closed") {
        return Ok(());
    }
    if app::flag("--add") {
        std::process::exit(cli_add());
    }
    iced::application(App::new, update, shown)
        .title(|_: &App| "Retro to Steam".to_string())
        .subscription(subscription)
        .theme(|a: &App| app::iced_theme(&a.p))
        .window(app::window("hearth-retro-to-steam", (1600.0, 920.0), (1100.0, 640.0)))
        .default_font(app::INTER)
        .run()
}
