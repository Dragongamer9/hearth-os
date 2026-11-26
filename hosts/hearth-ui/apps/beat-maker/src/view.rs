//! What Hearth Beat Maker looks like (board 18): the title bar with the transport (⏮ ▶ ■ ●), where the song is, its
//! tempo, time and key, CPU and Export…; Instruments and sounds (Find a sound, In this song, Library, the picked
//! instrument's knobs); the Song (tracks, their clips on the bars, Draw / Pick / Cut, the playhead); the Beat (16
//! steps, swing, rows); the Notes (the piano roll, its grid, Fit to the key, Make chords…, How hard); and Export for
//! Hearth's videos. Every part is a block (hearth_ui::blocks), with Beat Maker's own buttons, panes and info (KIT).

use crate::song::{self, Body, Kind};
use crate::{App, Dlg, Mode, Msg};
use hearth_ui::blocks::{bl, PageDef};
use hearth_ui::studio::{tile, Kit};
use hearth_ui::ui::{self, gap, t, t1, tb, ts, G};
use hearth_ui::Pal;
use iced::widget::canvas::{self, Frame, Geometry, Path, Stroke};
use iced::widget::{button, column, container, mouse_area, row, scrollable, text, text_input, Column, Row, Space};
use iced::{mouse, Background, Border, Color, Element, Length, Padding, Pixels, Point, Rectangle, Renderer, Shadow, Size, Theme};

pub type E<'a> = Element<'a, Msg>;

fn hexc(h: u32) -> Color {
    Color::from_rgb8((h >> 16) as u8, (h >> 8) as u8, h as u8)
}
fn pad(t: f32, r: f32, b: f32, l: f32) -> Padding {
    Padding { top: t, right: r, bottom: b, left: l }
}
pub fn thousands(n: usize) -> String {
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

fn mark<'a>(letters: String, from: u32, to: u32, size: f32) -> E<'a> {
    container(tb(letters, size * 0.36, Color::WHITE))
        .center_x(Length::Fixed(size))
        .center_y(Length::Fixed(size))
        .style(move |_: &Theme| container::Style { background: Some(iced::Gradient::Linear(iced::gradient::Linear::new(2.36).add_stop(0.0, hexc(from)).add_stop(1.0, hexc(to))).into()), border: iced::border::rounded(size * 0.3), ..container::Style::default() })
        .into()
}

#[derive(Clone, Copy, PartialEq)]
enum Ck {
    Acc,
    Normal,
}

fn chip<'a>(p: &Pal, label: impl text::IntoFragment<'a>, kind: Ck, fg_over: Option<Color>, on: Option<Msg>) -> E<'a> {
    let p = *p;
    let fg = fg_over.unwrap_or(if kind == Ck::Acc { p.ink } else { p.text });
    let lbl = if kind == Ck::Acc { ts(label, 12.0, fg) } else { t(label, 12.0, fg) };
    button(container(lbl.wrapping(text::Wrapping::None)).center_y(Length::Fixed(28.0)))
        .padding(pad(0.0, 10.0, 0.0, 10.0))
        .on_press_maybe(on)
        .style(move |_: &Theme, st: button::Status| button::Style {
            background: Some(Background::Color(match (kind, matches!(st, button::Status::Hovered)) {
                (Ck::Acc, false) => p.accent,
                (Ck::Acc, true) => Pal::mix(p.accent, Color::WHITE, 0.1),
                (Ck::Normal, false) => p.panel2,
                (Ck::Normal, true) => Pal::mix(p.panel2, p.text, 0.06),
            })),
            text_color: fg,
            border: Border { color: if kind == Ck::Acc { p.accent } else { p.line }, width: 1.0, radius: 8.0.into() },
            shadow: Shadow::default(),
            snap: true,
        })
        .into()
}

fn field<'a>(p: &Pal, s: String, m: Option<Msg>) -> E<'a> {
    let f = container(t1(s, 12.0, p.text)).padding(pad(0.0, 10.0, 0.0, 10.0)).height(Length::Fixed(28.0)).center_y(Length::Fixed(28.0)).style(ui::boxed(p.bg, 8.0, Some((p.line, 1.0))));
    match m {
        Some(m) => mouse_area(f).on_press(m).interaction(mouse::Interaction::Pointer).into(),
        None => f.into(),
    }
}

fn seg<'a>(p: &Pal, parts: Vec<(&'a str, bool, Msg)>) -> E<'a> {
    let p = *p;
    let mut r = Row::new();
    for (label, on, m) in parts {
        r = r.push(
            button(container(if on { ts(label, 12.0, p.ink) } else { t(label, 12.0, p.muted) }).padding(pad(5.0, 9.0, 5.0, 9.0)))
                .padding(0.0)
                .on_press(m)
                .style(move |_: &Theme, st: button::Status| button::Style {
                    background: Some(Background::Color(if on { p.accent } else if matches!(st, button::Status::Hovered) { p.hover() } else { Color::TRANSPARENT })),
                    text_color: p.text,
                    border: Border::default(),
                    shadow: Shadow::default(),
                    snap: true,
                }),
        );
    }
    container(r).clip(true).style(ui::boxed(Color::TRANSPARENT, 8.0, Some((p.line, 1.0)))).into()
}

fn switch<'a>(p: &Pal, on: bool, m: Msg) -> E<'a> {
    let knob = container(Space::new()).width(Length::Fixed(14.0)).height(Length::Fixed(14.0)).style(ui::boxed(Color::WHITE, 7.0, None));
    let inner: E = if on { row![Space::new().width(Length::Fill), knob].into() } else { row![knob, Space::new().width(Length::Fill)].into() };
    mouse_area(container(inner).padding(3.0).width(Length::Fixed(34.0)).height(Length::Fixed(20.0)).style(ui::boxed(if on { p.accent } else { p.line }, 10.0, None))).on_press(m).interaction(mouse::Interaction::Pointer).into()
}

/// A pane (.pn): the panel, a line, its heading bar (40 high, a line under it), its body.
fn pane<'a>(p: &Pal, head: E<'a>, body: E<'a>) -> E<'a> {
    column![container(head).padding(pad(0.0, 14.0, 0.0, 14.0)).height(Length::Fixed(40.0)).center_y(Length::Fixed(40.0)).width(Length::Fill), ui::hline(p.line), body]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn paned<'a>(p: &Pal, inner: E<'a>) -> E<'a> {
    container(inner).width(Length::Fill).height(Length::Fill).clip(true).style(ui::boxed(p.panel, 12.0, Some((p.line, 1.0)))).into()
}

fn ctext(s: impl Into<String>, at: Point, size: f32, c: Color, bold: bool) -> canvas::Text {
    canvas::Text {
        content: s.into(),
        position: at,
        color: c,
        size: Pixels(size),
        font: iced::Font { weight: if bold { iced::font::Weight::Bold } else { iced::font::Weight::Normal }, ..hearth_ui::app::INTER },
        ..canvas::Text::default()
    }
}

