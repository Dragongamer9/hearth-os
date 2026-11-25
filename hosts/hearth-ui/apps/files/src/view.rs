//! Hearth Files' window, drawn the way the approved mockups show it (docs/mockups/next-update/files-1 … -8 and
//! cdrive-1, -3): tabs on top like Windows 11, the navigation row (back, forward, up, refresh, the address with its
//! steps, search), the command bar (New, cut, copy, paste, rename, share, delete, Sort, View, Details, ···), the left
//! menu, the folder (Details, List, Tiles and the icon sizes), the Home page, This PC, Hearth (C:)'s pages, the details
//! pane and the status bar; and the right-click menu, View, Sort, Properties and the other dialogs.
//! Long folders are drawn a screenful at a time (only the rows you can see are made).

use crate::app::*;
use crate::drive;
use crate::icons::{self, ico, Ic};
use crate::model::{self, base, Entry};
use hearth_ui::ui::{self, btn, gap, hline, t, t1, tb, tm, ts, vline, Kind};
use hearth_ui::Pal;
use iced::widget::{button, column, container, mouse_area, opaque, pin, responsive, row, scrollable, stack, text, text_input, Column, Row, Space};
use iced::{mouse, Background, Border, Color, ContentFit, Element, Length, Padding, Shadow, Size, Theme};
use serde_json::Value;

type El<'a> = Element<'a, Msg>;

fn img(h: &iced::widget::image::Handle) -> iced::widget::Image<iced::widget::image::Handle> {
    iced::widget::Image::<iced::widget::image::Handle>::new(h.clone())
}

fn pad(t: f32, r: f32, b: f32, l: f32) -> Padding {
    Padding { top: t, right: r, bottom: b, left: l }
}

fn sv(v: &Value, k: &str) -> String {
    v.get(k).and_then(Value::as_str).unwrap_or("").to_string()
}

/// The field colour (address bar, search, text boxes): a little darker than the window.
fn field_bg(p: &Pal) -> Color {
    if p.dark { Pal::mix(p.bg, Color::BLACK, 0.28) } else { Pal::mix(p.bg, Color::WHITE, 0.6) }
}
fn row_bg(p: &Pal) -> Color {
    Pal::mix(p.bg, p.panel, 0.55)
}

/// A flat button: transparent, a hover tint, the controller's ring when it's on it.
fn flat<'a>(p: &Pal, inner: El<'a>, msg: Option<Msg>, padding: Padding) -> El<'a> {
    let p = *p;
    button(inner)
        .padding(padding)
        .on_press_maybe(msg)
        .style(move |_: &Theme, st: button::Status| button::Style {
            background: Some(Background::Color(match st {
                button::Status::Hovered | button::Status::Pressed => p.hover(),
                _ => Color::TRANSPARENT,
            })),
            text_color: p.text,
            border: Border { radius: p.r(6.0).into(), ..Border::default() },
            shadow: Shadow::default(),
            snap: true,
        })
        .into()
}

fn input_style(p: Pal, focus_ring: bool) -> impl Fn(&Theme, text_input::Status) -> text_input::Style {
    move |_t, st| {
        let focused = matches!(st, text_input::Status::Focused { .. });
        text_input::Style {
            background: Background::Color(field_bg(&p)),
            border: Border { color: if focused && focus_ring { p.accent } else { p.line }, width: if focused && focus_ring { 1.5 } else { 1.0 }, radius: p.r(6.0).into() },
            icon: p.muted,
            placeholder: p.muted,
            value: p.text,
            selection: Pal::a(p.accent, 0.35),
        }
    }
}

fn heading<'a>(p: &Pal, s: &str) -> El<'a> {
    container(text(s.to_uppercase()).size(11.5).color(p.muted).font(ui::weight(iced::font::Weight::Medium))).padding(pad(14.0, 0.0, 8.0, 4.0)).into()
}

// ------------------------------------------------------------------ the window
/// Hearth Files' blocks (hearth_ui::blocks), and the file chooser's.
const FILE_BLOCKS: &[hearth_ui::blocks::BlockDef] = &[
    hearth_ui::blocks::bl("sidebar", "The navigation pane", 0, 0, 2, 12),
    hearth_ui::blocks::bl("folder", "The folder", 2, 0, 7, 12),
    hearth_ui::blocks::bl("preview", "The details pane", 9, 0, 3, 12),
];
/// The window's bars round the panes (the owner, Nov 24: every part a block): the title bar and tabs, the address bar,
/// the toolbar, the panes (the "window" page's blocks inside), the status bar.
const BAR_BLOCKS: &[hearth_ui::blocks::BlockDef] = &[
    hearth_ui::blocks::bl("tabs", "The title bar and tabs", 0, 0, 12, 2),
    hearth_ui::blocks::bl("address", "The address bar", 0, 2, 12, 2),
    hearth_ui::blocks::bl("toolbar", "The toolbar", 0, 4, 12, 2),
    hearth_ui::blocks::bl("panes", "The panes (navigation, folder, details)", 0, 6, 12, 5),
    hearth_ui::blocks::bl("status", "The status bar", 0, 11, 12, 1),
];
pub const BLOCKS: &[hearth_ui::blocks::PageDef] = &[
    hearth_ui::blocks::PageDef { id: "window", title: "The panes", blocks: FILE_BLOCKS },
    hearth_ui::blocks::PageDef { id: "chooser", title: "The file chooser", blocks: FILE_BLOCKS },
    hearth_ui::blocks::PageDef { id: "bars", title: "Hearth Files", blocks: BAR_BLOCKS },
];

/// A bar a right-click edits (Edit layout with it picked). While the bars are being arranged, the bar's own content
/// sits under its block's bar (Arrange's bar is drawn over the top of each block), so it stays visible.
fn bar_part<'a>(a: &App, id: &'static str, el: El<'a>) -> (&'static str, El<'a>) {
    let el = if a.describe.arr.as_ref().map(|x| x.page) == Some("bars") { container(el).padding(pad(26.0, 0.0, 0.0, 0.0)).into() } else { el };
    (id, hearth_ui::arrange::right_edit(el, Msg::EditAt("bars", Some(id))))
}

pub fn view(a: &App) -> El<'_> {
    let p = a.p;
    let picking = a.picking();
    let mut col = Column::new();
    if let Some(fm) = hearth_ui::ui::frame_meter() {
        col = col.push(fm); // CI's frame counter only (HEARTH_UI_FRAMES)
    }
    if picking {
        col = col.push(pick_head(a)).push(hline(p.line));
    }
    // Describe it's sidebar: "icons" (icons only) or "hidden", with the dashed outline while it's being tried
    let has_nav = a.nav_shown() && a.describe.knob("sidebar") != "hidden";
    let mut pieces: Vec<(&'static str, El)> = Vec::new();
    if has_nav {
        pieces.push(("sidebar", hearth_ui::describe::outline(&p, a.describe.changing("sidebar"), nav_pane(a))));
    }
    // The folder, with Act as administrator's red bar above it and the terminal panel (F4) under it.
    let mut mid = Column::new().width(Length::FillPortion(64)).height(Length::Fill);
    if crate::extras::admin_here(a) {
        mid = mid.push(admin_bar(a));
    }
    let folder: El = container(main_area(a)).width(Length::Fill).height(Length::Fill).into();
    let folder: El = if a.drag.is_some() && a.path.starts_with('/') {
        mouse_area(folder).on_enter(Msg::Dnd(crate::dnd::DMsg::Over(1, a.path.clone()))).on_exit(Msg::Dnd(crate::dnd::DMsg::Out(1, a.path.clone()))).into()
    } else {
        folder
    };
    mid = mid.push(folder);
    if a.term.on && !picking {
        let h = (a.area.get().1 * 0.4).clamp(150.0, 260.0);
        mid = mid.push(crate::terminal::view(a, h));
    }
    let mut folder_row = Row::new().height(Length::Fill).push(mid);
    // Split view (F9): a second folder beside this one.
    if a.split.is_some() && !picking {
        folder_row = folder_row.push(vline(p.line)).push(container(crate::split::view(a)).width(Length::FillPortion(36)).height(Length::Fill));
    }
    pieces.push(("folder", folder_row.into()));
    // Describe it's preview: "right" / "right-big" (on the right, bigger) shows it, "off" hides it
    let preview = a.describe.knob("preview");
    let has_preview = (a.details_pane || preview.starts_with("right")) && preview != "off" && !picking;
    if has_preview {
        pieces.push(("preview", hearth_ui::describe::outline(&p, a.describe.changing("preview"), details_pane(a))));
    }
    // its blocks (hearth_ui::blocks): the sidebar, the folder and the preview, moved, sized, hidden or restyled; the
    // file chooser has its own; as it was when nothing is
    let body = hearth_ui::arrange::parts(
        &a.describe,
        &p,
        if picking { "chooser" } else { "window" },
        pieces,
        move |ps| {
            let mut body = Row::new().height(Length::Fill);
            if has_nav {
                body = body.push(ps.take("sidebar")).push(vline(p.line));
            }
            body = body.push(ps.take("folder"));
            if has_preview {
                body = body.push(vline(p.line)).push(ps.take("preview"));
            }
            body.into()
        },
        Msg::Describe,
    );
    if picking {
        col = col.push(body);
        if let Some(j) = &a.job {
            col = col.push(hline(p.line)).push(job_bar(a, j));
        }
    } else {
        // The window's bars and the panes, each a block ("bars"): Describe it's tabs (none: the window's own buttons
        // stay, on a thin strip) and every bar from its pieces; as it was when nothing is.
        let top = zone_row(a, "top");
        let bar = zone_row(a, "bar");
        let (has_top, has_bar) = (top.is_some(), bar.is_some());
        let show_status = a.describe.knob("status") != "hidden";
        let mut items: Vec<(&'static str, El)> = vec![bar_part(a, "tabs", tab_strip(a))];
        if let Some(r) = top {
            items.push(bar_part(a, "address", r));
        }
        if let Some(r) = bar {
            items.push(bar_part(a, "toolbar", r));
        }
        items.push(("panes", column![hline(p.line), body].into()));
        if show_status {
            items.push(bar_part(a, "status", status_bar(a)));
        }
        // what sits between the panes and the status bar: the copy under way, Describe it's box (board 11), the
        // bottom bar; laid out, they stay under the blocks
        let mut extras: Vec<El> = Vec::new();
        if let Some(j) = &a.job {
            extras.push(hline(p.line));
            extras.push(job_bar(a, j));
        }
        if a.describe.open {
            extras.push(container(hearth_ui::describe::bar(&p, &a.describe, Msg::Describe)).padding(pad(12.0, 16.0, 12.0, 16.0)).into());
        }
        if let Some(r) = zone_row(a, "bottom") {
            extras.push(hline(p.line));
            extras.push(r);
        }
        let laid = a.describe.laid("bars").is_some() || a.describe.arr.as_ref().map(|x| x.page == "bars").unwrap_or(false);
        let (inner, outer) = if laid { (Vec::new(), extras) } else { (extras, Vec::new()) };
        let bars = hearth_ui::arrange::parts(
            &a.describe,
            &p,
            "bars",
            items,
            move |ps| {
                let mut c = Column::new().push(ps.take("tabs"));
                if has_top {
                    c = c.push(ps.take("address"));
                }
                if has_bar {
                    c = c.push(ps.take("toolbar"));
                }
                c = c.push(ps.take("panes"));
                for e in inner {
                    c = c.push(e);
                }
                if show_status {
                    c = c.push(ps.take("status"));
                }
                c.into()
            },
            Msg::Describe,
        );
        col = col.push(bars);
        for e in outer {
            col = col.push(e);
        }
    }
    if picking {
        col = col.push(hline(p.line)).push(pick_bar(a));
        // Game Mode: Hearth's on-screen keyboard at the bottom of the window (Y on the controller).
        if a.pad.kb {
            let h = (a.area.get().1 * 0.75).clamp(220.0, 300.0);
            col = col.push(responsive(move |size: Size| crate::osk_view::keyboard(a, Size::new(size.width, h))).height(Length::Fixed(h + 6.0)));
        }
    }
    let base: El = container(col).width(Length::Fill).height(Length::Fill).style(ui::boxed(p.bg, 0.0, None)).into();
    let base: El = mouse_area(base).on_move(Msg::Cursor).on_release(Msg::Dnd(crate::dnd::DMsg::Release)).into();
    // What's being dragged, beside the pointer.
    let base: El = match crate::dnd::ghost(a) {
        Some((at, g)) => stack![base, pin(g).x(at.x).y(at.y)].into(),
        None => base,
    };
    let base = match &a.overlay {
        Some(Overlay::Menu(m)) => {
            let (x, y) = menu_place(a, m);
            let catcher = mouse_area(Space::new().width(Length::Fill).height(Length::Fill)).on_press(Msg::MenuClose).on_right_press(Msg::MenuClose);
            stack![base, catcher, pin(opaque(menu(a, m))).x(x).y(y)].into()
        }
        Some(Overlay::Props(pr)) => ui::dialog(base, props_dialog(a, pr), None),
        Some(Overlay::Bc) => ui::dialog(base, crate::briefcase::dialog(a), None),
        Some(Overlay::Sort) => ui::dialog(base, crate::sortdl::dialog(a), Some(Msg::Sort(crate::sortdl::SMsg::Cancel))),
        Some(ov) => ui::dialog(base, dialog(a, ov), None),
        None => base,
    };
    // Sort my Downloads' note, with Undo (sortdl.rs), low in the middle of the window.
    let base: El = match crate::sortdl::note(a) {
        Some(n) => stack![base, container(opaque(n)).center_x(Length::Fill).align_bottom(Length::Fill).padding(pad(0.0, 0.0, 58.0, 0.0))].into(),
        None => base,
    };
    ui::edges(base, Msg::Resize)
}

fn menu_place(a: &App, m: &Menu) -> (f32, f32) {
    let items_h: f32 = m.items.iter().map(|i| if i.sep { 9.0 } else if i.heading { 28.0 } else if i.quick && m.short { 0.0 } else { 30.0 }).sum();
    let h = items_h + if m.short { 46.0 } else { 34.0 } + 10.0;
    menu_xy(m.at.x, m.at.y, m.width.max(200.0), h, a.win)
}

/// Where a right-click menu opens (Windows 11's rule): at the pointer, below and to the right of it; above it when
/// there's no room below, to its left when there's no room on the right; and only when it fits neither way, moved
/// just enough to stay in the window, never further from the pointer than that. (1.17 fix: it used a fixed 760-pixel
/// window, so a long menu jumped to the top of the window.)
pub fn menu_xy(px: f32, py: f32, mw: f32, mh: f32, win: (f32, f32)) -> (f32, f32) {
    let (ww, wh) = if win.0 > 50.0 && win.1 > 50.0 { win } else { (1600.0, 900.0) };
    let m = 4.0;
    let x = if px + mw <= ww - m { px } else if px - mw >= m { px - mw } else { (ww - mw - m).max(m) };
    let y = if py + mh <= wh - m { py } else if py - mh >= m { py - mh } else { (wh - mh - m).clamp(m, py.max(m)) };
    (x.max(m), y.max(m))
}

#[cfg(test)]
mod menu_place_tests {
    use super::menu_xy;
    #[test]
    fn opens_at_the_pointer() {
        assert_eq!(menu_xy(300.0, 200.0, 260.0, 300.0, (1366.0, 768.0)), (300.0, 200.0));
    }
    #[test]
    fn flips_up_when_no_room_below() {
        assert_eq!(menu_xy(300.0, 600.0, 260.0, 300.0, (1366.0, 768.0)), (300.0, 300.0));
    }
    #[test]
    fn flips_left_when_no_room_on_the_right() {
        assert_eq!(menu_xy(1300.0, 200.0, 260.0, 300.0, (1366.0, 768.0)).0, 1040.0);
    }
    #[test]
    fn a_tall_menu_moves_only_as_much_as_needed() {
        // 600 tall at y = 300 in a 768 window: fits neither way, so its bottom sits at the window's edge (y 164),
        // not at the top of the window.
        assert_eq!(menu_xy(300.0, 300.0, 260.0, 600.0, (1366.0, 768.0)).1, 164.0);
    }
}

// ------------------------------------------------------------------ the tabs (the window's title bar)
fn tab_strip(a: &App) -> El<'_> {
    zone_titlebar(a)
}

