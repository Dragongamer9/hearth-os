//! hearth-home-server-app: the Home server helper (docs/HOME-SERVER-PARTS.md; boards docs/mockups/server-expansion,
//! approved Nov 26, 2026). One window for the Server side of Hearth OS: its sidebar (Server home, then every part from
//! hearth_server_parts::parts, one line each), the This server box, and each part's page from src/pages (one module
//! per page, listed once in pages/mod.rs). Made of blocks like every Hearth app (`.with_pages(BLOCKS).with_kit(&KIT)`).
//!
//! Everything shown comes from the parts' engines (`<engine> status`, read when a page opens and again while it's open
//! at the page's own pace: the Dashboard every 2 seconds), and every button is one of an engine's commands; nothing
//! here writes a setting itself (the engines write theirs safely, red flag 9). The R2 keys you type go straight into
//! the system's password store (secret-tool), never into a file, a command line or a log.
//!
//! The controller (hearth-pad's keys): A (Enter) does the lit thing, X (x) Show me, Y (y) Edit layout, LB / RB (Tab)
//! the TV view's pages, B (Esc) back. `--page home|dash|tv|phone|vault|checks|backup` opens a page (the menu's
//! actions, CI's pictures); $HEARTH_SERVER_FIXTURES: made-up answers for CI's pictures (nothing runs).

mod pages;

use hearth_oss::w::Tg;
use hearth_server_parts::home::{self, HAct, HomeData};
use hearth_server_parts::{engine, parts};
use hearth_ui::app::{self, win};
use hearth_ui::describe::DMsg;
use hearth_ui::run;
use hearth_ui::ui::{self, t, tb, TitleMsgs, G};
use hearth_ui::Pal;
use iced::keyboard::{key::Named, Event as KEvent, Key};
use iced::widget::{column, container, row, Space};
use iced::{Color, Element, Length, Size, Subscription, Task};
use serde_json::{json, Value};
use std::collections::HashMap;

pub type E<'a> = Element<'a, Msg>;

