//! The Hearth Store in the Windows 11 Store's style (approved Nov 22, 2026; mockups docs/mockups/store-win11/
//! store-w11-1…6, checklist docs/mockups/checklists/store-win11.md): the title bar with the search in the middle, the
//! left rail (Home, Apps, Hearth parts, Tests, then Library and Settings), the page on a lighter layer with a rounded
//! top-left corner, and the pages: Home, Apps, Library and Tests (the app's page, a test version's page and Settings
//! are view.rs's, drawn in this frame).

use crate::look::{self, app_icon, btn, bullets, chip, click, f, tone, z, Chip, B, E, ORANGE, PURPLE};
use crate::view;
use crate::{App, Msg, Page, State, SEARCH_ID};
use hearth_store::catalog::{Entry, Kind};
use hearth_ui::ui::{self, gap, t, tb, ts, G};
use hearth_ui::Pal;
use iced::widget::{canvas, column, container, mouse_area, row, scrollable, text_input, Column, Row, Space};
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

fn title_bar<'a>(a: &'a App) -> E<'a, Msg> {
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
        Page::Updates | Page::Yours | Page::Work | Page::System => "library",
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
    let n_up = a.ready().len() + a.parts_ready();
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
    let n = a.ready().len() + a.parts_ready();
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
fn tile<'a>(a: &'a App, e: &Entry) -> E<'a, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let kind = if e.test { "Test version".to_string() } else if e.part.is_some() || e.kind == Kind::BuiltIn { format!("Hearth part{}", if e.card.is_empty() { String::new() } else { String::new() }) } else { category(e).to_string() };
    let kind = if !e.tagline.is_empty() && e.kind == Kind::Install { format!("{} · {}", e.tagline, category(e)) } else { kind };
    let c = container(column![app_icon(e, z(64.0)), gap(1.0, z(22.0)), tb(e.name.clone(), f(14.0), tn.text), t(kind, f(12.0), tn.muted), Space::new().height(Length::Fill), foot(a, e)].spacing(z(3.0)))
        .padding(z(18.0))
        .width(Length::Fill)
        .height(Length::Fixed(z(272.0)))
        .style(look::bx(look::tone(&p).card, 8.0, Some((tn.line, 1.0))));
    click(c, Some(Msg::OpenApp(e.id.clone())))
}

/// A side tile on Home (the Command Center, a test version): its colours and picture, kicker and name.
fn side_tile<'a>(a: &'a App, e: &Entry) -> E<'a, Msg> {
    let p = a.p;
    let from = look::hex(&e.from, ORANGE);
    let to = look::hex(&e.to, ORANGE);
    let kicker: E<'a, Msg> = if e.test { chip_test() } else { ts(if e.tagline.is_empty() { category(e).to_uppercase() } else { e.tagline.split([':', ',', '.']).next().unwrap_or("").trim().to_uppercase() }, f(12.0), Pal::mix(Color::WHITE, from, 0.3)).into() };
    let ver = a.seen.rel(&e.id).map(|r| format!(" {}", hearth_store::short_version(&r.version))).or_else(|| a.inst.get(&e.id).map(|i| format!(" {}", hearth_store::short_version(&i.version)))).unwrap_or_default();
    let name = if e.test { e.short.trim_end_matches(" (test)").to_string() } else { format!("{}{ver}", e.short) };
    let words = column![Space::new().height(Length::Fill), kicker, tb(name, f(20.0), Color::WHITE)].spacing(z(6.0));
    let art = iced::widget::canvas(Pic { kind: pic_of(e), from, to }).width(Length::Fill).height(Length::Fill);
    let inner = iced::widget::stack![art, container(words).padding(z(22.0)).width(Length::Fill).height(Length::Fill)];
    let c = container(inner).width(Length::Fill).height(Length::Fill).clip(true).style(look::grad(150.0, Pal::mix(to, Color::BLACK, 0.25), Pal::mix(from, Color::BLACK, 0.55), 8.0, Some((Pal::a(Color::WHITE, 0.12), 1.0))));
    let _ = p;
    click(c, Some(Msg::OpenApp(e.id.clone())))
}

/// The yellow TEST chip, filled (store-w11-1's side tile, store-w11-5's notice).
fn chip_test<'a>() -> E<'a, Msg> {
    container(tb("TEST", f(12.0), look::INK).wrapping(iced::widget::text::Wrapping::None)).padding(look::pad(z(3.0), z(10.0), z(3.0), z(10.0))).style(look::bx(look::WARN, 11.0, None)).into()
}

// ------------------------------------------------------------------ 1. Home
/// The featured app, big (store-w11-1): the carousel's dots, its icon, NEW VERSION and its name, its line, Update and
/// See details, and its picture filling the right half.
fn hero<'a>(a: &'a App, e: &Entry, st: &State) -> E<'a, Msg> {
    let p = a.p;
    let from = look::hex(&e.from, PURPLE);
    let to = look::hex(&e.to, PURPLE);
    let (kicker, title, button) = match st {
        State::Ready(_, r) => ("NEW VERSION", format!("{} {}", e.name, hearth_store::short_version(&r.version)), view::open_label(e, st)),
        State::Get(r) => ("NEW IN THE STORE", format!("{} {}", e.name, hearth_store::short_version(&r.version)), "Get"),
        _ => ("YOUR APP", e.name.clone(), "Open"),
    };
    let line = if e.hero.is_empty() {
        match st {
            State::Ready(_, r) | State::Get(r) => r.notes.first().cloned().unwrap_or_else(|| e.about.clone()),
            _ => e.about.clone(),
        }
    } else {
        e.hero.clone()
    };
    let line = if e.tagline.is_empty() || line.starts_with(&e.tagline) { line } else { format!("{}. {line}", e.tagline.trim_end_matches('.')) };
    let dots = row![
        container(Space::new()).width(Length::Fixed(z(22.0))).height(Length::Fixed(z(4.0))).style(look::bx(Color::WHITE, 2.0, None)),
        container(Space::new()).width(Length::Fixed(z(6.0))).height(Length::Fixed(z(4.0))).style(look::bx(Pal::a(Color::WHITE, 0.35), 2.0, None)),
        container(Space::new()).width(Length::Fixed(z(6.0))).height(Length::Fixed(z(4.0))).style(look::bx(Pal::a(Color::WHITE, 0.35), 2.0, None)),
    ]
    .spacing(z(6.0));
    let left = column![
        dots,
        Space::new().height(Length::Fill),
        row![app_icon(e, z(72.0)), column![ts(kicker, f(13.0), PURPLE), tb(title, f(40.0), Color::WHITE)].spacing(z(2.0))].spacing(z(16.0)).align_y(iced::Center),
        container(t(line, f(17.0), Pal::mix(Color::WHITE, tone(&p).muted, 0.25))).max_width(z(620.0)).padding(look::pad(z(18.0), 0.0, z(22.0), 0.0)),
        row![btn(&p, button, B::Primary, false, false, view::primary(e, st)), btn(&p, "See details", B::Normal, false, false, Some(Msg::OpenApp(e.id.clone())))].spacing(z(12.0)),
    ];
    let left = container(left).padding(look::pad(z(24.0), z(44.0), z(44.0), z(44.0))).width(Length::FillPortion(55)).height(Length::Fill);
    let art = container(iced::widget::canvas(look::Brain).width(Length::Fill).height(Length::Fill)).width(Length::FillPortion(45)).height(Length::Fill);
    let inner = container(row![left, art]).width(Length::Fill).height(Length::Fill).clip(true).style(look::grad(110.0, Pal::mix(Color::from_rgb8(0x1c, 0x14, 0x2c), from, 0.10), Pal::mix(Color::from_rgb8(0x24, 0x14, 0x30), to, 0.14), 8.0, Some((Pal::a(Color::WHITE, 0.08), 1.0))));
    click(inner, Some(Msg::OpenApp(e.id.clone())))
}

/// Which two apps get the side tiles (store-w11-1): one of yours that isn't featured, then a test version (with Get
/// test versions on), else the next app.
fn side_picks<'a>(a: &'a App, feat: Option<&str>) -> Vec<&'a Entry> {
    let ok = |e: &&Entry| Some(e.id.as_str()) != feat;
    let mut out: Vec<&Entry> = Vec::new();
    if let Some(e) = a.cat.apps.iter().filter(ok).find(|e| e.kind == Kind::Install && a.inst.contains_key(&e.id)) {
        out.push(e);
    }
    if a.prefs.test {
        if let Some(e) = a.cat.apps.iter().filter(ok).find(|e| e.test) {
            out.push(e);
        }
    }
    for e in a.cat.apps.iter().filter(ok).filter(|e| e.kind == Kind::Install) {
        if out.len() >= 2 {
            break;
        }
        if !out.iter().any(|x| x.id == e.id) {
            out.push(e);
        }
    }
    out
}