/// The tabs (each with its ✕), as they were drawn along the window's top.
fn tabs_row(a: &App) -> El<'_> {
    let p = a.p;
    let mut r = Row::new().spacing(2.0).align_y(iced::Bottom);
    for (i, tb_) in a.tabs.iter().enumerate() {
        let on = i == a.tab_index;
        let path = if on { a.path.clone() } else { tb_.path.clone() };
        let label = a.title_of(&path);
        let w = (label.chars().count() as f32 * 7.4 + 58.0).clamp(120.0, 240.0);
        let x = mouse_area(container(ico::<Msg>(Ic::Close, p.muted, 11.0)).padding(4.0)).on_press(Msg::CloseTab(i)).interaction(mouse::Interaction::Pointer);
        let inner = row![t1(ui::fit(&label, 26), 13.0, if on { p.text } else { p.muted }).font(if on { ui::weight(iced::font::Weight::Medium) } else { hearth_ui::app::INTER }), Space::new().width(Length::Fill), x]
            .align_y(iced::Center)
            .padding(pad(0.0, 8.0, 0.0, 16.0));
        let bg = if on { p.bg } else { Color::TRANSPARENT };
        let rr = p.r(8.0);
        let tab = container(inner).width(Length::Fixed(w)).height(Length::Fixed(32.0)).center_y(Length::Fixed(32.0)).style(move |_: &Theme| container::Style {
            background: Some(Background::Color(bg)),
            border: Border { radius: iced::border::Radius { top_left: rr, top_right: rr, bottom_left: 0.0, bottom_right: 0.0 }, ..Border::default() },
            ..container::Style::default()
        });
        r = r.push(drop_on(a, &path, mouse_area(tab).on_press(Msg::Tab(i)).on_middle_press(Msg::CloseTab(i)).into()));
    }
    r.into()
}

/// One of the window's own buttons, in Describe it's look for them ("" Windows-style, dots: macOS-style coloured
/// circles, square), wherever it's been put.
fn win_button<'a>(a: &'a App, id: &str) -> El<'a> {
    let p = a.p;
    let (ic, msg, red) = match id {
        "minimize" => (Ic::Minimize, Msg::Minimize, false),
        "maximize" => (Ic::Maximize, Msg::Maximize, false),
        _ => (Ic::Close, Msg::Close, true),
    };
    match a.describe.knob("winbuttons").as_str() {
        "dots" => {
            let c = match id {
                "close" => Color::from_rgb8(0xff, 0x5f, 0x57),
                "minimize" => Color::from_rgb8(0xfe, 0xbc, 0x2e),
                _ => Color::from_rgb8(0x28, 0xc8, 0x40),
            };
            let dot = container(Space::new()).width(Length::Fixed(12.0)).height(Length::Fixed(12.0)).style(ui::boxed(c, 6.0, None));
            mouse_area(container(dot).center_x(Length::Fixed(22.0)).center_y(Length::Fixed(40.0))).on_press(msg).interaction(mouse::Interaction::Pointer).into()
        }
        look => {
            let sq = look == "square";
            button(container(ico::<Msg>(ic, p.muted, 13.0)).center_x(Length::Fixed(if sq { 40.0 } else { 46.0 })).center_y(Length::Fixed(if sq { 32.0 } else { 40.0 })))
                .padding(0.0)
                .on_press(msg)
                .style(move |_: &Theme, st: button::Status| button::Style {
                    background: Some(Background::Color(match st {
                        button::Status::Hovered | button::Status::Pressed if red => Color::from_rgb8(0xc4, 0x2b, 0x1c),
                        button::Status::Hovered | button::Status::Pressed => p.hover(),
                        _ if sq => Pal::a(p.text, 0.06),
                        _ => Color::TRANSPARENT,
                    })),
                    text_color: p.text,
                    border: Border { radius: if sq { 4.0.into() } else { 0.0.into() }, ..Border::default() },
                    shadow: Shadow::default(),
                    snap: true,
                })
                .into()
        }
    }
}

/// The window's own bar (titlebar zone): the tabs, New tab, the folder's name, the window's buttons and empty space
/// that moves the window, in Describe it's order (the owner, Nov 22: "even the top bar were the minimise maximise and
/// the close button, The entire APP"). Other pieces put here are drawn as in any bar.
fn zone_titlebar(a: &App) -> El<'_> {
    let p = a.p;
    let mut items = zone_items(a, "titlebar");
    if a.describe.knob("tabs") == "hidden" {
        items.retain(|i| i != "tabs" && i != "newtab");
        if !items.iter().any(|i| i == "title") {
            items.insert(0, "title".into());
        }
    }
    let drag = || -> El<'_> { mouse_area(Space::new().width(Length::Fill).height(Length::Fixed(40.0))).on_press(Msg::Drag).on_double_click(Msg::Maximize).into() };
    let mut r = Row::new().align_y(iced::Center).padding(pad(0.0, 0.0, 0.0, 10.0));
    let mut dragged = false;
    for i in &items {
        let el: El<'_> = match i.as_str() {
            "tabs" => container(tabs_row(a)).height(Length::Fixed(40.0)).align_bottom(Length::Fixed(40.0)).into(),
            "newtab" => container(flat(&p, ico(Ic::Plus, p.muted, 15.0), Some(Msg::NewTab), pad(6.0, 8.0, 6.0, 8.0))).padding(pad(0.0, 0.0, 0.0, 4.0)).into(),
            "title" => container(t1(format!("{} — Hearth Files", a.title_of(&a.path)), 13.0, p.text)).padding(pad(0.0, 8.0, 0.0, 6.0)).into(),
            "minimize" | "maximize" | "close" => win_button(a, i),
            "spacer" => {
                dragged = true;
                drag()
            }
            other => match piece(a, other) {
                Some(e) => e,
                None => continue,
            },
        };
        r = r.push(el);
        if i == "title" || i == "newtab" || i == "tabs" {
            if a.describe.open && !items.iter().skip_while(|x| *x != i).skip(1).any(|x| x == "title" || x == "tabs" || x == "newtab") {
                r = r.push(container(hearth_ui::describe::editing::<Msg>(&p)).padding(pad(0.0, 0.0, 0.0, 10.0)));
            }
        }
    }
    if !dragged {
        // the window can always be moved: what's left of the bar moves it
        r = r.push(drag());
    }
    // the window's own buttons taken out of the title bar and put nowhere come back here (close is never lost)
    let placed = a.describe.placed();
    for b in ["minimize", "maximize", "close"] {
        if !items.iter().any(|x| x == b) && !placed.iter().any(|x| x == b) {
            r = r.push(win_button(a, b));
        }
    }
    let bar = container(r).width(Length::Fill).height(Length::Fixed(40.0)).style(ui::boxed(Pal::mix(p.bg, Color::BLACK, if p.dark { 0.3 } else { 0.06 }), 0.0, None));
    hearth_ui::describe::outline(&p, a.describe.zone_changing("titlebar") || a.describe.changing("winbuttons") || a.describe.changing("tabs"), bar.into())
}

// ------------------------------------------------------------------ back, forward, up, refresh, the address, search
fn address<'a>(a: &'a App, height: f32) -> El<'a> {
    let p = a.p;
    if a.editing_path {
        return text_input("", &a.path_text)
            .id(PATH_ID)
            .on_input(Msg::PathText)
            .on_submit(Msg::PathGo)
            .size(14.0)
            .padding(pad(6.0, 10.0, 6.0, 10.0))
            .style(input_style(p, true))
            .into();
    }
    let crumbs = a.crumbs();
    let lead = if a.on_home() { Ic::Home } else if a.in_trash() { Ic::TrashBin } else { Ic::Laptop };
    let mut r = Row::new().spacing(2.0).align_y(iced::Center).push(gap(10.0, 1.0)).push(ico(lead, p.text, 17.0)).push(gap(6.0, 1.0));
    let n = crumbs.len();
    // Long paths keep their last steps (the folder you're in).
    let skip = n.saturating_sub(6);
    for (i, c) in crumbs.iter().enumerate().skip(skip) {
        if i > skip {
            r = r.push(t("›", 15.0, p.muted));
        }
        let label = t1(ui::fit(&c.label, 28), 14.0, if drop_lit(a, &c.path) { p.accent } else { p.text });
        r = r.push(drop_on(a, &c.path, flat(&p, label.into(), Some(Msg::Go(c.path.clone())), pad(3.0, 6.0, 3.0, 6.0))));
    }
    let rest = mouse_area(Space::new().width(Length::Fill).height(Length::Fixed(height))).on_press(Msg::EditPath).interaction(mouse::Interaction::Text);
    r = r.push(rest);
    container(r).width(Length::Fill).height(Length::Fixed(height)).clip(true).style(ui::boxed(field_bg(&p), p.r(6.0), Some((p.line, 1.0)))).into()
}

/// Act as administrator: a red bar while it's on, with Stop (FileManager.qml).
fn admin_bar(a: &App) -> El<'_> {
    let p = a.p;
    let stop = button(container(t1("Stop", 13.0, p.text)).center_y(Length::Fixed(20.0)))
        .padding(pad(4.0, 14.0, 4.0, 14.0))
        .on_press(Msg::Fx(crate::extras::XMsg::AdminOff))
        .style(move |_: &Theme, st: button::Status| button::Style {
            background: Some(Background::Color(if matches!(st, button::Status::Hovered | button::Status::Pressed) { p.panel3 } else { p.panel2 })),
            text_color: p.text,
            border: Border { color: p.line, width: 1.0, radius: p.r(14.0).into() },
            shadow: Shadow::default(),
            snap: true,
        });
    let r = row![
        ico::<Msg>(Ic::Shield, p.danger, 18.0),
        container(t1("Acting as administrator: changes here affect the whole PC and ask for an administrator's password. What starts the PC stays locked.", 13.0, p.text).font(ui::weight(iced::font::Weight::Semibold))).width(Length::Fill).clip(true),
        stop,
    ]
    .spacing(10.0)
    .align_y(iced::Center)
    .padding(pad(0.0, 12.0, 0.0, 16.0));
    container(column![container(r).height(Length::Fixed(43.0)).center_y(Length::Fixed(43.0)), hline::<Msg>(p.danger)])
        .width(Length::Fill)
        .style(ui::boxed(Pal::a(p.danger, 0.16), 0.0, None))
        .into()
}

/// A folder, a place, a tab or an address step that takes a drop while something is dragged.
fn drop_on<'a>(a: &App, path: &str, inner: El<'a>) -> El<'a> {
    if a.drag.is_none() || path.is_empty() || !path.starts_with('/') {
        return inner;
    }
    mouse_area(inner).on_enter(Msg::Dnd(crate::dnd::DMsg::Over(0, path.to_string()))).on_exit(Msg::Dnd(crate::dnd::DMsg::Out(0, path.to_string()))).into()
}

fn drop_lit(a: &App, path: &str) -> bool {
    a.drag.is_some() && crate::dnd::target(a).as_deref() == Some(path)
}


// ------------------------------------------------------------------ the command bar
/// Sort my Downloads on the bar, only in Downloads: in the accent while something is loose (mockup 1), plain once
/// nothing is (mockup 2).
fn sort_button(a: &App) -> El<'_> {
    let p = a.p;
    if !crate::sortdl::here(a) || a.picking() {
        return Space::new().width(Length::Shrink).into();
    }
    let lit = crate::sortdl::loose(a) > 0;
    let c = if lit { Color::from_rgb8(0x1d, 0x12, 0x0c) } else { p.text };
    let inner = row![t1("✦", 14.0, c), t1("Sort my Downloads", 14.0, c).font(ui::weight(if lit { iced::font::Weight::Bold } else { iced::font::Weight::Normal }))].spacing(7.0).align_y(iced::Center);
    let bx = container(inner).padding(pad(6.0, 14.0, 6.0, 14.0)).style(ui::boxed(if lit { p.accent } else { Color::TRANSPARENT }, 8.0, if lit { None } else { Some((p.line, 1.0)) }));
    container(mouse_area(bx).on_press(Msg::Sort(crate::sortdl::SMsg::Open)).interaction(mouse::Interaction::Pointer)).padding(pad(0.0, 8.0, 0.0, 4.0)).into()
}


/// The default pieces of Files' bars (hearth-layout app.rs FILES.zones), and what Describe it changed: a zone it names
/// is its pieces in that order; a piece it put somewhere else (or hid) leaves its own zone (the owner, Nov 22: "they need
/// to be able to edit the top bar as well, basicly the enditre app").
const TITLEBAR: &[&str] = &["tabs", "newtab", "spacer", "minimize", "maximize", "close"];
const TOP: &[&str] = &["back", "forward", "up", "refresh", "address", "search"];
const BAR: &[&str] = &["new", "sep", "cut", "copy", "paste", "rename", "share", "delete", "sep", "sort", "view", "tools", "spacer", "sortdl", "features", "details", "more"];

pub fn zone_items(a: &App, zone: &str) -> Vec<String> {
    if let Some(z) = a.describe.zone(zone) {
        return z;
    }
    let placed = a.describe.placed();
    let own: &[&str] = match zone {
        "titlebar" => TITLEBAR,
        "top" => TOP,
        "bar" => BAR,
        _ => &[],
    };
    let v: Vec<String> = own.iter().filter(|i| **i == "sep" || **i == "spacer" || !placed.iter().any(|x| x == *i)).map(|i| i.to_string()).collect();
    // a piece an answer took out of its own zone without putting it anywhere (not even "hidden") stays where it was:
    // only what was asked for changes
    v
}

