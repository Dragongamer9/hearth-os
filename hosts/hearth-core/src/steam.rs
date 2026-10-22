//! Installed Steam games: libraryfolders.vdf, then appmanifest_*.acf in every library. Play time and last
//! played come from the most recent user's localconfig.vdf. Art: custom grid images, then Steam's library
//! cache, then Steam's CDN. Port of library/steam.js.

use crate::vdf::{self, Kv};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

const STEAM_ID64_BASE: u64 = 76561197960265728;
/// Runtimes and tools that show up as apps but aren't games.
const SKIP_APP_IDS: &[&str] = &["228980", "250820", "1070560", "1391110", "1628350", "1493710", "1826330", "2180100", "2348590"];
const SKIP_PREFIXES: &[&str] = &["steamworks", "proton", "steam linux runtime", "steamvr"];
const FULLY_INSTALLED: i64 = 4;
const UPDATE_PAUSED: i64 = 512;
const BUSY: i64 = 256 | 1024 | 524288 | 1048576 | 2097152 | 4194304;
pub const CDN: &str = "https://cdn.cloudflare.steamstatic.com/steam/apps/";
const FLATPAK_ID: &str = "com.valvesoftware.Steam";

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SteamGame {
    pub app_id: String,
    pub title: String,
    pub install_path: PathBuf,
    pub size_bytes: Option<u64>,
    pub added: u64,
    pub minutes: u64,
    pub last_played: u64,
    pub installed: bool,
    pub state: &'static str,
    pub bytes_done: Option<u64>,
    pub bytes_total: Option<u64>,
    pub art: Vec<(&'static str, String)>,
}

pub fn candidates(home: &Path) -> Vec<PathBuf> {
    vec![
        home.join(".local/share/Steam"),
        home.join(".steam/steam"),
        home.join(".steam/root"),
        home.join(".var/app").join(FLATPAK_ID).join(".local/share/Steam"),
        home.join(".var/app").join(FLATPAK_ID).join("data/Steam"),
    ]
}

fn looks_like_steam(p: &Path) -> bool {
    p.join("steamapps").is_dir() || p.join("config/loginusers.vdf").exists() || p.join("steam.sh").exists()
}

/// Where Steam keeps its data (native or Flatpak), if it's here.
pub fn steam_path(home: &Path) -> Option<PathBuf> {
    candidates(home).into_iter().find(|p| looks_like_steam(p)).map(|p| std::fs::canonicalize(&p).unwrap_or(p))
}

pub fn libraries(steam: &Path) -> Vec<PathBuf> {
    let mut out = vec![steam.to_path_buf()];
    let mut seen: HashSet<PathBuf> = out.iter().cloned().collect();
    if let Some(kv) = vdf::load(&steam.join("steamapps/libraryfolders.vdf")) {
        if let Some(items) = kv.obj(&["libraryfolders"]) {
            for (_, entry) in items {
                // New format: "0" { "path" "/mnt/games" ... }   Old format: "1" "/mnt/games"
                let p = match entry { Kv::Obj(_) => entry.str(&["path"]).map(str::to_string), Kv::Str(s) => Some(s.clone()) };
                let Some(p) = p else { continue };
                let Ok(real) = std::fs::canonicalize(&p) else { continue };
                if real.is_dir() && seen.insert(real.clone()) { out.push(real); }
            }
        }
    }
    out
}

// ------------------------------------------------------------------ adding a library (Hearth Disks' "Use for games")

fn vdf_quote(s: &str) -> String { s.replace('\\', "\\\\").replace('"', "\\\"") }

/// One library entry in libraryfolders.vdf's own layout (what Steam writes for a new library).
fn library_entry(key: u64, path: &str, contentid: u64) -> String {
    format!("\t\"{key}\"\n\t{{\n\t\t\"path\"\t\t\"{}\"\n\t\t\"label\"\t\t\"\"\n\t\t\"contentid\"\t\t\"{contentid}\"\n\t\t\"totalsize\"\t\t\"0\"\n\t\t\"update_clean_bytes_tally\"\t\t\"0\"\n\t\t\"time_last_update_verified\"\t\t\"0\"\n\t\t\"apps\"\n\t\t{{\n\t\t}}\n\t}}\n", vdf_quote(path))
}

/// libraryfolders.vdf with one more library at `path`, the rest kept as it was. None when it's already there.
/// No file yet (Steam never ran with libraries): a new one with Steam's own folder first, as Steam makes it.
pub fn with_library(existing: Option<&str>, steam: &Path, path: &Path, contentid: u64) -> Option<String> {
    let want = path.to_string_lossy().trim_end_matches('/').to_string();
    let Some(text) = existing.filter(|t| t.contains('}')) else {
        return Some(format!("\"libraryfolders\"\n{{\n{}{}}}\n", library_entry(0, &steam.to_string_lossy(), contentid ^ 1), library_entry(1, &want, contentid)));
    };
    let kv = vdf::parse(text);
    let mut next = 0u64;
    if let Some(items) = kv.obj(&["libraryfolders"]) {
        for (key, entry) in items {
            let p = match entry { Kv::Obj(_) => entry.str(&["path"]).map(str::to_string), Kv::Str(s) => Some(s.clone()) };
            if p.map(|p| p.trim_end_matches('/') == want).unwrap_or(false) { return None; }
            if let Ok(n) = key.parse::<u64>() { next = next.max(n + 1); }
        }
    }
    // The new entry goes in before the file's last closing brace (the end of "libraryfolders").
    let end = text.rfind('}')?;
    let mut out = text[..end].trim_end_matches(['\t', ' ']).to_string();
    if !out.ends_with('\n') { out.push('\n'); }
    out.push_str(&library_entry(next, &want, contentid));
    out.push_str(&text[end..]);
    Some(out)
}

/// Makes `folder` a Steam library: its steamapps folder and libraryfolder.vdf, then an entry in Steam's
/// libraryfolders.vdf. Steam rewrites that file when it closes, so a running Steam is closed first and started
/// again (hidden) afterwards. Returns whether Steam was restarted. Blocks up to ~40 s: call it off the UI thread.
pub fn add_library(home: &Path, folder: &Path) -> Result<bool, String> {
    let steam = steam_path(home).ok_or("Steam isn't set up yet. Open Steam once, then try again.")?;
    std::fs::create_dir_all(folder.join("steamapps")).map_err(|e| format!("Couldn't make the games folder there ({e})."))?;
    let mut id = [0u8; 8];
    let _ = getrandom::getrandom(&mut id);
    let contentid = u64::from_le_bytes(id) >> 1;
    let marker = folder.join("libraryfolder.vdf");
    if !marker.exists() {
        let text = format!("\"libraryfolder\"\n{{\n\t\"contentid\"\t\t\"{contentid}\"\n\t\"label\"\t\t\"\"\n}}\n");
        std::fs::write(&marker, text).map_err(|e| format!("Couldn't write to the drive ({e})."))?;
    }
    let file = steam.join("steamapps/libraryfolders.vdf");
    let existing = std::fs::read_to_string(&file).ok();
    let Some(text) = with_library(existing.as_deref(), &steam, folder, contentid) else { return Ok(false) };
    let was_running = is_running();
    if was_running {
        let _ = spawn(home, "-shutdown");
        for _ in 0..60 { if !is_running() { break; } std::thread::sleep(std::time::Duration::from_millis(500)); }
        if is_running() { return Err("Steam didn't close. Close Steam, then try again.".into()); }
    }
    crate::paths::write_atomic(&file, text.as_bytes()).map_err(|e| format!("Couldn't update Steam's list of libraries ({e})."))?;
    if was_running { let _ = spawn(home, "-silent"); }
    Ok(was_running)
}

/// The signed-in (most recent) Steam user's folder: userdata/<account id>.
pub fn account_dir(steam: &Path) -> Option<PathBuf> {
    let userdata = steam.join("userdata");
    if !userdata.is_dir() { return None; }
    if let Some(users) = vdf::load(&steam.join("config/loginusers.vdf")).and_then(|k| k.obj(&["users"]).cloned()) {
        for (id64, u) in &users {
            if u.str(&["MostRecent"]) != Some("1") { continue; }
            if let Ok(n) = id64.parse::<u64>() { let dir = userdata.join((n.saturating_sub(STEAM_ID64_BASE)).to_string()); if dir.is_dir() { return Some(dir); } }
        }
    }
    std::fs::read_dir(&userdata).ok().into_iter().flatten().flatten()
        .filter_map(|e| { let cfg = e.path().join("config/localconfig.vdf"); cfg.metadata().and_then(|m| m.modified()).ok().map(|t| (t, e.path())) })
        .max_by_key(|(t, _)| *t).map(|(_, p)| p)
}
pub const ID64_BASE: u64 = STEAM_ID64_BASE;

/// The signed-in Steam account's SteamID64, when this Steam has more than one account (with one, every game is its).
/// Each Hearth profile signs Steam in as its own account (profiles.rs), so this is the profile's account.
/// `Some(0)`: no account is marked to sign in (a profile that hasn't signed in to Steam yet): none of the installed
/// games are its own.
pub fn signed_in_id64(steam: &Path) -> Option<u64> {
    let users = std::fs::read_dir(steam.join("userdata")).ok()?.flatten()
        .filter(|e| e.file_name().to_string_lossy().parse::<u64>().map(|n| n > 0).unwrap_or(false)).count();
    if users == 0 { return None; }
    let login = vdf::load(&steam.join("config/loginusers.vdf"))?;
    let recent = login.obj(&["users"]).and_then(|u| u.iter().find(|(_, x)| x.str(&["MostRecent"]) == Some("1")).and_then(|(id, _)| id.parse::<u64>().ok()));
    match recent {
        Some(_) if users < 2 => None,
        Some(id) => Some(id),
        None => Some(0),
    }
}

/// Whether an installed Steam game belongs in the signed-in account's library: everything when there's one account;
/// otherwise games that account installed (the manifest's LastOwner), or has played or launched (its localconfig
/// lists it), or that no account is named for. A game another profile's account installed stays out of yours.
pub fn mine(me: Option<u64>, owner: u64, in_my_config: bool) -> bool {
    match me { None => true, Some(0) => false, Some(m) => owner == 0 || owner == m || in_my_config }
}

/// The most recent Steam user's folder (userdata/<account id>): its grid art and play stats.
pub fn user_data(steam: &Path) -> (Option<PathBuf>, HashMap<String, (u64, u64)>) {
    let mut stats = HashMap::new();
    let userdata = steam.join("userdata");
    if !userdata.is_dir() { return (None, stats); }
    let mut account: Option<PathBuf> = None;
    if let Some(users) = vdf::load(&steam.join("config/loginusers.vdf")).and_then(|k| k.obj(&["users"]).cloned()) {
        for (id64, u) in &users {
            if u.str(&["MostRecent"]) != Some("1") { continue; }
            if let Ok(n) = id64.parse::<u64>() {
                let dir = userdata.join((n.saturating_sub(STEAM_ID64_BASE)).to_string());
                if dir.is_dir() { account = Some(dir); }
            }
        }
    }
    if account.is_none() {
        // The one whose settings changed last.
        account = std::fs::read_dir(&userdata).ok().into_iter().flatten().flatten()
            .filter_map(|e| { let cfg = e.path().join("config/localconfig.vdf"); cfg.metadata().and_then(|m| m.modified()).ok().map(|t| (t, e.path())) })
            .max_by_key(|(t, _)| *t).map(|(_, p)| p);
    }
    let Some(account) = account else { return (None, stats) };
    if let Some(local) = vdf::load(&account.join("config/localconfig.vdf")) {
        if let Some(apps) = local.obj(&["UserLocalConfigStore", "Software", "Valve", "Steam", "apps"]) {
            for (id, a) in apps {
                let last = a.num(&["LastPlayed"]).max(0) as u64;
                stats.insert(id.clone(), (a.num(&["Playtime"]).max(0) as u64, last * 1000));
            }
        }
    }
    let grid = account.join("config/grid");
    (if grid.is_dir() { Some(grid) } else { None }, stats)
}

fn find_in_tree(dir: &Path, test: &dyn Fn(&str) -> bool, depth: u32) -> Option<PathBuf> {
    let entries: Vec<_> = std::fs::read_dir(dir).ok()?.flatten().collect();
    for e in &entries {
        let name = e.file_name().to_string_lossy().to_string();
        if test(&name) && !e.path().is_dir() { return Some(e.path()); }
    }
    if depth == 0 { return None; }
    for e in &entries { if e.path().is_dir() { if let Some(hit) = find_in_tree(&e.path(), test, depth - 1) { return Some(hit); } } }
    None
}

/// Art for a game: (kind, file path or URL) for cover, hero, logo and wide.
pub fn find_art(app_id: &str, grid: Option<&Path>, cache: &Path) -> Vec<(&'static str, String)> {
    let mut art = Vec::new();
    let kinds: [(&str, String, String, &[(&str, &str)], &str); 4] = [
        ("cover", format!("{app_id}p"), format!("{app_id}_library_600x900.jpg"), &[("library_600x900", ".jpg"), ("library_capsule", ".jpg")], "library_600x900.jpg"),
        ("hero", format!("{app_id}_hero"), format!("{app_id}_library_hero.jpg"), &[("library_hero", ".jpg")], "library_hero.jpg"),
        ("logo", format!("{app_id}_logo"), format!("{app_id}_logo.png"), &[("logo", ".png")], "logo.png"),
        ("wide", app_id.to_string(), format!("{app_id}_header.jpg"), &[("header", ".jpg"), ("library_header", ".jpg")], "header.jpg"),
    ];
    'kind: for (kind, grid_name, legacy, patterns, cdn) in kinds.iter() {
        if let Some(g) = grid {
            for ext in [".png", ".jpg", ".jpeg", ".webp"] {
                let f = g.join(format!("{grid_name}{ext}"));
                if f.exists() { art.push((*kind, f.to_string_lossy().into())); continue 'kind; }
            }
        }
        let l = cache.join(legacy);
        if l.exists() { art.push((*kind, l.to_string_lossy().into())); continue; }
        let dir = cache.join(app_id);
        if dir.is_dir() {
            for (start, end) in patterns.iter() {
                if let Some(f) = find_in_tree(&dir, &|n: &str| n.starts_with(start) && n.ends_with(end), 2) { art.push((*kind, f.to_string_lossy().into())); continue 'kind; }
            }
        }
        art.push((*kind, format!("{CDN}{app_id}/{cdn}")));   // needs internet; the UI shows a gradient without it
    }
    art
}

/// What the library shows for a manifest's StateFlags: installed, updating, paused, downloading or queued.
pub fn app_state(flags: i64) -> (bool, &'static str) {
    let installed = flags & FULLY_INSTALLED != 0;
    let state = if installed { if flags & BUSY != 0 && flags & UPDATE_PAUSED == 0 { "updating" } else { "installed" } }
        else if flags & UPDATE_PAUSED != 0 { "paused" } else if flags & BUSY != 0 { "downloading" } else { "queued" };
    (installed, state)
}

pub fn skip(app_id: &str, name: &str) -> bool {
    let low = name.to_lowercase();
    SKIP_APP_IDS.contains(&app_id) || SKIP_PREFIXES.iter().any(|p| low.starts_with(p))
}

fn millis(t: std::io::Result<std::time::SystemTime>) -> u64 {
    t.ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_millis() as u64).unwrap_or(0)
}

/// Installed games, and games Steam is installing, downloading or updating.
/// Installed Steam games. Only with more than one Hearth profile are they narrowed to the signed-in account's
/// (each profile has its own Steam account); with one profile every installed game shows, as before 0.2.3.
pub fn scan(home: &Path) -> Vec<SteamGame> { scan_with(home, several_profiles()) }

/// More than one Hearth profile (profiles.json in Hearth's data folder).
fn several_profiles() -> bool {
    let f = crate::paths::data_dir().join("profiles.json");
    std::fs::read(f).ok().and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
        .and_then(|v| v["list"].as_array().map(|l| l.len() > 1)).unwrap_or(false)
}

pub fn scan_with(home: &Path, per_account: bool) -> Vec<SteamGame> {
    let mut games = Vec::new();
    let Some(steam) = steam_path(home) else { return games };
    let (grid, stats) = user_data(&steam);
    let me = if per_account { signed_in_id64(&steam) } else { None };
    let cache = steam.join("appcache/librarycache");
    for lib in libraries(&steam) {
        let apps = lib.join("steamapps");
        let Ok(rd) = std::fs::read_dir(&apps) else { continue };
        for e in rd.flatten() {
            let fname = e.file_name().to_string_lossy().to_string();
            if !(fname.starts_with("appmanifest_") && fname.ends_with(".acf")) { continue; }
            let Some(kv) = vdf::load(&e.path()) else { continue };
            let app_id = kv.str(&["AppState", "appid"]).unwrap_or("").to_string();
            let name = kv.str(&["AppState", "name"]).unwrap_or("").trim().to_string();
            let dir_name = kv.str(&["AppState", "installdir"]).unwrap_or("").to_string();
            if app_id.is_empty() || name.is_empty() || dir_name.is_empty() || skip(&app_id, &name) { continue; }
            let owner = kv.str(&["AppState", "LastOwner"]).and_then(|o| o.parse::<u64>().ok()).unwrap_or(0);
            if !mine(me, owner, stats.contains_key(&app_id)) { continue; }
            let dir = apps.join("common").join(&dir_name);
            let flags = kv.num(&["AppState", "StateFlags"]);
            let size = kv.num(&["AppState", "SizeOnDisk"]).max(0) as u64;
            let to_dl = kv.num(&["AppState", "BytesToDownload"]).max(0) as u64;
            let dl = kv.num(&["AppState", "BytesDownloaded"]).max(0) as u64;
            let (installed, state) = app_state(flags);
            let (minutes, last) = stats.get(&app_id).cloned().unwrap_or((0, 0));
            let manifest = e.path();
            let meta = std::fs::metadata(if dir.is_dir() { &dir } else { &manifest }).ok();
            let added = meta.map(|m| { let b = millis(m.created()); if b > 0 { b } else { millis(m.modified()) } }).unwrap_or(0);
            games.push(SteamGame {
                art: find_art(&app_id, grid.as_deref(), &cache),
                app_id, title: name, install_path: dir,
                size_bytes: if size > 0 { Some(size) } else { None }, added, minutes, last_played: last, installed, state,
                bytes_done: if to_dl > 0 { Some(dl) } else { None }, bytes_total: if to_dl > 0 { Some(to_dl) } else { None },
            });
        }
    }
    games
}

#[cfg(test)]
mod tests {
    use super::*;
    fn put(p: &Path, text: &str) { std::fs::create_dir_all(p.parent().unwrap()).unwrap(); std::fs::write(p, text).unwrap(); }

    #[test]
    fn scans_a_steam_folder() {
        let home = tempfile::tempdir().unwrap();
        let steam = home.path().join(".local/share/Steam");
        let other = home.path().join("games");
        put(&steam.join("steamapps/libraryfolders.vdf"), &format!(r#""libraryfolders" {{ "0" {{ "path" "{}" }} "1" {{ "path" "{}" }} }}"#, steam.display(), other.display()));
        put(&steam.join("steamapps/appmanifest_620.acf"), r#""AppState" { "appid" "620" "name" "Portal 2" "installdir" "Portal 2" "StateFlags" "4" "SizeOnDisk" "12000" }"#);
        put(&steam.join("steamapps/appmanifest_1628350.acf"), r#""AppState" { "appid" "1628350" "name" "Steam Linux Runtime 3.0" "installdir" "SLR" "StateFlags" "4" }"#);
        put(&other.join("steamapps/appmanifest_400.acf"), r#""AppState" { "appid" "400" "name" "Portal" "installdir" "Portal" "StateFlags" "1026" "BytesToDownload" "100" "BytesDownloaded" "25" }"#);
        put(&steam.join("config/loginusers.vdf"), r#""users" { "76561197960265729" { "AccountName" "sam" "MostRecent" "1" } }"#);
        put(&steam.join("userdata/1/config/localconfig.vdf"), r#""UserLocalConfigStore" { "Software" { "Valve" { "Steam" { "apps" { "620" { "Playtime" "90" "LastPlayed" "1700000000" } } } } } }"#);
        put(&steam.join("userdata/1/config/grid/620p.png"), "png");
        let mut g = scan(home.path());
        g.sort_by(|a, b| a.app_id.cmp(&b.app_id));
        assert_eq!(g.len(), 2, "the runtime isn't a game");
        assert_eq!((g[0].title.as_str(), g[0].state, g[0].bytes_total), ("Portal", "downloading", Some(100)));
        let p2 = &g[1];
        assert_eq!((p2.installed, p2.minutes, p2.last_played, p2.size_bytes), (true, 90, 1_700_000_000_000, Some(12000)));
        assert!(p2.art.iter().any(|(k, v)| *k == "cover" && v.ends_with("grid/620p.png")));
        assert!(p2.art.iter().any(|(k, v)| *k == "hero" && v.starts_with("https://")));
        // One account: its games all show, whoever installed them.
        put(&steam.join("steamapps/appmanifest_70.acf"), r#""AppState" { "appid" "70" "name" "Half-Life" "installdir" "Half-Life" "StateFlags" "4" "LastOwner" "76561197960265730" }"#);
        assert_eq!(scan_with(home.path(), true).len(), 3);
        // Two accounts (two profiles): another account's install isn't sam's; sam's own and what sam has played are.
        put(&steam.join("userdata/2/config/localconfig.vdf"), r#""UserLocalConfigStore" { }"#);
        let ids: Vec<String> = scan_with(home.path(), true).into_iter().map(|g| g.app_id).collect();
        // One Hearth profile: every installed game, whoever installed it.
        assert_eq!(scan_with(home.path(), false).len(), 3);
        assert!(!ids.contains(&"70".to_string()) && ids.contains(&"620".to_string()) && ids.contains(&"400".to_string()), "{ids:?}");
        assert_eq!(signed_in_id64(&steam), Some(76561197960265729));
        // A profile that hasn't signed in yet: no account marked, none of these are its games.
        put(&steam.join("config/loginusers.vdf"), r#""users" { "76561197960265729" { "AccountName" "sam" "MostRecent" "0" } }"#);
        assert_eq!(signed_in_id64(&steam), Some(0));
        assert!(scan_with(home.path(), true).is_empty());
        assert_eq!(scan_with(home.path(), false).len(), 3, "a single profile still sees them");
    }
    #[test]
    fn whose_games() {
        assert!(mine(None, 5, false));
        assert!(mine(Some(5), 5, false) && mine(Some(5), 0, false) && mine(Some(5), 6, true));
        assert!(!mine(Some(5), 6, false));
        assert!(!mine(Some(0), 0, true), "nobody signed in");
    }
}

// ------------------------------------------------------------------ running Steam (port of steam.js)

fn flatpak_installed(home: &Path) -> bool {
    Path::new("/var/lib/flatpak/app").join(FLATPAK_ID).is_dir() || home.join(".local/share/flatpak/app").join(FLATPAK_ID).is_dir()
}

/// How to hand Steam a steam:// URL or an option (-silent): the native client, the Flatpak's, or the desktop's
/// URL handler. The Flatpak when that's where Steam keeps its data (or the only Steam there is).
pub fn command(home: &Path, arg: &str) -> (String, Vec<String>) {
    let flatpak_data = steam_path(home).map(|p| p.to_string_lossy().contains(&format!(".var/app/{FLATPAK_ID}"))).unwrap_or(false);
    let native = crate::system::has("steam");
    let flatpak = crate::system::has("flatpak") && (flatpak_data || flatpak_installed(home));
    if native && !flatpak_data { return ("steam".into(), vec![arg.into()]); }
    if flatpak { return ("flatpak".into(), vec!["run".into(), FLATPAK_ID.into(), arg.into()]); }
    if native { return ("steam".into(), vec![arg.into()]); }
    ("xdg-open".into(), vec![arg.into()])
}

/// Is Steam here at all (data, the native client or the Flatpak)?
pub fn client_installed(home: &Path) -> bool {
    steam_path(home).is_some() || crate::system::has("steam") || flatpak_installed(home)
}

/// Hands Steam a URL or option, detached (no output kept).
pub fn spawn(home: &Path, arg: &str) -> Result<(), String> {
    let (cmd, args) = command(home, arg);
    std::process::Command::new(&cmd).args(&args).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null())
        .spawn().map(|c| { crate::procs::detach(c); }).map_err(|e| format!("Couldn't start Steam: {e}"))
}

/// Is the Steam client running (native or the Flatpak's)?
pub fn is_running() -> bool {
    let Ok(dir) = std::fs::read_dir("/proc") else { return false };
    dir.flatten().any(|e| {
        let name = e.file_name();
        let n = name.to_string_lossy();
        n.chars().all(|c| c.is_ascii_digit()) && matches!(std::fs::read_to_string(e.path().join("comm")).unwrap_or_default().trim(), "steam" | "steamwebhelper")
    })
}

/// Starts Steam hidden (-silent: no window, only its background part) if it isn't running, and waits until it's
/// up, so a game started next comes straight up instead of Steam's update and main windows. Blocks up to ~35 s:
/// call it off the UI thread.
pub fn ensure_running_silent(home: &Path) {
    crate::steam_ctl::touch();
    if is_running() || !client_installed(home) { return; }
    crate::bridge::log("steam: starting it in the background (-silent)");
    if spawn(home, "-silent").is_err() { return; }
    // Started by Hearth: Hearth may close it again once nothing needs it (steam_ctl, "Start Steam only when needed").
    crate::steam_ctl::started_by_hearth();
    for _ in 0..60 {
        std::thread::sleep(std::time::Duration::from_millis(500));
        if is_running() { std::thread::sleep(std::time::Duration::from_secs(3)); return; }
    }
}

#[cfg(test)]
mod library_tests {
    use super::*;
    #[test]
    fn adds_a_library_once() {
        let steam = Path::new("/home/sam/.local/share/Steam");
        let old = "\"libraryfolders\"\n{\n\t\"0\"\n\t{\n\t\t\"path\"\t\t\"/home/sam/.local/share/Steam\"\n\t}\n\t\"3\"\n\t{\n\t\t\"path\"\t\t\"/mnt/old\"\n\t}\n}\n";
        let new = with_library(Some(old), steam, Path::new("/var/mnt/games-sd/SteamLibrary"), 42).unwrap();
        let kv = vdf::parse(&new);
        assert_eq!(kv.str(&["libraryfolders", "4", "path"]), Some("/var/mnt/games-sd/SteamLibrary"));
        assert_eq!(kv.str(&["libraryfolders", "3", "path"]), Some("/mnt/old"));
        assert_eq!(kv.str(&["libraryfolders", "4", "contentid"]), Some("42"));
        assert!(with_library(Some(&new), steam, Path::new("/var/mnt/games-sd/SteamLibrary/"), 7).is_none());
        let fresh = with_library(None, steam, Path::new("/run/media/sam/SD"), 9).unwrap();
        let kv = vdf::parse(&fresh);
        assert_eq!(kv.str(&["libraryfolders", "0", "path"]), Some("/home/sam/.local/share/Steam"));
        assert_eq!(kv.str(&["libraryfolders", "1", "path"]), Some("/run/media/sam/SD"));
    }
}

#[cfg(test)]
mod run_tests {
    use super::*;
    #[test]
    fn hands_steam_its_argument() {
        let home = std::env::temp_dir().join(format!("hearth-steam-cmd-{}", std::process::id()));
        let (cmd, args) = command(&home, "steam://rungameid/10");
        assert!(["steam", "flatpak", "xdg-open"].contains(&cmd.as_str()));
        assert_eq!(args.last().map(String::as_str), Some("steam://rungameid/10"));
    }
}
