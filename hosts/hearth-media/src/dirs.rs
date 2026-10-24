//! Where things go: ~/Videos/Recordings, ~/Pictures/Camera... (the user's own Videos, Pictures and Music folders
//! from ~/.config/user-dirs.dirs, so a translated "Vidéos" works), and the apps' runtime, cache and config folders.

use std::path::{Path, PathBuf};

pub fn home() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/tmp"))
}

fn xdg(var: &str, fallback: &str) -> PathBuf {
    std::env::var_os(var).map(PathBuf::from).filter(|p| p.is_absolute()).unwrap_or_else(|| home().join(fallback))
}

/// One folder from user-dirs.dirs' text: XDG_VIDEOS_DIR="$HOME/Videos" → /home/you/Videos.
pub fn parse_user_dirs(text: &str, home: &Path, key: &str) -> Option<PathBuf> {
    let want = format!("XDG_{key}_DIR=");
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('#') { continue; }
        let Some(v) = line.strip_prefix(&want) else { continue };
        let v = v.trim().trim_matches('"');
        let p = if let Some(rest) = v.strip_prefix("$HOME") {
            home.join(rest.trim_start_matches('/'))
        } else {
            PathBuf::from(v)
        };
        // "$HOME/" alone means the home folder itself: no special folder then.
        if p.is_absolute() && p != home { return Some(p); }
    }
    None
}

/// The user's folder of a kind ("VIDEOS", "PICTURES", "MUSIC"), else ~/<fallback>.
pub fn user_dir(key: &str, fallback: &str) -> PathBuf {
    let h = home();
    let f = xdg("XDG_CONFIG_HOME", ".config").join("user-dirs.dirs");
    std::fs::read_to_string(f).ok().and_then(|t| parse_user_dirs(&t, &h, key)).unwrap_or_else(|| h.join(fallback))
}

pub fn videos() -> PathBuf { user_dir("VIDEOS", "Videos") }
pub fn pictures() -> PathBuf { user_dir("PICTURES", "Pictures") }
pub fn music() -> PathBuf { user_dir("MUSIC", "Music") }

/// $XDG_RUNTIME_DIR/<app> (only this user can see it), made if needed.
pub fn runtime(app: &str) -> PathBuf {
    let base = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).filter(|p| p.is_dir())
        .unwrap_or_else(|| std::env::temp_dir().join(format!("hearth-{}", crate::tools::uid())));
    let d = base.join(app);
    make_private(&d);
    d
}
pub fn cache(app: &str) -> PathBuf { let d = xdg("XDG_CACHE_HOME", ".cache").join(app); let _ = std::fs::create_dir_all(&d); d }
pub fn config(app: &str) -> PathBuf { let d = xdg("XDG_CONFIG_HOME", ".config").join(app); let _ = std::fs::create_dir_all(&d); d }

fn make_private(d: &Path) {
    let _ = std::fs::create_dir_all(d);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(d, std::fs::Permissions::from_mode(0o700));
    }
}

/// The local date and time for file names: 2026-10-22 14-03-05.
pub fn stamp() -> String {
    #[cfg(unix)]
    unsafe {
        let t = libc::time(std::ptr::null_mut());
        let mut tm: libc::tm = std::mem::zeroed();
        libc::localtime_r(&t, &mut tm);
        format!("{:04}-{:02}-{:02} {:02}-{:02}-{:02}", tm.tm_year + 1900, tm.tm_mon + 1, tm.tm_mday, tm.tm_hour, tm.tm_min, tm.tm_sec)
    }
    #[cfg(not(unix))]
    { format!("{}", now_ms() / 1000) }
}

pub fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

/// A new file name in `dir`: "<prefix> <date time>.<ext>", with " (2)", " (3)"... when that's taken. The folder is made.
pub fn unique(dir: &Path, prefix: &str, ext: &str) -> PathBuf {
    let _ = std::fs::create_dir_all(dir);
    let base = format!("{prefix} {}", stamp());
    let mut p = dir.join(format!("{base}.{ext}"));
    let mut n = 2;
    while p.exists() && n < 1000 {
        p = dir.join(format!("{base} ({n}).{ext}"));
        n += 1;
    }
    p
}

/// Writes a file in one go (a reader never sees half of it).
pub fn write_atomic(p: &Path, data: &[u8]) -> std::io::Result<()> {
    if let Some(d) = p.parent() { std::fs::create_dir_all(d)?; }
    let tmp = p.with_extension(format!("tmp{}", std::process::id()));
    std::fs::write(&tmp, data)?;
    std::fs::rename(&tmp, p)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn user_dirs() {
        let t = "# comment\nXDG_DESKTOP_DIR=\"$HOME/Desktop\"\nXDG_VIDEOS_DIR=\"$HOME/Vidéos\"\nXDG_MUSIC_DIR=\"$HOME/\"\nXDG_PICTURES_DIR=\"/data/pics\"\n";
        let h = Path::new("/home/you");
        assert_eq!(parse_user_dirs(t, h, "VIDEOS").unwrap(), PathBuf::from("/home/you/Vidéos"));
        assert_eq!(parse_user_dirs(t, h, "PICTURES").unwrap(), PathBuf::from("/data/pics"));
        assert_eq!(parse_user_dirs(t, h, "MUSIC"), None);
        assert_eq!(parse_user_dirs(t, h, "DOCUMENTS"), None);
    }
    #[test]
    fn names() {
        let s = stamp();
        assert_eq!(s.len(), 19, "{s}");
        let d = std::env::temp_dir().join(format!("hm-names-{}", std::process::id()));
        let a = unique(&d, "Recording", "mp4");
        std::fs::write(&a, b"x").unwrap();
        let b = unique(&d, "Recording", "mp4");
        assert_ne!(a, b);
        assert!(b.to_string_lossy().contains("(2)") || !b.to_string_lossy().contains(&s));
        write_atomic(&d.join("x.json"), b"{}").unwrap();
        assert_eq!(std::fs::read(d.join("x.json")).unwrap(), b"{}");
        let _ = std::fs::remove_dir_all(&d);
    }
}