/// One of an engine's commands (or, with no engine, a program: pkexec hearth-admin, the Store, Settings).
#[derive(Debug, Clone, Default)]
pub struct Act {
    pub engine: &'static str,
    pub args: Vec<String>,
    pub cmd: Vec<String>,
    /// what to say when it worked
    pub say: String,
    /// asked twice first (removing a model): this is the question
    pub confirm: String,
    /// started and not waited for (a window: Settings, the Store, Hearth Forge)
    pub detach: bool,
    /// how many times it's been asked (removing a model asks twice)
    pub asked: u8,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Describe(DMsg),
    Theme,
    Tick,
    HomeTick,
    Home(Box<HomeData>),
    Got(&'static str, run::Output),
    Go(&'static str),
    /// a page's own choice (its tab, the picked row), kept only while the window is open
    Set(String, Value),
    /// typing in a field
    Text(String, String),
    Do(Box<Act>),
    Did(Box<Act>, run::Output),
    /// the confirm box: yes, no
    Confirm(bool),
    HomeAct(HAct),
    /// an R2 key typed into its field goes into the keyring (which one: "access" or "secret")
    SaveKey(&'static str),
    KeySaved(&'static str, run::Output),
    Key(KEvent),
    Drag,
    Max,
    Min,
    Close,
    Resize(iced::window::Direction),
    Resized(Size),
    /// feature/server-2's pages (pages/more.rs)
    More(hearth_server_more::Msg),
}

pub struct App {
    pub describe: hearth_ui::describe::Describe,
    pub p: Pal,
    pub page: &'static str,
    /// each engine's last `status`
    pub data: HashMap<&'static str, Value>,
    pub home: HomeData,
    pub ui: HashMap<String, Value>,
    pub said: String,
    pub bad: bool,
    pub ask: Option<Act>,
    pub size: Size,
    pub fixture: bool,
    reading: Vec<&'static str>,
    /// feature/server-2's pages (pages/more.rs)
    pub more: hearth_server_more::Hub,
}

impl App {
    fn new() -> (App, Task<Msg>) {
        let start = app::arg("--page").and_then(|p| pages::PAGES.iter().find(|x| x.id == p).map(|x| x.id)).unwrap_or("home");
        let mut a = App {
            describe: hearth_ui::describe::Describe::new("home-server", "Home server helper").with_pages(pages::BLOCKS).with_kit(&KIT),
            p: hearth_ui::theme::load(),
            page: start,
            data: HashMap::new(),
            home: HomeData::default(),
            ui: HashMap::new(),
            said: String::new(),
            bad: false,
            ask: None,
            size: Size::new(1920.0, 1080.0),
            fixture: engine::fixtures().is_some(),
            reading: Vec::new(),
            more: hearth_server_more::Hub::new(),
        };
        // CI's pictures: --set key=value picks a tab or a row (--set vaultTab=Colibri)
        let args: Vec<String> = std::env::args().collect();
        for w in args.windows(2) {
            if w[0] == "--set" {
                if let Some((k, v)) = w[1].split_once('=') {
                    a.ui.insert(k.to_string(), json!(v));
                }
            }
        }
        let t = Task::batch([a.read_page(), Task::perform(home::load(), |d| Msg::Home(Box::new(d))), pages::more::load_all()]);
        (a, t)
    }

    pub fn pg(&self) -> &'static pages::Page {
        pages::PAGES.iter().find(|x| x.id == self.page).unwrap_or(&pages::PAGES[0])
    }

    /// The open page's engine `status`, read again.
    fn read_page(&mut self) -> Task<Msg> {
        let mut t = Vec::new();
        for &e in self.pg().engines {
            if self.reading.contains(&e) {
                continue;
            }
            self.reading.push(e);
            let e2: &'static str = e;
            let mut args = vec!["status".to_string()];
            // the dashboard's window of time (Last 24 h ▾)
            if e2 == "dash" {
                args.push(self.ui_s("dashRange", "24h"));
            }
            t.push(Task::perform(engine::spawn(e2, args, String::new()), move |o| Msg::Got(e2, o)));
        }
        Task::batch(t)
    }

    pub fn d(&self, engine: &str) -> &Value {
        self.data.get(engine).unwrap_or(&Value::Null)
    }
    pub fn ui_s(&self, k: &str, def: &str) -> String {
        self.ui.get(k).and_then(|v| v.as_str().map(str::to_string).or_else(|| (!v.is_null()).then(|| v.to_string()))).unwrap_or_else(|| def.to_string())
    }
    pub fn ui_b(&self, k: &str) -> bool {
        self.ui.get(k).map(|v| v.as_bool().unwrap_or(v.as_str() == Some("true"))).unwrap_or(false)
    }
    pub fn ui_n(&self, k: &str) -> usize {
        self.ui.get(k).and_then(|v| v.as_u64().or_else(|| v.as_str().and_then(|s| s.parse().ok()))).unwrap_or(0) as usize
    }

    fn run(&mut self, a: Act) -> Task<Msg> {
        if !a.confirm.is_empty() && self.ask.is_none() {
            self.ask = Some(a);
            return Task::none();
        }
        if a.detach {
            if !self.fixture {
                let v: Vec<&str> = a.cmd.iter().map(|s| s.as_str()).collect();
                run::detached(&v);
            }
            return Task::none();
        }
        let b = Box::new(a.clone());
        if a.engine.is_empty() {
            if self.fixture {
                return Task::done(Msg::Did(b, run::Output { ok: true, code: 0, out: "{\"ok\": true}".into(), err: String::new() }));
            }
            return Task::perform(run::spawn(a.cmd.clone()), move |o| Msg::Did(b.clone(), o));
        }
        Task::perform(engine::spawn(a.engine, a.args.clone(), String::new()), move |o| Msg::Did(b.clone(), o))
    }
}

/// An engine's command as a button's message.
pub fn act(engine: &'static str, args: &[&str], say: &str) -> Msg {
    Msg::Do(Box::new(Act { engine, args: args.iter().map(|s| s.to_string()).collect(), say: say.into(), ..Default::default() }))
}
pub fn act_v(engine: &'static str, args: Vec<String>, say: &str) -> Msg {
    Msg::Do(Box::new(Act { engine, args, say: say.into(), ..Default::default() }))
}
/// The same, asked twice first.
pub fn act_ask(engine: &'static str, args: Vec<String>, question: &str, say: &str) -> Msg {
    Msg::Do(Box::new(Act { engine, args, say: say.into(), confirm: question.into(), ..Default::default() }))
}
/// A window (Settings' page, the Store, an app), started and not waited for.
pub fn open(cmd: &[&str]) -> Msg {
    Msg::Do(Box::new(Act { cmd: cmd.iter().map(|s| s.to_string()).collect(), detach: true, ..Default::default() }))
}
pub fn set(k: &str, v: Value) -> Msg {
    Msg::Set(k.to_string(), v)
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
            if let Some(t) = pages::more::open(a.page) {
                return t;
            }
            return a.read_page();
        }
        Msg::HomeTick => return Task::perform(home::load(), |d| Msg::Home(Box::new(d))),
        Msg::Home(d) => a.home = *d,
        Msg::Got(e, o) => {
            a.reading.retain(|x| *x != e);
            match o.json() {
                Some(v) if v.get("ok").and_then(Value::as_bool) != Some(false) => {
                    a.data.insert(e, v);
                }
                _ => {
                    a.data.insert(e, json!({"ok": false, "error": engine::problem(&o)}));
                }
            }
        }
        Msg::Go(p) => {
            if let Some(pg) = pages::PAGES.iter().find(|x| x.id == p) {
                a.page = pg.id;
                a.said.clear();
                if let Some(t) = pages::more::open(p) {
                    return t;
                }
                return a.read_page();
            }
        }
        Msg::More(hearth_server_more::Msg::D(dm)) => return update(a, Msg::Describe(dm)),
        Msg::More(m) => return a.more.update(m).map(Msg::More),
        Msg::Set(k, v) => {
            let refresh = k == "dashRange";
            a.ui.insert(k, v);
            if refresh {
                return a.read_page();
            }
        }
        Msg::Text(k, v) => {
            a.ui.insert(k, json!(v));
        }
        Msg::Do(x) => return a.run(*x),
        Msg::Confirm(yes) => {
            if let Some(mut x) = a.ask.take() {
                if yes && x.asked == 0 {
                    // asked twice: the second time plainly
                    x.asked = 1;
                    x.confirm = "Are you sure? It can't be undone from here.".into();
                    a.ask = Some(x);
                } else if yes {
                    x.confirm.clear();
                    return a.run(x);
                }
            }
        }
        Msg::Did(x, o) => {
            let why = engine::problem(&o);
            // what a command answered stays for its page (a night's files, a join code): ui "last.<engine>.<command>"
            if let (false, Some(first), Some(v)) = (x.engine.is_empty(), x.args.first(), o.json()) {
                a.ui.insert(format!("last.{}.{}", x.engine, first), v);
            }
            if why.is_empty() {
                a.bad = false;
                let said = o.json().and_then(|v| v.get("said").and_then(Value::as_str).map(str::to_string)).unwrap_or_default();
                a.said = if said.is_empty() { x.say.clone() } else { said };
            } else {
                a.bad = true;
                a.said = why;
            }
            return Task::batch([a.read_page(), Task::perform(home::load(), |d| Msg::Home(Box::new(d)))]);
        }
        Msg::HomeAct(h) => return home_act(a, h),
        Msg::SaveKey(which) => {
            let k = format!("r2{which}");
            let typed = a.ui.remove(&k).and_then(|v| v.as_str().map(str::to_string)).unwrap_or_default();
            if typed.trim().is_empty() {
                return Task::none();
            }
            if a.fixture {
                return Task::done(Msg::KeySaved(which, run::Output { ok: true, code: 0, out: String::new(), err: String::new() }));
            }
            let label = if which == "access" { "Hearth Model vault: Cloudflare R2 access key" } else { "Hearth Model vault: Cloudflare R2 secret" };
            return Task::perform(hearth_oss::secret::store("home-server", &format!("r2-{which}"), label, typed.trim().to_string()), move |o| Msg::KeySaved(which, o));
        }
        Msg::KeySaved(which, o) => {
            if o.ok {
                a.bad = false;
                a.said = format!("Your R2 {} is in the system's password store", if which == "access" { "access key" } else { "secret" });
                return a.read_page();
            }
            a.bad = true;
            a.said = "The key couldn't be kept: the system's password store didn't answer (nothing was saved anywhere else)".into();
        }
        Msg::Key(ev) => return key(a, ev),
        Msg::Drag => return win::drag(),
        Msg::Max => return win::toggle_maximize(),
        Msg::Min => return win::minimize(),
        Msg::Close => return win::close(),
        Msg::Resize(d) => return win::resize(d),
        Msg::Resized(s) => a.size = s,
    }
    Task::none()
}

/// What Server home's buttons do here.
fn home_act(a: &mut App, h: HAct) -> Task<Msg> {
    match h {
        HAct::Open(id) => return update(a, Msg::Go(id)),
        HAct::Dashboard => return update(a, Msg::Go("dash")),
        HAct::Fix => {
            let first = hearth_server_parts::arr(a.home.overall(), "needs").first().map(|n| hearth_server_parts::s(n, "id")).unwrap_or_default();
            if !first.is_empty() {
                return a.run(Act { engine: "dash", args: vec!["fix".into(), first], say: "Fixed".into(), ..Default::default() });
            }
        }
        HAct::ServerMode => return a.run(Act { cmd: vec!["hearth-settings-app".into(), "--page".into(), "hs-server".into()], detach: true, ..Default::default() }),
        HAct::Update(id) | HAct::Test(id) if a.fixture => {
            a.said = format!("{id}: the Store installs it");
        }
        HAct::Update(id) => {
            let u = a.home.up(&id).cloned().unwrap_or_else(|| hearth_server_parts::store::Up { id: id.clone(), system: id == parts::HELPER_PART, ..Default::default() });
            let cmd = hearth_server_parts::store::install_cmd(&u, false);
            let detach = !u.system;
            return a.run(Act { cmd, detach, say: "Updated from the Hearth Store".into(), ..Default::default() });
        }
        HAct::Test(id) => {
            let u = a.home.up(&id).cloned().unwrap_or_default();
            let cmd = hearth_server_parts::store::install_cmd(&u, true);
            return a.run(Act { cmd, detach: !u.system, say: "The test version is in: the one before stays, to go back to".into(), ..Default::default() });
        }
        HAct::Notes(id) => {
            let open = a.ui_s("notes", "") == id;
            a.ui.insert("notes".into(), json!(if open { String::new() } else { id }));
        }
        HAct::Auto(on) => return a.run(Act { engine: "dash", args: vec!["autoupdate".into(), if on { "on" } else { "off" }.into()], say: if on { "Server apps update by themselves now (only from the Hearth Store)".into() } else { "Server apps update only when you press Update".into() }, ..Default::default() }),
        HAct::Idle(on) => return a.run(Act { engine: "dash", args: vec!["set".into(), "onlyIdle".into(), if on { "on" } else { "off" }.into()], say: String::new(), ..Default::default() }),
        HAct::Also(go) => return a.run(Act { cmd: vec!["hearth-settings-app".into(), "--page".into(), go.into()], detach: true, ..Default::default() }),
        HAct::EditLayout => return update(a, Msg::Describe(hearth_ui::arrange::edit_layout())),
    }
    Task::none()
}

fn key(a: &mut App, ev: KEvent) -> Task<Msg> {
    let KEvent::KeyPressed { key, modifiers, .. } = ev else { return Task::none() };
    if modifiers.control() && matches!(key.as_ref(), Key::Character("q")) {
        return win::close();
    }
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
    if a.ask.is_some() {
        return match key.as_ref() {
            Key::Named(Named::Enter) => update(a, Msg::Confirm(true)),
            Key::Named(Named::Escape) => update(a, Msg::Confirm(false)),
            _ => Task::none(),
        };
    }
    if let Some(m) = (a.pg().key)(a, key.as_ref(), modifiers.shift()) {
        return update(a, m);
    }
    match key.as_ref() {
        Key::Character("y") => update(a, Msg::Describe(hearth_ui::arrange::edit_layout())),
        Key::Named(Named::Escape) => {
            let back = a.pg().back;
            if back.is_empty() { Task::none() } else { update(a, Msg::Go(back)) }
        }
        _ => Task::none(),
    }
}

// ------------------------------------------------------------------ the window

use hearth_ui::blocks::{bl, PageDef};
/// The window's own blocks: its sidebar and the page.
pub const WINDOW: PageDef = PageDef { id: "window", title: "Home server helper", blocks: &[bl("side", "The sidebar: the server's parts and This server", 0, 0, 2, 12), bl("page", "The page", 2, 0, 10, 12)] };

fn sidebar(a: &App) -> E<'_> {
    let p = a.p;
    let on = a.pg().side;
    let q = a.ui_s("sideSearch", "").to_lowercase();
    let mut c = iced::widget::Column::new().spacing(2.0);
    c = c.push(hearth_oss::w::search(&p, "Search the server", &a.ui_s("sideSearch", ""), |s| Msg::Text("sideSearch".into(), s)));
    c = c.push(container(tb("HOME SERVER", 11.0, p.muted)).padding(home_pad(14.0, 0.0, 8.0, 0.0)));
    let mut items: Vec<(&'static str, &'static str, String)> = vec![("home", "Server home", String::new())];
    for pt in parts::sorted() {
        let n = home::count(&a.home, pt.id);
        items.push((pt.id, pt.name, if n.is_empty() { pages::more::count(a, pt.id) } else { n }));
    }
    for (id, name, n) in items {
        if !q.is_empty() && !name.to_lowercase().contains(&q) {
            continue;
        }
        c = c.push(hearth_oss::w::nav(&p, None, name, n, on == id, Msg::Go(id)));
    }
    // This server: what the Dashboard's summary says about this PC
    let sv = hearth_server_parts::at(a.home.overall(), "server");
    c = c.push(container(tb("THIS SERVER", 11.0, p.muted)).padding(home_pad(22.0, 0.0, 8.0, 0.0)));
    if sv.is_null() {
        c = c.push(t(if a.home.loaded { "The Dashboard part says what this PC is doing" } else { "Looking…" }, 12.0, p.muted));
    } else {
        c = c.push(row![hearth_server_parts::draw::st(&hearth_server_parts::s(sv, "state"), 10.0), t(hearth_server_parts::s(sv, "name"), 12.0, p.text)].spacing(6.0).align_y(iced::Center));
        for l in hearth_server_parts::strs(sv, "lines") {
            c = c.push(t(l, 12.0, p.muted));
        }
    }
    c = c.push(Space::new().height(Length::Fixed(12.0)));
    c = c.push(row![t("●", 12.0, p.accent), t("Updates in the Hearth Store", 12.0, p.text)].spacing(6.0));
    c = c.push(t("every part is its own Store app · never a Hearth OS update", 12.0, p.muted));
    c = c.push(Space::new().height(Length::Fill));
    c = c.push(row![t("Made of blocks · ", 12.0, p.muted), iced::widget::mouse_area(tb("Edit layout", 12.0, p.accent)).on_press(Msg::Describe(hearth_ui::arrange::edit_layout())).interaction(iced::mouse::Interaction::Pointer)]);
    container(c).padding(home_pad(14.0, 12.0, 16.0, 12.0)).width(Length::Fixed(230.0)).height(Length::Fill).style(ui::boxed(Pal::mix(p.bg, Color::BLACK, 0.18), 0.0, None)).into()
}

