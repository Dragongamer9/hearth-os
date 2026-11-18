//! `firehold-shell couch`: the Couch (mockups 1, 4 and 5): Home, the Library, Apps, Store, and Everything open. One
//! full-screen layer surface. While the Couch is up it's on the overlay layer with the keyboard; on the Desk (or with
//! something launched over it) it steps down to the background layer, draws only the warm background (the Desk's
//! wallpaper) and takes no keys. Moved with the controller (the hub's nav), the keyboard's arrows, or the mouse.

use iced::widget::{canvas, column, container, image, mouse_area, responsive, row, stack, text, text_input, Space};
use iced::{Background, Border, Color, Element, Length, Padding, Size, Subscription, Task};
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, Settings, StartMode};
use iced_layershell::to_layer_message;
use serde_json::json;

use super::art::{Dashed, Flame};
use super::style::*;
use super::{hint, key_nav, tile_bg};
use crate::link::{self, HubMsg};
use crate::model::{self, Game, Win};
use crate::proto::State;
use crate::sys::App;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Zone {
    Tabs,
    Hero,
    Recent,
    Open,
    Filters,
    Grid,
    Switcher,
}

const TABS: [(&str, &str); 4] = [("home", "Home"), ("library", "Library"), ("apps", "Apps"), ("store", "Store")];

#[to_layer_message]
#[derive(Debug, Clone)]
pub enum Message {
    Hub(HubMsg),
    Key(iced::keyboard::Event),
    Pick(u8, usize),
    Press(u8, usize),
    Search(String),
}

struct Couch {
    st: State,
    lib: Vec<Game>,
    apps: Vec<App>,
    tab: String,
    switcher: bool,
    zone: Zone,
    idx: usize,
    filter: usize,
    search: String,
    layer: Option<bool>,
    told_page: String,
}

fn zone_code(z: Zone) -> u8 {
    z as u8
}
fn zone_of(c: u8) -> Zone {
    [Zone::Tabs, Zone::Hero, Zone::Recent, Zone::Open, Zone::Filters, Zone::Grid, Zone::Switcher].get(c as usize).copied().unwrap_or(Zone::Tabs)
}

impl Couch {
    fn hero(&self) -> Option<&Game> {
        model::continue_game(&self.lib, &self.st.wins)
    }
    fn recent(&self) -> Vec<&Game> {
        model::recent(&self.lib, self.hero(), 7)
    }
    fn open_wins(&self) -> Vec<&Win> {
        self.st.wins.iter().filter(|w| !w.game).collect()
    }
    fn filters(&self) -> Vec<(String, String, usize)> {
        model::counts(&self.lib)
    }
    fn library_list(&self) -> Vec<Game> {
        let f = self.filters().get(self.filter).map(|f| f.0.clone()).unwrap_or_default();
        let q = self.search.trim().to_lowercase();
        model::library_order(&self.lib).into_iter().filter(|g| model::filter_ok(g, &f) && (q.is_empty() || g.title.to_lowercase().contains(&q))).collect()
    }
    fn grid_len(&self) -> usize {
        match self.tab.as_str() {
            "library" => self.library_list().len(),
            "apps" => self.apps.len(),
            "store" => 1,
            _ => 0,
        }
    }
    fn grid_cols(&self) -> usize {
        if self.tab == "apps" { 8 } else { 6 }
    }
    fn zone_len(&self, z: Zone) -> usize {
        match z {
            Zone::Tabs => TABS.len(),
            Zone::Hero => if self.hero().is_some() { 2 } else { 0 },
            Zone::Recent => self.recent().len(),
            Zone::Open => self.open_wins().len(),
            Zone::Filters => self.filters().len(),
            Zone::Grid => self.grid_len(),
            Zone::Switcher => self.st.wins.len(),
        }
    }
    fn zones(&self) -> Vec<Zone> {
        if self.switcher {
            return vec![Zone::Switcher];
        }
        let all: Vec<Zone> = match self.tab.as_str() {
            "home" => vec![Zone::Tabs, Zone::Hero, Zone::Recent, Zone::Open],
            "library" => vec![Zone::Tabs, Zone::Filters, Zone::Grid],
            _ => vec![Zone::Tabs, Zone::Grid],
        };
        all.into_iter().filter(|z| *z == Zone::Tabs || self.zone_len(*z) > 0).collect()
    }
    fn tab_index(&self) -> usize {
        TABS.iter().position(|(k, _)| *k == self.tab).unwrap_or(0)
    }
    fn go_tab(&mut self, i: usize) {
        self.tab = TABS[i % TABS.len()].0.to_string();
        self.switcher = false;
        self.zone = if self.tab == "home" && self.hero().is_some() { Zone::Hero } else if self.tab == "library" { Zone::Grid } else { Zone::Grid };
        self.idx = 0;
        self.fix();
    }
    fn fix(&mut self) {
        let zs = self.zones();
        if !zs.contains(&self.zone) {
            self.zone = zs.get(1).copied().unwrap_or(zs[0]);
            self.idx = if self.zone == Zone::Tabs { self.tab_index() } else { 0 };
        }
        let n = self.zone_len(self.zone);
        if n == 0 {
            self.idx = 0;
        } else if self.idx >= n {
            self.idx = n - 1;
        }
    }
    fn page_name(&self) -> String {
        if self.switcher { "switcher".into() } else { self.tab.clone() }
    }

