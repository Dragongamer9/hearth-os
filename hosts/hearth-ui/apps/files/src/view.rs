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
pub fn view(a: &App) -> El<'_> {
    let p = a.p;
    let picking = a.picking();
    let mut col = Column::new();
    if let Some(fm) = hearth_ui::ui::frame_meter() {
        col = col.push(fm); // CI's frame counter only (HEARTH_UI_FRAMES)
    }
    if picking {
        col = col.push(pick_head(a));
    } else {
        col = col.push(tab_strip(a)).push(nav_row(a)).push(command_bar(a));
    }
    col = col.push(hline(p.line));
    let mut body = Row::new().height(Length::Fill);
    if a.nav_shown() {
        body = body.push(nav_pane(a)).push(vline(p.line));
    }
    body = body.push(container(main_area(a)).width(Length::Fill).height(Length::Fill));
    if a.details_pane && !picking {
        body = body.push(vline(p.line)).push(details_pane(a));
    }
    col = col.push(body);
    if let Some(j) = &a.job {
        col = col.push(hline(p.line)).push(job_bar(a, j));
    }
    if picking {
        col = col.push(hline(p.line)).push(pick_bar(a));
    } else {
        col = col.push(status_bar(a));
    }
    let base: El = container(col).width(Length::Fill).height(Length::Fill).style(ui::boxed(p.bg, 0.0, None)).into();
    let base: El = mouse_area(base).on_move(Msg::Cursor).into();
    let base = match &a.overlay {
        Some(Overlay::Menu(m)) => {
            let (x, y) = menu_place(a, m);
            let catcher = mouse_area(Space::new().width(Length::Fill).height(Length::Fill)).on_press(Msg::MenuClose).on_right_press(Msg::MenuClose);
            stack![base, catcher, pin(opaque(menu(a, m))).x(x).y(y)].into()
        }
        Some(Overlay::Props(pr)) => ui::dialog(base, props_dialog(a, pr), None),
        Some(ov) => ui::dialog(base, dialog(a, ov), None),
        None => base,
    };
    ui::edges(base, Msg::Resize)
}

fn menu_place(a: &App, m: &Menu) -> (f32, f32) {
    let items_h: f32 = m.items.iter().map(|i| if i.sep { 9.0 } else if i.heading { 28.0 } else if i.quick && m.short { 0.0 } else { 30.0 }).sum();
    let h = items_h + if m.short { 46.0 } else { 34.0 } + 10.0;
    let x = m.at.x.min(1600.0).max(4.0);
    let y = if m.at.y + h > 760.0 { (m.at.y - h).max(40.0).min(m.at.y) } else { m.at.y };
    (x, y.max(4.0))
}

// ------------------------------------------------------------------ the tabs (the window's title bar)
fn tab_strip(a: &App) -> El<'_> {
    let p = a.p;
    let mut r = Row::new().spacing(2.0).align_y(iced::Bottom).padding(pad(0.0, 0.0, 0.0, 10.0));
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
        r = r.push(mouse_area(tab).on_press(Msg::Tab(i)).on_middle_press(Msg::CloseTab(i)));
    }
    r = r.push(container(flat(&p, ico(Ic::Plus, p.muted, 15.0), Some(Msg::NewTab), pad(6.0, 8.0, 6.0, 8.0))).padding(pad(0.0, 0.0, 2.0, 4.0)));
    let drag = mouse_area(Space::new().width(Length::Fill).height(Length::Fixed(40.0))).on_press(Msg::Drag).on_double_click(Msg::Maximize);
    let wbtn = |ic: Ic, msg: Msg, red: bool| -> El<'static> {
        button(container(ico::<Msg>(ic, p.muted, 13.0)).center_x(Length::Fixed(46.0)).center_y(Length::Fixed(40.0)))
            .padding(0.0)
            .on_press(msg)
            .style(move |_: &Theme, st: button::Status| button::Style {
                background: Some(Background::Color(match st {
                    button::Status::Hovered | button::Status::Pressed if red => Color::from_rgb8(0xc4, 0x2b, 0x1c),
                    button::Status::Hovered | button::Status::Pressed => p.hover(),
                    _ => Color::TRANSPARENT,
                })),
                text_color: p.text,
                border: Border::default(),
                shadow: Shadow::default(),
                snap: true,
            })
            .into()
    };
    let bar = row![r, drag, wbtn(Ic::Minimize, Msg::Minimize, false), wbtn(Ic::Maximize, Msg::Maximize, false), wbtn(Ic::Close, Msg::Close, true)].align_y(iced::Bottom);
    container(bar).width(Length::Fill).height(Length::Fixed(40.0)).style(ui::boxed(Pal::mix(p.bg, Color::BLACK, if p.dark { 0.3 } else { 0.06 }), 0.0, None)).into()
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
        let label = t1(ui::fit(&c.label, 28), 14.0, p.text);
        r = r.push(flat(&p, label.into(), Some(Msg::Go(c.path.clone())), pad(3.0, 6.0, 3.0, 6.0)));
    }
    let rest = mouse_area(Space::new().width(Length::Fill).height(Length::Fixed(height))).on_press(Msg::EditPath).interaction(mouse::Interaction::Text);
    r = r.push(rest);
    container(r).width(Length::Fill).height(Length::Fixed(height)).clip(true).style(ui::boxed(field_bg(&p), p.r(6.0), Some((p.line, 1.0)))).into()
}

