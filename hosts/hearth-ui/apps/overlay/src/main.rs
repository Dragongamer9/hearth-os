//! hearth-overlay: Hearth Overlay's settings (docs/mockups/checklists/open-source-03-overlay.md; the approved board
//! docs/mockups/open-source-apps/03-overlay.png): the numbers over your games (frame rate, GPU, CPU, memory…), for
//! all games or one, where they sit and how they look, the frame-rate limit, the keys that show and hide them, saving
//! them to a file, and which level of Game Mode's Quick menu shows them. Built on MangoHud (MIT), which draws them in
//! the game, and GOverlay (GPL-3), whose settings these are: everything is written to MangoHud's own files
//! (conf.rs), which MangoHud reads again as they change, so changes show live in the game.
//!
//! Made of blocks: the games' tabs, Start from, Shows, Where, Look, Frame rate, In Game Mode and the buttons are each a
//! block (Edit layout), with the app's own buttons, panes and info in its blocks window.

mod conf;

use hearth_oss_kit::need::{self, Have, Need};
use hearth_oss_kit::{self as k, Ch, Sz, E};
use hearth_ui::app::{self, win};
use hearth_ui::run;
use hearth_ui::ui::{self, t, tb, TitleMsgs, G};
use hearth_ui::Pal;
use iced::widget::text::Wrapping;
use iced::widget::{column, container, row, scrollable, Column, Row, Space};
use iced::{Color, Element, Length, Subscription, Task};
use serde_json::{json, Value};
use std::path::PathBuf;

const APP: &str = "overlay";
const MANGOHUD: Need = Need { name: "MangoHud", cmd: "mangohud", flatpak: "org.freedesktop.Platform.VulkanLayer.MangoHud", flatpak_cmd: "", licence: "MIT" };

/// What the overlay can show, and MangoHud's options for each (the first one says whether it's on).
const SHOWS: [(&str, &[&str]); 11] = [
    ("Frame rate and frame time", &["fps", "frametime"]),
    ("Frame-time graph", &["frame_timing"]),
    ("GPU use, temperature, power", &["gpu_stats", "gpu_temp", "gpu_power"]),
    ("CPU use, temperature, clock", &["cpu_stats", "cpu_temp", "cpu_mhz"]),
    ("Video and system memory", &["vram", "ram"]),
    ("Game name and runner", &["exec_name", "wine", "engine_version"]),
    ("Each CPU core", &["core_load"]),
    ("Fan speeds", &["fan"]),
    ("Battery and time left", &["battery", "battery_time"]),
    ("Clock", &["time"]),
    ("Network up and down", &["network"]),
];
/// MangoHud's places, as the board's 3 × 3 grid (the middle has none: it would cover the game).
const PLACES: [(&str, &str); 9] = [("top-left", "Top left"), ("top-center", "Top middle"), ("top-right", "Top right"), ("middle-left", "Middle left"), ("", ""), ("middle-right", "Middle right"), ("bottom-left", "Bottom left"), ("bottom-center", "Bottom middle"), ("bottom-right", "Bottom right")];
/// The colours you can give the rows (the board's four: GPU, CPU, memory, frame rate).
const COLOUR_KEYS: [(&str, &str, &str); 4] = [("gpu_color", "GPU", "9be36b"), ("cpu_color", "CPU", "5ec8ff"), ("frametime_color", "Frame rate", "ff8a3d"), ("vram_color", "Memory", "e36bd3")];
const PALETTE: [&str; 8] = ["9be36b", "5ec8ff", "ff8a3d", "e36bd3", "ffd36b", "ff6b6b", "f1e9e2", "b9a2ff"];
const LIMITS: [u32; 6] = [0, 30, 40, 60, 120, 144];
const KEYS: [(&str, &str); 5] = [("Shift_R+F12", "Shift + F12"), ("Shift_L+F12", "Left Shift + F12"), ("Shift_R+F11", "Shift + F11"), ("Control_L+F12", "Ctrl + F12"), ("F12", "F12")];
const START: [&str; 6] = ["Just FPS", "Small", "Full", "Battery (Ally)", "Stream-safe", "My own…"];
const LEVELS: [&str; 5] = ["0 Off", "1 FPS", "2 Small", "3 Full", "4 Everything"];

#[derive(Debug, Clone, PartialEq)]
enum Pop {
    Colour(usize),
    Warn,
    Keys,
    Limit,
    AddGame,
}

