//! The overview (Overview.qml; docs/mockups/desktop-layouts/layouts-3-gnome-style-overview.png), opened by the
//! desktops pill and Task view: "Type to search", the desktop you're on with its windows side by side and their
//! names under them, the previous and next desktops at the sides, and the dash (your pinned apps, a line, the app
//! grid). Under the main bar, which stays. Esc or a click outside closes it; Enter opens the first window found, or
//! searches the app menu when no window has that name.

use iced::widget::{button, column, container, mouse_area, row, stack, Column, Row, Space};
use iced::{border, keyboard, Alignment, Background, Border, Element, Length, Padding};

use crate::look::{self, a};
use crate::shell::{Message, Shell};

#[derive(Debug, Clone, Default)]
pub struct State {
    pub q: String,
}

pub enum KeyResult {
    None,
    Close,
    Search(String),
    Open(u32),
}

fn shown<'a>(s: &'a Shell) -> Vec<&'a crate::wm::Win> {
    let q = s.overview.q.to_lowercase();
    s.wm.wins.iter().filter(|w| !(w.title.is_empty() && w.app_id.is_empty()) && !w.app_id.contains("hearth-mantel")).filter(|w| q.is_empty() || format!("{} {}", w.title, w.app_id).to_lowercase().contains(&q)).collect()
}

pub fn key(s: &mut Shell, e: keyboard::Event) -> KeyResult {
    use keyboard::key::Named;
    let keyboard::Event::KeyPressed { key, text, .. } = e else { return KeyResult::None };
    match key.as_ref() {
        keyboard::Key::Named(Named::Escape) => return KeyResult::Close,
        keyboard::Key::Named(Named::Backspace) => {
            s.overview.q.pop();
        }
        keyboard::Key::Named(Named::Enter) => {
            if let Some(w) = shown(s).first() {
                return KeyResult::Open(w.id);
            }
            if !s.overview.q.is_empty() {
                return KeyResult::Search(s.overview.q.clone());
            }
        }
        _ => {
            if let Some(t) = text {
                if t.chars().all(|c| !c.is_control()) {
                    s.overview.q.push_str(&t);
                }
            }
        }
    }
    KeyResult::None
}

