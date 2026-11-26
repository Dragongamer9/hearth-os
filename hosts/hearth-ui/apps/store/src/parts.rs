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

/// A test version's update: only once you've got it, made for this Hearth OS, newer than yours.
pub fn test_offer<'a>(p: &'a Part, h: Option<&Have>, hearth: &str) -> Option<&'a Part> {
    let h = h?;
    (fits(&p.os, hearth) && h.version != p.version && crate::newer(&p.version, &h.version)).then_some(p)
}

/// A test version can be got here: made for this Hearth OS (whatever its own number).
pub fn test_fits(p: &Part, hearth: &str) -> bool {
    fits(&p.os, hearth)
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
/// Ok(Some(seconds)) for Mantel: how long its desktop took to come back with the new one (None: not seen within 15 s).
pub fn install(p: &Part) -> Result<Option<u64>, String> {
    install_channel(p, false)
}

/// The same, for the main or the test version (a system part's test version comes from the list's "tests").
pub fn install_channel(p: &Part, test: bool) -> Result<Option<u64>, String> {
    if p.system {
        install_system(p, test)?;
        return Ok(None);
    }
    helper_run(&["install", &p.id, &p.version, &p.url, &p.sha256, &p.os, &p.path])?;
    if p.id == "mantel" || p.id == "mantel-rust" {
        let t0 = std::time::Instant::now();
        restart_mantel();
        return Ok(came_back(t0));
    }
    Ok(None)
}

/// Whether your desktop's shell is running again (polled for up to 15 s after the restart), and after how long.
fn came_back(t0: std::time::Instant) -> Option<u64> {
    let uid = unsafe { libc::getuid() }.to_string();
    std::thread::sleep(std::time::Duration::from_millis(300));
    while t0.elapsed().as_secs() < 15 {
        let up = ["quickshell -p .*/shell$", "/hearth-mantel session$"]
            .iter()
            .any(|pat| std::process::Command::new("pgrep").args(["-u", &uid, "-f", pat]).stdout(std::process::Stdio::null()).status().map(|s| s.success()).unwrap_or(false));
        if up {
            return Some(t0.elapsed().as_secs().max(1));
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
    None
}

/// Back to the built-in one for good (the copy goes to the Trash).
pub fn remove(id: &str) -> Result<(), String> {
    helper_run(&["remove", id])?;
    if id == "mantel" || id == "mantel-rust" {
        restart_mantel();
    }
    Ok(())
}

/// Try it again: its failures are forgotten (hearth-part retry).
pub fn retry(id: &str) -> Result<(), String> {
    helper_run(&["retry", id])
}

// ------------------------------------------------------------------ system parts (1.18: helpers that run as root)
/// hearth-system-part's view of the system parts: id → the version in use ("" for the built-in one) and versions it
/// gave up on. Read as you (status needs no root); the fixture's for CI's pictures.
pub fn system_status() -> std::collections::BTreeMap<String, Have> {
    let mut out = std::collections::BTreeMap::new();
    let text = if let Some(f) = crate::paths::fixture() {
        std::fs::read_to_string(f.join("system-parts.json")).unwrap_or_default()
    } else if std::path::Path::new("/var/lib/hearth/system-parts").is_dir() {
        std::process::Command::new("/usr/libexec/hearth/hearth-system-part").arg("status").output().map(|o| String::from_utf8_lossy(&o.stdout).to_string()).unwrap_or_default()
    } else {
        String::new()
    };
    let v: serde_json::Value = serde_json::from_str(&text).unwrap_or_default();
    if let Some(m) = v.as_object() {
        for (id, x) in m {
            let s = |k: &str| x.get(k).and_then(|y| y.as_str()).unwrap_or("").to_string();
            let version = s("version");
            if version.is_empty() {
                continue;
            }
            let bad = x.get("bad").and_then(|b| b.as_array()).map(|b| b.iter().any(|y| y.as_str() == Some(version.as_str()))).unwrap_or(false);
            out.insert(id.clone(), Have { version, os: if x.get("fits").and_then(|f| f.as_bool()).unwrap_or(false) { "fits".into() } else { String::new() }, installed: 0, fails: if bad { 2 } else { 0 }, failed_now: false });
        }
    }
    out
}

/// A system part's update: the admin password (pkexec, 49-hearth-admin.rules), then hearth-system-part checks the
/// signed list itself (the built-in one, or the Store's refreshed one, given here only as text it checks) and installs
/// only the copy that list names. Its services start again with it; the PC doesn't restart.
pub fn install_system(p: &Part, test: bool) -> Result<(), String> {
    use std::io::Write;
    let (cj, cs) = crate::catalog::cached_paths();
    let extra = match (std::fs::read_to_string(&cj), std::fs::read_to_string(&cs)) {
        (Ok(c), Ok(s)) => serde_json::json!({"catalog": c, "sig": s.trim()}).to_string(),
        _ => "null".into(),
    };
    let mut child = std::process::Command::new("pkexec")
        .args(["/usr/libexec/nimbus/hearth-admin", "system-part-install", &p.id])
        .args(if test { &["test"][..] } else { &[][..] })
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("The admin helper couldn't start ({e})."))?;
    if let Some(mut i) = child.stdin.take() {
        let _ = i.write_all(extra.as_bytes());
    }
    let out = child.wait_with_output().map_err(|e| format!("{e}"))?;
    let text = String::from_utf8_lossy(&out.stdout);
    if out.status.success() && !text.lines().any(|l| l.starts_with("error: ")) {
        Ok(())
    } else if out.status.code() == Some(126) || out.status.code() == Some(127) {
        Err("It needs the admin password; nothing changed.".into())
    } else {
        Err(text.lines().rev().find_map(|l| l.strip_prefix("error: ")).map(|s| format!("{s}; nothing changed.")).unwrap_or_else(|| "It didn't install; nothing changed.".into()))
    }
}

/// Remove or try again a system part (the admin password: hearth-admin through pkexec).
pub fn system_cmd(cmd: &str, id: &str) -> Result<(), String> {
    let out = std::process::Command::new("pkexec").args(["/usr/libexec/nimbus/hearth-admin", cmd, id]).stdin(std::process::Stdio::null()).output().map_err(|e| format!("The admin helper couldn't start ({e})."))?;
    let text = String::from_utf8_lossy(&out.stdout);
    if out.status.success() && !text.lines().any(|l| l.starts_with("error: ")) {
        Ok(())
    } else {
        Err(text.lines().rev().find_map(|l| l.strip_prefix("error: ")).map(|s| format!("{s}; nothing changed.")).unwrap_or_else(|| "It needs the admin password; nothing changed.".into()))
    }
}

// ------------------------------------------------------------------ Decky plugins (Hearth Sync, Hearth Link, Hearth Music)
pub const DECKY: &str = "/usr/libexec/nimbus/hearth-decky";

/// hearth-decky plugins-status (as you): folder → (installed or built-in version, newer one on offer or "").
pub fn decky_status() -> std::collections::BTreeMap<String, (String, String)> {
    let text = if let Some(f) = crate::paths::fixture() {
        std::fs::read_to_string(f.join("decky-status.json")).unwrap_or_default()
    } else if std::path::Path::new(DECKY).exists() {
        crate::net::run_short(30, &[DECKY, "plugins-status"]).unwrap_or_default()
    } else {
        String::new()
    };
    decky_parse(&text)
}

pub fn decky_parse(text: &str) -> std::collections::BTreeMap<String, (String, String)> {
    let mut out = std::collections::BTreeMap::new();
    let line = text.lines().rev().map(str::trim).find(|l| l.starts_with('{')).unwrap_or("");
    let v: serde_json::Value = serde_json::from_str(line).unwrap_or_default();
    let avail = v.get("available").cloned().unwrap_or_default();
    if let Some(m) = v.get("plugins").and_then(|p| p.as_object()) {
        for (id, x) in m {
            let s = |k: &str| x.get(k).and_then(|y| y.as_str()).unwrap_or("").to_string();
            let have = if s("installed").is_empty() { s("builtin") } else { s("installed") };
            let new = avail.get(id).and_then(|y| y.as_str()).unwrap_or("").to_string();
            out.insert(id.clone(), (have, new));
        }
    }
    out
}

/// Get the newer Hearth plugins (checked against their release's SHA-256 by hearth-decky; put in place at Decky's next
/// start, the built-in ones kept to go back to).
pub fn decky_update() -> Result<(), String> {
    let out = std::process::Command::new(DECKY).arg("plugins-update").output().map_err(|e| format!("Hearth's plugin updater couldn't start ({e})."))?;
    let text = String::from_utf8_lossy(&out.stdout);
    let v: serde_json::Value = text.lines().rev().find_map(|l| serde_json::from_str(l.trim()).ok()).unwrap_or_default();
    if v.get("ok").and_then(|x| x.as_bool()).unwrap_or(false) {
        Ok(())
    } else {
        Err(v.get("error").and_then(|x| x.as_str()).map(|s| format!("{s}.")).unwrap_or_else(|| "The plugins didn't update; nothing changed.".into()))
    }
}

fn restart_mantel() {
    // Only Mantel's own shell (quickshell -p …/shell), only yours; the session's loop starts it again right away.
    let uid = unsafe { libc::getuid() }.to_string();
    let _ = std::process::Command::new("pkill").args(["-u", &uid, "-f", "quickshell -p .*/shell$"]).status();
    // Mantel in Rust (test): its one program; labwc's autostart then starts whichever may be used.
    let _ = std::process::Command::new("pkill").args(["-u", &uid, "-f", "/hearth-mantel session$"]).status();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn part(v: &str, os: &str) -> Part {
        Part { id: "hearth-files-app".into(), version: v.into(), os: os.into(), path: "hearth-files-app".into(), url: "https://x/y".into(), sha256: "0".repeat(64), size: 1, notes: vec![], missing: vec![], system: false, restart: String::new() }
    }

    #[test]
    fn a_test_version_is_only_updated_once_you_have_it() {
        let p = part("1.17.2", "1.17");
        assert!(test_offer(&p, None, "1.17").is_none(), "not got: no update offered");
        let h = Have { version: "1.17.1".into(), os: "1.17".into(), ..Default::default() };
        assert!(test_offer(&p, Some(&h), "1.17.4").is_some(), "its own number, not the Hearth OS one");
        let same = Have { version: "1.17.2".into(), os: "1.17".into(), ..Default::default() };
        assert!(test_offer(&p, Some(&same), "1.17").is_none());
        assert!(test_fits(&p, "1.17.3") && !test_fits(&p, "1.18"));
    }

    #[test]
    fn reads_the_decky_plugins() {
        let m = decky_parse("{\"ok\": true, \"plugins\": {\"decky-hearth-sync\": {\"installed\": \"1.18.0\", \"builtin\": \"1.18.0\"}, \"decky-hearth-link\": {\"installed\": null, \"builtin\": \"1.18.0\"}}, \"available\": {\"decky-hearth-sync\": \"1.18.1\"}}");
        assert_eq!(m["decky-hearth-sync"], ("1.18.0".to_string(), "1.18.1".to_string()));
        assert_eq!(m["decky-hearth-link"], ("1.18.0".to_string(), String::new()));
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