#[derive(Debug, Clone)]
enum Msg {
    Describe(hearth_ui::describe::DMsg),
    Theme,
    Have(Have),
    Tab(Option<String>),
    OnOff,
    Start(usize),
    Show(usize),
    Place(usize),
    Size(f32),
    See(f32),
    Saved,
    OneLine(bool),
    Pop(Pop),
    Close,
    Colour(usize, &'static str),
    Warn(u32),
    Limit(u32),
    Text(String),
    OwnLimit,
    Pacing,
    Key(&'static str),
    Log,
    Level(u8),
    AddGame(String),
    Reset,
    CopyAll,
    Done,
    GetIt,
    GotIt(run::Output),
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
    tab: Option<String>,
    /// the file being changed, as it is now
    text: String,
    files: Vec<String>,
    set: Value,
    pop: Option<Pop>,
    input: String,
    picked_colour: usize,
    size: f32,
    see: f32,
    said: String,
    getting: bool,
}

fn obj(v: Value) -> serde_json::Map<String, Value> {
    v.as_object().cloned().unwrap_or_default()
}

fn env_file() -> PathBuf {
    let h = k::home();
    std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| h.join(".config")).join("environment.d/90-hearth-overlay.conf")
}

/// MangoHud's settings for one of Start from's choices.
fn preset(i: usize) -> (Vec<usize>, Vec<(&'static str, Option<&'static str>)>) {
    match i {
        0 => (vec![0], vec![("font_size", Some("16")), ("horizontal", None)]),
        1 => (vec![0, 2, 3], vec![("horizontal", None)]),
        2 => (vec![0, 1, 2, 3, 4, 5], vec![("horizontal", None)]),
        3 => (vec![0, 3, 8], vec![("fps_limit", Some("40")), ("horizontal", None)]),
        _ => (vec![0, 2, 3], vec![("horizontal", Some("")), ("position", Some("top-right")), ("font_size", Some("12"))]),
    }
}

impl App {
    fn new() -> (App, Task<Msg>) {
        let set = k::read_settings(APP);
        let mut a = App {
            describe: hearth_ui::describe::Describe::new(APP, "Hearth Overlay").with_pages(BLOCKS).with_kit(&KIT),
            p: hearth_ui::theme::load(),
            have: Have::Looking,
            tab: app::arg("--game"),
            text: String::new(),
            files: Vec::new(),
            set,
            pop: None,
            input: String::new(),
            picked_colour: 2,
            size: 14.0,
            see: 62.0,
            said: String::new(),
            getting: false,
        };
        a.load();
        (a, Task::perform(need::probe(&MANGOHUD), Msg::Have))
    }
    fn path(&self) -> PathBuf {
        conf::dir().join(match &self.tab {
            Some(f) => f.clone(),
            None => "MangoHud.conf".into(),
        })
    }
    fn load(&mut self) {
        let mut files: Vec<String> = std::fs::read_dir(conf::dir()).map(|r| r.flatten().map(|e| e.file_name().to_string_lossy().to_string()).filter(|f| conf::program_of(f).is_some()).collect()).unwrap_or_default();
        files.sort();
        self.files = files;
        if self.tab.as_ref().map(|t| !self.files.contains(t)).unwrap_or(false) {
            // --game NAME: its file, or the program's
            let f = conf::file_for(self.tab.as_deref().unwrap_or(""));
            self.tab = self.files.contains(&f).then_some(f);
        }
        self.text = std::fs::read_to_string(self.path()).unwrap_or_default();
        self.size = conf::get(&self.text, "font_size").and_then(|v| v.parse().ok()).unwrap_or(24.0);
        self.see = conf::get(&self.text, "background_alpha").and_then(|v| v.parse::<f32>().ok()).map(|a| ((1.0 - a) * 100.0).round()).unwrap_or(50.0);
    }
    fn name_of(&self, file: &str) -> String {
        self.set.pointer(&format!("/names/{}", file.replace('/', "~1"))).and_then(Value::as_str).map(str::to_string).unwrap_or_else(|| conf::program_of(file).unwrap_or_default())
    }
    /// The file changed: one line at a time, atomically with a .bak (red flag 9). MangoHud sees it at once.
    fn change(&mut self, f: impl FnOnce(&str) -> String) {
        let p = self.path();
        let _ = std::fs::create_dir_all(conf::dir());
        let r = if p.exists() {
            k::settings_file::update_text(&p, f)
        } else {
            let body = f(&format!("# Hearth Overlay (MangoHud's settings{})\nlegacy_layout=false\n", if self.tab.is_some() { " for this game" } else { "" }));
            k::settings_file::write_text(&p, &body).map(|_| body)
        };
        if let Err(e) = r {
            self.said = format!("Couldn't save that: {e}");
        }
        self.load();
    }
    fn show_on(&self, i: usize) -> bool {
        conf::on(&self.text, SHOWS[i].1[0])
    }
    fn which_start(&self) -> Option<usize> {
        (0..5).find(|&i| {
            let (shows, _) = preset(i);
            (0..SHOWS.len()).all(|s| self.show_on(s) == shows.contains(&s)) && (i != 4 || conf::on(&self.text, "horizontal")) && (i == 4 || !conf::on(&self.text, "horizontal")) && (i != 3 || conf::get(&self.text, "fps_limit").as_deref() == Some("40"))
        })
    }
    fn save(&mut self, keys: Value) {
        if let Err(e) = k::save_settings(APP, obj(keys)) {
            self.said = format!("Couldn't save that: {e}");
        }
        self.set = k::read_settings(APP);
    }
    fn level(&self) -> u8 {
        self.set.get("gameModeLevel").and_then(Value::as_u64).unwrap_or(3).min(4) as u8
    }
    /// Game Mode's level (Steam's Quick menu › Performance) gets these settings: presets.conf's [preset N].
    fn write_level(&mut self) {
        let n = self.level();
        if n == 0 {
            return;
        }
        let body: String = self.text.lines().filter(|l| !l.trim().starts_with('#') && !l.trim().is_empty()).collect::<Vec<_>>().join("\n");
        let p = conf::dir().join("presets.conf");
        let r = if p.exists() { k::settings_file::update_text(&p, |t| conf::set_preset(t, n, &body)).map(|_| ()) } else { k::settings_file::write_text(&p, &conf::set_preset("", n, &body)).map(|_| ()) };
        if let Err(e) = r {
            self.said = format!("Couldn't set Game Mode's level: {e}");
        }
    }
}

/// Programs running now that look like games (for ＋ Game): Windows programs and anything MangoHud is in.
fn running_programs() -> Vec<String> {
    let mut v: Vec<String> = Vec::new();
    if let Ok(rd) = std::fs::read_dir("/proc") {
        for e in rd.flatten() {
            if !e.file_name().to_string_lossy().chars().all(|c| c.is_ascii_digit()) {
                continue;
            }
            let Ok(c) = std::fs::read(e.path().join("cmdline")) else { continue };
            let first = c.split(|b| *b == 0).next().map(|b| String::from_utf8_lossy(b).to_string()).unwrap_or_default();
            let name = first.rsplit(['/', '\\']).next().unwrap_or("").to_string();
            if name.to_lowercase().ends_with(".exe") && !["wineserver", "explorer.exe", "services.exe", "winedevice.exe", "plugplay.exe", "rpcss.exe", "svchost.exe", "steam.exe", "conhost.exe", "start.exe", "tabtip.exe"].contains(&name.to_lowercase().as_str()) && !v.contains(&name) {
                v.push(name);
            }
        }
    }
    v
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
        Msg::Tab(t) => {
            a.tab = t;
            a.said.clear();
            a.load();
        }
        Msg::OnOff => {
            if a.tab.is_some() {
                // one game: shown or not when it starts (MangoHud's no_display; the keys still bring it back)
                let off = conf::on(&a.text, "no_display");
                a.change(|t| conf::set(t, "no_display", if off { None } else { Some("") }));
            } else {
                // every game: MangoHud loaded in each (MANGOHUD=1 for your session, from your next sign-in)
                let f = env_file();
                if f.exists() {
                    let _ = std::fs::remove_file(&f);
                    a.said = "From your next sign-in, games start without the overlay unless one is set to show it.".into();
                } else {
                    let _ = std::fs::create_dir_all(f.parent().unwrap_or(&f));
                    match k::settings_file::write_text(&f, "# Hearth Overlay: MangoHud in every game (remove this file to turn it off)\nMANGOHUD=1\n") {
                        Ok(_) => a.said = "The overlay is in every game from your next sign-in (games you start through Hearth or Steam have it now).".into(),
                        Err(e) => a.said = format!("Couldn't turn it on: {e}"),
                    }
                }
            }
        }
        Msg::Start(i) => {
            if i == 5 {
                // My own…: what you made, kept; the first press keeps it
                match a.set.get("own").and_then(Value::as_str).map(str::to_string) {
                    Some(own) if !own.is_empty() && a.which_start().is_some() => {
                        a.change(move |_| own);
                    }
                    _ => {
                        let body = a.text.clone();
                        a.save(json!({"own": body}));
                        a.said = "Kept as your own: My own… brings it back.".into();
                    }
                }
            } else {
                let (shows, extra) = preset(i);
                a.change(|t| {
                    let mut t = t.to_string();
                    for (s, (_, keys)) in SHOWS.iter().enumerate() {
                        for k_ in keys.iter() {
                            t = conf::set(&t, k_, Some(if shows.contains(&s) { "" } else { "0" }));
                        }
                    }
                    for (k_, v) in extra {
                        t = conf::set(&t, k_, v);
                    }
                    t
                });
            }
            a.write_level();
        }
        Msg::Show(i) => {
            let on = !a.show_on(i);
            a.change(|t| {
                let mut t = t.to_string();
                for k_ in SHOWS[i].1 {
                    t = conf::set(&t, k_, Some(if on { "" } else { "0" }));
                }
                t
            });
            a.write_level();
        }
        Msg::Place(i) => {
            let pl = PLACES[i].0;
            if !pl.is_empty() {
                a.change(|t| conf::set(t, "position", Some(pl)));
                a.write_level();
            }
        }
        Msg::Size(v) => a.size = v,
        Msg::See(v) => a.see = v,
        Msg::Saved => {
            let fs = format!("{}", a.size.round() as u32);
            let al = format!("{:.2}", 1.0 - a.see / 100.0);
            a.change(|t| conf::set(&conf::set(t, "font_size", Some(fs.as_str())), "background_alpha", Some(al.as_str())));
            a.write_level();
        }
        Msg::OneLine(on) => {
            a.change(|t| conf::set(t, "horizontal", if on { Some("") } else { None }));
            a.write_level();
        }
        Msg::Pop(p) => {
            if let Pop::Colour(i) = p {
                a.picked_colour = i;
            }
            a.input.clear();
            a.pop = Some(p);
        }
        Msg::Close => a.pop = None,
        Msg::Colour(i, c) => {
            a.pop = None;
            let key = COLOUR_KEYS[i].0;
            a.change(|t| conf::set(t, key, Some(c)));
            a.write_level();
        }
        Msg::Warn(v) => {
            a.pop = None;
            a.save(json!({"warnGpu": v}));
        }
        Msg::Limit(v) => {
            a.pop = None;
            let vs = v.to_string();
            a.change(|t| conf::set(t, "fps_limit", if v == 0 { None } else { Some(vs.as_str()) }));
            a.write_level();
        }
        Msg::Text(s) => a.input = s,
        Msg::OwnLimit => {
            if let Ok(v) = a.input.trim().parse::<u32>() {
                return update(a, Msg::Limit(v.min(1000)));
            }
        }
        Msg::Pacing => {
            let early = conf::get(&a.text, "fps_limit_method").as_deref() == Some("early");
            a.change(|t| conf::set(t, "fps_limit_method", Some(if early { "late" } else { "early" })));
        }
        Msg::Key(kk) => {
            a.pop = None;
            a.change(|t| conf::set(t, "toggle_hud", Some(kk)));
        }
        Msg::Log => {
            let on = conf::on(&a.text, "autostart_log");
            let folder = k::home().join("Hearth/Benchmarks");
            let _ = std::fs::create_dir_all(&folder);
            let fs = folder.to_string_lossy().to_string();
            a.change(|t| conf::set(&conf::set(t, "output_folder", Some(fs.as_str())), "autostart_log", if on { None } else { Some("1") }));
        }
        Msg::Level(n) => {
            a.save(json!({"gameModeLevel": n}));
            a.write_level();
            a.said = if n == 0 { "Level 0 is Off in Game Mode.".into() } else { format!("Game Mode's level {n} shows these settings.") };
        }
        Msg::AddGame(prog) => {
            a.pop = None;
            let prog = prog.trim().to_string();
            if prog.is_empty() {
                return Task::none();
            }
            let f = conf::file_for(&prog);
            let p = conf::dir().join(&f);
            if !p.exists() {
                let _ = std::fs::create_dir_all(conf::dir());
                let base = std::fs::read_to_string(conf::dir().join("MangoHud.conf")).unwrap_or_else(|_| "legacy_layout=false\n".into());
                let _ = k::settings_file::write_text(&p, &format!("# Hearth Overlay: {prog}\n{}", base.lines().filter(|l| !l.starts_with('#')).collect::<Vec<_>>().join("\n") + "\n"));
            }
            let mut names = a.set.get("names").cloned().filter(Value::is_object).unwrap_or(json!({}));
            if names.get(&f).is_none() {
                names[&f] = json!(prog.trim_end_matches(".exe"));
                a.save(json!({"names": names}));
            }
            a.tab = Some(f);
            a.load();
        }
        Msg::Reset => {
            if let Some(f) = a.tab.clone() {
                let p = conf::dir().join(&f);
                // the game goes back to All games' settings (its own file kept aside as .bak)
                let _ = std::fs::rename(&p, k::settings_file::bak_path(&p));
                a.tab = None;
                a.said = format!("{} uses All games' settings again.", a.name_of(&f));
                a.load();
            } else {
                return update(a, Msg::Start(2));
            }
        }
        Msg::CopyAll => {
            let body = a.text.clone();
            let p = conf::dir().join("MangoHud.conf");
            let _ = std::fs::create_dir_all(conf::dir());
            match k::settings_file::write_text(&p, &body) {
                Ok(_) => a.said = "Every game uses these settings now.".into(),
                Err(e) => a.said = format!("Couldn't copy them: {e}"),
            }
            a.load();
        }
        Msg::Done => return win::close(),
        Msg::GetIt => {
            a.getting = true;
            return Task::perform(run::spawn(need::install_argv(&MANGOHUD)), Msg::GotIt);
        }
        Msg::GotIt(o) => {
            a.getting = false;
            a.said = if o.ok { "MangoHud is here for your Flatpak games.".into() } else { format!("Couldn't get MangoHud: {}", o.why()) };
            return Task::perform(need::probe(&MANGOHUD), Msg::Have);
        }
        Msg::Drag => return win::drag(),
        Msg::Max => return win::toggle_maximize(),
        Msg::Min => return win::minimize(),
        Msg::WClose => return win::close(),
        Msg::Resize(d) => return win::resize(d),
    }
    Task::none()
}

