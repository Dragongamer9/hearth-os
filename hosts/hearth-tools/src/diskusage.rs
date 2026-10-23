//! Disk Usage: what's filling a folder or a drive. A scan walks the folder on every processor core (rayon), adds up
//! the space each file really takes on the drive (like `du`), counts hard-linked files once, and stops at other
//! drives and mounts (they're shown, not added in). The finished tree stays here; the page asks for the part it
//! shows (a ring chart a few levels deep, or one folder's list). Moving to the Trash goes through Hearth Files
//! (hearth-files trash), and the tree is corrected without scanning again.

use crate::util;
use rayon::prelude::*;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Files at least this big are kept by name in their folder; smaller ones are only counted ("small files").
const KEEP_FILE: u64 = 256 * 1024;
/// At most this many named files per folder (the biggest).
const KEEP_PER_DIR: usize = 300;

#[derive(Debug, Clone, Default)]
pub struct FileEntry {
    pub name: String,
    pub size: u64,
    pub mtime: i64,
}

#[derive(Debug, Clone, Default)]
pub struct Node {
    pub name: String,
    pub size: u64,
    pub files: u64,
    pub dirs: Vec<Node>,
    pub big: Vec<FileEntry>,
    /// Space taken by the files not kept by name, and how many.
    pub small: u64,
    pub small_count: u64,
    /// Another drive or mount: shown, not scanned.
    pub mount: bool,
    /// Couldn't be read (no permission).
    pub denied: bool,
}

struct Ctx<'a> {
    cancel: &'a AtomicBool,
    mounts: HashSet<PathBuf>,
    root: PathBuf,
    links: Mutex<HashSet<(u64, u64)>>,
    files: AtomicU64,
    bytes: AtomicU64,
    current: Mutex<String>,
}

#[cfg(unix)]
fn on_disk(m: &std::fs::Metadata) -> u64 {
    use std::os::unix::fs::MetadataExt;
    // Allocated blocks: what the file really takes (sparse files and compression included). Fall back to the length.
    let b = m.blocks().saturating_mul(512);
    if b == 0 && m.len() > 0 && m.len() < 4096 { m.len() } else { b }
}
#[cfg(not(unix))]
fn on_disk(m: &std::fs::Metadata) -> u64 { m.len() }

#[cfg(unix)]
fn link_key(m: &std::fs::Metadata) -> Option<(u64, u64)> {
    use std::os::unix::fs::MetadataExt;
    if m.nlink() > 1 { Some((m.dev(), m.ino())) } else { None }
}
#[cfg(not(unix))]
fn link_key(_: &std::fs::Metadata) -> Option<(u64, u64)> { None }

fn mtime(m: &std::fs::Metadata) -> i64 {
    m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs() as i64).unwrap_or(0)
}

fn scan_dir(path: &Path, name: String, ctx: &Ctx) -> Node {
    let mut node = Node { name, ..Default::default() };
    if ctx.cancel.load(Ordering::Relaxed) { return node; }
    if path != ctx.root && ctx.mounts.contains(path) { node.mount = true; return node; }
    if let Ok(mut c) = ctx.current.try_lock() { *c = path.to_string_lossy().into_owned(); }
    let rd = match std::fs::read_dir(path) { Ok(r) => r, Err(_) => { node.denied = true; return node; } };
    let mut subdirs: Vec<(PathBuf, String)> = Vec::new();
    let mut big: Vec<FileEntry> = Vec::new();
    for e in rd.flatten() {
        let Ok(m) = e.metadata() else { continue };   // DirEntry::metadata doesn't follow symlinks
        let fname = e.file_name().to_string_lossy().into_owned();
        if m.is_dir() { subdirs.push((e.path(), fname)); continue; }
        let mut size = on_disk(&m);
        if let Some(k) = link_key(&m) {
            if let Ok(mut l) = ctx.links.lock() { if !l.insert(k) { size = 0; } }
        }
        node.files += 1;
        node.size += size;
        ctx.files.fetch_add(1, Ordering::Relaxed);
        ctx.bytes.fetch_add(size, Ordering::Relaxed);
        if size >= KEEP_FILE && m.is_file() { big.push(FileEntry { name: fname, size, mtime: mtime(&m) }); } else { node.small += size; node.small_count += 1; }
    }
    big.sort_by(|a, b| b.size.cmp(&a.size));
    for f in big.drain(KEEP_PER_DIR.min(big.len())..) { node.small += f.size; node.small_count += 1; }
    node.big = big;
    node.dirs = subdirs.into_par_iter().map(|(p, n)| scan_dir(&p, n, ctx)).collect();
    for d in &node.dirs { node.size += d.size; node.files += d.files; }
    node.dirs.sort_by(|a, b| b.size.cmp(&a.size));
    node
}

