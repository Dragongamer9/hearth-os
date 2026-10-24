//! Game Mode › Store: browsing Steam, GOG, Epic and Xbox (Microsoft Store / PC Game Pass) from Hearth, with each
//! store's public, sign-in-free endpoints (store pages, catalogs, prices, pictures). Nothing here signs in to
//! anything or sends a sign-in anywhere.
//!
//! Buying and wishlists happen on the stores' own pages, never in Hearth:
//!   - Steam: the Steam client's own store (steam://store/<appid>), in Steam's window, already signed in.
//!   - GOG, Epic, Xbox: the store's web page in Hearth's store window, which uses the same browser profile as that
//!     store's sign-in window in Library › Stores (web_profile()), so signing in once there covers both.
//! See docs/STORE.md.
//!
//! Answers are kept a while in cache/storefront (store home 30 minutes, searches 15 minutes, game pages 12 hours),
//! used past that when the store can't be reached, and asked for one at a time per store with a gap between
//! (gentle on the stores' limits).

pub mod epic;
pub mod gog;
pub mod steam;
pub mod xbox;

use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

pub const STORES: [&str; 4] = ["steam", "gog", "epic", "xbox"];

const HOME_TTL_MS: u64 = 30 * 60 * 1000;
const SEARCH_TTL_MS: u64 = 15 * 60 * 1000;
const DETAILS_TTL_MS: u64 = 12 * 3600 * 1000;

// ------------------------------------------------------------------ what the page gets

/// A price, as the store shows it: final ("$9.99", or "Free"), the price before a discount, the discount in percent.
pub fn price(final_text: &str, base_text: Option<&str>, discount: u64, free: bool) -> Value {
    json!({"final": if free { "Free".to_string() } else { final_text.to_string() },
           "base": base_text.filter(|b| !b.is_empty() && *b != final_text && discount > 0), "discount": discount, "free": free})
}

/// An amount in cents as text, with the currency's sign for the common ones ("$9.99", "9,99 €"), else "9.99 PLN".
pub fn money(cents: i64, currency: &str) -> String {
    let neg = cents < 0;
    let c = cents.unsigned_abs();
    let n = format!("{}.{:02}", c / 100, c % 100);
    let s = match currency {
        "USD" => format!("${n}"), "CAD" => format!("CA${n}"), "AUD" => format!("A${n}"), "GBP" => format!("£{n}"),
        "EUR" => format!("{} €", n.replace('.', ",")), "JPY" => format!("¥{}", c / 100), "" => n,
        other => format!("{n} {other}"),
    };
    if neg { format!("-{s}") } else { s }
}

/// A shelf on a store's page: {id, title, items}.
pub fn section(id: &str, title: &str, items: Vec<Value>) -> Value { json!({"id": id, "title": title, "items": items}) }

/// One game in a list: the fields every store fills (more may be added per store).
#[allow(clippy::too_many_arguments)]
pub fn item(store: &str, id: &str, title: &str, wide: Option<String>, tall: Option<String>, price: Value, url: Option<String>) -> Value {
    json!({"store": store, "id": id, "title": title.trim(), "wide": wide, "tall": tall, "price": price, "url": url})
}

/// Text from a store's HTML description: paragraphs, no tags, entities decoded, at most `max` paragraphs.
/// The page shows it as text (never as HTML), so nothing in a description can run or load anything.
pub fn html_text(html: &str, max: usize) -> Vec<String> {
    let mut out = String::with_capacity(html.len());
    let mut i = 0;
    let b = html.as_bytes();
    while i < b.len() {
        if b[i] == b'<' {
            let close = html[i..].find('>').map(|e| i + e);
            let end = close.map(|c| c + 1).unwrap_or(b.len());
            let tag = html[i + 1..close.unwrap_or(b.len())].trim_start_matches('/').to_ascii_lowercase();
            let name: String = tag.chars().take_while(|c| c.is_ascii_alphanumeric()).collect();
            // Scripts and styles: their insides aren't text.
            if (name == "script" || name == "style") && !html[i + 1..].starts_with('/') {
                let close = format!("</{name}");
                i = html[end..].to_ascii_lowercase().find(&close).map(|c| end + c).unwrap_or(b.len());
                continue;
            }
            if matches!(name.as_str(), "br" | "p" | "div" | "li" | "ul" | "ol" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "hr" | "tr" | "table") { out.push('\n'); }
            if name == "li" && !html[i + 1..].starts_with('/') { out.push_str("• "); }
            i = end;
        } else {
            let next = html[i..].find('<').map(|n| i + n).unwrap_or(b.len());
            out.push_str(&decode_entities(&html[i..next]));
            i = next;
        }
    }
    out.split('\n').map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|l| !l.is_empty() && l != "•").take(max).collect()
}

