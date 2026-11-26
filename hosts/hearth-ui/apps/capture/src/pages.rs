//! Capture's window (Capture.qml's): the header with its pages, Take (what to capture, the wait, the pointer, Take
//! and Record, and Recent at a glance), Recent (the pictures saved lately), the editor's page (editor.rs), and the
//! two small windows: the countdown and the scrolling capture's pill. The arrow keys (the D-pad) move along the
//! controls, Enter / Space (A) presses, Esc (B) closes, as in Mantel.

use crate::btn::{b, label};
use crate::glyph::glyph;
use crate::helper;
use crate::{App, Msg, Page, Phase};
use hearth_ui::ui::{self, boxed, gap, TitleMsgs, G};
use hearth_ui::Pal;
use iced::keyboard::{key::Named, Key};
use iced::widget::{column, container, image, mouse_area, row, scrollable, stack, Column, Row, Space};
use iced::{Border, Color, ContentFit, Element, Length, Padding, Shadow, Task, Theme};

/// What the keyboard / D-pad is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fid {
    TabTake,
    TabEdit,
    TabRecent,
    Tile(usize),
    Delay(usize),
    ScrollAuto,
    Pointer,
    Take,
    Record,
    SeeAll,
    Thumb(usize),
}

pub const TAKE_BUTTON: Fid = Fid::Take;
pub const DELAYS: [i64; 4] = [0, 3, 5, 10];

/// The five things to capture: (id, glyph, name, what it is).
pub const TILES: [(&str, &str, &str, &str); 5] = [
    ("area", "area", "An area", "Drag a box on a still picture"),
    ("window", "window", "A window", "The window in front"),
    ("screen", "screen", "This screen", "The screen Capture is on"),
    ("all", "all", "All screens", "Everything, side by side"),
    ("scroll", "scroll", "Scrolling", "A long picture of a page that scrolls"),
];

fn chain(a: &App) -> Vec<Fid> {
    let mut v = vec![Fid::TabTake];
    if !a.shot.is_empty() {
        v.push(Fid::TabEdit);
    }
    v.push(Fid::TabRecent);
    if a.page == Page::Take {
        v.extend((0..TILES.len()).map(Fid::Tile));
        v.extend((0..DELAYS.len()).map(Fid::Delay));
        if a.mode() == "scroll" {
            v.push(Fid::ScrollAuto);
        }
        v.extend([Fid::Pointer, Fid::Take, Fid::Record]);
        if !a.recent.is_empty() {
            v.push(Fid::SeeAll);
        }
        v.extend((0..a.recent.len().min(6)).map(Fid::Thumb));
    }
    v
}

fn activate(a: &App, f: Fid) -> Option<Msg> {
    Some(match f {
        Fid::TabTake => Msg::Page(Page::Take),
        Fid::TabEdit => Msg::Page(Page::Edit),
        Fid::TabRecent => Msg::Page(Page::Recent),
        Fid::Tile(i) => Msg::SetMode(TILES.get(i)?.0),
        Fid::Delay(i) => Msg::SetDelay(*DELAYS.get(i)?),
        Fid::ScrollAuto => Msg::ToggleScrollAuto,
        Fid::Pointer => Msg::TogglePointer,
        Fid::Take => Msg::Take,
        Fid::Record => Msg::Record,
        Fid::SeeAll => Msg::Page(Page::Recent),
        Fid::Thumb(i) => Msg::RecentEdit(a.recent.get(i)?.path.clone()),
    })
}

pub fn take_key(a: &mut App, key: &Key) -> Task<Msg> {
    let ch = chain(a);
    let at = ch.iter().position(|f| *f == a.focus).unwrap_or(0);
    match key.as_ref() {
        Key::Named(Named::ArrowRight) | Key::Named(Named::ArrowDown) | Key::Named(Named::Tab) => a.focus = ch[(at + 1) % ch.len()],
        Key::Named(Named::ArrowLeft) | Key::Named(Named::ArrowUp) => a.focus = ch[(at + ch.len() - 1) % ch.len()],
        Key::Named(Named::Enter) | Key::Named(Named::Space) => {
            if let Some(m) = activate(a, a.focus) {
                return Task::done(m);
            }
        }
        Key::Named(Named::Escape) => return Task::done(Msg::Close),
        _ => {}
    }
    Task::none()
}

