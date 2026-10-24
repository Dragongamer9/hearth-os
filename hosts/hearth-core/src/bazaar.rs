//! Bazaar (io.github.kolunmi.Bazaar), Hearth OS's app store for Flathub apps, opened from Game Mode's menu (Apps).
//! Hearth's own store is archived (docs/ARCHIVED.md); hearth-flatpak-setup installs Bazaar for everyone at every boot
//! until it's there. In Game Mode it opens full screen inside gamescope like Moonlight (its window is a program
//! Hearth opened: gamemode.rs puts it in front), with the controller mouse on (padmouse.rs) so a controller can use it.

use std::path::{Path, PathBuf};

pub const ID: &str = "io.github.kolunmi.Bazaar";
pub const NOT_YET: &str = "Bazaar isn't installed yet. Hearth OS installs it for everyone when the PC is online \
    (a few minutes after the first start or an update; it keeps trying every 15 minutes).";

/// How Bazaar is installed: the system image's package (the `bazaar` command), or a Flatpak (the system's, or yours).
#[derive(Debug, Clone, PartialEq)]
pub enum Install { Package(PathBuf), Flatpak }

/// Where Bazaar is, looking in `root` ("/" normally) and `home`.
pub fn find_in(root: &Path, home: &Path) -> Option<Install> {
    let desktop = format!("{ID}.desktop");
    let packaged = ["usr/share/applications", "usr/local/share/applications"].iter().any(|d| root.join(d).join(&desktop).is_file());
    if packaged {
        if let Some(b) = ["usr/bin/bazaar", "usr/local/bin/bazaar"].iter().map(|b| root.join(b)).find(|b| b.is_file()) {
            return Some(Install::Package(b));
        }
    }
    let flatpak = root.join("var/lib/flatpak/app").join(ID).is_dir() || home.join(".local/share/flatpak/app").join(ID).is_dir();
    if flatpak { Some(Install::Flatpak) } else { None }
}

pub fn find() -> Option<Install> { find_in(Path::new("/"), &crate::paths::home()) }

/// Whether Game Mode shows Apps in its menu: on a Linux system with Flatpak (Bazaar may still be on its way).
pub fn available() -> bool { cfg!(target_os = "linux") && (crate::system::has("flatpak") || find().is_some()) }

/// The program and its arguments.
pub fn command(install: &Install) -> (String, Vec<String>) {
    match install {
        Install::Package(b) => (b.to_string_lossy().to_string(), vec![]),
        Install::Flatpak => ("flatpak".to_string(), vec!["run".to_string(), ID.to_string()]),
    }
}

/// Opens Bazaar. In Game Mode the controller mouse comes on too (right stick moves the pointer, A clicks; View + Menu
/// turns it off). Returns {ok, mouse}.
pub fn open(game_mode: bool) -> Result<serde_json::Value, String> {
    let install = find().ok_or(NOT_YET)?;
    let (prog, args) = command(&install);
    crate::bridge::log(&format!("bazaar: opening {prog} {}", args.join(" ")));
    std::process::Command::new(&prog).args(&args).current_dir(crate::paths::home())
        .stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null())
        .spawn().map(crate::procs::detach).map_err(|e| format!("Couldn't open Bazaar: {e}"))?;
    let mut mouse = false;
    if game_mode {
        match crate::padmouse::start() {
            Ok(()) => { mouse = true; crate::stores::notify(serde_json::json!({"type": "event", "name": "padMouse", "data": {"on": true}})); }
            Err(e) => crate::bridge::log(&format!("bazaar: no controller mouse ({e})")),
        }
    }
    Ok(serde_json::json!({"ok": true, "mouse": mouse}))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("hearth-bazaar-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn found_where_it_is() {
        let root = tmp("root");
        let home = tmp("home");
        assert_eq!(find_in(&root, &home), None);
        std::fs::create_dir_all(home.join(".local/share/flatpak/app").join(ID)).unwrap();
        assert_eq!(find_in(&root, &home), Some(Install::Flatpak));
        std::fs::create_dir_all(root.join("usr/share/applications")).unwrap();
        std::fs::write(root.join("usr/share/applications").join(format!("{ID}.desktop")), "[Desktop Entry]\n").unwrap();
        std::fs::create_dir_all(root.join("usr/bin")).unwrap();
        std::fs::write(root.join("usr/bin/bazaar"), "").unwrap();
        assert_eq!(find_in(&root, &home), Some(Install::Package(root.join("usr/bin/bazaar"))));
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn commands() {
        assert_eq!(command(&Install::Flatpak), ("flatpak".to_string(), vec!["run".to_string(), ID.to_string()]));
        assert_eq!(command(&Install::Package(PathBuf::from("/usr/bin/bazaar"))).0, "/usr/bin/bazaar");
    }
}
