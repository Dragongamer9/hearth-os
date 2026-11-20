//! The desktop and laptop pages (mockups 1a, 1b, 1c, 2, 3, 4, 4b, 5, 6 and 8, 1920 × 1080): the title bar, the sidebar
//! (My consoles with its count, Cloud marked optional, Settings; the gamertag and the console's status; "From the
//! Hearth Store · not made by Microsoft"), each page, and the controller hints along the bottom.

use crate::look::*;
use crate::{App, CloudTab, Dd, Load, Msg, Page};
use crp::data;
use crp::xhome::{Kind, Power};
use iced::widget::{column, container, image, mouse_area, pin, row, scrollable, stack, text_input, Column, Row, Space};
use iced::{Color, Length, Theme};
use serde_json::{json, Value};

pub fn power_color(p: Power) -> Color {
    match p {
        Power::On => OK,
        Power::Sleeping => SLEEP,
        Power::Off => OFF,
    }
}

pub fn power_chip<'a>(p: Power, size: f32) -> E<'a, Msg> {
    match p {
        Power::On => chip("On", Ch::Ok, Some(OK), size),
        Power::Sleeping => chip("Sleeping", Ch::Sleep, Some(SLEEP), size),
        Power::Off => chip("Offline", Ch::Plain, Some(OFF), size),
    }
}

pub fn ago(seen: u64) -> String {
    let s = crp::now().saturating_sub(seen);
    match s {
        0..=59 => "just now".into(),
        60..=119 => "a minute ago".into(),
        120..=3599 => format!("{} minutes ago", s / 60),
        3600..=7199 => "an hour ago".into(),
        _ => format!("{} hours ago", s / 3600),
    }
}

/// "this laptop" or "this PC".
pub fn this_device(a: &App) -> &'static str {
    if a.laptop {
        "this laptop"
    } else {
        "this PC"
    }
}

// ------------------------------------------------------------------ the frame
fn title_bar<'a>() -> E<'a, Msg> {
    let wb = |g: G, m: Msg| -> E<'a, Msg> { click(container(glyph(g, MUT, 14.0)).center_x(Length::Fixed(24.0)).center_y(Length::Fixed(52.0)), Some(m)) };
    let name = row![app_icon(28.0, 18.0, 8.0), gap(12.0, 1.0), one(t6("Hearth Console Remote Play", 17.0, TX)), gap(8.0, 1.0), one(t("· Hearth", 15.0, MUT))].align_y(iced::Center);
    let drag = mouse_area(container(row![name, fill()].align_y(iced::Center)).height(Length::Fixed(52.0)).width(Length::Fill)).on_press(Msg::Drag).on_double_click(Msg::Max);
    let r = row![gap(20.0, 1.0), drag, wb(G::Min, Msg::Min), gap(24.0, 1.0), wb(G::Max, Msg::Max), gap(24.0, 1.0), wb(G::X, Msg::Close), gap(20.0, 1.0)].align_y(iced::Center);
    column![container(r).height(Length::Fixed(51.0)).width(Length::Fill).style(bx(SIDE, 0.0, None)), hline(LINE)].into()
}

fn nav_item<'a>(g: G, label: &str, on: bool, right: E<'a, Msg>, m: Msg) -> E<'a, Msg> {
    let col = if on { Color::WHITE } else { Color::from_rgb8(0xd5, 0xd8, 0xde) };
    let bar: E<'a, Msg> = if on { container(Space::new()).width(Length::Fixed(3.0)).height(Length::Fixed(24.0)).style(bx(ACC, 2.0, None)).into() } else { gap(3.0, 24.0) };
    let r = row![bar, gap(11.0, 1.0), glyph(g, if on { ACC2 } else { MUT }, 20.0), gap(12.0, 1.0), one(t(label.to_string(), 16.0, col)), fill(), right, gap(14.0, 1.0)].align_y(iced::Center);
    click(container(r).height(Length::Fixed(44.0)).center_y(Length::Fixed(44.0)).width(Length::Fill).style(bx(if on { CARD2 } else { Color::TRANSPARENT }, 9.0, None)), Some(m))
}

fn sidebar<'a>(a: &'a App) -> E<'a, Msg> {
    let n = match &a.consoles {
        Load::Ready(v) => v.len().to_string(),
        _ => String::new(),
    };
    let cur = match a.page {
        Page::Settings => 2,
        Page::Cloud => 1,
        _ => 0,
    };
    let mut items = Column::new().spacing(4.0);
    items = items.push(nav_item(G::Console, "My consoles", cur == 0, one(t(n, 13.0, OFF)).into(), Msg::Go(Page::Consoles)));
    if a.cloud || a.page == Page::Cloud {
        let opt = container(one(t("optional", 11.5, MUT))).padding(pad(1.0, 8.0, 1.0, 8.0)).style(bx(Color::TRANSPARENT, 10.0, Some((EDGE, 1.0))));
        items = items.push(nav_item(G::Cloud, "Cloud", cur == 1, opt.into(), Msg::Go(Page::Cloud)));
    }
    items = items.push(nav_item(G::Gear, "Settings", cur == 2, gap(1.0, 1.0), Msg::Go(Page::Settings)));
    let mut foot = Column::new().spacing(6.0).push(one(t6(if a.gamertag.is_empty() { "Not signed in".to_string() } else { a.gamertag.clone() }, 14.0, SOFT2)));
    if let Some((name, p)) = a.console_status() {
        foot = foot.push(row![dotted(power_color(p), 9.0), gap(7.0, 1.0), one(t(format!("{name} · {}", p.words()), 14.0, MUT))].align_y(iced::Center));
    }
    foot = foot.push(one(t("From the Hearth Store · not made by Microsoft", 13.0, MUT)));
    let col = column![items, vfill(), hline(LINE), container(foot).padding(pad(14.0, 8.0, 0.0, 8.0))];
    row![container(col).padding(pad(18.0, 14.0, 18.0, 14.0)).width(Length::Fixed(329.0)).height(Length::Fill).style(bx(SIDE, 0.0, None)), container(Space::new()).width(Length::Fixed(1.0)).height(Length::Fill).style(bx(LINE, 0.0, None))].into()
}

fn lbrb<'a>() -> E<'a, Msg> {
    hint(&["LB", "RB"], "Consoles / Cloud / Settings", 16.0)
}

fn frame<'a>(a: &'a App, nav: bool, page: E<'a, Msg>, left: Vec<E<'a, Msg>>, right: Vec<E<'a, Msg>>) -> E<'a, Msg> {
    let body: E<'a, Msg> = if nav { row![sidebar(a), page].height(Length::Fill).into() } else { container(page).width(Length::Fill).height(Length::Fill).into() };
    let all: E<'a, Msg> = column![title_bar(), body, hints(left, right, 54.0, 28.0, 28.0, BG)].into();
    let all = container(all).style(bx(BG, 0.0, None)).width(Length::Fill).height(Length::Fill);
    hearth_ui::ui::edges(all.into(), Msg::Resize)
}

fn h1<'a>(s: &str) -> E<'a, Msg> {
    one(t6(s.to_string(), 34.0, TX)).into()
}

fn sub<'a>(s: &str) -> E<'a, Msg> {
    container(t(s.to_string(), 15.5, MUT).line_height(1.45)).padding(pad(8.0, 0.0, 20.0, 0.0)).into()
}

fn note<'a>(a: &'a App) -> Option<E<'a, Msg>> {
    a.note.as_ref().map(|n| container(row![glyph(G::Data, WARN, 18.0), gap(10.0, 1.0), t(n.clone(), 14.5, Color::from_rgb8(0xff, 0xe0, 0x8a))].align_y(iced::Center)).padding(pad(10.0, 16.0, 10.0, 16.0)).style(bx(QUIET, 10.0, Some((Color::from_rgb8(0x7a, 0x66, 0x33), 1.0)))).into())
}

