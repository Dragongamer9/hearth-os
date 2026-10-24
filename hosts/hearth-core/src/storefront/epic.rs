//! The Epic Games Store, from its public store GraphQL (store.epicgames.com/graphql: lists, search, a game's page,
//! its rating) and its free-games feed. Buying happens on store.epicgames.com in Hearth's store window, signed in
//! with the same browser profile as Library › Stores' Epic sign-in.

use super::{fetch, html_text, item, locale, money, price, section};
use serde_json::{json, Value};

const GRAPHQL: &str = "https://store.epicgames.com/graphql";
const FREE: &str = "https://store-site-backend-static-ipv4.ak.epicgames.com/freeGamesPromotions";

/// The fields asked for each game in a list.
const FIELDS: &str = "title id namespace description effectiveDate keyImages{type url} productSlug urlSlug \
    catalogNs{mappings(pageType:\"productHome\"){pageSlug pageType}} offerMappings{pageSlug pageType} seller{name}";

fn country() -> String { locale().0 }
fn lang() -> String { let (m, l) = locale(); format!("{}-{}", &l[..2.min(l.len())], m) }
fn price_fields() -> String {
    format!("price(country:\"{}\"){{totalPrice{{discountPrice originalPrice currencyCode fmtPrice(locale:\"{}\"){{originalPrice discountPrice}}}}}}", country(), lang())
}

/// A GraphQL string (JSON's escaping is GraphQL's).
fn gql_str(s: &str) -> String { serde_json::to_string(s).unwrap_or_else(|_| "\"\"".into()) }

/// What the page says when Epic's store answers with its browser check (Cloudflare) instead of data: Hearth doesn't
/// try to get around it; the page offers Epic's own site in the store window (a real browser) instead.
pub const BROWSER_CHECK: &str = "The Epic Games Store asked for a browser check, so Hearth can't read this part of it right now. Open Epic's store in the store window instead.";

fn ask(kind: &str, query: &str) -> Result<Value, String> {
    let v = fetch("epic", kind, "POST", GRAPHQL, Some(&json!({"query": query}).to_string()), &[])
        .map_err(|e| if e.contains("HTTP 403") { BROWSER_CHECK.to_string() } else { e })?;
    if v["data"].is_null() {
        let why = v["errors"][0]["message"].as_str().unwrap_or("no data");
        return Err(format!("The Epic Games Store didn't answer ({why})"));
    }
    Ok(v)
}

/// The game's page address on the store ("…/p/hades-ii").
pub fn slug_of(e: &Value) -> Option<String> {
    let maps = e["catalogNs"]["mappings"].as_array().into_iter().flatten().chain(e["offerMappings"].as_array().into_iter().flatten());
    let from_map = maps.filter_map(|m| m["pageSlug"].as_str()).find(|s| !s.is_empty()).map(str::to_string);
    from_map.or_else(|| e["productSlug"].as_str().map(|s| s.trim_end_matches("/home").to_string()).filter(|s| !s.is_empty()))
        .or_else(|| e["urlSlug"].as_str().filter(|s| !(s.len() == 32 && s.chars().all(|c| c.is_ascii_hexdigit()))).map(str::to_string))
}
fn image(e: &Value, kinds: &[&str]) -> Option<String> {
    let imgs = e["keyImages"].as_array()?;
    kinds.iter().find_map(|k| imgs.iter().find(|i| i["type"] == *k).and_then(|i| i["url"].as_str()).map(str::to_string))
}
pub fn page(slug: &str) -> String { format!("https://store.epicgames.com/{}/p/{slug}", lang()) }
/// Epic's own checkout for an offer (the store's Buy button goes to the same place).
pub fn checkout(namespace: &str, id: &str) -> String { format!("https://store.epicgames.com/purchase?offers=1-{namespace}-{id}") }

fn price_of(e: &Value) -> Value {
    let t = &e["price"]["totalPrice"];
    if !t.is_object() { return Value::Null; }
    let (d, o) = (t["discountPrice"].as_i64().unwrap_or(0), t["originalPrice"].as_i64().unwrap_or(0));
    let cur = t["currencyCode"].as_str().unwrap_or("");
    let f = &t["fmtPrice"];
    let fin = f["discountPrice"].as_str().filter(|s| *s != "0" && !s.is_empty()).map(str::to_string).unwrap_or_else(|| money(d, cur));
    let base = f["originalPrice"].as_str().filter(|s| *s != "0" && !s.is_empty()).map(str::to_string).unwrap_or_else(|| money(o, cur));
    let disc = if o > d && o > 0 { (((o - d) * 100) as f64 / o as f64).round() as u64 } else { 0 };
    price(&fin, Some(&base), disc, d == 0)
}

