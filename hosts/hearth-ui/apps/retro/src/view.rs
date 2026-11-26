//! What Hearth Retro looks like (board 04): the top bar (its mark and name, LB · Shelves · Set up · BIOS check ·
//! Controllers · Search · RB, how many games and consoles, the time and battery), the shelves (each console's games,
//! the picked one lifted and lit), the Picked card, Set up in one go, the BIOS check, the controllers, and Game
//! Mode's button hints with the credit. Every part is a block (hearth_ui::blocks), moved, sized, hidden or restyled in
//! Edit layout, and Hearth Retro's own buttons, panes and info can be dragged in (KIT).

use crate::{App, Dlg, Msg, Tab};
use hearth_retro::bios::{self, State};
use hearth_retro::emus;
use hearth_retro::library::{self, thousands, Game};
use hearth_ui::blocks::{bl, bls, PageDef};
use hearth_ui::studio::{tile, Kit};
use hearth_ui::ui::{self, gap, t, t1, tb, tm, ts, G};
use hearth_ui::Pal;
use iced::widget::{button, column, container, image, mouse_area, row, scrollable, stack, text, text_input, Column, Row, Space};
use iced::{mouse, Background, Border, Color, Element, Length, Padding, Shadow, Theme};

pub type E<'a> = Element<'a, Msg>;

pub const GREEN: Color = Color::from_rgb(0x9b as f32 / 255.0, 0xe3 as f32 / 255.0, 0x6b as f32 / 255.0);
pub const YELLOW: Color = Color::from_rgb(1.0, 0xd3 as f32 / 255.0, 0x6b as f32 / 255.0);
pub const BLUE: Color = Color::from_rgb(0x5e as f32 / 255.0, 0xc8 as f32 / 255.0, 1.0);
pub const RED: Color = Color::from_rgb(1.0, 0x6b as f32 / 255.0, 0x6b as f32 / 255.0);

pub fn rgb(c: [u8; 3]) -> Color {
    Color::from_rgb8(c[0], c[1], c[2])
}
pub fn hexc(h: u32) -> Color {
    Color::from_rgb8((h >> 16) as u8, (h >> 8) as u8, h as u8)
}
pub fn pad(t: f32, r: f32, b: f32, l: f32) -> Padding {
    Padding { top: t, right: r, bottom: b, left: l }
}

/// A linear gradient between two colours (angle in radians, iced's: PI is top to bottom).
pub fn grad(angle: f32, a: Color, b: Color) -> Background {
    iced::Gradient::Linear(iced::gradient::Linear::new(angle).add_stop(0.0, a).add_stop(1.0, b)).into()
}

/// The app's mark: Hearth's own simple shape (a rounded square in its colours with its letters), no copied icon.
pub fn mark<'a>(letters: &'a str, from: u32, to: u32, size: f32, font: f32) -> E<'a> {
    container(tb(letters, font, Color::WHITE))
        .center_x(Length::Fixed(size))
        .center_y(Length::Fixed(size))
        .style(move |_: &Theme| container::Style { background: Some(grad(2.36, hexc(from), hexc(to))), border: iced::border::rounded(size * 0.28), ..container::Style::default() })
        .into()
}

#[derive(Clone, Copy, PartialEq)]
pub enum Ck {
    Acc,
    Normal,
    Ghost,
}

/// A chip as the boards draw them (.chip, .chip.acc, .chip.ghost; sm 28, 36, lg 44 high).
pub fn chip<'a>(p: &Pal, label: impl text::IntoFragment<'a>, kind: Ck, h: f32, size: f32, on: Option<Msg>) -> E<'a> {
    let p = *p;
    let fg = if kind == Ck::Acc { p.ink } else { p.text };
    let lbl = if kind == Ck::Acc { ts(label, size, fg) } else { t(label, size, fg) };
    let padx = if h >= 44.0 { 20.0 } else if h <= 28.0 { 10.0 } else { 14.0 };
    button(container(lbl.wrapping(text::Wrapping::None)).center_y(Length::Fixed(h)))
        .padding(pad(0.0, padx, 0.0, padx))
        .on_press_maybe(on)
        .style(move |_: &Theme, st: button::Status| {
            let hover = matches!(st, button::Status::Hovered | button::Status::Pressed);
            let bg = match kind {
                Ck::Acc => if hover { Pal::mix(p.accent, Color::WHITE, 0.1) } else { p.accent },
                Ck::Normal => if hover { Pal::mix(p.panel2, p.text, 0.06) } else { p.panel2 },
                Ck::Ghost => if hover { Pal::a(p.text, 0.06) } else { Color::TRANSPARENT },
            };
            button::Style {
                background: Some(Background::Color(bg)),
                text_color: fg,
                border: Border { color: if kind == Ck::Acc { p.accent } else { p.line }, width: 1.0, radius: (if h >= 44.0 { 12.0 } else if h <= 28.0 { 8.0 } else { 10.0 }).into() },
                shadow: Shadow::default(),
                snap: true,
            }
        })
        .into()
}

#[derive(Clone, Copy)]
pub enum Tg {
    Ok,
    Dk,
    Bl,
    Warn,
    Bad,
    Acc,
}

/// A small-caps tag (.tag.ok, .dk, .bl, .warn, .bad).
pub fn tag<'a>(p: &Pal, label: impl text::IntoFragment<'a>, k: Tg) -> E<'a> {
    let (bg, fg, line) = match k {
        Tg::Ok => (hexc(0x21361b), GREEN, hexc(0x3b5a2e)),
        Tg::Dk => (p.panel2, p.muted, p.line),
        Tg::Bl => (hexc(0x173040), BLUE, hexc(0x2d5670)),
        Tg::Warn => (hexc(0x3d2a14), YELLOW, hexc(0x6a4a1e)),
        Tg::Bad => (hexc(0x3d1717), RED, hexc(0x6a2a2a)),
        Tg::Acc => (p.accent, p.ink, p.accent),
    };
    container(tb(label, 11.0, fg).wrapping(text::Wrapping::None)).padding(pad(3.0, 8.0, 3.0, 8.0)).style(ui::boxed(bg, 6.0, Some((line, 1.0)))).into()
}

