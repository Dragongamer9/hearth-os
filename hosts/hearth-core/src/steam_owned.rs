//! Steam games you own but haven't installed, so the library can show them with an Install button (like Epic's
//! and GOG's). The list comes from the signed-in account's public Steam Community games page
//! (steamcommunity.com/profiles/<id64>/games?tab=all&xml=1, or the page's own game list when the XML answer isn't
//! there): no Steam Web API key, no password, nothing from Steam's network protocol. Since autumn 2026 Steam sends
//! every visitor who isn't signed in from those pages to its sign-in page (seen from CI for public profiles, and on
//! the owner's device); Hearth never signs in to Steam's website, so it then shows the games Steam itself remembers
//! on this device for that account (device_apps: played, opened in Steam's library, or put in a collection), and
//! Library › Stores says so. The log says which case it was: private, sign-in page, or a page Hearth doesn't know
//! (its shape only). Names, types and install folders of apps Steam already knows come from Steam's own appinfo.vdf
//! (read only). Kept in Hearth's cache folder per Steam account, refreshed when older than 12 hours (1 hour for a
//! list from this device), like owned.rs.

use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

/// Why the list couldn't be read from the community site: the profile (or its game details) is private; Steam asked
/// to sign in before showing it (since autumn 2026 every games page, the XML one too, sends a visitor who isn't
/// signed in to steamcommunity.com/login); Steam answered with something Hearth doesn't know (its shape, no personal
/// data: tag names and size); or something passing (no connection, Steam busy).
#[derive(Debug, Clone, PartialEq)]
pub enum Problem { Private, SignIn, Unknown(String), Other(String) }

impl Problem {
    /// What the cache keeps: "private", "signin", "format: <shape>", or the passing problem's text.
    pub fn code(&self) -> String {
        match self { Problem::Private => "private".into(), Problem::SignIn => "signin".into(), Problem::Unknown(s) => format!("format: {s}"), Problem::Other(e) => e.clone() }
    }
    /// For Hearth's log.
    pub fn describe(&self) -> String {
        match self {
            Problem::Private => format!("Steam says the profile or its game details aren't public. {STEAM_PRIVACY}"),
            Problem::SignIn => "Steam's games pages need a signed-in visitor now (the profile itself is public)".into(),
            Problem::Unknown(s) => format!("Steam's games page is in a form Hearth doesn't know: {s}"),
            Problem::Other(e) => e.clone(),
        }
    }
    fn passing(&self) -> bool { matches!(self, Problem::Other(_)) }
}

/// The exact Steam settings that decide whether a games list is public.
const STEAM_PRIVACY: &str = "In Steam: your profile › Edit Profile › Privacy Settings: My profile: Public, and Game details: Public (Friends only isn't enough).";
pub const PRIVATE_NOTE: &str = "Only installed games show: Steam says your profile or its game details aren't public. In Steam: your profile › Edit Profile › Privacy Settings: My profile: Public, and Game details: Public (Friends only isn't enough).";
/// With the community page out of reach, the games Steam remembers on this device show instead (device_games).
pub const PRIVATE_DEVICE_NOTE: &str = "Steam says your profile or its game details aren't public, so Hearth shows the games Steam remembers on this device: ones you've played, opened or seen in Steam's library, or put in a collection. In Steam: your profile › Edit Profile › Privacy Settings: My profile: Public, and Game details: Public (Friends only isn't enough).";
pub const SIGNIN_NOTE: &str = "Your Steam profile is public, but Steam's website no longer shows anyone's games list to a visitor who isn't signed in, and Hearth never signs in to it. So Hearth shows the games Steam remembers on this device: ones you've played, opened or seen in Steam's library, or put in a collection. Scroll through Steam's own library once to fill it in.";
pub const SIGNIN_EMPTY_NOTE: &str = "Your Steam profile is public, but Steam's website no longer shows anyone's games list to a visitor who isn't signed in, and Hearth never signs in to it, so only installed games show. Open Steam's library once (Library › Stores › Steam) so Steam's own files on this device list your games, then press Refresh.";

#[derive(Debug, Clone, PartialEq, Default)]
pub struct OwnedGame { pub app_id: u32, pub title: String, pub minutes: u64, pub last_played: u64 }

// ------------------------------------------------------------------ reading the community page

fn unescape(s: &str) -> String {
    s.replace("&quot;", "\"").replace("&#34;", "\"").replace("&#39;", "'").replace("&#039;", "'").replace("&apos;", "'").replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&")
}

/// Steam's sign-in page, which every games page (XML or not) answers with for a visitor who isn't signed in
/// (a 302 to /login/?redir=..., which curl follows). Its marks: the page's login feature, its title, or the sign-in
/// links with the games page as where to go back.
pub fn is_sign_in_page(page: &str) -> bool {
    page.contains("data-featuretarget=\"login\"")
        || (page.contains("/login/home/") && (page.contains("<title>Sign In</title>") || page.contains("login_modal")))
        || (page.contains("g_sessionID") && page.contains("/login/?redir=") && !page.contains("rgGames") && !page.contains("gameslist"))
}

/// What an answer looks like, for the log when Hearth doesn't know it: its first tag names and size, never its text
/// (a profile's page has the person's name in it).
pub fn shape(page: &str) -> String {
    let re = regex::Regex::new(r"<\s*([!?/]?[A-Za-z][A-Za-z0-9:_-]{0,30})").unwrap();
    let tags: Vec<String> = re.captures_iter(&page[..page.len().min(4096)]).take(8).map(|c| format!("<{}", &c[1])).collect();
    let kb = (page.len() + 512) / 1024;
    if tags.is_empty() { format!("{} bytes, no tags", page.len()) } else { format!("{} … ({} KB)", tags.join(" "), kb) }
}

