//! Hearth Theater without the window: your saved sites (a list you make yourself by pasting links), what a site is
//! called and looks like (read from the page's own title, preview image and icon), and where all that is kept
//! (~/.local/share/hearth-theater). Theater never pulls videos out of a site: a site always opens as itself, in
//! Theater's full-screen viewer (main.rs), and plays there.
//!
//! Each site's catalog (the shows, movies and videos its own pages list) is read by reader.js inside Theater's own
//! web view, with your own sign-ins; what it hands back is checked and tidied here (`sanitize`) and kept per site in
//! catalog/<id>.json, with poster pictures in art/c-<id>/. Titles you open or favorite are kept in titles.json.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const CATEGORIES: &[&str] = &["Anime", "TV", "Movies", "Other"];
pub const MAX_PAGE_BYTES: u64 = 2 * 1024 * 1024;
pub const MAX_ART_BYTES: u64 = 4 * 1024 * 1024;
pub const MAX_NAME: usize = 80;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Site {
    pub id: String,
    pub name: String,
    pub url: String,
    pub category: String,
    /// The preview image's file in the art folder ("" when there isn't one).
    pub art: String,
    /// The site's icon's file in the art folder ("" when there isn't one).
    pub icon: String,
    pub favorite: bool,
    /// When it was added and last opened (ms since 1970; 0: never opened).
    pub added: u64,
    pub opened: u64,
    pub opens: u32,
    /// The site's catalog rows you hid (by name).
    pub hidden: Vec<String>,
}

/// What a page says about itself.
#[derive(Serialize, Clone, Debug, PartialEq, Default)]
pub struct Meta {
    pub title: String,
    pub image: String,
    pub icon: String,
}

pub fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

pub fn home() -> PathBuf {
    std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from).unwrap_or_else(std::env::temp_dir)
}

/// ~/.local/share/hearth-theater (XDG_DATA_HOME's), or %APPDATA%\Hearth Theater on Windows (Ember).
pub fn data_dir() -> PathBuf {
    if let Some(d) = std::env::var_os("HEARTH_THEATER_DIR") { return PathBuf::from(d); }
    if cfg!(windows) {
        return std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(home).join("Hearth Theater");
    }
    std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|| home().join(".local/share")).join("hearth-theater")
}

pub fn art_dir(dir: &Path) -> PathBuf { dir.join("art") }

/// A link as typed or pasted, made into a web address: "example.com/show" gets https://; only http and https are
/// allowed (no file:, javascript: or the like). Spaces around it and a trailing newline from pasting are dropped.
pub fn normalize_url(input: &str) -> Result<String, String> {
    let t = input.trim();
    if t.is_empty() { return Err("Paste or type a link first".into()); }
    if t.chars().any(char::is_whitespace) { return Err("That doesn't look like a link (it has spaces)".into()); }
    let lower = t.to_ascii_lowercase();
    let with = if lower.starts_with("http://") || lower.starts_with("https://") {
        t.to_string()
    } else if lower.contains("://") || lower.starts_with("javascript:") || lower.starts_with("data:") || lower.starts_with("file:") {
        return Err("Only web links (http or https) can be added".into());
    } else {
        format!("https://{t}")
    };
    let u = url::Url::parse(&with).map_err(|_| "That doesn't look like a link".to_string())?;
    match u.host_str() {
        Some(h) if h.contains('.') || h == "localhost" || h.parse::<std::net::IpAddr>().is_ok() => {}
        _ => return Err("That doesn't look like a link (no website name)".into()),
    }
    Ok(u.to_string())
}

/// A short name for a site when its page doesn't give one: its host without "www.".
pub fn host_name(url: &str) -> String {
    url::Url::parse(url).ok().and_then(|u| u.host_str().map(|h| h.trim_start_matches("www.").to_string())).unwrap_or_default()
}

/// A page title made fit for a tile: entities decoded, spaces squeezed, and the usual " | Watch free online" style
/// tails left on (people name their own tiles if they want); cut to MAX_NAME characters.
pub fn clean_title(t: &str) -> String {
    let t = decode_entities(t);
    let t = t.split_whitespace().collect::<Vec<_>>().join(" ");
    if t.chars().count() > MAX_NAME { t.chars().take(MAX_NAME - 1).collect::<String>() + "…" } else { t }
}