fn nav_row(a: &App) -> El<'_> {
    let p = a.p;
    let nb = |ic: Ic, on: bool, msg: Msg| -> El<'static> { flat(&p, ico(ic, if on { p.text } else { Pal::a(p.muted, 0.6) }, 17.0), if on { Some(msg) } else { None }, pad(7.0, 8.0, 7.0, 8.0)) };
    let place = if a.found.is_some() { format!("Found “{}”", a.found_query) } else if a.on_home() { "Search Home".into() } else if a.on_this_pc() { "Search".into() } else { format!("Search {}", a.title_of(&a.path)) };
    let mut sr = Row::new().align_y(iced::Center).push(
        text_input(&place, &a.search_text).id(SEARCH_ID).on_input(Msg::SearchText).on_submit(Msg::SearchGo).size(14.0).padding(pad(6.0, 10.0, 6.0, 12.0)).style(input_style(p, true)),
    );
    if !a.search_text.is_empty() || a.found.is_some() {
        sr = sr.push(flat(&p, ico(Ic::Close, p.muted, 12.0), Some(Msg::SearchClear), pad(6.0, 6.0, 6.0, 6.0)));
    }
    let search = container(sr).width(Length::Fixed(if a.area.get().0 < 700.0 { 170.0 } else { 240.0 }));
    container(
        row![
            nb(Ic::Back, !a.back.is_empty(), Msg::Back),
            nb(Ic::Forward, !a.fwd.is_empty(), Msg::Forward),
            nb(Ic::Up, !(a.path == "/" || a.on_home()), Msg::Up),
            nb(Ic::Refresh, true, Msg::Refresh),
            gap(10.0, 1.0),
            address(a, 32.0),
            gap(8.0, 1.0),
            search,
        ]
        .align_y(iced::Center)
        .spacing(4.0),
    )
    .padding(pad(8.0, 12.0, 6.0, 12.0))
    .into()
}

// ------------------------------------------------------------------ the command bar
fn command_bar(a: &App) -> El<'_> {
    let p = a.p;
    let has = !a.targets().is_empty();
    let w = a.writable();
    let sep = || -> El<'static> { container(vline::<Msg>(p.line)).height(Length::Fixed(20.0)).padding(pad(0.0, 6.0, 0.0, 6.0)).into() };
    let ib = |ic: Ic, on: bool, cmd: &'static str| -> El<'static> {
        flat(&p, ico(ic, if on { p.text } else { Pal::a(p.muted, 0.55) }, 17.0), if on { Some(Msg::Cmd(cmd)) } else { None }, pad(7.0, 11.0, 7.0, 11.0))
    };
    let lb = |ic: Ic, label: &'static str, more: bool, on: bool, cmd: &'static str| -> El<'static> {
        let c = if on { p.text } else { Pal::a(p.muted, 0.55) };
        let mut r = Row::new().spacing(7.0).align_y(iced::Center).push(ico(ic, c, 16.0)).push(t1(label, 14.0, c));
        if more {
            r = r.push(ico(Ic::ChevDown, c, 9.0));
        }
        flat(&p, r.into(), if on { Some(Msg::Cmd(cmd)) } else { None }, pad(6.0, 10.0, 6.0, 10.0))
    };
    let r = row![
        lb(Ic::Plus, "New", true, w, "new"),
        sep(),
        ib(Ic::Cut, has && w, "cut"),
        ib(Ic::Copy, has, "copy"),
        ib(Ic::Paste, w, "paste"),
        ib(Ic::Rename, a.current().is_some() && !a.on_special() && !a.on_home() && w, "rename"),
        ib(Ic::Share, has || !a.on_home(), "share"),
        ib(Ic::Delete, has && (w || a.in_trash()), "delete"),
        sep(),
        lb(Ic::Sort, "Sort", true, true, "sort"),
        lb(Ic::ViewList, "View", true, true, "view"),
        Space::new().width(Length::Fill),
        {
            let c = if a.details_pane { p.accent } else { p.text };
            flat(&p, row![t1("Details", 14.0, c), ico(Ic::Pane, c, 16.0)].spacing(7.0).align_y(iced::Center).into(), Some(Msg::Cmd("details")), pad(6.0, 10.0, 6.0, 10.0))
        },
        ib(Ic::Dots, true, "more"),
    ]
    .align_y(iced::Center)
    .spacing(2.0);
    container(r).padding(pad(2.0, 12.0, 6.0, 12.0)).width(Length::Fill).into()
}

