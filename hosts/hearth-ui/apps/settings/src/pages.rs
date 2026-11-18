//! What Settings draws: the frame (the left list, the page's title, the button hints) and every page, as the approved
//! mockups draw them (settings-A1-home-mouse.png, settings-A2-display-mouse.png).

use crate::data::{self, Drive};
use crate::{page, App, Msg, CATS, PAGES};
use hearth_ui::ui::{self, btn, gap, glyph, hline, t, t1, tb, tm, ts, Kind, TitleMsgs, G};
use hearth_ui::Pal;
use iced::widget::{column, container, mouse_area, row, scrollable, slider, text_input, Column, Row, Space};
use iced::{mouse, Color, Element, Length, Padding};

type E<'a> = Element<'a, Msg>;

fn cat_glyph(id: &str) -> G {
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

fn page_glyph(id: &str) -> G {
    match id {
        "display" => G::Display,
        "sound" => G::Sound,
        "power" => G::Power,
        "storage" | "memory" => G::Drive,
        "your-desktop" => G::Grid,
        "startup" => G::Power,
        "datetime" => G::Clock,
        "about" => G::Info,
        "wifi" | "hotspot" | "proxy" | "vpn" | "hs-tailscale" => G::Wifi,
        "bluetooth" | "phone" => G::Bt,
        "keyboard" | "mouse" => G::Pad,
        "printers" => G::List,
        "notifications" => G::Moon,
        "clipboard" => G::List,
        "region" => G::Search,
        _ => G::Gear,
    }
}

// ------------------------------------------------------------------ the pieces
/// A switch: the accent track with the knob on the right when on.
fn switch<'a>(p: &Pal, on: bool, msg: Option<Msg>) -> E<'a> {
    let p = *p;
    let knob = container(Space::new()).width(Length::Fixed(16.0)).height(Length::Fixed(16.0)).style(ui::boxed(if on { p.ink } else { p.text }, 8.0, None));
    let inner: E = if on { row![Space::new().width(Length::Fill), knob].into() } else { row![knob, Space::new().width(Length::Fill)].into() };
    let track = container(inner)
        .padding(Padding { top: 3.0, right: 4.0, bottom: 3.0, left: 4.0 })
        .width(Length::Fixed(46.0))
        .height(Length::Fixed(24.0))
        .style(ui::boxed(if on { p.accent } else { Pal::mix(p.panel2, p.text, 0.12) }, 12.0, None));
    let words = t(if on { "On" } else { "Off" }, 13.0, p.muted);
    let r = row![words, track].spacing(12.0).align_y(iced::Center);
    match msg {
        Some(m) => mouse_area(r).on_press(m).interaction(mouse::Interaction::Pointer).into(),
        None => r.into(),
    }
}

/// Choices side by side, the picked one in the accent (Scale 100% 125% 150%, Start in Game Mode / Desktop Mode).
fn seg<'a>(p: &Pal, items: Vec<(String, bool, Option<Msg>)>) -> E<'a> {
    let p = *p;
    let mut r = Row::new().spacing(2.0);
    for (label, on, msg) in items {
        let fg = if on { p.ink } else if msg.is_some() { p.text } else { p.muted };
        let cell = container(t1(label, 13.0, fg).font(if on { ui::weight(iced::font::Weight::Semibold) } else { hearth_ui::app::INTER }))
            .padding(Padding { top: 6.0, right: 14.0, bottom: 6.0, left: 14.0 })
            .style(ui::boxed(if on { p.accent } else { Color::TRANSPARENT }, p.r(6.0), None));
        r = r.push(match msg {
            Some(m) if !on => E::from(mouse_area(cell).on_press(m).interaction(mouse::Interaction::Pointer)),
            _ => cell.into(),
        });
    }
    container(r).padding(3.0).style(ui::boxed(Pal::mix(p.panel, p.bg, 0.4), p.r(8.0), None)).into()
}

fn icon_box<'a>(p: &Pal, g: G) -> E<'a> {
    container(glyph(g, p.text, 18.0, 1.8)).center_x(Length::Fixed(40.0)).center_y(Length::Fixed(40.0)).style(ui::boxed(Pal::mix(p.panel, p.text, 0.06), p.r(8.0), None)).into()
}

/// A setting's row: its icon, name, the line under it, and its control on the right.
fn srow<'a>(p: &Pal, g: G, title: impl Into<String>, sub: impl Into<String>, control: E<'a>) -> E<'a> {
    let sub: String = sub.into();
    let mut words = Column::new().spacing(3.0).push(tm(title.into(), 15.0, p.text));
    if !sub.is_empty() {
        words = words.push(t(sub, 13.0, p.muted));
    }
    container(row![icon_box(p, g), words.width(Length::Fill), control].spacing(16.0).align_y(iced::Center))
        .padding(Padding { top: 14.0, right: 20.0, bottom: 14.0, left: 20.0 })
        .width(Length::Fill)
        .style(ui::boxed(p.panel, p.r(10.0), None))
        .into()
}

/// A row that opens another page (›).
fn link_row<'a>(p: &Pal, g: G, title: impl Into<String>, sub: impl Into<String>, msg: Msg) -> E<'a> {
    let r = srow(p, g, title, sub, glyph(G::Chevron, p.muted, 14.0, 2.0));
    mouse_area(r).on_press(msg).interaction(mouse::Interaction::Pointer).into()
}

fn head<'a>(p: &Pal, s: &'a str) -> E<'a> {
    container(ts(s, 14.0, p.muted)).padding(Padding { top: 14.0, right: 0.0, bottom: 2.0, left: 2.0 }).into()
}

fn card<'a>(p: &Pal, inner: impl Into<E<'a>>) -> E<'a> {
    container(inner).padding(22.0).width(Length::Fill).style(ui::boxed(p.panel, p.r(12.0), None)).into()
}

fn pill<'a>(p: &Pal, label: &'a str, accent: bool, msg: Option<Msg>) -> E<'a> {
    let c = container(t1(label, 13.0, if accent { p.accent } else { p.muted }))
        .padding(Padding { top: 4.0, right: 12.0, bottom: 4.0, left: 12.0 })
        .style(ui::boxed(Color::TRANSPARENT, 14.0, Some((if accent { p.accent } else { p.line }, 1.0))));
    match msg {
        Some(m) => mouse_area(c).on_press(m).interaction(mouse::Interaction::Pointer).into(),
        None => c.into(),
    }
}

