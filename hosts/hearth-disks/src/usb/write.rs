//! Writing to the USB drive. The only root steps go through Hearth's helper (`pkexec hearth-admin usb-*`, see
//! os/system_files/usr/libexec/nimbus/hearth-usb), which checks the drive again itself:
//!  - raw: Disks reads the image (unpacking .xz/.zst/.gz) and hands it to `usb-raw` on its standard input, hashing
//!    what it sends; the helper writes it, reads it back from the drive and both SHA-256s must match Disks' own.
//!    Root never opens a file you name.
//!  - files (Windows, or a UEFI ISO that isn't hybrid): `usb-prepare` makes one FAT32 partition and mounts it owned
//!    by you; Disks copies the ISO's files as you (splitting install.wim with wimlib when it's over 4 GB), adds
//!    autounattend.xml for Windows 11's options, checks the copies, and `usb-finish` flushes and unmounts it.
//! HEARTH_DISKS_ADMIN replaces "pkexec /usr/libexec/nimbus/hearth-admin" (CI runs the helper with sudo).

use super::{image, log, Emit, UsbDrive, CANCEL, CANCELLED};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

pub const ADMIN: &str = "/usr/libexec/nimbus/hearth-admin";

pub fn admin() -> Command {
    if let Ok(s) = std::env::var("HEARTH_DISKS_ADMIN") {
        let parts: Vec<&str> = s.split_whitespace().collect();
        if let Some((first, rest)) = parts.split_first() {
            let mut c = Command::new(first);
            c.args(rest);
            return c;
        }
    }
    let mut c = Command::new("pkexec");
    c.arg(ADMIN);
    c
}

/// The helper's arguments for a drive: the device, its size and serial as Disks showed them, and --allow-internal
/// for a drive that isn't removable (polkit then asks for an administrator's password).
pub fn drive_args(cmd: &str, d: &UsbDrive) -> Vec<String> {
    let mut a = vec![cmd.to_string(), d.device.clone(), "--expect-size".into(), d.size.to_string()];
    let serial_ok = !d.serial.is_empty() && d.serial.len() <= 64 && d.serial.chars().all(|c| c.is_ascii_alphanumeric() || " ._:-".contains(c));
    if serial_ok { a.push("--expect-serial".into()); a.push(d.serial.clone()); }
    if d.internal { a.push("--allow-internal".into()); }
    a
}

/// What the helper said: its last JSON line, or its error.
fn helper_result(out: &str, err: &str, ok: bool) -> Result<Value, String> {
    for l in out.lines().rev() {
        if let Some(e) = l.strip_prefix("error: ") { return Err(e.to_string()); }
        if l.starts_with('{') { if let Ok(v) = serde_json::from_str::<Value>(l) { return if ok || v["ok"] == true { Ok(v) } else { Err(format!("The helper reported a problem: {l}")) }; } }
    }
    let e = err.trim();
    if e.contains("dismissed") || e.contains("Not authorized") || e.contains("Request dismissed") { return Err("Cancelled: the administrator's password wasn't given. Nothing was changed.".into()); }
    Err(if e.is_empty() { "Hearth's system helper didn't answer (is this Hearth OS?).".into() } else { format!("Hearth's system helper failed: {}", e.lines().last().unwrap_or(e)) })
}