/// A mount from /proc/self/mountinfo.
#[derive(Debug, Clone, PartialEq)]
pub struct Mount {
    pub point: String,
    pub fstype: String,
    pub source: String,
}

fn unescape(s: &str) -> String {
    // mountinfo writes spaces, tabs, newlines and backslashes as \040 \011 \012 \134.
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'\\' && i + 3 < b.len() && b[i + 1..i + 4].iter().all(|c| (b'0'..=b'7').contains(c)) {
            out.push(((b[i + 1] - b'0') << 6) | ((b[i + 2] - b'0') << 3) | (b[i + 3] - b'0'));
            i += 4;
        } else { out.push(b[i]); i += 1; }
    }
    String::from_utf8_lossy(&out).into_owned()
}

pub fn parse_mountinfo(text: &str) -> Vec<Mount> {
    text.lines().filter_map(|l| {
        let (left, right) = l.split_once(" - ")?;
        let f: Vec<&str> = left.split(' ').collect();
        let r: Vec<&str> = right.split(' ').collect();
        Some(Mount { point: unescape(f.get(4)?), fstype: r.first()?.to_string(), source: unescape(r.get(1).unwrap_or(&"")) })
    }).collect()
}

fn mounts() -> Vec<Mount> { parse_mountinfo(&std::fs::read_to_string("/proc/self/mountinfo").unwrap_or_default()) }

/// File systems that hold files people care about (not /proc, tmpfs and the like).
const REAL_FS: [&str; 14] = ["ext4", "ext3", "ext2", "btrfs", "xfs", "f2fs", "vfat", "exfat", "ntfs", "ntfs3", "fuseblk", "bcachefs", "zfs", "jfs"];

#[cfg(unix)]
fn space(p: &str) -> Option<(u64, u64)> {
    let c = std::ffi::CString::new(p).ok()?;
    let mut s: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statvfs(c.as_ptr(), &mut s) } != 0 { return None; }
    let frag = if s.f_frsize > 0 { s.f_frsize as u64 } else { s.f_bsize as u64 };
    let total = s.f_blocks as u64 * frag;
    Some((total.saturating_sub(s.f_bfree as u64 * frag), total))
}
#[cfg(not(unix))]
fn space(_: &str) -> Option<(u64, u64)> { None }

