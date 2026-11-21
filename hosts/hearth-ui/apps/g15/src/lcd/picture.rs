//! Pictures on your own screens (the approved mockup g15-6: "Pictures are turned into the screen's dots for you").
//!
//! A picture element keeps its dots in g15.json, already made, so the service only copies them onto the screen and
//! never opens the picture file itself (nothing heavy at rest):
//!
//!   {"type": "picture", "x": 0, "y": 0, "w": 40, "h": 43, "bits": "<base64>", "bw": 40, "bh": 43,
//!    "src": "/home/…/dad.png", "shade": true, "threshold": 128}
//!
//! `bw` x `bh` is the size the dots were made at (older elements without them: `w` x `h`); while you drag a corner the
//! dots are stretched to the new size, and when you let go the app makes them again from `src` at that size (if the
//! file is still there). `shade`: Floyd-Steinberg dithering (greys become patterns of dots) or sharp (dark or light);
//! `threshold`: how dark a dot has to be.
//!
//! PNG and JPEG, read with the `image` crate. The last few pictures read are kept in memory (keyed by the file and
//! its change time), so resizing and changing the settings don't read the file again each time.

use serde_json::{json, Value};
use std::sync::Mutex;

/// A picture as brightness 0-255, w x h.
#[derive(Clone)]
pub struct Gray {
    pub w: usize,
    pub h: usize,
    pub px: Vec<u8>,
}

static CACHE: Mutex<Vec<(String, u128, Gray)>> = Mutex::new(Vec::new());
const KEEP: usize = 4;
/// Pictures bigger than this are made smaller first (the screen is 160 x 43; this keeps memory small).
const MAX_SIDE: u32 = 640;

fn mtime(path: &str) -> u128 {
    std::fs::metadata(path).and_then(|m| m.modified()).ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_nanos()).unwrap_or(0)
}

/// Reads a PNG or JPEG as brightness (transparent parts count as light: the screen's background).
pub fn load(path: &str) -> Result<Gray, String> {
    let t = mtime(path);
    if let Ok(c) = CACHE.lock() {
        if let Some((_, _, g)) = c.iter().find(|(p, m, _)| p == path && *m == t) {
            return Ok(g.clone());
        }
    }
    let img = image::open(path).map_err(|e| e.to_string())?;
    let img = if img.width() > MAX_SIDE || img.height() > MAX_SIDE { img.thumbnail(MAX_SIDE, MAX_SIDE) } else { img };
    let rgba = img.to_rgba8();
    let (w, h) = (rgba.width() as usize, rgba.height() as usize);
    if w == 0 || h == 0 {
        return Err("it's empty".into());
    }
    let px: Vec<u8> = rgba
        .pixels()
        .map(|p| {
            let [r, g, b, a] = p.0;
            let l = (r as f32 * 0.299 + g as f32 * 0.587 + b as f32 * 0.114) / 255.0;
            let a = a as f32 / 255.0;
            ((l * a + (1.0 - a)) * 255.0).round().clamp(0.0, 255.0) as u8
        })
        .collect();
    let g = Gray { w, h, px };
    if let Ok(mut c) = CACHE.lock() {
        c.retain(|(p, _, _)| p != path);
        c.push((path.to_string(), t, g.clone()));
        while c.len() > KEEP {
            c.remove(0);
        }
    }
    Ok(g)
}

/// The size a new picture gets: the screen's height (or less, for a small picture), its own shape, at most the
/// screen's width.
pub fn fit_size(w: usize, h: usize) -> (usize, usize) {
    let oh = 43usize.min(h.max(1));
    let ow = ((w * oh) / h.max(1)).clamp(1, 160);
    (ow, oh)
}

/// Turns a picture into dots at w x h with the element's choices (Frame::picture's format).
pub fn dots(g: &Gray, w: usize, h: usize, shade: bool, threshold: u8) -> Vec<u8> {
    super::dither_with(&g.px, g.w, g.h, w.max(1), h.max(1), threshold, shade)
}

fn opts(e: &Value) -> (bool, u8) {
    (e.get("shade").and_then(|v| v.as_bool()).unwrap_or(true), e.get("threshold").and_then(|v| v.as_u64()).unwrap_or(128).clamp(1, 254) as u8)
}

/// A new picture element from a file, at (x, y).
pub fn element(path: &str, x: i32, y: i32) -> Result<Value, String> {
    let g = load(path)?;
    let (w, h) = fit_size(g.w, g.h);
    let bits = dots(&g, w, h, true, 128);
    Ok(json!({"type": "picture", "x": x, "y": y, "w": w, "h": h, "bw": w, "bh": h, "bits": super::base64(&bits), "src": path, "shade": true, "threshold": 128}))
}

