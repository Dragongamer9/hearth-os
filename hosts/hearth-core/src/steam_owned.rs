//! Steam games you own but haven't installed, so the library can show them with an Install button (like Epic's
//! and GOG's). The list comes from the signed-in account's public Steam Community games page
//! (steamcommunity.com/profiles/<id64>/games?tab=all&xml=1, or the page's own game list when the XML answer isn't
//! there): no Steam Web API key, no password, nothing from Steam's network protocol. It only works when the
//! profile's "Game details" are public; otherwise Hearth shows installed games only and says why.
//! Names, types and install folders of apps Steam already knows come from Steam's own appinfo.vdf (read only).
//! Kept in Hearth's cache folder per Steam account, refreshed when older than 12 hours (like owned.rs).

use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

/// Why the list couldn't be read: the profile (or its game details) is private, or something passing.
#[derive(Debug, Clone, PartialEq)]
pub enum Problem { Private, Other(String) }

pub const PRIVATE_NOTE: &str = "Only installed games show: your Steam profile's game details are private. In Steam, set Privacy › Game details to Public to see all your games here.";

#[derive(Debug, Clone, PartialEq, Default)]
pub struct OwnedGame { pub app_id: u32, pub title: String, pub minutes: u64, pub last_played: u64 }

// ------------------------------------------------------------------ reading the community page

fn unescape(s: &str) -> String {
    s.replace("&quot;", "\"").replace("&#39;", "'").replace("&#039;", "'").replace("&apos;", "'").replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&")
}

