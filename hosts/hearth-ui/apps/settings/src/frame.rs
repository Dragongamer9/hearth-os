//! The frame around every page, in two looks (the approved new Settings, Nov 15, 2026):
//!
//! A, the keyboard-and-mouse look (settings-A1/A2): you, Search settings and the 13 categories on the left; the page's
//! path as its title; the page; the button hints along the bottom. A category's own page (its landing, or any page
//! not built yet) lists its pages here.
//!
//! B, the controller look (settings-B1/B2, Mantel's SettingsFrameB.qml), shown by itself while a controller is what
//! you use (main.rs): "Settings" and the LB / RB tabs on top (Quick, System, Network, Devices, Gaming, Look, Accounts,
//! Server); the tab's pages on the left, the page in big rows in the middle, and on the right the help for the focused
//! setting (what it does, how to change it with the controller, when it was last changed and by whom, from the change
//! history); what the buttons do along the bottom, X being the page's own action (Pin to Quick, Keep these settings)
//! or Default. The Quick tab is the tiles alone. The same pages draw both looks (widgets.rs grows them for B).

use crate::nav::{self, Kind};
use crate::registry::{self, CATEGORIES, PAGES, TABS};
use crate::widgets::*;
use crate::{App, Msg};
use hearth_ui::ui::{self, glyph, hline, t, t1, tb, ts, TitleMsgs, G};
use hearth_ui::Pal;
use iced::widget::{column, container, mouse_area, row, scrollable, stack, text_input, Column, Row, Space};
use iced::{mouse, Background, Color, Element, Length, Padding};

pub fn cat_glyph(id: &str) -> G {
    match id {
        "home" => G::Home,
        "system" => G::Display,
        "net" => G::Wifi,
        "bt" => G::Bt,
        "link" => G::Link,
        "pers" => G::Palette,
        "apps" => G::Store,
        "gaming" => G::Pad,
        "accounts" => G::User,
        "hs" => G::Server,
        "privacy" => G::Shield,
        "a11y" => G::Access,
        _ => G::Update,
    }
}

/// A page's icon from Mantel's icon name (settings-pages.js "icon" isn't carried over: the category's, or a close one).
pub fn page_glyph(id: &str) -> G {
    let p = registry::page(id);
    match id {
        "display" | "info-gpu" | "info-graphics" => G::Display,
        "sound" | "sounds" | "sound-speakers" | "sound-apps" | "push-to-talk" => G::Sound,
        "power" | "power-profiles" | "power-awake" | "battery" | "startup" => G::Power,
        "storage" | "drives" | "memory" | "usb-devices" => G::Drive,
        "datetime" => G::Clock,
        "about" | "info-cpu" => G::Info,
        "notifications" => G::Moon,
        "quick" => G::Grid,
        // Bluetooth & devices: each page its own mark (not the category's Bluetooth one for all of them).
        "bluetooth" | "bluetooth-devices" => G::Bt,
        "bluetooth-receive" => G::Down,
        "keyboard" => G::Keyboard,
        "mouse" | "mouse-buttons" => G::Mouse,
        "phone" => G::Link,
        "phone-messages" | "print-queue" => G::List,
        "kwin-tablet" | "ink-levels" => G::Palette,
        "printers" => G::Grid,
        "scanners" => G::Camera,
        "scan-pdf" => G::Folder,
        "thunderbolt" => G::Rocket,
        _ => p.map(|p| cat_glyph(p.cat)).unwrap_or(G::Gear),
    }
}

/// What X does on this page: its own action (Keep these settings while a display change waits, Pin to Quick on the
/// Quick tab), else "Default" for the focused setting (only in the controller look, as settings-B2 shows).
pub fn x_label(a: &App) -> Option<&'static str> {
    if a.system.keep.is_some() {
        Some("Keep these settings")
    } else if a.page == "quick" {
        Some(crate::quick::x_label(&a.quick))
    } else if a.sh.big {
        Some("Default")
    } else {
        None
    }
}

/// The window's blocks (hearth_ui::blocks): Hearth's flame and "Settings", the search, the categories, the page's title
/// and the page (each page's own sections are its blocks too: secs.rs). As it was when nothing is customised.
pub const WINDOW: hearth_ui::blocks::PageDef = hearth_ui::blocks::PageDef {
    id: "window",
    title: "Settings' window",
    blocks: &[
        hearth_ui::blocks::bl("brand", "Hearth's flame and Settings", 0, 0, 3, 2),
        hearth_ui::blocks::bl("search", "Search settings", 0, 2, 3, 2),
        hearth_ui::blocks::bl("cats", "The categories", 0, 4, 3, 8),
        hearth_ui::blocks::bl("title", "The page's title", 3, 0, 9, 2),
        hearth_ui::blocks::bl("page", "The page", 3, 2, 9, 10),
    ],
};

