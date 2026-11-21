//! The controller layout (page 9): big focus rings, Today / Notes / Graph / Sync with LB and RB, the recent notes and
//! a reader, and a quick note to today's log typed on the on-screen keyboard (A types, B deletes, X space, Y shift,
//! LT the kind, RT adds it, ☰ closes the keyboard). The mouse layout's buttons work the same way with a controller.

use crate::look::{self, accent2, bx, click, f, pad, tone, z, E};
use crate::{pad, App, Msg, Page, PadPage};
use hearth_notes::{day, md};
use hearth_ui::ui::{gap, t, t1, tb, tm};
use hearth_ui::Pal;
use iced::widget::{column, container, row, Column, Row, Space};
use iced::{Color, Length, Task};

pub const KEYS: [&[&str]; 5] = [
    &["1", "2", "3", "4", "5", "6", "7", "8", "9", "0", "-", "⌫"],
    &["q", "w", "e", "r", "t", "y", "u", "i", "o", "p", "[", "]"],
    &["a", "s", "d", "f", "g", "h", "j", "k", "l", ";", "'", "⏎"],
    &["⇧", "z", "x", "c", "v", "b", "n", "m", ",", ".", "/", "⇧"],
    &["[[ link", "space", "#tag"],
];
const PAD_KINDS: [&str; 3] = ["Made", "Decided", "Waiting"];

/// The recent notes: today's log first, then the most opened.
pub fn recent(a: &App) -> Vec<usize> {
    let mut v: Vec<usize> = Vec::new();
    if let Some(i) = a.vault.find(&a.today_rel()) {
        v.push(i);
    }
    let mut by: Vec<(&String, &u64)> = a.opened.iter().collect();
    by.sort_by(|x, y| y.1.cmp(x.1));
    for (rel, _) in by {
        if let Some(i) = a.vault.find(rel) {
            if !v.contains(&i) {
                v.push(i);
            }
        }
        if v.len() >= 4 {
            break;
        }
    }
    let mut extra: Vec<usize> = (0..a.vault.notes.len()).filter(|i| !v.contains(i)).collect();
    extra.sort_by(|x, y| a.vault.notes[*y].mtime.cmp(&a.vault.notes[*x].mtime));
    for i in extra {
        if v.len() >= 4 {
            break;
        }
        v.push(i);
    }
    v
}

fn title_of(a: &App, i: usize) -> (String, String) {
    let n = &a.vault.notes[i];
    if n.log.is_some() && n.rel == a.today_rel() {
        return ("Today's log".into(), format!("{} · {} lines", n.rel, n.text.lines().filter(|l| !l.trim().is_empty() && !l.starts_with('#')).count()));
    }
    let title = n.title.trim_end_matches(" (mockups and plan)").to_string();
    let status = md::properties(&n.text).into_iter().find(|x| x.0 == "Status").map(|x| x.1.split_whitespace().next().unwrap_or("").to_lowercase()).unwrap_or_default();
    let dir = n.rel.rsplit('/').nth(2).map(|_| { let p: Vec<&str> = n.rel.split('/').collect(); p[..2].join("/") }).unwrap_or_else(|| n.rel.rsplit_once('/').map(|x| x.0.to_string()).unwrap_or_default());
    (title, if status.is_empty() { dir } else { format!("{dir} · {status}") })
}

