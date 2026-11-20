//! Today (page 2): the day's log (log/YYYY-MM-DD.md, US Central, the day starting at 6:00 AM by default) as a
//! dashboard: its sections, the quick add that writes one line in the log's own format, the optional productivity
//! and habit trackers (kept apart from the log, on this PC), and on the right what's still waiting from the last logs.

use crate::look::{self, accent2, bx, click, f, pad, tone, z, E, MONO};
use crate::{App, Msg, Page, QUICK_ID, QUICK_KINDS};
use hearth_notes::{board, day, md};
use hearth_ui::ui::{gap, t, t1, tb, tm, ts};
use hearth_ui::Pal;
use iced::widget::{column, container, row, scrollable, text_input, Column, Row, Space};
use iced::{Color, Length, Task};

#[derive(Debug, Default, Clone)]
pub struct Sections {
    pub made: Vec<String>,
    pub decided: Vec<String>,
    pub stored: Vec<String>,
    pub waiting: Vec<String>,
    pub happened: Vec<String>,
    /// (line, done, text)
    pub tasks: Vec<(usize, bool, String)>,
}

fn strip_time(s: &str) -> &str {
    let b = s.as_bytes();
    if b.len() > 6 && b[2] == b':' && b[..2].iter().all(u8::is_ascii_digit) && b[3..5].iter().all(u8::is_ascii_digit) && b[5] == b' ' {
        &s[6..]
    } else {
        s
    }
}

fn short(s: &str, n: usize) -> String {
    let s = md::plain(s.trim());
    let s = s.trim().trim_end_matches('.').to_string();
    if s.chars().count() > n {
        let mut o: String = s.chars().take(n - 1).collect();
        o = o.trim_end().to_string();
        o.push('…');
        o
    } else {
        s
    }
}

/// The log's lines sorted into Today's sections (the log's own format: "- 03:04 Made: …", "Waiting: …", tasks).
pub fn sections(text: &str) -> Sections {
    let mut s = Sections::default();
    let mut in_code = false;
    for (li, line) in text.lines().enumerate() {
        if line.trim_start().starts_with("```") {
            in_code = !in_code;
            continue;
        }
        if in_code || line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((_, st, txt)) = md::task(line) {
            s.tasks.push((li, st == 'x', txt));
            continue;
        }
        let Some((indent, _, body)) = md::bullet(line) else {
            s.happened.push(short(line, 150));
            continue;
        };
        if indent > 0 {
            continue;
        }
        let body = strip_time(body.trim());
        // Parts that name a place or something waiting, wherever they are in the line.
        for part in body.split(". ") {
            let part = part.trim();
            for k in ["Stored: ", "Stored at: ", "Pictures: "] {
                if let Some(r) = part.strip_prefix(k).filter(|r| !r.contains("[[")) {
                    s.stored.push(short(r, 70));
                }
            }
        }
        for sp in md::spans(body) {
            if let md::Span::Wiki { target, .. } = sp {
                s.stored.push(format!("[[{target}]]"));
            }
        }
        if let Some(at) = body.find("Waiting: ") {
            s.waiting.push(short(&body[at + 9..], 90));
        }
        let lead = body.split(". Waiting: ").next().unwrap_or(body);
        let lead = lead.split(". Stored: ").next().unwrap_or(lead);
        let low = lead.to_lowercase();
        if let Some(r) = lead.strip_prefix("Decided: ") {
            s.decided.push(short(r, 110));
        } else if let Some(r) = lead.strip_prefix("Made: ").or_else(|| lead.strip_prefix("Changed: ")) {
            s.made.push(short(r, 130));
        } else if ["Waiting: ", "Stored: ", "Stored at: ", "Pictures: "].iter().any(|k| lead.starts_with(k)) {
            // Only a place or what's waiting: in those sections already.
        } else if let Some(r) = lead.strip_prefix("Note: ") {
            s.happened.push(short(r, 150));
        } else if low.starts_with("owner") || low.contains("(owner") || low.contains("owner approved") || low.contains("owner:") {
            s.decided.push(short(lead, 110));
        } else {
            s.made.push(short(lead, 130));
        }
    }
    s
}

