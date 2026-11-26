//! What Hearth Studio looks like (board 12): the title bar with LIVE and REC and OBS's numbers; what viewers see;
//! Going live (where, the title, the quality, the 9:16 copy and the six buttons); the stream's health; the scenes;
//! what's in the scene; the sound mixer; switching scenes and the hotkeys. Every part is a block (hearth_ui::blocks),
//! moved, sized, hidden or restyled in Edit layout, with Hearth Studio's own buttons, panes and info (KIT).

use crate::{obs, App, Audio, Dlg, Item, Msg};
use hearth_ui::blocks::{bl, PageDef};
use hearth_ui::studio::{tile, Kit};
use hearth_ui::ui::{self, gap, t, t1, tb, ts, G};
use hearth_ui::Pal;
use iced::widget::canvas::{self, Frame, Geometry, Path};
use iced::widget::{button, column, container, image, mouse_area, row, stack, text, text_input, Column, Row, Space};
use iced::{mouse, Background, Border, Color, Element, Length, Padding, Point, Rectangle, Renderer, Shadow, Size, Theme};

pub type E<'a> = Element<'a, Msg>;

const GREEN: Color = Color::from_rgb(0x9b as f32 / 255.0, 0xe3 as f32 / 255.0, 0x6b as f32 / 255.0);
const RED: Color = Color::from_rgb(1.0, 0x6b as f32 / 255.0, 0x6b as f32 / 255.0);

fn hexc(h: u32) -> Color {
    Color::from_rgb8((h >> 16) as u8, (h >> 8) as u8, h as u8)
}
fn pad(t: f32, r: f32, b: f32, l: f32) -> Padding {
    Padding { top: t, right: r, bottom: b, left: l }
}

fn mark<'a>() -> E<'a> {
    container(tb("ST", 9.0, Color::WHITE))
        .center_x(Length::Fixed(22.0))
        .center_y(Length::Fixed(22.0))
        .style(|_: &Theme| container::Style { background: Some(iced::Gradient::Linear(iced::gradient::Linear::new(2.36).add_stop(0.0, hexc(0xff9a8a)).add_stop(1.0, hexc(0xd8413a))).into()), border: iced::border::rounded(7.0), ..container::Style::default() })
        .into()
}

fn tag<'a>(label: impl text::IntoFragment<'a>, bg: Color, fg: Color, line: Color) -> E<'a> {
    container(tb(label, 11.0, fg).wrapping(text::Wrapping::None)).padding(pad(3.0, 8.0, 3.0, 8.0)).style(ui::boxed(bg, 6.0, Some((line, 1.0)))).into()
}

#[derive(Clone, Copy, PartialEq)]
enum Ck {
    Acc,
    Normal,
}

fn chip<'a>(p: &Pal, label: impl text::IntoFragment<'a>, kind: Ck, h: f32, on: Option<Msg>) -> E<'a> {
    let p = *p;
    let fg = if kind == Ck::Acc { p.ink } else { p.text };
    let lbl = if kind == Ck::Acc { ts(label, 12.0, fg) } else { t(label, 12.0, fg) };
    button(container(lbl.wrapping(text::Wrapping::None)).center_y(Length::Fixed(h)))
        .padding(pad(0.0, 10.0, 0.0, 10.0))
        .on_press_maybe(on)
        .style(move |_: &Theme, st: button::Status| {
            let hover = matches!(st, button::Status::Hovered);
            button::Style {
                background: Some(Background::Color(match kind {
                    Ck::Acc => if hover { Pal::mix(p.accent, Color::WHITE, 0.1) } else { p.accent },
                    Ck::Normal => if hover { Pal::mix(p.panel2, p.text, 0.06) } else { p.panel2 },
                })),
                text_color: fg,
                border: Border { color: if kind == Ck::Acc { p.accent } else { p.line }, width: 1.0, radius: 8.0.into() },
                shadow: Shadow::default(),
                snap: true,
            }
        })
        .into()
}

fn switch<'a>(p: &Pal, on: bool, m: Msg) -> E<'a> {
    let knob = container(Space::new()).width(Length::Fixed(14.0)).height(Length::Fixed(14.0)).style(ui::boxed(Color::WHITE, 7.0, None));
    let inner: E = if on { row![Space::new().width(Length::Fill), knob].into() } else { row![knob, Space::new().width(Length::Fill)].into() };
    mouse_area(container(inner).padding(3.0).width(Length::Fixed(34.0)).height(Length::Fixed(20.0)).style(ui::boxed(if on { p.accent } else { p.line }, 10.0, None))).on_press(m).interaction(mouse::Interaction::Pointer).into()
}

/// A segmented choice (.seg): each part, the one in use in the accent.
fn seg<'a>(p: &Pal, parts: Vec<(String, bool, Option<Msg>)>) -> E<'a> {
    let p = *p;
    let mut r = Row::new();
    for (label, on, m) in parts {
        let dim = m.is_none() && !on;
        r = r.push(
            button(container(if on { ts(label, 12.0, p.ink) } else { t(label, 12.0, if dim { Pal::a(p.muted, 0.5) } else { p.muted }) }).padding(pad(5.0, 9.0, 5.0, 9.0)))
                .padding(0.0)
                .on_press_maybe(m)
                .style(move |_: &Theme, st: button::Status| button::Style {
                    background: Some(Background::Color(if on { p.accent } else if matches!(st, button::Status::Hovered) { p.hover() } else { Color::TRANSPARENT })),
                    text_color: p.text,
                    border: Border::default(),
                    shadow: Shadow::default(),
                    snap: true,
                }),
        );
    }
    container(r).style(ui::boxed(Color::TRANSPARENT, 8.0, Some((p.line, 1.0)))).clip(true).into()
}

