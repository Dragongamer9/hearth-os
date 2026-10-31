//! More on a library game's page (docs/HYDRA-FEATURES.md): its description, genres, developer, release date and PC
//! and Linux system requirements from Steam's public store page (any game Steam sells, found by name when it isn't
//! a Steam game), and how long it takes to beat from IGDB (only with the player's own Twitch developer app, in
//! Settings › Games › Integrations).
//!
//! HowLongToBeat has no public API and its terms don't allow scripted use, so Hearth doesn't read it. IGDB's
//! official API has the same kind of numbers (game_time_to_beats: rushed, normally, completely).

use serde_json::{json, Value};
use std::sync::Mutex;

const DAY_MS: u64 = 86_400_000;

/// Steam's requirements (minimum and recommended, PC and Linux) as lines of text; empty ones are left out.
pub fn requirements(d: &Value) -> Value {
    let mut out = serde_json::Map::new();
    for (k, name) in [("pc_requirements", "pc"), ("linux_requirements", "linux")] {
        let r = &d[k];
        if !r.is_object() { continue; }   // Steam sends [] when there are none
        let mut one = serde_json::Map::new();
        for level in ["minimum", "recommended"] {
            let lines: Vec<String> = crate::storefront::html_text(r[level].as_str().unwrap_or(""), 20).into_iter()
                .filter(|l| !l.eq_ignore_ascii_case("minimum:") && !l.eq_ignore_ascii_case("recommended:")).collect();
            if !lines.is_empty() { one.insert(level.to_string(), json!(lines)); }
        }
        if !one.is_empty() { out.insert(name.to_string(), Value::Object(one)); }
    }
    Value::Object(out)
}

/// The page's facts from appdetails: the store page's own reading (description, genres and so on) and the requirements.
pub fn from_appdetails(appid: u64, v: &Value) -> Result<Value, String> {
    let id = appid.to_string();
    let mut d = crate::storefront::steam::parse_details(&id, v)?;
    d["requirements"] = requirements(&v[&id]["data"]);
    d["appid"] = json!(appid);
    Ok(d)
}

/// Steam's app id for a library game: its own for Steam games, else found by name (kept 30 days, gameinfo.rs).
pub fn steam_id(id: &str, title: &str, source: &str) -> u64 {
    if let Some(n) = id.strip_prefix("steam:").and_then(|s| s.parse().ok()) { return n; }
    if source == "xbox" || source == "emu" || title.trim().is_empty() { return 0; }
    crate::gameinfo::steam_app_id_for(title)
}

/// Everything for a library game's page: {found, appid?, description, genres, developers, publishers, released,
/// requirements, timeToBeat?, igdbOff}.
pub fn for_game(id: &str, title: &str, source: &str) -> Value {
    let mut out = json!({"found": false});
    let appid = steam_id(id, title, source);
    if appid > 0 {
        let (market, lang) = crate::storefront::locale();
        let got = crate::storefront::cached(&format!("play-details-{market}-{lang}-{appid}"), 12 * 3600 * 1000, || {
            let v = crate::storefront::fetch("steam", "details", "GET", &format!("https://store.steampowered.com/api/appdetails?appids={appid}&cc={market}&l=english"), None, &[])?;
            from_appdetails(appid, &v)
        });
        if let Ok(d) = got { out = d; out["found"] = json!(true); }
    }
    match super::keys::get("igdb") {
        Some(k) => { if let Ok(t) = time_to_beat(&k, title) { out["timeToBeat"] = t; } }
        None => { out["igdbOff"] = json!(true); }
    }
    out
}

// ------------------------------------------------------------------ IGDB: how long to beat

/// IGDB's query language wants the name in double quotes: quotes and backslashes taken out.
pub fn igdb_name(title: &str) -> String { title.chars().filter(|c| *c != '"' && *c != '\\' && !c.is_control()).take(100).collect() }

/// The best game in an IGDB search: the same name, else the first.
pub fn igdb_pick(v: &Value, title: &str) -> Option<u64> {
    let want = crate::gameinfo::norm(title);
    let list = v.as_array()?;
    list.iter().find(|g| crate::gameinfo::norm(g["name"].as_str().unwrap_or("")) == want).or(list.first()).and_then(|g| g["id"].as_u64())
}
/// game_time_to_beats (seconds) as hours: {rushed, normally, completely, count, source}.
pub fn parse_ttb(v: &Value) -> Option<Value> {
    let t = v.as_array()?.first()?;
    let h = |k: &str| t[k].as_f64().map(|s| (s / 360.0).round() / 10.0);
    let (a, b, c) = (h("hastily"), h("normally"), h("completely"));
    if a.is_none() && b.is_none() && c.is_none() { return None; }
    Some(json!({"rushed": a, "normally": b, "completely": c, "count": t["count"].as_u64().unwrap_or(0), "source": "IGDB"}))
}

static TOKEN: Mutex<Option<(String, String, u64)>> = Mutex::new(None);   // (client id, token, valid until ms)

