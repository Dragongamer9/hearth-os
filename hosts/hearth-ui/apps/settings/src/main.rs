//! Hearth Settings as its own app, in pure Rust (hearth-ui, iced): not part of Mantel's shell and not the Game Mode
//! app, so it opens the same on KDE Plasma, Firehold or any desktop. Two looks of the same pages, as the approved new
//! Settings (Nov 15, 2026): layout A while you use a keyboard and mouse (settings-A1-home-mouse.png,
//! settings-A2-display-mouse.png) and layout B, the controller look, by itself while a controller is used
//! (settings-B1-quick-controller.png, settings-B2-display-controller.png), back to A when the mouse or keyboard is
//! used again; the page and the setting you were on stay picked across the switch. Every page of Mantel's Settings is
//! in the registry (pages_list.rs); each category is its own module (cats/, all of the same shape: cats/mod.rs, and
//! quick.rs for the Quick tab), and the frame (frame.rs) draws the rest. What's built: docs/mockups/checklists/settings-rust.md.
//!
//! The controller (nav.rs, pad.rs): D-pad / left stick or the arrow keys move between rows and controls, A (Enter)
//! selects, B (Esc) goes back, Y (Ctrl+F) searches, LB / RB (Page Up / Down) the previous / next category or tab, X
//! (Ctrl+D) the page's own action or Default, ☰ (Ctrl+W) closes. The buttons come as keys (hearth-pad, Steam Input)
//! and are also read from the controller itself, so they work on Plasma too, and nothing happens twice.
//!
//! Hearth's own settings are written only through hearth_settings_io.py (atomic, .bak, merged, the change history
//! with Undo: red flag 9); the system's through its own tools. Opening it writes nothing.
//!
//! `hearth-settings-app [PAGE[:part]]` or `--page PAGE` opens at a page (Mantel's page ids); `--layout a|b` keeps one
//! look (pictures, tests; also HEARTH_SETTINGS_LAYOUT); `--focus TITLE` puts the focus on a row; `--find WORDS` searches.

mod cats;
mod data;
mod frame;
mod nav;
mod pad;
mod pages_list;
mod pics;
mod quick;
mod registry;
mod shared;
mod widgets;

use cats::{a11y, accounts, apps, bt, g15, gaming, home, hs, link, net, pers, privacy, system, updates};
use hearth_ui::app::{self, win};
use hearth_ui::run;
use iced::keyboard::{key::Named, Event as KEvent, Key, Modifiers};
use iced::{Element, Subscription, Task};
use shared::Shared;
use std::time::Instant;

/// Every category module, once: (its field in App, its Msg variant, the module). Adding a category is one line here.
macro_rules! each_cat {
    ($mac:ident) => {
        $mac! {
            (home, Home, home),
            (quick, Quick, quick),
            (system, System, system),
            (net, Net, net),
            (bt, Bt, bt),
            (g15, G15, g15),
            (link, Link, link),
            (pers, Pers, pers),
            (apps, Apps, apps),
            (gaming, Gaming, gaming),
            (accounts, Accounts, accounts),
            (hs, Hs, hs),
            (privacy, Privacy, privacy),
            (a11y, A11y, a11y),
            (updates, Updates, updates)
        }
    };
}

/// What a key or a controller button does in Settings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Action {
    Up,
    Down,
    Left,
    Right,
    Select,
    Back,
    Search,
    X,
    Prev,
    Next,
    Close,
}

