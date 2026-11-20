//! The desktop and laptop pages (mockups 1, 1b, 2, 2b, 3, 4 and 4b, 1920 × 1080): the viewer with its slim bar, the
//! window pages with the title bar, the sidebar (Live view, Recordings & screenshots with its count, Settings; the
//! card's status; "From the Hearth Store") and the controller hints along the bottom. In Game Mode the window pages
//! are the same, full screen.

use crate::look::*;
use crate::{App, Dd, Dlg, Msg, Page, Sig, Tab};
use cc::library::Kind;
use iced::widget::{column, container, image, mouse_area, pin, row, scrollable, stack, text_input, Column, Row, Space};
use iced::{Color, Length, Theme};
use serde_json::{json, Value};

pub const RENAME_FIELD: &str = "capture-card-rename";

const SIG_BG: Color = Color { r: 0x17 as f32 / 255.0, g: 0x35 as f32 / 255.0, b: 0x24 as f32 / 255.0, a: 1.0 };
const SIG_TX: Color = Color { r: 0x8f as f32 / 255.0, g: 0xf0 as f32 / 255.0, b: 0xb3 as f32 / 255.0, a: 1.0 };
const VB_TX: Color = Color { r: 0xe3 as f32 / 255.0, g: 0xe6 as f32 / 255.0, b: 0xeb as f32 / 255.0, a: 1.0 };
const REC_RED: Color = Color { r: 1.0, g: 0x4b as f32 / 255.0, b: 0x4b as f32 / 255.0, a: 1.0 };

fn aw(alpha: f32) -> Color {
    Color { r: 1.0, g: 1.0, b: 1.0, a: alpha }
}

fn glass(alpha: f32) -> Color {
    Color { a: alpha, ..Color::from_rgb8(0x12, 0x14, 0x19) }
}

/// "1080p60", from the mode in use or the last one.
pub fn mode_short(a: &App) -> String {
    a.mode.as_ref().map(|m| m.short()).or_else(|| a.last_live.as_ref().map(|l| l.0.clone())).unwrap_or_else(|| format!("{}p{}", a.prefs.h, a.prefs.fps))
}

pub fn delay_words(a: &App) -> String {
    a.delay_ms.or_else(|| a.last_live.as_ref().map(|l| l.1)).map(|d| format!("{d} ms")).unwrap_or_else(|| "– ms".into())
}

pub fn card_name(a: &App) -> String {
    a.card.as_ref().map(|c| c.name.clone()).unwrap_or_else(|| "Capture card".into())
}

// ------------------------------------------------------------------ the frame
fn title_bar<'a>(a: &'a App) -> E<'a, Msg> {
    let wb = |s: &str, m: Msg| -> E<'a, Msg> { click(container(one(t(s.to_string(), 17.0, MUT))).center_x(Length::Fixed(24.0)).center_y(Length::Fixed(52.0)), Some(m)) };
    let name = row![app_icon(28.0, 18.0, 8.0), gap(12.0, 1.0), one(t6("Capture Card", 17.0, TX)), gap(8.0, 1.0), one(t("· Hearth", 15.0, MUT))].align_y(iced::Center);
    let drag = mouse_area(container(row![name, fill()].align_y(iced::Center)).height(Length::Fixed(52.0)).width(Length::Fill)).on_press(Msg::Drag).on_double_click(Msg::Max);
    let mut r = Row::new().align_y(iced::Center).push(gap(20.0, 1.0)).push(drag);
    if !a.game {
        r = r.push(wb("–", Msg::Min)).push(gap(24.0, 1.0)).push(wb("▢", Msg::Max)).push(gap(24.0, 1.0)).push(wb("✕", Msg::Close));
    }
    r = r.push(gap(20.0, 1.0));
    column![container(r).height(Length::Fixed(51.0)).width(Length::Fill).style(bx(SIDE, 0.0, None)), hline(LINE)].into()
}

fn nav_item<'a>(g: G, label: &str, on: bool, right: E<'a, Msg>, m: Msg) -> E<'a, Msg> {
    let col = if on { Color::WHITE } else { Color::from_rgb8(0xd5, 0xd8, 0xde) };
    let bar: E<'a, Msg> = if on { container(Space::new()).width(Length::Fixed(3.0)).height(Length::Fixed(24.0)).style(bx(ACC, 2.0, None)).into() } else { gap(3.0, 24.0) };
    let r = row![bar, gap(11.0, 1.0), glyph(g, if on { ACC2 } else { MUT }, 20.0), gap(12.0, 1.0), one(t(label.to_string(), 16.0, col)), fill(), right, gap(14.0, 1.0)].align_y(iced::Center);
    click(container(r).height(Length::Fixed(44.0)).center_y(Length::Fixed(44.0)).width(Length::Fill).style(bx(if on { CARD2 } else { Color::TRANSPARENT }, 9.0, None)), Some(m))
}

/// The card's status at the foot of the sidebar: (bold line, dot, line).
pub fn status(a: &App) -> (String, Color, String) {
    match (a.card.as_ref(), a.sig) {
        (None, _) => ("No capture card".into(), WARN, "Plug one into USB 3.0".into()),
        (Some(c), Sig::NoSignal) => (c.name.clone(), BAD, "No HDMI signal".into()),
        (Some(c), Sig::Live) => (c.name.clone(), OK, format!("{} · delay {}", mode_short(a), delay_words(a))),
        (Some(c), _) => match &a.last_live {
            Some((m, d)) => (c.name.clone(), OK, format!("{m} · delay {d} ms")),
            None => (c.name.clone(), OK, format!("{} · ready", mode_short(a))),
        },
    }
}

fn sidebar<'a>(a: &'a App) -> E<'a, Msg> {
    let n = a.items.len();
    let count = if n > 0 || a.page == Page::Library { n.to_string() } else { String::new() };
    let mut items = Column::new().spacing(4.0);
    items = items.push(nav_item(G::Live, "Live view", a.page == Page::Live, gap(1.0, 1.0), Msg::Go(Page::Live)));
    items = items.push(nav_item(G::List, "Recordings & screenshots", a.page == Page::Library, one(t(count, 13.0, OFF)).into(), Msg::Go(Page::Library)));
    items = items.push(nav_item(G::Gear, "Settings", a.page == Page::Settings, gap(1.0, 1.0), Msg::Go(Page::Settings)));
    let (b, dot, line) = status(a);
    let foot = column![
        one(t6(b, 14.0, SOFT2)),
        row![dotted(dot, 9.0), gap(7.0, 1.0), one(t(line, 14.0, MUT))].align_y(iced::Center),
        one(t("From the Hearth Store", 13.0, MUT))
    ]
    .spacing(6.0);
    let col = column![items, vfill(), hline(LINE), container(foot).padding(pad(14.0, 8.0, 0.0, 8.0))];
    row![container(col).padding(pad(18.0, 14.0, 18.0, 14.0)).width(Length::Fixed(329.0)).height(Length::Fill).style(bx(SIDE, 0.0, None)), container(Space::new()).width(Length::Fixed(1.0)).height(Length::Fill).style(bx(LINE, 0.0, None))].into()
}

