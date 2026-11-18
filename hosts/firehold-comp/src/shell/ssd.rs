//! Hearth's own window frames (server-side decorations): a title bar in Mantel's colours (theme.rs) with the window's
//! title and minimize, maximize and close buttons, drawn on the CPU into one picture per window and redrawn only when
//! something in it changes (size, focus, hover, title, colours). Started from Anvil's header bar.

use smithay::{
    backend::{
        allocator::Fourcc,
        input::ButtonState,
        renderer::{
            element::{
                memory::{MemoryRenderBuffer, MemoryRenderBufferRenderElement},
                Kind,
            },
            ImportMem, Renderer,
        },
    },
    desktop::WindowSurface,
    input::{pointer::ButtonEvent, Seat},
    utils::{IsAlive, Logical, Point, Rectangle, Serial, Transform},
    wayland::shell::xdg::XdgShellHandler,
};

use std::{
    cell::{RefCell, RefMut},
    sync::OnceLock,
    time::Instant,
};

use crate::{state::Backend, theme, HearthState};

use super::WindowElement;

pub struct WindowState {
    pub is_ssd: bool,
    pub active: bool,
    pub header_bar: HeaderBar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    Minimize,
    Maximize,
    Close,
    /// Firehold's game chip, "Full screen: F11 or ≡" (frames.rs).
    Fullscreen,
}

#[derive(Debug, Clone, PartialEq)]
struct DrawKey {
    width: i32,
    scale: i32,
    active: bool,
    hover: Option<Button>,
    pressed: Option<Button>,
    title: String,
    serial: u64,
    /// Firehold's look (frames.rs): its serial and this window's label, None for Hearth's plain frames.
    firehold: Option<(u64, crate::frames::Label)>,
}

/// Where Firehold's bar takes clicks (set when it's drawn).
#[derive(Debug, Clone, PartialEq)]
struct FhHits {
    game: bool,
    chip: (f64, f64),
}

#[derive(Debug, Clone)]
pub struct HeaderBar {
    pub pointer_loc: Option<Point<f64, Logical>>,
    pub width: u32,
    pressed: Option<Button>,
    last_click: Option<Instant>,
    drawn: Option<DrawKey>,
    buffer: Option<MemoryRenderBuffer>,
    stale: bool,
    fh: Option<FhHits>,
    /// A game's accent border (one pixel, stretched), with its colour.
    border: Option<(MemoryRenderBuffer, [u8; 4])>,
}

pub const HEADER_BAR_HEIGHT: i32 = 34;
/// The invisible resize border around a Hearth frame (logical pixels), and how far along it a corner reaches.
pub const BORDER: i32 = 6;
const CORNER: f64 = 18.0;

/// Which edges a point (relative to the window, its frame included) resizes, if it's on the border around it.
pub fn edge_at(
    geo: Rectangle<i32, Logical>,
    p: Point<f64, Logical>,
) -> Option<super::ResizeEdge> {
    use super::ResizeEdge as E;
    let (x0, y0) = (geo.loc.x as f64, geo.loc.y as f64);
    let (x1, y1) = (x0 + geo.size.w as f64, y0 + geo.size.h as f64);
    let b = BORDER as f64;
    if p.x < x0 - b || p.x >= x1 + b || p.y < y0 - b || p.y >= y1 + b {
        return None;
    }
    let inside = p.x >= x0 && p.x < x1 && p.y >= y0 && p.y < y1;
    if inside {
        return None;
    }
    let mut e = E::empty();
    if p.x < x0 + CORNER.min(geo.size.w as f64 / 3.0) && (p.x < x0 || p.y < y0 || p.y >= y1) {
        e |= E::LEFT;
    }
    if p.x >= x1 - CORNER.min(geo.size.w as f64 / 3.0) && (p.x >= x1 || p.y < y0 || p.y >= y1) {
        e |= E::RIGHT;
    }
    if p.y < y0 + CORNER.min(geo.size.h as f64 / 3.0) && (p.y < y0 || p.x < x0 || p.x >= x1) {
        e |= E::TOP;
    }
    if p.y >= y1 - CORNER.min(geo.size.h as f64 / 3.0) && (p.y >= y1 || p.x < x0 || p.x >= x1) {
        e |= E::BOTTOM;
    }
    (!e.is_empty()).then_some(e)
}
const BUTTON_WIDTH: i32 = 40;

impl HeaderBar {
    fn new() -> Self {
        HeaderBar {
            pointer_loc: None,
            width: 0,
            pressed: None,
            last_click: None,
            drawn: None,
            buffer: None,
            stale: false,
            fh: None,
            border: None,
        }
    }