macro_rules! define {
    ($(($f:ident, $v:ident, $m:ident)),*) => {
        pub struct App {
            pub sh: Shared,
            pub cat: usize,
            pub page: &'static str,
            pub query: String,
            /// The look was picked with --layout (Some(true): B), so it doesn't switch by itself.
            pub fixed: Option<bool>,
            /// The window has the keyboard (the controller's presses are for it).
            pub win_focused: bool,
            /// The controller look's search (Y) is open.
            pub finding: bool,
            /// X Default asked first: the setting's name and what it goes back to; what puts it back.
            pub confirm: Option<(String, String)>,
            pub confirm_acts: Vec<Msg>,
            /// The last press read from the controller and the last key, to do each press once.
            pub last_pad: Option<(Action, Instant)>,
            pub last_key: Option<(Action, Instant)>,
            $(pub $f: $m::State,)*
        }

        #[derive(Debug, Clone)]
        pub enum Msg {
            Theme,
            Info(run::Output),
            History(run::Output),
            Cat(usize),
            Go(&'static str),
            Query(String),
            Key(KEvent),
            Pad(pad::Button),
            Focus(bool),
            /// The mouse was used while the controller look shows.
            Pointer,
            Tab(usize),
            X,
            Find(bool),
            Confirm(bool),
            /// `--press`: controller presses, one after another, once the page has had a moment to read.
            Press(Vec<pad::Button>),
            Drag,
            Maximize,
            Minimize,
            Close,
            Resize(iced::window::Direction),
            $($v($m::Msg),)*
        }

        fn new_app(first: &'static str, part: String) -> App {
            let mut sh = Shared::new();
            sh.part = part;
            App {
                sh,
                cat: registry::cat_of(first),
                page: first,
                query: String::new(),
                fixed: None,
                win_focused: true,
                finding: false,
                confirm: None,
                confirm_acts: Vec::new(),
                last_pad: None,
                last_key: None,
                $($f: $m::State::default(),)*
            }
        }

        /// The page's own module draws it (None: no module has it, the frame lists the category).
        pub fn page_view(a: &App) -> Option<Element<'_, Msg>> {
            $(if $m::PAGES.contains(&a.page) {
                return $m::view(a, a.page).map(|e| e.map(Msg::$v));
            })*
            None
        }

        pub fn dialog_view(a: &App) -> Option<(Element<'_, Msg>, Option<Msg>)> {
            $(if let Some((d, o)) = $m::dialog(a) {
                return Some((d.map(Msg::$v), o.map(Msg::$v)));
            })*
            None
        }

        fn open_page(a: &mut App, id: &'static str) -> Task<Msg> {
            $(if $m::PAGES.contains(&id) {
                return $m::open(&mut a.sh, &mut a.$f, id).map(Msg::$v);
            })*
            Task::none()
        }

        fn update_cat(a: &mut App, m: Msg) -> Task<Msg> {
            match m {
                $(Msg::$v(x) => $m::update(&mut a.sh, &mut a.$f, x).map(Msg::$v),)*
                _ => Task::none(),
            }
        }

        fn back_cat(a: &mut App) -> bool {
            $(if $m::back(&mut a.$f) { return true; })*
            false
        }

        fn cat_subs(a: &App) -> Vec<Subscription<Msg>> {
            vec![$($m::subscription(&a.$f).map(Msg::$v),)*]
        }

        /// What a row registered for A / left / right (a module's message, or the frame's), as the app's message.
        fn lift(b: nav::Act) -> Option<Msg> {
            let b = match b.downcast::<Msg>() {
                Ok(m) => return Some(*m),
                Err(b) => b,
            };
            $(let b = match b.downcast::<$m::Msg>() {
                Ok(m) => return Some(Msg::$v(*m)),
                Err(b) => b,
            };)*
            drop(b);
            None
        }
    };
}

each_cat!(define);

fn info() -> Task<Msg> {
    Task::perform(run::spawn(data::sh(data::INFO)), Msg::Info)
}
fn history() -> Task<Msg> {
    Task::perform(run::spawn(vec!["python3".into(), shared::io_helper(), "history".into(), "20".into()]), Msg::History)
}

/// The page asked for on the command line: `--page X`, or the first word that isn't a flag or a flag's value.
fn page_arg() -> Option<String> {
    if let Some(p) = app::arg("--page") {
        return Some(p);
    }
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        if ["--layout", "--focus", "--find", "--press"].contains(&a.as_str()) {
            it.next();
            continue;
        }
        if !a.starts_with('-') {
            return Some(a);
        }
    }
    None
}