pub fn home(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let mut c = Column::new().push(head(a, "Home", false));
    if !a.search.trim().is_empty() {
        return search_results(a, c);
    }
    let feat = view::featured(a);
    let mut right = Column::new().spacing(z(16.0)).width(Length::FillPortion(30)).height(Length::Fill);
    for e in side_picks(a, feat.as_ref().map(|f| f.0.id.as_str())) {
        right = right.push(container(side_tile(a, e)).height(Length::FillPortion(1)));
    }
    let big: E<Msg> = match &feat {
        Some((e, st)) => hero(a, e, st),
        None => Space::new().into(),
    };
    c = c.push(row![container(big).width(Length::FillPortion(100)).height(Length::Fill), right].spacing(z(16.0)).height(Length::Fixed(z(420.0))));
    let mine: Vec<&Entry> = a.cat.apps.iter().filter(|e| !e.test && ((e.kind == Kind::Install && a.inst.contains_key(&e.id)) || (e.kind == Kind::BuiltIn && a.here.contains(&e.id)) || a.parts.contains_key(&e.id))).collect();
    c = c.push(container(click(row![ts("Your apps", f(20.0), tn.text), t("  ›", f(20.0), tn.muted)].align_y(iced::Center), Some(Msg::Go(Page::Updates)))).padding(look::pad(z(34.0), 0.0, z(14.0), 0.0)));
    let mut r = Row::new().spacing(z(16.0));
    for e in mine.iter().take(8) {
        r = r.push(container(tile(a, e)).width(Length::FillPortion(1)));
    }
    for _ in mine.len().min(8)..8 {
        r = r.push(Space::new().width(Length::FillPortion(1)));
    }
    c.push(r).into()
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
    let list: Vec<&Entry> = a.cat.apps.iter().filter(|e| !e.test && e.part.is_none() && e.group_of() == "apps" && (filter.is_empty() || filter == "All" || category(e) == filter)).collect();
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
    if let Some(u) = updating(a) {
        return u;
    }
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
    // This PC's Hearth OS: Library › Hearth OS (store-everything/store-2), the system underneath and its updates.
    c = c.push(container(row![Space::new().width(Length::Fill), click(t(format!("This PC: {} ›", view::this_pc_pub(a)), f(12.0), tn.muted), Some(Msg::Go(Page::System)))]).padding(look::pad(z(18.0), 0.0, 0.0, 0.0)));
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


// ------------------------------------------------------------------ the pictures behind tiles and pages
/// What's drawn behind an app's side tile and the top of its page (store-w11-1, -3, -5): the AI team round one task
/// (the Command Center), Ithil's brain (Hearth AI), a desktop with its bar and dock (Mantel and its test), or soft
/// rings in the app's colours.
fn pic_of(e: &Entry) -> &'static str {
    match e.id.as_str() {
        "command-center" => "orbit",
        "hearth-ai" => "brain",
        id if id.contains("mantel") => "desk",
        _ => "rings",
    }
}

struct Pic {
    kind: &'static str,
    from: Color,
    to: Color,
}

impl canvas::Program<Msg> for Pic {
    type State = canvas::Cache;
    fn draw(&self, cache: &canvas::Cache, renderer: &iced::Renderer, th: &Theme, b: iced::Rectangle, cur: iced::mouse::Cursor) -> Vec<canvas::Geometry> {
        use canvas::{Path, Stroke, Text};
        use iced::Point;
        if self.kind == "brain" {
            return <look::Brain as canvas::Program<Msg>>::draw(&look::Brain, cache, renderer, th, b, cur);
        }
        vec![cache.draw(renderer, b.size(), |fr: &mut canvas::Frame| {
            let (w, h) = (b.width, b.height);
            match self.kind {
                "orbit" => {
                    // One task in the middle, the team round it on a dashed ring (on the right of the tile or page).
                    let c = Point::new(w * 0.66, h * 0.5);
                    let r = (h * 0.40).min(w * 0.28);
                    let ring = Stroke::default().with_color(Pal::a(Color::WHITE, 0.18)).with_width(1.2);
                    for k in 0..48 {
                        if k % 2 == 0 {
                            let a0 = k as f32 / 48.0 * std::f32::consts::TAU;
                            let a1 = (k + 1) as f32 / 48.0 * std::f32::consts::TAU;
                            fr.stroke(&Path::line(Point::new(c.x + r * a0.cos(), c.y + r * a0.sin()), Point::new(c.x + r * a1.cos(), c.y + r * a1.sin())), ring);
                        }
                    }
                    let pill = Path::rounded_rectangle(Point::new(c.x - r * 0.5, c.y - r * 0.2), iced::Size::new(r, r * 0.4), (r * 0.08).into());
                    fr.fill(&pill, Color::from_rgb8(0x2b, 0x5c, 0xe6));
                    fr.fill_text(Text { content: "One task".into(), position: c, color: Color::WHITE, size: (r * 0.16).max(11.0).into(), align_x: iced::widget::text::Alignment::Center, align_y: iced::alignment::Vertical::Center, ..Text::default() });
                    for (name, ang, col) in [("Ithil", -2.0f32, Color::from_rgb8(0x8b, 0x6c, 0xf0)), ("Claude", -0.55, Color::from_rgb8(0xd9, 0x77, 0x57)), ("Gemini", 2.55, Color::from_rgb8(0x4f, 0x8c, 0xf0)), ("ChatGPT", 1.15, Color::from_rgb8(0x3f, 0xb3, 0x8a))] {
                        let q = Point::new(c.x + r * ang.cos(), c.y + r * ang.sin());
                        let rr = r * 0.25;
                        fr.fill(&Path::circle(q, rr), Pal::a(col, 0.28));
                        fr.stroke(&Path::circle(q, rr), Stroke::default().with_color(Pal::a(col, 0.9)).with_width(1.5));
                        fr.fill_text(Text { content: name.into(), position: q, color: Pal::a(Color::WHITE, 0.85), size: (rr * 0.42).max(10.0).into(), align_x: iced::widget::text::Alignment::Center, align_y: iced::alignment::Vertical::Center, ..Text::default() });
                    }
                }
                "desk" => {
                    // A desktop: the bar with its orange button, a window, the dock's coloured icons.
                    let x0 = w * 0.42;
                    let dw = w - x0 - w * 0.04;
                    fr.fill(&Path::rounded_rectangle(Point::new(x0, h * 0.10), iced::Size::new(dw, h * 0.11), (h * 0.055).into()), Pal::a(Color::WHITE, 0.08));
                    fr.fill(&Path::circle(Point::new(x0 + h * 0.06, h * 0.155), h * 0.035), self.from);
                    fr.fill(&Path::rounded_rectangle(Point::new(x0 + dw * 0.28, h * 0.32), iced::Size::new(dw * 0.44, h * 0.34), (h * 0.03).into()), Pal::a(Color::WHITE, 0.07));
                    fr.stroke(&Path::rounded_rectangle(Point::new(x0 + dw * 0.28, h * 0.32), iced::Size::new(dw * 0.44, h * 0.34), (h * 0.03).into()), Stroke::default().with_color(Pal::a(Color::WHITE, 0.12)).with_width(1.0));
                    let dock = [Color::from_rgb8(0xe8, 0x83, 0x4a), Color::from_rgb8(0x4f, 0x8c, 0xf0), Color::from_rgb8(0x2f, 0xb8, 0x8f), Color::from_rgb8(0xf2, 0xc1, 0x4e), Color::from_rgb8(0xa7, 0x8b, 0xfa)];
                    let dx = x0 + dw * 0.30;
                    let step = dw * 0.4 / dock.len() as f32;
                    fr.fill(&Path::rounded_rectangle(Point::new(dx - step * 0.4, h * 0.76), iced::Size::new(step * (dock.len() as f32 + 0.3), h * 0.10), (h * 0.05).into()), Pal::a(Color::WHITE, 0.08));
                    for (k, c) in dock.iter().enumerate() {
                        fr.fill(&Path::circle(Point::new(dx + step * (k as f32 + 0.25), h * 0.81), h * 0.03), *c);
                    }
                }
                _ => {
                    for k in 0..4 {
                        let c = Point::new(w * 0.78, h * 0.45);
                        fr.stroke(&Path::circle(c, h * (0.18 + 0.12 * k as f32)), Stroke::default().with_color(Pal::a(Pal::mix(self.from, self.to, k as f32 / 3.0), 0.22)).with_width(1.4));
                    }
                }
            }
        })]
    }
}

