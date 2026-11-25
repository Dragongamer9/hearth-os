//! The window around every page (mockups 1–8): the title bar, the thin icon rail, the sidebar (the vault, the folder
//! tree, search or Settings' sections, the sync line and This PC), the page, and the controller-hint strip.

use crate::look::{self, bx, click, f, pad, tone, z, E};
use crate::{App, Msg, Page, VaultState};
use hearth_notes::vault::PROJECTS;
use hearth_ui::ui::{self, gap, glyph, t, t1, tb, tm, G};
use hearth_ui::Pal;
use iced::widget::{column, container, mouse_area, row, scrollable, stack, text, Column, Space};
use iced::{Color, Length, Theme};

/// Hearth Notes' blocks (hearth_ui::blocks).
/// The window, and every page's own parts (each page's file puts them together the app's own way).
use hearth_ui::blocks::{bl, bls, PageDef};
pub const BLOCKS: &[PageDef] = &[
    PageDef { id: "window", title: "Hearth Notes", blocks: &[bl("rail", "The icon rail", 0, 0, 1, 12), bl("side", "The sidebar", 1, 0, 3, 12), bl("page", "The page", 4, 0, 8, 12)] },
    PageDef {
        id: "files",
        title: "Files",
        blocks: &[
            bl("tabs", "The tabs", 0, 0, 5, 1),
            bl("crumb", "Where the note is", 0, 1, 5, 1),
            bl("note", "The note", 0, 2, 5, 9),
            bl("status", "The status bar", 0, 11, 5, 1),
            bl("graph", "The graph", 5, 0, 4, 12),
            bl("right", "Backlinks and outline", 9, 0, 3, 12),
        ],
    },
    PageDef { id: "search", title: "Search", blocks: &[bl("tabs", "The tabs", 0, 0, 12, 1), bl("crumb", "Where the note is", 0, 1, 12, 1), bl("doc", "The note with its matches", 0, 2, 12, 10)] },
    PageDef { id: "today", title: "Today", blocks: &[bl("tabs", "The tabs", 0, 0, 9, 1), bl("crumb", "Where the note is", 0, 1, 9, 1), bl("dashboard", "Today", 0, 2, 9, 10), bl("right", "Backlinks and outline", 9, 0, 3, 12)] },
    PageDef { id: "board", title: "Board", blocks: &[bl("tabs", "The tabs", 0, 0, 12, 1), bl("head", "Group by and the filters", 0, 1, 12, 1), bl("columns", "The columns", 0, 2, 12, 9), bl("foot", "The line at the bottom", 0, 11, 12, 1)] },
    PageDef { id: "graph", title: "Graph", blocks: &[bl("graph", "The graph", 0, 0, 9, 12), bls("customise", "Customise the graph", 9, 0, 3, 12)] },
    PageDef { id: "modes", title: "Graph modes", blocks: &[bl("head", "Title", 0, 0, 12, 1), bls("grid", "The modes", 0, 1, 12, 8), bl("bottom", "Reduce motion, Customise, Stays light", 0, 9, 12, 3), bls("customise", "Customise the graph", 9, 0, 3, 12)] },
    PageDef {
        id: "sync",
        title: "Sync",
        blocks: &[
            bl("title", "Title", 0, 0, 12, 1),
            bls("secrets", "Looks like a secret", 0, 1, 12, 1),
            bls("changes", "Changes", 0, 2, 6, 4),
            bls("check", "The workspace check", 0, 6, 6, 3),
            bls("commit", "Commit and push", 0, 9, 6, 3),
            bls("diff", "What changed", 6, 2, 6, 4),
            bls("auto", "Sync by itself", 6, 6, 6, 3),
            bls("tidy", "Keep it tidy", 6, 9, 6, 3),
        ],
    },
    PageDef {
        id: "conflict",
        title: "A note changed on two PCs",
        blocks: &[bl("title", "Title", 0, 0, 12, 1), bl("sub", "What happened", 0, 1, 12, 1), bls("panes", "Both versions", 0, 2, 12, 6), bl("choices", "What to keep", 0, 8, 12, 2), bl("safe", "Nothing is lost", 0, 10, 12, 2)],
    },
    PageDef {
        id: "settings",
        title: "Settings",
        blocks: &[
            bl("title", "Title", 0, 0, 12, 1),
            bls("github", "GitHub", 0, 1, 12, 3),
            bls("sync", "Sync", 0, 4, 12, 2),
            bls("editor", "Editor", 0, 6, 6, 4),
            bls("looks", "Looks and Today", 6, 6, 6, 4),
            bls("about", "About", 0, 10, 12, 1),
            bl("unreadable", "The settings file couldn't be read", 0, 11, 12, 1),
        ],
    },
];