/// A pane (.pane): the panel, a line, rounded 12, its heading.
fn pane<'a>(p: &Pal, head: E<'a>, body: E<'a>) -> E<'a> {
    pane_h(p, head, body, Length::Fill)
}

fn pane_h<'a>(p: &Pal, head: E<'a>, body: E<'a>, h: Length) -> E<'a> {
    container(column![container(head).padding(pad(0.0, 0.0, 10.0, 0.0)), body]).padding(14.0).width(Length::Fill).height(h).style(ui::boxed(p.panel, 12.0, Some((p.line, 1.0)))).into()
}

fn head<'a>(p: &Pal, title: &'a str, right: Option<E<'a>>) -> E<'a> {
    let mut r = row![ts(title, 13.0, p.text), Space::new().width(Length::Fill)].align_y(iced::Center).spacing(8.0);
    if let Some(x) = right {
        r = r.push(x);
    }
    r.into()
}

fn mu<'a>(p: &Pal, s: impl text::IntoFragment<'a>) -> E<'a> {
    t1(s, 12.0, p.muted).into()
}

fn row_of<'a>(p: &Pal, label: &'a str, right: E<'a>) -> E<'a> {
    container(row![t(label, 13.0, p.text), Space::new().width(Length::Fill), right].align_y(iced::Center)).padding(pad(7.0, 0.0, 7.0, 0.0)).into()
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

// ------------------------------------------------------------------ the title bar
fn title_bar(a: &App) -> E<'_> {
    let p = a.p;
    let mut left = row![mark(), gap(10.0, 1.0), ts("Hearth Studio", 14.0, p.text)].align_y(iced::Center).spacing(0.0);
    if a.stream.active {
        left = left.push(container(tag(format!("● LIVE {}", a.stream.timecode), hexc(0xff4a4a), Color::WHITE, hexc(0xff4a4a))).padding(pad(0.0, 0.0, 0.0, 12.0)));
    }
    if a.record.active {
        let label = if a.record.paused { format!("❚❚ PAUSED {}", a.record.timecode) } else { format!("● REC {}", a.record.timecode) };
        left = left.push(container(tag(label, hexc(0x3d1717), RED, hexc(0x6a2a2a))).padding(pad(0.0, 0.0, 0.0, 6.0)));
    }
    if a.describe.open || a.describe.studio_editing() {
        left = left.push(container(hearth_ui::describe::editing::<Msg>(&p)).padding(pad(0.0, 0.0, 0.0, 10.0)));
    }
    let mut stats: Vec<String> = Vec::new();
    if a.up {
        stats.push(format!("{} fps", a.fps.round() as i64));
        stats.push(format!("CPU {}%", a.cpu.round() as i64));
        if !a.encoder.is_empty() {
            stats.push(a.encoder.clone());
        }
        if a.stream.active {
            stats.push(format!("{:.1} Mbps", a.mbps));
        }
        stats.push(format!("{} dropped", a.stream.skipped));
    } else {
        stats.push("OBS isn't connected".into());
    }
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
    let drag = |e: E<'static>| -> E<'static> { mouse_area(e).on_press(Msg::Drag).on_double_click(Msg::Max).on_right_press(Msg::Describe(hearth_ui::describe::DMsg::Open)).into() };
    let r = row![
        drag(container(left).padding(pad(0.0, 0.0, 0.0, 16.0)).center_y(Length::Fixed(44.0)).into()),
        drag(container(Space::new()).width(Length::Fill).height(Length::Fixed(44.0)).into()),
        t1(stats.join(" · "), 12.0, p.muted),
        gap(14.0, 1.0),
        mouse_area(t1("Edit layout", 12.0, p.muted)).on_press(Msg::Describe(hearth_ui::arrange::edit_layout())).interaction(mouse::Interaction::Pointer),
        gap(14.0, 1.0),
        wb(G::Minimize, Msg::Min, false),
        wb(G::Maximize, Msg::Max, false),
        wb(G::Close, Msg::Quit, true),
        gap(10.0, 1.0)
    ]
    .align_y(iced::Center);
    column![container(r).width(Length::Fill).height(Length::Fixed(44.0)), ui::hline(p.line)].into()
}

