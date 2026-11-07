//! Which Proton a Windows game starts with when you haven't picked one (the game's options say "Default").
//!
//! Hearth OS 1.7 builds the newest GE-Proton into the image (Steam's system folder for compatibility tools,
//! `/usr/share/steam/compatibilitytools.d/GE-Proton*`), so Windows games and apps start without a download, offline.
//! When it's there, "Default" means that one. When it isn't (an older image, another distro), nothing is set and
//! umu-launcher picks UMU-Proton, downloaded when needed, as before.
//!
//! The Proton detector (docs/WINDOWS-APPS.md) decides in `auto_for`, in the same order as Windows apps
//! (hosts/linux/desktop/shell/hearth_proton.py): the Proton the game last worked with on this device
//! (~/.local/share/nimbus/proton-worked.json, key "store:id"), what Hearth's bundled table says for the game
//! (/usr/share/hearth/proton-rules.json, `"match": {"game": "gog:1"}`), the built-in GE-Proton, then the newest Valve
//! Proton Steam has installed. Your own pick in a game's options always wins: this is only asked when there's none.

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

/// Hearth's bundled Proton table (shared with Windows apps): HEARTH_PROTON_RULES or /usr/share/hearth/proton-rules.json.
pub const RULES: &str = "/usr/share/hearth/proton-rules.json";

fn home() -> Option<PathBuf> { std::env::var_os("HOME").filter(|v| !v.is_empty()).map(PathBuf::from) }

fn read_json(p: &Path) -> Option<serde_json::Value> {
    serde_json::from_str(&std::fs::read_to_string(p).ok()?).ok()
}

/// Steam's folders on this device (native and Flatpak), for Valve's Protons and Steam's own compatibility tools.
fn steam_roots(home: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    for r in [".local/share/Steam", ".steam/steam", ".steam/root", ".var/app/com.valvesoftware.Steam/.local/share/Steam"] {
        if let Ok(real) = std::fs::canonicalize(home.join(r)) {
            if real.is_dir() && !out.contains(&real) { out.push(real); }
        }
    }
    out
}

/// The newest Valve Proton in Steam's folders ("Proton 10.0" before "Proton - Experimental").
pub fn newest_valve_in(roots: &[PathBuf]) -> Option<PathBuf> {
    let mut found: Vec<((u8, Vec<u64>), PathBuf)> = Vec::new();
    for r in roots {
        let Ok(rd) = std::fs::read_dir(r.join("steamapps/common")) else { continue };
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            let low = name.to_lowercase();
            if !name.starts_with("Proton") || low.contains("hotfix") || low.replace(' ', "").contains("easyanticheat") { continue; }
            if !e.path().join("proton").is_file() { continue; }
            let k = version_key(&name);
            found.push(((if k.is_empty() { 0 } else { 1 }, k), e.path()));
        }
    }
    found.sort();
    found.pop().map(|(_, p)| p)
}

/// A Proton by its name, wherever it is: the built-in folder, Hearth's downloads (~/.local/share/hearth/proton),
/// Steam's own compatibility tools, or Valve's in steamapps/common.
pub fn find_named(name: &str, builtin_dir: &Path, home: Option<&Path>) -> Option<PathBuf> {
    if name.is_empty() || name.contains('/') || name.starts_with('.') { return None; }
    let mut dirs = vec![builtin_dir.to_path_buf()];
    if let Some(h) = home {
        dirs.push(h.join(".local/share/hearth/proton"));
        for r in steam_roots(h) { dirs.push(r.join("compatibilitytools.d")); dirs.push(r.join("steamapps/common")); }
    }
    dirs.into_iter().map(|d| d.join(name)).find(|p| p.join("proton").is_file())
}

/// A Proton by its name on this device (Hearth Mods: the GE-Proton a prefix was made with, from its `version` note).
pub fn named(name: &str) -> Option<PathBuf> { find_named(name, &system_dir(), home().as_deref()) }

