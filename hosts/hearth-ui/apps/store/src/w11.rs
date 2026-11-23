//! The Hearth Store in the Windows 11 Store's style (approved Nov 22, 2026; mockups docs/mockups/store-win11/
//! store-w11-1…6, checklist docs/mockups/checklists/store-win11.md): the title bar with the search in the middle, the
//! left rail (Home, Apps, Hearth parts, Tests, then Library and Settings), the page on a lighter layer with a rounded
//! top-left corner, and the pages: Home, Apps, Library and Tests (the app's page, a test version's page and Settings
//! are view.rs's, drawn in this frame).

use crate::look::{self, app_icon, btn, chip, click, f, tone, z, Chip, B, E, ORANGE, PURPLE};
use crate::view;
use crate::{App, Msg, Page, State, SEARCH_ID};
use hearth_store::catalog::{Entry, Kind};
use hearth_ui::ui::{self, gap, t, tb, ts, G};
use hearth_ui::Pal;
use iced::widget::{column, container, mouse_area, row, scrollable, text_input, Column, Row, Space};
use iced::{Color, Length, Theme};

/// What an app is, for the filter pills (Apps) and the tiles' second line.
pub fn category(e: &Entry) -> &'static str {
    match e.id.as_str() {
        "command-center" | "hearth-ai" => "AI",
        "console-remote-play" | "beacon" | "capture-card" => "Gaming",
        "g15" | "usb-maker" => "Devices",
        "hearth-buddy" => "Fun",
        _ => "Tools",
    }
}

fn layer(p: &Pal) -> Color {
    Pal::mix(p.bg, p.panel, 0.42)
}

/// The page on its lighter layer with the rounded top-left corner (Windows 11's look).
pub fn frame<'a>(a: &'a App, body: E<'a, Msg>) -> E<'a, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let page = container(scrollable(container(body).padding(look::pad(z(26.0), z(44.0), z(30.0), z(44.0))).width(Length::Fill)).height(Length::Fill))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_: &Theme| container::Style {
            background: Some(iced::Background::Color(layer(&p))),
            border: iced::Border { color: tn.line, width: 1.0, radius: iced::border::Radius { top_left: z(10.0), top_right: 0.0, bottom_left: 0.0, bottom_right: 0.0 } },
            ..container::Style::default()
        });
    let main = row![rail(a), page].height(Length::Fill);
    container(column![title_bar(a), main]).width(Length::Fill).height(Length::Fill).style(look::bx(tn.page, 0.0, None)).into()
}

fn title_bar(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let name = row![gap(z(18.0), 1.0), look::mark(z(22.0)), gap(z(12.0), 1.0), t("Hearth Store", f(14.0), tn.text)].align_y(iced::Center);
    let field = text_input("Search Hearth apps, parts and test versions", &a.search)
        .id(SEARCH_ID)
        .on_input(Msg::Search)
        .size(f(15.0))
        .padding(0.0)
        .style(move |_t: &Theme, _s| text_input::Style {
            background: iced::Background::Color(Color::TRANSPARENT),
            border: iced::Border::default(),
            icon: tn.muted,
            placeholder: tn.muted,
            value: tn.text,
            selection: Pal::a(ORANGE, 0.35),
        });
    let key = container(t("Ctrl+K", f(11.0), tn.muted)).padding(look::pad(1.0, z(6.0), 1.0, z(6.0))).style(look::bx(Color::TRANSPARENT, 4.0, Some((Pal::mix(tn.line, tn.text, 0.12), 1.0))));
    let search = container(row![ui::glyph(G::Search, tn.muted, z(15.0), 1.6), field.width(Length::Fill), key].spacing(z(10.0)).align_y(iced::Center))
        .padding(look::pad(z(8.0), z(14.0), z(8.0), z(14.0)))
        .width(Length::Fixed(z(640.0)))
        .style(look::bx(Pal::a(Color::WHITE, 0.05), 8.0, Some((tn.line, 1.0))));
    let me = container(tb(initial(), f(13.0), Color::WHITE)).center_x(Length::Fixed(z(32.0))).center_y(Length::Fixed(z(32.0))).style(look::grad(135.0, Color::from_rgb8(0x5b, 0x8c, 0xff), Color::from_rgb8(0x8f, 0x5b, 0xff), z(16.0), None));
    let wbtn = |g: G, m: Msg| -> E<'static, Msg> { click(container(ui::glyph(g, tn.muted, 10.0, 1.6)).center_x(Length::Fixed(z(58.0))).center_y(Length::Fixed(z(56.0))), Some(m)) };
    let drag = |e: E<'a, Msg>| -> E<'a, Msg> { mouse_area(container(e).width(Length::Fill).center_y(Length::Fixed(z(56.0)))).on_press(Msg::Drag).on_double_click(Msg::Maximize).into() };
    container(
        row![
            drag(name.into()),
            search,
            drag(row![Space::new().width(Length::Fill), me, gap(z(26.0), 1.0)].align_y(iced::Center).into()),
            wbtn(G::Minimize, Msg::Minimize),
            wbtn(G::Maximize, Msg::Maximize),
            wbtn(G::Close, Msg::Close)
        ]
        .align_y(iced::Center),
    )
    .width(Length::Fill)
    .height(Length::Fixed(z(56.0)))
    .into()
}

