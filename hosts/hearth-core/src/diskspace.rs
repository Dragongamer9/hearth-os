//! Settings › Storage (docs/STORAGE.md): every drive Hearth can put games on with what fills it, every installed
//! game by size (its folder, Proton prefix and shader cache apart), moving a game to another drive with one press,
//! uninstalling, cleaning up (shader caches, prefixes of games that are gone, unused Proton versions, leftover
//! downloads) and the low space warning.
//!
//! Safety, everywhere in this file:
//! * Only known game and cache places are ever deleted (`Guard::remove`): a path must be inside the folder it was
//!   found in (a Steam library's steamapps/<part>, Hearth's prefixes folder, a games folder, a cache folder), must
//!   not be a system area (`SYSTEM_ROOTS`), the home folder or one of its parents, or a mount point. Links are
//!   removed, never followed.
//! * Moving copies first, checks every byte of the copy against the original (read back from the drive), switches
//!   the store's record over, and only then deletes the original. A journal (diskspace-move.json) makes a move
//!   that was cut off (power, crash) finish or undo the next time the page is opened. Nothing runs at start-up.
//! * Prefixes with saves are kept unless Steam Cloud has them, and then only after a second confirmation.
//! * Every delete is listed in Hearth's log first (a dry-run listing), then done.

use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

fn log(t: &str) { crate::bridge::log(&format!("storage: {t}")); }
fn now() -> u64 { crate::library::now_ms() }
const GB: u64 = 1_000_000_000;
const CHUNK: usize = 8 << 20;
pub const STOPPED: &str = "Stopped";

/// Never deleted, whatever a record or a list says.
pub const SYSTEM_ROOTS: &[&str] = &["/usr", "/etc", "/boot", "/efi", "/sysroot", "/ostree", "/proc", "/sys", "/dev", "/bin", "/sbin",
    "/lib", "/lib32", "/lib64", "/opt", "/root", "/srv", "/var/lib", "/var/log", "/var/cache", "/var/tmp", "/var/spool", "/tmp", "/snap", "/nix"];

fn read_json(p: &Path) -> Value { std::fs::read(p).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null) }
fn dev_of(p: &Path) -> Option<u64> { std::fs::metadata(p).ok().map(|m| m.dev()) }
fn exists(p: &Path) -> bool { std::fs::symlink_metadata(p).is_ok() }
fn s(p: &Path) -> String { p.to_string_lossy().to_string() }

// ------------------------------------------------------------------ sizes

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct TreeSize { pub bytes: u64, pub apparent: u64, pub files: u64 }

/// How much a folder (or a file) takes on its drive, like du: allocated blocks, links not followed, drives mounted
/// inside left out, hard links counted once. Nothing there: zero.
pub fn tree_size(p: &Path) -> TreeSize {
    let mut out = TreeSize::default();
    let Ok(top) = std::fs::symlink_metadata(p) else { return out };
    let dev = top.dev();
    let mut seen: HashSet<(u64, u64)> = HashSet::new();
    let mut stack: Vec<(PathBuf, std::fs::Metadata)> = vec![(p.to_path_buf(), top)];
    while let Some((path, m)) = stack.pop() {
        let ft = m.file_type();
        if ft.is_symlink() { out.files += 1; continue; }
        if !ft.is_dir() && m.nlink() > 1 && !seen.insert((m.dev(), m.ino())) { continue; }
        out.bytes += m.blocks() * 512;
        if ft.is_file() { out.apparent += m.len(); out.files += 1; }
        if ft.is_dir() {
            let Ok(rd) = std::fs::read_dir(&path) else { continue };
            for e in rd.flatten() {
                // DirEntry::metadata doesn't follow links on Unix.
                let Ok(cm) = e.metadata() else { continue };
                if cm.dev() != dev { continue; }
                stack.push((e.path(), cm));
            }
        }
    }
    out
}

/// A path worth adding up: not the home folder, the root or a top-level folder (a game you added yourself whose
/// "folder" is your home would otherwise count everything you have).
pub fn sizable(p: &Path, home: &Path) -> bool {
    p.is_absolute() && p.components().count() >= 3 && p != home && !home.starts_with(p)
}

// ------------------------------------------------------------------ drives

#[derive(Debug, Clone, PartialEq, Default)]
pub struct DfRow { pub source: String, pub fstype: String, pub size: u64, pub used: u64, pub free: u64, pub mount: String }

/// df -B1 --output=source,fstype,size,used,avail,target: one row per path asked (the mount point can have spaces).
pub fn parse_df(text: &str) -> Vec<DfRow> {
    let mut out = Vec::new();
    for line in text.lines().skip(1) {
        let mut parts = line.split_whitespace();
        let (Some(source), Some(fstype), Some(size), Some(used), Some(free)) = (parts.next(), parts.next(), parts.next(), parts.next(), parts.next()) else { continue };
        let (Ok(size), Ok(used), Ok(free)) = (size.parse::<u64>(), used.parse::<u64>(), free.parse::<u64>()) else { continue };
        let mount = parts.collect::<Vec<_>>().join(" ");
        if mount.is_empty() { continue; }
        out.push(DfRow { source: source.into(), fstype: fstype.into(), size, used, free, mount });
    }
    out
}

/// File systems that never hold games (memory, the image-based system's read-only root, CDs).
pub fn real_fs(t: &str) -> bool {
    !matches!(t, "tmpfs" | "devtmpfs" | "overlay" | "squashfs" | "proc" | "sysfs" | "ramfs" | "composefs" | "erofs" | "iso9660" | "udf" | "autofs" | "efivarfs")
        && !t.starts_with("fuse.portal")
}

/// Mount points from /proc/self/mounts that are read-only ("ro" first in the options).
pub fn read_only_mounts(text: &str) -> HashSet<String> {
    let un = |x: &str| x.replace("\\040", " ").replace("\\011", "\t").replace("\\012", "\n").replace("\\134", "\\");
    text.lines().filter_map(|l| {
        let f: Vec<&str> = l.split(' ').collect();
        if f.len() < 4 { return None; }
        if f[3].split(',').next() == Some("ro") { Some(un(f[1])) } else { None }
    }).collect()
}

/// What kind of drive a mount is: the one Hearth (your home folder) is on, an SD card, a drive set up with Hearth
/// Disks' "Use for games", a USB drive (or another drive the desktop mounted), or another fixed drive.
pub fn kind_of(source: &str, mount: &str, system: bool) -> &'static str {
    if system { return "system"; }
    if source.starts_with("/dev/mmcblk") { return "sd"; }
    if mount.starts_with("/var/mnt/games-") || mount.starts_with("/mnt/games-") { return "games"; }
    if mount.starts_with("/run/media/") || mount.starts_with("/media/") { return "usb"; }
    "extra"
}

pub fn name_of(kind: &str, mount: &str) -> String {
    let base = Path::new(mount).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let base = base.strip_prefix("games-").map(str::to_string).unwrap_or(base);
    match kind {
        "system" => "This device".into(),
        "sd" => if base.is_empty() { "SD card".into() } else { format!("SD card ({base})") },
        _ => if base.is_empty() { mount.to_string() } else { base },
    }
}

#[derive(Debug, Clone, Default)]
pub struct Drive {
    pub name: String, pub kind: &'static str, pub mount: PathBuf, pub fstype: String, pub source: String,
    pub size: u64, pub used: u64, pub free: u64, pub dev: u64, pub read_only: bool, pub steam_libs: Vec<PathBuf>,
}

/// Places a games drive can be mounted: Hearth Disks' (/var/mnt/games-*), the desktop's (/run/media/<you>/*), /mnt, /media.
pub fn mount_candidates() -> Vec<PathBuf> {
    let user = std::env::var("USER").unwrap_or_default();
    let mut v = Vec::new();
    for base in [PathBuf::from("/var/mnt"), PathBuf::from("/mnt"), PathBuf::from("/run/media").join(&user), PathBuf::from("/media").join(&user), PathBuf::from("/media")] {
        for e in std::fs::read_dir(&base).into_iter().flatten().flatten() { let p = e.path(); if p.is_dir() { v.push(p); } }
    }
    v
}

/// The drives games can be on, from df rows: home's drive first, each mount once, memory and system file systems
/// left out. `devs`: (mount, its device number) for the rows (read from the drive; tests give them).
pub fn drives_from(rows: &[DfRow], home_dev: Option<u64>, devs: &HashMap<String, u64>, ro: &HashSet<String>, libs: &[(PathBuf, u64)]) -> Vec<Drive> {
    let mut out: Vec<Drive> = Vec::new();
    for r in rows {
        if !real_fs(&r.fstype) || r.size == 0 { continue; }
        if out.iter().any(|d| s(&d.mount) == r.mount) { continue; }
        let Some(dev) = devs.get(&r.mount).copied() else { continue };
        if out.iter().any(|d| d.dev == dev) { continue; }
        if ["/boot", "/boot/efi", "/efi", "/usr", "/etc"].contains(&r.mount.as_str()) { continue; }
        let system = home_dev == Some(dev);
        let kind = kind_of(&r.source, &r.mount, system);
        out.push(Drive { name: name_of(kind, &r.mount), kind, mount: PathBuf::from(&r.mount), fstype: r.fstype.clone(), source: r.source.clone(),
            size: r.size, used: r.used, free: r.free, dev, read_only: ro.contains(&r.mount),
            steam_libs: libs.iter().filter(|(_, d)| *d == dev).map(|(l, _)| l.clone()).collect() });
    }
    out.sort_by_key(|d| if d.kind == "system" { 0 } else { 1 });
    out
}

/// The drives here (df over home, the Steam libraries, Hearth's install folder and the usual mount places).
pub fn drives(ctx: &Ctx) -> Vec<Drive> {
    let mut paths: Vec<PathBuf> = vec![ctx.home.clone(), ctx.install_root.clone()];
    paths.extend(ctx.libs.iter().cloned());
    paths.extend(mount_candidates());
    let mut seen = HashSet::new();
    let paths: Vec<PathBuf> = paths.into_iter().filter(|p| p.is_dir() && seen.insert(p.clone())).collect();
    let mut args: Vec<String> = vec!["-B1".into(), "--output=source,fstype,size,used,avail,target".into(), "--".into()];
    args.extend(paths.iter().map(|p| s(p)));
    // df says 1 when one path can't be read and still lists the others: its output is used either way.
    let text = std::process::Command::new("df").args(&args).env("LC_ALL", "C").stdin(std::process::Stdio::null()).stderr(std::process::Stdio::null())
        .output().map(|o| String::from_utf8_lossy(&o.stdout).to_string()).unwrap_or_default();
    let rows = parse_df(&text);
    let devs: HashMap<String, u64> = rows.iter().filter_map(|r| dev_of(Path::new(&r.mount)).map(|d| (r.mount.clone(), d))).collect();
    let ro = read_only_mounts(&std::fs::read_to_string("/proc/self/mounts").unwrap_or_default());
    let libs: Vec<(PathBuf, u64)> = ctx.libs.iter().filter_map(|l| dev_of(l).map(|d| (l.clone(), d))).collect();
    drives_from(&rows, dev_of(&ctx.home), &devs, &ro, &libs)
}

// ------------------------------------------------------------------ what Hearth knows (from the library)

#[derive(Debug, Clone, Default)]
pub struct GameRef { pub id: String, pub title: String, pub source: String, pub path: PathBuf, pub shared: bool }

/// What this file needs from the library, copied so the slow work runs off the bridge.
#[derive(Debug, Clone, Default)]
pub struct Ctx {
    pub home: PathBuf, pub data: PathBuf, pub steam: Option<PathBuf>, pub libs: Vec<PathBuf>, pub install_root: PathBuf,
    pub games: Vec<GameRef>,
    /// Every game's title (installed or not): names prefixes of games that are gone.
    pub names: HashMap<String, String>,
    /// GOG games Hearth installed: id → folder (library.json gogInstalls).
    pub gog: HashMap<String, String>,
    /// Proton versions picked in Game settings (paths or names).
    pub protons: Vec<String>,
    /// Games running or put aside (Quick Resume), and games downloading in Hearth's queue.
    pub running: Vec<String>, pub queued: Vec<String>,
}

pub fn ctx_from(lib: &crate::library::Library) -> Ctx {
    let home = lib.home.clone();
    let steam = crate::steam::steam_path(&home);
    let libs = steam.as_ref().map(|st| crate::steam::libraries(st)).unwrap_or_default();
    let games = lib.games.values().filter(|g| g.installed && !g.cloud).filter_map(|g| {
        let path = g.install_path.clone()?;
        Some(GameRef { id: g.id.clone(), title: g.title.clone(), source: g.source.to_string(), path, shared: g.shared_from.is_some() || g.borrowed })
    }).collect();
    let names = lib.games.values().map(|g| (g.id.clone(), g.title.clone())).collect();
    let gog = lib.store.data["gogInstalls"].as_object().map(|o| o.iter().filter_map(|(k, v)| v.as_str().map(|x| (k.clone(), x.to_string()))).collect()).unwrap_or_default();
    let protons = lib.store.data["gameOptions"].as_object().map(|o| o.values().filter_map(|v| v["proton"].as_str().filter(|x| !x.is_empty()).map(str::to_string)).collect()).unwrap_or_default();
    let mut running: Vec<String> = lib.running_id().into_iter().collect();
    running.extend(lib.frozen.iter().map(|f| f.id.clone()));
    let queued = lib.queue.lock().map(|q| q.ids()).unwrap_or_default();
    Ctx { home, data: crate::paths::data_dir(), steam, libs, install_root: lib.install_root(), games, names, gog, protons, running, queued }
}

