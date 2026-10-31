//! Hearth Theater without the window: your saved sites (a list you make yourself by pasting links), what a site is
//! called and looks like (read from the page's own title, preview image and icon), and where all that is kept
//! (~/.local/share/hearth-theater). Theater never pulls videos out of a site: a site always opens as itself, in
//! Theater's full-screen viewer (main.rs), and plays there.

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

/// Removes a site and its pictures.
pub fn remove(dir: &Path, sites: &mut Vec<Site>, id: &str) -> Result<(), String> {
    let i = sites.iter().position(|s| s.id == id).ok_or("That site isn't in Theater")?;
    let s = sites.remove(i);
    for f in [&s.art, &s.icon] { if !f.is_empty() && !f.contains('/') && !f.contains('\\') { let _ = std::fs::remove_file(art_dir(dir).join(f)); } }
    Ok(())
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
    fn art_files() {
        assert_eq!(art_name("s1", "art", "image/png", "https://a/x"), "s1-art.png");
        assert_eq!(art_name("s1", "icon", "", "https://a/favicon.ico"), "s1-icon.ico");
        assert_eq!(mime_for("s1-art.webp"), "image/webp");
    }
}
