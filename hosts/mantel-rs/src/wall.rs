//! The wallpaper, behind everything (the background layer), drawn as the QML Mantel's Wallpaper.qml draws it:
//! a picture you chose (builtin:name, a file), or the look's own scene: Hearth's fireside (the default: warm darkness
//! lit from below by the fire's glow), Slate Tide (the Steam-style look's: soft blue light over blue-slate) and the
//! other moving ones as their still pictures. This test draws every scene at rest (the QML Mantel holds them still
//! too with Reduce motion, behind windows, on battery and without a GPU): drawn once, nothing runs while you work.

use iced::widget::image::Handle;
use iced::{Color, Size};
use std::path::PathBuf;

use crate::look::{self, Look};

/// What desktop.json's wallpaper (or the look's own) means: a picture file, or one of the drawn scenes.
#[derive(Debug, Clone, PartialEq)]
pub enum Scene {
    Picture(PathBuf),
    Fireside,
    Tide,
}

pub fn scene_for(w: &str, share: &std::path::Path) -> Scene {
    let img = share.join("desktop/shell/images");
    if w.is_empty() {
        return Scene::Fireside;
    }
    if let Some(an) = w.strip_prefix("anim:") {
        return match an {
            "tide" => Scene::Tide,
            "lava" | "aurora" | "ridges" | "motes" => {
                let f = img.join(format!("wall-{an}.jpg"));
                if f.exists() { Scene::Picture(f) } else { Scene::Fireside }
            }
            _ => Scene::Fireside,
        };
    }
    let f = if let Some(b) = w.strip_prefix("builtin:") { img.join("wallpapers").join(format!("{b}.jpg")) } else { PathBuf::from(w.strip_prefix("file://").unwrap_or(w)) };
    if f.exists() { Scene::Picture(f) } else { Scene::Fireside }
}

pub fn scene(l: &Look) -> Scene {
    scene_for(&l.wall, &look::share())
}

fn hex(s: &str) -> Color {
    hearth_ui::theme::hex(s).unwrap_or(Color::BLACK)
}

/// #AARRGGBB (QML's colour with alpha first)
fn argb(s: &str) -> Color {
    let v = u32::from_str_radix(s.trim_start_matches('#'), 16).unwrap_or(0);
    Color::from_rgba8(((v >> 16) & 255) as u8, ((v >> 8) & 255) as u8, (v & 255) as u8, ((v >> 24) & 255) as f32 / 255.0)
}

// ---------------------------------------------------------------- blended once, as Qt blends it

/// A JPEG's or PNG's size from its header (no decoding).
pub fn image_size(f: &std::path::Path) -> Option<(u32, u32)> {
    image::image_dimensions(f).ok()
}

fn over(dst: &mut [f32; 3], src: [f32; 4], k: f32) {
    let a = src[3] * k;
    for i in 0..3 {
        dst[i] = src[i] * a + dst[i] * (1.0 - a);
    }
}

fn lerp(a: Color, b: Color, t: f32) -> [f32; 4] {
    [a.r + (b.r - a.r) * t, a.g + (b.g - a.g) * t, a.b + (b.b - a.b) * t, a.a + (b.a - a.a) * t]
}

/// A vertical gradient's colour at y (0..1) through its stops.
fn at(stops: &[(f32, Color)], y: f32) -> [f32; 4] {
    if y <= stops[0].0 {
        let c = stops[0].1;
        return [c.r, c.g, c.b, c.a];
    }
    for w in stops.windows(2) {
        if y <= w[1].0 {
            let t = if w[1].0 > w[0].0 { (y - w[0].0) / (w[1].0 - w[0].0) } else { 1.0 };
            return lerp(w[0].1, w[1].1, t);
        }
    }
    let c = stops[stops.len() - 1].1;
    [c.r, c.g, c.b, c.a]
}

struct Canvas {
    w: u32,
    h: u32,
    px: Vec<[f32; 3]>,
}

impl Canvas {
    fn new(w: u32, h: u32, c: Color) -> Canvas {
        Canvas { w, h, px: vec![[c.r, c.g, c.b]; (w * h) as usize] }
    }
    fn gradient(&mut self, stops: &[(f32, Color)]) {
        for y in 0..self.h {
            let c = at(stops, y as f32 / (self.h.max(2) - 1) as f32);
            for x in 0..self.w {
                over(&mut self.px[(y * self.w + x) as usize], c, 1.0);
            }
        }
    }
    /// A picture scaled into (x, y, w, h) at this opacity (smooth scaling, straight alpha, as Qt's Image).
    fn picture(&mut self, file: &std::path::Path, x: f32, y: f32, w: f32, h: f32, opacity: f32) {
        let Ok(img) = image::open(file) else { return };
        let (tw, th) = (w.round().max(1.0) as u32, h.round().max(1.0) as u32);
        let img = image::imageops::resize(&img.to_rgba8(), tw, th, image::imageops::FilterType::Triangle);
        let (x0, y0) = (x.round() as i64, y.round() as i64);
        for (ix, iy, p) in img.enumerate_pixels() {
            let (sx, sy) = (x0 + ix as i64, y0 + iy as i64);
            if sx < 0 || sy < 0 || sx >= self.w as i64 || sy >= self.h as i64 || p[3] == 0 {
                continue;
            }
            let s = [p[0] as f32 / 255.0, p[1] as f32 / 255.0, p[2] as f32 / 255.0, p[3] as f32 / 255.0];
            over(&mut self.px[(sy as u64 * self.w as u64 + sx as u64) as usize], s, opacity);
        }
    }
    fn rgba(&self) -> Vec<u8> {
        let mut v = Vec::with_capacity(self.px.len() * 4);
        for p in &self.px {
            v.extend([(p[0].clamp(0.0, 1.0) * 255.0).round() as u8, (p[1].clamp(0.0, 1.0) * 255.0).round() as u8, (p[2].clamp(0.0, 1.0) * 255.0).round() as u8, 255]);
        }
        v
    }
}

