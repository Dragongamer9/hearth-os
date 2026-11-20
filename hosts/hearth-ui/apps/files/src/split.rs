//! Hearth Files' split view (F9, like Dolphin's Split; Mantel's FilesSplitPane.qml): a second folder beside the one
//! you're in, to copy or move between them. It starts in the same folder; double-click (or Enter) opens a folder in
//! it, Backspace goes up, and "Copy here" / "Move here" put what's selected on the left into this folder (Files' own
//! copy and move, with Undo). Drop files on it to copy or move them there. Middle-click a folder to open it on the left.

use crate::app::{App, Msg, Zone};
use crate::backend::{self, Reply};
use crate::icons::{self, ico, Ic};
use crate::model::{self, parent_of, Entry};
use hearth_ui::ui::{self, t1};
use hearth_ui::Pal;
use iced::widget::{button, column, container, mouse_area, row, scrollable, Column, Space};
use iced::{mouse, Background, Border, Color, Element, Length, Padding, Shadow, Task, Theme};

#[derive(Debug, Clone, Default)]
pub struct Split {
    pub path: String,
    pub entries: Vec<Entry>,
    pub error: String,
    pub cur: usize,
    pub seq: u64,
    pub last_click: Option<(usize, std::time::Instant)>,
}

#[derive(Debug, Clone)]
pub enum SMsg {
    Toggle,
    Close,
    Listed(u64, Reply),
    Up,
    Press(usize),
    Middle(usize),
    CopyHere,
    MoveHere,
}

pub fn load(a: &mut App, p: &str) -> Task<Msg> {
    let Some(s) = a.split.as_mut() else { return Task::none() };
    if !p.is_empty() {
        s.path = p.to_string();
    }
    s.seq += 1;
    let seq = s.seq;
    let path = s.path.clone();
    Task::perform(backend::call(backend::a(&["list", &path]), 30), move |r| Msg::Split(SMsg::Listed(seq, r)))
}

pub fn reload(a: &mut App) -> Task<Msg> {
    if a.split.is_some() { load(a, "") } else { Task::none() }
}

fn activate(a: &mut App, i: usize) -> Task<Msg> {
    let Some(e) = a.split.as_ref().and_then(|s| s.entries.get(i).cloned()) else { return Task::none() };
    if e.dir {
        return load(a, &e.path);
    }
    Task::perform(backend::call(backend::a(&["open", &e.path]), 20), |r| Msg::Did(crate::app::Then::Nothing, r))
}

pub fn update(a: &mut App, m: SMsg) -> Task<Msg> {
    match m {
        SMsg::Toggle => {
            if a.split.is_some() {
                a.split = None;
                if a.zone == Zone::Split {
                    a.zone = Zone::Main;
                }
                return Task::none();
            }
            let start = if a.path.starts_with('/') { a.path.clone() } else { a.home.clone() };
            a.split = Some(Split { path: start, ..Default::default() });
            return load(a, "");
        }
        SMsg::Close => {
            a.split = None;
            if a.zone == Zone::Split {
                a.zone = Zone::Main;
            }
        }
        SMsg::Listed(seq, r) => {
            let Some(s) = a.split.as_mut() else { return Task::none() };
            if seq != s.seq {
                return Task::none();
            }
            if !r.ok() {
                s.error = r.err.clone();
                s.entries.clear();
                return Task::none();
            }
            s.error.clear();
            let l = model::Listing::from_json(&r.value);
            if !l.path.is_empty() {
                s.path = l.path.clone();
            }
            let mut v = l.entries;
            v.sort_by(|x, y| y.dir.cmp(&x.dir).then_with(|| model::natural(&x.name, &y.name)));
            s.entries = v;
            s.cur = 0;
        }
        SMsg::Up => {
            let p = a.split.as_ref().map(|s| parent_of(&s.path)).unwrap_or_default();
            return load(a, &p);
        }
        SMsg::Press(i) => {
            a.zone = Zone::Split;
            let now = std::time::Instant::now();
            let Some(s) = a.split.as_mut() else { return Task::none() };
            let double = s.last_click.map(|(j, t)| j == i && now.duration_since(t).as_millis() < 450).unwrap_or(false);
            s.cur = i;
            s.last_click = if double { None } else { Some((i, now)) };
            if double {
                return activate(a, i);
            }
            // A drag from here (onto a folder on the left, a place, a tab) starts like one from the folder.
            if let Some(e) = a.split.as_ref().and_then(|s| s.entries.get(i).cloned()) {
                crate::dnd::pending(a, vec![e.path.clone()], &e.name, true);
            }
        }
        SMsg::Middle(i) => {
            if let Some(e) = a.split.as_ref().and_then(|s| s.entries.get(i).cloned()) {
                if e.dir {
                    return a.navigate(&e.path, "");
                }
            }
        }
        SMsg::CopyHere | SMsg::MoveHere => {
            let Some(dest) = a.split.as_ref().map(|s| s.path.clone()) else { return Task::none() };
            let t = a.targets();
            if t.is_empty() {
                return a.say("Pick something on the left first", false);
            }
            return a.transfer_to(if matches!(m, SMsg::CopyHere) { "copy" } else { "move" }, t, dest);
        }
    }
    Task::none()
}