/// A controller button's hint (.pb: round, A in the accent; ≡, LB, RB square).
pub fn pb<'a>(p: &Pal, label: &'a str, acc: bool, square: bool) -> E<'a> {
    let bg = if acc { p.accent } else { hexc(0xf1e9e2) };
    let w = if square { Length::Shrink } else { Length::Fixed(28.0) };
    container(tb(label, 13.0, p.ink).wrapping(text::Wrapping::None))
        .width(w)
        .height(Length::Fixed(28.0))
        .padding(pad(0.0, if square { 8.0 } else { 0.0 }, 0.0, if square { 8.0 } else { 0.0 }))
        .center_x(w)
        .center_y(Length::Fixed(28.0))
        .style(ui::boxed(bg, if square { 7.0 } else { 14.0 }, None))
        .into()
}

pub fn cap<'a>(p: &Pal, s: &'a str) -> E<'a> {
    tb(s.to_uppercase(), 11.0, p.muted).into()
}

/// A card (.card): the panel, a line, rounded 12.
pub fn card<'a>(p: &Pal, inner: impl Into<E<'a>>) -> E<'a> {
    container(inner).padding(pad(16.0, 20.0, 16.0, 20.0)).width(Length::Fill).style(ui::boxed(p.panel, 12.0, Some((p.line, 1.0)))).into()
}

fn dot<'a>(c: Color) -> E<'a> {
    t("●", 12.0, c).into()
}

fn click<'a>(e: impl Into<E<'a>>, m: Msg) -> E<'a> {
    mouse_area(e).on_press(m).interaction(mouse::Interaction::Pointer).into()
}

// ------------------------------------------------------------------ the top bar
fn top_bar(a: &App) -> E<'_> {
    let p = a.p;
    let mut tabs = Row::new().spacing(10.0).align_y(iced::Center).push(pb(&p, "LB", false, true));
    for tb_ in Tab::ALL {
        let on = a.tab == tb_;
        tabs = tabs.push(chip(&p, tb_.label(), if on { Ck::Acc } else { Ck::Ghost }, 44.0, 15.0, Some(Msg::Tab(tb_))));
    }
    tabs = tabs.push(pb(&p, "RB", false, true));
    let counts = format!("{} games · {} consoles", thousands(a.total()), a.shelves.len());
    let clock = match a.battery {
        Some(b) => format!("{} · {}%", hearth_ui::time::clock(a.now), b),
        None => hearth_ui::time::clock(a.now),
    };
    let mut r = row![
        mark("R", 0xff9be8, 0xb04fc8, 44.0, 15.0),
        gap(18.0, 1.0),
        tb("Hearth Retro", 26.0, p.text).wrapping(text::Wrapping::None),
        gap(40.0, 1.0),
        tabs,
        mouse_area(container(Space::new()).width(Length::Fill).height(Length::Fixed(84.0))).on_press(Msg::Drag).on_double_click(Msg::Max).on_right_press(Msg::Describe(hearth_ui::describe::DMsg::Open)),
        t1(counts, 16.0, p.muted),
        gap(18.0, 1.0),
        chip(&p, clock, Ck::Normal, 44.0, 15.0, None),
    ]
    .align_y(iced::Center);
    if a.describe.open || a.describe.studio_editing() {
        r = r.push(container(hearth_ui::describe::editing::<Msg>(&p)).padding(pad(0.0, 0.0, 0.0, 12.0)));
    }
    if !a.game_mode {
        // in the desktop, a window: Edit layout (always findable) and its buttons
        r = r.push(gap(12.0, 1.0)).push(chip(&p, "Edit layout", Ck::Ghost, 36.0, 13.0, Some(Msg::Describe(hearth_ui::arrange::edit_layout()))));
        for (g, m) in [(G::Minimize, Msg::Min), (G::Maximize, Msg::Max), (G::Close, Msg::Close)] {
            r = r.push(button(container(ui::glyph(g, p.muted, 10.0, 2.2)).center_x(Length::Fixed(40.0)).center_y(Length::Fixed(36.0))).padding(0.0).on_press(m).style(move |_: &Theme, st: button::Status| button::Style {
                background: Some(Background::Color(if matches!(st, button::Status::Hovered) { p.hover() } else { Color::TRANSPARENT })),
                text_color: p.text,
                border: Border::default(),
                shadow: Shadow::default(),
                snap: true,
            }));
        }
    }
    container(r)
        .padding(pad(0.0, 48.0, 0.0, 48.0))
        .width(Length::Fill)
        .height(Length::Fixed(84.0))
        .style(move |_: &Theme| container::Style {
            background: Some(Background::Color(Color { a: 0.7, ..hexc(0x14110f) })),
            border: Border { color: p.line, width: 0.0, radius: 0.0.into() },
            ..container::Style::default()
        })
        .into()
}

