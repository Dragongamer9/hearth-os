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

// ------------------------------------------------------------------ the window
pub fn main_view(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    let title = ui::title_bar(&p, G::Camera, "Capture", None, TitleMsgs { drag: Msg::Drag, maximize: Msg::Maximize, minimize: Some(Msg::Minimize), close: Msg::Close }, p.bg2);
    let body: Element<'_, Msg> = match a.page {
        Page::Take => take_page(a),
        Page::Edit => crate::editor::view(&a.ed, &p, &a.tools, &a.scroll_note, &a.shot),
        Page::Recent => recent_page(a),
    };
    let inner = column![title, ui::hline(p.line), header(a), container(body).width(Length::Fill).height(Length::Fill)];
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
    let mut left = Column::new().spacing(14.0).width(Length::Fixed(560.0)).height(Length::Fill);
    left = left.push(label("What to capture", 17.0, p.text, true));
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
    left = left.push(grid);
    left = left.push(column![gap(1.0, 6.0), label("Wait before taking it", 15.0, p.text, true)]);
    let mut delays = Row::new().spacing(8.0);
    for (i, d) in DELAYS.iter().enumerate() {
        let t = if *d == 0 { "No wait".to_string() } else { format!("{d} s") };
        let mut bt = b(t).selected(a.delay() == *d).focus(a.focus == Fid::Delay(i));
        if *d != 0 {
            bt = bt.glyph("timer");
        }
        delays = delays.push(bt.view(&p, Msg::SetDelay(*d)));
    }
    left = left.push(delays);
    if a.mode() == "scroll" {
        let t = if a.scroll_auto() { "Hearth scrolls it for me" } else { "I'll scroll it myself" };
        left = left.push(b(t).glyph("scroll").selected(a.scroll_auto()).focus(a.focus == Fid::ScrollAuto).view(&p, Msg::ToggleScrollAuto));
    }
    let pt = if a.pointer() { "The pointer is in the picture" } else { "Leave the pointer out" };
    left = left.push(column![gap(1.0, 4.0), b(pt).glyph("pointer").selected(a.pointer()).focus(a.focus == Fid::Pointer).view(&p, Msg::TogglePointer)]);
    left = left.push(Space::new().height(Length::Fill));
    if !a.error.is_empty() {
        left = left.push(label(a.error.clone(), 13.0, p.danger, false));
    }
    let take_t = if a.delay() > 0 { format!("Take in {} s", a.delay()) } else { "Take a screenshot".to_string() };
    left = left.push(
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
    right = right.push(Space::new().height(Length::Fill));
    right = right.push(label("Print Screen saves the whole screen right away · Super+Shift+S picks an area", 12.0, p.muted, false));
    let rpanel = container(right).padding(18.0).width(Length::Fill).height(Length::Fill).style(boxed(p.panel, p.r(16.0).max(8.0), Some((p.line, 1.0))));
    container(row![left, rpanel].spacing(26.0)).padding(26.0).width(Length::Fill).height(Length::Fill).into()
}

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
    column![scrollable(container(grid).padding(15.0)).height(Length::Fill), hint].into()
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
