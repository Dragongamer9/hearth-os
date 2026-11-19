//! `firehold-shell away`: the Away screen (Firehold's third posture: you're away, the home server keeps running). It
//! covers every screen (one a screen) with a calm page: the time, what this PC is doing for your other devices (Hearth's
//! home server: shared folders, backups, the AI services, Use this PC from anywhere, anything that needs a look, from
//! `hearth-home-server quick`, read once a minute while it's up), and what was paused for a game, with a way to let it
//! run. Nothing you had open is closed, and nothing moves on it (no animation): any button, the mouse, a key or a click
//! brings back what you were in.

use iced::widget::{canvas, column, container, mouse_area, row, Space};
use iced::{Color, Element, Length, Padding, Subscription, Task};
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, Settings};
use iced_layershell::to_layer_message;
use serde_json::json;

use super::art::Flame;
use super::style::*;
use crate::link::{self, HubMsg};
use crate::proto::State;

#[to_layer_message]
#[derive(Debug, Clone)]
pub enum Message {
    Hub(HubMsg),
    Key(iced::keyboard::Event),
    Back,
    Resume,
}

#[derive(Default)]
struct A {
    st: State,
}

fn update(s: &mut A, m: Message) -> Task<Message> {
    match m {
        Message::Hub(HubMsg::State(st)) => {
            s.st = *st;
            if s.st.posture != "away" {
                return iced::exit();
            }
        }
        Message::Hub(HubMsg::Quit) | Message::Hub(HubMsg::Gone) => return iced::exit(),
        Message::Hub(_) => {}
        Message::Key(iced::keyboard::Event::KeyPressed { .. }) | Message::Back => link::send(json!({"op": "posture", "to": "back"})),
        Message::Resume => link::op("awayResume"),
        _ => {}
    }
    Task::none()
}

fn view(s: &A) -> Element<'_, Message> {
    let a = &s.st.away;
    let logo = container(canvas(Flame(c_ink())).width(Length::Fixed(22.0)).height(Length::Fixed(22.0))).center_x(Length::Fixed(38.0)).center_y(Length::Fixed(38.0)).style(boxed(c_accent(), 9.0, None));
    let top = row![logo, tb("Away", 22.0, c_text()), t(if a.host.is_empty() { String::new() } else { format!("· {}", a.host) }, 20.0, c_muted())].spacing(14.0).align_y(iced::Center);
    let clock = column![tb(s.st.clock.clone(), 120.0, c_text()), t(s.st.date.clone(), 26.0, c_muted())].spacing(4.0).align_x(iced::Center);
    let lead = t("This PC keeps your home server running for your other devices. Everything you had open is still open.", 20.0, soft(0.6));
    let mut cards = column![].spacing(14.0);
    if !a.note.is_empty() {
        cards = cards.push(container(t(a.note.clone(), 18.0, c_muted())).center_x(Length::Fill));
    }
    for pair in a.lines.chunks(2) {
        let mut r = row![].spacing(14.0);
        for (what, how, lit) in pair {
            let dot = container(Space::new().width(Length::Fixed(10.0)).height(Length::Fixed(10.0))).style(boxed(if *lit { c_good() } else { c_flame() }, 5.0, None));
            let card = container(row![dot, column![ts(what.clone(), 18.0, c_text()), t(how.clone(), 15.0, c_muted())].spacing(4.0)].spacing(14.0).align_y(iced::Center))
                .padding(Padding { top: 0.0, right: 22.0, bottom: 0.0, left: 22.0 })
                .width(Length::FillPortion(1))
                .center_y(Length::Fixed(82.0))
                .style(boxed(shade(0.38), 14.0, Some((c_line(), 1.0))));
            r = r.push(card);
        }
        if pair.len() == 1 {
            r = r.push(Space::new().width(Length::FillPortion(1)));
        }
        cards = cards.push(r);
    }
    if !a.paused.is_empty() {
        let what = a.paused.join(", ");
        let line: Element<Message> = if a.resumed {
            t(format!("Running now, though a game is open: {what} (for up to 4 hours)"), 16.0, c_muted()).into()
        } else {
            row![
                t(format!("Paused for your game: {what}"), 16.0, c_muted()),
                mouse_area(container(ts("Let them run now", 15.0, c_ink())).padding(Padding { top: 9.0, right: 16.0, bottom: 9.0, left: 16.0 }).style(boxed(c_accent(), 10.0, None))).on_press(Message::Resume)
            ]
            .spacing(16.0)
            .align_y(iced::Center)
            .into()
        };
        cards = cards.push(container(line).center_x(Length::Fill));
    }
    let foot = column![
        tb("Press any button, move the mouse or press a key to come back", 18.0, c_text()),
        t(if a.since.is_empty() { String::new() } else { format!("Away since {}", a.since) }, 15.0, c_muted()),
    ]
    .spacing(6.0)
    .align_x(iced::Center);
    let body = column![
        top,
        Space::new().height(Length::FillPortion(2)),
        container(clock).center_x(Length::Fill),
        Space::new().height(Length::Fixed(18.0)),
        container(lead).center_x(Length::Fill),
        Space::new().height(Length::Fixed(40.0)),
        container(cards.max_width(1100.0)).center_x(Length::Fill),
        Space::new().height(Length::FillPortion(3)),
        container(foot).center_x(Length::Fill),
    ];
    let page = container(body).padding(Padding { top: 46.0, right: 72.0, bottom: 46.0, left: 72.0 }).width(Length::Fill).height(Length::Fill).style(boxed(page_bg(glow(0.05)), 0.0, None));
    mouse_area(page).on_press(Message::Back).into()
}

fn subscription(_s: &A) -> Subscription<Message> {
    Subscription::batch([Subscription::run(link::stream).map(Message::Hub), iced::keyboard::listen().map(Message::Key)])
}

pub fn run() -> i32 {
    link::set_surface("away");
    iced_layershell::disable_clipboard();
    let layer_settings = LayerShellSettings {
        anchor: Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right,
        layer: Layer::Overlay,
        exclusive_zone: -1,
        size: Some((0, 0)),
        keyboard_interactivity: KeyboardInteractivity::Exclusive,
        start_mode: link::start_mode(),
        ..Default::default()
    };
    let r = iced_layershell::application(A::default, || "hearth-firehold-away".to_string(), update, view)
        .style(|_s: &A, _t: &iced::Theme| iced::theme::Style { background_color: c_bg(), text_color: c_text() })
        .subscription(subscription)
        .settings(Settings { layer_settings, default_font: INTER, ..Default::default() })
        .run();
    super::ended("away", r)
}
