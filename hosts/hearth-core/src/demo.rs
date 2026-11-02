//! Show demo (docs/DEMO.md): Quad Mech, Hearth's multi-seat demo game (games/quadmech). CI builds it and puts it on
//! every Hearth OS release (.github/workflows/quadmech.yml): QuadMech-linux-x86_64.zip and quadmech.json, which says
//! the version and each zip's address, SHA-256 and size.
//!
//! * `demo.install` downloads the Linux zip (this repository's releases only, https, size cap, SHA-256), checks every
//!   name in it (no absolute paths, no "..", no links), unpacks it into ~/.local/share/nimbus/demo/quadmech/<version>/
//!   and keeps the zip beside it (the other seats get a copy: their accounts can't read player 1's home).
//! * `demo.start` picks how: two seats on this screen (multi-seat's split screen, set up or started for the demo:
//!   `hearth-seats demo-plan`), one copy on this screen, or the self-playing attract mode when no controller is
//!   connected. Each seat's Hearth starts the game itself from a request file (see [`Request`]): player 1's in
//!   ~/.local/share/nimbus/demo/autostart.json (it survives Game Mode restarting to split the screen), player N's in
//!   /run/hearth-seats/hearth-pN/demo.json (player 1's folder for that player, group hearth-pN: only player 1 and
//!   root can write there). The request holds no command line: only a role (host, join, local), attract on/off and
//!   a player name, from which [`build_args`] makes the fixed argument sets. Hearth starts the game through the
//!   library (Quick Resume, closing, the watchdog and gamescope's window ids work as for any game).
//! * `demo.stop` closes the game, removes the requests (player N's Hearth sees its request go and closes its copy),
//!   and turns off or removes the multi-seat set-up if the demo made it.
//!
//! Long work runs on its own thread and reports as events: `demo.progress` {action, pct, text} and `demo.done`
//! {action, ok, error?, ...}. Nothing here holds the bridge: player 1's own game is started by an internal message
//! ("demoLaunch") that the bridge handles on its own thread.

use crate::system::bt_scan::Emit;
use serde_json::{json, Value};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub const MANIFEST_URL: &str = "https://github.com/Dragongamer9/hearth-os/releases/latest/download/quadmech.json";
const RELEASES: &str = "https://github.com/Dragongamer9/hearth-os/releases/";
pub const LINUX_ZIP: &str = "QuadMech-linux-x86_64.zip";
pub const WINDOWS_ZIP: &str = "QuadMech-windows-x64.zip";
pub const BINARY: &str = "QuadMech.x86_64";
pub const GAME_ID: &str = "demo:quadmech";
pub const TITLE: &str = "Quad Mech (demo)";
/// The biggest zip Hearth downloads (the game is one file of about 70 MB, zipped smaller).
pub const MAX_ZIP: u64 = 400 * 1024 * 1024;
const MAX_UNPACKED: u64 = 1024 * 1024 * 1024;
const MAX_ENTRIES: usize = 64;
/// A request older than this is ignored (a PC turned off mid-demo doesn't start it again days later).
const REQUEST_TTL_MS: u64 = 15 * 60 * 1000;
/// Attract mode: the game's --selftest drives the mechs by itself for this long, then quits.
pub const ATTRACT_SECONDS: u32 = 3600;
pub const SEAT_RUN: &str = "/run/hearth-seats";
pub const REQUEST_FILE: &str = "demo.json";
pub const HANDOFF_ZIP: &str = "demo-quadmech.zip";

fn now_ms() -> u64 { crate::library::now_ms() }
fn log(t: &str) { crate::bridge::log(&format!("demo: {t}")); }

// ------------------------------------------------------------------------------------------------ checks
/// A version from quadmech.json ("20261020-1a2b3c4"): it names a folder, so letters, digits, ".", "_", "-" only,
/// starting with a letter or digit (never "." or "..").
pub fn valid_version(v: &str) -> bool {
    let b = v.as_bytes();
    !b.is_empty() && b.len() <= 64 && b[0].is_ascii_alphanumeric() && b.iter().all(|c| c.is_ascii_alphanumeric() || b"._-".contains(c))
}
pub fn valid_sha256(s: &str) -> bool { s.len() == 64 && s.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)) }

/// Only this repository's release downloads, over https, of exactly that file: …/releases/download/<tag>/<file> or
/// …/releases/latest/download/<file>.
pub fn valid_asset_url(url: &str, file: &str) -> bool {
    let Some(rest) = url.strip_prefix(RELEASES) else { return false };
    if let Some(f) = rest.strip_prefix("latest/download/") { return f == file; }
    let Some(r) = rest.strip_prefix("download/") else { return false };
    let Some((tag, f)) = r.split_once('/') else { return false };
    f == file && tag.len() > 1 && tag.len() <= 40 && tag.starts_with('v') && tag[1..].bytes().all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
}

#[derive(Debug, Clone, PartialEq)]
pub struct Asset { pub url: String, pub sha256: String, pub size: u64 }
#[derive(Debug, Clone, PartialEq)]
pub struct Manifest { pub version: String, pub linux: Asset }

pub fn parse_asset(v: &Value, file: &str) -> Result<Asset, String> {
    let url = v["url"].as_str().unwrap_or("");
    if !valid_asset_url(url, file) { return Err(format!("The demo's download ({file}) isn't from Hearth's releases, so it wasn't used.")); }
    let sha = v["sha256"].as_str().unwrap_or("").to_ascii_lowercase();
    if !valid_sha256(&sha) { return Err("The demo's checksum is missing.".into()); }
    let size = v["size"].as_u64().unwrap_or(0);
    if size == 0 || size > MAX_ZIP { return Err("The demo's download size isn't right.".into()); }
    Ok(Asset { url: url.to_string(), sha256: sha, size })
}

/// quadmech.json: {version, files: {linux: {url, sha256, size}, windows: {…}}}
pub fn parse_manifest(v: &Value) -> Result<Manifest, String> {
    let version = v["version"].as_str().unwrap_or("");
    if !valid_version(version) { return Err("The demo's version isn't right.".into()); }
    Ok(Manifest { version: version.to_string(), linux: parse_asset(&v["files"]["linux"], LINUX_ZIP)? })
}

// ------------------------------------------------------------------------------------------------ zip files
#[derive(Debug, Clone, PartialEq)]
pub struct ZipFile { pub name: String, pub size: u64 }

fn u16le(b: &[u8], i: usize) -> Option<u64> { b.get(i..i + 2).map(|s| u16::from_le_bytes([s[0], s[1]]) as u64) }
fn u32le(b: &[u8], i: usize) -> Option<u64> { b.get(i..i + 4).map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]) as u64) }

/// A name that can only land inside the folder it's unpacked into: relative, no "." or ".." parts, plain characters.
pub fn safe_name(name: &str) -> bool {
    if name.is_empty() || name.len() > 200 || name.starts_with('/') { return false; }
    if !name.bytes().all(|c| c.is_ascii_alphanumeric() || b"._-/".contains(&c)) { return false; }
    let trimmed = name.strip_suffix('/').unwrap_or(name);
    !trimmed.is_empty() && trimmed.split('/').all(|p| !p.is_empty() && p != "." && p != "..")
}

