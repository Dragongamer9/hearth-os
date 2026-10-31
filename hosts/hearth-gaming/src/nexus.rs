//! Nexus Mods: the account (a personal API key, kept in ~/.config/nimbus/nexus.json, readable by you only), the
//! v1 REST API Hearth Mods uses (mods, files, download links, the game list), `nxm://` links (the site's "Mod
//! Manager Download" button) and which Nexus game a game in Hearth's library is.
//!
//! Downloads: Premium accounts can ask the API for a file's download link directly. Free accounts can't: each file
//! needs the "Mod Manager Download" button on the site, which makes an nxm:// link carrying a short-lived key; Hearth
//! turns that key into the download (downloads.rs). Every web request goes through curl (on every Hearth OS) with
//! the address and the key given to curl on its standard input, never on its command line (any program can read a
//! command line). Nexus reports how many requests are left this hour and day; the last numbers are kept in QUOTA.

use crate::util;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Mutex;

pub const API: &str = "https://api.nexusmods.com/v1";
pub const SITE: &str = "https://www.nexusmods.com";
/// Where you make a personal API key (bottom of the page).
pub const KEY_PAGE: &str = "https://next.nexusmods.com/settings/api-keys";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub key: String,
    pub user_id: u64,
    pub name: String,
    pub premium: bool,
    pub supporter: bool,
    #[serde(default)]
    pub avatar: String,
    #[serde(default)]
    pub checked: u64,
}
impl Account {
    /// The account for the page: everything but the key.
    pub fn public(&self) -> Value {
        json!({"userId": self.user_id, "name": self.name, "premium": self.premium, "supporter": self.supporter, "avatar": self.avatar, "checked": self.checked})
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Quota { pub hourly: Option<u64>, pub daily: Option<u64>, pub at: u64 }
pub static QUOTA: Mutex<Quota> = Mutex::new(Quota { hourly: None, daily: None, at: 0 });
pub fn quota() -> Quota { *QUOTA.lock().unwrap_or_else(|e| e.into_inner()) }

fn account_file() -> PathBuf { util::config_dir().join("nexus.json") }

/// The signed-in account, if any (with its key).
pub fn account() -> Option<Account> {
    let a: Account = serde_json::from_value(util::read_json(&account_file())).ok()?;
    if a.key.trim().is_empty() { None } else { Some(a) }
}

/// Checks a key with Nexus and keeps it (only you can read the file).
pub fn sign_in(key: &str) -> Result<Account, String> {
    let key = key.trim();
    if key.len() < 20 || key.contains(char::is_whitespace) { return Err("That doesn't look like a Nexus Mods API key. Copy the whole key from your Nexus account's API Keys page.".into()); }
    let v = call(key, "/users/validate.json")?;
    let a = Account {
        key: key.to_string(),
        user_id: v["user_id"].as_u64().unwrap_or(0),
        name: v["name"].as_str().unwrap_or("").to_string(),
        premium: v["is_premium"] == true || v["is_premium?"] == true,
        supporter: v["is_supporter"] == true || v["is_supporter?"] == true,
        avatar: v["profile_url"].as_str().unwrap_or("").to_string(),
        checked: util::now_ms(),
    };
    save_account(&a)?;
    Ok(a)
}

/// Checks the saved key again (Premium may have started or ended). Keeps the old details if Nexus can't be reached.
pub fn refresh() -> Result<Account, String> {
    let a = account().ok_or("Sign in to Nexus Mods first.")?;
    match sign_in(&a.key) {
        Ok(n) => Ok(n),
        Err(e) if e.starts_with("No connection") => Ok(a),
        Err(e) => Err(e),
    }
}

fn save_account(a: &Account) -> Result<(), String> {
    let f = account_file();
    let bytes = serde_json::to_vec_pretty(a).map_err(|e| e.to_string())?;
    util::write_atomic(&f, &bytes).map_err(|e| format!("Couldn't keep the sign-in ({e})"))?;
    #[cfg(unix)]
    { use std::os::unix::fs::PermissionsExt; let _ = std::fs::set_permissions(&f, std::fs::Permissions::from_mode(0o600)); }
    Ok(())
}

pub fn sign_out() { let _ = std::fs::remove_file(account_file()); }

// ------------------------------------------------------------------ the API

/// One request to the API: the reply as JSON, or a message saying what went wrong in plain words.
pub fn call(key: &str, path: &str) -> Result<Value, String> {
    let url = format!("{API}{path}");
    let (status, headers, body) = curl_get(&url, &[("apikey", key), ("Application-Name", "Hearth"), ("Application-Version", env!("CARGO_PKG_VERSION")), ("Accept", "application/json")], 30)?;
    note_quota(&headers);
    let v: Value = serde_json::from_str(&body).unwrap_or(Value::Null);
    let said = v["message"].as_str().or(v["error"].as_str()).unwrap_or("").to_string();
    match status {
        200..=299 => Ok(v),
        401 => Err("Nexus Mods didn't accept the API key (it may have been deleted). Sign in again.".into()),
        403 if said.to_lowercase().contains("premium") => Err("That needs Nexus Premium. Use the \"Mod Manager Download\" button on the mod's page instead.".into()),
        403 => Err(format!("Nexus Mods said no{}", if said.is_empty() { ".".into() } else { format!(": {said}") })),
        404 => Err("Nexus Mods doesn't have that (the mod or file may have been removed or hidden).".into()),
        410 => Err("That download link has run out. Press \"Mod Manager Download\" on the site again.".into()),
        429 => Err("Nexus Mods' request limit for this hour is used up. Try again later.".into()),
        0 => Err("No connection to Nexus Mods. Is this PC online?".into()),
        s => Err(format!("Nexus Mods had a problem ({s}{}).", if said.is_empty() { String::new() } else { format!(": {said}") })),
    }
}

fn note_quota(headers: &str) {
    let mut q = quota();
    for line in headers.lines() {
        let Some((k, v)) = line.split_once(':') else { continue };
        let n = v.trim().parse::<u64>().ok();
        match k.trim().to_ascii_lowercase().as_str() { "x-rl-hourly-remaining" => q.hourly = n, "x-rl-daily-remaining" => q.daily = n, _ => continue }
        q.at = util::now_ms();
    }
    *QUOTA.lock().unwrap_or_else(|e| e.into_inner()) = q;
}

/// A GET through curl: (status, response headers, body). The address and headers go in on standard input.
pub fn curl_get(url: &str, headers: &[(&str, &str)], timeout_s: u32) -> Result<(u16, String, String), String> {
    use std::io::Write;
    let mut cfg = format!("url = {}\n", curl_quote(url));
    for (k, v) in headers { cfg.push_str(&format!("header = {}\n", curl_quote(&format!("{k}: {v}")))); }
    let mut child = std::process::Command::new("curl")
        .args(["-sS", "-L", "--proto", "=https", "-m", &timeout_s.to_string(), "-A", concat!("Hearth/", env!("CARGO_PKG_VERSION")), "-D", "-", "-w", "\n%{http_code}", "-K", "-"])
        .stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped())
        .spawn().map_err(|e| format!("No connection (curl: {e})"))?;
    if let Some(mut i) = child.stdin.take() { let _ = i.write_all(cfg.as_bytes()); }
    let out = child.wait_with_output().map_err(|e| format!("No connection ({e})"))?;
    let all = String::from_utf8_lossy(&out.stdout).to_string();
    let (rest, code) = all.rsplit_once('\n').unwrap_or((&all, "0"));
    let status: u16 = code.trim().parse().unwrap_or(0);
    if status == 0 { return Err(format!("No connection to Nexus Mods ({})", String::from_utf8_lossy(&out.stderr).trim())); }
    // -L: one header block per redirect; the last one is the reply's.
    let (head, body) = split_head(rest);
    Ok((status, head, body))
}

fn split_head(s: &str) -> (String, String) {
    let mut rest = s;
    let mut head = String::new();
    while rest.starts_with("HTTP/") {
        let Some(i) = rest.find("\r\n\r\n").map(|i| (i, 4)).or_else(|| rest.find("\n\n").map(|i| (i, 2))) else { break };
        head = rest[..i.0].to_string();
        rest = &rest[i.0 + i.1..];
    }
    (head, rest.to_string())
}

/// A value for curl's config: in double quotes, with \ " and line breaks escaped.
pub fn curl_quote(v: &str) -> String {
    let mut o = String::from("\"");
    for c in v.chars() { match c { '\\' => o.push_str("\\\\"), '"' => o.push_str("\\\""), '\n' => o.push_str("\\n"), '\r' => o.push_str("\\r"), '\t' => o.push_str("\\t"), c => o.push(c) } }
    o.push('"');
    o
}

fn domain_ok(d: &str) -> bool { !d.is_empty() && d.len() <= 64 && d.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') }

pub fn mod_info(key: &str, domain: &str, mod_id: u64) -> Result<Value, String> {
    if !domain_ok(domain) { return Err("That isn't a Nexus game.".into()); }
    call(key, &format!("/games/{domain}/mods/{mod_id}.json"))
}
pub fn files(key: &str, domain: &str, mod_id: u64) -> Result<Value, String> {
    if !domain_ok(domain) { return Err("That isn't a Nexus game.".into()); }
    call(key, &format!("/games/{domain}/mods/{mod_id}/files.json"))
}
pub fn file_info(key: &str, domain: &str, mod_id: u64, file_id: u64) -> Result<Value, String> {
    if !domain_ok(domain) { return Err("That isn't a Nexus game.".into()); }
    call(key, &format!("/games/{domain}/mods/{mod_id}/files/{file_id}.json"))
}

/// A file's download address. Premium: straight from the API. Free: only with the key and expiry from an nxm link.
pub fn download_link(acct: &Account, domain: &str, mod_id: u64, file_id: u64, nxm_key: Option<(&str, u64)>) -> Result<String, String> {
    if !domain_ok(domain) { return Err("That isn't a Nexus game.".into()); }
    let mut path = format!("/games/{domain}/mods/{mod_id}/files/{file_id}/download_link.json");
    match nxm_key {
        Some((k, exp)) => path.push_str(&format!("?key={}&expires={exp}", urlencode(k))),
        None if !acct.premium => return Err("Downloading straight from Hearth needs Nexus Premium. Use the \"Mod Manager Download\" button on the mod's page instead.".into()),
        None => {}
    }
    let v = call(&acct.key, &path)?;
    let links = v.as_array().cloned().unwrap_or_default();
    // Nexus lists its CDN first, then mirrors; any https one will do.
    links.iter().filter_map(|l| l["URI"].as_str()).find(|u| u.starts_with("https://")).map(str::to_string)
        .ok_or_else(|| "Nexus Mods gave no download address for that file.".to_string())
}

/// Every game on Nexus Mods (id, name, domain_name), kept for a week.
pub fn games(key: &str) -> Value {
    let f = util::data_dir().join("mods").join("nexus-games.json");
    let cached = util::read_json(&f);
    let fresh = std::fs::metadata(&f).ok().and_then(|m| m.modified().ok()).and_then(|t| t.elapsed().ok()).map(|d| d.as_secs() < 7 * 86400).unwrap_or(false);
    if fresh && cached.is_array() { return cached; }
    match call(key, "/games.json") {
        Ok(v) if v.is_array() => {
            let slim: Vec<Value> = v.as_array().unwrap().iter().map(|g| json!({"id": g["id"], "name": g["name"], "domain": g["domain_name"]})).collect();
            let v = Value::Array(slim);
            let _ = util::write_json(&f, &v);
            v
        }
        _ => if cached.is_array() { cached } else { Value::Array(vec![]) },
    }
}

/// The Nexus game for a game in Hearth's library: by its Steam app (the common ones), then by its name.
pub fn domain_for(g: &crate::games::GameEntry, games: &Value) -> Option<String> {
    if let Some(app) = g.steam_app { if let Some(d) = STEAM_DOMAINS.iter().find(|x| x.0 == app).map(|x| x.1) { return Some(d.to_string()); } }
    let want = squash(&g.title);
    if want.is_empty() { return None; }
    games.as_array()?.iter().find(|x| squash(x["name"].as_str().unwrap_or("")) == want).and_then(|x| x["domain"].as_str()).map(str::to_string)
}
fn squash(s: &str) -> String { s.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect() }

/// Steam app → Nexus game, for games whose names differ between the two (and the most modded ones).
pub const STEAM_DOMAINS: [(u64, &str); 24] = [
    (489830, "skyrimspecialedition"), (72850, "skyrim"), (611670, "skyrimspecialedition"), (377160, "fallout4"), (611660, "fallout4"),
    (22380, "newvegas"), (22490, "newvegas"), (22370, "fallout3"), (22300, "fallout3"), (22330, "oblivion"), (2623190, "oblivionremastered"),
    (22320, "morrowind"), (1716740, "starfield"), (1091500, "cyberpunk2077"), (1086940, "baldursgate3"), (292030, "witcher3"),
    (413150, "stardewvalley"), (976730, "enderalspecialedition"), (1151340, "fallout76"), (1245620, "eldenring"),
    (374320, "darksouls3"), (1174180, "reddeadredemption2"), (271590, "gta5"), (2420110, "horizonforbiddenwest"),
];

// ------------------------------------------------------------------ nxm:// links

/// "Mod Manager Download" on the site: nxm://<game>/mods/<mod>/files/<file>?key=..&expires=..&user_id=..
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Nxm { pub domain: String, pub mod_id: u64, pub file_id: u64, pub key: Option<String>, pub expires: Option<u64>, pub user_id: Option<u64> }

pub fn parse_nxm(s: &str) -> Result<Nxm, String> {
    let bad = || "That isn't a Nexus Mods download link.".to_string();
    let rest = s.trim().strip_prefix("nxm://").or_else(|| s.trim().strip_prefix("NXM://")).ok_or_else(bad)?;
    let (path, query) = rest.split_once('?').unwrap_or((rest, ""));
    let parts: Vec<&str> = path.trim_end_matches('/').split('/').collect();
    // Collections (nxm://<game>/collections/<slug>/revisions/<n>) aren't mods.
    if parts.get(1) == Some(&"collections") { return Err("That's a Nexus collection. Hearth Mods installs collections later; for now it installs single mods and Wabbajack modlists.".into()); }
    if parts.len() != 5 || parts[1] != "mods" || parts[3] != "files" { return Err(bad()); }
    let domain = parts[0].to_ascii_lowercase();
    if !domain_ok(&domain) { return Err(bad()); }
    let mut n = Nxm { domain, mod_id: parts[2].parse().map_err(|_| bad())?, file_id: parts[4].parse().map_err(|_| bad())?, ..Default::default() };
    for kv in query.split('&') {
        let (k, v) = kv.split_once('=').unwrap_or((kv, ""));
        match k { "key" => n.key = Some(urldecode(v)).filter(|s| !s.is_empty()), "expires" => n.expires = v.parse().ok(), "user_id" => n.user_id = v.parse().ok(), _ => {} }
    }
    Ok(n)
}

/// A mod's Files tab on the site, with the file picked (`nmm=1` makes its "Mod Manager Download" button the default).
pub fn files_page(domain: &str, mod_id: u64, file_id: Option<u64>) -> String {
    match file_id { Some(f) => format!("{SITE}/{domain}/mods/{mod_id}?tab=files&file_id={f}&nmm=1"), None => format!("{SITE}/{domain}/mods/{mod_id}?tab=files") }
}

pub fn urlencode(s: &str) -> String {
    s.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect()
}
pub fn urldecode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) { out.push(v); i += 3; continue; }
        }
        out.push(if b[i] == b'+' { b' ' } else { b[i] });
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