// ------------------------------------------------------------------ the shelves
/// One game's tile: its box art (or its own colours), its name on the dark fade; the picked one lifted and lit.
pub fn game_tile<'a>(a: &'a App, g: &'a Game, sel: bool, si: usize, gi: usize) -> E<'a> {
    let p = a.p;
    let (w, h) = if sel { (159.0, 212.0) } else { (150.0, 200.0) };
    let (top, bottom) = library::tile_colours(&g.title);
    let base: E = match library::art(g) {
        Some(path) => image(image::Handle::from_path(path)).width(Length::Fixed(w)).height(Length::Fixed(h)).content_fit(iced::ContentFit::Cover).into(),
        None => container(Space::new()).width(Length::Fixed(w)).height(Length::Fixed(h)).style(move |_: &Theme| container::Style { background: Some(grad(std::f32::consts::PI * 0.9, rgb(top), rgb(bottom))), border: iced::border::rounded(12.0), ..container::Style::default() }).into(),
    };
    let fade = container(Space::new()).width(Length::Fixed(w)).height(Length::Fixed(h)).style(|_: &Theme| container::Style {
        background: Some(iced::Gradient::Linear(iced::gradient::Linear::new(0.0).add_stop(0.0, Color::from_rgba(0.0, 0.0, 0.0, 0.72)).add_stop(0.55, Color::TRANSPARENT).add_stop(1.0, Color::TRANSPARENT)).into()),
        border: iced::border::rounded(12.0),
        ..container::Style::default()
    });
    let mut name_row = Row::new();
    if a.is_fav(g) {
        name_row = name_row.push(t("★ ", 13.0, p.accent));
    }
    name_row = name_row.push(tb(&g.title, if sel { 14.0 } else { 13.0 }, Color::WHITE));
    let name = container(name_row).width(Length::Fixed(w)).height(Length::Fixed(h)).padding(10.0).align_bottom(Length::Fixed(h));
    let face: E = stack![base, fade, name].into();
    let face: E = if sel {
        container(face)
            .padding(4.0)
            .style(move |_: &Theme| container::Style {
                border: Border { color: p.accent, width: 4.0, radius: 16.0.into() },
                shadow: Shadow { color: Pal::a(p.accent, 0.45), offset: iced::Vector::new(0.0, 0.0), blur_radius: 40.0 },
                ..container::Style::default()
            })
            .into()
    } else {
        container(face).padding(4.0).into()
    };
    mouse_area(face).on_press(Msg::Pick(si, gi)).on_double_click(Msg::PlayGame(si, gi)).interaction(mouse::Interaction::Pointer).into()
}

fn shelves(a: &App) -> E<'_> {
    let p = a.p;
    if a.shelves.is_empty() {
        let words = if a.scanned {
            format!(
                "No games yet. Put them in {} (a folder per console: snes, psx, n64…), or in RetroDECK's or EmuDeck's roms folder, then Read the games again.",
                library::tilde(&hearth_retro::retro_dir().join("roms"))
            )
        } else {
            "Reading your games…".into()
        };
        return column![tb("Your shelves", 20.0, p.text), t(words, 15.0, p.muted), chip(&p, "Set up in one go", Ck::Acc, 44.0, 15.0, Some(Msg::Tab(Tab::Setup)))].spacing(14.0).into();
    }
    let mut c = Column::new().spacing(22.0);
    for (si, (sys, games)) in a.shelves.iter().enumerate().skip(a.top) {
        let header = row![
            tb(sys.name, 20.0, p.text).wrapping(text::Wrapping::None),
            gap(12.0, 1.0),
            t1(format!("{} games", thousands(games.len())), 16.0, p.muted),
            Space::new().width(Length::Fill),
            click(t1(format!("{} · see all ›", sys.emu), 12.0, p.muted), Msg::SeeAll(sys.id)),
        ]
        .align_y(iced::Alignment::End);
        let picked_here = a.sel.0 == si;
        let first = if picked_here { a.sel.1.saturating_sub(7) } else { 0 };
        let mut tiles = Row::new().spacing(6.0).align_y(iced::Center).height(Length::Fixed(222.0));
        for (gi, g) in games.iter().enumerate().skip(first).take(8) {
            tiles = tiles.push(game_tile(a, g, picked_here && a.sel.1 == gi, si, gi));
        }
        c = c.push(column![header, tiles].spacing(8.0));
    }
    scrollable(container(c).width(Length::Fixed(1310.0)).padding(pad(0.0, 0.0, 20.0, 0.0))).height(Length::Fill).into()
}

// ------------------------------------------------------------------ the cards on the right
fn ago(now: i64, then: i64) -> String {
    let d = (now - then).max(0);
    if d < 3600 {
        "just now".into()
    } else if d < 86400 {
        let h = d / 3600;
        format!("{h} hour{} ago", if h == 1 { "" } else { "s" })
    } else {
        let n = d / 86400;
        format!("{n} day{} ago", if n == 1 { "" } else { "s" })
    }
}

fn picked_card(a: &App) -> E<'_> {
    let p = a.p;
    let Some((sys, g)) = a.picked() else {
        return card(&p, column![cap(&p, "Picked"), t("Pick a game on a shelf.", 15.0, p.muted)].spacing(8.0));
    };
    let mut line = vec![sys.name.to_string()];
    if !g.year.is_empty() {
        line.push(g.year.clone());
    }
    line.push(format!("emulator {}", sys.emu));
    line.push(match a.last_played(g) {
        Some(ts) => format!("last played {}", ago(a.now, ts)),
        None => "not played yet".into(),
    });
    let states = a.states(g).len();
    let (crt, smooth) = hearth_retro::prefs::look(&a.prefs, &g.path.display().to_string());
    let mut tags = Row::new().spacing(8.0).push(tag(&p, format!("SAVE STATES · {states}"), if states > 0 { Tg::Ok } else { Tg::Dk }));
    if crt {
        tags = tags.push(tag(&p, "CRT LOOK", Tg::Dk));
    }
    if smooth {
        tags = tags.push(tag(&p, "4:3 · SMOOTH", Tg::Dk));
    }
    let mut c = column![cap(&p, "Picked"), gap(1.0, 2.0), tb(&g.title, 26.0, p.text), container(t(line.join(" · "), 16.0, p.muted)).padding(pad(4.0, 0.0, 0.0, 0.0)), gap(1.0, 8.0), tags];
    if let Some((won, of)) = a.ra.get(&g.title.to_lowercase()) {
        c = c.push(container(tag(&p, format!("RETRO ACHIEVEMENTS {won}/{of}"), Tg::Bl)).padding(pad(8.0, 0.0, 0.0, 0.0)));
    }
    let play = button(container(ts("▶ Play", 15.0, p.ink)).center_x(Length::Fill).center_y(Length::Fixed(44.0)))
        .width(Length::Fill)
        .padding(0.0)
        .on_press(Msg::Play)
        .style(move |_: &Theme, st: button::Status| button::Style {
            background: Some(Background::Color(if matches!(st, button::Status::Hovered) { Pal::mix(p.accent, Color::WHITE, 0.1) } else { p.accent })),
            text_color: p.ink,
            border: iced::border::rounded(12.0),
            shadow: Shadow::default(),
            snap: true,
        });
    c = c.push(container(row![play, chip(&p, "Load a save", Ck::Normal, 44.0, 15.0, Some(Msg::Saves)), chip(&p, "⋯", Ck::Normal, 44.0, 15.0, Some(Msg::Options))].spacing(10.0)).padding(pad(18.0, 0.0, 0.0, 0.0)));
    card(&p, c)
}