/// The welcome pages' warm glow behind the middle (CSS's radial-gradient).
fn glow<'a>(inner: E<'a, Msg>, warm: Color) -> E<'a, Msg> {
    stack![iced::widget::canvas(Glow { warm }).width(Length::Fill).height(Length::Fill), inner].into()
}

struct Glow {
    warm: Color,
}

impl<M> iced::widget::canvas::Program<M> for Glow {
    type State = Kept<(Color, iced::Size)>;
    fn draw(&self, st: &Self::State, r: &iced::Renderer, _t: &Theme, b: iced::Rectangle, _c: iced::mouse::Cursor) -> Vec<iced::widget::canvas::Geometry> {
        use iced::widget::canvas::Path;
        st.check((self.warm, b.size()));
        vec![st.cache.draw(r, b.size(), |f| {
            f.fill_rectangle(iced::Point::ORIGIN, b.size(), BG);
            // 1200 × 700 at 50 % 30 %, fading to the page colour by 70 %.
            let (cx, cy) = (b.width * 0.5, b.height * 0.3);
            let n = 28;
            for i in (0..n).rev() {
                let k = (i + 1) as f32 / n as f32 * 0.7;
                let col = Color { r: self.warm.r + (BG.r - self.warm.r) * (k / 0.7), g: self.warm.g + (BG.g - self.warm.g) * (k / 0.7), b: self.warm.b + (BG.b - self.warm.b) * (k / 0.7), a: 1.0 };
                let (rx, ry) = (1200.0 * k, 700.0 * k);
                let pts: Vec<iced::Point> = (0..=48).map(|j| {
                    let t = std::f32::consts::TAU * j as f32 / 48.0;
                    iced::Point::new(cx + rx * t.cos(), cy + ry * t.sin())
                }).collect();
                let p = Path::new(|pb| {
                    pb.move_to(pts[0]);
                    for q in &pts[1..] {
                        pb.line_to(*q);
                    }
                    pb.close();
                });
                f.fill(&p, col);
            }
        })]
    }
}

// ------------------------------------------------------------------ the pages
pub fn view(a: &App) -> E<'_, Msg> {
    let page = match a.page {
        Page::Welcome | Page::SignIn => welcome(a),
        Page::SignedIn => signed_in(a),
        Page::Consoles => consoles(a),
        Page::Connecting => connecting(a),
        Page::Stream => return stream_view(a),
        Page::Trouble => trouble(a),
        Page::Settings => settings(a),
        Page::Cloud => cloud(a),
    };
    let mut layers: Vec<E<'_, Msg>> = vec![page];
    if let Some(d) = a.dd {
        layers.push(chooser(a, d));
    }
    if a.ask_limit.is_some() {
        layers.push(limit_dialog(a));
    }
    if layers.len() == 1 {
        return layers.pop().unwrap();
    }
    stack(layers).into()
}

fn welcome(a: &App) -> E<'_, Msg> {
    let signing = a.page == Page::SignIn;
    let card = |g: G, title: &str, line: &str| -> E<'_, Msg> {
        container(row![ri(g, 40.0), column![t(title.to_string(), 17.0, TX), t(line.to_string(), 14.0, MUT).line_height(1.4)].spacing(3.0)].spacing(16.0).align_y(iced::Top))
            .padding(pad(14.0, 20.0, 14.0, 20.0))
            .width(Length::Fill)
            .style(bx(CARD, 12.0, None))
            .into()
    };
    let icon = container(app_icon(110.0, 64.0, 28.0)).style(shadowed(Color::TRANSPARENT, 28.0, None, 0.0));
    let mut col = Column::new().align_x(iced::Center).width(Length::Fixed(1040.0));
    col = col.push(icon).push(gap(1.0, 26.0));
    col = col.push(one(t7("Play your Xbox from anywhere", 44.0, TX)));
    col = col.push(container(t("Hearth Console Remote Play shows your own Xbox at home on this screen, over the internet. Your games keep running on the Xbox; you see and hear them here and play with this PC's controller.", 19.0, MUT).line_height(1.5).align_x(iced::Center)).max_width(820.0).padding(pad(12.0, 0.0, 34.0, 0.0)));
    col = col.push(btn("Sign in with Microsoft", Some(G::Lock), B::Primary, true, 19.0, a.ring && a.focus == 0 && !signing, Some(Msg::SignIn)));
    col = col.push(gap(1.0, 12.0));
    col = col.push(row![glyph(G::Lock, OK, 16.0), gap(6.0, 1.0), one(t("Opens Microsoft's own sign-in page in a window here. You type your password on Microsoft's page, never into this app.", 15.0, MUT))].align_y(iced::Center));
    if let Some(n) = note(a) {
        col = col.push(gap(1.0, 14.0)).push(n);
    }
    col = col.push(gap(1.0, 46.0));
    col = col.push(row![
        card(G::User, "The account on your Xbox", "Sign in with the same Microsoft account your Xbox uses."),
        card(G::Remote, "Remote features on", "On the Xbox, once. The next page shows where."),
        card(G::Wifi, "Good internet both ends", "About 10 Mbps upload at home is enough for a sharp picture.")
    ]
    .spacing(16.0));
    col = col.push(gap(1.0, 30.0));
    col = col.push(one(t("Hearth Console Remote Play is a Hearth app. It isn't made by Microsoft.", 13.5, OFF)));
    let mut page = glow(container(col).center_x(Length::Fill).center_y(Length::Fill).into(), Color::from_rgb8(0x2a, 0x1c, 0x12));
    if signing {
        // 1b: the app dimmed under the sign-in window (Microsoft's page, its own window: the helper).
        page = stack![page, container(Space::new()).width(Length::Fill).height(Length::Fill).style(bx(a_(0.66), 0.0, None))].into();
        return frame(a, false, page, vec![hint(&["B"], "Cancel", 16.0)], vec![one(t("From the Hearth Store", 16.0, Color::from_rgb8(0xc9, 0xcc, 0xd3))).into()]);
    }
    frame(a, false, page, vec![hint(&["A"], "Sign in", 16.0), hint(&["B"], "Close", 16.0)], vec![one(t("From the Hearth Store", 16.0, Color::from_rgb8(0xc9, 0xcc, 0xd3))).into()])
}

fn a_(alpha: f32) -> Color {
    Color { r: 0.0, g: 0.0, b: 0.0, a: alpha }
}

fn signed_in(a: &App) -> E<'_, Msg> {
    let check = container(one(t7("✓", 56.0, Color::from_rgb8(0x8f, 0xf0, 0xb3)))).center_x(Length::Fixed(110.0)).center_y(Length::Fixed(110.0)).style(bx(Color::from_rgb8(0x17, 0x35, 0x24), 55.0, None));
    let initial = a.gamertag.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default();
    let avatar = container(one(t7(initial, 22.0, INK))).center_x(Length::Fixed(56.0)).center_y(Length::Fixed(56.0)).style(grad(135.0, ACC2, Color::from_rgb8(0x8a, 0x3b, 0x14), 28.0));
    let who = container(row![avatar, column![t("Signed in as", 14.0, MUT), one(t6(a.gamertag.clone(), 24.0, TX))].spacing(2.0), fill(), chip("Signed in", Ch::Ok, None, 13.5)].spacing(16.0).align_y(iced::Center))
        .padding(pad(18.0, 22.0, 18.0, 22.0))
        .width(Length::Fixed(520.0))
        .style(bx(CARD, 12.0, None));
    let mut col = Column::new().align_x(iced::Center).width(Length::Fixed(820.0));
    col = col.push(check).push(gap(1.0, 24.0)).push(one(t7("You're signed in", 40.0, TX))).push(gap(1.0, 28.0)).push(who);
    col = col.push(container(t("Only your gamertag is shown. Your email and password stay with Microsoft.", 15.0, MUT)).padding(pad(14.0, 0.0, 0.0, 0.0)));
    if let Some(n) = note(a) {
        col = col.push(gap(1.0, 12.0)).push(n);
    }
    col = col.push(gap(1.0, 34.0)).push(btn("Find my Xbox", None, B::Primary, true, 18.0, a.ring && a.focus == 0, Some(Msg::Go(Page::Consoles))));
    let page = glow(container(col).center_x(Length::Fill).center_y(Length::Fill).into(), Color::from_rgb8(0x1b, 0x2a, 0x1f));
    frame(a, false, page, vec![hint(&["A"], "Find my Xbox", 16.0)], vec![one(t("From the Hearth Store", 16.0, Color::from_rgb8(0xc9, 0xcc, 0xd3))).into()])
}