/// The drives worth scanning, one entry per drive (a drive mounted in several places is listed once, at its most
/// natural place), with Home first.
pub fn pick_places(ms: &[Mount], home: &str) -> Vec<(String, String, String)> {
    let mut out: Vec<(String, String, String)> = vec![(home.to_string(), "Home folder".into(), "home".into())];
    let mut seen: Vec<String> = Vec::new();
    let mut real: Vec<&Mount> = ms.iter().filter(|m| REAL_FS.contains(&m.fstype.as_str())).filter(|m| {
        let p = m.point.as_str();
        !(p == "/boot" || p.starts_with("/boot/") || p == "/efi" || p.starts_with("/sysroot/") || p.starts_with("/proc") || p.starts_with("/sys/")
            || p == "/etc" || p == "/usr" || p.starts_with("/var/lib/containers") || p.contains("/.snapshots"))
    }).collect();
    // The most natural place first: / (a normal system), then /var (image-based Hearth OS keeps your data there),
    // then the shortest path.
    let rank = |p: &str| match p { "/" => 0, "/var" => 1, "/var/home" | "/home" => 2, "/sysroot" => 9, _ => 3 };
    real.sort_by(|a, b| rank(&a.point).cmp(&rank(&b.point)).then(a.point.len().cmp(&b.point.len())));
    for m in real {
        let key = if m.source.starts_with("/dev/") { m.source.clone() } else { format!("{}:{}", m.fstype, m.point) };
        if seen.contains(&key) { continue; }
        seen.push(key);
        let label = match m.point.as_str() {
            "/" => "The system's drive".to_string(),
            "/var" => "The system's drive (your data)".to_string(),
            "/sysroot" => "The system's drive (all of it)".to_string(),
            p => Path::new(p).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| p.to_string()),
        };
        let kind = if m.point.starts_with("/run/media/") || m.point.starts_with("/media/") { "removable" } else { "drive" };
        out.push((m.point.clone(), label, kind.into()));
    }
    out
}

pub fn places() -> Result<Value, String> {
    let ms = mounts();
    let home = util::home().to_string_lossy().into_owned();
    let list: Vec<Value> = pick_places(&ms, &home).into_iter().map(|(p, label, kind)| {
        let (used, total) = space(&p).unwrap_or((0, 0));
        json!({"path": p, "label": label, "kind": kind, "used": used, "total": total})
    }).collect();
    Ok(json!({"places": list, "home": home}))
}

static TREE: Mutex<Option<(PathBuf, Node)>> = Mutex::new(None);

/// Scans `path` (a folder), reporting {files, bytes, current} every quarter second. Keeps the tree for view/list.
pub fn scan(path: &str, cancel: &AtomicBool, progress: &(dyn Fn(Value) + Sync)) -> Result<Value, String> {
    let root = PathBuf::from(path);
    if !root.is_absolute() || !root.is_dir() { return Err("That folder isn't there.".into()); }
    let root = std::fs::canonicalize(&root).unwrap_or(root);
    let ms = mounts();
    let ctx = Ctx { cancel, mounts: ms.iter().map(|m| PathBuf::from(&m.point)).collect(), root: root.clone(), links: Mutex::new(HashSet::new()),
        files: AtomicU64::new(0), bytes: AtomicU64::new(0), current: Mutex::new(String::new()) };
    let start = Instant::now();
    let done = AtomicBool::new(false);
    let node = std::thread::scope(|s| {
        s.spawn(|| {
            while !done.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(250));
                let cur = ctx.current.lock().map(|c| c.clone()).unwrap_or_default();
                progress(json!({"files": ctx.files.load(Ordering::Relaxed), "bytes": ctx.bytes.load(Ordering::Relaxed), "current": cur}));
            }
        });
        let n = scan_dir(&root, root.to_string_lossy().into_owned(), &ctx);
        done.store(true, Ordering::Relaxed);
        n
    });
    if cancel.load(Ordering::SeqCst) { return Err("Stopped.".into()); }
    let (used, total) = space(&root.to_string_lossy()).unwrap_or((0, 0));
    let summary = json!({"path": root.to_string_lossy(), "size": node.size, "files": node.files, "ms": start.elapsed().as_millis() as u64,
        "driveUsed": used, "driveTotal": total});
    *TREE.lock().map_err(|_| "busy")? = Some((root, node));
    Ok(summary)
}

fn find<'a>(root: &Path, node: &'a Node, path: &Path) -> Option<&'a Node> {
    let rel = path.strip_prefix(root).ok()?;
    let mut n = node;
    for c in rel.components() {
        let name = c.as_os_str().to_string_lossy();
        n = n.dirs.iter().find(|d| d.name == name)?;
    }
    Some(n)
}

