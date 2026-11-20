//! The Board (page 2b): columns made from every checklist and "Waiting:" line; each card names its note and tag and
//! is coloured by project; group by status, project, tag or date. Moving a card (drag it, or X) ticks its box or
//! edits its line in the note, saved like any other change.

use crate::look::{self, accent2, bx, click, f, pad, project_color, tone, z, E, MONO};
use crate::{App, Group, Msg};
use hearth_notes::board::{self, Card, Status, STATUSES};
use hearth_notes::day;
use hearth_ui::ui::{self, gap, glyph, t, t1, ts, G};
use hearth_ui::Pal;
use iced::widget::{column, container, mouse_area, row, scrollable, stack, text, Column, Row, Space};
use iced::{Color, Length, Task};

fn status_color(p: &Pal, s: Status) -> Color {
    match s {
        Status::Waiting => p.accent,
        Status::Todo => Color::from_rgb8(0x9a, 0x9f, 0xab),
        Status::Doing => look::WARN,
        Status::Done => look::GOOD,
    }
}

/// The columns for the grouping: (title, dot colour, the cards' places in `cards`, the status a drop moves to).
fn columns(a: &App, cards: &[Card]) -> Vec<(String, Color, Vec<usize>, Option<Status>)> {
    let p = a.p;
    match a.group {
        Group::Status => STATUSES.iter().map(|s| (s.title().to_string(), status_color(&p, *s), cards.iter().enumerate().filter(|(_, c)| c.status == *s).map(|(i, _)| i).collect(), Some(*s))).collect(),
        Group::Project => {
            let mut keys: Vec<String> = cards.iter().map(|c| if c.project == "log" { "daily logs".to_string() } else if c.project.is_empty() { "other files".into() } else { c.project.clone() }).collect();
            keys.sort();
            keys.dedup();
            keys.into_iter().take(6).map(|k| {
                let col = project_color(&p, if k == "daily logs" { "log" } else { &k });
                let idx = cards.iter().enumerate().filter(|(_, c)| (if c.project == "log" { "daily logs" } else if c.project.is_empty() { "other files" } else { c.project.as_str() }) == k).map(|(i, _)| i).collect();
                (k, col, idx, None)
            }).collect()
        }
        Group::Tag => {
            let mut keys: Vec<(String, usize)> = Vec::new();
            for c in cards {
                let k = if c.tag.is_empty() { "no tag".to_string() } else { format!("#{}", c.tag) };
                match keys.iter_mut().find(|x| x.0 == k) {
                    Some(x) => x.1 += 1,
                    None => keys.push((k, 1)),
                }
            }
            keys.sort_by(|x, y| y.1.cmp(&x.1));
            keys.into_iter().take(5).map(|(k, _)| {
                let idx = cards.iter().enumerate().filter(|(_, c)| (if c.tag.is_empty() { "no tag".to_string() } else { format!("#{}", c.tag) }) == k).map(|(i, _)| i).collect();
                (k, p.accent, idx, None)
            }).collect()
        }
        Group::Date => {
            let mut days: Vec<day::Ymd> = cards.iter().map(|c| c.day).collect();
            days.sort_by(|x, y| y.cmp(x));
            days.dedup();
            days.into_iter().take(5).map(|d| (day::short(d), p.accent, cards.iter().enumerate().filter(|(_, c)| c.day == d).map(|(i, _)| i).collect(), None)).collect()
        }
    }
}

