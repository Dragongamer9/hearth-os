//! What Retro to Steam looks like (board 05): its title bar, the consoles from Hearth Retro (each on or off), where the
//! games are and the Steam user; Found N games with the art kinds, Find a game and the filters; every game's card with
//! its art, its choice (‹ 1 of 6 ›) and new / needs a pick; the picked game as it will look in Steam (hero with its
//! logo, grid, wide art, icon and logo), its name, collection, Game Mode's Retro shelf and your own image; and the bar
//! that adds them (Steam closed first). Every part is a block (hearth_ui::blocks), with Retro to Steam's own buttons,
//! panes and info to drag in (KIT).

use crate::art::Kind;
use crate::{rom, App, Dlg, Filter, Msg};
use hearth_retro::library::{self, thousands, Game};
use hearth_ui::blocks::{bl, bls, PageDef};
use hearth_ui::studio::{tile, Kit};
use hearth_ui::ui::{self, gap, t, t1, tb, ts, G};
use hearth_ui::Pal;
use iced::widget::canvas::{self, Frame, Geometry, LineDash, Path, Stroke};
use iced::widget::{button, column, container, image, mouse_area, row, scrollable, stack, text, text_input, Column, Row, Space};
use iced::{mouse, Background, Border, Color, Element, Length, Padding, Point, Rectangle, Renderer, Shadow, Size, Theme};

pub type E<'a> = Element<'a, Msg>;

const GREEN: Color = Color::from_rgb(0x9b as f32 / 255.0, 0xe3 as f32 / 255.0, 0x6b as f32 / 255.0);
const YELLOW: Color = Color::from_rgb(1.0, 0xd3 as f32 / 255.0, 0x6b as f32 / 255.0);

fn hexc(h: u32) -> Color {
    Color::from_rgb8((h >> 16) as u8, (h >> 8) as u8, h as u8)
}
fn rgb(c: [u8; 3]) -> Color {
    Color::from_rgb8(c[0], c[1], c[2])
}
fn pad(t: f32, r: f32, b: f32, l: f32) -> Padding {
    Padding { top: t, right: r, bottom: b, left: l }
}
fn grad(angle: f32, a: Color, b: Color) -> Background {
    iced::Gradient::Linear(iced::gradient::Linear::new(angle).add_stop(0.0, a).add_stop(1.0, b)).into()
}

fn mark<'a>(letters: String, from: u32, to: u32, size: f32, font: f32, r: f32) -> E<'a> {
    container(tb(letters, font, Color::WHITE))
        .center_x(Length::Fixed(size))
        .center_y(Length::Fixed(size))
        .style(move |_: &Theme| container::Style { background: Some(grad(2.36, hexc(from), hexc(to))), border: iced::border::rounded(r), ..container::Style::default() })
        .into()
}

#[derive(Clone, Copy, PartialEq)]
enum Ck {
    Acc,
    Normal,
}

fn chip<'a>(p: &Pal, label: impl text::IntoFragment<'a>, kind: Ck, h: f32, size: f32, on: Option<Msg>) -> E<'a> {
    let p = *p;
    let fg = if kind == Ck::Acc { p.ink } else { p.text };
    let lbl = if kind == Ck::Acc { ts(label, size, fg) } else { t(label, size, fg) };
    let padx = if h >= 44.0 { 20.0 } else if h <= 28.0 { 10.0 } else { 14.0 };
    let r = if h >= 44.0 { 12.0 } else if h <= 28.0 { 8.0 } else { 10.0 };
    button(container(lbl.wrapping(text::Wrapping::None)).center_y(Length::Fixed(h)))
        .padding(pad(0.0, padx, 0.0, padx))
        .on_press_maybe(on)
        .style(move |_: &Theme, st: button::Status| {
            let hover = matches!(st, button::Status::Hovered | button::Status::Pressed);
            let bg = match kind {
                Ck::Acc => if hover { Pal::mix(p.accent, Color::WHITE, 0.1) } else { p.accent },
                Ck::Normal => if hover { Pal::mix(p.panel2, p.text, 0.06) } else { p.panel2 },
            };
            button::Style { background: Some(Background::Color(bg)), text_color: fg, border: Border { color: if kind == Ck::Acc { p.accent } else { p.line }, width: 1.0, radius: r.into() }, shadow: Shadow::default(), snap: true }
        })
        .into()
}

/// A tab as the boards draw them (.tab, .tab.on: small, the accent when it's on).
fn tab<'a>(p: &Pal, label: &'a str, on: bool, m: Msg) -> E<'a> {
    let p = *p;
    button(container(if on { ts(label, 12.0, p.ink) } else { t(label, 12.0, p.muted) }).padding(pad(6.0, 10.0, 6.0, 10.0)))
        .padding(0.0)
        .on_press(m)
        .style(move |_: &Theme, st: button::Status| button::Style {
            background: Some(Background::Color(if on { p.accent } else if matches!(st, button::Status::Hovered) { p.hover() } else { Color::TRANSPARENT })),
            text_color: p.text,
            border: Border { color: if on { p.accent } else { p.line }, width: 1.0, radius: 8.0.into() },
            shadow: Shadow::default(),
            snap: true,
        })
        .into()
}