fn recent_cols(a: &App) -> usize {
    (((a.main_size.width - 44.0) / 260.0).floor() as usize).max(1)
}

pub fn recent_key(a: &mut App, key: &Key) -> Task<Msg> {
    let n = a.recent.len();
    let cols = recent_cols(a);
    let cur = a.recent_cur.unwrap_or(0);
    match key.as_ref() {
        Key::Named(Named::Escape) => {
            a.page = Page::Take;
            return Task::none();
        }
        _ if n == 0 => return Task::none(),
        Key::Named(Named::ArrowRight) => a.recent_cur = Some((cur + 1).min(n - 1)),
        Key::Named(Named::ArrowLeft) => a.recent_cur = Some(cur.saturating_sub(1)),
        Key::Named(Named::ArrowDown) => a.recent_cur = Some((cur + cols).min(n - 1)),
        Key::Named(Named::ArrowUp) => a.recent_cur = Some(cur.saturating_sub(cols)),
        Key::Named(Named::Enter) => return Task::done(Msg::RecentEdit(a.recent[cur.min(n - 1)].path.clone())),
        Key::Named(Named::Delete) => return Task::done(Msg::RecentForget(a.recent[cur.min(n - 1)].path.clone())),
        Key::Named(Named::ContextMenu) => return Task::done(Msg::RecentFolder(a.recent[cur.min(n - 1)].path.clone())),
        _ => {}
    }
    Task::none()
}

// ------------------------------------------------------------------ drag and drop (board 3: the same in every app)
use hearth_ui::studio::Kit;
/// Capture's blocks window: its buttons (take, record, what to capture, the wait, its pages), its panes, its info.
pub const KIT: Kit = Kit {
    name: "Capture",
    buttons: &[
        hearth_ui::studio::tile("take", "Take a screenshot", G::Camera),
        hearth_ui::studio::tile("record", "Record", G::Play),
        hearth_ui::studio::tile("area", "An area", G::Grid),
        hearth_ui::studio::tile("window", "A window", G::Display),
        hearth_ui::studio::tile("screen", "This screen", G::Display),
        hearth_ui::studio::tile("all", "All screens", G::Columns),
        hearth_ui::studio::tile("scroll", "Scrolling", G::Down),
        hearth_ui::studio::tile("no-wait", "No wait", G::Clock),
        hearth_ui::studio::tile("wait-3", "Wait 3 s", G::Clock),
        hearth_ui::studio::tile("wait-5", "Wait 5 s", G::Clock),
        hearth_ui::studio::tile("wait-10", "Wait 10 s", G::Clock),
        hearth_ui::studio::tile("pointer", "The pointer in or out", G::Mouse),
        hearth_ui::studio::tile("take-page", "Take (the page)", G::Camera),
        hearth_ui::studio::tile("edit-page", "Edit (the page)", G::Pencil),
        hearth_ui::studio::tile("recent-page", "Recent (the page)", G::Clock),
    ],
    panes: &[hearth_ui::studio::tile("recent", "Recent pictures", G::Grid), hearth_ui::studio::tile("last", "The last picture", G::Camera), hearth_ui::studio::tile("keys", "Shortcuts", G::Keyboard)],
    info: &[
        hearth_ui::studio::tile("count", "Pictures kept", G::Grid),
        hearth_ui::studio::tile("last", "The last one", G::Camera),
        hearth_ui::studio::tile("mode", "What it captures", G::Display),
        hearth_ui::studio::tile("delay", "The wait", G::Clock),
    ],
    pane_page: "window",
    bars: &[],
    menus: &[],
};

/// One of Capture's buttons dragged in, pressed.
pub fn block_action(id: &str) -> Option<Msg> {
    Some(match id {
        "take" => Msg::Take,
        "record" => Msg::Record,
        "area" => Msg::SetMode("area"),
        "window" => Msg::SetMode("window"),
        "screen" => Msg::SetMode("screen"),
        "all" => Msg::SetMode("all"),
        "scroll" => Msg::SetMode("scroll"),
        "no-wait" => Msg::SetDelay(0),
        "wait-3" => Msg::SetDelay(3),
        "wait-5" => Msg::SetDelay(5),
        "wait-10" => Msg::SetDelay(10),
        "pointer" => Msg::TogglePointer,
        "take-page" => Msg::Page(Page::Take),
        "edit-page" => Msg::Page(Page::Edit),
        "recent-page" => Msg::Page(Page::Recent),
        _ => return None,
    })
}