pub fn view(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let tab = |name: &'static str, pg: PadPage| -> E<'static, Msg> {
        let on = a.pad_page == pg;
        let c = container(if on { tb(name, f(22.0), tn.text) } else { t(name, f(22.0), tn.muted) }).padding(pad(z(8.0), z(22.0), z(8.0), z(22.0))).style(bx(if on { Pal::mix(tn.card, tn.text, 0.06) } else { Color::TRANSPARENT }, z(22.0), if on { Some((Pal::a(p.accent, 0.8), 1.5)) } else { None }));
        click(c, Some(Msg::PadPage(pg)))
    };
    let btn = |s: &'static str| -> E<'static, Msg> { container(tb(s, f(12.0), tn.muted)).center_x(Length::Fixed(z(44.0))).center_y(Length::Fixed(z(28.0))).style(bx(Color::TRANSPARENT, z(14.0), Some((tn.muted, 1.5)))).into() };
    let (sc, sl) = a.sync_line();
    let c = a.central_now();
    let status = if sl.starts_with("Up to date") { format!("Up to date · {}", day::clock(c.hour, c.min)) } else { sl };
    let top = row![
        look::mark(&p, z(52.0)),
        gap(z(22.0), 1.0),
        tb("Hearth Notes", f(32.0), tn.text),
        gap(z(80.0), 1.0),
        btn("LB"),
        gap(z(14.0), 1.0),
        tab("Today", PadPage::Today),
        tab("Notes", PadPage::Notes),
        tab("Graph", PadPage::Graph),
        tab("Sync", PadPage::Sync),
        gap(z(14.0), 1.0),
        btn("RB"),
        Space::new().width(Length::Fill),
        look::dot(sc, z(10.0)),
        gap(z(10.0), 1.0),
        t1(status, f(17.0), Pal::mix(tn.text, tn.muted, 0.2)),
    ]
    .align_y(iced::Center);
    let body: E<Msg> = match a.pad_page {
        PadPage::Graph => container(crate::gview::sky(a, &a.g.mode, false, false)).height(Length::Fill).width(Length::Fill).style(bx(Color::BLACK, 12.0, None)).clip(true).into(),
        PadPage::Sync => {
            let s = &a.sync;
            let n = s.changes.len();
            column![
                tb(if n == 0 { "Up to date with GitHub".to_string() } else { format!("{n} change{} to push", crate::plural(n)) }, f(34.0), tn.text),
                t(if s.said.is_empty() { "X syncs now: it pulls, runs the workspace check, then pushes.".to_string() } else { s.said.clone() }, f(20.0), tn.muted),
            ]
            .spacing(z(14.0))
            .into()
        }
        _ => {
            let rec = recent(a);
            let mut list = Column::new().spacing(z(14.0)).push(tm("Recent notes", f(19.0), tn.muted));
            for (k, i) in rec.iter().enumerate() {
                let (ti, sub) = title_of(a, *i);
                let on = k == a.pad_note;
                let card = container(column![tb(ti, f(24.0), if k == 3 && !on { tn.muted } else { tn.text }), t1(sub, f(16.0), tn.muted)].spacing(z(4.0))).padding(pad(z(18.0), z(26.0), z(18.0), z(26.0))).width(Length::Fill).style(bx(if on { Pal::mix(tn.card, tn.text, 0.05) } else { Pal::a(tn.card, if k == 3 { 0.5 } else { 0.85 }) }, 12.0, None));
                list = list.push(look::ring(&p, on, 12.0, click(card, Some(Msg::PadNote(k)))));
            }
            let reader: E<Msg> = match rec.get(a.pad_note).or(rec.first()) {
                Some(i) => reader(a, *i),
                None => t("No notes yet", f(20.0), tn.muted).into(),
            };
            row![container(list).width(Length::FillPortion(9)), container(reader).padding(pad(z(26.0), z(36.0), z(26.0), z(36.0))).width(Length::FillPortion(15)).height(Length::Fill).style(bx(Pal::a(tn.card, 0.7), 14.0, None))].spacing(z(30.0)).height(Length::Fill).into()
        }
    };
    let mut page = column![container(top).padding(pad(z(26.0), z(76.0), z(14.0), z(76.0))), container(body).padding(pad(0.0, z(76.0), z(18.0), z(76.0))).height(Length::Fill)];
    if a.osk.open {
        page = page.push(quick_note(a));
    }
    page = page.push(hints(a));
    let bg = crate::look::grad(160.0, Color::from_rgb8(0x2a, 0x1d, 0x14), tn.page, 0.0, None);
    container(page).width(Length::Fill).height(Length::Fill).style(bg).into()
}