fn bar_of<'a>(p: &Pal, used: f64, size: f64) -> E<'a> {
    let f = if size > 0.0 { (used / size).clamp(0.0, 1.0) as f32 } else { 0.0 };
    ui::bar(vec![(f, p.accent)], Pal::mix(p.panel2, p.text, 0.08), 8.0, 4.0)
}

fn drive_rows<'a>(p: &Pal, drives: &'a [Drive]) -> Column<'a, Msg> {
    let mut c = Column::new().spacing(14.0);
    for d in drives {
        c = c.push(column![
            row![t(d.name.clone(), 14.0, p.text), Space::new().width(Length::Fill), t(format!("{} of {}", ui::bytes(d.used as f64), ui::bytes(d.size as f64)), 13.0, p.muted)],
            bar_of(p, d.used as f64, d.size as f64)
        ].spacing(8.0));
    }
    c
}

fn input_style(p: Pal) -> impl Fn(&iced::Theme, text_input::Status) -> text_input::Style {
    move |_t, _s| text_input::Style {
        background: iced::Background::Color(Pal::mix(p.panel, p.bg, 0.5)),
        border: iced::Border { color: p.line, width: 1.0, radius: p.r(8.0).into() },
        icon: p.muted,
        placeholder: p.muted,
        value: p.text,
        selection: Pal::a(p.accent, 0.35),
    }
}

/// A plain note in the page (errors in red).
fn note<'a>(p: &Pal, s: String, bad: bool) -> E<'a> {
    container(t(s, 13.0, if bad { p.danger } else { p.muted })).padding(Padding { top: 4.0, right: 0.0, bottom: 4.0, left: 2.0 }).into()
}

// ------------------------------------------------------------------ the frame
pub fn frame(a: &App) -> E<'_> {
    let p = a.p;
    let rail_bg = Pal::mix(p.bg, p.panel, 0.55);
    let initial = a.info.real.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_else(|| "·".into());
    let avatar = container(tb(initial, 22.0, p.ink)).center_x(Length::Fixed(56.0)).center_y(Length::Fixed(56.0)).style(ui::boxed(p.accent, 28.0, None));
    let who = column![
        tb(if a.info.real.is_empty() { "You".to_string() } else { a.info.real.clone() }, 16.0, p.text),
        t(format!("{} · {}", if a.info.admin { "Administrator" } else { "Standard account" }, a.info.host), 13.0, p.muted),
    ]
    .spacing(2.0);
    let search = text_input("Search settings", &a.query).on_input(Msg::Query).size(14.0).padding(10.0).style(input_style(p));
    let mut cats = Column::new().spacing(4.0);
    for (i, (id, label, _)) in CATS.iter().enumerate() {
        let on = i == a.cat && a.query.is_empty();
        let mark = container(Space::new()).width(Length::Fixed(3.0)).height(Length::Fixed(18.0)).style(ui::boxed(if on { p.accent } else { Color::TRANSPARENT }, 2.0, None));
        let line = row![mark, glyph(cat_glyph(id), if on { p.accent } else { p.muted }, 18.0, 1.8), t1(*label, 15.0, p.text)].spacing(14.0).align_y(iced::Center);
        cats = cats.push(
            mouse_area(container(line).padding(Padding { top: 10.0, right: 12.0, bottom: 10.0, left: 4.0 }).width(Length::Fill).style(ui::boxed(if on { Pal::mix(rail_bg, p.text, 0.07) } else { Color::TRANSPARENT }, p.r(8.0), None)))
                .on_press(Msg::Cat(i))
                .interaction(mouse::Interaction::Pointer),
        );
    }
    let rail = container(column![row![avatar, who].spacing(14.0).align_y(iced::Center), gap(1.0, 18.0), search, gap(1.0, 14.0), scrollable(cats).height(Length::Fill)])
        .padding(Padding { top: 20.0, right: 18.0, bottom: 12.0, left: 18.0 })
        .width(Length::Fixed(320.0))
        .height(Length::Fill)
        .style(ui::boxed(rail_bg, 0.0, None));

    let body: E = if !a.query.is_empty() { search_page(a) } else { page_body(a) };
    let title: E = crumbs(a);
    let mut main = Column::new().push(title).push(gap(1.0, 18.0));
    if !a.error.is_empty() {
        main = main.push(note(&p, a.error.clone(), true));
    }
    if !a.note.is_empty() {
        main = main.push(note(&p, a.note.clone(), false));
    }
    let main = container(main.push(scrollable(container(body).padding(Padding { top: 0.0, right: 14.0, bottom: 24.0, left: 0.0 })).height(Length::Fill)))
        .padding(Padding { top: 18.0, right: 36.0, bottom: 0.0, left: 44.0 })
        .width(Length::Fill)
        .height(Length::Fill);

    let titlebar = ui::title_bar(&p, G::Gear, "Hearth Settings", None, TitleMsgs { drag: Msg::Drag, maximize: Msg::Maximize, minimize: Some(Msg::Minimize), close: Msg::Close }, rail_bg);
    let page = column![titlebar, row![rail, main].height(Length::Fill), hline(p.line), hints(a)];
    let base: E = container(page).width(Length::Fill).height(Length::Fill).style(ui::boxed(p.bg, 0.0, None)).into();
    let base = match &a.join {
        Some((ssid, pw)) => ui::dialog(base, join_dialog(a, ssid, pw), Some(Msg::WifiCancel)),
        None => base,
    };
    ui::edges(base, Msg::Resize)
}

fn crumbs(a: &App) -> E<'_> {
    let p = a.p;
    if !a.query.is_empty() {
        return tb("Search", 34.0, p.text).into();
    }
    let pg = page(a.page);
    let cat = CATS[a.cat];
    let label = pg.map(|x| x.label).unwrap_or("Settings");
    if pg.map(|x| x.id == cat.2).unwrap_or(true) || label == cat.1 {
        tb(label, 34.0, p.text).into()
    } else {
        row![
            mouse_area(tb(cat.1, 34.0, p.muted)).on_press(Msg::Cat(a.cat)).interaction(mouse::Interaction::Pointer),
            container(glyph(G::Chevron, p.text, 22.0, 2.4)).padding(Padding { top: 6.0, right: 0.0, bottom: 0.0, left: 0.0 }),
            tb(label, 34.0, p.text)
        ]
        .spacing(12.0)
        .align_y(iced::Center)
        .into()
    }
}

