//! The boot splashes' and startup animations' pictures (Mantel's SetBootCard.qml, SetBootFrames.qml and
//! SetBootPreview.qml): each choice's preview is a sheet of frames (tools/make-boot-previews.py, listed by
//! hearth-boot-look status: frames side by side, `columns` to a row, `fps` a second). A card shows its poster frame and
//! plays while the mouse is over it or the controller's focus is on it; the preview plays while it's open. Nothing
//! plays otherwise (no timer runs), and only one sheet's frames are kept at a time (dropped when it stops).
//!
//! Decoding happens on a thread of its own, as pics.rs does; the page is drawn again when the pictures are ready.
//! Looking at a preview changes nothing.

use hearth_ui::ui::{self, glyph, t, G};
use hearth_ui::Pal;
use iced::widget::image::Handle;
use iced::widget::{container, image};
use iced::{ContentFit, Element, Length};
use serde_json::Value;
use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex, OnceLock};

/// A sheet this big isn't decoded (the real ones are about 200 KB).
const MAX_BYTES: u64 = 40 * 1024 * 1024;
/// A card's picture on the page (SetBootCard.qml: 128 × 72).
pub const CARD_W: f32 = 128.0;
pub const CARD_H: f32 = 72.0;
/// The slate the pictures sit on (SetBootFrames.qml).
pub const SLATE: &str = "#0b1016";

/// The sheets' frame size, frames to a row and frames a second (hearth-boot-look status: preview).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Geo {
    pub fw: u32,
    pub fh: u32,
    pub cols: u32,
    pub fps: u32,
}

/// The sheets' shape from hearth-boot-look status (None: this version has no previews).
pub fn geo(status: &Value) -> Option<Geo> {
    let p = status.get("preview")?;
    let n = |k: &str| p.get(k).and_then(Value::as_u64).filter(|x| *x > 0 && *x < 10_000).map(|x| x as u32);
    Some(Geo { fw: n("frameWidth")?, fh: n("frameHeight")?, cols: n("columns")?, fps: n("fps")?.min(30) })
}

/// One choice's sheet: its file, how many frames and the poster frame.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Sheet {
    pub path: String,
    pub frames: u32,
    pub poster: u32,
}

/// A choice's sheet (hearth-boot-look's "preview": {sheet, frames, poster}), when it has one.
pub fn sheet(item: &Value) -> Option<Sheet> {
    let p = item.get("preview")?;
    let path = p.get("sheet").and_then(Value::as_str).unwrap_or("").to_string();
    if path.is_empty() {
        return None;
    }
    let frames = (p.get("frames").and_then(Value::as_u64).unwrap_or(1).clamp(1, 1000)) as u32;
    let poster = (p.get("poster").and_then(Value::as_u64).unwrap_or(0) as u32).min(frames - 1);
    Some(Sheet { path, frames, poster })
}

struct Store {
    /// Each sheet's poster frame, small (a card's picture, twice its size for sharp edges); None: can't be read.
    posters: HashMap<String, Option<Handle>>,
    /// The sheet playing now and its frames (None while they're being read).
    playing: Option<(String, Option<Arc<Vec<Handle>>>)>,
}

fn store() -> &'static Mutex<Store> {
    static S: OnceLock<Mutex<Store>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(Store { posters: HashMap::new(), playing: None }))
}

/// A sheet's pixels (RGB) at about `want_w` wide or more, and the sheet's frame shape at that size. The sheets are
/// JPEGs: decoded at 1/2, 1/4 or 1/8 of their size straight from the file when that's enough (a card's poster needs
/// half), so a poster never holds a whole sheet in memory; anything else is read whole by the image crate.
fn decode(path: &str, g: Geo, want_w: u32) -> Option<(::image::RgbImage, Geo)> {
    if std::fs::metadata(path).map(|m| m.len() > MAX_BYTES).unwrap_or(true) {
        return None;
    }
    let (img, ow) = match jpeg_scaled(path, want_w) {
        Some(x) => x,
        None => {
            if let Ok((w, h)) = ::image::image_dimensions(path) {
                if (w as u64) * (h as u64) > 60_000_000 {
                    return None;
                }
            }
            let img = ::image::ImageReader::open(path).ok()?.with_guessed_format().ok()?.decode().ok()?.to_rgb8();
            let w = img.width();
            (img, w)
        }
    };
    let k = img.width() as f64 / ow.max(1) as f64;
    let sg = Geo { fw: ((g.fw as f64 * k).round() as u32).max(1), fh: ((g.fh as f64 * k).round() as u32).max(1), ..g };
    Some((img, sg))
}

