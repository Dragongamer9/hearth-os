//! Decoding pictures here (for thumbnails, for types the web view can't draw, and for turning non-JPEGs), with a
//! cache of thumbnails and converted pictures in ~/.cache/hearth-photos.
//! The image crate decodes PNG, JPEG, GIF, WebP, BMP, ICO and TIFF. HEIC/HEIF (and AVIF or JPEG XL where the web
//! view can't) go through the system's converters when they're installed: libheif's heif-dec / heif-convert,
//! djxl, or ImageMagick.

use crate::formats::{ext, DECODED_HERE};
use crate::orient;
use image::DynamicImage;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Condvar, Mutex};
use std::time::UNIX_EPOCH;

/// At most this many pictures are decoded at once (a big folder asks for many thumbnails together).
static SLOTS: Mutex<usize> = Mutex::new(0);
static FREED: Condvar = Condvar::new();

struct Slot;
impl Slot {
    fn take() -> Slot {
        let max = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).clamp(2, 8);
        let mut n = SLOTS.lock().unwrap();
        while *n >= max { n = FREED.wait(n).unwrap(); }
        *n += 1;
        Slot
    }
}
impl Drop for Slot {
    fn drop(&mut self) {
        *SLOTS.lock().unwrap() -= 1;
        FREED.notify_one();
    }
}

pub fn cache_dir(kind: &str) -> PathBuf {
    let base = std::env::var_os("HEARTH_PHOTOS_CACHE").map(PathBuf::from)
        .or_else(|| dirs::cache_dir().map(|d| d.join("hearth-photos")))
        .unwrap_or_else(|| std::env::temp_dir().join("hearth-photos"));
    base.join(kind)
}

/// A name for a cached picture: the file's path, size and last change (a changed picture gets a new name).
fn cache_key(path: &Path, extra: &str) -> String {
    let meta = std::fs::metadata(path).ok();
    let len = meta.as_ref().map(|m| m.len()).unwrap_or(0);
    let mtime = meta.and_then(|m| m.modified().ok()).and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_nanos()).unwrap_or(0);
    // FNV-1a: stable between builds, unlike std's hasher.
    let mut h: u64 = 0xcbf29ce484222325;
    for b in path.to_string_lossy().as_bytes().iter().chain(format!("|{len}|{mtime}|{extra}").as_bytes()) {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}

/// The orientation a file's EXIF details give (1 when there are none).
pub fn file_orientation(path: &Path) -> u8 {
    let Ok(f) = std::fs::File::open(path) else { return 1 };
    let mut r = std::io::BufReader::new(f);
    exif::Reader::new().read_from_container(&mut r).ok()
        .and_then(|e| e.get_field(exif::Tag::Orientation, exif::In::PRIMARY).and_then(|f| f.value.get_uint(0)))
        .map(|v| v as u8).filter(|v| (1..=8).contains(v)).unwrap_or(1)
}

/// Decodes a picture as stored (not yet turned upright). Types the image crate can't read go through a converter.
pub fn decode(path: &Path) -> Result<DynamicImage, String> {
    let e = ext(path);
    if DECODED_HERE.contains(&e.as_str()) {
        let r = image::ImageReader::open(path).map_err(|e| crate::friendly_io(&e))?
            .with_guessed_format().map_err(|e| e.to_string())?;
        let decoded = { let _slot = Slot::take(); r.decode() };
        return match decoded {
            Ok(img) => Ok(img),
            // Mislabelled files (a PNG called .jpg) are handled by the guess above; anything else may still open
            // through a converter.
            Err(err) => decode_external(path).map_err(|_| format!("Photos couldn't read this picture ({err})")),
        };
    }
    decode_external(path)
}

/// The system's converters, tried in order, each writing a PNG.
fn decode_external(path: &Path) -> Result<DynamicImage, String> {
    let _slot = Slot::take();
    let dir = cache_dir("tmp");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let out = dir.join(format!("{}-{}.png", cache_key(path, "ext"), std::process::id()));
    let p = path.to_string_lossy().into_owned();
    let o = out.to_string_lossy().into_owned();
    let e = ext(path);
    let mut tries: Vec<Vec<String>> = Vec::new();
    if matches!(e.as_str(), "heic" | "heif" | "hif" | "avif") {
        tries.push(vec!["heif-dec".into(), p.clone(), o.clone()]);
        tries.push(vec!["heif-convert".into(), p.clone(), o.clone()]);
    }
    if e == "jxl" { tries.push(vec!["djxl".into(), p.clone(), o.clone()]); }
    tries.push(vec!["magick".into(), format!("{p}[0]"), o.clone()]);
    tries.push(vec!["convert".into(), format!("{p}[0]"), o.clone()]);
    let mut tried = Vec::new();
    for argv in tries {
        let Ok(status) = Command::new(&argv[0]).args(&argv[1..]).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).status() else { continue };
        tried.push(argv[0].clone());
        // heif-convert writes "name-1.png" etc. for files with several pictures.
        let made = if out.exists() { Some(out.clone()) } else {
            let alt = dir.join(format!("{}-1.png", out.file_stem().unwrap().to_string_lossy()));
            alt.exists().then_some(alt)
        };
        if status.success() {
            if let Some(m) = made {
                let img = image::open(&m).map_err(|e| e.to_string());
                let _ = std::fs::remove_file(&m);
                if let Ok(i) = img { return Ok(i); }
            }
        }
    }
    Err(match e.as_str() {
        "heic" | "heif" | "hif" => "HEIC pictures need libheif's tools (heif-dec), which aren't installed".into(),
        "jxl" => "JPEG XL pictures need djxl (libjxl's tools), which isn't installed".into(),
        _ if tried.is_empty() => "Photos can't read this kind of picture".into(),
        _ => "Photos couldn't read this picture".into(),
    })
}