/// The games XML (games?tab=all&xml=1): <gamesList><games><game><appID>..</appID><name><![CDATA[..]]></name>
/// <hoursOnRecord>1,234.5</hoursOnRecord>..</game>..</games></gamesList>. A private profile answers <error>.
pub fn parse_xml(xml: &str) -> Result<Vec<OwnedGame>, Problem> {
    if !xml.contains("<gamesList") {
        return Err(if xml.contains("<error>") { Problem::Private } else { Problem::Other("Steam's answer wasn't a game list".into()) });
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

/// The games page itself (games/?tab=all): its list is in data-profile-gameslist="{..rgGames:[..]..}" (the
/// current page) or `var rgGames = [..];` (the older one). A private profile says so in the page.
pub fn parse_html(html: &str) -> Result<Vec<OwnedGame>, Problem> {
    let list: Option<Value> = if let Some(i) = html.find("data-profile-gameslist=\"") {
        let start = i + "data-profile-gameslist=\"".len();
        html[start..].find('"').and_then(|end| serde_json::from_str::<Value>(&unescape(&html[start..start + end])).ok()).map(|v| v["rgGames"].clone())
    } else if let Some(i) = html.find("var rgGames") {
        html[i..].find('[').and_then(|b| crate::owned::json_in(&html[i + b..], '['))
    } else { None };
    let Some(Value::Array(games)) = list else {
        return Err(if html.contains("profile_private_info") || html.contains("This profile is private") { Problem::Private } else { Problem::Other("Steam's games page didn't have a game list".into()) });
    };
    let out: Vec<OwnedGame> = games.iter().filter_map(|g| {
        let id = g["appid"].as_u64().or_else(|| g["appid"].as_str().and_then(|s| s.parse().ok()))? as u32;
        let title = g["name"].as_str().map(unescape).filter(|t| !t.is_empty())?;
        let minutes = g["playtime_forever"].as_u64().unwrap_or(0);
        let last = g["rtime_last_played"].as_u64().unwrap_or(0) * 1000;
        Some(OwnedGame { app_id: id, title, minutes, last_played: last })
    }).collect();
    // A public profile with private game details shows the page with no games.
    if out.is_empty() { return Err(Problem::Private); }
    Ok(out)
}

/// Asks the community site (XML first, then the page). Blocking: call it off the UI thread.
pub fn fetch(id64: u64) -> Result<Vec<OwnedGame>, Problem> {
    let xml = crate::http::get(&format!("https://steamcommunity.com/profiles/{id64}/games?tab=all&xml=1"), &[]);
    let first = match &xml {
        Ok(r) if r.ok() => parse_xml(&r.text),
        Ok(r) => Err(Problem::Other(format!("Steam Community answered {}", r.status))),
        Err(e) => Err(Problem::Other(e.clone())),
    };
    if let Ok(list) = first { return Ok(list); }
    let page = crate::http::get(&format!("https://steamcommunity.com/profiles/{id64}/games/?tab=all"), &[]);
    match page {
        Ok(r) if r.ok() => match parse_html(&r.text) {
            Ok(list) => Ok(list),
            // Either answer saying "private" wins over a vaguer problem with the other.
            Err(e) => Err(if first == Err(Problem::Private) { Problem::Private } else { e }),
        },
        Ok(r) => Err(if first == Err(Problem::Private) { Problem::Private } else { Problem::Other(format!("Steam Community answered {}", r.status)) }),
        Err(e) => Err(if first == Err(Problem::Private) { Problem::Private } else { Problem::Other(e) }),
    }
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

/// The cache entry for a fetch: the games that are games (by appinfo.vdf, when Steam knows the app), or the
/// problem with the old list kept (a passing problem shouldn't empty your library).
pub fn entry(old: &Value, result: Result<Vec<OwnedGame>, Problem>, info: &HashMap<u32, AppInfo>, now: u64) -> Value {
    match result {
        Ok(list) => {
            let games: Vec<Value> = list.into_iter().filter(|g| {
                let kind = info.get(&g.app_id).map(|i| i.kind.as_str()).unwrap_or("");
                !not_a_game(kind) && !crate::steam::skip(&g.app_id.to_string(), &g.title)
            }).map(|g| json!({"id": g.app_id.to_string(), "title": g.title, "minutes": g.minutes, "lastPlayed": g.last_played})).collect();
            json!({"fetched": now, "games": games, "problem": null})
        }
        // Private: there's nothing to show beyond the installed games.
        Err(Problem::Private) => json!({"fetched": now, "games": [], "problem": "private"}),
        Err(Problem::Other(e)) => json!({"fetched": old["fetched"], "games": old["games"], "problem": e}),
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
    if !force && crate::library::now_ms().saturating_sub(old["fetched"].as_u64().unwrap_or(0)) < 12 * 3600 * 1000 { return Ok(false); }
    if BUSY.swap(true, Ordering::SeqCst) { return Ok(false); }
    crate::stores::notify(json!({"type": "event", "name": "accounts.changed", "data": {}}));
    std::thread::spawn(move || {
        let result = fetch(id64);
        let ids: HashSet<u32> = match &result { Ok(l) => l.iter().map(|g| g.app_id).collect(), Err(_) => HashSet::new() };
        let info = appinfo(&steam.join("appcache/appinfo.vdf"), &ids);
        let e = entry(&old, result, &info, crate::library::now_ms());
        crate::steam::note("owned", &format!("owned games list: {} games{}", e["games"].as_array().map(|a| a.len()).unwrap_or(0),
            e["problem"].as_str().map(|p| format!(" ({p})")).unwrap_or_default()));
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
        assert!(matches!(parse_xml("<html>busy</html>"), Err(Problem::Other(_))));
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

    #[test]
    fn cache_entries() {
        let old = json!({"fetched": 5, "games": [{"id": "1", "title": "Old"}], "problem": null});
        let mut info = HashMap::new();
        info.insert(3u32, AppInfo { name: "OST".into(), kind: "music".into(), installdir: String::new() });
        let list = vec![OwnedGame { app_id: 620, title: "Portal 2".into(), ..Default::default() }, OwnedGame { app_id: 3, title: "Soundtrack".into(), ..Default::default() },
                        OwnedGame { app_id: 1628350, title: "Steam Linux Runtime 3.0 (sniper)".into(), ..Default::default() }];
        let e = entry(&old, Ok(list), &info, 9);
        assert_eq!(e["games"].as_array().unwrap().len(), 1, "{e}");
        assert_eq!(e["games"][0]["id"], "620");
        assert_eq!(entry(&old, Err(Problem::Private), &info, 9)["problem"], "private");
        let kept = entry(&old, Err(Problem::Other("No connection".into())), &info, 9);
        assert_eq!((kept["fetched"].as_u64(), kept["games"][0]["title"].as_str()), (Some(5), Some("Old")), "a passing problem keeps the old list");
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