/// One piece of Files' bars.
fn piece<'a>(a: &'a App, id: &str) -> Option<El<'a>> {
    let p = a.p;
    let has = !a.targets().is_empty();
    let w = a.writable();
    let compact = a.describe.knob("toolbar") == "compact";
    let nb = |ic: Ic, on: bool, msg: Msg| -> El<'static> { flat(&p, ico(ic, if on { p.text } else { Pal::a(p.muted, 0.6) }, 17.0), if on { Some(msg) } else { None }, pad(7.0, 8.0, 7.0, 8.0)) };
    let ib = |ic: Ic, on: bool, cmd: &'static str| -> El<'static> {
        flat(&p, ico(ic, if on { p.text } else { Pal::a(p.muted, 0.55) }, 17.0), if on { Some(Msg::Cmd(cmd)) } else { None }, pad(7.0, 11.0, 7.0, 11.0))
    };
    let lb = |ic: Ic, label: &'static str, more: bool, on: bool, cmd: &'static str| -> El<'static> {
        let c = if on { p.text } else { Pal::a(p.muted, 0.55) };
        let mut r = Row::new().spacing(7.0).align_y(iced::Center).push(ico(ic, c, 16.0));
        if !compact {
            r = r.push(t1(label, 14.0, c));
        }
        if more {
            r = r.push(ico(Ic::ChevDown, c, 9.0));
        }
        flat(&p, r.into(), if on { Some(Msg::Cmd(cmd)) } else { None }, pad(6.0, 10.0, 6.0, 10.0))
    };
    Some(match id {
        "back" => nb(Ic::Back, !a.back.is_empty(), Msg::Back),
        "forward" => nb(Ic::Forward, !a.fwd.is_empty(), Msg::Forward),
        "up" => nb(Ic::Up, !(a.path == "/" || a.on_home()), Msg::Up),
        "refresh" => nb(Ic::Refresh, true, Msg::Refresh),
        "address" => container(address(a, 32.0)).padding(pad(0.0, 4.0, 0.0, 6.0)).width(Length::Fill).into(),
        "search" => {
            let place = if a.found.is_some() { format!("Found “{}”", a.found_query) } else if a.on_home() { "Search Home".into() } else if a.on_this_pc() { "Search".into() } else { format!("Search {}", a.title_of(&a.path)) };
            let mut sr = Row::new().align_y(iced::Center).push(
                text_input(&place, &a.search_text).id(SEARCH_ID).on_input(Msg::SearchText).on_submit(Msg::SearchGo).size(14.0).padding(pad(6.0, 10.0, 6.0, 12.0)).style(input_style(p, true)),
            );
            if !a.search_text.is_empty() || a.found.is_some() {
                sr = sr.push(flat(&p, ico(Ic::Close, p.muted, 12.0), Some(Msg::SearchClear), pad(6.0, 6.0, 6.0, 6.0)));
            }
            container(sr).width(Length::Fixed(if a.area.get().0 < 700.0 { 170.0 } else { 240.0 })).padding(pad(0.0, 0.0, 0.0, 4.0)).into()
        }
        "new" => lb(Ic::Plus, "New", true, w, "new"),
        "cut" => ib(Ic::Cut, has && w, "cut"),
        "copy" => ib(Ic::Copy, has, "copy"),
        "paste" => ib(Ic::Paste, w, "paste"),
        "rename" => ib(Ic::Rename, a.current().is_some() && !a.on_special() && !a.on_home() && w, "rename"),
        "share" => ib(Ic::Share, has || !a.on_home(), "share"),
        "delete" => ib(Ic::Delete, has && (w || a.in_trash()), "delete"),
        "sort" => lb(Ic::Sort, "Sort", true, true, "sort"),
        "view" => lb(Ic::ViewList, "View", true, true, "view"),
        "tools" => bc_tools(a),
        "sortdl" => sort_button(a),
        "features" => match crate::ldescribe::buttons(a) {
            Some(b) => container(b).padding(pad(0.0, 8.0, 0.0, 0.0)).into(),
            None => return None,
        },
        "details" => {
            let c = if a.details_pane { p.accent } else { p.text };
            let mut r = Row::new().spacing(7.0).align_y(iced::Center);
            if !compact {
                r = r.push(t1("Details", 14.0, c));
            }
            flat(&p, r.push(ico(Ic::Pane, c, 16.0)).into(), Some(Msg::Cmd("details")), pad(6.0, 10.0, 6.0, 10.0))
        }
        "more" => ib(Ic::Dots, true, "more"),
        "sep" => container(vline::<Msg>(p.line)).height(Length::Fixed(20.0)).padding(pad(0.0, 6.0, 0.0, 6.0)).into(),
        "spacer" => Space::new().width(Length::Fill).into(),
        "minimize" | "maximize" | "close" => win_button(a, id),
        "tabs" => tabs_row(a),
        "newtab" => flat(&p, ico(Ic::Plus, p.muted, 15.0), Some(Msg::NewTab), pad(6.0, 8.0, 6.0, 8.0)),
        "title" => t1(format!("{} — Hearth Files", a.title_of(&a.path)), 13.0, p.text).into(),
        _ => return None,
    })
}

/// A zone of Files' bars (top: the address row; bar: the buttons; bottom: a bar above the status bar), with Describe
/// it's dashed outline while it's being changed; None when it's empty.
fn zone_row<'a>(a: &'a App, zone: &str) -> Option<El<'a>> {
    let items = zone_items(a, zone);
    if items.iter().all(|i| i == "sep" || i == "spacer") {
        return None;
    }
    let mut r = Row::new().align_y(iced::Center).spacing(if zone == "top" { 4.0 } else { 2.0 });
    for i in &items {
        if let Some(e) = piece(a, i) {
            r = r.push(e);
        }
    }
    let (t, b) = match a.describe.knob("density").as_str() {
        "roomy" => (12.0, 10.0),
        "compact" => (2.0, 2.0),
        _ => if zone == "top" { (8.0, 6.0) } else { (2.0, 6.0) },
    };
    let feats = zone == "bar" && a.describe.open && a.describe.trying().map(|r| r.get("features").and_then(Value::as_array).map(|f| !f.is_empty()).unwrap_or(false)).unwrap_or(false);
    let on = a.describe.zone_changing(zone) || (zone == "bar" && a.describe.changing("toolbar")) || feats;
    Some(hearth_ui::describe::outline(&a.p, on, container(r).padding(pad(t, 12.0, b, 12.0)).width(Length::Fill).into()))
}

/// Inside a Briefcase, the command bar's own buttons: Update All, Unlock or Lock, Customize (FileManager.qml).
fn bc_tools(a: &App) -> El<'_> {
    let p = a.p;
    if !crate::briefcase::in_briefcase(a) {
        return Space::new().into();
    }
    let mut r = Row::new().spacing(2.0).align_y(iced::Center).push(container(vline::<Msg>(p.line)).height(Length::Fixed(20.0)).padding(pad(0.0, 6.0, 0.0, 6.0)));
    let look = a.listing.as_ref().map(|l| l.briefcase.clone()).unwrap_or(Value::Null);
    for (t, ic, label) in [("bc-update-all", Ic::Refresh, "Update All"), ("bc-unlock", Ic::Lock, "Unlock"), ("bc-lock", Ic::Lock, "Lock"), ("bc-customize", crate::briefcase::icon_of(&look), "Customize")] {
        if !crate::briefcase::tool_shown(a, t) {
            continue;
        }
        let c = if t == "bc-customize" { crate::briefcase::color_of(a, &look) } else { p.text };
        let inner = row![ico::<Msg>(ic, c, 16.0), t1(label, 14.0, p.text)].spacing(7.0).align_y(iced::Center);
        let cmd: &'static str = t;
        r = r.push(flat(&p, inner.into(), Some(Msg::Cmd(cmd)), pad(6.0, 10.0, 6.0, 10.0)));
    }
    r.into()
}

// ------------------------------------------------------------------ the left menu
fn nav_pane(a: &App) -> El<'_> {
    let p = a.p;
    let icons_only = a.describe.knob("sidebar") == "icons";
    let wide = a.nav_w >= 200.0;
    let wide = wide && !icons_only;
    let mut col = Column::new().spacing(1.0).padding(pad(10.0, 4.0, 10.0, 4.0));
    let mut last_section = String::new();
    for (i, q) in a.places.iter().enumerate() {
        if i > 0 && q.section != last_section {
            col = col.push(container(hline::<Msg>(p.line)).padding(pad(6.0, 8.0, 6.0, 8.0)));
        }
        last_section = q.section.clone();
        let here = !q.path.is_empty() && a.path == q.path && !(q.id == "gallery");
        let lit = a.zone == Zone::Places && a.place_index == i;
        let depth = q.depth as f32;
        let arrow: El = if !q.tree.is_empty() && !q.empty {
            mouse_area(container(ico::<Msg>(if q.open { Ic::ChevDown } else { Ic::ChevRight }, p.muted, 8.0)).center_x(Length::Fixed(12.0)).center_y(Length::Fixed(26.0)))
                .on_press(Msg::Fold(q.tree.clone()))
                .interaction(mouse::Interaction::Pointer)
                .into()
        } else {
            gap(12.0, 26.0)
        };
        let tag_dot = if q.kind == "tag" { Some(crate::extras::tag_color(&q.label)) } else { None };
        let bc_look = if q.kind == "briefcase" { q.raw.get("look").cloned().filter(|l| l.is_object()).or_else(|| Some(serde_json::json!({}))) } else { None };
        let ic = match q.id.as_str() {
            "gallery" => Ic::Gallery,
            "homeserver" => Ic::Cloud,
            _ if q.kind == "system" || (q.section == "pc" && q.depth == 1 && q.raw.get("letter").is_some() && q.kind != "usb" && q.raw.get("removable") != Some(&Value::Bool(true))) => Ic::Disc,
            // CD/DVD drives (hearth-files' optical_drives): a disc, not a USB stick.
            _ if q.kind == "optical" => Ic::Disc,
            _ if q.raw.get("removable") == Some(&Value::Bool(true)) => Ic::Usb,
            _ => icons::place_icon(&q.kind),
        };
        let lead: El = match (&tag_dot, &bc_look) {
            (Some(c), _) => container(Space::new().width(Length::Fixed(10.0)).height(Length::Fixed(10.0))).style(ui::boxed(*c, 5.0, None)).center_x(Length::Fixed(16.0)).center_y(Length::Fixed(16.0)).into(),
            (_, Some(l)) => crate::briefcase::bc_icon(a, l, false, 16.0),
            _ => ico(ic, p.text, 16.0),
        };
        if icons_only {
            // board 11: one icon per place, its name as the tooltip's place would be (top-level places only)
            if q.depth > 0 {
                continue;
            }
            let here = !q.path.is_empty() && a.path == q.path;
            let bg = if here { p.sel() } else { Color::TRANSPARENT };
            let cell = container(lead).center_x(Length::Fixed(40.0)).center_y(Length::Fixed(36.0)).style(ui::boxed(bg, p.r(6.0), None));
            col = col.push(container(mouse_area(cell).on_press(Msg::Place(i)).on_right_press(Msg::PlaceRight(i)).interaction(mouse::Interaction::Pointer)).center_x(Length::Fill));
            continue;
        }
        let mut r = Row::new()
            .spacing(0.0)
            .align_y(iced::Center)
            .push(gap(depth * 9.0, 1.0))
            .push(arrow)
            .push(gap(4.0, 1.0))
            .push(lead)
            .push(gap(8.0, 1.0))
            .push(t1(q.label.clone(), 13.5, if lit { p.ink } else { p.text }).font(if here { ui::weight(iced::font::Weight::Medium) } else { hearth_ui::app::INTER }));
        r = r.push(Space::new().width(Length::Fill));
        if wide && !q.sub.is_empty() && (q.id == "homeserver" || q.id == "linux") {
            r = r.push(t1(q.sub.clone(), 12.5, p.muted));
        }
        if q.kind == "tag" {
            r = r.push(t1(q.sub.clone(), 12.0, p.muted));
        }
        if wide && q.pinned {
            r = r.push(ico(Ic::Pin, p.text, 13.0));
        }
        if q.kind == "trash" {
            let n = q.raw.get("count").and_then(Value::as_u64).unwrap_or(0);
            if n > 0 {
                r = r.push(t1(n.to_string(), 12.0, p.muted));
            }
        }
        let over = drop_lit(a, &q.path);
        let bg = if lit { p.accent } else if here || over { p.sel() } else { Color::TRANSPARENT };
        let accent = p.accent;
        let rr = p.r(6.0);
        let ring = here || over;
        let cell = container(r.padding(pad(0.0, 8.0, 0.0, 0.0))).width(Length::Fill).height(Length::Fixed(28.0)).center_y(Length::Fixed(28.0)).style(move |_: &Theme| container::Style {
            background: Some(Background::Color(bg)),
            border: Border { color: if ring { Pal::a(accent, if over { 1.0 } else { 0.7 }) } else { Color::TRANSPARENT }, width: if ring { 1.0 } else { 0.0 }, radius: rr.into() },
            ..container::Style::default()
        });
        let cell: El = if here {
            // The bar on the place you're in (Windows 11's navigation pane).
            let bar = container(Space::new().width(Length::Fixed(3.0)).height(Length::Fixed(16.0))).style(ui::boxed(accent, 2.0, None));
            stack![cell, container(bar).height(Length::Fixed(28.0)).center_y(Length::Fixed(28.0))].into()
        } else {
            cell.into()
        };
        let tgt = if q.kind == "trash" { a.trash_path() } else { q.path.clone() };
        col = col.push(drop_on(a, &tgt, mouse_area(cell).on_press(Msg::Place(i)).on_right_press(Msg::PlaceRight(i)).on_middle_press(Msg::PlaceMiddle(i)).interaction(mouse::Interaction::Pointer).into()));
    }
    container(scrollable(col).height(Length::Fill)).width(Length::Fixed(if icons_only { 64.0 } else { a.nav_w })).height(Length::Fill).into()
}

// ------------------------------------------------------------------ the folder
fn main_area(a: &App) -> El<'_> {
    let p = a.p;
    if let Some(l) = &a.listing {
        if !l.error.is_empty() {
            return container(
                column![ico::<Msg>(Ic::Folder, p.muted, 54.0), ts("Can't open this folder", 18.0, p.text), t(l.error.clone(), 13.0, p.muted)].spacing(10.0).align_x(iced::Center),
            )
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into();
        }
    }
    if a.on_home() {
        return home_page(a);
    }
    if a.on_this_pc() {
        return this_pc(a);
    }
    if a.on_cview() {
        return c_page(a);
    }
    if a.on_apps() {
        return apps_page(a);
    }
    if a.entries.is_empty() {
        let msg = if a.loading {
            String::new()
        } else if a.search.is_some() {
            format!("Looking for “{}”…", a.found_query)
        } else if a.found.is_some() {
            format!("Nothing called “{}” in here", a.found_query)
        } else if !a.query.is_empty() {
            format!("Nothing here matches “{}”", a.query)
        } else {
            "This folder is empty".into()
        };
        let bg = mouse_area(container(t(msg, 14.0, p.muted)).center_x(Length::Fill).center_y(Length::Fill)).on_press(Msg::BgPress).on_right_press(Msg::BgRight);
        return bg.into();
    }
    match a.shown_view() {
        View::Details => details_list(a),
        View::Compact => compact_list(a),
        v => icon_grid(a, v),
    }
}

