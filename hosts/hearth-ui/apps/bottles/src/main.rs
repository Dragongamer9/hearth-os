//! hearth-bottles: Hearth Bottles (docs/mockups/checklists/open-source-08-bottles.md; the approved board
//! docs/mockups/open-source-apps/08-bottles.png). Windows apps, each in its own bottle: your bottles, the programs in
//! each, its extras, snapshots to go back to, what's running in it now, getting a program, and the bottle's settings.
//! Built on Bottles (GPL-3): every bottle is Bottles' own (its bottle.yml, read and changed one line at a time with the
//! safe writer; red flag 9) and runs through Bottles' command (`bottles-cli`, its own or its Flathub app's); the
//! community's installers are Bottles' programs list (bottlesdevs/programs). Snapshots and backups are Hearth's own
//! (a copy of the bottle you can go back to), kept in ~/.local/share/hearth/bottles-snapshots.
//!
//! Made of blocks: the side, the bottle's head, its programs, Extras installed, Snapshots, Running now, Get a program
//! and This bottle's settings, each moved, sized, hidden and customized (Edit layout), with its own buttons, panes and
//! info in its blocks window. `--bottle NAME` opens that bottle.

mod installer;

use hearth_oss_kit_a::need::{self, Have, Need};
use hearth_oss_kit_a::{self as k, yaml, Ch, Sz, Tag, E};
use hearth_ui::app::{self, win};
use hearth_ui::run;
use hearth_ui::ui::{self, t, tb, TitleMsgs, G};
use hearth_ui::Pal;
use iced::widget::text::Wrapping;
use iced::widget::{column, container, row, scrollable, Column, Row, Space};
use iced::{Color, Element, Length, Subscription, Task};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

const APP: &str = "bottles";
const BOTTLES: Need = Need { name: "Bottles", cmd: "bottles-cli", flatpak: "com.usebottles.bottles", flatpak_cmd: "bottles-cli", licence: "GPL-3" };
const MARKS: [(&str, &str); 6] = [("#7fd6ff", "#2c7fd8"), ("#ffb36b", "#ff6a1f"), ("#ff9be8", "#b04fc8"), ("#b8f07f", "#3ba55c"), ("#c9c1ba", "#6d645d"), ("#ffe28a", "#f0a020")];
const WINDOWS: [(&str, &str); 6] = [("win11", "Windows 11"), ("win10", "Windows 10"), ("win81", "Windows 8.1"), ("win7", "Windows 7"), ("winxp", "Windows XP"), ("win98", "Windows 98")];
const FOLDERS: [&str; 6] = ["Documents", "Downloads", "Pictures", "Music", "Videos", "Desktop"];
const EXTRAS: [(&str, &str); 8] = [("dotnet48", ".NET Framework 4.8"), ("vcrun2022", "Visual C++ 2015–2022"), ("corefonts", "Core fonts (Arial, Times…)"), ("mf", "Media Foundation"), ("d3dx9", "DirectX 9 parts"), ("vcrun2005", "Visual C++ 2005"), ("dotnet40", ".NET Framework 4.0"), ("xact", "XAudio (XACT)")];

#[derive(Debug, Clone, Default)]
struct Prog {
    id: String,
    name: String,
    exe: String,
    path: String,
}

#[derive(Debug, Clone, Default)]
struct Bottle {
    name: String,
    dir: PathBuf,
    file: PathBuf,
    yml: String,
    progs: Vec<Prog>,
    size: u64,
}

impl Bottle {
    fn get(&self, p: &[&str]) -> Option<String> {
        yaml::yget(&self.yml, p)
    }
    fn kind(&self) -> String {
        match self.get(&["Environment"]).unwrap_or_default().to_lowercase().as_str() {
            "gaming" => "Games".into(),
            "application" => "Apps".into(),
            "" | "custom" => if self.progs.is_empty() { "Empty".into() } else { "Custom".into() },
            o => o.to_string(),
        }
    }
    fn runner(&self) -> String {
        pretty_runner(&self.get(&["Runner"]).unwrap_or_default())
    }
    fn flag(&self, p: &[&str]) -> bool {
        self.get(p).map(|v| v == "true").unwrap_or(false)
    }
}

/// A runner's folder name as people say it ("wine-ge-8-26" → Wine-GE 8-26, "soda-7.0-9" → Soda 7.0-9).
fn pretty_runner(r: &str) -> String {
    let s = r.trim().trim_end_matches("-x86_64");
    let low = s.to_lowercase();
    for (pre, name) in [("proton-hearth", "Proton Hearth "), ("wine-ge-proton", "Wine-GE "), ("wine-ge-", "Wine-GE "), ("lutris-ge-proton", "Wine-GE "), ("ge-proton", "Proton-GE "), ("soda-", "Soda "), ("caffe-", "Caffe "), ("sys-wine-", "Wine "), ("wine-", "Wine ")] {
        if let Some(rest) = low.strip_prefix(pre) {
            return format!("{name}{}", s[s.len() - rest.len()..].trim_start_matches(['-', '_']));
        }
    }
    s.to_string()
}

/// Bottles' own folders (native and Flathub) and Hearth's.
fn bottle_dirs() -> Vec<PathBuf> {
    let h = k::home();
    let data = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|| h.join(".local/share"));
    vec![data.join("hearth/bottles"), data.join("bottles/bottles"), h.join(".var/app/com.usebottles.bottles/data/bottles/bottles")]
}
fn runner_dirs() -> Vec<PathBuf> {
    let h = k::home();
    let data = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|| h.join(".local/share"));
    vec![data.join("bottles/runners"), h.join(".var/app/com.usebottles.bottles/data/bottles/runners")]
}

fn read_bottles() -> Vec<Bottle> {
    let mut v: Vec<Bottle> = Vec::new();
    for d in bottle_dirs() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        let mut ents: Vec<_> = rd.flatten().collect();
        ents.sort_by_key(|e| std::fs::metadata(e.path().join("bottle.yml")).and_then(|m| m.created().or_else(|_| m.modified())).unwrap_or(std::time::UNIX_EPOCH));
        for e in ents {
            let file = e.path().join("bottle.yml");
            let Ok(yml) = std::fs::read_to_string(&file) else { continue };
            let name = yaml::yget(&yml, &["Name"]).unwrap_or_else(|| e.file_name().to_string_lossy().to_string());
            if v.iter().any(|b| b.name == name) {
                continue;
            }
            let progs = yaml::ymap(&yml, &["External_Programs"])
                .into_iter()
                .map(|(id, _)| Prog {
                    name: yaml::yget(&yml, &["External_Programs", &id, "name"]).unwrap_or_default(),
                    exe: yaml::yget(&yml, &["External_Programs", &id, "executable"]).unwrap_or_default(),
                    path: yaml::yget(&yml, &["External_Programs", &id, "path"]).unwrap_or_default(),
                    id,
                })
                .filter(|p| !p.name.is_empty())
                .collect();
            v.push(Bottle { name, dir: e.path(), file, yml, progs, size: 0 });
        }
    }
    // oldest first (Bottles' own Creation_Date)
    v.sort_by_key(|b| b.get(&["Creation_Date"]).unwrap_or_default());
    v
}

/// A program running in a bottle: its process, words, memory, its CPU ticks.
#[derive(Debug, Clone)]
struct Proc {
    pid: u32,
    words: String,
    mem: u64,
    ticks: u64,
    cpu: f32,
}

fn running_in(dir: &Path) -> Vec<Proc> {
    let mut v = Vec::new();
    let want = std::fs::canonicalize(dir).unwrap_or(dir.to_path_buf()).to_string_lossy().to_string();
    let Ok(rd) = std::fs::read_dir("/proc") else { return v };
    for e in rd.flatten() {
        let Ok(pid) = e.file_name().to_string_lossy().parse::<u32>() else { continue };
        let Ok(env) = std::fs::read(e.path().join("environ")) else { continue };
        let pre = env.split(|b| *b == 0).filter_map(|x| std::str::from_utf8(x).ok()).find_map(|x| x.strip_prefix("WINEPREFIX=").map(str::to_string));
        let Some(pre) = pre else { continue };
        if std::fs::canonicalize(&pre).map(|p| p.to_string_lossy().to_string()).unwrap_or(pre) != want {
            continue;
        }
        let cmd: Vec<String> = std::fs::read(e.path().join("cmdline")).unwrap_or_default().split(|b| *b == 0).filter(|x| !x.is_empty()).map(|x| String::from_utf8_lossy(x).to_string()).collect();
        let Some(exe) = cmd.iter().find(|a| a.to_lowercase().ends_with(".exe")).map(|a| a.rsplit(['/', '\\']).next().unwrap_or(a).to_string()) else { continue };
        if ["wineserver", "services.exe", "winedevice.exe", "plugplay.exe", "explorer.exe", "rpcss.exe", "svchost.exe", "start.exe", "conhost.exe", "tabtip.exe"].contains(&exe.to_lowercase().as_str()) {
            continue;
        }
        let file = cmd.last().filter(|a| !a.to_lowercase().ends_with(".exe") && a.contains('.')).map(|a| a.rsplit(['/', '\\']).next().unwrap_or(a).to_string());
        let mem = std::fs::read_to_string(e.path().join("status")).ok().and_then(|s| s.lines().find(|l| l.starts_with("VmRSS:")).and_then(|l| l.split_whitespace().nth(1).and_then(|n| n.parse::<u64>().ok()))).unwrap_or(0) * 1024;
        let ticks = std::fs::read_to_string(e.path().join("stat")).ok().and_then(|s| s.rsplit(')').next().map(|r| r.split_whitespace().skip(11).take(2).filter_map(|x| x.parse::<u64>().ok()).sum())).unwrap_or(0);
        let words = match file {
            Some(f) => format!("{} · {f}", exe.trim_end_matches(".exe").trim_end_matches(".EXE")),
            None => exe.trim_end_matches(".exe").trim_end_matches(".EXE").to_string(),
        };
        v.push(Proc { pid, words, mem, ticks, cpu: 0.0 });
    }
    v
}

#[derive(Debug, Clone, PartialEq)]
enum Pop {
    Runner,
    Windows,
    Shared,
    Extras,
    New,
    More,
    ProgMore(String),
    Delete,
    Take,
    GoBack(String),
    Dropped(String),
    Get(usize),
    Installing,
}