/// The wallpaper for this look and screen size, blended into one picture (drawn once; at rest, like the QML Mantel's
/// moving ones with Reduce motion or without a GPU).
pub fn compose(l: &Look, size: Size) -> Handle {
    let (w, h) = (size.width.max(1.0).round() as u32, size.height.max(1.0).round() as u32);
    let (wf, hf) = (w as f32, h as f32);
    let img = |f: &str| look::shell_file(&format!("images/{f}"));
    let mut c = Canvas::new(w, h, l.p.bg);
    match scene(l) {
        Scene::Picture(f) => {
            // PreserveAspectCrop: filling the screen, cut at the sides or top and bottom
            let (iw, ih) = image_size(&f).unwrap_or((w, h));
            let k = (wf / iw as f32).max(hf / ih as f32);
            let (pw, ph) = (iw as f32 * k, ih as f32 * k);
            c.picture(&f, (wf - pw) / 2.0, (hf - ph) / 2.0, pw, ph, 1.0);
        }
        Scene::Fireside => {
            c.gradient(&[(0.0, hex("#0b0807")), (0.6, l.p.bg2), (1.0, hex("#1f140e"))]);
            let (gw, gh) = (wf * 1.5, hf * 1.1);
            c.picture(&img("ember-glow.png"), (wf - gw) / 2.0, hf - gh * 0.45, gw, gh, 0.32);
            let (cw, ch) = (wf * 0.7, hf * 0.6);
            c.picture(&img("ember-glow.png"), (wf - cw) / 2.0, hf - ch * 0.42, cw, ch, 0.22);
        }
        Scene::Tide => {
            c.gradient(&[(0.0, hex("#0e131b")), (1.0, hex("#1b2838"))]);
            for (src, x, y, sz, o) in [
                ("blob-blue.png", 0.22, 0.64, 0.62, 0.55),
                ("blob-slate.png", 0.52, 0.32, 0.70, 0.7),
                ("blob-sky.png", 0.76, 0.62, 0.46, 0.35),
                ("blob-blue.png", 0.84, 0.20, 0.40, 0.45),
            ] {
                let d = hf * sz * 1.7;
                c.picture(&img(src), wf * x - d / 2.0, hf * y - d / 2.0, d, d, o);
            }
            c.gradient(&[(0.0, Color::TRANSPARENT), (0.55, Color::TRANSPARENT), (1.0, argb("#80090c12"))]);
        }
    }
    Handle::from_rgba(w, h, c.rgba())
}

/// A picture with its opacity blended in (the menu's glow), made once per file and opacity.
pub fn faded(file: &std::path::Path, opacity: f32) -> Handle {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};
    static CACHE: OnceLock<Mutex<HashMap<(PathBuf, u32), Handle>>> = OnceLock::new();
    let key = (file.to_path_buf(), (opacity * 1000.0) as u32);
    let m = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(h) = m.lock().ok().and_then(|g| g.get(&key).cloned()) {
        return h;
    }
    let h = match image::open(file) {
        Ok(i) => {
            let mut i = i.to_rgba8();
            for p in i.pixels_mut() {
                p[3] = (p[3] as f32 * opacity).round() as u8;
            }
            Handle::from_rgba(i.width(), i.height(), i.into_raw())
        }
        Err(_) => Handle::from_path(file),
    };
    if let Ok(mut g) = m.lock() {
        g.insert(key, h.clone());
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wallpaper_names_become_scenes() {
        let d = tempfile::tempdir().unwrap();
        let img = d.path().join("desktop/shell/images");
        std::fs::create_dir_all(img.join("wallpapers")).unwrap();
        std::fs::write(img.join("wallpapers/slate-dusk.jpg"), b"x").unwrap();
        std::fs::write(img.join("wall-lava.jpg"), b"x").unwrap();
        assert_eq!(scene_for("builtin:slate-dusk", d.path()), Scene::Picture(img.join("wallpapers/slate-dusk.jpg")));
        assert_eq!(scene_for("anim:tide", d.path()), Scene::Tide);
        assert_eq!(scene_for("anim:lava", d.path()), Scene::Picture(img.join("wall-lava.jpg")));
        assert_eq!(scene_for("", d.path()), Scene::Fireside);
        assert_eq!(scene_for("/nope.png", d.path()), Scene::Fireside);
    }

    #[test]
    fn argb_has_alpha_first() {
        let c = argb("#80090c12");
        assert!((c.a - 0.502).abs() < 0.01);
    }
}
