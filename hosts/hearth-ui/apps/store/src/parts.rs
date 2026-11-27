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

// ------------------------------------------------------------------ Store apps (Hearth's apps that aren't in the image)
/// Where a Store app's program would be if it were built into Hearth OS: hearth-part starts the Store's copy when
/// there's none there (it never is, for a Store app), the same way every Hearth shortcut calls it.
pub fn builtin_path(pid: &str) -> String {
    format!("/usr/libexec/hearth/apps/{pid}")
}

/// The folder of the Store's copy now in use (~/.local/share/hearth/parts/<id>/<version>).
pub fn copy_dir(pid: &str) -> Option<PathBuf> {
    let h = have(pid)?;
    Some(root().join(pid).join(h.version))
}

/// The Store app's command, as its menu entry and Open run it.
pub fn store_command(p: &Part) -> Vec<String> {
    vec![helper().to_string_lossy().to_string(), "run".into(), p.id.clone(), builtin_path(&p.id)]
}

/// Opens a Store app (its Store copy, through hearth-part), not waiting for it.
pub fn store_launch(p: &Part) -> bool {
    let c = store_command(p);
    std::process::Command::new(&c[0]).args(&c[1..]).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn().is_ok()
}

fn bin_dir() -> PathBuf {
    home().join(".local/bin")
}
const MARK: &str = "# Hearth Store: a Hearth app from the Store";

/// The menu entry made from the one that came in the Store copy (share/applications/*.desktop) when it has one, its
/// Exec pointed at the Store copy (through hearth-part); else one made from the list. `icon`: the icon's path.
pub fn store_desktop(e: &crate::catalog::Entry, p: &Part, shipped: Option<&str>, icon: &str) -> String {
    let cmd = store_command(p).join(" ");
    let one = |s: &str| s.replace(['\n', '\r'], " ");
    let mut out = String::new();
    if let Some(d) = shipped.filter(|d| d.contains("[Desktop Entry]")) {
        let mut in_main = false;
        for l in d.lines() {
            let t = l.trim();
            if t.starts_with('[') {
                in_main = t == "[Desktop Entry]";
                if !in_main {
                    // (other actions run other programs: left out)
                    break;
                }
                out.push_str(l);
                out.push('\n');
                continue;
            }
            if !in_main || t.starts_with("TryExec=") || t.starts_with("X-Hearth-Store=") || t.starts_with("Actions=") || t.starts_with("DBusActivatable=") {
                continue;
            }
            if let Some(x) = t.strip_prefix("Exec=") {
                // the program (its first word) becomes the Store copy's start; its arguments stay
                let rest: Vec<&str> = x.split_whitespace().skip(1).collect();
                out.push_str(&format!("Exec={cmd}{}{}\n", if rest.is_empty() { "" } else { " " }, rest.join(" ")));
            } else if t.starts_with("Icon=") {
                out.push_str(&format!("Icon={icon}\n"));
            } else {
                out.push_str(l);
                out.push('\n');
            }
        }
        if !out.contains("\nExec=") {
            out.push_str(&format!("Exec={cmd}\n"));
        }
        if !out.contains("\nIcon=") {
            out.push_str(&format!("Icon={icon}\n"));
        }
    } else {
        out = format!(
            "[Desktop Entry]\nType=Application\nName={}\nComment={}\nExec={cmd}\nIcon={icon}\nCategories=Utility;\nTerminal=false\n",
            one(&e.name),
            one(if e.tagline.is_empty() { &e.card } else { &e.tagline })
        );
    }
    out.push_str(&format!("X-Hearth-Store={}\n", e.id));
    out
}

/// After Get (and each update): the app's menu entry, its icon and its command (~/.local/bin/<program>, so anything
/// that starts it by name finds the Store's copy: red flag 13), from what came in its Store copy.
pub fn store_menu(e: &crate::catalog::Entry) -> std::io::Result<()> {
    let Some(p) = e.part.as_ref() else { return Ok(()) };
    let dir = copy_dir(&p.id);
    let first = |sub: &str, ext: &str| -> Option<PathBuf> {
        let d = dir.as_ref()?.join(sub);
        let mut v: Vec<PathBuf> = std::fs::read_dir(d).ok()?.flatten().map(|x| x.path()).filter(|x| x.extension().map(|y| y == ext).unwrap_or(false)).collect();
        v.sort();
        v.into_iter().next()
    };
    let icon = crate::paths::icon_file(&e.id);
    match first("share/icons/hicolor/scalable/apps", "svg").and_then(|f| std::fs::read(f).ok()) {
        Some(b) => crate::files::write_atomic(&icon, &b)?,
        None => crate::files::write_atomic(&icon, crate::files::icon_svg(e).as_bytes())?,
    }
    let shipped = first("share/applications", "desktop").and_then(|f| std::fs::read_to_string(f).ok());
    crate::files::write_atomic(&crate::paths::desktop_file(&e.id), store_desktop(e, p, shipped.as_deref(), &icon.to_string_lossy()).as_bytes())?;
    let prog = e.program();
    if !prog.is_empty() && !prog.starts_with('.') && prog.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        let b = bin_dir().join(&prog);
        let ours = std::fs::read_to_string(&b).map(|t| t.contains(MARK)).unwrap_or(true);
        if ours {
            let c = store_command(p);
            crate::files::write_atomic(&b, format!("#!/bin/sh\n{MARK} ({}); the Store removes it with the app.\nexec {} {} {} {} \"$@\"\n", e.name, c[0], c[1], c[2], c[3]).as_bytes())?;
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&b, std::fs::Permissions::from_mode(0o755))?;
        }
    }
    Ok(())
}

