//! The main window (page 1): the note in live preview (the cursor's line shown raw, `[[links]]` with autocomplete,
//! checklists you can tick, embedded pictures, code blocks), Source and Read, the live graph pane following the open
//! note, and the right column (month calendar, outline, backlinks, tags, modified today).

use crate::look::{self, accent2, bx, click, f, pad, tone, z, E, MONO, SERIF};
use crate::{App, Doc, Mode, Msg, Tab, LINE_ID};
use hearth_notes::day;
use hearth_notes::md::{self, Block, Span};
use hearth_ui::ui::{self, gap, glyph, t, t1, tb, tm, ts, G};
use hearth_ui::Pal;
use iced::widget::{column, container, image, row, scrollable, stack, text, text_editor, text_input, Column, Row, Space};
use iced::{font, Color, Font, Length, Theme};

pub fn body_font(a: &App) -> Font {
    match a.prefs.font.as_str() {
        "serif" => SERIF,
        "mono" => MONO,
        _ => hearth_ui::app::INTER,
    }
}

fn bold(fnt: Font) -> Font {
    Font { weight: font::Weight::Bold, ..fnt }
}

/// The tab strip (shared by the note, Today and the Board).
pub fn tabs(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let mut r = Row::new().spacing(z(6.0)).align_y(iced::Bottom);
    for (i, tb_) in a.tabs.iter().enumerate() {
        let on = i == a.tab && tab_on(a, tb_);
        let (lead, name, small): (&str, String, String) = match tb_ {
            Tab::Note(rel) if rel.starts_with("log/") => ("◷ ", rel.trim_start_matches("log/").trim_end_matches(".md").to_string(), "log".into()),
            Tab::Note(rel) if rel.starts_with("atlas:") => ("◈ ", format!("{} · map", &rel[6..]), String::new()),
            Tab::Note(rel) => {
                let file = rel.rsplit('/').next().unwrap_or(rel).to_string();
                let parent = rel.rsplit('/').nth(1).unwrap_or("").to_string();
                ("", file, if parent.is_empty() { String::new() } else { parent })
            }
            Tab::Board => ("▥ ", "Board".into(), String::new()),
            Tab::Graph => ("✳ ", "Graph view".into(), String::new()),
        };
        let mut inner = row![t1(format!("{lead}{name}"), f(15.0), if on { tn.text } else { tn.muted })].align_y(iced::Center).spacing(z(8.0));
        if !small.is_empty() {
            inner = inner.push(t1(small, f(12.0), tn.faint));
        }
        if on {
            inner = inner.push(gap(z(8.0), 1.0)).push(click(t("✕", f(12.0), tn.muted), Some(Msg::TabClose(i))));
        }
        let tab = container(inner).padding(pad(z(9.0), z(18.0), z(9.0), z(18.0))).style(bx(if on { tn.page } else { Color::TRANSPARENT }, 6.0, if on { Some((Pal::a(p.accent, 0.75), 1.0)) } else { None }));
        r = r.push(click(tab, Some(Msg::TabPick(i))));
    }
    r = r.push(click(container(t("+", f(17.0), tn.muted)).padding(pad(z(6.0), z(14.0), z(8.0), z(14.0))), Some(Msg::QsOpen)));
    column![container(r).padding(pad(z(8.0), z(12.0), 0.0, z(12.0))).height(Length::Fixed(z(56.0))).align_bottom(Length::Fixed(z(56.0))), ui::hline(tn.line)].into()
}

fn tab_on(a: &App, t: &Tab) -> bool {
    match (t, a.page) {
        (Tab::Board, crate::Page::Board) => true,
        (Tab::Graph, crate::Page::Graph | crate::Page::Modes) => true,
        (Tab::Note(r), crate::Page::Files | crate::Page::Today | crate::Page::Search) => a.doc.as_ref().map(|d| &d.rel == r).unwrap_or(false),
        _ => false,
    }
}

/// "‹ › hearth › mockups › hearth-store › README.md" and the Live / Source / Read switch.
pub fn crumb<'a>(a: &'a App, right: Option<E<'static, Msg>>) -> E<'a, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let rel = a.doc.as_ref().map(|d| d.rel.clone()).unwrap_or_default();
    let mut r = row![t("‹ ›", f(15.0), tn.faint), gap(z(12.0), 1.0)].align_y(iced::Center);
    let parts: Vec<String> = if let Some(w) = rel.strip_prefix("atlas:") { vec!["Atlas".into(), format!("{w} · map")] } else { rel.split('/').map(str::to_string).collect() };
    let n = parts.len();
    for (i, part) in parts.into_iter().enumerate() {
        if i + 1 == n {
            r = r.push(tb(part, f(15.0), tn.text));
        } else {
            r = r.push(t(format!("{part} › "), f(15.0), tn.muted));
        }
    }
    let mut r = row![container(r).width(Length::Fill).clip(true)].align_y(iced::Center);
    if let Some(x) = right {
        r = r.push(x);
    } else {
        let m = a.mode;
        r = r.push(look::seg(&p, vec![("Live".into(), m == Mode::Live, Msg::SetMode(Mode::Live)), ("Source".into(), m == Mode::Source, Msg::SetMode(Mode::Source)), ("Read".into(), m == Mode::Read, Msg::SetMode(Mode::Read))], false));
    }
    column![container(r).padding(pad(z(6.0), z(22.0), z(6.0), z(26.0))).height(Length::Fixed(z(52.0))).center_y(Length::Fixed(z(52.0))), ui::hline(tn.line)].into()
}