/// The scrollable every long view sits in (its offset is kept for drawing only the rows on screen).
/// The file view's empty space: a press clears the picks and starts drag to select; the band is drawn over the files.
fn bg_area<'a>(a: &'a App, content: El<'a>) -> El<'a> {
    let inner: El = match a.band_rect() {
        Some(r) => stack![
            content,
            pin(container(Space::new().width(Length::Fixed(r.width.max(1.0))).height(Length::Fixed(r.height.max(1.0)))).style({
                let c = a.p.accent;
                move |_: &Theme| container::Style { background: Some(Background::Color(Pal::a(c, 0.18))), border: Border { color: Pal::a(c, 0.8), width: 1.0, radius: 2.0.into() }, ..container::Style::default() }
            }))
            .x(r.x)
            .y(r.y)
        ]
        .into(),
        None => content,
    };
    mouse_area(inner).on_press(Msg::BgPress).on_right_press(Msg::BgRight).on_move(Msg::BandMove).into()
}

fn scroller<'a>(inner: El<'a>) -> El<'a> {
    scrollable(inner).id(SCROLL_ID).on_scroll(|v| Msg::Scrolled(v.absolute_offset().y)).height(Length::Fill).width(Length::Fill).into()
}

fn entry_icon<'a>(a: &'a App, e: &Entry, size: f32) -> El<'a> {
    if let Some(cv) = &e.cv {
        if cv.lock {
            return ico(Ic::Lock, a.p.text, size);
        }
    }
    // A Briefcase: its style in its colour, its badge, a padlock when it's locked.
    let base: El = if e.briefcase.is_object() {
        crate::briefcase::bc_icon(a, &e.briefcase, e.briefcase.get("locked").and_then(Value::as_bool).unwrap_or(false), size)
    } else if e.dir && crate::sortdl::here(a) && crate::sortdl::TOP.contains(&e.name.as_str()) {
        // Sort my Downloads' five folders, each in its colour (mockup 2).
        crate::sortdl::folder_icon(&e.name, size)
    } else {
        ico(icons::file_icon(model::kind_of(e), &e.name), a.p.text, size)
    };
    with_marks(a, e, base, size)
}

/// The small marks on an item's icon (FilesMarks.qml, Briefcase.qml): its git status (a coloured badge at the bottom
/// right: M changed, + added, ? not in git yet, ! in conflict, ✓ up to date), its tags (coloured dots at the bottom
/// left, as Dolphin shows them) and, in a Briefcase, its status (a badge at the top right).
fn with_marks<'a>(a: &'a App, e: &Entry, base: El<'a>, size: f32) -> El<'a> {
    let p = a.p;
    let git = crate::extras::git_of(a, e);
    let git = if git == "ignored" { "" } else { git };
    let bcb = if size >= 14.0 { crate::briefcase::badge_for(a, e) } else { None };
    if git.is_empty() && e.tags.is_empty() && bcb.is_none() {
        return base;
    }
    let mut layers: Vec<El<'a>> = vec![container(base).center_x(Length::Fixed(size)).center_y(Length::Fixed(size)).into()];
    let bg = p.panel;
    if !git.is_empty() {
        let d = (size * 0.3).clamp(9.0, 28.0);
        let c = crate::extras::git_color(a, git);
        let (mark, _) = crate::extras::git_mark(git);
        let dot = container(t1(mark, (d * 0.56).max(7.0), Color::WHITE).font(ui::weight(iced::font::Weight::Bold))).center_x(Length::Fixed(d)).center_y(Length::Fixed(d)).style(ui::boxed(c, d / 2.0, Some((bg, if size >= 20.0 { 2.0 } else { 1.0 }))));
        layers.push(container(dot).align_right(Length::Fixed(size)).align_bottom(Length::Fixed(size)).into());
    }
    if !e.tags.is_empty() {
        let d = (size * 0.16).clamp(5.0, 14.0);
        let mut r = Row::new().spacing(1.0);
        for tg in e.tags.iter().take(3) {
            r = r.push(container(Space::new().width(Length::Fixed(d)).height(Length::Fixed(d))).style(ui::boxed(crate::extras::tag_color(tg), d / 2.0, Some((bg, 1.0)))));
        }
        layers.push(container(r).align_left(Length::Fixed(size)).align_bottom(Length::Fixed(size)).into());
    }
    if let Some((c, g)) = bcb {
        let d = (size * 0.3).max(10.0);
        let dot = container(t1(g, (d * 0.62).max(7.0), Color::WHITE).font(ui::weight(iced::font::Weight::Bold))).center_x(Length::Fixed(d)).center_y(Length::Fixed(d)).style(ui::boxed(c, d / 2.0, Some((p.bg, 1.0))));
        layers.push(container(dot).align_right(Length::Fixed(size)).align_top(Length::Fixed(size)).into());
    }
    iced::widget::Stack::with_children(layers).width(Length::Fixed(size)).height(Length::Fixed(size)).into()
}

/// What a row does with the mouse: click picks, double-click opens, right-click asks, middle-click opens in a tab.
fn clickable<'a>(inner: El<'a>, i: usize) -> El<'a> {
    mouse_area(inner).on_press(Msg::Press(i)).on_right_press(Msg::Right(i)).on_middle_press(Msg::Middle(i)).into()
}

fn name_cell<'a>(a: &'a App, e: &Entry, size: f32) -> El<'a> {
    let p = a.p;
    if a.renaming.as_deref() == Some(e.path.as_str()) {
        return text_input("", &a.rename_text).id(RENAME_ID).on_input(Msg::RenameText).on_submit(Msg::RenameDone).size(size).padding(pad(2.0, 6.0, 2.0, 6.0)).style(input_style(p, true)).into();
    }
    let mut name = model::shown_name(e, a.show_ext);
    if a.found.is_some() && !e.where_.is_empty() && e.where_ != "." {
        name = format!("{name}  ·  {}", e.where_);
    }
    if !e.orig_path.is_empty() {
        name = format!("{name}   ·   from {}", model::parent_of(&e.orig_path).replacen(&a.home, "~", 1));
    }
    let dim = e.cv.as_ref().map(|c| c.dim).unwrap_or(false) || e.name.starts_with('.');
    t1(name, size, if dim { p.muted } else { p.text }).into()
}

fn details_header<'a>(a: &'a App, cols: &[(&'static str, &'static str, f32)]) -> El<'a> {
    let p = a.p;
    let mut r = Row::new().align_y(iced::Center).padding(pad(0.0, 14.0, 0.0, 14.0));
    for (key, label, w) in cols {
        let arrow = if a.sort == *key && !key.is_empty() { if a.desc { "  ˅" } else { "  ˄" } } else { "" };
        let cell = container(t1(format!("{label}{arrow}"), 13.5, p.muted)).width(if *w == 0.0 { Length::Fill } else { Length::Fixed(*w) });
        let k: &'static str = *key;
        r = r.push(if k.is_empty() { cell.into() } else { El::from(mouse_area(cell).on_press(Msg::Cmd(match k { "date" => "sortdate", "type" => "sorttype", "size" => "sortsize", _ => "sortname" })).interaction(mouse::Interaction::Pointer)) });
    }
    container(column![container(r).height(Length::Fixed(30.0)).center_y(Length::Fixed(30.0)), container(hline::<Msg>(p.line)).padding(pad(0.0, 12.0, 0.0, 12.0))]).into()
}

fn row_box<'a>(a: &'a App, inner: El<'a>, sel: bool, lit: bool, h: f32) -> El<'a> {
    let p = a.p;
    let bg = if sel { p.sel() } else { row_bg(&p) };
    let ring = lit && a.zone == Zone::Main && a.touched;
    let accent = p.accent;
    let rr = p.r(4.0);
    container(inner)
        .width(Length::Fill)
        .height(Length::Fixed(h))
        .center_y(Length::Fixed(h))
        .style(move |_: &Theme| container::Style {
            background: Some(Background::Color(bg)),
            border: Border { color: if ring && !sel { Pal::a(accent, 0.6) } else { Color::TRANSPARENT }, width: if ring && !sel { 1.0 } else { 0.0 }, radius: rr.into() },
            ..container::Style::default()
        })
        .into()
}

enum DRow {
    Head(String),
    Item(usize),
}

fn d_rows(a: &App) -> Vec<DRow> {
    let mut out = Vec::with_capacity(a.entries.len() + 8);
    let mut last = String::new();
    for (i, e) in a.entries.iter().enumerate() {
        if a.group != "none" {
            let g = a.group_name(e);
            if g != last {
                out.push(DRow::Head(g.clone()));
                last = g;
            }
        }
        out.push(DRow::Item(i));
    }
    out
}

fn details_list(a: &App) -> El<'_> {
    let p = a.p;
    let in_bc = crate::briefcase::in_briefcase(a);
    let header = if in_bc {
        details_header(a, &[("name", "Name", 0.0), ("date", "Date modified", 160.0), ("type", "Type", 150.0), ("size", "Size", 96.0), ("", "Status", 190.0)])
    } else {
        details_header(a, &[("name", "Name", 0.0), ("date", "Date modified", 160.0), ("type", "Type", 150.0), ("size", "Size", 96.0)])
    };
    let rows = d_rows(a);
    let total = rows.len();
    let body = responsive(move |size: Size| {
        a.area.set((size.width, size.height));
        a.cols.set(1);
        let rh = ROW_H + 2.0;
        let first = ((a.scroll_y / rh).floor() as usize).saturating_sub(4);
        let last = (((a.scroll_y + size.height) / rh).ceil() as usize + 4).min(total);
        let mut col = Column::new().spacing(2.0).padding(pad(4.0, 12.0, 8.0, 12.0));
        if first > 0 {
            col = col.push(gap(1.0, first as f32 * rh - 2.0));
        }
        for r in rows.iter().take(last).skip(first) {
            match r {
                DRow::Head(g) => col = col.push(container(t1(g.clone(), 13.0, p.accent).font(ui::weight(iced::font::Weight::Semibold))).height(Length::Fixed(ROW_H)).center_y(Length::Fixed(ROW_H)).padding(pad(0.0, 0.0, 0.0, 4.0))),
                DRow::Item(i) => {
                    let e = &a.entries[*i];
                    let sel = a.marked.contains(&e.path) || (a.marked.is_empty() && *i == a.cur && a.zone == Zone::Main && a.has_focus_sel());
                    let mut r = row![
                        container(row![entry_icon(a, e, 16.0), name_cell(a, e, 14.0)].spacing(9.0).align_y(iced::Center)).width(Length::Fill),
                        container(t1(model::fmt_date(e.mtime, false), 13.5, p.muted)).width(Length::Fixed(160.0)).clip(true),
                        container(t1(model::kind_name(e), 13.5, p.muted)).width(Length::Fixed(150.0)).padding(pad(0.0, 8.0, 0.0, 0.0)).clip(true),
                        container(t1(if e.dir { String::new() } else { model::fmt_size(e.size) }, 13.5, p.muted)).width(Length::Fixed(96.0)).clip(true),
                    ]
                    .align_y(iced::Center)
                    .padding(pad(0.0, 2.0, 0.0, 12.0));
                    if in_bc {
                        // A Briefcase's Status column: each item's status in its colour (Briefcase.qml).
                        let c = crate::briefcase::badge_for(a, e).map(|b| b.0).unwrap_or(p.muted);
                        r = r.push(container(row![container(Space::new().width(Length::Fixed(8.0)).height(Length::Fixed(8.0))).style(ui::boxed(c, 4.0, None)), t1(crate::briefcase::status_label(a, e), 13.0, p.text)].spacing(7.0).align_y(iced::Center)).width(Length::Fixed(190.0)).clip(true));
                    }
                    let el = clickable(row_box(a, r.into(), sel || drop_lit(a, &e.path), *i == a.cur, ROW_H), *i);
                    col = col.push(if e.dir { drop_on(a, &e.path, el) } else { el });
                }
            }
        }
        if last < total {
            col = col.push(gap(1.0, (total - last) as f32 * rh));
        }
        scroller(bg_area(a, container(col).width(Length::Fill).into()))
    });
    column![header, body].into()
}

impl App {
    /// The row you're on looks picked once you've clicked or moved (a folder opens with nothing lit, as the mockups
    /// show it).
    pub fn has_focus_sel(&self) -> bool {
        self.touched
    }
}

fn compact_list(a: &App) -> El<'_> {
    let p = a.p;
    responsive(move |size: Size| {
        a.area.set((size.width, size.height));
        let per = ((size.height - 12.0) / ROW_H).floor().max(1.0) as usize;
        let mut cols = Row::new().spacing(6.0).padding(6.0);
        let mut i = 0;
        while i < a.entries.len() {
            let mut c = Column::new().width(Length::Fixed(230.0));
            for j in i..(i + per).min(a.entries.len()) {
                let e = &a.entries[j];
                let sel = a.marked.contains(&e.path) || (a.marked.is_empty() && j == a.cur && a.has_focus_sel());
                let r = row![entry_icon(a, e, 16.0), name_cell(a, e, 13.5)].spacing(8.0).align_y(iced::Center).padding(pad(0.0, 6.0, 0.0, 8.0));
                let el = clickable(row_box(a, r.into(), sel || drop_lit(a, &e.path), j == a.cur, ROW_H), j);
                c = c.push(if e.dir { drop_on(a, &e.path, el) } else { el });
            }
            cols = cols.push(c);
            i += per;
        }
        a.cols.set(1);
        let _ = p;
        scrollable(bg_area(a, cols.into()))
            .direction(scrollable::Direction::Horizontal(scrollable::Scrollbar::new()))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    })
    .into()
}

fn thumb_of<'a>(a: &'a App, e: &Entry) -> Option<&'a iced::widget::image::Handle> {
    a.thumbs.get(&e.path).and_then(|t| t.1.as_ref())
}

fn icon_grid(a: &App, v: View) -> El<'_> {
    let p = a.p;
    let (cw, ch) = v.cell();
    responsive(move |size: Size| {
        a.area.set((size.width, size.height));
        let inner_w = size.width - 24.0;
        let cols = ((inner_w + 8.0) / (cw + 8.0)).floor().max(1.0) as usize;
        a.cols.set(cols);
        let rh = ch + 8.0;
        let nrows = a.entries.len().div_ceil(cols);
        let first = ((a.scroll_y / rh).floor() as usize).saturating_sub(2);
        let last = (((a.scroll_y + size.height) / rh).ceil() as usize + 2).min(nrows);
        let mut col = Column::new().spacing(8.0).padding(pad(10.0, 12.0, 10.0, 12.0));
        if first > 0 {
            col = col.push(gap(1.0, first as f32 * rh - 8.0));
        }
        for r in first..last {
            let mut rw = Row::new().spacing(8.0);
            for c in 0..cols {
                let i = r * cols + c;
                let Some(e) = a.entries.get(i) else { break };
                rw = rw.push(cell(a, e, i, v));
            }
            col = col.push(rw);
        }
        if last < nrows {
            col = col.push(gap(1.0, (nrows - last) as f32 * rh));
        }
        let _ = p;
        scroller(bg_area(a, container(col).width(Length::Fill).into()))
    })
    .into()
}

