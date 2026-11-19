//! `firehold-shell quick`: Quick Settings inside Firehold (it's its own desktop: Plasma's and Mantel's panels aren't
//! here), Mantel's QuickAccess.qml ported: you and the battery, Settings / Lock / Power; Return to Game Mode, big;
//! tiles for Wi-Fi, Bluetooth, Do not disturb, the on-screen keyboard, Away and the Couch; volume and brightness; the
//! power profile; and the notifications that came. Opened from the Desk bar's Wi-Fi and battery (or `ctl quick`); the
//! mouse, the keyboard (arrows, Enter, Escape) and the controller (D-pad, A, B). It reads what it shows when it opens
//! (the hub runs nmcli, bluetoothctl, wpctl and powerprofilesctl once, with time limits), never on a timer.

use iced::widget::{column, container, mouse_area, row, text, Space};
use iced::{Color, Element, Length, Padding, Subscription, Task};
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, Settings};
use iced_layershell::to_layer_message;
use serde_json::json;

use hearth_ui::ui::{glyph, G};

use super::key_nav;
use super::style::*;
use crate::link::{self, HubMsg};
use crate::proto::State;

#[to_layer_message]
#[derive(Debug, Clone)]
pub enum Message {
    Hub(HubMsg),
    Key(iced::keyboard::Event),
    Do(String),
    Hover(String),
}

#[derive(Default)]
struct Q {
    st: State,
    /// The control the controller or the arrows are on (its id).
    sel: String,
    h: u32,
}

pub const W: u32 = 460;

/// The controls as rows, for the controller and the arrows (Mantel's navRows, Firehold's tiles).
fn nav_rows(st: &State) -> Vec<Vec<String>> {
    let q = &st.quick;
    let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<String>>();
    let mut rows = vec![if q.can_lock { s(&["settings", "lock", "power"]) } else { s(&["settings", "power"]) }];
    if q.power_open {
        rows.push(s(&["sleep", "restart", "shutdown"]));
    }
    rows.push(s(&["game"]));
    rows.push(s(&["wifi", "bt"]));
    rows.push(s(&["dnd", "keyboard"]));
    rows.push(s(&["away", "couch"]));
    rows.push(s(&["volume"]));
    if q.brightness.is_some() {
        rows.push(s(&["brightness"]));
    }
    if !q.profile.is_empty() {
        rows.push(s(&["power-saver", "balanced", "performance"]));
    }
    if !st.history.is_empty() {
        rows.push(s(&["clearNotes"]));
    }
    rows
}

/// The panel's height for what it shows now (the surface is exactly the panel).
fn height(st: &State) -> u32 {
    let q = &st.quick;
    let mut h = 24.0 + 50.0 + 14.0 + 72.0 + 14.0 + 3.0 * 76.0 + 2.0 * 10.0 + 14.0 + 52.0;
    if q.power_open {
        h += 10.0 + 44.0;
    }
    if q.brightness.is_some() {
        h += 10.0 + 52.0;
    }
    if !q.profile.is_empty() {
        h += 14.0 + 20.0 + 44.0;
    }
    if st.history.is_empty() {
        h += 14.0 + 40.0;
    } else {
        h += 14.0 + 30.0 + st.history.len().min(4) as f32 * 72.0;
    }
    (h + 24.0).ceil() as u32
}

fn update(s: &mut Q, m: Message) -> Task<Message> {
    match m {
        Message::Hub(HubMsg::State(st)) => {
            s.st = *st;
            if !s.st.quick.up {
                return iced::exit();
            }
            if s.sel.is_empty() {
                s.sel = "game".into();
            }
            let h = height(&s.st);
            if h != s.h {
                s.h = h;
                return Task::done(Message::SizeChange((W, h)));
            }
        }
        Message::Hub(HubMsg::Nav(k)) => return nav(s, &k),
        Message::Hub(HubMsg::Quit) | Message::Hub(HubMsg::Gone) => return iced::exit(),
        Message::Hub(_) => {}
        Message::Key(ev) => {
            if let Some(k) = key_nav(&ev) {
                return nav(s, k);
            }
        }
        Message::Hover(id) => s.sel = id,
        Message::Do(id) => press(&id, 0),
        _ => {}
    }
    Task::none()
}

