//! hearth-family-app: Family and friends with Moonlight (hearth-link's moonlight.rs; docs/HEARTH-LINK.md "Family and
//! friends with Moonlight"). Three windows:
//!
//! - `--ask --name N --what desktop|games --limit MIN`: on the PC being connected to, "N wants to connect" with Allow,
//!   Deny and Always allow N. Prints allow, deny or always and closes; nobody answering in 60 s is a no. Hearth Link
//!   starts this and waits for it.
//! - `--notice --name N --what W --until MS`: while N is connected, a small window that stays on screen and in the
//!   taskbar ("N is connected (Moonlight)") with **Stop sharing now** (`hearth-link call moonlight.stop`). It has no
//!   close button: it goes when the session ends (Hearth Link closes it) or with Stop sharing now.
//! - `--page friends` (the default): on the connecting PC, the PCs that let this one connect, online or not (each
//!   asked once when this opens), Connect, Remove and Add with a code. Hearth Games, Steam Game Mode's Quick Access
//!   and Firehold open this (or show the same list from `hearth-link call moonlight.friends`).
//!
//! `--layout b` (or $HEARTH_FAMILY_LAYOUT=b, or a controller look in Accessibility): the controller look, bigger,
//! with the focus ring and the buttons' hints. Mouse, keyboard and the controller (as keys): Up / Down between rows,
//! Left / Right between buttons, Enter presses, Esc closes (Deny on the question).
//! $HEARTH_FAMILY_FIXTURE: a JSON file read instead of Hearth Link (CI's pictures).

use hearth_ui::app::{self, win};
use hearth_ui::run;
use hearth_ui::ui::{self, gap, glyph, t, tb, tm, TitleMsgs, G};
use hearth_ui::Pal;
use iced::keyboard::{key::Named, Event as KEvent, Key};
use iced::widget::{button, column, container, mouse_area, row, scrollable, text_input, Column, Space};
use iced::{mouse, Background, Border, Color, Element, Length, Padding, Shadow, Subscription, Task, Theme};
use serde_json::{json, Value};

#[derive(Debug, Clone, Copy, PartialEq)]
enum Mode {
    Ask,
    Notice,
    Friends,
}

#[derive(Debug, Clone)]
enum Msg {
    /// Describe it (hearth_ui::describe; every Hearth app has it)
    Describe(hearth_ui::describe::DMsg),
    /// Edit layout right there (a right-click on a part or the title bar): Arrange on this page, this part picked
    EditAt(&'static str, Option<&'static str>),
    Theme,
    Tick,
    Key(KEvent),
    Answer(&'static str),
    Stop,
    Stopped(run::Output),
    Got(run::Output),
    Did(String, run::Output),
    Connect(String, String),
    Remove(String, String),
    Disconnect,
    Code(String),
    Join,
    Focus(usize, usize),
    Drag,
    Nothing,
    Close,
}

struct App {
    pub describe: hearth_ui::describe::Describe,
    p: Pal,
    mode: Mode,
    big: bool,
    name: String,
    what: String,
    limit: u64,
    until: u64,
    started: u64,
    data: Value,
    loading: bool,
    busy: String,
    said: (String, bool),
    code: String,
    /// The focused row and button (controller and keys).
    fr: usize,
    fb: usize,
}

fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}
fn gs(v: &Value, k: &str) -> String {
    v.get(k).and_then(Value::as_str).unwrap_or("").to_string()
}
fn gb(v: &Value, k: &str) -> bool {
    v.get(k).and_then(Value::as_bool).unwrap_or(false)
}
fn what_word(w: &str) -> &'static str {
    if w == "games" { "Games only" } else { "Whole desktop" }
}
fn limit_word(m: u64) -> String {
    match m {
        0 => String::new(),
        60 => "for up to 1 hour".into(),
        120 => "for up to 2 hours".into(),
        180 => "for up to 3 hours".into(),
        m => format!("for up to {m} minutes"),
    }
}
fn left(until: u64) -> String {
    if until == 0 {
        return "No time limit".into();
    }
    let m = until.saturating_sub(now_ms()).div_ceil(60_000);
    if m >= 60 { format!("{} h {} min left", m / 60, m % 60) } else { format!("{m} min left") }
}

