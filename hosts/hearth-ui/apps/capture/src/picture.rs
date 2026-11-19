//! Pictures read for showing: a screenshot for the editor (its pixels, and the two small copies Blur and Pixelate
//! show in the view, as Mantel's editor does with Qt's), the area picker's still picture, and the recent list's
//! thumbnails (made small once, off the window's thread, so the list stays light).

use iced::widget::image::Handle;

#[derive(Clone)]
pub struct Pic {
    pub path: String,
    pub w: u32,
    pub h: u32,
    /// Its pixels (straight, not premultiplied, RGBA: w × h × 4), kept once, in the handle.
    pub handle: Handle,
    /// Pixelate's small copy (1/14 of the size) and Blur's (1/24).
    pub pix: Handle,
    pub blur: Handle,
}

impl Pic {
    pub fn rgba(&self) -> &[u8] {
        match &self.handle {
            Handle::Rgba { pixels, .. } => pixels,
            _ => &[],
        }
    }
}

impl std::fmt::Debug for Pic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Pic({} {}x{})", self.path, self.w, self.h)
    }
}

/// PNG, JPEG or WEBP bytes of the file → (w, h, straight RGBA). PNG only is read here (the helper and KWin give PNG;
/// Save as .jpg / .webp are never opened again by the editor).
pub fn decode(path: &str) -> Result<(u32, u32, Vec<u8>), String> {
    let data = std::fs::read(path).map_err(|e| format!("{e}"))?;
    let pm = tiny_skia::Pixmap::decode_png(&data).map_err(|e| format!("not a picture Capture can open ({e})"))?;
    let (w, h) = (pm.width(), pm.height());
    let mut out = Vec::with_capacity((w * h * 4) as usize);
    for px in pm.pixels() {
        let c = px.demultiply();
        out.extend_from_slice(&[c.red(), c.green(), c.blue(), c.alpha()]);
    }
    Ok((w, h, out))
}

/// The picture made K times smaller, each new pixel the average of the ones it covers (at least 1 × 1).
pub fn shrink(rgba: &[u8], w: u32, h: u32, k: f32) -> (u32, u32, Vec<u8>) {
    let nw = ((w as f32 * k).round() as u32).max(1);
    let nh = ((h as f32 * k).round() as u32).max(1);
    let mut out = vec![0u8; (nw * nh * 4) as usize];
    for ny in 0..nh {
        let y0 = (ny as u64 * h as u64 / nh as u64) as u32;
        let y1 = (((ny + 1) as u64 * h as u64 / nh as u64) as u32).max(y0 + 1).min(h);
        for nx in 0..nw {
            let x0 = (nx as u64 * w as u64 / nw as u64) as u32;
            let x1 = (((nx + 1) as u64 * w as u64 / nw as u64) as u32).max(x0 + 1).min(w);
            let mut s = [0u64; 4];
            let mut n = 0u64;
            // A big picture's square is sampled on a grid of at most 8 × 8 (enough for a colour, and quick).
            let sy = ((y1 - y0) / 8).max(1);
            let sx = ((x1 - x0) / 8).max(1);
            let mut y = y0;
            while y < y1 {
                let mut x = x0;
                while x < x1 {
                    let i = ((y * w + x) * 4) as usize;
                    for c in 0..4 {
                        s[c] += rgba[i + c] as u64;
                    }
                    n += 1;
                    x += sx;
                }
                y += sy;
            }
            let o = ((ny * nw + nx) * 4) as usize;
            for c in 0..4 {
                out[o + c] = (s[c] / n.max(1)) as u8;
            }
        }
    }
    (nw, nh, out)
}

pub fn load(path: &str) -> Result<Pic, String> {
    let (w, h, rgba) = decode(path)?;
    let (pw, ph, pix) = shrink(&rgba, w, h, crate::draw::PIX);
    let (bw, bh, blur) = shrink(&rgba, w, h, crate::draw::BLUR);
    Ok(Pic {
        path: path.to_string(),
        w,
        h,
        handle: Handle::from_rgba(w, h, rgba),
        pix: Handle::from_rgba(pw, ph, pix),
        blur: Handle::from_rgba(bw, bh, blur),
    })
}

/// A thumbnail at most MAX px wide (the recent list's, 480 as Mantel's).
pub fn thumb(path: &str, max: u32) -> Option<(Handle, u32, u32)> {
    let (w, h, rgba) = decode(path).ok()?;
    if w <= max {
        return Some((Handle::from_rgba(w, h, rgba), w, h));
    }
    let k = max as f32 / w as f32;
    let (tw, th, small) = shrink(&rgba, w, h, k);
    Some((Handle::from_rgba(tw, th, small), w, h))
}

/// Loads off the window's thread.
pub fn load_async(path: String) -> impl std::future::Future<Output = Result<Pic, String>> + Send + 'static {
    let (tx, rx) = futures::channel::oneshot::channel();
    let _ = std::thread::Builder::new().name("capture-load".into()).spawn(move || {
        let _ = tx.send(load(&path));
    });
    async move { rx.await.unwrap_or_else(|_| Err("it stopped".into())) }
}

pub fn thumbs_async(paths: Vec<String>) -> impl std::future::Future<Output = Vec<(String, Option<(Handle, u32, u32)>)>> + Send + 'static {
    let (tx, rx) = futures::channel::oneshot::channel();
    let _ = std::thread::Builder::new().name("capture-thumbs".into()).spawn(move || {
        let v: Vec<_> = paths.into_iter().map(|p| {
            let t = thumb(&p, 480);
            (p, t)
        }).collect();
        let _ = tx.send(v);
    });
    async move { rx.await.unwrap_or_default() }
}
