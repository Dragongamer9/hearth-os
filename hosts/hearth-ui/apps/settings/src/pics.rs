//! Pictures drawn as the pictures themselves (Personalization's wallpapers, your account picture, the login screen's
//! background): decoded once on a thread of their own, cut to the shape they're shown in (the middle of the picture,
//! as a wallpaper is), shrunk, and kept in memory for as long as Settings is open. Nothing is written anywhere and
//! nothing runs at rest: a page asks for the pictures it shows, once, and is drawn again when they're ready. A picture
//! that can't be read stays drawn as colour, as before.
//!
//! The cutting is done here, in the pixels, not by the window (iced's software renderer draws a picture that's cut to
//! fit past its own edges), and a round picture is made round here too, so it's round on whatever it sits on.

use iced::widget::image;
use iced::widget::image::Handle;
use iced::{ContentFit, Element, Length};
use std::collections::HashMap;
use std::future::Future;
use std::sync::{Mutex, OnceLock};

/// Pictures this big aren't decoded (it would take too long and too much memory), as in Hearth Files.
const MAX_BYTES: u64 = 60 * 1024 * 1024;

/// The shape a picture is shown in: its width and height on screen (it's made twice that, for sharp edges), whether
/// it's a circle, and whether the whole picture fits inside it (an app's screenshot) rather than its middle filling it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cut {
    pub w: u32,
    pub h: u32,
    pub round: bool,
    pub fit: bool,
}

/// Your account picture (drawn at any size up to this, in its circle).
pub const ROUND: Cut = Cut { w: 64, h: 64, round: true, fit: false };
/// A wallpaper's tile in Personalization.
pub const WALL: Cut = Cut { w: 190, h: 96, round: false, fit: false };
/// A small picture beside its name in a list.
pub const ROW: Cut = Cut { w: 64, h: 40, round: false, fit: false };
/// The login screen's preview: a screen, 16:9.
pub const SCREEN: Cut = Cut { w: 462, h: 260, round: false, fit: false };
/// An app's icon in the App store (48 in a list, 96 on its page: drawn from the same picture).
pub const ICON: Cut = Cut { w: 96, h: 96, round: false, fit: true };
/// An app's screenshot in the App store, the whole picture (SetStore.qml: PreserveAspectFit).
pub const SHOT: Cut = Cut { w: 320, h: 180, round: false, fit: true };

fn key(path: &str, c: Cut) -> String {
    format!("{path}|{}x{}{}{}", c.w, c.h, if c.round { "o" } else { "" }, if c.fit { "f" } else { "" })
}

fn cache() -> &'static Mutex<HashMap<String, Option<Handle>>> {
    static C: OnceLock<Mutex<HashMap<String, Option<Handle>>>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(HashMap::new()))
}

/// The picture file for a wallpaper: the file itself, or a KDE wallpaper package's picture (its contents/images,
/// the biggest one, else its screenshot).
pub fn file_of(path: &str) -> String {
    let p = std::path::Path::new(path);
    if !p.is_dir() {
        return path.to_string();
    }
    let mut best: Option<(u64, String)> = None;
    if let Ok(rd) = std::fs::read_dir(p.join("contents/images")) {
        for e in rd.flatten() {
            let f = e.path();
            let ext = f.extension().and_then(|x| x.to_str()).unwrap_or("").to_lowercase();
            if !["jpg", "jpeg", "png", "webp"].contains(&ext.as_str()) {
                continue;
            }
            let n = e.metadata().map(|m| m.len()).unwrap_or(0);
            if best.as_ref().map(|b| n > b.0).unwrap_or(true) {
                best = Some((n, f.to_string_lossy().to_string()));
            }
        }
    }
    if let Some((_, f)) = best {
        return f;
    }
    for s in ["contents/screenshot.png", "contents/screenshot.jpg"] {
        if p.join(s).is_file() {
            return p.join(s).to_string_lossy().to_string();
        }
    }
    String::new()
}

/// The picture in this shape, when it's been read (None while it's being read, or when it can't be).
pub fn get(path: &str, c: Cut) -> Option<Handle> {
    if path.is_empty() {
        return None;
    }
    cache().lock().ok()?.get(&key(path, c)).cloned().flatten()
}

/// Read this picture again the next time it's asked for (its file has a new picture in it).
pub fn forget(path: &str) {
    if path.is_empty() {
        return;
    }
    if let Ok(mut c) = cache().lock() {
        let pre = format!("{path}|");
        c.retain(|k, _| !k.starts_with(&pre));
    }
}