fn reader<'a>(a: &'a App, i: usize) -> E<'a, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let n = &a.vault.notes[i];
    let mut c = Column::new().spacing(z(12.0)).push(tb(n.title.clone(), f(34.0), tn.text));
    let mut count = 0;
    for l in n.text.lines() {
        if l.starts_with('#') || l.trim().is_empty() {
            continue;
        }
        let body = md::plain(l.trim().trim_start_matches("- ").trim_start_matches("* "));
        let (main, waiting) = match body.find(" Waiting: ") {
            Some(at) => (body[..at + 1].to_string(), Some(body[at + 1..].to_string())),
            None => (body.clone(), None),
        };
        let mut r = Row::new().push(container(look::dot(p.accent, z(14.0))).padding(pad(z(9.0), 0.0, 0.0, 0.0))).push(gap(z(18.0), 1.0));
        let mut words = Row::new().push(t(hearth_ui::ui::fit(&main, 160), f(20.0), tn.text));
        if let Some(w) = waiting {
            words = words.push(t(format!(" {}", hearth_ui::ui::fit(&w, 80)), f(20.0), tn.muted));
        }
        r = r.push(words.wrap());
        c = c.push(r);
        count += 1;
        if count >= 6 {
            break;
        }
    }
    c.into()
}

fn quick_note(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let mut kinds = Row::new().spacing(z(10.0));
    for (k, nm) in PAD_KINDS.iter().enumerate() {
        let on = k == a.osk.kind;
        kinds = kinds.push(click(container(t(*nm, f(15.0), if on { accent2(&p) } else { Pal::mix(tn.text, tn.muted, 0.2) })).padding(pad(z(4.0), z(14.0), z(4.0), z(14.0))).style(bx(if on { Pal::a(p.accent, 0.1) } else { Color::TRANSPARENT }, z(14.0), Some((if on { p.accent } else { tn.line }, 1.0)))), Some(Msg::OskKind(k))));
    }
    let tiny = |s: &'static str| -> E<'static, Msg> { container(tb(s, f(10.0), tn.muted)).center_x(Length::Fixed(z(40.0))).center_y(Length::Fixed(z(24.0))).style(bx(Color::TRANSPARENT, z(12.0), Some((tn.muted, 1.2)))).into() };
    let head = row![tb("Quick note to today's log", f(19.0), tn.text), gap(z(18.0), 1.0), kinds, Space::new().width(Length::Fill), tiny("LT"), t(" kind · ", f(14.0), tn.muted), tiny("RT"), t(" add", f(14.0), tn.muted)].align_y(iced::Center);
    let field = container(row![t(a.osk.text.clone(), f(24.0), tn.text), container(Space::new()).width(Length::Fixed(2.0)).height(Length::Fixed(z(34.0))).style(bx(p.accent, 1.0, None))].align_y(iced::Center)).padding(pad(z(14.0), z(22.0), z(14.0), z(22.0))).width(Length::Fill).style(bx(Pal::mix(tn.page, tn.card, 0.4), 10.0, Some((p.accent, 2.0))));
    let mut grid = Column::new().spacing(z(10.0));
    for (r, keys) in KEYS.iter().enumerate() {
        let mut rw = Row::new().spacing(z(10.0));
        for (c, k) in keys.iter().enumerate() {
            let on = r == a.osk.row && c == a.osk.col;
            let label = if a.osk.shift && k.len() == 1 { k.to_uppercase() } else { k.to_string() };
            let key = container(t(label, f(20.0), tn.text)).center_x(Length::Fill).center_y(Length::Fixed(z(56.0))).style(bx(if on { Pal::mix(tn.card, p.accent, 0.12) } else { Pal::mix(tn.card, tn.text, 0.03) }, 8.0, None));
            let portion = if r == 4 { if c == 1 { 8 } else { 2 } } else { 1 };
            rw = rw.push(container(look::ring(&p, on, 8.0, click(key, Some(Msg::OskKey(r, c))))).width(Length::FillPortion(portion)));
        }
        grid = grid.push(rw);
    }
    container(column![head, field, grid].spacing(z(14.0))).padding(pad(z(22.0), z(76.0), z(14.0), z(76.0))).width(Length::Fill).style(bx(Pal::mix(tn.page, Color::BLACK, 0.15), 0.0, None)).into()
}

