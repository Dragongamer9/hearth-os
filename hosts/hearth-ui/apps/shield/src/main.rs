//! hearth-shield-app: Hearth Shield (board docs/mockups/open-source-apps/15-shield.png; checklist
//! docs/mockups/checklists/open-source-15-shield.md). Ads and trackers blocked for every device at home, built on
//! AdGuard Home (GPL-3: Hearth's own, as Server mode runs it on this PC) or a Pi-hole (EUPL-1.2) on your network
//! (data.rs). Today's numbers, the day hour by hour, every device and its own rules, what was asked (live, with Allow
//! and Block), the block lists, always allow / always block, family rules and where it runs.
//!
//! Shield can never cut a device or the network off: devices use it only while it answers (Hearth devices'
//! hearth-dns-client goes back to normal look-ups within seconds; for the rest, the router's second address), and
//! AdGuard Home is never part of starting the PC (hearth-server-dns.timer starts it a minute after, a broken one only
//! stops itself). A Pi-hole's password is kept only in this PC's keyring.

#[path = "../../oss-kit/kit.rs"]
mod kit;
mod count;
mod data;

use data::{Data, Dev};
use hearth_ui::app::{self, win};
use hearth_ui::blocks::{bl, PageDef};
use hearth_ui::studio::{tile, Kit};
use hearth_ui::ui::{self, t, tb, ts, TitleMsgs, G};
use hearth_ui::Pal;
use iced::widget::canvas::{self, Frame, Geometry, Path};
use iced::widget::{column, container, row, scrollable, text_input, Column, Row, Space};
use iced::{mouse, Color, Element, Length, Point, Rectangle, Renderer, Size, Subscription, Task, Theme};
use kit::{caps, chip, click, commas, pad, seg, switch, tag, Tag, E};
use serde_json::{json, Value};

const APP: &str = "shield";

#[derive(Debug, Clone, Copy, PartialEq)]
enum Page {
    Today,
    Devices,
    Log,
    Lists,
    Allow,
    Block,
    Family,
    Settings,
    Device,
}

