//! Steam's store, read from its public storefront API (store.steampowered.com/api: featured lists, search, app
//! details). No Steam account, token or Steam client library is used here: buying, wishlists and installing
//! happen in the Steam client itself (steam://store/<appid>, steam://install/<appid>), which is signed in already.

use super::{fetch, html_text, item, locale, money, price, section};
use serde_json::{json, Value};

const API: &str = "https://store.steampowered.com";

fn cc() -> String { locale().0 }
/// A Steam picture made from the app id (Steam's own address for it). Newer games keep their pictures under a
/// longer address, so the page falls back to the one Steam sent ("thumb") when this one isn't there.
fn art(id: u64, file: &str) -> String { format!("https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/{id}/{file}") }
pub fn page(id: u64) -> String { format!("{API}/app/{id}") }

/// A game from one of Steam's lists (featured, specials, top sellers...). `unpriced_is_free`: in lists of games
/// already out, no price and 0 means free; in Coming soon it means "no price yet".
pub fn list_item(i: &Value, unpriced_is_free: bool) -> Option<Value> {
    let id = i["id"].as_u64()?;
    let title = i["name"].as_str()?;
    let cur = i["currency"].as_str().unwrap_or("");
    let fin = i["final_price"].as_i64();
    let orig = i["original_price"].as_i64();
    let disc = i["discount_percent"].as_u64().unwrap_or(0);
    let p = match (fin, orig) {
        (Some(0), None) if unpriced_is_free => price("", None, 0, true),
        (Some(0), None) | (None, _) => Value::Null,
        (Some(f), o) => price(&money(f, cur), o.map(|o| money(o, cur)).as_deref(), disc, f == 0),
    };
    let wide = i["header_image"].as_str().or(i["large_capsule_image"].as_str()).map(str::to_string);
    let mut v = item("steam", &id.to_string(), title, wide, Some(art(id, "library_600x900.jpg")), p, Some(page(id)));
    v["linux"] = json!(i["linux_available"] == true);
    if let Some(c) = i["controller_support"].as_str() { v["controller"] = json!(c); }
    Some(v)
}

fn list(v: &Value, unpriced_is_free: bool) -> Vec<Value> {
    let mut seen = std::collections::HashSet::new();
    v.as_array().into_iter().flatten().filter_map(|i| list_item(i, unpriced_is_free)).filter(|x| seen.insert(x["id"].as_str().unwrap_or("").to_string())).collect()
}

/// featuredcategories: Specials and Top sellers. (Its New releases and Coming soon are Steam's unsorted feed of
/// every new store page, adult games included, so those come from Steam's curated lists instead: parse_featured.)
pub fn parse_categories(v: &Value) -> Vec<Value> {
    vec![section("deals", "Deals", list(&v["specials"]["items"], true)), section("top", "Top sellers", list(&v["top_sellers"]["items"], true))]
}
/// /api/featured: the games on Steam's front page, for this kind of PC (Windows games; Linux ones are marked).
pub fn parse_featured(v: &Value) -> Vec<Value> {
    vec![section("featured", "Featured", list(&v["featured_win"], true))]
}
/// The search page's "popular new releases" (search/results?json=1): names and a small picture with the app id in
/// its address. Prices come on the game's page.
pub fn parse_popular_new(v: &Value) -> Vec<Value> {
    let re = regex::Regex::new(r"/apps/(\d+)/").unwrap();
    let mut seen = std::collections::HashSet::new();
    let items = v["items"].as_array().into_iter().flatten().filter_map(|i| {
        let name = i["name"].as_str()?;
        let logo = i["logo"].as_str()?;
        let id: u64 = re.captures(logo)?.get(1)?.as_str().parse().ok()?;
        if !seen.insert(id) { return None; }
        let mut v = item("steam", &id.to_string(), name, Some(art(id, "header.jpg")), Some(art(id, "library_600x900.jpg")), Value::Null, Some(page(id)));
        v["thumb"] = json!(logo);
        Some(v)
    }).collect();
    vec![section("new", "Popular new releases", items)]
}

pub fn home() -> Result<Vec<Value>, String> {
    let cc = cc();
    let cats = fetch("steam", "home", "GET", &format!("{API}/api/featuredcategories?cc={cc}&l=english"), None, &[])?;
    let mut out = parse_categories(&cats);
    // The others are extras: the page still works without them.
    if let Ok(f) = fetch("steam", "home", "GET", &format!("{API}/api/featured/?cc={cc}&l=english"), None, &[]) { out.splice(0..0, parse_featured(&f)); }
    if let Ok(n) = fetch("steam", "home", "GET", &format!("{API}/search/results/?query&start=0&count=24&filter=popularnew&json=1&cc={cc}&l=english"), None, &[]) {
        out.extend(parse_popular_new(&n));
    }
    Ok(out)
}

