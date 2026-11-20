//! Hearth's on-screen keyboard inside the Open / Save window (Game Mode): Mantel's OskBoard.qml / OskKey.qml, drawing
//! what the shared engine says (ui/osk/hearth-osk.js, run as it is by apps/pad/src/osk.rs). The same drawing as
//! hearth-pad-host's keyboard (apps/pad/src/view.rs), here inside the window instead of an overlay.

use crate::app::{App, Msg};
use crate::looks::Look;
use crate::osk::{Key, Vm, MOD_KEYS};
use hearth_ui::ui::{self, t, tb};
use hearth_ui::Pal;
use iced::widget::canvas::{self, Frame, Geometry, Path, Stroke};
use iced::widget::{column, container, mouse_area, row, text, Space, Stack};
use iced::{font, mouse, Background, Border, Color, Element, Length, Padding, Point, Rectangle, Renderer, Shadow, Size, Theme, Vector};

fn at<'a>(x: f32, y: f32, e: Element<'a, Msg>) -> Element<'a, Msg> {
    container(e).padding(Padding { top: y.max(0.0), left: x.max(0.0), right: 0.0, bottom: 0.0 }).into()
}
fn sized<'a>(w: f32, h: f32, e: Element<'a, Msg>) -> Element<'a, Msg> {
    container(e).center_x(Length::Fixed(w)).center_y(Length::Fixed(h)).into()
}
fn wt(w: font::Weight) -> iced::Font {
    ui::weight(w)
}

// ------------------------------------------------------------------ the on-screen keyboard (OskBoard.qml)

/// The sizes OskBoard.qml works out from the board's height and the size setting.
pub struct Metrics {
    pub gap: f32,
    pub top_h: f32,
    pub key_h: f32,
    pub font: f32,
}

pub fn metrics(vm: &Vm, board_h: f32) -> Metrics {
    let size = vm.size.as_str();
    let rows = vm.rows.len().max(1) as f32;
    let gap = match size { "s" => 5.0, "l" => 8.0, _ => 7.0 };
    let top_h = ((match size { "s" => 46.0, "l" => 72.0, _ => 58.0 }) * 0.66f32).round();
    let max_k = match size { "s" => 54.0, "l" => 86.0, _ => 72.0 };
    let key_h = ((board_h - 26.0 - top_h - gap * rows) / rows).min(max_k).max(30.0).round();
    Metrics { gap, top_h, key_h, font: (key_h * 0.36).round() }
}

fn key_radius(look: &Look, vm: &Vm, h: f32) -> f32 {
    match look.kr {
        Some(r) if r < 0.0 => h / 2.0,
        Some(r) => r,
        None => match vm.size.as_str() { "s" => 9.0, "l" => 13.0, _ => 11.0 },
    }
}

/// Where each key is in the keys' area (x, width), as OskBoard.qml lays the rows out.
pub fn key_spans(vm: &Vm, iw: f32, gap: f32) -> Vec<Vec<(f32, f32)>> {
    let lay = |keys: &[&Key], x0: f32, width: f32| -> Vec<(i64, f32, f32)> {
        let total: f32 = keys.iter().map(|k| k.w).sum::<f32>().max(0.01);
        let room = (width - gap * (keys.len().saturating_sub(1)) as f32).max(0.0);
        let mut x = x0;
        keys.iter()
            .map(|k| {
                let kw = room * k.w / total;
                let o = (k.col, x, kw);
                x += kw + gap;
                o
            })
            .collect()
    };
    vm.rows
        .iter()
        .map(|r| {
            let mut out = vec![(0.0, 0.0); r.len()];
            let placed: Vec<(i64, f32, f32)> = if vm.split_shape() {
                let l: Vec<&Key> = r.iter().filter(|k| k.half == "l").collect();
                let rr: Vec<&Key> = r.iter().filter(|k| k.half == "r").collect();
                let mut v = lay(&l, 0.0, iw * 0.38);
                v.extend(lay(&rr, iw * 0.62, iw * 0.38));
                v
            } else {
                let all: Vec<&Key> = r.iter().collect();
                lay(&all, 0.0, iw)
            };
            for (c, x, w) in placed {
                if c >= 0 && (c as usize) < out.len() {
                    out[c as usize] = (x, w);
                }
            }
            out
        })
        .collect()
}