impl App {
    fn new() -> (App, Task<Msg>) {
        let (first, part) = page_arg().as_deref().and_then(registry::resolve).unwrap_or(("home", String::new()));
        let mut a = new_app(first, part);
        let layout = app::arg("--layout").or_else(|| std::env::var("HEARTH_SETTINGS_LAYOUT").ok()).unwrap_or_default();
        a.fixed = match layout.to_lowercase().as_str() {
            "b" | "controller" | "pad" => Some(true),
            "a" | "mouse" => Some(false),
            _ => None,
        };
        nav::set_fixed(a.fixed.is_some());
        let b = a.fixed.unwrap_or(false);
        set_layout(&mut a, b);
        let t = open_page(&mut a, first);
        if let Some(f) = app::arg("--focus") {
            nav::want(&f);
            nav::set_shown(true);
        }
        if let Some(q) = app::arg("--find") {
            a.query = q;
            a.finding = a.sh.big;
            nav::set_in_page(true);
        }
        let mut tasks = vec![info(), history(), t];
        // `--press "right down a"`: the controller's presses, for the pictures and tests (as if read from it).
        if let Some(p) = app::arg("--press") {
            let presses: Vec<pad::Button> = p.split_whitespace().filter_map(pad::named).collect();
            tasks.push(Task::perform(run::after(900), move |_| Msg::Press(presses.clone())));
        }
        (a, Task::batch(tasks))
    }

    fn open(&mut self, id: &'static str) -> Task<Msg> {
        self.page = id;
        self.cat = registry::cat_of(id);
        self.query.clear();
        self.finding = false;
        self.sh.error.clear();
        self.sh.note.clear();
        nav::reset();
        open_page(self, id)
    }
}

/// Shows the controller look (B) or the keyboard-and-mouse look (A). The page and the focused row stay.
fn set_layout(a: &mut App, b: bool) {
    a.sh.big = b;
    nav::set_pad_look(b);
    if b {
        nav::set_shown(true);
        if a.page == "quick" {
            nav::set_in_page(true);
        }
    } else {
        nav::set_shown(false);
        a.finding = false;
    }
}

fn update(a: &mut App, m: Msg) -> Task<Msg> {
    let t = match m {
        Msg::Theme => {
            a.sh.p = hearth_ui::theme::load();
            a.sh.desktop = shared::read_desktop();
            Task::none()
        }
        Msg::Info(o) => {
            a.sh.info = data::info(&o.out);
            Task::none()
        }
        Msg::History(o) => {
            a.sh.history = o.json().map(|v| data::changes(&v)).unwrap_or_default();
            Task::none()
        }
        Msg::Cat(i) => {
            if i < registry::CATEGORIES.len() {
                a.open(registry::CATEGORIES[i].2)
            } else {
                Task::none()
            }
        }
        Msg::Go(id) => a.open(id),
        Msg::Query(q) => {
            a.query = q;
            nav::reset();
            Task::none()
        }
        Msg::Key(KEvent::KeyPressed { key, modifiers, .. }) => key_press(a, key, modifiers),
        Msg::Key(_) => Task::none(),
        Msg::Pad(b) => pad_press(a, b),
        Msg::Focus(on) => {
            a.win_focused = on;
            Task::none()
        }
        Msg::Pointer => {
            if a.sh.big && a.fixed.is_none() {
                set_layout(a, false);
            }
            Task::none()
        }
        Msg::Tab(i) => open_tab(a, i),
        Msg::X => x_action(a),
        Msg::Find(on) => {
            a.finding = on;
            if !on {
                a.query.clear();
            }
            Task::none()
        }
        Msg::Confirm(yes) => {
            a.confirm = None;
            let acts = std::mem::take(&mut a.confirm_acts);
            if yes {
                Task::batch(acts.into_iter().map(|m| update(a, m)).collect::<Vec<_>>())
            } else {
                Task::none()
            }
        }
        Msg::Press(mut v) => {
            if v.is_empty() {
                return Task::none();
            }
            // One press at a time, the page drawn again between them (as a person presses).
            let b = v.remove(0);
            a.last_key = None;
            let t = pad_press(a, b);
            Task::batch([t, Task::perform(run::after(250), move |_| Msg::Press(v.clone()))])
        }
        Msg::Drag => win::drag(),
        Msg::Maximize => win::toggle_maximize(),
        Msg::Minimize => win::minimize(),
        Msg::Close => win::close(),
        Msg::Resize(d) => win::resize(d),
        other => update_cat(a, other),
    };
    // What a module asked for: another page, the change history read again.
    let mut more = Vec::new();
    if let Some(id) = a.sh.go.take() {
        more.push(a.open(id));
    }
    if std::mem::take(&mut a.sh.reread_history) {
        more.push(history());
    }
    if more.is_empty() {
        t
    } else {
        more.insert(0, t);
        Task::batch(more)
    }
}

