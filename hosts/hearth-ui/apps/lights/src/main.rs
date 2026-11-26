//! hearth-lights-app: Hearth Lights (board docs/mockups/open-source-apps/09-lights.png; checklist
//! docs/mockups/checklists/open-source-09-lights.md). Every RGB light in the PC in one place, built on OpenRGB
//! (GPL-2): it talks to OpenRGB's SDK server on this PC (orgb.rs), so every device OpenRGB knows is here, in its real
//! colours. One colour or effect for all, or for one part (picked on the left or in the picture), each light of a fan
//! painted, and profiles that can switch by themselves when a game starts, in Desktop Mode or when the screen sleeps
//! (only if you say so: "ask me" by default).
//!
//! Where a device has a mode of its own for the effect it's used and saved on the device; Hearth's own effects (Ember
//! wave, Hotter = redder, To the music, Game colours) are sent by the runner (`--run`, hearth-lights.service), which
//! also puts your lights back after a restart. OpenRGB not here: the app offers to get it (Flathub, the system's
//! Flatpak); not running: it starts it (hearth-openrgb.service). Neither is ever part of starting the PC.
//!
//! $HEARTH_OPENRGB (host:port): a pretend OpenRGB server (CI's pictures); no services are started then.

#[path = "../../oss-kit/kit.rs"]
mod kit;
mod fx;
mod orgb;
mod stage;

use fx::{Choices, Look, Way};
use hearth_ui::app::{self, win};
use hearth_ui::blocks::{bl, PageDef};
use hearth_ui::studio::{tile, Kit};
use hearth_ui::ui::{self, t, tb, tm, ts, TitleMsgs, G};
use hearth_ui::Pal;
use iced::widget::{canvas, column, container, row, scrollable, slider, text_input, Column, Row, Space};
use iced::{Color, Element, Length, Subscription, Task};
use kit::{card, caps, chip, click, dot, pad, seg, switch, tag, Tag, E};
use orgb::{kind, Device, Rgb};
use serde_json::json;

