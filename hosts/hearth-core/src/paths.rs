//! Where Hearth keeps things. The same folders the Electron host uses, so both read the same data.

use std::path::PathBuf;

pub fn home() -> PathBuf { std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/")) }

/// ~/.local/share/nimbus (or $XDG_DATA_HOME/nimbus)
pub fn data_dir() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|| home().join(".local/share")).join("nimbus")
}

/// ~/.config/nimbus (or $XDG_CONFIG_HOME/nimbus)
pub fn config_dir() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| home().join(".config")).join("nimbus")
}

/// Writes a file all at once (a temporary file, then a rename), so a crash never leaves half a file.
pub fn write_atomic(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() { std::fs::create_dir_all(dir)?; }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(tmp, path)
}

/// Hearth's desktop files (the Desktop Mode shell, and helpers like hearth-buttons): shipped beside the app.
pub fn desktop_dir() -> PathBuf {
    if let Some(d) = std::env::var_os("HEARTH_DESKTOP_DIR") { return PathBuf::from(d); }
    let exe = std::env::current_exe().unwrap_or_default();
    let mut c: Vec<PathBuf> = Vec::new();
    if let Some(dir) = exe.parent() {
        c.push(dir.join("desktop"));
        c.push(dir.join("../share/hearth/desktop"));
        c.push(dir.join("../lib/hearth/desktop"));
        // Running from the source tree (target/release/hearth): the Linux host's folder.
        for a in dir.ancestors() { c.push(a.join("hosts/linux/desktop")); }
    }
    c.push(PathBuf::from("/usr/share/nimbus/desktop"));   // Hearth, the OS
    c.push(PathBuf::from("/usr/lib/hearth/desktop"));
    c.push(PathBuf::from("/usr/lib/nimbus/resources/desktop"));
    c.into_iter().find(|p| p.join("shell").is_dir()).unwrap_or_else(|| PathBuf::from("/usr/lib/hearth/desktop"))
}
