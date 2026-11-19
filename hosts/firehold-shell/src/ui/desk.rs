//! The Desk's own parts (mockup 2): `firehold-shell bar` (the top bar: the focused app and its menus, the date and the
//! clock in the middle, the posture pill, Wi-Fi and the battery), `firehold-shell dock` (Home, every open app with
//! its running dot, the Library and Settings) and `firehold-shell notice` (the card "Switched to Desk", and the apps'
//! notifications under it). The windows' title bars are drawn by firehold-comp itself, named by the hub (frames.rs).
//! File, Edit and View open their menus (`firehold-shell menu`, menu.rs), the pill opens Couch / Desk / Away, and
//! Wi-Fi and the battery open Quick Settings (quick.rs).

use iced::widget::{column, container, mouse_area, row, stack, text, Space};
use iced::{border, Color, Element, Length, Padding, Subscription, Task};
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, Settings, StartMode};
use iced_layershell::to_layer_message;
use serde_json::json;

use super::style::*;
use crate::link::{self, HubMsg};
use crate::model;
use crate::proto::State;

#[to_layer_message]
#[derive(Debug, Clone)]
pub enum Message {
    Hub(HubMsg),
    Click(String),
    /// A menu's title in the bar, and where it starts (from the screen's left edge).
    Menu(String, i32),
    /// A notification's card: (id, its button's key, or "default" for the card, "dismiss" for its ×).
    Note(u32, String),
}

#[derive(Default)]
struct Desk {
    kind: &'static str,
    st: State,
    width: u32,
}

/// What the dock shows: (key, letter, tint, running).
fn dock_items(st: &State) -> Vec<(String, String, Color, bool)> {
    let mut v = vec![("home".to_string(), "H".to_string(), c_accent(), false)];
    let mut seen = std::collections::HashSet::new();
    for w in &st.wins {
        let key = if w.game { format!("game:{}", w.game_id) } else { w.app_id.clone() };
        if !seen.insert(key) {
            continue;
        }
        let tint = if w.game {
            c(0x5a, 0x2c, 0x1c)
        } else {
            match w.name.as_str() {
                "Discord" => c(0x2c, 0x2a, 0x3e),
                "Steam" => c(0x1f, 0x2a, 0x36),
                _ => shade(0.88),
            }
        };
        v.push((format!("win:{}", w.id), model::dock_letter(&w.name, w.game), tint, true));
    }
    v.push(("library".into(), "L".into(), shade(0.88), false));
    v.push(("settings".into(), "⚙".into(), shade(0.88), false));
    v
}

const ITEM: f32 = 56.0;
/// The dock's and the notice's surfaces are exactly their panels (mockup 2: 88 tall each): a software-drawn surface
/// has no see-through parts, so space around a panel showed as a black band.
const DOCK_H: u32 = 88;
const NOTICE_H: u32 = 88;
const GAP: f32 = 10.0;
const PAD: f32 = 14.0;
/// A notification card's height (taller with the app's buttons), and the space between cards.
const NOTE_H: u32 = 96;
const NOTE_BTN_H: u32 = 40;
const NOTE_GAP: u32 = 10;

fn notice_height(st: &State) -> u32 {
    let mut h = 0;
    let mut n = 0;
    if st.notice.is_some() {
        h += NOTICE_H;
        n += 1;
    }
    for note in &st.notes {
        h += NOTE_H + if note.actions.iter().any(|(k, _)| k != "default") { NOTE_BTN_H } else { 0 };
        n += 1;
    }
    h + NOTE_GAP * n.saturating_sub(1) as u32
}

/// About how wide Inter draws a word (for where a menu opens under its title): 0.6 em a letter bold, 0.55 regular.
fn text_w(s: &str, size: f32, bold: bool) -> f32 {
    s.chars().count() as f32 * size * if bold { 0.6 } else { 0.55 }
}