#[derive(Debug, Clone)]
pub enum Msg {
    Describe(hearth_ui::describe::DMsg),
    Theme,
    Loaded(Result<Vec<Device>, String>),
    Applied(Result<Vec<(String, Way)>, String>),
    Pick(String),
    Toggle(String),
    AllOff,
    LookAgain,
    Wheel(Rgb),
    Hex(String),
    Effect(&'static str),
    Speed(u8),
    Bright(u8),
    Released,
    Dir(&'static str),
    Profile(String),
    NewProfile,
    SwitchBy,
    ZoneMode(&'static str),
    Paint(usize, usize),
    GetIt,
    StartIt,
    Hints(Vec<Hint>),
    AllowI2c,
    Done(Result<(), String>),
    Drag,
    Max,
    Min,
    Close,
    Resize(iced::window::Direction),
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Link {
    Looking,
    Ok,
    /// OpenRGB is here but its server isn't answering
    NotRunning,
    /// OpenRGB isn't on this PC
    Missing,
    Busy,
}

/// Why a light might not be seen yet, in words, and what fixes it (Not seen yet? · Hearth explains each one).
#[derive(Debug, Clone, PartialEq)]
pub struct Hint {
    words: String,
    /// the one-time OK (memory and motherboard lights: the i2c-dev module), when that's the fix
    allow: bool,
}

/// What could be hiding lights: memory and motherboard lights need the i2c-dev module (a one-time OK), and a maker's
/// own program holding its devices keeps OpenRGB from them.
fn hints() -> Vec<Hint> {
    let mut v = Vec::new();
    if std::env::var("HEARTH_OPENRGB").is_ok() {
        return v;
    }
    if !std::path::Path::new("/sys/module/i2c_dev").exists() {
        v.push(Hint { words: "Memory and motherboard lights need a one-time OK (Hearth loads the i2c-dev module, now and at every start).".into(), allow: true });
    }
    let holders = [("ckb-next-daemon", "ckb-next holds Corsair keyboards and mice: close it, then Look again."), ("openrazer-daemon", "OpenRazer holds Razer devices: close it, then Look again."), ("asusd", "asusd holds ASUS laptop lights: Hearth Lights can share them only when asusd's own lighting is off."), ("liquidctl", "liquidctl is talking to your cooler: let it finish, then Look again."), ("signalrgb", "SignalRGB is running: close it, then Look again.")];
    let mut running: Vec<String> = Vec::new();
    if let Ok(rd) = std::fs::read_dir("/proc") {
        for e in rd.flatten() {
            if let Ok(c) = std::fs::read_to_string(e.path().join("comm")) {
                running.push(c.trim().to_lowercase());
            }
        }
    }
    for (name, words) in holders {
        if running.iter().any(|r| r.starts_with(&name[..name.len().min(15)])) {
            v.push(Hint { words: words.into(), allow: false });
        }
    }
    v
}

/// The board's devices: OpenRGB's devices, the memory sticks as one.
#[derive(Debug, Clone)]
struct Group {
    key: String,
    label: String,
    sub: String,
    dot: Rgb,
    members: Vec<usize>,
}

struct App {
    describe: hearth_ui::describe::Describe,
    p: Pal,
    ch: Choices,
    devs: Vec<Device>,
    groups: Vec<Group>,
    link: Link,
    /// the group picked ("" all devices)
    picked: String,
    hex: String,
    ways: Vec<(String, Way)>,
    said: String,
    fixture: bool,
    hints: Vec<Hint>,
}

const SWATCHES: [Rgb; 6] = [[0xff, 0x8a, 0x3d], [0xff, 0x3d, 0x6a], [0xb0, 0x4f, 0xc8], [0x5e, 0xc8, 0xff], [0x9b, 0xe3, 0x6b], [0xff, 0xff, 0xff]];
const PROFILE_DOTS: [Rgb; 6] = [[0xff, 0x8a, 0x3d], [0x5e, 0xc8, 0xff], [0xb0, 0x4f, 0xc8], [0xa5, 0x9a, 0x91], [0x9b, 0xe3, 0x6b], [0xff, 0xd3, 0x6b]];

fn rc(x: Rgb) -> Color {
    Color::from_rgb8(x[0], x[1], x[2])
}

fn installed() -> bool {
    if let Ok(v) = std::env::var("HEARTH_LIGHTS_INSTALLED") {
        return v == "1";
    }
    kit::have("openrgb") || std::path::Path::new("/var/lib/flatpak/app/org.openrgb.OpenRGB").exists() || hearth_ui::paths::home().join(".local/share/flatpak/app/org.openrgb.OpenRGB").exists()
}

fn find_hints() -> Task<Msg> {
    Task::perform(kit::bg(hints), |h| Msg::Hints(h.unwrap_or_default()))
}

fn load() -> Task<Msg> {
    Task::perform(kit::bg(|| orgb::Conn::open().and_then(|mut c| c.devices()).map_err(|e| e.to_string())), |r| Msg::Loaded(r.unwrap_or_else(|| Err("stopped".into()))))
}

/// The friendly names and lines the board shows ("Memory (4 sticks)" · "16 lights each").
fn groups_of(devs: &[Device]) -> Vec<Group> {
    let mut out: Vec<Group> = Vec::new();
    let leds = |d: &Device| d.zones.iter().map(|z| z.leds as usize).sum::<usize>().max(d.leds.len());
    let first = |d: &Device| d.colors.first().copied().unwrap_or([0xff, 0x8a, 0x3d]);
    let ram: Vec<usize> = devs.iter().enumerate().filter(|(_, d)| d.kind == kind::DRAM).map(|(i, _)| i).collect();
    let mut ram_done = false;
    for (i, d) in devs.iter().enumerate() {
        if d.kind == kind::DRAM {
            if ram_done {
                continue;
            }
            ram_done = true;
            let each = leds(d);
            out.push(Group { key: fx::key_of(d), label: format!("Memory ({} stick{})", ram.len(), if ram.len() == 1 { "" } else { "s" }), sub: format!("{each} lights each"), dot: first(d), members: ram.clone() });
            continue;
        }
        let n = leds(d);
        let zl = |z: &orgb::Zone| z.name.to_lowercase();
        let fanlike = d.name.to_lowercase().contains("fan");
        let (label, sub) = match d.kind {
            kind::MOTHERBOARD => ("Motherboard".to_string(), format!("{} header{}", d.zones.len(), if d.zones.len() == 1 { "" } else { "s" })),
            kind::GPU => ("Graphics card".to_string(), format!("{}, {n} lights", d.zones.first().map(zl).unwrap_or_else(|| "lights".into()))),
            kind::KEYBOARD => ("Keyboard".to_string(), if d.zones.iter().any(|z| z.rows > 0) { format!("per-key, {n}") } else { format!("{n} lights") }),
            kind::MOUSE => ("Mouse".to_string(), d.zones.iter().map(zl).collect::<Vec<_>>().join(" + ")),
            kind::COOLER if !fanlike => ("CPU cooler".to_string(), format!("{}, {n} lights", d.zones.first().map(zl).unwrap_or_else(|| "lights".into()))),
            _ => {
                let sub = if d.zones.len() > 1 && d.zones.iter().all(|z| z.leds == d.zones[0].leds) {
                    format!("{} × {} lights", d.zones.len(), d.zones[0].leds)
                } else if d.location.to_uppercase().starts_with("HID") || d.location.to_lowercase().contains("usb") {
                    format!("USB, {n} lights")
                } else {
                    format!("{n} lights")
                };
                (d.name.clone(), sub)
            }
        };
        out.push(Group { key: fx::key_of(d), label, sub, dot: first(d), members: vec![i] });
    }
    out
}

impl App {
    fn new() -> (App, Task<Msg>) {
        let ch = Choices::read();
        let fixture = std::env::var("HEARTH_OPENRGB").is_ok();
        let look = ch.profile(&ch.current).look.clone();
        let a = App {
            describe: hearth_ui::describe::Describe::new("lights", "Hearth Lights").with_pages(BLOCKS).with_kit(&KIT),
            p: hearth_ui::theme::load(),
            hex: fx::rgb_hex(look.colour),
            ch,
            devs: Vec::new(),
            groups: Vec::new(),
            link: Link::Looking,
            picked: app::arg("--pick").unwrap_or_default(),
            ways: Vec::new(),
            said: String::new(),
            fixture,
            hints: Vec::new(),
        };
        (a, Task::batch([load(), find_hints()]))
    }
    fn profile_look(&self) -> Look {
        self.ch.profile(&self.ch.current).look.clone()
    }
    /// The look the panel shows: the picked part's own, or the profile's.
    fn look(&self) -> Look {
        let pl = self.profile_look();
        match self.group(&self.picked) {
            Some(g) => self.ch.look_for(&g.key, &pl),
            None => pl,
        }
    }
    fn group(&self, key: &str) -> Option<&Group> {
        if key.is_empty() {
            return None;
        }
        self.groups.iter().find(|g| g.key == key)
    }
    fn member_keys(&self, g: &Group) -> Vec<String> {
        g.members.iter().filter_map(|i| self.devs.get(*i)).map(fx::key_of).collect()
    }
    fn is_off(&self, g: &Group) -> bool {
        self.member_keys(g).iter().all(|k| self.ch.off.contains(k))
    }

    /// The look changed in the panel: kept (profile's, or the picked part's own), then shown on the lights.
    fn set_look(&mut self, f: impl Fn(&mut Look)) -> Task<Msg> {
        let mut l = self.look();
        f(&mut l);
        self.hex = fx::rgb_hex(l.colour);
        match self.group(&self.picked).cloned() {
            Some(g) => {
                for k in self.member_keys(&g) {
                    self.ch.own.insert(k, fx::look_json(&l));
                }
                let own = serde_json::Value::Object(self.ch.own.clone());
                self.keep(vec![("own", own)]);
            }
            None => {
                // All devices: one colour or effect for all (parts given their own look follow it again)
                let cur = self.ch.current.clone();
                if let Some(p) = self.ch.profiles.iter_mut().find(|p| p.id == cur) {
                    p.look = l;
                }
                self.ch.own.clear();
                let pj = self.ch.profiles_json();
                self.keep(vec![("profiles", pj), ("own", json!({}))]);
            }
        }
        self.show()
    }

    fn keep(&mut self, changes: Vec<(&str, serde_json::Value)>) {
        if let Err(e) = kit::save(fx::APP, changes) {
            self.said = format!("Couldn't keep that: {e}");
        }
    }

    /// Shows the choices on every device now (off the window's thread), and keeps the runner going when Hearth's own
    /// frames or profile switching need it.
    fn show(&mut self) -> Task<Msg> {
        self.show_now(true)
    }
    /// `changed`: the person just changed something (only then is the runner turned on: opening the app changes
    /// nothing by itself).
    fn show_now(&mut self, changed: bool) -> Task<Msg> {
        if self.link != Link::Ok {
            return Task::none();
        }
        let ch = self.ch.clone();
        let devs = self.devs.clone();
        let needs_runner = changed && !self.fixture;
        Task::perform(
            kit::bg(move || -> Result<Vec<(String, Way)>, String> {
                let mut conn = orgb::Conn::open().map_err(|e| e.to_string())?;
                let pl = ch.profile(&ch.current).look.clone();
                let inp = fx::Inputs { temp: fx::cpu_temp(), ..Default::default() };
                let mut out = Vec::new();
                let mut moving = false;
                for d in &devs {
                    let k = fx::key_of(d);
                    let l = ch.look_for(&k, &pl);
                    let off = ch.all_off || ch.off.contains(&k);
                    let w = fx::apply(&mut conn, d, &l, &inp, ch.painted_of(&k).as_ref(), off).map_err(|e| e.to_string())?;
                    moving |= w == Way::Frames && fx::moves(&l.effect);
                    out.push((k, w));
                }
                if needs_runner {
                    // the runner puts the lights back after a restart, sends Hearth's frames, and switches profiles
                    kit::user_unit("enable", "hearth-lights.service");
                    if moving || ch.switch != "no" || !kit::user_unit_active("hearth-lights.service") {
                        kit::user_unit("restart", "hearth-lights.service");
                    }
                }
                Ok(out)
            }),
            |r| Msg::Applied(r.unwrap_or_else(|| Err("stopped".into()))),
        )
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
        Msg::Loaded(r) => match r {
            Ok(d) => {
                a.devs = d;
                a.groups = groups_of(&a.devs);
                a.link = Link::Ok;
                if a.group(&a.picked).is_none() {
                    a.picked.clear();
                }
                a.hex = fx::rgb_hex(a.look().colour);
                // what the window says is what the lights show (nothing is kept by opening it)
                return a.show_now(false);
            }
            Err(_) => {
                a.link = if installed() { Link::NotRunning } else { Link::Missing };
                a.devs.clear();
                a.groups.clear();
            }
        },
        Msg::Applied(r) => match r {
            Ok(w) => {
                a.ways = w;
                a.said.clear();
            }
            Err(e) => a.said = format!("The lights didn't answer: {e}"),
        },
        Msg::Pick(k) => {
            a.picked = if a.picked == k { String::new() } else { k };
            a.hex = fx::rgb_hex(a.look().colour);
        }
        Msg::Toggle(k) => {
            let Some(g) = a.group(&k).cloned() else { return Task::none() };
            let keys = a.member_keys(&g);
            let off = a.is_off(&g);
            a.ch.off.retain(|x| !keys.contains(x));
            if !off {
                a.ch.off.extend(keys);
            }
            let v = json!(a.ch.off);
            a.keep(vec![("off", v)]);
            return a.show();
        }
        Msg::AllOff => {
            a.ch.all_off = !a.ch.all_off;
            let v = a.ch.all_off;
            a.keep(vec![("allOff", json!(v))]);
            return a.show();
        }
        Msg::LookAgain => {
            a.link = Link::Looking;
            let rescan = Task::perform(kit::bg(|| orgb::Conn::open().and_then(|mut c| c.rescan()).is_ok()), |_| Msg::Done(Ok(())));
            return Task::batch([rescan.chain(Task::perform(run_after(2500), |_| Msg::Done(Ok(())))).chain(load()), find_hints()]);
        }
        Msg::Wheel(x) => return a.set_look(|l| l.colour = x),
        Msg::Hex(s) => {
            a.hex = s.chars().filter(|c| c.is_ascii_hexdigit() || *c == '#').take(7).collect();
            if let Some(x) = fx::hex_rgb(&a.hex) {
                let h = a.hex.clone();
                let t = a.set_look(|l| l.colour = x);
                a.hex = h;
                return t;
            }
        }
        Msg::Effect(e) => return a.set_look(|l| l.effect = e.to_string()),
        Msg::Speed(v) => {
            // shown as it moves; kept and sent when it's let go
            let cur = a.ch.current.clone();
            if a.picked.is_empty() {
                if let Some(p) = a.ch.profiles.iter_mut().find(|p| p.id == cur) {
                    p.look.speed = v;
                }
            } else if let Some(g) = a.group(&a.picked).cloned() {
                let mut l = a.look();
                l.speed = v;
                for k in a.member_keys(&g) {
                    a.ch.own.insert(k, fx::look_json(&l));
                }
            }
        }
        Msg::Bright(v) => {
            let cur = a.ch.current.clone();
            if a.picked.is_empty() {
                if let Some(p) = a.ch.profiles.iter_mut().find(|p| p.id == cur) {
                    p.look.brightness = v;
                }
            } else if let Some(g) = a.group(&a.picked).cloned() {
                let mut l = a.look();
                l.brightness = v;
                for k in a.member_keys(&g) {
                    a.ch.own.insert(k, fx::look_json(&l));
                }
            }
        }
        Msg::Released => return a.set_look(|_| {}),
        Msg::Dir(d) => return a.set_look(|l| l.direction = d.to_string()),
        Msg::Profile(id) => {
            a.ch.current = id.clone();
            a.picked.clear();
            a.hex = fx::rgb_hex(a.look().colour);
            a.keep(vec![("current", json!(id))]);
            return a.show();
        }
        Msg::NewProfile => {
            let n = a.ch.profiles.len() + 1;
            let mut id = format!("profile-{n}");
            let mut k = n;
            while a.ch.profiles.iter().any(|p| p.id == id) {
                k += 1;
                id = format!("profile-{k}");
            }
            let look = a.profile_look();
            a.ch.profiles.push(fx::Profile { id: id.clone(), name: format!("Profile {n}"), when: "hand".into(), look });
            a.ch.current = id.clone();
            let pj = a.ch.profiles_json();
            a.keep(vec![("profiles", pj), ("current", json!(id))]);
        }
        Msg::SwitchBy => {
            a.ch.switch = match a.ch.switch.as_str() {
                "ask" => "yes",
                "yes" => "no",
                _ => "ask",
            }
            .to_string();
            let v = a.ch.switch.clone();
            a.keep(vec![("switch", json!(v))]);
            return a.show();
        }
        Msg::ZoneMode(z) => {
            a.ch.zone_mode = z.to_string();
            a.keep(vec![("zoneMode", json!(z))]);
        }
        Msg::Paint(z, j) => {
            let Some(di) = zone_device(a) else { return Task::none() };
            let d = a.devs[di].clone();
            let k = fx::key_of(&d);
            let look = a.look();
            let colour = look.colour;
            let mut painted: Vec<Vec<Rgb>> = a.ch.painted_of(&k).unwrap_or_else(|| {
                let fr = fx::frame(&d, &Look { brightness: 100, ..look.clone() }, 0.0, &fx::Inputs::default(), None);
                d.zones.iter().enumerate().map(|(zi, zn)| (0..zn.leds as usize).map(|x| fr.get(d.zone_start(zi) + x).copied().unwrap_or(colour)).collect()).collect()
            });
            while painted.len() < d.zones.len() {
                painted.push(Vec::new());
            }
            for (zi, zn) in d.zones.iter().enumerate() {
                painted[zi].resize(zn.leds as usize, colour);
            }
            match a.ch.zone_mode.as_str() {
                "light" => {
                    if let Some(x) = painted.get_mut(z).and_then(|r| r.get_mut(j)) {
                        *x = colour;
                    }
                }
                "fan" => {
                    if let Some(r) = painted.get_mut(z) {
                        r.iter_mut().for_each(|x| *x = colour);
                    }
                }
                _ => {
                    for r in painted.iter_mut() {
                        if let Some(x) = r.get_mut(j) {
                            *x = colour;
                        }
                    }
                }
            }
            a.ch.painted.insert(k.clone(), json!(painted.iter().map(|r| r.iter().map(|c| fx::rgb_hex(*c)).collect::<Vec<_>>()).collect::<Vec<_>>()));
            // painted lights show while the part is Still
            if look.effect != "still" {
                a.ch.own.insert(k, fx::look_json(&Look { effect: "still".into(), ..look }));
            }
            let (pv, ov) = (serde_json::Value::Object(a.ch.painted.clone()), serde_json::Value::Object(a.ch.own.clone()));
            a.keep(vec![("painted", pv), ("own", ov)]);
            return a.show();
        }
        Msg::GetIt => {
            a.link = Link::Busy;
            a.said = "Getting OpenRGB from Flathub…".into();
            return Task::perform(
                kit::bg(|| {
                    let o = hearth_ui::run::blocking(&hearth_ui::run::args(&["flatpak", "install", "--system", "--noninteractive", "-y", "flathub", "org.openrgb.OpenRGB"]));
                    if o.ok { Ok(()) } else { Err(o.why()) }
                }),
                |r| Msg::Done(r.unwrap_or_else(|| Err("stopped".into()))),
            );
        }
        Msg::StartIt => {
            a.link = Link::Busy;
            a.said = "Starting OpenRGB…".into();
            return Task::perform(
                kit::bg(|| {
                    if kit::user_unit("start", "hearth-openrgb.service") {
                        kit::user_unit("enable", "hearth-openrgb.service");
                        Ok(())
                    } else {
                        Err("OpenRGB couldn't be started.".to_string())
                    }
                }),
                |r| Msg::Done(r.unwrap_or_else(|| Err("stopped".into()))),
            )
            .chain(Task::perform(run_after(3000), |_| Msg::Done(Ok(()))))
            .chain(load());
        }
        Msg::Hints(h) => a.hints = h,
        Msg::AllowI2c => {
            return Task::perform(
                kit::bg(|| {
                    let o = hearth_ui::run::blocking(&hearth_ui::run::args(&["pkexec", "/usr/libexec/nimbus/hearth-lights-admin", "i2c"]));
                    if o.ok { Ok(()) } else { Err(format!("The one-time OK didn't go through: {}", o.why())) }
                }),
                |r| Msg::Done(r.unwrap_or_else(|| Err("stopped".into()))),
            )
            .chain(Task::done(Msg::LookAgain));
        }
        Msg::Done(r) => {
            if let Err(e) = r {
                a.said = e;
                a.link = if installed() { Link::NotRunning } else { Link::Missing };
            } else if a.link == Link::Busy && a.said.starts_with("Getting") {
                a.said.clear();
                return update(a, Msg::StartIt);
            }
        }
        Msg::Drag => return win::drag(),
        Msg::Max => return win::toggle_maximize(),
        Msg::Min => return win::minimize(),
        Msg::Close => return win::close(),
        Msg::Resize(d) => return win::resize(d),
    }
    Task::none()
}

fn run_after(ms: u64) -> impl std::future::Future<Output = ()> + Send + 'static {
    hearth_ui::run::after(ms)
}

/// The device the Zones card shows: the picked part if it has several zones, else the first fans.
fn zone_device(a: &App) -> Option<usize> {
    if let Some(g) = a.group(&a.picked) {
        if let Some(i) = g.members.first() {
            if a.devs.get(*i).map(|d| d.zones.len() > 1 || d.zones.first().map(|z| z.leds > 1).unwrap_or(false)).unwrap_or(false) {
                return Some(*i);
            }
        }
    }
    a.devs.iter().position(|d| d.kind == kind::COOLER && d.name.to_lowercase().contains("fan") && d.zones.len() > 1).or_else(|| a.devs.iter().position(|d| d.zones.len() > 1))
}

/// A device's lights as they look now (from the choices; what the runner sends).
fn lights_of(a: &App, i: usize) -> Vec<Rgb> {
    let d = &a.devs[i];
    let k = fx::key_of(d);
    if a.ch.all_off || a.ch.off.contains(&k) {
        return vec![[0x2a, 0x24, 0x20]; d.leds.len().max(1)];
    }
    let l = a.ch.look_for(&k, &a.profile_look());
    let mut l = Look { brightness: l.brightness.max(35), ..l };
    if l.effect == "off" {
        l.brightness = 0;
    }
    fx::frame(d, &l, 0.0, &fx::Inputs { temp: 55.0, level: 0.7, game: Vec::new() }, a.ch.painted_of(&k).as_ref())
}

// ------------------------------------------------------------------ the window

fn side_row<'a>(p: &Pal, on: bool, lead: E<'a, Msg>, name: String, sub: String, end: Option<E<'a, Msg>>, m: Msg) -> E<'a, Msg> {
    let mut r = row![lead, column![t(name, 13.0, p.text), t(sub, 11.0, p.muted)].spacing(2.0).width(Length::Fill)].spacing(10.0).align_y(iced::Center);
    if let Some(e) = end {
        r = r.push(e);
    }
    let body = container(r).padding(pad(9.0, 10.0, 9.0, 10.0)).width(Length::Fill);
    let el: E<'a, Msg> = if on {
        container(row![container(Space::new()).width(Length::Fixed(3.0)).height(Length::Fill).style(ui::boxed(p.accent, 0.0, None)), body].height(Length::Shrink))
            .clip(true)
            .style(ui::boxed(Color::from_rgb8(0x3b, 0x2a, 0x1d), 9.0, None))
            .into()
    } else {
        body.into()
    };
    click(el, Some(m))
}

fn devices_block(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let mut c = Column::new().spacing(2.0).push(container(caps(&p, format!("Devices · {} found", a.groups.len()))).padding(pad(2.0, 0.0, 8.0, 0.0)));
    let all_dot = container(Space::new()).width(Length::Fixed(12.0)).height(Length::Fixed(12.0)).style(kit::gradient(135.0, rc(fx::EMBER[0]), kit::BLUE, 6.0, None));
    c = c.push(side_row(&p, a.picked.is_empty(), all_dot.into(), "All devices".into(), "one colour or effect for all".into(), None, Msg::Pick(String::new())));
    for g in &a.groups {
        let on = !a.is_off(g) && !a.ch.all_off;
        c = c.push(side_row(&p, a.picked == g.key, dot(rc(g.dot), 12.0), g.label.clone(), g.sub.clone(), Some(switch(&p, on, Some(Msg::Toggle(g.key.clone())))), Msg::Pick(g.key.clone())));
    }
    if a.link != Link::Ok && a.groups.is_empty() {
        c = c.push(container(t(if a.link == Link::Looking { "Looking for lights…" } else { "None yet: OpenRGB isn't answering." }, 12.0, p.muted)).padding(pad(6.0, 10.0, 6.0, 10.0)));
    }
    scrollable(c).height(Length::Fill).into()
}

fn notseen_block(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let mut why = Column::new().spacing(8.0);
    for h in &a.hints {
        let mut r = Column::new().spacing(6.0).push(t(h.words.clone(), 12.0, p.text).line_height(1.5));
        if h.allow {
            r = r.push(chip(&p, "Give the one-time OK", Some(Msg::AllowI2c)));
        }
        why = why.push(container(r).padding(10.0).width(Length::Fill).style(ui::boxed(p.panel2, 8.0, Some((p.line, 1.0)))));
    }
    column![
        caps(&p, "Not seen yet?"),
        t("Some lights need their makers' apps closed or a one-time OK. Hearth explains each one.", 12.0, p.muted).line_height(1.6),
        why,
        chip(&p, "Look again", Some(Msg::LookAgain)),
        click(row![ui::glyph(G::Pencil, p.muted, 12.0, 1.8), t("Edit layout", 12.0, p.muted)].spacing(6.0).align_y(iced::Center), Some(Msg::Describe(hearth_ui::arrange::edit_layout()))),
    ]
    .spacing(10.0)
    .into()
}

fn picture(a: &App) -> E<'_, Msg> {
    let mut parts = Vec::new();
    let mut fans = 0usize;
    let mut sticks = 0usize;
    for (i, d) in a.devs.iter().enumerate() {
        let gkey = a.groups.iter().find(|g| g.members.contains(&i)).map(|g| g.key.clone()).unwrap_or_default();
        let lights = lights_of(a, i);
        match d.kind {
            kind::DRAM if sticks < 4 => {
                parts.push(stage::Part { key: gkey, what: stage::PartKind::Ram(sticks), lights });
                sticks += 1;
            }
            kind::GPU if !parts.iter().any(|p: &stage::Part| p.what == stage::PartKind::Gpu) => parts.push(stage::Part { key: gkey, what: stage::PartKind::Gpu, lights }),
            kind::COOLER if d.name.to_lowercase().contains("fan") => {
                for (zi, z) in d.zones.iter().enumerate() {
                    if fans >= 4 {
                        break;
                    }
                    let s = d.zone_start(zi);
                    let l: Vec<Rgb> = (0..z.leds as usize).map(|x| lights.get(s + x).copied().unwrap_or([0x40, 0x38, 0x32])).collect();
                    parts.push(stage::Part { key: gkey.clone(), what: stage::PartKind::Fan(fans), lights: l });
                    fans += 1;
                }
            }
            kind::COOLER if !parts.iter().any(|p: &stage::Part| p.what == stage::PartKind::Cooler) => parts.push(stage::Part { key: gkey, what: stage::PartKind::Cooler, lights }),
            _ => {}
        }
    }
    // the fans count the board shows: every fan OpenRGB knows of
    let all_fans: usize = a.devs.iter().filter(|d| d.kind == kind::COOLER && d.name.to_lowercase().contains("fan")).map(|d| d.zones.len()).sum();
    canvas(stage::Case { parts, picked: a.picked.clone(), fans: all_fans.max(fans), sticks: a.devs.iter().filter(|d| d.kind == kind::DRAM).count() }).width(Length::Fill).height(Length::Fill).into()
}

fn desk_picture(a: &App) -> E<'_, Msg> {
    let find = |k: i32| a.devs.iter().position(|d| d.kind == k);
    let gk = |i: usize| a.groups.iter().find(|g| g.members.contains(&i)).map(|g| g.key.clone()).unwrap_or_default();
    let keys = find(kind::KEYBOARD).map(|i| (gk(i), lights_of(a, i)));
    let mouse = find(kind::MOUSE).map(|i| {
        let l = lights_of(a, i);
        (gk(i), l.first().copied().unwrap_or([0xff, 0x8a, 0x3d]), l.last().copied().unwrap_or([0xb0, 0x4f, 0xc8]))
    });
    canvas(stage::Desk { keys, mouse, picked: a.picked.clone() }).width(Length::Fixed(455.0)).height(Length::Fixed(180.0)).into()
}

