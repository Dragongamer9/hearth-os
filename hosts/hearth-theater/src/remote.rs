// Jellyfin servers and sync with your other devices: the talking (jf.rs in the library has the rest, with tests).

use super::{agent, emit, lock, log, st};
use hearth_theater as th;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::Read;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

const MAX_JSON: u64 = 8 * 1024 * 1024;

fn device_name() -> String {
    std::fs::read_to_string("/etc/hostname").ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).unwrap_or_else(|| "Hearth".into())
}

/// This Theater's own id for Jellyfin (its devices list shows it), made once.
fn device_id() -> String {
    let f = st().dir.join("jellyfin").join("device-id");
    if let Ok(s) = std::fs::read_to_string(&f) { let s = s.trim(); if s.len() >= 8 && s.chars().all(|c| c.is_ascii_alphanumeric()) { return s.to_string(); } }
    let mut h: u64 = 0xcbf29ce484222325;
    for b in format!("{}-{}-{}", th::now_ms(), std::process::id(), device_name()).bytes() { h ^= b as u64; h = h.wrapping_mul(0x100000001b3); }
    let id = format!("hearth-theater-{h:016x}");
    let _ = std::fs::create_dir_all(f.parent().unwrap());
    let _ = std::fs::write(&f, &id);
    id
}

fn auth(token: &str) -> String { th::jf::auth_header(&device_name(), &device_id(), env!("CARGO_PKG_VERSION"), token) }

fn short_agent() -> ureq::Agent {
    ureq::AgentBuilder::new().timeout_connect(Duration::from_millis(1500)).timeout(Duration::from_secs(4)).build()
}

fn body(r: ureq::Response) -> Result<Value, String> {
    let mut b = Vec::new();
    r.into_reader().take(MAX_JSON).read_to_end(&mut b).map_err(|e| e.to_string())?;
    if b.is_empty() { return Ok(Value::Null); }
    serde_json::from_slice(&b).map_err(|_| "Jellyfin's answer didn't make sense".to_string())
}

fn jf_err(e: ureq::Error) -> String {
    match e {
        ureq::Error::Status(401, _) | ureq::Error::Status(403, _) => "Jellyfin said no (signed out? Connect it again)".into(),
        ureq::Error::Status(code, _) => format!("Jellyfin answered with an error ({code})"),
        e => format!("Couldn't reach Jellyfin ({})", e.kind()),
    }
}

fn get(base: &str, path: &str, token: &str) -> Result<Value, ureq::Error> {
    let a = auth(token);
    let r = agent().get(&format!("{base}{path}")).set("Authorization", &a).set("X-Emby-Authorization", &a).call()?;
    Ok(body(r).unwrap_or(Value::Null))
}

fn post(base: &str, path: &str, token: &str, data: Value) -> Result<Value, ureq::Error> {
    let a = auth(token);
    let r = agent().post(&format!("{base}{path}")).set("Authorization", &a).set("X-Emby-Authorization", &a).set("Content-Type", "application/json").send_string(&data.to_string())?;
    Ok(body(r).unwrap_or(Value::Null))
}

fn enc(s: &str) -> String { url::form_urlencoded::byte_serialize(s.as_bytes()).collect() }

// ------------------------------------------------------------------ finding Jellyfin

fn hearth_link_status() -> Option<Value> {
    let out = std::process::Command::new("hearth-link").arg("status").env("HEARTH_LINK_CALL_TIMEOUT", "5")
        .stdin(std::process::Stdio::null()).stderr(std::process::Stdio::null()).output().ok()?;
    let v: Value = serde_json::from_slice(&out.stdout).ok()?;
    (out.status.success() && v.get("error").is_none()).then_some(v)
}

fn tailscale_status() -> Value {
    std::process::Command::new("tailscale").args(["status", "--json"]).stdin(std::process::Stdio::null()).stderr(std::process::Stdio::null()).output().ok()
        .and_then(|o| serde_json::from_slice(&o.stdout).ok()).unwrap_or(Value::Null)
}