fn cap<'a>(p: &Pal, s: &'a str) -> E<'a> {
    container(tb(s.to_uppercase(), 11.0, p.muted)).padding(pad(14.0, 0.0, 8.0, 0.0)).into()
}

/// The switch (.sw, .sw.off).
fn switch<'a>(p: &Pal, on: bool, m: Msg) -> E<'a> {
    let p = *p;
    let knob = container(Space::new()).width(Length::Fixed(14.0)).height(Length::Fixed(14.0)).style(ui::boxed(Color::WHITE, 7.0, None));
    let inner: E = if on { row![Space::new().width(Length::Fill), knob].into() } else { row![knob, Space::new().width(Length::Fill)].into() };
    mouse_area(container(inner).padding(3.0).width(Length::Fixed(34.0)).height(Length::Fixed(20.0)).style(ui::boxed(if on { p.accent } else { p.line }, 10.0, None))).on_press(m).interaction(mouse::Interaction::Pointer).into()
}

/// A field (.field): the page's background, a line, rounded.
fn field<'a>(p: &Pal, s: impl text::IntoFragment<'a>, m: Option<Msg>) -> E<'a> {
    let f = container(t1(s, 12.0, p.text)).padding(pad(0.0, 10.0, 0.0, 10.0)).height(Length::Fixed(34.0)).center_y(Length::Fixed(34.0)).width(Length::Fill).style(ui::boxed(p.bg, 8.0, Some((p.line, 1.0))));
    match m {
        Some(m) => mouse_area(f).on_press(m).interaction(mouse::Interaction::Pointer).into(),
        None => f.into(),
    }
}

fn input_style(p: Pal) -> impl Fn(&Theme, text_input::Status) -> text_input::Style {
    move |_, st| text_input::Style {
        background: Background::Color(p.bg),
        border: Border { color: if matches!(st, text_input::Status::Focused { .. }) { p.accent } else { p.line }, width: 1.0, radius: 8.0.into() },
        icon: p.muted,
        placeholder: p.muted,
        value: p.text,
        selection: Pal::a(p.accent, 0.35),
    }
}

// ------------------------------------------------------------------ the art
struct Dashes(Color, f32);
impl canvas::Program<Msg> for Dashes {
    type State = ();
    fn draw(&self, _s: &(), r: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        let mut f = Frame::new(r, b.size());
        let path = Path::rounded_rectangle(Point::new(1.0, 1.0), Size::new((b.width - 2.0).max(0.0), (b.height - 2.0).max(0.0)), self.1.into());
        f.stroke(&path, Stroke { line_dash: LineDash { segments: &[6.0, 5.0], offset: 0 }, ..Stroke::default().with_color(self.0).with_width(2.0) });
        vec![f.into_geometry()]
    }
}

/// The board's mark on art not downloaded yet: a diamond.
struct Diamond;
impl canvas::Program<Msg> for Diamond {
    type State = ();
    fn draw(&self, _s: &(), r: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        let mut f = Frame::new(r, b.size());
        let (cx, cy, s) = (b.width / 2.0, 95.0f32.min(b.height * 0.33), 49.0);
        let path = Path::new(|p| {
            p.move_to(Point::new(cx, cy - s));
            p.line_to(Point::new(cx + s, cy));
            p.line_to(Point::new(cx, cy + s));
            p.line_to(Point::new(cx - s, cy));
            p.close();
        });
        f.stroke(&path, Stroke::default().with_color(Color::from_rgba(1.0, 1.0, 1.0, 0.27)).with_width(3.0));
        vec![f.into_geometry()]
    }
}

/// A picture of the game's art (its file when downloaded, else its own colours), with its name over the dark fade.
fn art_box<'a>(a: &'a App, g: &'a Game, k: Kind, w: Length, h: f32, label: Option<String>, label_size: f32, diamond: bool) -> E<'a> {
    let (top, bottom) = library::tile_colours(&g.title);
    let base: E = match a.shown(g, k) {
        Some(f) => image(image::Handle::from_path(f)).width(w).height(Length::Fixed(h)).content_fit(iced::ContentFit::Cover).into(),
        None => {
            let bg: E = container(Space::new()).width(w).height(Length::Fixed(h)).style(move |_: &Theme| container::Style { background: Some(grad(std::f32::consts::PI * 0.92, rgb(top), rgb(bottom))), border: iced::border::rounded(12.0), ..container::Style::default() }).into();
            if diamond {
                stack![bg, canvas::Canvas::new(Diamond).width(w).height(Length::Fixed(h))].into()
            } else {
                bg
            }
        }
    };
    let fade = container(Space::new()).width(w).height(Length::Fixed(h)).style(|_: &Theme| container::Style {
        background: Some(iced::Gradient::Linear(iced::gradient::Linear::new(0.0).add_stop(0.0, Color::from_rgba(0.0, 0.0, 0.0, 0.66)).add_stop(0.55, Color::TRANSPARENT).add_stop(1.0, Color::TRANSPARENT)).into()),
        border: iced::border::rounded(12.0),
        ..container::Style::default()
    });
    let mut layers: Vec<E> = vec![base, fade.into()];
    if let Some(l) = label {
        layers.push(container(tb(l, label_size, Color::WHITE)).width(w).height(Length::Fixed(h)).padding(pad(0.0, 14.0, 12.0, 14.0)).align_bottom(Length::Fixed(h)).into());
    }
    iced::widget::Stack::with_children(layers).width(w).height(Length::Fixed(h)).into()
}