#[derive(Debug, Clone)]
enum Msg {
    Describe(hearth_ui::describe::DMsg),
    Theme,
    Have(Have),
    Reload,
    Sized(Vec<(String, u64)>),
    Pick(String),
    Find(String),
    RunProgram,
    Picked(Option<String>),
    RunExe(String, String),
    Ran(run::Output),
    Run(String),
    OpenC,
    Pop(Pop),
    Close,
    Text(String),
    Kind(&'static str),
    NewBottle,
    MenuEntry(String),
    RemoveProg(String),
    Extra(&'static str),
    ExtraDone(&'static str, run::Output),
    Snapshot,
    Snapped(Result<String, String>),
    GoBack(String),
    WentBack(Result<String, String>),
    Tick,
    Stop(u32),
    StopAll,
    Installers(run::Output),
    Manifest(run::Output),
    InstallTick,
    GetProgram(usize),
    Runner(String),
    Windows(&'static str),
    Graphics(u8),
    Param(&'static str),
    Home,
    Net,
    Folder(&'static str),
    Shows(&'static str),
    Backup,
    Copy,
    Delete,
    Done(String, Result<String, String>),
    Tool(&'static str),
    Dropped(PathBuf),
    GetBottles,
    GotBottles(run::Output),
    Drag,
    Max,
    Min,
    WClose,
    Resize(iced::window::Direction),
}

struct App {
    describe: hearth_ui::describe::Describe,
    p: Pal,
    have: Have,
    bottles: Vec<Bottle>,
    pick: String,
    find: String,
    pop: Option<Pop>,
    text: String,
    new_kind: &'static str,
    procs: Vec<Proc>,
    installers: Vec<Value>,
    /// a community installer Hearth is running (installer.rs): its name and its steps' states
    installing: Option<(String, installer::Progress)>,
    install_done: bool,
    notes: Value,
    said: String,
    busy: bool,
}

fn obj(v: Value) -> serde_json::Map<String, Value> {
    v.as_object().cloned().unwrap_or_default()
}
fn snaps_dir(b: &str) -> PathBuf {
    k::data_dir("bottles-snapshots").join(b.replace('/', "_"))
}
fn snapshots(b: &str) -> Vec<(String, String, i64)> {
    let mut v: Vec<(String, String, i64)> = std::fs::read_dir(snaps_dir(b))
        .map(|r| {
            r.flatten()
                .filter_map(|e| {
                    let f = e.file_name().to_string_lossy().to_string();
                    let stem = f.strip_suffix(".tar.zst").or_else(|| f.strip_suffix(".tar"))?;
                    let (ts, name) = stem.split_once('-')?;
                    Some((f.clone(), name.replace('_', " "), ts.parse().ok()?))
                })
                .collect()
        })
        .unwrap_or_default();
    v.sort_by_key(|x| std::cmp::Reverse(x.2));
    v
}

/// Off the window's thread, then a message.
fn thread<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static, m: impl Fn(T) -> Msg + Send + 'static) -> Task<Msg> {
    let (tx, rx) = futures::channel::oneshot::channel();
    let _ = std::thread::Builder::new().name("bottles-work".into()).spawn(move || {
        let _ = tx.send(f());
    });
    Task::perform(async move { rx.await.ok() }, move |r| match r {
        Some(v) => m(v),
        None => Msg::Reload,
    })
}

fn when_words(ts: i64) -> String {
    let now = hearth_ui::time::now();
    let a = hearth_ui::time::local(ts);
    let n = hearth_ui::time::local(now);
    if a.year == n.year && a.mon == n.mon && a.day == n.day {
        format!("Today {}", hearth_ui::time::clock(ts))
    } else {
        hearth_ui::time::day(ts)
    }
}
fn ago(ts: i64) -> String {
    let s = (hearth_ui::time::now() - ts).max(0);
    if s < 3600 {
        format!("{} min ago", (s / 60).max(1))
    } else if s < 86400 * 2 {
        format!("{} h ago", s / 3600)
    } else {
        format!("{} days ago", s / 86400)
    }
}

impl App {
    fn new() -> (App, Task<Msg>) {
        let mut a = App {
            describe: hearth_ui::describe::Describe::new(APP, "Hearth Bottles").with_pages(BLOCKS).with_kit(&KIT),
            p: hearth_ui::theme::load(),
            have: Have::Looking,
            bottles: Vec::new(),
            pick: app::arg("--bottle").unwrap_or_default(),
            find: String::new(),
            pop: None,
            text: String::new(),
            new_kind: "application",
            procs: Vec::new(),
            installers: Vec::new(),
            installing: None,
            install_done: false,
            notes: k::read_settings(APP),
            said: String::new(),
            busy: false,
        };
        let t = a.reload();
        let idx = Task::perform(run::spawn(run::args(&["curl", "-fsSL", "-m", "20", "https://raw.githubusercontent.com/bottlesdevs/programs/main/index.yml"])), Msg::Installers);
        (a, Task::batch([Task::perform(need::probe(&BOTTLES), Msg::Have), t, idx]))
    }
    fn reload(&mut self) -> Task<Msg> {
        self.bottles = read_bottles();
        if !self.bottles.iter().any(|b| b.name == self.pick) {
            self.pick = self.bottles.first().map(|b| b.name.clone()).unwrap_or_default();
        }
        self.notes = k::read_settings(APP);
        self.refresh_procs();
        let dirs: Vec<(String, PathBuf)> = self.bottles.iter().map(|b| (b.name.clone(), b.dir.clone())).collect();
        thread(move || dirs.into_iter().map(|(n, d)| (n, k::folder_size(&d))).collect(), Msg::Sized)
    }
    fn bottle(&self) -> Option<&Bottle> {
        self.bottles.iter().find(|b| b.name == self.pick)
    }
    fn cli(&self, args: &[&str]) -> Vec<String> {
        need::argv(&BOTTLES, &self.have, args)
    }
    fn note(&self, path: &str) -> Option<&Value> {
        self.notes.pointer(path)
    }
    fn refresh_procs(&mut self) {
        let Some(b) = self.bottle() else {
            self.procs.clear();
            return;
        };
        let mut now = running_in(&b.dir);
        for p in now.iter_mut() {
            if let Some(o) = self.procs.iter().find(|x| x.pid == p.pid) {
                p.cpu = (p.ticks.saturating_sub(o.ticks)) as f32 / 2.0;
            }
        }
        self.procs = now;
    }
    /// The bottle's settings changed (bottle.yml one line at a time, atomic, .bak).
    fn change(&mut self, f: impl FnOnce(&str) -> String) {
        let Some(b) = self.bottle().cloned() else { return };
        if let Err(e) = k::settings_file::update_text(&b.file, f) {
            self.said = format!("Couldn't change {}: {e}", b.name);
        }
        let _ = self.reload();
    }
    fn save(&mut self, keys: Value) {
        if let Err(e) = k::save_settings(APP, obj(keys)) {
            self.said = format!("Couldn't save that: {e}");
        }
        self.notes = k::read_settings(APP);
    }
    fn wine_of(&self, b: &Bottle) -> String {
        let r = b.get(&["Runner"]).unwrap_or_default();
        for d in runner_dirs() {
            let w = d.join(&r).join("bin/wine");
            if w.is_file() {
                return w.to_string_lossy().to_string();
            }
        }
        "wine".into()
    }
}

/// A desktop entry for a program (Apps menu; with the file types it opens, Files' Open with).
fn entry_path(b: &str, prog: &str) -> PathBuf {
    let slug: String = format!("{b}-{prog}").chars().map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { '-' }).collect();
    k::home().join(format!(".local/share/applications/hearth-bottles-{slug}.desktop"))
}
fn write_entry(cli: &[String], b: &str, p: &Prog, mimes: &str) -> std::io::Result<bool> {
    let f = entry_path(b, &p.name);
    if let Some(d) = f.parent() {
        std::fs::create_dir_all(d)?;
    }
    let exec: Vec<String> = cli.iter().map(|a| if a.contains(' ') { format!("\"{a}\"") } else { a.clone() }).collect();
    let body = format!(
        "[Desktop Entry]\nType=Application\nName={}\nComment=In the bottle {b} (Hearth Bottles)\nExec={} run -b \"{b}\" -p \"{}\"{}\nIcon=hearth-bottles\nCategories=Wine;\n{}",
        p.name,
        exec.join(" "),
        p.name,
        if mimes.is_empty() { "" } else { " -- %f" },
        if mimes.is_empty() { String::new() } else { format!("MimeType={mimes}\n") }
    );
    k::settings_file::write_text(&f, &body)
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
        Msg::Have(h) => a.have = h,
        Msg::Reload => return a.reload(),
        Msg::Sized(v) => {
            for (n, s) in v {
                if let Some(b) = a.bottles.iter_mut().find(|b| b.name == n) {
                    b.size = s;
                }
            }
        }
        Msg::Pick(n) => {
            a.pick = n;
            a.procs.clear();
            a.refresh_procs();
            a.said.clear();
        }
        Msg::Find(s) => a.find = s,
        Msg::RunProgram => return Task::perform(k::pick_file("A Windows program", &["*.exe", "*.msi", "*.bat"]), Msg::Picked),
        Msg::Picked(Some(path)) => {
            let b = a.pick.clone();
            return update(a, Msg::RunExe(b, path));
        }
        Msg::Picked(None) => {}
        Msg::RunExe(b, path) => {
            a.pop = None;
            let low = path.to_lowercase();
            let installer = low.ends_with(".msi") || low.contains("setup") || low.contains("install");
            let snap_first = a.bottles.iter().find(|x| x.name == b).map(|x| x.flag(&["Parameters", "versioning_automatic"])).unwrap_or(false);
            let cmd = a.cli(&["run", "-b", &b, "-e", &path]);
            a.said = format!("Starting {} in {b}…", Path::new(&path).file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default());
            if installer && snap_first {
                let name = format!("Before installing {}", Path::new(&path).file_stem().map(|f| f.to_string_lossy().to_string()).unwrap_or_default());
                let dir = a.bottles.iter().find(|x| x.name == b).map(|x| x.dir.clone()).unwrap_or_default();
                let bn = b.clone();
                return thread(move || snapshot(&bn, &dir, &name), Msg::Snapped).chain(Task::perform(run::spawn(cmd), Msg::Ran));
            }
            return Task::perform(run::spawn(cmd), Msg::Ran);
        }
        Msg::Ran(o) => {
            if !o.ok {
                a.said = format!("It didn't start: {}", o.why());
            }
            a.refresh_procs();
        }
        Msg::Run(prog) => {
            let b = a.pick.clone();
            let cmd = a.cli(&["run", "-b", &b, "-p", &prog]);
            a.said = format!("Starting {prog}…");
            return Task::perform(run::spawn(cmd), Msg::Ran);
        }
        Msg::OpenC => {
            if let Some(b) = a.bottle() {
                run::detached(&["xdg-open", &b.dir.join("drive_c").to_string_lossy()]);
            }
        }
        Msg::Pop(p) => {
            a.text = match &p {
                Pop::New => String::new(),
                Pop::Take => "Taken by hand".into(),
                Pop::Dropped(_) => String::new(),
                _ => a.text.clone(),
            };
            a.pop = Some(p);
        }
        Msg::Close => {
            if a.pop == Some(Pop::Installing) && a.install_done {
                a.installing = None;
                a.install_done = false;
            }
            a.pop = None;
        }
        Msg::Text(s) => a.text = s,
        Msg::Kind(kd) => a.new_kind = kd,
        Msg::NewBottle => {
            let name = a.text.trim().to_string();
            if name.is_empty() {
                return Task::none();
            }
            a.pop = None;
            a.busy = true;
            a.said = format!("Making the bottle {name}…");
            let kind = a.new_kind;
            let cmd = a.cli(&["new", "--bottle-name", &name, "--environment", kind]);
            a.pick = name.clone();
            return Task::perform(run::spawn(cmd), move |o| Msg::Done("new".into(), if o.ok { Ok(String::new()) } else { Err(o.why()) }));
        }
        Msg::MenuEntry(prog) => {
            let Some(b) = a.bottle().cloned() else { return Task::none() };
            let Some(pg) = b.progs.iter().find(|p| p.name == prog).cloned() else { return Task::none() };
            let f = entry_path(&b.name, &pg.name);
            if f.exists() {
                let _ = std::fs::remove_file(&f);
            } else {
                let mimes = a.note(&format!("/opens/{}", pg.name.replace('/', "~1"))).and_then(Value::as_str).unwrap_or("").to_string();
                let files_on = a.note(&format!("/shows/{}/files", b.name)).and_then(Value::as_bool).unwrap_or(true);
                if let Err(e) = write_entry(&a.cli(&[]), &b.name, &pg, if files_on { &mimes } else { "" }) {
                    a.said = format!("Couldn't add it to the Apps menu: {e}");
                }
            }
        }
        Msg::RemoveProg(id) => {
            a.pop = None;
            a.change(|t| yaml::yset(t, &["External_Programs", &id], None));
        }
        Msg::Extra(verb) => {
            a.pop = None;
            let Some(b) = a.bottle().cloned() else { return Task::none() };
            a.busy = true;
            a.said = format!("Installing {} into {}…", EXTRAS.iter().find(|x| x.0 == verb).map(|x| x.1).unwrap_or(verb), b.name);
            let wine = a.wine_of(&b);
            let cmd = vec!["env".to_string(), format!("WINEPREFIX={}", b.dir.display()), format!("WINE={wine}"), "winetricks".into(), "-q".into(), verb.into()];
            return Task::perform(run::spawn(cmd), move |o| Msg::ExtraDone(verb, o));
        }
        Msg::ExtraDone(verb, o) => {
            a.busy = false;
            if o.ok {
                // Bottles' own list of what's in the bottle
                a.change(|t| {
                    let have = yaml::ylist(t, &["Installed_Dependencies"]);
                    if have.iter().any(|x| x == verb) {
                        return t.to_string();
                    }
                    let mut items = have;
                    items.push(verb.to_string());
                    let body: String = items.iter().map(|i| format!("- {i}\n")).collect();
                    let mut out = yaml::yset(t, &["Installed_Dependencies"], None);
                    out.push_str(&format!("Installed_Dependencies:\n{body}"));
                    out
                });
                a.said = "Installed.".into();
            } else {
                a.said = if o.err.contains("winetricks: ") || o.code == 127 { "Extras are installed with winetricks, which isn't on this device.".into() } else { format!("It didn't install: {}", o.why()) };
            }
        }
        Msg::Snapshot => {
            a.pop = None;
            let Some(b) = a.bottle().cloned() else { return Task::none() };
            let name = if a.text.trim().is_empty() { "Taken by hand".to_string() } else { a.text.trim().to_string() };
            a.busy = true;
            a.said = format!("Taking a snapshot of {}…", b.name);
            return thread(move || snapshot(&b.name, &b.dir, &name), Msg::Snapped);
        }
        Msg::Snapped(r) => {
            a.busy = false;
            a.said = match r {
                Ok(n) => format!("Snapshot kept: {n}."),
                Err(e) => format!("Couldn't take the snapshot: {e}"),
            };
        }
        Msg::GoBack(file) => {
            a.pop = None;
            let Some(b) = a.bottle().cloned() else { return Task::none() };
            if !a.procs.is_empty() {
                a.said = "Stop what's running in the bottle first.".into();
                return Task::none();
            }
            a.busy = true;
            return thread(move || go_back(&b.name, &b.dir, &file), Msg::WentBack);
        }
        Msg::WentBack(r) => {
            a.busy = false;
            a.said = match r {
                Ok(n) => format!("The bottle is as it was: {n}. What it had a moment ago is kept as a snapshot too."),
                Err(e) => format!("Couldn't go back: {e}"),
            };
            return a.reload();
        }
        Msg::Tick => a.refresh_procs(),
        Msg::Stop(pid) => {
            unsafe {
                libc_kill(pid);
            }
            a.refresh_procs();
        }
        Msg::StopAll => {
            for p in a.procs.clone() {
                unsafe {
                    libc_kill(p.pid);
                }
            }
            a.refresh_procs();
        }
        Msg::Installers(o) => {
            // Bottles' programs list: "Name: {Description, Grade, Category, Icon}" (YAML)
            if o.ok {
                let mut v = Vec::new();
                let ls = o.out.lines().collect::<Vec<_>>();
                for (i, l) in ls.iter().enumerate() {
                    if !l.starts_with(' ') && l.ends_with(':') && !l.starts_with('#') {
                        let key = l.trim_end_matches(':').trim().to_string();
                        let block: String = ls[i + 1..].iter().take_while(|x| x.starts_with(' ')).map(|x| format!("{x}\n")).collect();
                        let grade = yaml::yget(&block, &["Grade"]).unwrap_or_default();
                        v.push(json!({"key": key, "name": yaml::yget(&block, &["Name"]).unwrap_or(key.clone()), "about": yaml::yget(&block, &["Description"]).unwrap_or_default(), "grade": grade, "category": yaml::yget(&block, &["Category"]).unwrap_or_else(|| "Software".into())}));
                    }
                }
                // the ones people checked first (Platinum, Gold)
                v.sort_by_key(|x| match x["grade"].as_str().unwrap_or("") {
                    "Platinum" => 0,
                    "Gold" => 1,
                    "Silver" => 2,
                    _ => 3,
                });
                a.installers = v;
                // --install NAME: that community installer, straight away (CI's pictures; a link from the Store)
                if let Some(key) = app::arg("--install") {
                    if let Some(i) = a.installers.iter().position(|x| x["key"].as_str() == Some(key.as_str())) {
                        return update(a, Msg::GetProgram(i));
                    }
                }
            }
        }
        Msg::GetProgram(i) => {
            a.pop = None;
            let Some(inst) = a.installers.get(i).cloned() else { return Task::none() };
            let key = inst["key"].as_str().unwrap_or("").to_string();
            let cat = inst["category"].as_str().unwrap_or("Software").to_string();
            // the community's manifest for it, run by Hearth step by step in this bottle (installer.rs)
            let url = format!("{}/{}/{}.yml", installer::BASE, installer::enc(&cat), installer::enc(&key));
            a.said = format!("Getting the community's installer for {}…", inst["name"].as_str().unwrap_or(""));
            return Task::perform(run::spawn(run::args(&["curl", "-fsSL", "-m", "30", &url])), Msg::Manifest);
        }
        Msg::Manifest(o) => {
            if !o.ok || o.out.trim().is_empty() {
                a.said = format!("The installer couldn't be got: {}", o.why());
                return Task::none();
            }
            let Some(b) = a.bottle().cloned() else { return Task::none() };
            let m = installer::parse(&o.out);
            let prog: installer::Progress = std::sync::Arc::new(std::sync::Mutex::new(installer::lines_of(&m)));
            a.installing = Some((m.name.clone(), prog.clone()));
            a.pop = Some(Pop::Installing);
            let ctx = installer::Ctx { bottle: b.name.clone(), dir: b.dir.clone(), yml: b.file.clone(), wine: a.wine_of(&b) };
            let snap_first = b.flag(&["Parameters", "versioning_automatic"]);
            let name = m.name.clone();
            let _ = std::thread::Builder::new().name("hearth-bottles-installer".into()).spawn(move || {
                if snap_first {
                    let _ = snapshot(&ctx.bottle, &ctx.dir, &format!("Before installing {name}"));
                }
                installer::run(ctx, m, prog);
            });
        }
        Msg::InstallTick => {
            if let Some((_, p)) = &a.installing {
                let done = p.lock().map(|g| g.iter().all(|l| !matches!(l.st, installer::St::Waiting | installer::St::Doing)) || g.iter().any(|l| matches!(l.st, installer::St::Failed(_)))).unwrap_or(true);
                if done {
                    a.install_done = true;
                    return a.reload();
                }
            }
        }
        Msg::Runner(r) => {
            a.pop = None;
            let b = a.pick.clone();
            let cmd = a.cli(&["edit", "-b", &b, "--runner", &r]);
            let q = yaml::quote(&r);
            return Task::perform(run::spawn(cmd), move |o| Msg::Done(format!("runner:{q}"), if o.ok { Ok(String::new()) } else { Err(o.why()) }));
        }
        Msg::Windows(w) => {
            a.pop = None;
            let b = a.pick.clone();
            let cmd = a.cli(&["edit", "-b", &b, "--win", w]);
            return Task::perform(run::spawn(cmd), move |o| Msg::Done(format!("win:{w}"), if o.ok { Ok(String::new()) } else { Err(o.why()) }));
        }
        Msg::Graphics(i) => {
            let (dx, vk) = match i {
                0 => ("true", "false"),
                1 => ("true", "true"),
                _ => ("false", "false"),
            };
            a.change(|t| yaml::yset(&yaml::yset(t, &["Parameters", "dxvk"], Some(dx)), &["Parameters", "vkd3d"], Some(vk)));
        }
        Msg::Param(key) => {
            // Controller and USB is on until it's turned off (Wine lets them in by itself)
            let on = a.bottle().map(|b| if key == "hearth_devices" { b.get(&["Parameters", key]).map(|v| v == "true").unwrap_or(true) } else { b.flag(&["Parameters", key]) }).unwrap_or(false);
            let Some(b) = a.bottle().cloned() else { return Task::none() };
            a.change(|t| yaml::yset(t, &["Parameters", key], Some(if on { "false" } else { "true" })));
            // what Wine itself needs for it (the registry), with the bottle's own Wine
            let reg = match key {
                "hearth_sharp_fonts" => Some(("HKCU\\Control Panel\\Desktop", "FontSmoothing", if on { "0" } else { "2" }, "REG_SZ")),
                "hearth_fit_scale" => Some(("HKCU\\Control Panel\\Desktop", "LogPixels", if on { "96" } else { "144" }, "REG_DWORD")),
                _ => None,
            };
            if key == "hearth_devices" {
                // Wine's own bus driver: controllers and USB devices reach the bottle or are kept apart (installer.rs)
                let now_on = !b.get(&["Parameters", key]).map(|v| v == "true").unwrap_or(true);
                let f = b.dir.join("hearth-devices.reg");
                if let Err(e) = std::fs::write(&f, installer::devices_reg(now_on)) {
                    a.said = format!("Couldn't change it: {e}");
                    return Task::none();
                }
                let cmd = vec!["env".to_string(), format!("WINEPREFIX={}", b.dir.display()), a.wine_of(&b), "reg".into(), "import".into(), f.to_string_lossy().to_string()];
                a.said = if now_on { "Controllers and USB devices reach this bottle's programs.".into() } else { "Controllers and USB devices are kept apart from this bottle.".into() };
                return Task::perform(run::spawn(cmd), |o| Msg::Done("reg".into(), if o.ok { Ok(String::new()) } else { Err(o.why()) }));
            }
            if let Some((rk, rv, rd, rt)) = reg {
                let cmd = vec!["env".to_string(), format!("WINEPREFIX={}", b.dir.display()), a.wine_of(&b), "reg".into(), "add".into(), rk.into(), "/v".into(), rv.into(), "/t".into(), rt.into(), "/d".into(), rd.into(), "/f".into()];
                return Task::perform(run::spawn(cmd), |o| Msg::Done("reg".into(), if o.ok { Ok(String::new()) } else { Err(o.why()) }));
            }
        }
        Msg::Home => {
            let sandboxed = a.bottle().map(|b| b.flag(&["Parameters", "sandbox"])).unwrap_or(false);
            a.change(|t| yaml::yset(t, &["Parameters", "sandbox"], Some(if sandboxed { "false" } else { "true" })));
        }
        Msg::Net => {
            let on = a.bottle().map(|b| b.get(&["Sandbox", "share_net"]).map(|v| v == "true").unwrap_or(true)).unwrap_or(true);
            a.change(|t| yaml::yset(t, &["Sandbox", "share_net"], Some(if on { "false" } else { "true" })));
        }
        Msg::Folder(f) => {
            // Wine's own shell folder in the bottle: a link to yours (shared), or a folder of its own
            let Some(b) = a.bottle().cloned() else { return Task::none() };
            let user = std::env::var("USER").unwrap_or_else(|_| "user".into());
            let inside = b.dir.join("drive_c/users").join(&user).join(f);
            let mine = k::home().join(f);
            let linked = std::fs::symlink_metadata(&inside).map(|m| m.file_type().is_symlink()).unwrap_or(false);
            let r = if linked {
                std::fs::remove_file(&inside).and_then(|_| std::fs::create_dir_all(&inside))
            } else {
                let empty = std::fs::read_dir(&inside).map(|mut d| d.next().is_none()).unwrap_or(true);
                if !empty {
                    a.said = format!("The bottle's own {f} has files in it, so it stays as it is.");
                    return Task::none();
                }
                let _ = std::fs::remove_dir(&inside);
                let _ = std::fs::create_dir_all(inside.parent().unwrap_or(&inside));
                #[cfg(unix)]
                let r = std::os::unix::fs::symlink(&mine, &inside);
                r
            };
            if let Err(e) = r {
                a.said = format!("Couldn't change {f}: {e}");
            }
        }
        Msg::Shows(which) => {
            let Some(b) = a.bottle().cloned() else { return Task::none() };
            let key = format!("/shows/{}/{which}", b.name);
            let on = !a.note(&key).and_then(Value::as_bool).unwrap_or(true);
            let mut shows = a.notes.get("shows").cloned().filter(Value::is_object).unwrap_or(json!({}));
            if !shows[&b.name].is_object() {
                shows[&b.name] = json!({});
            }
            shows[&b.name][which] = json!(on);
            a.save(json!({"shows": shows}));
            // the Apps menu: every program's entry made or taken out (Files' Open with: their file types)
            let cli = a.cli(&[]);
            let files_on = a.note(&format!("/shows/{}/files", b.name)).and_then(Value::as_bool).unwrap_or(true);
            let menu_on = a.note(&format!("/shows/{}/menu", b.name)).and_then(Value::as_bool).unwrap_or(true);
            for pg in &b.progs {
                let f = entry_path(&b.name, &pg.name);
                if !menu_on {
                    let _ = std::fs::remove_file(&f);
                } else if which == "menu" || f.exists() {
                    let mimes = a.note(&format!("/opens/{}", pg.name.replace('/', "~1"))).and_then(Value::as_str).unwrap_or("").to_string();
                    let _ = write_entry(&cli, &b.name, pg, if files_on { &mimes } else { "" });
                }
            }
        }
        Msg::Backup => {
            let Some(b) = a.bottle().cloned() else { return Task::none() };
            a.busy = true;
            a.said = format!("Backing up {}…", b.name);
            return thread(
                move || {
                    let d = k::home().join("Hearth/Backups");
                    std::fs::create_dir_all(&d).map_err(|e| e.to_string())?;
                    let f = d.join(format!("{}-{}.tar.zst", b.name.replace(['/', ' '], "_"), hearth_ui::time::now()));
                    let o = run::blocking(&vec!["tar".to_string(), "--zstd".into(), "-cf".into(), f.to_string_lossy().to_string(), "-C".into(), b.dir.parent().unwrap_or(&b.dir).to_string_lossy().to_string(), b.dir.file_name().map(|x| x.to_string_lossy().to_string()).unwrap_or_default()]);
                    if o.ok { Ok(k::tilde(&f)) } else { Err(o.why()) }
                },
                |r| Msg::Done("backup".into(), r),
            );
        }
        Msg::Copy => {
            let Some(b) = a.bottle().cloned() else { return Task::none() };
            let mut n = 2;
            let name = loop {
                let c = format!("{} {n}", b.name);
                if !a.bottles.iter().any(|x| x.name == c) {
                    break c;
                }
                n += 1;
            };
            a.busy = true;
            let to = b.dir.with_file_name(name.replace('/', "_"));
            return thread(
                move || {
                    let o = run::blocking(&vec!["cp".to_string(), "-a".into(), b.dir.to_string_lossy().to_string(), to.to_string_lossy().to_string()]);
                    if !o.ok {
                        return Err(o.why());
                    }
                    let q = yaml::quote(&name);
                    let p = to.to_string_lossy().to_string();
                    k::settings_file::update_text(&to.join("bottle.yml"), |t| yaml::yset(&yaml::yset(t, &["Name"], Some(q.as_str())), &["Path"], Some(yaml::quote(&p).as_str()))).map_err(|e| e.to_string())?;
                    Ok(name)
                },
                |r| Msg::Done("copy".into(), r),
            );
        }
        Msg::Delete => {
            a.pop = None;
            let Some(b) = a.bottle().cloned() else { return Task::none() };
            let home = k::home();
            if !b.dir.starts_with(&home) || !b.file.is_file() {
                a.said = "That bottle isn't in a folder of yours.".into();
                return Task::none();
            }
            for pg in &b.progs {
                let _ = std::fs::remove_file(entry_path(&b.name, &pg.name));
            }
            match std::fs::remove_dir_all(&b.dir) {
                Ok(()) => a.said = format!("{} was deleted ({} back). Its snapshots are kept.", b.name, k::size_words(b.size)),
                Err(e) => a.said = format!("Couldn't delete it: {e}"),
            }
            return a.reload();
        }
        Msg::Done(what, r) => {
            a.busy = false;
            match (what.as_str(), r) {
                (w, Ok(_)) if w.starts_with("runner:") || w.starts_with("win:") => {
                    // Bottles changed it; read again
                }
                ("new", Ok(_)) => a.said = "The bottle is ready.".into(),
                ("backup", Ok(f)) => a.said = format!("Backed up to {f}."),
                ("copy", Ok(n)) => {
                    a.said = format!("Copied as {n}.");
                    a.pick = n;
                }
                (w, Err(e)) => {
                    if let Some(q) = w.strip_prefix("runner:") {
                        // Bottles' command couldn't: its file, one line
                        let q = q.to_string();
                        a.change(move |t| yaml::yset(t, &["Runner"], Some(q.as_str())));
                    } else {
                        a.said = format!("That didn't work: {e}");
                    }
                }
                _ => {}
            }
            return a.reload();
        }
        Msg::Tool(tool) => {
            a.pop = None;
            let b = a.pick.clone();
            let cmd = a.cli(&["tools", "-b", &b, tool]);
            return Task::perform(run::spawn(cmd), Msg::Ran);
        }
        Msg::Dropped(path) => {
            let s = path.to_string_lossy().to_string();
            let low = s.to_lowercase();
            if low.ends_with(".exe") || low.ends_with(".msi") || low.ends_with(".bat") {
                a.pop = Some(Pop::Dropped(s));
            }
        }
        Msg::GetBottles => {
            a.busy = true;
            return Task::perform(run::spawn(need::install_argv(&BOTTLES)), Msg::GotBottles);
        }
        Msg::GotBottles(o) => {
            a.busy = false;
            a.said = if o.ok { "Bottles is here.".into() } else { format!("Couldn't get Bottles: {}", o.why()) };
            return Task::perform(need::probe(&BOTTLES), Msg::Have);
        }
        Msg::Drag => return win::drag(),
        Msg::Max => return win::toggle_maximize(),
        Msg::Min => return win::minimize(),
        Msg::WClose => return win::close(),
        Msg::Resize(d) => return win::resize(d),
    }
    Task::none()
}

unsafe fn libc_kill(pid: u32) {
    extern "C" {
        fn kill(pid: i32, sig: i32) -> i32;
    }
    let _ = kill(pid as i32, 15);
}

/// A snapshot: the whole bottle in one file Hearth keeps (its time and name in the file's name).
fn snapshot(name: &str, dir: &Path, what: &str) -> Result<String, String> {
    let d = snaps_dir(name);
    std::fs::create_dir_all(&d).map_err(|e| e.to_string())?;
    let clean: String = what.chars().map(|c| if c.is_alphanumeric() || c == '.' || c == '+' { c } else { '_' }).collect();
    let f = d.join(format!("{}-{clean}.tar.zst", hearth_ui::time::now()));
    let o = run::blocking(&vec!["tar".to_string(), "--zstd".into(), "-cf".into(), f.to_string_lossy().to_string(), "-C".into(), dir.to_string_lossy().to_string(), ".".into()]);
    if o.ok { Ok(what.to_string()) } else { Err(o.why()) }
}

/// Back to a snapshot: what's there now is kept as a snapshot first, then the bottle is put back as it was.
fn go_back(name: &str, dir: &Path, file: &str) -> Result<String, String> {
    let src = snaps_dir(name).join(file);
    if !src.is_file() || file.contains('/') {
        return Err("that snapshot isn't there".into());
    }
    snapshot(name, dir, "Before going back")?;
    let tmp = dir.with_extension("hearth-restoring");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    let o = run::blocking(&vec!["tar".to_string(), "--zstd".into(), "-xf".into(), src.to_string_lossy().to_string(), "-C".into(), tmp.to_string_lossy().to_string()]);
    if !o.ok || !tmp.join("bottle.yml").is_file() {
        let _ = std::fs::remove_dir_all(&tmp);
        return Err(format!("it couldn't be unpacked: {}", o.why()));
    }
    let old = dir.with_extension("hearth-old");
    let _ = std::fs::remove_dir_all(&old);
    std::fs::rename(dir, &old).map_err(|e| e.to_string())?;
    if let Err(e) = std::fs::rename(&tmp, dir) {
        let _ = std::fs::rename(&old, dir);
        return Err(e.to_string());
    }
    let _ = std::fs::remove_dir_all(&old);
    Ok(file.split_once('-').map(|x| x.1).unwrap_or(file).trim_end_matches(".tar.zst").replace('_', " "))
}

// ------------------------------------------------------------------ the window

/// The board's bottle: a rounded body in the bottle's colours with its neck on top.
fn bottle_mark<'a>(i: usize, w: f32, h: f32) -> E<'a, Msg> {
    let (a, b) = MARKS[i % MARKS.len()];
    let (a, b) = (k::rgb(a), k::rgb(b));
    let g = iced::gradient::Linear::new(iced::Degrees(180.0)).add_stop(0.0, a).add_stop(1.0, b);
    let body = container(Space::new()).width(Length::Fixed(w)).height(Length::Fixed(h * 0.84)).style(move |_: &iced::Theme| container::Style { background: Some(iced::Background::Gradient(iced::Gradient::Linear(g))), border: iced::Border { radius: (w * 0.26).into(), ..iced::Border::default() }, ..container::Style::default() });
    let neck = container(Space::new()).width(Length::Fixed(w * 0.34)).height(Length::Fixed(h * 0.14)).style(ui::boxed(a, 2.0, None));
    column![neck, body].align_x(iced::Center).spacing(0.0).into()
}

fn side(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let q = a.find.to_lowercase();
    let mut list = Column::new().spacing(4.0);
    for (i, b) in a.bottles.iter().enumerate() {
        if !q.is_empty() && !b.name.to_lowercase().contains(&q) && !b.progs.iter().any(|x| x.name.to_lowercase().contains(&q)) {
            continue;
        }
        let n = b.progs.len();
        let line = if n == 0 { format!("{} · {}", b.kind(), b.runner()) } else { format!("{} · {} · {n} program{}", b.kind(), b.runner(), if n == 1 { "" } else { "s" }) };
        let line = match a.note(&format!("/kinds/{}", b.name)).and_then(Value::as_str) {
            Some(kd) if n > 0 => format!("{kd} · {} · {n} programs", b.runner()),
            Some(kd) => format!("{kd} · {}", b.runner()),
            None => line,
        };
        let inner = row![bottle_mark(i, 22.0, 30.0), column![t(b.name.clone(), 13.0, p.text).wrapping(Wrapping::None), t(line, 10.0, p.muted).wrapping(Wrapping::None)].spacing(1.0)].spacing(10.0).align_y(iced::Center);
        list = list.push(k::side_row(&p, container(inner).padding(k::pad(3.0, 0.0, 3.0, 0.0)), b.name == a.pick, 36.0, Msg::Pick(b.name.clone())));
    }
    if a.bottles.is_empty() && a.have.ok() {
        list = list.push(k::mu(&p, "No bottles yet: + New bottle makes one.", 12.0));
    }
    let drop = container(column![tb("Run a .exe", 11.0, p.text), k::para(p.muted, "Drop a Windows program here. Hearth picks a bottle for it or makes a new one.", 11.0)].spacing(3.0))
        .padding(k::pad(9.0, 10.0, 9.0, 10.0))
        .width(Length::Fill)
        .style(ui::boxed(Color::TRANSPARENT, p.r(10.0), Some((p.line, 1.0))));
    let body = hearth_ui::arrange::parts(
        &a.describe,
        &p,
        "side",
        vec![("find", k::search(&p, "Find a bottle or app", &a.find, Msg::Find)), ("bottles", scrollable(list).height(Length::Shrink).into()), ("drop", k::click(drop, Some(Msg::RunProgram))), ("edit", k::edit_layout(&p, Msg::Describe(hearth_ui::arrange::edit_layout())))],
        |ps| column![ps.take("find"), k::gap(1.0, 10.0), ps.take("bottles"), k::gap(1.0, 10.0), ps.take("drop"), Space::new().height(Length::Fill), ps.take("edit")].into(),
        Msg::Describe,
    );
    k::side(&p, body, 210.0)
}

fn prog_mark<'a>(a: &App, name: &str) -> E<'a, Msg> {
    let cols = [("#7fd6ff", "#2c7fd8"), ("#b9a2ff", "#6a4fe0"), ("#b8f07f", "#3ba55c"), ("#ffb36b", "#ff6a1f"), ("#c9c1ba", "#6d645d"), ("#ffe28a", "#f0a020"), ("#ff9be8", "#b04fc8")];
    // the mark you gave it (Hearth Bottles' notes: its letters and colour), else made from its name
    if let Some(m) = a.note(&format!("/marks/{}", name.replace('/', "~1"))).and_then(Value::as_array) {
        let l = m.first().and_then(Value::as_str).unwrap_or("?");
        let (x, y) = cols[m.get(1).and_then(Value::as_u64).unwrap_or(0) as usize % cols.len()];
        return k::mark(l, k::rgb(x), k::rgb(y), 26.0);
    }
    let letters: String = {
        let w: Vec<&str> = name.split(|c: char| c.is_whitespace() || c == '-').filter(|x| !x.is_empty()).collect();
        let first = |s: &str| s.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default();
        match w.as_slice() {
            [one] if one.ends_with("++") => format!("{}+", first(one)),
            [one] => {
                let caps: String = one.chars().filter(|c| c.is_uppercase()).take(2).collect();
                if caps.chars().count() >= 2 { caps } else { first(one) }
            }
            [x, y, ..] if y.starts_with(|c: char| c.is_ascii_digit()) => first(x),
            [x, y, ..] => format!("{}{}", first(x), first(y)),
            [] => "?".into(),
        }
    };
    let h = name.bytes().fold(0u32, |h, b| h.wrapping_mul(31).wrapping_add(b as u32)) as usize;
    let (x, y) = cols[h % cols.len()];
    k::mark(&letters, k::rgb(x), k::rgb(y), 26.0)
}

fn head(a: &App, b: &Bottle, i: usize) -> E<'static, Msg> {
    let p = a.p;
    let made = b.get(&["Creation_Date"]).map(|d| k::day_words(&d)).filter(|d| !d.is_empty());
    let mut meta = vec![format!("Kind: {}", a.note(&format!("/kinds/{}", b.name)).and_then(Value::as_str).map(str::to_string).unwrap_or_else(|| b.kind()))];
    if let Some(m) = made {
        meta.push(format!("made {m}"));
    }
    if b.size > 0 {
        meta.push(k::size_words(b.size));
    }
    meta.push(k::tilde(&b.dir));
    let title = row![
        bottle_mark(i, 40.0, 52.0),
        column![k::h1::<Msg>(&p, b.name.clone()), k::mu1::<Msg>(&p, meta.join(" · "), 12.0)].spacing(3.0),
        k::fill(),
        k::chip(&p, "▶ Run a program…", Ch::Acc, Sz::Md, a.have.ok().then_some(Msg::RunProgram)),
        k::chip(&p, "Open its C: drive", Ch::Plain, Sz::Md, Some(Msg::OpenC)),
        k::chip(&p, "⋯", Ch::Plain, Sz::Md, Some(Msg::Pop(Pop::More))),
    ]
    .spacing(10.0)
    .align_y(iced::Center);
    let arch = if b.get(&["Arch"]).as_deref() == Some("win32") { "32-bit" } else { "64-bit" };
    let win = WINDOWS.iter().find(|w| Some(w.0.to_string()) == b.get(&["Windows"])).map(|w| w.1).unwrap_or("Windows");
    let mut tags = Row::new().spacing(6.0).push(k::tag(&p, "Ready", Tag::Ok)).push(k::tag(&p, format!("Runner {}", b.runner()), Tag::Dk)).push(k::tag(&p, format!("{win} · {arch}"), Tag::Dk));
    if b.flag(&["Parameters", "dxvk"]) {
        let v = b.get(&["DXVK"]).map(|d| d.trim_start_matches("dxvk-").to_string()).unwrap_or_default();
        tags = tags.push(k::tag(&p, if v.is_empty() { "DXVK".to_string() } else { format!("DXVK {v}") }, Tag::Dk));
    }
    if let Some(s) = snapshots(&b.name).first() {
        tags = tags.push(k::tag(&p, format!("Snapshot {}", ago(s.2)), Tag::Bl));
    }
    column![title, tags].spacing(10.0).into()
}

fn programs(a: &App, b: &Bottle) -> E<'static, Msg> {
    let p = a.p;
    let mut grid = Column::new().spacing(10.0);
    let mut r = Row::new().spacing(10.0);
    let n = b.progs.len();
    for (i, pg) in b.progs.iter().enumerate() {
        let about = a.note(&format!("/about/{}", pg.name.replace('/', "~1"))).and_then(Value::as_str).map(str::to_string).unwrap_or_else(|| pg.exe.clone());
        let in_menu = entry_path(&b.name, &pg.name).exists();
        let card = container(
            column![
                row![prog_mark(a, &pg.name), column![tb(pg.name.clone(), 12.0, p.text).wrapping(Wrapping::None), k::mu1::<Msg>(&p, about, 10.0)].spacing(1.0)].spacing(9.0).align_y(iced::Center),
                row![
                    k::chip(&p, "▶ Run", Ch::Acc, Sz::Sm, a.have.ok().then(|| Msg::Run(pg.name.clone()))),
                    k::chip(&p, if in_menu { "In Apps menu ✓" } else { "Add to Apps menu" }, Ch::Plain, Sz::Sm, Some(Msg::MenuEntry(pg.name.clone()))),
                    k::fill(),
                    k::chip(&p, "⋯", Ch::Plain, Sz::Sm, Some(Msg::Pop(Pop::ProgMore(pg.id.clone())))),
                ]
                .spacing(6.0)
                .align_y(iced::Center),
            ]
            .spacing(10.0),
        )
        .padding(11.0)
        .width(Length::FillPortion(1))
        .style(ui::boxed(p.panel, p.r(12.0), Some((p.line, 1.0))));
        r = r.push(card);
        if i % 3 == 2 || i + 1 == n {
            for _ in 0..(2 - i % 3) {
                r = r.push(Space::new().width(Length::FillPortion(1)));
            }
            grid = grid.push(r);
            r = Row::new().spacing(10.0);
        }
    }
    if n == 0 {
        grid = grid.push(k::mu(&p, "Nothing in this bottle yet: ▶ Run a program… or Get a program below.", 12.0));
    }
    column![k::cap::<Msg>(&p, format!("Programs in this bottle · {n}")), k::gap(1.0, 8.0), grid].into()
}

fn extras(a: &App, b: &Bottle) -> E<'static, Msg> {
    let p = a.p;
    let deps = yaml::ylist(&b.yml, &["Installed_Dependencies"]);
    let mut c = Column::new().push(row![k::h2::<Msg>(&p, "Extras installed"), k::fill(), k::chip(&p, "+ Add", Ch::Plain, Sz::Sm, Some(Msg::Pop(Pop::Extras)))].align_y(iced::Center)).push(k::gap(1.0, 4.0));
    let last = deps.len().saturating_sub(1);
    for (i, d) in deps.iter().enumerate() {
        let name = EXTRAS.iter().find(|x| x.0 == d || d.starts_with(x.0)).map(|x| x.1.to_string()).unwrap_or_else(|| d.clone());
        let why = a.note(&format!("/extras/{}/{d}", b.name)).and_then(Value::as_str).unwrap_or("").to_string();
        c = c.push(container(row![k::dot::<Msg>(k::GREEN, 7.0), t(name, 12.0, p.text).wrapping(Wrapping::None), k::fill(), k::mu1::<Msg>(&p, why, 10.0)].spacing(9.0).align_y(iced::Center)).padding(k::pad(6.0, 0.0, 6.0, 0.0)));
        if i < last {
            c = c.push(k::line(&p));
        }
    }
    if deps.is_empty() {
        c = c.push(k::mu(&p, "None yet.", 12.0));
    }
    k::card_fit(&p, c)
}

fn snaps(a: &App, b: &Bottle) -> E<'static, Msg> {
    let p = a.p;
    let s = snapshots(&b.name);
    let mut c = Column::new().push(row![k::h2::<Msg>(&p, "Snapshots"), k::fill(), k::chip(&p, "Take one now", Ch::Plain, Sz::Sm, (!a.busy).then_some(Msg::Pop(Pop::Take)))].align_y(iced::Center)).push(k::gap(1.0, 4.0));
    let n = s.len().min(4);
    for (i, (f, name, ts)) in s.into_iter().take(4).enumerate() {
        c = c.push(container(row![t(name, 12.0, p.text).wrapping(Wrapping::None), k::fill(), k::mu1::<Msg>(&p, when_words(ts), 10.0), k::chip(&p, "Go back", Ch::Plain, Sz::Sm, Some(Msg::Pop(Pop::GoBack(f))))].spacing(8.0).align_y(iced::Center)).padding(k::pad(5.0, 0.0, 5.0, 0.0)));
        if i + 1 < n {
            c = c.push(k::line(&p));
        }
    }
    if n == 0 {
        c = c.push(k::mu(&p, "None yet: Take one now keeps the bottle as it is, to go back to.", 12.0));
    }
    k::card_fit(&p, c)
}

fn running(a: &App) -> E<'static, Msg> {
    let p = a.p;
    let mut c = Column::new().push(row![k::h2::<Msg>(&p, "Running now"), k::fill(), k::chip(&p, "Stop all", Ch::Plain, Sz::Sm, (!a.procs.is_empty()).then_some(Msg::StopAll))].align_y(iced::Center)).push(k::gap(1.0, 4.0));
    let n = a.procs.len();
    for (i, pr) in a.procs.iter().enumerate() {
        let first = pr.words.split(" · ").next().unwrap_or("").to_string();
        // the program's own name when you gave one (WINWORD is Word), and its mark
        let named = a.note(&format!("/names/{first}")).and_then(Value::as_str).map(str::to_string);
        let words = match &named {
            Some(n) => pr.words.replacen(&first, n, 1),
            None => pr.words.clone(),
        };
        let markname = a.note(&format!("/procMarks/{first}")).and_then(Value::as_str).map(str::to_string).unwrap_or_else(|| named.clone().unwrap_or(first.clone()));
        c = c.push(
            container(row![prog_mark(a, &markname), t(words, 12.0, p.text).wrapping(Wrapping::None), k::fill(), k::mu1::<Msg>(&p, format!("{} · {}% CPU", k::size_words(pr.mem), pr.cpu.round() as u32), 10.0), k::chip(&p, "Stop", Ch::Plain, Sz::Sm, Some(Msg::Stop(pr.pid)))].spacing(8.0).align_y(iced::Center))
                .padding(k::pad(5.0, 0.0, 5.0, 0.0)),
        );
        if i + 1 < n {
            c = c.push(k::line(&p));
        }
    }
    if n == 0 {
        c = c.push(k::mu(&p, "Nothing's running in this bottle.", 12.0));
    }
    k::card_fit(&p, c)
}

