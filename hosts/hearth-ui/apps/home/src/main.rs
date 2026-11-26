//! hearth-home-app: Hearth Home (docs/mockups/checklists/open-source-14-home.md; the approved board
//! docs/mockups/open-source-apps/14-home.png). A big, TV-friendly smart-home dashboard on Home Assistant (Apache-2.0),
//! which runs on the Nvidia PC (hearth-oss-server) and is reached here, on the Ally and on the phone: lights by room,
//! the thermostat, cameras, plugs and today's energy, scenes and the routines coming up. Everything is Home
//! Assistant's own (its REST API, ha.rs): what's on the screen is what your home says, and every press is a service call.
//!
//! The controller (hearth-pad's keys): A turns the picked one on or off, ◀ ▶ brightness, Y scenes, B back; the arrows
//! move. The token you make in Home Assistant is kept in your keyring, never in a file.
//! Your choices: ~/.config/nimbus/home.json. $HEARTH_HOME_FIXTURE: made-up answers (CI's pictures; nothing is sent).

mod ha;
mod mjpeg;

use ha::Ha;
use hearth_oss::prefs::Prefs;
use hearth_oss::w::{self, Sz, Tg};
use hearth_ui::app::{self, win};
use hearth_ui::run;
use hearth_ui::ui::{self, t, tb};
use hearth_ui::Pal;
use iced::keyboard::{key::Named, Event as KEvent, Key};
use iced::widget::{canvas, column, container, image, mouse_area, row, slider, Column, Row, Space};
use iced::{mouse, Color, Element, Length, Padding, Subscription, Task};
use serde_json::{json, Value};
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
struct World {
    states: Vec<Value>,
    areas: Vec<(String, String, Vec<String>)>,
    energy: Vec<f64>,
    next: Vec<(String, i64)>,
}

