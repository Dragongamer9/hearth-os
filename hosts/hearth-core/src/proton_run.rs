//! How Hearth starts a Windows program with Proton: **hearth-proton-run**, the offline launcher in Hearth OS's image
//! (`/usr/bin/hearth-proton-run`, hosts/linux/desktop/shell/hearth_proton_run.py; docs/WINDOWS-APPS.md, "Proton:
//! offline and without Steam"). Owner's red flag: the built-in Proton works without Steam, 100% offline.
//!
//! It takes umu-run's command line and environment (`<exe> [args]`, WINEPREFIX, PROTONPATH, GAMEID, STORE, `winetricks
//! <verbs>`), so every place that ran umu-run runs it instead: Hearth Games' GOG and Epic games, Steam-engine direct
//! starts, games you added yourself, Recomps' Windows builds, Hearth's own Battle.net setup, and Hearth Mods. Unlike
//! umu-run it never downloads anything (umu-launcher itself, the Steam Linux Runtime, UMU-Proton), never needs Steam or
//! a Steam folder, and keeps Proton's own game fixes from going online. A Proton you picked that's on this device is
//! used; a pick that isn't, or none, means the GE-Proton built into Hearth OS.
//!
//! Only where hearth-proton-run isn't installed (another distro, an image from before it) does Hearth fall back to
//! umu-run as before: the system's, else Hearth's own downloaded copy (tools.rs).

use std::path::PathBuf;

/// Where the image puts it.
pub const LAUNCHER: &str = "/usr/bin/hearth-proton-run";

/// LAUNCHER, or HEARTH_PROTON_RUN (the tests).
fn launcher_path() -> PathBuf {
    std::env::var_os("HEARTH_PROTON_RUN").filter(|v| !v.is_empty()).map(PathBuf::from).unwrap_or_else(|| PathBuf::from(LAUNCHER))
}

/// The offline launcher, when this image has it.
pub fn offline() -> Option<String> {
    use std::os::unix::fs::PermissionsExt;
    let p = launcher_path();
    std::fs::metadata(&p).ok().filter(|m| m.is_file() && m.permissions().mode() & 0o111 != 0).map(|_| p.to_string_lossy().to_string())
}

/// The program that starts Windows programs with Proton: hearth-proton-run (offline, no Steam), or where it isn't
/// installed, umu-run (the system's, else Hearth's downloaded copy, which needs the internet the first time).
pub fn command() -> Result<String, String> {
    match offline() {
        Some(p) => Ok(p),
        None => crate::tools::ensure(&crate::tools::UMU),
    }
}

/// The program, picked from what's here (pure, for the tests): the offline launcher, else a system umu-run, else None.
pub fn pick(offline: Option<String>, has_umu: bool) -> Option<String> {
    offline.or_else(|| has_umu.then(|| "umu-run".to_string()))
}

/// What to start a Windows program with right now, without downloading anything: hearth-proton-run, else a system
/// umu-run, else None (games you added yourself then try Wine).
pub fn ready() -> Option<String> { pick(offline(), crate::system::has("umu-run")) }

/// The program's name for a menu entry's Exec line (desktop files): hearth-proton-run is on the PATH in Hearth OS.
pub fn exec_name() -> &'static str { if offline().is_some() { "hearth-proton-run" } else { "umu-run" } }

/// Whether a command is umu-run, bare or by a full path (`~/.local/share/nimbus/tools/umu/umu-run`).
pub fn is_umu(cmd: &str) -> bool { std::path::Path::new(cmd).file_name().map(|n| n == "umu-run").unwrap_or(false) }

/// Exec words from a menu entry Hearth wrote before hearth-proton-run (Hearth Mods' Wabbajack lists, Recomps' Windows
/// builds): umu-run swapped for `runner` (pure, for the tests). Under umu, the built-in GE-Proton fails on devices
/// (umu's Steam Runtime container can't see /usr/share/hearth/proton: "proton: not found", exit 127).
pub fn swap_umu(argv: Vec<String>, runner: &str) -> (Vec<String>, Vec<String>) {
    let mut old = vec![];
    let v = argv.into_iter().map(|t| if is_umu(&t) { old.push(t); runner.to_string() } else { t }).collect();
    (v, old)
}

/// For a .desktop file Hearth starts: its Exec words with umu-run replaced by hearth-proton-run when this image has it.
/// A file Hearth wrote itself (a Wabbajack list's or a Recomp's entry) is also rewritten in place, atomically, so menus
/// start it the same way. Anything else is only changed for this start.
pub fn modernize_desktop(file: &std::path::Path, argv: Vec<String>) -> Vec<String> {
    let Some(runner) = offline() else { return argv };
    let (v, old) = swap_umu(argv, &runner);
    if old.is_empty() { return v; }
    crate::bridge::log(&format!("launch: {} named umu-run; starting it with {runner} (offline, without Steam)", file.display()));
    let name = file.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let ours = (name.starts_with("hearth-recomp-") || file.parent().and_then(|p| p.file_name()).map(|n| n == "modlists").unwrap_or(false))
        && name.ends_with(".desktop");
    if ours {
        if let Ok(text) = std::fs::read_to_string(file) {
            let mut new = text.clone();
            for o in &old { new = new.replace(o.as_str(), &runner); }
            if new != text {
                let tmp = file.with_extension("desktop.tmp");
                if std::fs::write(&tmp, &new).is_ok() && std::fs::rename(&tmp, file).is_ok() {
                    crate::bridge::log(&format!("launch: {} now names {runner}", file.display()));
                } else { let _ = std::fs::remove_file(&tmp); }
            }
        }
    }
    v
}