// ------------------------------------------------------------------ drag and drop (board 3: the same in every app)
use hearth_ui::studio::{tile, Kit};
/// Settings' blocks window: a button for each category (it opens it), its panes, its info.
pub const KIT: Kit = Kit {
    name: "Settings",
    buttons: &[
        tile("home", "Home", G::Home),
        tile("system", "System", G::Display),
        tile("net", "Network & internet", G::Wifi),
        tile("bt", "Bluetooth & devices", G::Bt),
        tile("link", "Hearth Link", G::Link),
        tile("pers", "Personalization", G::Palette),
        tile("apps", "Apps", G::Grid),
        tile("gaming", "Gaming", G::Pad),
        tile("ai", "AI", G::Asterisk),
        tile("accounts", "Accounts", G::User),
        tile("hs", "Home server", G::Server),
        tile("privacy", "Privacy & security", G::Shield),
        tile("a11y", "Accessibility", G::Access),
        tile("updates", "Updates & recovery", G::Update),
    ],
    panes: &[tile("about", "This PC", G::Display), tile("changes", "Recent changes", G::Clock), tile("links", "Every category", G::List)],
    info: &[tile("host", "This PC's name", G::Display), tile("os", "Hearth OS", G::Info), tile("user", "You", G::User), tile("memory", "Memory", G::Pulse), tile("changes", "Settings changed", G::Clock)],
    pane_page: "window",
    bars: &[],
    menus: &[],
};

/// One of Settings' buttons dragged in, pressed: its category.
pub fn block_action(id: &str) -> Option<Msg> {
    CATEGORIES.iter().position(|c| c.0 == id).map(Msg::Cat)
}

fn studio_values(a: &App) {
    let i = &a.sh.info;
    a.describe.values(vec![
        ("host", i.host.clone()),
        ("os", if i.version.is_empty() { i.os.clone() } else { format!("{} {}", i.os, i.version) }),
        ("user", if i.real.is_empty() { i.user.clone() } else { i.real.clone() }),
        ("memory", hearth_ui::ui::bytes(i.mem_kb as f64 * 1024.0)),
        ("changes", a.sh.history.len().to_string()),
    ]);
}

/// This PC (a pane of its own): its name, Hearth OS, the processor, the graphics card, the memory.
fn about_pane(a: &App) -> Element<'_, Msg> {
    let p = a.sh.p;
    let i = &a.sh.info;
    let mut c = Column::new().spacing(6.0);
    for (k, v) in [("Name", i.host.clone()), ("Hearth OS", format!("{} {}", i.os, i.version)), ("Processor", i.cpu.clone()), ("Graphics", i.gpu.clone()), ("Memory", hearth_ui::ui::bytes(i.mem_kb as f64 * 1024.0))] {
        c = c.push(row![t1(k, 13.0, p.muted), Space::new().width(Length::Fill), t1(ui::fit(&v, 40), 13.0, p.text)].spacing(10.0));
    }
    c.into()
}

/// Recent changes (a pane of its own): the last settings changed, by whom and when.
fn changes_pane(a: &App) -> Element<'_, Msg> {
    let p = a.sh.p;
    let mut c = Column::new().spacing(6.0);
    if a.sh.history.is_empty() {
        c = c.push(t("Nothing changed yet.", 13.0, p.muted));
    }
    for ch in a.sh.history.iter().rev().take(8) {
        c = c.push(column![t1(ui::fit(&format!("{} → {}", ch.key, ch.to), 44), 13.0, p.text), t1(format!("{} · {}", ch.who, hearth_ui::time::when(ch.at)), 11.5, p.muted)].spacing(1.0));
    }
    scrollable(c).height(Length::Fill).into()
}

/// Every category (a pane of its own): each one a press away.
fn links_pane(a: &App) -> Element<'_, Msg> {
    let p = a.sh.p;
    let mut c = Column::new().spacing(4.0);
    for (i, (id, name, _)) in CATEGORIES.iter().enumerate() {
        c = c.push(mouse_area(row![glyph(cat_glyph(id), p.muted, 15.0, 1.8), t1(*name, 13.0, p.text)].spacing(10.0).align_y(iced::Center)).on_press(Msg::Cat(i)).interaction(mouse::Interaction::Pointer));
    }
    scrollable(c).height(Length::Fill).into()
}

pub fn frame(a: &App) -> Element<'_, Msg> {
    let p = a.sh.p;
    set_big(a.sh.big);
    set_pal(p);
    nav::begin();
    crate::secs::begin(a.page);
    let el = if a.sh.big { frame_b(a) } else { frame_a(a) };
    if a.query.is_empty() {
        crate::secs::end(a.page);
    }
    nav::end();
    nav::Gate::new(el, Msg::Pointer).into()
}

