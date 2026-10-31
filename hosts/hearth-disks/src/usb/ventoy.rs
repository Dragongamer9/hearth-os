//! Ventoy (multi-ISO): one USB drive that starts several systems. Disks puts upstream Ventoy on the drive and copies
//! the ISOs you pick onto it (Hearth OS editions, Hearth Recovery, anything from the list, your own ISOs); at start-up
//! Ventoy shows a menu of them.
//!
//! Ventoy itself is the pinned upstream release (VERSION, SHA-256 from Ventoy's own sha256.txt): Disks downloads it
//! into ~/Downloads/Hearth USB and checks it; the root helper (`hearth-admin usb-ventoy`, hearth-usb) checks the
//! SHA-256 again itself against its own pinned copy before anything of it runs, checks the drive the same way as for
//! every other write (whole removable drive, never the system's, same size and serial as confirmed, nothing mounted),
//! runs Ventoy's Ventoy2Disk.sh and mounts Ventoy's exFAT partition owned by you. Disks copies the ISOs as you, checks
//! each copy's SHA-256 against the checked image, and `usb-finish` flushes, unmounts and checks the file system.
//! Every ISO is downloaded and checked (or confirmed by you, for your own files) before the drive is touched.
//!
//! HEARTH_VENTOY_URL replaces the download address (tests); the SHA-256 never changes without a new build.

use super::{catalog, fetch, log, sha256_cached, sidecar_checksum, verify, write, Emit, Job, KeyFn, CANCEL, CANCELLED};
use crate::model::human;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

pub const VERSION: &str = "1.1.06";
pub const SHA256: &str = "6176cc1ea7c8b822f5ee11fa5c3c71331aa194c72167a35196d8f1380b2feb17";
/// Ventoy's own partition after the exFAT one (VTOYEFI, 32 MB) and some room.
const RESERVED: u64 = 64 * 1024 * 1024;

pub fn file_name() -> String { format!("ventoy-{VERSION}-linux.tar.gz") }
pub fn url() -> String {
    std::env::var("HEARTH_VENTOY_URL").ok().filter(|u| !u.is_empty())
        .unwrap_or_else(|| format!("https://github.com/ventoy/Ventoy/releases/download/v{VERSION}/{}", file_name()))
}

/// One ISO for the Ventoy drive: from the list (`catalog`, `id`) or your own (`file`, `path`).
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Item {
    pub source: String,
    pub id: String,
    pub path: String,
    /// Your own file with nothing to check it against: copied only because you said so.
    pub accept_unverified: bool,
}

/// What the page shows about Ventoy (Make a bootable USB › Several ISOs).
pub fn info() -> Value {
    json!({"version": VERSION, "sha256": SHA256, "url": url(), "homepage": "https://www.ventoy.net/", "license": "GPL-3.0"})
}

/// Ventoy's release, downloaded once into the cache folder and checked against the pinned SHA-256.
pub fn fetch_ventoy(emit: &Emit) -> Result<PathBuf, String> {
    let dir = super::cache_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("Couldn't make {}: {e}", dir.display()))?;
    let fin = dir.join(file_name());
    if fin.is_file() {
        if let Ok(h) = verify::sha256_file(&fin, &CANCEL, &|_, _| {}) { if h == SHA256 { return Ok(fin); } }
        let _ = std::fs::remove_file(&fin);
    }
    let part = dir.join(format!(".{}.part", file_name()));
    emit(json!({"stage": "download", "pct": 0.0, "text": format!("Downloading Ventoy {VERSION}")}));
    fetch::download(&url(), &part, None, &CANCEL, &|g: fetch::Got| {
        emit(json!({"stage": "download", "pct": g.total.map(|t| g.done as f64 / t.max(1) as f64), "done": g.done, "total": g.total, "speed": g.speed, "eta": g.eta, "text": format!("Downloading Ventoy {VERSION}")}));
    })?;
    let got = verify::sha256_file(&part, &CANCEL, &|_, _| {})?;
    if got != SHA256 {
        let _ = std::fs::remove_file(&part);
        log(&format!("ventoy: SHA-256 mismatch: got {got}, expected {SHA256}; deleted"));
        return Err(format!("The Ventoy download isn't Ventoy {VERSION} as released, so it was deleted and nothing was written."));
    }
    std::fs::rename(&part, &fin).map_err(|e| format!("Couldn't keep the Ventoy download: {e}"))?;
    log(&format!("ventoy: {VERSION} downloaded and checked ({SHA256})"));
    Ok(fin)
}

