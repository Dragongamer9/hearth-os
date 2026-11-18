//! `firehold-shell guide`: the Guide over a game (mockup 3), a panel on the right on the overlay layer with the
//! keyboard and the controller, started by the hub when the Guide button is pressed and ended when it's closed. Its
//! game rows are firehold-comp's own (game.rs: frame limit, resolution scaling, VRR, tearing), changed through the hub.
//! `firehold-shell perf`: the performance overlay's line at the top left (only while a game is in front and the
//! overlay is on).

use iced::widget::{column, container, mouse_area, row, text, Space};
use iced::{Color, Element, Length, Padding, Subscription, Task};
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, Settings, StartMode};
use iced_layershell::to_layer_message;
use serde_json::json;

use super::key_nav;
use super::style::*;
use crate::link::{self, HubMsg};
use crate::proto::State;

#[to_layer_message]
#[derive(Debug, Clone)]
pub enum Message {
    Hub(HubMsg),
    Key(iced::keyboard::Event),
    Pick(usize),
    Press(usize),
    Step(usize, bool),
}

#[derive(Default)]
struct Guide {
    st: State,
    sel: usize,
    perf: bool,
}

const TILES: usize = 6;
const ROWS: usize = 6;
const BUTTONS: usize = 3;
const ROW_KEYS: [&str; ROWS] = ["frameLimit", "scaling", "vrr", "tearing", "power", "overlay"];

impl Guide {
    fn step(&mut self, row: usize, up: bool) {
        let dir = if up { "up" } else { "down" };
        match ROW_KEYS[row] {
            "power" => link::send(json!({"op": "guideTile", "what": if up { "powerUp" } else { "powerDown" }})),
            "overlay" => link::send(json!({"op": "overlayStep", "dir": dir})),
            k => link::send(json!({"op": "gameRow", "key": k, "dir": dir})),
        }
    }
    fn press(&mut self, i: usize) {
        match i {
            0 => link::send(json!({"op": "guideTile", "what": "screenshot"})),
            3 => link::send(json!({"op": "guideTile", "what": "keyboard"})),
            4 => link::send(json!({"op": "guideTile", "what": "volumeUp"})),
            5 => link::send(json!({"op": "guideTile", "what": "brightnessUp"})),
            i if (TILES..TILES + ROWS).contains(&i) => self.step(i - TILES, true),
            i if i == TILES + ROWS => link::op("gameToDesk"),
            i if i == TILES + ROWS + 1 => link::op("gameHome"),
            i if i == TILES + ROWS + 2 => link::op("quitGame"),
            _ => {}
        }
    }
    fn nav(&mut self, key: &str) {
        let n = TILES + ROWS + BUTTONS;
        let tile = self.sel < TILES;
        let rowsel = (TILES..TILES + ROWS).contains(&self.sel);
        match key {
            "left" if tile && self.sel % 3 > 0 => self.sel -= 1,
            "right" if tile && self.sel % 3 < 2 => self.sel += 1,
            "left" | "right" if rowsel => self.step(self.sel - TILES, key == "right"),
            // Volume and brightness: the triggers (or Home/End) lower and raise them on their tiles; A raises.
            "lt" | "rt" | "lb" | "rb" if self.sel == 4 || self.sel == 5 => {
                let up = key == "rt" || key == "rb";
                let what = match (self.sel, up) {
                    (4, true) => "volumeUp",
                    (4, false) => "volumeDown",
                    (_, true) => "brightnessUp",
                    _ => "brightnessDown",
                };
                link::send(json!({"op": "guideTile", "what": what}));
            }
            "up" if tile && self.sel >= 3 => self.sel -= 3,
            "up" if !tile && self.sel == TILES => self.sel = 3,
            "up" if self.sel > TILES => self.sel -= 1,
            "down" if tile && self.sel < 3 => self.sel += 3,
            "down" if tile => self.sel = TILES,
            "down" if self.sel + 1 < n => self.sel += 1,
            "a" => {
                let i = self.sel;
                self.press(i)
            }
            "b" => link::op("guideClose"),
            _ => {}
        }
    }
}

