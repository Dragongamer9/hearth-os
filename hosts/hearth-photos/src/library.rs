//! The photo library (1.14, docs/mockups/checklists/photos-new.md): every picture and video in your Pictures and
//! Videos folders, Steam's screenshots and the folders you add, with when each was taken, its shape, what kind it is
//! (photo, video, screenshot, game capture, made by your AI), which game a capture is from and where it came from.
//!
//! Two files:
//! * the index (`<cache>/library.json`): what was found, so the next start is instant. It can always be made again
//!   from the files, so it's a cache, written whole.
//! * your own choices (`~/.local/share/nimbus/photos.json`: favorites, albums, added folders). Those are settings
//!   (red flag 9): read and changed only through hearth-core's settings_file (lock, merge, atomic, .bak), never
//!   rebuilt from the index, never written by opening the app.
//!
//! Nothing here leaves the PC.

use crate::formats::{ext, is_picture};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

pub const VIDEOS: &[&str] = &["mp4", "m4v", "mov", "mkv", "webm", "avi", "3gp"];
const INDEX_VERSION: u32 = 1;

pub fn is_video(p: &Path) -> bool { VIDEOS.contains(&ext(p).as_str()) }

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct Item {
    pub path: String,
    pub name: String,
    pub size: u64,
    /// Last changed (ms since 1970): with `size`, how the index knows a file is unchanged.
    pub mtime: u64,
    /// When it was taken (ms since 1970, local time as the camera wrote it), or when the file was made.
    pub taken: u64,
    /// As shown (turned pictures already swapped); 0 when unknown.
    pub w: u32,
    pub h: u32,
    /// photo | video | screenshot | capture | ai
    pub kind: String,
    /// Where it came from: phone | steam | gamemode | desktop | ai | pc
    pub src: String,
    /// For game captures: the game's name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub camera: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gps: Option<(f64, f64)>,
    /// Videos: seconds (when ffprobe is there).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secs: Option<f64>,
}

#[derive(Serialize, Deserialize, Default)]
struct Index { version: u32, items: Vec<Item> }

// ------------------------------------------------------------------ where things are