// ------------------------------------------------------------------ 3. An app's page
/// A number and what it is under it (3.15.0 Installed · 129 MB Download · Stable Channel).
fn stat<'a>(tn: &look::Tone, v: String, k: &str) -> E<'a, Msg> {
    column![tb(v, f(18.0), tn.text), t(k.to_string(), f(12.0), tn.muted)].spacing(z(1.0)).into()
}

/// A line with a small mark in front ("Checked with the release's SHA-256 list").
fn note_line<'a>(tn: &look::Tone, g: G, s: impl Into<String>) -> E<'a, Msg> {
    row![ui::glyph(g, tn.muted, z(14.0), 1.5), t(s.into(), f(13.0), tn.muted)].spacing(z(12.0)).align_y(iced::Center).into()
}

/// The big box on a page's left: the icon, name, who made it, the buttons, the numbers and the notes under them.
fn side_card<'a>(a: &'a App, body: Column<'a, Msg>) -> E<'a, Msg> {
    let tn = tone(&a.p);
    container(body.spacing(z(4.0))).padding(z(28.0)).width(Length::Fixed(z(420.0))).style(look::bx(Pal::mix(tn.card, Color::BLACK, 0.08), 8.0, Some((tn.line, 1.0)))).into()
}

/// A wide button (Open, Get test version).
fn wide<'a>(a: &'a App, label: &str, k: B, m: Option<Msg>) -> E<'a, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let enabled = m.is_some();
    let (bg, line, fg) = match k {
        B::Primary => (ORANGE, ORANGE, look::INK),
        _ => (Pal::a(Color::WHITE, 0.05), Pal::mix(tn.line, tn.text, 0.1), tn.text),
    };
    let (bg, fg) = if enabled { (bg, fg) } else { (Pal::a(bg, 0.4), Pal::a(fg, 0.5)) };
    click(container(tb(label.to_string(), f(16.0), fg)).center_x(Length::Fill).padding(look::pad(z(10.0), 0.0, z(10.0), 0.0)).style(look::bx(bg, 6.0, Some((line, 1.0)))), m)
}

/// The art across the top of a page, behind its right side (the app's own colours and picture).
fn backdrop<'a>(e: &Entry, h: f32) -> E<'a, Msg> {
    let from = look::hex(&e.from, ORANGE);
    let to = look::hex(&e.to, ORANGE);
    let art = iced::widget::canvas(Pic { kind: pic_of(e), from, to }).width(Length::Fill).height(Length::Fill);
    container(art).width(Length::Fill).height(Length::Fixed(h)).style(look::grad(180.0, Pal::mix(from, Color::BLACK, 0.55), Pal::a(Pal::mix(to, Color::BLACK, 0.75), 0.0), 0.0, None)).into()
}

/// A sketch of an app's window, for Screenshots when no picture of it is kept yet (the mockup's three).
fn sketch<'a>(tn: &look::Tone, c: Color) -> E<'a, Msg> {
    let bar = |w: f32| -> E<'a, Msg> { container(Space::new()).width(Length::Fixed(z(w))).height(Length::Fixed(z(7.0))).style(look::bx(Pal::a(Color::WHITE, 0.08), 3.0, None)).into() };
    let blk = |w: Length, h: f32, col: Color| -> E<'a, Msg> { container(Space::new()).width(w).height(Length::Fixed(z(h))).style(look::bx(col, 6.0, None)).into() };
    let side = container(column![bar(60.0), bar(76.0), bar(88.0), bar(60.0), bar(76.0)].spacing(z(16.0))).padding(z(16.0)).width(Length::FillPortion(25)).height(Length::Fill).style(look::bx(Pal::a(Color::BLACK, 0.15), 0.0, None));
    let main = container(column![blk(Length::Fill, 60.0, Pal::a(c, 0.55)), blk(Length::FillPortion(1), 42.0, Pal::a(Color::WHITE, 0.05)), blk(Length::FillPortion(1), 42.0, Pal::a(Color::WHITE, 0.05))].spacing(z(16.0)))
        .padding(z(28.0))
        .width(Length::FillPortion(75))
        .height(Length::Fill);
    container(row![side, main]).width(Length::Fill).height(Length::Fixed(z(236.0))).clip(true).style(look::bx(Pal::mix(tn.card, c, 0.12), 6.0, Some((tn.line, 1.0)))).into()
}

pub fn app_page<'a>(a: &'a App, e: &Entry) -> E<'a, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let st = a.state(e);
    let rel = match &st {
        State::Get(r) | State::Ready(_, r) => Some(r.clone()),
        _ => a.seen.rel(&e.id),
    };
    // The card on the left
    let mut l = Column::new().push(app_icon(e, z(112.0))).push(gap(1.0, z(18.0)));
    l = l.push(tb(e.name.clone(), f(28.0), tn.text)).push(t(e.author.clone(), f(15.0), look::hex("#8fb2ff", ORANGE)));
    l = l.push(gap(1.0, z(18.0)));
    let main: E<Msg> = match &st {
        State::BuiltIn => wide(a, "Open", B::Primary, Some(Msg::Launch(e.id.clone()))),
        State::Coming => wide(a, "Coming", B::Normal, None),
        _ => wide(a, match &st { State::Get(_) => "Get", _ => view::open_label(e, &st) }, B::Primary, if a.offline() && matches!(st, State::Get(_) | State::Ready(..)) && !e.self_updates { None } else { view::primary(e, &st) }),
    };
    l = l.push(main).push(gap(1.0, z(10.0)));
    let more: E<Msg> = if a.inst.contains_key(&e.id) { btn(&p, "⋯", B::Normal, false, a.menu, Some(Msg::Menu)) } else { Space::new().into() };
    l = l.push(row![container(wide(a, "⟳  Check for updates", B::Normal, (!a.checking).then_some(Msg::TryAgain))).width(Length::Fill), more].spacing(z(8.0)).align_y(iced::Center));
    l = l.push(container(ui::hline(tn.line)).padding(look::pad(z(22.0), 0.0, z(18.0), 0.0)));
    let (ver, ver_k, size) = match (&st, rel.as_ref()) {
        (State::UpToDate(i), _) => (i.version.clone(), "Installed", i.size),
        (State::Ready(i, r), _) => (i.version.clone(), "Installed", r.size),
        (State::Get(r), _) => (r.version.clone(), "Newest", r.size),
        (State::BuiltIn, _) => (if a.hearth.is_empty() { "Hearth OS".into() } else { a.hearth.clone() }, "Comes with Hearth OS", 0),
        (_, Some(r)) => (r.version.clone(), "Newest", r.size),
        _ => ("Not out yet".into(), "Version", 0),
    };
    let test_rel = rel.as_ref().map(|r| r.test).unwrap_or(false);
    let mut stats = Row::new().spacing(z(26.0)).push(stat(&tn, ver, ver_k));
    if size > 0 {
        stats = stats.push(stat(&tn, hearth_store::mb(size, false), "Download"));
    }
    stats = stats.push(stat(&tn, if test_rel { "Test".into() } else { "Stable".into() }, "Channel"));
    l = l.push(stats).push(gap(1.0, z(14.0)));
    match &st {
        State::BuiltIn => {
            l = l.push(note_line(&tn, G::Shield, "Part of Hearth OS, checked with it")).push(gap(1.0, z(6.0))).push(note_line(&tn, G::Sync, "Updates with Hearth OS"));
        }
        _ => {
            l = l.push(note_line(&tn, G::Shield, if e.pin.is_some() { "Checked with the SHA-256 in Hearth OS's list" } else { "Checked with the release's SHA-256 list" }));
            if e.self_updates {
                l = l.push(gap(1.0, z(6.0))).push(note_line(&tn, G::Sync, "Updates itself, from its own release"));
            }
        }
    }
    let left = side_card(a, l);
    // The right: Screenshots, then What's new and Description, and Additional information
    let top = iced::widget::stack![backdrop(e, z(300.0)), column![Space::new().height(Length::Fixed(z(250.0))), ts("Screenshots", f(20.0), tn.text)]];
    let mut r = Column::new().push(top).push(gap(1.0, z(14.0)));
    let shots: Vec<&iced::widget::image::Handle> = e.pictures.iter().filter_map(|u| a.pics.get(u)).take(3).collect();
    let mut sr = Row::new().spacing(z(16.0));
    if shots.is_empty() {
        for c in [look::hex(&e.from, ORANGE), PURPLE, look::OK_DOT] {
            sr = sr.push(container(sketch(&tn, c)).width(Length::FillPortion(1)));
        }
    } else {
        for h in shots {
            let im = iced::widget::image(h.clone()).width(Length::Fill).height(Length::Fill).content_fit(iced::ContentFit::Cover);
            sr = sr.push(container(im).width(Length::FillPortion(1)).height(Length::Fixed(z(236.0))).clip(true).style(look::bx(Color::from_rgb8(0x0f, 0x11, 0x15), 6.0, Some((tn.line, 1.0)))));
        }
    }
    r = r.push(sr).push(gap(1.0, z(26.0)));
    let mut words = Column::new().spacing(z(8.0));
    if let Some(rl) = rel.as_ref().filter(|r| !r.notes.is_empty()) {
        words = words.push(ts(format!("What's new in {}", hearth_store::short_version(&rl.version)), f(18.0), tn.text)).push(bullets(&p, &rl.notes, f(14.0))).push(gap(1.0, z(12.0)));
    }
    words = words.push(ts("Description", f(18.0), tn.text)).push(t(if e.about.is_empty() { e.card.clone() } else { e.about.clone() }, f(14.0), Pal::mix(tn.text, tn.muted, 0.15)));
    let info_kv = |k: &str, v: String| -> E<'a, Msg> { column![t(k.to_string(), f(13.0), tn.muted), t(v, f(14.0), tn.text)].spacing(z(2.0)).width(Length::FillPortion(1)).into() };
    let date = rel.as_ref().map(|r| view::released(&r.date)).unwrap_or_default();
    let needs = format!("{} · no password", if a.hearth.is_empty() { "Hearth OS".to_string() } else { format!("Hearth OS {}", a.hearth) });
    let (installs, updates, source) = match &st {
        State::BuiltIn => ("Part of Hearth OS".to_string(), "With Hearth OS".to_string(), "Hearth OS".to_string()),
        _ => ("AppImage in your home folder".to_string(), if e.self_updates { "By itself".to_string() } else { "From the Store, when you say so".to_string() }, if e.pin.is_some() { "Hearth OS's list".to_string() } else { "GitHub release".to_string() }),
    };
    let info = column![
        ts("Additional information", f(18.0), tn.text),
        row![info_kv("Made by", e.author.clone()), info_kv("Released", if date.is_empty() { "—".into() } else { date })].spacing(z(20.0)),
        row![info_kv("Installs as", installs), info_kv("Needs", needs)].spacing(z(20.0)),
        row![info_kv("Updates", updates), info_kv("Source", source)].spacing(z(20.0)),
    ]
    .spacing(z(16.0));
    let boxed = |c: E<'a, Msg>, w: u16| -> E<'a, Msg> { container(c).padding(z(22.0)).width(Length::FillPortion(w)).style(look::bx(Pal::mix(tn.card, Color::BLACK, 0.04), 8.0, Some((tn.line, 1.0)))).into() };
    r = r.push(row![boxed(words.into(), 58), boxed(info.into(), 42)].spacing(z(20.0)));
    let page = row![container(left).padding(look::pad(z(30.0), 0.0, 0.0, 0.0)), container(r).width(Length::Fill)].spacing(z(28.0));
    with_note(a, page.into())
}