// ------------------------------------------------------------------ the title bar
fn title_bar(a: &App) -> E<'_> {
    let p = a.p;
    let sg = &a.song;
    let bar = a.bar_at(a.pos).floor() as i64 + 1;
    let pos = format!("{}:{:04.1} · bar {}", (a.pos / 60.0) as i64, a.pos % 60.0, bar.min(sg.bars as i64));
    let playing = a.playing.is_some();
    let sub = if sg.note.is_empty() { format!("· {}", sg.name) } else { format!("· {} ({})", sg.name, sg.note) };
    let mut left = row![mark("BM".into(), 0xff9be8, 0xb04fc8, 22.0), gap(10.0, 1.0), ts("Hearth Beat Maker", 14.0, p.text), gap(8.0, 1.0), t1(sub, 14.0, p.muted)].align_y(iced::Center);
    if a.describe.open || a.describe.studio_editing() {
        left = left.push(container(hearth_ui::describe::editing::<Msg>(&p)).padding(pad(0.0, 0.0, 0.0, 10.0)));
    }
    let transport = row![
        chip(&p, "⏮", Ck::Normal, None, Some(Msg::ToStart)),
        chip(&p, if playing { "❚❚" } else { "▶" }, Ck::Acc, None, Some(Msg::Play)),
        chip(&p, "■", Ck::Normal, None, Some(Msg::Stop)),
        chip(&p, "●", if a.recording.is_some() { Ck::Acc } else { Ck::Normal }, Some(if a.recording.is_some() { p.ink } else { hexc(0xff6b6b) }), Some(Msg::Record))
    ]
    .spacing(6.0);
    let fields = row![
        field(&p, pos, None),
        mouse_area(field(&p, format!("{} BPM", sg.bpm.round()), Some(Msg::Bpm(1)))).on_right_press(Msg::Bpm(-1)).on_scroll(|d| match d {
            mouse::ScrollDelta::Lines { y, .. } | mouse::ScrollDelta::Pixels { y, .. } => Msg::Bpm(if y > 0.0 { 1 } else { -1 }),
        }),
        field(&p, format!("{} / {}", sg.sig.0, sg.sig.1), Some(Msg::Sig)),
        field(&p, sg.key_name(), Some(Msg::Open(Dlg::Key)))
    ]
    .spacing(6.0);
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
        gap(24.0, 1.0),
        transport,
        gap(12.0, 1.0),
        fields,
        drag(container(Space::new()).width(Length::Fill).height(Length::Fixed(44.0)).into()),
        t1(if a.busy.is_empty() { format!("CPU {}%", a.cpu.round() as i64) } else { a.busy.clone() }, 12.0, p.muted),
        gap(10.0, 1.0),
        chip(&p, "Export…", Ck::Normal, None, Some(Msg::ExportOpen)),
        gap(10.0, 1.0),
        mouse_area(t1("Edit layout", 12.0, p.muted)).on_press(Msg::Describe(hearth_ui::arrange::edit_layout())).interaction(mouse::Interaction::Pointer),
        gap(10.0, 1.0),
        wb(G::Minimize, Msg::Min, false),
        wb(G::Maximize, Msg::Max, false),
        wb(G::Close, Msg::Quit, true),
        gap(10.0, 1.0)
    ]
    .align_y(iced::Center);
    column![container(r).width(Length::Fill).height(Length::Fixed(44.0)), ui::hline(p.line)].into()
}

// ------------------------------------------------------------------ instruments and sounds
fn inst_find(a: &App) -> E<'_> {
    let p = a.p;
    container(
        text_input("⌕ Find a sound", &a.find)
            .on_input(Msg::Find)
            .size(13.0)
            .padding(pad(7.0, 12.0, 7.0, 12.0))
            .style(move |_: &Theme, _st| text_input::Style { background: Background::Color(p.bg), border: Border { color: p.line, width: 1.0, radius: 10.0.into() }, icon: p.muted, placeholder: p.muted, value: p.text, selection: Pal::a(p.accent, 0.35) }),
    )
    .padding(pad(10.0, 10.0, 0.0, 10.0))
    .into()
}

fn inst_list(a: &App) -> E<'_> {
    let p = a.p;
    let mut c = Column::new().push(container(tb("IN THIS SONG", 11.0, p.muted)).padding(pad(14.0, 8.0, 6.0, 14.0)));
    for (i, tr) in a.song.tracks.iter().enumerate() {
        if tr.inst.kind == Kind::Audio {
            continue;
        }
        let on = a.sel_track == i;
        let body = row![mark(tr.inst.mark.clone(), tr.inst.from, tr.inst.to, 28.0), column![t1(tr.inst.name.clone(), 13.0, p.text), t1(tr.inst.sub.clone(), 11.0, p.muted)]].spacing(10.0).align_y(iced::Center);
        c = c.push(mouse_area(container(body).padding(pad(8.0, 12.0, 8.0, 12.0)).width(Length::Fill).style(ui::boxed(if on { hexc(0x3b2a1d) } else { Color::TRANSPARENT }, 8.0, None))).on_press(Msg::PickTrack(i)).interaction(mouse::Interaction::Pointer));
    }
    container(c).padding(pad(0.0, 6.0, 0.0, 6.0)).into()
}

fn library(a: &App) -> E<'_> {
    let p = a.p;
    let mut c = Column::new().push(container(tb("LIBRARY", 11.0, p.muted)).padding(pad(16.0, 8.0, 6.0, 14.0)));
    for (name, n) in a.library() {
        let label = format!("{name} · {n}");
        c = c.push(mouse_area(container(t1(label, 12.0, p.muted)).padding(pad(4.0, 12.0, 4.0, 18.0))).on_press(Msg::Open(Dlg::Library(name))).interaction(mouse::Interaction::Pointer));
    }
    c = c.push(mouse_area(container(t1("LMMS · open this song in LMMS", 12.0, Pal::a(p.accent, 0.9))).padding(pad(8.0, 12.0, 4.0, 18.0))).on_press(Msg::OpenLmms).interaction(mouse::Interaction::Pointer));
    c.into()
}

/// A knob (.knob): the arc of what it's set to, three quarters round; drag up or down to turn it.
struct Knob {
    k: usize,
    v: f32,
    acc: Color,
    track: Color,
    inner: Color,
}

#[derive(Default)]
struct KnobState {
    held: Option<(f32, f32)>,
}

impl canvas::Program<Msg> for Knob {
    type State = KnobState;
    fn update(&self, st: &mut KnobState, ev: &canvas::Event, b: Rectangle, c: mouse::Cursor) -> Option<canvas::Action<Msg>> {
        match ev {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) if c.is_over(b) => {
                st.held = c.position().map(|p| (p.y, self.v));
                Some(canvas::Action::capture())
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { position }) => {
                let (y0, v0) = st.held?;
                Some(canvas::Action::publish(Msg::Knob(self.k, (v0 + (y0 - position.y) / 150.0).clamp(0.0, 1.0))).and_capture())
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) if st.held.is_some() => {
                st.held = None;
                Some(canvas::Action::capture())
            }
            canvas::Event::Mouse(mouse::Event::WheelScrolled { delta }) if c.is_over(b) => {
                let y = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => *y,
                    mouse::ScrollDelta::Pixels { y, .. } => *y / 40.0,
                };
                Some(canvas::Action::publish(Msg::Knob(self.k, (self.v + y * 0.04).clamp(0.0, 1.0))).and_capture())
            }
            _ => None,
        }
    }
    fn draw(&self, _s: &KnobState, r: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        let mut f = Frame::new(r, b.size());
        let c = Point::new(b.width / 2.0, b.height / 2.0);
        let rad = b.width.min(b.height) / 2.0 - 4.0;
        let start = std::f32::consts::PI * 0.75;
        let arc = |to: f32| {
            Path::new(|pb| {
                let n = 32;
                for i in 0..=n {
                    let a = start + (to - start) * i as f32 / n as f32;
                    let p = Point::new(c.x + rad * a.cos(), c.y + rad * a.sin());
                    if i == 0 {
                        pb.move_to(p)
                    } else {
                        pb.line_to(p)
                    }
                }
            })
        };
        let full = start + std::f32::consts::PI * 1.5;
        f.stroke(&arc(full), Stroke::default().with_color(self.track).with_width(7.0));
        f.stroke(&arc(start + std::f32::consts::PI * 1.5 * self.v.max(0.001)), Stroke::default().with_color(self.acc).with_width(7.0));
        f.fill(&Path::circle(c, rad - 6.0), self.inner);
        vec![f.into_geometry()]
    }
    fn mouse_interaction(&self, _s: &KnobState, b: Rectangle, c: mouse::Cursor) -> mouse::Interaction {
        if c.is_over(b) { mouse::Interaction::ResizingVertically } else { mouse::Interaction::default() }
    }
}