    pub fn pointer_enter(&mut self, loc: Point<f64, Logical>) {
        self.pointer_loc = Some(loc);
    }

    pub fn pointer_leave(&mut self) {
        self.pointer_loc = None;
        self.pressed = None;
    }

    /// Mantel's colours changed: drawn again next frame.
    pub fn theme_changed(&mut self) {
        self.stale = true;
    }

    fn button_at(&self, x: f64) -> Option<Button> {
        let w = self.width as f64;
        if let Some(h) = &self.fh {
            if h.game {
                return (x >= h.chip.0 && x < h.chip.1).then_some(Button::Fullscreen);
            }
            let c = crate::frames::dot_centres(w);
            let half = (crate::frames::DOT + 10.0) / 2.0;
            return [Button::Minimize, Button::Maximize, Button::Close]
                .into_iter()
                .zip(c)
                .find(|(_, cx)| (x - cx).abs() < half)
                .map(|(b, _)| b);
        }
        let b = BUTTON_WIDTH as f64;
        if x >= w - b {
            Some(Button::Close)
        } else if x >= w - 2.0 * b {
            Some(Button::Maximize)
        } else if x >= w - 3.0 * b {
            Some(Button::Minimize)
        } else {
            None
        }
    }

    fn hover(&self) -> Option<Button> {
        self.pointer_loc.and_then(|l| self.button_at(l.x))
    }