/// Where each menu title starts in the bar: the bar's padding, the app tile, the app's name, then each title.
pub fn menu_xs(front: &str) -> Vec<(String, i32)> {
    let mut x = 16.0 + 32.0 + 16.0 + text_w(front, 15.0, true) + 16.0;
    let mut v = Vec::new();
    for m in ["File", "Edit", "View"] {
        v.push((m.to_string(), (x - 10.0).round() as i32));
        x += text_w(m, 14.0, false) + 16.0;
    }
    v
}

fn dock_width(n: usize) -> u32 {
    (n as f32 * ITEM + (n.saturating_sub(1)) as f32 * GAP + 2.0 * PAD + 2.0).ceil() as u32
}

fn update(s: &mut Desk, m: Message) -> Task<Message> {
    match m {
        Message::Hub(HubMsg::State(st)) => {
            s.st = *st;
            if s.kind == "notice" && s.st.notice.is_none() && s.st.notes.is_empty() {
                return iced::exit();
            }
            if s.kind == "notice" {
                let h = notice_height(&s.st).max(40);
                if h != s.width {
                    s.width = h;
                    return Task::done(Message::SizeChange((490, h)));
                }
            }
            if s.kind == "dock" {
                let w = dock_width(dock_items(&s.st).len());
                if w != s.width {
                    s.width = w;
                    return Task::done(Message::SizeChange((w, DOCK_H)));
                }
            }
        }
        Message::Hub(HubMsg::Quit) | Message::Hub(HubMsg::Gone) => return iced::exit(),
        Message::Hub(_) => {}
        Message::Menu(name, x) => link::send(json!({"op": "menu", "name": name, "x": x, "output": link::output()})),
        Message::Note(id, key) => link::send(json!({"op": "noteAct", "id": id, "key": key})),
        Message::Click(k) => match k.as_str() {
            "home" => link::op("home"),
            "posture" => link::send(json!({"op": "menu", "name": "posture", "x": -1, "output": link::output()})),
            "quick" => link::send(json!({"op": "quick", "output": link::output()})),
            "library" => link::send(json!({"op": "tab", "tab": "library"})),
            "settings" => link::send(json!({"op": "page", "page": "settings"})),
            "notice" => link::op("home"),
            k => {
                if let Some(id) = k.strip_prefix("win:") {
                    link::send(json!({"op": "activate", "id": id}));
                }
            }
        },
        _ => {}
    }
    Task::none()
}

fn bar(s: &Desk) -> Element<'_, Message> {
    let st = &s.st;
    let front = st.wins.iter().find(|w| w.id == st.active).map(|w| w.name.clone()).unwrap_or_else(|| "Firehold".into());
    let tile = mouse_area(container(Space::new()).width(Length::Fixed(32.0)).height(Length::Fixed(32.0)).style(boxed(c_accent(), 8.0, None))).on_press(Message::Click("home".into()));
    let xs = menu_xs(&front);
    let mut left = row![tile, tb(front, 15.0, c_text())].spacing(16.0).align_y(iced::Center);
    for (m, x) in xs {
        let open = st.menu.open == m && st.menu.output == link::output();
        let label = container(t(m.clone(), 14.0, if open { c_text() } else { c_muted() }))
            .padding(Padding { top: 4.0, right: 0.0, bottom: 4.0, left: 0.0 })
            .style(boxed(Color::TRANSPARENT, 6.0, None));
        left = left.push(mouse_area(label).on_press(Message::Menu(m, x)));
    }
    let dot = container(Space::new().width(Length::Fixed(8.0)).height(Length::Fixed(8.0))).style(boxed(BLUE, 4.0, None));
    let pill = mouse_area(
        container(row![dot, ts("Desk", 14.0, c_text())].spacing(8.0).align_y(iced::Center))
            .padding(Padding { top: 5.0, right: 12.0, bottom: 5.0, left: 12.0 })
            .style(boxed(c_card(), 14.0, Some((c_line(), 1.0)))),
    )
    .on_press(Message::Click("posture".into()));
    let status = [st.net.clone(), st.battery.clone()].into_iter().filter(|x| !x.is_empty()).collect::<Vec<_>>().join(" · ");
    let status = mouse_area(container(t(status, 14.0, if st.quick.up { c_text() } else { c_muted() })).padding(Padding { top: 6.0, right: 0.0, bottom: 6.0, left: 4.0 }))
        .on_press(Message::Click("quick".into()));
    let right = row![pill, status].spacing(14.0).align_y(iced::Center);
    let ends = row![left.width(Length::Shrink), Space::new().width(Length::Fill), right].align_y(iced::Center).padding(Padding { top: 0.0, right: 16.0, bottom: 0.0, left: 16.0 });
    let clock = container(ts(format!("{} · {}", st.date, st.clock), 15.0, c_text())).center(Length::Fill);
    let line = container(Space::new()).width(Length::Fill).height(Length::Fixed(1.0)).style(boxed(c_line(), 0.0, None));
    column![container(stack![container(ends).center_y(Length::Fill), clock]).height(Length::Fixed(43.0)).style(boxed(c_panel(), 0.0, None)), line].into()
}