/// The keys the split view takes while it's the one you're in (from menus.rs's key handler).
pub fn key(a: &mut App, named: Option<iced::keyboard::key::Named>) -> Option<Task<Msg>> {
    use iced::keyboard::key::Named;
    let s = a.split.as_mut()?;
    let n = s.entries.len();
    match named {
        Some(Named::ArrowUp) => {
            s.cur = s.cur.saturating_sub(1);
            Some(Task::none())
        }
        Some(Named::ArrowDown) => {
            s.cur = (s.cur + 1).min(n.saturating_sub(1));
            Some(Task::none())
        }
        Some(Named::Enter) => {
            let i = s.cur;
            Some(activate(a, i))
        }
        Some(Named::Backspace) => Some(update(a, SMsg::Up)),
        Some(Named::ArrowLeft) | Some(Named::Escape) => {
            a.zone = Zone::Main;
            Some(Task::none())
        }
        _ => None,
    }
}

fn chip<'a>(p: &Pal, label: String, on: bool, msg: Msg) -> Element<'a, Msg> {
    let p = *p;
    button(container(t1(label, 12.0, if on { p.text } else { p.muted })).center_y(Length::Fixed(20.0)))
        .padding(Padding { top: 4.0, right: 11.0, bottom: 4.0, left: 11.0 })
        .on_press_maybe(if on { Some(msg) } else { None })
        .style(move |_: &Theme, st: button::Status| button::Style {
            background: Some(Background::Color(if !on {
                Color::TRANSPARENT
            } else if matches!(st, button::Status::Hovered | button::Status::Pressed) {
                p.panel3
            } else {
                p.panel2
            })),
            text_color: p.text,
            border: Border { color: p.line, width: 1.0, radius: p.r(6.0).into() },
            shadow: Shadow::default(),
            snap: true,
        })
        .into()
}

pub fn view(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    let Some(s) = &a.split else { return Space::new().into() };
    let sel = a.targets().len();
    let head = row![
        mouse_area(ico::<Msg>(Ic::Up, p.text, 16.0)).on_press(Msg::Split(SMsg::Up)).interaction(mouse::Interaction::Pointer),
        container(t1(ui::fit(&s.path, 48), 13.0, p.text).font(ui::weight(iced::font::Weight::Semibold))).width(Length::Fill).clip(true),
        mouse_area(t1("✕", 13.0, p.muted)).on_press(Msg::Split(SMsg::Close)).interaction(mouse::Interaction::Pointer),
    ]
    .spacing(8.0)
    .align_y(iced::Center);
    let n = if sel > 0 { format!(" ({sel})") } else { String::new() };
    let tools = row![
        chip(&p, format!("Copy here{n}"), sel > 0, Msg::Split(SMsg::CopyHere)),
        chip(&p, format!("Move here{n}"), sel > 0, Msg::Split(SMsg::MoveHere)),
        Space::new().width(Length::Fill),
        t1(model::plural(s.entries.len(), "item", "items"), 11.0, p.muted),
    ]
    .spacing(6.0)
    .align_y(iced::Center);
    let mut list = Column::new().spacing(0.0);
    let lit_zone = a.zone == Zone::Split;
    let target = crate::dnd::target(a);
    for (i, e) in s.entries.iter().enumerate() {
        let on = i == s.cur;
        let over = e.dir && target.as_deref() == Some(e.path.as_str());
        let mut r = row![
            ico::<Msg>(icons::file_icon(model::kind_of(e), &e.name), p.text, 16.0),
            container(t1(e.name.clone(), 13.0, p.text)).width(Length::Fill).clip(true),
        ]
        .spacing(8.0)
        .align_y(iced::Center)
        .padding(Padding { top: 0.0, right: 6.0, bottom: 0.0, left: 6.0 });
        if !e.dir {
            r = r.push(t1(model::fmt_size(e.size), 11.0, p.muted));
        }
        let bg = if on { Pal::a(p.accent, if lit_zone { 0.24 } else { 0.14 }) } else { Color::TRANSPARENT };
        let bc = if over { p.accent } else { Color::TRANSPARENT };
        let cell = container(r).width(Length::Fill).height(Length::Fixed(30.0)).center_y(Length::Fixed(30.0)).style(ui::boxed(bg, p.r(6.0), Some((bc, if over { 1.5 } else { 0.0 }))));
        let mut m = mouse_area(cell).on_press(Msg::Split(SMsg::Press(i))).on_middle_press(Msg::Split(SMsg::Middle(i)));
        if a.drag.is_some() && e.dir {
            m = m.on_enter(Msg::Dnd(crate::dnd::DMsg::Over(0, e.path.clone()))).on_exit(Msg::Dnd(crate::dnd::DMsg::Out(0, e.path.clone())));
        }
        list = list.push(m);
    }
    let mut col = column![head, tools, scrollable(list).height(Length::Fill)].spacing(6.0).padding(Padding { top: 8.0, right: 8.0, bottom: 8.0, left: 10.0 });
    if !s.error.is_empty() {
        col = col.push(t1(s.error.clone(), 12.0, p.danger));
    }
    let over_pane = a.drag.is_some() && target.as_deref() == Some(s.path.as_str());
    let body = container(col).width(Length::Fill).height(Length::Fill).style(ui::boxed(if over_pane { Pal::mix(p.panel, p.accent, 0.08) } else { p.panel }, 0.0, None));
    let mut m = mouse_area(body);
    if a.drag.is_some() {
        m = m.on_enter(Msg::Dnd(crate::dnd::DMsg::Over(1, s.path.clone()))).on_exit(Msg::Dnd(crate::dnd::DMsg::Out(1, s.path.clone())));
    }
    m.into()
}