/// The page itself (or the search results, or a category's list), and the lines over it.
fn page_column(a: &App, search_here: bool) -> Column<'_, Msg> {
    let p = a.sh.p;
    // a kit page laid out or being arranged is drawn block by block (secs.rs split)
    let body: Element<Msg> = if search_here && !a.query.is_empty() { search_page(a) } else { crate::secs::split(a).or_else(|| crate::page_view(a)).unwrap_or_else(|| category(a)) };
    let mut main = Column::new();
    if !a.sh.error.is_empty() {
        main = main.push(note(&p, a.sh.error.clone(), true));
    }
    if !a.sh.note.is_empty() {
        main = main.push(note(&p, a.sh.note.clone(), false));
    }
    main.push(scrollable(container(body).padding(Padding { top: if a.sh.big { 4.0 } else { 0.0 }, right: 14.0, bottom: 24.0, left: if a.sh.big { 4.0 } else { 0.0 } })).id(nav::PAGE).height(Length::Fill))
}

/// The dialogs over everything: a page's own, the controller look's search and "back to its default?".
fn with_dialogs<'a>(a: &'a App, base: Element<'a, Msg>) -> Element<'a, Msg> {
    let p = a.sh.p;
    let mark = nav::len();
    if let Some((title, _)) = &a.confirm {
        let d = ui::dialog_box(&p, 520.0, column![
            ts(format!("Put {title} back to its default?"), 19.0, p.text),
            t(a.confirm.as_ref().map(|c| c.1.clone()).unwrap_or_default(), 14.0, p.muted),
            row![hint(&p, "A", "Put it back", false, Some(Msg::Confirm(true))), hint(&p, "B", "Keep it", false, Some(Msg::Confirm(false)))].spacing(26.0)
        ].spacing(14.0));
        return ui::dialog(base, d, Some(Msg::Confirm(false)));
    }
    if a.finding {
        nav::set_layer(mark);
        let input = text_input("Search settings", &a.query).id(nav::SEARCH).on_input(Msg::Query).size(19.0).padding(12.0).style(input_style(p));
        let mut c = Column::new().spacing(8.0).push(row![glyph(G::Search, p.muted, 22.0, 2.0), input].spacing(12.0).align_y(iced::Center));
        if !a.query.is_empty() {
            c = c.push(search_page(a));
        }
        let d = container(c).padding(18.0).width(Length::Fixed(760.0)).style(ui::boxed(p.panel, p.r(14.0), Some((p.accent, 2.0))));
        let d = container(d).padding(Padding { top: 80.0, right: 0.0, bottom: 0.0, left: 0.0 }).center_x(Length::Fill).height(Length::Fill);
        return ui::dialog(base, d.into(), Some(Msg::Find(false)));
    }
    match crate::dialog_view(a) {
        Some((d, outside)) => {
            nav::set_layer(mark);
            ui::dialog(base, d, outside)
        }
        None => base,
    }
}

// ------------------------------------------------------------------ A: keyboard and mouse