pub fn home_pad(t: f32, r: f32, b: f32, l: f32) -> iced::Padding {
    iced::Padding { top: t, right: r, bottom: b, left: l }
}

fn title(a: &App) -> E<'_> {
    let p = a.p;
    let pg = a.pg();
    let mut mid = row![t(format!("· {}", pg.title), 14.0, p.muted).wrapping(iced::widget::text::Wrapping::None), Space::new().width(Length::Fill)].spacing(8.0).align_y(iced::Center);
    for (label, k) in (pg.tags)(a) {
        mid = mid.push(hearth_oss::w::tag(&p, label, k));
    }
    let mid = iced::widget::mouse_area(container(mid).width(Length::Fill).center_y(Length::Fixed(44.0)).padding(home_pad(0.0, 10.0, 0.0, 6.0))).on_press(Msg::Drag).on_double_click(Msg::Max).on_right_press(Msg::Describe(DMsg::Open));
    hearth_ui::describe::title_bar_with(&p, &a.describe, hearth_oss::w::mark("HS", "or", 22.0), "Home server helper", Some(mid.into()), TitleMsgs { drag: Msg::Drag, maximize: Msg::Max, minimize: Some(Msg::Min), close: Msg::Close }, Pal::mix(p.bg, Color::BLACK, 0.2), Msg::Describe(DMsg::Open))
}