/// The profile's own XML (profiles/<id64>/?xml=1, still public): its privacyState ("public", "friendsonly",
/// "private"). Says whether a games page behind a sign-in would have been readable at all.
pub fn profile_private(xml: &str) -> Option<bool> {
    if !xml.contains("<profile") { return None; }
    crate::friends::tag(xml, "privacyState").map(|s| s != "public")
}

/// The games XML (games?tab=all&xml=1): <gamesList><games><game><appID>..</appID><name><![CDATA[..]]></name>
/// <hoursOnRecord>1,234.5</hoursOnRecord>..</game>..</games></gamesList>. A private profile answers <error>.
pub fn parse_xml(xml: &str) -> Result<Vec<OwnedGame>, Problem> {
    if !xml.contains("<gamesList") {
        if xml.contains("<error>") { return Err(Problem::Private); }
        if is_sign_in_page(xml) { return Err(Problem::SignIn); }
        return Err(Problem::Unknown(shape(xml)));
    }
    if xml.contains("<error>") && !xml.contains("<game>") { return Err(Problem::Private); }
    let mut out = Vec::new();
    let mut rest = xml;
    while let Some(i) = rest.find("<game>") {
        let after = &rest[i + 6..];
        let Some(end) = after.find("</game>") else { break };
        let block = &after[..end];
        rest = &after[end + 7..];
        let Some(id) = crate::friends::tag(block, "appID").and_then(|t| t.parse::<u32>().ok()) else { continue };
        let title = crate::friends::tag(block, "name").map(|t| unescape(&t)).unwrap_or_default();
        let hours: f64 = crate::friends::tag(block, "hoursOnRecord").map(|h| h.replace(',', "")).and_then(|h| h.parse().ok()).unwrap_or(0.0);
        if title.is_empty() { continue; }
        out.push(OwnedGame { app_id: id, title, minutes: (hours * 60.0).round() as u64, last_played: 0 });
    }
    if out.is_empty() && !xml.contains("<games>") { return Err(Problem::Private); }
    Ok(out)
}

/// One game from a page's list: {appid (number or text), name, playtime_forever (minutes) or hours_forever ("1,024"),
/// rtime_last_played or last_played (seconds)}.
fn page_game(g: &Value) -> Option<OwnedGame> {
    let id = g["appid"].as_u64().or_else(|| g["appid"].as_str().and_then(|s| s.parse().ok())).filter(|n| *n > 0 && *n <= u32::MAX as u64)? as u32;
    let title = g["name"].as_str().map(unescape).filter(|t| !t.trim().is_empty())?;
    let minutes = g["playtime_forever"].as_u64().or_else(|| g["hours_forever"].as_str().and_then(|h| h.replace(',', "").parse::<f64>().ok()).map(|h| (h * 60.0).round() as u64)).unwrap_or(0);
    let last = g["rtime_last_played"].as_u64().or(g["last_played"].as_u64()).unwrap_or(0) * 1000;
    Some(OwnedGame { app_id: id, title, minutes, last_played: last })
}

/// The games page itself (games/?tab=all): its list is in data-profile-gameslist="{..rgGames:[..]..}" (2023's
/// page) or `var rgGames = [..];` (the older one); failing both, any list of {"appid":..,"name":..} objects in the
/// page (a newer page that keeps the same fields elsewhere). A private profile says so in the page; Steam's sign-in
/// page and anything else say which they are.
pub fn parse_html(html: &str) -> Result<Vec<OwnedGame>, Problem> {
    let attr = |name: &str| -> Option<Value> {
        let key = format!("{name}=\"");
        let i = html.find(&key)?;
        let start = i + key.len();
        let end = html[start..].find('"')?;
        serde_json::from_str::<Value>(&unescape(&html[start..start + end])).ok()
    };
    let list: Option<Value> = if let Some(v) = attr("data-profile-gameslist") { Some(v["rgGames"].clone()) }
        else if let Some(i) = html.find("var rgGames") { html[i..].find('[').and_then(|b| crate::owned::json_in(&html[i + b..], '[')) }
        else { None };
    if let Some(Value::Array(games)) = list {
        let out: Vec<OwnedGame> = games.iter().filter_map(page_game).collect();
        // A public profile with private game details shows the page with no games.
        return if out.is_empty() { Err(Problem::Private) } else { Ok(out) };
    }
    if html.contains("profile_private_info") || html.contains("This profile is private") { return Err(Problem::Private); }
    if is_sign_in_page(html) { return Err(Problem::SignIn); }
    let loose = loose_games(html);
    if !loose.is_empty() { return Ok(loose); }
    Err(Problem::Unknown(shape(html)))
}

