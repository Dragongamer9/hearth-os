//! hearth-game-fixer: Hearth Game Fixer (docs/mockups/checklists/open-source-01-game-fixer.md; the approved board
//! docs/mockups/open-source-apps/01-game-fixer.png). Any Windows game, with the community's fixes: the games Lutris has,
//! each one's runner and its settings, the fixes its install script brought (each one turned off and back on), the
//! script's steps and the game's logs. Built on Lutris (GPL-3): everything here is Lutris' own (its command, its
//! games' settings files, lutris.net's install scripts); see lutris.rs.
//!
//! Made of blocks (hearth_ui::blocks / studio): the side and the page, the side's search, games and scripts, and the
//! game's banner, Runner, Fixes, Install script and Logs are each a block that can be moved, sized, hidden and
//! customized; its own buttons, panes and info are in its blocks window (Edit layout).
//!
//! `--game SLUG` opens that game; `--show install` opens Install a game.

mod lutris;

use hearth_oss_kit::need::{self, Have, Need};
use hearth_oss_kit::{self as k, Ch, Sz, Tag, E};
use hearth_ui::app::{self, win};
use hearth_ui::run;
use hearth_ui::ui::{self, t, tb, TitleMsgs, G};
use hearth_ui::Pal;
use iced::widget::canvas::{self, Frame, Geometry, Path};
use iced::widget::text::Wrapping;
use iced::widget::{column, container, row, scrollable, stack, Column, Row, Space};
use iced::{mouse, Color, Element, Length, Point, Rectangle, Renderer, Subscription, Task, Theme};
use lutris::{Fix, Game};
use serde_json::{json, Value};
use std::io::{BufRead, Write as _};

const LUTRIS: Need = Need { name: "Lutris", cmd: "lutris", flatpak: "net.lutris.Lutris", flatpak_cmd: "", licence: "GPL-3" };
const MONO: iced::Font = iced::Font::with_name("DejaVu Sans Mono");

#[derive(Debug, Clone, PartialEq)]
enum Pop {
    RunsWith,
    Resolution,
    Prelaunch,
    Extra,
    More,
    Install,
}

#[derive(Debug, Clone)]
enum Msg {
    Describe(hearth_ui::describe::DMsg),
    Theme,
    Have(Have),
    Games(run::Output),
    Wines(run::Output),
    Count(run::Output),
    Reload,
    Pick(String),
    Find(String),
    FindEnter,
    Play,
    Played(String, i32, u64),
    OpenFolder,
    Shortcut,
    Pop(Pop),
    Close,
    RunsWith(String),
    Graphics(u8),
    Screen(u8),
    Resolution(String),
    Text(String),
    SavePrelaunch,
    SaveExtra,
    Overlay(u8),
    Fix(String, bool),
    LogTab(u8),
    CopyLog,
    SendLogs,
    Sent(run::Output),
    OpenProton,
    ShowInLutris,
    CopyPath,
    Search(String),
    SearchGo,
    Found(run::Output),
    PickResult(String),
    Scripts(String, run::Output),
    InstallScript(usize),
    Installed(String, i32, u64),
    GetLutris,
    GotLutris(run::Output),
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
    games: Vec<Game>,
    loading: bool,
    problem: String,
    pick: String,
    find: String,
    wines: Vec<String>,
    others: Vec<String>,
    count: Option<u64>,
    log_tab: u8,
    logs: [String; 2],
    pop: Option<Pop>,
    text: String,
    search: String,
    results: Vec<Value>,
    picked_result: String,
    scripts: Vec<Value>,
    said: String,
    running: Option<String>,
    getting: bool,
}

fn now() -> i64 {
    hearth_ui::time::now()
}
fn hms(ts: i64) -> String {
    let t = hearth_ui::time::local(ts);
    format!("{:02}:{:02}:{:02}", t.hour, t.min, t.sec)
}

impl App {
    fn new() -> (App, Task<Msg>) {
        let a = App {
            describe: hearth_ui::describe::Describe::new("game-fixer", "Hearth Game Fixer").with_pages(BLOCKS).with_kit(&KIT),
            p: hearth_ui::theme::load(),
            have: Have::Looking,
            games: Vec::new(),
            loading: true,
            problem: String::new(),
            pick: app::arg("--game").unwrap_or_default(),
            find: String::new(),
            wines: Vec::new(),
            others: lutris::other_runners(),
            count: None,
            log_tab: 0,
            logs: [String::new(), String::new()],
            pop: if app::arg("--show").as_deref() == Some("install") { Some(Pop::Install) } else { None },
            text: String::new(),
            search: String::new(),
            results: Vec::new(),
            picked_result: String::new(),
            scripts: Vec::new(),
            said: String::new(),
            running: None,
            getting: false,
        };
        let t = Task::batch([Task::perform(need::probe(&LUTRIS), Msg::Have), curl("https://lutris.net/api/installers?page_size=1", Msg::Count)]);
        (a, t)
    }
    fn game(&self) -> Option<&Game> {
        self.games.iter().find(|g| g.slug == self.pick).or(self.games.first())
    }
    fn lutris(&self, args: &[&str]) -> Vec<String> {
        need::argv(&LUTRIS, &self.have, args)
    }
    fn load_logs(&mut self) {
        let slug = self.game().map(|g| g.slug.clone()).unwrap_or_default();
        for (i, k) in ["start", "install"].iter().enumerate() {
            self.logs[i] = std::fs::read_to_string(lutris::logs_dir().join(format!("{slug}-{k}.log"))).unwrap_or_default();
        }
    }
    /// The picked game's settings file changed: read again.
    fn reread(&mut self, slug: &str) {
        if let Some(g) = self.games.iter_mut().find(|g| g.slug == slug) {
            g.config = lutris::config_for(&g.slug);
            g.yaml = g.config.as_ref().and_then(|c| std::fs::read_to_string(c).ok()).unwrap_or_default();
            g.note = lutris::read_note(&g.slug);
        }
    }
    /// The picked game's Lutris settings changed one line at a time (atomic, .bak; red flag 9).
    fn change(&mut self, f: impl FnOnce(&str) -> String) {
        let Some(g) = self.game().cloned() else { return };
        let Some(path) = g.config.clone() else {
            self.said = format!("Lutris has no settings file for {} yet, so nothing was changed.", g.name);
            return;
        };
        match k::settings_file::update_text(&path, f) {
            Ok(_) => self.said.clear(),
            Err(e) => self.said = format!("Couldn't change {}'s settings: {e}", g.name),
        }
        self.reread(&g.slug);
    }
}

/// curl (lutris.net's API), off the window's thread.
fn curl(url: &str, m: impl Fn(run::Output) -> Msg + Send + 'static) -> Task<Msg> {
    Task::perform(run::spawn(run::args(&["curl", "-fsSL", "-m", "25", "-H", "Accept: application/json", url])), m)
}