// ------------------------------------------------------------------ 5. A test version's page
pub fn test_page<'a>(a: &'a App, e: &Entry) -> E<'a, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let have = a.parts.get(&e.id);
    let pt = e.part.clone();
    let usual = if e.id.contains("mantel") { "Mantel".to_string() } else { e.name.trim_end_matches(" (test)").to_string() };
    let notice = container(
        row![chip_test(), t(format!("A test version: it may change from week to week. Your usual {usual} stays one click away, and it comes back by itself if this one doesn't start."), f(14.0), tn.text).width(Length::Fill)]
            .spacing(z(14.0))
            .align_y(iced::Center),
    )
    .padding(look::pad(z(12.0), z(18.0), z(12.0), z(18.0)))
    .width(Length::Fixed(z(900.0)))
    .style(look::bx(Pal::a(look::WARN, 0.10), 6.0, Some((Pal::a(look::WARN, 0.55), 1.0))));
    let name = e.short.trim_end_matches(" (test)").to_string();
    let mut l = Column::new().push(app_icon(e, z(112.0))).push(gap(1.0, z(18.0)));
    l = l.push(tb(name, f(28.0), tn.text)).push(t(format!("{} · Hearth part", e.author), f(15.0), look::ORANGE2)).push(gap(1.0, z(18.0)));
    let busy = a.part_busy.contains(&e.id);
    let fits = pt.as_ref().map(|pt| hearth_store::parts::test_fits(pt, &a.hearth)).unwrap_or(false);
    if have.is_some() {
        if a.part_offer(e).is_some() {
            l = l.push(wide(a, if busy { "Updating…" } else { "Update test version" }, B::Primary, if busy || a.offline() { None } else { Some(Msg::PartUpdate(e.id.clone())) })).push(gap(1.0, z(8.0)));
        }
        if have.map(|h| h.gave_up() || h.failed_now).unwrap_or(false) {
            l = l.push(wide(a, "Try it again", B::Normal, if busy { None } else { Some(Msg::PartRetry(e.id.clone())) })).push(gap(1.0, z(8.0)));
        }
        l = l.push(wide(a, if busy { "Removing…" } else { "Remove" }, B::Normal, if busy { None } else { Some(Msg::PartRemove(e.id.clone())) }));
    } else {
        l = l.push(wide(a, if busy { "Getting…" } else { "Get test version" }, B::Primary, if busy || a.offline() || !fits { None } else { Some(Msg::PartGet(e.id.clone())) }));
        if !fits && pt.is_some() {
            l = l.push(container(t(format!("Not made for Hearth OS {} yet", a.hearth), f(13.0), tn.muted)).padding(look::pad(z(8.0), 0.0, 0.0, 0.0)));
        }
    }
    l = l.push(container(ui::hline(tn.line)).padding(look::pad(z(22.0), 0.0, z(18.0), 0.0)));
    if let Some(pt) = pt.as_ref() {
        let mut stats = Row::new().spacing(z(26.0)).push(stat(&tn, have.map(|h| h.version.clone()).unwrap_or_else(|| pt.version.clone()), if have.is_some() { "In use" } else { "Test" }));
        if pt.size > 0 {
            stats = stats.push(stat(&tn, hearth_store::mb(pt.size, false), "Download"));
        }
        l = l.push(stats.push(stat(&tn, pt.os.clone(), "For Hearth OS"))).push(gap(1.0, z(14.0)));
    }
    l = l.push(note_line(&tn, G::Shield, "Signed and checked before it starts"));
    if let Some(h) = have.filter(|h| h.gave_up() || h.failed_now) {
        l = l.push(gap(1.0, z(6.0))).push(note_line(&tn, G::Info, format!("{} didn't start, so the usual one is used", h.version)));
    }
    let left = side_card(a, l);
    let lines = |v: &[String], none: &str| -> Vec<String> { if v.is_empty() { vec![none.to_string()] } else { v.to_vec() } };
    let notes = pt.as_ref().map(|p| p.notes.clone()).unwrap_or_default();
    let missing = pt.as_ref().map(|p| p.missing.clone()).unwrap_or_default();
    let boxed = |title: &str, v: Vec<String>| -> E<'a, Msg> {
        container(column![ts(title.to_string(), f(18.0), tn.text), bullets(&p, &v, f(14.0))].spacing(z(10.0))).padding(z(22.0)).width(Length::FillPortion(1)).style(look::bx(Pal::mix(tn.card, Color::BLACK, 0.04), 8.0, Some((tn.line, 1.0)))).into()
    };
    let others: Vec<&Entry> = a.cat.apps.iter().filter(|x| x.test && x.id != e.id).collect();
    let more_line = if others.is_empty() {
        format!("Only {} for now. Test versions show here while Get test versions is on in Settings.", e.short.trim_end_matches(" (test)"))
    } else {
        format!("Also: {}. Test versions show here while Get test versions is on in Settings.", others.iter().map(|x| x.short.trim_end_matches(" (test)").to_string()).collect::<Vec<_>>().join(", "))
    };
    let more = container(row![t(more_line, f(13.0), tn.muted).width(Length::Fill), btn(&p, "Settings", B::Normal, false, false, Some(Msg::Go(Page::Settings)))].align_y(iced::Center))
        .padding(look::pad(z(12.0), z(18.0), z(12.0), z(18.0)))
        .width(Length::Fill)
        .style(look::bx(Pal::mix(tn.card, Color::BLACK, 0.04), 8.0, Some((tn.line, 1.0))));
    let r = column![
        backdrop(e, z(250.0)),
        gap(1.0, z(16.0)),
        row![boxed("What's in it", lines(&notes, "What this test version brings is on its release page.")), boxed("Not in it yet", lines(&missing, "Nothing left out that we know of."))].spacing(z(20.0)),
        gap(1.0, z(24.0)),
        click(row![ts("More test versions", f(20.0), tn.text), t("  ›", f(20.0), tn.muted)].align_y(iced::Center), Some(Msg::Go(Page::Tests))),
        gap(1.0, z(14.0)),
        more
    ];
    let page = column![notice, gap(1.0, z(26.0)), row![left, container(r).width(Length::Fill)].spacing(z(28.0))];
    with_note(a, page.into())
}

