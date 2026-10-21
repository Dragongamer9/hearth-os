//! Moving to a new Hearth OS from the Bazzite-based one, as a clean install that keeps your games (Settings ›
//! Updates): Hearth OS on Anatase (recommended), or one of the Beta Experimental editions (Ubuntu, CachyOS,
//! Peppermint). Afterwards, on the new system, the files that were put aside (~/.old-hearth-<date>: Settings ›
//! System, "Delete now" / "Restore a folder"; deleted after 30 days).
//! The move itself is hearth-admin move-to-new-os --edition NAME (root, through pkexec; the edition picks the image
//! from a fixed list there too); the first start of the new system (hearth-clean-install, in every edition's image)
//! does the clean-up and writes the list of what it put aside.

use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

pub const IMAGE: &str = "ghcr.io/dragongamer9/hearth-os:latest";
const BAZZITE_INFO: &str = "/usr/share/ublue-os/image-info.json";
const OWN_INFO: &str = "/usr/share/nimbus/image-info.json";
pub const OLD_PREFIX: &str = ".old-hearth-";
pub const MANIFEST: &str = ".hearth-old-files.json";

/// One Hearth OS the old one can move to. `tag` is what's offered (:testing for the Beta Experimental editions,
/// which have nothing else); hearth-admin has the same list (NEW_OS_EDITIONS).
pub struct Edition { pub id: &'static str, pub label: &'static str, pub repo: &'static str, pub tag: &'static str, pub beta: bool, pub about: &'static str }

pub const EDITIONS: &[Edition] = &[
    Edition { id: "anatase", label: "Hearth OS on Anatase", repo: "dragongamer9/hearth-os", tag: "latest", beta: false,
              about: "The next Hearth OS. A smaller system that starts and updates faster, the same for every PC and handheld." },
    Edition { id: "ubuntu", label: "Ubuntu", repo: "dragongamer9/hearth-ubuntu", tag: "testing", beta: true,
              about: "Hearth OS built on Ubuntu. Still being tested: some things may not work yet." },
    Edition { id: "cachyos", label: "CachyOS", repo: "dragongamer9/hearth-cachyos", tag: "testing", beta: true,
              about: "Hearth OS built on CachyOS, tuned for newer processors. Still being tested: some things may not work yet." },
    Edition { id: "peppermint", label: "Peppermint", repo: "dragongamer9/hearth-peppermint", tag: "testing", beta: true,
              about: "Hearth OS built on Peppermint OS (Debian). Still being tested: some things may not work yet." },
];

pub fn edition(id: &str) -> Option<&'static Edition> { EDITIONS.iter().find(|e| e.id == id) }

/// The tag used: :testing when asked for (or the only one there is), else the edition's own.
pub fn tag_for(e: &Edition, testing: bool) -> &'static str { if testing { "testing" } else { e.tag } }

pub fn image_of(e: &Edition, testing: bool) -> String { format!("ghcr.io/{}:{}", e.repo, tag_for(e, testing)) }

/// The Bazzite-based Hearth OS (the one the move is for): Bazzite's image info, not a newer edition's own.
/// This system's edition (its image info's "base"), when it's one of the new editions.
pub fn current_at(root: &Path) -> Option<&'static str> {
    let v: Value = std::fs::read(root.join(OWN_INFO.trim_start_matches('/'))).ok().and_then(|b| serde_json::from_slice(&b).ok())?;
    let base = v["base"].as_str()?;
    EDITIONS.iter().find(|e| e.id == base).map(|e| e.id)
}
pub fn current() -> Option<&'static str> { current_at(Path::new("/")) }

/// Whether a move is offered: from the Bazzite-based Hearth to any edition, or from one edition to another
/// ("Switch edition": the same clean install that keeps your games).
pub fn eligible_at(root: &Path) -> bool {
    if !root.join("run/ostree-booted").exists() { return false; }
    let own = root.join(OWN_INFO.trim_start_matches('/')).exists();
    (root.join(BAZZITE_INFO.trim_start_matches('/')).exists() && !own) || current_at(root).is_some()
}
pub fn eligible() -> bool { eligible_at(Path::new("/")) }

