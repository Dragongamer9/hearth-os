//! Make a bootable USB (Linux): pick a system from the catalog (or an ISO you have), and Disks downloads it,
//! checks it against the project's signed checksum, and writes it to a USB drive or SD card.
//!
//! The flow of one job (`run`): find the newest release (catalog.rs) → check the checksum file's signature → download
//! into ~/Downloads/Hearth USB (resumable) → SHA-256 (and the image's own signature where the project signs images)
//! → look inside (image.rs: hybrid → raw, Windows → files) → let go of the drive (udisks) → write (write.rs, the root
//! steps through hearth-admin) → read back / check the copies → done (the drive can be removed safely).
//! A download that fails a check is deleted and never written. Your own ISO is checked against a checksum file next
//! to it when there is one; without one it's only written after you say so (the page warns first).
//! Windows: Microsoft's own page opens in your browser and Disks spots the ISO when it lands in Downloads
//! (`find_windows_iso`); its SHA-256 is shown to compare with Microsoft's list.

pub mod catalog;
pub mod fetch;
pub mod image;
pub mod keys;
pub mod verify;
pub mod write;

use crate::model::{human, Drive};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub const CANCELLED: &str = "Cancelled. Nothing more was written; the USB drive may need writing again before it's used.";
pub static CANCEL: AtomicBool = AtomicBool::new(false);

/// Progress for the page: {stage, pct (0..1 or null), done, total, speed, eta, text}.
pub type Emit = Arc<dyn Fn(Value) + Send + Sync>;

pub fn cancel() { CANCEL.store(true, Ordering::SeqCst); }

// ---------------------------------------------------------------- the log

fn data_dir() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).filter(|p| p.is_absolute())
        .unwrap_or_else(|| home().join(".local/share")).join("nimbus")
}
fn home() -> PathBuf { std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/tmp")) }
pub fn log_path() -> PathBuf { data_dir().join("usb-maker.log") }

/// "2026-10-25 14:03:07" (UTC) for the log.
fn stamp() -> String {
    let s = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let (days, rem) = (s / 86400, s % 86400);
    // civil-from-days (Howard Hinnant)
    let z = days as i64 + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{y:04}-{m:02}-{d:02} {:02}:{:02}:{:02}", rem / 3600, rem / 60 % 60, rem % 60)
}

pub fn log(line: &str) {
    use std::io::Write;
    let p = log_path();
    if let Some(d) = p.parent() { let _ = std::fs::create_dir_all(d); }
    // Keep it small: start over past 2 MB.
    if std::fs::metadata(&p).map(|m| m.len() > 2_000_000).unwrap_or(false) { let _ = std::fs::rename(&p, p.with_extension("log.old")); }
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&p) { let _ = writeln!(f, "{} {line}", stamp()); }
    eprintln!("usb: {line}");
}

pub fn log_tail() -> String {
    let t = std::fs::read_to_string(log_path()).unwrap_or_default();
    let lines: Vec<&str> = t.lines().collect();
    lines[lines.len().saturating_sub(200)..].join("\n")
}

// ---------------------------------------------------------------- where downloads go

/// Your Downloads folder (xdg-user-dirs), else ~/Downloads.
pub fn downloads_dir() -> PathBuf {
    let h = home();
    let cfg = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| h.join(".config")).join("user-dirs.dirs");
    if let Ok(t) = std::fs::read_to_string(cfg) {
        for l in t.lines() {
            if let Some(v) = l.trim().strip_prefix("XDG_DOWNLOAD_DIR=") {
                let v = v.trim().trim_matches('"').replace("$HOME", &h.to_string_lossy());
                let p = PathBuf::from(v);
                if p.is_absolute() && p != h { return p; }
            }
        }
    }
    h.join("Downloads")
}

pub fn cache_dir() -> PathBuf { downloads_dir().join("Hearth USB") }

/// What Disks remembers about a download it checked: "<file>.hearth.json" next to it.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Checked { pub entry: String, pub name: String, pub version: String, pub sha256: String, pub verified_by: String, pub when: String, pub url: String }

fn sidecar(p: &Path) -> PathBuf { PathBuf::from(format!("{}.hearth.json", p.display())) }

/// Downloads kept in the cache folder.
pub fn cache_list() -> Vec<Value> {
    let mut v = vec![];
    for e in std::fs::read_dir(cache_dir()).into_iter().flatten().flatten() {
        let p = e.path();
        let n = p.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        if n.ends_with(".hearth.json") || n.ends_with(".part") || n.starts_with('.') { continue; }
        let c: Checked = std::fs::read(sidecar(&p)).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        v.push(json!({"path": p, "file": n, "size": e.metadata().map(|m| m.len()).unwrap_or(0), "entry": c.entry, "name": c.name, "version": c.version, "sha256": c.sha256, "verifiedBy": c.verified_by, "when": c.when}));
    }
    v
}

