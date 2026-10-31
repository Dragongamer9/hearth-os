//! Pictures for any game, from SteamGridDB (https://www.steamgriddb.com, community-made art), with the player's
//! own SteamGridDB API key (Settings › Games › Integrations; off until a key is typed). Chosen pictures are saved
//! in Hearth's data folder (art/custom) and kept in library.json "customArt" {game id: {cover, wide, hero, logo}},
//! which the library puts over the store's own pictures (library.rs dto).

use serde_json::{json, Value};
use std::path::PathBuf;

const API: &str = "https://www.steamgriddb.com/api/v2";
pub const KINDS: [&str; 4] = ["cover", "wide", "hero", "logo"];

/// Games SteamGridDB knows by that name: [(id, name)].
pub fn parse_search(v: &Value) -> Vec<(u64, String)> {
    v["data"].as_array().into_iter().flatten().filter_map(|g| Some((g["id"].as_u64()?, g["name"].as_str()?.to_string()))).collect()
}
/// The pictures in an answer: [{url, thumb}] (only SteamGridDB's own addresses).
pub fn parse_images(v: &Value) -> Vec<Value> {
    v["data"].as_array().into_iter().flatten().filter_map(|i| {
        let url = i["url"].as_str().filter(|u| allowed(u))?;
        let thumb = i["thumb"].as_str().filter(|u| allowed(u)).unwrap_or(url);
        Some(json!({"url": url, "thumb": thumb}))
    }).take(24).collect()
}
/// The same name (after the usual tidying), else the first.
pub fn best_match(list: &[(u64, String)], title: &str) -> Option<u64> {
    let want = crate::gameinfo::norm(title);
    list.iter().find(|(_, n)| crate::gameinfo::norm(n) == want).or(list.first()).map(|(id, _)| *id)
}
/// Only pictures from SteamGridDB's own servers are downloaded.
pub fn allowed(url: &str) -> bool {
    let Some(rest) = url.strip_prefix("https://") else { return false };
    let host = rest.split(['/', '?', '#']).next().unwrap_or("").to_ascii_lowercase();
    !host.contains(':') && (host == "steamgriddb.com" || host.ends_with(".steamgriddb.com"))
}

fn ask(key: &str, path: &str) -> Result<Value, String> {
    let auth = format!("Bearer {key}");
    let r = crate::http::request("GET", &format!("{API}{path}"), &[("Authorization", &auth), ("Accept", "application/json")], None, 20)?;
    if r.status == 401 || r.status == 403 { return Err("SteamGridDB didn't accept your API key. Check it in Settings › Games › Integrations.".into()); }
    if !r.ok() { return Err(format!("SteamGridDB didn't answer (HTTP {})", r.status)); }
    Ok(r.json())
}

/// Pictures for a game by its name: {game: {id, name}, others: [names], cover, wide, hero, logo}.
pub fn search(title: &str) -> Result<Value, String> {
    let k = super::keys::get("steamgriddb").ok_or("Turn on SteamGridDB in Settings › Games › Integrations (it needs your own free API key).")?;
    let key = k["key"].as_str().unwrap_or("");
    let found = parse_search(&ask(key, &format!("/search/autocomplete/{}", crate::http::urlencode(title)))?);
    let id = best_match(&found, title).ok_or(format!("SteamGridDB has no game called {title}."))?;
    let name = found.iter().find(|(i, _)| *i == id).map(|(_, n)| n.clone()).unwrap_or_default();
    let get = |p: &str| ask(key, p).map(|v| parse_images(&v)).unwrap_or_default();
    Ok(json!({
        "game": {"id": id, "name": name}, "others": found.iter().filter(|(i, _)| *i != id).map(|(_, n)| n).take(5).collect::<Vec<_>>(),
        "cover": get(&format!("/grids/game/{id}?dimensions=600x900")),
        "wide": get(&format!("/grids/game/{id}?dimensions=920x430,460x215")),
        "hero": get(&format!("/heroes/game/{id}")),
        "logo": get(&format!("/logos/game/{id}")),
    }))
}

pub fn folder() -> PathBuf { crate::paths::data_dir().join("art/custom") }

