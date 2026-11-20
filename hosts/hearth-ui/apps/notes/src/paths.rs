//! Where Hearth Notes keeps things. Your notes stay where they are (the vault: any folder); the app's own files are in
//! your home folder only, outside the vault, so the workspace check stays tidy and nothing of the app's is committed:
//!
//! | What | Where |
//! |---|---|
//! | Settings | `~/.config/nimbus/hearth-notes.json` (settings_file.rs) |
//! | Productivity and habits | `~/.local/share/hearth/notes/trackers.json` |
//! | A note's last good copy (.bak) | `~/.local/share/hearth/notes/bak/<vault>/<note>.bak` |
//! | Copies kept from a conflict | `~/.local/share/hearth/notes/kept/<date time>/` |
//! | Times each note was opened | `~/.local/share/hearth/notes/opened.json` |

use std::path::{Path, PathBuf};

pub fn home() -> PathBuf {
    hearth_ui::paths::home()
}

fn xdg(var: &str, dflt: &str) -> PathBuf {
    std::env::var_os(var).filter(|v| !v.is_empty()).map(PathBuf::from).unwrap_or_else(|| home().join(dflt))
}

pub fn config() -> PathBuf {
    xdg("XDG_CONFIG_HOME", ".config")
}

pub fn data() -> PathBuf {
    xdg("XDG_DATA_HOME", ".local/share").join("hearth/notes")
}

pub fn prefs() -> PathBuf {
    config().join("nimbus/hearth-notes.json")
}

pub fn trackers() -> PathBuf {
    data().join("trackers.json")
}

pub fn kept() -> PathBuf {
    data().join("kept")
}

pub fn baks() -> PathBuf {
    data().join("bak")
}

pub fn opened() -> PathBuf {
    data().join("opened.json")
}

/// The vault when none is chosen: a clone of hearth-workspace in the home folder.
pub fn default_vault() -> PathBuf {
    home().join("hearth-workspace")
}

/// "~/hearth-workspace" for a path in the home folder.
pub fn tilde(p: &Path) -> String {
    let h = home();
    match p.strip_prefix(&h) {
        Ok(rest) if rest.as_os_str().is_empty() => "~".to_string(),
        Ok(rest) => format!("~/{}", rest.to_string_lossy()),
        Err(_) => p.to_string_lossy().to_string(),
    }
}

/// A folder name for one vault's own files (its path with the slashes as dashes).
pub fn vault_key(vault: &Path) -> String {
    let s: String = vault.to_string_lossy().chars().map(|c| if c.is_ascii_alphanumeric() || c == '.' || c == '_' { c } else { '-' }).collect();
    let s = s.trim_matches('-').to_string();
    if s.is_empty() { "vault".into() } else { s }
}

/// Seconds since 1970. CI's pictures use a fixed time (HEARTH_NOTES_NOW), so the pages show the mockups' night.
pub fn now() -> i64 {
    if let Some(t) = std::env::var("HEARTH_NOTES_NOW").ok().and_then(|s| s.trim().parse::<i64>().ok()) {
        return t;
    }
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}