fn key_view<'a>(vm: &Vm, look: &Look, k: &Key, r: usize, c: usize, m: &Metrics) -> Element<'a, Msg> {
    let th = *look;
    let modk = MOD_KEYS.contains(&k.t.as_str());
    let ember = vm.active && !vm.wheel.as_ref().map(|w| w.0).unwrap_or(false);
    let sel = vm.active && vm.sel == (r as i64, c as i64);
    let aim_l = vm.split.as_ref().and_then(|s| s.0) == Some((r as i64, c as i64));
    let aim_r = vm.split.as_ref().and_then(|s| s.1) == Some((r as i64, c as i64));
    let lit = sel || aim_l || aim_r;
    let none = k.t == "none" || k.emoji;
    let mut bg = if none {
        Color::TRANSPARENT
    } else if lit {
        if k.go { th.accent } else { th.sel }
    } else if k.go || k.lock {
        th.accent
    } else if k.on {
        Pal::mix(th.key, th.accent, 0.38)
    } else if modk {
        th.modk
    } else {
        th.key
    };
    let mut fg = if lit && !k.go {
        th.sel_text
    } else if k.go || k.lock {
        th.ink
    } else if modk || k.t == "none" {
        th.muted
    } else {
        th.text
    };
    if k.off {
        bg.a *= 0.35;
        fg.a *= 0.35;
    }
    let outlined = th.kb > 0.0 && k.t != "none" && !k.emoji && !lit;
    let (bw, bc) = if aim_r {
        (3.0, Color::from_rgb8(0xff, 0xd3, 0x6b))
    } else if aim_l {
        (3.0, th.accent)
    } else if sel && ember {
        (3.0, th.accent)
    } else if outlined {
        (th.kb, if modk { th.mbc.unwrap_or(th.kbc) } else { th.kbc })
    } else {
        (0.0, Color::TRANSPARENT)
    };
    let size = th.fs
        * if k.t == "space" || k.t == "layout" {
            12.0
        } else if k.emoji {
            m.font * 1.25
        } else if k.clip {
            m.font * 0.8
        } else if modk {
            m.font * 0.85
        } else {
            m.font
        };
    let radius = key_radius(look, vm, m.key_h);
    let label = text(vm.label(k)).size(size.round()).color(fg).font(wt(th.fw)).wrapping(text::Wrapping::None);
    let mut inner: Vec<Element<'a, Msg>> = vec![container(label).center_x(Length::Fill).center_y(Length::Fill).clip(true).into()];
    let g = vm.glyph(k);
    if !g.is_empty() {
        let gc = Pal::a(if lit { th.sel_text } else { th.muted }, 0.6);
        inner.push(container(text(g).size(10.0).color(gc).font(wt(font::Weight::Black))).align_right(Length::Fill).padding(Padding { top: 4.0, right: 6.0, ..Padding::ZERO }).into());
    }
    let glow = sel && ember;
    let accent = th.accent;
    let body = container(Stack::with_children(inner).width(Length::Fill).height(Length::Fill))
        .width(Length::Fill)
        .height(Length::Fixed(m.key_h))
        .style(move |_: &Theme| container::Style {
            text_color: None,
            background: Some(Background::Color(bg)),
            border: Border { color: bc, width: bw, radius: radius.into() },
            shadow: if glow { Shadow { color: Pal::a(accent, 0.35), offset: Vector::new(0.0, 0.0), blur_radius: 14.0 } } else { Shadow::default() },
            snap: true,
        });
    if k.t == "none" {
        return body.into();
    }
    mouse_area(body).on_press(Msg::Pad(crate::pad::PMsg::Key(r, c))).interaction(mouse::Interaction::Pointer).into()
}

