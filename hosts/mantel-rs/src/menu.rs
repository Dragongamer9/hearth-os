//! The app menu, drawn as the QML Mantel's Hearth menu (Launcher.qml): a large centred panel over the darkened
//! desktop. "Apps" with its Apps / Games buttons, the search ("just start typing"), ALL APPS (your apps and Hearth's,
//! then the apps that came with the system in one "System apps" folder), and along the bottom Game Mode, Settings,
//! Sleep, Restart, Shut down and Log out, with the controller's hints. Arrows move, Enter opens, Tab switches
//! between Apps and Games, Esc closes (or leaves the folder). It's its own surface only while it's open.

use iced::widget::{button, column, container, mouse_area, row, scrollable, stack, text, Space};
use iced::{border, keyboard, Alignment, Background, Border, Color, Element, Length, Padding};
use serde_json::Value;
use std::collections::HashSet;

use crate::apps::{self, App};
use crate::look::{self, a, lighter, Look};
use crate::shell::{Message, Shell};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Tab {
    #[default]
    Apps,
    Games,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Act {
    GameMode,
    Settings,
    Sleep,
    Restart,
    ShutDown,
    LogOut,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Game {
    pub id: String,
    pub title: String,
    pub cover: String,
}

#[derive(Debug, Default)]
pub struct State {
    pub q: String,
    pub tab: Tab,
    pub current: usize,
    pub folder_open: bool,
    pub games: Vec<Game>,
    pub games_loading: bool,
    /// the apps that came with the system image, and the ones not for this desktop
    pub system_ids: HashSet<String>,
    pub hidden: HashSet<String>,
}

impl State {
    pub fn reset(&mut self) {
        self.q.clear();
        self.tab = Tab::Apps;
        self.current = 0;
        self.folder_open = false;
        self.games_loading = true;
    }
}

/// One tile in the grid.
#[derive(Debug, Clone)]
pub enum Tile<'a> {
    App(&'a App),
    Folder,
    Game(&'a Game),
}

/// Hearth's own apps (and Beacon, Xodus Nexus, Bazaar): never in the System apps folder.
fn ours(a: &App) -> bool {
    let id = a.id.to_lowercase();
    id.starts_with("hearth") || id.starts_with("nimbus") || id.starts_with("beacon") || id == "xodus-nexus" || a.id == "io.github.kolunmi.Bazaar" || a.name.starts_with("Hearth")
}

fn listed(s: &Shell) -> Vec<&App> {
    let legacy = std::env::var("HEARTH_LEGACY_GAME_MODE").map(|v| v == "1").unwrap_or(false);
    let mut v: Vec<&App> = s.apps.iter().filter(|a| !s.menu.hidden.contains(&a.id) && a.id != "qt5ct" && a.id != "qt6ct" && (legacy || a.id != "nimbus")).collect();
    v.sort_by(|x, y| x.name.to_lowercase().cmp(&y.name.to_lowercase()));
    v
}

fn system_app(s: &Shell, a: &App) -> bool {
    !ours(a) && s.menu.system_ids.contains(&a.id)
}

/// What the grid shows now.
pub fn tiles(s: &Shell) -> Vec<Tile<'_>> {
    let q = s.menu.q.trim().to_lowercase();
    if s.menu.tab == Tab::Games {
        let mut g: Vec<&Game> = s.menu.games.iter().filter(|g| q.is_empty() || g.title.to_lowercase().contains(&q)).collect();
        g.sort_by_key(|g| !g.title.to_lowercase().starts_with(&q));
        return g.into_iter().map(Tile::Game).collect();
    }
    let all = listed(s);
    if q.is_empty() {
        let (sys, main): (Vec<&App>, Vec<&App>) = all.into_iter().partition(|a| system_app(s, a));
        if s.menu.folder_open {
            return sys.into_iter().map(Tile::App).collect();
        }
        let mut v: Vec<Tile> = main.into_iter().map(Tile::App).collect();
        if !sys.is_empty() {
            v.push(Tile::Folder);
        }
        return v;
    }
    // ranked: a name that is the query or starts with it first, then a word starting with it, then anywhere
    let mut hits: Vec<(u8, &App)> = all
        .into_iter()
        .filter(|a| apps::matches(a, &q))
        .map(|a| {
            let n = a.name.to_lowercase();
            let rank = if n == q { 0 } else if n.starts_with(&q) { 1 } else if n.split_whitespace().any(|w| w.starts_with(&q)) { 2 } else { 3 };
            (rank, a)
        })
        .collect();
    hits.sort_by(|x, y| x.0.cmp(&y.0).then(x.1.name.to_lowercase().cmp(&y.1.name.to_lowercase())));
    let mut v: Vec<Tile> = hits.into_iter().map(|h| Tile::App(h.1)).collect();
    for g in s.menu.games.iter().filter(|g| g.title.to_lowercase().contains(&q)).take(8) {
        v.push(Tile::Game(g));
    }
    v
}

fn shell_helper(name: &str) -> String {
    look::shell_file(name).to_string_lossy().to_string()
}

/// Your installed games from Hearth's library (hearth-games: what it said last, at once).
pub fn load_games() -> Vec<Game> {
    let run = |arg: &str| -> Option<Vec<Game>> {
        let o = std::process::Command::new("python3").arg(shell_helper("hearth-games")).arg(arg).output().ok()?;
        let t = String::from_utf8_lossy(&o.stdout);
        let v: Value = serde_json::from_str(t.trim().lines().last()?).ok()?;
        Some(
            v.as_array()?
                .iter()
                .filter_map(|g| {
                    let s = |k: &str| g.get(k).and_then(Value::as_str).unwrap_or("").to_string();
                    let title = s("title");
                    (!title.is_empty()).then(|| Game { id: s("id"), cover: [s("cover"), s("wide"), s("icon")].into_iter().find(|x| !x.is_empty()).unwrap_or_default(), title })
                })
                .collect(),
        )
    };
    run("cached").filter(|g| !g.is_empty()).or_else(|| run("list")).unwrap_or_default()
}

/// The apps that came with the system (the image's /usr/share/applications, less the ones you added through Hearth's
/// package layer), and the ones hearth-app-kinds says this desktop doesn't show.
pub fn system_lists() -> (HashSet<String>, HashSet<String>) {
    let added: HashSet<String> = std::fs::read_to_string("/var/lib/hearth/layer/added-apps").unwrap_or_default().lines().map(|l| l.trim().trim_end_matches(".desktop").to_string()).filter(|l| !l.is_empty()).collect();
    let sys: HashSet<String> = std::fs::read_dir("/usr/share/applications")
        .map(|d| d.flatten().filter_map(|e| e.file_name().to_str().and_then(|n| n.strip_suffix(".desktop")).map(str::to_string)).filter(|n| !added.contains(n)).collect())
        .unwrap_or_default();
    let hidden: HashSet<String> = std::process::Command::new("python3")
        .args(["/usr/libexec/hearth-app-kinds", "hidden"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).lines().map(|l| l.trim().trim_end_matches(".desktop").to_string()).filter(|l| !l.is_empty()).collect())
        .unwrap_or_default();
    (sys, hidden)
}

pub fn set_tab(s: &mut Shell, t: Tab) {
    if s.menu.tab != t {
        s.menu.tab = t;
        s.menu.folder_open = false;
        s.menu.current = 0;
    }
}

/// Opens the tile at i; true when the menu closes for it.
pub fn open_index(s: &mut Shell, i: usize) -> bool {
    let what = tiles(s).get(i).map(|t| match t {
        Tile::App(a) => Some(Ok((*a).clone())),
        Tile::Game(g) => Some(Err(g.id.clone())),
        Tile::Folder => None,
    });
    match what {
        Some(Some(Ok(a))) => {
            apps::launch(&a);
            true
        }
        Some(Some(Err(id))) => {
            apps::run_detached(&format!("python3 {} play {}", apps::shell_quote(&shell_helper("hearth-games")), apps::shell_quote(&id)));
            true
        }
        Some(None) => {
            s.menu.folder_open = true;
            s.menu.current = 0;
            false
        }
        None => false,
    }
}

pub fn act(a: Act) {
    let cmd = match a {
        Act::GameMode => "nimbus-mode game".to_string(),
        Act::Settings => "hearth-settings".to_string(),
        Act::Sleep => format!("python3 {} sleep 'Desktop Mode: menu'", apps::shell_quote(&shell_helper("hearth-buttons"))),
        Act::Restart => "systemctl reboot".to_string(),
        Act::ShutDown => "systemctl poweroff".to_string(),
        Act::LogOut => "nimbus-mode logout".to_string(),
    };
    apps::run_detached(&cmd);
}

pub enum KeyResult {
    None,
    Close,
    Launched,
}

/// Columns in the grid now (GridView: as many 140 px cells as fit).
fn cols(s: &Shell) -> usize {
    let (bw, _) = box_size(s.screen);
    let cw = bw - 60.0;
    ((cw / if s.menu.tab == Tab::Games { 150.0 } else { 140.0 }).floor() as usize).max(1)
}

pub fn key(s: &mut Shell, e: keyboard::Event) -> KeyResult {
    use keyboard::key::Named;
    let keyboard::Event::KeyPressed { key, text, modifiers, .. } = e else { return KeyResult::None };
    let n = tiles(s).len();
    match key.as_ref() {
        keyboard::Key::Named(Named::Escape) => {
            if s.menu.folder_open && s.menu.q.is_empty() {
                s.menu.folder_open = false;
                s.menu.current = 0;
            } else {
                return KeyResult::Close;
            }
        }
        keyboard::Key::Named(Named::Enter) => {
            let i = s.menu.current;
            if open_index(s, i) {
                return KeyResult::Launched;
            }
        }
        keyboard::Key::Named(Named::Tab) => {
            let t = if s.menu.tab == Tab::Apps { Tab::Games } else { Tab::Apps };
            set_tab(s, t);
        }
        keyboard::Key::Named(Named::ArrowRight) => s.menu.current = (s.menu.current + 1).min(n.saturating_sub(1)),
        keyboard::Key::Named(Named::ArrowLeft) => s.menu.current = s.menu.current.saturating_sub(1),
        keyboard::Key::Named(Named::ArrowDown) => s.menu.current = (s.menu.current + cols(s)).min(n.saturating_sub(1)),
        keyboard::Key::Named(Named::ArrowUp) => s.menu.current = s.menu.current.saturating_sub(cols(s)),
        keyboard::Key::Named(Named::Backspace) => {
            s.menu.q.pop();
            s.menu.current = 0;
        }
        _ => {
            if let Some(t) = text {
                if !modifiers.control() && !modifiers.alt() && !modifiers.logo() && t.chars().all(|c| !c.is_control()) {
                    s.menu.q.push_str(&t);
                    s.menu.current = 0;
                }
            }
        }
    }
    KeyResult::None
}

/// The panel's size on this screen (insets: the bar at the top, the dock at the bottom).
fn box_size(screen: iced::Size) -> (f32, f32) {
    let (w, h) = if screen.width > 0.0 { (screen.width, screen.height) } else { (1920.0, 1080.0) };
    ((w - 60.0).min(1240.0), h)
}

fn tab_btn<'a>(l: &Look, label: &'a str, current: bool, t: Tab) -> Element<'a, Message> {
    let p = l.p;
    let r = l.r(14.0);
    let txt = if current { l.tb(label, 14.0, p.ink) } else { l.t(label, 14.0, p.text) };
    button(container(txt).center_y(Length::Fixed(44.0)))
        .padding(Padding { left: 22.0, right: 22.0, ..Padding::ZERO })
        .height(Length::Fixed(44.0))
        .on_press(Message::Tab(t))
        .style(move |_, st| button::Style {
            background: Some(Background::Color(if current {
                p.accent
            } else if matches!(st, button::Status::Hovered | button::Status::Pressed) {
                p.panel3
            } else {
                p.panel2
            })),
            border: border::rounded(r),
            ..button::Style::default()
        })
        .into()
}