// ------------------------------------------------------------------ 6. Settings
fn srow_w11<'a>(a: &'a App, g: G, title: &str, sub: String, end: E<'a, Msg>, m: Option<Msg>) -> E<'a, Msg> {
    let tn = tone(&a.p);
    let r = row![ui::glyph(g, tn.text, z(18.0), 1.5), column![tb(title.to_string(), f(14.0), tn.text), t(sub, f(13.0), tn.muted)].spacing(z(2.0)).width(Length::Fill), end].spacing(z(20.0)).align_y(iced::Center);
    click(container(r).padding(look::pad(z(14.0), z(20.0), z(14.0), z(20.0))).width(Length::Fill), m)
}

fn group<'a>(a: &'a App, title: &str, rows: Vec<E<'a, Msg>>) -> E<'a, Msg> {
    let tn = tone(&a.p);
    let mut c = Column::new();
    for (k, r) in rows.into_iter().enumerate() {
        if k > 0 {
            c = c.push(ui::hline(tn.line));
        }
        c = c.push(r);
    }
    column![container(tb(title.to_string(), f(14.0), tn.text)).padding(look::pad(z(26.0), 0.0, z(12.0), 0.0)), container(c).width(Length::Fill).style(look::bx(Pal::mix(tn.card, Color::BLACK, 0.04), 6.0, Some((tn.line, 1.0))))].into()
}

fn toggle<'a>(a: &'a App, on: bool, m: Msg) -> E<'a, Msg> {
    let p = a.p;
    let tn = tone(&p);
    row![t(if on { "On" } else { "Off" }, f(13.0), tn.muted), look::switch(&p, on, false, Some(m))].spacing(z(14.0)).align_y(iced::Center).into()
}

pub fn settings(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let n = a.ready().len() + a.part_offers().len();
    let last = if a.seen.checked > 0 { format!("Last checked {}", view::day_time_pub(a.seen.checked)) } else { "Not checked yet".to_string() };
    let ready = if n == 0 { "everything is up to date".to_string() } else { format!("{n} update{} ready", if n == 1 { "" } else { "s" }) };
    let check = btn(&p, if a.checking { "Checking…" } else { "Check now" }, B::Primary, false, false, (!a.checking).then_some(Msg::TryAgain));
    let updates = group(a, "Updates", vec![
        srow_w11(a, G::Sync, "Check for updates", format!("{last} · {ready}"), check, None),
        srow_w11(a, G::Down, "Look for updates every day", "Once a day in the background, and tells you when some are ready".into(), toggle(a, a.prefs.daily, Msg::SetDaily(!a.prefs.daily)), Some(Msg::SetDaily(!a.prefs.daily))),
        srow_w11(a, G::Grid, "Install updates by themselves", "Only Hearth parts and apps that update themselves anyway; never while you play".into(), toggle(a, a.prefs.auto, Msg::SetFlag("auto", !a.prefs.auto)), Some(Msg::SetFlag("auto", !a.prefs.auto))),
    ]);
    let tests = group(a, "Test versions", vec![srow_w11(a, G::Pulse, "Get test versions", "Shows test versions (marked TEST) under Tests, with Get and Remove. Your usual version always stays.".into(), toggle(a, a.prefs.test, Msg::SetFlag("test", !a.prefs.test)), Some(Msg::SetFlag("test", !a.prefs.test)))]);
    let signed = if a.cat.source == "hearthos.fun" {
        format!("Signed by Dragongamer9 · list {} · checked {}", a.cat.serial, view::day_time_pub(a.cat.signed_at).split(" at ").next().unwrap_or("").to_lowercase())
    } else {
        format!("Built into {}", if a.hearth.is_empty() { "Hearth OS".to_string() } else { format!("Hearth OS {}", a.hearth) })
    };
    let trusted: E<Msg> = if a.cat.source == "hearthos.fun" || a.cat.serial > 0 { chip(&p, "Trusted", Chip::Ok) } else { Space::new().into() };
    let more = group(a, "More", vec![
        srow_w11(a, G::Folder, "Where apps go", format!("{} (your home folder: no password, nothing system-wide)", hearth_store::paths::tilde(&hearth_store::paths::apps())), btn(&p, if a.moving { "Moving…" } else { "Change…" }, B::Normal, false, false, (!a.moving).then_some(Msg::AppsDir)), None),
        srow_w11(a, G::Shield, "The app list", signed, trusted, None),
        srow_w11(a, G::Gear, "About Hearth Store", format!("{} · This PC: {}", store_version(a), view::this_pc_pub(a)), Space::new().into(), None),
    ]);
    let mut c = column![ts("Settings", f(30.0), tn.text), updates, tests, more].width(Length::Fixed(z(1010.0)));
    if a.prefs.unreadable {
        c = c.push(container(t("The Store's settings file couldn't be read, so nothing is written over it (it's left for you to look at).", f(14.0), look::BAD)).padding(look::pad(z(14.0), 0.0, 0.0, 0.0)));
    }
    if !a.note.is_empty() {
        c = c.push(container(t(a.note.clone(), f(14.0), if a.note_bad { look::BAD } else { look::ORANGE2 })).padding(look::pad(z(14.0), 0.0, 0.0, 0.0)));
    }
    c.into()
}

/// The Store's own version: its quick update when that's what runs, else the Hearth OS it came with.
fn store_version(a: &App) -> String {
    let have = a.cat.apps.iter().find(|e| e.part.as_ref().map(|p| p.id == "hearth-store-app").unwrap_or(false)).and_then(|e| a.parts.get(&e.id));
    let v = hearth_store::parts::running(have, &a.hearth);
    if v.is_empty() { "Built into Hearth OS".into() } else { v }
}

/// What the Store just said ("Updating Mantel to 1.17.4…", what went wrong), over a page that has no place of its own.
fn with_note<'a>(a: &'a App, page: E<'a, Msg>) -> E<'a, Msg> {
    if a.note.is_empty() {
        return page;
    }
    column![container(t(a.note.clone(), f(14.0), if a.note_bad { look::BAD } else { look::ORANGE2 })).padding(look::pad(0.0, 0.0, z(12.0), 0.0)), page].into()
}

// ------------------------------------------------------------------ Hearth parts (store-everything/store-1)
/// A tile with letters or a mark in two colours (the system card's H, the Fedora / Debian card's letter).
fn tile_mark<'a>(inner: E<'a, Msg>, from: Color, to: Color, size: f32) -> E<'a, Msg> {
    container(inner).center_x(Length::Fixed(size)).center_y(Length::Fixed(size)).style(look::grad(135.0, from, to, (size * 0.22).round(), None)).into()
}

fn letters<'a>(s: &str, size: f32) -> E<'a, Msg> {
    tile_mark(tb(s.to_string(), f(size * 0.42), Color::WHITE).into(), look::ORANGE2, Color::from_rgb8(0xd9, 0x53, 0x1e), size)
}

/// The parts page's groups: (id, pill, section title)
const GROUPS: [(&str, &str, &str); 5] = [("desktop", "Desktop", "DESKTOP"), ("apps", "Apps", "APPS"), ("gamemode", "Game Mode", "GAME MODE"), ("services", "Services", "SERVICES · run in the background"), ("looks", "Looks and sounds", "LOOKS AND SOUNDS")];

fn in_group(e: &Entry, g: &str) -> bool {
    let mine = e.group_of();
    // Looks and sounds sit with the desktop in the drawn sections, and have their own pill.
    mine == g || (g == "desktop" && mine == "looks")
}