pub fn checklist<'a>(gt: &str, ally: bool) -> E<'a, Msg> {
    let num = |n: &str| -> E<'a, Msg> { container(one(t7(n.to_string(), 14.0, ACC2))).center_x(Length::Fixed(30.0)).center_y(Length::Fixed(30.0)).style(bx(Color::TRANSPARENT, 15.0, Some((ACC, 2.0)))).into() };
    let items: Vec<(&str, &str, String)> = if ally {
        vec![
            ("1", "", "Settings › Devices & connections › Remote features › **Enable remote features**".into()),
            ("2", "", "Settings › General › Power options › **Sleep**".into()),
            ("3", "", "Leave it **plugged in**, ideally **wired** to the router".into()),
            ("4", "", format!("Signed in as **{gt}** on the Xbox")),
        ]
    } else {
        vec![
            ("1", "Turn on remote features", "Settings › Devices & connections › Remote features › tick **Enable remote features**.".into()),
            ("2", "Set Power options to Sleep", "Settings › General › Power options › **Sleep**. With Shutdown (energy saving) it can't be woken from here.".into()),
            ("3", "Leave it plugged in, ideally wired", "Power on at the wall, and an Ethernet cable to your router if you can. Wi-Fi works, but wired is steadier.".into()),
            ("4", "Same account", format!("The Xbox is signed in as **{gt}**, the account you used here.")),
        ]
    };
    let mut c = Column::new();
    let n = items.len();
    for (i, (k, title, line)) in items.into_iter().enumerate() {
        let mut txt = Column::new().spacing(3.0);
        if !title.is_empty() {
            txt = txt.push(t(title.to_string(), 17.0, TX));
        }
        txt = txt.push(rich(&line, if ally { 15.0 } else { 14.0 }, MUT, SOFT, false));
        c = c.push(container(row![num(k), txt.width(Length::Fill)].spacing(14.0).align_y(iced::Top)).padding(pad(if ally { 10.0 } else { 12.0 }, 0.0, if ally { 10.0 } else { 12.0 }, 0.0)));
        if i + 1 < n {
            c = c.push(hline(LINE));
        }
    }
    c.into()
}

fn consoles(a: &App) -> E<'_, Msg> {
    let head = row![h1("My consoles"), fill(), btn("Refresh", Some(G::Refresh), B::Normal, false, 15.0, false, Some(Msg::Refresh))].align_y(iced::Center);
    let mut left = Column::new();
    match &a.consoles {
        Load::Ready(list) if !list.is_empty() => {
            for (i, c) in list.iter().enumerate() {
                let waking = a.waking.as_deref() == Some(c.id.as_str());
                let chips = row![
                    if waking { chip("Waking…", Ch::On, Some(ACC2), 13.5) } else { power_chip(c.power, 13.5) },
                    if c.remote_on { chip("Remote features on", Ch::Ok, None, 13.5) } else { chip("Remote features off", Ch::Warn, None, 13.5) },
                    one(t(format!("Last seen {}", ago(c.seen)), 14.0, MUT))
                ]
                .spacing(10.0)
                .align_y(iced::Center);
                let buttons = row![
                    btn(if waking { "Waking…" } else { "Wake" }, Some(G::Power), B::Normal, true, 18.0, false, (c.power == Power::Sleeping && !waking).then_some(Msg::Wake(i))),
                    btn("Stream", Some(G::Play), B::Primary, true, 18.0, false, (c.power != Power::Off).then_some(Msg::Stream(i)))
                ]
                .spacing(12.0);
                let info = column![
                    one(t6(c.name.clone(), 30.0, TX)),
                    gap(1.0, 4.0),
                    one(t(format!("{} · signed in as {}", c.model, a.gamertag), 15.5, MUT)),
                    container(chips).padding(pad(16.0, 0.0, 20.0, 0.0)),
                    buttons,
                    container(t("Stream wakes it first if it's sleeping.", 14.0, MUT)).padding(pad(12.0, 0.0, 0.0, 0.0))
                ];
                let card = ring0(a.ring && a.focus == i, 16.0, CARD, container(row![console_box(150.0, 170.0, 18.0, power_color(c.power)), info.width(Length::Fill)].spacing(28.0).align_y(iced::Center)).padding(pad(26.0, 28.0, 26.0, 28.0)));
                left = left.push(card).push(gap(1.0, 12.0));
            }
        }
        Load::Ready(_) => {
            left = left.push(container(column![one(t6("No Xbox on this account yet", 22.0, TX)), t("Hearth found no Xbox signed in with this account that can stream. Check the list on the right, then press Refresh.", 15.0, MUT)].spacing(8.0)).padding(26.0).width(Length::Fill).style(bx(CARD, 16.0, None)));
        }
        Load::Failed(e) => {
            left = left.push(container(column![one(t6("Couldn't get your consoles", 22.0, TX)), t(e.clone(), 15.0, MUT), gap(1.0, 8.0), btn("Try again", Some(G::Refresh), B::Normal, false, 15.0, a.ring, Some(Msg::Refresh))].spacing(8.0)).padding(26.0).width(Length::Fill).style(bx(CARD, 16.0, None)));
        }
        _ => {
            left = left.push(container(row![glyph(G::Refresh, MUT, 22.0), gap(12.0, 1.0), t("Looking for your Xbox…", 18.0, MUT)].align_y(iced::Center)).padding(26.0).width(Length::Fill).style(bx(CARD, 16.0, None)));
        }
    }
    if let Some(n) = note(a) {
        left = left.push(n);
    }
    left = left.push(sec("What the status means", 8.0));
    let legend = |c: E<'static, Msg>, line: &str| -> E<'static, Msg> { container(row![c, rich(line, 15.0, MUT, SOFT, false)].spacing(16.0).align_y(iced::Center)).padding(pad(14.0, 20.0, 14.0, 20.0)).width(Length::Fill).style(bx(CARD, 12.0, None)).into() };
    left = left.push(legend(chip_w("On", Ch::Ok, Some(OK), 120.0), "Ready to stream right away.")).push(gap(1.0, 8.0));
    left = left.push(legend(chip_w("Sleeping", Ch::Sleep, Some(SLEEP), 120.0), "Asleep but listening. **Wake** turns it on from here, in about 15 seconds.")).push(gap(1.0, 8.0));
    left = left.push(legend(chip_w("Offline", Ch::Plain, Some(OFF), 120.0), "Fully off, unplugged or without internet. It can't be woken from here; check the list on the right."));
    let right = container(column![one(t6("Can't see your Xbox?", 21.0, TX)), gap(1.0, 4.0), t("Do these once, on the Xbox itself:", 15.0, MUT), gap(1.0, 6.0), checklist(&a.gamertag, false)]).padding(pad(22.0, 26.0, 22.0, 26.0)).width(Length::Fixed(600.0)).style(bx(SIDE, 16.0, None));
    let body = column![head, sub("The Xboxes signed in with your account. Hearth asks Microsoft for this list only when this page opens or you press Refresh."), row![left.width(Length::Fill), column![right]].spacing(26.0)];
    let page = container(scrollable(body)).padding(pad(30.0, 44.0, 30.0, 44.0)).width(Length::Fill).height(Length::Fill);
    frame(a, true, page.into(), vec![hint(&["A"], "Stream", 16.0), hint(&["X"], "Wake", 16.0), hint(&["Y"], "Refresh", 16.0), hint(&["B"], "Back", 16.0)], vec![lbrb()])
}