/// Jellyfin's answers on the home network to its discovery question (port 7359), for 1.2 seconds.
fn lan_spots() -> Vec<th::jf::Spot> {
    let mut out = Vec::new();
    let Ok(sock) = std::net::UdpSocket::bind("0.0.0.0:0") else { return out };
    let _ = sock.set_broadcast(true);
    let _ = sock.set_read_timeout(Some(Duration::from_millis(300)));
    if sock.send_to(b"Who is JellyfinServer?", "255.255.255.255:7359").is_err() { return out; }
    let until = std::time::Instant::now() + Duration::from_millis(1200);
    let mut buf = [0u8; 2048];
    while std::time::Instant::now() < until && out.len() < 8 {
        if let Ok((n, _)) = sock.recv_from(&mut buf) { if let Some(s) = th::jf::lan_spot(&buf[..n]) { out.push(s); } }
    }
    out
}

/// Jellyfin servers on your Tailscale (the home server's first), your home network and this device.
pub fn find() -> Value {
    let link = hearth_link_status();
    let home: Vec<String> = link.as_ref().map(|l| l["storage"]["servers"].as_array().into_iter().flatten().filter_map(|s| s["name"].as_str().map(String::from)).collect()).unwrap_or_default();
    let ts = tailscale_status();
    let mut spots = th::jf::tailscale_spots(&ts, &home);
    spots.extend(lan_spots());
    spots.push(th::jf::Spot { url: format!("http://127.0.0.1:{}/", th::jf::PORT), where_: "on this device".into(), rank: 4 });
    spots.truncate(40);
    let found: Mutex<Vec<(th::jf::Spot, Value)>> = Mutex::new(Vec::new());
    std::thread::scope(|s| {
        for spot in &spots {
            let found = &found;
            s.spawn(move || {
                let Ok(r) = short_agent().get(&format!("{}System/Info/Public", spot.url)).call() else { return };
                let Ok(v) = body(r) else { return };
                if v["Id"].as_str().map(|x| !x.is_empty()).unwrap_or(false) { lock(found).push((spot.clone(), v)); }
            });
        }
    });
    let mut found = found.into_inner().unwrap_or_default();
    found.sort_by_key(|(s, _)| s.rank);
    let mut seen = Vec::new();
    let mut servers = Vec::new();
    for (spot, info) in found {
        let id = info["Id"].as_str().unwrap_or("").to_string();
        if seen.contains(&id) { continue; }
        seen.push(id);
        // A Tailscale number is kept as its name when this device can look the name up (the same on your other devices).
        let mut url = spot.url.clone();
        if let Some(host) = url::Url::parse(&spot.url).ok().and_then(|u| u.host_str().map(String::from)) {
            if let Some(n) = th::jf::ts_name(&ts, &host) {
                use std::net::ToSocketAddrs;
                if (n.as_str(), th::jf::PORT).to_socket_addrs().map(|mut a| a.next().is_some()).unwrap_or(false) { url = format!("http://{n}:{}/", th::jf::PORT); }
            }
        }
        servers.push(json!({"name": info["ServerName"].as_str().unwrap_or("Jellyfin"), "url": url, "version": th::jf::short_version(info["Version"].as_str().unwrap_or("")), "where": spot.where_, "id": info["Id"]}));
    }
    log(&format!("jellyfin: found {} server(s)", servers.len()));
    json!({"servers": servers})
}

// ------------------------------------------------------------------ connecting (Quick Connect)

fn pending() -> &'static Mutex<HashMap<String, String>> { static P: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new(); P.get_or_init(Default::default) }

