//! The bar (Mantel 2026's islands, at the top): the Hearth flame (opens the app menu) and the desktops on the left,
//! the clock pill in the middle (time and date), the status island on the right (Wi-Fi or wired, the volume, the
//! battery) and the Game Mode button, always there. "bar" looks (Steam-style) draw one full-width bar instead.
//! Clicks: the volume's scroll steps it by 5, middle-click mutes; a desktop pill switches to it.

use iced::widget::{button, container, image, mouse_area, row, stack, Space};
use iced::{border, Alignment, Color, Element, Length, Padding, Subscription, Task};
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, Settings, StartMode};
use iced_layershell::to_layer_message;

use crate::look::{self, a, icon, island, t, tb, Pal, Tone};
use crate::{sys, wm};

#[to_layer_message]
#[derive(Debug, Clone)]
pub enum Message {
    Tick,
    Look,
    Wm(wm::Snapshot),
    Menu,
    Desk(u32),
    GameMode,
    VolUp,
    VolDown,
    Mute,
}

struct Bar {
    wm: wm::Snapshot,
    status: sys::Status,
    clock: String,
    date: String,
    ticks: u32,
}

impl Default for Bar {
    fn default() -> Self {
        Bar { wm: wm::Snapshot::default(), status: sys::Status::read(true), clock: sys::clock_text(), date: sys::date_text(), ticks: 0 }
    }
}

pub const H_FLOAT: u32 = 52 + 10 + 6;
pub const H_BAR: u32 = 44;

fn height() -> u32 {
    if look::bar_style() == "floating" {
        H_FLOAT
    } else {
        H_BAR
    }
}

fn update(s: &mut Bar, m: Message) -> Task<Message> {
    match m {
        Message::Tick => {
            s.ticks = s.ticks.wrapping_add(1);
            s.clock = sys::clock_text();
            s.date = sys::date_text();
            // The volume asks wpctl (a process): every 5th tick only; the rest are file reads.
            let fresh = sys::Status::read(s.ticks % 5 == 0);
            s.status = sys::Status { volume: fresh.volume.or(s.status.volume), ..fresh };
        }
        Message::Look => {}
        Message::Wm(w) => s.wm = w,
        Message::Menu => crate::menu::toggle(),
        Message::Desk(id) => wm::go_desk(id),
        Message::GameMode => crate::apps::run_detached("nimbus-mode game"),
        Message::VolUp | Message::VolDown | Message::Mute => {
            match m {
                Message::VolUp => sys::volume_step(true),
                Message::VolDown => sys::volume_step(false),
                _ => sys::volume_mute(),
            }
            s.status.volume = sys::Status::read(true).volume;
        }
        _ => {}
    }
    Task::none()
}

fn flame<'a>(size: f32) -> Element<'a, Message> {
    let f = look::share().join("desktop/shell/images/hearth-logo.png");
    image(image::Handle::from_path(f)).width(size).height(size).into()
}

fn plain(p: Pal) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_, st| button::Style {
        background: matches!(st, button::Status::Hovered | button::Status::Pressed).then(|| p.hover().into()),
        border: border::rounded(p.r(12.0)),
        text_color: p.text,
        ..button::Style::default()
    }
}

fn left<'a>(s: &'a Bar, p: Pal, h: f32) -> Element<'a, Message> {
    let fl = button(container(flame(h - 22.0)).center(Length::Fixed(h - 12.0))).padding(0).style(plain(p)).on_press(Message::Menu);
    let mut desks = row![].spacing(6).align_y(Alignment::Center);
    for d in &s.wm.desks {
        let w = if d.active { 34.0 } else { 12.0 };
        let c = if d.active { p.accent } else { a(p.muted, 0.6) };
        let pill = container(Space::new().width(Length::Fixed(w)).height(Length::Fixed(12.0))).style(look::boxed(c, 6.0));
        desks = desks.push(mouse_area(pill).on_press(Message::Desk(d.id)));
    }
    let inner = row![fl, desks].spacing(10).align_y(Alignment::Center).padding(Padding::from([0.0, 10.0]));
    container(inner).height(h).align_y(Alignment::Center).style(island(p, h / 2.4, false)).into()
}

fn centre<'a>(s: &'a Bar, p: Pal, h: f32) -> Element<'a, Message> {
    let inner = row![tb(s.clock.as_str(), 15.0, p.text), t(s.date.as_str(), 13.0, p.muted)].spacing(10).align_y(Alignment::Center);
    container(inner).height(h).padding(Padding::from([0.0, 16.0])).align_y(Alignment::Center).style(island(p, h / 2.0, false)).into()
}