fn lbrb<'a>(what: &str) -> E<'a, Msg> {
    hint(&["LB", "RB"], what, 16.0)
}

fn frame<'a>(a: &'a App, page: E<'a, Msg>, left: Vec<E<'a, Msg>>, right: Vec<E<'a, Msg>>) -> E<'a, Msg> {
    let body: E<'a, Msg> = row![sidebar(a), page].height(Length::Fill).into();
    let all: E<'a, Msg> = column![title_bar(a), body, hints(left, right, 54.0, 28.0, 28.0, BG)].into();
    let all = container(all).style(bx(BG, 0.0, None)).width(Length::Fill).height(Length::Fill);
    if a.game {
        all.into()
    } else {
        hearth_ui::ui::edges(all.into(), Msg::Resize)
    }
}

fn h1<'a>(s: &str) -> E<'a, Msg> {
    container(one(t6(s.to_string(), 34.0, TX))).padding(pad(0.0, 0.0, 8.0, 0.0)).into()
}

fn note<'a>(a: &'a App) -> Option<E<'a, Msg>> {
    a.note.as_ref().map(|n| {
        container(row![glyph(G::Bolt, WARN, 16.0), gap(10.0, 1.0), t(n.clone(), 14.5, Color::from_rgb8(0xff, 0xe0, 0x8a)), gap(12.0, 1.0), click(one(t("✕", 14.0, MUT)), Some(Msg::NoteOff))].align_y(iced::Center))
            .padding(pad(10.0, 16.0, 10.0, 16.0))
            .style(bx(QUIET, 10.0, Some((Color::from_rgb8(0x7a, 0x66, 0x33), 1.0))))
            .into()
    })
}

pub fn view(a: &App) -> E<'_, Msg> {
    let page = match a.page {
        Page::Live => match (a.card.is_some(), a.sig) {
            (false, Sig::Looking) => looking(a),
            (false, _) => no_card(a),
            (true, _) => no_signal(a),
        },
        Page::Library => library(a),
        Page::Settings => settings(a),
    };
    let mut layers: Vec<E<'_, Msg>> = vec![page];
    if let Some(d) = a.dd {
        layers.push(chooser(a, d));
    }
    if let Some(d) = &a.dlg {
        layers.push(dialog(a, d));
    }
    if let Some(n) = note(a) {
        layers.push(container(n).center_x(Length::Fill).align_bottom(Length::Fill).padding(pad(0.0, 0.0, 70.0, 0.0)).into());
    }
    if layers.len() == 1 {
        return layers.pop().unwrap();
    }
    stack(layers).width(Length::Fill).height(Length::Fill).into()
}

fn looking(a: &App) -> E<'_, Msg> {
    let page = container(Space::new()).width(Length::Fill).height(Length::Fill).style(bx(DEEP, 0.0, None));
    frame(a, page.into(), vec![hint(&["A"], "Select", 16.0), hint(&["B"], "Back", 16.0)], vec![lbrb("Live view / Recordings / Settings")])
}

// ------------------------------------------------------------------ 2 no signal, 2b no card
fn tip_row<'a>(g: G, title: &str, line: &str, end: Option<E<'a, Msg>>) -> E<'a, Msg> {
    srow(Some(ri(g, 40.0)), title, Some(rich_w(line, 14.0, MUT, SOFT)), end, 17.0, CARD, pad(14.0, 20.0, 14.0, 20.0), false)
}

fn no_signal(a: &App) -> E<'_, Msg> {
    let c = a.card.as_ref();
    let name = card_name(a);
    let usb = c.map(|c| c.usb_words()).unwrap_or("USB");
    let usb3 = c.map(|c| c.usb3()).unwrap_or(false);
    let sound = a.sound.is_some();
    let icon = container(glyph(G::Live, ACC2, 52.0)).center_x(Length::Fixed(96.0)).center_y(Length::Fixed(96.0)).style(bx(Color::from_rgb8(0x2a, 0x20, 0x16), 24.0, Some((Color::from_rgb8(0x5a, 0x3b, 0x22), 1.0))));
    let head = row![
        icon,
        column![
            one(t6("No picture from your Xbox yet", 34.0, TX)),
            t("Your capture card is plugged in, but nothing is coming through its HDMI. The picture shows up here by itself as soon as it does.", 17.0, MUT).line_height(1.25)
        ]
        .spacing(6.0)
        .width(Length::Fill)
    ]
    .spacing(26.0)
    .align_y(iced::Center);
    let st = |ok: bool, s: String| -> E<'static, Msg> { row![dotted(if ok { OK } else { BAD }, 9.0), gap(7.0, 1.0), rich1(&s, 16.0, TX, TX)].align_y(iced::Center).into() };
    let status = container(
        row![
            st(true, format!("Card: **{name}**")),
            st(usb3, format!("On a **{usb}** port")),
            st(sound, if sound { "Sound: **found**".into() } else { "Sound: **not found**".into() }),
            st(false, "HDMI: **no signal**".into()),
            fill(),
            btn("Settings", Some(G::Gear), B::Normal, false, 15.0, a.ring && a.focus == 0, Some(Msg::Go(Page::Settings)))
        ]
        .spacing(30.0)
        .align_y(iced::Center),
    )
    .padding(pad(12.0, 22.0, 12.0, 22.0))
    .width(Length::Fill)
    .style(bx(QUIET, 12.0, None));
    let usb_end: Option<E<'_, Msg>> = if usb3 { Some(chip("You're on USB 3.0", Ch::Ok, None, 13.5)) } else if c.map(|c| c.speed > 0).unwrap_or(false) { Some(chip("You're on USB 2.0", Ch::Bad, None, 13.5)) } else { None };
    let mut body = Column::new().width(Length::Fill).max_width(1080.0);
    body = body.push(head).push(gap(1.0, 26.0)).push(status).push(gap(1.0, 22.0));
    body = body.push(sec("If it stays black, try these", 0.0));
    body = body.push(tip_row(G::Usb, "Use a USB 3.0 port", "The blue (or SS-marked) ports on your laptop. On USB 2.0 the card can only send a small, choppy picture.", usb_end)).push(gap(1.0, 8.0));
    body = body.push(tip_row(G::Shield, "Turn off HDCP on the Xbox", "On the Xbox: **Settings › General › TV & display options › Video modes**, then untick **Allow HDCP**. With HDCP on, the Xbox sends nothing to a capture card. (Some streaming apps on the Xbox need it on.)", None)).push(gap(1.0, 8.0));
    body = body.push(tip_row(G::Tv, "Set the Xbox to 1080p at 60 Hz", "On the Xbox: Settings › General › TV & display options › **Resolution: 1080p** and **Refresh rate: 60 Hz**. That's the sharpest, smoothest picture this card can bring in.", None)).push(gap(1.0, 8.0));
    body = body.push(tip_row(G::Card, "Check the HDMI cable", "Xbox HDMI out goes into the card's **HDMI IN**, and the Xbox is on (not in sleep).", None));
    let page = container(body).padding(pad(0.0, 40.0, 0.0, 40.0)).center_x(Length::Fill).center_y(Length::Fill).style(bx(DEEP, 0.0, None));
    frame(a, page.into(), vec![hint(&["A"], "Select", 16.0), hint(&["B"], "Back", 16.0)], vec![lbrb("Live view / Recordings / Settings")])
}

