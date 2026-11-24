//! The system underneath, in the Hearth Store (1.18; mockups docs/mockups/store-everything/store-2 and store-3,
//! checklist docs/mockups/checklists/store-everything.md; the plan docs/STORE-SYSTEM-PARTS.md).
//!
//! Hearth OS on Debian is one bootc image: Debian's packages are inside it, so the system's updates are a newer Hearth
//! OS image, named by the day it was made ("Debian updates · Nov 23"), and a new Hearth OS (1.19) when Dragongamer9
//! releases it. Both download only when you say (Get it now), or by themselves when you turned that on, and apply at
//! your next restart: the version before stays in the boot menu and comes back by itself if the new one doesn't start.
//! Nothing here ever restarts the PC by itself; Restart now is the only way, and only when you press it.
//!
//! The root work is hearth-admin's (update-status, update, update-pause, auto-update), through pkexec; the Store only
//! reads what it says.

use serde_json::Value;
use std::path::PathBuf;

pub const ADMIN: &str = "/usr/libexec/nimbus/hearth-admin";
pub const AUTO_FILE: &str = "/etc/hearth/auto-update";
pub const NO_AUTO_FILE: &str = "/etc/hearth/no-auto-update";

/// One version of the system (bootc's view).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Ver {
    pub version: String,
    /// When the image was made (RFC 3339), "" when bootc doesn't say.
    pub timestamp: String,
}

impl Ver {
    fn from(v: Option<&Value>) -> Option<Ver> {
        let v = v.filter(|v| v.is_object())?;
        let s = |k: &str| v.get(k).and_then(Value::as_str).unwrap_or("").to_string();
        Some(Ver { version: s("version"), timestamp: s("timestamp") })
    }
    /// "Nov 23" (the day the image was made), or "" when unknown.
    pub fn day(&self) -> String {
        day_words(&self.timestamp)
    }
}

/// What hearth-admin update-status / update-check said.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SysState {
    pub booted: Option<Ver>,
    /// Downloaded and waiting for the next restart.
    pub staged: Option<Ver>,
    /// Out, not downloaded yet.
    pub available: Option<Ver>,
    /// The version before, kept in the boot menu.
    pub rollback: Option<Ver>,
}

pub fn parse_status(text: &str) -> Option<SysState> {
    // hearth-admin prints log:/progress:/stage: lines first; the JSON is the last line.
    let line = text.lines().rev().map(str::trim).find(|l| l.starts_with('{'))?;
    let v: Value = serde_json::from_str(line).ok()?;
    Some(SysState { booted: Ver::from(v.get("booted")), staged: Ver::from(v.get("staged")), available: Ver::from(v.get("available")), rollback: Ver::from(v.get("rollback")) })
}

const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/// "2026-11-23T04:10:00Z" or "2026-11-23" → "Nov 23".
pub fn day_words(ts: &str) -> String {
    let d: Vec<&str> = ts.get(..10).unwrap_or("").split('-').collect();
    match (d.get(1).and_then(|m| m.parse::<usize>().ok()), d.get(2).and_then(|x| x.parse::<u32>().ok())) {
        (Some(m), Some(day)) if (1..=12).contains(&m) => format!("{} {day}", MONTHS[m - 1]),
        _ => String::new(),
    }
}

/// What the signed list says about the system update on offer (its top-level "system"): written by the release
/// workflow next to each dated image. Everything is optional; without it the Store says only what bootc knows.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SysInfo {
    /// The day of the image this describes ("2026-11-23"): used only when it's the one on offer.
    pub date: String,
    pub packages: u32,
    /// What they fix, in a few words ("security fixes for the kernel, Mesa and Firefox").
    pub fixes: String,
    /// Only what changed, in bytes.
    pub size: u64,
    /// "kernel 6.12.57 → 6.12.58", one each.
    pub changes: Vec<String>,
    /// The next Hearth OS ("1.19") and whether it's out.
    pub next: String,
    pub next_out: bool,
}