fn cell<'a>(a: &'a App, e: &Entry, i: usize, v: View) -> El<'a> {
    let p = a.p;
    let (cw, ch) = v.cell();
    let sel = a.marked.contains(&e.path) || (a.marked.is_empty() && i == a.cur && a.has_focus_sel());
    let lit = i == a.cur && a.zone == Zone::Main && a.has_focus_sel();
    let accent = p.accent;
    let rr = p.r(8.0);
    let panel = row_bg(&p);
    let selbg = p.sel();
    let line = p.line;
    let frame = move |sel: bool, lit: bool| {
        move |_: &Theme| container::Style {
            background: Some(Background::Color(if sel { selbg } else { panel })),
            border: Border { color: if sel || lit { accent } else { Pal::a(line, 0.6) }, width: if sel || lit { 2.0 } else { 1.0 }, radius: rr.into() },
            ..container::Style::default()
        }
    };
    let inner: El = match v {
        View::XLarge | View::Large => {
            let th = ch - 32.0;
            let pic: El = match thumb_of(a, e) {
                Some(h) => {
                    let im: El = img(h).width(Length::Fill).height(Length::Fixed(th)).content_fit(ContentFit::Cover).border_radius(iced::border::Radius { top_left: rr - 1.0, top_right: rr - 1.0, bottom_left: 0.0, bottom_right: 0.0 }).into();
                    let marks = with_marks(a, e, Space::new().into(), (th * 0.4).min(56.0));
                    stack![im, container(marks).align_right(Length::Fill).align_bottom(Length::Fixed(th)).padding(6.0)].into()
                }
                None => container(entry_icon(a, e, th * 0.55)).center_x(Length::Fill).center_y(Length::Fixed(th)).into(),
            };
            column![pic, container(name_cell(a, e, 14.0)).padding(pad(0.0, 10.0, 0.0, 10.0)).height(Length::Fixed(32.0)).center_y(Length::Fixed(32.0)).clip(true)].into()
        }
        View::Grid => {
            let pic: El = match thumb_of(a, e) {
                Some(h) => img(h).width(Length::Fixed(64.0)).height(Length::Fixed(56.0)).content_fit(ContentFit::Contain).into(),
                None => entry_icon(a, e, 48.0),
            };
            column![container(pic).center_x(Length::Fill).height(Length::Fixed(62.0)).center_y(Length::Fixed(62.0)), container(text(model::shown_name(e, a.show_ext)).size(13.0).color(p.text).center()).padding(pad(0.0, 6.0, 0.0, 6.0)).center_x(Length::Fill).height(Length::Fixed(34.0)).clip(true)]
                .padding(pad(4.0, 0.0, 0.0, 0.0))
                .into()
        }
        View::Small => row![entry_icon(a, e, 20.0), name_cell(a, e, 13.5)].spacing(8.0).align_y(iced::Center).padding(pad(0.0, 8.0, 0.0, 8.0)).into(),
        _ => {
            let pic: El = match thumb_of(a, e) {
                Some(h) => img(h).width(Length::Fixed(44.0)).height(Length::Fixed(44.0)).content_fit(ContentFit::Cover).into(),
                None => entry_icon(a, e, 38.0),
            };
            row![pic, column![name_cell(a, e, 14.0), t1(format!("{}{}", model::kind_name(e), if e.dir { String::new() } else { format!("  ·  {}", model::fmt_size(e.size)) }), 12.5, p.muted)].spacing(2.0)]
                .spacing(10.0)
                .align_y(iced::Center)
                .padding(pad(0.0, 8.0, 0.0, 8.0))
                .into()
        }
    };
    let over = drop_lit(a, &e.path);
    let bx = container(inner).width(Length::Fixed(cw)).height(Length::Fixed(ch)).center_y(Length::Fixed(ch)).clip(true).style(frame(sel || over, lit || over));
    let el = clickable(bx.into(), i);
    if e.dir { drop_on(a, &e.path, el) } else { el }
}

// ------------------------------------------------------------------ the Home page (mockup files-3)
fn home_page(a: &App) -> El<'_> {
    let p = a.p;
    let qa = a.quick_access();
    responsive(move |size: Size| {
        a.area.set((size.width, size.height));
        a.cols.set(1);
        let mut col = Column::new().padding(pad(4.0, 14.0, 12.0, 14.0)).spacing(0.0);
        col = col.push(heading(&p, "Quick access"));
        let per = 3usize;
        let tw = ((size.width - 28.0 - 16.0) / per as f32).floor();
        for chunk in qa.chunks(per) {
            let mut r = Row::new().spacing(8.0);
            for q in chunk {
                let sub = if q.id == "savedgames" && a.saved_games > 0 {
                    format!("Pinned · {} games", a.saved_games)
                } else {
                    "Pinned".to_string()
                };
                let inner = row![ico(icons::place_icon(&q.kind), p.text, 28.0), column![t1(q.label.clone(), 15.0, p.text).font(ui::weight(iced::font::Weight::Medium)), t1(sub, 12.5, p.muted)].spacing(2.0)]
                    .spacing(14.0)
                    .align_y(iced::Center)
                    .padding(pad(0.0, 12.0, 0.0, 16.0));
                let tile = container(inner).width(Length::Fixed(tw)).height(Length::Fixed(54.0)).center_y(Length::Fixed(54.0)).style(ui::boxed(row_bg(&p), p.r(8.0), Some((p.line, 1.0))));
                r = r.push(mouse_area(tile).on_press(Msg::Quick(q.path.clone())).on_middle_press(Msg::QuickMiddle(q.path.clone())).interaction(mouse::Interaction::Pointer));
            }
            col = col.push(r).push(gap(1.0, 8.0));
        }
        col = col.push(gap(1.0, 12.0)).push(heading(&p, "Recent"));
        if a.entries.is_empty() {
            col = col.push(container(t(if a.query.is_empty() { "Files you open show up here." } else { "No recent file matches." }, 13.5, p.muted)).padding(pad(6.0, 0.0, 0.0, 4.0)));
        }
        let ctx = a.ctx();
        for (i, e) in a.entries.iter().enumerate() {
            let sel = a.marked.contains(&e.path) || (a.marked.is_empty() && i == a.cur && a.has_focus_sel());
            let where_ = drive::to_windows(&model::parent_of(&e.path), &ctx);
            let r = row![
                container(row![entry_icon(a, e, 16.0), name_cell(a, e, 14.0)].spacing(9.0).align_y(iced::Center)).width(Length::Fill),
                container(t1(model::fmt_date(e.mtime, true), 13.5, p.muted)).width(Length::Fixed(160.0)),
                container(text(where_).size(13.5).color(p.muted)).width(Length::Fixed(246.0)),
            ]
            .align_y(iced::Center)
            .padding(pad(4.0, 2.0, 4.0, 12.0));
            let bg = if sel { p.sel() } else { row_bg(&p) };
            let bx = container(r).width(Length::Fill).style(ui::boxed(bg, p.r(4.0), None));
            col = col.push(clickable(bx.into(), i)).push(gap(1.0, 2.0));
        }
        scroller(mouse_area(col).on_press(Msg::BgPress).into())
    })
    .into()
}

// ------------------------------------------------------------------ This PC (mockup files-5)
fn space_bar<'a>(p: &Pal, used: f32, w: f32) -> El<'a> {
    let c = if used > 0.95 { p.danger } else if used > 0.8 { p.warn } else { p.accent };
    container(ui::bar(vec![(used.clamp(0.0, 1.0), c)], Pal::mix(p.panel, p.line, 0.6), 6.0, 3.0)).width(Length::Fixed(w)).into()
}

fn this_pc(a: &App) -> El<'_> {
    let p = a.p;
    responsive(move |size: Size| {
        a.area.set((size.width, size.height));
        let per = 3usize;
        a.cols.set(per);
        let tw = ((size.width - 28.0 - 16.0) / per as f32).floor();
        let mut col = Column::new().padding(pad(4.0, 14.0, 12.0, 14.0));
        let groups = [("drive", "Drives"), ("net", "Network locations")];
        for (k, title) in groups {
            let list: Vec<(usize, &Entry)> = a.entries.iter().enumerate().filter(|(_, e)| e.pc == k).collect();
            if list.is_empty() {
                continue;
            }
            col = col.push(heading(&p, title));
            if a.pc_view == "details" {
                for (i, e) in list {
                    let d = &e.item;
                    let total = d.get("total").and_then(Value::as_u64).unwrap_or(0);
                    let free = d.get("free").and_then(Value::as_u64);
                    let r = row![
                        container(row![ico(pc_icon(e), p.text, 18.0), t1(e.name.clone(), 14.0, p.text)].spacing(10.0).align_y(iced::Center)).width(Length::Fill),
                        container(t1(if k == "net" { "Network location".to_string() } else if sv(d, "key") == "system" { "Local disk".into() } else if d.get("removable") == Some(&Value::Bool(true)) { "Removable disk".into() } else { "Local disk".into() }, 13.5, p.muted)).width(Length::Fixed(170.0)),
                        container(t1(if total > 0 { model::fmt_space(total) } else { String::new() }, 13.5, p.muted)).width(Length::Fixed(110.0)),
                        container(t1(free.map(model::fmt_space).unwrap_or_default(), 13.5, p.muted)).width(Length::Fixed(120.0)),
                    ]
                    .align_y(iced::Center)
                    .padding(pad(0.0, 4.0, 0.0, 12.0));
                    let sel = i == a.cur && a.has_focus_sel();
                    col = col.push(clickable(row_box(a, r.into(), sel, i == a.cur, 30.0), i)).push(gap(1.0, 2.0));
                }
                continue;
            }
            for chunk in list.chunks(per) {
                let mut r = Row::new().spacing(10.0);
                for (i, e) in chunk {
                    r = r.push(pc_tile(a, e, *i, tw));
                }
                col = col.push(r).push(gap(1.0, 10.0));
            }
        }
        if a.entries.is_empty() {
            col = col.push(container(t(if a.query.is_empty() { "Looking at your drives…".to_string() } else { format!("Nothing here matches “{}”", a.query) }, 14.0, p.muted)).padding(20.0));
        }
        scroller(mouse_area(col).on_press(Msg::BgPress).on_right_press(Msg::BgRight).into())
    })
    .into()
}

fn pc_icon(e: &Entry) -> Ic {
    let d = &e.item;
    if e.pc == "net" {
        let k = sv(d, "kind");
        if k == "device" || k == "phone" || sv(d, "sub").contains("Hearth Link") { Ic::Phone } else { Ic::Server }
    } else if sv(d, "kind") == "optical" {
        Ic::Disc
    } else if d.get("removable") == Some(&Value::Bool(true)) || matches!(sv(d, "kind").as_str(), "usb" | "sd" | "removable") {
        Ic::Usb
    } else {
        Ic::Disc
    }
}

fn pc_tile<'a>(a: &'a App, e: &Entry, i: usize, w: f32) -> El<'a> {
    let p = a.p;
    let d = &e.item;
    let total = d.get("total").and_then(Value::as_u64).unwrap_or(0);
    let free = d.get("free").and_then(Value::as_u64);
    let mut info = Column::new().spacing(6.0).push(t1(e.name.clone(), 15.0, p.text));
    let line = if d.get("mounted") == Some(&Value::Bool(false)) {
        if sv(d, "sub").is_empty() { "Not mounted".to_string() } else { sv(d, "sub") }
    } else if let (Some(f), true) = (free, total > 0) {
        let host = if e.pc == "net" { sv(d, "host") } else { String::new() };
        let host = if host.is_empty() && e.pc == "net" { sv(d, "sub") } else { host };
        format!("{} free of {}{}", model::fmt_space(f), model::fmt_space(total), if host.is_empty() { String::new() } else { format!(" · {host}") })
    } else {
        sv(d, "sub")
    };
    if let (Some(f), true) = (free, total > 0) {
        let used = 1.0 - f as f32 / total as f32;
        info = info.push(space_bar(&p, used, (w - 110.0).max(80.0)));
    }
    info = info.push(t1(line, 13.0, p.muted));
    let sel = i == a.cur && a.has_focus_sel();
    let inner = row![ico(pc_icon(e), p.text, 40.0), info].spacing(16.0).align_y(iced::Center).padding(pad(0.0, 14.0, 0.0, 18.0));
    let bg = if sel { p.sel() } else { row_bg(&p) };
    let bx = container(inner).width(Length::Fixed(w)).height(Length::Fixed(72.0)).center_y(Length::Fixed(72.0)).style(ui::boxed(bg, p.r(8.0), Some((if sel { p.accent } else { p.line }, 1.0))));
    clickable(bx.into(), i)
}

// ------------------------------------------------------------------ Hearth (C:)'s pages (FilesCDrive.qml)
fn c_page(a: &App) -> El<'_> {
    let p = a.p;
    let pg = a.c_page();
    let banner = match pg {
        "root" => "This is Hearth’s drive with Windows folder names. Nothing has been moved: each folder shows where its files really are.",
        "pf" => "Every installed app in one place. These are names only: Flatpaks, Hearth’s apps and games stay where they are.",
        "x86" => "Windows apps and Steam, named the way Windows names them. Each Windows app still keeps its own space.",
        _ => "Where apps keep their settings and data, like on Windows: Roaming is ~/.config, Local is ~/.local/share.",
    };
    let mut top = Column::new().spacing(8.0).padding(pad(10.0, 14.0, 4.0, 14.0));
    let mut br = Row::new().spacing(12.0).align_y(iced::Center).push(ico(Ic::Info, p.accent, 18.0)).push(container(t(banner, 13.0, p.text)).width(Length::Fill));
    if pg == "root" {
        br = br.push(ui::check(&p, a.c_linux(), "Show the Linux folders too", false, Msg::Cmd("clinux")));
    }
    top = top.push(container(br).padding(10.0).width(Length::Fill).style(ui::boxed(Pal::mix(p.panel, p.accent, 0.12), p.r(8.0), Some((Pal::a(p.accent, 0.4), 1.0)))));
    let header = details_header(a, &[("name", "Name", 0.0), ("", "Type", 210.0), ("", "Size", 90.0), ("", "On Linux", 260.0)]);
    let rows = responsive(move |size: Size| {
        a.area.set((size.width, size.height));
        a.cols.set(1);
        let mut col = Column::new().spacing(2.0).padding(pad(4.0, 12.0, 8.0, 12.0));
        for (i, e) in a.entries.iter().enumerate() {
            let cv = e.cv.clone().unwrap_or_default();
            if !cv.group.is_empty() {
                col = col.push(heading(&p, &cv.group));
            }
            let sel = i == a.cur && a.has_focus_sel();
            let r = row![
                container(row![entry_icon(a, e, 16.0), name_cell(a, e, 14.0)].spacing(9.0).align_y(iced::Center)).width(Length::Fill),
                container(t1(cv.kind_text.clone(), 13.5, p.muted)).width(Length::Fixed(210.0)),
                container(t1(if cv.size > 0 { model::fmt_size(cv.size) } else { String::new() }, 13.5, p.muted)).width(Length::Fixed(90.0)),
                container(t1(ui::fit(&cv.linux, 36), 13.5, p.muted)).width(Length::Fixed(260.0)),
            ]
            .align_y(iced::Center)
            .padding(pad(0.0, 2.0, 0.0, 12.0));
            col = col.push(clickable(row_box(a, r.into(), sel, i == a.cur, ROW_H), i));
        }
        if a.entries.is_empty() {
            col = col.push(container(t(if a.query.is_empty() { "Looking…".to_string() } else { format!("Nothing here matches “{}”", a.query) }, 14.0, p.muted)).padding(16.0));
        }
        scroller(mouse_area(col).on_press(Msg::BgPress).on_right_press(Msg::BgRight).into())
    });
    column![top, header, rows].into()
}