fn no_card(a: &App) -> E<'_, Msg> {
    let small = |g: G, title: &str, line: &str| -> E<'static, Msg> {
        container(row![ri(g, 40.0), column![t(title.to_string(), 17.0, TX), t(line.to_string(), 14.0, MUT).line_height(1.4)].spacing(3.0).width(Length::Fill)].spacing(16.0).align_y(iced::Top))
            .padding(pad(14.0, 20.0, 14.0, 20.0))
            .width(Length::FillPortion(1))
            .height(Length::Fixed(if a.size.0 < 1600.0 { 130.0 } else { 110.0 }))
            .style(bx(CARD, 12.0, None))
            .into()
    };
    let body = column![
        iced::widget::canvas(Plug).width(Length::Fixed(520.0)).height(Length::Fixed(180.0)),
        gap(1.0, 26.0),
        one(t6("Plug in your capture card", 34.0, TX)),
        gap(1.0, 8.0),
        container(t("Connect the card to a USB 3.0 port on this PC and your Xbox's HDMI to the card. Capture Card finds it by itself, there's nothing to install.", 17.0, MUT).line_height(1.25).align_x(iced::Center).width(Length::Fill)).width(Length::Fixed(760.0)),
        gap(1.0, 30.0),
        row![
            small(G::Usb, "A USB 3.0 port", "The blue or SS-marked ones. USB 2.0 gives a small, choppy picture."),
            small(G::Shield, "HDCP off on the Xbox", "Settings › General › TV & display options › Video modes › untick Allow HDCP."),
            small(G::Tv, "1080p at 60 Hz", "On the Xbox, in TV & display options. The best this card brings in.")
        ]
        .spacing(16.0)
        .width(Length::Fill),
        gap(1.0, 26.0),
        row![dotted(WARN, 9.0), gap(7.0, 1.0), one(t("Looking for a capture card… (it watches for USB devices being plugged in; nothing runs in between)", 15.0, MUT))].align_y(iced::Center)
    ]
    .align_x(iced::Center)
    .width(Length::Fill)
    .max_width(1080.0);
    let page = container(body).padding(pad(0.0, 40.0, 0.0, 40.0)).center_x(Length::Fill).center_y(Length::Fill).style(bx(DEEP, 0.0, None));
    frame(a, page.into(), vec![hint(&["A"], "Select", 16.0), hint(&["B"], "Back", 16.0)], vec![lbrb("Live view / Recordings / Settings")])
}

/// The Xbox › card › This PC picture (2b's SVG, drawn the same).
struct Plug;

impl<M> iced::widget::canvas::Program<M> for Plug {
    type State = Kept<iced::Size>;
    fn draw(&self, st: &Self::State, r: &iced::Renderer, _t: &Theme, b: iced::Rectangle, _c: iced::mouse::Cursor) -> Vec<iced::widget::canvas::Geometry> {
        use iced::widget::canvas::{Path, Stroke, Text};
        use iced::Point;
        st.check(b.size());
        vec![st.cache.draw(r, b.size(), |f| {
            let k = b.width / 520.0;
            let p = |x: f32, y: f32| Point::new(x * k, y * k);
            let rr = |x: f32, y: f32, w: f32, h: f32, rad: f32| Path::rounded_rectangle(p(x, y), iced::Size::new(w * k, h * k), (rad * k).into());
            let edge = Color::from_rgb8(0x3c, 0x41, 0x4c);
            let label = |f: &mut iced::widget::canvas::Frame, s: &str, cx: f32, base: f32, size: f32, col: Color, bold: bool| {
                // Centred on cx, the baseline at base (the text's top is about 0.75 of its size above it).
                let wd = s.chars().count() as f32 * size * 0.56;
                f.fill_text(Text { content: s.into(), position: p(cx - wd / 2.0, base - size * 0.95), color: col, size: (size * k).into(), font: if bold { w(iced::font::Weight::Bold) } else { hearth_ui::app::INTER }, ..Text::default() });
            };
            let xbox = rr(20.0, 50.0, 150.0, 80.0, 14.0);
            f.fill(&xbox, CARD);
            f.stroke(&xbox, Stroke::default().with_color(edge).with_width(2.0 * k));
            label(f, "Xbox", 95.0, 96.0, 16.0, MUT, false);
            f.stroke(&Path::line(p(170.0, 90.0), p(230.0, 90.0)), Stroke::default().with_color(MUT).with_width(5.0 * k).with_line_cap(iced::widget::canvas::LineCap::Round));
            label(f, "HDMI", 200.0, 76.0, 12.0, OFF, false);
            let card = rr(230.0, 66.0, 80.0, 48.0, 9.0);
            f.fill(&card, Color::from_rgb8(0x2a, 0x20, 0x16));
            f.stroke(&card, Stroke::default().with_color(ACC).with_width(2.0 * k));
            label(f, "card", 270.0, 95.0, 13.0, ACC2, true);
            // The dashed USB line: 10 on, 9 off.
            let mut x: f32 = 310.0;
            while x < 370.0 {
                let e = (x + 10.0).min(370.0);
                f.stroke(&Path::line(p(x, 90.0), p(e, 90.0)), Stroke::default().with_color(ACC).with_width(5.0 * k).with_line_cap(iced::widget::canvas::LineCap::Round));
                x += 19.0;
            }
            label(f, "USB 3.0", 340.0, 76.0, 12.0, ACC2, false);
            let pc = rr(370.0, 40.0, 140.0, 90.0, 8.0);
            f.fill(&pc, CARD);
            f.stroke(&pc, Stroke::default().with_color(edge).with_width(2.0 * k));
            f.fill(&rr(350.0, 130.0, 180.0, 12.0, 5.0), LINE);
            label(f, "This PC", 440.0, 92.0, 16.0, MUT, false);
        })]
    }
}

// ------------------------------------------------------------------ 3 Settings
pub fn dd_choices(a: &App, d: Dd) -> Vec<(Value, String)> {
    match d {
        Dd::Card => {
            let mut v: Vec<(Value, String)> = a.cards.iter().map(|c| (json!(c.id), format!("{} ({})", c.name, c.usb_words()))).collect();
            if v.is_empty() {
                v.push((json!(""), "No capture card found".into()));
            }
            v
        }
        Dd::Size => {
            let fmt = a.mode.as_ref().map(|m| m.format.clone()).unwrap_or_else(|| a.prefs.format.clone());
            let sizes = a.card.as_ref().map(|c| c.sizes(&fmt)).filter(|s| !s.is_empty()).unwrap_or_else(|| vec![(1920, 1080), (1280, 720)]);
            sizes.into_iter().map(|(w, h)| (json!(format!("{w}x{h}")), format!("{w} × {h}"))).collect()
        }
        Dd::Sound => {
            let mut v = vec![(json!(""), sound_auto(a))];
            for s in &a.sources {
                if a.sound.as_ref().map(|x| x.name == s.name).unwrap_or(false) && a.prefs.sound.is_empty() {
                    continue;
                }
                v.push((json!(s.name), s.description.clone()));
            }
            v
        }
    }
}