/// A step's circle: done (✓), now (a turning ring), waiting (its number).
pub fn step_circle<'a>(kind: u8, n: &str, size: f32) -> E<'a, Msg> {
    match kind {
        0 => container(one(t7("✓", size * 0.45, Color::from_rgb8(0x8f, 0xf0, 0xb3)))).center_x(Length::Fixed(size)).center_y(Length::Fixed(size)).style(bx(Color::from_rgb8(0x17, 0x35, 0x24), size / 2.0, None)).into(),
        1 => iced::widget::canvas(Spin).width(Length::Fixed(size)).height(Length::Fixed(size)).into(),
        _ => container(one(t7(n.to_string(), size * 0.4, OFF))).center_x(Length::Fixed(size)).center_y(Length::Fixed(size)).style(bx(Color::TRANSPARENT, size / 2.0, Some((Color::from_rgb8(0x3a, 0x3f, 0x49), 2.0)))).into(),
    }
}

struct Spin;

impl<M> iced::widget::canvas::Program<M> for Spin {
    type State = ();
    fn draw(&self, _s: &(), r: &iced::Renderer, _t: &Theme, b: iced::Rectangle, _c: iced::mouse::Cursor) -> Vec<iced::widget::canvas::Geometry> {
        use iced::widget::canvas::{Frame, Path, Stroke};
        let mut f = Frame::new(r, b.size());
        let c = iced::Point::new(b.width / 2.0, b.height / 2.0);
        let rad = b.width / 2.0 - 1.5;
        f.stroke(&Path::circle(c, rad), Stroke::default().with_color(Color::from_rgb8(0x3a, 0x3f, 0x49)).with_width(3.0));
        let arc = Path::new(|p| {
            p.arc(iced::widget::canvas::path::Arc { center: c, radius: rad, start_angle: iced::Radians(-2.36), end_angle: iced::Radians(-0.79) });
        });
        f.stroke(&arc, Stroke::default().with_color(ACC).with_width(3.0));
        vec![f.into_geometry()]
    }
}

pub fn step_row<'a>(kind: u8, n: &str, title: &str, line: Option<String>, right: Option<String>, size: (f32, f32, f32)) -> E<'a, Msg> {
    let (circle, ts, ls) = size;
    let mut txt = Column::new().spacing(3.0).push(t(title.to_string(), ts, if kind == 2 { Color::from_rgb8(0x8a, 0x90, 0x9b) } else { TX }));
    if let Some(l) = line {
        txt = txt.push(t(l, ls, MUT));
    }
    let mut r = row![step_circle(kind, n, circle), txt.width(Length::Fill)].spacing(18.0).align_y(iced::Center);
    if let Some(x) = right {
        r = r.push(one(t(x, ls + 1.0, MUT)));
    }
    container(r).padding(pad(18.0, 22.0, 18.0, 22.0)).width(Length::Fill).style(bx(CARD, 12.0, None)).into()
}

pub fn clock(secs: u64) -> String {
    format!("{}:{:02}", secs / 60, secs % 60)
}

/// The three steps' words: (kind, title, line, timer) for each.
pub fn steps(a: &App, short: bool) -> Vec<(u8, String, Option<String>, Option<String>)> {
    let Some(c) = &a.conn else { return vec![] };
    let el = clock(c.since.elapsed().as_secs());
    let mut v = Vec::new();
    if c.kind == Kind::Home {
        let (k, line) = if c.step <= 1 && c.waking {
            (1, Some(if short { "Waking it up".to_string() } else { "It was sleeping · waking it up".to_string() }))
        } else if let Some(s) = c.woke {
            (0, Some(if short { format!("Awake after {s} s") } else { format!("It was sleeping · awake after {s} s") }))
        } else {
            (0, Some("It was on already".to_string()))
        };
        v.push((k, "Waking your Xbox".to_string(), line, if k == 1 { Some(el.clone()) } else { None }));
    }
    let ck = if c.step == 2 { 1 } else if c.step > 2 { 0 } else { 2 };
    let cline = match c.queue {
        Some(q) if q > 0 => format!("In the queue for a server · about {} min", (q + 59) / 60),
        _ if short => "Checking the connection speed".to_string(),
        _ => "Finding the best way through · checking the connection speed".to_string(),
    };
    v.push((ck, "Connecting".to_string(), Some(cline), if ck == 1 { Some(el.clone()) } else { None }));
    let pk = if c.step >= 3 { 1 } else { 2 };
    v.push((pk, "Starting the picture".to_string(), if short { None } else { Some("Picture and sound start, then it goes full screen".to_string()) }, if pk == 1 { Some(el) } else { None }));
    v
}

fn connecting(a: &App) -> E<'_, Msg> {
    let (name, model, cloud) = a.conn.as_ref().map(|c| (c.name.clone(), c.model.clone(), c.kind == Kind::Cloud)).unwrap_or(("your Xbox".into(), String::new(), false));
    let mut col = Column::new().width(Length::Fixed(900.0));
    col = col.push(
        row![
            console_box(96.0, 110.0, 14.0, ACC2),
            column![
                one(t6(format!("Connecting to {name}"), 34.0, TX)),
                t(if cloud { format!("Over the internet, from {} to Microsoft's servers.", this_device(a)) } else { format!("Over the internet, from {} to your Xbox at home.", this_device(a)) }, 17.0, MUT)
            ]
            .spacing(6.0)
        ]
        .spacing(24.0)
        .align_y(iced::Center),
    );
    col = col.push(gap(1.0, 28.0));
    for (i, (k, title, line, timer)) in steps(a, false).into_iter().enumerate() {
        col = col.push(step_row(k, &(i + 1).to_string(), &title, line, timer, (38.0, 17.0, 14.0))).push(gap(1.0, 10.0));
    }
    col = col.push(gap(1.0, 12.0));
    col = col.push(row![btn("Cancel", Some(G::X), B::Normal, true, 18.0, a.ring && a.focus == 0, Some(Msg::Cancel)), t(if cloud { "This usually takes 10 to 30 seconds." } else { "This usually takes 10 to 30 seconds. Your Xbox stays on if you cancel." }, 15.0, MUT)].spacing(16.0).align_y(iced::Center));
    let page = container(col).center_x(Length::Fill).center_y(Length::Fill).style(bx(DEEP, 0.0, None));
    let right = if model.is_empty() { name.clone() } else { format!("{name} · {model}") };
    frame(a, true, page.into(), vec![hint(&["B"], "Cancel", 16.0)], vec![one(t(right, 16.0, Color::from_rgb8(0xc9, 0xcc, 0xd3))).into()])
}

/// What went wrong, in plain words.
pub fn trouble_words(a: &App) -> String {
    let name = a.conn.as_ref().map(|c| c.name.clone()).unwrap_or_else(|| "Your Xbox".into());
    match &a.trouble {
        Some((why, true)) if why.contains("in time") => format!("{name} woke up, but the picture couldn't get through to {} in time.", this_device(a)),
        Some((why, _)) => why.clone(),
        None => String::new(),
    }
}

