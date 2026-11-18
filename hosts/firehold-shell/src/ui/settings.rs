//! `firehold-shell settings`: Settings › Couch and Desk (mockup 6), an ordinary window drawn as the mockup has it: its
//! own rounded edge and no title bar (Escape closes it); on the Couch it stays a window in the middle. Every change goes to the hub, which writes it with settings_file.rs (one key,
//! only when it's different); opening it writes nothing. System holds this test's Desktop Mode choice (Firehold or
//! KDE Plasma) and the way back to Game Mode.

use iced::widget::{column, container, mouse_area, row, text, Space};
use iced::{Color, Element, Length, Padding, Size, Subscription, Task};
use serde_json::json;

use super::style::*;
use crate::link::{self, HubMsg};
use crate::proto::State;

#[derive(Debug, Clone)]
pub enum Message {
    Hub(HubMsg),
    Page(usize),
    Posture(&'static str),
    Cycle(&'static str),
    Choose(&'static str),
    Leave(&'static str),
    Key(iced::keyboard::Event),
}

struct Win {
    st: State,
    page: usize,
}

const PAGES: [&str; 9] = ["System", "Couch and Desk", "Display", "Gaming", "Controllers", "Network", "Sound", "Accounts", "Updates"];

fn start_label(v: &str) -> &'static str {
    match v {
        "couch" => "Couch",
        "last" => "Where you left it",
        _ => "Desk",
    }
}
fn guide_label(v: &str) -> &'static str {
    if v == "open" { "Everything open" } else { "Quick Access" }
}
fn games_label(v: &str) -> &'static str {
    match v {
        "fullscreen" => "Always full screen",
        "window" => "Always in a window",
        _ => "Follow the posture",
    }
}
fn next<'a>(seq: &[&'a str], cur: &str) -> &'a str {
    let i = seq.iter().position(|s| *s == cur).unwrap_or(0);
    seq[(i + 1) % seq.len()]
}

fn update(s: &mut Win, m: Message) -> Task<Message> {
    let cfg = s.st.cfg.clone();
    match m {
        Message::Hub(HubMsg::State(st)) => s.st = *st,
        // The hub (Super+I, tests): open a page by name.
        Message::Hub(HubMsg::Cmd(cmd, arg)) if cmd == "settingsPage" => {
            if let Some(i) = PAGES.iter().position(|p| p.eq_ignore_ascii_case(&arg) || (arg == "couch" && *p == "Couch and Desk")) {
                s.page = i;
            }
        }
        Message::Hub(HubMsg::Quit) | Message::Hub(HubMsg::Gone) => return iced::exit(),
        Message::Hub(_) => {}
        Message::Page(p) => s.page = p,
        Message::Posture(p) => link::send(json!({"op": "set", "key": "posture", "value": p})),
        Message::Cycle(k) => {
            let v = match k {
                "startIn" => json!(next(&["couch", "desk", "last"], &cfg.start_in)),
                "guide" => json!(next(&["quick", "open"], &cfg.guide)),
                "games" => json!(next(&["follow", "fullscreen", "window"], &cfg.games)),
                "bigPicture" => json!(!cfg.big_picture),
                _ => return Task::none(),
            };
            link::send(json!({"op": "set", "key": k, "value": v}));
        }
        Message::Choose(w) => link::send(json!({"op": "choose", "what": w})),
        Message::Leave(to) => link::send(json!({"op": "leave", "to": to})),
        Message::Key(ev) => {
            use iced::keyboard::{key::Named, Event, Key};
            if let Event::KeyPressed { key, .. } = ev {
                match key.as_ref() {
                    Key::Named(Named::Escape) => return iced::exit(),
                    Key::Named(Named::ArrowUp) if s.page > 0 => s.page -= 1,
                    Key::Named(Named::ArrowDown) if s.page + 1 < PAGES.len() => s.page += 1,
                    _ => {}
                }
            }
        }
    }
    Task::none()
}

/// The window's corner (mockup 6).
const WIN_R: f32 = 22.0;

fn value_chip<'a>(v: String, m: Message) -> Element<'a, Message> {
    mouse_area(container(ts(v, 16.0, c_text())).padding(Padding { top: 9.0, right: 16.0, bottom: 9.0, left: 16.0 }).style(boxed(shade(0.76), 10.0, None)))
        .on_press(m)
        .into()
}