// ------------------------------------------------------------------ what viewers see
fn setup_card(a: &App) -> E<'_> {
    let p = a.p;
    let (title, words, button_): (&str, String, E) = if !obs::installed() {
        ("OBS Studio isn't installed", "Hearth Studio is built on OBS Studio. Install it from Flathub, for you only.".into(), chip(&p, if a.busy.is_empty() { "Install OBS Studio".to_string() } else { a.busy.clone() }, Ck::Acc, 36.0, if a.busy.is_empty() { Some(Msg::Install) } else { None }))
    } else if a.down_why == "not running" {
        ("OBS Studio isn't running", "Start it in the background: Hearth Studio turns its remote control on first and adds Hearth's Ember transition.".into(), chip(&p, "Start OBS", Ck::Acc, 36.0, Some(Msg::StartObs)))
    } else if a.down_why.contains("password") {
        ("OBS asks for its remote control's password", "OBS › Tools › WebSocket Server Settings shows it. Hearth Studio keeps it only on this PC.".into(), chip(&p, "Type the password", Ck::Acc, 36.0, Some(Msg::Open(Dlg::Password))))
    } else if a.down_why.contains("remote control off") || a.down_why.contains("can't reach") {
        ("OBS's remote control is off", format!("Close OBS, then Start OBS here: Hearth turns it on ({}).", a.down_why), chip(&p, "Start OBS", Ck::Acc, 36.0, Some(Msg::StartObs)))
    } else {
        ("Connecting to OBS…", a.down_why.clone(), Space::new().into())
    };
    container(column![ts(title, 18.0, Color::WHITE), t(words, 13.0, hexc(0xd8cfc8)), button_].spacing(12.0).width(Length::Fixed(520.0)))
        .padding(24.0)
        .style(ui::boxed(Color::from_rgba(0.0, 0.0, 0.0, 0.6), 12.0, Some((Color::from_rgba(1.0, 1.0, 1.0, 0.13), 1.0))))
        .into()
}

fn preview(a: &App) -> E<'_> {
    let p = a.p;
    let pic: E = match (&a.shot, a.up) {
        (Some(h), true) => image(h.clone()).width(Length::Fill).height(Length::Fill).content_fit(iced::ContentFit::Contain).into(),
        _ => Space::new().width(Length::Fill).height(Length::Fill).into(),
    };
    let label = container(tb(format!("PREVIEW · WHAT VIEWERS SEE · {} × {}", a.base.0, a.base.1), 11.0, p.muted).wrapping(text::Wrapping::None)).padding(pad(3.0, 8.0, 3.0, 8.0)).style(ui::boxed(p.panel2, 6.0, Some((p.line, 1.0))));
    let mut layers: Vec<E> = vec![pic, container(label).width(Length::Fill).height(Length::Fill).padding(pad(0.0, 0.0, 14.0, 0.0)).align_bottom(Length::Fill).center_x(Length::Fill).into()];
    if !a.up {
        layers.push(container(setup_card(a)).width(Length::Fill).height(Length::Fill).center_x(Length::Fill).center_y(Length::Fill).into());
    }
    container(iced::widget::Stack::with_children(layers).width(Length::Fill).height(Length::Fill)).clip(true).width(Length::Fill).height(Length::Fill).style(ui::boxed(Color::BLACK, 12.0, Some((p.line, 1.0)))).into()
}

// ------------------------------------------------------------------ going live
fn big<'a>(p: &Pal, label: String, bg: Color, fg: Color, line: Color, m: Option<Msg>) -> E<'a> {
    let p = *p;
    button(container(ts(label, 14.0, fg)).center_x(Length::Fill).center_y(Length::Fixed(46.0)))
        .width(Length::Fill)
        .padding(0.0)
        .on_press_maybe(m)
        .style(move |_: &Theme, st: button::Status| button::Style {
            background: Some(Background::Color(if matches!(st, button::Status::Hovered) { Pal::mix(bg, p.text, 0.06) } else { bg })),
            text_color: fg,
            border: Border { color: line, width: 1.0, radius: 10.0.into() },
            shadow: Shadow::default(),
            snap: true,
        })
        .into()
}

fn live(a: &App) -> E<'_> {
    let p = a.p;
    let state = if a.up { tag("CONNECTED", hexc(0x21361b), GREEN, hexc(0x3b5a2e)) } else { tag("NOT CONNECTED", hexc(0x3d1717), RED, hexc(0x6a2a2a)) };
    let mut where_ = Row::new().spacing(6.0);
    for s in &a.services {
        where_ = where_.push(chip(&p, s.clone(), Ck::Acc, 28.0, Some(Msg::Open(Dlg::Where))));
    }
    where_ = where_.push(chip(&p, "+", Ck::Normal, 28.0, Some(Msg::Open(Dlg::Where))));
    let title = container(text_input("Your stream's title", &a.title_edit).on_input(Msg::Title).on_submit(Msg::TitleDone).size(12.0).padding(pad(7.0, 10.0, 7.0, 10.0)).style(input_style(p))).width(Length::Fixed(330.0));
    let q = |w: u32, h: u32| a.output.0 == w && a.output.1 == h;
    let qual = seg(&p, vec![("720p".into(), q(1280, 720), Some(Msg::Quality(1280, 720))), ("1080p60".into(), q(1920, 1080), Some(Msg::Quality(1920, 1080))), ("1440p".into(), q(2560, 1440), Some(Msg::Quality(2560, 1440)))]);
    let en = a.up;
    let on = |m: Msg| if en { Some(m) } else { None };
    let (s_bg, s_fg, s_line, s_label) = if a.stream.active { (hexc(0xff4a4a), Color::WHITE, hexc(0xff4a4a), "■ End stream") } else { (p.accent, p.ink, p.accent, "● Go live") };
    let (r_bg, r_fg, r_line, r_label) = if a.record.active { (hexc(0x3d1717), hexc(0xff9a9a), hexc(0x6a2a2a), "■ Stop recording") } else { (p.panel2, p.text, p.line, "● Start recording") };
    let grid = column![
        row![big(&p, s_label.into(), s_bg, s_fg, s_line, on(Msg::Stream)), big(&p, r_label.into(), r_bg, r_fg, r_line, on(Msg::Record))].spacing(10.0),
        row![
            big(&p, if a.record.paused { "▶ Resume recording".into() } else { "❚❚ Pause recording".into() }, p.panel2, p.text, p.line, if en && a.record.active { Some(Msg::Pause) } else { None }),
            big(&p, if a.replay { "↺ Save last 30 s".into() } else { "↺ Keep the last 30 s".into() }, p.panel2, p.text, p.line, on(Msg::Replay))
        ]
        .spacing(10.0),
        row![
            big(&p, if a.vcam { "◉ Virtual camera on".into() } else { "◉ Virtual camera".into() }, if a.vcam { Pal::mix(p.panel2, p.accent, 0.25) } else { p.panel2 }, p.text, p.line, on(Msg::VirtualCam)),
            big(&p, if a.studio_mode { "◫ Studio mode on".into() } else { "◫ Studio mode".into() }, if a.studio_mode { Pal::mix(p.panel2, p.accent, 0.25) } else { p.panel2 }, p.text, p.line, on(Msg::StudioMode))
        ]
        .spacing(10.0)
    ]
    .spacing(10.0);
    let body = column![
        row_of(&p, "Where", where_.into()),
        row_of(&p, "Title", title.into()),
        row_of(&p, "Quality", qual),
        row_of(&p, "Also make a 9:16 for phones", switch(&p, a.vertical(), Msg::Vertical(!a.vertical()))),
        container(grid).padding(pad(10.0, 0.0, 0.0, 0.0))
    ];
    pane_h(&p, head(&p, "Going live", Some(state)), body.into(), Length::Shrink)
}

