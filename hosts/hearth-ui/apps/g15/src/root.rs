//! Where the kernel's files are. Normally /sys, /dev and /proc; with HEARTH_G15_FIXTURE=DIR (CI, the pictures and the
//! tests) DIR/sys, DIR/dev and DIR/proc, plus pretend readings in DIR (stats.json, media.json, notifications.json),
//! so the pages and the screen can be pictured without a real keyboard. `hearth-g15 make-fixture DIR` makes one.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Root {
    pub sys: PathBuf,
    pub dev: PathBuf,
    pub proc_: PathBuf,
    /// The fixture folder, when this is a pretend keyboard.
    pub fixture: Option<PathBuf>,
}

impl Root {
    pub fn real() -> Root {
        Root { sys: "/sys".into(), dev: "/dev".into(), proc_: "/proc".into(), fixture: None }
    }
    pub fn at(dir: &Path) -> Root {
        let proc_ = if dir.join("proc").is_dir() { dir.join("proc") } else { PathBuf::from("/proc") };
        Root { sys: dir.join("sys"), dev: dir.join("dev"), proc_, fixture: Some(dir.to_path_buf()) }
    }
    pub fn from_env() -> Root {
        match std::env::var_os("HEARTH_G15_FIXTURE") {
            Some(d) if !d.is_empty() => Root::at(Path::new(&d)),
            _ => Root::real(),
        }
    }
    /// A file of the fixture (None on a real keyboard).
    pub fn fixture_file(&self, name: &str) -> Option<PathBuf> {
        self.fixture.as_ref().map(|d| d.join(name)).filter(|p| p.exists())
    }
}

pub fn read(p: &Path) -> Option<String> {
    std::fs::read_to_string(p).ok().map(|s| s.trim().to_string())
}

pub fn read_u32(p: &Path) -> Option<u32> {
    read(p)?.parse().ok()
}

pub fn home() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/tmp"))
}

/// ~/.config/nimbus (XDG_CONFIG_HOME respected), where g15.json lives with Hearth's other settings.
pub fn config_dir() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| home().join(".config")).join("nimbus")
}

/// ~/.local/state/nimbus (Mantel's notification history is there).
pub fn state_dir() -> PathBuf {
    std::env::var_os("XDG_STATE_HOME").map(PathBuf::from).unwrap_or_else(|| home().join(".local/state")).join("nimbus")
}

/// The runtime folder for small live files (what the screen shows now, for the app).
pub fn runtime_dir() -> PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap_or_else(std::env::temp_dir).join("hearth")
}