fn strip_card(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let Some(i) = a.devs.iter().position(|d| matches!(d.kind, kind::LEDSTRIP | kind::LIGHT)) else { return Space::new().into() };
    let g = a.groups.iter().find(|g| g.members.contains(&i));
    let lights = lights_of(a, i);
    let stops: Vec<Color> = (0..6).map(|k| rc(lights.get(k * lights.len().max(1) / 6).copied().unwrap_or([0xff, 0x8a, 0x3d]))).collect();
    let bar = container(Space::new()).width(Length::Fill).height(Length::Fixed(10.0)).style(kit::gradient_n(stops, 5.0));
    let inner = column![
        row![dot(kit::YELLOW, 16.0), tb(g.map(|g| g.label.clone()).unwrap_or_default(), 12.0, p.text), Space::new().width(Length::Fill), t(g.map(|g| g.sub.replace(", ", " · ").replace("USB · ", "")).map(|s| format!("{s} · USB")).unwrap_or_default(), 12.0, p.muted)].spacing(10.0).align_y(iced::Center),
        bar
    ]
    .spacing(12.0);
    let key = g.map(|g| g.key.clone()).unwrap_or_default();
    click(card(&p, inner).width(Length::Fixed(455.0)), Some(Msg::Pick(key)))
}

fn now_block(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let l = a.look();
    let pf = a.ch.profile(&a.ch.current);
    let n = a.groups.len();
    let who = match a.group(&a.picked) {
        Some(g) => format!("On {} only", g.label),
        None => format!("On all {n} device{}", if n == 1 { "" } else { "s" }),
    };
    let in_step = a.picked.is_empty() && a.ch.own.is_empty() && l.effect != "off";
    let mut line = vec![who];
    if in_step {
        line.push("in step".into());
    }
    if l.effect != "off" {
        line.push(format!("speed {}%", l.speed));
        line.push(format!("brightness {}%", l.brightness));
    }
    let saved = !a.ways.is_empty() && a.ways.iter().all(|w| w.1 != Way::No);
    let mut tags = Row::new().spacing(8.0);
    if a.ch.all_off {
        tags = tags.push(tag(&p, "All lights off", Tag::Warn));
    } else if in_step {
        tags = tags.push(tag(&p, "In step", Tag::Ok));
    }
    if saved {
        tags = tags.push(tag(&p, "Saved to the devices", Tag::Dark));
    }
    tags = tags.push(tag(&p, format!("Profile: {}", pf.name), Tag::Blue));
    let mut c = column![
        tb(fx::effect_name(&l.effect), 26.0, p.text),
        t(line.join(" · "), 14.0, p.muted),
        container(tags).padding(pad(10.0, 0.0, 0.0, 0.0)),
        container(t("Click any light in the picture to change just that part. Lights stay as you set them after a restart.", 12.0, p.muted).line_height(1.6)).padding(pad(10.0, 0.0, 0.0, 0.0)),
    ]
    .spacing(6.0)
    .width(Length::Fixed(460.0));
    if !a.said.is_empty() {
        c = c.push(t(a.said.clone(), 12.0, kit::YELLOW));
    }
    c.into()
}