/// storesearch: games matching a name (apps only; bundles and DLC left out by Steam).
pub fn parse_search(v: &Value) -> Vec<Value> {
    v["items"].as_array().into_iter().flatten().filter(|i| i["type"].as_str().unwrap_or("app") == "app").filter_map(|i| {
        let id = i["id"].as_u64()?;
        let title = i["name"].as_str()?;
        let p = &i["price"];
        let pr = if p.is_object() {
            let cur = p["currency"].as_str().unwrap_or("");
            let (f, o) = (p["final"].as_i64().unwrap_or(0), p["initial"].as_i64().unwrap_or(0));
            let disc = if o > f && o > 0 { ((o - f) * 100 / o) as u64 } else { 0 };
            price(&money(f, cur), Some(&money(o, cur)), disc, f == 0)
        } else { Value::Null };   // free games and ones with no price yet: the game's page says which
        let mut v = item("steam", &id.to_string(), title, Some(art(id, "header.jpg")), Some(art(id, "library_600x900.jpg")), pr, Some(page(id)));
        v["thumb"] = i["tiny_image"].clone();
        v["linux"] = json!(i["platforms"]["linux"] == true);
        if let Some(m) = i["metascore"].as_str().filter(|m| !m.is_empty()) { v["metascore"] = json!(m); }
        Some(v)
    }).collect()
}

pub fn search(q: &str) -> Result<Vec<Value>, String> {
    let v = fetch("steam", "search", "GET", &format!("{API}/api/storesearch/?term={}&l=english&cc={}", crate::http::urlencode(q), cc()), None, &[])?;
    Ok(parse_search(&v))
}

/// appdetails for one app: the game's page.
pub fn parse_details(id: &str, v: &Value) -> Result<Value, String> {
    let e = &v[id];
    if e["success"] != true || !e["data"].is_object() { return Err("Steam doesn't have a store page for this game in your country.".into()); }
    let d = &e["data"];
    let n: u64 = id.parse().map_err(|_| "Not a Steam game".to_string())?;
    let names = |k: &str| -> Vec<String> { d[k].as_array().into_iter().flatten().filter_map(|x| x.as_str().or(x["description"].as_str()).map(str::to_string)).collect() };
    let po = &d["price_overview"];
    let pr = if d["is_free"] == true { price("", None, 0, true) }
        else if po.is_object() {
            let cur = po["currency"].as_str().unwrap_or("");
            let fin = po["final_formatted"].as_str().filter(|s| !s.is_empty()).map(str::to_string).unwrap_or_else(|| money(po["final"].as_i64().unwrap_or(0), cur));
            let base = po["initial_formatted"].as_str().filter(|s| !s.is_empty()).map(str::to_string).or_else(|| po["initial"].as_i64().map(|i| money(i, cur)));
            price(&fin, base.as_deref(), po["discount_percent"].as_u64().unwrap_or(0), false)
        } else { Value::Null };
    let mut rating = Value::Null;
    if let Some(s) = d["metacritic"]["score"].as_u64() { rating = json!({"label": format!("Metacritic {s}"), "score": s}); }
    let recs = d["recommendations"]["total"].as_u64();
    let shots: Vec<Value> = d["screenshots"].as_array().into_iter().flatten().filter_map(|s| s["path_full"].as_str().map(|u| json!(u))).take(12).collect();
    let about = d["about_the_game"].as_str().or(d["detailed_description"].as_str()).unwrap_or("");
    let mut text = html_text(about, 40);
    if text.is_empty() { text = html_text(d["short_description"].as_str().unwrap_or(""), 5); }
    Ok(json!({
        "store": "steam", "id": id, "title": d["name"].as_str().unwrap_or("").trim(), "kind": d["type"],
        "wide": d["header_image"], "tall": art(n, "library_600x900.jpg"),
        "hero": d["background_raw"].as_str().or(d["background"].as_str()).or(d["header_image"].as_str()),
        "short": html_text(d["short_description"].as_str().unwrap_or(""), 3).join(" "),
        "description": text, "screenshots": shots,
        "developers": names("developers"), "publishers": names("publishers"), "genres": names("genres"),
        "released": d["release_date"]["date"], "comingSoon": d["release_date"]["coming_soon"] == true,
        "price": pr, "rating": rating, "recommendations": recs,
        "linux": d["platforms"]["linux"] == true, "controller": d["controller_support"],
        "age": d["required_age"].as_u64().or_else(|| d["required_age"].as_str().and_then(|s| s.parse().ok())).unwrap_or(0),
        "url": page(n), "steamStore": format!("steam://store/{n}"), "steamInstall": format!("steam://install/{n}"),
    }))
}

