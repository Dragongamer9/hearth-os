//! The pieces Hearth's open-source apps share (Hearth Lights, Remap, Mouse, Shield; boards
//! docs/mockups/open-source-apps): their window bar (the app's mark, its name and its line, its own buttons, the
//! window's buttons), the boards' small parts (switch, choices side by side, tags, chips, sliders, dots), the credit
//! line naming the open-source project and its licence, their settings file (hearth_ui::settings_file: atomic, a .bak,
//! merged, never defaults over a file that can't be read; red flag 9), work off the window's thread, and the "it isn't
//! here yet" card that offers to get or start the project the Hearth way.
//!
//! Included by path in each app (`#[path = "../../oss-kit/kit.rs"] mod kit;`), so it isn't a crate of its own.
#![allow(dead_code)]

use hearth_ui::ui::{self, t, tb, tm, ts};
use hearth_ui::Pal;
use iced::widget::{button, column, container, mouse_area, row, text, Column, Row, Space};
use iced::{mouse, Background, Border, Color, Element, Length, Padding, Shadow, Theme};
use serde_json::{Map, Value};

pub type E<'a, M> = Element<'a, M>;

// ------------------------------------------------------------------ the boards' colours (style.css)
pub const BLUE: Color = Color::from_rgb(0x5e as f32 / 255.0, 0xc8 as f32 / 255.0, 1.0);
pub const GREEN: Color = Color::from_rgb(0x9b as f32 / 255.0, 0xe3 as f32 / 255.0, 0x6b as f32 / 255.0);
pub const PINK: Color = Color::from_rgb(0xe3 as f32 / 255.0, 0x6b as f32 / 255.0, 0xd3 as f32 / 255.0);
pub const RED: Color = Color::from_rgb(1.0, 0x6b as f32 / 255.0, 0x6b as f32 / 255.0);
pub const YELLOW: Color = Color::from_rgb(1.0, 0xd3 as f32 / 255.0, 0x6b as f32 / 255.0);
pub const PURPLE: Color = Color::from_rgb(0xb0 as f32 / 255.0, 0x4f as f32 / 255.0, 0xc8 as f32 / 255.0);
pub const ROSE: Color = Color::from_rgb(1.0, 0x3d as f32 / 255.0, 0x6a as f32 / 255.0);

pub fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgb8(r, g, b)
}

/// "#ff8a3d" → a colour.
pub fn hex(s: &str) -> Option<Color> {
    let s = s.trim().trim_start_matches('#');
    if s.len() != 6 || !s.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let v = u32::from_str_radix(s, 16).ok()?;
    Some(Color::from_rgb8((v >> 16) as u8, (v >> 8) as u8, v as u8))
}
pub fn to_hex(c: Color) -> String {
    let [r, g, b, _] = c.into_rgba8();
    format!("#{r:02X}{g:02X}{b:02X}")
}

pub fn pad(t: f32, r: f32, b: f32, l: f32) -> Padding {
    Padding { top: t, right: r, bottom: b, left: l }
}

