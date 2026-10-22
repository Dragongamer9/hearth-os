//! Hearth Disks: Hearth OS's own disk manager. See every drive with its health and a map of its partitions;
//! mount, safely remove, rename, format, make and delete partitions, check and repair, and "Use for games" (a
//! Steam library). Linux: udisks2 over D-Bus (linux.rs; polkit asks for the password when it's needed). Windows:
//! the Storage PowerShell module (winps.rs; changes run elevated, so Windows asks first).
//! model.rs is what both show; safety.rs decides what's offered and checks it again right before it runs.

pub mod model;
pub mod safety;
pub mod udisks;
pub mod windows;
#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(windows)]
pub mod winps;

use model::{Drive, Partition};
use safety::{Opts, Plan};

/// Which system this is: the rules and the file system choices differ a little.
pub const HOST: &str = if cfg!(windows) { "windows" } else { "linux" };

/// Tells the page how a long job is going: what's happening, and how far (0 to 1) when that's known.
pub type Progress<'a> = &'a (dyn Fn(&str, Option<f64>) + Sync);

const GONE: &str = "That drive isn't there anymore. It may have been unplugged.";

/// Every drive, with what can be done to each.
pub fn list() -> Result<Vec<Drive>, String> {
    #[cfg(target_os = "linux")]
    let mut d = linux::list()?;
    #[cfg(windows)]
    let mut d = winps::list()?;
    #[cfg(not(any(target_os = "linux", windows)))]
    let mut d: Vec<Drive> = Vec::new();
    safety::annotate(&mut d, HOST);
    Ok(d)
}

/// A drive by its id, or a partition (or free space) and its drive.
pub fn find<'a>(drives: &'a [Drive], id: &str) -> Option<(&'a Drive, Option<&'a Partition>)> {
    for d in drives {
        if d.id == id { return Some((d, None)); }
        if let Some(p) = d.partitions.iter().find(|p| p.id == id) { return Some((d, Some(p))); }
    }
    None
}

/// What an action would do (from a fresh look at the drives).
pub fn plan(action: &str, id: &str) -> Result<Plan, String> {
    let drives = list()?;
    let (d, p) = find(&drives, id).ok_or(GONE)?;
    Ok(safety::plan(action, d, p, HOST))
}

/// Runs an action: looks at the drives again, checks it again, then does it. Returns what to tell you.
pub fn run(action: &str, id: &str, opts: &Opts, progress: Progress) -> Result<String, String> {
    progress("Checking the drive", None);
    let drives = list()?;
    let (d, p) = find(&drives, id).ok_or(GONE)?;
    safety::approve(action, d, p, HOST, opts)?;
    #[cfg(target_os = "linux")]
    { linux::execute(action, d, p, opts, progress) }
    #[cfg(windows)]
    { winps::execute(action, d, p, opts, progress) }
    #[cfg(not(any(target_os = "linux", windows)))]
    { let _ = (d, p); Err("Disks doesn't work on this system.".into()) }
}

/// A folder name for a mount point or library from a label: letters, digits, - and _ only.
pub fn safe_name(label: &str, fallback: &str) -> String {
    let s: String = label.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '-' }).collect();
    let s = s.trim_matches('-').to_string();
    if s.is_empty() { fallback.chars().filter(|c| c.is_ascii_alphanumeric()).take(8).collect() } else { s.chars().take(32).collect() }
}

#[cfg(test)]
mod tests {
    #[test]
    fn folder_names() {
        assert_eq!(super::safe_name("My Games", "x"), "My-Games");
        assert_eq!(super::safe_name("../..", "1234-ABCD-ef"), "1234ABCD");
        assert_eq!(super::safe_name("", "abc"), "abc");
    }

    #[test]
    fn finds_things() {
        let d = crate::udisks::parse(&crate::udisks::tests::sample(), &crate::udisks::Ctx { uid: 1000, steam: &|_: &[String]| false, usage: &|_: &str| None });
        let (drive, part) = super::find(&d, "/org/freedesktop/UDisks2/block_devices/mmcblk0p1").unwrap();
        assert_eq!(drive.kind, "sd");
        assert_eq!(part.unwrap().label, "SWITCH");
        assert!(super::find(&d, "/org/freedesktop/UDisks2/drives/SanDisk_SD").unwrap().1.is_none());
        assert!(super::find(&d, "nope").is_none());
    }
}