fn none_box<'a>(p: &Pal, h: f32) -> E<'a> {
    let words = column![t("No art found", 12.0, p.muted), t("Pick one or use your own", 12.0, p.muted)].align_x(iced::Center);
    stack![
        canvas::Canvas::new(Dashes(p.line, 12.0)).width(Length::Fill).height(Length::Fixed(h)),
        container(words).width(Length::Fill).height(Length::Fixed(h)).center_x(Length::Fill).center_y(Length::Fixed(h))
    ]
    .into()
}

fn step_btn<'a>(p: &Pal, s: &'a str, m: Msg) -> E<'a> {
    let p = *p;
    button(container(t(s, 11.0, p.text)).center_x(Length::Fixed(20.0)).center_y(Length::Fixed(20.0)))
        .padding(0.0)
        .on_press(m)
        .style(move |_: &Theme, st: button::Status| button::Style {
            background: Some(Background::Color(if matches!(st, button::Status::Hovered) { p.hover() } else { p.panel2 })),
            text_color: p.text,
            border: Border { color: p.line, width: 1.0, radius: 6.0.into() },
            shadow: Shadow::default(),
            snap: true,
        })
        .into()
}

fn game_card<'a>(a: &'a App, g: &'a Game) -> E<'a> {
    let p = a.p;
    let r = rom(g);
    let sel = a.picked == r;
    let n = a.cands(g, a.kind).len();
    let own = a.own(g, a.kind).is_some();
    let pic: E = if n == 0 && !own && a.index.is_some() { none_box(&p, 290.0) } else { art_box(a, g, a.kind, Length::Fill, 290.0, Some(a.name_of(g)), 14.0, true) };
    let pic: E = if sel {
        container(pic).padding(3.0).style(move |_: &Theme| container::Style { border: Border { color: p.accent, width: 3.0, radius: 15.0.into() }, ..container::Style::default() }).into()
    } else {
        container(pic).padding(3.0).into()
    };
    let status: E = if n == 0 && !own && a.index.is_some() {
        row![t1("needs a pick · ", 11.0, YELLOW), step_btn(&p, "…", Msg::Choose)].align_y(iced::Center).into()
    } else {
        let i = a.pick_of(g, a.kind).min(n.saturating_sub(1)) + 1;
        let which = if own { "your own".to_string() } else if a.index.is_none() { "finding…".to_string() } else { format!("{i} of {n}") };
        row![
            step_btn(&p, "‹", Msg::Cycle(r.clone(), a.kind, -1)),
            t1(which, 11.0, p.muted),
            step_btn(&p, "›", Msg::Cycle(r.clone(), a.kind, 1)),
            Space::new().width(Length::Fill),
            if a.in_steam(g) { t1("in Steam", 11.0, p.muted) } else { t1("● new", 11.0, GREEN) }
        ]
        .spacing(6.0)
        .align_y(iced::Center)
        .into()
    };
    let c = column![pic, container(t1(a.name_of(g), 13.0, p.text)).padding(pad(5.0, 3.0, 0.0, 3.0)), container(status).padding(pad(3.0, 3.0, 0.0, 3.0))];
    mouse_area(c).on_press(Msg::Pick(r)).interaction(mouse::Interaction::Pointer).into()
}

// ------------------------------------------------------------------ the parts
fn title_bar(a: &App) -> E<'_> {
    let p = a.p;
    let wb = |g: G, m: Msg, red: bool| -> E<'static> {
        button(container(ui::glyph(g, p.muted, 10.0, 2.0)).center_x(Length::Fixed(26.0)).center_y(Length::Fixed(44.0)))
            .padding(0.0)
            .on_press(m)
            .style(move |_: &Theme, st: button::Status| button::Style {
                background: Some(Background::Color(match st {
                    button::Status::Hovered if red => Color::from_rgb8(0xc4, 0x2b, 0x1c),
                    button::Status::Hovered => p.hover(),
                    _ => Color::TRANSPARENT,
                })),
                text_color: p.text,
                border: Border::default(),
                shadow: Shadow::default(),
                snap: true,
            })
            .into()
    };
    let mut left = row![mark("RS".into(), 0x7ff0d8, 0x1f9f8f, 22.0, 9.0, 7.0), gap(10.0, 1.0), ts("Retro to Steam", 14.0, p.text), gap(8.0, 1.0), t1("· your emulator games in the Steam library, with art", 14.0, p.muted)].align_y(iced::Center);
    if a.describe.open || a.describe.studio_editing() {
        left = left.push(container(hearth_ui::describe::editing::<Msg>(&p)).padding(pad(0.0, 0.0, 0.0, 10.0)));
    }
    let r = row![
        mouse_area(container(left).padding(pad(0.0, 0.0, 0.0, 16.0)).center_y(Length::Fixed(44.0))).on_press(Msg::Drag).on_double_click(Msg::Max).on_right_press(Msg::Describe(hearth_ui::describe::DMsg::Open)),
        mouse_area(container(Space::new()).width(Length::Fill).height(Length::Fixed(44.0))).on_press(Msg::Drag).on_double_click(Msg::Max).on_right_press(Msg::Describe(hearth_ui::describe::DMsg::Open)),
        mouse_area(t1("Edit layout", 12.0, p.muted)).on_press(Msg::Describe(hearth_ui::arrange::edit_layout())).interaction(mouse::Interaction::Pointer),
        gap(16.0, 1.0),
        wb(G::Minimize, Msg::Min, false),
        wb(G::Maximize, Msg::Max, false),
        wb(G::Close, Msg::Quit, true),
        gap(10.0, 1.0),
    ]
    .align_y(iced::Center);
    column![container(r).width(Length::Fill).height(Length::Fixed(44.0)), ui::hline(p.line)].into()
}

