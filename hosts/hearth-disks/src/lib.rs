//! Hearth Disks: Hearth OS's own disk manager. See every drive with its health and a map of its partitions;
//! mount, safely remove, rename, format, make and delete partitions, check and repair, and "Use for games" (a
//! Steam library). Linux: udisks2 over D-Bus (linux.rs; polkit asks for the password when it's needed). Windows:
//! the Storage PowerShell module (winps.rs; changes run elevated, so Windows asks first).
//! model.rs is what both show; safety.rs decides what's offered and checks it again right before it runs.
//! usb/ is Make a bootable USB (Hearth OS only): download a system, check it, write it to a USB drive.

pub mod model;
pub mod safety;
pub mod udisks;
pub mod windows;
#[cfg(target_os = "linux")]
pub mod linux;
/// Hearth's drive letters (hearth_drive_letters.py), Linux only.
#[cfg(target_os = "linux")]
pub mod letters;
#[cfg(windows)]
pub mod winps;
/// Make a bootable USB (Linux only: Hearth OS).
#[cfg(target_os = "linux")]
pub mod usb;

use model::{Drive, Partition};
use safety::{Opts, Plan};
use serde_json::{json, Value};

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
    #[allow(unused_mut)]
    let mut plan = safety::plan(action, d, p, HOST);
    #[cfg(target_os = "linux")]
    if plan.refuse.is_none() { linux_extras(action, d, p, &mut plan); }
    Ok(plan)
}

/// What only Hearth OS knows for a plan: the games on what's erased (Storage's sizes and Steam's records), the
/// letters a volume can have, and who is using a volume that's about to be resized.
#[cfg(target_os = "linux")]
fn linux_extras(action: &str, d: &Drive, p: Option<&Partition>, plan: &mut Plan) {
    let erases = matches!(action, "format" | "delete" | "format_drive") || (action == "games" && plan.danger);
    if erases {
        let parts: Vec<&Partition> = match p { Some(p) => vec![p], None => d.partitions.iter().collect() };
        let (n, bytes) = parts.iter().map(|p| games_on(&p.mount_points)).fold((0, 0), |a, b| (a.0 + b.0, a.1 + b.1));
        if n > 0 { plan.games = Some(n); plan.games_bytes = Some(bytes); }
    }
    if let Some(p) = p {
        if action == "letter" { plan.letters = letters::free_for(&p.uuid, &letters::load()); }
        // A new volume (New Simple Volume, or one for games) can be given a letter no drive has.
        if p.kind == "free" && matches!(action, "create" | "games") { plan.letters = letters::free_for("", &letters::load()); }
        if matches!(action, "extend" | "shrink") {
            let users = linux::busy_with(p);
            if !users.is_empty() { plan.refuse = Some(busy_text(&users)); }
        }
    }
}

/// Why a volume can't be resized while these use it.
pub fn busy_text(users: &[String]) -> String {
    if users.iter().any(|u| u == "Steam") { return "Steam is open, and this volume holds games. Close Steam (and any game) first, then try again. Nothing was changed.".into(); }
    format!("It's in use by {}. Close {} first, then try again. Nothing was changed.", users.join(", "), if users.len() == 1 { "it" } else { "them" })
}

/// One line of a Steam app manifest (.acf): the value of `key`.
pub fn acf_field(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|l| {
        let parts: Vec<&str> = l.split('"').collect();
        (parts.len() >= 4 && parts[1].eq_ignore_ascii_case(key)).then(|| parts[3].to_string())
    })
}

