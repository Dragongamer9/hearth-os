//! Play together (Beta Experimental): Remote Play Together, Hearth's way. The host streams the game it's playing with
//! Sunshine (Settings › Remote play's own Sunshine) and a friend's Hearth plays it through Moonlight, their controller
//! becoming another player on the host's game. Since 1.6 the invitation travels through Discord (discord.rs: a Rich
//! Presence join secret with the host's address and a one-time 4-digit PIN, docs/DISCORD.md); before, it travelled in
//! a Hearth Friends (Matrix) chat (archived: archive/hosts/hearth-core/src/matrix_friends.rs).
//! How the friend reaches the host is a Transport: the local network, Tailscale (a shared node), or Hearth Link once it
//! answers `hearth-link call play.address`.

use serde_json::{json, Value};
use std::path::Path;
use std::time::{Duration, Instant};

pub trait Transport: Send + Sync {
    fn id(&self) -> &'static str;
    fn ready(&self) -> bool;
    /// Addresses a friend can reach this device's stream at.
    fn host_addresses(&self) -> Vec<String>;
}

/// The same network: this device's own addresses.
pub struct Lan;
impl Transport for Lan {
    fn id(&self) -> &'static str { "lan" }
    fn ready(&self) -> bool { true }
    fn host_addresses(&self) -> Vec<String> { crate::remote::addresses() }
}

/// Over the internet through Hearth Link (hosts/hearth-link: iroh QUIC, peer-to-peer with relays): asked with
/// `hearth-link call play.address {}`, which is to answer {"ready": bool, "addresses": [...]} — addresses a friend's
/// Moonlight can reach this device's Sunshine at through Link's tunnel. Hearth Link doesn't have that call yet (it
/// links your own paired devices; a tunnel for a friend's device is still to come), so until it does this
/// transport says it isn't ready and Play together uses the local network.
pub struct Link;
impl Link {
    fn program() -> Option<&'static str> {
        ["/usr/bin/hearth-link", "/usr/libexec/nimbus/hearth-link"].into_iter().find(|p| Path::new(p).is_file())
    }
    fn ask() -> Option<Value> {
        let p = Self::program()?;
        let out = crate::system::run_timeout(p, &["call", "play.address", "{}"], Duration::from_secs(8)).ok()?;
        let v: Value = serde_json::from_str(out.trim()).ok()?;
        if v.get("error").is_some() { None } else { Some(v) }
    }
}

impl Transport for Link {
    fn id(&self) -> &'static str { "link" }
    fn ready(&self) -> bool { Self::ask().map(|v| v["ready"] == true).unwrap_or(false) }
    fn host_addresses(&self) -> Vec<String> {
        Self::ask().and_then(|v| v["addresses"].as_array().cloned()).unwrap_or_default().iter().filter_map(|a| a.as_str().map(str::to_string)).take(4).collect()
    }
}

pub fn transports() -> Vec<Box<dyn Transport>> { vec![Box::new(Lan), Box::new(Link)] }

/// A Tailscale address (100.64.0.0/10): a friend you shared this PC with in Tailscale (Share node) reaches it there.
pub fn is_tailscale(a: &str) -> bool {
    let p: Vec<u8> = a.split('.').filter_map(|x| x.parse().ok()).collect();
    p.len() == 4 && p[0] == 100 && (64..128).contains(&p[1])
}

/// The address a Discord join secret carries: Tailscale's first, else the first of the local network's. Only while
/// Sunshine is on (Hearth never turns it on by itself for this) and can host.
pub fn pick_address(addrs: &[String]) -> Option<String> {
    addrs.iter().find(|a| is_tailscale(a)).or_else(|| addrs.first()).cloned()
}
pub fn host_address() -> Option<String> {
    if !sunshine_installed() || crate::remote::status()["on"] != true { return None; }
    let mut all: Vec<String> = transports().iter().filter(|t| t.ready()).flat_map(|t| t.host_addresses()).collect();
    all.dedup();
    pick_address(&all)
}

fn sunshine_installed() -> bool { crate::system::has("sunshine") || Path::new("/usr/bin/sunshine").exists() }

