//! Opening and saving files. Big files never freeze the window: up to EDIT_MAX they open normally, up to VIEW_MAX
//! read-only, and anything bigger shows its first (or last) PART_SIZE bytes read-only, read straight from the
//! file without loading the rest. Saving writes a new file next to the old one and swaps it in, so a crash or a
//! full disk never leaves half a file; permissions are kept, and a link is saved through to the file it points at.

use crate::textfile::{self, Decoded};
use serde::Serialize;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

/// Up to this size a file can be edited.
pub const EDIT_MAX: u64 = 12 * 1024 * 1024;
/// Up to this size a file is shown whole (read-only above EDIT_MAX).
pub const VIEW_MAX: u64 = 48 * 1024 * 1024;
/// Bigger files: this much of the start (or the end) is shown.
pub const PART_SIZE: u64 = 8 * 1024 * 1024;

#[derive(Serialize, Clone, Debug)]
pub struct Opened {
    pub path: String,
    pub name: String,
    #[serde(flatten)]
    pub decoded: Decoded,
    pub size: u64,
    pub mtime: u64,
    /// You can save over it (false: read-only, offer Save as… or, on Linux, Edit as administrator).
    pub writable: bool,
    /// Too big to edit: shown read-only.
    pub too_big: bool,
    /// Only part of it is shown: "start" or "end" (then `shown` bytes of `size`).
    pub part: Option<String>,
    pub shown: u64,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Stat {
    pub exists: bool,
    pub size: u64,
    pub mtime: u64,
    pub writable: bool,
}

pub fn mtime_ms(m: &fs::Metadata) -> u64 {
    m.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_millis() as u64).unwrap_or(0)
}

/// Whether the file can be written (opening it for writing changes nothing). A file that isn't there yet: whether
/// its folder can take it.
pub fn can_write(p: &Path) -> bool {
    if p.is_file() { return OpenOptions::new().write(true).open(p).is_ok(); }
    match p.parent() {
        Some(d) if d.is_dir() => dir_writable(d),
        _ => false,
    }
}

#[cfg(unix)]
fn dir_writable(d: &Path) -> bool {
    use std::os::unix::ffi::OsStrExt;
    let Ok(c) = std::ffi::CString::new(d.as_os_str().as_bytes()) else { return false };
    unsafe { libc::access(c.as_ptr(), libc::W_OK | libc::X_OK) == 0 }
}
#[cfg(not(unix))]
fn dir_writable(d: &Path) -> bool {
    fs::metadata(d).map(|m| !m.permissions().readonly()).unwrap_or(false)
}

pub fn stat(p: &Path) -> Stat {
    match fs::metadata(p) {
        Ok(m) => Stat { exists: true, size: m.len(), mtime: mtime_ms(&m), writable: can_write(p) },
        Err(_) => Stat { exists: false, size: 0, mtime: 0, writable: can_write(p) },
    }
}

/// Plain-English reasons for the usual failures.
pub fn friendly(e: &std::io::Error, p: &Path) -> String {
    let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| p.display().to_string());
    match e.kind() {
        std::io::ErrorKind::NotFound => format!("“{name}” isn't there any more"),
        std::io::ErrorKind::PermissionDenied => format!("You don't have permission to open “{name}”"),
        _ => match e.raw_os_error() {
            Some(21) => format!("“{name}” is a folder"),
            Some(30) => format!("“{name}” is on a part of the system that can't be changed"),
            Some(28) => "The drive is full".into(),
            _ => format!("{e} ({name})"),
        },
    }
}

