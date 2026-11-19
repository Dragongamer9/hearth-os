//! System: Display (settings-A2-display-mouse.png), Sound, Power & battery, Storage, Your desktop, Startup,
//! Date & time, About. Read with KDE's and the system's own tools (kscreen-doctor, wpctl/pactl, powerprofilesctl,
//! timedatectl, df); Hearth's own settings through hearth_settings_io.py.

use crate::data;
use crate::shared::Shared;
use crate::widgets::*;
use crate::App;
use hearth_ui::run;
use hearth_ui::ui::{self, btn, t, t1, tb, ts, Kind, G};
use hearth_ui::Pal;
use iced::widget::{column, container, row, Column, Row};
use iced::{Element, Length, Subscription, Task};

pub const PAGES: &[&str] = &["display", "sound", "power", "storage", "your-desktop", "startup", "datetime", "about"];

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Src {
    Display,
    Sound,
    Power,
    Storage,
    Time,
    Zones,
    Desk,
    Updates,
}

/// A display change waiting to be kept: the command that puts the old one back, and seconds left.
pub struct Keep {
    pub undo: Vec<String>,
    pub left: u32,
}

#[derive(Default)]
pub struct State {
    pub display: data::Display,
    pub display_read: bool,
    pub sound: data::Sound,
    pub sound_read: bool,
    pub power: data::Power,
    pub storage: Vec<data::Drive>,
    pub time: data::Time,
    pub zone_query: String,
    pub desk: data::Desk,
    pub desk_saved: bool,
    pub start: String,
    pub updates: data::Updates,
    pub keep: Option<Keep>,
    pub rename: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Got(Src, run::Output),
    Did(Src, &'static str, run::Output),
    Vol(f32),
    VolSet,
    Mute,
    Mic(f32),
    MicSet,
    MicMute,
    Out(String),
    In(String),
    Bright(u32),
    BrightSet,
    Night(bool),
    Res(String, u32, u32),
    Rate(String, String),
    Scale(String, f32),
    Keep,
    Revert,
    Tick,
    Profile(String),
    Ntp(bool),
    ZoneQuery(String),
    Zone(String),
    Desk(&'static str),
    SwitchNow,
    StartIn(&'static str),
    Rename,
    RenameText(String),
    RenameSave,
    RenameCancel,
    OpenApp(&'static str),
    Go(&'static str),
}

pub fn read(s: Src) -> Task<Msg> {
    let cmd = match s {
        Src::Display => data::sh(data::DISPLAY),
        Src::Sound => data::sh(data::SOUND),
        Src::Power => data::sh(data::POWER),
        Src::Storage => data::sh(data::STORAGE),
        Src::Time => data::sh(data::TIME),
        Src::Zones => data::sh("timedatectl list-timezones 2>/dev/null"),
        Src::Desk => data::sh(data::DESK),
        Src::Updates => vec!["pkexec".into(), data::ADMIN.into(), "update-status".into()],
    };
    Task::perform(run::spawn(cmd), move |o| Msg::Got(s, o))
}

pub fn open(_sh: &mut Shared, st: &mut State, page: &'static str) -> Task<Msg> {
    st.desk_saved = false;
    st.rename = None;
    st.start = data::start_mode();
    let srcs: &[Src] = match page {
        "display" => &[Src::Display],
        "sound" => &[Src::Sound],
        "power" => &[Src::Power],
        "storage" => &[Src::Storage],
        "datetime" => &[Src::Time, Src::Zones],
        "your-desktop" => &[Src::Desk],
        "about" => &[Src::Updates],
        _ => &[],
    };
    Task::batch(srcs.iter().map(|s| read(*s)))
}

fn act(cmd: Vec<String>, then: Src, what: &'static str) -> Task<Msg> {
    Task::perform(run::spawn(cmd), move |o| Msg::Did(then, what, o))
}

fn s(v: &[&str]) -> Vec<String> {
    v.iter().map(|x| x.to_string()).collect()
}

/// A resolution, refresh rate or scale change: put on now, and back by itself in 15 s unless it's kept (the
/// mockup's "X Keep these settings").
fn display_change(st: &mut State, out: &str, set: String, undo: String) -> Task<Msg> {
    st.keep = Some(Keep { undo: vec!["kscreen-doctor".into(), format!("output.{out}.{undo}")], left: 15 });
    act(vec!["kscreen-doctor".into(), format!("output.{out}.{set}")], Src::Display, "That display setting didn't take")
}

pub fn update(sh: &mut Shared, st: &mut State, m: Msg) -> Task<Msg> {
    match m {
        Msg::Got(src, o) => match src {
            Src::Display => {
                st.display = data::display(&o.out);
                st.display_read = true;
            }
            Src::Sound => {
                st.sound = data::sound(&o.out);
                st.sound_read = true;
            }
            Src::Power => st.power = data::power(&o.out),
            Src::Storage => st.storage = data::storage(&o.out),
            Src::Time => {
                let zones = std::mem::take(&mut st.time.zones);
                st.time = data::time(&o.out);
                st.time.zones = zones;
            }
            Src::Zones => st.time.zones = o.out.lines().map(String::from).collect(),
            Src::Desk => st.desk = data::desk(&o.out),
            Src::Updates => {
                if let Some(v) = o.json().filter(|_| o.ok) {
                    st.updates = data::updates(&v);
                }
            }
        },
        Msg::Did(src, what, o) => {
            sh.result(what, &o);
            return read(src);
        }
        Msg::Vol(v) => st.sound.vol = v,
        Msg::VolSet => return act(vec!["wpctl".into(), "set-volume".into(), "@DEFAULT_AUDIO_SINK@".into(), format!("{:.2}", st.sound.vol)], Src::Sound, "Couldn't change the volume"),
        Msg::Mute => return act(data::sh("wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle"), Src::Sound, "Couldn't mute"),
        Msg::Mic(v) => st.sound.mic = v,
        Msg::MicSet => return act(vec!["wpctl".into(), "set-volume".into(), "@DEFAULT_AUDIO_SOURCE@".into(), format!("{:.2}", st.sound.mic)], Src::Sound, "Couldn't change the microphone's volume"),
        Msg::MicMute => return act(data::sh("wpctl set-mute @DEFAULT_AUDIO_SOURCE@ toggle"), Src::Sound, "Couldn't mute the microphone"),
        Msg::Out(n) => return act(vec!["pactl".into(), "set-default-sink".into(), n], Src::Sound, "Couldn't switch the speakers"),
        Msg::In(n) => return act(vec!["pactl".into(), "set-default-source".into(), n], Src::Sound, "Couldn't switch the microphone"),
        Msg::Bright(v) => st.display.bright = Some(v),
        Msg::BrightSet => {
            if let Some(b) = st.display.bright {
                return act(vec!["brightnessctl".into(), "-q".into(), "-c".into(), "backlight".into(), "set".into(), format!("{}%", b.max(1))], Src::Display, "Couldn't change the brightness");
            }
        }
        Msg::Night(on) => return act(data::night_cmd(on), Src::Display, "Couldn't change Night light"),
        Msg::Res(out, w, h) => {
            if let Some(o) = st.display.outputs.iter().find(|o| o.name == out).cloned() {
                if let Some(id) = o.mode_for(w, h) {
                    return display_change(st, &o.name, format!("mode.{id}"), format!("mode.{}", o.current));
                }
            }
        }
        Msg::Rate(out, id) => {
            if let Some(o) = st.display.outputs.iter().find(|o| o.name == out).cloned() {
                return display_change(st, &o.name, format!("mode.{id}"), format!("mode.{}", o.current));
            }
        }
        Msg::Scale(out, sc) => {
            if let Some(o) = st.display.outputs.iter().find(|o| o.name == out).cloned() {
                return display_change(st, &o.name, format!("scale.{sc}"), format!("scale.{}", o.scale));
            }
        }
        Msg::Keep => st.keep = None,
        Msg::Revert => {
            if let Some(k) = st.keep.take() {
                return act(k.undo, Src::Display, "Couldn't put the display back");
            }
        }
        Msg::Tick => {
            let done = match st.keep.as_mut() {
                Some(k) => {
                    k.left = k.left.saturating_sub(1);
                    k.left == 0
                }
                None => false,
            };
            if done {
                return update(sh, st, Msg::Revert);
            }
        }
        Msg::Profile(pr) => return act(vec!["powerprofilesctl".into(), "set".into(), pr], Src::Power, "Couldn't change the power mode"),
        Msg::Ntp(on) => return act(s(&["timedatectl", "set-ntp", if on { "true" } else { "false" }]), Src::Time, "Couldn't change that"),
        Msg::ZoneQuery(q) => st.zone_query = q,
        Msg::Zone(z) => {
            st.zone_query.clear();
            return act(vec!["timedatectl".into(), "set-timezone".into(), z], Src::Time, "Couldn't change the time zone");
        }
        Msg::Desk(pick) => {
            st.desk_saved = true;
            st.desk.chosen = pick.to_string();
            return act(data::desk_cmd(&st.desk, pick), Src::Desk, "Couldn't save the desktop");
        }
        Msg::SwitchNow => run::detached(&["nimbus-mode", "desktop"]),
        Msg::StartIn(mode) => {
            st.start = mode.to_string();
            let cmd = sh.conf_cmd(&data::session_conf(), "NIMBUS_START_MODE", mode);
            return act(cmd, Src::Desk, "Couldn't save Start in");
        }
        Msg::Rename => st.rename = Some(sh.info.host.clone()),
        Msg::RenameText(t) => st.rename = Some(data::host_chars(&t)),
        Msg::RenameSave => {
            if let Some(n) = st.rename.take().filter(|n| !n.is_empty()) {
                sh.info.host = n.clone();
                return act(vec!["pkexec".into(), data::ADMIN.into(), "hostname".into(), n], Src::Updates, "Couldn't rename this PC");
            }
        }
        Msg::RenameCancel => st.rename = None,
        Msg::OpenApp(bin) => run::detached(&[bin]),
        Msg::Go(id) => sh.go(id),
    }
    Task::none()
}

pub fn view<'a>(a: &'a App, page: &'static str) -> Option<Element<'a, Msg>> {
    Some(match page {
        "display" => display(a),
        "sound" => sound(a),
        "power" => power(a),
        "storage" => storage(a),
        "your-desktop" => your_desktop(a),
        "startup" => startup(a),
        "datetime" => datetime(a),
        "about" => about(a),
        _ => return None,
    })
}

fn display(a: &App) -> Element<'_, Msg> {
    let p = a.sh.p;
    let st = &a.system;
    let d = &st.display;
    let mut c = Column::new().spacing(10.0);
    if !d.ok {
        c = c.push(note(&p, if st.display_read { "Your displays couldn't be read (KDE's kscreen-doctor didn't answer)." } else { "Reading your displays…" }, st.display_read));
    }
    let Some(main) = d.outputs.first() else { return c.into() };
    let mut boxes = Row::new().spacing(14.0).align_y(iced::Center);
    for n in 0..d.outputs.len() {
        let big = n == 0;
        boxes = boxes.push(
            container(tb(format!("{}", n + 1), if big { 34.0 } else { 26.0 }, p.text))
                .center_x(Length::Fixed(if big { 240.0 } else { 156.0 }))
                .center_y(Length::Fixed(if big { 136.0 } else { 88.0 }))
                .style(ui::boxed(if big { Pal::mix(p.panel2, p.accent, 0.08) } else { p.panel2 }, p.r(8.0), Some((if big { p.accent } else { p.line }, 2.0)))),
        );
    }
    let cur = main.cur();
    let facts = cur.map(|m| format!("{} × {} · {} Hz · scale {}%", m.w, m.h, m.hz.round(), (main.scale * 100.0).round())).unwrap_or_default();
    let gpu = if a.sh.info.gpu.is_empty() { String::new() } else { format!(" · connected to the {}", a.sh.info.gpu) };
    c = c.push(card(&p, row![boxes, column![ts(format!("{} · main display", main.name), sz(18.0), p.text), t(format!("{facts}{gpu}"), sz(13.0), p.muted)].spacing(6.0)].spacing(26.0).align_y(iced::Center)));
    if let Some(k) = &st.keep {
        c = c.push(card(&p, row![
            column![ts("Keep these display settings?", sz(15.0), p.text), t(format!("The old ones come back by themselves in {} s.", k.left), sz(13.0), p.muted)].spacing(2.0).width(Length::Fill),
            btn(&p, "Keep these settings", Kind::Primary, false, Some(Msg::Keep)),
            btn(&p, "Put back", Kind::Normal, false, Some(Msg::Revert))
        ].spacing(10.0).align_y(iced::Center)));
    }
    c = c.push(head(&p, "Brightness & colour"));
    if let Some(b) = d.bright {
        c = c.push(srow(&p, G::Display, "Brightness", format!("{b}%"), slider_u32(b, 1..=100, Msg::Bright, Msg::BrightSet)));
    }
    c = c.push(srow(&p, G::Moon, "Night light", "Warmer colours from sunset to sunrise", switch(&p, d.night, Some(Msg::Night(!d.night)))));
    c = c.push(head(&p, "Scale & layout"));
    let name = main.name.clone();
    let scales = [1.0f32, 1.25, 1.5, 1.75, 2.0];
    c = c.push(srow(&p, G::Grid, "Scale", "Text, apps and the bar", seg(&p, scales.iter().map(|sc| (format!("{}%", (sc * 100.0).round()), (main.scale - sc).abs() < 0.01, Some(Msg::Scale(name.clone(), *sc)))).collect())));
    let rec = main.preferred.first().and_then(|id| main.modes.iter().find(|m| &m.id == id)).map(|m| format!("Recommended: {} × {}", m.w, m.h)).unwrap_or_default();
    let shown: Vec<(u32, u32)> = main.sizes().into_iter().take(3).collect();
    c = c.push(srow(&p, G::Display, "Resolution", rec, seg(&p, shown.iter().map(|(w, h)| (format!("{w} × {h}"), cur.map(|m| m.w == *w && m.h == *h).unwrap_or(false), Some(Msg::Res(name.clone(), *w, *h)))).collect())));
    c = c.push(srow(&p, G::Update, "Refresh rate", "How many pictures a second the display shows", seg(&p, main.rates().into_iter().take(4).map(|(hz, id)| (format!("{hz} Hz"), cur.map(|m| m.id == id).unwrap_or(false), Some(Msg::Rate(name.clone(), id)))).collect())));
    c.into()
}

fn sound(a: &App) -> Element<'_, Msg> {
    let p = a.sh.p;
    let st = &a.system;
    let s = &st.sound;
    let mut c = Column::new().spacing(10.0);
    if !s.ok {
        return note(&p, if st.sound_read { "Sound couldn't be read (PipeWire's wpctl didn't answer)." } else { "Reading sound…" }, st.sound_read);
    }
    c = c.push(head(&p, "Output"));
    c = c.push(srow(&p, G::Sound, "Volume", if s.muted { "Muted".to_string() } else { format!("{}%", (s.vol * 100.0).round()) }, row![
        slider_f32(s.vol.min(1.0), 0.0..=1.0, 0.01, Msg::Vol, Msg::VolSet),
        btn(&p, if s.muted { "Unmute" } else { "Mute" }, Kind::Normal, false, Some(Msg::Mute))
    ].spacing(14.0).align_y(iced::Center).into()));
    for o in &s.outs {
        let on = o.name == s.out;
        c = c.push(srow(&p, G::Sound, o.desc.clone(), if on { "Playing here" } else { "" }, if on { t1("In use", sz(13.0), p.accent).into() } else { btn(&p, "Use", Kind::Normal, false, Some(Msg::Out(o.name.clone()))) }));
    }
    c = c.push(head(&p, "Input"));
    c = c.push(srow(&p, G::Sound, "Microphone", if s.mic_muted { "Muted".to_string() } else { format!("{}%", (s.mic * 100.0).round()) }, row![
        slider_f32(s.mic.min(1.0), 0.0..=1.0, 0.01, Msg::Mic, Msg::MicSet),
        btn(&p, if s.mic_muted { "Unmute" } else { "Mute" }, Kind::Normal, false, Some(Msg::MicMute))
    ].spacing(14.0).align_y(iced::Center).into()));
    for o in &s.ins {
        let on = o.name == s.inp;
        c = c.push(srow(&p, G::Sound, o.desc.clone(), if on { "Recording from here" } else { "" }, if on { t1("In use", sz(13.0), p.accent).into() } else { btn(&p, "Use", Kind::Normal, false, Some(Msg::In(o.name.clone()))) }));
    }
    c.into()
}

fn power(a: &App) -> Element<'_, Msg> {
    let p = a.sh.p;
    let w = &a.system.power;
    let mut c = Column::new().spacing(10.0);
    if let Some(b) = w.battery {
        c = c.push(srow(&p, G::Power, "Battery", format!("{b}% · {}", if w.charging.is_empty() { "Unknown" } else { w.charging.as_str() }), bar_of(&p, b as f64, 100.0)));
    }
    if w.profiles.is_empty() {
        c = c.push(srow(&p, G::Power, "Power mode", "This PC doesn't offer power modes", t1("", 13.0, p.muted).into()));
    } else {
        c = c.push(srow(&p, G::Power, "Power mode", "Power saver for a longer battery, Performance for games", seg(&p, w.profiles.iter().map(|pr| (data::profile_name(pr).to_string(), *pr == w.profile, Some(Msg::Profile(pr.clone())))).collect())));
    }
    c.into()
}

fn storage(a: &App) -> Element<'_, Msg> {
    let p = a.sh.p;
    column![card(&p, drive_rows(&p, &a.system.storage)), link_row(&p, G::Drive, "Disk Usage", "What's using the space, folder by folder", Msg::OpenApp("hearth-disk-usage"))].spacing(10.0).into()
}

fn startup(a: &App) -> Element<'_, Msg> {
    let p = a.sh.p;
    let st = &a.system;
    column![srow(&p, G::Power, "Start in", "What this PC opens when it turns on", seg(&p, vec![
        ("Game Mode".into(), st.start == "game", Some(Msg::StartIn("game"))),
        ("Desktop Mode".into(), st.start == "desktop", Some(Msg::StartIn("desktop"))),
    ]))]
    .into()
}