#[derive(Debug, Clone)]
enum Msg {
    Describe(hearth_ui::describe::DMsg),
    Theme,
    Tick,
    Secret(String),
    Got(Result<World, String>),
    Cam(String, Option<Vec<u8>>),
    /// a picture of a camera's moving stream (mjpeg.rs)
    Moving(String, Vec<u8>),
    Tab(&'static str),
    Toggle(String),
    Room(String),
    Bright(f32),
    BrightDone,
    Temp(f64),
    Mode(&'static str),
    Scene(String),
    AllOff,
    Done(Result<(), String>),
    Key(KEvent),
    Server(String),
    Token(String),
    SignIn,
    Status(Value),
    Install,
    TurnOn,
    Ran(run::Output),
    Close,
    Drag,
}

struct App {
    describe: hearth_ui::describe::Describe,
    p: Pal,
    prefs: Prefs,
    ha: Option<Ha>,
    world: World,
    cams: HashMap<String, iced::widget::image::Handle>,
    /// when each camera's moving picture last came in
    moving: HashMap<String, std::time::Instant>,
    tab: &'static str,
    room: String,
    bright: f32,
    focus: usize,
    row: usize,
    token_typed: String,
    status: Value,
    said: String,
    loading: bool,
    fixture: bool,
}

fn bg<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> impl std::future::Future<Output = Option<T>> + Send + 'static {
    let (tx, rx) = futures::channel::oneshot::channel();
    let _ = std::thread::Builder::new().name("hearth-home".into()).spawn(move || {
        let _ = tx.send(f());
    });
    async move { rx.await.ok() }
}

fn day_start() -> i64 {
    let now = hearth_oss::now();
    let t = hearth_ui::time::local(now);
    now - (t.hour * 3600 + t.min * 60 + t.sec) as i64
}
fn iso(t: i64) -> String {
    let (d, s) = (t.div_euclid(86_400), t.rem_euclid(86_400));
    let z = d + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let dd = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{y:04}-{m:02}-{dd:02}T{:02}:{:02}:{:02}+00:00", s / 3600, s / 60 % 60, s % 60)
}

/// Your home, read from Home Assistant: every state, the rooms, today's energy, when routines run next.
fn read(h: Ha, energy_sensor: String) -> Result<World, String> {
    let states = h.get("/api/states")?.as_array().cloned().unwrap_or_default();
    let areas: Vec<(String, String, Vec<String>)> = h
        .template(ha::AREAS)
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default()
        .iter()
        .map(|a| (a["id"].as_str().unwrap_or("").to_string(), a["name"].as_str().unwrap_or("").to_string(), a["entities"].as_array().map(|e| e.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default()))
        .collect();
    let es = if energy_sensor.is_empty() {
        states.iter().find(|s| ha::attr(s, "device_class").and_then(Value::as_str) == Some("energy") && ha::attr(s, "state_class").and_then(Value::as_str) == Some("total_increasing")).map(|s| ha::id(s).to_string()).unwrap_or_default()
    } else {
        energy_sensor
    };
    let energy = if es.is_empty() { Vec::new() } else { h.get(&format!("/api/history/period/{}?filter_entity_id={es}&minimal_response&no_attributes", hearth_oss::http::enc(&iso(day_start())))).map(|v| ha::hourly(&v, day_start())).unwrap_or_default() };
    let mut next = Vec::new();
    for s in states.iter().filter(|s| ha::domain(s) == "automation" && ha::state(s) == "on") {
        if let Some(cid) = ha::attr(s, "id").and_then(Value::as_str) {
            if let Ok(cfg) = h.get(&format!("/api/config/automation/config/{cid}")) {
                if let Some(at) = ha::next_time(&cfg) {
                    next.push((ha::id(s).to_string(), at));
                }
            }
        }
    }
    Ok(World { states, areas, energy, next })
}

fn fixture() -> Option<World> {
    let f = std::env::var("HEARTH_HOME_FIXTURE").ok()?;
    let j: Value = serde_json::from_str(&std::fs::read_to_string(f).ok()?).ok()?;
    let mut states = j["states"].as_array().cloned().unwrap_or_default();
    // the fixture's "on for" times, from now
    let now = hearth_oss::now();
    for s in states.iter_mut() {
        if let Some(ago) = s.get("agoSecs").and_then(Value::as_i64) {
            s["last_changed"] = json!(iso(now - ago));
        }
    }
    let areas = j["areas"].as_array().cloned().unwrap_or_default().iter().map(|a| (a["id"].as_str().unwrap_or("").to_string(), a["name"].as_str().unwrap_or("").to_string(), a["entities"].as_array().map(|e| e.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default())).collect();
    let energy = j["energy"].as_array().map(|a| a.iter().filter_map(Value::as_f64).collect()).unwrap_or_default();
    let next = j["next"].as_array().cloned().unwrap_or_default().iter().map(|n| (n[0].as_str().unwrap_or("").to_string(), n[1].as_i64().unwrap_or(0))).collect();
    Some(World { states, areas, energy, next })
}

impl App {
    fn new() -> (App, Task<Msg>) {
        let prefs = Prefs::load("home");
        let mut a = App {
            describe: hearth_ui::describe::Describe::new("home", "Hearth Home").with_pages(BLOCKS).with_kit(&KIT),
            p: hearth_ui::theme::load(),
            room: prefs.s("room", ""),
            prefs,
            ha: None,
            world: World::default(),
            cams: HashMap::new(),
            moving: HashMap::new(),
            tab: "home",
            bright: 60.0,
            focus: 0,
            row: 0,
            token_typed: String::new(),
            status: Value::Null,
            said: String::new(),
            loading: true,
            fixture: false,
        };
        if let Some(wd) = fixture() {
            a.fixture = true;
            a.world = wd;
            a.loading = false;
            a.ha = Some(Ha { base: a.base(), token: String::new() });
            a.fix_room();
            let dir = std::env::var("HEARTH_HOME_FIXTURE").ok().and_then(|f| std::path::Path::new(&f).parent().map(|p| p.to_path_buf()));
            if let Some(d) = dir {
                for c in a.cameras() {
                    let id = ha::id(&c).to_string();
                    if let Ok(b) = std::fs::read(d.join(format!("{id}.png"))) {
                        a.cams.insert(id, iced::widget::image::Handle::from_bytes(b));
                    }
                }
            }
            return (a, Task::none());
        }
        let t = vec![
            Task::perform(hearth_oss::secret::lookup("hearth-home", "token"), Msg::Secret),
            Task::perform(run::spawn(run::args(&["python3", &run::shell_helper("hearth-oss-server"), "status"])), |o| Msg::Status(o.json().unwrap_or(Value::Null))),
        ];
        (a, Task::batch(t))
    }
    fn base(&self) -> String {
        self.prefs.s("server", "http://127.0.0.1:8123").trim_end_matches('/').to_string()
    }
    fn get(&self, id: &str) -> Option<&Value> {
        self.world.states.iter().find(|s| ha::id(s) == id)
    }
    fn of(&self, d: &str) -> Vec<Value> {
        self.world.states.iter().filter(|s| ha::domain(s) == d).cloned().collect()
    }
    fn lights_in(&self, room: &str) -> Vec<Value> {
        match self.world.areas.iter().find(|a| a.0 == room) {
            Some((_, _, ents)) => ents.iter().filter_map(|e| self.get(e)).filter(|s| ha::domain(s) == "light").cloned().collect(),
            None => self.of("light"),
        }
    }
    fn room_name(&self, room: &str) -> String {
        self.world.areas.iter().find(|a| a.0 == room).map(|a| a.1.clone()).unwrap_or_else(|| "Every light".into())
    }
    /// The room it opens on: the one you picked, else the one with the most lights on.
    fn fix_room(&mut self) {
        if self.room.is_empty() || !self.world.areas.iter().any(|a| a.0 == self.room) {
            let best = self.world.areas.iter().max_by_key(|a| a.2.iter().filter(|e| self.get(e).map(|s| ha::domain(s) == "light" && ha::state(s) == "on").unwrap_or(false)).count()).map(|a| a.0.clone());
            self.room = best.unwrap_or_default();
        }
        let on: Vec<f64> = self.lights_in(&self.room).iter().filter(|s| ha::state(s) == "on").filter_map(|s| ha::attr(s, "brightness").and_then(Value::as_f64)).collect();
        if !on.is_empty() {
            self.bright = (on.iter().sum::<f64>() / on.len() as f64 / 255.0 * 100.0).round() as f32;
        }
    }
    fn cameras(&self) -> Vec<Value> {
        self.of("camera").into_iter().take(2).collect()
    }
    /// Where the cameras' moving pictures come from: Home Assistant once signed in (CI's pictures: a stand-in
    /// stream, $HEARTH_HOME_STREAM).
    fn stream_from(&self) -> Option<(String, String)> {
        if self.fixture {
            return std::env::var("HEARTH_HOME_STREAM").ok().map(|b| (b, String::new()));
        }
        self.ha.as_ref().filter(|h| !h.token.is_empty()).map(|h| (h.base.clone(), h.token.clone()))
    }
    fn plugs(&self) -> Vec<Value> {
        self.of("switch").into_iter().filter(|s| !ha::id(s).contains("automation")).take(4).collect()
    }
    /// A plug's power sensor (sensor.<its name>_power).
    fn watts(&self, plug: &Value) -> Option<f64> {
        let base = ha::id(plug).split('.').nth(1).unwrap_or("");
        self.get(&format!("sensor.{base}_power")).and_then(ha::num)
    }
    fn reload(&mut self) -> Task<Msg> {
        let Some(h) = self.ha.clone() else { return Task::none() };
        if self.fixture {
            return Task::none();
        }
        let es = self.prefs.s("energySensor", "");
        Task::perform(bg(move || read(h, es)), |r| Msg::Got(r.unwrap_or_else(|| Err("it stopped".into()))))
    }
    fn call(&mut self, domain: &str, service: &str, data: Value) -> Task<Msg> {
        let Some(h) = self.ha.clone() else { return Task::none() };
        if self.fixture {
            return Task::none();
        }
        let (d, s) = (domain.to_string(), service.to_string());
        Task::perform(bg(move || h.call(&d, &s, &data)), |r| Msg::Done(r.unwrap_or_else(|| Err("it stopped".into()))))
    }
    fn set(&mut self, k: &str, v: Value) {
        if let Err(e) = self.prefs.set(k, v) {
            self.said = e;
        }
    }
}

fn update(a: &mut App, m: Msg) -> Task<Msg> {
    match m {
        Msg::Describe(dm) => {
            if let Some(m) = hearth_ui::studio::act(&dm).and_then(|id| block_action(a, id)) {
                return update(a, m);
            }
            return a.describe.update(dm).map(Msg::Describe);
        }
        Msg::Theme => a.p = hearth_ui::theme::load(),
        Msg::Tick => {
            // the cameras' pictures and the states, again (only while it's open)
            let mut t = vec![a.reload()];
            if let Some(h) = a.ha.clone().filter(|_| !a.fixture) {
                for c in a.cameras() {
                    let id = ha::id(&c).to_string();
                    // a camera whose moving picture is coming in doesn't need its still
                    if a.moving.get(&id).map(|t| t.elapsed().as_secs() < 5).unwrap_or(false) {
                        continue;
                    }
                    let h2 = h.clone();
                    t.push(Task::perform(bg(move || { let b = h2.bytes(&format!("/api/camera_proxy/{id}")).ok(); (id, b) }), |r| { let (id, b) = r.unwrap_or_default(); Msg::Cam(id, b) }));
                }
            }
            return Task::batch(t);
        }
        Msg::Secret(s) => {
            if s.is_empty() {
                a.loading = false;
                return Task::none();
            }
            a.ha = Some(Ha { base: a.base(), token: s });
            return update(a, Msg::Tick);
        }
        Msg::Got(r) => {
            a.loading = false;
            match r {
                Ok(wd) => {
                    a.world = wd;
                    a.fix_room();
                }
                Err(e) => a.said = e,
            }
        }
        Msg::Cam(id, b) => {
            if let Some(b) = b {
                a.cams.insert(id, iced::widget::image::Handle::from_bytes(b));
            }
        }
        Msg::Moving(id, b) => {
            a.moving.insert(id.clone(), std::time::Instant::now());
            a.cams.insert(id, iced::widget::image::Handle::from_bytes(b));
        }
        Msg::Tab(t) => a.tab = t,
        Msg::Toggle(id) => {
            let dom = id.split('.').next().unwrap_or("homeassistant").to_string();
            if let Some(s) = a.world.states.iter_mut().find(|s| ha::id(s) == id) {
                let on = ha::state(s) == "on";
                s["state"] = json!(if on { "off" } else { "on" });
            }
            return a.call(&dom, "toggle", json!({"entity_id": id}));
        }
        Msg::Room(r) => {
            a.room = r.clone();
            a.set("room", json!(r));
            a.fix_room();
        }
        Msg::Bright(v) => a.bright = v.round(),
        Msg::BrightDone => {
            let room = a.room.clone();
            let v = a.bright;
            let target = if room.is_empty() { json!({"entity_id": a.of("light").iter().map(|s| ha::id(s).to_string()).collect::<Vec<_>>(), "brightness_pct": v}) } else { json!({"area_id": room, "brightness_pct": v}) };
            return a.call("light", "turn_on", target);
        }
        Msg::Temp(d) => {
            if let Some(c) = a.of("climate").first() {
                let id = ha::id(c).to_string();
                let cur = ha::attr(c, "temperature").and_then(Value::as_f64).unwrap_or(70.0);
                if let Some(s) = a.world.states.iter_mut().find(|s| ha::id(s) == id) {
                    s["attributes"]["temperature"] = json!(cur + d);
                }
                return a.call("climate", "set_temperature", json!({"entity_id": id, "temperature": cur + d}));
            }
        }
        Msg::Mode(md) => {
            if let Some(c) = a.of("climate").first() {
                let id = ha::id(c).to_string();
                if let Some(s) = a.world.states.iter_mut().find(|s| ha::id(s) == id) {
                    s["state"] = json!(md);
                }
                return a.call("climate", "set_hvac_mode", json!({"entity_id": id, "hvac_mode": md}));
            }
        }
        Msg::Scene(id) => {
            a.said = format!("{} …", a.get(&id).map(ha::name).unwrap_or_default());
            return a.call("scene", "turn_on", json!({"entity_id": id}));
        }
        Msg::AllOff => {
            let ids: Vec<String> = a.of("light").iter().map(|s| ha::id(s).to_string()).collect();
            for s in a.world.states.iter_mut().filter(|s| ha::domain(s) == "light") {
                s["state"] = json!("off");
            }
            return a.call("light", "turn_off", json!({"entity_id": ids}));
        }
        Msg::Done(r) => {
            if let Err(e) = r {
                a.said = e;
            }
            return a.reload();
        }
        Msg::Key(ev) => return key(a, ev),
        Msg::Server(s) => {
            a.prefs.m.insert("server".into(), json!(s.trim()));
        }
        Msg::Token(s) => a.token_typed = s,
        Msg::SignIn => {
            let tok = std::mem::take(&mut a.token_typed);
            if tok.trim().is_empty() {
                return Task::none();
            }
            let b = a.base();
            if Prefs::load("home").s("server", "") != b {
                a.set("server", json!(b));
            }
            a.ha = Some(Ha { base: b, token: tok.trim().to_string() });
            return Task::batch([Task::perform(hearth_oss::secret::store("hearth-home", "token", "Hearth Home (Home Assistant)", tok.trim().to_string()), |_| Msg::Done(Ok(()))), update(a, Msg::Tick)]);
        }
        Msg::Status(v) => a.status = v,
        Msg::Install => return Task::perform(run::spawn(run::args(&["python3", &run::shell_helper("hearth-oss-server"), "install", "homeassistant"])), Msg::Ran),
        Msg::TurnOn => return Task::perform(run::spawn(run::args(&["python3", &run::shell_helper("hearth-oss-server"), "on", "homeassistant"])), Msg::Ran),
        Msg::Ran(o) => {
            a.said = if o.ok { "Done. Set your home up on Home Assistant's own page, make a long-lived token there, and paste it here.".into() } else { format!("That didn't work: {}", o.why()) };
            return Task::perform(run::spawn(run::args(&["python3", &run::shell_helper("hearth-oss-server"), "status"])), |o| Msg::Status(o.json().unwrap_or(Value::Null)));
        }
        Msg::Drag => return win::drag(),
        Msg::Close => return win::close(),
    }
    Task::none()
}

/// The controller through hearth-pad's keys (and the keyboard): A / Enter on or off, ◀ ▶ brightness, Y scenes,
/// B / Esc back, the arrows move between the tiles and the lights.
fn key(a: &mut App, ev: KEvent) -> Task<Msg> {
    let KEvent::KeyPressed { key, modifiers, .. } = ev else { return Task::none() };
    if modifiers.control() && matches!(key.as_ref(), Key::Character("q")) {
        return win::close();
    }
    // Edit layout takes the controller while the dashboard is being edited
    let pad_ev = match key.as_ref() {
        Key::Named(Named::ArrowUp) => "up",
        Key::Named(Named::ArrowDown) => "down",
        Key::Named(Named::ArrowLeft) => "left",
        Key::Named(Named::ArrowRight) => "right",
        Key::Named(Named::Escape) => "b",
        _ => "",
    };
    if !pad_ev.is_empty() {
        if let Some(t) = hearth_ui::arrange::pad(&mut a.describe, pad_ev) {
            return t.map(Msg::Describe);
        }
    }
    let lights = a.lights_in(&a.room.clone());
    match key.as_ref() {
        Key::Named(Named::ArrowLeft) | Key::Named(Named::ArrowRight) if a.focus == 0 => {
            let d = if matches!(key.as_ref(), Key::Named(Named::ArrowLeft)) { -10.0 } else { 10.0 };
            a.bright = (a.bright + d).clamp(1.0, 100.0);
            return update(a, Msg::BrightDone);
        }
        Key::Named(Named::ArrowLeft) => a.focus = a.focus.saturating_sub(1),
        Key::Named(Named::ArrowRight) | Key::Named(Named::Tab) => a.focus = (a.focus + 1) % 5,
        Key::Named(Named::ArrowDown) if a.focus == 0 => a.row = (a.row + 1).min(lights.len().saturating_sub(1)),
        Key::Named(Named::ArrowUp) if a.focus == 0 => a.row = a.row.saturating_sub(1),
        Key::Named(Named::Enter) | Key::Named(Named::Space) => {
            if a.focus == 0 {
                if let Some(l) = lights.get(a.row) {
                    return update(a, Msg::Toggle(ha::id(l).to_string()));
                }
            } else if a.focus == 3 {
                if let Some(pl) = a.plugs().get(a.row) {
                    return update(a, Msg::Toggle(ha::id(pl).to_string()));
                }
            }
        }
        Key::Named(Named::Escape) => {
            a.tab = "home";
            a.focus = 0;
        }
        Key::Character(c) if c == "y" => a.focus = 4,
        _ => {}
    }
    Task::none()
}

// ---------------------------------------------------------------- the dashboard

fn tile<'a>(p: &Pal, inner: Element<'a, Msg>, sel: bool) -> Element<'a, Msg> {
    let c = container(inner).padding(22.0).width(Length::Fill).height(Length::Fill).style(ui::boxed(p.panel, 20.0, Some((p.line, 1.0))));
    if sel {
        container(container(c).padding(4.0)).padding(0.0).style(ui::boxed(Color::TRANSPARENT, 26.0, Some((p.accent, 4.0)))).into()
    } else {
        container(c).padding(4.0).into()
    }
}
fn k<'a>(p: &Pal, s: impl Into<String>) -> Element<'a, Msg> {
    tb(s.into().to_uppercase(), 13.0, p.muted).into()
}

fn lights_tile(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    let all = a.of("light");
    let on = all.iter().filter(|s| ha::state(s) == "on").count();
    let mut c = column![k(&p, format!("Lights · {on} of {} on", all.len())), tb(a.room_name(&a.room), 30.0, p.text)].spacing(6.0);
    let lights = a.lights_in(&a.room);
    let mut list = Column::new();
    let now = hearth_oss::now();
    for (i, l) in lights.iter().enumerate().take(6) {
        let lit = ha::state(l) == "on";
        let sw = if lit { w::swatch_g(if ha::attr(l, "hearth_gradient").is_some() { "pk" } else { "or" }, 40.0, 40.0, 12.0) } else { w::swatch(Pal::mix(p.panel, p.line, 0.5), 40.0, 40.0, 12.0) };
        let focus = a.focus == 0 && a.row == i;
        let r = row![sw, column![t(ha::name(l), 18.0, p.text), t(ha::light_words(l, now), 13.0, p.muted)].spacing(2.0).width(Length::Fill), w::sw_k(&p, lit, Some(Msg::Toggle(ha::id(l).to_string())), 1.4)].spacing(14.0).align_y(iced::Center);
        list = list.push(container(r).padding(Padding { top: 12.0, right: 6.0, bottom: 12.0, left: 6.0 }).style(ui::boxed(if focus { Pal::mix(p.panel, p.accent, 0.1) } else { Color::TRANSPARENT }, 10.0, None)));
        if i + 1 < lights.len().min(6) {
            list = list.push(ui::hline(Pal::mix(p.line, p.bg, 0.4)));
        }
    }
    c = c.push(list);
    c = c.push(container(k(&p, format!("Brightness · {}", a.room_name(&a.room).to_lowercase()))).padding(Padding { top: 18.0, right: 0.0, bottom: 4.0, left: 0.0 }));
    c = c.push(row![slider(1.0..=100.0, a.bright, Msg::Bright).step(1.0f32).on_release(Msg::BrightDone).style(w::slider_style(&p, 1.5)).width(Length::Fill), t(format!("{}%", a.bright as i32), 18.0, p.text)].spacing(16.0).align_y(iced::Center));
    c = c.push(container(k(&p, "Other rooms")).padding(Padding { top: 22.0, right: 0.0, bottom: 4.0, left: 0.0 }));
    let mut grid = Column::new().spacing(12.0);
    let others: Vec<&(String, String, Vec<String>)> = a.world.areas.iter().filter(|r| r.0 != a.room).take(4).collect();
    for pair in others.chunks(2) {
        let mut rw = Row::new().spacing(12.0);
        for r in pair {
            let ls: Vec<&Value> = r.2.iter().filter_map(|e| a.get(e)).filter(|s| ha::domain(s) == "light").collect();
            let on: Vec<&&Value> = ls.iter().filter(|s| ha::state(s) == "on").collect();
            let mut sub = match on.len() {
                0 => "all off".to_string(),
                1 => format!("1 light on{}", ha::attr(on[0], "brightness").and_then(Value::as_f64).map(|b| format!(" · {}%", (b / 2.55).round())).unwrap_or_default()),
                n => format!("{n} on"),
            };
            if let Some(note) = r.2.iter().filter_map(|e| a.get(e)).find_map(|s| ha::attr(s, "hearth_room_note").and_then(Value::as_str)) {
                sub = format!("{sub} · {note}");
            }
            let cell = container(column![tb(r.1.clone(), 19.0, p.text), t(sub, 13.0, p.muted)].spacing(6.0)).padding(Padding { top: 0.0, right: 20.0, bottom: 0.0, left: 20.0 }).center_y(Length::Fixed(84.0)).width(Length::FillPortion(1)).style(ui::boxed(p.panel2, 16.0, Some((p.line, 1.0))));
            rw = rw.push(mouse_area(cell).on_press(Msg::Room(r.0.clone())).interaction(mouse::Interaction::Pointer));
        }
        grid = grid.push(rw);
    }
    c = c.push(grid);
    tile(&p, c.into(), a.focus == 0)
}

/// The thermostat's dial (the board's arc and knob).
struct Dial {
    frac: f32,
    color: Color,
    track: Color,
}
impl canvas::Program<Msg> for Dial {
    type State = ();
    fn draw(&self, _s: &(), r: &iced::Renderer, _t: &iced::Theme, b: iced::Rectangle, _c: mouse::Cursor) -> Vec<canvas::Geometry> {
        use iced::widget::canvas::{path::Arc, Frame, Path, Stroke};
        let mut f = Frame::new(r, b.size());
        let c = iced::Point::new(b.width / 2.0, b.height * 0.58);
        let rad = b.width.min(b.height * 1.3) / 2.0 - 16.0;
        let (a0, span) = (std::f32::consts::PI * 0.75, std::f32::consts::PI * 1.5);
        let arc = |to: f32| Path::new(|p| p.arc(Arc { center: c, radius: rad, start_angle: iced::Radians(a0), end_angle: iced::Radians(a0 + span * to) }));
        f.stroke(&arc(1.0), Stroke::default().with_color(self.track).with_width(18.0).with_line_cap(canvas::LineCap::Round));
        f.stroke(&arc(self.frac.clamp(0.01, 1.0)), Stroke::default().with_color(self.color).with_width(18.0).with_line_cap(canvas::LineCap::Round));
        let ang = a0 + span * self.frac.clamp(0.0, 1.0);
        f.fill(&Path::circle(iced::Point::new(c.x + rad * ang.cos(), c.y + rad * ang.sin()), 14.0), Color::WHITE);
        vec![f.into_geometry()]
    }
}

fn thermostat_tile(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    let Some(c) = a.of("climate").into_iter().next() else {
        return tile(&p, column![k(&p, "Thermostat"), t("No thermostat in Home Assistant yet.", 14.0, p.muted)].spacing(10.0).into(), a.focus == 1);
    };
    let set = ha::attr(&c, "temperature").and_then(Value::as_f64).unwrap_or(70.0);
    let now_t = ha::attr(&c, "current_temperature").and_then(Value::as_f64).unwrap_or(set);
    let (lo, hi) = (ha::attr(&c, "min_temp").and_then(Value::as_f64).unwrap_or(45.0), ha::attr(&c, "max_temp").and_then(Value::as_f64).unwrap_or(95.0));
    let mode = ha::state(&c).to_string();
    let action = ha::attr(&c, "hvac_action").and_then(Value::as_str).unwrap_or(&mode).to_string();
    let unit = ha::attr(&c, "temperature_unit").and_then(Value::as_str).unwrap_or("°F").to_string();
    let doing = match action.as_str() {
        "heating" => "Heating",
        "cooling" => "Cooling",
        "idle" => "Idle",
        _ if mode == "off" => "Off",
        _ => "Idle",
    };
    let room = a.world.areas.iter().find(|r| r.2.iter().any(|e| e == ha::id(&c))).map(|r| r.1.clone()).unwrap_or_else(|| ha::name(&c));
    let dial = iced::widget::stack![
        canvas(Dial { frac: ((set - lo) / (hi - lo).max(1.0)) as f32, color: p.accent, track: Pal::mix(p.panel, p.line, 0.8) }).width(Length::Fixed(280.0)).height(Length::Fixed(230.0)),
        container(column![tb(format!("{}°", set.round()), 64.0, p.text), t(format!("{doing} · now {}{unit}", now_t.round()), 16.0, p.muted)].align_x(iced::Center)).width(Length::Fixed(280.0)).height(Length::Fixed(230.0)).center_x(Length::Fixed(280.0)).padding(Padding { top: 74.0, right: 0.0, bottom: 0.0, left: 0.0 }),
    ];
    let mode_chip = |m: &'static str, l: &str| w::chip(&p, l, mode == m, Sz::Lg, Some(Msg::Mode(m)));
    let buttons = row![w::chip(&p, "−", false, Sz::Lg, Some(Msg::Temp(-1.0))), mode_chip("heat", "Heat"), mode_chip("cool", "Cool"), mode_chip("off", "Off"), w::chip(&p, "＋", false, Sz::Lg, Some(Msg::Temp(1.0)))].spacing(10.0);
    tile(&p, column![k(&p, format!("Thermostat · {room}")), container(dial).center_x(Length::Fill), container(buttons).center_x(Length::Fill)].spacing(6.0).into(), a.focus == 1)
}

fn cameras_tile(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    let cams = a.cameras();
    let mut grid = Row::new().spacing(14.0);
    for c in &cams {
        let id = ha::id(c).to_string();
        let pic: Element<'_, Msg> = match a.cams.get(&id) {
            Some(h) => image(h.clone()).content_fit(iced::ContentFit::Cover).width(Length::Fill).height(Length::Fixed(220.0)).into(),
            None => container(Space::new()).width(Length::Fill).height(Length::Fixed(220.0)).style(ui::boxed(Pal::mix(p.bg, Color::from_rgb8(0x1a, 0x2a, 0x3a), 0.6), 0.0, None)).into(),
        };
        let over = container(column![w::tag(&p, "● Live", Tg::Live), Space::new().height(Length::Fill), tb(ha::name(c), 16.0, Color::WHITE)]).padding(12.0).width(Length::Fill).height(Length::Fixed(220.0));
        grid = grid.push(container(iced::widget::stack![pic, over]).clip(true).width(Length::FillPortion(1)).style(ui::boxed(Color::TRANSPARENT, 16.0, None)));
    }
    if cams.is_empty() {
        grid = grid.push(t("No cameras in Home Assistant yet.", 14.0, p.muted));
    }
    let motion: Vec<Value> = a.of("binary_sensor").into_iter().filter(|s| ha::attr(s, "device_class").and_then(Value::as_str) == Some("motion")).collect();
    let quiet = motion.iter().all(|m| ha::state(m) != "on");
    let last = motion.iter().max_by_key(|m| m.get("last_changed").and_then(Value::as_str).map(ha::iso_secs).unwrap_or(0));
    let last_line = last.map(|m| {
        let when = m.get("last_changed").and_then(Value::as_str).map(ha::iso_secs).unwrap_or(0);
        let what = ha::attr(m, "hearth_seen").and_then(Value::as_str).map(|x| format!(" · {x}")).unwrap_or_default();
        format!("Last movement: {} · {}{what}", ha::name(m).trim_end_matches(" motion").trim_end_matches(" Motion"), hearth_ui::time::clock(when))
    }).unwrap_or_default();
    tile(&p, column![row![k(&p, "Cameras"), Space::new().width(Length::Fill), w::tag(&p, if quiet { "All quiet" } else { "Movement now" }, if quiet { Tg::Ok } else { Tg::Warn })].align_y(iced::Center), grid, t(last_line, 14.0, p.muted)].spacing(14.0).into(), a.focus == 2)
}