fn decode(path: &str, c: Cut) -> Option<Handle> {
    let f = file_of(path);
    if f.is_empty() {
        return None;
    }
    if std::fs::metadata(&f).map(|m| m.len() > MAX_BYTES).unwrap_or(true) {
        return None;
    }
    if let Ok((w, h)) = ::image::image_dimensions(&f) {
        if (w as u64) * (h as u64) > 80_000_000 {
            return None;
        }
    }
    let img = ::image::ImageReader::open(&f).ok()?.with_guessed_format().ok()?.decode().ok()?;
    let (iw, ih) = (img.width().max(1), img.height().max(1));
    if c.fit {
        // The whole picture, as big as fits in twice the shape (never bigger than it is).
        let k = ((c.w * 2) as f64 / iw as f64).min((c.h * 2) as f64 / ih as f64).min(1.0);
        let (tw, th) = (((iw as f64) * k).round().max(1.0) as u32, ((ih as f64) * k).round().max(1.0) as u32);
        let px = if (tw, th) == (iw, ih) { img.to_rgba8() } else { img.resize_exact(tw, th, ::image::imageops::FilterType::Triangle).to_rgba8() };
        let (w, h) = (px.width(), px.height());
        return Some(Handle::from_rgba(w, h, px.into_raw()));
    }
    // The middle of the picture in the shape's proportions.
    let want = c.w.max(1) as f64 / c.h.max(1) as f64;
    let (cw, ch) = if iw as f64 / ih as f64 > want {
        (((ih as f64) * want).round().max(1.0) as u32, ih)
    } else {
        (iw, ((iw as f64) / want).round().max(1.0) as u32)
    };
    let (cw, ch) = (cw.min(iw), ch.min(ih));
    let cut = img.crop_imm((iw - cw) / 2, (ih - ch) / 2, cw, ch);
    let (tw, th) = ((c.w * 2).min(cw).max(1), (c.h * 2).min(ch).max(1));
    let mut px = cut.resize_exact(tw, th, ::image::imageops::FilterType::Triangle).to_rgba8();
    if c.round {
        // Everything outside the circle see-through, its edge softened over a pixel.
        let (w, h) = (px.width() as f32, px.height() as f32);
        let (cx, cy, r) = (w / 2.0, h / 2.0, w.min(h) / 2.0);
        for (x, y, p) in px.enumerate_pixels_mut() {
            let d = ((x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2)).sqrt();
            let a = (r - d + 0.5).clamp(0.0, 1.0);
            p.0[3] = (p.0[3] as f32 * a).round() as u8;
        }
    }
    let (w, h) = (px.width(), px.height());
    Some(Handle::from_rgba(w, h, px.into_raw()))
}

/// Reads the pictures not read yet in these shapes, off the window's thread; None when there's nothing to read.
/// The page turns the answer into a message of its own (so it's drawn again with them).
pub fn load(items: Vec<(String, Cut)>) -> Option<impl Future<Output = ()> + Send + 'static> {
    let mut todo: Vec<(String, Cut)> = Vec::new();
    {
        let mut c = cache().lock().ok()?;
        for (p, cut) in items {
            let k = key(&p, cut);
            if p.is_empty() || c.contains_key(&k) {
                continue;
            }
            // Marked at once, so it's asked for only once.
            c.insert(k, None);
            todo.push((p, cut));
        }
    }
    if todo.is_empty() {
        return None;
    }
    let (tx, rx) = futures::channel::oneshot::channel::<()>();
    let _ = std::thread::Builder::new().name("hearth-settings-pictures".into()).spawn(move || {
        for (p, cut) in todo {
            let h = decode(&p, cut);
            if let Ok(mut c) = cache().lock() {
                c.insert(key(&p, cut), h);
            }
        }
        let _ = tx.send(());
    });
    Some(async move {
        let _ = rx.await;
    })
}

/// The picture (already in its shape) drawn w × h.
pub fn tile<'a, M: 'a>(h: Handle, w: f32, ht: f32) -> Element<'a, M> {
    image(h).width(Length::Fixed(w)).height(Length::Fixed(ht)).content_fit(ContentFit::Fill).into()
}

/// The whole picture inside w × h (an app's screenshot or icon: SetStore.qml's PreserveAspectFit).
pub fn fit<'a, M: 'a>(h: Handle, w: f32, ht: f32) -> Element<'a, M> {
    image(h).width(Length::Fixed(w)).height(Length::Fixed(ht)).content_fit(ContentFit::Contain).into()
}

/// Your picture in its circle (as Mantel shows it), `size` across.
pub fn round<'a, M: 'a>(h: Handle, size: f32) -> Element<'a, M> {
    tile(h, size, size)
}
