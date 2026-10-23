//! A folder browser for the pages (choosing a backup folder, a mod file, a save folder) that works with a
//! controller: where to start (home, Downloads, USB drives), and what's in a folder.

use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// Places to start from.
pub fn places() -> Value {
    let home = crate::util::home();
    let mut v = vec![json!({"label": "Home", "path": home})];
    for (n, label) in [("Downloads", "Downloads"), ("Games", "Games"), ("Backups", "Backups")] {
        let p = home.join(n);
        if p.is_dir() { v.push(json!({"label": label, "path": p})); }
    }
    let user = std::env::var("USER").unwrap_or_default();
    for root in [format!("/run/media/{user}"), format!("/media/{user}"), "/var/mnt".to_string(), "/mnt".to_string()] {
        let Ok(rd) = std::fs::read_dir(&root) else { continue };
        for e in rd.flatten() { if e.path().is_dir() { v.push(json!({"label": e.file_name().to_string_lossy(), "path": e.path(), "drive": true})); } }
    }
    Value::Array(v)
}

/// What's in a folder: folders first, then files (only archives when `archives`), A to Z; hidden ones left out
/// unless asked for.
pub fn list(dir: &Path, files: bool, archives: bool, hidden: bool) -> Result<Value, String> {
    let rd = std::fs::read_dir(dir).map_err(|e| format!("Couldn't open {} ({e})", dir.display()))?;
    let mut dirs = Vec::new();
    let mut fs = Vec::new();
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if !hidden && name.starts_with('.') { continue; }
        let p = e.path();
        let Ok(md) = std::fs::metadata(&p) else { continue };
        if md.is_dir() { dirs.push(json!({"name": name, "path": p, "dir": true})); }
        else if files && md.is_file() && (!archives || crate::mods::is_archive(&p)) {
            fs.push(json!({"name": name, "path": p, "dir": false, "size": md.len(), "archive": crate::mods::is_archive(&p)}));
        }
    }
    let key = |v: &Value| v["name"].as_str().unwrap_or("").to_lowercase();
    dirs.sort_by_key(key);
    fs.sort_by_key(key);
    dirs.extend(fs);
    let parent: Option<PathBuf> = dir.parent().map(Path::to_path_buf);
    Ok(json!({"path": dir, "parent": parent, "entries": dirs}))
}