fn decode_entities(s: &str) -> String {
    if !s.contains('&') { return s.to_string(); }
    let mut o = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(a) = rest.find('&') {
        o.push_str(&rest[..a]);
        let after = &rest[a..];
        let end = after.find(';').filter(|e| *e <= 10);
        let ent = end.map(|e| &after[1..e]).unwrap_or("");
        let ch = match ent {
            "amp" => Some("&".to_string()), "lt" => Some("<".into()), "gt" => Some(">".into()), "quot" => Some("\"".into()),
            "apos" | "#39" => Some("'".into()), "nbsp" => Some(" ".into()), "mdash" => Some("—".into()), "ndash" => Some("–".into()),
            "hellip" => Some("…".into()), "rsquo" => Some("’".into()), "lsquo" => Some("‘".into()), "ldquo" => Some("“".into()), "rdquo" => Some("”".into()),
            "trade" => Some("™".into()), "reg" => Some("®".into()), "copy" => Some("©".into()),
            e if e.starts_with("#x") => u32::from_str_radix(&e[2..], 16).ok().and_then(char::from_u32).map(|c| c.to_string()),
            e if e.starts_with('#') => e[1..].parse::<u32>().ok().and_then(char::from_u32).map(|c| c.to_string()),
            _ => None,
        };
        match (ch, end) {
            (Some(c), Some(e)) => { o.push_str(&c); rest = &after[e + 1..]; }
            _ => { o.push('&'); rest = &after[1..]; }
        }
    }
    o.push_str(rest);
    o
}

/// http:// and //host addresses as https://.
pub fn https(u: &str) -> String { crate::owned::https(u) }

/// Where the store pages are for: the device's country and language (from LANG, like Xbox Cloud Gaming's
/// catalog): ("US", "en-us").
pub fn locale() -> (String, String) { crate::stores::xcloud::locale() }

// ------------------------------------------------------------------ asking the stores

/// One request at a time per store, at least this far apart.
fn gap(store: &str, kind: &str) -> Duration {
    match (store, kind) { ("steam", "details") => Duration::from_millis(1500), _ => Duration::from_millis(350) }
}
static LAST: OnceLock<Mutex<std::collections::HashMap<String, Arc<Mutex<Option<Instant>>>>>> = OnceLock::new();
fn turn(store: &str) -> Arc<Mutex<Option<Instant>>> {
    LAST.get_or_init(Default::default).lock().unwrap().entry(store.to_string()).or_default().clone()
}

/// A JSON answer from a store's public endpoint (GET, or POST with a JSON body), waiting its turn.
pub fn fetch(store: &str, kind: &str, method: &str, url: &str, body: Option<&str>, headers: &[(&str, &str)]) -> Result<Value, String> {
    let slot = turn(store);
    let mut last = slot.lock().unwrap();
    if let Some(t) = *last { let g = gap(store, kind); let e = t.elapsed(); if e < g { std::thread::sleep(g - e); } }
    let mut h: Vec<(&str, &str)> = vec![("Accept", "application/json")];
    if body.is_some() { h.push(("Content-Type", "application/json")); }
    h.extend_from_slice(headers);
    let r = crate::http::request(method, url, &h, body, 25);
    *last = Some(Instant::now());
    drop(last);
    let r = r?;
    if r.status == 429 { return Err(format!("{} is busy right now. Try again in a minute.", label(store))); }
    if !r.ok() { return Err(format!("{} didn't answer (HTTP {})", label(store), r.status)); }
    let v = r.json();
    if v.is_null() { return Err(format!("{} sent something Hearth couldn't read", label(store))); }
    Ok(v)
}

pub fn label(store: &str) -> &'static str {
    match store { "steam" => "Steam", "gog" => "GOG", "epic" => "Epic Games Store", "xbox" => "Microsoft Store", _ => "The store" }
}

// ------------------------------------------------------------------ keeping answers

