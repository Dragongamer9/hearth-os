//! Extra facts on a game's page: its ProtonDB rating (Steam games by id; others by finding the same game on
//! Steam first), and your Steam achievements from Steam's own files. Answers are kept a few days
//! (cache/game-info.json) so pages open instantly and offline. Port of game-info.js; web calls through curl.

use serde_json::{json, Value};
use std::path::Path;

const DAY_MS: u64 = 86_400_000;

fn cache_file() -> std::path::PathBuf { crate::paths::data_dir().join("cache/game-info.json") }
/// Always an object (adding to anything else, from a damaged file, panicked).
fn load() -> Value { std::fs::read(cache_file()).ok().and_then(|b| serde_json::from_slice::<Value>(&b).ok()).filter(|v| v.is_object()).unwrap_or(json!({})) }
fn keep(k: &str, mut v: Value) -> Value {
    let mut c = load();
    v["at"] = json!(crate::library::now_ms());
    c[k] = v.clone();
    let _ = crate::paths::write_atomic(&cache_file(), c.to_string().as_bytes());
    v
}
fn fresh(k: &str, days: u64) -> Option<Value> { let e = load()[k].clone(); if is_fresh(&e, crate::library::now_ms(), days) { Some(e) } else { None } }
/// Kept less than `days` ago. One kept "in the future" (the clock was set back since, as on a handheld whose clock
/// starts wrong) counts as old: the subtraction overflowed there.
fn is_fresh(e: &Value, now: u64, days: u64) -> bool {
    let at = e["at"].as_u64().unwrap_or(0);
    e.is_object() && at <= now && now - at < days * DAY_MS
}

fn get_json(url: &str) -> Result<(u16, Value), String> {
    let out = crate::system::run("curl", &["-s", "-m", "10", "-A", "Hearth", "-w", "\n%{http_code}", url])?;
    let (body, code) = out.rsplit_once('\n').unwrap_or(("", "0"));
    Ok((code.trim().parse().unwrap_or(0), serde_json::from_str(body).unwrap_or(Value::Null)))
}

pub fn norm(s: &str) -> String {
    let low = s.to_lowercase();
    let cleaned: String = low.chars().map(|c| if "®™©:'’.,!?-–—_()[]".contains(c) { ' ' } else { c }).collect();
    let re = regex::Regex::new(r"\b(the|edition|goty|game of the year|definitive|enhanced|remastered|deluxe|complete)\b").unwrap();
    re.replace_all(&cleaned, " ").split_whitespace().collect::<Vec<_>>().join(" ")
}

fn tier_text(t: &str) -> (&'static str, &'static str) {
    match t { "native" => ("Native", "Made for Linux"), "platinum" => ("Platinum", "Runs perfectly"), "gold" => ("Gold", "Runs well after a tweak or two"),
        "silver" => ("Silver", "Runs, with some problems"), "bronze" => ("Bronze", "Runs, but badly"), "borked" => ("Borked", "Doesn't run"), _ => ("No rating yet", "Not enough reports") }
}

pub fn steam_app_id_for(title: &str) -> u64 {
    let k = format!("appid:{}", norm(title));
    if let Some(c) = fresh(&k, 30) { return c["appid"].as_u64().unwrap_or(0); }
    let url = format!("https://store.steampowered.com/api/storesearch/?term={}&l=english&cc=US", title.bytes().map(|b| if b.is_ascii_alphanumeric() { (b as char).to_string() } else { format!("%{b:02X}") }).collect::<String>());
    let Ok((200, j)) = get_json(&url) else { return 0 };
    let want = norm(title);
    let items = j["items"].as_array().cloned().unwrap_or_default();
    let hit = items.iter().find(|i| norm(i["name"].as_str().unwrap_or("")) == want)
        .or_else(|| items.iter().find(|i| { let n = norm(i["name"].as_str().unwrap_or("")); !n.is_empty() && (n.starts_with(&want) || want.starts_with(&n)) }));
    keep(&k, json!({"appid": hit.and_then(|h| h["id"].as_u64()).unwrap_or(0)}))["appid"].as_u64().unwrap_or(0)
}