fn frame_a<'a>(a: &'a App) -> Element<'a, Msg> {
    let p = a.sh.p;
    let rail_bg = Pal::mix(p.bg, p.panel, 0.55);
    let initial = a.sh.info.real.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_else(|| "·".into());
    // Your account picture, round (pics.rs, read once off the window's thread); your initial until then or without one.
    let avatar: Element<'_, Msg> = match crate::pics::get(&a.sh.face, crate::pics::ROUND) {
        Some(h) => container(crate::pics::round(h, 56.0)).width(Length::Fixed(56.0)).height(Length::Fixed(56.0)).into(),
        None => container(tb(initial, 22.0, p.ink)).center_x(Length::Fixed(56.0)).center_y(Length::Fixed(56.0)).style(ui::boxed(p.accent, 28.0, None)).into(),
    };
    let who: Column<'_, Msg> = column![
        tb(if a.sh.info.real.is_empty() { "You".to_string() } else { a.sh.info.real.clone() }, 16.0, p.text),
        t(format!("{} · {}", if a.sh.info.admin { "Administrator" } else { "Standard account" }, a.sh.info.host), 13.0, p.muted),
    ]
    .spacing(2.0);
    // The search field isn't one of the page's places: Y (Ctrl+F) puts the typing there.
    // Windows 11's look again (settings-windows-11, the owner, Nov 28): "Find a setting", Y beside it, an accent line under it.
    let field = text_input("Find a setting", &a.query).id(nav::SEARCH).on_input(Msg::Query).size(14.0).padding(10.0).style(input_style(p));
    let ycap = container(tb("Y", 11.0, p.text)).center_x(Length::Fixed(24.0)).center_y(Length::Fixed(24.0)).style(ui::boxed(Color::TRANSPARENT, 12.0, Some((p.muted, 1.2))));
    let search = column![
        row![glyph(G::Search, p.muted, 16.0, 2.0), field, ycap].spacing(8.0).align_y(iced::Center).padding(Padding { top: 0.0, right: 10.0, bottom: 0.0, left: 12.0 }),
        container(Space::new()).width(Length::Fill).height(Length::Fixed(2.0)).style(ui::boxed(p.accent, 1.0, None)),
    ];
    let search = container(search).style(ui::boxed(Pal::mix(rail_bg, p.text, 0.06), p.r(10.0), None));
    let ring = nav::shown() && !nav::in_page();
    let mut cats = Column::new().spacing(2.0);
    for (i, (id, label, _)) in CATEGORIES.iter().enumerate() {
        let on = i == a.cat && a.query.is_empty();
        let mark = container(Space::new()).width(Length::Fixed(3.0)).height(Length::Fixed(18.0)).style(ui::boxed(if on { p.accent } else { Color::TRANSPARENT }, 2.0, None));
        let line = row![mark, glyph(cat_glyph(id), if on { p.accent } else { p.muted }, 18.0, 1.8), t1(*label, 15.0, p.text)].spacing(14.0).align_y(iced::Center);
        let bg = if on { Pal::mix(rail_bg, p.text, 0.07) } else { Color::TRANSPARENT };
        cats = cats.push(
            mouse_area(container(line).padding(Padding { top: 8.0, right: 12.0, bottom: 8.0, left: 4.0 }).width(Length::Fill).style(row_style(p, bg, p.r(8.0), on && ring)))
                .on_press(Msg::Cat(i))
                .interaction(mouse::Interaction::Pointer),
        );
    }
    // Your picture and name over the search, as in Windows 11 (settings-windows-11; the Nov 23 cleanup had Hearth's flame).
    let brand: Element<'_, Msg> = row![avatar, who].spacing(14.0).align_y(iced::Center).into();
    // its blocks (WINDOW), each a right-click away from Edit layout (that part picked); as it was when nothing is
    // while the window is being arranged, each part's own content sits under its block's bar (drawn over its top)
    let edit = |id: &'static str, el: Element<'a, Msg>| -> (&'static str, Element<'a, Msg>) { (id, hearth_ui::arrange::right_edit(el, Msg::Describe(hearth_ui::describe::DMsg::Pick(id)))) };
    studio_values(a);
    let mut items = vec![
        edit("brand", brand),
        edit("search", search.into()),
        edit("cats", scrollable(cats).height(Length::Fill).into()),
        edit("title", hearth_ui::studio::made(&a.describe, "window", "title", || crumbs(a))),
        ("page", page_column(a, true).into()),
    ];
    // drag and drop: Settings' panes where they were dragged in
    if a.describe.wants_pane("about") {
        items.push(("pane:about", about_pane(a)));
    }
    if a.describe.wants_pane("changes") {
        items.push(("pane:changes", changes_pane(a)));
    }
    if a.describe.wants_pane("links") {
        items.push(("pane:links", links_pane(a)));
    }
    let body = hearth_ui::arrange::parts(
        &a.describe,
        &p,
        "window",
        items,
        move |ps| {
            let rail = container(column![ps.take("brand"), Space::new().height(Length::Fixed(18.0)), ps.take("search"), Space::new().height(Length::Fixed(14.0)), ps.take("cats")])
                .padding(Padding { top: 20.0, right: 18.0, bottom: 12.0, left: 18.0 })
                .width(Length::Fixed(320.0))
                .height(Length::Fill)
                .style(ui::boxed(rail_bg, 0.0, None));
            let main = Column::new().push(ps.take("title")).push(Space::new().height(Length::Fixed(18.0))).push(ps.take("page"));
            let main = container(main).padding(Padding { top: 18.0, right: 36.0, bottom: 0.0, left: 44.0 }).width(Length::Fill).height(Length::Fill);
            row![rail, main].height(Length::Fill).into()
        },
        Msg::Describe,
    );

    let titlebar = hearth_ui::describe::title_bar(&p, &a.describe, G::Gear, "Hearth Settings", None, TitleMsgs { drag: Msg::Drag, maximize: Msg::Maximize, minimize: Some(Msg::Minimize), close: Msg::Close }, rail_bg, Msg::Describe(hearth_ui::describe::DMsg::Open));
    // The controller's buttons along the bottom again (settings-windows-11: A Select, B Back, Y Find a setting, LB / RB
    // between sections, ☰ Close).
    let page = column![titlebar, body, hints(a)];
    let base: Element<Msg> = container(page).width(Length::Fill).height(Length::Fill).style(ui::boxed(p.bg, 0.0, None)).into();
    ui::edges(with_dialogs(a, base), Msg::Resize)
}