// ------------------------------------------------------------------ finding mods (Game Mode's Browse, update checks)

/// One mod for a list (Game Mode's Browse): what the v1 lists and the v2 search both give, in one shape.
pub fn mod_card(m: &Value) -> Value {
    let pic = m["picture_url"].as_str().or(m["pictureUrl"].as_str()).or(m["thumbnailUrl"].as_str()).unwrap_or("");
    json!({"modId": m["mod_id"].as_u64().or(m["modId"].as_u64()).unwrap_or(0), "name": m["name"].as_str().unwrap_or(""), "summary": m["summary"].as_str().unwrap_or(""),
           "author": m["author"].as_str().unwrap_or(""), "version": m["version"].as_str().unwrap_or(""), "picture": pic,
           "endorsements": m["endorsement_count"].as_u64().or(m["endorsements"].as_u64()).unwrap_or(0), "downloads": m["mod_downloads"].as_u64().or(m["downloads"].as_u64()).unwrap_or(0),
           "adult": m["contains_adult_content"].as_bool().or(m["adultContent"].as_bool()).unwrap_or(false)})
}

/// A game's lists on Nexus: `trending`, `latest_added` or `latest_updated` (the v1 API's own lists).
pub fn browse(key: &str, domain: &str, kind: &str) -> Result<Value, String> {
    if !domain_ok(domain) { return Err("That isn't a Nexus game.".into()); }
    let kind = match kind { "latest" | "latest_added" => "latest_added", "updated" | "latest_updated" => "latest_updated", _ => "trending" };
    let v = call(key, &format!("/games/{domain}/mods/{kind}.json"))?;
    Ok(json!(v.as_array().cloned().unwrap_or_default().iter().filter(|m| m["available"] != false).map(mod_card).collect::<Vec<_>>()))
}