fn initial() -> String {
    std::env::var("USER").ok().and_then(|u| u.chars().next()).map(|c| c.to_uppercase().to_string()).unwrap_or_else(|| "H".into())
}

/// Which rail item a page belongs to.
fn rail_of(pg: &Page, a: &App) -> &'static str {
    match pg {
        Page::Home => "home",
        Page::Apps => "apps",
        Page::BuiltIn => "parts",
        Page::Tests => "tests",
        Page::Updates | Page::Yours | Page::Work => "library",
        Page::Settings => "settings",
        Page::App(id) | Page::Repair(id) => match a.entry(id) {
            Some(e) if e.test => "tests",
            Some(e) if e.part.is_some() || e.kind == Kind::BuiltIn => "parts",
            _ if a.inst.contains_key(id) => "library",
            _ => "apps",
        },
    }
}

fn rail(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let here = rail_of(&a.page, a);
    let n_up = a.ready().len() + a.part_offers().len();
    let item = |id: &'static str, g: G, label: &'static str, pg: Page, badge: usize| -> E<'_, Msg> {
        let on = here == id;
        let bar = container(Space::new()).width(Length::Fixed(3.0)).height(Length::Fixed(z(22.0))).style(look::bx(if on { ORANGE } else { Color::TRANSPARENT }, 2.0, None));
        let mut icon: E<'_, Msg> = ui::glyph(g, tn.text, z(20.0), 1.6);
        if badge > 0 {
            let b = container(tb(badge.to_string(), f(10.0), look::INK)).padding(look::pad(0.0, z(4.0), 0.0, z(4.0))).style(look::bx(ORANGE, 8.0, None));
            icon = iced::widget::stack![container(icon).padding(look::pad(z(4.0), z(10.0), 0.0, 0.0)), container(b).align_right(Length::Fixed(z(34.0)))].into();
        }
        let label_t = t(label, f(11.0), if on { tn.text } else { tn.muted });
        let cell = container(column![icon, label_t].spacing(z(5.0)).align_x(iced::Center))
            .center_x(Length::Fixed(z(64.0)))
            .padding(look::pad(z(9.0), 0.0, z(8.0), 0.0))
            .style(look::bx(if on { Pal::a(Color::WHITE, 0.06) } else { Color::TRANSPARENT }, 8.0, None));
        click(row![bar, cell].align_y(iced::Center), Some(Msg::Go(pg)))
    };
    let mut c = Column::new().spacing(z(4.0)).align_x(iced::Center).padding(look::pad(z(6.0), 0.0, z(10.0), z(3.0)));
    c = c.push(item("home", G::Home, "Home", Page::Home, 0));
    c = c.push(item("apps", G::Grid, "Apps", Page::Apps, 0));
    c = c.push(item("parts", G::Store, "Hearth\nparts", Page::BuiltIn, 0));
    if a.prefs.test {
        c = c.push(item("tests", G::Pulse, "Tests", Page::Tests, 0));
    }
    c = c.push(Space::new().height(Length::Fill));
    c = c.push(item("library", G::List, "Library", Page::Updates, n_up));
    c = c.push(item("settings", G::Gear, "Settings", Page::Settings, 0));
    container(c).width(Length::Fixed(z(76.0))).height(Length::Fill).into()
}