pub fn view(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let items: Vec<(&'static str, E<Msg>)> = vec![
        ("tabs", tabs(a)),
        ("crumb", crumb(a, None)),
        ("note", container(note(a)).height(Length::Fill).into()),
        ("status", status_bar(a)),
        ("graph", container(crate::gview::pane(a)).width(Length::Fill).height(Length::Fill).into()),
        ("right", right_column(a, false)),
    ];
    // its blocks (hearth_ui::blocks "files", side.rs): as it was when nothing is customised
    hearth_ui::arrange::parts(
        &a.describe,
        &p,
        "files",
        items,
        move |ps| {
            let left = column![ps.take("tabs"), ps.take("crumb"), ps.take("note"), ps.take("status")];
            let pane = container(left).width(Length::Fixed(z(640.0))).height(Length::Fill).style(bx(tn.page, 0.0, None));
            row![pane, ui::vline(tn.line), ps.take("graph"), ui::vline(tn.line), ps.take("right")].height(Length::Fill).into()
        },
        Msg::Describe,
    )
}

/// The note: live preview, Source or Read.
pub fn note(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let Some(d) = a.doc.as_ref() else {
        return container(t("Open a note from the folders, or press Ctrl+O.", f(17.0), tn.muted)).center_x(Length::Fill).center_y(Length::Fill).into();
    };
    if a.mode == Mode::Source && !d.generated {
        let size = a.prefs.font_size;
        let ed = text_editor(&d.source)
            .on_action(Msg::Source)
            .size(size)
            .font(MONO)
            .height(Length::Fill)
            .padding(z(26.0))
            .style(move |_: &Theme, _s: text_editor::Status| text_editor::Style { background: iced::Background::Color(Color::TRANSPARENT), border: iced::Border::default(), placeholder: p.muted, value: p.text, selection: Pal::a(p.accent, 0.35) });
        return container(ed).height(Length::Fill).into();
    }
    let blocks = render(a, d, None);
    scrollable(container(blocks).padding(pad(z(26.0), z(36.0), z(40.0), z(36.0))).width(Length::Fill)).height(Length::Fill).into()
}

