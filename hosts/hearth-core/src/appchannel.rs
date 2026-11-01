//! Hearth app updates without a system update (docs/APP-UPDATES.md).
//!
//! Hearth's own app (the `nimbus` program with its screens built in, and its Chromium window app) can update by
//! itself in minutes, between Hearth OS system updates. The version built into the system image is never touched and
//! always stays the fallback. Everything here runs as you and only ever writes in ~/.local/share/hearth/app:
//!
//!   current -> <version>     the app bundle in use (a relative symlink, switched atomically after checking)
//!   previous -> <version>    the one before it (kept)
//!   <version>/               an unpacked bundle: nimbus, electron/, manifest.json, manifest.json.sig
//!   staging/                 downloads and unpacking in progress
//!   marks/trying             the start in progress: "<version>\n<failed starts in a row>\n<since ms>"
//!   marks/good/<version>     this version brought Hearth's screen up and ran for a minute
//!   marks/bad/<version>      this version is never used again (why, inside)
//!   marks/last-fallback      the last time Hearth went back to the built-in version, and why
//!
//! A bundle is only started when ALL of these hold (`decide`): its manifest is signed by a key the image trusts
//! (/usr/share/nimbus/app-signing.pub), it was made for this edition, processor and image ABI
//! (/usr/share/nimbus/app-abi), it's newer than the image's own Hearth (/usr/share/nimbus/version), it isn't marked
//! bad, its files are there with the right sizes and its program has the right SHA-256. Game Mode's starts are
//! counted: a start with a bundle that never reaches "good" twice in a row marks that bundle bad, and the built-in
//! Hearth starts instead (said once on the next screen and in Settings › Updates).
//!
//! The decision is made by the image's own `nimbus` (/usr/lib/nimbus/nimbus, from the session's watchdog), which then
//! hands over to the bundle's program (`handoff`). A bundle can't change how it's chosen.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

/// What the signatures are made for (ssh-keygen -Y sign -n hearth-app).
pub const NAMESPACE: &str = "hearth-app";
/// A bundle that ran this long with Hearth's screen up is good.
pub const GOOD_AFTER_SECS: u64 = 60;
/// Starts in a row that never reached good before a bundle is marked bad.
pub const FAILS_TO_BAD: u32 = 2;
const REPO: &str = "Dragongamer9/hearth-os";
/// The image's own Hearth (the only program allowed to choose).
const BUILTIN_EXE: &str = "/usr/lib/nimbus/nimbus";

fn now_ms() -> u64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0) }
fn log(t: &str) { crate::bridge::log(&format!("app updates: {t}")); }

// ------------------------------------------------------------------------------------------------------------ paths

/// ~/.local/share/hearth/app (or $XDG_DATA_HOME/hearth/app)
pub fn root() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|| crate::paths::home().join(".local/share")).join("hearth/app")
}
/// Where the image keeps Hearth's facts (/usr/share/nimbus). HEARTH_APP_IMAGE_DIR stands in for it in tests.
pub fn image_dir() -> PathBuf { std::env::var_os("HEARTH_APP_IMAGE_DIR").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/usr/share/nimbus")) }
fn mode_file() -> PathBuf { crate::paths::config_dir().join("app-updates") }
fn marks(root: &Path) -> PathBuf { root.join("marks") }

// --------------------------------------------------------------------------------------------------------- versions

/// "0.6.61" or "0.6.61+app.3": the numbers, then the app build number (0 for none). Anything after a '-' is ignored.
pub fn parse_version(v: &str) -> (Vec<u64>, u64) {
    let v = v.trim().trim_start_matches('v');
    let (base, extra) = match v.split_once('+') { Some((b, e)) => (b, e), None => (v, "") };
    let base = base.split('-').next().unwrap_or("");
    let nums: Vec<u64> = base.split('.').map(|p| p.parse().unwrap_or(0)).collect();
    let app = extra.trim_start_matches("app").trim_start_matches(['.', '-']).parse().unwrap_or(0);
    (if nums.is_empty() { vec![0] } else { nums }, app)
}
pub fn cmp_versions(a: &str, b: &str) -> std::cmp::Ordering {
    let ((na, xa), (nb, xb)) = (parse_version(a), parse_version(b));
    for i in 0..na.len().max(nb.len()) {
        let (x, y) = (na.get(i).copied().unwrap_or(0), nb.get(i).copied().unwrap_or(0));
        if x != y { return x.cmp(&y); }
    }
    xa.cmp(&xb)
}
pub fn newer(a: &str, b: &str) -> bool { cmp_versions(a, b) == std::cmp::Ordering::Greater }
/// A version usable as a folder name: digits, letters, '.', '+', '-', '_' only.
pub fn safe_version(v: &str) -> bool {
    !v.is_empty() && v.len() <= 64 && !v.starts_with('.') && v.chars().all(|c| c.is_ascii_alphanumeric() || ".+-_".contains(c))
}

// --------------------------------------------------------------------------------------------------- signatures

/// Standard base64 (whitespace ignored).
pub fn b64decode(s: &str) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let (mut buf, mut bits) = (0u32, 0u32);
    for c in s.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' => break,
            b' ' | b'\n' | b'\r' | b'\t' => continue,
            _ => return None,
        } as u32;
        buf = (buf << 6) | v;
        bits += 6;
        if bits >= 8 { bits -= 8; out.push((buf >> bits) as u8); buf &= (1 << bits) - 1; }
    }
    Some(out)
}

/// Reads SSH wire format: big-endian u32 lengths before strings.
struct Rd<'a>(&'a [u8]);
impl<'a> Rd<'a> {
    fn bytes(&mut self, n: usize) -> Option<&'a [u8]> {
        if self.0.len() < n { return None; }
        let (a, b) = self.0.split_at(n);
        self.0 = b;
        Some(a)
    }
    fn u32(&mut self) -> Option<u32> { self.bytes(4).map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]])) }
    fn string(&mut self) -> Option<&'a [u8]> { let n = self.u32()? as usize; self.bytes(n) }
}
fn sshstr(out: &mut Vec<u8>, s: &[u8]) { out.extend_from_slice(&(s.len() as u32).to_be_bytes()); out.extend_from_slice(s); }

/// An ssh-ed25519 public key blob: its 32 bytes.
fn ed25519_blob(blob: &[u8]) -> Option<[u8; 32]> {
    let mut r = Rd(blob);
    if r.string()? != b"ssh-ed25519" { return None; }
    let k = r.string()?;
    k.try_into().ok()
}

/// The trusted keys in an OpenSSH public key file ("ssh-ed25519 AAAA… comment" lines; '#' lines and anything else
/// are skipped). The image's placeholder file has none, and then nothing is ever trusted.
pub fn parse_pubkeys(text: &str) -> Vec<[u8; 32]> {
    text.lines().filter_map(|l| {
        let mut w = l.split_whitespace();
        if w.next()? != "ssh-ed25519" { return None; }
        ed25519_blob(&b64decode(w.next()?)?)
    }).collect()
}

/// Checks an `ssh-keygen -Y sign` signature (the SSHSIG format, OpenSSH's PROTOCOL.sshsig) of `msg`, made with one of
/// `keys` for `namespace`.
pub fn verify_sshsig(msg: &[u8], armored: &str, namespace: &str, keys: &[[u8; 32]]) -> Result<(), String> {
    use ed25519_dalek::{Signature, VerifyingKey};
    use sha2::Digest;
    let body: String = armored.lines().map(str::trim)
        .skip_while(|l| *l != "-----BEGIN SSH SIGNATURE-----").skip(1)
        .take_while(|l| *l != "-----END SSH SIGNATURE-----").collect();
    let blob = b64decode(&body).filter(|b| !b.is_empty()).ok_or("the signature file isn't an SSH signature")?;
    let bad = || "the signature file is damaged".to_string();
    let mut r = Rd(&blob);
    if r.bytes(6).ok_or_else(bad)? != b"SSHSIG" { return Err(bad()); }
    if r.u32().ok_or_else(bad)? != 1 { return Err("the signature is a version this Hearth doesn't know".into()); }
    let pk = r.string().ok_or_else(bad)?;
    let ns = r.string().ok_or_else(bad)?;
    let reserved = r.string().ok_or_else(bad)?;
    let hash = r.string().ok_or_else(bad)?;
    let sig = r.string().ok_or_else(bad)?;
    if ns != namespace.as_bytes() { return Err("the signature was made for something else".into()); }
    let key = ed25519_blob(pk).ok_or("the signature isn't an Ed25519 one")?;
    if !keys.contains(&key) { return Err("the signature was made with a key this Hearth OS doesn't trust".into()); }
    let h: Vec<u8> = match hash {
        b"sha512" => sha2::Sha512::digest(msg).to_vec(),
        b"sha256" => sha2::Sha256::digest(msg).to_vec(),
        _ => return Err("the signature uses an unknown hash".into()),
    };
    let mut sr = Rd(sig);
    if sr.string().ok_or_else(bad)? != b"ssh-ed25519" { return Err(bad()); }
    let raw: [u8; 64] = sr.string().ok_or_else(bad)?.try_into().map_err(|_| bad())?;
    let mut signed = b"SSHSIG".to_vec();
    sshstr(&mut signed, ns);
    sshstr(&mut signed, reserved);
    sshstr(&mut signed, hash);
    sshstr(&mut signed, &h);
    let vk = VerifyingKey::from_bytes(&key).map_err(|_| "the signing key is damaged".to_string())?;
    vk.verify_strict(&signed, &Signature::from_bytes(&raw)).map_err(|_| "the signature doesn't match (the file was changed, or signed by someone else)".to_string())
}