fn setup_card(a: &App) -> E<'_> {
    let p = a.p;
    let missing = a.here.as_ref().map(|h| h.missing().len()).unwrap_or(0);
    let done = a.here.is_some() && missing == 0 && emus::cfg_ok();
    let state: E = if a.here.is_none() {
        tag(&p, "LOOKING", Tg::Dk)
    } else if done {
        tag(&p, "DONE", Tg::Ok)
    } else if missing > 0 {
        tag(&p, format!("{missing} TO SET UP"), Tg::Warn)
    } else {
        tag(&p, "ONE STEP LEFT", Tg::Warn)
    };
    let desc = "Every emulator installed and set the same way: your controller, hotkeys, saves in ~/Hearth/Retro, synced with Hearth Link.";
    let mut c = column![row![ts("Set up in one go", 18.0, p.text), Space::new().width(Length::Fill), state].align_y(iced::Center), container(t(desc, 12.0, p.muted).line_height(1.6)).padding(pad(6.0, 0.0, 0.0, 0.0))];
    let shown: Vec<&emus::Emu> = emus::EMUS.iter().take(10).collect();
    let mut chips: Vec<E> = shown
        .iter()
        .map(|e| {
            let here = a.here.as_ref().map(|h| h.has(e)).unwrap_or(false);
            chip_dot(&p, e.name, if here { GREEN } else { YELLOW })
        })
        .collect();
    if emus::EMUS.len() > 10 {
        chips.push(chip(&p, format!("+{} more", emus::EMUS.len() - 10), Ck::Normal, 28.0, 12.0, Some(Msg::Tab(Tab::Setup))));
    }
    let mut rows = Column::new().spacing(6.0);
    let mut it = chips.into_iter();
    for n in [4usize, 4, 3] {
        let mut r = Row::new().spacing(6.0);
        for _ in 0..n {
            if let Some(ch) = it.next() {
                r = r.push(ch);
            }
        }
        rows = rows.push(r);
    }
    c = c.push(container(rows).padding(pad(12.0, 0.0, 0.0, 0.0)));
    if !done {
        let label = if a.busy.is_empty() { "Set it all up".to_string() } else { a.busy.clone() };
        c = c.push(container(chip(&p, label, Ck::Acc, 36.0, 13.0, if a.busy.is_empty() { Some(Msg::SetUp) } else { None })).padding(pad(10.0, 0.0, 0.0, 0.0)));
    }
    card(&p, c)
}

pub fn chip_dot<'a>(p: &Pal, name: &'a str, c: Color) -> E<'a> {
    container(row![t("●", 10.0, c), t1(name, 12.0, p.text)].spacing(8.0).align_y(iced::Center))
        .padding(pad(0.0, 10.0, 0.0, 10.0))
        .height(Length::Fixed(28.0))
        .center_y(Length::Fixed(28.0))
        .style(ui::boxed(p.panel2, 8.0, Some((p.line, 1.0))))
        .into()
}

fn bios_row<'a>(a: &'a App, i: usize, c: &'a bios::Check, last: bool, focused: bool) -> E<'a> {
    let p = a.p;
    let (col, words, right): (Color, String, E) = match &c.state {
        State::Good(f) => (GREEN, format!("{} · {}", c.need.console, f), t("checked", 12.0, p.muted).into()),
        State::Missing => (YELLOW, format!("{} · not found", c.need.console), chip(&p, "Where to put it", Ck::Normal, 28.0, 12.0, Some(Msg::Where(i)))),
        State::Bad(..) => (YELLOW, format!("{} · wrong file (bad check)", c.need.console), chip(&p, "Why", Ck::Normal, 28.0, 12.0, Some(Msg::Why(i)))),
    };
    let r = row![dot(col), t1(words, 14.0, p.text), Space::new().width(Length::Fill), right].spacing(10.0).align_y(iced::Center);
    let r = container(r).padding(pad(8.0, 0.0, 8.0, 0.0)).width(Length::Fill);
    let r: E = if focused { container(r).style(ui::boxed(p.sel(), 8.0, None)).into() } else { r.into() };
    if last {
        r
    } else {
        column![r, ui::hline(hexc(0x2c2622))].into()
    }
}

fn bios_card(a: &App) -> E<'_> {
    let p = a.p;
    let good = a.bios.iter().filter(|c| matches!(c.state, State::Good(_))).count();
    let all = a.bios.len();
    let mut c = column![row![ts("BIOS check", 18.0, p.text), Space::new().width(Length::Fill), tag(&p, format!("{good} OF {all}"), if good == all && all > 0 { Tg::Ok } else { Tg::Warn })].align_y(iced::Center)];
    let shown = bios::shown(&a.bios);
    let n = shown.len().min(3);
    let mut rows = Column::new();
    for (k, ch) in shown.into_iter().take(3).enumerate() {
        let i = a.bios.iter().position(|x| std::ptr::eq(x, ch)).unwrap_or(0);
        rows = rows.push(bios_row(a, i, ch, k + 1 == n, false));
    }
    c = c.push(container(rows).padding(pad(6.0, 0.0, 0.0, 0.0)));
    card(&p, c)
}

