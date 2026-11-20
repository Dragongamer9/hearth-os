//! Recordings & screenshots: Capture Card's own files in the two folders (recordings: *.mp4 in Videos › Capture Card;
//! screenshots: "Capture *.png" in Pictures › Screenshots, so Hearth's Capture tool's "Screenshot …" pictures aren't
//! mixed in). Each file's thumbnail and its length and picture size are worked out once and kept in a cache
//! (~/.cache/hearth/capture-card: library.json and thumbs/), looked at again only when the file changes. The list
//! is read when the page opens or after something is saved, renamed or moved to the Trash; nothing polls.

use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Rec,
    Shot,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub path: PathBuf,
    pub kind: Kind,
    pub size: u64,
    pub mtime: u64,
    /// From the cache (None until worked out).
    pub info: Option<Info>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Info {
    pub secs: u64,
    pub w: u32,
    pub h: u32,
    pub fps: u32,
    pub thumb: Option<PathBuf>,
}

impl Item {
    pub fn name(&self) -> String {
        self.path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default()
    }
    /// The screenshots' short name in their tiles ("03-58-21").
    pub fn short(&self) -> String {
        let n = self.name();
        n.rsplit(' ').next().unwrap_or(&n).to_string()
    }
    pub fn ext(&self) -> String {
        self.path.extension().map(|s| s.to_string_lossy().to_uppercase()).unwrap_or_default()
    }
    pub fn folder(&self) -> PathBuf {
        self.path.parent().map(|p| p.to_path_buf()).unwrap_or_default()
    }
}

fn mtime(m: &std::fs::Metadata) -> u64 {
    m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0)
}

pub fn is_rec(name: &str) -> bool {
    let l = name.to_lowercase();
    (l.ends_with(".mp4") || l.ends_with(".mkv")) && !name.starts_with('.')
}

pub fn is_shot(name: &str) -> bool {
    name.starts_with("Capture ") && name.to_lowercase().ends_with(".png")
}

fn cache_file() -> PathBuf {
    crate::cache_dir().join("library.json")
}

fn key(p: &Path, size: u64, mtime: u64) -> String {
    format!("{}|{size}|{mtime}", p.display())
}

pub fn read_cache() -> Map<String, Value> {
    std::fs::read(cache_file()).ok().and_then(|b| serde_json::from_slice::<Value>(&b).ok()).and_then(|v| v.as_object().cloned()).unwrap_or_default()
}

fn info_from(v: &Value) -> Option<Info> {
    Some(Info {
        secs: v["secs"].as_u64().unwrap_or(0),
        w: v["w"].as_u64().unwrap_or(0) as u32,
        h: v["h"].as_u64().unwrap_or(0) as u32,
        fps: v["fps"].as_u64().unwrap_or(0) as u32,
        thumb: v["thumb"].as_str().map(|t| crate::cache_dir().join("thumbs").join(t)).filter(|p| p.is_file()),
    })
}

/// The files in both folders, newest first, with what the cache knows about each.
pub fn scan(rec: &Path, shot: &Path) -> Vec<Item> {
    let cache = read_cache();
    let mut out = Vec::new();
    for (dir, kind) in [(rec, Kind::Rec), (shot, Kind::Shot)] {
        let Ok(rd) = std::fs::read_dir(dir) else { continue };
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            let ok = match kind {
                Kind::Rec => is_rec(&name),
                Kind::Shot => is_shot(&name),
            };
            let Ok(m) = e.metadata() else { continue };
            if !ok || !m.is_file() {
                continue;
            }
            let (size, mt) = (m.len(), mtime(&m));
            let path = e.path();
            let info = cache.get(&key(&path, size, mt)).and_then(info_from);
            out.push(Item { path, kind, size, mtime: mt, info });
        }
    }
    out.sort_by(|a, b| b.mtime.cmp(&a.mtime).then_with(|| b.path.cmp(&a.path)));
    out
}

/// A small, stable name for a thumbnail (FNV-1a of the file's key).
pub fn thumb_name(p: &Path, size: u64, mtime: u64) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in key(p, size, mtime).bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{h:016x}.png")
}