fn datetime(a: &App) -> Element<'_, Msg> {
    let p = a.sh.p;
    let st = &a.system;
    let tm_ = &st.time;
    let mut c = Column::new().spacing(10.0);
    c = c.push(srow(&p, G::Clock, "Set the time by itself", "From the internet (NTP)", switch(&p, tm_.ntp, Some(Msg::Ntp(!tm_.ntp)))));
    c = c.push(srow(&p, G::Clock, "Time zone", if tm_.zone.is_empty() { "…".to_string() } else { tm_.zone.clone() }, field(&p, "Find a time zone", &st.zone_query, Msg::ZoneQuery).width(Length::Fixed(280.0)).into()));
    if st.zone_query.chars().count() >= 2 {
        let q = st.zone_query.to_lowercase().replace(' ', "_");
        for z in tm_.zones.iter().filter(|z| z.to_lowercase().contains(&q)).take(12) {
            c = c.push(pick_row(&p, z.replace('_', " "), Msg::Zone(z.clone())));
        }
    }
    c.into()
}

fn your_desktop(a: &App) -> Element<'_, Msg> {
    let p = a.sh.p;
    let st = &a.system;
    let d = &st.desk;
    let mut opts: Vec<(&'static str, &'static str, &'static str)> = vec![("plasma", "KDE Plasma", "The desktop, with Hearth's look (the default)")];
    if d.rust {
        opts.push(("firehold-rust", "Firehold (Rust, test)", "Hearth's own Couch and Desk, all in Rust. If it doesn't start, KDE Plasma opens"));
    }
    if d.plasma_fh {
        opts.push(("firehold-plasma", "Firehold (Plasma, test)", "Firehold's Couch and Desk on KDE Plasma"));
    }
    let mut c = Column::new().spacing(10.0).push(t("What Desktop Mode opens. Nothing changes until you switch to Desktop Mode again.", sz(14.0), p.muted));
    for (id, name, sub) in opts {
        let on = d.chosen == id;
        c = c.push(srow(&p, G::Grid, name, sub, if on { t1("Picked", sz(13.0), p.accent).into() } else { btn(&p, "Use", Kind::Normal, false, Some(Msg::Desk(id))) }));
    }
    if st.desk_saved {
        c = c.push(card(&p, row![
            column![ts("Saved", sz(15.0), p.text), t("It opens the next time you switch to Desktop Mode. Switching now closes open apps.", sz(13.0), p.muted)].spacing(2.0).width(Length::Fill),
            btn(&p, "Switch now", Kind::Primary, false, Some(Msg::SwitchNow))
        ].spacing(10.0).align_y(iced::Center)));
    }
    c.into()
}

