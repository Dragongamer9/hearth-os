//! hearth-ui: Hearth's look and the pieces every pure-Rust Hearth app uses (iced 0.14, tiny-skia).
//!
//! * [`theme`]: the colours of the person's chosen look (palettes.json + desktop.json), followed while open;
//! * [`ui`]: text, buttons, cards, chips, lines, the window's own title bar and a dimmed dialog;
//! * [`run`]: helpers that run a program (Hearth's Python helpers) off the window's thread;
//! * [`app`]: the window every app opens (its id, size, the probe CI measures);
//! * [`settings_file`]: hearth-core's settings_file.rs, the one safe way to write a setting (red flag 9).

pub mod app;
pub mod run;
pub mod theme;
pub mod time;
pub mod ui;

#[path = "../../hearth-core/src/settings_file.rs"]
#[allow(dead_code)]
pub mod settings_file;

/// The home folder (settings_file.rs asks for it).
pub mod paths {
    pub fn home() -> std::path::PathBuf {
        std::env::var_os("HOME").map(std::path::PathBuf::from).unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
    }
}

#[cfg(test)]
mod admin {
    /// hearth-core's, for settings_file.rs's own test.
    pub fn write_start_mode(text: &str, mode: &str) -> String {
        let line = format!("NIMBUS_START_MODE={mode}");
        if text.lines().any(|l| l.trim_start().starts_with("NIMBUS_START_MODE=")) {
            let mut out: Vec<String> = text
                .lines()
                .map(|l| if l.trim_start().starts_with("NIMBUS_START_MODE=") { line.clone() } else { l.to_string() })
                .collect();
            out.push(String::new());
            out.join("\n")
        } else {
            format!("{}{}{line}\n", text, if !text.is_empty() && !text.ends_with('\n') { "\n" } else { "" })
        }
    }
}

pub use theme::Pal;