pub fn view(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let cards = a.cards();
    let notes: std::collections::BTreeSet<usize> = cards.iter().map(|c| c.note).collect();
    let g = a.group;
    let group = look::seg(&p, vec![("Status".into(), g == Group::Status, Msg::BoardGroup(Group::Status)), ("Project".into(), g == Group::Project, Msg::BoardGroup(Group::Project)), ("Tag".into(), g == Group::Tag, Msg::BoardGroup(Group::Tag)), ("Date".into(), g == Group::Date, Msg::BoardGroup(Group::Date))], false);
    let pick = |label: String, n: u8| -> E<'static, Msg> { click(container(t1(format!("{label} ▾"), f(14.0), Pal::mix(tn.text, tn.muted, 0.15))).padding(pad(z(5.0), z(14.0), z(5.0), z(14.0))).style(bx(Color::TRANSPARENT, z(14.0), Some((tn.line, 1.0)))), Some(Msg::BoardMenu(n))) };
    let head = row![
        t1("Board · every ", f(15.0), tn.muted),
        iced::widget::text(" - [ ] ").size(f(14.0)).font(MONO).color(tn.text),
        t1(" and \"Waiting:\" line in the workspace", f(15.0), tn.muted),
        Space::new().width(Length::Fill),
        t1("Group by", f(15.0), tn.muted),
        gap(z(12.0), 1.0),
        group,
        gap(z(14.0), 1.0),
        pick(a.board_project.clone().unwrap_or_else(|| "All projects".into()), 1),
        gap(z(10.0), 1.0),
        pick(a.board_tag.clone().map(|x| format!("#{x}")).unwrap_or_else(|| "All tags".into()), 2),
    ]
    .align_y(iced::Center);
    let mut cols = Row::new().spacing(z(22.0)).height(Length::Fill);
    for (ci, (title, dot, idx, status)) in columns(a, &cards).into_iter().enumerate() {
        let mut c = Column::new().spacing(z(10.0)).push(row![look::dot(dot, z(10.0)), gap(z(10.0), 1.0), ts(title, f(19.0), tn.text), Space::new().width(Length::Fill), t(idx.len().to_string(), f(15.0), tn.muted)].align_y(iced::Center));
        for i in idx {
            c = c.push(card(a, &cards[i], i));
        }
        if let Some(st) = status {
            c = c.push(click(t("+ Add a card", f(15.0), tn.muted), Some(Msg::AddCard(st))));
        }
        let colbox = container(scrollable(c).height(Length::Fill)).padding(pad(z(16.0), z(14.0), z(16.0), z(14.0))).width(Length::Fill).height(Length::Fill).style(bx(Pal::mix(tn.page, tn.side, 0.5), 10.0, Some((tn.line, 1.0))));
        let drop: E<Msg> = if status.is_some() { mouse_area(colbox).on_release(Msg::CardDrop(ci)).into() } else { colbox.into() };
        cols = cols.push(drop);
    }
    let (sc, sl) = a.sync_line();
    let foot = row![
        t1(format!("{} card{} from {} note{}", cards.len(), crate::plural(cards.len()), notes.len(), crate::plural(notes.len())), f(14.0), tn.muted),
        gap(z(30.0), 1.0),
        t1("Moving a card ticks its box or edits its line in the note", f(14.0), tn.muted),
        Space::new().width(Length::Fill),
        look::dot(sc, z(9.0)),
        gap(z(8.0), 1.0),
        t1(if sl.starts_with("Up to date") { "Up to date with GitHub".to_string() } else { sl }, f(14.0), tn.muted),
    ]
    .align_y(iced::Center);
    let body = column![container(head).padding(pad(z(12.0), z(28.0), z(12.0), z(28.0))), container(cols).padding(pad(0.0, z(26.0), z(14.0), z(26.0))).height(Length::Fill), ui::hline(tn.line), container(foot).padding(pad(0.0, z(28.0), 0.0, z(28.0))).center_y(Length::Fixed(z(42.0)))];
    let page: E<Msg> = column![crate::editor::tabs(a), body].into();
    if a.board_menu > 0 {
        stack![page, menu(a)].into()
    } else {
        page
    }
}

fn card<'a>(a: &'a App, c: &Card, i: usize) -> E<'a, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let done = c.status == Status::Done;
    let focused = i == a.card_focus && (a.pad || !a.focus.is_empty() || crate::app_flag_focus());
    let b = container(if done { glyph(G::Check, p.ink, z(14.0), 3.0) } else { gap(1.0, 1.0) }).center_x(Length::Fixed(z(22.0))).center_y(Length::Fixed(z(22.0))).style(bx(if done { p.accent } else { Color::TRANSPARENT }, 4.0, if done { None } else { Some((tn.muted, 1.2)) }));
    let txt: E<Msg> = if done {
        stack![t(c.text.clone(), f(16.0), tn.muted), container(ui::hline(tn.muted)).height(Length::Fill).center_y(Length::Fill)].into()
    } else {
        t(c.text.clone(), f(16.0), tn.text).into()
    };
    let n = &a.vault.notes[c.note];
    let from = if n.rel.ends_with("/README.md") { n.rel.trim_end_matches("/README.md").to_string() } else { n.rel.trim_end_matches(".md").to_string() };
    let tag: E<Msg> = if c.tag.is_empty() { gap(1.0, 1.0) } else { t1(format!("#{}", c.tag), f(13.0), accent2(&p)).into() };
    let meta = row![text(from).size(f(12.5)).font(MONO).color(tn.muted).wrapping(text::Wrapping::None), gap(z(12.0), 1.0), tag].align_y(iced::Center);
    let inner = column![row![click(b, Some(Msg::CardTick(i))), gap(z(14.0), 1.0), container(txt).width(Length::Fill)].align_y(iced::Top), row![gap(z(36.0), 1.0), meta]].spacing(z(6.0));
    let stripe = container(Space::new()).width(Length::Fixed(3.0)).height(Length::Fill).style(bx(project_color(&p, if c.project.is_empty() { "?" } else { &c.project }), 2.0, None));
    let body = container(row![stripe, container(inner).padding(pad(z(14.0), z(16.0), z(14.0), z(14.0))).width(Length::Fill)].height(Length::Shrink)).width(Length::Fill).style(bx(tn.card, 8.0, Some((tn.line, 1.0)))).clip(true);
    let ma = mouse_area(body).on_press(Msg::CardDrag(i)).on_double_click(Msg::CardOpen(i)).interaction(iced::mouse::Interaction::Grab);
    look::ring(&p, focused, 8.0, ma)
}