fn consoles(a: &App) -> E<'_> {
    let p = a.p;
    let mut c = Column::new().push(container(tb("CONSOLES · FROM HEARTH RETRO", 11.0, p.muted)).padding(pad(2.0, 0.0, 8.0, 0.0)));
    if a.shelves.is_empty() {
        c = c.push(t(if a.scanned { "No games in Hearth Retro's folders yet." } else { "Reading Hearth Retro's games…" }, 12.0, p.muted));
    }
    for (i, (sys, games)) in a.shelves.iter().enumerate() {
        let on = a.console == i;
        let body = row![
            column![t1(sys.name, 13.0, p.text), t1(format!("{} · {}", thousands(games.len()), sys.emu), 11.0, p.muted)].spacing(2.0).width(Length::Fill),
            switch(&p, !a.off(sys.id), Msg::ToggleConsole(sys.id))
        ]
        .align_y(iced::Center)
        .spacing(10.0);
        let row_: E = container(body)
            .padding(pad(9.0, 10.0, 9.0, 10.0))
            .width(Length::Fill)
            .style(move |_: &Theme| container::Style {
                background: Some(Background::Color(if on { hexc(0x3b2a1d) } else { Color::TRANSPARENT })),
                border: Border { color: Color::TRANSPARENT, width: 0.0, radius: 9.0.into() },
                ..container::Style::default()
            })
            .into();
        let row_: E = if on { row![container(Space::new()).width(Length::Fixed(3.0)).height(Length::Fixed(48.0)).style(ui::boxed(p.accent, 2.0, None)), row_].align_y(iced::Center).into() } else { row_ };
        c = c.push(mouse_area(row_).on_press(Msg::Console(i)).interaction(mouse::Interaction::Pointer));
    }
    c.into()
}

fn where_part(a: &App) -> E<'_> {
    let p = a.p;
    column![
        cap(&p, "Where the games are"),
        text_input("~/Hearth/Retro/roms", &a.roms_edit).on_input(Msg::RomsFolder).on_submit(Msg::RomsFolderDone).size(12.0).padding(pad(9.0, 10.0, 9.0, 10.0)).style(input_style(p)),
        container(t("Each game starts in the emulator Hearth Retro set up, with your controller and saves.", 12.0, p.muted).line_height(1.6)).padding(pad(8.0, 0.0, 0.0, 0.0))
    ]
    .into()
}

fn user_part(a: &App) -> E<'_> {
    let p = a.p;
    let label = if a.users.is_empty() {
        "No Steam on this PC yet".to_string()
    } else if a.users.len() == 1 {
        "The one on this PC ▾".to_string()
    } else {
        let n = a.users.iter().find(|u| u.id == a.user).map(|u| if u.name.is_empty() { u.id.clone() } else { u.name.clone() }).unwrap_or_default();
        format!("{n} ▾")
    };
    column![cap(&p, "Steam user"), field(&p, label, if a.users.is_empty() { None } else { Some(Msg::Open(Dlg::Users)) })].into()
}

fn head(a: &App) -> E<'_> {
    let p = a.p;
    let (all, needs, new, in_steam) = a.counts();
    let found = all - needs;
    let sub = if a.index.is_none() && a.scanned { format!("Finding art for {} games…", thousands(all)) } else { format!("Art found for {} · {} need a pick · {} already in Steam (kept as they are)", thousands(found), thousands(needs), thousands(in_steam)) };
    let _ = new;
    let mut kinds = Row::new().spacing(6.0);
    for k in Kind::ALL {
        kinds = kinds.push(tab(&p, k.label(), a.kind == k, Msg::Kind(k)));
    }
    let search = container(text_input("⌕ Find a game", &a.search).on_input(Msg::Search).size(13.0).padding(pad(9.0, 12.0, 9.0, 12.0)).style(move |_: &Theme, st| {
        let mut s = input_style(p)(&Theme::Dark, st);
        s.border.radius = 10.0.into();
        s
    }))
    .width(Length::Fixed(240.0));
    row![column![tb(format!("Found {} games", thousands(all)), 26.0, p.text), container(t1(sub, 12.0, p.muted)).padding(pad(4.0, 0.0, 0.0, 0.0))], Space::new().width(Length::Fill), kinds, gap(12.0, 1.0), search].align_y(iced::Center).into()
}

