//! Search (page 4): the sidebar's search across the vault (Aa match case, "" whole phrase), its filters by folder, tag
//! and date, the matches counted and shown in their lines; the open match in the note; and the quick switcher
//! (Ctrl+O): fuzzy names, notes that mention it, open, a new tab, or make it a new note.

use crate::look::{self, accent2, bx, click, f, pad, tone, z, E, MONO};
use crate::{App, Msg, QS_ID, SEARCH_ID};
use hearth_notes::search;
use hearth_ui::ui::{gap, t, t1, tm};
use hearth_ui::Pal;
use iced::widget::{column, container, mouse_area, row, scrollable, stack, text, text_input, Column, Row, Space};
use iced::{Color, Length};

/// A line with its matches boxed in the accent.
fn marked<'a>(p: &Pal, line: &str, at: &[(usize, usize)], size: f32, c: Color) -> E<'a, Msg> {
    let mut items: Vec<E<'a, Msg>> = Vec::new();
    let mut pos = 0;
    let space = size * 0.27;
    let words = |s: &str, hit: bool, items: &mut Vec<E<'a, Msg>>| {
        for (i, w) in s.split(' ').enumerate() {
            if i > 0 {
                items.push(gap(space, 1.0));
            }
            if w.is_empty() {
                continue;
            }
            if hit {
                items.push(container(t(w.to_string(), size, Pal::mix(c, Color::WHITE, 0.2))).padding(pad(0.0, 2.0, 0.0, 2.0)).style(bx(Pal::a(p.accent, 0.35), 3.0, Some((Pal::a(p.accent, 0.85), 1.0)))).into());
            } else {
                items.push(t(w.to_string(), size, c).into());
            }
        }
    };
    for &(s, e) in at {
        if s < pos || e > line.len() || !line.is_char_boundary(s) || !line.is_char_boundary(e) {
            continue;
        }
        words(&line[pos..s], false, &mut items);
        words(&line[s..e], true, &mut items);
        pos = e;
    }
    words(&line[pos..], false, &mut items);
    Row::with_children(items).align_y(iced::Center).wrap().into()
}

