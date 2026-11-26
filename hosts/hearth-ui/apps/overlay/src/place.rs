//! `hearth-overlay --place [--game FILE]`: "Drag it in the game to place it anywhere" (board 03, Where). MangoHud
//! can't be dragged itself, so Hearth puts a see-through window over the whole screen, over the game, with a ghost of
//! the overlay where MangoHud draws it. Drag the ghost (or nudge it with the arrows, the controller's D-pad): while
//! you drag, Hearth writes its spot to MangoHud's own file as MangoHud's position=top-left with offset_x / offset_y
//! (at most every 80 ms, with the safe writer: one line at a time, .bak kept; red flag 9), and MangoHud, which reads
//! its file again as it changes, moves the real overlay in the game under the ghost. Enter or Done keeps it, Esc
//! puts back what it was. Nothing runs once the window closes.

use crate::conf;
use hearth_oss_kit_a as k;
use hearth_ui::app;
use hearth_ui::ui::{self, t, tb};
use hearth_ui::Pal;
use iced::widget::{canvas, column, container, row, stack, Space};
use iced::{mouse, Color, Element, Length, Point, Rectangle, Renderer, Size, Subscription, Task, Theme};
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub enum M {
    Theme,
    Down(Point),
    Move(Point),
    Up,
    Nudge(f32, f32),
    Done,
    Undo,
    Key(iced::keyboard::Event),
    Sized(Size),
}

pub struct Place {
    p: Pal,
    file: PathBuf,
    /// the file as it was (Esc puts it back)
    was: Option<String>,
    pos: Point,
    size: Size,
    grab: Option<(Point, Point)>,
    wrote: Instant,
    said: String,
    /// moved yet (until then the ghost follows MangoHud's own spot on the real screen size)
    moved: bool,
    fixed: bool,
}

/// The overlay's size on screen, near enough for its ghost: the rows MangoHud shows at its font size.
pub fn ghost_size(text: &str) -> Size {
    let fs = conf::get(text, "font_size").and_then(|v| v.parse::<f32>().ok()).unwrap_or(24.0).clamp(8.0, 64.0);
    let one = conf::on(text, "horizontal");
    let rows = crate::SHOWS.iter().filter(|(_, ks)| conf::on(text, ks[0])).count().max(1) as f32;
    let graph = if conf::on(text, "frame_timing") { fs * 3.0 } else { 0.0 };
    if one {
        Size::new((fs * 7.5 * rows).min(1800.0), fs * 1.8)
    } else {
        Size::new(fs * 17.0, rows * fs * 1.55 + graph + fs)
    }
}

/// Where MangoHud draws it now, from its position and offsets, on a screen of `screen`.
pub fn spot(text: &str, screen: Size, g: Size) -> Point {
    let pos = conf::get(text, "position").unwrap_or_else(|| "top-left".into());
    let ox = conf::get(text, "offset_x").and_then(|v| v.parse::<f32>().ok()).unwrap_or(0.0);
    let oy = conf::get(text, "offset_y").and_then(|v| v.parse::<f32>().ok()).unwrap_or(0.0);
    let x = if pos.ends_with("right") { screen.width - g.width - ox } else if pos.ends_with("center") { (screen.width - g.width) / 2.0 + ox } else { ox };
    let y = if pos.starts_with("bottom") { screen.height - g.height - oy } else if pos.starts_with("middle") { (screen.height - g.height) / 2.0 + oy } else { oy };
    Point::new(x.max(0.0), y.max(0.0))
}

/// MangoHud's file with the overlay at `at` (top left, offsets in pixels).
pub fn placed(text: &str, at: Point) -> String {
    let x = format!("{}", at.x.max(0.0).round() as i32);
    let y = format!("{}", at.y.max(0.0).round() as i32);
    conf::set(&conf::set(&conf::set(text, "position", Some("top-left")), "offset_x", Some(&x)), "offset_y", Some(&y))
}