    fn nav(&mut self, key: &str) {
        let zs = self.zones();
        let zi = zs.iter().position(|z| *z == self.zone).unwrap_or(0);
        let grid = matches!(self.zone, Zone::Grid | Zone::Switcher);
        let cols = if self.zone == Zone::Switcher { 3 } else { self.grid_cols() };
        let n = self.zone_len(self.zone);
        match key {
            "left" if self.idx > 0 && (!grid || self.idx % cols > 0) => self.idx -= 1,
            "right" if self.idx + 1 < n && (!grid || (self.idx + 1) % cols > 0) => self.idx += 1,
            "up" if grid && self.idx >= cols => self.idx -= cols,
            "down" if grid && self.idx + cols < n => self.idx += cols,
            "down" if grid && self.idx / cols < (n.saturating_sub(1)) / cols => self.idx = n - 1,
            "up" if zi > 0 => {
                self.zone = zs[zi - 1];
                self.idx = if self.zone == Zone::Tabs { self.tab_index() } else { 0 };
            }
            "down" if zi + 1 < zs.len() => {
                self.zone = zs[zi + 1];
                self.idx = 0;
            }
            "lb" => {
                let i = (self.tab_index() + TABS.len() - 1) % TABS.len();
                self.go_tab(i)
            }
            "rb" => {
                let i = self.tab_index() + 1;
                self.go_tab(i)
            }
            "lt" | "rt" if self.tab == "library" && !self.switcher => {
                let m = self.filters().len();
                self.filter = if key == "rt" { (self.filter + 1) % m } else { (self.filter + m - 1) % m };
                self.idx = 0;
            }
            "a" => self.press(),
            "b" => {
                if self.switcher {
                    self.switcher = false;
                    self.go_tab(0);
                } else if self.tab != "home" {
                    self.go_tab(0);
                }
            }
            "x" => self.x(),
            "y" => {
                if self.switcher {
                    if let Some(w) = self.st.wins.get(self.idx) {
                        link::send(json!({"op": "fullscreen", "id": w.id}));
                    }
                } else {
                    self.go_tab(1);
                }
            }
            _ => {}
        }
        self.fix();
    }

    fn x(&mut self) {
        if self.switcher {
            if let Some(w) = self.st.wins.get(self.idx) {
                link::send(json!({"op": "close", "id": w.id}));
            }
            return;
        }
        let id = match self.zone {
            Zone::Hero => self.hero().map(|g| g.id.clone()),
            Zone::Recent => self.recent().get(self.idx).map(|g| g.id.clone()),
            Zone::Grid if self.tab == "library" => self.library_list().get(self.idx).map(|g| g.id.clone()),
            _ => None,
        };
        if let Some(id) = id {
            link::send(json!({"op": "gamePage", "id": id}));
        }
    }

    fn press(&mut self) {
        match self.zone {
            Zone::Tabs => {
                let i = self.idx;
                self.go_tab(i);
            }
            Zone::Hero => {
                if let Some(g) = self.hero() {
                    let op = if self.idx == 1 { "openWindowed" } else { "play" };
                    link::send(json!({"op": op, "id": g.id}));
                }
            }
            Zone::Recent => {
                if let Some(g) = self.recent().get(self.idx) {
                    link::send(json!({"op": "play", "id": g.id}));
                }
            }
            Zone::Open => {
                if let Some(w) = self.open_wins().get(self.idx) {
                    link::send(json!({"op": "activate", "id": w.id}));
                }
            }
            Zone::Filters => {
                self.filter = self.idx;
                self.zone = Zone::Grid;
                self.idx = 0;
            }
            Zone::Grid => match self.tab.as_str() {
                "library" => {
                    if let Some(g) = self.library_list().get(self.idx) {
                        link::send(json!({"op": "play", "id": g.id}));
                    }
                }
                "apps" => {
                    if let Some(a) = self.apps.get(self.idx) {
                        link::send(json!({"op": "launch", "app": a.id}));
                    }
                }
                "store" => link::op("store"),
                _ => {}
            },
            Zone::Switcher => {
                if let Some(w) = self.st.wins.get(self.idx) {
                    link::send(json!({"op": "activate", "id": w.id}));
                    self.switcher = false;
                }
            }
        }
    }