/// The line the quick add writes, in the log's own format.
pub fn quick_line(kind: usize, hhmm: &str, what: &str, at: &str, waiting: &str) -> String {
    let k = match kind {
        3 => "Stored",
        i => QUICK_KINDS.get(i).copied().unwrap_or("Made"),
    };
    let mut s = format!("- {hhmm} {k}: {}", what.trim().trim_end_matches('.'));
    s.push('.');
    if !at.trim().is_empty() && kind != 3 {
        s.push_str(&format!(" Stored: {}.", at.trim().trim_end_matches('.')));
    }
    if !waiting.trim().is_empty() && kind != 4 {
        s.push_str(&format!(" Waiting: {}.", waiting.trim().trim_end_matches('.')));
    }
    s
}

pub fn add(a: &mut App) -> Task<Msg> {
    if a.quick.what.trim().is_empty() {
        return Task::none();
    }
    let hhmm = day::hhmm(a.central_now());
    let line = quick_line(a.quick.kind, &hhmm, &a.quick.what, &a.quick.at, &a.quick.waiting);
    let rel = day::log_path(a.day);
    let flush = if a.doc.as_ref().map(|d| d.rel != rel).unwrap_or(true) { a.flush_doc() } else { Task::none() };
    let text = match a.doc.as_ref().filter(|d| d.rel == rel) {
        Some(d) => d.text(),
        None => a.vault.find(&rel).map(|i| a.vault.notes[i].text.clone()).unwrap_or_default(),
    };
    let new = hearth_notes::newitem::log_with(if text.trim().is_empty() { None } else { Some(&text) }, &day::ymd(a.day), &line);
    a.doc = Some(crate::Doc::from_text(&rel, &new));
    a.quick = crate::Quick { kind: a.quick.kind, ..Default::default() };
    let t = a.edited();
    Task::batch([flush, t, iced::widget::operation::focus(QUICK_ID)])
}

pub fn action(a: &mut App, w: &str) -> Task<Msg> {
    match w {
        "home" => a.open_rel("INDEX.md"),
        "new" => crate::update(a, Msg::QsOpen),
        "week" => {
            a.group = crate::Group::Date;
            a.rail(Page::Board)
        }
        "atlas" => a.open_atlas("logs"),
        "close" => {
            a.page = Page::Files;
            Task::none()
        }
        _ => a.rail(Page::Today),
    }
}

pub fn view(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let c = a.central_now();
    let chip: E<'static, Msg> = container(t(format!("US Central · {}", day::clock(c.hour, c.min)), f(14.0), Pal::mix(tn.text, tn.muted, 0.2))).padding(pad(z(4.0), z(14.0), z(4.0), z(14.0))).style(bx(Color::TRANSPARENT, z(14.0), Some((tn.line, 1.0)))).into();
    let left = column![crate::editor::tabs(a), crate::editor::crumb(a, Some(chip)), scrollable(container(dashboard(a)).padding(pad(z(22.0), z(30.0), z(30.0), z(30.0))).width(Length::Fill)).height(Length::Fill)];
    row![container(left).width(Length::Fill).height(Length::Fill), hearth_ui::ui::vline(tn.line), crate::editor::right_column(a, true)].height(Length::Fill).into()
}

fn action_btn<'a>(a: &'a App, label: &str, on: bool, w: &'static str) -> E<'a, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let c = container(t1(label.to_string(), f(16.0), if on { accent2(&p) } else { Pal::mix(tn.text, tn.muted, 0.1) })).padding(pad(z(10.0), z(22.0), z(10.0), z(22.0))).style(bx(if on { Pal::a(p.accent, 0.08) } else { tn.card }, 7.0, Some((if on { p.accent } else { tn.line }, 1.0))));
    click(c, Some(Msg::TodayAction(w)))
}

