//! hearth-proton-manager: Proton Manager in Settings › Gaming (docs/mockups/checklists/open-source-02-proton-manager.md;
//! the approved board docs/mockups/open-source-apps/02-proton-manager.png). What runs your Windows games: the Proton
//! and Wine versions this device has (each one's size and how many games use it, Make default, Remove), getting more
//! (Proton Hearth, Proton-GE, Wine-GE, Valve's Proton, Luxtorpeda), each game's own pick, and how Hearth picks.
//! Built on ProtonUp-Qt (GPL-3): the same sources (each project's GitHub releases), checked the same way (their
//! SHA-512), unpacked into the same folders Steam and Lutris read (compatibilitytools.d, Lutris' wine runners); each
//! game's pick is Steam's own (config.vdf, changed only while Steam is closed), Lutris' own (the game's YAML) or the
//! Proton detector's (proton-worked.json, Hearth Games). Nothing is added, removed or switched unless you say so.
//!
//! Made of blocks: the side and the page, and the page's title, Installed, Get more, Each game's pick and How Hearth
//! picks, each moved, sized, hidden and customized in Edit layout; its own buttons, panes and info in its blocks window.

mod steam;

use hearth_oss_kit::{self as k, yaml, Ch, Sz, Tag, E};
use hearth_ui::app::{self, win};
use hearth_ui::run;
use hearth_ui::ui::{self, t, tb, TitleMsgs, G};
use hearth_ui::Pal;
use iced::widget::text::Wrapping;
use iced::widget::{column, container, row, scrollable, Column, Row};
use iced::{Color, Element, Length, Subscription, Task};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

const APP: &str = "proton-manager";

#[derive(Debug, Clone, Copy, PartialEq)]
enum Kind {
    Proton,
    Wine,
}

/// One version on this device.
#[derive(Debug, Clone)]
struct Ver {
    /// the name Steam (and Hearth's detector) knows it by: its folder, or proton_9 for Valve's
    id: String,
    name: String,
    kind: Kind,
    source: String,
    path: PathBuf,
    size: u64,
    /// in a folder of yours (Hearth's runners, Steam's or Lutris' own): Remove takes it out
    yours: bool,
    /// one of Valve's (Steam installs and removes it)
    valve: Option<u32>,
    /// Lutris' name for it (its folder in Lutris' wine runners)
    lutris: Option<String>,
}

/// A game with a pick of its own.
#[derive(Debug, Clone)]
struct Pick {
    key: String,
    name: String,
    from: String,
    runs: String,
    why: String,
}

/// What was found (on its own thread: folder sizes take a moment).
#[derive(Debug, Clone, Default)]
struct Scan {
    vers: Vec<Ver>,
    picks: Vec<Pick>,
    uses: Vec<(String, usize)>,
    total: usize,
    steam_default: String,
    roots: Vec<PathBuf>,
}

/// A family to get versions of (the board's tabs).
struct Family {
    tab: &'static str,
    repo: &'static str,
    kind: Kind,
}
const FAMILIES: [Family; 5] = [
    Family { tab: "Proton Hearth", repo: "Hearth-Organization/proton-hearth", kind: Kind::Proton },
    Family { tab: "Proton-GE", repo: "GloriousEggroll/proton-ge-custom", kind: Kind::Proton },
    Family { tab: "Wine-GE", repo: "GloriousEggroll/wine-ge-custom", kind: Kind::Wine },
    Family { tab: "Valve Proton", repo: "", kind: Kind::Proton },
    Family { tab: "Luxtorpeda", repo: "luxtorpeda-dev/luxtorpeda", kind: Kind::Proton },
];

#[derive(Debug, Clone)]
struct Rel {
    name: String,
    tag: String,
    day: String,
    about: String,
    url: String,
    size: u64,
    sha: String,
    valve: Option<u32>,
}

#[derive(Debug, Clone)]
struct Dl {
    name: String,
    family: usize,
    url: String,
    size: u64,
    sha: String,
    part: PathBuf,
    got: u64,
    paused: bool,
}

#[derive(Debug, Clone, PartialEq)]
enum Pop {
    Default,
    Runs(String),
    Remove(String),
}