/// Decoded and turned upright (the way the web view shows it).
pub fn decode_upright(path: &Path) -> Result<DynamicImage, String> {
    let img = decode(path)?;
    // libheif already applies HEIF's own turn; EXIF's orientation counts for the rest.
    let o = if matches!(ext(path).as_str(), "heic" | "heif" | "hif" | "avif") { 1 } else { file_orientation(path) };
    Ok(orient::apply(img, o))
}

pub fn encode_png(img: &DynamicImage, fast: bool) -> Result<Vec<u8>, String> {
    use image::codecs::png::{CompressionType, FilterType, PngEncoder};
    let mut buf = Vec::new();
    let enc = PngEncoder::new_with_quality(&mut buf, if fast { CompressionType::Fast } else { CompressionType::Default }, FilterType::Adaptive);
    img.write_with_encoder(enc).map_err(|e| e.to_string())?;
    Ok(buf)
}

pub fn encode_jpeg(img: &DynamicImage, quality: u8) -> Result<Vec<u8>, String> {
    let mut buf = Vec::new();
    let enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, quality.clamp(1, 100));
    DynamicImage::ImageRgb8(img.to_rgb8()).write_with_encoder(enc).map_err(|e| e.to_string())?;
    Ok(buf)
}

/// WebP as the image crate writes it: lossless.
pub fn encode_webp_lossless(img: &DynamicImage) -> Result<Vec<u8>, String> {
    let mut buf = Vec::new();
    let enc = image::codecs::webp::WebPEncoder::new_lossless(&mut buf);
    DynamicImage::ImageRgba8(img.to_rgba8()).write_with_encoder(enc).map_err(|e| e.to_string())?;
    Ok(buf)
}

pub fn encode_as(img: &DynamicImage, format: image::ImageFormat) -> Result<Vec<u8>, String> {
    let mut buf = Vec::new();
    match format {
        image::ImageFormat::Png => return encode_png(img, false),
        image::ImageFormat::Jpeg => return encode_jpeg(img, 92),
        image::ImageFormat::WebP => return encode_webp_lossless(img),
        image::ImageFormat::Gif | image::ImageFormat::Bmp | image::ImageFormat::Ico => {
            DynamicImage::ImageRgba8(img.to_rgba8()).write_to(&mut Cursor::new(&mut buf), format).map_err(|e| e.to_string())?
        }
        _ => img.write_to(&mut Cursor::new(&mut buf), format).map_err(|e| e.to_string())?,
    }
    Ok(buf)
}

/// A small copy for the strip of thumbnails (and the folder's grid): (bytes, MIME type). JPEGs use the preview
/// picture the camera stored in their EXIF details when it's big enough, which is much faster than decoding a
/// 24-megapixel photo. Cached.
pub fn thumbnail(path: &Path, size: u32) -> Result<(Vec<u8>, &'static str), String> {
    let size = size.clamp(32, 1024);
    let dir = cache_dir("thumbs");
    let key = cache_key(path, &format!("t{size}"));
    for (ext, mime) in [("jpg", "image/jpeg"), ("png", "image/png")] {
        if let Ok(b) = std::fs::read(dir.join(format!("{key}.{ext}"))) { return Ok((b, mime)); }
    }
    let o = file_orientation(path);
    let small = if crate::formats::is_jpeg(path) && size <= 256 { let _slot = Slot::take(); exif_preview(path) } else { None };
    let img = match small {
        Some(s) if s.width().max(s.height()) >= size.min(160) => orient::apply(s, o),
        _ => decode_upright(path)?,
    };
    let t = img.thumbnail(size, size);
    let (bytes, ext, mime) = if t.color().has_alpha() {
        (encode_png(&t, true)?, "png", "image/png")
    } else {
        (encode_jpeg(&t, 82)?, "jpg", "image/jpeg")
    };
    if std::fs::create_dir_all(&dir).is_ok() { let _ = crate::write_atomic(&dir.join(format!("{key}.{ext}")), &bytes); }
    Ok((bytes, mime))
}

