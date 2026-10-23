//! Hearth Photos: Hearth OS's picture viewer (and simple editor), on Linux (Mantel) and Windows (Ember). The
//! window is ui/index.html, drawn by the system's web view (WebKitGTK or WebView2), which shows the pictures it
//! can decode itself (PNG, JPEG, GIF, WebP, BMP, ICO, SVG, AVIF where the web view has it). This library does what
//! a page can't: the folder's pictures (folder.rs), thumbnails and pictures the web view can't decode (imaging.rs:
//! TIFF here, HEIC/HEIF and others through the system's converters), camera details (info.rs), turning pictures
//! without losing quality (orient.rs: JPEGs get a new EXIF orientation, nothing is re-compressed), saving edits
//! (save.rs) and the system's parts: Trash, clipboard, wallpaper, the file manager (system.rs).

pub mod folder;
pub mod formats;
pub mod imaging;
pub mod info;
pub mod orient;
pub mod save;
pub mod system;

use std::path::{Path, PathBuf};

/// A path from the command line or a link: file:// URIs (from .desktop files' %U) become paths.
pub fn path_from_arg(arg: &str) -> PathBuf {
    if let Some(rest) = arg.strip_prefix("file://") {
        let rest = rest.strip_prefix("localhost").unwrap_or(rest);
        let decoded = percent_encoding::percent_decode_str(rest).decode_utf8_lossy().into_owned();
        // file:///C:/x on Windows
        #[cfg(windows)]
        let decoded = decoded.trim_start_matches('/').replace('/', "\\");
        return PathBuf::from(decoded);
    }
    PathBuf::from(arg)
}

/// Writes a file next to where it goes and then moves it into place, so a half-written file never replaces a
/// picture (a crash, a full drive).
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let dir = path.parent().ok_or("That folder isn't there")?;
    let name = path.file_name().ok_or("No file name")?.to_string_lossy().into_owned();
    let tmp = dir.join(format!(".{name}.hearth-photos-tmp"));
    std::fs::write(&tmp, bytes).map_err(|e| friendly_io(&e))?;
    // Keep the old file's permissions (a read-only-for-others picture stays that way).
    if let Ok(meta) = std::fs::metadata(path) { let _ = std::fs::set_permissions(&tmp, meta.permissions()); }
    std::fs::rename(&tmp, path).map_err(|e| { let _ = std::fs::remove_file(&tmp); friendly_io(&e) })
}

/// A free name for a copy next to a picture: "Beach (edited).jpg", then "Beach (edited 2).jpg"…
pub fn free_name(dir: &Path, stem: &str, label: &str, ext: &str) -> PathBuf {
    let first = dir.join(format!("{stem} ({label}).{ext}"));
    if !first.exists() { return first; }
    for n in 2..10_000 {
        let p = dir.join(format!("{stem} ({label} {n}).{ext}"));
        if !p.exists() { return p; }
    }
    dir.join(format!("{stem} ({label} {}).{ext}", std::process::id()))
}

pub fn friendly_io(e: &std::io::Error) -> String {
    match e.kind() {
        std::io::ErrorKind::PermissionDenied => "You don't have permission to change that file or folder".into(),
        std::io::ErrorKind::NotFound => "That picture isn't there any more".into(),
        _ if e.raw_os_error() == Some(28) => "The drive is full".into(),
        _ => e.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uris_become_paths() {
        #[cfg(not(windows))]
        {
            assert_eq!(path_from_arg("file:///home/a/My%20Pics/x.jpg"), PathBuf::from("/home/a/My Pics/x.jpg"));
            assert_eq!(path_from_arg("file://localhost/tmp/y.png"), PathBuf::from("/tmp/y.png"));
            assert_eq!(path_from_arg("/tmp/z.png"), PathBuf::from("/tmp/z.png"));
        }
        #[cfg(windows)]
        assert_eq!(path_from_arg("file:///C:/Users/a/b%20c.jpg"), PathBuf::from("C:\\Users\\a\\b c.jpg"));
    }

    #[test]
    fn copies_get_free_names() {
        let d = std::env::temp_dir().join(format!("hp-free-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let a = free_name(&d, "Beach", "edited", "jpg");
        assert_eq!(a.file_name().unwrap(), "Beach (edited).jpg");
        std::fs::write(&a, b"x").unwrap();
        assert_eq!(free_name(&d, "Beach", "edited", "jpg").file_name().unwrap(), "Beach (edited 2).jpg");
        write_atomic(&a, b"new").unwrap();
        assert_eq!(std::fs::read(&a).unwrap(), b"new");
        std::fs::remove_dir_all(&d).unwrap();
    }
}
