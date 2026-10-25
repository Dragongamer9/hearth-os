//! Checking downloads: SHA-256 of a file (with progress, and it can be cancelled), and OpenPGP signatures with gpg
//! against the keys built into Disks (keys.rs), in a keyring of its own that's thrown away afterwards. A signature
//! only counts when gpg says it's good (VALIDSIG) and the key that made it is one the catalog names for it.

use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// SHA-256 of a file as lower-case hex. `progress(done, total)` now and then; stops when `cancel` is set.
pub fn sha256_file(path: &Path, cancel: &AtomicBool, progress: &dyn Fn(u64, u64)) -> Result<String, String> {
    let mut f = std::fs::File::open(path).map_err(|e| format!("Couldn't read {}: {e}", path.display()))?;
    let total = f.metadata().map(|m| m.len()).unwrap_or(0);
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 4 << 20];
    let mut done = 0u64;
    let mut last = std::time::Instant::now();
    loop {
        if cancel.load(Ordering::SeqCst) { return Err(super::CANCELLED.into()); }
        let n = f.read(&mut buf).map_err(|e| format!("Couldn't read {}: {e}", path.display()))?;
        if n == 0 { break; }
        h.update(&buf[..n]);
        done += n as u64;
        if last.elapsed().as_millis() > 300 { last = std::time::Instant::now(); progress(done, total); }
    }
    progress(done, total);
    Ok(hex(&h.finalize()))
}

pub fn hex(b: &[u8]) -> String { b.iter().map(|x| format!("{x:02x}")).collect() }

pub fn sha256_bytes(b: &[u8]) -> String { hex(&Sha256::digest(b)) }

static N: AtomicU64 = AtomicU64::new(0);

/// A private gpg home with the given keys in it; removed when dropped.
struct Ring { dir: PathBuf }
impl Drop for Ring { fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.dir); } }

fn ring(keys: &[&str]) -> Result<Ring, String> {
    let dir = std::env::temp_dir().join(format!("hearth-usb-gpg-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst)));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).map_err(|e| format!("Couldn't make a keyring folder: {e}"))?;
    #[cfg(unix)]
    { use std::os::unix::fs::PermissionsExt; let _ = std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)); }
    let r = Ring { dir };
    for (i, k) in keys.iter().enumerate() {
        let f = r.dir.join(format!("key{i}.asc"));
        std::fs::write(&f, k).map_err(|e| e.to_string())?;
        let out = gpg(&r.dir).args(["--import"]).arg(&f).output().map_err(|e| format!("gpg isn't installed ({e}), so the signature can't be checked"))?;
        if !out.status.success() { return Err(format!("gpg couldn't read a built-in key: {}", String::from_utf8_lossy(&out.stderr).trim())); }
    }
    Ok(r)
}

fn gpg(home: &Path) -> Command {
    let mut c = Command::new("gpg");
    c.arg("--homedir").arg(home).args(["--batch", "--no-tty", "--quiet", "--no-auto-key-retrieve", "--no-autostart"]);
    c.env("LC_ALL", "C");
    c
}

/// The fingerprint gpg vouches for in its --status-fd output, if the signature is good and made by one of `allowed`.
pub fn signer(status: &str, allowed: &[String]) -> Result<String, String> {
    let mut good = false;
    let mut valid: Option<(String, String)> = None;
    for l in status.lines() {
        let f: Vec<&str> = l.split_whitespace().collect();
        if f.len() < 2 || f[0] != "[GNUPG:]" { continue; }
        match f[1] {
            "GOODSIG" => good = true,
            "BADSIG" => return Err("The signature doesn't match: this file isn't what the project signed.".into()),
            "EXPKEYSIG" | "REVKEYSIG" => return Err("The key that signed this has expired or been revoked.".into()),
            "ERRSIG" | "NO_PUBKEY" => return Err("It's signed with a key Hearth doesn't know.".into()),
            "VALIDSIG" if f.len() >= 3 => {
                let sub = f[2].to_uppercase();
                let primary = f.last().map(|s| s.to_uppercase()).filter(|s| s.len() == 40).unwrap_or_else(|| sub.clone());
                valid = Some((sub, primary));
            }
            _ => {}
        }
    }
    let (sub, primary) = valid.filter(|_| good).ok_or("The signature couldn't be checked.")?;
    let ok = |fp: &str| allowed.iter().any(|a| a.eq_ignore_ascii_case(fp));
    if ok(&primary) { Ok(primary) } else if ok(&sub) { Ok(sub) } else { Err(format!("It's signed by a key ({primary}) that isn't the one this project uses.")) }
}

/// Checks a detached signature over `data`. Returns the signer's fingerprint.
pub fn detached_bytes(keys: &[&str], allowed: &[String], data: &[u8], sig: &[u8]) -> Result<String, String> {
    let r = ring(keys)?;
    let d = r.dir.join("data");
    std::fs::write(&d, data).map_err(|e| e.to_string())?;
    detached_file(&r, allowed, &d, sig)
}