/// Last resort: every {"appid": N, "name": "..."} object in the page (quotes escaped or not), first of each app.
fn loose_games(html: &str) -> Vec<OwnedGame> {
    let text = unescape(html);
    let re = regex::Regex::new(r#"\{\s*"appid"\s*:\s*"?(\d{1,10})"?\s*,\s*"name"\s*:\s*"((?:[^"\\]|\\.){1,300})""#).unwrap();
    let mut seen = HashSet::new();
    re.captures_iter(&text).filter_map(|c| {
        let id: u32 = c[1].parse().ok().filter(|n| *n > 0)?;
        let title: String = serde_json::from_str::<String>(&format!("\"{}\"", &c[2])).ok()?;
        if title.trim().is_empty() || !seen.insert(id) { return None; }
        Some(OwnedGame { app_id: id, title, minutes: 0, last_played: 0 })
    }).collect()
}

/// Which problem says more: private or a sign-in wall over a vaguer one, a known page over a passing failure.
fn better(a: Problem, b: Problem) -> Problem {
    let rank = |p: &Problem| match p { Problem::Private => 3, Problem::SignIn => 2, Problem::Unknown(_) => 1, Problem::Other(_) => 0 };
    if rank(&b) > rank(&a) { b } else { a }
}

/// Asks the community site (XML first, then the page). Blocking: call it off the UI thread. When both are behind
/// Steam's sign-in, the profile's own XML (public) tells a private profile apart.
pub fn fetch(id64: u64) -> Result<Vec<OwnedGame>, Problem> {
    let answer = |url: String, parse: fn(&str) -> Result<Vec<OwnedGame>, Problem>| match crate::http::get(&url, &[]) {
        Ok(r) if r.ok() => parse(&r.text),
        Ok(r) if r.status == 401 || r.status == 403 => Err(Problem::SignIn),
        Ok(r) => Err(Problem::Other(format!("Steam Community answered {}", r.status))),
        Err(e) => Err(Problem::Other(e)),
    };
    let first = match answer(format!("https://steamcommunity.com/profiles/{id64}/games?tab=all&xml=1"), parse_xml) { Ok(l) => return Ok(l), Err(e) => e };
    let second = match answer(format!("https://steamcommunity.com/profiles/{id64}/games/?tab=all"), parse_html) { Ok(l) => return Ok(l), Err(e) => e };
    Err(verdict(first, second, || {
        let r = crate::http::get(&format!("https://steamcommunity.com/profiles/{id64}/?xml=1"), &[]).ok()?;
        if r.ok() { profile_private(&r.text) } else { None }
    }))
}

/// Which case it was, from the XML answer, the page's answer and (asked only when it decides) whether the profile's
/// own public XML says the profile isn't public. The games XML answers a visitor who isn't signed in with Steam's
/// "This profile is private" error even when Game details is Public (the owner's device, October 2026), and the
/// games page with the sign-in page: so "private" is only said when the page itself says so or the profile is
/// private / friends only; a public profile whose list Steam won't show a visitor is the sign-in case.
pub fn verdict(xml: Problem, page: Problem, profile_private: impl FnOnce() -> Option<bool>) -> Problem {
    if page == Problem::Private { return Problem::Private; }
    let walled = xml == Problem::SignIn || page == Problem::SignIn;
    if walled || xml == Problem::Private {
        return match profile_private() {
            Some(true) => Problem::Private,
            Some(false) => Problem::SignIn,
            None => if walled { Problem::SignIn } else { Problem::Private },
        };
    }
    better(xml, page)
}

// ------------------------------------------------------------------ the games Steam remembers on this device

/// The account's games from Steam's own files on this device, for when the community page can't be read: apps in
/// its localconfig.vdf (played or started), in its library cache (userdata/<id>/config/librarycache/<appid>.json:
/// games whose page was opened in Steam's library) and in its collections (cloud-storage-namespace-1.json,
/// sharedconfig.vdf). Only that account's folder; nothing shared between accounts, nothing from the network.
/// Names (and whether an app is a game) come from appinfo.vdf afterwards: an app Steam has no name for is left out.
pub fn device_apps(account: &Path) -> HashMap<u32, (u64, u64)> { device_apps_with(account, None) }

/// device_apps, plus Steam's own library art cache (appcache/librarycache: a folder, or older <appid>_*.jpg files,
/// for each game Steam's library has shown) when `cache` is given: only with one account in this Steam folder, as
/// that cache is shared by every account on it. Steam's licensecache isn't read: the client keeps it encrypted.
pub fn device_apps_with(account: &Path, cache: Option<&Path>) -> HashMap<u32, (u64, u64)> {
    let mut out: HashMap<u32, (u64, u64)> = HashMap::new();
    let mut add = |id: &str, minutes: u64, last: u64| {
        if let Ok(n) = id.trim().parse::<u32>() { if n > 0 { let e = out.entry(n).or_insert((0, 0)); e.0 = e.0.max(minutes); e.1 = e.1.max(last); } }
    };
    if let Some(local) = crate::vdf::load(&account.join("config/localconfig.vdf")) {
        for (id, a) in local.obj(&["UserLocalConfigStore", "Software", "Valve", "Steam", "apps"]).into_iter().flatten() {
            add(id, a.num(&["Playtime"]).max(0) as u64, a.num(&["LastPlayed"]).max(0) as u64 * 1000);
        }
    }
    if let Some(Ok(d)) = cache.map(std::fs::read_dir) {
        for e in d.flatten() {
            let n = e.file_name().to_string_lossy().to_string();
            let id: String = n.chars().take_while(|c| c.is_ascii_digit()).collect();
            let rest = &n[id.len()..];
            if !id.is_empty() && (rest.is_empty() || rest.starts_with('_')) { add(&id, 0, 0); }
        }
    }
    if let Ok(d) = std::fs::read_dir(account.join("config/librarycache")) {
        for e in d.flatten() {
            let n = e.file_name().to_string_lossy().to_string();
            if let Some(id) = n.strip_suffix(".json") { add(id, 0, 0); }
        }
    }
    if let Some(shared) = crate::vdf::load(&account.join("7/remote/sharedconfig.vdf")) {
        for (id, _) in shared.obj(&["UserRoamingConfigStore", "Software", "Valve", "Steam", "apps"]).into_iter().flatten() { add(id, 0, 0); }
    }
    if let Some(v) = std::fs::read(account.join("config/cloudstorage/cloud-storage-namespace-1.json")).ok().and_then(|b| serde_json::from_slice::<Value>(&b).ok()) {
        for entry in v.as_array().into_iter().flatten() {
            let (key, item) = (entry[0].as_str().unwrap_or(""), &entry[1]);
            if !key.starts_with("user-collections.") || item["is_deleted"] == true { continue; }
            let Some(c) = item["value"].as_str().and_then(|t| serde_json::from_str::<Value>(t).ok()) else { continue };
            for id in c["added"].as_array().into_iter().flatten() {
                if let Some(n) = id.as_u64() { add(&n.to_string(), 0, 0); } else if let Some(s) = id.as_str() { add(s, 0, 0); }
            }
        }
    }
    out
}

/// device_apps with names from appinfo.vdf: games only (not tools, soundtracks, DLC), ones Steam has a name for.
pub fn device_games(info: &HashMap<u32, AppInfo>, apps: &HashMap<u32, (u64, u64)>) -> Vec<OwnedGame> {
    let mut out: Vec<OwnedGame> = apps.iter().filter_map(|(id, (minutes, last))| {
        let i = info.get(id)?;
        if i.name.trim().is_empty() || i.kind != "game" { return None; }
        Some(OwnedGame { app_id: *id, title: i.name.clone(), minutes: *minutes, last_played: *last })
    }).collect();
    out.sort_by_key(|g| g.app_id);
    out
}

// ------------------------------------------------------------------ Steam's appinfo.vdf (names, types, folders)

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AppInfo { pub name: String, pub kind: String, pub installdir: String }

/// Reads the apps asked for from Steam's appinfo.vdf (versions 27, 28 and 29), skipping the rest without reading
/// them (the file can be large). Nothing is changed; an unknown version gives nothing.
pub fn appinfo(path: &Path, wanted: &HashSet<u32>) -> HashMap<u32, AppInfo> {
    let mut out = HashMap::new();
    if wanted.is_empty() { return out; }
    let Ok(mut f) = std::fs::File::open(path) else { return out };
    let u32_at = |f: &mut std::fs::File| -> Option<u32> { let mut b = [0u8; 4]; f.read_exact(&mut b).ok()?; Some(u32::from_le_bytes(b)) };
    let Some(magic) = u32_at(&mut f) else { return out };
    let version = match magic { 0x07564427 => 27, 0x07564428 => 28, 0x07564429 => 29, _ => return out };
    if u32_at(&mut f).is_none() { return out; }   // universe
    // Version 29 keeps every key name once, in a table at the end of the file.
    let mut keys: Option<Vec<String>> = None;
    if version >= 29 {
        let mut b = [0u8; 8];
        if f.read_exact(&mut b).is_err() { return out; }
        let table = i64::from_le_bytes(b);
        let here = f.stream_position().unwrap_or(16);
        if table <= 0 || f.seek(SeekFrom::Start(table as u64)).is_err() { return out; }
        let mut rest = Vec::new();
        if f.read_to_end(&mut rest).is_err() || rest.len() < 4 { return out; }
        let n = u32::from_le_bytes(rest[..4].try_into().unwrap()) as usize;
        let mut list = Vec::with_capacity(n.min(1 << 20));
        let mut i = 4;
        while list.len() < n && i < rest.len() {
            let end = rest[i..].iter().position(|c| *c == 0).map(|p| p + i).unwrap_or(rest.len());
            list.push(String::from_utf8_lossy(&rest[i..end]).into_owned());
            i = end + 1;
        }
        keys = Some(list);
        if f.seek(SeekFrom::Start(here)).is_err() { return out; }
    }
    // Each app: id, size of the rest, then state, last update, token, text checksum, change number, (28+) data
    // checksum, then its binary KeyValues.
    let head = 4 + 4 + 8 + 20 + 4 + if version >= 28 { 20 } else { 0 };
    let file_len = f.metadata().map(|m| m.len()).unwrap_or(0);
    loop {
        let Some(id) = u32_at(&mut f) else { break };
        if id == 0 { break; }
        let Some(size) = u32_at(&mut f) else { break };
        let start = f.stream_position().unwrap_or(0);
        // An entry can't be bigger than what's left of the file: a damaged size (up to 4 GB) was allocated as it was.
        let left = file_len.saturating_sub(start);
        if size as u64 > left { break; }
        if wanted.contains(&id) && size as usize > head {
            let mut buf = vec![0u8; size as usize];
            if f.read_exact(&mut buf).is_err() { break; }
            let mut r = crate::binkv::Reader::new(&buf[head..]);
            if let Some(v) = crate::binkv::read(&mut r, keys.as_deref()) {
                let a = &v["appinfo"];
                out.insert(id, AppInfo { name: a["common"]["name"].as_str().unwrap_or("").to_string(), kind: a["common"]["type"].as_str().unwrap_or("").to_lowercase(),
                    installdir: a["config"]["installdir"].as_str().unwrap_or("").to_string() });
            }
            if out.len() == wanted.len() { break; }
        }
        if f.seek(SeekFrom::Start(start + size as u64)).is_err() { break; }
    }
    out
}

/// Things that show in a games list but aren't games to install (tools, soundtracks, videos, DLC).
pub fn not_a_game(kind: &str) -> bool { matches!(kind, "tool" | "config" | "dlc" | "music" | "video" | "series" | "episode" | "hardware" | "beta") }

// ------------------------------------------------------------------ the cached list

fn cache_name(id64: u64) -> String { format!("steam-owned-{id64}.json") }
fn cache_file(id64: u64) -> PathBuf { crate::paths::data_dir().join("cache").join(cache_name(id64)) }

/// The signed-in account's SteamID64 (steam::find_account: MostRecent, AutoLoginUser, and with one Hearth profile
/// the only or last account).
pub fn current_id64(home: &Path, steam: &Path) -> Option<u64> {
    crate::steam::find_account(home, steam, !crate::steam::several_profiles()).ok().map(|(id, _)| id)
}

/// What's cached for an account: {fetched, games: [{id, title, minutes, lastPlayed}], problem: "private"|text|null}
pub fn load(id64: u64) -> Value {
    std::fs::read(cache_file(id64)).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(json!({"fetched": 0, "games": [], "problem": null}))
}
pub fn load_current(home: &Path) -> Option<Value> { crate::steam::current_account(home).ok().map(load) }

/// The cache entry for a fetch: {fetched, games, problem, source: "community"|"device"}. The community list when
/// it came; otherwise the games Steam remembers on this device (`device`, already named and filtered), with the
/// problem kept so the card says why. A passing problem keeps an earlier community list (it shouldn't empty your
/// library); a private profile with nothing on the device shows installed games only.
pub fn entry(old: &Value, result: Result<Vec<OwnedGame>, Problem>, device: Vec<OwnedGame>, info: &HashMap<u32, AppInfo>, now: u64) -> Value {
    let games_of = |list: Vec<OwnedGame>| -> Vec<Value> {
        list.into_iter().filter(|g| {
            let kind = info.get(&g.app_id).map(|i| i.kind.as_str()).unwrap_or("");
            !not_a_game(kind) && !crate::steam::skip(&g.app_id.to_string(), &g.title)
        }).map(|g| json!({"id": g.app_id.to_string(), "title": g.title, "minutes": g.minutes, "lastPlayed": g.last_played})).collect()
    };
    match result {
        Ok(list) => json!({"fetched": now, "games": games_of(list), "problem": null, "source": "community"}),
        Err(p) => {
            let old_community = old["source"].as_str().unwrap_or("community") == "community" && old["games"].as_array().map(|a| !a.is_empty()).unwrap_or(false);
            if p.passing() && old_community { return json!({"fetched": old["fetched"], "games": old["games"], "problem": p.code(), "source": "community"}); }
            if !device.is_empty() { return json!({"fetched": now, "games": games_of(device), "problem": p.code(), "source": "device"}); }
            match p {
                Problem::Other(_) => json!({"fetched": old["fetched"], "games": old["games"], "problem": p.code(), "source": old["source"]}),
                _ => json!({"fetched": now, "games": [], "problem": p.code(), "source": "device"}),
            }
        }
    }
}

/// What Library › Stores' Steam card says about the list: (hint, error).
pub fn card_text(problem: Option<&str>, source: Option<&str>, games: usize) -> (Option<String>, Option<String>) {
    let device = source == Some("device") && games > 0;
    match problem {
        None => (None, None),
        Some("private") => (Some(if device { PRIVATE_DEVICE_NOTE } else { PRIVATE_NOTE }.to_string()), None),
        Some("signin") => (Some(if device { SIGNIN_NOTE } else { SIGNIN_EMPTY_NOTE }.to_string()), None),
        Some(p) if p.starts_with("format: ") => (None, Some(format!("Steam's games page changed and Hearth can't read it yet{}. Send the logs (Settings › System) so it can be fixed.",
            if device { "; the games Steam remembers on this device show meanwhile" } else { "" }))),
        Some(p) => (None, Some(format!("Couldn't get your Steam games list: {p}"))),
    }
}

static BUSY: AtomicBool = AtomicBool::new(false);
pub fn busy() -> bool { BUSY.load(Ordering::SeqCst) }

/// Fetches the signed-in account's list in the background (once at a time); the host hears {what:'rescan'} and
/// {name:'accounts.changed'} when it's in. `force`: even when the list is fresh. Err: why it can't (no Steam
/// folder, no signed-in account), also said in Hearth's log; Ok(false): the list is fresh or being fetched.
pub fn refresh(home: &Path, force: bool) -> Result<bool, String> {
    let r = start(home, force);
    match &r {
        Err(e) => crate::steam::note("owned", &format!("owned games list skipped: {e}")),
        Ok(false) => {}
        Ok(true) => crate::steam::note("owned", "owned games list: fetching"),
    }
    r
}

fn start(home: &Path, force: bool) -> Result<bool, String> {
    let steam = crate::steam::steam_path(home).ok_or_else(|| crate::steam::no_folder_reason(home))?;
    let (id64, _) = crate::steam::find_account(home, &steam, !crate::steam::several_profiles())?;
    let old = load(id64);
    // A list from this device's files is read again sooner: it grows as games are played or opened in Steam.
    let fresh_for: u64 = if old["source"] == "device" { 3600 * 1000 } else { 12 * 3600 * 1000 };
    if !force && crate::library::now_ms().saturating_sub(old["fetched"].as_u64().unwrap_or(0)) < fresh_for { return Ok(false); }
    if BUSY.swap(true, Ordering::SeqCst) { return Ok(false); }
    crate::stores::notify(json!({"type": "event", "name": "accounts.changed", "data": {}}));
    std::thread::spawn(move || {
        let result = fetch(id64);
        // The community page out of reach: the games Steam remembers on this device (that account's folder only).
        let apps = match &result {
            Err(_) => {
                let accounts = std::fs::read_dir(steam.join("userdata")).map(|d| d.flatten().filter(|e| e.file_name().to_string_lossy().parse::<u64>().map(|n| n > 0).unwrap_or(false)).count()).unwrap_or(0);
                let cache = steam.join("appcache/librarycache");
                crate::steam::account_folder(&steam, id64).map(|a| device_apps_with(&a, if accounts == 1 { Some(cache.as_path()) } else { None })).unwrap_or_default()
            }
            Ok(_) => HashMap::new(),
        };
        let ids: HashSet<u32> = match &result { Ok(l) => l.iter().map(|g| g.app_id).collect(), Err(_) => apps.keys().copied().collect() };
        let info = appinfo(&steam.join("appcache/appinfo.vdf"), &ids);
        let device = device_games(&info, &apps);
        let why = result.as_ref().err().map(|p| p.describe());
        let e = entry(&old, result, device, &info, crate::library::now_ms());
        let n = e["games"].as_array().map(|a| a.len()).unwrap_or(0);
        crate::steam::note("owned", &match why {
            None => format!("owned games list: {n} games (steamcommunity.com)"),
            Some(w) if e["source"] == "device" => format!("owned games list: {n} games from this device's Steam files ({w}; {} apps Steam remembers, {} with a name)", apps.len(), info.len()),
            Some(w) => format!("owned games list: {n} games ({w})"),
        });
        let _ = crate::paths::write_atomic(&cache_file(id64), e.to_string().as_bytes());
        BUSY.store(false, Ordering::SeqCst);
        crate::stores::notify(json!({"type": "event", "name": "accounts.changed", "data": {}}));
        crate::stores::notify(json!({"type": "internal", "what": "rescan", "reply": null}));
    });
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_games_xml() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><gamesList><steamID64>76561197960287930</steamID64><steamID><![CDATA[Rabscuttle]]></steamID>
<games><game><appID>620</appID><name><![CDATA[Portal 2]]></name><logo><![CDATA[https://x/620.jpg]]></logo><hoursOnRecord>1,204.5</hoursOnRecord></game>
<game><appID>400</appID><name><![CDATA[Portal &amp; Friends]]></name></game><game><appID>x</appID><name>bad</name></game></games></gamesList>"#;
        let g = parse_xml(xml).unwrap();
        assert_eq!(g.len(), 2);
        assert_eq!(g[0], OwnedGame { app_id: 620, title: "Portal 2".into(), minutes: 72270, last_played: 0 });
        assert_eq!(g[1].title, "Portal & Friends");
        assert_eq!(parse_xml("<?xml?><response><error><![CDATA[This profile is private.]]></error></response>"), Err(Problem::Private));
        assert_eq!(parse_xml("<gamesList><steamID64>1</steamID64><error>private</error></gamesList>"), Err(Problem::Private));
        assert!(matches!(parse_xml("<html>busy</html>"), Err(Problem::Unknown(_))));
        assert_eq!(parse_xml("<gamesList><games></games></gamesList>"), Ok(vec![]));
    }

    #[test]
    fn reads_the_games_page() {
        let page = r#"<div><template id="gameslist_config" data-profile-gameslist="{&quot;strProfileName&quot;:&quot;Sam&quot;,&quot;rgGames&quot;:[{&quot;appid&quot;:620,&quot;name&quot;:&quot;Portal 2&quot;,&quot;playtime_forever&quot;:90,&quot;rtime_last_played&quot;:1700000000},{&quot;appid&quot;:70,&quot;name&quot;:&quot;Half-Life&quot;}]}"></template></div>"#;
        let g = parse_html(page).unwrap();
        assert_eq!(g[0], OwnedGame { app_id: 620, title: "Portal 2".into(), minutes: 90, last_played: 1_700_000_000_000 });
        assert_eq!(g[1].app_id, 70);
        let old = r#"<script>var rgGames = [{"appid":440,"name":"Team Fortress 2","hours_forever":"3"}]; var x = 1;</script>"#;
        assert_eq!(parse_html(old).unwrap()[0].title, "Team Fortress 2");
        assert_eq!(parse_html(r#"<div class="profile_private_info">This profile is private.</div>"#), Err(Problem::Private));
        assert_eq!(parse_html(r#"<template data-profile-gameslist="{&quot;rgGames&quot;:[]}"></template>"#), Err(Problem::Private));
    }

    // ---- what steamcommunity.com answers (fixtures/steam_community: real answers saved from CI in October 2026,
    // with session ids and profile ids replaced; the synthetic_* ones are the list formats Steam used before)

    const XML_WALL: &str = include_str!("fixtures/steam_community/games_xml_signin_wall.html");
    const PAGE_WALL: &str = include_str!("fixtures/steam_community/games_page_signin_wall.html");
    const PROFILE_PUBLIC: &str = include_str!("fixtures/steam_community/profile_public.xml");
    const PROFILE_PRIVATE: &str = include_str!("fixtures/steam_community/profile_private.xml");
    const OLD_XML: &str = include_str!("fixtures/steam_community/synthetic_games_list.xml");
    const OLD_TEMPLATE: &str = include_str!("fixtures/steam_community/synthetic_games_page_template.html");
    const OLD_RGGAMES: &str = include_str!("fixtures/steam_community/synthetic_games_page_rggames.html");

    #[test]
    fn steams_sign_in_wall_is_named() {
        // Every games page, XML or not, public profile or not, now ends on Steam's sign-in page for a visitor.
        assert!(is_sign_in_page(XML_WALL) && is_sign_in_page(PAGE_WALL));
        assert_eq!(parse_xml(XML_WALL), Err(Problem::SignIn));
        assert_eq!(parse_html(PAGE_WALL), Err(Problem::SignIn));
        assert_eq!(parse_html(XML_WALL), Err(Problem::SignIn));
        assert_eq!(Problem::SignIn.code(), "signin");
        for page in [OLD_XML, OLD_TEMPLATE, OLD_RGGAMES, PROFILE_PUBLIC] { assert!(!is_sign_in_page(page)); }
        // The profile's own XML is still public: it tells a private profile apart.
        assert_eq!(profile_private(PROFILE_PUBLIC), Some(false));
        assert_eq!(profile_private(PROFILE_PRIVATE), Some(true));
        assert_eq!(profile_private(PAGE_WALL), None);
    }

    #[test]
    fn earlier_list_formats_still_read() {
        let g = parse_xml(OLD_XML).unwrap();
        assert_eq!(g, vec![OwnedGame { app_id: 620, title: "Portal 2".into(), minutes: 72270, last_played: 0 }, OwnedGame { app_id: 70, title: "Half-Life".into(), ..Default::default() }]);
        let g = parse_html(OLD_TEMPLATE).unwrap();
        assert_eq!(g[0], OwnedGame { app_id: 620, title: "Portal 2".into(), minutes: 72270, last_played: 1_700_000_000_000 });
        assert_eq!(g[1].title, "Half-Life & Friends");
        let g = parse_html(OLD_RGGAMES).unwrap();
        assert_eq!(g[0], OwnedGame { app_id: 440, title: "Team Fortress 2".into(), minutes: 180, last_played: 1_600_000_000_000 });
        assert_eq!((g[1].app_id, g[1].minutes), (570, 61440));
    }

    #[test]
    fn private_only_when_steam_really_says_so() {
        let asked = std::cell::Cell::new(false);
        let public = || { asked.set(true); Some(false) };
        // The owner's case: XML says "private" to a visitor, the page is the sign-in page, the profile is public.
        assert_eq!(verdict(Problem::Private, Problem::SignIn, public), Problem::SignIn);
        assert!(asked.get());
        assert_eq!(verdict(Problem::Private, Problem::Unknown("x".into()), || Some(false)), Problem::SignIn);
        assert_eq!(verdict(Problem::SignIn, Problem::SignIn, || Some(false)), Problem::SignIn);
        // Profile private or friends only: private, with the settings to change.
        assert_eq!(verdict(Problem::Private, Problem::SignIn, || Some(true)), Problem::Private);
        assert_eq!(verdict(Problem::SignIn, Problem::SignIn, || Some(true)), Problem::Private);
        assert_eq!(profile_private(&PROFILE_PUBLIC.replace(">public<", ">friendsonly<")), Some(true));
        // The page itself saying private wins; the profile XML out of reach keeps what the answers said.
        assert_eq!(verdict(Problem::SignIn, Problem::Private, || panic!("not needed")), Problem::Private);
        assert_eq!(verdict(Problem::Private, Problem::Other("busy".into()), || None), Problem::Private);
        assert_eq!(verdict(Problem::Private, Problem::SignIn, || None), Problem::SignIn);
        assert_eq!(verdict(Problem::Other("a".into()), Problem::Other("b".into()), || panic!("not needed")), Problem::Other("a".into()));
        assert!(Problem::Private.describe().contains("Game details: Public"));
        assert!(PRIVATE_NOTE.contains("Friends only"));
    }

    #[test]
    fn an_unknown_page_says_its_shape_not_its_text() {
        let page = r#"<!DOCTYPE html><html><head><title>Steam Community :: Sam Smith :: Games</title></head><body><div id="app"></div></body></html>"#;
        let Err(Problem::Unknown(s)) = parse_html(page) else { panic!("{:?}", parse_html(page)) };
        assert!(s.starts_with("<!DOCTYPE <html <head <title </title"), "{s}");
        assert!(!s.contains("Sam"), "no personal data in the log: {s}");
        assert_eq!(Problem::Unknown(s.clone()).code(), format!("format: {s}"));
        // A newer page with the same fields somewhere else still gives its games.
        let newer = r#"<html><script>window.SSR={"loaderData":[{"games":[{"appid":620,"name":"Portal 2","playtime":3},{"appid":"70","name":"Half-Life \u0026 more"}]}]}</script></html>"#;
        let g = parse_html(newer).unwrap();
        assert_eq!(g.iter().map(|x| (x.app_id, x.title.as_str())).collect::<Vec<_>>(), vec![(620, "Portal 2"), (70, "Half-Life & more")]);
        assert_eq!(shape(""), "0 bytes, no tags");
    }

    #[test]
    fn the_games_steam_remembers_on_this_device() {
        let dir = tempfile::tempdir().unwrap();
        let acc = dir.path().join("userdata/123");
        let put = |rel: &str, text: &str| { let p = acc.join(rel); std::fs::create_dir_all(p.parent().unwrap()).unwrap(); std::fs::write(p, text).unwrap(); };
        put("config/localconfig.vdf", r#""UserLocalConfigStore" { "Software" { "Valve" { "Steam" { "apps" { "620" { "Playtime" "90" "LastPlayed" "1700000000" } "1628350" { "LastPlayed" "1" } } } } } }"#);
        put("config/librarycache/70.json", "[]");
        put("config/librarycache/notanapp.txt", "");
        put("7/remote/sharedconfig.vdf", r#""UserRoamingConfigStore" { "Software" { "Valve" { "Steam" { "apps" { "440" { "tags" { "0" "favorite" } } } } } } }"#);
        let coll = json!([["user-collections.uc-1", {"key": "user-collections.uc-1", "value": "{\"id\":\"uc-1\",\"name\":\"RTS\",\"added\":[2330, 3]}"}],
                          ["user-collections.uc-2", {"key": "user-collections.uc-2", "is_deleted": true, "value": "{\"added\":[999]}"}],
                          ["showcases.1", {"value": "{\"added\":[888]}"}]]);
        put("config/cloudstorage/cloud-storage-namespace-1.json", &coll.to_string());
        let apps = device_apps(&acc);
        let mut ids: Vec<u32> = apps.keys().copied().collect();
        ids.sort();
        assert_eq!(ids, vec![3, 70, 440, 620, 2330, 1628350]);
        assert_eq!(apps[&620], (90, 1_700_000_000_000));
        let mut info = HashMap::new();
        for (id, name, kind) in [(620u32, "Portal 2", "game"), (70, "Half-Life", "game"), (440, "Team Fortress 2", "game"), (3, "OST", "music"), (1628350, "Steam Linux Runtime 3.0 (sniper)", "tool")] {
            info.insert(id, AppInfo { name: name.into(), kind: kind.into(), installdir: String::new() });
        }
        // 2330: Steam has no name for it here, so it's left out.
        let g = device_games(&info, &apps);
        assert_eq!(g.iter().map(|x| x.app_id).collect::<Vec<_>>(), vec![70, 440, 620]);
        assert_eq!(g[2].minutes, 90);
        assert!(device_apps(&dir.path().join("nobody")).is_empty());
        // Steam's library art cache (one account only): a folder per app, or the older loose files.
        let cache = dir.path().join("appcache/librarycache");
        std::fs::create_dir_all(cache.join("1145360")).unwrap();
        std::fs::write(cache.join("292030_library_600x900.jpg"), "").unwrap();
        std::fs::write(cache.join("assets.vdf"), "").unwrap();
        let with = device_apps_with(&acc, Some(&cache));
        assert!(with.contains_key(&1145360) && with.contains_key(&292030) && with.contains_key(&620), "{with:?}");
        assert_eq!(with.len(), apps.len() + 2);
    }

    #[test]
    fn cache_entries() {
        let old = json!({"fetched": 5, "games": [{"id": "1", "title": "Old"}], "problem": null});
        let mut info = HashMap::new();
        info.insert(3u32, AppInfo { name: "OST".into(), kind: "music".into(), installdir: String::new() });
        let list = vec![OwnedGame { app_id: 620, title: "Portal 2".into(), ..Default::default() }, OwnedGame { app_id: 3, title: "Soundtrack".into(), ..Default::default() },
                        OwnedGame { app_id: 1628350, title: "Steam Linux Runtime 3.0 (sniper)".into(), ..Default::default() }];
        let e = entry(&old, Ok(list), vec![], &info, 9);
        assert_eq!(e["games"].as_array().unwrap().len(), 1, "{e}");
        assert_eq!((e["games"][0]["id"].as_str(), e["source"].as_str()), (Some("620"), Some("community")));
        assert_eq!(entry(&old, Err(Problem::Private), vec![], &info, 9)["problem"], "private");
        let kept = entry(&old, Err(Problem::Other("No connection".into())), vec![], &info, 9);
        assert_eq!((kept["fetched"].as_u64(), kept["games"][0]["title"].as_str()), (Some(5), Some("Old")), "a passing problem keeps the old list");
        // Behind the sign-in: the games Steam remembers on this device, and why.
        let device = vec![OwnedGame { app_id: 70, title: "Half-Life".into(), ..Default::default() }];
        let e = entry(&old, Err(Problem::SignIn), device.clone(), &info, 9);
        assert_eq!((e["games"][0]["id"].as_str(), e["source"].as_str(), e["problem"].as_str(), e["fetched"].as_u64()), (Some("70"), Some("device"), Some("signin"), Some(9)));
        // A passing problem after a device list: the device list again (fresh), not the old one.
        let e2 = entry(&e, Err(Problem::Other("No connection".into())), device, &info, 10);
        assert_eq!((e2["source"].as_str(), e2["fetched"].as_u64()), (Some("device"), Some(10)));
        let none = entry(&old, Err(Problem::SignIn), vec![], &info, 9);
        assert_eq!((none["games"].as_array().unwrap().len(), none["problem"].as_str()), (0, Some("signin")));
        // What the Steam card says.
        assert_eq!(card_text(Some("signin"), Some("device"), 3).0.as_deref(), Some(SIGNIN_NOTE));
        assert_eq!(card_text(Some("signin"), Some("device"), 0).0.as_deref(), Some(SIGNIN_EMPTY_NOTE));
        assert_eq!(card_text(Some("private"), None, 0).0.as_deref(), Some(PRIVATE_NOTE));
        assert!(card_text(Some("format: <html … (5 KB)"), Some("device"), 2).1.unwrap().contains("changed"));
        assert_eq!(card_text(None, Some("community"), 9), (None, None));
    }

    /// A small appinfo.vdf in version 29's layout (keys in a table at the end) and in version 27's.
    fn appinfo_file(version: u32) -> Vec<u8> {
        let keys = ["appinfo", "common", "name", "type", "config", "installdir"];
        let mut f = Vec::new();
        f.extend((0x07564400u32 + version / 10 * 16 + version % 10).to_le_bytes());
        f.extend(1u32.to_le_bytes());
        let table_at = f.len();
        if version >= 29 { f.extend(0i64.to_le_bytes()); }
        let kstr = |out: &mut Vec<u8>, k: &str| { if version >= 29 { out.extend((keys.iter().position(|x| *x == k).unwrap() as i32).to_le_bytes()); } else { out.extend(k.as_bytes()); out.push(0); } };
        for (id, name, kind, dir) in [(10u32, "Counter-Strike", "Game", "Half-Life"), (620, "Portal 2", "game", "Portal 2")] {
            let mut kv = Vec::new();
            kv.push(0); kstr(&mut kv, "appinfo");
            kv.push(0); kstr(&mut kv, "common");
            kv.push(1); kstr(&mut kv, "name"); kv.extend(name.as_bytes()); kv.push(0);
            kv.push(1); kstr(&mut kv, "type"); kv.extend(kind.as_bytes()); kv.push(0);
            kv.push(8);
            kv.push(0); kstr(&mut kv, "config");
            kv.push(1); kstr(&mut kv, "installdir"); kv.extend(dir.as_bytes()); kv.push(0);
            kv.push(8); kv.push(8); kv.push(8);
            let head = 4 + 4 + 8 + 20 + 4 + if version >= 28 { 20 } else { 0 };
            f.extend(id.to_le_bytes());
            f.extend(((head + kv.len()) as u32).to_le_bytes());
            f.extend(vec![0u8; head]);
            f.extend(kv);
        }
        f.extend(0u32.to_le_bytes());
        if version >= 29 {
            let at = f.len() as i64;
            f[table_at..table_at + 8].copy_from_slice(&at.to_le_bytes());
            f.extend((keys.len() as u32).to_le_bytes());
            for k in keys { f.extend(k.as_bytes()); f.push(0); }
        }
        f
    }

    #[test]
    fn reads_appinfo() {
        for v in [27u32, 28, 29] {
            let dir = tempfile::tempdir().unwrap();
            let p = dir.path().join("appinfo.vdf");
            std::fs::write(&p, appinfo_file(v)).unwrap();
            let want: HashSet<u32> = [620u32, 999].into_iter().collect();
            let got = appinfo(&p, &want);
            assert_eq!(got.len(), 1, "version {v}");
            assert_eq!(got[&620], AppInfo { name: "Portal 2".into(), kind: "game".into(), installdir: "Portal 2".into() }, "version {v}");
            let both: HashSet<u32> = [10u32, 620].into_iter().collect();
            assert_eq!(appinfo(&p, &both)[&10].installdir, "Half-Life");
        }
        assert!(appinfo(Path::new("/nonexistent/appinfo.vdf"), &[1u32].into_iter().collect()).is_empty());
        // A damaged entry size (bigger than the file) ends the reading instead of allocating it.
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("appinfo.vdf");
        let mut f = appinfo_file(27);
        f[12..16].copy_from_slice(&u32::MAX.to_le_bytes());
        std::fs::write(&p, f).unwrap();
        assert!(appinfo(&p, &[10u32].into_iter().collect()).is_empty());
    }
}