pub fn decode_entities(s: &str) -> String {
    if !s.contains('&') { return s.to_string(); }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        rest = &rest[i..];
        let end = rest[..rest.len().min(12)].find(';');
        let Some(end) = end else { out.push('&'); rest = &rest[1..]; continue; };
        let ent = &rest[1..end];
        let ch = match ent {
            "amp" => Some('&'), "lt" => Some('<'), "gt" => Some('>'), "quot" => Some('"'), "apos" | "#39" => Some('\''),
            "nbsp" => Some(' '), "ndash" => Some('–'), "mdash" => Some('—'), "hellip" => Some('…'),
            _ if ent.starts_with("#x") || ent.starts_with("#X") => u32::from_str_radix(&ent[2..], 16).ok().and_then(char::from_u32),
            _ if ent.starts_with('#') => ent[1..].parse::<u32>().ok().and_then(char::from_u32),
            _ => None,
        };
        match ch { Some(c) => { out.push(c); rest = &rest[end + 1..]; } None => { out.push('&'); rest = &rest[1..]; } }
    }
    out.push_str(rest);
    out
}

/// The attributes of one tag's text (`<meta property="og:image" content="...">` → [(property, og:image), ...]),
/// names lowercased.
fn attrs(tag: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let b = tag.as_bytes();
    let mut i = 0;
    // skip the tag name
    while i < b.len() && !b[i].is_ascii_whitespace() { i += 1; }
    while i < b.len() {
        while i < b.len() && (b[i].is_ascii_whitespace() || b[i] == b'/') { i += 1; }
        let ns = i;
        while i < b.len() && !b[i].is_ascii_whitespace() && b[i] != b'=' && b[i] != b'/' { i += 1; }
        if ns == i { i += 1; continue; }
        let name = tag[ns..i].to_ascii_lowercase();
        while i < b.len() && b[i].is_ascii_whitespace() { i += 1; }
        if i < b.len() && b[i] == b'=' {
            i += 1;
            while i < b.len() && b[i].is_ascii_whitespace() { i += 1; }
            let (vs, ve);
            if i < b.len() && (b[i] == b'"' || b[i] == b'\'') {
                let q = b[i]; i += 1; vs = i;
                while i < b.len() && b[i] != q { i += 1; }
                ve = i; i += 1;
            } else {
                vs = i;
                while i < b.len() && !b[i].is_ascii_whitespace() { i += 1; }
                ve = i;
            }
            out.push((name, decode_entities(&tag[vs..ve.min(tag.len())])));
        } else {
            out.push((name, String::new()));
        }
    }
    out
}

fn get<'a>(a: &'a [(String, String)], k: &str) -> Option<&'a str> {
    a.iter().find(|(n, _)| n == k).map(|(_, v)| v.as_str())
}

/// Reads a page's title, preview image and icon from its <head>: og:/twitter: tags first, then <title>, then the
/// icon links; addresses made absolute against `base`.
pub fn parse_meta(html: &str, base: &str) -> Meta {
    let base_u = url::Url::parse(base).ok();
    let abs = |s: &str| -> String {
        let s = s.trim();
        if s.is_empty() || s.starts_with("data:") { return String::new(); }
        match &base_u {
            Some(b) => b.join(s).map(|u| u.to_string()).ok().filter(|u| u.starts_with("http")).unwrap_or_default(),
            None => String::new(),
        }
    };
    let lower = html.to_ascii_lowercase();
    // Only the head matters (and keeps big pages quick).
    let head_end = lower.find("</head>").or_else(|| lower.find("<body")).unwrap_or(html.len());
    let (html, lower) = (&html[..head_end], &lower[..head_end]);
    let mut m = Meta::default();
    let (mut og_title, mut tw_title, mut og_image, mut tw_image) = (String::new(), String::new(), String::new(), String::new());
    let mut icons: Vec<(i32, String)> = Vec::new();
    let mut pos = 0;
    while let Some(i) = lower[pos..].find('<') {
        let start = pos + i + 1;
        let Some(len) = lower[start..].find('>') else { break };
        let tag = &html[start..start + len];
        let ltag = &lower[start..start + len];
        pos = start + len + 1;
        if ltag.starts_with("meta") {
            let a = attrs(tag);
            let key = get(&a, "property").or_else(|| get(&a, "name")).unwrap_or("").to_ascii_lowercase();
            let content = get(&a, "content").unwrap_or("");
            match key.as_str() {
                "og:title" if og_title.is_empty() => og_title = content.to_string(),
                "twitter:title" if tw_title.is_empty() => tw_title = content.to_string(),
                "og:image" | "og:image:url" | "og:image:secure_url" if og_image.is_empty() => og_image = content.to_string(),
                "twitter:image" | "twitter:image:src" if tw_image.is_empty() => tw_image = content.to_string(),
                _ => {}
            }
        } else if ltag.starts_with("title") && m.title.is_empty() {
            if let Some(e) = lower[pos..].find("</title") { m.title = html[pos..pos + e].to_string(); pos += e; }
        } else if ltag.starts_with("link") {
            let a = attrs(tag);
            let rel = get(&a, "rel").unwrap_or("").to_ascii_lowercase();
            let href = get(&a, "href").unwrap_or("");
            if href.is_empty() { continue; }
            let rank = if rel.contains("apple-touch-icon") { 3 } else if rel.split_whitespace().any(|r| r == "icon") {
                if get(&a, "type").map(|t| t.contains("svg")).unwrap_or(false) || href.ends_with(".svg") { 1 } else { 2 }
            } else { continue };
            icons.push((rank, href.to_string()));
        }
    }
    let title = [og_title, tw_title, m.title.clone()].into_iter().map(|t| clean_title(&t)).find(|t| !t.is_empty()).unwrap_or_default();
    m.title = title;
    m.image = [og_image, tw_image].iter().map(|s| abs(s)).find(|s| !s.is_empty()).unwrap_or_default();
    icons.sort_by(|a, b| b.0.cmp(&a.0));
    m.icon = icons.iter().map(|(_, h)| abs(h)).find(|s| !s.is_empty())
        .or_else(|| base_u.as_ref().and_then(|b| b.join("/favicon.ico").ok()).map(|u| u.to_string()))
        .unwrap_or_default();
    m
}

