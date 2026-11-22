//! The wallpaper, behind everything (the background layer), drawn as the QML Mantel's Wallpaper.qml draws it:
//! a picture you chose (builtin:name, a file), or the look's own scene: Hearth's fireside (the default: warm darkness
//! lit from below by the fire's glow), Slate Tide (the Steam-style look's: soft blue light over blue-slate) and the
//! other moving ones as their still pictures. This test draws every scene at rest (the QML Mantel holds them still
//! too with Reduce motion, behind windows, on battery and without a GPU): drawn once, nothing runs while you work.

use iced::widget::canvas::{self, gradient, Cache, Frame, Geometry};
use iced::widget::image::Handle;
use iced::{mouse, Color, Point, Rectangle, Renderer, Size, Theme};
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

pub struct Wall<'a> {
    pub look: Look,
    pub cache: &'a Cache,
}

fn vertical(frame: &mut Frame, s: Size, stops: &[(f32, Color)]) {
    let mut g = gradient::Linear::new(Point::new(0.0, 0.0), Point::new(0.0, s.height));
    for (o, c) in stops {
        g = g.add_stop(*o, *c);
    }
    frame.fill_rectangle(Point::ORIGIN, s, g);
}

fn pic(frame: &mut Frame, file: &str, x: f32, y: f32, w: f32, h: f32, opacity: f32) {
    let h_ = look::image_handle(&look::shell_file(&format!("images/{file}")));
    frame.draw_image(Rectangle { x, y, width: w, height: h }, canvas::Image::new(h_).opacity(opacity));
}

/// PreserveAspectCrop: the picture filling the screen, cut at the sides or top and bottom.
fn crop(frame: &mut Frame, s: Size, f: &std::path::Path) {
    let handle: Handle = look::image_handle(f);
    let (iw, ih) = image_size(f).unwrap_or((s.width as u32, s.height as u32));
    let k = (s.width / iw as f32).max(s.height / ih as f32);
    let (w, h) = (iw as f32 * k, ih as f32 * k);
    frame.draw_image(Rectangle { x: (s.width - w) / 2.0, y: (s.height - h) / 2.0, width: w, height: h }, canvas::Image::new(handle));
}

/// A JPEG's or PNG's size from its header (no decoding).
pub fn image_size(f: &std::path::Path) -> Option<(u32, u32)> {
    let b = std::fs::read(f).ok()?;
    if b.starts_with(b"\x89PNG") && b.len() > 24 {
        return Some((u32::from_be_bytes(b[16..20].try_into().ok()?), u32::from_be_bytes(b[20..24].try_into().ok()?)));
    }
    let mut i = 2;
    while i + 9 < b.len() {
        if b[i] != 0xFF {
            i += 1;
            continue;
        }
        let m = b[i + 1];
        let len = u16::from_be_bytes([b[i + 2], b[i + 3]]) as usize;
        if (0xC0..=0xCF).contains(&m) && m != 0xC4 && m != 0xC8 && m != 0xCC {
            let h = u16::from_be_bytes([b[i + 5], b[i + 6]]) as u32;
            let w = u16::from_be_bytes([b[i + 7], b[i + 8]]) as u32;
            return Some((w, h));
        }
        i += 2 + len;
    }
    None
}

impl<'a, M> canvas::Program<M> for Wall<'a> {
    type State = ();
    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let l = &self.look;
        let g = self.cache.draw(renderer, bounds.size(), |frame| {
            let s = frame.size();
            let (w, h) = (s.width, s.height);
            frame.fill_rectangle(Point::ORIGIN, s, l.p.bg);
            match scene(l) {
                Scene::Picture(f) => crop(frame, s, &f),
                Scene::Fireside => {
                    vertical(frame, s, &[(0.0, hex("#0b0807")), (0.6, l.p.bg2), (1.0, hex("#1f140e"))]);
                    // the fire's light, and its smaller, hotter core (their resting brightness)
                    let (gw, gh) = (w * 1.5, h * 1.1);
                    pic(frame, "ember-glow.png", (w - gw) / 2.0, h - gh * 0.45, gw, gh, 0.32);
                    let (cw, ch) = (w * 0.7, h * 0.6);
                    pic(frame, "ember-glow.png", (w - cw) / 2.0, h - ch * 0.42, cw, ch, 0.22);
                }
                Scene::Tide => {
                    vertical(frame, s, &[(0.0, hex("#0e131b")), (1.0, hex("#1b2838"))]);
                    for (src, x, y, sz, o) in [
                        ("blob-blue.png", 0.22, 0.64, 0.62, 0.55),
                        ("blob-slate.png", 0.52, 0.32, 0.70, 0.7),
                        ("blob-sky.png", 0.76, 0.62, 0.46, 0.35),
                        ("blob-blue.png", 0.84, 0.20, 0.40, 0.45),
                    ] {
                        let d = h * sz * 1.7;
                        pic(frame, src, w * x - d / 2.0, h * y - d / 2.0, d, d, o);
                    }
                    vertical(frame, s, &[(0.0, Color::TRANSPARENT), (0.55, Color::TRANSPARENT), (1.0, argb("#80090c12"))]);
                }
            }
        });
        vec![g]
    }
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