fn filters(a: &App) -> E<'_> {
    let p = a.p;
    let (all, needs, new, in_steam) = a.counts();
    let f = |label: String, which: Filter| -> E<'static> { chip(&a.p, label, if a.filter == which { Ck::Acc } else { Ck::Normal }, 28.0, 12.0, Some(Msg::Filter(which))) };
    row![
        f(format!("All {}", thousands(all)), Filter::All),
        f(format!("Needs a pick {}", thousands(needs)), Filter::Needs),
        f(format!("New {}", thousands(new)), Filter::New),
        f(format!("Already in Steam {}", thousands(in_steam)), Filter::InSteam),
        Space::new().width(Length::Fill),
        t1("Art from community art sites · only images for that game", 12.0, p.muted),
        gap(8.0, 1.0),
        mouse_area(t1("Art sources…", 12.0, p.accent)).on_press(Msg::Open(Dlg::Sources)).interaction(mouse::Interaction::Pointer)
    ]
    .spacing(8.0)
    .align_y(iced::Center)
    .into()
}

fn grid(a: &App) -> E<'_> {
    let p = a.p;
    let games = a.shown_games();
    if games.is_empty() {
        return container(t(if a.scanned { "No games here with this filter." } else { "Reading Hearth Retro's games…" }, 14.0, p.muted)).padding(20.0).into();
    }
    let mut c = Column::new().spacing(22.0);
    for chunk in games.chunks(6) {
        let mut r = Row::new().spacing(16.0);
        for g in chunk {
            r = r.push(container(game_card(a, g)).width(Length::FillPortion(1)));
        }
        for _ in chunk.len()..6 {
            r = r.push(Space::new().width(Length::FillPortion(1)));
        }
        c = c.push(r);
    }
    scrollable(c).height(Length::Fill).into()
}

fn preview(a: &App) -> E<'_> {
    let p = a.p;
    let Some(g) = a.picked_game() else { return column![container(tb("PICKED · HOW IT WILL LOOK IN STEAM", 11.0, p.muted)), t("Pick a game.", 13.0, p.muted)].spacing(8.0).into() };
    let sys = library::system(g.system);
    let file = g.path.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
    let name = a.name_of(g);
    let hn = a.cands(g, Kind::Hero).len();
    let hi = a.pick_of(g, Kind::Hero).min(hn.saturating_sub(1)) + 1;
    // the hero, with the logo over it (the logo's picture when there's one, else the name as Steam's title)
    let logo: E = match a.shown(g, Kind::Logo) {
        Some(f) => container(image(image::Handle::from_path(f)).height(Length::Fixed(60.0))).padding(pad(0.0, 20.0, 20.0, 20.0)).height(Length::Fixed(180.0)).align_bottom(Length::Fixed(180.0)).into(),
        None => container(text(name.to_uppercase()).size(34.0).color(hexc(0xffd8c8)).font(iced::Font { weight: iced::font::Weight::Black, style: iced::font::Style::Italic, ..app_font() }))
            .padding(pad(0.0, 20.0, 20.0, 20.0))
            .height(Length::Fixed(180.0))
            .align_bottom(Length::Fixed(180.0))
            .into(),
    };
    let hero_tag = container(tb(if hn > 0 { format!("HERO · {hi} OF {hn}") } else { "HERO · NONE YET".into() }, 11.0, p.muted)).padding(pad(3.0, 8.0, 3.0, 8.0)).style(ui::boxed(p.panel2, 6.0, Some((p.line, 1.0))));
    let hero = mouse_area(stack![art_box(a, g, Kind::Hero, Length::Fill, 180.0, None, 12.0, false), logo, container(hero_tag).padding(pad(12.0, 0.0, 0.0, 14.0))]).on_press(Msg::Cycle(rom(g), Kind::Hero, 1)).interaction(mouse::Interaction::Pointer);
    let icon_found = a.shown(g, Kind::Icon).is_some() || !a.cands(g, Kind::Icon).is_empty();
    let logo_found = a.shown(g, Kind::Logo).is_some() || !a.cands(g, Kind::Logo).is_empty();
    let found = match (icon_found, logo_found) {
        (true, true) => "both found",
        (true, false) => "icon found · no logo yet",
        (false, true) => "logo found · no icon yet",
        _ => "none found yet",
    };
    let initials: String = name.split_whitespace().filter_map(|w| w.chars().next()).filter(|c| c.is_alphanumeric()).take(2).collect::<String>().to_uppercase();
    let icon: E = match a.shown(g, Kind::Icon) {
        Some(f) => image(image::Handle::from_path(f)).width(Length::Fixed(44.0)).height(Length::Fixed(44.0)).into(),
        None => mark(initials, 0xff9a8a, 0xd8413a, 44.0, 15.0, 12.0),
    };
    let small = row![
        mouse_area(container(art_box(a, g, Kind::Grid, Length::Fixed(120.0), 170.0, Some(name.clone()), 12.0, false)).width(Length::Fixed(120.0))).on_press(Msg::Cycle(rom(g), Kind::Grid, 1)).interaction(mouse::Interaction::Pointer),
        column![
            mouse_area(art_box(a, g, Kind::Wide, Length::Fill, 84.0, Some("Wide art".into()), 12.0, false)).on_press(Msg::Cycle(rom(g), Kind::Wide, 1)).interaction(mouse::Interaction::Pointer),
            row![icon, column![t1("Icon · Logo", 12.0, p.muted), t1(found, 12.0, p.text)]].spacing(10.0).align_y(iced::Center)
        ]
        .spacing(12.0)
    ]
    .spacing(12.0);
    column![
        container(tb("PICKED · HOW IT WILL LOOK IN STEAM", 11.0, p.muted)).padding(pad(0.0, 0.0, 8.0, 0.0)),
        tb(name.clone(), 22.0, p.text),
        container(t1(format!("{} · {} · {}", sys.map(|s| s.name).unwrap_or(""), sys.map(|s| s.emu).unwrap_or(""), file), 12.0, p.muted)).padding(pad(2.0, 0.0, 14.0, 0.0)),
        hero,
        container(small).padding(pad(12.0, 0.0, 0.0, 0.0))
    ]
    .into()
}