pub fn window(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let body: E<Msg> = match &a.vs {
        VaultState::Ready => page(a),
        _ => first_run(a),
    };
    // its blocks (hearth_ui::blocks): the rail, the sidebar and the page, moved, sized, hidden or restyled; as it was
    // when nothing is
    let main = hearth_ui::arrange::parts(
        &a.describe,
        &p,
        "window",
        vec![("rail", rail(a)), ("side", sidebar(a)), ("page", container(body).width(Length::Fill).height(Length::Fill).into())],
        |ps| row![ps.take("rail"), ps.take("side"), ps.take("page")].height(Length::Fill).into(),
        Msg::Describe,
    );
    let mut col = column![title(a), ui::hline(tn.line), main, hints_for(a)];
    if !a.note.is_empty() {
        col = col.push(container(t(a.note.clone(), f(15.0), if a.note_bad { look::BAD } else { tn.muted })).padding(pad(4.0, z(30.0), 6.0, z(30.0))).width(Length::Fill).style(bx(tn.page, 0.0, None)));
    }
    let base: E<Msg> = container(col).width(Length::Fill).height(Length::Fill).style(bx(tn.page, 0.0, None)).into();
    let mut out = ui::edges(base, Msg::Resize);
    if a.qs.is_some() {
        out = stack![out, crate::searchv::switcher(a)].into();
    }
    if a.new_item.is_some() {
        out = ui::dialog(out, crate::newitemv::view(a), Some(Msg::NewCancel));
    }
    out
}

fn page(a: &App) -> E<'_, Msg> {
    match a.page {
        Page::Files => crate::editor::view(a),
        Page::Search => crate::searchv::content(a),
        Page::Today => crate::today::view(a),
        Page::Board => crate::boardv::view(a),
        Page::Graph => crate::gview::page(a),
        Page::Modes => crate::gview::modes(a),
        Page::Sync => crate::syncv::view(a),
        Page::Conflict => crate::syncv::conflict(a),
        Page::Settings => crate::settingsv::view(a),
    }
}

/// The title bar: the mark, "Hearth Notes · hearth-workspace", the window's buttons.
fn title(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let wbtn = |g: G, m: Msg, red: bool| -> E<'static, Msg> {
        iced::widget::button(container(glyph(g, tn.muted, 10.0, 2.2)).center_x(Length::Fixed(44.0)).center_y(Length::Fixed(z(60.0))))
            .padding(0.0)
            .on_press(m)
            .style(move |_: &Theme, st: iced::widget::button::Status| iced::widget::button::Style {
                background: Some(iced::Background::Color(match st {
                    iced::widget::button::Status::Hovered | iced::widget::button::Status::Pressed if red => Color::from_rgb8(0xc4, 0x2b, 0x1c),
                    iced::widget::button::Status::Hovered | iced::widget::button::Status::Pressed => p.hover(),
                    _ => Color::TRANSPARENT,
                })),
                text_color: tn.text,
                border: iced::Border::default(),
                shadow: iced::Shadow::default(),
                snap: true,
            })
            .into()
    };
    let name = row![gap(z(22.0), 1.0), look::mark(&p, z(34.0)), gap(z(16.0), 1.0), tb("Hearth Notes", f(19.0), tn.text), gap(z(10.0), 1.0), t1(format!("· {}", a.vault_name()), f(15.0), tn.muted)].align_y(iced::Center);
    let drag = mouse_area(container(row![name, Space::new().width(Length::Fill)].align_y(iced::Center)).width(Length::Fill).center_y(Length::Fixed(z(60.0)))).on_press(Msg::Drag).on_double_click(Msg::Maximize).on_right_press(Msg::Describe(hearth_ui::describe::DMsg::Open));
    let mut r = row![drag, wbtn(G::Minimize, Msg::Minimize, false), wbtn(G::Maximize, Msg::Maximize, false), wbtn(G::Close, Msg::Close, true)].align_y(iced::Center);
    if let Some(fm) = ui::frame_meter() {
        r = r.push(fm);
    }
    container(r).width(Length::Fill).height(Length::Fixed(z(60.0))).style(bx(tn.side, 0.0, None)).into()
}