#[derive(Debug, Clone)]
enum Msg {
    Describe(hearth_ui::describe::DMsg),
    Theme,
    Scanned(Box<Scan>),
    Who(String, String, String),
    Filter(u8),
    Family(usize),
    Releases(usize, run::Output),
    CheckAll,
    Get(usize, usize),
    Pause,
    Resume,
    Tick,
    Downloaded(String, bool),
    Unpacked(String, Result<String, String>),
    MakeDefault(String),
    Pop(Pop),
    Close,
    Remove(String),
    SetPick(String, Option<String>),
    FindGame(String),
    Toggle(&'static str),
    Nav(&'static str),
    FindSetting(String),
    Drag,
    Max,
    Min,
    WClose,
    Resize(iced::window::Direction),
}

struct App {
    describe: hearth_ui::describe::Describe,
    p: Pal,
    scan: Scan,
    scanning: bool,
    who: (String, String, String),
    filter: u8,
    family: usize,
    rels: Vec<Vec<Rel>>,
    checked: Vec<bool>,
    dl: Option<Dl>,
    child: Arc<Mutex<Option<std::process::Child>>>,
    pop: Option<Pop>,
    find: String,
    find_setting: String,
    set: Value,
    said: String,
}

fn obj(v: Value) -> serde_json::Map<String, Value> {
    v.as_object().cloned().unwrap_or_default()
}

/// Hearth's own folder for the versions it gets (the board's "Where they live").
fn runners_dir() -> PathBuf {
    k::data_dir("runners")
}
fn lutris_wine_dirs() -> Vec<PathBuf> {
    let h = k::home();
    let data = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|| h.join(".local/share"));
    vec![data.join("lutris/runners/wine"), h.join(".var/app/net.lutris.Lutris/data/lutris/runners/wine")]
}
fn builtin_dir() -> PathBuf {
    std::env::var_os("HEARTH_BUILTIN_PROTON_DIR").filter(|v| !v.is_empty()).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/usr/share/steam/compatibilitytools.d"))
}

/// A version's folder name as people say it.
fn pretty(folder: &str, kind_hint: Option<Kind>) -> String {
    let s = folder.trim().trim_end_matches("-x86_64").trim_end_matches("-i686");
    let low = s.to_lowercase();
    let digits = |r: &str| r.trim_start_matches(['-', '_', ' ']).to_string();
    if let Some(r) = low.strip_prefix("proton-hearth").or_else(|| low.strip_prefix("proton hearth")) {
        return format!("Proton Hearth {}", digits(&s[s.len() - r.len()..]));
    }
    if let Some(r) = low.strip_prefix("lutris-ge-proton").or_else(|| low.strip_prefix("wine-ge-")) {
        return format!("Wine-GE {}", digits(&s[s.len() - r.len()..]));
    }
    if let Some(r) = low.strip_prefix("ge-proton") {
        return if kind_hint == Some(Kind::Wine) { format!("Wine-GE {}", digits(&s[s.len() - r.len()..])) } else { format!("Proton-GE {}", digits(&s[s.len() - r.len()..])) };
    }
    if let Some(r) = low.strip_prefix("proton-") {
        return format!("Proton {}", &s[s.len() - r.len()..]);
    }
    if let Some(r) = low.strip_prefix("luxtorpeda") {
        return format!("Luxtorpeda {}", digits(&s[s.len() - r.len()..]).trim_start_matches('v'));
    }
    if let Some(r) = low.strip_prefix("wine-") {
        return format!("Wine {}", &s[s.len() - r.len()..]);
    }
    s.to_string()
}

fn uses_of(sc: &Scan, v: &Ver) -> usize {
    sc.uses.iter().filter(|(n, _)| *n == v.id || *n == v.name || Some(n) == v.lutris.as_ref()).map(|x| x.1).sum()
}

/// Everything found: the versions (with their sizes), each game's pick and how many games use each version.
fn scan(default: String, why: Value, changed: Value) -> Scan {
    let mut vers: Vec<Ver> = Vec::new();
    let add = |v: Ver, vers: &mut Vec<Ver>| {
        let real = std::fs::canonicalize(&v.path).unwrap_or(v.path.clone());
        if !vers.iter().any(|x| std::fs::canonicalize(&x.path).unwrap_or(x.path.clone()) == real) {
            vers.push(v);
        }
    };
    let src_of = |n: &str| if n.contains("Hearth") { "Hearth's own build".to_string() } else if n.contains("GE") || n.starts_with("Luxtorpeda") { "Community".into() } else { "Your own".into() };
    // built into Hearth OS
    if let Ok(rd) = std::fs::read_dir(builtin_dir()) {
        for e in rd.flatten() {
            let f = e.file_name().to_string_lossy().to_string();
            if e.path().join("proton").is_file() {
                let name = pretty(&f, Some(Kind::Proton));
                add(Ver { id: f.clone(), source: if name.contains("Hearth") { "Hearth's own build".into() } else { "Built into Hearth OS".into() }, name, kind: Kind::Proton, path: e.path(), size: 0, yours: false, valve: None, lutris: None }, &mut vers);
            }
        }
    }
    let roots = steam::roots();
    // Hearth's runners, then Steam's own compatibility folders
    let mut dirs: Vec<PathBuf> = vec![runners_dir()];
    dirs.extend(roots.iter().map(|r| r.join("compatibilitytools.d")));
    for d in dirs {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let f = e.file_name().to_string_lossy().to_string();
            let p = e.path();
            let proton = p.join("proton").is_file() || p.join("compatibilitytool.vdf").is_file();
            if !proton {
                continue;
            }
            let name = pretty(&f, Some(Kind::Proton));
            add(Ver { id: f.clone(), source: src_of(&name), name, kind: Kind::Proton, path: p, size: 0, yours: true, valve: None, lutris: None }, &mut vers);
        }
    }
    // Valve's, in Steam's libraries
    let libs = steam::libraries(&roots);
    for l in &libs {
        let Ok(rd) = std::fs::read_dir(l.join("steamapps/common")) else { continue };
        for e in rd.flatten() {
            let f = e.file_name().to_string_lossy().to_string();
            if !f.starts_with("Proton") || !e.path().join("proton").is_file() {
                continue;
            }
            let ver = std::fs::read_to_string(e.path().join("version")).ok().and_then(|s| s.split_whitespace().last().map(str::to_string)).unwrap_or_default();
            let name = if ver.is_empty() { f.clone() } else { pretty(&ver, Some(Kind::Proton)) };
            let (id, app) = steam::valve_tool(&f);
            add(Ver { id, name, kind: Kind::Proton, source: "Valve".into(), path: e.path(), size: 0, yours: false, valve: app, lutris: None }, &mut vers);
        }
    }
    // Lutris' wine runners
    for d in lutris_wine_dirs() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let f = e.file_name().to_string_lossy().to_string();
            if !e.path().is_dir() {
                continue;
            }
            let name = pretty(&f, Some(Kind::Wine));
            let source = if name.starts_with("Wine-GE") { "Community".to_string() } else { "Lutris".to_string() };
            add(Ver { id: f.clone(), name, kind: Kind::Wine, source, path: e.path(), size: 0, yours: true, valve: None, lutris: Some(f) }, &mut vers);
        }
    }
    // the system's own Wine
    if let Some(w) = which("wine") {
        let v = std::process::Command::new(&w).arg("--version").output().ok().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default();
        if !v.is_empty() {
            let lib = w.parent().and_then(Path::parent).map(|p| p.join("lib64/wine")).unwrap_or_default();
            vers.push(Ver { id: "system".into(), name: pretty(&v, Some(Kind::Wine)), kind: Kind::Wine, source: "Fedora's Wine".into(), path: lib, size: 0, yours: false, valve: None, lutris: Some("system".into()) });
        }
    }
    for v in vers.iter_mut() {
        v.size = k::folder_size(&v.path);
    }

    // each game, what it runs with, and the ones with a pick of their own
    let mapping = steam::mapping(&roots);
    let steam_default = mapping.iter().find(|m| m.0 == "0").map(|m| m.1.clone()).unwrap_or_default();
    let def = if default.is_empty() { steam_default.clone() } else { default.clone() };
    let mut uses: Vec<(String, usize)> = Vec::new();
    let bump = |n: &str, uses: &mut Vec<(String, usize)>| match uses.iter_mut().find(|u| u.0 == n) {
        Some(u) => u.1 += 1,
        None => uses.push((n.to_string(), 1)),
    };
    let mut picks: Vec<Pick> = Vec::new();
    let why_of = |key: &str| why.get(key).and_then(Value::as_str).unwrap_or("").to_string();
    let games = steam::games(&libs);
    let names_of = |id: &str| vers.iter().find(|v| v.id == id).map(|v| v.name.clone()).unwrap_or_else(|| pretty(id, None));
    for g in &games {
        match mapping.iter().find(|m| m.0 == g.app) {
            Some((_, tool)) => {
                bump(tool, &mut uses);
                let w = why_of(&format!("steam:{}", g.app));
                picks.push(Pick { key: format!("steam:{}", g.app), name: g.name.clone(), from: "Steam".into(), runs: tool.clone(), why: if w.is_empty() { "You picked it".into() } else { w } });
            }
            None => {
                // what its prefix last ran with, when that's here; else the default
                let ran = pretty(&g.ran_with, Some(Kind::Proton));
                if !g.ran_with.is_empty() && vers.iter().any(|v| v.name == ran || v.id == g.ran_with) {
                    let id = vers.iter().find(|v| v.name == ran || v.id == g.ran_with).map(|v| v.id.clone()).unwrap_or_default();
                    bump(&id, &mut uses);
                } else if !def.is_empty() {
                    bump(&def, &mut uses);
                }
            }
        }
    }
    let mut total = games.len();
    // Lutris' games (Hearth Game Fixer's): its YAML's wine version
    let lut_games = lutris_games();
    for (slug, name, version) in &lut_games {
        total += 1;
        let v = if version.is_empty() { "system".to_string() } else { version.clone() };
        bump(&v, &mut uses);
        let fixer = k::data_dir("game-fixer").join("games").join(format!("{slug}.json"));
        let note = k::settings_file::read_json(&fixer);
        let script_asks = note.pointer("/script/asks").and_then(Value::as_str).map(|s| !s.is_empty()).unwrap_or(false);
        let w = why_of(&format!("lutris:{slug}"));
        if !w.is_empty() || script_asks {
            let from = if fixer.is_file() { "Game Fixer" } else { "Added by hand" };
            picks.push(Pick { key: format!("lutris:{slug}"), name: name.clone(), from: from.into(), runs: v, why: if w.is_empty() { "Script asks for it".into() } else { w } });
        }
    }
    // Hearth Games: the Proton detector's picks (proton-worked.json)
    let worked = k::settings_file::read_json(&k::home().join(".local/share/nimbus/proton-worked.json"));
    if let Some(m) = worked.as_object() {
        for (key, e) in m {
            if e.get("picked").and_then(Value::as_bool) != Some(true) {
                continue;
            }
            let Some(w) = e.get("worked").and_then(Value::as_str) else { continue };
            total += 1;
            bump(w, &mut uses);
            let store = key.split(':').next().unwrap_or("");
            let store = match store {
                "epic" => "Epic",
                "gog" => "GOG",
                "amazon" => "Amazon",
                o => o,
            };
            picks.push(Pick {
                key: format!("hearth:{key}"),
                name: e.get("title").and_then(Value::as_str).unwrap_or(key).to_string(),
                from: format!("Hearth Games · {store}"),
                runs: w.to_string(),
                why: e.get("why").and_then(Value::as_str).unwrap_or("You picked it").to_string(),
            });
        }
    }
    let _ = names_of;
    // the newest change first (when each was picked, kept by Proton Manager)
    picks.sort_by_key(|pk| std::cmp::Reverse(changed.get(&pk.key).and_then(Value::as_i64).unwrap_or(0)));
    // the most used first
    let mut sc = Scan { vers: Vec::new(), picks, uses, total, steam_default, roots };
    vers.sort_by_key(|v| std::cmp::Reverse(uses_of(&sc, v)));
    sc.vers = vers;
    sc
}

fn which(cmd: &str) -> Option<PathBuf> {
    std::env::var_os("PATH").and_then(|p| std::env::split_paths(&p).map(|d| d.join(cmd)).find(|f| f.is_file()))
}