struct Glow {
    c: Color,
    r: f32,
}

impl canvas::Program<Msg> for Glow {
    type State = ();
    fn draw(&self, _s: &(), renderer: &Renderer, _t: &Theme, bounds: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        let mut f = Frame::new(renderer, bounds.size());
        let top = bounds.height * 0.3;
        let path = Path::rounded_rectangle(Point::new(0.0, top), Size::new(bounds.width, bounds.height - top), self.r.into());
        let g = canvas::gradient::Linear::new(Point::new(0.0, top), Point::new(0.0, bounds.height)).add_stop(0.0, Pal::a(self.c, 0.0)).add_stop(1.0, self.c);
        f.fill(&path, g);
        vec![f.into_geometry()]
    }
}

/// The Ember Wheel's coals (OskBoard.qml): eight around the hearth, the tilted one bigger and lit.
struct Wheel {
    look: Look,
    petal: i64,
    n: usize,
}

impl canvas::Program<Msg> for Wheel {
    type State = ();
    fn draw(&self, _s: &(), renderer: &Renderer, _t: &Theme, bounds: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        let mut f = Frame::new(renderer, bounds.size());
        let w = bounds.width.min(bounds.height);
        let c = Point::new(w / 2.0, w / 2.0);
        let th = self.look;
        f.fill(&Path::circle(c, w / 2.0), Pal::a(th.bg, 0.9));
        let coal = Path::circle(c, w * 0.11);
        let g = canvas::gradient::Linear::new(Point::new(c.x, c.y - w * 0.11), Point::new(c.x, c.y + w * 0.11))
            .add_stop(0.0, Color::from_rgb8(0xff, 0xd9, 0xa8))
            .add_stop(0.5, th.accent)
            .add_stop(1.0, Color::from_rgb8(0x6a, 0x2a, 0x0a));
        f.fill(&coal, g);
        for i in 0..self.n {
            let on = self.petal == i as i64;
            let ang = i as f32 * std::f32::consts::PI / 4.0;
            let d = w * 0.26 * if on { 1.45 } else { 1.0 };
            let pc = Point::new(c.x + ang.sin() * w * 0.36, c.y - ang.cos() * w * 0.36);
            let path = Path::circle(pc, d / 2.0);
            f.fill(&path, if on { th.sel } else { th.key });
            f.stroke(&path, Stroke::default().with_color(if on { th.accent } else { Pal::a(Color::WHITE, 0.07) }).with_width(if on { 3.0 } else { 1.0 }));
        }
        vec![f.into_geometry()]
    }
}

