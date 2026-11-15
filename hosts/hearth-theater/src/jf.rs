//! Jellyfin and sync, the parts with no network in them (main.rs's remote.rs does the talking).
//!
//! A Jellyfin server is a site of kind "jellyfin": its url is the server's address ("http://chris-hearth:8096/").
//! Connect signs in with Jellyfin's Quick Connect (a code you approve in Jellyfin while signed in there), so Theater
//! never sees your password; the token it gets is kept in jellyfin/<site>.json (only you can read it) and never
//! synced: each device connects once. Its rows come from Jellyfin's own lists (where you stopped, next up, recently
//! added, movies, shows), and its titles open in Jellyfin's own web player, which keeps where you stopped on the
//! server, so every device carries on there.
//!
//! Sync: Hearth Link (hosts/hearth-link sync.rs) carries your sites and titles between your paired devices. It reads
//! sites.json and titles.json, and puts what changed on another device in link-inbox.json, which Theater applies
//! (`apply_inbox`); a file "sync-off" means this device doesn't sync Theater.

use crate::{Catalog, Item, Row, Site, Title};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub const KIND: &str = "jellyfin";
pub const PORT: u16 = 8096;

/// What Theater keeps of a connected Jellyfin (jellyfin/<site>.json, mode 0600).
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct Login {
    pub url: String,
    pub token: String,
    pub user_id: String,
    pub user: String,
    pub server_id: String,
}

pub fn login_file(dir: &Path, site: &str) -> PathBuf { dir.join("jellyfin").join(format!("{site}.json")) }

pub fn load_login(dir: &Path, site: &str) -> Option<Login> {
    if !crate::safe_id(site) { return None; }
    let l: Login = serde_json::from_slice(&std::fs::read(login_file(dir, site)).ok()?).ok()?;
    (!l.token.is_empty() && !l.user_id.is_empty()).then_some(l)
}

