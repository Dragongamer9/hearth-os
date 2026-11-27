//! hearth-mouse-app: Hearth Mouse (board docs/mockups/open-source-apps/11-mouse.png; checklist
//! docs/mockups/checklists/open-source-11-mouse.md). Your gaming mouse, set up on Hearth, not tied to one maker: built on
//! libratbag (MIT) and its daemon ratbagd, the same as Piper (GPL-2) uses (rb.rs). DPI steps, every button, report
//! rate, angle snapping, its lights and the profiles kept on the mouse itself, all written to the mouse (Commit).
//!
//! libratbag not on this PC: Install libratbag gets Debian's own ratbagd (debs.rs, hearth-admin deb-install); its daemon is started by
//! D-Bus when asked, never at start-up. "Switch with the game" is the runner (`--run`, hearth-mouse.service), only
//! once you change it, and "ask me" by default.
//!
//! $HEARTH_MOUSE_FIXTURE: ratbagd's objects from a file (CI's pictures).

#[path = "../../oss-kit/kit.rs"]
mod kit;
#[path = "../../oss-kit/debs.rs"]
mod debs;
mod hid;
mod pic;
mod rb;

use hearth_ui::app::{self, win};
use hearth_ui::blocks::{bl, PageDef};
use hearth_ui::studio::{tile, Kit};
use hearth_ui::ui::{self, t, tb, ts, TitleMsgs, G};
use hearth_ui::Pal;
use iced::widget::{canvas, column, container, row, scrollable, slider, text_input, Column, Row, Space};
use iced::{Color, Element, Length, Subscription, Task};
use kit::{card, chip, click, pad, seg, switch, tag, Tag, E};
use serde_json::{json, Value};