/// Removes a download from the cache folder (only from there).
pub fn cache_remove(path: &str) -> Result<(), String> {
    let p = PathBuf::from(path);
    let dir = cache_dir();
    let real = p.canonicalize().map_err(|_| "It's already gone.".to_string())?;
    if real.parent() != dir.canonicalize().ok().as_deref() { return Err("Only downloads in the Hearth USB folder are removed here.".into()); }
    std::fs::remove_file(&real).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(sidecar(&real));
    Ok(())
}

// ---------------------------------------------------------------- drives

/// A drive Make a bootable USB can write to, as the page shows it.
#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UsbDrive {
    pub id: String,
    pub device: String,
    pub name: String,
    pub size: u64,
    pub serial: String,
    pub kind: String,
    pub connection: String,
    /// Not removable: only with "Show all drives", a typed ERASE and an administrator's password.
    pub internal: bool,
    pub partitions: Vec<String>,
    pub fingerprint: String,
}

fn allow_loop() -> bool { std::env::var("HEARTH_USB_ALLOW_LOOP").map(|v| v == "1").unwrap_or(false) }

/// Which drives are offered: never the system's, a read-only one or a disc drive; USB drives and SD cards always;
/// internal drives only with `show_all`; disk images only in CI.
pub fn eligible(d: &Drive, show_all: bool) -> bool {
    if d.system || d.read_only || d.kind == "optical" || d.size == 0 { return false; }
    if d.kind == "image" { return allow_loop(); }
    removable(d) || show_all
}

fn removable(d: &Drive) -> bool { matches!(d.kind.as_str(), "usb" | "sd") || d.removable }

pub fn usb_drive(d: &Drive) -> UsbDrive {
    UsbDrive {
        id: d.id.clone(), device: d.device.clone(), name: d.name(), size: d.size, serial: d.serial.clone(), kind: d.kind.clone(), connection: d.connection.clone(),
        internal: !removable(d) && d.kind != "image",
        partitions: d.partitions.iter().filter(|p| p.kind != "free").map(|p| {
            let mut s = if p.label.is_empty() { p.what.clone() } else { format!("\"{}\" ({})", p.label, p.what) };
            s.push_str(&format!(", {}", human(p.size)));
            if let Some(u) = p.used { if u > 0 { s.push_str(&format!(", {} used", human(u))); } }
            s
        }).collect(),
        fingerprint: crate::safety::fingerprint(d, None),
    }
}

pub fn drives(show_all: bool) -> Result<Vec<UsbDrive>, String> {
    Ok(crate::list()?.iter().filter(|d| eligible(d, show_all)).map(usb_drive).collect())
}

// ---------------------------------------------------------------- the catalog for the page

pub fn catalog_for_page() -> Value {
    let c = catalog::load();
    let entries: Vec<Value> = c.entries.iter().map(|e| {
        let signed = e.signature.as_ref().and_then(|s| c.keys.get(&s.key)).map(|k| k.owner.clone());
        json!({"id": e.id, "group": e.group, "family": e.family, "name": e.name, "flavour": e.flavour, "arch": e.arch, "homepage": e.homepage,
               "about": e.about, "sizeHint": e.size_hint, "windows": matches!(e.resolve, catalog::Resolve::MicrosoftPage { .. }),
               "signedBy": signed, "note": e.note, "pinned": e.pinned})
    }).collect();
    json!({"groups": c.groups, "entries": entries, "cache": cache_dir(), "downloads": downloads_dir()})
}

/// The newest release of an entry, for the page (network).
pub fn resolve_for_page(id: &str) -> Result<Value, String> {
    let c = catalog::load();
    let e = c.entry(id).ok_or("That isn't in the list.")?;
    let r = catalog::resolve(e, &fetch::get)?;
    let cached = cache_dir().join(&r.file_name);
    let ck: Option<Checked> = std::fs::read(sidecar(&cached)).ok().and_then(|b| serde_json::from_slice(&b).ok());
    let signed = e.signature.as_ref().and_then(|s| c.keys.get(&s.key)).map(|k| k.owner.clone());
    Ok(json!({"id": r.id, "name": r.name, "version": r.version, "url": r.url, "file": r.file_name, "size": r.size,
              "signedBy": signed, "signatureOf": r.signature_of, "checksums": r.checksums_url, "note": e.note,
              "cached": cached.is_file() && ck.is_some(), "cachedPath": cached}))
}