fn hints(a: &App) -> E<'_, Msg> {
    let p = a.p;
    use look::{hint, hint2};
    if a.osk.open {
        look::hints(&p, vec![hint(&p, "A", "Type"), hint(&p, "B", "Delete"), hint(&p, "X", "Space"), hint(&p, "Y", "Shift"), hint(&p, "RT", "Add to log")], vec![hint(&p, "LS", "Move"), hint2(&p, "LB", "RB", "Pages"), hint(&p, "≡", "Close keyboard")])
    } else {
        look::hints(&p, vec![hint(&p, "A", "Open"), hint(&p, "B", "Back"), hint(&p, "X", "Sync now"), hint(&p, "Y", "Search")], vec![hint2(&p, "LB", "RB", "Pages"), hint(&p, "≡", "Keyboard")])
    }
}

/// A key pressed on the on-screen keyboard.
pub fn osk_key(a: &mut App, r: usize, c: usize) -> Task<Msg> {
    a.osk.row = r;
    a.osk.col = c;
    let Some(k) = KEYS.get(r).and_then(|row| row.get(c)) else { return Task::none() };
    match *k {
        "⌫" => {
            a.osk.text.pop();
        }
        "⏎" => return add_quick(a),
        "⇧" => a.osk.shift = !a.osk.shift,
        "space" => a.osk.text.push(' '),
        "[[ link" => a.osk.text.push_str("[["),
        "#tag" => a.osk.text.push('#'),
        s => {
            let s = if a.osk.shift { s.to_uppercase() } else { s.to_string() };
            a.osk.text.push_str(&s);
            a.osk.shift = false;
        }
    }
    Task::none()
}

fn add_quick(a: &mut App) -> Task<Msg> {
    if a.osk.text.trim().is_empty() {
        return Task::none();
    }
    a.day = a.today;
    a.quick.kind = match a.osk.kind {
        1 => 2,
        2 => 4,
        _ => 0,
    };
    a.quick.what = std::mem::take(&mut a.osk.text);
    a.quick.at.clear();
    a.quick.waiting.clear();
    let rel = a.today_rel();
    if a.doc.as_ref().map(|d| d.rel != rel).unwrap_or(true) {
        let text = a.vault.find(&rel).map(|i| a.vault.notes[i].text.clone()).unwrap_or_default();
        a.doc = Some(crate::Doc::from_text(&rel, &text));
    }
    let t_ = crate::today::add(a);
    a.say("Added to today's log", false);
    t_
}

fn move_key(a: &mut App, dr: i32, dc: i32) {
    let rows = KEYS.len() as i32;
    let mut r = (a.osk.row as i32 + dr).clamp(0, rows - 1) as usize;
    if r != a.osk.row {
        // Keep roughly the same place across rows of different lengths.
        let from = KEYS[a.osk.row].len() as f32;
        let to = KEYS[r].len() as f32;
        let mut c = ((a.osk.col as f32 + 0.5) / from * to) as usize;
        if r == 4 {
            c = if a.osk.col < 2 { 0 } else if a.osk.col < 10 { 1 } else { 2 };
        } else if a.osk.row == 4 {
            c = [0usize, 5, 10][a.osk.col.min(2)];
        }
        a.osk.col = c.min(KEYS[r].len() - 1);
    } else {
        let n = KEYS[r].len() as i32;
        a.osk.col = ((a.osk.col as i32 + dc).rem_euclid(n)) as usize;
    }
    r = r.min(KEYS.len() - 1);
    a.osk.row = r;
}