fn home() -> PathBuf { dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")) }

/// Your own choices (favorites, albums, added folders). HEARTH_PHOTOS_STORE for tests.
pub fn store_path() -> PathBuf {
    if let Some(p) = std::env::var_os("HEARTH_PHOTOS_STORE") { return PathBuf::from(p); }
    dirs::data_dir().unwrap_or_else(|| home().join(".local/share")).join("nimbus").join("photos.json")
}

/// HEARTH_PHOTOS_INDEX for tests (the cache folder is shared with other tests running at the same time).
fn index_path() -> PathBuf {
    std::env::var_os("HEARTH_PHOTOS_INDEX").map(PathBuf::from).unwrap_or_else(|| crate::imaging::cache_dir("library").join("index.json"))
}

/// Steam's folders: its own and each library's, for screenshots and game names.
fn steam_roots() -> Vec<PathBuf> {
    let h = home();
    let mut v: Vec<PathBuf> = [".local/share/Steam", ".steam/steam", ".var/app/com.valvesoftware.Steam/.local/share/Steam"]
        .iter().map(|r| h.join(r)).filter(|p| p.is_dir()).collect();
    v.sort(); v.dedup_by(|a, b| a.canonicalize().ok() == b.canonicalize().ok());
    v
}

/// The folders the library looks in: Pictures, Videos, Steam's screenshots, ComfyUI's pictures, and yours.
pub fn roots(store: &Value) -> Vec<(PathBuf, &'static str)> {
    let mut v: Vec<(PathBuf, &'static str)> = Vec::new();
    if let Some(p) = std::env::var_os("HEARTH_PHOTOS_ROOTS") {
        for r in std::env::split_paths(&p) { v.push((r, "pc")); }
        return v;
    }
    if let Some(p) = dirs::picture_dir() { v.push((p, "pc")); }
    if let Some(p) = dirs::video_dir() { v.push((p, "pc")); }
    for s in steam_roots() { v.push((s.join("userdata"), "steam")); }
    for c in ["ComfyUI/output", ".local/share/hearth/comfyui/output", "comfyui/output"] {
        let p = home().join(c);
        if p.is_dir() { v.push((p, "ai")); }
    }
    if let Some(list) = store.get("folders").and_then(Value::as_array) {
        for f in list.iter().filter_map(Value::as_str) { v.push((PathBuf::from(f), "pc")); }
    }
    v
}

// ------------------------------------------------------------------ what a file is

/// Steam's game names by app id, from each library's appmanifest_<id>.acf.
pub fn steam_names() -> HashMap<String, String> {
    let mut libs: Vec<PathBuf> = Vec::new();
    for s in steam_roots() {
        libs.push(s.join("steamapps"));
        if let Ok(t) = std::fs::read_to_string(s.join("steamapps/libraryfolders.vdf")) {
            for line in t.lines() {
                let l = line.trim();
                if let Some(rest) = l.strip_prefix("\"path\"") { libs.push(PathBuf::from(unquote(rest)).join("steamapps")); }
            }
        }
    }
    let mut m = HashMap::new();
    for lib in libs {
        for e in std::fs::read_dir(&lib).into_iter().flatten().flatten() {
            let n = e.file_name().to_string_lossy().into_owned();
            let Some(id) = n.strip_prefix("appmanifest_").and_then(|r| r.strip_suffix(".acf")) else { continue };
            if let Ok(t) = std::fs::read_to_string(e.path()) {
                if let Some(name) = t.lines().find_map(|l| l.trim().strip_prefix("\"name\"").map(unquote)) { m.insert(id.to_string(), name); }
            }
        }
    }
    m
}

fn unquote(s: &str) -> String { s.trim().trim_matches('"').replace("\\\\", "\\") }

/// Kind, source and game from where a file is. `root_src` is the root's own source.
pub fn classify(p: &Path, root_src: &str, names: &HashMap<String, String>) -> (String, String, Option<String>) {
    let s = p.to_string_lossy();
    let lower = s.to_lowercase();
    let video = is_video(p);
    // Steam: userdata/<user>/760/remote/<appid>/screenshots/x.jpg
    if let Some(i) = s.find("/760/remote/") {
        let id = s[i + 12..].split('/').next().unwrap_or("");
        let game = names.get(id).cloned().or_else(|| (!id.is_empty()).then(|| format!("Steam game {id}")));
        return ("capture".into(), "steam".into(), game);
    }
    if root_src == "ai" { return ("ai".into(), "ai".into(), None); }
    if lower.contains("/screenshots/hearth/") || lower.contains("/game captures/") || lower.contains("/gamescope") {
        return ("capture".into(), "gamemode".into(), None);
    }
    if lower.contains("/screenshots/") || p.file_name().map(|n| n.to_string_lossy().to_lowercase().starts_with("screenshot")).unwrap_or(false) {
        return ("screenshot".into(), "desktop".into(), None);
    }
    let phone = ["/iphone/", "/dcim/", "/camera roll/", "/from your phone/", "/icloud photos/", "/android/"].iter().any(|k| lower.contains(k));
    (if video { "video" } else { "photo" }.into(), if phone { "phone" } else { "pc" }.into(), None)
}

/// "2024:06:01 18:22:05" → ms since 1970 (as if UTC: the page shows it as written, without moving it by time zones).
pub fn exif_ms(t: &str) -> Option<u64> {
    let d: Vec<u64> = t.split(|c: char| !c.is_ascii_digit()).filter(|x| !x.is_empty()).filter_map(|x| x.parse().ok()).collect();
    if d.len() < 6 || d[0] < 1971 || d[1] == 0 || d[1] > 12 || d[2] == 0 || d[2] > 31 { return None; }
    let days = days_from_civil(d[0] as i64, d[1] as i64, d[2] as i64);
    if days < 0 { return None; }
    Some(((days as u64) * 86_400 + d[3] * 3600 + d[4] * 60 + d[5]) * 1000)
}

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The local time zone's offset in ms, so file times line up with camera times (which are local).
fn local_offset_ms() -> i64 {
    // TZ offsets from `date +%z`: cheap, and right for the PC's own zone.
    std::process::Command::new("date").arg("+%z").output().ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| {
            let s = s.trim();
            let sign = if s.starts_with('-') { -1 } else { 1 };
            let n: i64 = s.trim_start_matches(['+', '-']).parse().ok()?;
            Some(sign * ((n / 100) * 3600 + (n % 100) * 60) * 1000)
        })
        .unwrap_or(0)
}

/// File names phones and tools write: "IMG_20240601_182205.jpg", "Screenshot 2024-06-01 18-22-05.png",
/// "PXL_20240601_182205123.jpg", "20240601_182205.mp4".
pub fn name_ms(name: &str) -> Option<u64> {
    let digits: String = name.chars().map(|c| if c.is_ascii_digit() { c } else { ' ' }).collect();
    let parts: Vec<&str> = digits.split_whitespace().collect();
    // 8 digits then 6 digits (or more), or y m d h m s as separate groups.
    for w in parts.windows(2) {
        if w[0].len() == 8 && w[1].len() >= 6 {
            let (a, b) = (w[0], &w[1][..6]);
            return exif_ms(&format!("{}:{}:{} {}:{}:{}", &a[..4], &a[4..6], &a[6..8], &b[..2], &b[2..4], &b[4..6]));
        }
    }
    if let Some(i) = parts.iter().position(|p| p.len() == 4 && p.starts_with("20")) {
        if parts.len() >= i + 6 { return exif_ms(&parts[i..i + 6].join(":")); }
    }
    None
}

fn ms_of(t: std::io::Result<std::time::SystemTime>) -> u64 {
    t.ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_millis() as u64).unwrap_or(0)
}