pub fn details(id: &str) -> Result<Value, String> {
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_digit()) { return Err("Not a Steam game".into()); }
    let v = fetch("steam", "details", "GET", &format!("{API}/api/appdetails?appids={id}&cc={}&l=english", cc()), None, &[])?;
    let mut d = parse_details(id, &v)?;
    // How it runs on Linux (ProtonDB's rating, the same one the library's game pages show; kept a few days).
    d["protondb"] = crate::gameinfo::protondb(&format!("steam:{id}"), "", "steam");
    Ok(d)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fx(name: &str) -> Value {
        serde_json::from_str(match name {
            "featured" => include_str!("fixtures/steam_featured.json"),
            "search" => include_str!("fixtures/steam_search.json"),
            "details" => include_str!("fixtures/steam_details.json"),
            _ => unreachable!(),
        }).unwrap()
    }

    #[test]
    fn featured_lists() {
        let s = parse_categories(&fx("featured"));
        assert_eq!(s.len(), 2);
        assert_eq!(s[0]["id"], "deals");
        let cp = &s[0]["items"][0];
        assert_eq!(cp["id"], "1091500");
        assert_eq!(cp["title"], "Cyberpunk 2077");
        assert_eq!(cp["price"], json!({"final": "$17.99", "base": "$59.99", "discount": 70, "free": false}));
        assert_eq!(cp["linux"], false);
        assert_eq!(cp["url"], "https://store.steampowered.com/app/1091500");
        assert!(cp["wide"].as_str().unwrap().starts_with("https://shared.akamai.steamstatic.com/"));
        // Top sellers lists a game twice: once here.
        let top: Vec<&str> = s[1]["items"].as_array().unwrap().iter().map(|i| i["id"].as_str().unwrap()).collect();
        assert_eq!(top, vec!["4435490", "4165910"]);
        assert_eq!(s[1]["items"][0]["price"]["base"], Value::Null);   // full price: no "before"
        assert_eq!(s[1]["items"][1]["linux"], true);
        // Coming soon's "0, no price": no price yet, not free.
        let soon = list_item(&fx("featured")["coming_soon"]["items"][0], false).unwrap();
        assert_eq!(soon["price"], Value::Null);
        let new = list_item(&fx("featured")["new_releases"]["items"][0], true).unwrap();
        assert_eq!(new["price"]["free"], true);
        assert_eq!(new["price"]["final"], "Free");
        // The same item shape from /api/featured.
        assert_eq!(parse_featured(&json!({"featured_win": fx("featured")["specials"]["items"]}))[0]["items"][2]["title"], "Enshrouded");
    }

    #[test]
    fn popular_new_from_the_search_page() {
        let v = json!({"desc": "", "items": [
            {"name": "Some Game", "logo": "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/2050650/abc/capsule_231x87.jpg?t=1"},
            {"name": "Twice", "logo": "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/2050650/x.jpg"},
            {"name": "No id", "logo": "https://example.com/a.jpg"}]});
        let s = parse_popular_new(&v);
        assert_eq!(s[0]["items"].as_array().unwrap().len(), 1);
        assert_eq!(s[0]["items"][0]["id"], "2050650");
        assert_eq!(s[0]["items"][0]["price"], Value::Null);
        assert!(s[0]["items"][0]["thumb"].as_str().unwrap().contains("capsule_231x87"));
    }

    #[test]
    fn search_results() {
        let r = parse_search(&fx("search"));
        assert_eq!(r.len(), 3);
        assert_eq!(r[0]["id"], "1145350");
        assert_eq!(r[0]["title"], "Hades II");
        assert_eq!(r[0]["price"]["final"], "$29.99");
        assert_eq!(r[0]["metascore"], "94");
        assert_eq!(r[1]["price"]["final"], "$24.99");
        assert_eq!(r[2]["price"], Value::Null);   // Hades' Star: free to play, no price in search
        assert!(r[0]["thumb"].as_str().unwrap().starts_with("https://"));
    }

    #[test]
    fn game_page() {
        let d = parse_details("1145360", &fx("details")).unwrap();
        assert_eq!(d["title"], "Hades");
        assert_eq!(d["price"], json!({"final": "$24.99", "base": null, "discount": 0, "free": false}));
        assert_eq!(d["rating"]["label"], "Metacritic 93");
        assert_eq!(d["recommendations"], 287452);
        assert_eq!(d["developers"], json!(["Supergiant Games"]));
        assert_eq!(d["genres"], json!(["Action", "Indie", "RPG"]));
        assert_eq!(d["released"], "Sep 17, 2020");
        assert_eq!(d["screenshots"].as_array().unwrap().len(), 3);
        assert_eq!(d["linux"], false);
        assert_eq!(d["steamStore"], "steam://store/1145360");
        assert_eq!(d["steamInstall"], "steam://install/1145360");
        let text = d["description"].as_array().unwrap();
        assert!(!text.is_empty() && text.iter().all(|p| !p.as_str().unwrap().contains('<')), "{text:?}");
        assert!(d["short"].as_str().unwrap().starts_with("Defy the god of the dead"));
        // Not on sale here, or not a game Steam knows.
        assert!(parse_details("1", &json!({"1": {"success": false}})).is_err());
        assert!(parse_details("1145360", &json!({})).is_err());
        assert!(details("12a").is_err() && details("").is_err());
    }
}