fn hint<'a>(p: &Pal, k: &'a str, label: &'a str) -> E<'a> {
    let key = container(tb(k, 11.0, p.text)).center_x(Length::Fixed(if k.len() > 1 { 30.0 } else { 24.0 })).center_y(Length::Fixed(24.0)).style(ui::boxed(Color::TRANSPARENT, 12.0, Some((p.text, 1.2))));
    row![key, t1(label, 14.0, p.text)].spacing(8.0).align_y(iced::Center).into()
}

fn hints(a: &App) -> E<'_> {
    let p = a.p;
    let mut r = Row::new().spacing(24.0).align_y(iced::Center).push(hint(&p, "A", "Select")).push(hint(&p, "B", "Back")).push(hint(&p, "Y", "Search"));
    if a.keep.is_some() {
        r = r.push(hint(&p, "X", "Keep these settings"));
    }
    r = r.push(Space::new().width(Length::Fill)).push(row![hint(&p, "LB", ""), hint(&p, "RB", "Previous / next category")].spacing(6.0)).push(hint(&p, "≡", "Close"));
    container(r).padding(Padding { top: 0.0, right: 26.0, bottom: 0.0, left: 26.0 }).center_y(Length::Fixed(52.0)).width(Length::Fill).style(ui::boxed(Pal::mix(p.bg, p.panel, 0.55), 0.0, None)).into()
}

fn search_page(a: &App) -> E<'_> {
    let p = a.p;
    let q = a.query.to_lowercase();
    let mut c = Column::new().spacing(8.0);
    let mut n = 0;
    for pg in PAGES.iter().filter(|x| !x.desc.is_empty() || x.id == "home" || x.id == "appearance" || x.id == "updates") {
        let hay = format!("{} {} {}", pg.label, pg.desc, pg.words).to_lowercase();
        if q.split_whitespace().all(|w| hay.contains(w)) {
            let cat = CATS.iter().find(|c| c.0 == pg.cat).map(|c| c.1).unwrap_or("");
            let sub = if pg.built { format!("{cat} › {}", pg.desc) } else { format!("{cat} · not in this app yet") };
            c = c.push(link_row(&p, page_glyph(pg.id), pg.label, sub, Msg::SearchPick(pg.id)));
            n += 1;
        }
    }
    if n == 0 {
        c = c.push(note(&p, format!("Nothing in Settings matches \"{}\".", a.query), false));
    }
    c.into()
}

fn page_body(a: &App) -> E<'_> {
    match a.page {
        "home" => home(a),
        "display" => display(a),
        "sound" => sound(a),
        "power" => power(a),
        "storage" => storage(a),
        "your-desktop" => your_desktop(a),
        "startup" => startup(a),
        "datetime" => datetime(a),
        "about" => about(a),
        "wifi" => wifi(a),
        "bluetooth" => bluetooth(a),
        "appearance" => appearance(a),
        "updates" => updates(a),
        _ => category(a),
    }
}

/// A category's own page: its pages as rows. Pages not in this app yet say so plainly and open nothing.
fn category(a: &App) -> E<'_> {
    let p = a.p;
    let cat = CATS[a.cat].0;
    let mut c = Column::new().spacing(8.0);
    let mut unbuilt = Vec::new();
    for pg in PAGES.iter().filter(|x| x.cat == cat && !x.desc.is_empty()) {
        if pg.built {
            c = c.push(link_row(&p, page_glyph(pg.id), pg.label, pg.desc, Msg::Open(pg.id)));
        } else {
            unbuilt.push(pg);
        }
    }
    if !unbuilt.is_empty() {
        c = c.push(head(&p, "Not in this app yet"));
        for pg in unbuilt {
            c = c.push(srow(&p, page_glyph(pg.id), pg.label, pg.desc, t1("Coming next", 13.0, p.muted).into()));
        }
    }
    c.into()
}