fn get_program(a: &App) -> E<'static, Msg> {
    let p = a.p;
    let mut chips: Vec<E<'static, Msg>> = a.installers.iter().take(5).enumerate().map(|(i, x)| k::chip(&p, x["name"].as_str().unwrap_or("").to_string(), Ch::Plain, Sz::Sm, Some(Msg::Pop(Pop::Get(i))))).collect();
    chips.push(k::chip(&p, "Your own .exe…", Ch::Plain, Sz::Sm, Some(Msg::RunProgram)));
    let mut rows = Column::new().spacing(6.0);
    let mut cur = Row::new().spacing(6.0);
    let mut w = 0usize;
    let names: Vec<String> = a.installers.iter().take(5).map(|x| x["name"].as_str().unwrap_or("").to_string()).chain(["Your own .exe…".to_string()]).collect();
    for (i, ch) in chips.into_iter().enumerate() {
        let cw = names[i].chars().count() + 4;
        if w > 0 && w + cw > 52 {
            rows = rows.push(cur);
            cur = Row::new().spacing(6.0);
            w = 0;
        }
        cur = cur.push(ch);
        w += cw;
    }
    rows = rows.push(cur);
    k::card_fit(&p, column![row![k::h2::<Msg>(&p, "Get a program"), k::fill(), k::mu1::<Msg>(&p, "installers checked by the community", 10.0)].align_y(iced::Center), k::gap(1.0, 8.0), rows])
}