    /// The layer and keyboard the hub's state asks for, as layer-shell changes (only when they change).
    fn place(&mut self) -> Task<Message> {
        let up = self.st.couch_up && self.st.posture == "couch";
        if self.layer == Some(up) {
            return Task::none();
        }
        self.layer = Some(up);
        let (layer, kb) = if up { (Layer::Overlay, KeyboardInteractivity::Exclusive) } else { (Layer::Background, KeyboardInteractivity::None) };
        Task::batch([Task::done(Message::LayerChange(layer)), Task::done(Message::KeyboardInteractivityChange(kb))])
    }
    fn tell_page(&mut self) {
        let p = if self.st.couch_up && self.st.posture == "couch" { self.page_name() } else { "hidden".into() };
        if p != self.told_page {
            self.told_page = p.clone();
            link::send(json!({"op": "couchPage", "page": p}));
        }
    }
}

fn update(s: &mut Couch, m: Message) -> Task<Message> {
    let mut task = Task::none();
    match m {
        Message::Hub(h) => match h {
            HubMsg::State(st) => {
                s.st = *st;
                s.fix();
                task = s.place();
            }
            HubMsg::Library(l) => {
                s.lib = l;
                s.fix();
            }
            HubMsg::Apps(a) => s.apps = a,
            HubMsg::Nav(k) => s.nav(&k),
            HubMsg::Cmd(cmd, arg) => match (cmd.as_str(), arg.as_str()) {
                ("page", "switcher") => {
                    s.switcher = true;
                    s.zone = Zone::Switcher;
                    s.idx = s.st.wins.iter().position(|w| w.id == s.st.active).unwrap_or(0);
                    s.fix();
                }
                ("page", "home") => s.go_tab(0),
                ("tab", t) => {
                    if let Some(i) = TABS.iter().position(|(k, _)| *k == t) {
                        s.go_tab(i);
                    }
                }
                ("filter", f) => {
                    if let Some(i) = s.filters().iter().position(|x| x.0 == f) {
                        s.filter = i;
                    }
                }
                _ => {}
            },
            HubMsg::Quit | HubMsg::Gone => return iced::exit(),
        },
        Message::Key(ev) => {
            if s.st.couch_up {
                if let Some(k) = key_nav(&ev) {
                    s.nav(k);
                }
            }
        }
        Message::Pick(z, i) => {
            s.zone = zone_of(z);
            s.idx = i;
            s.fix();
        }
        Message::Press(z, i) => {
            s.zone = zone_of(z);
            s.idx = i;
            s.fix();
            s.press();
        }
        Message::Search(q) => {
            s.search = q;
            s.idx = 0;
        }
        _ => {}
    }
    s.tell_page();
    task
}

fn sel(s: &Couch, z: Zone, i: usize) -> bool {
    s.zone == z && s.idx == i
}

fn clickable<'a>(e: impl Into<Element<'a, Message>>, z: Zone, i: usize) -> Element<'a, Message> {
    mouse_area(e).on_press(Message::Press(zone_code(z), i)).on_enter(Message::Pick(zone_code(z), i)).into()
}

