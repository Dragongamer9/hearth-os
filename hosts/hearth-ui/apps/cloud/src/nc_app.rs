//! FROM for every file (board 13, Recent files): Nextcloud itself doesn't keep which device changed a file, so Hearth
//! Cloud puts its own small Nextcloud app into your Nextcloud (nextcloud/hearthdevice, AGPL-3): on every file written,
//! from a phone, the desktop client, a browser or Hearth, it notes the device's name (its app password's or session's)
//! as Hearth's WebDAV property {https://hearthos.fun/ns}device, which the FROM column reads (dav.rs). It ships inside
//! this app (so it arrives and updates with Hearth Cloud from the Store), is copied into the Nextcloud that
//! hearth-oss-server runs (~/.local/share/hearth/nextcloud/custom_apps) and turned on with Nextcloud's own occ, when
//! you press "Note the device on every change". Turned off again with occ app:disable.

use std::path::PathBuf;

const FILES: [(&str, &str); 3] = [
    ("appinfo/info.xml", include_str!("../nextcloud/hearthdevice/appinfo/info.xml")),
    ("lib/AppInfo/Application.php", include_str!("../nextcloud/hearthdevice/lib/AppInfo/Application.php")),
    ("lib/Listener/Written.php", include_str!("../nextcloud/hearthdevice/lib/Listener/Written.php")),
];
pub const CONTAINER: &str = "hearth-nextcloud";

/// Where hearth-oss-server keeps Nextcloud's files (its /var/www/html).
pub fn nextcloud_dir() -> PathBuf {
    let h = hearth_ui::paths::home();
    std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|| h.join(".local/share")).join("hearth/nextcloud")
}

/// Copies the app in and turns it on (Nextcloud's occ, as its own web server's user).
pub fn install() -> Result<(), String> {
    let base = nextcloud_dir();
    if !base.join("config").exists() {
        return Err("your Nextcloud isn't on this PC (Hearth Cloud › Install it here first)".into());
    }
    let dir = base.join("custom_apps/hearthdevice");
    for (rel, body) in FILES {
        let f = dir.join(rel);
        if let Some(p) = f.parent() {
            std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
        }
        let tmp = f.with_extension("part");
        std::fs::write(&tmp, body).and_then(|_| std::fs::rename(&tmp, &f)).map_err(|e| e.to_string())?;
    }
    let o = hearth_ui::run::blocking(&["podman".to_string(), "exec".into(), "-u".into(), "www-data".into(), CONTAINER.into(), "php".into(), "occ".into(), "app:enable".into(), "hearthdevice".into()]);
    if o.ok { Ok(()) } else { Err(format!("Nextcloud didn't turn it on: {}", o.why())) }
}

pub fn uninstall() -> Result<(), String> {
    let o = hearth_ui::run::blocking(&["podman".to_string(), "exec".into(), "-u".into(), "www-data".into(), CONTAINER.into(), "php".into(), "occ".into(), "app:disable".into(), "hearthdevice".into()]);
    if o.ok { Ok(()) } else { Err(o.why()) }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_app_is_whole() {
        assert!(super::FILES[0].1.contains("<id>hearthdevice</id>"));
        assert!(super::FILES[2].1.contains("{https://hearthos.fun/ns}device"));
        assert_eq!(crate::dav::NS, "https://hearthos.fun/ns");
    }
}