fn dock(s: &Desk) -> Element<'_, Message> {
    let mut r = row![].spacing(GAP);
    for (key, letter, tint, running) in dock_items(&s.st) {
        let home = key == "home";
        let cell = container(text(letter).size(20.0).font(bold()).color(if home { c_ink() } else { c_text() }))
            .center_x(Length::Fixed(ITEM))
            .center_y(Length::Fixed(ITEM))
            .style(boxed(tint, 12.0, None));
        let dot: Element<Message> = if running {
            container(Space::new().width(Length::Fixed(6.0)).height(Length::Fixed(6.0))).style(boxed(c_accent(), 3.0, None)).into()
        } else {
            Space::new().width(Length::Fixed(6.0)).height(Length::Fixed(6.0)).into()
        };
        let item = column![cell, container(dot).center_x(Length::Fixed(ITEM))].spacing(4.0);
        r = r.push(mouse_area(item).on_press(Message::Click(key)));
    }
    container(r)
        .padding(Padding { top: PAD, right: PAD, bottom: 8.0, left: PAD })
        .width(Length::Fill)
        .height(Length::Fill)
        .style(boxed(shade(0.30), 16.0, Some((c_line(), 1.0))))
        .into()
}

fn card<'a>(icon_c: Color, title: String, body: String, small: String, buttons: Vec<(String, String)>, id: Option<u32>, h: u32) -> Element<'a, Message> {
    let icon = container(Space::new()).width(Length::Fixed(40.0)).height(Length::Fixed(40.0)).style(boxed(icon_c, 8.0, None));
    let mut words = column![].spacing(3.0).width(Length::Fill);
    if !small.is_empty() {
        words = words.push(t(small, 12.0, c_muted()));
    }
    words = words.push(tb(title, 16.0, c_text()));
    if !body.is_empty() {
        words = words.push(t(body, 14.0, soft(0.43)));
    }
    let mut body_row = row![icon, words].spacing(14.0).align_y(iced::Center);
    if let Some(id) = id {
        let x = mouse_area(container(t("×", 18.0, c_muted())).padding(Padding { top: 0.0, right: 4.0, bottom: 0.0, left: 8.0 })).on_press(Message::Note(id, "dismiss".into()));
        body_row = body_row.push(x);
    }
    let mut inner = column![body_row].spacing(10.0);
    let btns: Vec<(String, String)> = buttons.into_iter().filter(|(k, _)| k != "default").collect();
    if let (Some(id), false) = (id, btns.is_empty()) {
        let mut r = row![Space::new().width(Length::Fixed(54.0))].spacing(8.0);
        for (k, label) in btns {
            r = r.push(mouse_area(container(ts(label, 13.0, c_text())).padding(Padding { top: 6.0, right: 12.0, bottom: 6.0, left: 12.0 }).style(boxed(shade(0.76), 8.0, None))).on_press(Message::Note(id, k)));
        }
        inner = inner.push(r);
    }
    let radius = border::Radius::new(12.0).top_right(0.0).bottom_right(0.0);
    let c = container(inner)
        .padding(Padding { top: 0.0, right: 20.0, bottom: 0.0, left: 18.0 })
        .width(Length::Fill)
        .height(Length::Fixed(h as f32))
        .center_y(Length::Fixed(h as f32))
        .style(move |_| container::Style {
            background: Some(shade(0.39).into()),
            border: iced::Border { color: c_line(), width: 1.0, radius },
            ..container::Style::default()
        });
    match id {
        Some(id) => mouse_area(c).on_press(Message::Note(id, "default".into())).into(),
        None => mouse_area(c).on_press(Message::Click("notice".into())).into(),
    }
}