fn con_box<'a>(p: &Pal) -> E<'a> {
    container(Space::new()).width(Length::Fixed(60.0)).height(Length::Fixed(36.0)).style(ui::boxed(p.panel2, 8.0, Some((p.line, 1.0)))).into()
}

fn pads_rows(a: &App, all: bool) -> E<'_> {
    let p = a.p;
    let mut c = Column::new().spacing(10.0);
    if a.pads.is_empty() {
        c = c.push(t("No controller yet: plug one in or pair it in Settings › Bluetooth.", 13.0, p.muted));
    }
    for (i, pd) in a.pads.iter().enumerate().take(if all { 8 } else { 2 }) {
        let sub = if i == 0 {
            "Hotkey: ≡ + button · ≡+R1 save · ≡+L1 load".to_string()
        } else {
            match pd.battery {
                Some(b) => format!("Ready · battery {b}%"),
                None => "Ready".into(),
            }
        };
        c = c.push(row![con_box(&p), column![t1(format!("Player {} · {}", i + 1, pd.name), 14.0, p.text), t1(sub, 12.0, p.muted)]].spacing(12.0).align_y(iced::Center));
    }
    c.into()
}

fn pads_card(a: &App) -> E<'_> {
    let p = a.p;
    card(&p, column![row![ts("Controllers", 18.0, p.text), Space::new().width(Length::Fill), t1("same buttons in every emulator", 12.0, p.muted)].align_y(iced::Center), gap(1.0, 12.0), pads_rows(a, false)])
}

// ------------------------------------------------------------------ the footer
fn foot(a: &App) -> E<'_> {
    let p = a.p;
    let left = if !a.said.is_empty() {
        a.said.clone()
    } else if a.game_mode {
        "Hearth Retro in Game Mode · also in the desktop as a window".into()
    } else {
        "Hearth Retro in the desktop · also in Game Mode, full screen".into()
    };
    let hint = |b: &'static str, acc: bool, sq: bool, label: &'static str, m: Msg| -> E<'static> { click(row![pb(&a.p, b, acc, sq), gap(8.0, 1.0), t1(label, 15.0, a.p.text)].align_y(iced::Center), m) };
    let pads = row![hint("A", true, false, "Play", Msg::Play), hint("X", false, false, "Saves", Msg::Saves), hint("Y", false, false, "Favourite", Msg::Fav), hint("≡", false, true, "Options", Msg::Options), hint("B", false, false, "Back", Msg::Back)].spacing(22.0).align_y(iced::Center);
    let bar = container(row![t1(left, 15.0, p.muted), Space::new().width(Length::Fill), pads].align_y(iced::Center))
        .padding(pad(0.0, 40.0, 0.0, 40.0))
        .width(Length::Fill)
        .height(Length::Fixed(64.0))
        .center_y(Length::Fixed(64.0))
        .style(move |_: &Theme| container::Style {
            background: Some(grad(0.0, Color::from_rgba(0.04, 0.027, 0.02, 0.95), Color::from_rgba(0.04, 0.027, 0.02, 0.6))),
            border: Border { color: p.line, width: 1.0, radius: 0.0.into() },
            ..container::Style::default()
        });
    let credit = row![Space::new().width(Length::Fill), t1("Hearth Retro · ", 11.0, hexc(0x8f847b)), text("Built on RetroDECK and EmuDeck (GPL-3)").size(11.0).color(p.muted).font(ui::weight(iced::font::Weight::Semibold))];
    column![container(credit).padding(pad(0.0, 24.0, 6.0, 0.0)), bar].into()
}

// ------------------------------------------------------------------ the other tabs
fn setup_page(a: &App) -> E<'_> {
    let p = a.p;
    let mut list = Column::new().spacing(8.0);
    for e in emus::EMUS {
        let here = a.here.as_ref().map(|h| h.has(e)).unwrap_or(false);
        let plays: Vec<&str> = library::SYSTEMS.iter().filter(|s| s.emu == e.name).map(|s| s.name).collect();
        list = list.push(row![dot(if here { GREEN } else { YELLOW }), t1(e.name, 15.0, p.text).width(Length::Fixed(160.0)), t1(if here { "set up" } else { "not yet" }, 13.0, p.muted).width(Length::Fixed(90.0)), t1(plays.join(" · "), 13.0, p.muted)].spacing(12.0).align_y(iced::Center));
    }
    let emus_block = card(&p, column![row![ts("Every emulator", 18.0, p.text), Space::new().width(Length::Fill), setup_card_state(a)].align_y(iced::Center), gap(1.0, 10.0), scrollable(list).height(Length::Fill)]);
    let r = hearth_retro::retro_dir();
    let roots: Vec<String> = library::roots().iter().filter(|x| x.is_dir()).map(|x| library::tilde(x)).collect();
    let folders = card(
        &p,
        column![
            ts("Where things are", 18.0, p.text),
            t(format!("Games: {}", if roots.is_empty() { library::tilde(&r.join("roms")) } else { roots.join(" · ") }), 13.0, p.text),
            t(format!("Saves: {} · states: {}", library::tilde(&r.join("saves")), library::tilde(&r.join("states"))), 13.0, p.text),
            t(format!("BIOS: {}", library::tilde(&r.join("bios"))), 13.0, p.text),
            t(format!("RetroArch's hotkeys and folders: {}{}", library::tilde(&emus::retroarch_cfg()), if emus::cfg_ok() { " (written)" } else { " (Set it all up writes it)" }), 13.0, p.text),
            t("Installing uses Flathub, for you only: RetroDECK carries every emulator. Nothing is installed as root.", 12.0, p.muted),
            row![
                chip(&p, if a.busy.is_empty() { "Set it all up".to_string() } else { a.busy.clone() }, Ck::Acc, 36.0, 13.0, if a.busy.is_empty() { Some(Msg::SetUp) } else { None }),
                chip(&p, "Open the folder", Ck::Normal, 36.0, 13.0, Some(Msg::OpenDir(r.display().to_string()))),
                chip(&p, "Read the games again", Ck::Normal, 36.0, 13.0, Some(Msg::ReadAgain))
            ]
            .spacing(8.0)
        ]
        .spacing(8.0),
    );
    hearth_ui::arrange::parts(&a.describe, &p, "setup", vec![("emus", emus_block), ("folders", folders)], |ps| row![ps.take("emus"), container(ps.take("folders")).width(Length::Fixed(560.0))].spacing(14.0).into(), Msg::Describe)
}

