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

/// Steam Flatpaks whose folders Hearth knows: Flathub's, and Anatase's own (hearth-steam-launch starts either).
pub const FLATPAK_IDS: &[&str] = &[FLATPAK_ID, "org.anatase.Steam"];

/// Every place Steam keeps its data. Native Steam: ~/.local/share/Steam (Valve's launcher, RPM Fusion's package),
/// ~/.steam/debian-installation (Debian's and Ubuntu's steam-installer: Peppermint, and a home folder brought
/// from those), and ~/.steam/steam and ~/.steam/root, the links Steam's launcher keeps to whichever one it uses.
/// Then the Flatpaks' own.
pub fn candidates(home: &Path) -> Vec<PathBuf> {
    let mut v = vec![
        home.join(".local/share/Steam"),
        home.join(".steam/steam"),
        home.join(".steam/root"),
        home.join(".steam/debian-installation"),
    ];
    for id in FLATPAK_IDS {
        v.push(home.join(".var/app").join(id).join(".local/share/Steam"));
        v.push(home.join(".var/app").join(id).join("data/Steam"));
    }
    v
}

fn looks_like_steam(p: &Path) -> bool {
    p.join("steamapps").is_dir() || p.join("config/loginusers.vdf").exists() || p.join("steam.sh").exists()
}

fn mtime_ms(p: &Path) -> u64 { std::fs::metadata(p).map(|m| millis(m.modified())).unwrap_or(0) }

/// When Steam last used a folder: its newest sign-in, settings or library file (ms, 0 when it has none).
pub fn last_used(p: &Path) -> u64 {
    ["config/loginusers.vdf", "config/config.vdf", "steamapps/libraryfolders.vdf", "config/libraryfolders.vdf"]
        .iter().map(|f| mtime_ms(&p.join(f))).max().unwrap_or(0)
}

/// The Steam folders here, each once (~/.steam/steam, ~/.steam/root and the folder they point to are one folder;
/// so are /home/me/... and /var/home/me/... on the image-based editions): (real path, last used), in the order of
/// `candidates`.
pub fn steam_dirs(home: &Path) -> Vec<(PathBuf, u64)> {
    let mut out: Vec<(PathBuf, u64)> = Vec::new();
    for c in candidates(home) {
        if !looks_like_steam(&c) { continue; }
        let real = std::fs::canonicalize(&c).unwrap_or(c);
        if out.iter().any(|(p, _)| *p == real) { continue; }
        let used = last_used(&real);
        out.push((real, used));
    }
    out
}

/// The one Steam uses: the most recently used when there are several (a folder left from another edition or an
/// older install stays behind with its old files); the first in `candidates` order on a tie.
pub fn pick(dirs: &[(PathBuf, u64)]) -> Option<PathBuf> {
    let mut best: Option<&(PathBuf, u64)> = None;
    for d in dirs { if best.map(|b| d.1 > b.1).unwrap_or(true) { best = Some(d); } }
    best.map(|b| b.0.clone())
}

/// Where Steam keeps its data (native or Flatpak), if it's here.
pub fn steam_path(home: &Path) -> Option<PathBuf> { pick(&steam_dirs(home)) }

/// The Flatpak a Steam folder belongs to, if it's a Flatpak's.
pub fn flatpak_of(steam: &Path) -> Option<&'static str> {
    let s = steam.to_string_lossy();
    FLATPAK_IDS.iter().copied().find(|id| s.contains(&format!(".var/app/{id}/")))
}

/// A path with the home folder as ~ (for Hearth's log; the home folder has the user's name in it).
pub fn tilde(home: &Path, p: &Path) -> String {
    let s = p.to_string_lossy().to_string();
    let mut homes = vec![home.to_path_buf()];
    if let Ok(r) = std::fs::canonicalize(home) { homes.push(r); }
    for h in homes {
        let h = h.to_string_lossy().trim_end_matches('/').to_string();
        if h.len() > 1 && (s == h || s.starts_with(&format!("{h}/"))) { return format!("~{}", &s[h.len()..]); }
    }
    s
}

// ------------------------------------------------------------------ the signed-in account