// ------------------------------------------------------------------ small parts
/// The window's title bar as the boards draw it: the app's mark (its letters on its gradient), its name, its line
/// ("· every light in your PC, one place"), the app's own buttons on the right, then Minimize, Maximize and Close.
/// Dragging the bar moves the window, a double click maximizes, a right-click opens Describe it (every Hearth app).
#[allow(clippy::too_many_arguments)]
pub fn title_bar<'a, M: Clone + 'a>(p: &Pal, letters: &'a str, from: Color, to: Color, name: &'a str, line: &'a str, right: Vec<E<'a, M>>, m: ui::TitleMsgs<M>, describe: M) -> E<'a, M> {
    let p = *p;
    let mark = container(tb(letters, 10.0, Color::WHITE).wrapping(text::Wrapping::None)).center_x(Length::Fixed(22.0)).center_y(Length::Fixed(22.0)).style(gradient(135.0, from, to, 7.0, None));
    let drag = |e: E<'a, M>| -> E<'a, M> { mouse_area(e).on_press(m.drag.clone()).on_double_click(m.maximize.clone()).on_right_press(describe.clone()).into() };
    let name_row = row![mark, ts(name, 14.0, p.text).wrapping(text::Wrapping::None), t(format!("· {line}"), 14.0, p.muted).wrapping(text::Wrapping::None)].spacing(10.0).align_y(iced::Center);
    let mut r = Row::new().align_y(iced::Center).push(drag(container(name_row).padding(pad(0.0, 8.0, 0.0, 16.0)).center_y(Length::Fixed(44.0)).into()));
    r = r.push(drag(container(Space::new()).width(Length::Fill).height(Length::Fixed(44.0)).into()));
    for x in right {
        r = r.push(container(x).padding(pad(0.0, 6.0, 0.0, 0.0)));
    }
    let wbtn = |g: ui::G, msg: M, red: bool| -> E<'a, M> {
        button(container(ui::glyph(g, p.muted, 10.0, 2.2)).center_x(Length::Fixed(40.0)).center_y(Length::Fixed(44.0)))
            .padding(0.0)
            .on_press(msg)
            .style(move |_: &Theme, st: button::Status| button::Style {
                background: Some(Background::Color(match st {
                    button::Status::Hovered | button::Status::Pressed if red => Color::from_rgb8(0xc4, 0x2b, 0x1c),
                    button::Status::Hovered | button::Status::Pressed => p.hover(),
                    _ => Color::TRANSPARENT,
                })),
                text_color: p.text,
                border: Border::default(),
                shadow: Shadow::default(),
                snap: true,
            })
            .into()
    };
    if let Some(min) = m.minimize.clone() {
        r = r.push(wbtn(ui::G::Minimize, min, false));
        r = r.push(wbtn(ui::G::Maximize, m.maximize.clone(), false));
    }
    r = r.push(wbtn(ui::G::Close, m.close.clone(), true));
    if let Some(fm) = ui::frame_meter() {
        r = r.push(fm);
    }
    column![
        container(r).width(Length::Fill).height(Length::Fixed(44.0)).style(ui::boxed(Pal::mix(p.bg, Color::BLACK, 0.12), 0.0, None)),
        ui::hline(p.line)
    ]
    .into()
}

/// A container with a CSS-style gradient (angle in degrees).
pub fn gradient(deg: f32, a: Color, b: Color, r: f32, line: Option<(Color, f32)>) -> impl Fn(&Theme) -> container::Style {
    move |_| {
        let g = iced::gradient::Linear::new(iced::Radians((deg - 90.0).to_radians())).add_stop(0.0, a).add_stop(1.0, b);
        container::Style {
            text_color: None,
            background: Some(Background::Gradient(g.into())),
            border: match line {
                Some((c, w)) => Border { color: c, width: w, radius: r.into() },
                None => iced::border::rounded(r),
            },
            shadow: Shadow::default(),
            snap: true,
        }
    }
}

/// A gradient through several colours (left to right), for bars and strips.
pub fn gradient_n(stops: Vec<Color>, r: f32) -> impl Fn(&Theme) -> container::Style {
    move |_| {
        let mut g = iced::gradient::Linear::new(iced::Radians(std::f32::consts::FRAC_PI_2));
        let n = stops.len().max(2) - 1;
        for (i, c) in stops.iter().enumerate().take(8) {
            g = g.add_stop(i as f32 / n as f32, *c);
        }
        container::Style { text_color: None, background: Some(Background::Gradient(g.into())), border: iced::border::rounded(r), shadow: Shadow::default(), snap: true }
    }
}

/// The small uppercase section title ("DEVICES · 9 FOUND").
pub fn caps<'a, M: 'a>(p: &Pal, s: impl Into<String>) -> E<'a, M> {
    tb(s.into().to_uppercase(), 11.0, p.muted).wrapping(text::Wrapping::None).into()
}