fn knobs(a: &App) -> E<'_> {
    let p = a.p;
    let Some(tr) = a.song.tracks.get(a.sel_track) else { return Space::new().into() };
    let mut r = Row::new().spacing(8.0);
    for (k, name) in ["Cutoff", "Warmth", "Attack", "Reverb"].iter().enumerate() {
        r = r.push(column![canvas::Canvas::new(Knob { k, v: tr.inst.knobs[k], acc: p.accent, track: hexc(0x3a322c), inner: p.panel2 }).width(Length::Fixed(46.0)).height(Length::Fixed(46.0)), t1(*name, 11.0, p.muted)].spacing(4.0).align_x(iced::Center).width(Length::Fill));
    }
    column![ui::hline(p.line), container(column![tb(tr.inst.name.to_uppercase(), 11.0, p.muted), r].spacing(10.0)).padding(12.0)].into()
}

// ------------------------------------------------------------------ the song
struct Lanes<'a> {
    a: &'a App,
    name_w: f32,
}

#[derive(Default)]
struct LaneState {
    /// a clip being dragged (Pick): its track, its index, where it was grabbed (bars into it), where it is now
    drag: Option<(usize, usize, f32, f32)>,
}

const HEAD: f32 = 26.0;
const ROW: f32 = 44.0;

impl<'a> Lanes<'a> {
    fn bar_w(&self, w: f32) -> f32 {
        (w - self.name_w) / self.a.song.bars.max(1) as f32
    }
    fn at(&self, b: Rectangle, p: Point) -> (Option<usize>, f32) {
        let bar = ((p.x - self.name_w) / self.bar_w(b.width)).max(0.0);
        let row = if p.y < HEAD { None } else { Some(((p.y - HEAD) / ROW) as usize) };
        (row, bar)
    }
}

impl<'a> canvas::Program<Msg> for Lanes<'a> {
    type State = LaneState;
    fn update(&self, st: &mut LaneState, ev: &canvas::Event, b: Rectangle, c: mouse::Cursor) -> Option<canvas::Action<Msg>> {
        match ev {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let p = c.position_in(b)?;
                if p.x < self.name_w {
                    return None;
                }
                let (row, bar) = self.at(b, p);
                let Some(ti) = row else { return Some(canvas::Action::publish(Msg::Seek(bar)).and_capture()) };
                let t = self.a.song.tracks.get(ti)?;
                if self.a.mode == Mode::Pick {
                    if let Some(ci) = t.clips.iter().position(|cl| bar >= cl.start && bar < cl.start + cl.len) {
                        st.drag = Some((ti, ci, bar - t.clips[ci].start, t.clips[ci].start));
                    }
                }
                Some(canvas::Action::publish(Msg::Lane(ti, bar)).and_capture())
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let (ti, ci, grab, _) = st.drag?;
                let p = c.position_in(b).or_else(|| c.position().map(|q| Point::new(q.x - b.x, q.y - b.y)))?;
                let (_, bar) = self.at(b, p);
                st.drag = Some((ti, ci, grab, (bar - grab).round().max(0.0)));
                Some(canvas::Action::request_redraw())
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                let (ti, ci, _, now) = st.drag.take()?;
                Some(canvas::Action::publish(Msg::MoveClip(ti, ci, now)).and_capture())
            }
            _ => None,
        }
    }
    fn draw(&self, st: &LaneState, r: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        let a = self.a;
        let p = a.p;
        let sg = &a.song;
        let mut f = Frame::new(r, b.size());
        let bw = self.bar_w(b.width);
        let x0 = self.name_w;
        let line = hexc(0x3a322c);
        let rule = hexc(0x2c2622);
        // the header: TRACKS and the bars (every other one numbered)
        f.fill_text(ctext("TRACKS", Point::new(12.0, 7.0), 11.0, p.muted, false));
        for k in 0..sg.bars {
            let x = x0 + k as f32 * bw;
            if k % 2 == 0 {
                f.fill_rectangle(Point::new(x, 0.0), Size::new(1.0, b.height), line);
                f.fill_text(ctext(format!("{}", k + 1), Point::new(x + 4.0, 6.0), 10.0, p.muted, false));
            }
        }
        f.fill_rectangle(Point::new(0.0, HEAD - 1.0), Size::new(b.width, 1.0), rule);
        f.fill_rectangle(Point::new(x0 - 1.0, 0.0), Size::new(1.0, b.height), p.line);
        for (ti, t) in sg.tracks.iter().enumerate() {
            let y = HEAD + ti as f32 * ROW;
            if y > b.height {
                break;
            }
            f.fill_rectangle(Point::new(0.0, y + ROW - 1.0), Size::new(b.width, 1.0), rule);
            // its name: the colour, the name, M S
            f.fill(&Path::rounded_rectangle(Point::new(12.0, y + ROW / 2.0 - 8.0), Size::new(16.0, 16.0), 4.0.into()), Color { a: if t.mute { 0.35 } else { 0.85 }, ..hexc(t.color) });
            f.fill_text(ctext(t.name.clone(), Point::new(36.0, y + ROW / 2.0 - 8.0), 12.5, if t.mute { p.muted } else { p.text }, false));
            for c in t.clips.iter().enumerate().map(|(ci, c)| (ci, c)) {
                let (ci, c) = c;
                let start = match st.drag {
                    Some((dt, dc, _, now)) if dt == ti && dc == ci => now,
                    _ => c.start,
                };
                let x = x0 + start * bw + 2.0;
                let w = (c.len * bw - 4.0).max(4.0);
                let sel = a.sel_clip == Some((ti, ci));
                let col = Color { a: if t.mute { 0.4 } else { 1.0 }, ..hexc(t.color) };
                f.fill(&Path::rounded_rectangle(Point::new(x, y + 6.0), Size::new(w, ROW - 12.0), 6.0.into()), col);
                if sel {
                    f.stroke(&Path::rounded_rectangle(Point::new(x, y + 6.0), Size::new(w, ROW - 12.0), 6.0.into()), Stroke::default().with_color(Color::WHITE).with_width(2.0));
                }
                let label = sg.patterns.get(c.pat).map(|p| p.name.clone()).unwrap_or_default();
                let max_chars = ((w - 10.0) / 6.0).max(0.0) as usize;
                let shown: String = label.chars().take(max_chars).collect();
                f.fill_text(ctext(shown, Point::new(x + 6.0, y + 9.0), 10.0, hexc(0x1b0f06), true));
            }
        }
        // M S beside the names, after the clips (they sit in the names' column)
        for (ti, t) in sg.tracks.iter().enumerate() {
            let y = HEAD + ti as f32 * ROW + ROW / 2.0 - 7.0;
            f.fill_text(ctext("M", Point::new(x0 - 32.0, y), 10.0, if t.mute { p.accent } else { p.muted }, t.mute));
            f.fill_text(ctext("S", Point::new(x0 - 20.0, y), 10.0, if t.solo { p.accent } else { p.muted }, t.solo));
        }
        // the playhead
        let px = x0 + a.bar_at(a.pos) * bw;
        f.fill_rectangle(Point::new(px - 1.0, 0.0), Size::new(2.0, b.height), p.accent);
        f.fill_rectangle(Point::new(px - 3.0, 0.0), Size::new(6.0, b.height), Pal::a(p.accent, 0.18));
        vec![f.into_geometry()]
    }
}

