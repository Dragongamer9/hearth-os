//! The lists of recordings, photos and videos the apps show (newest first), and deleting one safely (only
//! inside the app's own folders).

use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub const VIDEO: &[&str] = &["mp4", "mkv", "webm", "mov"];
pub const AUDIO: &[&str] = &["opus", "ogg", "oga", "mp3", "m4a", "flac", "wav"];
pub const PHOTO: &[&str] = &["jpg", "jpeg", "png", "webp"];

/// Files in `dir` with one of `exts`, newest first: [{name, path, size, time, kind}].
pub fn list(dir: &Path, exts: &[&str], limit: usize) -> Vec<Value> {
    let mut v: Vec<(u64, Value)> = std::fs::read_dir(dir).into_iter().flatten().flatten().filter_map(|e| {
        let p = e.path();
        let ext = p.extension()?.to_str()?.to_ascii_lowercase();
        if !exts.contains(&ext.as_str()) { return None; }
        let name = p.file_name()?.to_string_lossy().to_string();
        if name.starts_with('.') { return None; }
        let m = e.metadata().ok()?;
        if !m.is_file() || m.len() == 0 { return None; }
        let t = m.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_millis() as u64;
        let kind = if VIDEO.contains(&ext.as_str()) { "video" } else if AUDIO.contains(&ext.as_str()) { "audio" } else { "photo" };
        Some((t, json!({"name": name, "path": p.to_string_lossy(), "size": m.len(), "time": t, "kind": kind})))
    }).collect();
    v.sort_by(|a, b| b.0.cmp(&a.0));
    v.into_iter().take(limit).map(|(_, x)| x).collect()
}

/// `file`, if it's really inside one of `dirs` (no "..", no links out).
pub fn inside(file: &str, dirs: &[PathBuf]) -> Option<PathBuf> {
    let f = std::fs::canonicalize(file).ok()?;
    dirs.iter().filter_map(|d| std::fs::canonicalize(d).ok()).any(|d| f.starts_with(&d) && f != d).then_some(f)
}

/// Deletes a file from one of the app's folders.
pub fn delete(file: &str, dirs: &[PathBuf]) -> Result<(), String> {
    let f = inside(file, dirs).ok_or("That file isn't in this app's folders")?;
    std::fs::remove_file(&f).map_err(|e| format!("Couldn't delete it: {e}"))
}

/// Renames a file in place (a new name without a folder; the extension is kept).
pub fn rename(file: &str, name: &str, dirs: &[PathBuf]) -> Result<PathBuf, String> {
    let f = inside(file, dirs).ok_or("That file isn't in this app's folders")?;
    let clean: String = name.trim().chars().filter(|c| !matches!(c, '/' | '\\' | '\0')).collect();
    let clean = clean.trim_matches('.').trim().to_string();
    if clean.is_empty() { return Err("Type a name".into()); }
    let ext = f.extension().map(|e| e.to_string_lossy().to_string()).unwrap_or_default();
    let new = f.with_file_name(if ext.is_empty() || clean.to_ascii_lowercase().ends_with(&format!(".{}", ext.to_ascii_lowercase())) { clean } else { format!("{clean}.{ext}") });
    if new.exists() { return Err("There's already a file with that name".into()); }
    std::fs::rename(&f, &new).map_err(|e| format!("Couldn't rename it: {e}"))?;
    Ok(new)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lists_and_guards() {
        let d = std::env::temp_dir().join(format!("hm-files-{}", std::process::id()));
        let other = std::env::temp_dir().join(format!("hm-other-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        std::fs::create_dir_all(&other).unwrap();
        std::fs::write(d.join("a.mp4"), b"1").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(d.join("b.opus"), b"2").unwrap();
        std::fs::write(d.join("c.txt"), b"3").unwrap();
        std::fs::write(d.join("empty.mp4"), b"").unwrap();
        std::fs::write(other.join("x.mp4"), b"4").unwrap();
        let l = list(&d, &["mp4", "opus"], 10);
        assert_eq!(l.len(), 2);
        assert_eq!(l[0]["name"], "b.opus");
        assert_eq!(l[0]["kind"], "audio");
        let dirs = vec![d.clone()];
        assert!(delete(&other.join("x.mp4").to_string_lossy(), &dirs).is_err());
        assert!(delete(&format!("{}/../{}/x.mp4", d.display(), other.file_name().unwrap().to_string_lossy()), &dirs).is_err());
        let n = rename(&d.join("a.mp4").to_string_lossy(), "Boss fight", &dirs).unwrap();
        assert_eq!(n.file_name().unwrap(), "Boss fight.mp4");
        assert!(rename(&n.to_string_lossy(), "../evil", &dirs).unwrap().starts_with(std::fs::canonicalize(&d).unwrap()));
        delete(&d.join("b.opus").to_string_lossy(), &dirs).unwrap();
        assert!(!d.join("b.opus").exists());
        let _ = std::fs::remove_dir_all(&d);
        let _ = std::fs::remove_dir_all(&other);
    }
}
