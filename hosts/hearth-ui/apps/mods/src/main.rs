//! hearth-mods: our Mod Manager (docs/mockups/checklists/mod-manager.md; the approved boards docs/mockups/mod-manager).
//! Every game's mods in one place: the priority ladder with separators, conflicts file by file, plugins and their
//! masters, Sort with LOOT's masterlist, profiles, health, installing (FOMOD done in Hearth), one search across Nexus
//! Mods, Thunderstore, mod.io and CurseForge, updates with changelogs and whole-setup snapshots, saves marked with the
//! mods they need, crash logs in plain words, Describe it with Hearth AI, and a big controller view for Game Mode.
//!
//! The work is Hearth Mods' own (hosts/hearth-gaming: the Hearth Mod Organizer, Nexus Mods, Wabbajack, Play modded,
//! Add to Steam), called through hearth_gaming::mods_app as the old page did; the game's own folder is never touched
//! (the organizer lays the mods over it while it runs). Made of blocks (hearth_ui::blocks / studio).
//!
//! The program keeps its name, `hearth-mods`, and every command line the old one had (`--nxm`, `--play-modded`,
//! `--serve`, `--organizer …`, …: mods_app::cli), so the nxm handler, Game Mode's worker and the Steam shortcuts work
//! the same. `--game ID` opens a game, `--page NAME` a page, `--fullscreen` / `--tv` the controller view.

mod back;
mod logic;
mod more;
mod pad;
mod pages;
mod patch;
mod sources;
mod sync;

use back::{Detail, Game};
use hearth_oss_kit_a::{self as k, Ch, Sz, E};
use hearth_ui::app::{self, win};
use hearth_ui::ui::{self, t, TitleMsgs, G};
use hearth_ui::Pal;
use iced::widget::{column, container, row, scrollable, Column, Space};
use iced::{Element, Length, Subscription, Task};
use serde_json::{json, Value};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Page {
    Every,
    Find,
    Downloads,
    Updates,
    Crashes,
    Describe,
    Modlists,
    Organizer,
    Accounts,
    Settings,
    Game,
}

pub const SECTIONS: &[(Page, &str)] = &[
    (Page::Every, "Every game"),
    (Page::Find, "Find mods"),
    (Page::Downloads, "Downloads"),
    (Page::Updates, "Updates & going back"),
    (Page::Crashes, "Crashes"),
    (Page::Describe, "Describe it"),
    (Page::Modlists, "Modlists & collections"),
    (Page::Organizer, "Organizer instances"),
    (Page::Accounts, "Accounts"),
    (Page::Settings, "Settings"),
];

fn page_named(s: &str) -> Option<Page> {
    Some(match s {
        "every" | "games" => Page::Every,
        "find" => Page::Find,
        "downloads" | "install" | "add" => Page::Downloads,
        "updates" => Page::Updates,
        "crashes" | "crash" => Page::Crashes,
        "describe" => Page::Describe,
        "modlists" => Page::Modlists,
        "organizer" => Page::Organizer,
        "accounts" => Page::Accounts,
        "settings" => Page::Settings,
        "game" | "mods" => Page::Game,
        _ => return None,
    })
}