fn song_pane(a: &App) -> E<'_> {
    let p = a.p;
    let sg = &a.song;
    let secs = sg.length_secs();
    let head = row![
        ts("Song", 13.0, p.text),
        t1(format!("· {} bars · {}:{:02}", sg.bars, (secs / 60.0) as i64, (secs % 60.0).round() as i64), 13.0, p.muted),
        Space::new().width(Length::Fill),
        seg(&p, vec![("Draw", a.mode == Mode::Draw, Msg::Mode(Mode::Draw)), ("Pick", a.mode == Mode::Pick, Msg::Mode(Mode::Pick)), ("Cut", a.mode == Mode::Cut, Msg::Mode(Mode::Cut))])
    ]
    .spacing(8.0)
    .align_y(iced::Center);
    // the M and S of each track, as buttons over the canvas's names column
    let mut ms = Column::new().push(Space::new().height(Length::Fixed(HEAD)));
    for (i, _) in sg.tracks.iter().enumerate() {
        ms = ms.push(
            row![
                mouse_area(container(Space::new()).width(Length::Fixed(150.0)).height(Length::Fixed(ROW))).on_press(Msg::PickTrack(i)),
                mouse_area(container(Space::new()).width(Length::Fixed(14.0)).height(Length::Fixed(ROW))).on_press(Msg::Mute(i)).interaction(mouse::Interaction::Pointer),
                mouse_area(container(Space::new()).width(Length::Fixed(14.0)).height(Length::Fixed(ROW))).on_press(Msg::Solo(i)).interaction(mouse::Interaction::Pointer)
            ]
            .padding(pad(0.0, 0.0, 0.0, 0.0)),
        );
    }
    let body = iced::widget::stack![canvas::Canvas::new(Lanes { a, name_w: 200.0 }).width(Length::Fill).height(Length::Fill), ms];
    paned(&p, pane(&p, head.into(), body.into()))
}

// ------------------------------------------------------------------ the beat
struct Steps<'a> {
    rows: &'a [song::Row],
    steps: usize,
    acc: Color,
    cell: Color,
    line: Color,
    text: Color,
}

#[derive(Default)]
struct StepState {
    held: Option<(usize, usize, f32, u8, bool)>,
}

const NAME_W: f32 = 90.0;
const CELL_H: f32 = 30.0;
const GAP: f32 = 5.0;
const ROW_H: f32 = 37.0;

impl<'a> Steps<'a> {
    fn cell_w(&self, w: f32) -> f32 {
        ((w - NAME_W - GAP * (self.steps as f32 - 1.0)) / self.steps as f32).max(4.0)
    }
    fn at(&self, w: f32, p: Point) -> Option<(usize, usize)> {
        if p.x < NAME_W {
            return None;
        }
        let r = (p.y / ROW_H) as usize;
        let i = ((p.x - NAME_W) / (self.cell_w(w) + GAP)) as usize;
        (r < self.rows.len() && i < self.steps).then_some((r, i))
    }
}

impl<'a> canvas::Program<Msg> for Steps<'a> {
    type State = StepState;
    fn update(&self, st: &mut StepState, ev: &canvas::Event, b: Rectangle, c: mouse::Cursor) -> Option<canvas::Action<Msg>> {
        match ev {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let p = c.position_in(b)?;
                let (r, i) = self.at(b.width, p)?;
                let v = self.rows[r].hits.get(i).copied().unwrap_or(0);
                if v == 0 {
                    return Some(canvas::Action::publish(Msg::Step(r, i)).and_capture());
                }
                st.held = Some((r, i, p.y, v, false));
                Some(canvas::Action::capture())
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { position }) => {
                let (r, i, y0, v0, _) = st.held?;
                let y = position.y - b.y;
                if (y - y0).abs() < 3.0 {
                    return None;
                }
                st.held = Some((r, i, y0, v0, true));
                let v = (v0 as f32 + (y0 - y) * 1.5).clamp(1.0, 127.0) as u8;
                Some(canvas::Action::publish(Msg::StepVel(r, i, v)).and_capture())
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                let (r, i, _, _, moved) = st.held.take()?;
                if moved {
                    Some(canvas::Action::capture())
                } else {
                    Some(canvas::Action::publish(Msg::Step(r, i)).and_capture())
                }
            }
            _ => None,
        }
    }
    fn draw(&self, _s: &StepState, r: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        let mut f = Frame::new(r, b.size());
        let cw = self.cell_w(b.width);
        for (ri, row) in self.rows.iter().enumerate() {
            let y = ri as f32 * ROW_H;
            f.fill_text(ctext(row.name.clone(), Point::new(0.0, y + 8.0), 12.0, self.text, false));
            for i in 0..self.steps {
                let x = NAME_W + i as f32 * (cw + GAP);
                let v = row.hits.get(i).copied().unwrap_or(0);
                let alpha = if i % 4 == 0 { 1.0 } else { 0.75 };
                let rect = Path::rounded_rectangle(Point::new(x, y), Size::new(cw, CELL_H), 6.0.into());
                if v > 0 {
                    f.fill(&Path::rounded_rectangle(Point::new(x - 3.0, y - 3.0), Size::new(cw + 6.0, CELL_H + 6.0), 8.0.into()), Color { a: 0.18 * alpha, ..self.acc });
                    let k = 0.55 + 0.45 * (v as f32 / 110.0).min(1.0);
                    f.fill(&rect, Color { a: alpha * k, ..self.acc });
                } else {
                    f.fill(&rect, Color { a: alpha, ..self.cell });
                    f.stroke(&rect, Stroke::default().with_color(Color { a: alpha, ..self.line }).with_width(1.0));
                }
            }
        }
        vec![f.into_geometry()]
    }
}