fn trouble(a: &App) -> E<'_, Msg> {
    let net = &a.net;
    let dev = if a.laptop { "This laptop" } else { "This PC" };
    let netline = if net.name.is_empty() { format!("{dev}: **{}**", net.words()) } else { format!("{dev}: **{}** ({})", net.words(), net.name) };
    let weak = net.kind == "wifi" && net.band == "2.4";
    let status = row![
        row![dotted(OK, 9.0), gap(7.0, 1.0), rich("Xbox: **On**", 15.0, TX, Color::WHITE, false)].align_y(iced::Center),
        row![dotted(if a.signed_in { OK } else { BAD }, 9.0), gap(7.0, 1.0), t("Signed in", 15.0, TX)].align_y(iced::Center),
        row![dotted(if weak || net.kind.is_empty() { WARN } else { OK }, 9.0), gap(7.0, 1.0), rich(&netline, 15.0, TX, Color::WHITE, false)].align_y(iced::Center),
        fill(),
        btn("Try again", Some(G::Refresh), B::Normal, false, 15.0, a.ring && a.focus == 0, Some(Msg::TryAgain(true))),
        btn("Try at 720p", None, B::Primary, false, 15.0, a.ring && a.focus == 1, Some(Msg::TryAgain(false)))
    ]
    .spacing(30.0)
    .align_y(iced::Center);
    let tip = |g: G, title: &str, line: &str, end: Option<E<'static, Msg>>| -> E<'static, Msg> { srow(Some(ri(g, 40.0)), title, Some(rich(line, 14.0, MUT, SOFT, false)), end, 17.0, CARD, pad(14.0, 20.0, 14.0, 20.0), false) };
    let wifi_line = if weak {
        "You're on 2.4 GHz, which is slower and busier. Pick the network's 5 GHz name if it has one, or turn on your phone's hotspot (watch the data: about 4–6 GB an hour at 1080p, 2–3 with Data saver).".to_string()
    } else {
        "2.4 GHz Wi-Fi is slower and busier. Pick the network's 5 GHz name if it has one, or turn on your phone's hotspot (watch the data: about 4–6 GB an hour at 1080p, 2–3 with Data saver).".to_string()
    };
    let mut col = Column::new().width(Length::Fixed(1100.0));
    col = col.push(
        row![
            container(glyph(G::Console, Color::from_rgb8(0xff, 0x8a, 0x8a), 52.0)).center_x(Length::Fixed(96.0)).center_y(Length::Fixed(96.0)).style(bx(Color::from_rgb8(0x2a, 0x16, 0x16), 24.0, Some((Color::from_rgb8(0x6a, 0x2d, 0x2d), 1.0)))),
            column![one(t6("Couldn't reach your Xbox", 34.0, TX)), t(trouble_words(a), 17.0, MUT)].spacing(6.0).width(Length::Fill)
        ]
        .spacing(26.0)
        .align_y(iced::Center),
    );
    col = col.push(gap(1.0, 24.0));
    col = col.push(container(status).padding(pad(14.0, 22.0, 14.0, 22.0)).width(Length::Fill).style(bx(QUIET, 12.0, None)));
    col = col.push(gap(1.0, 22.0)).push(sec("Things that usually fix it", 0.0));
    col = col.push(tip(G::Up, "Check your home upload speed", "Your Xbox sends the picture with your home internet's **upload**. About **10 Mbps or more** is good; under 5 Mbps the picture stutters or won't start. Run a speed test at home and look at the upload number.", None)).push(gap(1.0, 8.0));
    col = col.push(tip(G::Wifi, "Use 5 GHz Wi-Fi or your phone's hotspot", &wifi_line, None)).push(gap(1.0, 8.0));
    col = col.push(tip(G::Building, "Work Wi-Fi may block game streaming", "Many work and school networks block it on purpose. If it works on your hotspot but not here, that's why, and there's nothing to fix on your side.", None)).push(gap(1.0, 8.0));
    col = col.push(tip(G::Remote, "Still nothing?", "Check remote features and Power options on the Xbox (the list on My consoles).", Some(btn("Show the list", None, B::Normal, false, 15.0, a.ring && a.focus == 2, Some(Msg::ShowList)))));
    let page = container(col).center_x(Length::Fill).center_y(Length::Fill).style(bx(DEEP, 0.0, None));
    frame(a, true, page.into(), vec![hint(&["A"], "Try again", 16.0), hint(&["X"], "Try at 720p", 16.0), hint(&["B"], "Back", 16.0)], vec![lbrb()])
}

/// A drop-down's choices: (value, words).
pub fn dd_choices(a: &App, d: Dd) -> Vec<(Value, String)> {
    match d {
        Dd::Limit => crp::prefs::LIMITS.iter().map(|g| (json!(g), format!("{g} GB"))).collect(),
        Dd::Warn => crp::prefs::WARNS.iter().map(|p| (json!(p), format!("{p} %"))).collect(),
        Dd::Sound => {
            let mut v = vec![(json!(""), if a.ally { "Ally's speakers".to_string() } else { "This PC's speakers".to_string() })];
            for (n, desc) in &a.sounds {
                v.push((json!(n), if desc.is_empty() { n.clone() } else { desc.clone() }));
            }
            v
        }
    }
}

pub fn dd_now(a: &App, d: Dd) -> Value {
    match d {
        Dd::Limit => json!(a.prefs.limit_gb),
        Dd::Warn => json!(a.prefs.warn_pct),
        Dd::Sound => json!(a.prefs.sound),
    }
}

pub fn sound_words(a: &App) -> String {
    if a.prefs.sound.is_empty() {
        return if a.ally { "Ally's speakers".into() } else { "This PC's speakers".into() };
    }
    a.sounds.iter().find(|(n, _)| *n == a.prefs.sound).map(|(_, d)| d.clone()).unwrap_or_else(|| a.prefs.sound.clone())
}

/// The chooser a drop-down opens.
fn chooser(a: &App, d: Dd) -> E<'_, Msg> {
    let now = dd_now(a, d);
    let title = match d {
        Dd::Limit => "Monthly data limit",
        Dd::Warn => "Warn me at",
        Dd::Sound => "Sound plays on",
    };
    let mut c = Column::new().spacing(6.0).push(one(t6(title, 18.0, TX))).push(gap(1.0, 6.0));
    for (v, words) in dd_choices(a, d) {
        let on = v == now;
        c = c.push(click(container(row![one(t(words, 16.0, if on { INK } else { TX })), fill()]).padding(pad(10.0, 14.0, 10.0, 14.0)).width(Length::Fill).style(bx(if on { ACC } else { CARD }, 9.0, None)), Some(Msg::Pick(d, v))));
    }
    c = c.push(gap(1.0, 6.0)).push(row![fill(), btn("Close", None, B::Normal, false, 15.0, false, Some(Msg::Dd(None)))]);
    hearth_ui::ui::dialog(Space::new().into(), container(c).padding(20.0).width(Length::Fixed(380.0)).style(shadowed(SIDE, 14.0, Some((LINE, 1.0)), 40.0)).into(), Some(Msg::Dd(None)))
}

fn limit_dialog(a: &App) -> E<'_, Msg> {
    let c = column![
        one(t6(format!("You've used your {} GB this month", a.prefs.limit_gb), 20.0, TX)),
        t(format!("{} used on every network since the {}. Stream anyway? It won't stop your game; you can change the limit in Settings.", data::gb(a.month_bytes()), ordinal(a.prefs.reset_day)), 15.0, MUT).line_height(1.45),
        row![fill(), btn("Not now", None, B::Normal, false, 15.0, false, Some(Msg::NotNow)), btn("Stream anyway", None, B::Primary, false, 15.0, true, Some(Msg::StreamAnyway))].spacing(10.0)
    ]
    .spacing(14.0);
    hearth_ui::ui::dialog(Space::new().into(), container(c).padding(22.0).width(Length::Fixed(520.0)).style(shadowed(SIDE, 14.0, Some((LINE, 1.0)), 40.0)).into(), Some(Msg::NotNow))
}

pub fn ordinal(d: u32) -> String {
    let s = match (d % 10, d % 100) {
        (1, x) if x != 11 => "st",
        (2, x) if x != 12 => "nd",
        (3, x) if x != 13 => "rd",
        _ => "th",
    };
    format!("{d}{s}")
}