/// Opens `path`. `mode`: "auto" (the start, if it's too big to show whole), "end" (its last part: big logs),
/// "view" (whole, read-only, up to VIEW_MAX). `encoding` forces one (Reopen with encoding).
pub fn open(path: &Path, mode: &str, encoding: Option<&str>) -> Result<Opened, String> {
    let meta = fs::metadata(path).map_err(|e| friendly(&e, path))?;
    if meta.is_dir() { return Err(format!("“{}” is a folder", path.display())); }
    if !meta.is_file() { return Err(format!("“{}” isn't a regular file (a device or a pipe), so it isn't opened", path.display())); }
    let size = meta.len();
    let mut f = File::open(path).map_err(|e| friendly(&e, path))?;
    let (bytes, part) = if size > VIEW_MAX || (mode == "end" && size > PART_SIZE) {
        let mut buf = vec![0u8; PART_SIZE as usize];
        if mode == "end" {
            f.seek(SeekFrom::Start(size - PART_SIZE)).map_err(|e| friendly(&e, path))?;
            f.read_exact(&mut buf).map_err(|e| friendly(&e, path))?;
            // Start at a whole line (and so at a whole character).
            let cut = buf.iter().position(|b| *b == b'\n').map(|i| i + 1).unwrap_or(0);
            (buf.split_off(cut), Some("end".to_string()))
        } else {
            f.read_exact(&mut buf).map_err(|e| friendly(&e, path))?;
            (buf, Some("start".to_string()))
        }
    } else {
        let mut buf = Vec::with_capacity(size as usize);
        f.read_to_end(&mut buf).map_err(|e| friendly(&e, path))?;
        (buf, None)
    };
    let shown = bytes.len() as u64;
    let decoded = textfile::decode(&bytes, encoding);
    let shown_path = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    Ok(Opened {
        path: display_path(&shown_path),
        name: path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
        decoded,
        size,
        mtime: mtime_ms(&meta),
        writable: can_write(path),
        too_big: size > EDIT_MAX,
        part,
        shown,
    })
}