fn app_font() -> iced::Font {
    hearth_ui::app::INTER
}

fn details(a: &App) -> E<'_> {
    let p = a.p;
    let Some(g) = a.picked_game() else { return Space::new().into() };
    column![
        cap(&p, "Name in Steam"),
        text_input("Its name in Steam", &a.name_edit).on_input(Msg::Name).size(12.0).padding(pad(9.0, 10.0, 9.0, 10.0)).style(input_style(p)),
        cap(&p, "Collection"),
        field(&p, format!("{} ▾", a.collection_of(g)), Some(Msg::Open(Dlg::Collection))),
        container(row![t("Show in Game Mode's Retro shelf", 13.0, p.text), Space::new().width(Length::Fill), switch(&p, a.shelf_of(g), Msg::Shelf(!a.shelf_of(g)))].align_y(iced::Center)).padding(pad(14.0, 0.0, 7.0, 0.0)),
        container(row![t(format!("Use my own image{}", if a.own(g, a.kind).is_some() { format!(" ({} chosen)", a.kind.label().to_lowercase()) } else { String::new() }), 13.0, p.text), Space::new().width(Length::Fill), chip(&p, "Choose…", Ck::Normal, 28.0, 12.0, Some(Msg::Choose))].align_y(iced::Center)).padding(pad(7.0, 0.0, 0.0, 0.0)),
    ]
    .into()
}

fn bottom(a: &App) -> E<'_> {
    let p = a.p;
    let (_, _, new, _) = a.counts();
    let words = if let Some(_) = &a.busy {
        a.busy_text.clone()
    } else if !a.said.is_empty() {
        a.said.clone()
    } else {
        "Steam has to be closed while games are added. Hearth asks first and opens it again after.".to_string()
    };
    let busy = a.busy.is_some();
    let has_added = a.prefs.get("added").and_then(|v| v.get(&a.user)).and_then(|v| v.as_array()).map(|v| !v.is_empty()).unwrap_or(false);
    let r = row![
        container(Space::new()).width(Length::Fixed(16.0)).height(Length::Fixed(16.0)).style(ui::boxed(YELLOW, 8.0, None)),
        t1(words, 12.0, p.text),
        Space::new().width(Length::Fill),
        chip(&p, "Take out what I added", Ck::Normal, 36.0, 14.0, if busy || !has_added { None } else { Some(Msg::Remove) }),
        chip(&p, "Preview only", Ck::Normal, 36.0, 14.0, if busy { None } else { Some(Msg::Preview) }),
        chip(&p, format!("Add {} games to Steam", thousands(new)), Ck::Acc, 44.0, 15.0, if busy || new == 0 { None } else { Some(Msg::Add) }),
    ]
    .spacing(12.0)
    .align_y(iced::Center);
    container(r)
        .padding(pad(0.0, 24.0, 0.0, 24.0))
        .width(Length::Fill)
        .height(Length::Fixed(74.0))
        .center_y(Length::Fixed(74.0))
        .style(move |_: &Theme| container::Style { background: Some(Background::Color(Color { a: 0.98, ..hexc(0x1a1614) })), border: Border { color: p.line, width: 1.0, radius: 0.0.into() }, ..container::Style::default() })
        .into()
}


/// The credit, as the board has it: the app, and what it's built on with its licence.
fn credit<'a>(p: &Pal, app: &'a str, built: &'a str) -> E<'a> {
    container(row![Space::new().width(Length::Fill), t1(format!("{app} · "), 11.0, hexc(0x8f847b)), text(built).size(11.0).color(p.muted).font(ui::weight(iced::font::Weight::Semibold))])
        .padding(pad(4.0, 24.0, 6.0, 0.0))
        .width(Length::Fill)
        .into()
}