pub fn dd_now(a: &App, d: Dd) -> Value {
    match d {
        Dd::Card => json!(a.card.as_ref().map(|c| c.id.clone()).unwrap_or_default()),
        Dd::Size => json!(format!("{}x{}", a.prefs.w, a.prefs.h)),
        Dd::Sound => json!(a.prefs.sound),
    }
}

/// "USB3 Video (the card's sound)".
fn sound_auto(a: &App) -> String {
    match (&a.sound, a.prefs.sound.is_empty()) {
        (Some(s), true) => {
            let base = a.card.as_ref().map(|c| c.chip.split(" (").next().unwrap_or("").to_string()).filter(|b| !b.is_empty()).unwrap_or_else(|| s.description.clone());
            format!("{base} (the card's sound)")
        }
        (None, true) => "The card's sound (not found)".into(),
        (_, false) => "The card's sound".into(),
    }
}

fn sound_words(a: &App) -> String {
    if a.prefs.sound.is_empty() {
        return sound_auto(a);
    }
    a.sources.iter().find(|s| s.name == a.prefs.sound).map(|s| s.description.clone()).unwrap_or_else(|| a.prefs.sound.clone())
}

fn chooser(a: &App, d: Dd) -> E<'_, Msg> {
    let now = dd_now(a, d);
    let title = match d {
        Dd::Card => "Capture card",
        Dd::Size => "Resolution",
        Dd::Sound => "Sound comes from",
    };
    let mut c = Column::new().spacing(6.0).push(one(t6(title, 18.0, TX))).push(gap(1.0, 6.0));
    for (v, words) in dd_choices(a, d) {
        let on = v == now;
        c = c.push(click(container(row![one(t(words, 16.0, if on { INK } else { TX })), fill()]).padding(pad(10.0, 14.0, 10.0, 14.0)).width(Length::Fill).style(bx(if on { ACC } else { CARD }, 9.0, None)), Some(Msg::Pick(d, v))));
    }
    c = c.push(gap(1.0, 6.0)).push(row![fill(), btn("Close", None, B::Normal, false, 15.0, false, Some(Msg::Dd(None)))]);
    hearth_ui::ui::dialog(Space::new().into(), container(c).padding(20.0).width(Length::Fixed(420.0)).style(shadowed(SIDE, 14.0, Some((LINE, 1.0)), 40.0)).into(), Some(Msg::Dd(None)))
}

fn settings(a: &App) -> E<'_, Msg> {
    let p = &a.prefs;
    let f = |i: usize| a.ring && a.focus == i;
    let row_ = |icon: G, title: &str, sub: Option<E<'static, Msg>>, end: E<'static, Msg>| -> E<'static, Msg> { srow(Some(ri(icon, 40.0)), title, sub, Some(end), 17.0, CARD, pad(14.0, 20.0, 14.0, 20.0), false) };
    let line = |s: &str| -> Option<E<'static, Msg>> { Some(t(s.to_string(), 14.0, MUT).line_height(1.4).into()) };
    let on_off = |on: bool| -> E<'static, Msg> { one(t(if on { "On" } else { "Off" }, 14.0, MUT)).into() };
    let c = a.card.as_ref();
    let conn: E<'static, Msg> = match c {
        Some(c) => row![dotted(if c.usb3() { OK } else { WARN }, 9.0), gap(7.0, 1.0), one(t(format!("Connected on {} · {}", c.usb_words(), c.chip), 14.0, MUT))].align_y(iced::Center).into(),
        None => row![dotted(WARN, 9.0), gap(7.0, 1.0), one(t("No capture card plugged in", 14.0, MUT))].align_y(iced::Center).into(),
    };
    let fps_seg = {
        let items: Vec<(String, bool, Msg)> = [30u32, 50, 60].iter().map(|r| (if *r == 60 { "60 fps".to_string() } else { r.to_string() }, p.fps == *r, Msg::Set("fps", json!(r)))).collect();
        seg(items, 15.0, 7.0, 15.0, f(2))
    };
    let fmt_seg = seg(vec![("MJPEG".into(), p.format == "MJPG", Msg::Set("format", json!("MJPG"))), ("YUYV".into(), p.format == "YUYV", Msg::Set("format", json!("YUYV")))], 15.0, 7.0, 15.0, f(3));
    let mut left = Column::new();
    left = left.push(sec("The card", 6.0));
    left = left.push(row_(G::Card, "Capture card", Some(conn), dd(&c.map(|c| c.name.clone()).unwrap_or_else(|| "None found".into()), 330.0, f(0), Msg::Dd(Some(Dd::Card))))).push(gap(1.0, 8.0));
    left = left.push(row_(G::Tv, "Resolution and frame rate", line("What the card sends this PC. The Xbox's own setting should match."), row![dd(&format!("{} × {}", p.w, p.h), 200.0, f(1), Msg::Dd(Some(Dd::Size))), fps_seg].spacing(10.0).align_y(iced::Center).into())).push(gap(1.0, 8.0));
    left = left.push(row_(G::Film, "Video format", line("MJPEG is lightest on the laptop and the card's best at 1080p60. YUYV is uncompressed: a touch sharper, more work for USB."), fmt_seg)).push(gap(1.0, 8.0));
    left = left.push(row_(G::Bolt, "Low delay", line("Shows each frame the moment it arrives, without smoothing. Best for playing; turn it off if the picture stutters."), row![on_off(p.low_delay), toggle(p.low_delay, 1.0, f(4), Some(Msg::Set("lowDelay", json!(!p.low_delay))))].spacing(10.0).align_y(iced::Center).into()));
    left = left.push(sec("Sound", 20.0));
    left = left.push(row_(G::Mic, "Sound comes from", line("The card's sound shows up as a USB microphone; this is picked for you."), dd(&sound_words(a), 330.0, f(5), Msg::Dd(Some(Dd::Sound))))).push(gap(1.0, 8.0));
    left = left.push(row_(G::Vol, "Play the Xbox's sound", line("Off if you hear the game through headphones on the Xbox controller."), row![on_off(p.play_sound), toggle(p.play_sound, 1.0, f(6), Some(Msg::Set("playSound", json!(!p.play_sound))))].spacing(10.0).align_y(iced::Center).into()));
    left = left.push(sec("Saving", 20.0));
    let folder = |s: String| -> Option<E<'static, Msg>> { Some(one(t(s, 13.5, MUT).font(MONO)).into()) };
    left = left.push(row_(G::Film, "Recordings are saved in", folder(cc::tilde(&a.rec_dir())), row![btn("Open", None, B::Normal, false, 15.0, f(7), Some(Msg::OpenDir(a.rec_dir()))), btn("Change…", None, B::Normal, false, 15.0, f(8), Some(Msg::ChangeDir("recordingsDir")))].spacing(10.0).into())).push(gap(1.0, 8.0));
    left = left.push(row_(G::Cam, "Screenshots are saved in", folder(cc::tilde(&a.shot_dir())), row![btn("Open", None, B::Normal, false, 15.0, f(9), Some(Msg::OpenDir(a.shot_dir()))), btn("Change…", None, B::Normal, false, 15.0, f(10), Some(Msg::ChangeDir("screenshotsDir")))].spacing(10.0).into()));
    let mut right = Column::new();
    right = right.push(sec("When it opens", 6.0));
    right = right.push(row_(G::Full, "Start full screen", line("Straight into the picture when you open it."), toggle(p.full_screen, 1.0, f(11), Some(Msg::Set("startFullScreen", json!(!p.full_screen)))))).push(gap(1.0, 8.0));
    right = right.push(row_(G::Sun, "Keep the screen awake", line("While a picture is showing, the screen doesn't dim or lock. It sleeps as usual when there's no signal."), toggle(p.keep_awake, 1.0, f(12), Some(Msg::Set("keepAwake", json!(!p.keep_awake))))));
    right = right.push(sec("Hearth Store", 20.0));
    let (chip_l, chip_ok) = &a.store_chip;
    let store = column![
        t("Capture Card is a Hearth Store app", 17.0, TX),
        t("It updates through the Store, on its own schedule, apart from Hearth OS updates.", 14.0, MUT).line_height(1.4),
        gap(1.0, 9.0),
        row![chip(chip_l, if *chip_ok { Ch::Ok } else { Ch::On }, None, 13.5), btn("Open in the Store", None, B::Normal, false, 15.0, f(13), Some(Msg::OpenStore))].spacing(10.0).align_y(iced::Center)
    ]
    .spacing(3.0);
    right = right.push(container(row![ri(G::Store, 40.0), store.width(Length::Fill)].spacing(16.0).align_y(iced::Top)).padding(pad(14.0, 20.0, 14.0, 20.0)).width(Length::Fill).style(bx(CARD, 12.0, None))).push(gap(1.0, 8.0));
    right = right.push(container(t("Your choices are saved the moment you change one, and stay as you left them.", 14.0, MUT).line_height(1.4)).padding(pad(14.0, 20.0, 14.0, 20.0)).width(Length::Fill).style(bx(QUIET, 12.0, None)));
    let body = column![h1("Settings"), row![left.width(Length::Fill), right.width(Length::Fixed(560.0))].spacing(28.0)];
    let page = container(scrollable(body)).padding(pad(26.0, 44.0, 26.0, 44.0)).width(Length::Fill).height(Length::Fill);
    frame(a, page.into(), vec![hint(&["A"], "Select", 16.0), hint(&["B"], "Back", 16.0), hint(&["◀▶"], "Change", 16.0)], vec![lbrb("Live view / Recordings / Settings")])
}