// ------------------------------------------------------------------ Home (settings-A1)
fn home(a: &App) -> E<'_> {
    let p = a.p;
    let i = &a.info;
    let shot = container(Space::new()).width(Length::Fixed(220.0)).height(Length::Fixed(124.0)).style(ui::boxed(Pal::mix(p.bg2, p.accent, 0.18), p.r(8.0), Some((Color::BLACK, 6.0))));
    let mut facts = vec![format!("Hearth OS {}", data::shown_version(&a.updates.booted)), "Desktop Mode".to_string()];
    if !i.gpu.is_empty() {
        facts.push(i.gpu.clone());
    }
    if !i.cpu.is_empty() {
        facts.push(data::short_cpu(&i.cpu));
    }
    let pc_words = column![
        tb(if i.host.is_empty() { "This PC".to_string() } else { i.host.clone() }, 30.0, p.text),
        t(facts.join(" · "), 14.0, p.muted),
        row![pill(&p, "Rename", true, Some(Msg::Rename)), pill(&p, "About this PC", false, Some(Msg::Open("about")))].spacing(10.0)
    ]
    .spacing(8.0);
    let pc: E = if let Some(n) = &a.rename {
        card(&p, row![shot, rename_box(a, n)].spacing(26.0).align_y(iced::Center))
    } else {
        card(&p, row![shot, pc_words].spacing(26.0).align_y(iced::Center))
    };
    let wifi_s = if !a.wifi.wired.is_empty() {
        "Wired · connected".to_string()
    } else if let Some(n) = a.wifi.nets.iter().find(|n| n.active) {
        format!("{} · {}", n.ssid, data::signal_word(n.signal))
    } else if !a.wifi_read {
        "…".into()
    } else if a.wifi.on {
        "Not connected".into()
    } else {
        "Off".into()
    };
    let bt_n = a.bt.devs.iter().filter(|d| d.connected).count();
    let bt_s = if !a.bt_read { "…".to_string() } else if !a.bt.has { "No Bluetooth".into() } else if !a.bt.on { "Off".into() } else { format!("On · {bt_n} connected") };
    let up_s = if !a.updates.staged.is_empty() {
        "Restart to update".to_string()
    } else if !a.updates.available.is_empty() {
        format!("{} available", data::shown_version(&a.updates.available))
    } else if a.updates.booted.is_empty() {
        "…".into()
    } else {
        format!("Up to date · {}", data::shown_version(&a.updates.booted))
    };
    let tiles = column![
        row![tile(&p, G::Wifi, "Wi-Fi", wifi_s, "wifi"), tile(&p, G::Bt, "Bluetooth", bt_s, "bluetooth")].spacing(16.0),
        row![tile(&p, G::Update, "Updates", up_s, "updates"), tile(&p, G::Link, "Hearth Link", "Not in this app yet".into(), "link")].spacing(16.0)
    ]
    .spacing(16.0)
    .width(Length::FillPortion(9));
    let top = row![container(pc).width(Length::FillPortion(13)), tiles].spacing(18.0);

    let night = a.display.night;
    let rec = column![
        row![ts("Recommended settings", 17.0, p.text), Space::new().width(Length::Fill), mouse_area(tm("See more", 14.0, p.accent)).on_press(Msg::Cat(1)).interaction(mouse::Interaction::Pointer)],
        gap(1.0, 6.0),
        rec_row(&p, G::Moon, "Night light", "Warmer colours from sunset to sunrise", switch(&p, night, Some(Msg::Night(!night)))),
        hline(p.line),
        rec_row(&p, G::Power, "Start in", "What this PC opens when it turns on", seg(&p, vec![
            ("Game Mode".into(), a.start == "game", Some(Msg::StartIn("game"))),
            ("Desktop Mode".into(), a.start == "desktop", Some(Msg::StartIn("desktop"))),
        ])),
    ]
    .spacing(10.0);
    let storage = column![
        row![ts("Storage", 17.0, p.text), Space::new().width(Length::Fill), mouse_area(tm("Manage", 14.0, p.accent)).on_press(Msg::Open("storage")).interaction(mouse::Interaction::Pointer)],
        drive_rows(&p, &a.storage)
    ]
    .spacing(14.0);
    let mut changed = Column::new().spacing(10.0).push(row![ts("Changed lately", 17.0, p.text), Space::new().width(Length::Fill)]);
    if a.history.is_empty() {
        changed = changed.push(t("Nothing changed in Settings yet.", 13.0, p.muted));
    }
    for (n, c) in a.history.iter().take(3).enumerate() {
        if n > 0 {
            changed = changed.push(hline(p.line));
        }
        changed = changed.push(
            row![
                column![t(format!("{} → {}", data::setting_name(&c.key), c.to), 15.0, p.text), t(format!("{} · {}", c.who, hearth_ui::time::when(c.at)), 13.0, p.muted)].spacing(2.0).width(Length::Fill),
                btn(&p, "Undo", Kind::Normal, false, Some(Msg::Undo(c.id)))
            ]
            .align_y(iced::Center),
        );
    }
    let lower = row![
        container(card(&p, rec)).width(Length::FillPortion(1)),
        column![card(&p, storage), card(&p, changed)].spacing(18.0).width(Length::FillPortion(1))
    ]
    .spacing(18.0);
    column![top, lower].spacing(18.0).into()
}

fn tile<'a>(p: &Pal, g: G, title: &'a str, sub: String, to: &'static str) -> E<'a> {
    mouse_area(
        container(row![icon_box(p, g), column![tm(title, 15.0, p.text), t1(sub, 13.0, p.muted)].spacing(2.0)].spacing(14.0).align_y(iced::Center))
            .padding(16.0)
            .width(Length::Fill)
            .style(ui::boxed(p.panel, p.r(12.0), None)),
    )
    .on_press(Msg::Open(to))
    .interaction(mouse::Interaction::Pointer)
    .into()
}

fn fact<'a>(p: &Pal, k: &'a str, v: String) -> E<'a> {
    row![container(t(k, 14.0, p.muted)).width(Length::Fixed(180.0)), t(v, 14.0, p.text)].into()
}

fn rec_row<'a>(p: &Pal, g: G, title: &'a str, sub: &'a str, control: E<'a>) -> E<'a> {
    row![icon_box(p, g), column![tm(title, 15.0, p.text), t(sub, 13.0, p.muted)].spacing(2.0).width(Length::Fill), control].spacing(16.0).align_y(iced::Center).into()
}

fn rename_box<'a>(a: &'a App, n: &'a str) -> E<'a> {
    let p = a.p;
    column![
        ts("Rename this PC", 18.0, p.text),
        t("Letters, numbers and dashes. Your other devices see this name.", 13.0, p.muted),
        text_input("chris-hearth", n).on_input(Msg::RenameText).on_submit(Msg::RenameSave).size(15.0).padding(10.0).style(input_style(p)).width(Length::Fixed(320.0)),
        row![btn(&p, "Save", Kind::Primary, false, if n.is_empty() { None } else { Some(Msg::RenameSave) }), btn(&p, "Cancel", Kind::Normal, false, Some(Msg::RenameCancel))].spacing(8.0)
    ]
    .spacing(10.0)
    .into()
}