/// Downloads one picture into art/custom (named after its address) and returns where it is.
pub fn save(url: &str) -> Result<PathBuf, String> {
    if !allowed(url) { return Err("Only SteamGridDB's pictures can be used here.".into()); }
    let ext = url.rsplit('.').next().map(|e| e.split(['?', '#']).next().unwrap_or("").to_ascii_lowercase()).filter(|e| ["png", "jpg", "jpeg", "webp"].contains(&e.as_str())).unwrap_or_else(|| "png".into());
    let dir = folder();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let file = dir.join(format!("{}.{ext}", crate::artcache::name_of(url)));
    if file.is_file() { return Ok(file); }
    let part = file.with_extension("part");
    let p = part.to_string_lossy().to_string();
    crate::system::run_timeout("curl", &["-sSfL", "--proto", "=https", "-m", "60", "-A", "Hearth", "-o", &p, url], std::time::Duration::from_secs(70))
        .map_err(|e| format!("Couldn't download that picture ({e})"))?;
    std::fs::rename(&part, &file).map_err(|e| e.to_string())?;
    Ok(file)
}

/// Sets (Some) or removes (None) one of a game's own pictures in library.json.
pub fn set(store: &mut crate::store::Store, game: &str, kind: &str, file: Option<&str>) -> Result<(), String> {
    if !KINDS.contains(&kind) { return Err("Unknown kind of picture".into()); }
    store.update(|d| {
        let all = crate::store::object_in(d, "customArt");
        let one = crate::store::object_at(all, game.to_string(), json!({}));
        if !one.is_object() { *one = json!({}); }
        match file { Some(f) => { one[kind] = json!(f); } None => { if let Some(o) = one.as_object_mut() { o.remove(kind); } } }
        if one.as_object().map(|o| o.is_empty()).unwrap_or(true) { all.remove(game); }
    }).map_err(|e| e.to_string())
}
pub fn clear(store: &mut crate::store::Store, game: &str) -> Result<(), String> {
    store.update(|d| { crate::store::object_in(d, "customArt").remove(game); }).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_steamgriddb_answers() {
        let s: Value = serde_json::from_str(r#"{"success":true,"data":[{"id":5,"name":"Hollow Knight: Voidheart Edition"},{"id":7,"name":"Hollow Knight","types":["steam"]}]}"#).unwrap();
        let l = parse_search(&s);
        assert_eq!(l.len(), 2);
        assert_eq!(best_match(&l, "Hollow Knight"), Some(7));
        assert_eq!(best_match(&l, "Something else"), Some(5), "no exact name: the first");
        assert_eq!(best_match(&[], "x"), None);
        let g: Value = serde_json::from_str(r#"{"success":true,"data":[
            {"id":1,"url":"https://cdn2.steamgriddb.com/grid/abc.png","thumb":"https://cdn2.steamgriddb.com/thumb/abc.jpg"},
            {"id":2,"url":"https://evil.example.com/x.png"},
            {"id":3,"url":"http://cdn2.steamgriddb.com/grid/plain.png"}]}"#).unwrap();
        let im = parse_images(&g);
        assert_eq!(im.len(), 1, "only https pictures on SteamGridDB's servers");
        assert_eq!(im[0]["thumb"], "https://cdn2.steamgriddb.com/thumb/abc.jpg");
        assert!(allowed("https://steamgriddb.com/x.png"));
        assert!(!allowed("https://steamgriddb.com.evil.io/x.png"));
        assert!(!allowed("https://cdn2.steamgriddb.com:8443/x.png"));
        assert!(!allowed("file:///etc/passwd"));
    }

    #[test]
    fn custom_pictures_in_the_library() {
        let d = tempfile::tempdir().unwrap();
        let mut st = crate::store::Store::open(d.path().join("library.json"));
        set(&mut st, "manual:1", "cover", Some("/p/c.png")).unwrap();
        set(&mut st, "manual:1", "logo", Some("/p/l.png")).unwrap();
        assert_eq!(st.data["customArt"]["manual:1"]["cover"], "/p/c.png");
        set(&mut st, "manual:1", "cover", None).unwrap();
        assert!(st.data["customArt"]["manual:1"]["cover"].is_null());
        set(&mut st, "manual:1", "logo", None).unwrap();
        assert!(st.data["customArt"].get("manual:1").is_none(), "nothing left: the game's entry goes");
        assert!(set(&mut st, "manual:1", "banner", Some("/x")).is_err());
        set(&mut st, "steam:1", "hero", Some("/h.png")).unwrap();
        clear(&mut st, "steam:1").unwrap();
        assert!(st.data["customArt"].get("steam:1").is_none());
    }
}