fn crumbs(a: &App) -> Element<'_, Msg> {
    let p = a.sh.p;
    if !a.query.is_empty() {
        return tb("Search", 34.0, p.text).into();
    }
    // A section's front page: "Settings › System" and its line (cats/sections.rs); Home: "Home" and this PC.
    if let Some(t) = crate::cats::sections::title(&p, a.page) {
        return t;
    }
    if a.page == "home" {
        let i = &a.sh.info;
        let mut bits = vec![i.host.clone()];
        // Hearth OS's own version (the image writes it) and what's underneath ("Hearth OS (Debian 13)" → Debian 13).
        let ver = std::fs::read_to_string("/usr/share/nimbus/version").map(|v| crate::data::shown_version(v.trim())).unwrap_or_default();
        let under = i.os.split('(').nth(1).map(|x| x.trim_end_matches(')').to_string()).unwrap_or_default();
        if !ver.is_empty() && ver != "0.0" {
            bits.push(if under.is_empty() { format!("Hearth OS {ver}") } else { format!("Hearth OS {ver} on {under}") });
        }
        // Windows 11's look (settings-windows-11 home.png): just "Home"; this PC's name and version are on its card.
        let _ = bits;
        return tb("Home", 34.0, p.text).into();
    }
    let cat = CATEGORIES[a.cat];
    let mut chain = registry::chain(a.page);
    // The category's own name leads, unless the page is the category's landing page (Home, Wi-Fi under Network…
    // keep their own name, as the mockups show "System › Display" and "Home").
    let landing = registry::page(a.page).map(|x| x.id == cat.2).unwrap_or(true);
    if landing {
        return tb(registry::page(a.page).map(|x| x.label).unwrap_or(cat.1), 34.0, p.text).into();
    }
    chain.retain(|x| x.id != cat.2);
    // The whole path: the category, then the pages down to this one. A long one (Bluetooth & devices › Bluetooth ›
    // Your Bluetooth devices) would run past the title's room and wrap, so only its last two parts show, after "…"
    // (which goes to the category, as its name would).
    let mut parts: Vec<(&'static str, Msg)> = vec![(cat.1, Msg::Cat(a.cat))];
    parts.extend(chain.into_iter().map(|x| (x.label, Msg::Go(x.id))));
    // Two parts can still be too long ("Firmware & device security › Firmware update history"): then only the page's
    // own name, after "…" (about 46 letters fit at the title's size beside the list, "… › " counted).
    const ROOM: usize = 46;
    let len = |v: &[(&str, Msg)], cut: bool| v.iter().map(|x| x.0.chars().count() + 3).sum::<usize>() + if cut { 4 } else { 0 };
    let mut cut = parts.len() > 2;
    let mut shown: Vec<(&'static str, Msg)> = if cut { parts.split_off(parts.len() - 2) } else { parts };
    if shown.len() > 1 && len(&shown, cut) > ROOM {
        shown = shown.split_off(shown.len() - 1);
        cut = true;
    }
    let chev = || container(glyph(G::Chevron, p.text, 22.0, 2.4)).padding(Padding { top: 6.0, right: 0.0, bottom: 0.0, left: 0.0 });
    let mut r = Row::new().spacing(12.0).align_y(iced::Center);
    if cut {
        r = r.push(mouse_area(tb("…", 34.0, p.muted)).on_press(Msg::Cat(a.cat)).interaction(mouse::Interaction::Pointer)).push(chev());
    }
    let n = shown.len();
    for (i, (label, go)) in shown.into_iter().enumerate() {
        if i > 0 {
            r = r.push(chev());
        }
        if i + 1 == n {
            r = r.push(tb(label, 34.0, p.text).wrapping(iced::widget::text::Wrapping::None));
        } else {
            r = r.push(mouse_area(tb(label, 34.0, p.muted).wrapping(iced::widget::text::Wrapping::None)).on_press(go).interaction(mouse::Interaction::Pointer));
        }
    }
    r.into()
}

/// A button's cap and what it does (A Select). Round for the face buttons, rounded squares for LB / RB / ☰ (as the
/// mockups draw them); a click does it too when it's given a message.
fn hint<'a>(p: &Pal, k: &'a str, label: &'a str, square: bool, msg: Option<Msg>) -> Element<'a, Msg> {
    let w = if k.chars().count() > 1 { 32.0 } else { 24.0 };
    let key = container(tb(k, 11.0, p.text)).center_x(Length::Fixed(w)).center_y(Length::Fixed(24.0)).style(ui::boxed(Color::TRANSPARENT, if square { 7.0 } else { 12.0 }, Some((p.text, 1.2))));
    let r = row![key, t1(label, 14.0, p.text)].spacing(8.0).align_y(iced::Center);
    match msg {
        Some(m) => mouse_area(r).on_press(m).interaction(mouse::Interaction::Pointer).into(),
        None => r.into(),
    }
}

fn hints(a: &App) -> Element<'_, Msg> {
    let p = a.sh.p;
    let mut r = Row::new().spacing(24.0).align_y(iced::Center).push(hint(&p, "A", "Select", false, None)).push(hint(&p, "B", "Back", false, None)).push(hint(&p, "Y", "Find a setting", false, Some(Msg::Find(true))));
    if let Some(x) = x_label(a) {
        r = r.push(hint(&p, "X", x, false, Some(Msg::X)));
    }
    r = r.push(Space::new().width(Length::Fill)).push(row![hint(&p, "LB", "", true, None), hint(&p, "RB", "Previous / next section", true, None)].spacing(6.0)).push(hint(&p, "≡", "Close", true, Some(Msg::Close)));
    container(r).padding(Padding { top: 0.0, right: 26.0, bottom: 0.0, left: 26.0 }).center_y(Length::Fixed(if a.sh.big { 50.0 } else { 52.0 })).width(Length::Fill).style(ui::boxed(Pal::mix(p.bg, p.panel, 0.55), 0.0, None)).into()
}