/// Lutris' games from their YAML files: (slug, name, wine version). The name from Lutris' list when it has one.
fn lutris_games() -> Vec<(String, String, String)> {
    let h = k::home();
    let cfg = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| h.join(".config"));
    let data = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|| h.join(".local/share"));
    let names: Value = std::fs::read_to_string(k::data_dir("game-fixer").join("names.json")).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(Value::Null);
    let mut v: Vec<(String, String, String)> = Vec::new();
    for d in [data.join("lutris/games"), cfg.join("lutris/games"), h.join(".var/app/net.lutris.Lutris/config/lutris/games")] {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let f = e.file_name().to_string_lossy().to_string();
            let Some(stem) = f.strip_suffix(".yml") else { continue };
            let slug = match stem.rsplit_once('-') {
                Some((s, n)) if n.chars().all(|c| c.is_ascii_digit()) => s.to_string(),
                _ => stem.to_string(),
            };
            if v.iter().any(|x| x.0 == slug) {
                continue;
            }
            let text = std::fs::read_to_string(e.path()).unwrap_or_default();
            if yaml::yget(&text, &["game", "exe"]).is_none() && yaml::yget(&text, &["wine", "version"]).is_none() {
                continue;
            }
            let ver = yaml::yget(&text, &["wine", "version"]).unwrap_or_default();
            let name = names.get(&slug).and_then(Value::as_str).map(str::to_string).unwrap_or_else(|| slug.split('-').map(|w| { let mut c = w.chars(); c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default() }).collect::<Vec<_>>().join(" "));
            v.push((slug, name, ver));
        }
    }
    v
}

fn now_secs() -> i64 {
    hearth_ui::time::now()
}

/// GitHub's releases answer → the versions on offer.
fn releases_of(fam: usize, j: &Value) -> Vec<Rel> {
    let kind = FAMILIES[fam].kind;
    j.as_array()
        .map(|a| {
            a.iter()
                .filter(|r| !r.get("draft").and_then(Value::as_bool).unwrap_or(false) && !r.get("prerelease").and_then(Value::as_bool).unwrap_or(false))
                .filter_map(|r| {
                    let tag = r.get("tag_name").and_then(Value::as_str)?.to_string();
                    let assets = r.get("assets").and_then(Value::as_array).cloned().unwrap_or_default();
                    let tar = assets.iter().find(|x| x.get("name").and_then(Value::as_str).map(|n| (n.ends_with(".tar.gz") || n.ends_with(".tar.xz") || n.ends_with(".tar.zst")) && !n.contains("i686")).unwrap_or(false))?;
                    let sha = assets.iter().find(|x| x.get("name").and_then(Value::as_str).map(|n| n.ends_with(".sha512sum")).unwrap_or(false)).and_then(|x| x.get("browser_download_url").and_then(Value::as_str)).unwrap_or("").to_string();
                    let body = r.get("body").and_then(Value::as_str).unwrap_or("");
                    let about = body.lines().map(|l| l.trim().trim_start_matches(['#', '*', '-', ' '])).find(|l| !l.is_empty()).unwrap_or("").chars().take(70).collect::<String>();
                    Some(Rel {
                        name: pretty(if FAMILIES[fam].tab == "Proton Hearth" && !tag.to_lowercase().contains("hearth") { "" } else { tag.as_str() }, Some(kind)).replace("Proton-GE", if kind == Kind::Wine { "Wine-GE" } else { "Proton-GE" }),
                        tag,
                        day: k::day_words(r.get("published_at").and_then(Value::as_str).unwrap_or("")),
                        about,
                        url: tar.get("browser_download_url").and_then(Value::as_str).unwrap_or("").to_string(),
                        size: tar.get("size").and_then(Value::as_u64).unwrap_or(0),
                        sha,
                        valve: None,
                    })
                })
                .filter(|r| !r.name.is_empty())
                .take(8)
                .collect()
        })
        .unwrap_or_default()
}

/// Valve's Protons, which Steam itself installs.
fn valve_rels() -> Vec<Rel> {
    [("Proton 10.0", 3658110u32), ("Proton 9.0", 2805730), ("Proton 8.0", 2348590), ("Proton 7.0", 1887720), ("Proton - Experimental", 1493710)]
        .iter()
        .map(|(n, a)| Rel { name: n.to_string(), tag: n.to_string(), day: String::new(), about: "installed by Steam".into(), url: String::new(), size: 0, sha: String::new(), valve: Some(*a) })
        .collect()
}

impl App {
    fn new() -> (App, Task<Msg>) {
        let set = k::read_settings(APP);
        let mut a = App {
            describe: hearth_ui::describe::Describe::new(APP, "Proton Manager").with_pages(BLOCKS).with_kit(&KIT),
            p: hearth_ui::theme::load(),
            scan: Scan::default(),
            scanning: true,
            who: (String::new(), String::new(), String::new()),
            filter: 0,
            family: 0,
            rels: (0..FAMILIES.len()).map(|_| Vec::new()).collect(),
            checked: vec![false; FAMILIES.len()],
            dl: None,
            child: Arc::new(Mutex::new(None)),
            pop: None,
            find: String::new(),
            find_setting: String::new(),
            set,
            said: String::new(),
        };
        a.rels[3] = valve_rels();
        a.checked[3] = true;
        // a download that was going when the app closed goes on where it stopped
        if let Some(d) = a.set.get("download").filter(|d| d.is_object()) {
            let part = PathBuf::from(d.get("part").and_then(Value::as_str).unwrap_or(""));
            if part.starts_with(runners_dir()) {
                a.dl = Some(Dl {
                    name: d.get("name").and_then(Value::as_str).unwrap_or("").into(),
                    family: d.get("family").and_then(Value::as_u64).unwrap_or(1) as usize,
                    url: d.get("url").and_then(Value::as_str).unwrap_or("").into(),
                    size: d.get("size").and_then(Value::as_u64).unwrap_or(0),
                    sha: d.get("sha").and_then(Value::as_str).unwrap_or("").into(),
                    got: std::fs::metadata(&part).map(|m| m.len()).unwrap_or(0),
                    part,
                    paused: true,
                });
            }
        }
        let mut t = vec![a.rescan(), who_task(), a.fetch(0)];
        if a.dl.is_some() {
            t.push(Task::done(Msg::Resume));
        }
        (a, Task::batch(t))
    }
    fn on(&self, key: &str, dflt: bool) -> bool {
        self.set.get(key).and_then(Value::as_bool).unwrap_or(dflt)
    }
    fn default(&self) -> String {
        self.set.get("default").and_then(Value::as_str).map(str::to_string).filter(|s| !s.is_empty()).unwrap_or_else(|| self.scan.steam_default.clone())
    }
    fn rescan(&mut self) -> Task<Msg> {
        self.scanning = true;
        let d = self.set.get("default").and_then(Value::as_str).unwrap_or("").to_string();
        let why = self.set.get("why").cloned().unwrap_or(Value::Null);
        let changed = self.set.get("changed").cloned().unwrap_or(Value::Null);
        let (tx, rx) = futures::channel::oneshot::channel();
        let _ = std::thread::Builder::new().name("proton-scan".into()).spawn(move || {
            let _ = tx.send(scan(d, why, changed));
        });
        Task::perform(async move { rx.await.unwrap_or_default() }, |s| Msg::Scanned(Box::new(s)))
    }
    fn fetch(&mut self, fam: usize) -> Task<Msg> {
        let repo = FAMILIES[fam].repo;
        if repo.is_empty() || self.checked[fam] {
            return Task::none();
        }
        self.checked[fam] = true;
        let url = format!("https://api.github.com/repos/{repo}/releases?per_page=10");
        Task::perform(run::spawn(run::args(&["curl", "-fsSL", "-m", "25", "-H", "Accept: application/vnd.github+json", &url])), move |o| Msg::Releases(fam, o))
    }
    fn save(&mut self, keys: Value) {
        if let Err(e) = k::save_settings(APP, obj(keys)) {
            self.said = format!("Couldn't save that: {e}");
        }
        self.set = k::read_settings(APP);
    }
    fn ver(&self, id: &str) -> Option<&Ver> {
        self.scan.vers.iter().find(|v| v.id == id || v.name == id || v.lutris.as_deref() == Some(id))
    }
    fn installed(&self, name: &str) -> bool {
        self.scan.vers.iter().any(|v| v.name == name || v.id == name)
    }
    /// The newest version on offer in the default's family that isn't here yet.
    fn newer(&self) -> Option<(usize, usize)> {
        let d = self.default();
        let dn = self.ver(&d).map(|v| v.name.clone()).unwrap_or(d);
        let fam = FAMILIES.iter().position(|f| dn.starts_with(f.tab) || (f.tab == "Proton-GE" && dn.starts_with("Proton-GE")))?;
        let r = self.rels[fam].first()?;
        (!self.installed(&r.name) && r.name != dn).then_some((fam, 0))
    }
}