fn beat_pane(a: &App) -> E<'_> {
    let p = a.p;
    let pat = a.beat_pat.and_then(|i| a.song.patterns.get(i));
    let (name, steps, swing, rows): (String, usize, f32, &[song::Row]) = match pat.map(|x| (&x.name, &x.body)) {
        Some((n, Body::Beat { rows, steps, swing })) => (n.clone(), *steps, *swing, rows.as_slice()),
        _ => (String::new(), 16, 0.0, &[]),
    };
    let head = row![
        ts(if name.is_empty() { "Beat".to_string() } else { format!("Beat · \"{name}\"") }, 13.0, p.text),
        t1(format!("· {steps} steps"), 13.0, p.muted),
        Space::new().width(Length::Fill),
        chip(&p, format!("Swing {}%", swing.round() as i64), Ck::Normal, None, if pat.is_some() { Some(Msg::Swing) } else { None })
    ]
    .spacing(8.0)
    .align_y(iced::Center);
    let grid: E = if rows.is_empty() {
        container(t("Pick a drum clip in the song to edit its beat.", 13.0, p.muted)).padding(14.0).into()
    } else {
        canvas::Canvas::new(Steps { rows, steps, acc: p.accent, cell: p.panel2, line: p.line, text: p.text }).width(Length::Fill).height(Length::Fixed(rows.len() as f32 * ROW_H)).into()
    };
    let foot = row![t1("Click to add a hit · drag up and down for how hard", 12.0, p.muted), Space::new().width(Length::Fill), chip(&p, "+ Row", Ck::Normal, None, if pat.is_some() { Some(Msg::AddRow) } else { None })].align_y(iced::Center);
    let body = column![scrollable(grid).height(Length::Fill), container(foot).padding(pad(6.0, 0.0, 0.0, 0.0))];
    paned(&p, pane(&p, head.into(), container(body).padding(14.0).into()))
}

// ------------------------------------------------------------------ the notes
const KEYS_W: f32 = 70.0;
const ROWS: i32 = 13;

struct Roll<'a> {
    a: &'a App,
    notes: &'a [song::Note],
    beats: f32,
}

#[derive(Default)]
struct RollState {
    /// a note held: its index, how (true: its end, to size it), where it was grabbed (key, beat) and where it is now
    held: Option<(usize, bool, i32, f32, i32, f32)>,
}

impl<'a> Roll<'a> {
    fn row_h(&self, h: f32) -> f32 {
        (h / ROWS as f32).max(12.0)
    }
    fn beat_w(&self, w: f32) -> f32 {
        (w - KEYS_W) / self.beats.max(1.0)
    }
    fn key_at(&self, h: f32, y: f32) -> i32 {
        self.a.roll_top - (y / self.row_h(h)) as i32
    }
    fn beat_at(&self, w: f32, x: f32) -> f32 {
        ((x - KEYS_W) / self.beat_w(w)).max(0.0)
    }
    fn note_at(&self, b: Rectangle, p: Point) -> Option<(usize, bool)> {
        let (bw, rh) = (self.beat_w(b.width), self.row_h(b.height));
        self.notes.iter().enumerate().rev().find_map(|(i, n)| {
            let x = KEYS_W + n.start * bw;
            let y = (self.a.roll_top - n.key) as f32 * rh;
            let w = n.len * bw;
            (p.x >= x && p.x <= x + w && p.y >= y && p.y <= y + rh).then_some((i, p.x > x + w - 8.0))
        })
    }
}

impl<'a> canvas::Program<Msg> for Roll<'a> {
    type State = RollState;
    fn update(&self, st: &mut RollState, ev: &canvas::Event, b: Rectangle, c: mouse::Cursor) -> Option<canvas::Action<Msg>> {
        match ev {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let p = c.position_in(b)?;
                if p.x < KEYS_W {
                    return None;
                }
                match self.note_at(b, p) {
                    Some((i, edge)) => {
                        let n = &self.notes[i];
                        st.held = Some((i, edge, n.key, n.start, self.key_at(b.height, p.y), self.beat_at(b.width, p.x)));
                        Some(canvas::Action::publish(Msg::PickNote(i, false)).and_capture())
                    }
                    None => Some(canvas::Action::publish(Msg::AddNote(self.key_at(b.height, p.y), self.beat_at(b.width, p.x))).and_capture()),
                }
            }
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) => {
                let p = c.position_in(b)?;
                let (i, _) = self.note_at(b, p)?;
                Some(canvas::Action::publish(Msg::DelNote(i)).and_capture())
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { position }) => {
                let (i, edge, k0, s0, gk, gb) = st.held?;
                let p = Point::new(position.x - b.x, position.y - b.y);
                let (k, bt) = (self.key_at(b.height, p.y), self.beat_at(b.width, p.x));
                let n = self.notes.get(i)?;
                let m = if edge { Msg::SizeNote(i, (bt - n.start).max(0.05)) } else { Msg::MoveNote(i, k0 + (k - gk), (s0 + (bt - gb)).max(0.0)) };
                Some(canvas::Action::publish(m).and_capture())
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                st.held.take()?;
                Some(canvas::Action::capture())
            }
            canvas::Event::Mouse(mouse::Event::WheelScrolled { delta }) if c.is_over(b) => {
                let y = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => *y,
                    mouse::ScrollDelta::Pixels { y, .. } => *y / 30.0,
                };
                Some(canvas::Action::publish(Msg::RollScroll(if y > 0.0 { 1 } else { -1 })).and_capture())
            }
            _ => None,
        }
    }
    fn draw(&self, _s: &RollState, r: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        let a = self.a;
        let p = a.p;
        let mut f = Frame::new(r, b.size());
        let (bw, rh) = (self.beat_w(b.width), self.row_h(b.height));
        // the keys
        for i in 0..ROWS {
            let key = a.roll_top - i;
            let pc = key.rem_euclid(12) as usize;
            let black = matches!(pc, 1 | 3 | 6 | 8 | 10);
            let y = i as f32 * rh;
            f.fill_rectangle(Point::new(0.0, y), Size::new(KEYS_W, rh), if black { hexc(0x141110) } else { hexc(0x2a2420) });
            f.fill_rectangle(Point::new(0.0, y + rh - 1.0), Size::new(KEYS_W, 1.0), hexc(0x1a1614));
            let label = if pc == 0 { format!("C{}", key / 12 - 1) } else { song::KEYS[pc].to_string() };
            f.fill_text(canvas::Text { align_x: iced::alignment::Horizontal::Right.into(), ..ctext(label, Point::new(KEYS_W - 8.0, y + rh / 2.0 - 6.0), 10.0, if black { hexc(0x7d726a) } else { hexc(0xd8cfc8) }, false) });
            // the lane, a line under it
            f.fill_rectangle(Point::new(KEYS_W, y + rh - 1.0), Size::new(b.width - KEYS_W, 1.0), hexc(0x2c2622));
        }
        f.fill_rectangle(Point::new(KEYS_W - 1.0, 0.0), Size::new(1.0, b.height), p.line);
        // the grid
        let mut g = 0.0;
        while g <= self.beats + 0.001 {
            let x = KEYS_W + g * bw;
            f.fill_rectangle(Point::new(x, 0.0), Size::new(1.0, b.height), hexc(0x3a322c));
            g += a.grid;
        }
        // the notes: the picked ones lighter
        for (i, n) in self.notes.iter().enumerate() {
            let row = a.roll_top - n.key;
            if !(0..ROWS).contains(&row) {
                continue;
            }
            let x = KEYS_W + n.start * bw + 2.0;
            let y = row as f32 * rh + 3.0;
            let w = (n.len * bw - 4.0).max(3.0);
            let sel = a.sel_notes.contains(&i);
            let col = if sel { hexc(0xffd36b) } else { hexc(0xff8a3d) };
            f.fill(&Path::rounded_rectangle(Point::new(x - 2.0, y - 2.0), Size::new(w + 4.0, rh - 2.0), 5.0.into()), Color { a: 0.18, ..col });
            f.fill(&Path::rounded_rectangle(Point::new(x, y), Size::new(w, rh - 6.0), 4.0.into()), col);
            f.stroke(&Path::rounded_rectangle(Point::new(x, y), Size::new(w, rh - 6.0), 4.0.into()), Stroke::default().with_color(Color::from_rgba8(0x1b, 0x0f, 0x06, 0.33)).with_width(1.0));
        }
        // the playhead, where the song is in this pattern
        if let Some(beat) = roll_playhead(a) {
            let x = KEYS_W + beat * bw;
            f.fill_rectangle(Point::new(x - 1.0, 0.0), Size::new(2.0, b.height), p.accent);
        }
        vec![f.into_geometry()]
    }
}