fn health(a: &App) -> E<'_> {
    let p = a.p;
    let good = a.stream.skipped == 0 || a.stream.total == 0 || (a.stream.skipped as f64 / a.stream.total as f64) < 0.005;
    let right = if !a.up {
        "not connected".to_string()
    } else if !a.stream.active {
        "not live".to_string()
    } else if good && a.stream.congestion < 0.2 {
        if a.services.len() > 1 { "both look good".into() } else { "looks good".into() }
    } else {
        "dropping frames: lower the quality".into()
    };
    let vals: Vec<f32> = a.rate.iter().copied().collect();
    let mx = vals.iter().copied().fold(1.0f32, f32::max) * 1.25;
    let g = ui::graph(ui::Graph { values: vals, dashed: vec![], max: mx, min: 0.0, color: GREEN, grid: hexc(0x2c2622), fill: true, width: 2.0, centred: false, gridlines: vec![mx * 0.25, mx * 0.5, mx * 0.75] }, Length::Fill, Length::Fixed(70.0));
    let st = |l: &'static str, v: String| -> E<'static> {
        column![container(row![t1(l, 12.0, a.p.muted), Space::new().width(Length::Fill), t1(v, 12.0, a.p.text)]).padding(pad(5.0, 0.0, 5.0, 0.0)), ui::hline(hexc(0x2c2622))].into()
    };
    let upload = match a.prefs.get("upload").and_then(serde_json::Value::as_f64) {
        Some(u) if u > 0.0 => format!("{:.1} Mbps of {}", a.mbps, u as i64),
        _ => format!("{:.1} Mbps", a.mbps),
    };
    let rec_to = if a.record_dir.is_empty() {
        "—".to_string()
    } else {
        let d = hearth_retro_tilde(&a.record_dir);
        if a.record.active { format!("{d} · {} so far", ui::bytes(a.record.bytes as f64)) } else { d }
    };
    let stats = row![
        column![st("Viewers", a.viewers.map(|v| v.to_string()).unwrap_or_else(|| "—".into())), st("Upload", upload)].width(Length::Fill),
        gap(24.0, 1.0),
        column![st("Frames dropped", format!("{} of {}", a.stream.skipped, thousands(a.stream.total))), st("Recording to", rec_to)].width(Length::Fill)
    ];
    pane(&p, head(&p, "Stream health", Some(mu(&p, right))), column![g, container(stats).padding(pad(8.0, 0.0, 0.0, 0.0))].into())
}

fn thousands(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

fn hearth_retro_tilde(p: &str) -> String {
    let h = hearth_ui::paths::home().display().to_string();
    match p.strip_prefix(&h) {
        Some(r) => format!("~{r}"),
        None => p.to_string(),
    }
}

// ------------------------------------------------------------------ scenes, sources, sound, switching
fn it_row<'a>(p: &Pal, on: bool, inner: E<'a>, m: Option<Msg>) -> E<'a> {
    let p = *p;
    let c = container(inner).padding(pad(0.0, 10.0, 0.0, 10.0)).height(Length::Fixed(36.0)).center_y(Length::Fixed(36.0)).width(Length::Fill).style(move |_: &Theme| container::Style {
        background: Some(Background::Color(if on { hexc(0x3b2a1d) } else { Color::TRANSPARENT })),
        border: iced::border::rounded(8.0),
        ..container::Style::default()
    });
    let c: E = if on { row![container(Space::new()).width(Length::Fixed(3.0)).height(Length::Fixed(36.0)).style(ui::boxed(p.accent, 2.0, None)), c].into() } else { c.into() };
    match m {
        Some(m) => mouse_area(c).on_press(m).interaction(mouse::Interaction::Pointer).into(),
        None => c,
    }
}