// ------------------------------------------------------------------ System › Display (settings-A2)
fn display(a: &App) -> E<'_> {
    let p = a.p;
    let d = &a.display;
    let mut c = Column::new().spacing(10.0);
    if !d.ok {
        c = c.push(note(&p, "Reading your displays…".into(), false));
    }
    if let Some(main) = d.outputs.first() {
        let mut boxes = Row::new().spacing(14.0).align_y(iced::Center);
        for (n, o) in d.outputs.iter().enumerate() {
            let big = n == 0;
            boxes = boxes.push(
                container(tb(format!("{}", n + 1), if big { 34.0 } else { 26.0 }, p.text))
                    .center_x(Length::Fixed(if big { 240.0 } else { 156.0 }))
                    .center_y(Length::Fixed(if big { 136.0 } else { 88.0 }))
                    .style(ui::boxed(if big { Pal::mix(p.panel2, p.accent, 0.08) } else { p.panel2 }, p.r(8.0), Some((if big { p.accent } else { p.line }, 2.0)))),
            );
            let _ = o;
        }
        let cur = main.cur();
        let facts = match cur {
            Some(m) => format!("{} × {} · {} Hz · scale {}%", m.w, m.h, m.hz.round(), (main.scale * 100.0).round()),
            None => String::new(),
        };
        let gpu = if a.info.gpu.is_empty() { String::new() } else { format!(" · connected to the {}", a.info.gpu) };
        c = c.push(card(&p, row![boxes, column![ts(format!("{} · main display", main.name), 18.0, p.text), t(format!("{facts}{gpu}"), 13.0, p.muted)].spacing(6.0)].spacing(26.0).align_y(iced::Center)));
        if let Some(k) = &a.keep {
            c = c.push(card(&p, row![
                column![ts("Keep these display settings?", 15.0, p.text), t(format!("The old ones come back by themselves in {} s.", k.left), 13.0, p.muted)].spacing(2.0).width(Length::Fill),
                btn(&p, "Keep these settings", Kind::Primary, false, Some(Msg::Keep)),
                btn(&p, "Put back", Kind::Normal, false, Some(Msg::Revert))
            ].spacing(10.0).align_y(iced::Center)));
        }
        c = c.push(head(&p, "Brightness & colour"));
        if let Some(b) = d.bright {
            c = c.push(srow(&p, G::Display, "Brightness", format!("{b}%"), slider(1..=100, b, Msg::Bright).on_release(Msg::BrightSet).width(Length::Fixed(280.0)).into()));
        }
        c = c.push(srow(&p, G::Moon, "Night light", "Warmer colours from sunset to sunrise", switch(&p, d.night, Some(Msg::Night(!d.night)))));
        c = c.push(head(&p, "Scale & layout"));
        let name = main.name.clone();
        let scales = [1.0f32, 1.25, 1.5, 1.75, 2.0];
        c = c.push(srow(&p, G::Grid, "Scale", "Text, apps and the bar", seg(&p, scales.iter().map(|s| (format!("{}%", (s * 100.0).round()), (main.scale - s).abs() < 0.01, Some(Msg::Scale(name.clone(), *s)))).collect())));
        let sizes = main.sizes();
        let rec = main.preferred.first().and_then(|id| main.modes.iter().find(|m| &m.id == id)).map(|m| format!("Recommended: {} × {}", m.w, m.h)).unwrap_or_default();
        let shown: Vec<(u32, u32)> = sizes.iter().take(3).cloned().collect();
        c = c.push(srow(&p, G::Display, "Resolution", rec, seg(&p, shown.iter().map(|(w, h)| (format!("{w} × {h}"), cur.map(|m| m.w == *w && m.h == *h).unwrap_or(false), Some(Msg::Res(name.clone(), *w, *h)))).collect())));
        let rates = main.rates();
        c = c.push(srow(&p, G::Update, "Refresh rate", "How many pictures a second the display shows", seg(&p, rates.iter().take(4).map(|(hz, id)| (format!("{hz} Hz"), cur.map(|m| &m.id == id).unwrap_or(false), Some(Msg::Rate(name.clone(), id.clone())))).collect())));
    } else if d.ok {
        c = c.push(note(&p, "No display was found.".into(), false));
    }
    c.into()
}

// ------------------------------------------------------------------ System › Sound
fn sound(a: &App) -> E<'_> {
    let p = a.p;
    let s = &a.sound;
    let mut c = Column::new().spacing(10.0);
    if !s.ok {
        c = c.push(note(&p, "Reading sound…".into(), false));
        return c.into();
    }
    c = c.push(head(&p, "Output"));
    let vol = (s.vol * 100.0).round() as u32;
    c = c.push(srow(&p, G::Sound, "Volume", if s.muted { "Muted".to_string() } else { format!("{vol}%") }, row![
        slider(0.0..=1.0, s.vol.min(1.0), Msg::Vol).step(0.01).on_release(Msg::VolSet).width(Length::Fixed(260.0)),
        btn(&p, if s.muted { "Unmute" } else { "Mute" }, Kind::Normal, false, Some(Msg::Mute))
    ].spacing(14.0).align_y(iced::Center).into()));
    for o in &s.outs {
        let on = o.name == s.out;
        c = c.push(srow(&p, G::Sound, o.desc.clone(), if on { "Playing here" } else { "" }, if on { t1("In use", 13.0, p.accent).into() } else { btn(&p, "Use", Kind::Normal, false, Some(Msg::Out(o.name.clone()))) }));
    }
    c = c.push(head(&p, "Input"));
    let mic = (s.mic * 100.0).round() as u32;
    c = c.push(srow(&p, G::Sound, "Microphone", if s.mic_muted { "Muted".to_string() } else { format!("{mic}%") }, row![
        slider(0.0..=1.0, s.mic.min(1.0), Msg::Mic).step(0.01).on_release(Msg::MicSet).width(Length::Fixed(260.0)),
        btn(&p, if s.mic_muted { "Unmute" } else { "Mute" }, Kind::Normal, false, Some(Msg::MicMute))
    ].spacing(14.0).align_y(iced::Center).into()));
    for o in &s.ins {
        let on = o.name == s.inp;
        c = c.push(srow(&p, G::Sound, o.desc.clone(), if on { "Recording from here" } else { "" }, if on { t1("In use", 13.0, p.accent).into() } else { btn(&p, "Use", Kind::Normal, false, Some(Msg::In(o.name.clone()))) }));
    }
    c.into()
}

// ------------------------------------------------------------------ System › Power, Storage, Startup, Date & time, Your desktop, About
fn power(a: &App) -> E<'_> {
    let p = a.p;
    let w = &a.power;
    let mut c = Column::new().spacing(10.0);
    if let Some(b) = w.battery {
        c = c.push(srow(&p, G::Power, "Battery", format!("{b}% · {}", if w.charging.is_empty() { "Unknown" } else { w.charging.as_str() }), bar_of(&p, b as f64, 100.0)));
    }
    if w.profiles.is_empty() {
        c = c.push(srow(&p, G::Power, "Power mode", "This PC doesn't offer power modes", t1("", 13.0, p.muted).into()));
    } else {
        c = c.push(srow(&p, G::Power, "Power mode", "Power saver for a longer battery, Performance for games", seg(&p, w.profiles.iter().map(|pr| (data::profile_name(pr).to_string(), *pr == w.profile, Some(Msg::Profile(pr.clone())))).collect())));
    }
    c.into()
}

fn storage(a: &App) -> E<'_> {
    let p = a.p;
    let mut c = Column::new().spacing(10.0);
    c = c.push(card(&p, drive_rows(&p, &a.storage)));
    c = c.push(link_row(&p, G::Drive, "Disk Usage", "What's using the space, folder by folder", Msg::OpenApp("hearth-disk-usage")));
    c.into()
}