fn menu(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let mut c = Column::new().spacing(2.0);
    let item = |label: String, on: bool, m: Msg| -> E<'static, Msg> { click(container(t1(label, f(15.0), if on { accent2(&p) } else { tn.text })).padding(pad(z(6.0), z(14.0), z(6.0), z(14.0))).width(Length::Fill), Some(m)) };
    if a.board_menu == 1 {
        c = c.push(item("All projects".into(), a.board_project.is_none(), Msg::BoardProject(None)));
        let counts = a.vault.project_counts();
        for (pj, n) in counts.iter() {
            if pj == "home" || pj == "tools" || pj.is_empty() {
                continue;
            }
            let label = if pj == "log" { format!("daily logs ({n})") } else if *n == 0 { format!("{pj} (no notes yet)") } else { format!("{pj} ({n})") };
            c = c.push(item(label, a.board_project.as_deref() == Some(pj), Msg::BoardProject(Some(pj.clone()))));
        }
    } else {
        c = c.push(item("All tags".into(), a.board_tag.is_none(), Msg::BoardTag(None)));
        for (tg, n) in a.vault.tag_counts().into_iter().take(12) {
            c = c.push(item(format!("#{tg} ({n})"), a.board_tag.as_deref() == Some(tg.as_str()), Msg::BoardTag(Some(tg))));
        }
    }
    let catcher = mouse_area(Space::new().width(Length::Fill).height(Length::Fill)).on_press(Msg::BoardMenu(0));
    let x = if a.board_menu == 1 { z(1140.0) } else { z(1280.0) };
    stack![catcher, iced::widget::pin(container(scrollable(c)).padding(z(6.0)).width(Length::Fixed(z(300.0))).max_height(z(560.0)).style(bx(tn.card, 8.0, Some((tn.line, 1.0))))).x(x).y(z(110.0))].into()
}

/// Moves card `i` (of the Board's list) to `to`: its line edited in its note and saved.
pub fn move_card(a: &mut App, i: usize, to: Status) -> Task<Msg> {
    let cards = a.cards();
    let Some(c) = cards.get(i).cloned() else { return Task::none() };
    if c.status == to {
        return Task::none();
    }
    let rel = a.vault.notes[c.note].rel.clone();
    let open = a.doc.as_ref().map(|d| d.rel == rel).unwrap_or(false);
    let text = if open { a.doc.as_ref().map(|d| d.text()).unwrap_or_default() } else { a.vault.notes[c.note].text.clone() };
    let new = board::move_in(&text, c.line, c.task, to);
    if new == text {
        return Task::none();
    }
    a.vault.update(&rel, new.clone(), a.now);
    if open {
        if let Some(d) = a.doc.as_mut() {
            let nd = crate::Doc::from_text(&rel, &new);
            d.lines = nd.lines;
            d.trailing_nl = nd.trailing_nl;
        }
        return a.edited();
    }
    crate::save_task_pub(a.root.clone(), rel, new)
}

/// "+ Add a card": a new line in today's log, ready to type.
pub fn add_card(a: &mut App, st: Status) -> Task<Msg> {
    let line = match st {
        Status::Waiting => "- [ ] Waiting: ",
        Status::Todo => "- [ ] ",
        Status::Doing => "- [/] ",
        Status::Done => "- [x] ",
    };
    let t = a.rail(crate::Page::Today);
    let rel = a.today_rel();
    let text = a.doc.as_ref().filter(|d| d.rel == rel).map(|d| d.text()).unwrap_or_default();
    let new = hearth_notes::newitem::log_with(if text.trim().is_empty() { None } else { Some(&text) }, &day::ymd(a.today), line);
    let mut d = crate::Doc::from_text(&rel, &new);
    d.cur = Some(d.lines.len().saturating_sub(1));
    a.doc = Some(d);
    a.page = crate::Page::Files;
    a.mode = crate::Mode::Live;
    let e = a.edited();
    Task::batch([t, e, iced::widget::operation::focus(crate::LINE_ID), iced::widget::operation::move_cursor_to_end(crate::LINE_ID)])
}