/// The files in a zip, read from its central directory and checked: safe names, no links or special files, no
/// encryption, stored or deflated only, no zip64, at most MAX_ENTRIES files and MAX_UNPACKED bytes, and `need` (the
/// game) at the top. `tail` is the end of the file, starting at byte `tail_at` (the directory must be in it).
pub fn zip_entries(tail: &[u8], tail_at: u64, need: &str) -> Result<Vec<ZipFile>, String> {
    let bad = |m: &str| -> Result<Vec<ZipFile>, String> { Err(format!("The demo's download isn't a zip Hearth can use ({m}).")) };
    if tail.len() < 22 { return bad("too short"); }
    // The end-of-directory record: the last 22 bytes, or earlier when the zip has a comment (up to 64 KiB).
    let mut eocd = None;
    let mut i = tail.len() - 22;
    loop {
        if u32le(tail, i) == Some(0x0605_4b50) { eocd = Some(i); break; }
        if i == 0 || tail.len() - i > 22 + 65535 { break; }
        i -= 1;
    }
    let Some(e) = eocd else { return bad("no directory") };
    let count = u16le(tail, e + 10).unwrap_or(0) as usize;
    let cd_size = u32le(tail, e + 12).unwrap_or(0);
    let cd_at = u32le(tail, e + 16).unwrap_or(0);
    if count == 0xFFFF || cd_size == 0xFFFF_FFFF || cd_at == 0xFFFF_FFFF { return bad("zip64"); }
    if count == 0 || count > MAX_ENTRIES { return bad("number of files"); }
    if cd_at < tail_at { return bad("directory out of reach"); }
    let mut p = (cd_at - tail_at) as usize;
    let end = match p.checked_add(cd_size as usize) { Some(x) if x <= e => x, _ => return bad("directory size") };
    let mut out: Vec<ZipFile> = Vec::new();
    let mut total: u64 = 0;
    for _ in 0..count {
        if u32le(tail, p) != Some(0x0201_4b50) { return bad("directory entry"); }
        let made_by_os = tail.get(p + 5).copied().unwrap_or(0);
        let flags = u16le(tail, p + 8).unwrap_or(1);
        let method = u16le(tail, p + 10).unwrap_or(99);
        let size = u32le(tail, p + 24).unwrap_or(0xFFFF_FFFF);
        let nlen = u16le(tail, p + 28).unwrap_or(0) as usize;
        let xlen = u16le(tail, p + 30).unwrap_or(0) as usize;
        let clen = u16le(tail, p + 32).unwrap_or(0) as usize;
        let attr = u32le(tail, p + 38).unwrap_or(0);
        let Some(nb) = tail.get(p + 46..p + 46 + nlen) else { return bad("name") };
        let Ok(name) = std::str::from_utf8(nb) else { return bad("name") };
        if !safe_name(name) { return bad("a file name that could land outside its folder"); }
        if flags & 1 != 0 { return bad("encrypted"); }
        if method != 0 && method != 8 { return bad("compression"); }
        if size == 0xFFFF_FFFF { return bad("zip64"); }
        // Made on Unix: the top of the attributes is the file's mode; only plain files and folders.
        if made_by_os == 3 {
            let kind = (attr >> 16) & 0o170000;
            if kind != 0 && kind != 0o100000 && kind != 0o040000 { return bad("a link or special file"); }
        }
        p += 46 + nlen + xlen + clen;
        if p > end { return bad("directory entry"); }
        if name.ends_with('/') { continue; }
        total += size;
        if total > MAX_UNPACKED { return bad("too big unpacked"); }
        if out.iter().any(|f| f.name == name) { return bad("a file twice"); }
        out.push(ZipFile { name: name.to_string(), size });
    }
    if !out.iter().any(|f| f.name == need) { return Err("The demo's download doesn't have the game in it.".into()); }
    Ok(out)
}

fn read_zip_entries(path: &Path, need: &str) -> Result<Vec<ZipFile>, String> {
    let mut f = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let len = f.metadata().map_err(|e| e.to_string())?.len();
    let n = len.min(256 * 1024);
    f.seek(SeekFrom::Start(len - n)).map_err(|e| e.to_string())?;
    let mut buf = vec![0u8; n as usize];
    f.read_exact(&mut buf).map_err(|e| e.to_string())?;
    zip_entries(&buf, len - n, need)
}

/// After unpacking: only the files the directory listed, all plain files (no links, devices or pipes), within the cap.
pub fn check_tree(root: &Path, files: &[ZipFile]) -> Result<(), String> {
    let allowed: std::collections::HashSet<&str> = files.iter().map(|f| f.name.as_str()).collect();
    let (mut seen, mut total) = (0usize, 0u64);
    let mut stack = vec![root.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).map_err(|e| e.to_string())? {
            let p = e.map_err(|e| e.to_string())?.path();
            let m = std::fs::symlink_metadata(&p).map_err(|e| e.to_string())?;
            let ft = m.file_type();
            if ft.is_symlink() { return Err("The demo's download had a link in it, so it wasn't used.".into()); }
            if ft.is_dir() { stack.push(p); continue; }
            if !ft.is_file() { return Err("The demo's download had a special file in it, so it wasn't used.".into()); }
            let rel = p.strip_prefix(root).map(|r| r.to_string_lossy().to_string()).unwrap_or_default();
            if !allowed.contains(rel.as_str()) { return Err(format!("The demo's download unpacked a file it didn't list ({rel}).")); }
            total += m.len();
            if total > MAX_UNPACKED { return Err("The demo unpacked bigger than it said.".into()); }
            seen += 1;
        }
    }
    if seen != files.len() { return Err("Some of the demo's files didn't unpack.".into()); }
    Ok(())
}

/// Unpacks a checked zip into `dest` (through a staging folder beside it, so a failure leaves nothing half done):
/// Python's zipfile (on every Hearth OS), or unzip. The game is made executable.
fn unpack(zip: &Path, files: &[ZipFile], dest: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let name = dest.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let staging = dest.with_file_name(format!(".unpack-{name}"));
    let _ = std::fs::remove_dir_all(&staging);
    std::fs::create_dir_all(&staging).map_err(|e| e.to_string())?;
    let (z, s) = (zip.to_string_lossy().to_string(), staging.to_string_lossy().to_string());
    let limit = Duration::from_secs(300);
    let r = crate::system::run_timeout("python3", &["-I", "-m", "zipfile", "-e", &z, &s], limit)
        .or_else(|e| if crate::system::has("unzip") { crate::system::run_timeout("unzip", &["-qq", "-o", &z, "-d", &s], limit) } else { Err(e) });
    let r = r.and_then(|_| check_tree(&staging, files)).and_then(|_| {
        for f in files {
            let mode = if f.name == BINARY || f.name.ends_with(".exe") { 0o755 } else { 0o644 };
            std::fs::set_permissions(staging.join(&f.name), std::fs::Permissions::from_mode(mode)).map_err(|e| e.to_string())?;
        }
        Ok(())
    });
    if let Err(e) = r { let _ = std::fs::remove_dir_all(&staging); return Err(format!("Couldn't unpack the demo: {e}")); }
    let _ = std::fs::remove_dir_all(dest);
    std::fs::rename(&staging, dest).map_err(|e| e.to_string())
}

