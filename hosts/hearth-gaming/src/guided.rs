//! Files the Wabbajack engine can't download by itself (LoversLab, Vector Plexus, "manual" sites, anything that
//! failed): Hearth opens the file's page in Hearth Browser, where you're signed in with your own browser session, you
//! press Download, and Hearth catches the file in the browser's download folder, checks it is the file the modlist
//! wants (name, size, and Wabbajack's xxHash64 when the list gives one), and moves it into the list's downloads.
//!
//! Hearth Browser (Firefox) saves to the Downloads folder without asking, unless you set another folder in its
//! settings: both are watched (browser_download_dirs). Firefox writes `name.part` (and an empty `name`) while it
//! downloads and renames at the end, so a file with a `.part` next to it isn't finished yet.

use crate::util;
use std::collections::HashSet;
use std::io::Read;
use std::path::{Path, PathBuf};

// ------------------------------------------------------------------ xxHash64 (Wabbajack's file hash)

const P1: u64 = 11400714785074694791;
const P2: u64 = 14029467366897019727;
const P3: u64 = 1609587929392839161;
const P4: u64 = 9650029242287828579;
const P5: u64 = 2870177450012600261;

fn round(acc: u64, input: u64) -> u64 { acc.wrapping_add(input.wrapping_mul(P2)).rotate_left(31).wrapping_mul(P1) }
fn merge(acc: u64, v: u64) -> u64 { (acc ^ round(0, v)).wrapping_mul(P1).wrapping_add(P4) }

/// Streaming xxHash64 with seed 0, as Wabbajack hashes archives.
pub struct Xxh64 { v: [u64; 4], buf: [u8; 32], used: usize, total: u64 }
impl Default for Xxh64 { fn default() -> Self { Self::new() } }
impl Xxh64 {
    pub fn new() -> Self { Xxh64 { v: [P1.wrapping_add(P2), P2, 0, 0u64.wrapping_sub(P1)], buf: [0; 32], used: 0, total: 0 } }
    fn stripe(&mut self, s: &[u8]) {
        for i in 0..4 { self.v[i] = round(self.v[i], u64::from_le_bytes(s[i * 8..i * 8 + 8].try_into().unwrap())); }
    }
    pub fn update(&mut self, mut data: &[u8]) {
        self.total += data.len() as u64;
        if self.used > 0 {
            let take = (32 - self.used).min(data.len());
            self.buf[self.used..self.used + take].copy_from_slice(&data[..take]);
            self.used += take;
            data = &data[take..];
            if self.used < 32 { return; }
            let b = self.buf;
            self.stripe(&b);
            self.used = 0;
        }
        while data.len() >= 32 { self.stripe(&data[..32]); data = &data[32..]; }
        self.buf[..data.len()].copy_from_slice(data);
        self.used = data.len();
    }
    pub fn digest(&self) -> u64 {
        let mut h = if self.total >= 32 {
            let v = self.v;
            let mut h = v[0].rotate_left(1).wrapping_add(v[1].rotate_left(7)).wrapping_add(v[2].rotate_left(12)).wrapping_add(v[3].rotate_left(18));
            for x in v { h = merge(h, x); }
            h
        } else { P5 };
        h = h.wrapping_add(self.total);
        let mut rest = &self.buf[..self.used];
        while rest.len() >= 8 {
            h ^= round(0, u64::from_le_bytes(rest[..8].try_into().unwrap()));
            h = h.rotate_left(27).wrapping_mul(P1).wrapping_add(P4);
            rest = &rest[8..];
        }
        if rest.len() >= 4 {
            h ^= (u32::from_le_bytes(rest[..4].try_into().unwrap()) as u64).wrapping_mul(P1);
            h = h.rotate_left(23).wrapping_mul(P2).wrapping_add(P3);
            rest = &rest[4..];
        }
        for &b in rest { h ^= (b as u64).wrapping_mul(P5); h = h.rotate_left(11).wrapping_mul(P1); }
        h ^= h >> 33; h = h.wrapping_mul(P2); h ^= h >> 29; h = h.wrapping_mul(P3); h ^= h >> 32;
        h
    }
}
pub fn xxh64(data: &[u8]) -> u64 { let mut x = Xxh64::new(); x.update(data); x.digest() }

