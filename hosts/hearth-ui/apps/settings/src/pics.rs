//! Pictures drawn as the pictures themselves (Personalization's wallpapers, your account picture, the login screen's
//! background): decoded once on a thread of their own, shrunk, and kept in memory for as long as Settings is open.
//! Nothing is written anywhere and nothing runs at rest: a page asks for the pictures it shows, once, and redraws when
//! they're ready. A picture that can't be read stays drawn as colour, as before.

use iced::widget::canvas::{self, fill, Fill, Frame, Geometry, Path, Style};
use iced::widget::image::Handle;
use iced::widget::{canvas as canvas_w, image, stack};
use iced::{mouse, Color, ContentFit, Element, Length, Point, Rectangle, Renderer, Size, Theme};
use std::collections::HashMap;
use std::future::Future;
use std::sync::{Mutex, OnceLock};

/// Pictures this big aren't decoded (it would take too long and too much memory), as in Hearth Files.
const MAX_BYTES: u64 = 60 * 1024 * 1024;
/// The long side the pictures are shrunk to: enough for the biggest tile (the login screen's preview).
pub const SIDE: u32 = 640;

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

/// The picture, when it's been read (None while it's being read, or when it can't be).
pub fn get(path: &str) -> Option<Handle> {
    if path.is_empty() {
        return None;
    }
    cache().lock().ok()?.get(path).cloned().flatten()
}

/// Read this picture again the next time it's asked for (its file has a new picture in it).
pub fn forget(path: &str) {
    if let Ok(mut c) = cache().lock() {
        c.remove(path);
    }
}

fn decode(path: &str) -> Option<Handle> {
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
    let th = img.thumbnail(SIDE, SIDE).to_rgba8();
    let (w, h) = (th.width(), th.height());
    Some(Handle::from_rgba(w, h, th.into_raw()))
}

/// Reads the pictures not read yet, off the window's thread; None when there's nothing to read. The page turns the
/// answer into a message of its own (so it's drawn again with them).
pub fn load(paths: Vec<String>) -> Option<impl Future<Output = ()> + Send + 'static> {
    let mut todo: Vec<String> = Vec::new();
    {
        let mut c = cache().lock().ok()?;
        for p in paths {
            if p.is_empty() || c.contains_key(&p) || todo.contains(&p) {
                continue;
            }
            // Marked at once, so it's asked for only once.
            c.insert(p.clone(), None);
            todo.push(p);
        }
    }
    if todo.is_empty() {
        return None;
    }
    let (tx, rx) = futures::channel::oneshot::channel::<()>();
    let _ = std::thread::Builder::new().name("hearth-settings-pictures".into()).spawn(move || {
        for p in todo {
            let h = decode(&p);
            if let Ok(mut c) = cache().lock() {
                c.insert(p, h);
            }
        }
        let _ = tx.send(());
    });
    Some(async move {
        let _ = rx.await;
    })
}

/// The picture filling a w × h tile (cut to fit, never stretched), its corners rounded by `r`.
pub fn tile<'a, M: 'a>(h: Handle, w: f32, ht: f32, r: f32) -> Element<'a, M> {
    image(h).width(Length::Fixed(w)).height(Length::Fixed(ht)).content_fit(ContentFit::Cover).border_radius(r).into()
}

/// The picture in a circle (your account picture, as Mantel shows it): cut to fit the square, then everything
/// outside the circle covered with `bg`, the colour it sits on (so it's round on every renderer).
pub fn round<'a, M: 'a>(h: Handle, size: f32, bg: Color) -> Element<'a, M> {
    stack![
        image(h).width(Length::Fixed(size)).height(Length::Fixed(size)).content_fit(ContentFit::Cover),
        canvas_w(Mask { bg }).width(Length::Fixed(size)).height(Length::Fixed(size)),
    ]
    .width(Length::Fixed(size))
    .height(Length::Fixed(size))
    .into()
}

struct Mask {
    bg: Color,
}

impl<M> canvas::Program<M> for Mask {
    type State = ();
    fn draw(&self, _s: &(), renderer: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        let mut f = Frame::new(renderer, b.size());
        let r = b.width.min(b.height) / 2.0;
        // The square with the circle cut out of it (even-odd: the circle is a hole).
        let path = Path::new(|pb| {
            pb.rectangle(Point::ORIGIN, Size::new(b.width, b.height));
            pb.circle(Point::new(b.width / 2.0, b.height / 2.0), r);
        });
        f.fill(&path, Fill { style: Style::Solid(self.bg), rule: fill::Rule::EvenOdd });
        vec![f.into_geometry()]
    }
}