fn settings(a: &App) -> E<'_, Msg> {
    let p = &a.prefs;
    let f = |i: usize| a.ring && a.focus == i;
    let row_ = |icon: G, title: &str, line: &str, end: E<'static, Msg>, focused: bool| -> E<'static, Msg> { srow(Some(ri(icon, 40.0)), title, if line.is_empty() { None } else { Some(rich(line, 14.0, MUT, SOFT, false)) }, Some(end), 17.0, CARD, pad(14.0, 20.0, 14.0, 20.0), focused) };
    let seg3 = |key: &'static str, opts: &[(&str, &str)], now: &str, focused: bool| -> E<'static, Msg> { seg(opts.iter().map(|(v, l)| (l.to_string(), *v == now, Msg::Set(key, json!(v)))).collect(), 15.0, 7.0, 15.0, focused) };
    let mut left = Column::new();
    left = left.push(sec("Picture", 6.0));
    left = left.push(row_(G::Stats, "Default quality", "Auto picks the sharpest your connection holds, and steps down when it slows.", seg3("quality", &[("auto", "Auto"), ("720p", "720p"), ("1080p", "1080p")], &p.quality, f(0)), false)).push(gap(1.0, 8.0));
    left = left.push(row_(G::Leaf, "Data saver", "720p and a lower bitrate: about 2–3 GB an hour instead of 4–6.", seg3("dataSaver", &[("off", "Off"), ("hotspot", "On a hotspot"), ("always", "Always")], &p.saver, f(1)), false));
    left = left.push(sec("Monthly data", 20.0));
    let (period, day, len) = data::period(data::today(), p.reset_day);
    let _ = period;
    let used = a.month_bytes();
    let limit = p.limit_gb as f64 * 1e9;
    let warn_at = limit * p.warn_pct as f64 / 100.0;
    let starts = if p.reset_day == 1 { "Starts again on the 1st.".to_string() } else { format!("Starts again on the {}.", ordinal(p.reset_day)) };
    let top = row![column![t("Monthly data limit", 17.0, TX), t(format!("Counted on every network: home, work Wi-Fi and your phone's hotspot. {starts}"), 14.0, MUT)].spacing(3.0).width(Length::Fill), toggle(p.limit_on, 1.0, f(2), Some(Msg::Set("limitOn", json!(!p.limit_on))))].align_y(iced::Center);
    let mut box_ = Column::new().push(top);
    if p.limit_on {
        box_ = box_.push(gap(1.0, 14.0)).push(
            row![
                one(t("Limit", 14.0, SOFT)),
                dd(&format!("{} GB", p.limit_gb), 120.0, f(3), Msg::Dd(Some(Dd::Limit))),
                gap(12.0, 1.0),
                one(t("Warn me at", 14.0, SOFT)),
                dd(&format!("{} %", p.warn_pct), 110.0, f(4), Msg::Dd(Some(Dd::Warn))),
                fill(),
                one(t(format!("Day {day} of {len}"), 14.0, MUT))
            ]
            .spacing(12.0)
            .align_y(iced::Center),
        );
        box_ = box_.push(gap(1.0, 14.0)).push(meter((used as f64 / limit) as f32, p.warn_pct as f32 / 100.0));
        box_ = box_.push(gap(1.0, 6.0)).push(row![rich(&format!("**{}** used this month", data::gb(used)), 14.0, MUT, SOFT, false), fill(), one(t(format!("warning at {} · limit {} GB", data::gb(warn_at as u64).replace(".0 GB", " GB"), p.limit_gb), 14.0, MUT))]);
        box_ = box_.push(gap(1.0, 8.0)).push(t("At the limit Hearth asks before streaming more; it never stops a game by itself.", 14.0, MUT));
    }
    left = left.push(ring(false, 12.0, container(row![ri(G::Data, 40.0), box_.width(Length::Fill)].spacing(16.0).align_y(iced::Top)).padding(pad(14.0, 20.0, 14.0, 20.0)).width(Length::Fill).style(bx(CARD, 12.0, None))));
    left = left.push(sec("Controller", 20.0));
    left = left.push(row_(G::Pad, "Play with", "Every button goes to your Xbox, except the menu combo.", seg3("playWith", &[("builtin", "Built-in controls (Ally)"), ("paired", "A paired controller")], &p.play_with, f(5)), false)).push(gap(1.0, 8.0));
    left = left.push(row_(G::Buzz, "Vibration", "Rumble from the game on this controller.", row![one(t(if p.vibration { "On" } else { "Off" }, 14.0, MUT)), toggle(p.vibration, 1.0, f(6), Some(Msg::Set("vibration", json!(!p.vibration))))].spacing(10.0).align_y(iced::Center).into(), false)).push(gap(1.0, 8.0));
    left = left.push(srow(Some(ri(G::Pad, 40.0)), "Open the menu while streaming", Some(rich("Hold **View + Menu** for 1 second. On a keyboard: **Ctrl+Shift+M**.", 14.0, MUT, SOFT, false)), None, 17.0, CARD, pad(14.0, 20.0, 14.0, 20.0), false));
    let mut right = Column::new();
    right = right.push(sec("Screen and sound", 6.0));
    right = right.push(row_(G::Full, "Start full screen", "Straight into the game when the picture starts.", toggle(p.full_screen, 1.0, f(7), Some(Msg::Set("startFullScreen", json!(!p.full_screen)))), false)).push(gap(1.0, 8.0));
    right = right.push(row_(G::Sun, "Keep the screen awake", "While streaming, the screen doesn't dim or lock.", toggle(p.keep_awake, 1.0, f(8), Some(Msg::Set("keepAwake", json!(!p.keep_awake)))), false)).push(gap(1.0, 8.0));
    right = right.push(row_(G::Speaker, "Sound plays on", "", dd(&sound_words(a), 270.0, f(9), Msg::Dd(Some(Dd::Sound))), false));
    right = right.push(sec("Account", 20.0));
    let initial = a.gamertag.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default();
    let avatar = container(one(t7(initial, 16.0, INK))).center_x(Length::Fixed(40.0)).center_y(Length::Fixed(40.0)).style(grad(135.0, ACC2, Color::from_rgb8(0x8a, 0x3b, 0x14), 20.0));
    right = right.push(container(row![avatar, column![t("Signed in as", 14.0, MUT), one(t6(a.gamertag.clone(), 17.0, TX))], fill(), btn("Sign out", None, B::Normal, false, 15.0, f(10), Some(Msg::SignOut))].spacing(16.0).align_y(iced::Center)).padding(pad(14.0, 20.0, 14.0, 20.0)).width(Length::Fill).style(bx(CARD, 12.0, None)));
    right = right.push(sec("Hearth Store", 20.0));
    let (chip_l, chip_ok) = &a.store_chip;
    let store = column![
        t("Hearth Console Remote Play is a Hearth Store app", 17.0, TX),
        t("It updates through the Store, on its own schedule, apart from Hearth OS updates. It isn't made by Microsoft.", 14.0, MUT).line_height(1.4),
        gap(1.0, 9.0),
        row![chip(chip_l, if *chip_ok { Ch::Ok } else { Ch::On }, None, 13.5), container(btn("Open in the Store", None, B::Normal, false, 15.0, f(11), Some(Msg::OpenStore)))].spacing(10.0).align_y(iced::Center)
    ]
    .spacing(3.0);
    right = right.push(container(row![ri(G::Store, 40.0), store.width(Length::Fill)].spacing(16.0).align_y(iced::Top)).padding(pad(14.0, 20.0, 14.0, 20.0)).width(Length::Fill).style(bx(CARD, 12.0, None))).push(gap(1.0, 8.0));
    right = right.push(container(t("Your choices are saved the moment you change one, and stay as you left them.", 14.0, MUT)).padding(pad(14.0, 20.0, 14.0, 20.0)).width(Length::Fill).style(bx(QUIET, 12.0, None)));
    if let Some(n) = note(a) {
        right = right.push(gap(1.0, 8.0)).push(n);
    }
    let body = column![h1("Settings"), row![left.width(Length::Fill), right.width(Length::Fixed(600.0))].spacing(28.0)];
    let page = container(scrollable(body)).padding(pad(26.0, 44.0, 26.0, 44.0)).width(Length::Fill).height(Length::Fill);
    frame(a, true, page.into(), vec![hint(&["A"], "Select", 16.0), hint(&["B"], "Back", 16.0), hint(&["◀▶"], "Change", 16.0)], vec![lbrb()])
}