pub fn keyboard<'a>(a: &'a App, size: Size) -> Element<'a, Msg> {
    let Some(vm) = a.pad.vm.as_ref() else {
        let p = a.p;
        let why = if a.pad.err.is_empty() { "Opening the keyboard…".to_string() } else { format!("The keyboard couldn't start: {}", a.pad.err) };
        return container(t(why, 14.0, p.text)).center_x(Length::Fill).center_y(Length::Fill).style(ui::boxed(p.panel, 12.0, Some((p.line, 1.0)))).into();
    };
    let Some(th) = a.pad.look else { return Space::new().into() };
    let top_pos = vm.position == "top";
    let board_w = (size.width - 16.0).max(100.0);
    let board_h = (size.height - 6.0).max(60.0);
    let m = metrics(vm, board_h);
    let iw = board_w - 24.0;

    // the strip: word suggestions, or what the controller does
    let mut strip = row![].spacing(6.0).align_y(iced::Center).height(Length::Fixed(m.top_h));
    if vm.sugg.is_empty() {
        let hint = if !vm.hints {
            ""
        } else if vm.fast == "wheel" {
            "Left stick: Ember Wheel · A B X Y type its letters · right stick moves the pointer, RT clicks"
        } else if vm.fast == "split" {
            "Point with both sticks · LT / RT type"
        } else {
            "A type · X ⌫ · Y space · LT ⇧ · LB RB move · ☰ Enter · B hide · hold A for accents · RT clicks"
        };
        strip = strip.push(container(t(hint, 12.0, th.muted).wrapping(text::Wrapping::None)).width(Length::Fill).clip(true));
    } else {
        for (i, s) in vm.sugg.iter().enumerate() {
            let sel = vm.active && vm.sel == (-1, i as i64);
            let fg = if sel { th.sel_text } else if i == 0 { th.accent } else { th.text };
            let h = m.top_h;
            let (bg, acc) = (if sel { th.sel } else { th.bg2 }, th.accent);
            let pill = container(text(s.clone()).size((m.font * 0.9).round()).color(fg).font(wt(font::Weight::Semibold)).wrapping(text::Wrapping::None))
                .center_x(Length::Fill)
                .center_y(Length::Fixed(h))
                .clip(true)
                .style(move |_: &Theme| container::Style {
                    background: Some(Background::Color(bg)),
                    border: Border { color: acc, width: if sel { 3.0 } else { 0.0 }, radius: (h / 2.0).into() },
                    ..container::Style::default()
                });
            strip = strip.push(mouse_area(pill).on_press(Msg::Pad(crate::pad::PMsg::Sugg(i))).interaction(mouse::Interaction::Pointer));
        }
    }
    let badge = match vm.panel.as_str() {
        "emoji" => "Emoji".to_string(),
        "clip" => "Clipboard".to_string(),
        _ => vm.layout_short.clone(),
    };
    strip = strip.push(container(tb(badge, 12.0, th.muted)).padding(Padding { top: 0.0, bottom: 0.0, left: 8.0, right: 8.0 }).center_y(Length::Fixed(22.0)).style(ui::boxed(th.bg2, 6.0, None)));

    // the keys
    let mut keys = column![].spacing(m.gap);
    for (r, line) in vm.rows.iter().enumerate() {
        let portion = |k: &Key| Length::FillPortion(((k.w * 100.0).round() as u16).max(1));
        if vm.split_shape() {
            let half = |side: &str| -> Element<'a, Msg> {
                let mut rw = row![].spacing(m.gap);
                for k in line.iter().filter(|k| k.half == side) {
                    let c = k.col.max(0) as usize;
                    rw = rw.push(container(key_view(vm, &th, k, r, c, &m)).width(portion(k)));
                }
                rw.width(Length::FillPortion(38)).into()
            };
            keys = keys.push(row![half("l"), Space::new().width(Length::FillPortion(24)), half("r")].height(Length::Fixed(m.key_h)));
        } else {
            let mut rw = row![].spacing(m.gap).height(Length::Fixed(m.key_h));
            for (c, k) in line.iter().enumerate() {
                rw = rw.push(container(key_view(vm, &th, k, r, c, &m)).width(portion(k)));
            }
            keys = keys.push(rw);
        }
    }
    let content = column![strip, keys].spacing(m.gap).padding(Padding { top: 8.0, right: 12.0, bottom: 12.0, left: 12.0 });

    let mut layers: Vec<Element<'a, Msg>> = Vec::new();
    if let Some(g) = th.glow {
        layers.push(canvas::Canvas::new(Glow { c: g, r: th.br }).width(Length::Fill).height(Length::Fill).into());
    }
    layers.push(content.into());
    // where the keys start on the board
    let (kx, ky) = (12.0, 8.0 + m.top_h + m.gap);
    let spans = key_spans(vm, iw, m.gap);
    let sel_rect = || -> Option<(f32, f32, f32)> {
        let (r, c) = vm.sel;
        if r < 0 {
            return None;
        }
        let (x, w) = *spans.get(r as usize)?.get(c as usize)?;
        Some((x, r as f32 * (m.key_h + m.gap), w))
    };
    // accents (hold A, View, or a long touch)
    if let (Some((alts, pi)), Some((sx, sy, sw))) = (vm.popup.as_ref(), sel_rect()) {
        let pw = iw.min((m.key_h + 6.0) * alts.len() as f32 + 12.0);
        let ph = m.key_h + 12.0;
        let x = (sx + sw / 2.0 - pw / 2.0).min(iw - pw).max(0.0);
        let y = (sy - ph - 6.0).max(-ph / 2.0);
        let mut r = row![].spacing(6.0);
        for (i, s) in alts.iter().enumerate() {
            let sel = *pi == i as i64;
            let (bg, fg, acc) = (if sel { th.sel } else { th.key }, if sel { th.sel_text } else { th.text }, th.accent);
            let cell = container(text(s.clone()).size(m.font).color(fg).font(wt(font::Weight::Semibold)))
                .center_x(Length::Fixed(m.key_h))
                .center_y(Length::Fixed(m.key_h))
                .style(move |_: &Theme| container::Style {
                    background: Some(Background::Color(bg)),
                    border: Border { color: acc, width: if sel { 3.0 } else { 0.0 }, radius: 10.0.into() },
                    ..container::Style::default()
                });
            r = r.push(mouse_area(cell).on_press(Msg::Pad(crate::pad::PMsg::Alt(i))).interaction(mouse::Interaction::Pointer));
        }
        let pop = container(r).padding(6.0).width(Length::Fixed(pw)).height(Length::Fixed(ph)).style(ui::boxed(th.bg2, 14.0, Some((Pal::a(Color::WHITE, 0.08), 1.0))));
        layers.push(at(kx + x, ky + y, pop.into()));
    }
    // the Ember Wheel: eight coals around the hearth; the tilted one glows with its letters on Y X B A
    if let Some((true, petal, petals)) = vm.wheel.as_ref() {
        let area_h = board_h - ky - 12.0;
        let wd = 300.0f32.min(area_h * 1.1);
        let (wx, wy) = (kx + (iw - wd) / 2.0, ky + (area_h - wd) / 2.0);
        layers.push(at(wx, wy, canvas::Canvas::new(Wheel { look: th, petal: *petal, n: petals.len() }).width(Length::Fixed(wd)).height(Length::Fixed(wd)).into()));
        for (i, pstr) in petals.iter().enumerate() {
            let on = *petal == i as i64;
            let ang = i as f32 * std::f32::consts::PI / 4.0;
            let pd = wd * 0.26 * if on { 1.45 } else { 1.0 };
            let (pcx, pcy) = (wd / 2.0 + ang.sin() * wd * 0.36, wd / 2.0 - ang.cos() * wd * 0.36);
            let (x0, y0) = (pcx - pd / 2.0, pcy - pd / 2.0);
            for (j, ch) in pstr.chars().take(4).enumerate() {
                let (lx, ly) = match j {
                    0 => (pd / 2.0, pd * 0.06 + 8.0),
                    1 => (pd * 0.08 + 7.0, pd / 2.0),
                    2 => (pd * 0.92 - 7.0, pd / 2.0),
                    _ => (pd / 2.0, pd * 0.94 - 8.0),
                };
                let s = if vm.shift || vm.caps { ch.to_uppercase().to_string() } else { ch.to_string() };
                let cell = sized(14.0, 16.0, tb(s, 12.0, if on { th.sel_text } else { th.muted }).into());
                layers.push(at(wx + x0 + lx - 7.0, wy + y0 + ly - 8.0, cell));
            }
        }
    }
    let (bg, br, bbc) = (th.bg, th.br, th.bbc);
    let board = container(Stack::with_children(layers).width(Length::Fill).height(Length::Fill))
        .width(Length::Fixed(board_w))
        .height(Length::Fixed(board_h))
        .clip(true)
        .style(move |_: &Theme| container::Style {
            text_color: None,
            background: Some(Background::Color(bg)),
            border: Border { color: bbc, width: 1.0, radius: br.into() },
            shadow: Shadow::default(),
            snap: true,
        });
    container(board)
        .padding(Padding { left: 8.0, right: 8.0, top: if top_pos { 0.0 } else { 6.0 }, bottom: if top_pos { 6.0 } else { 0.0 } })
        .into()
}
