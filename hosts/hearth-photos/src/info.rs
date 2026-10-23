//! The info panel: the file (size, dates, type, dimensions) and, from EXIF, the camera and how the photo was taken.

use crate::formats::{ext, mime};
use serde::Serialize;
use std::path::Path;
use std::time::UNIX_EPOCH;

#[derive(Serialize, Debug, Default)]
pub struct Info {
    pub name: String,
    pub path: String,
    pub folder: String,
    pub size: u64,
    pub mime: String,
    /// Milliseconds since 1970.
    pub modified: Option<u64>,
    pub created: Option<u64>,
    /// As stored (the page swaps them for turned pictures; `orientation` says how).
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub orientation: u8,
    /// When the photo was taken, as the camera wrote it ("2024-06-01 18:22:05").
    pub taken: Option<String>,
    /// Labelled camera details, in the order they're shown.
    pub camera: Vec<(String, String)>,
    /// Decimal degrees, for "Show on a map".
    pub gps: Option<(f64, f64)>,
}

fn ms(t: std::io::Result<std::time::SystemTime>) -> Option<u64> {
    t.ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_millis() as u64)
}

pub fn read(path: &Path) -> Result<Info, String> {
    let meta = std::fs::metadata(path).map_err(|e| crate::friendly_io(&e))?;
    let mut info = Info {
        name: path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
        path: path.to_string_lossy().into_owned(),
        folder: path.parent().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default(),
        size: meta.len(),
        mime: mime(&ext(path)).into(),
        modified: ms(meta.modified()),
        created: ms(meta.created()),
        orientation: 1,
        ..Default::default()
    };
    if let Ok((w, h)) = image::image_dimensions(path) { info.width = Some(w); info.height = Some(h); }
    if ext(path) == "svg" { if let Some((w, h)) = svg_size(path) { info.width = Some(w); info.height = Some(h); } }
    if let Ok(f) = std::fs::File::open(path) {
        if let Ok(e) = exif::Reader::new().read_from_container(&mut std::io::BufReader::new(f)) { add_exif(&mut info, &e); }
    }
    Ok(info)
}

/// The width and height an SVG says it has (its width/height, or its viewBox).
fn svg_size(path: &Path) -> Option<(u32, u32)> {
    let text = std::fs::read_to_string(path).ok()?;
    let start = text.find("<svg")?;
    let tag = &text[start..start + text[start..].find('>')?];
    let attr = |name: &str| -> Option<String> {
        let at = tag.find(&format!(" {name}=")).or_else(|| tag.find(&format!("\n{name}=")))? + name.len() + 2;
        let q = tag[at..].chars().next()?;
        let rest = &tag[at + 1..];
        Some(rest[..rest.find(q)?].to_string())
    };
    let num = |s: &str| s.trim().trim_end_matches("px").parse::<f64>().ok();
    if let (Some(w), Some(h)) = (attr("width").and_then(|s| num(&s)), attr("height").and_then(|s| num(&s))) {
        return Some((w.round() as u32, h.round() as u32));
    }
    let vb: Vec<f64> = attr("viewBox")?.split(|c: char| c == ' ' || c == ',').filter_map(|s| s.parse().ok()).collect();
    (vb.len() == 4).then(|| (vb[2].round() as u32, vb[3].round() as u32))
}