/// The page title with what's on its right ("Checked 2 min ago · 3 updates", Check for updates).
fn head<'a>(a: &'a App, title: &str, big_check: bool) -> E<'a, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let n = a.ready().len() + a.part_offers().len();
    let checked = if a.checking { "Checking…".to_string() } else if big_check { format!("Last checked: {}", view::day_time_pub(a.seen.checked)) } else { format!("Checked {}{}", view::ago_pub(a, a.seen.checked), if n > 0 { format!(" · {n} update{}", if n == 1 { "" } else { "s" }) } else { String::new() }) };
    let check = btn(&p, "⟳  Check for updates", if big_check { B::Primary } else { B::Normal }, big_check, false, (!a.checking).then_some(Msg::TryAgain));
    container(row![ts(title.to_string(), f(30.0), tn.text), Space::new().width(Length::Fill), t(checked, f(13.0), tn.muted), gap(z(14.0), 1.0), check].align_y(iced::Center)).padding(look::pad(0.0, 0.0, z(18.0), 0.0)).width(Length::Fill).into()
}

/// The chip at a tile's foot: Up to date, Update, the new version, Built in, Installed, TEST, Coming.
fn foot<'a>(a: &App, e: &Entry) -> E<'a, Msg> {
    let p = a.p;
    if let Some(o) = a.part_offer(e) {
        return chip(&p, o.version.clone(), Chip::Up);
    }
    if let Some(h) = a.parts.get(&e.id) {
        return chip(&p, h.version.clone(), Chip::Up);
    }
    match a.state(e) {
        State::UpToDate(_) => chip(&p, "Up to date", Chip::Ok),
        State::Ready(..) => chip(&p, "Update", Chip::Up),
        State::BuiltIn if e.test => chip(&p, "TEST", Chip::Test),
        State::BuiltIn => chip(&p, "Built in", Chip::Built),
        State::Coming => chip(&p, "Coming", Chip::Coming),
        State::Get(_) => chip(&p, "Get", Chip::Plain),
    }
}

/// A tile (Home's Your apps): icon, name, kind, and its chip at the foot.
fn tile<'a>(a: &'a App, e: &Entry, w: f32) -> E<'a, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let kind = if e.test { "Test version".to_string() } else if e.part.is_some() || e.kind == Kind::BuiltIn { format!("Hearth part{}", if e.card.is_empty() { String::new() } else { String::new() }) } else { category(e).to_string() };
    let kind = if !e.tagline.is_empty() && e.kind == Kind::Install { format!("{} · {}", e.tagline, category(e)) } else { kind };
    let c = container(column![app_icon(e, z(64.0)), gap(1.0, z(22.0)), tb(e.name.clone(), f(14.0), tn.text), t(kind, f(12.0), tn.muted), Space::new().height(Length::Fill), foot(a, e)].spacing(z(3.0)))
        .padding(z(18.0))
        .width(Length::Fixed(w))
        .height(Length::Fixed(z(272.0)))
        .style(look::bx(look::tone(&p).card, 8.0, Some((tn.line, 1.0))));
    click(c, Some(Msg::OpenApp(e.id.clone())))
}

/// A side tile on Home (the Command Center, a test version): its colours, kicker and name.
fn side_tile<'a>(a: &'a App, e: &Entry) -> E<'a, Msg> {
    let p = a.p;
    let from = look::hex(&e.from, ORANGE);
    let to = look::hex(&e.to, ORANGE);
    let kicker: E<'a, Msg> = if e.test { chip(&p, "TEST", Chip::Test) } else { ts(e.tagline.to_uppercase(), f(12.0), Pal::mix(Color::WHITE, from, 0.3)).into() };
    let ver = a.seen.rel(&e.id).map(|r| format!(" {}", hearth_store::short_version(&r.version))).unwrap_or_default();
    let inner = column![Space::new().height(Length::Fill), kicker, tb(format!("{}{ver}", e.short), f(20.0), Color::WHITE)].spacing(z(6.0));
    let c = container(inner).padding(z(22.0)).width(Length::Fill).height(Length::Fill).style(look::grad(150.0, Pal::mix(to, Color::BLACK, 0.2), Pal::mix(from, Color::BLACK, 0.55), 8.0, Some((Pal::a(Color::WHITE, 0.12), 1.0))));
    click(c, Some(Msg::OpenApp(e.id.clone())))
}