// ------------------------------------------------------------------ the window

fn cap<'a>(p: &Pal, s: &str) -> E<'a, Msg> {
    container(k::cap::<Msg>(p, s)).padding(k::pad(12.0, 0.0, 6.0, 0.0)).into()
}

fn view_body(a: &App) -> E<'_, Msg> {
    let p = a.p;
    // the games' tabs and On
    let mut tabs: Vec<(String, bool, Msg)> = vec![("All games".into(), a.tab.is_none(), Msg::Tab(None))];
    for f in &a.files {
        tabs.push((a.name_of(f), a.tab.as_deref() == Some(f.as_str()), Msg::Tab(Some(f.clone()))));
    }
    tabs.push(("＋ Game".into(), false, Msg::Pop(Pop::AddGame)));
    let on = if a.tab.is_some() { !conf::on(&a.text, "no_display") } else { env_file().exists() };
    let head = row![k::tabs(&p, tabs), k::fill(), t("On", 12.0, p.text), k::switch(&p, on, Some(Msg::OnOff))].spacing(8.0).align_y(iced::Center);

    let cur = a.which_start();
    let start = column![cap(&p, "Start from"), Row::with_children(START.iter().enumerate().map(|(i, s)| k::chip(&p, *s, if cur == Some(i) { Ch::Acc } else { Ch::Plain }, Sz::Sm, Some(Msg::Start(i))))).spacing(6.0)];

    let mut shows = Column::new().spacing(2.0).push(cap(&p, "Shows"));
    for (i, (label, _)) in SHOWS.iter().enumerate() {
        shows = shows.push(k::check(&p, a.show_on(i), *label, Msg::Show(i)));
    }

    let pos = conf::get(&a.text, "position").unwrap_or_else(|| "top-left".into());
    let mut grid = Column::new().spacing(4.0);
    for r in 0..3 {
        let mut rr = Row::new().spacing(4.0);
        for c in 0..3 {
            let i = r * 3 + c;
            let lit = PLACES[i].0 == pos;
            let cell = container(Space::new()).width(Length::Fixed(30.0)).height(Length::Fixed(20.0)).style(ui::boxed(if lit { p.accent } else { p.bg }, 5.0, Some((if lit { p.accent } else { p.line }, 1.0))));
            rr = rr.push(k::click(cell, (!PLACES[i].0.is_empty()).then_some(Msg::Place(i))));
        }
        grid = grid.push(rr);
    }
    let pos_name = PLACES.iter().find(|x| x.0 == pos).map(|x| x.1).unwrap_or("Top left");
    let where_ = column![cap(&p, "Where"), row![grid, k::para(p.muted, format!("{pos_name}\nDrag it in the game\nto place it anywhere"), 11.0)].spacing(14.0)];

    let one = conf::on(&a.text, "horizontal");
    let mut dots = Row::new().spacing(5.0);
    for (i, (key, _, d)) in COLOUR_KEYS.iter().enumerate() {
        let c = conf::get(&a.text, key).filter(|v| v.len() == 6).unwrap_or_else(|| d.to_string());
        let ring = if i == a.picked_colour { Color::WHITE } else { Color::TRANSPARENT };
        dots = dots.push(k::click(container(Space::new()).width(Length::Fixed(17.0)).height(Length::Fixed(17.0)).style(ui::boxed(k::rgb(&format!("#{c}")), 4.0, Some((ring, 1.5)))), Some(Msg::Pop(Pop::Colour(i)))));
    }
    let warn = a.set.get("warnGpu").and_then(Value::as_u64).unwrap_or(83);
    let look = column![
        cap(&p, "Look"),
        k::kv(&p, "Size", row![k::slider(&p, 8.0..=32.0, a.size, 110.0, Msg::Size, Some(Msg::Saved)), container(k::mu1::<Msg>(&p, format!("{} px", a.size.round()), 11.0)).width(Length::Fixed(44.0)).align_x(iced::alignment::Horizontal::Right)].spacing(8.0).align_y(iced::Center)),
        k::kv(&p, "See-through", row![k::slider(&p, 0.0..=90.0, a.see, 110.0, Msg::See, Some(Msg::Saved)), container(k::mu1::<Msg>(&p, format!("{} %", a.see.round()), 11.0)).width(Length::Fixed(44.0)).align_x(iced::alignment::Horizontal::Right)].spacing(8.0).align_y(iced::Center)),
        k::kv(&p, "Layout", k::seg(&p, vec![("Rows".into(), !one, Msg::OneLine(false)), ("One line".into(), one, Msg::OneLine(true))])),
        k::kv(&p, "Colours", dots),
        k::kv(&p, "Warn when hot", k::field(&p, format!("GPU {warn} °C"), 0.0, Some(Msg::Pop(Pop::Warn)))),
    ];

    let limit: u32 = conf::get(&a.text, "fps_limit").and_then(|v| v.split(',').next().and_then(|x| x.trim().parse().ok())).unwrap_or(0);
    let mut lim: Vec<(String, bool, Msg)> = LIMITS.iter().map(|l| (if *l == 0 { "Off".to_string() } else { l.to_string() }, *l == limit, Msg::Limit(*l))).collect();
    let own = !LIMITS.contains(&limit);
    lim.push((if own { format!("{limit}") } else { "Own…".into() }, own, Msg::Pop(Pop::Limit)));
    let key = conf::get(&a.text, "toggle_hud").unwrap_or_else(|| "Shift_R+F12".into());
    let key_name = KEYS.iter().find(|x| x.0 == key).map(|x| x.1.to_string()).unwrap_or(key.replace('_', " ").replace('+', " + "));
    let folder = conf::get(&a.text, "output_folder").map(|f| k::tilde(std::path::Path::new(&f))).unwrap_or_else(|| "~/Hearth/Benchmarks".into());
    let frame = column![
        cap(&p, "Frame rate"),
        k::kv(&p, "Limit", k::seg(&p, lim)),
        k::kv(&p, "Smoother frame pacing", k::switch(&p, conf::get(&a.text, "fps_limit_method").as_deref() == Some("early"), Some(Msg::Pacing))),
        k::kv(&p, "Show and hide", row![k::field(&p, key_name, 0.0, Some(Msg::Pop(Pop::Keys))), k::mu1::<Msg>(&p, "or", 11.0), k::field::<Msg>(&p, "Controller: ≡ + R3", 0.0, None)].spacing(6.0).align_y(iced::Center)),
        k::kv(&p, "Save the numbers to a file", row![k::mu1::<Msg>(&p, folder, 11.0), k::switch(&p, conf::on(&a.text, "autostart_log"), Some(Msg::Log))].spacing(8.0).align_y(iced::Center)),
    ];

    let lv = a.level();
    let gm = k::card_fit(
        &p,
        column![
            row![tb("In Game Mode", 12.0, p.text), k::fill(), k::mu1::<Msg>(&p, "Quick menu › Performance › Overlay level 0–4", 11.0)].align_y(iced::Center),
            Row::with_children(LEVELS.iter().enumerate().map(|(i, s)| k::chip(&p, *s, if lv as usize == i { Ch::Acc } else { Ch::Plain }, Sz::Sm, Some(Msg::Level(i as u8))))).spacing(6.0),
        ]
        .spacing(9.0),
    );
    let foot = row![
        k::chip(&p, "Reset this game", Ch::Plain, Sz::Md, Some(Msg::Reset)),
        k::chip(&p, "Copy to all games", Ch::Plain, Sz::Md, a.tab.is_some().then_some(Msg::CopyAll)),
        k::fill(),
        k::chip(&p, "Done", Ch::Acc, Sz::Md, Some(Msg::Done)),
    ]
    .spacing(8.0);

    let need: E<'_, Msg> = match &a.have {
        Have::Missing => k::note_card(
            &p,
            row![k::para(p.text, format!("{} Games from Flathub need it from there; Hearth OS brings it for the rest.", need::missing_words(&MANGOHUD)), 11.0), k::chip(&p, if a.getting { "Getting it…" } else { "Get MangoHud" }, Ch::Acc, Sz::Sm, (!a.getting).then_some(Msg::GetIt))].spacing(10.0).align_y(iced::Center),
            k::rgb("#2a2017"),
            k::rgb("#5a3b22"),
            false,
        ),
        _ => k::gap(1.0, 1.0),
    };
    let has_need = a.have == Have::Missing;
    let has_said = !a.said.is_empty();
    hearth_ui::arrange::parts(
        &a.describe,
        &p,
        "settings",
        vec![("tabs", head.into()), ("start", start.into()), ("shows", shows.into()), ("where", where_.into()), ("look", look.into()), ("frame", frame.into()), ("gamemode", gm), ("buttons", foot.into()), ("need", need), ("said", k::said(&p, &a.said))],
        move |ps| {
            let mut c = Column::new();
            if has_need {
                c = c.push(ps.take("need")).push(k::gap(1.0, 8.0));
            }
            c = c.push(ps.take("tabs")).push(ps.take("start")).push(row![container(ps.take("shows")).width(Length::FillPortion(1)), column![ps.take("where"), ps.take("look")].width(Length::FillPortion(1))].spacing(18.0));
            c = c.push(ps.take("frame")).push(k::gap(1.0, 10.0)).push(ps.take("gamemode")).push(k::gap(1.0, 12.0)).push(ps.take("buttons"));
            if has_said {
                c = c.push(k::gap(1.0, 8.0)).push(ps.take("said"));
            }
            scrollable(c.padding(k::pad(12.0, 18.0, 12.0, 18.0))).height(Length::Fill).into()
        },
        Msg::Describe,
    )
}