/// The tab's first page (the controller look's LB / RB and a click on a tab).
fn open_tab(a: &mut App, i: usize) -> Task<Msg> {
    let i = i % registry::TABS.len();
    let id = if i == 0 { "quick" } else { frame::tab_list(i).first().map(|p| p.id).unwrap_or("home") };
    // Quick has no list on the left: the focus goes to its tiles (as Mantel's stepTab).
    nav::set_in_page(i == 0);
    a.open(id)
}

/// B / Esc: a dialog or the search closes, then up one level (a subpage to its page, a page to the list on the left).
fn back(a: &mut App) -> Task<Msg> {
    if a.confirm.take().is_some() {
        a.confirm_acts.clear();
        return Task::none();
    }
    if a.finding {
        a.finding = false;
        a.query.clear();
        return Task::none();
    }
    if back_cat(a) {
        return Task::none();
    }
    if !a.query.is_empty() {
        a.query.clear();
        return Task::none();
    }
    let Some(p) = registry::page(a.page) else { return Task::none() };
    if !p.parent.is_empty() {
        if let Some(up) = registry::page(p.parent) {
            // The row that opened this page gets the focus back.
            nav::want(p.label);
            return a.open(up.id);
        }
    }
    if a.sh.big {
        if nav::in_page() && a.page != "quick" {
            nav::set_in_page(false);
            return Task::none();
        }
        if !nav::in_page() {
            // B on the list on the left closes Settings, as Mantel's controller look.
            return win::close();
        }
        return Task::none();
    }
    let landing = registry::CATEGORIES[registry::cat_of(p.id)].2;
    if a.page != landing {
        nav::want(p.label);
        return a.open(landing);
    }
    nav::set_in_page(false);
    Task::none()
}

fn run_acts(a: &mut App, acts: Vec<nav::Act>) -> Task<Msg> {
    let msgs: Vec<Msg> = acts.into_iter().filter_map(lift).collect();
    Task::batch(msgs.into_iter().map(|m| update(a, m)).collect::<Vec<_>>())
}

/// Into the page from the list on the left: the focus on its first row.
fn enter_page(a: &mut App) -> Task<Msg> {
    let _ = a;
    nav::set_in_page(true);
    nav::reset();
    nav::reveal()
}

/// Up / Down on the list on the left: the page above or below opens (the categories in A, the tab's pages in B).
fn list_step(a: &mut App, d: i32) -> Task<Msg> {
    if a.sh.big {
        let tab = registry::tab_of(a.page);
        let list = frame::tab_list(tab);
        let top = registry::top_of(a.page);
        let i = list.iter().position(|p| p.id == top || p.id == a.page).unwrap_or(0) as i32;
        let to = (i + d).clamp(0, list.len() as i32 - 1);
        match list.get(to as usize) {
            Some(p) if to != i => a.open(p.id),
            _ => Task::none(),
        }
    } else {
        let to = (a.cat as i32 + d).clamp(0, registry::CATEGORIES.len() as i32 - 1) as usize;
        if to != a.cat {
            update(a, Msg::Cat(to))
        } else {
            Task::none()
        }
    }
}