// ------------------------------------------------------------------------------------------------ installed copy
pub fn demo_dir() -> PathBuf { crate::paths::data_dir().join("demo") }
pub fn game_root() -> PathBuf { demo_dir().join("quadmech") }
fn state_path() -> PathBuf { demo_dir().join("seats.json") }
fn seen_path() -> PathBuf { demo_dir().join("seen") }
fn autostart_path() -> PathBuf { demo_dir().join("autostart.json") }

/// The installed version and its folder.
pub fn installed() -> Option<(String, PathBuf)> {
    let root = game_root();
    let v = std::fs::read_to_string(root.join("current")).ok()?.trim().to_string();
    if !valid_version(&v) { return None; }
    let dir = root.join(&v);
    if dir.join(BINARY).is_file() { Some((v, dir)) } else { None }
}

/// The library's entry once installed: "Quad Mech (demo)", with the art that came in the zip.
pub fn library_game() -> Option<crate::library::Game> {
    let (_, dir) = installed()?;
    let exe = dir.join(BINARY).to_string_lossy().to_string();
    let added = std::fs::metadata(game_root().join("current")).and_then(|m| m.modified()).ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_millis() as u64).unwrap_or(0);
    let art: Vec<(&'static str, String)> = [("cover", "cover.png"), ("wide", "wide.png"), ("hero", "hero.png")].iter()
        .filter(|(_, f)| dir.join(f).is_file()).map(|(k, f)| (*k, dir.join(f).to_string_lossy().to_string())).collect();
    Some(crate::library::Game { id: GAME_ID.to_string(), title: TITLE.to_string(), source: "demo", install_path: Some(dir), installed: true,
        state: "installed", exe: Some(exe), art, added, ..Default::default() })
}

fn sha256_file(p: &Path) -> Result<String, String> {
    let out = crate::system::run_timeout("sha256sum", &[&p.to_string_lossy()], Duration::from_secs(180))?;
    Ok(out.split_whitespace().next().unwrap_or("").to_ascii_lowercase())
}

/// A zip (downloaded, or handed over by player 1), checked and unpacked: becomes the installed demo.
fn install_zip(zip: &Path, version: &str, sha: &str, size: u64, progress: &dyn Fn(f64, &str)) -> Result<PathBuf, String> {
    if !valid_version(version) || !valid_sha256(sha) { return Err("The demo's version or checksum isn't right.".into()); }
    let len = std::fs::metadata(zip).map(|m| m.len()).map_err(|e| e.to_string())?;
    if len != size { return Err(format!("The demo's download is the wrong size ({len} of {size} bytes).")); }
    progress(0.92, "Checking the download");
    if sha256_file(zip)? != sha { return Err("The demo's download didn't match its checksum, so it wasn't used.".into()); }
    let files = read_zip_entries(zip, BINARY)?;
    progress(0.96, "Unpacking");
    let root = game_root();
    let dest = root.join(version);
    unpack(zip, &files, &dest)?;
    // Kept as <version>.zip: Show demo hands a copy to the other seats.
    let keep = root.join(format!("{version}.zip"));
    if zip != keep.as_path() {
        let _ = std::fs::remove_file(&keep);
        if std::fs::rename(zip, &keep).is_err() { let _ = std::fs::copy(zip, &keep); let _ = std::fs::remove_file(zip); }
    }
    crate::paths::write_atomic(&root.join("current"), version.as_bytes()).map_err(|e| e.to_string())?;
    // Older versions go.
    let keep_zip = format!("{version}.zip");
    if let Ok(rd) = std::fs::read_dir(&root) {
        for e in rd.flatten() {
            let n = e.file_name().to_string_lossy().to_string();
            if n == "current" || n == version || n == keep_zip || n.starts_with('.') { continue; }
            let p = e.path();
            if std::fs::symlink_metadata(&p).map(|m| m.is_dir()).unwrap_or(false) { let _ = std::fs::remove_dir_all(&p); } else { let _ = std::fs::remove_file(&p); }
        }
    }
    log(&format!("installed {version}"));
    Ok(dest)
}

fn mb(n: u64) -> String { format!("{:.0} MB", n as f64 / 1_048_576.0) }

fn curl_error(code: Option<i32>) -> String {
    match code {
        Some(6) | Some(7) => "No connection to GitHub. Check the network and try again.".into(),
        Some(22) => "GitHub didn't have the demo (it comes with each Hearth OS release). Try again later.".into(),
        Some(28) => "The download took too long. Try again.".into(),
        Some(63) => "The demo's download is bigger than it should be, so it was stopped.".into(),
        Some(c) => format!("The download didn't work (curl {c})."),
        None => "The download was stopped.".into(),
    }
}

/// Downloads a release asset with curl (https only, also on redirects; size cap; timeouts), reporting progress.
fn download(asset: &Asset, to: &Path, progress: &dyn Fn(f64, &str)) -> Result<(), String> {
    let _ = std::fs::remove_file(to);
    if let Some(d) = to.parent() { std::fs::create_dir_all(d).map_err(|e| e.to_string())?; }
    let max = asset.size.to_string();
    let mut child = std::process::Command::new("curl")
        .args(["-fsSL", "--proto", "=https", "--proto-redir", "=https", "--retry", "3", "--connect-timeout", "20", "-m", "1800", "-A", "Hearth",
               "--max-filesize", max.as_str(), "-o"])
        .arg(to).arg(&asset.url)
        .stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null())
        .spawn().map_err(|e| format!("Couldn't download the demo: {e}"))?;
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(st)) if st.success() => return Ok(()),
            Ok(Some(st)) => { let _ = std::fs::remove_file(to); return Err(curl_error(st.code())); }
            Ok(None) => {
                let done = std::fs::metadata(to).map(|m| m.len()).unwrap_or(0);
                let too_big = done > asset.size;
                if too_big || start.elapsed() > Duration::from_secs(1900) {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = std::fs::remove_file(to);
                    return Err(if too_big { curl_error(Some(63)) } else { curl_error(Some(28)) });
                }
                progress(0.02 + 0.88 * (done as f64 / asset.size as f64), &format!("Downloading the demo: {} of {}", mb(done), mb(asset.size)));
                std::thread::sleep(Duration::from_millis(500));
            }
            Err(e) => { let _ = child.kill(); let _ = child.wait(); return Err(e.to_string()); }
        }
    }
}

fn fetch_manifest(timeout_s: u32) -> Result<Manifest, String> {
    let r = crate::http::request("GET", MANIFEST_URL, &[], None, timeout_s)?;
    if !r.ok() { return Err(if r.status == 404 { "GitHub doesn't have the demo yet (it comes with each Hearth OS release).".into() } else { format!("GitHub answered {}", r.status) }); }
    parse_manifest(&r.json())
}