fn top_bar<'a>(s: &'a Couch, k: f32, search: bool) -> Element<'a, Message> {
    let logo = container(canvas(Flame(DARK_INK)).width(Length::Fixed(26.0 * k)).height(Length::Fixed(26.0 * k)))
        .center_x(Length::Fixed(44.0 * k))
        .center_y(Length::Fixed(44.0 * k))
        .style(boxed(ACCENT, 10.0 * k, None));
    let mut tabs = row![].spacing(28.0 * k).align_y(iced::Center);
    for (i, (key, label)) in TABS.iter().enumerate() {
        let on = s.tab == *key && !s.switcher;
        let focus = sel(s, Zone::Tabs, i);
        let lbl = text(*label).size(22.0 * k).font(if on { bold() } else { medium() }).color(if on || focus { TEXT } else { MUTED });
        // The underline, as wide as the word (about 0.58 em a letter in Inter at this size).
        let under_w = label.chars().count() as f32 * 0.58 * 22.0 * k;
        let under = container(Space::new().width(Length::Fixed(under_w)).height(Length::Fixed(3.0 * k)))
            .width(Length::Fixed(under_w))
            .style(boxed(if on { ACCENT } else if focus { MUTED } else { Color::TRANSPARENT }, 2.0, None));
        tabs = tabs.push(clickable(column![lbl, under].spacing(6.0 * k).width(Length::Shrink), Zone::Tabs, i));
    }
    let left = row![logo, tabs].spacing(28.0 * k).align_y(iced::Center);
    let right: Element<Message> = if search {
        let count = s.lib.len();
        let input = text_input(&format!("Search {count} games"), &s.search)
            .on_input(Message::Search)
            .size(19.0 * k)
            .font(INTER)
            .padding(Padding { top: 14.0 * k, right: 20.0 * k, bottom: 14.0 * k, left: 0.0 })
            .style(move |_t, _st| text_input::Style {
                background: Background::Color(Color::TRANSPARENT),
                border: Border::default(),
                icon: MUTED,
                placeholder: MUTED,
                value: TEXT,
                selection: Color { a: 0.4, ..ACCENT },
            });
        container(row![t("⌕", 22.0 * k, MUTED), input].spacing(12.0 * k).align_y(iced::Center))
            .padding(Padding { top: 0.0, right: 8.0 * k, bottom: 0.0, left: 26.0 * k })
            .width(Length::Fixed(498.0 * k))
            .height(Length::Fixed(54.0 * k))
            .center_y(Length::Fixed(54.0 * k))
            .style(boxed(c(0x1d, 0x15, 0x12), 27.0 * k, Some((LINE, 1.0))))
            .into()
    } else {
        let pill_text = if s.st.cfg.posture == "couch" { "Couch".to_string() } else { "Couch · switches to Desk when you touch the mouse".to_string() };
        let dot = container(Space::new().width(Length::Fixed(8.0 * k)).height(Length::Fixed(8.0 * k))).style(boxed(ACCENT, 4.0 * k, None));
        let pill = container(row![dot, ts(pill_text, 15.0 * k, TEXT)].spacing(10.0 * k).align_y(iced::Center))
            .padding(Padding { top: 9.0 * k, right: 18.0 * k, bottom: 9.0 * k, left: 18.0 * k })
            .style(boxed(c(0x24, 0x1b, 0x16), 20.0 * k, None));
        let avatar = container(tb(s.st.initial.clone(), 20.0 * k, TEXT))
            .center_x(Length::Fixed(44.0 * k))
            .center_y(Length::Fixed(44.0 * k))
            .style(boxed(c(0x2a, 0x20, 0x1a), 22.0 * k, None));
        let mut r = row![pill].spacing(22.0 * k).align_y(iced::Center);
        if !s.st.net.is_empty() {
            r = r.push(t(s.st.net.clone(), 20.0 * k, MUTED));
        }
        if !s.st.battery.is_empty() {
            r = r.push(t(s.st.battery.clone(), 20.0 * k, MUTED));
        }
        r.push(tb(s.st.clock.clone(), 20.0 * k, TEXT)).push(avatar).into()
    };
    row![left, Space::new().width(Length::Fill), right].align_y(iced::Center).height(Length::Fixed(if search { 54.0 } else { 44.0 } * k)).into()
}

fn button_box<'a>(label: &'a str, primary: bool, focus: bool, k: f32) -> Element<'a, Message> {
    let (bg, fg) = if primary { (ACCENT, DARK_INK) } else { (c(0x1e, 0x15, 0x12), TEXT) };
    container(text(label).size(21.0 * k).font(bold()).color(fg))
        .padding(Padding { top: 0.0, right: 30.0 * k, bottom: 0.0, left: 30.0 * k })
        .center_y(Length::Fixed(58.0 * k))
        .style(boxed(bg, 12.0 * k, focus.then_some((c(0xff, 0xc0, 0x80), 3.0))))
        .into()
}

