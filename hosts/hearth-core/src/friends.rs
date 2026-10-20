//! Friends from Steam, Xbox and Epic: who's online and what they're playing, and ways to message them or join
//! their game. Steam: The list comes from the Steam client on this device
//! (its cached friends in localconfig.vdf, so no Steam Web API key is needed); each friend's state from their
//! public Steam Community profile (?xml=1), fetched in the background and kept for a minute. Friends with a
//! private profile show without a state.

use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// `person`: Steam's profile service answered with a person's profile (Some(true)), or said there's no such person
/// (Some(false): a group or game community Steam keeps in the same list); None: couldn't tell (offline).
struct Status { at: u64, state: String, game: Option<String>, message: Option<String>, avatar: Option<String>, name: Option<String>, server: Option<String>, appid: Option<u64>, person: Option<bool> }

/// Is this profile XML a person's profile? Groups and game communities (which Steam keeps in the same friends
/// list on disk) answer with an error or without a steamID64 and online state.
pub fn is_person(xml: &str) -> bool {
    !xml.contains("<error>") && tag(xml, "steamID64").is_some() && tag(xml, "onlineState").is_some()
}

fn cache() -> &'static Mutex<HashMap<u64, Status>> { static C: OnceLock<Mutex<HashMap<u64, Status>>> = OnceLock::new(); C.get_or_init(Default::default) }
static FETCHING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Friends the Steam client knows: (id64, name, avatar hash)
pub fn known(home: &std::path::Path) -> Vec<(u64, String, Option<String>)> {
    let Some(steam) = crate::steam::steam_path(home) else { return vec![] };
    let Some(acct) = crate::steam::account_dir(&steam) else { return vec![] };
    let Some(cfg) = crate::vdf::load(&acct.join("config/localconfig.vdf")) else { return vec![] };
    let mine: u64 = acct.file_name().and_then(|n| n.to_str()).and_then(|n| n.parse().ok()).unwrap_or(0);
    let mut out = Vec::new();
    for (k, v) in cfg.obj(&["UserLocalConfigStore", "friends"]).cloned().unwrap_or_default() {
        let Ok(acc) = k.parse::<u64>() else { continue };   // other keys are settings, not friends
        if acc == mine || acc == 0 { continue; }
        let Some(name) = v.str(&["name"]).map(str::to_string) else { continue };
        let avatar = v.str(&["avatar"]).filter(|h| h.len() >= 20 && !h.chars().all(|c| c == '0')).map(str::to_string);
        out.push((crate::steam::ID64_BASE + acc, name, avatar));
    }
    out
}

/// One <tag>...</tag> from Steam's profile XML (CDATA or plain).
pub fn tag(xml: &str, t: &str) -> Option<String> {
    let start = xml.find(&format!("<{t}>"))? + t.len() + 2;
    let end = xml[start..].find(&format!("</{t}>"))? + start;
    let v = xml[start..end].trim().trim_start_matches("<![CDATA[").trim_end_matches("]]>").trim();
    if v.is_empty() { None } else { Some(v.to_string()) }
}

fn fetch(id: u64) -> Option<Status> {
    let r = crate::http::get(&format!("https://steamcommunity.com/profiles/{id}/?xml=1"), &[]).ok()?;
    if !r.ok() { return None; }
    let x = &r.text;
    if !is_person(x) {
        return Some(Status { at: crate::library::now_ms(), state: "unknown".into(), game: None, message: None, avatar: None, name: None, server: None, appid: None, person: Some(false) });
    }
    let game = tag(x, "inGameInfo").and_then(|g| tag(&g, "gameName"));
    let state = tag(x, "onlineState").unwrap_or_else(|| "unknown".into());
    let info = tag(x, "inGameInfo").unwrap_or_default();
    let appid = tag(&info, "gameLink").and_then(|l| l.rsplit('/').find(|p| !p.is_empty()).and_then(|p| p.parse().ok()));
    // A game on a server (Counter-Strike, Team Fortress…) can be joined straight away.
    let server = tag(x, "inGameServerIP").filter(|ip| ip.contains(':') && !ip.starts_with("0.0.0.0"));
    Some(Status { at: crate::library::now_ms(), state: if game.is_some() { "ingame".into() } else { state }, game, message: tag(x, "stateMessage").map(|m| m.replace("<br/>", " · ")),
                  avatar: tag(x, "avatarMedium"), name: tag(x, "steamID"), server, appid, person: Some(true) })
}

