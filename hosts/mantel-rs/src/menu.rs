//! The app menu (Mantel's Apps grid): a panel over the desktop with "Search apps" on top (just start typing) and
//! every app as an icon and its name. Enter or a click opens it, Escape or a click outside closes the menu. The bar's
//! flame opens and closes it (one menu at a time: its pid in the runtime folder). Its own process, so nothing of it
//! stays in memory while it's closed.

use iced::widget::{button, column, container, image, mouse_area, row, scrollable, svg, text_input, Space};
use iced::{border, keyboard, Alignment, Color, Element, Length, Padding, Subscription, Task};
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, Settings, StartMode};
use iced_layershell::to_layer_message;

use crate::apps::{self, App};
use crate::look::{self, a, t, tb, Pal};

#[to_layer_message]
#[derive(Debug, Clone)]
pub enum Message {
    Query(String),
    Open(usize),
    First,
    Close,
    Key(keyboard::Event),
}

struct Menu {
    apps: Vec<App>,
    q: String,
}

const COLS: usize = 7;

fn pid_file() -> std::path::PathBuf {
    crate::sys::run_dir().join("hearth-mantel-menu.pid")
}

/// The flame: opens the menu, or closes the one that's open.
pub fn toggle() {
    if let Ok(s) = std::fs::read_to_string(pid_file()) {
        if let Ok(pid) = s.trim().parse::<i32>() {
            if pid > 1 && unsafe { libc::kill(pid, 0) } == 0 {
                unsafe { libc::kill(pid, libc::SIGTERM) };
                let _ = std::fs::remove_file(pid_file());
                return;
            }
        }
    }
    if let Ok(me) = std::env::current_exe() {
        apps::run_detached(&format!("{} menu", apps::shell_quote(&me.to_string_lossy())));
    }
}

fn shown(s: &Menu) -> Vec<usize> {
    s.apps.iter().enumerate().filter(|(_, x)| s.q.trim().is_empty() || apps::matches(x, &s.q)).map(|(i, _)| i).collect()
}

fn update(s: &mut Menu, m: Message) -> Task<Message> {
    match m {
        Message::Query(q) => s.q = q,
        Message::Open(i) => {
            if let Some(x) = s.apps.get(i) {
                apps::launch(x);
            }
            return close();
        }
        Message::First => {
            if let Some(&i) = shown(s).first() {
                apps::launch(&s.apps[i]);
                return close();
            }
        }
        Message::Close => return close(),
        Message::Key(keyboard::Event::KeyPressed { key: keyboard::Key::Named(keyboard::key::Named::Escape), .. }) => return close(),
        _ => {}
    }
    Task::none()
}

fn close() -> Task<Message> {
    let _ = std::fs::remove_file(pid_file());
    iced::exit()
}

fn tile<'a>(p: Pal, x: &'a App, i: usize) -> Element<'a, Message> {
    let pic: Element<'a, Message> = match &x.icon_path {
        Some(f) if f.extension().and_then(|e| e.to_str()) == Some("svg") => svg(svg::Handle::from_path(f)).width(48).height(48).into(),
        Some(f) => image(image::Handle::from_path(f)).width(48).height(48).into(),
        None => container(tb(x.name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default(), 22.0, p.ink))
            .center(Length::Fixed(48.0))
            .style(look::boxed(p.accent, p.r(12.0)))
            .into(),
    };
    let name = t(hearth_ui::ui::fit(&x.name, 18), 12.0, p.text).width(Length::Fill).align_x(Alignment::Center);
    button(column![pic, name].spacing(8).align_x(Alignment::Center).width(Length::Fixed(104.0)))
        .padding(Padding::from([10.0, 4.0]))
        .style(move |_, st| button::Style {
            background: matches!(st, button::Status::Hovered | button::Status::Pressed).then(|| p.hover().into()),
            border: border::rounded(p.r(12.0)),
            text_color: p.text,
            ..button::Style::default()
        })
        .on_press(Message::Open(i))
        .into()
}

fn view(s: &Menu) -> Element<'_, Message> {
    let p = look::pal();
    let search = text_input("Search apps, just start typing", &s.q)
        .id("q")
        .on_input(Message::Query)
        .on_submit(Message::First)
        .size(16)
        .padding(12)
        .style(move |_, _| text_input::Style {
            background: p.panel2.into(),
            border: border::rounded(p.r(10.0)).color(p.line).width(1.0),
            icon: p.muted,
            placeholder: p.muted,
            value: p.text,
            selection: a(p.accent, 0.35),
        });
    let list = shown(s);
    let mut grid = column![].spacing(4);
    for chunk in list.chunks(COLS) {
        let mut r = row![].spacing(4);
        for &i in chunk {
            r = r.push(tile(p, &s.apps[i], i));
        }
        grid = grid.push(r);
    }
    let body: Element<'_, Message> = if list.is_empty() { t("No apps match.", 14.0, p.muted).into() } else { scrollable(grid).height(Length::Fill).into() };
    let panel = container(column![row![tb("Apps", 20.0, p.text), Space::new().width(Length::Fill), t("Esc closes", 12.0, p.muted)].align_y(Alignment::Center), search, body].spacing(14))
        .width(Length::Fixed((COLS as f32) * 108.0 + 48.0))
        .height(Length::Fixed(620.0))
        .padding(24)
        .style(look::island(p, 16.0, false));
    // A click outside the panel closes the menu.
    let inside = mouse_area(panel).on_press(Message::Query(s.q.clone()));
    mouse_area(container(inside).center(Length::Fill).style(look::boxed(Color { a: 0.35, ..Color::BLACK }, 0.0))).on_press(Message::Close).into()
}

fn subscription(_s: &Menu) -> Subscription<Message> {
    keyboard::listen().map(Message::Key)
}

pub fn run() -> i32 {
    let _ = std::fs::write(pid_file(), std::process::id().to_string());
    iced_layershell::disable_clipboard();
    let layer_settings = LayerShellSettings {
        anchor: Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right,
        layer: Layer::Overlay,
        exclusive_zone: -1,
        size: Some((0, 0)),
        keyboard_interactivity: KeyboardInteractivity::Exclusive,
        start_mode: StartMode::Active,
        ..Default::default()
    };
    let boot = || (Menu { apps: apps::all(), q: String::new() }, iced::widget::operation::focus("q"));
    let r = iced_layershell::application(boot, || "hearth-menu".to_string(), update, view)
        .style(|_s: &Menu, _t: &iced::Theme| iced::theme::Style { background_color: Color::TRANSPARENT, text_color: look::pal().text })
        .subscription(subscription)
        .settings(Settings { layer_settings, default_font: look::INTER, ..Default::default() })
        .run();
    let _ = std::fs::remove_file(pid_file());
    crate::ended("menu", r)
}