/// <lib>/steamapps/common/<dir> → <lib>
pub fn steam_lib_of(game: &Path) -> Option<PathBuf> {
    let common = game.parent()?;
    if common.file_name()? != "common" { return None; }
    let apps = common.parent()?;
    if apps.file_name()? != "steamapps" { return None; }
    apps.parent().map(Path::to_path_buf)
}

/// Hearth's Proton prefix for an Epic or GOG game (the same folder stores::prefix_of gives).
pub fn hearth_prefix(data: &Path, store: &str, id: &str) -> PathBuf {
    let safe: String = id.chars().map(|c| if c.is_ascii_alphanumeric() || "._-".contains(c) { c } else { '_' }).collect();
    data.join("prefixes").join(format!("{store}-{safe}"))
}

/// A game's Proton prefix and shader cache, where it has them.
pub fn parts_of(g: &GameRef, data: &Path) -> (Option<PathBuf>, Option<PathBuf>) {
    match g.source.as_str() {
        "steam" => {
            let Some(app) = g.id.strip_prefix("steam:") else { return (None, None) };
            let Some(lib) = steam_lib_of(&g.path) else { return (None, None) };
            let apps = lib.join("steamapps");
            (Some(apps.join("compatdata").join(app)), Some(apps.join("shadercache").join(app)))
        }
        "epic" | "gog" => {
            let id = g.id.split_once(':').map(|x| x.1).unwrap_or("");
            (Some(hearth_prefix(data, &g.source, id)), None)
        }
        _ => (None, None),
    }
}

pub fn store_label(source: &str) -> &'static str {
    match source {
        "steam" => "Steam", "epic" => "Epic Games", "gog" => "GOG", "battlenet" => "Battle.net", "xodus" => "Xbox PC", "xbox" => "Xbox",
        "beacon" => "Beacon", "recomps" => "Recomps", "emu" => "Emulators", "manual" => "Your games", _ => "Other",
    }
}

/// Whether Hearth can move a game to another drive itself, or why not (said on the page instead of trying).
pub fn move_support(g: &GameRef) -> Result<(), String> {
    if g.shared { return Err("This is another player's copy (Multi-seat). Only they can move it.".into()); }
    match g.source.as_str() {
        "steam" => if steam_lib_of(&g.path).is_some() && g.id.strip_prefix("steam:").and_then(|a| a.parse::<u32>().ok()).is_some() { Ok(()) }
                   else { Err("Steam keeps this game somewhere Hearth doesn't recognise as a Steam library.".into()) },
        "epic" | "gog" => Ok(()),
        "recomps" => Ok(()),
        "battlenet" => Err("Battle.net keeps its games inside its own Wine prefix: move them with Battle.net itself.".into()),
        "xodus" => Err("Xodus Nexus manages its games: change where it keeps them in Xodus Nexus.".into()),
        "beacon" => Err("Beacon's games are moved in Beacon (its game page shows where each one is).".into()),
        "emu" => Err("ROMs are files in your ROMs folder: move them with Files, then refresh the library.".into()),
        _ => Err("Hearth doesn't know which files belong to a game you added yourself: move its folder with Files and add it again.".into()),
    }
}

/// How a game is uninstalled from this page: through the same path the library uses ("game": game.uninstall,
/// "recomp": recomps.uninstall), or why not here.
pub fn uninstall_support(g: &GameRef) -> Result<&'static str, String> {
    if g.shared { return Err("This is another player's copy (Multi-seat).".into()); }
    match g.source.as_str() {
        "steam" | "epic" | "gog" => Ok("game"),
        "recomps" => Ok("recomp"),
        "battlenet" => Err("Uninstall it in Battle.net.".into()),
        "xodus" => Err("Uninstall it in Xodus Nexus.".into()),
        "beacon" => Err("Uninstall it in Beacon.".into()),
        _ => Err("Remove it from the library; its files stay where they are.".into()),
    }
}

// ------------------------------------------------------------------ the size scan (in the background)

static SCAN: Mutex<Option<Value>> = Mutex::new(None);
static SCANNING: AtomicBool = AtomicBool::new(false);
fn cache_file(data: &Path) -> PathBuf { data.join("diskspace-sizes.json") }

fn scan_state(data: &Path) -> Value {
    let mut g = SCAN.lock().unwrap_or_else(|p| p.into_inner());
    if g.is_none() { let v = read_json(&cache_file(data)); if v.is_object() { *g = Some(v); } }
    g.clone().unwrap_or_else(|| json!({"at": 0, "games": {}, "totals": {}}))
}

fn emit(data: Value) { crate::stores::notify(json!({"type": "event", "name": "diskspace", "data": data})); }

/// Gpu shader caches outside Steam (Mesa's, Nvidia's), also inside the Steam Flatpak.
pub fn gpu_caches(home: &Path) -> Vec<PathBuf> {
    let mut bases = vec![home.join(".cache")];
    for id in crate::steam::FLATPAK_IDS { bases.push(home.join(".var/app").join(id).join("cache")); }
    let mut v = Vec::new();
    for b in &bases {
        for n in ["mesa_shader_cache", "mesa_shader_cache_db", "radv_builtin_shaders", "nvidia/GLCache", "nvidia/DXCache"] { v.push(b.join(n)); }
    }
    v.push(home.join(".nv/GLCache"));
    v
}

/// Starts adding up every game, prefix and cache (a few seconds to a minute; events "diskspace" {scan}).
pub fn scan(ctx: Ctx) -> Value {
    if SCANNING.swap(true, Ordering::SeqCst) { return json!({"scanning": true}); }
    std::thread::spawn(move || {
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| scan_now(&ctx)));
        SCANNING.store(false, Ordering::SeqCst);
        match r {
            Ok(v) => {
                let _ = crate::paths::write_atomic(&cache_file(&ctx.data), v.to_string().as_bytes());
                *SCAN.lock().unwrap_or_else(|p| p.into_inner()) = Some(v);
                emit(json!({"scan": {"state": "done", "at": now()}}));
            }
            Err(_) => { log("size scan failed"); emit(json!({"scan": {"state": "error"}})); }
        }
    });
    json!({"scanning": true})
}

fn scan_now(ctx: &Ctx) -> Value {
    let started = now();
    let drives = drives(ctx);
    let mount_of = |p: &Path| -> Option<String> { let d = dev_of(p)?; drives.iter().find(|x| x.dev == d).map(|x| s(&x.mount)) };
    let mut totals: HashMap<String, [u64; 4]> = HashMap::new();
    let mut add = |p: &Path, i: usize, n: u64| { if n == 0 { return; } if let Some(m) = mount_of(p) { totals.entry(m).or_insert([0; 4])[i] += n; } };
    let n = ctx.games.len().max(1);
    let mut last = std::time::Instant::now();
    let mut games = serde_json::Map::new();
    let mut sized: HashSet<PathBuf> = HashSet::new();
    for (i, g) in ctx.games.iter().enumerate() {
        if last.elapsed() > Duration::from_millis(700) { last = std::time::Instant::now(); emit(json!({"scan": {"state": "scanning", "done": i, "total": n, "name": g.title}})); }
        if !sizable(&g.path, &ctx.home) || !sized.insert(g.path.clone()) { continue; }
        let size = tree_size(&g.path).bytes;
        add(&g.path, 0, size);
        let (pre, sh) = parts_of(g, &ctx.data);
        let pre_b = pre.as_deref().map(|p| tree_size(p).bytes).unwrap_or(0);
        let sh_b = sh.as_deref().map(|p| tree_size(p).bytes).unwrap_or(0);
        games.insert(g.id.clone(), json!({"path": s(&g.path), "size": size, "prefix": pre_b, "shader": sh_b}));
    }
    // Per drive, orphans included: every prefix, shader cache and unfinished download.
    for lib in &ctx.libs {
        let apps = lib.join("steamapps");
        add(&apps, 1, tree_size(&apps.join("compatdata")).bytes);
        add(&apps, 2, tree_size(&apps.join("shadercache")).bytes);
        add(&apps, 3, tree_size(&apps.join("downloading")).bytes + tree_size(&apps.join("temp")).bytes);
    }
    let pre = ctx.data.join("prefixes");
    add(&pre, 1, tree_size(&pre).bytes);
    for c in gpu_caches(&ctx.home) { add(&c, 2, tree_size(&c).bytes); }
    let totals: serde_json::Map<String, Value> = totals.into_iter().map(|(k, v)| (k, json!({"games": v[0], "prefixes": v[1], "shader": v[2], "downloads": v[3]}))).collect();
    log(&format!("sizes: {} games on {} drives in {} s", games.len(), totals.len(), now().saturating_sub(started) / 1000));
    json!({"at": now(), "games": games, "totals": totals})
}

// ------------------------------------------------------------------ the page's state

pub fn warn_settings(data: &Value) -> (bool, u64) {
    let w = &data["lowSpace"];
    (w["on"] != false, w["gb"].as_u64().filter(|g| (1..=500).contains(g)).unwrap_or(15))
}

/// Drives that hold games (Hearth's own, Steam libraries, the install folder's, SD cards and games drives).
fn games_drive(d: &Drive, ctx: &Ctx) -> bool {
    d.kind == "system" || d.kind == "sd" || d.kind == "games" || !d.steam_libs.is_empty() || dev_of(&ctx.install_root) == Some(d.dev)
        || ctx.games.iter().any(|g| dev_of(&g.path) == Some(d.dev))
}

/// Games drives below the warning threshold.
pub fn low_from(drives: &[(String, u64, String, bool)], gb: u64) -> Vec<Value> {
    drives.iter().filter(|(_, free, _, games)| *games && *free < gb * GB).map(|(name, free, mount, _)| json!({"name": name, "free": free, "mount": mount})).collect()
}

pub fn low(ctx: &Ctx, warn: (bool, u64)) -> Value {
    if !warn.0 { return json!({"on": false, "low": []}); }
    let ds: Vec<(String, u64, String, bool)> = drives(ctx).iter().filter(|d| !d.read_only).map(|d| (d.name.clone(), d.free, s(&d.mount), games_drive(d, ctx))).collect();
    json!({"on": true, "gb": warn.1, "low": low_from(&ds, warn.1)})
}

/// Before an install: is the drive it goes to low on space? {low, free, name, gb}
pub fn install_check(dest: &Path, warn: (bool, u64)) -> Value {
    let free = crate::steam_ctl::free_bytes(dest);
    let low = warn.0 && free.map(|f| f < warn.1 * GB).unwrap_or(false);
    json!({"low": low, "free": free, "gb": warn.1, "path": s(dest)})
}

pub fn status(ctx: &Ctx, warn: (bool, u64), want_scan: bool) -> Value {
    let recovered = if MOVING.load(Ordering::SeqCst) { None } else { recover(ctx) };
    let drives = drives(ctx);
    let cache = scan_state(&ctx.data);
    let stale = cache["at"].as_u64().map(|t| now().saturating_sub(t) > 30 * 60_000).unwrap_or(true);
    if want_scan && stale { scan(ctx.clone()); }
    let ds: Vec<Value> = drives.iter().map(|d| {
        let t = &cache["totals"][s(&d.mount)];
        let parts = ["games", "prefixes", "shader", "downloads"].map(|k| t[k].as_u64().unwrap_or(0));
        let known: u64 = parts.iter().sum();
        json!({"id": s(&d.mount), "name": d.name, "kind": d.kind, "mount": s(&d.mount), "fs": d.fstype, "size": d.size, "used": d.used, "free": d.free,
               "readOnly": d.read_only, "games": games_drive(d, ctx), "steamLibs": d.steam_libs.iter().map(|l| s(l)).collect::<Vec<_>>(),
               "parts": {"games": parts[0], "prefixes": parts[1], "shader": parts[2], "downloads": parts[3], "other": d.used.saturating_sub(known)}})
    }).collect();
    let games: Vec<Value> = ctx.games.iter().map(|g| {
        let c = &cache["games"][&g.id];
        let fresh = c["path"].as_str() == Some(s(&g.path).as_str());
        let drive = dev_of(&g.path).and_then(|d| drives.iter().find(|x| x.dev == d)).map(|x| s(&x.mount));
        let mv = move_support(g);
        let un = uninstall_support(g);
        json!({"id": g.id, "title": g.title, "source": g.source, "store": store_label(&g.source), "folder": g.path.file_name().map(|f| f.to_string_lossy().to_string()),
               "path": s(&g.path), "drive": drive, "size": if fresh { c["size"].clone() } else { Value::Null }, "prefix": if fresh { c["prefix"].clone() } else { Value::Null },
               "shader": if fresh { c["shader"].clone() } else { Value::Null }, "canMove": mv.is_ok(), "moveWhy": mv.err(),
               "uninstall": un.as_ref().ok(), "uninstallWhy": un.err(), "running": ctx.running.contains(&g.id)})
    }).collect();
    let (on, gb) = warn;
    let low: Vec<(String, u64, String, bool)> = drives.iter().filter(|d| !d.read_only).map(|d| (d.name.clone(), d.free, s(&d.mount), games_drive(d, ctx))).collect();
    json!({"drives": ds, "games": games, "scan": {"state": if SCANNING.load(Ordering::SeqCst) { "scanning" } else { "idle" }, "at": cache["at"]},
           "job": job_state(), "warn": {"on": on, "gb": gb}, "low": if on { low_from(&low, gb) } else { vec![] }, "recovered": recovered})
}