/// One level of the chart or list: the folder's subfolders and named files, biggest first, with what's too small
/// to show folded into "smaller items".
fn view_node(n: &Node, path: &Path, depth: u32, min: u64, max_items: usize) -> Value {
    let mut kids: Vec<Value> = Vec::new();
    let mut items: Vec<(u64, Value)> = Vec::new();
    for d in &n.dirs {
        let p = path.join(&d.name);
        let kind = if d.mount { "mount" } else if d.denied { "denied" } else { "dir" };
        items.push((d.size, if depth > 1 && d.size >= min && !d.mount { let mut v = view_node(d, &p, depth - 1, min, max_items); v["kind"] = json!(kind); v }
            else { json!({"name": d.name, "path": p.to_string_lossy(), "size": d.size, "files": d.files, "kind": kind}) }));
    }
    for f in &n.big {
        items.push((f.size, json!({"name": f.name, "path": path.join(&f.name).to_string_lossy(), "size": f.size, "kind": "file", "mtime": f.mtime})));
    }
    items.sort_by(|a, b| b.0.cmp(&a.0));
    let mut rest = n.small;
    let mut rest_n = n.small_count;
    for (i, (size, v)) in items.into_iter().enumerate() {
        if i < max_items && size >= min { kids.push(v); } else { rest += size; rest_n += 1; }
    }
    if rest > 0 { kids.push(json!({"name": format!("{rest_n} smaller items"), "size": rest, "kind": "small"})); }
    json!({"name": n.name, "path": path.to_string_lossy(), "size": n.size, "files": n.files, "kind": "dir", "children": kids})
}

/// The ring chart's data for a folder of the last scan: `depth` levels, items under 0.4% of it folded together.
pub fn view(path: &str, depth: u32) -> Result<Value, String> {
    let g = TREE.lock().map_err(|_| "busy")?;
    let (root, node) = g.as_ref().ok_or("Nothing scanned yet.")?;
    let p = if path.is_empty() { root.clone() } else { PathBuf::from(path) };
    let n = find(root, node, &p).ok_or("That folder isn't in the scan (scan again?).")?;
    let min = (n.size / 250).max(1);
    let mut v = view_node(n, &p, depth.clamp(1, 5), min, 60);
    v["root"] = json!(root.to_string_lossy());
    v["parent"] = json!(if p == *root { Value::Null } else { json!(p.parent().map(|x| x.to_string_lossy().into_owned())) });
    Ok(v)
}

/// The list beside the chart: everything in one folder, biggest first (up to 500 items).
pub fn list(path: &str) -> Result<Value, String> {
    let g = TREE.lock().map_err(|_| "busy")?;
    let (root, node) = g.as_ref().ok_or("Nothing scanned yet.")?;
    let p = if path.is_empty() { root.clone() } else { PathBuf::from(path) };
    let n = find(root, node, &p).ok_or("That folder isn't in the scan (scan again?).")?;
    Ok(view_node(n, &p, 1, 1, 500))
}

fn collect_big(n: &Node, path: &Path, min: u64, out: &mut Vec<(u64, String, i64)>) {
    for f in &n.big { if f.size >= min { out.push((f.size, path.join(&f.name).to_string_lossy().into_owned(), f.mtime)); } }
    for d in &n.dirs { if d.size >= min { collect_big(d, &path.join(&d.name), min, out); } }
}

/// Quick find: the biggest files of the last scan (100 MB and up, at most 100).
pub fn big_files() -> Result<Value, String> {
    let g = TREE.lock().map_err(|_| "busy")?;
    let (root, node) = g.as_ref().ok_or("Scan something first.")?;
    let mut out = Vec::new();
    collect_big(node, root, 100 * 1000 * 1000, &mut out);
    out.sort_by(|a, b| b.0.cmp(&a.0));
    out.truncate(100);
    Ok(json!({"items": out.into_iter().map(|(s, p, m)| json!({"path": p, "size": s, "mtime": m, "kind": "file"})).collect::<Vec<_>>()}))
}