/// A controller button: in the controller layout, or the mouse layout's own buttons.
pub fn press(a: &mut App, b: pad::Button) -> Task<Msg> {
    use pad::Button as P;
    pad::ctrl_used();
    if !a.pad && !a.pad_fixed {
        a.pad = true;
        pad::PAD_LOOK.store(true, std::sync::atomic::Ordering::Relaxed);
        return Task::none();
    }
    if !a.pad {
        return mouse_layout(a, b);
    }
    let pages = [PadPage::Today, PadPage::Notes, PadPage::Graph, PadPage::Sync];
    let at = pages.iter().position(|x| *x == a.pad_page).unwrap_or(1);
    match b {
        P::Lb => a.pad_page = pages[(at + 3) % 4],
        P::Rb => a.pad_page = pages[(at + 1) % 4],
        P::Menu => a.osk.open = !a.osk.open,
        P::Lt => a.osk.kind = (a.osk.kind + 1) % PAD_KINDS.len(),
        P::Rt => return add_quick(a),
        _ if a.osk.open => match b {
            P::Up => move_key(a, -1, 0),
            P::Down => move_key(a, 1, 0),
            P::Left => move_key(a, 0, -1),
            P::Right => move_key(a, 0, 1),
            P::A => {
                let (r, c) = (a.osk.row, a.osk.col);
                return osk_key(a, r, c);
            }
            P::B => {
                a.osk.text.pop();
            }
            P::X => a.osk.text.push(' '),
            P::Y => a.osk.shift = !a.osk.shift,
            _ => {}
        },
        P::Up => a.pad_note = a.pad_note.saturating_sub(1),
        P::Down => a.pad_note = (a.pad_note + 1).min(recent(a).len().saturating_sub(1)),
        P::A => {
            let rec = recent(a);
            if let Some(i) = rec.get(a.pad_note) {
                a.pad = false;
                a.pad_fixed = true;
                return crate::update(a, Msg::OpenNote(*i));
            }
        }
        P::X => return crate::update(a, Msg::SyncNow),
        P::Y => {
            a.pad = false;
            return a.rail(Page::Search);
        }
        P::RUp => return crate::update(a, Msg::GZoom(1.15)),
        P::RDown => return crate::update(a, Msg::GZoom(1.0 / 1.15)),
        _ => {}
    }
    Task::none()
}