// ------------------------------------------------------------------ deleting, only where it's known to be safe

pub struct Guard { pub home: PathBuf, pub system: Vec<PathBuf> }

impl Guard {
    pub fn real(home: &Path) -> Guard {
        // Tests work in temporary folders (under /tmp): there, only /tmp itself is allowed.
        let system = SYSTEM_ROOTS.iter().filter(|r| !cfg!(test) || **r != "/tmp").map(PathBuf::from).collect();
        Guard { home: home.to_path_buf(), system }
    }

    /// System areas, the home folder and its parents, relative or odd paths, top-level folders.
    pub fn forbidden(&self, p: &Path) -> bool {
        let t = p.to_string_lossy();
        if !p.is_absolute() || t.contains("/../") || t.ends_with("/..") || t.contains("/./") || p.components().count() < 3 { return true; }
        if self.system.iter().any(|r| p.starts_with(r)) { return true; }
        if p.starts_with("/run") && !p.starts_with("/run/media") { return true; }
        if p == self.home || self.home.starts_with(p) { return true; }
        false
    }

    /// Whether `p` may be deleted as something found inside `within` (at least one level below it, its real parent
    /// inside within's real path), and isn't forbidden or a mount point.
    pub fn allowed(&self, p: &Path, within: &Path) -> Result<(), String> {
        let refuse = |why: &str| Err(format!("{} was left alone: {why}", p.display()));
        if self.forbidden(p) || self.forbidden(within) && within != self.home { return refuse("it's a protected place"); }
        let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        if name.is_empty() || name == "." || name == ".." { return refuse("not a folder name"); }
        if !p.starts_with(within) || p == within { return refuse("it isn't inside the place it was found in"); }
        let parent = p.parent().and_then(|x| std::fs::canonicalize(x).ok());
        let w = std::fs::canonicalize(within).ok();
        match (parent, w) {
            (Some(pa), Some(w)) if pa.starts_with(&w) => {}
            _ => return refuse("its folder leads somewhere else"),
        }
        if let (Ok(m), Some(pm)) = (std::fs::symlink_metadata(p), p.parent().and_then(|x| std::fs::metadata(x).ok())) {
            if !m.file_type().is_symlink() && m.is_dir() && m.dev() != pm.dev() { return refuse("it's a drive of its own"); }
        }
        Ok(())
    }

    /// Deletes it when `allowed`: a link or file is removed, a folder with everything in it (links inside are
    /// removed, never followed). Missing already: fine.
    pub fn remove(&self, p: &Path, within: &Path) -> Result<(), String> {
        let Ok(m) = std::fs::symlink_metadata(p) else { return Ok(()) };
        self.allowed(p, within)?;
        let r = if m.file_type().is_symlink() || !m.is_dir() { std::fs::remove_file(p) } else { std::fs::remove_dir_all(p) };
        r.map_err(|e| format!("Couldn't delete {} ({e})", p.display()))
    }
}

// ------------------------------------------------------------------ copying and checking

fn read_full(f: &mut std::fs::File, buf: &mut [u8]) -> std::io::Result<usize> {
    use std::io::Read;
    let mut n = 0;
    while n < buf.len() { let k = f.read(&mut buf[n..])?; if k == 0 { break; } n += k; }
    Ok(n)
}

fn copy_file(from: &Path, to: &Path, m: &std::fs::Metadata, cancel: &AtomicBool, tick: &mut dyn FnMut(u64)) -> Result<(), String> {
    use std::io::Write;
    let err = |e: std::io::Error| format!("Couldn't copy {} ({e})", from.display());
    let mut a = std::fs::File::open(from).map_err(err)?;
    let mut b = std::fs::OpenOptions::new().write(true).create_new(true).open(to).map_err(|e| format!("Couldn't write {} ({e})", to.display()))?;
    let mut buf = vec![0u8; (m.len() as usize).clamp(1, CHUNK)];
    loop {
        if cancel.load(Ordering::SeqCst) { return Err(STOPPED.into()); }
        let n = read_full(&mut a, &mut buf).map_err(err)?;
        if n == 0 { break; }
        b.write_all(&buf[..n]).map_err(|e| format!("Couldn't write {} ({e})", to.display()))?;
        tick(n as u64);
        if n < buf.len() { break; }
    }
    drop(a);
    let _ = std::fs::set_permissions(to, std::fs::Permissions::from_mode(m.mode() & 0o7777));
    if let Ok(t) = m.modified() { let _ = b.set_modified(t); }
    Ok(())
}

fn copy_entry(from: &Path, to: &Path, m: &std::fs::Metadata, cancel: &AtomicBool, tick: &mut dyn FnMut(u64)) -> Result<(), String> {
    if cancel.load(Ordering::SeqCst) { return Err(STOPPED.into()); }
    let ft = m.file_type();
    if ft.is_symlink() {
        let target = std::fs::read_link(from).map_err(|e| format!("Couldn't read the link {} ({e})", from.display()))?;
        std::os::unix::fs::symlink(&target, to).map_err(|e| format!("Couldn't make the link {} ({e})", to.display()))?;
    } else if ft.is_dir() {
        std::fs::create_dir(to).map_err(|e| format!("Couldn't make {} ({e})", to.display()))?;
        let rd = std::fs::read_dir(from).map_err(|e| format!("Couldn't read {} ({e})", from.display()))?;
        for e in rd {
            let e = e.map_err(|e| format!("Couldn't read {} ({e})", from.display()))?;
            let cm = e.metadata().map_err(|err| format!("Couldn't read {} ({err})", e.path().display()))?;
            copy_entry(&e.path(), &to.join(e.file_name()), &cm, cancel, tick)?;
        }
        let _ = std::fs::set_permissions(to, std::fs::Permissions::from_mode(m.mode() & 0o7777));
    } else if ft.is_file() {
        copy_file(from, to, m, cancel, tick)?;
    } else {
        return Err(format!("{} is a special file (a pipe or device) that can't be copied", from.display()));
    }
    Ok(())
}

/// Copies a folder (or a file, or a link) to `to`, which must not exist yet. `tick` hears each piece's bytes.
pub fn copy_tree(from: &Path, to: &Path, cancel: &AtomicBool, tick: &mut dyn FnMut(u64)) -> Result<(), String> {
    let top = std::fs::symlink_metadata(from).map_err(|e| format!("Couldn't read {} ({e})", from.display()))?;
    if exists(to) { return Err(format!("{} is already there", to.display())); }
    if let Some(p) = to.parent() { std::fs::create_dir_all(p).map_err(|e| format!("Couldn't make {} ({e})", p.display()))?; }
    copy_entry(from, to, &top, cancel, tick)
}

/// The copy is read from the drive, not from memory (what was just written is still cached).
fn drop_cache(f: &std::fs::File) {
    use std::os::unix::io::AsRawFd;
    unsafe { libc::posix_fadvise(f.as_raw_fd(), 0, 0, libc::POSIX_FADV_DONTNEED); }
}

fn same_bytes(a: &Path, b: &Path, len: u64, cancel: &AtomicBool, tick: &mut dyn FnMut(u64)) -> Result<(), String> {
    let mut fa = std::fs::File::open(a).map_err(|e| format!("Couldn't read {} ({e})", a.display()))?;
    let mut fb = std::fs::File::open(b).map_err(|e| format!("Couldn't read the copy {} ({e})", b.display()))?;
    drop_cache(&fb);
    let size = (len as usize).clamp(1, CHUNK);
    let (mut ba, mut bb) = (vec![0u8; size], vec![0u8; size]);
    loop {
        if cancel.load(Ordering::SeqCst) { return Err(STOPPED.into()); }
        let na = read_full(&mut fa, &mut ba).map_err(|e| format!("Couldn't read {} ({e})", a.display()))?;
        let nb = read_full(&mut fb, &mut bb).map_err(|e| format!("Couldn't read the copy {} ({e})", b.display()))?;
        if na != nb || ba[..na] != bb[..nb] { return Err(format!("{} doesn't match the original", b.display())); }
        if na == 0 { break; }
        tick(na as u64);
    }
    drop_cache(&fb);
    Ok(())
}

fn verify_entry(from: &Path, to: &Path, cancel: &AtomicBool, tick: &mut dyn FnMut(u64), count: &mut u64) -> Result<(), String> {
    *count += 1;
    let a = std::fs::symlink_metadata(from).map_err(|e| format!("Couldn't read {} ({e})", from.display()))?;
    let b = std::fs::symlink_metadata(to).map_err(|_| format!("{} is missing in the copy", to.display()))?;
    let (fa, fb) = (a.file_type(), b.file_type());
    if fa.is_symlink() != fb.is_symlink() || fa.is_dir() != fb.is_dir() || fa.is_file() != fb.is_file() { return Err(format!("{} isn't the same kind of thing as the original", to.display())); }
    if fa.is_symlink() {
        if std::fs::read_link(from).ok() != std::fs::read_link(to).ok() { return Err(format!("The link {} points somewhere else", to.display())); }
    } else if fa.is_dir() {
        let rd = std::fs::read_dir(from).map_err(|e| format!("Couldn't read {} ({e})", from.display()))?;
        for e in rd.flatten() { verify_entry(&e.path(), &to.join(e.file_name()), cancel, tick, count)?; }
    } else if fa.is_file() {
        if a.len() != b.len() { return Err(format!("{} has a different size than the original", to.display())); }
        same_bytes(from, to, a.len(), cancel, tick)?;
    } else { return Err(format!("{} is a special file", from.display())); }
    Ok(())
}

fn count_entries(p: &Path) -> u64 {
    let mut n = 0u64;
    let mut stack = vec![p.to_path_buf()];
    while let Some(x) = stack.pop() {
        n += 1;
        let Ok(m) = std::fs::symlink_metadata(&x) else { continue };
        if m.is_dir() && !m.file_type().is_symlink() { for e in std::fs::read_dir(&x).into_iter().flatten().flatten() { stack.push(e.path()); } }
    }
    n
}

/// Every file, folder and link under `from` must be in `to` with the same kind, size and bytes (the copy read back
/// from the drive), and the copy must have nothing extra.
pub fn verify_tree(from: &Path, to: &Path, cancel: &AtomicBool, tick: &mut dyn FnMut(u64)) -> Result<(), String> {
    unsafe { libc::sync(); }
    let mut n = 0u64;
    verify_entry(from, to, cancel, tick, &mut n)?;
    let m = count_entries(to);
    if m != n { return Err(format!("The copy has {m} items but the original has {n}")); }
    Ok(())
}

/// Where an item is copied to first: beside its final place, named so it never looks installed.
pub fn partial_of(to: &Path) -> PathBuf {
    let name = to.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    to.with_file_name(format!(".hearth-partial-{name}"))
}

// ------------------------------------------------------------------ Steam's records