fn press(id: &str, dir: i32) {
    let send = |what: &str, arg: &str| link::send(json!({"op": "quickDo", "what": what, "arg": arg}));
    match id {
        "volume" => send(if dir < 0 { "volDown" } else if dir > 0 { "volUp" } else { "mute" }, ""),
        "brightness" => send(if dir < 0 { "brightDown" } else { "brightUp" }, ""),
        "power-saver" | "balanced" | "performance" => send("profile", id),
        other => send(other, ""),
    }
}

fn nav(s: &mut Q, key: &str) -> Task<Message> {
    let rows = nav_rows(&s.st);
    let (mut r, mut c) = rows.iter().enumerate().find_map(|(r, row)| row.iter().position(|x| *x == s.sel).map(|c| (r, c))).unwrap_or((0, 0));
    let slider = matches!(s.sel.as_str(), "volume" | "brightness");
    match key {
        "up" if r > 0 => r -= 1,
        "down" if r + 1 < rows.len() => r += 1,
        "left" if slider => press(&s.sel, -1),
        "right" if slider => press(&s.sel, 1),
        "left" if c > 0 => c -= 1,
        "right" => c += 1,
        "a" => press(&s.sel, 0),
        "b" => link::send(json!({"op": "quick", "arg": "off"})),
        _ => {}
    }
    if let Some(row) = rows.get(r) {
        c = c.min(row.len().saturating_sub(1));
        s.sel = row[c].clone();
    }
    Task::none()
}

fn round_btn<'a>(id: &'static str, g: Option<G>, label: &'static str, sel: &str) -> Element<'a, Message> {
    let on = sel == id;
    let inner: Element<Message> = match g {
        Some(g) => glyph(g, c_text(), 18.0, 1.8),
        None => ts(label, 13.0, c_text()).into(),
    };
    let b = container(inner)
        .center_x(Length::Fixed(if g.is_some() { 40.0 } else { 56.0 }))
        .center_y(Length::Fixed(40.0))
        .style(boxed(shade(0.76), 20.0, on.then_some((c_accent(), 2.0))));
    mouse_area(b).on_press(Message::Do(id.into())).on_enter(Message::Hover(id.into())).into()
}

fn tile<'a>(id: &'static str, g: G, label: &'a str, detail: String, lit: bool, sel: &str) -> Element<'a, Message> {
    let on = sel == id;
    let (bg, fg, sub) = if lit { (c_accent(), c_ink(), Pal::mix(c_ink(), c_accent(), 0.3)) } else { (shade(0.62), c_text(), c_muted()) };
    let icon = container(glyph(g, fg, 20.0, 1.8)).center_x(Length::Fixed(36.0)).center_y(Length::Fixed(36.0));
    let words = column![tb(label, 15.0, fg), text(detail).size(12.0).font(INTER).color(sub)].spacing(2.0).width(Length::Fill);
    let cell = container(row![icon, words].spacing(10.0).align_y(iced::Center))
        .padding(Padding { top: 0.0, right: 12.0, bottom: 0.0, left: 12.0 })
        .width(Length::FillPortion(1))
        .center_y(Length::Fixed(76.0))
        .style(boxed(bg, 14.0, on.then_some((c_text(), 2.0))));
    mouse_area(cell).on_press(Message::Do(id.into())).on_enter(Message::Hover(id.into())).into()
}

fn slider<'a>(id: &'static str, g: G, value: Option<i64>, muted: bool, sel: &str) -> Element<'a, Message> {
    let on = sel == id;
    let v = value.unwrap_or(0).clamp(0, 100) as f32;
    let fill = if muted { c_muted() } else { c_accent() };
    let bar = row![
        container(Space::new()).width(Length::FillPortion((v as u16).max(1))).height(Length::Fixed(8.0)).style(boxed(fill, 4.0, None)),
        container(Space::new()).width(Length::FillPortion((100 - v as u16).max(1))).height(Length::Fixed(8.0)).style(boxed(shade(0.9), 4.0, None)),
    ]
    .spacing(0.0);
    let step = |dir: i32, label: &'static str| {
        mouse_area(container(tb(label, 18.0, c_text())).center_x(Length::Fixed(32.0)).center_y(Length::Fixed(32.0)).style(boxed(shade(0.76), 16.0, None)))
            .on_press(Message::Do(format!("{id}:{dir}")))
    };
    let pct = match value {
        Some(_) if muted => "Muted".to_string(),
        Some(x) => format!("{x}%"),
        None => "Not here".to_string(),
    };
    let icon = mouse_area(glyph(g, c_text(), 20.0, 1.8)).on_press(Message::Do(id.into()));
    let line = row![icon, step(-1, "−"), container(bar).width(Length::Fill).center_y(Length::Fixed(32.0)), step(1, "+"), container(t(pct, 13.0, c_muted())).width(Length::Fixed(58.0))]
        .spacing(10.0)
        .align_y(iced::Center);
    let cell = container(line).padding(Padding { top: 0.0, right: 12.0, bottom: 0.0, left: 14.0 }).width(Length::Fill).center_y(Length::Fixed(52.0)).style(boxed(shade(0.55), 26.0, on.then_some((c_accent(), 2.0))));
    mouse_area(cell).on_enter(Message::Hover(id.into())).into()
}