// ------------------------------------------------------------------ 1. Home
pub fn home(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let mut c = Column::new().push(head(a, "Home", false));
    if !a.search.trim().is_empty() {
        return search_results(a, c);
    }
    let feat = view::featured(a);
    let mut sides: Vec<&Entry> = a.cat.apps.iter().filter(|e| Some(e.id.as_str()) != feat.as_ref().map(|f| f.0.id.as_str()) && (e.kind == Kind::Install || e.test) && (!e.test || a.prefs.test)).collect();
    sides.sort_by_key(|e| !e.test);
    sides.reverse();
    let sides: Vec<&Entry> = sides.into_iter().take(2).collect();
    let mut right = Column::new().spacing(z(16.0)).width(Length::Fixed(z(400.0)));
    for e in &sides {
        right = right.push(container(side_tile(a, e)).height(Length::Fixed(z(202.0))));
    }
    let big: E<Msg> = match &feat {
        Some((e, st)) => view::hero_pub(a, e, st),
        None => Space::new().into(),
    };
    c = c.push(row![container(big).width(Length::Fill).height(Length::Fixed(z(420.0))), right].spacing(z(16.0)));
    let mine: Vec<&Entry> = a.cat.apps.iter().filter(|e| (e.kind == Kind::Install && a.inst.contains_key(&e.id)) || (e.kind == Kind::BuiltIn && !e.test && a.here.contains(&e.id)) || a.parts.contains_key(&e.id)).collect();
    c = c.push(container(click(row![ts("Your apps", f(20.0), tn.text), t("  ›", f(20.0), tn.muted)].align_y(iced::Center), Some(Msg::Go(Page::Updates)))).padding(look::pad(z(34.0), 0.0, z(14.0), 0.0)));
    let mut r = Row::new().spacing(z(16.0));
    for e in mine.iter().take(8) {
        r = r.push(tile(a, e, z(205.0)));
    }
    c = c.push(scrollable(r).direction(scrollable::Direction::Horizontal(scrollable::Scrollbar::new().width(0.0).scroller_width(0.0))));
    c.into()
}

fn search_results<'a>(a: &'a App, mut c: Column<'a, Msg>) -> E<'a, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let q = a.search.trim().to_lowercase();
    let hits: Vec<&Entry> = a.cat.apps.iter().filter(|e| (!e.test || a.prefs.test) && (e.name.to_lowercase().contains(&q) || e.card.to_lowercase().contains(&q) || e.tagline.to_lowercase().contains(&q))).collect();
    c = c.push(t(format!("Results for “{}”", a.search.trim()), f(16.0), tn.muted));
    if hits.is_empty() {
        c = c.push(container(t("Nothing here by that name.", f(15.0), tn.muted)).padding(look::pad(z(14.0), 0.0, 0.0, 0.0)));
    }
    c = c.push(gap(1.0, z(14.0))).push(cards(a, &hits));
    c.into()
}

// ------------------------------------------------------------------ 2. Apps
fn button_for<'a>(a: &'a App, e: &Entry) -> E<'a, Msg> {
    let p = a.p;
    if a.part_offer(e).is_some() {
        return btn(&p, "Update", B::Primary, false, false, Some(Msg::PartUpdate(e.id.clone())));
    }
    match a.state(e) {
        State::Get(_) => btn(&p, "Get", B::Primary, false, false, if a.offline() { None } else { view::primary(e, &a.state(e)) }),
        State::Ready(..) => btn(&p, "Update", B::Primary, false, false, view::primary(e, &a.state(e))),
        State::UpToDate(_) => btn(&p, "Open", B::Normal, false, false, Some(Msg::Launch(e.id.clone()))),
        State::BuiltIn if e.test => btn(&p, "See it", B::Normal, false, false, Some(Msg::OpenApp(e.id.clone()))),
        State::BuiltIn => chip(&p, "Built in", Chip::Built),
        State::Coming => chip(&p, "Coming", Chip::Coming),
    }
}