fn plugs_tile(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    let plugs = a.plugs();
    let total: f64 = plugs.iter().filter_map(|s| a.watts(s)).sum();
    let mut list = Column::new();
    for (i, pl) in plugs.iter().enumerate() {
        let on = ha::state(pl) == "on";
        let wt = a.watts(pl);
        let note = ha::attr(pl, "hearth_note").and_then(Value::as_str).unwrap_or("");
        let right = match (on, wt) {
            (true, Some(x)) if x < 20.0 => format!("{} W · idle", x.round()),
            (true, Some(x)) => format!("{} W", x.round()),
            (false, _) if !note.is_empty() => note.to_string(),
            (false, _) => "off".into(),
            (true, None) => "on".into(),
        };
        let focus = a.focus == 3 && a.row == i;
        list = list.push(container(row![w::dot(if on { w::green() } else { p.muted }, 16.0), t(ha::name(pl), 18.0, p.text).width(Length::Fill), t(right, 14.0, p.muted), w::sw_k(&p, on, Some(Msg::Toggle(ha::id(pl).to_string())), 1.3)].spacing(12.0).align_y(iced::Center)).padding(Padding { top: 11.0, right: 0.0, bottom: 11.0, left: 0.0 }).style(ui::boxed(if focus { Pal::mix(p.panel, p.accent, 0.1) } else { Color::TRANSPARENT }, 8.0, None)));
        if i + 1 < plugs.len() {
            list = list.push(ui::hline(Pal::mix(p.line, p.bg, 0.4)));
        }
    }
    let kwh: f64 = a.world.energy.iter().sum();
    let top = a.world.energy.iter().cloned().fold(0.01f64, f64::max);
    let vals: Vec<f32> = a.world.energy.iter().map(|v| (v / top) as f32).collect();
    let bars = canvas(w::Bars { vals, color: p.accent, head: None, head_color: p.accent, bright_over: 0.8, centred: false }).width(Length::Fill).height(Length::Fixed(150.0));
    let hours = row![t("12 AM", 13.0, p.muted), Space::new().width(Length::Fill), t("6 AM", 13.0, p.muted), Space::new().width(Length::Fill), t("12 PM", 13.0, p.muted), Space::new().width(Length::Fill), t("6 PM", 13.0, p.muted), Space::new().width(Length::Fill), t("now", 13.0, p.muted)];
    tile(&p, column![row![k(&p, "Plugs"), Space::new().width(Length::Fill), t(format!("{} W now", total.round()), 14.0, p.muted)], list, container(k(&p, format!("Energy today · {kwh:.1} kWh"))).padding(Padding { top: 18.0, right: 0.0, bottom: 4.0, left: 0.0 }), bars, hours].spacing(6.0).into(), a.focus == 3)
}