/// One part's row: its letter icon, name and short line, the version in use, then Up to date, or the new version with
/// Update, or what it comes with ("with system updates").
fn part_row<'a>(a: &'a App, e: &Entry) -> E<'a, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let line = if e.card.is_empty() { e.tagline.clone() } else { e.card.clone() };
    let ver = a.part_version(e);
    let busy = a.part_busy.contains(&e.id) || (!e.decky.is_empty() && a.decky_busy);
    let gave_up = e.part.as_ref().map(|pt| if pt.system { a.sys_parts.get(&pt.id).map(|h| h.gave_up()).unwrap_or(false) } else { a.parts.get(&e.id).map(|h| h.gave_up() || h.failed_now).unwrap_or(false) }).unwrap_or(false);
    let mut end = Row::new().spacing(z(14.0)).align_y(iced::Center);
    let mut fix = String::new();
    if let Some(o) = a.part_offer(e) {
        fix = o.notes.first().cloned().unwrap_or_default();
        end = end.push(chip(&p, o.version.clone(), Chip::Up)).push(btn(&p, if busy { "Updating…" } else { "Update" }, B::Normal, false, false, (!busy && !a.offline()).then(|| Msg::PartUpdate(e.id.clone()))));
    } else if let Some(v) = (!e.decky.is_empty()).then(|| a.decky_offer(e)).flatten() {
        end = end.push(chip(&p, v, Chip::Up)).push(btn(&p, if busy { "Updating…" } else { "Update" }, B::Normal, false, false, (!busy && !a.offline()).then_some(Msg::DeckyUpdate)));
    } else if e.with == "system" {
        end = end.push(chip(&p, "with system updates", Chip::Plain));
    } else if !e.with.is_empty() {
        let carrier = a.cat.apps.iter().find(|x| x.id == e.with || x.part.as_ref().map(|pt| pt.id == e.with).unwrap_or(false)).map(|x| x.short.clone()).unwrap_or_else(|| e.with.clone());
        end = end.push(chip(&p, format!("with {carrier}"), Chip::Plain));
    } else if gave_up {
        end = end.push(chip(&p, "back to the one before", Chip::Warn));
    } else {
        end = end.push(chip(&p, "Up to date", Chip::Ok));
    }
    let mut r = row![
        app_icon(e, z(44.0)),
        column![tb(e.short.clone(), f(15.0), tn.text), t(line, f(13.0), tn.muted)].spacing(z(3.0)).width(Length::Fill),
        container(t(ver, f(13.0), tn.muted)).width(Length::Fixed(z(70.0))),
        end
    ]
    .spacing(z(16.0))
    .align_y(iced::Center);
    if !fix.is_empty() {
        r = r.push(container(t(fix, f(13.0), tn.muted)).max_width(z(220.0)));
    }
    click(container(r).padding(look::pad(z(14.0), z(18.0), z(14.0), z(18.0))).width(Length::Fill), Some(Msg::OpenApp(e.id.clone())))
}

fn section<'a>(a: &'a App, title: &str, list: Vec<&Entry>) -> Option<E<'a, Msg>> {
    if list.is_empty() {
        return None;
    }
    let tn = tone(&a.p);
    let mut c = Column::new();
    for (k, e) in list.iter().enumerate() {
        if k > 0 {
            c = c.push(ui::hline(tn.line));
        }
        c = c.push(part_row(a, e));
    }
    let (head, rest) = title.split_once(" · ").map(|(h, r)| (h.to_string(), format!(" · {r}"))).unwrap_or((title.to_string(), String::new()));
    Some(
        column![
            container(row![t(head, f(13.0), tn.muted), t(rest, f(13.0), tn.muted)]).padding(look::pad(z(18.0), 0.0, z(8.0), z(4.0))),
            container(c).width(Length::Fill).style(look::bx(Pal::mix(tn.card, Color::BLACK, 0.04), 8.0, Some((tn.line, 1.0))))
        ]
        .into(),
    )
}

pub fn parts(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let all: Vec<&Entry> = a.cat.apps.iter().filter(|e| e.is_hearth_part()).collect();
    let n = a.parts_ready();
    // Title, when it last looked, Check for updates and Update all
    let checked = if a.checking { "Checking…".to_string() } else { format!("Checked {}{}", view::ago_pub(a, a.seen.checked), if n > 0 { format!(" · {n} update{} · none needs a restart", if n == 1 { "" } else { "s" }) } else { " · everything is up to date".to_string() }) };
    let head = row![
        ts("Hearth parts", f(30.0), tn.text),
        Space::new().width(Length::Fill),
        t(checked, f(13.0), tn.muted),
        gap(z(14.0), 1.0),
        btn(&p, "⟳  Check for updates", B::Normal, false, false, (!a.checking).then_some(Msg::TryAgain)),
        gap(z(14.0), 1.0),
        btn(&p, "Update all", B::Primary, false, false, (n > 0 && !a.offline()).then_some(Msg::UpdateAll))
    ]
    .align_y(iced::Center);
    let mut c = Column::new().push(head).push(container(t("Everything Hearth makes, each with its own updates: fixes reach you the day they're made, no system update and no restart. A part that doesn't start goes back to the one before by itself.", f(14.0), Pal::mix(tn.text, tn.muted, 0.3))).padding(look::pad(z(4.0), 0.0, z(16.0), 0.0)));
    // The filter pills with their counts
    let f_on = a.parts_filter.clone();
    let mut pills = Row::new().spacing(z(10.0)).align_y(iced::Center);
    let count = |g: &str| all.iter().filter(|e| if g == "desktop" { e.group_of() == "desktop" } else { e.group_of() == g }).count();
    let mut items: Vec<(String, String)> = vec![("all".into(), format!("All {}", all.len()))];
    for (id, pill, _) in GROUPS {
        items.push((id.to_string(), format!("{pill} {}", count(id))));
    }
    for (id, label) in items {
        let on = f_on == id || (f_on.is_empty() && id == "all");
        let pill = container(if on { tb(label, f(14.0), look::INK) } else { t(label, f(14.0), tn.text) })
            .padding(look::pad(z(6.0), z(16.0), z(6.0), z(16.0)))
            .style(look::bx(if on { ORANGE } else { Pal::a(Color::WHITE, 0.03) }, 16.0, Some((if on { Color::TRANSPARENT } else { Pal::mix(tn.line, tn.text, 0.1) }, 1.0))));
        pills = pills.push(click(pill, Some(Msg::PartsFilter(id))));
    }
    c = c.push(pills);
    let pick = |g: &str| -> Vec<&Entry> {
        all.iter().copied().filter(|e| if f_on.is_empty() || f_on == "all" { in_group(e, g) } else if f_on == "desktop" { g == "desktop" && in_group(e, g) } else { g == f_on && e.group_of() == g }).collect()
    };
    let note = container(
        row![ui::glyph(G::Shield, tn.muted, z(18.0), 1.5), t("Every part is checked with SHA-256 against the signed list. Anything that touches starting the PC (boot menu, Recovery) only comes with a system update, so the one before always stays startable.", f(13.0), Pal::mix(tn.text, tn.muted, 0.2)).width(Length::Fill)]
            .spacing(z(14.0))
            .align_y(iced::Center),
    )
    .padding(look::pad(z(16.0), z(20.0), z(16.0), z(20.0)))
    .width(Length::Fill)
    .style(look::bx(Pal::mix(tn.card, Color::BLACK, 0.04), 8.0, Some((tn.line, 1.0))));
    let mut left = Column::new().width(Length::FillPortion(1));
    let mut right = Column::new().width(Length::FillPortion(1));
    for (id, _, title) in [GROUPS[0], GROUPS[2], GROUPS[1]] {
        if let Some(s) = section(a, title, pick(id)) {
            left = left.push(s);
        }
    }
    if f_on == "looks" {
        if let Some(s) = section(a, GROUPS[4].2, pick("looks")) {
            left = left.push(s);
        }
    }
    if let Some(s) = section(a, GROUPS[3].2, pick("services")) {
        right = right.push(s);
    }
    right = right.push(gap(1.0, z(18.0))).push(note);
    c = c.push(row![left, right].spacing(z(18.0)));
    with_note(a, c.into())
}

// ------------------------------------------------------------------ Library › Hearth OS (store-everything/store-2)
fn sys_ver_words(v: &hearth_store::system::Ver) -> String {
    let d = v.day();
    format!("Hearth OS {}{}", hearth_store::short_version(&v.version), if d.is_empty() { String::new() } else { format!(" · {d}") })
}