// ------------------------------------------------------------------ 4 Recordings & screenshots
fn thumb<'a>(a: &'a App, it: &cc::library::Item, w: f32, h: f32) -> E<'a, Msg> {
    let pic: E<'a, Msg> = match a.thumbs.get(&it.path) {
        Some(hd) => image(hd.clone()).content_fit(iced::ContentFit::Fill).width(Length::Fill).height(Length::Fixed(h)).into(),
        None => Space::new().width(Length::Fill).height(Length::Fixed(h)).into(),
    };
    let _ = w;
    container(pic).width(Length::Fill).height(Length::Fixed(h)).clip(true).style(bx(DEEP, 0.0, None)).into()
}

fn tile<'a>(a: &'a App, i: usize, it: &cc::library::Item, small: bool) -> E<'a, Msg> {
    let sel = i == a.sel;
    let h = if small { 144.0 } else { 206.0 };
    let mut im: Vec<E<'a, Msg>> = vec![thumb(a, it, 0.0, h)];
    if it.kind == Kind::Rec {
        let len = it.info.as_ref().map(|x| cc::length(x.secs)).unwrap_or_else(|| "–:––".into());
        let badge = container(one(t6(len, 13.0, TX))).padding(pad(3.0, 8.0, 3.0, 8.0)).style(bx(Color { r: 0.0, g: 0.0, b: 0.0, a: 0.73 }, 6.0, None));
        im.push(container(badge).align_right(Length::Fill).align_bottom(Length::Fixed(h)).padding(10.0).into());
    }
    let now = cc::now();
    let (b, s) = match (it.kind, small) {
        (Kind::Shot, true) => (it.short(), cc::when(it.mtime, now)),
        (Kind::Shot, false) => (it.name(), cc::when(it.mtime, now)),
        (Kind::Rec, _) => (it.name(), format!("{} · {}", cc::when(it.mtime, now), cc::size(it.size))),
    };
    let cap = if small {
        container(column![one(t6(b, 14.0, TX)), one(t(s, 12.5, MUT))].spacing(2.0)).padding(pad(9.0, 12.0, 9.0, 12.0))
    } else {
        container(column![one(t6(b, 15.5, TX)), one(t(s, 13.5, MUT))].spacing(2.0)).padding(pad(12.0, 14.0, 12.0, 14.0))
    };
    let body = column![stack(im).width(Length::Fill).height(Length::Fixed(h)), cap];
    let tile = container(body)
        .width(Length::Fill)
        .clip(true)
        .style(move |_: &Theme| container::Style { background: Some(iced::Background::Color(CARD)), border: iced::Border { color: if sel { ACC } else { Color::TRANSPARENT }, width: 3.0, radius: 12.0.into() }, ..container::Style::default() });
    click(tile, Some(Msg::Select(i)))
}

fn kv<'a>(k: &str, v: E<'a, Msg>, last: bool) -> E<'a, Msg> {
    let r = container(row![one(t(k.to_string(), 14.0, MUT)), fill(), v].align_y(iced::Center)).padding(pad(8.0, 0.0, 8.0, 0.0));
    if last {
        r.into()
    } else {
        column![r, hline(LINE)].into()
    }
}