/// A name for the ISO on the drive: its own file name, letters FAT/exFAT and Ventoy's menu take, and unique.
pub fn name_on_drive(file: &str, taken: &[String]) -> String {
    let clean: String = file.chars().map(|c| if c.is_ascii_alphanumeric() || "._-+ ()".contains(c) { c } else { '_' }).collect();
    let clean = clean.trim_matches(|c: char| c == '.' || c == ' ').to_string();
    let base = if clean.is_empty() { "image.iso".to_string() } else { clean };
    if !taken.iter().any(|t| t.eq_ignore_ascii_case(&base)) { return base; }
    let (stem, ext) = match base.rfind('.') { Some(i) if i > 0 => (base[..i].to_string(), base[i..].to_string()), _ => (base.clone(), String::new()) };
    (2..).map(|n| format!("{stem} ({n}){ext}")).find(|c| !taken.iter().any(|t| t.eq_ignore_ascii_case(c))).unwrap()
}

/// Ventoy starts ISO, IMG, WIM, VHD(x) and EFI files as they are; not compressed images.
pub fn bootable_name(file: &str) -> bool {
    let l = file.to_lowercase();
    [".iso", ".img", ".wim", ".vhd", ".vhdx", ".efi", ".vtoy"].iter().any(|e| l.ends_with(e))
}

struct Ready { path: PathBuf, name: String, size: u64, sha: String, what: String, verified_by: String, from_catalog: bool }