fn studio_values(a: &App) {
    let last = a.recent.first().map(|it| it.path.rsplit('/').next().unwrap_or("").to_string()).unwrap_or_default();
    let mode = TILES.iter().find(|x| x.0 == a.mode()).map(|x| x.2).unwrap_or("").to_string();
    a.describe.values(vec![("count", a.recent.len().to_string()), ("last", last), ("mode", mode), ("delay", if a.delay() == 0 { "No wait".into() } else { format!("{} s", a.delay()) })]);
}

/// Recent pictures (a pane of its own): the last four, each a press away from Edit.
fn recent_pane(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    if a.recent.is_empty() {
        return label("Screenshots you save show up here.", 13.0, p.muted, false).into();
    }
    let mut c = Column::new().spacing(10.0);
    for (i, it) in a.recent.iter().take(4).enumerate() {
        c = c.push(container(thumb(a, it, i, false, false)).height(Length::Fixed(110.0)));
    }
    scrollable(c).height(Length::Fill).into()
}

/// The last picture (a pane of its own), as big as the pane.
fn last_pane(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    match a.recent.first().and_then(|it| a.thumbs.get(&it.path).map(|h| (it, h))) {
        Some((it, h)) => mouse_area(image(h.clone()).width(Length::Fill).height(Length::Fill).content_fit(ContentFit::Contain)).on_press(Msg::RecentEdit(it.path.clone())).into(),
        None => label("The last screenshot shows here.", 13.0, p.muted, false).into(),
    }
}

/// Shortcuts (a pane of its own).
fn keys_pane(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    let mut c = Column::new().spacing(6.0);
    for (k, what) in [("Print Screen", "the whole screen, saved"), ("Super+Shift+S", "pick an area"), ("Ctrl+Z", "undo (Edit)"), ("Ctrl+C", "copy (Edit)"), ("Ctrl+S", "save (Edit)")] {
        c = c.push(row![label(k, 13.0, p.text, true), Space::new().width(Length::Fill), label(what, 13.0, p.muted, false)].spacing(10.0));
    }
    c.into()
}

// ------------------------------------------------------------------ the window
pub fn main_view(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    let title = hearth_ui::describe::title_bar(&p, &a.describe, G::Camera, "Capture", None, TitleMsgs { drag: Msg::Drag, maximize: Msg::Maximize, minimize: Some(Msg::Minimize), close: Msg::Close }, p.bg2, Msg::Describe(hearth_ui::describe::DMsg::Open));
    let body: Element<'_, Msg> = match a.page {
        Page::Take => take_page(a),
        Page::Edit => edit_page(a),
        Page::Recent => recent_page(a),
    };
    // its blocks (hearth_ui::blocks "window"): the header with the pages, and the page under it
    studio_values(a);
    let mut items: Vec<(&'static str, Element<'_, Msg>)> = vec![("header", hearth_ui::studio::made(&a.describe, "window", "header", || header(a))), ("page", container(body).width(Length::Fill).height(Length::Fill).into())];
    if a.describe.wants_pane("recent") {
        items.push(("pane:recent", recent_pane(a)));
    }
    if a.describe.wants_pane("last") {
        items.push(("pane:last", last_pane(a)));
    }
    if a.describe.wants_pane("keys") {
        items.push(("pane:keys", keys_pane(a)));
    }
    let page = hearth_ui::arrange::parts(
        &a.describe,
        &p,
        "window",
        items,
        |ps| column![ps.take("header"), ps.take("page")].into(),
        Msg::Describe,
    );
    let inner = column![title, ui::hline(p.line), page];
    let framed = container(inner).width(Length::Fill).height(Length::Fill).style(boxed(p.bg, 0.0, Some((p.line, 1.0))));
    ui::edges(framed.into(), Msg::Resize)
}