// ------------------------------------------------------------------ Installed apps (FilesApps.qml)
fn apps_page(a: &App) -> El<'_> {
    let p = a.p;
    let head = column![
        ts("Installed apps", 26.0, p.text),
        t(if a.apps_loading && a.app_list.is_empty() { "Looking at what's installed…".to_string() } else { format!("Everything on this PC: Hearth, Flatpak, Windows apps, Steam games, Hearth Games and Recomps. {}", model::plural(a.app_list.len(), "app", "apps")) }, 13.0, p.muted),
    ]
    .spacing(4.0)
    .padding(pad(14.0, 18.0, 10.0, 18.0));
    let list = responsive(move |size: Size| {
        a.area.set((size.width, size.height));
        a.cols.set(1);
        let mut col = Column::new().spacing(4.0).padding(pad(0.0, 14.0, 10.0, 14.0));
        for (i, e) in a.entries.iter().enumerate() {
            let ap = e.app.clone().unwrap_or(Value::Null);
            let kind = sv(&ap, "kind");
            let tag = match kind.as_str() {
                "hearth" => "Hearth",
                "flatpak" => "Flatpak",
                "winapp" => "Windows app",
                "steam" => "Steam",
                "recomp" => "Recomp",
                "games" => "Hearth Games",
                _ => "App",
            };
            let letter = e.name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_else(|| "?".into());
            let av = container(tb(letter, 15.0, Color::WHITE)).center_x(Length::Fixed(34.0)).center_y(Length::Fixed(34.0)).style(ui::boxed(Pal::mix(p.accent, Color::BLACK, 0.25), p.r(8.0), None));
            let sub = { let s = sv(&ap, "publisher"); if s.is_empty() { sv(&ap, "sub") } else { s } };
            let r = row![
                av,
                column![row![t1(e.name.clone(), 14.0, p.text), ui::chip(tag, Pal::mix(p.panel, p.accent, 0.25), p.text)].spacing(8.0).align_y(iced::Center), t1(sub, 12.0, p.muted)].spacing(2.0).width(Length::Fill),
                t1(if e.size > 0 { model::fmt_size(e.size) } else { String::new() }, 13.0, p.muted),
                ico(Ic::Dots, p.text, 16.0),
            ]
            .spacing(12.0)
            .align_y(iced::Center)
            .padding(pad(0.0, 12.0, 0.0, 12.0));
            let sel = i == a.cur && a.has_focus_sel();
            col = col.push(clickable(row_box(a, r.into(), sel, i == a.cur, 48.0), i));
        }
        scroller(col.into())
    });
    column![head, list].into()
}

// ------------------------------------------------------------------ the details pane (mockup files-6)
fn details_pane(a: &App) -> El<'_> {
    let p = a.p;
    // Describe it's "preview right, bigger" (board 11)
    let w = if a.describe.knob("preview") == "right-big" { 420.0 } else { 262.0 };
    let mut col = Column::new().spacing(10.0).padding(pad(14.0, 14.0, 14.0, 14.0)).width(Length::Fixed(w));
    let sel: Vec<&Entry> = a.entries.iter().filter(|e| a.marked.contains(&e.path)).collect();
    if sel.len() > 1 {
        let total: u64 = sel.iter().map(|e| e.size).sum();
        col = col.push(container(ico::<Msg>(Ic::Copy, p.muted, 60.0)).center_x(Length::Fill).height(Length::Fixed(150.0)).center_y(Length::Fixed(150.0)));
        col = col.push(tb(format!("{} items selected", sel.len()), 15.0, p.text));
        col = col.push(t(model::fmt_size(total), 13.0, p.muted));
        return container(col).height(Length::Fill).into();
    }
    let Some(e) = a.current() else {
        col = col.push(t("Pick something to see its details.", 13.0, p.muted));
        // In a Briefcase: its note, lock, sync and items.
        let mut grid = Column::new().spacing(6.0);
        for (k, v) in crate::briefcase::detail_rows(a, None) {
            grid = grid.push(row![t1(k, 13.5, p.muted), Space::new().width(Length::Fill), t1(ui::fit(&v, 30), 13.5, p.text)].align_y(iced::Center));
        }
        col = col.push(grid);
        return container(col).height(Length::Fill).into();
    };
    let th = a.thumbs.get(&e.path);
    let pic: El = match th.and_then(|t| t.1.as_ref()) {
        Some(h) => img(h).width(Length::Fill).height(Length::Fixed(150.0)).content_fit(ContentFit::Cover).border_radius(p.r(8.0)).into(),
        None => container(entry_icon(a, e, 72.0)).center_x(Length::Fill).height(Length::Fixed(150.0)).center_y(Length::Fixed(150.0)).style(ui::boxed(row_bg(&p), p.r(8.0), None)).into(),
    };
    col = col.push(pic);
    col = col.push(column![tb(ui::fit(&e.name, 30), 15.0, p.text), t1(format!("{}{}", model::kind_name(e), if e.dir { String::new() } else { format!(" · {}", model::fmt_size(e.size)) }), 13.0, p.muted)].spacing(3.0));
    let mut rows: Vec<(String, String)> = vec![];
    let kind = model::kind_of(e);
    if let Some(t) = th.filter(|t| t.2 > 0) {
        rows.push(("Size".into(), format!("{} × {}", t.2, t.3)));
    } else if !e.dir && e.cv.is_none() {
        rows.push(("Size".into(), model::fmt_size(e.size)));
    }
    let taken = th.map(|t| t.4.clone()).unwrap_or_default();
    if kind == "image" && !taken.is_empty() {
        rows.push(("Taken".into(), taken));
    } else if kind == "image" && e.mtime > 0 {
        rows.push(("Taken".into(), model::fmt_date(e.mtime, false)));
    } else if e.mtime > 0 {
        rows.push(("Modified".into(), model::fmt_date(e.mtime, false)));
    }
    if e.path.starts_with('/') {
        let w = a.win_path(&model::parent_of(&e.path));
        rows.push(("Where".into(), w));
    } else if let Some(cv) = &e.cv {
        rows.push(("On Linux".into(), cv.linux.clone()));
    }
    if !e.tags.is_empty() {
        rows.push(("Tags".into(), e.tags.join(", ")));
    }
    let gs = crate::extras::git_of(a, e);
    if !gs.is_empty() {
        rows.push(("Git".into(), format!("{}{}", crate::extras::git_mark(gs).1, if a.fx.git_branch.is_empty() { String::new() } else { format!(" · {}", a.fx.git_branch) })));
    }
    if a.versions_of == e.path {
        rows.push(("Previous versions".into(), a.versions.len().to_string()));
    }
    rows.extend(crate::briefcase::detail_rows(a, Some(e)));
    let mut grid = Column::new().spacing(6.0);
    for (k, v) in rows {
        grid = grid.push(row![t1(k, 13.5, p.muted), Space::new().width(Length::Fill), t1(ui::fit(&v, 30), 13.5, p.text)].align_y(iced::Center));
    }
    col = col.push(grid);
    if e.path.starts_with('/') && !e.dir {
        let link = |s: &'static str, m: Msg| -> El<'static> { mouse_area(t1(s, 13.5, p.accent)).on_press(m).interaction(mouse::Interaction::Pointer).into() };
        let mut lr = Row::new().spacing(6.0).align_y(iced::Center);
        if kind == "image" {
            lr = lr.push(link("Set as wallpaper", Msg::Wallpaper)).push(t1("·", 13.5, p.muted));
        }
        lr = lr.push(link("Open with ▾", Msg::OpenWithMenu));
        col = col.push(lr);
    }
    container(scrollable(col)).height(Length::Fill).width(Length::Fixed(w)).into()
}

// ------------------------------------------------------------------ the bottom
fn status_bar(a: &App) -> El<'_> {
    let p = a.p;
    let n = a.entries.len();
    let left = if !a.note.is_empty() {
        t1(a.note.clone(), 13.0, if a.note_err { p.danger } else { p.text })
    } else if crate::sortdl::here(a) && a.marked.is_empty() && a.search.is_none() {
        // Downloads (Sort my Downloads' mockups): what's loose, or the folders and "nothing loose".
        let loose = crate::sortdl::loose(a);
        if loose > 0 {
            t1(format!("{} loose in Downloads", model::plural(loose, "item", "items")), 13.0, p.muted)
        } else {
            t1(format!("{} · nothing loose", model::plural(n, "folder", "folders")), 13.0, p.muted)
        }
    } else {
        let mut s = model::plural(n, "item", "items");
        let sel: Vec<&Entry> = a.entries.iter().filter(|e| a.marked.contains(&e.path)).collect();
        if !sel.is_empty() {
            let total: u64 = sel.iter().filter(|e| !e.dir).map(|e| e.size).sum();
            s = format!("{s} · {} selected{}", sel.len(), if total > 0 { format!(", {}", model::fmt_size(total)) } else { String::new() });
        }
        if a.search.is_some() {
            s = format!("{s} · searching…");
        }
        if let Some(l) = &a.listing {
            if !l.writable && l.error.is_empty() && !a.on_special() && !a.on_home() && a.path.starts_with('/') {
                s = format!("{s}{}", if crate::extras::admin_here(a) { " · as administrator" } else { " · read-only" });
            }
        }
        t1(s, 13.0, p.muted)
    };
    // The drive you're on, the Windows way: "C: 96 GB free of 238 GB".
    let (free, total) = match &a.listing {
        Some(l) if l.free.is_some() && l.total.is_some() => (l.free, l.total),
        _ => {
            let h = a.raw_places.iter().find(|q| sv(q, "id") == "home");
            (h.and_then(|q| q.get("free").and_then(Value::as_u64)), h.and_then(|q| q.get("total").and_then(Value::as_u64)))
        }
    };
    let letter = drive::drive_for(&a.path, &a.ctx()).map(|d| d.letter.clone()).unwrap_or_else(|| "C".into());
    let right = match (free, total) {
        (Some(f), Some(t)) if t > 0 => format!("{letter}: {} free of {}", model::fmt_space(f), model::fmt_space(t)),
        _ => String::new(),
    };
    // The branch of a git repository, a Briefcase's lock and sync (FileManager.qml's status bar).
    let mut extra = Row::new().spacing(14.0).align_y(iced::Center);
    let gl = crate::extras::git_line(a);
    if !gl.is_empty() {
        extra = extra.push(t1(gl, 13.0, p.muted));
    }
    if crate::briefcase::in_briefcase(a) {
        let bl: Vec<String> = [crate::briefcase::lock_line(a), crate::briefcase::sync_line(a)].into_iter().filter(|x| !x.is_empty()).collect();
        if !bl.is_empty() {
            extra = extra.push(t1(bl.join(" · "), 13.0, p.muted));
        }
    }
    container(row![left, Space::new().width(Length::Fill), extra, gap(14.0, 1.0), t1(right, 13.0, p.muted)].align_y(iced::Center))
        .padding(pad(0.0, 16.0, 0.0, 16.0))
        .height(Length::Fixed(26.0))
        .center_y(Length::Fixed(26.0))
        .width(Length::Fill)
        .style(ui::boxed(Pal::mix(p.bg, Color::BLACK, if p.dark { 0.15 } else { 0.03 }), 0.0, None))
        .into()
}

fn job_bar<'a>(a: &'a App, j: &'a Job) -> El<'a> {
    let p = a.p;
    let verb = match j.req.verb.as_str() {
        "copy" => "Copying",
        "move" => "Moving",
        "compress" => "Compressing",
        "extract" => "Unpacking",
        _ => "Working on",
    };
    let what = if j.name.is_empty() { model::plural(j.req.paths.len(), "item", "items") } else { format!("“{}”", j.name) };
    let frac = if j.total > 0.0 { (j.done / j.total) as f32 } else { 0.0 };
    let mut line = format!("{verb} {what} to “{}”", j.dest_name);
    if j.total > 0.0 {
        line = format!("{line} · {}%", (frac * 100.0).round());
    }
    if j.speed > 0.0 {
        line = format!("{line} · {}/s", model::fmt_size(j.speed as u64));
    }
    container(
        row![
            column![t1(line, 13.0, p.text), container(ui::bar(vec![(frac, p.accent)], Pal::mix(p.panel, p.line, 0.6), 4.0, 2.0)).width(Length::Fill)].spacing(5.0).width(Length::Fill),
            btn(&p, "Stop", Kind::Normal, false, Some(Msg::StopJob)),
        ]
        .spacing(14.0)
        .align_y(iced::Center),
    )
    .padding(pad(8.0, 16.0, 8.0, 16.0))
    .into()
}

// ------------------------------------------------------------------ the Open / Save window (mockup cdrive-1)
fn pick_head(a: &App) -> El<'_> {
    let p = a.p;
    let title = a.pick.as_ref().map(|k| k.title.clone()).unwrap_or_default();
    let drag = mouse_area(container(tb(title, 17.0, p.text)).padding(pad(0.0, 18.0, 0.0, 22.0)).center_y(Length::Fixed(56.0))).on_press(Msg::Drag).on_double_click(Msg::Maximize);
    container(row![drag, container(address(a, 34.0)).width(Length::Fill).padding(pad(0.0, 14.0, 0.0, 0.0))].align_y(iced::Center))
        .height(Length::Fixed(60.0))
        .center_y(Length::Fixed(60.0))
        .width(Length::Fill)
        .style(ui::boxed(Pal::mix(p.bg, p.panel, 0.5), 0.0, None))
        .into()
}