/// The v2 API's search request (GraphQL) for mods of one game whose name has `q`.
pub fn search_body(domain: &str, q: &str) -> Value {
    json!({"query": "query HearthSearch($filter: ModsFilter, $count: Int) { mods(filter: $filter, count: $count) { nodes { modId name summary author version pictureUrl thumbnailUrl endorsements downloads adultContent } } }",
           "variables": {"count": 40, "filter": {"gameDomainName": [{"value": domain, "op": "EQUALS"}], "name": [{"value": q.trim(), "op": "WILDCARD"}]}}})
}
pub const API2: &str = "https://api.nexusmods.com/v2/graphql";
/// Searches a game's mods by name.
pub fn search(key: &str, domain: &str, q: &str) -> Result<Value, String> {
    if !domain_ok(domain) { return Err("That isn't a Nexus game.".into()); }
    if q.trim().is_empty() { return browse(key, domain, "trending"); }
    let body = serde_json::to_string(&search_body(domain, q)).unwrap_or_default();
    let (st, text) = crate::accounts::curl_post(API2, &[("apikey", key), ("Content-Type", "application/json"), ("Application-Name", "Hearth"), ("Application-Version", env!("CARGO_PKG_VERSION"))], &body, 30)?;
    let v: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
    if !(200..300).contains(&st) || v["data"].is_null() {
        return Err(format!("Nexus Mods' search didn't answer ({st}{}).", v["errors"][0]["message"].as_str().map(|e| format!(": {e}")).unwrap_or_default()));
    }
    Ok(json!(v["data"]["mods"]["nodes"].as_array().cloned().unwrap_or_default().iter().map(mod_card).collect::<Vec<_>>()))
}