/// X: the page's own action (Keep these settings, Pin to Quick), or Default for the focused setting (the controller
/// look), asked first: only that one setting goes back, through its own row (the same tool that changes it).
fn x_action(a: &mut App) -> Task<Msg> {
    if a.system.keep.is_some() {
        return update(a, Msg::System(system::Msg::Keep));
    }
    if a.page == "quick" {
        nav::reset();
        return update(a, Msg::Quick(quick::Msg::Picking));
    }
    if !a.sh.big {
        return Task::none();
    }
    let f = if nav::in_page() { nav::focused() } else { None };
    let Some(f) = f else {
        a.sh.note = "Pick a setting first".into();
        return Task::none();
    };
    match (f.kind, frame::default_of(&f.title)) {
        (nav::Kind::Toggle(on), Some(d)) if on == d => a.sh.note = format!("{} is already at its default ({})", f.title, if d { "On" } else { "Off" }),
        (nav::Kind::Toggle(_), Some(d)) => {
            let (acts, _) = nav::take(nav::Which::A);
            a.confirm_acts = acts.into_iter().filter_map(lift).collect();
            if !a.confirm_acts.is_empty() {
                a.confirm = Some((f.title.clone(), format!("It goes back to {}. Only this one setting changes, and Undo on Home puts it back.", if d { "On" } else { "Off" })));
            }
        }
        _ => a.sh.note = format!("{} has no default to go back to", if f.title.is_empty() { "This" } else { f.title.as_str() }),
    }
    Task::none()
}

fn do_action(a: &mut App, act: Action) -> Task<Msg> {
    use Action::*;
    if a.confirm.is_some() {
        return match act {
            Select => update(a, Msg::Confirm(true)),
            Back => update(a, Msg::Confirm(false)),
            _ => Task::none(),
        };
    }
    match act {
        Close => return win::close(),
        Back => return back(a),
        X => return x_action(a),
        Prev | Next => {
            let d: i32 = if act == Next { 1 } else { -1 };
            return if a.sh.big {
                let n = registry::TABS.len() as i32;
                open_tab(a, ((registry::tab_of(a.page) as i32 + d + n) % n) as usize)
            } else {
                let n = registry::CATEGORIES.len() as i32;
                nav::set_in_page(false);
                update(a, Msg::Cat(((a.cat as i32 + d + n) % n) as usize))
            };
        }
        Search => {
            nav::set_shown(true);
            nav::set_in_page(true);
            nav::reset();
            if a.sh.big {
                a.finding = true;
                a.query.clear();
            }
            return iced::widget::operation::focus(iced::widget::Id::new(nav::SEARCH));
        }
        _ => {}
    }
    // The first arrow or A in the keyboard-and-mouse look shows where the focus is.
    if !nav::shown() {
        nav::set_shown(true);
        if !nav::in_page() && matches!(act, Right | Select) {
            return enter_page(a);
        }
        return nav::reveal();
    }
    let has_list = !(a.sh.big && a.page == "quick") && !a.finding;
    if !nav::in_page() {
        return match act {
            Up => list_step(a, -1),
            Down => list_step(a, 1),
            Right | Select => enter_page(a),
            _ => Task::none(),
        };
    }
    match act {
        Up | Down => {
            if nav::step(if act == Down { 1 } else { -1 }) {
                nav::reveal()
            } else {
                Task::none()
            }
        }
        Left | Right => {
            let (acts, _) = nav::take(if act == Left { nav::Which::Left } else { nav::Which::Right });
            if !acts.is_empty() {
                return run_acts(a, acts);
            }
            let grid = nav::focused().map(|f| f.grid).unwrap_or(false);
            if act == Right {
                if grid && nav::step(1) {
                    return nav::reveal();
                }
                return Task::none();
            }
            if grid && !nav::at_first() {
                nav::step(-1);
                return nav::reveal();
            }
            if has_list {
                nav::set_in_page(false);
            }
            Task::none()
        }
        Select => {
            let (acts, field) = nav::take(nav::Which::A);
            if let Some(id) = field {
                return iced::widget::operation::focus(id);
            }
            run_acts(a, acts)
        }
        _ => Task::none(),
    }
}

/// How long after a press read from the controller the same thing typed as a key (hearth-pad, Steam) is the same
/// press, and the other way round.
const SAME_PRESS_MS: u128 = 350;

fn same(last: Option<(Action, Instant)>, act: Action) -> bool {
    matches!(last, Some((l, at)) if l == act && at.elapsed().as_millis() < SAME_PRESS_MS)
}