/// A path as it's shown and remembered (Windows' canonical form starts with \\?\, which nobody wants to see).
pub fn display_path(p: &Path) -> String {
    let s = p.to_string_lossy().into_owned();
    if cfg!(windows) {
        if let Some(rest) = s.strip_prefix(r"\\?\UNC\") { return format!(r"\\{rest}"); }
        if let Some(rest) = s.strip_prefix(r"\\?\") { return rest.to_string(); }
    }
    s
}

/// The file a save really writes: through a link to what it points at, so the link stays a link.
pub fn save_target(path: &Path) -> PathBuf {
    match fs::symlink_metadata(path) {
        Ok(m) if m.file_type().is_symlink() => fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf()),
        _ => path.to_path_buf(),
    }
}

/// Saves `bytes` as `path`. A new file is written beside it and swapped in; when that can't keep the file as it
/// was (someone else owns it, other names point at the same file, or the folder can't take a new file) the file
/// is written over in place instead. Returns the file's new state.
pub fn save(path: &Path, bytes: &[u8]) -> Result<Stat, String> {
    let target = save_target(path);
    let meta = fs::metadata(&target).ok();
    if let Some(m) = &meta {
        if m.is_dir() { return Err(format!("“{}” is a folder", target.display())); }
        if !m.is_file() { return Err(format!("“{}” isn't a regular file, so it isn't saved over", target.display())); }
    }
    let swap = meta.as_ref().map(|m| keeps_identity_when_swapped(m)).unwrap_or(true)
        && target.parent().map(dir_writable).unwrap_or(false);
    let done = if swap { save_swapped(&target, bytes, meta.as_ref()) } else { Err(std::io::Error::other("in place")) };
    if done.is_err() {
        if meta.is_none() && !swap {
            // A new file in a folder we can't write: the honest error.
            File::create(&target).map_err(|e| friendly(&e, &target))?;
        }
        let mut f = OpenOptions::new().write(true).create(true).truncate(true).open(&target).map_err(|e| friendly(&e, &target))?;
        f.write_all(bytes).map_err(|e| friendly(&e, &target))?;
        f.sync_all().map_err(|e| friendly(&e, &target))?;
    }
    Ok(stat(&target))
}

#[cfg(unix)]
fn keeps_identity_when_swapped(m: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    m.nlink() <= 1 && m.uid() == unsafe { libc::geteuid() }
}
#[cfg(not(unix))]
fn keeps_identity_when_swapped(_m: &fs::Metadata) -> bool { true }

fn save_swapped(target: &Path, bytes: &[u8], old: Option<&fs::Metadata>) -> std::io::Result<()> {
    let dir = target.parent().unwrap_or(Path::new("."));
    let name = target.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "file".into());
    let tmp = (0..100)
        .map(|i| dir.join(format!(".{name}.hearth-text-{}-{i}.tmp", std::process::id())))
        .find(|p| !p.exists())
        .ok_or_else(|| std::io::Error::other("no temporary name"))?;
    let result = (|| {
        let mut f = OpenOptions::new().write(true).create_new(true).open(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        drop(f);
        if let Some(m) = old { fs::set_permissions(&tmp, m.permissions())?; }
        fs::rename(&tmp, target)
    })();
    if result.is_err() { let _ = fs::remove_file(&tmp); }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("hearth-text-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn save_and_open_again() {
        let d = dir("save");
        let p = d.join("notes.txt");
        let st = save(&p, b"hello\r\nworld\r\n").unwrap();
        assert!(st.exists && st.writable);
        let o = open(&p, "auto", None).unwrap();
        assert_eq!((o.decoded.text.as_str(), o.decoded.eol.as_str(), o.too_big, o.part.clone()), ("hello\nworld\n", "crlf", false, None));
        save(&p, b"second").unwrap();
        assert_eq!(fs::read(&p).unwrap(), b"second");
        // Nothing left over beside it.
        assert_eq!(fs::read_dir(&d).unwrap().count(), 1);
        fs::remove_dir_all(&d).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn keeps_permissions_and_links() {
        use std::os::unix::fs::PermissionsExt;
        let d = dir("perm");
        let p = d.join("run.sh");
        fs::write(&p, "#!/bin/sh\n").unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(0o750)).unwrap();
        save(&p, b"#!/bin/sh\necho hi\n").unwrap();
        assert_eq!(fs::metadata(&p).unwrap().permissions().mode() & 0o777, 0o750);
        let link = d.join("link.sh");
        std::os::unix::fs::symlink(&p, &link).unwrap();
        save(&link, b"changed\n").unwrap();
        assert!(fs::symlink_metadata(&link).unwrap().file_type().is_symlink());
        assert_eq!(fs::read(&p).unwrap(), b"changed\n");
        // A second name for the same file (a hard link) still sees the change.
        let hard = d.join("hard.sh");
        fs::hard_link(&p, &hard).unwrap();
        save(&p, b"both\n").unwrap();
        assert_eq!(fs::read(&hard).unwrap(), b"both\n");
        fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn big_files_open_in_part() {
        let d = dir("big");
        let p = d.join("big.log");
        let line = "2026-10-22 12:00:00 something happened in the log file\n";
        let mut f = File::create(&p).unwrap();
        let mut n = 0u64;
        let mut i = 0u64;
        while n <= VIEW_MAX + 1024 { let l = format!("{i} {line}"); f.write_all(l.as_bytes()).unwrap(); n += l.len() as u64; i += 1; }
        drop(f);
        let o = open(&p, "auto", None).unwrap();
        assert_eq!(o.part.as_deref(), Some("start"));
        assert!(o.too_big);
        assert!(o.decoded.text.starts_with("0 2026"));
        let o = open(&p, "end", None).unwrap();
        assert_eq!(o.part.as_deref(), Some("end"));
        assert!(o.decoded.text.ends_with(&format!("{} {line}", i - 1)));
        assert!(o.decoded.text.chars().next().unwrap().is_ascii_digit());
        fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn folders_and_missing_files() {
        let d = dir("odd");
        assert!(open(&d, "auto", None).unwrap_err().contains("folder"));
        assert!(open(&d.join("nope.txt"), "auto", None).unwrap_err().contains("isn't there"));
        assert!(!stat(&d.join("nope.txt")).exists);
        assert!(stat(&d.join("nope.txt")).writable);
        fs::remove_dir_all(&d).unwrap();
    }
}