/// Makes a picture element's dots again at its size and choices: from its file when it's still there, otherwise by
/// stretching the dots it has. Returns false when nothing changed.
pub fn redo(e: &mut Value) -> bool {
    let n = |e: &Value, k: &str, d: i64| e.get(k).and_then(|v| v.as_i64()).unwrap_or(d);
    let (w, h) = (n(e, "w", 1).clamp(1, 160) as usize, n(e, "h", 1).clamp(1, 43) as usize);
    let (bw, bh) = (n(e, "bw", w as i64) as usize, n(e, "bh", h as i64) as usize);
    let (shade, threshold) = opts(e);
    let src = e.get("src").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let bits = match (!src.is_empty()).then(|| load(&src)).and_then(|r| r.ok()) {
        Some(g) => dots(&g, w, h, shade, threshold),
        None => {
            if bw == w && bh == h {
                return false;
            }
            let old = super::screen::bits_of(e);
            stretch(&old, bw, bh, w, h)
        }
    };
    e["bits"] = json!(super::base64(&bits));
    e["bw"] = json!(w);
    e["bh"] = json!(h);
    true
}

/// Dots made at bw x bh, stretched (nearest dot) to w x h.
pub fn stretch(bits: &[u8], bw: usize, bh: usize, w: usize, h: usize) -> Vec<u8> {
    let mut f = super::Frame::new();
    f.picture_scaled(0, 0, w as i32, h as i32, bits, bw as i32, bh as i32);
    let stride = (w + 7) / 8;
    let mut out = vec![0u8; stride * h];
    for y in 0..h {
        for x in 0..w {
            if f.get(x as i32, y as i32) {
                out[y * stride + x / 8] |= 0x80 >> (x % 8);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    fn count(bits: &[u8]) -> u32 {
        bits.iter().map(|b| b.count_ones()).sum()
    }
    #[test]
    fn dithers_greys_into_patterns() {
        // Mid grey: about half the dots, in a pattern; black: all; white: none.
        let grey = vec![128u8; 64 * 16];
        let bits = super::super::dither(&grey, 64, 16, 64, 16);
        let n = count(&bits);
        assert!((400..=620).contains(&n), "mid grey made {n} of 1024 dots");
        assert_eq!(count(&super::super::dither(&vec![0u8; 64], 8, 8, 8, 8)), 64);
        assert_eq!(count(&super::super::dither(&vec![255u8; 64], 8, 8, 8, 8)), 0);
        // Sharp: the same grey just under the threshold is all dots, above it none.
        assert_eq!(count(&super::super::dither_with(&vec![120u8; 64], 8, 8, 8, 8, 128, false)), 64);
        assert_eq!(count(&super::super::dither_with(&vec![140u8; 64], 8, 8, 8, 8, 128, false)), 0);
        // A higher threshold makes it darker.
        let lo = count(&super::super::dither_with(&grey, 64, 16, 64, 16, 90, true));
        let hi = count(&super::super::dither_with(&grey, 64, 16, 64, 16, 170, true));
        assert!(lo < n && n < hi, "{lo} {n} {hi}");
    }
    #[test]
    fn scales_down_and_keeps_rows_whole() {
        // A 200 x 100 picture, dark left half: scaled to 20 x 10, the left 10 columns are dots.
        let mut px = vec![255u8; 200 * 100];
        for y in 0..100 {
            for x in 0..100 {
                px[y * 200 + x] = 0;
            }
        }
        let bits = super::super::dither(&px, 200, 100, 20, 10);
        assert_eq!(bits.len(), 3 * 10, "rows of 20 dots take 3 bytes");
        let mut f = super::super::Frame::new();
        f.picture(0, 0, 20, 10, &bits);
        for y in 0..10 {
            for x in 0..20 {
                assert_eq!(f.get(x, y), x < 10, "dot {x},{y}");
            }
        }
        assert_eq!(fit_size(200, 100), (86, 43));
        assert_eq!(fit_size(1000, 10), (160, 10));
    }
    #[test]
    fn stretches_old_dots_to_a_new_size() {
        let bits = vec![0b1000_0000u8, 0]; // 2 x 2: top-left dot only
        let s = stretch(&bits, 2, 2, 4, 4);
        let mut f = super::super::Frame::new();
        f.picture(0, 0, 4, 4, &s);
        assert!(f.get(0, 0) && f.get(1, 1) && !f.get(2, 0) && !f.get(0, 2));
        // An element without its file: redo stretches and records the new size.
        let mut e = json!({"type": "picture", "w": 4, "h": 4, "bw": 2, "bh": 2, "bits": super::super::base64(&bits)});
        assert!(redo(&mut e));
        assert_eq!(e["bw"], json!(4));
        assert!(!redo(&mut e), "nothing to do at the same size");
    }
    #[test]
    fn reads_a_png_once() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("dad.png");
        let mut img = image::GrayImage::new(80, 40);
        for (x, _, px) in img.enumerate_pixels_mut() {
            px.0 = [if x < 40 { 0 } else { 255 }];
        }
        img.save(&p).unwrap();
        let path = p.to_string_lossy().to_string();
        let e = element(&path, 3, 0).unwrap();
        assert_eq!((e["w"].as_i64(), e["h"].as_i64(), e["x"].as_i64()), (Some(80), Some(40), Some(3)));
        // Cached: still there after the file is gone.
        std::fs::remove_file(&p).unwrap();
        assert!(CACHE.lock().unwrap().iter().any(|(k, _, _)| *k == path));
    }
}