/// Fetches stale states in the background (a few at a time); the host hears {name:'friends'} when they're in.
fn refresh(ids: Vec<u64>) {
    if ids.is_empty() || FETCHING.swap(true, std::sync::atomic::Ordering::SeqCst) { return; }
    std::thread::spawn(move || {
        let chunks: Vec<Vec<u64>> = ids.chunks(ids.len().div_ceil(4).max(1)).map(|c| c.to_vec()).collect();
        let workers: Vec<_> = chunks.into_iter().map(|c| std::thread::spawn(move || for id in c {
            let st = fetch(id).unwrap_or(Status { at: crate::library::now_ms(), state: "unknown".into(), game: None, message: None, avatar: None, name: None, server: None, appid: None, person: None });
            cache().lock().unwrap().insert(id, st);
        })).collect();
        for w in workers { let _ = w.join(); }
        FETCHING.store(false, std::sync::atomic::Ordering::SeqCst);
        crate::stores::notify(json!({"type": "event", "name": "friends", "data": {}}));
    });
}

/// Steam friends for the screen: playing first, then online, then the rest, each by name.
pub fn steam_list(home: &std::path::Path) -> Value {
    let friends = known(home);
    let now = crate::library::now_ms();
    let mut stale = Vec::new();
    let mut out: Vec<Value> = {
        let c = cache().lock().unwrap();
        // Groups and game communities Steam keeps in the same list aren't friends: left out once Steam says so.
        friends.iter().filter(|(id, _, _)| c.get(id).and_then(|s| s.person) != Some(false)).map(|(id, name, hash)| {
            let st = c.get(id);
            if st.map(|s| now.saturating_sub(s.at) > 60_000).unwrap_or(true) { stale.push(*id); }
            let avatar = st.and_then(|s| s.avatar.clone()).or(hash.as_ref().map(|h| format!("https://avatars.steamstatic.com/{h}_medium.jpg")));
            json!({"id": id.to_string(), "platform": "steam", "name": st.and_then(|s| s.name.clone()).unwrap_or_else(|| name.clone()),
                   "avatar": avatar.map(|a| crate::artcache::url(&a)), "state": st.map(|s| s.state.clone()).unwrap_or_else(|| "unknown".into()),
                   "game": st.and_then(|s| s.game.clone()), "message": st.and_then(|s| s.message.clone()),
                   "server": st.and_then(|s| s.server.clone()), "appid": st.and_then(|s| s.appid),
                   "canMessage": true, "canJoin": st.map(|s| s.server.is_some()).unwrap_or(false)})
        }).collect()
    };
    refresh(stale);
    let rank = |v: &Value| match v["state"].as_str().unwrap_or("") { "ingame" => 0, "online" => 1, "unknown" => 2, _ => 3 };
    out.sort_by(|a, b| rank(a).cmp(&rank(b)).then(a["name"].as_str().unwrap_or("").to_lowercase().cmp(&b["name"].as_str().unwrap_or("").to_lowercase())));
    let online = out.iter().filter(|f| rank(f) < 2).count();
    json!({"friends": out, "online": online, "signedIn": crate::steam::steam_path(home).and_then(|s| crate::steam::account_dir(&s)).is_some()})
}

// ------------------------------------------------------------------ Xbox and Epic