/// A JPEG decoded at the smallest of its own scales (1, 1/2, 1/4, 1/8) that's at least `want_w` wide, and its full
/// width. None when it isn't a JPEG this can read.
fn jpeg_scaled(path: &str, want_w: u32) -> Option<(::image::RgbImage, u32)> {
    let f = std::fs::File::open(path).ok()?;
    let mut d = jpeg_decoder::Decoder::new(std::io::BufReader::new(f));
    d.read_info().ok()?;
    let info = d.info()?;
    let (ow, oh) = (info.width as u32, info.height as u32);
    if ow == 0 || oh == 0 || (ow as u64) * (oh as u64) > 60_000_000 {
        return None;
    }
    let tw = want_w.clamp(1, ow);
    let th = ((oh as u64 * tw as u64) / ow as u64).max(1) as u32;
    let (w, h) = d.scale(tw as u16, th as u16).ok()?;
    let px = d.decode().ok()?;
    let rgb: Vec<u8> = match d.info()?.pixel_format {
        jpeg_decoder::PixelFormat::RGB24 => px,
        jpeg_decoder::PixelFormat::L8 => px.iter().flat_map(|&v| [v, v, v]).collect(),
        _ => return None,
    };
    Some((::image::RgbImage::from_raw(w as u32, h as u32, rgb)?, ow))
}

/// Frame `i` of a sheet (inside the sheet: a sheet that's smaller than it says gives what's there), as RGBA.
fn frame_of(img: &::image::RgbImage, g: Geo, i: u32) -> Option<::image::RgbaImage> {
    let (x, y) = ((i % g.cols) * g.fw, (i / g.cols) * g.fh);
    if x >= img.width() || y >= img.height() {
        return None;
    }
    let (w, h) = (g.fw.min(img.width() - x), g.fh.min(img.height() - y));
    Some(::image::DynamicImage::ImageRgb8(::image::imageops::crop_imm(img, x, y, w, h).to_image()).to_rgba8())
}

/// The memory a sheet took while it was read goes back to the system (a whole sheet is ~20 MB as pixels; without this
/// the window would keep holding it at rest).
fn give_back() {
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    unsafe {
        libc::malloc_trim(0);
    }
}

fn handle(px: ::image::RgbaImage) -> Handle {
    let (w, h) = (px.width(), px.height());
    Handle::from_rgba(w, h, px.into_raw())
}

/// A sheet's poster frame, once it's been read.
pub fn poster(s: &Sheet) -> Option<Handle> {
    store().lock().ok()?.posters.get(&s.path).cloned().flatten()
}

/// Reads the poster frames not read yet, off the window's thread (None: nothing to read).
pub fn load_posters(sheets: Vec<Sheet>, g: Geo) -> Option<impl Future<Output = ()> + Send + 'static> {
    let mut todo = Vec::new();
    {
        let mut st = store().lock().ok()?;
        for s in sheets {
            if !st.posters.contains_key(&s.path) {
                st.posters.insert(s.path.clone(), None);
                todo.push(s);
            }
        }
    }
    if todo.is_empty() {
        return None;
    }
    let (tx, rx) = futures::channel::oneshot::channel::<()>();
    let _ = std::thread::Builder::new().name("hearth-settings-boot-pictures".into()).spawn(move || {
        for s in todo {
            // Half the sheet's size is enough for a card (its frames are 200 wide then; a card is 128).
            let h = decode(&s.path, g, g.cols * CARD_W as u32).and_then(|(img, sg)| frame_of(&img, sg, s.poster)).map(|f| {
                let (w, h) = ((CARD_W as u32) * 2, (CARD_H as u32) * 2);
                handle(::image::imageops::resize(&f, w.min(f.width()), h.min(f.height()), ::image::imageops::FilterType::Triangle))
            });
            if let Ok(mut st) = store().lock() {
                st.posters.insert(s.path.clone(), h);
            }
        }
        give_back();
        let _ = tx.send(());
    });
    Some(async move {
        let _ = rx.await;
    })
}

