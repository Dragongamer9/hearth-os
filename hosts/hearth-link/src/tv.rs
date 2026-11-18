//! Send to TV (docs/HEARTH-TV.md): a Hearth PC sends a link, a video or music file, or a show from your PC's
//! library to a Hearth TV paired with it, and the TV plays it in Hearth TV.
//!
//! - **On the PC** (`tv.list`, `tv.send {peer?, url? | path? | media?, title?}`): the TVs are your own paired devices
//!   that said they're a TV (`kind` in hello). A link goes as it is; a file stays on the PC and is shared with that
//!   one TV for 12 hours under a random token (`tv.read`, only from that TV, only that file); a show or movie of your
//!   PC's library goes as its id, and the TV opens it in its own My Media (the same server).
//! - **On the TV** (`tv.play` from the PC; `tv.inbox` from Hearth TV's app, every few seconds): what was sent waits
//!   here until Hearth TV takes it. A file's item gets a local address (127.0.0.1 only, Hearth Storage's media links,
//!   storage.rs) that the TV's player plays with seeking, each piece read from the PC over Hearth Link.
//! - Only your own devices: a friend's device can't send to your TV or read a shared file (family.rs).
//!
//! Between devices: tv.play {item}, tv.read {token, offset, len} (a stream like storage.read). From the UI:
//! tv.list, tv.send, tv.inbox.

use crate::node::Node;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

/// Hearth Storage's media links (storage.rs) read a sent file through tv.read when their share is this.
pub const SHARE: &str = "\u{1}tv";
/// What waits on the TV until Hearth TV takes it.
pub const MAX_INBOX: usize = 20;
/// How long a sent file can be played from the PC.
pub const SHARE_FOR_MS: u64 = 12 * 3600 * 1000;
pub const MAX_SHARES: usize = 64;
pub const MAX_URL: usize = 4000;

/// A file this PC shared with one TV.
#[derive(Debug, Clone)]
pub struct Shared { pub token: String, pub peer: String, pub path: PathBuf, pub size: u64, pub until: u64 }

fn inbox() -> &'static Mutex<Vec<Value>> { static I: OnceLock<Mutex<Vec<Value>>> = OnceLock::new(); I.get_or_init(|| Mutex::new(Vec::new())) }
fn shared() -> &'static Mutex<Vec<Shared>> { static S: OnceLock<Mutex<Vec<Shared>>> = OnceLock::new(); S.get_or_init(|| Mutex::new(Vec::new())) }

/// A web link that can be sent: http or https, no spaces or control characters, not too long.
pub fn valid_url(u: &str) -> bool {
    (u.starts_with("https://") || u.starts_with("http://")) && u.len() > 8 && u.len() <= MAX_URL
        && !u.chars().any(|c| c.is_whitespace() || c.is_control())
}

/// Text from the other device, for showing: no control characters, at most `max` characters.
pub fn clean(s: &str, max: usize) -> String { s.chars().filter(|c| !c.is_control()).take(max).collect::<String>().trim().to_string() }

pub fn valid_token(t: &str) -> bool { t.len() == 32 && t.bytes().all(|b| b.is_ascii_hexdigit()) }