fn scenes_tile(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    let scenes = a.of("scene");
    let mut grid = Column::new().spacing(14.0);
    let newest = scenes.iter().max_by_key(|s| s.get("state").and_then(Value::as_str).map(ha::iso_secs).unwrap_or(0)).map(|s| ha::id(s).to_string());
    for pair in scenes.chunks(2).take(2) {
        let mut rw = Row::new().spacing(14.0);
        for s in pair {
            let on = newest.as_deref() == Some(ha::id(s));
            let sub = ha::attr(s, "hearth_note").and_then(Value::as_str).map(str::to_string).unwrap_or_else(|| ha::attr(s, "entity_id").and_then(Value::as_array).map(|e| format!("{} things", e.len())).unwrap_or_default());
            let cell = container(column![tb(ha::name(s), 19.0, p.text), t(sub, 13.0, p.muted)].spacing(6.0)).padding(Padding { top: 0.0, right: 20.0, bottom: 0.0, left: 20.0 }).center_y(Length::Fixed(96.0)).width(Length::FillPortion(1)).style(ui::boxed(if on { Pal::mix(p.panel, p.accent, 0.15) } else { p.panel2 }, 16.0, Some((if on { p.accent } else { p.line }, 1.0))));
            rw = rw.push(mouse_area(cell).on_press(Msg::Scene(ha::id(s).to_string())).interaction(mouse::Interaction::Pointer));
        }
        grid = grid.push(rw);
    }
    let mut routines = Column::new();
    let mut next = a.world.next.clone();
    let now_s = hearth_oss::now() - day_start();
    next.sort_by_key(|(_, at)| if *at >= now_s { *at } else { *at + 86_400 });
    let cols = [p.accent, w::blue(), w::yellow()];
    for (i, (id, at)) in next.iter().take(3).enumerate() {
        let name = a.get(id).map(ha::name).unwrap_or_default();
        let h = at / 3600;
        let when = format!("{}:{:02} {}", if h % 12 == 0 { 12 } else { h % 12 }, at / 60 % 60, if h < 12 { "AM" } else { "PM" });
        routines = routines.push(container(row![w::dot(cols[i % 3], 16.0), t(name, 18.0, p.text).width(Length::Fill), t(when, 14.0, p.muted)].spacing(12.0).align_y(iced::Center)).padding(Padding { top: 11.0, right: 0.0, bottom: 11.0, left: 0.0 }));
        if i + 1 < next.len().min(3) {
            routines = routines.push(ui::hline(Pal::mix(p.line, p.bg, 0.4)));
        }
    }
    tile(&p, column![k(&p, "Scenes"), grid, container(k(&p, "Routines coming up")).padding(Padding { top: 22.0, right: 0.0, bottom: 4.0, left: 0.0 }), routines].spacing(14.0).into(), a.focus == 4)
}

