//! `firehold-shell menu`: a menu open from the Desk bar (mockup 2's File, Edit and View, and the posture pill's
//! Couch / Desk / Away), right under its title. The surface is exactly the menu (a software-drawn surface has no
//! see-through parts) and never takes the keyboard, so the app in front keeps it: what's picked is sent to that app as
//! its own shortcut, or done to its window, by the hub (model::app_menu). A click on the title again, another app in
//! front, or a pick closes it.

use iced::widget::{column, container, mouse_area, row, Space};
use iced::{Color, Element, Length, Padding, Subscription, Task};
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, Settings};
use iced_layershell::to_layer_message;
use serde_json::json;

use super::style::*;
use crate::link::{self, HubMsg};
use crate::proto::State;

#[to_layer_message]
#[derive(Debug, Clone)]
pub enum Message {
    Hub(HubMsg),
    Pick(String),
    Hover(usize),
}

#[derive(Default)]
struct M {
    st: State,
    hover: Option<usize>,
    placed: (u32, u32, i32),
}

pub const W: u32 = 300;
const ITEM_H: f32 = 34.0;
const SEP_H: f32 = 9.0;
const PAD: f32 = 6.0;

fn height(st: &State) -> u32 {
    let seps = st.menu.items.iter().enumerate().filter(|(i, it)| it.sep && *i > 0).count() as f32;
    (st.menu.items.len() as f32 * ITEM_H + seps * SEP_H + 2.0 * PAD + 2.0).ceil() as u32
}

fn update(s: &mut M, m: Message) -> Task<Message> {
    match m {
        Message::Hub(HubMsg::State(st)) => {
            s.st = *st;
            if s.st.menu.open.is_empty() {
                return iced::exit();
            }
            let h = height(&s.st);
            let x = s.st.menu.x;
            if (W, h, x) != s.placed {
                s.placed = (W, h, x);
                s.hover = None;
                // Under its title: from the left (File, Edit, View), or from the right (the posture pill, x < 0).
                let (anchor, margin) = if x < 0 { (Anchor::Top | Anchor::Right, (50, 120, 0, 0)) } else { (Anchor::Top | Anchor::Left, (46, 0, 0, x.max(0))) };
                return Task::batch([Task::done(Message::AnchorSizeChange(anchor, (W, h))), Task::done(Message::MarginChange(margin))]);
            }
        }
        Message::Hub(HubMsg::Quit) | Message::Hub(HubMsg::Gone) => return iced::exit(),
        Message::Hub(_) => {}
        Message::Pick(id) => link::send(json!({"op": "menuPick", "id": id})),
        Message::Hover(i) => s.hover = Some(i),
        _ => {}
    }
    Task::none()
}

fn view(s: &M) -> Element<'_, Message> {
    let mut col = column![].spacing(0.0);
    for (i, it) in s.st.menu.items.iter().enumerate() {
        if it.sep && i > 0 {
            col = col.push(container(container(Space::new()).width(Length::Fill).height(Length::Fixed(1.0)).style(boxed(c_line(), 0.0, None))).padding(Padding { top: 4.0, right: 8.0, bottom: 4.0, left: 8.0 }));
        }
        let hot = s.hover == Some(i);
        let mark = if it.on { "•" } else { "" };
        let line = row![
            container(t(mark, 14.0, c_accent())).width(Length::Fixed(14.0)),
            t(it.label.clone(), 14.0, c_text()),
            Space::new().width(Length::Fill),
            t(it.keys.clone(), 12.0, c_muted()),
        ]
        .spacing(6.0)
        .align_y(iced::Center);
        let cell = container(line)
            .padding(Padding { top: 0.0, right: 12.0, bottom: 0.0, left: 8.0 })
            .width(Length::Fill)
            .center_y(Length::Fixed(ITEM_H))
            .style(boxed(if hot { shade(0.85) } else { Color::TRANSPARENT }, 7.0, None));
        col = col.push(mouse_area(cell).on_press(Message::Pick(it.id.clone())).on_enter(Message::Hover(i)));
    }
    container(col).padding(PAD).width(Length::Fill).height(Length::Fill).style(boxed(shade(0.45), 10.0, Some((c_line(), 1.0)))).into()
}

fn subscription(_s: &M) -> Subscription<Message> {
    Subscription::run(link::stream).map(Message::Hub)
}

pub fn run() -> i32 {
    link::set_surface("menu");
    iced_layershell::disable_clipboard();
    let layer_settings = LayerShellSettings {
        anchor: Anchor::Top | Anchor::Left,
        layer: Layer::Overlay,
        exclusive_zone: 0,
        size: Some((W, 120)),
        margin: (46, 0, 0, 120),
        keyboard_interactivity: KeyboardInteractivity::None,
        start_mode: link::start_mode(),
        ..Default::default()
    };
    let r = iced_layershell::application(M::default, || "hearth-firehold-menu".to_string(), update, view)
        .style(|_s: &M, _t: &iced::Theme| iced::theme::Style { background_color: Color::TRANSPARENT, text_color: c_text() })
        .subscription(subscription)
        .settings(Settings { layer_settings, default_font: INTER, ..Default::default() })
        .run();
    super::ended("menu", r)
}