fn search_page(a: &App) -> Element<'_, Msg> {
    let p = a.sh.p;
    let q = a.query.to_lowercase();
    let mut c = Column::new().spacing(8.0);
    let mut n = 0;
    let kw = crate::shared::kwin_desktop();
    for pg in PAGES.iter().filter(|x| (!x.desc.is_empty() || x.landing) && !x.live && (kw || !crate::shared::kwin_page(x.id))) {
        let hay = format!("{} {} {}", pg.label, pg.desc, pg.words).to_lowercase();
        if !q.split_whitespace().all(|w| hay.contains(w)) {
            continue;
        }
        let cat = CATEGORIES.iter().find(|c| c.0 == pg.cat).map(|c| c.1).unwrap_or("");
        let sub = if registry::built(pg.id) { format!("{cat} › {}", pg.desc) } else { format!("{cat} · coming next") };
        c = c.push(link_row(&p, page_glyph(pg.id), pg.label, sub, Msg::Go(pg.id)));
        n += 1;
        if n >= if a.sh.big { 6 } else { 40 } {
            break;
        }
    }
    if n == 0 {
        c = c.push(note(&p, format!("Nothing in Settings matches \"{}\".", a.query), false));
    }
    c.into()
}

/// A category's own page: its pages as rows (the ones under another page, and hidden ones, aren't listed). Pages
/// not built yet say so plainly and open nothing.
pub fn category(a: &App) -> Element<'_, Msg> {
    let p = a.sh.p;
    let cat = CATEGORIES[a.cat].0;
    // A page that isn't built, opened on its own (a link, `hearth-settings PAGE`): say so at the top.
    let mut c = Column::new().spacing(8.0);
    if let Some(pg) = registry::page(a.page) {
        if !registry::built(pg.id) && !pg.landing {
            c = c.push(card(&p, column![t(format!("{} isn't in the new Settings app yet.", pg.label), 15.0, p.text), t(pg.desc, 13.0, p.muted)].spacing(4.0)));
        }
    }
    let mut unbuilt = Vec::new();
    let kw = crate::shared::kwin_desktop();
    for pg in PAGES.iter().filter(|x| x.cat == cat && !x.hidden && !x.live && x.parent.is_empty() && !x.desc.is_empty() && (kw || !crate::shared::kwin_page(x.id))) {
        if registry::built(pg.id) {
            c = c.push(link_row(&p, page_glyph(pg.id), pg.label, pg.desc, Msg::Go(pg.id)));
        } else {
            unbuilt.push(pg);
        }
    }
    if !unbuilt.is_empty() {
        c = c.push(head(&p, "Coming next"));
        for pg in unbuilt {
            c = c.push(srow(&p, page_glyph(pg.id), pg.label, pg.desc, t1("Coming next", 13.0, p.muted).into()));
        }
    }
    c.into()
}

