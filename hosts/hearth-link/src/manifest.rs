//! A game's files as a list (the manifest), and the checks every path that comes over the network goes through.
//!
//! A game is one or more roots: its install folder, its Proton/Wine prefix, ... Each entry is a path relative to its
//! root: a folder, a file (size, modification time, permissions) or a symbolic link (where it points; Proton
//! prefixes are full of them). Paths in the manifest only ever have plain parts (no "..", nothing absolute), on both
//! sides; the receiving side also never writes through a link (every file's folder must really be inside the root).

use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    /// Which root.
    pub r: u32,
    /// The path in it, with "/" between parts.
    pub p: String,
    /// "f" file, "d" folder, "l" symbolic link.
    pub k: String,
    #[serde(default)]
    pub s: u64,
    /// Modification time (seconds) and nanoseconds.
    #[serde(default)]
    pub m: i64,
    #[serde(default)]
    pub n: u32,
    /// Permission bits.
    #[serde(default)]
    pub x: u32,
    /// A link's target.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub t: Option<String>,
}

impl Entry {
    pub fn chunks(&self) -> u64 { self.s.div_ceil(crate::wire::CHUNK) }
}

/// A relative path from the network: only plain parts, no empty or "." or ".." ones, nothing absolute, no NUL.
pub fn safe_rel(p: &str) -> Option<PathBuf> {
    if p.is_empty() || p.len() > 4096 || p.contains('\0') || p.starts_with('/') || p.contains('\\') && cfg!(windows) { return None; }
    let mut out = PathBuf::new();
    for part in p.split('/') {
        if part.is_empty() || part == "." || part == ".." { return None; }
        out.push(part);
    }
    if out.components().all(|c| matches!(c, Component::Normal(_))) { Some(out) } else { None }
}

/// `root/rel`, or None when `rel` isn't safe or a link in between leads outside `root`.
pub fn resolve_inside(root: &Path, rel: &str) -> Option<PathBuf> {
    let r = safe_rel(rel)?;
    // A root that's one file (walk lists it under its own name): that file, and nothing else.
    if std::fs::symlink_metadata(root).map(|m| m.is_file()).unwrap_or(false) {
        return (root.file_name()? == r.as_os_str()).then(|| root.to_path_buf());
    }
    let full = root.join(&r);
    let real_root = std::fs::canonicalize(root).ok()?;
    let parent = full.parent()?;
    let real_parent = std::fs::canonicalize(parent).ok()?;
    if !real_parent.starts_with(&real_root) { return None; }
    Some(real_parent.join(full.file_name()?))
}

/// Every entry under `root` (not following links), folders before what's in them. `skip` leaves out paths
/// (relative, "/"-separated) and everything under them.
pub fn walk(root: &Path, r: u32, skip: &[&str], out: &mut Vec<Entry>) -> std::io::Result<()> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    let meta = std::fs::symlink_metadata(root)?;
    if meta.is_file() {
        // A root that's one file (a Steam appmanifest): its entry has the file's own name.
        let name = root.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        out.push(Entry { r, p: name, k: "f".into(), s: meta.len(), m: meta.mtime(), n: meta.mtime_nsec() as u32, x: meta.permissions().mode() & 0o7777, t: None });
        return Ok(());
    }
    let mut stack: Vec<(PathBuf, String)> = vec![(root.to_path_buf(), String::new())];
    while let Some((dir, rel)) = stack.pop() {
        let mut items: Vec<_> = std::fs::read_dir(&dir)?.flatten().collect();
        items.sort_by_key(|e| e.file_name());
        for e in items {
            let name = e.file_name();
            let Some(name) = name.to_str() else { continue };   // names that aren't UTF-8 can't go in the manifest
            let p = if rel.is_empty() { name.to_string() } else { format!("{rel}/{name}") };
            if skip.iter().any(|s| p == *s || p.starts_with(&format!("{s}/"))) { continue; }
            let Ok(md) = std::fs::symlink_metadata(e.path()) else { continue };
            let ft = md.file_type();
            let mode = md.permissions().mode() & 0o7777;
            if ft.is_symlink() {
                let t = std::fs::read_link(e.path()).map(|t| t.to_string_lossy().to_string()).unwrap_or_default();
                out.push(Entry { r, p, k: "l".into(), s: 0, m: md.mtime(), n: 0, x: 0, t: Some(t) });
            } else if ft.is_dir() {
                out.push(Entry { r, p: p.clone(), k: "d".into(), s: 0, m: md.mtime(), n: 0, x: mode, t: None });
                stack.push((e.path(), p));
            } else if ft.is_file() {
                out.push(Entry { r, p, k: "f".into(), s: md.len(), m: md.mtime(), n: md.mtime_nsec() as u32, x: mode, t: None });
            }
            // Sockets, pipes and devices aren't part of a game.
        }
    }
    Ok(())
}