/// A press anywhere on `e` sends `m` (with the pointer hand).
pub fn click<'a, M: Clone + 'a>(e: impl Into<E<'a, M>>, m: Option<M>) -> E<'a, M> {
    match m {
        Some(m) => mouse_area(e).on_press(m).interaction(mouse::Interaction::Pointer).into(),
        None => e.into(),
    }
}

/// The boards' switch (`.sw`): the accent with the knob on the right when on.
pub fn switch<'a, M: Clone + 'a>(p: &Pal, on: bool, m: Option<M>) -> E<'a, M> {
    let knob = container(Space::new()).width(Length::Fixed(14.0)).height(Length::Fixed(14.0)).style(ui::boxed(Color::WHITE, 7.0, None));
    let inner: E<'a, M> = if on { row![Space::new().width(Length::Fill), knob].into() } else { row![knob, Space::new().width(Length::Fill)].into() };
    let track = container(inner).padding(3.0).width(Length::Fixed(34.0)).height(Length::Fixed(20.0)).style(ui::boxed(if on { p.accent } else { p.line }, 10.0, None));
    click(track, m)
}

/// Choices side by side (`.seg`): the picked one in the accent.
pub fn seg<'a, M: Clone + 'a>(p: &Pal, items: Vec<(String, bool, Option<M>)>) -> E<'a, M> {
    let p = *p;
    let mut r = Row::new();
    for (l, on, m) in items {
        let c = container(if on { ts(l, 12.0, p.ink) } else { t(l, 12.0, p.muted) }.wrapping(text::Wrapping::None))
            .padding(pad(5.0, 9.0, 5.0, 9.0))
            .style(ui::boxed(if on { p.accent } else { Color::TRANSPARENT }, 0.0, None));
        r = r.push(click(c, m));
    }
    container(r).clip(true).style(ui::boxed(Color::TRANSPARENT, 8.0, Some((p.line, 1.0)))).into()
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tag {
    Acc,
    Dark,
    Ok,
    Blue,
    Warn,
    Bad,
}

/// The boards' tags (`.tag`): IN STEP, SAVED TO THE DEVICES, PROFILE: GAMING, ON…
pub fn tag<'a, M: 'a>(p: &Pal, s: impl Into<String>, k: Tag) -> E<'a, M> {
    let (bg, fg, line) = match k {
        Tag::Acc => (p.accent, p.ink, p.accent),
        Tag::Dark => (p.panel2, p.muted, p.line),
        Tag::Ok => (rgb(0x21, 0x36, 0x1b), GREEN, rgb(0x3b, 0x5a, 0x2e)),
        Tag::Blue => (rgb(0x17, 0x30, 0x40), BLUE, rgb(0x2d, 0x56, 0x70)),
        Tag::Warn => (rgb(0x3d, 0x2a, 0x14), YELLOW, rgb(0x6a, 0x4a, 0x1e)),
        Tag::Bad => (rgb(0x3d, 0x17, 0x17), RED, rgb(0x6a, 0x2a, 0x2a)),
    };
    container(tb(s.into().to_uppercase(), 11.0, fg).wrapping(text::Wrapping::None)).padding(pad(3.0, 8.0, 3.0, 8.0)).style(ui::boxed(bg, 6.0, Some((line, 1.0)))).into()
}

/// A small chip button (`.chip.sm`).
pub fn chip<'a, M: Clone + 'a>(p: &Pal, s: impl Into<String>, m: Option<M>) -> E<'a, M> {
    let p = *p;
    let fg = if m.is_some() { p.text } else { p.muted };
    let c = container(tm(s.into(), 12.0, fg).wrapping(text::Wrapping::None)).padding(pad(6.0, 10.0, 6.0, 10.0)).style(ui::boxed(p.panel2, 8.0, Some((p.line, 1.0))));
    click(c, m)
}
/// The accent chip (`.chip.acc`).
pub fn chip_acc<'a, M: Clone + 'a>(p: &Pal, s: impl Into<String>, m: Option<M>) -> E<'a, M> {
    let c = container(ts(s.into(), 12.0, p.ink).wrapping(text::Wrapping::None)).padding(pad(6.0, 12.0, 6.0, 12.0)).style(ui::boxed(p.accent, 8.0, None));
    click(c, m)
}

/// A round dot of colour (`.ic.r`, a light's colour, a device's colour).
pub fn dot<'a, M: 'a>(c: Color, size: f32) -> E<'a, M> {
    container(Space::new()).width(Length::Fixed(size)).height(Length::Fixed(size)).style(ui::boxed(c, size / 2.0, None)).into()
}
/// A rounded square of colour (`.ic`, `.dot`).
pub fn square<'a, M: 'a>(c: Color, size: f32, r: f32, line: Option<(Color, f32)>) -> E<'a, M> {
    container(Space::new()).width(Length::Fixed(size)).height(Length::Fixed(size)).style(ui::boxed(c, r, line)).into()
}

