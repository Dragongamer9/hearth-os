//! Saving: a turn or flip made in the viewer, and pictures from the editor. The original is only ever replaced
//! when you ask for that (Overwrite, or saving a turn); everything else is a new file next to it.

use crate::formats::{ext, is_jpeg};
use crate::orient::{self, Patch};
use crate::{free_name, imaging, write_atomic};
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Serialize, Debug, PartialEq)]
pub struct Saved {
    pub path: String,
    /// "exif": only the orientation in the JPEG's details changed (nothing re-compressed); "pixels": the picture
    /// was turned and written again without loss (PNG, BMP, TIFF, lossless WebP…); "copy": saved as a new file.
    pub how: String,
    pub message: String,
}

fn stem(path: &Path) -> String { path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "Picture".into()) }

fn animated_gif(path: &Path) -> bool {
    use image::AnimationDecoder;
    let Ok(f) = std::fs::File::open(path) else { return false };
    image::codecs::gif::GifDecoder::new(std::io::BufReader::new(f)).map(|d| d.into_frames().take(2).count() > 1).unwrap_or(false)
}

fn animated_webp(path: &Path) -> bool {
    // An animated WebP has an ANIM chunk; reading the first few KB is enough.
    std::fs::read(path).map(|b| b.windows(4).take(4096).any(|w| w == b"ANIM")).unwrap_or(false)
}

/// Saves turn/flip `op` (an EXIF orientation, 1 to 8, made of the turns and flips done on screen).
pub fn save_turn(path: &Path, op: u8) -> Result<Saved, String> {
    if op == 1 { return Ok(Saved { path: path.to_string_lossy().into(), how: "none".into(), message: "Nothing to save".into() }); }
    let e = ext(path);
    if is_jpeg(path) {
        let bytes = std::fs::read(path).map_err(|e| crate::friendly_io(&e))?;
        let now = orient::jpeg_orientation(&bytes);
        match orient::set_jpeg_orientation(&bytes, orient::then(now, op), false)? {
            Patch::Done(out) => {
                write_atomic(path, &out)?;
                return Ok(Saved { path: path.to_string_lossy().into(), how: "exif".into(), message: "Saved without losing any quality".into() });
            }
            Patch::NoTag => {
                // Rare: EXIF details without an orientation entry. A turned copy (re-compressed at high quality).
                let img = orient::apply(imaging::decode_upright(path)?, op);
                let out = free_name(path.parent().unwrap_or(Path::new(".")), &stem(path), "turned", &e);
                write_atomic(&out, &imaging::encode_jpeg(&img, 95)?)?;
                return Ok(Saved { path: out.to_string_lossy().into(), how: "copy".into(), message: format!("Saved as a turned copy, {}", name(&out)) });
            }
        }
    }
    if (e == "gif" && animated_gif(path)) || (e == "webp" && animated_webp(path)) {
        return Err("Moving pictures can't be turned yet".into());
    }
    let fmt = match e.as_str() {
        "png" | "apng" => Some(image::ImageFormat::Png),
        "bmp" | "dib" => Some(image::ImageFormat::Bmp),
        "tif" | "tiff" => Some(image::ImageFormat::Tiff),
        "gif" => Some(image::ImageFormat::Gif),
        "webp" => Some(image::ImageFormat::WebP),
        _ => None,
    };
    let img = orient::apply(imaging::decode_upright(path)?, op);
    match fmt {
        Some(f) => {
            write_atomic(path, &imaging::encode_as(&img, f)?)?;
            let message = if e == "webp" { "Saved (as lossless WebP)" } else { "Saved without losing any quality" };
            Ok(Saved { path: path.to_string_lossy().into(), how: "pixels".into(), message: message.into() })
        }
        // SVG, HEIC, AVIF, JPEG XL, ICO: a turned PNG next to it.
        None => {
            let out = free_name(path.parent().unwrap_or(Path::new(".")), &stem(path), "turned", "png");
            write_atomic(&out, &imaging::encode_png(&img, false)?)?;
            Ok(Saved { path: out.to_string_lossy().into(), how: "copy".into(), message: format!("Saved as a turned copy, {}", name(&out)) })
        }
    }
}

fn name(p: &Path) -> String { p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default() }