/// The size of a folder (or file) on the drive, without keeping a tree (quick finds).
fn size_of(p: &Path, cancel: &AtomicBool) -> u64 {
    if cancel.load(Ordering::Relaxed) { return 0; }
    let Ok(m) = std::fs::symlink_metadata(p) else { return 0 };
    if !m.is_dir() { return on_disk(&m); }
    let entries: Vec<PathBuf> = std::fs::read_dir(p).into_iter().flatten().flatten().map(|e| e.path()).collect();
    entries.par_iter().map(|e| size_of(e, cancel)).sum()
}

/// Quick finds: "downloads" (files in Downloads you haven't touched in 90 days) or "caches" (apps' caches,
/// thumbnails, the Trash, Steam's shader caches and Flatpak apps' caches).
pub fn quick(kind: &str, cancel: &AtomicBool) -> Result<Value, String> {
    let h = util::home();
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    match kind {
        "downloads" => {
            let dir = util::user_dir("DOWNLOAD", "Downloads");
            let mut items = Vec::new();
            for e in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
                let Ok(m) = e.metadata() else { continue };
                let touched = mtime(&m).max(m.accessed().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs() as i64).unwrap_or(0));
                if now - touched < 90 * 86400 { continue; }
                let size = size_of(&e.path(), cancel);
                items.push(json!({"path": e.path().to_string_lossy(), "size": size, "mtime": mtime(&m), "kind": if m.is_dir() { "dir" } else { "file" }}));
            }
            items.sort_by(|a, b| b["size"].as_u64().cmp(&a["size"].as_u64()));
            Ok(json!({"items": items, "folder": dir.to_string_lossy()}))
        }
        "caches" => {
            let mut cands: Vec<(PathBuf, String)> = Vec::new();
            let cache = std::env::var_os("XDG_CACHE_HOME").map(PathBuf::from).unwrap_or_else(|| h.join(".cache"));
            for e in std::fs::read_dir(&cache).into_iter().flatten().flatten() {
                let n = e.file_name().to_string_lossy().into_owned();
                cands.push((e.path(), if n == "thumbnails" { "Picture thumbnails".into() } else { format!("{n} (app cache)") }));
            }
            for (p, label) in [
                (h.join(".local/share/Steam/steamapps/shadercache"), "Steam's shader caches (games rebuild them)"),
                (h.join(".var/app/com.valvesoftware.Steam/.local/share/Steam/steamapps/shadercache"), "Steam's shader caches (Flatpak Steam)"),
                (h.join(".local/share/Trash"), "The Trash"),
            ] { cands.push((p, label.into())); }
            for e in std::fs::read_dir(h.join(".var/app")).into_iter().flatten().flatten() {
                cands.push((e.path().join("cache"), format!("{} (app cache)", e.file_name().to_string_lossy())));
            }
            let mut items: Vec<Value> = cands.par_iter().filter(|(p, _)| p.exists()).map(|(p, label)| {
                json!({"path": p.to_string_lossy(), "label": label, "size": size_of(p, cancel), "kind": "dir", "trash": p.ends_with(".local/share/Trash")})
            }).filter(|v| v["size"].as_u64().unwrap_or(0) >= 1_000_000).collect();
            items.sort_by(|a, b| b["size"].as_u64().cmp(&a["size"].as_u64()));
            Ok(json!({"items": items}))
        }
        _ => Err("unknown quick find".into()),
    }
}