// ------------------------------------------------------------------ dialogs
fn dialog(a: &App) -> Option<E<'_>> {
    let p = a.p;
    let d = a.dlg.as_ref()?;
    let boxed = |inner: E<'static>| -> E<'static> { ui::dialog_box(&a.p, 560.0, inner) };
    Some(match d {
        Dlg::CloseSteam(remove) => boxed(
            column![
                ts("Close Steam first?", 17.0, p.text),
                t("Steam has to be closed while Retro to Steam writes its list of non-Steam games. Hearth closes it, adds them (with their art) and opens it again.", 14.0, p.text),
                t("Nothing is signed in to and no account is used: only Steam's own files on this PC.", 12.0, p.muted),
                row![chip(&p, if *remove { "Close Steam and take them out" } else { "Close Steam and add" }, Ck::Acc, 36.0, 13.0, Some(Msg::Go(*remove))), chip(&p, "Cancel", Ck::Normal, 36.0, 13.0, Some(Msg::Close))].spacing(8.0)
            ]
            .spacing(12.0)
            .into(),
        ),
        Dlg::Preview(s) => boxed(column![ts("Preview only", 17.0, p.text), t(s.clone(), 13.0, p.text), chip(&p, "OK", Ck::Acc, 36.0, 13.0, Some(Msg::Close))].spacing(12.0).into()),
        Dlg::Said(h, s) => boxed(column![ts(h.clone(), 17.0, p.text), t(s.clone(), 14.0, p.text), chip(&p, "OK", Ck::Acc, 36.0, 13.0, Some(Msg::Close))].spacing(12.0).into()),
        Dlg::Sources => {
            let has = a.prefs.get("sgdbKey").and_then(|v| v.as_str()).map(|s| !s.is_empty()).unwrap_or(false);
            ui::dialog_box(
                &p,
                600.0,
                column![
                    ts("Where the art comes from", 17.0, p.text),
                    t("libretro's thumbnails (thumbnails.libretro.com): box art, title screens and screenshots, matched to each game by its name. No account or key.", 13.0, p.text),
                    t("SteamGridDB (steamgriddb.com): grids, wide art, heroes, logos and icons, with your own API key (steamgriddb.com › Preferences › API). It's kept only on this PC, readable only by you.", 13.0, p.text),
                    text_input(if has { "Your SteamGridDB key (kept; type to change it, empty to remove it)" } else { "Your SteamGridDB key" }, &a.sgdb_key).on_input(Msg::SgdbKey).secure(true).size(13.0).padding(pad(9.0, 10.0, 9.0, 10.0)).style(input_style(p)),
                    row![chip(&p, "Save", Ck::Acc, 36.0, 13.0, Some(Msg::SgdbSave)), chip(&p, "Close", Ck::Normal, 36.0, 13.0, Some(Msg::Close))].spacing(8.0)
                ]
                .spacing(12.0),
            )
        }
        Dlg::Users => {
            let mut c = Column::new().spacing(6.0).push(ts("Steam user", 17.0, p.text));
            for u in &a.users {
                let label = if u.name.is_empty() { format!("Steam user {}", u.id) } else { u.name.clone() };
                c = c.push(chip(&p, label, if u.id == a.user { Ck::Acc } else { Ck::Normal }, 36.0, 13.0, Some(Msg::User(u.id.clone()))));
            }
            ui::dialog_box(&p, 420.0, c)
        }
        Dlg::Collection => {
            let g = a.picked_game()?;
            let sys = library::system(g.system).map(|s| s.name).unwrap_or("");
            let mut c = Column::new().spacing(6.0).push(ts("Collection", 17.0, p.text));
            for o in [format!("Retro · {sys}"), "Retro".to_string(), sys.to_string(), "None".to_string()] {
                let on = a.collection_of(g) == o;
                c = c.push(chip(&p, o.clone(), if on { Ck::Acc } else { Ck::Normal }, 36.0, 13.0, Some(Msg::Collection(o))));
            }
            ui::dialog_box(&p, 420.0, c)
        }
    })
}

// ------------------------------------------------------------------ blocks
pub const BLOCKS: &[PageDef] = &[
    PageDef { id: "window", title: "Retro to Steam", blocks: &[bl("title", "The title bar", 0, 0, 12, 1), bl("side", "Consoles and Steam", 0, 1, 2, 10), bl("games", "The games", 2, 1, 7, 9), bl("picked", "Picked", 9, 1, 3, 9), bl("bottom", "Add to Steam", 2, 10, 10, 1), bl("credit", "Built on Steam ROM Manager", 0, 11, 12, 1)] },
    PageDef { id: "side", title: "Consoles and Steam", blocks: &[bls("consoles", "Consoles from Hearth Retro", 0, 0, 12, 7), bl("where", "Where the games are", 0, 7, 12, 3), bl("user", "Steam user", 0, 10, 12, 2)] },
    PageDef { id: "games", title: "The games", blocks: &[bl("head", "Found and the art", 0, 0, 12, 1), bl("filters", "Filters", 0, 1, 12, 1), bls("grid", "The games", 0, 2, 12, 10)] },
    PageDef { id: "picked", title: "Picked", blocks: &[bl("preview", "How it will look in Steam", 0, 0, 12, 7), bl("details", "Name, collection and image", 0, 7, 12, 5)] },
];