fn startup(a: &App) -> E<'_> {
    let p = a.p;
    column![srow(&p, G::Power, "Start in", "What this PC opens when it turns on", seg(&p, vec![
        ("Game Mode".into(), a.start == "game", Some(Msg::StartIn("game"))),
        ("Desktop Mode".into(), a.start == "desktop", Some(Msg::StartIn("desktop"))),
    ]))]
    .into()
}

fn datetime(a: &App) -> E<'_> {
    let p = a.p;
    let tm_ = &a.time;
    let mut c = Column::new().spacing(10.0);
    c = c.push(srow(&p, G::Clock, "Set the time by itself", "From the internet (NTP)", switch(&p, tm_.ntp, Some(Msg::Ntp(!tm_.ntp)))));
    c = c.push(srow(&p, G::Clock, "Time zone", if tm_.zone.is_empty() { "…".to_string() } else { tm_.zone.clone() }, text_input("Find a time zone", &a.zone_query).on_input(Msg::ZoneQuery).size(14.0).padding(8.0).style(input_style(p)).width(Length::Fixed(280.0)).into()));
    if a.zone_query.chars().count() >= 2 {
        let q = a.zone_query.to_lowercase().replace(' ', "_");
        let mut list = Column::new().spacing(4.0);
        for z in tm_.zones.iter().filter(|z| z.to_lowercase().contains(&q)).take(12) {
            list = list.push(mouse_area(container(t(z.replace('_', " "), 14.0, p.text)).padding(Padding { top: 8.0, right: 14.0, bottom: 8.0, left: 14.0 }).width(Length::Fill).style(ui::boxed(p.panel, p.r(8.0), None))).on_press(Msg::Zone(z.clone())).interaction(mouse::Interaction::Pointer));
        }
        c = c.push(list);
    }
    c.into()
}

fn your_desktop(a: &App) -> E<'_> {
    let p = a.p;
    let d = &a.desk;
    let mut opts: Vec<(&'static str, &'static str, &'static str)> = vec![("plasma", "KDE Plasma", "The desktop, with Hearth's look (the default)")];
    if d.rust {
        opts.push(("firehold-rust", "Firehold (Rust, test)", "Hearth's own Couch and Desk, all in Rust. If it doesn't start, KDE Plasma opens"));
    }
    if d.plasma_fh {
        opts.push(("firehold-plasma", "Firehold (Plasma, test)", "Firehold's Couch and Desk on KDE Plasma"));
    }
    let mut c = Column::new().spacing(10.0).push(t("What Desktop Mode opens. Nothing changes until you switch to Desktop Mode again.", 14.0, p.muted));
    for (id, name, sub) in opts {
        let on = d.chosen == id;
        c = c.push(srow(&p, G::Grid, name, sub, if on { t1("Picked", 13.0, p.accent).into() } else { btn(&p, "Use", Kind::Normal, false, Some(Msg::Desk(id))) }));
    }
    if a.desk_saved {
        c = c.push(card(&p, row![
            column![ts("Saved", 15.0, p.text), t("It opens the next time you switch to Desktop Mode. Switching now closes open apps.", 13.0, p.muted)].spacing(2.0).width(Length::Fill),
            btn(&p, "Switch now", Kind::Primary, false, Some(Msg::SwitchNow))
        ].spacing(10.0).align_y(iced::Center)));
    }
    c.into()
}

fn about(a: &App) -> E<'_> {
    let p = a.p;
    let i = &a.info;
    let mut c = Column::new().spacing(10.0);
    if let Some(n) = &a.rename {
        c = c.push(card(&p, rename_box(a, n)));
    } else {
        c = c.push(srow(&p, G::Info, if i.host.is_empty() { "This PC".to_string() } else { i.host.clone() }, "This PC's name on your network", btn(&p, "Rename", Kind::Normal, false, Some(Msg::Rename))));
    }
    let mem = if i.mem_kb > 0 { ui::bytes(i.mem_kb as f64 * 1024.0) } else { String::new() };
    c = c.push(card(&p, column![
        fact(&p, "Hearth OS", format!("{} {}", data::shown_version(&a.updates.booted), if i.os.contains("Fedora") || i.os.to_lowercase().contains("anatase") || a.updates.booted_image.contains("hearth-os:") { "· Fedora edition" } else { "" })),
        fact(&p, "Built on", i.os.clone()),
        fact(&p, "Image", a.updates.booted_image.clone()),
        fact(&p, "Kernel", i.kernel.clone()),
        fact(&p, "Processor", data::short_cpu(&i.cpu)),
        fact(&p, "Graphics", i.gpu.clone()),
        fact(&p, "Memory", mem),
        fact(&p, "Desktop", i.desktop.clone()),
    ].spacing(10.0)));
    c = c.push(link_row(&p, G::Info, "System Info", "Everything about this PC's parts, to copy or send", Msg::OpenApp("hearth-system-info")));
    c.into()
}

// ------------------------------------------------------------------ Network › Wi-Fi
fn wifi(a: &App) -> E<'_> {
    let p = a.p;
    let w = &a.wifi;
    let mut c = Column::new().spacing(10.0);
    if !a.wifi_read {
        return note(&p, "Looking at your network…".into(), false);
    }
    if !w.wired.is_empty() {
        c = c.push(srow(&p, G::Link, "Ethernet", format!("Connected · {}", w.wired), t1("", 13.0, p.muted).into()));
    }
    if !w.has {
        c = c.push(note(&p, "This PC has no Wi-Fi.".into(), false));
        return c.into();
    }
    c = c.push(srow(&p, G::Wifi, "Wi-Fi", if w.on { "Find and connect to wireless networks" } else { "Off" }, switch(&p, w.on, Some(Msg::WifiOn(!w.on)))));
    if w.on {
        c = c.push(row![head(&p, "Networks"), Space::new().width(Length::Fill), container(btn(&p, if a.busy == Some("wifi") { "Connecting…" } else { "Look again" }, Kind::Normal, false, Some(Msg::WifiScan))).padding(Padding { top: 8.0, right: 0.0, bottom: 0.0, left: 0.0 })].align_y(iced::Center));
        for n in &w.nets {
            let sub = format!("{}{}{}", if n.active { "Connected · " } else { "" }, data::signal_word(n.signal), if n.secure { " · secured" } else { " · open" });
            let ctl: E = if n.active {
                row![btn(&p, "Disconnect", Kind::Normal, false, Some(Msg::WifiDisconnect(n.ssid.clone()))), btn(&p, "Forget", Kind::Normal, false, Some(Msg::WifiForget(n.ssid.clone())))].spacing(8.0).into()
            } else if n.known {
                row![btn(&p, "Connect", Kind::Primary, false, Some(Msg::WifiPick(n.ssid.clone()))), btn(&p, "Forget", Kind::Normal, false, Some(Msg::WifiForget(n.ssid.clone())))].spacing(8.0).into()
            } else {
                btn(&p, "Connect", Kind::Normal, false, Some(Msg::WifiPick(n.ssid.clone())))
            };
            c = c.push(srow(&p, G::Wifi, n.ssid.clone(), sub, ctl));
        }
        if w.nets.is_empty() {
            c = c.push(note(&p, "No networks found. Look again in a moment.".into(), false));
        }
    }
    c.into()
}