/// The note's blocks as the live preview (and Read) draw them. `marks`: search matches to highlight (line, ranges).
pub fn render<'a>(a: &'a App, d: &'a Doc, marks: Option<&[(usize, Vec<(usize, usize)>)]>) -> E<'a, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let size = a.prefs.font_size * 0.88;
    let fnt = body_font(a);
    let live = a.mode == Mode::Live && !d.generated;
    let parsed = md::parse_lines(&d.lines);
    let mut c = Column::new().spacing(z(6.0)).width(Length::Fill);
    for b in &parsed {
        let editing = live && d.cur.map(|cur| cur >= b.first && cur <= b.last).unwrap_or(false);
        if editing {
            // The block the cursor is in, raw: its lines, the cursor's one typed in.
            let cur = d.cur.unwrap_or(b.first);
            for li in b.first..=b.last {
                if li == cur {
                    let f2 = if matches!(b.block, Block::Code { .. }) { MONO } else { fnt };
                    let input = text_input("", &d.lines[li]).id(LINE_ID).on_input(Msg::LineEdit).on_submit(Msg::LineEnter).size(size).font(f2).padding(pad(1.0, 0.0, 1.0, 0.0)).style(look::bare_style(&p));
                    c = c.push(input);
                    if let Some(list) = autocomplete(a) {
                        c = c.push(ac_list(a, list, d.ac));
                    }
                } else {
                    c = c.push(click(text(d.lines[li].clone()).size(size).font(MONO).color(tn.muted), Some(Msg::LineClick(li))));
                }
            }
            continue;
        }
        let mk = marks.map(|m| m.iter().filter(|x| x.0 >= b.first && x.0 <= b.last).flat_map(|x| x.1.clone()).collect::<Vec<_>>()).unwrap_or_default();
        let el: E<Msg> = match &b.block {
            Block::Blank => gap(1.0, size * 0.45),
            Block::Heading { level, text: h } => {
                let sz = match level {
                    1 => size * 1.85,
                    2 => size * 1.35,
                    3 => size * 1.15,
                    _ => size * 1.05,
                };
                container(inline(a, h, sz, tn.text, bold(fnt), &mk)).padding(pad(if *level <= 2 { z(14.0) } else { z(6.0) }, 0.0, z(4.0), 0.0)).into()
            }
            Block::Para(s) => {
                let meta = s.trim_start().starts_with("Status:") || s.trim_start().starts_with("Belongs to:");
                if meta {
                    inline(a, s, size * 0.88, tn.muted, fnt, &mk)
                } else if only_tags(s) {
                    tag_chips(a, s)
                } else {
                    inline(a, s, size, Pal::mix(tn.text, tn.muted, 0.08), fnt, &mk)
                }
            }
            Block::Bullet { indent, mark, text: s } => {
                let dot: E<Msg> = if mark.ends_with('.') || mark.ends_with(')') { t(mark.clone(), size, tn.muted).into() } else { container(look::dot(p.accent, z(7.0))).padding(pad(size * 0.5, 0.0, 0.0, 0.0)).into() };
                row![gap(z(14.0) + *indent as f32 * 6.0, 1.0), dot, gap(z(12.0), 1.0), container(inline(a, s, size, Pal::mix(tn.text, tn.muted, 0.08), fnt, &mk)).width(Length::Fill)].align_y(iced::Top).into()
            }
            Block::Task { indent, state, text: s } => {
                let done = *state == 'x';
                let doing = *state == '/';
                let bx_: E<Msg> = container(if done { glyph(G::Check, p.ink, z(14.0), 3.0) } else if doing { t("–", f(14.0), p.accent).into() } else { gap(1.0, 1.0) })
                    .center_x(Length::Fixed(z(22.0)))
                    .center_y(Length::Fixed(z(22.0)))
                    .style(bx(if done { p.accent } else { Color::TRANSPARENT }, 4.0, if done { None } else { Some((tn.muted, 1.2)) }))
                    .into();
                let label = if done { strike(inline(a, s, size, tn.muted, fnt, &mk), tn.muted) } else { inline(a, s, size, Pal::mix(tn.text, tn.muted, 0.08), fnt, &mk) };
                let li = b.first;
                row![gap(*indent as f32 * 6.0, 1.0), click(bx_, if d.generated { None } else { Some(Msg::TaskToggle(li)) }), gap(z(14.0), 1.0), container(label).width(Length::Fill)].align_y(iced::Center).into()
            }
            Block::Code { lang, lines } => {
                let mut cc = Column::new().spacing(2.0);
                for l in lines {
                    cc = cc.push(code_line(a, l, size * 0.9));
                }
                let head = row![Space::new().width(Length::Fill), t(lang.clone(), f(13.0), tn.faint)];
                container(column![head, cc].spacing(z(2.0))).padding(pad(z(14.0), z(20.0), z(18.0), z(20.0))).width(Length::Fill).style(bx(Pal::mix(tn.page, tn.card, 0.55), 8.0, Some((tn.line, 1.0)))).into()
            }
            Block::Image { target, .. } => picture(a, &d.rel, target, size),
            Block::Quote(s) => row![container(Space::new()).width(Length::Fixed(3.0)).height(Length::Fixed(size * 1.6)).style(bx(Pal::a(p.accent, 0.6), 1.0, None)), gap(z(14.0), 1.0), inline(a, s, size, tn.muted, fnt, &mk)].align_y(iced::Center).into(),
            Block::Rule => container(ui::hline(tn.line)).padding(pad(z(10.0), 0.0, z(10.0), 0.0)).into(),
            Block::Front(kv) => {
                let mut cc = Column::new().spacing(z(4.0));
                for (k, v) in kv {
                    cc = cc.push(row![container(t1(k.clone(), f(14.0), tn.muted)).width(Length::Fixed(z(160.0))), t(v.clone(), f(14.0), tn.text)]);
                }
                container(cc).padding(z(12.0)).width(Length::Fill).style(bx(Pal::mix(tn.page, tn.card, 0.5), 8.0, Some((tn.line, 1.0)))).into()
            }
        };
        if live && !matches!(b.block, Block::Task { .. }) {
            c = c.push(click(el, Some(Msg::LineClick(b.first))));
        } else {
            c = c.push(el);
        }
    }
    if live && d.cur.is_none() {
        // Clicking below the last line types at the end.
        c = c.push(click(Space::new().width(Length::Fill).height(Length::Fixed(z(120.0))), Some(Msg::LineClick(d.lines.len().saturating_sub(1)))));
    }
    c.into()
}

fn only_tags(s: &str) -> bool {
    let sp = md::spans(s);
    !sp.is_empty() && sp.iter().all(|x| matches!(x, Span::Tag(_)) || matches!(x, Span::Text(t) if t.trim().is_empty()))
}

fn tag_chips<'a>(a: &'a App, s: &str) -> E<'a, Msg> {
    let p = a.p;
    let mut r = Row::new().spacing(z(10.0));
    for sp in md::spans(s) {
        if let Span::Tag(tg) = sp {
            r = r.push(look::chip(&p, format!("#{tg}"), look::Chip::Tag));
        }
    }
    r.into()
}

