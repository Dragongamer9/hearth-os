//! The Hearth Store (hearth-store-app), Hearth OS 1.16, Fedora edition: the owner's own apps that aren't part of
//! Hearth OS, installed as AppImages in your home folder, updated, checked and repaired, uninstalled; the apps built
//! into Hearth shown as "Built in". Built exactly to the approved mockups (docs/mockups/hearth-store, checklist
//! docs/mockups/checklists/hearth-store.md) and the plan in docs/HEARTH-STORE.md.
//!
//! Light at rest: nothing polls. It looks for updates when it opens (at most every 15 minutes) and the daily check is a
//! separate small run from a user timer (`--check`, hearth-store-check.timer). A download's progress is read only while
//! it runs; the controller look's clock wakes once a minute. Opening it writes none of your settings.
//!
//! Mouse, keyboard and controller: arrows / the D-pad move the focus, Enter / A presses, Esc / B goes back, Ctrl+F / Y
//! searches (on an app or update: what's new), X updates all, Ctrl+Page Up / Down and LB / RB change pages, ☰ opens
//! an app's ⋯ menu.

mod look;
mod pad;
mod view;
mod w11;

use hearth_store::catalog::{self, Catalog, Entry, Kind};
use hearth_store::check;
use hearth_store::files::{self, Check, Installed};
use hearth_store::net::{self, Cancel, Fail};
use hearth_store::parts;
use hearth_store::prefs::{self, Prefs};
use hearth_store::release::{self, Rel, Seen};
use hearth_store::{newer, paths};
use hearth_ui::app::{self, win};
use hearth_ui::run;
use hearth_ui::Pal;
use iced::keyboard::{key::Named, Event as KEvent, Key};
use iced::widget::{image, operation};
use iced::{Subscription, Task};
use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

pub const SEARCH_ID: &str = "store-search";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Page {
    Home,
    /// The Windows 11 Store's rail (store-win11): every app with filter pills, and the test versions.
    Apps,
    Tests,
    Yours,
    Updates,
    BuiltIn,
    Settings,
    App(String),
    Work,
    Repair(String),
}

/// The pages in the sidebar (and LB / RB's order).
pub const PAGES: [Page; 5] = [Page::Home, Page::Yours, Page::Updates, Page::BuiltIn, Page::Settings];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobKind {
    Install,
    Update,
    Repair,
}

/// An install, update or repair that's running (or just ended).
#[derive(Debug, Clone)]
pub struct Job {
    pub id: String,
    pub kind: JobKind,
    /// 0 download, 1 check, 2 close the app, 3 put it in place, 4 the menu entry (and open it again), 5 done.
    pub step: u8,
    pub got: u64,
    pub total: u64,
    pub t0: Instant,
    pub from: String,
    pub rel: Rel,
    pub cancel: Cancel,
    pub error: String,
    pub was_running: bool,
    /// The old version was kept (.previous) until the new one has started.
    pub kept: bool,
    pub details: bool,
    /// CI's still picture of a download under way (--demo-progress, only with HEARTH_STORE_FIXTURE).
    pub demo: Option<f32>,
}

