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