fn scenes(a: &App) -> E<'_> {
    let p = a.p;
    let mut c = Column::new().spacing(0.0);
    if a.scenes.is_empty() {
        c = c.push(mu(&p, if a.up { "No scenes yet" } else { "OBS isn't connected" }));
    }
    for sc in &a.scenes {
        let on = *sc == a.scene;
        let ic = container(Space::new()).width(Length::Fixed(16.0)).height(Length::Fixed(16.0)).style(ui::boxed(if on { p.accent } else { hexc(0xd8cfc8) }, 4.0, None));
        c = c.push(it_row(&p, on, row![ic, t1(sc.clone(), 13.0, p.text)].spacing(10.0).align_y(iced::Center).into(), Some(Msg::Scene(sc.clone()))));
    }
    pane(&p, head(&p, "Scenes", Some(mouse_area(t("+", 14.0, p.muted)).on_press(Msg::Open(Dlg::NewScene)).interaction(mouse::Interaction::Pointer).into())), iced::widget::scrollable(c).height(Length::Fill).into())
}

fn kind_colour(p: &Pal, i: &Item) -> Color {
    let k = i.kind.as_str();
    let n = i.name.to_lowercase();
    if k.starts_with("text_") {
        hexc(0x5ec8ff)
    } else if k.contains("v4l2") && (n.contains("capture") || n.contains("card")) {
        hexc(0xe36bd3)
    } else if k.contains("v4l2") || n.contains("camera") {
        p.accent
    } else if k.contains("window") || k.contains("game") || n.contains("game") {
        GREEN
    } else if k.contains("screen") || k.contains("desktop") || n.contains("screen") {
        hexc(0xffd36b)
    } else {
        p.muted
    }
}

/// The lock beside a part that's locked in place (a small padlock, drawn).
struct Lock(Color);
impl canvas::Program<Msg> for Lock {
    type State = ();
    fn draw(&self, _s: &(), r: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        let mut f = Frame::new(r, b.size());
        let w = b.width;
        f.fill(&Path::rounded_rectangle(Point::new(w * 0.15, b.height * 0.45), Size::new(w * 0.7, b.height * 0.5), 2.0.into()), self.0);
        let arc = Path::new(|pb| {
            pb.move_to(Point::new(w * 0.3, b.height * 0.47));
            pb.line_to(Point::new(w * 0.3, b.height * 0.28));
            pb.arc_to(Point::new(w * 0.3, b.height * 0.05), Point::new(w * 0.5, b.height * 0.05), w * 0.2);
            pb.arc_to(Point::new(w * 0.7, b.height * 0.05), Point::new(w * 0.7, b.height * 0.28), w * 0.2);
            pb.line_to(Point::new(w * 0.7, b.height * 0.47));
        });
        f.stroke(&arc, canvas::Stroke::default().with_color(self.0).with_width(1.6));
        vec![f.into_geometry()]
    }
}

fn items(a: &App) -> E<'_> {
    let p = a.p;
    let mut c = Column::new();
    for i in &a.items {
        let col = kind_colour(&p, i);
        let dot = container(Space::new()).width(Length::Fixed(16.0)).height(Length::Fixed(16.0)).style(ui::boxed(col, 8.0, None));
        let eye = mouse_area(t(if i.on { "◉" } else { "○" }, 12.0, p.muted)).on_press(Msg::ToggleItem(i.id, !i.on)).interaction(mouse::Interaction::Pointer);
        let lock: E = mouse_area(container(canvas::Canvas::new(Lock(if i.locked { hexc(0xe8b84a) } else { Pal::a(p.muted, 0.35) })).width(Length::Fixed(12.0)).height(Length::Fixed(14.0))).padding(pad(0.0, 0.0, 0.0, 8.0)))
            .on_press(Msg::LockItem(i.id, !i.locked))
            .interaction(mouse::Interaction::Pointer)
            .into();
        let name = t1(i.name.clone(), 13.0, if i.on { p.text } else { Pal::a(p.text, 0.55) });
        let inner = row![dot, name, Space::new().width(Length::Fill), eye, lock].spacing(10.0).align_y(iced::Center);
        c = c.push(it_row(&p, a.item_sel == Some(i.id), inner.into(), Some(Msg::PickItem(i.id))));
    }
    if a.items.is_empty() {
        c = c.push(mu(&p, if a.up { "Nothing in this scene yet" } else { "OBS isn't connected" }));
    }
    let add = mouse_area(t1("+ Add", 13.0, p.muted)).on_press(Msg::Open(Dlg::AddSource)).interaction(mouse::Interaction::Pointer);
    pane(&p, head(&p, "In this scene", Some(add.into())), iced::widget::scrollable(c).height(Length::Fill).into())
}

/// A channel's meter and its fader: drag to set (x³ is the volume, as OBS's own fader), the white line is where it's set.
struct Fader {
    name: String,
    x: f32,
}

#[derive(Default)]
struct FaderState {
    held: bool,
}