/// `hearth-link call CMD [ARGS]`, or the fixture.
fn link(cmd: &str, args: Value) -> Task<Msg> {
    let mut c = vec!["hearth-link".to_string(), "call".into(), cmd.into()];
    if !args.is_null() {
        c.push(args.to_string());
    }
    let cmd_name = cmd.to_string();
    if let Ok(f) = std::env::var("HEARTH_FAMILY_FIXTURE") {
        let text = std::fs::read_to_string(f).unwrap_or_default();
        let v: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
        let out = v.get(&cmd_name).cloned().unwrap_or(json!({}));
        return Task::done(run::Output { ok: true, code: 0, out: out.to_string(), err: String::new() }).map(move |o| if cmd_name == "moonlight.friends" { Msg::Got(o) } else { Msg::Did(String::new(), o) });
    }
    let timeout = if cmd == "moonlight.connect" { "200" } else { "40" };
    let mut full = vec!["env".to_string(), format!("HEARTH_LINK_CALL_TIMEOUT={timeout}"), "timeout".into(), "210".into()];
    full.extend(c);
    let is_list = cmd == "moonlight.friends";
    Task::perform(run::spawn(full), move |o| if is_list { Msg::Got(o) } else { Msg::Did(String::new(), o) })
}

impl App {
    fn new() -> (App, Task<Msg>) {
        let mode = if app::flag("--ask") { Mode::Ask } else if app::flag("--notice") { Mode::Notice } else { Mode::Friends };
        let layout = app::arg("--layout").or_else(|| std::env::var("HEARTH_FAMILY_LAYOUT").ok()).unwrap_or_default();
        let big = matches!(layout.as_str(), "b" | "controller" | "pad");
        let name: String = app::arg("--name").unwrap_or_else(|| "Someone".into()).chars().filter(|c| !c.is_control()).take(40).collect();
        let a = App {
            describe: hearth_ui::describe::Describe::new("family", "Family and friends")
                .with_pages(match mode {
                    // each window its own page (the question and the notice are windows of their own)
                    Mode::Friends => BLOCKS.get(0..1).unwrap_or(BLOCKS),
                    Mode::Ask => BLOCKS.get(1..2).unwrap_or(BLOCKS),
                    Mode::Notice => BLOCKS.get(2..3).unwrap_or(BLOCKS),
                })
                .with_kit(&KIT),
            p: hearth_ui::theme::load(),
            mode,
            big,
            name,
            what: app::arg("--what").unwrap_or_else(|| "desktop".into()),
            limit: app::arg("--limit").and_then(|l| l.parse().ok()).unwrap_or(0),
            until: app::arg("--until").and_then(|l| l.parse().ok()).unwrap_or(0),
            started: now_ms(),
            data: Value::Null,
            loading: mode == Mode::Friends,
            busy: String::new(),
            said: (String::new(), false),
            code: String::new(),
            fr: 0,
            fb: 0,
        };
        let t = if mode == Mode::Friends { link("moonlight.friends", json!({"check": true})) } else { Task::none() };
        (a, t)
    }

    fn s(&self, n: f32) -> f32 {
        if self.big { n * 1.35 } else { n }
    }

    fn friends(&self) -> Vec<Value> {
        self.data.get("friends").and_then(Value::as_array).cloned().unwrap_or_default()
    }

    /// Rows that can be focused on the list: each PC, then the code field.
    fn rows(&self) -> usize {
        self.friends().len() + 1 + usize::from(self.data.get("connected").map(|c| !c.is_null()).unwrap_or(false))
    }

    /// A button as the mockups draw them, bigger in the controller look, with the focus ring.
    fn b<'a>(&self, label: impl Into<String>, kind: ui::Kind, focused: bool, on: Option<Msg>) -> Element<'a, Msg> {
        let p = self.p;
        let enabled = on.is_some();
        let fg = match kind {
            ui::Kind::Primary if enabled => p.ink,
            ui::Kind::Primary => p.muted,
            ui::Kind::Danger => Color::WHITE,
            ui::Kind::Normal => if enabled { p.text } else { p.muted },
        };
        let size = self.s(14.0);
        let pad = self.s(8.0);
        let r = p.r(6.0);
        let lbl = tm(label.into(), size, fg).wrapping(iced::widget::text::Wrapping::None);
        let bt = button(container(lbl).center_y(Length::Fixed(size + 8.0)))
            .padding(Padding { top: pad, right: pad * 2.2, bottom: pad, left: pad * 2.2 })
            .on_press_maybe(on)
            .style(move |_: &Theme, st: button::Status| {
                let hover = matches!(st, button::Status::Hovered | button::Status::Pressed);
                let bg = match kind {
                    ui::Kind::Primary if enabled => if hover { Pal::mix(p.accent, Color::WHITE, 0.12) } else { p.accent },
                    ui::Kind::Primary => p.line,
                    ui::Kind::Danger => if hover { Pal::mix(p.danger, Color::WHITE, 0.12) } else { p.danger },
                    ui::Kind::Normal => if hover && enabled { p.hover() } else { Color::TRANSPARENT },
                };
                button::Style {
                    background: Some(Background::Color(bg)),
                    text_color: fg,
                    border: Border { color: if kind == ui::Kind::Normal { p.line } else { Color::TRANSPARENT }, width: if kind == ui::Kind::Normal { 1.0 } else { 0.0 }, radius: r.into() },
                    shadow: Shadow::default(),
                    snap: true,
                }
            });
        ui::ring(p, focused, r, bt.into())
    }