/// The pages under a page (its children), as rows: for a built page that has some.
pub fn children<'a, M: Clone + Send + 'static>(p: &Pal, parent: &str, go: impl Fn(&'static str) -> M) -> Column<'a, M> {
    let mut c = Column::new().spacing(8.0);
    for pg in PAGES.iter().filter(|x| x.parent == parent && !x.hidden) {
        if registry::built(pg.id) {
            c = c.push(link_row(p, page_glyph(pg.id), pg.label, pg.desc, go(pg.id)));
        }
    }
    c
}

// ------------------------------------------------------------------ B: the controller look

/// The pages on the left for this tab (a tab with none built yet lists its categories' own pages).
pub fn tab_list(tab: usize) -> Vec<&'static registry::Page> {
    let v = registry::tab_pages(tab);
    if !v.is_empty() || tab == 0 {
        return v;
    }
    TABS[tab].2.iter().filter_map(|c| CATEGORIES.iter().find(|x| x.0 == *c).and_then(|x| registry::page(x.2))).collect()
}

/// What a setting does, in more words than its row has room for (the help panel, settings-B2).
pub fn help_for(title: &str) -> Option<&'static str> {
    Some(match title {
        "Resolution" => "How many dots the screen draws. Higher is sharper; lower can make games faster.",
        "Refresh rate" => "How many pictures a second the display shows. Higher is smoother in games.",
        "Scale" => "Makes text, apps and the bar bigger or smaller.",
        "Brightness" => "How bright the screen is.",
        "Night light" => "Warmer colours in the evening, easier on your eyes at night.",
        "HDR" => "A brighter picture with more colour in games and videos that have it.",
        "Volume" => "How loud this PC plays. Left and right change it.",
        "Microphone" => "How loud your microphone records.",
        "Start in" => "What this PC opens when it turns on: Game Mode (Steam) or Desktop Mode.",
        "Light mode while you play" => "Big home server jobs (backups, AI models, downloads) wait until your game ends, so games stay smooth.",
        "Do not disturb" => "Notifications go straight to the history without popping up.",
        "Wi-Fi" => "The wireless network this PC uses.",
        "Bluetooth" => "Controllers, headphones and other wireless devices.",
        _ => return None,
    })
}

/// How to change the focused setting with the controller (Mantel's SettingsFrameB howTo).
pub fn how_to(kind: Kind, title: &str) -> String {
    let base = match kind {
        Kind::Choice | Kind::Slider => "Pick with ‹ › (left and right on the stick or D-pad).",
        Kind::Toggle(_) => "Press A to turn it on or off.",
        Kind::Open => "Press A to open it; B comes back.",
        Kind::Press => "Press A to choose it.",
        Kind::Field => "Press A to type here; the on-screen keyboard opens with it.",
        Kind::Plain => "",
    };
    if matches!(title, "Resolution" | "Refresh rate" | "Scale") {
        format!("{base} If nothing shows after a change, Hearth puts the old one back by itself after 15 seconds.")
    } else {
        base.to_string()
    }
}

/// The settings X can put back to their default (settings-changes.js DEFAULTS, the switches), by the name their row has.
pub fn default_of(title: &str) -> Option<bool> {
    Some(match title {
        "Night light" => false,
        "Reduce motion" => false,
        "Lock when it wakes" => false,
        "24-hour clock" => false,
        "Clipboard history" => true,
        "Sunset mode" => false,
        "Do not disturb" => false,
        _ => return None,
    })
}

fn keycap<'a>(p: &Pal, k: &'a str) -> Element<'a, Msg> {
    container(tb(k, 11.0, p.text)).center_x(Length::Fixed(32.0)).center_y(Length::Fixed(22.0)).style(ui::boxed(Color::TRANSPARENT, 6.0, Some((p.muted, 1.4)))).into()
}

