//! Hearth Documents: Hearth OS's reader for PDFs (pdf.js), e-books (EPUB, epub.js) and comic books (CBZ). The
//! window is ui/index.html, drawn by the system's web view; pdf.js and epub.js are vendored in ui/vendor (nothing
//! is downloaded while it runs). This library keeps the recent documents and where you were in each (state.rs) and
//! reads comic book archives (cbz.rs).

pub mod cbz;
pub mod state;

use std::path::{Path, PathBuf};

/// The kinds of documents Documents opens, by extension.
pub fn kind(path: &Path) -> Option<&'static str> {
    match path.extension()?.to_string_lossy().to_ascii_lowercase().as_str() {
        "pdf" => Some("pdf"),
        "epub" => Some("epub"),
        "cbz" => Some("cbz"),
        _ => None,
    }
}

/// A path from the command line or a link: file:// URIs (from .desktop files' %U) become paths.
pub fn path_from_arg(arg: &str) -> PathBuf {
    if let Some(rest) = arg.strip_prefix("file://") {
        let rest = rest.strip_prefix("localhost").unwrap_or(rest);
        let decoded = percent_encoding::percent_decode_str(rest).decode_utf8_lossy().into_owned();
        #[cfg(windows)]
        let decoded = decoded.trim_start_matches('/').replace('/', "\\");
        return PathBuf::from(decoded);
    }
    PathBuf::from(arg)
}

pub fn friendly_io(e: &std::io::Error) -> String {
    match e.kind() {
        std::io::ErrorKind::PermissionDenied => "You don't have permission to open that file".into(),
        std::io::ErrorKind::NotFound => "That file isn't there any more".into(),
        _ => e.to_string(),
    }
}

/// "page 2" before "page 10", ignoring case.
pub fn natural_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;
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
                let (ta, tb) = (na.trim_start_matches('0'), nb.trim_start_matches('0'));
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
    fn kinds_and_paths() {
        assert_eq!(kind(Path::new("/a/Book.EPUB")), Some("epub"));
        assert_eq!(kind(Path::new("x.pdf")), Some("pdf"));
        assert_eq!(kind(Path::new("x.txt")), None);
        #[cfg(not(windows))]
        assert_eq!(path_from_arg("file:///home/a/My%20Book.pdf"), PathBuf::from("/home/a/My Book.pdf"));
        let mut v = vec!["p10.jpg", "p2.jpg", "P1.jpg"];
        v.sort_by(|a, b| natural_cmp(a, b));
        assert_eq!(v, vec!["P1.jpg", "p2.jpg", "p10.jpg"]);
    }
}