/// A program run off the window's thread with what it prints kept in `log` (each line with its time), and how it
/// ended (its exit code, how long it ran).
fn run_logged(cmd: Vec<String>, log: std::path::PathBuf, first: String) -> impl std::future::Future<Output = (i32, u64)> + Send + 'static {
    let (tx, rx) = futures::channel::oneshot::channel();
    let _ = std::thread::Builder::new().name("game-fixer-run".into()).spawn(move || {
        let started = std::time::Instant::now();
        if let Some(d) = log.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        let file = std::sync::Arc::new(std::sync::Mutex::new(std::fs::File::create(&log).ok()));
        let say = {
            let file = file.clone();
            move |l: &str| {
                if let Ok(mut f) = file.lock() {
                    if let Some(f) = f.as_mut() {
                        let _ = writeln!(f, "{} {}", hms(now()), l);
                    }
                }
            }
        };
        say(&first);
        let child = std::process::Command::new(&cmd[0]).args(&cmd[1..]).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).spawn();
        let code = match child {
            Ok(mut c) => {
                let out = c.stdout.take();
                let err = c.stderr.take();
                let s1 = say.clone();
                let h = std::thread::spawn(move || {
                    if let Some(o) = out {
                        for l in std::io::BufReader::new(o).lines().map_while(Result::ok) {
                            s1(&l);
                        }
                    }
                });
                if let Some(e) = err {
                    for l in std::io::BufReader::new(e).lines().map_while(Result::ok) {
                        say(&l);
                    }
                }
                let _ = h.join();
                c.wait().ok().and_then(|s| s.code()).unwrap_or(-1)
            }
            Err(e) => {
                say(&format!("err: {} couldn't start: {e}", cmd[0]));
                -1
            }
        };
        let secs = started.elapsed().as_secs();
        if code == 0 {
            say(&format!("Game closed normally · played {} min", (secs + 30) / 60));
        } else {
            say(&format!("It stopped by itself after {secs} s (code {code})"));
        }
        let _ = tx.send((code, secs));
    });
    async move { rx.await.unwrap_or((-1, 0)) }
}

