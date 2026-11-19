//! The area picker (Capture.qml's): a still picture of the screen, dimmed outside the box you drag; the box's
//! corners, its size by it, what to do at the top, and Take / Cancel by the box. Drag a new box, drag inside it to
//! move it, double-click to take. The D-pad / arrows move it (Shift: a little), X / Tab / Space switches to sizing
//! it, Enter (A) takes, Esc (B) cancels.

use crate::btn::{b, label};
use crate::draw::{self, R};
use crate::glyph::glyph;
use crate::{App, Msg};
use hearth_ui::Pal;
use iced::keyboard::{key::Named, Key, Modifiers};
use iced::widget::canvas::{self, Frame, Geometry, Path, Stroke};
use iced::widget::image::FilterMethod;
use iced::widget::{container, pin, row, stack, Space};
use iced::{mouse, Color, Element, Length, Padding, Pixels, Point, Rectangle, Renderer, Size, Task, Theme};
use std::time::Instant;

struct Picker<'a> {
    a: &'a App,
}

#[derive(Default)]
struct PickState {
    pressed: bool,
    last: Option<Instant>,
}

impl<'a> canvas::Program<Msg> for Picker<'a> {
    type State = PickState;

    fn update(&self, st: &mut PickState, event: &canvas::Event, b: Rectangle, cursor: mouse::Cursor) -> Option<canvas::Action<Msg>> {
        match event {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let q = cursor.position_in(b)?;
                let now = Instant::now();
                let double = st.last.map(|t| now.duration_since(t).as_millis() < 400).unwrap_or(false);
                st.last = Some(now);
                if double {
                    st.pressed = false;
                    return Some(canvas::Action::publish(Msg::PickDone).and_capture());
                }
                st.pressed = true;
                Some(canvas::Action::publish(Msg::PickPress(q.x, q.y, b.width, b.height)).and_capture())
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { position }) if st.pressed => {
                Some(canvas::Action::publish(Msg::PickMove(position.x - b.x, position.y - b.y, b.width, b.height)).and_capture())
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) if st.pressed => {
                st.pressed = false;
                Some(canvas::Action::publish(Msg::PickRelease).and_capture())
            }
            _ => None,
        }
    }

    fn draw(&self, _st: &PickState, renderer: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        let a = self.a;
        let p = a.p;
        let mut f = Frame::new(renderer, b.size());
        let (w, h) = (b.width, b.height);
        // The still picture is the image under this canvas (a canvas draws its images over its shapes).
        // Dimmed outside the box (all of it when there's no box).
        let dim = Color::from_rgba8(5, 7, 10, 0x8c as f32 / 255.0);
        match a.sel {
            None => f.fill_rectangle(Point::ORIGIN, b.size(), dim),
            Some(s) => {
                f.fill_rectangle(Point::ORIGIN, Size::new(w, s.y), dim);
                f.fill_rectangle(Point::new(0.0, s.y + s.h), Size::new(w, (h - s.y - s.h).max(0.0)), dim);
                f.fill_rectangle(Point::new(0.0, s.y), Size::new(s.x, s.h), dim);
                f.fill_rectangle(Point::new(s.x + s.w, s.y), Size::new((w - s.x - s.w).max(0.0), s.h), dim);
                f.stroke(&Path::rectangle(Point::new(s.x - 1.0, s.y - 1.0), Size::new(s.w + 2.0, s.h + 2.0)), Stroke::default().with_color(p.accent).with_width(2.0));
                for (cx, cy) in [(s.x - 1.0, s.y - 1.0), (s.x + s.w + 1.0, s.y - 1.0), (s.x - 1.0, s.y + s.h + 1.0), (s.x + s.w + 1.0, s.y + s.h + 1.0)] {
                    let dot = Path::circle(Point::new(cx, cy), 6.0);
                    f.fill(&dot, p.accent);
                    f.stroke(&dot, Stroke::default().with_color(Color::WHITE).with_width(2.0));
                }
                // The size, by the box.
                let t = format!("{} × {}{}", s.w.round(), s.h.round(), if a.sel_sizing { "  ·  sizing" } else { "" });
                let tw = t.chars().count() as f32 * 7.6 + 20.0;
                let x = (w - tw - 8.0).min(s.x).max(0.0);
                let y = if s.y > 40.0 { s.y - 28.0 - 8.0 } else { (h - 28.0 - 8.0).min(s.y + s.h + 8.0) };
                f.fill(&Path::rounded_rectangle(Point::new(x, y), Size::new(tw, 28.0), 14.0.into()), Pal::a(p.panel, 0.95));
                f.fill_text(canvas::Text {
                    content: t,
                    position: Point::new(x + tw / 2.0, y + 14.0),
                    color: p.text,
                    size: Pixels(13.0),
                    font: hearth_ui::ui::weight(iced::font::Weight::Semibold),
                    align_x: iced::widget::text::Alignment::Center,
                    align_y: iced::alignment::Vertical::Center,
                    ..canvas::Text::default()
                });
            }
        }
        vec![f.into_geometry()]
    }

    fn mouse_interaction(&self, _st: &PickState, _b: Rectangle, _c: mouse::Cursor) -> mouse::Interaction {
        mouse::Interaction::Crosshair
    }
}