/// Games on a volume mounted at `mounts`: (how many, how much space). From what Settings › Storage last added up
/// (diskspace-sizes.json: every store's games) and Steam's own records on it, each game counted once.
pub fn games_in(mounts: &[String], storage: &Value) -> (u64, u64) {
    let mounts: Vec<&String> = mounts.iter().filter(|m| !model::is_system_mount(m)).collect();
    if mounts.is_empty() { return (0, 0); }
    let under = |path: &str| mounts.iter().any(|m| std::path::Path::new(path).starts_with(m.as_str()));
    let mut seen = std::collections::HashSet::new();
    let (mut n, mut bytes) = (0u64, 0u64);
    for g in storage["games"].as_object().into_iter().flat_map(|o| o.values()) {
        let path = g["path"].as_str().unwrap_or("");
        if !path.is_empty() && under(path) && seen.insert(std::path::PathBuf::from(path)) { n += 1; bytes += g["size"].as_u64().unwrap_or(0); }
    }
    for m in &mounts {
        for apps in [std::path::Path::new(m.as_str()).join("steamapps"), std::path::Path::new(m.as_str()).join("SteamLibrary/steamapps")] {
            let Ok(rd) = std::fs::read_dir(&apps) else { continue };
            for e in rd.flatten() {
                let name = e.file_name().to_string_lossy().to_string();
                if !(name.starts_with("appmanifest_") && name.ends_with(".acf")) { continue; }
                let Ok(text) = std::fs::read_to_string(e.path()) else { continue };
                let Some(dir) = acf_field(&text, "installdir") else { continue };
                if seen.insert(apps.join("common").join(&dir)) {
                    n += 1;
                    bytes += acf_field(&text, "SizeOnDisk").and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
                }
            }
        }
    }
    (n, bytes)
}

#[cfg(target_os = "linux")]
fn games_on(mounts: &[String]) -> (u64, u64) {
    let storage = std::fs::read(hearth_core::paths::data_dir().join("diskspace-sizes.json")).ok()
        .and_then(|b| serde_json::from_slice::<Value>(&b).ok()).unwrap_or(Value::Null);
    games_in(mounts, &storage)
}

/// Properties › Health: what the drive reports (SMART's lines), beyond what `list` has.
pub fn health_details(id: &str) -> Result<Value, String> {
    let drives = list()?;
    let (d, _) = find(&drives, id).ok_or(GONE)?;
    #[cfg(target_os = "linux")]
    let lines = linux::health_lines(d);
    #[cfg(not(target_os = "linux"))]
    let lines: Vec<Value> = d.health.details.iter().map(|t| json!({"name": t, "value": "", "ok": false})).collect();
    Ok(json!({"health": d.health, "lines": lines, "alertKey": alert_key(d), "alert": alert_on(&alerts(), d), "alerts": cfg!(target_os = "linux")}))
}

/// "Tell me if this drive starts to fail" is kept per drive, by its serial number.
pub fn alert_key(d: &Drive) -> String { if d.serial.trim().is_empty() { format!("id:{}", d.id) } else { format!("serial:{}", d.serial.trim()) } }
fn alert_on(alerts: &Value, d: &Drive) -> bool { alerts[alert_key(d)] == true }

/// ~/.config/nimbus/disks.json: Hearth Disks' settings (only "failAlerts" so far; nothing is in it until you
/// turn one on).
#[cfg(target_os = "linux")]
pub fn settings_path() -> std::path::PathBuf { hearth_core::paths::config_dir().join("disks.json") }

/// The drives you asked to hear about: {key: true}. Read only.
pub fn alerts() -> Value {
    #[cfg(target_os = "linux")]
    { hearth_core::settings_file::read_json(&settings_path())["failAlerts"].clone() }
    #[cfg(not(target_os = "linux"))]
    { Value::Null }
}

/// Turns "Tell me if this drive starts to fail" on or off for one drive: only that drive's key changes
/// (settings_file: under the lock, atomic, with a .bak). Off removes it, so the file holds only what you chose.
pub fn set_alert(id: &str, on: bool) -> Result<bool, String> {
    let drives = list()?;
    let (d, _) = find(&drives, id).ok_or(GONE)?;
    let key = alert_key(d);
    #[cfg(target_os = "linux")]
    {
        hearth_core::settings_file::update_json(&settings_path(), |m| {
            let fa = m.entry("failAlerts").or_insert_with(|| json!({}));
            if !fa.is_object() { *fa = json!({}); }
            let o = fa.as_object_mut().expect("an object");
            if on { o.insert(key.clone(), json!(true)); } else { o.remove(&key); }
            if o.is_empty() { m.remove("failAlerts"); }
        }).map_err(|e| format!("Couldn't save it: {e}"))?;
        Ok(on)
    }
    #[cfg(not(target_os = "linux"))]
    { let _ = (key, on); Err("Hearth Disks can't watch drives on Windows.".into()) }
}