fn obj(v: Value) -> serde_json::Map<String, Value> {
    v.as_object().cloned().unwrap_or_default()
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
        Msg::Have(h) => {
            a.have = h;
            if a.have.ok() {
                return update(a, Msg::Reload);
            }
            a.loading = false;
        }
        Msg::Reload => {
            a.loading = true;
            a.others = lutris::other_runners();
            return Task::batch([
                Task::perform(run::spawn(a.lutris(&["--list-games", "--installed", "--json"])), Msg::Games),
                Task::perform(run::spawn(a.lutris(&["--list-wine-versions"])), Msg::Wines),
            ]);
        }
        Msg::Games(o) => {
            a.loading = false;
            let j = o.json().unwrap_or(Value::Null);
            if !o.ok && j.is_null() {
                a.problem = format!("Lutris couldn't list its games: {}", o.why());
                return Task::none();
            }
            a.problem.clear();
            let mut gs = lutris::games_of(&j);
            for g in gs.iter_mut() {
                g.config = lutris::config_for(&g.slug);
                g.yaml = g.config.as_ref().and_then(|c| std::fs::read_to_string(c).ok()).unwrap_or_default();
                g.note = lutris::read_note(&g.slug);
            }
            a.games = gs;
            if !a.games.iter().any(|g| g.slug == a.pick) {
                a.pick = a.games.first().map(|g| g.slug.clone()).unwrap_or_default();
            }
            a.load_logs();
        }
        Msg::Wines(o) => a.wines = lutris::wine_versions(&o.out),
        Msg::Count(o) => {
            a.count = o.json().and_then(|j| j.get("count").and_then(Value::as_u64));
        }
        Msg::Pick(s) => {
            a.pick = s;
            a.said.clear();
            a.load_logs();
        }
        Msg::Find(s) => a.find = s,
        Msg::FindEnter => {
            // nothing of yours by that name: look for it in the community's scripts
            let q = a.find.to_lowercase();
            if !q.is_empty() && !a.games.iter().any(|g| g.name.to_lowercase().contains(&q)) {
                a.search = a.find.clone();
                a.pop = Some(Pop::Install);
                return update(a, Msg::SearchGo);
            }
        }
        Msg::Play => {
            let Some(g) = a.game().cloned() else { return Task::none() };
            if a.running.is_some() {
                return Task::none();
            }
            a.running = Some(g.slug.clone());
            a.said = format!("Starting {}…", g.name);
            let cmd = a.lutris(&[&format!("lutris:rungame/{}", g.slug)]);
            let log = lutris::logs_dir().join(format!("{}-start.log", g.slug));
            let slug = g.slug.clone();
            return Task::perform(run_logged(cmd, log, format!("Starting {} with {}", g.name, g.runs_with())), move |(c, s)| Msg::Played(slug.clone(), c, s));
        }
        Msg::Played(slug, code, secs) => {
            a.running = None;
            // closing by itself within half a minute: it needs a fix (shown in the list until it starts well)
            let bad = code != 0 && secs < 30;
            let _ = lutris::write_note(&slug, obj(json!({"needsFix": bad, "lastStart": now()})));
            a.said = if bad { "It stopped by itself right away. Its log is below; Send logs if you'd like help.".into() } else { String::new() };
            a.reread(&slug);
            a.load_logs();
        }
        Msg::OpenFolder => {
            if let Some(g) = a.game() {
                let d = g.dir();
                run::detached(&["xdg-open", &d.to_string_lossy()]);
            }
        }
        Msg::Shortcut => {
            let Some(g) = a.game().cloned() else { return Task::none() };
            let body = lutris::shortcut(&g, &a.lutris(&[]));
            let name = format!("hearth-game-{}.desktop", g.slug);
            let apps = k::home().join(".local/share/applications");
            let desk = std::process::Command::new("xdg-user-dir").arg("DESKTOP").output().ok().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).filter(|s| !s.is_empty()).map(std::path::PathBuf::from).unwrap_or_else(|| k::home().join("Desktop"));
            let mut ok = Vec::new();
            for d in [apps, desk] {
                if std::fs::create_dir_all(&d).is_ok() && k::settings_file::write_text(&d.join(&name), &body).is_ok() {
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        let _ = std::fs::set_permissions(d.join(&name), std::fs::Permissions::from_mode(0o755));
                    }
                    ok.push(k::tilde(&d));
                }
            }
            a.said = if ok.is_empty() { "Couldn't make the shortcut.".into() } else { format!("{} is on your desktop and in the Apps menu ({}).", g.name, ok.join(", ")) };
        }
        Msg::Pop(p) => {
            a.text = match (&p, a.game()) {
                (Pop::Prelaunch, Some(g)) => g.get(&["system", "prelaunch_command"]).unwrap_or_default(),
                (Pop::Extra, Some(g)) => lutris::ymap(&g.yaml, &["system", "env"]).into_iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join("; "),
                _ => String::new(),
            };
            a.pop = Some(p);
        }
        Msg::Close => a.pop = None,
        Msg::RunsWith(v) => {
            a.pop = None;
            let q = lutris::quote(&v);
            a.change(move |t| lutris::yset(t, &["wine", "version"], Some(q.as_str())));
        }
        Msg::Graphics(i) => a.change(|t| {
            let t = lutris::yset(t, &["wine", "dxvk"], Some(if i == 0 { "true" } else { "false" }));
            lutris::yset(&t, &["wine", "Renderer"], if i == 1 { Some("gl") } else { None })
        }),
        Msg::Screen(i) => a.change(|t| {
            let t = lutris::yset(t, &["wine", "Desktop"], if i == 0 { Some("true") } else { None });
            lutris::yset(&t, &["system", "gamescope"], if i == 2 { Some("true") } else { None })
        }),
        Msg::Resolution(r) => {
            a.pop = None;
            a.change(move |t| lutris::yset(t, &["system", "resolution"], if r.is_empty() { None } else { Some(r.as_str()) }));
        }
        Msg::Text(s) => a.text = s,
        Msg::SavePrelaunch => {
            a.pop = None;
            let v = lutris::quote(a.text.trim());
            let empty = a.text.trim().is_empty();
            a.change(move |t| lutris::yset(t, &["system", "prelaunch_command"], if empty { None } else { Some(v.as_str()) }));
        }
        Msg::SaveExtra => {
            a.pop = None;
            let want: Vec<(String, String)> = a.text.split(';').filter_map(|p| p.trim().split_once('=')).map(|(k, v)| (k.trim().to_string(), v.trim().to_string())).filter(|(k, _)| !k.is_empty() && !k.contains(' ')).collect();
            a.change(|t| {
                let mut t = t.to_string();
                for (k, _) in lutris::ymap(&t, &["system", "env"]) {
                    if !want.iter().any(|w| w.0 == k) {
                        t = lutris::yset(&t, &["system", "env", &k], None);
                    }
                }
                for (k, v) in &want {
                    let q = lutris::quote(v);
                    t = lutris::yset(&t, &["system", "env", k], Some(q.as_str()));
                }
                t
            });
        }
        Msg::Overlay(i) => a.change(|t| {
            let t = lutris::yset(t, &["system", "mangohud"], if i == 0 { None } else { Some("true") });
            lutris::yset(&t, &["system", "env", "MANGOHUD_CONFIG"], if i == 1 { Some("preset=1") } else { None })
        }),
        Msg::Fix(id, on) => {
            let Some(g) = a.game().cloned() else { return Task::none() };
            let Some(f) = lutris::fixes_of(&g).into_iter().find(|f| f.id == id) else { return Task::none() };
            if let Err(e) = lutris::move_fix_files(&g.dir(), &f, on) {
                a.said = format!("Couldn't move the fix's files: {e}");
                return Task::none();
            }
            if !f.set.is_empty() {
                a.change(|t| lutris::apply_fix(t, &f, on));
            } else {
                a.reread(&g.slug);
            }
        }
        Msg::LogTab(i) => a.log_tab = i,
        Msg::CopyLog => {
            let t = a.logs[a.log_tab as usize].clone();
            return iced::clipboard::write(t);
        }
        Msg::SendLogs => {
            let Some(g) = a.game() else { return Task::none() };
            let tail: Vec<&str> = a.logs[a.log_tab as usize].lines().rev().take(30).collect::<Vec<_>>().into_iter().rev().collect();
            let note = format!("Hearth Game Fixer · {} ({}) · {}: {}", g.name, g.runs_with(), if a.log_tab == 0 { "last start" } else { "install" }, tail.join(" | "));
            a.said = "Sending logs…".into();
            return Task::perform(run::spawn(vec!["hearth-send-logs".into(), note.chars().take(3000).collect()]), Msg::Sent);
        }
        Msg::Sent(o) => a.said = if o.ok { format!("Sent. {}", o.out.lines().last().unwrap_or("").trim()) } else { format!("Couldn't send the logs: {}", o.why()) },
        Msg::OpenProton => run::detached(&["hearth-proton-manager"]),
        Msg::ShowInLutris => {
            a.pop = None;
            let c = a.lutris(&[]);
            let v: Vec<&str> = c.iter().map(String::as_str).collect();
            run::detached(&v);
        }
        Msg::CopyPath => {
            a.pop = None;
            if let Some(g) = a.game() {
                return iced::clipboard::write(g.dir().to_string_lossy().to_string());
            }
        }
        Msg::Search(s) => a.search = s,
        Msg::SearchGo => {
            let q: String = a.search.trim().chars().map(|c| if c.is_alphanumeric() { c } else { '+' }).collect();
            if q.is_empty() {
                return Task::none();
            }
            a.scripts.clear();
            a.picked_result.clear();
            return curl(&format!("https://lutris.net/api/games?search={q}"), Msg::Found);
        }
        Msg::Found(o) => {
            a.results = o.json().and_then(|j| j.get("results").and_then(Value::as_array).cloned()).unwrap_or_default();
            if !o.ok {
                a.said = format!("Couldn't reach lutris.net: {}", o.why());
            }
        }
        Msg::PickResult(slug) => {
            a.picked_result = slug.clone();
            let s2 = slug.clone();
            return curl(&format!("https://lutris.net/api/installers/{slug}"), move |o| Msg::Scripts(s2.clone(), o));
        }
        Msg::Scripts(_, o) => {
            a.scripts = o.json().and_then(|j| j.get("results").and_then(Value::as_array).cloned().or_else(|| j.as_array().cloned())).unwrap_or_default();
        }
        Msg::InstallScript(i) => {
            let Some(s) = a.scripts.get(i).cloned() else { return Task::none() };
            let game_slug = s.get("game_slug").and_then(Value::as_str).unwrap_or(&a.picked_result).to_string();
            let inst = s.get("slug").and_then(Value::as_str).unwrap_or("").to_string();
            if inst.is_empty() || a.running.is_some() {
                return Task::none();
            }
            let mut note = json!({"script": lutris::script_note(&s), "installing": 0, "about": format!("installed with the community's script ({})", s.get("version").and_then(Value::as_str).unwrap_or(""))});
            if let Some(n) = s.get("installs").and_then(Value::as_u64) {
                note["script"]["installs"] = json!(n);
            }
            let _ = lutris::write_note(&game_slug, obj(note));
            a.pop = None;
            a.running = Some(game_slug.clone());
            a.said = "Lutris asks its questions (where your disc images are) in its own window; the steps and the log show here.".into();
            let cmd = a.lutris(&[&format!("lutris:install/{inst}")]);
            let log = lutris::logs_dir().join(format!("{game_slug}-install.log"));
            let gs = game_slug.clone();
            return Task::perform(run_logged(cmd, log, format!("Installing with the script {inst}")), move |(c, s)| Msg::Installed(gs.clone(), c, s));
        }
        Msg::Installed(slug, code, _) => {
            a.running = None;
            let mut n = lutris::read_note(&slug);
            if let Some(steps) = n.pointer_mut("/script/steps").and_then(Value::as_array_mut) {
                for s in steps.iter_mut() {
                    s["done"] = json!(code == 0);
                }
            }
            let mut keys = obj(json!({"installing": Value::Null}));
            if let Some(s) = n.get("script") {
                keys.insert("script".into(), s.clone());
            }
            let _ = lutris::write_note(&slug, keys);
            a.said = if code == 0 { "Installed.".into() } else { format!("The install didn't finish (code {code}). Its log is under Logs › Install.") };
            a.pick = slug;
            a.log_tab = 1;
            return update(a, Msg::Reload);
        }
        Msg::GetLutris => {
            a.getting = true;
            a.said = "Getting Lutris from Flathub…".into();
            return Task::perform(run::spawn(need::install_argv(&LUTRIS)), Msg::GotLutris);
        }
        Msg::GotLutris(o) => {
            a.getting = false;
            a.said = if o.ok { "Lutris is here.".into() } else { format!("Couldn't get Lutris: {}", o.why()) };
            return Task::perform(need::probe(&LUTRIS), Msg::Have);
        }
        Msg::Drag => return win::drag(),
        Msg::Max => return win::toggle_maximize(),
        Msg::Min => return win::minimize(),
        Msg::WClose => return win::close(),
        Msg::Resize(d) => return win::resize(d),
    }
    Task::none()
}