/// libraryfolders.vdf with an app's line moved from one library's "apps" to another's (Steam keeps a list there
/// of what each library holds). Line by line, as Steam writes it; None when either library isn't found (the file
/// is then left as it is: Steam rebuilds the lists when it starts). `same`: whether a "path" value is that library.
pub fn move_app_entry(text: &str, app: u32, same_from: &dyn Fn(&str) -> bool, same_to: &dyn Fn(&str) -> bool) -> Option<String> {
    let key = format!("\"{app}\"");
    let lines: Vec<&str> = text.lines().collect();
    // First pass: which library block each line is in (depth 2), and where each "apps" block ends.
    let mut depth = 0i32;
    let mut lib_path: Option<String> = None;
    let mut in_apps = false;
    let mut apps_depth = 0;
    let mut cur_is: (bool, bool) = (false, false);
    let mut size: Option<String> = None;
    let mut drop_line: Option<usize> = None;
    let mut insert_at: Option<(usize, String)> = None;
    let mut found_from = false;
    let mut found_to = false;
    for (i, raw) in lines.iter().enumerate() {
        let l = raw.trim();
        if l == "{" { depth += 1; continue; }
        if l == "}" {
            if in_apps && depth == apps_depth {
                in_apps = false;
                if cur_is.1 && insert_at.is_none() {
                    let indent: String = raw.chars().take_while(|c| c.is_whitespace()).collect();
                    insert_at = Some((i, format!("{indent}\t")));
                }
            }
            depth -= 1;
            if depth <= 1 { lib_path = None; cur_is = (false, false); }
            continue;
        }
        let toks: Vec<&str> = l.split('"').collect();
        // "key" "value" → ["", key, ws, value, ""]; "key" → ["", key, ""]
        if toks.len() >= 5 && depth == 2 && toks[1].eq_ignore_ascii_case("path") {
            let p = toks[3].replace("\\\\", "\\");
            cur_is = (same_from(&p), same_to(&p));
            if cur_is.0 { found_from = true; }
            if cur_is.1 { found_to = true; }
            lib_path = Some(p);
            continue;
        }
        if toks.len() == 3 && depth == 2 && toks[1].eq_ignore_ascii_case("apps") { in_apps = true; apps_depth = depth + 1; continue; }
        if in_apps && depth == apps_depth && toks.len() >= 5 && l.starts_with(&key) && cur_is.0 && lib_path.is_some() {
            size = Some(toks[3].to_string());
            drop_line = Some(i);
        }
    }
    if !found_from || !found_to { return None; }
    let (at, indent) = insert_at?;
    let size = size.unwrap_or_else(|| "0".into());
    let mut out = String::new();
    for (i, raw) in lines.iter().enumerate() {
        if Some(i) == drop_line { continue; }
        if i == at { out.push_str(&format!("{indent}\"{app}\"\t\t\"{size}\"\n")); }
        out.push_str(raw);
        out.push('\n');
    }
    Some(out)
}

fn same_path(a: &str, b: &Path) -> bool {
    let a = a.trim_end_matches('/');
    if a == s(b).trim_end_matches('/') { return true; }
    std::fs::canonicalize(a).ok().zip(std::fs::canonicalize(b).ok()).map(|(x, y)| x == y).unwrap_or(false)
}

/// Every Steam record in the libraries: app → (library, StateFlags, name).
pub fn steam_apps(libs: &[PathBuf]) -> HashMap<u32, (PathBuf, i64, String)> {
    let mut out = HashMap::new();
    for lib in libs {
        for e in std::fs::read_dir(lib.join("steamapps")).into_iter().flatten().flatten() {
            let n = e.file_name().to_string_lossy().to_string();
            let Some(id) = n.strip_prefix("appmanifest_").and_then(|r| r.strip_suffix(".acf")).and_then(|r| r.parse::<u32>().ok()) else { continue };
            let kv = crate::vdf::load(&e.path());
            let flags = kv.as_ref().map(|k| k.num(&["AppState", "StateFlags"])).unwrap_or(0);
            let name = kv.as_ref().and_then(|k| k.str(&["AppState", "name"]).map(str::to_string)).unwrap_or_default();
            out.entry(id).or_insert((lib.clone(), flags, name));
        }
    }
    out
}

// ------------------------------------------------------------------ moving a game

#[derive(Debug, Clone, PartialEq)]
pub struct Item { pub from: PathBuf, pub to: PathBuf }

/// What a Steam game takes with it to another library: its folder, Proton prefix, shader cache and Workshop items.
pub fn steam_items(src_lib: &Path, dst_lib: &Path, app: u32, installdir: &str) -> Vec<Item> {
    let (a, b) = (src_lib.join("steamapps"), dst_lib.join("steamapps"));
    let mut v = vec![Item { from: a.join("common").join(installdir), to: b.join("common").join(installdir) }];
    for sub in ["compatdata", "shadercache", "workshop/content"] {
        let f = a.join(sub).join(app.to_string());
        if exists(&f) { v.push(Item { from: f, to: b.join(sub).join(app.to_string()) }); }
    }
    let ws = format!("appworkshop_{app}.acf");
    if exists(&a.join("workshop").join(&ws)) { v.push(Item { from: a.join("workshop").join(&ws), to: b.join("workshop").join(&ws) }); }
    v
}

/// The folders an item may be deleted from after a move (or undone into): a Steam library's parts, a games folder.
fn within_of(p: &Path) -> PathBuf { p.parent().map(Path::to_path_buf).unwrap_or_default() }

/// Where games of a store go on a drive: Hearth's install folder when it's on that drive, else <drive>/Games.
pub fn games_base(drive_mount: &Path, drive_dev: u64, install_root: &Path) -> PathBuf {
    if dev_of(install_root) == Some(drive_dev) { install_root.to_path_buf() } else { drive_mount.join("Games") }
}

#[derive(Debug, Clone)]
pub struct Plan {
    pub id: String, pub title: String, pub store: String, pub items: Vec<Item>, pub total: u64, pub to_name: String,
    /// Steam: (source library, destination library, app, make the destination a library first).
    pub steam: Option<(PathBuf, PathBuf, u32, bool)>,
    /// Epic: (app, new base folder). GOG: id. Recomps: id.
    pub epic: Option<(String, PathBuf)>, pub gog: Option<String>, pub recomp: Option<String>,
}

static JOB: Mutex<Option<Value>> = Mutex::new(None);
static MOVING: AtomicBool = AtomicBool::new(false);
static CANCEL: AtomicBool = AtomicBool::new(false);

fn job_state() -> Value { JOB.lock().unwrap_or_else(|p| p.into_inner()).clone().unwrap_or(Value::Null) }
fn set_job(v: Value) { *JOB.lock().unwrap_or_else(|p| p.into_inner()) = Some(v.clone()); emit(json!({"job": v})); }

pub fn cancel() -> Value {
    if MOVING.load(Ordering::SeqCst) { CANCEL.store(true, Ordering::SeqCst); log("move: stop asked"); }
    json!({"job": job_state()})
}

/// Something has the game's files open (a process whose program or folder is in them).
pub fn in_use(dir: &Path) -> bool {
    let Ok(rd) = std::fs::read_dir("/proc") else { return false };
    let real = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
    rd.flatten().any(|e| {
        if !e.file_name().to_string_lossy().chars().all(|c| c.is_ascii_digit()) { return false; }
        ["exe", "cwd"].iter().any(|k| std::fs::read_link(e.path().join(k)).map(|t| t.starts_with(&real)).unwrap_or(false))
    })
}

fn journal(data: &Path) -> PathBuf { data.join("diskspace-move.json") }
fn write_journal(data: &Path, p: &Plan, phase: &str) {
    let v = json!({"id": p.id, "title": p.title, "store": p.store, "phase": phase, "at": now(),
        "items": p.items.iter().map(|i| json!({"from": s(&i.from), "to": s(&i.to)})).collect::<Vec<_>>(),
        "steam": p.steam.as_ref().map(|(a, b, app, _)| json!({"src": s(a), "dst": s(b), "app": app})),
        "epic": p.epic.as_ref().map(|(app, _)| json!({"app": app})), "gog": p.gog, "recomp": p.recomp});
    let _ = crate::paths::write_atomic(&journal(data), v.to_string().as_bytes());
}

fn epic_path(app: &str) -> Option<String> {
    read_json(&crate::stores::epic::config().join("installed.json"))[app]["install_path"].as_str().map(|x| x.trim_end_matches(['/', '\\']).to_string())
}

/// Builds the move: what's copied where, and checks it can go (room, nothing in the way). Quick: on the bridge.
pub fn plan(ctx: &Ctx, id: &str, to_mount: &str) -> Result<Plan, String> {
    let g = ctx.games.iter().find(|g| g.id == id).ok_or("That game isn't installed anymore.")?.clone();
    move_support(&g)?;
    if ctx.running.contains(&g.id) { return Err(format!("Exit {} first.", g.title)); }
    if ctx.queued.contains(&g.id) { return Err(format!("{} is downloading. Move it when that's done.", g.title)); }
    let ds = drives(ctx);
    let dst = ds.iter().find(|d| s(&d.mount) == to_mount).ok_or("That drive isn't there anymore. Look again.")?;
    if dst.read_only { return Err(format!("{} can only be read, not written to.", dst.name)); }
    if dev_of(&g.path) == Some(dst.dev) { return Err(format!("{} is already on {}.", g.title, dst.name)); }
    let mut p = Plan { id: g.id.clone(), title: g.title.clone(), store: g.source.clone(), items: vec![], total: 0, to_name: dst.name.clone(),
        steam: None, epic: None, gog: None, recomp: None };
    match g.source.as_str() {
        "steam" => {
            let app: u32 = g.id.trim_start_matches("steam:").parse().map_err(|_| "That isn't a Steam game.")?;
            let src = steam_lib_of(&g.path).ok_or("Steam keeps it somewhere Hearth doesn't recognise.")?;
            let installdir = g.path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            if !crate::steam_ctl::safe_dir_name(&installdir) { return Err("Steam's record for it looks wrong. Check it in Steam first.".into()); }
            let kv = crate::vdf::load(&src.join("steamapps").join(format!("appmanifest_{app}.acf"))).ok_or("Steam's record for it is missing.")?;
            if crate::steam_ctl::writing(kv.num(&["AppState", "StateFlags"])) { return Err(format!("Steam is working on {} (downloading, updating or checking it). Try again when it's done.", g.title)); }
            if crate::steam_ctl::steam_game_running(Some(app)) { return Err(format!("Exit {} first.", g.title)); }
            let (dst_lib, make) = match dst.steam_libs.first() { Some(l) => (l.clone(), false), None => (dst.mount.join("SteamLibrary"), true) };
            if exists(&dst_lib.join("steamapps").join(format!("appmanifest_{app}.acf"))) { return Err(format!("Steam already has a record for it on {}.", dst.name)); }
            p.items = steam_items(&src, &dst_lib, app, &installdir);
            p.steam = Some((src, dst_lib, app, make));
        }
        "epic" => {
            let app = g.id.trim_start_matches("epic:").to_string();
            let rec = epic_path(&app).ok_or("Epic's records don't list it. Refresh the library.")?;
            if rec != s(&g.path).trim_end_matches('/') { return Err("Epic's records changed. Refresh the library.".into()); }
            let folder = g.path.file_name().ok_or("Its folder looks wrong.")?.to_owned();
            let base = games_base(&dst.mount, dst.dev, &ctx.install_root).join("Epic");
            p.items = vec![Item { from: g.path.clone(), to: base.join(folder) }];
            p.epic = Some((app, base));
        }
        "gog" => {
            let gid = g.id.trim_start_matches("gog:").to_string();
            let dir = PathBuf::from(ctx.gog.get(&gid).ok_or("Hearth didn't install this GOG game, so it can't move it.")?);
            if !dir.join(format!("goggame-{gid}.info")).exists() { return Err("Its folder doesn't look like the GOG game anymore.".into()); }
            let folder = dir.file_name().ok_or("Its folder looks wrong.")?.to_owned();
            let base = games_base(&dst.mount, dst.dev, &ctx.install_root).join("GOG");
            p.items = vec![Item { from: dir, to: base.join(folder) }];
            p.gog = Some(gid);
        }
        "recomps" => {
            let rid = crate::recomps::id_of(&g.id).ok_or("That isn't a Recomps port.")?.to_string();
            let from = crate::recomps::movable_dir(&rid)?;
            let to = if dev_of(&crate::recomps::root()) == Some(dst.dev) { crate::recomps::root().join(&rid).join("app") } else { dst.mount.join("Hearth Recomps").join(&rid) };
            p.items = vec![Item { from, to }];
            p.recomp = Some(rid);
        }
        _ => return Err("Hearth can't move this game.".into()),
    }
    for it in &p.items {
        if !exists(&it.from) { return Err(format!("{} is missing.", it.from.display())); }
        if exists(&it.to) { return Err(format!("There's already a {} on {}. Remove it first (or move the game there with its store).", it.to.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(), dst.name)); }
        if exists(&partial_of(&it.to)) { return Err(format!("An earlier move left an unfinished copy on {}. Open Clean up to remove it.", dst.name)); }
    }
    if in_use(&p.items[0].from) { return Err(format!("Something is still using {}'s files (is it running?). Close it first.", g.title)); }
    let total: u64 = p.items.iter().map(|i| { let t = tree_size(&i.from); t.bytes.max(t.apparent) }).sum();
    p.total = total;
    let need = total + GB;
    if dst.free < need { return Err(format!("Not enough space on {}: {} needs {:.1} GB, {:.1} GB is free.", dst.name, g.title, need as f64 / GB as f64, dst.free as f64 / GB as f64)); }
    Ok(p)
}