    /// A pointer button on the title bar: pressing the title moves the window (twice quickly: maximize or back);
    /// a button acts when it's let go over the same button.
    pub fn clicked<BackendData: Backend + 'static>(
        &mut self,
        seat: &Seat<HearthState<BackendData>>,
        state: &mut HearthState<BackendData>,
        window: &WindowElement,
        event: &ButtonEvent,
    ) {
        let Some(loc) = self.pointer_loc else { return };
        let under = self.button_at(loc.x);
        if event.state == ButtonState::Pressed {
            self.pressed = under;
            if under.is_some() {
                return;
            }
            let now = Instant::now();
            let double = self.last_click.is_some_and(|t| now.duration_since(t).as_millis() < 400);
            self.last_click = Some(now);
            if double {
                self.last_click = None;
                let w = window.clone();
                state.handle.insert_idle(move |data| data.toggle_maximized(&w));
                return;
            }
            start_move(seat, state, window, event.serial);
            return;
        }
        let pressed = self.pressed.take();
        if pressed.is_none() || pressed != under {
            return;
        }
        let w = window.clone();
        match under {
            Some(Button::Close) => state.handle.insert_idle(move |data| data.close_window(&w)),
            Some(Button::Maximize) => state.handle.insert_idle(move |data| data.toggle_maximized(&w)),
            Some(Button::Minimize) => state.handle.insert_idle(move |data| data.minimize_window(&w)),
            Some(Button::Fullscreen) => state.handle.insert_idle(move |data| data.set_fullscreen(&w, true, None)),
            None => return,
        };
    }

    pub fn touch_down<BackendData: Backend + 'static>(
        &mut self,
        seat: &Seat<HearthState<BackendData>>,
        state: &mut HearthState<BackendData>,
        window: &WindowElement,
        serial: Serial,
    ) {
        let Some(loc) = self.pointer_loc else { return };
        self.pressed = self.button_at(loc.x);
        if self.pressed.is_none() {
            start_move(seat, state, window, serial);
        }
    }

    pub fn touch_up<BackendData: Backend + 'static>(
        &mut self,
        _seat: &Seat<HearthState<BackendData>>,
        state: &mut HearthState<BackendData>,
        window: &WindowElement,
        _serial: Serial,
    ) {
        let pressed = self.pressed.take();
        let under = self.pointer_loc.and_then(|l| self.button_at(l.x));
        if pressed.is_none() || pressed != under {
            return;
        }
        let w = window.clone();
        match under {
            Some(Button::Close) => state.handle.insert_idle(move |data| data.close_window(&w)),
            Some(Button::Maximize) => state.handle.insert_idle(move |data| data.toggle_maximized(&w)),
            Some(Button::Minimize) => state.handle.insert_idle(move |data| data.minimize_window(&w)),
            Some(Button::Fullscreen) => state.handle.insert_idle(move |data| data.set_fullscreen(&w, true, None)),
            None => return,
        };
    }

    /// Draws the bar again when what it shows changed. `scale`: the screen's scale, rounded up.
    pub fn redraw(&mut self, width: u32, scale: i32, active: bool, title: &str, id: u64) {
        self.width = width;
        if width == 0 {
            return;
        }
        let t = theme::current();
        let firehold = crate::frames::with(|l| l.firehold.then_some(l.serial))
            .map(|serial| (serial, crate::frames::label_for(id, title)));
        let key = DrawKey {
            firehold,
            width: width as i32,
            scale: scale.max(1),
            active,
            hover: self.hover(),
            pressed: self.pressed,
            title: title.to_string(),
            serial: t.serial,
        };
        if !self.stale && self.drawn.as_ref() == Some(&key) && self.buffer.is_some() {
            return;
        }
        self.stale = false;
        let (w, h) = (key.width * key.scale, HEADER_BAR_HEIGHT * key.scale);
        let (pixels, opaque) = match &key.firehold {
            Some((_, label)) => {
                let c = crate::frames::with(|l| l.colours.clone());
                let (px, chip) = draw_firehold_bar(&key, label, &c);
                self.fh = Some(FhHits { game: label.game, chip });
                let a = c.accent;
                let rgba = [to_u8(a[2]), to_u8(a[1]), to_u8(a[0]), 255];
                if label.game && self.border.as_ref().map(|b| b.1) != Some(rgba) {
                    self.border = Some((
                        MemoryRenderBuffer::from_slice(&rgba, Fourcc::Argb8888, (1, 1), 1, Transform::Normal, None),
                        rgba,
                    ));
                }
                if !label.game {
                    self.border = None;
                }
                // The rounded top corners show what's behind.
                let r = FH_RADIUS * key.scale;
                (px, Rectangle::new((0, r).into(), (w, (h - r).max(0)).into()))
            }
            None => {
                self.fh = None;
                self.border = None;
                (draw_bar(&key, &t), Rectangle::from_size((w, h).into()))
            }
        };
        self.buffer = Some(MemoryRenderBuffer::from_slice(
            &pixels,
            Fourcc::Argb8888,
            (w, h),
            key.scale,
            Transform::Normal,
            Some(vec![opaque]),
        ));
        self.drawn = Some(key);
    }

    /// Firehold's game border, 2 px all round the window (its title bar included): `full` is the frame's size
    /// (logical), `location` its top left (physical). Empty unless it's a game in Firehold's look.
    pub fn border_elements<R, C>(
        &self,
        renderer: &mut R,
        location: Point<i32, smithay::utils::Physical>,
        full: (i32, i32),
        scale: f64,
        alpha: f32,
    ) -> Vec<C>
    where
        R: Renderer + ImportMem,
        R::TextureId: Send + Clone + 'static,
        C: From<MemoryRenderBufferRenderElement<R>>,
    {
        let Some((buf, _)) = self.border.as_ref() else { return Vec::new() };
        if !self.fh.as_ref().is_some_and(|h| h.game) {
            return Vec::new();
        }
        let (w, h) = full;
        let b = 2;
        let sides = [(-b, -b, w + 2 * b, b), (-b, 0, b, h), (w, 0, b, h), (-b, h, w + 2 * b, b)];
        sides
            .into_iter()
            .filter_map(|(x, y, sw, sh)| {
                let at = location.to_f64() + Point::<f64, smithay::utils::Physical>::from((x as f64 * scale, y as f64 * scale));
                MemoryRenderBufferRenderElement::from_buffer(
                    renderer,
                    at,
                    buf,
                    Some(alpha),
                    None,
                    Some((sw, sh).into()),
                    Kind::Unspecified,
                )
                .ok()
                .map(C::from)
            })
            .collect()
    }

    pub fn render_elements<R, C>(&self, renderer: &mut R, location: Point<i32, smithay::utils::Physical>, alpha: f32) -> Vec<C>
    where
        R: Renderer + ImportMem,
        R::TextureId: Send + Clone + 'static,
        C: From<MemoryRenderBufferRenderElement<R>>,
    {
        let Some(buffer) = self.buffer.as_ref() else { return Vec::new() };
        match MemoryRenderBufferRenderElement::from_buffer(
            renderer,
            location.to_f64(),
            buffer,
            Some(alpha),
            None,
            None,
            Kind::Unspecified,
        ) {
            Ok(e) => vec![C::from(e)],
            Err(_) => Vec::new(),
        }
    }
}

fn start_move<BackendData: Backend + 'static>(
    seat: &Seat<HearthState<BackendData>>,
    state: &mut HearthState<BackendData>,
    window: &WindowElement,
    serial: Serial,
) {
    match window.0.underlying_surface() {
        WindowSurface::Wayland(w) => {
            let seat = seat.clone();
            let toplevel = w.clone();
            state
                .handle
                .insert_idle(move |data| data.move_request_xdg(&toplevel, &seat, serial));
        }
        #[cfg(feature = "xwayland")]
        WindowSurface::X11(w) => {
            let window = w.clone();
            state.handle.insert_idle(move |data| data.move_request_x11(&window));
        }
    };
}