// ------------------------------------------------------------------ the left menu
fn nav_pane(a: &App) -> El<'_> {
    let p = a.p;
    let wide = a.nav_w >= 200.0;
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
        let ic = match q.id.as_str() {
            "gallery" => Ic::Gallery,
            "homeserver" => Ic::Cloud,
            _ if q.kind == "system" || (q.section == "pc" && q.depth == 1 && q.raw.get("letter").is_some() && q.kind != "usb" && q.raw.get("removable") != Some(&Value::Bool(true))) => Ic::Disc,
            _ if q.raw.get("removable") == Some(&Value::Bool(true)) => Ic::Usb,
            _ => icons::place_icon(&q.kind),
        };
        let mut r = Row::new()
            .spacing(0.0)
            .align_y(iced::Center)
            .push(gap(depth * 9.0, 1.0))
            .push(arrow)
            .push(gap(4.0, 1.0))
            .push(ico(ic, p.text, 16.0))
            .push(gap(8.0, 1.0))
            .push(t1(q.label.clone(), 13.5, if lit { p.ink } else { p.text }).font(if here { ui::weight(iced::font::Weight::Medium) } else { hearth_ui::app::INTER }));
        r = r.push(Space::new().width(Length::Fill));
        if wide && !q.sub.is_empty() && (q.id == "homeserver" || q.id == "linux") {
            r = r.push(t1(q.sub.clone(), 12.5, p.muted));
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
        let bg = if lit { p.accent } else if here { p.sel() } else { Color::TRANSPARENT };
        let accent = p.accent;
        let rr = p.r(6.0);
        let cell = container(r.padding(pad(0.0, 8.0, 0.0, 0.0))).width(Length::Fill).height(Length::Fixed(28.0)).center_y(Length::Fixed(28.0)).style(move |_: &Theme| container::Style {
            background: Some(Background::Color(bg)),
            border: Border { color: if here { Pal::a(accent, 0.7) } else { Color::TRANSPARENT }, width: if here { 1.0 } else { 0.0 }, radius: rr.into() },
            ..container::Style::default()
        });
        let cell: El = if here {
            // The bar on the place you're in (Windows 11's navigation pane).
            let bar = container(Space::new().width(Length::Fixed(3.0)).height(Length::Fixed(16.0))).style(ui::boxed(accent, 2.0, None));
            stack![cell, container(bar).height(Length::Fixed(28.0)).center_y(Length::Fixed(28.0))].into()
        } else {
            cell.into()
        };
        col = col.push(mouse_area(cell).on_press(Msg::Place(i)).on_right_press(Msg::PlaceRight(i)).on_middle_press(Msg::PlaceMiddle(i)).interaction(mouse::Interaction::Pointer));
    }
    container(scrollable(col).height(Length::Fill)).width(Length::Fixed(a.nav_w)).height(Length::Fill).into()
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
fn scroller<'a>(inner: El<'a>) -> El<'a> {
    scrollable(inner).id(SCROLL_ID).on_scroll(|v| Msg::Scrolled(v.absolute_offset().y)).height(Length::Fill).width(Length::Fill).into()
}