/// The frames of the sheet playing, once they've been read.
pub fn frames(path: &str) -> Option<Arc<Vec<Handle>>> {
    match &store().lock().ok()?.playing {
        Some((p, Some(f))) if p == path => Some(f.clone()),
        _ => None,
    }
}

/// Starts reading this sheet's frames (the one before is dropped); None when it's already playing or being read.
pub fn load_frames(s: &Sheet, g: Geo) -> Option<impl Future<Output = ()> + Send + 'static> {
    {
        let mut st = store().lock().ok()?;
        if matches!(&st.playing, Some((p, _)) if *p == s.path) {
            return None;
        }
        st.playing = Some((s.path.clone(), None));
    }
    let s = s.clone();
    let (tx, rx) = futures::channel::oneshot::channel::<()>();
    let _ = std::thread::Builder::new().name("hearth-settings-boot-frames".into()).spawn(move || {
        let mut out = Vec::new();
        if let Some((img, sg)) = decode(&s.path, g, g.cols * g.fw) {
            for i in 0..s.frames {
                match frame_of(&img, sg, i) {
                    Some(f) => out.push(handle(f)),
                    None => break,
                }
            }
        }
        if let Ok(mut st) = store().lock() {
            // Still the one wanted (another may have started meanwhile).
            if matches!(&st.playing, Some((p, None)) if *p == s.path) {
                st.playing = Some((s.path.clone(), Some(Arc::new(out))));
            }
        }
        give_back();
        let _ = tx.send(());
    });
    Some(async move {
        let _ = rx.await;
    })
}

/// Nothing plays any more: its frames go.
pub fn stop() {
    let had = store().lock().ok().and_then(|mut st| st.playing.take()).is_some();
    if had {
        give_back();
    }
}

/// What a page keeps for its pictures: the frame shown (counted from the poster frame), the card under the mouse,
/// and the sheet that plays.
#[derive(Debug, Clone, Default)]
pub struct Flip {
    pub frame: u32,
    pub hover: String,
    pub playing: String,
}

impl Flip {
    /// The mouse is over a card (its sheet), or left one.
    pub fn hover(&mut self, path: String) {
        self.hover = path;
    }
    pub fn unhover(&mut self, path: &str) {
        if self.hover == path {
            self.hover.clear();
        }
    }
    /// A tick of the clock while something plays (`active`): the next frame, or its frames read first. Returns the
    /// reading to wait for, if it has to start.
    pub fn tick(&mut self, active: Option<&Sheet>, g: Option<Geo>) -> Option<impl Future<Output = ()> + Send + 'static> {
        let (Some(s), Some(g)) = (active, g) else {
            self.halt();
            return None;
        };
        if self.playing != s.path {
            self.playing = s.path.clone();
            self.frame = 0;
            return load_frames(s, g);
        }
        if frames(&s.path).is_some() {
            self.frame = self.frame.wrapping_add(1);
            return None;
        }
        load_frames(s, g)
    }
    /// Nothing plays: back to the poster frames, and the frames go.
    pub fn halt(&mut self) {
        if !self.playing.is_empty() {
            self.playing.clear();
            self.frame = 0;
            stop();
        }
    }
    /// The frame to show for this sheet now: playing, its next one; else its poster.
    pub fn handle(&self, s: &Sheet) -> Option<Handle> {
        if self.playing == s.path {
            if let Some(f) = frames(&s.path) {
                if !f.is_empty() {
                    let i = ((s.poster + self.frame) % s.frames.max(1)) as usize % f.len();
                    return Some(f[i].clone());
                }
            }
        }
        poster(s)
    }
}

/// How often the pictures move while one plays (the sheets' frames a second).
pub fn every(g: Option<Geo>) -> std::time::Duration {
    std::time::Duration::from_millis(1000 / g.map(|g| g.fps).unwrap_or(10).max(1) as u64)
}