fn missing_block(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let busy = a.link == Link::Busy;
    match a.link {
        Link::Missing => kit::missing(&p, "OpenRGB isn't on this PC yet", "Hearth Lights is built on OpenRGB (GPL-2), which talks to every light. Get it from Flathub (it's free and open source), then Hearth starts it for you, only while you use your lights.", Some(("Get OpenRGB".into(), Msg::GetIt)), busy),
        Link::Looking => kit::missing(&p, "Looking for your lights…", "Asking OpenRGB which lights this PC has.", None, true),
        _ => kit::missing(&p, "OpenRGB isn't running", "Start it and Hearth keeps it running in the background for your lights (never part of starting the PC: if it fails, nothing else waits for it).", Some(("Start OpenRGB".into(), Msg::StartIt)), busy),
    }
}

fn zones_block(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let Some(di) = zone_device(a) else {
        return card(&p, column![tb("Zones", 18.0, p.text), t("A part with several zones (fans, strips) shows its lights here.", 12.0, p.muted)].spacing(8.0)).height(Length::Fill).into();
    };
    let d = &a.devs[di];
    let label = a.groups.iter().find(|g| g.members.contains(&di)).map(|g| g.label.clone()).unwrap_or_else(|| d.name.clone());
    let fanlike = d.name.to_lowercase().contains("fan");
    let same = d.zones.iter().all(|z| z.leds == d.zones[0].leds);
    let words = if same {
        format!("{} {} · {} lights each · click to pick, drag to paint", d.zones.len(), if fanlike { "fans" } else { "zones" }, d.zones[0].leds)
    } else {
        format!("{} zones · click to pick, drag to paint", d.zones.len())
    };
    let lights = lights_of(a, di);
    let rows: Vec<(String, Vec<Rgb>)> = d
        .zones
        .iter()
        .enumerate()
        .map(|(zi, z)| {
            let s = d.zone_start(zi);
            (z.name.clone(), (0..(z.leds as usize).min(48)).map(|x| lights.get(s + x).copied().unwrap_or([0x40, 0x38, 0x32])).collect())
        })
        .collect();
    let h = rows.len() as f32 * 26.0;
    let zm = a.ch.zone_mode.as_str();
    let s = seg(&p, vec![("Same for all".into(), zm == "same", Some(Msg::ZoneMode("same"))), ("Each fan".into(), zm == "fan", Some(Msg::ZoneMode("fan"))), ("Each light".into(), zm == "light", Some(Msg::ZoneMode("light")))]);
    let head = row![tb(format!("Zones · {label}"), 18.0, p.text), t(words, 12.0, p.muted), Space::new().width(Length::Fill), s].spacing(10.0).align_y(iced::Center);
    card(&p, column![head, canvas(stage::Zones { rows }).width(Length::Fill).height(Length::Fixed(h.max(26.0)))].spacing(16.0)).height(Length::Fill).into()
}

