//! Files: where a received file may go (never outside the receive folder, whatever name the sender gives), names
//! that don't overwrite anything, and the folder browser the window uses to pick files to send (works with a
//! controller, unlike a system file dialog).

use serde_json::{json, Value};
use std::path::{Component, Path, PathBuf};

/// The sender's file name made safe, as a path inside the receive folder: LocalSend sends "folder/file.jpg" for
/// files in a folder, so folders are kept, but "..", absolute paths, drive letters, hidden-file tricks and control
/// characters are not. None when nothing usable is left.
pub fn safe_relative(name: &str) -> Option<PathBuf> {
    let mut out = PathBuf::new();
    for part in name.split(['/', '\\']) {
        let clean: String = part.chars().filter(|c| !c.is_control() && !matches!(c, ':' | '*' | '?' | '"' | '<' | '>' | '|')).collect();
        let clean = clean.trim().trim_start_matches('.').trim().to_string();
        if clean.is_empty() || clean == "." || clean == ".." { continue; }
        // Long names: keep the end (the extension) and as much of the start as fits.
        let clean = if clean.len() > 200 {
            let ext = Path::new(&clean).extension().map(|e| e.to_string_lossy().to_string()).unwrap_or_default();
            let mut head: String = clean.chars().take(180).collect();
            if !ext.is_empty() && ext.len() < 16 { head.push('.'); head.push_str(&ext); }
            head
        } else { clean };
        out.push(clean);
    }
    if out.as_os_str().is_empty() { return None; }
    // Belt and braces: only plain names.
    if out.components().any(|c| !matches!(c, Component::Normal(_))) { return None; }
    if out.components().count() > 32 { return None; }
    Some(out)
}

/// `path`, or "name (1).ext", "name (2).ext"... if it's taken.
pub fn unique(path: &Path) -> PathBuf {
    if !path.exists() { return path.to_path_buf(); }
    let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let ext = path.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    let dir = path.parent().unwrap_or(Path::new("."));
    for n in 1..10000 {
        let p = dir.join(format!("{stem} ({n}){ext}"));
        if !p.exists() { return p; }
    }
    dir.join(format!("{stem} ({}){ext}", crate::random_id(4)))
}

/// Is `path` inside `root` (both as they are on disk)?
pub fn inside(root: &Path, path: &Path) -> bool {
    match (root.canonicalize(), path.canonicalize()) { (Ok(r), Ok(p)) => p.starts_with(r), _ => false }
}

fn home() -> PathBuf { std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/")) }

/// The places the file browser starts from.
pub fn places() -> Value {
    let h = home();
    let mut out = vec![json!({"name": "Home", "path": h.to_string_lossy()})];
    let user_dirs = [("Downloads", crate::downloads_dir()), ("Pictures", h.join("Pictures")), ("Videos", h.join("Videos")),
        ("Music", h.join("Music")), ("Documents", h.join("Documents")), ("Screenshots", h.join("Pictures").join("Screenshots")), ("Received", crate::receive_dir())];
    for (n, p) in user_dirs {
        if p.is_dir() { out.push(json!({"name": n, "path": p.to_string_lossy()})); }
    }
    // Drives: USB sticks and SD cards (udisks mounts them here).
    let user = std::env::var("USER").unwrap_or_default();
    for base in [PathBuf::from("/run/media").join(&user), PathBuf::from("/media").join(&user), PathBuf::from("/var/mnt")] {
        if let Ok(rd) = std::fs::read_dir(&base) {
            for e in rd.flatten() {
                if e.path().is_dir() { out.push(json!({"name": e.file_name().to_string_lossy(), "path": e.path().to_string_lossy(), "drive": true})); }
            }
        }
    }
    Value::Array(out)
}

/// One folder's contents for the browser: folders first, then files, hidden ones left out.
pub fn list_dir(path: &str) -> Result<Value, String> {
    let p = if path.is_empty() { home() } else { PathBuf::from(path) };
    let rd = std::fs::read_dir(&p).map_err(|e| format!("Can't open {}: {e}", p.display()))?;
    let mut dirs = vec![];
    let mut files = vec![];
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if name.starts_with('.') { continue; }
        let Ok(md) = std::fs::metadata(e.path()) else { continue };
        let modified = md.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0);
        let item = json!({"name": name, "path": e.path().to_string_lossy(), "dir": md.is_dir(), "size": if md.is_dir() { 0 } else { md.len() }, "modified": modified});
        if md.is_dir() { dirs.push(item) } else { files.push(item) }
    }
    let key = |v: &Value| v["name"].as_str().unwrap_or("").to_lowercase();
    dirs.sort_by_key(key);
    files.sort_by_key(key);
    dirs.extend(files);
    let parent = p.parent().map(|x| x.to_string_lossy().to_string());
    Ok(json!({"path": p.to_string_lossy(), "parent": parent, "items": dirs}))
}