pub fn protondb(id: &str, title: &str, source: &str) -> Value {
    let mut appid = id.strip_prefix("steam:").and_then(|s| s.parse().ok()).unwrap_or(0u64);
    if appid == 0 && !title.is_empty() && source != "xbox" { appid = steam_app_id_for(title); }
    if appid == 0 { return Value::Null; }
    let k = format!("pdb:{appid}");
    if let Some(c) = fresh(&k, 3) { return if c["tier"].is_string() { let mut c = c; c["appid"] = json!(appid); c } else { Value::Null }; }
    match get_json(&format!("https://www.protondb.com/api/v1/reports/summaries/{appid}.json")) {
        Ok((404, _)) => { keep(&k, json!({"tier": null})); Value::Null }
        Ok((200, j)) => {
            let tier = j["tier"].as_str().or(j["trendingTier"].as_str()).unwrap_or("pending").to_lowercase();
            let (label, text) = tier_text(&tier);
            let mut out = keep(&k, json!({"tier": tier, "label": label, "text": text, "reports": j["total"].as_u64().unwrap_or(0), "url": format!("https://www.protondb.com/app/{appid}")}));
            out["appid"] = json!(appid); out
        }
        _ => { let old = load()[&k].clone(); if old["tier"].is_string() { let mut o = old; o["appid"] = json!(appid); o } else { Value::Null } }   // offline: the last answer
    }
}

/// Your achievements for a Steam game, from appcache/stats (the game's list and your progress).
pub fn read_achievements(stats: &Path, appid: &str, account: u64) -> Value {
    let Some(schema) = crate::binkv::load(&stats.join(format!("UserGameStatsSchema_{appid}.bin"))) else { return Value::Null };
    let root = if schema[appid].is_object() { schema[appid].clone() } else { schema.as_object().and_then(|o| o.values().next().cloned()).unwrap_or(json!({})) };
    let user = crate::binkv::load(&stats.join(format!("UserGameStats_{account}_{appid}.bin"))).unwrap_or(json!({}));
    let ucache = if user["cache"].is_object() { user["cache"].clone() } else { user };
    let text = |v: &Value| -> String { if let Some(o) = v.as_object() { o.get("english").or(o.values().next()).and_then(|x| x.as_str()).unwrap_or("").to_string() } else { v.as_str().unwrap_or("").to_string() } };
    let mut list = Vec::new();
    for (stat_id, st) in root["stats"].as_object().cloned().unwrap_or_default() {
        let Some(bits) = st["bits"].as_object() else { continue };
        let mine = &ucache[&stat_id];
        let data = mine["data"].as_i64().unwrap_or(0) as u32;
        for (bit, a) in bits {
            let d = &a["display"];
            let b: u32 = bit.parse().unwrap_or(0);
            let done = b < 32 && (data >> b) & 1 == 1;
            let at = if done { mine["achievementtimes"][bit].as_i64().map(|t| t * 1000) } else { None };
            let icon = d["icon"].as_str().map(|i| format!("https://cdn.akamai.steamstatic.com/steamcommunity/public/images/apps/{appid}/{}", if done { i } else { d["icon_gray"].as_str().unwrap_or(i) }));
            let name = text(&d["name"]);
            list.push(json!({"id": a["name"], "name": if name.is_empty() { a["name"].as_str().unwrap_or("").to_string() } else { name }, "desc": text(&d["desc"]),
                             "hidden": d["hidden"] == "1" || d["hidden"] == 1, "done": done, "at": at, "icon": icon}));
        }
    }
    if list.is_empty() { return Value::Null; }
    list.sort_by(|a, b| b["done"].as_bool().cmp(&a["done"].as_bool()).then(b["at"].as_i64().unwrap_or(0).cmp(&a["at"].as_i64().unwrap_or(0))));
    let done = list.iter().filter(|a| a["done"] == true).count();
    json!({"total": list.len(), "done": done, "list": list})
}

pub fn achievements(home: &Path, appid: &str) -> Value {
    let Some(steam) = crate::steam::steam_path(home) else { return Value::Null };
    let users = crate::vdf::load(&steam.join("config/loginusers.vdf"));
    let id64 = users.as_ref().and_then(|u| u.obj(&["users"]).and_then(|items| items.iter().find(|(_, v)| v.str(&["MostRecent"]) == Some("1")).map(|(k, _)| k.clone())));
    let Some(account) = id64.and_then(|i| i.parse::<u64>().ok()).map(|i| i.saturating_sub(76561197960265728)) else { return Value::Null };
    read_achievements(&steam.join("appcache/stats"), appid, account)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn names_match_across_stores() {
        assert_eq!(norm("The Witcher® 3: Wild Hunt – Game of the Year Edition"), "witcher 3 wild hunt");
        assert_eq!(norm("DOOM (2016)"), "doom 2016");
    }
    #[test]
    fn cached_answers_from_a_clock_set_back_are_old() {
        let now = 100 * DAY_MS;
        assert!(is_fresh(&json!({"at": now - DAY_MS}), now, 3));
        assert!(!is_fresh(&json!({"at": now - 4 * DAY_MS}), now, 3));
        assert!(!is_fresh(&json!({"at": now + 1000}), now, 3));
        assert!(!is_fresh(&Value::Null, now, 3));
    }
}
