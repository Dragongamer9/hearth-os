//! hearth-layout: the part of desktop layouts and Describe it that both Mantels (the QML one and Mantel in Rust) and
//! Hearth's own apps share (docs/LAYOUTS.md; approved mockups docs/mockups/checklists/desktop-layouts.md):
//!
//! * [`blocks`]: every block a bar or panel can hold, by the names Mantel already uses (widget-tools.js);
//! * [`layout`]: a layout file (the bars, the panels, the dock, the app menu, the window buttons) and its check;
//! * [`apply`]: putting a layout in place through Mantel's own settings (desktop.json, bar.json), with Undo;
//! * [`feature`]: a new feature as a recipe of Hearth's own parts (what it shows, what a click does), and its check;
//! * [`idea`]: what's sent to Hearth's makers when something needs new work, exactly as it's shown first;
//! * [`app`]: Describe it in Hearth's own apps (their knobs, places, sources and actions; what you kept, with Undo);
//! * [`describe`]: Describe it: your words (and a sketch) to a layout or a feature by your AI, checked here.
//!
//! Nothing here runs code that came in a file: a layout or a feature only names blocks, sources and actions Hearth
//! already has, and anything else is refused with a plain reason (or becomes an idea to send).

pub mod app;
pub mod apply;
pub mod blocks;
pub mod describe;
pub mod feature;
pub mod idea;
pub mod layout;
pub mod made;
pub mod offline;
pub mod screen;

/// Drag and drop in every Hearth app (drag-drop board 3): what `layout.studio` may hold, checked with the apps' own
/// rules (the same file hearth-ui reads it with).
#[path = "../../hearth-ui/src/studio_model.rs"]
#[allow(dead_code)]
pub mod studio_model;

#[path = "../../hearth-core/src/settings_file.rs"]
#[allow(dead_code)]
pub mod settings_file;

/// The home folder and Mantel's settings files.
pub mod paths {
    use std::path::PathBuf;
    pub fn home() -> PathBuf {
        std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/tmp"))
    }
    fn config() -> PathBuf {
        std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| home().join(".config"))
    }
    /// Mantel's look, bar and app menu settings (Settings.qml).
    pub fn desktop_json() -> PathBuf {
        std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|| home().join(".local/share")).join("nimbus/desktop.json")
    }
    /// The main bar's order and the extra panels (BarLayout.qml).
    pub fn bar_json() -> PathBuf {
        config().join("nimbus/bar.json")
    }
    /// Your own layouts and features, and what Undo puts back.
    pub fn layouts_dir() -> PathBuf {
        config().join("nimbus/layouts")
    }
    pub fn features_dir() -> PathBuf {
        config().join("nimbus/features")
    }
    /// The ready-made layouts that come with Hearth OS.
    pub fn builtin_dir() -> PathBuf {
        if let Some(s) = std::env::var_os("NIMBUS_SHARE") {
            return PathBuf::from(s).join("desktop/layouts");
        }
        for p in ["/usr/share/nimbus", "/usr/local/share/nimbus"] {
            let d = PathBuf::from(p).join("desktop/layouts");
            if d.is_dir() {
                return d;
            }
        }
        PathBuf::from("/usr/share/nimbus/desktop/layouts")
    }
}

#[cfg(test)]
mod admin {
    /// hearth-core's, for settings_file.rs's own test.
    pub fn write_start_mode(text: &str, mode: &str) -> String {
        let line = format!("NIMBUS_START_MODE={mode}");
        if text.lines().any(|l| l.trim_start().starts_with("NIMBUS_START_MODE=")) {
            let mut out: Vec<String> = text.lines().map(|l| if l.trim_start().starts_with("NIMBUS_START_MODE=") { line.clone() } else { l.to_string() }).collect();
            out.push(String::new());
            out.join("\n")
        } else {
            format!("{}{}{line}\n", text, if !text.is_empty() && !text.ends_with('\n') { "\n" } else { "" })
        }
    }
}

/// A short id: lowercase letters, digits and dashes (a file name, a block's or layout's id).
pub fn is_slug(s: &str) -> bool {
    !s.is_empty() && s.len() <= 48 && s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') && !s.starts_with('-')
}