impl canvas::Program<Msg> for Fader {
    type State = FaderState;
    fn update(&self, st: &mut FaderState, ev: &canvas::Event, b: Rectangle, c: mouse::Cursor) -> Option<canvas::Action<Msg>> {
        let at = |c: mouse::Cursor| c.position_in(b).map(|p| (p.x / b.width).clamp(0.0, 1.0)).or_else(|| c.position().map(|p| ((p.x - b.x) / b.width).clamp(0.0, 1.0)));
        match ev {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) if c.is_over(b) => {
                st.held = true;
                at(c).map(|x| canvas::Action::publish(Msg::Volume(self.name.clone(), x)).and_capture())
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) if st.held => at(c).map(|x| canvas::Action::publish(Msg::Volume(self.name.clone(), x)).and_capture()),
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) if st.held => {
                st.held = false;
                Some(canvas::Action::capture())
            }
            _ => None,
        }
    }
    fn draw(&self, _s: &FaderState, r: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        let mut f = Frame::new(r, b.size());
        f.fill_rectangle(Point::new((self.x * b.width - 1.0).clamp(0.0, b.width - 2.0), 0.0), Size::new(2.0, b.height), Color::WHITE);
        vec![f.into_geometry()]
    }
    fn mouse_interaction(&self, _s: &FaderState, b: Rectangle, c: mouse::Cursor) -> mouse::Interaction {
        if c.is_over(b) { mouse::Interaction::ResizingHorizontally } else { mouse::Interaction::default() }
    }
}

fn level_x(mul: f32) -> f32 {
    // the same scale as the fader: x = mul^(1/3)
    mul.max(0.0).powf(1.0 / 3.0).min(1.0)
}

fn sound_row<'a>(p: &Pal, au: &'a Audio) -> E<'a> {
    let p = *p;
    let fill = if au.muted { 0.0 } else { level_x(au.level) };
    let fp = (fill * 1000.0).round() as u16;
    let mut bar = Row::new();
    if fp > 0 {
        bar = bar.push(container(Space::new()).width(Length::FillPortion(fp)).height(Length::Fixed(12.0)).style(|_: &Theme| container::Style {
            background: Some(iced::Gradient::Linear(iced::gradient::Linear::new(std::f32::consts::FRAC_PI_2).add_stop(0.0, hexc(0x3ba55c)).add_stop(0.6, hexc(0x9be36b)).add_stop(0.82, hexc(0xffd36b)).add_stop(1.0, hexc(0xff6b6b))).into()),
            ..container::Style::default()
        }));
    }
    if fp < 1000 {
        bar = bar.push(Space::new().width(Length::FillPortion(1000 - fp)).height(Length::Fixed(12.0)));
    }
    let x = if au.db <= -99.0 { 0.0 } else { 10f32.powf(au.db as f32 / 60.0).min(1.0) };
    let meter = stack![
        container(bar).width(Length::Fill).height(Length::Fixed(12.0)).clip(true).style(ui::boxed(hexc(0x141110), 3.0, None)),
        canvas::Canvas::new(Fader { name: au.name.clone(), x }).width(Length::Fill).height(Length::Fixed(12.0))
    ];
    let db = if au.muted { "muted".to_string() } else if au.db <= -99.0 { "−∞ dB".into() } else { format!("{} dB", (au.db.round() as i64).to_string().replace('-', "−")) };
    let dot = mouse_area(t("●", 12.0, if au.muted { RED } else { GREEN })).on_press(Msg::Mute(au.name.clone(), !au.muted)).interaction(mouse::Interaction::Pointer);
    container(row![container(t1(au.name.clone(), 12.0, p.text)).width(Length::Fixed(110.0)), meter, container(t1(db, 12.0, p.muted)).width(Length::Fixed(50.0)), container(dot).width(Length::Fixed(24.0)).center_x(Length::Fixed(24.0))].spacing(10.0).align_y(iced::Center)).padding(pad(8.0, 0.0, 8.0, 0.0)).into()
}

fn sound(a: &App) -> E<'_> {
    let p = a.p;
    let mut c = Column::new();
    for au in &a.audio {
        c = c.push(sound_row(&p, au));
    }
    if a.audio.is_empty() {
        c = c.push(mu(&p, if a.up { "No sound sources" } else { "OBS isn't connected" }));
    }
    let has = |k: &str| a.mic_filters.iter().any(|f| f.1.contains(k) && f.2);
    let clean = if a.mic.is_empty() {
        "No mic in OBS yet".to_string()
    } else {
        format!("Mic clean-up: noise removal {} · leveller {}", if has("noise_suppress") { "on" } else { "off" }, if has("compressor") || has("limiter") || has("expander") { "on" } else { "off" })
    };
    let foot = row![t1(clean, 12.0, p.muted), Space::new().width(Length::Fill), chip(&p, "Filters…", Ck::Normal, 28.0, if a.mic.is_empty() { None } else { Some(Msg::Open(Dlg::Filters)) })].align_y(iced::Center).spacing(8.0);
    pane(&p, head(&p, "Sound", Some(mu(&p, "drag to set · click the dot to mute"))), column![iced::widget::scrollable(c).height(Length::Fill), foot].into())
}