fn view(s: &Q) -> Element<'_, Message> {
    let q = &s.st.quick;
    let sel = s.sel.as_str();
    let avatar = container(tb(s.st.initial.clone(), 20.0, c_ink())).center_x(Length::Fixed(46.0)).center_y(Length::Fixed(46.0)).style(boxed(c_accent(), 23.0, None));
    let me = column![ts(q.user.clone(), 16.0, c_text()), t(q.line.clone(), 12.0, c_muted())].spacing(2.0).width(Length::Fill);
    let mut head = row![avatar, me, round_btn("settings", Some(G::Gear), "", sel)].spacing(10.0).align_y(iced::Center);
    if q.can_lock {
        head = head.push(round_btn("lock", None, "Lock", sel));
    }
    head = head.push(round_btn("power", Some(G::Power), "", sel));
    let mut col = column![head].spacing(14.0);
    if q.power_open {
        let b = |id: &'static str, label: &'static str| -> Element<'static, Message> {
            let on = s.sel == id;
            mouse_area(container(ts(label, 14.0, c_text())).center_x(Length::FillPortion(1)).center_y(Length::Fixed(44.0)).style(boxed(shade(0.76), 10.0, on.then_some((c_accent(), 2.0)))))
                .on_press(Message::Do(id.into()))
                .on_enter(Message::Hover(id.into()))
                .into()
        };
        col = col.push(row![b("sleep", "Sleep"), b("restart", "Restart"), b("shutdown", "Shut down")].spacing(10.0));
    }
    // Return to Game Mode: first and biggest (Mantel's).
    let game_on = sel == "game";
    let game = container(
        row![
            glyph(G::Pad, c_ink(), 28.0, 2.0),
            column![tb("Return to Game Mode", 17.0, c_ink()), t("Your apps stay open in Desktop Mode", 12.0, Pal::mix(c_ink(), c_accent(), 0.25))].spacing(1.0)
        ]
        .spacing(14.0)
        .align_y(iced::Center),
    )
    .padding(Padding { top: 0.0, right: 18.0, bottom: 0.0, left: 18.0 })
    .width(Length::Fill)
    .center_y(Length::Fixed(72.0))
    .style(boxed(c_accent(), 16.0, game_on.then_some((c_text(), 2.0))));
    col = col.push(mouse_area(game).on_press(Message::Do("game".into())).on_enter(Message::Hover("game".into())));

    let onoff = |v: Option<bool>, none: &str| match v {
        Some(true) => "On".to_string(),
        Some(false) => "Off".to_string(),
        None => none.to_string(),
    };
    let bt_detail = match q.bt {
        None => "No adapter".to_string(),
        Some(_) if !q.bt_line.is_empty() => q.bt_line.clone(),
        b => onoff(b, ""),
    };
    let tiles = column![
        row![tile("wifi", G::Wifi, "Wi-Fi", onoff(q.wifi, "Not available"), q.wifi == Some(true), sel), tile("bt", G::Bt, "Bluetooth", bt_detail, q.bt == Some(true), sel)].spacing(10.0),
        row![
            tile("dnd", G::Moon, "Do not disturb", if s.st.dnd { "On".into() } else { "Off".into() }, s.st.dnd, sel),
            tile("keyboard", G::Grid, "Keyboard", if s.st.osk { "On-screen · on".into() } else { "On-screen".into() }, s.st.osk, sel)
        ]
        .spacing(10.0),
        row![
            tile("away", G::Server, "Away", "Home server keeps running".into(), false, sel),
            tile("couch", G::Home, "Couch", "Big tiles for the controller".into(), s.st.any_couch(), sel)
        ]
        .spacing(10.0),
    ]
    .spacing(10.0);
    col = col.push(tiles);
    col = col.push(slider("volume", G::Sound, q.volume, q.muted, sel));
    if q.brightness.is_some() {
        col = col.push(slider("brightness", G::Display, q.brightness, false, sel));
    }
    if !q.profile.is_empty() {
        let mut r = row![].spacing(8.0);
        for (id, label) in [("power-saver", "Battery saver"), ("balanced", "Balanced"), ("performance", "Performance")] {
            let lit = q.profile == id;
            let on = sel == id;
            let (bg, fg) = if lit { (c_accent(), c_ink()) } else { (shade(0.62), c_text()) };
            r = r.push(
                mouse_area(container(ts(label, 13.0, fg)).center_x(Length::FillPortion(1)).center_y(Length::Fixed(44.0)).style(boxed(bg, 22.0, on.then_some((c_text(), 2.0)))))
                    .on_press(Message::Do(id.into()))
                    .on_enter(Message::Hover(id.into())),
            );
        }
        col = col.push(column![t("POWER PROFILE", 11.0, c_muted()), r].spacing(8.0));
    }
    // The notifications that came (newest first).
    if s.st.history.is_empty() {
        col = col.push(container(t("No notifications", 13.0, c_muted())).center_x(Length::Fill).center_y(Length::Fixed(40.0)));
    } else {
        let clear_on = sel == "clearNotes";
        let clear = mouse_area(container(ts("Clear all", 13.0, if clear_on { c_accent() } else { c_muted() })).padding(4.0)).on_press(Message::Do("clearNotes".into())).on_enter(Message::Hover("clearNotes".into()));
        let mut list = column![row![ts("Notifications", 14.0, c_text()), Space::new().width(Length::Fill), clear].align_y(iced::Center).height(Length::Fixed(30.0))].spacing(0.0);
        for n in s.st.history.iter().take(4) {
            let small = [n.app.clone(), n.at.clone()].into_iter().filter(|x| !x.is_empty()).collect::<Vec<_>>().join(" · ");
            let body: String = n.body.chars().take(70).collect();
            list = list.push(
                container(column![t(small, 11.0, c_muted()), ts(n.title.clone(), 14.0, c_text()), t(body, 12.0, soft(0.4))].spacing(2.0))
                    .padding(Padding { top: 6.0, right: 12.0, bottom: 6.0, left: 12.0 })
                    .width(Length::Fill)
                    .height(Length::Fixed(68.0))
                    .style(boxed(shade(0.55), 10.0, None)),
            );
            list = list.push(Space::new().height(Length::Fixed(4.0)));
        }
        col = col.push(list);
    }
    container(col).padding(24.0).width(Length::Fill).height(Length::Fill).style(boxed(shade(0.38), 12.0, Some((c_line(), 1.0)))).into()
}