fn signin(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    let st = a.status.get("homeassistant").cloned().unwrap_or(Value::Null);
    let mut c = column![w::mark("HM", "gr", 64.0), tb("Hearth Home", 30.0, p.text), t("Your home on the TV, the Ally and the phone, from Home Assistant on your Nvidia PC.", 15.0, p.muted)].spacing(14.0).align_x(iced::Center).width(Length::Fixed(560.0));
    if st.is_object() && !st.get("installed").and_then(Value::as_bool).unwrap_or(false) {
        c = c.push(w::card2(&p, column![t("Home Assistant isn't on this PC yet. Hearth installs its own image in your Podman, reached only here and over Tailscale.", 13.0, p.text), w::chip(&p, "Install Home Assistant", true, Sz::Md, Some(Msg::Install))].spacing(10.0)));
    } else if st.is_object() && !st.get("on").and_then(Value::as_bool).unwrap_or(false) {
        c = c.push(w::card2(&p, column![t("Home Assistant is installed but off.", 13.0, p.text), w::chip(&p, "Turn it on", true, Sz::Md, Some(Msg::TurnOn))].spacing(10.0)));
    }
    c = c.push(row![t("Home Assistant", 13.0, p.muted), w::input(&p, "http://nvidia-pc:8123", &a.prefs.s("server", "http://127.0.0.1:8123"), 13.0, Msg::Server)].spacing(10.0).align_y(iced::Center));
    c = c.push(w::secret_input(&p, "A long-lived token (your profile › Security in Home Assistant)", &a.token_typed, Msg::Token, Msg::SignIn));
    c = c.push(w::wide(&p, "Open my home", (!a.token_typed.trim().is_empty()).then_some(Msg::SignIn)));
    if !a.said.is_empty() {
        c = c.push(t(a.said.clone(), 13.0, p.muted));
    }
    container(c).center_x(Length::Fill).center_y(Length::Fill).into()
}

