//! This device's key (its identity: other devices pin it at pairing), its name and Hearth Link's settings, and the
//! devices it's paired with. All in Hearth Link's folder (lib.rs home_dir): identity.key (0600), config.json,
//! peers.json.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// The device's secret key: made once, kept readable only by this user.
pub fn load_or_make_key(dir: &Path) -> Result<iroh::SecretKey, String> {
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
    let f = dir.join("identity.key");
    if let Ok(b) = std::fs::read(&f) {
        if b.len() == 32 {
            let mut k = [0u8; 32];
            k.copy_from_slice(&b);
            return Ok(iroh::SecretKey::from_bytes(&k));
        }
    }
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let _ = std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700));
    let key = iroh::SecretKey::from_bytes(&crate::random_bytes::<32>());
    let tmp = dir.join(".identity.key.tmp");
    let _ = std::fs::remove_file(&tmp);
    {
        use std::io::Write;
        let mut o = std::fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(&tmp).map_err(|e| e.to_string())?;
        o.write_all(&key.to_bytes()).map_err(|e| e.to_string())?;
        o.sync_all().map_err(|e| e.to_string())?;
    }
    std::fs::rename(&tmp, &f).map_err(|e| e.to_string())?;
    Ok(key)
}

/// The relay setting: n0's public relays (default), your own, or none (same network only).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Relay { Default, Custom, Off }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Config {
    /// The name other devices see ("Living room PC"); the computer's name when empty.
    pub name: String,
    pub relay: Relay,
    /// Your own relay (https://relay.example.com), for Relay::Custom.
    pub relay_url: String,
    /// Copies sent from any paired device start without asking.
    pub always_accept: bool,
    /// Keep Hearth data in sync with paired devices.
    pub sync: bool,
    /// Compression: "auto" (per chunk, only where it helps), "off".
    pub compress: String,
    /// Parallel streams for a copy.
    pub streams: usize,
}

impl Default for Config {
    fn default() -> Self {
        Config { name: String::new(), relay: Relay::Default, relay_url: String::new(), always_accept: false, sync: true, compress: "auto".into(), streams: 8 }
    }
}