/// The icon rail: files, search, today, board, graph, sync, new item; settings and help at the bottom.
fn rail(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let on_page = |pg: Page| -> bool {
        match pg {
            Page::Graph => matches!(a.page, Page::Graph | Page::Modes),
            Page::Sync => matches!(a.page, Page::Sync | Page::Conflict),
            x => a.page == x && a.new_item.is_none(),
        }
    };
    let icon = |g: G, on: bool, badge: Option<String>, m: Msg| -> E<'static, Msg> {
        let c = if on { p.accent } else { tn.muted };
        let b = container(glyph(g, c, z(24.0), 2.0)).center_x(Length::Fixed(z(42.0))).center_y(Length::Fixed(z(42.0))).style(bx(if on { Pal::a(p.accent, 0.14) } else { Color::TRANSPARENT }, 7.0, if on { Some((Pal::a(p.accent, 0.7), 1.0)) } else { None }));
        let mut s = stack![b];
        if let Some(n) = badge {
            s = s.push(container(container(tb(n, f(10.0), p.ink)).center_x(Length::Fixed(z(20.0))).center_y(Length::Fixed(z(20.0))).style(bx(p.accent, z(10.0), None))).align_right(Length::Fixed(z(46.0))).padding(pad(0.0, 0.0, 0.0, 0.0)));
        }
        click(container(s).padding(pad(z(6.0), 0.0, z(6.0), 0.0)), Some(m))
    };
    let sync_badge = {
        let n = if !a.sync.conflicts.is_empty() || a.conflict.is_some() { a.sync.conflicts.len().max(1) } else { a.sync.changes.len() };
        (n > 0 && a.sync.is_repo).then(|| n.to_string())
    };
    let col = column![
        gap(1.0, z(12.0)),
        icon(G::Doc, on_page(Page::Files), None, Msg::Rail(Page::Files)),
        icon(G::Search, on_page(Page::Search), None, Msg::Rail(Page::Search)),
        icon(G::Clock, on_page(Page::Today), None, Msg::Rail(Page::Today)),
        icon(G::Columns, on_page(Page::Board), None, Msg::Rail(Page::Board)),
        icon(G::Asterisk, on_page(Page::Graph), None, Msg::Rail(Page::Graph)),
        icon(G::Sync, on_page(Page::Sync), sync_badge, Msg::Rail(Page::Sync)),
        icon(G::Plus, a.new_item.is_some(), None, Msg::NewOpen),
        Space::new().height(Length::Fill),
        icon(G::Gear, on_page(Page::Settings), None, Msg::Rail(Page::Settings)),
        icon(G::Help, false, None, Msg::OpenRel("README.md".into())),
        // Edit layout, always findable (the owner, Nov 24): the page in front, edited in place
        icon(G::Pencil, a.describe.is_editing(), None, Msg::Describe(hearth_ui::arrange::edit_layout())),
        gap(1.0, z(12.0)),
    ]
    .align_x(iced::Center)
    .width(Length::Fixed(z(56.0)));
    row![container(col).height(Length::Fill).style(bx(tn.rail, 0.0, None)), ui::vline(tn.line)].into()
}