pub fn sha256_file(p: &Path) -> std::io::Result<String> {
    use sha2::Digest;
    use std::io::Read;
    let mut f = std::fs::File::open(p)?;
    let mut h = sha2::Sha256::new();
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 { break; }
        h.update(&buf[..n]);
    }
    Ok(h.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

// ----------------------------------------------------------------------------------------------- image & manifest

/// What the running system image says about its own Hearth.
#[derive(Debug, Clone)]
pub struct Image { pub version: String, pub abi: String, pub edition: String, pub arch: String, pub keys: Vec<[u8; 32]> }

/// The image's facts from `dir`, or why app updates can't work on it.
pub fn image_at(dir: &Path) -> Result<Image, String> {
    let read = |n: &str| std::fs::read_to_string(dir.join(n)).map(|s| s.trim().to_string()).unwrap_or_default();
    let abi = read("app-abi");
    if abi.is_empty() { return Err("this Hearth OS version can't take Hearth app updates (no app ABI in the image)".into()); }
    let version = read("version");
    if version.is_empty() || version == "0.0.0" { return Err("this Hearth OS image doesn't say its version".into()); }
    let info: Value = serde_json::from_str(&read("image-info.json")).unwrap_or(Value::Null);
    let edition = info["base"].as_str().unwrap_or("").to_string();
    if edition.is_empty() { return Err("this Hearth OS image doesn't say its edition".into()); }
    let keys = parse_pubkeys(&read("app-signing.pub"));
    if keys.is_empty() { return Err("this Hearth OS image has no signing key for Hearth app updates yet".into()); }
    Ok(Image { version, abi, edition, arch: std::env::consts::ARCH.to_string(), keys })
}
pub fn image() -> Result<Image, String> { image_at(&image_dir()) }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileEntry {
    pub path: String,
    #[serde(default)] pub sha256: String,
    #[serde(default)] pub size: u64,
    #[serde(default)] pub mode: u32,
    /// A symlink (its target, relative, inside the bundle) instead of a file.
    #[serde(default, skip_serializing_if = "Option::is_none")] pub link: Option<String>,
}

/// manifest.json, as tools/app-bundle.py writes it (release.yml signs it).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub format: u32,
    pub version: String,
    pub edition: String,
    pub arch: String,
    pub abi: String,
    #[serde(default)] pub min_image_version: String,
    #[serde(default)] pub max_image_version: String,
    /// The archive (hearth-app-<edition>-<arch>.tar.zst): its SHA-256 and size.
    pub sha256: String,
    pub size: u64,
    #[serde(default)] pub archive: String,
    #[serde(default)] pub commit: String,
    pub files: Vec<FileEntry>,
}

/// A path inside a bundle: relative, no "..", nothing odd.
pub fn safe_rel(p: &str) -> bool {
    !p.is_empty() && !p.contains('\0') && !p.starts_with('/')
        && Path::new(p).components().all(|c| matches!(c, Component::Normal(_)))
}

/// Can this bundle run on this image? (The signature is checked before this, by `verify_manifest`.)
pub fn check_manifest(m: &Manifest, img: &Image) -> Result<(), String> {
    if m.format != 1 { return Err(format!("the bundle's format ({}) is one this Hearth doesn't know", m.format)); }
    if !safe_version(&m.version) { return Err("the bundle's version isn't a version".into()); }
    if m.edition != img.edition { return Err(format!("the bundle is for the {} edition, this is {}", m.edition, img.edition)); }
    if m.arch != img.arch { return Err(format!("the bundle is for {} processors, this is {}", m.arch, img.arch)); }
    if m.abi != img.abi { return Err(format!("the bundle {} needs a newer (or different) Hearth OS system (app ABI {}, this system has {})", m.version, m.abi, img.abi)); }
    if !m.min_image_version.is_empty() && newer(&m.min_image_version, &img.version) {
        return Err(format!("the bundle {} needs Hearth OS {} or newer", m.version, m.min_image_version));
    }
    if !m.max_image_version.is_empty() && newer(&img.version, &m.max_image_version) {
        return Err(format!("the bundle {} is only for Hearth OS up to {}", m.version, m.max_image_version));
    }
    if !newer(&m.version, &img.version) {
        return Err(format!("the system's own Hearth ({}) is the same or newer than the bundle ({})", img.version, m.version));
    }
    for f in &m.files {
        if !safe_rel(&f.path) { return Err(format!("the bundle names an unsafe path ({})", f.path)); }
        if let Some(l) = &f.link { if !safe_rel(l) { return Err(format!("the bundle has a link pointing outside it ({})", f.path)); } }
        else if f.sha256.len() != 64 { return Err(format!("the bundle doesn't give {}'s checksum", f.path)); }
    }
    match m.files.iter().find(|f| f.path == "nimbus") {
        Some(f) if f.link.is_none() && f.mode & 0o111 != 0 => Ok(()),
        _ => Err("the bundle has no Hearth program".into()),
    }
}

/// The manifest's signature, then whether it fits this image.
pub fn verify_manifest(bytes: &[u8], sig: &str, img: &Image) -> Result<Manifest, String> {
    verify_sshsig(bytes, sig, NAMESPACE, &img.keys)?;
    let m: Manifest = serde_json::from_slice(bytes).map_err(|e| format!("the bundle's manifest can't be read ({e})"))?;
    check_manifest(&m, img)?;
    Ok(m)
}

/// The unpacked bundle in `dir` is exactly what the manifest says. `full`: every file's SHA-256 and no extra files
/// (after unpacking); otherwise sizes, links and the program's SHA-256 (every start).
pub fn verify_tree(dir: &Path, m: &Manifest, full: bool) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    for f in &m.files {
        let p = dir.join(&f.path);
        let md = std::fs::symlink_metadata(&p).map_err(|_| format!("{} is missing", f.path))?;
        if let Some(l) = &f.link {
            if !md.file_type().is_symlink() || std::fs::read_link(&p).map(|t| t != Path::new(l)).unwrap_or(true) { return Err(format!("{} isn't the link it should be", f.path)); }
            continue;
        }
        if !md.file_type().is_file() { return Err(format!("{} isn't a file", f.path)); }
        if md.len() != f.size { return Err(format!("{} has the wrong size", f.path)); }
        if f.mode & 0o111 != 0 && md.permissions().mode() & 0o100 == 0 { return Err(format!("{} can't be run", f.path)); }
        if (full || f.path == "nimbus") && sha256_file(&p).map(|h| h != f.sha256).unwrap_or(true) { return Err(format!("{} was changed (checksum)", f.path)); }
    }
    if full {
        let listed: std::collections::HashSet<&str> = m.files.iter().map(|f| f.path.as_str()).collect();
        let mut stack = vec![PathBuf::new()];
        while let Some(rel) = stack.pop() {
            for e in std::fs::read_dir(dir.join(&rel)).map_err(|e| e.to_string())?.flatten() {
                let r = rel.join(e.file_name());
                let rs = r.to_string_lossy().to_string();
                if rel.as_os_str().is_empty() && (rs == "manifest.json" || rs == "manifest.json.sig") { continue; }
                let ft = e.file_type().map_err(|e| e.to_string())?;
                if ft.is_dir() { stack.push(r); continue; }
                if !listed.contains(rs.as_str()) { return Err(format!("{rs} isn't in the bundle's manifest")); }
            }
        }
    }
    Ok(())
}

// ------------------------------------------------------------------------------------------------------- marks

#[derive(Debug, Clone, PartialEq)]
pub struct Trying { pub version: String, pub fails: u32, pub since: u64 }

pub fn read_trying(root: &Path) -> Option<Trying> {
    let t = std::fs::read_to_string(marks(root).join("trying")).ok()?;
    let mut l = t.lines();
    let version = l.next()?.trim().to_string();
    if !safe_version(&version) { return None; }
    Some(Trying { version, fails: l.next().and_then(|s| s.trim().parse().ok()).unwrap_or(0), since: l.next().and_then(|s| s.trim().parse().ok()).unwrap_or(0) })
}
fn write_trying(root: &Path, t: &Trying) {
    let _ = crate::paths::write_atomic(&marks(root).join("trying"), format!("{}\n{}\n{}\n", t.version, t.fails, t.since).as_bytes());
}
pub fn is_good(root: &Path, v: &str) -> bool { safe_version(v) && marks(root).join("good").join(v).is_file() }
pub fn bad_reason(root: &Path, v: &str) -> Option<String> {
    if !safe_version(v) { return None; }
    std::fs::read_to_string(marks(root).join("bad").join(v)).ok().map(|s| s.trim().to_string())
}
pub fn mark_good(root: &Path, v: &str) {
    if !safe_version(v) { return; }
    let _ = crate::paths::write_atomic(&marks(root).join("good").join(v), chrono::Local::now().to_rfc3339().as_bytes());
    if read_trying(root).map(|t| t.version == v).unwrap_or(false) { let _ = std::fs::remove_file(marks(root).join("trying")); }
}
pub fn mark_bad(root: &Path, v: &str, why: &str) {
    if !safe_version(v) { return; }
    let _ = crate::paths::write_atomic(&marks(root).join("bad").join(v), why.as_bytes());
    let _ = std::fs::remove_file(marks(root).join("good").join(v));
    if read_trying(root).map(|t| t.version == v).unwrap_or(false) { let _ = std::fs::remove_file(marks(root).join("trying")); }
}
/// The built-in Hearth started instead of a bundle: remembered for Settings and Send logs, and said once on screen.
fn note_fallback(root: &Path, text: &str) {
    let _ = crate::paths::write_atomic(&marks(root).join("last-fallback"), format!("{} {text}", chrono::Local::now().to_rfc3339()).as_bytes());
    let _ = crate::paths::write_atomic(&marks(root).join("fallback-unseen"), text.as_bytes());
}
/// What the next screen should say (once): Hearth went back to its built-in version.
pub fn take_note() -> Option<String> {
    let f = marks(&root()).join("fallback-unseen");
    let t = std::fs::read_to_string(&f).ok()?;
    let _ = std::fs::remove_file(&f);
    let t = t.trim();
    (!t.is_empty()).then(|| format!("Hearth started its built-in version: {t}. Settings › Updates has the details."))
}