fn who_task() -> Task<Msg> {
    let cmd = vec![
        "sh".to_string(),
        "-c".into(),
        "u=$(id -un); n=$(getent passwd \"$u\" | cut -d: -f5 | cut -d, -f1); echo \"${HEARTH_PM_WHO:-${n:-$u}}\"; pc=$(sed -n 's/^PRETTY_HOSTNAME=//p' /etc/machine-info 2>/dev/null | tr -d '\"'); echo \"${HEARTH_PM_PC:-${pc:-$(cat /etc/hostname 2>/dev/null)}}\"; bootc status --format json 2>/dev/null | tr -d '\\n ' | grep -o '\"booted\":{\"image\":{\"image\":{[^}]*},\"version\":\"[^\"]*\"' | sed 's/.*\"version\":\"//; s/\"$//'".into(),
    ];
    Task::perform(run::spawn(cmd), |o| {
        let mut l = o.out.lines();
        Msg::Who(l.next().unwrap_or("").trim().into(), l.next().unwrap_or("").trim().into(), l.next().unwrap_or("").trim().into())
    })
}

/// A download, with what it fetched kept in `part` so a pause goes on where it stopped (curl -C -).
fn start(dl: &Dl, child: Arc<Mutex<Option<std::process::Child>>>) -> Task<Msg> {
    let url = dl.url.clone();
    let part = dl.part.clone();
    let name = dl.name.clone();
    let (tx, rx) = futures::channel::oneshot::channel();
    let _ = std::thread::Builder::new().name("proton-get".into()).spawn(move || {
        if let Some(d) = part.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        let c = std::process::Command::new("curl").args(["-fsSL", "-C", "-", "-o"]).arg(&part).arg(&url).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn();
        let ok = match c {
            Ok(c) => {
                *child.lock().unwrap() = Some(c);
                loop {
                    std::thread::sleep(std::time::Duration::from_millis(300));
                    let mut g = child.lock().unwrap();
                    match g.as_mut().map(|c| c.try_wait()) {
                        Some(Ok(Some(s))) => {
                            *g = None;
                            break s.success();
                        }
                        Some(Ok(None)) => continue,
                        _ => break false,
                    }
                }
            }
            Err(_) => false,
        };
        let _ = tx.send(ok);
    });
    Task::perform(async move { rx.await.unwrap_or(false) }, move |ok| Msg::Downloaded(name.clone(), ok))
}

/// Checked (its SHA-512, when the project gives one) and unpacked into Hearth's runners, then shown to Steam (its
/// compatibility folder) or Lutris (its wine runners) by a link.
fn unpack(dl: Dl, kind: Kind, steam_root: Option<PathBuf>, show_in_steam: bool) -> Task<Msg> {
    let (tx, rx) = futures::channel::oneshot::channel();
    let name = dl.name.clone();
    let _ = std::thread::Builder::new().name("proton-unpack".into()).spawn(move || {
        let r = (|| -> Result<String, String> {
            if !dl.sha.is_empty() {
                let want = run::blocking(&run::args(&["curl", "-fsSL", "-m", "30", &dl.sha]));
                let got = run::blocking(&vec!["sha512sum".to_string(), dl.part.to_string_lossy().to_string()]);
                let w = want.out.split_whitespace().next().unwrap_or("").to_lowercase();
                let g = got.out.split_whitespace().next().unwrap_or("").to_lowercase();
                if w.is_empty() || w != g {
                    let _ = std::fs::remove_file(&dl.part);
                    return Err("its checksum didn't match, so it was thrown away".into());
                }
            }
            let dir = runners_dir();
            let before: Vec<String> = std::fs::read_dir(&dir).map(|r| r.flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect()).unwrap_or_default();
            let o = run::blocking(&vec!["tar".to_string(), "-xf".into(), dl.part.to_string_lossy().to_string(), "-C".into(), dir.to_string_lossy().to_string()]);
            if !o.ok {
                return Err(format!("it couldn't be unpacked: {}", o.why()));
            }
            let _ = std::fs::remove_file(&dl.part);
            let new: Vec<String> = std::fs::read_dir(&dir).map(|r| r.flatten().map(|e| e.file_name().to_string_lossy().to_string()).filter(|n| !before.contains(n) && !n.ends_with(".part")).collect()).unwrap_or_default();
            for n in &new {
                let target = dir.join(n);
                let link = match kind {
                    Kind::Proton if show_in_steam => steam_root.as_ref().map(|r| r.join("compatibilitytools.d").join(n)),
                    Kind::Wine => lutris_wine_dirs().into_iter().next().map(|d| d.join(n)),
                    _ => None,
                };
                if let Some(l) = link {
                    if let Some(p) = l.parent() {
                        let _ = std::fs::create_dir_all(p);
                    }
                    if !l.exists() {
                        #[cfg(unix)]
                        let _ = std::os::unix::fs::symlink(&target, &l);
                    }
                }
            }
            Ok(new.join(", "))
        })();
        let _ = tx.send(r);
    });
    Task::perform(async move { rx.await.unwrap_or(Err("it stopped".into())) }, move |r| Msg::Unpacked(name.clone(), r))
}