fn setting_row<'a>(title: &'a str, sub: &'a str, value: Element<'a, Message>, last: bool) -> Element<'a, Message> {
    let line = container(row![column![ts(title, 17.0, c_text()), t(sub, 14.0, c_muted())].spacing(4.0), Space::new().width(Length::Fill), value].align_y(iced::Center))
        .padding(Padding { top: 0.0, right: 22.0, bottom: 0.0, left: 22.0 })
        .center_y(Length::Fixed(76.0))
        .width(Length::Fill);
    if last {
        line.into()
    } else {
        column![line, container(Space::new()).width(Length::Fill).height(Length::Fixed(1.0)).style(boxed(c_line(), 0.0, None))].into()
    }
}

fn posture_card<'a>(key: &'static str, title: &'a str, sub: &'a str, on: bool) -> Element<'a, Message> {
    let picture: Element<Message> = match key {
        "auto" => row![
            container(Space::new()).width(Length::FillPortion(1)).height(Length::Fill).style(boxed(c(0x5a, 0x2a, 0x1c), 0.0, None)),
            container(Space::new()).width(Length::FillPortion(1)).height(Length::Fill).style(boxed(shade(0.94), 0.0, None)),
        ]
        .into(),
        "couch" => container(Space::new()).width(Length::Fill).height(Length::Fill).style(boxed(hero_bg(), 0.0, None)).into(),
        _ => container(Space::new()).width(Length::Fill).height(Length::Fill).style(boxed(shade(0.94), 0.0, None)).into(),
    };
    let pic = container(picture).width(Length::Fill).height(Length::Fixed(130.0)).clip(true).style(boxed(Color::TRANSPARENT, 12.0, None));
    let card = container(column![pic, Space::new().height(Length::Fixed(18.0)), tb(title, 19.0, c_text()), Space::new().height(Length::Fixed(8.0)), t(sub, 14.0, c_muted())])
        .padding(17.0)
        .width(Length::FillPortion(1))
        .height(Length::Fixed(240.0))
        .style(boxed(shade(0.30), 14.0, Some(if on { (c_accent(), 2.0) } else { (c_line(), 1.0) })));
    mouse_area(card).on_press(Message::Posture(key)).into()
}

fn couch_desk(s: &Win) -> Element<'_, Message> {
    let cfg = &s.st.cfg;
    let cards = row![
        posture_card("auto", "Automatic", "Controller → Couch, mouse or keyboard → Desk", cfg.posture == "auto"),
        posture_card("couch", "Always Couch", "A console: tiles, full screen, Guide button", cfg.posture == "couch"),
        posture_card("desk", "Always Desk", "A PC: windows, bar and dock", cfg.posture == "desk"),
    ]
    .spacing(20.0);
    let per = if cfg.per_screen.is_empty() {
        "Every screen the same".to_string()
    } else {
        cfg.per_screen
            .iter()
            .map(|(k, v)| format!("{k}: {}", if v.as_str() == Some("couch") { "Couch" } else { "Desk" }))
            .collect::<Vec<_>>()
            .join(" · ")
    };
    let rows = column![
        setting_row("Per screen", "The TV stays Couch while the monitor is Desk", value_chip(per, Message::Page(1)), false),
        setting_row("Start in", "What you see when the PC turns on", value_chip(start_label(&cfg.start_in).into(), Message::Cycle("startIn")), false),
        setting_row("Guide button", "Press once: Quick Access. Twice: everything open", value_chip(guide_label(&cfg.guide).into(), Message::Cycle("guide")), false),
        setting_row("Games", "Open full screen on the Couch, in a window on the Desk", value_chip(games_label(&cfg.games).into(), Message::Cycle("games")), false),
        setting_row("Big Picture", "Steam's own TV mode isn't needed; Steam is an app like the others", value_chip(if cfg.big_picture { "On".into() } else { "Off".into() }, Message::Cycle("bigPicture")), true),
    ];
    column![
        tb("Couch and Desk", 36.0, c_text()),
        Space::new().height(Length::Fixed(24.0)),
        t("One desktop, two ways to sit. Couch is for the controller and the TV; Desk is for the mouse and keyboard. Switching never closes anything.", 18.0, soft(0.64)),
        Space::new().height(Length::Fixed(26.0)),
        cards,
        Space::new().height(Length::Fixed(26.0)),
        container(rows).style(boxed(shade(0.30), 12.0, None)),
    ]
    .into()
}