fn pop_box(a: &App) -> Option<E<'_, Msg>> {
    let p = a.p;
    Some(match a.pop.clone()? {
        Pop::Colour(i) => {
            let cur = conf::get(&a.text, COLOUR_KEYS[i].0).unwrap_or_else(|| COLOUR_KEYS[i].2.to_string());
            let mut r = Row::new().spacing(8.0);
            for c in PALETTE {
                let sel = c == cur;
                r = r.push(k::click(container(Space::new()).width(Length::Fixed(28.0)).height(Length::Fixed(28.0)).style(ui::boxed(k::rgb(&format!("#{c}")), 6.0, Some((if sel { Color::WHITE } else { Color::TRANSPARENT }, 2.0)))), Some(Msg::Colour(i, c))));
            }
            ui::dialog_box(&p, 360.0, column![row![k::h2::<Msg>(&p, format!("{}'s colour", COLOUR_KEYS[i].1)), k::fill(), k::chip(&p, "Close", Ch::Plain, Sz::Sm, Some(Msg::Close))], r].spacing(12.0))
        }
        Pop::Warn => k::chooser(&p, "Warn when the GPU is this hot", [75u32, 80, 83, 85, 90].iter().map(|v| (format!("GPU {v} °C"), a.set.get("warnGpu").and_then(Value::as_u64).unwrap_or(83) == *v as u64, Msg::Warn(*v))).collect(), Msg::Close),
        Pop::Keys => {
            let cur = conf::get(&a.text, "toggle_hud").unwrap_or_else(|| "Shift_R+F12".into());
            k::chooser(&p, "Show and hide", KEYS.iter().map(|(kk, n)| (n.to_string(), *kk == cur, Msg::Key(kk))).collect(), Msg::Close)
        }
        Pop::Limit => ui::dialog_box(
            &p,
            360.0,
            column![k::h2::<Msg>(&p, "Your own limit"), k::input(&p, "frames a second, like 90", &a.input, Msg::Text).on_submit(Msg::OwnLimit), row![k::fill(), k::chip(&p, "Cancel", Ch::Plain, Sz::Md, Some(Msg::Close)), k::chip(&p, "Set it", Ch::Acc, Sz::Md, Some(Msg::OwnLimit))].spacing(8.0)].spacing(10.0),
        ),
        Pop::AddGame => {
            let mut c = Column::new().spacing(8.0).push(row![k::h2::<Msg>(&p, "Settings for one game"), k::fill(), k::chip(&p, "Close", Ch::Plain, Sz::Sm, Some(Msg::Close))]);
            let running = running_programs();
            if !running.is_empty() {
                c = c.push(k::mu(&p, "Running now:", 11.0)).push(Row::with_children(running.into_iter().take(6).map(|r| k::chip(&p, r.clone(), Ch::Plain, Sz::Sm, Some(Msg::AddGame(r))))).spacing(6.0));
            }
            c = c.push(k::mu(&p, "Or its program's name (like ForzaHorizon5.exe):", 11.0)).push(k::input(&p, "ForzaHorizon5.exe", &a.input, Msg::Text).on_submit(Msg::AddGame(a.input.clone())));
            ui::dialog_box(&p, 440.0, c)
        }
    })
}