fn action<'a>(l: &Look, label: &'a str, icon: &str, primary: bool, act: Act) -> Element<'a, Message> {
    let p = l.p;
    let r = l.r(14.0);
    let c = if primary { p.ink } else { p.text };
    let txt = if primary { l.tb(label, 15.0, c) } else { l.t(label, 15.0, c) };
    button(container(row![look::icon(icon, 20.0, c), txt].spacing(10).align_y(Alignment::Center)).center_y(Length::Fixed(52.0)))
        .padding(Padding { left: 18.0, right: 18.0, ..Padding::ZERO })
        .height(Length::Fixed(52.0))
        .on_press(Message::Action(act))
        .style(move |_, st| {
            let hover = matches!(st, button::Status::Hovered | button::Status::Pressed);
            button::Style {
                background: Some(Background::Color(if primary {
                    if hover { lighter(p.accent, 1.12) } else { p.accent }
                } else if hover {
                    p.panel3
                } else {
                    p.panel2
                })),
                border: Border { color: if primary { Color::TRANSPARENT } else { p.line }, width: 1.0, radius: r.into() },
                ..button::Style::default()
            }
        })
        .into()
}

fn hint<'a>(l: &Look, b: &'a str, c: Color, t: &'a str) -> Element<'a, Message> {
    let circle = container(text(b).size(if b.len() > 1 { 9.0 } else { 11.0 }).color(Color::WHITE).font(l.bold()))
        .center(Length::Fixed(24.0))
        .style(move |_| container::Style { background: Some(c.into()), border: border::rounded(12.0), ..container::Style::default() });
    row![circle, l.t(t, 13.0, l.p.muted)].spacing(6).align_y(Alignment::Center).into()
}