/// Uninstall a Store app: its Store copy to the Trash (hearth-part remove), its menu entry, icon and command gone; with
/// `keep` its own folders stay, else they go to the Trash too. Returns what of its own went to the Trash.
pub fn store_uninstall(e: &crate::catalog::Entry, keep: bool) -> Result<Vec<PathBuf>, String> {
    let Some(p) = e.part.as_ref() else { return Ok(Vec::new()) };
    remove(&p.id)?;
    let _ = std::fs::remove_file(crate::paths::desktop_file(&e.id));
    let _ = std::fs::remove_file(crate::paths::icon_file(&e.id));
    let prog = e.program();
    if !prog.is_empty() && !prog.contains('/') {
        let b = bin_dir().join(&prog);
        if std::fs::read_to_string(&b).map(|t| t.contains(MARK)).unwrap_or(false) {
            let _ = std::fs::remove_file(&b);
        }
    }
    let mut moved = Vec::new();
    if !keep {
        for d in crate::files::data_dirs(e) {
            moved.push(crate::files::trash(&d).map_err(|x| x.to_string())?);
        }
    }
    Ok(moved)
}

/// A Store app's update: once you've got it, made for this Hearth OS, another version than yours (its own number:
/// there's no built-in one to compare with).
pub fn store_offer<'a>(p: &'a Part, h: Option<&Have>, hearth: &str) -> Option<&'a Part> {
    let h = h?;
    (fits(&p.os, hearth) && h.version != p.version && (crate::newer(&p.version, &h.version) || !fits(&h.os, hearth))).then_some(p)
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
        Part { id: "hearth-files-app".into(), version: v.into(), os: os.into(), path: "hearth-files-app".into(), url: "https://x/y".into(), sha256: "0".repeat(64), size: 1, notes: vec![], missing: vec![], system: false }
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
    fn a_store_app_updates_only_once_its_got() {
        let p = part("1.18.2", "1.18");
        assert!(store_offer(&p, None, "1.18").is_none(), "not got: Get, never an update");
        let h = Have { version: "1.18.1".into(), os: "1.18".into(), ..Default::default() };
        assert!(store_offer(&p, Some(&h), "1.18").is_some());
        let same = Have { version: "1.18.2".into(), os: "1.18".into(), ..Default::default() };
        assert!(store_offer(&p, Some(&same), "1.18").is_none());
        assert!(store_offer(&p, Some(&h), "1.19").is_none(), "not made for this Hearth OS");
    }

    #[test]
    fn a_store_apps_menu_entry_runs_its_store_copy() {
        let e = crate::catalog::Entry::from_json(&serde_json::json!({"id": "forge", "name": "Hearth Forge", "kind": "app", "open": "hearth-forge-app",
            "part": {"id": "hearth-forge-app", "version": "1.18.1", "os": "1.18", "path": "hearth-forge-app", "url": "https://x/y", "sha256": "0".repeat(64)}}), false).unwrap();
        let p = e.part.clone().unwrap();
        let shipped = "[Desktop Entry]\nType=Application\nName=Hearth Forge\nExec=/usr/bin/hearth-forge-app %f\nTryExec=/usr/bin/hearth-forge-app\nIcon=hearth-forge\nCategories=Graphics;\n\n[Desktop Action x]\nExec=/usr/bin/other\n";
        let d = store_desktop(&e, &p, Some(shipped), "/h/icon.svg");
        assert!(d.contains("\nExec=/usr/libexec/hearth/hearth-part run hearth-forge-app /usr/libexec/hearth/apps/hearth-forge-app %f\n"), "{d}");
        assert!(!d.contains("TryExec") && !d.contains("/usr/bin/other"));
        assert!(d.contains("\nIcon=/h/icon.svg\n") && d.contains("Categories=Graphics;") && d.contains("X-Hearth-Store=forge"));
        let made = store_desktop(&e, &p, None, "/h/icon.svg");
        assert!(made.starts_with("[Desktop Entry]") && made.contains("Name=Hearth Forge") && made.contains("Exec=/usr/libexec/hearth/hearth-part run hearth-forge-app"));
    }

    /// Got (the Store copy as hearth-part leaves it), its menu entry, icon and command; then uninstalled with the real
    /// hearth-part: the copy goes to the Trash, the entry, icon and command go, its own folders stay.
    #[test]
    fn a_store_app_got_then_uninstalled() {
        let d = tempfile::tempdir().unwrap();
        let h = d.path();
        for (k, v) in [("HOME", h.to_path_buf()), ("XDG_DATA_HOME", h.join(".local/share")), ("XDG_CONFIG_HOME", h.join(".config")), ("XDG_CACHE_HOME", h.join(".cache")), ("HEARTH_PARTS", h.join(".local/share/hearth/parts")), ("HEARTH_PARTS_STATE", h.join(".local/state/hearth/parts"))] {
            std::env::set_var(k, v);
        }
        let e = crate::catalog::Entry::from_json(&serde_json::json!({"id": "forge", "name": "Hearth Forge", "kind": "app", "open": "hearth-forge-app", "data": ["~/.config/hearth-forge"],
            "part": {"id": "hearth-forge-app", "version": "1.18.1", "os": "1.18", "path": "hearth-forge-app", "url": "https://x/y", "sha256": "0".repeat(64)}}), false).unwrap();
        assert!(have("hearth-forge-app").is_none());
        let v = h.join(".local/share/hearth/parts/hearth-forge-app/1.18.1");
        std::fs::create_dir_all(v.join("share/applications")).unwrap();
        std::fs::create_dir_all(v.join("share/icons/hicolor/scalable/apps")).unwrap();
        std::fs::write(v.join("hearth-forge-app"), b"#!/bin/sh\n").unwrap();
        std::fs::write(v.join("share/applications/hearth-forge.desktop"), "[Desktop Entry]\nName=Hearth Forge\nExec=/usr/bin/hearth-forge-app\nIcon=hearth-forge\n").unwrap();
        std::fs::write(v.join("share/icons/hicolor/scalable/apps/hearth-forge.svg"), "<svg>forge</svg>").unwrap();
        std::fs::write(h.join(".local/share/hearth/parts/hearth-forge-app/part.json"), r#"{"id": "hearth-forge-app", "version": "1.18.1", "os": "1.18", "path": "hearth-forge-app", "installed": 1}"#).unwrap();
        std::fs::create_dir_all(h.join(".config/hearth-forge")).unwrap();
        assert_eq!(have("hearth-forge-app").map(|x| x.version), Some("1.18.1".into()));
        store_menu(&e).unwrap();
        let desk = std::fs::read_to_string(crate::paths::desktop_file("forge")).unwrap();
        assert!(desk.contains("Exec=/usr/libexec/hearth/hearth-part run hearth-forge-app"), "{desk}");
        assert_eq!(std::fs::read_to_string(crate::paths::icon_file("forge")).unwrap(), "<svg>forge</svg>", "the icon that came with it");
        let bin = std::fs::read_to_string(h.join(".local/bin/hearth-forge-app")).unwrap();
        assert!(bin.contains(MARK) && bin.contains("run hearth-forge-app"));
        // someone's own command of that name is never replaced
        std::fs::write(h.join(".local/bin/hearth-forge-app"), "#!/bin/sh\necho mine\n").unwrap();
        store_menu(&e).unwrap();
        assert!(std::fs::read_to_string(h.join(".local/bin/hearth-forge-app")).unwrap().contains("mine"));
        std::fs::remove_file(h.join(".local/bin/hearth-forge-app")).unwrap();
        store_menu(&e).unwrap();
        let hp = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../os/system_files/usr/libexec/hearth/hearth-part");
        if hp.exists() && std::process::Command::new("python3").arg("--version").output().is_ok() {
            std::env::set_var("HEARTH_PART", &hp);
            store_uninstall(&e, true).unwrap();
            std::env::remove_var("HEARTH_PART");
            assert!(have("hearth-forge-app").is_none(), "its Store copy is gone");
            assert!(!crate::paths::desktop_file("forge").exists() && !crate::paths::icon_file("forge").exists());
            assert!(!h.join(".local/bin/hearth-forge-app").exists());
            assert!(h.join(".config/hearth-forge").exists(), "its own folders stay by default");
        }
        std::env::remove_var("HEARTH_PARTS");
        std::env::remove_var("HEARTH_PARTS_STATE");
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
