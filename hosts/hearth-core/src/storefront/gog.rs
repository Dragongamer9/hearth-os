//! GOG's store, from its public catalog (catalog.gog.com: lists, prices, search) and product API (api.gog.com:
//! descriptions, screenshots). Buying and wishlists happen on gog.com in Hearth's store window, signed in with the
//! same browser profile as Library › Stores' GOG sign-in.

use super::{fetch, html_text, https, item, locale, price, section};
use serde_json::{json, Value};

const CATALOG: &str = "https://catalog.gog.com/v1/catalog";

fn query(extra: &str) -> String {
    let (market, lang) = locale();
    let l = format!("{}-{}", &lang[..2.min(lang.len())], market);
    // GOG prices in the country's own currency; the US one is named (others: GOG picks the country's).
    let cur = if market == "US" { "&currencyCode=USD" } else { "" };
    format!("{CATALOG}?limit=24&productType=in:game,pack&countryCode={market}&locale={l}{cur}&{extra}")
}

/// A product from the catalog: price ("-60%" as 60), rating (out of 5), Linux build, pictures.
pub fn product(p: &Value) -> Option<Value> {
    let id = p["id"].as_str().map(str::to_string).or_else(|| p["id"].as_u64().map(|n| n.to_string()))?;
    let title = p["title"].as_str()?;
    let pr = &p["price"];
    let pv = if pr.is_object() {
        let disc: u64 = pr["discount"].as_str().unwrap_or("").trim_start_matches('-').trim_end_matches('%').trim().parse().unwrap_or(0);
        let free = pr["finalMoney"]["amount"].as_str().map(|a| a.trim_start_matches(['0', '.']).is_empty()).unwrap_or(false);
        price(pr["final"].as_str().unwrap_or(""), pr["base"].as_str(), disc, free)
    } else { Value::Null };   // not for sale on its own (coming soon, or in a bundle only)
    let slug = p["slug"].as_str().unwrap_or("");
    let url = p["storeLink"].as_str().map(str::to_string).or_else(|| (!slug.is_empty()).then(|| format!("https://www.gog.com/en/game/{slug}")));
    let mut v = item("gog", &id, title, p["coverHorizontal"].as_str().map(https), p["coverVertical"].as_str().map(https), pv, url);
    v["slug"] = json!(slug);
    let os: Vec<&str> = p["operatingSystems"].as_array().into_iter().flatten().filter_map(|o| o.as_str()).collect();
    v["linux"] = json!(os.contains(&"linux"));
    if let Some(r) = p["reviewsRating"].as_u64().filter(|r| *r > 0) { v["rating"] = json!({"label": format!("{:.1} / 5", r as f64 / 10.0), "score": r * 2}); }
    v["genres"] = json!(p["genres"].as_array().into_iter().flatten().filter_map(|g| g["name"].as_str()).collect::<Vec<_>>());
    v["developers"] = p["developers"].clone();
    v["publishers"] = p["publishers"].clone();
    v["released"] = json!(p["releaseDate"].as_str().map(|d| d.replace('.', "-")));
    v["screenshots"] = json!(p["screenshots"].as_array().into_iter().flatten().filter_map(|s| s.as_str()).map(|s| shot(s)).take(12).collect::<Vec<_>>());
    Some(v)
}
/// A screenshot address from GOG's template ("…_{formatter}.jpg"), in its large size.
fn shot(template: &str) -> String { https(&template.replace("{formatter}", "ggvgl")) }

pub fn parse_catalog(v: &Value) -> Vec<Value> {
    v["products"].as_array().into_iter().flatten().filter_map(product).collect()
}

pub fn home() -> Result<Vec<Value>, String> {
    let lists = [("trending", "Trending", "order=desc:trending"), ("top", "Bestsellers", "order=desc:bestselling"),
                 ("deals", "Deals", "order=desc:discount&discounted=eq:true"), ("new", "New on GOG", "order=desc:storeReleaseDate")];
    let mut out = Vec::new();
    let mut first_err = None;
    for (id, title, q) in lists {
        match fetch("gog", "home", "GET", &query(q), None, &[]) {
            Ok(v) => out.push(section(id, title, parse_catalog(&v))),
            Err(e) => { first_err.get_or_insert(e); }
        }
    }
    if out.is_empty() { return Err(first_err.unwrap_or_else(|| "GOG didn't answer".into())); }
    Ok(out)
}

pub fn search(q: &str) -> Result<Vec<Value>, String> {
    let v = fetch("gog", "search", "GET", &query(&format!("order=desc:score&query=like:{}", crate::http::urlencode(q))), None, &[])?;
    Ok(parse_catalog(&v))
}