/// The vault's header: its icon, name, "GitHub · main".
fn vault_head(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let icon = container(glyph(G::Pencil, p.accent, z(20.0), 2.0)).center_x(Length::Fixed(z(40.0))).center_y(Length::Fixed(z(40.0))).style(bx(Pal::mix(tn.card2, p.accent, 0.10), 8.0, Some((Pal::a(p.accent, 0.35), 1.0))));
    let sub = if a.sync.is_repo {
        let host = if a.sync.repo.remote.contains('/') && !a.sync.repo.remote.contains(':') { "GitHub" } else { "Git" };
        format!("{host} · {}", if a.sync.repo.branch.is_empty() { "main" } else { &a.sync.repo.branch })
    } else {
        hearth_notes::paths::tilde(&a.root)
    };
    let r = row![icon, gap(z(14.0), 1.0), column![tb(a.vault_name(), f(19.0), tn.text), t1(sub, f(14.0), tn.muted)].spacing(1.0).width(Length::Fill), t("▾", f(14.0), tn.muted)].align_y(iced::Center);
    container(click(r, Some(Msg::ChangeVault))).padding(pad(z(16.0), z(18.0), z(10.0), z(18.0))).into()
}

pub fn sidebar(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let mut c = Column::new().push(vault_head(a));
    let body: E<Msg> = match a.page {
        Page::Search => crate::searchv::side(a),
        Page::Settings => crate::settingsv::side(a),
        _ => files(a),
    };
    c = c.push(container(body).height(Length::Fill));
    c = c.push(footer(a));
    row![container(c).width(Length::Fixed(z(324.0))).height(Length::Fill).style(bx(tn.side, 0.0, None)), ui::vline(tn.line)].into()
}

fn footer(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let (c, line) = a.sync_line();
    column![
        ui::hline(tn.line),
        container(column![
            row![look::dot(c, z(9.0)), gap(z(10.0), 1.0), t1(line, f(14.0), Pal::mix(tn.text, tn.muted, 0.25))].align_y(iced::Center),
            row![tb("This PC: ", f(14.0), Pal::mix(tn.text, tn.muted, 0.15)), t1(a.this_pc.clone(), f(14.0), Pal::mix(tn.text, tn.muted, 0.25))],
        ]
        .spacing(z(6.0)))
        .padding(pad(z(14.0), z(20.0), z(16.0), z(22.0)))
    ]
    .into()
}

// ------------------------------------------------------------------ the folder tree
/// The tree's number for a top folder: 00 Atlas, 10, 20 … for the projects that have notes, 90 log, 99 tools.
pub fn number(a: &App, top: &str) -> String {
    match top {
        "log" => "90".into(),
        "tools" => "99".into(),
        x => {
            let have: Vec<&str> = PROJECTS.iter().copied().filter(|pj| a.vault.dirs.iter().any(|d| d == pj)).collect();
            match have.iter().position(|pj| *pj == x) {
                Some(i) => format!("{}", (i + 1) * 10),
                None => "80".into(),
            }
        }
    }
}

/// How many: a project's or kind's items, else what's in the folder.
fn count(a: &App, dir: &str) -> usize {
    let depth = dir.split('/').count();
    if PROJECTS.contains(&dir.split('/').next().unwrap_or("")) && depth <= 2 {
        return a.vault.dirs.iter().filter(|d| d.starts_with(&format!("{dir}/")) && d.split('/').count() == 3).count();
    }
    let kids_d = a.vault.dirs.iter().filter(|d| d.starts_with(&format!("{dir}/")) && d.split('/').count() == depth + 1 && !a.vault.ignored.contains(d)).count();
    let kids_f = a.vault.notes.iter().filter(|n| n.rel.starts_with(&format!("{dir}/")) && n.rel.split('/').count() == depth + 1).count()
        + a.vault.files.iter().filter(|x| x.starts_with(&format!("{dir}/")) && x.split('/').count() == depth + 1).count();
    kids_d + kids_f
}

enum Row {
    Dir(String, String, usize),
    File(String, String, usize),
    Moc(String, String, usize),
    Atlas,
}