/// The header: Capture, its pages, and what the controller does.
fn header(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    let mark = container(glyph("cam", p.accent, 20.0)).center_x(Length::Fixed(36.0)).center_y(Length::Fixed(36.0)).style(boxed(Pal::a(p.accent, 0.18), 10.0, None));
    let mut r = row![mark, label("Capture", 22.0, p.text, true), gap(18.0, 1.0)].spacing(12.0).align_y(iced::Center);
    r = r.push(b("Take").selected(a.page == Page::Take).focus(a.focus == Fid::TabTake && a.page != Page::Edit).view(&p, Msg::Page(Page::Take)));
    if !a.shot.is_empty() {
        r = r.push(b("Edit").selected(a.page == Page::Edit).focus(a.focus == Fid::TabEdit && a.page != Page::Edit).view(&p, Msg::Page(Page::Edit)));
    }
    let rt = if a.recent.is_empty() { "Recent".to_string() } else { format!("Recent  {}", a.recent.len()) };
    r = r.push(b(rt).selected(a.page == Page::Recent).focus(a.focus == Fid::TabRecent && a.page != Page::Edit).view(&p, Msg::Page(Page::Recent)));
    r = r.push(Space::new().width(Length::Fill));
    let hint = if a.page == Page::Edit { "Ctrl+Z undo  ·  Ctrl+C copy  ·  Ctrl+S save" } else { "A choose  ·  B close" };
    r = r.push(label(hint, 13.0, p.muted, false));
    // Edit layout, always findable (the owner, Nov 24): the page in front, edited in place
    r = r.push(gap(6.0, 1.0)).push(hearth_ui::arrange::edit_button(&p, Msg::Describe(hearth_ui::arrange::edit_layout())));
    let bar = container(r).padding(Padding { top: 0.0, right: 22.0, bottom: 0.0, left: 22.0 }).center_y(Length::Fixed(63.0)).width(Length::Fill).style(boxed(p.bg2, 0.0, None));
    column![bar, ui::hline(p.line)].into()
}

fn tile<'a>(a: &'a App, i: usize) -> Element<'a, Msg> {
    let p = a.p;
    let (id, g, name, what) = TILES[i];
    let chosen = a.mode() == id;
    let focused = a.focus == Fid::Tile(i);
    let hovered = a.hover.as_deref() == Some(id);
    let bg = if chosen { Pal::a(p.accent, 0.16) } else if hovered { p.panel3 } else { p.panel2 };
    let r = p.r(14.0).max(8.0);
    let inner = row![
        glyph(g, if chosen { p.accent } else { p.text }, 34.0),
        column![label(name, 16.0, p.text, true), label(what, 12.0, p.muted, false)].spacing(3.0)
    ]
    .spacing(14.0)
    .align_y(iced::Center);
    let c = container(inner)
        .padding(Padding { top: 0.0, right: 12.0, bottom: 0.0, left: 16.0 })
        .center_y(Length::Fixed(78.0))
        .width(Length::Fill)
        .style(move |_: &Theme| container::Style {
            background: Some(iced::Background::Color(bg)),
            border: Border { color: if chosen || focused { p.accent } else { p.line }, width: if chosen { 2.0 } else if focused { 3.0 } else { 1.0 }, radius: r.into() },
            shadow: if focused { Shadow { color: Pal::a(p.accent, 0.5), offset: iced::Vector::new(0.0, 0.0), blur_radius: 8.0 } } else { Shadow::default() },
            ..container::Style::default()
        });
    mouse_area(c)
        .on_press(Msg::SetMode(id))
        .on_double_click(Msg::TakeMode(id))
        .on_enter(Msg::Hover(Some(id.to_string())))
        .on_exit(Msg::Hover(None))
        .interaction(iced::mouse::Interaction::Pointer)
        .into()
}