/// The detector's pick for a game, from its pieces (so the tests can give their own folders).
pub fn detect(key: &str, builtin_dir: &Path, rules: &Path, home: Option<&Path>) -> Option<PathBuf> {
    // 1. What it last worked with here (and not since closed right away with).
    if let Some(h) = home {
        if let Some(e) = read_json(&h.join(".local/share/nimbus/proton-worked.json")).and_then(|v| v.get(key).cloned()) {
            if let Some(w) = e.get("worked").and_then(|w| w.as_str()) {
                if let Some(p) = find_named(w, builtin_dir, Some(h)) { return Some(p); }
            }
        }
    }
    // 2. Hearth's table.
    if let Some(table) = read_json(rules) {
        let rule = table.get("rules").and_then(|r| r.as_array()).and_then(|rs| {
            rs.iter().find(|r| r.get("match").and_then(|m| m.get("game")).and_then(|g| g.as_str()) == Some(key)).cloned()
        });
        if let Some(want) = rule.as_ref().and_then(|r| r.get("proton")).and_then(|p| p.as_str()) {
            let hit = match want {
                "builtin" => newest_ge_in(builtin_dir),
                "valve" => home.and_then(|h| newest_valve_in(&steam_roots(h))),
                other => find_named(other, builtin_dir, home),
            };
            if hit.is_some() { return hit; }
        }
    }
    // 3. Built into Hearth OS; 4. Valve's newest.
    newest_ge_in(builtin_dir).or_else(|| home.and_then(|h| newest_valve_in(&steam_roots(h))))
}

/// The Proton for a game with no pick of its own (store and id as in stores::prefix_of): the Proton detector's choice.
/// None: let umu-launcher choose (UMU-Proton, downloaded when needed).
pub fn auto_for(store: &str, id: &str) -> Option<PathBuf> {
    let rules = std::env::var_os("HEARTH_PROTON_RULES").filter(|v| !v.is_empty()).map(PathBuf::from).unwrap_or_else(|| PathBuf::from(RULES));
    detect(&format!("{store}:{id}"), &system_dir(), &rules, home().as_deref())
}

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
    fn detector_order_worked_then_table_then_builtin() {
        let t = tempfile::tempdir().unwrap();
        let sys = t.path().join("sys");
        let home = t.path().join("home");
        tool(&sys, "GE-Proton11-3", true);
        tool(&home.join(".local/share/hearth/proton"), "GE-Proton9-27", true);
        tool(&home.join(".local/share/hearth/proton"), "GE-Proton10-34", true);
        let rules = t.path().join("rules.json");
        std::fs::write(&rules, r#"{"rules":[{"match":{"game":"gog:7"},"proton":"GE-Proton10-34"}]}"#).unwrap();
        // Nothing known: the built-in one.
        assert_eq!(detect("gog:1", &sys, &rules, Some(&home)).unwrap().file_name().unwrap(), "GE-Proton11-3");
        // The table's pick for that game.
        assert_eq!(detect("gog:7", &sys, &rules, Some(&home)).unwrap().file_name().unwrap(), "GE-Proton10-34");
        // What it last worked with beats the table.
        std::fs::create_dir_all(home.join(".local/share/nimbus")).unwrap();
        std::fs::write(home.join(".local/share/nimbus/proton-worked.json"), r#"{"gog:7":{"worked":"GE-Proton9-27"}}"#).unwrap();
        assert_eq!(detect("gog:7", &sys, &rules, Some(&home)).unwrap().file_name().unwrap(), "GE-Proton9-27");
        // A remembered one that's gone, or a damaged table, falls through to the built-in one.
        std::fs::write(home.join(".local/share/nimbus/proton-worked.json"), r#"{"gog:7":{"worked":"../evil"}}"#).unwrap();
        std::fs::write(&rules, "not json").unwrap();
        assert_eq!(detect("gog:7", &sys, &rules, Some(&home)).unwrap().file_name().unwrap(), "GE-Proton11-3");
    }

    #[test]
    fn valve_numbered_before_experimental() {
        let t = tempfile::tempdir().unwrap();
        let root = t.path().join("Steam");
        let common = root.join("steamapps/common");
        tool(&common, "Proton - Experimental", true);
        tool(&common, "Proton 9.0 (Beta)", true);
        tool(&common, "Proton 10.0", true);
        tool(&common, "Proton Hotfix", true);
        assert_eq!(newest_valve_in(&[root]).unwrap().file_name().unwrap(), "Proton 10.0");
    }

    #[test]
    fn your_own_pick_is_never_replaced() {
        let mut env = vec![("PROTONPATH".to_string(), "GE-Proton".to_string())];
        fill_in(&mut env, "gog", "1");
        assert_eq!(env, vec![("PROTONPATH".to_string(), "GE-Proton".to_string())]);
    }
}