impl Config {
    pub fn load(dir: &Path) -> Config {
        std::fs::read(dir.join("config.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
    }
    pub fn save(&self, dir: &Path) -> Result<(), String> {
        crate::write_atomic(&dir.join("config.json"), serde_json::to_string_pretty(self).unwrap().as_bytes())
    }
    pub fn display_name(&self) -> String {
        let n = self.name.trim();
        if !n.is_empty() { return n.chars().take(40).collect(); }
        hostname()
    }
    /// Changes from the UI, checked.
    pub fn apply(&mut self, a: &Value) -> Result<(), String> {
        if let Some(n) = a.get("name").and_then(Value::as_str) {
            let n: String = n.trim().chars().filter(|c| !c.is_control()).take(40).collect();
            self.name = n;
        }
        if let Some(r) = a.get("relay").and_then(Value::as_str) {
            self.relay = match r { "default" => Relay::Default, "custom" => Relay::Custom, "off" => Relay::Off, _ => return Err("Unknown relay choice".into()) };
        }
        if let Some(u) = a.get("relayUrl").and_then(Value::as_str) {
            let u = u.trim();
            if !u.is_empty() && !valid_relay_url(u) { return Err("A relay's address starts with https:// (or http:// on your own network)".into()); }
            self.relay_url = u.to_string();
        }
        if self.relay == Relay::Custom && self.relay_url.is_empty() { return Err("Enter your relay's address first".into()); }
        if let Some(b) = a.get("alwaysAccept").and_then(Value::as_bool) { self.always_accept = b; }
        if let Some(b) = a.get("sync").and_then(Value::as_bool) { self.sync = b; }
        if let Some(c) = a.get("compress").and_then(Value::as_str) {
            if c != "auto" && c != "off" { return Err("Compression is auto or off".into()); }
            self.compress = c.into();
        }
        if let Some(s) = a.get("streams").and_then(Value::as_u64) { self.streams = (s as usize).clamp(1, 32); }
        Ok(())
    }
    pub fn to_json(&self) -> Value {
        json!({"name": self.display_name(), "customName": self.name, "relay": self.relay, "relayUrl": self.relay_url, "alwaysAccept": self.always_accept,
               "sync": self.sync, "compress": self.compress, "streams": self.streams})
    }
}

pub fn valid_relay_url(u: &str) -> bool {
    (u.starts_with("https://") || u.starts_with("http://")) && u.len() < 300 && !u.chars().any(|c| c.is_whitespace() || c.is_control())
        && u.parse::<iroh::RelayUrl>().is_ok()
}

pub fn hostname() -> String {
    let mut buf = [0u8; 256];
    let r = unsafe { libc::gethostname(buf.as_mut_ptr() as *mut libc::c_char, buf.len()) };
    let n = if r == 0 { String::from_utf8_lossy(&buf[..buf.iter().position(|b| *b == 0).unwrap_or(buf.len())]).to_string() } else { String::new() };
    let n = n.trim().to_string();
    if n.is_empty() || n == "localhost" { "Hearth".into() } else { n.chars().take(40).collect() }
}

/// A paired device.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct Peer {
    /// Its key (hex): the only thing that can be this device.
    pub id: String,
    pub name: String,
    pub paired_at: u64,
    pub last_seen: u64,
    /// Where it was last reached directly ("192.168.1.20:47740").
    pub addrs: Vec<String>,
    /// Copies it sends start without asking.
    pub always_accept: bool,
    /// What it said about itself the last time: stores it's signed in to.
    pub signed_in: Value,
}

pub struct Peers { pub file: PathBuf, pub list: Vec<Peer> }

impl Peers {
    pub fn load(dir: &Path) -> Peers {
        let file = dir.join("peers.json");
        let list = std::fs::read(&file).ok().and_then(|b| serde_json::from_slice::<Vec<Peer>>(&b).ok()).unwrap_or_default();
        Peers { file, list }
    }
    pub fn save(&self) -> Result<(), String> { crate::write_atomic(&self.file, serde_json::to_string_pretty(&self.list).unwrap().as_bytes()) }
    pub fn get(&self, id: &str) -> Option<&Peer> { self.list.iter().find(|p| p.id == id) }
    pub fn get_mut(&mut self, id: &str) -> Option<&mut Peer> { self.list.iter_mut().find(|p| p.id == id) }
    pub fn is_paired(&self, id: &str) -> bool { self.get(id).is_some() }
    pub fn add(&mut self, id: &str, name: &str) {
        let name: String = name.chars().filter(|c| !c.is_control()).take(40).collect();
        if let Some(p) = self.get_mut(id) { p.name = name; p.last_seen = crate::now_ms(); }
        else { self.list.push(Peer { id: id.to_string(), name, paired_at: crate::now_ms(), last_seen: crate::now_ms(), ..Default::default() }); }
        let _ = self.save();
    }
    pub fn remove(&mut self, id: &str) -> bool {
        let n = self.list.len();
        self.list.retain(|p| p.id != id);
        let changed = self.list.len() != n;
        if changed { let _ = self.save(); }
        changed
    }
    pub fn seen(&mut self, id: &str, addr: Option<std::net::SocketAddr>) {
        if let Some(p) = self.get_mut(id) {
            p.last_seen = crate::now_ms();
            if let Some(a) = addr {
                let s = a.to_string();
                p.addrs.retain(|x| x != &s);
                p.addrs.insert(0, s);
                p.addrs.truncate(4);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn the_key_is_made_once_and_kept_private() {
        use std::os::unix::fs::PermissionsExt;
        let d = tempfile::tempdir().unwrap();
        let a = load_or_make_key(d.path()).unwrap();
        let b = load_or_make_key(d.path()).unwrap();
        assert_eq!(a.public(), b.public());
        let mode = std::fs::metadata(d.path().join("identity.key")).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }
    #[test]
    fn settings_are_checked() {
        let mut c = Config::default();
        assert!(c.apply(&json!({"relay": "custom"})).is_err());
        assert!(c.apply(&json!({"relay": "custom", "relayUrl": "ftp://x"})).is_err());
        c.apply(&json!({"relay": "custom", "relayUrl": "https://relay.example.com"})).unwrap();
        assert_eq!(c.relay, Relay::Custom);
        c.apply(&json!({"name": "  Living\u{7} room  ", "streams": 99})).unwrap();
        assert_eq!(c.name, "Living room");
        assert_eq!(c.streams, 32);
        assert!(c.apply(&json!({"compress": "max"})).is_err());
    }
    #[test]
    fn peers_are_kept() {
        let d = tempfile::tempdir().unwrap();
        let mut p = Peers::load(d.path());
        p.add("ab", "Deck");
        p.seen("ab", Some("10.0.0.2:47740".parse().unwrap()));
        let q = Peers::load(d.path());
        assert!(q.is_paired("ab"));
        assert_eq!(q.get("ab").unwrap().name, "Deck");
        let mut q = q;
        assert!(q.remove("ab"));
        assert!(!Peers::load(d.path()).is_paired("ab"));
    }
}