impl Job {
    pub fn done(&self) -> bool {
        self.step >= 5
    }
    /// Cancel works until the new version is put in place.
    pub fn can_cancel(&self) -> bool {
        self.step < 3 && self.error.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct Repair {
    pub id: String,
    pub check: Option<Check>,
    pub fixing: bool,
    pub said: String,
    pub got: u64,
    pub cancel: Cancel,
    pub every_file: bool,
}

#[derive(Debug, Clone)]
pub struct Uninst {
    pub id: String,
    pub keep: bool,
    pub size: Option<u64>,
    pub focus: u8,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Theme,
    Key(KEvent),
    Pad(pad::Button),
    Drag,
    Maximize,
    Minimize,
    Close,
    Resize(iced::window::Direction),
    Go(Page),
    Back,
    Focus(String),
    OpenApp(String),
    Launch(String),
    Start(String, JobKind),
    Metered(String, JobKind, bool),
    UpdateAll,
    Cancel,
    Downloaded(Result<(), Fail>),
    Tick,
    Verified(Result<(), String>),
    Closed(bool),
    Placed(Result<bool, String>),
    Started(bool),
    RepairOpen(String),
    RepairChecked(Check),
    RepairNow,
    Repaired(Result<String, String>),
    EveryFile,
    AskUninstall(String),
    DataSize(u64),
    DlgKeep,
    DlgCancel,
    DlgUninstall,
    Uninstalled(String, Result<usize, String>),
    Menu,
    ShowFiles(String),
    ReleasePage(String),
    ToggleNew(String),
    Search(String),
    SearchFocus,
    SetDaily(bool),
    SetFlag(&'static str, bool),
    OpenFlathub,
    OpenFolder,
    OpenHearthUpdates,
    TryAgain,
    Checked(Box<Seen>, bool),
    Reconciled(String, Option<Installed>),
    Pictures(String, Vec<(String, String)>),
    Clock,
    Details,
    /// Quick updates (parts.rs): update, back to the built-in one, try a copy that failed again, and when it's done.
    PartUpdate(String),
    /// Get a test version (Mantel in Rust): it isn't an update to anything built in.
    PartGet(String),
    PartRemove(String),
    PartRetry(String),
    PartDone(String, Result<String, String>),
    /// Apps' filter pills, Library's Apps / Hearth parts / Tests (store-win11)
    Filter(String),
    LibTab(String),
}

pub struct App {
    pub p: Pal,
    pub cat: Catalog,
    pub seen: Seen,
    pub prefs: Prefs,
    pub inst: BTreeMap<String, Installed>,
    pub checking: bool,
    pub page: Page,
    pub trail: Vec<Page>,
    pub focus: String,
    pub menu: bool,
    pub menu_at: usize,
    pub dlg: Option<Uninst>,
    pub shown_new: BTreeSet<String>,
    pub hidden_new: BTreeSet<String>,
    pub job: Option<Job>,
    pub repair: Option<Repair>,
    pub search: String,
    pub pad: bool,
    pub pad_fixed: bool,
    pub note: String,
    pub note_bad: bool,
    pub hearth: String,
    pub edition: String,
    pub now: i64,
    /// Pictures on this PC (cache): url → its picture.
    pub pics: BTreeMap<String, image::Handle>,
    /// Built-in apps that are here (their program exists).
    pub here: BTreeSet<String>,
    /// Update all is going through the updates one after another.
    pub all: bool,
    /// Quick updates installed in your home folder (by the entry's id), and the ones being worked on.
    pub parts: BTreeMap<String, parts::Have>,
    pub part_busy: BTreeSet<String>,
    /// Apps' filter ("" all) and Library's list ("" apps), store-win11
    pub filter: String,
    pub lib_tab: String,
}

/// Where an app is: what its buttons say and do.
#[derive(Debug, Clone, PartialEq)]
pub enum State {
    BuiltIn,
    Coming,
    /// Not installed, a release to install (or offline: what was seen).
    Get(Rel),
    UpToDate(Installed),
    /// Installed and a newer release (Update; for an app that updates itself: Open to update).
    Ready(Installed, Rel),
}

fn this_pc() -> (String, String) {
    if let Some(f) = paths::fixture() {
        if let Ok(v) = std::fs::read(f.join("this-pc.json")).map(|b| serde_json::from_slice::<serde_json::Value>(&b).unwrap_or_default()) {
            let s = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
            return (s("hearth"), s("edition"));
        }
    }
    let info: serde_json::Value = std::fs::read("/usr/share/nimbus/image-info.json").ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
    let ver = std::fs::read_to_string("/usr/share/nimbus/version").map(|s| s.trim().to_string()).unwrap_or_default();
    let ver = if ver.is_empty() { info.get("version").and_then(|x| x.as_str()).unwrap_or("").to_string() } else { ver };
    let base = info.get("base").and_then(|x| x.as_str()).unwrap_or("");
    let edition = if base == "anatase" || std::path::Path::new("/usr/bin/rpm-ostree").exists() || std::path::Path::new("/usr/bin/dnf5").exists() { "Fedora edition" } else { "" };
    (hearth_store::short_version(&ver), edition.to_string())
}

fn which(cmd: &str) -> bool {
    let first = cmd.split_whitespace().next().unwrap_or("");
    if first.is_empty() {
        return false;
    }
    if first.contains('/') {
        return std::path::Path::new(first).exists();
    }
    std::env::var_os("PATH").map(|p| std::env::split_paths(&p).any(|d| d.join(first).is_file())).unwrap_or(false)
}

fn page_of(s: &str) -> Page {
    match s {
        "yours" | "your-apps" => Page::Yours,
        "updates" | "library" => Page::Updates,
        "apps" => Page::Apps,
        "tests" => Page::Tests,
        "built-in" | "builtin" => Page::BuiltIn,
        "settings" => Page::Settings,
        "work" | "updating" => Page::Work,
        x if x.starts_with("app:") => Page::App(x[4..].to_string()),
        x if x.starts_with("repair:") => Page::Repair(x[7..].to_string()),
        _ => Page::Home,
    }
}

impl App {
    fn new() -> (App, Task<Msg>) {
        let cat = catalog::load();
        let (hearth, edition) = this_pc();
        // Mantel has no program to open: it's here when its desktop is (Quickshell and its shell).
        let here = cat.apps.iter().filter(|e| e.kind == Kind::BuiltIn && (which(&e.open) || (e.id == "mantel" && which("quickshell")))).map(|e| e.id.clone()).collect();
        let mut a = App {
            p: hearth_ui::theme::load(),
            seen: release::read_seen(),
            prefs: prefs::load(),
            inst: BTreeMap::new(),
            checking: false,
            page: Page::Home,
            trail: Vec::new(),
            focus: String::new(),
            menu: false,
            menu_at: 0,
            dlg: None,
            shown_new: BTreeSet::new(),
            hidden_new: BTreeSet::new(),
            job: None,
            repair: None,
            search: String::new(),
            pad: app::flag("--controller"),
            pad_fixed: app::flag("--controller") || app::flag("--mouse"),
            note: String::new(),
            note_bad: false,
            hearth,
            edition,
            now: files::now(),
            pics: BTreeMap::new(),
            here,
            all: false,
            parts: BTreeMap::new(),
            part_busy: BTreeSet::new(),
            filter: String::new(),
            lib_tab: String::new(),
            cat,
        };
        pad::PAD_LOOK.store(a.pad, std::sync::atomic::Ordering::Relaxed);
        a.read_installed();
        let mut tasks = vec![a.reconcile()];
        if let Some(pg) = app::arg("--page") {
            let pg = page_of(&pg);
            tasks.push(a.go(pg));
        } else {
            a.focus = view::first_target(&a);
        }
        // The check when the Store opens: at most every 15 minutes (what was seen is shown meanwhile).
        if a.now - a.seen.checked > 15 * 60 || paths::fixture().is_some() {
            tasks.push(a.check_now());
        }
        if let Some(id) = app::arg("--app") {
            tasks.push(a.go(Page::App(id)));
        }
        match app::arg("--show").as_deref() {
            Some("menu") => {
                a.menu = true;
                a.menu_at = 0;
            }
            Some(x) if x.starts_with("uninstall:") => tasks.push(update(&mut a, Msg::AskUninstall(x[10..].to_string()))),
            _ => {}
        }
        // CI's picture of a download under way (only with the pretend world).
        if let (Some(id), Some(pct)) = (app::arg("--demo-progress-app"), app::arg("--demo-progress")) {
            if paths::fixture().is_some() {
                a.demo_job(&id, pct.parse().unwrap_or(62.0) / 100.0);
            }
        }
        if let Some(fx) = app::arg("--focus") {
            a.focus = fx;
        }
        if app::flag("--expand-all") {
            for e in &a.cat.apps {
                a.shown_new.insert(e.id.clone());
            }
        }
        (a, Task::batch(tasks))
    }

    pub fn entry(&self, id: &str) -> Option<&Entry> {
        self.cat.apps.iter().find(|e| e.id == id)
    }

    pub fn offline(&self) -> bool {
        self.seen.offline
    }

    pub fn state(&self, e: &Entry) -> State {
        match e.kind {
            Kind::BuiltIn => State::BuiltIn,
            Kind::Coming => State::Coming,
            Kind::Install => {
                let rel = self.seen.rel(&e.id);
                match (self.inst.get(&e.id), rel) {
                    (Some(i), Some(r)) if newer(&r.version, &i.version) => State::Ready(i.clone(), r),
                    (Some(i), _) => State::UpToDate(i.clone()),
                    (None, Some(r)) => State::Get(r),
                    (None, None) => State::Coming,
                }
            }
        }
    }

    /// Updates ready (the sidebar's count, Update all).
    pub fn ready(&self) -> Vec<(Entry, Installed, Rel)> {
        self.cat.apps.iter().filter_map(|e| match self.state(e) {
            State::Ready(i, r) => Some((e.clone(), i, r)),
            _ => None,
        }).collect()
    }

    /// A quick update to offer for this entry (made for this Hearth OS, newer than what runs).
    pub fn part_offer(&self, e: &Entry) -> Option<catalog::Part> {
        e.part.as_ref().and_then(|p| if e.test { parts::test_offer(p, self.parts.get(&e.id), &self.hearth).cloned() } else { parts::offer(p, self.parts.get(&e.id), &self.hearth).cloned() })
    }

    /// Every quick update ready (Updates, Update all, the sidebar's count).
    pub fn part_offers(&self) -> Vec<(Entry, catalog::Part)> {
        self.cat.apps.iter().filter_map(|e| self.part_offer(e).map(|p| (e.clone(), p))).collect()
    }

    fn read_installed(&mut self) {
        self.parts.clear();
        for e in self.cat.apps.iter() {
            if let Some(h) = e.part.as_ref().and_then(|p| parts::have(&p.id)) {
                self.parts.insert(e.id.clone(), h);
            }
        }
        self.inst.clear();
        for e in self.cat.apps.iter().filter(|e| e.kind == Kind::Install) {
            if let Some(i) = files::read_installed(&e.id) {
                self.inst.insert(e.id.clone(), i);
            }
        }
    }

    /// Apps that update themselves: when their file changed, which version is it now? (Read once, off the window's
    /// thread, and recorded, so the Store just shows the new version.) Also: the version kept from before an update
    /// goes once the new one is running.
    fn reconcile(&self) -> Task<Msg> {
        let mut tasks = Vec::new();
        for e in self.cat.apps.iter().filter(|e| e.kind == Kind::Install) {
            let Some(i) = self.inst.get(&e.id).cloned() else { continue };
            if files::previous_file(e).exists() {
                let e2 = e.clone();
                tasks.push(Task::perform(net::off(move || {
                    if !files::running(&e2).is_empty() {
                        files::drop_previous(&e2);
                    }
                }), |_| Msg::Tick));
            }
            if !files::changed_since(e, &i) {
                continue;
            }
            let (e2, rel) = (e.clone(), self.seen.rel(&e.id));
            let id = e.id.clone();
            tasks.push(Task::perform(
                net::off(move || {
                    let h = files::sha256_file(&files::app_file(&e2)).ok()?;
                    match rel {
                        Some(r) if r.sha256.eq_ignore_ascii_case(&h) => files::record(&e2, &r.version, &h, r.size, &r.url, &r.page).ok(),
                        _ => None,
                    }
                }),
                move |r| Msg::Reconciled(id.clone(), r.flatten()),
            ));
        }
        Task::batch(tasks)
    }

    fn check_now(&mut self) -> Task<Msg> {
        if self.checking {
            return Task::none();
        }
        self.checking = true;
        let cat = self.cat.clone();
        let test = self.prefs.test;
        let ago = paths::fixture().and_then(|f| std::fs::read_to_string(f.join("checked-ago")).ok()).and_then(|s| s.trim().parse::<i64>().ok());
        Task::perform(
            net::off(move || {
                let refreshed = check::refresh_catalog(&cat);
                let cat = if refreshed { catalog::load() } else { cat };
                let mut s = check::check_all(&cat, test);
                if let (Some(a), false) = (ago, s.offline) {
                    s.checked = files::now() - a;
                }
                let _ = release::write_seen(&s);
                (s, refreshed)
            }),
            |r| match r {
                Some((s, refreshed)) => Msg::Checked(Box::new(s), refreshed),
                None => Msg::Checked(Box::new(release::read_seen()), false),
            },
        )
    }

    pub fn go(&mut self, pg: Page) -> Task<Msg> {
        self.menu = false;
        if self.page != pg && !matches!(self.page, Page::Work) {
            self.trail.push(self.page.clone());
            if self.trail.len() > 20 {
                self.trail.remove(0);
            }
        }
        if PAGES.contains(&pg) {
            self.trail.clear();
        }
        self.page = pg.clone();
        self.note.clear();
        self.focus = view::first_target(self);
        match pg {
            Page::App(id) => self.load_pictures(&id),
            Page::Repair(id) => update(self, Msg::RepairOpen(id)),
            _ => Task::none(),
        }
    }

    fn load_pictures(&mut self, id: &str) -> Task<Msg> {
        let Some(e) = self.entry(id).cloned() else { return Task::none() };
        if e.pictures.is_empty() {
            return Task::none();
        }
        let offline = self.offline();
        let id = id.to_string();
        Task::perform(
            net::off(move || {
                let dir = paths::pictures();
                let _ = std::fs::create_dir_all(&dir);
                let mut out = Vec::new();
                for (n, url) in e.pictures.iter().enumerate().take(3) {
                    let ext = if url.ends_with(".jpg") || url.ends_with(".jpeg") { "jpg" } else if url.ends_with(".webp") { "webp" } else { "png" };
                    let p = dir.join(format!("{}-{}.{ext}", e.id, n + 1));
                    if !p.exists() && !offline {
                        if let Ok(b) = net::fetch(url, 8_000_000, 30) {
                            let _ = files::write_atomic(&p, &b);
                        }
                    }
                    if p.exists() {
                        out.push((url.clone(), p.to_string_lossy().to_string()));
                    }
                }
                out
            }),
            move |r| Msg::Pictures(id.clone(), r.unwrap_or_default()),
        )
    }

    fn back(&mut self) -> Task<Msg> {
        if self.menu {
            self.menu = false;
            return Task::none();
        }
        if self.dlg.is_some() {
            self.dlg = None;
            return Task::none();
        }
        let to = match &self.page {
            Page::Work => self.job.as_ref().map(|j| if j.kind == JobKind::Update { Page::Updates } else { Page::App(j.id.clone()) }).unwrap_or(Page::Home),
            Page::Repair(id) => Page::App(id.clone()),
            Page::App(id) => self.trail.pop().unwrap_or(if self.inst.contains_key(id) { Page::Yours } else { Page::Home }),
            _ => self.trail.pop().unwrap_or(Page::Home),
        };
        let keep = self.trail.clone();
        let t = self.go(to);
        self.trail = keep;
        t
    }

    fn say(&mut self, s: impl Into<String>, bad: bool) {
        self.note = s.into();
        self.note_bad = bad;
    }

    fn demo_job(&mut self, id: &str, frac: f32) {
        let Some(e) = self.entry(id).cloned() else { return };
        let rel = self.seen.rel(id).unwrap_or_default();
        let total = rel.size.max(1);
        let from = self.inst.get(id).map(|i| i.version.clone()).unwrap_or_default();
        self.job = Some(Job {
            id: e.id.clone(),
            kind: if from.is_empty() { JobKind::Install } else { JobKind::Update },
            step: 0,
            got: (total as f32 * frac) as u64,
            total,
            t0: Instant::now(),
            from,
            rel,
            cancel: Cancel::default(),
            error: String::new(),
            was_running: false,
            kept: false,
            details: false,
            demo: Some(frac),
        });
        self.trail = vec![Page::Updates];
        self.page = Page::Work;
        self.focus = "cancel".into();
    }

    // ------------------------------------------------------------------ install / update
    fn start(&mut self, id: &str, kind: JobKind) -> Task<Msg> {
        if self.job.as_ref().map(|j| !j.done() && j.error.is_empty()).unwrap_or(false) {
            self.say("Another app is installing; this one can start when it's done.", true);
            return Task::none();
        }
        let Some(e) = self.entry(id).cloned() else { return Task::none() };
        let Some(rel) = self.seen.rel(id) else {
            self.say(format!("{} has no release to install yet.", e.name), true);
            return Task::none();
        };
        if self.offline() {
            self.say("You're offline: installing and updates wait until you're back online.", true);
            return Task::none();
        }
        if rel.sha256.len() != 64 {
            self.say(format!("{} {}'s release has no SHA-256 list, so the Store won't install it.", e.name, rel.version), true);
            return Task::none();
        }
        let from = self.inst.get(id).map(|i| i.version.clone()).unwrap_or_default();
        self.job = Some(Job {
            id: id.to_string(),
            kind,
            step: 0,
            got: 0,
            total: rel.size,
            t0: Instant::now(),
            from,
            rel: rel.clone(),
            cancel: Cancel::default(),
            error: String::new(),
            was_running: false,
            kept: false,
            details: false,
            demo: None,
        });
        if self.page != Page::Work {
            self.trail.push(self.page.clone());
        }
        self.page = Page::Work;
        self.focus = "cancel".into();
        let c = self.job.as_ref().map(|j| j.cancel.clone()).unwrap_or_default();
        let dest = files::part_file(&e);
        let url = rel.url.clone();
        Task::batch([Task::perform(net::off(move || net::download(&url, &dest, &c)), |r| Msg::Downloaded(r.unwrap_or(Err(Fail::Other("it stopped".into()))))), tick()])
    }

    fn job_entry(&self) -> Option<(Job, Entry)> {
        let j = self.job.clone()?;
        let e = self.entry(&j.id)?.clone();
        Some((j, e))
    }

    fn job_fail(&mut self, why: String) {
        self.all = false;
        if let Some(j) = self.job.as_mut() {
            j.error = why;
        }
        self.focus = "back".into();
    }
}

fn tick() -> Task<Msg> {
    Task::perform(run::after(400), |_| Msg::Tick)
}

fn clock() -> Task<Msg> {
    let s = (hearth_ui::time::now().rem_euclid(60)) as u64;
    Task::perform(run::after((60 - s) * 1000 + 50), |_| Msg::Clock)
}

fn detached(cmd: &str) {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    run::detached(&parts);
}

pub fn update(a: &mut App, m: Msg) -> Task<Msg> {
    match m {
        Msg::Theme => a.p = hearth_ui::theme::load(),
        Msg::Key(ev) => return key(a, ev),
        Msg::Pad(b) => return pad_press(a, b),
        Msg::Drag => return win::drag(),
        Msg::Maximize => return win::toggle_maximize(),
        Msg::Minimize => return win::minimize(),
        Msg::Close => return win::close(),
        Msg::Resize(d) => return win::resize(d),
        Msg::Go(pg) => return a.go(pg),
        Msg::Back => return a.back(),
        Msg::Focus(id) => a.focus = id,
        Msg::OpenApp(id) => return a.go(Page::App(id)),
        Msg::Launch(id) => {
            let Some(e) = a.entry(&id).cloned() else { return Task::none() };
            if e.kind == Kind::BuiltIn {
                if e.id == "mantel" {
                    a.say("Mantel is Hearth's own desktop: Settings › Your desktop picks it.", false);
                } else if which(&e.open) {
                    detached(&e.open);
                } else {
                    a.say(format!("{} isn't on this PC.", e.name), true);
                }
            } else if a.inst.contains_key(&id) {
                if !files::start(&e) {
                    a.say(format!("{} couldn't start. Check and repair can put it right.", e.name), true);
                }
            }
        }
        Msg::PartUpdate(id) => {
            let Some(e) = a.entry(&id).cloned() else { return Task::none() };
            let Some(pt) = a.part_offer(&e) else { return Task::none() };
            if a.offline() || !a.part_busy.insert(id.clone()) {
                return Task::none();
            }
            a.say(format!("Updating {} to {}…", e.name, pt.version), false);
            let done = if pt.id == "mantel" {
                format!("{} is on {}: your desktop restarts with it in a moment.", e.name, pt.version)
            } else if pt.id == "hearth-store-app" {
                format!("The Hearth Store is on {}: close it and open it again to use it.", pt.version)
            } else {
                format!("{} is on {}: it opens with it next time.", e.name, pt.version)
            };
            return Task::perform(net::off(move || parts::install(&pt).map(|_| done)), move |r| Msg::PartDone(id.clone(), r.unwrap_or_else(|| Err("The update stopped; nothing changed.".into()))));
        }
        Msg::PartGet(id) => {
            let Some(e) = a.entry(&id).cloned() else { return Task::none() };
            let Some(pt) = e.part.clone().filter(|p| e.test && parts::test_fits(p, &a.hearth)) else { return Task::none() };
            if a.offline() || !a.part_busy.insert(id.clone()) {
                return Task::none();
            }
            a.say(format!("Getting {} {}…", e.name, pt.version), false);
            let done = if pt.id == "mantel-rust" {
                format!("{} {} is in: your desktop restarts with it in a moment. Remove it here to go back.", e.name, pt.version)
            } else {
                format!("{} {} is in.", e.name, pt.version)
            };
            return Task::perform(net::off(move || parts::install(&pt).map(|_| done)), move |r| Msg::PartDone(id.clone(), r.unwrap_or_else(|| Err("It didn't install; nothing changed.".into()))));
        }
        Msg::PartRemove(id) => {
            let Some(e) = a.entry(&id).cloned() else { return Task::none() };
            let Some(pid) = e.part.as_ref().map(|p| p.id.clone()) else { return Task::none() };
            if !a.part_busy.insert(id.clone()) {
                return Task::none();
            }
            let done = if e.test {
                format!("{} is removed (it's in the Trash): the usual one is back.", e.name)
            } else {
                format!("{} is back to the one built into Hearth OS. Its update is in the Trash.", e.name)
            };
            return Task::perform(net::off(move || parts::remove(&pid).map(|_| done)), move |r| Msg::PartDone(id.clone(), r.unwrap_or_else(|| Err("Nothing changed.".into()))));
        }
        Msg::PartRetry(id) => {
            let Some(e) = a.entry(&id).cloned() else { return Task::none() };
            let Some(pid) = e.part.as_ref().map(|p| p.id.clone()) else { return Task::none() };
            if !a.part_busy.insert(id.clone()) {
                return Task::none();
            }
            let done = format!("{} tries its update again the next time it starts.", e.name);
            return Task::perform(net::off(move || parts::retry(&pid).map(|_| done)), move |r| Msg::PartDone(id.clone(), r.unwrap_or_else(|| Err("Nothing changed.".into()))));
        }
        Msg::Filter(c) => {
            a.filter = if c == "All" { String::new() } else { c };
        }
        Msg::LibTab(t) => a.lib_tab = t,
        Msg::PartDone(id, r) => {
            a.part_busy.remove(&id);
            a.read_installed();
            match r {
                Ok(s) => a.say(s, false),
                Err(s) => a.say(s, true),
            }
        }
        Msg::Start(id, kind) => {
            if a.prefs.metered {
                return a.start(&id, kind);
            }
            return Task::perform(net::off(net::metered), move |m| Msg::Metered(id.clone(), kind, m.unwrap_or(false)));
        }
        Msg::Metered(id, kind, metered) => {
            if metered {
                a.say("This connection is metered (a phone hotspot?). Turn on \"Download on metered connections\" in Store settings to download anyway.", true);
                return Task::none();
            }
            return a.start(&id, kind);
        }
        Msg::UpdateAll => {
            // Quick updates all at once (each is small and goes in its own folder); then apps that update themselves
            // are opened instead (their own updater), and the others one after another.
            let offers = a.part_offers();
            let mut tasks: Vec<Task<Msg>> = offers.iter().map(|(e, _)| update(a, Msg::PartUpdate(e.id.clone()))).collect();
            let ready = a.ready();
            if let Some((e, _, _)) = ready.iter().find(|(e, _, _)| !e.self_updates) {
                a.all = true;
                tasks.push(update(a, Msg::Start(e.id.clone(), JobKind::Update)));
                return Task::batch(tasks);
            }
            if !tasks.is_empty() {
                return Task::batch(tasks);
            }
            if ready.is_empty() {
                a.say("Everything is up to date.", false);
            } else {
                a.say("The apps that update themselves update when you open them (Open to update).", false);
            }
        }
        Msg::Cancel => {
            if let Some(j) = a.job.as_mut() {
                if j.can_cancel() {
                    j.cancel.cancel();
                    if j.demo.is_some() {
                        a.job = None;
                        return a.back();
                    }
                }
            }
            if let Some(r) = a.repair.as_ref() {
                if r.fixing {
                    r.cancel.cancel();
                }
            }
        }
        Msg::Tick => {
            let mut more = false;
            if let Some((j, e)) = a.job_entry() {
                if j.step == 0 && j.demo.is_none() && j.error.is_empty() {
                    let got = std::fs::metadata(files::part_file(&e)).map(|m| m.len()).unwrap_or(0);
                    if let Some(j) = a.job.as_mut() {
                        j.got = got;
                    }
                    more = true;
                }
            }
            if let Some(r) = a.repair.as_mut() {
                if r.fixing {
                    if let Some(e) = a.cat.apps.iter().find(|e| e.id == r.id) {
                        r.got = std::fs::metadata(files::part_file(e)).map(|m| m.len()).unwrap_or(0);
                    }
                    more = true;
                }
            }
            if more {
                return tick();
            }
        }
        Msg::Downloaded(r) => {
            let Some((j, e)) = a.job_entry() else { return Task::none() };
            match r {
                Ok(()) => {
                    if let Some(j) = a.job.as_mut() {
                        j.step = 1;
                        j.got = j.total;
                    }
                    let (p, sha) = (files::part_file(&e), j.rel.sha256.clone());
                    return Task::perform(net::off(move || files::check_download(&p, &sha)), |r| Msg::Verified(r.unwrap_or(Err("it stopped".into()))));
                }
                Err(Fail::Other(s)) if s == "Cancelled" => {
                    a.job = None;
                    a.say(format!("Cancelled. {} wasn't changed.", e.name), false);
                    return a.back();
                }
                Err(f) => a.job_fail(format!("The download stopped: {} Nothing was changed.", f.words())),
            }
        }
        Msg::Verified(r) => {
            let Some((j, e)) = a.job_entry() else { return Task::none() };
            match r {
                Err(s) => a.job_fail(s),
                Ok(()) => {
                    if j.cancel.cancelled() {
                        let _ = std::fs::remove_file(files::part_file(&e));
                        a.job = None;
                        return a.back();
                    }
                    let running = !files::running(&e).is_empty();
                    if let Some(j) = a.job.as_mut() {
                        j.step = 2;
                        j.was_running = running;
                    }
                    if running {
                        let e2 = e.clone();
                        return Task::perform(net::off(move || files::close(&e2, 20)), |r| Msg::Closed(r.unwrap_or(false)));
                    }
                    return update(a, Msg::Closed(true));
                }
            }
        }
        Msg::Closed(ok) => {
            let Some((_, e)) = a.job_entry() else { return Task::none() };
            if !ok {
                let _ = std::fs::remove_file(files::part_file(&e));
                a.job_fail(format!("{} didn't close, so nothing was changed. Close it and try again.", e.name));
                return Task::none();
            }
            if let Some(j) = a.job.as_mut() {
                j.step = 3;
            }
            let Some(j) = a.job.clone() else { return Task::none() };
            let e2 = e.clone();
            return Task::perform(
                net::off(move || -> Result<bool, String> {
                    let kept = files::put_in_place(&e2, &files::part_file(&e2)).map_err(|x| format!("Couldn't put it in place ({x}). The version before is still there."))?;
                    files::record(&e2, &j.rel.version, &j.rel.sha256, j.rel.size, &j.rel.url, &j.rel.page).map_err(|x| x.to_string())?;
                    files::write_menu(&e2).map_err(|x| format!("Installed, but its menu entry couldn't be written ({x}). Check and repair can write it."))?;
                    Ok(kept)
                }),
                |r| Msg::Placed(r.unwrap_or(Err("it stopped".into()))),
            );
        }
        Msg::Placed(r) => {
            let Some((j, e)) = a.job_entry() else { return Task::none() };
            a.read_installed();
            match r {
                Err(s) => a.job_fail(s),
                Ok(kept) => {
                    if let Some(j) = a.job.as_mut() {
                        j.step = 4;
                        j.kept = kept;
                    }
                    if j.was_running {
                        files::start(&e);
                        // It has started fine when it's still running after 8 s: then the version before goes.
                        let e2 = e.clone();
                        return Task::perform(
                            async move {
                                run::after(8000).await;
                                net::off(move || {
                                    let ok = !files::running(&e2).is_empty();
                                    if ok {
                                        files::drop_previous(&e2);
                                    }
                                    ok
                                })
                                .await
                                .unwrap_or(false)
                            },
                            Msg::Started,
                        );
                    }
                    return update(a, Msg::Started(false));
                }
            }
        }
        Msg::Started(_) => {
            if let Some(j) = a.job.as_mut() {
                j.step = 5;
            }
            a.focus = "open".into();
            // Update all: the next one.
            if a.all {
                let done = a.job.as_ref().map(|j| j.id.clone()).unwrap_or_default();
                match a.ready().into_iter().find(|(e, _, _)| !e.self_updates && e.id != done) {
                    Some((e, _, _)) => return update(a, Msg::Start(e.id, JobKind::Update)),
                    None => a.all = false,
                }
            }
        }
        Msg::RepairOpen(id) => {
            let Some(e) = a.entry(&id).cloned() else { return Task::none() };
            let sha = a.inst.get(&id).map(|i| i.sha256.clone()).unwrap_or_default();
            a.repair = Some(Repair { id: id.clone(), check: None, fixing: false, said: String::new(), got: 0, cancel: Cancel::default(), every_file: false });
            a.focus = "repair-now".into();
            return Task::perform(net::off(move || files::check(&e, &sha)), |c| Msg::RepairChecked(c.unwrap_or_default()));
        }
        Msg::RepairChecked(c) => {
            if let Some(r) = a.repair.as_mut() {
                r.check = Some(c);
            }
        }
        Msg::RepairNow => {
            let Some(r) = a.repair.clone() else { return Task::none() };
            let Some(c) = r.check.clone() else { return Task::none() };
            let Some(e) = a.entry(&r.id).cloned() else { return Task::none() };
            let Some(i) = a.inst.get(&r.id).cloned() else { return Task::none() };
            if c.wrong.is_empty() {
                a.repair = None;
                return a.back();
            }
            if c.needs_download() && a.offline() {
                a.say("You're offline: the repair downloads the same version again, so it waits until you're back online.", true);
                return Task::none();
            }
            if let Some(r) = a.repair.as_mut() {
                r.fixing = true;
                r.said.clear();
            }
            let cancel = r.cancel.clone();
            let need = c.needs_download();
            let t = Task::perform(
                net::off(move || -> Result<String, String> {
                    if need {
                        if i.url.is_empty() || i.sha256.len() != 64 {
                            return Err("The Store doesn't know where this version came from, so it can't download it again. Uninstall it and install it again.".into());
                        }
                        let part = files::part_file(&e);
                        net::download(&i.url, &part, &cancel).map_err(|f| format!("The download stopped: {} Nothing was changed.", f.words()))?;
                        files::check_download(&part, &i.sha256)?;
                        files::put_in_place(&e, &part).map_err(|x| x.to_string())?;
                        files::drop_previous(&e);
                        files::record(&e, &i.version, &i.sha256, i.size, &i.url, &i.page).map_err(|x| x.to_string())?;
                    }
                    files::fix_local(&e).map_err(|x| x.to_string())?;
                    Ok(format!("{} {} is put back as it was released. Its settings weren't touched.", e.name, i.version))
                }),
                |r| Msg::Repaired(r.unwrap_or(Err("it stopped".into()))),
            );
            return if need { Task::batch([t, tick()]) } else { t };
        }
        Msg::Repaired(r) => {
            a.read_installed();
            let id = a.repair.as_ref().map(|r| r.id.clone()).unwrap_or_default();
            match r {
                Ok(s) => {
                    a.repair = None;
                    let t = a.go(Page::App(id));
                    a.say(s, false);
                    return t;
                }
                Err(s) => {
                    if let Some(r) = a.repair.as_mut() {
                        r.fixing = false;
                        r.said = s;
                    }
                }
            }
        }
        Msg::EveryFile => {
            if let Some(r) = a.repair.as_mut() {
                r.every_file = !r.every_file;
            }
        }
        Msg::AskUninstall(id) => {
            let Some(e) = a.entry(&id).cloned() else { return Task::none() };
            a.menu = false;
            if !matches!(a.page, Page::App(_)) {
                a.trail.push(a.page.clone());
                a.page = Page::App(id.clone());
            }
            a.dlg = Some(Uninst { id, keep: true, size: None, focus: 0 });
            return Task::perform(net::off(move || files::data_size(&e)), |s| Msg::DataSize(s.unwrap_or(0)));
        }
        Msg::DataSize(n) => {
            if let Some(d) = a.dlg.as_mut() {
                d.size = Some(n);
            }
        }
        Msg::DlgKeep => {
            if let Some(d) = a.dlg.as_mut() {
                d.keep = !d.keep;
            }
        }
        Msg::DlgCancel => a.dlg = None,
        Msg::DlgUninstall => {
            let Some(d) = a.dlg.take() else { return Task::none() };
            let Some(e) = a.entry(&d.id).cloned() else { return Task::none() };
            let id = d.id.clone();
            return Task::perform(
                net::off(move || {
                    let _ = files::close(&e, 15);
                    files::uninstall(&e, d.keep).map(|v| v.len()).map_err(|x| x.to_string())
                }),
                move |r| Msg::Uninstalled(id.clone(), r.unwrap_or(Err("it stopped".into()))),
            );
        }
        Msg::Uninstalled(id, r) => {
            a.read_installed();
            let name = a.entry(&id).map(|e| e.name.clone()).unwrap_or_default();
            let t = a.go(Page::Home);
            match r {
                Ok(0) => a.say(format!("{name} was uninstalled. Its settings and data are still in your home folder."), false),
                Ok(_) => a.say(format!("{name} was uninstalled, and its settings and data are in the Trash."), false),
                Err(s) => a.say(format!("{name} couldn't be uninstalled completely: {s}"), true),
            }
            return t;
        }
        Msg::Menu => {
            a.menu = !a.menu;
            a.menu_at = 0;
        }
        Msg::ShowFiles(id) => {
            a.menu = false;
            let d = paths::app_dir(&id);
            run::detached(&["xdg-open", &d.to_string_lossy()]);
        }
        Msg::ReleasePage(id) => {
            a.menu = false;
            let page = a.inst.get(&id).map(|i| i.page.clone()).filter(|p| !p.is_empty()).or_else(|| a.seen.rel(&id).map(|r| r.page).filter(|p| !p.is_empty()));
            let gh = a.entry(&id).map(|e| e.github.clone()).unwrap_or_default();
            let url = page.unwrap_or_else(|| format!("https://github.com/{gh}/releases"));
            if url.starts_with("https://") {
                run::detached(&["xdg-open", &url]);
            }
        }
        Msg::ToggleNew(id) => {
            if view::new_shown(a, &id) {
                a.shown_new.remove(&id);
                a.hidden_new.insert(id);
            } else {
                a.hidden_new.remove(&id);
                a.shown_new.insert(id);
            }
        }
        Msg::Search(s) => {
            a.search = s;
            if a.page != Page::Home {
                a.page = Page::Home;
                a.trail.clear();
            }
        }
        Msg::SearchFocus => return operation::focus(SEARCH_ID),
        Msg::SetDaily(on) => return set_pref(a, "daily", on),
        Msg::SetFlag(k, on) => return set_pref(a, k, on),
        Msg::OpenFlathub => {
            if which("hearth-settings-app") {
                run::detached(&["hearth-settings-app", "--page", "store"]);
            } else {
                run::detached(&["plasma-discover"]);
            }
        }
        Msg::OpenFolder => {
            let d = paths::apps();
            let _ = std::fs::create_dir_all(&d);
            run::detached(&["xdg-open", &d.to_string_lossy()]);
        }
        Msg::OpenHearthUpdates => {
            if which("hearth-settings-app") {
                run::detached(&["hearth-settings-app", "--page", "updates"]);
            }
        }
        Msg::TryAgain => return a.check_now(),
        Msg::Checked(s, refreshed) => {
            a.checking = false;
            a.seen = *s;
            a.now = files::now();
            if refreshed {
                a.cat = catalog::load();
                a.read_installed();
            }
            if a.focus.is_empty() || !view::targets(a).iter().any(|t| t.id == a.focus) {
                a.focus = view::first_target(a);
            }
            return a.reconcile();
        }
        Msg::Reconciled(id, i) => {
            if let Some(i) = i {
                a.inst.insert(id, i);
            }
        }
        Msg::Pictures(_, list) => {
            for (url, path) in list {
                a.pics.entry(url).or_insert_with(|| image::Handle::from_path(path));
            }
        }
        Msg::Clock => {
            a.now = files::now();
            if a.pad {
                return clock();
            }
        }
        Msg::Details => {
            if let Some(j) = a.job.as_mut() {
                j.details = !j.details;
            }
        }
    }
    Task::none()
}

fn set_pref(a: &mut App, k: &'static str, on: bool) -> Task<Msg> {
    match prefs::set(k, on) {
        Ok(p) => {
            let test_changed = p.test != a.prefs.test;
            a.prefs = p;
            if test_changed {
                // test versions show or hide with it
                a.cat = catalog::load();
                a.read_installed();
                return a.check_now();
            }
        }
        Err(s) => a.say(s, true),
    }
    Task::none()
}

// ------------------------------------------------------------------ keys and the controller
fn move_focus(a: &mut App, dir: (i32, i32)) {
    let ts = view::targets(a);
    if ts.is_empty() {
        return;
    }
    let cur = ts.iter().position(|t| t.id == a.focus);
    let Some(ci) = cur else {
        a.focus = ts[0].id.clone();
        return;
    };
    let (row, col) = (ts[ci].row, ts.iter().filter(|t| t.row == ts[ci].row).position(|t| t.id == a.focus).unwrap_or(0));
    let next = match dir {
        (dx, 0) => {
            let same: Vec<&view::Target> = ts.iter().filter(|t| t.row == row).collect();
            let ni = col as i32 + dx;
            if ni >= 0 && (ni as usize) < same.len() {
                Some(same[ni as usize].id.clone())
            } else {
                None
            }
        }
        (0, dy) => {
            let rows: Vec<u16> = {
                let mut r: Vec<u16> = ts.iter().map(|t| t.row).collect();
                r.dedup();
                r
            };
            let ri = rows.iter().position(|r| *r == row).unwrap_or(0) as i32 + dy;
            if ri >= 0 && (ri as usize) < rows.len() {
                let nr = rows[ri as usize];
                let same: Vec<&view::Target> = ts.iter().filter(|t| t.row == nr).collect();
                Some(same[col.min(same.len() - 1)].id.clone())
            } else {
                None
            }
        }
        _ => None,
    };
    if let Some(n) = next {
        a.focus = n;
    }
}

fn press_focused(a: &mut App) -> Task<Msg> {
    if let Some(t) = view::targets(a).into_iter().find(|t| t.id == a.focus) {
        if let Some(m) = t.enter {
            return update(a, m);
        }
    }
    Task::none()
}

fn page_step(a: &mut App, d: i32) -> Task<Msg> {
    let i = PAGES.iter().position(|p| *p == a.page).unwrap_or(0) as i32;
    let n = PAGES.len() as i32;
    let to = PAGES[((i + d).rem_euclid(n)) as usize].clone();
    a.go(to)
}

/// Y: search on the lists; what's new on an app or an update; every file on the repair page.
fn y_action(a: &mut App) -> Task<Msg> {
    match a.page.clone() {
        Page::Repair(_) => update(a, Msg::EveryFile),
        Page::Updates => {
            let id = a.focus.strip_prefix("upd-").map(str::to_string);
            match id {
                Some(id) => update(a, Msg::ToggleNew(id)),
                None => update(a, Msg::SearchFocus),
            }
        }
        Page::Home if a.pad => match view::featured(a) {
            Some((e, _)) => a.go(Page::App(e.id)),
            None => Task::none(),
        },
        _ => update(a, Msg::SearchFocus),
    }
}

fn menu_items(a: &App) -> Vec<Msg> {
    match &a.page {
        Page::App(id) => vec![Msg::Go(Page::Repair(id.clone())), Msg::ShowFiles(id.clone()), Msg::ReleasePage(id.clone()), Msg::AskUninstall(id.clone())],
        _ => Vec::new(),
    }
}

fn dlg_key(a: &mut App, k: &str) -> Task<Msg> {
    let Some(d) = a.dlg.as_mut() else { return Task::none() };
    match k {
        "up" => d.focus = 0,
        "down" => d.focus = if d.focus == 0 { 2 } else { d.focus },
        "left" if d.focus > 1 => d.focus = 1,
        "right" if d.focus >= 1 => d.focus = 2,
        "enter" => {
            let f = d.focus;
            return update(a, match f { 0 => Msg::DlgKeep, 1 => Msg::DlgCancel, _ => Msg::DlgUninstall });
        }
        _ => {}
    }
    Task::none()
}

fn menu_key(a: &mut App, k: &str) -> Task<Msg> {
    let items = menu_items(a);
    match k {
        "up" => a.menu_at = a.menu_at.saturating_sub(1),
        "down" => a.menu_at = (a.menu_at + 1).min(items.len().saturating_sub(1)),
        "enter" => {
            if let Some(m) = items.get(a.menu_at).cloned() {
                a.menu = false;
                return update(a, m);
            }
        }
        _ => {}
    }
    Task::none()
}

fn nav(a: &mut App, k: &str) -> Task<Msg> {
    if a.dlg.is_some() {
        return dlg_key(a, k);
    }
    if a.menu {
        return menu_key(a, k);
    }
    match k {
        "up" => move_focus(a, (0, -1)),
        "down" => move_focus(a, (0, 1)),
        "left" => move_focus(a, (-1, 0)),
        "right" => move_focus(a, (1, 0)),
        "enter" => return press_focused(a),
        _ => {}
    }
    Task::none()
}

fn key(a: &mut App, ev: KEvent) -> Task<Msg> {
    let KEvent::KeyPressed { key, modifiers, .. } = ev else { return Task::none() };
    if a.pad && !a.pad_fixed && pad::desk_in_use() {
        set_look(a, false);
    }
    let ctrl = modifiers.control();
    match key.as_ref() {
        Key::Named(Named::PageUp) if ctrl => page_step(a, -1),
        Key::Named(Named::PageDown) if ctrl => page_step(a, 1),
        Key::Character("f") if ctrl => update(a, Msg::SearchFocus),
        Key::Named(Named::Escape) => {
            if !a.search.is_empty() {
                a.search.clear();
                return Task::none();
            }
            a.back()
        }
        Key::Named(Named::ArrowUp) => nav(a, "up"),
        Key::Named(Named::ArrowDown) => nav(a, "down"),
        Key::Named(Named::ArrowLeft) => nav(a, "left"),
        Key::Named(Named::ArrowRight) => nav(a, "right"),
        Key::Named(Named::Enter) => nav(a, "enter"),
        Key::Named(Named::ContextMenu) => update(a, Msg::Menu),
        _ => Task::none(),
    }
}

fn set_look(a: &mut App, pad_on: bool) {
    if a.pad != pad_on {
        a.pad = pad_on;
        pad::PAD_LOOK.store(pad_on, std::sync::atomic::Ordering::Relaxed);
    }
}

fn pad_press(a: &mut App, b: pad::Button) -> Task<Msg> {
    use pad::Button as P;
    pad::ctrl_used();
    let mut t = Task::none();
    if !a.pad && !a.pad_fixed {
        set_look(a, true);
        a.focus = view::first_target(a);
        t = clock();
    }
    let r = match b {
        P::Up => nav(a, "up"),
        P::Down => nav(a, "down"),
        P::Left => nav(a, "left"),
        P::Right => nav(a, "right"),
        P::A => nav(a, "enter"),
        P::B => a.back(),
        P::X => {
            if a.dlg.is_some() {
                update(a, Msg::DlgUninstall)
            } else {
                update(a, Msg::UpdateAll)
            }
        }
        P::Y => y_action(a),
        P::Lb => page_step(a, -1),
        P::Rb => page_step(a, 1),
        P::Menu => {
            if matches!(a.page, Page::App(_)) {
                update(a, Msg::Menu)
            } else if a.page == Page::Work {
                update(a, Msg::Details)
            } else if a.pad && a.page == Page::Home {
                match view::featured(a) {
                    Some((e, _)) => {
                        let t = a.go(Page::App(e.id));
                        a.menu = true;
                        t
                    }
                    None => Task::none(),
                }
            } else {
                Task::none()
            }
        }
        P::View => Task::none(),
    };
    Task::batch([t, r])
}

fn subscription(_a: &App) -> Subscription<Msg> {
    Subscription::batch([
        Subscription::run(hearth_ui::theme::watch).map(|_| Msg::Theme),
        iced::keyboard::listen().map(Msg::Key),
        Subscription::run(pad::presses).map(Msg::Pad),
    ])
}

fn main() -> iced::Result {
    if app::handle_cli(
        "hearth-store-app",
        "Hearth Store: Dragongamer9's apps for Hearth OS, installed, updated, repaired and uninstalled.\n  --check                 the daily look for updates (the user timer), no window\n  --page home|yours|updates|built-in|settings|app:ID|repair:ID\n  --app ID                then an app's page, from the page --page opened (its back goes there)\n  --show menu|uninstall:ID   open with the ⋯ menu or the uninstall dialog (pictures)\n  --controller / --mouse  the controller look, or the other, whatever is used",
    ) {
        return Ok(());
    }
    if app::flag("--check") {
        std::process::exit(check::daily());
    }
    iced::application(App::new, update, view::view)
        .title("Hearth Store")
        .subscription(subscription)
        .theme(|a: &App| app::iced_theme(&a.p))
        .window(app::window("hearth-store", (1280.0, 800.0), (960.0, 620.0)))
        .default_font(app::INTER)
        .run()
}