    /// The controller look's hints under the window.
    fn hints<'a>(&self, items: &[(&'static str, &'static str)]) -> Element<'a, Msg> {
        let p = self.p;
        if !self.big {
            return Space::new().height(Length::Fixed(0.0)).into();
        }
        let mut r = iced::widget::Row::new().spacing(22.0).align_y(iced::Center);
        for (k, what) in items {
            r = r.push(
                row![
                    container(tb(*k, self.s(11.0), p.ink)).center_x(Length::Fixed(self.s(22.0))).center_y(Length::Fixed(self.s(22.0))).style(ui::boxed(p.muted, self.s(11.0), None)),
                    t(*what, self.s(12.0), p.muted)
                ]
                .spacing(8.0)
                .align_y(iced::Center),
            );
        }
        container(r).padding(Padding { top: 10.0, right: 18.0, bottom: 12.0, left: 18.0 }).width(Length::Fill).style(ui::boxed(Pal::mix(self.p.bg, Color::BLACK, 0.25), 0.0, None)).into()
    }
}

fn update(a: &mut App, m: Msg) -> Task<Msg> {
    match m {
        Msg::Describe(dm) => {
            // one of Family and friends' buttons dragged in (drag and drop), pressed
            if let Some(m) = hearth_ui::studio::act(&dm).and_then(|id| block_action(a, id)) {
                return update(a, m);
            }
            return a.describe.update(dm).map(Msg::Describe);
        }
        Msg::EditAt(page, pick) => return hearth_ui::arrange::edit_at(&mut a.describe, page, pick).map(Msg::Describe),
        Msg::Theme => a.p = hearth_ui::theme::load(),
        Msg::Tick => {
            if a.mode == Mode::Ask && now_ms().saturating_sub(a.started) > 60_000 {
                return update(a, Msg::Answer("deny"));
            }
            if a.mode == Mode::Notice && a.until > 0 && now_ms() > a.until + 30_000 {
                return iced::exit();
            }
        }
        Msg::Answer(w) => {
            println!("{w}");
            return iced::exit();
        }
        Msg::Stop => {
            a.busy = "stop".into();
            if std::env::var("HEARTH_FAMILY_FIXTURE").is_ok() {
                return Task::none();
            }
            return Task::perform(run::spawn(run::args(&["timeout", "20", "hearth-link", "call", "moonlight.stop"])), Msg::Stopped);
        }
        Msg::Stopped(o) => {
            a.busy.clear();
            if o.ok {
                return iced::exit();
            }
            a.said = (format!("Couldn't stop it from here ({}). Turn off Let family and friends connect in Settings.", o.why()), true);
        }
        Msg::Got(o) => {
            a.loading = false;
            match o.json() {
                Some(v) if o.ok && v.get("error").is_none() => a.data = v,
                Some(v) => a.said = (gs(&v, "error"), true),
                None => a.said = (if o.why().is_empty() { "Hearth Link isn't running".into() } else { o.why() }, true),
            }
            a.fr = a.fr.min(a.rows().saturating_sub(1));
        }
        Msg::Did(said, o) => {
            a.busy.clear();
            let v = o.json().unwrap_or(Value::Null);
            if o.ok && v.get("error").is_none() {
                if !said.is_empty() {
                    a.said = (said, false);
                }
            } else {
                a.said = (if gs(&v, "error").is_empty() { o.why() } else { gs(&v, "error") }, true);
            }
            return link("moonlight.friends", json!({"check": true}));
        }
        Msg::Connect(id, name) => {
            a.busy = format!("connect:{id}");
            a.said = (format!("Asking {name}… they may need to say yes on their PC"), false);
            return link("moonlight.connect", json!({"id": id})).map(move |m| match m {
                Msg::Did(_, o) => Msg::Did(format!("Connected to {name}: Moonlight is opening."), o),
                m => m,
            });
        }
        Msg::Remove(id, name) => {
            a.busy = format!("remove:{id}");
            return link("moonlight.forget", json!({"id": id})).map(move |m| match m {
                Msg::Did(_, o) => Msg::Did(format!("{name} is off your list. They can add you again with a code."), o),
                m => m,
            });
        }
        Msg::Disconnect => {
            a.busy = "disconnect".into();
            return link("moonlight.disconnect", Value::Null).map(|m| match m {
                Msg::Did(_, o) => Msg::Did("Disconnected.".into(), o),
                m => m,
            });
        }
        Msg::Code(c) => a.code = c,
        Msg::Join => {
            let code = a.code.trim().to_string();
            if code.is_empty() {
                a.said = ("Type the code their PC shows".into(), true);
                return Task::none();
            }
            a.code.clear();
            a.busy = "join".into();
            return link("moonlight.join", json!({"code": code})).map(|m| match m {
                Msg::Did(_, o) => Msg::Did("Added. Their PC shows up here once it says you may connect.".into(), o),
                m => m,
            });
        }
        Msg::Focus(r, b) => {
            a.fr = r;
            a.fb = b;
        }
        Msg::Key(KEvent::KeyPressed { key, modifiers, .. }) => {
            // arranging: Esc keeps it and leaves, Ctrl+Z undoes
            if let Some(dm) = hearth_ui::arrange::arrange_key(&a.describe, &key, modifiers) {
                return a.describe.update(dm).map(Msg::Describe);
            }
            return key_press(a, key);
        }
        Msg::Key(_) | Msg::Nothing => {}
        Msg::Drag => return win::drag(),
        Msg::Close => {
            if a.mode == Mode::Ask {
                return update(a, Msg::Answer("deny"));
            }
            return iced::exit();
        }
    }
    Task::none()
}