#[derive(Debug, Clone)]
pub enum Msg {
    Describe(hearth_ui::describe::DMsg),
    Theme,
    Loaded(Result<Vec<rb::Mouse>, String>),
    Wrote(Result<(), String>),
    PickMouse(usize),
    Pop(Pop),
    Pin(String),
    Step(usize),
    Dpi(u32),
    DpiY(u32),
    DpiText(String),
    DpiDone,
    Separate,
    AddStep,
    Action(u32, u32, u32),
    Lines(u32),
    Rate(u32),
    Snap,
    /// Sleep after: Hearth's own write to the mouse (hid.rs), for the models whose protocol is known
    Sleep(u32),
    Slept(u32, Result<(), String>),
    SleepNow(Option<u32>),
    /// Hearth Mouse's udev rule for those models (a small .deb, hearth-admin deb-install)
    Access,
    Accessed(Result<(), String>),
    LedMode(usize, u32),
    LedColour(usize, [u8; 3]),
    DpiColour,
    ToggleDpiColour,
    Follow,
    Profile(usize),
    SwitchBy,
    OpenRemap,
    StartIt,
    /// Debian's own libratbag (ratbagd), installed from here (debs.rs)
    InstallIt,
    Drag,
    Max,
    Min,
    Close,
    Resize(iced::window::Direction),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Pop {
    None,
    Mice,
    Button(u32),
    Wheel,
    Colour(usize),
    Sleep,
}

struct App {
    describe: hearth_ui::describe::Describe,
    p: Pal,
    mice: Vec<rb::Mouse>,
    at: usize,
    loading: bool,
    problem: String,
    pop: Pop,
    picked: String,
    step: usize,
    dpi_text: String,
    saved: bool,
    said: String,
    /// the mouse's sleep time (read from the mouse where it can say, else what Hearth set)
    sleep_min: Option<u32>,
    settings: Value,
    lines: u32,
    battery: Option<u32>,
}

const STEP_COLOURS: [u32; 5] = [0x5ec8ff, 0x9be36b, 0xff8a3d, 0xe36bd3, 0xff6b6b];
const SWATCHES: [[u8; 3]; 6] = [[0xff, 0x8a, 0x3d], [0xb0, 0x4f, 0xc8], [0x5e, 0xc8, 0xff], [0x9b, 0xe3, 0x6b], [0xff, 0x3d, 0x6a], [0xff, 0xff, 0xff]];
const APP: &str = "mouse";

fn hc(s: u32) -> Color {
    Color::from_rgb8((s >> 16) as u8, (s >> 8) as u8, s as u8)
}
fn rgb_of(s: u32) -> [u8; 3] {
    [(s >> 16) as u8, (s >> 8) as u8, s as u8]
}

/// The board's pin for each of libratbag's buttons (1 left, 2 right, 3 wheel press, 4 DPI, 5 side front, 6 side back).
fn pin_of(i: u32) -> String {
    match i {
        0 => "1".into(),
        1 => "2".into(),
        2 => "3".into(),
        5 => "4".into(),
        4 => "5".into(),
        3 => "6".into(),
        n => (n + 1).to_string(),
    }
}

/// The desktop's scroll lines (KDE's WheelScrollLines, 3 when it isn't set).
fn scroll_lines() -> u32 {
    let f = std::env::var_os("XDG_CONFIG_HOME").map(std::path::PathBuf::from).unwrap_or_else(|| hearth_ui::paths::home().join(".config")).join("kdeglobals");
    std::fs::read_to_string(f).ok().and_then(|t| t.lines().find(|l| l.trim_start().starts_with("WheelScrollLines=")).and_then(|l| l.split('=').nth(1)).and_then(|v| v.trim().parse().ok())).unwrap_or(3)
}

fn load() -> Task<Msg> {
    Task::perform(kit::bg(rb::mice), |r| Msg::Loaded(r.unwrap_or_else(|| Err("stopped".into()))))
}

impl App {
    fn new() -> (App, Task<Msg>) {
        let a = App {
            describe: hearth_ui::describe::Describe::new("mouse", "Hearth Mouse").with_pages(BLOCKS).with_kit(&KIT),
            p: hearth_ui::theme::load(),
            mice: Vec::new(),
            at: 0,
            loading: true,
            problem: String::new(),
            pop: if app::arg("--show").as_deref() == Some("sleep") { Pop::Sleep } else { Pop::None },
            picked: app::arg("--pin").unwrap_or_default(),
            step: usize::MAX,
            dpi_text: String::new(),
            saved: false,
            said: String::new(),
            sleep_min: None,
            settings: kit::read_settings(APP),
            lines: scroll_lines(),
            battery: None,
        };
        (a, load())
    }
    fn mouse(&self) -> Option<&rb::Mouse> {
        self.mice.get(self.at)
    }
    fn profile(&self) -> Option<&rb::Profile> {
        let m = self.mouse()?;
        m.profiles.iter().find(|p| p.active).or(m.profiles.first())
    }
    fn res(&self) -> Vec<&rb::Res> {
        self.profile().map(|p| p.res.iter().filter(|r| !r.disabled).collect()).unwrap_or_default()
    }
    fn cur_step(&self) -> usize {
        let r = self.res();
        if self.step < r.len() {
            return self.step;
        }
        r.iter().position(|x| x.active).unwrap_or(0)
    }
    fn flag(&self, k: &str, d: bool) -> bool {
        self.settings.get(k).and_then(Value::as_bool).unwrap_or(d)
    }
    fn keep(&mut self, k: &str, v: Value) {
        self.settings[k] = v.clone();
        if let Err(e) = kit::save(APP, vec![(k, v)]) {
            self.said = format!("Couldn't keep that: {e}");
        }
    }
    /// A change on ratbagd's objects, then written to the mouse (Commit), then read back.
    fn write(&mut self, f: impl FnOnce() -> Result<(), String> + Send + 'static) -> Task<Msg> {
        let Some(m) = self.mouse().map(|m| m.path.clone()) else { return Task::none() };
        self.saved = false;
        Task::perform(kit::bg(move || f().and_then(|_| rb::commit(&m))), |r| Msg::Wrote(r.unwrap_or_else(|| Err("stopped".into())))).chain(load())
    }
    /// Show the DPI step's colour on the logo, or Hearth Lights' colour on every light (the person's own switches).
    fn lights_follow(&mut self) -> Task<Msg> {
        let Some(p) = self.profile().cloned() else { return Task::none() };
        if self.flag("followLights", false) {
            let lv = kit::read_settings("lights");
            let cur = lv.get("current").and_then(Value::as_str).unwrap_or("gaming").to_string();
            let look = lv.get("profiles").and_then(Value::as_array).and_then(|a| a.iter().find(|x| x.get("id").and_then(Value::as_str) == Some(cur.as_str()))).and_then(|x| x.get("look")).cloned().unwrap_or(Value::Null);
            let colour = look.get("colour").and_then(Value::as_str).and_then(kit::hex).map(|c| {
                let [r, g, b, _] = c.into_rgba8();
                [r, g, b]
            });
            let colour = colour.unwrap_or([0xff, 0x8a, 0x3d]);
            let mode = match look.get("effect").and_then(Value::as_str).unwrap_or("ember") {
                "off" => 0,
                "breathing" => 3,
                "still" | "keys" => 1,
                _ => 2,
            };
            let leds = p.leds.clone();
            return self.write(move || {
                for l in &leds {
                    rb::set_led(l, if l.modes.is_empty() || l.modes.contains(&mode) { mode } else { 1 }, colour)?;
                }
                Ok(())
            });
        }
        if self.flag("dpiColour", true) {
            let i = self.cur_step();
            if let Some(l) = p.leds.first().cloned() {
                let c = rgb_of(STEP_COLOURS[i % 5]);
                if l.colour != c {
                    let mode = l.mode.max(1);
                    return self.write(move || rb::set_led(&l, mode, c));
                }
            }
        }
        Task::none()
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
        Msg::Loaded(r) => {
            a.loading = false;
            match r {
                Ok(m) => {
                    a.problem.clear();
                    a.mice = m;
                    if a.at >= a.mice.len() {
                        a.at = 0;
                    }
                    let name = a.mouse().map(|m| m.name.clone()).unwrap_or_default();
                    a.battery = rb::battery(&name);
                    a.dpi_text = a.res().get(a.cur_step()).map(|r| format!("{} DPI", r.x)).unwrap_or_default();
                    // a mouse Hearth can ask itself: its sleep time now
                    if let Some(k) = a.mouse().and_then(|x| hid::known(&x.model)).filter(|_| std::env::var_os("HEARTH_MOUSE_FIXTURE").is_none()) {
                        return Task::perform(kit::bg(move || hid::sleep_now(&k)), |r| Msg::SleepNow(r.flatten()));
                    }
                }
                Err(e) => a.problem = e,
            }
        }
        Msg::Wrote(r) => match r {
            Ok(()) => {
                a.saved = true;
                a.said.clear();
            }
            Err(e) => a.said = format!("The mouse didn't take it: {e}"),
        },
        Msg::PickMouse(i) => {
            a.at = i;
            a.pop = Pop::None;
            a.step = usize::MAX;
        }
        Msg::Pop(p) => a.pop = if a.pop == p { Pop::None } else { p },
        Msg::Pin(p) => a.picked = p,
        Msg::Step(i) => {
            a.step = i;
            let r = a.res().get(i).map(|r| (*r).clone());
            if let Some(r) = r {
                a.dpi_text = format!("{} DPI", r.x);
                return a.write(move || rb::call(&r.path, "Resolution", "SetActive")).chain(Task::done(Msg::DpiColour));
            }
        }
        Msg::Dpi(v) | Msg::DpiY(v) if a.res().is_empty() => {
            let _ = v;
        }
        Msg::Dpi(v) => {
            let i = a.cur_step();
            a.dpi_text = format!("{v} DPI");
            if let Some(p) = a.mice.get_mut(a.at).and_then(|m| m.profiles.iter_mut().find(|p| p.active)) {
                if let Some(r) = p.res.iter_mut().filter(|r| !r.disabled).nth(i) {
                    let sep = r.pair && r.x != r.y;
                    r.x = v;
                    if !sep {
                        r.y = v;
                    }
                }
            }
        }
        Msg::DpiY(v) => {
            let i = a.cur_step();
            if let Some(p) = a.mice.get_mut(a.at).and_then(|m| m.profiles.iter_mut().find(|p| p.active)) {
                if let Some(r) = p.res.iter_mut().filter(|r| !r.disabled).nth(i) {
                    r.y = v;
                }
            }
        }
        Msg::DpiText(s) => a.dpi_text = s,
        Msg::DpiDone => {
            let i = a.cur_step();
            let typed: Option<u32> = a.dpi_text.chars().filter(|c| c.is_ascii_digit()).collect::<String>().parse().ok();
            let Some(r) = a.res().get(i).map(|r| (*r).clone()) else { return Task::none() };
            let snap = |v: u32| r.choices.iter().copied().min_by_key(|c| (*c as i64 - v as i64).abs()).unwrap_or(v);
            let (x, y) = match typed {
                Some(v) if a.dpi_text.contains(char::is_numeric) && a.dpi_text != format!("{} DPI", r.x) => (snap(v), if r.x == r.y { snap(v) } else { r.y }),
                _ => (snap(r.x), snap(r.y)),
            };
            a.dpi_text = format!("{x} DPI");
            return a.write(move || rb::set_resolution(&r, x, y));
        }
        Msg::Separate => {
            let i = a.cur_step();
            let Some(r) = a.res().get(i).map(|r| (*r).clone()) else { return Task::none() };
            if !r.pair {
                a.said = "This mouse moves the same speed up and down as across.".into();
                return Task::none();
            }
            // back to one speed: up and down take the speed across
            let (x, y) = if r.x != r.y { (r.x, r.x) } else { (r.x, r.y) };
            if r.x != r.y {
                return a.write(move || rb::set_resolution(&r, x, y));
            }
            if let Some(p) = a.mice.get_mut(a.at).and_then(|m| m.profiles.iter_mut().find(|p| p.active)) {
                if let Some(rr) = p.res.iter_mut().filter(|r| !r.disabled).nth(i) {
                    rr.y = rr.choices.iter().copied().filter(|c| *c < rr.x).max().unwrap_or(rr.x);
                }
            }
        }
        Msg::AddStep => {
            let off = a.profile().and_then(|p| p.res.iter().find(|r| r.disabled).cloned());
            match off {
                Some(r) => return a.write(move || rb::set_disabled(&r, false)),
                None => a.said = "This mouse has no more steps to add.".into(),
            }
        }
        Msg::Action(i, kind, value) => {
            a.pop = Pop::None;
            let b = a.profile().and_then(|p| p.buttons.iter().find(|b| b.index == i).cloned());
            if let Some(b) = b {
                return a.write(move || rb::set_mapping(&b, kind, value));
            }
        }
        Msg::Lines(n) => {
            a.pop = Pop::None;
            a.lines = n;
            if std::env::var("HEARTH_MOUSE_FIXTURE").is_err() {
                let o = hearth_ui::run::blocking(&hearth_ui::run::args(&["kwriteconfig6", "--file", "kdeglobals", "--group", "KDE", "--key", "WheelScrollLines", &n.to_string()]));
                a.said = if o.ok { format!("The wheel scrolls {n} lines (the desktop's setting, for every mouse).") } else { format!("The desktop didn't take it: {}", o.why()) };
            }
        }
        Msg::Rate(hz) => {
            if let Some(p) = a.profile().cloned() {
                return a.write(move || rb::set_rate(&p, hz));
            }
        }
        Msg::Sleep(m) => {
            a.pop = Pop::None;
            let Some(k) = a.mouse().and_then(|x| hid::known(&x.model)) else { return Task::none() };
            if std::env::var_os("HEARTH_MOUSE_FIXTURE").is_some() {
                return update(a, Msg::Slept(m, Ok(())));
            }
            return Task::perform(kit::bg(move || hid::set_sleep(&k, m)), move |r| Msg::Slept(m, r.unwrap_or_else(|| Err("stopped".into()))));
        }
        Msg::Slept(m, r) => match r {
            Ok(()) => {
                a.sleep_min = Some(m);
                a.said = format!("The mouse sleeps after {m} min without moving.");
            }
            Err(e) => a.said = format!("The mouse didn't take it: {e}"),
        },
        Msg::SleepNow(m) => a.sleep_min = m.or(a.sleep_min),
        Msg::Access => {
            a.said = "Asking for the administrator's password to let Hearth Mouse set this mouse…".into();
            return Task::perform(kit::bg(|| {
                let deb = debs::build("hearth-mouse-access", "1", "Hearth Mouse: the person at the PC may set the sleep time of known mice", &[("/usr/lib/udev/rules.d/70-hearth-mouse.rules", hid::udev_rule(), 0o644)], "udevadm control --reload-rules || true
udevadm trigger --subsystem-match=hidraw || true")?;
                debs::install(&deb)
            }), |r| Msg::Accessed(r.unwrap_or_else(|| Err("stopped".into()))));
        }
        Msg::Accessed(r) => {
            a.said = match r {
                Ok(()) => "Hearth Mouse can set this mouse now.".into(),
                Err(e) => format!("It wasn't allowed: {e}"),
            };
            if let Some(k) = a.mouse().and_then(|x| hid::known(&x.model)) {
                return Task::perform(kit::bg(move || hid::sleep_now(&k)), |r| Msg::SleepNow(r.flatten()));
            }
        }
        Msg::Snap => {
            if let Some(p) = a.profile().cloned() {
                if p.snapping >= 0 {
                    let on = p.snapping == 0;
                    return a.write(move || rb::set_snapping(&p, on));
                }
                a.said = "This mouse can't snap to straight lines.".into();
            }
        }
        Msg::LedMode(i, mode) => {
            if let Some(l) = a.profile().and_then(|p| p.leds.get(i).cloned()) {
                let c = l.colour;
                return a.write(move || rb::set_led(&l, mode, c));
            }
        }
        Msg::LedColour(i, c) => {
            a.pop = Pop::None;
            if let Some(l) = a.profile().and_then(|p| p.leds.get(i).cloned()) {
                let mode = l.mode.max(1);
                return a.write(move || rb::set_led(&l, mode, c));
            }
        }
        Msg::DpiColour => return a.lights_follow(),
        Msg::ToggleDpiColour => {
            let v = !a.flag("dpiColour", true);
            a.keep("dpiColour", json!(v));
            return a.lights_follow();
        }
        Msg::Follow => {
            let v = !a.flag("followLights", false);
            a.keep("followLights", json!(v));
            return a.lights_follow();
        }
        Msg::Profile(i) => {
            let p = a.mouse().and_then(|m| m.profiles.get(i).cloned());
            if let Some(p) = p {
                a.step = usize::MAX;
                let n = i + 1;
                return a.write(move || {
                    if !p.enabled {
                        rb::set_enabled(&p, true)?;
                        if p.name.is_empty() {
                            rb::set_name(&p, &format!("Profile {n}"))?;
                        }
                    }
                    rb::call(&p.path, "Profile", "SetActive")
                });
            }
        }
        Msg::SwitchBy => {
            let s = match a.settings.get("switch").and_then(Value::as_str).unwrap_or("ask") {
                "ask" => "yes",
                "yes" => "no",
                _ => "ask",
            };
            a.keep("switch", json!(s));
            if std::env::var("HEARTH_MOUSE_FIXTURE").is_err() {
                if s == "no" {
                    kit::user_unit("disable", "hearth-mouse.service");
                    kit::user_unit("stop", "hearth-mouse.service");
                } else {
                    kit::user_unit("enable", "hearth-mouse.service");
                    kit::user_unit("start", "hearth-mouse.service");
                }
            }
        }
        Msg::OpenRemap => {
            let name = a.mouse().map(|m| m.name.clone()).unwrap_or_default();
            hearth_ui::run::detached(&["hearth-remap-app", "--device", &name]);
        }
        Msg::InstallIt => {
            a.said = "Getting Debian's libratbag…".into();
            return Task::perform(kit::bg(|| debs::ratbagd()), |r| Msg::Wrote(r.unwrap_or_else(|| Err("stopped".into())))).chain(load());
        }
        Msg::StartIt => {
            a.said = "Starting libratbag's ratbagd…".into();
            return Task::perform(
                kit::bg(|| {
                    let o = hearth_ui::run::blocking(&hearth_ui::run::args(&["pkexec", "systemctl", "start", "ratbagd.service"]));
                    if o.ok { Ok(()) } else { Err(o.why()) }
                }),
                |r| Msg::Wrote(r.unwrap_or_else(|| Err("stopped".into()))),
            )
            .chain(load());
        }
        Msg::Drag => return win::drag(),
        Msg::Max => return win::toggle_maximize(),
        Msg::Min => return win::minimize(),
        Msg::Close => return win::close(),
        Msg::Resize(d) => return win::resize(d),
    }
    Task::none()
}

// ------------------------------------------------------------------ the window

fn field<'a>(p: &Pal, s: String, w: f32, m: Option<Msg>) -> E<'a, Msg> {
    let p = *p;
    click(
        container(row![t(s, 12.0, p.text), Space::new().width(Length::Fill), t("▾", 12.0, p.muted)].align_y(iced::Center)).padding(pad(6.0, 10.0, 6.0, 10.0)).width(Length::Fixed(w)).style(ui::boxed(p.bg, 8.0, Some((p.line, 1.0)))),
        m,
    )
}

fn mouse_block(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let Some(m) = a.mouse() else {
        let words = if a.loading {
            "Asking libratbag which mice are here…".to_string()
        } else if !rb::installed() {
            return kit::missing(&p, "libratbag isn't on this PC yet", "Hearth Mouse is built on libratbag (MIT), which knows how to talk to gaming mice from many makers, like Piper does. Install Debian's own libratbag from here (it asks for the administrator's password once; its daemon starts only when Hearth Mouse asks it something).", Some(("Install libratbag".into(), Msg::InstallIt)), false);
        } else if !a.problem.is_empty() {
            return kit::missing(&p, "libratbag isn't answering", format!("Its daemon, ratbagd, didn't answer ({}). Start it (it asks once, as administrator); it's never part of starting the PC.", a.problem), Some(("Start ratbagd".into(), Msg::StartIt)), false);
        } else {
            "No gaming mouse libratbag knows is plugged in. Plug one in (or its USB receiver) and it shows here.".to_string()
        };
        return container(column![tb("Your mouse", 26.0, p.text), t(words, 13.0, p.muted)].spacing(8.0)).padding(pad(22.0, 28.0, 22.0, 28.0)).into();
    };
    let pr = a.profile();
    let buttons = pr.map(|p| p.buttons.len()).unwrap_or(0) + 1;
    let max = pr.map(|p| p.res.iter().flat_map(|r| r.choices.iter().copied()).max().unwrap_or(0)).unwrap_or(0);
    let wireless = a.battery.is_some() || m.name.to_lowercase().contains("receiver") || m.name.to_lowercase().contains("wireless") || m.name.to_lowercase().contains("lightspeed");
    let mut line = vec![format!("{buttons} buttons")];
    if max > 0 {
        line.push(format!("{} DPI sensor", kit::commas(max as u64)));
    }
    if wireless {
        line.push("wireless".into());
    }
    if let Some(b) = a.battery {
        line.push(format!("battery {b}%"));
    }
    let short = m.name.split(" (").next().unwrap_or(&m.name).to_string();
    let head = row![column![tb(short, 26.0, p.text), t(line.join(" · "), 12.0, p.muted)].spacing(4.0).width(Length::Fill), if a.saved || a.said.is_empty() { tag(&p, "Saved on the mouse", Tag::Ok) } else { tag(&p, "Not saved yet", Tag::Warn) }].align_y(iced::Alignment::Start);
    let led_c = |kind_want: u32, idx: usize| -> Option<[u8; 3]> { pr.and_then(|p| p.leds.iter().find(|l| l.kind == kind_want).or(p.leds.get(idx))).filter(|l| l.mode != 0).map(|l| l.colour) };
    let mut pins: Vec<String> = pr.map(|p| p.buttons.iter().map(|b| pin_of(b.index)).filter(|x| x.len() == 1 && x.as_str() <= "6").collect()).unwrap_or_default();
    if pr.map(|p| !p.leds.is_empty()).unwrap_or(false) {
        pins.push("L".into());
    }
    let picture = canvas(pic::Pic { picked: a.picked.clone(), logo: led_c(1, 0), strip: if pr.map(|p| p.leds.len() > 1).unwrap_or(false) { led_c(2, 1) } else { None }, pins }).width(Length::Fill).height(Length::Fill);
    let foot = row![t("Click a number or press the mouse's button to pick it", 12.0, p.muted), Space::new().width(Length::Fill), t("Not tied to one maker", 12.0, p.muted)];
    let mut c = column![head, container(picture).height(Length::Fill).padding(pad(30.0, 0.0, 10.0, 0.0)), foot].spacing(6.0);
    if !a.said.is_empty() {
        c = c.push(t(a.said.clone(), 12.0, kit::YELLOW));
    }
    container(c).padding(pad(22.0, 24.0, 24.0, 28.0)).height(Length::Fill).into()
}

fn dpi_block(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let res = a.res();
    let i = a.cur_step();
    let mut tiles = Row::new().spacing(12.0);
    for (k, r) in res.iter().enumerate() {
        let on = k == i;
        let label = if r.active { format!("Step {} · now", k + 1) } else { format!("Step {}", k + 1) };
        let inner = column![t(label, 12.0, p.muted), tb(r.x.to_string(), 24.0, p.text), container(Space::new()).width(Length::Fixed(28.0)).height(Length::Fixed(6.0)).style(ui::boxed(hc(STEP_COLOURS[k % 5]), 3.0, None))].spacing(6.0).align_x(iced::Center);
        let tile = container(inner).center_x(Length::Fill).center_y(Length::Fixed(118.0)).style(ui::boxed(if on { Color::from_rgb8(0x3b, 0x2a, 0x1d) } else { p.panel2 }, 12.0, Some((if on { p.accent } else { p.line }, if on { 2.0 } else { 1.0 }))));
        tiles = tiles.push(container(click(tile, Some(Msg::Step(k)))).width(Length::FillPortion(1)));
    }
    let head = row![ts("How fast it moves (DPI)", 18.0, p.text), t(format!("{} steps · the DPI button goes through them", res.len()), 12.0, p.muted), Space::new().width(Length::Fill), chip(&p, "＋ Step", Some(Msg::AddStep))].spacing(10.0).align_y(iced::Center);
    let mut c = Column::new().spacing(14.0).push(head).push(tiles);
    if let Some(r) = res.get(i) {
        let (lo, hi) = (r.choices.iter().copied().min().unwrap_or(100) as f32, r.choices.iter().copied().max().unwrap_or(26000).max(200) as f32);
        let sl = |v: u32, m: fn(u32) -> Msg| -> E<'static, Msg> { slider(lo..=hi, (v as f32).clamp(lo, hi), move |x| m(x.round() as u32)).on_release(Msg::DpiDone).step(50.0).width(Length::Fixed(520.0)).style(kit::slider_style(&p)).into() };
        let sep = r.pair && r.x != r.y;
        let box_ = text_input("1600 DPI", &a.dpi_text).on_input(Msg::DpiText).on_submit(Msg::DpiDone).size(12.0).width(Length::Fixed(90.0)).padding(pad(7.0, 10.0, 7.0, 10.0)).style(kit::field_style(&p));
        c = c.push(row![t(if sep { format!("Step {} · across", i + 1) } else { format!("Step {}", i + 1) }, 13.0, p.text), Space::new().width(Length::Fill), sl(r.x, Msg::Dpi), box_].spacing(10.0).align_y(iced::Center));
        if sep {
            c = c.push(row![t(format!("Step {} · up and down", i + 1), 13.0, p.text), Space::new().width(Length::Fill), sl(r.y, Msg::DpiY), container(t(format!("{} DPI", r.y), 12.0, p.muted)).width(Length::Fixed(90.0))].spacing(10.0).align_y(iced::Center));
        }
        c = c.push(kit::line(&p, "Up and down separately", switch(&p, sep, Some(Msg::Separate))));
    } else {
        c = c.push(t("This profile has no DPI steps the mouse lets libratbag change.", 12.0, p.muted));
    }
    card(&p, c).into()
}

fn buttons_block(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let mut c = Column::new().push(row![ts("Buttons", 18.0, p.text), Space::new().width(Length::Fill), t("this profile", 12.0, p.muted)].align_y(iced::Center));
    let mut bs: Vec<&rb::Button> = a.profile().map(|p| p.buttons.iter().collect()).unwrap_or_default();
    bs.sort_by_key(|b| pin_of(b.index).parse::<u32>().unwrap_or(99));
    let num = |s: String, on: bool| -> E<'static, Msg> {
        container(tb(s, 12.0, if on { p.ink } else { p.text })).center_x(Length::Fixed(26.0)).center_y(Length::Fixed(26.0)).style(ui::boxed(if on { p.accent } else { p.bg }, 13.0, Some((if on { p.accent } else { p.line }, 1.0)))).into()
    };
    let mut rows = Column::new();
    for b in bs {
        let pin = pin_of(b.index);
        let on = pin == a.picked;
        let r = row![num(pin.clone(), on), container(t(rb::button_name(b.index), 13.0, p.text)).width(Length::Fill), field(&p, rb::action_words(b.kind, b.value), 274.0, Some(Msg::Pop(Pop::Button(b.index))))].spacing(12.0).align_y(iced::Center);
        rows = rows.push(click(container(r).padding(pad(9.0, 0.0, 9.0, 0.0)), Some(Msg::Pin(pin))));
        if a.pop == Pop::Button(b.index) {
            let mut opts = Column::new().spacing(5.0);
            for (k, v, w) in rb::ACTIONS {
                if b.kinds.is_empty() || b.kinds.contains(&k) || k == 0 {
                    opts = opts.push(click(t(w, 12.0, if b.kind == k && b.value == v { p.accent } else { p.text }), Some(Msg::Action(b.index, k, v))));
                }
            }
            rows = rows.push(container(opts).padding(10.0).width(Length::Fill).style(ui::boxed(p.panel2, 8.0, Some((p.line, 1.0)))));
        }
        rows = rows.push(ui::hline(Color::from_rgb8(0x2c, 0x26, 0x22)));
    }
    let wheel = row![num("↕".into(), false), container(t("Wheel", 13.0, p.text)).width(Length::Fill), field(&p, format!("Scroll · {} lines", a.lines), 274.0, Some(Msg::Pop(Pop::Wheel)))].spacing(12.0).align_y(iced::Center);
    rows = rows.push(container(wheel).padding(pad(9.0, 0.0, 9.0, 0.0)));
    if a.pop == Pop::Wheel {
        let mut r = Row::new().spacing(6.0);
        for n in [1, 2, 3, 4, 5, 6, 8, 10] {
            r = r.push(chip(&p, format!("{n}"), Some(Msg::Lines(n))));
        }
        rows = rows.push(r);
    }
    c = c.push(container(rows).padding(pad(6.0, 0.0, 0.0, 0.0)));
    c = c.push(container(row![t("More: any key, a macro from Hearth Remap, a Hearth action", 12.0, p.muted), Space::new().width(Length::Fill), chip(&p, "Hearth Remap ›", Some(Msg::OpenRemap))].spacing(8.0).align_y(iced::Center)).padding(pad(12.0, 0.0, 0.0, 0.0)));
    card(&p, scrollable(c).height(Length::Fill)).height(Length::Fill).into()
}

fn rate_block(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let pr = a.profile();
    let rates = pr.map(|p| p.rates.clone()).filter(|r| !r.is_empty()).unwrap_or_else(|| vec![125, 500, 1000, 2000, 4000]);
    let cur = pr.map(|p| p.rate).unwrap_or(0);
    let s = seg(&p, rates.iter().map(|hz| (format!("{hz} Hz"), *hz == cur, Some(Msg::Rate(*hz)))).collect());
    let snap = pr.map(|p| p.snapping).unwrap_or(-1);
    // Lift-off distance: no maker's protocol for it is known well enough to write (hid.rs), so it says so;
    // Sleep after: Hearth's own write for the mice whose protocol is known (hid.rs), else it says so
    let lod: E<'_, Msg> = row![seg(&p, vec![("Low".into(), false, None), ("Mid".into(), false, None), ("High".into(), false, None)]), t("not on this mouse from Linux", 11.0, p.muted)].spacing(8.0).align_y(iced::Center).into();
    let known = a.mouse().and_then(|m| hid::known(&m.model));
    let sleep: E<'_, Msg> = match known {
        Some(k) => {
            let mut c = Column::new().spacing(6.0).push(row![field(&p, a.sleep_min.map(|m| format!("{m} min ▾")).unwrap_or_else(|| "5 min ▾".into()), 70.0, Some(Msg::Pop(Pop::Sleep)))].spacing(6.0).align_y(iced::Center));
            if a.pop == Pop::Sleep {
                let mut r = Row::new().spacing(6.0);
                for m in hid::choices(k.kind) {
                    r = r.push(chip(&p, format!("{m} min"), Some(Msg::Sleep(*m))));
                }
                c = c.push(r).push(row![t("No access to it yet?", 11.0, p.muted), chip(&p, "Let Hearth Mouse set it", Some(Msg::Access))].spacing(6.0).align_y(iced::Center));
            }
            c.into()
        }
        None => row![field(&p, "5 min".into(), 70.0, None), t("this mouse can't be told from Linux", 11.0, p.muted)].spacing(8.0).align_y(iced::Center).into(),
    };
    column![
        ts("Reports per second", 18.0, p.text),
        t("Higher is smoother, uses a bit more battery", 12.0, p.muted),
        container(s).padding(pad(8.0, 0.0, 4.0, 0.0)),
        kit::line(&p, "Lift-off distance", lod),
        kit::line(&p, "Angle snapping", switch(&p, snap > 0, if snap >= 0 { Some(Msg::Snap) } else { None })),
        kit::line(&p, "Sleep after", sleep),
    ]
    .spacing(8.0)
    .into()
}