fn view(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    studio_values(a);
    let mut items: Vec<(&'static str, E<'_, Msg>)> = vec![("body", view_body(a))];
    for (id, which) in [("pane:shows", "shows"), ("pane:file", "file")] {
        if a.describe.wants_pane(which) {
            items.push((id, studio_pane(a, which)));
        }
    }
    let mut body = hearth_ui::arrange::parts(&a.describe, &p, "window", items, |ps| ps.take("body"), Msg::Describe);
    if let Some(d) = pop_box(a) {
        body = ui::dialog(body, d, Some(Msg::Close));
    }
    let bar = k::Bar {
        letters: "HO",
        from: k::rgb("#ffb36b"),
        to: k::rgb("#ff6a1f"),
        title: "Hearth Overlay",
        line: "· settings · changes show live in the game".into(),
        middle: None,
        right: vec![],
        msgs: TitleMsgs { drag: Msg::Drag, maximize: Msg::Max, minimize: Some(Msg::Min), close: Msg::WClose },
        open: Msg::Describe(hearth_ui::describe::DMsg::Open),
    };
    k::window(&p, &a.describe, bar, body, ("Hearth Overlay", "Built on MangoHud (MIT) and GOverlay (GPL-3)"), Msg::Resize, Msg::Describe)
}

use hearth_ui::blocks::{bl, bls, PageDef};
const BLOCKS: &[PageDef] = &[
    PageDef { id: "window", title: "Hearth Overlay", blocks: &[bls("body", "The settings", 0, 0, 12, 12)] },
    PageDef {
        id: "settings",
        title: "The settings",
        blocks: &[
            bl("need", "MangoHud", 0, 0, 12, 1),
            bl("tabs", "All games and each game", 0, 0, 12, 1),
            bl("start", "Start from", 0, 1, 12, 1),
            bl("shows", "Shows", 0, 2, 6, 5),
            bl("where", "Where", 6, 2, 6, 2),
            bl("look", "Look", 6, 4, 6, 3),
            bl("frame", "Frame rate", 0, 7, 12, 2),
            bl("gamemode", "In Game Mode", 0, 9, 12, 2),
            bl("buttons", "Reset, Copy and Done", 0, 11, 12, 1),
            bl("said", "What happened", 0, 11, 12, 1),
        ],
    },
];