fn update(s: &mut Guide, m: Message) -> Task<Message> {
    match m {
        Message::Hub(HubMsg::State(st)) => {
            s.st = *st;
            if !s.perf && !s.st.guide.up {
                return iced::exit();
            }
        }
        Message::Hub(HubMsg::Nav(k)) => s.nav(&k),
        Message::Hub(HubMsg::Quit) | Message::Hub(HubMsg::Gone) => return iced::exit(),
        Message::Hub(_) => {}
        Message::Key(ev) => {
            if let Some(k) = key_nav(&ev) {
                s.nav(k);
            }
        }
        Message::Pick(i) => s.sel = i,
        Message::Press(i) => {
            s.sel = i;
            s.press(i);
        }
        Message::Step(r, up) => {
            s.sel = TILES + r;
            s.step(r, up);
        }
        _ => {}
    }
    Task::none()
}

fn chip<'a>(value: String, colour: Color, on: bool, r: usize) -> Element<'a, Message> {
    let arrow = |up: bool| mouse_area(t(if up { "›" } else { "‹" }, 16.0, colour)).on_press(Message::Step(r, up));
    container(row![arrow(false), tb(value, 15.0, colour), arrow(true)].spacing(6.0).align_y(iced::Center))
        .padding(Padding { top: 6.0, right: 12.0, bottom: 6.0, left: 12.0 })
        .style(boxed(c(0x2a, 0x20, 0x1a), 8.0, on.then_some((ACCENT, 2.0))))
        .into()
}

fn guide_view(s: &Guide) -> Element<'_, Message> {
    let g = &s.st.guide;
    let name = if g.game.is_empty() { "Quick Access".to_string() } else { g.game.clone() };
    let sub = if g.game.is_empty() { "No game is running".to_string() } else { format!("{} · native, no Game Mode", g.playing) };
    let art = container(Space::new()).width(Length::Fixed(56.0)).height(Length::Fixed(56.0)).style(boxed(hero_bg(), 10.0, None));
    let head = row![art, column![tb(name, 22.0, TEXT), t(sub, 15.0, MUTED)].spacing(4.0)].spacing(16.0).align_y(iced::Center);

    let pct = |v: Option<i64>| v.map(|x| format!("{x}%")).unwrap_or_else(|| "Not here".into());
    let tiles: [(&str, String); TILES] = [
        ("Screenshot", if g.screenshot.is_empty() { "Saved to Pictures".into() } else { "Saved to Pictures ✓".into() }),
        ("Record", "Not yet".into()),
        ("Friends", "Not yet".into()),
        ("Keyboard", "On-screen".into()),
        ("Volume", pct(g.volume)),
        ("Brightness", pct(g.brightness)),
    ];
    let mut grid = column![].spacing(10.0);
    for r in 0..2 {
        let mut line = row![].spacing(10.0);
        for cix in 0..3 {
            let i = r * 3 + cix;
            let on = s.sel == i;
            let (title, sub) = (&tiles[i].0, tiles[i].1.clone());
            let cell = container(column![ts(*title, 16.0, TEXT), t(sub, 13.0, MUTED)].spacing(6.0).align_x(iced::Center))
                .center_x(Length::Fill)
                .center_y(Length::Fixed(78.0))
                .style(boxed(c(0x22, 0x1a, 0x16), 12.0, Some(if on { (ACCENT, 2.0) } else { (LINE, 1.0) })));
            line = line.push(mouse_area(cell).on_press(Message::Press(i)).on_enter(Message::Pick(i)));
        }
        grid = grid.push(line);
    }

    let none = |x: &str| if g.game.is_empty() { "No game".to_string() } else { x.to_string() };
    let rows: [(&str, &str, String, Color); ROWS] = [
        ("Frame limit", "Steadier, cooler, longer battery", none(&g.frame_limit), TEXT),
        ("Resolution scaling", if g.scaling_note.is_empty() { "Firehold upscales the game itself" } else { "Firehold upscales the game itself (not this one yet)" }, none(&g.scaling), TEXT),
        ("VRR / FreeSync", "Smooth when it dips below 60", none(if g.vrr { "On" } else { "Off" }), if g.vrr && !g.game.is_empty() { GREEN } else { TEXT }),
        ("Allow tearing", "Lowest delay for fast games", none(if g.tearing { "On" } else { "Off" }), TEXT),
        ("Power", "Handhelds and laptops", g.power.clone(), TEXT),
        ("Performance overlay", "Top-left corner", match s.st.cfg.overlay.as_str() { "basic" => "Basic".into(), "full" => "Full".into(), _ => "Off".into() }, TEXT),
    ];
    let mut list = column![];
    for (r, (title, sub, value, colour)) in rows.into_iter().enumerate() {
        let i = TILES + r;
        let on = s.sel == i;
        let words = column![ts(title, 17.0, TEXT), t(sub, 14.0, MUTED)].spacing(3.0);
        let line = container(row![words, Space::new().width(Length::Fill), chip(value, colour, on, r)].align_y(iced::Center))
            .padding(Padding { top: 0.0, right: 18.0, bottom: 0.0, left: 18.0 })
            .center_y(Length::Fixed(71.0))
            .width(Length::Fill);
        list = list.push(mouse_area(line).on_enter(Message::Pick(i)));
        if r + 1 < ROWS {
            list = list.push(container(Space::new()).width(Length::Fill).height(Length::Fixed(1.0)).style(boxed(LINE, 0.0, None)));
        }
    }
    let list = container(list).style(boxed(c(0x1f, 0x17, 0x14), 12.0, None));

    let mut buttons = column![].spacing(10.0);
    for (b, (label, danger)) in [("Put it in a window on the Desk", false), ("Home (the game keeps running)", false), ("Quit game", true)].into_iter().enumerate() {
        let i = TILES + ROWS + b;
        let on = s.sel == i;
        let (bg, fg, edge) = if danger { (c(0x1a, 0x12, 0x10), RED, Some((RED, 2.0))) } else { (c(0x2a, 0x20, 0x1c), TEXT, None) };
        let edge = if on { Some((ACCENT, 3.0)) } else { edge };
        let cell = container(ts(label, 17.0, fg)).center_x(Length::Fill).center_y(Length::Fixed(54.0)).style(boxed(bg, 10.0, edge));
        buttons = buttons.push(mouse_area(cell).on_press(Message::Press(i)).on_enter(Message::Pick(i)));
    }

    let body = column![head, Space::new().height(Length::Fixed(20.0)), grid, Space::new().height(Length::Fixed(20.0)), list, Space::new().height(Length::Fill), buttons];
    container(body)
        .padding(Padding { top: 32.0, right: 32.0, bottom: 32.0, left: 32.0 })
        .width(Length::Fill)
        .height(Length::Fill)
        .style(boxed(c(0x1c, 0x15, 0x12), 0.0, None))
        .into()
}

