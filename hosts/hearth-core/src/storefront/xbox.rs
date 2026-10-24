//! Xbox PC games: the Microsoft Store and PC Game Pass, from public catalog endpoints: xbox.com's browse lists
//! (emerald.xboxservices.com), PC Game Pass's lists (catalog.gamepass.com, the same catalog Hearth's Xbox Cloud
//! Gaming entry reads) and the Microsoft Store's search and product pages (storeedgefd.dsx.mp.microsoft.com).
//! Buying happens on xbox.com in Hearth's store window, signed in with the same browser profile as Library ›
//! Stores' Microsoft (Xbox) sign-in; Hearth installs Xbox PC games through Xodus Nexus.

use super::{fetch, html_text, https, item, locale, money, price, section};
use serde_json::{json, Value};

const EMERALD: &str = "https://emerald.xboxservices.com/xboxcomfd/browse";
const GAMEPASS: &str = "https://catalog.gamepass.com";
const EDGE: &str = "https://storeedgefd.dsx.mp.microsoft.com/v9.0";

/// PC Game Pass lists: (id, title, list id).
const PASS_LISTS: [(&str, &str, &str); 2] = [("pass", "Popular with PC Game Pass", "a884932a-f02b-40c8-a903-a008c23b1df1"),
                                             ("passNew", "New to PC Game Pass", "163cdff5-442e-4957-97f5-1050a3546511")];

fn lang_region() -> String { let (m, l) = locale(); format!("{}-{}", &l[..2.min(l.len())], m) }
pub fn page(title: &str, id: &str) -> String {
    format!("https://www.xbox.com/{}/games/store/{}/{id}", lang_region(), crate::stores::xcloud::slug(title))
}

/// A price in a currency's units (19.99) as text.
fn amount(v: f64, cur: &str) -> String { money((v * 100.0).round() as i64, cur) }