/// What happened last (a fix done, why something didn't work), under the page.
pub fn said(a: &App) -> E<'_> {
    if a.said.is_empty() {
        return Space::new().height(Length::Fixed(0.0)).into();
    }
    let p = a.p;
    container(t(a.said.clone(), 12.0, if a.bad { hearth_oss::w::red() } else { p.text })).padding(home_pad(6.0, 12.0, 6.0, 12.0)).width(Length::Fill).style(ui::boxed(p.panel2, p.r(8.0), Some((p.line, 1.0)))).into()
}

fn view(a: &App) -> E<'_> {
    let p = a.p;
    studio_values(a);
    let pg = a.pg();
    // the TV and the phone: the page on its own, no window around it
    if pg.bare {
        return container((pg.view)(a)).width(Length::Fill).height(Length::Fill).style(ui::boxed(p.bg, 0.0, None)).into();
    }
    let items: Vec<(&'static str, E<'_>)> = vec![("side", hearth_ui::studio::made(&a.describe, "window", "side", || sidebar(a))), ("page", (pg.view)(a))];
    let body = hearth_ui::arrange::parts(&a.describe, &p, "window", items, move |ps| row![ps.take("side"), ui::vline(p.line), container(ps.take("page")).width(Length::Fill).height(Length::Fill)].height(Length::Fill).into(), Msg::Describe);
    let mut all = column![title(a), ui::hline(p.line), container(body).width(Length::Fill).height(Length::Fill)];
    if let Some(x) = &a.ask {
        all = all.push(confirm_bar(a, &x.confirm));
    }
    let framed = container(all).width(Length::Fill).height(Length::Fill).style(ui::boxed(p.bg, p.window_corner, Some((p.line, 1.0))));
    ui::edges(framed.into(), Msg::Resize)
}