/// Runs a helper command to the end, passing its progress lines on.
fn helper(args: &[String], stage: &str, emit: &Emit) -> Result<Value, String> {
    log(&format!("helper: {}", args.join(" ")));
    let mut child = admin().args(args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()
        .map_err(|e| format!("Couldn't start Hearth's system helper ({e})"))?;
    let stdout = child.stdout.take().unwrap();
    let mut err = String::new();
    let mut stderr = child.stderr.take().unwrap();
    let errt = std::thread::spawn(move || { let mut s = String::new(); let _ = stderr.read_to_string(&mut s); s });
    let mut out = String::new();
    for line in BufReader::new(stdout).lines().map_while(Result::ok) {
        if let Some(p) = line.strip_prefix("progress: ") {
            let (pct, what) = p.split_once(' ').unwrap_or((p, ""));
            emit(json!({"stage": stage, "pct": pct.parse::<f64>().ok().map(|x| x / 100.0), "text": what}));
        }
        out.push_str(&line);
        out.push('\n');
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    if let Ok(s) = errt.join() { err = s; }
    let r = helper_result(&out, &err, status.success());
    log(&format!("helper: {} -> {}", args.first().cloned().unwrap_or_default(), match &r { Ok(v) => v.to_string(), Err(e) => format!("error: {e}") }));
    if !status.success() && r.is_ok() { return Err(format!("Hearth's system helper failed: {}", err.trim())); }
    r
}

/// The uncompressed size of an .xz image (xz --robot --list), when it can be known.
pub fn unpacked_size(path: &Path, format: &str) -> Option<u64> {
    match format {
        "iso" | "img" => std::fs::metadata(path).ok().map(|m| m.len()),
        "img.xz" => {
            let o = Command::new("xz").args(["--robot", "--list"]).arg(path).output().ok()?;
            String::from_utf8_lossy(&o.stdout).lines().find(|l| l.starts_with("totals")).and_then(|l| l.split('\t').nth(4)).and_then(|s| s.parse().ok())
        }
        _ => None,
    }
}

/// Writes an image to the whole drive, dd-style, and checks it by reading it back. Returns the SHA-256 written.
pub fn raw(src: &Path, format: &str, d: &UsbDrive, emit: Emit) -> Result<String, String> {
    let total = unpacked_size(src, format);
    if let Some(t) = total { if t > d.size { return Err(format!("The image ({}) doesn't fit on {} ({}).", crate::model::human(t), d.name, crate::model::human(d.size))); } }
    let mut args = drive_args("usb-raw", d);
    if let Some(t) = total { args.push("--size".into()); args.push(t.to_string()); }
    log(&format!("raw write of {} ({format}) to {}", src.display(), d.device));
    let mut child = admin().args(&args).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()
        .map_err(|e| format!("Couldn't start Hearth's system helper ({e})"))?;
    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();
    let errt = std::thread::spawn(move || { let mut s = String::new(); let _ = stderr.read_to_string(&mut s); s });
    let sent = Arc::new(AtomicU64::new(0));
    let (src2, fmt2, sent2, emit2) = (src.to_path_buf(), format.to_string(), sent.clone(), emit.clone());
    // The pump: read the image (unpacked), hash it, hand it to the helper.
    let pump = std::thread::spawn(move || -> Result<(String, u64), String> {
        let mut unpack = None;
        let mut reader: Box<dyn Read> = match image::unpacker(&fmt2) {
            Some((cmd, a)) => {
                let mut c = Command::new(cmd).args(&a).arg(&src2).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().map_err(|e| format!("{cmd} isn't installed ({e})"))?;
                let out = c.stdout.take().unwrap();
                unpack = Some(c);
                Box::new(out)
            }
            None => Box::new(std::fs::File::open(&src2).map_err(|e| format!("Couldn't read the image: {e}"))?),
        };
        let mut h = Sha256::new();
        let mut buf = vec![0u8; 4 << 20];
        let mut n = 0u64;
        let started = Instant::now();
        let mut last = Instant::now();
        let r = loop {
            if CANCEL.load(Ordering::SeqCst) { break Err(CANCELLED.to_string()); }
            let k = match reader.read(&mut buf) { Ok(0) => break Ok(()), Ok(k) => k, Err(e) => break Err(format!("Couldn't read the image: {e}")) };
            h.update(&buf[..k]);
            if let Err(e) = stdin.write_all(&buf[..k]) { break Err(format!("The helper stopped taking the image ({e})")); }
            n += k as u64;
            sent2.store(n, Ordering::SeqCst);
            if last.elapsed().as_millis() > 500 {
                last = Instant::now();
                let speed = n as f64 / started.elapsed().as_secs_f64().max(0.1);
                let eta = total.map(|t| (t.saturating_sub(n) as f64 / speed.max(1.0)) as u64);
                emit2(json!({"stage": "write", "pct": total.map(|t| n as f64 / t.max(1) as f64), "done": n, "total": total, "speed": speed, "eta": eta, "text": "Writing"}));
            }
        };
        drop(stdin);
        if let Some(mut c) = unpack { if r.is_err() { let _ = c.kill(); } let st = c.wait(); if r.is_ok() && !st.map(|s| s.success()).unwrap_or(false) { return Err("The image couldn't be unpacked (is it damaged?).".into()); } }
        r.map(|_| (super::verify::hex(&h.finalize()), n))
    });
    let mut out = String::new();
    for line in BufReader::new(stdout).lines().map_while(Result::ok) {
        if let Some(p) = line.strip_prefix("progress: ") {
            let (pct, what) = p.split_once(' ').unwrap_or((p, ""));
            if what == "verifying" || what == "flushing" {
                emit(json!({"stage": "readback", "pct": pct.parse::<f64>().ok().map(|x| x / 100.0), "text": if what == "flushing" { "Finishing the write" } else { "Reading it back to check" }}));
            }
        }
        out.push_str(&line);
        out.push('\n');
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    let err = errt.join().unwrap_or_default();
    let pumped = pump.join().map_err(|_| "The image reader stopped unexpectedly.".to_string())?;
    let (sha, n) = pumped?;
    let v = helper_result(&out, &err, status.success())?;
    log(&format!("raw write: sent {n} bytes, sha256 {sha}; helper said {v}"));
    if v["written"].as_u64() != Some(n) { return Err(format!("The drive took {} bytes of {n}. Write it again.", v["written"])); }
    if v["writtenSha256"].as_str() != Some(sha.as_str()) { return Err("What reached the drive isn't what Disks sent. Write it again, or try another USB port or drive.".into()); }
    if v["readbackSha256"].as_str() != Some(sha.as_str()) { return Err("Reading the drive back gave something different from what was written: the USB drive may be failing or fake. Try another one.".into()); }
    Ok(sha)
}

/// An ISO mounted read-only through udisks (as you); unmounted and let go when dropped.
pub struct IsoMount { pub dev: String, pub root: PathBuf }

impl Drop for IsoMount {
    fn drop(&mut self) {
        let _ = Command::new("udisksctl").args(["unmount", "--no-user-interaction", "-b", &self.dev]).output();
        let _ = Command::new("udisksctl").args(["loop-delete", "--no-user-interaction", "-b", &self.dev]).output();
    }
}

fn mount_point_of(dev: &str) -> Option<PathBuf> {
    let o = Command::new("findmnt").args(["-n", "-o", "TARGET", "--source", dev]).output().ok()?;
    String::from_utf8_lossy(&o.stdout).lines().next().map(|s| PathBuf::from(s.trim())).filter(|p| p.is_dir())
}

pub fn mount_iso(path: &Path) -> Result<IsoMount, String> {
    let o = Command::new("udisksctl").args(["loop-setup", "--no-user-interaction", "-r", "-f"]).arg(path).output()
        .map_err(|e| format!("udisks2 isn't installed ({e}), so the ISO can't be opened"))?;
    let text = String::from_utf8_lossy(&o.stdout).to_string() + &String::from_utf8_lossy(&o.stderr);
    let dev = regex::Regex::new(r"/dev/loop[0-9]+").unwrap().find(&text).map(|m| m.as_str().trim_end_matches('.').to_string())
        .ok_or_else(|| format!("The ISO couldn't be opened: {}", text.trim()))?;
    let m = IsoMount { dev: dev.clone(), root: PathBuf::new() };
    // udisks may mount it by itself; give it a moment, then mount it ourselves.
    let mut root = None;
    for _ in 0..10 { if let Some(p) = mount_point_of(&dev) { root = Some(p); break; } std::thread::sleep(std::time::Duration::from_millis(200)); }
    if root.is_none() {
        let o = Command::new("udisksctl").args(["mount", "--no-user-interaction", "-o", "ro", "-b", &dev]).output().map_err(|e| e.to_string())?;
        let t = String::from_utf8_lossy(&o.stdout).to_string();
        root = t.split(" at ").nth(1).map(|s| PathBuf::from(s.trim().trim_end_matches('.'))).filter(|p| p.is_dir()).or_else(|| mount_point_of(&dev));
        if root.is_none() { return Err(format!("The ISO couldn't be opened: {}{}", t.trim(), String::from_utf8_lossy(&o.stderr).trim())); }
    }
    Ok(IsoMount { root: root.unwrap(), ..m })
}

/// Every file under `root` (relative paths) and the bytes they add up to.
fn tree(root: &Path) -> Result<(Vec<(PathBuf, u64)>, Vec<PathBuf>), String> {
    let mut files = vec![];
    let mut dirs = vec![];
    let mut stack = vec![PathBuf::new()];
    while let Some(rel) = stack.pop() {
        let rd = std::fs::read_dir(root.join(&rel)).map_err(|e| format!("Couldn't read the ISO: {e}"))?;
        for e in rd.flatten() {
            let r = rel.join(e.file_name());
            let ft = e.file_type().map_err(|e| e.to_string())?;
            if ft.is_dir() { dirs.push(r.clone()); stack.push(r); }
            else if ft.is_file() { files.push((r, e.metadata().map(|m| m.len()).unwrap_or(0))); }
        }
    }
    files.sort();
    dirs.sort();
    Ok((files, dirs))
}

fn copy_file(from: &Path, to: &Path, done: &mut u64, total: u64, started: Instant, last: &mut Instant, emit: &Emit) -> Result<(), String> {
    let mut r = std::fs::File::open(from).map_err(|e| format!("Couldn't read {}: {e}", from.display()))?;
    let mut w = std::fs::File::create(to).map_err(|e| format!("Couldn't write {} on the USB drive: {e}", to.display()))?;
    let mut buf = vec![0u8; 4 << 20];
    loop {
        if CANCEL.load(Ordering::SeqCst) { return Err(CANCELLED.into()); }
        let n = r.read(&mut buf).map_err(|e| format!("Couldn't read {}: {e}", from.display()))?;
        if n == 0 { break; }
        w.write_all(&buf[..n]).map_err(|e| format!("Couldn't write to the USB drive ({e}). Is it full or failing?"))?;
        *done += n as u64;
        if last.elapsed().as_millis() > 500 {
            *last = Instant::now();
            let speed = *done as f64 / started.elapsed().as_secs_f64().max(0.1);
            emit(json!({"stage": "copy", "pct": *done as f64 / total.max(1) as f64, "done": *done, "total": total, "speed": speed, "eta": (total.saturating_sub(*done) as f64 / speed.max(1.0)) as u64, "text": "Copying the files"}));
        }
    }
    w.sync_all().map_err(|e| format!("Couldn't finish writing to the USB drive: {e}"))?;
    Ok(())
}

fn sha_file(p: &Path) -> Option<String> {
    let mut f = std::fs::File::open(p).ok()?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop { let n = f.read(&mut buf).ok()?; if n == 0 { break; } h.update(&buf[..n]); }
    Some(super::verify::hex(&h.finalize()))
}

/// The FAT32 label for the drive: from the ISO's label (Windows: CCCOMA_X64FRE_EN-US_DV9 → "CCCOMA_X64F"), capitals,
/// digits, - and _ only, at most 11.
pub fn fat_label(iso_label: &str, fallback: &str) -> String {
    let s: String = iso_label.to_uppercase().chars().filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-').take(11).collect();
    if s.is_empty() { fallback.to_string() } else { s }
}

pub struct FilesJob<'a> {
    pub iso_root: &'a Path,
    pub windows: Option<&'a image::WinInfo>,
    pub scheme: &'a str,
    pub label: &'a str,
    pub unattend: Option<String>,
}

/// FAT32 and the ISO's files. Returns a summary for the log.
pub fn files(j: &FilesJob, d: &UsbDrive, emit: Emit) -> Result<Value, String> {
    let (all, dirs) = tree(j.iso_root)?;
    let split = j.windows.map(|w| w.needs_split).unwrap_or(false);
    let wim_rel = PathBuf::from("sources").join("install.wim");
    let is_wim = |p: &Path| p.to_string_lossy().eq_ignore_ascii_case(&wim_rel.to_string_lossy());
    let limit = image::split_limit();
    if let Some((p, s)) = all.iter().find(|(p, s)| *s > limit && !(split && is_wim(p))) {
        return Err(format!("{} is {}: too big for FAT32 and it can't be split. This ISO can't be written this way.", p.display(), crate::model::human(*s)));
    }
    let total: u64 = all.iter().map(|(_, s)| *s).sum();
    if total > d.size { return Err(format!("The files ({}) don't fit on {} ({}).", crate::model::human(total), d.name, crate::model::human(d.size))); }
    emit(json!({"stage": "prepare", "pct": null, "text": "Erasing the drive and making a FAT32 partition"}));
    let prep = helper(&{ let mut a = drive_args("usb-prepare", d); a.extend(["--scheme".into(), j.scheme.into(), "--label".into(), j.label.into()]); a }, "prepare", &emit)?;
    let mount = PathBuf::from(prep["mount"].as_str().ok_or("The helper didn't say where it mounted the drive")?);
    let result = (|| -> Result<Value, String> {
        for dir in &dirs { std::fs::create_dir_all(mount.join(dir)).map_err(|e| format!("Couldn't make {} on the USB drive: {e}", dir.display()))?; }
        let started = Instant::now();
        let mut last = Instant::now();
        let mut done = 0u64;
        let mut copied = vec![];
        for (rel, _) in &all {
            if split && is_wim(rel) { continue; }
            copy_file(&j.iso_root.join(rel), &mount.join(rel), &mut done, total, started, &mut last, &emit)?;
            copied.push(rel.clone());
        }
        let mut parts = 0;
        if split {
            let src = all.iter().find(|(p, _)| is_wim(p)).map(|(p, _)| j.iso_root.join(p)).ok_or("install.wim went missing")?;
            let part_mb = if std::env::var("HEARTH_USB_SPLIT_MB").is_ok() { (limit / 1024 / 1024).saturating_sub(1).max(1) } else { 4000 };
            emit(json!({"stage": "split", "pct": null, "text": "Splitting install.wim into parts FAT32 can hold (this takes a while)"}));
            let dst = mount.join("sources").join("install.swm");
            log(&format!("wimlib-imagex split {} {} {part_mb}", src.display(), dst.display()));
            let o = Command::new("wimlib-imagex").arg("split").arg(&src).arg(&dst).arg(part_mb.to_string()).output()
                .map_err(|e| format!("wimlib isn't installed ({e}), so Windows' big install.wim can't be split"))?;
            if !o.status.success() { return Err(format!("Splitting install.wim didn't work: {}", String::from_utf8_lossy(&o.stderr).trim())); }
            parts = std::fs::read_dir(mount.join("sources")).map(|rd| rd.flatten().filter(|e| e.file_name().to_string_lossy().to_lowercase().ends_with(".swm")).count()).unwrap_or(0);
            if parts == 0 { return Err("Splitting install.wim made no parts.".into()); }
        }
        if let Some(x) = &j.unattend {
            std::fs::write(mount.join("autounattend.xml"), x).map_err(|e| format!("Couldn't write autounattend.xml: {e}"))?;
        }
        // Check: every file there with its size, and the start-up files byte for byte.
        emit(json!({"stage": "readback", "pct": null, "text": "Checking the copied files"}));
        for (rel, size) in &all {
            if split && is_wim(rel) { continue; }
            let got = std::fs::metadata(mount.join(rel)).map(|m| m.len()).map_err(|_| format!("{} is missing on the USB drive", rel.display()))?;
            if got != *size { return Err(format!("{} on the USB drive is {got} bytes, not {size}. Write it again.", rel.display())); }
        }
        for key in ["efi/boot/bootx64.efi", "efi/boot/bootaa64.efi", "bootmgr", "bootmgr.efi", "sources/boot.wim"] {
            if let Some(src) = image::find_ci(j.iso_root, key) {
                let rel = src.strip_prefix(j.iso_root).unwrap_or(&src).to_path_buf();
                if sha_file(&src) != sha_file(&mount.join(&rel)) { return Err(format!("{} on the USB drive isn't the same as on the ISO. Write it again.", rel.display())); }
            }
        }
        Ok(json!({"files": copied.len(), "bytes": done, "swmParts": parts, "unattend": j.unattend.is_some()}))
    })();
    emit(json!({"stage": "finish", "pct": null, "text": "Flushing and checking the file system"}));
    let fin = helper(&[String::from("usb-finish"), d.device.clone()], "finish", &emit);
    let summary = result?;
    let fin = fin?;
    if fin["ok"] != true { return Err(format!("The FAT32 check found problems: {}", fin["fsck"].as_str().unwrap_or(""))); }
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels() {
        assert_eq!(fat_label("CCCOMA_X64FRE_EN-US_DV9", "WINUSB"), "CCCOMA_X64F");
        assert_eq!(fat_label("", "WINUSB"), "WINUSB");
        assert_eq!(fat_label("Fedora 44 x", "USB"), "FEDORA44X");
    }

    #[test]
    fn helper_answers() {
        assert_eq!(helper_result("progress: 50 writing\n{\"ok\": true, \"written\": 5}\n", "", true).unwrap()["written"], 5);
        assert_eq!(helper_result("error: The drive is in use\n", "", false).unwrap_err(), "The drive is in use");
        assert!(helper_result("", "Error executing command as another user: Request dismissed\n", false).unwrap_err().contains("Cancelled"));
        assert!(helper_result("", "", false).is_err());
    }

    #[test]
    fn arguments() {
        let d = UsbDrive { device: "/dev/sdb".into(), size: 16, serial: "AA00 11".into(), ..Default::default() };
        assert_eq!(drive_args("usb-raw", &d), vec!["usb-raw", "/dev/sdb", "--expect-size", "16", "--expect-serial", "AA00 11"]);
        let d = UsbDrive { device: "/dev/sda".into(), size: 1, serial: "weird;serial".into(), internal: true, ..Default::default() };
        assert_eq!(drive_args("usb-prepare", &d), vec!["usb-prepare", "/dev/sda", "--expect-size", "1", "--allow-internal"]);
    }
}