fn join_dialog<'a>(a: &'a App, ssid: &'a str, pw: &'a str) -> E<'a> {
    let p = a.p;
    ui::dialog_box(&p, 440.0, column![
        ts(format!("Connect to {ssid}"), 17.0, p.text),
        t("The network's password:", 13.0, p.muted),
        text_input("Password", pw).secure(true).on_input(Msg::WifiPw).on_submit(Msg::WifiJoin).size(15.0).padding(10.0).style(input_style(p)),
        container(row![btn(&p, "Connect", Kind::Primary, true, if pw.len() >= 8 { Some(Msg::WifiJoin) } else { None }), btn(&p, "Cancel", Kind::Normal, false, Some(Msg::WifiCancel))].spacing(8.0)).align_right(Length::Fill)
    ].spacing(12.0))
}

// ------------------------------------------------------------------ Bluetooth & devices › Bluetooth
fn bluetooth(a: &App) -> E<'_> {
    let p = a.p;
    let b = &a.bt;
    if !a.bt_read {
        return note(&p, "Looking for Bluetooth…".into(), false);
    }
    if !b.has {
        return note(&p, "This PC has no Bluetooth, or it's switched off in its firmware.".into(), false);
    }
    let mut c = Column::new().spacing(10.0);
    c = c.push(srow(&p, G::Bt, "Bluetooth", if b.on { "Headphones, controllers and other wireless devices" } else { "Off" }, switch(&p, b.on, Some(Msg::BtOn(!b.on)))));
    if b.on {
        let paired: Vec<_> = b.devs.iter().filter(|d| d.paired).collect();
        let others: Vec<_> = b.devs.iter().filter(|d| !d.paired).collect();
        c = c.push(head(&p, "Your devices"));
        if paired.is_empty() {
            c = c.push(note(&p, "None paired yet.".into(), false));
        }
        for d in paired {
            let ctl: E = row![
                btn(&p, if d.connected { "Disconnect" } else { "Connect" }, if d.connected { Kind::Normal } else { Kind::Primary }, false, if a.busy == Some("bt") { None } else { Some(Msg::BtConnect(d.mac.clone(), !d.connected)) }),
                btn(&p, "Forget", Kind::Normal, false, Some(Msg::BtForget(d.mac.clone())))
            ]
            .spacing(8.0)
            .into();
            c = c.push(srow(&p, G::Bt, d.name.clone(), if d.connected { "Connected" } else { "Not connected" }, ctl));
        }
        c = c.push(row![head(&p, "Add a device"), Space::new().width(Length::Fill), container(btn(&p, if a.bt_scanning { "Looking…" } else { "Look for devices" }, Kind::Normal, false, if a.bt_scanning { None } else { Some(Msg::BtScan) })).padding(Padding { top: 8.0, right: 0.0, bottom: 0.0, left: 0.0 })].align_y(iced::Center));
        c = c.push(t("Put the device in pairing mode, then look for it here.", 13.0, p.muted));
        for d in others {
            c = c.push(srow(&p, G::Bt, d.name.clone(), d.mac.clone(), btn(&p, if a.busy == Some("bt") { "Pairing…" } else { "Pair" }, Kind::Normal, false, if a.busy == Some("bt") { None } else { Some(Msg::BtPair(d.mac.clone())) })));
        }
    }
    c.into()
}