/// The bundle `current` points at (its version), if any.
pub fn current_version(root: &Path) -> Option<String> { link_version(&root.join("current")) }
pub fn previous_version(root: &Path) -> Option<String> { link_version(&root.join("previous")) }
fn link_version(l: &Path) -> Option<String> {
    let t = std::fs::read_link(l).ok()?;
    let v = t.to_str()?.trim_end_matches('/').to_string();
    safe_version(&v).then_some(v)
}

/// Points `name` (current/previous) at `version`, atomically (a new link renamed over the old one).
pub fn set_link(root: &Path, name: &str, version: &str) -> std::io::Result<()> {
    let tmp = root.join(format!(".{name}.{}.tmp", std::process::id()));
    let _ = std::fs::remove_file(&tmp);
    std::os::unix::fs::symlink(version, &tmp)?;
    std::fs::rename(&tmp, root.join(name)).inspect_err(|_| { let _ = std::fs::remove_file(&tmp); })
}
/// Makes `version` (already unpacked and checked in root/<version>) the current bundle; the one before becomes
/// previous.
pub fn switch_current(root: &Path, version: &str) -> std::io::Result<()> {
    if !safe_version(version) || !root.join(version).is_dir() { return Err(std::io::Error::other("not an unpacked bundle")); }
    let old = current_version(root);
    set_link(root, "current", version)?;
    if let Some(o) = old.filter(|o| o != version) { let _ = set_link(root, "previous", &o); }
    Ok(())
}

// --------------------------------------------------------------------------------------------------- choosing

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode { Auto, Ask, Off }
impl Mode {
    pub fn name(self) -> &'static str { match self { Mode::Auto => "auto", Mode::Ask => "ask", Mode::Off => "off" } }
    pub fn parse(s: &str) -> Option<Mode> { match s.trim() { "auto" | "automatic" => Some(Mode::Auto), "ask" => Some(Mode::Ask), "off" => Some(Mode::Off), _ => None } }
}
/// Settings › Updates › Hearth app updates (Automatic by default).
pub fn mode() -> Mode { std::fs::read_to_string(mode_file()).ok().as_deref().and_then(Mode::parse).unwrap_or(Mode::Auto) }
pub fn set_mode(m: Mode) -> Result<(), String> {
    log(&format!("Settings: Hearth app updates {}", m.name()));
    if m == Mode::Auto { let _ = std::fs::remove_file(mode_file()); Ok(()) }
    else { crate::paths::write_atomic(&mode_file(), m.name().as_bytes()).map_err(|e| e.to_string()) }
}

/// Which start this is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Context {
    /// Game Mode itself (the session's watchdog, `--session`): starts are counted.
    Primary,
    /// Anything else (a window in Desktop Mode, an extra multi-seat player): only a bundle already marked good.
    Secondary,
}