fn take_page(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    let mut kinds = Column::new().spacing(14.0);
    kinds = kinds.push(label("What to capture", 17.0, p.text, true));
    let mut grid = Column::new().spacing(10.0).width(Length::Fill);
    for pair in [0usize, 2, 4] {
        let mut r = Row::new().spacing(12.0).width(Length::Fill);
        r = r.push(container(tile(a, pair)).width(Length::FillPortion(1)));
        if pair + 1 < TILES.len() {
            r = r.push(container(tile(a, pair + 1)).width(Length::FillPortion(1)));
        } else {
            r = r.push(Space::new().width(Length::FillPortion(1)));
        }
        grid = grid.push(r);
    }
    kinds = kinds.push(grid);
    let mut wait = Column::new().spacing(14.0);
    wait = wait.push(column![gap(1.0, 6.0), label("Wait before taking it", 15.0, p.text, true)]);
    let mut delays = Row::new().spacing(8.0);
    for (i, d) in DELAYS.iter().enumerate() {
        let t = if *d == 0 { "No wait".to_string() } else { format!("{d} s") };
        let mut bt = b(t).selected(a.delay() == *d).focus(a.focus == Fid::Delay(i));
        if *d != 0 {
            bt = bt.glyph("timer");
        }
        delays = delays.push(bt.view(&p, Msg::SetDelay(*d)));
    }
    wait = wait.push(delays);
    if a.mode() == "scroll" {
        let t = if a.scroll_auto() { "Hearth scrolls it for me" } else { "I'll scroll it myself" };
        wait = wait.push(b(t).glyph("scroll").selected(a.scroll_auto()).focus(a.focus == Fid::ScrollAuto).view(&p, Msg::ToggleScrollAuto));
    }
    let pt = if a.pointer() { "The pointer is in the picture" } else { "Leave the pointer out" };
    let pointer = column![gap(1.0, 4.0), b(pt).glyph("pointer").selected(a.pointer()).focus(a.focus == Fid::Pointer).view(&p, Msg::TogglePointer)];
    let mut take = Column::new().spacing(14.0);
    if !a.error.is_empty() {
        take = take.push(label(a.error.clone(), 13.0, p.danger, false));
    }
    let take_t = if a.delay() > 0 { format!("Take in {} s", a.delay()) } else { "Take a screenshot".to_string() };
    take = take.push(
        row![
            b(take_t).glyph(a.mode()).big().primary().focus(a.focus == Fid::Take).view(&p, Msg::Take),
            b("Record").glyph("record").big().dim(a.has_recorder()).focus(a.focus == Fid::Record).view(&p, Msg::Record)
        ]
        .spacing(12.0),
    );

    // Recent, at a glance.
    let mut right = Column::new().spacing(12.0).width(Length::Fill).height(Length::Fill);
    let mut head = row![container(label("Recent", 17.0, p.text, true)).width(Length::Fill)].align_y(iced::Center);
    if !a.recent.is_empty() {
        head = head.push(b("See all").focus(a.focus == Fid::SeeAll).view(&p, Msg::Page(Page::Recent)));
    }
    right = right.push(head);
    let few: Vec<_> = a.recent.iter().take(6).collect();
    for (ri, pair) in few.chunks(2).enumerate() {
        let mut r = Row::new().spacing(10.0).width(Length::Fill);
        for (ci, it) in pair.iter().enumerate() {
            let i = ri * 2 + ci;
            r = r.push(container(thumb(a, it, i, false, a.focus == Fid::Thumb(i))).width(Length::FillPortion(1)).height(Length::Fixed(118.0)));
        }
        if pair.len() == 1 {
            r = r.push(Space::new().width(Length::FillPortion(1)));
        }
        right = right.push(r);
    }
    if a.recent.is_empty() {
        right = right.push(label("Screenshots you save show up here. They're kept in Pictures › Screenshots.", 13.0, p.muted, false));
    }
    let tip = label("Print Screen saves the whole screen right away · Super+Shift+S picks an area", 12.0, p.muted, false);
    // its blocks (hearth_ui::blocks): what to capture, the wait, the pointer, Take and Record, and Recent with its tip,
    // moved, sized, hidden or restyled; as it was when nothing is
    hearth_ui::arrange::parts(
        &a.describe,
        &p,
        "take",
        vec![("kinds", kinds.into()), ("wait", wait.into()), ("pointer", pointer.into()), ("take", take.into()), ("recent", right.into()), ("tip", tip.into())],
        |ps| {
            let left = column![ps.take("kinds"), ps.take("wait"), ps.take("pointer"), Space::new().height(Length::Fill), ps.take("take")].spacing(14.0).width(Length::Fixed(560.0)).height(Length::Fill);
            let right = column![ps.take("recent"), Space::new().height(Length::Fill), ps.take("tip")].spacing(12.0).width(Length::Fill).height(Length::Fill);
            let rpanel = container(right).padding(18.0).width(Length::Fill).height(Length::Fill).style(boxed(p.panel, p.r(16.0).max(8.0), Some((p.line, 1.0))));
            container(row![left, rpanel].spacing(26.0)).padding(26.0).width(Length::Fill).height(Length::Fill).into()
        },
        Msg::Describe,
    )
}