/// Asks Jellyfin for a Quick Connect code to approve there.
pub fn connect(url: &str) -> Result<Value, String> {
    let base = th::jf::base_url(url)?;
    let r = match post(&base, "QuickConnect/Initiate", "", json!({})) {
        Err(ureq::Error::Status(404 | 405, _)) => get(&base, "QuickConnect/Initiate", ""),
        r => r,
    };
    let v = r.map_err(|e| match e {
        ureq::Error::Status(401 | 403, _) => "Quick Connect is off in this Jellyfin: turn it on in its Dashboard › General › Quick Connect".to_string(),
        e => jf_err(e),
    })?;
    let (Some(secret), Some(code)) = (v["Secret"].as_str(), v["Code"].as_str()) else { return Err("This Jellyfin didn't give a Quick Connect code".into()) };
    lock(pending()).insert(base.clone(), secret.to_string());
    log(&format!("jellyfin: Quick Connect started on {}", th::log_url(&base)));
    Ok(json!({"code": code}))
}

pub fn cancel(url: &str) { if let Ok(b) = th::jf::base_url(url) { lock(pending()).remove(&b); } }

/// {} while the code isn't approved yet; {"site", "user"} once it is (the site added, or the one you had connected).
pub fn wait(url: &str, category: &str) -> Result<Value, String> {
    let base = th::jf::base_url(url)?;
    let secret = lock(pending()).get(&base).cloned().ok_or("That Quick Connect code ran out: start again")?;
    let v = get(&base, &format!("QuickConnect/Connect?secret={}", enc(&secret)), "").map_err(|e| match e {
        ureq::Error::Status(404, _) => { lock(pending()).remove(&base); "That Quick Connect code ran out: start again".to_string() }
        e => jf_err(e),
    })?;
    if v["Authenticated"] != true { return Ok(json!({})); }
    lock(pending()).remove(&base);
    let a = post(&base, "Users/AuthenticateWithQuickConnect", "", json!({"Secret": secret})).map_err(jf_err)?;
    let (Some(token), Some(uid)) = (a["AccessToken"].as_str(), a["User"]["Id"].as_str()) else { return Err("Jellyfin didn't sign Theater in".into()) };
    let user = a["User"]["Name"].as_str().unwrap_or("you").to_string();
    let server_id = a["ServerId"].as_str().map(String::from).or_else(|| get(&base, "System/Info/Public", "").ok().and_then(|i| i["Id"].as_str().map(String::from))).unwrap_or_default();
    let server_name = get(&base, "System/Info/Public", "").ok().and_then(|i| i["ServerName"].as_str().map(String::from)).unwrap_or_else(|| th::host_name(&base));
    let s = st();
    let site = {
        let mut sites = lock(&s.sites);
        let site = match sites.iter().position(|x| x.kind == th::jf::KIND && th::same_site(&x.url, &base)) {
            Some(i) => sites[i].clone(),
            None => {
                let mut x = th::add(&mut sites, &base, &format!("Jellyfin · {server_name}"), category)?;
                if let Some(y) = sites.iter_mut().find(|y| y.id == x.id) { y.kind = th::jf::KIND.into(); y.url = base.clone(); }
                x.kind = th::jf::KIND.into(); x.url = base.clone();
                x
            }
        };
        th::jf::save_login(&s.dir, &site.id, &th::jf::Login { url: base.clone(), token: token.into(), user_id: uid.into(), user: user.clone(), server_id })?;
        th::save(&s.dir, &sites)?;
        site
    };
    log(&format!("jellyfin: connected {} as a user (site {})", th::log_url(&base), site.id));
    Ok(json!({"site": site, "user": user}))
}

/// Who each connected Jellyfin is signed in as here (site id → user).
pub fn connected() -> Value {
    let s = st();
    let m: serde_json::Map<String, Value> = lock(&s.sites).iter().filter(|x| x.kind == th::jf::KIND)
        .filter_map(|x| th::jf::load_login(&s.dir, &x.id).map(|l| (x.id.clone(), json!(l.user)))).collect();
    Value::Object(m)
}

