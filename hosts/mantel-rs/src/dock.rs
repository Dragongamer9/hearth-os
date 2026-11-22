//! The dock (Mantel 2026, along the bottom): the app grid button (the menu), your pinned apps (desk.json, the QML
//! Mantel's, only read here) and then every other open app, each app's windows sharing one button. A running app has
//! a dot, the one in front a long accent dot. Click: open it, or bring its window to the front (in front already:
//! minimize it); middle-click: open another one.

use iced::widget::{button, column, container, image, mouse_area, row, svg, Space};
use iced::{border, Alignment, Color, Element, Length, Padding, Subscription, Task};
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, Settings, StartMode};
use iced_layershell::to_layer_message;

use crate::apps::{self, App};
use crate::look::{self, a, icon, tb, Pal, Tone};
use crate::{sys, wm};

#[to_layer_message]
#[derive(Debug, Clone)]
pub enum Message {
    Wm(wm::Snapshot),
    Look,
    Pins,
    Menu,
    Click(usize),
    Middle(usize),
}

/// One button: the app (when it's installed), its windows, pinned or not.
#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub app: Option<App>,
    pub key: String,
    pub name: String,
    pub wins: Vec<wm::Win>,
    pub pinned: bool,
}

struct Dock {
    apps: Vec<App>,
    pins: Vec<String>,
    wm: wm::Snapshot,
}

pub const H: u32 = 64 + 10 + 6;

/// The buttons, in order: pins first (in pin order), then the other open apps by when they opened.
pub fn items(apps: &[App], pins: &[String], wins: &[wm::Win]) -> Vec<Item> {
    let mut v: Vec<Item> = Vec::new();
    for p in pins {
        let app = apps::find(apps, p).cloned();
        let Some(app) = app else { continue };
        v.push(Item { key: app.id.clone(), name: app.name.clone(), app: Some(app), wins: Vec::new(), pinned: true });
    }
    for w in wins {
        let app = apps::for_window(apps, &w.app_id);
        let key = app.map(|x| x.id.clone()).unwrap_or_else(|| w.app_id.clone());
        if key.is_empty() || key.contains("hearth-mantel") {
            continue;
        }
        if let Some(it) = v.iter_mut().find(|i| i.key == key) {
            it.wins.push(w.clone());
        } else {
            v.push(Item { key, name: app.map(|x| x.name.clone()).unwrap_or_else(|| w.title.clone()), app: app.cloned(), wins: vec![w.clone()], pinned: false });
        }
    }
    v
}

fn update(s: &mut Dock, m: Message) -> Task<Message> {
    match m {
        Message::Wm(w) => s.wm = w,
        Message::Look => {}
        Message::Pins => s.pins = sys::pins(),
        Message::Menu => crate::menu::toggle(),
        Message::Click(i) => {
            if let Some(it) = items(&s.apps, &s.pins, &s.wm.wins).get(i) {
                match it.wins.iter().find(|w| w.active) {
                    Some(w) if it.wins.len() == 1 => wm::minimize(w.id),
                    _ => match it.wins.first() {
                        Some(w) => wm::activate(w.id),
                        None => {
                            if let Some(a) = &it.app {
                                apps::launch(a);
                            }
                        }
                    },
                }
            }
        }
        Message::Middle(i) => {
            if let Some(a) = items(&s.apps, &s.pins, &s.wm.wins).get(i).and_then(|it| it.app.clone()) {
                apps::launch(&a);
            }
        }
        _ => {}
    }
    Task::none()
}

fn pic<'a>(p: Pal, it: &Item, size: f32) -> Element<'a, Message> {
    match it.app.as_ref().and_then(|a| a.icon_path.clone()) {
        Some(f) if f.extension().and_then(|e| e.to_str()) == Some("svg") => svg(svg::Handle::from_path(f)).width(size).height(size).into(),
        Some(f) => image(image::Handle::from_path(f)).width(size).height(size).into(),
        None => container(tb(it.name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_else(|| "?".into()), size * 0.45, p.ink))
            .center(Length::Fixed(size))
            .style(look::boxed(p.accent, p.r(size / 4.0)))
            .into(),
    }
}