fn pick_bar(a: &App) -> El<'_> {
    let p = a.p;
    let Some(pk) = &a.pick else { return Space::new().into() };
    let mut r = Row::new().spacing(12.0).align_y(iced::Center);
    if pk.kind == "save" {
        r = r.push(t1("File name", 15.0, p.text));
        r = r.push(text_input("", &a.pick_name).id(PICKNAME_ID).on_input(Msg::PickName).on_submit(Msg::PickAccept).size(15.0).padding(pad(8.0, 12.0, 8.0, 12.0)).style(input_style(p, true)).width(Length::Fill));
    } else if pk.kind == "savefiles" {
        r = r.push(container(t1(format!("{} go into “{}”", model::plural(pk.files.len(), "file", "files"), a.listing.as_ref().map(|l| l.name.clone()).unwrap_or_default()), 14.0, p.muted)).width(Length::Fill));
    } else {
        let t_ = a.pick_targets();
        let shown = t_.iter().map(|x| base(x)).collect::<Vec<_>>().join(", ");
        r = r.push(t1("File name", 15.0, p.text));
        r = r.push(container(t1(ui::fit(&shown, 80), 15.0, p.text)).padding(pad(8.0, 12.0, 8.0, 12.0)).width(Length::Fill).style(ui::boxed(field_bg(&p), p.r(6.0), Some((p.line, 1.0)))));
    }
    if !pk.filters.is_empty() {
        let name = pk.filters.get(a.pick_filter.max(0) as usize).map(|f| f.0.clone()).unwrap_or_default();
        r = r.push(flat(&p, row![t1(ui::fit(&name, 30), 14.0, p.text), ico(Ic::ChevDown, p.text, 9.0)].spacing(8.0).align_y(iced::Center).into(), if pk.filters.len() > 1 { Some(Msg::PickFilter) } else { None }, pad(8.0, 12.0, 8.0, 12.0)));
    }
    for (i, c) in pk.choices.iter().enumerate() {
        let v = a.pick_choice.get(&c.0).and_then(Value::as_str).unwrap_or("").to_string();
        let label = if c.2.is_empty() { format!("{} {}", if v == "true" { "☑" } else { "☐" }, c.1) } else { format!("{}: {}  ▾", c.1, c.2.iter().find(|o| o.0 == v).map(|o| o.1.clone()).unwrap_or(v)) };
        r = r.push(flat(&p, t1(label, 13.5, p.text).into(), Some(Msg::PickChoice(i)), pad(8.0, 10.0, 8.0, 10.0)));
    }
    r = r.push(big_btn(&p, "Cancel", false, Some(Msg::PickCancel)));
    let ok_on = pk.kind != "open" || !a.pick_targets().is_empty();
    r = r.push(big_btn(&p, &a.pick_accept_label(), true, if ok_on { Some(Msg::PickAccept) } else { None }));
    container(r).padding(pad(12.0, 22.0, 12.0, 22.0)).width(Length::Fill).style(ui::boxed(Pal::mix(p.bg, p.panel, 0.5), 0.0, None)).into()
}

fn big_btn<'a>(p: &Pal, label: &str, primary: bool, msg: Option<Msg>) -> El<'a> {
    let p = *p;
    let on = msg.is_some();
    let fg = if primary { if on { p.ink } else { p.muted } } else { p.text };
    button(container(t1(label.to_string(), 15.0, fg)).center_y(Length::Fixed(22.0)))
        .padding(pad(8.0, 22.0, 8.0, 22.0))
        .on_press_maybe(msg)
        .style(move |_: &Theme, st: button::Status| {
            let hover = matches!(st, button::Status::Hovered | button::Status::Pressed);
            let bg = if primary { if on { if hover { Pal::mix(p.accent, Color::WHITE, 0.12) } else { p.accent } } else { p.line } } else if hover { p.hover() } else { Pal::mix(p.bg, p.panel, 0.7) };
            button::Style { background: Some(Background::Color(bg)), text_color: fg, border: Border { color: if primary { Color::TRANSPARENT } else { p.line }, width: if primary { 0.0 } else { 1.0 }, radius: p.r(8.0).into() }, shadow: Shadow::default(), snap: true }
        })
        .into()
}

// ------------------------------------------------------------------ menus
fn menu<'a>(a: &'a App, m: &'a Menu) -> El<'a> {
    let p = a.p;
    let mut col = Column::new().spacing(0.0);
    if m.short {
        // Windows 11's icon row: Cut, Copy, Rename, Copy as path, Delete.
        let mut q = Row::new().spacing(0.0).align_y(iced::Center);
        for (i, it) in m.items.iter().enumerate().filter(|(_, it)| it.quick) {
            q = q.push(container(flat(&p, ico(it.icon.unwrap_or(Ic::Dots), p.text, 17.0), Some(Msg::MenuPick(i)), pad(8.0, 10.0, 8.0, 10.0))).center_x(Length::FillPortion(1)));
        }
        col = col.push(container(q).width(Length::Fill).padding(pad(6.0, 4.0, 4.0, 4.0)));
    } else if !m.items.first().map(|i| i.heading).unwrap_or(false) {
        col = col.push(container(t1(ui::fit(&m.title, 40), 12.0, p.muted)).padding(pad(6.0, 12.0, 6.0, 12.0)));
    }
    let any_check = m.items.iter().any(|i| i.check.is_some());
    for (i, it) in m.items.iter().enumerate() {
        if it.quick && m.short {
            continue;
        }
        if it.sep {
            col = col.push(container(hline::<Msg>(p.line)).padding(pad(4.0, 6.0, 4.0, 6.0)));
            continue;
        }
        if it.heading {
            col = col.push(container(text(it.label.clone()).size(11.5).color(p.muted).font(ui::weight(iced::font::Weight::Medium))).padding(pad(8.0, 12.0, 4.0, 12.0)));
            continue;
        }
        let fg = if it.danger { p.danger } else { p.text };
        let mut r = Row::new().spacing(8.0).align_y(iced::Center);
        if any_check {
            r = r.push(container(if it.check == Some(true) { ico(Ic::Check, p.accent, 13.0) } else { gap(13.0, 13.0) }).width(Length::Fixed(16.0)));
        }
        if let Some(c) = it.dot {
            r = r.push(container(Space::new().width(Length::Fixed(10.0)).height(Length::Fixed(10.0))).style(ui::boxed(c, 5.0, None)).center_x(Length::Fixed(16.0)).center_y(Length::Fixed(16.0)));
        } else if let Some(ic) = it.icon {
            r = r.push(ico(ic, p.text, 16.0));
        } else if !any_check {
            r = r.push(gap(16.0, 16.0));
        }
        let mut lc = Column::new().push(t1(it.label.clone(), 14.0, fg));
        if !it.sub.is_empty() {
            lc = lc.push(t1(ui::fit(&it.sub, 44), 11.5, p.muted));
        }
        r = r.push(lc).push(Space::new().width(Length::Fill));
        if !it.key.is_empty() {
            r = r.push(t1(it.key.clone(), 12.5, p.muted));
        }
        let lit = i == m.sel;
        let bx = container(r).padding(pad(6.0, 12.0, 6.0, 10.0)).width(Length::Fill).style(ui::boxed(if lit { p.sel() } else { Color::TRANSPARENT }, p.r(6.0), None));
        col = col.push(mouse_area(bx).on_press(Msg::MenuPick(i)).interaction(mouse::Interaction::Pointer));
    }
    container(col)
        .padding(5.0)
        .width(Length::Fixed(m.width))
        .style(move |_t: &Theme| container::Style {
            background: Some(Background::Color(p.panel)),
            border: Border { color: p.line, width: 1.0, radius: p.r(10.0).into() },
            shadow: Shadow { color: Color { a: 0.45, ..Color::BLACK }, offset: iced::Vector::new(0.0, 6.0), blur_radius: 18.0 },
            ..container::Style::default()
        })
        .into()
}

// ------------------------------------------------------------------ dialogs
fn buttons<'a>(list: Vec<El<'a>>) -> El<'a> {
    let mut r = Row::new().spacing(8.0);
    for b in list {
        r = r.push(b);
    }
    container(r).align_right(Length::Fill).into()
}

fn dialog<'a>(a: &'a App, ov: &'a Overlay) -> El<'a> {
    let p = a.p;
    match ov {
        Overlay::Input(i) => {
            let col = column![
                ts(i.title.clone(), 17.0, p.text),
                text_input("", &i.text).id(DLG_ID).on_input(Msg::DlgText).on_submit(Msg::DlgOk).secure(i.secret).size(14.0).padding(9.0).style(input_style(p, true)),
                buttons(vec![btn(&p, "Cancel", Kind::Normal, false, Some(Msg::DlgCancel)), btn(&p, i.ok.clone(), Kind::Primary, false, Some(Msg::DlgOk))]),
            ]
            .spacing(14.0);
            ui::dialog_box(&p, 460.0, col)
        }
        Overlay::Confirm(c) => {
            let col = column![
                ts(c.title.clone(), 17.0, p.text),
                t(c.text.clone(), 13.5, p.muted),
                buttons({
                    let mut b = vec![btn(&p, "Cancel", Kind::Normal, c.button == 0, Some(Msg::DlgCancel))];
                    if let Some((label, _)) = &c.alt {
                        b.push(btn(&p, label.clone(), Kind::Normal, c.button == 2, Some(Msg::DlgAlt)));
                    }
                    b.push(btn(&p, c.ok.clone(), if c.danger { Kind::Danger } else { Kind::Primary }, c.button == 1, Some(Msg::DlgOk)));
                    b
                }),
            ]
            .spacing(14.0);
            ui::dialog_box(&p, 480.0, col)
        }
        Overlay::Apps(d) => {
            let mut col = Column::new().spacing(6.0).push(ts(d.title.clone(), 17.0, p.text)).push(gap(1.0, 6.0));
            if !d.loaded {
                col = col.push(t("Looking for apps…", 13.5, p.muted));
            } else if d.apps.is_empty() {
                col = col.push(t("No installed app says it can open this kind of file. Try Other app…", 13.5, p.muted));
            }
            let mut list = Column::new().spacing(2.0);
            for (i, ap) in d.apps.iter().enumerate() {
                let mut r = Row::new().spacing(10.0).align_y(iced::Center).push(ico(Ic::OpenWith, p.text, 16.0)).push(t1(sv(ap, "name"), 14.0, p.text));
                if ap.get("default").and_then(Value::as_bool) == Some(true) {
                    r = r.push(ui::chip("Default", Pal::mix(p.panel, p.accent, 0.3), p.accent));
                }
                let bx = container(r).padding(pad(7.0, 10.0, 7.0, 10.0)).width(Length::Fill).style(ui::boxed(if i == d.sel { p.sel() } else { Color::TRANSPARENT }, p.r(6.0), None));
                list = list.push(mouse_area(bx).on_press(Msg::OpenWith(i)).interaction(mouse::Interaction::Pointer));
            }
            let other = container(row![ico(Ic::Dots, p.text, 16.0), t1(if d.all { "Only the suggested apps" } else { "Other app…" }, 14.0, p.text)].spacing(10.0).align_y(iced::Center))
                .padding(pad(7.0, 10.0, 7.0, 10.0))
                .width(Length::Fill)
                .style(ui::boxed(if d.sel >= d.apps.len() { p.sel() } else { Color::TRANSPARENT }, p.r(6.0), None));
            list = list.push(mouse_area(other).on_press(Msg::OtherApps).interaction(mouse::Interaction::Pointer));
            col = col.push(container(scrollable(list)).max_height(360.0));
            col = col.push(buttons(vec![btn(&p, "Close", Kind::Normal, false, Some(Msg::DlgCancel))]));
            ui::dialog_box(&p, 460.0, col)
        }
        Overlay::Conflict(c) => {
            let x = c.list.get(c.i).cloned().unwrap_or(Value::Null);
            let td = x.get("targetDir").and_then(Value::as_bool).unwrap_or(false);
            let sd = x.get("srcDir").and_then(Value::as_bool).unwrap_or(false);
            let line = |dir: bool, size: &str, mt: &str| -> String {
                format!("{} · modified {}", if dir { "Folder".to_string() } else { model::fmt_size(x.get(size).and_then(Value::as_u64).unwrap_or(0)) }, model::fmt_date(x.get(mt).and_then(Value::as_i64).unwrap_or(0), true))
            };
            let many = c.list.len() - c.i > 1;
            let mut col = column![
                ts(format!("“{}” is already in “{}”", sv(&x, "name"), base(&c.dest)), 17.0, p.text),
                t(if td && sd { "Merging puts what's in the new folder into the one that's there (files with the same names are replaced)." } else { "Replace it, skip it, or keep both (the new one gets a number)." }, 13.0, p.muted),
                row![t1("There now", 13.0, p.muted).width(Length::Fixed(90.0)), t1(line(td, "targetSize", "targetMtime"), 13.0, p.text)].spacing(10.0),
                row![t1("New one", 13.0, p.muted).width(Length::Fixed(90.0)), t1(line(sd, "srcSize", "srcMtime"), 13.0, p.text)].spacing(10.0),
            ]
            .spacing(10.0);
            if many {
                col = col.push(ui::check(&p, c.all, format!("Do the same for the other {} with names that are there", c.list.len() - c.i - 1), c.button == 3, Msg::ConflictAll));
            }
            col = col.push(buttons(vec![
                btn(&p, "Skip", Kind::Normal, c.button == 0, Some(Msg::ConflictPick("skip"))),
                btn(&p, "Keep both", Kind::Normal, c.button == 1, Some(Msg::ConflictPick("rename"))),
                btn(&p, if td && sd { "Merge" } else { "Replace" }, Kind::Primary, c.button == 2, Some(Msg::ConflictPick("replace"))),
            ]));
            ui::dialog_box(&p, 520.0, col)
        }
        Overlay::Preview(pv) => {
            let e = &pv.e;
            let body: El = if let Some(h) = a.thumbs.get(&e.path).and_then(|t| t.1.as_ref()) {
                img(h).width(Length::Fill).height(Length::Fixed(360.0)).content_fit(ContentFit::Contain).into()
            } else if !pv.text.is_empty() {
                container(scrollable(text(pv.text.clone()).size(12.5).color(p.text).font(iced::Font::MONOSPACE))).height(Length::Fixed(360.0)).padding(10.0).style(ui::boxed(field_bg(&p), p.r(6.0), None)).into()
            } else {
                container(entry_icon(a, e, 96.0)).center_x(Length::Fill).height(Length::Fixed(220.0)).center_y(Length::Fixed(220.0)).into()
            };
            let col = column![
                body,
                row![entry_icon(a, e, 20.0), column![ts(e.name.clone(), 15.0, p.text), t1(format!("{}{} · {}", model::kind_name(e), if e.dir { String::new() } else { format!(" · {}", model::fmt_size(e.size)) }, model::fmt_date(e.mtime, true)), 12.5, p.muted)].spacing(2.0)].spacing(10.0).align_y(iced::Center),
                t1("← → previous / next · Enter opens · Space or Esc closes", 12.0, p.muted),
            ]
            .spacing(12.0);
            ui::dialog_box(&p, 640.0, col)
        }
        _ => Space::new().into(),
    }
}