fn setup_card_state(a: &App) -> E<'_> {
    let missing = a.here.as_ref().map(|h| h.missing().len()).unwrap_or(0);
    if a.here.is_some() && missing == 0 {
        tag(&a.p, "DONE", Tg::Ok)
    } else {
        tag(&a.p, format!("{missing} TO SET UP"), Tg::Warn)
    }
}

fn bios_page(a: &App) -> E<'_> {
    let p = a.p;
    let mut rows = Column::new();
    let n = a.bios.len();
    for (i, c) in a.bios.iter().enumerate() {
        rows = rows.push(bios_row(a, i, c, i + 1 == n, i == a.bios_sel));
    }
    let good = a.bios.iter().filter(|c| matches!(c.state, State::Good(_))).count();
    let list = card(&p, column![row![ts("BIOS check", 18.0, p.text), Space::new().width(Length::Fill), tag(&p, format!("{good} OF {n}"), if good == n { Tg::Ok } else { Tg::Warn })].align_y(iced::Center), scrollable(rows).height(Length::Fill)].spacing(8.0));
    let dirs: Vec<String> = bios::dirs().iter().map(|d| library::tilde(d)).collect();
    let where_ = card(&p, column![ts("Where BIOS files go", 18.0, p.text), t(format!("Put them in {} (Hearth Retro looks in {} too). Each file is checked against the good copies' sums.", dirs[0], dirs[1..].join(", ")), 13.0, p.muted), chip(&p, "Open the BIOS folder", Ck::Normal, 36.0, 13.0, Some(Msg::OpenDir(bios::dirs()[0].display().to_string())))].spacing(10.0));
    hearth_ui::arrange::parts(&a.describe, &p, "bios", vec![("list", list), ("where", where_)], |ps| row![ps.take("list"), container(ps.take("where")).width(Length::Fixed(520.0))].spacing(14.0).into(), Msg::Describe)
}

fn pads_page(a: &App) -> E<'_> {
    let p = a.p;
    let list = card(&p, column![row![ts("Controllers", 18.0, p.text), Space::new().width(Length::Fill), t1("same buttons in every emulator", 12.0, p.muted)].align_y(iced::Center), gap(1.0, 12.0), pads_rows(a, true)]);
    let keys = card(
        &p,
        column![
            ts("Hotkeys in every emulator", 18.0, p.text),
            t("≡ + R1 · save a state", 14.0, p.text),
            t("≡ + L1 · load the last state", 14.0, p.text),
            t("≡ + B · the emulator's menu", 14.0, p.text),
            t("≡ + Start · back to Hearth Retro", 14.0, p.text),
            t("Set the same way for RetroArch and its cores by Set up in one go.", 12.0, p.muted)
        ]
        .spacing(8.0),
    );
    hearth_ui::arrange::parts(&a.describe, &p, "pads", vec![("list", list), ("hotkeys", keys)], |ps| row![ps.take("list"), container(ps.take("hotkeys")).width(Length::Fixed(520.0))].spacing(14.0).into(), Msg::Describe)
}

fn search_page(a: &App) -> E<'_> {
    let p = a.p;
    let label = match a.search_sys.and_then(library::system) {
        Some(s) => format!("Find a game in {}", s.name),
        None => "Find a game".into(),
    };
    let field = container(
        text_input(&label, &a.search)
            .on_input(Msg::Search)
            .size(18.0)
            .padding(pad(12.0, 16.0, 12.0, 16.0))
            .style(move |_: &Theme, _st| text_input::Style { background: Background::Color(p.bg), border: Border { color: p.line, width: 1.0, radius: 12.0.into() }, icon: p.muted, placeholder: p.muted, value: p.text, selection: Pal::a(p.accent, 0.35) }),
    )
    .width(Length::Fixed(720.0));
    let res = a.results();
    let mut grid = Column::new().spacing(14.0);
    for (ri, chunk) in res.chunks(8).enumerate() {
        let mut r = Row::new().spacing(6.0);
        for (k, &(si, gi)) in chunk.iter().enumerate() {
            let g = &a.shelves[si].1[gi];
            r = r.push(game_tile(a, g, ri * 8 + k == a.search_sel, si, gi));
        }
        grid = grid.push(r);
    }
    let results: E = column![t1(format!("{} games", thousands(res.len())), 14.0, p.muted), scrollable(grid).height(Length::Fill)].spacing(10.0).into();
    hearth_ui::arrange::parts(&a.describe, &p, "search", vec![("field", field.into()), ("results", results)], |ps| column![ps.take("field"), ps.take("results")].spacing(18.0).into(), Msg::Describe)
}

// ------------------------------------------------------------------ dialogs
fn menu_box<'a>(a: &'a App, title: String, items: Vec<(String, Msg)>) -> E<'a> {
    let p = a.p;
    let mut c = Column::new().spacing(4.0).push(container(ts(title, 17.0, p.text)).padding(pad(0.0, 0.0, 8.0, 4.0)));
    for (i, (label, _)) in items.iter().enumerate() {
        let sel = i == a.menu_sel;
        c = c.push(
            button(container(t1(label.clone(), 15.0, p.text)).padding(pad(0.0, 14.0, 0.0, 14.0)).center_y(Length::Fixed(42.0)))
                .width(Length::Fill)
                .padding(0.0)
                .on_press(Msg::MenuPick(i))
                .style(move |_: &Theme, st: button::Status| button::Style {
                    background: Some(Background::Color(if sel { p.sel() } else if matches!(st, button::Status::Hovered) { p.hover() } else { Color::TRANSPARENT })),
                    text_color: p.text,
                    border: Border { color: if sel { p.accent } else { Color::TRANSPARENT }, width: if sel { 2.0 } else { 0.0 }, radius: 8.0.into() },
                    shadow: Shadow::default(),
                    snap: true,
                }),
        );
    }
    c = c.push(container(row![pb(&p, "A", true, false), t1(" Pick", 13.0, p.muted), gap(16.0, 1.0), pb(&p, "B", false, false), t1(" Close", 13.0, p.muted)].align_y(iced::Center)).padding(pad(10.0, 0.0, 0.0, 4.0)));
    ui::dialog_box(&p, 460.0, c)
}