/// Removes a moved-away path from the tree and takes its size off every folder above it.
fn forget(root: &Path, node: &mut Node, path: &Path) -> bool {
    let Ok(rel) = path.strip_prefix(root) else { return false };
    let parts: Vec<String> = rel.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect();
    fn walk(n: &mut Node, parts: &[String]) -> Option<(u64, u64)> {
        let (first, rest) = parts.split_first()?;
        let gone = if rest.is_empty() {
            if let Some(i) = n.dirs.iter().position(|d| &d.name == first) { let d = n.dirs.remove(i); Some((d.size, d.files)) }
            else if let Some(i) = n.big.iter().position(|f| &f.name == first) { let f = n.big.remove(i); Some((f.size, 1)) }
            else { None }
        } else {
            let d = n.dirs.iter_mut().find(|d| &d.name == first)?;
            walk(d, rest)
        };
        if let Some((s, f)) = gone { n.size = n.size.saturating_sub(s); n.files = n.files.saturating_sub(f); }
        gone
    }
    walk(node, &parts).is_some()
}

/// Paths the page may act on: absolute, real, not the scanned folder itself, not your home or the system's top
/// folders.
fn may_trash(p: &Path) -> bool {
    let s = p.to_string_lossy();
    let h = util::home();
    p.is_absolute() && !s.contains("/../") && !s.ends_with("/..") && p != h && p.parent().is_some()
        && !["/", "/usr", "/etc", "/var", "/boot", "/sysroot", "/home", "/var/home", "/run", "/proc", "/sys", "/dev", "/opt", "/srv", "/root", "/tmp"].contains(&s.as_ref())
}

/// Moves files and folders to the Trash (Hearth Files' own way: gio trash or the freedesktop Trash).
pub fn trash(paths: &[String]) -> Result<Value, String> {
    if paths.is_empty() { return Err("Nothing chosen.".into()); }
    for p in paths { if !may_trash(Path::new(p)) { return Err(format!("“{p}” can't be moved to the Trash from here.")); } }
    let helper = ["/usr/share/nimbus/desktop/shell/hearth-files", "/usr/local/share/nimbus/desktop/shell/hearth-files"].into_iter().find(|p| Path::new(p).exists());
    let mut args: Vec<&str> = vec!["trash"];
    args.extend(paths.iter().map(String::as_str));
    let o = match helper {
        Some(h) => util::run_in(h, &args, None, Duration::from_secs(300))?,
        None => { let mut a = vec!["trash", "--"]; a.extend(paths.iter().map(String::as_str)); util::run_in("gio", &a, None, Duration::from_secs(300))? }
    };
    let gone: Vec<&String> = paths.iter().filter(|p| std::fs::symlink_metadata(p).is_err()).collect();
    if let Ok(mut g) = TREE.lock() {
        if let Some((root, node)) = g.as_mut() { for p in &gone { forget(root, node, Path::new(p.as_str())); } }
    }
    if gone.len() < paths.len() {
        let why = o.out.lines().chain(o.err.lines()).find_map(|l| l.strip_prefix("error: ")).map(str::to_string).unwrap_or_else(|| o.why());
        return Err(format!("{} of {} moved to the Trash. {}", gone.len(), paths.len(), why));
    }
    Ok(json!({"ok": true, "trashed": gone.len()}))
}

/// Empties the Trash (asked for from the caches quick find).
pub fn empty_trash() -> Result<Value, String> {
    let helper = ["/usr/share/nimbus/desktop/shell/hearth-files", "/usr/local/share/nimbus/desktop/shell/hearth-files"].into_iter().find(|p| Path::new(p).exists())
        .ok_or("Hearth Files isn't installed here.")?;
    let o = util::run(helper, &["empty-trash"], 600)?;
    if o.ok() { Ok(json!({"ok": true})) } else { Err(o.why()) }
}