fn frame_b(a: &App) -> Element<'_, Msg> {
    let p = a.sh.p;
    let tab = registry::tab_of(a.page);
    // Top: "Settings" and the tabs.
    let mut tabs = Row::new().spacing(2.0).align_y(iced::Center).push(keycap(&p, "LB")).push(Space::new().width(Length::Fixed(4.0)));
    for (i, (_, label, _)) in TABS.iter().enumerate() {
        let lit = i == tab;
        let l = t1(*label, 14.0, if lit { p.ink } else { p.muted }).font(if lit { ui::weight(iced::font::Weight::Semibold) } else { hearth_ui::app::INTER });
        tabs = tabs.push(mouse_area(container(l).padding(Padding { top: 7.0, right: 16.0, bottom: 7.0, left: 16.0 }).style(ui::boxed(if lit { p.accent } else { Color::TRANSPARENT }, p.r(10.0), None))).on_press(Msg::Tab(i)).interaction(mouse::Interaction::Pointer));
    }
    let tabs = container(tabs.push(Space::new().width(Length::Fixed(4.0))).push(keycap(&p, "RB"))).padding(Padding { top: 5.0, right: 8.0, bottom: 5.0, left: 8.0 }).style(ui::boxed(p.panel, p.r(12.0), Some((Pal::a(p.text, 0.06), 1.0))));
    let header = mouse_area(container(row![glyph(G::Gear, p.text, 24.0, 2.6), tb("Settings", 28.0, p.text), Space::new().width(Length::Fill), tabs].spacing(12.0).align_y(iced::Center)).padding(Padding { top: 14.0, right: 32.0, bottom: 14.0, left: 32.0 })).on_press(Msg::Drag);

    // Middle: the page (built first: its rows are the places the help panel talks about).
    let page = page_column(a, false);
    nav::flush();
    let body: Element<Msg> = if tab == 0 {
        container(page).width(Length::Fill).height(Length::Fill).into()
    } else {
        // Left: the tab's pages.
        let top = registry::top_of(a.page);
        let ring = nav::shown() && !nav::in_page();
        let mut list = Column::new().spacing(6.0);
        for pg in tab_list(tab) {
            let lit = pg.id == top || pg.id == a.page;
            let mark = container(Space::new()).width(Length::Fixed(3.0)).height(Length::Fixed(22.0)).style(ui::boxed(if lit { p.accent } else { Color::TRANSPARENT }, 2.0, None));
            let l = t1(pg.label, 15.0, p.text).font(if lit { ui::weight(iced::font::Weight::Semibold) } else { hearth_ui::app::INTER });
            let line = row![mark, glyph(page_glyph(pg.id), if lit { p.accent } else { p.muted }, 18.0, 1.8), l].spacing(12.0).align_y(iced::Center);
            list = list.push(
                mouse_area(container(line).padding(Padding { top: 10.0, right: 14.0, bottom: 10.0, left: 4.0 }).width(Length::Fill).style(row_style(p, if lit { p.panel } else { Color::TRANSPARENT }, p.r(12.0), lit && ring)))
                    .on_press(Msg::Go(pg.id))
                    .interaction(mouse::Interaction::Pointer),
            );
        }
        let list = scrollable(container(list).padding(Padding { top: 4.0, right: 4.0, bottom: 4.0, left: 4.0 })).width(Length::Fixed(232.0)).height(Length::Fill);
        row![list, container(page).width(Length::Fill).height(Length::Fill), help(a)].spacing(22.0).height(Length::Fill).into()
    };
    let body = container(body).padding(Padding { top: 4.0, right: 32.0, bottom: 0.0, left: 28.0 }).width(Length::Fill).height(Length::Fill);
    let content = column![header, body, hline(p.line), hints(a)];
    // A warm glow in the top right, as in the mockups.
    let glow = iced::gradient::Linear::new(iced::Radians(std::f32::consts::FRAC_PI_2)).add_stop(0.0, Color::TRANSPARENT).add_stop(1.0, Pal::a(p.accent, 0.09));
    let glow = row![Space::new().width(Length::FillPortion(1)), container(Space::new()).width(Length::FillPortion(1)).height(Length::Fixed(170.0)).style(move |_: &iced::Theme| container::Style { background: Some(Background::Gradient(glow.into())), ..container::Style::default() })];
    // (A stack is as big as its first layer: the glow's layer fills the window.)
    let glow = container(column![glow, Space::new().height(Length::Fill)]).width(Length::Fill).height(Length::Fill);
    let base: Element<Msg> = container(stack![glow, content].width(Length::Fill).height(Length::Fill)).width(Length::Fill).height(Length::Fill).style(ui::boxed(p.bg, 0.0, None)).into();
    ui::edges(with_dialogs(a, base), Msg::Resize)
}

/// The help panel (settings-B2): the focused setting's name, what it does, how to change it with the controller, and
/// when it was last changed and by whom (the change history). Without a focused setting, the page's own description.
fn help(a: &App) -> Element<'_, Msg> {
    let p = a.sh.p;
    let pg = registry::page(a.page);
    let f = if nav::in_page() { nav::focused() } else { None };
    let (title, desc, how) = match &f {
        Some(f) if !f.title.is_empty() => (f.title.clone(), help_for(&f.title).map(String::from).unwrap_or_else(|| f.sub.clone()), how_to(f.kind, &f.title)),
        _ => (pg.map(|x| x.label).unwrap_or("Settings").to_string(), pg.map(|x| x.desc).unwrap_or("").to_string(), "Press A or → to go into the page; up and down pick another page.".to_string()),
    };
    let mut c = Column::new().spacing(14.0).push(ts(title.clone(), 19.0, p.text));
    if !desc.is_empty() {
        c = c.push(t(desc, 14.0, p.muted).line_height(1.3));
    }
    if !how.is_empty() {
        c = c.push(t(how, 14.0, p.muted).line_height(1.3));
    }
    if let Some(ch) = a.sh.history.iter().find(|c| crate::data::setting_name(&c.key) == title) {
        let when = hearth_ui::time::when(ch.at);
        let who = if ch.who.is_empty() { "you".to_string() } else { ch.who.clone() };
        c = c.push(container(t1(if when.is_empty() { format!("Changed by {who}") } else { format!("Changed by {who} · {when}") }, 12.0, p.muted)).padding(Padding { top: 3.0, right: 12.0, bottom: 3.0, left: 12.0 }).style(ui::boxed(Color::TRANSPARENT, 12.0, Some((p.line, 1.0)))));
    }
    container(c).padding(22.0).width(Length::Fixed(286.0)).height(Length::Fill).style(ui::boxed(p.panel, p.r(14.0), Some((Pal::a(p.text, 0.05), 1.0)))).into()
}