/// Takes a version out: its folder (only in your own folders) and the links Hearth made to it.
fn remove_ver(v: &Ver) -> Result<(), String> {
    let home = k::home();
    if !v.yours || !v.path.starts_with(&home) {
        return Err("That one isn't in a folder of yours.".into());
    }
    let real = std::fs::canonicalize(&v.path).unwrap_or(v.path.clone());
    for d in steam::roots().into_iter().map(|r| r.join("compatibilitytools.d")).chain(lutris_wine_dirs()).chain([runners_dir()]) {
        if let Ok(rd) = std::fs::read_dir(&d) {
            for e in rd.flatten() {
                let p = e.path();
                if std::fs::symlink_metadata(&p).map(|m| m.file_type().is_symlink()).unwrap_or(false) && std::fs::canonicalize(&p).map(|x| x == real).unwrap_or(false) {
                    let _ = std::fs::remove_file(&p);
                }
            }
        }
    }
    if real.starts_with(&home) {
        std::fs::remove_dir_all(&real).map_err(|e| e.to_string())?;
    }
    if v.path != real && std::fs::symlink_metadata(&v.path).is_ok() {
        let _ = std::fs::remove_file(&v.path);
    }
    Ok(())
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
        Msg::Scanned(s) => {
            a.scan = *s;
            a.scanning = false;
            // the default's family, for "a new version is out"
            let d = a.default();
            let dn = a.ver(&d).map(|v| v.name.clone()).unwrap_or(d);
            if let Some(f) = FAMILIES.iter().position(|f| dn.starts_with(f.tab)) {
                return a.fetch(f);
            }
        }
        Msg::Who(n, pc, v) => a.who = (n, pc, v),
        Msg::Filter(i) => a.filter = i,
        Msg::Family(i) => {
            a.family = i;
            return a.fetch(i);
        }
        Msg::Releases(f, o) => {
            if o.ok {
                a.rels[f] = releases_of(f, &o.json().unwrap_or(Value::Null));
            } else if FAMILIES[f].tab != "Proton Hearth" {
                a.said = format!("Couldn't reach GitHub for {}: {}", FAMILIES[f].tab, o.why());
            }
            // Get new versions by themselves (only while this is open, and only the default's family)
            if a.on("auto", false) && a.dl.is_none() {
                if let Some((fam, i)) = a.newer().filter(|x| x.0 == f) {
                    return update(a, Msg::Get(fam, i));
                }
            }
        }
        Msg::CheckAll => {
            for c in a.checked.iter_mut() {
                *c = false;
            }
            a.checked[3] = true;
            a.said = "Looking for new versions…".into();
            let t: Vec<Task<Msg>> = (0..FAMILIES.len()).map(|f| a.fetch(f)).collect();
            return Task::batch(t);
        }
        Msg::Get(f, i) => {
            let Some(r) = a.rels.get(f).and_then(|v| v.get(i)).cloned() else { return Task::none() };
            if let Some(app) = r.valve {
                run::detached(&["xdg-open", &format!("steam://install/{app}")]);
                a.said = format!("Steam installs {} (it's Steam's own).", r.name);
                return Task::none();
            }
            if a.dl.is_some() || r.url.is_empty() {
                return Task::none();
            }
            let file = r.url.rsplit('/').next().unwrap_or("download.tar").to_string();
            let dl = Dl { name: r.name.clone(), family: f, url: r.url.clone(), size: r.size, sha: r.sha.clone(), part: runners_dir().join(format!("{file}.part")), got: 0, paused: false };
            a.save(json!({"download": {"name": dl.name, "family": f, "url": dl.url, "size": dl.size, "sha": dl.sha, "part": dl.part.to_string_lossy()}}));
            let t = start(&dl, a.child.clone());
            a.dl = Some(dl);
            return t;
        }
        Msg::Pause => {
            if let Some(d) = a.dl.as_mut() {
                d.paused = true;
                if let Some(c) = a.child.lock().unwrap().as_mut() {
                    let _ = c.kill();
                }
            }
        }
        Msg::Resume => {
            if let Some(d) = a.dl.as_mut() {
                d.paused = false;
                let d = d.clone();
                return start(&d, a.child.clone());
            }
        }
        Msg::Tick => {
            if let Some(d) = a.dl.as_mut() {
                d.got = std::fs::metadata(&d.part).map(|m| m.len()).unwrap_or(d.got);
            }
        }
        Msg::Downloaded(name, ok) => {
            let Some(d) = a.dl.clone().filter(|d| d.name == name) else { return Task::none() };
            if d.paused {
                return Task::none();
            }
            if !ok {
                a.dl.as_mut().map(|d| d.paused = true);
                a.said = format!("{name} stopped downloading (no internet?). Resume goes on where it stopped.");
                return Task::none();
            }
            a.said = format!("Checking and unpacking {name}…");
            let kind = FAMILIES[d.family].kind;
            let show = a.on("gameMenu", true);
            return unpack(d, kind, a.scan.roots.first().cloned(), show);
        }
        Msg::Unpacked(name, r) => {
            a.dl = None;
            a.save(json!({"download": Value::Null}));
            match r {
                Ok(_) => a.said = format!("{name} is here. Your games stay on what they use until you move them."),
                Err(e) => a.said = format!("Couldn't add {name}: {e}"),
            }
            let t = a.rescan();
            // Remove versions no game uses: only ones Hearth got, and never the default
            if a.on("tidy", false) {
                let d = a.default();
                for v in a.scan.vers.clone() {
                    if v.yours && v.path.starts_with(runners_dir()) && uses_of(&a.scan, &v) == 0 && v.id != d && v.name != name {
                        let _ = remove_ver(&v);
                    }
                }
            }
            return t;
        }
        Msg::MakeDefault(id) => {
            a.pop = None;
            a.save(json!({"default": id}));
            // Steam's own default too (its "0"), when it's closed; Lutris' default wine for a wine version
            let v = a.ver(&id).cloned();
            let mut notes = Vec::new();
            if let (Some(r), Some(v)) = (a.scan.roots.first(), v.as_ref()) {
                if v.kind == Kind::Proton {
                    if let Err(e) = steam::set_mapping(r, "0", Some(v.id.as_str())) {
                        notes.push(format!("Steam's own default stays as it is: {e}"));
                    }
                }
            }
            if let Some(l) = v.as_ref().and_then(|v| v.lutris.clone()) {
                let h = k::home();
                let cfg = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| h.join(".config"));
                let f = cfg.join("lutris/runners/wine.yml");
                if f.parent().map(Path::is_dir).unwrap_or(false) {
                    let q = yaml::quote(&l);
                    let _ = k::settings_file::update_text(&f, |t| yaml::yset(t, &["wine", "version"], Some(q.as_str())));
                }
            }
            a.said = if notes.is_empty() { format!("New games start with {}.", v.map(|v| v.name).unwrap_or(id)) } else { notes.join(" ") };
            return a.rescan();
        }
        Msg::Pop(p) => a.pop = Some(p),
        Msg::Close => a.pop = None,
        Msg::Remove(id) => {
            a.pop = None;
            let Some(v) = a.ver(&id).cloned() else { return Task::none() };
            if let Some(app) = v.valve {
                run::detached(&["xdg-open", &format!("steam://uninstall/{app}")]);
                a.said = format!("Steam removes {} (it's Steam's own).", v.name);
                return Task::none();
            }
            match remove_ver(&v) {
                Ok(()) => a.said = format!("{} was removed ({} back).", v.name, k::size_words(v.size)),
                Err(e) => a.said = format!("Couldn't remove {}: {e}", v.name),
            }
            return a.rescan();
        }
        Msg::SetPick(key, to) => {
            a.pop = None;
            let r = if let Some(app) = key.strip_prefix("steam:") {
                match a.scan.roots.first() {
                    Some(root) => steam::set_mapping(root, app, to.as_deref().and_then(|t| a.ver(t)).map(|v| v.id.as_str())),
                    None => Err("Steam isn't on this device.".into()),
                }
            } else if let Some(slug) = key.strip_prefix("lutris:") {
                let f = lutris_config(slug);
                match f {
                    Some(f) => {
                        let val = to.as_deref().and_then(|t| a.ver(t)).map(|v| v.lutris.clone().unwrap_or(v.name.clone()));
                        let q = val.as_deref().map(yaml::quote);
                        k::settings_file::update_text(&f, |t| yaml::yset(t, &["wine", "version"], q.as_deref())).map(|_| ()).map_err(|e| e.to_string())
                    }
                    None => Err("Lutris has no settings file for it.".into()),
                }
            } else if let Some(hk) = key.strip_prefix("hearth:") {
                let p = k::home().join(".local/share/nimbus/proton-worked.json");
                let name = to.as_deref().and_then(|t| a.ver(t)).map(|v| v.id.clone());
                let hk = hk.to_string();
                k::settings_file::update_json(&p, |m| {
                    let e = m.entry(hk).or_insert(json!({}));
                    if !e.is_object() {
                        *e = json!({});
                    }
                    match name {
                        Some(n) => {
                            e["worked"] = json!(n);
                            e["picked"] = json!(true);
                        }
                        None => {
                            if let Some(o) = e.as_object_mut() {
                                o.remove("worked");
                                o.remove("picked");
                            }
                        }
                    }
                    e["at"] = json!(now_secs());
                })
                .map(|_| ())
                .map_err(|e| e.to_string())
            } else {
                Ok(())
            };
            match r {
                Ok(()) => {
                    let mut why = a.set.get("why").cloned().filter(Value::is_object).unwrap_or(json!({}));
                    let mut changed = a.set.get("changed").cloned().filter(Value::is_object).unwrap_or(json!({}));
                    if to.is_some() {
                        why[&key] = json!("You picked it");
                        changed[&key] = json!(now_secs());
                    } else {
                        why.as_object_mut().map(|o| o.remove(&key));
                        changed.as_object_mut().map(|o| o.remove(&key));
                    }
                    a.save(json!({"why": why, "changed": changed}));
                    a.said.clear();
                }
                Err(e) => a.said = e,
            }
            return a.rescan();
        }
        Msg::FindGame(s) => a.find = s,
        Msg::Toggle(key) => {
            let d = matches!(key, "tell" | "gameMenu");
            let v = !a.on(key, d);
            let mut m = serde_json::Map::new();
            m.insert(key.to_string(), json!(v));
            a.save(Value::Object(m));
        }
        Msg::Nav(page) => run::detached(&["hearth-settings-app", "--page", page]),
        Msg::FindSetting(s) => a.find_setting = s,
        Msg::Drag => return win::drag(),
        Msg::Max => return win::toggle_maximize(),
        Msg::Min => return win::minimize(),
        Msg::WClose => return win::close(),
        Msg::Resize(d) => return win::resize(d),
    }
    Task::none()
}

fn lutris_config(slug: &str) -> Option<PathBuf> {
    let h = k::home();
    let cfg = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| h.join(".config"));
    let data = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|| h.join(".local/share"));
    for d in [data.join("lutris/games"), cfg.join("lutris/games"), h.join(".var/app/net.lutris.Lutris/config/lutris/games")] {
        if let Ok(rd) = std::fs::read_dir(&d) {
            for e in rd.flatten() {
                let n = e.file_name().to_string_lossy().to_string();
                if n == format!("{slug}.yml") || n.strip_prefix(slug).map(|r| r.starts_with('-') && r.ends_with(".yml")).unwrap_or(false) {
                    return Some(e.path());
                }
            }
        }
    }
    None
}

// ------------------------------------------------------------------ the window

const NAV: [(&str, &str); 14] = [
    ("Home", "home"),
    ("System", "system"),
    ("Network", "wifi"),
    ("Bluetooth & devices", "bt"),
    ("Hearth Link", "link"),
    ("Personalization", "appearance"),
    ("Apps", "apps-home"),
    ("Gaming", "gaming"),
    ("AI", "ai"),
    ("Accounts", "accounts"),
    ("Home server", "hs-overview"),
    ("Privacy", "privacy"),
    ("Accessibility", "accessibility"),
    ("Updates & recovery", "updates"),
];