/// The newest file that replaces `file_id`, following the files list's `file_updates` (old → new), when there is one.
pub fn newer_file(files: &Value, file_id: u64) -> Option<Value> {
    let ups = files["file_updates"].as_array().cloned().unwrap_or_default();
    let mut cur = file_id;
    for _ in 0..64 {
        match ups.iter().filter(|u| u["old_file_id"].as_u64() == Some(cur)).filter_map(|u| u["new_file_id"].as_u64()).max() { Some(n) if n != cur => cur = n, _ => break }
    }
    if cur == file_id { return None; }
    let f = files["files"].as_array().and_then(|l| l.iter().find(|f| f["file_id"].as_u64() == Some(cur)).cloned()).unwrap_or(Value::Null);
    if f["category_name"].as_str().map(|c| c == "DELETED").unwrap_or(false) { return None; }
    Some(json!({"fileId": cur, "name": f["name"], "version": f["version"], "fileName": f["file_name"]}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nxm_links() {
        let n = parse_nxm("nxm://skyrimspecialedition/mods/12604/files/35834?key=ab%2Bc&expires=1700000000&user_id=42").unwrap();
        assert_eq!(n, Nxm { domain: "skyrimspecialedition".into(), mod_id: 12604, file_id: 35834, key: Some("ab+c".into()), expires: Some(1700000000), user_id: Some(42) });
        let p = parse_nxm("nxm://fallout4/mods/100/files/200").unwrap();
        assert_eq!((p.mod_id, p.file_id, p.key), (100, 200, None));
        assert!(parse_nxm("nxm://fallout4/mods/x/files/200").is_err());
        assert!(parse_nxm("https://www.nexusmods.com/fallout4/mods/100").is_err());
        assert!(parse_nxm("nxm://../mods/1/files/2").is_err());
        assert!(parse_nxm("nxm://skyrimspecialedition/collections/abcd/revisions/3").unwrap_err().contains("collection"));
    }

    #[test]
    fn headers_and_quota() {
        let raw = "HTTP/2 302\r\nlocation: x\r\n\r\nHTTP/2 200\r\nX-RL-Hourly-Remaining: 99\r\nx-rl-daily-remaining: 19000\r\n\r\n{\"a\":1}";
        let (h, b) = split_head(raw);
        assert_eq!(b, "{\"a\":1}");
        note_quota(&h);
        let q = quota();
        assert_eq!((q.hourly, q.daily), (Some(99), Some(19000)));
    }

    #[test]
    fn games_by_steam_app_and_name() {
        let games = json!([{"name": "Stardew Valley", "domain": "stardewvalley"}, {"name": "Mount & Blade II: Bannerlord", "domain": "mountandblade2bannerlord"}]);
        let g = crate::games::GameEntry { title: "Fallout: New Vegas".into(), steam_app: Some(22380), ..Default::default() };
        assert_eq!(domain_for(&g, &games).as_deref(), Some("newvegas"));
        let b = crate::games::GameEntry { title: "Mount & Blade II: Bannerlord".into(), ..Default::default() };
        assert_eq!(domain_for(&b, &games).as_deref(), Some("mountandblade2bannerlord"));
        let none = crate::games::GameEntry { title: "Nope".into(), ..Default::default() };
        assert_eq!(domain_for(&none, &games), None);
    }

    #[test]
    fn encoding() {
        assert_eq!(urldecode("a%20b+c%2"), "a b c%2");
        assert_eq!(urlencode("a+b/c"), "a%2Bb%2Fc");
        assert_eq!(curl_quote("a\"b\\c"), "\"a\\\"b\\\\c\"");
        assert!(files_page("fallout4", 1, Some(2)).ends_with("file_id=2&nmm=1"));
    }

    #[test]
    fn updates_and_cards() {
        let files = json!({"files": [{"file_id": 1, "name": "Main", "version": "1.0"}, {"file_id": 5, "name": "Main", "version": "1.2", "file_name": "m-1.2.7z"}],
                           "file_updates": [{"old_file_id": 1, "new_file_id": 3}, {"old_file_id": 3, "new_file_id": 5}]});
        let n = newer_file(&files, 1).unwrap();
        assert_eq!((n["fileId"].as_u64(), n["version"].as_str()), (Some(5), Some("1.2")));
        assert!(newer_file(&files, 5).is_none());
        assert!(newer_file(&json!({}), 1).is_none());
        let loopy = json!({"file_updates": [{"old_file_id": 1, "new_file_id": 2}, {"old_file_id": 2, "new_file_id": 1}]});
        assert!(newer_file(&loopy, 1).is_some() || newer_file(&loopy, 1).is_none());   // never hangs
        let c = mod_card(&json!({"mod_id": 7, "name": "SkyUI", "endorsement_count": 9, "picture_url": "https://x/p.jpg"}));
        assert_eq!((c["modId"].as_u64(), c["endorsements"].as_u64(), c["picture"].as_str()), (Some(7), Some(9), Some("https://x/p.jpg")));
        let c2 = mod_card(&json!({"modId": 8, "name": "A", "thumbnailUrl": "https://t", "adultContent": true}));
        assert_eq!((c2["modId"].as_u64(), c2["adult"].as_bool()), (Some(8), Some(true)));
        let b = search_body("skyrimspecialedition", " sky ");
        assert_eq!(b["variables"]["filter"]["name"][0]["value"], "sky");
        assert!(browse("k", "../x", "trending").is_err());
    }
}