use hearth_ui::studio::{tile, Kit};
const KIT: Kit = Kit {
    name: "Hearth Overlay",
    buttons: &[
        tile("on", "Overlay on or off", G::Power),
        tile("fps", "Just FPS", G::Pulse),
        tile("full", "Full", G::Grid),
        tile("small", "Small", G::List),
        tile("one-line", "One line", G::Columns),
        tile("rows", "Rows", G::List),
        tile("copy", "Copy to all games", G::Doc),
        tile("reset", "Reset this game", G::Update),
        tile("done", "Done", G::Check),
    ],
    panes: &[tile("shows", "What it shows", G::List), tile("file", "MangoHud's settings, as written", G::Doc)],
    info: &[tile("shows", "Things shown", G::Pulse), tile("limit", "Frame-rate limit", G::Clock), tile("place", "Where it sits", G::Grid), tile("games", "Games with their own", G::Pad)],
    pane_page: "window",
    bars: &[],
    menus: &[],
};

fn block_action(id: &str) -> Option<Msg> {
    Some(match id {
        "on" => Msg::OnOff,
        "fps" => Msg::Start(0),
        "small" => Msg::Start(1),
        "full" => Msg::Start(2),
        "one-line" => Msg::OneLine(true),
        "rows" => Msg::OneLine(false),
        "copy" => Msg::CopyAll,
        "reset" => Msg::Reset,
        "done" => Msg::Done,
        _ => return None,
    })
}