/// A game in one of Epic's lists. Its id is "<namespace>:<offer id>" (both are needed for its page and checkout).
pub fn element(e: &Value) -> Option<Value> {
    let (id, ns, title) = (e["id"].as_str()?, e["namespace"].as_str()?, e["title"].as_str()?);
    let slug = slug_of(e);
    let mut v = item("epic", &format!("{ns}:{id}"), title,
        image(e, &["OfferImageWide", "DieselStoreFrontWide", "DieselGameBoxWide", "featuredMedia"]),
        image(e, &["OfferImageTall", "DieselStoreFrontTall", "DieselGameBoxTall", "Thumbnail"]),
        price_of(e), slug.as_deref().map(page));
    v["slug"] = json!(slug);
    v["checkout"] = json!(checkout(ns, id));
    if let Some(s) = e["seller"]["name"].as_str() { v["publishers"] = json!([s]); }
    if let Some(d) = e["description"].as_str().filter(|d| d.trim() != title.trim() && !d.is_empty()) { v["short"] = json!(d); }
    Some(v)
}

pub fn parse_list(v: &Value) -> Vec<Value> {
    v["data"]["Catalog"]["searchStore"]["elements"].as_array().into_iter().flatten().filter_map(element).collect()
}

/// The free-games feed: free now (an offer on now at 100% off; Epic writes 100% off as discountPercentage 0) and
/// free next. `now` is an RFC 3339 time.
pub fn parse_free(v: &Value, now: &str) -> (Vec<Value>, Vec<Value>) {
    let window = |p: &Value| -> Vec<(String, String, u64)> {
        p.as_array().into_iter().flatten().flat_map(|g| g["promotionalOffers"].as_array().cloned().unwrap_or_default())
            .map(|o| (o["startDate"].as_str().unwrap_or("").to_string(), o["endDate"].as_str().unwrap_or("").to_string(), o["discountSetting"]["discountPercentage"].as_u64().unwrap_or(100))).collect()
    };
    let (mut cur, mut next) = (Vec::new(), Vec::new());
    for e in v["data"]["Catalog"]["searchStore"]["elements"].as_array().into_iter().flatten() {
        let Some(mut it) = element(e) else { continue };
        let p = &e["promotions"];
        if let Some((_, end, _)) = window(&p["promotionalOffers"]).into_iter().find(|(s, en, pct)| *pct == 0 && s.as_str() <= now && now < en.as_str()) {
            it["until"] = json!(end);
            it["price"]["final"] = json!("Free");
            it["price"]["free"] = json!(true);
            cur.push(it);
        } else if let Some((start, _, _)) = window(&p["upcomingPromotionalOffers"]).into_iter().find(|(s, _, pct)| *pct == 0 && now < s.as_str()) {
            it["from"] = json!(start);
            next.push(it);
        }
    }
    (cur, next)
}

fn now_rfc3339() -> String { chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string() }

fn list_query(args: &str) -> String {
    format!("{{Catalog{{searchStore(category:\"games/edition/base|bundles/games\",count:24,country:\"{}\",locale:\"{}\",{args}){{elements{{{FIELDS} {}}}}}}}}}",
        country(), lang(), price_fields())
}

pub fn home() -> Result<Vec<Value>, String> {
    let now = now_rfc3339();
    let mut out = Vec::new();
    let mut first_err = None;
    match fetch("epic", "home", "GET", &format!("{FREE}?locale={}&country={c}&allowCountries={c}", lang(), c = country()), None, &[]) {
        Ok(v) => { let (a, b) = parse_free(&v, &now); out.push(section("free", "Free now", a)); out.push(section("freeNext", "Free next", b)); }
        Err(e) => { first_err = Some(e); }
    }
    let lists = [("deals", "On sale", "onSale:true,sortBy:\"relevancy\",sortDir:\"DESC\"".to_string()),
                 ("new", "New releases", format!("sortBy:\"releaseDate\",sortDir:\"DESC\",releaseDate:\"[,{now}]\"")),
                 ("soon", "Coming soon", format!("sortBy:\"releaseDate\",sortDir:\"ASC\",releaseDate:\"[{now},]\""))];
    for (id, title, args) in lists {
        match ask("home", &list_query(&args)) {
            Ok(v) => out.push(section(id, title, parse_list(&v))),
            Err(e) => { first_err.get_or_insert(e); }
        }
    }
    if out.iter().all(|s| s["items"].as_array().map(|a| a.is_empty()).unwrap_or(true)) { return Err(first_err.unwrap_or_else(|| "The Epic Games Store didn't send any games".into())); }
    Ok(out)
}