/// Friends see "Joinable" while a game runs on a device that can host (Sunshine is there), unless the player
/// turned "Let friends ask to join" off.
pub fn joinable() -> bool {
    sunshine_installed() && crate::store::Store::open(crate::paths::data_dir().join("friends-play.json")).data["askToJoin"] != false
}

pub fn set_ask_to_join(on: bool) -> Result<Value, String> {
    let mut s = crate::store::Store::open(crate::paths::data_dir().join("friends-play.json"));
    s.update(|d| { d.insert("askToJoin".into(), json!(on)); }).map_err(|e| e.to_string())?;
    Ok(host_info())
}

pub fn host_info() -> Value {
    let ts = transports();
    let ready: Vec<&Box<dyn Transport>> = ts.iter().filter(|t| t.ready()).collect();
    let addrs: Vec<String> = ready.iter().flat_map(|t| t.host_addresses()).collect();
    json!({"canHost": sunshine_installed(), "canJoin": crate::remote::moonlight_installed(), "addresses": addrs,
           "transports": ready.iter().map(|t| t.id()).collect::<Vec<_>>(), "port": 47989,
           "askToJoin": crate::store::Store::open(crate::paths::data_dir().join("friends-play.json")).data["askToJoin"] != false})
}

fn emit(ev: &str, data: Value) { crate::stores::notify(json!({"type": "event", "name": "discord", "data": {"event": ev, "data": data}})); }

/// The name a friend's device is paired under in Sunshine, so it can be found and removed later.
pub fn paired_name(friend: &str, device: &str) -> String {
    let f: String = friend.chars().filter(|c| c.is_alphanumeric() || " ._-@:".contains(*c)).take(24).collect();
    let d: String = device.chars().filter(|c| c.is_alphanumeric() || " ._-".contains(*c)).take(12).collect();
    format!("Friend {f} {d}").trim().to_string()
}

/// Host: the friend's request was accepted. Sunshine on, the friend's input set, then the PIN entered as soon as
/// their Moonlight starts pairing (Sunshine only takes a PIN while one is pending): tried for 90 seconds.
pub fn host_accept(session: &str, pin: &str, friend: &str, device: &str, controller_only: bool) -> Result<Value, String> {
    if pin.len() != 4 || !pin.chars().all(|c| c.is_ascii_digit()) { return Err("That join request has no valid PIN.".into()); }
    if !sunshine_installed() { return Err("Play together needs Sunshine (Settings › Remote play), which comes with Hearth OS.".into()); }
    let (session, pin, name) = (session.to_string(), pin.to_string(), paired_name(friend, device));
    std::thread::spawn(move || {
        let st = crate::remote::status();
        if st["on"] != true { if let Err(e) = crate::remote::set_on(true) { emit("ptFailed", json!({"session": session, "why": e})); return; } }
        let _ = set_input(controller_only);
        let until = Instant::now() + Duration::from_secs(90);
        while Instant::now() < until {
            if crate::remote::pair(&pin, &name).is_ok() { emit("ptPaired", json!({"session": session, "name": name})); return; }
            std::thread::sleep(Duration::from_secs(2));
        }
        emit("ptFailed", json!({"session": session, "why": "Your friend's device didn't connect in time."}));
    });
    Ok(json!({"waiting": true}))
}

/// The friend's keyboard and mouse off (controller only), or back on: Sunshine's own settings, merged with what
/// it has (its API replaces the whole file), then Sunshine is asked to restart for it to take effect.
pub fn set_input(controller_only: bool) -> Result<(), String> {
    let want = if controller_only { "disabled" } else { "enabled" };
    let cur = crate::remote::api("GET", "/api/config", None, creds())?;
    let mut cfg = serde_json::Map::new();
    if let Some(o) = cur.as_object() {
        for (k, v) in o {
            // What GET adds that isn't a setting.
            if ["status", "platform", "version", "restart_supported", "vdisplayStatus"].contains(&k.as_str()) { continue; }
            if let Some(s) = v.as_str() { cfg.insert(k.clone(), json!(s)); }
        }
    }
    if cfg.get("keyboard").and_then(|v| v.as_str()) == Some(want) && cfg.get("mouse").and_then(|v| v.as_str()) == Some(want) { return Ok(()); }
    cfg.insert("keyboard".into(), json!(want));
    cfg.insert("mouse".into(), json!(want));
    crate::remote::api("POST", "/api/config", Some(&Value::Object(cfg)), creds())?;
    let _ = crate::remote::api("POST", "/api/restart", None, creds());
    Ok(())
}