impl<BackendData: Backend + 'static> HearthState<BackendData> {
    /// Resizing by a Hearth frame's border (the same grab apps start with xdg_toplevel.resize).
    pub fn start_frame_resize(&mut self, window: &WindowElement, edges: super::ResizeEdge, serial: Serial, button: u32) {
        use smithay::{input::pointer::{Focus, GrabStartData}, wayland::compositor::with_states};
        if !window.alive() || self.lock.locked {
            return;
        }
        let Some(loc) = self.space.element_location(window) else { return };
        let size = smithay::desktop::space::SpaceElement::geometry(window).size;
        let pointer = self.pointer.clone();
        if let Some(surface) = window.wl_surface() {
            with_states(&surface, |states| {
                if let Some(d) = states.data_map.get::<RefCell<super::SurfaceData>>() {
                    d.borrow_mut().resize_state = super::ResizeState::Resizing(super::ResizeData {
                        edges,
                        initial_window_location: loc,
                        initial_window_size: size,
                    });
                }
            });
        }
        // A snapped window resized by hand isn't snapped any more (dragging it away keeps this size).
        if let Some(r) = window.user_data().get::<crate::wm::Restore>() {
            r.0.borrow_mut().take();
        }
        let grab = super::PointerResizeSurfaceGrab {
            start_data: GrabStartData { focus: None, button, location: pointer.current_location() },
            window: window.clone(),
            edges,
            initial_window_location: loc,
            initial_window_size: size,
            last_window_size: size,
        };
        pointer.set_grab(self, grab, serial, Focus::Clear);
    }

    pub fn toggle_maximized(&mut self, w: &WindowElement) {
        let maximized = match w.0.underlying_surface() {
            WindowSurface::Wayland(t) => t.current_state().states.contains(
                smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel::State::Maximized,
            ),
            #[cfg(feature = "xwayland")]
            WindowSurface::X11(x) => x.is_maximized(),
        };
        if !maximized && self.unsnap(w) {
            return;
        }
        match w.0.underlying_surface() {
            WindowSurface::Wayland(t) => {
                if maximized {
                    self.unmaximize_request(t.clone())
                } else {
                    self.maximize_request(t.clone())
                }
            }
            #[cfg(feature = "xwayland")]
            WindowSurface::X11(_) => self.set_maximized(w, !maximized),
        }
    }
}

// ---- Drawing (Argb8888: bytes B, G, R, A).

fn to_u8(c: f32) -> u8 {
    (c.clamp(0.0, 1.0) * 255.0 + 0.5) as u8
}

pub(crate) struct Canvas {
    pub(crate) w: i32,
    pub(crate) h: i32,
    pub(crate) px: Vec<u8>,
}

impl Canvas {
    /// A canvas that's see-through until something is painted on it.
    pub(crate) fn clear(w: i32, h: i32) -> Canvas {
        Canvas { w, h, px: vec![0u8; (w.max(0) * h.max(0) * 4) as usize] }
    }

    pub(crate) fn new(w: i32, h: i32, bg: theme::Rgba) -> Canvas {
        let mut px = vec![0u8; (w * h * 4) as usize];
        for p in px.chunks_exact_mut(4) {
            p[0] = to_u8(bg[2]);
            p[1] = to_u8(bg[1]);
            p[2] = to_u8(bg[0]);
            p[3] = 255;
        }
        Canvas { w, h, px }
    }

    /// Paints `c` over a pixel with coverage `a` (0..1).
    pub(crate) fn blend(&mut self, x: i32, y: i32, c: theme::Rgba, a: f32) {
        if x < 0 || y < 0 || x >= self.w || y >= self.h || a <= 0.0 {
            return;
        }
        // Premultiplied "over" (what the renderer expects): the same as before on an opaque canvas, and right on
        // Firehold's transparent corners.
        let a = (a * c[3]).clamp(0.0, 1.0);
        let i = ((y * self.w + x) * 4) as usize;
        for (k, ch) in [(0usize, 2usize), (1, 1), (2, 0)] {
            let dst = self.px[i + k] as f32 / 255.0;
            self.px[i + k] = to_u8(c[ch] * a + dst * (1.0 - a));
        }
        let da = self.px[i + 3] as f32 / 255.0;
        self.px[i + 3] = to_u8(a + da * (1.0 - a));
    }