fn dialog(a: &App) -> Option<E<'_>> {
    let p = a.p;
    let d = a.dlg.as_ref()?;
    Some(match d {
        Dlg::Options => {
            let items = a.menu_items();
            let title = a.picked().map(|(_, g)| g.title.clone()).unwrap_or_else(|| "Hearth Retro".into());
            menu_box(a, title, items)
        }
        Dlg::Saves(s) => {
            let title = a.picked().map(|(_, g)| format!("Load a save · {}", g.title)).unwrap_or_default();
            if s.is_empty() {
                ui::dialog_box(&p, 460.0, column![ts(title, 17.0, p.text), t("No save states yet: ≡ + R1 saves one while you play.", 14.0, p.muted), chip(&p, "OK", Ck::Acc, 36.0, 13.0, Some(Msg::CloseDlg))].spacing(12.0))
            } else {
                let items = s
                    .iter()
                    .map(|(f, slot, when)| {
                        let which = if *slot < 0 { "Auto save".to_string() } else { format!("Slot {slot}") };
                        (format!("{which} · {} · {}", ago(a.now, *when), ui::fit(f, 28)), Msg::PlaySlot(*slot))
                    })
                    .collect();
                menu_box(a, title, items)
            }
        }
        Dlg::Where(i) => {
            let c = a.bios.get(*i)?;
            let dir = library::tilde(&bios::dirs()[0]);
            let sub = if c.need.sub.is_empty() { dir.clone() } else { format!("{dir} (or {dir}/{})", c.need.sub) };
            ui::dialog_box(
                &p,
                520.0,
                column![
                    ts(format!("{} · where to put it", c.need.console), 17.0, p.text),
                    t(format!("Put {} in {sub}. Use your own console's BIOS: Hearth can't give you one.", c.need.files.join(" or ")), 14.0, p.text),
                    t("Hearth Retro checks it again the moment you come back.", 13.0, p.muted),
                    row![chip(&p, "Open the BIOS folder", Ck::Acc, 36.0, 13.0, Some(Msg::OpenDir(bios::dirs()[0].display().to_string()))), chip(&p, "OK", Ck::Normal, 36.0, 13.0, Some(Msg::CloseDlg))].spacing(8.0)
                ]
                .spacing(12.0),
            )
        }
        Dlg::Why(i) => {
            let c = a.bios.get(*i)?;
            let (file, sum) = match &c.state {
                State::Bad(f, s) => (f.clone(), s.clone()),
                _ => (String::new(), String::new()),
            };
            ui::dialog_box(
                &p,
                560.0,
                column![
                    ts(format!("{} · why it's the wrong file", c.need.console), 17.0, p.text),
                    t(format!("{file} is there, but its check (MD5 {sum}) isn't a good {} BIOS's. It's another region's or a damaged copy, so games may not start.", c.need.console), 14.0, p.text),
                    t(format!("A good one's check is {}.", c.need.sums.join(" or ")), 12.0, p.muted),
                    t(format!("Found at {}", c.at.as_ref().map(|x| library::tilde(x)).unwrap_or_default()), 12.0, p.muted),
                    row![chip(&p, "Open the BIOS folder", Ck::Acc, 36.0, 13.0, Some(Msg::OpenDir(bios::dirs()[0].display().to_string()))), chip(&p, "OK", Ck::Normal, 36.0, 13.0, Some(Msg::CloseDlg))].spacing(8.0)
                ]
                .spacing(12.0),
            )
        }
        Dlg::Ra => {
            let has_key = a.prefs.get("raKey").and_then(|v| v.as_str()).map(|s| !s.is_empty()).unwrap_or(false);
            ui::dialog_box(
                &p,
                520.0,
                column![
                    ts("RetroAchievements", 17.0, p.text),
                    t("Your progress on the Picked card. Your own name and web API key (retroachievements.org › Settings); kept only on this PC, readable only by you.", 13.0, p.muted),
                    ra_field(p, "Your RetroAchievements name", &a.ra_name, Msg::RaName, false),
                    ra_field(p, if has_key { "Your web API key (kept; type to change it)" } else { "Your web API key" }, &a.ra_key, Msg::RaKey, true),
                    row![chip(&p, "Save", Ck::Acc, 36.0, 13.0, Some(Msg::RaSave)), chip(&p, "Cancel", Ck::Normal, 36.0, 13.0, Some(Msg::CloseDlg))].spacing(8.0)
                ]
                .spacing(12.0),
            )
        }
    })
}

fn ra_field<'a>(p: Pal, ph: &'a str, v: &'a str, f: fn(String) -> Msg, secure: bool) -> E<'a> {
    text_input(ph, v)
        .on_input(f)
        .secure(secure)
        .size(15.0)
        .padding(pad(9.0, 12.0, 9.0, 12.0))
        .style(move |_: &Theme, _st| text_input::Style { background: Background::Color(p.bg), border: Border { color: p.line, width: 1.0, radius: 8.0.into() }, icon: p.muted, placeholder: p.muted, value: p.text, selection: Pal::a(p.accent, 0.35) })
        .into()
}

