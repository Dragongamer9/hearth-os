//! Hearth Paint without the window: pictures in and out (read and written as bytes; the page draws them), the
//! page's own file picker, and the clipboard (wl-paste / wl-copy on Wayland, xclip on X11), so a screenshot you
//! copied can be pasted in to mark up.

use serde::Serialize;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub const IMAGE_EXTS: &[&str] = &["png", "jpg", "jpeg", "webp", "bmp", "gif"];
pub const SAVE_EXTS: &[&str] = &["png", "jpg", "jpeg"];
pub const MAX_IMAGE_BYTES: u64 = 200 * 1024 * 1024;

pub fn home() -> PathBuf {
    std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from).unwrap_or_else(std::env::temp_dir)
}

/// Pictures (where screenshots go, in Pictures/Screenshots), or home.
pub fn pictures() -> PathBuf {
    let p = home().join("Pictures");
    if p.is_dir() { p } else { home() }
}

pub fn ext_of(p: &Path) -> String { p.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default() }

/// What kind of picture these bytes are, from their first bytes.
pub fn sniff(b: &[u8]) -> Option<&'static str> {
    if b.starts_with(b"\x89PNG\r\n\x1a\n") { Some("image/png") }
    else if b.starts_with(&[0xFF, 0xD8, 0xFF]) { Some("image/jpeg") }
    else if b.len() > 12 && &b[..4] == b"RIFF" && &b[8..12] == b"WEBP" { Some("image/webp") }
    else if b.starts_with(b"GIF87a") || b.starts_with(b"GIF89a") { Some("image/gif") }
    else if b.starts_with(b"BM") { Some("image/bmp") }
    else { None }
}

pub fn read_image(path: &Path) -> Result<(Vec<u8>, &'static str), String> {
    let m = std::fs::metadata(path).map_err(|_| "That picture isn't there.".to_string())?;
    if !m.is_file() { return Err("That isn't a file.".into()); }
    if m.len() > MAX_IMAGE_BYTES { return Err("That picture is too big to open.".into()); }
    let b = std::fs::read(path).map_err(|e| e.to_string())?;
    let mime = sniff(&b).ok_or("That file isn't a picture Paint can open (PNG, JPEG, WebP, GIF or BMP).")?;
    Ok((b, mime))
}

/// Writes a picture where you chose (a .png or .jpg in a folder that exists), never over a folder.
pub fn write_image(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if !SAVE_EXTS.contains(&ext_of(path).as_str()) { return Err("Save as .png or .jpg.".into()); }
    if path.is_dir() { return Err("That's a folder.".into()); }
    let parent = path.parent().filter(|p| !p.as_os_str().is_empty()).ok_or("Pick a folder to save in.")?;
    if !parent.is_dir() { return Err("That folder isn't there.".into()); }
    let mime = sniff(bytes).ok_or("Nothing to save.")?;
    let want = if ext_of(path) == "png" { "image/png" } else { "image/jpeg" };
    if mime != want { return Err("The picture and its name's ending don't match.".into()); }
    let tmp = path.with_file_name(format!(".{}.hearth-paint-tmp", path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()));
    std::fs::write(&tmp, bytes).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| { let _ = std::fs::remove_file(&tmp); e.to_string() })
}

#[derive(Serialize, Debug)]
pub struct Entry { pub name: String, pub path: String, pub dir: bool }

/// A folder for the page's own file picker: folders and pictures (hidden ones left out), folders first.
pub fn browse(dir: &str) -> Result<(String, Option<String>, Vec<Entry>), String> {
    let d = if dir.is_empty() { pictures() } else { PathBuf::from(dir) };
    let d = std::fs::canonicalize(&d).map_err(|_| "That folder isn't there.".to_string())?;
    let mut v: Vec<Entry> = std::fs::read_dir(&d).map_err(|e| e.to_string())?.flatten().filter_map(|e| {
        let name = e.file_name().to_string_lossy().to_string();
        if name.starts_with('.') { return None; }
        let p = e.path();
        let dir = p.is_dir();
        if !dir && !IMAGE_EXTS.contains(&ext_of(&p).as_str()) { return None; }
        Some(Entry { name, path: p.display().to_string(), dir })
    }).collect();
    v.sort_by(|a, b| (!a.dir, a.name.to_lowercase()).cmp(&(!b.dir, b.name.to_lowercase())));
    Ok((d.display().to_string(), d.parent().map(|p| p.display().to_string()), v))
}