/// Starts the move in the background (events "diskspace" {job}). One at a time.
pub fn start_move(ctx: Ctx, id: &str, to_mount: &str, steam_on_demand: bool) -> Result<Value, String> {
    if MOVING.load(Ordering::SeqCst) { return Err("Hearth is already moving a game. Wait for it, or stop it.".into()); }
    if exists(&journal(&ctx.data)) { if let Some(m) = recover(&ctx) { log(&format!("move: {m}")); } }
    let p = plan(&ctx, id, to_mount)?;
    if MOVING.swap(true, Ordering::SeqCst) { return Err("Hearth is already moving a game.".into()); }
    CANCEL.store(false, Ordering::SeqCst);
    log(&format!("move: {} ({}) to {}: {} items, {} bytes", p.title, p.id, p.to_name, p.items.len(), p.total));
    for it in &p.items { log(&format!("move plan: {} -> {}", crate::steam::tilde(&ctx.home, &it.from), crate::steam::tilde(&ctx.home, &it.to))); }
    set_job(json!({"state": "moving", "id": p.id, "title": p.title, "to": p.to_name, "phase": "prepare", "done": 0, "total": p.total * 2, "bps": 0}));
    let v = job_state();
    std::thread::spawn(move || {
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run_move(&p, &ctx, steam_on_demand))).unwrap_or_else(|_| Err("Something went wrong in Hearth while moving.".into()));
        MOVING.store(false, Ordering::SeqCst);
        match r {
            Ok(()) => { log(&format!("move: {} is on {} now", p.title, p.to_name)); set_job(json!({"state": "done", "id": p.id, "title": p.title, "to": p.to_name, "freed": p.total})); }
            Err(e) => { log(&format!("move: {}: {e}", p.title)); set_job(json!({"state": if e == STOPPED { "stopped" } else { "error" }, "id": p.id, "title": p.title, "to": p.to_name, "error": e})); }
        }
        crate::stores::notify(json!({"type": "internal", "what": "rescan", "reply": null}));
        scan(ctx.clone());
    });
    Ok(json!({"job": v}))
}

fn run_move(p: &Plan, ctx: &Ctx, on_demand: bool) -> Result<(), String> {
    let mut steam_was = false;
    if let Some((_, dst_lib, _, make)) = &p.steam {
        // Steam rewrites its records as it closes, and could start writing into the game: it's closed for the move.
        crate::steam_engine::release();
        steam_was = crate::steam::is_running();
        if steam_was {
            if crate::steam_ctl::steam_game_running(None) { return Err("Exit the Steam game that's running first: Steam has to close to move a game.".into()); }
            set_job(json!({"state": "moving", "id": p.id, "title": p.title, "to": p.to_name, "phase": "steam", "done": 0, "total": p.total * 2}));
            crate::steam_ctl::shutdown(&ctx.home)?;
        }
        if *make {
            crate::steam::add_library(&ctx.home, dst_lib).map_err(|e| format!("Couldn't make a Steam library on {}: {e}", p.to_name))?;
        }
    }
    let r = move_files(p, ctx);
    if steam_was && !on_demand { let home = ctx.home.clone(); std::thread::spawn(move || crate::steam::ensure_running_silent(&home)); }
    r
}

fn move_files(p: &Plan, ctx: &Ctx) -> Result<(), String> {
    let guard = Guard::real(&ctx.home);
    let total = p.total * 2;
    let done = std::cell::Cell::new(0u64);
    let started = std::time::Instant::now();
    let last = std::cell::Cell::new(std::time::Instant::now() - Duration::from_secs(1));
    let report = |phase: &str, name: &str| {
        if last.get().elapsed() < Duration::from_millis(400) { return; }
        last.set(std::time::Instant::now());
        let secs = started.elapsed().as_secs_f64().max(0.5);
        set_job(json!({"state": "moving", "id": p.id, "title": p.title, "to": p.to_name, "phase": phase, "name": name, "done": done.get(), "total": total, "bps": (done.get() as f64 / secs) as u64}));
    };
    let undo_partials = |items: &[Item]| { for it in items { let pa = partial_of(&it.to); let _ = guard.remove(&pa, &within_of(&pa)); } };
    write_journal(&ctx.data, p, "copy");
    // 1. Copy each item beside its new place.
    for it in &p.items {
        let name = it.from.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let mut tick = |n: u64| { done.set(done.get() + n); report("copy", &name); };
        if let Err(e) = copy_tree(&it.from, &partial_of(&it.to), &CANCEL, &mut tick) { undo_partials(&p.items); let _ = std::fs::remove_file(journal(&ctx.data)); return Err(e); }
    }
    // 2. Check every byte, read back from the drive.
    done.set(p.total);
    for it in &p.items {
        let name = it.from.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let mut tick = |n: u64| { done.set(done.get() + n); report("verify", &name); };
        if let Err(e) = verify_tree(&it.from, &partial_of(&it.to), &CANCEL, &mut tick) {
            undo_partials(&p.items); let _ = std::fs::remove_file(journal(&ctx.data));
            return Err(if e == STOPPED { e } else { format!("The copy didn't match, so nothing changed: {e}") });
        }
    }
    if CANCEL.load(Ordering::SeqCst) { undo_partials(&p.items); let _ = std::fs::remove_file(journal(&ctx.data)); return Err(STOPPED.into()); }
    // 3. The copies take their real names (same drive: instant).
    set_job(json!({"state": "moving", "id": p.id, "title": p.title, "to": p.to_name, "phase": "switch", "done": total, "total": total}));
    for it in &p.items {
        if let Err(e) = std::fs::rename(partial_of(&it.to), &it.to) {
            for x in &p.items { let _ = guard.remove(&x.to, &within_of(&x.to)); }
            undo_partials(&p.items); let _ = std::fs::remove_file(journal(&ctx.data));
            return Err(format!("Couldn't put the copy in place ({e}). Nothing changed."));
        }
    }
    write_journal(&ctx.data, p, "placed");
    // 4. The store's record points at the new place. Not possible: the copies go, the original stays.
    if let Err(e) = register(p, ctx) {
        for x in &p.items { if let Err(r) = guard.remove(&x.to, &within_of(&x.to)) { log(&format!("move undo: {r}")); } }
        let _ = std::fs::remove_file(journal(&ctx.data));
        return Err(format!("{e} Nothing changed: {} is where it was.", p.title));
    }
    write_journal(&ctx.data, p, "registered");
    // 5. Only now the original goes.
    set_job(json!({"state": "moving", "id": p.id, "title": p.title, "to": p.to_name, "phase": "delete", "done": total, "total": total}));
    delete_sources(p, &guard);
    let _ = std::fs::remove_file(journal(&ctx.data));
    Ok(())
}

static GOG_ACK: Mutex<Option<std::sync::mpsc::Sender<Result<(), String>>>> = Mutex::new(None);
/// The bridge recorded a moved GOG game's new folder (library.json belongs to the bridge).
pub fn gog_ack(r: Result<(), String>) { if let Some(tx) = GOG_ACK.lock().unwrap_or_else(|p| p.into_inner()).take() { let _ = tx.send(r); } }

fn register(p: &Plan, _ctx: &Ctx) -> Result<(), String> {
    if let Some((src, dst, app, _)) = &p.steam {
        if crate::steam::is_running() { return Err("Steam was started during the move.".into()); }
        let from = src.join("steamapps").join(format!("appmanifest_{app}.acf"));
        let to = dst.join("steamapps").join(format!("appmanifest_{app}.acf"));
        let text = std::fs::read(&from).map_err(|e| format!("Couldn't read Steam's record ({e})."))?;
        if exists(&to) { return Err("Steam already has a record for it there.".into()); }
        crate::paths::write_atomic(&to, &text).map_err(|e| format!("Couldn't write Steam's record ({e})."))?;
        if let Err(e) = std::fs::rename(&from, from.with_extension("acf.hearth-moving")) { let _ = std::fs::remove_file(&to); return Err(format!("Couldn't change Steam's record ({e}).")); }
        // Steam's list of what each library holds (rebuilt by Steam if this doesn't fit).
        if let Some(steam) = crate::steam::steam_path(&crate::paths::home()) {
            let f = steam.join("steamapps/libraryfolders.vdf");
            if let Ok(t) = std::fs::read_to_string(&f) {
                let (a, b) = (src.clone(), dst.clone());
                if let Some(n) = move_app_entry(&t, *app, &|x: &str| same_path(x, &a), &|x: &str| same_path(x, &b)) { let _ = crate::paths::write_atomic(&f, n.as_bytes()); }
            }
        }
        return Ok(());
    }
    if let Some((app, base)) = &p.epic {
        let b = s(base);
        let r = crate::tools::run(&crate::tools::LEGENDARY, &["move", app.as_str(), b.as_str(), "--skip-move"], &crate::stores::epic::env(), None, None, Some(Duration::from_secs(120)))?;
        let want = s(&p.items[0].to);
        if epic_path(app).as_deref() != Some(want.trim_end_matches('/')) { return Err(format!("Epic (legendary) didn't take the new folder: {}.", crate::tools::problem(&r))); }
        return Ok(());
    }
    if let Some(gid) = &p.gog {
        let (tx, rx) = std::sync::mpsc::channel();
        *GOG_ACK.lock().unwrap_or_else(|x| x.into_inner()) = Some(tx);
        crate::stores::notify(json!({"type": "internal", "what": "diskspaceGog", "gid": gid, "dir": s(&p.items[0].to)}));
        return rx.recv_timeout(Duration::from_secs(60)).map_err(|_| "Hearth didn't record the new folder in time.".to_string())?;
    }
    if let Some(rid) = &p.recomp { return crate::recomps::relocate(rid, &p.items[0].to); }
    Err("Hearth can't record this move.".into())
}

fn delete_sources(p: &Plan, guard: &Guard) {
    for it in &p.items {
        log(&format!("move: deleting the original {}", it.from.display()));
        if let Err(e) = guard.remove(&it.from, &within_of(&it.from)) { log(&format!("move: {e}")); }
    }
    if let Some((src, _, app, _)) = &p.steam {
        let aside = src.join("steamapps").join(format!("appmanifest_{app}.acf.hearth-moving"));
        let _ = std::fs::remove_file(aside);
    }
}

/// A move that was cut off (power, crash): finished when the record already points at the copy, otherwise undone
/// (the copy removed, the original kept). Runs when the page is opened, never at start-up.
pub fn recover(ctx: &Ctx) -> Option<String> {
    let j = read_json(&journal(&ctx.data));
    if !j.is_object() { return None; }
    let title = j["title"].as_str().unwrap_or("A game").to_string();
    let items: Vec<Item> = j["items"].as_array().cloned().unwrap_or_default().iter().filter_map(|i| Some(Item { from: PathBuf::from(i["from"].as_str()?), to: PathBuf::from(i["to"].as_str()?) })).collect();
    let guard = Guard::real(&ctx.home);
    let first_to = items.first().map(|i| s(&i.to)).unwrap_or_default();
    let steam = &j["steam"];
    let (src_m, dst_m) = match (steam["src"].as_str(), steam["dst"].as_str(), steam["app"].as_u64()) {
        (Some(a), Some(b), Some(app)) => (Some(Path::new(a).join("steamapps").join(format!("appmanifest_{app}.acf"))), Some(Path::new(b).join("steamapps").join(format!("appmanifest_{app}.acf")))),
        _ => (None, None),
    };
    let registered = j["phase"] == "registered" || match j["store"].as_str().unwrap_or("") {
        "steam" => dst_m.as_deref().map(exists).unwrap_or(false) && !src_m.as_deref().map(exists).unwrap_or(true),
        "epic" => j["epic"]["app"].as_str().and_then(epic_path).as_deref() == Some(first_to.trim_end_matches('/')),
        "gog" => j["gog"].as_str().and_then(|g| ctx.gog.get(g)).map(|d| d == &first_to).unwrap_or(false),
        "recomps" => j["recomp"].as_str().and_then(crate::recomps::installed_state).and_then(|st| st["dir"].as_str().map(|d| d == first_to)).unwrap_or(false),
        _ => false,
    };
    let msg = if registered {
        for it in &items { if let Err(e) = guard.remove(&it.from, &within_of(&it.from)) { log(&format!("move recovery: {e}")); } }
        if let Some(sm) = &src_m { let _ = std::fs::remove_file(sm.with_extension("acf.hearth-moving")); }
        format!("Moving {title} was cut off near the end. Hearth finished it.")
    } else {
        for it in &items {
            let pa = partial_of(&it.to);
            if let Err(e) = guard.remove(&pa, &within_of(&pa)) { log(&format!("move recovery: {e}")); }
            if j["phase"] == "placed" { if let Err(e) = guard.remove(&it.to, &within_of(&it.to)) { log(&format!("move recovery: {e}")); } }
        }
        if let (Some(sm), Some(dm)) = (&src_m, &dst_m) {
            let aside = sm.with_extension("acf.hearth-moving");
            if exists(&aside) && !exists(sm) { let _ = std::fs::rename(&aside, sm); }
            if exists(sm) && exists(dm) { let _ = std::fs::remove_file(dm); }
        }
        format!("Moving {title} didn't finish. Its unfinished copy was removed; {title} is where it was.")
    };
    let _ = std::fs::remove_file(journal(&ctx.data));
    log(&format!("move recovery: {msg}"));
    Some(msg)
}