// ------------------------------------------------------------------------------------------------ seats and requests
/// This Hearth is an extra player's (multi-seat): their number (2-4).
pub fn extra_seat() -> Option<u8> {
    std::env::var("HEARTH_SEAT").ok().and_then(|s| s.parse::<u8>().ok()).filter(|n| (2..=4).contains(n))
}
pub fn seat_dir(n: u8) -> PathBuf { PathBuf::from(SEAT_RUN).join(format!("hearth-p{n}")) }
/// Where this Hearth looks for its request: player N's folder from player 1, or player 1's own file.
pub fn request_path() -> PathBuf { match extra_seat() { Some(n) => seat_dir(n).join(REQUEST_FILE), None => autostart_path() } }

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Role { Host, Join, Local }
impl Role {
    pub fn as_str(&self) -> &'static str { match self { Role::Host => "host", Role::Join => "join", Role::Local => "local" } }
    pub fn parse(s: &str) -> Option<Role> { match s { "host" => Some(Role::Host), "join" => Some(Role::Join), "local" => Some(Role::Local), _ => None } }
}

/// "Start the demo" for one seat's Hearth. No command line in it: [`build_args`] makes the game's arguments.
#[derive(Debug, Clone, PartialEq)]
pub struct Request { pub id: String, pub created: u64, pub version: String, pub sha256: String, pub size: u64, pub role: Role, pub attract: bool, pub name: String, pub zip: bool }

pub fn valid_name(n: &str) -> bool { matches!(n, "Seat1" | "Seat2" | "Seat3" | "Seat4") }

pub fn parse_request(v: &Value) -> Result<Request, String> {
    let s = |k: &str| v[k].as_str().unwrap_or("").to_string();
    let id = s("id");
    if !(16..=64).contains(&id.len()) || !id.bytes().all(|c| c.is_ascii_hexdigit()) { return Err("bad id".into()); }
    let version = s("version");
    if !valid_version(&version) { return Err("bad version".into()); }
    let sha256 = s("sha256");
    if !valid_sha256(&sha256) { return Err("bad checksum".into()); }
    let size = v["size"].as_u64().filter(|n| *n > 0 && *n <= MAX_ZIP).ok_or("bad size")?;
    let role = Role::parse(&s("role")).ok_or("bad role")?;
    let name = s("name");
    if !valid_name(&name) { return Err("bad name".into()); }
    let zip = match &v["zip"] { Value::Null => false, Value::String(z) if z == HANDOFF_ZIP => true, _ => return Err("bad zip".into()) };
    let created = v["created"].as_u64().ok_or("bad time")?;
    Ok(Request { id, created, version, sha256, size, role, attract: v["attract"] == true, name, zip })
}

pub fn request_json(r: &Request) -> Value {
    json!({"id": r.id, "created": r.created, "version": r.version, "sha256": r.sha256, "size": r.size, "role": r.role.as_str(),
           "attract": r.attract, "name": r.name, "zip": if r.zip { Some(HANDOFF_ZIP) } else { None }})
}

/// The game's arguments for each seat (after Godot's "--"): seat 1 hosts a co-op match, the others join the first
/// match found (on this PC: the game finds other seats' matches by itself), or one copy plays split-screen on this
/// screen. Each copy takes the controllers it can see (`--players=auto`): multi-seat gives every seat only its own.
/// Attract mode adds the game's self-test (the mechs walk, aim and fire by themselves) at low quality.
pub fn build_args(role: Role, attract: bool, name: &str) -> Vec<String> {
    let name = if valid_name(name) { name } else { "Seat1" };
    let mut a: Vec<String> = vec!["--".into()];
    match role {
        Role::Host => a.extend(["--host".to_string(), "--mode=coop".into(), format!("--name={name}"), "--players=auto".into()]),
        Role::Join => a.extend(["--join=auto".to_string(), format!("--name={name}"), "--players=auto".into()]),
        Role::Local => a.extend(["--local".to_string(), "--players=auto".into()]),
    }
    if attract { a.push(format!("--selftest={ATTRACT_SECONDS}")); a.push("--quality=low".into()); }
    a
}

/// The internal message "demoLaunch" {role, attract, name} → the game's arguments (checked again).
pub fn launch_args(v: &Value) -> Result<Vec<String>, String> {
    let role = Role::parse(v["role"].as_str().unwrap_or("")).ok_or("Unknown demo role")?;
    let name = v["name"].as_str().unwrap_or("Seat1");
    if !valid_name(name) { return Err("Unknown demo player".into()); }
    Ok(build_args(role, v["attract"] == true, name))
}

/// How Show demo plays: on two seats of this screen or one copy here, and whether it plays by itself (no controllers).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Choice { pub seats: bool, pub attract: bool }
/// `asked`: "seats", "local", "attract" or nothing (Hearth picks). Seats only when multi-seat can split this screen;
/// attract mode uses seats only when they're already set up (it never asks to set them up).
pub fn pick_mode(asked: Option<&str>, controllers: u64, seats_possible: bool, seats_existing: bool) -> Choice {
    let attract = match asked { Some("attract") => true, Some(_) => false, None => controllers == 0 };
    let seats = match asked { Some("local") => false, _ if attract => seats_existing && seats_possible, _ => seats_possible };
    Choice { seats, attract }
}

fn new_id() -> String {
    let mut b = [0u8; 16];
    if getrandom::getrandom(&mut b).is_err() { return format!("{:032x}", now_ms() as u128 * 7919); }
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn write_private(path: &Path, bytes: &[u8], mode: u32) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let tmp = path.with_file_name(format!(".{name}.tmp"));
    std::fs::write(&tmp, bytes).map_err(|e| e.to_string())?;
    std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(mode)).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())
}

/// Player N's request, and a copy of the zip (their account can't read player 1's home). The folder is player 1's
/// with group hearth-pN (setgid), so the files are readable by that player only (0640).
fn write_seat_request(n: u8, zip: &Path, req: &Request) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let dir = seat_dir(n);
    if !dir.is_dir() { return Err(format!("Player {n}'s seat isn't ready ({} is missing).", dir.display())); }
    let tmp = dir.join(format!(".{HANDOFF_ZIP}.tmp"));
    std::fs::copy(zip, &tmp).map_err(|e| format!("Couldn't give player {n} the demo: {e}"))?;
    std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o640)).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, dir.join(HANDOFF_ZIP)).map_err(|e| e.to_string())?;
    write_private(&dir.join(REQUEST_FILE), format!("{}\n", request_json(req)).as_bytes(), 0o640)
}

/// Every request goes (player 1's own and each seat's, with the zips). Each seat's Hearth sees its request go and
/// closes its copy of the game.
pub fn clear_requests() {
    let _ = std::fs::remove_file(autostart_path());
    if extra_seat().is_some() { return; }
    for n in 2..=4u8 {
        let d = seat_dir(n);
        let _ = std::fs::remove_file(d.join(REQUEST_FILE));
        let _ = std::fs::remove_file(d.join(HANDOFF_ZIP));
    }
}