fn about(a: &App) -> Element<'_, Msg> {
    let p = a.sh.p;
    let st = &a.system;
    let i = &a.sh.info;
    let mut c = Column::new().spacing(10.0);
    c = c.push(match &st.rename {
        Some(n) => card(&p, crate::cats::home::rename_box(&p, n, Msg::RenameText, Msg::RenameSave, Msg::RenameCancel)),
        None => srow(&p, G::Info, if i.host.is_empty() { "This PC".to_string() } else { i.host.clone() }, "This PC's name on your network", btn(&p, "Rename", Kind::Normal, false, Some(Msg::Rename))),
    });
    let mem = if i.mem_kb > 0 { ui::bytes(i.mem_kb as f64 * 1024.0) } else { String::new() };
    let fedora = st.updates.booted_image.contains("hearth-os:") || i.os.contains("Fedora") || i.os.to_lowercase().contains("anatase");
    c = c.push(card(&p, column![
        fact(&p, "Hearth OS", format!("{}{}", data::shown_version(&st.updates.booted), if fedora { " · Fedora edition" } else { "" })),
        fact(&p, "Built on", i.os.clone()),
        fact(&p, "Image", st.updates.booted_image.clone()),
        fact(&p, "Kernel", i.kernel.clone()),
        fact(&p, "Processor", data::short_cpu(&i.cpu)),
        fact(&p, "Graphics", i.gpu.clone()),
        fact(&p, "Memory", mem),
        fact(&p, "Desktop", i.desktop.clone()),
    ].spacing(10.0)));
    c = c.push(link_row(&p, G::Info, "System Info", "Everything about this PC's parts, to copy or send", Msg::OpenApp("hearth-system-info")));
    c.into()
}

pub fn subscription(st: &State) -> Subscription<Msg> {
    if st.keep.is_some() {
        iced::time::every(std::time::Duration::from_secs(1)).map(|_| Msg::Tick)
    } else {
        Subscription::none()
    }
}

pub fn dialog<'a>(_a: &'a App) -> Option<(Element<'a, Msg>, Option<Msg>)> {
    None
}

pub fn back(st: &mut State) -> bool {
    st.rename.take().is_some()
}
