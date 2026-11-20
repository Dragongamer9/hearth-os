//! capture-draw.js in Rust: the markup's shapes and the area maths, pixelate and blur. Shapes are kept in the
//! picture's own pixels, so the same list draws the editor's view (scaled to fit, editor.rs) and the saved picture
//! (1:1, export.rs).
//!   shape: t = arrow | box | pen | hl | text | blur | pix | step | crop, c = colour, w = width,
//!          x1, y1, x2, y2 (arrow, box, blur, pix, crop), pts [x, y, x, y...] (pen, hl), x1, y1, text, size (text),
//!          x1, y1, n (step)

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct R {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl R {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> R {
        R { x, y, w, h }
    }
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && x <= self.x + self.w && y >= self.y && y <= self.y + self.h
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum T {
    Arrow,
    Box,
    Pen,
    Hl,
    Text,
    Blur,
    Pix,
    Step,
    Crop,
}

/// The tools, in the bar's order: (tool, glyph, name).
pub const TOOLS: [(T, &str, &str); 9] = [
    (T::Arrow, "arrow", "Arrow"),
    (T::Box, "box", "Box"),
    (T::Pen, "pen", "Pen"),
    (T::Hl, "highlighter", "Highlighter"),
    (T::Text, "text", "Text"),
    (T::Blur, "blur", "Blur"),
    (T::Pix, "pixelate", "Pixelate"),
    (T::Step, "steps", "Numbered steps"),
    (T::Crop, "crop", "Crop"),
];

/// The seven colours (as capture-draw.js's COLORS).
pub const COLORS: [u32; 7] = [0xff4d4d, 0xff8a3d, 0xffd166, 0x4cd97b, 0x4aa8ff, 0xffffff, 0x111111];

/// S, M, L.
pub fn width_of(k: char) -> f32 {
    match k {
        's' => 3.0,
        'l' => 10.0,
        _ => 6.0,
    }
}
/// The text size for S, M, L.
pub fn text_size_of(k: char) -> f32 {
    match k {
        's' => 22.0,
        'l' => 52.0,
        _ => 34.0,
    }
}

/// Pixelate's and Blur's small copies: 1/14 and 1/24 of the picture.
pub const PIX: f32 = 1.0 / 14.0;
pub const BLUR: f32 = 1.0 / 24.0;

#[derive(Debug, Clone, PartialEq)]
pub struct Shape {
    pub t: T,
    pub c: u32,
    pub w: f32,
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
    pub pts: Vec<f32>,
    pub text: String,
    pub size: f32,
    pub n: u32,
}

impl Shape {
    pub fn new(t: T, c: u32, w: f32, x1: f32, y1: f32) -> Shape {
        Shape { t, c, w, x1, y1, x2: x1, y2: y1, pts: Vec::new(), text: String::new(), size: 34.0, n: 0 }
    }
    pub fn rect(t: T, c: u32, w: f32, x1: f32, y1: f32, x2: f32, y2: f32) -> Shape {
        Shape { x2, y2, ..Shape::new(t, c, w, x1, y1) }
    }
    pub fn line(t: T, c: u32, w: f32, pts: Vec<f32>) -> Shape {
        let (x1, y1) = (pts.first().copied().unwrap_or(0.0), pts.get(1).copied().unwrap_or(0.0));
        Shape { pts, ..Shape::new(t, c, w, x1, y1) }
    }
    pub fn text(c: u32, w: f32, x1: f32, y1: f32, text: &str, size: f32) -> Shape {
        Shape { text: text.to_string(), size, ..Shape::new(T::Text, c, w, x1, y1) }
    }
    pub fn step(c: u32, w: f32, x1: f32, y1: f32, n: u32) -> Shape {
        Shape { n, ..Shape::new(T::Step, c, w, x1, y1) }
    }
    pub fn hides(&self) -> bool {
        matches!(self.t, T::Blur | T::Pix)
    }
}

pub fn norm_rect(x1: f32, y1: f32, x2: f32, y2: f32) -> R {
    R { x: x1.min(x2), y: y1.min(y2), w: (x2 - x1).abs(), h: (y2 - y1).abs() }
}

/// A rectangle inside 0..W, 0..H, whole pixels, at least 1×1.
pub fn clamp_rect(r: R, w: f32, h: f32) -> R {
    let x = r.x.round().min(w - 1.0).max(0.0);
    let y = r.y.round().min(h - 1.0).max(0.0);
    let cw = r.w.round().min(w - x).max(1.0);
    let ch = r.h.round().min(h - y).max(1.0);
    R { x, y, w: cw, h: ch }
}

/// The scale that fits W×H into an area (never more than 2× for a small crop).
pub fn fit(area_w: f32, area_h: f32, w: f32, h: f32) -> f32 {
    if w <= 0.0 || h <= 0.0 || area_w <= 0.0 || area_h <= 0.0 {
        return 1.0;
    }
    (area_w / w).min(area_h / h).min(2.0)
}

/// The area picker works in the screen's own (logical) size; the picture of that screen can be bigger (a scaled
/// screen: 2560 pixels for 1280 logical). The crop in the picture's pixels.
pub fn picker_to_crop(sel: R, screen_w: f32, screen_h: f32, img_w: f32, img_h: f32) -> R {
    let kx = if screen_w > 0.0 { img_w / screen_w } else { 1.0 };
    let ky = if screen_h > 0.0 { img_h / screen_h } else { 1.0 };
    clamp_rect(R { x: sel.x * kx, y: sel.y * ky, w: sel.w * kx, h: sel.h * ky }, img_w, img_h)
}

/// The scrolling capture's pill goes below the area when the area starts near the top of its screen (and leaves
/// room under it), else at the top (Mantel's CaptureIndicator placement).
pub fn pill_below(area: R, screen_h: f32) -> bool {
    area.y < 110.0 && area.y + area.h < screen_h - 110.0
}

/// The picker's box moved (D-pad / arrows) or sized, kept on the screen.
pub fn move_sel(s: R, dx: f32, dy: f32, w: f32, h: f32) -> R {
    R { x: (s.x + dx).min(w - s.w).max(0.0), y: (s.y + dy).min(h - s.h).max(0.0), w: s.w, h: s.h }
}
pub fn resize_sel(s: R, dw: f32, dh: f32, w: f32, h: f32) -> R {
    let m = 24.0;
    R { x: s.x, y: s.y, w: (s.w + dw).min(w - s.x).max(m), h: (s.h + dh).min(h - s.y).max(m) }
}
/// The box a controller starts with: the middle half of the screen.
pub fn start_sel(w: f32, h: f32) -> R {
    R { x: (w / 4.0).round(), y: (h / 4.0).round(), w: (w / 2.0).round(), h: (h / 2.0).round() }
}

pub fn next_step(shapes: &[Shape]) -> u32 {
    shapes.iter().filter(|s| s.t == T::Step).map(|s| s.n).max().unwrap_or(0) + 1
}

/// The two back corners of an arrow's head at (x2, y2).
pub fn arrow_head(x1: f32, y1: f32, x2: f32, y2: f32, len: f32) -> [f32; 4] {
    let a = (y2 - y1).atan2(x2 - x1);
    let s = std::f32::consts::PI / 7.0;
    [x2 - len * (a - s).cos(), y2 - len * (a - s).sin(), x2 - len * (a + s).cos(), y2 - len * (a + s).sin()]
}

/// Pixelate DATA (RGBA, W×H) in place: each BLOCK×BLOCK square becomes its average colour.
pub fn pixelate(data: &mut [u8], w: usize, h: usize, block: usize) {
    let b = block.max(2);
    let mut by = 0;
    while by < h {
        let mut bx = 0;
        while bx < w {
            let (ex, ey) = ((bx + b).min(w), (by + b).min(h));
            let mut sum = [0u64; 4];
            let mut n = 0u64;
            for y in by..ey {
                for x in bx..ex {
                    let i = (y * w + x) * 4;
                    for c in 0..4 {
                        sum[c] += data[i + c] as u64;
                    }
                    n += 1;
                }
            }
            let avg: Vec<u8> = sum.iter().map(|s| ((*s as f64 / n.max(1) as f64).round()) as u8).collect();
            for y in by..ey {
                for x in bx..ex {
                    let i = (y * w + x) * 4;
                    data[i..i + 4].copy_from_slice(&avg);
                }
            }
            bx += b;
        }
        by += b;
    }
}

/// Blur: pixelate to RADIUS, then one box blur across and down (nothing readable is left: the small squares are
/// averaged before they're smoothed).
pub fn blur(data: &mut [u8], w: usize, h: usize, radius: usize) {
    let rad = radius.max(1);
    pixelate(data, w, h, rad);
    if w == 0 || h == 0 {
        return;
    }
    let mut tmp = vec![0f32; w * h * 4];
    for y in 0..h {
        for c in 0..4 {
            let (mut sum, mut n) = (0f32, 0f32);
            for k in 0..=rad.min(w - 1) {
                sum += data[(y * w + k) * 4 + c] as f32;
                n += 1.0;
            }
            for x in 0..w {
                tmp[(y * w + x) * 4 + c] = sum / n;
                if x + rad + 1 < w {
                    sum += data[(y * w + x + rad + 1) * 4 + c] as f32;
                    n += 1.0;
                }
                if x >= rad {
                    sum -= data[(y * w + x - rad) * 4 + c] as f32;
                    n -= 1.0;
                }
            }
        }
    }
    for x in 0..w {
        for c in 0..4 {
            let (mut sum, mut n) = (0f32, 0f32);
            for k in 0..=rad.min(h - 1) {
                sum += tmp[(k * w + x) * 4 + c];
                n += 1.0;
            }
            for y in 0..h {
                let i = (y * w + x) * 4 + c;
                data[i] = (sum / n).round().clamp(0.0, 255.0) as u8;
                if y + rad + 1 < h {
                    sum += tmp[((y + rad + 1) * w + x) * 4 + c];
                    n += 1.0;
                }
                if y >= rad {
                    sum -= tmp[((y - rad) * w + x) * 4 + c];
                    n -= 1.0;
                }
            }
        }
    }
}

/// A shape's box in picture pixels.
pub fn bounds(s: &Shape) -> R {
    if !s.pts.is_empty() {
        let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for p in s.pts.chunks(2) {
            if p.len() < 2 {
                break;
            }
            x0 = x0.min(p[0]);
            x1 = x1.max(p[0]);
            y0 = y0.min(p[1]);
            y1 = y1.max(p[1]);
        }
        return R { x: x0, y: y0, w: x1 - x0, h: y1 - y0 };
    }
    match s.t {
        T::Text | T::Step => R { x: s.x1, y: s.y1, w: 0.0, h: 0.0 },
        _ => norm_rect(s.x1, s.y1, s.x2, s.y2),
    }
}

/// A shape worth keeping (a click without a drag makes no arrow or box).
pub fn worth_keeping(s: &Shape) -> bool {
    match s.t {
        T::Text => !s.text.trim().is_empty(),
        T::Step => true,
        T::Pen | T::Hl => s.pts.len() >= 2,
        T::Arrow => (s.x2 - s.x1).abs() + (s.y2 - s.y1).abs() >= 6.0,
        _ => {
            let b = bounds(s);
            b.w >= 4.0 && b.h >= 4.0
        }
    }
}

fn p2(n: i32) -> String {
    format!("{n:02}")
}

/// "Screenshot 2026-11-16 21-04-05.png", the name the helper and open.sh give a new picture.
pub fn default_name(ts: i64) -> String {
    let t = hearth_ui::time::local(ts);
    format!("Screenshot {}-{}-{} {}-{}-{}.png", t.year, p2(t.mon as i32 + 1), p2(t.day), p2(t.hour), p2(t.min), p2(t.sec))
}

/// "Just now", "5 min ago", "Today 21:04", "Nov 12" for the recent list.
pub fn when(t: i64, now: i64) -> String {
    let s = (now - t).max(0);
    if s < 60 {
        return "Just now".into();
    }
    if s < 3600 {
        return format!("{} min ago", s / 60);
    }
    let d = hearth_ui::time::local(t);
    let n = hearth_ui::time::local(now);
    if d.year == n.year && d.mon == n.mon && d.day == n.day {
        return format!("Today {}:{}", p2(d.hour), p2(d.min));
    }
    format!("{} {}", hearth_ui::time::month(d.mon), d.day)
}

/// "#rrggbb" of a colour.
pub fn hex(c: u32) -> String {
    format!("#{c:06x}")
}
pub fn rgb(c: u32) -> (u8, u8, u8) {
    (((c >> 16) & 255) as u8, ((c >> 8) & 255) as u8, (c & 255) as u8)
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_scroll_pill_goes_below_an_area_near_the_top() {
        assert!(super::pill_below(super::R::new(100.0, 40.0, 600.0, 400.0), 1080.0));
        assert!(!super::pill_below(super::R::new(100.0, 300.0, 600.0, 400.0), 1080.0));
        assert!(!super::pill_below(super::R::new(0.0, 0.0, 1920.0, 1080.0), 1080.0));
    }

    use super::*;

    #[test]
    fn rects() {
        assert_eq!(norm_rect(10.0, 20.0, 4.0, 2.0), R::new(4.0, 2.0, 6.0, 18.0));
        assert_eq!(clamp_rect(R::new(-5.0, 3.4, 500.0, 10.0), 100.0, 50.0), R::new(0.0, 3.0, 100.0, 10.0));
        assert_eq!(fit(800.0, 600.0, 1600.0, 900.0), 0.5);
        assert_eq!(fit(800.0, 600.0, 10.0, 10.0), 2.0);
        // A scaled screen: 1280 logical, 2560 pixels.
        assert_eq!(picker_to_crop(R::new(10.0, 10.0, 100.0, 50.0), 1280.0, 800.0, 2560.0, 1600.0), R::new(20.0, 20.0, 200.0, 100.0));
        assert_eq!(move_sel(R::new(10.0, 10.0, 100.0, 100.0), -50.0, 1000.0, 400.0, 300.0), R::new(0.0, 200.0, 100.0, 100.0));
        assert_eq!(resize_sel(R::new(10.0, 10.0, 30.0, 30.0), -100.0, 1000.0, 400.0, 300.0), R::new(10.0, 10.0, 24.0, 290.0));
        assert_eq!(start_sel(1280.0, 800.0), R::new(320.0, 200.0, 640.0, 400.0));
    }

    #[test]
    fn shapes() {
        let a = Shape::rect(T::Arrow, COLORS[0], 6.0, 0.0, 0.0, 2.0, 2.0);
        assert!(!worth_keeping(&a));
        assert!(worth_keeping(&Shape::rect(T::Arrow, COLORS[0], 6.0, 0.0, 0.0, 6.0, 0.0)));
        assert!(!worth_keeping(&Shape::text(COLORS[0], 6.0, 0.0, 0.0, "  ", 34.0)));
        let s = vec![Shape::step(COLORS[4], 6.0, 1.0, 1.0, 1), Shape::step(COLORS[4], 6.0, 1.0, 1.0, 4)];
        assert_eq!(next_step(&s), 5);
        assert_eq!(next_step(&[]), 1);
        let h = arrow_head(0.0, 0.0, 10.0, 0.0, 5.0);
        assert!(h[0] < 10.0 && h[2] < 10.0 && (h[1] + h[3]).abs() < 1e-4);
    }

    #[test]
    fn hiding_leaves_nothing_readable() {
        // A checkerboard: after pixelate every 4×4 block is one grey; after blur no pixel is pure black or white.
        let (w, h) = (16usize, 16usize);
        let mut d = vec![0u8; w * h * 4];
        for y in 0..h {
            for x in 0..w {
                let v = if (x + y) % 2 == 0 { 255 } else { 0 };
                let i = (y * w + x) * 4;
                d[i..i + 4].copy_from_slice(&[v, v, v, 255]);
            }
        }
        let mut p = d.clone();
        pixelate(&mut p, w, h, 4);
        assert!(p.chunks(4).all(|c| c[0] == 128 || c[0] == 127));
        let mut b = d.clone();
        blur(&mut b, w, h, 4);
        assert!(b.chunks(4).all(|c| c[0] > 20 && c[0] < 235));
    }

    #[test]
    fn words() {
        assert_eq!(when(1000, 1030), "Just now");
        assert_eq!(when(1000, 1000 + 300), "5 min ago");
        assert!(default_name(0).starts_with("Screenshot 19"));
        assert!(default_name(0).ends_with(".png"));
        assert_eq!(hex(0xff4d4d), "#ff4d4d");
    }
}