fn add_exif(info: &mut Info, e: &exif::Exif) {
    use exif::{In, Tag};
    let get = |t: Tag| e.get_field(t, In::PRIMARY);
    let text = |t: Tag| get(t).map(|f| f.display_value().with_unit(e).to_string().trim().trim_matches('"').trim().to_string()).filter(|s| !s.is_empty());
    if let Some(o) = get(Tag::Orientation).and_then(|f| f.value.get_uint(0)) { if (1..=8).contains(&o) { info.orientation = o as u8; } }
    // Width and height from EXIF only when the file's own header couldn't be read (HEIC).
    if info.width.is_none() {
        info.width = get(Tag::PixelXDimension).and_then(|f| f.value.get_uint(0));
        info.height = get(Tag::PixelYDimension).and_then(|f| f.value.get_uint(0));
    }
    info.taken = text(Tag::DateTimeOriginal).or_else(|| text(Tag::DateTime));
    let mut cam = Vec::new();
    let make = text(Tag::Make).unwrap_or_default();
    let model = text(Tag::Model).unwrap_or_default();
    // "Canon Canon EOS R6" reads badly: the model often starts with the make.
    let camera = if !make.is_empty() && model.to_lowercase().starts_with(&make.to_lowercase().split_whitespace().next().unwrap_or("").to_string()) { model.clone() } else { format!("{make} {model}").trim().to_string() };
    if !camera.is_empty() { cam.push(("Camera".into(), camera)); }
    if let Some(l) = text(Tag::LensModel) { cam.push(("Lens".into(), l)); }
    let mut shot = Vec::new();
    if let Some(f) = get(Tag::FNumber) { if let exif::Value::Rational(ref v) = f.value { if let Some(r) = v.first() { if r.denom != 0 { shot.push(format!("ƒ/{}", trim_num(r.to_f64()))); } } } }
    if let Some(f) = get(Tag::ExposureTime) {
        if let exif::Value::Rational(ref v) = f.value {
            if let Some(r) = v.first() {
                let s = r.to_f64();
                if s > 0.0 { shot.push(if s < 1.0 { format!("1/{} s", (1.0 / s).round()) } else { format!("{} s", trim_num(s)) }); }
            }
        }
    }
    if let Some(iso) = get(Tag::PhotographicSensitivity).and_then(|f| f.value.get_uint(0)) { shot.push(format!("ISO {iso}")); }
    if let Some(f) = get(Tag::FocalLength) {
        if let exif::Value::Rational(ref v) = f.value {
            if let Some(r) = v.first() {
                let mut s = format!("{} mm", trim_num(r.to_f64()));
                if let Some(eq) = get(Tag::FocalLengthIn35mmFilm).and_then(|f| f.value.get_uint(0)) { if eq > 0 { s += &format!(" ({eq} mm full-frame)"); } }
                shot.push(s);
            }
        }
    }
    if !shot.is_empty() { cam.push(("Exposure".into(), shot.join("  ·  "))); }
    if let Some(f) = get(Tag::Flash).and_then(|f| f.value.get_uint(0)) { cam.push(("Flash".into(), if f & 1 == 1 { "Fired".into() } else { "Didn't fire".into() })); }
    if let Some(s) = text(Tag::Software) { cam.push(("Software".into(), s)); }
    if let Some(s) = text(Tag::Artist) { cam.push(("By".into(), s)); }
    if let Some(s) = text(Tag::Copyright) { cam.push(("Copyright".into(), s)); }
    info.gps = gps(e);
    info.camera = cam;
}

fn trim_num(v: f64) -> String {
    let s = format!("{v:.1}");
    s.strip_suffix(".0").map(str::to_string).unwrap_or(s)
}

fn gps(e: &exif::Exif) -> Option<(f64, f64)> {
    use exif::{In, Tag, Value};
    let deg = |t: Tag| -> Option<f64> {
        match &e.get_field(t, In::PRIMARY)?.value {
            Value::Rational(v) if v.len() >= 3 => Some(v[0].to_f64() + v[1].to_f64() / 60.0 + v[2].to_f64() / 3600.0),
            _ => None,
        }
    };
    let side = |t: Tag| e.get_field(t, In::PRIMARY).map(|f| f.display_value().to_string()).unwrap_or_default();
    let mut lat = deg(Tag::GPSLatitude)?;
    let mut lon = deg(Tag::GPSLongitude)?;
    if side(Tag::GPSLatitudeRef).contains('S') { lat = -lat; }
    if side(Tag::GPSLongitudeRef).contains('W') { lon = -lon; }
    (lat.is_finite() && lon.is_finite() && (lat != 0.0 || lon != 0.0)).then_some((lat, lon))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_details() {
        let d = crate::imaging::tests::sample_dir("info");
        let img = crate::imaging::tests::gradient(300, 200);
        let p = d.join("x.jpg");
        let j = crate::imaging::encode_jpeg(&img, 80).unwrap();
        let crate::orient::Patch::Done(j) = crate::orient::set_jpeg_orientation(&j, 6, false).unwrap() else { panic!() };
        std::fs::write(&p, j).unwrap();
        let i = read(&p).unwrap();
        assert_eq!((i.width, i.height, i.orientation), (Some(300), Some(200), 6));
        assert_eq!(i.mime, "image/jpeg");
        assert!(i.modified.is_some());
        std::fs::write(d.join("v.svg"), r#"<?xml version="1.0"?><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 32"><rect/></svg>"#).unwrap();
        let s = read(&d.join("v.svg")).unwrap();
        assert_eq!((s.width, s.height), (Some(64), Some(32)));
        std::fs::remove_dir_all(&d).unwrap();
    }
}
