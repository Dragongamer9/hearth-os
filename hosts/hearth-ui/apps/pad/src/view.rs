//! What the overlays draw, each ported from Mantel's QML: the Hearth menu (RadialMenu.qml), the controls card and
//! the short messages (Help.qml, with osk-plasma.qml's list for Plasma), the button hints (HintStrip.qml) and the
//! on-screen keyboard (OskBoard.qml / OskKey.qml, drawing what the shared engine says).

use crate::anim;
use crate::looks::Look;
use crate::osk::{Key, Vm, MOD_KEYS};
use crate::{gm_hidden, App, Msg, ITEMS};
use iced::widget::image;
use std::sync::OnceLock;
use hearth_ui::ui::{self, glyph, t, tb, ts};
use hearth_ui::Pal;
use iced::widget::canvas::{self, Frame, Geometry, Path, Stroke};
use iced::widget::{column, container, mouse_area, row, stack, text, Space, Stack};
use iced::{font, mouse, Background, Border, Color, Element, Length, Padding, Point, Rectangle, Renderer, Shadow, Size, Theme};

fn at<'a>(x: f32, y: f32, e: Element<'a, Msg>) -> Element<'a, Msg> {
    container(e).padding(Padding { top: y.max(0.0), left: x.max(0.0), right: 0.0, bottom: 0.0 }).into()
}
fn sized<'a>(w: f32, h: f32, e: Element<'a, Msg>) -> Element<'a, Msg> {
    container(e).center_x(Length::Fixed(w)).center_y(Length::Fixed(h)).into()
}
/// Mantel's Theme.glass: the panel, all but opaque (0.97); drawn opaque here, as the overlays' alpha adds up
/// over the dimmed screen and let the window behind read through.
fn glass(p: &Pal) -> Color {
    p.panel
}
fn wt(w: font::Weight) -> iced::Font {
    ui::weight(w)
}

// ------------------------------------------------------------------ the Hearth menu (RadialMenu.qml)

struct Ring {
    p: Pal,
    size: f32,
    /// the ring's opacity while it opens / closes, and how lit each item is (0..1) while the light moves
    op: f32,
    lit: [f32; 8],
    /// no glow picture: the warmth drawn instead
    warm: bool,
}

/// Mantel's pictures (desktop/shell/images), read from the shell folder the first time they're drawn; None when
/// they aren't there (then the flame mark and a drawn warmth stand in).
fn picture(name: &'static str, slot: &'static OnceLock<Option<image::Handle>>) -> Option<image::Handle> {
    slot.get_or_init(|| match std::fs::read(hearth_ui::run::shell_dir().join("images").join(name)) {
        Ok(b) => Some(image::Handle::from_bytes(b)),
        Err(e) => {
            eprintln!("hearth-pad-host: images/{name}: {e} (drawn without it)");
            None
        }
    })
    .clone()
}
pub fn logo_picture() -> Option<image::Handle> {
    static L: OnceLock<Option<image::Handle>> = OnceLock::new();
    picture("hearth-logo.png", &L)
}
fn glow_picture() -> Option<image::Handle> {
    static G: OnceLock<Option<image::Handle>> = OnceLock::new();
    picture("ember-glow.png", &G)
}

/// The Hearth logo (images/hearth-logo.png, as RadialMenu.qml and Help.qml show it), or the flame mark without it.
pub fn logo<'a>(p: Pal, size: f32, op: f32) -> Element<'a, Msg> {
    match logo_picture() {
        Some(h) => image(h).width(Length::Fixed(size)).height(Length::Fixed(size)).opacity(op).into(),
        None => flame(Pal { accent: Pal::a(p.accent, op), flame: Pal::a(p.flame, op), ..p }, size),
    }
}