pub fn search(q: &str) -> Result<Vec<Value>, String> {
    let v = ask("search", &list_query(&format!("keywords:{},sortBy:\"relevancy\",sortDir:\"DESC\"", gql_str(q))))?;
    Ok(parse_list(&v))
}

/// Epic's long descriptions are Markdown with HTML comments between blocks: as text paragraphs.
pub fn markdown_text(md: &str, max: usize) -> Vec<String> {
    let img = regex::Regex::new(r"!\[[^\]]*\]\([^)]*\)").unwrap();
    let link = regex::Regex::new(r"\[([^\]]*)\]\([^)]*\)").unwrap();
    let t = img.replace_all(md, "");
    let t = link.replace_all(&t, "$1");
    let lines: Vec<String> = t.lines().map(|l| {
        let l = l.trim().trim_start_matches('#').trim();
        let l = l.strip_prefix("- ").or_else(|| l.strip_prefix("* ")).map(|r| format!("• {r}")).unwrap_or_else(|| l.to_string());
        l.replace("**", "").replace("__", "")
    }).collect();
    html_text(&lines.join("<br>"), max)
}

pub fn parse_details(v: &Value, rating: Option<f64>) -> Result<Value, String> {
    let o = &v["data"]["Catalog"]["catalogOffer"];
    if !o.is_object() { return Err("The Epic Games Store doesn't have this game".into()); }
    let base = element(o).ok_or("The Epic Games Store doesn't have this game")?;
    let shots: Vec<Value> = o["keyImages"].as_array().into_iter().flatten()
        .filter(|i| matches!(i["type"].as_str(), Some("featuredMedia" | "GalleryImage" | "Screenshot")))
        .filter_map(|i| i["url"].as_str().map(|u| json!(u))).take(12).collect();
    let genres: Vec<&str> = o["tags"].as_array().into_iter().flatten().filter(|t| t["groupName"] == "genre").filter_map(|t| t["name"].as_str()).collect();
    let mut text = markdown_text(o["longDescription"].as_str().unwrap_or(""), 40);
    if text.is_empty() { text = html_text(o["description"].as_str().unwrap_or(""), 5); }
    let names = |k: &str| o[k].as_str().filter(|s| !s.is_empty()).map(|s| json!([s])).unwrap_or(json!([]));
    let mut d = base;
    d["short"] = o["description"].clone();
    d["description"] = json!(text);
    d["screenshots"] = json!(shots);
    d["hero"] = json!(image(o, &["DieselStoreFrontWide", "OfferImageWide", "featuredMedia"]));
    d["developers"] = names("developerDisplayName");
    d["publishers"] = names("publisherDisplayName");
    d["genres"] = json!(genres);
    d["released"] = json!(o["releaseDate"].as_str().map(|s| s.chars().take(10).collect::<String>()));
    if let Some(r) = rating.filter(|r| *r > 0.0) { d["rating"] = json!({"label": format!("{r:.1} / 5"), "score": (r * 20.0).round() as u64}); }
    Ok(d)
}