fn side<'a>(a: &'a App) -> E<'a, Msg> {
    let Some(it) = a.picked() else {
        let empty = column![one(t6("Nothing here yet", 19.0, TX)), t("Screenshots and recordings you make in Live view show up here.", 14.0, MUT).line_height(1.4)].spacing(6.0);
        return container(empty).padding(20.0).width(Length::Fixed(470.0)).style(bx(SIDE, 14.0, None)).into();
    };
    let prev_h = 252.0;
    let pic: E<'a, Msg> = match a.previews.get(&it.path) {
        Some(hd) => image(hd.clone()).content_fit(iced::ContentFit::Fill).width(Length::Fill).height(Length::Fixed(prev_h)).into(),
        None => Space::new().width(Length::Fill).height(Length::Fixed(prev_h)).into(),
    };
    let mut layers: Vec<E<'a, Msg>> = vec![pic];
    if it.kind == Kind::Rec {
        let play = container(container(glyph(G::Play, TX, 28.0)).padding(pad(0.0, 0.0, 0.0, 5.0))).center_x(Length::Fixed(70.0)).center_y(Length::Fixed(70.0)).style(bx(Color { r: 0.0, g: 0.0, b: 0.0, a: 0.67 }, 35.0, None));
        layers.push(container(click(play, Some(Msg::Play(it.path.clone())))).center_x(Length::Fill).center_y(Length::Fixed(prev_h)).into());
    }
    let preview = container(stack(layers).width(Length::Fill)).width(Length::Fill).height(Length::Fixed(prev_h)).clip(true).style(bx(DEEP, 10.0, None));
    let info = it.info.clone();
    let pic_words = match (&info, it.kind) {
        (Some(x), Kind::Rec) if x.w > 0 => format!("{} × {} · {} fps", x.w, x.h, x.fps),
        (Some(x), Kind::Shot) if x.w > 0 => format!("{} × {} · PNG", x.w, x.h),
        _ => "–".into(),
    };
    let mut rows = Column::new();
    if it.kind == Kind::Rec {
        rows = rows.push(kv("Length", one(t(info.as_ref().map(|x| cc::length(x.secs)).unwrap_or_else(|| "–".into()), 15.0, TX)).into(), false));
    }
    rows = rows.push(kv("Picture", one(t(pic_words, 15.0, TX)).into(), false));
    rows = rows.push(kv("Size", one(t(if it.kind == Kind::Rec { format!("{} · {}", cc::size(it.size), it.ext()) } else { cc::size(it.size) }, 15.0, TX)).into(), false));
    rows = rows.push(kv("Saved in", one(t(cc::tilde(&it.folder()), 13.5, TX).font(MONO)).into(), true));
    let first = if it.kind == Kind::Rec {
        primary_play(it.path.clone())
    } else {
        btn("Copy", None, B::Primary, false, 15.0, false, Some(Msg::CopyFile(it.path.clone())))
    };
    let buttons = column![
        row![first, btn("Show in folder", Some(G::Folder), B::Normal, false, 15.0, false, Some(Msg::ShowFile(it.path.clone()))), btn("Rename", None, B::Normal, false, 15.0, false, Some(Msg::Ask(Dlg::Rename(it.path.clone()))))].spacing(10.0),
        btn("Delete", None, B::Danger, false, 15.0, false, Some(Msg::Ask(Dlg::Delete(it.path.clone()))))
    ]
    .spacing(10.0);
    let c = column![
        preview,
        gap(1.0, 16.0),
        one(t6(it.name(), 19.0, TX)),
        gap(1.0, 4.0),
        one(t(cc::when(it.mtime, cc::now()), 14.0, MUT)),
        gap(1.0, 12.0),
        container(rows).padding(pad(6.0, 16.0, 6.0, 16.0)).width(Length::Fill).style(bx(CARD, 12.0, None)),
        gap(1.0, 14.0),
        buttons,
        gap(1.0, 14.0),
        one(t(format!("{} on this PC", cc::free_words(&it.folder())), 14.0, MUT))
    ];
    container(c).padding(20.0).width(Length::Fixed(470.0)).style(bx(SIDE, 14.0, None)).into()
}

/// "▶ Play": the primary button with the play triangle.
fn primary_play<'a>(p: std::path::PathBuf) -> E<'a, Msg> {
    let r = row![glyph(G::Play, INK, 16.0), one(t7("Play", 15.0, INK))].spacing(6.0).align_y(iced::Center);
    ring(false, 9.0, click(container(r).padding(pad(9.0, 18.0, 9.0, 18.0)).style(bx(ACC, 9.0, Some((ACC, 1.0)))), Some(Msg::Play(p))))
}

fn library(a: &App) -> E<'_, Msg> {
    let (nr, ns) = (a.recs(), a.shots());
    let tabs = seg(
        vec![(format!("Recordings · {nr}"), a.tab == Tab::Rec, Msg::Tab(Tab::Rec)), (format!("Screenshots · {ns}"), a.tab == Tab::Shot, Msg::Tab(Tab::Shot)), ("Everything".into(), a.tab == Tab::All, Msg::Tab(Tab::All))],
        15.0,
        7.0,
        15.0,
        false,
    );
    let sort = click(one(t(if a.prefs.sort == "oldest" { "Oldest first ▾" } else { "Newest first ▾" }, 14.0, MUT)), Some(Msg::Sort));
    let shown = a.shown();
    let small = a.tab == Tab::Shot;
    let cols = if small { 4 } else { 3 };
    let mut grid = Column::new().spacing(if small { 14.0 } else { 18.0 });
    for (r, chunk) in shown.chunks(cols).enumerate() {
        let mut rr = Row::new().spacing(if small { 14.0 } else { 18.0 });
        for (c, it) in chunk.iter().enumerate() {
            rr = rr.push(container(tile(a, r * cols + c, it, small)).width(Length::FillPortion(1)));
        }
        for _ in chunk.len()..cols {
            rr = rr.push(Space::new().width(Length::FillPortion(1)));
        }
        grid = grid.push(rr);
    }
    if shown.is_empty() {
        grid = grid.push(one(t(
            match a.tab {
                Tab::Rec => "No recordings yet. Press Record (or Y on a controller) in Live view.",
                Tab::Shot => "No screenshots yet. Press Screenshot (or X on a controller) in Live view.",
                Tab::All => "Nothing here yet.",
            },
            15.0,
            MUT,
        )));
    }
    let left = column![h1("Recordings & screenshots"), row![tabs, fill(), sort].align_y(iced::Center), gap(1.0, 18.0), scrollable(grid).height(Length::Fill)].width(Length::Fill);
    let page = container(row![left, column![side(a)]].spacing(26.0)).padding(pad(26.0, 34.0, 26.0, 34.0)).width(Length::Fill).height(Length::Fill);
    let rec_sel = a.picked().map(|i| i.kind == Kind::Rec).unwrap_or(a.tab != Tab::Shot);
    frame(
        a,
        page.into(),
        vec![hint(&["A"], if rec_sel { "Play" } else { "Copy" }, 16.0), hint(&["X"], "Delete", 16.0), hint(&["Y"], "Rename", 16.0), hint(&["B"], "Back", 16.0)],
        vec![lbrb("Recordings / Screenshots")],
    )
}

fn dialog<'a>(a: &'a App, d: &Dlg) -> E<'a, Msg> {
    let c = match d {
        Dlg::Rename(p) => column![
            one(t6(if p.extension().map(|e| e == "png").unwrap_or(false) { "Rename this screenshot" } else { "Rename this recording" }, 20.0, TX)),
            text_input("Name", &a.rename_text).id(RENAME_FIELD).on_input(Msg::RenameText).on_submit(Msg::DoRename).size(16.0).padding(pad(10.0, 14.0, 10.0, 14.0)),
            row![fill(), btn("Cancel", None, B::Normal, false, 15.0, false, Some(Msg::CloseDlg)), btn("Rename", None, B::Primary, false, 15.0, true, Some(Msg::DoRename))].spacing(10.0)
        ]
        .spacing(14.0),
        Dlg::Delete(p) => column![
            one(t6(format!("Move “{}” to the Trash?", p.file_stem().unwrap_or_default().to_string_lossy()), 20.0, TX)),
            t("You can take it back from the Trash later.", 15.0, MUT),
            row![fill(), btn("Cancel", None, B::Normal, false, 15.0, false, Some(Msg::CloseDlg)), btn("Delete", None, B::Danger, false, 15.0, true, Some(Msg::DoDelete))].spacing(10.0)
        ]
        .spacing(14.0),
    };
    hearth_ui::ui::dialog(Space::new().into(), container(c).padding(22.0).width(Length::Fixed(520.0)).style(shadowed(SIDE, 14.0, Some((LINE, 1.0)), 40.0)).into(), Some(Msg::CloseDlg))
}