/// Where an edit goes: a new file next to the original (the default), or over it.
pub fn edit_target(original: &Path, format: &str, overwrite: bool) -> Result<PathBuf, String> {
    let ext = match format { "jpeg" | "jpg" => "jpg", "png" => "png", "webp" => "webp", _ => return Err("Photos saves edits as JPEG, PNG or WebP".into()) };
    if overwrite {
        let same = match ext { "jpg" => is_jpeg(original), e => crate::formats::ext(original) == e };
        if !same { return Err("To save in another format, save a copy".into()); }
        return Ok(original.to_path_buf());
    }
    // Keep the original's spelling of the extension when it's the same type (".jpeg" stays ".jpeg").
    let keep = crate::formats::ext(original);
    let ext = if (ext == "jpg" && is_jpeg(original)) || keep == ext { original.extension().map(|e| e.to_string_lossy().into_owned()).unwrap_or(ext.into()) } else { ext.into() };
    Ok(free_name(original.parent().unwrap_or(Path::new(".")), &stem(original), "edited", &ext))
}

/// Writes an edited picture. `bytes` is what the editor made: PNG, or already JPEG/WebP when `encoded` (the web
/// view's own encoder, which has a quality setting). A PNG asked to be WebP is written as lossless WebP here; a
/// JPEG keeps the original JPEG's camera details (with orientation reset: the pixels are upright now).
pub fn save_edit(original: &Path, bytes: &[u8], format: &str, quality: u8, encoded: bool, overwrite: bool) -> Result<Saved, String> {
    let target = edit_target(original, format, overwrite)?;
    let mut out = match (format, encoded) {
        (_, true) | ("png", false) => bytes.to_vec(),
        ("webp", false) => imaging::encode_webp_lossless(&image::load_from_memory(bytes).map_err(|e| e.to_string())?)?,
        (_, false) => imaging::encode_jpeg(&image::load_from_memory(bytes).map_err(|e| e.to_string())?, quality)?,
    };
    if matches!(format, "jpeg" | "jpg") && is_jpeg(original) {
        if let Ok(orig) = std::fs::read(original) { out = with_exif_of(&orig, out); }
    }
    write_atomic(&target, &out)?;
    let message = if overwrite { "Saved".to_string() } else { format!("Saved as {}", name(&target)) };
    Ok(Saved { path: target.to_string_lossy().into(), how: if overwrite { "overwrite".into() } else { "copy".into() }, message })
}

/// `jpeg` with `orig`'s EXIF block put in (orientation set to 1, its old preview picture unlinked).
pub fn with_exif_of(orig: &[u8], jpeg: Vec<u8>) -> Vec<u8> {
    let Some(block) = exif_segment(orig) else { return jpeg };
    let Ok(Patch::Done(fixed)) = orient::set_jpeg_orientation(&[&[0xFFu8, 0xD8][..], &block[..], &[0xFFu8, 0xD9][..]].concat(), 1, true) else { return jpeg };
    let seg = &fixed[2..fixed.len() - 2];
    if jpeg.len() < 2 || jpeg[0] != 0xFF || jpeg[1] != 0xD8 { return jpeg; }
    // Any EXIF the new JPEG has already is replaced; it goes after JFIF's APP0 when there is one.
    let mut out = Vec::with_capacity(jpeg.len() + seg.len());
    out.extend_from_slice(&jpeg[..2]);
    let mut i = 2;
    if jpeg.get(2..4) == Some(&[0xFFu8, 0xE0][..]) {
        let len = u16::from_be_bytes([jpeg[4], jpeg[5]]) as usize;
        out.extend_from_slice(&jpeg[2..4 + len]);
        i = 4 + len;
    }
    out.extend_from_slice(seg);
    if jpeg.get(i..i + 2) == Some(&[0xFFu8, 0xE1][..]) && jpeg.get(i + 4..i + 10) == Some(&b"Exif\0\0"[..]) {
        i += 2 + u16::from_be_bytes([jpeg[i + 2], jpeg[i + 3]]) as usize;
    }
    out.extend_from_slice(&jpeg[i..]);
    out
}