/// One account in config/loginusers.vdf.
#[derive(Debug, Clone, PartialEq)]
pub struct LoginUser { pub id64: u64, pub account: String, pub most_recent: bool, pub timestamp: u64 }

/// The accounts in loginusers.vdf; Err: the file isn't there (Steam hasn't been signed in to in this folder) or
/// can't be read.
pub fn login_users(steam: &Path) -> Result<Vec<LoginUser>, String> {
    let f = steam.join("config/loginusers.vdf");
    if !f.exists() { return Err("missing".into()); }
    let kv = vdf::load(&f).ok_or("unreadable")?;
    Ok(kv.obj(&["users"]).map(|u| u.iter().filter_map(|(id, x)| {
        let id64 = id.trim().parse::<u64>().ok().filter(|n| *n > STEAM_ID64_BASE)?;
        Some(LoginUser { id64, account: x.str(&["AccountName"]).unwrap_or("").to_string(), most_recent: x.str(&["MostRecent"]).map(|v| v.trim() == "1").unwrap_or(false),
                         timestamp: x.num(&["Timestamp"]).max(0) as u64 })
    }).collect()).unwrap_or_default())
}

/// Steam's registry.vdf for a Steam folder: next to ~/.steam/debian-installation (that's ~/.steam), ~/.steam's for
/// native Steam, the Flatpak's own ~/.steam for a Flatpak's folder.
pub fn registry_files(home: &Path, steam: &Path) -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Some(id) = flatpak_of(steam) { v.push(home.join(".var/app").join(id).join(".steam/registry.vdf")); }
    v.push(home.join(".steam/registry.vdf"));
    if let Some(p) = steam.parent() { v.push(p.join("registry.vdf")); }
    let mut seen = HashSet::new();
    v.into_iter().filter(|p| seen.insert(p.clone())).collect()
}

/// The account Steam signs in as by itself (registry.vdf's AutoLoginUser), if it names one.
pub fn auto_login_user(home: &Path, steam: &Path) -> Option<String> {
    registry_files(home, steam).into_iter().find_map(|f| vdf::load(&f))
        .and_then(|r| r.str(&["Registry", "HKCU", "Software", "Valve", "Steam", "AutoLoginUser"]).map(|s| s.trim().to_string()))
        .filter(|s| !s.is_empty())
}

/// The signed-in Steam account's SteamID64 and how it was found. In order: the account loginusers.vdf marks
/// MostRecent (the newest Timestamp if it marks several), the one registry.vdf's AutoLoginUser names, and with
/// `guess` (one Hearth profile, so no other person's account to show by mistake) the only account, or the one
/// signed in last. Err: why none, with counts and the file's path only (never a name or an id).
pub fn find_account(home: &Path, steam: &Path, guess: bool) -> Result<(u64, &'static str), String> {
    let file = steam.join("config/loginusers.vdf");
    let users = match login_users(steam) {
        Ok(u) => u,
        Err(e) if e == "missing" => return Err(format!("no config/loginusers.vdf in {} (Steam hasn't been signed in to there)", tilde(home, steam))),
        Err(_) => return Err(format!("couldn't read {}", tilde(home, &file))),
    };
    if users.is_empty() { return Err(format!("{} lists no accounts", tilde(home, &file))); }
    if let Some(u) = users.iter().filter(|u| u.most_recent).max_by_key(|u| u.timestamp) { return Ok((u.id64, "MostRecent")); }
    let auto = auto_login_user(home, steam);
    if let Some(a) = &auto {
        if let Some(u) = users.iter().find(|u| !u.account.is_empty() && u.account.eq_ignore_ascii_case(a)) { return Ok((u.id64, "AutoLoginUser")); }
    }
    if guess {
        if users.len() == 1 { return Ok((users[0].id64, "the only account")); }
        if let Some(u) = users.iter().filter(|u| u.timestamp > 0).max_by_key(|u| u.timestamp) { return Ok((u.id64, "signed in last")); }
    }
    let reg = if auto.is_some() { "registry.vdf's AutoLoginUser isn't one of them" }
              else if registry_files(home, steam).iter().any(|f| f.exists()) { "registry.vdf names no account" } else { "no registry.vdf" };
    Err(format!("signed in account not found in {} ({} user{}, none MostRecent; {reg}{})", tilde(home, &file), users.len(), if users.len() == 1 { "" } else { "s" },
        if guess { "" } else { "; several Hearth profiles, so Hearth doesn't guess" }))
}