/// What Enter does on the focused row and button (the list).
fn press(a: &mut App) -> Task<Msg> {
    let list = a.friends();
    let conn = a.data.get("connected").filter(|c| !c.is_null()).cloned();
    let mut r = a.fr;
    if conn.is_some() {
        if r == 0 {
            return update(a, Msg::Disconnect);
        }
        r -= 1;
    }
    if let Some(f) = list.get(r) {
        let (id, name) = (gs(f, "id"), gs(f, "name"));
        return update(a, if a.fb == 0 { Msg::Connect(id, name) } else { Msg::Remove(id, name) });
    }
    update(a, Msg::Join)
}

fn key_press(a: &mut App, key: Key) -> Task<Msg> {
    match a.mode {
        Mode::Ask => match key {
            Key::Named(Named::ArrowLeft) => a.fb = a.fb.saturating_sub(1),
            Key::Named(Named::ArrowRight) | Key::Named(Named::Tab) => a.fb = (a.fb + 1).min(2),
            Key::Named(Named::Enter) | Key::Named(Named::Space) => return update(a, Msg::Answer(["allow", "deny", "always"][a.fb.min(2)])),
            Key::Named(Named::Escape) => return update(a, Msg::Answer("deny")),
            _ => {}
        },
        Mode::Notice => {
            if matches!(key, Key::Named(Named::Enter) | Key::Named(Named::Space)) {
                return update(a, Msg::Stop);
            }
        }
        Mode::Friends => match key {
            Key::Named(Named::ArrowUp) => a.fr = a.fr.saturating_sub(1),
            Key::Named(Named::ArrowDown) => a.fr = (a.fr + 1).min(a.rows().saturating_sub(1)),
            Key::Named(Named::ArrowLeft) => a.fb = 0,
            Key::Named(Named::ArrowRight) => a.fb = 1,
            Key::Named(Named::Enter) => return press(a),
            Key::Named(Named::Escape) => return iced::exit(),
            _ => {}
        },
    }
    Task::none()
}

fn frame<'a>(a: &'a App, title: &'static str, mark: G, body: Element<'a, Msg>, hints: Element<'a, Msg>, closable: bool) -> Element<'a, Msg> {
    let p = a.p;
    let bar: Element<'a, Msg> = if closable {
        hearth_ui::describe::title_bar(&p, &a.describe, mark, title, None, TitleMsgs { drag: Msg::Drag, maximize: Msg::Nothing, minimize: None, close: Msg::Close }, Pal::mix(p.bg, Color::BLACK, 0.3), Msg::Describe(hearth_ui::describe::DMsg::Open))
    } else {
        // The notice: no close button (it's never hidden); it can still be moved.
        mouse_area(
            container(row![gap(14.0, 1.0), glyph(mark, p.accent, 16.0, 2.0), gap(10.0, 1.0), tm(title, 13.0, p.text)].align_y(iced::Center))
                .width(Length::Fill)
                .center_y(Length::Fixed(40.0))
                .style(ui::boxed(Pal::mix(p.bg, Color::BLACK, 0.3), 0.0, None)),
        )
        .on_press(Msg::Drag)
        .into()
    };
    // a right-click on the title bar: Edit layout (the window's page)
    let page = match a.mode {
        Mode::Friends => "friends",
        Mode::Ask => "ask",
        Mode::Notice => "notice",
    };
    let bar = hearth_ui::arrange::right_edit(bar, Msg::EditAt(page, None));
    container(column![bar, container(body).padding(a.s(22.0)).height(Length::Fill), hints])
        .width(Length::Fill)
        .height(Length::Fill)
        .style(ui::boxed(p.bg, p.window_corner, Some((p.line, 1.0))))
        .into()
}

