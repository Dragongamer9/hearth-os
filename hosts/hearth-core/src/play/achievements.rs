//! Achievements from official sources only (docs/HYDRA-FEATURES.md), each with the player's own key:
//!  - Steam's Web API (ISteamUserStats GetPlayerAchievements + GetSchemaForGame), for Steam games whose achievements
//!    aren't on this device. The key and SteamID64 are the player's; Hearth never signs in to Steam.
//!  - RetroAchievements (retroachievements.org's Web API, the player's user name and web API key), for emulated games.
//! Hearth never reads achievement files written by cracks or Steam emulators.
//! The answer has the same shape as the library's Steam achievements (gameinfo.rs): {done, total, list, source}.

use serde_json::{json, Value};

const HOUR_MS: u64 = 3_600_000;

/// GetPlayerAchievements + GetSchemaForGame → the page's list.
pub fn parse_steam(player: &Value, schema: &Value) -> Option<Value> {
    let got = player["playerstats"]["achievements"].as_array()?;
    let defs: Vec<Value> = schema["game"]["availableGameStats"]["achievements"].as_array().cloned().unwrap_or_default();
    let mut list: Vec<Value> = got.iter().filter_map(|a| {
        let api = a["apiname"].as_str()?;
        let def = defs.iter().find(|d| d["name"] == api);
        let done = a["achieved"].as_u64().unwrap_or(0) == 1;
        let name = def.and_then(|d| d["displayName"].as_str()).or(a["name"].as_str()).unwrap_or(api);
        let desc = def.and_then(|d| d["description"].as_str()).or(a["description"].as_str()).unwrap_or("");
        let icon = def.and_then(|d| if done { d["icon"].as_str() } else { d["icongray"].as_str() });
        let at = a["unlocktime"].as_u64().filter(|t| *t > 0).map(|t| t * 1000);
        Some(json!({"name": name, "desc": desc, "icon": icon, "done": done, "at": at, "hidden": def.map(|d| d["hidden"] == 1).unwrap_or(false)}))
    }).collect();
    if list.is_empty() { return None; }
    list.sort_by_key(|a| (a["done"] != true, std::cmp::Reverse(a["at"].as_u64().unwrap_or(0))));
    let done = list.iter().filter(|a| a["done"] == true).count();
    Some(json!({"done": done, "total": list.len(), "list": list, "source": "Steam Web API"}))
}

/// The game of yours on RetroAchievements with this name (from your completion progress list).
pub fn ra_find(progress: &Value, title: &str) -> Option<u64> {
    // ROM name tags ("(USA)", "[!]") aren't part of the name.
    let want = crate::gameinfo::norm(&regex::Regex::new(r"\s*[(\[][^)\]]*[)\]]").unwrap().replace_all(title, ""));
    let list = progress["Results"].as_array()?;
    list.iter().find(|g| crate::gameinfo::norm(g["Title"].as_str().unwrap_or("")) == want)
        .or_else(|| list.iter().find(|g| { let n = crate::gameinfo::norm(g["Title"].as_str().unwrap_or("")); !n.is_empty() && !want.is_empty() && (n.starts_with(&want) || want.starts_with(&n)) }))
        .and_then(|g| g["GameID"].as_u64())
}