/// The picture being edited: its tools along the top, the picture, and the side with what you can do with it.
fn edit_page(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    hearth_ui::arrange::parts(
        &a.describe,
        &p,
        "edit",
        vec![
            ("toolbar", crate::editor::toolbar(&a.ed, &p)),
            ("stage", crate::editor::stage(&a.ed, &p)),
            ("side", crate::editor::side(&a.ed, &p, &a.tools, &a.scroll_note, &a.shot)),
        ],
        |ps| column![ps.take("toolbar"), row![ps.take("stage"), ps.take("side")].height(Length::Fill)].into(),
        Msg::Describe,
    )
}

/// Capture's blocks (hearth_ui::blocks): the window, and each of its pages.
use hearth_ui::blocks::{bl, PageDef};
pub const BLOCKS: &[PageDef] = &[
    PageDef { id: "window", title: "The window", blocks: &[bl("header", "Capture and its pages", 0, 0, 12, 1), bl("page", "The page", 0, 1, 12, 11)] },
    PageDef {
        id: "take",
        title: "Take",
        blocks: &[
            bl("kinds", "What to capture", 0, 0, 6, 5),
            bl("wait", "Wait before taking it", 0, 5, 6, 2),
            bl("pointer", "The pointer", 0, 7, 6, 1),
            bl("take", "Take and Record", 0, 10, 6, 2),
            bl("recent", "Recent", 6, 0, 6, 10),
            bl("tip", "Print Screen tip", 6, 10, 6, 2),
        ],
    },
    PageDef { id: "edit", title: "Edit", blocks: &[bl("toolbar", "The tools", 0, 0, 12, 1), bl("stage", "The picture", 0, 1, 9, 11), bl("side", "Copy, save and the rest", 9, 1, 3, 11)] },
    PageDef { id: "recent", title: "Recent", blocks: &[bl("grid", "The pictures", 0, 0, 12, 11), bl("hint", "What the buttons do", 0, 11, 12, 1)] },
];

/// A recent picture: its thumbnail, its name, and (big) when, its size and what you can do with it.
fn thumb<'a>(a: &'a App, it: &'a crate::Item, i: usize, big: bool, current: bool) -> Element<'a, Msg> {
    let p = a.p;
    let pic: Element<'a, Msg> = match a.thumbs.get(&it.path) {
        Some(h) => image(h.clone()).content_fit(ContentFit::Contain).width(Length::Fill).height(Length::Fill).into(),
        None => Space::new().width(Length::Fill).height(Length::Fill).into(),
    };
    let name = it.path.rsplit('/').next().unwrap_or("").to_string();
    let mut info = column![label(elide_middle(&name, if big { 34 } else { 26 }), 12.0, p.text, false).font(ui::weight(iced::font::Weight::Medium))].spacing(2.0);
    if big {
        let mut w = crate::draw::when(it.time, hearth_ui::time::now());
        if it.w > 0 {
            w.push_str(&format!("  ·  {} × {}", it.w, it.h));
        }
        info = info.push(label(w, 11.0, p.muted, false));
    }
    let r = p.r(12.0).max(6.0);
    let body = column![container(pic).padding(6.0).width(Length::Fill).height(Length::Fill), container(info).padding(Padding { top: 0.0, right: 8.0, bottom: 8.0, left: 8.0 })];
    let card = container(body).width(Length::Fill).height(Length::Fill).clip(true).style(move |_: &Theme| container::Style {
        background: Some(iced::Background::Color(p.panel2)),
        border: Border { color: if current { p.accent } else { p.line }, width: if current { 3.0 } else { 1.0 }, radius: r.into() },
        ..container::Style::default()
    });
    let key = format!("thumb:{}", it.path);
    let hovered = a.hover.as_deref() == Some(key.as_str());
    let mut layers = stack![mouse_area(card).on_press(Msg::RecentEdit(it.path.clone())).on_enter(Msg::Hover(Some(key.clone()))).on_exit(Msg::Hover(None)).interaction(iced::mouse::Interaction::Pointer)];
    if big && (hovered || current) {
        let path = it.path.clone();
        let acts = row![
            b("").glyph("copy").view(&p, Msg::RecentCopy(path.clone())),
            b("").glyph("folder").view(&p, Msg::RecentFolder(path.clone())),
            b("").glyph("close").view(&p, Msg::RecentForget(path))
        ]
        .spacing(6.0);
        layers = layers.push(container(acts).padding(10.0).align_right(Length::Fill));
    }
    let _ = i;
    layers.into()
}