pub const KIT: Kit = Kit {
    name: "Retro to Steam",
    buttons: &[
        tile("add", "Add to Steam", G::Plus),
        tile("preview", "Preview only", G::Search),
        tile("remove", "Take out what I added", G::Close),
        tile("grid", "Grid art", G::Grid),
        tile("wide", "Wide art", G::Columns),
        tile("hero", "Hero", G::Display),
        tile("logo", "Logo", G::Pencil),
        tile("icon", "Icon", G::Asterisk),
        tile("needs", "Needs a pick", G::Help),
        tile("new", "New", G::Bell),
        tile("sources", "Art sources", G::Link),
        tile("choose", "Use my own image", G::Folder),
    ],
    panes: &[tile("preview", "How it will look in Steam", G::Display), tile("consoles", "Consoles", G::List)],
    info: &[tile("found", "Games found", G::Grid), tile("new", "New for Steam", G::Bell), tile("needs", "Need a pick", G::Help), tile("in-steam", "Already in Steam", G::Check)],
    pane_page: "window",
    bars: &[],
    menus: &[],
};

pub fn block_action(id: &str) -> Option<Msg> {
    Some(match id {
        "add" => Msg::Add,
        "preview" => Msg::Preview,
        "remove" => Msg::Remove,
        "grid" => Msg::Kind(Kind::Grid),
        "wide" => Msg::Kind(Kind::Wide),
        "hero" => Msg::Kind(Kind::Hero),
        "logo" => Msg::Kind(Kind::Logo),
        "icon" => Msg::Kind(Kind::Icon),
        "needs" => Msg::Filter(Filter::Needs),
        "new" => Msg::Filter(Filter::New),
        "sources" => Msg::Open(Dlg::Sources),
        "choose" => Msg::Choose,
        _ => return None,
    })
}

pub fn view(a: &App) -> E<'_> {
    let p = a.p;
    let (all, needs, new, in_steam) = a.counts();
    a.describe.values(vec![("found", thousands(all)), ("new", thousands(new)), ("needs", thousands(needs)), ("in-steam", thousands(in_steam))]);
    let side = hearth_ui::arrange::parts(&a.describe, &p, "side", vec![("consoles", consoles(a)), ("where", where_part(a)), ("user", user_part(a))], |ps| scrollable(column![ps.take("consoles"), ps.take("where"), ps.take("user")]).height(Length::Fill).into(), Msg::Describe);
    let side: E = container(side).padding(pad(14.0, 12.0, 14.0, 12.0)).width(Length::Fixed(270.0)).height(Length::Fill).style(move |_: &Theme| container::Style { background: Some(Background::Color(Color { a: 0.5, ..hexc(0x14110f) })), ..container::Style::default() }).into();
    let games = hearth_ui::arrange::parts(&a.describe, &p, "games", vec![("head", head(a)), ("filters", filters(a)), ("grid", grid(a))], |ps| column![ps.take("head"), container(ps.take("filters")).padding(pad(14.0, 0.0, 16.0, 0.0)), ps.take("grid")].into(), Msg::Describe);
    let games: E = container(games).padding(pad(18.0, 24.0, 14.0, 24.0)).width(Length::Fill).height(Length::Fill).into();
    let picked = hearth_ui::arrange::parts(&a.describe, &p, "picked", vec![("preview", preview(a)), ("details", details(a))], |ps| column![ps.take("preview"), ps.take("details")].into(), Msg::Describe);
    let picked: E = container(scrollable(picked)).padding(pad(18.0, 20.0, 14.0, 20.0)).width(Length::Fixed(430.0)).height(Length::Fill).style(move |_: &Theme| container::Style { background: Some(Background::Color(Color { a: 0.5, ..hexc(0x14110f) })), ..container::Style::default() }).into();
    let mut items: Vec<(&'static str, E)> = vec![("title", hearth_ui::studio::made(&a.describe, "window", "title", || title_bar(a))), ("side", side), ("games", games), ("picked", picked), ("bottom", bottom(a)), ("credit", credit(&p, "Retro to Steam", "Built on Steam ROM Manager (GPL-3)"))];
    if a.describe.wants_pane("preview") {
        items.push(("pane:preview", preview(a)));
    }
    if a.describe.wants_pane("consoles") {
        items.push(("pane:consoles", consoles(a)));
    }
    let body = hearth_ui::arrange::parts(
        &a.describe,
        &p,
        "window",
        items,
        |ps| {
            let right = column![row![ps.take("games"), ui::vline(p.line), ps.take("picked")].height(Length::Fill), ps.take("bottom")];
            column![ps.take("title"), row![ps.take("side"), ui::vline(p.line), right].height(Length::Fill), ps.take("credit")].into()
        },
        Msg::Describe,
    );
    let base: E = container(body).width(Length::Fill).height(Length::Fill).style(ui::boxed(Color { a: 0.97, ..hexc(0x1a1614) }, p.window_corner, Some((p.line, 1.0)))).into();
    let base = match dialog(a) {
        Some(d) => ui::dialog(base, d, Some(Msg::Close)),
        None => base,
    };
    ui::edges(base, Msg::Resize)
}