fn other_tab(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    let mut c = Column::new().spacing(10.0);
    match a.tab {
        "rooms" => {
            for r in &a.world.areas {
                c = c.push(mouse_area(row![tb(r.1.clone(), 20.0, p.text).width(Length::Fixed(300.0)), t(format!("{} things", r.2.len()), 15.0, p.muted)].spacing(14.0)).on_press(Msg::Room(r.0.clone())).interaction(mouse::Interaction::Pointer));
            }
        }
        "cameras" => c = c.push(cameras_tile(a)),
        "energy" => c = c.push(plugs_tile(a)),
        _ => {
            for s in a.of("automation") {
                let on = ha::state(&s) == "on";
                c = c.push(row![t(ha::name(&s), 18.0, p.text).width(Length::Fill), w::sw_k(&p, on, Some(Msg::Toggle(ha::id(&s).to_string())), 1.3)].spacing(14.0).align_y(iced::Center));
            }
        }
    }
    container(iced::widget::scrollable(c)).padding(Padding { top: 0.0, right: 56.0, bottom: 0.0, left: 56.0 }).height(Length::Fill).into()
}

use hearth_ui::blocks::{bl, PageDef};
const BLOCKS: &[PageDef] = &[
    PageDef { id: "window", title: "Hearth Home", blocks: &[bl("top", "Hearth Home, its tabs, the time", 0, 0, 12, 1), bl("board", "The dashboard", 0, 1, 12, 10), bl("foot", "Where it runs and the controller", 0, 11, 12, 1)] },
    PageDef {
        id: "board",
        title: "The dashboard",
        blocks: &[bl("lights", "Lights", 0, 0, 4, 12), bl("thermostat", "Thermostat", 4, 0, 4, 5), bl("cameras", "Cameras", 8, 0, 4, 5), bl("plugs", "Plugs and energy", 4, 5, 4, 7), bl("scenes", "Scenes and routines", 8, 5, 4, 7)],
    },
];

