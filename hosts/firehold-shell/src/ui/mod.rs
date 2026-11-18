//! The surfaces, each its own small iced program (`firehold-shell couch|bar|dock|guide|notice|perf|settings`): one
//! wlr-layer-shell surface each (Settings is an ordinary window), started and stopped by the hub. A surface sleeps
//! until the hub sends something or the person touches it: no timers of their own, no animation.

pub mod art;
pub mod couch;
pub mod desk;
pub mod guide;
pub mod settings;
pub mod style;

use iced::widget::{container, row, text};
use iced::{border, Element, Length};

use style::*;

/// The keyboard's keys as the controller's (arrows, Enter = A, Escape or Backspace = B, Page Up/Down = LB/RB).
pub fn key_nav(ev: &iced::keyboard::Event) -> Option<&'static str> {
    use iced::keyboard::key::Named;
    use iced::keyboard::{Event, Key};
    let Event::KeyPressed { key, .. } = ev else { return None };
    match key.as_ref() {
        Key::Named(Named::ArrowUp) => Some("up"),
        Key::Named(Named::ArrowDown) => Some("down"),
        Key::Named(Named::ArrowLeft) => Some("left"),
        Key::Named(Named::ArrowRight) => Some("right"),
        Key::Named(Named::Enter) | Key::Named(Named::Space) => Some("a"),
        Key::Named(Named::Escape) | Key::Named(Named::Backspace) => Some("b"),
        Key::Named(Named::PageUp) => Some("lb"),
        Key::Named(Named::PageDown) => Some("rb"),
        Key::Named(Named::Home) => Some("lt"),
        Key::Named(Named::End) => Some("rt"),
        Key::Character("x") => Some("x"),
        Key::Character("y") => Some("y"),
        _ => None,
    }
}

/// A button hint in the bar at the bottom (mockups 1 and 4): a round key in its colour, then what it does.
pub fn hint<'a, M: 'a>(key: &'a str, label: &'a str, k: f32) -> Element<'a, M> {
    let (bg, fg) = match key {
        "A" => (c_good(), c_ink()),
        "X" => (BLUE, c_ink()),
        "Y" => (c_flame(), c_ink()),
        "B" => (c_danger(), c_ink()),
        _ => (Pal::mix(c_bg(), c_muted(), 0.82), c_ink()),
    };
    let wide = key.chars().count() > 1;
    let keycap = container(text(key).size(15.0 * k).font(bold()).color(fg))
        .center_x(Length::Fixed(if wide { 54.0 } else { 32.0 } * k))
        .center_y(Length::Fixed(32.0 * k))
        .style(move |_| container::Style { background: Some(bg.into()), border: border::rounded(16.0 * k), ..container::Style::default() });
    row![keycap, t(label, 18.0 * k, c_text())].spacing(10.0 * k).align_y(iced::Center).into()
}

/// The stand-in for a game's art (Hearth has no art for most): its colour, from model::tile_colour.
pub fn tile_bg(id: &str) -> iced::Color {
    rgb(crate::model::tile_colour(id))
}

/// Starts a layer surface's iced program and says why it couldn't, for the hub's log.
pub fn ended(name: &str, r: Result<(), iced_layershell::Error>) -> i32 {
    match r {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("firehold-shell {name}: {e}");
            1
        }
    }
}