impl canvas::Program<Msg> for Ring {
    type State = ();
    fn draw(&self, _s: &(), renderer: &Renderer, _t: &Theme, bounds: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        let mut f = Frame::new(renderer, bounds.size());
        let (cx, cy, s) = (bounds.width / 2.0, bounds.height / 2.0, self.size);
        let p = self.p;
        let op = self.op;
        let al = |c: Color| Pal::a(c, c.a * op);
        // The soft disc behind the items, warmed by the fire in the middle (the glow picture is its own layer).
        let disc = Path::circle(Point::new(cx, cy), s / 2.0);
        f.fill(&disc, al(glass(&p)));
        f.stroke(&disc, Stroke::default().with_color(al(p.line)).with_width(1.0));
        if self.warm {
            for i in 0..12 {
                let r = s * 0.275 * (1.0 - i as f32 / 12.0);
                f.fill(&Path::circle(Point::new(cx, cy), r), Pal::a(p.accent, 0.016 * op));
            }
        }
        for i in 0..ITEMS.len() {
            // Game Mode off on this PC: its slot stays empty.
            if gm_hidden(ITEMS[i].action) {
                continue;
            }
            let ang = i as f32 / ITEMS.len() as f32 * std::f32::consts::TAU;
            let r = s * 0.36;
            let k = self.lit[i];
            let d = s * 0.17 * (1.0 + 0.14 * k);
            let c = Path::circle(Point::new(cx + ang.sin() * r, cy - ang.cos() * r), d / 2.0);
            f.fill(&c, al(Pal::mix(p.panel2, p.accent, k)));
            f.stroke(&c, Stroke::default().with_color(al(Pal::mix(p.line, Pal::mix(p.accent, Color::WHITE, 0.2), k))).with_width(1.0));
        }
        vec![f.into_geometry()]
    }
}

/// The Hearth mark: a flame in the accent with its flame-coloured heart.
struct Flame {
    p: Pal,
}

impl canvas::Program<Msg> for Flame {
    type State = ();
    fn draw(&self, _s: &(), renderer: &Renderer, _t: &Theme, bounds: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        let mut f = Frame::new(renderer, bounds.size());
        let k = bounds.width.min(bounds.height);
        let q = |x: f32, y: f32| Point::new(x * k, y * k);
        let outer = Path::new(|b| {
            b.move_to(q(0.5, 0.04));
            b.bezier_curve_to(q(0.66, 0.26), q(0.86, 0.40), q(0.86, 0.64));
            b.bezier_curve_to(q(0.86, 0.85), q(0.69, 0.98), q(0.5, 0.98));
            b.bezier_curve_to(q(0.31, 0.98), q(0.14, 0.85), q(0.14, 0.64));
            b.bezier_curve_to(q(0.14, 0.40), q(0.34, 0.26), q(0.5, 0.04));
            b.close();
        });
        let g = canvas::gradient::Linear::new(q(0.5, 0.0), q(0.5, 1.0)).add_stop(0.0, self.p.flame).add_stop(1.0, self.p.accent);
        f.fill(&outer, g);
        let inner = Path::new(|b| {
            b.move_to(q(0.5, 0.42));
            b.bezier_curve_to(q(0.58, 0.56), q(0.68, 0.64), q(0.68, 0.76));
            b.bezier_curve_to(q(0.68, 0.88), q(0.60, 0.94), q(0.5, 0.94));
            b.bezier_curve_to(q(0.40, 0.94), q(0.32, 0.88), q(0.32, 0.76));
            b.bezier_curve_to(q(0.32, 0.64), q(0.42, 0.56), q(0.5, 0.42));
            b.close();
        });
        f.fill(&inner, Pal::mix(self.p.flame, Color::WHITE, 0.45));
        vec![f.into_geometry()]
    }
}

pub fn flame<'a>(p: Pal, size: f32) -> Element<'a, Msg> {
    canvas::Canvas::new(Flame { p }).width(Length::Fixed(size)).height(Length::Fixed(size)).into()
}