fn studio_values(a: &App) {
    let n = (0..SHOWS.len()).filter(|i| a.show_on(*i)).count();
    let lim = conf::get(&a.text, "fps_limit").unwrap_or_else(|| "Off".into());
    let pos = conf::get(&a.text, "position").unwrap_or_else(|| "top-left".into());
    a.describe.values(vec![("shows", n.to_string()), ("limit", lim), ("place", pos.replace('-', " ")), ("games", a.files.len().to_string())]);
}

fn studio_pane<'a>(a: &'a App, which: &str) -> E<'a, Msg> {
    let p = a.p;
    let mut c = Column::new().spacing(4.0);
    if which == "shows" {
        for (i, (l, _)) in SHOWS.iter().enumerate() {
            c = c.push(k::check(&p, a.show_on(i), *l, Msg::Show(i)));
        }
    } else {
        for l in a.text.lines() {
            c = c.push(t(l.to_string(), 11.0, p.text).font(iced::Font::with_name("DejaVu Sans Mono")).wrapping(Wrapping::None));
        }
    }
    scrollable(c).height(Length::Fill).into()
}

fn subscription(_a: &App) -> Subscription<Msg> {
    Subscription::run(hearth_ui::theme::watch).map(|_| Msg::Theme)
}

fn main() -> iced::Result {
    if app::handle_cli("hearth-overlay", "Hearth Overlay: the numbers over your games (built on MangoHud and GOverlay).\n  --game PROGRAM   that game's own settings") {
        return Ok(());
    }
    let w = app::window("hearth-overlay", (700.0, 780.0), (560.0, 560.0));
    iced::application(App::new, update, view).title(|_: &App| "Hearth Overlay".to_string()).subscription(subscription).theme(|a: &App| app::iced_theme(&a.p)).window(w).default_font(app::INTER).run()
}