/// A strike through `inner` (a done checklist line): the line sits on the text, as wide as the text.
fn strike<'a>(inner: E<'a, Msg>, c: Color) -> E<'a, Msg> {
    stack![inner, container(ui::hline(c)).height(Length::Fill).center_y(Length::Fill)].into()
}

fn code_line<'a>(a: &'a App, l: &str, size: f32) -> E<'a, Msg> {
    let tn = tone(&a.p);
    // A comment after # is muted (the mockup's "# store.html → build/store-N-*.png").
    match l.find(" #") {
        Some(at) => row![text(l[..at].to_string()).size(size).font(MONO).color(tn.text), text(l[at..].to_string()).size(size).font(MONO).color(tn.faint)].into(),
        None => text(l.to_string()).size(size).font(MONO).color(tn.text).into(),
    }
}

/// A picture's size from its header (PNG and JPEG), for its caption.
pub fn picture_size(path: &std::path::Path) -> Option<(u32, u32)> {
    use std::io::Read;
    let mut f = std::fs::File::open(path).ok()?;
    let mut b = vec![0u8; 64 * 1024];
    let n = f.read(&mut b).ok()?;
    let b = &b[..n];
    if b.len() > 24 && &b[1..4] == b"PNG" {
        return Some((u32::from_be_bytes([b[16], b[17], b[18], b[19]]), u32::from_be_bytes([b[20], b[21], b[22], b[23]])));
    }
    if b.len() > 4 && b[0] == 0xFF && b[1] == 0xD8 {
        let mut i = 2;
        while i + 9 < b.len() {
            if b[i] != 0xFF {
                i += 1;
                continue;
            }
            let mk = b[i + 1];
            let len = u16::from_be_bytes([b[i + 2], b[i + 3]]) as usize;
            if (0xC0..=0xC3).contains(&mk) {
                return Some((u16::from_be_bytes([b[i + 7], b[i + 8]]) as u32, u16::from_be_bytes([b[i + 5], b[i + 6]]) as u32));
            }
            i += 2 + len;
        }
    }
    None
}

fn picture<'a>(a: &'a App, from: &str, target: &str, size: f32) -> E<'a, Msg> {
    let p = a.p;
    let tn = tone(&p);
    match a.vault.resolve_file(target, from) {
        Some(path) => {
            let (w, h) = picture_size(&path).unwrap_or((16, 10));
            let shown_w = z(560.0);
            let shown_h = shown_w * h as f32 / w.max(1) as f32;
            let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            let dir = path.parent().and_then(|d| d.file_name()).map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            let img = image(image::Handle::from_path(&path)).width(Length::Fixed(shown_w)).height(Length::Fixed(shown_h.min(z(700.0)))).content_fit(iced::ContentFit::Contain);
            column![
                container(img).padding(1.0).style(bx(tn.card, 6.0, Some((tn.line, 1.0)))),
                t(format!("{name} · {w} × {h} · from {dir}/"), f(13.0), tn.faint)
            ]
            .spacing(z(8.0))
            .into()
        }
        None => container(t(format!("![[{target}]] (not in this vault)"), size * 0.9, tn.faint)).padding(z(10.0)).style(bx(tn.card, 6.0, Some((tn.line, 1.0)))).into(),
    }
}