fn cache_dir() -> PathBuf { crate::paths::data_dir().join("cache/storefront") }
fn cache_file(dir: &std::path::Path, key: &str) -> PathBuf {
    let safe: String = key.chars().map(|c| if c.is_ascii_alphanumeric() || "-_".contains(c) { c } else { '_' }).take(80).collect();
    let h = sha1_smol::Sha1::from(key).digest().to_string();
    dir.join(format!("{safe}-{}.json", &h[..10]))
}
/// A kept answer: (value, age in ms). One kept "in the future" (a clock set back since) counts as old.
fn kept(dir: &std::path::Path, key: &str) -> Option<(Value, u64)> {
    let v: Value = serde_json::from_slice(&std::fs::read(cache_file(dir, key)).ok()?).ok()?;
    let at = v["at"].as_u64()?;
    let now = crate::library::now_ms();
    Some((v["v"].clone(), if at > now { u64::MAX } else { now - at }))
}
fn keep(dir: &std::path::Path, key: &str, v: &Value) {
    let _ = std::fs::create_dir_all(dir);
    let _ = crate::paths::write_atomic(&cache_file(dir, key), json!({"at": crate::library::now_ms(), "v": v}).to_string().as_bytes());
}
/// The answer for `key`: kept if fresh, else fetched (and kept); a failed fetch falls back to an old answer, marked
/// {"stale": true}.
pub fn cached(key: &str, ttl_ms: u64, get: impl FnOnce() -> Result<Value, String>) -> Result<Value, String> { cached_in(&cache_dir(), key, ttl_ms, get) }
fn cached_in(dir: &std::path::Path, key: &str, ttl_ms: u64, get: impl FnOnce() -> Result<Value, String>) -> Result<Value, String> {
    let old = kept(dir, key);
    if let Some((v, age)) = &old { if *age < ttl_ms { return Ok(v.clone()); } }
    match get() {
        Ok(v) => { keep(dir, key, &v); Ok(v) }
        Err(e) => match old {
            Some((mut v, _)) => { if v.is_object() { v["stale"] = json!(true); } Ok(v) }
            None => Err(e),
        },
    }
}

/// Old kept answers (older than a week) are removed now and then, so the folder doesn't grow for good.
pub fn tidy() {
    let now = std::time::SystemTime::now();
    for e in std::fs::read_dir(cache_dir()).into_iter().flatten().flatten() {
        let old = e.metadata().and_then(|m| m.modified()).ok().and_then(|m| now.duration_since(m).ok()).map(|d| d.as_secs() > 7 * 86400).unwrap_or(false);
        if old { let _ = std::fs::remove_file(e.path()); }
    }
}

// ------------------------------------------------------------------ the page's calls

fn known(store: &str) -> Result<&'static str, String> { STORES.into_iter().find(|s| *s == store).ok_or_else(|| "Unknown store".to_string()) }

/// A store's front page: {store, sections: [{id, title, items}], stale?}.
pub fn home(store: &str) -> Result<Value, String> {
    let s = known(store)?;
    let (market, lang) = locale();
    let v = cached(&format!("{s}-home-{market}-{lang}"), HOME_TTL_MS, || {
        let sections = match s { "steam" => steam::home(), "gog" => gog::home(), "epic" => epic::home(), _ => xbox::home() }?;
        let sections: Vec<Value> = sections.into_iter().filter(|x| x["items"].as_array().map(|a| !a.is_empty()).unwrap_or(false)).collect();
        if sections.is_empty() { return Err(format!("{} didn't send any games", label(s))); }
        Ok(json!({"store": s, "sections": sections}))
    })?;
    if store == "steam" { tidy(); }
    Ok(v)
}

/// Games matching what was typed: {store, query, items}.
pub fn search(store: &str, q: &str) -> Result<Value, String> {
    let s = known(store)?;
    let q = q.trim();
    if q.is_empty() { return Ok(json!({"store": s, "query": q, "items": []})); }
    if q.chars().count() > 80 { return Err("That's a long search. Try a shorter name.".into()); }
    let (market, lang) = locale();
    cached(&format!("{s}-search-{market}-{lang}-{}", q.to_lowercase()), SEARCH_TTL_MS, || {
        let items = match s { "steam" => steam::search(q), "gog" => gog::search(q), "epic" => epic::search(q), _ => xbox::search(q) }?;
        Ok(json!({"store": s, "query": q, "items": items}))
    })
}