pub fn radial(a: &App, size: Size) -> Element<'_, Msg> {
    let p = a.p;
    let (w, h) = (size.width, size.height);
    // RadialMenu.qml: the ring grows from 0.85 (OutBack) and fades in, the screen dims; closing runs it back.
    let (scale, op, dim_a) = a.radial_anim.as_ref().map(|r| anim::radial_look(r, &a.clock)).unwrap_or((1.0, 1.0, 0.55));
    let s = w.min(h) * 0.62 * scale;
    let (cx, cy) = (w / 2.0, h / 2.0);
    let al = |c: Color| Pal::a(c, c.a * op);
    let mut lit = [0.0f32; 8];
    for (i, l) in lit.iter_mut().enumerate() {
        *l = match a.radial_anim.as_ref() {
            Some(r) => anim::lit_amount(r, a.sel, i as i32, &a.clock),
            None => (a.sel == i as i32) as i32 as f32,
        };
    }
    let dim = mouse_area(container(Space::new()).width(Length::Fill).height(Length::Fill).style(ui::boxed(Pal::a(Color::BLACK, dim_a), 0.0, None)))
        .on_press(Msg::RadialDismiss);
    let glow = glow_picture();
    let ring = canvas::Canvas::new(Ring { p, size: s, op, lit, warm: glow.is_none() }).width(Length::Fill).height(Length::Fill);
    let label = if a.sel >= 0 && (a.sel as usize) < ITEMS.len() { item_label(a, a.sel as usize) } else { "Hearth".to_string() };
    let centre = container(
        column![
            logo(p, s * 0.16, op),
            text(label).size((s * 0.05).max(16.0)).color(al(p.text)).font(wt(font::Weight::Semibold)),
            t("Stick to choose · A select · B close", (s * 0.028).max(11.0), al(p.muted)),
        ]
        .spacing(s * 0.02)
        .align_x(iced::Center),
    )
    .center_x(Length::Fill)
    .center_y(Length::Fill);
    let mut layers: Vec<Element<'_, Msg>> = vec![dim.into(), ring.into()];
    // The ember's glow behind the middle (images/ember-glow.png at 0.18).
    if let Some(g) = glow {
        let gs = s * 0.55;
        layers.push(container(image(g).width(Length::Fixed(gs)).height(Length::Fixed(gs)).opacity(0.18 * op)).center_x(Length::Fill).center_y(Length::Fill).into());
    }
    layers.push(centre.into());
    for (i, it) in ITEMS.iter().enumerate() {
        if gm_hidden(it.action) {
            continue;
        }
        let ang = i as f32 / ITEMS.len() as f32 * std::f32::consts::TAU;
        let d = s * 0.17;
        let (x, y) = (cx + ang.sin() * s * 0.36 - d / 2.0, cy - ang.cos() * s * 0.36 - d / 2.0);
        let ic = al(Pal::mix(p.text, p.ink, lit[i]));
        let icon = sized(d, d, glyph(it.icon, ic, d * 0.42, 2.0));
        let hit = mouse_area(icon).on_enter(Msg::RadialHover(i as i32)).on_press(Msg::RadialPick(i as i32)).interaction(mouse::Interaction::Pointer);
        layers.push(at(x, y, hit.into()));
    }
    Stack::with_children(layers).width(Length::Fill).height(Length::Fill).into()
}

pub fn item_label(a: &App, i: usize) -> String {
    if gm_hidden(ITEMS[i].action) {
        return "Game Mode is off".into();
    }
    if ITEMS[i].action == "mouse" {
        return if a.mouse_on { "Controller mouse: on".into() } else { "Controller mouse: off".into() };
    }
    ITEMS[i].label.to_string()
}

// ------------------------------------------------------------------ the controls card and messages (Help.qml)

/// osk-plasma.qml's list: what the controller does in Plasma.
pub const CONTROLS: [(&str, &str); 15] = [
    ("Right stick", "Move the pointer (press it in: precise)"),
    ("Left stick", "Scroll"),
    ("A  /  RT", "Click (in menus A chooses, RT still clicks)"),
    ("X  /  LT", "Right-click"),
    ("B", "Back (Esc)"),
    ("Y", "On-screen keyboard"),
    ("D-pad", "Move around menus and lists (arrow keys)"),
    ("LB  /  RB", "Switch windows"),
    ("Menu ☰", "Apps"),
    ("Guide", "Hearth menu  ·  Guide + X: keyboard"),
    ("View + D-pad", "Tile left / right, maximize, minimize"),
    ("View + A", "The taskbar (D-pad along it, A opens)"),
    ("View + X  /  Y", "All windows  /  clipboard history"),
    ("View + B", "Close the window"),
    ("View", "This list"),
];