// ------------------------------------------------------------------ the side

fn colours(g: &Game) -> (Color, Color) {
    if let Some(a) = g.note.get("art").and_then(Value::as_array) {
        if let (Some(x), Some(y)) = (a.first().and_then(Value::as_str), a.get(1).and_then(Value::as_str)) {
            return (k::rgb(x), k::rgb(y));
        }
    }
    let h = g.slug.bytes().fold(7u32, |h, b| h.wrapping_mul(31).wrapping_add(b as u32));
    let pal = [("#6a7a3a", "#1e2614"), ("#3a6aa8", "#0d1424"), ("#c8a050", "#3a2a10"), ("#a85a3a", "#2a120a"), ("#7a4aa8", "#1a0d2a"), ("#3a8a6a", "#0d2a20"), ("#a83a4a", "#2a0d14"), ("#4a8aa8", "#0d202a")];
    let (x, y) = pal[(h as usize) % pal.len()];
    (k::rgb(x), k::rgb(y))
}

fn thumb<'a>(g: &Game, s: f32) -> E<'a, Msg> {
    let (x, y) = colours(g);
    let gr = iced::gradient::Linear::new(iced::Degrees(135.0)).add_stop(0.0, x).add_stop(1.0, y);
    container(Space::new())
        .width(Length::Fixed(s))
        .height(Length::Fixed(s))
        .style(move |_: &Theme| container::Style { background: Some(iced::Background::Gradient(iced::Gradient::Linear(gr))), border: iced::Border { radius: 6.0.into(), ..iced::Border::default() }, ..container::Style::default() })
        .into()
}

fn side(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let search: E<'_, Msg> = k::input(&p, "⌕ Search games and scripts", &a.find, Msg::Find).on_submit(Msg::FindEnter).into();
    let q = a.find.to_lowercase();
    let mut list = Column::new().spacing(1.0);
    let shown: Vec<&Game> = a.games.iter().filter(|g| q.is_empty() || g.name.to_lowercase().contains(&q)).collect();
    for g in &shown {
        let (st, kind) = g.state();
        let c = match kind {
            1 => k::YELLOW,
            2 => p.accent,
            _ => p.muted,
        };
        let line = format!("{} · {}", g.runs_with(), st);
        let inner = row![thumb(g, 26.0), column![t(ui::fit(&g.name, 27), 12.0, p.text).wrapping(Wrapping::None), t(line, 10.0, c).wrapping(Wrapping::None)].spacing(1.0)].spacing(9.0).align_y(iced::Center);
        list = list.push(k::side_row(&p, inner, a.game().map(|x| x.slug == g.slug).unwrap_or(false), 30.0, Msg::Pick(g.slug.clone())));
    }
    if a.loading {
        list = list.push(k::mu(&p, "Asking Lutris for your games…", 12.0));
    } else if a.games.is_empty() && a.have.ok() && a.problem.is_empty() {
        list = list.push(k::mu(&p, "No games in Lutris yet. ＋ Install a game picks one with the community's script.", 12.0));
    }
    let games = column![k::cap(&p, format!("Your games · {}", a.games.len())), gap(6.0), scrollable(list).height(Length::Fill)].spacing(0.0);
    let words = match a.count {
        Some(n) => format!("{} install scripts, checked by people who play them. Pick a game and Hearth does the rest: runner, fixes, extras.", k::commas(n)),
        None => "Install scripts made and checked by people who play them. Pick a game and Hearth does the rest: runner, fixes, extras.".into(),
    };
    let scripts = column![k::cap(&p, "Community scripts"), gap(4.0), k::para(p.muted, words, 11.0), gap(6.0), k::chip(&p, "Browse scripts", Ch::Plain, Sz::Sm, Some(Msg::Pop(Pop::Install)))];
    let edit = k::edit_layout(&p, Msg::Describe(hearth_ui::arrange::edit_layout()));
    let body = hearth_ui::arrange::parts(
        &a.describe,
        &p,
        "side",
        vec![("search", search), ("games", games.height(Length::Fill).into()), ("scripts", scripts.into()), ("edit", edit)],
        |ps| column![ps.take("search"), gap(10.0), ps.take("games"), gap(10.0), ps.take("scripts"), gap(6.0), ps.take("edit")].into(),
        Msg::Describe,
    );
    k::side(&p, body, 212.0)
}

fn gap<'a>(h: f32) -> E<'a, Msg> {
    k::gap(1.0, h)
}

// ------------------------------------------------------------------ the game's page

struct Hills;
impl canvas::Program<Msg> for Hills {
    type State = ();
    fn draw(&self, _s: &(), r: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        let mut f = Frame::new(r, b.size());
        // the board's warm light top right, and its hills along the bottom
        f.fill(&Path::circle(Point::new(b.width * 0.8, b.height * 0.1), b.height * 0.9), Color { r: 1.0, g: 0.85, b: 0.54, a: 0.07 });
        let w = (b.width * 0.56).min(900.0 * 0.7);
        let x0 = b.width - w;
        let h = b.height * 0.6;
        let pts = [(0.0, 1.0), (0.10, 0.5), (0.22, 0.7), (0.36, 0.2), (0.50, 0.55), (0.64, 0.3), (0.78, 0.6), (0.90, 0.25), (1.0, 0.45), (1.0, 1.0)];
        let path = Path::new(|pb| {
            pb.move_to(Point::new(x0, b.height));
            for (x, y) in pts {
                pb.line_to(Point::new(x0 + x * w, b.height - h + y * h));
            }
            pb.close();
        });
        f.fill(&path, Color::from_rgb8(0x1e, 0x26, 0x14));
        vec![f.into_geometry()]
    }
}

