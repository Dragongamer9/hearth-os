//! Quick updates for Hearth's own apps and Mantel (Hearth OS 1.17; docs/HEARTH-STORE.md "Quick updates"; mockups
//! docs/mockups/1.17-quick-updates). The work is done by `/usr/libexec/hearth/hearth-part`, the same helper every
//! app's start asks which copy to use, so the Store and the starts never disagree: the Store only reads what's
//! installed (its part.json and its failures) and asks the helper to install, remove or try again.

use crate::catalog::Part;
use std::path::PathBuf;

/// The helper (HEARTH_PART for the tests' fixture).
pub fn helper() -> PathBuf {
    std::env::var_os("HEARTH_PART").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/usr/libexec/hearth/hearth-part"))
}

fn home() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/nonexistent"))
}

fn root() -> PathBuf {
    std::env::var_os("HEARTH_PARTS").map(PathBuf::from).unwrap_or_else(|| home().join(".local/share/hearth/parts"))
}

fn state_dir() -> PathBuf {
    std::env::var_os("HEARTH_PARTS_STATE").map(PathBuf::from).unwrap_or_else(|| home().join(".local/state/hearth/parts"))
}

fn run_dir() -> PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/tmp"))
}

/// What's installed of one part.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Have {
    pub version: String,
    pub os: String,
    /// When it was installed (unix time).
    pub installed: i64,
    /// Failed starts in a row of this version (2: the built-in one until the next version).
    pub fails: u32,
    /// It failed in this sign-in (the built-in one until you sign in again).
    pub failed_now: bool,
}

impl Have {
    /// The built-in one is used instead of it until the next version (it didn't start twice).
    pub fn gave_up(&self) -> bool {
        self.fails >= 2
    }
}

fn json(p: PathBuf) -> Option<serde_json::Value> {
    std::fs::read(p).ok().and_then(|b| serde_json::from_slice(&b).ok())
}

/// The part's copy in your home folder, if there is one.
pub fn have(id: &str) -> Option<Have> {
    if id.is_empty() || id.contains('/') || id.starts_with('.') {
        return None;
    }
    let i = json(root().join(id).join("part.json"))?;
    let s = |k: &str| i.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
    let version = s("version");
    if version.is_empty() {
        return None;
    }
    let st = json(state_dir().join(format!("{id}.json"))).unwrap_or_default();
    let fails = if st.get("version").and_then(|x| x.as_str()) == Some(version.as_str()) { st.get("fails").and_then(|x| x.as_u64()).unwrap_or(0) as u32 } else { 0 };
    let os = match i.get("os") {
        Some(serde_json::Value::Array(a)) => a.iter().filter_map(|x| x.as_str()).collect::<Vec<_>>().join(", "),
        _ => s("os"),
    };
    Some(Have { version, os, installed: i.get("installed").and_then(|x| x.as_i64()).unwrap_or(0), fails, failed_now: run_dir().join(format!("hearth-part-{id}.failed")).exists() })
}

/// Made for this Hearth OS: "1.17" fits 1.17 and 1.17.2 (never 1.170); nothing known: no.
pub fn fits(made_for: &str, here: &str) -> bool {
    let here = here.trim();
    !here.is_empty() && made_for.split(',').map(str::trim).any(|m| !m.is_empty() && (here == m || here.starts_with(&format!("{m}."))))
}

/// The version that's running now: the Store's copy when it may be used, else the one built into Hearth OS (shown as
/// the Hearth OS version, which is the built-in one's version).
pub fn running(h: Option<&Have>, hearth: &str) -> String {
    match h {
        Some(h) if fits(&h.os, hearth) && !h.gave_up() && !h.failed_now => h.version.clone(),
        _ => hearth.to_string(),
    }
}

/// The update to offer: made for this Hearth OS and newer than what runs (and than the built-in one).
pub fn offer<'a>(p: &'a Part, h: Option<&Have>, hearth: &str) -> Option<&'a Part> {
    if !fits(&p.os, hearth) {
        return None;
    }
    let base = match h {
        Some(h) if fits(&h.os, hearth) && crate::newer(&h.version, hearth) => h.version.clone(),
        _ => hearth.to_string(),
    };
    (crate::newer(&p.version, &base) && h.map(|h| h.version != p.version).unwrap_or(true)).then_some(p)
}

fn helper_run(args: &[&str]) -> Result<(), String> {
    let out = std::process::Command::new(helper()).args(args).output().map_err(|e| format!("Hearth's update helper couldn't start ({e})."))?;
    if out.status.success() {
        Ok(())
    } else {
        let err = String::from_utf8_lossy(&out.stderr);
        let last = err.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("").trim_start_matches("hearth-part: ").to_string();
        Err(if last.is_empty() { "The update didn't install; nothing changed.".into() } else { format!("{last}.") })
    }
}

/// Download, check against the signed list's SHA-256, put in place in one step (hearth-part install). Then, for
/// Mantel, its desktop restarts to use it (labwc's autostart starts it again at once; windows stay open).
pub fn install(p: &Part) -> Result<(), String> {
    helper_run(&["install", &p.id, &p.version, &p.url, &p.sha256, &p.os, &p.path])?;
    if p.id == "mantel" {
        restart_mantel();
    }
    Ok(())
}

/// Back to the built-in one for good (the copy goes to the Trash).
pub fn remove(id: &str) -> Result<(), String> {
    helper_run(&["remove", id])?;
    if id == "mantel" {
        restart_mantel();
    }
    Ok(())
}

/// Try it again: its failures are forgotten (hearth-part retry).
pub fn retry(id: &str) -> Result<(), String> {
    helper_run(&["retry", id])
}

fn restart_mantel() {
    // Only Mantel's own shell (quickshell -p …/shell), only yours; the session's loop starts it again right away.
    let uid = unsafe { libc::getuid() }.to_string();
    let _ = std::process::Command::new("pkill").args(["-u", &uid, "-f", "quickshell -p .*/shell$"]).status();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn part(v: &str, os: &str) -> Part {
        Part { id: "hearth-files-app".into(), version: v.into(), os: os.into(), path: "hearth-files-app".into(), url: "https://x/y".into(), sha256: "0".repeat(64), size: 1, notes: vec![] }
    }

    #[test]
    fn fits_only_its_hearth_os() {
        assert!(fits("1.17", "1.17") && fits("1.17", "1.17.2") && !fits("1.17", "1.170") && !fits("1.1", "1.17") && !fits("1.17", "") && fits("1.16, 1.17", "1.17"));
    }

    #[test]
    fn offers_only_newer_for_this_hearth_os() {
        assert!(offer(&part("1.17.1", "1.17"), None, "1.17").is_some());
        assert!(offer(&part("1.17.1", "1.16"), None, "1.17").is_none());
        assert!(offer(&part("1.17", "1.17"), None, "1.17").is_none()); // the built-in one is that already
        let h = Have { version: "1.17.1".into(), os: "1.17".into(), ..Default::default() };
        assert!(offer(&part("1.17.1", "1.17"), Some(&h), "1.17").is_none());
        assert!(offer(&part("1.17.2", "1.17"), Some(&h), "1.17").is_some());
        let gave_up = Have { fails: 2, ..h.clone() };
        assert_eq!(running(Some(&gave_up), "1.17"), "1.17");
        assert_eq!(running(Some(&h), "1.17"), "1.17.1");
    }
}