fn notice(s: &Desk) -> Element<'_, Message> {
    let mut col = column![].spacing(NOTE_GAP as f32);
    if let Some(n) = s.st.notice.as_ref() {
        col = col.push(card(BLUE, n.title.clone(), n.body.clone(), String::new(), Vec::new(), None, NOTICE_H));
    }
    for n in &s.st.notes {
        let h = NOTE_H + if n.actions.iter().any(|(k, _)| k != "default") { NOTE_BTN_H } else { 0 };
        let small = [n.app.clone(), n.at.clone()].into_iter().filter(|x| !x.is_empty()).collect::<Vec<_>>().join(" · ");
        col = col.push(card(if n.urgent { c_danger() } else { c_accent() }, n.title.clone(), n.body.clone(), small, n.actions.clone(), Some(n.id), h));
    }
    col.into()
}

fn view(s: &Desk) -> Element<'_, Message> {
    match s.kind {
        "bar" => bar(s),
        "dock" => dock(s),
        _ => notice(s),
    }
}

fn subscription(_s: &Desk) -> Subscription<Message> {
    Subscription::run(link::stream).map(Message::Hub)
}

pub fn run(kind: &'static str) -> i32 {
    link::set_surface(kind);
    let start_mode = if kind == "notice" { StartMode::Active } else { link::start_mode() };
    iced_layershell::disable_clipboard();
    let layer_settings = match kind {
        "bar" => LayerShellSettings {
            anchor: Anchor::Top | Anchor::Left | Anchor::Right,
            layer: Layer::Top,
            exclusive_zone: 44,
            size: Some((0, 44)),
            keyboard_interactivity: KeyboardInteractivity::None,
            start_mode: start_mode.clone(),
            ..Default::default()
        },
        "dock" => LayerShellSettings {
            anchor: Anchor::Bottom,
            layer: Layer::Top,
            exclusive_zone: 0,
            size: Some((dock_width(3), DOCK_H)),
            margin: (0, 0, 22, 0),
            keyboard_interactivity: KeyboardInteractivity::None,
            start_mode: start_mode.clone(),
            ..Default::default()
        },
        _ => LayerShellSettings {
            anchor: Anchor::Top | Anchor::Right,
            layer: Layer::Overlay,
            exclusive_zone: 0,
            size: Some((490, NOTICE_H)),
            margin: (64, 0, 0, 0),
            keyboard_interactivity: KeyboardInteractivity::None,
            start_mode,
            ..Default::default()
        },
    };
    let r = iced_layershell::application(
        move || Desk { kind, ..Desk::default() },
        move || format!("hearth-firehold-{kind}"),
        update,
        view,
    )
    .style(|_s: &Desk, _t: &iced::Theme| iced::theme::Style { background_color: Color::TRANSPARENT, text_color: c_text() })
    .subscription(subscription)
    .settings(Settings { layer_settings, default_font: INTER, ..Default::default() })
    .run();
    super::ended(kind, r)
}