/// xbox.com's browse filters for a list: base64 of {"orderby": {...}, "PlayWith": PC, ...}.
/// Written out in xbox.com's own order (serde_json would sort the keys).
pub fn browse_filter(order: &str, on_sale: bool) -> String {
    let q = |s: &str| serde_json::to_string(s).unwrap_or_default();
    let sale = if on_sale { r#","Price":{"id":"Price","choices":[{"id":"OnSale"}]}"# } else { "" };
    let f = format!(r#"{{"orderby":{{"id":"orderby","choices":[{{"id":{}}}]}},"PlayWith":{{"id":"PlayWith","choices":[{{"id":"PC"}}]}}{sale}}}"#, q(order));
    base64(f.as_bytes())
}
fn base64(b: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut o = String::new();
    for c in b.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        o.push(T[(n >> 18) as usize & 63] as char);
        o.push(T[(n >> 12) as usize & 63] as char);
        o.push(if c.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        o.push(if c.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    o
}

/// A game from xbox.com's browse list.
pub fn browse_item(p: &Value) -> Option<Value> {
    let id = p["productId"].as_str()?;
    let title = p["title"].as_str()?;
    let img = |k: &str| p["images"][k]["url"].as_str().map(https);
    let pr = p["specificPrices"]["purchaseable"].as_array().and_then(|a| a.first()).map(|s| {
        let cur = s["currency"].as_str().unwrap_or("");
        let (list, msrp) = (s["listPrice"].as_f64().unwrap_or(0.0), s["msrp"].as_f64().unwrap_or(0.0));
        let disc = s["discountPercentage"].as_f64().unwrap_or(0.0).round() as u64;
        price(&amount(list, cur), Some(&amount(msrp, cur)), disc, list == 0.0)
    }).unwrap_or(Value::Null);
    let mut v = item("xbox", id, title, img("superHeroArt").or_else(|| img("boxArt")), img("poster").or_else(|| img("boxArt")), pr, Some(page(title, id)));
    v["gamePass"] = json!(p["includedWithPassesProductIds"].as_array().map(|a| !a.is_empty()).unwrap_or(false));
    v["pc"] = json!(p["availableOn"].as_array().map(|a| a.iter().any(|x| x == "PC")).unwrap_or(false));
    if let Some(r) = p["averageRating"].as_f64().filter(|r| *r > 0.0) { v["rating"] = json!({"label": format!("{r:.1} / 5"), "score": (r * 20.0).round() as u64, "count": p["ratingCount"]}); }
    v["publishers"] = json!(p["publisherName"].as_str().map(|s| vec![s]).unwrap_or_default());
    Some(v)
}
pub fn parse_browse(v: &Value) -> Vec<Value> {
    v["productSummaries"].as_array().into_iter().flatten().filter_map(browse_item).take(24).collect()
}
fn browse(order: &str, on_sale: bool) -> Result<Vec<Value>, String> {
    let f = browse_filter(order, on_sale);
    let body = json!({"Filters": f, "ReturnFilters": false, "ChannelKeyToBeUsedInResponse": format!("BROWSE_CHANNELID=_FILTERS={f}"), "EncodedFilters": f, "ChannelId": ""});
    let v = fetch("xbox", "home", "POST", &format!("{EMERALD}?locale={}", lang_region()), Some(&body.to_string()),
        // xbox.com's service wants these two (a correlation id, its API version), as its own page sends them.
        &[("x-ms-api-version", "1.1"), ("ms-cv", "cv"), ("calling-app-name", "Hearth"), ("calling-app-version", "1")])?;
    Ok(parse_browse(&v))
}

/// The ids in a PC Game Pass list (its first entry describes the list itself).
pub fn parse_sigl(v: &Value) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    v.as_array().into_iter().flatten().filter_map(|x| x["id"].as_str().map(str::to_string)).filter(|i| seen.insert(i.clone())).collect()
}
/// A game from catalog.gamepass.com's products (with "RemoteHighSapphire0", the Xbox app's own detail level).
pub fn pass_item(id: &str, p: &Value) -> Option<Value> {
    let title = p["ProductTitle"].as_str()?;
    let img = |k: &str| p[k]["URL"].as_str().map(https);
    let ap = &p["AnonymousPrice"];
    let cur = ap["MSRP"]["currencyCode"].as_str().unwrap_or("");
    let pr = match (ap["MSRP"]["value"].as_f64(), ap["SalePrice"]["value"].as_f64()) {
        (Some(m), Some(s)) if s < m => price(&amount(s, cur), Some(&amount(m, cur)), (((m - s) / m) * 100.0).round() as u64, s == 0.0),
        (Some(m), _) => price(&amount(m, cur), None, 0, m == 0.0),
        _ => Value::Null,
    };
    let mut v = item("xbox", id, title, img("Image_Hero").or_else(|| img("Image_TitledHero")).or_else(|| img("Image_Tile")),
        img("Image_Poster").or_else(|| img("Image_BoxArt")), pr, Some(page(title, id)));
    v["gamePass"] = json!(p["PassMetadataByPassProductId"].as_object().map(|o| !o.is_empty()).unwrap_or(true));
    v["pc"] = json!(p["AvailablePlatforms"].as_array().map(|a| a.iter().any(|x| x == "PC")).unwrap_or(true));
    v["titleId"] = p["XboxTitleId"].clone();
    v["publishers"] = json!(p["PublisherName"].as_str().map(|s| vec![s]).unwrap_or_default());
    Some(v)
}
pub fn parse_pass_products(ids: &[String], v: &Value) -> Vec<Value> {
    ids.iter().filter_map(|id| pass_item(id, &v["Products"][id.as_str()])).collect()
}
fn pass_list(list: &str) -> Result<Vec<Value>, String> {
    let (market, lang) = locale();
    let ids: Vec<String> = parse_sigl(&fetch("xbox", "home", "GET", &format!("{GAMEPASS}/sigls/v2?id={list}&market={market}&language={lang}"), None, &[])?).into_iter().take(24).collect();
    if ids.is_empty() { return Ok(vec![]); }
    let v = fetch("xbox", "home", "POST", &format!("{GAMEPASS}/v3/products?market={market}&language={lang}&hydration=RemoteHighSapphire0"),
        Some(&json!({"Products": ids}).to_string()), &[("ms-cv", "cv"), ("calling-app-name", "Hearth"), ("calling-app-version", "1")])?;
    Ok(parse_pass_products(&ids, &v))
}

pub fn home() -> Result<Vec<Value>, String> {
    let mut out = Vec::new();
    let mut first_err = None;
    let mut add = |r: Result<Vec<Value>, String>, id: &str, title: &str| match r {
        Ok(items) => out.push(section(id, title, items)),
        Err(e) => { first_err.get_or_insert(e); }
    };
    add(browse("MostPopular desc", false), "top", "Most popular on PC");
    add(browse("DiscountPercentage desc", true), "deals", "Deals");
    add(browse("ReleaseDate desc", false), "new", "New releases");
    for (id, title, list) in PASS_LISTS { add(pass_list(list), id, title); }
    if out.is_empty() { return Err(first_err.unwrap_or_else(|| "The Microsoft Store didn't answer".into())); }
    Ok(out)
}

fn edge_image(r: &Value, list: &str, kinds: &[&str]) -> Option<String> {
    let imgs = r[list].as_array()?;
    kinds.iter().find_map(|k| imgs.iter().find(|i| i["ImageType"].as_str().map(|t| t.eq_ignore_ascii_case(k)).unwrap_or(false)).and_then(|i| i["Url"].as_str()).map(https))
}
/// The Microsoft Store's price for a product (search results and product pages): the default sale price against
/// the list price, and whether Game Pass includes it.
fn edge_price(r: &Value) -> (Value, bool) {
    let sku = r["SkusSummary"].as_array().and_then(|a| a.iter().find(|s| s["MSRP"].is_number())).cloned().unwrap_or(Value::Null);
    let sales = sku["SalePrices"].as_array().cloned().unwrap_or_default();
    let pass = r["PlaintextPassName"].as_str().map(|s| !s.is_empty()).unwrap_or(false) || sales.iter().any(|s| s["BadgeId"] == "gamepass");
    let Some(msrp) = sku["MSRP"].as_f64() else { return (Value::Null, pass) };
    let def = sales.iter().find(|s| s["Conditions"].is_null() && s["BadgeId"].as_str().unwrap_or("default") == "default");
    let now = def.and_then(|s| s["Price"].as_f64()).unwrap_or(msrp);
    let shown = def.and_then(|s| s["DisplayPrice"].as_str()).map(str::to_string);
    let base = sku["DisplayMSRP"].as_str().map(str::to_string);
    let disc = if msrp > 0.0 && now < msrp { (((msrp - now) / msrp) * 100.0).round() as u64 } else { 0 };
    let fin = shown.unwrap_or_else(|| format!("{now:.2}"));
    (price(&fin, base.as_deref(), disc, now == 0.0), pass)
}
pub fn edge_item(r: &Value) -> Option<Value> {
    let id = r["ProductId"].as_str()?;
    let title = r["Title"].as_str()?;
    if r["ProductFamilyName"].as_str().map(|f| f != "Games").unwrap_or(false) { return None; }
    let (pr, pass) = edge_price(r);
    let mut v = item("xbox", id, title, edge_image(r, "Previews", &["SuperHeroArt", "hero"]).or_else(|| edge_image(r, "Images", &["hero", "SuperHeroArt", "BoxArt"])),
        edge_image(r, "Images", &["Poster", "BoxArt"]), pr, Some(page(title, id)));
    v["gamePass"] = json!(pass);
    v["pc"] = json!(r["AvailableDevicesNarratorText"].as_str().map(|t| t.contains("PC")).unwrap_or(true));
    if let Some(a) = r["AverageRating"].as_f64().filter(|a| *a > 0.0) { v["rating"] = json!({"label": format!("{a:.1} / 5"), "score": (a * 20.0).round() as u64}); }
    let pfns: Vec<&str> = r["PackageFamilyNames"].as_array().into_iter().flatten().filter_map(|p| p.as_str()).filter(|p| !p.is_empty()).collect();
    v["pfn"] = json!(pfns.first());
    v["publishers"] = json!(r["PublisherName"].as_str().map(|s| vec![s]).unwrap_or_default());
    Some(v)
}
pub fn parse_search(v: &Value) -> Vec<Value> {
    v["Payload"]["SearchResults"].as_array().into_iter().flatten().filter_map(edge_item).collect()
}
pub fn search(q: &str) -> Result<Vec<Value>, String> {
    let (market, _) = locale();
    let v = fetch("xbox", "search", "GET", &format!("{EDGE}/search?market={market}&locale={}&deviceFamily=Windows.Desktop&query={}&mediaType=games", lang_region(), crate::http::urlencode(q)), None, &[])?;
    Ok(parse_search(&v))
}

/// The Microsoft Store's product page.
pub fn parse_details(v: &Value) -> Result<Value, String> {
    let p = &v["Payload"];
    let mut d = edge_item(p).ok_or("The Microsoft Store doesn't have this game")?;
    let shots: Vec<Value> = p["Images"].as_array().into_iter().flatten().filter(|i| i["ImageType"].as_str().map(|t| t.eq_ignore_ascii_case("screenshot")).unwrap_or(false))
        .filter_map(|i| i["Url"].as_str().map(|u| json!(https(u)))).take(12).collect();
    let desc = p["Description"].as_str().unwrap_or("");
    // Plain text with line breaks (sometimes with a little HTML): paragraphs either way.
    let text = html_text(&desc.replace("\r\n", "\n").replace('\n', "<br>"), 40);
    d["hero"] = json!(edge_image(p, "Images", &["hero", "SuperHeroArt", "BoxArt"]));
    if d["wide"].is_null() { d["wide"] = d["hero"].clone(); }
    d["short"] = json!(p["ShortDescription"].as_str().filter(|s| !s.is_empty()).map(str::to_string).or_else(|| text.first().cloned()));
    d["description"] = json!(text);
    d["screenshots"] = json!(shots);
    d["developers"] = json!(p["DeveloperName"].as_str().filter(|s| !s.is_empty()).map(|s| vec![s]).unwrap_or_default());
    d["genres"] = json!(p["Categories"].as_array().cloned().unwrap_or_default());
    d["released"] = json!(p["ReleaseDateUtc"].as_str().map(|s| s.chars().take(10).collect::<String>()));
    if let Some(n) = p["RatingCount"].as_u64() { if d["rating"].is_object() { d["rating"]["count"] = json!(n); } }
    Ok(d)
}
pub fn details(id: &str) -> Result<Value, String> {
    if !id.chars().all(|c| c.is_ascii_alphanumeric()) { return Err("Not an Xbox game".into()); }
    let (market, _) = locale();
    let v = fetch("xbox", "details", "GET", &format!("{EDGE}/products/{id}?market={market}&locale={}&deviceFamily=Windows.Desktop", lang_region()), None, &[])?;
    let mut d = parse_details(&v)?;
    // Xbox PC games install through Xodus Nexus (Library › Stores): whether it's set up.
    d["xodus"] = json!(crate::xodus::installed());
    Ok(d)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fx(name: &str) -> Value {
        serde_json::from_str(match name {
            "popular" => include_str!("fixtures/xbox_browse_popular.json"),
            "deals" => include_str!("fixtures/xbox_browse_deals.json"),
            "sigl" => include_str!("fixtures/xbox_sigl.json"),
            "products" => include_str!("fixtures/xbox_products.json"),
            "search" => include_str!("fixtures/xbox_search.json"),
            "pdp" => include_str!("fixtures/xbox_pdp.json"),
            _ => unreachable!(),
        }).unwrap()
    }

    #[test]
    fn browse_lists() {
        let p = parse_browse(&fx("popular"));
        assert_eq!(p.len(), 2);
        assert_eq!(p[0]["id"], "9PMF91N3LZ3M");
        assert_eq!(p[0]["price"]["final"], "Free");
        assert_eq!(p[0]["pc"], true);
        assert_eq!(p[0]["gamePass"], false);
        assert_eq!(p[1]["gamePass"], true);   // Minecraft Launcher: in PC Game Pass
        assert_eq!(p[0]["rating"]["label"], "4.0 / 5");
        assert!(p[0]["url"].as_str().unwrap().ends_with("/games/store/roblox-windows/9PMF91N3LZ3M"));
        let d = parse_browse(&fx("deals"));
        assert_eq!(d[0]["title"], "Tomb Raider: Definitive Edition");
        assert_eq!(d[0]["price"], json!({"final": "$1.99", "base": "$19.99", "discount": 90, "free": false}));
        assert!(d[0]["wide"].as_str().unwrap().starts_with("https://store-images.s-microsoft.com/"));
    }

    #[test]
    fn browse_filters_are_xboxcoms() {
        // The same encoding xbox.com's own browse page sends.
        assert_eq!(base64(b"{}"), "e30=");
        assert_eq!(base64(b"ab"), "YWI=");
        assert_eq!(base64(b"abc"), "YWJj");
        let f = browse_filter("MostPopular desc", false);
        assert_eq!(f, "eyJvcmRlcmJ5Ijp7ImlkIjoib3JkZXJieSIsImNob2ljZXMiOlt7ImlkIjoiTW9zdFBvcHVsYXIgZGVzYyJ9XX0sIlBsYXlXaXRoIjp7ImlkIjoiUGxheVdpdGgiLCJjaG9pY2VzIjpbeyJpZCI6IlBDIn1dfX0=");
        assert_eq!(browse_filter("DiscountPercentage desc", true), "eyJvcmRlcmJ5Ijp7ImlkIjoib3JkZXJieSIsImNob2ljZXMiOlt7ImlkIjoiRGlzY291bnRQZXJjZW50YWdlIGRlc2MifV19LCJQbGF5V2l0aCI6eyJpZCI6IlBsYXlXaXRoIiwiY2hvaWNlcyI6W3siaWQiOiJQQyJ9XX0sIlByaWNlIjp7ImlkIjoiUHJpY2UiLCJjaG9pY2VzIjpbeyJpZCI6Ik9uU2FsZSJ9XX19");
    }

    #[test]
    fn game_pass_lists() {
        let ids = parse_sigl(&fx("sigl"));
        assert_eq!(ids, vec!["9N4PT8HGCDHQ", "9NXP44L49SHJ", "9PB301R051CQ"]);
        let ids = vec!["9NPDN9R45JX4".to_string(), "MISSING".to_string(), "9P8LR42PTRGJ".to_string()];
        let l = parse_pass_products(&ids, &fx("products"));
        assert_eq!(l.len(), 2);
        assert_eq!(l[0]["title"], "1000xRESIST");
        assert_eq!(l[0]["price"]["final"], "$19.99");
        assert_eq!(l[0]["gamePass"], true);
        assert_eq!(l[0]["titleId"], "1615431074");
        assert!(l[0]["wide"].as_str().unwrap().starts_with("https://store-images.s-microsoft.com/"));
        assert_eq!(l[1]["title"], "33 Immortals");
    }

    #[test]
    fn search_and_game_page() {
        let s = parse_search(&fx("search"));
        assert_eq!(s.len(), 3);
        assert_eq!(s[0]["title"], "Halo: Combat Evolved Anniversary");
        assert_eq!(s[0]["price"]["final"], "$9.99");
        assert_eq!(s[0]["gamePass"], true);
        assert_eq!(s[0]["pfn"], Value::Null);
        assert_eq!(s[1]["price"]["final"], "Free");   // Halo Infinite
        assert_eq!(s[1]["pfn"], "Microsoft.254428597CFE2_8wekyb3d8bbwe");
        assert_eq!(s[1]["pc"], true);
        let d = parse_details(&fx("pdp")).unwrap();
        assert_eq!(d["id"], "9NWQMVSB63N4");
        assert_eq!(d["developers"], json!(["343 Industries, Bungie"]));
        assert_eq!(d["genres"], json!(["Shooter"]));
        assert_eq!(d["released"], "2020-03-03");
        assert_eq!(d["rating"]["count"], 2032);
        assert!(!d["screenshots"].as_array().unwrap().is_empty());
        assert!(d["description"][0].as_str().unwrap().starts_with("Halo: Combat Evolved Anniversary"));
        assert!(parse_details(&json!({"Payload": {}})).is_err());
        assert!(details("../x").is_err());
    }
}