pub fn tile<'a>(a: &'a App, i: usize, g: &crp::xhome::Game, art_h: f32, small: bool, focused: bool, cap_size: f32, line: bool) -> E<'a, Msg> {
    let wide = if small { a.wide.get(&g.picture) } else { None };
    let art: E<'a, Msg> = match wide.or_else(|| a.pics.get(&g.picture)) {
        Some(h) => image(h.clone()).content_fit(if wide.is_some() { iced::ContentFit::Fill } else { iced::ContentFit::Cover }).width(Length::Fill).height(Length::Fixed(art_h)).into(),
        None => container(Space::new()).width(Length::Fill).height(Length::Fixed(art_h)).style(bx(CARD2, 0.0, None)).into(),
    };
    let mut cap = Column::new().spacing(2.0).push(one(t6(g.name.clone(), cap_size, TX)));
    if line {
        let what = if small { format!("{} · in the cloud library", g.kind) } else { g.kind.clone() };
        cap = cap.push(one(t(what, 13.5, MUT)));
    }
    let inner = column![container(art).clip(true), container(cap).padding(pad(12.0, 14.0, 12.0, 14.0))];
    let c = ring0(focused, 14.0, CARD, container(inner).clip(true)).clip(true);
    // The library's other games sit back a little (the mockup's opacity .75): the page colour laid over them.
    let el: E<'a, Msg> = if small { stack![c, container(Space::new()).width(Length::Fill).height(Length::Fill).style(bx(a_bg(0.25), 14.0, None))].into() } else { c.into() };
    click(container(el).width(Length::Fill), Some(Msg::Play(i)))
}

fn cloud(a: &App) -> E<'_, Msg> {
    let head = row![h1("Cloud"), gap(16.0, 1.0), chip("Optional", Ch::On, None, 14.0), fill(), chip("Game Pass Ultimate", Ch::Ok, None, 13.5)].align_y(iced::Center);
    let tabs = seg(vec![("Recently played".into(), a.tab == CloudTab::Recent, Msg::CloudTab(CloudTab::Recent)), ("All cloud games".into(), a.tab == CloudTab::All, Msg::CloudTab(CloudTab::All)), ("Search".into(), a.tab == CloudTab::Search, Msg::CloudTab(CloudTab::Search))], 15.0, 7.0, 15.0, a.ring && a.focus == 0 && a.tab != CloudTab::Recent);
    let mut top = Row::new().spacing(12.0).align_y(iced::Center).push(tabs);
    if a.tab == CloudTab::Search {
        top = top.push(text_input("Search cloud games", &a.search).on_input(Msg::Search).size(15.0).padding(pad(9.0, 14.0, 9.0, 14.0)).width(Length::Fixed(360.0)));
    }
    let list = a.shown_games();
    let mut grid = Column::new().spacing(18.0);
    let state: Option<E<'_, Msg>> = match if a.tab == CloudTab::Recent { &a.games } else { &a.all_games } {
        Load::Loading | Load::Idle => Some(t("Getting the cloud games…", 16.0, MUT).into()),
        Load::Failed(e) => Some(t(e.clone(), 16.0, MUT).into()),
        _ if list.is_empty() => Some(t("Nothing here yet.", 16.0, MUT).into()),
        _ => None,
    };
    if let Some(s) = state {
        grid = grid.push(s);
    } else {
        // The first four large (Recently played's look), the rest smaller, as the mockup.
        let mut i = 0;
        while i < list.len() {
            let big = i < 4;
            let mut r = Row::new().spacing(18.0);
            for j in i..(i + 4) {
                let cell: E<'_, Msg> = match list.get(j) {
                    Some(g) => tile(a, j, g, if big { 250.0 } else { 130.0 }, !big, a.ring && a.focus == j + 1, 16.0, true),
                    None => Space::new().width(Length::Fill).into(),
                };
                r = r.push(container(cell).width(Length::FillPortion(1)));
            }
            grid = grid.push(r);
            i += 4;
        }
    }
    let foot = srow(Some(ri(G::Cloud, 40.0)), "", Some(t("Cloud games use the same picture, bar, quality and data settings as your Xbox. The names and pictures here are made up for the mockup.", 15.0, MUT).into()), None, 1.0, QUIET, pad(14.0, 20.0, 14.0, 20.0), false);
    let body = column![head, sub("Games that run on Microsoft's servers instead of your Xbox, if your account has Game Pass Ultimate. Your Xbox can stay asleep. Without it, this tab isn't shown."), top, gap(1.0, 18.0), grid, gap(1.0, 18.0), foot];
    let page = container(scrollable(body)).padding(pad(26.0, 40.0, 26.0, 40.0)).width(Length::Fill).height(Length::Fill);
    frame(a, true, page.into(), vec![hint(&["A"], "Play", 16.0), hint(&["B"], "Back", 16.0)], vec![lbrb()])
}

// ------------------------------------------------------------------ streaming (4, 4b)
fn src<'a>(b: String, s: &str, bcol: Color) -> E<'a, Msg> {
    column![one(t6(b, 15.0, bcol)), one(t(s.to_string(), 12.5, MUT))].spacing(1.0).into()
}

fn sep<'a>() -> E<'a, Msg> {
    vline(a(Color::WHITE, 0.12), 34.0)
}

fn grp<'a>(items: Vec<E<'a, Msg>>) -> E<'a, Msg> {
    let mut r = Row::new().spacing(10.0).align_y(iced::Center);
    for x in items {
        r = r.push(x);
    }
    container(r).padding(pad(0.0, 10.0, 0.0, 10.0)).center_y(Length::Fixed(44.0)).into()
}

fn vb<'a>(icon: G, label: Option<&str>, bg: Color, fg: Color, m: Msg) -> E<'a, Msg> {
    let mut r = Row::new().spacing(8.0).align_y(iced::Center).push(glyph(icon, fg, 21.0));
    if let Some(l) = label {
        r = r.push(one(t(l.to_string(), 14.5, fg)));
    }
    click(container(r).padding(pad(0.0, if label.is_some() { 12.0 } else { 10.0 }, 0.0, if label.is_some() { 12.0 } else { 10.0 })).center_y(Length::Fixed(44.0)).style(bx(bg, 10.0, None)), Some(m))
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

pub fn picture<'a>(a: &'a App) -> E<'a, Msg> {
    let pic: E<'a, Msg> = match a.frame.as_ref() {
        Some(h) => image(h.clone()).content_fit(iced::ContentFit::Contain).width(Length::Fill).height(Length::Fill).into(),
        None => Space::new().width(Length::Fill).height(Length::Fill).into(),
    };
    container(pic).center_x(Length::Fill).center_y(Length::Fill).style(bx(Color::BLACK, 0.0, None)).into()
}

