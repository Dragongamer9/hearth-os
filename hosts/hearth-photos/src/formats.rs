//! Which files are pictures, what the web view draws by itself, and the order pictures are shown in.

use std::cmp::Ordering;
use std::path::Path;

/// Every picture type Photos lists in a folder.
pub const PICTURES: &[&str] = &[
    "jpg", "jpeg", "jpe", "jfif", "png", "apng", "gif", "webp", "bmp", "dib", "ico", "tif", "tiff", "svg", "avif", "heic",
    "heif", "hif", "jxl",
];

/// Types both web views (WebKitGTK and WebView2) draw by themselves. The others go through `imaging::convert`
/// (AVIF and JPEG XL are tried in the web view first: some builds have them).
pub const WEB_NATIVE: &[&str] = &["jpg", "jpeg", "jpe", "jfif", "png", "apng", "gif", "webp", "bmp", "dib", "ico", "svg"];

/// Types the image crate decodes here (thumbnails, turning, converting).
pub const DECODED_HERE: &[&str] = &["jpg", "jpeg", "jpe", "jfif", "png", "apng", "gif", "webp", "bmp", "dib", "ico", "tif", "tiff"];

pub fn ext(path: &Path) -> String {
    path.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default()
}

pub fn is_picture(path: &Path) -> bool { PICTURES.contains(&ext(path).as_str()) }

pub fn is_jpeg(path: &Path) -> bool { matches!(ext(path).as_str(), "jpg" | "jpeg" | "jpe" | "jfif") }

pub fn mime(ext: &str) -> &'static str {
    match ext {
        "jpg" | "jpeg" | "jpe" | "jfif" => "image/jpeg",
        "png" | "apng" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" | "dib" => "image/bmp",
        "ico" => "image/x-icon",
        "tif" | "tiff" => "image/tiff",
        "svg" => "image/svg+xml",
        "avif" => "image/avif",
        "heic" | "hif" => "image/heic",
        "heif" => "image/heif",
        "jxl" => "image/jxl",
        _ => "application/octet-stream",
    }
}

/// "IMG_2.jpg" before "IMG_10.jpg", ignoring case: the order file managers show.
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let (mut ai, mut bi) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (ai.peek().copied(), bi.peek().copied()) {
            (None, None) => return a.cmp(b),
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) if x.is_ascii_digit() && y.is_ascii_digit() => {
                let mut na = String::new();
                while let Some(c) = ai.peek().copied().filter(|c| c.is_ascii_digit()) { na.push(c); ai.next(); }
                let mut nb = String::new();
                while let Some(c) = bi.peek().copied().filter(|c| c.is_ascii_digit()) { nb.push(c); bi.next(); }
                let ta = na.trim_start_matches('0');
                let tb = nb.trim_start_matches('0');
                let o = ta.len().cmp(&tb.len()).then_with(|| ta.cmp(tb));
                if o != Ordering::Equal { return o; }
            }
            (Some(x), Some(y)) => {
                let o = x.to_lowercase().cmp(y.to_lowercase());
                if o != Ordering::Equal { return o; }
                ai.next();
                bi.next();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn natural_order() {
        let mut v = vec!["IMG_10.jpg", "img_2.jpg", "IMG_1.jpg", "a.png", "IMG_02.jpg", "B.png"];
        v.sort_by(|a, b| natural_cmp(a, b));
        assert_eq!(v, vec!["a.png", "B.png", "IMG_1.jpg", "IMG_02.jpg", "img_2.jpg", "IMG_10.jpg"]);
    }

    #[test]
    fn picture_types() {
        assert!(is_picture(Path::new("/x/Photo.JPG")));
        assert!(is_picture(Path::new("a.heic")));
        assert!(!is_picture(Path::new("a.txt")));
        assert!(!is_picture(Path::new("jpg")));
        assert_eq!(mime("webp"), "image/webp");
    }
}