/// Opens a folder in Hearth Files (a file: its folder, with the file picked).
pub fn open(path: &str) -> Result<Value, String> {
    if !Path::new(path).is_absolute() { return Err("Not a full path.".into()); }
    if util::which("hearth-filemanager").is_some() { util::spawn("hearth-filemanager", &[path])?; }
    else { util::spawn("xdg-open", &[Path::new(path).parent().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default().as_str()])?; }
    Ok(json!({"ok": true}))
}

#[cfg(test)]
mod tests {
    use super::*;

    const MI: &str = "22 1 0:21 / / rw,relatime shared:1 - btrfs /dev/nvme0n1p3 rw,ssd\n\
        23 22 0:5 / /proc rw - proc proc rw\n\
        24 22 0:21 /var /var rw shared:2 - btrfs /dev/nvme0n1p3 rw\n\
        25 22 259:1 / /boot/efi rw - vfat /dev/nvme0n1p1 rw\n\
        26 22 8:17 / /run/media/sam/My\\040Games rw - exfat /dev/sdb1 rw\n\
        27 22 0:30 / /tmp rw - tmpfs tmpfs rw\n";

    #[test]
    fn mountinfo() {
        let m = parse_mountinfo(MI);
        assert_eq!(m.len(), 6);
        assert_eq!(m[4].point, "/run/media/sam/My Games");
        assert_eq!(m[4].fstype, "exfat");
        let p = pick_places(&m, "/home/sam");
        let names: Vec<&str> = p.iter().map(|x| x.0.as_str()).collect();
        assert_eq!(names, ["/home/sam", "/", "/run/media/sam/My Games"]);
        assert_eq!(p[2].2, "removable");
    }

    #[test]
    fn image_based_system_shows_var() {
        let mi = "1 0 0:30 / / ro - overlay composefs ro\n2 1 0:31 / /sysroot ro - btrfs /dev/nvme0n1p3 ro\n3 1 0:31 /ostree/deploy/default/var /var rw - btrfs /dev/nvme0n1p3 rw\n";
        let p = pick_places(&parse_mountinfo(mi), "/var/home/sam");
        assert_eq!(p.iter().map(|x| x.0.as_str()).collect::<Vec<_>>(), ["/var/home/sam", "/var"]);
    }

    fn tmp() -> PathBuf {
        let d = std::env::temp_dir().join(format!("hearth-du-test-{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn scans_and_forgets() {
        let d = tmp();
        std::fs::create_dir_all(d.join("a/b")).unwrap();
        std::fs::write(d.join("a/b/big.bin"), vec![7u8; 600_000]).unwrap();
        std::fs::write(d.join("a/small.txt"), b"hello").unwrap();
        std::fs::write(d.join("top.bin"), vec![1u8; 300_000]).unwrap();
        #[cfg(unix)]
        std::fs::hard_link(d.join("top.bin"), d.join("a/same.bin")).unwrap();
        let stop = AtomicBool::new(false);
        let s = scan(d.to_str().unwrap(), &stop, &|_| {}).unwrap();
        assert_eq!(s["files"], if cfg!(unix) { 4 } else { 3 });
        let total = s["size"].as_u64().unwrap();
        assert!(total >= 900_000, "{total}");
        let v = view("", 3).unwrap();
        assert_eq!(v["children"][0]["name"], "a");
        assert_eq!(v["children"][0]["children"][0]["name"], "b");
        let l = list(d.join("a/b").to_str().unwrap()).unwrap();
        assert_eq!(l["children"][0]["kind"], "file");
        {
            let mut g = TREE.lock().unwrap();
            let (root, node) = g.as_mut().unwrap();
            assert!(forget(root, node, &d.join("a/b/big.bin")));
            assert!(node.size < total - 500_000);
        }
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn trash_refuses_system_folders() {
        assert!(!may_trash(Path::new("/")) && !may_trash(Path::new("/usr")) && !may_trash(&util::home()) && !may_trash(Path::new("relative")));
        assert!(may_trash(Path::new("/var/home/sam/Downloads/x.iso")));
    }
}
