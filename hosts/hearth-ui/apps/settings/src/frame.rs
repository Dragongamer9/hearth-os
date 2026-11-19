//! The frame around every page (settings-A1/A2): you, Search settings and the 13 categories on the left; the page's
//! path as its title; the page; the button hints along the bottom. A category's own page (its landing, or any page
//! not built yet) lists its pages here.

use crate::registry::{self, CATEGORIES, PAGES};
use crate::widgets::*;
use crate::{App, Msg};
use hearth_ui::ui::{self, glyph, hline, t, t1, tb, TitleMsgs, G};
use hearth_ui::Pal;
use iced::widget::{column, container, mouse_area, row, scrollable, Column, Row, Space};
use iced::{mouse, Color, Element, Length, Padding};

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
        _ => p.map(|p| cat_glyph(p.cat)).unwrap_or(G::Gear),
    }
}

pub fn frame(a: &App) -> Element<'_, Msg> {
    let p = a.sh.p;
    set_big(a.sh.big);
    let rail_bg = Pal::mix(p.bg, p.panel, 0.55);
    let initial = a.sh.info.real.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_else(|| "·".into());
    let avatar = container(tb(initial, 22.0, p.ink)).center_x(Length::Fixed(56.0)).center_y(Length::Fixed(56.0)).style(ui::boxed(p.accent, 28.0, None));
    let who = column![
        tb(if a.sh.info.real.is_empty() { "You".to_string() } else { a.sh.info.real.clone() }, 16.0, p.text),
        t(format!("{} · {}", if a.sh.info.admin { "Administrator" } else { "Standard account" }, a.sh.info.host), 13.0, p.muted),
    ]
    .spacing(2.0);
    let search = field(&p, "Search settings", &a.query, Msg::Query).padding(10.0);
    let mut cats = Column::new().spacing(2.0);
    for (i, (id, label, _)) in CATEGORIES.iter().enumerate() {
        let on = i == a.cat && a.query.is_empty();
        let mark = container(Space::new()).width(Length::Fixed(3.0)).height(Length::Fixed(18.0)).style(ui::boxed(if on { p.accent } else { Color::TRANSPARENT }, 2.0, None));
        let line = row![mark, glyph(cat_glyph(id), if on { p.accent } else { p.muted }, 18.0, 1.8), t1(*label, 15.0, p.text)].spacing(14.0).align_y(iced::Center);
        cats = cats.push(
            mouse_area(container(line).padding(Padding { top: 8.0, right: 12.0, bottom: 8.0, left: 4.0 }).width(Length::Fill).style(ui::boxed(if on { Pal::mix(rail_bg, p.text, 0.07) } else { Color::TRANSPARENT }, p.r(8.0), None)))
                .on_press(Msg::Cat(i))
                .interaction(mouse::Interaction::Pointer),
        );
    }
    let rail = container(column![row![avatar, who].spacing(14.0).align_y(iced::Center), Space::new().height(Length::Fixed(18.0)), search, Space::new().height(Length::Fixed(14.0)), scrollable(cats).height(Length::Fill)])
        .padding(Padding { top: 20.0, right: 18.0, bottom: 12.0, left: 18.0 })
        .width(Length::Fixed(320.0))
        .height(Length::Fill)
        .style(ui::boxed(rail_bg, 0.0, None));

    let body: Element<Msg> = if !a.query.is_empty() { search_page(a) } else { crate::page_view(a).unwrap_or_else(|| category(a)) };
    let mut main = Column::new().push(crumbs(a)).push(Space::new().height(Length::Fixed(18.0)));
    if !a.sh.error.is_empty() {
        main = main.push(note(&p, a.sh.error.clone(), true));
    }
    if !a.sh.note.is_empty() {
        main = main.push(note(&p, a.sh.note.clone(), false));
    }
    let main = container(main.push(scrollable(container(body).padding(Padding { top: 0.0, right: 14.0, bottom: 24.0, left: 0.0 })).height(Length::Fill)))
        .padding(Padding { top: 18.0, right: 36.0, bottom: 0.0, left: 44.0 })
        .width(Length::Fill)
        .height(Length::Fill);

    let titlebar = ui::title_bar(&p, G::Gear, "Hearth Settings", None, TitleMsgs { drag: Msg::Drag, maximize: Msg::Maximize, minimize: Some(Msg::Minimize), close: Msg::Close }, rail_bg);
    let page = column![titlebar, row![rail, main].height(Length::Fill), hline(p.line), hints(a)];
    let base: Element<Msg> = container(page).width(Length::Fill).height(Length::Fill).style(ui::boxed(p.bg, 0.0, None)).into();
    let base = match crate::dialog_view(a) {
        Some((d, outside)) => ui::dialog(base, d, outside),
        None => base,
    };
    ui::edges(base, Msg::Resize)
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
    let mut r = Row::new().spacing(12.0).align_y(iced::Center);
    r = r.push(mouse_area(tb(cat.1, 34.0, p.muted)).on_press(Msg::Cat(a.cat)).interaction(mouse::Interaction::Pointer));
    let n = chain.len();
    for (i, x) in chain.into_iter().enumerate() {
        r = r.push(container(glyph(G::Chevron, p.text, 22.0, 2.4)).padding(Padding { top: 6.0, right: 0.0, bottom: 0.0, left: 0.0 }));
        if i + 1 == n {
            r = r.push(tb(x.label, 34.0, p.text));
        } else {
            r = r.push(mouse_area(tb(x.label, 34.0, p.muted)).on_press(Msg::Go(x.id)).interaction(mouse::Interaction::Pointer));
        }
    }
    r.into()
}

fn hint<'a>(p: &Pal, k: &'a str, label: &'a str) -> Element<'a, Msg> {
    let key = container(tb(k, 11.0, p.text)).center_x(Length::Fixed(if k.chars().count() > 1 { 30.0 } else { 24.0 })).center_y(Length::Fixed(24.0)).style(ui::boxed(Color::TRANSPARENT, 12.0, Some((p.text, 1.2))));
    row![key, t1(label, 14.0, p.text)].spacing(8.0).align_y(iced::Center).into()
}

fn hints(a: &App) -> Element<'_, Msg> {
    let p = a.sh.p;
    let mut r = Row::new().spacing(24.0).align_y(iced::Center).push(hint(&p, "A", "Select")).push(hint(&p, "B", "Back")).push(hint(&p, "Y", "Search"));
    if a.system.keep.is_some() {
        r = r.push(hint(&p, "X", "Keep these settings"));
    }
    r = r.push(Space::new().width(Length::Fill)).push(row![hint(&p, "LB", ""), hint(&p, "RB", "Previous / next category")].spacing(6.0)).push(hint(&p, "≡", "Close"));
    container(r).padding(Padding { top: 0.0, right: 26.0, bottom: 0.0, left: 26.0 }).center_y(Length::Fixed(52.0)).width(Length::Fill).style(ui::boxed(Pal::mix(p.bg, p.panel, 0.55), 0.0, None)).into()
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
        if n >= 40 {
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
pub fn children<'a, M: Clone + 'a>(p: &Pal, parent: &str, go: impl Fn(&'static str) -> M) -> Column<'a, M> {
    let mut c = Column::new().spacing(8.0);
    for pg in PAGES.iter().filter(|x| x.parent == parent && !x.hidden) {
        if registry::built(pg.id) {
            c = c.push(link_row(p, page_glyph(pg.id), pg.label, pg.desc, go(pg.id)));
        }
    }
    c
}