fn side(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let (n, pc, v) = &a.who;
    let initial = n.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_else(|| "·".into());
    let line = match (pc.is_empty(), v.is_empty()) {
        (false, false) => format!("{pc} · Hearth OS {}", v.strip_suffix(".0").unwrap_or(v)),
        (false, true) => pc.clone(),
        (true, false) => format!("Hearth OS {v}"),
        _ => String::new(),
    };
    let me = row![k::mark::<Msg>(&initial, k::rgb("#b9a2ff"), k::rgb("#6a4fe0"), 30.0), column![tb(n.clone(), 13.0, p.text).wrapping(Wrapping::None), k::mu1::<Msg>(&p, line, 10.0)].spacing(1.0)].spacing(9.0).align_y(iced::Center);
    let mut nav = Column::new().spacing(1.0);
    let q = a.find_setting.to_lowercase();
    for (label, page) in NAV {
        if !q.is_empty() && !label.to_lowercase().contains(&q) {
            continue;
        }
        let on = page == "gaming";
        let sq = container(k::gap::<Msg>(1.0, 1.0)).width(Length::Fixed(11.0)).height(Length::Fixed(11.0)).style(ui::boxed(if on { p.text } else { Pal::mix(p.muted, p.bg, 0.45) }, 3.0, None));
        nav = nav.push(k::side_row(&p, row![sq, t(label, 12.0, if on { p.text } else { p.muted }).wrapping(Wrapping::None)].spacing(9.0).align_y(iced::Center), on, 16.0, Msg::Nav(page)));
    }
    let body = hearth_ui::arrange::parts(
        &a.describe,
        &p,
        "side",
        vec![
            ("me", container(me).padding(k::pad(2.0, 4.0, 8.0, 4.0)).into()),
            ("find", k::search(&p, "Find a setting", &a.find_setting, Msg::FindSetting)),
            ("nav", scrollable(nav).height(Length::Fill).into()),
            ("edit", k::edit_layout(&p, Msg::Describe(hearth_ui::arrange::edit_layout()))),
        ],
        |ps| column![ps.take("me"), ps.take("find"), k::gap(1.0, 8.0), ps.take("nav"), ps.take("edit")].spacing(2.0).into(),
        Msg::Describe,
    );
    k::side(&p, body, 186.0)
}

fn pmark<'a>(kind: Kind) -> E<'a, Msg> {
    match kind {
        Kind::Proton => k::mark("P", k::rgb("#ffb36b"), k::rgb("#ff6a1f"), 21.0),
        Kind::Wine => k::mark("W", k::rgb("#ff9be8"), k::rgb("#b04fc8"), 21.0),
    }
}

fn installed_card(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let total: u64 = a.scan.vers.iter().map(|v| v.size).sum();
    let head = row![
        k::h2::<Msg>(&p, "Installed"),
        k::mu1(&p, format!("{} · {}", a.scan.vers.len(), k::size_words(total)), 11.0),
        k::fill(),
        k::tabs(&p, vec![("All".into(), a.filter == 0, Msg::Filter(0)), ("Proton".into(), a.filter == 1, Msg::Filter(1)), ("Wine".into(), a.filter == 2, Msg::Filter(2))]),
    ]
    .spacing(8.0)
    .align_y(iced::Center);
    let def = a.default();
    let mut c = Column::new();
    let shown: Vec<&Ver> = a.scan.vers.iter().filter(|v| a.filter == 0 || (a.filter == 1) == (v.kind == Kind::Proton)).collect();
    let last = shown.len().saturating_sub(1);
    for (i, v) in shown.into_iter().enumerate() {
        let is_def = v.id == def || v.name == def;
        let n = uses_of(&a.scan, v);
        let mut name = Row::new().spacing(4.0).push(t(v.name.clone(), 12.0, p.text).wrapping(Wrapping::None));
        if is_def {
            name = name.push(tb("· DEFAULT", 10.0, p.accent).wrapping(Wrapping::None));
        }
        let line = format!("{} · {}", v.source, if v.size > 0 { k::size_words(v.size) } else { "size unknown".into() });
        let uses = if n == 0 { "no game uses it".to_string() } else if n == 1 { "1 game uses it".into() } else { format!("{n} games use it") };
        let can_remove = n == 0 && !is_def && (v.yours || v.valve.is_some());
        let r = row![
            pmark(v.kind),
            column![name, k::mu1::<Msg>(&p, line, 10.0)].spacing(1.0).width(Length::Fill),
            container(k::mu1::<Msg>(&p, uses, 11.0)).width(Length::Fixed(110.0)),
            k::chip(&p, if is_def { "Default" } else { "Make default" }, Ch::Plain, Sz::Sm, (!is_def).then(|| Msg::MakeDefault(v.id.clone()))),
            k::chip(&p, "Remove", Ch::Plain, Sz::Sm, can_remove.then(|| Msg::Pop(Pop::Remove(v.id.clone())))),
        ]
        .spacing(8.0)
        .align_y(iced::Center);
        c = c.push(container(r).padding(k::pad(7.0, 0.0, 7.0, 0.0)));
        if i < last {
            c = c.push(k::line(&p));
        }
    }
    if a.scanning && a.scan.vers.is_empty() {
        c = c.push(k::mu(&p, "Looking at what's here…", 12.0));
    } else if a.scan.vers.is_empty() {
        c = c.push(k::mu(&p, "No Proton or Wine here yet: Get more on the right.", 12.0));
    }
    k::card(&p, column![head, k::gap(1.0, 4.0), scrollable(c).height(Length::Fill)])
}

fn get_card(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let head = row![k::h2::<Msg>(&p, "Get more"), k::fill(), k::seg(&p, FAMILIES.iter().enumerate().map(|(i, f)| (f.tab.to_string(), a.family == i, Msg::Family(i))).collect())].align_y(iced::Center);
    let mut c = Column::new();
    let rels = &a.rels[a.family];
    let newest_new = rels.first().map(|r| !a.installed(&r.name)).unwrap_or(false);
    for (i, r) in rels.iter().enumerate().take(5) {
        let inst = a.installed(&r.name) || a.scan.vers.iter().any(|v| v.valve.is_some() && v.valve == r.valve);
        let mut name = Row::new().spacing(6.0).align_y(iced::Center).push(t(r.name.clone(), 12.0, p.text).wrapping(Wrapping::None));
        if i == 0 && newest_new && r.valve.is_none() {
            name = name.push(k::tag(&p, "New", Tag::Acc));
        }
        let line = if r.valve.is_some() {
            if inst { "installed · Steam's own".to_string() } else { "Steam installs it".to_string() }
        } else if inst {
            format!("{} · installed", r.day)
        } else if i == 0 {
            format!("{} · {}", r.day, if r.about.is_empty() { "the newest".to_string() } else { r.about.clone() })
        } else {
            format!("{} · older", r.day)
        };
        let downloading = a.dl.as_ref().map(|d| d.name == r.name).unwrap_or(false);
        let right: E<'_, Msg> = if inst {
            k::tag(&p, "Installed", Tag::Ok)
        } else {
            k::chip(&p, "Get", if i == 0 { Ch::Acc } else { Ch::Plain }, Sz::Sm, (a.dl.is_none() && !downloading).then_some(Msg::Get(a.family, i)))
        };
        if downloading {
            continue;
        }
        c = c.push(container(row![pmark(FAMILIES[a.family].kind), column![name, k::mu1::<Msg>(&p, line, 10.0)].spacing(1.0).width(Length::Fill), right].spacing(9.0).align_y(iced::Center)).padding(k::pad(7.0, 0.0, 7.0, 0.0)));
        c = c.push(k::line(&p));
    }
    if rels.is_empty() {
        let msg = if FAMILIES[a.family].tab == "Proton Hearth" && a.checked[0] { "No Proton Hearth releases are out yet." } else { "Looking for versions…" };
        c = c.push(container(k::mu(&p, msg, 12.0)).padding(k::pad(8.0, 0.0, 8.0, 0.0)));
    }
    // the one downloading, whichever tab it's from
    if let Some(d) = &a.dl {
        let frac = if d.size > 0 { d.got as f32 / d.size as f32 } else { 0.0 };
        let mb = |b: u64| (b as f64 / 1_048_576.0).round() as u64;
        let r = row![
            pmark(FAMILIES[d.family].kind),
            column![row![t(d.name.clone(), 12.0, p.text).wrapping(Wrapping::None), k::mu1::<Msg>(&p, if d.paused { "· paused" } else { "· downloading" }, 10.0)].spacing(4.0), k::gap(1.0, 4.0), k::bar(&p, frac, 4.0, Length::Fixed(210.0))].width(Length::Fill),
            k::mu1::<Msg>(&p, format!("{}% · {} of {} MB", (frac * 100.0).round() as u32, mb(d.got), mb(d.size)), 11.0),
            if d.paused { k::chip(&p, "Resume", Ch::Plain, Sz::Sm, Some(Msg::Resume)) } else { k::chip(&p, "Pause", Ch::Plain, Sz::Sm, Some(Msg::Pause)) },
        ]
        .spacing(9.0)
        .align_y(iced::Center);
        c = c.push(container(r).padding(k::pad(7.0, 0.0, 7.0, 0.0)));
    }
    k::card(&p, column![head, k::gap(1.0, 4.0), scrollable(c).height(Length::Fill)])
}