fn probe_video(p: &Path) -> (u32, u32, Option<f64>) {
    if !crate::system::which("ffprobe") { return (0, 0, None); }
    let out = std::process::Command::new("ffprobe")
        .args(["-v", "error", "-select_streams", "v:0", "-show_entries", "stream=width,height:stream_tags=rotate:format=duration", "-of", "default=nw=1"])
        .arg(p).output();
    let Ok(o) = out else { return (0, 0, None) };
    let t = String::from_utf8_lossy(&o.stdout);
    let get = |k: &str| t.lines().find_map(|l| l.strip_prefix(&format!("{k}="))).map(str::trim);
    let (mut w, mut h) = (get("width").and_then(|v| v.parse().ok()).unwrap_or(0), get("height").and_then(|v| v.parse().ok()).unwrap_or(0));
    if matches!(get("TAG:rotate"), Some("90") | Some("270") | Some("-90")) { std::mem::swap(&mut w, &mut h); }
    (w, h, get("duration").and_then(|v| v.parse().ok()))
}

/// Everything the library keeps about one file (read fresh).
pub fn read_item(p: &Path, root_src: &str, names: &HashMap<String, String>, tz: i64) -> Option<Item> {
    let m = std::fs::metadata(p).ok()?;
    if !m.is_file() { return None; }
    let (kind, src, game) = classify(p, root_src, names);
    let name = p.file_name()?.to_string_lossy().into_owned();
    let mtime = ms_of(m.modified());
    let mut it = Item { path: p.to_string_lossy().into_owned(), name: name.clone(), size: m.len(), mtime, kind, src, game, ..Default::default() };
    let file_time = (mtime as i64 + tz).max(0) as u64;
    if is_video(p) {
        let (w, h, secs) = probe_video(p);
        it.w = w; it.h = h; it.secs = secs;
        it.taken = name_ms(&name).unwrap_or(file_time);
    } else {
        let info = crate::info::read(p).ok();
        if let Some(i) = &info {
            let (mut w, mut h) = (i.width.unwrap_or(0), i.height.unwrap_or(0));
            if (5..=8).contains(&i.orientation) { std::mem::swap(&mut w, &mut h); }
            it.w = w; it.h = h;
            it.gps = i.gps;
            it.camera = i.camera.iter().find(|(k, _)| k == "Camera").map(|(_, v)| v.clone());
        }
        it.taken = info.as_ref().and_then(|i| i.taken.as_deref()).and_then(exif_ms).or_else(|| name_ms(&name)).unwrap_or(file_time);
    }
    Some(it)
}

fn wanted(p: &Path) -> bool {
    (is_picture(p) || is_video(p)) && !p.file_name().map(|n| { let n = n.to_string_lossy(); n.starts_with('.') || n.ends_with(".hearth-photos-tmp") }).unwrap_or(true)
}

/// Walks a root (hidden folders, Steam's thumbnails and cache folders skipped), at most 12 levels down.
fn walk(dir: &Path, depth: u32, out: &mut Vec<PathBuf>, seen: &mut HashSet<PathBuf>) {
    if depth > 12 { return; }
    let Ok(real) = dir.canonicalize() else { return };
    if !seen.insert(real) { return; }
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        let n = e.file_name().to_string_lossy().into_owned();
        if n.starts_with('.') { continue; }
        let Ok(ft) = e.file_type() else { continue };
        let is_dir = ft.is_dir() || (ft.is_symlink() && p.is_dir());
        if is_dir {
            if n == "thumbnails" || n == "node_modules" || n == "__pycache__" { continue; }
            walk(&p, depth + 1, out, seen);
        } else if wanted(&p) { out.push(p); }
    }
}