fn system(s: &Win) -> Element<'_, Message> {
    let d = s.st.desktop.as_str();
    let choice = |key: &'static str, title: &'static str, sub: &'static str, on: bool| -> Element<'static, Message> {
        mouse_area(
            container(column![tb(title, 19.0, c_text()), t(sub, 14.0, c_muted())].spacing(8.0))
                .padding(20.0)
                .width(Length::FillPortion(1))
                .style(boxed(shade(0.30), 14.0, Some(if on { (c_accent(), 2.0) } else { (c_line(), 1.0) }))),
        )
        .on_press(Message::Choose(key))
        .into()
    };
    let leave = |label: &'static str, to: &'static str| -> Element<'static, Message> {
        mouse_area(container(ts(label, 16.0, c_text())).padding(Padding { top: 12.0, right: 20.0, bottom: 12.0, left: 20.0 }).style(boxed(shade(0.76), 10.0, None)))
            .on_press(Message::Leave(to))
            .into()
    };
    column![
        tb("System", 36.0, c_text()),
        Space::new().height(Length::Fixed(24.0)),
        ts("Desktop Mode on this test edition", 20.0, c_text()),
        Space::new().height(Length::Fixed(8.0)),
        t("Which desktop opens when you switch to Desktop Mode. It's used from the next switch, and it stays as you leave it.", 16.0, c_muted()),
        Space::new().height(Length::Fixed(16.0)),
        {
            let mut r = row![choice("firehold", "Firehold (Rust, test)", "Couch and Desk in one session, all in Rust", d == "firehold")].spacing(20.0);
            if s.st.desktop_plasma {
                r = r.push(choice("firehold-plasma", "Firehold on Plasma (test)", "Couch and Desk as Plasma shells on KWin", d == "firehold-plasma"));
            }
            r.push(choice("plasma", "KDE Plasma", "The default: Plasma with Hearth's apps", d != "firehold" && d != "firehold-plasma"))
        },
        Space::new().height(Length::Fixed(30.0)),
        row![leave("Back to Game Mode", "game"), leave("Switch to KDE Plasma now", "plasma")].spacing(14.0),
    ]
    .into()
}

fn elsewhere<'a>(title: &'a str) -> Element<'a, Message> {
    column![
        tb(title, 36.0, c_text()),
        Space::new().height(Length::Fixed(20.0)),
        t("This page isn't in Firehold's Rust test yet. Couch and Desk and System are.", 17.0, c_muted()),
    ]
    .into()
}

fn view(s: &Win) -> Element<'_, Message> {
    let mut list = column![].spacing(4.0);
    for (i, p) in PAGES.iter().enumerate() {
        let on = i == s.page;
        let item = container(text(*p).size(17.0).font(if on { semi() } else { INTER }).color(if on { c_text() } else { soft(0.43) }))
            .padding(Padding { top: 12.0, right: 16.0, bottom: 12.0, left: 16.0 })
            .width(Length::Fill)
            .style(boxed(if on { shade(1.00) } else { Color::TRANSPARENT }, 10.0, None));
        list = list.push(mouse_area(item).on_press(Message::Page(i)));
    }
    // The window's own rounded edge (it has no title bar): the list's side rounded on the left, the page on the right.
    let side_bg = shade(0.22);
    let side = container(list).padding(Padding { top: 40.0, right: 18.0, bottom: 20.0, left: 18.0 }).width(Length::Fixed(340.0)).height(Length::Fill).style(move |_| container::Style {
        background: Some(side_bg.into()),
        border: iced::Border { radius: iced::border::Radius::new(WIN_R).top_right(0.0).bottom_right(0.0), ..Default::default() },
        ..container::Style::default()
    });
    let page: Element<Message> = match s.page {
        0 => system(s),
        1 => couch_desk(s),
        p => elsewhere(PAGES[p]),
    };
    let body = container(page).padding(Padding { top: 46.0, right: 52.0, bottom: 30.0, left: 52.0 }).width(Length::Fill).height(Length::Fill);
    container(row![side, body]).width(Length::Fill).height(Length::Fill).style(boxed(shade(0.38), WIN_R, Some((c_line(), 1.0)))).into()
}

fn subscription(_s: &Win) -> Subscription<Message> {
    Subscription::batch([Subscription::run(link::stream).map(Message::Hub), iced::keyboard::listen().map(Message::Key)])
}

pub fn run() -> i32 {
    link::set_surface("settings");
    let window = iced::window::Settings {
        size: Size::new(1520.0, 940.0),
        platform_specific: iced::window::settings::PlatformSpecific { application_id: "firehold-settings".into(), ..Default::default() },
        // Mockup 6: the window itself with its rounded edge and no title bar (Escape closes it, as B does).
        decorations: false,
        transparent: true,
        ..Default::default()
    };
    let r = iced::application(|| Win { st: State::default(), page: 1 }, update, view)
        .title("Settings")
        .window(window)
        .default_font(INTER)
        .subscription(subscription)
        .style(|_s: &Win, _t: &iced::Theme| iced::theme::Style { background_color: Color::TRANSPARENT, text_color: c_text() })
        .run();
    match r {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("firehold-shell settings: {e}");
            1
        }
    }
}
