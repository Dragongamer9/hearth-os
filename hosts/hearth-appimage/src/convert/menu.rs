//! The app menu: an entry for an AppImage in ~/.local/share/applications (TryExec on the AppImage, so the entry
//! hides itself if the file is deleted), and taking it out again.

use super::appdir::Desktop;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub fn apps_dir() -> PathBuf { dirs::data_dir().unwrap_or_else(|| PathBuf::from("/tmp")).join("applications") }

pub fn entry_path(id: &str) -> PathBuf { apps_dir().join(format!("hearth-appimage-{id}.desktop")) }

/// Exec= quoting (Desktop Entry spec): in double quotes, with " ` $ \ escaped.
pub fn exec_quote(p: &str) -> String {
    let mut s = String::from("\"");
    for c in p.chars() {
        if matches!(c, '"' | '`' | '$' | '\\') { s.push('\\'); }
        s.push(c);
    }
    s.push('"');
    s
}

pub fn add(appimage: &Path, desktop: &Desktop, id: &str, icon: &Path) -> io::Result<PathBuf> {
    let mut d = desktop.clone();
    let field = d.get("Exec").and_then(|e| ["%U", "%u", "%F", "%f"].into_iter().find(|f| e.contains(f))).map(|f| format!(" {f}")).unwrap_or_default();
    let path = appimage.display().to_string();
    d.set("Exec", &format!("{}{field}", exec_quote(&path)));
    d.set("TryExec", &path);
    d.set("Icon", &icon.display().to_string());
    d.set("X-AppImage-Path", &path);
    d.set("X-Hearth-AppImage", "true");
    let p = entry_path(id);
    fs::create_dir_all(apps_dir())?;
    let tmp = p.with_extension("desktop.part");
    fs::write(&tmp, d.text())?;
    fs::rename(&tmp, &p)?;
    let _ = std::process::Command::new("update-desktop-database").arg(apps_dir()).output();
    Ok(p)
}

/// Takes an entry out, only if it is one of ours.
pub fn remove(entry: &Path) -> io::Result<()> {
    let ours = entry.parent() == Some(apps_dir().as_path())
        && entry.file_name().map(|n| n.to_string_lossy().starts_with("hearth-appimage-")).unwrap_or(false);
    if !ours { return Err(io::Error::other("that menu entry wasn't made by AppImage Maker")); }
    match fs::remove_file(entry) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(e),
    }
    let _ = std::process::Command::new("update-desktop-database").arg(apps_dir()).output();
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn quoting() {
        assert_eq!(super::exec_quote("/home/me/My Apps/a$b.AppImage"), "\"/home/me/My Apps/a\\$b.AppImage\"");
    }
}
