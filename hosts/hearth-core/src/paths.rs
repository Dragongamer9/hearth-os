//! Where Hearth keeps things. The same folders the Electron host uses, so both read the same data.

use std::path::{Path, PathBuf};

pub fn home() -> PathBuf { std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/")) }

/// ~/.local/share/nimbus (or $XDG_DATA_HOME/nimbus)
pub fn data_dir() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|| home().join(".local/share")).join("nimbus")
}

/// ~/.config/nimbus (or $XDG_CONFIG_HOME/nimbus)
pub fn config_dir() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| home().join(".config")).join("nimbus")
}

/// Writes a file all at once (a temporary file, then a rename), so a crash never leaves half a file. (No fsync:
/// ext4 and btrfs put a file's contents on the drive before a rename that replaces it, and an fsync here, with the
/// bridge held, could wait seconds behind a big download.) The temporary file's name is this write's own: two writes of the same file at once
/// (or of library.json and library.cfg, which shared library.tmp) used to write into one temporary file, and the
/// rename could put a mix of both in place.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    if let Some(dir) = path.parent() { if !dir.as_os_str().is_empty() { std::fs::create_dir_all(dir)?; } }
    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let tmp = path.with_file_name(format!(".{name}.{}-{}.tmp", std::process::id(), SEQ.fetch_add(1, Ordering::SeqCst)));
    let r = (|| {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        drop(f);
        std::fs::rename(&tmp, path)
    })();
    if r.is_err() { let _ = std::fs::remove_file(&tmp); }
    r
}

/// Where to look for Hearth's desktop files, in order. `share`: $NIMBUS_SHARE, which the Hearth session sets to
/// where it keeps them (/usr/share/nimbus on Hearth OS, /usr/local/share/nimbus when the session was added from
/// the app); the helpers (hearth-power, hearth-hhd, hearth-buttons...) must be the same ones the session runs.
pub fn desktop_candidates(exe: &Path, share: Option<&Path>) -> Vec<PathBuf> {
    let mut c: Vec<PathBuf> = Vec::new();
    if let Some(s) = share.filter(|s| !s.as_os_str().is_empty()) { c.push(s.join("desktop")); }
    if let Some(dir) = exe.parent() {
        c.push(dir.join("desktop"));
        c.push(dir.join("../share/hearth/desktop"));
        c.push(dir.join("../lib/hearth/desktop"));
        // Running from the source tree (target/release/hearth): the Linux host's folder.
        for a in dir.ancestors() { c.push(a.join("hosts/linux/desktop")); }
    }
    c.push(PathBuf::from("/usr/share/nimbus/desktop"));   // Hearth, the OS
    c.push(PathBuf::from("/usr/local/share/nimbus/desktop"));   // the session added from the app (install-session.sh)
    c.push(PathBuf::from("/usr/lib/hearth/desktop"));
    c.push(PathBuf::from("/usr/lib/nimbus/resources/desktop"));
    c
}

/// Hearth's desktop files (the Desktop Mode shell, and helpers like hearth-buttons): shipped beside the app.
pub fn desktop_dir() -> PathBuf {
    if let Some(d) = std::env::var_os("HEARTH_DESKTOP_DIR") { return PathBuf::from(d); }
    let exe = std::env::current_exe().unwrap_or_default();
    let share = std::env::var_os("NIMBUS_SHARE").map(PathBuf::from);
    desktop_candidates(&exe, share.as_deref()).into_iter().find(|p| p.join("shell").is_dir()).unwrap_or_else(|| PathBuf::from("/usr/lib/hearth/desktop"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_whole_files_and_leaves_no_temporary_ones() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("sub/library.json");
        write_atomic(&f, b"one").unwrap();
        write_atomic(&f, b"two").unwrap();
        assert_eq!(std::fs::read_to_string(&f).unwrap(), "two");
        let names: Vec<String> = std::fs::read_dir(d.path().join("sub")).unwrap().flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect();
        assert_eq!(names, vec!["library.json".to_string()]);
    }

    #[test]
    fn writes_of_the_same_file_at_once_never_mix() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("cache.json");
        let big = |c: u8| vec![c; 256 * 1024];
        let ts: Vec<_> = (0..8u8).map(|i| { let f = f.clone(); std::thread::spawn(move || for _ in 0..10 { write_atomic(&f, &big(b'a' + i)).unwrap(); }) }).collect();
        for t in ts { t.join().unwrap(); }
        let got = std::fs::read(&f).unwrap();
        assert_eq!(got.len(), 256 * 1024);
        assert!(got.iter().all(|b| *b == got[0]), "a mix of two writes");
    }

    #[test]
    fn the_sessions_desktop_files_come_first() {
        let c = desktop_candidates(Path::new("/tmp/x/Hearth.AppImage"), Some(Path::new("/usr/local/share/nimbus")));
        assert_eq!(c[0], PathBuf::from("/usr/local/share/nimbus/desktop"));
        let c = desktop_candidates(Path::new("/tmp/x/Hearth.AppImage"), None);
        assert!(c.contains(&PathBuf::from("/usr/local/share/nimbus/desktop")));
        assert_eq!(c[0], PathBuf::from("/tmp/x/desktop"));
        assert_eq!(desktop_candidates(Path::new("/tmp/x/h"), Some(Path::new("")))[0], PathBuf::from("/tmp/x/desktop"));
    }
}