fn picks_card(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let q = a.find.to_lowercase();
    let head = row![
        k::h2::<Msg>(&p, "Each game's pick"),
        k::mu1::<Msg>(&p, format!("only games you changed are listed · {} of {}", a.scan.picks.len(), a.scan.total), 11.0),
        k::fill(),
        container(k::search(&p, "Find a game", &a.find, Msg::FindGame)).width(Length::Fixed(170.0)),
    ]
    .spacing(8.0)
    .align_y(iced::Center);
    let w = [Length::FillPortion(26), Length::FillPortion(22), Length::FillPortion(24), Length::FillPortion(22), Length::FillPortion(14)];
    let mut c = Column::new().push(container(row![container(col(&p, "Game")).width(w[0]), container(col(&p, "From")).width(w[1]), container(col(&p, "Runs with")).width(w[2]), container(col(&p, "Why")).width(w[3]), container(k::gap::<Msg>(1.0, 1.0)).width(w[4])].spacing(8.0)).padding(k::pad(4.0, 6.0, 6.0, 6.0))).push(k::line(&p));
    for pk in a.scan.picks.iter().filter(|x| q.is_empty() || x.name.to_lowercase().contains(&q)) {
        let runs = a.ver(&pk.runs).map(|v| v.name.clone()).unwrap_or_else(|| pretty(&pk.runs, None));
        let r = row![
            container(t(ui::fit(&pk.name, 30), 12.0, p.text).wrapping(Wrapping::None)).width(w[0]),
            container(k::mu1::<Msg>(&p, pk.from.clone(), 12.0)).width(w[1]),
            container(k::field(&p, format!("{runs} ▾"), 140.0, Some(Msg::Pop(Pop::Runs(pk.key.clone()))))).width(w[2]),
            container(k::mu1::<Msg>(&p, pk.why.clone(), 12.0)).width(w[3]),
            container(row![k::fill(), k::chip(&p, "Use default", Ch::Plain, Sz::Sm, Some(Msg::SetPick(pk.key.clone(), None)))]).width(w[4]),
        ]
        .spacing(8.0)
        .align_y(iced::Center);
        c = c.push(container(r).padding(k::pad(5.0, 6.0, 5.0, 6.0))).push(k::line(&p));
    }
    if a.scan.picks.is_empty() && !a.scanning {
        c = c.push(container(k::mu(&p, "Every game runs with the default.", 12.0)).padding(8.0));
    }
    k::card(&p, column![head, k::gap(1.0, 6.0), scrollable(c).height(Length::Fill)])
}

fn col<'a>(p: &Pal, s: &str) -> E<'a, Msg> {
    tb(s.to_uppercase(), 9.0, p.muted).wrapping(Wrapping::None).into()
}

fn how_card(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let def = a.default();
    let dn = a.ver(&def).map(|v| v.name.clone()).unwrap_or_else(|| if def.is_empty() { "Steam's own".into() } else { pretty(&def, None) });
    let mut c = Column::new().spacing(2.0).push(k::h2(&p, "How Hearth picks")).push(k::gap(1.0, 8.0));
    c = c.push(k::kv(&p, "For new games", k::field(&p, format!("{dn} ▾"), 150.0, Some(Msg::Pop(Pop::Default)))));
    for (label, key, d) in [("Get new versions by themselves", "auto", false), ("Tell me when a new version is out", "tell", true), ("Remove versions no game uses", "tidy", false), ("Show versions in Game Mode's game menu", "gameMenu", true)] {
        c = c.push(k::kv(&p, label, k::switch(&p, a.on(key, d), Some(Msg::Toggle(key)))));
    }
    c = c.push(k::kv(&p, "Where they live", k::mu1::<Msg>(&p, k::tilde(&runners_dir()), 11.0)));
    if a.on("tell", true) {
        if let Some((fam, i)) = a.newer() {
            let r = &a.rels[fam][i];
            let cur = a.ver(&def).map(|v| v.name.clone()).unwrap_or_default();
            let short = |n: &str| n.rsplit(' ').next().unwrap_or(n).to_string();
            let note = column![
                row![k::dot::<Msg>(p.accent, 10.0), tb(format!("{} is out", r.name), 12.0, p.text).wrapping(Wrapping::None), k::fill(), k::chip(&p, "Get it", Ch::Acc, Sz::Sm, a.dl.is_none().then_some(Msg::Get(fam, i)))].spacing(8.0).align_y(iced::Center),
                k::para(p.muted, format!("Your games stay on {} until you move them. Try it on one game first from that game's page.", short(&cur)), 11.0),
            ]
            .spacing(6.0);
            c = c.push(k::gap(1.0, 10.0)).push(k::note_card(&p, note, k::rgb("#2a2017"), k::rgb("#5a3b22"), false));
        }
    }
    k::card(&p, c)
}

fn page(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let title = column![
        k::mu1::<Msg>(&p, "Gaming ›", 11.0),
        row![k::h1::<Msg>(&p, "Proton and Wine"), k::gap(8.0, 1.0), k::tag(&p, "Proton Manager", Tag::Dk), k::fill(), k::chip(&p, "↻ Check for new versions", Ch::Plain, Sz::Md, Some(Msg::CheckAll))].align_y(iced::Center),
        k::para(p.muted, "What runs your Windows games. Hearth keeps the versions you have; nothing is added, removed or switched unless you say so.", 12.0),
    ]
    .spacing(3.0);
    let has_said = !a.said.is_empty();
    hearth_ui::arrange::parts(
        &a.describe,
        &p,
        "page",
        vec![("title", title.into()), ("installed", installed_card(a)), ("get", get_card(a)), ("picks", picks_card(a)), ("how", how_card(a)), ("said", k::said(&p, &a.said))],
        move |ps| {
            let top = row![ps.take("installed"), ps.take("get")].spacing(12.0).height(Length::FillPortion(10));
            let bottom = row![container(ps.take("picks")).width(Length::FillPortion(16)), container(ps.take("how")).width(Length::FillPortion(10))].spacing(12.0).height(Length::FillPortion(10));
            let mut c = column![ps.take("title"), top, bottom].spacing(12.0).padding(k::pad(14.0, 20.0, 10.0, 20.0));
            if has_said {
                c = c.push(ps.take("said"));
            }
            c.into()
        },
        Msg::Describe,
    )
}

fn pop_box(a: &App) -> Option<E<'_, Msg>> {
    let p = a.p;
    Some(match a.pop.clone()? {
        Pop::Default => {
            let d = a.default();
            k::chooser(&p, "For new games", a.scan.vers.iter().map(|v| (v.name.clone(), v.id == d || v.name == d, Msg::MakeDefault(v.id.clone()))).collect(), Msg::Close)
        }
        Pop::Runs(key) => {
            let cur = a.scan.picks.iter().find(|x| x.key == key).map(|x| x.runs.clone()).unwrap_or_default();
            let steam = key.starts_with("steam:") || key.starts_with("hearth:");
            let opts = a.scan.vers.iter().filter(|v| !steam || v.kind == Kind::Proton).map(|v| (v.name.clone(), v.id == cur || v.name == cur || v.lutris.as_deref() == Some(cur.as_str()), Msg::SetPick(key.clone(), Some(v.id.clone())))).collect();
            k::chooser(&p, "Runs with", opts, Msg::Close)
        }
        Pop::Remove(id) => {
            let v = a.ver(&id)?;
            let body = column![
                k::h2::<Msg>(&p, format!("Remove {}?", v.name)),
                k::para(p.muted, if v.valve.is_some() { "Steam removes it (it's Steam's own). No game uses it.".to_string() } else { format!("No game uses it. It frees {}.", k::size_words(v.size)) }, 12.0),
                row![k::fill(), k::chip(&p, "Cancel", Ch::Plain, Sz::Md, Some(Msg::Close)), k::chip(&p, "Remove", Ch::Acc, Sz::Md, Some(Msg::Remove(id.clone())))].spacing(8.0),
            ]
            .spacing(10.0);
            ui::dialog_box(&p, 420.0, body)
        }
    })
}