/// A line's inline marks, wrapped: words in a wrapping row, links in the accent (a click opens them), tags as chips,
/// code in mono, search matches highlighted.
pub fn inline<'a>(a: &'a App, s: &str, size: f32, color: Color, fnt: Font, marks: &[(usize, usize)]) -> E<'a, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let spans = md::spans(s);
    if marks.is_empty() && spans.iter().all(|x| matches!(x, Span::Text(_))) {
        return text(s.to_string()).size(size).color(color).font(fnt).into();
    }
    let space = size * 0.27;
    let mut items: Vec<E<'a, Msg>> = Vec::new();
    // Matches are byte ranges into `s`; each span's text is found in `s` again to know where it is.
    let mut pos = 0usize;
    let push_words = |items: &mut Vec<E<'a, Msg>>, txt: &str, start: usize, style: &dyn Fn(String, bool) -> E<'a, Msg>| {
        let mut i = 0;
        let b = txt.as_bytes();
        while i < txt.len() {
            if b[i].is_ascii_whitespace() {
                items.push(gap(space, 1.0));
                while i < txt.len() && b[i].is_ascii_whitespace() {
                    i += 1;
                }
                continue;
            }
            let st = i;
            while i < txt.len() && !b[i].is_ascii_whitespace() {
                i += 1;
            }
            let w = &txt[st..i];
            let (ws, we) = (start + st, start + i);
            let hit = marks.iter().any(|(ms, me)| *ms < we && *me > ws);
            items.push(style(w.to_string(), hit));
        }
    };
    for sp in spans {
        let (raw_len, find): (usize, String) = match &sp {
            Span::Text(x) | Span::Bold(x) | Span::Italic(x) | Span::Strike(x) | Span::Code(x) | Span::Embed(x) => (0, x.clone()),
            Span::Wiki { label, .. } => (0, label.clone()),
            Span::Url { label, .. } => (0, label.clone()),
            Span::Tag(x) => (0, format!("#{x}")),
        };
        let _ = raw_len;
        let start = s[pos.min(s.len())..].find(find.as_str()).map(|x| x + pos).unwrap_or(pos);
        pos = start + find.len();
        let hl = move |e: E<'a, Msg>, hit: bool| -> E<'a, Msg> {
            if hit {
                container(e).padding(pad(0.0, 2.0, 0.0, 2.0)).style(bx(Pal::a(p.accent, 0.35), 3.0, Some((Pal::a(p.accent, 0.8), 1.0)))).into()
            } else {
                e
            }
        };
        match sp {
            Span::Text(x) => push_words(&mut items, &x, start, &|w, hit| hl(text(w).size(size).color(color).font(fnt).into(), hit)),
            Span::Bold(x) => push_words(&mut items, &x, start, &|w, hit| hl(text(w).size(size).color(tn.text).font(bold(fnt)).into(), hit)),
            Span::Italic(x) => push_words(&mut items, &x, start, &|w, hit| hl(text(w).size(size).color(color).font(Font { style: font::Style::Italic, ..fnt }).into(), hit)),
            Span::Strike(x) => push_words(&mut items, &x, start, &|w, hit| hl(strike(text(w).size(size).color(tn.muted).font(fnt).into(), tn.muted), hit)),
            Span::Code(x) => items.push(hl(container(text(x).size(size * 0.88).font(MONO).color(accent2(&p))).padding(pad(0.0, 4.0, 0.0, 4.0)).style(bx(Pal::mix(tn.page, tn.card, 0.7), 3.0, None)).into(), false)),
            Span::Tag(x) => items.push(look::chip(&p, format!("#{x}"), look::Chip::Tag)),
            Span::Wiki { target, label } => {
                let rel = a.vault.resolve(&target).map(|i| a.vault.notes[i].rel.clone());
                let c2 = if rel.is_some() { accent2(&p) } else { Pal::a(accent2(&p), 0.6) };
                let m = rel.map(Msg::OpenRel);
                push_words(&mut items, &label, start, &|w, hit| hl(click(text(w).size(size).color(c2).font(fnt), m.clone()), hit));
            }
            Span::Embed(x) => items.push(text(format!("![[{x}]]")).size(size).color(accent2(&p)).font(fnt).into()),
            Span::Url { label, .. } => push_words(&mut items, &label, start, &|w, hit| hl(text(w).size(size).color(accent2(&p)).font(fnt).into(), hit)),
        }
    }
    Row::with_children(items).align_y(iced::Center).wrap().into()
}

/// The `[[` autocomplete's list for the line being typed (None: no `[[` open).
pub fn autocomplete(a: &App) -> Option<Vec<String>> {
    let d = a.doc.as_ref()?;
    let c = d.cur?;
    let q = md::open_link(d.lines.get(c)?)?;
    let mut v: Vec<(i32, String)> = a
        .vault
        .notes
        .iter()
        .filter_map(|n| {
            let target = if n.rel.ends_with("/README.md") { n.rel.trim_end_matches("/README.md").to_string() } else { n.rel.trim_end_matches(".md").to_string() };
            hearth_notes::search::fuzzy(&q, &target).map(|(s, _)| (s, target))
        })
        .collect();
    // Folders (items) are link targets too.
    for it in a.vault.items() {
        if !v.iter().any(|x| x.1 == it) {
            if let Some((s, _)) = hearth_notes::search::fuzzy(&q, &it) {
                v.push((s, it));
            }
        }
    }
    v.sort_by(|x, y| y.0.cmp(&x.0).then(x.1.cmp(&y.1)));
    v.truncate(6);
    if v.is_empty() {
        return None;
    }
    Some(v.into_iter().map(|x| x.1).collect())
}

fn ac_list<'a>(a: &'a App, list: Vec<String>, sel: usize) -> E<'a, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let mut c = Column::new().spacing(2.0);
    for (i, it) in list.into_iter().enumerate() {
        let on = i == sel.min(5);
        let r = container(row![t("[[", f(13.0), tn.faint), gap(z(6.0), 1.0), t1(it.clone(), f(15.0), if on { tn.text } else { tn.muted })].align_y(iced::Center)).padding(pad(z(6.0), z(12.0), z(6.0), z(12.0))).width(Length::Fill).style(bx(if on { Pal::mix(tn.card, p.accent, 0.16) } else { Color::TRANSPARENT }, 5.0, None));
        c = c.push(click(r, Some(Msg::AcPick(it))));
    }
    let hint = t("↑↓ pick · Enter or Tab links it · Esc", f(12.0), tn.faint);
    container(column![c, hint].spacing(z(6.0))).padding(z(8.0)).width(Length::Fixed(z(460.0))).style(bx(tn.card, 8.0, Some((tn.line, 1.0)))).into()
}