fn dashboard(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let actions = row![
        action_btn(a, "⌂ Home", false, "home"),
        action_btn(a, "+ New note", false, "new"),
        action_btn(a, "◷ Today's log", a.day == a.today, "today"),
        action_btn(a, "▦ This week", false, "week"),
        action_btn(a, "◈ Atlas", false, "atlas"),
        action_btn(a, "✕ Close", false, "close"),
    ]
    .spacing(z(14.0));
    let title = column![
        tb(day::ymd(a.day), f(38.0), tn.text),
        tm(day::long(a.day), f(26.0), accent2(&p)),
        row![click(t("« Yesterday", f(15.0), accent2(&p)), Some(Msg::DayStep(-1))), t(" | ", f(15.0), tn.faint), click(t("Tomorrow »", f(15.0), if a.day < a.today { accent2(&p) } else { tn.faint }), if a.day < a.today { Some(Msg::DayStep(1)) } else { None })],
    ]
    .spacing(z(4.0));
    let mut top = Row::new().spacing(z(18.0)).push(container(title).width(Length::Shrink));
    if a.prefs.trackers {
        top = top.push(productivity(a)).push(habits(a));
    }
    let mut c = column![container(actions).center_x(Length::Fill), top, quick(a)].spacing(z(20.0));
    let rel = day::log_path(a.day);
    let text = a.doc.as_ref().filter(|d| d.rel == rel).map(|d| d.text()).unwrap_or_else(|| a.vault.find(&rel).map(|i| a.vault.notes[i].text.clone()).unwrap_or_default());
    let s = sections(&text);
    let sec = |name: &str, sub: &str, items: &[String], mono: bool| -> E<'static, Msg> {
        let mut col = Column::new().spacing(z(6.0)).push(row![tb(name.to_string(), f(24.0), accent2(&p)), gap(z(10.0), 1.0), t(sub.to_string(), f(13.0), tn.muted)].align_y(iced::Center));
        for it in items.iter().take(4) {
            let tx: E<Msg> = if mono { iced::widget::text(it.clone()).size(f(14.5)).font(MONO).color(Pal::mix(tn.text, tn.muted, 0.1)).into() } else { t(it.clone(), f(16.0), Pal::mix(tn.text, tn.muted, 0.08)).into() };
            col = col.push(row![container(look::dot(p.accent, z(7.0))).padding(pad(z(9.0), 0.0, 0.0, 0.0)), gap(z(12.0), 1.0), container(tx).width(Length::Fill)].align_y(iced::Top));
        }
        if items.is_empty() {
            col = col.push(t("Nothing yet", f(14.0), tn.faint));
        }
        col.width(Length::Fill).into()
    };
    let mut tasks = Column::new().spacing(z(8.0)).push(row![tb("New tasks", f(24.0), accent2(&p)), gap(z(10.0), 1.0), t("show up on the Board", f(13.0), tn.muted)].align_y(iced::Center));
    for (li, done, txt) in s.tasks.iter().filter(|x| !x.2.starts_with("Waiting:")).take(6) {
        let b = container(if *done { hearth_ui::ui::glyph(hearth_ui::ui::G::Check, p.ink, z(14.0), 3.0) } else { gap(1.0, 1.0) }).center_x(Length::Fixed(z(22.0))).center_y(Length::Fixed(z(22.0))).style(bx(if *done { p.accent } else { Color::TRANSPARENT }, 4.0, if *done { None } else { Some((tn.muted, 1.2)) }));
        tasks = tasks.push(row![click(b, Some(Msg::TaskToggle(*li))), gap(z(14.0), 1.0), t(md::plain(txt), f(16.0), Pal::mix(tn.text, tn.muted, 0.08))].align_y(iced::Center));
    }
    if s.tasks.is_empty() {
        tasks = tasks.push(t("Nothing yet", f(14.0), tn.faint));
    }
    let grid = column![
        row![sec("Made", "what was made or changed", &s.made, false), sec("Decided", "what the owner decided", &s.decided, false)].spacing(z(40.0)),
        row![sec("Stored at", "where each thing is", &s.stored, true), sec("Waiting", "what still needs someone", &s.waiting, false)].spacing(z(40.0)),
        row![sec("What happened today", "free notes", &s.happened, false), container(tasks).width(Length::Fill)].spacing(z(40.0)),
    ]
    .spacing(z(26.0));
    c = c.push(grid);
    c.into()
}

fn productivity(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let v = a.track.productivity.unwrap_or(0);
    let head = container(row![ts("Productivity", f(15.0), tn.text), Space::new().width(Length::Fill), t(format!("{v} / 10"), f(15.0), tn.text)]).padding(pad(z(8.0), z(14.0), z(8.0), z(14.0))).style(bx(Color::from_rgb8(0x1d, 0x3b, 0x2a), 0.0, None));
    let mut steps = Row::new();
    for i in 1..=10u8 {
        steps = steps.push(click(container(Space::new()).width(Length::Fill).height(Length::Fixed(z(16.0))), Some(Msg::Productivity(i))));
    }
    let bar = iced::widget::stack![look::progress(&p, v as f32 / 10.0, z(16.0), Length::Fill), steps];
    let scale = row![t("0", f(11.0), tn.faint), Space::new().width(Length::Fill), t("5", f(11.0), tn.faint), Space::new().width(Length::Fill), t("10", f(11.0), tn.faint)];
    container(column![head, container(column![bar, scale].spacing(z(4.0))).padding(pad(z(10.0), z(14.0), z(8.0), z(14.0)))]).width(Length::Fixed(z(260.0))).style(bx(tn.card, 8.0, Some((tn.line, 1.0)))).clip(true).into()
}