fn tile_view<'a>(s: &'a Shell, l: &Look, t: &Tile<'a>, i: usize, cell_w: f32, cell_h: f32) -> Element<'a, Message> {
    let p = l.p;
    let cur = i == s.menu.current;
    let r = l.r(14.0);
    let (pic, label): (Element<'a, Message>, String) = match t {
        Tile::App(a) => (look::app_pic(a.icon_path.as_deref(), 64.0), a.name.clone()),
        Tile::Folder => (look::app_pic(apps::icon_path("folder").as_deref(), 64.0), "System apps".into()),
        Tile::Game(g) => {
            let r10 = l.r(10.0);
            let p3 = p.panel3;
            let cover: Element<'a, Message> = if !g.cover.is_empty() && std::path::Path::new(g.cover.trim_start_matches("file://")).exists() {
                iced::widget::image(look::image_handle(std::path::Path::new(g.cover.trim_start_matches("file://")))).width(120).height(168).content_fit(iced::ContentFit::Cover).into()
            } else {
                l.t(g.title.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default(), 40.0, p.muted).into()
            };
            (container(cover).center_x(Length::Fixed(120.0)).center_y(Length::Fixed(168.0)).style(move |_| container::Style { background: Some(p3.into()), border: border::rounded(r10), ..container::Style::default() }).into(), g.title.clone())
        }
    };
    let name = l.t(label, 13.0, p.text).width(Length::Fixed(cell_w - 16.0)).align_x(Alignment::Center);
    let face = column![pic, container(name).max_height(36.0)].spacing(8).align_x(Alignment::Center);
    let pinned = matches!(t, Tile::App(a) if s.pins.iter().any(|x| x == &a.id));
    let mut layers = stack![container(face).center(Length::Fill)];
    if pinned {
        layers = layers.push(container(look::icon("pin", 14.0, p.muted)).align_right(Length::Fill).padding(12));
    }
    let accent = p.accent;
    let (p2, p3) = (p.panel2, p.panel3);
    button(layers)
        .width(Length::Fixed(cell_w))
        .height(Length::Fixed(cell_h))
        .padding(0)
        .on_press(Message::Open(i))
        .style(move |_, st| {
            let hover = matches!(st, button::Status::Hovered | button::Status::Pressed);
            if cur {
                button::Style { background: Some(p3.into()), border: Border { color: accent, width: 1.0, radius: r.into() }, ..button::Style::default() }
            } else if hover {
                button::Style { background: Some(p2.into()), border: border::rounded(r), ..button::Style::default() }
            } else {
                button::Style::default()
            }
        })
        .into()
}