fn colour_block(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let l = a.look();
    let mut sw = Row::new().spacing(6.0);
    for s in SWATCHES {
        let on = s == l.colour;
        sw = sw.push(click(kit::square(rc(s), 22.0, 6.0, Some((if on { Color::WHITE } else { Color::from_rgba8(255, 255, 255, 0.2) }, 2.0))), Some(Msg::Wheel(s))));
    }
    let field = text_input("#FF8A3D", &a.hex).on_input(Msg::Hex).size(12.0).padding(pad(8.0, 10.0, 8.0, 10.0)).style(kit::field_style(&p));
    column![
        ts("Colour", 18.0, p.text),
        row![canvas(stage::Wheel { at: None, colour: l.colour }).width(Length::Fixed(150.0)).height(Length::Fixed(150.0)), column![field, sw].spacing(10.0).width(Length::Fill)].spacing(16.0).align_y(iced::Center),
    ]
    .spacing(12.0)
    .into()
}

fn fx_tile<'a>(p: &Pal, id: &'static str, name: &'static str, on: bool) -> E<'a, Msg> {
    let swatch: Box<dyn Fn(&iced::Theme) -> container::Style> = match id {
        "still" => Box::new(ui::boxed(p.accent, 4.0, None)),
        "breathing" => Box::new(kit::gradient_n(vec![Color::TRANSPARENT, p.accent, Color::TRANSPARENT], 4.0)),
        "ember" => Box::new(kit::gradient_n(vec![rc(fx::EMBER[0]), rc(fx::EMBER[1]), rc(fx::EMBER[2])], 4.0)),
        "rainbow" => Box::new(kit::gradient_n(vec![Color::from_rgb(1.0, 0.0, 0.0), Color::from_rgb(1.0, 1.0, 0.0), Color::from_rgb(0.0, 1.0, 0.0), Color::from_rgb(0.0, 1.0, 1.0), Color::from_rgb(0.0, 0.0, 1.0), Color::from_rgb(1.0, 0.0, 1.0)], 4.0)),
        "keys" => Box::new(kit::gradient_n(vec![kit::BLUE, Color::TRANSPARENT, kit::BLUE, Color::TRANSPARENT, kit::BLUE, Color::TRANSPARENT, kit::BLUE], 1.0)),
        "heat" => Box::new(kit::gradient_n(vec![kit::GREEN, kit::YELLOW, kit::RED], 4.0)),
        "music" => Box::new(kit::gradient_n(vec![kit::PINK, Color::TRANSPARENT, kit::PINK, Color::TRANSPARENT, kit::PINK, Color::TRANSPARENT, kit::PINK, Color::TRANSPARENT], 1.0)),
        "game" => Box::new(kit::gradient_n(vec![Color::from_rgb8(0x3a, 0x32, 0x2c), Color::from_rgb8(0xa5, 0x9a, 0x91)], 4.0)),
        _ => Box::new(ui::boxed(Color::from_rgb8(0x2a, 0x24, 0x20), 4.0, None)),
    };
    let bar = container(Space::new()).width(Length::Fixed(44.0)).height(Length::Fixed(8.0)).style(swatch);
    let tile = container(column![bar, t(name, 12.0, p.text)].spacing(6.0).align_x(iced::Center))
        .center_x(Length::Fill)
        .center_y(Length::Fixed(62.0))
        .style(ui::boxed(if on { Color::from_rgb8(0x3b, 0x2a, 0x1d) } else { p.panel2 }, 10.0, Some((if on { p.accent } else { p.line }, 1.0))));
    click(tile, Some(Msg::Effect(id)))
}