fn lights_block(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let mut c = Column::new().spacing(10.0).push(ts("Lights", 18.0, p.text));
    let leds = a.profile().map(|p| p.leds.clone()).unwrap_or_default();
    for (i, l) in leds.iter().enumerate() {
        let name = match (l.kind, i) {
            (1, _) | (0, 0) => "Logo".to_string(),
            (2, _) | (0, 1) => "Bottom strip".to_string(),
            (5, _) => "Wheel".to_string(),
            (4, _) => "DPI light".to_string(),
            _ => format!("Light {}", i + 1),
        };
        let cbox = click(kit::square(Color::from_rgb8(l.colour[0], l.colour[1], l.colour[2]), 22.0, 6.0, Some((if i == 0 { Color::WHITE } else { Color::from_rgba8(255, 255, 255, 0.2) }, 2.0))), Some(Msg::Pop(Pop::Colour(i))));
        let has = |m: u32| l.modes.is_empty() || l.modes.contains(&m);
        let modes = seg(&p, vec![("Still".into(), l.mode == 1, has(1).then_some(Msg::LedMode(i, 1))), ("Breathing".into(), l.mode == 3, has(3).then_some(Msg::LedMode(i, 3))), ("Wave".into(), l.mode == 2, has(2).then_some(Msg::LedMode(i, 2)))]);
        c = c.push(kit::line(&p, name, row![cbox, modes].spacing(6.0).align_y(iced::Center).into()));
        if a.pop == Pop::Colour(i) {
            let mut r = Row::new().spacing(6.0);
            for s in SWATCHES {
                r = r.push(click(kit::square(Color::from_rgb8(s[0], s[1], s[2]), 20.0, 5.0, None), Some(Msg::LedColour(i, s))));
            }
            c = c.push(r);
        }
    }
    if leds.is_empty() {
        c = c.push(t("This mouse has no lights libratbag can change.", 12.0, p.muted));
    }
    c = c.push(kit::line(&p, "Show the DPI step colour", switch(&p, a.flag("dpiColour", true), Some(Msg::ToggleDpiColour))));
    c = c.push(kit::line(&p, "Follow Hearth Lights", switch(&p, a.flag("followLights", false), Some(Msg::Follow))));
    c.into()
}