pub fn side(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let q = &a.query;
    let input = text_input("Search all notes", &q.text).id(SEARCH_ID).on_input(Msg::SearchText).size(f(17.0)).padding(pad(z(9.0), z(70.0), z(9.0), z(36.0))).style(look::field_style(&p, true));
    let toggles = row![click(t("Aa", f(13.0), if q.case { accent2(&p) } else { tn.muted }), Some(Msg::SearchCase)), gap(z(8.0), 1.0), click(t("\"\"", f(13.0), if q.exact { accent2(&p) } else { tn.muted }), Some(Msg::SearchExact))].align_y(iced::Center);
    let field = stack![
        input,
        container(t("⌕", f(15.0), tn.muted)).center_y(Length::Fill).padding(pad(0.0, 0.0, 0.0, z(12.0))),
        container(toggles).align_right(Length::Fill).center_y(Length::Fill).padding(pad(0.0, z(14.0), 0.0, 0.0)),
    ];
    let chip_on = |label: String, clear: Msg| -> E<'static, Msg> {
        container(row![t1(label, f(14.0), accent2(&p)), gap(z(8.0), 1.0), click(t("✕", f(12.0), accent2(&p)), Some(clear))].align_y(iced::Center)).padding(pad(z(4.0), z(12.0), z(4.0), z(12.0))).style(bx(Color::TRANSPARENT, z(14.0), Some((p.accent, 1.0)))).into()
    };
    let chip_off = |label: String, m: Msg| -> E<'static, Msg> { click(container(t1(label, f(14.0), Pal::mix(tn.text, tn.muted, 0.15))).padding(pad(z(4.0), z(12.0), z(4.0), z(12.0))).style(bx(Color::TRANSPARENT, z(14.0), Some((tn.line, 1.0)))), Some(m)) };
    let folder: E<Msg> = if q.folders.is_empty() { chip_off("Folder: any ▾".into(), Msg::SearchFilterMenu(1)) } else { chip_on(format!("Folder: {}", q.folders.join(", ")), Msg::SearchFolder(None)) };
    let tag: E<Msg> = match &q.tag {
        Some(tg) => chip_on(format!("Tag: #{tg}"), Msg::SearchTag(None)),
        None => chip_off("Tag: any ▾".into(), Msg::SearchFilterMenu(2)),
    };
    let date: E<Msg> = match q.days {
        Some(d) => chip_on(format!("Date: last {d} days"), Msg::SearchDays(None)),
        None => chip_off("Date: any ▾".into(), Msg::SearchFilterMenu(3)),
    };
    let notes = a.hits.len();
    let total: usize = a.hits.iter().map(|h| h.count).sum();
    let mut list = Column::new().spacing(z(10.0));
    for (i, h) in a.hits.iter().enumerate().take(60) {
        let n = &a.vault.notes[h.note];
        let name = if n.rel.ends_with("/README.md") { n.rel.trim_end_matches("/README.md").to_string() } else { n.rel.clone() };
        let on = i == a.hit_focus;
        let mut c = Column::new().spacing(z(6.0)).push(row![container(t(name, f(15.0), tn.text)).width(Length::Fill), t(h.count.to_string(), f(14.0), tn.muted)].align_y(iced::Top));
        for (line, at) in h.lines.iter().take(2) {
            let (l2, at2) = trim_line(line, at, 120);
            c = c.push(marked(&p, &l2, &at2, f(14.0), Pal::mix(tn.text, tn.muted, 0.2)));
        }
        let card = container(c).padding(pad(z(12.0), z(14.0), z(12.0), z(14.0))).width(Length::Fill).style(bx(tn.card, 8.0, Some((if on { p.accent } else { tn.line }, if on { 1.5 } else { 1.0 }))));
        list = list.push(click(card, Some(Msg::SearchOpen(i))));
    }
    let count = if q.text.trim().is_empty() { "Type to search every note".to_string() } else { format!("{total} match{} in {notes} note{}", if total == 1 { "" } else { "es" }, crate::plural(notes)) };
    let mut col = column![
        field,
        row![folder].spacing(z(8.0)),
        row![tag, date].spacing(z(8.0)),
        t(count, f(14.0), tn.muted),
    ]
    .spacing(z(12.0));
    if a.filter_menu > 0 {
        col = col.push(filter_menu(a));
    }
    column![container(col).padding(pad(z(6.0), z(16.0), z(10.0), z(16.0))), scrollable(container(list).padding(pad(0.0, z(16.0), z(16.0), z(16.0)))).height(Length::Fill)].into()
}

fn trim_line(line: &str, at: &[(usize, usize)], width: usize) -> (String, Vec<(usize, usize)>) {
    let l = line.trim_start_matches(['-', ' ', '*']);
    let off = line.len() - l.len();
    let at: Vec<(usize, usize)> = at.iter().filter(|x| x.0 >= off).map(|x| (x.0 - off, x.1 - off)).collect();
    if l.chars().count() <= width {
        return (l.to_string(), at);
    }
    let first = at.first().map(|x| x.0).unwrap_or(0);
    let mut start = first.saturating_sub(width / 3);
    while start > 0 && !l.is_char_boundary(start) {
        start -= 1;
    }
    let mut end = (start + width).min(l.len());
    while end < l.len() && !l.is_char_boundary(end) {
        end += 1;
    }
    let mut s = l[start..end].to_string();
    let lead = if start > 0 { "…" } else { "" };
    let at: Vec<(usize, usize)> = at.iter().filter(|x| x.0 >= start && x.1 <= end).map(|x| (x.0 - start + lead.len(), x.1 - start + lead.len())).collect();
    s = format!("{lead}{s}{}", if end < l.len() { "…" } else { "" });
    (s, at)
}

fn filter_menu(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let item = |label: String, m: Msg| -> E<'static, Msg> { click(container(t1(label, f(14.0), tn.text)).padding(pad(z(5.0), z(12.0), z(5.0), z(12.0))).width(Length::Fill), Some(m)) };
    let mut c = Column::new().spacing(1.0);
    match a.filter_menu {
        1 => {
            let mut tops: Vec<String> = a.vault.dirs.iter().filter(|d| d.split('/').count() <= 2).cloned().collect();
            tops.sort();
            for d in tops.into_iter().take(14) {
                c = c.push(item(d.clone(), Msg::SearchFolder(Some(d))));
            }
        }
        2 => {
            for (tg, n) in a.vault.tag_counts().into_iter().take(12) {
                c = c.push(item(format!("#{tg} ({n})"), Msg::SearchTag(Some(tg))));
            }
        }
        _ => {
            for d in [1u32, 7, 30, 365] {
                c = c.push(item(if d == 1 { "Today".into() } else { format!("Last {d} days") }, Msg::SearchDays(Some(d))));
            }
        }
    }
    container(scrollable(c)).padding(z(6.0)).max_height(z(360.0)).width(Length::Fill).style(bx(tn.card, 8.0, Some((tn.line, 1.0)))).into()
}