fn entry_icon<'a>(a: &'a App, e: &Entry, size: f32) -> El<'a> {
    if let Some(cv) = &e.cv {
        if cv.lock {
            return ico(Ic::Lock, a.p.text, size);
        }
    }
    ico(icons::file_icon(model::kind_of(e), &e.name), a.p.text, size)
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
    let cols = [("name", "Name", 0.0), ("date", "Date modified", 160.0), ("type", "Type", 150.0), ("size", "Size", 96.0)];
    let header = details_header(a, &cols);
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
                    let r = row![
                        container(row![entry_icon(a, e, 16.0), name_cell(a, e, 14.0)].spacing(9.0).align_y(iced::Center)).width(Length::Fill),
                        container(t1(model::fmt_date(e.mtime, false), 13.5, p.muted)).width(Length::Fixed(160.0)).clip(true),
                        container(t1(model::kind_name(e), 13.5, p.muted)).width(Length::Fixed(150.0)).padding(pad(0.0, 8.0, 0.0, 0.0)).clip(true),
                        container(t1(if e.dir { String::new() } else { model::fmt_size(e.size) }, 13.5, p.muted)).width(Length::Fixed(96.0)).clip(true),
                    ]
                    .align_y(iced::Center)
                    .padding(pad(0.0, 2.0, 0.0, 12.0));
                    col = col.push(clickable(row_box(a, r.into(), sel, *i == a.cur, ROW_H), *i));
                }
            }
        }
        if last < total {
            col = col.push(gap(1.0, (total - last) as f32 * rh));
        }
        let bg = mouse_area(container(col).width(Length::Fill)).on_press(Msg::BgPress).on_right_press(Msg::BgRight);
        scroller(bg.into())
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
                c = c.push(clickable(row_box(a, r.into(), sel, j == a.cur, ROW_H), j));
            }
            cols = cols.push(c);
            i += per;
        }
        a.cols.set(1);
        let _ = p;
        scrollable(mouse_area(cols).on_press(Msg::BgPress).on_right_press(Msg::BgRight))
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
        let bg = mouse_area(container(col).width(Length::Fill)).on_press(Msg::BgPress).on_right_press(Msg::BgRight);
        scroller(bg.into())
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
                Some(h) => img(h).width(Length::Fill).height(Length::Fixed(th)).content_fit(ContentFit::Cover).border_radius(iced::border::Radius { top_left: rr - 1.0, top_right: rr - 1.0, bottom_left: 0.0, bottom_right: 0.0 }).into(),
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
    let bx = container(inner).width(Length::Fixed(cw)).height(Length::Fixed(ch)).center_y(Length::Fixed(ch)).clip(true).style(frame(sel, lit));
    clickable(bx.into(), i)
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
    let w = 262.0;
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
    if a.versions_of == e.path {
        rows.push(("Previous versions".into(), a.versions.len().to_string()));
    }
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
    container(row![left, Space::new().width(Length::Fill), t1(right, 13.0, p.muted)].align_y(iced::Center))
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
        if let Some(ic) = it.icon {
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
                text_input("", &i.text).id(DLG_ID).on_input(Msg::DlgText).on_submit(Msg::DlgOk).size(14.0).padding(9.0).style(input_style(p, true)),
                buttons(vec![btn(&p, "Cancel", Kind::Normal, false, Some(Msg::DlgCancel)), btn(&p, i.ok.clone(), Kind::Primary, false, Some(Msg::DlgOk))]),
            ]
            .spacing(14.0);
            ui::dialog_box(&p, 460.0, col)
        }
        Overlay::Confirm(c) => {
            let col = column![
                ts(c.title.clone(), 17.0, p.text),
                t(c.text.clone(), 13.5, p.muted),
                buttons(vec![
                    btn(&p, "Cancel", Kind::Normal, c.button == 0, Some(Msg::DlgCancel)),
                    btn(&p, c.ok.clone(), if c.danger { Kind::Danger } else { Kind::Primary }, c.button == 1, Some(Msg::DlgOk)),
                ]),
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
        let mut tabs = vec!["General"];
        if single && info.get("trashed").and_then(Value::as_bool) != Some(true) && info.get("isLink").and_then(Value::as_bool) != Some(true) {
            tabs.push("Permissions");
        }
        if is_file {
            tabs.push("Checksums");
        }
        let mut tr = Row::new().spacing(4.0);
        for (i, tname) in tabs.iter().enumerate() {
            let on = i == pr.tab;
            let c = if on { p.text } else { p.muted };
            tr = tr.push(flat(&p, column![t1(tname.to_string(), 14.0, c), container(Space::new().width(Length::Fill).height(Length::Fixed(2.0))).style(ui::boxed(if on { p.accent } else { Color::TRANSPARENT }, 1.0, None)).width(Length::Fill)].spacing(5.0).width(Length::Shrink).into(), Some(Msg::PropsTab(i)), pad(4.0, 10.0, 0.0, 10.0)));
        }
        if tabs.len() > 1 {
            body = body.push(tr);
        }
        let tab = tabs.get(pr.tab).copied().unwrap_or("General");
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