/// Family and friends' blocks (hearth_ui::blocks).
/// The list's people keep their ids from 1.18; the title and its line are a block of their own now.
use hearth_ui::blocks::{bl, bls, PageDef};
const BLOCKS: &[PageDef] = &[
    PageDef { id: "friends", title: "Family and friends", blocks: &[bls("people", "Their PCs", 0, 1, 12, 8), bls("add", "Add with a code", 0, 9, 12, 3), bl("head", "Title and its line", 0, 0, 12, 1)] },
    PageDef { id: "ask", title: "Someone wants to connect", blocks: &[bl("who", "Who wants to connect", 0, 0, 12, 3), bl("what", "What they could do", 0, 3, 12, 4), bl("buttons", "Allow, Deny, Always allow", 0, 7, 12, 3), bl("time", "Time to answer", 0, 10, 12, 2)] },
    PageDef { id: "notice", title: "Sharing this PC", blocks: &[bl("who", "Who is connected", 0, 0, 12, 4), bl("what", "What they can do", 0, 4, 12, 4), bl("stop", "Stop sharing now", 0, 8, 12, 4)] },
];

// ------------------------------------------------------------------ drag and drop (board 3: the same in every app)
use hearth_ui::studio::{tile, Kit};
/// Family and friends' blocks window: its buttons, its panes, its info.
const KIT: Kit = Kit {
    name: "Family and friends",
    buttons: &[
        tile("add", "Add with the code", G::Plus),
        tile("connect-first", "Connect to the first one online", G::Display),
        tile("disconnect", "Disconnect", G::Close),
        tile("stop", "Stop sharing now", G::Stop),
        tile("allow", "Allow", G::Check),
        tile("deny", "Deny", G::Close),
        tile("always", "Always allow", G::Check),
        tile("close", "Close the window", G::Close),
    ],
    panes: &[tile("online", "Who's online", G::Users), tile("connected", "The connection now", G::Link), tile("how", "How it works", G::Help)],
    info: &[tile("people", "People", G::Users), tile("online", "Online now", G::Users), tile("connected", "Connected to", G::Link)],
    pane_page: "friends",
    bars: &[],
    menus: &[],
};

/// One of Family and friends' buttons dragged in, pressed.
fn block_action(a: &App, id: &str) -> Option<Msg> {
    Some(match id {
        "add" => Msg::Join,
        "connect-first" => {
            let f = a.friends().into_iter().find(|f| gb(f, "online"))?;
            Msg::Connect(gs(&f, "id"), gs(&f, "name"))
        }
        "disconnect" => Msg::Disconnect,
        "stop" => Msg::Stop,
        "allow" => Msg::Answer("allow"),
        "deny" => Msg::Answer("deny"),
        "always" => Msg::Answer("always"),
        "close" => Msg::Close,
        _ => return None,
    })
}

fn studio_values(a: &App) {
    let list = a.friends();
    let conn = a.data.get("connected").filter(|c| !c.is_null()).map(|c| gs(c, "name")).unwrap_or_default();
    a.describe.values(vec![("people", list.len().to_string()), ("online", list.iter().filter(|f| gb(f, "online")).count().to_string()), ("connected", conn)]);
}

/// A pane of Family and friends' own: who's online, the connection now, how it works.
fn studio_pane<'a>(a: &'a App, which: &str) -> Element<'a, Msg> {
    let p = a.p;
    let mut c = Column::new().spacing(6.0);
    match which {
        "online" => {
            let list: Vec<Value> = a.friends().into_iter().filter(|f| gb(f, "online")).collect();
            if list.is_empty() {
                c = c.push(t("Nobody's online right now.", 13.0, p.muted));
            }
            for f in list {
                c = c.push(mouse_area(t(gs(&f, "name"), 13.0, p.text)).on_press(Msg::Connect(gs(&f, "id"), gs(&f, "name"))).interaction(mouse::Interaction::Pointer));
            }
        }
        "connected" => match a.data.get("connected").filter(|x| !x.is_null()) {
            Some(conn) => {
                c = c.push(tm(format!("Connected to {}", gs(conn, "name")), 14.0, p.text));
                c = c.push(a.b("Disconnect", ui::Kind::Danger, false, Some(Msg::Disconnect)));
            }
            None => c = c.push(t("Not connected to anyone.", 13.0, p.muted)),
        },
        _ => {
            c = c.push(t("Moonlight shows their PC on yours; they may be asked to say yes first. The picture goes through Hearth Link's own connection, encrypted end to end, and no router ports are opened.", 13.0, p.muted));
        }
    }
    c.into()
}