fn effect_block(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let l = a.look();
    let mut c = Column::new().spacing(8.0).push(caps(&p, "Effect"));
    for r in 0..3 {
        let mut rw = Row::new().spacing(8.0);
        for k in 0..3 {
            let (id, name) = fx::EFFECTS[r * 3 + k];
            rw = rw.push(container(fx_tile(&p, id, name, l.effect == id)).width(Length::FillPortion(1)));
        }
        c = c.push(rw);
    }
    c.into()
}

fn tuning_block(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let l = a.look();
    let sl = |v: u8, m: fn(u8) -> Msg| -> E<'static, Msg> { slider(0.0..=100.0, v as f32, move |x| m(x.round() as u8)).on_release(Msg::Released).width(Length::Fixed(170.0)).style(kit::slider_style(&p)).into() };
    let num = |v: u8| -> E<'static, Msg> { container(t(format!("{v} %"), 12.0, p.muted)).width(Length::Fixed(44.0)).align_x(iced::alignment::Horizontal::Right).into() };
    let dir = seg(&p, fx::DIRECTIONS.iter().map(|(id, s)| (s.to_string(), l.direction == *id, Some(Msg::Dir(*id)))).collect());
    column![
        row![t("Speed", 13.0, p.text), Space::new().width(Length::Fill), sl(l.speed, Msg::Speed), num(l.speed)].spacing(10.0).align_y(iced::Center),
        row![t("Brightness", 13.0, p.text), Space::new().width(Length::Fill), sl(l.brightness, Msg::Bright), num(l.brightness)].spacing(10.0).align_y(iced::Center),
        row![t("Direction", 13.0, p.text), Space::new().width(Length::Fill), dir].spacing(10.0).align_y(iced::Center),
    ]
    .spacing(10.0)
    .into()
}