pub fn parse_info(v: Option<&Value>) -> Option<SysInfo> {
    let v = v.filter(|v| v.is_object())?;
    let s = |k: &str| v.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    Some(SysInfo {
        date: s("date"),
        packages: v.get("packages").and_then(Value::as_u64).unwrap_or(0) as u32,
        fixes: s("fixes"),
        size: v.get("size").and_then(Value::as_u64).unwrap_or(0),
        changes: v.get("changes").and_then(Value::as_array).map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default(),
        next: v.get("next").and_then(|n| n.get("version")).and_then(Value::as_str).unwrap_or("").to_string(),
        next_out: v.get("next").and_then(|n| n.get("out")).and_then(Value::as_bool).unwrap_or(false),
    })
}

/// The system underneath, in words: "Debian 13" (from /etc/os-release), or the fixture's.
pub fn base_name() -> String {
    if let Some(f) = crate::paths::fixture() {
        if let Ok(s) = std::fs::read_to_string(f.join("system-base")) {
            return s.trim().to_string();
        }
    }
    let t = std::fs::read_to_string("/etc/os-release").unwrap_or_default();
    let get = |k: &str| t.lines().find_map(|l| l.strip_prefix(&format!("{k}="))).map(|v| v.trim_matches('"').to_string()).unwrap_or_default();
    // Hearth's own os-release names Hearth; the base is in ID_LIKE / HEARTH_BASE / VERSION_CODENAME.
    let like = format!("{} {} {}", get("ID"), get("ID_LIKE"), get("HEARTH_BASE")).to_lowercase();
    let deb = std::fs::read_to_string("/etc/debian_version").map(|s| s.trim().split('.').next().unwrap_or("").to_string()).unwrap_or_default();
    if like.contains("fedora") {
        format!("Fedora {}", get("VERSION_ID")).trim().to_string()
    } else if !deb.is_empty() && deb.chars().all(|c| c.is_ascii_digit()) {
        format!("Debian {deb}")
    } else if like.contains("debian") || std::path::Path::new("/etc/debian_version").exists() {
        "Debian".into()
    } else {
        "the system".into()
    }
}

/// "Debian" from "Debian 13" (the card names: "Debian updates").
pub fn base_word(base: &str) -> String {
    base.split_whitespace().next().filter(|w| w.chars().next().map(char::is_uppercase).unwrap_or(false)).unwrap_or("System").to_string()
}

/// System updates by themselves (Library › Hearth OS's switch): off unless you turned it on.
pub fn auto_on() -> bool {
    if let Some(f) = crate::paths::fixture() {
        return f.join("system-auto").exists();
    }
    std::path::Path::new(AUTO_FILE).exists() && !std::path::Path::new(NO_AUTO_FILE).exists()
}

fn fixture_status() -> Option<PathBuf> {
    crate::paths::fixture().map(|f| f.join("system-status.json")).filter(|p| p.exists())
}

/// hearth-admin update-status (or update-check with `check`), through pkexec (the person at the device, no password:
/// 49-hearth-admin.rules). None when it can't say (no pkexec, not an administrator, no bootc).
pub fn status(check: bool) -> Option<SysState> {
    if let Some(p) = fixture_status() {
        return std::fs::read_to_string(p).ok().and_then(|t| parse_status(&t));
    }
    if !std::path::Path::new(ADMIN).exists() {
        return None;
    }
    let out = std::process::Command::new("pkexec").args([ADMIN, if check { "update-check" } else { "update-status" }]).stdin(std::process::Stdio::null()).output().ok()?;
    parse_status(&String::from_utf8_lossy(&out.stdout))
}

/// One line of hearth-admin update's progress: (percent, words, bytes done, bytes in all).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Step {
    pub pct: Option<f32>,
    pub desc: String,
    pub done: u64,
    pub total: u64,
    pub phase: String,
}