/// Everything the choice depends on (gathered by `facts`; tests make their own).
#[derive(Debug, Clone)]
pub struct Facts {
    pub root: PathBuf,
    pub image: Result<Image, String>,
    /// Why the built-in version must start (a flag, the boot option, Recovery), if one says so.
    pub forced: Option<String>,
    pub mode: Mode,
    /// A live USB or the installer: always the built-in version.
    pub live: bool,
    /// Chromium's own sandbox can work (unprivileged user namespaces).
    pub userns: bool,
    /// The renderer that would draw with a bundle that has (true) or hasn't (false) its own Chromium window app.
    pub engine: fn(bool) -> crate::renderer::Engine,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Choice { Builtin(String), Bundle { dir: PathBuf, version: String } }

/// Chooses between the built-in Hearth and the current bundle. Counts Game Mode starts (marks/trying) and marks a
/// bundle bad after FAILS_TO_BAD starts in a row that never got to good.
pub fn decide(f: &Facts, ctx: Context) -> Choice {
    use crate::renderer::Engine;
    let b = |s: &str| Choice::Builtin(s.to_string());
    if let Some(why) = &f.forced { return Choice::Builtin(why.clone()); }
    if f.live { return b("a live USB or the installer always uses the built-in version"); }
    let Some(version) = current_version(&f.root) else { return b("no Hearth app update is installed") };
    if f.mode == Mode::Off { return b("Hearth app updates are off (Settings › Updates)"); }
    let img = match &f.image { Ok(i) => i, Err(e) => return Choice::Builtin(e.clone()) };
    let dir = f.root.join(&version);
    if let Some(why) = bad_reason(&f.root, &version) { return Choice::Builtin(format!("{version} is set aside ({why})")); }
    let manifest = std::fs::read(dir.join("manifest.json")).map_err(|_| "its manifest is missing".to_string());
    let sig = std::fs::read_to_string(dir.join("manifest.json.sig")).map_err(|_| "its signature is missing".to_string());
    let m = match manifest.and_then(|bytes| sig.and_then(|s| verify_manifest(&bytes, &s, img))) {
        Ok(m) => m,
        Err(e) => return Choice::Builtin(format!("{version} can't be used: {e}")),
    };
    if m.version != version { return Choice::Builtin(format!("{version}'s manifest says it's {}", m.version)); }
    if let Err(e) = verify_tree(&dir, &m, false) { return Choice::Builtin(format!("{version} can't be used: {e}")); }
    let has_electron = m.files.iter().any(|x| x.path == "electron/hearth-electron");
    match (f.engine)(has_electron) {
        Engine::Qt => return b("the Qt renderer (Experimental) is chosen, and Hearth app updates don't cover it"),
        Engine::Chromium if !has_electron => return Choice::Builtin(format!("{version} has no Chromium window app, which this PC draws with")),
        Engine::Chromium if !f.userns => return b("Chromium's sandbox can't work on this system (user namespaces are off)"),
        _ => {}
    }
    if ctx == Context::Secondary {
        return if is_good(&f.root, &version) { Choice::Bundle { dir, version } }
               else { Choice::Builtin(format!("{version} hasn't come up in Game Mode yet")) };
    }
    let fails = read_trying(&f.root).filter(|t| t.version == version).map(|t| t.fails + 1).unwrap_or(0);
    if fails >= FAILS_TO_BAD {
        let why = format!("{version} didn't bring Hearth's screen up {fails} times in a row");
        mark_bad(&f.root, &version, &why);
        note_fallback(&f.root, &why);
        return Choice::Builtin(why);
    }
    write_trying(&f.root, &Trying { version: version.clone(), fails, since: now_ms() });
    Choice::Bundle { dir, version }
}

/// Unprivileged user namespaces, which Chromium's sandbox needs when its setuid helper can't be used (a bundle in
/// your home folder can't have one).
pub fn userns_ok() -> bool {
    let read = |p: &str| std::fs::read_to_string(p).map(|s| s.trim().to_string()).ok();
    if read("/proc/sys/kernel/unprivileged_userns_clone").as_deref() == Some("0") { return false; }
    if read("/proc/sys/user/max_user_namespaces").as_deref() == Some("0") { return false; }
    if read("/proc/sys/kernel/apparmor_restrict_unprivileged_userns").as_deref() == Some("1") { return false; }
    true
}

/// Why the built-in version must start this time, if anything says so:
///   HEARTH_APP=builtin, the boot option hearth.app=builtin, ~/.config/nimbus/app-builtin (until removed),
///   $XDG_RUNTIME_DIR/hearth-app-builtin (this sign-in), /run/hearth/app-builtin or /run/hearth/safe-graphics
///   (Hearth Recovery, this start).
pub fn forced() -> Option<String> {
    if std::env::var("HEARTH_APP").map(|v| v.trim() == "builtin").unwrap_or(false) { return Some("HEARTH_APP=builtin asks for the built-in version".into()); }
    let cmdline = std::env::var_os("HEARTH_CMDLINE").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/proc/cmdline"));
    if std::fs::read_to_string(cmdline).map(|c| c.split_whitespace().any(|w| w == "hearth.app=builtin")).unwrap_or(false) {
        return Some("the boot option hearth.app=builtin asks for the built-in version".into());
    }
    if crate::paths::config_dir().join("app-builtin").exists() { return Some("~/.config/nimbus/app-builtin asks for the built-in version".into()); }
    if let Some(r) = std::env::var_os("XDG_RUNTIME_DIR") { if Path::new(&r).join("hearth-app-builtin").exists() { return Some("the built-in version was asked for, for this sign-in".into()); } }
    if Path::new("/run/hearth/app-builtin").exists() { return Some("Hearth Recovery asked for the built-in version".into()); }
    if Path::new("/run/hearth/safe-graphics").exists() { return Some("Hearth Recovery's safe graphics use the built-in version".into()); }
    None
}

pub fn facts() -> Facts {
    Facts {
        root: root(),
        image: image(),
        forced: forced(),
        mode: mode(),
        live: Path::new("/etc/hearth-live").exists() || Path::new("/etc/hearth-installer").exists(),
        userns: userns_ok(),
        engine: crate::renderer::planned,
    }
}

/// The bundle this program is running from (its version and folder), or None for the built-in Hearth.
pub fn running_bundle() -> Option<(String, PathBuf)> {
    let exe = std::env::current_exe().ok()?.canonicalize().ok()?;
    let root = root().canonicalize().ok()?;
    let rel = exe.strip_prefix(&root).ok()?;
    let v = rel.components().next()?.as_os_str().to_str()?.to_string();
    safe_version(&v).then(|| (v.clone(), root.join(v)))
}
/// The bundle's own Chromium window app, when this program is a bundle (renderer.rs): never the image's one, whose
/// screens belong to the built-in version.
pub fn running_bundle_electron() -> Option<PathBuf> { running_bundle().map(|(_, d)| d.join("electron/hearth-electron")) }

/// May this program choose? Only the image's own Hearth (or a test standing in for it).
fn may_choose() -> bool {
    if running_bundle().is_some() { return false; }
    if std::env::var_os("HEARTH_APP_IMAGE_DIR").is_some() { return true; }
    std::env::current_exe().ok().and_then(|e| e.canonicalize().ok()).map(|e| e == Path::new(BUILTIN_EXE)).unwrap_or(false)
}

static CHOSEN_CTX: OnceLock<Context> = OnceLock::new();

/// At Hearth's start (hearth-rs main): the bundle's program to hand over to and whether this is Game Mode's own start
/// (then the built-in Hearth stays as its parent and watches it: `supervise`), or None to go on as the built-in
/// Hearth. Never panics; anything unexpected means the built-in version.
pub fn handoff(args: &[String]) -> Option<(PathBuf, bool)> {
    if !may_choose() { return None; }
    let primary = args.iter().any(|a| a == "--session") && !args.iter().any(|a| a == "--windowed")
        && crate::gamescope::in_session() && std::env::var("HEARTH_SEAT").map(|s| s.is_empty()).unwrap_or(true);
    let ctx = if primary { Context::Primary } else { Context::Secondary };
    let _ = CHOSEN_CTX.set(ctx);
    let r = std::panic::catch_unwind(|| decide(&facts(), ctx));
    match r {
        Ok(Choice::Bundle { dir, version }) => { log(&format!("starting the Hearth app update {version} ({ctx:?})")); Some((dir.join("nimbus"), ctx == Context::Primary)) }
        Ok(Choice::Builtin(why)) => { if current_version(&root()).is_some() { log(&format!("the built-in Hearth starts: {why}")); } None }
        Err(_) => { log("choosing the version panicked; the built-in Hearth starts"); None }
    }
}
/// The bundle's program couldn't be started at all: it's set aside and the built-in Hearth goes on.
pub fn handoff_failed(app: &Path, err: &str) {
    let r = root();
    let v = app.parent().and_then(|d| d.file_name()).and_then(|n| n.to_str()).unwrap_or("").to_string();
    let why = format!("{v} couldn't be started ({err})");
    log(&why);
    mark_bad(&r, &v, &why);
    note_fallback(&r, &why);
}

/// How long a Game Mode start with a bundle may take to bring Hearth's screen up before it counts as failed (a black
/// screen that never crashes would otherwise never be noticed). HEARTH_APP_UI_TIMEOUT (seconds) for tests.
fn ui_timeout() -> u64 { std::env::var("HEARTH_APP_UI_TIMEOUT").ok().and_then(|s| s.parse().ok()).unwrap_or(90) }

/// Game Mode's start with a bundle: the bundle runs as this (built-in) Hearth's child, so the image's own code watches
/// it. Its screen not up within `ui_timeout` (marks/ui-up, written by the bundle's core): it's stopped and this start
/// ends as failed, so the session's watchdog starts Hearth again and the count goes on. Returns the exit code to end
/// with (the bundle's own: 0 leave, 75 start again, anything else a crash), or None if it couldn't even start (it's
/// set aside then, and the built-in Hearth goes on in this process).
pub fn supervise(app: &Path, args: &[String]) -> Option<i32> {
    use std::os::unix::process::{CommandExt, ExitStatusExt};
    let started = now_ms();
    let mut cmd = std::process::Command::new(app);
    cmd.args(&args[1..]);
    // If this process goes (the session ends, the watchdog stops it), the bundle goes too.
    unsafe { cmd.pre_exec(|| { libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM); Ok(()) }); }
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => { handoff_failed(app, &e.to_string()); return None; }
    };
    let version = app.parent().and_then(|d| d.file_name()).and_then(|n| n.to_str()).unwrap_or("").to_string();
    let limit = ui_timeout() * 1000;
    let mut up = false;
    loop {
        match child.try_wait() {
            Ok(Some(st)) => {
                let code = st.code().unwrap_or_else(|| 128 + st.signal().unwrap_or(0));
                if up || ui_up_since(&root(), &version, started) { return Some(code); }
                // Leaving (0) or restarting (75) before the screen ever came up isn't something Hearth does: counted
                // as a crash, so the session's watchdog starts Hearth again (the count goes on) instead of the session
                // ending.
                log(&format!("{version} ended before its screen came up (code {code})"));
                return Some(if code == 0 || code == 75 { 1 } else { code });
            }
            Ok(None) => {}
            Err(_) => return Some(1),
        }
        if !up {
            up = ui_up_since(&root(), &version, started);
            if !up && now_ms().saturating_sub(started) > limit {
                log(&format!("{version} didn't bring Hearth's screen up within {} s; stopping it (counted as a failed start)", limit / 1000));
                unsafe { libc::kill(child.id() as i32, libc::SIGTERM); }
                for _ in 0..10 { if matches!(child.try_wait(), Ok(Some(_))) { break; } std::thread::sleep(std::time::Duration::from_millis(500)); }
                let _ = child.kill();
                let _ = child.wait();
                return Some(1);
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}
fn ui_up_since(root: &Path, version: &str, since: u64) -> bool {
    let t = std::fs::read_to_string(marks(root).join("ui-up")).unwrap_or_default();
    let mut l = t.lines();
    l.next().map(str::trim) == Some(version) && l.next().and_then(|x| x.trim().parse::<u64>().ok()).map(|t| t >= since).unwrap_or(false)
}

static UI_UP: AtomicBool = AtomicBool::new(false);
/// Hearth's main screen came up (the page's first "ready"). A bundle that keeps running GOOD_AFTER_SECS from here is
/// marked good.
pub fn note_ui_ready() {
    if UI_UP.swap(true, Ordering::SeqCst) { return; }
    let Some((v, _)) = running_bundle() else { return };
    let _ = crate::paths::write_atomic(&marks(&root()).join("ui-up"), format!("{v}\n{}\n", now_ms()).as_bytes());
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(GOOD_AFTER_SECS));
        if !is_good(&root(), &v) { log(&format!("{v} is good (its screen came up and it ran for a minute)")); }
        mark_good(&root(), &v);
    });
}
/// Hearth is leaving on purpose (to Desktop Mode, restarting) after its screen came up: that start was good.
pub fn note_clean_exit() {
    if !UI_UP.load(Ordering::SeqCst) { return; }
    if let Some((v, _)) = running_bundle() { mark_good(&root(), &v); }
}
/// `--app-mark-good`: marks the running bundle good now (CI's end-to-end test; same as note_ui_ready's timer).
pub fn mark_running_good() -> Option<String> { running_bundle().map(|(v, _)| { mark_good(&root(), &v); v }) }
/// `--app-ui-up`: says the screen is up, as the page's "ready" does (CI's end-to-end test of `supervise`).
pub fn note_ui_up_now() -> Option<String> {
    let (v, _) = running_bundle()?;
    let _ = crate::paths::write_atomic(&marks(&root()).join("ui-up"), format!("{v}\n{}\n", now_ms()).as_bytes());
    Some(v)
}

/// Settings › Updates › Use the built-in version: the current bundle is set aside (a newer one can still come),
/// and Hearth starts its built-in version from the next start.
pub fn use_builtin() -> Result<(), String> {
    let r = root();
    let v = current_version(&r).ok_or("Hearth is already using its built-in version")?;
    mark_bad(&r, &v, "you chose the built-in version in Settings");
    let _ = std::fs::remove_file(r.join("current"));
    log(&format!("Settings: {v} set aside; the built-in Hearth starts from the next start"));
    Ok(())
}

// ------------------------------------------------------------------------------------------------- installing

/// Unpacks a .tar.zst (zstd, which every Hearth OS image has, decompresses; the tar is read here) into `dest`:
/// files, folders and links inside it only.
fn unpack(archive: &Path, dest: &Path) -> Result<(), String> {
    use std::process::{Command, Stdio};
    let mut child = Command::new("zstd").args(["-dcq", "--"]).arg(archive).stdout(Stdio::piped()).stderr(Stdio::null()).spawn()
        .map_err(|e| format!("zstd isn't available ({e})"))?;
    let out = child.stdout.take().ok_or("zstd didn't start")?;
    let r = (|| -> Result<(), String> {
        let mut ar = tar::Archive::new(out);
        for e in ar.entries().map_err(|e| e.to_string())? {
            let mut e = e.map_err(|e| e.to_string())?;
            let path = e.path().map_err(|e| e.to_string())?.into_owned();
            let mut rel = PathBuf::new();
            for c in path.components() {
                match c { Component::Normal(x) => rel.push(x), Component::CurDir => {}, _ => return Err(format!("the archive has an unsafe path ({})", path.display())) }
            }
            if rel.as_os_str().is_empty() { continue; }
            let to = dest.join(&rel);
            match e.header().entry_type() {
                tar::EntryType::Directory => std::fs::create_dir_all(&to).map_err(|e| e.to_string())?,
                tar::EntryType::Regular | tar::EntryType::Continuous => {
                    if let Some(p) = to.parent() { std::fs::create_dir_all(p).map_err(|e| e.to_string())?; }
                    let mut f = std::fs::File::create(&to).map_err(|e| e.to_string())?;
                    std::io::copy(&mut e, &mut f).map_err(|e| e.to_string())?;
                }
                tar::EntryType::Symlink => {
                    let target = e.link_name().map_err(|e| e.to_string())?.ok_or("a link without a target")?.into_owned();
                    if !safe_rel(&target.to_string_lossy()) { return Err(format!("the archive has a link pointing outside it ({})", rel.display())); }
                    if let Some(p) = to.parent() { std::fs::create_dir_all(p).map_err(|e| e.to_string())?; }
                    std::os::unix::fs::symlink(&target, &to).map_err(|e| e.to_string())?;
                }
                _ => return Err(format!("the archive has something that isn't a file ({})", rel.display())),
            }
        }
        // The tar ends before the stream does (tar pads to a full record): read the rest, so zstd isn't cut off.
        let mut rest = ar.into_inner();
        std::io::copy(&mut rest, &mut std::io::sink()).map_err(|e| e.to_string())?;
        Ok(())
    })();
    let st = child.wait().map_err(|e| e.to_string())?;
    r?;
    if !st.success() { return Err("the archive is damaged (zstd couldn't unpack it)".into()); }
    Ok(())
}