fn hero(a: &App, g: &Game) -> E<'static, Msg> {
    let p = a.p;
    let (st, kind) = g.state();
    let mut tags = Row::new().spacing(6.0);
    tags = tags.push(match kind {
        0 => k::tag(&p, "Ready to play", Tag::Ok),
        1 => k::tag(&p, "Needs a fix", Tag::Warn),
        _ => k::tag(&p, st, Tag::Acc),
    });
    if let Some(s) = g.note.get("script") {
        let name = s.get("name").and_then(Value::as_str).unwrap_or("");
        let ver = s.get("v").and_then(Value::as_str).unwrap_or("");
        if !name.is_empty() {
            tags = tags.push(k::tag(&p, if ver.is_empty() { format!("Script: {name}") } else { format!("Script: {name} · v{ver}") }, Tag::Dk));
        }
    }
    let size = g.note.get("size").and_then(Value::as_u64).filter(|s| *s > 0);
    if let Some(s) = size {
        tags = tags.push(k::tag(&p, k::size_words(s), Tag::Dk));
    }
    let about = g.note.get("about").and_then(Value::as_str).map(str::to_string).unwrap_or_else(|| format!("{} · in Lutris", if g.platform.is_empty() { "Windows".to_string() } else { g.platform.clone() }));
    let words = column![tags, gap(6.0), tb(g.name.clone(), 28.0, p.text).wrapping(Wrapping::None), t(about, 13.0, p.muted).wrapping(Wrapping::None)].spacing(2.0);
    let playing = a.running.as_deref() == Some(g.slug.as_str());
    let btns = row![
        k::chip(&p, if playing { "Playing…" } else { "▶ Play" }, Ch::Acc, Sz::Lg, (a.running.is_none() && a.have.ok()).then_some(Msg::Play)),
        k::chip(&p, "Open folder", Ch::Plain, Sz::Lg, Some(Msg::OpenFolder)),
        k::chip(&p, "Make a desktop shortcut", Ch::Plain, Sz::Lg, Some(Msg::Shortcut)),
        k::chip(&p, "⋯", Ch::Plain, Sz::Lg, Some(Msg::Pop(Pop::More))),
    ]
    .spacing(7.0)
    .align_y(iced::Center);
    let front = container(row![words, k::fill(), btns].align_y(iced::Bottom)).padding(k::pad(0.0, 22.0, 18.0, 22.0)).width(Length::Fill).height(Length::Fill).align_y(iced::alignment::Vertical::Bottom);
    let (x, y) = colours(g);
    let gr = iced::gradient::Linear::new(iced::Degrees(120.0)).add_stop(0.0, x).add_stop(0.4, Pal::mix(x, y, 0.6)).add_stop(1.0, p.bg);
    let back = container(Space::new()).width(Length::Fill).height(Length::Fill).style(move |_: &Theme| container::Style { background: Some(iced::Background::Gradient(iced::Gradient::Linear(gr))), ..container::Style::default() });
    let hills = canvas::Canvas::new(Hills).width(Length::Fill).height(Length::Fill);
    container(stack![back, hills, front]).width(Length::Fill).height(Length::Fixed(160.0)).into()
}

fn graphics_of(g: &Game) -> u8 {
    if g.get(&["wine", "dxvk"]).map(|v| v != "false" && v != "False").unwrap_or(true) {
        0
    } else if g.get(&["wine", "Renderer"]).as_deref() == Some("gl") {
        1
    } else {
        2
    }
}
fn screen_of(g: &Game) -> u8 {
    if g.get(&["system", "gamescope"]).as_deref() == Some("true") {
        2
    } else if g.get(&["wine", "Desktop"]).as_deref() == Some("true") {
        0
    } else {
        1
    }
}
fn overlay_of(g: &Game) -> u8 {
    match (g.get(&["system", "mangohud"]).as_deref() == Some("true"), g.get(&["system", "env", "MANGOHUD_CONFIG"])) {
        (false, _) => 0,
        (true, Some(c)) if c.contains("preset=1") => 1,
        _ => 2,
    }
}

fn runner_card(a: &App, g: &Game) -> E<'static, Msg> {
    let p = a.p;
    let wine = g.runner == "wine";
    let mut c = Column::new().spacing(1.0).push(k::h2(&p, "Runner")).push(k::mu(&p, "What runs this Windows game on Hearth", 11.0)).push(gap(8.0));
    c = c.push(k::kv(&p, "Runs with", k::field(&p, format!("{} ▾", g.runs_with()), 150.0, wine.then_some(Msg::Pop(Pop::RunsWith)))));
    let asks = g.note.pointer("/script/asks").and_then(Value::as_str).unwrap_or("");
    c = c.push(k::kv(&p, "Script asks for", k::mu1(&p, if asks.is_empty() { "nothing in particular".to_string() } else { asks.to_string() }, 11.0)));
    if wine {
        let dx = g.get(&["wine", "dxvk_version"]).map(|v| format!("DXVK {}", v.trim_start_matches('v'))).unwrap_or_else(|| "DXVK".into());
        let gi = graphics_of(g);
        c = c.push(k::kv(&p, "Graphics", k::seg(&p, vec![(dx, gi == 0, Msg::Graphics(0)), ("OpenGL".into(), gi == 1, Msg::Graphics(1)), ("WineD3D".into(), gi == 2, Msg::Graphics(2))])));
        let si = screen_of(g);
        c = c.push(k::kv(&p, "Screen", k::seg(&p, vec![("Window".into(), si == 0, Msg::Screen(0)), ("Full screen".into(), si == 1, Msg::Screen(1)), ("Gamescope".into(), si == 2, Msg::Screen(2))])));
    }
    let res = g.get(&["system", "resolution"]).map(|r| r.replace('x', " × ")).unwrap_or_else(|| "As it is".into());
    c = c.push(k::kv(&p, "Resolution", k::field(&p, format!("{res} ▾"), 0.0, Some(Msg::Pop(Pop::Resolution)))));
    let prefix = g.get(&["game", "prefix"]).map(|x| format!("{} · {}", x, if g.get(&["game", "arch"]).as_deref() == Some("win32") { "32-bit" } else { "64-bit" })).unwrap_or_else(|| "Lutris' own".into());
    if wine {
        c = c.push(k::kv(&p, "Prefix", k::mu1(&p, prefix, 11.0)));
    }
    let pre = g.get(&["system", "prelaunch_command"]).unwrap_or_else(|| "Nothing".into());
    c = c.push(k::kv(&p, "Run before the game", k::field(&p, ui::fit(&pre, 26), 150.0, Some(Msg::Pop(Pop::Prelaunch)))));
    let env = lutris::ymap(&g.yaml, &["system", "env"]);
    let extra = match env.split_first() {
        Some(((kk, v), rest)) if rest.is_empty() => format!("{kk}={v}"),
        Some(((kk, v), rest)) => format!("{kk}={v} · {} more", rest.len()),
        None => "None".into(),
    };
    c = c.push(k::kv(&p, "Extra settings", k::click(k::mu1::<Msg>(&p, ui::fit(&extra, 40), 11.0), Some(Msg::Pop(Pop::Extra)))));
    let oi = overlay_of(g);
    c = c.push(k::kv(&p, "Overlay in this game", k::seg(&p, vec![("Off".into(), oi == 0, Msg::Overlay(0)), ("FPS".into(), oi == 1, Msg::Overlay(1)), ("Full".into(), oi == 2, Msg::Overlay(2))])));
    c = c.push(gap(8.0)).push(k::cap(&p, "Other runners you have")).push(gap(5.0));
    let mut chips: Vec<E<'static, Msg>> = Vec::new();
    for w in a.wines.iter() {
        let pw = lutris::pretty(w);
        chips.push(k::chip(&p, pw, Ch::Plain, Sz::Sm, (wine && g.runs_with() != lutris::pretty(w)).then(|| Msg::RunsWith(w.clone()))));
    }
    for r in a.others.iter() {
        chips.push(k::chip(&p, r.clone(), Ch::Plain, Sz::Sm, None));
    }
    // wrapped by hand: three or four to a line
    let mut lines = Column::new().spacing(5.0);
    let mut cur = Row::new().spacing(5.0);
    let mut n = 0;
    let mut width = 0usize;
    let names: Vec<String> = a.wines.iter().map(|w| lutris::pretty(w)).chain(a.others.iter().cloned()).collect();
    for (i, ch) in chips.into_iter().enumerate() {
        let w = names.get(i).map(|s| s.chars().count() + 4).unwrap_or(10);
        if n > 0 && width + w > 46 {
            lines = lines.push(cur);
            cur = Row::new().spacing(5.0);
            width = 0;
            n = 0;
        }
        cur = cur.push(ch);
        width += w;
        n += 1;
    }
    if n > 0 {
        lines = lines.push(cur);
    }
    c = c.push(lines);
    c = c.push(Space::new().height(Length::Fill));
    c = c.push(k::line(&p)).push(gap(8.0)).push(row![k::mu1::<Msg>(&p, "Managed in Settings › Gaming › Proton and Wine", 11.0), k::fill(), k::chip(&p, "Open", Ch::Plain, Sz::Sm, Some(Msg::OpenProton))].align_y(iced::Center));
    k::card(&p, c)
}