// ------------------------------------------------------------------ its rows and search

fn login_for(site: &th::Site) -> Result<th::jf::Login, String> {
    th::jf::load_login(&st().dir, &site.id).filter(|l| th::same_site(&l.url, &site.url))
        .ok_or_else(|| "Connect this Jellyfin on this device first: Add a site › Your Jellyfin › Connect".to_string())
}

const FIELDS: &str = "Fields=Overview,RecursiveItemCount&EnableTotalRecordCount=false";

/// Newer Jellyfin first (10.9+), then the older address of the same list.
fn list(base: &str, token: &str, new: &str, old: &str) -> Result<Value, String> {
    match get(base, new, token) {
        Err(ureq::Error::Status(404, _)) => get(base, old, token).map_err(jf_err),
        r => r.map_err(jf_err),
    }
}

pub fn catalog(site: &th::Site) -> th::Catalog {
    let l = match login_for(site) { Ok(l) => l, Err(e) => return th::jf::catalog(Vec::new(), e) };
    let (b, t, u) = (site.url.as_str(), l.token.as_str(), enc(&l.user_id));
    let resume = list(b, t, &format!("UserItems/Resume?userId={u}&Limit=24&MediaTypes=Video&{FIELDS}"), &format!("Users/{u}/Items/Resume?Limit=24&MediaTypes=Video&{FIELDS}"));
    if let Err(e) = &resume { if e.starts_with("Jellyfin said no") { return th::jf::catalog(Vec::new(), e.clone()); } }
    let next = list(b, t, &format!("Shows/NextUp?userId={u}&Limit=24&{FIELDS}"), &format!("Shows/NextUp?UserId={u}&Limit=24&{FIELDS}"));
    let latest = list(b, t, &format!("Items/Latest?userId={u}&Limit=24&{FIELDS}"), &format!("Users/{u}/Items/Latest?Limit=24&{FIELDS}"));
    let movies = list(b, t, &format!("Items?userId={u}&IncludeItemTypes=Movie&Recursive=true&SortBy=SortName&Limit=200&{FIELDS}"), &format!("Users/{u}/Items?IncludeItemTypes=Movie&Recursive=true&SortBy=SortName&Limit=200&{FIELDS}"));
    let shows = list(b, t, &format!("Items?userId={u}&IncludeItemTypes=Series&Recursive=true&SortBy=SortName&Limit=200&{FIELDS}"), &format!("Users/{u}/Items?IncludeItemTypes=Series&Recursive=true&SortBy=SortName&Limit=200&{FIELDS}"));
    let error = [&resume, &next, &latest, &movies, &shows].iter().find_map(|r| r.as_ref().err().cloned()).unwrap_or_default();
    let null = Value::Null;
    let v = |r: &Result<Value, String>| -> Value { r.as_ref().unwrap_or(&null).clone() };
    let (a, bb, c, d, e) = (v(&resume), v(&next), v(&latest), v(&movies), v(&shows));
    let rows = th::jf::rows(&[("Continue watching", &a), ("Next up", &bb), ("Recently added", &c), ("Movies", &d), ("Shows", &e)], b, &l.server_id);
    let error = if rows.is_empty() { error } else { String::new() };
    th::jf::catalog(rows, error)
}

pub fn search(site: &th::Site, q: &str) -> Result<Vec<th::Item>, String> {
    let l = login_for(site)?;
    let (b, u) = (site.url.as_str(), enc(&l.user_id));
    let v = list(b, &l.token, &format!("Items?userId={u}&searchTerm={}&IncludeItemTypes=Movie,Series,Episode&Recursive=true&Limit=40&{FIELDS}", enc(q)),
        &format!("Users/{u}/Items?searchTerm={}&IncludeItemTypes=Movie,Series,Episode&Recursive=true&Limit=40&{FIELDS}", enc(q)))?;
    Ok(th::jf::rows(&[("", &v)], b, &l.server_id).into_iter().flat_map(|r| r.items).collect())
}