/// Works out what the cache doesn't know yet (a worker: screenshots by reading them, recordings through GStreamer),
/// and keeps it. Returns whether anything was added.
pub fn fill(items: &[Item]) -> bool {
    let todo: Vec<&Item> = items.iter().filter(|i| i.info.as_ref().map(|x| x.thumb.is_none()).unwrap_or(true)).collect();
    if todo.is_empty() {
        return false;
    }
    let mut cache = read_cache();
    // Forget files that are gone, and the old entries of files that changed (by their key).
    let live: std::collections::HashSet<String> = items.iter().map(|i| key(&i.path, i.size, i.mtime)).collect();
    let listed: std::collections::HashSet<String> = items.iter().map(|i| i.path.display().to_string()).collect();
    cache.retain(|k, _| {
        let p = k.split('|').next().unwrap_or("");
        live.contains(k) || (!listed.contains(p) && Path::new(p).exists())
    });
    let dir = crate::cache_dir().join("thumbs");
    let _ = std::fs::create_dir_all(&dir);
    let mut added = false;
    for it in todo {
        let tn = thumb_name(&it.path, it.size, it.mtime);
        let (secs, w, h, fps, ok) = match it.kind {
            Kind::Shot => match image::open(&it.path) {
                Ok(img) => {
                    let (w, h) = (img.width(), img.height());
                    let t = img.thumbnail(480, 270);
                    let ok = t.save_with_format(dir.join(&tn), image::ImageFormat::Png).is_ok();
                    (0, w, h, 0, ok)
                }
                Err(_) => (0, 0, 0, 0, false),
            },
            Kind::Rec => match crate::video::probe(&it.path) {
                Some(p) => {
                    let ok = match image::RgbaImage::from_raw(p.tw, p.th, p.rgba) {
                        Some(img) => img.save_with_format(dir.join(&tn), image::ImageFormat::Png).is_ok(),
                        None => false,
                    };
                    (p.secs, p.w, p.h, p.fps, ok)
                }
                None => (0, 0, 0, 0, false),
            },
        };
        let mut v = json!({"secs": secs, "w": w, "h": h, "fps": fps});
        if ok {
            v["thumb"] = json!(tn);
        }
        cache.insert(key(&it.path, it.size, it.mtime), v);
        added = true;
    }
    if added {
        let _ = crate::write_atomic(&cache_file(), serde_json::to_string(&Value::Object(cache)).unwrap_or_default().as_bytes());
    }
    added
}

/// Renames a file in its own folder (the name without its extension). Never over another file.
pub fn rename(p: &Path, new_stem: &str) -> Result<PathBuf, String> {
    let stem = new_stem.trim();
    if stem.is_empty() || stem.contains('/') || stem.starts_with('.') {
        return Err("That name can't be used.".into());
    }
    let ext = p.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    // Screenshots keep "Capture " in front so they stay in the list.
    let stem = if is_shot(&p.file_name().unwrap_or_default().to_string_lossy()) && !stem.starts_with("Capture ") { format!("Capture {stem}") } else { stem.to_string() };
    let to = p.with_file_name(format!("{stem}{ext}"));
    if to == p {
        return Ok(to);
    }
    if to.exists() {
        return Err(format!("There's already a file called {}.", to.file_name().unwrap_or_default().to_string_lossy()));
    }
    std::fs::rename(p, &to).map_err(|e| format!("Couldn't rename it: {e}"))?;
    Ok(to)
}

/// Moves a file to the Trash (gio, else KDE's kioclient): it can be taken back from there.
pub fn trash(p: &Path) -> Result<(), String> {
    let s = p.to_string_lossy().to_string();
    if crate::output(&["gio", "trash", &s], 10).is_some() || crate::output(&["kioclient", "move", &s, "trash:/"], 10).is_some() {
        if !p.exists() {
            return Ok(());
        }
    }
    Err("Couldn't move it to the Trash.".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_capture_cards_own_files_newest_first() {
        let d = tempfile::tempdir().unwrap();
        let rec = d.path().join("Videos/Capture Card");
        let shot = d.path().join("Pictures/Screenshots");
        std::fs::create_dir_all(&rec).unwrap();
        std::fs::create_dir_all(&shot).unwrap();
        std::fs::write(rec.join("Capture 2026-11-20 03-41-07.mp4"), b"x").unwrap();
        std::fs::write(shot.join("Capture 2026-11-20 03-58-21.png"), b"x").unwrap();
        std::fs::write(shot.join("Screenshot 2026-11-20 01-00-00.png"), b"x").unwrap();
        std::fs::write(rec.join("notes.txt"), b"x").unwrap();
        let v = scan(&rec, &shot);
        assert_eq!(v.len(), 2, "Hearth Capture's screenshots and other files aren't listed");
        assert!(v.iter().any(|i| i.kind == Kind::Rec && i.name() == "Capture 2026-11-20 03-41-07"));
        let s = v.iter().find(|i| i.kind == Kind::Shot).unwrap();
        assert_eq!(s.short(), "03-58-21");
        assert_eq!(s.ext(), "PNG");
    }

    #[test]
    fn renaming_keeps_the_folder_and_never_overwrites() {
        let d = tempfile::tempdir().unwrap();
        let a = d.path().join("Capture 2026-11-20 03-58-21.png");
        let b = d.path().join("Capture Lap record.png");
        std::fs::write(&a, b"a").unwrap();
        std::fs::write(&b, b"b").unwrap();
        assert!(rename(&a, "Lap record").is_err());
        let to = rename(&a, "Best lap").unwrap();
        assert_eq!(to, d.path().join("Capture Best lap.png"));
        assert!(rename(&to, "../x").is_err());
        assert_eq!(thumb_name(&a, 1, 2), thumb_name(&a, 1, 2));
        assert_ne!(thumb_name(&a, 1, 2), thumb_name(&a, 1, 3));
    }
}