fn files(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let head = row![look::caps(&p, "Files"), Space::new().width(Length::Fill), click(t("＋", f(16.0), tn.muted), Some(Msg::NewOpen)), gap(z(16.0), 1.0), t("⇅", f(16.0), tn.muted), gap(z(16.0), 1.0), t("⋯", f(16.0), tn.muted)].align_y(iced::Center);
    let mut rows: Vec<Row> = vec![Row::Atlas];
    if a.tree_open.contains("atlas") {
        rows.push(Row::Moc("home".into(), "Home".into(), 1));
        for pj in PROJECTS {
            if a.vault.notes.iter().any(|n| n.project == pj) {
                rows.push(Row::Moc(pj.into(), format!("{pj} · map"), 1));
            }
        }
        rows.push(Row::Moc("logs".into(), "Daily logs · map".into(), 1));
    }
    let mut tops: Vec<String> = a.vault.dirs.iter().filter(|d| !d.contains('/') && !a.vault.ignored.contains(d)).cloned().collect();
    tops.sort_by_key(|d| (number(a, d), d.clone()));
    for d in tops {
        walk(a, &d, 0, &mut rows);
    }
    for n in a.vault.notes.iter().filter(|n| !n.rel.contains('/')) {
        rows.push(Row::File(n.rel.clone(), n.rel.clone(), 0));
    }
    let open_rel = a.doc.as_ref().map(|d| d.rel.clone()).unwrap_or_default();
    let mut c = Column::new().spacing(0.0);
    for r in rows {
        c = c.push(match r {
            Row::Atlas => {
                let open = a.tree_open.contains("atlas");
                trow(a, 0, open, true, Some("00"), "Atlas".into(), if open { String::new() } else { "4".into() }, None, false, Msg::Toggle("atlas".into()))
            }
            Row::Moc(w, label, depth) => {
                let extra = if w == "home" { Some(" · INDEX.md") } else { None };
                moc_row(a, depth, label, extra, open_rel == format!("atlas:{w}") || (w == "home" && open_rel == "INDEX.md"), Msg::OpenAtlas(w))
            }
            Row::Dir(path, name, depth) => {
                let open = a.tree_open.contains(&path);
                let num = if depth == 0 { Some(number(a, &path)) } else { None };
                let n = if open { String::new() } else { let c = count(a, &path); if c > 0 { c.to_string() } else { String::new() } };
                trow(a, depth, open, true, num.as_deref(), name, n, None, false, Msg::Toggle(path))
            }
            Row::File(rel, name, depth) => {
                let today = rel == a.today_rel();
                trow(a, depth, false, false, None, name, if today { "today".into() } else { String::new() }, None, rel == open_rel, Msg::OpenRel(rel))
            }
        });
    }
    let empty: Vec<&str> = PROJECTS.iter().copied().filter(|pj| !a.vault.notes.iter().any(|n| n.project == *pj)).collect();
    if !empty.is_empty() && a.vault.notes.len() > 0 {
        let s = format!("{} {} no notes yet.", join_and(&empty), if empty.len() == 1 { "has" } else { "have" });
        c = c.push(container(t(s, f(14.0), tn.faint)).padding(pad(z(18.0), z(16.0), 0.0, z(4.0))));
    }
    column![container(head).padding(pad(z(6.0), z(20.0), z(8.0), z(22.0))), scrollable(container(c).padding(pad(0.0, z(12.0), z(10.0), z(12.0)))).height(Length::Fill)].into()
}

pub fn join_and(v: &[&str]) -> String {
    match v.len() {
        0 => String::new(),
        1 => v[0].to_string(),
        n => format!("{} and {}", v[..n - 1].join(", "), v[n - 1]),
    }
}

fn walk(a: &App, dir: &str, depth: usize, rows: &mut Vec<Row>) {
    let name = dir.rsplit('/').next().unwrap_or(dir).to_string();
    rows.push(Row::Dir(dir.to_string(), name, depth));
    if !a.tree_open.contains(dir) {
        return;
    }
    let d = dir.split('/').count();
    let mut kids: Vec<&String> = a.vault.dirs.iter().filter(|x| x.starts_with(&format!("{dir}/")) && x.split('/').count() == d + 1 && !a.vault.ignored.contains(x)).collect();
    kids.sort();
    for k in kids {
        walk(a, k, depth + 1, rows);
    }
    let mut fs: Vec<String> = a.vault.notes.iter().filter(|n| n.rel.starts_with(&format!("{dir}/")) && n.rel.split('/').count() == d + 1).map(|n| n.rel.clone()).collect();
    fs.extend(a.vault.files.iter().filter(|x| x.starts_with(&format!("{dir}/")) && x.split('/').count() == d + 1).cloned());
    if dir == "log" {
        fs.sort_by(|x, y| y.cmp(x));
    } else {
        fs.sort_by_key(|f| (!f.ends_with("README.md"), f.clone()));
    }
    for f in fs {
        let name = f.rsplit('/').next().unwrap_or(&f).to_string();
        rows.push(Row::File(f.clone(), name, depth + 1));
    }
}