pub fn system<'a>(a: &'a App) -> E<'a, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let word = hearth_store::system::base_word(&a.sys_base);
    let info = a.cat.system.clone().unwrap_or_default();
    let st = a.sys.clone().unwrap_or_default();
    let avail = st.available.clone();
    let staged = st.staged.clone();
    let day = avail.as_ref().or(staged.as_ref()).map(|v| v.day()).unwrap_or_default();
    let info = if info.date.is_empty() || hearth_store::system::day_words(&info.date) == day { info } else { hearth_store::system::SysInfo { next: info.next.clone(), next_out: info.next_out, ..Default::default() } };
    let prog = a.sys_prog();
    let running = prog.as_ref().map(|g| g.end.is_none()).unwrap_or(false);
    // Title
    let head = row![
        click(ts("Library", f(30.0), tn.text), Some(Msg::Go(Page::Updates))),
        t("  ›  ", f(26.0), tn.muted),
        ts("Hearth OS", f(30.0), tn.text),
        Space::new().width(Length::Fill),
        btn(&p, if a.sys_looking { "Checking…" } else { "⟳  Check for updates" }, B::Normal, false, false, (!a.sys_looking && !a.checking).then_some(Msg::TryAgain))
    ]
    .align_y(iced::Center);
    // The system card
    let kicker = format!("THE SYSTEM · {} · HEARTH OS {}", a.sys_base.to_uppercase(), if a.hearth.is_empty() { "".to_string() } else { a.hearth.clone() });
    let title = if staged.is_some() && avail.is_none() {
        format!("{word} updates{} · ready", if day.is_empty() { String::new() } else { format!(" · {day}") })
    } else if avail.is_some() {
        format!("{word} updates{}", if day.is_empty() { String::new() } else { format!(" · {day}") })
    } else if a.sys.is_none() {
        format!("{word} updates")
    } else {
        format!("{word} is up to date")
    };
    let mut bits: Vec<String> = Vec::new();
    if info.packages > 0 {
        bits.push(format!("{} packages", info.packages));
    }
    if !info.fixes.is_empty() {
        bits.push(info.fixes.clone());
    }
    if info.size > 0 {
        bits.push(format!("{} (only what changed)", hearth_store::mb(info.size, false)));
    }
    let line = if bits.is_empty() {
        if avail.is_some() {
            format!("{word}'s newest fixes, built into Hearth OS {} and tested by Hearth · only what changed is downloaded", a.hearth)
        } else if a.sys.is_none() {
            "Check for updates looks for a newer one (it asks bootc through Hearth's admin helper)".to_string()
        } else {
            format!("Hearth OS {} on {} has every fix that's out", a.hearth, a.sys_base)
        }
    } else {
        bits.join(" · ")
    };
    let mins = if info.size > 0 { format!("About {} min to download · ", (info.size / 60_000_000).max(1)) } else { String::new() };
    let right = column![
        btn(&p, if running { "Downloading…" } else { "Get it now" }, B::Primary, true, false, (avail.is_some() && !running && !a.offline()).then_some(Msg::SysGet)),
        t(format!("{mins}applies on your next restart"), f(12.0), tn.muted)
    ]
    .spacing(z(10.0))
    .align_x(iced::Right);
    let card = container(
        row![
            letters("H", z(88.0)),
            column![ts(kicker, f(13.0), ORANGE), tb(title, f(32.0), tn.text), t(line, f(16.0), Pal::mix(tn.text, tn.muted, 0.25))].spacing(z(4.0)).width(Length::Fill),
            right
        ]
        .spacing(z(24.0))
        .align_y(iced::Center),
    )
    .padding(look::pad(z(24.0), z(28.0), z(24.0), z(28.0)))
    .width(Length::Fill)
    .style(look::grad(100.0, Pal::mix(tn.card, ORANGE, 0.10), Pal::mix(tn.card, Color::BLACK, 0.06), 8.0, Some((tn.line, 1.0))));
    // Three kinds of update
    let kind_row = |mark: E<'a, Msg>, name: String, tag: &str, tag_ok: bool, sub: String, end: E<'a, Msg>, m: Option<Msg>| -> E<'a, Msg> {
        let tagc = container(t(tag.to_string(), f(13.0), if tag_ok { look::OK_TX } else { Pal::mix(tn.text, tn.muted, 0.3) })).padding(look::pad(z(2.0), z(9.0), z(2.0), z(9.0))).style(look::bx(Color::TRANSPARENT, 10.0, Some((if tag_ok { look::OK_LINE } else { tn.line }, 1.0))));
        let r = row![mark, column![row![tb(name, f(15.0), tn.text), tagc].spacing(z(8.0)).align_y(iced::Center), t(sub, f(13.0), tn.muted)].spacing(z(4.0)).width(Length::Fill), end].spacing(z(16.0)).align_y(iced::Center);
        click(container(r).padding(look::pad(z(14.0), z(18.0), z(14.0), z(18.0))).width(Length::Fill), m)
    };
    let bolt = tile_mark(ui::glyph(G::Update, Color::WHITE, z(20.0), 2.0), look::ORANGE2, Color::from_rgb8(0xd9, 0x53, 0x1e), z(44.0));
    let base_mark = tile_mark(tb(word.chars().next().unwrap_or('S').to_string(), f(20.0), Color::WHITE).into(), Color::from_rgb8(0x4f, 0x8c, 0xff), Color::from_rgb8(0x25, 0x5f, 0xe0), z(44.0));
    let h_mark = tile_mark(tb("H", f(20.0), Color::WHITE).into(), Color::from_rgb8(0x3a, 0x2a, 0x22), Color::from_rgb8(0x1c, 0x14, 0x10), z(44.0));
    let sys_state: E<Msg> = if staged.is_some() { chip(&p, "ready", Chip::Up) } else if running { chip(&p, "downloading", Chip::Plain) } else if avail.is_some() { chip(&p, "out", Chip::Up) } else { chip(&p, "up to date", Chip::Ok) };
    let next_name = if info.next.is_empty() { "The next Hearth OS".to_string() } else { format!("Hearth OS {}", info.next) };
    let next_state: E<Msg> = if info.next_out { chip(&p, "out", Chip::Up) } else { chip(&p, "not yet", Chip::Plain) };
    let n_parts = a.parts_ready();
    let parts_end: E<Msg> = if n_parts > 0 { chip(&p, format!("{n_parts} ready"), Chip::Up) } else { chip(&p, "up to date", Chip::Ok) };
    let kinds = column![
        container(t("THREE KINDS OF UPDATE", f(13.0), tn.muted)).padding(look::pad(z(16.0), z(18.0), z(12.0), z(18.0))),
        ui::hline(tn.line),
        kind_row(bolt, "Hearth parts".into(), "seconds · no restart", true, "Mantel, the apps, the services: on the Hearth parts page, each on its own".into(), parts_end, Some(Msg::Go(Page::BuiltIn))),
        ui::hline(tn.line),
        kind_row(base_mark, format!("{word} updates"), "minutes · next restart", false, format!("Security and driver fixes, built and tested by Hearth the day {word} ships them, named by date{}, downloaded when you say", if day.is_empty() { String::new() } else { format!(" ({day})") }), sys_state, None),
        ui::hline(tn.line),
        kind_row(h_mark, next_name, "bigger · next restart", false, "A new Hearth OS: comes when Dragongamer9 releases it".into(), next_state, None),
    ];
    let boxed = |c: E<'a, Msg>, w: u16| -> E<'a, Msg> { container(c).width(Length::FillPortion(w)).style(look::bx(Pal::mix(tn.card, Color::BLACK, 0.04), 8.0, Some((tn.line, 1.0)))).into() };
    // Safe every time
    let before = st.rollback.as_ref().filter(|_| staged.is_none()).or(st.booted.as_ref()).map(sys_ver_words).unwrap_or_else(|| format!("Hearth OS {}", a.hearth));
    let safe_row = |g: G, title: &str, sub: String| -> E<'a, Msg> { container(row![ui::glyph(g, tn.text, z(20.0), 1.6), column![tb(title.to_string(), f(15.0), tn.text), t(sub, f(13.0), tn.muted)].spacing(z(3.0)).width(Length::Fill)].spacing(z(18.0)).align_y(iced::Center)).padding(look::pad(z(14.0), z(18.0), z(14.0), z(18.0))).width(Length::Fill).into() };
    let safe = column![
        container(t("SAFE EVERY TIME", f(13.0), tn.muted)).padding(look::pad(z(16.0), z(18.0), z(12.0), z(18.0))),
        ui::hline(tn.line),
        safe_row(G::Shield, "The one before stays startable", format!("{before} stays in the boot menu")),
        ui::hline(tn.line),
        safe_row(G::Chevron, "Back by itself", "If the new one doesn't reach Game Mode twice, the one before starts and Hearth tells you why".into()),
        ui::hline(tn.line),
        safe_row(G::List, "Your settings, apps and parts stay", "Only the system underneath changes".into()),
    ];
    let two = row![boxed(kinds.into(), 55), boxed(safe.into(), 45)].spacing(z(18.0));
    // What's in this one, and the switch
    let all_shown = a.shown_new.contains("system-changes");
    let shown: Vec<String> = if all_shown { info.changes.clone() } else { info.changes.iter().take(4).cloned().collect() };
    let more = info.changes.len().saturating_sub(shown.len());
    let total = (info.packages as usize).max(info.changes.len());
    let changes_line = if shown.is_empty() {
        if avail.is_some() { "The list of what changed comes with the update's notes on its release".to_string() } else { "Nothing new to download".to_string() }
    } else {
        format!("{}{}", shown.join(" · "), if more > 0 || total > shown.len() { format!(" · and {} more", total.saturating_sub(shown.len())) } else { String::new() })
    };
    let see_all: E<Msg> = if info.changes.len() > 4 || total > shown.len() { btn(&p, if all_shown { "Show fewer".to_string() } else { format!("See all {total}") }, B::Normal, false, false, Some(Msg::ToggleNew("system-changes".into()))) } else { Space::new().into() };
    let whats = container(row![column![tb("What's in this one", f(15.0), tn.text), t(changes_line, f(13.0), tn.muted)].spacing(z(4.0)).width(Length::Fill), see_all].align_y(iced::Center)).padding(look::pad(z(14.0), z(18.0), z(14.0), z(18.0))).width(Length::Fill);
    let auto = container(
        row![
            column![tb(format!("{word} updates by themselves"), f(15.0), tn.text), t("Off: nothing downloads until you press Get it now. On: downloads in the background, applies when you restart. Never restarts on its own.", f(13.0), tn.muted)].spacing(z(4.0)).width(Length::Fill),
            look::switch(&p, a.sys_auto, false, Some(Msg::SysAuto(!a.sys_auto)))
        ]
        .align_y(iced::Center),
    )
    .padding(look::pad(z(14.0), z(18.0), z(14.0), z(18.0)))
    .width(Length::Fill);
    let bottom = container(column![whats, ui::hline(tn.line), auto]).width(Length::Fill).style(look::bx(Pal::mix(tn.card, Color::BLACK, 0.04), 8.0, Some((tn.line, 1.0))));
    let c = column![head, gap(1.0, z(18.0)), card, gap(1.0, z(18.0)), two, gap(1.0, z(18.0)), bottom];
    with_note(a, c.into())
}