/// Checks a detached signature over a file on disk (an image: gpg reads it, nothing is copied).
pub fn detached_path(keys: &[&str], allowed: &[String], file: &Path, sig: &[u8]) -> Result<String, String> {
    let r = ring(keys)?;
    detached_file(&r, allowed, file, sig)
}

fn detached_file(r: &Ring, allowed: &[String], file: &Path, sig: &[u8]) -> Result<String, String> {
    let s = r.dir.join("data.sig");
    std::fs::write(&s, sig).map_err(|e| e.to_string())?;
    let out = gpg(&r.dir).args(["--status-fd", "1", "--verify"]).arg(&s).arg(file).output().map_err(|e| format!("gpg isn't installed ({e})"))?;
    let status = String::from_utf8_lossy(&out.stdout);
    let who = signer(&status, allowed)?;
    if !out.status.success() { return Err("gpg didn't accept the signature.".into()); }
    Ok(who)
}

/// Checks a clearsigned text (Fedora's CHECKSUM). Returns (signer, the text that was signed): only that text is used.
pub fn clearsigned(keys: &[&str], allowed: &[String], text: &[u8]) -> Result<(String, String), String> {
    let r = ring(keys)?;
    let m = r.dir.join("msg.asc");
    let p = r.dir.join("plain");
    std::fs::write(&m, text).map_err(|e| e.to_string())?;
    let out = gpg(&r.dir).args(["--status-fd", "1", "--output"]).arg(&p).arg("--decrypt").arg(&m).output().map_err(|e| format!("gpg isn't installed ({e})"))?;
    let who = signer(&String::from_utf8_lossy(&out.stdout), allowed)?;
    if !out.status.success() { return Err("gpg didn't accept the signature.".into()); }
    let plain = std::fs::read_to_string(&p).map_err(|e| format!("Couldn't read the signed text: {e}"))?;
    Ok((who, plain))
}

#[cfg(test)]
mod tests {
    use super::*;
    const FP: &str = "843938DF228D22F7B3742BC0D94AA3F0EFE21092";

    #[test]
    fn reads_gpg_status() {
        let allowed = vec![FP.to_string()];
        let ok = format!("[GNUPG:] NEWSIG\n[GNUPG:] GOODSIG D94AA3F0EFE21092 Ubuntu\n[GNUPG:] VALIDSIG {FP} 2026-10-15 1 0 4 0 1 10 00 {FP}\n");
        assert_eq!(signer(&ok, &allowed).unwrap(), FP);
        // A subkey's signature: the primary key (last field) is what's matched.
        let sub = format!("[GNUPG:] GOODSIG X Y\n[GNUPG:] VALIDSIG 1111111111111111111111111111111111111111 2026 1 0 4 0 1 10 00 {FP}\n");
        assert_eq!(signer(&sub, &allowed).unwrap(), FP);
        let other = "[GNUPG:] GOODSIG X Y\n[GNUPG:] VALIDSIG 2222222222222222222222222222222222222222 2026 1 0 4 0 1 10 00 2222222222222222222222222222222222222222\n";
        assert!(signer(other, &allowed).unwrap_err().contains("isn't the one"));
        assert!(signer("[GNUPG:] BADSIG X Y\n", &allowed).is_err());
        assert!(signer("[GNUPG:] ERRSIG X 1 8 00 1 9\n[GNUPG:] NO_PUBKEY X\n", &allowed).is_err());
        assert!(signer(&format!("[GNUPG:] EXPKEYSIG X Y\n[GNUPG:] VALIDSIG {FP} x\n"), &allowed).is_err());
        assert!(signer("", &allowed).is_err(), "no status at all isn't a pass");
        // VALIDSIG without GOODSIG isn't enough.
        assert!(signer(&format!("[GNUPG:] VALIDSIG {FP} 2026 1 0 4 0 1 10 00 {FP}\n"), &allowed).is_err());
    }

    #[test]
    fn hashes() {
        assert_eq!(sha256_bytes(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        let p = std::env::temp_dir().join(format!("hearth-usb-hash-{}", std::process::id()));
        std::fs::write(&p, b"abc").unwrap();
        let cancel = AtomicBool::new(false);
        assert_eq!(sha256_file(&p, &cancel, &|_, _| {}).unwrap(), sha256_bytes(b"abc"));
        cancel.store(true, Ordering::SeqCst);
        assert!(sha256_file(&p, &cancel, &|_, _| {}).is_err());
        let _ = std::fs::remove_file(p);
    }

    #[test]
    fn every_built_in_key_imports() {
        if Command::new("gpg").arg("--version").output().is_err() { eprintln!("no gpg: skipped"); return; }
        let c = super::super::catalog::load();
        for (name, k) in &c.keys {
            let text = super::super::keys::text(&k.file).unwrap();
            let r = ring(&[text]).unwrap_or_else(|e| panic!("{name}: {e}"));
            let out = gpg(&r.dir).args(["--with-colons", "--fingerprint", "--list-keys"]).output().unwrap();
            let listed = String::from_utf8_lossy(&out.stdout).to_string();
            for fp in &k.fingerprints { assert!(listed.contains(fp.as_str()), "{name}: {fp} isn't in usb/keys/{}", k.file); }
        }
    }
}