fn view(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    studio_values(a);
    // the top: the mark, the name, the tabs, the time
    let tab = |id: &'static str, l: &str| if a.tab == id { w::chip(&p, l, true, Sz::Lg, Some(Msg::Tab(id))) } else { w::ghost(&p, l, Sz::Lg, Some(Msg::Tab(id))) };
    let now = hearth_ui::time::local(hearth_oss::now());
    const DAYS: [&str; 7] = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
    let weather = a.of("weather").into_iter().next().map(|w| format!(" · {}° {}", ha::attr(&w, "temperature").and_then(Value::as_f64).map(|t| t.round()).unwrap_or(0.0), ha::state(&w).replace("clear-night", "clear").replace("sunny", "clear").replace('-', " "))).unwrap_or_default();
    let on = a.prefs.s("screenName", "the TV");
    let top = row![
        w::mark("HM", "gr", 44.0),
        tb("Hearth Home", 34.0, p.text),
        row![tab("home", "Home"), tab("rooms", "Rooms"), tab("cameras", "Cameras"), tab("energy", "Energy"), tab("routines", "Routines")].spacing(10.0).padding(Padding { top: 0.0, right: 0.0, bottom: 0.0, left: 36.0 }),
        Space::new().width(Length::Fill),
        column![t(hearth_ui::time::clock(hearth_oss::now()), 30.0, p.text).font(iced::Font { weight: iced::font::Weight::Light, ..app::INTER }), t(format!("{}{weather} · on {on}", DAYS[now.wday]), 15.0, p.muted)].align_x(iced::Right),
    ]
    .spacing(18.0)
    .align_y(iced::Center);
    let top = mouse_area(container(top).padding(Padding { top: 36.0, right: 56.0, bottom: 20.0, left: 56.0 })).on_press(Msg::Drag);
    let board: Element<'_, Msg> = if a.ha.is_none() && !a.loading {
        signin(a)
    } else if a.tab != "home" {
        other_tab(a)
    } else {
        let items: Vec<(&'static str, Element<'_, Msg>)> = vec![("lights", lights_tile(a)), ("thermostat", thermostat_tile(a)), ("cameras", cameras_tile(a)), ("plugs", plugs_tile(a)), ("scenes", scenes_tile(a))];
        let d = hearth_ui::arrange::parts(
            &a.describe,
            &p,
            "board",
            items,
            |ps| row![container(ps.take("lights")).width(Length::FillPortion(100)), column![ps.take("thermostat"), ps.take("plugs")].spacing(18.0).width(Length::FillPortion(100)), column![ps.take("cameras"), ps.take("scenes")].spacing(18.0).width(Length::FillPortion(105))].spacing(18.0).into(),
            Msg::Describe,
        );
        container(d).padding(Padding { top: 0.0, right: 52.0, bottom: 0.0, left: 52.0 }).height(Length::Fill).into()
    };
    let pb = |l: &str, round: bool| -> Element<'static, Msg> { container(tb(l.to_string(), 13.0, Color::WHITE)).center_x(Length::Fixed(if round { 28.0 } else { 44.0 })).center_y(Length::Fixed(28.0)).style(ui::boxed(if l == "A" { Color::from_rgb8(0xff, 0x8a, 0x3d) } else { Color::from_rgba(1.0, 1.0, 1.0, 0.15) }, 14.0, None)).into() };
    let hint = |b: Element<'static, Msg>, s: &str| -> Element<'static, Msg> { row![b, t(s.to_string(), 15.0, Color::from_rgb8(0xf1, 0xe9, 0xe2))].spacing(8.0).align_y(iced::Center).into() };
    let foot = container(
        row![
            t(format!("Hearth Home · runs on the {} · on the TV, the Ally and the phone", a.prefs.s("serverName", "Nvidia PC")), 15.0, p.muted),
            Space::new().width(Length::Fill),
            hint(pb("A", true), "Turn on / off"),
            hint(pb("◀ ▶", false), "Brightness"),
            hint(pb("Y", true), "Scenes"),
            hint(pb("B", true), "Back"),
            mouse_area(t("Edit layout", 13.0, p.muted)).on_press(Msg::Describe(hearth_ui::arrange::edit_layout())).interaction(mouse::Interaction::Pointer),
        ]
        .spacing(22.0)
        .align_y(iced::Center),
    )
    .padding(Padding { top: 16.0, right: 40.0, bottom: 16.0, left: 40.0 })
    .width(Length::Fill)
    .style(ui::boxed(Pal::mix(p.bg, Color::BLACK, 0.35), 0.0, None));
    let foot = column![w::credit(&p, "Hearth Home", "Built on Home Assistant (Apache-2.0)"), foot];
    let mut items: Vec<(&'static str, Element<'_, Msg>)> = vec![("top", top.into()), ("board", board), ("foot", foot.into())];
    for (id, which) in [("pane:lights", "lights"), ("pane:energy", "energy"), ("pane:scenes", "scenes")] {
        if a.describe.wants_pane(which) {
            items.push((id, studio_pane(a, which)));
        }
    }
    let base = hearth_ui::arrange::parts(&a.describe, &p, "window", items, |ps| column![ps.take("top"), container(ps.take("board")).height(Length::Fill), ps.take("foot")].into(), Msg::Describe);
    let mut all = Column::new().push(container(base).width(Length::Fill).height(Length::Fill).style(ui::boxed(p.bg, 0.0, None)));
    if !a.said.is_empty() && a.ha.is_some() {
        all = all.push(container(t(a.said.clone(), 13.0, p.muted)).padding(Padding { top: 4.0, right: 56.0, bottom: 4.0, left: 56.0 }));
    }
    all.into()
}