fn view(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    studio_values(a);
    let mut items: Vec<(&'static str, E<'_, Msg>)> = vec![("side", hearth_ui::studio::made(&a.describe, "window", "side", || side(a))), ("page", page(a))];
    for (id, which) in [("pane:installed", "installed"), ("pane:picks", "picks")] {
        if a.describe.wants_pane(which) {
            items.push((id, studio_pane(a, which)));
        }
    }
    let mut body = hearth_ui::arrange::parts(&a.describe, &p, "window", items, move |ps| row![ps.take("side"), ui::vline(p.line), ps.take("page")].into(), Msg::Describe);
    if let Some(d) = pop_box(a) {
        body = ui::dialog(body, d, Some(Msg::Close));
    }
    let bar = k::Bar {
        letters: "⚙",
        from: k::rgb("#c9c1ba"),
        to: k::rgb("#6d645d"),
        title: "Settings",
        line: String::new(),
        middle: None,
        right: vec![],
        msgs: TitleMsgs { drag: Msg::Drag, maximize: Msg::Max, minimize: Some(Msg::Min), close: Msg::WClose },
        open: Msg::Describe(hearth_ui::describe::DMsg::Open),
    };
    k::window(&p, &a.describe, bar, body, ("Proton Manager in Settings › Gaming", "Built on ProtonUp-Qt (GPL-3)"), Msg::Resize, Msg::Describe)
}

use hearth_ui::blocks::{bl, bls, PageDef};
const BLOCKS: &[PageDef] = &[
    PageDef { id: "window", title: "Proton Manager", blocks: &[bl("side", "Settings' side", 0, 0, 2, 12), bl("page", "Proton and Wine", 2, 0, 10, 12)] },
    PageDef { id: "side", title: "Settings' side", blocks: &[bl("me", "You and this PC", 0, 0, 12, 1), bl("find", "Find a setting", 0, 1, 12, 1), bls("nav", "Settings' pages", 0, 2, 12, 9), bl("edit", "Edit layout", 0, 11, 12, 1)] },
    PageDef {
        id: "page",
        title: "Proton and Wine",
        blocks: &[bl("title", "Its name and Check for new versions", 0, 0, 12, 2), bls("installed", "Installed", 0, 2, 6, 5), bls("get", "Get more", 6, 2, 6, 5), bls("picks", "Each game's pick", 0, 7, 7, 5), bl("how", "How Hearth picks", 7, 7, 5, 5), bl("said", "What happened", 0, 11, 12, 1)],
    },
];

use hearth_ui::studio::{tile, Kit};
const KIT: Kit = Kit {
    name: "Proton Manager",
    buttons: &[
        tile("check", "Check for new versions", G::Sync),
        tile("get-newest", "Get the newest", G::Down),
        tile("pause", "Pause the download", G::Stop),
        tile("resume", "Resume the download", G::Play),
        tile("all", "Show all", G::List),
        tile("proton", "Show Proton", G::Rocket),
        tile("wine", "Show Wine", G::Cog),
        tile("default", "Pick the default", G::Check),
    ],
    panes: &[tile("installed", "What's installed", G::List), tile("picks", "Games with their own pick", G::Grid)],
    info: &[tile("installed", "Installed", G::Drive), tile("size", "Space they use", G::Drive), tile("default", "The default", G::Check), tile("picks", "Games with their own pick", G::Grid)],
    pane_page: "window",
    bars: &[],
    menus: &[],
};

fn block_action(id: &str) -> Option<Msg> {
    Some(match id {
        "check" => Msg::CheckAll,
        "get-newest" => Msg::Get(1, 0),
        "pause" => Msg::Pause,
        "resume" => Msg::Resume,
        "all" => Msg::Filter(0),
        "proton" => Msg::Filter(1),
        "wine" => Msg::Filter(2),
        "default" => Msg::Pop(Pop::Default),
        _ => return None,
    })
}

fn studio_values(a: &App) {
    let total: u64 = a.scan.vers.iter().map(|v| v.size).sum();
    let d = a.default();
    a.describe.values(vec![("installed", a.scan.vers.len().to_string()), ("size", k::size_words(total)), ("default", a.ver(&d).map(|v| v.name.clone()).unwrap_or(d)), ("picks", a.scan.picks.len().to_string())]);
}

fn studio_pane<'a>(a: &'a App, which: &str) -> E<'a, Msg> {
    let p = a.p;
    let mut c = Column::new().spacing(5.0);
    if which == "installed" {
        for v in &a.scan.vers {
            c = c.push(row![pmark(v.kind), t(v.name.clone(), 12.0, p.text), k::fill(), k::mu1::<Msg>(&p, k::size_words(v.size), 10.0)].spacing(8.0).align_y(iced::Center));
        }
    } else {
        for pk in &a.scan.picks {
            c = c.push(row![t(ui::fit(&pk.name, 28), 12.0, p.text), k::fill(), k::mu1::<Msg>(&p, a.ver(&pk.runs).map(|v| v.name.clone()).unwrap_or(pk.runs.clone()), 10.0)].spacing(8.0));
        }
    }
    scrollable(c).height(Length::Fill).into()
}

fn subscription(a: &App) -> Subscription<Msg> {
    let mut s = vec![Subscription::run(hearth_ui::theme::watch).map(|_| Msg::Theme)];
    // only while something downloads: how far it got, twice a second
    if a.dl.as_ref().map(|d| !d.paused).unwrap_or(false) {
        s.push(iced::time::every(std::time::Duration::from_millis(500)).map(|_| Msg::Tick));
    }
    Subscription::batch(s)
}

fn main() -> iced::Result {
    if app::handle_cli("hearth-proton-manager", "Proton Manager (Settings › Gaming › Proton and Wine): the Proton and Wine versions your Windows games run with (built on ProtonUp-Qt).") {
        return Ok(());
    }
    let w = app::window("hearth-proton-manager", (1280.0, 800.0), (960.0, 600.0));
    iced::application(App::new, update, view).title(|_: &App| "Proton and Wine · Settings".to_string()).subscription(subscription).theme(|a: &App| app::iced_theme(&a.p)).window(w).default_font(app::INTER).run()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_as_people_say_them() {
        assert_eq!(pretty("Proton-Hearth9-4", Some(Kind::Proton)), "Proton Hearth 9-4");
        assert_eq!(pretty("GE-Proton9-20", Some(Kind::Proton)), "Proton-GE 9-20");
        assert_eq!(pretty("lutris-GE-Proton8-26-x86_64", Some(Kind::Wine)), "Wine-GE 8-26");
        assert_eq!(pretty("proton-9.0-3", Some(Kind::Proton)), "Proton 9.0-3");
        assert_eq!(pretty("wine-9.20", Some(Kind::Wine)), "Wine 9.20");
        assert_eq!(pretty("luxtorpeda-v71", Some(Kind::Proton)), "Luxtorpeda 71");
    }

    #[test]
    fn releases_from_github() {
        let j = json!([{"tag_name": "GE-Proton9-21", "published_at": "2026-11-20T00:00:00Z", "body": "## Fixes\n- faster", "assets": [{"name": "GE-Proton9-21.tar.gz", "size": 828375040, "browser_download_url": "https://x/GE-Proton9-21.tar.gz"}, {"name": "GE-Proton9-21.sha512sum", "browser_download_url": "https://x/s"}]}, {"tag_name": "draft", "draft": true, "assets": []}]);
        let r = releases_of(1, &j);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].name, "Proton-GE 9-21");
        assert_eq!(r[0].day, "Nov 20");
        assert_eq!(r[0].about, "Fixes");
        assert_eq!(r[0].sha, "https://x/s");
        let w = releases_of(2, &json!([{"tag_name": "GE-Proton8-26", "published_at": "2026-01-01", "assets": [{"name": "wine-lutris-GE-Proton8-26-x86_64.tar.xz", "size": 1, "browser_download_url": "u"}]}]));
        assert_eq!(w[0].name, "Wine-GE 8-26");
    }
}