fn view(s: &Dock) -> Element<'_, Message> {
    let p = look::pal();
    let h = 64.0;
    let flat = look::bar_style() != "floating";
    let btn_style = move |_: &iced::Theme, st: button::Status| button::Style {
        background: matches!(st, button::Status::Hovered | button::Status::Pressed).then(|| p.hover().into()),
        border: border::rounded(p.r(12.0)),
        text_color: p.text,
        ..button::Style::default()
    };
    let grid = button(container(icon("grid", Tone::Light, 26.0)).center(Length::Fixed(h - 14.0))).padding(0).style(btn_style).on_press(Message::Menu);
    let mut r = row![grid].spacing(4).align_y(Alignment::Center);
    for (i, it) in items(&s.apps, &s.pins, &s.wm.wins).into_iter().enumerate() {
        let front = it.wins.iter().any(|w| w.active);
        let dot: Element<'_, Message> = if it.wins.is_empty() {
            Space::new().height(Length::Fixed(4.0)).into()
        } else {
            container(Space::new().width(Length::Fixed(if front { 18.0 } else { 5.0 })).height(Length::Fixed(4.0))).style(look::boxed(if front { p.accent } else { a(p.text, 0.7) }, 2.0)).into()
        };
        let face = column![pic(p, &it, 38.0), dot].spacing(4).align_x(Alignment::Center);
        let b = button(container(face).center_x(Length::Fixed(h - 10.0)).padding(Padding { top: 6.0, bottom: 2.0, ..Padding::ZERO })).padding(0).style(btn_style).on_press(Message::Click(i));
        r = r.push(mouse_area(b).on_middle_press(Message::Middle(i)));
    }
    let shelf = container(r).padding(Padding::from([0.0, 8.0])).height(h).align_y(Alignment::Center).style(look::island(p, if flat { 0.0 } else { 18.0 }, false));
    container(shelf).center_x(Length::Fill).padding(Padding { top: 6.0, bottom: 10.0, ..Padding::ZERO }).height(Length::Fill).into()
}

fn subscription(_s: &Dock) -> Subscription<Message> {
    Subscription::batch([
        Subscription::run(wm::stream).map(Message::Wm),
        Subscription::run(look::changes).map(|_| Message::Look),
        // desk.json: pins change when the QML Mantel (or Settings) pins an app; a cheap read every 5 s.
        iced::time::every(std::time::Duration::from_secs(5)).map(|_| Message::Pins),
    ])
}

pub fn run() -> i32 {
    iced_layershell::disable_clipboard();
    let layer_settings = LayerShellSettings {
        anchor: Anchor::Bottom | Anchor::Left | Anchor::Right,
        layer: Layer::Top,
        exclusive_zone: H as i32,
        size: Some((0, H)),
        keyboard_interactivity: KeyboardInteractivity::None,
        start_mode: StartMode::Active,
        ..Default::default()
    };
    let boot = || Dock { apps: apps::all(), pins: sys::pins(), wm: wm::Snapshot::default() };
    let r = iced_layershell::application(boot, || "hearth-dock".to_string(), update, view)
        .style(|_s: &Dock, _t: &iced::Theme| iced::theme::Style { background_color: Color::TRANSPARENT, text_color: look::pal().text })
        .subscription(subscription)
        .settings(Settings { layer_settings, default_font: look::INTER, ..Default::default() })
        .run();
    crate::ended("dock", r)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(id: &str) -> App {
        crate::apps::parse(id, &format!("[Desktop Entry]\nName={id}\nExec={id}\n")).unwrap()
    }

    #[test]
    fn pins_first_then_open_apps_sharing_a_button() {
        let apps = vec![app("org.a.files"), app("firefox"), app("steam")];
        let pins = vec!["firefox".to_string(), "gone.app".to_string()];
        let wins = vec![
            wm::Win { id: 1, app_id: "steam".into(), title: "Steam".into(), ..Default::default() },
            wm::Win { id: 2, app_id: "firefox".into(), title: "Web".into(), active: true, ..Default::default() },
            wm::Win { id: 3, app_id: "steam".into(), title: "Friends".into(), ..Default::default() },
        ];
        let v = items(&apps, &pins, &wins);
        assert_eq!(v.iter().map(|i| i.key.as_str()).collect::<Vec<_>>(), ["firefox", "steam"]);
        assert!(v[0].pinned && v[0].wins.len() == 1);
        assert_eq!(v[1].wins.len(), 2);
    }
}