/// A game's page: description, pictures, price, rating, where to buy it.
pub fn details(store: &str, id: &str) -> Result<Value, String> {
    let s = known(store)?;
    let ok_id = !id.is_empty() && id.len() <= 120 && id.chars().all(|c| c.is_ascii_alphanumeric() || "-_:.".contains(c));
    if !ok_id { return Err("That game isn't in the store".into()); }
    let (market, lang) = locale();
    cached(&format!("{s}-details-{market}-{lang}-{id}"), DETAILS_TTL_MS, || match s {
        "steam" => steam::details(id), "gog" => gog::details(id), "epic" => epic::details(id), _ => xbox::details(id),
    })
}

// ------------------------------------------------------------------ the store window and its sign-in

/// A store's own web page in Hearth's store window (GOG, Epic, Xbox): checkout, wishlist, the game's page.
#[derive(Debug, Clone, PartialEq)]
pub struct WebPage { pub store: String, pub url: String, pub title: String, pub profile: PathBuf }
impl WebPage {
    /// The browser profile folder for one drawing engine ("webkit" or "chromium").
    pub fn profile_for(&self, engine: &str) -> PathBuf { self.profile.join(engine) }
}

/// The browser profile a store's web pages use: its sign-in window in Library › Stores and the store window share
/// it, so one sign-in (on the store's own page) covers both. Kept with the person's other sign-ins (accounts/,
/// which moves with their profile when people switch), one folder per store; `engine` is "webkit" or "chromium"
/// (each drawing engine keeps its own files). A new folder name after signing out (reset_web_profile), so a
/// window still open from before can't carry the old sign-in over.
pub fn web_profile(store: &str, engine: &str) -> PathBuf { web_profile_in(&web_base(), store, engine) }
fn web_base() -> PathBuf { crate::paths::data_dir().join("accounts/web") }
fn web_profile_in(base: &std::path::Path, store: &str, engine: &str) -> PathBuf {
    let idf = base.join(format!("{store}.id"));
    let id = std::fs::read_to_string(&idf).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric()))
        .unwrap_or_else(|| {
            let id = format!("{:x}", crate::library::now_ms());
            let _ = std::fs::create_dir_all(base);
            let _ = crate::paths::write_atomic(&idf, id.as_bytes());
            id
        });
    base.join(format!("{store}-{id}")).join(engine)
}
/// Signing out of a store: its web sign-in goes too (cookies and all), and the next sign-in starts fresh.
pub fn reset_web_profile(store: &str) { reset_web_profile_in(&web_base(), store) }
fn reset_web_profile_in(base: &std::path::Path, store: &str) {
    let idf = base.join(format!("{store}.id"));
    if let Ok(id) = std::fs::read_to_string(&idf) {
        let id = id.trim();
        if !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric()) { let _ = std::fs::remove_dir_all(base.join(format!("{store}-{id}"))); }
    }
    let _ = std::fs::remove_file(idf);
}

/// The web addresses a store window may start at, per store (the page it opens on must be the store's own).
pub fn allowed_start(store: &str, url: &str) -> bool {
    let Some(rest) = url.strip_prefix("https://") else { return false };
    let host = rest.split(['/', '?', '#']).next().unwrap_or("").to_ascii_lowercase();
    if host.contains('@') || host.contains(':') { return false; }
    let under = |d: &str| host == d || host.ends_with(&format!(".{d}"));
    match store {
        "gog" => under("gog.com"),
        "epic" => under("epicgames.com"),
        "xbox" => under("xbox.com") || under("microsoft.com"),
        _ => false,
    }
}

type Opener = Arc<dyn Fn(Option<WebPage>) + Send + Sync>;
static OPENER: OnceLock<Opener> = OnceLock::new();
/// Set by the app at start: how to open (Some) or close (None) the store window.
pub fn set_web_opener(f: Opener) { let _ = OPENER.set(f); }

/// Opens a store's page in the store window, signed in with that store's sign-in from Library › Stores. Steam has
/// no store window: Steam's own client shows its store (steam://store/<appid>).
pub fn open_web(store: &str, url: &str, title: &str) -> Result<(), String> {
    if store == "steam" { return Err("Steam's store opens in Steam".into()); }
    let s = known(store)?;
    if !allowed_start(s, url) { return Err("That page isn't on the store's own website".into()); }
    let open = OPENER.get().ok_or("The store window isn't available here")?;
    let title = if title.trim().is_empty() { label(s).to_string() } else { title.chars().take(80).collect() };
    // The store's folder: the app adds its engine's own ("webkit" or "chromium") to it.
    let profile = web_profile(s, "webkit").parent().map(|p| p.to_path_buf()).unwrap_or_default();
    crate::bridge::log(&format!("store: opening {s}'s page in the store window"));
    open(Some(WebPage { store: s.to_string(), url: url.to_string(), title, profile }));
    Ok(())
}