/// A library item's id (Jellyfin's are 32 hex characters, Plex's are numbers): letters, digits and dashes.
pub fn valid_media_id(t: &str) -> bool { !t.is_empty() && t.len() <= 64 && t.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') }

/// A title for a link when none was given: the site's name ("youtube.com").
pub fn site_of(url: &str) -> String {
    let rest = url.split("://").nth(1).unwrap_or(url);
    let host = rest.split(['/', '?', '#']).next().unwrap_or("").rsplit('@').next().unwrap_or("");
    let host = host.split(':').next().unwrap_or("");
    host.strip_prefix("www.").unwrap_or(host).to_string()
}

/// What a PC sends (tv.play's item), checked on the TV: {kind: link|file|media, title, url | file | media}.
pub fn check_item(v: &Value) -> Result<Value, String> {
    let kind = v["kind"].as_str().unwrap_or("");
    let title = clean(v["title"].as_str().unwrap_or(""), 200);
    match kind {
        "link" => {
            let url = v["url"].as_str().unwrap_or("");
            if !valid_url(url) { return Err("That isn't a web link".into()); }
            Ok(json!({"kind": "link", "url": url, "title": if title.is_empty() { site_of(url) } else { title }}))
        }
        "file" => {
            let f = &v["file"];
            let token = f["token"].as_str().unwrap_or("");
            let name = clean(f["name"].as_str().unwrap_or(""), 200);
            if !valid_token(token) || !crate::sync::valid_part(&name) { return Err("That isn't a file the TV can play".into()); }
            if !crate::storage::is_media(&name) { return Err("Only videos and music play on the TV".into()); }
            let size = f["size"].as_u64().unwrap_or(0);
            Ok(json!({"kind": "file", "title": if title.is_empty() { name.clone() } else { title }, "file": {"token": token, "name": name, "size": size, "mime": crate::storage::mime(&name)}}))
        }
        "media" => {
            let m = &v["media"];
            let id = m["id"].as_str().unwrap_or("");
            if !valid_media_id(id) { return Err("That isn't something from your library".into()); }
            let what = match m["type"].as_str().unwrap_or("") { t @ ("show" | "movie" | "episode") => t, _ => "movie" };
            Ok(json!({"kind": "media", "title": title, "media": {"id": id, "type": what}}))
        }
        _ => Err("The TV doesn't know what to do with that".into()),
    }
}

/// A file on this PC that can be sent: a regular file (links followed), a video or music.
pub fn check_file(path: &Path) -> Result<(PathBuf, String, u64), String> {
    let full = std::fs::canonicalize(path).map_err(|_| "That file isn't there".to_string())?;
    let md = std::fs::metadata(&full).map_err(|_| "That file can't be read".to_string())?;
    if !md.is_file() { return Err("Only a file can be sent to the TV".into()); }
    let name = full.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    if !crate::storage::is_media(&name) { return Err("Only videos and music play on the TV".into()); }
    std::fs::File::open(&full).map_err(|_| "That file can't be read".to_string())?;
    Ok((full, name, md.len()))
}

/// The file shared under `token` with `peer`, while it still is.
pub fn shared_file(token: &str, peer: &str) -> Option<Shared> {
    let now = crate::now_ms();
    let mut l = crate::lock(shared());
    l.retain(|s| s.until > now);
    l.iter().find(|s| s.token == token && s.peer == peer).cloned()
}

fn share_file(peer: &str, path: PathBuf, size: u64) -> String {
    let token = crate::hex(&crate::random_bytes::<16>());
    let now = crate::now_ms();
    let mut l = crate::lock(shared());
    l.retain(|s| s.until > now);
    // The same file sent to the same TV again: one token for it.
    l.retain(|s| !(s.peer == peer && s.path == path));
    l.push(Shared { token: token.clone(), peer: peer.to_string(), path, size, until: now + SHARE_FOR_MS });
    while l.len() > MAX_SHARES { l.remove(0); }
    token
}

/// Puts something sent here in the inbox (oldest dropped past MAX_INBOX).
pub fn push_inbox(item: Value) {
    let mut l = crate::lock(inbox());
    l.push(item);
    while l.len() > MAX_INBOX { l.remove(0); }
}

pub fn take_inbox() -> Vec<Value> { std::mem::take(&mut *crate::lock(inbox())) }

impl Node {
    /// This device's TVs: your own paired devices that said they're a TV (or announce themselves as one here).
    pub fn tv_list(&self) -> Vec<Value> {
        let now = crate::now_ms();
        let found = crate::lock(&self.found).clone();
        let peers = crate::lock(&self.peers);
        let out: Vec<Value> = peers.mine().filter(|p| p.kind == "tv" || found.get(&p.id).map(|f| f.kind == "tv").unwrap_or(false)).map(|p| {
            let lan = found.get(&p.id).map(|f| now.saturating_sub(f.at) < 60_000).unwrap_or(false);
            json!({"id": p.id, "name": p.name, "online": lan || now.saturating_sub(p.last_seen) < 60_000, "lan": lan})
        }).collect();
        out
    }

    /// Requests from another device: tv.play (on a TV).
    pub async fn tv_answer(self: Arc<Self>, peer: &str, op: &str, req: &Value) -> Result<Value, String> {
        match op {
            "tv.play" => {
                if self.kind != "tv" { return Err(format!("{} isn't a Hearth TV", self.name())); }
                let mut item = check_item(&req["item"])?;
                let from = crate::lock(&self.peers).get(peer).map(|p| p.name.clone()).unwrap_or_else(|| "your PC".into());
                item["id"] = json!(crate::hex(&crate::random_bytes::<6>()));
                item["peer"] = json!(peer);
                item["from"] = json!(from);
                item["at"] = json!(crate::now_ms());
                eprintln!("hearth-link: tv: {} sent from {from}", item["kind"].as_str().unwrap_or(""));
                push_inbox(item);
                Ok(json!({"ok": true, "name": self.name()}))
            }
            _ => Err(format!("Unknown request: {op}")),
        }
    }

    /// The bytes of a file this PC sent to `peer` (tv.read), as storage.read's chunk frames.
    pub async fn tv_serve_read(self: &Arc<Self>, peer: &str, req: &Value, s: &mut iroh::endpoint::SendStream) -> Result<(), String> {
        let sh = shared_file(req["token"].as_str().unwrap_or(""), peer).ok_or("That file isn't shared with this TV anymore (send it again)")?;
        let off = req["offset"].as_u64().unwrap_or(0);
        let len = req["len"].as_u64().unwrap_or(crate::wire::CHUNK).min(crate::storage::READ_MAX);
        let (tx, mut rx) = tokio::sync::mpsc::channel::<bytes::Bytes>(4);
        let path = sh.path.clone();
        tokio::task::spawn_blocking(move || crate::storage::read_range(path, off, len, tx));
        while let Some(b) = rx.recv().await { s.write_chunk(b).await.map_err(|e| e.to_string())?; }
        Ok(())
    }

    /// The UI's tv.* commands.
    pub async fn tv_command(self: &Arc<Self>, cmd: &str, a: &Value) -> Result<Value, String> {
        match cmd {
            "tv.list" => Ok(json!({"tvs": self.tv_list(), "me": {"kind": self.kind}})),
            "tv.send" => {
                let tvs = self.tv_list();
                let peer = match a["peer"].as_str().filter(|p| !p.is_empty()) {
                    Some(p) => {
                        if !crate::lock(&self.peers).is_mine(p) { return Err("That isn't one of your paired devices".into()); }
                        p.to_string()
                    }
                    None => match tvs.len() {
                        0 => return Err("No Hearth TV is paired with this PC. On the TV: Settings › Hearth Link › Pair a device".into()),
                        1 => tvs[0]["id"].as_str().unwrap_or("").to_string(),
                        _ => return Err(format!("Pick a TV: {}", tvs.iter().filter_map(|t| t["name"].as_str()).collect::<Vec<_>>().join(", "))),
                    },
                };
                let name = crate::lock(&self.peers).get(&peer).map(|p| p.name.clone()).unwrap_or_else(|| "the TV".into());
                let title = clean(a["title"].as_str().unwrap_or(""), 200);
                let item = if let Some(p) = a["path"].as_str().filter(|p| !p.is_empty()) {
                    let (full, fname, size) = check_file(Path::new(p))?;
                    let token = share_file(&peer, full, size);
                    json!({"kind": "file", "title": if title.is_empty() { fname.clone() } else { title }, "file": {"token": token, "name": fname, "size": size}})
                } else if a["media"].is_object() {
                    json!({"kind": "media", "title": title, "media": a["media"]})
                } else {
                    let url = a["url"].as_str().unwrap_or("").trim();
                    if !valid_url(url) { return Err("That isn't a web link (it has to start with https://)".into()); }
                    json!({"kind": "link", "url": url, "title": title})
                };
                // Checked here too, so a mistake is said on the PC rather than by the TV.
                check_item(&item)?;
                match tokio::time::timeout(Duration::from_secs(30), self.request(&peer, &json!({"op": "tv.play", "item": item}))).await {
                    Ok(Ok(_)) => {
                        eprintln!("hearth-link: tv: sent a {} to {name}", item["kind"].as_str().unwrap_or(""));
                        Ok(json!({"sent": true, "tv": name, "peer": peer, "kind": item["kind"]}))
                    }
                    Ok(Err(e)) => Err(format!("Couldn't send it to {name}: {}", e.trim_start_matches("connection: "))),
                    Err(_) => Err(format!("{name} didn't answer (is it on?)")),
                }
            }
            "tv.inbox" => {
                let mut items = take_inbox();
                for it in items.iter_mut() {
                    if it["kind"] != "file" { continue; }
                    // A file plays from this TV's own address, read from the PC piece by piece (with seeking).
                    let port = self.storage_http().await?;
                    let local = crate::hex(&crate::random_bytes::<16>());
                    let name = it["file"]["name"].as_str().unwrap_or("video").to_string();
                    let st = crate::storage::Stream {
                        server: it["peer"].as_str().unwrap_or("").to_string(), share: SHARE.into(),
                        path: it["file"]["token"].as_str().unwrap_or("").to_string(), size: it["file"]["size"].as_u64().unwrap_or(0),
                        name: name.clone(), until: crate::now_ms() + SHARE_FOR_MS,
                    };
                    {
                        let mut m = crate::lock(&self.storage.streams);
                        m.retain(|_, v| v.until > crate::now_ms());
                        m.insert(local.clone(), st);
                    }
                    let safe: String = name.chars().map(|c| if c.is_ascii_alphanumeric() || ".-_".contains(c) { c } else { '_' }).collect();
                    it["url"] = json!(format!("http://127.0.0.1:{port}/{local}/{safe}"));
                }
                Ok(json!({"items": items}))
            }
            _ => Err(format!("Unknown command: {cmd}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links_are_checked() {
        assert!(valid_url("https://www.youtube.com/watch?v=aqz-KE-bpKQ"));
        assert!(valid_url("http://192.168.1.20:8096/web/"));
        assert!(!valid_url("file:///etc/passwd"));
        assert!(!valid_url("javascript:alert(1)"));
        assert!(!valid_url("https://a b"));
        assert!(!valid_url("https://"));
        assert!(!valid_url(&format!("https://x/{}", "a".repeat(5000))));
        assert_eq!(site_of("https://www.youtube.com/watch?v=1"), "youtube.com");
        assert_eq!(site_of("https://user@vimeo.com:443/1"), "vimeo.com");
    }

    #[test]
    fn items_from_a_pc_are_checked() {
        let l = check_item(&json!({"kind": "link", "url": "https://vimeo.com/1", "title": "Sintel\u{7}, trailer"})).unwrap();
        assert_eq!(l["title"], "Sintel, trailer");
        assert_eq!(check_item(&json!({"kind": "link", "url": "https://twitch.tv/x"})).unwrap()["title"], "twitch.tv");
        assert!(check_item(&json!({"kind": "link", "url": "intent://x"})).is_err());
        let tok = "0123456789abcdef0123456789abcdef";
        let f = check_item(&json!({"kind": "file", "file": {"token": tok, "name": "Holiday 2026.mkv", "size": 12}})).unwrap();
        assert_eq!(f["file"]["mime"], "video/x-matroska");
        assert_eq!(f["title"], "Holiday 2026.mkv");
        assert!(check_item(&json!({"kind": "file", "file": {"token": tok, "name": "../x.mp4"}})).is_err());
        assert!(check_item(&json!({"kind": "file", "file": {"token": tok, "name": "notes.txt"}})).is_err());
        assert!(check_item(&json!({"kind": "file", "file": {"token": "zz", "name": "a.mp4"}})).is_err());
        let m = check_item(&json!({"kind": "media", "title": "Severance", "media": {"id": "a1b2c3", "type": "show"}})).unwrap();
        assert_eq!(m["media"]["type"], "show");
        assert!(check_item(&json!({"kind": "media", "media": {"id": "../../x"}})).is_err());
        assert!(check_item(&json!({"kind": "app", "url": "https://x.y"})).is_err());
    }

    #[test]
    fn files_are_checked_and_shared_with_one_tv() {
        let d = tempfile::tempdir().unwrap();
        let v = d.path().join("film.mp4");
        std::fs::write(&v, b"0123456789").unwrap();
        std::fs::write(d.path().join("notes.txt"), b"x").unwrap();
        let (full, name, size) = check_file(&v).unwrap();
        assert_eq!((name.as_str(), size), ("film.mp4", 10));
        assert!(check_file(&d.path().join("notes.txt")).is_err());
        assert!(check_file(d.path()).is_err());
        assert!(check_file(&d.path().join("gone.mp4")).is_err());
        let t = share_file("tv1", full.clone(), size);
        assert!(valid_token(&t));
        assert_eq!(shared_file(&t, "tv1").unwrap().path, full);
        assert!(shared_file(&t, "someone-else").is_none(), "only the TV it was sent to");
        // Sent again: the old token is replaced.
        let t2 = share_file("tv1", full.clone(), size);
        assert!(shared_file(&t, "tv1").is_none());
        assert!(shared_file(&t2, "tv1").is_some());
    }

    #[test]
    fn the_inbox_keeps_the_newest() {
        take_inbox();
        for i in 0..(MAX_INBOX + 5) { push_inbox(json!({"n": i})); }
        let l = take_inbox();
        assert_eq!(l.len(), MAX_INBOX);
        assert_eq!(l[0]["n"], 5);
        assert!(take_inbox().is_empty(), "taken once");
    }
}