/// Asked twice: the question, Yes and No (Enter, Esc).
fn confirm_bar<'a>(a: &'a App, q: &str) -> E<'a> {
    let p = a.p;
    let r = row![t(q.to_string(), 13.0, p.text), Space::new().width(Length::Fill), hearth_oss::w::chip(&p, "No, keep it", false, hearth_oss::w::Sz::Md, Some(Msg::Confirm(false))), hearth_oss::w::chip(&p, "Yes", true, hearth_oss::w::Sz::Md, Some(Msg::Confirm(true)))].spacing(10.0).align_y(iced::Center);
    container(r).padding(home_pad(12.0, 18.0, 12.0, 18.0)).width(Length::Fill).style(ui::boxed(Pal::mix(p.panel, hearth_oss::w::red(), 0.12), 0.0, Some((p.line, 1.0)))).into()
}

use hearth_ui::studio::{tile as ktile, Kit};
const KIT: Kit = Kit {
    name: "Home server helper",
    buttons: &[
        ktile("fix", "Fix everything that needs it", G::Check),
        ktile("check-models", "Check the models now", G::Sync),
        ktile("backup-all", "Back up all now", G::Drive),
        ktile("dashboard", "Dashboard", G::Pulse),
        ktile("vault", "Model vault", G::Drive),
        ktile("backups", "Backups", G::Clock),
        ktile("tv", "The TV view", G::Display),
    ],
    panes: &[ktile("needs", "Needs a look", G::Info), ktile("devices", "Your devices", G::Users)],
    info: &[ktile("healthy", "Services healthy", G::Check), ktile("kept", "Models kept", G::Drive), ktile("backed", "Devices backed up", G::Clock)],
    pane_page: "window",
    bars: &[],
    menus: &[],
};