/// Keeps current, previous and the running bundle; anything else (older attempts, bundles the system has caught up
/// with) goes.
pub fn tidy(root: &Path, img: Option<&Image>) {
    let keep: Vec<String> = [current_version(root), previous_version(root), running_bundle().map(|r| r.0)].into_iter().flatten().collect();
    // A bundle the system has caught up with is never used again: the built-in version wins.
    if let (Some(i), Some(c)) = (img, current_version(root)) {
        if !newer(&c, &i.version) && running_bundle().map(|r| r.0 != c).unwrap_or(true) {
            log(&format!("the system's own Hearth ({}) has caught up with {c}; the bundle goes", i.version));
            let _ = std::fs::remove_file(root.join("current"));
        }
    }
    for e in std::fs::read_dir(root).into_iter().flatten().flatten() {
        let n = e.file_name().to_string_lossy().to_string();
        if n == "staging" || n == "marks" || !safe_version(&n) || keep.contains(&n) || e.path().is_symlink() || !e.path().is_dir() { continue; }
        let _ = std::fs::remove_dir_all(e.path());
    }
}

/// Installs a downloaded bundle: checks the manifest's signature and fit, the archive's size and SHA-256, unpacks it
/// in staging, checks every file, then moves it in place and switches `current` to it. Nothing outside `root`.
pub fn install_archive(root: &Path, img: &Image, archive: &Path, manifest: &[u8], sig: &str) -> Result<String, String> {
    let m = verify_manifest(manifest, sig, img)?;
    if let Some(why) = bad_reason(root, &m.version) { return Err(format!("{} was set aside ({why})", m.version)); }
    let size = std::fs::metadata(archive).map_err(|e| e.to_string())?.len();
    if size != m.size { return Err(format!("the download has the wrong size ({size} bytes, the manifest says {})", m.size)); }
    if sha256_file(archive).map_err(|e| e.to_string())? != m.sha256 { return Err("the download is damaged (checksum)".into()); }
    let staging = root.join("staging");
    let work = staging.join(format!("{}.partial", m.version));
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work).map_err(|e| e.to_string())?;
    let r = (|| -> Result<(), String> {
        unpack(archive, &work)?;
        {
            use std::os::unix::fs::PermissionsExt;
            for f in m.files.iter().filter(|f| f.link.is_none()) {
                let mode = if f.mode & 0o111 != 0 { 0o755 } else { 0o644 };
                let _ = std::fs::set_permissions(work.join(&f.path), std::fs::Permissions::from_mode(mode));
            }
        }
        std::fs::write(work.join("manifest.json"), manifest).map_err(|e| e.to_string())?;
        std::fs::write(work.join("manifest.json.sig"), sig).map_err(|e| e.to_string())?;
        verify_tree(&work, &m, true)
    })();
    if let Err(e) = r { let _ = std::fs::remove_dir_all(&work); return Err(e); }
    let to = root.join(&m.version);
    if current_version(root).as_deref() == Some(m.version.as_str()) { let _ = std::fs::remove_file(root.join("current")); }
    let _ = std::fs::remove_dir_all(&to);
    std::fs::rename(&work, &to).map_err(|e| e.to_string())?;
    // A fresh install gets a fresh count.
    let _ = std::fs::remove_file(marks(root).join("good").join(&m.version));
    if read_trying(root).map(|t| t.version == m.version).unwrap_or(false) { let _ = std::fs::remove_file(marks(root).join("trying")); }
    switch_current(root, &m.version).map_err(|e| e.to_string())?;
    tidy(root, Some(img));
    log(&format!("installed the Hearth app update {} (used from the next start)", m.version));
    Ok(m.version)
}

/// One updater at a time per user (flock on root/.lock), across Hearth's processes.
struct Lock(#[allow(dead_code)] std::fs::File);
fn lock(root: &Path) -> Option<Lock> {
    use std::os::unix::io::AsRawFd;
    std::fs::create_dir_all(root).ok()?;
    let f = std::fs::OpenOptions::new().create(true).truncate(false).write(true).open(root.join(".lock")).ok()?;
    (unsafe { libc::flock(f.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0).then_some(Lock(f))
}

// ---------------------------------------------------------------------------------------------------- updater

/// Release files for this image: hearth-app-<edition>-<arch>.{tar.zst,json,json.sig}.
pub fn asset_base(img: &Image) -> String { format!("hearth-app-{}-{}", img.edition, img.arch) }

#[derive(Clone)]
pub struct Updater { state: Arc<Mutex<Value>> }
static UPDATER: OnceLock<Updater> = OnceLock::new();
pub fn updater() -> &'static Updater {
    UPDATER.get_or_init(|| Updater { state: Arc::new(Mutex::new(json!({"state": "idle", "error": null, "latest": null, "lastCheck": 0, "progress": null}))) })
}

impl Updater {
    fn set(&self, patch: Value) {
        if let Ok(mut s) = self.state.lock() { for (k, v) in patch.as_object().cloned().unwrap_or_default() { s[k] = v; } }
        let _ = crate::paths::write_atomic(&marks(&root()).join("last-check.json"), serde_json::to_string(&self.state.lock().map(|s| s.clone()).unwrap_or(Value::Null)).unwrap_or_default().as_bytes());
        crate::stores::notify(json!({"type": "event", "name": "appUpdates", "data": status()}));
    }
    fn busy(&self) -> bool { self.state.lock().map(|s| s["state"] == "checking" || s["state"] == "downloading").unwrap_or(false) }

    /// Started with Hearth's Game Mode: a check two minutes after the start and every 6 hours (Automatic installs,
    /// Ask only says so, Off does nothing). Also tidies away bundles the system has caught up with.
    pub fn start_background(&'static self) {
        static STARTED: AtomicBool = AtomicBool::new(false);
        if STARTED.swap(true, Ordering::SeqCst) { return; }
        let Ok(img) = image() else { return };
        tidy(&root(), Some(&img));
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(120));
            loop {
                match mode() { Mode::Off => {}, m => { self.check(m == Mode::Auto); } }
                std::thread::sleep(std::time::Duration::from_secs(6 * 3600));
            }
        });
    }

    /// Looks at the newest release's app bundle; with `install`, downloads and installs it too. Blocking.
    pub fn check(&self, install: bool) -> Value {
        if self.busy() { return status(); }
        let img = match image() { Ok(i) => i, Err(e) => { self.set(json!({"state": "unavailable", "error": e, "lastCheck": now_ms()})); return status(); } };
        self.set(json!({"state": "checking", "error": null}));
        let fail = |s: &Self, e: String| { s.set(json!({"state": "error", "error": e, "lastCheck": now_ms(), "progress": null})); status() };
        let rel = match crate::http::get_json(&format!("https://api.github.com/repos/{REPO}/releases/latest"), &[("Accept", "application/vnd.github+json")]) {
            Ok(r) => r, Err(e) => return fail(self, format!("Couldn't check for Hearth app updates: {e}")),
        };
        let base = asset_base(&img);
        let url = |name: String| rel["assets"].as_array().and_then(|a| a.iter().find(|x| x["name"] == name.as_str())).and_then(|x| x["browser_download_url"].as_str()).map(str::to_string);
        let (Some(murl), Some(surl), Some(aurl)) = (url(format!("{base}.json")), url(format!("{base}.json.sig")), url(format!("{base}.tar.zst"))) else {
            self.set(json!({"state": "upToDate", "latest": null, "lastCheck": now_ms(), "note": "the newest release has no Hearth app update for this edition"}));
            return status();
        };
        let get = |u: &str| crate::http::get(u, &[]).ok().filter(|r| r.ok()).map(|r| r.text);
        let (Some(mtext), Some(sig)) = (get(&murl), get(&surl)) else { return fail(self, "Couldn't download the Hearth app update's manifest".into()) };
        let m = match verify_manifest(mtext.as_bytes(), &sig, &img) {
            Ok(m) => m,
            Err(e) => {
                // Not for this system (it needs a system update first, or the system already has it): not an error.
                self.set(json!({"state": "upToDate", "latest": null, "lastCheck": now_ms(), "note": e}));
                return status();
            }
        };
        let latest = json!({"version": m.version, "size": m.size, "commit": m.commit});
        let r = root();
        if current_version(&r).map(|c| !newer(&m.version, &c)).unwrap_or(false) {
            self.set(json!({"state": "upToDate", "latest": latest, "lastCheck": now_ms(), "note": null}));
            return status();
        }
        if let Some(why) = bad_reason(&r, &m.version) {
            self.set(json!({"state": "upToDate", "latest": latest, "lastCheck": now_ms(), "note": format!("{} is set aside ({why})", m.version)}));
            return status();
        }
        self.set(json!({"state": "available", "latest": latest, "lastCheck": now_ms(), "note": null}));
        if !install { return status(); }
        let Some(_l) = lock(&r) else { return fail(self, "Another Hearth is installing an app update right now".into()) };
        self.set(json!({"state": "downloading", "progress": 0}));
        let staging = r.join("staging");
        let _ = std::fs::create_dir_all(&staging);
        let file = staging.join(format!("{base}.tar.zst"));
        let _ = std::fs::remove_file(&file);
        let mut child = match std::process::Command::new("curl").args(["-fsSL", "--retry", "3", "--speed-limit", "1024", "--speed-time", "60", "--max-filesize"])
            .arg(m.size.to_string()).arg("-o").arg(&file).arg(&aurl).spawn() {
            Ok(c) => c, Err(e) => return fail(self, format!("Couldn't download the Hearth app update: {e}")),
        };
        let ok = loop {
            if let Ok(Some(s)) = child.try_wait() { break s.success(); }
            std::thread::sleep(std::time::Duration::from_millis(700));
            let done = std::fs::metadata(&file).map(|x| x.len()).unwrap_or(0);
            if let Ok(mut s) = self.state.lock() { s["progress"] = json!((done as f64 / m.size.max(1) as f64).min(1.0)); }
            crate::stores::notify(json!({"type": "event", "name": "appUpdates", "data": status()}));
        };
        if !ok { let _ = std::fs::remove_file(&file); return fail(self, "The Hearth app update's download didn't finish".into()); }
        let res = install_archive(&r, &img, &file, mtext.as_bytes(), &sig);
        let _ = std::fs::remove_file(&file);
        match res {
            Ok(v) => { self.set(json!({"state": "installed", "progress": null, "lastCheck": now_ms(), "installed": v})); status() }
            Err(e) => fail(self, format!("The Hearth app update wasn't installed: {e}")),
        }
    }
}