// ------------------------------------------------------------------ clean up

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Clean {
    pub key: String, pub kind: &'static str, pub title: String, pub path: PathBuf, pub within: PathBuf, pub bytes: u64,
    pub saves: Vec<String>, pub cloud: bool, pub selectable: bool, pub default_on: bool, pub confirm: bool, pub why: String,
}

/// The rules: (can be chosen, chosen at first, needs a second confirmation, why not / what to know).
/// A prefix whose game is installed is never offered; one with saves only when Steam Cloud has them, never chosen
/// at first and confirmed again with its saves listed.
pub fn rules(kind: &str, installed: bool, saves: &[String], cloud: bool) -> (bool, bool, bool, String) {
    if installed && kind == "prefix" { return (false, false, false, "Its game is installed".into()); }
    match kind {
        "prefix" if saves.is_empty() => (true, true, false, String::new()),
        "prefix" if cloud => (true, false, true, "Has saves; Steam Cloud has a copy".into()),
        "prefix" => (false, false, false, "Has saves that aren't backed up anywhere Hearth knows".into()),
        "tool" => (true, false, false, String::new()),
        _ => (true, true, false, String::new()),
    }
}

/// Saves found in a Wine prefix: folders with files in the usual save places (Documents, Saved Games, AppData),
/// leaving out what Windows and Proton put there themselves. Paths relative to the user folder, at most two levels.
pub fn find_saves(prefix: &Path) -> Vec<String> {
    let drive = [prefix.join("pfx/drive_c"), prefix.join("drive_c")].into_iter().find(|d| d.is_dir());
    let Some(drive) = drive else { return vec![] };
    let user = drive.join("users/steamuser");
    let noise = ["Microsoft", "Temp", "temp", "openvr", "D3DSCache", "CrashDumps", "cache", "Cache", "desktop.ini", "Packages", "Microsoft Games",
                 "My Music", "My Pictures", "My Videos", "Downloads", "Desktop", "Templates", "Start Menu", "NVIDIA", "Wine"];
    let mut out = Vec::new();
    for base in ["Documents", "Saved Games", "AppData/Roaming", "AppData/Local", "AppData/LocalLow"] {
        let dir = user.join(base);
        for e in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
            let n = e.file_name().to_string_lossy().to_string();
            let Ok(m) = std::fs::symlink_metadata(e.path()) else { continue };
            if m.file_type().is_symlink() || noise.contains(&n.as_str()) { continue; }
            if m.is_file() { if m.len() > 0 { out.push(format!("{base}/{n}")); } continue; }
            if tree_size(&e.path()).apparent == 0 { continue; }
            // One more level for the folders that hold many games' folders (Documents/My Games/<game>).
            let inner: Vec<String> = std::fs::read_dir(e.path()).into_iter().flatten().flatten()
                .filter(|x| x.file_type().map(|t| t.is_dir()).unwrap_or(false) && tree_size(&x.path()).apparent > 0)
                .map(|x| format!("{base}/{n}/{}", x.file_name().to_string_lossy())).collect();
            if n == "My Games" && !inner.is_empty() { out.extend(inner); } else { out.push(format!("{base}/{n}")); }
        }
    }
    out.sort();
    out.truncate(12);
    out
}

/// Steam Cloud has saves for this game (a remote folder with files under any account's userdata).
pub fn steam_cloud(steam: &Path, app: u32) -> bool {
    std::fs::read_dir(steam.join("userdata")).into_iter().flatten().flatten().any(|u| {
        let r = u.path().join(app.to_string()).join("remote");
        tree_size(&r).apparent > 0
    })
}

/// "GE-Proton9-20" → ("GE-Proton", [9, 20]): a tool's family and its version numbers, to keep the newest of each.
pub fn tool_version(name: &str) -> (String, Vec<u64>) {
    let fam: String = name.chars().take_while(|c| !c.is_ascii_digit()).collect();
    let fam = fam.trim_end_matches(['-', '_', ' ', '.']).to_string();
    let nums = name[fam.len().min(name.len())..].split(|c: char| !c.is_ascii_digit()).filter(|x| !x.is_empty()).filter_map(|x| x.parse().ok()).collect();
    (fam, nums)
}

/// Steam's compatibility tool choices (config.vdf CompatToolMapping): the tools' names.
pub fn tools_in_use(config_vdf: &str) -> HashSet<String> {
    let kv = crate::vdf::parse(config_vdf);
    let mut out = HashSet::new();
    if let Some(m) = kv.obj(&["InstallConfigStore", "Software", "Valve", "Steam", "CompatToolMapping"]) {
        for (_, e) in m { if let Some(n) = e.str(&["name"]) { if !n.is_empty() { out.insert(n.to_string()); } } }
    }
    out
}

/// The names a tool folder answers to (compatibilitytool.vdf's compat_tools keys), and its folder name.
fn tool_names(dir: &Path) -> Vec<String> {
    let mut v = vec![dir.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()];
    if let Some(kv) = crate::vdf::load(&dir.join("compatibilitytool.vdf")) {
        if let Some(t) = kv.obj(&["compatibilitytools", "compat_tools"]) { for (k, _) in t { v.push(k.clone()); } }
    }
    v
}

fn compat_roots(home: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = crate::steam::steam_dirs(home).into_iter().map(|(p, _)| p.join("compatibilitytools.d")).collect();
    v.push(home.join(".steam/root/compatibilitytools.d"));
    let mut seen = HashSet::new();
    v.into_iter().filter(|p| p.is_dir() && seen.insert(std::fs::canonicalize(p).unwrap_or(p.clone()))).collect()
}

/// Unused compatibility tools: not chosen for any game in Steam or in Hearth's Game settings, and not the newest of
/// their family (the newest GE-Proton, UMU-Proton... always stays). (folder, family's newest name).
pub fn unused_tools(tools: &[(PathBuf, Vec<String>)], used: &HashSet<String>, picked: &[String]) -> Vec<PathBuf> {
    let mut newest: HashMap<String, (Vec<u64>, PathBuf)> = HashMap::new();
    for (p, names) in tools {
        let (fam, ver) = tool_version(&names[0]);
        let e = newest.entry(fam).or_insert((ver.clone(), p.clone()));
        if ver > e.0 { *e = (ver, p.clone()); }
    }
    tools.iter().filter(|(p, names)| {
        let (fam, _) = tool_version(&names[0]);
        let keep_newest = newest.get(&fam).map(|(_, n)| n == p).unwrap_or(false);
        let chosen = names.iter().any(|n| used.contains(n)) || picked.iter().any(|x| Path::new(x) == p.as_path() || names.contains(x));
        !keep_newest && !chosen
    }).map(|(p, _)| p.clone()).collect()
}

fn add_clean(out: &mut Vec<Clean>, key: String, kind: &'static str, title: String, path: PathBuf, installed: bool, saves: Vec<String>, cloud: bool) {
    let (selectable, default_on, confirm, why) = rules(kind, installed, &saves, cloud);
    let within = within_of(&path);
    let bytes = tree_size(&path).bytes;
    if bytes == 0 && kind != "prefix" { return; }
    out.push(Clean { key, kind, title, path, within, bytes, saves, cloud, selectable, default_on, confirm, why });
}

/// Everything that could be cleaned up, with sizes (slow: off the bridge).
pub fn clean_candidates(ctx: &Ctx) -> Vec<Clean> {
    let mut out = Vec::new();
    let apps = steam_apps(&ctx.libs);
    let title_of = |app: u32| apps.get(&app).map(|x| x.2.clone()).filter(|n| !n.is_empty())
        .or_else(|| ctx.names.get(&format!("steam:{app}")).cloned()).unwrap_or_else(|| format!("Steam game {app}"));
    for lib in &ctx.libs {
        let sa = lib.join("steamapps");
        // Steam's shader caches: rebuilt by Steam when the game next starts.
        for e in std::fs::read_dir(sa.join("shadercache")).into_iter().flatten().flatten() {
            let Some(app) = e.file_name().to_string_lossy().parse::<u32>().ok() else { continue };
            add_clean(&mut out, format!("shader:{}", s(&e.path())), "shader", title_of(app), e.path(), false, vec![], false);
        }
        // Proton prefixes of games that aren't installed (Steam's own; 0 is Steam's default one, and numbers above
        // 2^31 are games you added to Steam yourself, which Hearth can't tell are gone).
        for e in std::fs::read_dir(sa.join("compatdata")).into_iter().flatten().flatten() {
            let Some(app) = e.file_name().to_string_lossy().parse::<u32>().ok() else { continue };
            if app == 0 || app >= 0x8000_0000 { continue; }
            let installed = apps.contains_key(&app);
            if installed { continue; }
            let cloud = ctx.steam.as_deref().map(|st| steam_cloud(st, app)).unwrap_or(false);
            add_clean(&mut out, format!("prefix:{}", s(&e.path())), "prefix", title_of(app), e.path(), false, find_saves(&e.path()), cloud);
        }
        // Unfinished downloads of games that aren't downloading (no record, or installed with nothing waiting).
        for part in ["downloading", "temp"] {
            for e in std::fs::read_dir(sa.join(part)).into_iter().flatten().flatten() {
                let Some(app) = e.file_name().to_string_lossy().parse::<u32>().ok() else { continue };
                if let Some((_, flags, _)) = apps.get(&app) { if *flags != 4 { continue; } }
                add_clean(&mut out, format!("dl:{}", s(&e.path())), "download", title_of(app), e.path(), false, vec![], false);
            }
        }
    }
    // Hearth's own prefixes (Epic, GOG) of games that aren't installed.
    let epic: HashSet<String> = read_json(&crate::stores::epic::config().join("installed.json")).as_object().map(|o| o.keys().map(|k| hearth_prefix(&ctx.data, "epic", k)).map(|p| s(&p)).collect()).unwrap_or_default();
    let gog: HashSet<String> = ctx.gog.keys().map(|k| s(&hearth_prefix(&ctx.data, "gog", k))).collect();
    for e in std::fs::read_dir(ctx.data.join("prefixes")).into_iter().flatten().flatten() {
        let n = e.file_name().to_string_lossy().to_string();
        let p = s(&e.path());
        let (store, id) = match n.split_once('-') { Some(x) => x, None => continue };
        let installed = match store { "epic" => epic.contains(&p), "gog" => gog.contains(&p), _ => continue };
        if installed { continue; }
        let title = ctx.names.get(&format!("{store}:{id}")).cloned().unwrap_or_else(|| format!("{} game {id}", store_label(store)));
        add_clean(&mut out, format!("prefix:{p}"), "prefix", title, e.path(), false, find_saves(&e.path()), false);
    }
    // Mesa's and Nvidia's shader caches.
    for c in gpu_caches(&ctx.home) {
        if !exists(&c) { continue; }
        let t = if s(&c).contains("nvidia") || s(&c).contains(".nv/") { "Nvidia shader cache" } else { "Mesa shader cache" };
        let t = if s(&c).contains(".var/app") { format!("{t} (Steam Flatpak)") } else { t.to_string() };
        add_clean(&mut out, format!("gpu:{}", s(&c)), "gpu", t, c, false, vec![], false);
    }
    // Old Proton and other compatibility tool versions nothing uses.
    let mut used = HashSet::new();
    for (d, _) in crate::steam::steam_dirs(&ctx.home) { used.extend(tools_in_use(&std::fs::read_to_string(d.join("config/config.vdf")).unwrap_or_default())); }
    let mut tools = Vec::new();
    for r in compat_roots(&ctx.home) {
        for e in std::fs::read_dir(&r).into_iter().flatten().flatten() {
            let p = e.path();
            if !p.is_dir() || !(p.join("proton").exists() || p.join("compatibilitytool.vdf").exists()) { continue; }
            tools.push((p.clone(), tool_names(&p)));
        }
    }
    for p in unused_tools(&tools, &used, &ctx.protons) {
        let n = p.file_name().map(|x| x.to_string_lossy().to_string()).unwrap_or_default();
        add_clean(&mut out, format!("tool:{}", s(&p)), "tool", n, p, false, vec![], false);
    }
    // Unfinished copies and leftovers from moves and imports.
    if !MOVING.load(Ordering::SeqCst) {
        let mut places: Vec<PathBuf> = ctx.libs.iter().map(|l| l.join("steamapps/common")).collect();
        for d in drives(ctx) { let b = games_base(&d.mount, d.dev, &ctx.install_root); places.push(b.join("Epic")); places.push(b.join("GOG")); }
        let mut seen = HashSet::new();
        for pl in places.into_iter().filter(|p| seen.insert(p.clone())) {
            for e in std::fs::read_dir(&pl).into_iter().flatten().flatten() {
                let n = e.file_name().to_string_lossy().to_string();
                if !(n.starts_with(".hearth-partial-") || n.starts_with(".hearth-moved-")) { continue; }
                add_clean(&mut out, format!("left:{}", s(&e.path())), "leftover", n.trim_start_matches(".hearth-partial-").trim_start_matches(".hearth-moved-").to_string(), e.path(), false, vec![], false);
            }
        }
    }
    out.sort_by(|a, b| b.bytes.cmp(&a.bytes));
    out
}