fn switching(a: &App) -> E<'_> {
    let p = a.p;
    let parts: Vec<(String, bool, Option<Msg>)> = ["Cut", "Fade", "Slide", "Ember"]
        .iter()
        .map(|n| {
            let here = a.transitions.iter().any(|x| x == n);
            (n.to_string(), a.transition == *n, if a.up && (here || *n == "Ember") { Some(Msg::Transition(n.to_string())) } else { None })
        })
        .collect();
    let len = container(text_input("300 ms", &a.duration).on_input(Msg::Duration).on_submit(Msg::DurationDone).size(12.0).padding(pad(6.0, 10.0, 6.0, 10.0)).style(input_style(p))).width(Length::Fixed(80.0));
    let body = column![
        seg(&p, parts),
        container(row_of(&p, "Length", len.into())).padding(pad(6.0, 0.0, 0.0, 0.0)),
        container(tb("HOTKEYS", 11.0, p.muted)).padding(pad(14.0, 0.0, 8.0, 0.0)),
        column![t("Ctrl+1–6 · scenes", 12.0, p.text), t("Ctrl+Shift+R · save last 30 s", 12.0, p.text), t("Controller: ≡ + Y · next scene", 12.0, p.text)].spacing(10.0)
    ];
    pane(&p, head(&p, "Switching scenes", None), body.into())
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
    let field = |ph: &str, v: &str, f: fn(String) -> Msg, secure: bool| -> E<'static> { text_input(ph, v).on_input(f).secure(secure).size(14.0).padding(pad(9.0, 12.0, 9.0, 12.0)).style(input_style(a.p)).into() };
    let buttons = |ok: &'static str, m: Msg| -> E<'static> { row![chip(&a.p, ok, Ck::Acc, 34.0, Some(m)), chip(&a.p, "Cancel", Ck::Normal, 34.0, Some(Msg::Close))].spacing(8.0).into() };
    let inner: E = match d {
        Dlg::NewScene => column![ts("A new scene", 17.0, p.text), field("Its name", &a.new_name, Msg::NewName, false), buttons("Make it", Msg::CreateScene)].spacing(12.0).into(),
        Dlg::AddSource => {
            let mut c = Column::new().spacing(6.0).push(ts(format!("Add to {}", a.scene), 17.0, p.text)).push(field("Its name (or the kind's)", &a.new_name, Msg::NewName, false));
            for (kind, label) in [
                ("v4l2_input", "Camera"),
                ("v4l2_input", "Capture Card"),
                ("pipewire-screen-capture-source", "Whole screen"),
                ("pipewire-window-capture-source", "Game window"),
                ("pulse_input_capture", "Mic"),
                ("pulse_output_capture", "Game and desktop sound"),
                ("text_ft2_source_v2", "Overlay: title"),
                ("image_source", "Picture"),
            ] {
                c = c.push(chip(&p, label, Ck::Normal, 34.0, Some(Msg::AddSource(kind, label))));
            }
            c.push(chip(&p, "Cancel", Ck::Normal, 34.0, Some(Msg::Close))).into()
        }
        Dlg::Filters => {
            let mut c = Column::new().spacing(8.0).push(ts(format!("{}'s filters", a.mic), 17.0, p.text));
            for (name, kind, on) in &a.mic_filters {
                c = c.push(row![t1(name.clone(), 13.0, p.text), mu(&p, format!("  {kind}")), Space::new().width(Length::Fill), switch(&p, *on, Msg::Filter(name.clone(), !*on))].align_y(iced::Center));
            }
            if !a.mic_filters.iter().any(|f| f.1.contains("noise_suppress")) {
                c = c.push(chip(&p, "Add noise removal", Ck::Normal, 30.0, Some(Msg::AddFilter("noise_suppress_filter_v2", "Noise removal"))));
            }
            if !a.mic_filters.iter().any(|f| f.1.contains("compressor")) {
                c = c.push(chip(&p, "Add the leveller", Ck::Normal, 30.0, Some(Msg::AddFilter("compressor_filter", "Leveller"))));
            }
            c.push(chip(&p, "Done", Ck::Acc, 34.0, Some(Msg::Close))).into()
        }
        Dlg::Where => {
            let mut svc = Row::new().spacing(6.0);
            for s in ["Twitch", "YouTube", "Kick"] {
                svc = svc.push(chip(&p, s, if a.service_pick == s { Ck::Acc } else { Ck::Normal }, 30.0, Some(Msg::Service(s))));
            }
            column![
                ts("Where you go live", 17.0, p.text),
                t("OBS goes live to one place at a time; this sets it in OBS. Your stream key is given to OBS and kept there, not by Hearth.", 13.0, p.muted),
                svc,
                field("Your stream key (empty: keep the one OBS has)", &a.key_edit, Msg::Key, true),
                buttons("Set it", Msg::SaveService)
            ]
            .spacing(12.0)
            .into()
        }
        Dlg::Password => column![
            ts("OBS's remote control password", 17.0, p.text),
            t("OBS › Tools › WebSocket Server Settings › Show Connect Info. Kept only on this PC, readable only by you.", 13.0, p.muted),
            field("The password", &a.pw_edit, Msg::Password, true),
            buttons("Connect", Msg::SavePassword)
        ]
        .spacing(12.0)
        .into(),
        Dlg::Ember => column![
            ts("Ember: Hearth's own transition", 17.0, p.text),
            t("An ember-orange fade between scenes. OBS takes new transitions only while it's closed: close OBS, then Start OBS here, and Hearth Studio adds Ember first.", 13.0, p.text),
            chip(&p, "OK", Ck::Acc, 34.0, Some(Msg::Close))
        ]
        .spacing(12.0)
        .into(),
        Dlg::Said(s) => column![t(s.clone(), 14.0, p.text), chip(&p, "OK", Ck::Acc, 34.0, Some(Msg::Close))].spacing(12.0).into(),
    };
    Some(ui::dialog_box(&p, 520.0, inner))
}