/// The boards' card (`.card`): the panel, a border, 12 px corners, 16 px inside.
pub fn card<'a, M: 'a>(p: &Pal, inner: impl Into<E<'a, M>>) -> container::Container<'a, M> {
    container(inner).padding(16.0).width(Length::Fill).style(ui::boxed(p.panel, 12.0, Some((p.line, 1.0))))
}

/// The slider in the boards' look (`.sl`): the accent rail and a white knob.
pub fn slider_style(p: &Pal) -> impl Fn(&Theme, iced::widget::slider::Status) -> iced::widget::slider::Style {
    use iced::widget::slider;
    let p = *p;
    move |_, _| slider::Style {
        rail: slider::Rail { backgrounds: (Background::Color(p.accent), Background::Color(p.line)), width: 4.0, border: Border { color: Color::TRANSPARENT, width: 0.0, radius: 2.0.into() } },
        handle: slider::Handle { shape: slider::HandleShape::Circle { radius: 8.0 }, background: Background::Color(Color::WHITE), border_width: 0.0, border_color: Color::TRANSPARENT },
    }
}

/// A text field in the boards' look (`.field`).
pub fn field_style(p: &Pal) -> impl Fn(&Theme, iced::widget::text_input::Status) -> iced::widget::text_input::Style {
    use iced::widget::text_input;
    let p = *p;
    move |_, st| text_input::Style {
        background: Background::Color(p.bg),
        border: Border { color: if matches!(st, text_input::Status::Focused { .. }) { p.accent } else { p.line }, width: 1.0, radius: 8.0.into() },
        icon: p.muted,
        placeholder: p.muted,
        value: p.text,
        selection: Pal::a(p.accent, 0.35),
    }
}

/// A row: words on the left, something on the right (`.row`).
pub fn line<'a, M: 'a>(p: &Pal, left: impl Into<String>, right: E<'a, M>) -> E<'a, M> {
    row![t(left.into(), 13.0, p.text), Space::new().width(Length::Fill), right].spacing(10.0).align_y(iced::Center).into()
}

/// The credit (every board's bottom right): "Hearth Lights · Built on OpenRGB (GPL-2)".
pub fn credit<'a, M: 'a>(p: &Pal, app: &'a str, built_on: &'a str) -> E<'a, M> {
    container(row![Space::new().width(Length::Fill), t(format!("{app} · "), 11.0, p.muted), tb(format!("Built on {built_on}"), 11.0, Pal::mix(p.muted, p.text, 0.4))].align_y(iced::Center))
        .padding(pad(5.0, 22.0, 6.0, 22.0))
        .width(Length::Fill)
        .into()
}

/// The card shown where the open-source project isn't there or isn't running: what's missing, in words, and the
/// button that gets or starts it the Hearth way (or nothing, when only an update can bring it).
pub fn missing<'a, M: Clone + 'a>(p: &Pal, title: impl Into<String>, words: impl Into<String>, button: Option<(String, M)>, busy: bool) -> E<'a, M> {
    let mut c = Column::new().spacing(8.0).push(ts(title.into(), 15.0, p.text)).push(t(words.into(), 13.0, p.muted));
    if let Some((label, m)) = button {
        c = c.push(row![ui::btn(p, label, ui::Kind::Primary, false, if busy { None } else { Some(m) })]);
    }
    container(c).padding(16.0).width(Length::Fill).style(ui::boxed(Pal::a(p.accent, 0.08), 12.0, Some((Pal::a(p.accent, 0.5), 1.0)))).into()
}