// ------------------------------------------------------------------ Library, updating (store-everything/store-3)
/// Library while something updates or just did: the system download (Pause), the parts done, ready for the next
/// restart (Later, Restart now), and Done before.
pub fn updating(a: &App) -> Option<E<'_, Msg>> {
    let prog = a.sys_prog();
    let staged = a.sys.as_ref().and_then(|s| s.staged.clone());
    let show_ready = staged.is_some() && !a.sys_later;
    if prog.is_none() && a.done_now.is_empty() && !show_ready {
        return None;
    }
    let p = a.p;
    let tn = tone(&p);
    let word = hearth_store::system::base_word(&a.sys_base);
    let day = a.sys.as_ref().and_then(|s| s.available.as_ref().or(s.staged.as_ref())).map(|v| v.day()).unwrap_or_default();
    let mut c = Column::new().push(head(a, "Library", true));
    let mut box_ = Column::new();
    let base_mark = |size: f32| tile_mark(tb(word.chars().next().unwrap_or('S').to_string(), f(size * 0.42), Color::WHITE).into(), Color::from_rgb8(0x4f, 0x8c, 0xff), Color::from_rgb8(0x25, 0x5f, 0xe0), size);
    let mut rows = 0;
    if let Some(g) = prog.as_ref().filter(|g| g.end.is_none()) {
        let amount = if g.total > 0 { format!("{} of {}", hearth_store::mb(g.done, false).trim_end_matches(" MB"), hearth_store::mb(g.total, false)) } else { format!("{:.0} %", g.pct) };
        let what = if g.desc.is_empty() { "downloading what changed".to_string() } else { g.desc.to_lowercase() };
        let frac = if g.total > 0 { g.done as f32 / g.total as f32 } else { g.pct / 100.0 };
        let r = row![
            base_mark(z(50.0)),
            column![
                tb(format!("{word} updates{}", if day.is_empty() { String::new() } else { format!(" · {day}") }), f(15.0), tn.text),
                t(format!("You pressed Get it now · {what} · {amount} · you can keep playing"), f(13.0), tn.muted),
                container(look::progress(&p, frac.clamp(0.0, 1.0), z(4.0), Length::Fixed(z(440.0)))).padding(look::pad(z(6.0), 0.0, 0.0, 0.0))
            ]
            .spacing(z(3.0))
            .width(Length::Fill),
            btn(&p, if g.paused { "Pausing…" } else { "Pause" }, B::Normal, false, false, (!g.paused).then_some(Msg::SysPause))
        ]
        .spacing(z(16.0))
        .align_y(iced::Center);
        box_ = box_.push(container(r).padding(look::pad(z(14.0), z(18.0), z(14.0), z(18.0))).width(Length::Fill));
        rows += 1;
    }
    for d in &a.done_now {
        let Some(e) = a.entry(&d.id) else { continue };
        if rows > 0 {
            box_ = box_.push(ui::hline(tn.line));
        }
        let r = row![app_icon(e, z(50.0)), column![tb(d.title.clone(), f(15.0), tn.text), t(d.line.clone(), f(13.0), tn.muted)].spacing(z(3.0)).width(Length::Fill), chip(&p, "Done", Chip::Ok)].spacing(z(16.0)).align_y(iced::Center);
        box_ = box_.push(container(r).padding(look::pad(z(14.0), z(18.0), z(14.0), z(18.0))).width(Length::Fill));
        rows += 1;
    }
    if rows > 0 {
        c = c.push(container(box_).width(Length::Fill).style(look::bx(Pal::mix(tn.card, Color::BLACK, 0.04), 8.0, Some((tn.line, 1.0)))));
    }
    if show_ready {
        let booted = a.sys.as_ref().and_then(|s| s.booted.clone());
        let before = booted.map(|v| { let d = v.day(); format!("Hearth OS {}{}", hearth_store::short_version(&v.version), if d.is_empty() { String::new() } else { format!(" ({d})") }) }).unwrap_or_else(|| format!("Hearth OS {}", a.hearth));
        let banner = container(
            row![
                ui::glyph(G::Shield, tn.text, z(30.0), 1.6),
                column![tb(format!("{word} updates are ready for your next restart"), f(22.0), tn.text), t(format!("Nothing restarts by itself. {before} stays in the boot menu, and comes back by itself if the new one doesn't start."), f(15.0), Pal::mix(tn.text, tn.muted, 0.25))].spacing(z(4.0)).width(Length::Fill),
                btn(&p, "Later", B::Normal, false, false, Some(Msg::SysLater)),
                btn(&p, "Restart now", B::Primary, true, false, Some(Msg::SysRestart))
            ]
            .spacing(z(20.0))
            .align_y(iced::Center),
        )
        .padding(look::pad(z(22.0), z(28.0), z(22.0), z(28.0)))
        .width(Length::Fill)
        .style(look::bx(Pal::mix(tn.card, ORANGE, 0.05), 8.0, Some((Pal::a(ORANGE, 0.6), 1.0))));
        c = c.push(gap(1.0, z(20.0))).push(banner);
    }
    let hist = hearth_store::system::history();
    let all = a.shown_new.contains("history");
    let words: Vec<String> = hist.iter().take(if all { 40 } else { 3 }).map(|d| { let day = view::day_time_pub(d.at); format!("{} · {}", d.what, day.split(" at ").next().unwrap_or("")) }).collect();
    let line = if words.is_empty() { "Nothing yet".to_string() } else { words.join(" · ") };
    let done_before = container(row![column![tb("Done before", f(15.0), tn.text), t(line, f(13.0), tn.muted)].spacing(z(4.0)).width(Length::Fill), btn(&p, if all { "Fewer" } else { "History" }, B::Normal, false, false, Some(Msg::ToggleNew("history".into())))].align_y(iced::Center))
        .padding(look::pad(z(14.0), z(18.0), z(14.0), z(18.0)))
        .width(Length::Fill)
        .style(look::bx(Pal::mix(tn.card, Color::BLACK, 0.04), 8.0, Some((tn.line, 1.0))));
    c = c.push(gap(1.0, z(20.0))).push(done_before);
    Some(c.into())
}