/// The whole APP1 Exif segment (marker included) of a JPEG.
fn exif_segment(j: &[u8]) -> Option<Vec<u8>> {
    if j.len() < 4 || j[0] != 0xFF || j[1] != 0xD8 { return None; }
    let mut i = 2;
    while i + 4 <= j.len() && j[i] == 0xFF {
        let m = j[i + 1];
        if m == 0xDA || m == 0xD9 { break; }
        let len = u16::from_be_bytes([j[i + 2], j[i + 3]]) as usize;
        if i + 2 + len > j.len() { break; }
        if m == 0xE1 && j.get(i + 4..i + 10) == Some(&b"Exif\0\0"[..]) { return Some(j[i..i + 2 + len].to_vec()); }
        i += 2 + len;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::imaging::tests::{gradient, sample_dir};

    #[test]
    fn turning_a_jpeg_loses_nothing() {
        let d = sample_dir("turn");
        let p = d.join("a.jpg");
        let j = imaging::encode_jpeg(&gradient(120, 80), 85).unwrap();
        std::fs::write(&p, &j).unwrap();
        let s = save_turn(&p, orient::ROTATE_RIGHT).unwrap();
        assert_eq!(s.how, "exif");
        let after = std::fs::read(&p).unwrap();
        assert_eq!(orient::jpeg_orientation(&after), 6);
        assert!(after.ends_with(&j[j.len() - 200..])); // the picture data is byte for byte the same
        save_turn(&p, orient::ROTATE_LEFT).unwrap();
        assert_eq!(orient::jpeg_orientation(&std::fs::read(&p).unwrap()), 1);
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn turning_other_types() {
        let d = sample_dir("turn2");
        let img = gradient(120, 80);
        std::fs::write(d.join("a.png"), imaging::encode_png(&img, true).unwrap()).unwrap();
        let s = save_turn(&d.join("a.png"), orient::ROTATE_RIGHT).unwrap();
        assert_eq!(s.how, "pixels");
        let back = image::open(d.join("a.png")).unwrap();
        assert_eq!((back.width(), back.height()), (80, 120));
        // Lossless: turning back gives the very same pixels.
        save_turn(&d.join("a.png"), orient::ROTATE_LEFT).unwrap();
        assert_eq!(image::open(d.join("a.png")).unwrap().to_rgb8(), img.to_rgb8());
        // ICO (like SVG and HEIC) isn't turned in place: a PNG copy.
        std::fs::write(d.join("b.ico"), imaging::encode_as(&gradient(32, 16), image::ImageFormat::Ico).unwrap()).unwrap();
        let c = save_turn(&d.join("b.ico"), orient::FLIP_H).unwrap();
        assert_eq!(c.how, "copy");
        assert!(c.path.ends_with("b (turned).png"));
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn edits_go_to_a_copy() {
        let d = sample_dir("edit");
        let orig = d.join("Beach.jpeg");
        let j = imaging::encode_jpeg(&gradient(100, 50), 85).unwrap();
        let orient::Patch::Done(j) = orient::set_jpeg_orientation(&j, 6, false).unwrap() else { panic!() };
        std::fs::write(&orig, &j).unwrap();
        let png = imaging::encode_png(&gradient(50, 100), true).unwrap();
        let s = save_edit(&orig, &png, "jpeg", 90, false, false).unwrap();
        assert!(s.path.ends_with("Beach (edited).jpeg"), "{}", s.path);
        let out = std::fs::read(&s.path).unwrap();
        // The camera details came along, with the orientation reset.
        assert_eq!(orient::jpeg_orientation(&out), 1);
        assert!(exif_segment(&out).is_some());
        assert_eq!(image::load_from_memory(&out).unwrap().width(), 50);
        // The original is untouched.
        assert_eq!(std::fs::read(&orig).unwrap(), j);
        // A second copy gets the next free name; WebP from PNG is written here.
        let w = save_edit(&orig, &png, "webp", 90, false, false).unwrap();
        assert!(w.path.ends_with("Beach (edited).webp"));
        assert_eq!(image::open(&w.path).unwrap().height(), 100);
        // Overwrite only in the same format.
        assert!(save_edit(&orig, &png, "png", 90, false, true).is_err());
        let o = save_edit(&orig, &png, "jpeg", 90, false, true).unwrap();
        assert_eq!(PathBuf::from(&o.path), orig);
        assert_eq!(image::open(&orig).unwrap().width(), 50);
        std::fs::remove_dir_all(&d).unwrap();
    }
}