pub fn open_page(id: &str) -> Result<(), String> {
    let c = catalog::load();
    let e = c.entry(id).ok_or("That isn't in the list.")?;
    let url = match &e.resolve { catalog::Resolve::MicrosoftPage { page, .. } => page.clone(), _ => e.homepage.clone() };
    if !url.starts_with("https://") { return Err("Only https pages open from here.".into()); }
    std::process::Command::new("xdg-open").arg(&url).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null())
        .spawn().map(|mut ch| { std::thread::spawn(move || { let _ = ch.wait(); }); }).map_err(|e| format!("Couldn't open the browser ({e})"))
}

// ---------------------------------------------------------------- Windows from Microsoft's page

static SEEN: Mutex<Option<HashMap<PathBuf, u64>>> = Mutex::new(None);

/// A Windows ISO that finished downloading in Downloads since `since_ms` (Unix ms): one whose size stayed the same
/// since the last look and that has no browser .part / .crdownload beside it. The newest wins.
pub fn find_windows_iso(since_ms: u64) -> Option<Value> {
    let dir = downloads_dir();
    let mut found: Vec<(u64, PathBuf, u64)> = vec![];
    let mut seen = SEEN.lock().unwrap();
    let map = seen.get_or_insert_with(HashMap::new);
    for e in std::fs::read_dir(&dir).ok()?.flatten() {
        let p = e.path();
        let n = p.file_name().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default();
        if !n.ends_with(".iso") { continue; }
        let Ok(m) = e.metadata() else { continue };
        let mtime = m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_millis() as u64).unwrap_or(0);
        if mtime + 5000 < since_ms { continue; }
        let busy = ["part", "crdownload", "download", "partial"].iter().any(|x| dir.join(format!("{}.{x}", p.file_name().unwrap().to_string_lossy())).exists());
        let size = m.len();
        let stable = map.get(&p) == Some(&size) && size > 100 * 1024 * 1024;
        map.insert(p.clone(), size);
        if busy || !stable { continue; }
        let windowsish = n.starts_with("win") || n.contains("windows") || n.contains("x64fre") || n.contains("cpba") || n.contains("cccoma");
        found.push((if windowsish { mtime + (1 << 50) } else { mtime }, p, size));
    }
    found.sort();
    found.pop().map(|(_, p, size)| json!({"path": p, "file": p.file_name().map(|s| s.to_string_lossy().into_owned()), "size": size}))
}

// ---------------------------------------------------------------- your own file

static HASHES: Mutex<Option<HashMap<(PathBuf, u64, u64), String>>> = Mutex::new(None);

fn file_key(p: &Path) -> Option<(PathBuf, u64, u64)> {
    let m = std::fs::metadata(p).ok()?;
    let mt = m.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_millis() as u64;
    Some((p.to_path_buf(), m.len(), mt))
}

/// SHA-256 of a file, remembered for the session (by path, size and time) so a big ISO is hashed once.
pub fn sha256_cached(p: &Path, emit: &Emit) -> Result<String, String> {
    let key = file_key(p);
    if let Some(k) = &key { if let Some(h) = HASHES.lock().unwrap().get_or_insert_with(HashMap::new).get(k) { return Ok(h.clone()); } }
    let started = std::time::Instant::now();
    let h = verify::sha256_file(p, &CANCEL, &|done, total| {
        let speed = done as f64 / started.elapsed().as_secs_f64().max(0.1);
        emit(json!({"stage": "verify", "pct": done as f64 / total.max(1) as f64, "done": done, "total": total, "speed": speed, "eta": (total.saturating_sub(done) as f64 / speed.max(1.0)) as u64, "text": "Working out the SHA-256"}));
    })?;
    if let Some(k) = key { HASHES.lock().unwrap().get_or_insert_with(HashMap::new).insert(k, h.clone()); }
    Ok(h)
}

/// A checksum file next to your own image that lists it (name.sha256, SHA256SUMS ...): (its name, the SHA-256 it gives).
pub fn sidecar_checksum(p: &Path) -> Option<(String, String)> {
    let dir = p.parent()?;
    let name = p.file_name()?.to_string_lossy().into_owned();
    let mut cands: Vec<PathBuf> = ["sha256", "sha256sum", "sha256.txt", "SHA256"].iter().map(|e| dir.join(format!("{name}.{e}"))).collect();
    cands.push(dir.join(format!("{name}-CHECKSUM")));
    for f in ["SHA256SUMS", "sha256sums.txt", "sha256sum.txt", "SHA256SUMS.txt", "CHECKSUM"] { cands.push(dir.join(f)); }
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let n = e.file_name().to_string_lossy().into_owned();
        if n.ends_with("-CHECKSUM") { cands.push(e.path()); }
    }
    for c in cands {
        let Ok(t) = std::fs::read_to_string(&c) else { continue };
        if t.len() > 1_000_000 { continue; }
        let sums = catalog::parse_sums(&t);
        let hit = sums.iter().find(|(n, _)| n == &name).map(|(_, h)| h.clone())
            .or_else(|| if sums.len() == 1 && (sums[0].0.is_empty() || c.file_name().map(|f| f.to_string_lossy().starts_with(&name)).unwrap_or(false)) { Some(sums[0].1.clone()) } else { None });
        if let Some(h) = hit { return Some((c.file_name().unwrap().to_string_lossy().into_owned(), h)); }
    }
    None
}