// ------------------------------------------------------------------ 1 the viewer
pub fn picture(a: &App) -> E<'_, Msg> {
    let fit = match a.prefs.fit.as_str() {
        "fill" => iced::ContentFit::Cover,
        "stretch" => iced::ContentFit::Fill,
        _ => iced::ContentFit::Contain,
    };
    let pic: E<'_, Msg> = match a.frame.as_ref() {
        Some(h) => image(h.clone()).content_fit(fit).width(Length::Fill).height(Length::Fill).into(),
        None => Space::new().width(Length::Fill).height(Length::Fill).into(),
    };
    // The black around a picture of another shape is its own layer under the picture, so a panel drawn over both
    // (the quick menu) is never covered by it.
    stack![container(Space::new()).width(Length::Fill).height(Length::Fill).style(bx(Color::BLACK, 0.0, None)), container(pic).center_x(Length::Fill).center_y(Length::Fill)].width(Length::Fill).height(Length::Fill).into()
}

fn src<'a>(b: String, s: String) -> E<'a, Msg> {
    column![one(t6(b, 15.0, TX)), one(t(s, 12.5, MUT))].spacing(1.0).into()
}

fn sep<'a>() -> E<'a, Msg> {
    vline(aw(0.12), 34.0)
}

fn grp<'a>(items: Vec<E<'a, Msg>>, l: f32) -> E<'a, Msg> {
    let mut r = Row::new().spacing(10.0).align_y(iced::Center);
    for x in items {
        r = r.push(x);
    }
    container(r).padding(pad(0.0, 12.0, 0.0, l)).center_y(Length::Fixed(44.0)).into()
}

fn vb<'a>(icon: Option<E<'a, Msg>>, label: Option<String>, bg: Color, fg: Color, ph: f32, m: Msg) -> E<'a, Msg> {
    let mut r = Row::new().spacing(8.0).align_y(iced::Center);
    if let Some(i) = icon {
        r = r.push(i);
    }
    if let Some(l) = label {
        r = r.push(one(t(l, 14.5, fg)));
    }
    click(container(r).padding(pad(0.0, ph, 0.0, ph)).center_y(Length::Fixed(44.0)).style(bx(bg, 10.0, None)), Some(m))
}

pub fn volume_slider<'a>(v: u32, width: f32) -> E<'a, Msg> {
    iced::widget::slider(0..=100u32, v, Msg::Volume)
        .on_release(Msg::VolumeDone)
        .width(Length::Fixed(width))
        .height(17.0)
        .style(|_t: &Theme, _s: iced::widget::slider::Status| iced::widget::slider::Style {
            rail: iced::widget::slider::Rail { backgrounds: (iced::Background::Color(ACC), iced::Background::Color(Color::from_rgb8(0x3a, 0x3f, 0x49))), width: 5.0, border: iced::border::rounded(3.0) },
            handle: iced::widget::slider::Handle { shape: iced::widget::slider::HandleShape::Circle { radius: 8.5 }, background: iced::Background::Color(Color::WHITE), border_width: 0.0, border_color: Color::TRANSPARENT },
        })
        .into()
}

pub fn fit_seg<'a>(a: &App, size: f32, pv: f32, ph: f32) -> E<'a, Msg> {
    let f = a.prefs.fit.as_str();
    seg(vec![("Fit".into(), f == "fit", Msg::Fit("fit")), ("Fill".into(), f == "fill", Msg::Fit("fill")), ("Stretch".into(), f == "stretch", Msg::Fit("stretch"))], size, pv, ph, false)
}

/// The REC pill (1b): the time, the file's size and the free space.
pub fn rec_pill<'a>(a: &'a App, size: f32) -> Option<E<'a, Msg>> {
    let (since, _) = a.rec.as_ref()?;
    let glow = container(dotted(Color::from_rgb8(0xff, 0x3b, 0x3b), 9.0)).style(|_: &Theme| container::Style { shadow: iced::Shadow { color: Color::from_rgb8(0xff, 0x3b, 0x3b), offset: iced::Vector::ZERO, blur_radius: 10.0 }, ..container::Style::default() });
    let free = if a.rec_free.is_empty() { cc::free_words(&a.rec_dir()) } else { a.rec_free.clone() };
    let r = row![glow, one(t7(format!("REC {}", rec_clock(since.elapsed().as_secs())), size, TX)), one(t(format!("{} · {free}", cc::size(a.rec_size.max(1))), 13.5, MUT))].spacing(12.0).align_y(iced::Center);
    Some(container(r).padding(pad(9.0, 15.0, 9.0, 15.0)).style(shadowed(glass(0.91), 12.0, Some((aw(0.11), 1.0)), 40.0)).into())
}

/// "04:12" (the REC pill's clock; "1:02:03" past an hour).
pub fn rec_clock(s: u64) -> String {
    if s >= 3600 {
        cc::length(s)
    } else {
        format!("{:02}:{:02}", s / 60, s % 60)
    }
}

pub fn shot_toast<'a>(a: &'a App, bottom: f32) -> Option<E<'a, Msg>> {
    let (p, h) = a.shot.as_ref()?;
    let thumb = container(image(h.clone()).content_fit(iced::ContentFit::Cover).width(Length::Fixed(112.0)).height(Length::Fixed(63.0))).clip(true).style(bx(Color::BLACK, 7.0, None));
    let shown = p.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
    let dir = p.parent().map(|d| cc::tilde(d)).unwrap_or_default();
    let where_ = if dir == "~/Pictures/Screenshots" { format!("Pictures › Screenshots › {shown}") } else { format!("{} › {shown}", dir.trim_start_matches("~/").replace('/', " › ")) };
    let small = |l: &str, m: Msg| -> E<'a, Msg> { click(container(one(t(l.to_string(), 15.0, SOFT))).padding(pad(7.0, 14.0, 7.0, 14.0)).style(bx(Color::TRANSPARENT, 9.0, Some((EDGE, 1.0)))), Some(m)) };
    let toast = container(
        row![thumb, column![one(t6("Screenshot saved", 15.0, TX)), one(t(where_, 13.0, MUT))].spacing(2.0), gap(10.0, 1.0), small("Show", Msg::ShowFile(p.clone())), small("Copy", Msg::CopyFile(p.clone()))].spacing(12.0).align_y(iced::Center),
    )
    .padding(pad(12.0, 18.0, 12.0, 18.0))
    .style(shadowed(glass(0.91), 12.0, Some((aw(0.11), 1.0)), 40.0));
    Some(container(toast).center_x(Length::Fill).align_bottom(Length::Fill).padding(pad(0.0, 0.0, bottom, 0.0)).into())
}