fn state() -> &'static Arc<Mutex<Value>> {
    static S: OnceLock<Arc<Mutex<Value>>> = OnceLock::new();
    S.get_or_init(|| Arc::new(Mutex::new(json!({"state": "idle", "line": "", "progress": null, "error": null, "stage": null, "phases": null, "result": null, "edition": null}))))
}

fn published_cache() -> &'static Mutex<HashMap<String, (std::time::Instant, bool)>> {
    static CACHE: OnceLock<Mutex<HashMap<String, (std::time::Instant, bool)>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// What's remembered about repo:tag (asked less than ten minutes ago), without asking.
pub fn published_cached(repo: &str, tag: &str) -> Option<bool> {
    let key = format!("{repo}:{tag}");
    let c = published_cache().lock().unwrap();
    let v = c.get(&key).filter(|(t, _)| t.elapsed().as_secs() < 600).map(|(_, v)| *v);
    v
}

/// Is this image (e.g. "dragongamer9/hearth-os", "latest") published on GitHub's registry? Asked anonymously, the
/// way bootc does; remembered for ten minutes per image. None: couldn't tell (offline).
pub fn published(repo: &str, tag: &str) -> Option<bool> {
    if let Some(v) = published_cached(repo, tag) { return Some(v); }
    let tok = crate::http::get_json(&format!("https://ghcr.io/token?scope=repository:{repo}:pull"), &[]).ok()?;
    let token = tok["token"].as_str()?.to_string();
    let auth = format!("Bearer {token}");
    let r = crate::http::get(&format!("https://ghcr.io/v2/{repo}/manifests/{tag}"), &[("Authorization", &auth),
        ("Accept", "application/vnd.oci.image.index.v1+json, application/vnd.oci.image.manifest.v1+json, application/vnd.docker.distribution.manifest.v2+json, application/vnd.docker.distribution.manifest.list.v2+json")]).ok()?;
    if r.status != 200 && r.status != 404 && r.status != 401 && r.status != 403 { return None; }
    let v = r.ok();
    published_cache().lock().unwrap().insert(format!("{repo}:{tag}"), (std::time::Instant::now(), v));
    Some(v)
}

/// CachyOS' image is built for x86-64-v3: every processor listed in /proc/cpuinfo has these features.
pub fn x86_64_v3(cpuinfo: &str) -> bool {
    let flags: Vec<Vec<&str>> = cpuinfo.lines()
        .filter_map(|l| l.split_once(':').filter(|(k, _)| k.trim() == "flags").map(|(_, v)| v.split_whitespace().collect()))
        .collect();
    !flags.is_empty() && flags.iter().all(|f| ["avx2", "bmi2", "fma", "movbe"].iter().all(|x| f.contains(x)))
}
pub fn cpu_v3() -> bool { x86_64_v3(&std::fs::read_to_string("/proc/cpuinfo").unwrap_or_default()) }

/// `mokutil --sb-state`'s answer ("SecureBoot enabled" / "SecureBoot disabled"). None: it didn't say.
pub fn secure_boot_mokutil(out: &str) -> Option<bool> {
    let o = out.to_lowercase();
    if o.contains("secureboot enabled") { Some(true) } else if o.contains("secureboot disabled") { Some(false) } else { None }
}
/// The SecureBoot EFI variable: 4 bytes of attributes, then 1 when it's on.
pub fn secure_boot_efivar(bytes: &[u8]) -> Option<bool> { bytes.get(4).map(|b| *b == 1) }
const SB_VAR: &str = "/sys/firmware/efi/efivars/SecureBoot-8be4df61-93ca-11d0-aa4b-00e098032b8c";
/// Is Secure Boot on? mokutil first, then the EFI variable. None: couldn't tell (not a UEFI PC, or neither works).
pub fn secure_boot() -> Option<bool> {
    let out = crate::system::run_timeout("mokutil", &["--sb-state"], std::time::Duration::from_secs(5)).unwrap_or_else(|e| e);
    secure_boot_mokutil(&out).or_else(|| std::fs::read(SB_VAR).ok().and_then(|b| secure_boot_efivar(&b)))
}