/// API_GetGameInfoAndUserProgress → the page's list.
pub fn parse_ra(v: &Value) -> Option<Value> {
    let a = v["Achievements"].as_object()?;
    let mut list: Vec<Value> = a.values().map(|x| {
        let earned = x["DateEarnedHardcore"].as_str().filter(|s| !s.is_empty()).or(x["DateEarned"].as_str().filter(|s| !s.is_empty()));
        let badge = x["BadgeName"].as_str().unwrap_or("00000");
        let at = earned.and_then(|s| chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").ok()).map(|t| t.and_utc().timestamp_millis().max(0) as u64);
        json!({"name": x["Title"], "desc": x["Description"], "done": earned.is_some(), "at": at, "hidden": false, "points": x["Points"],
               "hardcore": x["DateEarnedHardcore"].as_str().map(|s| !s.is_empty()).unwrap_or(false),
               "icon": format!("https://media.retroachievements.org/Badge/{badge}{}.png", if earned.is_some() { "" } else { "_lock" }),
               "order": x["DisplayOrder"].as_u64().unwrap_or(0)})
    }).collect();
    if list.is_empty() { return None; }
    list.sort_by_key(|a| (a["done"] != true, a["order"].as_u64().unwrap_or(0)));
    let done = list.iter().filter(|a| a["done"] == true).count();
    Some(json!({"done": done, "total": list.len(), "list": list, "source": "RetroAchievements", "gameId": v["ID"]}))
}

fn get(url: &str) -> Result<Value, String> {
    let r = crate::http::request("GET", url, &[("Accept", "application/json")], None, 20)?;
    if r.status == 401 || r.status == 403 { return Err("The service didn't accept your key. Check it in Settings › Games › Integrations.".into()); }
    if !r.ok() { return Err(format!("No answer (HTTP {})", r.status)); }
    Ok(r.json())
}

/// A library game's achievements from the service that fits it, or {none, why}.
pub fn for_game(id: &str, title: &str, source: &str) -> Value {
    let r = if let Some(app) = id.strip_prefix("steam:") { steam(app) }
        else if source == "emu" { retro(title) }
        else { Err(match source {
            "gog" => "GOG doesn't offer achievements to other apps without each game's own GOG Galaxy sign-in, so Hearth can't show them yet.",
            "epic" => "Epic doesn't offer achievements to other apps, so Hearth can't show them yet.",
            _ => "No official source for this game's achievements.",
        }.to_string()) };
    r.unwrap_or_else(|e| json!({"none": true, "why": e}))
}

fn steam(app: &str) -> Result<Value, String> {
    if !app.chars().all(|c| c.is_ascii_digit()) { return Err("Not a Steam game".into()); }
    let k = super::keys::get("steamweb").ok_or("Turn on Steam Web API in Settings › Games › Integrations to see these achievements here.")?;
    let (key, sid) = (k["key"].as_str().unwrap_or(""), k["steamId"].as_str().unwrap_or(""));
    // The key goes in the address (Steam's Web API wants it there); http.rs passes addresses to curl on its standard
    // input, never on a command line. Kept an hour per game.
    crate::storefront::cached(&format!("play-ach-steam-{sid}-{app}"), HOUR_MS, || {
        let p = get(&format!("https://api.steampowered.com/ISteamUserStats/GetPlayerAchievements/v1/?appid={app}&key={}&steamid={sid}&l=english", crate::http::urlencode(key)))?;
        if p["playerstats"]["success"] == false { return Ok(json!({"none": true, "why": p["playerstats"]["error"].as_str().unwrap_or("Steam says this game has no achievements for you (or your game details are private).")})); }
        let s = get(&format!("https://api.steampowered.com/ISteamUserStats/GetSchemaForGame/v2/?appid={app}&key={}&l=english", crate::http::urlencode(key))).unwrap_or(Value::Null);
        Ok(parse_steam(&p, &s).unwrap_or(json!({"none": true, "why": "This game has no achievements on Steam."})))
    })
}

fn retro(title: &str) -> Result<Value, String> {
    let k = super::keys::get("retro").ok_or("Connect RetroAchievements in Settings › Games › Integrations to see achievements for your retro games.")?;
    let (user, key) = (k["user"].as_str().unwrap_or(""), k["key"].as_str().unwrap_or(""));
    let base = "https://retroachievements.org/API";
    let q = format!("u={}&y={}", crate::http::urlencode(user), crate::http::urlencode(key));
    let progress = crate::storefront::cached(&format!("play-ra-progress-{user}"), HOUR_MS, || get(&format!("{base}/API_GetUserCompletionProgress.php?{q}&c=500")))?;
    let gid = ra_find(&progress, title).ok_or("RetroAchievements doesn't list this game for you yet: play it once with RetroAchievements on in RetroArch.")?;
    crate::storefront::cached(&format!("play-ra-game-{user}-{gid}"), HOUR_MS, || {
        let v = get(&format!("{base}/API_GetGameInfoAndUserProgress.php?g={gid}&{q}"))?;
        Ok(parse_ra(&v).unwrap_or(json!({"none": true, "why": "This game has no achievements on RetroAchievements."})))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steam_web_api() {
        let p: Value = serde_json::from_str(r#"{"playerstats":{"steamID":"7656","gameName":"Portal 2","success":true,"achievements":[
            {"apiname":"ACH_A","achieved":1,"unlocktime":1700000000},{"apiname":"ACH_B","achieved":0,"unlocktime":0},{"apiname":"ACH_C","achieved":1,"unlocktime":1600000000}]}}"#).unwrap();
        let s: Value = serde_json::from_str(r#"{"game":{"availableGameStats":{"achievements":[
            {"name":"ACH_A","displayName":"Wake Up Call","description":"Survive","icon":"https://cdn/a.jpg","icongray":"https://cdn/a_g.jpg","hidden":0},
            {"name":"ACH_B","displayName":"Secret","description":"","icon":"https://cdn/b.jpg","icongray":"https://cdn/b_g.jpg","hidden":1}]}}}"#).unwrap();
        let r = parse_steam(&p, &s).unwrap();
        assert_eq!((r["done"].as_u64(), r["total"].as_u64()), (Some(2), Some(3)));
        assert_eq!(r["list"][0]["name"], "Wake Up Call", "newest unlock first");
        assert_eq!(r["list"][0]["at"], 1_700_000_000_000u64);
        assert_eq!(r["list"][1]["name"], "ACH_C", "no schema entry: its API name");
        assert_eq!(r["list"][2]["icon"], "https://cdn/b_g.jpg", "locked ones get the grey icon");
        assert_eq!(r["list"][2]["hidden"], true);
        assert!(parse_steam(&json!({"playerstats": {"achievements": []}}), &Value::Null).is_none());
    }

    #[test]
    fn retroachievements() {
        let prog: Value = serde_json::from_str(r#"{"Count":2,"Total":2,"Results":[
            {"GameID":228,"Title":"Super Mario World","ConsoleName":"SNES/Super Famicom","NumAwarded":10,"MaxPossible":96},
            {"GameID":1,"Title":"Sonic the Hedgehog","ConsoleName":"Genesis/Mega Drive"}]}"#).unwrap();
        assert_eq!(ra_find(&prog, "Super Mario World (USA)"), Some(228), "ROM name tags are ignored");
        assert_eq!(ra_find(&prog, "Sonic the Hedgehog"), Some(1));
        assert_eq!(ra_find(&prog, "Zelda"), None);
        let g: Value = serde_json::from_str(r#"{"ID":228,"Title":"Super Mario World","Achievements":{
            "342":{"ID":342,"Title":"Giddy Up!","Description":"Catch a ride with a friend","Points":1,"BadgeName":"46580","DisplayOrder":2,"DateEarned":"2024-01-02 03:04:05","DateEarnedHardcore":""},
            "341":{"ID":341,"Title":"I Believe I Can Fly","Description":"Use a cape","Points":2,"BadgeName":"46581","DisplayOrder":1}}}"#).unwrap();
        let r = parse_ra(&g).unwrap();
        assert_eq!((r["done"].as_u64(), r["total"].as_u64()), (Some(1), Some(2)));
        assert_eq!(r["list"][0]["name"], "Giddy Up!");
        assert_eq!(r["list"][0]["icon"], "https://media.retroachievements.org/Badge/46580.png");
        assert_eq!(r["list"][0]["hardcore"], false);
        assert_eq!(r["list"][0]["at"], 1_704_164_645_000u64);
        assert_eq!(r["list"][1]["icon"], "https://media.retroachievements.org/Badge/46581_lock.png");
        assert!(parse_ra(&json!({"Achievements": {}})).is_none());
    }

    #[test]
    fn no_unofficial_sources() {
        let r = for_game("gog:1", "Witcher", "gog");
        assert_eq!(r["none"], true);
        let r = for_game("manual:1", "Some game", "manual");
        assert_eq!(r["none"], true);
    }
}