use hearth_ui::studio::{tile as ktile, Kit};
use hearth_ui::ui::G;
const KIT: Kit = Kit {
    name: "Home",
    buttons: &[
        ktile("all-off", "Every light off", G::Power),
        ktile("brighter", "Brighter", G::Plus),
        ktile("dimmer", "Dimmer", G::Minimize),
        ktile("warmer", "Warmer", G::Plus),
        ktile("cooler", "Cooler", G::Minimize),
        ktile("rooms", "Rooms", G::Home),
        ktile("cameras", "Cameras", G::Camera),
        ktile("energy", "Energy", G::Pulse),
        ktile("routines", "Routines", G::Clock),
    ],
    panes: &[ktile("lights", "Every light", G::Leaf), ktile("energy", "Energy today", G::Pulse), ktile("scenes", "Scenes", G::Grid)],
    info: &[ktile("on", "Lights on", G::Leaf), ktile("watts", "Watts now", G::Pulse), ktile("inside", "Inside", G::Home), ktile("kwh", "kWh today", G::Pulse)],
    pane_page: "window",
    bars: &[],
    menus: &[],
};

fn block_action(a: &App, id: &str) -> Option<Msg> {
    Some(match id {
        "brighter" => Msg::Bright((a.bright + 10.0).min(100.0)),
        "dimmer" => Msg::Bright((a.bright - 10.0).max(1.0)),
        "warmer" => Msg::Temp(1.0),
        "cooler" => Msg::Temp(-1.0),
        "rooms" => Msg::Tab("rooms"),
        "cameras" => Msg::Tab("cameras"),
        "energy" => Msg::Tab("energy"),
        "routines" => Msg::Tab("routines"),
        "all-off" => Msg::AllOff,
        _ => return None,
    })
}

fn studio_values(a: &App) {
    let watts: f64 = a.plugs().iter().filter_map(|s| a.watts(s)).sum();
    let inside = a.of("climate").first().and_then(|c| ha::attr(c, "current_temperature").and_then(Value::as_f64)).map(|t| format!("{}°", t.round())).unwrap_or_default();
    a.describe.values(vec![
        ("on", a.of("light").iter().filter(|s| ha::state(s) == "on").count().to_string()),
        ("watts", format!("{} W", watts.round())),
        ("inside", inside),
        ("kwh", format!("{:.1}", a.world.energy.iter().sum::<f64>())),
    ]);
}

fn studio_pane<'a>(a: &'a App, which: &str) -> Element<'a, Msg> {
    let p = a.p;
    let mut c = Column::new().spacing(6.0);
    match which {
        "lights" => {
            for l in a.of("light") {
                c = c.push(row![t(ha::name(&l), 14.0, p.text).width(Length::Fill), w::sw(&p, ha::state(&l) == "on", Some(Msg::Toggle(ha::id(&l).to_string())))]);
            }
        }
        "energy" => c = c.push(t(format!("{:.1} kWh today", a.world.energy.iter().sum::<f64>()), 14.0, p.text)),
        _ => {
            for s in a.of("scene") {
                c = c.push(w::chip(&p, ha::name(&s), false, Sz::Sm, Some(Msg::Scene(ha::id(&s).to_string()))));
            }
        }
    }
    iced::widget::scrollable(c).height(Length::Fill).into()
}

fn subscription(a: &App) -> Subscription<Msg> {
    let mut s = vec![Subscription::run(hearth_ui::theme::watch).map(|_| Msg::Theme), iced::keyboard::listen().map(Msg::Key)];
    // what your home says, again every 10 s while it's open and signed in (not in CI's pictures)
    if a.ha.is_some() && !a.fixture {
        s.push(iced::time::every(std::time::Duration::from_secs(10)).map(|_| Msg::Tick));
    }
    // the cameras as moving video (Home Assistant's camera_proxy_stream), only while their tile is shown
    if a.tab == "home" || a.tab == "cameras" {
        if let Some((base, token)) = a.stream_from() {
            for c in a.cameras() {
                let cam = mjpeg::Cam { base: base.clone(), token: token.clone(), id: ha::id(&c).to_string() };
                s.push(Subscription::run_with(cam, mjpeg::frames).map(|(id, b)| Msg::Moving(id, b)));
            }
        }
    }
    Subscription::batch(s)
}

fn shown(a: &App) -> Element<'_, Msg> {
    hearth_ui::describe::with_bar(view(a), &a.p, &a.describe, Msg::Describe)
}

fn main() -> iced::Result {
    if app::handle_cli("hearth-home-app", "Hearth Home: your home on the big screen (built on Home Assistant, Apache-2.0).") {
        return Ok(());
    }
    let w = app::window("hearth-home", (1920.0, 1080.0), (1100.0, 640.0));
    iced::application(App::new, update, shown).title(|_: &App| "Hearth Home".to_string()).subscription(subscription).theme(|a: &App| app::iced_theme(&a.p)).window(w).default_font(app::INTER).run()
}