/// A page's parts, each a right-click away from Edit layout (that part picked).
fn parts_of<'a>(a: &'a App, page: &'static str, items: Vec<(&'static str, Element<'a, Msg>)>, original: impl FnOnce(&mut hearth_ui::arrange::Parts<'a, Msg>) -> Element<'a, Msg>) -> Element<'a, Msg> {
    let items = items.into_iter().map(|(id, el)| (id, hearth_ui::arrange::right_edit(el, Msg::EditAt(page, Some(id))))).collect();
    hearth_ui::arrange::parts(&a.describe, &a.p, page, items, original, Msg::Describe)
}

fn view(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    match a.mode {
        Mode::Ask => {
            let secs = 60u64.saturating_sub(now_ms().saturating_sub(a.started) / 1000);
            let sees = if a.what == "games" { "your games (Steam's Big Picture) on this screen" } else { "this PC's whole screen" };
            let who: Element<Msg> = row![
                container(glyph(G::Users, p.ink, a.s(22.0), 2.0)).center_x(Length::Fixed(a.s(46.0))).center_y(Length::Fixed(a.s(46.0))).style(ui::boxed(p.accent, a.s(23.0), None)),
                column![tb(format!("{} wants to connect", a.name), a.s(20.0), p.text), t("Family and friends · Moonlight", a.s(13.0), p.muted)].spacing(2.0)
            ]
            .spacing(14.0)
            .align_y(iced::Center)
            .into();
            let what: Element<Msg> = t(format!("If you allow it, {} can see {sees}, control it with their mouse, keyboard or controller, and hear its sound{}.", a.name, if a.limit > 0 { format!(", {}", limit_word(a.limit)) } else { String::new() }), a.s(14.0), p.text).into();
            let buttons: Element<Msg> = row![
                a.b("Allow", ui::Kind::Primary, a.big && a.fb == 0, Some(Msg::Answer("allow"))),
                a.b("Deny", ui::Kind::Normal, a.big && a.fb == 1, Some(Msg::Answer("deny"))),
                Space::new().width(Length::Fill),
                a.b(format!("Always allow {}", a.name), ui::Kind::Normal, a.big && a.fb == 2, Some(Msg::Answer("always"))),
            ].spacing(10.0).align_y(iced::Center).into();
            let time: Element<Msg> = t(format!("Nobody answering in {secs} s is a no. You can change who may connect in Settings › Gaming › Family and friends."), a.s(12.0), p.muted).into();
            // its blocks (BLOCKS "ask"); as it was when nothing is
            let s14 = a.s(14.0);
            let body = parts_of(a, "ask", vec![("who", who), ("what", what), ("buttons", buttons), ("time", time)], move |ps| {
                column![ps.take("who"), ps.take("what"), ps.take("buttons"), ps.take("time")].spacing(s14).into()
            });
            frame(a, "Family and friends", G::Users, body, a.hints(&[("A", "Choose"), ("B", "Deny"), ("← →", "Move")]), true)
        }
        Mode::Notice => {
            let dot = container(Space::new()).width(Length::Fixed(a.s(10.0))).height(Length::Fixed(a.s(10.0))).style(ui::boxed(p.good, a.s(5.0), None));
            let s12 = a.s(12.0);
            let who: Element<Msg> = row![dot, tb(format!("{} is connected", a.name), a.s(18.0), p.text)].spacing(10.0).align_y(iced::Center).into();
            let what: Element<Msg> = t(format!("They can see and control this PC and hear its sound with Moonlight. {} · {}", what_word(&a.what), left(a.until)), a.s(13.0), p.muted).into();
            let mut stop = column![a.b(if a.busy == "stop" { "Stopping…" } else { "Stop sharing now" }, ui::Kind::Danger, a.big, if a.busy.is_empty() { Some(Msg::Stop) } else { None })].spacing(s12);
            if !a.said.0.is_empty() {
                stop = stop.push(t(a.said.0.clone(), a.s(12.0), p.danger));
            }
            // its blocks (BLOCKS "notice"); as it was when nothing is
            let body = parts_of(a, "notice", vec![("who", who), ("what", what), ("stop", stop.into())], move |ps| column![ps.take("who"), ps.take("what"), ps.take("stop")].spacing(s12).into());
            frame(a, "Sharing this PC (Moonlight)", G::Display, body, a.hints(&[("A", "Stop sharing now")]), false)
        }
        Mode::Friends => {
            let head: Element<Msg> = column![tb("Family and friends", a.s(22.0), p.text), t("Their PCs that let you connect with Moonlight. They may be asked to say yes first.", a.s(13.0), p.muted)].spacing(a.s(10.0)).into();
            let mut c = Column::new().spacing(a.s(10.0));
            let mut ri = 0usize;
            if let Some(conn) = a.data.get("connected").filter(|c| !c.is_null()) {
                let focused = a.big && a.fr == ri;
                c = c.push(card(a, G::Display, p.good, format!("Connected to {}", gs(conn, "name")), format!("Moonlight is open{}", if gb(conn, "relay") { " · through Hearth Link's relay" } else { "" }),
                    vec![a.b("Disconnect", ui::Kind::Danger, focused, Some(Msg::Disconnect))], focused));
                ri += 1;
            }
            if a.loading {
                c = c.push(t("Looking for them…", a.s(13.0), p.muted));
            }
            let list = a.friends();
            if !a.loading && list.is_empty() {
                c = c.push(t("Nobody yet. When someone adds you on their PC (Settings › Gaming › Family and friends › Add a person), type the code they give you below.", a.s(14.0), p.muted));
            }
            let connected = a.data.get("connected").map(|c| !c.is_null()).unwrap_or(false);
            for f in &list {
                let (id, name) = (gs(f, "id"), gs(f, "name"));
                let online = gb(f, "online");
                let sharing = f.get("sharing").and_then(Value::as_bool);
                let sub = if !online {
                    "Offline".to_string()
                } else if sharing == Some(false) {
                    "Online · not letting anyone connect right now".to_string()
                } else {
                    format!("Online · {}{}", what_word(&gs(f, "what")), if f.get("ask").and_then(Value::as_bool) == Some(false) { "" } else { " · they're asked first" })
                };
                let can = online && sharing != Some(false) && !connected && a.busy.is_empty();
                let focused = a.big && a.fr == ri;
                let busy_here = a.busy == format!("connect:{id}");
                c = c.push(card(a, G::Display, if online { p.good } else { p.muted }, name.clone(), sub, vec![
                    a.b(if busy_here { "Connecting…" } else { "Connect" }, ui::Kind::Primary, focused && a.fb == 0, if can { Some(Msg::Connect(id.clone(), name.clone())) } else { None }),
                    a.b("Remove", ui::Kind::Normal, focused && a.fb == 1, Some(Msg::Remove(id.clone(), name.clone()))),
                ], focused));
                ri += 1;
            }
            if !gb(&a.data, "moonlight") && !a.data.is_null() {
                c = c.push(t("Moonlight isn't installed yet: Settings › Gaming › Remote play › Moonlight › Install.", a.s(13.0), p.warn));
            }
            let focused = a.big && a.fr == ri;
            let mut d = Column::new().spacing(a.s(10.0));
            let input = text_input("Add with a code: the code their PC shows (like 482 913 or K7M-4QX-92HC)", &a.code)
                .on_input(Msg::Code)
                .on_submit(Msg::Join)
                .size(a.s(14.0))
                .padding(a.s(10.0));
            d = d.push(ui::ring(p, focused, 8.0, row![container(input).width(Length::Fill), a.b("Add", ui::Kind::Normal, false, Some(Msg::Join))].spacing(10.0).align_y(iced::Center).into()));
            if !a.said.0.is_empty() {
                d = d.push(t(a.said.0.clone(), a.s(13.0), if a.said.1 { p.danger } else { p.flame }));
            }
            d = d.push(t("The picture goes through Hearth Link's own connection, encrypted end to end. No router ports are opened.", a.s(12.0), p.muted));
            // its blocks (hearth_ui::blocks): the people and adding one, moved, sized, hidden or restyled
            let s10 = a.s(10.0);
            let body = parts_of(
                a,
                "friends",
                {
                    studio_values(a);
                    let mut items: Vec<(&'static str, Element<'_, Msg>)> = vec![("head", head), ("people", c.into()), ("add", d.into())];
                    for (id, which) in [("pane:online", "online"), ("pane:connected", "connected"), ("pane:how", "how")] {
                        if a.describe.wants_pane(which) {
                            items.push((id, studio_pane(a, which)));
                        }
                    }
                    items
                },
                // as it was: one list that scrolls, the title, the people then adding one
                move |ps| scrollable(column![ps.take("head"), ps.take("people"), ps.take("add")].spacing(s10)).height(Length::Fill).into(),
            );
            frame(a, "Family and friends", G::Users, body, a.hints(&[("A", "Connect"), ("← →", "Connect / Remove"), ("↑ ↓", "Move"), ("B", "Close")]), true)
        }
    }
}