/// Where the song's playhead is in the pattern open in the piano roll (beats into it), if a clip of it is playing.
fn roll_playhead(a: &App) -> Option<f32> {
    let pat = a.notes_pat?;
    let t = a.song.tracks.get(a.notes_track)?;
    let per_bar = a.song.per_bar();
    let bar = a.bar_at(a.pos);
    let c = t.clips.iter().find(|c| c.pat == pat && bar >= c.start && bar < c.start + c.len)?;
    let pass = a.song.patterns.get(pat)?.beats(per_bar);
    Some(((bar - c.start) * per_bar) % pass)
}

struct Vel<'a> {
    a: &'a App,
    notes: &'a [song::Note],
    beats: f32,
}

#[derive(Default)]
struct VelState {
    held: Option<usize>,
}

impl<'a> Vel<'a> {
    fn bw(&self, w: f32) -> f32 {
        (w - KEYS_W) / self.beats.max(1.0)
    }
    fn hit(&self, w: f32, x: f32) -> Option<usize> {
        let bw = self.bw(w);
        self.notes.iter().position(|n| (x - (KEYS_W + n.start * bw + 7.0)).abs() < 8.0)
    }
}

impl<'a> canvas::Program<Msg> for Vel<'a> {
    type State = VelState;
    fn update(&self, st: &mut VelState, ev: &canvas::Event, b: Rectangle, c: mouse::Cursor) -> Option<canvas::Action<Msg>> {
        let v_at = |y: f32| (((b.height - 6.0 - y) / (b.height - 12.0)) * 127.0).clamp(1.0, 127.0) as u8;
        match ev {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let p = c.position_in(b)?;
                let i = self.hit(b.width, p.x)?;
                st.held = Some(i);
                Some(canvas::Action::publish(Msg::NoteVel(i, v_at(p.y))).and_capture())
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { position }) => {
                let i = st.held?;
                Some(canvas::Action::publish(Msg::NoteVel(i, v_at(position.y - b.y))).and_capture())
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                st.held.take()?;
                Some(canvas::Action::capture())
            }
            _ => None,
        }
    }
    fn draw(&self, _s: &VelState, r: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        let mut f = Frame::new(r, b.size());
        f.fill_text(ctext("HOW HARD", Point::new(10.0, 18.0), 10.0, self.a.p.muted, false));
        let bw = self.bw(b.width);
        for n in self.notes {
            let h = (n.vel as f32 / 127.0) * (b.height - 12.0);
            let x = KEYS_W + n.start * bw + 4.0;
            f.fill(&Path::rounded_rectangle(Point::new(x, b.height - 6.0 - h), Size::new(6.0, h), 3.0.into()), hexc(0xff8a3d));
        }
        vec![f.into_geometry()]
    }
}

fn notes_head(a: &App) -> E<'_> {
    let p = a.p;
    let inst = a.song.tracks.get(a.notes_track).map(|t| t.inst.name.clone()).unwrap_or_default();
    let pat = a.notes_pat.and_then(|i| a.song.patterns.get(i)).map(|x| x.name.clone()).unwrap_or_default();
    let title = if pat.is_empty() { "Notes".to_string() } else { format!("Notes · {inst} · \"{pat}\"") };
    row![
        ts(title, 13.0, p.text),
        Space::new().width(Length::Fill),
        seg(&p, vec![("1/4", a.grid == 1.0, Msg::Grid(1.0)), ("1/8", a.grid == 0.5, Msg::Grid(0.5)), ("1/16", a.grid == 0.25, Msg::Grid(0.25))]),
        chip(&p, format!("Fit to {}{}", a.song.key_name(), if a.fit { " ✓" } else { "" }), if a.fit { Ck::Normal } else { Ck::Normal }, None, Some(Msg::Fit)),
        chip(&p, "Make chords…", Ck::Normal, None, Some(Msg::Open(Dlg::Chords)))
    ]
    .spacing(8.0)
    .align_y(iced::Center)
    .into()
}

fn notes_beats(a: &App) -> f32 {
    let per_bar = a.song.per_bar();
    a.notes_pat.and_then(|i| a.song.patterns.get(i)).map(|p| p.beats(per_bar)).unwrap_or(per_bar * 2.0).clamp(per_bar, 64.0)
}

fn roll(a: &App) -> E<'_> {
    let p = a.p;
    match a.notes() {
        Some(n) => canvas::Canvas::new(Roll { a, notes: n, beats: notes_beats(a) }).width(Length::Fill).height(Length::Fill).into(),
        None => container(t("Pick a clip of notes in the song (Ember Pad's, Glass Lead's…) to edit it here.", 13.0, p.muted)).padding(14.0).into(),
    }
}

fn vel(a: &App) -> E<'_> {
    let p = a.p;
    let lane: E = match a.notes() {
        Some(n) => canvas::Canvas::new(Vel { a, notes: n, beats: notes_beats(a) }).width(Length::Fill).height(Length::Fixed(54.0)).into(),
        None => Space::new().height(Length::Fixed(54.0)).into(),
    };
    column![ui::hline(p.line), lane].into()
}


/// The credit, as the board has it: the app, and what it's built on with its licence.
fn credit<'a>(p: &Pal, app: &'a str, built: &'a str) -> E<'a> {
    container(row![Space::new().width(Length::Fill), t1(format!("{app} · "), 11.0, hexc(0x8f847b)), text(built).size(11.0).color(p.muted).font(ui::weight(iced::font::Weight::Semibold))])
        .padding(pad(4.0, 24.0, 6.0, 0.0))
        .width(Length::Fill)
        .into()
}