/// Looks at an image: how it would be written, and (for an ISO that isn't hybrid) what's inside.
pub fn inspect(p: &Path) -> Result<image::Inspect, String> {
    let format = image::format_of(p);
    let size = write::unpacked_size(p, &format).or_else(|| std::fs::metadata(p).ok().map(|m| m.len())).unwrap_or(0);
    let mut i = image::Inspect { format: format.clone(), size, ..Default::default() };
    if format == "unknown" { i.mode = "unknown".into(); i.warnings.push("This isn't an ISO or a disk image (.iso, .img, .img.xz, .img.zst, .img.gz).".into()); return Ok(i); }
    let head = image::read_head(p, &format)?;
    let (table, iso, label) = image::sniff(&head);
    i.label = label;
    i.hybrid = table;
    if format.starts_with("img") {
        i.mode = "raw".into();
        if !table { i.warnings.push("This disk image has no partition table at its start. It's written as it is; check that it's meant for a USB drive.".into()); }
        return Ok(i);
    }
    if table { i.mode = "raw".into(); return Ok(i); }
    if !iso { i.mode = "unknown".into(); i.warnings.push("This file isn't an ISO image.".into()); return Ok(i); }
    let m = write::mount_iso(p)?;
    let (win, efi) = image::look_inside(&m.root);
    if let Some(w) = win {
        i.mode = "windows".into();
        if w.needs_split { i.warnings.push(format!("{} is {}, more than FAT32 can hold in one file: it's split into parts Windows Setup reads.", w.install, human(w.install_size))); }
        i.windows = Some(w);
    } else if efi {
        i.mode = "uefi-copy".into();
        i.warnings.push("This ISO can't be written as it is (it isn't a hybrid ISO). Its files are copied to a FAT32 drive instead: it will start only on UEFI PCs, and a few systems look for their own disc name and may not start this way.".into());
    } else {
        i.mode = "unknown".into();
        i.warnings.push("This ISO isn't made to start from a USB drive (it's neither hybrid nor UEFI-bootable). Hearth won't write it.".into());
    }
    Ok(i)
}

/// Your own file: its SHA-256, a checksum file beside it if there is one, and how it would be written.
pub fn check_file(path: &str, emit: &Emit) -> Result<Value, String> {
    let p = PathBuf::from(path);
    let m = std::fs::metadata(&p).map_err(|_| "That file isn't there.".to_string())?;
    if !m.is_file() { return Err("That isn't a file.".into()); }
    CANCEL.store(false, Ordering::SeqCst);
    let i = inspect(&p)?;
    let sha = sha256_cached(&p, emit)?;
    let side = sidecar_checksum(&p);
    let cached: Option<Checked> = std::fs::read(sidecar(&p)).ok().and_then(|b| serde_json::from_slice(&b).ok());
    let (verified, against, mismatch) = match (&side, &cached) {
        (Some((f, h)), _) => (h == &sha, Some(f.clone()), h != &sha),
        (None, Some(c)) if !c.sha256.is_empty() => (c.sha256 == sha, Some(format!("Hearth's check when it was downloaded ({})", c.verified_by)), c.sha256 != sha),
        _ => (false, None, false),
    };
    Ok(json!({"path": p, "file": p.file_name().map(|s| s.to_string_lossy().into_owned()), "size": m.len(), "sha256": sha, "verified": verified, "against": against, "mismatch": mismatch, "inspect": i}))
}

// ---------------------------------------------------------------- one job

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Job {
    /// catalog or file
    pub source: String,
    /// The catalog entry (source catalog).
    pub id: String,
    /// The image (source file).
    pub path: String,
    /// The drive's id and the fingerprint shown with it.
    pub drive: String,
    pub expect: String,
    pub show_all: bool,
    pub typed: String,
    /// gpt (the default) or mbr, for Windows.
    pub scheme: String,
    pub tweaks: image::Tweaks,
    /// Keep the download after writing.
    pub keep: bool,
    /// Write your own file even though nothing could check it (the page asked first).
    pub accept_unverified: bool,
}

/// A key file's text by name (the built-in keys; tests give their own).
pub type KeyFn<'a> = &'a dyn Fn(&str) -> Option<String>;
pub fn builtin_key(file: &str) -> Option<String> { keys::text(file).map(str::to_string) }

