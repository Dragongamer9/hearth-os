//! Pictures' thumbnails (the icon views and the details pane, as in the approved mockup files-6): decoded on a thread
//! of their own, a few at a time, only for the pictures on screen, shrunk and handed to the window as pixels. Nothing
//! is written to disk. Also a picture's size (from its header only) and, for photos, when it was taken (EXIF).

use iced::widget::image::Handle;
use std::future::Future;
use std::io::Read;

#[derive(Debug, Clone)]
pub struct Thumb {
    pub path: String,
    pub mtime: i64,
    pub handle: Option<Handle>,
    pub w: u32,
    pub h: u32,
    pub taken: String,
}

/// Pictures this big aren't decoded for a thumbnail (it would take too long and too much memory).
const MAX_BYTES: u64 = 60 * 1024 * 1024;

fn one(path: &str, mtime: i64, max: u32) -> Thumb {
    let mut t = Thumb { path: path.to_string(), mtime, handle: None, w: 0, h: 0, taken: String::new() };
    if let Ok((w, h)) = image::image_dimensions(path) {
        t.w = w;
        t.h = h;
    }
    t.taken = exif_taken(path);
    let big = std::fs::metadata(path).map(|m| m.len() > MAX_BYTES).unwrap_or(true);
    if big || (t.w as u64) * (t.h as u64) > 80_000_000 {
        return t;
    }
    let img = image::ImageReader::open(path).ok().and_then(|r| r.with_guessed_format().ok()).and_then(|r| r.decode().ok());
    if let Some(img) = img {
        let th = img.thumbnail(max, max).to_rgba8();
        let (w, h) = (th.width(), th.height());
        t.handle = Some(Handle::from_rgba(w, h, th.into_raw()));
    }
    t
}

/// Thumbnails for these pictures (path, modified), at most `max` px on their long side.
pub fn load(paths: Vec<(String, i64)>, max: u32) -> impl Future<Output = Vec<Thumb>> + Send + 'static {
    let (tx, rx) = futures::channel::oneshot::channel();
    let _ = std::thread::Builder::new().name("hearth-files-thumbs".into()).spawn(move || {
        let out: Vec<Thumb> = paths.iter().map(|(p, m)| one(p, *m, max)).collect();
        let _ = tx.send(out);
    });
    async move { rx.await.unwrap_or_default() }
}

/// "Nov 10, 23:55" from a JPEG's EXIF date ("2026:11:10 23:55:01"), or "".
pub fn exif_taken(path: &str) -> String {
    let low = path.to_lowercase();
    if !(low.ends_with(".jpg") || low.ends_with(".jpeg")) {
        return String::new();
    }
    let mut buf = vec![0u8; 65536];
    let n = match std::fs::File::open(path).and_then(|mut f| f.read(&mut buf)) {
        Ok(n) => n,
        Err(_) => return String::new(),
    };
    let b = &buf[..n];
    // The first "YYYY:MM:DD HH:MM:SS" in the EXIF block: DateTimeOriginal or DateTime.
    let mut i = 0;
    while i + 19 <= b.len() {
        let w = &b[i..i + 19];
        let dig = |j: usize| w[j].is_ascii_digit();
        if (0..4).all(dig) && w[4] == b':' && dig(5) && dig(6) && w[7] == b':' && dig(8) && dig(9) && w[10] == b' ' && dig(11) && dig(12) && w[13] == b':' && dig(14) && dig(15) && w[16] == b':' {
            let s = String::from_utf8_lossy(w).to_string();
            let num = |a: usize, l: usize| s[a..a + l].parse::<usize>().unwrap_or(0);
            let months = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
            let m = num(5, 2);
            if !(1..=12).contains(&m) {
                return String::new();
            }
            let y = num(0, 4) as i32;
            let this_year = hearth_ui::time::local(hearth_ui::time::now()).year;
            return if y == this_year {
                format!("{} {}, {}:{}", months[m - 1], num(8, 2), &s[11..13], &s[14..16])
            } else {
                format!("{} {}, {}", months[m - 1], num(8, 2), y)
            };
        }
        i += 1;
    }
    String::new()
}