/// Says a Steam problem (or that it's fixed) in Hearth's log once, not on every library scan: `topic` keeps the
/// last line said about one thing.
pub fn note(topic: &'static str, text: &str) {
    static SAID: std::sync::Mutex<std::collections::BTreeMap<&'static str, String>> = std::sync::Mutex::new(std::collections::BTreeMap::new());
    let mut said = SAID.lock().unwrap_or_else(|e| e.into_inner());
    if said.get(topic).map(|t| t == text).unwrap_or(false) { return; }
    said.insert(topic, text.to_string());
    drop(said);
    crate::bridge::log(&format!("steam: {text}"));
}

/// Why Hearth can't find Steam's folder, for the log and Library › Stores.
pub fn no_folder_reason(home: &Path) -> String {
    format!("no Steam folder (looked in {})", candidates(home).iter().map(|c| tilde(home, c)).collect::<Vec<_>>().join(", "))
}

/// Library › Stores' Steam card and the log: Ok(SteamID64) of the signed-in account, or why it isn't known.
pub fn current_account(home: &Path) -> Result<u64, String> {
    let steam = steam_path(home).ok_or_else(|| no_folder_reason(home))?;
    find_account(home, &steam, !several_profiles()).map(|(id, _)| id)
}

// ------------------------------------------------------------------ Send logs: the Steam section

fn when(ms: u64) -> String {
    if ms == 0 { return "never".into(); }
    use chrono::TimeZone;
    chrono::Local.timestamp_millis_opt(ms as i64).single().map(|t| t.format("%Y-%m-%d %H:%M").to_string()).unwrap_or_else(|| ms.to_string())
}