fn fixes_card(a: &App, g: &Game) -> E<'static, Msg> {
    let p = a.p;
    let fixes: Vec<Fix> = lutris::fixes_of(g);
    let on: Vec<bool> = fixes.iter().map(|f| lutris::fix_on(g, f)).collect();
    let n_on = on.iter().filter(|x| **x).count();
    let from_script = fixes.iter().any(|f| f.from_script);
    let label = if from_script { format!("{n_on} on · from the script") } else { format!("{n_on} on") };
    let mut c = Column::new().push(row![k::h2::<Msg>(&p, "Fixes"), k::fill(), k::tag(&p, label, Tag::Dk)].align_y(iced::Center)).push(k::mu(&p, "Each one can be turned off and back on", 11.0)).push(gap(6.0));
    let last = fixes.len().saturating_sub(1);
    for (i, f) in fixes.into_iter().enumerate() {
        let id = f.id.clone();
        let r = row![column![t(f.name.clone(), 12.0, p.text).wrapping(Wrapping::None), t(f.about.clone(), 10.0, p.muted).wrapping(Wrapping::None)].spacing(1.0).width(Length::Fill), k::switch(&p, on[i], Some(Msg::Fix(id, !on[i])))].spacing(8.0).align_y(iced::Center);
        c = c.push(container(r).padding(k::pad(7.0, 0.0, 7.0, 0.0)));
        if i < last {
            c = c.push(k::line(&p));
        }
    }
    k::card(&p, scrollable(c).height(Length::Fill))
}

fn script_card(a: &App, g: &Game) -> E<'static, Msg> {
    let p = a.p;
    let s = g.note.get("script").cloned().unwrap_or(Value::Null);
    let by = s.get("by").and_then(Value::as_str).filter(|x| !x.is_empty()).unwrap_or("the community").to_string();
    let mut meta = vec![format!("by {by}")];
    if let Some(n) = s.get("installs").and_then(Value::as_u64) {
        meta.push(format!("{} installs", k::commas(n)));
    }
    let d = k::day_words(s.get("updated").and_then(Value::as_str).unwrap_or(""));
    if !d.is_empty() {
        meta.push(format!("updated {d}"));
    }
    let mut c = Column::new().push(row![k::h2::<Msg>(&p, "Install script"), k::fill(), k::mu1(&p, if s.is_null() { "not installed with a script".to_string() } else { meta.join(" · ") }, 10.0)].align_y(iced::Center)).push(gap(6.0));
    let steps = s.get("steps").and_then(Value::as_array).cloned().unwrap_or_default();
    let installing = g.note.get("installing").is_some();
    let mut seen_now = false;
    for st in steps.iter() {
        let done = st.get("done").and_then(Value::as_bool).unwrap_or(false);
        let now_ = !done && installing && !seen_now;
        if now_ {
            seen_now = true;
        }
        let (bg, fg, line, mark) = if done {
            (k::rgb("#21361b"), k::GREEN, k::rgb("#3b5a2e"), "✓")
        } else if now_ {
            (p.accent, p.ink, p.accent, "…")
        } else {
            (Color::TRANSPARENT, p.muted, p.line, "")
        };
        let circle = container(tb(mark, 10.0, fg)).center_x(Length::Fixed(18.0)).center_y(Length::Fixed(18.0)).style(ui::boxed(bg, 9.0, Some((line, 1.0))));
        c = c.push(row![circle, t(st.get("text").and_then(Value::as_str).unwrap_or("").to_string(), 12.0, p.text)].spacing(10.0).align_y(iced::Center).padding(k::pad(4.0, 0.0, 4.0, 0.0)));
    }
    if steps.is_empty() {
        c = c.push(k::mu(&p, "This game wasn't installed with a community script, so there are no steps to show.", 11.0));
    }
    k::card_fit(&p, c)
}

fn logs_card(a: &App) -> E<'static, Msg> {
    let p = a.p;
    let head = row![
        k::h2::<Msg>(&p, "Logs"),
        k::gap(8.0, 1.0),
        k::tabs(&p, vec![("Last start".into(), a.log_tab == 0, Msg::LogTab(0)), ("Install".into(), a.log_tab == 1, Msg::LogTab(1))]),
        k::fill(),
        k::chip(&p, "Copy", Ch::Plain, Sz::Sm, Some(Msg::CopyLog)),
        k::chip(&p, "Send logs", Ch::Plain, Sz::Sm, Some(Msg::SendLogs)),
    ]
    .spacing(6.0)
    .align_y(iced::Center);
    let mut lines = Column::new().spacing(3.0);
    let text = &a.logs[a.log_tab as usize];
    for (time, rest, kind) in lutris::log_lines(text).into_iter().rev().take(200).collect::<Vec<_>>().into_iter().rev() {
        let c = match kind {
            1 => k::GREEN,
            2 => k::YELLOW,
            3 => k::RED,
            _ => Color::from_rgb8(0xcf, 0xc5, 0xbc),
        };
        lines = lines.push(row![t(time, 10.0, Color::from_rgb8(0x7d, 0x72, 0x6a)).font(MONO).wrapping(Wrapping::None), t(rest, 10.0, c).font(MONO).wrapping(Wrapping::None)].spacing(7.0));
    }
    if text.trim().is_empty() {
        lines = lines.push(t(if a.log_tab == 0 { "Nothing yet: ▶ Play keeps what the game says as it starts." } else { "Nothing yet: installing with a script keeps its log here." }, 11.0, p.muted));
    }
    let box_ = container(scrollable(lines).height(Length::Fill).direction(scrollable::Direction::Both { vertical: scrollable::Scrollbar::default(), horizontal: scrollable::Scrollbar::default() }))
        .padding(k::pad(8.0, 10.0, 8.0, 10.0))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(ui::boxed(p.bg, p.r(9.0), Some((p.line, 1.0))));
    k::card(&p, column![head, gap(9.0), box_])
}