#[allow(clippy::too_many_arguments)]
fn trow<'a>(a: &'a App, depth: usize, open: bool, dir: bool, num: Option<&str>, name: String, right: String, _x: Option<()>, on: bool, m: Msg) -> E<'a, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let arrow: E<Msg> = if dir { t(if open { "▾" } else { "▸" }, f(11.0), tn.faint).into() } else { gap(z(10.0), 1.0) };
    let icon: E<Msg> = if dir { glyph(G::Folder, p.accent, z(18.0), 2.2) } else { glyph(G::Doc, tn.muted, z(17.0), 1.8) };
    let mut r = row![gap(z(20.0) * depth as f32 + z(6.0), 1.0), container(arrow).width(Length::Fixed(z(16.0))), gap(z(6.0), 1.0), icon, gap(z(10.0), 1.0)].align_y(iced::Center);
    if let Some(n) = num {
        r = r.push(t(n.to_string(), f(13.0), tn.faint)).push(gap(z(10.0), 1.0));
    }
    r = r.push(container(t1(name, f(16.0), if on { tn.text } else { Pal::mix(tn.text, tn.muted, 0.12) })).width(Length::Fill).clip(true));
    if !right.is_empty() {
        r = r.push(t1(right, f(13.0), tn.faint)).push(gap(z(6.0), 1.0));
    }
    let bg = if on { Pal::mix(tn.side, p.text, 0.07) } else { Color::TRANSPARENT };
    let line = container(Space::new()).width(Length::Fixed(2.0)).height(Length::Fixed(z(30.0))).style(bx(if on { p.accent } else { Color::TRANSPARENT }, 1.0, None));
    click(container(row![line, r].align_y(iced::Center)).height(Length::Fixed(z(30.0))).width(Length::Fill).style(bx(bg, 5.0, None)), Some(m))
}

fn moc_row<'a>(a: &'a App, depth: usize, name: String, extra: Option<&'static str>, on: bool, m: Msg) -> E<'a, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let mut r = row![gap(z(20.0) * depth as f32 + z(28.0), 1.0), t("◈", f(14.0), p.accent), gap(z(10.0), 1.0), t1(name, f(16.0), Pal::mix(tn.text, tn.muted, 0.12))].align_y(iced::Center);
    if let Some(x) = extra {
        r = r.push(t1(x, f(16.0), tn.faint));
    }
    let bg = if on { Pal::mix(tn.side, p.text, 0.07) } else { Color::TRANSPARENT };
    click(container(r).height(Length::Fixed(z(30.0))).width(Length::Fill).center_y(Length::Fixed(z(30.0))).style(bx(bg, 5.0, None)), Some(m))
}