impl Place {
    fn boot() -> (Place, Task<M>) {
        let file = conf::dir().join(app::arg("--game").unwrap_or_else(|| "MangoHud.conf".into()));
        let was = std::fs::read_to_string(&file).ok();
        let text = was.clone().unwrap_or_default();
        let size = ghost_size(&text);
        let screen = Size::new(1920.0, 1080.0);
        let fixed = app::arg("--at").is_some();
        let pos = app::arg("--at").and_then(|s| s.split_once(',').and_then(|(a, b)| Some(Point::new(a.parse().ok()?, b.parse().ok()?)))).unwrap_or_else(|| spot(&text, screen, size));
        let p = Place { p: hearth_ui::theme::load(), file, was, pos, size, grab: None, wrote: Instant::now() - Duration::from_secs(1), said: String::new(), moved: false, fixed };
        (p, iced::window::latest().and_then(|id| iced::window::set_mode(id, iced::window::Mode::Fullscreen)))
    }
    /// The spot written to MangoHud's file (now, or `force`d: the last one of a drag).
    fn write(&mut self, force: bool) {
        self.moved = true;
        if !force && self.wrote.elapsed() < Duration::from_millis(80) {
            return;
        }
        self.wrote = Instant::now();
        let at = self.pos;
        let _ = std::fs::create_dir_all(conf::dir());
        let r = if self.file.exists() { k::settings_file::update_text(&self.file, |t| placed(t, at)).map(|_| ()) } else { k::settings_file::write_text(&self.file, &placed("# Hearth Overlay (MangoHud's settings)\nlegacy_layout=false\n", at)).map(|_| ()) };
        self.said = match r {
            Ok(()) => format!("{} × {} px from the top left", at.x.round(), at.y.round()),
            Err(e) => format!("Couldn't save it: {e}"),
        };
    }
}

fn update(a: &mut Place, m: M) -> Task<M> {
    match m {
        M::Theme => a.p = hearth_ui::theme::load(),
        M::Sized(sz) => {
            if !a.moved && !a.fixed {
                let text = a.was.clone().unwrap_or_default();
                a.pos = spot(&text, sz, a.size);
            }
        }
        M::Down(p) => {
            let r = Rectangle::new(a.pos, a.size);
            if r.contains(p) {
                a.grab = Some((p, a.pos));
            } else {
                // a press anywhere else puts it there
                a.pos = Point::new(p.x - a.size.width / 2.0, p.y - a.size.height / 2.0);
                a.grab = Some((p, a.pos));
                a.write(false);
            }
        }
        M::Move(p) => {
            if let Some((from, start)) = a.grab {
                a.pos = Point::new((start.x + p.x - from.x).max(0.0), (start.y + p.y - from.y).max(0.0));
                a.write(false);
            }
        }
        M::Up => {
            if a.grab.take().is_some() {
                a.write(true);
            }
        }
        M::Nudge(dx, dy) => {
            a.pos = Point::new((a.pos.x + dx).max(0.0), (a.pos.y + dy).max(0.0));
            a.write(true);
        }
        M::Done => return iced::exit(),
        M::Undo => {
            match &a.was {
                Some(t) => {
                    let _ = k::settings_file::write_text(&a.file, t);
                }
                None => {
                    let _ = std::fs::remove_file(&a.file);
                }
            }
            return iced::exit();
        }
        M::Key(ev) => {
            use iced::keyboard::{key::Named, Event as KE, Key};
            if let KE::KeyPressed { key, modifiers, .. } = ev {
                let step = if modifiers.shift() { 1.0 } else { 10.0 };
                return match key.as_ref() {
                    Key::Named(Named::ArrowLeft) => update(a, M::Nudge(-step, 0.0)),
                    Key::Named(Named::ArrowRight) => update(a, M::Nudge(step, 0.0)),
                    Key::Named(Named::ArrowUp) => update(a, M::Nudge(0.0, -step)),
                    Key::Named(Named::ArrowDown) => update(a, M::Nudge(0.0, step)),
                    Key::Named(Named::Enter) => update(a, M::Done),
                    Key::Named(Named::Escape) => update(a, M::Undo),
                    _ => Task::none(),
                };
            }
        }
    }
    Task::none()
}

struct Ghost<'a> {
    a: &'a Place,
}