pub fn help(a: &App, size: Size) -> Element<'_, Msg> {
    let p = a.p;
    let width = (size.width - 48.0).min(640.0).max(200.0);
    let mut grid = column![].spacing(8.0);
    for (k, v) in CONTROLS {
        grid = grid.push(row![container(text(k).size(14.0).color(p.accent).font(wt(font::Weight::Semibold))).width(Length::Fixed(150.0)), t(v, 14.0, p.text)].spacing(18.0));
    }
    let more = mouse_area(
        container(
            row![
                t("More help: open ", 13.0, p.text),
                tb("Welcome & Tips", 13.0, p.text),
                t(" for the tour and \u{201c}How do I\u{2026}\u{201d} answers", 13.0, p.text),
            ],
        )
        .padding(Padding { top: 10.0, bottom: 10.0, left: 12.0, right: 12.0 })
        .width(Length::Fill)
        .style(ui::boxed(Color::TRANSPARENT, p.r(8.0), Some((p.line, 1.0)))),
    )
    .on_press(Msg::OpenWelcome)
    .interaction(mouse::Interaction::Pointer);
    let card = container(
        column![
            row![logo(p, 36.0, 1.0), ts("Controller controls", 22.0, p.text)].spacing(12.0).align_y(iced::Center),
            t("The controller works like a mouse and keyboard everywhere in Desktop Mode. It pauses while a game is fullscreen.", 13.0, p.muted),
            container(grid).padding(Padding { top: 6.0, ..Padding::ZERO }),
            container(more).padding(Padding { top: 8.0, ..Padding::ZERO }),
        ]
        .spacing(10.0),
    )
    .padding(24.0)
    .width(Length::Fixed(width))
    .style(ui::boxed(glass(&p), p.r(10.0) + 6.0, Some((p.line, 1.0))));
    let base = mouse_area(container(Space::new()).width(Length::Fill).height(Length::Fill)).on_press(Msg::HelpClose);
    let front = container(mouse_area(card).on_press(Msg::HelpClose)).center_x(Length::Fill).center_y(Length::Fill);
    stack![base, front].into()
}

pub fn toast(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    let pill = container(t(a.toast.clone(), 14.0, p.text))
        .padding(Padding { top: 0.0, bottom: 0.0, left: 18.0, right: 18.0 })
        .center_y(Length::Fixed(44.0))
        .style(ui::boxed(glass(&p), 22.0, Some((p.line, 1.0))));
    container(pill).center_x(Length::Fill).padding(Padding { top: 24.0, ..Padding::ZERO }).into()
}

// ------------------------------------------------------------------ the button hints (HintStrip.qml)

pub fn hints(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    let items: [(&str, Color, &str); 6] = [
        ("A", Color::from_rgb8(0x3f, 0xae, 0x5a), "Open"),
        ("X", Color::from_rgb8(0x3b, 0x7d, 0xe0), "Options"),
        ("Y", Color::from_rgb8(0xd9, 0xa5, 0x21), "Keyboard"),
        ("☰", Color::from_rgb8(0x6c, 0x74, 0x80), "Apps"),
        ("⌂", p.accent, "Hearth menu"),
        ("⧉", Color::from_rgb8(0x6c, 0x74, 0x80), "Controls"),
    ];
    let mut r = row![].spacing(22.0).align_y(iced::Center);
    for (b, c, l) in items {
        let italic = matches!(b, "A" | "X" | "Y");
        let f = iced::Font { weight: font::Weight::Bold, style: if italic { font::Style::Italic } else { font::Style::Normal }, ..hearth_ui::app::INTER };
        let dot = container(text(b).size(12.0).color(Color::WHITE).font(f)).center_x(Length::Fixed(24.0)).center_y(Length::Fixed(24.0)).style(ui::boxed(c, 12.0, None));
        r = r.push(row![dot, t(l, 15.0, p.text)].spacing(8.0).align_y(iced::Center));
    }
    let pill = container(r).padding(Padding { top: 0.0, bottom: 0.0, left: 16.0, right: 16.0 }).center_y(Length::Fixed(38.0)).style(ui::boxed(glass(&p), p.r(19.0), Some((p.line, 1.0))));
    container(pill).center_x(Length::Fill).center_y(Length::Fill).into()
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
    let body = container(Stack::with_children(inner).width(Length::Fill).height(Length::Fill))
        .width(Length::Fill)
        .height(Length::Fixed(m.key_h))
        .style(move |_: &Theme| container::Style {
            text_color: None,
            background: Some(Background::Color(bg)),
            border: Border { color: bc, width: bw, radius: radius.into() },
            shadow: Shadow::default(),
            snap: true,
        });
    if k.t == "none" {
        return body.into();
    }
    // OskKey.qml's touch: a tap types on letting go; held 480 ms, the key's accents open (main.rs KeyDown / KeyUp).
    mouse_area(body).on_press(Msg::KeyDown(r, c)).on_release(Msg::KeyUp(r, c)).interaction(mouse::Interaction::Pointer).into()
}