/// The message for a start whose saved Proton isn't on this device, from `--print-proton`'s line (name, folder, note):
/// Some when hearth-proton-run substitutes the built-in one (pure, for the tests).
pub fn substitute_message(print_line: &str) -> Option<String> {
    let mut f = print_line.trim_end().splitn(3, '\t');
    let (name, _path, note) = (f.next()?, f.next()?, f.next()?);
    note.contains("isn't on this device").then(|| format!(
        "{note}. This start uses {name}; the game's own Proton choice is kept and used again once it's back on this device."))
}

/// Owner's red flag (a game's saved Proton is a setting): when the Proton saved in `env` (PROTONPATH) isn't on this
/// device, hearth-proton-run starts the game with the built-in one for this run. This tells the person, and never
/// changes the saved choice. Only for the offline launcher (umu-run elsewhere does its own thing).
pub fn warn_if_substituted(runner: &str, env: &[(String, String)]) {
    if offline().as_deref() != Some(runner) { return; }
    let Some((_, want)) = env.iter().rev().find(|(k, _)| k == "PROTONPATH") else { return };
    if want.trim().is_empty() { return; }
    let Ok(out) = std::process::Command::new(runner).arg("--print-proton").env("PROTONPATH", want)
        .stdin(std::process::Stdio::null()).stderr(std::process::Stdio::null()).output() else { return };
    if let Some(msg) = substitute_message(&String::from_utf8_lossy(&out.stdout)) {
        crate::bridge::log(&format!("proton: {msg}"));
        crate::stores::notify(serde_json::json!({"type": "toast", "text": msg}));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_entries_naming_umu_start_with_the_offline_launcher() {
        let argv: Vec<String> = ["env", "WINEPREFIX=/p/pfx", "GAMEID=umu-489830", "/h/.local/share/nimbus/tools/umu/umu-run", "/m/ModOrganizer.exe"]
            .iter().map(|s| s.to_string()).collect();
        let (v, old) = swap_umu(argv, "/usr/bin/hearth-proton-run");
        assert_eq!(v[3], "/usr/bin/hearth-proton-run");
        assert_eq!(old, vec!["/h/.local/share/nimbus/tools/umu/umu-run".to_string()]);
        assert_eq!(v[4], "/m/ModOrganizer.exe");
        let (v, old) = swap_umu(vec!["umu-run".into(), "/g/Game.exe".into()], "/usr/bin/hearth-proton-run");
        assert_eq!(v, vec!["/usr/bin/hearth-proton-run", "/g/Game.exe"]);
        assert_eq!(old.len(), 1);
        let (v, old) = swap_umu(vec!["/opt/game/run".into(), "my-umu-runner".into()], "/x");
        assert_eq!(v, vec!["/opt/game/run", "my-umu-runner"]);
        assert!(old.is_empty() && !is_umu("umu-run.sh") && is_umu("/usr/bin/umu-run"));
    }

    #[test]
    fn a_missing_saved_proton_is_told_and_kept() {
        let m = substitute_message("GE-Proton11-3\t/usr/share/hearth/proton/GE-Proton11-3\tGE-Proton9-1 isn't on this device: the one built into Hearth OS instead (nothing is downloaded)\n").unwrap();
        assert!(m.contains("GE-Proton9-1") && m.contains("uses GE-Proton11-3") && m.contains("kept"), "{m}");
        assert!(substitute_message("GE-Proton11-3\t/usr/share/hearth/proton/GE-Proton11-3\n").is_none());
        assert!(substitute_message("GE-Proton11-3\t/p\tno Proton picked: the one built into Hearth OS\n").is_none());
        assert!(substitute_message("none\n").is_none());
    }

    #[test]
    fn the_offline_launcher_comes_first() {
        assert_eq!(pick(Some("/usr/bin/hearth-proton-run".into()), true).as_deref(), Some("/usr/bin/hearth-proton-run"));
        assert_eq!(pick(Some("/usr/bin/hearth-proton-run".into()), false).as_deref(), Some("/usr/bin/hearth-proton-run"));
        // Only without it (not Hearth OS): a system umu-run, else nothing (Wine, or "install umu-launcher").
        assert_eq!(pick(None, true).as_deref(), Some("umu-run"));
        assert_eq!(pick(None, false), None);
    }

    #[test]
    fn the_image_ships_it_and_it_never_downloads() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let wrapper = std::fs::read_to_string(root.join("os/system_files/usr/bin/hearth-proton-run")).unwrap();
        assert!(wrapper.contains("desktop/shell/hearth_proton_run.py"), "{wrapper}");
        let py = std::fs::read_to_string(root.join("hosts/linux/desktop/shell/hearth_proton_run.py")).unwrap();
        assert!(py.contains("def block_network") && !py.contains("urlopen("), "the launcher never downloads");
        let build = std::fs::read_to_string(root.join("os-bases/peppermint/build.sh")).unwrap();
        assert!(build.contains("/usr/bin/hearth-proton-run") && build.contains("hearth-proton-run --print-proton"));
        assert_eq!(LAUNCHER, "/usr/bin/hearth-proton-run");
    }
}