pub fn details(id: &str) -> Result<Value, String> {
    let (ns, offer) = id.split_once(':').ok_or("Not an Epic game")?;
    let q = format!("{{Catalog{{catalogOffer(namespace:{},id:{},locale:\"{}\"){{{FIELDS} longDescription developerDisplayName publisherDisplayName releaseDate tags{{id name groupName}} {}}}}}}}",
        gql_str(ns), gql_str(offer), lang(), price_fields());
    let v = ask("details", &q)?;
    // The players' rating (out of 5): an extra, the page works without it.
    let rating = ask("details", &format!("{{RatingsPolls{{getProductResult(sandboxId:{},locale:\"{}\"){{averageRating}}}}}}", gql_str(ns), lang())).ok()
        .and_then(|r| r["data"]["RatingsPolls"]["getProductResult"]["averageRating"].as_f64());
    parse_details(&v, rating)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fx(name: &str) -> Value {
        serde_json::from_str(match name {
            "free" => include_str!("fixtures/epic_free.json"),
            "sale" => include_str!("fixtures/epic_sale.json"),
            "search" => include_str!("fixtures/epic_search.json"),
            "offer" => include_str!("fixtures/epic_offer.json"),
            "rating" => include_str!("fixtures/epic_rating.json"),
            _ => unreachable!(),
        }).unwrap()
    }

    #[test]
    fn lists_and_search() {
        let s = parse_list(&fx("search"));
        assert_eq!(s.len(), 2);
        let h = &s[0];
        assert_eq!(h["id"], "36a21c249987423a93f8442cc40a0e33:adbc94588a184ce3a6122a19aae940ae");
        assert_eq!(h["title"], "Hades II");
        assert_eq!(h["slug"], "hades-ii");
        assert!(h["url"].as_str().unwrap().ends_with("/p/hades-ii"));
        assert_eq!(h["checkout"], "https://store.epicgames.com/purchase?offers=1-36a21c249987423a93f8442cc40a0e33-adbc94588a184ce3a6122a19aae940ae");
        assert_eq!(h["price"], json!({"final": "$29.99", "base": null, "discount": 0, "free": false}));
        assert!(h["wide"].as_str().unwrap().starts_with("https://cdn1.epicgames.com/"));
        assert_eq!(s[1]["id"], "min:2ae6edb2223c4f8c97e9839b5b6497bb");   // Hades: an old namespace name
        assert!(h["short"].as_str().unwrap().starts_with("Battle beyond the Underworld"));
        let sale = parse_list(&fx("sale"));
        assert_eq!(sale[0]["price"], json!({"final": "$1.39", "base": "$1.99", "discount": 30, "free": false}));
        assert_eq!(sale[0]["slug"], "backrooms-realms-d21947");   // from catalogNs only
    }

    #[test]
    fn free_games() {
        let (now, next) = parse_free(&fx("free"), "2026-10-24T05:00:00.000Z");
        let names: Vec<&str> = now.iter().map(|g| g["title"].as_str().unwrap()).collect();
        assert_eq!(names, vec!["Sker Ritual"]);   // Eternal Threads is 20% off, not free
        assert_eq!(now[0]["price"]["final"], "Free");
        assert_eq!(now[0]["price"]["base"], "$24.99");
        assert_eq!(now[0]["until"], "2026-10-29T15:00:00.000Z");
        assert!(next.is_empty());   // Ghostrunner 2's coming offer is 20% off
        // After the offer ended: not free now.
        assert!(parse_free(&fx("free"), "2026-10-30T00:00:00.000Z").0.is_empty());
    }

    #[test]
    fn game_page() {
        let r = fx("rating")["data"]["RatingsPolls"]["getProductResult"]["averageRating"].as_f64();
        let d = parse_details(&fx("offer"), r).unwrap();
        assert_eq!(d["title"], "Hades II");
        assert_eq!(d["developers"], json!(["Supergiant Games"]));
        assert_eq!(d["released"], "2024-05-06");
        assert_eq!(d["rating"]["label"], "4.9 / 5");
        assert!(d["genres"].as_array().unwrap().iter().any(|g| g == "Action"));
        assert!(!d["screenshots"].as_array().unwrap().is_empty());
        let text: Vec<&str> = d["description"].as_array().unwrap().iter().map(|p| p.as_str().unwrap()).collect();
        assert!(text.contains(&"BATTLE BEYOND THE UNDERWORLD"), "{text:?}");
        assert!(text.iter().all(|p| !p.contains("<!--") && !p.starts_with('#')));
        assert!(parse_details(&json!({"data": {"Catalog": {"catalogOffer": null}}}), None).is_err());
        assert!(details("no-colon").is_err());
    }

    #[test]
    fn markdown_and_queries() {
        assert_eq!(markdown_text("# Title\n![x](https://a/b.png)\nSee [the site](https://x) **now**\n- one\n<!--c-->", 10), vec!["Title", "See the site now", "• one"]);
        // What's typed goes into the query as a quoted string, never as query text.
        assert_eq!(gql_str("a\" ){x}"), "\"a\\\" ){x}\"");
        assert!(list_query("keywords:\"x\"").starts_with("{Catalog{searchStore("));
        assert_eq!(list_query("x").matches('{').count(), list_query("x").matches('}').count());
    }
}