// ------------------------------------------------------------------ blocks
pub const BLOCKS: &[PageDef] = &[PageDef {
    id: "window",
    title: "Hearth Studio",
    blocks: &[
        bl("title", "The title bar", 0, 0, 12, 1),
        bl("preview", "What viewers see", 0, 1, 7, 7),
        bl("live", "Going live", 7, 1, 5, 4),
        bl("health", "Stream health", 7, 5, 5, 3),
        bl("scenes", "Scenes", 0, 8, 2, 3),
        bl("items", "In this scene", 2, 8, 3, 3),
        bl("sound", "Sound", 5, 8, 5, 3),
        bl("switching", "Switching scenes", 10, 8, 2, 3),
        bl("credit", "Built on OBS Studio", 0, 11, 12, 1),
    ],
}];

pub const KIT: Kit = Kit {
    name: "Hearth Studio",
    buttons: &[
        tile("stream", "Go live or end the stream", G::Play),
        tile("record", "Record or stop", G::Stop),
        tile("pause", "Pause recording", G::Clock),
        tile("replay", "Save last 30 s", G::Restore),
        tile("vcam", "Virtual camera", G::Camera),
        tile("studio-mode", "Studio mode", G::Columns),
        tile("new-scene", "A new scene", G::Plus),
        tile("add-source", "Add to the scene", G::Plus),
        tile("filters", "Mic filters", G::Sound),
        tile("start-obs", "Start OBS", G::Power),
    ],
    panes: &[tile("scenes", "Scenes", G::List), tile("sound", "Sound", G::Sound), tile("health", "Stream health", G::Pulse)],
    info: &[tile("live", "Live for", G::Clock), tile("rec", "Recording for", G::Clock), tile("fps", "Frames a second", G::Pulse), tile("dropped", "Frames dropped", G::Info)],
    pane_page: "window",
    bars: &[],
    menus: &[],
};

pub fn block_action(id: &str) -> Option<Msg> {
    Some(match id {
        "stream" => Msg::Stream,
        "record" => Msg::Record,
        "pause" => Msg::Pause,
        "replay" => Msg::Replay,
        "vcam" => Msg::VirtualCam,
        "studio-mode" => Msg::StudioMode,
        "new-scene" => Msg::Open(Dlg::NewScene),
        "add-source" => Msg::Open(Dlg::AddSource),
        "filters" => Msg::Open(Dlg::Filters),
        "start-obs" => Msg::StartObs,
        _ => return None,
    })
}

pub fn view(a: &App) -> E<'_> {
    let p = a.p;
    a.describe.values(vec![
        ("live", if a.stream.active { a.stream.timecode.clone() } else { "not live".into() }),
        ("rec", if a.record.active { a.record.timecode.clone() } else { "not recording".into() }),
        ("fps", format!("{}", a.fps.round() as i64)),
        ("dropped", a.stream.skipped.to_string()),
    ]);
    let mut items_: Vec<(&'static str, E)> = vec![
        ("title", hearth_ui::studio::made(&a.describe, "window", "title", || title_bar(a))),
        ("preview", preview(a)),
        ("live", live(a)),
        ("health", health(a)),
        ("scenes", scenes(a)),
        ("items", items(a)),
        ("sound", sound(a)),
        ("switching", switching(a)),
        ("credit", credit(&p, "Hearth Studio", "Built on OBS Studio (GPL-2)")),
    ];
    for (id, w) in [("pane:scenes", "scenes"), ("pane:sound", "sound"), ("pane:health", "health")] {
        if a.describe.wants_pane(w) {
            items_.push((id, match w {
                "scenes" => scenes(a),
                "sound" => sound(a),
                _ => health(a),
            }));
        }
    }
    let body = hearth_ui::arrange::parts(
        &a.describe,
        &p,
        "window",
        items_,
        |ps| {
            let top = row![container(ps.take("preview")).width(Length::FillPortion(1012)), container(column![ps.take("live"), ps.take("health")].spacing(14.0)).width(Length::FillPortion(780))].spacing(14.0).height(Length::FillPortion(584));
            let bottom = row![container(ps.take("scenes")).width(Length::FillPortion(280)), container(ps.take("items")).width(Length::FillPortion(380)), container(ps.take("sound")).width(Length::FillPortion(800)), container(ps.take("switching")).width(Length::FillPortion(300))].spacing(14.0).height(Length::FillPortion(296));
            column![ps.take("title"), container(column![top, bottom].spacing(14.0)).padding(pad(14.0, 16.0, 4.0, 16.0)).height(Length::Fill), ps.take("credit")].into()
        },
        Msg::Describe,
    );
    let mut base: E = container(body).width(Length::Fill).height(Length::Fill).style(ui::boxed(Color { a: 0.97, ..hexc(0x1a1614) }, p.window_corner, Some((p.line, 1.0)))).into();
    if !a.said.is_empty() || !a.busy.is_empty() {
        let line = if a.busy.is_empty() { a.said.clone() } else { a.busy.clone() };
        base = stack![base, container(container(t1(line, 12.0, p.text)).padding(pad(6.0, 12.0, 6.0, 12.0)).style(ui::boxed(p.panel2, 8.0, Some((p.line, 1.0))))).width(Length::Fill).height(Length::Fill).align_bottom(Length::Fill).center_x(Length::Fill).padding(pad(0.0, 0.0, 6.0, 0.0))].into();
    }
    let base = match dialog(a) {
        Some(d) => ui::dialog(base, d, Some(Msg::Close)),
        None => base,
    };
    ui::edges(base, Msg::Resize)
}