// ------------------------------------------------------------------ settings (red flag 9)
fn config_dir() -> std::path::PathBuf {
    std::env::var_os("XDG_CONFIG_HOME").map(std::path::PathBuf::from).unwrap_or_else(|| hearth_ui::paths::home().join(".config")).join("hearth")
}
/// The app's own settings file: ~/.config/hearth/<name>.json.
pub fn settings_path(name: &str) -> std::path::PathBuf {
    config_dir().join(format!("{name}.json"))
}
/// What's kept (an empty object when nothing is; a damaged file is put back from its .bak by settings_file and never
/// written over with defaults).
pub fn read_settings(name: &str) -> Value {
    hearth_ui::settings_file::read_json(&settings_path(name))
}
/// Only these keys changed, merged into the file under its lock (atomic, a .bak first). Nothing else is touched.
pub fn save(name: &str, changes: Vec<(&str, Value)>) -> Result<(), String> {
    let path = settings_path(name);
    if let Some(d) = path.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let mut m = Map::new();
    for (k, v) in changes {
        m.insert(k.to_string(), v);
    }
    hearth_ui::settings_file::merge_json(&path, &m).map(|_| ()).map_err(|e| e.to_string())
}

// ------------------------------------------------------------------ work off the window's thread
/// `f` on its own thread; ready when it's done (None if it panicked).
pub fn bg<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> impl std::future::Future<Output = Option<T>> + Send + 'static {
    let (tx, rx) = futures::channel::oneshot::channel();
    let _ = std::thread::Builder::new().name("hearth-oss".into()).spawn(move || {
        let _ = tx.send(f());
    });
    async move { rx.await.ok() }
}

/// Is this program on the PATH (or an absolute path that's there)?
pub fn have(cmd: &str) -> bool {
    if cmd.starts_with('/') {
        return std::path::Path::new(cmd).exists();
    }
    std::env::var_os("PATH").map(|ps| std::env::split_paths(&ps).any(|d| d.join(cmd).is_file())).unwrap_or(false)
}

/// The system keyring (the Secret Service through secret-tool, as hearth_accounts.py keeps passwords): the secret
/// goes in on stdin, never on a command line, never in a file, never in a log.
pub mod keyring {
    use std::io::Write;
    use std::process::{Command, Stdio};

    fn tool() -> String {
        std::env::var("HEARTH_SECRET_TOOL").unwrap_or_else(|_| "secret-tool".into())
    }
    pub fn store(app: &str, key: &str, label: &str, secret: &str) -> Result<(), String> {
        let mut c = Command::new(tool())
            .args(["store", &format!("--label={label}"), "hearth-app", app, "key", key])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|_| "The keyring (secret-tool) isn't here.".to_string())?;
        if let Some(mut i) = c.stdin.take() {
            let _ = i.write_all(secret.as_bytes());
        }
        let o = c.wait_with_output().map_err(|e| e.to_string())?;
        if o.status.success() { Ok(()) } else { Err("The keyring is locked or not running.".into()) }
    }
    pub fn lookup(app: &str, key: &str) -> Option<String> {
        let o = Command::new(tool()).args(["lookup", "hearth-app", app, "key", key]).stdin(Stdio::null()).stderr(Stdio::null()).output().ok()?;
        let s = String::from_utf8_lossy(&o.stdout).trim_end_matches('\n').to_string();
        (o.status.success() && !s.is_empty()).then_some(s)
    }
    pub fn forget(app: &str, key: &str) {
        let _ = Command::new(tool()).args(["clear", "hearth-app", app, "key", key]).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status();
    }
}

/// A user service of Hearth's (systemctl --user): started, stopped, kept on for the next sign-in or not. Every one
/// of these is guarded in its unit (nothing waits for it, it gives up after a few tries), so it can't hold up Game
/// Mode or the desktop.
pub fn user_unit(verb: &str, unit: &str) -> bool {
    std::process::Command::new("systemctl").args(["--user", verb, "--no-block", unit]).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).status().map(|s| s.success()).unwrap_or(false)
}
pub fn user_unit_active(unit: &str) -> bool {
    std::process::Command::new("systemctl").args(["--user", "is-active", "--quiet", unit]).stdin(std::process::Stdio::null()).status().map(|s| s.success()).unwrap_or(false)
}

/// Is a game being played (gamescope's Game Mode or Hearth's hearth-playing mark)?
pub fn game_running() -> bool {
    std::env::var_os("XDG_RUNTIME_DIR").map(|d| std::path::Path::new(&d).join("hearth-playing").exists()).unwrap_or(false)
}

/// A whole number with thousands commas: 48,213.
pub fn commas(n: u64) -> String {
    let s = n.to_string();
    let mut o = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            o.push(',');
        }
        o.push(c);
    }
    o
}