pub fn hash_file(p: &Path) -> std::io::Result<u64> {
    let mut f = std::fs::File::open(p)?;
    let mut x = Xxh64::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 { break; }
        x.update(&buf[..n]);
    }
    Ok(x.digest())
}

/// A hash as Wabbajack writes it: the engine's events give 16 hex digits of the little-endian bytes
/// (Hash.ToHex), a .wabbajack file gives the same 8 bytes in base64. Anything else: unknown.
pub fn parse_hash(s: &str) -> Option<u64> {
    let s = s.trim();
    if s.len() == 16 && s.chars().all(|c| c.is_ascii_hexdigit()) {
        let mut b = [0u8; 8];
        for i in 0..8 { b[i] = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok()?; }
        return Some(u64::from_le_bytes(b));
    }
    use base64::Engine;
    let b = base64::engine::general_purpose::STANDARD.decode(s).ok()?;
    let b: [u8; 8] = b.try_into().ok()?;
    Some(u64::from_le_bytes(b))
}
/// How Hearth keeps a hash (in a job's file and on the page): the number in hex.
pub fn hash_text(h: u64) -> String { format!("{h:016x}") }
pub fn hash_of_text(s: &str) -> Option<u64> { if s.len() == 16 { u64::from_str_radix(s, 16).ok() } else { None } }

// ------------------------------------------------------------------ is it the right file?

/// What the modlist expects of a file.
#[derive(Debug, Clone, Default)]
pub struct Want { pub name: String, pub size: u64, pub hash: Option<u64> }

#[derive(Debug, Clone, PartialEq)]
pub enum Check { Good, Missing, WrongSize(u64), WrongHash }

/// Checks a file against what the list wants: the size first (cheap), then the hash when the list gives one.
pub fn check(p: &Path, w: &Want) -> Check {
    let Ok(md) = std::fs::metadata(p) else { return Check::Missing };
    if !md.is_file() { return Check::Missing; }
    if w.size != 0 && md.len() != w.size { return Check::WrongSize(md.len()); }
    match w.hash {
        Some(h) => match hash_file(p) { Ok(x) if x == h => Check::Good, Ok(_) => Check::WrongHash, Err(_) => Check::Missing },
        None => Check::Good,
    }
}

/// Browsers add " (1)" (or "(1)") before the extension when a name is taken.
pub fn same_name(found: &str, want: &str) -> bool {
    if found == want { return true; }
    let (stem, ext) = match want.rsplit_once('.') { Some((s, e)) => (s, format!(".{e}")), None => (want, String::new()) };
    let Some(mid) = found.strip_prefix(stem).and_then(|r| r.strip_suffix(ext.as_str())) else { return false };
    let mid = mid.trim_start();
    mid.len() >= 3 && mid.starts_with('(') && mid.ends_with(')') && mid[1..mid.len() - 1].chars().all(|c| c.is_ascii_digit())
}

fn unfinished(p: &Path) -> bool {
    let n = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    if [".part", ".crdownload", ".tmp", ".hearth-part", ".download"].iter().any(|x| n.ends_with(x)) { return true; }
    p.with_file_name(format!("{n}.part")).exists()
}

/// Files in `dirs` that could be the one: same name (first) or, when the list gives a hash, any finished file of the
/// right size (sites sometimes serve a file under another name).
pub fn candidates(dirs: &[PathBuf], w: &Want) -> Vec<(PathBuf, bool)> {
    let (mut named, mut sized) = (Vec::new(), Vec::new());
    for d in dirs {
        let Ok(rd) = std::fs::read_dir(d) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            let Ok(md) = e.metadata() else { continue };
            if !md.is_file() || unfinished(&p) || md.len() == 0 { continue; }
            let n = e.file_name().to_string_lossy().to_string();
            let size_ok = w.size == 0 || md.len() == w.size;
            if same_name(&n, &w.name) { named.push((p, true)); } else if size_ok && w.hash.is_some() && w.size != 0 { sized.push((p, false)); }
        }
    }
    named.extend(sized);
    named
}