// ------------------------------------------------------------------ first run
/// The vault isn't there yet: clone hearth-workspace with the PC's own Git sign-in (or pick another folder).
fn first_run(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let where_ = hearth_notes::paths::tilde(&a.root);
    let (head, body, buttons): (String, String, Vec<E<Msg>>) = match &a.vs {
        VaultState::Reading => ("Reading your notes…".into(), where_.clone(), vec![]),
        VaultState::Cloning => ("Getting hearth-workspace from GitHub…".into(), format!("Into {where_}, with this PC's Git sign-in."), vec![]),
        VaultState::CloneFailed(e) => (
            "It couldn't be cloned".into(),
            format!("{e}"),
            vec![look::btn(&p, "Try again", look::B::Primary, true, false, Some(Msg::CloneNow)), look::btn(&p, "Use another folder…", look::B::Normal, true, false, Some(Msg::ChangeVault))],
        ),
        _ => (
            "Your notes aren't on this PC yet".into(),
            format!("Hearth Notes opens a folder of Markdown notes. Clone Dragongamer9/hearth-workspace into {where_} with this PC's Git sign-in (GitHub CLI or Git's own), or open any other folder. Hearth Notes never asks for or keeps a password or token."),
            vec![look::btn(&p, "Clone hearth-workspace", look::B::Primary, true, false, Some(Msg::CloneNow)), look::btn(&p, "Use another folder…", look::B::Normal, true, false, Some(Msg::ChangeVault))],
        ),
    };
    let mut br = iced::widget::Row::new().spacing(z(14.0));
    for b in buttons {
        br = br.push(b);
    }
    let card = container(column![tm(head, f(26.0), tn.text), t(body, f(17.0), tn.muted).width(Length::Fixed(z(760.0))), br].spacing(z(18.0))).padding(z(40.0)).style(bx(tn.card, 12.0, Some((tn.line, 1.0))));
    container(card).center_x(Length::Fill).center_y(Length::Fill).into()
}

// ------------------------------------------------------------------ the hint strip
fn hints_for(a: &App) -> E<'_, Msg> {
    let p = a.p;
    use look::{hint, hint2, hint_words};
    if a.new_item.is_some() {
        return look::hints(&p, vec![hint(&p, "A", "Select"), hint(&p, "B", "Cancel"), hint(&p, "X", "Use suggested name")], vec![hint2(&p, "LB", "RB", "Previous / next step")]);
    }
    let (l, r): (Vec<E<Msg>>, Vec<E<Msg>>) = match a.page {
        Page::Files => (vec![hint(&p, "A", "Select"), hint(&p, "B", "Back"), hint(&p, "Y", "Search"), hint(&p, "X", "Today")], vec![hint2(&p, "LB", "RB", "Previous / next tab"), hint_words(&p, "Ctrl+O Quick switcher")]),
        Page::Today => (vec![hint(&p, "A", "Select"), hint(&p, "B", "Back"), hint(&p, "X", "Add"), hint(&p, "Y", "Search")], vec![hint2(&p, "LB", "RB", "Yesterday / tomorrow")]),
        Page::Board => (vec![hint(&p, "A", "Open its note"), hint(&p, "X", "Move card"), hint(&p, "Y", "Search"), hint(&p, "B", "Back")], vec![hint2(&p, "LB", "RB", "Previous / next column")]),
        Page::Search => (vec![hint(&p, "A", "Open"), hint(&p, "B", "Close"), hint(&p, "Y", "Search"), hint(&p, "X", "Filters")], vec![hint_words(&p, "Ctrl+Shift+F Search all notes"), hint_words(&p, "Ctrl+O Quick switcher")]),
        Page::Sync => (vec![hint(&p, "A", "Fix"), hint(&p, "B", "Back"), hint(&p, "X", "Sync now"), hint(&p, "Y", "Check again")], vec![hint2(&p, "LB", "RB", "Previous / next file")]),
        Page::Conflict => (vec![hint(&p, "A", "Choose"), hint(&p, "B", "Decide later"), hint(&p, "X", "Merge both")], vec![hint2(&p, "LB", "RB", "Previous / next change")]),
        Page::Settings => (vec![hint(&p, "A", "Change"), hint(&p, "B", "Back")], vec![hint2(&p, "LB", "RB", "Previous / next page")]),
        Page::Modes => (vec![hint(&p, "A", "Use this mode"), hint(&p, "Y", "Next mode"), hint(&p, "B", "Back")], vec![hint(&p, "LS", "Pick"), hint_words(&p, "Remembered for next time")]),
        Page::Graph => crate::gview::hints(a),
    };
    look::hints(&p, l, r)
}

/// Text that wraps (the default) at this size and colour.
pub fn para<'a>(s: impl text::IntoFragment<'a>, size: f32, c: Color) -> E<'a, Msg> {
    t(s, size, c).into()
}