pub fn view(s: &Shell) -> Element<'_, Message> {
    let l = &s.look;
    let p = l.p;
    let (bg, line, panel, accent) = (p.bg, p.line, p.panel, p.accent);
    // (the surface starts under a bar along the top: shell.rs gives it that margin)
    let top = 0.0;
    let r23 = l.r(23.0);
    let has_q = !s.overview.q.is_empty();

    // "Type to search"
    let q = &s.overview.q;
    let search = container(row![look::icon("search", 18.0, accent), if q.is_empty() { l.t("Type to search", 15.0, p.muted) } else { l.t(q.clone(), 15.0, p.text) }].spacing(12).align_y(Alignment::Center))
        .padding(Padding { left: 20.0, right: 20.0, ..Padding::ZERO })
        .width(Length::Fixed(600.0))
        .height(Length::Fixed(46.0))
        .align_y(Alignment::Center)
        .style(move |_| iced::widget::container::Style {
            background: Some(a(p.bg2, 0.9).into()),
            border: Border { color: if has_q { accent } else { line }, width: 1.0, radius: r23.into() },
            ..Default::default()
        });

    // the windows, side by side
    let wins = shown(s);
    let mut cards: Vec<Element<'_, Message>> = Vec::new();
    for w in wins.iter().take(9) {
        let app = crate::apps::for_window(&s.apps, &w.app_id);
        let name = app.map(|a| a.name.clone()).unwrap_or_else(|| if w.app_id.is_empty() { w.title.clone() } else { w.app_id.clone() });
        let lit = w.active;
        let card = container(column![
            container(l.t(hearth_ui::ui::fit(&w.title, 48), 14.0, p.text)).center_x(Length::Fill).height(Length::Fixed(40.0)).align_y(Alignment::Center).style(move |_| iced::widget::container::Style { background: Some(panel.into()), ..Default::default() }),
            container(look::app_pic(app.and_then(|a| a.icon_path.as_deref()), 64.0)).center(Length::Fill),
        ])
        .width(Length::Fixed(400.0))
        .height(Length::Fixed(264.0))
        .style(move |_| iced::widget::container::Style { background: Some(p.bg2.into()), border: Border { color: if lit { accent } else { line }, width: 1.0, radius: 10.0.into() }, ..Default::default() });
        let id = w.id;
        cards.push(column![button(card).padding(0).on_press(Message::OverviewOpen(id)).style(|_, _| button::Style::default()), l.t(name, 14.0, p.muted)].spacing(18).into());
    }
    let mut grid = Column::new().spacing(40).align_x(Alignment::Center);
    let mut it = cards.into_iter().peekable();
    while it.peek().is_some() {
        let mut r = Row::new().spacing(50);
        for _ in 0..3 {
            if let Some(c) = it.next() {
                r = r.push(c);
            }
        }
        grid = grid.push(r);
    }
    let empty = wins.is_empty();
    let stage_inner: Element<'_, Message> = if empty {
        l.t(if q.is_empty() { "No windows open".to_string() } else { format!("No window called “{q}”: Enter searches your apps") }, 15.0, p.muted).into()
    } else {
        grid.into()
    };
    let stage = container(stage_inner).center(Length::Fill).style(move |_| iced::widget::container::Style { background: Some(a(panel, 0.45).into()), border: border::rounded(24.0), ..Default::default() });
    let at = s.wm.desks.iter().position(|d| d.active).unwrap_or(0);
    let side_desk = |on: bool| -> Element<'_, Message> {
        if on {
            container(Space::new()).width(Length::Fixed(220.0)).height(Length::FillPortion(8)).style(move |_| iced::widget::container::Style { background: Some(a(panel, 0.35).into()), border: border::rounded(18.0), ..Default::default() }).into()
        } else {
            Space::new().width(Length::Fixed(220.0)).into()
        }
    };
    let middle = row![
        container(side_desk(at > 0)).center_y(Length::Fill),
        stage,
        container(side_desk(at + 1 < s.wm.desks.len())).center_y(Length::Fill),
    ]
    .spacing(30)
    .height(Length::Fill);

    // the dash: your pinned apps, a line, the app grid
    let mut dash = Row::new().spacing(8).align_y(Alignment::Center);
    for id in s.pins.iter().take(10) {
        if let Some(app) = crate::apps::find(&s.apps, id) {
            let app2 = app.clone();
            dash = dash.push(button(container(look::app_pic(app.icon_path.as_deref(), 44.0)).center(Length::Fixed(56.0))).padding(0).on_press(Message::OverviewLaunch(app2.id.clone())).style(move |_, st| button::Style {
                background: matches!(st, button::Status::Hovered | button::Status::Pressed).then(|| Background::Color(p.panel3)),
                border: border::rounded(14.0),
                ..button::Style::default()
            }));
        }
    }
    dash = dash.push(container(Space::new().width(Length::Fixed(1.0)).height(Length::Fixed(44.0))).style(move |_| iced::widget::container::Style { background: Some(line.into()), ..Default::default() }));
    dash = dash.push(button(container(look::icon("grid", 26.0, p.text)).center(Length::Fixed(56.0))).padding(0).on_press(Message::OverviewApps).style(move |_, st| button::Style {
        background: Some(Background::Color(if matches!(st, button::Status::Hovered | button::Status::Pressed) { p.panel3 } else { a(p.text, 0.06) })),
        border: border::rounded(14.0),
        ..button::Style::default()
    }));
    let dash = container(dash).padding(10).style(move |_| iced::widget::container::Style { background: Some(a(panel, 0.85).into()), border: Border { color: line, width: 1.0, radius: 20.0.into() }, ..Default::default() });

    let body = column![
        Space::new().height(Length::Fixed(top + 66.0)),
        container(search).center_x(Length::Fill),
        Space::new().height(Length::Fixed(40.0)),
        container(middle).padding(Padding { left: 24.0, right: 24.0, ..Padding::ZERO }).height(Length::Fill),
        Space::new().height(Length::Fixed(30.0)),
        container(dash).center_x(Length::Fill),
        Space::new().height(Length::Fixed(26.0)),
    ];
    let shade = container(Space::new()).width(Length::Fill).height(Length::Fill).style(move |_| iced::widget::container::Style { background: Some(a(bg, 0.98).into()), ..Default::default() });
    stack![mouse_area(shade).on_press(Message::OverviewClose), body].into()
}