fn prow<'a>(p: &Pal, k: &str, v: El<'a>) -> El<'a> {
    row![container(t1(k.to_string(), 14.0, p.muted)).width(Length::Fixed(196.0)), v].align_y(iced::Center).into()
}

fn ptext<'a>(p: &Pal, v: String) -> El<'a> {
    t1(v, 14.0, p.text).into()
}

/// Properties (mockup cdrive-3, FileProperties.qml): General with the C:\ location and the Linux path, Permissions,
/// Checksums.
fn props_dialog<'a>(a: &'a App, pr: &'a Props) -> El<'a> {
    let p = a.p;
    let title = if pr.paths.len() > 1 { format!("{} Properties", model::plural(pr.paths.len(), "item", "items")) } else { format!("{} Properties", pr.name) };
    let head = container(tb(title, 17.0, p.text)).padding(pad(18.0, 24.0, 16.0, 24.0));
    let mut body = Column::new().spacing(12.0).padding(pad(18.0, 28.0, 20.0, 28.0));
    let info = &pr.info;
    if let Some((e, win)) = &pr.cview {
        let cv = e.cv.clone().unwrap_or_default();
        body = body.push(prow(&p, "Type", ptext(&p, if cv.kind_text.is_empty() { "Folder".into() } else { cv.kind_text.clone() })));
        body = body.push(prow(&p, "Location", ptext(&p, win.clone())));
        if cv.size > 0 {
            body = body.push(prow(&p, "Size", ptext(&p, model::fmt_size(cv.size))));
        }
        for (i, r) in cv.real.iter().enumerate() {
            body = body.push(prow(&p, if i == 0 { "Where it really is" } else { "" }, ptext(&p, r.clone())));
        }
        if !cv.note.is_empty() {
            body = body.push(t(cv.note.clone(), 13.0, p.muted));
        }
        body = body.push(copy_btn(&p, win.clone(), Msg::PropsCopy(false)));
        body = body.push(buttons(vec![btn(&p, "OK", Kind::Primary, false, Some(Msg::DlgCancel))]));
        return container(column![head, hline::<Msg>(p.line), body]).width(Length::Fixed(620.0)).style(ui::boxed(p.panel, p.r(12.0), Some((p.line, 1.0)))).into();
    }
    if !pr.loaded {
        body = body.push(t("Looking…", 13.5, p.muted));
    } else if !pr.error.is_empty() {
        body = body.push(t(pr.error.clone(), 13.5, p.danger));
    } else {
        let single = pr.paths.len() == 1;
        let is_file = single && info.get("dir").and_then(Value::as_bool) != Some(true);
        let tabs = props_tabs(pr);
        let mut tr = Row::new().spacing(4.0);
        let sel_tab = if pr.tab == 99 { tabs.iter().position(|t| *t == "Sharing").unwrap_or(0) } else { pr.tab };
        for (i, tname) in tabs.iter().enumerate() {
            let on = i == sel_tab;
            let c = if on { p.text } else { p.muted };
            tr = tr.push(flat(&p, column![t1(tname.to_string(), 14.0, c), container(Space::new().width(Length::Fill).height(Length::Fixed(2.0))).style(ui::boxed(if on { p.accent } else { Color::TRANSPARENT }, 1.0, None)).width(Length::Fill)].spacing(5.0).width(Length::Shrink).into(), Some(Msg::PropsTab(i)), pad(4.0, 10.0, 0.0, 10.0)));
        }
        if tabs.len() > 1 {
            body = body.push(tr);
        }
        let tab_i = if pr.tab == 99 { tabs.iter().position(|t| *t == "Sharing").unwrap_or(0) } else { pr.tab };
        let tab = tabs.get(tab_i).copied().unwrap_or("General");
        let path = pr.paths.first().cloned().unwrap_or_default();
        match tab {
            "Permissions" => {
                let mine = info.get("mine").and_then(Value::as_bool).unwrap_or(false);
                body = body.push(prow(&p, "Owner", ptext(&p, format!("{}{}", sv(info, "owner"), if mine { " (you)" } else { "" }))));
                body = body.push(prow(&p, "Group", ptext(&p, sv(info, "group"))));
                let mut g = Column::new().spacing(8.0).push(row![gap(120.0, 1.0), t1("Read", 13.0, p.muted).width(Length::Fixed(80.0)), t1("Write", 13.0, p.muted).width(Length::Fixed(80.0)), t1("Run", 13.0, p.muted).width(Length::Fixed(80.0))]);
                for (who, shift) in [("Owner", 6u32), ("Group", 3), ("Others", 0)] {
                    let mut r = Row::new().align_y(iced::Center).push(t1(who, 14.0, p.text).width(Length::Fixed(120.0)));
                    for (k, bit) in [(4u32, "r"), (2, "w"), (1, "x")] {
                        let b = k << shift;
                        let _ = bit;
                        r = r.push(container(ui::check(&p, pr.mode & b != 0, "", false, Msg::PropsBit(b))).width(Length::Fixed(80.0)));
                    }
                    g = g.push(r);
                }
                body = body.push(g);
                if is_file {
                    let x = pr.mode & 0o111 != 0;
                    body = body.push(ui::check(&p, x, "Allow executing as a program", false, Msg::PropsBit(if x { pr.mode & 0o111 } else { 0o100 | ((pr.mode & 0o044) >> 2) })));
                }
                if !mine {
                    body = body.push(t("Only its owner can change these.", 13.0, p.muted));
                }
            }
            "Sharing" => {
                // Share it on your network (Settings › Home server's shared folders, Samba), stop, or let others change files.
                body = body.push(t("Share this folder with your other devices on your home network (Windows, a Mac, a phone, another Hearth device), through Settings › Home server's shared folders. Shares start read only.", 13.0, p.muted));
                let sh = &pr.share;
                if sh.is_null() || sh.get("error").is_some() {
                    body = body.push(t(if sh.is_null() { "Looking…".to_string() } else { sv(sh, "error") }, 13.0, if sh.is_null() { p.muted } else { p.danger }));
                } else {
                    let shared = sh.get("shared").and_then(Value::as_bool).unwrap_or(false);
                    let s2 = sh.get("share").cloned().unwrap_or(Value::Null);
                    let write = s2.get("write").and_then(Value::as_bool).unwrap_or(false);
                    let value = if shared { format!("Yes, as “{}”{}", sv(&s2, "name"), if write { " · others can change files" } else { " · read only" }) } else { "No".to_string() };
                    let label = if pr.share_busy { "Working…" } else if shared { "Stop sharing" } else { "Share this folder" };
                    body = body.push(prow(&p, "Shared", row![t1(value, 14.0, p.text), Space::new().width(Length::Fill), btn(&p, label, Kind::Normal, false, Some(Msg::Fx(crate::extras::XMsg::Share(if shared { "off".into() } else { "on".into() }))))].align_y(iced::Center).into()));
                    if shared {
                        body = body.push(container(ui::check(&p, write, "Others can change and add files", false, Msg::Fx(crate::extras::XMsg::Share(if write { "write-off".into() } else { "write-on".into() })))).padding(pad(14.0, 14.0, 14.0, 14.0)).width(Length::Fill).style(ui::boxed(p.panel2, p.r(10.0), None)));
                        let how = s2.get("how").cloned().unwrap_or(Value::Null);
                        if how.is_object() {
                            body = body.push(prow(&p, "On Windows", ptext(&p, sv(&how, "windows"))));
                            body = body.push(prow(&p, "On a Mac or phone", ptext(&p, sv(&how, "mac"))));
                            if !sv(&how, "away").is_empty() {
                                body = body.push(prow(&p, "Away (Tailscale)", ptext(&p, sv(&how, "away"))));
                            }
                        }
                    }
                    body = body.push(t(if sh.get("samba") == Some(&Value::Bool(false)) { "Samba (Windows file sharing) comes with the next Hearth OS update." } else { "Your devices ask for your user name and your sharing password: set it in Settings › Home server › Shared folders. Who else can open it: Settings › Home server › People." }, 13.0, p.muted));
                }
            }
            "Checksums" => {
                body = body.push(t("Checksums show whether a download arrived whole: compare them with the ones the website lists.", 13.0, p.muted));
                for (id, label) in [("md5", "MD5"), ("sha1", "SHA-1"), ("sha256", "SHA-256"), ("sha512", "SHA-512")] {
                    let v = pr.sums.get(id).and_then(Value::as_str).unwrap_or("").to_string();
                    let val: El = if v.is_empty() { btn(&p, "Work it out", Kind::Normal, false, Some(Msg::Checksum(id))) } else { t1(ui::fit(&v, 64), 12.5, p.text).font(iced::Font::MONOSPACE).into() };
                    body = body.push(row![t1(label, 14.0, p.muted).width(Length::Fixed(90.0)), val].align_y(iced::Center));
                }
                body = body.push(text_input("Paste the checksum the website lists here", &pr.compare).on_input(Msg::CompareText).size(13.5).padding(8.0).style(input_style(p, true)));
                let cmp = pr.compare.trim().to_lowercase();
                if !cmp.is_empty() {
                    let hit = pr.sums.values().filter_map(Value::as_str).any(|s| s.to_lowercase() == cmp);
                    body = body.push(t1(if hit { "They match ✓" } else { "No match yet (work out the same kind first)" }, 13.5, if hit { p.good } else { p.muted }));
                }
            }
            _ => {
                let isdir = info.get("dir").and_then(Value::as_bool) == Some(true);
                let tdesc = if single {
                    let e = Entry { name: model::base(&path), dir: isdir, mime: sv(info, "mime"), ..Default::default() };
                    let w = model::kind_name(&e);
                    if w.ends_with(" file") && !sv(info, "desc").is_empty() { sv(info, "desc") } else { w }
                } else {
                    sv(info, "summary")
                };
                body = body.push(prow(&p, "Type", ptext(&p, tdesc)));
                let loc = if single { a.win_path(&model::parent_of(&path)) } else { String::new() };
                if single {
                    body = body.push(prow(&p, "Location", ptext(&p, loc)));
                }
                let size = info.get("size").and_then(Value::as_u64).unwrap_or(0);
                let cnt = &pr.count;
                if isdir || pr.paths.len() > 1 {
                    let s = cnt.get("size").and_then(Value::as_u64);
                    body = body.push(prow(&p, "Size", ptext(&p, s.map(model::fmt_size).unwrap_or_else(|| "Adding up…".into()))));
                    let files = cnt.get("files").and_then(Value::as_u64).unwrap_or(0) as usize;
                    let folders = cnt.get("folders").and_then(Value::as_u64).unwrap_or(0) as usize;
                    if !cnt.is_null() {
                        body = body.push(prow(&p, "Contains", ptext(&p, format!("{}, {}", model::plural(files, "file", "files"), model::plural(folders, "folder", "folders")))));
                    }
                } else {
                    body = body.push(prow(&p, "Size", ptext(&p, model::fmt_size(size))));
                }
                if single {
                    let bt = info.get("btime").and_then(Value::as_i64).unwrap_or(0);
                    body = body.push(prow(&p, "Created", ptext(&p, model::fmt_full(if bt > 0 { bt } else { info.get("ctime").and_then(Value::as_i64).unwrap_or(0) }))));
                    let lp = row![t1(path.clone(), 14.0, p.muted), container(t1("only with Show system folders", 12.5, p.muted)).padding(pad(4.0, 12.0, 4.0, 12.0)).style(ui::boxed(Color::TRANSPARENT, 14.0, Some((p.muted, 1.0))))].spacing(14.0).align_y(iced::Center);
                    body = body.push(prow(&p, "Linux path", lp.into()));
                    body = body.push(prow(&p, "Modified", ptext(&p, model::fmt_full(info.get("mtime").and_then(Value::as_i64).unwrap_or(0)))));
                    if let (Some(w), Some(h)) = (info.get("width").and_then(Value::as_u64), info.get("height").and_then(Value::as_u64)) {
                        body = body.push(prow(&p, "Picture size", ptext(&p, format!("{w} × {h}"))));
                    }
                    if let Some(app) = info.get("app").filter(|v| !v.is_null()) {
                        body = body.push(prow(&p, "Opens with", ptext(&p, sv(app, "name"))));
                    }
                    let cp = a.copy_form(&path, a.copy_linux());
                    body = body.push(copy_btn(&p, cp, Msg::PropsCopy(a.copy_linux())));
                }
            }
        }
    }
    body = body.push(buttons(vec![
        btn(&p, "Cancel", Kind::Normal, false, Some(Msg::DlgCancel)),
        btn(&p, "Apply", Kind::Normal, false, if pr.mode != pr.mode0 { Some(Msg::PropsApply(false)) } else { None }),
        btn(&p, "OK", Kind::Primary, false, Some(Msg::PropsApply(true))),
    ]));
    container(column![head, hline::<Msg>(p.line), body]).width(Length::Fixed(700.0)).style(ui::boxed(p.panel, p.r(12.0), Some((p.line, 1.0)))).into()
}

/// Properties' tabs (FileProperties.qml): General, Permissions, Checksums for a file, Sharing for a folder.
pub fn props_tabs(pr: &Props) -> Vec<&'static str> {
    let info = &pr.info;
    let single = pr.paths.len() == 1;
    let is_file = single && info.get("dir").and_then(Value::as_bool) != Some(true);
    let mut tabs = vec!["General"];
    if single && info.get("trashed").and_then(Value::as_bool) != Some(true) && info.get("isLink").and_then(Value::as_bool) != Some(true) {
        tabs.push("Permissions");
    }
    if is_file {
        tabs.push("Checksums");
    }
    if single && !is_file && info.get("trashed").and_then(Value::as_bool) != Some(true) && info.get("isLink").and_then(Value::as_bool) != Some(true) && pr.paths[0].starts_with('/') && pr.loaded {
        tabs.push("Sharing");
    }
    tabs
}

fn copy_btn<'a>(p: &Pal, path: String, msg: Msg) -> El<'a> {
    let p = *p;
    button(container(t1(format!("Copy path → {}", ui::fit(&path, 60)), 14.0, p.text)).center_y(Length::Fixed(24.0)))
        .padding(pad(10.0, 26.0, 10.0, 26.0))
        .on_press(msg)
        .style(move |_: &Theme, st: button::Status| button::Style {
            background: Some(Background::Color(if matches!(st, button::Status::Hovered | button::Status::Pressed) { p.hover() } else { Pal::mix(p.panel, p.bg2, 0.5) })),
            text_color: p.text,
            border: Border { color: p.line, width: 1.0, radius: p.r(8.0).into() },
            shadow: Shadow::default(),
            snap: true,
        })
        .into()
}

#[allow(dead_code)]
fn tm_unused<'a>(p: &Pal) -> El<'a> {
    tm("", 12.0, p.text).into()
}