// ------------------------------------------------------------------------------------------------ status & logs

/// For Settings › Updates and Send logs.
pub fn status() -> Value {
    let r = root();
    let img = image();
    let list = |d: &str| -> Vec<String> {
        let mut v: Vec<String> = std::fs::read_dir(marks(&r).join(d)).into_iter().flatten().flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect();
        v.sort_by(|a, b| cmp_versions(a, b));
        v
    };
    let bad: serde_json::Map<String, Value> = list("bad").into_iter().map(|v| { let why = bad_reason(&r, &v).unwrap_or_default(); (v, json!(why)) }).collect();
    let st = updater().state.lock().map(|s| s.clone()).unwrap_or(Value::Null);
    let last = if st["lastCheck"].as_u64().unwrap_or(0) > 0 { st.clone() }
               else { std::fs::read_to_string(marks(&r).join("last-check.json")).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or(st.clone()) };
    let running = running_bundle().map(|b| b.0);
    json!({
        "mode": mode().name(),
        "builtin": img.as_ref().ok().map(|i| i.version.clone()).or_else(|| std::fs::read_to_string(image_dir().join("version")).ok().map(|s| s.trim().to_string())),
        "running": running,
        "runningFrom": if running.is_some() { "bundle" } else { "built-in" },
        "bundle": current_version(&r),
        "previous": previous_version(&r),
        "good": list("good"),
        "bad": bad,
        "trying": read_trying(&r).map(|t| json!({"version": t.version, "fails": t.fails, "since": t.since})),
        "lastFallback": std::fs::read_to_string(marks(&r).join("last-fallback")).ok().map(|s| s.trim().to_string()),
        "forced": forced(),
        "available": img.is_ok(),
        "unavailable": img.as_ref().err(),
        "abi": img.as_ref().ok().map(|i| i.abi.clone()),
        "edition": img.as_ref().ok().map(|i| i.edition.clone()),
        "state": last["state"], "error": last["error"], "latest": last["latest"], "lastCheck": last["lastCheck"], "note": last["note"],
        "progress": st["progress"],
    })
}

/// Send logs' "Hearth app updates" section.
pub fn report() -> String {
    let s = status();
    let r = root();
    let dirs: Vec<String> = std::fs::read_dir(&r).into_iter().flatten().flatten().map(|e| e.file_name().to_string_lossy().to_string()).filter(|n| n != "marks" && n != ".lock").collect();
    format!("{}\n\nIn {}: {}", serde_json::to_string_pretty(&s).unwrap_or_default(), r.display(), dirs.join(", "))
}