/// Downloads and checks a catalog entry; returns the checked file and what vouched for it.
fn fetch_release(c: &catalog::Catalog, key_of: KeyFn, id: &str, emit: &Emit) -> Result<(PathBuf, Checked), String> {
    let e = c.entry(id).ok_or("That isn't in the list.")?.clone();
    emit(json!({"stage": "find", "pct": null, "text": format!("Finding the newest {}", e.name)}));
    let r = catalog::resolve(&e, &fetch::get)?;
    log(&format!("{}: {} {} from {}", e.id, r.name, r.version, r.url));
    let key = e.signature.as_ref().map(|s| c.keys.get(&s.key).cloned().ok_or("unknown key")).transpose()?;
    let key_owned: Vec<String> = key.iter().filter_map(|k| key_of(&k.file)).collect();
    let key_text: Vec<&str> = key_owned.iter().map(|s| s.as_str()).collect();
    if key.is_some() && key_text.is_empty() { return Err("The key that signs this isn't built into Disks.".into()); }
    let allowed: Vec<String> = key.as_ref().map(|k| k.fingerprints.clone()).unwrap_or_default();
    let mut verified_by = String::new();
    // 1. The checksum file, checked against its signature before it's believed.
    let mut sums_text: Option<String> = r.checksums_body.as_ref().map(|b| String::from_utf8_lossy(b).into_owned());
    if let (Some(s), Some(body)) = (&e.signature, &r.checksums_body) {
        if s.of == "checksums" {
            emit(json!({"stage": "find", "pct": null, "text": "Checking the signature on the checksums"}));
            if s.kind == "clearsigned" {
                let (who, plain) = verify::clearsigned(&key_text, &allowed, body)?;
                verified_by = format!("signed by {} ({who})", key.as_ref().unwrap().owner);
                sums_text = Some(plain);
            } else {
                let sig_url = r.signature_url.clone().ok_or("No signature address")?;
                let sig = fetch::get(&sig_url)?;
                let who = verify::detached_bytes(&key_text, &allowed, body, &sig)?;
                verified_by = format!("signed by {} ({who})", key.as_ref().unwrap().owner);
            }
            log(&format!("{}: checksums {verified_by}", e.id));
        }
    }
    let expected = catalog::expected_sha256(&r, sums_text.as_deref())?;
    if verified_by.is_empty() && e.signature.as_ref().map(|s| s.of != "image").unwrap_or(true) {
        verified_by = match &e.resolve {
            catalog::Resolve::Pop { .. } => "SHA-256 from System76's build service (not signed)".into(),
            _ => "SHA-256 from the project's checksum file (not signed)".into(),
        };
    }
    // 2. The download (or the copy from last time, if it's the same file).
    let dir = cache_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("Couldn't make {}: {e}", dir.display()))?;
    let fin = dir.join(&r.file_name);
    let prev: Option<Checked> = std::fs::read(sidecar(&fin)).ok().and_then(|b| serde_json::from_slice(&b).ok());
    let reuse = fin.is_file() && prev.as_ref().map(|p| p.sha256 == expected).unwrap_or(false);
    let path = if reuse {
        log(&format!("{}: using the earlier download {}", e.id, fin.display()));
        fin.clone()
    } else {
        let part = dir.join(format!(".{}.{}.part", r.file_name, &expected[..12.min(expected.len())]));
        let total = r.size.or_else(|| fetch::head(&r.url).ok().and_then(|(_, l)| l));
        emit(json!({"stage": "download", "pct": 0.0, "done": 0, "total": total, "text": format!("Downloading {}", r.file_name)}));
        fetch::download(&r.url, &part, total, &CANCEL, &|g: fetch::Got| {
            emit(json!({"stage": "download", "pct": g.total.map(|t| g.done as f64 / t.max(1) as f64), "done": g.done, "total": g.total, "speed": g.speed, "eta": g.eta, "text": format!("Downloading {}", r.file_name)}));
        })?;
        part
    };
    // 3. Its SHA-256, and the image's own signature where the project signs the image.
    let started = std::time::Instant::now();
    let got = verify::sha256_file(&path, &CANCEL, &|done, total| {
        let speed = done as f64 / started.elapsed().as_secs_f64().max(0.1);
        emit(json!({"stage": "verify", "pct": done as f64 / total.max(1) as f64, "done": done, "total": total, "speed": speed, "text": "Checking the download (SHA-256)"}));
    })?;
    if got != expected {
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(sidecar(&fin));
        log(&format!("{}: SHA-256 mismatch: got {got}, expected {expected}; deleted", e.id));
        return Err("The download doesn't match the project's checksum, so it was deleted and nothing was written. Try again (a mirror may have been out of date).".into());
    }
    if let Some(s) = &e.signature {
        if s.of == "image" {
            emit(json!({"stage": "verify", "pct": null, "text": "Checking the signature"}));
            let sig = fetch::get(r.signature_url.as_deref().ok_or("No signature address")?)?;
            let who = match verify::detached_path(&key_text, &allowed, &path, &sig) {
                Ok(w) => w,
                Err(err) => { let _ = std::fs::remove_file(&path); return Err(format!("{err} The download was deleted and nothing was written.")); }
            };
            if let Some(want) = &r.index_signer { if !want.eq_ignore_ascii_case(&who) { let _ = std::fs::remove_file(&path); return Err("It's signed, but not by the key the project's release list names. The download was deleted.".into()); } }
            verified_by = format!("SHA-256 and signed by {} ({who})", key.as_ref().unwrap().owner);
        } else if !verified_by.starts_with("SHA-256") {
            verified_by = format!("SHA-256 from the checksum file {verified_by}");
        }
    }
    if !reuse { std::fs::rename(&path, &fin).map_err(|e| format!("Couldn't keep the download: {e}"))?; }
    let ck = Checked { entry: e.id.clone(), name: r.name.clone(), version: r.version.clone(), sha256: got, verified_by: verified_by.clone(), when: stamp(), url: r.url.clone() };
    let _ = std::fs::write(sidecar(&fin), serde_json::to_vec_pretty(&ck).unwrap_or_default());
    log(&format!("{}: checked: {verified_by}", e.id));
    emit(json!({"stage": "verify", "pct": 1.0, "text": format!("Checked: {verified_by}")}));
    Ok((fin, ck))
}