fn profiles_block(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let ps = a.mouse().map(|m| m.profiles.clone()).unwrap_or_default();
    let by = match a.settings.get("switch").and_then(Value::as_str).unwrap_or("ask") {
        "yes" => "Switch with the game: yes",
        "no" => "Switch with the game: no",
        _ => "Switch with the game: ask me",
    };
    let head = row![ts("Profiles on the mouse", 18.0, p.text), t(format!("{} kept on the mouse itself, so they work on any PC", ps.len()), 12.0, p.muted), Space::new().width(Length::Fill), chip(&p, by, Some(Msg::SwitchBy))].spacing(10.0).align_y(iced::Center);
    let mut r = Row::new().spacing(10.0);
    for (i, pr) in ps.iter().enumerate() {
        let name = if !pr.enabled { "empty".to_string() } else if pr.name.is_empty() { format!("Profile {}", i + 1) } else { pr.name.clone() };
        let words = format!("{} · {name}", i + 1);
        let tile: E<'_, Msg> = if pr.active {
            container(ts(words, 13.0, p.ink)).center_x(Length::Fill).center_y(Length::Fixed(44.0)).style(ui::boxed(p.accent, 10.0, None)).into()
        } else if !pr.enabled {
            container(t(words, 13.0, p.muted)).center_x(Length::Fill).center_y(Length::Fixed(44.0)).style(ui::boxed(Color::TRANSPARENT, 10.0, Some((p.line, 1.0)))).into()
        } else {
            container(t(words, 13.0, p.text)).center_x(Length::Fill).center_y(Length::Fixed(44.0)).style(ui::boxed(p.panel2, 10.0, Some((p.line, 1.0)))).into()
        };
        r = r.push(container(click(tile, Some(Msg::Profile(i)))).width(Length::FillPortion(1)));
    }
    card(&p, column![head, r].spacing(12.0)).into()
}