fn home<'a>(s: &'a Couch, k: f32) -> Element<'a, Message> {
    let mut col = column![].spacing(0.0);
    // The hero: the game to continue.
    if let Some(g) = s.hero() {
        let ago = model::ago(g.last_played, s.st.now_ms);
        let running = s.st.wins.iter().any(|w| w.game && w.game_id == g.id);
        let line1 = if running { "CONTINUE · OPEN NOW".to_string() } else if ago.is_empty() { "CONTINUE".to_string() } else { format!("CONTINUE · {}", ago.to_uppercase()) };
        let buttons = row![
            clickable(button_box(if running || g.last_played > 0 { "Resume" } else { "Play" }, true, sel(s, Zone::Hero, 0), k), Zone::Hero, 0),
            clickable(button_box("Open in a window", false, sel(s, Zone::Hero, 1), k), Zone::Hero, 1),
        ]
        .spacing(14.0 * k);
        let words = column![
            ts(line1, 18.0 * k, FLAME),
            Space::new().height(Length::Fixed(8.0 * k)),
            tb(g.title.clone(), 64.0 * k, TEXT),
            Space::new().height(Length::Fixed(10.0 * k)),
            t(model::hero_line(g), 21.0 * k, c(0xe8, 0xdc, 0xd0)),
            Space::new().height(Length::Fixed(24.0 * k)),
            buttons,
        ]
        .width(Length::FillPortion(11));
        let art = stack![
            canvas(Dashed(Color { a: 0.45, ..c(0xf0, 0xd0, 0xb8) }, 16.0 * k)).width(Length::Fill).height(Length::Fill),
            container(t("Game art", 18.0 * k, Color { a: 0.7, ..TEXT })).center(Length::Fill),
        ];
        let art = container(art).width(Length::FillPortion(8)).height(Length::Fixed(344.0 * k));
        let hero = container(row![words, art].spacing(40.0 * k).align_y(iced::Center))
            .padding(Padding { top: 48.0 * k, right: 60.0 * k, bottom: 48.0 * k, left: 48.0 * k })
            .width(Length::Fill)
            .height(Length::Fixed(440.0 * k))
            .center_y(Length::Fixed(440.0 * k))
            .style(boxed(hero_bg(), 24.0 * k, None));
        col = col.push(hero);
    } else {
        col = col.push(
            container(column![tb("Welcome to Firehold", 48.0 * k, TEXT), t(if s.st.library_note.is_empty() { "Your games show up here once Hearth has found them.".to_string() } else { s.st.library_note.clone() }, 21.0 * k, MUTED)].spacing(12.0 * k))
                .padding(48.0 * k)
                .width(Length::Fill)
                .height(Length::Fixed(440.0 * k))
                .center_y(Length::Fixed(440.0 * k))
                .style(boxed(hero_bg(), 24.0 * k, None)),
        );
    }
    // Recent.
    let recent = s.recent();
    if !recent.is_empty() {
        col = col.push(Space::new().height(Length::Fixed(46.0 * k))).push(ts("Recent", 22.0 * k, TEXT)).push(Space::new().height(Length::Fixed(14.0 * k)));
        let mut r = row![].spacing(20.0 * k);
        for (i, g) in recent.iter().enumerate() {
            let on = sel(s, Zone::Recent, i);
            let tile = container(column![Space::new().height(Length::Fill), t(g.store(), 13.0 * k, c(0xe0, 0xd6, 0xcc))].padding(Padding { top: 0.0, right: 0.0, bottom: 10.0 * k, left: 10.0 * k }))
                .width(Length::Fixed(228.0 * k))
                .height(Length::Fixed(128.0 * k))
                .style(if on { Box::new(ring(tile_bg(&g.id), 12.0 * k)) as Box<dyn Fn(&iced::Theme) -> container::Style> } else { Box::new(boxed(tile_bg(&g.id), 12.0 * k, None)) });
            let item = column![tile, ts(g.title.clone(), 18.0 * k, TEXT)].spacing(14.0 * k).width(Length::Fixed(228.0 * k));
            r = r.push(clickable(item, Zone::Recent, i));
        }
        col = col.push(r);
    }
    // Open on this PC: the same windows the Desk shows.
    let open = s.open_wins();
    if !open.is_empty() {
        col = col
            .push(Space::new().height(Length::Fixed(40.0 * k)))
            .push(row![ts("Open on this PC", 22.0 * k, TEXT), t(" · the same windows you see on the Desk", 18.0 * k, MUTED)].align_y(iced::Alignment::End))
            .push(Space::new().height(Length::Fixed(14.0 * k)));
        let mut r = row![].spacing(16.0 * k);
        for (i, w) in open.iter().enumerate().take(6) {
            let on = sel(s, Zone::Open, i);
            let icon = container(Space::new()).width(Length::Fixed(40.0 * k)).height(Length::Fixed(40.0 * k)).style(boxed(tile_bg(&w.app_id), 8.0 * k, None));
            let mut words = column![ts(w.name.clone(), 18.0 * k, TEXT)].spacing(2.0 * k);
            if !w.sub.is_empty() {
                words = words.push(t(w.sub.clone(), 14.0 * k, MUTED));
            }
            let card = container(row![icon, words].spacing(14.0 * k).align_y(iced::Center))
                .padding(Padding { top: 0.0, right: 22.0 * k, bottom: 0.0, left: 22.0 * k })
                .center_y(Length::Fixed(74.0 * k))
                .style(if on { Box::new(ring(c(0x1d, 0x15, 0x12), 12.0 * k)) as Box<dyn Fn(&iced::Theme) -> container::Style> } else { Box::new(boxed(c(0x1d, 0x15, 0x12), 12.0 * k, Some((LINE, 1.0)))) });
            r = r.push(clickable(card, Zone::Open, i));
        }
        col = col.push(r);
    }
    col.into()
}