fn status_bar(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let Some(d) = a.doc.as_ref() else { return gap(1.0, 1.0) };
    let text_ = d.text();
    let i = a.vault.find(&d.rel);
    let back = i.map(|i| a.vault.backlinks(i).len()).unwrap_or(0);
    let props = md::properties(&text_).len();
    let words = md::words(&text_);
    let chars = text_.chars().count();
    let (dot, said) = if !d.error.is_empty() {
        (look::BAD, "Not saved".to_string())
    } else if d.generated {
        (tn.faint, "Made from the vault".to_string())
    } else if d.saved {
        (look::GOOD, "Saved".to_string())
    } else {
        (look::WARN, "Saving…".to_string())
    };
    let r = row![
        t1(format!("↩ {back} backlink{}", crate::plural(back)), f(14.0), tn.muted),
        gap(z(26.0), 1.0),
        t1(format!("☰ {props} propert{}", if props == 1 { "y" } else { "ies" }), f(14.0), tn.muted),
        gap(z(26.0), 1.0),
        t1(format!("{} words", thousands(words)), f(14.0), tn.muted),
        gap(z(26.0), 1.0),
        t1(format!("{} characters", thousands(chars)), f(14.0), tn.muted),
        Space::new().width(Length::Fill),
        look::dot(dot, z(9.0)),
        gap(z(8.0), 1.0),
        t1(said, f(14.0), tn.muted),
    ]
    .align_y(iced::Center);
    column![ui::hline(tn.line), container(r).padding(pad(0.0, z(26.0), 0.0, z(26.0))).height(Length::Fixed(z(42.0))).center_y(Length::Fixed(z(42.0)))].into()
}

pub fn thousands(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

// ------------------------------------------------------------------ the right column
pub fn right_column(a: &App, today_page: bool) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let mut c = Column::new().spacing(z(10.0)).width(Length::Fill);
    if !today_page {
        c = c.push(row![
            look::chip(&p, "▦ Calendar", look::Chip::Outline),
            look::chip(&p, "☰ Outline", look::Chip::Plain),
            look::chip(&p, "↩ Links", look::Chip::Plain),
            look::chip(&p, "# Tags", look::Chip::Plain)
        ]
        .spacing(z(8.0)));
    }
    c = c.push(calendar(a));
    if today_page {
        c = c.push(crate::today::right(a));
    } else {
        c = c.push(outline(a));
        c = c.push(backlinks(a));
        c = c.push(tags(a));
        c = c.push(modified_today(a));
    }
    container(scrollable(container(c).padding(pad(z(16.0), z(20.0), z(16.0), z(20.0))))).width(Length::Fixed(z(424.0))).height(Length::Fill).style(bx(Pal::mix(tn.page, tn.side, 0.6), 0.0, None)).into()
}

pub fn rh<'a>(p: &Pal, s: impl Into<String>, right: Option<String>) -> E<'a, Msg> {
    let tn = tone(p);
    let mut r = row![ts(s.into(), f(17.0), tn.text)].align_y(iced::Center);
    if let Some(x) = right {
        r = r.push(Space::new().width(Length::Fill)).push(t(x, f(14.0), tn.muted));
    }
    container(r).padding(pad(z(12.0), 0.0, z(2.0), 0.0)).into()
}