// ------------------------------------------------------------------ its blocks
const BLOCKS: &[PageDef] = &[PageDef {
    id: "main",
    title: "Hearth Mouse",
    blocks: &[
        bl("mouse", "The mouse", 0, 0, 4, 12),
        bl("dpi", "How fast it moves (DPI)", 4, 0, 8, 4),
        bl("buttons", "Buttons", 4, 4, 5, 6),
        bl("rate", "Reports per second", 9, 4, 3, 3),
        bl("lights", "Lights", 9, 7, 3, 3),
        bl("profiles", "Profiles on the mouse", 4, 10, 8, 2),
    ],
}];

const KIT: Kit = Kit {
    name: "Mouse",
    buttons: &[
        tile("add-step", "Add a DPI step", G::Plus),
        tile("next-profile", "Next profile", G::Arrow),
        tile("remap", "Hearth Remap", G::Pad),
        tile("switch", "Switch with the game", G::Sync),
        tile("follow", "Follow Hearth Lights", G::Palette),
    ],
    panes: &[tile("steps", "The DPI steps", G::List), tile("buttons", "The buttons", G::Mouse)],
    info: &[tile("dpi", "DPI now", G::Mouse), tile("profile", "The profile", G::User), tile("battery", "Battery", G::Power), tile("rate", "Reports per second", G::Pulse)],
    pane_page: "main",
    bars: &[],
    menus: &[],
};