/// Each platform's last answer (time, friends or a problem), refreshed in the background at most every minute.
fn others() -> &'static Mutex<HashMap<&'static str, (u64, Result<Vec<Value>, String>)>> { static C: OnceLock<Mutex<HashMap<&'static str, (u64, Result<Vec<Value>, String>)>>> = OnceLock::new(); C.get_or_init(Default::default) }
static FETCHING_OTHERS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn refresh_others(which: Vec<&'static str>) {
    if which.is_empty() || FETCHING_OTHERS.swap(true, std::sync::atomic::Ordering::SeqCst) { return; }
    std::thread::spawn(move || {
        for p in which {
            let r = match p { "xbox" => crate::stores::xbox::friends(), "epic" => crate::stores::epic::friends(), _ => continue };
            if let Err(e) = &r { crate::bridge::log(&format!("friends: {p}: {e}")); }
            others().lock().unwrap().insert(p, (crate::library::now_ms(), r));
        }
        FETCHING_OTHERS.store(false, std::sync::atomic::Ordering::SeqCst);
        crate::stores::notify(json!({"type": "event", "name": "friends", "data": {}}));
    });
}

fn rank(v: &Value) -> u8 { match v["state"].as_str().unwrap_or("") { "ingame" => 0, "online" => 1, "away" | "snooze" | "busy" => 2, "unknown" => 3, _ => 4 } }
fn sort(list: &mut [Value]) { list.sort_by(|a, b| rank(a).cmp(&rank(b)).then(a["name"].as_str().unwrap_or("").to_lowercase().cmp(&b["name"].as_str().unwrap_or("").to_lowercase()))); }

/// Everyone, by platform: {platforms: [{id, name, signedIn, error, friends}], friends (all), online, signedIn}
pub fn list(home: &std::path::Path) -> Value {
    let steam = steam_list(home);
    let now = crate::library::now_ms();
    let mut stale = vec![];
    let mut platforms = vec![json!({"id": "steam", "name": "Steam", "signedIn": steam["signedIn"], "friends": steam["friends"]})];
    for (p, name) in [("xbox", "Xbox"), ("epic", "Epic Games")] {
        let signed = crate::stores::signed_in(p);
        let cached = others().lock().unwrap().get(p).map(|(at, r)| (*at, r.clone()));
        if signed && cached.as_ref().map(|c| now.saturating_sub(c.0) > 60_000).unwrap_or(true) { stale.push(p); }
        let (friends, error) = match cached { Some((_, Ok(mut f))) if signed => { sort(&mut f); (json!(f), Value::Null) }, Some((_, Err(e))) if signed => (json!([]), json!(e)), _ => (json!([]), Value::Null) };
        platforms.push(json!({"id": p, "name": name, "signedIn": signed, "loading": signed && friends.as_array().map(|a| a.is_empty()).unwrap_or(true) && error.is_null() && !others().lock().unwrap().contains_key(p), "error": error, "friends": friends}));
    }
    refresh_others(stale);
    let mut all: Vec<Value> = platforms.iter().flat_map(|p| p["friends"].as_array().cloned().unwrap_or_default()).collect();
    sort(&mut all);
    let online = all.iter().filter(|f| rank(f) < 3).count();
    let signed = platforms.iter().any(|p| p["signedIn"] == true);
    json!({"platforms": platforms, "friends": all, "online": online, "signedIn": signed})
}

/// Message or join: Steam opens its chat (where Join Game is) or joins the server; Xbox Cloud Gaming joins
/// by playing the same game in the cloud. Returns what the host should do: {open: steam://…} or {play: id}.
pub fn action(platform: &str, id: &str, what: &str, server: Option<&str>) -> Result<Value, String> {
    match (platform, what) {
        ("steam", "message") => { if !id.chars().all(|c| c.is_ascii_digit()) { return Err("Unknown friend".into()); } Ok(json!({"open": format!("steam://friends/message/{id}")})) }
        ("steam", "join") => {
            let s = server.filter(|s| s.chars().all(|c| c.is_ascii_digit() || c == '.' || c == ':')).ok_or("Join them from Steam's chat: open Message.")?;
            Ok(json!({"open": format!("steam://connect/{s}")}))
        }
        ("steam", "profile") => Ok(json!({"open": format!("steam://url/SteamIDPage/{id}")})),
        _ => Err("That isn't possible for this friend yet.".into()),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn reads_steam_profile_xml() {
        let x = "<profile><steamID><![CDATA[Gabe]]></steamID><onlineState>in-game</onlineState><stateMessage><![CDATA[In-Game<br/>Half-Life 3]]></stateMessage><inGameInfo><gameName><![CDATA[Half-Life 3]]></gameName></inGameInfo></profile>";
        assert_eq!(super::tag(x, "steamID").as_deref(), Some("Gabe"));
        assert_eq!(super::tag(x, "inGameInfo").and_then(|g| super::tag(&g, "gameName")).as_deref(), Some("Half-Life 3"));
        assert_eq!(super::tag(x, "missing"), None);
    }
}

#[cfg(test)]
mod person_tests {
    use super::*;
    #[test]
    fn groups_arent_friends() {
        assert!(is_person("<profile><steamID64>76561198000000001</steamID64><steamID><![CDATA[Sam]]></steamID><onlineState>online</onlineState></profile>"));
        assert!(!is_person("<?xml version=\"1.0\"?><response><error><![CDATA[The specified profile could not be found.]]></error></response>"));
        assert!(!is_person("<memberList><groupID64>103582791429521408</groupID64></memberList>"));
    }
}
