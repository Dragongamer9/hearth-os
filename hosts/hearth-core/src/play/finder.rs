//! Library › Find games in a folder (docs/HYDRA-FEATURES.md): looks through a folder (your Games or Downloads
//! folder, a drive, a game you just unpacked) for programs that look like games (Windows .exe files, AppImages,
//! Linux game launchers), leaving out installers, uninstallers, crash reporters and redistributables, and games
//! already in your library. You pick which to add; they're added like Add a game (Windows ones run with Proton).

use serde_json::{json, Value};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

const SKIP_NAMES: &[&str] = &["unins", "uninstall", "setup", "install", "redist", "vcredist", "vc_redist", "dxsetup", "directx", "dotnet", "crash", "report",
    "updater", "update", "helper", "launcherpatcher", "cefprocess", "ue4prereq", "prereq", "physx", "oalinst", "easyanticheat", "battleye", "cleanup"];
const SKIP_DIRS: &[&str] = &["_commonredist", "redist", "redistributables", "__installer", "directx", "support", "prerequisites", "dotnet", "vcredist", "engine", ".git", "node_modules", "compatdata", "shadercache", "proton"];

/// Whether a file looks like a game's program. `exec`: it has the executable bit (for files without an extension).
pub fn looks_like_game(name: &str, exec: bool) -> Option<&'static str> {
    let low = name.to_ascii_lowercase();
    let stem = low.rsplit_once('.').map(|(s, _)| s).unwrap_or(&low);
    if SKIP_NAMES.iter().any(|s| stem.contains(s)) { return None; }
    if low.ends_with(".exe") { return Some("windows"); }
    if low.ends_with(".appimage") { return Some("appimage"); }
    if (low.ends_with(".x86_64") || low.ends_with(".x86")) && exec { return Some("linux"); }
    if low.ends_with(".sh") && exec && (stem.contains("start") || stem.contains("run") || stem.contains("launch") || stem.contains("play")) { return Some("linux"); }
    None
}

/// Candidates under `dirs` (at most 4 folders deep, 200 results), minus `known` (paths already in the library).
pub fn scan(dirs: &[PathBuf], known: &HashSet<String>) -> Vec<Value> {
    let mut out = vec![];
    for d in dirs { walk(d, 0, known, &mut out); }
    out.sort_by(|a, b| a["title"].as_str().unwrap_or("").to_lowercase().cmp(&b["title"].as_str().unwrap_or("").to_lowercase()));
    out
}

fn walk(dir: &Path, depth: usize, known: &HashSet<String>, out: &mut Vec<Value>) {
    if depth > 4 || out.len() >= 200 { return; }
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<_> = rd.flatten().collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        if out.len() >= 200 { return; }
        let p = e.path();
        let name = e.file_name().to_string_lossy().to_string();
        let Ok(meta) = std::fs::symlink_metadata(&p) else { continue };
        if meta.file_type().is_symlink() { continue; }
        if meta.is_dir() {
            if name.starts_with('.') || SKIP_DIRS.contains(&name.to_ascii_lowercase().as_str()) { continue; }
            walk(&p, depth + 1, known, out);
        } else if meta.is_file() {
            use std::os::unix::fs::PermissionsExt;
            let exec = meta.permissions().mode() & 0o111 != 0;
            let Some(kind) = looks_like_game(&name, exec) else { continue };
            let ps = p.to_string_lossy().to_string();
            if known.contains(&ps) { continue; }
            // A tiny .exe is a stub or a tool, not a game.
            if kind == "windows" && meta.len() < 100 * 1024 { continue; }
            let folder = p.parent().and_then(|f| f.file_name()).map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
            out.push(json!({"path": ps, "title": crate::manual::title_for(&ps), "kind": kind, "size": meta.len(), "folder": folder}));
        }
    }
}

/// The folders looked in when none is picked: ~/Games, ~/Downloads and the download manager's folder.
pub fn default_dirs(home: &Path, downloads: &Path) -> Vec<PathBuf> {
    let mut v = vec![home.join("Games"), home.join("Downloads")];
    if !downloads.starts_with(home.join("Downloads")) { v.push(downloads.to_path_buf()); }
    v.into_iter().filter(|d| d.is_dir()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn finds_games_not_installers() {
        assert_eq!(looks_like_game("Hades.exe", false), Some("windows"));
        assert_eq!(looks_like_game("unins000.exe", false), None);
        assert_eq!(looks_like_game("setup_game.exe", false), None);
        assert_eq!(looks_like_game("UnityCrashHandler64.exe", false), None);
        assert_eq!(looks_like_game("OpenRA-Red-Alert-x86_64.AppImage", false), Some("appimage"));
        assert_eq!(looks_like_game("game.x86_64", true), Some("linux"));
        assert_eq!(looks_like_game("game.x86_64", false), None);
        assert_eq!(looks_like_game("start.sh", true), Some("linux"));
        assert_eq!(looks_like_game("build.sh", true), None);

        let d = tempfile::tempdir().unwrap();
        let g = d.path().join("Games/Hollow Pines");
        std::fs::create_dir_all(g.join("_CommonRedist")).unwrap();
        std::fs::write(g.join("HollowPines.exe"), vec![0u8; 200 * 1024]).unwrap();
        std::fs::write(g.join("unins000.exe"), vec![0u8; 200 * 1024]).unwrap();
        std::fs::write(g.join("tiny.exe"), b"MZ").unwrap();
        std::fs::write(g.join("_CommonRedist/vc_redist.x64.exe"), vec![0u8; 200 * 1024]).unwrap();
        let s = d.path().join("Games/stk/run_game.sh");
        std::fs::create_dir_all(s.parent().unwrap()).unwrap();
        std::fs::write(&s, b"#!/bin/sh\n").unwrap();
        std::fs::set_permissions(&s, std::fs::Permissions::from_mode(0o755)).unwrap();
        let known: HashSet<String> = HashSet::new();
        let found = scan(&[d.path().join("Games")], &known);
        let names: Vec<&str> = found.iter().map(|f| f["path"].as_str().unwrap().rsplit('/').next().unwrap()).collect();
        assert_eq!(names, vec!["HollowPines.exe", "run_game.sh"], "{found:?}");
        assert_eq!(found[0]["kind"], "windows");
        assert_eq!(found[0]["folder"], "Hollow Pines");
        let known: HashSet<String> = [g.join("HollowPines.exe").to_string_lossy().to_string()].into();
        assert_eq!(scan(&[d.path().join("Games")], &known).len(), 1, "games already in the library are left out");
        std::fs::create_dir_all(d.path().join("Downloads")).unwrap();
        assert_eq!(default_dirs(d.path(), &d.path().join("Downloads/Hearth")).len(), 2);
    }
}