fn settings(a: &App, b: &Bottle) -> E<'static, Msg> {
    let p = a.p;
    let win = WINDOWS.iter().find(|w| Some(w.0.to_string()) == b.get(&["Windows"])).map(|w| w.1).unwrap_or("Windows 10");
    let gi = if !b.flag(&["Parameters", "dxvk"]) { 2 } else if b.flag(&["Parameters", "vkd3d"]) { 1 } else { 0 };
    let shared: Vec<&str> = FOLDERS.iter().copied().filter(|f| {
        let user = std::env::var("USER").unwrap_or_else(|_| "user".into());
        std::fs::symlink_metadata(b.dir.join("drive_c/users").join(user).join(f)).map(|m| m.file_type().is_symlink()).unwrap_or(false)
    }).collect();
    let shows = |w: &str| a.note(&format!("/shows/{}/{w}", b.name)).and_then(Value::as_bool).unwrap_or(true);
    let total: u64 = a.bottles.iter().map(|x| x.size).sum();
    let c = column![
        k::h2::<Msg>(&p, "This bottle's settings"),
        k::gap(1.0, 8.0),
        k::kv(&p, "Runner", k::field(&p, format!("{} ▾", b.runner()), 130.0, Some(Msg::Pop(Pop::Runner)))),
        k::kv(&p, "Acts like", k::field(&p, format!("{win} ▾"), 130.0, Some(Msg::Pop(Pop::Windows)))),
        k::kv(&p, "Graphics", k::seg(&p, vec![("DXVK".into(), gi == 0, Msg::Graphics(0)), ("VKD3D".into(), gi == 1, Msg::Graphics(1)), ("OpenGL".into(), gi == 2, Msg::Graphics(2))])),
        k::kv(&p, "Sharper fonts", k::switch(&p, b.flag(&["Parameters", "hearth_sharp_fonts"]), Some(Msg::Param("hearth_sharp_fonts")))),
        k::kv(&p, "Fit to my screen scale", k::switch(&p, b.flag(&["Parameters", "hearth_fit_scale"]), Some(Msg::Param("hearth_fit_scale")))),
        k::kv(&p, "Snapshot before each install", k::switch(&p, b.flag(&["Parameters", "versioning_automatic"]), Some(Msg::Param("versioning_automatic")))),
        container(k::cap::<Msg>(&p, "Kept apart")).padding(k::pad(10.0, 0.0, 4.0, 0.0)),
        k::kv(&p, "Can see my home folder", k::switch(&p, !b.flag(&["Parameters", "sandbox"]) && b.get(&["Parameters", "sandbox"]).is_some(), Some(Msg::Home))),
        k::kv(&p, "Shared folders", k::chip(&p, if shared.is_empty() { "None".to_string() } else { shared.join(", ") }, Ch::Plain, Sz::Sm, Some(Msg::Pop(Pop::Shared)))),
        k::kv(&p, "Internet", k::switch(&p, b.get(&["Sandbox", "share_net"]).map(|v| v == "true").unwrap_or(true), Some(Msg::Net))),
        k::kv(&p, "Controller and USB", k::switch(&p, b.get(&["Parameters", "hearth_devices"]).map(|v| v == "true").unwrap_or(true), Some(Msg::Param("hearth_devices")))),
        container(k::cap::<Msg>(&p, "Shows up in")).padding(k::pad(10.0, 0.0, 4.0, 0.0)),
        k::kv(&p, "Apps menu", k::switch(&p, shows("menu"), Some(Msg::Shows("menu")))),
        k::kv(&p, "Files' Open with", k::switch(&p, shows("files"), Some(Msg::Shows("files")))),
        container(k::cap::<Msg>(&p, "Space")).padding(k::pad(10.0, 0.0, 6.0, 0.0)),
        k::bar(&p, if total > 0 { b.size as f32 / total as f32 } else { 0.0 }, 6.0, Length::Fill),
        row![k::mu1::<Msg>(&p, format!("{} in this bottle", k::size_words(b.size)), 10.0), k::fill(), k::mu1::<Msg>(&p, format!("{} in all {}", k::size_words(total), a.bottles.len()), 10.0)],
        k::gap(1.0, 12.0),
        row![k::chip(&p, "Back up the bottle", Ch::Plain, Sz::Sm, (!a.busy).then_some(Msg::Backup)), k::chip(&p, "Copy it", Ch::Plain, Sz::Sm, (!a.busy).then_some(Msg::Copy)), k::chip(&p, "Delete…", Ch::Bad, Sz::Sm, Some(Msg::Pop(Pop::Delete)))].spacing(6.0),
    ]
    .spacing(2.0);
    container(scrollable(c).height(Length::Fill)).width(Length::Fixed(262.0)).height(Length::Fill).padding(k::pad(12.0, 14.0, 12.0, 14.0)).style(ui::boxed(Pal::mix(p.bg, Color::BLACK, 0.15), 0.0, None)).into()
}