/// /proc/bus/input/devices: how many devices have a joystick handler (js0, js1...): the controllers connected.
pub fn count_pads_in(text: &str) -> u64 {
    text.lines().filter_map(|l| l.strip_prefix("H: Handlers=")).filter(|h| h.split_whitespace().any(|t| t.starts_with("js"))).count() as u64
}
fn count_pads() -> u64 { count_pads_in(&std::fs::read_to_string("/proc/bus/input/devices").unwrap_or_default()) }

fn demo_plan() -> Option<Value> {
    if extra_seat().is_some() || !crate::multiplay::seats_available() { return None; }
    match crate::multiplay::ask(crate::multiplay::SEATS, &["demo-plan"]) { Ok(v) if v.is_object() => Some(v), Ok(_) => None, Err(e) => { log(&format!("demo-plan: {e}")); None } }
}

// ------------------------------------------------------------------------------------------------ the bridge's side
/// What's happening: {downloading, pct, text, error, busy, latest: {version, size}}. Shared with the jobs' threads;
/// the lock is only held to read or patch it.
#[derive(Clone)]
pub struct Demo { job: Arc<Mutex<Value>> }

impl Default for Demo { fn default() -> Self { Demo::new() } }

impl Demo {
    pub fn new() -> Demo { Demo { job: Arc::new(Mutex::new(json!({"downloading": false, "pct": 0, "text": "", "error": null, "busy": null, "latest": null}))) } }
    fn set(&self, patch: Value) {
        if let (Ok(mut j), Some(p)) = (self.job.lock(), patch.as_object()) { for (k, v) in p { j[k] = v.clone(); } }
    }
    pub fn job(&self) -> Value { self.job.lock().map(|j| j.clone()).unwrap_or(Value::Null) }

    /// demo.status (run off the bridge thread: it asks hearth-seats, and GitHub for the download size the first time).
    pub fn status(&self) -> Value {
        let inst = installed();
        let plan = demo_plan();
        if inst.is_none() && self.job()["latest"].is_null() && self.job()["downloading"] != true {
            if let Ok(m) = fetch_manifest(8) { self.set(json!({"latest": {"version": m.version, "size": m.linux.size}})); }
        }
        let job = self.job();
        let controllers = plan.as_ref().and_then(|p| p["controllers"].as_u64()).unwrap_or_else(count_pads);
        let p = plan.clone().unwrap_or(Value::Null);
        json!({"platform": "linux", "installed": inst.is_some(), "version": inst.as_ref().map(|(v, _)| v.clone()),
               "downloading": job["downloading"] == true, "pct": job["pct"], "text": job["text"], "error": job["error"], "busy": job["busy"],
               "size": job["latest"]["size"], "latest": job["latest"]["version"],
               "seatsAvailable": p["possible"] == true, "seatsHow": p["how"], "seatsReason": p["reason"], "seatsConfigured": p["configured"] == true,
               "controllers": controllers, "extraSeat": extra_seat().is_some(), "active": state_path().exists() || autostart_path().exists(),
               "attractSeconds": ATTRACT_SECONDS})
    }

    /// demo.install: download, check and unpack on a thread; demo.progress then demo.done {action: "install"}.
    pub fn install(&self, emit: Emit) -> Value {
        if self.job()["downloading"] == true { return self.job(); }
        self.set(json!({"downloading": true, "pct": 0, "text": "Asking GitHub for the demo", "error": null}));
        let me = self.clone();
        std::thread::spawn(move || {
            let e2 = emit.clone();
            let m2 = me.clone();
            let progress = move |f: f64, text: &str| {
                let pct = (f.clamp(0.0, 1.0) * 100.0).round();
                m2.set(json!({"pct": pct, "text": text}));
                e2(json!({"type": "event", "name": "demo.progress", "data": {"action": "install", "pct": pct, "text": text}}));
            };
            let r = (|| -> Result<String, String> {
                let m = fetch_manifest(30)?;
                if installed().map(|(v, _)| v == m.version).unwrap_or(false) { return Ok(m.version); }
                me.set(json!({"latest": {"version": m.version, "size": m.linux.size}}));
                let tmp = game_root().join(".download.zip");
                download(&m.linux, &tmp, &progress)?;
                let r = install_zip(&tmp, &m.version, &m.linux.sha256, m.linux.size, &progress);
                let _ = std::fs::remove_file(&tmp);
                r.map(|_| m.version)
            })();
            match r {
                Ok(v) => {
                    me.set(json!({"downloading": false, "pct": 100, "text": "", "error": null}));
                    crate::stores::notify(json!({"type": "internal", "what": "rescan", "reply": null}));
                    emit(json!({"type": "event", "name": "demo.done", "data": {"action": "install", "ok": true, "version": v}}));
                }
                Err(e) => {
                    log(&format!("install: {e}"));
                    me.set(json!({"downloading": false, "error": e}));
                    emit(json!({"type": "event", "name": "demo.done", "data": {"action": "install", "ok": false, "error": e}}));
                }
            }
        });
        self.job()
    }

    /// demo.start {mode}: on a thread (setting up seats asks for root and can take a while); demo.done {action:
    /// "start", mode, seats, restartGameMode}. With restartGameMode the page restarts Game Mode, which comes back as
    /// the split screen's host, and every seat's Hearth starts its copy from its request.
    pub fn start(&self, mode: Option<String>, emit: Emit) -> Result<Value, String> {
        if extra_seat().is_some() { return Err("Show demo starts from player 1's Hearth.".into()); }
        if let Some(m) = &mode { if !matches!(m.as_str(), "seats" | "local" | "attract") { return Err("Unknown demo mode".into()); } }
        let (version, _) = installed().ok_or("Download the demo first.")?;
        if self.job()["busy"].is_string() { return Err("Show demo is busy: one moment.".into()); }
        self.set(json!({"busy": "start"}));
        let me = self.clone();
        std::thread::spawn(move || {
            let e2 = emit.clone();
            let progress = move |pct: u64, text: &str| e2(json!({"type": "event", "name": "demo.progress", "data": {"action": "start", "pct": pct, "text": text}}));
            let r = start_job(mode.as_deref(), &version, &progress, &emit);
            me.set(json!({"busy": null}));
            let data = match r {
                Ok(mut v) => { v["action"] = json!("start"); v["ok"] = json!(true); v }
                Err(e) => { log(&format!("start: {e}")); json!({"action": "start", "ok": false, "error": e}) }
            };
            emit(json!({"type": "event", "name": "demo.done", "data": data}));
        });
        Ok(json!({"starting": true}))
    }