/// font.letterSpacing: 1.3 (iced has no letter spacing): a hair space between the letters.
fn spaced(t: &str) -> String {
    let mut o = String::new();
    for (i, c) in t.chars().enumerate() {
        if i > 0 {
            o.push('\u{200A}');
        }
        o.push(c);
    }
    o
}

pub fn view(s: &Shell) -> Element<'_, Message> {
    let l = &s.look;
    let p = l.p;
    let screen = if s.screen.width > 0.0 { s.screen } else { iced::Size::new(1920.0, 1080.0) };
    let top = crate::bar::band(l, s.compact()) as f32;
    let bottom = crate::dock::reserved(l, s.compact()) as f32;
    let (top_room, bottom_room) = (top.max(20.0), bottom.max(20.0));
    let bw = (screen.width - 60.0).min(1240.0);
    let bh = (screen.height - top_room - bottom_room - 30.0).min(860.0);
    let by = top_room + ((screen.height - top_room - bottom_room - bh) / 2.0).max(10.0);

    let games = s.menu.tab == Tab::Games;
    // header: "Apps", the section buttons, the hint
    let header = row![
        container(l.ts(if games { "Games" } else { "Apps" }, 22.0, p.text)).padding(Padding { right: 10.0, ..Padding::ZERO }),
        tab_btn(l, "Apps", !games, Tab::Apps),
        tab_btn(l, "Games", games, Tab::Games),
        Space::new().width(Length::Fill),
        l.t(if s.menu.folder_open && s.menu.q.is_empty() { "Esc goes back · Tab switches" } else { "Tab switches" }, 12.0, p.muted),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    // the search: the accent ring (it has the keys), the magnifier, what you typed or what to type
    let q = &s.menu.q;
    let typed: Element<'_, Message> = if q.is_empty() {
        l.t(if games { "Search games" } else { "Search apps, games and Settings pages" }, 19.0, p.muted).into()
    } else {
        row![l.t(q.clone(), 19.0, p.text), container(Space::new().width(Length::Fixed(2.0)).height(Length::Fixed(24.0))).style({
            let c = p.accent;
            move |_| container::Style { background: Some(c.into()), ..container::Style::default() }
        })]
        .spacing(1)
        .align_y(Alignment::Center)
        .into()
    };
    let mut srow = row![Space::new().width(Length::Fixed(20.0)), look::icon("search", 22.0, p.accent), Space::new().width(Length::Fixed(14.0)), container(typed).width(Length::Fill)].align_y(Alignment::Center);
    if q.is_empty() {
        srow = srow.push(l.t("Just start typing", 13.0, p.muted));
    }
    srow = srow.push(Space::new().width(Length::Fixed(24.0)));
    let (bg2, accent, rad30) = (p.bg2, p.accent, l.r(30.0));
    let search = container(srow).height(Length::Fixed(60.0)).width(Length::Fill).align_y(Alignment::Center).style(move |_| container::Style {
        background: Some(a(bg2, 0.8).into()),
        border: Border { color: accent, width: 2.0, radius: rad30.into() },
        ..container::Style::default()
    });

    // the grid
    let cw = bw - 60.0;
    let per = if games { 150.0 } else { 140.0 };
    let ncols = ((cw / per).floor() as usize).max(1);
    let cell_w = (cw / ncols as f32).floor();
    let cell_h = if games { 236.0 } else { 128.0 };
    let list = tiles(s);
    let mut grid = column![].spacing(0);
    for (ri, chunk) in list.chunks(ncols).enumerate() {
        let mut r = row![].spacing(0);
        for (ci, t) in chunk.iter().enumerate() {
            r = r.push(tile_view(s, l, t, ri * ncols + ci, cell_w, cell_h));
        }
        grid = grid.push(r);
    }
    let body: Element<'_, Message> = if list.is_empty() {
        let msg = if !q.is_empty() {
            format!("No {} match \"{}\"", if games { "games" } else { "apps" }, q)
        } else if games {
            if s.menu.games_loading { "Finding your games…".into() } else { "No games installed yet. Games you install in Hearth show up here.".into() }
        } else {
            "No apps".into()
        };
        container(l.t(msg, 14.0, p.muted)).center(Length::Fill).into()
    } else {
        scrollable(grid).height(Length::Fill).into()
    };

    let mut col = column![header, search].spacing(16);
    if !games && q.is_empty() {
        col = col.push(text(spaced(if s.menu.folder_open { "SYSTEM APPS" } else { "ALL APPS" })).size(12).color(p.muted).font(l.semi()));
    }
    let line = p.line;
    let mut actions = row![].spacing(10).align_y(Alignment::Center);
    if !l.game_mode_off {
        actions = actions.push(action(l, "Game Mode", "pad", true, Act::GameMode));
    }
    actions = actions
        .push(action(l, "Settings", "gear", false, Act::Settings))
        .push(action(l, "Sleep", "sleep", false, Act::Sleep))
        .push(action(l, "Restart", "restart", false, Act::Restart))
        .push(action(l, "Shut down", "power", false, Act::ShutDown))
        .push(action(l, "Log out", "logout", false, Act::LogOut))
        .push(Space::new().width(Length::Fill));
    if screen.width > 1500.0 {
        actions = actions
            .push(hint(l, "A", Color::from_rgb8(0x3f, 0xae, 0x5a), "Open"))
            .push(hint(l, "X", Color::from_rgb8(0x3b, 0x7d, 0xe0), "Options"))
            .push(hint(l, "LB", Color::from_rgb8(0x6c, 0x74, 0x80), "Tab"));
    }
    col = col
        .push(body)
        .push(container(Space::new()).width(Length::Fill).height(Length::Fixed(1.0)).style(move |_| container::Style { background: Some(line.into()), ..container::Style::default() }))
        .push(actions);

    // the panel (Frame.qml, solid: a little less see-through), over the darkened desktop and the fire's glow
    let fill = a(p.panel, l.glass_a.max(0.9));
    let (edge, r26) = (p.line, l.r(26.0));
    let panel = container(col)
        .padding(30)
        .width(Length::Fixed(bw))
        .height(Length::Fixed(bh))
        .style(move |_| container::Style { background: Some(fill.into()), border: Border { color: edge, width: 1.0, radius: r26.into() }, ..container::Style::default() });
    let panel = mouse_area(panel).on_press(Message::Nothing);
    let placed = container(panel).padding(Padding { top: by, ..Padding::ZERO }).center_x(Length::Fill).height(Length::Fill);
    let scrim = Color::from_rgba(0.02, 0.02, 0.03, 0.72);
    // (Launcher.qml has the ember glow over the scrim too, but the QML Mantel draws it so faintly it can't be seen in
    // its pictures; left out here so the two look the same.)
    let back = container(Space::new()).width(Length::Fill).height(Length::Fill).style(move |_| container::Style { background: Some(scrim.into()), ..container::Style::default() });
    mouse_area(stack![back, placed]).on_press(Message::CloseMenu).into()
}