impl Page {
    fn of(s: &str) -> Page {
        match s {
            "devices" => Page::Devices,
            "log" => Page::Log,
            "lists" => Page::Lists,
            "allow" => Page::Allow,
            "block" => Page::Block,
            "family" => Page::Family,
            "settings" => Page::Settings,
            "device" => Page::Device,
            _ => Page::Today,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Msg {
    Describe(hearth_ui::describe::DMsg),
    Theme,
    Tick,
    Loaded(Result<Data, String>),
    Done(Result<String, String>),
    Go(Page),
    Range(&'static str),
    OpenDev(String),
    DevRule(String, &'static str),
    DevName(String),
    DevAbout(String),
    DevKeep,
    Pause,
    TurnOn(bool),
    List(String, bool),
    Allow(String),
    Unallow(String),
    Block(String),
    Unblock(String),
    Typed(String),
    AddAllow,
    AddBlock,
    Safe(bool),
    Source(&'static str),
    PiUrl(String),
    PiPass(String),
    PiConnect,
    PiForget,
    Drag,
    Max,
    Min,
    Close,
    Resize(iced::window::Direction),
}

struct App {
    describe: hearth_ui::describe::Describe,
    p: Pal,
    settings: Value,
    d: Data,
    loaded: bool,
    problem: String,
    page: Page,
    range: &'static str,
    dev: String,
    dev_name: String,
    dev_about: String,
    typed: String,
    pi_url: String,
    pi_pass: String,
    said: String,
    busy: bool,
    at: i64,
    server: String,
}

fn this_pc() -> String {
    if let Ok(n) = std::env::var("HEARTH_SHIELD_SERVER") {
        return n;
    }
    let h = std::fs::read_to_string("/etc/hostname").unwrap_or_default().trim().to_string();
    let words: Vec<String> = h
        .split(['-', '_', '.'])
        .filter(|w| !w.is_empty())
        .map(|w| if w.len() <= 3 { w.to_uppercase() } else { format!("{}{}", w[..1].to_uppercase(), &w[1..]) })
        .collect();
    if words.is_empty() { "this PC".into() } else { words.join(" ") }
}

fn load(s: &Value) -> Task<Msg> {
    let names = s.get("names").cloned().unwrap_or(Value::Null);
    let source = s.get("source").and_then(Value::as_str).unwrap_or("here").to_string();
    let url = s.get("pihole").and_then(Value::as_str).unwrap_or("").to_string();
    Task::perform(
        kit::bg(move || {
            if source == "pihole" && !data::is_fixture() {
                let pass = kit::keyring::lookup("shield", "pihole").ok_or("The Pi-hole's password isn't in the keyring yet (Settings).")?;
                let pi = data::PiHole::sign_in(&url, &pass)?;
                return pi.read(&names);
            }
            data::read_here(&names)
        }),
        |r| Msg::Loaded(r.unwrap_or_else(|| Err("stopped".into()))),
    )
}

impl App {
    fn new() -> (App, Task<Msg>) {
        let settings = kit::read_settings(APP);
        let t = load(&settings);
        let a = App {
            describe: hearth_ui::describe::Describe::new("shield", "Hearth Shield").with_pages(BLOCKS).with_kit(&KIT),
            p: hearth_ui::theme::load(),
            pi_url: settings.get("pihole").and_then(Value::as_str).unwrap_or("http://pi.hole").to_string(),
            settings,
            d: Data::default(),
            loaded: false,
            problem: String::new(),
            page: app::arg("--page").map(|p| Page::of(&p)).unwrap_or(Page::Today),
            range: match app::arg("--range").as_deref() {
                Some("7") => "7",
                Some("30") => "30",
                _ => "today",
            },
            dev: app::arg("--device").unwrap_or_default(),
            dev_name: String::new(),
            dev_about: String::new(),
            typed: String::new(),
            pi_pass: String::new(),
            said: String::new(),
            busy: false,
            at: hearth_ui::time::now(),
            server: this_pc(),
        };
        (a, t)
    }
    fn pihole(&self) -> bool {
        self.settings.get("source").and_then(Value::as_str) == Some("pihole")
    }
    /// A change, made where Shield runs, then everything read again.
    fn change(&mut self, args: Vec<String>, pi: Option<(&'static str, String, Option<Value>)>, ok: &str) -> Task<Msg> {
        self.busy = true;
        let ok = ok.to_string();
        let here = !self.pihole();
        let url = self.pi_url.clone();
        let t = Task::perform(
            kit::bg(move || -> Result<String, String> {
                if here {
                    let a: Vec<&str> = args.iter().map(String::as_str).collect();
                    data::helper(&a)?;
                    // blocking on or off: Shield's own counter with it, where this Hearth OS doesn't count (count.rs)
                    if a.first() == Some(&"ads") && !data::is_fixture() {
                        count::set_on(a.get(1) == Some(&"on"))?;
                    }
                } else if let Some((m, path, body)) = pi {
                    if !data::is_fixture() {
                        let pass = kit::keyring::lookup("shield", "pihole").ok_or("The Pi-hole's password isn't in the keyring.")?;
                        data::PiHole::sign_in(&url, &pass)?.act(m, &path, body)?;
                    }
                }
                Ok(ok)
            }),
            |r| Msg::Done(r.unwrap_or_else(|| Err("stopped".into()))),
        );
        t.chain(load(&self.settings))
    }
    fn keep(&mut self, k: &str, v: Value) {
        self.settings[k] = v.clone();
        if let Err(e) = kit::save(APP, vec![(k, v)]) {
            self.said = format!("Couldn't keep that: {e}");
        }
    }
    fn paused_left(&self) -> u64 {
        (self.d.paused_until as i64 - hearth_ui::time::now()).max(0) as u64
    }
}

fn a(v: &[&str]) -> Vec<String> {
    v.iter().map(|x| x.to_string()).collect()
}
fn enc(s: &str) -> String {
    s.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-._~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect()
}

fn update(app: &mut App, m: Msg) -> Task<Msg> {
    let a_ = app;
    match m {
        Msg::Describe(dm) => {
            if let Some(m) = hearth_ui::studio::act(&dm).and_then(block_action) {
                return update(a_, m);
            }
            return a_.describe.update(dm).map(Msg::Describe);
        }
        Msg::Theme => a_.p = hearth_ui::theme::load(),
        Msg::Tick => {
            if !a_.busy {
                return load(&a_.settings);
            }
        }
        Msg::Loaded(r) => {
            a_.loaded = true;
            a_.at = hearth_ui::time::now();
            match r {
                Ok(d) => {
                    a_.d = d;
                    a_.problem.clear();
                }
                Err(e) => a_.problem = e,
            }
        }
        Msg::Done(r) => {
            a_.busy = false;
            match r {
                Ok(s) => a_.said = s,
                Err(e) => a_.said = format!("That didn't work: {e}"),
            }
        }
        Msg::Go(p) => {
            a_.page = p;
            a_.typed.clear();
        }
        Msg::Range(r) => a_.range = r,
        Msg::OpenDev(id) => {
            a_.dev = id.clone();
            let mine = a_.settings.pointer(&format!("/names/{}", id.replace('~', "~0").replace('/', "~1"))).cloned().unwrap_or(Value::Null);
            let d = a_.d.devices.iter().find(|d| d.id == id).cloned().unwrap_or_default();
            a_.dev_name = mine.get("name").and_then(Value::as_str).map(str::to_string).unwrap_or(d.name);
            a_.dev_about = mine.get("about").and_then(Value::as_str).map(str::to_string).unwrap_or(d.about);
            a_.page = Page::Device;
        }
        Msg::DevRule(id, how) => {
            let words = match how {
                "off" => "Blocking is off for this device.",
                "pause" => "Blocking is paused for this device for an hour.",
                _ => "Blocking is on for this device.",
            };
            return a_.change(a(&["device", &id, how]), None, words);
        }
        Msg::DevName(s) => a_.dev_name = s.chars().take(40).collect(),
        Msg::DevAbout(s) => a_.dev_about = s.chars().take(60).collect(),
        Msg::DevKeep => {
            let mut names = a_.settings.get("names").cloned().filter(Value::is_object).unwrap_or_else(|| json!({}));
            names[a_.dev.as_str()] = json!({"name": a_.dev_name.trim(), "about": a_.dev_about.trim()});
            a_.keep("names", names);
            a_.said = "Kept: that's its name in Hearth Shield.".into();
            return load(&a_.settings);
        }
        Msg::Pause => {
            if a_.paused_left() > 0 {
                return a_.change(a(&["ads-pause", "0"]), Some(("POST", "/api/dns/blocking".into(), Some(json!({"blocking": true})))), "Blocking again.");
            }
            return a_.change(a(&["ads-pause", "300"]), Some(("POST", "/api/dns/blocking".into(), Some(json!({"blocking": false, "timer": 300})))), "Paused for 5 minutes: it blocks again by itself.");
        }
        Msg::TurnOn(on) => {
            let v = if on { "on" } else { "off" };
            return a_.change(a(&["ads", v]), Some(("POST", "/api/dns/blocking".into(), Some(json!({"blocking": on})))), if on { "Shield is on." } else { "Shield is off: nothing is blocked." });
        }
        Msg::List(id, on) => {
            let path = format!("/api/lists/{}?type=block", enc(&id));
            return a_.change(a(&["list", &id, if on { "on" } else { "off" }]), Some(("PUT", path, Some(json!({"enabled": on, "type": "block"})))), if on { "That list is on." } else { "That list is off." });
        }
        Msg::Allow(d) => return a_.change(a(&["allow", &d]), Some(("POST", "/api/domains/allow/exact".into(), Some(json!({"domain": d.clone()})))), &format!("{d} is allowed: it's back in seconds, and Hearth remembers it.")),
        Msg::Unallow(d) => return a_.change(a(&["unallow", &d]), Some(("DELETE", format!("/api/domains/allow/exact/{}", enc(&d)), None)), &format!("{d} isn't always allowed any more.")),
        Msg::Block(d) => return a_.change(a(&["block", &d]), Some(("POST", "/api/domains/deny/exact".into(), Some(json!({"domain": d.clone()})))), &format!("{d} is always blocked.")),
        Msg::Unblock(d) => return a_.change(a(&["unblock", &d]), Some(("DELETE", format!("/api/domains/deny/exact/{}", enc(&d)), None)), &format!("{d} isn't always blocked any more.")),
        Msg::Typed(s) => a_.typed = s.chars().filter(|c| !c.is_whitespace()).take(253).collect(),
        Msg::AddAllow => {
            let d = std::mem::take(&mut a_.typed);
            if !d.is_empty() {
                return update(a_, Msg::Allow(d));
            }
        }
        Msg::AddBlock => {
            let d = std::mem::take(&mut a_.typed);
            if !d.is_empty() {
                return update(a_, Msg::Block(d));
            }
        }
        Msg::Safe(on) => return a_.change(a(&["safe-search", if on { "on" } else { "off" }]), None, if on { "Safe search is on for every device." } else { "Safe search is off." }),
        Msg::Source(s) => {
            a_.keep("source", json!(s));
            return load(&a_.settings);
        }
        Msg::PiUrl(s) => a_.pi_url = s.trim().to_string(),
        Msg::PiPass(s) => a_.pi_pass = s,
        Msg::PiConnect => {
            let (url, pass) = (a_.pi_url.clone(), std::mem::take(&mut a_.pi_pass));
            if !url.starts_with("http") {
                a_.said = "The Pi-hole's address starts with http:// or https://".into();
                return Task::none();
            }
            a_.keep("pihole", json!(url.clone()));
            a_.keep("source", json!("pihole"));
            return Task::perform(
                kit::bg(move || -> Result<String, String> {
                    if !pass.is_empty() {
                        data::PiHole::sign_in(&url, &pass)?;
                        kit::keyring::store("shield", "pihole", "Hearth Shield: the Pi-hole's password", &pass)?;
                    }
                    Ok("Connected to the Pi-hole. Its password is kept only in this PC's keyring.".into())
                }),
                |r| Msg::Done(r.unwrap_or_else(|| Err("stopped".into()))),
            )
            .chain(load(&a_.settings));
        }
        Msg::PiForget => {
            kit::keyring::forget("shield", "pihole");
            a_.keep("source", json!("here"));
            a_.said = "The Pi-hole's password is gone from this PC.".into();
            return load(&a_.settings);
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
const MARKS: [(u32, u32); 6] = [(0xff9be8, 0xb04fc8), (0xffb36b, 0xff6a1f), (0xb8f07f, 0x3ba55c), (0x7fd6ff, 0x2c7fd8), (0xb9a2ff, 0x6a4fe0), (0xc9c1ba, 0x6d645d)];
fn hc(s: u32) -> Color {
    Color::from_rgb8((s >> 16) as u8, (s >> 8) as u8, s as u8)
}

fn side(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let item = |pg: Page, label: &'static str, n: String| -> E<'static, Msg> {
        let on = a.page == pg || (pg == Page::Devices && a.page == Page::Device);
        let inner = row![kit::square(if on { p.text } else { p.muted }, 16.0, 4.0, None), t(label, 13.0, if on { p.text } else { p.muted }), Space::new().width(Length::Fill), t(n, 11.0, p.muted)].spacing(10.0).align_y(iced::Center);
        let body = container(inner).padding(pad(0.0, 10.0, 0.0, 10.0)).center_y(Length::Fixed(36.0)).width(Length::Fill);
        let el: E<'static, Msg> = if on {
            container(row![container(Space::new()).width(Length::Fixed(3.0)).height(Length::Fixed(36.0)).style(ui::boxed(p.accent, 0.0, None)), body]).clip(true).style(ui::boxed(Color::from_rgb8(0x3b, 0x2a, 0x1d), 9.0, None)).into()
        } else {
            body.into()
        };
        click(el, Some(Msg::Go(pg)))
    };
    let d = &a.d;
    column![
        item(Page::Today, "Today", String::new()),
        item(Page::Devices, "Devices", d.devices_seen.to_string()),
        item(Page::Log, "What was asked", "live".into()),
        item(Page::Lists, "Block lists", d.lists_n.to_string()),
        item(Page::Allow, "Always allow", d.allow.len().to_string()),
        item(Page::Block, "Always block", d.block.len().to_string()),
        item(Page::Family, "Family rules", String::new()),
        item(Page::Settings, "Settings", String::new()),
        container(caps(&p, "How it works")).padding(pad(14.0, 0.0, 0.0, 0.0)),
        t(format!("Your router hands out the {} as the address book for the whole house, so every device is covered, even ones you can't install apps on.", a.server), 12.0, p.muted).line_height(1.6),
        container(caps(&p, format!("If the {} is off", if a.server.ends_with("PC") { "PC" } else { "server" }))).padding(pad(14.0, 0.0, 0.0, 0.0)),
        t("Devices fall back to the router's normal one by themselves. Nothing stops working.", 12.0, p.muted).line_height(1.6),
        container(click(row![ui::glyph(G::Pencil, p.muted, 12.0, 1.8), t("Edit layout", 12.0, p.muted)].spacing(6.0).align_y(iced::Center), Some(Msg::Describe(hearth_ui::arrange::edit_layout())))).padding(pad(14.0, 0.0, 0.0, 10.0)),
    ]
    .spacing(2.0)
    .into()
}

fn kpi<'a>(p: &Pal, label: &str, value: String, sub: String, hot: bool) -> E<'a, Msg> {
    let (fg, bg, line) = if hot { (p.accent, Color::from_rgb8(0x2a, 0x20, 0x17), Color::from_rgb8(0x5a, 0x3b, 0x22)) } else { (p.text, p.panel, p.line) };
    container(column![tb(label.to_uppercase(), 11.0, if hot { p.accent } else { p.muted }), tb(value, 32.0, fg), t(sub, 12.0, p.muted)].spacing(6.0)).padding(pad(16.0, 18.0, 16.0, 18.0)).width(Length::Fill).style(ui::boxed(bg, 12.0, Some((line, 1.0)))).into()
}

/// The day's (or days') bars: each look-up hour's total in grey, its blocked part in the accent over it.
struct Bars {
    bars: Vec<(u64, u64)>,
    acc: Color,
}
impl canvas::Program<Msg> for Bars {
    type State = ();
    fn draw(&self, _s: &(), renderer: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        let mut f = Frame::new(renderer, b.size());
        let n = self.bars.len().max(1);
        let max = self.bars.iter().map(|x| x.0 + x.1).max().unwrap_or(1).max(1) as f32;
        let w = b.width / n as f32;
        for (i, (allowed, blocked)) in self.bars.iter().enumerate() {
            let total = (*allowed + *blocked) as f32;
            let x = i as f32 * w + w * 0.12;
            let bw = (w * 0.76).max(1.0);
            let h = total / max * (b.height - 2.0);
            f.fill(&Path::rectangle(Point::new(x, b.height - h), Size::new(bw, h)), Color::from_rgb8(0x5a, 0x4c, 0x42));
            let hb = *blocked as f32 / max * (b.height - 2.0);
            f.fill(&Path::rectangle(Point::new(x, b.height - hb), Size::new(bw, hb)), self.acc);
        }
        vec![f.into_geometry()]
    }
}

fn range_data(a: &App) -> (u64, u64, Vec<(u64, u64)>, &'static str) {
    let d = &a.d;
    match a.range {
        "today" => (d.asked, d.blocked, d.hours.clone(), "Today"),
        r => {
            let n = if r == "7" { 7 } else { 30 };
            let mut days: Vec<(String, u64, u64)> = d.days.iter().rev().take(n).cloned().collect();
            days.reverse();
            let asked: u64 = days.iter().map(|x| x.1).sum();
            let blocked: u64 = days.iter().map(|x| x.2).sum();
            (asked, blocked, days.iter().map(|x| (x.1.saturating_sub(x.2), x.2)).collect(), if n == 7 { "The last 7 days" } else { "The last 30 days" })
        }
    }
}

fn today_page(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let d = &a.d;
    let (asked, blocked, bars, title) = range_data(a);
    let ago = (hearth_ui::time::now() - a.at).max(0);
    let since = if a.range == "today" { "Since midnight" } else { "Each day's totals" };
    let upd = if ago <= 2 { "updated a second ago".to_string() } else { format!("updated {ago} seconds ago") };
    let r = a.range;
    let head = row![
        column![tb(title, 26.0, p.text), t(format!("{since} · {upd}"), 12.0, p.muted)].spacing(4.0),
        Space::new().width(Length::Fill),
        seg(&p, vec![("Today".into(), r == "today", Some(Msg::Range("today"))), ("7 days".into(), r == "7", Some(Msg::Range("7"))), ("30 days".into(), r == "30", Some(Msg::Range("30")))])
    ]
    .align_y(iced::Center);
    let share = if asked > 0 { blocked as f64 * 100.0 / asked as f64 } else { 0.0 };
    let addresses = if d.addresses >= 1_000_000 { format!("{:.2} M", d.addresses as f64 / 1e6) } else if d.addresses >= 1000 { format!("{} K", d.addresses / 1000) } else { d.addresses.to_string() };
    let saved_min = (blocked as f64 * 0.2 / 60.0).round() as u64;
    let kpis = row![
        kpi(&p, "Asked", commas(asked), format!("look-ups from {} devices", d.devices_seen), false),
        kpi(&p, "Blocked", commas(blocked), format!("{share:.1}% of everything asked"), true),
        kpi(&p, "Block lists", addresses, format!("addresses in {} lists{}", d.lists_n, if d.lists_at.is_empty() { String::new() } else { format!(" · updated {}", d.lists_at) }), false),
        kpi(&p, "Time saved", format!("~ {saved_min} min"), "of ads not loaded".into(), false),
    ]
    .spacing(14.0);
    let legend = row![t("■", 12.0, Color::from_rgb8(0x5a, 0x4c, 0x42)), t("allowed", 12.0, p.muted), t("■", 12.0, p.accent), t("blocked", 12.0, p.muted)].spacing(5.0).align_y(iced::Center);
    let labels: E<'_, Msg> = if a.range == "today" {
        let hrs = bars.len();
        let mut r = Row::new();
        for (i, l) in ["12 AM", "6 AM", "12 PM", "6 PM"].iter().enumerate() {
            if i * 6 < hrs.max(1) {
                r = r.push(container(t(*l, 12.0, p.muted)).width(Length::FillPortion(6.min((hrs - i * 6).max(1)) as u16)));
            }
        }
        r.push(t("now", 12.0, p.muted)).into()
    } else {
        row![t(format!("{} days ago", bars.len()), 12.0, p.muted), Space::new().width(Length::Fill), t("today", 12.0, p.muted)].into()
    };
    let chart = kit::card(&p, column![row![ts(if a.range == "today" { "Through the day" } else { "Day by day" }, 18.0, p.text), Space::new().width(Length::Fill), legend].align_y(iced::Center), canvas::Canvas::new(Bars { bars, acc: p.accent }).width(Length::Fill).height(Length::Fixed(170.0)), labels].spacing(10.0));
    // the devices that had the most blocked
    let mut devs: Vec<&Dev> = d.devices.iter().collect();
    devs.sort_by(|x, y| y.blocked.cmp(&x.blocked));
    let table = dev_table(a, devs.into_iter().take(6).collect());
    let devices = kit::card(&p, column![row![ts("Devices", 18.0, p.text), Space::new().width(Length::Fill), t("give one its own rules from its page", 12.0, p.muted)].align_y(iced::Center), table].spacing(8.0));
    let mut items: Vec<(&'static str, E<'_, Msg>)> = vec![("head", head.into()), ("kpis", kpis.into()), ("chart", chart.into()), ("devices", devices.into())];
    if a.describe.wants_pane("top") {
        items.push(("pane:top", pane(a, "top")));
    }
    hearth_ui::arrange::parts(&a.describe, &p, "today", items, |ps| scrollable(column![ps.take("head"), ps.take("kpis"), ps.take("chart"), ps.take("devices")].spacing(16.0)).height(Length::Fill).into(), Msg::Describe)
}

fn dev_table<'a>(a: &'a App, devs: Vec<&'a Dev>) -> E<'a, Msg> {
    let p = a.p;
    let hdr = |s: &'static str, w: Length| -> E<'a, Msg> { container(tb(s, 11.0, p.muted)).width(w).into() };
    let mut c = Column::new().push(container(row![hdr("", Length::Fixed(34.0)), hdr("DEVICE", Length::Fill), hdr("ASKED", Length::Fixed(110.0)), hdr("BLOCKED", Length::Fixed(120.0)), hdr("SHARE BLOCKED", Length::Fixed(140.0)), hdr("", Length::Fixed(40.0))].spacing(10.0)).padding(pad(9.0, 0.0, 9.0, 0.0)));
    c = c.push(ui::hline(Color::from_rgb8(0x2c, 0x26, 0x22)));
    for (i, d) in devs.into_iter().enumerate() {
        let (f, to) = MARKS[i % MARKS.len()];
        let letter = d.name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default();
        let mark = container(tb(letter, 9.0, Color::WHITE)).center_x(Length::Fixed(26.0)).center_y(Length::Fixed(26.0)).style(kit::gradient(135.0, hc(f), hc(to), 7.0, None));
        let pct = if d.asked > 0 { (d.blocked as f64 * 100.0 / d.asked as f64).round() as u64 } else { 0 };
        let bar = ui::bar(vec![(pct as f32 / 100.0, p.accent)], p.line, 6.0, 3.0);
        let r = row![
            container(mark).width(Length::Fixed(34.0)),
            container(column![t(d.name.clone(), 13.0, p.text), t(d.about.clone(), 12.0, p.muted)].spacing(1.0)).width(Length::Fill),
            container(t(commas(d.asked), 13.0, p.text)).width(Length::Fixed(110.0)),
            container(t(commas(d.blocked), 13.0, p.accent)).width(Length::Fixed(120.0)),
            container(row![container(bar).width(Length::Fill), t(format!("{pct}%"), 12.0, p.muted)].spacing(8.0).align_y(iced::Center)).width(Length::Fixed(140.0)),
            container(t("›", 13.0, p.muted)).width(Length::Fixed(40.0)).align_x(iced::alignment::Horizontal::Right),
        ]
        .spacing(10.0)
        .align_y(iced::Center);
        c = c.push(click(container(r).padding(pad(9.0, 0.0, 9.0, 0.0)), Some(Msg::OpenDev(d.id.clone()))));
        c = c.push(ui::hline(Color::from_rgb8(0x2c, 0x26, 0x22)));
    }
    c.into()
}

fn panel(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let d = &a.d;
    let mut top = Column::new();
    for (w, who, n) in d.top.iter().take(7) {
        let r = row![column![t(w.clone(), 13.0, p.text), t(who.clone(), 12.0, p.muted)].spacing(1.0).width(Length::Fill), ts(commas(*n), 13.0, p.accent)].spacing(10.0).align_y(iced::Center);
        top = top.push(container(r).padding(pad(8.0, 0.0, 8.0, 0.0)));
        top = top.push(ui::hline(Color::from_rgb8(0x2c, 0x26, 0x22)));
    }
    let top = column![row![ts("Blocked the most", 18.0, p.text), Space::new().width(Length::Fill), t(if a.range == "today" { "today" } else { "lately" }, 12.0, p.muted)].align_y(iced::Center), top].spacing(8.0);
    let mut lists = Column::new().spacing(10.0).push(container(caps(&p, "Block lists")).padding(pad(6.0, 0.0, 2.0, 0.0)));
    for l in &d.lists {
        lists = lists.push(kit::line(&p, l.label.clone(), switch(&p, l.on, Some(Msg::List(l.id.clone(), !l.on)))));
    }
    let tip = container(column![row![kit::dot(p.accent, 16.0), tb("Something broke on a site?", 12.0, p.text)].spacing(10.0).align_y(iced::Center), t("Open What was asked, find it, press Allow. It's back in seconds, and Hearth remembers it.", 12.0, p.muted).line_height(1.6)].spacing(6.0))
        .padding(pad(12.0, 14.0, 12.0, 14.0))
        .width(Length::Fill)
        .style(ui::boxed(Color::from_rgb8(0x2a, 0x20, 0x17), 12.0, Some((Color::from_rgb8(0x5a, 0x3b, 0x22), 1.0))));
    hearth_ui::arrange::parts(&a.describe, &p, "panel", vec![("top", top.into()), ("lists", lists.into()), ("tip", click(tip, Some(Msg::Go(Page::Log))))], |ps| scrollable(column![ps.take("top"), ps.take("lists"), ps.take("tip")].spacing(14.0)).height(Length::Fill).into(), Msg::Describe)
}

/// A page's title, its line, and what just happened.
fn head_of(a: &App, title: String, sub: String) -> E<'_, Msg> {
    let p = a.p;
    let mut head = column![tb(title, 26.0, p.text), t(sub, 12.0, p.muted)].spacing(4.0);
    if !a.said.is_empty() {
        head = head.push(t(a.said.clone(), 12.0, kit::YELLOW));
    }
    head.into()
}

fn devices_page(a: &App) -> E<'_, Msg> {
    let mut devs: Vec<&Dev> = a.d.devices.iter().collect();
    devs.sort_by(|x, y| y.asked.cmp(&x.asked));
    let p = a.p;
    let items = vec![("head", head_of(a, "Devices".into(), format!("{} asked today · give one its own rules from its page", a.d.devices_seen))), ("list", dev_table(a, devs))];
    hearth_ui::arrange::parts(&a.describe, &p, "devices", items, |ps| column![ps.take("head"), scrollable(ps.take("list")).height(Length::Fill)].spacing(16.0).into(), Msg::Describe)
}

fn device_page(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let d = a.d.devices.iter().find(|d| d.id == a.dev).cloned().unwrap_or(Dev { id: a.dev.clone(), name: a.dev.clone(), ..Default::default() });
    let pct = if d.asked > 0 { d.blocked * 100 / d.asked } else { 0 };
    let id = d.id.clone();
    let body = column![
        row![chip(&p, "‹ Devices", Some(Msg::Go(Page::Devices)))],
        row![kpi(&p, "Asked", commas(d.asked), "look-ups today".into(), false), kpi(&p, "Blocked", commas(d.blocked), format!("{pct}% of what it asked"), true)].spacing(14.0),
        caps(&p, "Its name in Hearth Shield"),
        row![text_input("Its name", &a.dev_name).on_input(Msg::DevName).size(13.0).padding(pad(7.0, 10.0, 7.0, 10.0)).style(kit::field_style(&p)), text_input("Where it is, what it is", &a.dev_about).on_input(Msg::DevAbout).size(13.0).padding(pad(7.0, 10.0, 7.0, 10.0)).style(kit::field_style(&p)), kit::chip_acc(&p, "Keep", Some(Msg::DevKeep))].spacing(8.0).align_y(iced::Center),
        caps(&p, "Its own rules"),
        t(format!("Right now: {}", if d.state.is_empty() { "Using it".to_string() } else { d.state.clone() }), 13.0, p.text),
        row![chip(&p, "Block for it", Some(Msg::DevRule(id.clone(), "on"))), chip(&p, "Don't block for it", Some(Msg::DevRule(id.clone(), "off"))), chip(&p, "Pause it for an hour", Some(Msg::DevRule(id, "pause")))].spacing(8.0),
        t("Its rules come before the block lists: a device that must see everything (a work laptop) can be left out.", 12.0, p.muted),
    ]
    .spacing(12.0);
    let items = vec![("head", head_of(a, d.name.clone(), d.about.clone())), ("rules", body.into())];
    hearth_ui::arrange::parts(&a.describe, &p, "device", items, |ps| column![ps.take("head"), scrollable(ps.take("rules")).height(Length::Fill)].spacing(16.0).into(), Msg::Describe)
}

fn log_page(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let mut c = Column::new();
    for q in a.d.log.iter().take(150) {
        let who = a.d.devices.iter().find(|d| d.id == q.who).map(|d| d.name.clone()).unwrap_or_else(|| q.who.clone());
        let act: E<'_, Msg> = if q.blocked { kit::chip_acc(&p, "Allow", Some(Msg::Allow(q.domain.clone()))) } else { chip(&p, "Block", Some(Msg::Block(q.domain.clone()))) };
        let r = row![
            container(t(q.at.clone(), 12.0, p.muted)).width(Length::Fixed(80.0)),
            container(t(q.domain.clone(), 13.0, p.text)).width(Length::Fill),
            container(t(who, 12.0, p.muted)).width(Length::Fixed(160.0)),
            container(if q.blocked { tag(&p, "Blocked", Tag::Acc) } else { tag(&p, "Allowed", Tag::Dark) }).width(Length::Fixed(90.0)),
            act
        ]
        .spacing(10.0)
        .align_y(iced::Center);
        c = c.push(container(r).padding(pad(7.0, 0.0, 7.0, 0.0)));
        c = c.push(ui::hline(Color::from_rgb8(0x2c, 0x26, 0x22)));
    }
    if a.d.log.is_empty() {
        c = c.push(t("Nothing asked yet.", 13.0, p.muted));
    }
    let items = vec![("head", head_of(a, "What was asked".into(), "Live: the newest first. Allow a site that broke; block one you don't want.".into())), ("list", c.into())];
    hearth_ui::arrange::parts(&a.describe, &p, "log", items, |ps| column![ps.take("head"), scrollable(ps.take("list")).height(Length::Fill)].spacing(16.0).into(), Msg::Describe)
}

fn lists_page(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let mut c = Column::new().spacing(12.0);
    for l in &a.d.lists {
        c = c.push(kit::line(&p, format!("{}{}", l.label, if l.count > 0 { format!(" · {} addresses", commas(l.count)) } else { String::new() }), switch(&p, l.on, Some(Msg::List(l.id.clone(), !l.on)))));
    }
    let items = vec![("head", head_of(a, "Block lists".into(), format!("{} lists{}", a.d.lists_n, if a.d.lists_at.is_empty() { String::new() } else { format!(" · updated {}", a.d.lists_at) }))), ("list", c.into())];
    hearth_ui::arrange::parts(&a.describe, &p, "lists", items, |ps| column![ps.take("head"), scrollable(ps.take("list")).height(Length::Fill)].spacing(16.0).into(), Msg::Describe)
}

fn domains_page<'a>(a: &'a App, allow: bool, title: &str, list: &'a [String], add: Msg, remove: fn(String) -> Msg) -> E<'a, Msg> {
    let p = a.p;
    let mut c = Column::new().spacing(8.0).push(row![text_input("a site, like example.com", &a.typed).on_input(Msg::Typed).on_submit(add.clone()).size(13.0).padding(pad(7.0, 10.0, 7.0, 10.0)).style(kit::field_style(&p)), kit::chip_acc(&p, "Add", Some(add))].spacing(8.0).align_y(iced::Center));
    for d in list {
        c = c.push(row![t(d.clone(), 13.0, p.text), Space::new().width(Length::Fill), chip(&p, "Remove", Some(remove(d.clone())))].align_y(iced::Center));
    }
    let items = vec![("head", head_of(a, title.into(), format!("{} sites", list.len()))), ("list", c.into())];
    if allow {
        return hearth_ui::arrange::parts(&a.describe, &p, "allow", items, |ps| column![ps.take("head"), scrollable(ps.take("list")).height(Length::Fill)].spacing(16.0).into(), Msg::Describe);
    }
    hearth_ui::arrange::parts(&a.describe, &p, "block", items, |ps| column![ps.take("head"), scrollable(ps.take("list")).height(Length::Fill)].spacing(16.0).into(), Msg::Describe)
}

fn family_page(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let adult = a.d.lists.iter().find(|l| l.id == "adult" || l.label.to_lowercase().contains("adult")).cloned();
    let mut c = Column::new().spacing(12.0);
    if let Some(l) = adult {
        c = c.push(kit::line(&p, "Adult sites", switch(&p, l.on, Some(Msg::List(l.id.clone(), !l.on)))));
    }
    if !a.pihole() {
        c = c.push(kit::line(&p, "Safe search (Google, Bing, DuckDuckGo, YouTube)", switch(&p, a.d.safe, Some(Msg::Safe(!a.d.safe)))));
    }
    c = c.push(t("For every device at home. A device that needs to be left out has its own rules on its page.", 12.0, p.muted));
    let items = vec![("head", head_of(a, "Family rules".into(), "What children shouldn't find by accident".into())), ("rules", c.into())];
    hearth_ui::arrange::parts(&a.describe, &p, "family", items, |ps| column![ps.take("head"), scrollable(ps.take("rules")).height(Length::Fill)].spacing(16.0).into(), Msg::Describe)
}

fn settings_page(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let pi = a.pihole();
    let mut c = Column::new().spacing(12.0).push(caps(&p, "Where Shield runs"));
    c = c.push(seg(&p, vec![(format!("This PC ({}, AdGuard Home)", a.server), !pi, Some(Msg::Source("here"))), ("A Pi-hole on your network".into(), pi, Some(Msg::Source("pihole")))]));
    if pi {
        c = c.push(row![text_input("http://pi.hole", &a.pi_url).on_input(Msg::PiUrl).size(13.0).padding(pad(7.0, 10.0, 7.0, 10.0)).style(kit::field_style(&p)), text_input("Its password", &a.pi_pass).on_input(Msg::PiPass).secure(true).on_submit(Msg::PiConnect).size(13.0).padding(pad(7.0, 10.0, 7.0, 10.0)).style(kit::field_style(&p)), kit::chip_acc(&p, "Connect", Some(Msg::PiConnect)), chip(&p, "Forget it", Some(Msg::PiForget))].spacing(8.0).align_y(iced::Center));
        c = c.push(t("Its password is kept only in this PC's keyring, never in a file.", 12.0, p.muted));
    } else {
        c = c.push(kit::line(&p, "Shield is on", switch(&p, a.d.on, Some(Msg::TurnOn(!a.d.on)))));
    }
    let fb = column![caps(&p, "Your router"), t(format!("Set your router's first DNS address to the {} and its second to the router itself (or your internet provider's). Devices then use Shield while it answers and the second one when it doesn't, so nothing ever stops working.", a.server), 12.0, p.muted).line_height(1.6)].spacing(10.0);
    let wh = column![head_of(a, "Settings".into(), "Where it runs, and how devices fall back".into()), c].spacing(16.0);
    hearth_ui::arrange::parts(&a.describe, &p, "settings", vec![("where", wh.into()), ("fallback", fb.into())], |ps| scrollable(column![ps.take("where"), ps.take("fallback")].spacing(16.0)).height(Length::Fill).into(), Msg::Describe)
}

fn off_card(a: &App) -> Option<E<'_, Msg>> {
    let p = a.p;
    if !a.loaded {
        return Some(kit::missing(&p, "Reading Shield's numbers…", "Asking the blocker for today.", None, true));
    }
    if !a.problem.is_empty() && !a.pihole() && !a.d.on {
        return Some(kit::missing(&p, "Shield is off", format!("Turn it on and the {} blocks ads and trackers for every device at home with AdGuard Home (GPL-3). It's never part of starting the PC, and devices fall back to normal look-ups whenever it doesn't answer.", a.server), Some(("Turn Shield on".into(), Msg::TurnOn(true))), a.busy));
    }
    if !a.problem.is_empty() {
        return Some(kit::missing(&p, "Shield isn't answering", a.problem.clone(), Some(("Settings".into(), Msg::Go(Page::Settings))), false));
    }
    None
}

// ------------------------------------------------------------------ its blocks
const BLOCKS: &[PageDef] = &[
    PageDef { id: "window", title: "Hearth Shield", blocks: &[bl("side", "The side", 0, 0, 2, 12), bl("page", "The page", 2, 0, 7, 12), bl("panel", "Blocked the most and the lists", 9, 0, 3, 12)] },
    PageDef { id: "today", title: "Today", blocks: &[bl("head", "Today and its range", 0, 0, 12, 1), bl("kpis", "Asked, blocked, lists, time saved", 0, 1, 12, 2), bl("chart", "Through the day", 0, 3, 12, 4), bl("devices", "Devices", 0, 7, 12, 5)] },
    PageDef { id: "panel", title: "The right side", blocks: &[bl("top", "Blocked the most", 0, 0, 12, 6), bl("lists", "Block lists", 0, 6, 12, 4), bl("tip", "Something broke on a site?", 0, 10, 12, 2)] },
    PageDef { id: "devices", title: "Devices", blocks: &[bl("head", "Its title", 0, 0, 12, 1), bl("list", "The devices", 0, 1, 12, 11)] },
    PageDef { id: "device", title: "A device", blocks: &[bl("head", "Its name", 0, 0, 12, 1), bl("rules", "Its own rules", 0, 1, 12, 11)] },
    PageDef { id: "log", title: "What was asked", blocks: &[bl("head", "Its title", 0, 0, 12, 1), bl("list", "What was asked", 0, 1, 12, 11)] },
    PageDef { id: "lists", title: "Block lists", blocks: &[bl("head", "Its title", 0, 0, 12, 1), bl("list", "The lists", 0, 1, 12, 11)] },
    PageDef { id: "allow", title: "Always allow", blocks: &[bl("head", "Its title", 0, 0, 12, 1), bl("list", "The sites", 0, 1, 12, 11)] },
    PageDef { id: "block", title: "Always block", blocks: &[bl("head", "Its title", 0, 0, 12, 1), bl("list", "The sites", 0, 1, 12, 11)] },
    PageDef { id: "family", title: "Family rules", blocks: &[bl("head", "Its title", 0, 0, 12, 1), bl("rules", "The rules", 0, 1, 12, 11)] },
    PageDef { id: "settings", title: "Settings", blocks: &[bl("where", "Where it runs", 0, 0, 12, 6), bl("fallback", "Your router", 0, 6, 12, 6)] },
];

const KIT: Kit = Kit {
    name: "Shield",
    buttons: &[
        tile("pause", "Pause 5 min", G::Clock),
        tile("today", "Today", G::Pulse),
        tile("devices", "Devices", G::Users),
        tile("log", "What was asked", G::List),
        tile("lists", "Block lists", G::Shield),
        tile("allow", "Always allow", G::Check),
        tile("block", "Always block", G::Close),
        tile("settings", "Settings", G::Gear),
    ],
    panes: &[tile("top", "Blocked the most", G::Shield), tile("live", "What was asked, live", G::List)],
    info: &[tile("asked", "Asked today", G::Info), tile("blocked", "Blocked today", G::Shield), tile("share", "Share blocked", G::Pulse), tile("devices", "Devices", G::Users)],
    pane_page: "today",
    bars: &[],
    menus: &[],
};

fn block_action(id: &str) -> Option<Msg> {
    Some(match id {
        "pause" => Msg::Pause,
        "today" => Msg::Go(Page::Today),
        "devices" => Msg::Go(Page::Devices),
        "log" => Msg::Go(Page::Log),
        "lists" => Msg::Go(Page::Lists),
        "allow" => Msg::Go(Page::Allow),
        "block" => Msg::Go(Page::Block),
        "settings" => Msg::Go(Page::Settings),
        _ => return None,
    })
}

fn pane(a: &App, which: &str) -> E<'static, Msg> {
    let p = a.p;
    let mut c = Column::new().spacing(6.0);
    if which == "top" {
        for (w, _, n) in a.d.top.iter().take(7) {
            c = c.push(row![t(w.clone(), 12.0, p.text), Space::new().width(Length::Fill), t(commas(*n), 12.0, p.accent)].spacing(8.0));
        }
    } else {
        for q in a.d.log.iter().take(12) {
            c = c.push(row![t(q.domain.clone(), 12.0, p.text), Space::new().width(Length::Fill), t(if q.blocked { "blocked" } else { "" }, 11.0, p.accent)].spacing(8.0));
        }
    }
    scrollable(c).height(Length::Fill).into()
}

fn view(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let share = if a.d.asked > 0 { format!("{:.1}%", a.d.blocked as f64 * 100.0 / a.d.asked as f64) } else { String::new() };
    a.describe.values(vec![("asked", commas(a.d.asked)), ("blocked", commas(a.d.blocked)), ("share", share), ("devices", a.d.devices_seen.to_string())]);
    let page: E<'_, Msg> = match (off_card(a), a.page) {
        (Some(card), Page::Today) => container(card).padding(pad(22.0, 26.0, 22.0, 26.0)).into(),
        (_, Page::Today) => container(today_page(a)).padding(pad(22.0, 26.0, 22.0, 26.0)).into(),
        (_, pg) => container(match pg {
            Page::Devices => devices_page(a),
            Page::Device => device_page(a),
            Page::Log => log_page(a),
            Page::Lists => lists_page(a),
            Page::Allow => domains_page(a, true, "Always allow", &a.d.allow, Msg::AddAllow, Msg::Unallow),
            Page::Block => domains_page(a, false, "Always block", &a.d.block, Msg::AddBlock, Msg::Unblock),
            Page::Family => family_page(a),
            _ => settings_page(a),
        })
        .padding(pad(22.0, 26.0, 22.0, 26.0))
        .into(),
    };
    let mut items: Vec<(&'static str, E<'_, Msg>)> = vec![
        ("side", hearth_ui::studio::made(&a.describe, "window", "side", || container(side(a)).width(Length::Fixed(240.0)).height(Length::Fill).padding(pad(14.0, 12.0, 14.0, 12.0)).style(ui::boxed(Color::from_rgba8(20, 17, 15, 0.5), 0.0, None)).into())),
        ("page", page),
        ("panel", container(panel(a)).width(Length::Fixed(420.0)).height(Length::Fill).padding(pad(18.0, 20.0, 18.0, 20.0)).style(ui::boxed(Color::from_rgba8(20, 17, 15, 0.5), 0.0, None)).into()),
    ];
    if a.describe.wants_pane("live") {
        items.push(("pane:live", pane(a, "live")));
    }
    let body = hearth_ui::arrange::parts(&a.describe, &p, "window", items, move |ps| row![ps.take("side"), ui::vline(p.line), container(ps.take("page")).width(Length::Fill).height(Length::Fill), ui::vline(p.line), ps.take("panel")].height(Length::Fill).into(), Msg::Describe);
    let left = a.paused_left();
    let status = if !a.loaded {
        tag(&p, "Looking…", Tag::Dark)
    } else if !a.d.on {
        tag(&p, "Off", Tag::Dark)
    } else if left > 0 {
        tag(&p, format!("Paused · {} min left", left.div_ceil(60)), Tag::Warn)
    } else if a.pihole() {
        tag(&p, "On · runs on your Pi-hole", Tag::Ok)
    } else {
        tag(&p, format!("On · runs on the {}", a.server), Tag::Ok)
    };
    let pause = if left > 0 { chip(&p, "Block again", Some(Msg::Pause)) } else { chip(&p, "Pause 5 min", if a.d.on { Some(Msg::Pause) } else { None }) };
    let bar = kit::title_bar(
        &p,
        "SH",
        Color::from_rgb8(0xb8, 0xf0, 0x7f),
        Color::from_rgb8(0x3b, 0xa5, 0x5c),
        "Hearth Shield",
        "ads and trackers blocked for every device at home",
        vec![status, pause],
        TitleMsgs { drag: Msg::Drag, maximize: Msg::Max, minimize: Some(Msg::Min), close: Msg::Close },
        Msg::Describe(hearth_ui::describe::DMsg::Open),
    );
    let all = container(column![bar, container(body).height(Length::Fill), kit::credit(&p, "Hearth Shield", "Pi-hole (EUPL-1.2) and AdGuard Home (GPL-3)")]).width(Length::Fill).height(Length::Fill).style(ui::boxed(p.bg, p.window_corner, Some((p.line, 1.0))));
    ui::edges(all.into(), Msg::Resize)
}

fn subscription(a: &App) -> Subscription<Msg> {
    let mut s = vec![Subscription::run(hearth_ui::theme::watch).map(|_| Msg::Theme)];
    // live while it's looked at: Today and What was asked, every 5 seconds (one small read)
    if matches!(a.page, Page::Today | Page::Log) && !data::is_fixture() {
        s.push(iced::time::every(std::time::Duration::from_secs(5)).map(|_| Msg::Tick));
    }
    Subscription::batch(s)
}

fn shown(a: &App) -> Element<'_, Msg> {
    hearth_ui::describe::with_bar(view(a), &a.p, &a.describe, Msg::Describe)
}

fn main() -> iced::Result {
    if app::flag("--count") {
        // one minute's count (hearth-shield-count.service; count.rs): nothing at all when AdGuard isn't answering
        std::process::exit(match count::tick() { Ok(()) => 0, Err(e) => { eprintln!("hearth-shield-app --count: {e}"); 0 } });
    }
    if app::handle_cli("hearth-shield-app", "Hearth Shield: ads and trackers blocked for every device at home (built on AdGuard Home and Pi-hole).\n  --page today|devices|log|lists|allow|block|family|settings|device\n  --device ADDRESS   with --page device") {
        return Ok(());
    }
    let w = app::window("hearth-shield", (1840.0, 970.0), (1100.0, 700.0));
    iced::application(
        || {
            let (mut a, t) = App::new();
            let mut t = t;
            if a.page == Page::Device && !a.dev.is_empty() {
                let id = a.dev.clone();
                a.page = Page::Today;
                t = t.chain(Task::done(Msg::OpenDev(id)));
            }
            (a, t)
        },
        update,
        shown,
    )
    .title(|_: &App| "Hearth Shield".to_string())
    .subscription(subscription)
    .theme(|a: &App| app::iced_theme(&a.p))
    .window(w)
    .default_font(app::INTER)
    .run()
}