fn block_action(id: &str) -> Option<Msg> {
    Some(match id {
        "add-step" => Msg::AddStep,
        "remap" => Msg::OpenRemap,
        "switch" => Msg::SwitchBy,
        "follow" => Msg::Follow,
        "next-profile" => Msg::Pop(Pop::None),
        _ => return None,
    })
}

fn pane(a: &App, which: &str) -> E<'static, Msg> {
    let p = a.p;
    let mut c = Column::new().spacing(6.0);
    if which == "steps" {
        for (k, r) in a.res().iter().enumerate() {
            c = c.push(click(row![t(format!("Step {}", k + 1), 13.0, p.text), Space::new().width(Length::Fill), t(format!("{} DPI", r.x), 12.0, p.muted)].spacing(8.0), Some(Msg::Step(k))));
        }
    } else if let Some(pr) = a.profile() {
        for b in &pr.buttons {
            c = c.push(click(row![t(rb::button_name(b.index), 13.0, p.text), Space::new().width(Length::Fill), t(rb::action_words(b.kind, b.value), 12.0, p.muted)].spacing(8.0), Some(Msg::Pin(pin_of(b.index)))));
        }
    }
    scrollable(c).height(Length::Fill).into()
}

fn view(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let dpi_now = a.res().iter().find(|r| r.active).map(|r| format!("{} DPI", r.x)).unwrap_or_default();
    a.describe.values(vec![("dpi", dpi_now), ("profile", a.profile().map(|p| p.name.clone()).unwrap_or_default()), ("battery", a.battery.map(|b| format!("{b}%")).unwrap_or_default()), ("rate", a.profile().map(|p| format!("{} Hz", p.rate)).unwrap_or_default())]);
    let has = a.mouse().is_some();
    let mut items: Vec<(&'static str, E<'_, Msg>)> = vec![
        ("mouse", hearth_ui::studio::made(&a.describe, "main", "mouse", || mouse_block(a))),
        ("dpi", if has { dpi_block(a) } else { Space::new().into() }),
        ("buttons", if has { buttons_block(a) } else { Space::new().into() }),
        ("rate", if has { card(&p, rate_block(a)).into() } else { Space::new().into() }),
        ("lights", if has { card(&p, lights_block(a)).into() } else { Space::new().into() }),
        ("profiles", if has { profiles_block(a) } else { Space::new().into() }),
    ];
    for (id, which) in [("pane:steps", "steps"), ("pane:buttons", "buttons")] {
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
            let left = container(ps.take("mouse")).width(Length::Fixed(640.0)).height(Length::Fill).style(kit::gradient(180.0, Color::from_rgb8(0x23, 0x1a, 0x14), Color::from_rgb8(0x16, 0x12, 0x10), 0.0, None));
            let grid = row![container(ps.take("buttons")).width(Length::FillPortion(13)).height(Length::Fill), column![ps.take("rate"), ps.take("lights")].spacing(16.0).width(Length::FillPortion(10))].spacing(16.0).height(Length::Fill);
            let right = column![ps.take("dpi"), grid, ps.take("profiles")].spacing(16.0).padding(pad(22.0, 28.0, 22.0, 28.0)).width(Length::Fill);
            row![left, ui::vline(p.line), right].height(Length::Fill).into()
        },
        Msg::Describe,
    );
    let pick = match a.mouse() {
        Some(m) => {
            let short = m.name.split(" (").next().unwrap_or(&m.name).to_string();
            let via = if m.name.to_lowercase().contains("receiver") || a.battery.is_some() { " · USB receiver" } else { " · USB" };
            format!("{short}{via}")
        }
        None => "No mouse yet".into(),
    };
    let right: Vec<E<'_, Msg>> = vec![field(&p, pick, 210.0, Some(Msg::Pop(Pop::Mice)))];
    let bar = kit::title_bar(
        &p,
        "MO",
        Color::from_rgb8(0xb8, 0xf0, 0x7f),
        Color::from_rgb8(0x3b, 0xa5, 0x5c),
        "Hearth Mouse",
        "your gaming mouse, set up on Hearth",
        right,
        TitleMsgs { drag: Msg::Drag, maximize: Msg::Max, minimize: Some(Msg::Min), close: Msg::Close },
        Msg::Describe(hearth_ui::describe::DMsg::Open),
    );
    let mut base: E<'_, Msg> = container(column![bar, container(body).height(Length::Fill), row![container(click(row![ui::glyph(G::Pencil, p.muted, 12.0, 1.8), t("Edit layout", 11.0, p.muted)].spacing(6.0).align_y(iced::Center), Some(Msg::Describe(hearth_ui::arrange::edit_layout())))).padding(pad(5.0, 0.0, 0.0, 22.0)), kit::credit(&p, "Hearth Mouse", "Piper (GPL-2) and libratbag (MIT)")]])
        .width(Length::Fill)
        .height(Length::Fill)
        .style(ui::boxed(p.bg, p.window_corner, Some((p.line, 1.0))))
        .into();
    if a.pop == Pop::Mice {
        let mut l = Column::new().spacing(8.0).push(ts("Your mice", 15.0, p.text));
        for (i, m) in a.mice.iter().enumerate() {
            l = l.push(click(t(m.name.clone(), 13.0, if i == a.at { p.accent } else { p.text }), Some(Msg::PickMouse(i))));
        }
        base = ui::dialog(base, ui::dialog_box(&p, 360.0, l), Some(Msg::Pop(Pop::Mice)));
    }
    ui::edges(base, Msg::Resize)
}