fn right<'a>(s: &'a Bar, p: Pal, h: f32) -> Element<'a, Message> {
    let mut st = row![].spacing(12).align_y(Alignment::Center);
    st = st.push(match s.status.wifi {
        Some(-1) => icon("wired", Tone::Light, 18.0),
        Some(_) => icon("wifi", Tone::Light, 18.0),
        None => icon("offline", Tone::Muted, 18.0),
    });
    if let Some((v, muted)) = s.status.volume {
        let ic = if muted || v == 0 { "vol-mute" } else if v < 40 { "vol-low" } else { "vol" };
        let vol = row![icon(ic, Tone::Light, 18.0), t(format!("{v}%"), 12.0, p.muted)].spacing(4).align_y(Alignment::Center);
        st = st.push(mouse_area(vol).on_middle_press(Message::Mute).on_scroll(|d| match d {
            iced::mouse::ScrollDelta::Lines { y, .. } | iced::mouse::ScrollDelta::Pixels { y, .. } => if y > 0.0 { Message::VolUp } else { Message::VolDown },
        }));
    }
    if let Some((pct, charging)) = s.status.battery {
        let ic = if charging { "batt-charging" } else if pct < 20 { "batt-low" } else { "batt" };
        st = st.push(row![icon(ic, Tone::Light, 18.0), t(format!("{pct}%"), 12.0, p.muted)].spacing(4).align_y(Alignment::Center));
    }
    let status = container(st).height(h).padding(Padding::from([0.0, 14.0])).align_y(Alignment::Center).style(island(p, h / 2.4, false));
    // Game Mode: always here, the biggest target in the bar (the accent, the controller icon and its words).
    let gm_in = row![icon("pad", Tone::Ink, 18.0), tb("Game Mode", 14.0, p.ink)].spacing(8).align_y(Alignment::Center);
    let gm = button(container(gm_in).height(h - 8.0).padding(Padding::from([0.0, 18.0])).align_y(Alignment::Center))
        .padding(0)
        .style(move |_, st| button::Style {
            background: Some((if matches!(st, button::Status::Hovered) { Pal::mix(p.accent, Color::WHITE, 0.12) } else { p.accent }).into()),
            border: border::rounded(p.r((h - 8.0) / 2.0)),
            text_color: p.ink,
            ..button::Style::default()
        })
        .on_press(Message::GameMode);
    row![status, gm].spacing(8).align_y(Alignment::Center).into()
}

fn view(s: &Bar) -> Element<'_, Message> {
    let p = look::pal();
    let floating = look::bar_style() == "floating";
    let h = if floating { 52.0 } else { 44.0 };
    let sides = row![left(s, p, h), Space::new().width(Length::Fill), right(s, p, h)].align_y(Alignment::Center);
    let mid = container(centre(s, p, h)).center_x(Length::Fill);
    let body = stack![sides, mid];
    if floating {
        container(body).padding(Padding { top: 10.0, right: 10.0, bottom: 6.0, left: 10.0 }).width(Length::Fill).height(Length::Fill).into()
    } else {
        // One full-width bar (Steam-style, Zen): the panel edge to edge, no islands' shadows.
        container(body).padding(Padding::from([0.0, 8.0])).width(Length::Fill).height(Length::Fill).style(look::boxed(p.panel, 0.0)).into()
    }
}

fn subscription(_s: &Bar) -> Subscription<Message> {
    Subscription::batch([
        iced::time::every(std::time::Duration::from_secs(2)).map(|_| Message::Tick),
        Subscription::run(wm::stream).map(Message::Wm),
        Subscription::run(look::changes).map(|_| Message::Look),
    ])
}

pub fn run() -> i32 {
    iced_layershell::disable_clipboard();
    let h = height();
    let layer_settings = LayerShellSettings {
        anchor: Anchor::Top | Anchor::Left | Anchor::Right,
        layer: Layer::Top,
        exclusive_zone: h as i32,
        size: Some((0, h)),
        keyboard_interactivity: KeyboardInteractivity::None,
        start_mode: StartMode::Active,
        ..Default::default()
    };
    let r = iced_layershell::application(Bar::default, || "hearth-bar".to_string(), update, view)
        .style(|_s: &Bar, _t: &iced::Theme| iced::theme::Style { background_color: Color::TRANSPARENT, text_color: look::pal().text })
        .subscription(subscription)
        .settings(Settings { layer_settings, default_font: look::INTER, ..Default::default() })
        .run();
    crate::ended("bar", r)
}