fn have(cmd: &str) -> bool {
    std::env::var_os("PATH").map(|p| std::env::split_paths(&p).any(|d| d.join(cmd).is_file())).unwrap_or(false)
}
fn wayland() -> bool { std::env::var_os("WAYLAND_DISPLAY").is_some() }

/// The picture on the clipboard (PNG, or whatever image type it has), if there is one.
pub fn clipboard_image() -> Result<Option<Vec<u8>>, String> {
    if wayland() && have("wl-paste") {
        let types = Command::new("wl-paste").arg("--list-types").output().map_err(|e| e.to_string())?;
        let types = String::from_utf8_lossy(&types.stdout);
        let pick = ["image/png", "image/jpeg", "image/webp", "image/bmp", "image/gif"].into_iter().find(|t| types.lines().any(|l| l.trim() == *t));
        let Some(t) = pick else { return Ok(None) };
        let out = Command::new("wl-paste").args(["--no-newline", "--type", t]).output().map_err(|e| e.to_string())?;
        return Ok(Some(out.stdout).filter(|b| sniff(b).is_some()));
    }
    if have("xclip") {
        let out = Command::new("xclip").args(["-selection", "clipboard", "-t", "image/png", "-o"]).output().map_err(|e| e.to_string())?;
        return Ok(Some(out.stdout).filter(|b| sniff(b).is_some()));
    }
    Err("No clipboard tool (wl-clipboard) is installed.".into())
}

/// Puts a PNG on the clipboard (wl-copy keeps serving it after Paint closes).
pub fn copy_image(png: &[u8]) -> Result<(), String> {
    if sniff(png) != Some("image/png") { return Err("Nothing to copy.".into()); }
    let mut c = if wayland() && have("wl-copy") {
        Command::new("wl-copy").args(["--type", "image/png"]).stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().map_err(|e| e.to_string())?
    } else if have("xclip") {
        Command::new("xclip").args(["-selection", "clipboard", "-t", "image/png"]).stdin(Stdio::piped()).spawn().map_err(|e| e.to_string())?
    } else { return Err("No clipboard tool (wl-clipboard) is installed.".into()) };
    if let Some(mut i) = c.stdin.take() { i.write_all(png).map_err(|e| e.to_string())?; }
    Ok(())
}

/// A picture of the whole screen (grim on Wayland), as PNG bytes.
pub fn screenshot() -> Result<Vec<u8>, String> {
    if wayland() && have("grim") {
        let out = Command::new("grim").args(["-t", "png", "-"]).output().map_err(|e| e.to_string())?;
        if sniff(&out.stdout) == Some("image/png") { return Ok(out.stdout); }
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Err("Taking a screenshot needs grim (Hearth Desktop Mode has it).".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    const PNG: &[u8] = b"\x89PNG\r\n\x1a\n0000";

    #[test]
    fn kinds() {
        assert_eq!(sniff(PNG), Some("image/png"));
        assert_eq!(sniff(&[0xFF, 0xD8, 0xFF, 0xE0]), Some("image/jpeg"));
        assert_eq!(sniff(b"RIFF0000WEBPVP8 "), Some("image/webp"));
        assert_eq!(sniff(b"hello"), None);
    }

    #[test]
    fn saving() {
        let d = std::env::temp_dir().join(format!("hearth-paint-test-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        write_image(&d.join("a.png"), PNG).unwrap();
        assert_eq!(read_image(&d.join("a.png")).unwrap().1, "image/png");
        assert!(write_image(&d.join("a.jpg"), PNG).is_err(), "PNG bytes in a .jpg");
        assert!(write_image(&d.join("a.txt"), PNG).is_err());
        assert!(write_image(&d.join("missing/a.png"), PNG).is_err());
        assert!(write_image(&d, PNG).is_err());
        std::fs::write(d.join("notes.txt"), b"x").unwrap();
        assert!(read_image(&d.join("notes.txt")).is_err());
        let (_, _, e) = browse(&d.display().to_string()).unwrap();
        assert_eq!(e.iter().map(|x| x.name.as_str()).collect::<Vec<_>>(), vec!["a.png"]);
        let _ = std::fs::remove_dir_all(&d);
    }
}