fn igdb_token(client: &str, secret: &str) -> Result<String, String> {
    let now = crate::library::now_ms();
    if let Some((c, t, until)) = TOKEN.lock().unwrap().clone() { if c == client && until > now + 60_000 { return Ok(t); } }
    let body = format!("client_id={}&client_secret={}&grant_type=client_credentials", crate::http::urlencode(client), crate::http::urlencode(secret));
    let r = crate::http::request("POST", "https://id.twitch.tv/oauth2/token", &[("Content-Type", "application/x-www-form-urlencoded")], Some(&body), 20)?;
    if !r.ok() { return Err("Twitch didn't accept your IGDB app's Client ID and secret.".into()); }
    let j = r.json();
    let t = j["access_token"].as_str().ok_or("Twitch didn't send a token")?.to_string();
    let until = now + j["expires_in"].as_u64().unwrap_or(3600) * 1000;
    *TOKEN.lock().unwrap() = Some((client.to_string(), t.clone(), until));
    Ok(t)
}

fn igdb(client: &str, token: &str, endpoint: &str, body: &str) -> Result<Value, String> {
    let auth = format!("Bearer {token}");
    let r = crate::http::request("POST", &format!("https://api.igdb.com/v4/{endpoint}"), &[("Client-ID", client), ("Authorization", &auth), ("Accept", "application/json")], Some(body), 20)?;
    if !r.ok() { return Err(format!("IGDB didn't answer (HTTP {})", r.status)); }
    Ok(r.json())
}

/// How long the game takes, kept a week per name.
pub fn time_to_beat(k: &Value, title: &str) -> Result<Value, String> {
    let (client, secret) = (k["clientId"].as_str().unwrap_or(""), k["clientSecret"].as_str().unwrap_or(""));
    let name = igdb_name(title);
    if name.trim().is_empty() { return Err("No name".into()); }
    crate::storefront::cached(&format!("play-ttb-{}", crate::gameinfo::norm(&name)), 7 * DAY_MS, || {
        let token = igdb_token(client, secret)?;
        let games = igdb(client, &token, "games", &format!("search \"{name}\"; fields id,name; limit 10;"))?;
        let Some(gid) = igdb_pick(&games, title) else { return Ok(json!({"none": true})) };
        let t = igdb(client, &token, "game_time_to_beats", &format!("fields game_id,hastily,normally,completely,count; where game_id = {gid};"))?;
        Ok(parse_ttb(&t).unwrap_or(json!({"none": true})))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requirements_from_steam() {
        let v: Value = serde_json::from_str(r#"{"620":{"success":true,"data":{"name":"Portal 2","type":"game","short_description":"The sequel.",
          "about_the_game":"<p>Portal 2 draws from the award-winning formula.</p>","header_image":"https://x/h.jpg",
          "genres":[{"id":"1","description":"Action"},{"id":"25","description":"Adventure"}],"developers":["Valve"],"publishers":["Valve"],
          "release_date":{"coming_soon":false,"date":"18 Apr, 2011"},"platforms":{"linux":true},
          "pc_requirements":{"minimum":"<strong>Minimum:</strong><br><ul class=\"bb_ul\"><li><strong>OS:</strong> Windows 7<br></li><li><strong>Memory:</strong> 2 GB RAM</li></ul>",
                             "recommended":"<strong>Recommended:</strong><ul><li>Memory: 4 GB RAM</li></ul>"},
          "linux_requirements":[] }}}"#).unwrap();
        let d = from_appdetails(620, &v).unwrap();
        assert_eq!(d["title"], "Portal 2");
        assert_eq!(d["genres"], json!(["Action", "Adventure"]));
        assert_eq!(d["appid"], 620);
        let min = d["requirements"]["pc"]["minimum"].as_array().unwrap();
        assert_eq!(min, &vec![json!("• OS: Windows 7"), json!("• Memory: 2 GB RAM")], "{min:?}");
        assert_eq!(d["requirements"]["pc"]["recommended"], json!(["• Memory: 4 GB RAM"]));
        assert!(d["requirements"].get("linux").is_none(), "Steam's [] means none");
        let bad: Value = serde_json::from_str(r#"{"5":{"success":false}}"#).unwrap();
        assert!(from_appdetails(5, &bad).is_err());
    }

    #[test]
    fn igdb_time_to_beat() {
        let games: Value = serde_json::from_str(r#"[{"id":11,"name":"Portal 2: Sixense"},{"id":72,"name":"Portal 2"}]"#).unwrap();
        assert_eq!(igdb_pick(&games, "Portal 2"), Some(72));
        assert_eq!(igdb_pick(&json!([]), "x"), None);
        let t: Value = serde_json::from_str(r#"[{"id":1,"game_id":72,"hastily":24300,"normally":36000,"completely":79200,"count":420}]"#).unwrap();
        let r = parse_ttb(&t).unwrap();
        assert_eq!(r["rushed"], 6.8);
        assert_eq!(r["normally"], 10.0);
        assert_eq!(r["completely"], 22.0);
        assert_eq!(r["count"], 420);
        assert!(parse_ttb(&json!([{"game_id": 1}])).is_none());
        assert!(parse_ttb(&json!([])).is_none());
        assert_eq!(igdb_name("Say \"hi\" \\ now"), "Say hi  now");
    }

    #[test]
    fn steam_ids() {
        assert_eq!(steam_id("steam:620", "Portal 2", "steam"), 620);
        assert_eq!(steam_id("emu:snes-x", "Super Mario World", "emu"), 0, "emulated games aren't looked up on Steam");
        assert_eq!(steam_id("xbox:1", "Halo", "xbox"), 0);
    }
}