/// Does the ember show (OskBoard.qml): on the picked key while the controller is used, not over the Ember Wheel.
pub fn ember_shown(vm: &Vm) -> bool {
    vm.active && vm.sel.0 >= 0 && !vm.wheel.as_ref().map(|w| w.0).unwrap_or(false)
}

/// The board inside the keyboard's overlay, and where its keys start on it (keyboard() lays them out the same way).
fn board_frame(vm: &Vm, size: Size) -> (f32, f32, Metrics, f32, f32, f32) {
    let board_w = (size.width - 16.0).max(100.0);
    let board_h = (size.height - 6.0).max(60.0);
    let m = metrics(vm, board_h);
    let iw = board_w - 24.0;
    let (kx, ky) = (12.0, 8.0 + m.top_h + m.gap);
    (board_w, board_h, m, iw, kx, ky)
}

/// The picked key on the board: x, y, width, height.
pub fn sel_rect(vm: &Vm, size: Size) -> Option<anim::Rect4> {
    let (_, _, m, iw, kx, ky) = board_frame(vm, size);
    let (r, c) = vm.sel;
    if r < 0 || c < 0 {
        return None;
    }
    let (x, w) = *key_spans(vm, iw, m.gap).get(r as usize)?.get(c as usize)?;
    Some((kx + x, ky + r as f32 * (m.key_h + m.gap), w, m.key_h))
}

/// For CI's touch test: each letter key's middle and the open accents' middles, in the overlay's own coordinates.
pub fn geometry(vm: &Vm, size: Size) -> serde_json::Value {
    let (_, _, m, iw, kx, ky) = board_frame(vm, size);
    let (ox, oy) = (8.0, if vm.position == "top" { 0.0 } else { 6.0 });
    let spans = key_spans(vm, iw, m.gap);
    let mut keys = serde_json::Map::new();
    for (r, row) in vm.rows.iter().enumerate() {
        for (c, k) in row.iter().enumerate() {
            if let Some((x, w)) = spans.get(r).and_then(|s| s.get(c)) {
                if k.t == "ch" {
                    let y = ky + r as f32 * (m.key_h + m.gap);
                    keys.insert(k.v.clone(), serde_json::json!([ox + kx + x + w / 2.0, oy + y + m.key_h / 2.0]));
                }
            }
        }
    }
    let alts: Vec<serde_json::Value> = popup_frame(vm, &m, iw, &spans)
        .map(|(x, y, n)| (0..n).map(|i| serde_json::json!([ox + kx + x + 6.0 + i as f32 * (m.key_h + 6.0) + m.key_h / 2.0, oy + ky + y + 6.0 + m.key_h / 2.0])).collect())
        .unwrap_or_default();
    serde_json::json!({ "w": size.width, "h": size.height, "keys": keys, "alts": alts, "popup": vm.popup.as_ref().map(|p| p.0.clone()) })
}