/// The small preview picture inside a JPEG's EXIF details, when its shape matches the photo (some cameras store a
/// letterboxed one).
fn exif_preview(path: &Path) -> Option<DynamicImage> {
    let f = std::fs::File::open(path).ok()?;
    let e = exif::Reader::new().read_from_container(&mut std::io::BufReader::new(f)).ok()?;
    let off = e.get_field(exif::Tag::JPEGInterchangeFormat, exif::In::THUMBNAIL)?.value.get_uint(0)? as usize;
    let len = e.get_field(exif::Tag::JPEGInterchangeFormatLength, exif::In::THUMBNAIL)?.value.get_uint(0)? as usize;
    let data = e.buf().get(off..off.checked_add(len)?)?;
    let t = image::load_from_memory_with_format(data, image::ImageFormat::Jpeg).ok()?;
    let (w, h) = image::image_dimensions(path).ok()?;
    let a = w as f64 / h.max(1) as f64;
    let b = t.width() as f64 / t.height().max(1) as f64;
    ((a - b).abs() / a < 0.03).then_some(t)
}

/// A picture the web view can't draw, as a PNG it can (cached): TIFF, HEIC, and AVIF or JPEG XL on web views
/// without them.
pub fn convert(path: &Path) -> Result<Vec<u8>, String> {
    let dir = cache_dir("view");
    let file = dir.join(format!("{}.png", cache_key(path, "view")));
    if let Ok(b) = std::fs::read(&file) { return Ok(b); }
    let img = decode_upright(path)?;
    let bytes = { let _slot = Slot::take(); encode_png(&img, true)? };
    if std::fs::create_dir_all(&dir).is_ok() { let _ = crate::write_atomic(&file, &bytes); }
    Ok(bytes)
}

/// Clears out cached pictures not used in 30 days (run once at start, in the background).
pub fn tidy_cache() {
    let old = std::time::SystemTime::now() - std::time::Duration::from_secs(30 * 86400);
    for kind in ["thumbs", "view", "tmp"] {
        let Ok(rd) = std::fs::read_dir(cache_dir(kind)) else { continue };
        for e in rd.flatten() {
            let stale = e.metadata().ok().and_then(|m| m.accessed().or_else(|_| m.modified()).ok()).map(|t| t < old).unwrap_or(false);
            if stale || kind == "tmp" { let _ = std::fs::remove_file(e.path()); }
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    pub fn sample_dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("hp-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        std::env::set_var("HEARTH_PHOTOS_CACHE", d.join("cache"));
        d
    }

    pub fn gradient(w: u32, h: u32) -> DynamicImage {
        DynamicImage::ImageRgb8(image::RgbImage::from_fn(w, h, |x, y| image::Rgb([(x * 255 / w) as u8, (y * 255 / h) as u8, 90])))
    }

    #[test]
    fn thumbnails_and_conversion() {
        let d = sample_dir("img");
        let img = gradient(640, 400);
        std::fs::write(d.join("a.png"), encode_png(&img, true).unwrap()).unwrap();
        std::fs::write(d.join("b.tiff"), encode_as(&img, image::ImageFormat::Tiff).unwrap()).unwrap();
        std::fs::write(d.join("c.jpg"), encode_jpeg(&img, 80).unwrap()).unwrap();
        let (t, mime) = thumbnail(&d.join("a.png"), 128).unwrap();
        assert_eq!(mime, "image/jpeg");
        let ti = image::load_from_memory(&t).unwrap();
        assert_eq!((ti.width(), ti.height()), (128, 80));
        // Second time from the cache, the same bytes.
        assert_eq!(thumbnail(&d.join("a.png"), 128).unwrap().0, t);
        // TIFF becomes a PNG the web view can show.
        let png = convert(&d.join("b.tiff")).unwrap();
        assert_eq!(image::load_from_memory(&png).unwrap().width(), 640);
        // A JPEG marked "turned right" gets an upright (tall) thumbnail.
        let j = std::fs::read(d.join("c.jpg")).unwrap();
        let orient::Patch::Done(turned) = orient::set_jpeg_orientation(&j, 6, false).unwrap() else { panic!() };
        std::fs::write(d.join("d.jpg"), turned).unwrap();
        let (t, _) = thumbnail(&d.join("d.jpg"), 100).unwrap();
        let ti = image::load_from_memory(&t).unwrap();
        assert_eq!(ti.height(), 100);
        assert!((62..=63).contains(&ti.width()), "{}", ti.width());
        // Something that isn't a picture at all.
        std::fs::write(d.join("e.png"), b"not a picture").unwrap();
        assert!(thumbnail(&d.join("e.png"), 100).is_err());
        std::fs::remove_dir_all(&d).unwrap();
    }
}