    pub(crate) fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: theme::Rgba) {
        for yy in y.max(0)..(y + h).min(self.h) {
            for xx in x.max(0)..(x + w).min(self.w) {
                self.blend(xx, yy, c, 1.0);
            }
        }
    }

    /// A rectangle with rounded corners (radius r), anti-aliased.
    pub(crate) fn round_rect(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, c: theme::Rgba) {
        for yy in (y.floor() as i32)..((y + h).ceil() as i32) {
            for xx in (x.floor() as i32)..((x + w).ceil() as i32) {
                let (px, py) = (xx as f32 + 0.5, yy as f32 + 0.5);
                let cx = px.clamp(x + r, x + w - r);
                let cy = py.clamp(y + r, y + h - r);
                let d = ((px - cx).powi(2) + (py - cy).powi(2)).sqrt() - r;
                self.blend(xx, yy, c, (0.5 - d).clamp(0.0, 1.0));
            }
        }
    }

    /// A line from (x0, y0) to (x1, y1), `t` thick, anti-aliased.
    pub(crate) fn line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, t: f32, c: theme::Rgba) {
        let (minx, maxx) = (x0.min(x1) - t, x0.max(x1) + t);
        let (miny, maxy) = (y0.min(y1) - t, y0.max(y1) + t);
        let (dx, dy) = (x1 - x0, y1 - y0);
        let len2 = (dx * dx + dy * dy).max(1e-6);
        for yy in (miny.floor() as i32)..=(maxy.ceil() as i32) {
            for xx in (minx.floor() as i32)..=(maxx.ceil() as i32) {
                let (px, py) = (xx as f32 + 0.5, yy as f32 + 0.5);
                let u = (((px - x0) * dx + (py - y0) * dy) / len2).clamp(0.0, 1.0);
                let (qx, qy) = (x0 + u * dx, y0 + u * dy);
                let d = ((px - qx).powi(2) + (py - qy).powi(2)).sqrt();
                self.blend(xx, yy, c, (t / 2.0 + 0.5 - d).clamp(0.0, 1.0));
            }
        }
    }
}

/// The font for titles: Mantel's (theme.json's "font"), else the usual sans fonts, found with fontconfig.
pub(crate) fn font() -> Option<&'static fontdue::Font> {
    static FONT: OnceLock<Option<fontdue::Font>> = OnceLock::new();
    FONT.get_or_init(|| {
        let family = theme::current().font;
        let mut tries: Vec<String> = Vec::new();
        if let Ok(p) = std::env::var("FIREHOLD_FONT") {
            tries.push(p);
        }
        for fam in [family.as_str(), "Inter", "Noto Sans", "DejaVu Sans", "sans-serif"] {
            if fam.is_empty() {
                continue;
            }
            if let Ok(out) = std::process::Command::new("fc-match")
                .args(["-f", "%{file}", &format!("{fam}:weight=medium")])
                .output()
            {
                let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !p.is_empty() {
                    tries.push(p);
                }
            }
        }
        for p in [
            "/usr/share/fonts/rsms-inter-fonts/Inter.ttc",
            "/usr/share/fonts/google-noto/NotoSans-Regular.ttf",
            "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
            "/usr/share/fonts/dejavu-sans-fonts/DejaVuSans.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        ] {
            tries.push(p.to_string());
        }
        for p in tries {
            // fontdue reads TrueType/OpenType (the first face of a collection).
            if let Ok(bytes) = std::fs::read(&p) {
                if let Ok(f) = fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default()) {
                    tracing::info!(font = %p, "frames: titles in this font");
                    return Some(f);
                }
            }
        }
        tracing::warn!("frames: no font found, titles aren't drawn");
        None
    })
    .as_ref()
}

fn draw_text(cv: &mut Canvas, text: &str, size: f32, x: f32, max_w: f32, center_in: Option<f32>, c: theme::Rgba) -> f32 {
    let Some(f) = font() else { return 0.0 };
    let glyphs: Vec<(fontdue::Metrics, Vec<u8>)> = text.chars().map(|ch| f.rasterize(ch, size)).collect();
    let ell = f.rasterize('…', size);
    let mut total: f32 = glyphs.iter().map(|(m, _)| m.advance_width).sum();
    // Too long: cut with "…".
    let mut n = glyphs.len();
    if total > max_w {
        while n > 0 && glyphs[..n].iter().map(|(m, _)| m.advance_width).sum::<f32>() + ell.0.advance_width > max_w {
            n -= 1;
        }
        total = glyphs[..n].iter().map(|(m, _)| m.advance_width).sum::<f32>() + ell.0.advance_width;
    }
    let lm = f.horizontal_line_metrics(size);
    let (ascent, descent) = lm.map(|l| (l.ascent, l.descent)).unwrap_or((size * 0.8, -size * 0.2));
    let baseline = ((cv.h as f32 - (ascent - descent)) / 2.0 + ascent).round();
    let mut pen = match center_in {
        Some(cw) => (x + (cw - total) / 2.0).max(x),
        None => x,
    };
    let draw = |cv: &mut Canvas, m: &fontdue::Metrics, bmp: &[u8], pen: f32| {
        let gx = (pen + m.xmin as f32).round() as i32;
        let gy = (baseline - m.height as f32 - m.ymin as f32).round() as i32;
        for row in 0..m.height {
            for col in 0..m.width {
                let a = bmp[row * m.width + col] as f32 / 255.0;
                cv.blend(gx + col as i32, gy + row as i32, c, a);
            }
        }
    };
    for (m, bmp) in &glyphs[..n] {
        draw(cv, m, bmp, pen);
        pen += m.advance_width;
    }
    if n < glyphs.len() {
        draw(cv, &ell.0, &ell.1, pen);
    }
    total
}