// ------------------------------------------------------------------ Personalization
fn appearance(a: &App) -> E<'_> {
    let p = a.p;
    let pal = hearth_ui::theme::palette_map();
    let cur_look = a.desktop.get("look").and_then(|v| v.as_str()).unwrap_or("hearth").to_string();
    let cur_mode = a.desktop.get("mode").and_then(|v| v.as_str()).unwrap_or("dark").to_string();
    let cur_accent = a.desktop.get("accent").and_then(|v| v.as_str()).unwrap_or("").to_lowercase();
    let mut looks: Vec<(String, String, i64, serde_json::Map<String, serde_json::Value>)> = pal
        .iter()
        .filter(|(k, v)| !k.starts_with('_') && v.get("dark").map(|d| d.is_object()).unwrap_or(false))
        .map(|(k, v)| (k.clone(), v.get("label").and_then(|l| l.as_str()).unwrap_or(k).to_string(), v.get("order").and_then(|o| o.as_i64()).unwrap_or(99), v.get("dark").and_then(|d| d.as_object()).cloned().unwrap_or_default()))
        .collect();
    looks.sort_by_key(|l| l.2);
    let mut c = Column::new().spacing(10.0);
    c = c.push(t("Hearth's look for the whole desktop: KDE's Global Theme, its colours and Hearth's apps all follow it. A Global Theme you pick in KDE's System Settings shows here too.", 14.0, p.muted));
    c = c.push(head(&p, "Look"));
    let mut grid = Column::new().spacing(14.0);
    let mut line = Row::new().spacing(14.0);
    for (n, (id, label, _, dark)) in looks.iter().enumerate() {
        let col = |k: &str, d: &str| dark.get(k).and_then(|v| v.as_str()).and_then(hearth_ui::theme::hex).unwrap_or_else(|| hearth_ui::theme::hex(d).unwrap());
        let (bg, panel, accent, text) = (col("bg", "#111111"), col("panel2", "#222222"), col("ember", "#ff8a3d"), col("text", "#eeeeee"));
        let on = *id == cur_look;
        let preview = container(column![
            container(Space::new()).width(Length::Fixed(110.0)).height(Length::Fixed(6.0)).style(ui::boxed(text, 3.0, None)),
            container(Space::new()).width(Length::Fixed(80.0)).height(Length::Fixed(5.0)).style(ui::boxed(Pal::a(text, 0.5), 3.0, None)),
            row![container(Space::new()).width(Length::Fixed(40.0)).height(Length::Fixed(14.0)).style(ui::boxed(accent, 3.0, None)), container(Space::new()).width(Length::Fixed(40.0)).height(Length::Fixed(14.0)).style(ui::boxed(Pal::mix(panel, text, 0.1), 3.0, None))].spacing(6.0)
        ].spacing(8.0))
        .padding(16.0)
        .width(Length::Fill)
        .height(Length::Fixed(96.0))
        .style(ui::boxed(bg, p.r(8.0), Some((if on { p.accent } else { Pal::mix(bg, text, 0.15) }, if on { 3.0 } else { 1.0 }))));
        let tile = mouse_area(column![preview, container(if on { ts(label.clone(), 14.0, p.text) } else { t(label.clone(), 14.0, p.text) }).center_x(Length::Fill)].spacing(8.0))
            .on_press(Msg::Look(id.clone()))
            .interaction(mouse::Interaction::Pointer);
        line = line.push(container(tile).width(Length::FillPortion(1)));
        if n % 4 == 3 {
            grid = grid.push(line);
            line = Row::new().spacing(14.0);
        }
    }
    let rest = looks.len() % 4;
    if rest != 0 {
        for _ in rest..4 {
            line = line.push(Space::new().width(Length::FillPortion(1)));
        }
        grid = grid.push(line);
    }
    c = c.push(card(&p, grid));
    c = c.push(head(&p, "Colours"));
    c = c.push(srow(&p, G::Moon, "Mode", "Dark or light", seg(&p, vec![("Dark".into(), cur_mode != "light", Some(Msg::Mode("dark"))), ("Light".into(), cur_mode == "light", Some(Msg::Mode("light")))])));
    let mut sw = Row::new().spacing(10.0).align_y(iced::Center);
    let own_on = cur_accent.is_empty();
    sw = sw.push(mouse_area(container(t1("The look's own", 13.0, if own_on { p.ink } else { p.text })).padding(Padding { top: 5.0, right: 12.0, bottom: 5.0, left: 12.0 }).style(ui::boxed(if own_on { p.accent } else { Color::TRANSPARENT }, 14.0, Some((p.line, 1.0))))).on_press(Msg::Accent(String::new())).interaction(mouse::Interaction::Pointer));
    if let Some(list) = pal.get("accents").and_then(|v| v.as_array()) {
        for acc in list {
            let Some(hex) = acc.get("color").and_then(|c| c.as_str()) else { continue };
            let Some(cc) = hearth_ui::theme::hex(hex) else { continue };
            let on = cur_accent == hex.to_lowercase();
            sw = sw.push(mouse_area(container(Space::new()).width(Length::Fixed(26.0)).height(Length::Fixed(26.0)).style(ui::boxed(cc, 13.0, Some((if on { p.text } else { Color::TRANSPARENT }, 2.5))))).on_press(Msg::Accent(hex.to_string())).interaction(mouse::Interaction::Pointer));
        }
    }
    c = c.push(srow(&p, G::Palette, "Accent colour", "Buttons, switches and what's picked", sw.into()));
    c.into()
}

// ------------------------------------------------------------------ Updates & recovery
fn updates(a: &App) -> E<'_> {
    let p = a.p;
    let u = &a.updates;
    let busy = a.busy;
    let (title, sub) = if busy == Some("update") {
        ("Updating…".to_string(), "Downloading and putting the update in place. Keep using the PC; don't turn it off.".to_string())
    } else if busy == Some("check") {
        ("Looking for updates…".to_string(), String::new())
    } else if !u.staged.is_empty() {
        (format!("{} is ready", data::shown_version(&u.staged)), "Restart to start it. The version you have now stays in the boot menu.".to_string())
    } else if !u.available.is_empty() {
        (format!("{} is available", data::shown_version(&u.available)), "A restore point is made first, and the version you have now stays in the boot menu.".to_string())
    } else if !u.read {
        ("Reading…".to_string(), String::new())
    } else if !u.error.is_empty() {
        ("Couldn't read the updates".to_string(), u.error.clone())
    } else {
        ("You're up to date".to_string(), format!("Hearth OS {}{}", data::shown_version(&u.booted), if u.booted_image.is_empty() { String::new() } else { format!(" · {}", u.booted_image) }))
    };
    let action: E = if !u.staged.is_empty() {
        btn(&p, "Restart now", Kind::Primary, false, Some(Msg::Restart))
    } else if !u.available.is_empty() {
        btn(&p, "Update now", Kind::Primary, false, if busy.is_none() { Some(Msg::Update) } else { None })
    } else {
        btn(&p, "Check for updates", Kind::Primary, false, if busy.is_none() { Some(Msg::Check) } else { None })
    };
    let top = card(&p, row![
        container(glyph(G::Update, p.accent, 28.0, 2.2)).center_x(Length::Fixed(56.0)).center_y(Length::Fixed(56.0)).style(ui::boxed(Pal::mix(p.panel, p.accent, 0.12), 28.0, None)),
        column![ts(title, 20.0, p.text), t(sub, 13.0, p.muted)].spacing(4.0).width(Length::Fill),
        action
    ].spacing(18.0).align_y(iced::Center));
    let mut c = Column::new().spacing(10.0).push(top);
    c = c.push(head(&p, "Recovery"));
    let back_sub = if u.rollback.is_empty() { "There's no earlier version kept on this PC".to_string() } else { format!("{} starts after a restart; this one stays", data::shown_version(&u.rollback)) };
    c = c.push(srow(&p, G::Restore, "Go back a version", back_sub, btn(&p, "Go back", Kind::Normal, false, if u.rollback.is_empty() || busy.is_some() || !u.staged.is_empty() { None } else { Some(Msg::GoBack) })));
    c = c.push(link_row(&p, G::Restore, "System Restore", "Hearth and its settings back to an earlier restore point", Msg::OpenApp("hearth-system-restore")));
    c = c.push(link_row(&p, G::Reliability, "Reliability history", "How Hearth has been running, day by day", Msg::OpenApp("hearth-reliability-history")));
    c.into()
}