pub fn view(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    let cv = canvas::Canvas::new(Picker { a }).width(Length::Fill).height(Length::Fill);
    // What to do, at the top.
    let hint = if a.pick_hint.is_empty() { "Drag around what you want".to_string() } else { a.pick_hint.clone() };
    let top = container(
        row![glyph("area", p.accent, 20.0), label(hint, 15.0, p.text, true), label("D-pad move  ·  X size  ·  A take  ·  B cancel", 13.0, p.muted, false)]
            .spacing(10.0)
            .align_y(iced::Center),
    )
    .padding(Padding { top: 0.0, right: 18.0, bottom: 0.0, left: 18.0 })
    .center_y(Length::Fixed(46.0))
    .style(hearth_ui::ui::boxed(Pal::a(p.panel, 0.95), 23.0, Some((p.line, 1.0))));
    let base: Element<'_, Msg> = match a.still.as_ref() {
        Some(still) => iced::widget::image(still.handle.clone()).content_fit(iced::ContentFit::Fill).filter_method(FilterMethod::Linear).width(Length::Fill).height(Length::Fill).into(),
        None => container(Space::new()).width(Length::Fill).height(Length::Fill).style(hearth_ui::ui::boxed(Color::BLACK, 0.0, None)).into(),
    };
    let mut layers = stack![base, cv, container(top).center_x(Length::Fill).padding(Padding { top: 28.0, right: 0.0, bottom: 0.0, left: 0.0 })].width(Length::Fill).height(Length::Fill);
    // Take and Cancel by the box (not while dragging it).
    if let Some(s) = a.sel {
        if a.pick_drag.is_none() && s.w >= 4.0 {
            let (w, h) = a.pick_size;
            let (bw, bh) = (180.0, 38.0);
            let x = (s.x + s.w - bw).min(w - bw - 8.0).max(8.0);
            let y = if s.y + s.h + 56.0 < h { s.y + s.h + 10.0 } else { (s.y + s.h - bh - 10.0).max(8.0) };
            let btns = row![b("Cancel").view(&p, Msg::PickCancel), b("Take").glyph("area").primary().view(&p, Msg::PickDone)].spacing(8.0);
            layers = layers.push(pin(btns).x(x).y(y));
        }
    }
    layers.into()
}

pub fn key(a: &mut App, key: &Key, m: Modifiers) -> Task<Msg> {
    let (w, h) = a.pick_size;
    let step = if m.shift() { 4.0 } else { 24.0 };
    let c = a.sel.unwrap_or_else(|| draw::start_sel(w, h));
    let (dx, dy) = match key.as_ref() {
        Key::Named(Named::ArrowLeft) => (-step, 0.0),
        Key::Named(Named::ArrowRight) => (step, 0.0),
        Key::Named(Named::ArrowUp) => (0.0, -step),
        Key::Named(Named::ArrowDown) => (0.0, step),
        _ => (0.0, 0.0),
    };
    if dx != 0.0 || dy != 0.0 {
        a.sel = Some(if a.sel_sizing { draw::resize_sel(c, dx, dy, w, h) } else { draw::move_sel(c, dx, dy, w, h) });
        return Task::none();
    }
    let ch = match key.as_ref() {
        Key::Character(s) => s.to_lowercase(),
        _ => String::new(),
    };
    match key.as_ref() {
        Key::Named(Named::ContextMenu) | Key::Named(Named::Tab) | Key::Named(Named::Space) => {
            if a.sel.is_none() {
                a.sel = Some(c);
            }
            a.sel_sizing = !a.sel_sizing;
        }
        Key::Named(Named::Enter) => {
            if a.sel.is_none() {
                a.sel = Some(c);
            } else {
                return Task::done(Msg::PickDone);
            }
        }
        Key::Named(Named::Escape) | Key::Named(Named::BrowserBack) | Key::Named(Named::GoBack) => return Task::done(Msg::PickCancel),
        _ if ch == "x" => {
            if a.sel.is_none() {
                a.sel = Some(c);
            }
            a.sel_sizing = !a.sel_sizing;
        }
        _ => {}
    }
    let _ = R::new(0.0, 0.0, 0.0, 0.0);
    Task::none()
}