/// One PC on the list: its dot, name, line, and its buttons.
fn card<'a>(a: &'a App, g: G, dot: Color, title: String, sub: String, buttons: Vec<Element<'a, Msg>>, focused: bool) -> Element<'a, Msg> {
    let p = a.p;
    let mut r = iced::widget::Row::new().spacing(12.0).align_y(iced::Center);
    r = r.push(container(glyph(g, p.muted, a.s(20.0), 1.8)).center_x(Length::Fixed(a.s(40.0))).center_y(Length::Fixed(a.s(40.0))).style(ui::boxed(p.panel2, p.r(8.0), None)));
    r = r.push(
        column![
            row![tm(title, a.s(15.0), p.text), container(Space::new()).width(Length::Fixed(a.s(8.0))).height(Length::Fixed(a.s(8.0))).style(ui::boxed(dot, a.s(4.0), None))].spacing(8.0).align_y(iced::Center),
            t(sub, a.s(13.0), p.muted)
        ]
        .spacing(2.0)
        .width(Length::Fill),
    );
    for b in buttons {
        r = r.push(b);
    }
    let inner = container(r).padding(Padding { top: a.s(12.0), right: 14.0, bottom: a.s(12.0), left: 14.0 }).width(Length::Fill).style(ui::boxed(p.panel, p.r(10.0), Some((if focused { p.accent } else { p.line }, if focused { 2.0 } else { 1.0 }))));
    mouse_area(inner).interaction(mouse::Interaction::Idle).into()
}