pub fn viewer(a: &App) -> E<'_, Msg> {
    let mut layers: Vec<E<'_, Msg>> = vec![picture(a)];
    let name = card_name(a);
    let m = a.mode.clone();
    let short = mode_short(a);
    if let Some(p) = rec_pill(a, 15.0) {
        layers.push(container(p).center_x(Length::Fill).padding(pad(26.0, 0.0, 0.0, 0.0)).into());
    } else if a.pill && !a.menu {
        let pill = container(row![dotted(OK, 9.0), column![one(t(format!("{name} · {short}"), 15.0, TX)), one(t("Move the mouse or press any key to show the bar · Esc leaves full screen", 13.0, MUT))].spacing(2.0)].spacing(12.0).align_y(iced::Center))
            .padding(pad(12.0, 18.0, 12.0, 18.0))
            .style(shadowed(glass(0.92 * 0.91), 12.0, Some((aw(0.11), 1.0)), 40.0));
        layers.push(container(pill).center_x(Length::Fill).padding(pad(26.0, 0.0, 0.0, 0.0)).into());
    }
    let bar_on = (a.bar || a.shows("bar")) && !a.menu;
    if bar_on {
        let usb = a.card.as_ref().map(|c| c.usb_words()).unwrap_or("USB");
        let fmt = m.as_ref().map(|m| m.format_words()).unwrap_or("MJPEG");
        let vol_icon = if a.prefs.muted { G::Mute } else { G::Vol };
        let recording = a.rec.is_some();
        let rec_btn = if recording {
            vb(Some(container(Space::new()).width(Length::Fixed(13.0)).height(Length::Fixed(13.0)).style(bx(Color::from_rgb8(0xff, 0xd0, 0xd0), 2.0, None)).into()), Some("Stop".into()), Color::from_rgb8(0x4a, 0x16, 0x16), Color::from_rgb8(0xff, 0xd0, 0xd0), 13.0, Msg::Record)
        } else {
            vb(Some(dotted(REC_RED, 14.0)), Some("Record".into()), Color::TRANSPARENT, VB_TX, 13.0, Msg::Record)
        };
        let tip_on = a.tip || a.shows("tip");
        let full_btn = mouse_area(vb(Some(glyph(if a.full { G::Unfull } else { G::Full }, VB_TX, 21.0)), Some("Full screen".into()), if tip_on { aw(0.08) } else { Color::TRANSPARENT }, VB_TX, 13.0, Msg::FullScreen)).on_enter(Msg::Tip(true)).on_exit(Msg::Tip(false));
        let bar = row![
            grp(vec![glyph(G::Card, ACC2, 24.0), src(name.clone(), format!("{usb} · {fmt}")), container(one(t7(short.clone(), 13.5, SIG_TX))).padding(pad(4.0, 9.0, 4.0, 9.0)).style(bx(SIG_BG, 7.0, None)).into()], 0.0),
            sep(),
            grp(vec![glyph(G::Bolt, ACC2, 18.0), src(delay_words(a), "delay".into()), container(one(t("Low delay", 14.0, Color::from_rgb8(0xdf, 0xe2, 0xe7)))).padding(pad(0.0, 0.0, 0.0, 6.0)).into(), toggle(a.prefs.low_delay, 0.85, false, Some(Msg::Set("lowDelay", json!(!a.prefs.low_delay))))], 12.0),
            sep(),
            grp(vec![vb(Some(glyph(vol_icon, VB_TX, 21.0)), None, Color::TRANSPARENT, VB_TX, 6.0, Msg::Mute), volume_slider(a.prefs.volume, 120.0), container(one(t(a.prefs.volume.to_string(), 14.0, MUT))).width(Length::Fixed(30.0)).into()], 12.0),
            sep(),
            grp(vec![fit_seg(a, 13.5, 6.0, 12.0)], 12.0),
            sep(),
            vb(Some(glyph(G::Cam, VB_TX, 21.0)), Some("Screenshot".into()), Color::TRANSPARENT, VB_TX, 13.0, Msg::Screenshot),
            rec_btn,
            full_btn,
            sep(),
            vb(Some(glyph(G::List, VB_TX, 21.0)), None, Color::TRANSPARENT, VB_TX, 10.0, Msg::Go(Page::Library)),
            vb(Some(glyph(G::Gear, VB_TX, 21.0)), None, Color::TRANSPARENT, VB_TX, 10.0, Msg::Go(Page::Settings))
        ]
        .spacing(6.0)
        .align_y(iced::Center);
        let bar = container(bar).padding(pad(0.0, 10.0, 0.0, 18.0)).center_y(Length::Fixed(68.0)).style(shadowed(glass(0.9), 16.0, Some((aw(0.11), 1.0)), 50.0));
        layers.push(container(bar).center_x(Length::Fill).align_bottom(Length::Fill).padding(pad(0.0, 0.0, 26.0, 0.0)).into());
        if tip_on {
            // Over Full screen (the bar is centred; Full screen sits about 544 px right of the middle, as drawn).
            let words = if a.full { "Leave full screen · F11" } else { "Full screen · F11" };
            let tw = if a.full { 164.0 } else { 128.0 };
            let tip = column![
                container(one(t6(words, 13.5, BG))).padding(pad(6.0, 11.0, 6.0, 11.0)).style(bx(TX, 7.0, None)),
                container(iced::widget::canvas(Arrow).width(Length::Fixed(10.0)).height(Length::Fixed(5.0))).center_x(Length::Fixed(tw))
            ];
            let (w, h) = a.size;
            let x = w / 2.0 + 544.0 - tw / 2.0;
            layers.push(pin(tip).x(x.max(0.0)).y((h - 26.0 - 68.0 - 36.0).max(0.0)).into());
        }
    }
    if let Some(tst) = shot_toast(a, if bar_on { 110.0 } else { 30.0 }) {
        layers.push(tst);
    }
    if let Some(n) = note(a) {
        layers.push(container(n).center_x(Length::Fill).padding(pad(90.0, 0.0, 0.0, 0.0)).into());
    }
    let all: E<'_, Msg> = stack(layers).width(Length::Fill).height(Length::Fill).into();
    if a.full {
        return all;
    }
    // In a window: its title bar above the picture.
    container(column![title_bar(a), all]).style(bx(BG, 0.0, None)).width(Length::Fill).height(Length::Fill).into()
}

/// The tooltip's little arrow, pointing down at the button.
struct Arrow;

impl<M> iced::widget::canvas::Program<M> for Arrow {
    type State = ();
    fn draw(&self, _s: &(), r: &iced::Renderer, _t: &Theme, b: iced::Rectangle, _c: iced::mouse::Cursor) -> Vec<iced::widget::canvas::Geometry> {
        let mut f = iced::widget::canvas::Frame::new(r, b.size());
        let p = iced::widget::canvas::Path::new(|pb| {
            pb.move_to(iced::Point::ORIGIN);
            pb.line_to(iced::Point::new(b.width, 0.0));
            pb.line_to(iced::Point::new(b.width / 2.0, b.height));
            pb.close();
        });
        f.fill(&p, TX);
        vec![f.into_geometry()]
    }
}
