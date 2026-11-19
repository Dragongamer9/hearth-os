//! CaptureBtn.qml in Rust: Capture's button. A text, a drawn picture (glyph.rs: a tool picture or a Hearth icon), or
//! both; primary (the accent), selected (a tool or a choice that's on), dimmed (still says why when pressed), big, or
//! a colour swatch. The controller's focus ring (an accent line and a soft glow) when the keyboard / D-pad is on it.

use crate::glyph;
use hearth_ui::app::INTER;
use hearth_ui::ui::weight;
use hearth_ui::Pal;
use iced::widget::{button, container, row, text, Space};
use iced::{font, Background, Border, Color, Element, Length, Padding, Shadow, Theme};

#[derive(Clone)]
pub struct B {
    pub text: String,
    pub glyph: &'static str,
    pub primary: bool,
    pub selected: bool,
    pub enabled_look: bool,
    pub big: bool,
    pub swatch: Option<Color>,
    pub focused: bool,
    pub fill: bool,
    pub height: Option<f32>,
}

pub fn b(text: impl Into<String>) -> B {
    B { text: text.into(), glyph: "", primary: false, selected: false, enabled_look: true, big: false, swatch: None, focused: false, fill: false, height: None }
}

impl B {
    pub fn glyph(mut self, g: &'static str) -> B {
        self.glyph = g;
        self
    }
    pub fn primary(mut self) -> B {
        self.primary = true;
        self
    }
    pub fn selected(mut self, on: bool) -> B {
        self.selected = on;
        self
    }
    pub fn dim(mut self, enabled: bool) -> B {
        self.enabled_look = enabled;
        self
    }
    pub fn big(mut self) -> B {
        self.big = true;
        self
    }
    pub fn swatch(mut self, c: Color) -> B {
        self.swatch = Some(c);
        self
    }
    pub fn focus(mut self, on: bool) -> B {
        self.focused = on;
        self
    }
    /// As wide as its place (Layout.fillWidth).
    pub fn fill(mut self) -> B {
        self.fill = true;
        self
    }
    pub fn height(mut self, h: f32) -> B {
        self.height = Some(h);
        self
    }

    pub fn view<'a, M: Clone + 'a>(self, p: &Pal, on: M) -> Element<'a, M> {
        let p = *p;
        let en = self.enabled_look;
        let dim = move |c: Color| if en { c } else { Color { a: c.a * 0.45, ..c } };
        let h = self.height.unwrap_or(if self.big { 52.0 } else { 38.0 });
        let r = p.r(9.0).max(6.0);
        let (primary, selected, swatch) = (self.primary, self.selected, self.swatch);
        let base = match swatch {
            Some(c) => c,
            None if primary => p.accent,
            None if selected => Pal::a(p.accent, 0.22),
            None => p.panel2,
        };
        let hover_bg = match swatch {
            Some(c) => c,
            None if primary => Pal::mix(p.accent, Color::WHITE, 0.08),
            None if selected => Pal::a(p.accent, 0.30),
            None => p.panel3,
        };
        let border_c = if selected { p.accent } else if swatch.is_some() { Color { r: 1.0, g: 1.0, b: 1.0, a: 0.25 } } else { p.line };
        let border_w = if selected || swatch.is_some() { 2.0 } else { 1.0 };
        let fg = if primary { p.ink } else { p.text };
        let gc = if primary { p.ink } else if selected { p.accent } else { p.text };
        let mut inner = row![].spacing(8.0).align_y(iced::Center);
        if !self.glyph.is_empty() {
            inner = inner.push(glyph::glyph_on(self.glyph, dim(gc), dim(base), if self.big { 22.0 } else { 18.0 }));
        }
        if !self.text.is_empty() {
            let wgt = if primary || selected { font::Weight::Semibold } else { font::Weight::Medium };
            inner = inner.push(text(self.text.clone()).size(if self.big { 16.0 } else { 14.0 }).color(dim(fg)).font(weight(wgt)).wrapping(text::Wrapping::None));
        }
        let pad_x = if swatch.is_some() { 0.0 } else if self.text.is_empty() { 9.0 } else { 14.0 };
        let content: Element<'a, M> = if swatch.is_some() {
            Space::new().width(Length::Fixed(34.0)).height(Length::Fixed(h)).into()
        } else {
            inner.into()
        };
        let mut c = container(content).center_y(Length::Fixed(h)).padding(Padding { top: 0.0, right: pad_x, bottom: 0.0, left: pad_x });
        c = if self.fill { c.center_x(Length::Fill) } else { c };
        let focused = self.focused;
        let bt = button(c)
            .padding(0.0)
            .on_press(on)
            .style(move |_: &Theme, st: button::Status| {
                let hov = matches!(st, button::Status::Hovered | button::Status::Pressed);
                let bg = if hov { hover_bg } else { base };
                button::Style {
                    background: Some(Background::Color(dim(bg))),
                    text_color: fg,
                    border: Border { color: if focused { p.accent } else { dim(border_c) }, width: if focused { 2.0 } else { border_w }, radius: r.into() },
                    // The controller's focus ring (SetBase.qml's soft glow), drawn outside so nothing moves.
                    shadow: if focused { Shadow { color: Pal::a(p.accent, 0.55), offset: iced::Vector::new(0.0, 0.0), blur_radius: 10.0 } } else { Shadow::default() },
                    snap: true,
                }
            });
        if self.fill { bt.width(Length::Fill).into() } else { bt.into() }
    }
}

/// Inter text, for labels outside buttons.
pub fn label<'a>(s: impl text::IntoFragment<'a>, size: f32, c: Color, semi: bool) -> iced::widget::Text<'a> {
    let f = if semi { weight(font::Weight::Semibold) } else { INTER };
    text(s).size(size).color(c).font(f)
}