/// What Hearth sees of Steam, for a bug report: the folders it looks in (and where the links go), the one it
/// uses, loginusers.vdf's counts (no names, no ids), the games' records per library, the owned-games list and
/// Steam's processes. Only file names, counts and times are read out.
pub fn report(home: &Path, source_on: bool) -> String {
    let mut o = String::new();
    macro_rules! say { ($e:expr) => {{ let t: String = $e; o.push_str(&t); o.push('\n'); }} }
    say!(format!("Library › Steam games: {}", if source_on { "on" } else { "OFF (Settings › Library): Hearth doesn't look for Steam games" }));
    say!(format!("Hearth profiles: {} (several: each profile sees its own Steam account's games)", profile_count()));
    say!(format!("Steam's program: {}{}", if crate::system::has("steam") { "steam is installed" } else { "no steam command" },
        FLATPAK_IDS.iter().filter(|id| Path::new("/var/lib/flatpak/app").join(id).is_dir() || home.join(".local/share/flatpak/app").join(id).is_dir()).map(|id| format!(", Flatpak {id}")).collect::<String>()));
    say!("--- folders Hearth looks in".into());
    for c in candidates(home) {
        let meta = std::fs::symlink_metadata(&c);
        let kind = match &meta {
            Err(_) => "missing".to_string(),
            Ok(m) if m.file_type().is_symlink() => {
                let to = std::fs::read_link(&c).map(|t| tilde(home, &t)).unwrap_or_else(|_| "?".into());
                format!("link -> {to}{}", if c.exists() { "" } else { " (BROKEN: it points to nothing)" })
            }
            Ok(m) if m.is_dir() => "folder".to_string(),
            Ok(_) => "a file, not a folder".to_string(),
        };
        let extra = if c.exists() {
            format!("; Steam's files: {}; last used {}", if looks_like_steam(&c) { "yes" } else { "no" }, when(last_used(&c)))
        } else { String::new() };
        say!(format!("{}: {kind}{extra}", tilde(home, &c)));
    }
    let dirs = steam_dirs(home);
    let Some(steam) = pick(&dirs) else {
        say!(format!("Steam folder used: none: {}", no_folder_reason(home)));
        return o;
    };
    say!(format!("Steam folder used: {} ({} Steam folder{} found)", tilde(home, &steam), dirs.len(), if dirs.len() == 1 { "" } else { "s" }));
    say!("--- account (counts only)".into());
    match login_users(&steam) {
        Ok(u) => say!(format!("config/loginusers.vdf: yes, {} user{}, {} marked MostRecent", u.len(), if u.len() == 1 { "" } else { "s" }, u.iter().filter(|x| x.most_recent).count())),
        Err(e) => say!(format!("config/loginusers.vdf: {e}")),
    }
    let regs: Vec<String> = registry_files(home, &steam).iter().filter(|f| f.exists()).map(|f| tilde(home, f)).collect();
    say!(format!("registry.vdf: {}; AutoLoginUser: {}", if regs.is_empty() { "none".into() } else { regs.join(", ") }, if auto_login_user(home, &steam).is_some() { "set" } else { "not set" }));
    let userdata = std::fs::read_dir(steam.join("userdata")).map(|d| d.flatten().filter(|e| e.file_name().to_string_lossy().parse::<u64>().map(|n| n > 0).unwrap_or(false)).count()).unwrap_or(0);
    say!(format!("userdata accounts: {userdata}"));
    let guess = profile_count() < 2;
    let acc = find_account(home, &steam, guess);
    say!(format!("signed-in account: {}", match &acc { Ok((_, how)) => format!("found ({how})"), Err(e) => format!("NOT FOUND: {e}") }));
    say!("--- libraries (appmanifest files)".into());
    for lib in libraries(&steam) {
        let n = std::fs::read_dir(lib.join("steamapps")).map(|d| d.flatten().filter(|e| { let f = e.file_name().to_string_lossy().to_string(); f.starts_with("appmanifest_") && f.ends_with(".acf") }).count());
        say!(format!("{}: {}", tilde(home, &lib), match n { Ok(n) => format!("{n} appmanifest files"), Err(_) => "no steamapps folder".into() }));
    }
    say!("--- owned games list (steamcommunity.com, or Steam's files on this device; cached)".into());
    match &acc {
        Ok((id, _)) => {
            let c = crate::steam_owned::load(*id);
            say!(format!("fetched: {}; games: {}; from: {}; problem: {}", when(c["fetched"].as_u64().unwrap_or(0)), c["games"].as_array().map(|a| a.len()).unwrap_or(0),
                match c["source"].as_str() { Some("device") => "this device's Steam files", Some(_) => "steamcommunity.com", None => "-" },
                c["problem"].as_str().unwrap_or("none")));
        }
        Err(_) => say!("not fetched: no signed-in account".into()),
    }
    let cached = std::fs::read_dir(crate::paths::data_dir().join("cache")).map(|d| d.flatten().filter(|e| e.file_name().to_string_lossy().starts_with("steam-owned-")).count()).unwrap_or(0);
    say!(format!("cached lists (all accounts): {cached}{}", if crate::steam_owned::busy() { "; fetching now" } else { "" }));
    say!("--- Steam's processes".into());
    let mut procs: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    if let Ok(d) = std::fs::read_dir("/proc") {
        for e in d.flatten() {
            if !e.file_name().to_string_lossy().chars().all(|c| c.is_ascii_digit()) { continue; }
            let comm = std::fs::read_to_string(e.path().join("comm")).unwrap_or_default().trim().to_string();
            if comm.to_lowercase().contains("steam") || comm == "reaper" { *procs.entry(comm).or_default() += 1; }
        }
    }
    say!(if procs.is_empty() { "none running".into() } else { procs.iter().map(|(k, n)| format!("{k} ×{n}")).collect::<Vec<_>>().join(", ") });
    o
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
/// One account's folder (userdata/<account id>) by its SteamID64, when it's there.
pub fn account_folder(steam: &Path, id64: u64) -> Option<PathBuf> {
    if id64 <= STEAM_ID64_BASE { return None; }
    let dir = steam.join("userdata").join((id64 - STEAM_ID64_BASE).to_string());
    if dir.is_dir() { Some(dir) } else { None }
}

/// The signed-in Steam account's SteamID64, when this Steam has more than one account (with one, every game is its).
/// Each Hearth profile signs Steam in as its own account (profiles.rs), so this is the profile's account.
/// `Some(0)`: no account is marked to sign in (MostRecent, or registry.vdf's AutoLoginUser: a profile that hasn't
/// signed in to Steam yet): none of the installed games are its own.
pub fn signed_in_id64(home: &Path, steam: &Path) -> Option<u64> {
    let users = std::fs::read_dir(steam.join("userdata")).ok()?.flatten()
        .filter(|e| e.file_name().to_string_lossy().parse::<u64>().map(|n| n > 0).unwrap_or(false)).count();
    if users == 0 { return None; }
    match find_account(home, steam, false) {
        Ok(_) if users < 2 => None,
        Ok((id, _)) => Some(id),
        Err(_) => Some(0),
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

/// How many Hearth profiles there are (profiles.json in Hearth's data folder; 1 without one).
pub fn profile_count() -> usize {
    let f = crate::paths::data_dir().join("profiles.json");
    std::fs::read(f).ok().and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
        .and_then(|v| v["list"].as_array().map(|l| l.len())).unwrap_or(1).max(1)
}
/// More than one Hearth profile.
pub fn several_profiles() -> bool { profile_count() > 1 }

pub fn scan_with(home: &Path, per_account: bool) -> Vec<SteamGame> {
    let mut games = Vec::new();
    let dirs = steam_dirs(home);
    let Some(steam) = pick(&dirs) else { note("folder", &no_folder_reason(home)); return games };
    note("folder", &format!("Steam folder {} ({} found)", tilde(home, &steam), dirs.len()));
    let (grid, stats) = user_data(&steam);
    let me = if per_account { signed_in_id64(home, &steam) } else { None };
    if me == Some(0) {
        let why = find_account(home, &steam, false).err().unwrap_or_default();
        note("mine", &format!("installed games hidden: several Hearth profiles and {why}"));
    } else { note("mine", "every installed game of the signed-in account shows"); }
    let cache = steam.join("appcache/librarycache");
    let libs = libraries(&steam);
    let mut manifests = 0usize;
    for lib in &libs {
        let apps = lib.join("steamapps");
        let Ok(rd) = std::fs::read_dir(&apps) else { continue };
        for e in rd.flatten() {
            let fname = e.file_name().to_string_lossy().to_string();
            if !(fname.starts_with("appmanifest_") && fname.ends_with(".acf")) { continue; }
            manifests += 1;
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
    note("scan", &format!("{} games from {manifests} appmanifest files in {} librar{}", games.len(), libs.len(), if libs.len() == 1 { "y" } else { "ies" }));
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
        assert_eq!(signed_in_id64(home.path(), &steam), Some(76561197960265729));
        // A profile that hasn't signed in yet: no account marked, none of these are its games.
        put(&steam.join("config/loginusers.vdf"), r#""users" { "76561197960265729" { "AccountName" "sam" "MostRecent" "0" } }"#);
        assert_eq!(signed_in_id64(home.path(), &steam), Some(0));
        assert!(scan_with(home.path(), true).is_empty());
        assert_eq!(scan_with(home.path(), false).len(), 3, "a single profile still sees them");
    }
    /// Sets a file's modified time (seconds after 2020) so "last used" can be compared.
    fn aged(p: &Path, secs: u64) {
        let t = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_577_836_800 + secs);
        std::fs::File::options().write(true).open(p).unwrap().set_modified(t).unwrap();
    }
    fn link(to: &Path, at: &Path) { std::fs::create_dir_all(at.parent().unwrap()).unwrap(); std::os::unix::fs::symlink(to, at).unwrap(); }
    const ONE_USER: &str = r#""users" { "76561197960265729" { "AccountName" "zqxsam" "PersonaName" "Zqxpersona" "MostRecent" "1" "Timestamp" "1700000000" } }"#;

    /// The owner's layout (Peppermint's Debian steam-installer, and the Anatase PC): Steam's data in
    /// ~/.steam/debian-installation, ~/.steam/steam and ~/.steam/root links to it, no ~/.local/share/Steam.
    #[test]
    fn finds_the_debian_installation() {
        let home = tempfile::tempdir().unwrap();
        let h = home.path();
        let deb = h.join(".steam/debian-installation");
        put(&deb.join("config/loginusers.vdf"), ONE_USER);
        put(&deb.join("steamapps/libraryfolders.vdf"), &format!(r#""libraryfolders" {{ "0" {{ "path" "{}" }} }}"#, deb.display()));
        put(&deb.join("steamapps/appmanifest_250420.acf"), r#""AppState" { "appid" "250420" "name" "Game A" "installdir" "A" "StateFlags" "4" }"#);
        put(&deb.join("steamapps/appmanifest_4183110.acf"), r#""AppState" { "appid" "4183110" "name" "Game B" "installdir" "B" "StateFlags" "4" }"#);
        put(&deb.join("steamapps/appmanifest_228980.acf"), r#""AppState" { "appid" "228980" "name" "Steamworks Common Redistributables" "installdir" "R" "StateFlags" "4" }"#);
        put(&h.join(".steam/registry.vdf"), r#""Registry" { "HKCU" { "Software" { "Valve" { "Steam" { "AutoLoginUser" "zqxsam" } } } } }"#);
        link(&deb, &h.join(".steam/steam"));
        link(&deb, &h.join(".steam/root"));
        let real = std::fs::canonicalize(&deb).unwrap();
        assert_eq!(steam_dirs(h).len(), 1, "the links and the folder are one Steam folder");
        assert_eq!(steam_path(h), Some(real.clone()));
        assert_eq!(find_account(h, &real, false), Ok((76561197960265729, "MostRecent")));
        assert_eq!(current_account(h), Ok(76561197960265729));
        let mut ids: Vec<String> = scan(h).into_iter().map(|g| g.app_id).collect();
        ids.sort();
        assert_eq!(ids, vec!["250420", "4183110"]);
        // Without the links (Steam's launcher hasn't made them yet), the folder itself is found too.
        std::fs::remove_file(h.join(".steam/steam")).unwrap();
        std::fs::remove_file(h.join(".steam/root")).unwrap();
        assert_eq!(steam_path(h), Some(real.clone()));
        // registry.vdf next to it is ~/.steam's.
        assert_eq!(auto_login_user(h, &real).as_deref(), Some("zqxsam"));
    }

    /// Several Steam folders: the one Steam used last wins (a stale ~/.local/share/Steam left from Bazzite or an
    /// older install, with games records but old files, doesn't hide the one Steam runs from now).
    #[test]
    fn several_folders_the_newest_wins() {
        let home = tempfile::tempdir().unwrap();
        let h = home.path();
        let old = h.join(".local/share/Steam");
        let new = h.join(".steam/debian-installation");
        put(&old.join("steamapps/libraryfolders.vdf"), r#""libraryfolders" { }"#);
        put(&old.join("config/loginusers.vdf"), ONE_USER);
        put(&new.join("config/loginusers.vdf"), ONE_USER);
        put(&new.join("steamapps/libraryfolders.vdf"), r#""libraryfolders" { }"#);
        for f in ["steamapps/libraryfolders.vdf", "config/loginusers.vdf"] { aged(&old.join(f), 100); aged(&new.join(f), 5000); }
        link(&new, &h.join(".steam/steam"));
        let dirs = steam_dirs(h);
        assert_eq!(dirs.len(), 2);
        assert_eq!(steam_path(h), Some(std::fs::canonicalize(&new).unwrap()), "{dirs:?}");
        // The other way round: the old one is Steam's again.
        aged(&old.join("config/loginusers.vdf"), 9000);
        assert_eq!(steam_path(h), Some(std::fs::canonicalize(&old).unwrap()));
        // A tie: candidates' order (~/.local/share/Steam first).
        aged(&new.join("config/loginusers.vdf"), 9000);
        assert_eq!(steam_path(h), Some(std::fs::canonicalize(&old).unwrap()));
        // An empty folder isn't a Steam folder.
        let home2 = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(home2.path().join(".local/share/Steam")).unwrap();
        assert_eq!(steam_path(home2.path()), None);
        assert!(no_folder_reason(home2.path()).contains("~/.steam/debian-installation"));
    }

    #[test]
    fn finds_flatpak_folders() {
        for (id, rel) in [("com.valvesoftware.Steam", ".local/share/Steam"), ("com.valvesoftware.Steam", "data/Steam"), ("org.anatase.Steam", "data/Steam")] {
            let home = tempfile::tempdir().unwrap();
            let h = home.path();
            let dir = h.join(".var/app").join(id).join(rel);
            put(&dir.join("config/loginusers.vdf"), ONE_USER);
            let real = std::fs::canonicalize(&dir).unwrap();
            assert_eq!(steam_path(h), Some(real.clone()), "{id} {rel}");
            assert_eq!(flatpak_of(&real), Some(id));
            put(&h.join(".var/app").join(id).join(".steam/registry.vdf"), r#""Registry" { "HKCU" { "Software" { "Valve" { "Steam" { "AutoLoginUser" "zqxkim" } } } } }"#);
            assert_eq!(registry_files(h, &real)[0], h.join(".var/app").join(id).join(".steam/registry.vdf"));
            assert_eq!(auto_login_user(h, &real).as_deref(), Some("zqxkim"));
        }
        assert_eq!(flatpak_of(Path::new("/home/x/.local/share/Steam")), None);
    }

    /// No account marked MostRecent: AutoLoginUser, then (one Hearth profile only) the only or newest account.
    /// Why not, with counts only: never an account name or a SteamID.
    #[test]
    fn signed_in_account_without_most_recent() {
        let home = tempfile::tempdir().unwrap();
        let h = home.path();
        let steam = h.join(".local/share/Steam");
        put(&steam.join("config/loginusers.vdf"), r#""users" { "76561197960265729" { "AccountName" "zqxsam" "MostRecent" "0" "Timestamp" "100" } }"#);
        assert_eq!(find_account(h, &steam, true), Ok((76561197960265729, "the only account")));
        let why = find_account(h, &steam, false).unwrap_err();
        assert!(why.contains("1 user, none MostRecent") && why.contains("config/loginusers.vdf") && why.contains("no registry.vdf"), "{why}");
        assert!(!why.contains("zqx") && !why.contains("7656119"), "no names or ids: {why}");
        // Several accounts, none marked: registry.vdf's AutoLoginUser (case doesn't matter), else the newest.
        put(&steam.join("config/loginusers.vdf"), r#""users" { "76561197960265729" { "AccountName" "zqxsam" "Timestamp" "100" } "76561197960265730" { "AccountName" "Zqxkim" "Timestamp" "200" } }"#);
        assert_eq!(find_account(h, &steam, true), Ok((76561197960265730, "signed in last")));
        put(&h.join(".steam/registry.vdf"), r#""Registry" { "HKCU" { "Software" { "Valve" { "Steam" { "AutoLoginUser" "ZQXSAM" } } } } }"#);
        assert_eq!(find_account(h, &steam, false), Ok((76561197960265729, "AutoLoginUser")));
        put(&h.join(".steam/registry.vdf"), r#""Registry" { "HKCU" { "Software" { "Valve" { "Steam" { "AutoLoginUser" "" } } } } }"#);
        let why = find_account(h, &steam, false).unwrap_err();
        assert!(why.contains("2 users, none MostRecent; registry.vdf names no account"), "{why}");
        // Several marked: the newest Timestamp.
        put(&steam.join("config/loginusers.vdf"), r#""users" { "76561197960265729" { "AccountName" "zqxsam" "MostRecent" "1" "Timestamp" "100" } "76561197960265730" { "AccountName" "zqxkim" "MostRecent" "1" "Timestamp" "200" } }"#);
        assert_eq!(find_account(h, &steam, false), Ok((76561197960265730, "MostRecent")));
        // No file, an empty list.
        std::fs::remove_file(steam.join("config/loginusers.vdf")).unwrap();
        assert!(find_account(h, &steam, true).unwrap_err().starts_with("no config/loginusers.vdf in"));
        put(&steam.join("config/loginusers.vdf"), r#""users" { }"#);
        assert!(find_account(h, &steam, true).unwrap_err().ends_with("lists no accounts"));
        // Per profile (several Hearth profiles): one account marked by AutoLoginUser still counts as signed in.
        put(&steam.join("config/loginusers.vdf"), r#""users" { "76561197960265729" { "AccountName" "zqxsam" } "76561197960265730" { "AccountName" "zqxkim" } }"#);
        put(&h.join(".steam/registry.vdf"), r#""Registry" { "HKCU" { "Software" { "Valve" { "Steam" { "AutoLoginUser" "zqxkim" } } } } }"#);
        put(&steam.join("userdata/1/config/localconfig.vdf"), r#""UserLocalConfigStore" { }"#);
        put(&steam.join("userdata/2/config/localconfig.vdf"), r#""UserLocalConfigStore" { }"#);
        assert_eq!(signed_in_id64(h, &steam), Some(76561197960265730));
    }

    /// Links: a broken ~/.steam/steam isn't a Steam folder; /home/me and /var/home/me (the image-based editions'
    /// two names for one home folder) find one folder, and the log shows it as ~.
    #[test]
    fn symlinks_and_var_home() {
        let root = tempfile::tempdir().unwrap();
        let var_home = root.path().join("var/home/chris");
        std::fs::create_dir_all(&var_home).unwrap();
        link(&root.path().join("var/home"), &root.path().join("home"));
        let h = root.path().join("home/chris");
        // Broken: points to a folder that isn't there.
        link(&var_home.join(".local/share/Steam"), &h.join(".steam/steam"));
        assert_eq!(steam_path(&h), None);
        std::fs::remove_file(h.join(".steam/steam")).unwrap();
        // The link names /var/home/..., Hearth's HOME is /home/...
        let deb = var_home.join(".steam/debian-installation");
        put(&deb.join("config/loginusers.vdf"), ONE_USER);
        put(&deb.join("steamapps/libraryfolders.vdf"), &format!(r#""libraryfolders" {{ "0" {{ "path" "{}" }} "1" {{ "path" "{}" }} }}"#, deb.display(), h.join(".steam/debian-installation").display()));
        link(&deb, &h.join(".steam/steam"));
        link(&deb, &h.join(".steam/root"));
        let real = std::fs::canonicalize(&deb).unwrap();
        assert_eq!(steam_dirs(&h), vec![(real.clone(), last_used(&real))]);
        assert_eq!(libraries(&real).len(), 1, "the same library by both names is one");
        assert_eq!(tilde(&h, &real), "~/.steam/debian-installation");
        assert_eq!(tilde(&h, &h.join(".steam/steam")), "~/.steam/steam");
        let r = report(&h, true);
        assert!(r.contains("~/.steam/steam: link -> ~/.steam/debian-installation"), "{r}");
        assert!(r.contains("Steam folder used: ~/.steam/debian-installation (1 Steam folder found)"), "{r}");
        assert!(r.contains("config/loginusers.vdf: yes, 1 user, 1 marked MostRecent"), "{r}");
        assert!(r.contains("~/.steam/debian-installation: 0 appmanifest files"), "{r}");
        assert!(!r.contains("zqx") && !r.contains("Zqx") && !r.contains("7656119"), "no names or ids: {r}");
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

/// The Steam Flatpak installed here (Flathub's first).
fn flatpak_app(home: &Path) -> Option<&'static str> {
    FLATPAK_IDS.iter().copied().find(|id| Path::new("/var/lib/flatpak/app").join(id).is_dir() || home.join(".local/share/flatpak/app").join(id).is_dir())
}
fn flatpak_installed(home: &Path) -> bool { flatpak_app(home).is_some() }

/// How to hand Steam a steam:// URL or an option (-silent): the native client, the Flatpak's, or the desktop's
/// URL handler. The Flatpak when that's where Steam keeps its data (or the only Steam there is).
pub fn command(home: &Path, arg: &str) -> (String, Vec<String>) {
    let data_of = steam_path(home).and_then(|p| flatpak_of(&p));
    let flatpak_data = data_of.is_some();
    let native = crate::system::has("steam");
    let flatpak = crate::system::has("flatpak") && (flatpak_data || flatpak_installed(home));
    if native && !flatpak_data { return ("steam".into(), vec![arg.into()]); }
    if flatpak { return ("flatpak".into(), vec!["run".into(), data_of.or(flatpak_app(home)).unwrap_or(FLATPAK_ID).into(), arg.into()]); }
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
