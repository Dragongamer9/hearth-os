//! The servers Hearth Passwords, Cloud and Home use (Vaultwarden, Nextcloud, Home Assistant: hearth-oss-server), from
//! each app's own Hearth Store copy (the owner, Nov 26: "All this has to be updated thru the store"): the helper and
//! its user services are built into the app itself, and the app puts its own copies in your home folder when it uses
//! them, so they arrive and update with the app, not with Hearth OS:
//!   ~/.local/share/hearth/oss-server/hearth-oss-server                 the helper (written only when it changed)
//!   ~/.config/systemd/user/hearth-<name>.service and .timer           its service and timer (they win over the image's)
//! The same guards as the image's: the service has no [Install] (started only by its timer, enabled only while it's
//! on, or by its app), it doesn't start at all before you installed it (ConditionPathExists), it's never part of
//! starting the PC, the session or Game Mode, and if it fails nothing else waits for it.

use std::path::PathBuf;

const HELPER: &str = include_str!("../../../../linux/desktop/shell/hearth-oss-server");
const UNITS: [(&str, &str); 6] = [
    ("hearth-vaultwarden.service", include_str!("../../../../../os/system_files/usr/lib/systemd/user/hearth-vaultwarden.service")),
    ("hearth-vaultwarden.timer", include_str!("../../../../../os/system_files/usr/lib/systemd/user/hearth-vaultwarden.timer")),
    ("hearth-nextcloud.service", include_str!("../../../../../os/system_files/usr/lib/systemd/user/hearth-nextcloud.service")),
    ("hearth-nextcloud.timer", include_str!("../../../../../os/system_files/usr/lib/systemd/user/hearth-nextcloud.timer")),
    ("hearth-home-assistant.service", include_str!("../../../../../os/system_files/usr/lib/systemd/user/hearth-home-assistant.service")),
    ("hearth-home-assistant.timer", include_str!("../../../../../os/system_files/usr/lib/systemd/user/hearth-home-assistant.timer")),
];

fn helper_path() -> PathBuf {
    crate::data_dir("oss-server").join("hearth-oss-server")
}
fn user_units() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| hearth_ui::paths::home().join(".config")).join("systemd/user")
}

/// A unit as the app installs it: its helper is the app's own copy.
pub fn unit_text(body: &str) -> String {
    body.replace("/usr/share/nimbus/desktop/shell/hearth-oss-server", "%h/.local/share/hearth/oss-server/hearth-oss-server")
}

fn put(p: &std::path::Path, body: &str, mode: u32) -> bool {
    use std::os::unix::fs::PermissionsExt;
    if std::fs::read_to_string(p).map(|t| t == body).unwrap_or(false) {
        return false;
    }
    if let Some(d) = p.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let tmp = p.with_extension("part");
    if std::fs::write(&tmp, body).is_ok() {
        let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(mode));
        let _ = std::fs::rename(&tmp, p);
    }
    true
}

/// The app's own helper in place, and (`units`: when you install or turn a server on or off) its units too, systemd
/// told when one changed. Just opening the app adds nothing to your systemd folder.
pub fn ensure(units: bool) -> PathBuf {
    let h = helper_path();
    put(&h, HELPER, 0o755);
    if !units {
        return h;
    }
    let mut changed = false;
    for (name, body) in UNITS {
        changed |= put(&user_units().join(name), &unit_text(body), 0o644);
    }
    if changed {
        let _ = hearth_ui::run::blocking(&["systemctl".to_string(), "--user".into(), "daemon-reload".into()]);
    }
    h
}

/// `hearth-oss-server ARGS`, run from the app's own copy (put in place first).
pub fn args(a: &[&str]) -> Vec<String> {
    let h = ensure(matches!(a.first(), Some(&"install") | Some(&"on") | Some(&"off")));
    let mut v = vec!["python3".to_string(), h.to_string_lossy().to_string()];
    v.extend(a.iter().map(|s| s.to_string()));
    v
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_units_use_the_apps_own_helper_and_keep_their_guards() {
        for (name, body) in super::UNITS {
            let t = super::unit_text(body);
            if name.ends_with(".service") {
                assert!(t.contains("%h/.local/share/hearth/oss-server/hearth-oss-server run"), "{name}");
                assert!(t.contains("ConditionPathExists="), "{name}");
                assert!(!t.contains("[Install]"), "{name}: started only by its timer or its app");
            }
        }
        assert!(super::HELPER.contains("def run_args("));
    }
}