/// How many matches the open note has (Match 2 of 5).
pub fn matches_in_open(a: &App) -> usize {
    let Some(d) = a.doc.as_ref() else { return 0 };
    let terms: Vec<String> = a.query.text.split_whitespace().map(|s| if a.query.case { s.to_string() } else { s.to_lowercase() }).collect();
    d.lines.iter().map(|l| search::find_all(l, &terms, a.query.case).len()).sum()
}

/// The page behind: the open match's note, its matches highlighted, "Match 2 of 5 in this note" with ↑ ↓.
pub fn content(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let n = matches_in_open(a);
    let right: E<'static, Msg> = if n > 0 {
        let arrow = |s: &'static str, d: i32| -> E<'static, Msg> { click(container(t(s, f(14.0), tn.muted)).center_x(Length::Fixed(z(34.0))).center_y(Length::Fixed(z(30.0))).style(bx(Color::TRANSPARENT, 6.0, Some((tn.line, 1.0)))), Some(Msg::MatchStep(d))) };
        row![t(format!("Match {} of {n} in this note", a.match_at.min(n - 1) + 1), f(14.0), tn.muted), gap(z(14.0), 1.0), arrow("↑", -1), gap(z(8.0), 1.0), arrow("↓", 1)].align_y(iced::Center).into()
    } else {
        gap(1.0, 1.0)
    };
    let body: E<Msg> = match a.doc.as_ref() {
        Some(d) => {
            let terms: Vec<String> = a.query.text.split_whitespace().map(|s| if a.query.case { s.to_string() } else { s.to_lowercase() }).collect();
            let marks: Vec<(usize, Vec<(usize, usize)>)> = d.lines.iter().enumerate().map(|(i, l)| (i, search::find_all(l, &terms, a.query.case))).filter(|x| !x.1.is_empty()).collect();
            scrollable(container(crate::editor::render(a, d, Some(&marks))).padding(pad(z(26.0), z(60.0), z(40.0), z(60.0))).width(Length::Fill)).height(Length::Fill).into()
        }
        None => container(t("Pick a match on the left", f(17.0), tn.muted)).center_x(Length::Fill).center_y(Length::Fill).into(),
    };
    column![crate::editor::tabs(a), crate::editor::crumb(a, Some(right)), body].into()
}