fn pad_press(a: &mut App, b: pad::Button) -> Task<Msg> {
    use pad::Button as P;
    if !a.win_focused {
        return Task::none(); // another window has the controller
    }
    nav::pad_used();
    if !a.sh.big && a.fixed.is_none() {
        set_layout(a, true);
    }
    let act = match b {
        P::A => Action::Select,
        P::B => Action::Back,
        P::X => Action::X,
        P::Y => Action::Search,
        P::Lb => Action::Prev,
        P::Rb => Action::Next,
        P::Up => Action::Up,
        P::Down => Action::Down,
        P::Left => Action::Left,
        P::Right => Action::Right,
        P::Menu => Action::Close,
        P::View => return Task::none(),
    };
    if same(a.last_key, act) {
        return Task::none(); // its key already did it
    }
    a.last_pad = Some((act, Instant::now()));
    do_action(a, act)
}

fn key_press(a: &mut App, key: Key, mods: Modifiers) -> Task<Msg> {
    use Action::*;
    let act = match key.as_ref() {
        Key::Named(Named::Escape) | Key::Named(Named::BrowserBack) => Some(Back),
        Key::Named(Named::ArrowUp) => Some(Up),
        Key::Named(Named::ArrowDown) => Some(Down),
        Key::Named(Named::ArrowLeft) => Some(Left),
        Key::Named(Named::ArrowRight) => Some(Right),
        Key::Named(Named::Enter) | Key::Named(Named::Space) => Some(Select),
        Key::Named(Named::PageDown) => Some(Next),
        Key::Named(Named::PageUp) => Some(Prev),
        Key::Character(c) if mods.control() && c.eq_ignore_ascii_case("f") => Some(Search),
        Key::Character(c) if mods.control() && c.eq_ignore_ascii_case("d") => Some(X),
        Key::Character(c) if mods.control() && c.eq_ignore_ascii_case("w") => Some(Close),
        Key::Character(c) if a.system.keep.is_some() && c.eq_ignore_ascii_case("x") => Some(X),
        _ => None,
    };
    // The keyboard used (not a key the controller typed): back to the keyboard-and-mouse look.
    if a.sh.big && a.fixed.is_none() && !nav::pad_recent(1500) {
        set_layout(a, false);
    }
    let Some(act) = act else { return Task::none() };
    if same(a.last_pad, act) {
        return Task::none(); // the controller's own press of it, already done
    }
    a.last_key = Some((act, Instant::now()));
    do_action(a, act)
}

fn window_focus(e: iced::Event, _s: iced::event::Status, _w: iced::window::Id) -> Option<Msg> {
    match e {
        iced::Event::Window(iced::window::Event::Focused) => Some(Msg::Focus(true)),
        iced::Event::Window(iced::window::Event::Unfocused) => Some(Msg::Focus(false)),
        _ => None,
    }
}

fn subscription(a: &App) -> Subscription<Msg> {
    let mut s = vec![
        Subscription::run(hearth_ui::theme::watch).map(|_| Msg::Theme),
        iced::keyboard::listen().map(Msg::Key),
        // The controller, read in its own sleeping thread (pad.rs): a message only when a button is pressed.
        Subscription::run(pad::presses).map(Msg::Pad),
        iced::event::listen_with(window_focus),
    ];
    s.extend(cat_subs(a));
    Subscription::batch(s)
}

fn view(a: &App) -> Element<'_, Msg> {
    frame::frame(a)
}

fn main() -> iced::Result {
    if app::handle_cli(
        "hearth-settings-app",
        "Hearth Settings: every page of Hearth's Settings in its own app.\n  [PAGE[:part]] | --page PAGE   open at a page (home, quick, wifi, bluetooth, display, sound, power, appearance, updates, about...)\n  --layout a|b   keep the keyboard-and-mouse look (a) or the controller look (b); otherwise it follows what you use\n  --focus TITLE  put the focus on the row with this name\n  --find WORDS   search\n  --press \"right down a\"  controller presses after a moment (up down left right a b x y lb rb menu)",
    ) {
        return Ok(());
    }
    iced::application(App::new, update, view)
        .title("Hearth Settings")
        .subscription(subscription)
        .theme(|a: &App| app::iced_theme(&a.sh.p))
        .window(app::window("hearth-settings", (1180.0, 760.0), (820.0, 540.0)))
        .default_font(app::INTER)
        .run()
}