fn page(a: &App) -> E<'_, Msg> {
    let p = a.p;
    if !a.have.ok() {
        let inner: E<'_, Msg> = match a.have {
            Have::Looking => k::mu(&p, "Looking for Lutris…", 13.0),
            _ => column![
                k::h1::<Msg>(&p, "Hearth Game Fixer needs Lutris"),
                k::para(p.muted, need::missing_words(&LUTRIS), 13.0),
                k::chip(&p, if a.getting { "Getting Lutris…" } else { "Get Lutris" }, Ch::Acc, Sz::Md, (!a.getting).then_some(Msg::GetLutris)),
                k::said(&p, &a.said),
            ]
            .spacing(12.0)
            .max_width(560.0)
            .into(),
        };
        return container(inner).padding(40.0).into();
    }
    let Some(g) = a.game().cloned() else {
        let msg = if a.loading { "Asking Lutris for your games…".to_string() } else if !a.problem.is_empty() { a.problem.clone() } else { "Pick a game, or ＋ Install a game.".into() };
        return container(column![k::mu(&p, msg, 13.0), k::said(&p, &a.said)].spacing(10.0)).padding(40.0).into();
    };
    let items: Vec<(&'static str, E<'_, Msg>)> = vec![
        ("hero", hero(a, &g)),
        ("runner", runner_card(a, &g)),
        ("fixes", fixes_card(a, &g)),
        ("script", script_card(a, &g)),
        ("logs", logs_card(a)),
        ("said", k::said(&p, &a.said)),
    ];
    let has_said = !a.said.is_empty();
    hearth_ui::arrange::parts(
        &a.describe,
        &p,
        "game",
        items,
        move |ps| {
            let right = column![ps.take("script"), ps.take("logs")].spacing(12.0).width(Length::FillPortion(5)).height(Length::Fill);
            let cards = row![container(ps.take("runner")).width(Length::FillPortion(4)).height(Length::Fill), container(ps.take("fixes")).width(Length::FillPortion(4)).height(Length::Fill), right].spacing(12.0).height(Length::Fill);
            let mut body = Column::new().push(cards).spacing(8.0).padding(k::pad(12.0, 16.0, 6.0, 16.0)).height(Length::Fill);
            if has_said {
                body = body.push(ps.take("said"));
            }
            column![ps.take("hero"), ui::hline(Pal::mix(p.line, p.bg, 0.2)), body].into()
        },
        Msg::Describe,
    )
}

// ------------------------------------------------------------------ the lists to pick from

fn pop_box(a: &App) -> Option<E<'_, Msg>> {
    let p = a.p;
    let g = a.game();
    let pop = a.pop.clone()?;
    Some(match pop {
        Pop::RunsWith => {
            let cur = g.and_then(|g| g.get(&["wine", "version"])).unwrap_or_default();
            k::chooser(&p, "Runs with", a.wines.iter().map(|w| (lutris::pretty(w), *w == cur, Msg::RunsWith(w.clone()))).collect(), Msg::Close)
        }
        Pop::Resolution => {
            let cur = g.and_then(|g| g.get(&["system", "resolution"])).unwrap_or_default();
            let opts = ["", "1280x720", "1280x800", "1920x1080", "1920x1200", "2560x1440", "3840x2160"];
            k::chooser(&p, "Resolution", opts.iter().map(|o| (if o.is_empty() { "As it is (the screen's own)".to_string() } else { o.replace('x', " × ") }, *o == cur, Msg::Resolution(o.to_string()))).collect(), Msg::Close)
        }
        Pop::Prelaunch | Pop::Extra => {
            let (title, hint, save) = if pop == Pop::Prelaunch {
                ("Run before the game", "a command or script, like set-hd-mode.sh", Msg::SavePrelaunch)
            } else {
                ("Extra settings", "NAME=value; OTHER=value", Msg::SaveExtra)
            };
            let body = column![
                k::h2::<Msg>(&p, title),
                k::mu(&p, if pop == Pop::Prelaunch { "Lutris runs it each time just before the game starts." } else { "Settings the game is started with (environment). Separate them with ;" }, 12.0),
                k::input(&p, hint, &a.text, Msg::Text).on_submit(save.clone()),
                row![k::fill(), k::chip(&p, "Cancel", Ch::Plain, Sz::Md, Some(Msg::Close)), k::chip(&p, "Save", Ch::Acc, Sz::Md, Some(save))].spacing(8.0),
            ]
            .spacing(10.0);
            ui::dialog_box(&p, 460.0, body)
        }
        Pop::More => k::chooser(
            &p,
            g.map(|g| g.name.clone()).unwrap_or_default(),
            vec![("Show it in Lutris' own window".into(), false, Msg::ShowInLutris), ("Copy the game folder's path".into(), false, Msg::CopyPath), ("Look for games again".into(), false, Msg::Reload)],
            Msg::Close,
        ),
        Pop::Install => {
            let mut c = Column::new().spacing(8.0).push(row![k::h2::<Msg>(&p, "Install a game"), k::fill(), k::chip(&p, "Close", Ch::Plain, Sz::Sm, Some(Msg::Close))].align_y(iced::Center));
            c = c.push(k::mu(&p, "The community's install scripts from lutris.net: each sets up the runner, the fixes and the extras a game needs. Lutris asks for your own disc images or installer in its window.", 12.0));
            c = c.push(k::input(&p, "⌕ A game's name", &a.search, Msg::Search).on_submit(Msg::SearchGo));
            let mut res = Column::new().spacing(2.0);
            for r in a.results.iter().take(12) {
                let slug = r.get("slug").and_then(Value::as_str).unwrap_or("").to_string();
                let name = r.get("name").and_then(Value::as_str).unwrap_or("").to_string();
                let year = r.get("year").and_then(Value::as_i64).map(|y| y.to_string()).unwrap_or_default();
                let on = slug == a.picked_result;
                res = res.push(k::click(container(row![t(name, 13.0, p.text), k::fill(), k::mu1::<Msg>(&p, year, 11.0)]).padding(k::pad(6.0, 8.0, 6.0, 8.0)).width(Length::Fill).style(ui::boxed(if on { p.sel() } else { Color::TRANSPARENT }, p.r(6.0), None)), Some(Msg::PickResult(slug))));
            }
            c = c.push(scrollable(res).height(Length::Fixed(170.0)));
            if !a.scripts.is_empty() {
                c = c.push(k::cap(&p, "Its scripts"));
                for (i, s) in a.scripts.iter().enumerate().take(6) {
                    let v = s.get("version").and_then(Value::as_str).unwrap_or("").to_string();
                    let who = s.get("user").and_then(Value::as_str).unwrap_or("").to_string();
                    let runner = lutris::runner_name(s.get("runner").and_then(Value::as_str).unwrap_or(""));
                    let day = k::day_words(s.get("updated_at").and_then(Value::as_str).unwrap_or(""));
                    c = c.push(row![column![t(v, 13.0, p.text), k::mu1::<Msg>(&p, format!("{runner} · by {who} · updated {day}"), 11.0)].width(Length::Fill), k::chip(&p, "Install", Ch::Acc, Sz::Sm, a.running.is_none().then_some(Msg::InstallScript(i)))].align_y(iced::Center));
                }
            }
            ui::dialog_box(&p, 560.0, c)
        }
    })
}