// ------------------------------------------------------------------ the index

fn read_index() -> Vec<Item> {
    std::fs::read(index_path()).ok()
        .and_then(|b| serde_json::from_slice::<Index>(&b).ok())
        .filter(|i| i.version == INDEX_VERSION)
        .map(|i| i.items).unwrap_or_default()
}

fn write_index(items: &[Item]) {
    let p = index_path();
    if let Ok(b) = serde_json::to_vec(&Index { version: INDEX_VERSION, items: items.to_vec() }) { let _ = crate::write_atomic(&p, &b); }
}

/// What the index had last time (instant; may be out of date).
pub fn cached() -> Vec<Item> { let mut v = read_index(); sort(&mut v); v }

fn sort(v: &mut [Item]) { v.sort_by(|a, b| b.taken.cmp(&a.taken).then_with(|| a.name.cmp(&b.name))); }

/// Looks through every root again: unchanged files come from the index, new and changed ones are read. Newest first.
pub fn scan() -> Vec<Item> {
    let store = store_read();
    let old: HashMap<String, Item> = read_index().into_iter().map(|i| (i.path.clone(), i)).collect();
    let names = steam_names();
    let tz = local_offset_ms();
    let mut seen = HashSet::new();
    let mut items = Vec::new();
    let mut paths_done = HashSet::new();
    for (root, src) in roots(&store) {
        let mut files = Vec::new();
        walk(&root, 0, &mut files, &mut seen);
        for f in files {
            let key = f.to_string_lossy().into_owned();
            if !paths_done.insert(key.clone()) { continue; }
            if let Some(o) = old.get(&key) {
                if let Ok(m) = std::fs::metadata(&f) {
                    if m.len() == o.size && ms_of(m.modified()) == o.mtime {
                        let mut o = o.clone();
                        // A game's name that has turned up since (installed later).
                        if o.kind == "capture" && o.src == "steam" { o.game = classify(&f, src, &names).2; }
                        items.push(o);
                        continue;
                    }
                }
            }
            if let Some(it) = read_item(&f, src, &names, tz) { items.push(it); }
        }
    }
    sort(&mut items);
    write_index(&items);
    items
}

// ------------------------------------------------------------------ your choices (settings: red flag 9)

#[cfg(target_os = "linux")]
pub fn store_read() -> Value { hearth_core::settings_file::read_json(&store_path()) }
#[cfg(not(target_os = "linux"))]
pub fn store_read() -> Value {
    std::fs::read(store_path()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_else(|| json!({}))
}

#[cfg(target_os = "linux")]
fn store_update(change: impl FnOnce(&mut Map<String, Value>) -> Result<(), String>) -> Result<Value, String> {
    let (m, r) = hearth_core::settings_file::update_json(&store_path(), change).map_err(|e| format!("Couldn't save that: {e}"))?;
    r.map(|_| Value::Object(m))
}
#[cfg(not(target_os = "linux"))]
fn store_update(change: impl FnOnce(&mut Map<String, Value>) -> Result<(), String>) -> Result<Value, String> {
    let mut m = store_read().as_object().cloned().unwrap_or_default();
    change(&mut m)?;
    let v = Value::Object(m);
    crate::write_atomic(&store_path(), format!("{}\n", serde_json::to_string_pretty(&v).unwrap_or_default()).as_bytes())?;
    Ok(v)
}

fn arr<'a>(m: &'a mut Map<String, Value>, k: &str) -> &'a mut Vec<Value> {
    if !m.get(k).map(Value::is_array).unwrap_or(false) { m.insert(k.into(), json!([])); }
    m.get_mut(k).and_then(Value::as_array_mut).expect("just made")
}

fn new_id(prefix: &str) -> String {
    let t = std::time::SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
    format!("{prefix}{t:x}")
}