/// The drive, looked at again now, and checked against what you confirmed.
fn the_drive(job: &Job) -> Result<(Drive, UsbDrive), String> {
    let all = crate::list()?;
    let d = all.into_iter().find(|d| d.id == job.drive).ok_or("That drive isn't there anymore. It may have been unplugged.")?;
    if !eligible(&d, job.show_all) { return Err("Hearth won't write to that drive.".into()); }
    let u = usb_drive(&d);
    if job.expect.is_empty() || u.fingerprint != job.expect { return Err("This isn't the same drive you confirmed (it may have been swapped or changed). Nothing was written.".into()); }
    if u.internal && job.typed.trim().to_uppercase() != crate::safety::ERASE_WORD { return Err("Type ERASE to write to an internal drive.".into()); }
    Ok((d, u))
}

/// Unmounts and locks what's on the drive (as you, through udisks), so the helper can open it alone.
fn let_go(d: &Drive) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    { crate::linux::release(d) }
    #[cfg(not(target_os = "linux"))]
    { let _ = d; Ok(()) }
}

/// Runs a whole job. Returns what to tell you.
pub fn run(job: &Job, emit: Emit) -> Result<Value, String> { run_in(job, &catalog::load(), &builtin_key, emit) }

/// A job against a given catalog and keys (the tests' own local server and test key).
pub fn run_in(job: &Job, cat: &catalog::Catalog, key_of: KeyFn, emit: Emit) -> Result<Value, String> {
    CANCEL.store(false, Ordering::SeqCst);
    log(&format!("job: {}", serde_json::to_string(job).unwrap_or_default()));
    let (_, first) = the_drive(job)?;
    // 1. The image.
    let (path, checked, from_catalog) = match job.source.as_str() {
        "catalog" => { let (p, c) = fetch_release(cat, key_of, &job.id, &emit)?; (p, Some(c), true) }
        "file" => (PathBuf::from(&job.path), None, false),
        _ => return Err("Choose a system or an ISO first.".into()),
    };
    // 2. Your own file: checked against what's beside it, or written only because you said so.
    let mut verified_by = checked.as_ref().map(|c| c.verified_by.clone()).unwrap_or_default();
    let sha;
    if !from_catalog {
        sha = sha256_cached(&path, &emit)?;
        match sidecar_checksum(&path) {
            Some((f, h)) if h == sha => verified_by = format!("SHA-256 matches {f}"),
            Some((f, _)) => return Err(format!("This file doesn't match the checksum in {f}: it's damaged or not the file it should be. Nothing was written.")),
            None => {
                if !job.accept_unverified { return Err("Nothing could check this file. Confirm that you trust it first.".into()); }
                verified_by = "not checked (you chose to write it anyway)".into();
            }
        }
    } else { sha = checked.as_ref().map(|c| c.sha256.clone()).unwrap_or_default(); }
    if CANCEL.load(Ordering::SeqCst) { return Err(CANCELLED.into()); }
    // 3. How it's written.
    emit(json!({"stage": "inspect", "pct": null, "text": "Looking inside the image"}));
    let info = inspect(&path)?;
    log(&format!("inspect: {}", serde_json::to_string(&info).unwrap_or_default()));
    if info.mode == "unknown" { return Err(info.warnings.join(" ")); }
    if info.size > first.size { return Err(format!("It doesn't fit: the image is {}, {} is {}.", human(info.size), first.name, human(first.size))); }
    // 4. The drive again, right before writing, then let go of it.
    let (d, u) = the_drive(job)?;
    emit(json!({"stage": "prepare", "pct": null, "text": format!("Getting {} ready", u.name)}));
    let_go(&d)?;
    // 5. Write.
    let summary = match info.mode.as_str() {
        "raw" => {
            let written = write::raw(&path, &info.format, &u, emit.clone())?;
            if info.format == "iso" && !sha.is_empty() && written != sha { return Err("What was written doesn't match the checked image. Write it again.".into()); }
            json!({"mode": "raw", "sha256": written})
        }
        "windows" | "uefi-copy" => {
            let m = write::mount_iso(&path)?;
            let win = info.windows.clone();
            let unattend = if win.is_some() && job.tweaks.any() { Some(image::autounattend(&job.tweaks, win.as_ref().map(|w| w.arch.as_str()).unwrap_or("x86_64"))?) } else { None };
            let label = write::fat_label(&info.label, if win.is_some() { "WINUSB" } else { "USB" });
            let scheme = if job.scheme == "mbr" { "mbr" } else { "gpt" };
            let fj = write::FilesJob { iso_root: &m.root, windows: win.as_ref(), scheme, label: &label, unattend };
            let mut s = write::files(&fj, &u, emit.clone())?;
            s["mode"] = json!(info.mode);
            s["scheme"] = json!(scheme);
            s
        }
        other => return Err(format!("Hearth doesn't know how to write a {other} image.")),
    };
    // 6. The download is kept or removed, as you chose.
    if from_catalog && !job.keep { let _ = std::fs::remove_file(&path); let _ = std::fs::remove_file(sidecar(&path)); }
    let what = checked.as_ref().map(|c| format!("{} {}", c.name, c.version)).unwrap_or_else(|| path.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default());
    let mut msg = format!("{what} is on {}. Checked: {verified_by}.", u.name);
    if info.mode == "windows" { msg.push_str(" Start the PC from it (UEFI) to install Windows."); }
    if info.mode == "uefi-copy" { msg.push_str(" It starts on UEFI PCs."); }
    log(&format!("done: {msg} {summary}"));
    Ok(json!({"message": msg, "summary": summary, "verifiedBy": verified_by, "drive": u.id, "info": info}))
}