/// `hearth-disks --health-watch`: a desktop notification for each drive you asked about that has started to fail
/// (once per change: the last state is kept in ~/.local/state/nimbus/disks-health.json). Returns how many.
#[cfg(target_os = "linux")]
pub fn health_watch() -> Result<usize, String> {
    let on = alerts();
    if !on.is_object() { return Ok(0); }
    let drives = list()?;
    let state_dir = std::env::var_os("XDG_STATE_HOME").map(std::path::PathBuf::from)
        .unwrap_or_else(|| hearth_core::paths::home().join(".local/state")).join("nimbus");
    let file = state_dir.join("disks-health.json");
    let mut seen: serde_json::Map<String, Value> = std::fs::read(&file).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
    let before = seen.clone();
    let mut told = 0;
    for d in drives.iter().filter(|d| alert_on(&on, d)) {
        let key = alert_key(d);
        let state = d.health.state.clone();
        if state == "unknown" { continue; }
        if matches!(state.as_str(), "warn" | "bad") && seen.get(&key).and_then(Value::as_str) != Some(state.as_str()) {
            let title = if state == "bad" { format!("{} is failing", d.name()) } else { format!("{} has problems", d.name()) };
            let body = format!("{}. Copy what matters off it. Hearth Disks › Properties › Health has the details.", d.health.text);
            let _ = std::process::Command::new("notify-send").args(["-a", "Hearth Disks", "-i", "hearth-disks", title.as_str(), body.as_str()]).status();
            told += 1;
        }
        seen.insert(key, json!(state));
    }
    if seen != before {
        let _ = std::fs::create_dir_all(&state_dir);
        hearth_core::paths::write_atomic(&file, Value::Object(seen).to_string().as_bytes()).map_err(|e| e.to_string())?;
    }
    Ok(told)
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
    fn games_on_a_volume() {
        let dir = std::env::temp_dir().join(format!("hearth-disks-games-{}", std::process::id()));
        let apps = dir.join("SteamLibrary/steamapps");
        std::fs::create_dir_all(apps.join("common")).unwrap();
        std::fs::write(apps.join("appmanifest_620.acf"), "\"AppState\"\n{\n\t\"appid\"\t\t\"620\"\n\t\"installdir\"\t\t\"Portal 2\"\n\t\"SizeOnDisk\"\t\t\"12000\"\n}\n").unwrap();
        std::fs::write(apps.join("appmanifest_70.acf"), "\"AppState\" { \"installdir\" \"Half-Life\" \"SizeOnDisk\" \"500\" }").unwrap();
        let m = dir.to_string_lossy().to_string();
        // Storage knows Portal 2 (bigger, with its own count) and a GOG game; Steam's records add Half-Life.
        let storage = serde_json::json!({"games": {
            "steam:620": {"path": format!("{m}/SteamLibrary/steamapps/common/Portal 2"), "size": 13000},
            "gog:1": {"path": format!("{m}/Games/Witcher"), "size": 40000},
            "gog:2": {"path": "/elsewhere/Game", "size": 7}}});
        assert_eq!(super::games_in(&[m.clone()], &storage), (3, 13000 + 40000 + 500));
        assert_eq!(super::games_in(&[], &storage), (0, 0));
        assert_eq!(super::games_in(&["/".to_string()], &storage), (0, 0), "the system's own mounts don't count");
        assert_eq!(super::acf_field("\t\"installdir\"\t\t\"Portal 2\"", "InstallDir").as_deref(), Some("Portal 2"));
        assert!(super::busy_text(&["Steam".into()]).contains("Close Steam"));
        assert!(super::busy_text(&["vlc".into(), "bash".into()]).contains("vlc, bash"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn finds_things() {
        let d = crate::udisks::parse(&crate::udisks::tests::sample(), &crate::udisks::Ctx { uid: 1000, steam: &|_: &[String]| false, usage: &|_: &str| None, slaves: &|_: &str| vec![] });
        let (drive, part) = super::find(&d, "/org/freedesktop/UDisks2/block_devices/mmcblk0p1").unwrap();
        assert_eq!(drive.kind, "sd");
        assert_eq!(part.unwrap().label, "SWITCH");
        assert!(super::find(&d, "/org/freedesktop/UDisks2/drives/SanDisk_SD").unwrap().1.is_none());
        assert!(super::find(&d, "nope").is_none());
    }
}
