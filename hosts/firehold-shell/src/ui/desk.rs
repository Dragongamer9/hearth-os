//! The Desk's own parts (mockup 2): `firehold-shell bar` (the top bar: the focused app and its menus, the date and the
//! clock in the middle, the posture pill, Wi-Fi and the battery), `firehold-shell dock` (Home, every open app with
//! its running dot, the Library and Settings) and `firehold-shell notice` (the card "Switched to Desk"). The windows'
//! title bars are drawn by firehold-comp itself, named by the hub (frames.rs).

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

fn dock_width(n: usize) -> u32 {
    (n as f32 * ITEM + (n.saturating_sub(1)) as f32 * GAP + 2.0 * PAD + 2.0).ceil() as u32
}

fn update(s: &mut Desk, m: Message) -> Task<Message> {
    match m {
        Message::Hub(HubMsg::State(st)) => {
            s.st = *st;
            if s.kind == "notice" && s.st.notice.is_none() {
                return iced::exit();
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
        Message::Click(k) => match k.as_str() {
            "home" | "posture" => link::op("home"),
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
    let mut left = row![tile, tb(front, 15.0, c_text())].spacing(16.0).align_y(iced::Center);
    for m in ["File", "Edit", "View"] {
        left = left.push(t(m, 14.0, c_muted()));
    }
    let dot = container(Space::new().width(Length::Fixed(8.0)).height(Length::Fixed(8.0))).style(boxed(BLUE, 4.0, None));
    let pill = mouse_area(
        container(row![dot, ts("Desk", 14.0, c_text())].spacing(8.0).align_y(iced::Center))
            .padding(Padding { top: 5.0, right: 12.0, bottom: 5.0, left: 12.0 })
            .style(boxed(c_card(), 14.0, Some((c_line(), 1.0)))),
    )
    .on_press(Message::Click("posture".into()));
    let status = [st.net.clone(), st.battery.clone()].into_iter().filter(|x| !x.is_empty()).collect::<Vec<_>>().join(" · ");
    let right = row![pill, t(status, 14.0, c_muted())].spacing(14.0).align_y(iced::Center);
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

fn notice(s: &Desk) -> Element<'_, Message> {
    let Some(n) = s.st.notice.as_ref() else { return Space::new().into() };
    let icon = container(Space::new()).width(Length::Fixed(40.0)).height(Length::Fixed(40.0)).style(boxed(BLUE, 8.0, None));
    let words = column![tb(n.title.clone(), 16.0, c_text()), t(n.body.clone(), 14.0, soft(0.43))].spacing(4.0).width(Length::Fill);
    let radius = border::Radius::new(12.0).top_right(0.0).bottom_right(0.0);
    let card = container(row![icon, words].spacing(14.0).align_y(iced::Center))
        .padding(Padding { top: 0.0, right: 20.0, bottom: 0.0, left: 18.0 })
        .width(Length::Fill)
        .height(Length::Fill)
        .center_y(Length::Fill)
        .style(move |_| container::Style {
            background: Some(shade(0.39).into()),
            border: iced::Border { color: c_line(), width: 1.0, radius },
            ..container::Style::default()
        });
    mouse_area(card).on_press(Message::Click("notice".into())).into()
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
    iced_layershell::disable_clipboard();
    let layer_settings = match kind {
        "bar" => LayerShellSettings {
            anchor: Anchor::Top | Anchor::Left | Anchor::Right,
            layer: Layer::Top,
            exclusive_zone: 44,
            size: Some((0, 44)),
            keyboard_interactivity: KeyboardInteractivity::None,
            start_mode: StartMode::Active,
            ..Default::default()
        },
        "dock" => LayerShellSettings {
            anchor: Anchor::Bottom,
            layer: Layer::Top,
            exclusive_zone: 0,
            size: Some((dock_width(3), DOCK_H)),
            margin: (0, 0, 22, 0),
            keyboard_interactivity: KeyboardInteractivity::None,
            start_mode: StartMode::Active,
            ..Default::default()
        },
        _ => LayerShellSettings {
            anchor: Anchor::Top | Anchor::Right,
            layer: Layer::Overlay,
            exclusive_zone: 0,
            size: Some((490, NOTICE_H)),
            margin: (64, 0, 0, 0),
            keyboard_interactivity: KeyboardInteractivity::None,
            start_mode: StartMode::Active,
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