fn stream_view(a: &App) -> E<'_, Msg> {
    let s = &a.stats;
    let (q, nbars) = s.quality();
    let qcol = match q {
        "Good" => Color::from_rgb8(0x8f, 0xf0, 0xb3),
        "Fair" => WARN,
        _ => BAD,
    };
    let (name, model) = a.conn.as_ref().map(|c| (c.name.clone(), c.model.clone())).unwrap_or(("Your Xbox".into(), String::new()));
    let mut layers: Vec<E<'_, Msg>> = vec![picture(a)];
    if a.pill && !a.menu {
        let pill = container(row![dotted(OK, 9.0), column![one(t(format!("{name} · streaming from home"), 15.0, TX)), one(t("Move the mouse to show the bar · F11 leaves full screen · your controller plays the Xbox", 13.0, MUT))].spacing(2.0)].spacing(12.0).align_y(iced::Center))
            .padding(pad(12.0, 18.0, 12.0, 18.0))
            .style(shadowed(Color { a: 0.92 * 0.91, ..Color::from_rgb8(0x12, 0x14, 0x19) }, 12.0, Some((a_w(0.11), 1.0)), 40.0));
        layers.push(container(pill).center_x(Length::Fill).padding(pad(26.0, 0.0, 0.0, 0.0)).into());
    }
    if a.warn_toast {
        let limit = a.prefs.limit_gb;
        let toast = container(
            row![
                glyph(G::Data, WARN, 24.0),
                column![
                    one(t6(format!("{} of your {} GB monthly streaming data used", data::gb(a.month_bytes()), limit), 15.0, TX)),
                    one(t("Data saver lowers the picture to 720p and uses about half as much.", 13.0, MUT)),
                    gap(1.0, 8.0),
                    row![btn("Turn on Data saver", None, B::Primary, false, 15.0, false, Some(Msg::SaverNow)), btn("Keep going", None, B::Normal, false, 15.0, false, Some(Msg::KeepGoing))].spacing(10.0)
                ]
                .spacing(2.0)
            ]
            .spacing(12.0)
            .align_y(iced::Top),
        )
        .padding(pad(12.0, 18.0, 12.0, 18.0))
        .style(shadowed(Color { a: 0.96, ..Color::from_rgb8(0x12, 0x14, 0x19) }, 12.0, Some((Color::from_rgb8(0x7a, 0x66, 0x33), 1.0)), 40.0));
        layers.push(container(toast).center_x(Length::Fill).padding(pad(26.0, 0.0, 0.0, 0.0)).into());
    }
    if a.bar && !a.menu {
        let res = s.resolution();
        let delay = s.delay_ms.map(|d| format!("{d} ms")).unwrap_or_else(|| "– ms".into());
        let bar = row![
            grp(vec![glyph(G::Console, ACC2, 24.0), src(name.clone(), &model, TX)]),
            sep(),
            grp(vec![bars(nbars, OK), src(q.to_string(), "connection", qcol)]),
            grp(vec![src(delay, "delay", TX), src(format!("{:.1} Mbps", s.mbps), "bitrate", TX), container(one(t7(res, 13.5, Color::from_rgb8(0x8f, 0xf0, 0xb3)))).padding(pad(4.0, 9.0, 4.0, 9.0)).style(bx(Color::from_rgb8(0x17, 0x35, 0x24), 7.0, None)).into()]),
            sep(),
            grp(vec![glyph(G::Data, MUT, 20.0), src(data::gb(s.session_bytes), "this session", TX)]),
            sep(),
            grp(vec![glyph(G::Vol, TX, 21.0), volume_slider(a.prefs.volume, 110.0), container(one(t(a.prefs.volume.to_string(), 14.0, MUT))).width(Length::Fixed(24.0)).into()]),
            sep(),
            grp(vec![one(t("Quality", 14.0, SOFT)).into(), seg(vec![("Auto".into(), a.prefs.quality == "auto", Msg::Set("quality", json!("auto"))), ("720p".into(), a.prefs.quality == "720p", Msg::Set("quality", json!("720p"))), ("1080p".into(), a.prefs.quality == "1080p", Msg::Set("quality", json!("1080p")))], 13.5, 6.0, 11.0, false)]),
            grp(vec![one(t("Data saver", 14.0, SOFT)).into(), toggle(a.prefs.saver == "always", 0.85, false, Some(Msg::Set("dataSaver", json!(if a.prefs.saver == "always" { "off" } else { "always" }))))]),
            sep(),
            vb(G::Cam, Some("Screenshot"), Color::TRANSPARENT, Color::from_rgb8(0xe3, 0xe6, 0xeb), Msg::Screenshot),
            vb(if a.full { G::Unfull } else { G::Full }, None, Color::TRANSPARENT, Color::from_rgb8(0xe3, 0xe6, 0xeb), Msg::FullScreen),
            vb(G::Gear, None, Color::TRANSPARENT, Color::from_rgb8(0xe3, 0xe6, 0xeb), Msg::Menu),
            mouse_area(vb(G::Exit, Some("Disconnect"), Color::from_rgb8(0x4a, 0x16, 0x16), Color::from_rgb8(0xff, 0xd0, 0xd0), Msg::Disconnect)).on_enter(Msg::Tip(true)).on_exit(Msg::Tip(false))
        ]
        .spacing(4.0)
        .align_y(iced::Center);
        let bar = container(bar).padding(pad(0.0, 10.0, 0.0, 16.0)).center_y(Length::Fixed(68.0)).style(shadowed(Color { a: 0.9, ..Color::from_rgb8(0x12, 0x14, 0x19) }, 16.0, Some((a_w(0.11), 1.0)), 50.0));
        layers.push(container(bar).center_x(Length::Fill).align_bottom(Length::Fill).padding(pad(0.0, 0.0, 26.0, 0.0)).into());
        if a.tip {
            // The tooltip over Disconnect (the bar is centred: Disconnect ends 10 px before its right edge).
            let tip = container(one(t6("Leave your Xbox on · it sleeps by itself later", 13.5, BG))).padding(pad(6.0, 11.0, 6.0, 11.0)).style(bx(TX, 7.0, None));
            let (w, h) = a.size;
            let bar_w = 1620.0f32.min(w - 20.0);
            let x = (w + bar_w) / 2.0 - 10.0 - 65.0 - 152.0;
            layers.push(pin(tip).x(x.max(0.0)).y((h - 26.0 - 68.0 - 40.0).max(0.0)).into());
        }
    }
    if let Some((p, h)) = &a.shot {
        let thumb = container(image(h.clone()).content_fit(iced::ContentFit::Cover).width(Length::Fixed(112.0)).height(Length::Fixed(63.0))).clip(true).style(bx(Color::BLACK, 7.0, None));
        let shown = p.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
        let toast = container(
            row![
                thumb,
                column![one(t6("Screenshot saved", 15.0, TX)), one(t(format!("Pictures › Screenshots › {shown}"), 13.0, MUT))].spacing(2.0),
                gap(10.0, 1.0),
                btn("Show", None, B::Normal, false, 15.0, false, Some(Msg::ShowShot)),
                btn("Copy", None, B::Normal, false, 15.0, false, Some(Msg::CopyShot))
            ]
            .spacing(12.0)
            .align_y(iced::Center),
        )
        .padding(pad(12.0, 18.0, 12.0, 18.0))
        .style(shadowed(Color { a: 0.91, ..Color::from_rgb8(0x12, 0x14, 0x19) }, 12.0, Some((a_w(0.11), 1.0)), 40.0));
        layers.push(container(toast).center_x(Length::Fill).align_bottom(Length::Fill).padding(pad(0.0, 0.0, if a.bar && !a.menu { 110.0 } else { 30.0 }, 0.0)).into());
    }
    if a.menu {
        layers.push(crate::ally::quick_menu(a, false));
    }
    if a.ask_limit.is_some() {
        layers.push(limit_dialog(a));
    }
    stack(layers).into()
}

fn a_bg(alpha: f32) -> Color {
    Color { a: alpha, ..BG }
}

fn a_w(alpha: f32) -> Color {
    Color { r: 1.0, g: 1.0, b: 1.0, a: alpha }
}