fn subscription(a: &App) -> Subscription<Msg> {
    let tick = match a.mode {
        // The question counts down; the notice's time left changes every minute.
        Mode::Ask => iced::time::every(std::time::Duration::from_secs(1)).map(|_| Msg::Tick),
        Mode::Notice => iced::time::every(std::time::Duration::from_secs(30)).map(|_| Msg::Tick),
        Mode::Friends => Subscription::none(),
    };
    Subscription::batch([Subscription::run(hearth_ui::theme::watch).map(|_| Msg::Theme), iced::keyboard::listen().map(Msg::Key), tick])
}

/// The app with Describe it's box under it when it's open (every Hearth app has it).
fn shown(a: &App) -> Element<'_, Msg> {
    hearth_ui::describe::with_bar(view(a), &a.p, &a.describe, Msg::Describe)
}

fn main() -> iced::Result {
    if app::handle_cli(
        "hearth-family-app",
        "Family and friends with Moonlight.\n  --page friends (default)       the PCs that let this one connect\n  --ask --name N --what W --limit MIN   \"N wants to connect\": prints allow, deny or always\n  --notice --name N --what W --until MS   while N is connected, with Stop sharing now\n  --layout b                     the controller look",
    ) {
        return Ok(());
    }
    let mode = if app::flag("--ask") { Mode::Ask } else if app::flag("--notice") { Mode::Notice } else { Mode::Friends };
    let big = matches!(app::arg("--layout").or_else(|| std::env::var("HEARTH_FAMILY_LAYOUT").ok()).unwrap_or_default().as_str(), "b" | "controller" | "pad");
    let k = if big { 1.35 } else { 1.0 };
    let (size, min) = match mode {
        Mode::Ask => ((600.0 * k, 290.0 * k), (480.0, 260.0)),
        Mode::Notice => ((460.0 * k, 200.0 * k), (380.0, 170.0)),
        Mode::Friends => ((820.0 * k, 600.0 * k), (560.0, 420.0)),
    };
    let title = match mode {
        Mode::Ask => format!("{} wants to connect", app::arg("--name").unwrap_or_else(|| "Someone".into())),
        Mode::Notice => format!("{} is connected (Moonlight)", app::arg("--name").unwrap_or_else(|| "Someone".into())),
        Mode::Friends => "Family and friends".to_string(),
    };
    let mut w = app::window(match mode { Mode::Notice => "hearth-family-notice", Mode::Ask => "hearth-family-ask", Mode::Friends => "hearth-family" }, size, min);
    if mode != Mode::Friends {
        // The question and the notice stay above other windows, so they're never hidden.
        w.level = iced::window::Level::AlwaysOnTop;
        w.resizable = false;
    }
    iced::application(App::new, update, shown)
        .title(move |_: &App| title.clone())
        .subscription(subscription)
        .theme(|a: &App| app::iced_theme(&a.p))
        .window(w)
        .default_font(app::INTER)
        .run()
}