// ------------------------------------------------------------------ blocks
pub const BLOCKS: &[PageDef] = &[
    PageDef { id: "window", title: "Hearth Retro", blocks: &[bl("bar", "The top bar", 0, 0, 12, 1), bl("page", "The page", 0, 1, 12, 10), bl("foot", "The button hints", 0, 11, 12, 1)] },
    PageDef {
        id: "shelves",
        title: "Shelves",
        blocks: &[bls("shelves", "The shelves", 0, 0, 9, 12), bl("picked", "Picked", 9, 0, 3, 3), bl("setup", "Set up in one go", 9, 3, 3, 3), bl("bios", "BIOS check", 9, 6, 3, 3), bl("pads", "Controllers", 9, 9, 3, 3)],
    },
    PageDef { id: "setup", title: "Set up", blocks: &[bls("emus", "Every emulator", 0, 0, 8, 12), bl("folders", "Where things are", 8, 0, 4, 12)] },
    PageDef { id: "bios", title: "BIOS check", blocks: &[bls("list", "Every BIOS", 0, 0, 8, 12), bl("where", "Where BIOS files go", 8, 0, 4, 12)] },
    PageDef { id: "pads", title: "Controllers", blocks: &[bl("list", "The controllers", 0, 0, 8, 12), bl("hotkeys", "The hotkeys", 8, 0, 4, 12)] },
    PageDef { id: "search", title: "Search", blocks: &[bl("field", "Find a game", 0, 0, 12, 1), bls("results", "The games found", 0, 1, 12, 11)] },
];

pub const KIT: Kit = Kit {
    name: "Hearth Retro",
    buttons: &[
        tile("shelves", "Shelves", G::Grid),
        tile("setup", "Set up", G::Cog),
        tile("bios", "BIOS check", G::Check),
        tile("pads", "Controllers", G::Pad),
        tile("search", "Search", G::Search),
        tile("play", "Play the picked game", G::Play),
        tile("load", "Load a save", G::Clock),
        tile("favourite", "Favourite", G::Asterisk),
        tile("options", "Options", G::Dots),
        tile("set-up", "Set it all up", G::Down),
        tile("read-again", "Read the games again", G::Sync),
    ],
    panes: &[tile("picked", "Picked", G::Play), tile("setup", "Set up in one go", G::Cog), tile("bios", "BIOS check", G::Check), tile("pads", "Controllers", G::Pad)],
    info: &[tile("games", "Games", G::Grid), tile("consoles", "Consoles", G::List), tile("picked", "The picked game", G::Play), tile("bios", "BIOS files checked", G::Check)],
    pane_page: "window",
    bars: &[],
    menus: &[],
};

pub fn block_action(id: &str) -> Option<Msg> {
    Some(match id {
        "shelves" => Msg::Tab(Tab::Shelves),
        "setup" => Msg::Tab(Tab::Setup),
        "bios" => Msg::Tab(Tab::Bios),
        "pads" => Msg::Tab(Tab::Pads),
        "search" => Msg::Tab(Tab::Search),
        "play" => Msg::Play,
        "load" => Msg::Saves,
        "favourite" => Msg::Fav,
        "options" => Msg::Options,
        "set-up" => Msg::SetUp,
        "read-again" => Msg::ReadAgain,
        _ => return None,
    })
}

fn studio_values(a: &App) {
    let good = a.bios.iter().filter(|c| matches!(c.state, State::Good(_))).count();
    a.describe.values(vec![
        ("games", thousands(a.total())),
        ("consoles", a.shelves.len().to_string()),
        ("picked", a.picked().map(|(_, g)| g.title.clone()).unwrap_or_default()),
        ("bios", format!("{good} of {}", a.bios.len())),
    ]);
}

pub fn view(a: &App) -> E<'_> {
    let p = a.p;
    studio_values(a);
    let page: E = match a.tab {
        Tab::Shelves => hearth_ui::arrange::parts(
            &a.describe,
            &p,
            "shelves",
            vec![("shelves", shelves(a)), ("picked", picked_card(a)), ("setup", setup_card(a)), ("bios", bios_card(a)), ("pads", pads_card(a))],
            |ps| {
                let right = column![ps.take("picked"), ps.take("setup"), ps.take("bios"), ps.take("pads")].spacing(14.0).width(Length::Fixed(470.0));
                row![container(ps.take("shelves")).width(Length::Fill).padding(pad(4.0, 0.0, 0.0, 0.0)), right].spacing(10.0).into()
            },
            Msg::Describe,
        ),
        Tab::Setup => setup_page(a),
        Tab::Bios => bios_page(a),
        Tab::Pads => pads_page(a),
        Tab::Search => search_page(a),
    };
    let page: E = container(page).padding(pad(24.0, 48.0, 12.0, 48.0)).width(Length::Fill).height(Length::Fill).into();
    let mut items: Vec<(&'static str, E)> = vec![("bar", hearth_ui::studio::made(&a.describe, "window", "bar", || top_bar(a))), ("page", page), ("foot", foot(a))];
    for (id, which) in [("pane:picked", "picked"), ("pane:setup", "setup"), ("pane:bios", "bios"), ("pane:pads", "pads")] {
        if a.describe.wants_pane(which) {
            let e = match which {
                "picked" => picked_card(a),
                "setup" => setup_card(a),
                "bios" => bios_card(a),
                _ => pads_card(a),
            };
            items.push((id, e));
        }
    }
    let body = hearth_ui::arrange::parts(&a.describe, &p, "window", items, |ps| column![ps.take("bar"), ps.take("page"), ps.take("foot")].into(), Msg::Describe);
    let bg = p.bg;
    let base: E = container(body)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_: &Theme| container::Style { background: Some(iced::Gradient::Linear(iced::gradient::Linear::new(std::f32::consts::PI).add_stop(0.0, bg).add_stop(0.62, bg).add_stop(1.0, Pal::mix(bg, hexc(0x7a3b15), 0.55))).into()), ..container::Style::default() })
        .into();
    let base = match dialog(a) {
        Some(d) => ui::dialog(base, d, Some(Msg::CloseDlg)),
        None => base,
    };
    if a.game_mode {
        base
    } else {
        ui::edges(base, Msg::Resize)
    }
}