fn page(a: &App) -> E<'_, Msg> {
    let p = a.p;
    if !a.have.ok() && a.have != Have::Looking {
        return container(column![k::h1::<Msg>(&p, "Hearth Bottles needs Bottles"), k::para(p.muted, need::missing_words(&BOTTLES), 13.0), k::chip(&p, if a.busy { "Getting Bottles…" } else { "Get Bottles" }, Ch::Acc, Sz::Md, (!a.busy).then_some(Msg::GetBottles)), k::said(&p, &a.said)].spacing(12.0).max_width(560.0)).padding(40.0).into();
    }
    let Some(b) = a.bottle().cloned() else {
        return container(column![k::mu(&p, "No bottles yet: + New bottle makes one.", 13.0), k::said(&p, &a.said)].spacing(10.0)).padding(40.0).into();
    };
    let i = a.bottles.iter().position(|x| x.name == b.name).unwrap_or(0);
    let has_said = !a.said.is_empty();
    hearth_ui::arrange::parts(
        &a.describe,
        &p,
        "bottle",
        vec![
            ("head", head(a, &b, i)),
            ("programs", programs(a, &b)),
            ("extras", extras(a, &b)),
            ("snapshots", snaps(a, &b)),
            ("running", running(a)),
            ("get", get_program(a)),
            ("settings", settings(a, &b)),
            ("said", k::said(&p, &a.said)),
        ],
        move |ps| {
            let mut mid = column![ps.take("head"), k::gap(1.0, 8.0), ps.take("programs"), row![ps.take("extras"), ps.take("snapshots")].spacing(12.0).height(Length::Shrink), row![ps.take("running"), ps.take("get")].spacing(12.0).height(Length::Shrink)].spacing(12.0);
            if has_said {
                mid = mid.push(ps.take("said"));
            }
            row![scrollable(mid.padding(k::pad(16.0, 20.0, 14.0, 20.0))).height(Length::Fill).width(Length::Fill), ui::vline(p.line), ps.take("settings")].into()
        },
        Msg::Describe,
    )
}