// ---------------------------------------------------------------- the catalog check (CI): `hearth-disks --usb-catalog-check`

/// Resolves every entry over the network, checks each checksum file's signature (or the signer of an image's
/// signature), and asks the image's server about it (HEAD). Returns (report lines, problems).
pub fn catalog_check() -> (Vec<String>, Vec<String>) {
    let c = catalog::load();
    let mut lines = vec![];
    let mut bad = catalog::validate(&c);
    for e in &c.entries {
        if matches!(e.resolve, catalog::Resolve::MicrosoftPage { .. }) { lines.push(format!("{}: Microsoft's page (opened in the browser; not fetched)", e.id)); continue; }
        let r = match catalog::resolve(e, &fetch::get) { Ok(r) => r, Err(err) => { bad.push(format!("{}: {err}", e.id)); continue; } };
        let key = e.signature.as_ref().and_then(|s| c.keys.get(&s.key));
        let key_text: Vec<&str> = key.iter().filter_map(|k| keys::text(&k.file)).collect();
        let allowed: Vec<String> = key.map(|k| k.fingerprints.clone()).unwrap_or_default();
        let mut how = String::from("unsigned");
        let mut sums = r.checksums_body.as_ref().map(|b| String::from_utf8_lossy(b).into_owned());
        if let Some(s) = &e.signature {
            let res: Result<String, String> = if s.of == "checksums" && s.kind == "clearsigned" {
                verify::clearsigned(&key_text, &allowed, r.checksums_body.as_deref().unwrap_or(b"")).map(|(w, plain)| { sums = Some(plain); w })
            } else if s.of == "checksums" {
                fetch::get(r.signature_url.as_deref().unwrap_or("")).and_then(|sig| verify::detached_bytes(&key_text, &allowed, r.checksums_body.as_deref().unwrap_or(b""), &sig))
            } else {
                // An image's signature: the image isn't downloaded here; the signature's signer is read instead.
                fetch::get(r.signature_url.as_deref().unwrap_or("")).and_then(|sig| issuer(&sig)).and_then(|fp| if allowed.iter().any(|a| fp.ends_with(a.as_str()) || a.ends_with(fp.as_str())) { Ok(fp) } else { Err(format!("the image is signed by {fp}, not a key in the catalog")) })
            };
            match res { Ok(w) => how = format!("signed by {w}"), Err(err) => bad.push(format!("{}: signature: {err}", e.id)) }
        }
        match catalog::expected_sha256(&r, sums.as_deref()) { Ok(h) => how.push_str(&format!(", sha256 {}…", &h[..12])), Err(err) => bad.push(format!("{}: {err}", e.id)) }
        match fetch::head(&r.url) {
            Ok((200, len)) => lines.push(format!("{}: {} {} — {} ({}) — {how}", e.id, r.name, r.version, r.url, len.map(human).unwrap_or_else(|| "size not given".into()))),
            Ok((code, _)) => bad.push(format!("{}: {} answered {code}", e.id, r.url)),
            Err(err) => bad.push(format!("{}: {}: {err}", e.id, r.url)),
        }
    }
    (lines, bad)
}