/// The month calendar: ISO weeks, Monday first, a dot on days with a log, Today.
pub fn calendar(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let (y, m) = a.cal;
    let head = row![
        tb(day::SHORT[(m - 1) as usize], f(21.0), tn.text),
        gap(z(6.0), 1.0),
        tb(y.to_string(), f(21.0), accent2(&p)),
        Space::new().width(Length::Fill),
        click(t("‹", f(15.0), tn.muted), Some(Msg::Cal(-1))),
        gap(z(10.0), 1.0),
        click(container(t("TODAY", f(11.0), tn.muted)).padding(pad(1.0, z(10.0), 1.0, z(10.0))).style(bx(Color::TRANSPARENT, z(10.0), Some((tn.line, 1.0)))), Some(Msg::Cal(0))),
        gap(z(10.0), 1.0),
        click(t("›", f(15.0), tn.muted), Some(Msg::Cal(1))),
    ]
    .align_y(iced::Center);
    let cell = z(52.0);
    let mut grid = Column::new().spacing(z(2.0));
    let mut hr = Row::new().push(container(t("W", f(11.0), tn.faint)).center_x(Length::Fixed(z(40.0))));
    for d in ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"] {
        hr = hr.push(container(t(d, f(11.0), tn.faint)).center_x(Length::Fixed(cell)));
    }
    grid = grid.push(hr);
    for w in day::month_grid(y, m) {
        let mut r = Row::new().align_y(iced::Center).push(container(t(day::iso_week(w[0].0).to_string(), f(12.0), tn.faint)).center_x(Length::Fixed(z(40.0))));
        for (d, inm) in w {
            let is_today = d == a.today;
            let has_log = a.vault.find(&day::log_path(d)).is_some();
            let shown = d == a.day && a.page == crate::Page::Today;
            let num = container(tm(d.2.to_string(), f(14.0), if is_today { p.ink } else if inm { tn.text } else { tn.faint }))
                .center_x(Length::Fixed(cell - z(8.0)))
                .center_y(Length::Fixed(z(34.0)))
                .style(bx(if is_today { p.accent } else if shown { Pal::a(p.accent, 0.2) } else { Color::TRANSPARENT }, 6.0, None));
            let dot: E<Msg> = if has_log { look::dot(if is_today { p.ink } else { p.accent }, z(6.0)) } else { gap(z(6.0), z(6.0)) };
            let cellc = container(stack![num, container(dot).center_x(Length::Fixed(cell - z(8.0))).align_bottom(Length::Fixed(z(34.0))).padding(pad(0.0, 0.0, z(3.0), 0.0))]).center_x(Length::Fixed(cell));
            r = r.push(click(cellc, if has_log || is_today { Some(Msg::CalDay(d)) } else { None }));
        }
        grid = grid.push(r);
    }
    container(column![head, grid].spacing(z(10.0))).padding(z(14.0)).style(bx(tn.card, 10.0, Some((tn.line, 1.0)))).into()
}

pub(crate) fn outline(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let mut c = Column::new().push(rh(&p, "Outline", None)).spacing(z(6.0));
    let Some(d) = a.doc.as_ref() else { return c.into() };
    let hs = md::headings(&d.text());
    let cur = d.cur.unwrap_or(0);
    let here = hs.iter().rposition(|h| h.2 <= cur).filter(|_| d.cur.is_some());
    for (i, (lvl, name, line)) in hs.iter().enumerate() {
        let on = here == Some(i);
        let mut r = row![gap(z(22.0) * (*lvl as f32 - 1.0), 1.0)].align_y(iced::Center);
        if on {
            r = r.push(look::dot(p.accent, z(10.0))).push(gap(z(8.0), 1.0));
        }
        r = r.push(t1(name.clone(), f(15.5), if on { accent2(&p) } else { Pal::mix(tn.text, tn.muted, 0.15) }));
        c = c.push(click(r, Some(Msg::Outline(*line))));
    }
    if hs.is_empty() {
        c = c.push(t("No headings in this note", f(14.0), tn.faint));
    }
    c.into()
}

pub(crate) fn backlinks(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let Some(i) = a.open_index() else { return column![rh(&p, "Backlinks", Some("0".into()))].into() };
    let bl = a.vault.backlinks(i);
    let mut c = Column::new().push(rh(&p, "Backlinks", Some(bl.len().to_string()))).spacing(z(8.0));
    let me = &a.vault.notes[i];
    let name = me.label.clone();
    for (j, line) in bl.iter().take(3) {
        let src = &a.vault.notes[*j];
        let snippet = snippet_around(line, &name, 46);
        let (pre, hit, post) = split_hit(&snippet, &name);
        let lr = row![t1(pre, f(14.0), tn.muted), t1(hit, f(14.0), accent2(&p)), t1(post, f(14.0), tn.muted)];
        let card = container(column![t1(src.rel.clone(), f(15.0), tn.text), lr].spacing(z(4.0))).padding(pad(z(10.0), z(14.0), z(10.0), z(14.0))).width(Length::Fill).style(bx(tn.card, 8.0, Some((tn.line, 1.0))));
        c = c.push(click(card, Some(Msg::OpenNote(*j))));
    }
    let more = bl.len().saturating_sub(3);
    let un = a.vault.unlinked(i);
    if more > 0 || un > 0 {
        c = c.push(t(format!("{}Unlinked mentions: {un}", if more > 0 { format!("+{more} more · ") } else { String::new() }), f(13.0), tn.muted));
    }
    c.into()
}

/// "…mockups approved — hearth-store"
pub fn snippet_around(line: &str, word: &str, width: usize) -> String {
    let l = line.trim();
    let at = l.to_lowercase().find(&word.to_lowercase()).unwrap_or(0);
    let chars: Vec<char> = l.chars().collect();
    let pos = l[..at].chars().count();
    let start = pos.saturating_sub(width.saturating_sub(word.chars().count() + 4));
    let end = (start + width).min(chars.len());
    let mut s: String = chars[start..end].iter().collect();
    if start > 0 {
        s = format!("…{s}");
    }
    s
}

