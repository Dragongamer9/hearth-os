//! `hearth-mantel session`: Desktop Mode's shell when the Rust Mantel test is installed (labwc's autostart runs it in
//! place of Quickshell; docs/MANTEL-RUST.md). One program with every surface, as the QML Mantel is one Quickshell:
//! the wallpaper (background layer), the bar along the top (Islands.qml), the dock along the bottom (Dock.qml) and,
//! while it's open, the app menu (Launcher.qml's Hearth menu, an overlay). One program shares one graphics context,
//! so it costs far less memory than a program per surface. If it can't start or keeps closing, labwc's autostart
//! marks the Store's copy as failed (hearth-part) and starts the QML Mantel, so Desktop Mode always comes up.

use iced::window::Id;
use iced::{keyboard, Color, Element, Size, Subscription, Task};
use iced_layershell::build_pattern::daemon;
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer, NewLayerShellSettings};
use iced_layershell::settings::{LayerShellSettings, Settings, StartMode};
use iced_layershell::to_layer_message;
use std::collections::HashSet;

use crate::apps::{self, App};
use crate::look::{self, Look};
use crate::{bar, desk, dock, menu, overview, panels, screen, sys, wall, wm};

#[to_layer_message(multi)]
#[derive(Debug, Clone)]
pub enum Message {
    Wm(wm::Snapshot),
    Look,
    Tick,
    Resized(Id, Size),
    WallReady(iced::widget::image::Handle),
    Cmd(String),
    // bar
    Flame,
    Desk(u32),
    GameMode,
    VolScroll(bool),
    VolMute,
    Nothing,
    // dock
    Task(usize),
    TaskMiddle(usize),
    ShowDesktop,
    // desktop layouts (desk.rs)
    DeskCheck,
    Overview,
    OverviewClose,
    OverviewOpen(u32),
    OverviewLaunch(String),
    OverviewApps,
    Win(u32),
    /// the Power block: the app menu, whose foot has Sign out, Restart and Shut down (no power menu of its own yet)
    Power,
    // menu
    Key(keyboard::Event),
    Open(usize),
    Tab(menu::Tab),
    Action(menu::Act),
    CloseMenu,
    Games(Vec<menu::Game>),
    System(HashSet<String>, HashSet<String>),
    // Build anything (screen.rs): your whole-screen desktop and the first start
    ScreenAct(String, String, String),
    ScreenPick(usize),
    ScreenTab(String),
    ScreenGuideAct(String, String),
    ScreenGames(Vec<screen::Game>),
    ScreenEdit,
    OwnTick,
    OwnOut(String, serde_json::Value),
    OwnAllow(String, String),
    FirstChoose(String),
    Editor(crate::editor::EMsg),
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Surf {
    Wall,
    Bar,
    Dock,
    Menu,
    Panel(usize),
    Overview,
    Screen,
    First,
    Editor,
}

pub struct Shell {
    surfs: Vec<(Id, Surf)>,
    pub look: Look,
    pub wall_img: Option<iced::widget::image::Handle>,
    pub apps: Vec<App>,
    pub pins: Vec<String>,
    pub wm: wm::Snapshot,
    pub status: sys::Status,
    pub bt: sys::Bt,
    pub clock: String,
    pub date: String,
    /// the screen's size (the wallpaper covers it all)
    pub screen_size: Size,
    /// Build anything: your whole-screen desktop (screen.rs)
    pub screen: screen::State,
    /// the block editor (editor.rs), while it's open
    pub editor: crate::editor::Editor,
    pub menu: menu::State,
    /// Show desktop: the windows it minimized (and the one in front), to bring back
    shown_desktop: Option<(Vec<u32>, Option<u32>)>,
    tick: u32,
    pub bar_band: u32,
    dock_h: u32,
    /// the desktop's layout (desk.rs): the bar's edge and blocks, the dock, the extra panels
    pub desk: desk::Desk,
    pub overview: overview::State,
}

impl Shell {
    fn id_of(&self, s: Surf) -> Option<Id> {
        self.surfs.iter().find(|x| x.1 == s).map(|x| x.0)
    }
    fn surf(&self, id: Id) -> Option<Surf> {
        self.surfs.iter().find(|x| x.0 == id).map(|x| x.1)
    }
    pub fn compact(&self) -> bool {
        self.screen_size.width > 0.0 && self.screen_size.width <= 1440.0
    }
    pub fn menu_open(&self) -> bool {
        self.id_of(Surf::Menu).is_some()
    }
    pub fn overview_open(&self) -> bool {
        self.id_of(Surf::Overview).is_some()
    }
}

fn layer(size: (u32, u32), anchor: Anchor, layer: Layer, zone: i32, keys: KeyboardInteractivity, ns: &str) -> NewLayerShellSettings {
    NewLayerShellSettings {
        size: Some(size),
        layer,
        anchor,
        exclusive_zone: Some(zone),
        keyboard_interactivity: keys,
        namespace: Some(ns.to_string()),
        ..Default::default()
    }
}

fn boot() -> (Shell, Task<Message>) {
    let l = look::get();
    let dk = desk::read();
    let st = Shell {
        surfs: Vec::new(),
        bar_band: bar::band(&l, false, dk.vertical()),
        dock_h: dock::height(&l, false, &dk.dock_style),
        look: l,
        wall_img: None,
        apps: apps::all(),
        pins: sys::pins(),
        wm: wm::Snapshot::default(),
        status: sys::Status::read(true),
        bt: sys::Bt::read(),
        clock: sys::clock_text(),
        date: sys::date_text(),
        screen_size: Size::ZERO,
        screen: { let mut st = screen::State::default(); screen::read(&mut st); st },
        editor: crate::editor::Editor::default(),
        menu: menu::State::default(),
        shown_desktop: None,
        tick: 0,
        desk: dk,
        overview: overview::State::default(),
    };
    let mut st = st;
    let w = Id::unique();
    st.surfs = vec![(w, Surf::Wall)];
    let all = Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right;
    let mut t = vec![Task::done(Message::NewLayerShell { settings: layer((0, 0), all, Layer::Background, -1, KeyboardInteractivity::None, "hearth-wallpaper"), id: w })];
    t.extend(build_bars(&mut st));
    t.extend(build_screen(&mut st));
    t.push(Task::perform(async { menu::system_lists() }, |(s, h)| Message::System(s, h)));
    t.push(Task::perform(async { screen::load_games() }, Message::ScreenGames));
    (st, Task::batch(t))
}

/// The main bar on its edge, the dock (when the layout has one) and the extra panels: made again whenever the
/// layout's surfaces change (an edge, the dock, a panel), so no space stays kept on an old edge.
fn build_bars(s: &mut Shell) -> Vec<Task<Message>> {
    let mut t = Vec::new();
    for (id, k) in s.surfs.clone() {
        if matches!(k, Surf::Bar | Surf::Dock | Surf::Panel(_)) {
            t.push(Task::done(Message::RemoveWindow(id)));
        }
    }
    s.surfs.retain(|x| !matches!(x.1, Surf::Bar | Surf::Dock | Surf::Panel(_)));
    // Build anything: a whole-screen desktop with its own Guide, menu and Game Mode covers the bar and dock
    if s.screen.hides_bars() {
        return t;
    }
    let c = s.compact();
    let vert = s.desk.vertical();
    s.bar_band = bar::band(&s.look, c, vert);
    let band = s.bar_band;
    let (anchor, size) = edge_anchor(&s.desk.edge, band);
    let b = Id::unique();
    s.surfs.push((b, Surf::Bar));
    t.push(Task::done(Message::NewLayerShell { settings: layer(size, anchor, Layer::Top, band as i32, KeyboardInteractivity::None, "hearth-islands"), id: b }));
    if s.desk.dock {
        let d = Id::unique();
        s.dock_h = dock::height(&s.look, c, &s.desk.dock_style);
        s.surfs.push((d, Surf::Dock));
        t.push(Task::done(Message::NewLayerShell {
            settings: layer((0, s.dock_h), Anchor::Bottom | Anchor::Left | Anchor::Right, Layer::Top, dock::reserved(&s.look, c, &s.desk.dock_style) as i32, KeyboardInteractivity::None, "hearth-dock"),
            id: d,
        }));
    }
    for (i, p) in s.desk.panels.iter().enumerate() {
        let th = panels::thickness(p);
        let (anchor, size) = edge_anchor(&p.edge, th);
        let id = Id::unique();
        s.surfs.push((id, Surf::Panel(i)));
        t.push(Task::done(Message::NewLayerShell { settings: layer(size, anchor, Layer::Top, if p.auto_hide { 0 } else { th as i32 }, KeyboardInteractivity::None, "hearth-panel"), id }));
    }
    t
}

/// Your whole-screen desktop on the bottom layer (under the windows) and, once on a new install, the first start.
fn build_screen(s: &mut Shell) -> Vec<Task<Message>> {
    let mut t = Vec::new();
    for (id, k) in s.surfs.clone() {
        if matches!(k, Surf::Screen) || (matches!(k, Surf::First) && !screen::first_due(&s.screen)) {
            t.push(Task::done(Message::RemoveWindow(id)));
            s.surfs.retain(|x| x.0 != id);
        }
    }
    let all = Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right;
    if s.screen.on {
        let id = Id::unique();
        s.surfs.push((id, Surf::Screen));
        t.push(Task::done(Message::NewLayerShell { settings: layer((0, 0), all, Layer::Bottom, -1, KeyboardInteractivity::OnDemand, "hearth-screen"), id }));
    }
    if screen::first_due(&s.screen) && s.id_of(Surf::First).is_none() {
        let id = Id::unique();
        s.surfs.push((id, Surf::First));
        t.push(Task::done(Message::NewLayerShell { settings: layer((0, 0), all, Layer::Overlay, -1, KeyboardInteractivity::Exclusive, "hearth-first-start"), id }));
    }
    t
}

/// The block editor (board 4), full screen over everything while it's open.
fn open_editor(s: &mut Shell, from: Option<serde_json::Value>, name: &str) -> Task<Message> {
    if s.id_of(Surf::Editor).is_some() {
        return Task::none();
    }
    s.editor = crate::editor::Editor::start(from.unwrap_or_else(|| s.screen.screen.clone()), name);
    let id = Id::unique();
    s.surfs.push((id, Surf::Editor));
    let all = Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right;
    Task::done(Message::NewLayerShell { settings: layer((0, 0), all, Layer::Overlay, -1, KeyboardInteractivity::Exclusive, "hearth-screen-editor"), id })
}
/// Try it full screen: the editor steps aside and the draft is drawn as your desktop; Escape brings the editor back.
pub fn try_full(s: &mut Shell, on: bool) -> Task<Message> {
    let mut t = Vec::new();
    let all = Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right;
    if on {
        if let Some(id) = s.id_of(Surf::Editor) {
            s.surfs.retain(|x| x.0 != id);
            t.push(Task::done(Message::RemoveWindow(id)));
        }
        if s.id_of(Surf::Screen).is_none() {
            let id = Id::unique();
            s.surfs.push((id, Surf::Screen));
            t.push(Task::done(Message::NewLayerShell { settings: layer((0, 0), all, Layer::Top, -1, KeyboardInteractivity::Exclusive, "hearth-screen"), id }));
        }
    } else {
        if !s.screen.on {
            if let Some(id) = s.id_of(Surf::Screen) {
                s.surfs.retain(|x| x.0 != id);
                t.push(Task::done(Message::RemoveWindow(id)));
            }
        }
        let id = Id::unique();
        s.surfs.push((id, Surf::Editor));
        t.push(Task::done(Message::NewLayerShell { settings: layer((0, 0), all, Layer::Overlay, -1, KeyboardInteractivity::Exclusive, "hearth-screen-editor"), id }));
    }
    Task::batch(t)
}
pub fn close_editor(s: &mut Shell) -> Task<Message> {
    match s.id_of(Surf::Editor) {
        Some(id) => {
            s.surfs.retain(|x| x.0 != id);
            Task::done(Message::RemoveWindow(id))
        }
        None => Task::none(),
    }
}

/// A bar's anchors and size along `edge`, `band` thick.
fn edge_anchor(edge: &str, band: u32) -> (Anchor, (u32, u32)) {
    match edge {
        "bottom" => (Anchor::Bottom | Anchor::Left | Anchor::Right, (0, band)),
        "left" => (Anchor::Left | Anchor::Top | Anchor::Bottom, (band, 0)),
        "right" => (Anchor::Right | Anchor::Top | Anchor::Bottom, (band, 0)),
        _ => (Anchor::Top | Anchor::Left | Anchor::Right, (0, band)),
    }
}

fn open_overview(s: &mut Shell) -> Task<Message> {
    if let Some(id) = s.id_of(Surf::Overview) {
        s.surfs.retain(|x| x.0 != id);
        return Task::done(Message::RemoveWindow(id));
    }
    let id = Id::unique();
    s.surfs.push((id, Surf::Overview));
    s.overview = overview::State::default();
    let all = Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right;
    // under a bar along the top, which stays
    let top = if s.desk.edge == "top" { s.bar_band as i32 } else { 0 };
    let settings = NewLayerShellSettings { margin: Some((top, 0, 0, 0)), ..layer((0, 0), all, Layer::Overlay, -1, KeyboardInteractivity::Exclusive, "hearth-overview") };
    Task::done(Message::NewLayerShell { settings, id })
}
fn close_overview(s: &mut Shell) -> Task<Message> {
    match s.id_of(Surf::Overview) {
        Some(id) => {
            s.surfs.retain(|x| x.0 != id);
            Task::done(Message::RemoveWindow(id))
        }
        None => Task::none(),
    }
}

/// After a change of look or screen: the bar's and dock's heights and reserved space, when they changed.
fn refit(s: &mut Shell) -> Task<Message> {
    let mut t = Vec::new();
    let c = s.compact();
    let vert = s.desk.vertical();
    let band = bar::band(&s.look, c, vert);
    if band != s.bar_band {
        s.bar_band = band;
        if let Some(id) = s.id_of(Surf::Bar) {
            t.push(Task::done(Message::SizeChange { id, size: if vert { (band, 0) } else { (0, band) } }));
            t.push(Task::done(Message::ExclusiveZoneChange { id, zone_size: band as i32 }));
        }
    }
    let dh = dock::height(&s.look, c, &s.desk.dock_style);
    if dh != s.dock_h {
        s.dock_h = dh;
        if let Some(id) = s.id_of(Surf::Dock) {
            t.push(Task::done(Message::SizeChange { id, size: (0, dh) }));
            t.push(Task::done(Message::ExclusiveZoneChange { id, zone_size: dock::reserved(&s.look, c, &s.desk.dock_style) as i32 }));
        }
    }
    Task::batch(t)
}

/// The wallpaper blended again (in the background: it takes a moment), for this look and screen.
fn redraw_wall(s: &Shell) -> Task<Message> {
    if s.screen_size.width <= 0.0 {
        return Task::none();
    }
    let (l, size) = (s.look.clone(), s.screen_size);
    Task::perform(async move { wall::compose(&l, size) }, Message::WallReady)
}

fn open_menu(s: &mut Shell) -> Task<Message> {
    if let Some(id) = s.id_of(Surf::Menu) {
        return close_menu(s, id);
    }
    let id = Id::unique();
    s.surfs.push((id, Surf::Menu));
    s.menu.reset();
    s.apps = apps::all();
    let all = Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right;
    Task::batch([
        Task::done(Message::NewLayerShell { settings: layer((0, 0), all, Layer::Overlay, -1, KeyboardInteractivity::Exclusive, "hearth-launcher"), id }),
        Task::perform(async { menu::load_games() }, Message::Games),
    ])
}

fn close_menu(s: &mut Shell, id: Id) -> Task<Message> {
    s.surfs.retain(|x| x.0 != id);
    Task::done(Message::RemoveWindow(id))
}

pub fn run_cmd(cmd: &str) {
    apps::run_detached(cmd);
}

fn update(s: &mut Shell, m: Message) -> Task<Message> {
    match m {
        Message::Wm(w) => s.wm = w,
        Message::Look => {
            s.look = look::get();
            s.clock = sys::clock_text();
            return Task::batch([refit(s), redraw_wall(s)]);
        }
        Message::Tick => {
            s.tick = s.tick.wrapping_add(1);
            s.clock = sys::clock_text();
            s.date = sys::date_text();
            let vol = true;
            let mut st = sys::Status::read(vol);
            if !vol {
                st.volume = s.status.volume;
            }
            s.status = st;
            s.bt = sys::Bt::read();
            s.pins = sys::pins();
        }
        Message::Resized(id, size) => {
            if s.surf(id) == Some(Surf::Wall) && size != s.screen_size {
                s.screen_size = size;
                return Task::batch([refit(s), redraw_wall(s)]);
            }
        }
        Message::Cmd(c) => {
            if c.trim() == "menu" || c.trim() == "launcher" {
                return open_menu(s);
            }
            if c.trim() == "overview" {
                return open_overview(s);
            }
            // hearth-mantel guide | edit | first (Build anything)
            match c.trim() {
                "guide" => s.screen.guide = !s.screen.guide,
                "edit" => return open_editor(s, None, ""),
                "first" => {
                    if s.id_of(Surf::First).is_none() {
                        let id = Id::unique();
                        s.surfs.push((id, Surf::First));
                        let all = Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right;
                        return Task::done(Message::NewLayerShell { settings: layer((0, 0), all, Layer::Overlay, -1, KeyboardInteractivity::Exclusive, "hearth-first-start"), id });
                    }
                }
                _ => {}
            }
        }
        Message::Flame => return open_menu(s),
        Message::DeskCheck => {
            let (was_on, was_hidden, was_from) = (s.screen.on, s.screen.hides_bars(), s.screen.from.clone());
            screen::read(&mut s.screen);
            let mut t = Vec::new();
            if s.screen.on != was_on || s.screen.from != was_from || s.id_of(Surf::First).is_some() != screen::first_due(&s.screen) {
                t.extend(build_screen(s));
            }
            if s.screen.hides_bars() != was_hidden {
                t.extend(build_bars(s));
            }
            if !t.is_empty() {
                return Task::batch(t);
            }
            let nd = desk::read();
            if nd != s.desk {
                let rebuild = nd.surfaces_key() != s.desk.surfaces_key();
                s.desk = nd;
                if rebuild {
                    return Task::batch(build_bars(s));
                }
            }
        }
        Message::Overview => return open_overview(s),
        Message::OverviewClose => return close_overview(s),
        Message::OverviewOpen(id) => {
            wm::activate(id);
            return close_overview(s);
        }
        Message::OverviewLaunch(app) => {
            if let Some(a) = apps::find(&s.apps, &app).cloned() {
                apps::launch(&a);
            }
            return close_overview(s);
        }
        Message::OverviewApps => return Task::batch([close_overview(s), open_menu(s)]),
        Message::Power => return open_menu(s),
        Message::Win(id) => {
            if s.wm.wins.iter().any(|w| w.id == id && w.active && !w.minimized) {
                wm::minimize(id);
            } else {
                wm::activate(id);
            }
        }
        Message::Desk(id) => wm::go_desk(id),
        Message::GameMode => run_cmd("nimbus-mode game"),
        Message::VolScroll(up) => {
            sys::volume_step(up);
            s.status.volume = sys::Status::read(true).volume;
        }
        Message::VolMute => {
            sys::volume_mute();
            s.status.volume = sys::Status::read(true).volume;
        }
        Message::Task(i) => {
            if let Some(it) = dock::items(&s.apps, &s.pins, &s.wm.wins).get(i) {
                dock::cycle(it);
            }
        }
        Message::TaskMiddle(i) => {
            if let Some(a) = dock::items(&s.apps, &s.pins, &s.wm.wins).get(i).and_then(|it| it.app.clone()) {
                apps::launch(&a);
            }
        }
        Message::ShowDesktop => {
            let open: Vec<&wm::Win> = s.wm.wins.iter().filter(|w| !w.minimized).collect();
            if !open.is_empty() {
                let front = open.iter().find(|w| w.active).map(|w| w.id);
                let ids: Vec<u32> = open.iter().map(|w| w.id).collect();
                for id in &ids {
                    wm::minimize(*id);
                }
                s.shown_desktop = Some((ids, front));
            } else if let Some((ids, front)) = s.shown_desktop.take() {
                for id in ids.iter().filter(|i| Some(**i) != front) {
                    wm::activate(*id);
                }
                if let Some(f) = front {
                    wm::activate(f);
                }
            }
        }
        Message::Key(k) => {
            if s.id_of(Surf::Editor).is_some() {
                return crate::editor::key(s, k);
            }
            if s.screen.on && !s.menu_open() && !s.overview_open() {
                if let keyboard::Event::KeyPressed { key: keyboard::Key::Named(nk), .. } = &k {
                    match nk {
                        keyboard::key::Named::Home => s.screen.guide = !s.screen.guide,
                        keyboard::key::Named::Escape => s.screen.guide = false,
                        keyboard::key::Named::ArrowLeft => s.screen.picked = s.screen.picked.saturating_sub(1),
                        keyboard::key::Named::ArrowRight => s.screen.picked = (s.screen.picked + 1).min(s.screen.games.len().saturating_sub(1)),
                        keyboard::key::Named::Enter => return screen::act(s, "play", ""),
                        _ => {}
                    }
                }
            }
            if s.overview_open() {
                return match overview::key(s, k) {
                    overview::KeyResult::Close => close_overview(s),
                    overview::KeyResult::Open(w) => {
                        wm::activate(w);
                        close_overview(s)
                    }
                    overview::KeyResult::Search(q) => {
                        let t = close_overview(s);
                        let m = open_menu(s);
                        s.menu.q = q;
                        Task::batch([t, m])
                    }
                    overview::KeyResult::None => Task::none(),
                };
            }
            if let Some(id) = s.id_of(Surf::Menu) {
                match menu::key(s, k) {
                    menu::KeyResult::Close => return close_menu(s, id),
                    menu::KeyResult::Launched => return close_menu(s, id),
                    menu::KeyResult::None => {}
                }
            }
        }
        Message::Open(i) => {
            if let Some(id) = s.id_of(Surf::Menu) {
                if menu::open_index(s, i) {
                    return close_menu(s, id);
                }
            }
        }
        Message::Tab(t) => menu::set_tab(s, t),
        Message::Action(a) => {
            menu::act(a);
            if let Some(id) = s.id_of(Surf::Menu) {
                return close_menu(s, id);
            }
        }
        Message::CloseMenu => {
            if let Some(id) = s.id_of(Surf::Menu) {
                return close_menu(s, id);
            }
        }
        Message::Games(g) => {
            s.menu.games = g;
            s.menu.games_loading = false;
        }
        Message::System(sysids, hidden) => {
            s.menu.system_ids = sysids;
            s.menu.hidden = hidden;
        }
        Message::ScreenAct(a, arg, snd) => {
            screen::sound(&snd);
            return screen::act(s, &a, &arg);
        }
        Message::ScreenGuideAct(a, arg) => {
            s.screen.guide = false;
            if a != "guide" {
                return screen::act(s, &a, &arg);
            }
        }
        Message::ScreenPick(i) => s.screen.picked = i,
        Message::ScreenTab(t) => s.screen.tab = t,
        Message::ScreenGames(g) => s.screen.games = g,
        Message::ScreenEdit => return open_editor(s, None, ""),
        Message::OwnTick => {
            let ids = s.screen.own_ids().into_iter().chain(s.editor.own_ids()).collect::<Vec<_>>();
            return Task::batch(ids.into_iter().map(|id| Task::perform(async move { let v = screen::run_own(&id, false); (id, v) }, |(id, v)| Message::OwnOut(id, v))));
        }
        Message::OwnOut(id, v) => {
            s.screen.own.insert(id, v);
        }
        Message::OwnAllow(id, ans) => {
            if ans == "once" {
                return Task::perform(async move { let v = screen::run_own(&id, true); (id, v) }, |(id, v)| Message::OwnOut(id, v));
            }
            let bin = screen::layout_bin();
            let _ = std::process::Command::new(bin).args(["block-allow", &id, &ans]).status();
            return Task::done(Message::OwnTick);
        }
        Message::FirstChoose(what) => {
            match what.as_str() {
                "build" => {
                    screen::choose(&mut s.screen, "later");
                    let v: serde_json::Value = serde_json::from_str(hearth_layout::screen::READY[0].1).unwrap_or_default();
                    return Task::batch([Task::batch(build_screen(s)), open_editor(s, Some(v), "Console dashboard")]);
                }
                "nothing" => {
                    screen::choose(&mut s.screen, "later");
                    let v = serde_json::json!({ "grid": { "cols": 24, "rows": 14 }, "background": { "kind": "gradient", "from": "#1a1410", "to": "#0c0907" }, "look": {}, "blocks": [] });
                    return Task::batch([Task::batch(build_screen(s)), open_editor(s, Some(v), "Start from nothing")]);
                }
                w => screen::choose(&mut s.screen, w),
            }
            let mut t = build_screen(s);
            t.extend(build_bars(s));
            return Task::batch(t);
        }
        Message::Editor(m) => return crate::editor::update(s, m),
        Message::WallReady(h) => {
            s.wall_img = Some(h);
            // the blending's working memory back to the system at once
            unsafe { libc::malloc_trim(0) };
        }
        Message::Nothing => {}
        _ => {}
    }
    Task::none()
}

fn view(s: &Shell, id: Id) -> Element<'_, Message> {
    match s.surf(id) {
        Some(Surf::Wall) => match &s.wall_img {
            Some(h) => iced::widget::image(h.clone()).width(iced::Length::Fill).height(iced::Length::Fill).content_fit(iced::ContentFit::Fill).into(),
            None => {
                let bg = s.look.p.bg;
                iced::widget::container(iced::widget::Space::new()).width(iced::Length::Fill).height(iced::Length::Fill).style(move |_| iced::widget::container::Style { background: Some(bg.into()), ..Default::default() }).into()
            }
        },
        Some(Surf::Bar) => bar::view(s),
        Some(Surf::Dock) => dock::view(s),
        Some(Surf::Menu) => menu::view(s),
        Some(Surf::Panel(i)) => panels::view(s, i),
        Some(Surf::Overview) => overview::view(s),
        Some(Surf::Screen) => screen::view(s),
        Some(Surf::First) => screen::first_view(s),
        Some(Surf::Editor) => crate::editor::view(s),
        None => iced::widget::Space::new().into(),
    }
}