/// A line of text's width (logical pixels at `size`).
fn text_width(text: &str, size: f32) -> f32 {
    font().map(|f| text.chars().map(|ch| f.metrics(ch, size).advance_width).sum()).unwrap_or(text.chars().count() as f32 * size * 0.55)
}

/// Firehold's bar's corner radius (logical pixels).
const FH_RADIUS: i32 = 12;

/// The chip's left and right edges (logical pixels) on a bar this wide.
const CHIP_WORDS: &str = "Full screen: F11 or ";
/// The Guide button's mark "≡", 12 px text's width of it (drawn as three lines when the font has no such glyph).
const MENU_MARK_W: f32 = 9.0;

fn chip_span(width: f64) -> (f64, f64) {
    let w = (text_width(CHIP_WORDS, 12.0) + MENU_MARK_W) as f64 + 20.0;
    let right = width - 14.0;
    (right - w, right)
}

/// Firehold's bar (frames.rs): its pixels and the game chip's span (logical).
fn draw_firehold_bar(k: &DrawKey, label: &crate::frames::Label, c: &crate::frames::Colours) -> (Vec<u8>, (f64, f64)) {
    let s = k.scale as f32;
    let (w, h) = (k.width * k.scale, HEADER_BAR_HEIGHT * k.scale);
    let mut cv = Canvas::clear(w, h);
    let r = FH_RADIUS as f32 * s;
    // Rounded at the top only: the lower half is square, where the window meets it.
    cv.round_rect(0.0, 0.0, w as f32, h as f32, r, c.panel2);
    cv.rect(0, h / 2, w, h - h / 2, c.panel2);
    if !label.game {
        cv.rect(0, h - k.scale, w, k.scale, c.line);
    }
    let ink = if k.active { c.text } else { c.muted };
    let chip = chip_span(k.width as f64);
    let right_room = if label.game { (k.width as f64 - chip.0) as f32 + 12.0 } else { 16.0 + 3.0 * 14.0 + 2.0 * 10.0 + 12.0 };
    let room = (w as f32 - (16.0 + right_room) * s).max(0.0);
    let x0 = 16.0 * s;
    let name = if label.title.is_empty() { "Window".to_string() } else { label.title.clone() };
    let used = draw_text(&mut cv, &name, 14.0 * s, x0, room, None, ink);
    if label.game {
        let line = if label.line.is_empty() { String::new() } else { format!(" · {}", label.line) };
        if !line.is_empty() && room - used > 20.0 * s {
            draw_text(&mut cv, &line, 14.0 * s, x0 + used, room - used, None, c.muted);
        }
        // The chip: "Full screen: F11 or ≡", flame on a card.
        let (cx0, cx1) = ((chip.0 as f32) * s, (chip.1 as f32) * s);
        let ch = 22.0 * s;
        let cy = (h as f32 - ch) / 2.0;
        cv.round_rect(cx0, cy, cx1 - cx0, ch, 10.0 * s, c.card);
        // "Full screen: F11 or ≡": the words, then the Guide button's mark (three short lines: many fonts have no ≡).
        let words = text_width(CHIP_WORDS, 12.0) * s;
        let tx = cx0 + 10.0 * s;
        draw_text(&mut cv, CHIP_WORDS, 12.0 * s, tx, words + 2.0 * s, None, c.flame);
        let mx = tx + words;
        let mid = h as f32 / 2.0;
        for dy in [-3.0f32, 0.0, 3.0] {
            cv.line(mx + 1.0 * s, mid + dy * s, mx + (MENU_MARK_W - 1.0) * s, mid + dy * s, 1.3 * s, c.flame);
        }
    } else {
        let hover_on = |b: Button| k.hover == Some(b) || k.pressed == Some(b);
        let centres = crate::frames::dot_centres(k.width as f64);
        for (b, cx) in [Button::Minimize, Button::Maximize, Button::Close].into_iter().zip(centres) {
            let mut fill = if b == Button::Close { c.accent } else { c.card2 };
            if hover_on(b) {
                fill = [fill[0] + (1.0 - fill[0]) * 0.25, fill[1] + (1.0 - fill[1]) * 0.25, fill[2] + (1.0 - fill[2]) * 0.25, 1.0];
            }
            let d = crate::frames::DOT as f32 * s;
            cv.round_rect(cx as f32 * s - d / 2.0, h as f32 / 2.0 - d / 2.0, d, d, d / 2.0, fill);
        }
    }
    (cv.px, chip)
}