/// One choice for the picker: whether it can be picked now, and if not, why ("Not ready yet": its image isn't
/// published; CachyOS on a processor without x86-64-v3); a warning to show before moving (Secure Boot is on and
/// the edition's kernel isn't signed for Fedora's boot loader, which the move keeps).
pub fn choice(e: &Edition, published: Option<bool>, v3: bool, secure_boot: Option<bool>) -> Value { choice_from(e, published, v3, secure_boot, None) }

/// `from`: the edition this system is (switching editions), or None (the Bazzite-based Hearth, Fedora's boot loader).
pub fn choice_from(e: &Edition, published: Option<bool>, v3: bool, secure_boot: Option<bool>, from: Option<&str>) -> Value {
    let cpu_ok = e.id != "cachyos" || v3;
    let reason = if !cpu_ok { Some("Needs a newer processor") }
        else { match published { Some(true) => None, Some(false) => Some("Not ready yet"), None => Some("Couldn't check (no connection?)") } };
    // Moving keeps this system's (Fedora's) boot loader, which with Secure Boot on only starts Fedora-signed
    // kernels: Ubuntu's and Debian's are signed by their own keys, and CachyOS's aren't signed at all.
    // Switching from an edition that isn't Fedora-based keeps that edition's boot loader, which with Secure Boot on
    // doesn't start another family's kernels either.
    let other_loader = from.map(|f| f != "anatase").unwrap_or(false);
    let warning = if (e.id != "anatase" || other_loader) && secure_boot == Some(true) {
        Some(if e.id == "cachyos" {
            "Secure Boot is on, and Hearth OS on CachyOS can't start with it on. Turn Secure Boot off in your PC's firmware settings (UEFI) before you restart, or choose another one."
        } else {
            "Secure Boot is on, and after the move this edition probably won't start with it on. Turn Secure Boot off in your PC's firmware settings (UEFI) before you restart, or choose another one."
        })
    } else { None };
    json!({"id": e.id, "label": e.label, "about": e.about, "image": image_of(e, false), "beta": e.beta, "recommended": !e.beta,
           "published": published, "allowed": reason.is_none(), "reason": reason, "warning": warning,
           "cpuNote": if e.id == "cachyos" && !v3 { json!("This PC's processor doesn't have the features CachyOS is built for (x86-64-v3: AVX2, BMI2, FMA and MOVBE).") } else { Value::Null }})
}

/// How the move is going (sent with every change while it runs), without the choices.
fn progress_status() -> Value {
    let mut s = state().lock().unwrap().clone();
    s["eligible"] = json!(eligible());
    // Switching editions (this is already one of them): which one it is, so it isn't offered.
    s["current"] = json!(current());
    s["switch"] = json!(current().is_some());
    s["image"] = json!(IMAGE);
    s["canAdmin"] = json!(crate::admin::can_admin());
    s
}

/// For Settings › Updates: whether to offer the move, the choices (each image asked about at once), and how it's going.
pub fn status() -> Value {
    let mut s = progress_status();
    if s["eligible"] != true { s["published"] = Value::Null; return s; }
    let pubs: Vec<Option<bool>> = std::thread::scope(|sc| {
        let hs: Vec<_> = EDITIONS.iter().map(|e| sc.spawn(move || published(e.repo, tag_for(e, false)))).collect();
        hs.into_iter().map(|h| h.join().unwrap_or(None)).collect()
    });
    let (v3, sb) = (cpu_v3(), secure_boot());
    s["published"] = json!(pubs[0]);
    let cur = current();
    s["editions"] = Value::Array(EDITIONS.iter().zip(pubs).filter(|(e, _)| Some(e.id) != cur).map(|(e, p)| choice_from(e, p, v3, sb, cur)).collect());
    s
}

