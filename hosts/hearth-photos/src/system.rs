//! The system's parts: the Trash (with undo), the clipboard, the wallpaper and the file manager.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;

/// Moves a picture to the Trash (Linux: the freedesktop trash; Windows: the Recycle Bin).
pub fn trash(path: &Path) -> Result<(), String> {
    if !path.exists() { return Err("That picture isn't there any more".into()); }
    trash::delete(path).map_err(|e| format!("Couldn't move it to the Trash ({e})"))
}

/// Takes the most recently trashed copy of `path` back out of the Trash.
pub fn untrash(path: &Path) -> Result<(), String> {
    #[cfg(any(windows, all(unix, not(target_os = "macos"), not(target_os = "ios"), not(target_os = "android"))))]
    {
        let items = trash::os_limited::list().map_err(|e| e.to_string())?;
        let newest = items.into_iter().filter(|i| i.original_path() == path).max_by_key(|i| i.time_deleted);
        let item = newest.ok_or("It isn't in the Trash any more")?;
        if path.exists() { return Err("There's a file with that name there now".into()); }
        trash::os_limited::restore_all([item]).map_err(|e| format!("Couldn't take it out of the Trash ({e})"))
    }
    #[cfg(not(any(windows, all(unix, not(target_os = "macos"), not(target_os = "ios"), not(target_os = "android")))))]
    { let _ = path; Err("Undo isn't available here".into()) }
}

static CLIP: Mutex<Option<arboard::Clipboard>> = Mutex::new(None);

/// Puts the picture on the clipboard (as an image; pasting into a chat or a document works).
pub fn copy_image(path: &Path) -> Result<(), String> {
    let img = crate::imaging::decode_upright(path)?;
    // Wayland (Hearth's Desktop Mode): wl-copy keeps serving the clipboard after Photos closes.
    #[cfg(target_os = "linux")]
    if std::env::var_os("WAYLAND_DISPLAY").is_some() && which("wl-copy") {
        let png = crate::imaging::encode_png(&img, true)?;
        if let Ok(mut child) = Command::new("wl-copy").args(["--type", "image/png"]).stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::null()).spawn() {
            use std::io::Write;
            if let Some(mut stdin) = child.stdin.take() { let _ = stdin.write_all(&png); }
            if child.wait().map(|s| s.success()).unwrap_or(false) { return Ok(()); }
        }
    }
    let rgba = img.to_rgba8();
    let data = arboard::ImageData { width: rgba.width() as usize, height: rgba.height() as usize, bytes: std::borrow::Cow::Owned(rgba.into_raw()) };
    let mut guard = CLIP.lock().unwrap();
    if guard.is_none() { *guard = Some(arboard::Clipboard::new().map_err(|e| format!("No clipboard ({e})"))?); }
    guard.as_mut().unwrap().set_image(data).map_err(|e| format!("Couldn't copy it ({e})"))
}

pub fn which(cmd: &str) -> bool {
    std::env::var_os("PATH").map(|p| std::env::split_paths(&p).any(|d| d.join(cmd).is_file())).unwrap_or(false)
}

/// A copy of the picture as JPEG or PNG in Photos' own folder, for a wallpaper (wallpapers can't be WebP, HEIC…
/// everywhere, and the original may be on a USB drive that gets unplugged).
fn wallpaper_copy(path: &Path) -> Result<PathBuf, String> {
    let dir = dirs::data_dir().ok_or("No data folder")?.join("hearth-photos").join("wallpaper");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let img = crate::imaging::decode_upright(path)?;
    let (bytes, ext) = if img.color().has_alpha() { (crate::imaging::encode_png(&img, false)?, "png") } else { (crate::imaging::encode_jpeg(&img, 94)?, "jpg") };
    // A new name each time, so the desktop notices the change; older copies are cleared out.
    if let Ok(rd) = std::fs::read_dir(&dir) { for e in rd.flatten() { let _ = std::fs::remove_file(e.path()); } }
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let out = dir.join(format!("wallpaper-{stamp}.{ext}"));
    std::fs::write(&out, bytes).map_err(|e| e.to_string())?;
    Ok(out)
}

/// Sets the picture as the wallpaper. Linux: Hearth's own helper (hearth-wallpaper, Desktop Mode's wallpaper),
/// and Plasma's too when Plasma is running.
pub fn set_wallpaper(path: &Path) -> Result<String, String> {
    let copy = wallpaper_copy(path)?;
    #[cfg(target_os = "linux")]
    {
        let mut done = Vec::new();
        if which("hearth-wallpaper") {
            let ok = Command::new("hearth-wallpaper").arg(&copy).stdout(Stdio::null()).status().map(|s| s.success()).unwrap_or(false);
            if ok { done.push("Hearth"); }
        }
        let plasma = std::env::var("XDG_CURRENT_DESKTOP").map(|d| d.to_uppercase().contains("KDE")).unwrap_or(false);
        if plasma && which("plasma-apply-wallpaperimage") {
            if Command::new("plasma-apply-wallpaperimage").arg(&copy).stdout(Stdio::null()).status().map(|s| s.success()).unwrap_or(false) { done.push("Plasma"); }
        }
        if done.is_empty() {
            return Err("There's no wallpaper helper here (hearth-wallpaper)".into());
        }
        return Ok("Set as your wallpaper".into());
    }
    #[cfg(not(target_os = "linux"))]
    { let _ = copy; Err("Setting the wallpaper from here comes later on this system".into()) }
}

/// Opens the picture's folder in the file manager with the picture picked.
pub fn show_in_folder(path: &Path) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        // hearth-filemanager opens Hearth Files at the file with it picked (another file manager without Hearth).
        if which("hearth-filemanager") && Command::new("hearth-filemanager").arg(path).spawn().is_ok() { return Ok(()); }
        // Other desktops: the freedesktop FileManager1 call picks the file; else just open the folder.
        let uri = format!("file://{}", percent_encoding::utf8_percent_encode(&path.to_string_lossy(), percent_encoding::NON_ALPHANUMERIC).to_string().replace("%2F", "/"));
        if which("dbus-send") {
            let ok = Command::new("dbus-send").args(["--session", "--print-reply", "--dest=org.freedesktop.FileManager1", "/org/freedesktop/FileManager1",
                "org.freedesktop.FileManager1.ShowItems", &format!("array:string:{uri}"), "string:"]).stdout(Stdio::null()).stderr(Stdio::null())
                .status().map(|s| s.success()).unwrap_or(false);
            if ok { return Ok(()); }
        }
        let dir = path.parent().unwrap_or(Path::new("/"));
        Command::new("xdg-open").arg(dir).spawn().map(|_| ()).map_err(|e| e.to_string())
    }
    #[cfg(windows)]
    { Command::new("explorer.exe").arg(format!("/select,{}", path.display())).spawn().map(|_| ()).map_err(|e| e.to_string()) }
    #[cfg(not(any(target_os = "linux", windows)))]
    { let _ = path; Err("Not on this system".into()) }
}

#[cfg(test)]
mod tests {
    #[test]
    fn trash_and_back() {
        // Only where there's a home folder with a Trash to use (CI's runner has one).
        let Some(home) = dirs::home_dir() else { return };
        let d = home.join(format!("hp-trash-test-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let p = d.join("gone.png");
        std::fs::write(&p, b"x").unwrap();
        if super::trash(&p).is_err() { std::fs::remove_dir_all(&d).ok(); return; }
        assert!(!p.exists());
        super::untrash(&p).unwrap();
        assert!(p.exists());
        std::fs::remove_dir_all(&d).unwrap();
    }
}