pub fn split_hit(s: &str, word: &str) -> (String, String, String) {
    match s.to_lowercase().find(&word.to_lowercase()) {
        Some(at) if s.is_char_boundary(at) && s.is_char_boundary(at + word.len()) => (s[..at].to_string(), s[at..at + word.len()].to_string(), s[at + word.len()..].to_string()),
        _ => (s.to_string(), String::new(), String::new()),
    }
}

fn tags(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let counts = a.vault.tag_counts();
    let mine: Vec<String> = a.open_index().map(|i| a.vault.notes[i].tags.clone()).unwrap_or_default();
    let mut list: Vec<(String, usize)> = mine.iter().filter_map(|t| counts.iter().find(|c| c.0.eq_ignore_ascii_case(t)).cloned()).collect();
    for c in &counts {
        if list.len() >= 5 {
            break;
        }
        if !list.iter().any(|x| x.0.eq_ignore_ascii_case(&c.0)) {
            list.push(c.clone());
        }
    }
    let mut r = Row::new().spacing(z(8.0));
    for (tg, n) in list.into_iter().take(5) {
        let chip = container(row![t1(format!("#{tg}"), f(14.0), accent2(&p)), gap(z(8.0), 1.0), t1(n.to_string(), f(13.0), tn.muted)].align_y(iced::Center)).padding(pad(z(4.0), z(12.0), z(4.0), z(12.0))).style(bx(Pal::a(p.accent, 0.14), z(12.0), None));
        r = r.push(click(chip, Some(Msg::SearchText(format!("#{tg}")))));
    }
    column![rh(&p, "Tags", None), r.wrap()].spacing(z(8.0)).into()
}

pub fn modified_today(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let mut list: Vec<(i64, usize)> = a.vault.notes.iter().enumerate().filter(|(_, n)| day::log_day(n.mtime, a.prefs.day_start) == a.today).map(|(i, n)| (n.mtime, i)).collect();
    list.sort_by(|x, y| y.0.cmp(&x.0));
    let mut c = Column::new().push(rh(&p, "Modified today", Some(day::short(a.today)))).spacing(z(8.0));
    for (mt, i) in list.iter().take(5) {
        let n = &a.vault.notes[*i];
        let name = if n.rel.ends_with("/README.md") { format!("{}/README.md", n.rel.rsplit('/').nth(1).unwrap_or("")) } else if n.rel.starts_with("log/") { n.rel.clone() } else { n.rel.rsplit('/').next().unwrap_or(&n.rel).trim_end_matches(".md").to_string() };
        let ct = day::central(*mt);
        let r = row![glyph(G::Doc, tn.muted, z(16.0), 1.8), gap(z(10.0), 1.0), t1(name, f(15.0), Pal::mix(tn.text, tn.muted, 0.12)), Space::new().width(Length::Fill), t1(day::clock(ct.hour, ct.min).trim_end_matches(" AM").trim_end_matches(" PM").to_string(), f(14.0), tn.muted)].align_y(iced::Center);
        c = c.push(click(r, Some(Msg::OpenNote(*i))));
    }
    if list.is_empty() {
        c = c.push(t("Nothing changed today yet", f(14.0), tn.faint));
    }
    c.into()
}

// ------------------------------------------------------------------ the Atlas
/// The Atlas's maps of content, made from the vault (read only): a project's items by kind, or the daily logs.
pub fn atlas_page(a: &App, which: &str) -> String {
    let mut s = String::new();
    if which == "logs" {
        s.push_str("# Daily logs · map\n\nEvery day's log, newest first.\n\n");
        let mut logs: Vec<&hearth_notes::vault::Note> = a.vault.notes.iter().filter(|n| n.log.is_some()).collect();
        logs.sort_by(|x, y| y.rel.cmp(&x.rel));
        for n in logs {
            let d = n.log.unwrap_or((1970, 1, 1));
            s.push_str(&format!("- [[{}]] · {} · {} lines\n", n.rel.trim_end_matches(".md"), day::long(d), n.text.lines().filter(|l| !l.trim().is_empty()).count()));
        }
        return s;
    }
    s.push_str(&format!("# {which} · map\n\nEvery item in {which}/, by kind.\n"));
    for k in hearth_notes::vault::KINDS {
        let items: Vec<String> = a.vault.items().into_iter().filter(|it| it.starts_with(&format!("{which}/{k}/"))).collect();
        if items.is_empty() {
            continue;
        }
        s.push_str(&format!("\n## {k} ({})\n", items.len()));
        for it in items {
            let status = a.vault.item_note(&it).and_then(|i| md::properties(&a.vault.notes[i].text).into_iter().find(|x| x.0 == "Status").map(|x| x.1)).unwrap_or_default();
            if status.is_empty() {
                s.push_str(&format!("- [[{it}]]\n"));
            } else {
                s.push_str(&format!("- [[{it}]] · {}\n", status.split_whitespace().next().unwrap_or("")));
            }
        }
    }
    s
}