/// One line of text centred on `center_x`, its baseline at `baseline` (the lock prompt's lines). Returns its width.
pub(crate) fn draw_text_line(cv: &mut Canvas, text: &str, size: f32, center_x: f32, baseline: f32, c: theme::Rgba) -> f32 {
    let Some(f) = font() else { return 0.0 };
    let glyphs: Vec<(fontdue::Metrics, Vec<u8>)> = text.chars().map(|ch| f.rasterize(ch, size)).collect();
    let total: f32 = glyphs.iter().map(|(m, _)| m.advance_width).sum();
    let mut pen = center_x - total / 2.0;
    for (m, bmp) in &glyphs {
        let gx = (pen + m.xmin as f32).round() as i32;
        let gy = (baseline - m.height as f32 - m.ymin as f32).round() as i32;
        for row in 0..m.height {
            for col in 0..m.width {
                let a = bmp[row * m.width + col] as f32 / 255.0;
                cv.blend(gx + col as i32, gy + row as i32, c, a);
            }
        }
        pen += m.advance_width;
    }
    total
}

fn draw_bar(k: &DrawKey, t: &theme::Theme) -> Vec<u8> {
    let s = k.scale as f32;
    let (w, h) = (k.width * k.scale, HEADER_BAR_HEIGHT * k.scale);
    let bg = if k.active { t.title_active } else { t.title_inactive };
    let mut cv = Canvas::new(w, h, bg);
    // The focused window: a thin line of Mantel's accent along the top. Every bar: a line under it.
    if k.active {
        cv.rect(0, 0, w, (2.0 * s) as i32, t.accent);
    }
    cv.rect(0, h - k.scale, w, k.scale, t.line);

    let bw = BUTTON_WIDTH as f32 * s;
    let ink = if k.active { t.text } else { t.muted };
    for (i, b) in [Button::Close, Button::Maximize, Button::Minimize].into_iter().enumerate() {
        let x0 = w as f32 - bw * (i as f32 + 1.0);
        let hover = k.hover == Some(b);
        let mut glyph = ink;
        if hover || k.pressed == Some(b) {
            let (pad_x, pad_y) = (5.0 * s, 6.0 * s);
            let fill = if b == Button::Close { t.danger } else { t.hover };
            cv.round_rect(x0 + pad_x, pad_y, bw - 2.0 * pad_x, h as f32 - 2.0 * pad_y, 6.0 * s, fill);
            if b == Button::Close {
                glyph = [1.0, 1.0, 1.0, 1.0];
            }
        }
        let (cx, cy) = (x0 + bw / 2.0, h as f32 / 2.0);
        let r = 5.0 * s;
        let th = 1.4 * s;
        match b {
            Button::Close => {
                cv.line(cx - r, cy - r, cx + r, cy + r, th, glyph);
                cv.line(cx - r, cy + r, cx + r, cy - r, th, glyph);
            }
            Button::Maximize => {
                cv.line(cx - r, cy - r, cx + r, cy - r, th, glyph);
                cv.line(cx + r, cy - r, cx + r, cy + r, th, glyph);
                cv.line(cx + r, cy + r, cx - r, cy + r, th, glyph);
                cv.line(cx - r, cy + r, cx - r, cy - r, th, glyph);
            }
            Button::Minimize => cv.line(cx - r, cy, cx + r, cy, th, glyph),
            Button::Fullscreen => {}
        }
    }
    // The title, in the middle of the space left of the buttons (as labwc's Hearth theme centres it).
    let room = w as f32 - 3.0 * bw - 28.0 * s;
    if room > 20.0 * s && !k.title.is_empty() {
        draw_text(&mut cv, &k.title, 13.5 * s, 14.0 * s, room, Some(room), ink);
    }
    cv.px
}