pub fn parse_line(l: &str) -> Option<Step> {
    if let Some(j) = l.strip_prefix("stage: ") {
        let v: Value = serde_json::from_str(j).ok()?;
        return Some(Step {
            pct: v.get("pct").and_then(Value::as_f64).map(|x| x as f32),
            desc: v.get("desc").and_then(Value::as_str).unwrap_or("").to_string(),
            done: v.get("done").and_then(Value::as_u64).unwrap_or(0),
            total: v.get("total").and_then(Value::as_u64).unwrap_or(0),
            phase: v.get("phase").and_then(Value::as_str).unwrap_or("").to_string(),
        });
    }
    if let Some(r) = l.strip_prefix("progress: ") {
        let (n, rest) = r.split_once(' ').unwrap_or((r, ""));
        return Some(Step { pct: n.parse().ok(), desc: rest.to_string(), ..Default::default() });
    }
    None
}

// ------------------------------------------------------------------ Done before (the Store's own short history)
#[derive(Debug, Clone, PartialEq)]
pub struct Done {
    /// "Mantel 1.18.1 (test)", "Debian updates"
    pub what: String,
    /// unix time
    pub at: i64,
}

fn history_file() -> PathBuf {
    crate::paths::data().join("hearth/store/history.json")
}

/// The last updates done (newest first). The fixture's for CI's pictures.
pub fn history() -> Vec<Done> {
    let p = crate::paths::fixture().map(|f| f.join("history.json")).filter(|p| p.exists()).unwrap_or_else(history_file);
    let v: Value = std::fs::read(p).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
    let mut out: Vec<Done> = v
        .get("done")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(|x| Some(Done { what: x.get("what")?.as_str()?.to_string(), at: x.get("at").and_then(Value::as_i64).unwrap_or(0) })).collect())
        .unwrap_or_default();
    out.sort_by(|a, b| b.at.cmp(&a.at));
    out
}

/// One more done (kept: the last 40). Written whole and atomically; a file that can't be read is never written over.
pub fn add_history(what: &str, at: i64) {
    if crate::paths::fixture().is_some() {
        return;
    }
    let p = history_file();
    let mut v: Value = match std::fs::read(&p) {
        Ok(b) => match serde_json::from_slice(&b) {
            Ok(v) => v,
            Err(_) => return,
        },
        Err(_) => serde_json::json!({"done": []}),
    };
    let Some(a) = v.get_mut("done").and_then(Value::as_array_mut) else { return };
    a.insert(0, serde_json::json!({"what": what, "at": at}));
    a.truncate(40);
    if let Some(d) = p.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let _ = crate::files::write_atomic(&p, serde_json::to_string_pretty(&v).unwrap_or_default().as_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_hearth_admins_answer() {
        let t = "log: x\n{\"booted\": {\"version\": \"1.18\", \"timestamp\": \"2026-11-21T03:00:00Z\"}, \"staged\": null, \"available\": {\"version\": \"1.18\", \"timestamp\": \"2026-11-23T03:00:00Z\"}, \"rollback\": {\"version\": \"1.17\", \"timestamp\": \"2026-11-19T03:00:00Z\"}}\n";
        let s = parse_status(t).unwrap();
        assert_eq!(s.available.unwrap().day(), "Nov 23");
        assert_eq!(s.rollback.unwrap().day(), "Nov 19");
        assert!(s.staged.is_none());
    }

    #[test]
    fn days_in_words() {
        assert_eq!(day_words("2026-11-23"), "Nov 23");
        assert_eq!(day_words("2026-01-02T00:00:00Z"), "Jan 2");
        assert_eq!(day_words(""), "");
        assert_eq!(base_word("Debian 13"), "Debian");
    }

    #[test]
    fn progress_lines() {
        let s = parse_line("stage: {\"phase\": \"download\", \"pct\": 40.0, \"done\": 100, \"total\": 250, \"desc\": \"Downloading\"}").unwrap();
        assert_eq!((s.done, s.total, s.phase.as_str()), (100, 250, "download"));
        assert_eq!(parse_line("progress: 12.5 Starting").unwrap().pct, Some(12.5));
        assert!(parse_line("log: x").is_none());
    }
}