#[derive(Debug, Clone)]
pub enum Msg {
    Describe(hearth_ui::describe::DMsg),
    Theme,
    Pad(pad::Button),
    Tick,
    World(Result<(Vec<Game>, Value), String>),
    Detail(Box<Detail>),
    Go(Page),
    PickGame(String),
    Search(String),
    Said(Result<Value, String>),
    Reload,
    // board 1
    Tab(u8),
    Filter(u8),
    FilterText(String),
    Fold(String),
    Pick(String),
    Toggle(String, bool),
    Lift(String),
    DropOn(i64),
    MoveBy(i64),
    PutBack,
    Lens(Result<Value, String>),
    LensTab(u8),
    Profile(String),
    ProfileNew,
    Starts(String),
    Play,
    AddSteam,
    More,
    FixMasters,
    ShowLosers,
    Sort,
    ApplySort,
    NotNow,
    Masterlist(Result<String, String>),
    MoveBelow(String),
    HideFile(String),
    PluginToggle(String, bool),
    // board 2
    PickFile,
    Picked(Option<String>),
    Addr(String),
    AddrGo,
    Install(String),
    Installed(Result<Value, String>),
    FomodStep(usize),
    FomodPick(usize, usize, usize),
    FomodNext,
    FomodBack,
    FomodCancel,
    IfIn(u8),
    InstallAll,
    Downloads(Result<Value, String>),
    // board 3
    FindQ(String),
    FindGo,
    Found(&'static str, Result<Vec<sources::Hit>, String>),
    FindSrc(u8),
    FindFlag(u8),
    FindPick(usize),
    FindFile(usize),
    FindInstall,
    Endorse,
    Open(String),
    Sets(Result<Value, String>),
    SetInstall(usize),
    // board 4
    CheckNow,
    Checked(Result<Value, String>),
    UpdTab(u8),
    UpdTick(String),
    UpdPick(String),
    UpdateApply,
    SnapPick(usize),
    TakeSnap,
    GoBack,
    CheckEvery(u8),
    Setting(&'static str, bool),
    // board 5
    CrashPick(usize),
    CrashTab(u8),
    CopyLog,
    Note(String),
    SendLogs,
    Sent(hearth_ui::run::Output),
    FixStep(u8),
    // board 6
    AskText(String),
    Ask,
    Asked(Result<Value, String>),
    Suggest(String),
    Speak,
    InstallSet,
    SaveLater,
    // board 8
    CheckAll,
    SetUp(String),
    HideGame(String),
    // accounts
    KeyText(String),
    KeyFor(&'static str),
    SaveKey,
    Keys(Vec<(&'static str, bool)>),
    Modlists(Result<Value, String>),
    // your other devices and the home server
    Devices(Vec<Value>),
    Inbox(Vec<Value>),
    SendGame(String),
    AlsoAdd,
    Measure,
    // the side's search
    SideMod(String, String),
    // the old window's parts: sign-ins, the deploy mode, Wabbajack's progress
    Accounts(Result<Value, String>),
    SignIn(&'static str),
    SignOut(&'static str),
    MegaEmail(String),
    MegaPass(String),
    MegaGo,
    DeployMode(&'static str),
    WjTick,
    // Health, the lens
    UpdateExt,
    MakePatch,
    Patched(Result<String, String>),
    // board 2
    ReadMe,
    Before(Box<Option<more::Before>>),
    // board 3
    FindPop(&'static str),
    FindGame(String),
    FindOrder(u8),
    Needs(String, Vec<String>, Vec<String>, String),
    Endorsed(Result<Value, String>),
    FindLike(String),
    // board 4
    KeepSnaps(u8),
    Versions(String, Result<Value, String>),
    UseVersion(String, String),
    // board 5
    TestStart,
    Tested(Result<String, String>),
    GetCrashLogger,
    Heard(Result<String, String>),
    Server((u64, usize)),
    Pictures,
    // board 7
    Drag,
    Max,
    Min,
    WClose,
    Resize(iced::window::Direction),
}

pub struct App {
    pub describe: hearth_ui::describe::Describe,
    pub p: Pal,
    pub tv: bool,
    pub page: Page,
    pub games: Vec<Game>,
    pub instances: Value,
    pub loading: bool,
    pub pick: String,
    pub detail: Detail,
    pub search: String,
    pub said: String,
    pub settings: Value,
    // board 1
    pub tab: u8,
    pub filter: u8,
    pub filter_text: String,
    pub folded: Vec<String>,
    pub picked: String,
    pub lifted: Option<(String, i64)>,
    pub lens: Value,
    pub lens_tab: u8,
    pub sort: Option<(Vec<String>, Vec<logic::Move>)>,
    pub starts: String,
    pub show_losers: bool,
    pub pop: Option<&'static str>,
    // board 2
    pub addr: String,
    pub fomod: Value,
    pub fomod_step: usize,
    pub choices: Vec<Vec<Vec<usize>>>,
    pub if_in: u8,
    pub downloads: Value,
    pub waiting: Vec<String>,
    // board 3
    pub find_q: String,
    pub hits: Vec<sources::Hit>,
    pub counts: Vec<(&'static str, Result<usize, String>)>,
    pub find_src: u8,
    pub find_flags: [bool; 3],
    pub find_pick: usize,
    pub find_file: usize,
    pub sets: Value,
    // board 4
    pub updates: Value,
    pub upd_tab: u8,
    pub upd_pick: String,
    pub snap_pick: usize,
    // board 5
    pub crash_pick: usize,
    pub crash_tab: u8,
    pub note: String,
    // board 6
    pub ask: String,
    pub talk: Value,
    pub asking: bool,
    // accounts
    pub key_for: &'static str,
    pub key_text: String,
    pub keys: Vec<(&'static str, bool)>,
    pub modlists: Value,
    pub devices: Vec<Value>,
    pub accounts: Value,
    pub mega: (String, String),
    pub before: Option<more::Before>,
    pub find_game: String,
    pub find_order: u8,
    pub versions: std::collections::BTreeMap<String, Vec<Value>>,
    pub testing: bool,
    pub moved_up: bool,
    pub server: (u64, usize),
}

pub fn now() -> i64 {
    hearth_ui::time::now()
}

impl App {
    fn new() -> (App, Task<Msg>) {
        let tv = app::flag("--fullscreen") || app::flag("--tv");
        let page = app::arg("--page").and_then(|p| page_named(&p)).unwrap_or(if app::arg("--game").is_some() { Page::Game } else { Page::Every });
        let a = App {
            describe: hearth_ui::describe::Describe::new("mods", "Mod Manager").with_pages(BLOCKS).with_kit(&KIT),
            p: hearth_ui::theme::load(),
            tv,
            page,
            games: Vec::new(),
            instances: Value::Null,
            loading: true,
            pick: app::arg("--game").unwrap_or_default(),
            detail: Detail::default(),
            search: String::new(),
            said: String::new(),
            settings: k::read_settings("mods"),
            tab: 0,
            filter: 0,
            filter_text: String::new(),
            folded: Vec::new(),
            picked: app::arg("--mod").unwrap_or_default(),
            lifted: None,
            lens: Value::Null,
            lens_tab: 0,
            sort: None,
            starts: String::new(),
            show_losers: false,
            pop: None,
            addr: String::new(),
            fomod: back::read_own("fomod-open.json"),
            fomod_step: app::arg("--step").and_then(|x| x.parse().ok()).unwrap_or(0),
            choices: Vec::new(),
            if_in: 0,
            downloads: Value::Null,
            waiting: Vec::new(),
            find_q: app::arg("--find").unwrap_or_default(),
            hits: Vec::new(),
            counts: Vec::new(),
            find_src: 0,
            find_flags: [true, true, false],
            find_pick: 0,
            find_file: 0,
            sets: Value::Null,
            updates: back::read_own("updates.json"),
            upd_tab: 0,
            upd_pick: String::new(),
            snap_pick: app::arg("--snap").and_then(|x| x.parse().ok()).unwrap_or(0),
            crash_pick: 0,
            crash_tab: app::arg("--crash-tab").and_then(|x| x.parse().ok()).unwrap_or(0),
            note: String::new(),
            ask: String::new(),
            talk: back::read_own("describe.json"),
            asking: false,
            key_for: "",
            key_text: String::new(),
            keys: Vec::new(),
            modlists: Value::Null,
            devices: Vec::new(),
            accounts: Value::Null,
            mega: (String::new(), String::new()),
            before: None,
            find_game: String::new(),
            find_order: 0,
            versions: std::collections::BTreeMap::new(),
            testing: false,
            moved_up: true,
            server: (0, 0),
        };
        let mut t = vec![Task::perform(back::work(back::load_world), Msg::World), Task::perform(back::call("downloads_list", Value::Null), Msg::Downloads), Task::perform(sources::have_keys(), Msg::Keys), Task::perform(back::work(sync::others), Msg::Devices), Task::perform(back::work(sync::take_inbox), Msg::Inbox), Task::perform(back::call("accounts_list", Value::Null), Msg::Accounts)];
        if a.settings["checkUpdates"].as_str() == Some("open") || a.settings["checkUpdates"].as_str().unwrap_or("night") == "night" && a.updates["checked"].as_i64().map(|c| now() - c > 20 * 3600).unwrap_or(false) {
            t.push(Task::perform(back::work(more::check_all), |_| Msg::Reload));
        }
        if a.page == Page::Every {
            t.push(Task::perform(back::work(sync::server_files), Msg::Server));
        }
        (a, Task::batch(t))
    }

    pub fn game(&self) -> Option<&Game> {
        self.games.iter().find(|g| g.key == self.pick).or(self.games.first())
    }
    pub fn inst_id(&self) -> Option<String> {
        self.game().and_then(|g| g.inst_id())
    }
    pub fn mods(&self) -> Vec<Value> {
        self.detail.s["mods"].as_array().cloned().unwrap_or_default()
    }
    pub fn plugins(&self) -> Vec<Value> {
        self.detail.s["plugins"].as_array().cloned().unwrap_or_default()
    }
    pub fn inst(&self) -> &Value {
        &self.detail.s["instance"]
    }
    pub fn profile(&self) -> String {
        self.inst()["profile"].as_str().unwrap_or("Default").to_string()
    }
    pub fn profile_dir(&self) -> std::path::PathBuf {
        std::path::PathBuf::from(self.inst()["dir"].as_str().unwrap_or("")).join("profiles").join(self.profile())
    }
    pub fn inst_dir(&self) -> std::path::PathBuf {
        std::path::PathBuf::from(self.inst()["dir"].as_str().unwrap_or(""))
    }
    pub fn mods_dir(&self) -> std::path::PathBuf {
        std::path::PathBuf::from(self.inst()["modsDir"].as_str().unwrap_or(""))
    }
    pub fn setting(&self, k: &str, default: bool) -> bool {
        self.settings[k].as_bool().unwrap_or(default)
    }

    fn reload_detail(&self) -> Task<Msg> {
        let Some(g) = self.game().cloned() else { return Task::none() };
        Task::perform(back::work(move || back::load_detail(&g)), |d| Msg::Detail(Box::new(d)))
    }

    /// An organizer command for the open game, then everything read again.
    fn org(&mut self, args: &[&str]) -> Task<Msg> {
        let Some(id) = self.inst_id() else {
            self.said = "This game has no Hearth Mod Organizer instance yet: Every game › Set up mods.".into();
            return Task::none();
        };
        let mut v = vec![id];
        v.extend(args.iter().map(|s| s.to_string()));
        Task::perform(back::organizer(v), Msg::Said)
    }

    /// A snapshot of the whole setup before a change (board 4: "a snapshot is made first").
    pub fn snapshot(&mut self, why: &str) -> bool {
        if !self.detail.organizer {
            return true;
        }
        match logic::take(&self.inst_dir(), &self.profile_dir(), &self.mods_dir(), why, "", false, now()) {
            Ok(_) => {
                more::prune(&self.inst_dir(), self.settings["keepSnapshots"].as_i64().unwrap_or(60), 20);
                true
            }
            Err(e) => {
                self.said = format!("Nothing was changed: the snapshot couldn't be made ({e}).");
                false
            }
        }
    }

    fn save_setting(&mut self, key: &str, v: Value) {
        let mut m = serde_json::Map::new();
        m.insert(key.into(), v);
        match k::save_settings("mods", m) {
            Ok(()) => self.settings = k::read_settings("mods"),
            Err(e) => self.said = format!("Couldn't save that setting: {e}"),
        }
    }
}

fn update(a: &mut App, m: Msg) -> Task<Msg> {
    match m {
        Msg::Describe(dm) => {
            if let Some(m) = hearth_ui::studio::act(&dm).and_then(block_action) {
                return update(a, m);
            }
            return a.describe.update(dm).map(Msg::Describe);
        }
        Msg::Theme => a.p = hearth_ui::theme::load(),
        Msg::Pad(b) => return pages::pad(a, b),
        Msg::Tick => {
            if a.page == Page::Modlists {
                return update(a, Msg::WjTick);
            }
            if a.page == Page::Downloads || a.tv {
                return Task::perform(back::call("downloads_list", Value::Null), Msg::Downloads);
            }
        }
        Msg::World(r) => {
            a.loading = false;
            match r {
                Ok((g, i)) => {
                    a.games = g;
                    a.instances = i;
                    // a dot for games with updates waiting (blue) or things to look at (amber)
                    let ups = a.updates["updates"].as_array().cloned().unwrap_or_default();
                    for g in a.games.iter_mut() {
                        if ups.iter().any(|u| u["game"].as_str() == Some(g.title.as_str())) {
                            g.dot = 2;
                        }
                    }
                    if !a.games.iter().any(|g| g.key == a.pick) {
                        a.pick = a.games.first().map(|g| g.key.clone()).unwrap_or_default();
                    }
                    // a search asked for when it opened (--find) goes once the games are known
                    if !a.find_q.is_empty() && a.counts.is_empty() && a.hits.is_empty() {
                        return Task::batch([a.reload_detail(), update(a, Msg::FindGo)]);
                    }
                }
                Err(e) => a.said = e,
            }
            return a.reload_detail();
        }
        Msg::Detail(d) => {
            if d.key == a.pick || a.game().map(|g| g.key == d.key).unwrap_or(false) {
                a.detail = *d;
                if let Some(g) = a.games.iter_mut().find(|g| g.key == a.detail.key) {
                    let w = a.detail.s["warnings"].as_array().map(|w| w.len()).unwrap_or(0);
                    if w > 0 && g.dot == 0 {
                        g.dot = 1;
                    }
                }
                if a.picked.is_empty() || !a.mods().iter().any(|m| m["id"].as_str() == Some(a.picked.as_str())) {
                    a.picked = a.mods().iter().find(|m| m["kind"] == "mod" && m["conflicts"]["losing"].as_u64().unwrap_or(0) > 0).or(a.mods().iter().find(|m| m["kind"] == "mod")).and_then(|m| m["id"].as_str()).unwrap_or("").to_string();
                }
                if app::flag("--lift") && a.lifted.is_none() {
                    let prio = a.mods().iter().find(|m| m["id"].as_str() == Some(a.picked.as_str())).and_then(|m| m["priority"].as_i64()).unwrap_or(0);
                    a.lifted = Some((a.picked.clone(), prio));
                }
                if a.choices.is_empty() {
                    a.choices = serde_json::from_value(a.fomod["evaluation"]["choices"].clone()).unwrap_or_default();
                }
                if a.starts.is_empty() {
                    a.starts = a.detail.s["executables"].as_array().and_then(|e| e.first()).and_then(|e| e["title"].as_str()).unwrap_or("").to_string();
                }
                let sort = if app::flag("--sort") && a.sort.is_none() { update(a, Msg::Sort) } else { Task::none() };
                let sort = Task::batch([sort, publish(a), preview(a)]);
                if let Some(id) = a.inst_id() {
                    if !a.picked.is_empty() {
                        return Task::batch([sort, Task::perform(back::organizer(vec![id, "conflicts".into(), a.picked.clone()]), Msg::Lens)]);
                    }
                }
            }
        }
        Msg::Go(p) => {
            a.page = p;
            a.pop = None;
            match p {
                Page::Find if a.sets.is_null() => return Task::perform(back::call("wj_gallery", json!({"refresh": false})), Msg::Sets),
                Page::Modlists => return Task::batch([Task::perform(back::call("wj_gallery", json!({"refresh": false})), Msg::Modlists), Task::perform(back::call("wj_jobs", Value::Null), Msg::Sets)]),
                Page::Downloads => return Task::perform(back::call("downloads_list", Value::Null), Msg::Downloads),
                Page::Every => return Task::batch([Task::perform(back::work(sync::server_files), Msg::Server), Task::perform(back::work(sync::others), Msg::Devices)]),
                Page::Describe => return pictures(a),
                _ => {}
            }
        }
        Msg::PickGame(key) => {
            a.pick = key;
            a.page = Page::Game;
            a.picked.clear();
            a.starts.clear();
            a.sort = None;
            a.lens = Value::Null;
            return a.reload_detail();
        }
        Msg::Search(s) => a.search = s,
        Msg::Said(r) => {
            a.said = match &r {
                Ok(v) if v["ok"] == false => v["error"].as_str().unwrap_or("That didn't work.").to_string(),
                Ok(v) => v["notes"].as_array().map(|n| n.iter().filter_map(|x| x.as_str()).collect::<Vec<_>>().join(" ")).unwrap_or_default(),
                Err(e) => e.clone(),
            };
            return Task::batch([a.reload_detail(), Task::perform(back::work(back::load_world), Msg::World)]);
        }
        Msg::Reload => return Task::batch([Task::perform(back::work(back::load_world), Msg::World)]),
        Msg::Tab(i) => a.tab = i,
        Msg::Filter(i) => a.filter = i,
        Msg::FilterText(s) => a.filter_text = s,
        Msg::Fold(s) => {
            if let Some(i) = a.folded.iter().position(|x| *x == s) {
                a.folded.remove(i);
            } else {
                a.folded.push(s);
            }
        }
        Msg::Pick(id) => {
            a.picked = id.clone();
            if let Some(i) = a.inst_id() {
                return Task::perform(back::organizer(vec![i, "conflicts".into(), id]), Msg::Lens);
            }
        }
        Msg::Lens(r) => a.lens = r.unwrap_or(Value::Null),
        Msg::LensTab(i) => a.lens_tab = i,
        Msg::Toggle(id, on) => {
            if a.detail.organizer {
                return a.org(&[if on { "enable" } else { "disable" }, &id]);
            }
            let g = a.pick.clone();
            return Task::perform(back::call("toggle", json!({"id": g, "m": id, "on": on})), Msg::Said);
        }
        Msg::Lift(id) => {
            let prio = a.mods().iter().find(|m| m["id"].as_str() == Some(id.as_str())).and_then(|m| m["priority"].as_i64()).unwrap_or(0);
            a.picked = id.clone();
            a.lifted = Some((id, prio));
        }
        Msg::DropOn(prio) => {
            if let Some((id, _)) = a.lifted.take() {
                if !a.snapshot(&format!("before {id} moved")) {
                    return Task::none();
                }
                if a.detail.organizer {
                    return a.org(&["move", &id, &prio.to_string()]);
                }
                let cur = a.mods().iter().find(|m| m["id"].as_str() == Some(id.as_str())).and_then(|m| m["order"].as_i64()).unwrap_or(prio);
                let g = a.pick.clone();
                return Task::perform(back::call("move_mod", json!({"id": g, "m": id, "by": prio - cur})), Msg::Said);
            }
        }
        Msg::MoveBy(by) => {
            if let Some((id, _)) = a.lifted.clone() {
                if a.detail.organizer {
                    return a.org(&["move", &id, "--by", &by.to_string()]);
                }
            } else if !a.picked.is_empty() && a.detail.organizer {
                let id = a.picked.clone();
                return a.org(&["move", &id, "--by", &by.to_string()]);
            }
        }
        Msg::PutBack => {
            if let Some((id, prio)) = a.lifted.take() {
                if a.detail.organizer {
                    return a.org(&["move", &id, &prio.to_string()]);
                }
            }
        }
        Msg::Profile(p) => {
            a.pop = None;
            return a.org(&["profile", &p]);
        }
        Msg::ProfileNew => {
            let name = format!("{} (copy)", a.profile());
            let from = a.profile();
            return a.org(&["profile-create", &name, "--from", &from]);
        }
        Msg::Starts(s) => a.starts = s,
        Msg::Play => {
            let Some(g) = a.game().cloned() else { return Task::none() };
            let on = a.mods().iter().filter(|m| m["kind"] == "mod" && m["enabled"] == true).count();
            more::note_play(&g.title, &a.profile(), &a.starts, on);
            // a measuring start another device asked for: MangoHud logs its first 60 seconds
            let m = back::read_own("measure.json");
            if m["game"].as_str() == Some(g.title.as_str()) {
                for (k2, v) in sync::measure_env() {
                    std::env::set_var(k2, v);
                }
                let _ = back::write_own("measure.json", &json!({}));
                a.said = format!("Measuring this start for {} (MangoHud's log, 60 seconds).", m["from"].as_str().unwrap_or("your other device"));
            } else {
                std::env::remove_var("MANGOHUD_CONFIG");
            }
            if let Some(id) = g.inst_id() {
                let exe = a.starts.clone();
                return Task::perform(back::call("organizer_run", json!({"id": id, "exe": exe})), Msg::Said);
            }
            return Task::perform(back::call("play", json!({"id": g.key})), Msg::Said);
        }
        Msg::AddSteam => {
            let Some(g) = a.game().cloned() else { return Task::none() };
            return Task::perform(back::call("add_to_steam", json!({"id": g.key, "game": g.key, "kinds": ["modded"]})), Msg::Said);
        }
        Msg::More => a.pop = if a.pop == Some("more") { None } else { Some("more") },
        Msg::FixMasters => {
            // turn the mods that have the missing masters back on
            let missing: Vec<String> = a.plugins().iter().filter(|p| p["active"] == true).flat_map(|p| p["missing"].as_array().cloned().unwrap_or_default()).filter_map(|x| x.as_str().map(str::to_string)).collect();
            let off: Vec<String> = a.plugins().iter().filter(|p| missing.iter().any(|m| p["name"].as_str().map(|n| n.eq_ignore_ascii_case(m)).unwrap_or(false))).filter_map(|p| p["mod"].as_str().map(str::to_string)).filter(|m| !m.is_empty()).collect();
            if off.is_empty() {
                a.said = "The missing masters aren't in any of your mods: get them from the mod's page (its requirements).".into();
                return Task::none();
            }
            if !a.snapshot("before Fix it turned masters back on") {
                return Task::none();
            }
            let mut t = Vec::new();
            for m in off {
                t.push(a.org(&["enable", &m]));
                let p: Vec<String> = a.plugins().iter().filter(|p| p["mod"].as_str() == Some(m.as_str())).filter_map(|p| p["name"].as_str().map(str::to_string)).collect();
                for pl in p {
                    t.push(a.org(&["plugin-enable", &pl]));
                }
            }
            return Task::batch(t);
        }
        Msg::ShowLosers => {
            a.show_losers = !a.show_losers;
            a.filter = if a.show_losers { 5 } else { 0 };
        }
        Msg::Sort => {
            let kind = a.inst()["game"].as_str().unwrap_or("").to_string();
            let list = logic::plugins_of(&a.detail.s);
            a.sort = Some(logic::sort(&list, &a.detail.rules));
            if a.detail.rules.is_empty() && logic::masterlist_url(&kind).is_some() {
                return Task::perform(back::work(move || back::fetch_masterlist(&kind)), Msg::Masterlist);
            }
        }
        Msg::Masterlist(r) => match r {
            Ok(text) => {
                a.detail.rules = logic::parse_masterlist(&text);
                a.detail.masterlist = format!("LOOT's masterlist, {}", hearth_ui::time::day(now()));
                let list = logic::plugins_of(&a.detail.s);
                a.sort = Some(logic::sort(&list, &a.detail.rules));
            }
            Err(e) => a.said = e,
        },
        Msg::ApplySort => {
            let Some((order, _)) = a.sort.take() else { return Task::none() };
            if !a.snapshot("before Sort moved plugins") {
                return Task::none();
            }
            let list = a.plugins();
            let mut t = Vec::new();
            for (i, n) in order.iter().enumerate() {
                let now_at = list.iter().position(|p| p["name"].as_str() == Some(n.as_str()));
                if now_at != Some(i) && !list.get(now_at.unwrap_or(0)).map(|p| p["implicit"] == true).unwrap_or(true) {
                    t.push(a.org(&["plugin-move", n, &i.to_string()]));
                }
            }
            return Task::batch(t);
        }
        Msg::NotNow => a.sort = None,
        Msg::MoveBelow(other) => {
            let prio = a.mods().iter().find(|m| m["id"].as_str() == Some(other.as_str())).and_then(|m| m["priority"].as_i64()).unwrap_or(0);
            let id = a.picked.clone();
            if !a.snapshot(&format!("before {id} moved below {other}")) {
                return Task::none();
            }
            return a.org(&["move", &id, &prio.to_string()]);
        }
        Msg::HideFile(f) => {
            let id = a.picked.clone();
            return a.org(&["hide", &id, &f]);
        }
        Msg::PluginToggle(n, on) => return a.org(&[if on { "plugin-enable" } else { "plugin-disable" }, &n]),
        Msg::Install(path) => {
            if !a.snapshot(&format!("before {} went in", std::path::Path::new(&path).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default())) {
                return Task::none();
            }
            a.waiting.retain(|w| *w != path);
            let Some(id) = a.inst_id() else {
                let g = a.pick.clone();
                return Task::perform(back::call("stage", json!({"id": g, "path": path})), Msg::Installed);
            };
            let mut args = vec![id, "install".into(), path];
            if a.if_in == 1 {
                args.push("--replace".into());
            } else if a.if_in == 2 {
                args.push("--merge".into());
            }
            return Task::perform(back::organizer(args), Msg::Installed);
        }
        Msg::PickFile => return Task::perform(k::pick_file("Add a mod", &["*.7z", "*.zip", "*.rar", "*.fomod"]), Msg::Picked),
        Msg::Picked(p) => {
            if let Some(p) = p {
                a.page = Page::Downloads;
                return update(a, Msg::Install(p));
            }
        }
        Msg::Addr(s) => a.addr = s,
        Msg::AddrGo => {
            let link = a.addr.trim().to_string();
            if link.starts_with("nxm://") {
                return Task::perform(back::call("download_nxm", json!({"link": link})), Msg::Said);
            }
            return Task::perform(back::call("nexus_files", json!({"url": link})), Msg::Said);
        }
        Msg::Installed(r) => match r {
            Ok(v) if v["needs"] == "fomod-eval" => {
                a.fomod["evaluation"] = v["evaluation"].clone();
                a.choices = serde_json::from_value(v["evaluation"]["choices"].clone()).unwrap_or_default();
                return preview(a);
            }
            Ok(v) if v["needs"] == "fomod" => {
                a.fomod = v;
                a.fomod_step = 0;
                a.choices = serde_json::from_value(a.fomod["evaluation"]["choices"].clone()).unwrap_or_default();
                let _ = back::write_own("fomod-open.json", &a.fomod);
                return preview(a);
            }
            Ok(v) => {
                a.fomod = Value::Null;
                let _ = back::write_own("fomod-open.json", &json!({}));
                return update(a, Msg::Said(Ok(v)));
            }
            Err(e) => a.said = e,
        },
        Msg::FomodStep(i) => a.fomod_step = i,
        Msg::FomodPick(s, g, p) => {
            let grp = &a.fomod["fomod"]["steps"][s]["groups"][g];
            let kind = grp["type"].as_str().or(grp["kind"].as_str()).unwrap_or("").to_string();
            while a.choices.len() <= s {
                a.choices.push(Vec::new());
            }
            while a.choices[s].len() <= g {
                a.choices[s].push(Vec::new());
            }
            let sel = &mut a.choices[s][g];
            if kind.contains("Any") || kind.contains("AtLeastOne") {
                if let Some(i) = sel.iter().position(|x| *x == p) {
                    sel.remove(i);
                } else {
                    sel.push(p);
                }
            } else {
                *sel = vec![p];
            }
            let (id, tok, ch) = (a.inst_id().unwrap_or_default(), a.fomod["token"].as_str().unwrap_or("").to_string(), serde_json::to_string(&a.choices).unwrap_or_default());
            return Task::perform(back::organizer(vec![id, "fomod-eval".into(), tok, "--choices".into(), ch]), |r| Msg::Installed(r.map(|mut v| {
                v["needs"] = json!("fomod-eval");
                v
            })));
        }
        Msg::FomodNext => {
            let n = a.fomod["fomod"]["steps"].as_array().map(|s| s.len()).unwrap_or(0);
            if a.fomod_step + 1 < n {
                a.fomod_step += 1;
            } else {
                let (id, tok, ch) = (a.inst_id().unwrap_or_default(), a.fomod["token"].as_str().unwrap_or("").to_string(), serde_json::to_string(&a.choices).unwrap_or_default());
                let name = a.fomod["name"].as_str().unwrap_or("").to_string();
                let _ = a.snapshot(&format!("before {name} went in"));
                let mut args = vec![id, "install".into(), tok, "--choices".into(), ch, "--name".into(), name];
                if a.if_in == 1 {
                    args.push("--replace".into());
                } else if a.if_in == 2 {
                    args.push("--merge".into());
                }
                return Task::perform(back::organizer(args), Msg::Installed);
            }
        }
        Msg::FomodBack => a.fomod_step = a.fomod_step.saturating_sub(1),
        Msg::FomodCancel => {
            let (id, tok) = (a.inst_id().unwrap_or_default(), a.fomod["token"].as_str().unwrap_or("").to_string());
            a.fomod = Value::Null;
            let _ = back::write_own("fomod-open.json", &json!({}));
            return Task::perform(back::organizer(vec![id, "cancel".into(), tok]), |_| Msg::Tick);
        }
        Msg::IfIn(i) => {
            a.if_in = i;
            return preview(a);
        }
        Msg::InstallAll => {
            let w = a.waiting.clone();
            return Task::batch(w.into_iter().map(|p| Task::done(Msg::Install(p))));
        }
        Msg::Downloads(r) => {
            if let Ok(v) = r {
                // finished archives wait to go in
                for i in v["items"].as_array().cloned().unwrap_or_default() {
                    if i["state"] == "done" {
                        if let Some(f) = i["path"].as_str().or(i["file"].as_str()) {
                            if !a.waiting.iter().any(|w| w == f) {
                                a.waiting.push(f.to_string());
                            }
                        }
                    }
                }
                a.downloads = v;
            }
        }
        Msg::FindQ(s) => a.find_q = s,
        Msg::FindGo => {
            a.hits.clear();
            a.counts.clear();
            a.find_pick = 0;
            let q = a.find_q.trim().to_string();
            if q.is_empty() {
                return Task::none();
            }
            let games: Vec<(String, String)> = a.games.iter().map(|g| (g.title.clone(), g.lib["steamApp"].as_u64().map(|x| x.to_string()).unwrap_or_default())).collect();
            return Task::batch(sources::SOURCES.iter().map(|s| {
                let (q, games) = (q.clone(), games.clone());
                let id = s.0;
                Task::perform(sources::search(id, q, games), move |r| Msg::Found(id, r))
            }));
        }
        Msg::Found(src, r) => {
            match r {
                Ok(h) => {
                    a.counts.push((src, Ok(h.len())));
                    a.hits.extend(h);
                }
                Err(e) => a.counts.push((src, Err(e))),
            }
            let mine: Vec<String> = a.mods().iter().filter_map(|m| m["name"].as_str().map(|s| s.to_lowercase())).collect();
            for h in a.hits.iter_mut() {
                h.check(&mine);
            }
            a.hits.sort_by(|x, y| y.endorsed.max(y.downloads / 50).cmp(&x.endorsed.max(x.downloads / 50)));
            if a.counts.len() == sources::SOURCES.len() {
                return details(a);
            }
        }
        Msg::FindSrc(i) => {
            a.find_src = i;
            a.find_pick = 0;
        }
        Msg::FindFlag(i) => a.find_flags[i as usize] = !a.find_flags[i as usize],
        Msg::FindPick(i) => {
            a.find_pick = i;
            a.find_file = 0;
            return details(a);
        }
        Msg::FindFile(i) => a.find_file = i,
        Msg::FindInstall => {
            let Some(h) = pages::shown_hits(a).get(a.find_pick).cloned() else { return Task::none() };
            return match h.source {
                "nexus" => {
                    let (d, m, f, g) = (h.domain.clone(), h.id.parse::<u64>().unwrap_or(0), h.files.get(a.find_file).map(|f| f.1).unwrap_or(0), a.pick.clone());
                    Task::perform(back::work(move || if f == 0 { more::newest_main(&d, m).and_then(|f| sync::nexus_get(d, m, f, g)) } else { sync::nexus_get(d, m, f, g) }), Msg::Said)
                }
                _ => Task::perform(sources::fetch(h, a.find_file), Msg::Picked),
            };
        }
        Msg::Endorse => {
            if let Some(h) = pages::shown_hits(a).get(a.find_pick).cloned() {
                return Task::perform(back::work(move || endorse(&h)), Msg::Endorsed);
            }
        }
        Msg::Open(u) => run_open(&u),
        Msg::Sets(r) => {
            if let Ok(v) = r {
                if a.page == Page::Modlists {
                    a.modlists["jobs"] = v;
                } else {
                    a.sets = v;
                }
            }
        }
        Msg::SetInstall(i) => {
            let list = a.sets["lists"].as_array().or(a.sets.as_array()).cloned().unwrap_or_default();
            if let Some(l) = list.get(i) {
                let title = l["title"].as_str().unwrap_or("").to_string();
                let game = l["game"].as_str().unwrap_or("").to_string();
                return Task::perform(back::call("wj_defaults", json!({"title": title, "game": game})), Msg::Said);
            }
        }
        Msg::CheckNow => {
            a.said = "Checking every mod's page…".into();
            let mods = a.mods();
            let game = a.game().map(|g| g.title.clone()).unwrap_or_default();
            let domain = hearth_gaming::nexus::STEAM_DOMAINS.iter().find(|d| Some(d.0) == a.game().and_then(|g| g.lib["steamApp"].as_u64())).map(|d| d.1.to_string()).unwrap_or_default();
            let key = a.pick.clone();
            return Task::perform(back::work(move || sources::check_updates(&key, &game, &domain, &mods)), Msg::Checked);
        }
        Msg::Checked(r) => match r {
            Ok(v) => {
                let mut all = a.updates.clone();
                all["checked"] = json!(now());
                let game = v["game"].as_str().unwrap_or("").to_string();
                let mut list: Vec<Value> = all["updates"].as_array().cloned().unwrap_or_default().into_iter().filter(|u| u["game"].as_str() != Some(game.as_str())).collect();
                list.extend(v["updates"].as_array().cloned().unwrap_or_default());
                all["updates"] = json!(list);
                if let Some(g) = v["gone"].as_array() {
                    all["gone"] = json!(g);
                }
                a.said = v["said"].as_str().unwrap_or("").to_string();
                let _ = back::write_own("updates.json", &all);
                a.updates = all;
            }
            Err(e) => a.said = e,
        },
        Msg::UpdTab(i) => a.upd_tab = i,
        Msg::UpdTick(id) => {
            if let Some(list) = a.updates["updates"].as_array_mut() {
                for u in list.iter_mut() {
                    if u["id"].as_str() == Some(id.as_str()) {
                        u["picked"] = json!(!u["picked"].as_bool().unwrap_or(false));
                    }
                }
            }
            let _ = back::write_own("updates.json", &a.updates);
        }
        Msg::UpdPick(id) => a.upd_pick = id,
        Msg::UpdateApply => {
            let picked: Vec<Value> = a.updates["updates"].as_array().cloned().unwrap_or_default().into_iter().filter(|u| u["picked"] == true).collect();
            if picked.is_empty() {
                return Task::none();
            }
            if !a.snapshot(&format!("before {} update{}", picked.len(), if picked.len() == 1 { "" } else { "s" })) {
                return Task::none();
            }
            let t: Vec<Task<Msg>> = picked
                .into_iter()
                .filter(|u| u["source"] == "nexus")
                .map(|u| {
                    let (d, m, f, g) = (u["domain"].as_str().unwrap_or("").to_string(), u["modId"].as_u64().unwrap_or(0), u["fileId"].as_u64().unwrap_or(0), a.pick.clone());
                    Task::perform(back::work(move || sync::nexus_get(d, m, f, g)), Msg::Said)
                })
                .collect();
            a.said = "The new files are downloading; each goes in from Downloads, replacing the old one, which is kept as a version (go back one mod any time).".into();
            a.if_in = 1;
            return Task::batch(t);
        }
        Msg::SnapPick(i) => a.snap_pick = i,
        Msg::TakeSnap => {
            if a.detail.organizer {
                match logic::take(&a.inst_dir(), &a.profile_dir(), &a.mods_dir(), "you took it by hand", "", true, now()) {
                    Ok(_) => a.said = "A snapshot of tonight's setup is kept.".into(),
                    Err(e) => a.said = e,
                }
                return a.reload_detail();
            }
        }
        Msg::GoBack => {
            if let Some(s) = a.detail.snaps.get(a.snap_pick).cloned() {
                match logic::restore(&s, &a.inst_dir(), &a.profile_dir(), &a.mods_dir(), now()) {
                    Ok(_) => a.said = format!("Back to {}. Tonight's setup is a snapshot too.", pages::snap_name(&s)),
                    Err(e) => a.said = e,
                }
                return a.reload_detail();
            }
        }
        Msg::CheckEvery(i) => {
            a.save_setting("checkUpdates", json!(["never", "night", "open"][i as usize]));
            if let Err(e) = more::nightly(i == 1) {
                a.said = format!("The nightly check couldn't be set up: {e}");
            }
        }
        Msg::Setting(key, on) => a.save_setting(key, json!(on)),
        Msg::CrashPick(i) => a.crash_pick = i,
        Msg::CrashTab(i) => a.crash_tab = i,
        Msg::CopyLog => {
            if let Some((_, c, _)) = a.detail.crashes.get(a.crash_pick) {
                return iced::clipboard::write(c.lines.join("\n"));
            }
        }
        Msg::Note(s) => a.note = s,
        Msg::SendLogs => {
            let c = a.detail.crashes.get(a.crash_pick).map(|(f, c, _)| format!("{} · {} · {}", f.display(), c.exception, c.suspects.join(", "))).unwrap_or_default();
            let note = format!("Mod Manager · {} · {} · {}", a.game().map(|g| g.title.clone()).unwrap_or_default(), a.note, c);
            a.said = "Sending logs…".into();
            return Task::perform(hearth_ui::run::spawn(vec!["hearth-send-logs".into(), note.chars().take(3000).collect()]), Msg::Sent);
        }
        Msg::Sent(o) => a.said = if o.ok { format!("Sent. {}", o.out.lines().last().unwrap_or("").trim()) } else { format!("Couldn't send the logs: {}", o.why()) },
        Msg::FixStep(i) => return pages::fix_step(a, i),
        Msg::AskText(s) => a.ask = s,
        Msg::Suggest(s) => {
            a.ask = s;
            return update(a, Msg::Ask);
        }
        Msg::Ask => {
            let words = a.ask.trim().to_string();
            if words.is_empty() || a.asking {
                return Task::none();
            }
            a.asking = true;
            let mut talk = a.talk["talk"].as_array().cloned().unwrap_or_default();
            talk.push(json!({"you": words}));
            a.talk["talk"] = json!(talk);
            a.ask.clear();
            let ctx = sources::ai_context(a);
            let t = a.talk.clone();
            return Task::perform(back::work(move || sources::ask_ai(&ctx, &t)), Msg::Asked);
        }
        Msg::Asked(r) => {
            a.asking = false;
            match r {
                Ok(v) => {
                    let mut talk = a.talk["talk"].as_array().cloned().unwrap_or_default();
                    talk.push(json!({"ai": v["reply"]}));
                    a.talk["talk"] = json!(talk);
                    if v["groups"].is_array() {
                        a.talk["set"] = v.clone();
                    }
                }
                Err(e) => a.said = e,
            }
        }
        Msg::Speak => {
            if more::listening() {
                a.said = "Turning your words into text…".into();
                return Task::perform(back::work(more::speak_stop), Msg::Heard);
            }
            match more::speak_start() {
                Ok(()) => a.said = "Listening… press Speak again when you're done.".into(),
                Err(e) => a.said = e,
            }
        }
        Msg::Heard(r) => match r {
            Ok(w) => {
                a.ask = w;
                a.said.clear();
            }
            Err(e) => a.said = e,
        },
        Msg::InstallSet => return pages::install_set(a),
        Msg::SaveLater => match back::write_own("describe.json", &a.talk) {
            Ok(()) => a.said = "Kept: it's here when you come back, nothing installed.".into(),
            Err(e) => a.said = e,
        },
        Msg::CheckAll => return Task::perform(back::work(back::load_world), Msg::World),
        Msg::SetUp(key) => return Task::perform(back::organizer(vec!["create".into(), key]), Msg::Said),
        Msg::HideGame(key) => {
            let mut h: Vec<Value> = a.settings["hidden"].as_array().cloned().unwrap_or_default();
            h.push(json!(key));
            a.save_setting("hidden", json!(h));
        }
        Msg::KeyText(s) => a.key_text = s,
        Msg::KeyFor(s) => {
            a.key_for = s;
            a.key_text.clear();
        }
        Msg::SaveKey => {
            let (src, key) = (a.key_for, a.key_text.trim().to_string());
            a.key_text.clear();
            a.key_for = "";
            if key.is_empty() {
                return Task::none();
            }
            if src == "nexus" {
                return Task::perform(back::call("nexus_sign_in", json!({"key": key})), Msg::Said);
            }
            return Task::perform(sources::keep_key(src, key), Msg::Keys);
        }
        Msg::Keys(v) => a.keys = v,
        Msg::Modlists(r) => {
            if let Ok(v) = r {
                a.modlists["gallery"] = v;
            }
        }
        Msg::Devices(d) => a.devices = d,
        Msg::Server(x) => a.server = x,
        Msg::Pictures => {}
        Msg::Inbox(reqs) => return inbox(a, reqs),
        Msg::SendGame(to) => {
            a.pop = None;
            let Some(g) = a.game().cloned() else { return Task::none() };
            let read = |f: &str| std::fs::read_to_string(a.profile_dir().join(f)).unwrap_or_default();
            let files: Vec<Value> = a.mods().iter().filter(|m| m["kind"] == "mod" && m["enabled"] == true).map(|m| json!({"mod": m["id"], "file": m["installedFrom"], "nexusId": m["nexusId"], "version": m["version"]})).collect();
            let req = json!({"kind": "profile", "game": g.title, "profile": a.profile(), "modlist": read("modlist.txt"), "plugins": read("plugins.txt"), "loadorder": read("loadorder.txt"), "mods": files});
            a.said = sync::send(&to, req).unwrap_or_else(|e| e);
        }
        Msg::AlsoAdd => {
            let Some(h) = pages::shown_hits(a).get(a.find_pick).cloned() else { return Task::none() };
            let devs: Vec<String> = sync::with_game(&a.devices, &h.game).iter().filter_map(|d| d["name"].as_str().map(str::to_string)).collect();
            let mut said = Vec::new();
            for d in &devs {
                let req = json!({"kind": "add", "game": h.game, "source": h.source, "id": h.id, "domain": h.domain, "name": h.name, "fileId": h.files.get(a.find_file).map(|f| f.1).unwrap_or(0), "url": h.files.get(a.find_file).map(|f| f.2.clone()).unwrap_or_default()});
                said.push(sync::send(d, req).unwrap_or_else(|e| e));
            }
            a.said = if said.is_empty() { "None of your other devices has this game modded.".into() } else { said.join(" ") };
        }
        Msg::Measure => {
            let Some(g) = a.game().cloned() else { return Task::none() };
            let to = a.settings["describeFor"].as_str().unwrap_or("the ROG Ally").trim_start_matches("the ").to_string();
            a.said = sync::send(&to, json!({"kind": "measure", "game": g.title, "profile": a.profile()})).unwrap_or_else(|e| e);
        }
        Msg::SideMod(game, m) => {
            a.search.clear();
            if a.pick != game {
                a.picked = m;
                a.pick = game;
                a.page = Page::Game;
                return a.reload_detail();
            }
            a.page = Page::Game;
            return update(a, Msg::Pick(m));
        }
        Msg::Accounts(r) => match r {
            Ok(v) if v.is_array() => a.accounts = v,
            Ok(_) => return Task::perform(back::call("accounts_list", Value::Null), Msg::Accounts),
            Err(e) => a.said = e,
        },
        Msg::SignIn(p) => {
            a.said = format!("Sign in to {} in the browser that opened; the Mod Manager waits for it.", hearth_gaming::accounts::label(p));
            return Task::perform(back::call("account_begin", json!({"id": p})), Msg::Accounts);
        }
        Msg::SignOut(p) => return Task::perform(back::call("account_sign_out", json!({"id": p})), |_| Msg::WjTick),
        Msg::MegaEmail(s2) => a.mega.0 = s2,
        Msg::MegaPass(s2) => a.mega.1 = s2,
        Msg::MegaGo => {
            let (e, pw) = (a.mega.0.trim().to_string(), std::mem::take(&mut a.mega.1));
            return Task::perform(back::call("mega_sign_in", json!({"email": e, "password": pw})), Msg::Accounts);
        }
        Msg::DeployMode(m) => {
            if a.detail.organizer {
                return a.org(&["deploy-mode", m]);
            }
            let g = a.pick.clone();
            return Task::perform(back::call("set_deploy", json!({"id": g, "mode": m})), Msg::Said);
        }
        Msg::WjTick => return Task::batch([Task::perform(back::call("wj_jobs", Value::Null), Msg::Sets), Task::perform(back::call("accounts_list", Value::Null), Msg::Accounts)]),
        Msg::UpdateExt => {
            let kind = a.inst()["game"].as_str().unwrap_or("").to_string();
            let Some((d, m)) = more::extender_page(&kind) else { return Task::none() };
            if !a.snapshot("before SKSE was updated") {
                return Task::none();
            }
            let g = a.pick.clone();
            a.said = "The new script extender is coming from its page: it goes in as a mod (Downloads › Install).".into();
            return Task::perform(back::work(move || more::newest_main(d, m).and_then(|f| sync::nexus_get(d.to_string(), m, f, g))), Msg::Said);
        }
        Msg::MakePatch => return make_patch(a),
        Msg::Patched(r) => match r {
            Ok(w) => {
                a.said = w;
                return Task::batch([a.reload_detail(), Task::perform(back::work(back::load_world), Msg::World)]);
            }
            Err(e) => a.said = e,
        },
        Msg::ReadMe => {
            let tok = a.fomod["token"].as_str().unwrap_or("").to_string();
            match more::readme(&a.inst_dir().join(".hearth/staging").join(tok).join("x")) {
                Some(f) => run_open(&f.to_string_lossy()),
                None => a.said = "This mod has no read me in it: its page has the author's words (Mod page).".into(),
            }
        }
        Msg::Before(b) => a.before = *b,
        Msg::FindPop(w) => a.pop = if a.pop == Some(w) { None } else { Some(w) },
        Msg::FindGame(g) => {
            a.pop = None;
            a.find_game = g;
            a.find_pick = 0;
        }
        Msg::FindOrder(i) => {
            a.pop = None;
            a.find_order = i;
            a.find_pick = 0;
        }
        Msg::Needs(id, needs, files, clash) => {
            let mine: Vec<String> = a.mods().iter().filter_map(|m| m["name"].as_str().map(|s| s.to_lowercase())).collect();
            for h in a.hits.iter_mut().filter(|h| h.id == id) {
                h.needs = needs.clone();
                h.preview = files.clone();
                h.clash = clash.clone();
                h.check(&mine);
            }
        }
        Msg::Endorsed(r) => a.said = match r {
            Ok(v) => v["message"].as_str().map(str::to_string).unwrap_or_else(|| "Endorsed: thank you sent to its author.".into()),
            Err(e) => e,
        },
        Msg::FindLike(name) => {
            a.page = Page::Find;
            a.find_q = name.split_whitespace().filter(|w| w.len() > 3).take(2).collect::<Vec<_>>().join(" ");
            return update(a, Msg::FindGo);
        }
        Msg::KeepSnaps(i) => a.save_setting("keepSnapshots", json!([30, 60, 90][i as usize])),
        Msg::Versions(m, r) => {
            if let Ok(v) = r {
                a.versions.insert(m, v["versions"].as_array().cloned().unwrap_or_default());
            }
        }
        Msg::UseVersion(m, f) => {
            if !a.snapshot(&format!("before {m} went back to {f}")) {
                return Task::none();
            }
            a.versions.remove(&m);
            return a.org(&["version-use", &m, &f]);
        }
        Msg::TestStart => {
            let Some(id) = a.inst_id() else { return Task::none() };
            a.testing = true;
            a.said = "Test start: the game starts with the mods; in 60 seconds you're told whether it held.".into();
            let roots = back::crash_roots(a.inst());
            let since = now();
            return Task::batch([update(a, Msg::Play), Task::perform(back::work(move || more::test_start_result(&id, since, &roots)), Msg::Tested)]);
        }
        Msg::Tested(r) => {
            a.testing = false;
            if let (Ok(_), Some((f, _, _))) = (&r, a.detail.crashes.get(a.crash_pick)) {
                more::mark_fix(f, "fixed", "a test start passed");
            }
            a.said = r.unwrap_or_else(|e| e);
        }
        Msg::GetCrashLogger => {
            let kind = a.inst()["game"].as_str().unwrap_or("").to_string();
            let Some((d, m, name)) = more::crash_logger_page(&kind) else {
                a.said = "This game has no crash logger Hearth knows.".into();
                return Task::none();
            };
            let g = a.pick.clone();
            a.said = format!("Getting {name} from its page: it waits in Downloads to go in.");
            return Task::perform(back::work(move || more::newest_main(d, m).and_then(|f| sync::nexus_get(d.to_string(), m, f, g))), Msg::Said);
        }
        Msg::Drag => return win::drag(),
        Msg::Max => return win::toggle_maximize(),
        Msg::Min => return win::minimize(),
        Msg::WClose => return win::close(),
        Msg::Resize(d) => return win::resize(d),
    }
    Task::none()
}

/// Describe it's pictures: the game's own (Now) and the set's first mod's from its Nexus Mods page (With the set).
fn pictures(a: &App) -> Task<Msg> {
    let first = a.talk["set"]["groups"][0]["mods"][0]["name"].as_str().unwrap_or("").to_string();
    let g = a.game().cloned();
    let Some(g) = g else { return Task::none() };
    Task::perform(
        back::work(move || {
            let dir = back::own_dir().join("pictures");
            let _ = std::fs::create_dir_all(&dir);
            let get = |url: &str, to: &std::path::Path| {
                let _ = std::process::Command::new("curl").args(["-fsSL", "-m", "30", "-o"]).arg(to).arg(url).status();
            };
            if let Ok(Value::String(art)) = hearth_gaming::mods_app::route("art", &json!({"id": g.key, "kind": "hero"})) {
                if let Some(b64) = art.split_once("base64,").map(|x| x.1) {
                    use std::io::Write;
                    let out = std::process::Command::new("base64").arg("-d").stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped()).spawn().and_then(|mut c| {
                        if let Some(mut i) = c.stdin.take() {
                            let _ = i.write_all(b64.as_bytes());
                        }
                        c.wait_with_output()
                    });
                    if let Ok(o) = out {
                        let _ = std::fs::write(dir.join("now.jpg"), o.stdout);
                    }
                } else if art.starts_with("http") {
                    get(&art, &dir.join("now.jpg"));
                }
            }
            let domain = hearth_gaming::nexus::STEAM_DOMAINS.iter().find(|d| Some(d.0) == g.lib["steamApp"].as_u64()).map(|d| d.1);
            if let (Some(acct), Some(domain), false) = (hearth_gaming::nexus::account(), domain, first.is_empty()) {
                let v = hearth_gaming::nexus::search(&acct.key, domain, &first).unwrap_or(Value::Null);
                let pic = v["mods"].as_array().or(v.as_array()).and_then(|m| m.first()).and_then(|m| m["pictureUrl"].as_str().or(m["picture_url"].as_str())).unwrap_or("").to_string();
                if !pic.is_empty() {
                    get(&pic, &dir.join("set.jpg"));
                }
            }
        }),
        |_| Msg::Pictures,
    )
}

/// The picked result's requirements and files, read from its page.
fn details(a: &App) -> Task<Msg> {
    let Some(h) = pages::shown_hits(a).get(a.find_pick).cloned() else { return Task::none() };
    if !h.preview.is_empty() || !h.needs.is_empty() {
        return Task::none();
    }
    let (inst, mods, mods_dir) = (a.inst().clone(), a.mods(), a.mods_dir());
    let mine = a.game().map(|g| g.title == h.game).unwrap_or(false);
    Task::perform(
        async move {
            let key = if h.source == "modio" { hearth_oss::secret::lookup("hearth-mods", "modio").await } else { String::new() };
            back::work(move || {
                let own = if mine { more::owners(&inst, &mods, &mods_dir).0 } else { Default::default() };
                let id = h.id.clone();
                let (n, f, c) = sources::details(h, key, own);
                (id, n, f, c)
            })
            .await
        },
        |(id, n, f, c)| Msg::Needs(id, n, f, c),
    )
}

/// This device's note for your other devices (its games, updates waiting, crashes, fps).
fn publish(a: &App) -> Task<Msg> {
    let games: Vec<Value> = a
        .games
        .iter()
        .filter(|g| g.mods > 0)
        .map(|g| {
            let profile = if a.game().map(|x| x.key == g.key).unwrap_or(false) { a.profile() } else { g.inst.as_ref().and_then(|i| i["profile"].as_str()).unwrap_or("").to_string() };
            json!({"title": g.title, "key": g.key, "mods": g.mods, "on": g.on, "profile": profile})
        })
        .collect();
    let ups: Vec<Value> = a.updates["updates"].as_array().cloned().unwrap_or_default().iter().map(|u| json!({"game": u["game"], "mod": u["mod"], "to": u["to"]})).collect();
    let fx = more::fixes();
    let crashes: Vec<Value> = a.detail.crashes.iter().take(6).map(|(f, c, t)| json!({"at": t, "game": a.game().map(|g| g.title.clone()).unwrap_or_default(), "module": c.suspects.first().cloned().unwrap_or_default(), "what": c.objects.first().cloned().unwrap_or_default(), "fixed": fx[f.to_string_lossy().as_ref()]["state"] == "fixed"})).collect();
    let measured = a.detail.measured.clone();
    Task::perform(back::work(move || sync::publish(json!(games), json!(ups), json!(crashes), measured)), |_| Msg::Pictures)
}

/// What an install would change (board 2's Before it goes in and Needs), worked out off the window's thread.
fn preview(a: &App) -> Task<Msg> {
    if a.fomod["evaluation"].is_null() || !a.detail.organizer {
        return Task::none();
    }
    let staged = a.inst_dir().join(".hearth/staging").join(a.fomod["token"].as_str().unwrap_or("")).join("x");
    let (ev, inst, mods, mods_dir) = (a.fomod["evaluation"].clone(), a.inst().clone(), a.mods(), a.mods_dir());
    let name = a.fomod["name"].as_str().unwrap_or("").to_string();
    let at = if a.if_in != 0 { mods.iter().find(|m| m["name"].as_str() == Some(name.as_str())).and_then(|m| m["priority"].as_i64()) } else { None };
    let have: Vec<String> = a.plugins().iter().filter(|p| p["active"] == true).filter_map(|p| p["name"].as_str().map(str::to_string)).collect();
    let ext = a.detail.ext.0.clone();
    Task::perform(
        back::work(move || {
            let files = more::fomod_files(&staged, &ev);
            let (own, prio) = more::owners(&inst, &mods, &mods_dir);
            Some(more::before(&files, &own, &prio, at, &have, &ext))
        }),
        |b| Msg::Before(Box::new(b)),
    )
}

/// What the other devices asked of this one.
fn inbox(a: &mut App, reqs: Vec<Value>) -> Task<Msg> {
    let mut t = Vec::new();
    let mut said = Vec::new();
    for r in reqs {
        let from = r["from"].as_str().unwrap_or("your other device").to_string();
        match r["kind"].as_str().unwrap_or("") {
            "add" => {
                let Some(g) = a.games.iter().find(|g| g.title.eq_ignore_ascii_case(r["game"].as_str().unwrap_or(""))).map(|g| g.key.clone()) else { continue };
                said.push(format!("{from} asked to add {} here too: it's downloading (once, from the home server when it's there).", r["name"].as_str().unwrap_or("a mod")));
                if r["source"] == "nexus" {
                    let (d, m, f) = (r["domain"].as_str().unwrap_or("").to_string(), r["id"].as_str().and_then(|x| x.parse().ok()).unwrap_or(0u64), r["fileId"].as_u64().unwrap_or(0));
                    t.push(Task::perform(back::work(move || if f == 0 { more::newest_main(&d, m).and_then(|f| sync::nexus_get(d, m, f, g)) } else { sync::nexus_get(d, m, f, g) }), Msg::Said));
                } else {
                    let h = sources::Hit { source: if r["source"] == "thunderstore" { "thunderstore" } else { "modio" }, id: r["id"].as_str().unwrap_or("").into(), files: vec![(r["name"].as_str().unwrap_or("").to_string(), r["fileId"].as_u64().unwrap_or(0), r["url"].as_str().unwrap_or("").to_string())], ..Default::default() };
                    t.push(Task::perform(sources::fetch(h, 0), |p| Msg::Downloads(Ok(json!({"items": p.map(|f| vec![json!({"state": "done", "path": f})]).unwrap_or_default()})))));
                }
            }
            "profile" => {
                let game = r["game"].as_str().unwrap_or("").to_string();
                let Some(g) = a.games.iter().find(|g| g.title.eq_ignore_ascii_case(&game)).cloned() else {
                    said.push(format!("{from} sent {game}'s mods, but {game} isn't on this device."));
                    continue;
                };
                let Some(id) = g.inst_id() else {
                    said.push(format!("{from} sent {game}'s mods: set up mods for it here first (Every game › Set up mods), then ask again."));
                    continue;
                };
                let name = format!("{} (from {from})", r["profile"].as_str().unwrap_or("Profile"));
                let r2 = r.clone();
                said.push(format!("{from} sent {game}'s mods: they're in a new profile, \"{name}\"; the mods this device doesn't have yet are named in it."));
                t.push(Task::perform(
                    back::work(move || {
                        let v = hearth_gaming::mods_app::route("organizer", &json!({"args": [id.clone(), "profile-create", name.clone()]}))?;
                        let dir = std::path::PathBuf::from(v["summary"]["instance"]["dir"].as_str().unwrap_or("")).join("profiles").join(v["created"].as_str().unwrap_or(&name));
                        let mods_dir = std::path::PathBuf::from(v["summary"]["instance"]["modsDir"].as_str().unwrap_or(""));
                        for (f, k2) in [("modlist.txt", "modlist"), ("plugins.txt", "plugins"), ("loadorder.txt", "loadorder")] {
                            if let Some(text) = r2[k2].as_str().filter(|x| !x.is_empty()) {
                                hearth_ui::settings_file::write_text(&dir.join(f), text).map_err(|e| e.to_string())?;
                            }
                        }
                        let missing: Vec<String> = r2["mods"].as_array().cloned().unwrap_or_default().iter().filter_map(|m| m["mod"].as_str()).filter(|m| !mods_dir.join(m).is_dir()).map(str::to_string).collect();
                        Ok(json!({"ok": true, "notes": [if missing.is_empty() { "Every mod it needs is here.".to_string() } else { format!("Still to get here: {}", missing.join(", ")) }]}))
                    }),
                    Msg::Said,
                ));
            }
            "measure" => {
                let _ = back::write_own("measure.json", &r);
                said.push(format!("{from} asked to measure {} here: the next Play modded logs its fps (MangoHud, 60 seconds) and sends it back.", r["game"].as_str().unwrap_or("a game")));
            }
            _ => {}
        }
    }
    if !said.is_empty() {
        a.said = said.join(" ");
    }
    Task::batch(t)
}

/// Board 1's Make a patch with the winners: every plugin that's on, read; the patch made; put in as its own mod, last.
fn make_patch(a: &mut App) -> Task<Msg> {
    let Some(id) = a.inst_id() else { return Task::none() };
    let picked = a.mods().iter().find(|m| m["id"].as_str() == Some(a.picked.as_str())).cloned().unwrap_or(Value::Null);
    let name = picked["name"].as_str().unwrap_or("your mods").to_string();
    let only: Vec<String> = picked["plugins"].as_array().cloned().unwrap_or_default().iter().filter_map(|x| x.as_str().map(str::to_string)).collect();
    let plugins: Vec<(String, std::path::PathBuf)> = a
        .plugins()
        .iter()
        .filter(|p| p["active"] == true)
        .map(|p| {
            let n = p["name"].as_str().unwrap_or("").to_string();
            let from = p["mod"].as_str().or(p["from"].as_str()).unwrap_or("").to_string();
            let path = if from.is_empty() { std::path::PathBuf::from(a.inst()["dataDir"].as_str().unwrap_or("")).join(&n) } else { a.mods_dir().join(&from).join(&n) };
            (n, path)
        })
        .collect();
    let light = hearth_gaming::organizer::kinds::KINDS.iter().find(|k| Some(k.id) == a.inst()["game"].as_str()).map(|k| k.light).unwrap_or(0);
    if !a.snapshot(&format!("before a patch was made for {name}")) {
        return Task::none();
    }
    a.said = "Reading every plugin that's on…".into();
    let work_dir = back::own_dir().join("patch");
    Task::perform(
        back::work(move || {
            let mut ps = Vec::new();
            for (n, p) in &plugins {
                let b = std::fs::read(p).map_err(|e| format!("{n} couldn't be read ({e})"))?;
                ps.push(patch::read(n, &b)?);
            }
            let m = patch::make(&ps, &only, light)?;
            let file = format!("Hearth Patch - {}.esp", hearth_gaming::util::safe_name(&name));
            let _ = std::fs::remove_dir_all(&work_dir);
            std::fs::create_dir_all(&work_dir).map_err(|e| e.to_string())?;
            std::fs::write(work_dir.join(&file), &m.bytes).map_err(|e| e.to_string())?;
            let mod_name = format!("Hearth patch · {name}");
            let v = hearth_gaming::mods_app::route("organizer", &json!({"args": [id.clone(), "install", work_dir.to_string_lossy(), "--name", mod_name.clone(), "--replace"]}))?;
            if v["ok"] == false {
                return Err(v["error"].as_str().unwrap_or("The patch couldn't go in.").to_string());
            }
            let _ = hearth_gaming::mods_app::route("organizer", &json!({"args": [id, "plugin-enable", file.clone()]}));
            Ok(format!("{file} is in, last: {} records with both sides' changes ({} parts brought back{}), from {} plugins. Undo: turn \"{mod_name}\" off.", m.records, m.parts, if m.kept > 0 { format!(", {} left as the winner has them", m.kept) } else { String::new() }, m.masters.len()))
        }),
        Msg::Patched,
    )
}

/// Endorse on Nexus Mods (its own API, your key): thanks its author, as the button on its page does.
fn endorse(h: &sources::Hit) -> Result<Value, String> {
    if h.source != "nexus" {
        return Err("Only Nexus Mods has endorsements: this one's page has its own way to thank the author.".into());
    }
    let acct = hearth_gaming::nexus::account().ok_or("Sign in to Nexus Mods (Accounts) first.")?;
    let url = format!("{}/games/{}/mods/{}/endorse.json", hearth_gaming::nexus::API, h.domain, h.id);
    let (code, body) = hearth_gaming::accounts::curl_post(&url, &[("apikey", acct.key.as_str()), ("Application-Name", "Hearth Mods"), ("Content-Type", "application/x-www-form-urlencoded")], &format!("version={}", hearth_gaming::nexus::urlencode(&h.version)), 20)?;
    let v: Value = serde_json::from_str(&body).unwrap_or(json!({"message": body}));
    if code >= 400 {
        return Err(v["message"].as_str().map(|m| format!("Nexus Mods said: {m}")).unwrap_or_else(|| format!("Nexus Mods said no (HTTP {code}).")));
    }
    Ok(json!({"message": format!("Endorsed {} on Nexus Mods.", h.name)}))
}

fn run_open(url: &str) {
    if url.is_empty() {
        return;
    }
    hearth_ui::run::detached(&["xdg-open", url]);
}

// ------------------------------------------------------------------ the side

fn side(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let search: E<'_, Msg> = k::input(&p, "⌕ Search mods, games, files", &a.search, Msg::Search).into();
    let counts = |pg: Page| -> String {
        match pg {
            Page::Downloads => a.downloads["items"].as_array().map(|i| i.iter().filter(|x| x["state"] != "done").count() + a.waiting.len()).filter(|n| *n > 0).map(|n| n.to_string()).unwrap_or_default(),
            Page::Updates => a.updates["updates"].as_array().map(|u| u.len()).filter(|n| *n > 0).map(|n| n.to_string()).unwrap_or_default(),
            Page::Crashes => Some(a.detail.crashes.len()).filter(|n| *n > 0).map(|n| n.to_string()).unwrap_or_default(),
            Page::Organizer => a.instances["instances"].as_array().map(|i| i.len()).filter(|n| *n > 0).map(|n| n.to_string()).unwrap_or_default(),
            _ => String::new(),
        }
    };
    let q = a.search.to_lowercase();
    let mut nav = Column::new().spacing(1.0);
    for (pg, label) in SECTIONS {
        let c = counts(*pg);
        let cc = if *pg == Page::Crashes { k::RED } else { k::BLUE };
        let inner = row![container(Space::new()).width(Length::Fixed(14.0)).height(Length::Fixed(14.0)).style(ui::boxed(Pal::mix(p.panel2, p.text, 0.25), 3.0, None)), t(*label, 13.0, p.text), k::fill(), t(c, 11.0, cc)].spacing(10.0).align_y(iced::Center);
        nav = nav.push(k::side_row(&p, inner, a.page == *pg, 20.0, Msg::Go(*pg)));
    }
    let mut games = Column::new().spacing(1.0);
    for g in a.games.iter().filter(|g| q.is_empty() || g.title.to_lowercase().contains(&q)) {
        let dot: E<'_, Msg> = match g.dot {
            1 => k::dot(k::YELLOW, 7.0),
            2 => k::dot(k::BLUE, 7.0),
            _ => k::gap(7.0, 1.0),
        };
        let inner = row![pages::thumb(&g.title, 22.0), t(ui::fit(&g.title, 20), 13.0, p.text), k::fill(), dot, t(if g.mods > 0 { g.mods.to_string() } else { String::new() }, 11.0, p.muted)].spacing(8.0).align_y(iced::Center);
        games = games.push(k::side_row(&p, inner, a.page == Page::Game && a.pick == g.key, 24.0, Msg::PickGame(g.key.clone())));
    }
    if a.loading {
        games = games.push(k::mu(&p, "Looking for your games…", 12.0));
    }
    if q.len() >= 2 {
        let key = a.game().map(|g| g.key.clone()).unwrap_or_default();
        let found: Vec<Value> = a.mods().into_iter().filter(|m| m["kind"] == "mod" && m["name"].as_str().map(|n| n.to_lowercase().contains(&q)).unwrap_or(false)).take(6).collect();
        if !found.is_empty() {
            games = games.push(k::gap(1.0, 6.0)).push(k::cap(&p, format!("Mods in {}", a.game().map(|g| g.short()).unwrap_or_default())));
            for m in found {
                let id = m["id"].as_str().unwrap_or("").to_string();
                games = games.push(k::side_row(&p, row![t(ui::fit(m["name"].as_str().unwrap_or(""), 26), 12.0, p.text), k::fill(), t(m["version"].as_str().unwrap_or("").to_string(), 10.0, p.muted)].spacing(6.0), false, 22.0, Msg::SideMod(key.clone(), id)));
            }
        }
        let files: Vec<(String, String)> = a.lens["files"].as_array().cloned().unwrap_or_default().iter().filter_map(|f| f["path"].as_str().map(str::to_string)).filter(|f| f.to_lowercase().contains(&q)).take(4).map(|f| (f, a.picked.clone())).chain(a.waiting.iter().filter(|w| w.to_lowercase().contains(&q)).take(3).map(|w| (w.clone(), String::new()))).collect();
        if !files.is_empty() {
            games = games.push(k::gap(1.0, 6.0)).push(k::cap(&p, "Files"));
            for (f, m) in files {
                let name = f.rsplit('/').next().unwrap_or(&f).to_string();
                let go = if m.is_empty() { Msg::Go(Page::Downloads) } else { Msg::SideMod(key.clone(), m) };
                games = games.push(k::side_row(&p, row![t(ui::fit(&name, 30), 11.0, p.text)], false, 20.0, go));
            }
        }
    }
    let store = column![row![k::dot(p.accent, 7.0), t("Updates in the Hearth Store", 12.0, p.text)].spacing(6.0).align_y(iced::Center), k::mu(&p, "our Mod Manager is its own Store app", 11.0)].spacing(2.0);
    let edit = row![k::mu1::<Msg>(&p, "Made of blocks ·", 11.0), k::click(t("Edit layout", 11.0, p.accent), Some(Msg::Describe(hearth_ui::arrange::edit_layout())))].spacing(4.0).align_y(iced::Center);
    let body = hearth_ui::arrange::parts(
        &a.describe,
        &p,
        "side",
        vec![("search", search), ("sections", column![k::cap(&p, "Mod Manager"), k::gap(1.0, 6.0), nav].into()), ("games", column![k::cap(&p, "Your games"), k::gap(1.0, 6.0), scrollable(games).height(Length::Fill)].height(Length::Fill).into()), ("store", store.into()), ("edit", edit.into())],
        |ps| column![ps.take("search"), k::gap(1.0, 12.0), ps.take("sections"), k::gap(1.0, 14.0), ps.take("games"), k::gap(1.0, 8.0), ps.take("store"), k::gap(1.0, 10.0), ps.take("edit")].into(),
        Msg::Describe,
    );
    k::side(&p, body, 236.0)
}

// ------------------------------------------------------------------ the window

pub fn page_title(a: &App) -> String {
    let g = a.game().map(|g| g.title.clone()).unwrap_or_default();
    match a.page {
        Page::Every => "· Every game".into(),
        Page::Find => "· Find mods".into(),
        Page::Downloads => format!("· {g} · Add mods"),
        Page::Updates => format!("· Updates and going back · {g}"),
        Page::Crashes => format!("· Crashes · {g}"),
        Page::Describe => format!("· Describe it · {g}"),
        Page::Modlists => "· Modlists & collections".into(),
        Page::Organizer => "· Organizer instances".into(),
        Page::Accounts => "· Accounts".into(),
        Page::Settings => "· Settings".into(),
        Page::Game => format!("· {g}"),
    }
}

fn credit(a: &App) -> &'static str {
    match a.page {
        Page::Game => "Hearth Mod Organizer, made to work like Mod Organizer 2 (GPL-3) · LOOT's masterlist (GPL-3) · fuse-overlayfs",
        Page::Downloads => "FOMOD installers read by Hearth Mod Organizer (written anew in Rust, after Mod Organizer 2's installer, GPL-3)",
        Page::Find => "searches Nexus Mods, Thunderstore, CurseForge and mod.io through their own APIs, with your accounts",
        Page::Updates => "changelogs from each mod's own page · snapshots kept beside the instance's Mod Organizer 2 files",
        Page::Crashes => "reads crash logs from Crash Logger SSE / F4SE (community mods) and Proton's own log",
        Page::Describe => "Describe it uses Hearth AI (Colibri, Ollama as the fallback) on your own PC · fps from community reports and your own test starts",
        Page::Every => "Battle for Middle-earth II mods through Beacon · Minecraft through Fabric · Stardew through SMAPI · Valheim through BepInEx",
        _ => "Hearth Mod Organizer (GPL-3) · Wabbajack lists through jackify-engine (GPL-3)",
    }
}

fn view(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    studio_values(a);
    if a.tv {
        return pages::tv(a);
    }
    let mut items: Vec<(&'static str, E<'_, Msg>)> = vec![("side", hearth_ui::studio::made(&a.describe, "window", "side", || side(a))), ("page", pages::page(a))];
    for (id, which) in [("pane:ladder", "ladder"), ("pane:health", "health"), ("pane:updates", "updates")] {
        if a.describe.wants_pane(which) {
            items.push((id, pages::studio_pane(a, which)));
        }
    }
    let mut body = hearth_ui::arrange::parts(&a.describe, &p, "window", items, move |ps| row![ps.take("side"), ui::vline(p.line), ps.take("page")].into(), Msg::Describe);
    if a.pop == Some("more") {
        let opts = vec![
            ("Deploy now (lay the mods over the game)".to_string(), false, Msg::Starts(String::new())),
            ("Open the instance's folder".to_string(), false, Msg::Open(a.inst()["dir"].as_str().unwrap_or("").to_string())),
            ("Look for games again".to_string(), false, Msg::Reload),
            ("Open in Mod Organizer 2 (under Proton)".to_string(), false, Msg::Open(String::new())),
        ];
        body = ui::dialog(body, k::chooser(&p, a.game().map(|g| g.title.clone()).unwrap_or_default(), opts, Msg::More), Some(Msg::More));
    }
    match a.pop {
        Some("games") => {
            let mut opts = vec![("All your games".to_string(), a.find_game.is_empty(), Msg::FindGame(String::new()))];
            for g in a.games.iter().filter(|g| g.mods > 0 || g.inst.is_some()) {
                opts.push((g.title.clone(), a.find_game == g.title, Msg::FindGame(g.title.clone())));
            }
            body = ui::dialog(body, k::chooser(&p, "Search for", opts, Msg::FindPop("games")), Some(Msg::FindPop("games")));
        }
        Some("order") => {
            let opts = pages::ORDERS.iter().enumerate().map(|(i, o)| (o.to_string(), a.find_order == i as u8, Msg::FindOrder(i as u8))).collect();
            body = ui::dialog(body, k::chooser(&p, "Order", opts, Msg::FindPop("order")), Some(Msg::FindPop("order")));
        }
        Some("send") => {
            let g = a.game().map(|g| g.title.clone()).unwrap_or_default();
            let opts = a.devices.iter().filter_map(|d| d["name"].as_str()).map(|n| (format!("{n}: {g}'s {} profile", a.profile()), false, Msg::SendGame(n.to_string()))).collect();
            body = ui::dialog(body, k::chooser(&p, format!("Send {g}'s mods to"), opts, Msg::FindPop("send")), Some(Msg::FindPop("send")));
        }
        _ => {}
    }
    let pill: Option<E<'_, Msg>> = match a.page {
        Page::Describe => Some(k::tag(&p, "Hearth AI · on your Nvidia PC", hearth_oss_kit_a::Tag::Ok)),
        Page::Every => Some(k::tag(&p, if sources::home_server().is_some() { "Home server · connected" } else { "Home server · not set up" }, hearth_oss_kit_a::Tag::Ok)),
        Page::Game => Some(k::tag(&p, sources::sync_words(&a.devices), hearth_oss_kit_a::Tag::Ok)),
        _ => None,
    };
    let bar = k::Bar {
        letters: "MM",
        from: k::rgb("#ffb36b"),
        to: k::rgb("#ff7a2e"),
        title: "Mod Manager",
        line: page_title(a),
        middle: None,
        right: pill.into_iter().collect(),
        msgs: TitleMsgs { drag: Msg::Drag, maximize: Msg::Max, minimize: Some(Msg::Min), close: Msg::WClose },
        open: Msg::Describe(hearth_ui::describe::DMsg::Open),
    };
    k::window(&p, &a.describe, bar, body, ("Mod Manager", credit(a)), Msg::Resize, Msg::Describe)
}

// ------------------------------------------------------------------ blocks

use hearth_ui::blocks::{bl, bls, PageDef};
/// Mod Manager's blocks (hearth_ui::blocks): the window, the side, and each board's page.
pub const BLOCKS: &[PageDef] = &[
    PageDef { id: "window", title: "Mod Manager", blocks: &[bl("side", "The side", 0, 0, 2, 12), bl("page", "The page", 2, 0, 10, 12)] },
    PageDef { id: "side", title: "The side", blocks: &[bl("search", "Search mods, games, files", 0, 0, 12, 1), bl("sections", "Mod Manager's sections", 0, 1, 12, 4), bls("games", "Your games", 0, 5, 12, 5), bl("store", "Updates in the Hearth Store", 0, 10, 12, 1), bl("edit", "Made of blocks · Edit layout", 0, 11, 12, 1)] },
    PageDef {
        id: "game",
        title: "A game's mods",
        blocks: &[bl("hero", "The game: Play modded, Add to Steam, Add mods", 0, 0, 12, 2), bls("ladder", "Mods (the priority ladder)", 0, 2, 5, 10), bl("health", "Health", 5, 2, 4, 4), bl("lens", "Conflict lens", 5, 6, 4, 6), bl("plugins", "Plugins · load order", 9, 2, 3, 3), bl("sort", "Sort · what it would change", 9, 5, 3, 2), bl("profiles", "Profiles", 9, 7, 3, 2), bl("folder", "The game folder", 9, 9, 3, 3)],
    },
    PageDef { id: "install", title: "Add mods", blocks: &[bl("add", "Add mods", 0, 0, 3, 3), bl("links", "From links", 0, 3, 3, 3), bl("waiting", "Waiting to go in", 0, 6, 3, 6), bls("installer", "The installer (FOMOD)", 3, 0, 6, 12), bl("before", "Before it goes in", 9, 0, 3, 4), bl("where", "Where it goes", 9, 4, 3, 3), bl("already", "If the mod is already in", 9, 7, 3, 5)] },
    PageDef { id: "find", title: "Find mods", blocks: &[bl("search", "One search", 0, 0, 12, 1), bls("results", "Results", 0, 1, 8, 7), bl("sets", "Whole sets, installed in one go", 0, 8, 8, 4), bl("picked", "The picked mod", 8, 1, 4, 11)] },
    PageDef { id: "updates", title: "Updates and going back", blocks: &[bl("banner", "Nothing updates by itself", 0, 0, 12, 1), bls("updates", "Updates", 0, 1, 4, 11), bl("snapshots", "Snapshots", 4, 1, 4, 11), bl("saves", "Saves that need a mod", 8, 1, 4, 5), bl("how", "How updates work", 8, 6, 4, 6)] },
    PageDef { id: "crashes", title: "Crashes", blocks: &[bl("head", "What crashed", 0, 0, 12, 2), bl("fix", "Fix it in one tap", 0, 2, 3, 4), bl("which", "Which mod probably did it", 0, 6, 3, 6), bls("log", "The crash log", 3, 2, 6, 10), bl("lately", "Crashes lately", 9, 2, 3, 3), bl("send", "Send logs", 9, 5, 3, 3), bl("when", "When a game crashes", 9, 8, 3, 4)] },
    PageDef { id: "describe", title: "Describe it", blocks: &[bls("say", "Say what you want", 0, 0, 4, 12), bl("set", "The proposed set", 4, 0, 4, 12), bl("ally", "On the ROG Ally", 8, 0, 4, 3), bl("looks", "How it looks", 8, 3, 4, 3), bl("go", "Install the set", 8, 6, 4, 6)] },
    PageDef { id: "every", title: "Every game", blocks: &[bl("head", "Every game, one place", 0, 0, 12, 1), bls("cards", "Your games", 0, 1, 12, 6), bl("found", "Found by themselves", 0, 7, 4, 5), bl("devices", "On your other devices", 4, 7, 4, 5), bl("server", "Kept on your home server", 8, 7, 4, 2), bl("accounts", "Accounts", 8, 9, 4, 3)] },
];

use hearth_ui::studio::{tile, Kit};
/// Mod Manager's blocks window: its buttons, its panes, its info.
const KIT: Kit = Kit {
    name: "Mod Manager",
    buttons: &[
        tile("play", "Play modded", G::Play),
        tile("steam", "Add to Steam", G::Link),
        tile("add", "Add mods", G::Plus),
        tile("find", "Find mods", G::Search),
        tile("sort", "Sort", G::List),
        tile("snapshot", "Take a snapshot", G::Clock),
        tile("check", "Check for updates", G::Update),
        tile("send", "Send logs", G::Rocket),
        tile("reload", "Look for games again", G::Grid),
    ],
    panes: &[tile("ladder", "The mods", G::List), tile("health", "Health", G::Pulse), tile("updates", "Updates", G::Update)],
    info: &[tile("games", "Games", G::Grid), tile("mods", "Mods", G::List), tile("on", "Mods on", G::Check), tile("updates", "Updates", G::Update)],
    pane_page: "window",
    bars: &[],
    menus: &[],
};

fn block_action(id: &str) -> Option<Msg> {
    Some(match id {
        "play" => Msg::Play,
        "steam" => Msg::AddSteam,
        "add" => Msg::Go(Page::Downloads),
        "find" => Msg::Go(Page::Find),
        "sort" => Msg::Sort,
        "snapshot" => Msg::TakeSnap,
        "check" => Msg::CheckNow,
        "send" => Msg::SendLogs,
        "reload" => Msg::Reload,
        _ => return None,
    })
}

fn studio_values(a: &App) {
    let (mods, on) = a.game().map(|g| (g.mods, g.on)).unwrap_or_default();
    let ups = a.updates["updates"].as_array().map(|u| u.len()).unwrap_or(0);
    a.describe.values(vec![("games", a.games.len().to_string()), ("mods", mods.to_string()), ("on", on.to_string()), ("updates", ups.to_string())]);
}

fn subscription(a: &App) -> Subscription<Msg> {
    let mut s = vec![Subscription::run(hearth_ui::theme::watch).map(|_| Msg::Theme), Subscription::run(pad::presses).map(Msg::Pad)];
    if a.page == Page::Downloads || a.page == Page::Modlists || a.tv {
        s.push(Subscription::run(sources::ticks).map(|_| Msg::Tick));
    }
    Subscription::batch(s)
}

fn main() -> iced::Result {
    let args: Vec<String> = std::env::args().collect();
    // Everything the old hearth-mods did without a window (the nxm handler, Play modded, the Steam shortcuts' commands,
    // Game Mode's worker): the same code, the same answers.
    if args.iter().any(|x| x == "--check-updates") {
        std::process::exit(more::check_all());
    }
    if args.iter().any(|x| x == "--serve") {
        std::process::exit(hearth_gaming::mods_app::serve_headless());
    }
    if let Some(code) = hearth_gaming::mods_app::cli(&args) {
        std::process::exit(code);
    }
    if app::handle_cli("hearth-mods", "Mod Manager: every game's mods in one place.\n  --game ID        open that game\n  --page NAME      every, find, downloads, updates, crashes, describe, modlists, organizer, accounts, settings\n  --fullscreen     the controller view (Game Mode; --tv too)\n  and every command of Hearth Mods (--nxm, --play-modded, --organizer, --serve, …)") {
        return Ok(());
    }
    // This window is the worker (the queue, the installs, nxm links) unless one is already running (Game Mode's).
    let _ = std::fs::create_dir_all(hearth_gaming::mods_app::data_root().join("mods"));
    let _ = std::fs::write(hearth_gaming::mods_app::pid_file(), std::process::id().to_string());
    hearth_gaming::mods_app::become_worker();
    let w = app::window("hearth-mods", (1600.0, 960.0), (1024.0, 640.0));
    iced::application(App::new, update, view).title(|_: &App| "Mod Manager".to_string()).subscription(subscription).theme(|a: &App| app::iced_theme(&a.p)).window(w).default_font(app::INTER).run()
}