impl WindowElement {
    pub fn decoration_state(&self) -> RefMut<'_, WindowState> {
        self.user_data().insert_if_missing(|| {
            RefCell::new(WindowState {
                is_ssd: false,
                active: false,
                header_bar: HeaderBar::new(),
            })
        });

        self.user_data()
            .get::<RefCell<WindowState>>()
            .unwrap()
            .borrow_mut()
    }

    pub fn set_ssd(&self, ssd: bool) {
        self.decoration_state().is_ssd = ssd;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resize_borders_and_corners() {
        use crate::shell::ResizeEdge as E;
        let g = Rectangle::new((0, 0).into(), (400, 300).into());
        assert_eq!(edge_at(g, (-3.0, 150.0).into()), Some(E::LEFT));
        assert_eq!(edge_at(g, (402.0, 150.0).into()), Some(E::RIGHT));
        assert_eq!(edge_at(g, (200.0, -2.0).into()), Some(E::TOP));
        assert_eq!(edge_at(g, (200.0, 304.0).into()), Some(E::BOTTOM));
        assert_eq!(edge_at(g, (-2.0, -2.0).into()), Some(E::TOP_LEFT));
        assert_eq!(edge_at(g, (403.0, 290.0).into()), Some(E::BOTTOM_RIGHT));
        assert_eq!(edge_at(g, (200.0, 150.0).into()), None);
        assert_eq!(edge_at(g, (-20.0, 150.0).into()), None);
    }

    #[test]
    fn a_bar_is_drawn_opaque_in_its_size() {
        let k = DrawKey {
            width: 300,
            scale: 2,
            active: true,
            hover: Some(Button::Close),
            pressed: None,
            title: "A rather long window title that has to be cut".into(),
            serial: 0,
            firehold: None,
        };
        let px = draw_bar(&k, &theme::Theme::default());
        assert_eq!(px.len(), (300 * 2 * HEADER_BAR_HEIGHT * 2 * 4) as usize);
        assert!(px.chunks_exact(4).all(|p| p[3] == 255));
        // Not one colour: the accent line, the buttons.
        let first = &px[0..4];
        assert!(px.chunks_exact(4).any(|p| p != first));
    }

    #[test]
    fn firehold_bars_have_round_corners_dots_and_a_game_chip() {
        let label = crate::frames::Label { title: "Hearth Files — Pictures".into(), ..Default::default() };
        let k = DrawKey { width: 400, scale: 1, active: true, hover: None, pressed: None, title: String::new(), serial: 0, firehold: Some((1, label.clone())) };
        let c = crate::frames::Colours::default();
        let (px, _) = draw_firehold_bar(&k, &label, &c);
        let at = |x: i32, y: i32| { let i = ((y * 400 + x) * 4) as usize; [px[i], px[i + 1], px[i + 2], px[i + 3]] };
        assert_eq!(at(0, 0)[3], 0, "the top-left corner is see-through");
        assert_eq!(at(200, HEADER_BAR_HEIGHT / 2)[3], 255);
        // The close dot is the accent colour (B, G, R, A).
        let cx = crate::frames::dot_centres(400.0)[2] as i32;
        let a = c.accent;
        assert_eq!(at(cx, HEADER_BAR_HEIGHT / 2), [to_u8(a[2]), to_u8(a[1]), to_u8(a[0]), 255]);
        // A game: the chip on the right, its clicks there.
        let game = crate::frames::Label { title: "Ember Trails".into(), game: true, line: "running · VRR".into() };
        let (_, chip) = draw_firehold_bar(&k, &game, &c);
        assert!(chip.1 <= 400.0 && chip.0 < chip.1);
        let mut b = HeaderBar::new();
        b.width = 400;
        b.fh = Some(FhHits { game: true, chip });
        assert_eq!(b.button_at((chip.0 + chip.1) / 2.0), Some(Button::Fullscreen));
        assert_eq!(b.button_at(20.0), None);
        b.fh = Some(FhHits { game: false, chip });
        assert_eq!(b.button_at(cx as f64), Some(Button::Close));
        assert_eq!(b.button_at(crate::frames::dot_centres(400.0)[0]), Some(Button::Minimize));
        assert_eq!(b.button_at(100.0), None);
    }

    #[test]
    fn buttons_from_the_right() {
        let mut b = HeaderBar::new();
        b.width = 400;
        assert_eq!(b.button_at(399.0), Some(Button::Close));
        assert_eq!(b.button_at(330.0), Some(Button::Maximize));
        assert_eq!(b.button_at(290.0), Some(Button::Minimize));
        assert_eq!(b.button_at(100.0), None);
    }
}