fn elide_middle(s: &str, n: usize) -> String {
    let c: Vec<char> = s.chars().collect();
    if c.len() <= n {
        return s.to_string();
    }
    let k = n.saturating_sub(1) / 2;
    let head: String = c[..k].iter().collect();
    let tail: String = c[c.len() - (n.saturating_sub(1) - k)..].iter().collect();
    format!("{head}…{tail}")
}

fn recent_page(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    if a.recent.is_empty() {
        return container(label("Nothing saved yet. Screenshots you save are kept in Pictures › Screenshots.", 15.0, p.muted, false)).center_x(Length::Fill).center_y(Length::Fill).into();
    }
    let cols = recent_cols(a);
    let mut grid = Column::new().width(Length::Fill);
    for (ri, chunk) in a.recent.chunks(cols).enumerate() {
        let mut r = Row::new().width(Length::Fill);
        for (ci, it) in chunk.iter().enumerate() {
            let i = ri * cols + ci;
            r = r.push(container(thumb(a, it, i, true, a.recent_cur == Some(i))).padding(7.0).width(Length::FillPortion(1)).height(Length::Fixed(210.0)));
        }
        for _ in chunk.len()..cols {
            r = r.push(Space::new().width(Length::FillPortion(1)));
        }
        grid = grid.push(r);
    }
    let hint = container(label("A edit  ·  X show in folder  ·  Delete take off the list", 12.0, p.muted, false)).padding(12.0).align_right(Length::Fill);
    hearth_ui::arrange::parts(
        &a.describe,
        &p,
        "recent",
        vec![("grid", scrollable(container(grid).padding(15.0)).height(Length::Fill).into()), ("hint", hint.into())],
        |ps| column![ps.take("grid"), ps.take("hint")].into(),
        Msg::Describe,
    )
}

// ------------------------------------------------------------------ the small windows
pub fn scroll_pill_width(a: &App) -> f32 {
    let t = if a.scroll_auto() { "Scrolling and joining the pictures…" } else { "Scroll it slowly: Hearth joins the pictures" };
    (t.chars().count() as f32 * 8.3 + 22.0 + 4.0 * 12.0 + 80.0 + 96.0 + 36.0).round()
}

pub fn pill_view(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    let bg = Pal::a(p.panel, 0.96);
    let inner: Element<'_, Msg> = if a.phase == Phase::Scroll {
        let title = if a.scroll_auto() { "Scrolling and joining the pictures…" } else { "Scroll it slowly: Hearth joins the pictures" };
        let frames = helper::n(&a.scroll_info, "frames").max(1);
        let tall = {
            let h = helper::n(&a.scroll_info, "height");
            if h > 0 {
                h
            } else {
                a.scroll_geo.map(|g| g.h as i64).unwrap_or(0)
            }
        };
        let sub = format!("{frames} {}  ·  {tall} pixels tall", if frames == 1 { "picture" } else { "pictures" });
        row![
            glyph("scroll", p.accent, 22.0),
            column![label(title, 15.0, p.text, true), label(sub, 12.0, p.muted, false)].spacing(1.0),
            b("Cancel").view(&p, Msg::ScrollStop(true)),
            b("Done").glyph("scroll").primary().view(&p, Msg::ScrollStop(false))
        ]
        .spacing(12.0)
        .align_y(iced::Center)
        .into()
    } else {
        mouse_area(row![glyph("timer", p.accent, 22.0), label(format!("Taking it in {}", a.countdown.max(1)), 18.0, p.text, true)].spacing(12.0).align_y(iced::Center))
            .on_press(Msg::CancelTake)
            .into()
    };
    let r = if a.phase == Phase::Scroll { 28.0 } else { 32.0 };
    container(inner)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(move |_: &Theme| container::Style {
            background: Some(iced::Background::Color(Color { a: 1.0, ..bg })),
            border: Border { color: p.line, width: 1.0, radius: r.into() },
            ..container::Style::default()
        })
        .into()
}