/// Closes the store window (if it's open).
pub fn close_web() { if let Some(open) = OPENER.get() { open(None); } }

/// What the Store's buttons need: per store, signed in (and expired), the store window, Steam, Xodus Nexus.
/// Sign-in state only: no names, tokens or anything else from the accounts.
pub fn status(steam_signed_in: bool, steam_installed: bool) -> Value {
    let mut stores = serde_json::Map::new();
    for s in crate::stores::STORES {
        let expired = crate::stores::expiry::is_expired(s);
        stores.insert(s.to_string(), json!({"signedIn": crate::stores::signed_in(s) && !expired, "expired": expired}));
    }
    stores.insert("steam".into(), json!({"signedIn": steam_signed_in, "installed": steam_installed}));
    json!({"stores": stores, "webWindow": OPENER.get().is_some(), "xodus": crate::xodus::installed(),
           "gameMode": crate::signin_handoff::game_mode(), "market": locale().0})
}

/// The real stores, through the same code the page uses (CI's store-live job, warn-only):
/// `cargo test --lib storefront::live -- --ignored --nocapture`. Says what each store sent, never fails.
#[cfg(test)]
mod live {
    #[test]
    #[ignore]
    fn live_endpoints() {
        let say = |what: &str, r: Result<String, String>| match r {
            Ok(m) => println!("::notice title=Store {what}::{m}"),
            Err(e) => println!("::warning title=Store {what}::{e}"),
        };
        let count = |v: &serde_json::Value| v["sections"].as_array().map(|s| s.iter().map(|x| format!("{} {}", x["title"].as_str().unwrap_or("?"), x["items"].as_array().map(|i| i.len()).unwrap_or(0))).collect::<Vec<_>>().join(", ")).unwrap_or_default();
        for (store, q) in [("steam", "hades"), ("gog", "witcher"), ("epic", "hades"), ("xbox", "halo")] {
            let home = super::home(store);
            say(&format!("{store} home"), home.as_ref().map(count).map_err(|e| e.clone()));
            let first = home.ok().and_then(|h| h["sections"].as_array().and_then(|s| s.iter().find_map(|x| x["items"][0]["id"].as_str().map(str::to_string))));
            let found = super::search(store, q);
            say(&format!("{store} search"), found.as_ref().map(|r| format!("{} results for {q}: {}", r["items"].as_array().map(|a| a.len()).unwrap_or(0),
                r["items"].as_array().into_iter().flatten().take(3).filter_map(|i| i["title"].as_str()).collect::<Vec<_>>().join(", "))).map_err(|e| e.clone()));
            let id = found.ok().and_then(|r| r["items"][0]["id"].as_str().map(str::to_string)).or(first);
            match id {
                Some(id) => say(&format!("{store} game page"), super::details(store, &id).map(|d| format!("{} · {} · {} paragraphs, {} screenshots · {}",
                    d["title"].as_str().unwrap_or("?"), d["price"]["final"].as_str().unwrap_or("no price"), d["description"].as_array().map(|a| a.len()).unwrap_or(0),
                    d["screenshots"].as_array().map(|a| a.len()).unwrap_or(0), d["rating"]["label"].as_str().unwrap_or("no rating")))),
                None => say(&format!("{store} game page"), Err("no game to open".into())),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn money_and_prices() {
        assert_eq!(money(1999, "USD"), "$19.99");
        assert_eq!(money(5, "EUR"), "0,05 €");
        assert_eq!(money(120000, "PLN"), "1200.00 PLN");
        assert_eq!(money(-100, "GBP"), "-£1.00");
        let p = price("$9.99", Some("$19.99"), 50, false);
        assert_eq!(p, json!({"final": "$9.99", "base": "$19.99", "discount": 50, "free": false}));
        // No discount: no "before" price.
        assert_eq!(price("$9.99", Some("$9.99"), 0, false)["base"], Value::Null);
        assert_eq!(price("$0.00", None, 0, true)["final"], "Free");
    }

    #[test]
    fn descriptions_become_text() {
        let t = html_text("<p>Hello&nbsp;<b>world</b> &amp; more</p><script>alert(1)</script><ul><li>One</li><li>Two &#8212; x</li></ul><br>&unknown; ok<img src=x onerror=alert(1)>", 10);
        assert_eq!(t, vec!["Hello world & more", "• One", "• Two — x", "&unknown; ok"]);
        assert_eq!(html_text("a<br>b<br>c", 2), vec!["a", "b"]);
        assert_eq!(html_text("<style>p{}</style>Only", 5), vec!["Only"]);
        assert_eq!(html_text("broken <b", 5), vec!["broken"]);
        assert_eq!(html_text("ünïcode <bé", 5), vec!["ünïcode"]);   // a tag cut off after a letter that isn't ASCII
        assert_eq!(html_text("&#x2019;quoted&rsquo; &", 5), vec!["’quoted’ &"]);
    }

    #[test]
    fn store_windows_start_on_the_store_only() {
        assert!(allowed_start("gog", "https://www.gog.com/en/game/x"));
        assert!(allowed_start("epic", "https://store.epicgames.com/purchase?offers=1-a-b"));
        assert!(allowed_start("xbox", "https://www.xbox.com/en-US/games/store/x/9ABC"));
        assert!(!allowed_start("gog", "http://www.gog.com/"));
        assert!(!allowed_start("gog", "https://evilgog.com/"));
        assert!(!allowed_start("gog", "https://www.gog.com.evil.net/"));
        assert!(!allowed_start("gog", "https://user@www.gog.com/"));
        assert!(!allowed_start("epic", "https://www.gog.com/"));
        assert!(!allowed_start("steam", "https://store.steampowered.com/"));
        assert!(open_web("steam", "https://store.steampowered.com/app/1", "x").is_err());
        assert!(open_web("gog", "https://example.com/", "x").is_err());
    }

    #[test]
    fn answers_are_kept_and_used_when_offline() {
        let d = tempfile::tempdir().unwrap();
        let dir = d.path();
        let key = "steam-search-US-en-us-half life: 3?";
        assert_eq!(cached_in(dir, key, 60_000, || Ok(json!({"n": 1}))).unwrap(), json!({"n": 1}));
        // Fresh: not asked again.
        assert_eq!(cached_in(dir, key, 60_000, || panic!("asked again")).unwrap(), json!({"n": 1}));
        // Too old and the store can't be reached: the old answer, marked stale.
        assert_eq!(cached_in(dir, key, 0, || Err("offline".into())).unwrap(), json!({"n": 1, "stale": true}));
        assert!(cached_in(dir, "never-kept", 0, || Err("offline".into())).is_err());
        // One file per answer, with a name that's safe on disk.
        let names: Vec<String> = std::fs::read_dir(dir).unwrap().flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect();
        assert_eq!(names.len(), 1);
        assert!(names[0].starts_with("steam-search-US-en-us-half_life__3_-") && names[0].ends_with(".json"), "{names:?}");
    }

    #[test]
    fn web_sign_in_folders() {
        let d = tempfile::tempdir().unwrap();
        let base = d.path();
        let a = web_profile_in(base, "gog", "webkit");
        assert_eq!(web_profile_in(base, "gog", "webkit"), a);   // the same folder each time
        assert!(a.starts_with(base) && a.ends_with("webkit"));
        assert_eq!(web_profile_in(base, "gog", "chromium").parent(), a.parent());   // each engine its own, side by side
        assert_ne!(web_profile_in(base, "epic", "webkit").parent(), a.parent());
        std::fs::create_dir_all(&a).unwrap();
        std::fs::write(a.join("cookies.sqlite"), "x").unwrap();
        reset_web_profile_in(base, "gog");
        assert!(!a.exists() && !a.parent().unwrap().exists());
        std::thread::sleep(Duration::from_millis(5));
        assert_ne!(web_profile_in(base, "gog", "webkit"), a);   // a new folder after signing out
        // The web sign-ins move with the person's profile (profiles.rs keeps accounts/ per person).
        assert!(crate::profiles::OWN.contains(&"accounts"));
        assert!(search("amazon", "x").is_err());
        assert!(details("steam", "../etc").is_err());
    }
}