/// The issuer fingerprint (or long key id) of a signature, from gpg --list-packets.
fn issuer(sig: &[u8]) -> Result<String, String> {
    use std::io::Write;
    let mut ch = std::process::Command::new("gpg").args(["--batch", "--list-packets"]).stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).spawn().map_err(|e| e.to_string())?;
    ch.stdin.take().unwrap().write_all(sig).map_err(|e| e.to_string())?;
    let o = ch.wait_with_output().map_err(|e| e.to_string())?;
    let t = String::from_utf8_lossy(&o.stdout).to_string() + &String::from_utf8_lossy(&o.stderr);
    let re = regex::Regex::new(r"issuer fpr v\d ([0-9A-F]{40})").unwrap();
    if let Some(c) = re.captures(&t) { return Ok(c[1].to_string()); }
    let re = regex::Regex::new(r"keyid ([0-9A-F]{16})").unwrap();
    re.captures(&t).map(|c| c[1].to_string()).ok_or_else(|| "the signature names no key".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Partition;

    fn drive(kind: &str, removable: bool, system: bool) -> Drive {
        Drive { id: format!("d-{kind}"), device: "/dev/sdx".into(), kind: kind.into(), removable, system, size: 16 << 30, model: "Stick".into(),
                partitions: vec![Partition { id: "p".into(), kind: "data".into(), what: "Files".into(), label: "STICK".into(), size: 16 << 30, used: Some(1 << 30), ..Default::default() }], ..Default::default() }
    }

    #[test]
    fn which_drives_are_offered() {
        assert!(eligible(&drive("usb", true, false), false));
        assert!(eligible(&drive("sd", true, false), false));
        assert!(!eligible(&drive("usb", true, true), true), "never the system's drive, even with Show all drives");
        assert!(!eligible(&drive("nvme", false, true), true));
        assert!(!eligible(&drive("ssd", false, false), false), "internal drives only with Show all drives");
        assert!(eligible(&drive("ssd", false, false), true));
        assert!(!eligible(&drive("optical", true, false), true));
        let mut ro = drive("usb", true, false); ro.read_only = true;
        assert!(!eligible(&ro, true));
        let mut empty = drive("sd", true, false); empty.size = 0;
        assert!(!eligible(&empty, false));
        std::env::remove_var("HEARTH_USB_ALLOW_LOOP");
        assert!(!eligible(&drive("image", true, false), true), "disk images only in CI");
        let u = usb_drive(&drive("ssd", false, false));
        assert!(u.internal);
        assert_eq!(u.partitions, vec!["\"STICK\" (Files), 17.2 GB, 1.1 GB used"]);
        assert!(!usb_drive(&drive("usb", true, false)).internal);
    }

    #[test]
    fn checksum_beside_your_file() {
        let dir = std::env::temp_dir().join(format!("hearth-usb-side-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let iso = dir.join("my.iso");
        std::fs::write(&iso, b"hello").unwrap();
        assert!(sidecar_checksum(&iso).is_none());
        let h = verify::sha256_bytes(b"hello");
        std::fs::write(dir.join("SHA256SUMS"), format!("{h}  my.iso\n{}  other.iso\n", "0".repeat(64))).unwrap();
        assert_eq!(sidecar_checksum(&iso).unwrap(), ("SHA256SUMS".to_string(), h.clone()));
        std::fs::remove_file(dir.join("SHA256SUMS")).unwrap();
        std::fs::write(dir.join("my.iso.sha256"), format!("{h}\n")).unwrap();
        assert_eq!(sidecar_checksum(&iso).unwrap().1, h);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn log_stamps() {
        let s = stamp();
        assert_eq!(s.len(), 19);
        assert!(s.starts_with("20"));
    }
}