/// A link's target from the network, checked for where it may point: inside its own root (relative, not climbing
/// out of the root) or an absolute path (Proton prefixes link "z:" to "/" and to the Proton install; those are
/// only links: nothing is ever written through one).
pub fn link_ok(entry_path: &str, target: &str) -> bool {
    if target.is_empty() || target.len() > 4096 || target.contains('\0') { return false; }
    if target.starts_with('/') { return true; }
    let mut depth: i64 = entry_path.split('/').count() as i64 - 1;
    for part in target.split('/') {
        match part { "" | "." => {}, ".." => { depth -= 1; if depth < 0 { return false; } }, _ => depth += 1 }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn network_paths_are_checked() {
        assert_eq!(safe_rel("a/b.pak"), Some(PathBuf::from("a/b.pak")));
        for bad in ["", "/etc/passwd", "../x", "a/../../x", "a//b", "a/./b", "a/\0"] { assert!(safe_rel(bad).is_none(), "{bad}"); }
    }
    #[test]
    fn links_cant_lead_writes_outside() {
        let d = tempfile::tempdir().unwrap();
        let root = d.path().join("game");
        std::fs::create_dir_all(root.join("data")).unwrap();
        std::fs::create_dir_all(d.path().join("outside")).unwrap();
        std::os::unix::fs::symlink(d.path().join("outside"), root.join("escape")).unwrap();
        assert!(resolve_inside(&root, "data/x.bin").is_some());
        assert!(resolve_inside(&root, "escape/authorized_keys").is_none());
        assert!(resolve_inside(&root, "../outside/x").is_none());
    }
    #[test]
    fn a_root_that_is_one_file() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("film.mkv");
        std::fs::write(&f, b"x").unwrap();
        let mut e = Vec::new();
        walk(&f, 0, &[], &mut e).unwrap();
        assert_eq!(e.len(), 1);
        assert_eq!(resolve_inside(&f, &e[0].p), Some(f.clone()));
        assert!(resolve_inside(&f, "other.mkv").is_none());
    }
    #[test]
    fn link_targets() {
        assert!(link_ok("pfx/dosdevices/z:", "/"));
        assert!(link_ok("a/b/link", "../c"));
        assert!(!link_ok("link", "../../etc"));
        assert!(!link_ok("a/link", "../../x"));
    }
    #[test]
    fn walks_a_game() {
        let d = tempfile::tempdir().unwrap();
        let root = d.path();
        std::fs::create_dir_all(root.join("bin/sub")).unwrap();
        std::fs::write(root.join("bin/game.exe"), vec![1u8; 3 << 20]).unwrap();
        std::fs::write(root.join("bin/sub/a.txt"), "x").unwrap();
        std::fs::create_dir_all(root.join("shadercache")).unwrap();
        std::fs::write(root.join("shadercache/c"), "x").unwrap();
        std::os::unix::fs::symlink("game.exe", root.join("bin/link")).unwrap();
        let mut v = Vec::new();
        walk(root, 0, &["shadercache"], &mut v).unwrap();
        let names: Vec<&str> = v.iter().map(|e| e.p.as_str()).collect();
        assert!(names.contains(&"bin") && names.contains(&"bin/game.exe") && names.contains(&"bin/sub/a.txt"));
        assert!(!names.iter().any(|n| n.starts_with("shadercache")));
        let exe = v.iter().find(|e| e.p == "bin/game.exe").unwrap();
        assert_eq!((exe.k.as_str(), exe.s, exe.chunks()), ("f", 3 << 20, 3));
        assert_eq!(v.iter().find(|e| e.p == "bin/link").unwrap().t.as_deref(), Some("game.exe"));
        // Folders come before what's in them.
        let i = |n: &str| names.iter().position(|x| *x == n).unwrap();
        assert!(i("bin") < i("bin/sub") && i("bin/sub") < i("bin/sub/a.txt"));
    }
}