fn habits(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let head = container(row![ts("⚡ Habits", f(15.0), tn.text), Space::new().width(Length::Fill), t("optional", f(12.0), tn.muted)].align_y(iced::Center)).padding(pad(z(8.0), z(14.0), z(8.0), z(14.0))).style(bx(Color::from_rgb8(0x4a, 0x1f, 0x22), 0.0, None));
    let mut r = Row::new().spacing(z(6.0)).align_y(iced::Center);
    for h in hearth_notes::trackers::HABITS {
        let on = a.track.habits.get(h).copied().unwrap_or(false);
        r = r.push(t1(h, f(13.5), tn.text)).push(look::switch(&p, on, false, Some(Msg::Habit(h.to_string()))));
    }
    container(column![head, container(r).padding(pad(z(12.0), z(10.0), z(12.0), z(10.0)))]).width(Length::Fill).style(bx(tn.card, 8.0, Some((tn.line, 1.0)))).clip(true).into()
}

fn quick(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let kinds: Vec<(String, bool, Msg)> = QUICK_KINDS.iter().enumerate().map(|(i, k)| (k.to_string(), a.quick.kind == i, Msg::QuickKind(i))).collect();
    let head = row![ts("Add to today's log", f(17.0), tn.text), gap(z(14.0), 1.0), look::seg(&p, kinds, false), Space::new().width(Length::Fill), t("Ctrl+Shift+L from anywhere", f(13.5), tn.muted)].align_y(iced::Center);
    let what = text_input("What was made or changed", &a.quick.what).id(QUICK_ID).on_input(Msg::QuickWhat).on_submit(Msg::QuickAdd).size(f(15.5)).padding(pad(z(10.0), z(13.0), z(10.0), z(13.0))).style(look::field_style(&p, true));
    let at_ok = !a.quick.at.trim().is_empty() && a.root.join(a.quick.at.trim()).exists();
    let at_field = text_input("Where it's stored", &a.quick.at).on_input(Msg::QuickAt).on_submit(Msg::QuickAdd).size(f(14.0)).font(MONO).padding(pad(z(10.0), z(13.0), z(10.0), z(13.0))).style(look::field_style(&p, false));
    let at: E<Msg> = if at_ok { iced::widget::stack![at_field, container(t("✓", f(15.0), look::GOOD)).align_right(Length::Fill).center_y(Length::Fill).padding(pad(0.0, z(12.0), 0.0, 0.0))].into() } else { at_field.into() };
    let wait = text_input("What's waiting", &a.quick.waiting).on_input(Msg::QuickWait).on_submit(Msg::QuickAdd).size(f(15.5)).padding(pad(z(10.0), z(13.0), z(10.0), z(13.0))).style(look::field_style(&p, false));
    let fields = row![container(what).width(Length::FillPortion(16)), container(at).width(Length::FillPortion(10)), container(wait).width(Length::FillPortion(10)), look::btn(&p, "Add ⏎", look::B::Primary, false, false, Some(Msg::QuickAdd))].spacing(z(10.0)).align_y(iced::Center);
    let preview = if a.quick.what.trim().is_empty() { String::new() } else { format!("Adds: {}", quick_line(a.quick.kind, &day::hhmm(a.central_now()), &a.quick.what, &a.quick.at, &a.quick.waiting)) };
    let mut c = column![head, fields].spacing(z(10.0));
    if !preview.is_empty() {
        c = c.push(iced::widget::text(preview).size(f(13.0)).font(MONO).color(tn.muted));
    }
    container(c).padding(pad(z(14.0), z(18.0), z(14.0), z(18.0))).width(Length::Fill).style(bx(tn.card, 10.0, Some((p.accent, 2.0)))).into()
}