/// One line of hearth-admin's output into the state the page shows. False when the line isn't progress.
pub fn apply_line(s: &mut Value, line: &str) -> bool {
    if let Some(l) = line.strip_prefix("log: ") { s["line"] = json!(l); }
    else if let Some(p) = line.strip_prefix("progress: ") {
        let (pct, what) = p.split_once(' ').unwrap_or((p, ""));
        s["progress"] = json!(pct.parse::<f64>().unwrap_or(0.0) / 100.0);
        if !what.is_empty() { s["line"] = json!(what); }
    } else if let Some(st) = line.strip_prefix("stage: ").and_then(|j| serde_json::from_str::<Value>(j).ok()) {
        if let Some(ph) = st["phase"].as_str() { if s["phases"].is_object() { s["phases"][ph] = st.clone(); } }
        s["stage"] = st;
    } else { return false; }
    true
}

/// hearth-admin's arguments for moving to this edition.
pub fn admin_args(e: &Edition, testing: bool) -> Vec<&'static str> {
    let mut a = vec!["move-to-new-os", "--edition", e.id];
    if testing { a.push("--testing"); }
    a
}

/// Starts the move to `edition` (download and stage the new system) in the background; `emit` gets every change
/// as the event "newOs". The new system starts at the next restart.
pub fn start(emit: crate::system::bt_scan::Emit, edition_id: &str, testing: bool) -> Result<Value, String> {
    if !eligible() { return Err("Hearth OS can't switch from here.".into()); }
    let e = edition(edition_id).ok_or("Unknown choice")?;
    if current() == Some(e.id) { return Err(format!("This is already {}.", e.label)); }
    if e.id == "cachyos" && !cpu_v3() { return Err("Hearth OS on CachyOS needs a newer processor.".into()); }
    if published_cached(e.repo, tag_for(e, testing)) == Some(false) { return Err(format!("{} isn't ready yet.", e.label)); }
    {
        let mut s = state().lock().unwrap();
        if s["state"] == "moving" { drop(s); return Ok(progress_status()); }
        *s = json!({"state": "moving", "line": "Starting…", "progress": 0, "error": null, "stage": null, "result": null,
                    "edition": e.id, "label": e.label, "phases": {"download": {"pct": 0}, "install": {"pct": 0}}});
    }
    let st = state().clone();
    let args = admin_args(e, testing);
    std::thread::spawn(move || {
        let push = |emit: &crate::system::bt_scan::Emit| emit(json!({"type": "event", "name": "newOs", "data": progress_status()}));
        let mut last = std::time::Instant::now() - std::time::Duration::from_secs(1);
        let r = crate::admin::run(&args, None, &mut |line: &str| {
            let changed = apply_line(&mut st.lock().unwrap(), line);
            if changed && last.elapsed().as_millis() >= 500 { last = std::time::Instant::now(); push(&emit); }
        });
        {
            let mut s = st.lock().unwrap();
            s["progress"] = Value::Null;
            match r {
                Ok(v) => { s["state"] = json!("done"); s["result"] = v; s["line"] = json!("Restart to start the new Hearth OS."); }
                Err(e) => { s["state"] = json!("idle"); s["error"] = json!(e); }
            }
        }
        push(&emit);
    });
    Ok(progress_status())
}

// ------------------------------------------------------------------ the old files, on the new system

/// "~/.old-hearth-2026-10-19" (or with "-2" after it): the date it was made.
pub fn old_dir_date(name: &str) -> Option<&str> {
    let rest = name.strip_prefix(OLD_PREFIX)?;
    let date = rest.get(..10)?;
    let ok = date.len() == 10 && date.chars().enumerate().all(|(i, c)| if i == 4 || i == 7 { c == '-' } else { c.is_ascii_digit() });
    let tail = &rest[10..];
    if ok && (tail.is_empty() || (tail.starts_with('-') && tail.len() <= 4 && tail[1..].chars().all(|c| c.is_ascii_digit()))) { Some(date) } else { None }
}

/// A moved item's name as the manifest has it ("Documents", ".config/foo"): relative, no "..", no empty parts.
pub fn safe_rel(rel: &str) -> bool {
    !rel.is_empty() && !rel.starts_with('/') && rel.len() < 4096 && rel.split('/').all(|p| !p.is_empty() && p != "." && p != "..")
}