fn cards<'a>(a: &'a App, list: &[&Entry]) -> E<'a, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let mut col = Column::new().spacing(z(16.0));
    for chunk in list.chunks(4) {
        let mut r = Row::new().spacing(z(16.0));
        for e in chunk {
            let card = container(
                column![
                    row![app_icon(e, z(56.0)), column![tb(e.name.clone(), f(15.0), tn.text), t(format!("{} · Dragongamer9", category(e)), f(12.0), tn.muted)].spacing(z(3.0))].spacing(z(14.0)).align_y(iced::Center),
                    container(t(e.card.clone(), f(13.0), Pal::mix(tn.text, tn.muted, 0.3))).height(Length::Fixed(z(56.0))),
                    row![Space::new().width(Length::Fill), button_for(a, e)].align_y(iced::Center)
                ]
                .spacing(z(14.0)),
            )
            .padding(z(18.0))
            .width(Length::FillPortion(1))
            .style(look::bx(tn.card, 8.0, Some((tn.line, 1.0))));
            r = r.push(click(card, Some(Msg::OpenApp(e.id.clone()))));
        }
        for _ in chunk.len()..4 {
            r = r.push(Space::new().width(Length::FillPortion(1)));
        }
        col = col.push(r);
    }
    col.into()
}

pub fn apps(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let mut c = Column::new().push(head(a, "Apps", false));
    if !a.search.trim().is_empty() {
        return search_results(a, c);
    }
    let filter = a.filter.clone();
    let mut pills = Row::new().spacing(z(10.0)).align_y(iced::Center);
    for cat in ["All", "AI", "Gaming", "Devices", "Tools", "Fun"] {
        let on = filter == cat || (filter.is_empty() && cat == "All");
        let pill = container(if on { tb(cat, f(14.0), look::INK) } else { t(cat, f(14.0), tn.text) })
            .padding(look::pad(z(6.0), z(16.0), z(6.0), z(16.0)))
            .style(look::bx(if on { ORANGE } else { Pal::a(Color::WHITE, 0.03) }, 16.0, Some((if on { Color::TRANSPARENT } else { Pal::mix(tn.line, tn.text, 0.1) }, 1.0))));
        pills = pills.push(click(pill, Some(Msg::Filter(cat.to_string()))));
    }
    pills = pills.push(Space::new().width(Length::Fill)).push(chip(&p, "Sort: Name", Chip::Plain));
    c = c.push(pills).push(gap(1.0, z(22.0)));
    if filter.is_empty() || filter == "All" || filter == "AI" {
        let banner = container(
            row![
                column![ts("YOUR AI TEAM, ON YOUR OWN PC", f(12.0), PURPLE), tb("Hearth AI and the Command Center", f(24.0), Color::WHITE), t("Ithil lives on your PC; the Command Center gets Claude, ChatGPT and Gemini working on one task together.", f(14.0), Pal::mix(Color::WHITE, tn.muted, 0.35))].spacing(z(6.0)).width(Length::Fill),
                btn(&p, "See them", B::Normal, false, false, Some(Msg::Filter("AI".into())))
            ]
            .align_y(iced::Center)
            .spacing(z(20.0)),
        )
        .padding(look::pad(z(24.0), z(28.0), z(24.0), z(28.0)))
        .width(Length::Fill)
        .style(look::grad(110.0, Color::from_rgb8(0x2a, 0x1d, 0x4a), Color::from_rgb8(0x14, 0x2a, 0x55), 8.0, None));
        c = c.push(banner).push(gap(1.0, z(22.0)));
    }
    let list: Vec<&Entry> = a.cat.apps.iter().filter(|e| !e.test && e.part.is_none() && (filter.is_empty() || filter == "All" || category(e) == filter)).collect();
    c = c.push(cards(a, &list));
    let flat = container(
        row![
            column![tb("Looking for other apps?", f(15.0), tn.text), t("Flathub's apps are in Settings › Apps › App store", f(13.0), tn.muted)].spacing(z(3.0)).width(Length::Fill),
            btn(&p, "Open App store ›", B::Normal, false, false, Some(Msg::OpenFlathub))
        ]
        .align_y(iced::Center),
    )
    .padding(z(18.0))
    .width(Length::Fill)
    .style(look::bx(tn.card, 8.0, Some((tn.line, 1.0))));
    c.push(gap(1.0, z(22.0))).push(flat).into()
}