/// The game's page: api.gog.com's product (description, screenshots, links) with the catalog's entry for it, when
/// one was found (price, rating, genres, makers).
pub fn parse_details(p: &Value, listed: Option<&Value>) -> Result<Value, String> {
    let id = p["id"].as_u64().map(|n| n.to_string()).or_else(|| p["id"].as_str().map(str::to_string)).ok_or("GOG doesn't have this game")?;
    let title = p["title"].as_str().unwrap_or("").trim().to_string();
    let slug = p["slug"].as_str().unwrap_or("");
    let shots: Vec<Value> = p["screenshots"].as_array().into_iter().flatten()
        .filter_map(|s| s["formatter_template_url"].as_str().map(|t| json!(shot(t)))).take(12).collect();
    let mut text = html_text(p["description"]["full"].as_str().unwrap_or(""), 40);
    if text.is_empty() { text = html_text(p["description"]["lead"].as_str().unwrap_or(""), 10); }
    let url = listed.and_then(|l| l["url"].as_str().map(str::to_string)).unwrap_or_else(|| format!("https://www.gog.com/en/game/{slug}"));
    let l = |k: &str| listed.map(|l| l[k].clone()).unwrap_or(Value::Null);
    let shots = if shots.is_empty() { l("screenshots") } else { json!(shots) };
    Ok(json!({
        "store": "gog", "id": id, "title": title, "slug": slug,
        "wide": l("wide"), "tall": l("tall"), "hero": p["images"]["background"].as_str().map(https),
        "short": html_text(p["description"]["lead"].as_str().unwrap_or(""), 3).join(" "),
        "description": text, "screenshots": shots,
        "developers": l("developers"), "publishers": l("publishers"), "genres": l("genres"),
        "released": p["release_date"].as_str().map(|d| d.chars().take(10).collect::<String>()),
        "price": l("price"), "rating": l("rating"),
        "linux": p["content_system_compatibility"]["linux"] == true,
        "url": url,
        // Not for sale on its own anymore (a bundle replaced it, say): the page says so.
        "forSale": listed.is_some(),
    }))
}

pub fn details(id: &str) -> Result<Value, String> {
    if !id.chars().all(|c| c.is_ascii_digit()) { return Err("Not a GOG game".into()); }
    let p = fetch("gog", "details", "GET", &format!("https://api.gog.com/products/{id}?expand=description,screenshots"), None, &[])?;
    // The catalog has the price and rating; it's looked up by the game's name.
    let title = p["title"].as_str().unwrap_or("");
    let listed = if title.is_empty() { None } else {
        fetch("gog", "details", "GET", &query(&format!("order=desc:score&query=like:{}", crate::http::urlencode(title))), None, &[]).ok()
            .and_then(|c| parse_catalog(&c).into_iter().find(|x| x["id"] == id))
    };
    parse_details(&p, listed.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fx(name: &str) -> Value {
        serde_json::from_str(match name {
            "catalog" => include_str!("fixtures/gog_catalog.json"),
            "deals" => include_str!("fixtures/gog_deals.json"),
            "product" => include_str!("fixtures/gog_product.json"),
            _ => unreachable!(),
        }).unwrap()
    }

    #[test]
    fn catalog_lists() {
        let l = parse_catalog(&fx("catalog"));
        assert_eq!(l.len(), 2);
        let c = &l[0];
        assert_eq!(c["id"], "1274966284");
        assert_eq!(c["title"], "Cyberpunk 2077: Ultimate Edition");
        assert_eq!(c["price"], json!({"final": "$31.99", "base": "$79.99", "discount": 60, "free": false}));
        assert_eq!(c["rating"]["label"], "4.3 / 5");
        assert_eq!(c["linux"], false);
        assert_eq!(c["url"], "https://www.gog.com/en/game/cyberpunk_2077_ultimate_edition");
        assert_eq!(c["released"], "2023-09-26");
        assert_eq!(c["genres"][0], "Role-playing");
        assert!(c["wide"].as_str().unwrap().starts_with("https://images.gog-statics.com/"));
        assert!(c["screenshots"][0].as_str().unwrap().ends_with("_ggvgl.jpg"));
        let d = parse_catalog(&fx("deals"));
        assert_eq!(d[0]["price"]["discount"], 95);
        assert_eq!(d[0]["linux"], true);
        // Free games, and ones with no price.
        let free = product(&json!({"id": "1", "title": "F", "price": {"final": "$0.00", "base": "$0.00", "discount": null, "finalMoney": {"amount": "0.00"}}})).unwrap();
        assert_eq!(free["price"]["final"], "Free");
        assert_eq!(product(&json!({"id": "2", "title": "Soon", "price": null})).unwrap()["price"], Value::Null);
        assert!(product(&json!({"title": "No id"})).is_none());
    }

    #[test]
    fn game_page() {
        let listed = json!({"url": "https://www.gog.com/en/game/the_witcher_3_wild_hunt", "price": {"final": "$9.99"}, "rating": {"label": "4.8 / 5"}, "genres": ["Role-playing"]});
        let d = parse_details(&fx("product"), Some(&listed)).unwrap();
        assert_eq!(d["id"], "1207664663");
        assert_eq!(d["title"], "The Witcher 3: Wild Hunt - Complete Edition");
        assert_eq!(d["price"]["final"], "$9.99");
        assert_eq!(d["released"], "2015-05-19");
        assert_eq!(d["linux"], false);
        assert_eq!(d["forSale"], true);
        assert_eq!(d["screenshots"].as_array().unwrap().len(), 2);
        assert!(d["hero"].as_str().unwrap().starts_with("https://images-1.gog-statics.com/"));
        let text = d["description"].as_array().unwrap();
        assert!(text.iter().any(|p| p.as_str().unwrap().contains("RPG epic")), "{text:?}");
        assert!(text.iter().all(|p| !p.as_str().unwrap().contains('<')));
        // Not in the catalog: no price, its own page address.
        let d = parse_details(&fx("product"), None).unwrap();
        assert_eq!(d["price"], Value::Null);
        assert_eq!(d["forSale"], false);
        assert_eq!(d["url"], "https://www.gog.com/en/game/the_witcher_3_wild_hunt");
        assert!(details("12x").is_err());
    }
}