/// Saved through a temporary file and a rename, readable by you only.
pub fn save_login(dir: &Path, site: &str, l: &Login) -> Result<(), String> {
    if !crate::safe_id(site) { return Err("bad site id".into()); }
    let f = login_file(dir, site);
    let folder = f.parent().unwrap();
    std::fs::create_dir_all(folder).map_err(|e| e.to_string())?;
    let tmp = f.with_extension("json.tmp");
    {
        let mut o = std::fs::OpenOptions::new();
        o.write(true).create(true).truncate(true);
        #[cfg(unix)]
        { use std::os::unix::fs::OpenOptionsExt; o.mode(0o600); }
        use std::io::Write;
        let mut file = o.open(&tmp).map_err(|e| e.to_string())?;
        file.write_all(&serde_json::to_vec(l).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
    }
    #[cfg(unix)]
    { use std::os::unix::fs::PermissionsExt; let _ = std::fs::set_permissions(folder, std::fs::Permissions::from_mode(0o700)); }
    std::fs::rename(&tmp, f).map_err(|e| e.to_string())
}

pub fn forget_login(dir: &Path, site: &str) { if crate::safe_id(site) { let _ = std::fs::remove_file(login_file(dir, site)); } }

/// A server's address as Theater keeps it: http(s), host and port, ending in "/" (no path: Jellyfin's own
/// pages are under it).
pub fn base_url(input: &str) -> Result<String, String> {
    let mut u = url::Url::parse(input.trim()).map_err(|_| "That isn't a Jellyfin address".to_string())?;
    if !matches!(u.scheme(), "http" | "https") || u.host_str().is_none() { return Err("That isn't a Jellyfin address".into()); }
    // Jellyfin's own pages are under <server>/web/: the server is what's before that.
    let p = u.path();
    let path = match p.find("/web/") { Some(i) => p[..i].to_string(), None => p.trim_end_matches('/').trim_end_matches("/web").to_string() };
    u.set_path(&format!("{path}/"));
    u.set_query(None);
    u.set_fragment(None);
    Ok(u.to_string())
}

/// Jellyfin's Authorization header (Quick Connect, and every call after it with the token).
pub fn auth_header(device: &str, device_id: &str, version: &str, token: &str) -> String {
    let clean = |s: &str| s.chars().filter(|c| *c != '"' && *c != ',' && !c.is_control()).take(60).collect::<String>();
    let mut h = format!("MediaBrowser Client=\"Hearth Theater\", Device=\"{}\", DeviceId=\"{}\", Version=\"{}\"", clean(device), clean(device_id), clean(version));
    if !token.is_empty() { h.push_str(&format!(", Token=\"{}\"", clean(token))); }
    h
}

/// "24 min left", "1 h 10 min left" ("" when it isn't known).
pub fn left_text(runtime_ticks: u64, position_ticks: u64) -> String {
    if runtime_ticks == 0 || position_ticks == 0 || position_ticks >= runtime_ticks { return String::new(); }
    let min = ((runtime_ticks - position_ticks) as f64 / 600_000_000.0).ceil() as u64;
    match min { 0 => String::new(), m if m < 60 => format!("{m} min left"), m if m % 60 == 0 => format!("{} h left", m / 60), m => format!("{} h {} min left", m / 60, m % 60) }
}

/// One of Jellyfin's items as a Theater title: it opens in Jellyfin's web player on that server.
pub fn item(v: &Value, base: &str, server_id: &str) -> Option<Item> {
    let id = v["Id"].as_str().filter(|s| !s.is_empty() && s.len() <= 64 && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'))?;
    let typ = v["Type"].as_str().unwrap_or("");
    let name = v["Name"].as_str().unwrap_or("").trim();
    let title = if typ == "Episode" {
        let series = v["SeriesName"].as_str().unwrap_or("").trim();
        let se = match (v["ParentIndexNumber"].as_u64(), v["IndexNumber"].as_u64()) {
            (Some(s), Some(e)) => format!("S{s} E{e}"),
            (None, Some(e)) => format!("E{e}"),
            _ => name.to_string(),
        };
        if series.is_empty() { name.to_string() } else { format!("{series} · {se}") }
    } else { name.to_string() };
    if title.is_empty() { return None; }
    let kind = match typ { "Movie" => "movie", "Series" => "series", "Episode" => "episode", _ => "video" };
    let image = if v["ImageTags"]["Primary"].is_string() {
        format!("{base}Items/{id}/Images/Primary?maxHeight={}&quality=85", if kind == "episode" || kind == "video" { 360 } else { 480 })
    } else if let (Some(sid), "episode") = (v["SeriesId"].as_str().filter(|s| s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')), kind) {
        format!("{base}Items/{sid}/Images/Primary?maxHeight=480&quality=85")
    } else { String::new() };
    let ud = &v["UserData"];
    let (run, pos) = (v["RunTimeTicks"].as_u64().unwrap_or(0), ud["PlaybackPositionTicks"].as_u64().unwrap_or(0));
    let progress = ud["PlayedPercentage"].as_f64().map(|p| p / 100.0).unwrap_or(if run > 0 { pos as f64 / run as f64 } else { 0.0 });
    let sid: String = server_id.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').take(64).collect();
    Some(Item {
        title: title.chars().take(160).collect(),
        url: format!("{base}web/index.html#/details?id={id}{}", if sid.is_empty() { String::new() } else { format!("&serverId={sid}") }),
        image,
        art: String::new(),
        kind: kind.into(),
        episodes: if kind == "series" { v["RecursiveItemCount"].as_u64().unwrap_or(0).min(100_000) as u32 } else { 0 },
        year: v["ProductionYear"].as_u64().map(|y| y.to_string()).unwrap_or_default(),
        desc: v["Overview"].as_str().unwrap_or("").chars().take(300).collect(),
        progress: if pos > 0 { progress.clamp(0.0, 1.0) } else { 0.0 },
        left: if pos > 0 { left_text(run, pos) } else { String::new() },
    })
}

/// Jellyfin's lists ({"Items": [...]}, or a plain array for Latest) as rows; empty rows are left out.
pub fn rows(lists: &[(&str, &Value)], base: &str, server_id: &str) -> Vec<Row> {
    lists.iter().filter_map(|(name, v)| {
        let arr = v.as_array().or_else(|| v["Items"].as_array())?;
        let items: Vec<Item> = arr.iter().filter_map(|x| item(x, base, server_id)).collect();
        (!items.is_empty()).then(|| Row { name: name.to_string(), items })
    }).collect()
}

pub fn catalog(rows: Vec<Row>, error: String) -> Catalog {
    Catalog { updated: crate::now_ms(), rows, search: KIND.into(), signin: false, sources: vec![KIND.into()], error }
}

/// A title of a Jellyfin site keeps its address as it is (its title is in the #part, which other sites' links lose).
pub fn own_link(site_url: &str, link: &str) -> bool {
    link.starts_with(site_url) && link[site_url.len()..].starts_with("web/index.html#/details?id=")
        && !link.chars().any(|c| c.is_whitespace() || c.is_control() || c == '"' || c == '<')
}

// ------------------------------------------------------------------ finding Jellyfin

/// A place a Jellyfin may be: the address to try and how to say where it is.
#[derive(Clone, Debug, PartialEq)]
pub struct Spot { pub url: String, pub where_: String, pub rank: u8 }

/// `tailscale status --json`'s devices (this one too) as places to try, with `home` the home servers' names.
pub fn tailscale_spots(status: &Value, home: &[String]) -> Vec<Spot> {
    let mut out = Vec::new();
    let me = std::iter::once((&status["Self"], true));
    let peers = status["Peer"].as_object().into_iter().flatten().map(|(_, v)| (v, false));
    for (p, is_self) in me.chain(peers) {
        if !is_self && p["Online"] != true { continue; }
        let host = p["HostName"].as_str().unwrap_or("").to_string();
        let dns = p["DNSName"].as_str().unwrap_or("").split('.').next().unwrap_or("").to_string();
        let ip = p["TailscaleIPs"].as_array().into_iter().flatten().filter_map(Value::as_str).find(|s| s.parse::<std::net::Ipv4Addr>().is_ok());
        let Some(ip) = ip else { continue };
        let is_home = home.iter().any(|h| h.eq_ignore_ascii_case(&host) || h.eq_ignore_ascii_case(&dns));
        let name = if dns.is_empty() { host.clone() } else { dns.clone() };
        let where_ = if is_self { "on this device".to_string() } else if is_home { "your home server, over Tailscale".into() } else { format!("{name}, over Tailscale") };
        out.push(Spot { url: format!("http://{ip}:{PORT}/"), where_, rank: if is_home { 0 } else if is_self { 2 } else { 1 } });
    }
    out
}

/// The name to keep in the address instead of a Tailscale number, when this device can look it up (MagicDNS).
pub fn ts_name(status: &Value, ip: &str) -> Option<String> {
    let all = std::iter::once(&status["Self"]).chain(status["Peer"].as_object().into_iter().flatten().map(|(_, v)| v));
    for p in all {
        if p["TailscaleIPs"].as_array().into_iter().flatten().any(|x| x.as_str() == Some(ip)) {
            let dns = p["DNSName"].as_str().unwrap_or("").split('.').next().unwrap_or("");
            if !dns.is_empty() && dns.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') { return Some(dns.to_string()); }
        }
    }
    None
}

/// Jellyfin's answer on its discovery port (7359) to "Who is JellyfinServer?": {"Address", "Id", "Name"}.
pub fn lan_spot(answer: &[u8]) -> Option<Spot> {
    let v: Value = serde_json::from_slice(answer).ok()?;
    let a = base_url(v["Address"].as_str()?).ok()?;
    Some(Spot { url: a, where_: "on your home network".into(), rank: 3 })
}

/// "10.10" from "10.10.7".
pub fn short_version(v: &str) -> String { v.split('.').take(2).collect::<Vec<_>>().join(".") }

// ------------------------------------------------------------------ sync (Hearth Link)

pub fn sync_off_file(dir: &Path) -> PathBuf { dir.join("sync-off") }
pub fn inbox_file(dir: &Path) -> PathBuf { dir.join("link-inbox.json") }
pub fn sync_on(dir: &Path) -> bool { !sync_off_file(dir).exists() }

/// What of a site follows you (the same in hearth-link sync.rs): not its pictures (each device gets them itself),
/// not how often you opened it here.
pub fn site_field(s: &Site) -> Value {
    json!({"id": s.id, "name": s.name, "url": s.url, "category": s.category, "favorite": s.favorite, "hidden": s.hidden, "added": s.added, "kind": s.kind})
}

/// "3 min ago" (for the devices list).
pub fn ago(now: u64, t: u64) -> String {
    let m = now.saturating_sub(t) / 60_000;
    match m { 0 => "just now".into(), m if m < 60 => format!("{m} min ago"), m if m < 1440 => format!("{} h ago", m / 60), m if m < 2880 => "yesterday".into(), m => format!("{} days ago", m / 1440) }
}

/// Sync's state for the Sync dialog, from `hearth-link status` (None: Hearth Link isn't running).
pub fn sync_status(on: bool, link: Option<&Value>, now: u64) -> Value {
    let Some(st) = link else { return json!({"on": on, "server": null, "last": 0, "devices": [], "link": false}) };
    let servers: Vec<String> = st["storage"]["servers"].as_array().into_iter().flatten().filter_map(|s| s["name"].as_str().map(String::from)).collect();
    let mut last = 0u64;
    let devices: Vec<Value> = st["peers"].as_array().into_iter().flatten().filter(|p| p["friend"] != true).map(|p| {
        let at = p["sync"]["at"].as_u64().unwrap_or(0);
        last = last.max(at);
        let state = if at > 0 { format!("synced {}", ago(now, at)) } else if p["online"] == true { "online, not synced yet".into() } else { "not seen lately".into() };
        json!({"name": p["name"].as_str().unwrap_or("?"), "state": state})
    }).collect();
    json!({"on": on, "server": servers.first(), "last": last, "devices": devices, "link": true})
}

/// Puts what Hearth Link brought from your other devices in place: {"sites": {id: site | null}, "titles": {key: title
/// | null}}. A site removed elsewhere goes here too; one added elsewhere with a link you already have here is left
/// out (your tile stays). Returns the ids of new sites (to read their catalog and picture) and the ids removed.
pub fn apply_inbox(sites: &mut Vec<Site>, titles: &mut Vec<Title>, inbox: &Value) -> (Vec<String>, Vec<String>) {
    let (mut added, mut removed) = (Vec::new(), Vec::new());
    for (id, v) in inbox["sites"].as_object().into_iter().flatten() {
        if !crate::safe_id(id) { continue; }
        if v.is_null() {
            if sites.iter().any(|s| &s.id == id) { removed.push(id.clone()); }
            continue;
        }
        let Ok(url) = crate::normalize_url(v["url"].as_str().unwrap_or("")) else { continue };
        let name = crate::clean_title(v["name"].as_str().unwrap_or(""));
        let name = if name.is_empty() { crate::host_name(&url) } else { name.chars().take(crate::MAX_NAME).collect() };
        let hidden: Vec<String> = v["hidden"].as_array().into_iter().flatten().filter_map(Value::as_str).take(60).map(|s| s.chars().take(80).collect()).collect();
        let kind = if v["kind"] == KIND { KIND.to_string() } else { String::new() };
        match sites.iter_mut().find(|s| &s.id == id) {
            Some(s) => {
                s.name = name; s.url = url; s.category = crate::fix_category(v["category"].as_str().unwrap_or(""));
                s.favorite = v["favorite"] == true; s.hidden = hidden; s.kind = kind;
            }
            None => {
                if sites.iter().any(|s| crate::same_site(&s.url, &url)) { continue; }
                sites.push(Site { id: id.clone(), name, url, category: crate::fix_category(v["category"].as_str().unwrap_or("")), favorite: v["favorite"] == true,
                    hidden, kind, added: v["added"].as_u64().unwrap_or_else(crate::now_ms), ..Default::default() });
                added.push(id.clone());
            }
        }
    }
    for (key, v) in inbox["titles"].as_object().into_iter().flatten() {
        let pos = titles.iter().position(|t| &title_key(t) == key);
        if v.is_null() { if let Some(i) = pos { titles.remove(i); } continue; }
        let Ok(t) = serde_json::from_value::<Title>(v.clone()) else { continue };
        if !crate::safe_id(&t.site) || t.item.title.is_empty() || !(t.item.url.starts_with("http://") || t.item.url.starts_with("https://")) { continue; }
        let mut t = t;
        t.item.art = String::new();   // posters are each device's own
        t.item.title = t.item.title.chars().take(160).collect();
        match pos {
            Some(i) => { let old = &titles[i]; t.opens = old.opens.max(t.opens); t.item.art = old.item.art.clone(); titles[i] = t; }
            None => titles.push(t),
        }
    }
    (added, removed)
}

/// A title's name in sync (its site and link).
pub fn title_key(t: &Title) -> String {
    // FNV-1a, 64 bits: short, the same on every device, nothing odd in it.
    let mut h: u64 = 0xcbf29ce484222325;
    for b in t.site.bytes().chain([0u8]).chain(t.item.url.bytes()) { h ^= b as u64; h = h.wrapping_mul(0x100000001b3); }
    format!("{h:016x}")
}

/// What of a title follows you (the same in hearth-link sync.rs).
pub fn title_field(t: &Title) -> Value {
    let mut it = t.item.clone();
    it.art = String::new();
    let mut v = serde_json::to_value(Title { item: it, ..t.clone() }).unwrap_or(Value::Null);
    if let Some(m) = v.as_object_mut() { m.remove("opens"); m.remove("art"); }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses() {
        assert_eq!(base_url("http://chris-hearth:8096").unwrap(), "http://chris-hearth:8096/");
        assert_eq!(base_url("http://chris-hearth:8096/web/").unwrap(), "http://chris-hearth:8096/");
        assert_eq!(base_url("https://jf.example.com/jellyfin/web/index.html#/home").unwrap(), "https://jf.example.com/jellyfin/");
        assert!(base_url("file:///etc").is_err());
        assert!(own_link("http://h:8096/", "http://h:8096/web/index.html#/details?id=abc&serverId=x"));
        assert!(!own_link("http://h:8096/", "http://other:8096/web/index.html#/details?id=abc"));
        assert!(!own_link("http://h:8096/", "http://h:8096/web/index.html#/details?id=a\"><x"));
    }

    #[test]
    fn header_and_time_left() {
        let h = auth_header("Ally\"", "d1", "0.1.0", "");
        assert_eq!(h, "MediaBrowser Client=\"Hearth Theater\", Device=\"Ally\", DeviceId=\"d1\", Version=\"0.1.0\"");
        assert!(auth_header("a", "b", "c", "tok").ends_with(", Token=\"tok\""));
        assert_eq!(left_text(60 * 60 * 10_000_000, 36 * 60 * 10_000_000), "24 min left");
        assert_eq!(left_text(120 * 60 * 10_000_000, 50 * 60 * 10_000_000), "1 h 10 min left");
        assert_eq!(left_text(120 * 60 * 10_000_000, 60 * 60 * 10_000_000), "1 h left");
        assert_eq!(left_text(0, 5), "");
    }

    #[test]
    fn items_and_rows() {
        let base = "http://h:8096/";
        let ep = json!({"Id": "e1", "Type": "Episode", "Name": "Pilot", "SeriesName": "The Long Road", "SeriesId": "s1", "ParentIndexNumber": 2, "IndexNumber": 5,
                        "RunTimeTicks": 60u64 * 60 * 10_000_000, "UserData": {"PlaybackPositionTicks": 36u64 * 60 * 10_000_000, "PlayedPercentage": 60.0}});
        let i = item(&ep, base, "srv").unwrap();
        assert_eq!(i.title, "The Long Road · S2 E5");
        assert_eq!(i.kind, "episode");
        assert_eq!(i.url, "http://h:8096/web/index.html#/details?id=e1&serverId=srv");
        assert_eq!(i.image, "http://h:8096/Items/s1/Images/Primary?maxHeight=480&quality=85");
        assert!((i.progress - 0.6).abs() < 1e-9);
        assert_eq!(i.left, "24 min left");
        let show = json!({"Id": "s1", "Type": "Series", "Name": "The Long Road", "RecursiveItemCount": 16, "ProductionYear": 2024, "ImageTags": {"Primary": "t"}});
        let s = item(&show, base, "").unwrap();
        assert_eq!((s.kind.as_str(), s.episodes, s.year.as_str(), s.progress), ("series", 16, "2024", 0.0));
        assert_eq!(s.url, "http://h:8096/web/index.html#/details?id=s1");
        assert!(item(&json!({"Id": "../x", "Name": "n"}), base, "").is_none());
        let latest = json!([show]);
        let resume = json!({"Items": [ep]});
        let empty = json!({"Items": []});
        let r = rows(&[("Continue watching", &resume), ("Next up", &empty), ("Recently added", &latest)], base, "srv");
        assert_eq!(r.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(), ["Continue watching", "Recently added"]);
    }

    #[test]
    fn finding() {
        let st = json!({"Self": {"HostName": "ally", "DNSName": "ally.tail.ts.net.", "TailscaleIPs": ["100.64.0.2", "fd7a::2"]},
            "Peer": {"k1": {"HostName": "chris-hearth", "DNSName": "chris-hearth.tail.ts.net.", "TailscaleIPs": ["100.64.0.1"], "Online": true},
                     "k2": {"HostName": "phone", "DNSName": "phone.tail.ts.net.", "TailscaleIPs": ["100.64.0.3"], "Online": false}}});
        let s = tailscale_spots(&st, &["Chris-Hearth".to_string()]);
        assert_eq!(s.len(), 2);
        let home = s.iter().find(|x| x.rank == 0).unwrap();
        assert_eq!(home.url, "http://100.64.0.1:8096/");
        assert_eq!(home.where_, "your home server, over Tailscale");
        assert_eq!(ts_name(&st, "100.64.0.1").as_deref(), Some("chris-hearth"));
        let l = lan_spot(br#"{"Address":"http://192.168.1.5:8096","Id":"x","Name":"Den"}"#).unwrap();
        assert_eq!(l.url, "http://192.168.1.5:8096/");
        assert_eq!(short_version("10.10.7"), "10.10");
    }

    #[test]
    fn sync_dialog_state() {
        let now = 10_000_000;
        let st = json!({"peers": [{"name": "ROG Ally", "sync": {"at": now - 120_000}, "online": true}, {"name": "Friend", "friend": true}, {"name": "Living room PC", "online": false}],
                        "storage": {"servers": [{"name": "chris-hearth"}]}});
        let v = sync_status(true, Some(&st), now);
        assert_eq!(v["server"], "chris-hearth");
        assert_eq!(v["last"], now - 120_000);
        assert_eq!(v["devices"], json!([{"name": "ROG Ally", "state": "synced 2 min ago"}, {"name": "Living room PC", "state": "not seen lately"}]));
        assert_eq!(sync_status(false, None, now)["link"], false);
    }

    #[test]
    fn inbox() {
        let mut sites = vec![Site { id: "s1".into(), name: "A".into(), url: "https://a.example/".into(), category: "TV".into(), art: "s1-art.png".into(), ..Default::default() },
                             Site { id: "s2".into(), name: "B".into(), url: "https://b.example/".into(), ..Default::default() }];
        let mut titles = vec![];
        let t = Title { site: "s1".into(), item: Item { title: "Show".into(), url: "https://a.example/show".into(), ..Default::default() }, favorite: true, opened: 5, opens: 2 };
        let key = title_key(&t);
        let inbox = json!({"sites": {
            "s1": {"name": "A2", "url": "https://a.example/", "category": "Anime", "favorite": true, "hidden": ["News"]},
            "s2": null,
            "s3": {"name": "Jelly", "url": "http://h:8096/", "category": "Movies", "kind": "jellyfin"},
            "s4": {"name": "Dup", "url": "https://b.example/"},
            "../x": {"name": "bad", "url": "https://c.example/"}},
            "titles": serde_json::Map::from_iter([(key.clone(), title_field(&t))])});
        let (added, removed) = apply_inbox(&mut sites, &mut titles, &inbox);
        assert_eq!(added, ["s3"]);
        assert_eq!(removed, ["s2"]);
        let s1 = &sites[0];
        assert_eq!((s1.name.as_str(), s1.category.as_str(), s1.favorite, s1.art.as_str()), ("A2", "Anime", true, "s1-art.png"));
        assert_eq!(sites.iter().find(|s| s.id == "s3").unwrap().kind, "jellyfin");
        assert_eq!(titles.len(), 1);
        assert!(titles[0].favorite);
        assert_eq!(title_key(&titles[0]), key);
        let (_, _) = apply_inbox(&mut sites, &mut titles, &json!({"titles": serde_json::Map::from_iter([(key, Value::Null)])}));
        assert!(titles.is_empty());
    }
}