/// The quick switcher (Ctrl+O), over the page.
pub fn switcher(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let Some((q, sel)) = a.qs.as_ref() else { return gap(1.0, 1.0) };
    let list = search::switcher(&a.vault, q, 4);
    let input = text_input("Find or make a note", q).id(QS_ID).on_input(Msg::QsText).on_submit(Msg::QsPick(*sel, false)).size(f(19.0)).padding(pad(z(10.0), z(200.0), z(10.0), z(40.0))).style(look::field_style(&p, true));
    let field = stack![input, container(t("⌕", f(15.0), tn.muted)).center_y(Length::Fill).padding(pad(0.0, 0.0, 0.0, z(14.0))), container(t("Quick switcher · Ctrl+O", f(13.0), tn.muted)).align_right(Length::Fill).center_y(Length::Fill).padding(pad(0.0, z(16.0), 0.0, 0.0))];
    let mut rows = Column::new().spacing(z(4.0));
    for (i, s) in list.iter().enumerate() {
        let on = i == *sel;
        let name: E<Msg> = fuzzy_name(&p, &s.name, &s.at, f(17.0), tn.text);
        let n = &a.vault.notes[s.note];
        let sub: E<Msg> = match &s.line {
            Some(l) => t1(format!("\"{l}\"{}", if n.log.is_some() { " · log" } else { "" }), f(14.0), tn.muted).into(),
            None => {
                let at2: Vec<usize> = {
                    // The path's matched letters too (the mockup's hearth/mockups/g15-app).
                    let ql: Vec<char> = q.to_lowercase().chars().collect();
                    let pl = s.path.to_lowercase();
                    pl.find(&ql.iter().collect::<String>()).map(|st| (pl[..st].chars().count()..pl[..st].chars().count() + ql.len()).collect()).unwrap_or_default()
                };
                fuzzy_mono(&p, &s.path, &at2, f(13.0), tn.muted)
            }
        };
        let mut right = Row::new().spacing(z(14.0)).align_y(iced::Center);
        if on {
            let status = hearth_notes::md::properties(&n.text).into_iter().find(|x| x.0 == "Status").map(|x| x.1.split_whitespace().next().unwrap_or("").trim_matches('*').to_string()).unwrap_or_default();
            if !status.is_empty() {
                right = right.push(look::chip(&p, status, look::Chip::Ok));
            }
            right = right.push(t("↩ open", f(13.0), tn.muted));
        } else if i == 1 {
            right = right.push(t("Ctrl+↩ new tab", f(13.0), tn.muted));
        }
        let r = container(row![column![name, sub].spacing(z(3.0)).width(Length::Fill), right].align_y(iced::Center)).padding(pad(z(10.0), z(16.0), z(10.0), z(16.0))).width(Length::Fill).style(bx(if on { Pal::mix(tn.card, tn.text, 0.04) } else { Color::TRANSPARENT }, 8.0, if on { Some((p.accent, 2.0)) } else { None }));
        rows = rows.push(click(r, Some(Msg::QsPick(i, false))));
    }
    if list.is_empty() && !q.trim().is_empty() {
        rows = rows.push(container(t(format!("No note called \"{}\" yet: Shift+↩ makes it", q.trim()), f(15.0), tn.muted)).padding(z(14.0)));
    }
    let make = if q.trim().is_empty() { "Shift+↩ make a new note".to_string() } else { format!("Shift+↩ make \"{}\" as a new note", q.trim()) };
    let foot = row![t("↑↓ move", f(13.0), tn.muted), gap(z(24.0), 1.0), t("↩ open", f(13.0), tn.muted), gap(z(24.0), 1.0), t("Ctrl+↩ new tab", f(13.0), tn.muted), gap(z(24.0), 1.0), click(t(make, f(13.0), tn.muted), Some(Msg::QsMake)), Space::new().width(Length::Fill), click(t("Esc close", f(13.0), tn.muted), Some(Msg::QsClose))].align_y(iced::Center);
    let panel = container(column![field, rows, foot].spacing(z(12.0))).padding(z(16.0)).width(Length::Fixed(z(900.0))).style(bx(tn.card, 12.0, Some((tn.line, 1.0))));
    let catcher = mouse_area(container(Space::new()).width(Length::Fill).height(Length::Fill).style(bx(Color { a: 0.45, ..Color::BLACK }, 0.0, None))).on_press(Msg::QsClose);
    stack![catcher, container(iced::widget::opaque(panel)).center_x(Length::Fill).padding(pad(z(118.0), 0.0, 0.0, z(320.0)))].into()
}

fn fuzzy_name<'a>(p: &Pal, s: &str, at: &[usize], size: f32, c: Color) -> E<'a, Msg> {
    let mut r = Row::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let hit = at.contains(&i);
        let mut j = i;
        while j < chars.len() && at.contains(&j) == hit {
            j += 1;
        }
        let part: String = chars[i..j].iter().collect();
        if hit {
            r = r.push(container(tm(part, size, Color::WHITE)).style(bx(Pal::a(p.accent, 0.45), 3.0, Some((p.accent, 1.0)))));
        } else {
            r = r.push(tm(part, size, c));
        }
        i = j;
    }
    r.into()
}

fn fuzzy_mono<'a>(p: &Pal, s: &str, at: &[usize], size: f32, c: Color) -> E<'a, Msg> {
    let mut r = Row::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let hit = at.contains(&i);
        let mut j = i;
        while j < chars.len() && at.contains(&j) == hit {
            j += 1;
        }
        let part: String = chars[i..j].iter().collect();
        let tx = text(part).size(size).font(MONO).color(if hit { Color::WHITE } else { c });
        if hit {
            r = r.push(container(tx).style(bx(Pal::a(p.accent, 0.45), 3.0, None)));
        } else {
            r = r.push(tx);
        }
        i = j;
    }
    r.into()
}