/// Today's right column below the calendar: the page's sections, what's still waiting, modified today.
pub fn right(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let mut c = Column::new().spacing(z(8.0)).push(crate::editor::rh(&p, "On this page", None));
    c = c.push(t(day::ymd(a.day), f(15.5), Pal::mix(tn.text, tn.muted, 0.15)));
    for (i, s) in ["Made", "Decided", "Stored at", "Waiting", "What happened today", "New tasks"].iter().enumerate() {
        let on = i == 3;
        let mut r = row![gap(z(18.0), 1.0)].align_y(iced::Center);
        if on {
            r = r.push(look::dot(p.accent, z(10.0))).push(gap(z(8.0), 1.0));
        }
        r = r.push(t1(s.to_string(), f(15.5), if on { accent2(&p) } else { Pal::mix(tn.text, tn.muted, 0.15) }));
        c = c.push(r);
    }
    // Still waiting, from the last five logs.
    let mut logs: Vec<usize> = a.vault.notes.iter().enumerate().filter(|(_, n)| n.log.map(|d| d <= a.today).unwrap_or(false)).map(|(i, _)| i).collect();
    logs.sort_by(|x, y| a.vault.notes[*y].rel.cmp(&a.vault.notes[*x].rel));
    logs.truncate(5);
    let mut waiting: Vec<(String, String, String)> = Vec::new();
    let cards = board::cards(&a.vault, a.today);
    for c_ in cards.iter().filter(|c| c.status == board::Status::Waiting) {
        let n = &a.vault.notes[c_.note];
        if n.log.is_some() && !logs.contains(&c_.note) {
            continue;
        }
        let what = if n.rel.ends_with("/README.md") { n.rel.rsplit('/').nth(1).unwrap_or("").to_string() } else { n.title.clone() };
        if !waiting.iter().any(|w| w.0 == c_.text) {
            waiting.push((c_.text.clone(), what, day::short(c_.day)));
        }
    }
    let total = cards.len();
    c = c.push(crate::editor::rh(&p, "Still waiting", Some("last 5 logs".into())));
    for (txt, what, d) in waiting.iter().take(3) {
        let card = container(column![t1(hearth_ui::ui::fit(txt, 34), f(15.0), tn.text), row![t1(hearth_ui::ui::fit(what, 26), f(13.0), tn.muted), t1(format!(" · {d}"), f(13.0), accent2(&p))]].spacing(z(3.0))).padding(pad(z(9.0), z(12.0), z(9.0), z(12.0))).width(Length::Fill).style(bx(tn.card, 8.0, Some((tn.line, 1.0))));
        c = c.push(card);
    }
    if waiting.is_empty() {
        c = c.push(t("Nothing waiting", f(14.0), tn.faint));
    }
    c = c.push(look::link(&p, format!("All {total} on the Board ›"), f(14.0), Msg::Rail(Page::Board)));
    c = c.push(crate::editor::modified_today(a));
    c.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_log_sorted_into_its_sections() {
        let log = "# 2026-11-20\n\n- KDE settings pages, the open parts: built.\n- 01:36 Decided: Sharper graphics (vkBasalt) approved\n- Owner approved quick ACKs on the loopback for 1.16\n- 03:04 Made: Hearth Notes mockups. Stored: hearth/mockups/hearth-notes. Waiting: the owner's look.\n- Pictures: [[hearth/assets/1.16-kde-settings-shots]]\n- Note: long night of 1.16 work\n- [ ] Look at labwc's busy minute\n- [x] Push the Hearth Notes mockups\n";
        let s = sections(log);
        assert_eq!(s.made, vec!["KDE settings pages, the open parts: built", "Hearth Notes mockups"]);
        assert_eq!(s.decided, vec!["Sharper graphics (vkBasalt) approved", "Owner approved quick ACKs on the loopback for 1.16"]);
        assert!(s.stored.contains(&"hearth/mockups/hearth-notes".to_string()));
        assert!(s.stored.contains(&"[[hearth/assets/1.16-kde-settings-shots]]".to_string()));
        assert_eq!(s.waiting, vec!["the owner's look"]);
        assert_eq!(s.happened, vec!["long night of 1.16 work"]);
        assert_eq!(s.tasks.len(), 2);
        assert_eq!(quick_line(0, "03:04", "Hearth Notes mockups", "hearth/mockups/hearth-notes", "the owner's look"), "- 03:04 Made: Hearth Notes mockups. Stored: hearth/mockups/hearth-notes. Waiting: the owner's look.");
    }
}