/// Where the accents' row sits over the picked key (x, y in the keys' area, how many).
fn popup_frame(vm: &Vm, m: &Metrics, iw: f32, spans: &[Vec<(f32, f32)>]) -> Option<(f32, f32, usize)> {
    let (alts, _) = vm.popup.as_ref()?;
    let (r, c) = vm.sel;
    if r < 0 || c < 0 {
        return None;
    }
    let (sx, sw) = *spans.get(r as usize)?.get(c as usize)?;
    let sy = r as f32 * (m.key_h + m.gap);
    let pw = iw.min((m.key_h + 6.0) * alts.len() as f32 + 12.0);
    let ph = m.key_h + 12.0;
    let x = (sx + sw / 2.0 - pw / 2.0).min(iw - pw).max(0.0);
    let y = (sy - ph - 6.0).max(-ph / 2.0);
    Some((x, y, alts.len()))
}

/// Hearth's ember (OskBoard.qml): a ring in the accent around the picked key that glides from key to key, the warmth
/// under it pulsing, and the short trail of sparks it leaves.
struct Ember {
    ring: anim::Rect4,
    warmth: f32,
    sparks: Vec<(f32, f32, f32, f32)>,
    accent: Color,
    radius: f32,
}

impl canvas::Program<Msg> for Ember {
    type State = ();
    fn draw(&self, _s: &(), renderer: &Renderer, _t: &Theme, bounds: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        let mut f = Frame::new(renderer, bounds.size());
        for (x, y, k, o) in &self.sparks {
            f.fill(&Path::circle(Point::new(*x, *y), 4.5 * k), Pal::a(self.accent, *o));
        }
        let (x, y, w, h) = self.ring;
        // the warmth under it (6 px beyond the ring)
        f.fill(&Path::rounded_rectangle(Point::new(x - 9.0, y - 9.0), Size::new(w + 18.0, h + 18.0), (self.radius + 8.0).into()), Pal::a(self.accent, self.warmth));
        let ring = Path::rounded_rectangle(Point::new(x - 1.5, y - 1.5), Size::new(w + 3.0, h + 3.0), (self.radius + 1.5).into());
        f.stroke(&ring, Stroke::default().with_color(self.accent).with_width(3.0));
        vec![f.into_geometry()]
    }
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
    let Some(vm) = a.vm.as_ref() else {
        let p = a.p;
        let why = if a.osk_err.is_empty() { "Opening the keyboard…".to_string() } else { format!("The keyboard couldn't start: {}", a.osk_err) };
        return container(t(why, 14.0, p.text)).center_x(Length::Fill).center_y(Length::Fill).style(ui::boxed(p.panel, 12.0, Some((p.line, 1.0)))).into();
    };
    let th = a.look;
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
            strip = strip.push(mouse_area(pill).on_press(Msg::SuggTap(i)).interaction(mouse::Interaction::Pointer));
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
    // Hearth's ember over the keys (OskBoard.qml z 2-3: the sparks, the warmth, the ring), where its glide has got to.
    if ember_shown(vm) {
        let ring = anim::ember_rect(&a.glides, &a.clock).or_else(|| sel_rect(vm, size));
        if let Some(ring) = ring {
            let warmth = a.ember_since.map(|t0| anim::warmth(t0, &a.clock)).unwrap_or(0.22);
            let sparks = anim::sparks(&a.glides, &a.clock);
            let radius = key_radius(&th, vm, m.key_h);
            layers.push(canvas::Canvas::new(Ember { ring, warmth, sparks, accent: th.accent, radius }).width(Length::Fill).height(Length::Fill).into());
        }
    }
    // accents (hold A, View, or a long touch)
    if let (Some((alts, pi)), Some((x, y, _))) = (vm.popup.as_ref(), popup_frame(vm, &m, iw, &spans)) {
        let pw = iw.min((m.key_h + 6.0) * alts.len() as f32 + 12.0);
        let ph = m.key_h + 12.0;
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
            // A tap picks it; after a long touch, sliding onto it lights it and lifting there types it.
            r = r.push(mouse_area(cell).on_press(Msg::AltTap(i)).on_enter(Msg::AltHover(i)).on_release(Msg::AltLift(i)).interaction(mouse::Interaction::Pointer));
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
    // A press let go anywhere else ends it (a long touch lifted off the keys leaves the accents open).
    container(mouse_area(board).on_release(Msg::PressEnd))
        .padding(Padding { left: 8.0, right: 8.0, top: if top_pos { 0.0 } else { 6.0 }, bottom: if top_pos { 6.0 } else { 0.0 } })
        .into()
}