fn clean_dto(c: &Clean, home: &Path) -> Value {
    json!({"key": c.key, "kind": c.kind, "title": c.title, "where": crate::steam::tilde(home, &c.path), "bytes": c.bytes, "saves": c.saves, "cloud": c.cloud,
           "selectable": c.selectable, "on": c.default_on, "confirm": c.confirm, "why": c.why})
}

pub fn clean_list(ctx: &Ctx) -> Value {
    let list = clean_candidates(ctx);
    json!({"items": list.iter().map(|c| clean_dto(c, &ctx.home)).collect::<Vec<_>>()})
}

/// Which of the chosen keys may go now: (to delete, refused with why). Pure, so the never-delete cases are tested.
pub fn choose<'a>(list: &'a [Clean], keys: &[String], confirmed: &[String]) -> (Vec<&'a Clean>, Vec<(String, String)>) {
    let mut go = Vec::new();
    let mut no = Vec::new();
    for k in keys {
        let Some(c) = list.iter().find(|c| &c.key == k) else { no.push((k.clone(), "It changed since the list was made. Look again.".into())); continue };
        if !c.selectable { no.push((c.title.clone(), c.why.clone())); continue; }
        if c.confirm && !confirmed.contains(k) { no.push((c.title.clone(), "Its saves weren't confirmed".into())); continue; }
        go.push(c);
    }
    (go, no)
}

/// Deletes what was chosen (and confirmed): listed again first, each delete written to the log before it's done.
pub fn clean(ctx: &Ctx, keys: &[String], confirmed: &[String]) -> Result<Value, String> {
    if !ctx.running.is_empty() || crate::steam_ctl::steam_game_running(None) { return Err("Exit the game that's running first.".into()); }
    if MOVING.load(Ordering::SeqCst) { return Err("Wait until the move is done.".into()); }
    let list = clean_candidates(ctx);
    let (go, no) = choose(&list, keys, confirmed);
    let guard = Guard::real(&ctx.home);
    for c in &go { log(&format!("clean (dry run): {} {} · {} bytes", c.kind, crate::steam::tilde(&ctx.home, &c.path), c.bytes)); }
    let mut freed = 0u64;
    let mut done = Vec::new();
    let mut failed: Vec<Value> = no.iter().map(|(t, w)| json!({"title": t, "why": w})).collect();
    for c in go {
        match guard.remove(&c.path, &c.within) {
            Ok(()) => { log(&format!("clean: deleted {}", crate::steam::tilde(&ctx.home, &c.path))); freed += c.bytes; done.push(json!(c.title)); }
            Err(e) => { log(&format!("clean: {e}")); failed.push(json!({"title": c.title, "why": e})); }
        }
    }
    scan(ctx.clone());
    Ok(json!({"freed": freed, "done": done, "failed": failed}))
}

// ------------------------------------------------------------------ Send logs