// ------------------------------------------------------------------ dialogs and the export box
fn export_box(a: &App) -> Option<E<'_>> {
    let p = a.p;
    let e = a.export.as_ref()?;
    let row_ = |label: &'static str, right: E<'static>| -> E<'static> { container(row![t(label, 13.0, a.p.text), Space::new().width(Length::Fill), right].align_y(iced::Center)).padding(pad(7.0, 0.0, 7.0, 0.0)).into() };
    let name = container(
        text_input("Its name", &e.name)
            .on_input(Msg::ExportName)
            .size(12.0)
            .padding(pad(7.0, 10.0, 7.0, 10.0))
            .style(move |_: &Theme, _st| text_input::Style { background: Background::Color(p.bg), border: Border { color: p.line, width: 1.0, radius: 8.0.into() }, icon: p.muted, placeholder: p.muted, value: p.text, selection: Pal::a(p.accent, 0.35) }),
    )
    .width(Length::Fixed(260.0));
    let files = row![
        chip(&p, ".wav", if e.wav { Ck::Acc } else { Ck::Normal }, None, Some(Msg::ExportToggle("wav"))),
        chip(&p, ".mp3", if e.mp3 { Ck::Acc } else { Ck::Normal }, None, Some(Msg::ExportToggle("mp3"))),
        chip(&p, "stems", if e.stems { Ck::Acc } else { Ck::Normal }, None, Some(Msg::ExportToggle("stems")))
    ]
    .spacing(6.0);
    let head = row![container(Space::new()).width(Length::Fixed(16.0)).height(Length::Fixed(16.0)).style(ui::boxed(p.accent, 4.0, None)), ts("Export for Hearth's videos", 14.0, p.text), Space::new().width(Length::Fill), mouse_area(t("✕", 14.0, p.muted)).on_press(Msg::Close).interaction(mouse::Interaction::Pointer)]
        .spacing(10.0)
        .align_y(iced::Center);
    let body = column![
        row_("Name", name.into()),
        row_("Files", files.into()),
        row_("Add to the hearthos.fun music list", switch(&p, e.site, Msg::ExportToggle("site"))),
        row_("Mark the beats for video cuts", switch(&p, e.beats, Msg::ExportToggle("beats"))),
        container(row![t1("Saved to hearth-workspace", 12.0, p.muted), Space::new().width(Length::Fill), chip(&p, "Cancel", Ck::Normal, None, Some(Msg::Close)), chip(&p, "Export", Ck::Acc, None, Some(Msg::ExportGo))].spacing(8.0).align_y(iced::Center)).padding(pad(12.0, 0.0, 0.0, 0.0))
    ];
    Some(
        container(column![container(head).padding(pad(0.0, 16.0, 0.0, 16.0)).height(Length::Fixed(44.0)).center_y(Length::Fixed(44.0)), ui::hline(p.line), container(body).padding(pad(14.0, 18.0, 14.0, 18.0))])
            .width(Length::Fixed(470.0))
            .style(move |_: &Theme| container::Style {
                background: Some(Background::Color(hexc(0x1d1815))),
                border: Border { color: hexc(0x5a3b22), width: 1.0, radius: 14.0.into() },
                shadow: Shadow { color: Color::from_rgba(0.0, 0.0, 0.0, 0.5), offset: iced::Vector::new(0.0, 20.0), blur_radius: 60.0 },
                ..container::Style::default()
            })
            .into(),
    )
}

fn lib_items(a: &App, cat: &str) -> Vec<(String, String, Msg)> {
    let mut v: Vec<(String, String, Msg)> = Vec::new();
    let q = a.find.to_lowercase();
    let want = |n: &str| q.is_empty() || n.to_lowercase().contains(&q);
    let presets = song::presets();
    if cat == "Synths" || cat == "Drum kits" || cat == "Find" {
        for (i, pr) in presets.iter().enumerate() {
            let drums = pr.kind == Kind::Drums;
            if (cat == "Synths" && drums) || (cat == "Drum kits" && !drums) || !want(&pr.name) {
                continue;
            }
            v.push((pr.name.clone(), format!("{} · add it as a track", pr.sub), Msg::AddInst(i)));
        }
    }
    let walk = |dir: std::path::PathBuf, exts: &[&str], out: &mut Vec<(String, String, Msg)>, limit: usize| {
        let mut stack = vec![dir];
        while let Some(d) = stack.pop() {
            let Ok(rd) = std::fs::read_dir(&d) else { continue };
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().map(|x| exts.contains(&x.to_string_lossy().to_lowercase().as_str())).unwrap_or(false) {
                    let n = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                    if want(&n) && out.len() < limit {
                        let folder = p.parent().and_then(|x| x.file_name()).map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
                        out.push((n, folder, Msg::AddSample(p.display().to_string())));
                    }
                }
            }
        }
    };
    if cat == "Samples" || cat == "Find" {
        walk(crate::lmms_data().join("samples"), &["wav", "ogg", "flac"], &mut v, 200);
    }
    if cat == "My recordings" || cat == "Find" {
        walk(crate::music_dir().join("recordings"), &["wav"], &mut v, 400);
    }
    if cat == "Effects" {
        if let Some(tr) = a.song.tracks.get(a.sel_track) {
            v.push((format!("Reverb · {}", if tr.inst.knobs[3] > 0.05 { "on" } else { "off" }), format!("on {}", tr.inst.name), Msg::Effect("reverb")));
            v.push((format!("Delay · {}", if tr.inst.delay { "on" } else { "off" }), format!("on {}", tr.inst.name), Msg::Effect("delay")));
            v.push((format!("Glue · {}", if tr.inst.glue { "on" } else { "off" }), format!("on {}", tr.inst.name), Msg::Effect("glue")));
        }
    }
    v
}

fn dialog(a: &App) -> Option<E<'_>> {
    let p = a.p;
    let d = a.dlg.as_ref()?;
    let inner: E = match d {
        Dlg::Library(cat) => {
            let mut c = Column::new().spacing(4.0).push(ts(if cat == "Find" { format!("Sounds for \"{}\"", a.find) } else { cat.clone() }, 17.0, p.text));
            let items = lib_items(a, cat);
            if items.is_empty() {
                c = c.push(t(if cat == "Samples" { "No samples yet: LMMS's come with LMMS (Hearth Store › LMMS)." } else { "Nothing here yet." }, 13.0, p.muted));
            }
            let mut list = Column::new().spacing(2.0);
            for (n, sub, m) in items {
                list = list.push(
                    button(container(column![t1(n, 13.0, p.text), t1(sub, 11.0, p.muted)]).padding(pad(6.0, 10.0, 6.0, 10.0)).width(Length::Fill))
                        .width(Length::Fill)
                        .padding(0.0)
                        .on_press(m)
                        .style(move |_: &Theme, st: button::Status| button::Style {
                            background: Some(Background::Color(if matches!(st, button::Status::Hovered) { p.hover() } else { Color::TRANSPARENT })),
                            text_color: p.text,
                            border: iced::border::rounded(8.0),
                            shadow: Shadow::default(),
                            snap: true,
                        }),
                );
            }
            c.push(scrollable(list).height(Length::Fixed(420.0))).push(chip(&p, "Close", Ck::Normal, None, Some(Msg::Close))).into()
        }
        Dlg::Key => {
            let mut c = Column::new().spacing(8.0).push(ts("The song's key", 17.0, p.text));
            for minor in [true, false] {
                let mut r = Row::new().spacing(4.0);
                for k in 0..12u8 {
                    let on = a.song.key == k && a.song.minor == minor;
                    r = r.push(chip(&p, format!("{}{}", song::KEYS[k as usize], if minor { "m" } else { "" }), if on { Ck::Acc } else { Ck::Normal }, None, Some(Msg::Key(k, minor))));
                }
                c = c.push(r);
            }
            c.push(chip(&p, "Close", Ck::Normal, None, Some(Msg::Close))).into()
        }
        Dlg::Chords => column![
            ts("Make chords", 17.0, p.text),
            t(format!("Each picked note (or every note, when none is picked) gets the notes of its chord in {}.", a.song.key_name()), 13.0, p.muted),
            row![chip(&p, "Triads", Ck::Acc, None, Some(Msg::MakeChords("triad"))), chip(&p, "Sevenths", Ck::Normal, None, Some(Msg::MakeChords("seventh"))), chip(&p, "Cancel", Ck::Normal, None, Some(Msg::Close))].spacing(8.0)
        ]
        .spacing(12.0)
        .into(),
        Dlg::Said(h, s) => column![ts(h.clone(), 17.0, p.text), t(s.clone(), 13.0, p.text), chip(&p, "OK", Ck::Acc, None, Some(Msg::Close))].spacing(12.0).into(),
        Dlg::NoLmms => column![
            ts("LMMS isn't installed", 17.0, p.text),
            t("Hearth Beat Maker is built on LMMS: it saves your song as an LMMS project. Install LMMS from Flathub (for you only) to open it there.", 13.0, p.text),
            row![chip(&p, "Install LMMS", Ck::Acc, None, Some(Msg::InstallLmms)), chip(&p, "Cancel", Ck::Normal, None, Some(Msg::Close))].spacing(8.0)
        ]
        .spacing(12.0)
        .into(),
    };
    Some(ui::dialog_box(&p, 560.0, inner))
}