/// Where a restored item goes: its old place, or "<name> (restored)" / "(restored 2)"... beside it when
/// something new is there now.
pub fn restore_target(home: &Path, rel: &str) -> PathBuf {
    let want = home.join(rel);
    if !want.exists() && std::fs::symlink_metadata(&want).is_err() { return want; }
    let name = want.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    for i in 1..100 {
        let n = if i == 1 { format!("{name} (restored)") } else { format!("{name} (restored {i})") };
        let p = want.with_file_name(n);
        if std::fs::symlink_metadata(&p).is_err() { return p; }
    }
    want.with_file_name(format!("{name} (restored {})", crate::library::now_ms()))
}

fn read_manifest(dir: &Path) -> Value { std::fs::read(dir.join(MANIFEST)).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(json!({})) }

/// The old-files folders in this home: [{name, date, created, deleteAfter, daysLeft, moved: [...], expired}].
pub fn list_in(home: &Path, now_s: u64) -> Vec<Value> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(home).into_iter().flatten().flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        let Some(date) = old_dir_date(&name) else { continue };
        if !e.file_type().map(|t| t.is_dir()).unwrap_or(false) { continue; }
        let m = read_manifest(&e.path());
        let created = m["created"].as_u64().unwrap_or(0);
        let delete_after = m["deleteAfter"].as_u64().unwrap_or(if created > 0 { created + 30 * 86400 } else { 0 });
        let moved: Vec<Value> = m["moved"].as_array().cloned().unwrap_or_default().into_iter()
            .filter(|v| v.as_str().map(|r| safe_rel(r) && e.path().join(r).symlink_metadata().is_ok()).unwrap_or(false)).collect();
        let left = if delete_after > now_s { (delete_after - now_s).div_ceil(86400) } else { 0 };
        out.push(json!({"name": name, "date": date, "created": created, "deleteAfter": delete_after, "daysLeft": left,
                        "expired": delete_after > 0 && delete_after <= now_s, "moved": moved, "from": m["from"]}));
    }
    out.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
    out
}

fn now_s() -> u64 { crate::library::now_ms() / 1000 }

/// For Settings › System. Folders past their 30 days are deleted here (in the background).
pub fn old_files_status() -> Value {
    let home = crate::paths::home();
    let list = list_in(&home, now_s());
    for f in list.iter().filter(|f| f["expired"] == true) {
        if let Some(n) = f["name"].as_str() {
            let p = home.join(n);
            crate::bridge::log(&format!("old files: {n} is past its 30 days, deleting it"));
            std::thread::spawn(move || { let _ = std::fs::remove_dir_all(p); });
        }
    }
    json!({"folders": list.into_iter().filter(|f| f["expired"] != true).collect::<Vec<_>>()})
}

fn checked_dir(name: &str) -> Result<PathBuf, String> {
    if old_dir_date(name).is_none() { return Err("Not a folder of old files".into()); }
    let p = crate::paths::home().join(name);
    if !p.is_dir() { return Err("That folder isn't there any more".into()); }
    Ok(p)
}

/// "Delete now": the whole folder (can take a while for a big one).
pub fn delete(name: &str) -> Result<Value, String> {
    let p = checked_dir(name)?;
    std::fs::remove_dir_all(&p).map_err(|e| format!("Couldn't delete it: {e}"))?;
    Ok(old_files_status())
}