// ------------------------------------------------------------------ 4. Library
pub fn library(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let mut c = Column::new().push(head(a, "Library", true));
    // Updates: how many, Pause, Update all, then each (from → to, what's new, size, Update; the downloading one)
    let ups = a.ready();
    let parts = a.part_offers();
    let n = ups.len() + parts.len();
    let busy = usize::from(a.job.is_some()) + a.part_busy.len();
    let mut box_ = Column::new();
    let pause: E<Msg> = if a.job.is_some() { btn(&p, "Pause", B::Normal, false, false, Some(Msg::Cancel)) } else { Space::new().into() };
    box_ = box_.push(
        container(
            row![
                column![tb("Updates", f(17.0), tn.text), t(if n == 0 { "Everything is up to date".to_string() } else { format!("{n} ready{}", if busy > 0 { format!(" · {busy} updating") } else { String::new() }) }, f(13.0), tn.muted)].spacing(z(2.0)).width(Length::Fill),
                pause,
                gap(z(10.0), 1.0),
                btn(&p, "Update all", B::Primary, false, false, (n > 0).then_some(Msg::UpdateAll))
            ]
            .align_y(iced::Center),
        )
        .padding(look::pad(z(16.0), z(18.0), z(16.0), z(18.0))),
    );
    let line_ = || -> E<'_, Msg> { ui::hline(tn.line) };
    for (e, pt) in &parts {
        let from = a.parts.get(&e.id).map(|h| h.version.clone()).unwrap_or_else(|| a.hearth.clone());
        let what = pt.notes.first().cloned().unwrap_or_default();
        let right: E<Msg> = if a.part_busy.contains(&e.id) { t("Updating…", f(13.0), tn.muted).into() } else { row![t(hearth_store::mb(pt.size, true), f(13.0), tn.muted), gap(z(14.0), 1.0), btn(&p, "Update", B::Primary, false, false, Some(Msg::PartUpdate(e.id.clone())))].align_y(iced::Center).into() };
        box_ = box_.push(line_()).push(lrow(a, e, format!("{from} → {} · Hearth part{}", pt.version, if what.is_empty() { String::new() } else { format!(" · {what}") }), right));
    }
    for (e, i, r) in &ups {
        let what = r.notes.first().cloned().unwrap_or_default();
        let st = a.state(e);
        let right: E<Msg> = match &a.job {
            Some(j) if j.id == e.id => row![
                column![
                    look::progress(&p, if j.total > 0 { j.got as f32 / j.total as f32 } else { j.demo.unwrap_or(0.0) }, z(4.0), Length::Fixed(z(220.0))),
                    t(format!("Downloading · {} of {}", hearth_store::mb(j.got, false), hearth_store::mb(j.total.max(r.size), false)), f(12.0), tn.muted)
                ]
                .spacing(z(5.0))
                .align_x(iced::Right),
                gap(z(14.0), 1.0),
                btn(&p, "✕", B::Normal, false, false, Some(Msg::Cancel))
            ]
            .align_y(iced::Center)
            .into(),
            _ => row![t(hearth_store::mb(r.size, true), f(13.0), tn.muted), gap(z(14.0), 1.0), btn(&p, if e.self_updates { "Open to update" } else { "Update" }, B::Primary, false, false, view::primary(e, &st))].align_y(iced::Center).into(),
        };
        box_ = box_.push(line_()).push(lrow(a, e, format!("{} → {}{}", i.version, r.version, if what.is_empty() { String::new() } else { format!(" · {what}") }), right));
    }
    c = c.push(container(box_).width(Length::Fill).style(look::bx(tn.card, 8.0, Some((tn.line, 1.0)))));
    // Your apps: Apps / Hearth parts / Tests, Sort by
    let tab = a.lib_tab.clone();
    let mut pills = Row::new().spacing(z(10.0)).align_y(iced::Center);
    for (id, label) in [("apps", "Apps"), ("parts", "Hearth parts"), ("tests", "Tests")] {
        if id == "tests" && !a.prefs.test {
            continue;
        }
        let on = tab == id || (tab.is_empty() && id == "apps");
        let pill = container(if on { tb(label, f(14.0), look::INK) } else { t(label, f(14.0), tn.text) })
            .padding(look::pad(z(6.0), z(16.0), z(6.0), z(16.0)))
            .style(look::bx(if on { ORANGE } else { Color::TRANSPARENT }, 16.0, Some((if on { Color::TRANSPARENT } else { Pal::mix(tn.line, tn.text, 0.1) }, 1.0))));
        pills = pills.push(click(pill, Some(Msg::LibTab(id.to_string()))));
    }
    pills = pills.push(Space::new().width(Length::Fill)).push(chip(&p, "Sort by: Date updated ▾", Chip::Plain));
    c = c.push(container(pills).padding(look::pad(z(28.0), 0.0, z(14.0), 0.0)));
    let tab = if tab.is_empty() { "apps".to_string() } else { tab };
    let list: Vec<&Entry> = a
        .cat
        .apps
        .iter()
        .filter(|e| match tab.as_str() {
            "tests" => e.test,
            "parts" => !e.test && (e.part.is_some() || (e.kind == Kind::BuiltIn && ["mantel", "files", "settings", "task-manager", "capture", "family", "server", "system-restore", "reliability", "resmon"].contains(&e.id.as_str()))),
            _ => !e.test && ((e.kind == Kind::Install && a.inst.contains_key(&e.id)) || (e.kind == Kind::BuiltIn && e.part.is_none() && a.here.contains(&e.id))),
        })
        .collect();
    let mut mine = Column::new();
    for (k, e) in list.iter().enumerate() {
        if k > 0 {
            mine = mine.push(ui::hline(tn.line));
        }
        let sub = match a.state(e) {
            State::UpToDate(i) | State::Ready(i, _) => format!("{}{}", i.version, if e.self_updates { " · updates itself" } else { "" }),
            _ if a.parts.contains_key(&e.id) => format!("Hearth part · {}", a.parts[&e.id].version),
            _ => format!("Built into {}", if a.hearth.is_empty() { "Hearth OS".to_string() } else { format!("Hearth OS {}", a.hearth) }),
        };
        let right: E<Msg> = row![
            btn(&p, "Open", B::Normal, false, false, (e.kind != Kind::Coming).then(|| Msg::Launch(e.id.clone()))),
            gap(z(10.0), 1.0),
            btn(&p, "⋯", B::Normal, false, false, Some(Msg::OpenApp(e.id.clone())))
        ]
        .align_y(iced::Center)
        .into();
        mine = mine.push(lrow(a, e, sub, right));
    }
    if list.is_empty() {
        mine = mine.push(container(t("Nothing here yet.", f(14.0), tn.muted)).padding(z(18.0)));
    }
    c = c.push(container(mine).width(Length::Fill).style(look::bx(tn.card, 8.0, Some((tn.line, 1.0)))));
    c = c.push(container(row![Space::new().width(Length::Fill), t(format!("This PC: {}", view::this_pc_pub(a)), f(12.0), tn.muted)]).padding(look::pad(z(18.0), 0.0, 0.0, 0.0)));
    c.into()
}

fn lrow<'a>(a: &'a App, e: &Entry, sub: String, right: E<'a, Msg>) -> E<'a, Msg> {
    let tn = tone(&a.p);
    let r = row![app_icon(e, z(44.0)), column![tb(e.name.clone(), f(15.0), tn.text), t(sub, f(13.0), tn.muted)].spacing(z(3.0)).width(Length::Fill), right].spacing(z(16.0)).align_y(iced::Center);
    click(container(r).padding(look::pad(z(14.0), z(18.0), z(14.0), z(18.0))).width(Length::Fill), Some(Msg::OpenApp(e.id.clone())))
}

// ------------------------------------------------------------------ Tests (the rail's, with Get test versions on)
pub fn tests(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let mut c = Column::new().push(head(a, "Tests", false));
    c = c.push(t("Test versions of Hearth's own apps and parts: they may change from week to week, and your usual one is always one click away.", f(14.0), tn.muted)).push(gap(1.0, z(18.0)));
    let list: Vec<&Entry> = a.cat.apps.iter().filter(|e| e.test).collect();
    if list.is_empty() {
        c = c.push(t("No test versions right now.", f(14.0), tn.muted));
    }
    c.push(cards(a, &list)).into()
}