fn library<'a>(s: &'a Couch, k: f32, avail: Size) -> Element<'a, Message> {
    let mut chips = row![].spacing(12.0 * k);
    for (i, (_, label, n)) in s.filters().into_iter().enumerate() {
        let on = i == s.filter;
        let focus = sel(s, Zone::Filters, i);
        let (bg, fg, cnt) = if on { (ACCENT, DARK_INK, c(0x6a, 0x2c, 0x0c)) } else { (c(0x24, 0x1b, 0x16), TEXT, MUTED) };
        let chip = container(row![tb(label, 18.0 * k, fg), t(n.to_string(), 18.0 * k, cnt)].spacing(6.0 * k))
            .padding(Padding { top: 10.0 * k, right: 20.0 * k, bottom: 10.0 * k, left: 20.0 * k })
            .style(boxed(bg, 22.0 * k, focus.then_some((TEXT, 2.0))));
        chips = chips.push(clickable(chip, Zone::Filters, i));
    }
    let list = s.library_list();
    let cols = 6usize;
    let tile_w = ((avail.width - 5.0 * 24.0 * k) / 6.0).max(80.0);
    let tile_h = tile_w * 1.2;
    let row_h = tile_h + 54.0 * k;
    let rows_fit = (((avail.height - 330.0 * k) / row_h).floor() as usize).max(1);
    let sel_row = if s.zone == Zone::Grid { s.idx / cols } else { 0 };
    let first_row = sel_row.saturating_sub(rows_fit - 1);
    let mut grid = column![].spacing(24.0 * k);
    for r in first_row..(first_row + rows_fit) {
        let mut line = row![].spacing(24.0 * k);
        let mut any = false;
        for cix in 0..cols {
            let i = r * cols + cix;
            let Some(g) = list.get(i) else { break };
            any = true;
            let on = sel(s, Zone::Grid, i);
            let playing = s.st.wins.iter().any(|w| w.game && w.game_id == g.id);
            let tag = container(text(g.store()).size(13.0 * k).font(semi()).color(TEXT))
                .padding(Padding { top: 4.0 * k, right: 9.0 * k, bottom: 4.0 * k, left: 9.0 * k })
                .style(boxed(c(0x1c, 0x14, 0x10), 6.0 * k, None));
            let inner = column![tag, Space::new().height(Length::Fill), t(g.state(playing), 14.0 * k, c(0xe0, 0xd6, 0xcc))].padding(14.0 * k);
            let tile = container(inner).width(Length::Fixed(tile_w)).height(Length::Fixed(tile_h)).style(if on {
                Box::new(boxed(tile_bg(&g.id), 14.0 * k, Some((ACCENT, 3.0)))) as Box<dyn Fn(&iced::Theme) -> container::Style>
            } else {
                Box::new(boxed(tile_bg(&g.id), 14.0 * k, None))
            });
            line = line.push(clickable(column![tile, ts(g.title.clone(), 18.0 * k, TEXT)].spacing(16.0 * k).width(Length::Fixed(tile_w)), Zone::Grid, i));
        }
        if !any {
            break;
        }
        grid = grid.push(line);
    }
    let mut col = column![chips, Space::new().height(Length::Fixed(40.0 * k))];
    if list.is_empty() {
        let why = if !s.st.library_note.is_empty() { s.st.library_note.clone() } else if s.lib.is_empty() { "Hearth hasn't found any games yet.".to_string() } else { "Nothing here matches.".to_string() };
        col = col.push(t(why, 21.0 * k, MUTED));
    } else {
        col = col.push(grid);
    }
    col.into()
}