/// One change you made: {op: "favorite", paths, on} | {op: "album.new", name, paths} | {op: "album.add", id, paths}
/// | {op: "album.remove", id, paths} | {op: "album.rename", id, name} | {op: "album.delete", id}
/// | {op: "folder.add", path} | {op: "folder.remove", path}. Returns the whole store after it.
pub fn change(c: &Value) -> Result<Value, String> {
    let op = c["op"].as_str().unwrap_or("");
    let paths: Vec<String> = c["paths"].as_array().map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default();
    let id = c["id"].as_str().unwrap_or("").to_string();
    let name = c["name"].as_str().unwrap_or("").trim().to_string();
    let path = c["path"].as_str().unwrap_or("").to_string();
    let on = c["on"].as_bool().unwrap_or(true);
    match op {
        "favorite" | "album.new" | "album.add" | "album.remove" | "album.rename" | "album.delete" | "folder.add" | "folder.remove" => {}
        _ => return Err(format!("Photos doesn't know how to \"{op}\"")),
    }
    store_update(move |m| {
        match op {
            "favorite" => {
                let f = arr(m, "favorites");
                for p in &paths {
                    let has = f.iter().any(|v| v.as_str() == Some(p));
                    if on && !has { f.push(json!(p)); }
                    if !on { f.retain(|v| v.as_str() != Some(p)); }
                }
            }
            "album.new" => {
                if name.is_empty() { return Err("An album needs a name".into()); }
                let a = arr(m, "albums");
                a.push(json!({"id": new_id("a"), "name": name, "items": paths, "made": std::time::SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)}));
            }
            "album.add" | "album.remove" | "album.rename" | "album.delete" => {
                let a = arr(m, "albums");
                let Some(i) = a.iter().position(|x| x["id"].as_str() == Some(&id)) else { return Err("That album isn't there any more".into()) };
                if op == "album.delete" { a.remove(i); return Ok(()); }
                let al = a[i].as_object_mut().ok_or("That album is damaged")?;
                if op == "album.rename" { if name.is_empty() { return Err("An album needs a name".into()); } al.insert("name".into(), json!(name)); return Ok(()); }
                let items = arr(al, "items");
                for p in &paths {
                    let has = items.iter().any(|v| v.as_str() == Some(p));
                    if op == "album.add" && !has { items.push(json!(p)); }
                    if op == "album.remove" { items.retain(|v| v.as_str() != Some(p)); }
                }
            }
            "folder.add" => {
                if !Path::new(&path).is_dir() { return Err("That folder isn't there".into()); }
                let f = arr(m, "folders");
                if !f.iter().any(|v| v.as_str() == Some(&path)) { f.push(json!(path)); }
            }
            "folder.remove" => { arr(m, "folders").retain(|v| v.as_str() != Some(&path)); }
            _ => {}
        }
        Ok(())
    })
}

/// How many pictures and videos are in the Trash (the Trash list's count).
pub fn trash_count() -> usize {
    let d = dirs::data_dir().unwrap_or_else(|| home().join(".local/share")).join("Trash/files");
    std::fs::read_dir(d).into_iter().flatten().flatten().filter(|e| wanted(&e.path())).count()
}

/// The Trash's pictures and videos (the Trash list), newest first.
pub fn trash_items() -> Vec<Item> {
    let d = dirs::data_dir().unwrap_or_else(|| home().join(".local/share")).join("Trash/files");
    let names = HashMap::new();
    let mut v: Vec<Item> = std::fs::read_dir(d).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| wanted(p))
        .filter_map(|p| { let mut i = read_item(&p, "pc", &names, 0)?; i.taken = i.mtime; Some(i) }).collect();
    sort(&mut v);
    v
}

/// The phone box: the newest file that came in from a phone (when), and the library's size.
pub fn summary(items: &[Item]) -> Value {
    let phone_last = items.iter().filter(|i| i.src == "phone").map(|i| i.mtime).max();
    let bytes: u64 = items.iter().map(|i| i.size).sum();
    json!({"count": items.len(), "bytes": bytes, "phoneLast": phone_last, "trash": trash_count(),
           "phoneName": std::env::var("HEARTH_PHONE_NAME").ok().or_else(phone_name)})
}

