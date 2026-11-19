//! Hearth Settings as its own app, in pure Rust (hearth-ui, iced): not part of Mantel's shell and not the Game Mode
//! app, so it opens the same on KDE Plasma, Firehold or any desktop. The layout is the approved new Settings'
//! (layout A, docs/mockups/next-update/settings-A1-home-mouse.png and settings-A2-display-mouse.png). Every page of
//! Mantel's Settings is in the registry (pages_list.rs); each category is its own module (cats/, all of the same
//! shape: cats/mod.rs), and the frame (frame.rs) draws the rest. What's built: docs/mockups/checklists/settings-rust.md.
//!
//! Hearth's own settings are written only through hearth_settings_io.py (atomic, .bak, merged, the change history
//! with Undo: red flag 9); the system's through its own tools. Opening it writes nothing.
//!
//! `hearth-settings-app [PAGE[:part]]` or `--page PAGE` opens at a page (Mantel's page ids).

mod cats;
mod data;
mod frame;
mod pages_list;
mod registry;
mod shared;
mod widgets;

use cats::{a11y, accounts, apps, bt, gaming, home, hs, link, net, pers, privacy, system, updates};
use hearth_ui::app::{self, win};
use hearth_ui::run;
use iced::keyboard::{key::Named, Event as KEvent, Key, Modifiers};
use iced::{Element, Subscription, Task};
use shared::Shared;

/// Every category module, once: (its field in App, its Msg variant, the module). Adding a category is one line here.
macro_rules! each_cat {
    ($mac:ident) => {
        $mac! {
            (home, Home, home),
            (system, System, system),
            (net, Net, net),
            (bt, Bt, bt),
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

macro_rules! define {
    ($(($f:ident, $v:ident, $m:ident)),*) => {
        pub struct App {
            pub sh: Shared,
            pub cat: usize,
            pub page: &'static str,
            pub query: String,
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
            App { sh, cat: registry::cat_of(first), page: first, query: String::new(), $($f: $m::State::default(),)* }
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
    };
}

each_cat!(define);

fn info() -> Task<Msg> {
    Task::perform(run::spawn(data::sh(data::INFO)), Msg::Info)
}
fn history() -> Task<Msg> {
    Task::perform(run::spawn(vec!["python3".into(), shared::io_helper(), "history".into(), "20".into()]), Msg::History)
}

impl App {
    fn new() -> (App, Task<Msg>) {
        let arg = app::arg("--page").or_else(|| std::env::args().skip(1).find(|a| !a.starts_with('-')));
        let (first, part) = arg.as_deref().and_then(registry::resolve).unwrap_or(("home", String::new()));
        let mut a = new_app(first, part);
        let t = open_page(&mut a, first);
        (a, Task::batch([info(), history(), t]))
    }

    fn open(&mut self, id: &'static str) -> Task<Msg> {
        self.page = id;
        self.cat = registry::cat_of(id);
        self.query.clear();
        self.sh.error.clear();
        self.sh.note.clear();
        open_page(self, id)
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
            Task::none()
        }
        Msg::Key(KEvent::KeyPressed { key, modifiers, .. }) => key_press(a, key, modifiers),
        Msg::Key(_) => Task::none(),
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

fn back(a: &mut App) -> Task<Msg> {
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
            return a.open(up.id);
        }
    }
    let landing = registry::CATEGORIES[registry::cat_of(p.id)].2;
    if a.page != landing {
        return a.open(landing);
    }
    Task::none()
}

fn key_press(a: &mut App, key: Key, mods: Modifiers) -> Task<Msg> {
    let n = registry::CATEGORIES.len();
    match key {
        Key::Named(Named::Escape) | Key::Named(Named::BrowserBack) => back(a),
        Key::Named(Named::PageDown) => update(a, Msg::Cat((a.cat + 1) % n)),
        Key::Named(Named::PageUp) => update(a, Msg::Cat((a.cat + n - 1) % n)),
        Key::Character(c) if mods.control() && c.as_str() == "w" => win::close(),
        Key::Character(c) if a.system.keep.is_some() && (c.as_str() == "x" || c.as_str() == "X") => update(a, Msg::System(system::Msg::Keep)),
        _ => Task::none(),
    }
}

fn subscription(a: &App) -> Subscription<Msg> {
    let mut s = vec![Subscription::run(hearth_ui::theme::watch).map(|_| Msg::Theme), iced::keyboard::listen().map(Msg::Key)];
    s.extend(cat_subs(a));
    Subscription::batch(s)
}

fn view(a: &App) -> Element<'_, Msg> {
    frame::frame(a)
}

fn main() -> iced::Result {
    if app::handle_cli("hearth-settings-app", "Hearth Settings: every page of Hearth's Settings in its own app.\n  [PAGE[:part]] | --page PAGE   open at a page (home, wifi, bluetooth, display, sound, power, appearance, updates, about...)") {
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