/// `--app-which`: which program is running after the choice (CI's end-to-end test).
pub fn which_json() -> String {
    json!({"exe": std::env::current_exe().map(|e| e.to_string_lossy().to_string()).unwrap_or_default(), "bundle": running_bundle().map(|b| b.0),
           "context": CHOSEN_CTX.get().map(|c| format!("{c:?}"))}).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::Engine;

    // A throwaway test key (its private half was never kept) and SSHSIG signatures made with it (namespace
    // hearth-app, sha512) over MSG; CI checks real ssh-keygen signatures end to end (hosts/linux/test/test_hearth_app_channel.py).
    const PUB: &str = include_str!("fixtures/appchannel/test-key.pub");
    const MSG: &[u8] = include_bytes!("fixtures/appchannel/message.txt");
    const SIG: &str = include_str!("fixtures/appchannel/message.txt.sig");
    const SIG_OTHER_NS: &str = include_str!("fixtures/appchannel/message.txt.sig-other-namespace");
    const SIG_OTHER_KEY: &str = include_str!("fixtures/appchannel/message.txt.sig-other-key");

    #[test]
    fn versions_compare() {
        assert!(newer("0.6.61", "0.6.60"));
        assert!(!newer("0.6.60", "0.6.60"));
        assert!(newer("0.6.60+app.1", "0.6.60"));
        assert!(newer("0.6.60+app.10", "0.6.60+app.9"));
        assert!(!newer("0.6.60+app.3", "0.6.61"));
        assert!(newer("v0.7.0", "0.6.99+app.4"));
        assert!(newer("0.6.6.1", "0.6.6"));
        assert_eq!(cmp_versions("0.6.60", "0.6.60.0"), std::cmp::Ordering::Equal);
        assert!(safe_version("0.6.61+app.2"));
        assert!(!safe_version("../x") && !safe_version("") && !safe_version("a/b") && !safe_version(".hidden"));
    }

    #[test]
    fn base64_and_keys() {
        assert_eq!(b64decode("aGVhcnRo").unwrap(), b"hearth");
        assert_eq!(b64decode("aGVhcnRoIQ==").unwrap(), b"hearth!");
        assert!(b64decode("not*base64").is_none());
        assert_eq!(parse_pubkeys(PUB).len(), 1);
        assert!(parse_pubkeys("# placeholder: the owner puts the real key here\n").is_empty());
        assert!(parse_pubkeys("ssh-rsa AAAAB3NzaC1yc2E x\n").is_empty());
    }

    #[test]
    fn signatures() {
        let keys = parse_pubkeys(PUB);
        assert_eq!(verify_sshsig(MSG, SIG, NAMESPACE, &keys), Ok(()));
        let mut changed = MSG.to_vec();
        changed[0] ^= 1;
        assert!(verify_sshsig(&changed, SIG, NAMESPACE, &keys).is_err(), "a changed message fails");
        assert!(verify_sshsig(MSG, SIG_OTHER_NS, NAMESPACE, &keys).is_err(), "a signature for another namespace fails");
        assert!(verify_sshsig(MSG, SIG_OTHER_KEY, NAMESPACE, &keys).unwrap_err().contains("doesn't trust"), "another key fails");
        assert!(verify_sshsig(MSG, SIG, NAMESPACE, &[]).is_err(), "no trusted key: nothing verifies");
        assert!(verify_sshsig(MSG, "garbage", NAMESPACE, &keys).is_err());
        // A flipped bit in the signature itself.
        let body: String = SIG.lines().filter(|l| !l.starts_with("-----")).collect();
        let mut raw = b64decode(&body).unwrap();
        let n = raw.len() - 3;
        raw[n] ^= 0x40;
        let enc = b64e(&raw);
        let armored = format!("-----BEGIN SSH SIGNATURE-----\n{enc}\n-----END SSH SIGNATURE-----\n");
        assert!(verify_sshsig(MSG, &armored, NAMESPACE, &keys).is_err(), "a damaged signature fails");
    }

    fn img() -> Image { Image { version: "0.6.60".into(), abi: "1-abc".into(), edition: "anatase".into(), arch: std::env::consts::ARCH.into(), keys: vec![[7u8; 32]] } }
    fn manifest(version: &str, files: Vec<FileEntry>) -> Manifest {
        Manifest { format: 1, version: version.into(), edition: "anatase".into(), arch: std::env::consts::ARCH.into(), abi: "1-abc".into(),
                   min_image_version: String::new(), max_image_version: String::new(), sha256: "0".repeat(64), size: 1, archive: String::new(), commit: String::new(), files }
    }
    fn file(path: &str, body: &[u8], mode: u32) -> FileEntry {
        use sha2::Digest;
        FileEntry { path: path.into(), sha256: sha2::Sha256::digest(body).iter().map(|b| format!("{b:02x}")).collect(), size: body.len() as u64, mode, link: None }
    }

    #[test]
    fn manifest_fit() {
        let ok = manifest("0.6.61", vec![file("nimbus", b"x", 0o755)]);
        assert_eq!(check_manifest(&ok, &img()), Ok(()));
        let with = |f: &dyn Fn(&mut Manifest)| { let mut m = ok.clone(); f(&mut m); check_manifest(&m, &img()) };
        assert!(with(&|m| m.abi = "2-def".into()).unwrap_err().contains("app ABI"));
        assert!(with(&|m| m.edition = "peppermint".into()).unwrap_err().contains("edition"));
        assert!(with(&|m| m.arch = "aarch64-nope".into()).is_err());
        assert!(with(&|m| m.version = "0.6.60".into()).unwrap_err().contains("same or newer"), "not newer than the built-in");
        assert!(with(&|m| m.version = "0.6.59".into()).is_err());
        assert!(with(&|m| m.min_image_version = "0.6.61".into()).unwrap_err().contains("or newer"));
        assert_eq!(with(&|m| m.min_image_version = "0.6.60".into()), Ok(()));
        assert!(with(&|m| m.max_image_version = "0.6.59".into()).is_err());
        assert!(with(&|m| m.files.push(file("../evil", b"", 0o644))).unwrap_err().contains("unsafe"));
        assert!(with(&|m| m.files.push(file("/etc/evil", b"", 0o644))).is_err());
        assert!(with(&|m| m.files.push(FileEntry { path: "l".into(), sha256: String::new(), size: 0, mode: 0, link: Some("../../x".into()) })).is_err());
        assert!(with(&|m| m.files = vec![file("nimbus", b"x", 0o644)]).unwrap_err().contains("no Hearth program"));
        assert!(with(&|m| m.version = "0.6.61/../x".into()).is_err());
        assert!(with(&|m| m.format = 2).is_err());
    }

    #[test]
    fn tree_checks() {
        use std::os::unix::fs::PermissionsExt;
        let t = tempfile::tempdir().unwrap();
        let d = t.path();
        std::fs::write(d.join("nimbus"), b"program").unwrap();
        std::fs::set_permissions(d.join("nimbus"), std::fs::Permissions::from_mode(0o755)).unwrap();
        std::fs::create_dir_all(d.join("electron")).unwrap();
        std::fs::write(d.join("electron/a.pak"), b"pak").unwrap();
        std::os::unix::fs::symlink("a.pak", d.join("electron/b.pak")).unwrap();
        std::fs::write(d.join("manifest.json"), b"{}").unwrap();
        let mut m = manifest("0.6.61", vec![file("nimbus", b"program", 0o755), file("electron/a.pak", b"pak", 0o644),
            FileEntry { path: "electron/b.pak".into(), sha256: String::new(), size: 0, mode: 0, link: Some("a.pak".into()) }]);
        assert_eq!(verify_tree(d, &m, true), Ok(()));
        std::fs::write(d.join("electron/extra"), b"?").unwrap();
        assert!(verify_tree(d, &m, true).unwrap_err().contains("isn't in the bundle's manifest"));
        assert_eq!(verify_tree(d, &m, false), Ok(()), "the quick check doesn't walk the folder");
        std::fs::remove_file(d.join("electron/extra")).unwrap();
        std::fs::write(d.join("electron/a.pak"), b"PAK").unwrap();
        assert_eq!(verify_tree(d, &m, false), Ok(()), "same size: only the full check reads it");
        assert!(verify_tree(d, &m, true).unwrap_err().contains("checksum"));
        std::fs::write(d.join("nimbus"), b"PROGRAM").unwrap();
        assert!(verify_tree(d, &m, false).unwrap_err().contains("checksum"), "the program is checked every start");
        m.files[0] = file("nimbus", b"PROGRAM", 0o755);
        std::fs::set_permissions(d.join("nimbus"), std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(verify_tree(d, &m, false).unwrap_err().contains("can't be run"));
        std::fs::remove_file(d.join("nimbus")).unwrap();
        assert!(verify_tree(d, &m, false).unwrap_err().contains("missing"));
    }

    #[test]
    fn links_switch_atomically_and_keep_the_previous() {
        let t = tempfile::tempdir().unwrap();
        let r = t.path();
        for v in ["0.6.61", "0.6.62", "0.6.63"] { std::fs::create_dir_all(r.join(v)).unwrap(); }
        assert!(switch_current(r, "0.6.70").is_err(), "not unpacked: no switch");
        assert!(switch_current(r, "../x").is_err());
        switch_current(r, "0.6.61").unwrap();
        assert_eq!(current_version(r).as_deref(), Some("0.6.61"));
        assert_eq!(previous_version(r), None);
        switch_current(r, "0.6.62").unwrap();
        assert_eq!(current_version(r).as_deref(), Some("0.6.62"));
        assert_eq!(previous_version(r).as_deref(), Some("0.6.61"));
        assert_eq!(std::fs::read_link(r.join("current")).unwrap(), Path::new("0.6.62"), "relative links");
        switch_current(r, "0.6.63").unwrap();
        assert_eq!(previous_version(r).as_deref(), Some("0.6.62"));
        tidy(r, Some(&img()));
        assert!(!r.join("0.6.61").exists(), "older ones go");
        assert!(r.join("0.6.62").is_dir() && r.join("0.6.63").is_dir());
        // The system catching up: the bundle isn't current any more.
        let mut caught = img(); caught.version = "0.6.63".into();
        tidy(r, Some(&caught));
        assert_eq!(current_version(r), None);
        assert!(!std::fs::read_dir(r).unwrap().flatten().any(|e| e.file_name().to_string_lossy().starts_with('.')), "no temporary links left");
    }

    #[test]
    fn marks_count_and_set_aside() {
        let t = tempfile::tempdir().unwrap();
        let r = t.path();
        assert_eq!(read_trying(r), None);
        write_trying(r, &Trying { version: "0.6.61".into(), fails: 1, since: 5 });
        assert_eq!(read_trying(r), Some(Trying { version: "0.6.61".into(), fails: 1, since: 5 }));
        mark_good(r, "0.6.61");
        assert!(is_good(r, "0.6.61"));
        assert_eq!(read_trying(r), None, "good clears the count");
        mark_bad(r, "0.6.61", "test");
        assert!(!is_good(r, "0.6.61"));
        assert_eq!(bad_reason(r, "0.6.61").as_deref(), Some("test"));
        mark_good(r, "../../x");
        assert!(!r.join("x").exists() && !r.parent().unwrap().join("x").exists());
    }

    /// A real signed bundle in `root` (key made here, so the test can sign).
    fn signed_bundle(root: &Path, version: &str, program: &[u8]) -> (Image, ed25519_dalek::SigningKey) {
        use std::os::unix::fs::PermissionsExt;
        let sk = ed25519_dalek::SigningKey::from_bytes(&[42u8; 32]);
        let mut im = img();
        im.keys = vec![sk.verifying_key().to_bytes()];
        let dir = root.join(version);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("nimbus"), program).unwrap();
        std::fs::set_permissions(dir.join("nimbus"), std::fs::Permissions::from_mode(0o755)).unwrap();
        let m = manifest(version, vec![file("nimbus", program, 0o755)]);
        let bytes = serde_json::to_vec(&m).unwrap();
        std::fs::write(dir.join("manifest.json"), &bytes).unwrap();
        std::fs::write(dir.join("manifest.json.sig"), sign(&sk, &bytes)).unwrap();
        switch_current(root, version).unwrap();
        (im, sk)
    }
    /// SSHSIG, as ssh-keygen -Y sign makes it.
    fn sign(sk: &ed25519_dalek::SigningKey, msg: &[u8]) -> String {
        use ed25519_dalek::Signer;
        use sha2::Digest;
        let mut pk = Vec::new(); sshstr(&mut pk, b"ssh-ed25519"); sshstr(&mut pk, &sk.verifying_key().to_bytes());
        let h = sha2::Sha512::digest(msg);
        let mut signed = b"SSHSIG".to_vec(); sshstr(&mut signed, NAMESPACE.as_bytes()); sshstr(&mut signed, b""); sshstr(&mut signed, b"sha512"); sshstr(&mut signed, &h);
        let s = sk.sign(&signed).to_bytes();
        let mut sig = Vec::new(); sshstr(&mut sig, b"ssh-ed25519"); sshstr(&mut sig, &s);
        let mut blob = b"SSHSIG".to_vec(); blob.extend_from_slice(&1u32.to_be_bytes());
        sshstr(&mut blob, &pk); sshstr(&mut blob, NAMESPACE.as_bytes()); sshstr(&mut blob, b""); sshstr(&mut blob, b"sha512"); sshstr(&mut blob, &sig);
        let s = b64e(&blob);
        format!("-----BEGIN SSH SIGNATURE-----\n{s}\n-----END SSH SIGNATURE-----\n")
    }
    fn b64e(b: &[u8]) -> String {
        const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut s = String::new();
        for c in b.chunks(3) {
            let x = [c[0], *c.get(1).unwrap_or(&0), *c.get(2).unwrap_or(&0)];
            let v = (x[0] as u32) << 16 | (x[1] as u32) << 8 | x[2] as u32;
            for i in 0..4 { s.push(if i <= c.len() { T[(v >> (18 - 6 * i) & 63) as usize] as char } else { '=' }); }
        }
        s
    }
    fn webkit(_: bool) -> Engine { Engine::WebKit }
    fn chromium(_: bool) -> Engine { Engine::Chromium }
    fn qt(_: bool) -> Engine { Engine::Qt }
    fn facts_for(root: &Path, im: Image) -> Facts {
        Facts { root: root.to_path_buf(), image: Ok(im), forced: None, mode: Mode::Auto, live: false, userns: true, engine: webkit }
    }
    fn is_bundle(c: &Choice) -> bool { matches!(c, Choice::Bundle { .. }) }

    #[test]
    fn picks_a_good_signed_bundle_and_falls_back_on_everything_else() {
        let t = tempfile::tempdir().unwrap();
        let r = t.path();
        let (im, _) = signed_bundle(r, "0.6.61", b"program");
        let f = facts_for(r, im.clone());
        assert!(is_bundle(&decide(&f, Context::Primary)));
        mark_good(r, "0.6.61");
        let builtin = |f: &Facts, ctx: Context, why: &str| match decide(f, ctx) { Choice::Builtin(w) => assert!(w.contains(why), "{w:?} should say {why:?}"), c => panic!("{c:?} should be the built-in ({why})") };
        builtin(&Facts { forced: Some("forced".into()), ..f.clone() }, Context::Primary, "forced");
        builtin(&Facts { live: true, ..f.clone() }, Context::Primary, "live USB");
        builtin(&Facts { mode: Mode::Off, ..f.clone() }, Context::Primary, "off");
        builtin(&Facts { image: Err("no app ABI".into()), ..f.clone() }, Context::Primary, "no app ABI");
        builtin(&Facts { image: Ok(Image { version: "0.6.61".into(), ..im.clone() }), ..f.clone() }, Context::Primary, "same or newer");
        builtin(&Facts { image: Ok(Image { abi: "2-new".into(), ..im.clone() }), ..f.clone() }, Context::Primary, "app ABI");
        builtin(&Facts { image: Ok(Image { keys: vec![[9u8; 32]], ..im.clone() }), ..f.clone() }, Context::Primary, "doesn't trust");
        builtin(&Facts { engine: qt, ..f.clone() }, Context::Primary, "Qt");
        builtin(&Facts { engine: chromium, ..f.clone() }, Context::Primary, "no Chromium window app");
        // Ask mode still uses an installed bundle (you accepted it).
        assert!(is_bundle(&decide(&Facts { mode: Mode::Ask, ..f.clone() }, Context::Primary)));
        // Secondary starts: only once it's been good.
        assert!(is_bundle(&decide(&f, Context::Secondary)));
        std::fs::remove_file(marks(r).join("good/0.6.61")).unwrap();
        builtin(&f, Context::Secondary, "hasn't come up");
        // A changed program.
        std::fs::write(r.join("0.6.61/nimbus"), b"PROGRAM").unwrap();
        builtin(&f, Context::Primary, "checksum");
        std::fs::write(r.join("0.6.61/nimbus"), b"program").unwrap();
        // A changed manifest (its signature no longer matches).
        let m = std::fs::read_to_string(r.join("0.6.61/manifest.json")).unwrap();
        std::fs::write(r.join("0.6.61/manifest.json"), m.replace("anatase", "anatasf")).unwrap();
        builtin(&f, Context::Primary, "signature doesn't match");
        std::fs::write(r.join("0.6.61/manifest.json"), &m).unwrap();
        std::fs::remove_file(r.join("0.6.61/manifest.json.sig")).unwrap();
        builtin(&f, Context::Primary, "signature is missing");
        std::fs::remove_file(r.join("current")).unwrap();
        builtin(&f, Context::Primary, "no Hearth app update");
    }

    #[test]
    fn two_starts_that_never_come_up_set_the_bundle_aside() {
        let t = tempfile::tempdir().unwrap();
        let r = t.path();
        let (im, _) = signed_bundle(r, "0.6.61", b"program");
        let f = facts_for(r, im);
        assert!(is_bundle(&decide(&f, Context::Primary)));
        assert_eq!(read_trying(r).unwrap().fails, 0);
        assert!(is_bundle(&decide(&f, Context::Primary)), "one failed start: tried again");
        assert_eq!(read_trying(r).unwrap().fails, 1);
        match decide(&f, Context::Primary) { Choice::Builtin(w) => assert!(w.contains("2 times in a row")), c => panic!("{c:?}") }
        assert!(bad_reason(r, "0.6.61").is_some());
        assert!(std::fs::read_to_string(marks(r).join("last-fallback")).unwrap().contains("in a row"));
        assert!(!is_bundle(&decide(&f, Context::Primary)), "stays aside");
        assert!(!is_bundle(&decide(&f, Context::Secondary)));
        // Secondary starts never count.
        let t2 = tempfile::tempdir().unwrap();
        let (im2, _) = signed_bundle(t2.path(), "0.6.61", b"program");
        mark_good(t2.path(), "0.6.61");
        let f2 = facts_for(t2.path(), im2);
        for _ in 0..5 { assert!(is_bundle(&decide(&f2, Context::Secondary))); }
        assert_eq!(read_trying(t2.path()), None);
        // A start that came up in between resets the count.
        assert!(is_bundle(&decide(&f2, Context::Primary)));
        mark_good(t2.path(), "0.6.61");
        assert!(is_bundle(&decide(&f2, Context::Primary)));
        assert_eq!(read_trying(t2.path()).unwrap().fails, 0);
    }

    #[test]
    fn chromium_needs_its_sandbox() {
        let t = tempfile::tempdir().unwrap();
        let r = t.path();
        let sk = ed25519_dalek::SigningKey::from_bytes(&[42u8; 32]);
        let mut im = img(); im.keys = vec![sk.verifying_key().to_bytes()];
        let dir = r.join("0.6.61");
        std::fs::create_dir_all(dir.join("electron")).unwrap();
        { use std::os::unix::fs::PermissionsExt;
          for p in ["nimbus", "electron/hearth-electron"] { std::fs::write(dir.join(p), b"x").unwrap(); std::fs::set_permissions(dir.join(p), std::fs::Permissions::from_mode(0o755)).unwrap(); } }
        let m = manifest("0.6.61", vec![file("nimbus", b"x", 0o755), file("electron/hearth-electron", b"x", 0o755)]);
        let bytes = serde_json::to_vec(&m).unwrap();
        std::fs::write(dir.join("manifest.json"), &bytes).unwrap();
        std::fs::write(dir.join("manifest.json.sig"), sign(&sk, &bytes)).unwrap();
        switch_current(r, "0.6.61").unwrap();
        let f = Facts { engine: chromium, ..facts_for(r, im) };
        assert!(is_bundle(&decide(&f, Context::Primary)));
        assert!(!is_bundle(&decide(&Facts { userns: false, ..f.clone() }, Context::Primary)));
    }

    #[test]
    fn installs_only_what_checks_out() {
        use std::io::Write;
        let has = |c: &str| std::process::Command::new(c).arg("--version").output().is_ok();
        if !has("zstd") || !has("tar") { eprintln!("zstd or tar missing: skipped"); return; }
        let t = tempfile::tempdir().unwrap();
        let src = t.path().join("src");
        std::fs::create_dir_all(src.join("electron")).unwrap();
        std::fs::write(src.join("nimbus"), b"#!/bin/sh\necho hi\n").unwrap();
        std::fs::write(src.join("electron/a.pak"), b"pak").unwrap();
        { use std::os::unix::fs::PermissionsExt; std::fs::set_permissions(src.join("nimbus"), std::fs::Permissions::from_mode(0o755)).unwrap(); }
        let tarf = t.path().join("b.tar");
        assert!(std::process::Command::new("tar").arg("-C").arg(&src).arg("-cf").arg(&tarf).arg(".").status().unwrap().success());
        let zst = t.path().join("b.tar.zst");
        assert!(std::process::Command::new("zstd").args(["-q", "-f", "-o"]).arg(&zst).arg(&tarf).status().unwrap().success());
        let sk = ed25519_dalek::SigningKey::from_bytes(&[42u8; 32]);
        let mut im = img(); im.keys = vec![sk.verifying_key().to_bytes()];
        let mut m = manifest("0.6.61", vec![file("nimbus", b"#!/bin/sh\necho hi\n", 0o755), file("electron/a.pak", b"pak", 0o644)]);
        m.size = std::fs::metadata(&zst).unwrap().len();
        m.sha256 = sha256_file(&zst).unwrap();
        let bytes = serde_json::to_vec(&m).unwrap();
        let sig = sign(&sk, &bytes);
        let root = t.path().join("app");
        assert_eq!(install_archive(&root, &im, &zst, &bytes, &sig), Ok("0.6.61".to_string()));
        assert_eq!(current_version(&root).as_deref(), Some("0.6.61"));
        assert!(is_bundle(&decide(&facts_for(&root, im.clone()), Context::Primary)));
        assert!(!root.join("staging/0.6.61.partial").exists());
        // Another key's signature, a damaged archive, a file the manifest doesn't list: nothing installed.
        let other = ed25519_dalek::SigningKey::from_bytes(&[1u8; 32]);
        let mut m2 = m.clone(); m2.version = "0.6.62".into();
        let b2 = serde_json::to_vec(&m2).unwrap();
        assert!(install_archive(&root, &im, &zst, &b2, &sign(&other, &b2)).unwrap_err().contains("doesn't trust"));
        let mut damaged = std::fs::read(&zst).unwrap(); let n = damaged.len() / 2; damaged[n] ^= 0xff;
        let dz = t.path().join("d.tar.zst"); std::fs::File::create(&dz).unwrap().write_all(&damaged).unwrap();
        assert!(install_archive(&root, &im, &dz, &b2, &sign(&sk, &b2)).unwrap_err().contains("checksum"));
        let mut m3 = m2.clone(); m3.files.pop();
        let b3 = serde_json::to_vec(&m3).unwrap();
        assert!(install_archive(&root, &im, &zst, &b3, &sign(&sk, &b3)).unwrap_err().contains("isn't in the bundle's manifest"));
        assert_eq!(current_version(&root).as_deref(), Some("0.6.61"), "a failed install leaves the current one");
        assert!(!root.join("0.6.62").exists());
        // A good newer one: current moves, previous is kept.
        let b4 = serde_json::to_vec(&m2).unwrap();
        assert_eq!(install_archive(&root, &im, &zst, &b4, &sign(&sk, &b4)), Ok("0.6.62".to_string()));
        assert_eq!(previous_version(&root).as_deref(), Some("0.6.61"));
        assert!(root.join("0.6.61/nimbus").is_file());
    }
}