fn subscription(s: &Shell) -> Subscription<Message> {
    let mut v = vec![
        Subscription::run(wm::stream).map(Message::Wm),
        Subscription::run(look::changes).map(|_| Message::Look),
        Subscription::run(sys::commands).map(Message::Cmd),
        Subscription::run(desk::changes).map(|_| Message::DeskCheck),
        // the clock, the status icons and the pins: once a minute, on the minute (the volume at once after you change
        // it). Every wake-up redraws the shell, so there's nothing in between: it stays asleep while you work.
        Subscription::run(sys::minutes).map(|_| Message::Tick),
        iced::window::resize_events().map(|(id, size)| Message::Resized(id, size)),
    ];
    if s.menu_open() || s.overview_open() || s.screen.on || s.id_of(Surf::Editor).is_some() {
        v.push(keyboard::listen().map(Message::Key));
    }
    // your AI's blocks: a look every 10 s while they're on screen (each its own boxed-in program)
    if (s.screen.on && !s.screen.own_ids().is_empty()) || !s.editor.own_ids().is_empty() {
        v.push(Subscription::run(screen::ten_seconds).map(|_| Message::OwnTick));
    }
    Subscription::batch(v)
}

pub fn run() -> i32 {
    iced_layershell::disable_clipboard();
    let font = look::get().font;
    let r = daemon(boot, || "hearth-mantel".to_string(), update, view)
        .style(|_s: &Shell, _t: &iced::Theme| iced::theme::Style { background_color: Color::TRANSPARENT, text_color: look::pal().text })
        .subscription(subscription)
        .settings(Settings { layer_settings: LayerShellSettings { start_mode: StartMode::Background, ..Default::default() }, default_font: font, ..Default::default() })
        .run();
    crate::ended("session", r)
}