// ------------------------------------------------------------------ sync (Hearth Link)

pub fn sync_status() -> Value { th::jf::sync_status(th::jf::sync_on(&st().dir), hearth_link_status().as_ref(), th::now_ms()) }

/// Turns sync on or off on this device (a file, so Hearth Link sees it too; nothing else of yours changes).
pub fn sync_set(on: bool) -> Result<Value, String> {
    let f = th::jf::sync_off_file(&st().dir);
    if on { match std::fs::remove_file(&f) { Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(e.to_string()), _ => {} } }
    else { std::fs::write(&f, "Theater's sites aren't synced from this device (Theater › Sync)\n").map_err(|e| e.to_string())?; }
    log(&format!("sync: turned {}", if on { "on" } else { "off" }));
    if on { apply_inbox(); }
    Ok(sync_status())
}

pub fn sync_now() -> Result<Value, String> {
    if !th::jf::sync_on(&st().dir) { return Err("Sync is off on this device".into()); }
    let out = std::process::Command::new("hearth-link").args(["call", "sync.now", "{}"]).env("HEARTH_LINK_CALL_TIMEOUT", "60")
        .stdin(std::process::Stdio::null()).stderr(std::process::Stdio::null()).output().map_err(|_| "Hearth Link isn't on this device".to_string())?;
    if !out.status.success() {
        let v: Value = serde_json::from_slice(&out.stdout).unwrap_or(Value::Null);
        return Err(format!("Couldn't sync: {}", v["error"].as_str().unwrap_or("Hearth Link isn't running")));
    }
    apply_inbox();
    Ok(sync_status())
}

/// What Hearth Link brought from your other devices, put in place (under the inbox's lock, which Hearth Link takes
/// too). Sites added elsewhere get their picture and titles read here.
pub fn apply_inbox() {
    let s = st();
    if !th::jf::sync_on(&s.dir) { return; }
    let file = th::jf::inbox_file(&s.dir);
    if !file.exists() { return; }
    let lockf = std::fs::OpenOptions::new().create(true).truncate(false).write(true).open(s.dir.join("link-inbox.lock"));
    if let Ok(l) = &lockf { let _ = l.lock(); }
    let inbox: Value = std::fs::read(&file).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null);
    let (added, removed, changed) = {
        let mut sites = lock(&s.sites);
        let mut titles = lock(&s.titles);
        let before = (sites.clone(), titles.clone());
        let (added, removed) = th::jf::apply_inbox(&mut sites, &mut titles, &inbox);
        for id in &removed { let _ = th::remove(&s.dir, &mut sites, id); th::jf::forget_login(&s.dir, id); titles.retain(|t| &t.site != id); }
        let changed = before != (sites.clone(), titles.clone());
        let saved = !changed || (th::save(&s.dir, &sites).is_ok() && th::save_titles(&s.dir, &titles).is_ok());
        if !saved { log("sync: couldn't save what came from your other devices; it stays in the inbox for next time"); if let Ok(l) = &lockf { let _ = l.unlock(); } return; }
        (added, removed, changed)
    };
    let _ = std::fs::remove_file(&file);
    if let Ok(l) = &lockf { let _ = l.unlock(); }
    if !changed { return; }
    log(&format!("sync: from your other devices: {} new site(s), {} removed", added.len(), removed.len()));
    let r = super::call("init", &Value::Null).unwrap_or(Value::Null);
    emit("synced", r);
    for id in added {
        std::thread::spawn(move || {
            let _ = super::call("refresh", &json!({"id": id}));
            let _ = super::read_catalog(&id);
        });
    }
}

/// Looks for what Hearth Link brought every 20 seconds.
pub fn watch_inbox() {
    std::thread::spawn(|| loop {
        apply_inbox();
        std::thread::sleep(Duration::from_secs(20));
    });
}
