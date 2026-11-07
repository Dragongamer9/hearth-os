//! Which Proton a Windows game starts with when you haven't picked one (the game's options say "Default").
//!
//! Hearth OS 1.7 builds the newest GE-Proton into the image (Steam's system folder for compatibility tools,
//! `/usr/share/steam/compatibilitytools.d/GE-Proton*`), so Windows games and apps start without a download, offline.
//! When it's there, "Default" means that one. When it isn't (an older image, another distro), nothing is set and
//! umu-launcher picks UMU-Proton, downloaded when needed, as before.
//!
//! The Proton detector (planned for 1.7, docs/ROADMAP.md) plugs in at `auto_for`: it will pick per game (a known fix,
//! the version the game last worked with), falling back to the built-in one. Your own pick in a game's options always
//! wins: this is only asked when there's none.

use std::path::{Path, PathBuf};

/// Steam's system folder for compatibility tools, where the image puts the built-in GE-Proton.
pub const SYSTEM_DIR: &str = "/usr/share/steam/compatibilitytools.d";

/// The folder to look in: SYSTEM_DIR, or HEARTH_BUILTIN_PROTON_DIR (the tests).
fn system_dir() -> PathBuf {
    std::env::var_os("HEARTH_BUILTIN_PROTON_DIR").filter(|v| !v.is_empty()).map(PathBuf::from).unwrap_or_else(|| PathBuf::from(SYSTEM_DIR))
}

/// "GE-Proton10-25" -> [10, 25]: the numbers in a name, to sort versions the way people read them.
fn version_key(name: &str) -> Vec<u64> {
    name.split(|c: char| !c.is_ascii_digit()).filter(|s| !s.is_empty()).filter_map(|s| s.parse().ok()).collect()
}

/// The newest GE-Proton in `dir` (a folder named GE-Proton* with a `proton` script in it).
pub fn newest_ge_in(dir: &Path) -> Option<PathBuf> {
    let rd = std::fs::read_dir(dir).ok()?;
    let mut found: Vec<(Vec<u64>, PathBuf)> = rd.flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            let p = e.path();
            (name.starts_with("GE-Proton") && p.join("proton").is_file()).then(|| (version_key(&name), p))
        })
        .collect();
    found.sort();
    found.pop().map(|(_, p)| p)
}

/// The GE-Proton built into Hearth OS, if this image has one.
pub fn builtin_ge() -> Option<PathBuf> { newest_ge_in(&system_dir()) }

/// The Proton for a game with no pick of its own (store and id as in stores::prefix_of): the built-in GE-Proton for
/// now; the Proton detector decides here later. None: let umu-launcher choose (UMU-Proton, downloaded when needed).
pub fn auto_for(_store: &str, _id: &str) -> Option<PathBuf> { builtin_ge() }

/// What "Default" means on this device, for the game options' list.
pub fn default_label() -> String {
    match builtin_ge().and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string())) {
        Some(n) => format!("{n}, built into Hearth OS (works offline)"),
        None => "UMU-Proton, the newest, downloaded when needed".into(),
    }
}

/// PROTONPATH for a start, unless the game's options (or its own environment) already set one.
pub fn fill_in(env: &mut Vec<(String, String)>, store: &str, id: &str) {
    if env.iter().any(|(k, _)| k == "PROTONPATH") { return; }
    if let Some(p) = auto_for(store, id) {
        crate::bridge::log(&format!("proton: {store}:{id} has no Proton of its own: using {}", p.display()));
        env.push(("PROTONPATH".into(), p.to_string_lossy().into()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tool(dir: &Path, name: &str, with_script: bool) {
        let d = dir.join(name);
        std::fs::create_dir_all(&d).unwrap();
        if with_script { std::fs::write(d.join("proton"), "#!/bin/sh\n").unwrap(); }
    }

    #[test]
    fn newest_ge_by_version_not_by_letters() {
        let t = tempfile::tempdir().unwrap();
        tool(t.path(), "GE-Proton9-27", true);
        tool(t.path(), "GE-Proton10-4", true);
        tool(t.path(), "GE-Proton10-25", true);
        tool(t.path(), "GE-Proton11-1", false);   // no proton script: not a usable tool
        tool(t.path(), "Proton-Experimental", true);
        assert_eq!(newest_ge_in(t.path()).unwrap().file_name().unwrap(), "GE-Proton10-25");
    }

    #[test]
    fn nothing_there_means_umu_chooses() {
        let t = tempfile::tempdir().unwrap();
        assert!(newest_ge_in(t.path()).is_none());
        assert!(newest_ge_in(&t.path().join("missing")).is_none());
    }

    #[test]
    fn your_own_pick_is_never_replaced() {
        let mut env = vec![("PROTONPATH".to_string(), "GE-Proton".to_string())];
        fill_in(&mut env, "gog", "1");
        assert_eq!(env, vec![("PROTONPATH".to_string(), "GE-Proton".to_string())]);
    }
}