fn apps<'a>(s: &'a Couch, k: f32) -> Element<'a, Message> {
    let cols = 8;
    let mut grid = column![].spacing(20.0 * k);
    let sel_row = if s.zone == Zone::Grid { s.idx / cols } else { 0 };
    let first = sel_row.saturating_sub(3);
    for r in first..first + 4 {
        let mut line = row![].spacing(20.0 * k);
        for cix in 0..cols {
            let i = r * cols + cix;
            let Some(a) = s.apps.get(i) else { break };
            let on = sel(s, Zone::Grid, i);
            let letter = model::dock_letter(&a.name, false);
            let tile = container(tb(letter, 40.0 * k, TEXT)).center_x(Length::Fixed(196.0 * k)).center_y(Length::Fixed(140.0 * k)).style(boxed(
                tile_bg(&a.id),
                14.0 * k,
                on.then_some((ACCENT, 3.0)),
            ));
            line = line.push(clickable(column![tile, ts(a.name.clone(), 17.0 * k, TEXT)].spacing(12.0 * k).width(Length::Fixed(196.0 * k)), Zone::Grid, i));
        }
        grid = grid.push(line);
    }
    column![ts(format!("{} apps", s.apps.len()), 22.0 * k, TEXT), grid].spacing(24.0 * k).into()
}

fn store<'a>(s: &'a Couch, k: f32) -> Element<'a, Message> {
    let on = sel(s, Zone::Grid, 0);
    let card = container(column![tb("Get apps and games", 30.0 * k, TEXT), t("Opens the app store (Bazaar, or KDE Discover) full screen.", 19.0 * k, MUTED)].spacing(10.0 * k))
        .padding(36.0 * k)
        .width(Length::Fixed(720.0 * k))
        .style(boxed(c(0x1d, 0x15, 0x12), 18.0 * k, Some(if on { (ACCENT, 3.0) } else { (LINE, 1.0) })));
    clickable(card, Zone::Grid, 0)
}

fn switcher<'a>(s: &'a Couch, k: f32, avail: Size) -> Element<'a, Message> {
    let head = column![tb("Open now", 40.0 * k, TEXT), t("Games and windows are one list. Pick one to jump to it, in Couch or Desk.", 19.0 * k, MUTED)].spacing(8.0 * k);
    let card_w = ((avail.width - 2.0 * 40.0 * k) / 3.0).max(120.0);
    let card_h = card_w * 0.546;
    let mut grid = column![].spacing(34.0 * k);
    let cols = 3;
    let rows_fit = (((avail.height - 330.0 * k) / (card_h + 70.0 * k)).floor() as usize).max(1);
    let first = (s.idx / cols).saturating_sub(rows_fit - 1);
    for r in first..first + rows_fit {
        let mut line = row![].spacing(40.0 * k);
        let mut any = false;
        for cix in 0..cols {
            let i = r * cols + cix;
            let Some(w) = s.st.wins.get(i) else { break };
            any = true;
            let on = sel(s, Zone::Switcher, i);
            let colour = if w.game { tile_bg(&w.game_id) } else { c(0x2a, 0x25, 0x22) };
            let pic: Element<Message> = if !w.pic.is_empty() {
                image(image::Handle::from_path(&w.pic)).width(Length::Fill).height(Length::Fill).content_fit(iced::ContentFit::Cover).into()
            } else {
                Space::new().width(Length::Fill).height(Length::Fill).into()
            };
            let tag = container(text(if w.game { "GAME" } else { "WINDOW" }).size(13.0 * k).font(bold()).color(if w.game { FLAME } else { c(0xd8, 0xcc, 0xc0) }))
                .padding(Padding { top: 5.0 * k, right: 11.0 * k, bottom: 5.0 * k, left: 11.0 * k })
                .style(boxed(c(0x1c, 0x14, 0x10), 8.0 * k, None));
            let over = container(tag).align_right(Length::Fill).padding(14.0 * k);
            let bg: Background = if w.game { hero_bg() } else { colour.into() };
            let card = container(stack![pic, over]).width(Length::Fixed(card_w)).height(Length::Fixed(card_h)).clip(true).style(if on {
                Box::new(ring(bg, 16.0 * k)) as Box<dyn Fn(&iced::Theme) -> container::Style>
            } else {
                Box::new(boxed(bg, 16.0 * k, None))
            });
            let label = row![ts(w.name.clone(), 21.0 * k, TEXT), t(w.sub.clone(), 16.0 * k, MUTED)].spacing(10.0 * k).align_y(iced::Alignment::End);
            line = line.push(clickable(column![card, label].spacing(12.0 * k).width(Length::Fixed(card_w)), Zone::Switcher, i));
        }
        if !any {
            break;
        }
        grid = grid.push(line);
    }
    let body: Element<Message> = if s.st.wins.is_empty() { t("Nothing's open right now.", 21.0 * k, MUTED).into() } else { grid.into() };
    column![head, Space::new().height(Length::Fixed(60.0 * k)), body].into()
}