fn block_action(_a: &App, id: &str) -> Option<Msg> {
    Some(match id {
        "fix" => act("dash", &["fix", "all"], "Fixed what needed it"),
        "check-models" => act("vault", &["check"], "Checked every model against Hugging Face"),
        "backup-all" => act("backups", &["now", "all"], "Every device backs up now"),
        "dashboard" => Msg::Go("dash"),
        "vault" => Msg::Go("vault"),
        "backups" => Msg::Go("backup"),
        "tv" => Msg::Go("tv"),
        _ => return None,
    })
}

fn studio_values(a: &App) {
    let (h, tot) = home::healthy(&a.home);
    a.describe.values(vec![
        ("healthy", if tot > 0.0 { format!("{h} of {tot}") } else { String::new() }),
        ("kept", home::count(&a.home, "vault")),
        ("backed", home::count(&a.home, "backup")),
    ]);
}

fn subscription(a: &App) -> Subscription<Msg> {
    let mut s = vec![Subscription::run(hearth_ui::theme::watch).map(|_| Msg::Theme), iced::keyboard::listen().map(Msg::Key), iced::window::resize_events().map(|(_, s)| Msg::Resized(s))];
    // read again only while the page is open, at its own pace (none in CI's pictures)
    let every = a.pg().every;
    if every > 0 && !a.fixture {
        s.push(iced::time::every(std::time::Duration::from_secs(every)).map(|_| Msg::Tick));
    }
    if a.page == "home" && !a.fixture {
        s.push(iced::time::every(std::time::Duration::from_secs(30)).map(|_| Msg::HomeTick));
    }
    Subscription::batch(s)
}

fn shown(a: &App) -> Element<'_, Msg> {
    hearth_ui::describe::with_bar(view(a), &a.p, &a.describe, Msg::Describe)
}

/// The tags a page shows at the right of the title bar.
pub type Tags = Vec<(String, Tg)>;

fn main() -> iced::Result {
    if app::handle_cli("hearth-home-server-app", "The Home server helper: the Server side of Hearth OS in one window (--page home|dash|tv|phone|vault|checks|backup).") {
        return Ok(());
    }
    let w = app::window("hearth-home-server", (1840.0, 980.0), (1100.0, 640.0));
    iced::application(App::new, update, shown).title(|a: &App| format!("Home server helper · {}", a.pg().title)).subscription(subscription).theme(|a: &App| app::iced_theme(&a.p)).window(w).default_font(app::INTER).run()
}