/// The whole job: every ISO checked first, then Ventoy, then the copies. Returns what to tell you.
pub fn run(job: &Job, cat: &catalog::Catalog, key_of: KeyFn, emit: Emit) -> Result<Value, String> {
    if job.items.is_empty() { return Err("Pick at least one ISO for the Ventoy drive.".into()); }
    if job.items.len() > 32 { return Err("That's more ISOs than one drive should hold (32 at most).".into()); }
    let (_, first) = super::the_drive(job)?;
    // 1. Every image, downloaded and checked (or confirmed) before anything is written.
    let mut ready: Vec<Ready> = vec![];
    for (i, it) in job.items.iter().enumerate() {
        if CANCEL.load(Ordering::SeqCst) { return Err(CANCELLED.into()); }
        emit(json!({"stage": "find", "pct": null, "text": format!("ISO {} of {}", i + 1, job.items.len())}));
        let r = match it.source.as_str() {
            "catalog" => {
                let (p, c) = super::fetch_release(cat, key_of, &it.id, &emit)?;
                let file = p.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                Ready { size: std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0), name: file, sha: c.sha256.clone(), what: format!("{} {}", c.name, c.version), verified_by: c.verified_by.clone(), path: p, from_catalog: true }
            }
            "file" => {
                let p = PathBuf::from(&it.path);
                if !p.is_file() { return Err(format!("{} isn't there anymore.", it.path)); }
                let sha = sha256_cached(&p, &emit)?;
                let verified_by = match sidecar_checksum(&p) {
                    Some((f, h)) if h == sha => format!("SHA-256 matches {f}"),
                    Some((f, _)) => return Err(format!("{} doesn't match the checksum in {f}: it's damaged or not the file it should be. Nothing was written.", p.display())),
                    None if it.accept_unverified => "not checked (you chose to copy it anyway)".to_string(),
                    None => return Err(format!("Nothing could check {}. Confirm that you trust it first.", p.display())),
                };
                let file = p.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                Ready { size: std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0), what: file.clone(), name: file, sha, verified_by, path: p, from_catalog: false }
            }
            _ => return Err("Each ISO comes from the list or is your own file.".into()),
        };
        if !bootable_name(&r.name) { return Err(format!("{} isn't something Ventoy starts (ISO, IMG, WIM, VHD or EFI files).", r.name)); }
        ready.push(r);
    }
    // Names on the drive, unique.
    let mut taken: Vec<String> = vec![];
    for r in ready.iter_mut() { r.name = name_on_drive(&r.name, &taken); taken.push(r.name.clone()); }
    let total: u64 = ready.iter().map(|r| r.size).sum();
    if total + RESERVED > first.size { return Err(format!("They don't fit: the ISOs add up to {}, {} is {} (Ventoy keeps about 64 MB).", human(total), first.name, human(first.size))); }
    // 2. Ventoy itself.
    let tarball = fetch_ventoy(&emit)?;
    if CANCEL.load(Ordering::SeqCst) { return Err(CANCELLED.into()); }
    // 3. The drive again, right before writing, then let go of it.
    let (d, u) = super::the_drive(job)?;
    emit(json!({"stage": "prepare", "pct": null, "text": format!("Getting {} ready", u.name)}));
    super::let_go(&d)?;
    emit(json!({"stage": "prepare", "pct": null, "text": format!("Putting Ventoy {VERSION} on {}", u.name)}));
    let mut args = write::drive_args("usb-ventoy", &u);
    args.extend(["--tarball".into(), tarball.to_string_lossy().into_owned(), "--label".into(), "Ventoy".into()]);
    if job.scheme == "gpt" { args.push("--gpt".into()); }
    let prep = write::helper(&args, "prepare", &emit)?;
    let mount = PathBuf::from(prep["mount"].as_str().ok_or("The helper didn't say where it mounted the drive")?);
    // 4. The copies, each checked against the checked image.
    let copied = (|| -> Result<Vec<Value>, String> {
        let started = std::time::Instant::now();
        let mut last = std::time::Instant::now();
        let mut done = 0u64;
        let mut out = vec![];
        for r in &ready {
            let to = mount.join(&r.name);
            write::copy_file(&r.path, &to, &mut done, total, started, &mut last, &emit)?;
            emit(json!({"stage": "readback", "pct": null, "text": format!("Checking {}", r.name)}));
            let got = sha_of(&to)?;
            if got != r.sha { return Err(format!("{} on the USB drive isn't the same as the checked image. Write it again.", r.name)); }
            out.push(json!({"name": r.name, "what": r.what, "bytes": r.size, "sha256": got, "verifiedBy": r.verified_by}));
        }
        Ok(out)
    })();
    emit(json!({"stage": "finish", "pct": null, "text": "Flushing and checking the file system"}));
    let fin = write::helper(&[String::from("usb-finish"), u.device.clone()], "finish", &emit);
    let copied = copied?;
    let fin = fin?;
    if fin["ok"] != true { return Err(format!("The file system check found problems: {}", fin["fsck"].as_str().unwrap_or(""))); }
    // 5. Downloads kept or removed, as you chose.
    if !job.keep { for r in ready.iter().filter(|r| r.from_catalog) { let _ = std::fs::remove_file(&r.path); let _ = std::fs::remove_file(super::sidecar(&r.path)); } }
    let names: Vec<&str> = ready.iter().map(|r| r.what.as_str()).collect();
    let msg = format!("Ventoy {VERSION} and {} ISO{} are on {}: {}. Start a PC from it and pick one in Ventoy's menu. Add more ISOs later by copying them onto its Ventoy partition.",
        ready.len(), if ready.len() == 1 { "" } else { "s" }, u.name, names.join(", "));
    log(&format!("done: {msg}"));
    Ok(json!({"message": msg, "summary": {"mode": "ventoy", "ventoy": VERSION, "isos": copied, "scheme": prep["scheme"].clone()}, "drive": u.id}))
}

fn sha_of(p: &Path) -> Result<String, String> {
    verify::sha256_file(p, &CANCEL, &|_, _| {}).map_err(|e| format!("Couldn't read back {}: {e}", p.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pinned_release() {
        assert_eq!(SHA256.len(), 64);
        assert!(SHA256.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
        assert!(url().starts_with("https://github.com/ventoy/Ventoy/releases/download/v"), "{}", url());
        assert!(url().ends_with(&file_name()));
    }

    #[test]
    fn names_on_the_drive() {
        let taken = vec!["hearth.iso".to_string()];
        assert_eq!(name_on_drive("debian-13.iso", &taken), "debian-13.iso");
        assert_eq!(name_on_drive("Hearth.iso", &taken), "Hearth (2).iso");
        assert_eq!(name_on_drive("a/b:c?.iso", &[]), "a_b_c_.iso");
        assert_eq!(name_on_drive("...", &[]), "image.iso");
        assert!(bootable_name("x.ISO") && bootable_name("y.img") && !bootable_name("z.img.xz") && !bootable_name("w.zst"));
    }
}