impl<'a> canvas::Program<M> for Ghost<'a> {
    type State = ();
    fn update(&self, _s: &mut (), ev: &iced::Event, b: Rectangle, c: mouse::Cursor) -> Option<canvas::Action<M>> {
        match ev {
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => c.position_in(b).map(|p| canvas::Action::publish(M::Down(p)).and_capture()),
            iced::Event::Mouse(mouse::Event::CursorMoved { .. }) => self.a.grab.and(c.position_in(b)).map(|p| canvas::Action::publish(M::Move(p)).and_capture()),
            iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => Some(canvas::Action::publish(M::Up)),
            _ => None,
        }
    }
    fn draw(&self, _s: &(), r: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<canvas::Geometry> {
        let mut f = canvas::Frame::new(r, b.size());
        let a = self.a;
        // a faint veil and the screen's thirds, so where it goes is easy to see
        f.fill_rectangle(Point::ORIGIN, b.size(), Color { a: 0.18, ..Color::BLACK });
        let line = Color { a: 0.22, ..Color::WHITE };
        for i in 1..3 {
            let x = b.width * i as f32 / 3.0;
            let y = b.height * i as f32 / 3.0;
            f.stroke(&canvas::Path::line(Point::new(x, 0.0), Point::new(x, b.height)), canvas::Stroke::default().with_color(line).with_width(1.0));
            f.stroke(&canvas::Path::line(Point::new(0.0, y), Point::new(b.width, y)), canvas::Stroke::default().with_color(line).with_width(1.0));
        }
        // the ghost: where MangoHud draws the overlay
        let rr = canvas::Path::rounded_rectangle(a.pos, a.size, 10.0.into());
        f.fill(&rr, Color { a: if a.grab.is_some() { 0.32 } else { 0.22 }, ..a.p.accent });
        f.stroke(&rr, canvas::Stroke { line_dash: canvas::LineDash { segments: &[8.0, 5.0], offset: 0 }, ..canvas::Stroke::default().with_color(a.p.accent).with_width(2.0) });
        vec![f.into_geometry()]
    }
    fn mouse_interaction(&self, _s: &(), b: Rectangle, c: mouse::Cursor) -> mouse::Interaction {
        if self.a.grab.is_some() {
            mouse::Interaction::Grabbing
        } else if c.position_in(b).map(|p| Rectangle::new(self.a.pos, self.a.size).contains(p)).unwrap_or(false) {
            mouse::Interaction::Grab
        } else {
            mouse::Interaction::Crosshair
        }
    }
}

fn view(a: &Place) -> Element<'_, M> {
    let p = a.p;
    let card = container(
        column![
            tb("Place the overlay", 16.0, p.text),
            t("Drag the box to where the numbers go: the overlay in your game moves with it.", 13.0, p.muted),
            t("Arrows nudge it (Shift: one pixel) · Enter keeps it · Esc puts it back", 12.0, p.muted),
            row![t(a.said.clone(), 12.0, p.accent), Space::new().width(Length::Fill), k::chip(&p, "Put it back", k::Ch::Plain, k::Sz::Md, Some(M::Undo)), k::chip(&p, "Done", k::Ch::Acc, k::Sz::Md, Some(M::Done))].spacing(8.0).align_y(iced::Center),
        ]
        .spacing(8.0),
    )
    .padding(18.0)
    .width(Length::Fixed(520.0))
    .style(ui::boxed(Pal::a(p.panel, 0.96), 14.0, Some((p.line, 1.0))));
    let ghost = canvas::Canvas::new(Ghost { a }).width(Length::Fill).height(Length::Fill);
    stack![ghost, container(card).center_x(Length::Fill).center_y(Length::Fill)].into()
}

fn subscription(_a: &Place) -> Subscription<M> {
    Subscription::batch([Subscription::run(hearth_ui::theme::watch).map(|_| M::Theme), iced::keyboard::listen().map(M::Key), iced::window::resize_events().map(|(_, s)| M::Sized(s))])
}

pub fn run() -> iced::Result {
    // see-through over the game (hearth-ui's softbuffer with alpha, on tiny-skia, as Live captions)
    std::env::set_var("HEARTH_SOFTBUFFER_ALPHA", "1");
    std::env::set_var("ICED_BACKEND", "tiny-skia");
    let mut s = app::window("hearth-overlay-place", (1920.0, 1080.0), (640.0, 400.0));
    s.transparent = true;
    s.level = iced::window::Level::AlwaysOnTop;
    iced::application(Place::boot, update, view)
        .title(|_: &Place| "Place the overlay".to_string())
        .subscription(subscription)
        .theme(|a: &Place| app::iced_theme(&a.p))
        .style(|a: &Place, _t: &iced::Theme| iced::theme::Style { background_color: Color::TRANSPARENT, text_color: a.p.text })
        .window(s)
        .default_font(app::INTER)
        .run()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn the_spot_is_mangohuds_offsets() {
        let t = "fps\nposition=bottom-right\nfont_size=20\n";
        let g = Size::new(200.0, 100.0);
        let p = spot(t, Size::new(1920.0, 1080.0), g);
        assert_eq!((p.x, p.y), (1720.0, 980.0));
        let w = placed(t, Point::new(612.4, 40.0));
        assert!(w.contains("position=top-left\n") && w.contains("offset_x=612\n") && w.contains("offset_y=40\n"), "{w}");
        assert_eq!(spot(&w, Size::new(1920.0, 1080.0), g), Point::new(612.0, 40.0));
        assert!(ghost_size(t).width > 0.0);
    }
}