#[derive(Debug, Clone, PartialEq)]
pub enum PickUp { Moved(PathBuf), Wrong(String), Nothing }

/// Looks for the file in `dirs`, checks it and moves it to `dest/<name>`. `seen` remembers files already found wrong
/// (by path, size and time), so a big wrong file isn't hashed again every look.
pub fn pick_up(dirs: &[PathBuf], dest: &Path, w: &Want, seen: &mut HashSet<(PathBuf, u64, u64)>) -> PickUp {
    if w.name.is_empty() { return PickUp::Nothing; }
    let mut wrong = PickUp::Nothing;
    for (p, by_name) in candidates(dirs, w) {
        let Ok(md) = std::fs::metadata(&p) else { continue };
        let key = (p.clone(), md.len(), util::mtime_ms(&md));
        if seen.contains(&key) { continue; }
        match check(&p, w) {
            Check::Good => {
                let to = dest.join(&w.name);
                if let Some(d) = to.parent() { let _ = std::fs::create_dir_all(d); }
                return match crate::mods::move_file(&p, &to) { Ok(_) => PickUp::Moved(to), Err(e) => PickUp::Wrong(format!("Couldn't move it into the modlist's downloads ({e})")) };
            }
            Check::WrongSize(n) if by_name => { seen.insert(key); wrong = PickUp::Wrong(format!("{} is {} bytes, the modlist wants {} bytes: probably another version of the file.", p.file_name().unwrap_or_default().to_string_lossy(), n, w.size)); }
            Check::WrongHash if by_name => { seen.insert(key); wrong = PickUp::Wrong(format!("{} isn't the exact file the modlist wants (its hash differs): probably another version.", p.file_name().unwrap_or_default().to_string_lossy())); }
            _ => { seen.insert(key); }
        }
    }
    wrong
}

// ------------------------------------------------------------------ where the browser saves

/// Your Downloads folder (XDG), and the folder Hearth Browser saves to when you chose another one in its settings.
pub fn browser_download_dirs() -> Vec<PathBuf> {
    let mut out = vec![user_downloads()];
    let prefs = util::data_dir().join("hearth-browser").join("prefs.js");
    if let Some(d) = firefox_download_dir(&std::fs::read_to_string(prefs).unwrap_or_default()) { if !out.contains(&d) { out.push(d); } }
    out
}

/// browser.download.dir from a Firefox prefs.js, when folderList is 2 (a folder of your choice).
pub fn firefox_download_dir(prefs: &str) -> Option<PathBuf> {
    let get = |k: &str| prefs.lines().find_map(|l| l.trim().strip_prefix(&format!("user_pref(\"{k}\", ")).and_then(|r| r.strip_suffix(");")).map(|v| v.trim().to_string()));
    if get("browser.download.folderList").as_deref() != Some("2") { return None; }
    let v = get("browser.download.dir")?;
    let v = v.trim_matches('"').replace("\\\\", "\\").replace("\\\"", "\"");
    if v.starts_with('/') { Some(PathBuf::from(v)) } else { None }
}

pub fn user_downloads() -> PathBuf {
    let dirs = std::fs::read_to_string(util::xdg_config().join("user-dirs.dirs")).unwrap_or_default();
    for l in dirs.lines() {
        if let Some(v) = l.strip_prefix("XDG_DOWNLOAD_DIR=") {
            let v = v.trim().trim_matches('"').replace("$HOME", &util::home().to_string_lossy());
            if !v.is_empty() { return PathBuf::from(v); }
        }
    }
    util::home().join("Downloads")
}