fn profiles_block(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let mut c = Column::new().push(container(caps(&p, "Profiles")).padding(pad(0.0, 0.0, 4.0, 0.0)));
    let n = a.ch.profiles.len();
    for (i, pf) in a.ch.profiles.iter().enumerate() {
        let on = pf.id == a.ch.current;
        let name: E<'_, Msg> = if on { tb(pf.name.clone(), 13.0, p.text).into() } else { t(pf.name.clone(), 13.0, p.text).into() };
        let colour = if pf.id == "sleep" { Color::from_rgb8(0xa5, 0x9a, 0x91) } else { rc(PROFILE_DOTS[i % PROFILE_DOTS.len()]) };
        let rw = row![dot(colour, 16.0), name, Space::new().width(Length::Fill), t(pf.when_words(), 12.0, p.muted)].spacing(10.0).align_y(iced::Center);
        c = c.push(click(container(rw).padding(pad(9.0, 0.0, 9.0, 0.0)).width(Length::Fill), Some(Msg::Profile(pf.id.clone()))));
        if i + 1 < n {
            c = c.push(ui::hline(Color::from_rgb8(0x2c, 0x26, 0x22)));
        }
    }
    let by = match a.ch.switch.as_str() {
        "yes" => "Switch by itself: yes",
        "no" => "Switch by itself: no",
        _ => "Switch by itself: ask me",
    };
    c = c.push(container(row![chip(&p, "＋ New profile", Some(Msg::NewProfile)), chip(&p, by, Some(Msg::SwitchBy))].spacing(8.0)).padding(pad(12.0, 0.0, 0.0, 0.0)));
    c.into()
}

// ------------------------------------------------------------------ its blocks (hearth_ui::blocks)
const BLOCKS: &[PageDef] = &[PageDef {
    id: "main",
    title: "Hearth Lights",
    blocks: &[
        bl("devices", "Devices", 0, 0, 2, 9),
        bl("notseen", "Not seen yet?", 0, 9, 2, 3),
        bl("picture", "The PC's lights", 2, 0, 4, 8),
        bl("now", "What's on now", 6, 0, 4, 3),
        bl("desk", "Keyboard, mouse and strip", 6, 3, 4, 5),
        bl("zones", "Zones", 2, 8, 8, 4),
        bl("colour", "Colour", 10, 0, 2, 3),
        bl("effect", "Effect", 10, 3, 2, 3),
        bl("tuning", "Speed, brightness, direction", 10, 6, 2, 2),
        bl("profiles", "Profiles", 10, 8, 2, 4),
    ],
}];

const KIT: Kit = Kit {
    name: "Lights",
    buttons: &[
        tile("all-off", "All lights off", G::Power),
        tile("look-again", "Look again", G::Search),
        tile("still", "Still", G::Palette),
        tile("ember", "Ember wave", G::Palette),
        tile("rainbow", "Rainbow", G::Palette),
        tile("off", "Effect: off", G::Moon),
        tile("gaming", "Gaming profile", G::Pad),
        tile("desk", "Desk profile", G::Display),
        tile("movie", "Movie night", G::Moon),
        tile("new-profile", "New profile", G::Plus),
    ],
    panes: &[tile("devices", "The devices", G::List), tile("profiles", "The profiles", G::Columns), tile("effects", "The effects", G::Palette)],
    info: &[tile("found", "Devices found", G::Info), tile("effect", "The effect", G::Palette), tile("profile", "The profile", G::User), tile("brightness", "Brightness", G::Display)],
    pane_page: "main",
    bars: &[],
    menus: &[],
};

fn block_action(id: &str) -> Option<Msg> {
    Some(match id {
        "all-off" => Msg::AllOff,
        "look-again" => Msg::LookAgain,
        "still" => Msg::Effect("still"),
        "ember" => Msg::Effect("ember"),
        "rainbow" => Msg::Effect("rainbow"),
        "off" => Msg::Effect("off"),
        "gaming" => Msg::Profile("gaming".into()),
        "desk" => Msg::Profile("desk".into()),
        "movie" => Msg::Profile("movie".into()),
        "new-profile" => Msg::NewProfile,
        _ => return None,
    })
}

fn pane(a: &App, which: &str) -> E<'static, Msg> {
    let p = a.p;
    let mut c = Column::new().spacing(6.0);
    match which {
        "devices" => {
            for g in &a.groups {
                c = c.push(click(row![dot(c_(g.dot), 10.0), t(g.label.clone(), 13.0, p.text), Space::new().width(Length::Fill), t(g.sub.clone(), 11.0, p.muted)].spacing(8.0).align_y(iced::Center), Some(Msg::Pick(g.key.clone()))));
            }
        }
        "profiles" => {
            for pf in &a.ch.profiles {
                c = c.push(click(row![t(pf.name.clone(), 13.0, p.text), Space::new().width(Length::Fill), t(pf.when_words(), 11.0, p.muted)].spacing(8.0), Some(Msg::Profile(pf.id.clone()))));
            }
        }
        _ => {
            for (id, name) in fx::EFFECTS {
                c = c.push(click(t(name, 13.0, if a.look().effect == id { p.accent } else { p.text }), Some(Msg::Effect(id))));
            }
        }
    }
    scrollable(c).height(Length::Fill).into()
}
fn c_(x: Rgb) -> Color {
    rc(x)
}