fn pop_box(a: &App) -> Option<E<'_, Msg>> {
    let p = a.p;
    let b = a.bottle();
    Some(match a.pop.clone()? {
        Pop::Runner => {
            let cur = b.and_then(|b| b.get(&["Runner"])).unwrap_or_default();
            let mut rs: Vec<String> = runner_dirs().iter().filter_map(|d| std::fs::read_dir(d).ok()).flat_map(|r| r.flatten().map(|e| e.file_name().to_string_lossy().to_string())).collect();
            rs.sort();
            rs.dedup();
            k::chooser(&p, "Runner", rs.into_iter().map(|r| (pretty_runner(&r), r == cur, Msg::Runner(r))).collect(), Msg::Close)
        }
        Pop::Windows => {
            let cur = b.and_then(|b| b.get(&["Windows"])).unwrap_or_default();
            k::chooser(&p, "Acts like", WINDOWS.iter().map(|(id, n)| (n.to_string(), *id == cur, Msg::Windows(id))).collect(), Msg::Close)
        }
        Pop::Shared => {
            let b = b?;
            let user = std::env::var("USER").unwrap_or_else(|_| "user".into());
            k::chooser(&p, "Shared with the bottle", FOLDERS.iter().map(|f| (format!("Your {f}"), std::fs::symlink_metadata(b.dir.join("drive_c/users").join(&user).join(f)).map(|m| m.file_type().is_symlink()).unwrap_or(false), Msg::Folder(f))).collect(), Msg::Close)
        }
        Pop::Extras => {
            let have = b.map(|b| yaml::ylist(&b.yml, &["Installed_Dependencies"])).unwrap_or_default();
            k::chooser(&p, "Add an extra", EXTRAS.iter().map(|(v, n)| (n.to_string(), have.iter().any(|h| h == v), Msg::Extra(v))).collect(), Msg::Close)
        }
        Pop::New => ui::dialog_box(
            &p,
            440.0,
            column![
                k::h2::<Msg>(&p, "A new bottle"),
                k::input(&p, "Its name", &a.text, Msg::Text).on_submit(Msg::NewBottle),
                k::seg(&p, vec![("Apps".into(), a.new_kind == "application", Msg::Kind("application")), ("Games".into(), a.new_kind == "gaming", Msg::Kind("gaming")), ("Empty".into(), a.new_kind == "custom", Msg::Kind("custom"))]),
                row![k::fill(), k::chip(&p, "Cancel", Ch::Plain, Sz::Md, Some(Msg::Close)), k::chip(&p, "Make it", Ch::Acc, Sz::Md, Some(Msg::NewBottle))].spacing(8.0),
            ]
            .spacing(10.0),
        ),
        Pop::More => k::chooser(&p, b.map(|b| b.name.clone()).unwrap_or_default(), vec![("Wine settings (winecfg)".into(), false, Msg::Tool("winecfg")), ("Task manager".into(), false, Msg::Tool("taskmgr")), ("Registry editor".into(), false, Msg::Tool("regedit")), ("Look at the bottles again".into(), false, Msg::Reload)], Msg::Close),
        Pop::ProgMore(id) => {
            let name = b.and_then(|b| b.progs.iter().find(|x| x.id == id)).map(|x| x.name.clone()).unwrap_or_default();
            k::chooser(&p, name, vec![("Take it off this list (its files stay)".into(), false, Msg::RemoveProg(id))], Msg::Close)
        }
        Pop::Delete => {
            let b = b?;
            ui::dialog_box(
                &p,
                440.0,
                column![
                    k::h2::<Msg>(&p, format!("Delete {}?", b.name)),
                    k::para(p.muted, format!("Its programs and everything in its C: drive go ({}). Its snapshots are kept.", k::size_words(b.size)), 12.0),
                    row![k::fill(), k::chip(&p, "Cancel", Ch::Plain, Sz::Md, Some(Msg::Close)), k::chip(&p, "Delete", Ch::Acc, Sz::Md, Some(Msg::Delete))].spacing(8.0),
                ]
                .spacing(10.0),
            )
        }
        Pop::Take => ui::dialog_box(&p, 420.0, column![k::h2::<Msg>(&p, "Take a snapshot"), k::input(&p, "What it's for, like Before Office update", &a.text, Msg::Text).on_submit(Msg::Snapshot), row![k::fill(), k::chip(&p, "Cancel", Ch::Plain, Sz::Md, Some(Msg::Close)), k::chip(&p, "Take it", Ch::Acc, Sz::Md, Some(Msg::Snapshot))].spacing(8.0)].spacing(10.0)),
        Pop::GoBack(f) => ui::dialog_box(
            &p,
            440.0,
            column![k::h2::<Msg>(&p, "Go back to this snapshot?"), k::para(p.muted, "The bottle is put back as it was then. What it has now is kept as a snapshot first, so you can come back.", 12.0), row![k::fill(), k::chip(&p, "Cancel", Ch::Plain, Sz::Md, Some(Msg::Close)), k::chip(&p, "Go back", Ch::Acc, Sz::Md, Some(Msg::GoBack(f)))].spacing(8.0)].spacing(10.0),
        ),
        Pop::Dropped(path) => {
            let file = Path::new(&path).file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
            let mut opts: Vec<(String, bool, Msg)> = a.bottles.iter().map(|x| (x.name.clone(), x.name == a.pick, Msg::RunExe(x.name.clone(), path.clone()))).collect();
            opts.push(("A new bottle for it…".into(), false, Msg::Pop(Pop::New)));
            k::chooser(&p, format!("Run {file} in…"), opts, Msg::Close)
        }
        Pop::Installing => {
            let (name, prog) = a.installing.as_ref()?;
            let lines = prog.lock().map(|g| g.clone()).unwrap_or_default();
            let mut c = Column::new().spacing(6.0).push(k::h2::<Msg>(&p, format!("Installing {name} into {}", a.pick)));
            for (i, l) in lines.iter().enumerate() {
                let (mark, col, why) = match &l.st {
                    installer::St::Waiting => ("○", p.muted, String::new()),
                    installer::St::Doing => ("◐", p.accent, String::new()),
                    installer::St::Done => ("✓", k::GREEN, String::new()),
                    installer::St::Skipped(w) => ("–", p.muted, w.clone()),
                    installer::St::Failed(w) => ("✕", k::RED, w.clone()),
                };
                let mut r = row![container(t(mark, 13.0, col)).width(Length::Fixed(18.0)), t(format!("{}. {}", i + 1, l.words), 12.0, p.text)].spacing(6.0).align_y(iced::Center);
                if !why.is_empty() {
                    r = r.push(k::mu1::<Msg>(&p, why, 11.0));
                }
                c = c.push(r);
            }
            let fin = a.install_done;
            c = c.push(row![k::fill(), k::chip(&p, if fin { "Done" } else { "Hide (it keeps going)" }, if fin { Ch::Acc } else { Ch::Plain }, Sz::Md, Some(Msg::Close))]);
            ui::dialog_box(&p, 560.0, c)
        }
        Pop::Get(i) => {
            let x = a.installers.get(i)?;
            ui::dialog_box(
                &p,
                460.0,
                column![
                    k::h2::<Msg>(&p, x["name"].as_str().unwrap_or("").to_string()),
                    k::para(p.muted, format!("{} The community checked it ({}). Hearth installs it into {} with its extras, step by step, as the community's installer says.", x["about"].as_str().unwrap_or(""), x["grade"].as_str().unwrap_or("not graded"), a.pick), 12.0),
                    row![k::fill(), k::chip(&p, "Cancel", Ch::Plain, Sz::Md, Some(Msg::Close)), k::chip(&p, "Install", Ch::Acc, Sz::Md, Some(Msg::GetProgram(i)))].spacing(8.0),
                ]
                .spacing(10.0),
            )
        }
    })
}