/// Opens a page in Hearth Browser (where you're signed in to the site), or the default browser without it.
pub fn open_in_browser(url: &str) -> Result<(), String> {
    if !url.starts_with("https://") { return Err("Only https pages open from here.".into()); }
    let hb = Path::new("/usr/bin/hearth-browser");
    let mut c = std::process::Command::new(if hb.is_file() { "/usr/bin/hearth-browser" } else { "xdg-open" });
    c.arg(url).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
    c.spawn().map(|mut ch| { std::thread::spawn(move || { let _ = ch.wait(); }); }).map_err(|e| format!("Couldn't open the page ({e})"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xxhash64_vectors() {
        assert_eq!(xxh64(b""), 0xEF46DB3751D8E999);
        assert_eq!(xxh64(b"abc"), 0x44BC2CF5AD770999);
        // Streaming in odd pieces gives the same as all at once (over and under the 32-byte stripes).
        let data: Vec<u8> = (0..1000u32).map(|i| (i * 7 % 251) as u8).collect();
        let mut x = Xxh64::new();
        for c in data.chunks(13) { x.update(c); }
        assert_eq!(x.digest(), xxh64(&data));
        let t = tempfile::tempdir().unwrap();
        std::fs::write(t.path().join("f"), &data).unwrap();
        assert_eq!(hash_file(&t.path().join("f")).unwrap(), xxh64(&data));
    }

    #[test]
    fn wabbajack_hash_forms() {
        let h = xxh64(b"abc");
        let le = h.to_le_bytes();
        let hex: String = le.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(parse_hash(&hex), Some(h));
        use base64::Engine;
        assert_eq!(parse_hash(&base64::engine::general_purpose::STANDARD.encode(le)), Some(h));
        assert_eq!(parse_hash("nope"), None);
        assert_eq!(hash_of_text(&hash_text(h)), Some(h));
    }

    #[test]
    fn names_browsers_give() {
        assert!(same_name("Mod 1.2.7z", "Mod 1.2.7z"));
        assert!(same_name("Mod 1.2 (1).7z", "Mod 1.2.7z"));
        assert!(same_name("Mod 1.2(2).7z", "Mod 1.2.7z"));
        assert!(!same_name("Mod 1.3.7z", "Mod 1.2.7z"));
        assert!(!same_name("Mod 1.2 (x).7z", "Mod 1.2.7z"));
    }

    #[test]
    fn a_downloaded_file_is_matched_and_moved() {
        let t = tempfile::tempdir().unwrap();
        let (dl, dest) = (t.path().join("Downloads"), t.path().join("list"));
        std::fs::create_dir_all(&dl).unwrap();
        let w = Want { name: "Body.7z".into(), size: 5, hash: Some(xxh64(b"hello")) };
        let mut seen = HashSet::new();
        assert_eq!(pick_up(&[dl.clone()], &dest, &w, &mut seen), PickUp::Nothing);
        // Still downloading: Firefox's empty file plus its .part.
        std::fs::write(dl.join("Body.7z"), "").unwrap();
        std::fs::write(dl.join("Body.7z.part"), "hel").unwrap();
        assert_eq!(pick_up(&[dl.clone()], &dest, &w, &mut seen), PickUp::Nothing);
        // Another version of it: right name, wrong bytes.
        std::fs::remove_file(dl.join("Body.7z.part")).unwrap();
        std::fs::write(dl.join("Body.7z"), "jello").unwrap();
        assert!(matches!(pick_up(&[dl.clone()], &dest, &w, &mut seen), PickUp::Wrong(_)));
        assert!(dl.join("Body.7z").exists(), "a wrong file stays where it is");
        // The right file under another name (same size and hash) is taken and renamed.
        std::fs::write(dl.join("body_v2.7z"), "hello").unwrap();
        assert_eq!(pick_up(&[dl.clone()], &dest, &w, &mut seen), PickUp::Moved(dest.join("Body.7z")));
        assert_eq!(std::fs::read(dest.join("Body.7z")).unwrap(), b"hello");
        assert!(!dl.join("body_v2.7z").exists());
        assert_eq!(check(&dest.join("Body.7z"), &w), Check::Good);
        assert_eq!(check(&dest.join("Body.7z"), &Want { size: 6, ..w.clone() }), Check::WrongSize(5));
    }

    #[test]
    fn hearth_browsers_own_folder() {
        let p = "user_pref(\"browser.download.dir\", \"/home/a/Mods in\");\nuser_pref(\"browser.download.folderList\", 2);\n";
        assert_eq!(firefox_download_dir(p), Some(PathBuf::from("/home/a/Mods in")));
        assert_eq!(firefox_download_dir("user_pref(\"browser.download.dir\", \"/x\");"), None);
    }
}