fn perf_view(s: &Guide) -> Element<'_, Message> {
    let line = if s.st.perf.line.is_empty() { "Firehold".to_string() } else { s.st.perf.line.clone() };
    let chip = container(text(line).size(15.0).font(MONO).color(GREEN))
        .padding(Padding { top: 6.0, right: 14.0, bottom: 6.0, left: 14.0 })
        .style(boxed(Color { a: 0.85, ..c(0x12, 0x0c, 0x09) }, 8.0, None));
    container(chip).width(Length::Fill).height(Length::Fill).into()
}

fn view(s: &Guide) -> Element<'_, Message> {
    if s.perf { perf_view(s) } else { guide_view(s) }
}

fn subscription(s: &Guide) -> Subscription<Message> {
    if s.perf {
        Subscription::run(link::stream).map(Message::Hub)
    } else {
        Subscription::batch([Subscription::run(link::stream).map(Message::Hub), iced::keyboard::listen().map(Message::Key)])
    }
}

pub fn run(perf: bool) -> i32 {
    let name = if perf { "perf" } else { "guide" };
    link::set_surface(name);
    iced_layershell::disable_clipboard();
    let layer_settings = if perf {
        LayerShellSettings {
            anchor: Anchor::Top | Anchor::Left,
            layer: Layer::Overlay,
            exclusive_zone: 0,
            size: Some((560, 36)),
            margin: (28, 0, 0, 32),
            keyboard_interactivity: KeyboardInteractivity::None,
            events_transparent: true,
            start_mode: StartMode::Active,
        }
    } else {
        LayerShellSettings {
            anchor: Anchor::Top | Anchor::Bottom | Anchor::Right,
            layer: Layer::Overlay,
            exclusive_zone: -1,
            size: Some((560, 0)),
            keyboard_interactivity: KeyboardInteractivity::Exclusive,
            start_mode: StartMode::Active,
            ..Default::default()
        }
    };
    let r = iced_layershell::application(move || Guide { perf, ..Guide::default() }, move || format!("hearth-firehold-{name}"), update, view)
        .style(|_s: &Guide, _t: &iced::Theme| iced::theme::Style { background_color: Color::TRANSPARENT, text_color: TEXT })
        .subscription(subscription)
        .settings(Settings { layer_settings, default_font: INTER, ..Default::default() })
        .run();
    super::ended(name, r)
}