// ------------------------------------------------------------------ blocks
pub const BLOCKS: &[PageDef] = &[
    PageDef { id: "window", title: "Hearth Beat Maker", blocks: &[bl("title", "The title bar", 0, 0, 12, 1), bl("inst", "Instruments and sounds", 0, 1, 2, 10), bl("song", "Song", 2, 1, 6, 4), bl("beat", "Beat", 8, 1, 4, 4), bl("notes", "Notes", 2, 5, 10, 6), bl("credit", "Built on LMMS", 0, 11, 12, 1)] },
    PageDef { id: "inst", title: "Instruments and sounds", blocks: &[bl("find", "Find a sound", 0, 0, 12, 1), bl("song", "In this song", 0, 1, 12, 5), bl("library", "Library", 0, 6, 12, 4), bl("knobs", "The instrument's knobs", 0, 10, 12, 2)] },
    PageDef { id: "notes", title: "Notes", blocks: &[bl("head", "Its name and tools", 0, 0, 12, 1), bl("roll", "The piano roll", 0, 1, 12, 9), bl("vel", "How hard", 0, 10, 12, 2)] },
];

pub const KIT: Kit = Kit {
    name: "Beat Maker",
    buttons: &[
        tile("play", "Play or pause", G::Play),
        tile("stop", "Stop", G::Stop),
        tile("start", "Back to the start", G::Restore),
        tile("record", "Record", G::Camera),
        tile("export", "Export…", G::Down),
        tile("lmms", "Open in LMMS", G::Link),
        tile("draw", "Draw", G::Pencil),
        tile("pick", "Pick", G::Arrow),
        tile("cut", "Cut", G::Close),
        tile("chords", "Make chords", G::Plus),
        tile("fit", "Fit to the key", G::Check),
    ],
    panes: &[tile("beat", "The beat", G::Grid), tile("knobs", "The instrument's knobs", G::Cog), tile("library", "Library", G::List)],
    info: &[tile("where", "Where the song is", G::Clock), tile("tempo", "Tempo", G::Pulse), tile("key", "Key", G::Asterisk), tile("length", "Length", G::Clock)],
    pane_page: "window",
    bars: &[],
    menus: &[],
};

pub fn block_action(id: &str) -> Option<Msg> {
    Some(match id {
        "play" => Msg::Play,
        "stop" => Msg::Stop,
        "start" => Msg::ToStart,
        "record" => Msg::Record,
        "export" => Msg::ExportOpen,
        "lmms" => Msg::OpenLmms,
        "draw" => Msg::Mode(Mode::Draw),
        "pick" => Msg::Mode(Mode::Pick),
        "cut" => Msg::Mode(Mode::Cut),
        "chords" => Msg::Open(Dlg::Chords),
        "fit" => Msg::Fit,
        _ => return None,
    })
}

pub fn view(a: &App) -> E<'_> {
    let p = a.p;
    let secs = a.song.length_secs();
    a.describe.values(vec![
        ("where", format!("bar {}", a.bar_at(a.pos).floor() as i64 + 1)),
        ("tempo", format!("{} BPM", a.song.bpm.round())),
        ("key", a.song.key_name()),
        ("length", format!("{}:{:02}", (secs / 60.0) as i64, (secs % 60.0).round() as i64)),
    ]);
    let inst = hearth_ui::arrange::parts(&a.describe, &p, "inst", vec![("find", inst_find(a)), ("song", inst_list(a)), ("library", library(a)), ("knobs", knobs(a))], |ps| column![ps.take("find"), scrollable(column![ps.take("song"), ps.take("library")]).height(Length::Fill), ps.take("knobs")].into(), Msg::Describe);
    let inst = paned(&p, pane(&p, row![ts("Instruments and sounds", 13.0, p.text), Space::new().width(Length::Fill), mouse_area(t("+", 14.0, p.muted)).on_press(Msg::Open(Dlg::Library("Synths".into()))).interaction(mouse::Interaction::Pointer)].align_y(iced::Center).into(), inst));
    let notes = hearth_ui::arrange::parts(&a.describe, &p, "notes", vec![("head", notes_head(a)), ("roll", roll(a)), ("vel", vel(a))], |ps| {
        column![container(ps.take("head")).padding(pad(0.0, 14.0, 0.0, 14.0)).height(Length::Fixed(40.0)).center_y(Length::Fixed(40.0)), ui::hline(p.line), ps.take("roll"), ps.take("vel")].into()
    }, Msg::Describe);
    let notes = paned(&p, notes);
    let mut items: Vec<(&'static str, E)> = vec![("title", hearth_ui::studio::made(&a.describe, "window", "title", || title_bar(a))), ("inst", inst), ("song", song_pane(a)), ("beat", beat_pane(a)), ("notes", notes), ("credit", credit(&p, "Hearth Beat Maker", "Built on LMMS (GPL-2)"))];
    for (id, w) in [("pane:beat", "beat"), ("pane:knobs", "knobs"), ("pane:library", "library")] {
        if a.describe.wants_pane(w) {
            items.push((id, match w {
                "beat" => beat_pane(a),
                "knobs" => knobs(a),
                _ => library(a),
            }));
        }
    }
    let body = hearth_ui::arrange::parts(
        &a.describe,
        &p,
        "window",
        items,
        |ps| {
            let top = row![container(ps.take("song")).width(Length::Fill), container(ps.take("beat")).width(Length::Fixed(470.0))].spacing(12.0).height(Length::Fixed(380.0));
            let right = column![top, ps.take("notes")].spacing(12.0);
            column![ps.take("title"), container(row![container(ps.take("inst")).width(Length::Fixed(270.0)), right].spacing(12.0)).padding(pad(12.0, 12.0, 2.0, 12.0)).height(Length::Fill), ps.take("credit")].into()
        },
        Msg::Describe,
    );
    let mut base: E = container(body).width(Length::Fill).height(Length::Fill).style(ui::boxed(Color { a: 0.97, ..hexc(0x1a1614) }, p.window_corner, Some((p.line, 1.0)))).into();
    if let Some(ex) = export_box(a) {
        base = iced::widget::stack![base, container(ex).width(Length::Fill).height(Length::Fill).align_right(Length::Fill).align_bottom(Length::Fill).padding(pad(0.0, 20.0, 24.0, 0.0))].into();
    }
    if !a.said.is_empty() {
        base = iced::widget::stack![base, container(container(t1(a.said.clone(), 12.0, p.text)).padding(pad(6.0, 12.0, 6.0, 12.0)).style(ui::boxed(p.panel2, 8.0, Some((p.line, 1.0))))).width(Length::Fill).height(Length::Fill).align_bottom(Length::Fill).center_x(Length::Fill).padding(pad(0.0, 0.0, 6.0, 0.0))].into();
    }
    let base = match dialog(a) {
        Some(d) => ui::dialog(base, d, Some(Msg::Close)),
        None => base,
    };
    ui::edges(base, Msg::Resize)
}