/// The mouse layout with a controller (its hint strip): the page's A / B / X / Y and LB / RB.
fn mouse_layout(a: &mut App, b: pad::Button) -> Task<Msg> {
    use pad::Button as P;
    match (a.page, b) {
        (_, P::B) => {
            if a.new_item.is_some() {
                a.new_item = None;
            } else if a.g.customise {
                a.g.customise = false;
            } else {
                a.page = Page::Files;
            }
            Task::none()
        }
        (Page::Graph | Page::Modes, P::Y) => {
            let i = crate::gview::MODES.iter().position(|m| m.0 == a.g.mode).unwrap_or(0);
            let next = crate::gview::MODES[(i + 1) % crate::gview::MODES.len()].0.to_string();
            crate::update(a, Msg::GMode(next))
        }
        (Page::Graph, P::A) => match a.g.sel.or(a.g.hover) {
            Some(i) => crate::update(a, Msg::GOpen(i)),
            None => Task::none(),
        },
        (Page::Graph, P::Left) => crate::update(a, Msg::GTurn(-0.15, 0.0)),
        (Page::Graph, P::Right) => crate::update(a, Msg::GTurn(0.15, 0.0)),
        (Page::Graph, P::Up) => crate::update(a, Msg::GPan(0.0, 0.05)),
        (Page::Graph, P::Down) => crate::update(a, Msg::GPan(0.0, -0.05)),
        (Page::Graph, P::RUp) => crate::update(a, Msg::GZoom(1.15)),
        (Page::Graph, P::RDown) => crate::update(a, Msg::GZoom(1.0 / 1.15)),
        (Page::Graph, P::X) => iced::widget::operation::focus(crate::FLY_ID),
        (Page::Graph, P::Lb | P::Rb) => {
            let n = a.graph.nodes.len();
            if n == 0 {
                return Task::none();
            }
            let cur = a.g.sel.unwrap_or(0);
            let next = if b == P::Rb { (cur + 1) % n } else { (cur + n - 1) % n };
            crate::gview::fly_to(a, next);
            Task::none()
        }
        (Page::Modes, P::Left) => {
            a.g.picker_focus = (a.g.picker_focus + 9) % 10;
            Task::none()
        }
        (Page::Modes, P::Right) => {
            a.g.picker_focus = (a.g.picker_focus + 1) % 10;
            Task::none()
        }
        (Page::Modes, P::A) => {
            let m = crate::gview::MODES[a.g.picker_focus.min(9)].0.to_string();
            crate::update(a, Msg::GMode(m))
        }
        (Page::Today, P::X) => crate::update(a, Msg::QuickAdd),
        (Page::Today, P::Lb) => crate::update(a, Msg::DayStep(-1)),
        (Page::Today, P::Rb) => crate::update(a, Msg::DayStep(1)),
        (Page::Board, P::Lb | P::Rb) => {
            let n = a.cards().len().max(1);
            a.card_focus = if b == P::Rb { (a.card_focus + 1) % n } else { (a.card_focus + n - 1) % n };
            Task::none()
        }
        (Page::Board, P::A) => {
            let i = a.card_focus;
            crate::update(a, Msg::CardOpen(i))
        }
        (Page::Board, P::X) => {
            let cards = a.cards();
            match cards.get(a.card_focus) {
                Some(c) => {
                    let next = match c.status {
                        hearth_notes::board::Status::Waiting => hearth_notes::board::Status::Todo,
                        hearth_notes::board::Status::Todo => hearth_notes::board::Status::Doing,
                        hearth_notes::board::Status::Doing => hearth_notes::board::Status::Done,
                        hearth_notes::board::Status::Done => hearth_notes::board::Status::Waiting,
                    };
                    let i = a.card_focus;
                    crate::update(a, Msg::CardMove(i, next))
                }
                None => Task::none(),
            }
        }
        (Page::Sync, P::X) => crate::update(a, Msg::SyncNow),
        (Page::Sync, P::Y) => crate::update(a, Msg::CheckAgain),
        (Page::Sync, P::A) => crate::update(a, Msg::Fix(0)),
        (Page::Conflict, P::X) => crate::update(a, Msg::ConflictPick(hearth_notes::conflict::Pick::Both)),
        (Page::Search, P::A) => {
            let i = a.hit_focus;
            crate::update(a, Msg::SearchOpen(i))
        }
        (Page::Files, P::X) => a.rail(Page::Today),
        (_, P::Y) => a.rail(Page::Search),
        (Page::Settings, P::Lb) => {
            let i = a.settings_sec.saturating_sub(1);
            crate::update(a, Msg::SetSec(i))
        }
        (Page::Settings, P::Rb) => {
            let i = (a.settings_sec + 1).min(6);
            crate::update(a, Msg::SetSec(i))
        }
        (_, P::Lb | P::Rb) if !a.tabs.is_empty() => {
            let n = a.tabs.len();
            let i = if b == P::Rb { (a.tab + 1) % n } else { (a.tab + n - 1) % n };
            crate::update(a, Msg::TabPick(i))
        }
        _ => Task::none(),
    }
}