/// The sheet that plays on a page: the preview's, else the card under the mouse, else the card the controller's focus
/// is on (by its name). `cards`: each card's name and sheet.
pub fn active(preview: Option<Sheet>, flip: &Flip, cards: &[(String, Sheet)]) -> Option<Sheet> {
    if preview.is_some() {
        return preview;
    }
    if !flip.hover.is_empty() {
        if let Some((_, s)) = cards.iter().find(|(_, s)| s.path == flip.hover) {
            return Some(s.clone());
        }
    }
    if crate::nav::shown() && crate::nav::in_page() {
        if let Some(f) = crate::nav::focused() {
            if let Some((_, s)) = cards.iter().find(|(n, _)| *n == f.title) {
                return Some(s.clone());
            }
        }
    }
    None
}

/// A picture on the slate, w × h: the frame (or poster), the display mark while there's none, or the words for a
/// choice with no preview in this version.
pub fn picture<'a, M: 'a>(p: &Pal, flip: &Flip, s: Option<&Sheet>, w: f32, h: f32, empty: &str) -> Element<'a, M> {
    let slate = hearth_ui::theme::hex(SLATE).unwrap_or(p.bg);
    let inner: Element<'a, M> = match s.and_then(|s| flip.handle(s)) {
        Some(hd) => image(hd).width(Length::Fixed(w - 2.0)).height(Length::Fixed(h - 2.0)).content_fit(ContentFit::Fill).into(),
        None if s.is_none() && !empty.is_empty() => t(empty.to_string(), 13.0, p.muted).into(),
        None => glyph(G::Display, p.muted, 22.0, 1.8),
    };
    container(inner).center_x(Length::Fixed(w)).center_y(Length::Fixed(h)).style(ui::boxed(slate, 0.0, Some((p.line, 1.0)))).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_sheets_from_hearth_boot_look() {
        let st = serde_json::json!({"preview": {"frameWidth": 400, "frameHeight": 225, "columns": 8, "fps": 10},
            "movie": {"list": [{"id": "aurora", "preview": {"sheet": "/x/movie-aurora.jpg", "frames": 50, "poster": 36}}, {"id": "plain", "preview": {}}]}});
        assert_eq!(geo(&st), Some(Geo { fw: 400, fh: 225, cols: 8, fps: 10 }));
        assert_eq!(geo(&serde_json::json!({"preview": null})), None);
        let l = st["movie"]["list"].as_array().unwrap();
        assert_eq!(sheet(&l[0]), Some(Sheet { path: "/x/movie-aurora.jpg".into(), frames: 50, poster: 36 }));
        assert_eq!(sheet(&l[1]), None);
        assert_eq!(every(geo(&st)), std::time::Duration::from_millis(100));
    }

    #[test]
    fn a_frame_is_cut_from_its_place_in_the_sheet() {
        let g = Geo { fw: 4, fh: 2, cols: 3, fps: 10 };
        let mut img = ::image::RgbImage::new(12, 4);
        for (x, y, px) in img.enumerate_pixels_mut() {
            *px = ::image::Rgb([(x / 4) as u8, (y / 2) as u8, 0]);
        }
        let f = frame_of(&img, g, 4).unwrap();
        assert_eq!((f.width(), f.height()), (4, 2));
        assert_eq!(f.get_pixel(0, 0).0, [1, 1, 0, 255]);
        assert!(frame_of(&img, g, 6).is_none());
    }

    #[test]
    fn nothing_plays_without_a_sheet() {
        let mut f = Flip { frame: 3, hover: String::new(), playing: "/x/a.jpg".into() };
        assert!(f.tick(None, Some(Geo { fw: 4, fh: 2, cols: 3, fps: 10 })).is_none());
        assert!(f.playing.is_empty() && f.frame == 0);
        f.hover("/x/b.jpg".into());
        f.unhover("/x/c.jpg");
        assert_eq!(f.hover, "/x/b.jpg");
        f.unhover("/x/b.jpg");
        assert!(f.hover.is_empty());
    }
}