/// The phone's name, from the iCloud mover's or Hearth Link's settings when one of them knows it.
fn phone_name() -> Option<String> {
    for f in [".config/hearth/icloud.json", ".config/nimbus/link.json", ".local/share/nimbus/photos.json"] {
        let Some(v) = std::fs::read(home().join(f)).ok().and_then(|b| serde_json::from_slice::<Value>(&b).ok()) else { continue };
        if let Some(n) = v.get("phoneName").or_else(|| v.get("deviceName")).and_then(Value::as_str) { return Some(n.to_string()); }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_dates() {
        assert_eq!(exif_ms("1970:01:02 00:00:00"), None);
        assert_eq!(exif_ms("2024:06:01 18:22:05"), Some(1_717_266_125_000));
        assert_eq!(name_ms("IMG_20240601_182205.jpg"), Some(1_717_266_125_000));
        assert_eq!(name_ms("PXL_20240601_182205123.jpg"), Some(1_717_266_125_000));
        assert_eq!(name_ms("Screenshot 2024-06-01 18-22-05.png"), Some(1_717_266_125_000));
        assert_eq!(name_ms("holiday.jpg"), None);
    }

    #[test]
    fn knows_kinds() {
        let mut n = HashMap::new();
        n.insert("1245620".to_string(), "ELDEN RING".to_string());
        let c = |p: &str, s: &str| classify(Path::new(p), s, &n);
        assert_eq!(c("/h/.local/share/Steam/userdata/9/760/remote/1245620/screenshots/a.jpg", "steam"), ("capture".into(), "steam".into(), Some("ELDEN RING".into())));
        assert_eq!(c("/h/Pictures/Screenshots/Hearth/Screenshot 1.png", "pc").0, "capture");
        assert_eq!(c("/h/Pictures/Screenshots/Screenshot 1.png", "pc"), ("screenshot".into(), "desktop".into(), None));
        assert_eq!(c("/h/Pictures/iPhone/2026/11/IMG_1.HEIC", "pc"), ("photo".into(), "phone".into(), None));
        assert_eq!(c("/h/Videos/clip.mp4", "pc"), ("video".into(), "pc".into(), None));
        assert_eq!(c("/h/ComfyUI/output/x.png", "ai").0, "ai");
    }

    #[test]
    fn scans_and_keeps_choices() {
        let d = crate::imaging::tests::sample_dir("library");
        let pics = d.join("pics");
        std::fs::create_dir_all(pics.join("iPhone/2026")).unwrap();
        std::fs::create_dir_all(pics.join(".hidden")).unwrap();
        crate::imaging::tests::gradient(300, 200).save(pics.join("iPhone/2026/IMG_20260101_101010.png")).unwrap();
        crate::imaging::tests::gradient(100, 200).save(pics.join("tall.png")).unwrap();
        crate::imaging::tests::gradient(10, 10).save(pics.join(".hidden/no.png")).unwrap();
        std::env::set_var("HEARTH_PHOTOS_ROOTS", &pics);
        std::env::set_var("HEARTH_PHOTOS_STORE", d.join("photos.json"));
        std::env::set_var("HEARTH_PHOTOS_INDEX", d.join("index.json"));
        let items = scan();
        assert_eq!(items.len(), 2, "{items:?}");
        let phone = items.iter().find(|i| i.src == "phone").unwrap();
        assert_eq!((phone.w, phone.h), (300, 200));
        assert_eq!(phone.taken, exif_ms("2026:01:01 10:10:10").unwrap());
        // The second scan comes from the index (same answer).
        assert_eq!(scan(), items);
        assert_eq!(cached(), items);
        // Choices: merged, never rebuilt.
        change(&json!({"op": "favorite", "paths": [phone.path], "on": true})).unwrap();
        let s = change(&json!({"op": "album.new", "name": "Lake trip", "paths": [phone.path]})).unwrap();
        let id = s["albums"][0]["id"].as_str().unwrap().to_string();
        let other = items.iter().find(|i| i.src != "phone").unwrap();
        change(&json!({"op": "album.add", "id": id, "paths": [other.path]})).unwrap();
        let s = change(&json!({"op": "favorite", "paths": [phone.path], "on": true})).unwrap();
        assert_eq!(s["favorites"].as_array().unwrap().len(), 1);
        assert_eq!(s["albums"][0]["items"].as_array().unwrap().len(), 2);
        assert!(change(&json!({"op": "album.new", "name": " "})).is_err());
        assert!(change(&json!({"op": "nope"})).is_err());
        let s = change(&json!({"op": "album.delete", "id": id})).unwrap();
        assert!(s["albums"].as_array().unwrap().is_empty());
        assert_eq!(s["favorites"].as_array().unwrap().len(), 1);
        std::env::remove_var("HEARTH_PHOTOS_ROOTS");
    }
}