fn update_do(s: &mut Q, m: Message) -> Task<Message> {
    // The sliders' − and + say which way: "volume:-1".
    if let Message::Do(id) = &m {
        if let Some((what, dir)) = id.split_once(':') {
            s.sel = what.to_string();
            press(what, dir.parse().unwrap_or(0));
            return Task::none();
        }
    }
    update(s, m)
}

fn subscription(_s: &Q) -> Subscription<Message> {
    Subscription::batch([Subscription::run(link::stream).map(Message::Hub), iced::keyboard::listen().map(Message::Key)])
}

pub fn run() -> i32 {
    link::set_surface("quick");
    iced_layershell::disable_clipboard();
    let layer_settings = LayerShellSettings {
        anchor: Anchor::Top | Anchor::Right,
        layer: Layer::Overlay,
        exclusive_zone: 0,
        size: Some((W, 700)),
        margin: (52, 12, 0, 0),
        keyboard_interactivity: KeyboardInteractivity::Exclusive,
        start_mode: link::start_mode(),
        ..Default::default()
    };
    let r = iced_layershell::application(Q::default, || "hearth-firehold-quick".to_string(), update_do, view)
        .style(|_s: &Q, _t: &iced::Theme| iced::theme::Style { background_color: Color::TRANSPARENT, text_color: c_text() })
        .subscription(subscription)
        .settings(Settings { layer_settings, default_font: INTER, ..Default::default() })
        .run();
    super::ended("quick", r)
}
