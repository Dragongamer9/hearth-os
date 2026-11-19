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

pub fn frame(a: &App) -> Element<'_, Msg> {
    let p = a.sh.p;
    set_big(a.sh.big);
    set_pal(p);
    nav::begin();
    let el = if a.sh.big { frame_b(a) } else { frame_a(a) };
    nav::end();
    nav::Gate::new(el, Msg::Pointer).into()
}

/// The page itself (or the search results, or a category's list), and the lines over it.
fn page_column(a: &App, search_here: bool) -> Column<'_, Msg> {
    let p = a.sh.p;
    let body: Element<Msg> = if search_here && !a.query.is_empty() { search_page(a) } else { crate::page_view(a).unwrap_or_else(|| category(a)) };
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

fn frame_a(a: &App) -> Element<'_, Msg> {
    let p = a.sh.p;
    let rail_bg = Pal::mix(p.bg, p.panel, 0.55);
    let initial = a.sh.info.real.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_else(|| "·".into());
    let avatar = container(tb(initial, 22.0, p.ink)).center_x(Length::Fixed(56.0)).center_y(Length::Fixed(56.0)).style(ui::boxed(p.accent, 28.0, None));
    let who = column![
        tb(if a.sh.info.real.is_empty() { "You".to_string() } else { a.sh.info.real.clone() }, 16.0, p.text),
        t(format!("{} · {}", if a.sh.info.admin { "Administrator" } else { "Standard account" }, a.sh.info.host), 13.0, p.muted),
    ]
    .spacing(2.0);
    // The search field isn't one of the page's places: Y (Ctrl+F) puts the typing there.
    let search = text_input("Search settings", &a.query).id(nav::SEARCH).on_input(Msg::Query).size(14.0).padding(10.0).style(input_style(p));
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
    let rail = container(column![row![avatar, who].spacing(14.0).align_y(iced::Center), Space::new().height(Length::Fixed(18.0)), search, Space::new().height(Length::Fixed(14.0)), scrollable(cats).height(Length::Fill)])
        .padding(Padding { top: 20.0, right: 18.0, bottom: 12.0, left: 18.0 })
        .width(Length::Fixed(320.0))
        .height(Length::Fill)
        .style(ui::boxed(rail_bg, 0.0, None));

    let main = Column::new().push(crumbs(a)).push(Space::new().height(Length::Fixed(18.0))).push(page_column(a, true));
    let main = container(main).padding(Padding { top: 18.0, right: 36.0, bottom: 0.0, left: 44.0 }).width(Length::Fill).height(Length::Fill);

    let titlebar = ui::title_bar(&p, G::Gear, "Hearth Settings", None, TitleMsgs { drag: Msg::Drag, maximize: Msg::Maximize, minimize: Some(Msg::Minimize), close: Msg::Close }, rail_bg);
    let page = column![titlebar, row![rail, main].height(Length::Fill), hline(p.line), hints(a)];
    let base: Element<Msg> = container(page).width(Length::Fill).height(Length::Fill).style(ui::boxed(p.bg, 0.0, None)).into();
    ui::edges(with_dialogs(a, base), Msg::Resize)
}

fn crumbs(a: &App) -> Element<'_, Msg> {
    let p = a.sh.p;
    if !a.query.is_empty() {
        return tb("Search", 34.0, p.text).into();
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
    let cut = parts.len() > 2;
    let shown: Vec<(&'static str, Msg)> = if cut { parts.split_off(parts.len() - 2) } else { parts };
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
    let mut r = Row::new().spacing(24.0).align_y(iced::Center).push(hint(&p, "A", "Select", false, None)).push(hint(&p, "B", "Back", false, None)).push(hint(&p, "Y", "Search", false, None));
    if let Some(x) = x_label(a) {
        r = r.push(hint(&p, "X", x, false, Some(Msg::X)));
    }
    r = r.push(Space::new().width(Length::Fill)).push(row![hint(&p, "LB", "", true, None), hint(&p, "RB", "Previous / next category", true, None)].spacing(6.0)).push(hint(&p, "≡", "Close", true, Some(Msg::Close)));
    container(r).padding(Padding { top: 0.0, right: 26.0, bottom: 0.0, left: 26.0 }).center_y(Length::Fixed(if a.sh.big { 50.0 } else { 52.0 })).width(Length::Fill).style(ui::boxed(Pal::mix(p.bg, p.panel, 0.55), 0.0, None)).into()
}

fn search_page(a: &App) -> Element<'_, Msg> {
    let p = a.sh.p;
    let q = a.query.to_lowercase();
    let mut c = Column::new().spacing(8.0);
    let mut n = 0;
    for pg in PAGES.iter().filter(|x| !x.desc.is_empty() || x.landing) {
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
    for pg in PAGES.iter().filter(|x| x.cat == cat && !x.hidden && x.parent.is_empty() && !x.desc.is_empty()) {
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