// ------------------------------------------------------------------------------------------- the list of sites

pub fn sites_file(dir: &Path) -> PathBuf { dir.join("sites.json") }

pub fn load(dir: &Path) -> Vec<Site> {
    std::fs::read_to_string(sites_file(dir)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

/// Saved through a temporary file and a rename, so a crash never leaves half a list.
pub fn save(dir: &Path, sites: &[Site]) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let tmp = dir.join(".sites.json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(sites).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, sites_file(dir)).map_err(|e| e.to_string())
}

pub fn new_id(sites: &[Site]) -> String {
    let mut n = now_ms();
    loop {
        let id = format!("s{n:x}");
        if !sites.iter().any(|s| s.id == id) { return id; }
        n += 1;
    }
}

pub fn fix_category(c: &str) -> String {
    CATEGORIES.iter().find(|k| k.eq_ignore_ascii_case(c.trim())).unwrap_or(&"Other").to_string()
}

/// Adds a site (the same address twice is refused, so a tile isn't doubled by accident).
pub fn add(sites: &mut Vec<Site>, url: &str, name: &str, category: &str) -> Result<Site, String> {
    let url = normalize_url(url)?;
    if sites.iter().any(|s| same_site(&s.url, &url)) { return Err("That link is already in Theater".into()); }
    let name = clean_title(name);
    let s = Site {
        id: new_id(sites),
        name: if name.is_empty() { host_name(&url) } else { name },
        url,
        category: fix_category(category),
        added: now_ms(),
        ..Default::default()
    };
    sites.push(s.clone());
    Ok(s)
}

/// Two addresses are the same site's tile when they differ only by a trailing slash or http/https.
pub fn same_site(a: &str, b: &str) -> bool {
    let k = |s: &str| s.trim_end_matches('/').trim_start_matches("https://").trim_start_matches("http://").to_ascii_lowercase();
    k(a) == k(b)
}

/// "Continue": the sites you opened, newest first.
pub fn recent(sites: &[Site], n: usize) -> Vec<Site> {
    let mut v: Vec<Site> = sites.iter().filter(|s| s.opened > 0).cloned().collect();
    v.sort_by(|a, b| b.opened.cmp(&a.opened));
    v.truncate(n);
    v
}

/// A file name in the art folder for a site's picture ("<id>-art.<ext>" or "<id>-icon.<ext>"), from its type.
pub fn art_name(id: &str, kind: &str, content_type: &str, url: &str) -> String {
    let ct = content_type.to_ascii_lowercase();
    let ext = if ct.contains("png") { "png" } else if ct.contains("jpeg") || ct.contains("jpg") { "jpg" } else if ct.contains("webp") { "webp" }
        else if ct.contains("gif") { "gif" } else if ct.contains("svg") { "svg" } else if ct.contains("icon") || url.ends_with(".ico") { "ico" }
        else if url.ends_with(".png") { "png" } else if url.ends_with(".webp") { "webp" } else { "jpg" };
    format!("{id}-{kind}.{ext}")
}

pub fn mime_for(file: &str) -> &'static str {
    match file.rsplit('.').next().unwrap_or("") {
        "png" => "image/png", "webp" => "image/webp", "gif" => "image/gif", "svg" => "image/svg+xml", "ico" => "image/x-icon",
        _ => "image/jpeg",
    }
}

/// Removes a site, its pictures, its catalog and the titles you kept from it.
pub fn remove(dir: &Path, sites: &mut Vec<Site>, id: &str) -> Result<(), String> {
    let i = sites.iter().position(|s| s.id == id).ok_or("That site isn't in Theater")?;
    let s = sites.remove(i);
    for f in [&s.art, &s.icon] { if !f.is_empty() && !f.contains('/') && !f.contains('\\') { let _ = std::fs::remove_file(art_dir(dir).join(f)); } }
    if safe_id(id) {
        let _ = std::fs::remove_file(catalog_file(dir, id));
        let _ = std::fs::remove_dir_all(art_dir(dir).join(format!("c-{id}")));
        let mut t = load_titles(dir);
        let n = t.len();
        t.retain(|x| x.site != id);
        if t.len() != n { let _ = save_titles(dir, &t); }
    }
    Ok(())
}

// ------------------------------------------------------------------------------------------- catalogs

pub const KINDS: &[&str] = &["movie", "series", "episode", "video"];
pub const MAX_ROWS: usize = 24;
pub const MAX_ROW_ITEMS: usize = 60;
pub const MAX_ITEMS: usize = 400;
/// How old a catalog may get before Theater reads the site again by itself (6 hours).
pub const STALE_MS: u64 = 6 * 3600 * 1000;

/// One title a site lists: a show, a movie, an episode or a video, and the page it's watched on.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Item {
    pub title: String,
    pub url: String,
    /// The poster's address on the site, and its saved copy in the art folder ("c-<site>/<file>"; "" when none).
    pub image: String,
    pub art: String,
    /// "movie", "series", "episode", "video" or "" (not known).
    pub kind: String,
    pub episodes: u32,
    pub year: String,
    pub desc: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Row {
    pub name: String,
    pub items: Vec<Item>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Catalog {
    /// When it was last read (ms; 0: never).
    pub updated: u64,
    pub rows: Vec<Row>,
    /// The site's search page, with {searchTerms} where the words go ("" when it has none Theater found).
    pub search: String,
    /// The page asked for a sign-in and listed (almost) nothing.
    pub signin: bool,
    /// Where the titles came from (json-ld, open-graph, rss, oembed, sitemap, cards, opensearch).
    pub sources: Vec<String>,
    /// Why the last reading failed ("" when it didn't).
    pub error: String,
}

/// A title you opened or made a favorite (for Continue and Favorites).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Title {
    pub site: String,
    #[serde(flatten)]
    pub item: Item,
    pub favorite: bool,
    pub opened: u64,
    pub opens: u32,
}

/// Site ids are made by `new_id` ("s" and hex); anything else is never used in a file name.
pub fn safe_id(id: &str) -> bool { !id.is_empty() && id.len() <= 40 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') }

/// A picture's name in the art folder is "<file>" or "c-<site>/<file>", nothing that climbs out of it.
pub fn safe_art(file: &str) -> bool {
    let parts: Vec<&str> = file.split('/').collect();
    !file.is_empty() && !file.contains('\\') && parts.len() <= 2 && parts.iter().all(|p| !p.is_empty() && !p.starts_with('.'))
        && (parts.len() == 1 || parts[0].starts_with("c-"))
}

pub fn catalog_file(dir: &Path, id: &str) -> PathBuf { dir.join("catalog").join(format!("{id}.json")) }

pub fn load_catalog(dir: &Path, id: &str) -> Catalog {
    if !safe_id(id) { return Catalog::default(); }
    std::fs::read_to_string(catalog_file(dir, id)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

pub fn save_catalog(dir: &Path, id: &str, c: &Catalog) -> Result<(), String> {
    if !safe_id(id) { return Err("bad site id".into()); }
    let f = catalog_file(dir, id);
    std::fs::create_dir_all(f.parent().unwrap()).map_err(|e| e.to_string())?;
    let tmp = f.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec(c).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, f).map_err(|e| e.to_string())
}

/// An address on a page made absolute (against `base`), web addresses only, without its #part; "" otherwise.
pub fn abs_http(s: &str, base: Option<&url::Url>) -> String {
    let s = s.trim();
    if s.is_empty() || s.len() > 2048 { return String::new(); }
    let u = match base { Some(b) => b.join(s), None => url::Url::parse(s) };
    match u {
        Ok(mut u) if matches!(u.scheme(), "http" | "https") && u.host_str().is_some() => { u.set_fragment(None); u.to_string() }
        _ => String::new(),
    }
}

fn short(s: &str, n: usize) -> String {
    let t = decode_entities(s).split_whitespace().collect::<Vec<_>>().join(" ");
    if t.chars().count() > n { t.chars().take(n - 1).collect::<String>() + "…" } else { t }
}

/// One title as reader.js found it, checked: a web address and a name are needed; the rest is tidied or dropped.
pub fn sanitize_item(v: &serde_json::Value, base: Option<&url::Url>) -> Option<Item> {
    let s = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("");
    let url = abs_http(s("url"), base);
    let title = clean_title(s("title"));
    if url.is_empty() || title.is_empty() { return None; }
    let kind = KINDS.iter().find(|k| **k == s("kind")).map(|k| k.to_string()).unwrap_or_default();
    let episodes = v.get("episodes").and_then(|x| x.as_u64().or_else(|| x.as_str().and_then(|t| t.parse().ok()))).unwrap_or(0).min(100_000) as u32;
    let year: String = s("year").chars().filter(char::is_ascii_digit).take(4).collect();
    Some(Item { title, url, image: abs_http(s("image"), base), art: String::new(), kind, episodes,
        year: if year.len() == 4 { year } else { String::new() }, desc: short(s("desc"), 300) })
}

/// What reader.js handed back for a site, made into its catalog: rows named after the site's own sections (or
/// "From <site>"), rows of the same name joined, each title once per row, the site's own home page left out, and
/// everything capped (MAX_ROWS, MAX_ROW_ITEMS, MAX_ITEMS).
pub fn sanitize(raw: &serde_json::Value, site_url: &str, site_name: &str) -> Catalog {
    let base = url::Url::parse(site_url).ok();
    let site_name = if site_name.trim().is_empty() { host_name(site_url) } else { site_name.trim().to_string() };
    let fallback = format!("From {site_name}");
    let mut rows: Vec<Row> = Vec::new();
    let mut total = 0;
    for r in raw.get("rows").and_then(|x| x.as_array()).map(|a| a.as_slice()).unwrap_or(&[]) {
        let mut name = short(r.get("name").and_then(|x| x.as_str()).unwrap_or(""), 60);
        if name.is_empty() { name = fallback.clone(); }
        let idx = match rows.iter().position(|x| x.name.eq_ignore_ascii_case(&name)) {
            Some(i) => i,
            None => { rows.push(Row { name, items: Vec::new() }); rows.len() - 1 }
        };
        for it in r.get("items").and_then(|x| x.as_array()).map(|a| a.as_slice()).unwrap_or(&[]) {
            if total >= MAX_ITEMS || rows[idx].items.len() >= MAX_ROW_ITEMS { break; }
            let Some(i) = sanitize_item(it, base.as_ref()) else { continue };
            if same_site(&i.url, site_url) || rows[idx].items.iter().any(|x| x.url == i.url) { continue; }
            rows[idx].items.push(i);
            total += 1;
        }
    }
    rows.retain(|r| !r.items.is_empty());
    rows.truncate(MAX_ROWS);
    let search = search_template(raw.get("search").and_then(|x| x.as_str()).unwrap_or(""), base.as_ref());
    let sources = raw.get("sources").and_then(|x| x.as_array()).map(|a| a.iter().filter_map(|x| x.as_str()).take(10).map(|x| short(x, 20)).collect()).unwrap_or_default();
    Catalog { updated: now_ms(), rows, search, signin: raw.get("signin").and_then(|x| x.as_bool()).unwrap_or(false), sources, error: String::new() }
}

/// A search's results from a site's search page, checked the same way (each title once, at most MAX_ROW_ITEMS).
pub fn sanitize_items(raw: &serde_json::Value, page: &str, site_url: &str) -> Vec<Item> {
    let base = url::Url::parse(page).ok();
    let mut out: Vec<Item> = Vec::new();
    for it in raw.get("items").and_then(|x| x.as_array()).map(|a| a.as_slice()).unwrap_or(&[]) {
        if out.len() >= MAX_ROW_ITEMS { break; }
        let Some(i) = sanitize_item(it, base.as_ref()) else { continue };
        if same_site(&i.url, site_url) || same_site(&i.url, page) || out.iter().any(|x| x.url == i.url) { continue; }
        out.push(i);
    }
    out
}

const QMARK: &str = "hearthsearchterms";

/// A search page's address with {searchTerms} where the words go (OpenSearch's form), made absolute; "" when it
/// isn't a web address or has no {searchTerms}.
pub fn search_template(t: &str, base: Option<&url::Url>) -> String {
    if !t.contains("{searchTerms}") { return String::new(); }
    let u = abs_http(&t.replace("{searchTerms}", QMARK), base);
    if u.is_empty() || !u.contains(QMARK) { return String::new(); }
    u.replace(QMARK, "{searchTerms}")
}

/// A search page's address for these words: {searchTerms} filled in, OpenSearch's other {name} or {name?} parts
/// emptied (except {startPage} and {startIndex}, which become 1).
pub fn search_url(template: &str, words: &str) -> String {
    let q: String = url::form_urlencoded::byte_serialize(words.trim().as_bytes()).collect();
    let mut out = String::new();
    let mut rest = template;
    while let Some(i) = rest.find('{') {
        out.push_str(&rest[..i]);
        let Some(j) = rest[i..].find('}') else { out.push_str(&rest[i..]); rest = ""; break };
        let name = rest[i + 1..i + j].trim_end_matches('?');
        let name = name.rsplit(':').next().unwrap_or(name);
        out.push_str(match name { "searchTerms" => &q, "startPage" | "startIndex" => "1", _ => "" });
        rest = &rest[i + j + 1..];
    }
    out.push_str(rest);
    out
}

/// Saved posters kept across readings: a title whose picture is the same as last time keeps its saved copy.
pub fn carry_art(old: &Catalog, new: &mut Catalog, dir: &Path) {
    let mut have = std::collections::HashMap::new();
    for i in old.rows.iter().flat_map(|r| &r.items) { if !i.art.is_empty() && !i.image.is_empty() { have.insert(i.image.clone(), i.art.clone()); } }
    for i in new.rows.iter_mut().flat_map(|r| r.items.iter_mut()) {
        if let Some(a) = have.get(&i.image) { if safe_art(a) && art_dir(dir).join(a).is_file() { i.art = a.clone(); } }
    }
}

/// A poster's file name in art/c-<site>/: a hash of its address, and the picture's type.
pub fn poster_name(site: &str, image: &str, content_type: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    image.hash(&mut h);
    let ext = art_name("x", "p", content_type, image.split('?').next().unwrap_or(image));
    let ext = ext.rsplit('.').next().unwrap_or("jpg");
    format!("c-{site}/{:016x}.{ext}", h.finish())
}

/// The catalogs that are older than STALE_MS (or were never read), in the order Theater reads them.
pub fn stale(sites: &[Site], updated: impl Fn(&str) -> u64, now: u64) -> Vec<String> {
    let mut v: Vec<&Site> = sites.iter().filter(|s| now.saturating_sub(updated(&s.id)) > STALE_MS).collect();
    v.sort_by(|a, b| b.opened.cmp(&a.opened).then(b.added.cmp(&a.added)));
    v.into_iter().map(|s| s.id.clone()).collect()
}

// ------------------------------------------------------------------------------------------- titles you kept

pub fn titles_file(dir: &Path) -> PathBuf { dir.join("titles.json") }

pub fn load_titles(dir: &Path) -> Vec<Title> {
    std::fs::read_to_string(titles_file(dir)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

pub fn save_titles(dir: &Path, t: &[Title]) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let tmp = dir.join(".titles.json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(t).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, titles_file(dir)).map_err(|e| e.to_string())
}

/// The kept title for this site's item (added when it's new; its name and picture brought up to date).
pub fn keep_title<'a>(titles: &'a mut Vec<Title>, site: &str, item: &Item) -> &'a mut Title {
    let i = match titles.iter().position(|t| t.site == site && t.item.url == item.url) {
        Some(i) => { titles[i].item = item.clone(); i }
        None => { titles.push(Title { site: site.to_string(), item: item.clone(), ..Default::default() }); titles.len() - 1 }
    };
    &mut titles[i]
}

/// Titles that are neither a favorite nor opened go; at most 300 opened ones are kept (the newest).
pub fn tidy_titles(titles: &mut Vec<Title>) {
    titles.retain(|t| t.favorite || t.opened > 0);
    let mut opened: Vec<u64> = titles.iter().filter(|t| !t.favorite).map(|t| t.opened).collect();
    if opened.len() > 300 {
        opened.sort_unstable_by(|a, b| b.cmp(a));
        let cut = opened[299];
        titles.retain(|t| t.favorite || t.opened >= cut);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links() {
        assert_eq!(normalize_url("example.com").unwrap(), "https://example.com/");
        assert_eq!(normalize_url("  https://Example.com/shows?id=3\n").unwrap(), "https://example.com/shows?id=3");
        assert_eq!(normalize_url("http://192.168.1.20:8096").unwrap(), "http://192.168.1.20:8096/");
        assert!(normalize_url("javascript:alert(1)").is_err());
        assert!(normalize_url("file:///etc/passwd").is_err());
        assert!(normalize_url("ftp://x.org").is_err());
        assert!(normalize_url("not a link").is_err());
        assert!(normalize_url("hello").is_err());
        assert!(normalize_url("").is_err());
        assert_eq!(host_name("https://www.example.org/a"), "example.org");
    }

    #[test]
    fn titles_and_entities() {
        assert_eq!(decode_entities("Tom &amp; Jerry&#39;s &#x2014; &hearts; &"), "Tom & Jerry's — &hearts; &");
        assert_eq!(clean_title("  Watch\n   Things  "), "Watch Things");
        assert_eq!(clean_title(&"a".repeat(200)).chars().count(), MAX_NAME);
    }

    #[test]
    fn page_meta() {
        let html = r#"<!doctype html><html><head><title>Plain title</title>
            <meta property="og:title" content="Big &amp; Bold">
            <meta name=twitter:image content='/img/card.png'>
            <link rel="icon" href="/favicon.svg" type="image/svg+xml">
            <link rel="apple-touch-icon" href="touch.png">
            </head><body><meta property="og:image" content="/ignored.png"></body></html>"#;
        let m = parse_meta(html, "https://site.example/shows/list");
        assert_eq!(m.title, "Big & Bold");
        assert_eq!(m.image, "https://site.example/img/card.png");
        assert_eq!(m.icon, "https://site.example/shows/touch.png");
        let m = parse_meta("<html><head><TITLE>Only &quot;this&quot;</TITLE></head>", "https://a.example/");
        assert_eq!(m.title, "Only \"this\"");
        assert_eq!(m.image, "");
        assert_eq!(m.icon, "https://a.example/favicon.ico");
        let m = parse_meta("<meta property=og:image content=\"javascript:x\"><meta property=og:title content=Hi>", "https://a.example/");
        assert_eq!(m.image, "");
        assert_eq!(m.title, "Hi");
    }

    #[test]
    fn the_list() {
        let dir = std::env::temp_dir().join(format!("hearth-theater-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut v = load(&dir);
        assert!(v.is_empty());
        let a = add(&mut v, "example.com", "", "anime").unwrap();
        assert_eq!(a.name, "example.com");
        assert_eq!(a.category, "Anime");
        assert!(add(&mut v, "http://example.com", "x", "TV").is_err());
        let b = add(&mut v, "https://other.example/watch", "Other place", "nope").unwrap();
        assert_eq!(b.category, "Other");
        assert_ne!(a.id, b.id);
        v[1].opened = 5;
        assert_eq!(recent(&v, 10).len(), 1);
        save(&dir, &v).unwrap();
        assert_eq!(load(&dir), v);
        std::fs::create_dir_all(art_dir(&dir)).unwrap();
        std::fs::write(art_dir(&dir).join("x-art.png"), b"p").unwrap();
        v[0].art = "x-art.png".into();
        let id = v[0].id.clone();
        remove(&dir, &mut v, &id).unwrap();
        assert!(!art_dir(&dir).join("x-art.png").exists());
        assert_eq!(v.len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn catalogs() {
        let raw = serde_json::json!({
            "rows": [
                {"name": "Popular", "items": [
                    {"title": "  Big &amp; Show ", "url": "/show/1#top", "image": "/p/1.jpg", "kind": "series", "episodes": 12, "year": "2021-04", "desc": "A  show"},
                    {"title": "Big Show again", "url": "https://site.example/show/1", "image": ""},
                    {"title": "", "url": "/show/2"},
                    {"title": "Bad link", "url": "javascript:alert(1)"},
                    {"title": "Home", "url": "https://site.example/"},
                    {"title": "A movie", "url": "https://site.example/movie/9", "image": "data:image/png;base64,xx", "kind": "film", "episodes": "3"}
                ]},
                {"name": "", "items": [{"title": "Loose", "url": "/v/7"}]},
                {"name": "popular", "items": [{"title": "Joined", "url": "/show/3"}]},
                {"name": "Empty", "items": []}
            ],
            "search": "/search?q={searchTerms}&page={startPage?}",
            "signin": false, "sources": ["json-ld", "cards"]
        });
        let c = sanitize(&raw, "https://site.example/", "Site");
        assert_eq!(c.rows.len(), 2);
        assert_eq!(c.rows[0].name, "Popular");
        let p = &c.rows[0].items;
        assert_eq!(p.len(), 3);
        assert_eq!(p[0].title, "Big & Show");
        assert_eq!(p[0].url, "https://site.example/show/1");
        assert_eq!(p[0].image, "https://site.example/p/1.jpg");
        assert_eq!((p[0].kind.as_str(), p[0].episodes, p[0].year.as_str(), p[0].desc.as_str()), ("series", 12, "2021", "A show"));
        assert_eq!((p[1].image.as_str(), p[1].kind.as_str(), p[1].episodes), ("", "", 3));
        assert_eq!(p[2].title, "Joined");
        assert_eq!(c.rows[1].name, "From Site");
        assert_eq!(c.search, "https://site.example/search?q={searchTerms}&page={startPage?}");
        assert_eq!(search_url(&c.search, "one piece & co"), "https://site.example/search?q=one+piece+%26+co&page=1");
        assert_eq!(search_url("https://a.example/s/{searchTerms}?n={count?}&l={language}", "x"), "https://a.example/s/x?n=&l=");
        assert_eq!(search_template("https://a.example/search", None), "");
        assert_eq!(search_template("javascript:{searchTerms}", None), "");
        assert_eq!(c.sources, vec!["json-ld", "cards"]);
        let items = sanitize_items(&serde_json::json!({"items": [{"title": "R", "url": "/show/5"}, {"title": "R2", "url": "/show/5"}, {"title": "Self", "url": "?q=x"}]}),
            "https://site.example/search?q=x", "https://site.example/");
        assert_eq!(items.len(), 1);
        assert!(sanitize(&serde_json::json!(null), "https://site.example/", "").rows.is_empty());
        let mut big = serde_json::json!({"rows": [{"name": "Lots", "items": []}]});
        for n in 0..200 { big["rows"][0]["items"].as_array_mut().unwrap().push(serde_json::json!({"title": format!("T{n}"), "url": format!("/t/{n}")})); }
        assert_eq!(sanitize(&big, "https://site.example/", "").rows[0].items.len(), MAX_ROW_ITEMS);
    }

    #[test]
    fn catalog_files_and_titles() {
        let dir = std::env::temp_dir().join(format!("hearth-theater-cat-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut v = Vec::new();
        let s = add(&mut v, "https://site.example/", "Site", "TV").unwrap();
        assert_eq!(load_catalog(&dir, &s.id), Catalog::default());
        let mut c = sanitize(&serde_json::json!({"rows": [{"name": "New", "items": [{"title": "A", "url": "/a", "image": "/a.jpg"}]}]}), &s.url, &s.name);
        let poster = poster_name(&s.id, "https://site.example/a.jpg", "image/webp");
        assert!(poster.starts_with(&format!("c-{}/", s.id)) && poster.ends_with(".webp") && safe_art(&poster));
        std::fs::create_dir_all(art_dir(&dir).join(format!("c-{}", s.id))).unwrap();
        std::fs::write(art_dir(&dir).join(&poster), b"p").unwrap();
        c.rows[0].items[0].art = poster.clone();
        save_catalog(&dir, &s.id, &c).unwrap();
        assert_eq!(load_catalog(&dir, &s.id), c);
        let mut again = sanitize(&serde_json::json!({"rows": [{"name": "New", "items": [{"title": "A2", "url": "/a", "image": "/a.jpg"}]}]}), &s.url, &s.name);
        carry_art(&c, &mut again, &dir);
        assert_eq!(again.rows[0].items[0].art, poster);
        assert!(!safe_art("../x") && !safe_art("a/b") && !safe_art("c-s1/../../x") && !safe_art(".hidden") && safe_art("s1-art.png"));
        assert!(!safe_id("../x") && safe_id(&s.id));
        assert!(save_catalog(&dir, "../evil", &c).is_err());
        // titles
        let mut t = load_titles(&dir);
        let item = c.rows[0].items[0].clone();
        keep_title(&mut t, &s.id, &item).favorite = true;
        keep_title(&mut t, &s.id, &Item { title: "Renamed".into(), ..item.clone() }).opened = 9;
        assert_eq!(t.len(), 1);
        assert!(t[0].favorite);
        assert_eq!(t[0].item.title, "Renamed");
        keep_title(&mut t, &s.id, &Item { url: "https://site.example/b".into(), ..item.clone() });
        tidy_titles(&mut t);
        assert_eq!(t.len(), 1);
        save_titles(&dir, &t).unwrap();
        assert_eq!(load_titles(&dir), t);
        assert_eq!(stale(&v, |_| 0, STALE_MS * 2), vec![s.id.clone()]);
        assert!(stale(&v, |_| STALE_MS, STALE_MS * 2 - 5).is_empty());
        // removing the site takes its catalog, posters and titles with it
        let id = s.id.clone();
        remove(&dir, &mut v, &id).unwrap();
        assert!(!catalog_file(&dir, &id).exists());
        assert!(!art_dir(&dir).join(&poster).exists());
        assert!(load_titles(&dir).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn art_files() {
        assert_eq!(art_name("s1", "art", "image/png", "https://a/x"), "s1-art.png");
        assert_eq!(art_name("s1", "icon", "", "https://a/favicon.ico"), "s1-icon.ico");
        assert_eq!(mime_for("s1-art.webp"), "image/webp");
    }
}