fn view(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let l = a.look();
    let pf = a.ch.profile(&a.ch.current).name.clone();
    a.describe.values(vec![("found", a.groups.len().to_string()), ("effect", fx::effect_name(&l.effect).to_string()), ("profile", pf), ("brightness", format!("{} %", l.brightness))]);
    let ok = a.link == Link::Ok;
    let mut items: Vec<(&'static str, E<'_, Msg>)> = vec![
        ("devices", hearth_ui::studio::made(&a.describe, "main", "devices", || devices_block(a))),
        ("notseen", notseen_block(a)),
        ("picture", if ok { picture(a) } else { container(missing_block(a)).padding(30.0).center_y(Length::Fill).into() }),
        ("now", now_block(a)),
        ("desk", if ok { column![desk_picture(a), strip_card(a)].spacing(30.0).into() } else { Space::new().into() }),
        ("zones", zones_block(a)),
        ("colour", colour_block(a)),
        ("effect", effect_block(a)),
        ("tuning", tuning_block(a)),
        ("profiles", profiles_block(a)),
    ];
    for (id, which) in [("pane:devices", "devices"), ("pane:profiles", "profiles"), ("pane:effects", "effects")] {
        if a.describe.wants_pane(which) {
            items.push((id, pane(a, which)));
        }
    }
    let body = hearth_ui::arrange::parts(
        &a.describe,
        &p,
        "main",
        items,
        move |ps| {
            let side = container(column![container(ps.take("devices")).height(Length::Fill), ps.take("notseen")].spacing(14.0)).width(Length::Fixed(280.0)).height(Length::Fill).padding(pad(16.0, 12.0, 16.0, 12.0)).style(ui::boxed(Color::from_rgba8(20, 17, 15, 0.5), 0.0, None));
            let right_of_case = column![ps.take("now"), Space::new().height(Length::Fill), ps.take("desk")].spacing(10.0).width(Length::Fixed(470.0)).height(Length::Fill);
            let stage_inner = row![container(ps.take("picture")).width(Length::Fill).height(Length::Fill), right_of_case].spacing(20.0).padding(pad(40.0, 24.0, 40.0, 24.0));
            let stage = container(stage_inner).width(Length::Fill).height(Length::Fill).style(kit::gradient(180.0, Color::from_rgb8(0x2a, 0x1d, 0x15), Color::from_rgb8(0x14, 0x11, 0x10), 14.0, Some((p.line, 1.0))));
            let main = column![stage, container(ps.take("zones")).height(Length::Fixed(170.0))].spacing(16.0).padding(pad(20.0, 24.0, 20.0, 24.0)).width(Length::Fill);
            let panel = container(scrollable(column![ps.take("colour"), ps.take("effect"), ps.take("tuning"), ps.take("profiles")].spacing(14.0)).height(Length::Fill))
                .width(Length::Fixed(400.0))
                .height(Length::Fill)
                .padding(pad(18.0, 20.0, 18.0, 20.0))
                .style(ui::boxed(Color::from_rgba8(20, 17, 15, 0.5), 0.0, None));
            row![side, ui::vline(p.line), main, ui::vline(p.line), panel].height(Length::Fill).into()
        },
        Msg::Describe,
    );
    let off_label = if a.ch.all_off { "Lights back on" } else { "All lights off" };
    let bar = kit::title_bar(
        &p,
        "LT",
        Color::from_rgb8(0xff, 0x9b, 0xe8),
        kit::PURPLE,
        "Hearth Lights",
        "every light in your PC, one place",
        vec![chip(&p, off_label, Some(Msg::AllOff))],
        TitleMsgs { drag: Msg::Drag, maximize: Msg::Max, minimize: Some(Msg::Min), close: Msg::Close },
        Msg::Describe(hearth_ui::describe::DMsg::Open),
    );
    let all = container(column![bar, container(body).height(Length::Fill), kit::credit(&p, "Hearth Lights", "OpenRGB (GPL-2)")])
        .width(Length::Fill)
        .height(Length::Fill)
        .style(ui::boxed(p.bg, p.window_corner, Some((p.line, 1.0))));
    ui::edges(all.into(), Msg::Resize)
}

fn subscription(_a: &App) -> Subscription<Msg> {
    Subscription::run(hearth_ui::theme::watch).map(|_| Msg::Theme)
}

fn shown(a: &App) -> Element<'_, Msg> {
    hearth_ui::describe::with_bar(view(a), &a.p, &a.describe, Msg::Describe)
}

fn main() -> iced::Result {
    if app::flag("--run") {
        std::process::exit(fx::run_forever());
    }
    if app::handle_cli("hearth-lights-app", "Hearth Lights: every RGB light in your PC, built on OpenRGB.\n  --run        keep the lights as you set them (hearth-lights.service)\n  --pick KEY   open with that device picked") {
        return Ok(());
    }
    let w = app::window("hearth-lights", (1840.0, 970.0), (1100.0, 700.0));
    iced::application(App::new, update, shown)
        .title(|_: &App| "Hearth Lights".to_string())
        .subscription(subscription)
        .theme(|a: &App| app::iced_theme(&a.p))
        .window(w)
        .default_font(app::INTER)
        .run()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dev(kind: i32, name: &str, zones: &[(&str, u32)]) -> Device {
        Device {
            kind,
            name: name.into(),
            location: "HID: /dev/hidraw1".into(),
            zones: zones.iter().map(|(n, l)| orgb::Zone { name: n.to_string(), leds: *l, ..Default::default() }).collect(),
            leds: vec![String::new(); zones.iter().map(|z| z.1 as usize).sum()],
            ..Default::default()
        }
    }

    #[test]
    fn the_board_names_the_devices() {
        let d = vec![dev(kind::DRAM, "Stick 1", &[("Stick", 16)]), dev(kind::DRAM, "Stick 2", &[("Stick", 16)]), dev(kind::COOLER, "Front fans", &[("Fan 1", 24), ("Fan 2", 24), ("Fan 3", 24)]), dev(kind::COOLER, "Pump", &[("Pump ring", 16)]), dev(kind::MOUSE, "M", &[("Logo", 1), ("Wheel", 1)])];
        let g = groups_of(&d);
        assert_eq!(g.len(), 4);
        assert_eq!(g[0].label, "Memory (2 sticks)");
        assert_eq!(g[0].sub, "16 lights each");
        assert_eq!(g[1].sub, "3 × 24 lights");
        assert_eq!(g[2].label, "CPU cooler");
        assert_eq!(g[2].sub, "pump ring, 16 lights");
        assert_eq!(g[3].sub, "logo + wheel");
    }
}