fn creds() -> Option<(String, String)> {
    let v: Value = std::fs::read(crate::paths::data_dir().join("remote-play.json")).ok().and_then(|b| serde_json::from_slice(&b).ok())?;
    Some((v["user"].as_str()?.to_string(), v["pass"].as_str()?.to_string()))
}

/// Host: remove a friend from the game (or end Play together): their device unpaired and the stream closed.
pub fn host_end(name: Option<&str>) -> Result<Value, String> {
    let st = crate::remote::status();
    let mut removed = 0;
    for c in st["clients"].as_array().cloned().unwrap_or_default() {
        let n = c["name"].as_str().unwrap_or("");
        let hit = match name { Some(want) => n == want, None => n.starts_with("Friend ") };
        if hit { if let Some(u) = c["uuid"].as_str() { if crate::remote::unpair(u).is_ok() { removed += 1; } } }
    }
    let _ = crate::remote::api("POST", "/api/apps/close", None, creds());
    Ok(json!({"removed": removed}))
}

fn moonlight(args: &[&str]) -> std::process::Command {
    let mut c = std::process::Command::new("flatpak");
    c.args(["run", "com.moonlight_stream.Moonlight"]).args(args);
    c
}

/// Friend: pair Moonlight with the host using our PIN (the host's Hearth enters it), then stream its game.
pub fn guest_join(session: &str, address: &str, pin: &str) -> Result<Value, String> {
    if !crate::remote::moonlight_installed() { return Err("Joining needs Moonlight. Install it from Settings › Remote play.".into()); }
    if pin.len() != 4 || !pin.chars().all(|c| c.is_ascii_digit()) { return Err("No valid PIN for this join.".into()); }
    let ok_addr = !address.is_empty() && address.len() <= 100 && address.chars().all(|c| c.is_ascii_alphanumeric() || ".:-[]".contains(c));
    if !ok_addr { return Err("That host address isn't right.".into()); }
    let (session, address, pin) = (session.to_string(), address.to_string(), pin.to_string());
    std::thread::spawn(move || {
        let out = moonlight(&["pair", &address, "--pin", &pin]).stdin(std::process::Stdio::null()).output();
        let paired = match &out { Ok(o) => o.status.success() || String::from_utf8_lossy(&o.stdout).to_lowercase().contains("already paired"), Err(_) => false };
        if !paired { emit("ptFailed", json!({"session": session, "why": "Couldn't pair with your friend's device. Are you on the same network?"})); return; }
        match moonlight(&["stream", &address, "Desktop"]).stdin(std::process::Stdio::null()).spawn() {
            Ok(c) => { crate::procs::detach(c); emit("ptStreaming", json!({"session": session})); }
            Err(e) => emit("ptFailed", json!({"session": session, "why": format!("Moonlight didn't start ({e})")})),
        }
    });
    Ok(json!({"joining": true}))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn picks_tailscale_first() {
        assert!(is_tailscale("100.101.1.2") && !is_tailscale("100.1.1.2") && !is_tailscale("192.168.1.2"));
        assert_eq!(pick_address(&["192.168.1.2".into(), "100.100.3.4".into()]).as_deref(), Some("100.100.3.4"));
        assert_eq!(pick_address(&["192.168.1.2".into()]).as_deref(), Some("192.168.1.2"));
        assert_eq!(pick_address(&[]), None);
    }
    #[test]
    fn paired_names_are_tidy() {
        assert_eq!(paired_name("sam", "Ally X"), "Friend sam Ally X");
        assert_eq!(paired_name("@a;rm -rf:x", "d\"ev"), "Friend @arm -rf:x dev");
        assert!(paired_name(&"x".repeat(100), "y").len() < 45);
    }
}