// ------------------------------------------------------------------ the window

fn view(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    studio_values(a);
    let mut items: Vec<(&'static str, E<'_, Msg>)> = vec![("side", hearth_ui::studio::made(&a.describe, "window", "side", || side(a))), ("page", page(a))];
    for (id, which) in [("pane:games", "games"), ("pane:log", "log"), ("pane:fixes", "fixes")] {
        if a.describe.wants_pane(which) {
            items.push((id, studio_pane(a, which)));
        }
    }
    let mut body = hearth_ui::arrange::parts(&a.describe, &p, "window", items, move |ps| row![ps.take("side"), ui::vline(p.line), ps.take("page")].into(), Msg::Describe);
    if let Some(d) = pop_box(a) {
        body = ui::dialog(body, d, Some(Msg::Close));
    }
    let bar = k::Bar {
        letters: "GF",
        from: k::rgb("#ff9a8a"),
        to: k::rgb("#d8413a"),
        title: "Hearth Game Fixer",
        line: "· any Windows game, with the community's fixes".into(),
        middle: None,
        right: vec![k::chip(&p, "＋ Install a game", Ch::Acc, Sz::Sm, Some(Msg::Pop(Pop::Install)))],
        msgs: TitleMsgs { drag: Msg::Drag, maximize: Msg::Max, minimize: Some(Msg::Min), close: Msg::WClose },
        open: Msg::Describe(hearth_ui::describe::DMsg::Open),
    };
    k::window(&p, &a.describe, bar, body, ("Hearth Game Fixer", "Built on Lutris (GPL-3)"), Msg::Resize, Msg::Describe)
}

/// Hearth Game Fixer's blocks (hearth_ui::blocks): the window, the side, a game's page.
use hearth_ui::blocks::{bl, bls, PageDef};
const BLOCKS: &[PageDef] = &[
    PageDef { id: "window", title: "Hearth Game Fixer", blocks: &[bl("side", "Your games", 0, 0, 2, 12), bl("page", "The game", 2, 0, 10, 12)] },
    PageDef { id: "side", title: "The side", blocks: &[bl("search", "Search games and scripts", 0, 0, 12, 1), bls("games", "Your games", 0, 1, 12, 8), bl("scripts", "Community scripts", 0, 9, 12, 2), bl("edit", "Edit layout", 0, 11, 12, 1)] },
    PageDef {
        id: "game",
        title: "A game",
        blocks: &[bl("hero", "The game's banner and buttons", 0, 0, 12, 3), bl("runner", "Runner", 0, 3, 4, 9), bls("fixes", "Fixes", 4, 3, 4, 9), bl("script", "Install script", 8, 3, 4, 4), bl("logs", "Logs", 8, 7, 4, 5), bl("said", "What happened", 0, 11, 12, 1)],
    },
];

use hearth_ui::studio::{tile, Kit};
/// Hearth Game Fixer's blocks window: its buttons, its panes, its info.
const KIT: Kit = Kit {
    name: "Game Fixer",
    buttons: &[
        tile("play", "Play", G::Play),
        tile("folder", "Open folder", G::Folder),
        tile("shortcut", "Make a desktop shortcut", G::Link),
        tile("install", "Install a game", G::Plus),
        tile("browse", "Browse scripts", G::Store),
        tile("copy", "Copy the log", G::Doc),
        tile("send", "Send logs", G::Rocket),
        tile("reload", "Look for games again", G::Sync),
        tile("proton", "Proton and Wine", G::Gear),
    ],
    panes: &[tile("games", "Your games", G::List), tile("log", "The last start's log", G::Doc), tile("fixes", "The game's fixes", G::Cog)],
    info: &[tile("games", "Games", G::Grid), tile("ready", "Ready to play", G::Check), tile("fixes", "Fixes on", G::Cog), tile("runner", "Runs with", G::Play)],
    pane_page: "window",
    bars: &[],
    menus: &[],
};

fn block_action(id: &str) -> Option<Msg> {
    Some(match id {
        "play" => Msg::Play,
        "folder" => Msg::OpenFolder,
        "shortcut" => Msg::Shortcut,
        "install" | "browse" => Msg::Pop(Pop::Install),
        "copy" => Msg::CopyLog,
        "send" => Msg::SendLogs,
        "reload" => Msg::Reload,
        "proton" => Msg::OpenProton,
        _ => return None,
    })
}

fn studio_values(a: &App) {
    let ready = a.games.iter().filter(|g| g.state().1 == 0).count();
    let (fixes, runner) = a.game().map(|g| (lutris::fixes_of(g).iter().filter(|f| lutris::fix_on(g, f)).count().to_string(), g.runs_with())).unwrap_or_default();
    a.describe.values(vec![("games", a.games.len().to_string()), ("ready", ready.to_string()), ("fixes", fixes), ("runner", runner)]);
}

/// One of Hearth Game Fixer's own panes.
fn studio_pane<'a>(a: &'a App, which: &str) -> E<'a, Msg> {
    let p = a.p;
    let mut c = Column::new().spacing(5.0);
    match which {
        "games" => {
            for g in &a.games {
                c = c.push(k::click(row![thumb(g, 16.0), t(ui::fit(&g.name, 30), 12.0, p.text), k::fill(), k::mu1::<Msg>(&p, g.state().0, 10.0)].spacing(8.0).align_y(iced::Center), Some(Msg::Pick(g.slug.clone()))));
            }
        }
        "log" => {
            for (time, rest, _) in lutris::log_lines(&a.logs[0]).into_iter().rev().take(12).collect::<Vec<_>>().into_iter().rev() {
                c = c.push(t(format!("{time} {rest}"), 10.0, p.text).font(MONO).wrapping(Wrapping::None));
            }
        }
        _ => {
            if let Some(g) = a.game() {
                for f in lutris::fixes_of(g) {
                    let on = lutris::fix_on(g, &f);
                    c = c.push(row![t(f.name.clone(), 12.0, p.text), k::fill(), k::switch(&p, on, Some(Msg::Fix(f.id.clone(), !on)))].align_y(iced::Center));
                }
            }
        }
    }
    scrollable(c).height(Length::Fill).into()
}

fn subscription(_a: &App) -> Subscription<Msg> {
    Subscription::run(hearth_ui::theme::watch).map(|_| Msg::Theme)
}

fn main() -> iced::Result {
    if app::handle_cli("hearth-game-fixer", "Hearth Game Fixer: any Windows game, with the community's fixes (built on Lutris).\n  --game SLUG      open that game\n  --show install   open Install a game") {
        return Ok(());
    }
    let w = app::window("hearth-game-fixer", (1280.0, 800.0), (960.0, 600.0));
    iced::application(App::new, update, view).title(|_: &App| "Hearth Game Fixer".to_string()).subscription(subscription).theme(|a: &App| app::iced_theme(&a.p)).window(w).default_font(app::INTER).run()
}