    /// demo.stop (after the bridge closed this seat's copy): the requests go, and on player 1's Hearth the seats the
    /// demo turned on or set up are turned off or removed again; demo.done {action: "stop", restartGameMode}.
    pub fn stop(&self, emit: Emit) -> Value {
        clear_requests();
        if extra_seat().is_some() { return json!({"stopped": true}); }
        let st: Value = std::fs::read(state_path()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null);
        if st.is_null() {
            emit(json!({"type": "event", "name": "demo.done", "data": {"action": "stop", "ok": true, "restartGameMode": false}}));
            return json!({"stopped": true});
        }
        self.set(json!({"busy": "stop"}));
        let me = self.clone();
        std::thread::spawn(move || {
            let progress = |pct: u64, text: &str| emit(json!({"type": "event", "name": "demo.progress", "data": {"action": "stop", "pct": pct, "text": text}}));
            let r = (|| -> Result<bool, String> {
                if st["remove"] == true {
                    progress(20, "Back to one screen: removing the demo's second seat");
                    crate::multiplay::run(crate::multiplay::SEATS, &["remove"], &mut |p, t| progress(20 + p * 70 / 100, t))?;
                    return Ok(true);
                }
                if st["stop"] == true {
                    progress(30, "Back to one screen");
                    crate::multiplay::ask(crate::multiplay::SEATS, &["stop"])?;
                    return Ok(true);
                }
                Ok(false)
            })();
            me.set(json!({"busy": null}));
            let data = match r {
                Ok(restart) => { let _ = std::fs::remove_file(state_path()); json!({"action": "stop", "ok": true, "restartGameMode": restart}) }
                Err(e) => { log(&format!("stop: {e}")); json!({"action": "stop", "ok": false, "error": e}) }
            };
            emit(json!({"type": "event", "name": "demo.done", "data": data}));
        });
        json!({"stopping": true})
    }
}

fn save_state(st: &Value) { let _ = crate::paths::write_atomic(&state_path(), format!("{st}\n").as_bytes()); }

fn start_job(mode: Option<&str>, version: &str, progress: &dyn Fn(u64, &str), emit: &Emit) -> Result<Value, String> {
    progress(5, "Looking at the screens and controllers");
    let plan = demo_plan();
    let controllers = plan.as_ref().and_then(|p| p["controllers"].as_u64()).unwrap_or_else(count_pads);
    let possible = plan.as_ref().map(|p| p["possible"] == true).unwrap_or(false);
    let existing = plan.as_ref().map(|p| p["how"] == "existing").unwrap_or(false);
    let choice = pick_mode(mode, controllers, possible, existing);
    let launch = |role: Role, name: &str| emit(json!({"type": "internal", "what": "demoLaunch", "role": role.as_str(), "attract": choice.attract, "name": name}));
    let label = if choice.attract { "attract" } else if choice.seats { "seats" } else { "local" };
    clear_requests();
    let Some(plan) = plan.filter(|_| choice.seats) else {
        launch(Role::Local, "Seat1");
        let why = if mode == Some("seats") { plan_reason(possible) } else { Value::Null };
        return Ok(json!({"mode": label, "seats": false, "restartGameMode": false, "reason": why}));
    };
    // The seats: set up (a two-player split screen) or turned on for the demo, remembered so demo.stop undoes it.
    let mut st = json!({"remove": false, "stop": false, "at": now_ms()});
    let restart;
    if plan["how"] == "setup" {
        progress(15, "Splitting the screen into two seats (Hearth may ask for your password)");
        let layout = plan["layout"].to_string();
        crate::multiplay::run(crate::multiplay::SEATS, &["setup", "--seats", &layout, "--start"], &mut |p, t| progress(15 + p * 65 / 100, t))?;
        st["remove"] = json!(true);
        save_state(&st);
        restart = true;   // player 1's Game Mode restarts as the split screen's host
    } else if plan["enabled"] != true {
        progress(30, "Turning multi-seat on");
        crate::multiplay::ask(crate::multiplay::SEATS, &["start"])?;
        st["stop"] = json!(true);
        save_state(&st);
        restart = true;
    } else {
        save_state(&st);
        restart = plan["host_running"] != true;
    }
    progress(85, "Handing the demo to the other seats");
    let zip = game_root().join(format!("{version}.zip"));
    let size = std::fs::metadata(&zip).map(|m| m.len()).map_err(|_| "The demo's download is missing: download it again.".to_string())?;
    let sha = sha256_file(&zip)?;
    let now = now_ms();
    let mut seats: Vec<u8> = plan["seats"].as_array().map(|l| l.iter().filter_map(|s| {
        let n = s["seat"].as_u64().filter(|n| (2..=4).contains(n))? as u8;
        (s["user"].as_str() == Some(format!("hearth-p{n}").as_str())).then_some(n)
    }).collect()).unwrap_or_default();
    seats.sort_unstable();
    seats.dedup();
    if seats.is_empty() { return Err("Multi-seat has no second seat on this screen.".into()); }
    for n in &seats {
        let req = Request { id: new_id(), created: now, version: version.to_string(), sha256: sha.clone(), size, role: Role::Join, attract: choice.attract, name: format!("Seat{n}"), zip: true };
        write_seat_request(*n, &zip, &req)?;
    }
    if restart {
        let req = Request { id: new_id(), created: now, version: version.to_string(), sha256: sha, size, role: Role::Host, attract: choice.attract, name: "Seat1".into(), zip: false };
        let _ = std::fs::create_dir_all(demo_dir());
        write_private(&autostart_path(), format!("{}\n", request_json(&req)).as_bytes(), 0o600)?;
    } else {
        launch(Role::Host, "Seat1");
    }
    progress(100, "Starting");
    Ok(json!({"mode": label, "seats": true, "players": seats.len() + 1, "restartGameMode": restart}))
}

fn plan_reason(possible: bool) -> Value {
    if possible { return Value::Null; }
    demo_plan().map(|p| p["reason"].clone()).unwrap_or_else(|| json!("Two seats on one screen need Hearth OS's multi-seat."))
}

/// A seat's Hearth: the zip player 1 left in its folder, copied into this account's own and installed.
fn install_from_handoff(from: &Path, r: &Request) -> Result<(), String> {
    let root = game_root();
    std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    let copy = root.join(".handoff.zip");
    std::fs::copy(from, &copy).map_err(|e| format!("Couldn't read the demo player 1 handed over: {e}"))?;
    let r2 = install_zip(&copy, &r.version, &r.sha256, r.size, &|_, _| {});
    let _ = std::fs::remove_file(&copy);
    r2.map(|_| ())
}

/// Every 2 seconds (from Bridge::new): a new request for this seat starts the game (after installing the copy player 1
/// handed over, on another seat); on an extra seat, the request going away closes it.
pub fn watch_autostart() {
    // A windowed Hearth (Desktop Mode, a store sign-in) leaves the requests to the full-screen one.
    if crate::signin_handoff::windowed() { return; }
    std::thread::spawn(|| {
        let mut active: Option<String> = None;
        loop {
            std::thread::sleep(Duration::from_secs(2));
            if !crate::stores::notify_ready() { continue; }
            if let Some(msg) = autostart_step(&mut active) { crate::stores::notify(msg); }
        }
    });
}