fn view(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    studio_values(a);
    let mut items: Vec<(&'static str, E<'_, Msg>)> = vec![("side", hearth_ui::studio::made(&a.describe, "window", "side", || side(a))), ("page", page(a))];
    for (id, which) in [("pane:bottles", "bottles"), ("pane:running", "running")] {
        if a.describe.wants_pane(which) {
            items.push((id, studio_pane(a, which)));
        }
    }
    let mut body = hearth_ui::arrange::parts(&a.describe, &p, "window", items, move |ps| row![ps.take("side"), ui::vline(p.line), ps.take("page")].into(), Msg::Describe);
    if let Some(d) = pop_box(a) {
        body = ui::dialog(body, d, Some(Msg::Close));
    }
    let bar = k::Bar {
        letters: "BO",
        from: k::rgb("#7ff0d8"),
        to: k::rgb("#1f9f8f"),
        title: "Hearth Bottles",
        line: "· Windows apps, each in its own bottle".into(),
        middle: None,
        right: vec![k::chip(&p, "+ New bottle", Ch::Acc, Sz::Sm, a.have.ok().then_some(Msg::Pop(Pop::New)))],
        msgs: TitleMsgs { drag: Msg::Drag, maximize: Msg::Max, minimize: Some(Msg::Min), close: Msg::WClose },
        open: Msg::Describe(hearth_ui::describe::DMsg::Open),
    };
    k::window(&p, &a.describe, bar, body, ("Hearth Bottles", "Built on Bottles (GPL-3)"), Msg::Resize, Msg::Describe)
}