fn hints<'a>(s: &'a Couch, k: f32) -> Element<'a, Message> {
    let mut r = row![].spacing(34.0 * k).align_y(iced::Center);
    if s.switcher {
        r = r.push(hint("A", "Go to it", k)).push(hint("X", "Close", k)).push(hint("Y", "Window ⇄ full screen", k)).push(hint("B", "Back", k));
    } else if s.tab == "library" {
        for l in ["A Play", "X Game options", "Y Search", "LT RT Filters"] {
            r = r.push(t(l, 18.0 * k, TEXT));
        }
    } else {
        r = r.push(hint("A", if s.tab == "home" { "Play" } else { "Open" }, k)).push(hint("X", "Options", k)).push(hint("Y", "Search", k));
        r = r.push(hint("≡", if s.st.cfg.guide == "open" { "Guide: everything open" } else { "Guide: Quick Access" }, k)).push(hint("LB RB", "Tabs", k));
    }
    row![Space::new().width(Length::Fill), r].into()
}

fn view(s: &Couch) -> Element<'_, Message> {
    let up = s.st.couch_up && s.st.posture == "couch";
    let glow = if s.switcher { c(0x2c, 0x16, 0x0a) } else if s.tab == "home" { c(0x2e, 0x17, 0x0b) } else { c(0x22, 0x12, 0x0a) };
    if !up {
        // The Desk's wallpaper: the same warm background, nothing else drawn.
        return container(Space::new()).width(Length::Fill).height(Length::Fill).style(boxed(page_bg(c(0x2e, 0x17, 0x0b)), 0.0, None)).into();
    }
    responsive(move |size: Size| {
        let k = (size.width / 1920.0).min(size.height / 1080.0).max(0.4);
        let pad_x = if s.switcher { 96.0 } else { 72.0 } * k;
        let body: Element<Message> = if s.switcher {
            switcher(s, k, Size::new(size.width - 2.0 * pad_x, size.height))
        } else {
            let main: Element<Message> = match s.tab.as_str() {
                "library" => library(s, k, Size::new(size.width - 2.0 * pad_x, size.height)),
                "apps" => apps(s, k),
                "store" => store(s, k),
                _ => home(s, k),
            };
            let gap = if s.tab == "library" { 40.0 } else { 46.0 } * k;
            column![top_bar(s, k, s.tab == "library"), Space::new().height(Length::Fixed(gap)), main].into()
        };
        let page = column![container(body).height(Length::Fill), hints(s, k)]
            .padding(Padding { top: if s.switcher { 64.0 } else { 46.0 } * k, right: pad_x, bottom: 30.0 * k, left: pad_x });
        container(page).width(Length::Fill).height(Length::Fill).style(boxed(page_bg(glow), 0.0, None)).into()
    })
    .into()
}

fn subscription(_s: &Couch) -> Subscription<Message> {
    Subscription::batch([Subscription::run(link::stream).map(Message::Hub), iced::keyboard::listen().map(Message::Key)])
}

pub fn run() -> i32 {
    link::set_surface("couch");
    iced_layershell::disable_clipboard();
    let r = iced_layershell::application(
        || Couch {
            st: State::default(),
            lib: Vec::new(),
            apps: Vec::new(),
            tab: "home".into(),
            switcher: false,
            zone: Zone::Hero,
            idx: 0,
            filter: 0,
            search: String::new(),
            layer: None,
            told_page: String::new(),
        },
        || "hearth-firehold-couch".to_string(),
        update,
        view,
    )
    .style(|_s: &Couch, _t: &iced::Theme| iced::theme::Style { background_color: BG, text_color: TEXT })
    .subscription(subscription)
    .settings(Settings {
        layer_settings: LayerShellSettings {
            anchor: Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right,
            layer: Layer::Background,
            exclusive_zone: -1,
            size: Some((0, 0)),
            keyboard_interactivity: KeyboardInteractivity::None,
            start_mode: StartMode::Active,
            ..Default::default()
        },
        default_font: INTER,
        ..Default::default()
    })
    .run();
    super::ended("couch", r)
}