fn autostart_step(active: &mut Option<String>) -> Option<Value> {
    let path = request_path();
    let seat = extra_seat().is_some();
    let req = std::fs::read(&path).ok().map(|b| serde_json::from_slice::<Value>(&b).map_err(|e| e.to_string()).and_then(|v| parse_request(&v)));
    let r = match req {
        None => {
            if seat && active.take().is_some() { return Some(json!({"type": "internal", "what": "demoStopSeat"})); }
            return None;
        }
        Some(Err(e)) => { log(&format!("request ignored: {e}")); if !seat { let _ = std::fs::remove_file(&path); } return None; }
        Some(Ok(r)) => r,
    };
    if active.as_deref() == Some(r.id.as_str()) { return None; }
    if !seat { let _ = std::fs::remove_file(&path); }
    *active = Some(r.id.clone());
    // Already started before this Hearth restarted: not again (the game kept running).
    if std::fs::read_to_string(seen_path()).map(|s| s.trim() == r.id).unwrap_or(false) { return None; }
    let _ = crate::paths::write_atomic(&seen_path(), r.id.as_bytes());
    if now_ms().saturating_sub(r.created) > REQUEST_TTL_MS { log("request too old: ignored"); return None; }
    if installed().map(|(v, _)| v != r.version).unwrap_or(true) {
        if !(seat && r.zip) { log("request for a version that isn't installed: ignored"); return None; }
        if let Err(e) = install_from_handoff(&path.with_file_name(HANDOFF_ZIP), &r) { log(&format!("handed-over demo: {e}")); return None; }
        crate::stores::notify(json!({"type": "internal", "what": "rescan", "reply": null}));
    }
    log(&format!("starting as {} ({})", r.name, r.role.as_str()));
    Some(json!({"type": "internal", "what": "demoLaunch", "role": r.role.as_str(), "attract": r.attract, "name": r.name}))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHA: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    #[test]
    fn manifest_only_from_this_repositorys_releases() {
        let good = json!({"version": "20261020-1a2b3c4", "files": {
            "linux": {"url": "https://github.com/Dragongamer9/hearth-os/releases/download/v0.2.2/QuadMech-linux-x86_64.zip", "sha256": SHA.to_uppercase(), "size": 31_000_000},
            "windows": {"url": "https://github.com/Dragongamer9/hearth-os/releases/download/v0.2.2/QuadMech-windows-x64.zip", "sha256": SHA, "size": 33_000_000}}});
        let m = parse_manifest(&good).unwrap();
        assert_eq!(m.version, "20261020-1a2b3c4");
        assert_eq!(m.linux.sha256, SHA);   // upper case accepted, kept lower case
        assert_eq!(m.linux.size, 31_000_000);
        assert!(parse_asset(&good["files"]["windows"], WINDOWS_ZIP).is_ok());
        assert!(valid_asset_url("https://github.com/Dragongamer9/hearth-os/releases/latest/download/QuadMech-linux-x86_64.zip", LINUX_ZIP));
        for url in ["http://github.com/Dragongamer9/hearth-os/releases/download/v0.2.2/QuadMech-linux-x86_64.zip",
                    "https://github.com/someone/hearth-os/releases/download/v0.2.2/QuadMech-linux-x86_64.zip",
                    "https://github.com/Dragongamer9/hearth-os/releases/download/v0.2.2/other.zip",
                    "https://github.com/Dragongamer9/hearth-os/releases/download/v0.2.2/QuadMech-linux-x86_64.zip?x=1",
                    "https://github.com/Dragongamer9/hearth-os/releases/download/../../x/QuadMech-linux-x86_64.zip",
                    "https://github.com/Dragongamer9/hearth-os/releases/download/v1/x/QuadMech-linux-x86_64.zip",
                    "https://github.com.evil.example/Dragongamer9/hearth-os/releases/download/v1/QuadMech-linux-x86_64.zip",
                    "https://evil.example/https://github.com/Dragongamer9/hearth-os/releases/download/v1/QuadMech-linux-x86_64.zip"] {
            let mut b = good.clone();
            b["files"]["linux"]["url"] = json!(url);
            assert!(parse_manifest(&b).is_err(), "{url}");
        }
        let mut b = good.clone(); b["files"]["linux"]["sha256"] = json!("abc"); assert!(parse_manifest(&b).is_err());
        let mut b = good.clone(); b["files"]["linux"]["sha256"] = json!(SHA.replace('0', "g")); assert!(parse_manifest(&b).is_err());
        let mut b = good.clone(); b["files"]["linux"]["size"] = json!(MAX_ZIP + 1); assert!(parse_manifest(&b).is_err());
        let mut b = good.clone(); b["files"]["linux"]["size"] = json!(0); assert!(parse_manifest(&b).is_err());
        let long = "x".repeat(65);
        for v in ["", "..", ".hidden", "1/2", "a b", long.as_str()] { let mut b = good.clone(); b["version"] = json!(v); assert!(parse_manifest(&b).is_err(), "{v}"); }
        assert!(parse_manifest(&json!({"version": "1"})).is_err());
    }

    /// A zip's central directory with one entry per (name, unix mode, size), plus the end record.
    fn zip_dir(entries: &[(&str, u32, u32)], prefix: usize) -> Vec<u8> {
        let mut cd = Vec::new();
        for (name, mode, size) in entries {
            cd.extend(0x0201_4b50u32.to_le_bytes());
            cd.extend([20u8, 3]);                         // made by: version 2.0, Unix
            cd.extend(20u16.to_le_bytes());               // needed
            cd.extend(0u16.to_le_bytes());                // flags
            cd.extend(8u16.to_le_bytes());                // deflate
            cd.extend([0u8; 8]);                          // time, date, crc
            cd.extend(size.to_le_bytes());                // compressed
            cd.extend(size.to_le_bytes());                // uncompressed
            cd.extend((name.len() as u16).to_le_bytes());
            cd.extend([0u8; 8]);                          // extra, comment, disk, internal attrs
            cd.extend((mode << 16).to_le_bytes());        // external attrs
            cd.extend(0u32.to_le_bytes());                // local header offset
            cd.extend(name.as_bytes());
        }
        let mut out = vec![0u8; prefix];                  // stands for the local headers and data
        let at = out.len() as u32;
        out.extend(&cd);
        out.extend(0x0605_4b50u32.to_le_bytes());
        out.extend([0u8; 4]);
        out.extend((entries.len() as u16).to_le_bytes());
        out.extend((entries.len() as u16).to_le_bytes());
        out.extend((cd.len() as u32).to_le_bytes());
        out.extend(at.to_le_bytes());
        out.extend(0u16.to_le_bytes());
        out
    }

    #[test]
    fn zip_listing_is_checked() {
        let good = zip_dir(&[("QuadMech.x86_64", 0o100755, 70_000_000), ("cover.png", 0o100644, 60_000), ("art/", 0o040755, 0), ("art/hero.png", 0o100644, 9)], 100);
        let files = zip_entries(&good, 0, BINARY).unwrap();
        assert_eq!(files.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(), vec!["QuadMech.x86_64", "cover.png", "art/hero.png"]);
        assert_eq!(files[0].size, 70_000_000);
        // Only the end of the file read (the directory is in it).
        assert_eq!(zip_entries(&good[60..], 60, BINARY).unwrap().len(), 3);
        assert!(zip_entries(&good[110..], 110, BINARY).is_err());   // directory starts before what was read
        for bad in ["../evil", "/etc/passwd", "a/../../b", "./x", "a//b", "a\\b", "C:x", "évil", ""] {
            let z = zip_dir(&[("QuadMech.x86_64", 0o100755, 10), (bad, 0o100644, 1)], 0);
            assert!(zip_entries(&z, 0, BINARY).is_err(), "{bad}");
        }
        let link = zip_dir(&[("QuadMech.x86_64", 0o100755, 10), ("cover.png", 0o120777, 11)], 0);
        assert!(zip_entries(&link, 0, BINARY).unwrap_err().contains("link"));
        let fifo = zip_dir(&[("QuadMech.x86_64", 0o010644, 10)], 0);
        assert!(zip_entries(&fifo, 0, BINARY).is_err());
        let missing = zip_dir(&[("cover.png", 0o100644, 1)], 0);
        assert!(zip_entries(&missing, 0, BINARY).unwrap_err().contains("game"));
        let dup = zip_dir(&[("QuadMech.x86_64", 0o100755, 1), ("QuadMech.x86_64", 0o100755, 1)], 0);
        assert!(zip_entries(&dup, 0, BINARY).is_err());
        let huge = zip_dir(&[("QuadMech.x86_64", 0o100755, 0xF000_0000), ("b", 0o100644, 0xF000_0000)], 0);
        assert!(zip_entries(&huge, 0, BINARY).is_err());
        let many: Vec<String> = (0..70).map(|i| format!("f{i}")).collect();
        let mut list: Vec<(&str, u32, u32)> = many.iter().map(|n| (n.as_str(), 0o100644, 1)).collect();
        list.push(("QuadMech.x86_64", 0o100755, 1));
        assert!(zip_entries(&zip_dir(&list, 0), 0, BINARY).is_err());
        assert!(zip_entries(b"not a zip at all, just some text here", 0, BINARY).is_err());
        let mut enc = zip_dir(&[("QuadMech.x86_64", 0o100755, 10)], 0);
        enc[8] = 1;   // encrypted flag
        assert!(zip_entries(&enc, 0, BINARY).is_err());
    }

    #[test]
    fn unpacked_tree_is_checked() {
        let t = tempfile::tempdir().unwrap();
        let root = t.path();
        std::fs::write(root.join("QuadMech.x86_64"), b"elf").unwrap();
        std::fs::create_dir(root.join("art")).unwrap();
        std::fs::write(root.join("art/cover.png"), b"png").unwrap();
        let files = vec![ZipFile { name: "QuadMech.x86_64".into(), size: 3 }, ZipFile { name: "art/cover.png".into(), size: 3 }];
        assert!(check_tree(root, &files).is_ok());
        std::os::unix::fs::symlink("/etc/passwd", root.join("art/link")).unwrap();
        assert!(check_tree(root, &files).unwrap_err().contains("link"));
        std::fs::remove_file(root.join("art/link")).unwrap();
        std::fs::write(root.join("extra"), b"x").unwrap();
        assert!(check_tree(root, &files).is_err());
        std::fs::remove_file(root.join("extra")).unwrap();
        std::fs::remove_file(root.join("art/cover.png")).unwrap();
        assert!(check_tree(root, &files).is_err());   // one missing
    }

    #[test]
    fn arguments_per_mode() {
        assert_eq!(build_args(Role::Host, false, "Seat1"), vec!["--", "--host", "--mode=coop", "--name=Seat1", "--players=auto"]);
        assert_eq!(build_args(Role::Join, false, "Seat2"), vec!["--", "--join=auto", "--name=Seat2", "--players=auto"]);
        assert_eq!(build_args(Role::Local, false, "Seat1"), vec!["--", "--local", "--players=auto"]);
        assert_eq!(build_args(Role::Join, true, "Seat3"), vec!["--", "--join=auto", "--name=Seat3", "--players=auto", "--selftest=3600", "--quality=low"]);
        assert_eq!(build_args(Role::Host, false, "Seat1; rm -rf ~"), vec!["--", "--host", "--mode=coop", "--name=Seat1", "--players=auto"]);
        assert_eq!(launch_args(&json!({"role": "join", "name": "Seat2", "attract": true})).unwrap().last().unwrap(), "--quality=low");
        assert!(launch_args(&json!({"role": "exec", "name": "Seat2"})).is_err());
        assert!(launch_args(&json!({"role": "host", "name": "--fullscreen"})).is_err());
    }

    #[test]
    fn mode_choice() {
        // Hearth picks: controllers and a split screen possible → seats; no controllers → attract (seats only if set up).
        assert_eq!(pick_mode(None, 2, true, false), Choice { seats: true, attract: false });
        assert_eq!(pick_mode(None, 0, true, false), Choice { seats: false, attract: true });
        assert_eq!(pick_mode(None, 0, true, true), Choice { seats: true, attract: true });
        assert_eq!(pick_mode(None, 1, false, false), Choice { seats: false, attract: false });
        assert_eq!(pick_mode(Some("seats"), 0, true, false), Choice { seats: true, attract: false });
        assert_eq!(pick_mode(Some("seats"), 2, false, false), Choice { seats: false, attract: false });   // falls back to this screen
        assert_eq!(pick_mode(Some("local"), 2, true, true), Choice { seats: false, attract: false });
        assert_eq!(pick_mode(Some("attract"), 2, false, false), Choice { seats: false, attract: true });
    }

    #[test]
    fn requests_round_trip_and_are_strict() {
        let r = Request { id: "00112233445566778899aabbccddeeff".into(), created: 5, version: "20261020-1a2b3c4".into(), sha256: SHA.into(), size: 9, role: Role::Join, attract: false, name: "Seat2".into(), zip: true };
        let v = request_json(&r);
        assert_eq!(v["zip"], HANDOFF_ZIP);
        assert_eq!(parse_request(&v).unwrap(), r);
        for (k, bad) in [("id", json!("xyz")), ("version", json!("../x")), ("sha256", json!("00")), ("size", json!(0)), ("role", json!("exec")),
                         ("name", json!("Seat9")), ("zip", json!("../../etc/shadow")), ("zip", json!(true)), ("created", json!("now"))] {
            let mut b = v.clone();
            b[k] = bad;
            assert!(parse_request(&b).is_err(), "{k}");
        }
        let mut local = v.clone();
        local["zip"] = Value::Null;
        assert!(!parse_request(&local).unwrap().zip);
    }

    #[test]
    fn counts_controllers() {
        let t = "I: Bus=0003\nN: Name=\"Xbox Controller\"\nH: Handlers=event5 js0 \n\nI: Bus=0011\nN: Name=\"Keyboard\"\nH: Handlers=sysrq kbd event2 leds\n\nH: Handlers=js1 event9\n";
        assert_eq!(count_pads_in(t), 2);
        assert_eq!(count_pads_in(""), 0);
    }
}