/// "Restore a folder": one item back where it was (or beside it). Answers with where it went.
pub fn restore(name: &str, item: &str) -> Result<Value, String> {
    let dir = checked_dir(name)?;
    if !safe_rel(item) { return Err("Unknown item".into()); }
    let mut m = read_manifest(&dir);
    let listed = m["moved"].as_array().map(|a| a.iter().any(|v| v == item)).unwrap_or(false);
    let src = dir.join(item);
    if !listed || src.symlink_metadata().is_err() { return Err("That isn't in the old files any more".into()); }
    let home = crate::paths::home();
    let dst = restore_target(&home, item);
    if let Some(parent) = dst.parent() { std::fs::create_dir_all(parent).map_err(|e| e.to_string())?; }
    std::fs::rename(&src, &dst).map_err(|e| format!("Couldn't move it back: {e}"))?;
    if let Some(a) = m["moved"].as_array_mut() { a.retain(|v| v != item); }
    let _ = crate::paths::write_atomic(&dir.join(MANIFEST), serde_json::to_string_pretty(&m).unwrap_or_default().as_bytes());
    let mut st = old_files_status();
    st["restored"] = json!(dst.to_string_lossy());
    Ok(st)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn knows_old_folders() {
        assert_eq!(old_dir_date(".old-hearth-2026-10-19"), Some("2026-10-19"));
        assert_eq!(old_dir_date(".old-hearth-2026-10-19-2"), Some("2026-10-19"));
        assert_eq!(old_dir_date(".old-hearth-2026-1-19"), None);
        assert_eq!(old_dir_date(".old-hearth-2026-10-19x"), None);
        assert_eq!(old_dir_date("Documents"), None);
        assert!(safe_rel("Documents"));
        assert!(safe_rel(".config/foo"));
        assert!(!safe_rel("../etc"));
        assert!(!safe_rel("/etc"));
        assert!(!safe_rel("a//b"));
        assert!(!safe_rel(""));
    }
    #[test]
    fn lists_and_restores_beside() {
        let home = tempfile::tempdir().unwrap();
        let old = home.path().join(".old-hearth-2026-10-19");
        std::fs::create_dir_all(old.join("Documents")).unwrap();
        std::fs::create_dir_all(old.join(".config/foo")).unwrap();
        std::fs::write(old.join(MANIFEST), json!({"created": 1000, "deleteAfter": 1000 + 30 * 86400, "moved": ["Documents", ".config/foo", "../x", "Gone"]}).to_string()).unwrap();
        let l = list_in(home.path(), 1000 + 86400);
        assert_eq!(l.len(), 1);
        assert_eq!(l[0]["moved"], json!(["Documents", ".config/foo"]));
        assert_eq!(l[0]["daysLeft"], 29);
        assert_eq!(l[0]["expired"], false);
        assert_eq!(list_in(home.path(), 1000 + 31 * 86400)[0]["expired"], true);
        // Something new where it was: restored beside it.
        assert_eq!(restore_target(home.path(), "Documents"), home.path().join("Documents"));
        std::fs::create_dir_all(home.path().join("Documents")).unwrap();
        assert_eq!(restore_target(home.path(), "Documents"), home.path().join("Documents (restored)"));
        std::fs::create_dir_all(home.path().join("Documents (restored)")).unwrap();
        assert_eq!(restore_target(home.path(), "Documents"), home.path().join("Documents (restored 2)"));
        assert_eq!(restore_target(home.path(), ".config/foo"), home.path().join(".config/foo"));
    }
    #[test]
    fn reads_progress_lines() {
        let mut s = json!({"phases": {"download": {"pct": 0}}});
        assert!(apply_line(&mut s, "progress: 42.5 Downloading (1.00 of 2.00 GB)"));
        assert_eq!(s["progress"], 0.425);
        assert_eq!(s["line"], "Downloading (1.00 of 2.00 GB)");
        assert!(apply_line(&mut s, "stage: {\"phase\": \"download\", \"pct\": 50}"));
        assert_eq!(s["phases"]["download"]["pct"], 50);
        assert!(apply_line(&mut s, "log: hello"));
        assert_eq!(s["line"], "hello");
        assert!(!apply_line(&mut s, "{\"moved\": true}"));
    }
    #[test]
    fn choices() {
        assert_eq!(EDITIONS.len(), 4);
        let a = edition("anatase").unwrap();
        assert_eq!(image_of(a, false), "ghcr.io/dragongamer9/hearth-os:latest");
        assert_eq!(image_of(a, true), "ghcr.io/dragongamer9/hearth-os:testing");
        let c = edition("cachyos").unwrap();
        assert_eq!(image_of(c, false), "ghcr.io/dragongamer9/hearth-cachyos:testing");
        assert_eq!(image_of(edition("ubuntu").unwrap(), false), "ghcr.io/dragongamer9/hearth-ubuntu:testing");
        assert_eq!(image_of(edition("peppermint").unwrap(), false), "ghcr.io/dragongamer9/hearth-peppermint:testing");
        assert!(edition("ghcr.io/evil/image:latest").is_none());
        assert_eq!(admin_args(c, false), vec!["move-to-new-os", "--edition", "cachyos"]);
        assert_eq!(admin_args(a, true), vec!["move-to-new-os", "--edition", "anatase", "--testing"]);
        // Published or not, the processor, Secure Boot.
        let v = choice(a, Some(true), false, Some(true));
        assert_eq!((v["allowed"].clone(), v["recommended"].clone(), v["warning"].clone()), (json!(true), json!(true), Value::Null));
        assert_eq!(choice(a, Some(false), true, None)["reason"], "Not ready yet");
        assert_eq!(choice(a, None, true, None)["allowed"], false);
        assert_eq!(choice(c, Some(true), false, None)["reason"], "Needs a newer processor");
        assert_eq!(choice(c, Some(false), false, None)["reason"], "Needs a newer processor");
        let v = choice(c, Some(true), true, Some(true));
        assert_eq!(v["allowed"], true);
        assert!(v["warning"].as_str().unwrap().contains("Secure Boot"));
        assert_eq!(v["beta"], true);
        assert!(choice(c, Some(true), true, Some(false))["warning"].is_null());
        assert!(choice(edition("ubuntu").unwrap(), Some(true), true, Some(true))["warning"].as_str().unwrap().contains("Secure Boot"));
        assert!(choice(edition("peppermint").unwrap(), Some(true), false, Some(true))["warning"].is_string());
    }
    #[test]
    fn reads_the_processor_and_secure_boot() {
        let v3 = "processor	: 0
flags		: fpu sse2 avx avx2 bmi1 bmi2 fma movbe

processor	: 1
flags		: fpu avx2 bmi2 fma movbe
";
        assert!(x86_64_v3(v3));
        assert!(!x86_64_v3("processor	: 0
flags		: fpu sse2 avx avx2 bmi2 fma
"));
        assert!(!x86_64_v3(""));
        assert_eq!(secure_boot_mokutil("SecureBoot enabled
"), Some(true));
        assert_eq!(secure_boot_mokutil("SecureBoot disabled
Platform is in Setup Mode
"), Some(false));
        assert_eq!(secure_boot_mokutil("EFI variables are not supported on this system"), None);
        assert_eq!(secure_boot_efivar(&[6, 0, 0, 0, 1]), Some(true));
        assert_eq!(secure_boot_efivar(&[6, 0, 0, 0, 0]), Some(false));
        assert_eq!(secure_boot_efivar(&[6, 0]), None);
    }
    #[test]
    fn who_can_move() {
        let root = tempfile::tempdir().unwrap();
        assert!(!eligible_at(root.path()));
        std::fs::create_dir_all(root.path().join("run")).unwrap();
        std::fs::write(root.path().join("run/ostree-booted"), "").unwrap();
        std::fs::create_dir_all(root.path().join("usr/share/ublue-os")).unwrap();
        std::fs::write(root.path().join("usr/share/ublue-os/image-info.json"), "{}").unwrap();
        assert!(eligible_at(root.path()));
        std::fs::create_dir_all(root.path().join("usr/share/nimbus")).unwrap();
        std::fs::write(root.path().join("usr/share/nimbus/image-info.json"), "{\"base\": \"anatase\"}").unwrap();
        // An edition switches to another edition.
        assert!(eligible_at(root.path()));
        assert_eq!(current_at(root.path()), Some("anatase"));
        std::fs::write(root.path().join("usr/share/nimbus/image-info.json"), "{\"base\": \"other\"}").unwrap();
        assert!(!eligible_at(root.path()));
        assert_eq!(current_at(root.path()), None);
    }
    #[test]
    fn switching_back_from_ubuntu_warns_about_secure_boot() {
        let a = edition("anatase").unwrap();
        assert!(choice_from(a, Some(true), true, Some(true), Some("ubuntu"))["warning"].is_string());
        assert!(choice_from(a, Some(true), true, Some(true), Some("anatase"))["warning"].is_null());
        assert!(choice_from(a, Some(true), true, Some(false), Some("ubuntu"))["warning"].is_null());
    }
}