fn subscription(_a: &App) -> Subscription<Msg> {
    Subscription::run(hearth_ui::theme::watch).map(|_| Msg::Theme)
}

fn shown(a: &App) -> Element<'_, Msg> {
    hearth_ui::describe::with_bar(view(a), &a.p, &a.describe, Msg::Describe)
}

/// `--run` (hearth-mouse.service): when a game starts, the mouse's profile of the same name (Switch with the game: yes,
/// or ask me and you said yes). Never writes your settings; it looks every 2 seconds.
fn run_forever() -> i32 {
    let mut last = String::new();
    loop {
        std::thread::sleep(std::time::Duration::from_secs(2));
        let s = kit::read_settings(APP);
        let how = s.get("switch").and_then(Value::as_str).unwrap_or("ask").to_string();
        if how == "no" {
            continue;
        }
        let home = hearth_ui::paths::home();
        let reg = std::fs::read_to_string(home.join(".steam/registry.vdf")).unwrap_or_default();
        let id = reg.lines().find(|l| l.contains("\"RunningAppID\"")).and_then(|l| l.split('"').filter(|x| !x.trim().is_empty()).nth(1).map(str::to_string)).unwrap_or_default();
        if id == last {
            continue;
        }
        last = id.clone();
        if id.is_empty() || id == "0" {
            continue;
        }
        let manifest = ["/.local/share/Steam/steamapps", "/.steam/steam/steamapps"].iter().map(|r| format!("{}{r}/appmanifest_{id}.acf", home.display())).find_map(|f| std::fs::read_to_string(f).ok()).unwrap_or_default();
        let game = manifest.lines().find(|l| l.trim_start().starts_with("\"name\"")).and_then(|l| l.split('"').nth(3)).unwrap_or("").to_lowercase();
        let Ok(mice) = rb::mice() else { continue };
        for m in mice {
            let Some(pr) = m.profiles.iter().find(|p| p.enabled && !p.name.is_empty() && (game.contains(&p.name.to_lowercase()) || initials(&game) == p.name.to_lowercase().replace(' ', ""))) else { continue };
            if pr.active {
                continue;
            }
            if how == "ask" {
                let o = std::process::Command::new("notify-send").args(["-a", "Hearth Mouse", "-A", "yes=Switch", "-A", "no=Not now", "-w", "Hearth Mouse", &format!("Switch the mouse to {}?", pr.name)]).stdin(std::process::Stdio::null()).stderr(std::process::Stdio::null()).output();
                if o.map(|o| String::from_utf8_lossy(&o.stdout).trim() != "yes").unwrap_or(true) {
                    continue;
                }
            }
            let _ = rb::call(&pr.path, "Profile", "SetActive").and_then(|_| rb::commit(&m.path));
        }
    }
}
/// "battle for middle-earth ii" → "bfmeii" (a profile named BFME II fits it).
fn initials(s: &str) -> String {
    s.split(|c: char| c == ' ' || c == '-').filter(|w| !w.is_empty()).map(|w| if w.chars().all(|c| c == 'i' || c == 'v' || c == 'x') { w.to_string() } else { w.chars().next().map(|c| c.to_string()).unwrap_or_default() }).collect()
}

fn main() -> iced::Result {
    if app::flag("--run") {
        std::process::exit(run_forever());
    }
    if app::handle_cli("hearth-mouse-app", "Hearth Mouse: your gaming mouse, set up on Hearth (built on libratbag, like Piper).\n  --run      switch the mouse's profile with the game (hearth-mouse.service)\n  --pin N    open with button N picked") {
        return Ok(());
    }
    let w = app::window("hearth-mouse", (1840.0, 970.0), (1100.0, 700.0));
    iced::application(App::new, update, shown)
        .title(|_: &App| "Hearth Mouse".to_string())
        .subscription(subscription)
        .theme(|a: &App| app::iced_theme(&a.p))
        .window(w)
        .default_font(app::INTER)
        .run()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_profile_named_for_the_game_fits_it() {
        assert_eq!(initials("battle for middle-earth ii"), "bfmeii");
        assert_eq!(pin_of(5), "4");
        assert_eq!(pin_of(3), "6");
    }
}