use hearth_ui::blocks::{bl, bls, PageDef};
const BLOCKS: &[PageDef] = &[
    PageDef { id: "window", title: "Hearth Bottles", blocks: &[bl("side", "Your bottles", 0, 0, 2, 12), bl("page", "The bottle", 2, 0, 10, 12)] },
    PageDef { id: "side", title: "Your bottles", blocks: &[bl("find", "Find a bottle or app", 0, 0, 12, 1), bls("bottles", "The bottles", 0, 1, 12, 7), bl("drop", "Run a .exe", 0, 8, 12, 2), bl("edit", "Edit layout", 0, 11, 12, 1)] },
    PageDef {
        id: "bottle",
        title: "A bottle",
        blocks: &[
            bl("head", "Its name, buttons and tags", 0, 0, 9, 2),
            bl("programs", "Programs in this bottle", 0, 2, 9, 4),
            bl("extras", "Extras installed", 0, 6, 4, 3),
            bl("snapshots", "Snapshots", 4, 6, 5, 3),
            bl("running", "Running now", 0, 9, 4, 3),
            bl("get", "Get a program", 4, 9, 5, 3),
            bls("settings", "This bottle's settings", 9, 0, 3, 12),
            bl("said", "What happened", 0, 11, 9, 1),
        ],
    },
];

use hearth_ui::studio::{tile, Kit};
const KIT: Kit = Kit {
    name: "Bottles",
    buttons: &[
        tile("run", "Run a program…", G::Play),
        tile("new", "New bottle", G::Plus),
        tile("c-drive", "Open its C: drive", G::Folder),
        tile("snapshot", "Take a snapshot", G::Camera),
        tile("stop-all", "Stop all", G::Stop),
        tile("backup", "Back up the bottle", G::Drive),
        tile("winecfg", "Wine settings", G::Gear),
        tile("reload", "Look again", G::Sync),
    ],
    panes: &[tile("bottles", "Your bottles", G::List), tile("running", "Running now", G::Pulse)],
    info: &[tile("bottles", "Bottles", G::Grid), tile("programs", "Programs", G::Grid), tile("running", "Running now", G::Pulse), tile("space", "Space", G::Drive)],
    pane_page: "window",
    bars: &[],
    menus: &[],
};

fn block_action(id: &str) -> Option<Msg> {
    Some(match id {
        "run" => Msg::RunProgram,
        "new" => Msg::Pop(Pop::New),
        "c-drive" => Msg::OpenC,
        "snapshot" => Msg::Pop(Pop::Take),
        "stop-all" => Msg::StopAll,
        "backup" => Msg::Backup,
        "winecfg" => Msg::Tool("winecfg"),
        "reload" => Msg::Reload,
        _ => return None,
    })
}

fn studio_values(a: &App) {
    let progs: usize = a.bottles.iter().map(|b| b.progs.len()).sum();
    let total: u64 = a.bottles.iter().map(|b| b.size).sum();
    a.describe.values(vec![("bottles", a.bottles.len().to_string()), ("programs", progs.to_string()), ("running", a.procs.len().to_string()), ("space", k::size_words(total))]);
}

fn studio_pane<'a>(a: &'a App, which: &str) -> E<'a, Msg> {
    let p = a.p;
    let mut c = Column::new().spacing(5.0);
    if which == "bottles" {
        for (i, b) in a.bottles.iter().enumerate() {
            c = c.push(k::click(row![bottle_mark(i, 12.0, 16.0), t(b.name.clone(), 12.0, p.text), k::fill(), k::mu1::<Msg>(&p, b.runner(), 10.0)].spacing(8.0).align_y(iced::Center), Some(Msg::Pick(b.name.clone()))));
        }
    } else {
        for pr in &a.procs {
            c = c.push(row![t(pr.words.clone(), 12.0, p.text), k::fill(), k::mu1::<Msg>(&p, k::size_words(pr.mem), 10.0)].spacing(8.0));
        }
    }
    scrollable(c).height(Length::Fill).into()
}

fn subscription(a: &App) -> Subscription<Msg> {
    let mut s = vec![
        Subscription::run(hearth_ui::theme::watch).map(|_| Msg::Theme),
        // a Windows program dropped on the window
        iced::event::listen_with(|e, _, _| match e {
            iced::Event::Window(iced::window::Event::FileDropped(p)) => Some(Msg::Dropped(p)),
            _ => None,
        }),
    ];
    // only while something runs in the bottle: its memory and CPU every 2 s
    if !a.procs.is_empty() {
        s.push(iced::time::every(std::time::Duration::from_secs(2)).map(|_| Msg::Tick));
    }
    // a community installer at work: its steps, twice a second
    if a.installing.is_some() && !a.install_done {
        s.push(iced::time::every(std::time::Duration::from_millis(500)).map(|_| Msg::InstallTick));
    }
    Subscription::batch(s)
}

fn main() -> iced::Result {
    if app::handle_cli("hearth-bottles", "Hearth Bottles: Windows apps, each in its own bottle (built on Bottles).\n  --bottle NAME   open that bottle\n  --install NAME  run that community installer in it") {
        return Ok(());
    }
    let w = app::window("hearth-bottles", (1280.0, 800.0), (960.0, 600.0));
    iced::application(App::new, update, view).title(|_: &App| "Hearth Bottles".to_string()).subscription(subscription).theme(|a: &App| app::iced_theme(&a.p)).window(w).default_font(app::INTER).run()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn runner_names() {
        assert_eq!(pretty_runner("wine-ge-8-26"), "Wine-GE 8-26");
        assert_eq!(pretty_runner("soda-7.0-9"), "Soda 7.0-9");
        assert_eq!(pretty_runner("sys-wine-9.20"), "Wine 9.20");
        assert_eq!(pretty_runner("proton-hearth-9-4"), "Proton Hearth 9-4");
    }
}