/// For Send logs: the drives and their space, Steam's libraries, the biggest games (by folder name only), and
/// any move or warning state. No other file names.
pub fn report() -> String {
    let home = crate::paths::home();
    let data = crate::paths::data_dir();
    let steam = crate::steam::steam_path(&home);
    let libs = steam.as_ref().map(|st| crate::steam::libraries(st)).unwrap_or_default();
    let lib = read_json(&data.join("library.json"));
    let root = lib["installRoot"].as_str().filter(|x| !x.is_empty()).map(PathBuf::from).unwrap_or_else(|| home.join("Games"));
    let ctx = Ctx { home: home.clone(), data: data.clone(), steam, libs: libs.clone(), install_root: root, ..Default::default() };
    let mut out = String::new();
    for d in drives(&ctx) {
        out.push_str(&format!("{} [{}] {} ({}{}): {:.1} GB, {:.1} GB used, {:.1} GB free\n", d.name, d.kind, crate::steam::tilde(&home, &d.mount), d.fstype,
            if d.read_only { ", read-only" } else { "" }, d.size as f64 / 1e9, d.used as f64 / 1e9, d.free as f64 / 1e9));
    }
    out.push_str(&format!("Steam libraries: {}\n", libs.iter().map(|l| crate::steam::tilde(&home, l)).collect::<Vec<_>>().join(", ")));
    let (on, gb) = warn_settings(&lib);
    out.push_str(&format!("Low space warning: {}\n", if on { format!("on, below {gb} GB") } else { "off".into() }));
    let cache = read_json(&cache_file(&data));
    if let Some(g) = cache["games"].as_object() {
        let mut v: Vec<(String, u64, u64, u64)> = g.values().map(|x| (Path::new(x["path"].as_str().unwrap_or("")).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
            x["size"].as_u64().unwrap_or(0), x["prefix"].as_u64().unwrap_or(0), x["shader"].as_u64().unwrap_or(0))).collect();
        v.sort_by(|a, b| b.1.cmp(&a.1));
        out.push_str(&format!("Sizes from {} min ago, {} games; biggest:\n", now().saturating_sub(cache["at"].as_u64().unwrap_or(0)) / 60_000, v.len()));
        for (f, a, b, c) in v.iter().take(25) { out.push_str(&format!("  {f}: {:.1} GB (prefix {:.1}, shader cache {:.1})\n", *a as f64 / 1e9, *b as f64 / 1e9, *c as f64 / 1e9)); }
    }
    let j = read_json(&journal(&data));
    if j.is_object() { out.push_str(&format!("Unfinished move: {} ({}, phase {})\n", j["title"], j["store"], j["phase"])); }
    if let Some(job) = JOB.lock().ok().and_then(|g| g.clone()) { out.push_str(&format!("Last move: {} {} → {} {}\n", job["title"], job["state"], job["to"], job["error"].as_str().unwrap_or(""))); }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(p: &Path, bytes: &[u8]) { std::fs::create_dir_all(p.parent().unwrap()).unwrap(); std::fs::write(p, bytes).unwrap(); }
    fn test_guard(home: &Path) -> Guard { Guard { home: home.to_path_buf(), system: ["/usr", "/etc", "/boot", "/sysroot", "/ostree", "/proc", "/sys", "/dev"].iter().map(PathBuf::from).collect() } }

    #[test]
    fn adds_up_folders_without_following_links() {
        let d = tempfile::tempdir().unwrap();
        w(&d.path().join("g/a.bin"), &[1u8; 1000]);
        w(&d.path().join("g/sub/b.bin"), &[2u8; 24]);
        w(&d.path().join("outside/big.bin"), &[3u8; 5000]);
        std::os::unix::fs::symlink(d.path().join("outside"), d.path().join("g/link")).unwrap();
        std::fs::hard_link(d.path().join("g/a.bin"), d.path().join("g/sub/a2.bin")).unwrap();
        let t = tree_size(&d.path().join("g"));
        assert_eq!(t.apparent, 1024, "the link's target isn't counted, the hard link once");
        assert!(t.bytes >= 1024 || t.bytes == 0);
        assert_eq!(tree_size(&d.path().join("missing")), TreeSize::default());
        assert!(!sizable(Path::new("/home/me"), Path::new("/home/me")));
        assert!(!sizable(Path::new("/home"), Path::new("/home/me")));
        assert!(sizable(Path::new("/home/me/Games/x"), Path::new("/home/me")));
    }

    #[test]
    fn reads_drives_from_df() {
        let text = "Filesystem     Type     1B-blocks         Used        Avail Mounted on\n\
            /dev/nvme0n1p3 btrfs 500000000000 200000000000 300000000000 /var\n\
            /dev/nvme0n1p3 btrfs 500000000000 200000000000 300000000000 /var\n\
            /dev/mmcblk0p1 ext4 256000000000 10000000000 246000000000 /var/mnt/games-SD Card\n\
            /dev/sda1 exfat 1000000000000 1000000 999999000000 /run/media/me/Big Drive\n\
            composefs overlay 30000000 30000000 0 /\n\
            tmpfs tmpfs 8000000000 0 8000000000 /tmp\n";
        let rows = parse_df(text);
        assert_eq!(rows.len(), 6);
        assert_eq!(rows[2].mount, "/var/mnt/games-SD Card");
        let devs: HashMap<String, u64> = [("/var", 1u64), ("/var/mnt/games-SD Card", 2), ("/run/media/me/Big Drive", 3), ("/", 4), ("/tmp", 5)].iter().map(|(a, b)| (a.to_string(), *b)).collect();
        let ro: HashSet<String> = ["/run/media/me/Big Drive".to_string()].into_iter().collect();
        let libs = vec![(PathBuf::from("/var/mnt/games-SD Card/SteamLibrary"), 2u64), (PathBuf::from("/var/home/me/.local/share/Steam"), 1)];
        let ds = drives_from(&rows, Some(1), &devs, &ro, &libs);
        assert_eq!(ds.iter().map(|d| (d.kind, d.name.as_str())).collect::<Vec<_>>(), vec![("system", "This device"), ("sd", "SD card (SD Card)"), ("usb", "Big Drive")]);
        assert_eq!(ds[1].steam_libs, vec![PathBuf::from("/var/mnt/games-SD Card/SteamLibrary")]);
        assert!(ds[2].read_only && !ds[0].read_only);
        assert_eq!(kind_of("/dev/sdb1", "/var/mnt/games-Games", false), "games");
        assert_eq!(kind_of("/dev/sdb1", "/data", false), "extra");
        let m = read_only_mounts("/dev/sda1 /run/media/me/Big\\040Drive exfat ro,nosuid 0 0\n/dev/nvme0n1p3 /var btrfs rw,relatime 0 0\n");
        assert!(m.contains("/run/media/me/Big Drive") && !m.contains("/var"));
    }

    #[test]
    fn finds_each_stores_parts() {
        let data = Path::new("/home/me/.local/share/nimbus");
        let g = GameRef { id: "steam:620".into(), title: "Portal 2".into(), source: "steam".into(), path: "/mnt/sd/SteamLibrary/steamapps/common/Portal 2".into(), shared: false };
        let (p, sh) = parts_of(&g, data);
        assert_eq!(p.unwrap(), PathBuf::from("/mnt/sd/SteamLibrary/steamapps/compatdata/620"));
        assert_eq!(sh.unwrap(), PathBuf::from("/mnt/sd/SteamLibrary/steamapps/shadercache/620"));
        let e = GameRef { id: "epic:Fortnite Thing".into(), source: "epic".into(), path: "/home/me/Games/Epic/F".into(), ..Default::default() };
        assert_eq!(parts_of(&e, data).0.unwrap(), data.join("prefixes/epic-Fortnite_Thing"));
        assert_eq!(hearth_prefix(&crate::paths::data_dir(), "gog", "12 3"), crate::stores::prefix_of("gog", "12 3"));
        assert_eq!(steam_lib_of(Path::new("/x/steamapps/common/G")), Some(PathBuf::from("/x")));
        assert_eq!(steam_lib_of(Path::new("/x/Games/G")), None);
        // What can move, and what is explained instead.
        for (src, ok) in [("epic", true), ("gog", true), ("recomps", true), ("battlenet", false), ("xodus", false), ("beacon", false), ("emu", false), ("manual", false)] {
            let g = GameRef { id: format!("{src}:1"), source: src.into(), path: "/a/b/c".into(), ..Default::default() };
            assert_eq!(move_support(&g).is_ok(), ok, "{src}");
        }
        assert!(move_support(&GameRef { shared: true, ..g.clone() }).is_err());
        assert!(move_support(&g).is_ok());
        assert_eq!(uninstall_support(&g), Ok("game"));
        assert_eq!(uninstall_support(&GameRef { source: "recomps".into(), ..Default::default() }), Ok("recomp"));
        assert!(uninstall_support(&GameRef { source: "battlenet".into(), ..Default::default() }).is_err());
    }

    #[test]
    fn copies_and_checks_every_byte() {
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("Game");
        w(&src.join("game.exe"), b"MZ program");
        w(&src.join("data/level1.pak"), &vec![7u8; 3 * 1024 * 1024 + 17]);
        std::fs::create_dir_all(src.join("empty")).unwrap();
        std::os::unix::fs::symlink("data/level1.pak", src.join("link.pak")).unwrap();
        std::fs::set_permissions(src.join("game.exe"), std::fs::Permissions::from_mode(0o755)).unwrap();
        let to = d.path().join("drive2/Games/Epic/Game");
        let pa = partial_of(&to);
        assert_eq!(pa.file_name().unwrap(), ".hearth-partial-Game");
        let cancel = AtomicBool::new(false);
        let mut copied = 0u64;
        copy_tree(&src, &pa, &cancel, &mut |n| copied += n).unwrap();
        assert_eq!(copied, 10 + 3 * 1024 * 1024 + 17);
        let mut checked = 0u64;
        verify_tree(&src, &pa, &cancel, &mut |n| checked += n).unwrap();
        assert_eq!(checked, copied);
        assert_eq!(std::fs::read_link(pa.join("link.pak")).unwrap(), PathBuf::from("data/level1.pak"));
        assert_eq!(std::fs::metadata(pa.join("game.exe")).unwrap().mode() & 0o777, 0o755);
        // A copy already there is never overwritten.
        assert!(copy_tree(&src, &pa, &cancel, &mut |_| {}).is_err());
        // One byte different, a file missing, or something extra: the check fails.
        let f = pa.join("data/level1.pak");
        let mut b = std::fs::read(&f).unwrap(); b[2_000_000] ^= 1; std::fs::write(&f, &b).unwrap();
        assert!(verify_tree(&src, &pa, &cancel, &mut |_| {}).unwrap_err().contains("doesn't match"));
        b[2_000_000] ^= 1; std::fs::write(&f, &b).unwrap();
        w(&pa.join("extra.txt"), b"x");
        assert!(verify_tree(&src, &pa, &cancel, &mut |_| {}).unwrap_err().contains("items"));
        std::fs::remove_file(pa.join("extra.txt")).unwrap();
        std::fs::remove_file(pa.join("game.exe")).unwrap();
        assert!(verify_tree(&src, &pa, &cancel, &mut |_| {}).unwrap_err().contains("missing"));
        // Stop: the copy stops, the caller removes the unfinished one.
        let stop = AtomicBool::new(true);
        let to2 = d.path().join("drive2/Other");
        assert_eq!(copy_tree(&src, &to2, &stop, &mut |_| {}).unwrap_err(), STOPPED);
        // Pipes and devices are refused.
        let fifo = d.path().join("pipe");
        let c = std::ffi::CString::new(fifo.to_string_lossy().as_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(c.as_ptr(), 0o644) }, 0);
        assert!(copy_tree(&fifo, &d.path().join("pipe2"), &cancel, &mut |_| {}).unwrap_err().contains("special"));
    }

    #[test]
    fn deletes_only_known_places() {
        let d = tempfile::tempdir().unwrap();
        let home = d.path().join("home/me");
        let lib = home.join(".local/share/Steam/steamapps");
        let g = test_guard(&home);
        w(&lib.join("shadercache/620/a"), b"x");
        w(&home.join("Documents/keep.txt"), b"keep");
        // Inside the place it was found: fine.
        g.remove(&lib.join("shadercache/620"), &lib.join("shadercache")).unwrap();
        assert!(!lib.join("shadercache/620").exists());
        // The place itself, its parent, outside it, home and system areas: refused.
        w(&lib.join("shadercache/730/a"), b"x");
        assert!(g.remove(&lib.join("shadercache"), &lib.join("shadercache")).is_err());
        assert!(g.remove(&home.join("Documents"), &lib.join("shadercache")).is_err());
        assert!(g.remove(&lib.join("shadercache/../../../../../Documents"), &lib.join("shadercache")).is_err());
        assert!(g.remove(&home, &home.parent().unwrap().to_path_buf()).is_err());
        assert!(g.forbidden(Path::new("/usr/share/steam")) && g.forbidden(Path::new("/sysroot/ostree/deploy")) && g.forbidden(Path::new("/etc/x")));
        assert!(g.forbidden(Path::new("/run/user/1000/x")) && !g.forbidden(Path::new("/run/media/me/SD/Games/x")));
        assert!(g.forbidden(Path::new("relative/x")) && g.forbidden(Path::new("/x")));
        assert!(home.join("Documents/keep.txt").exists());
        // A link in the list is removed, its target kept; a folder that leads elsewhere through a link is refused.
        std::os::unix::fs::symlink(home.join("Documents"), lib.join("shadercache/999")).unwrap();
        g.remove(&lib.join("shadercache/999"), &lib.join("shadercache")).unwrap();
        assert!(home.join("Documents/keep.txt").exists());
        std::os::unix::fs::symlink(home.join("Documents"), lib.join("shadercache/lnk")).unwrap();
        assert!(g.remove(&lib.join("shadercache/lnk/keep.txt"), &lib.join("shadercache")).is_err());
        assert!(home.join("Documents/keep.txt").exists(), "a delete never reaches through a link");
        // Real guard: system areas.
        let real = Guard::real(Path::new("/home/me"));
        assert!(real.forbidden(Path::new("/var/lib/flatpak/app")) && real.forbidden(Path::new("/usr/lib/x")) && !real.forbidden(Path::new("/var/home/me/Games/x")));
        assert!(SYSTEM_ROOTS.contains(&"/tmp") && SYSTEM_ROOTS.contains(&"/sysroot") && SYSTEM_ROOTS.contains(&"/ostree"));
    }

    #[test]
    fn cleanup_never_deletes_what_matters() {
        // Installed game: never. Saves without a cloud copy: never. Saves with one: chosen only with a confirmation.
        assert_eq!(rules("prefix", true, &[], false).0, false);
        assert_eq!(rules("prefix", false, &["Documents/My Games/Skyrim".into()], false), (false, false, false, "Has saves that aren't backed up anywhere Hearth knows".into()));
        assert_eq!(rules("prefix", false, &["AppData/LocalLow/Team Cherry".into()], true), (true, false, true, "Has saves; Steam Cloud has a copy".into()));
        assert_eq!(rules("prefix", false, &[], false), (true, true, false, String::new()));
        assert_eq!(rules("shader", false, &[], false).1, true);
        assert_eq!(rules("tool", false, &[], false).1, false);
        let mk = |key: &str, kind: &'static str, saves: Vec<String>, cloud: bool| {
            let (selectable, default_on, confirm, why) = rules(kind, false, &saves, cloud);
            Clean { key: key.into(), kind, title: key.into(), saves, cloud, selectable, default_on, confirm, why, ..Default::default() }
        };
        let list = vec![mk("a", "shader", vec![], false), mk("b", "prefix", vec!["Documents/x".into()], false), mk("c", "prefix", vec!["Documents/y".into()], true)];
        let keys: Vec<String> = ["a", "b", "c", "gone"].iter().map(|x| x.to_string()).collect();
        let (go, no) = choose(&list, &keys, &[]);
        assert_eq!(go.iter().map(|c| c.key.as_str()).collect::<Vec<_>>(), vec!["a"]);
        assert_eq!(no.len(), 3);
        let (go, _) = choose(&list, &keys, &["c".to_string(), "b".to_string()]);
        assert_eq!(go.iter().map(|c| c.key.as_str()).collect::<Vec<_>>(), vec!["a", "c"], "b has saves without a copy: never, even confirmed");
    }

    #[test]
    fn finds_saves_in_prefixes() {
        let d = tempfile::tempdir().unwrap();
        let u = d.path().join("pfx/drive_c/users/steamuser");
        w(&u.join("Documents/My Games/Skyrim/Saves/save1.ess"), b"save");
        w(&u.join("AppData/LocalLow/Team Cherry/Hollow Knight/user1.dat"), b"save");
        w(&u.join("AppData/Local/Microsoft/x"), b"noise");
        w(&u.join("AppData/Roaming/Empty/.keep"), b"");
        std::fs::create_dir_all(u.join("Saved Games")).unwrap();
        assert_eq!(find_saves(d.path()), vec!["AppData/LocalLow/Team Cherry".to_string(), "Documents/My Games/Skyrim".to_string()]);
        assert!(find_saves(&d.path().join("nothing")).is_empty());
    }

    #[test]
    fn keeps_proton_versions_in_use_and_the_newest() {
        assert_eq!(tool_version("GE-Proton9-20"), ("GE-Proton".to_string(), vec![9, 20]));
        assert_eq!(tool_version("UMU-Proton-9.0-3.2"), ("UMU-Proton".to_string(), vec![9, 0, 3, 2]));
        let t = |n: &str| (PathBuf::from(format!("/c/{n}")), vec![n.to_string()]);
        let tools = vec![t("GE-Proton8-25"), t("GE-Proton9-20"), t("GE-Proton9-5"), t("UMU-Proton-9.0-3.2"), t("Proton-Old7"), t("Luxtorpeda")];
        let used: HashSet<String> = ["GE-Proton8-25".to_string()].into_iter().collect();
        let mut gone = unused_tools(&tools, &used, &["/c/Proton-Old7".to_string()]);
        gone.sort();
        assert_eq!(gone, vec![PathBuf::from("/c/GE-Proton9-5")]);
        let cfg = "\"InstallConfigStore\"\n{\n\"Software\"\n{\n\"Valve\"\n{\n\"Steam\"\n{\n\"CompatToolMapping\"\n{\n\"0\"\n{\n\"name\"\t\"proton_9\"\n}\n\"620\"\n{\n\"name\"\t\"GE-Proton8-25\"\n}\n}\n}\n}\n}\n}\n";
        let u = tools_in_use(cfg);
        assert!(u.contains("GE-Proton8-25") && u.contains("proton_9"));
    }

    #[test]
    fn moves_an_apps_line_between_libraries() {
        let vdf = "\"libraryfolders\"\n{\n\t\"0\"\n\t{\n\t\t\"path\"\t\t\"/home/me/.local/share/Steam\"\n\t\t\"label\"\t\t\"\"\n\t\t\"apps\"\n\t\t{\n\t\t\t\"228980\"\t\t\"29212173\"\n\t\t\t\"620\"\t\t\"12000000000\"\n\t\t}\n\t}\n\t\"1\"\n\t{\n\t\t\"path\"\t\t\"/var/mnt/games-SD/SteamLibrary\"\n\t\t\"apps\"\n\t\t{\n\t\t}\n\t}\n}\n";
        let out = move_app_entry(vdf, 620, &|p: &str| p == "/home/me/.local/share/Steam", &|p: &str| p == "/var/mnt/games-SD/SteamLibrary").unwrap();
        let kv = crate::vdf::parse(&out);
        assert!(kv.str(&["libraryfolders", "0", "apps", "620"]).is_none());
        assert_eq!(kv.str(&["libraryfolders", "0", "apps", "228980"]), Some("29212173"));
        assert_eq!(kv.str(&["libraryfolders", "1", "apps", "620"]), Some("12000000000"));
        assert!(move_app_entry(vdf, 620, &|p: &str| p == "/nowhere", &|p: &str| p == "/var/mnt/games-SD/SteamLibrary").is_none());
        // What a move takes along.
        let d = tempfile::tempdir().unwrap();
        let (a, b) = (d.path().join("a"), d.path().join("b"));
        w(&a.join("steamapps/common/Portal 2/x"), b"x");
        w(&a.join("steamapps/compatdata/620/pfx/y"), b"y");
        w(&a.join("steamapps/workshop/appworkshop_620.acf"), b"w");
        let items = steam_items(&a, &b, 620, "Portal 2");
        assert_eq!(items.len(), 3);
        assert_eq!(items[1].to, b.join("steamapps/compatdata/620"));
        assert_eq!(items[2].to, b.join("steamapps/workshop/appworkshop_620.acf"));
    }

    #[test]
    fn warns_below_the_threshold() {
        assert_eq!(warn_settings(&json!({})), (true, 15));
        assert_eq!(warn_settings(&json!({"lowSpace": {"on": false, "gb": 40}})), (false, 40));
        assert_eq!(warn_settings(&json!({"lowSpace": {"gb": 9999}})), (true, 15));
        let ds = vec![("This device".to_string(), 9 * GB, "/var".to_string(), true), ("SD".to_string(), 100 * GB, "/sd".to_string(), true), ("USB".to_string(), GB, "/usb".to_string(), false)];
        let low = low_from(&ds, 15);
        assert_eq!(low.len(), 1);
        assert_eq!(low[0]["name"], "This device");
    }

    #[test]
    fn a_cut_off_move_is_undone_or_finished() {
        let d = tempfile::tempdir().unwrap();
        let home = d.path().join("home/me");
        let data = home.join(".local/share/nimbus");
        let src = home.join("Games/Epic/Game");
        let dst = d.path().join("media/sd/Games/Epic/Game");
        w(&src.join("a"), b"a");
        w(&partial_of(&dst).join("a"), b"a");
        let p = Plan { id: "manual:1".into(), title: "Game".into(), store: "manual".into(), items: vec![Item { from: src.clone(), to: dst.clone() }], total: 1, to_name: "SD".into(),
            steam: None, epic: None, gog: None, recomp: None };
        write_journal(&data, &p, "copy");
        let ctx = Ctx { home: home.clone(), data: data.clone(), ..Default::default() };
        // Not switched over yet: the unfinished copy goes, the original stays.
        let m = recover(&ctx).unwrap();
        assert!(m.contains("didn't finish"), "{m}");
        assert!(src.join("a").exists() && !partial_of(&dst).exists() && !journal(&data).exists());
        // Switched over: the original goes.
        w(&dst.join("a"), b"a");
        write_journal(&data, &p, "registered");
        assert!(recover(&ctx).unwrap().contains("finished"));
        assert!(!src.exists() && dst.join("a").exists());
        assert!(recover(&ctx).is_none());
    }
}