/// Every file to send for the chosen paths: files as they are, folders with everything in them (as "folder/…"
/// names, which LocalSend recreates). Symbolic links aren't followed into folders.
pub fn collect(paths: &[PathBuf]) -> Result<Vec<(PathBuf, String, u64)>, String> {
    let mut out = vec![];
    for p in paths {
        let md = std::fs::metadata(p).map_err(|e| format!("{}: {e}", p.display()))?;
        let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "file".into());
        if md.is_dir() { walk(p, &name, &mut out, 0)?; } else { out.push((p.clone(), name, md.len())); }
        if out.len() > 20000 { return Err("That's too many files to send at once".into()); }
    }
    Ok(out)
}

fn walk(dir: &Path, rel: &str, out: &mut Vec<(PathBuf, String, u64)>, depth: usize) -> Result<(), String> {
    if depth > 24 { return Ok(()); }
    let rd = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut entries: Vec<_> = rd.flatten().collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        let Ok(ft) = e.file_type() else { continue };
        let name = format!("{rel}/{}", e.file_name().to_string_lossy());
        if ft.is_dir() { walk(&e.path(), &name, out, depth + 1)?; }
        else if ft.is_file() { if let Ok(md) = e.metadata() { out.push((e.path(), name, md.len())); } }
        if out.len() > 20000 { return Err("That's too many files to send at once".into()); }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_from_senders_stay_inside() {
        assert_eq!(safe_relative("photo.jpg"), Some(PathBuf::from("photo.jpg")));
        assert_eq!(safe_relative("Trip/day 1/photo.jpg"), Some(PathBuf::from("Trip/day 1/photo.jpg")));
        assert_eq!(safe_relative("../../etc/passwd"), Some(PathBuf::from("etc/passwd")));
        assert_eq!(safe_relative("/etc/shadow"), Some(PathBuf::from("etc/shadow")));
        assert_eq!(safe_relative("C:\\Windows\\x.dll"), Some(PathBuf::from("C/Windows/x.dll")));
        assert_eq!(safe_relative(".bashrc"), Some(PathBuf::from("bashrc")));
        assert_eq!(safe_relative("a\u{0}b\nc.txt"), Some(PathBuf::from("abc.txt")));
        assert_eq!(safe_relative("../.."), None);
        assert_eq!(safe_relative(""), None);
        let long = format!("{}.mp4", "x".repeat(400));
        let p = safe_relative(&long).unwrap();
        assert!(p.to_string_lossy().len() < 200 && p.to_string_lossy().ends_with(".mp4"));
    }

    #[test]
    fn never_overwrites() {
        let d = std::env::temp_dir().join(format!("nearby-u-{}", crate::random_id(4)));
        std::fs::create_dir_all(&d).unwrap();
        let a = d.join("a.txt");
        assert_eq!(unique(&a), a);
        std::fs::write(&a, "1").unwrap();
        assert_eq!(unique(&a), d.join("a (1).txt"));
        std::fs::write(d.join("a (1).txt"), "1").unwrap();
        assert_eq!(unique(&a), d.join("a (2).txt"));
        assert!(inside(&d, &a));
        assert!(!inside(&d, Path::new("/etc")));
        std::fs::create_dir_all(d.join("sub")).unwrap();
        std::fs::write(d.join("sub/b.bin"), "22").unwrap();
        let c = collect(&[d.join("sub"), a.clone()]).unwrap();
        assert_eq!(c.len(), 2);
        assert_eq!(c[0].1, "sub/b.bin");
        assert_eq!(c[0].2, 2);
        assert!(list_dir(&d.to_string_lossy()).unwrap()["items"].as_array().unwrap().len() >= 3);
        let _ = std::fs::remove_dir_all(d);
    }
}
