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
    /// Share the clipboard's text with your own devices (clipboard.rs); off until the person turns it on.
    pub clipboard: bool,
    /// config.json was there but couldn't be read (and had no good .bak): these are defaults, so saving writes only
    /// what the person changes and never defaults over their file (red flag 9; settings_file.rs keeps it aside).
    #[serde(skip)]
    pub unreadable: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config { name: String::new(), relay: Relay::Default, relay_url: String::new(), always_accept: false, sync: true, compress: "auto".into(), streams: 16, clipboard: false, unreadable: false }
    }
}

impl Config {
    /// Red flag 9 (settings never change by themselves), through hearth-core's settings_file.rs: a damaged file is put
    /// back from config.json.bak; one that can't be read at all leaves Config marked `unreadable` (defaults shown,
    /// nothing written over it).
    pub fn load(dir: &Path) -> Config {
        use hearth_core::settings_file::{read, Read};
        match read(&dir.join("config.json")) {
            Read::Ok(m) | Read::Restored(m) => match serde_json::from_value::<Config>(Value::Object(m)) {
                Ok(c) => c,
                Err(e) => {
                    hearth_core::settings_file::log_event(&format!("Hearth Link: config.json has a value it can't use ({e}); nothing is written over it"));
                    Config { unreadable: true, ..Config::default() }
                }
            },
            Read::Missing => Config::default(),
            Read::Unreadable => Config { unreadable: true, ..Config::default() },
        }
    }
    /// Atomic, with config.json.bak, merged under the lock with what's on disk (other keys stay). After a file that
    /// couldn't be read, only what the person changed (what differs from the defaults) is written, never the defaults.
    pub fn save(&self, dir: &Path) -> Result<(), String> {
        let Ok(Value::Object(mine)) = serde_json::to_value(self) else { return Err("Couldn't save Hearth Link's settings".into()) };
        let changed: serde_json::Map<String, Value> = if self.unreadable {
            let Ok(Value::Object(def)) = serde_json::to_value(Config::default()) else { return Err("Couldn't save Hearth Link's settings".into()) };
            mine.into_iter().filter(|(k, v)| def.get(k) != Some(v)).collect()
        } else {
            mine
        };
        if changed.is_empty() {
            return Ok(());
        }
        hearth_core::settings_file::merge_json(&dir.join("config.json"), &changed).map(|_| ()).map_err(|e| format!("Couldn't save Hearth Link's settings: {e}"))
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
        if let Some(b) = a.get("clipboard").and_then(Value::as_bool) { self.clipboard = b; }
        if let Some(c) = a.get("compress").and_then(Value::as_str) {
            if c != "auto" && c != "off" { return Err("Compression is auto or off".into()); }
            self.compress = c.into();
        }
        if let Some(s) = a.get("streams").and_then(Value::as_u64) { self.streams = (s as usize).clamp(1, 32); }
        Ok(())
    }
    pub fn to_json(&self) -> Value {
        json!({"name": self.display_name(), "customName": self.name, "relay": self.relay, "relayUrl": self.relay_url, "alwaysAccept": self.always_accept,
               "sync": self.sync, "compress": self.compress, "streams": self.streams, "clipboard": self.clipboard})
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
    /// A friend's device (pairing: either side picked "A friend's device"): paired with this device only, and only
    /// for what makes sense between two people (game copies, seeing it online). Missing in peers.json from before
    /// this: false, since every device paired before was the owner's own.
    pub friend: bool,
    /// One of your devices that told this one about it (family.rs), instead of pairing with a code.
    pub introduced_by: Option<String>,
    /// What it said it is in hello (handheld, laptop, desktop, tablet, tv): a TV gets Send to TV (tv.rs).
    pub kind: String,
    /// Whose it is (family.rs): "mine", "family" or "friend". Missing in peers.json from before family devices:
    /// read from `friend` as before. `friend` is still written (true for a family member's device too), so an older
    /// Hearth reads a family device as a friend's: the safe side.
    pub relation: String,
    /// For a family member's device: "member" (it's a family member's device, this one helps on it), "helper" (it's
    /// a device of someone who helps on this one) or "both" (each picked the other as family).
    pub family_side: String,
    /// What this device lets that family device have (switches on this device only; family.rs).
    pub shares: Shares,
    /// What that family device said it lets this one have (shown only: what counts is checked on its side).
    pub their_shares: Shares,
    /// A family member's device's network cards, so your devices can wake it (it says them in hello).
    pub macs: Vec<String>,
    /// That PC lets this one connect with Moonlight (moonlight.rs; it said so): "desktop" or "games", empty if not.
    pub moonlight: String,
}

/// Whose a paired device is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Rel { #[default] Mine, Family, Friend }

impl Rel {
    pub fn as_str(self) -> &'static str { match self { Rel::Mine => "mine", Rel::Family => "family", Rel::Friend => "friend" } }
    pub fn parse(s: &str) -> Option<Rel> { match s { "mine" => Some(Rel::Mine), "family" => Some(Rel::Family), "friend" => Some(Rel::Friend), _ => None } }
}

/// A family member's device's switches, per device of the person who helps on it (family.rs). All off by default,
/// except its status (`None`: shared with the people who help on it, not the other way round).
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Shares {
    pub status: Option<bool>,
    /// Share my game library (favorites, hidden games, collections, play time; never settings).
    pub library: bool,
    /// Back up my saves (and saves follow it) on that person's Hearth Storage.
    pub saves: bool,
    /// Use Hearth Storage (shared folders, the game archive, the download and update caches).
    pub storage: bool,
    /// Back up this PC (saves and screenshots) to that person's Hearth Storage.
    pub backup: bool,
}

impl Shares {
    pub fn get(&self, what: &str) -> bool {
        match what { "library" => self.library, "saves" => self.saves, "storage" => self.storage, "backup" => self.backup, "status" => self.status == Some(true), _ => false }
    }
}

impl Peer {
    pub fn rel(&self) -> Rel {
        match (self.relation.as_str(), self.friend) {
            // `friend` set with "mine" (an older Hearth made it a friend's device): a friend's, the safe side.
            ("mine", false) => Rel::Mine,
            ("family", _) => Rel::Family,
            ("", false) => Rel::Mine,
            _ => Rel::Friend,
        }
    }
    pub fn mine(&self) -> bool { self.rel() == Rel::Mine }
    pub fn family(&self) -> bool { self.rel() == Rel::Family }
    /// Sets whose it is (and `friend` for older Hearth: true unless it's yours).
    pub fn set_rel(&mut self, r: Rel, side: &str) {
        self.relation = r.as_str().to_string();
        self.friend = r != Rel::Mine;
        self.family_side = if r == Rel::Family { side.to_string() } else { String::new() };
        if r != Rel::Mine { if let Some(relay) = self.signed_in.get("relay").cloned() { self.signed_in = json!({"relay": relay}); } else { self.signed_in = json!({}); } }
        if r != Rel::Family { self.shares = Shares::default(); self.their_shares = Shares::default(); self.macs.clear(); }
    }
    /// A family member's device this one helps on (it shows in Your devices; wake it, remote desktop).
    pub fn i_help(&self) -> bool { self.family() && matches!(self.family_side.as_str(), "member" | "both" | "") }
    /// A device of someone who helps on this one (this is the family member's device to it).
    pub fn helps_me(&self) -> bool { self.family() && matches!(self.family_side.as_str(), "helper" | "both") }
    /// This device lets it see its status: your own devices (Settings' own switch decides), a family device per its
    /// switch (on by default only toward the people who help on this one).
    pub fn status_shared(&self) -> bool {
        match self.rel() { Rel::Mine => true, Rel::Friend => false, Rel::Family => self.shares.status.unwrap_or(self.family_side == "helper") }
    }
    /// A kind of sharing (library, saves, storage, backup) is on between this device and that family device: the
    /// family member's own switch decides (on this device when it's the family member's, as it said otherwise).
    pub fn lets(&self, what: &str) -> bool {
        if !self.family() { return self.mine(); }
        (self.helps_me() && self.shares.get(what)) || (self.i_help() && self.their_shares.get(what))
    }
}

/// A device of yours that was removed (on this device or another of yours), so the others drop it too and nobody
/// adds it back from an older list. Pairing with it again (a newer pairedAt) wins.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Removed { pub id: String, pub at: u64 }

pub struct Peers { pub file: PathBuf, pub list: Vec<Peer>, pub removed: Vec<Removed> }

impl Peers {
    pub fn load(dir: &Path) -> Peers {
        let file = dir.join("peers.json");
        let list = std::fs::read(&file).ok().and_then(|b| serde_json::from_slice::<Vec<Peer>>(&b).ok()).unwrap_or_default();
        let removed = std::fs::read(dir.join("removed-devices.json")).ok().and_then(|b| serde_json::from_slice::<Vec<Removed>>(&b).ok()).unwrap_or_default();
        Peers { file, list, removed }
    }
    pub fn save(&self) -> Result<(), String> {
        crate::write_atomic(&self.file, serde_json::to_string_pretty(&self.list).unwrap().as_bytes())?;
        let rf = self.file.with_file_name("removed-devices.json");
        if !self.removed.is_empty() || rf.exists() { crate::write_atomic(&rf, serde_json::to_string_pretty(&self.removed).unwrap().as_bytes())?; }
        Ok(())
    }
    /// Your own devices (not friends'): the only ones anything personal goes to or comes from.
    pub fn mine(&self) -> impl Iterator<Item = &Peer> { self.list.iter().filter(|p| p.mine()) }
    pub fn is_mine(&self, id: &str) -> bool { self.get(id).map(Peer::mine).unwrap_or(false) }
    /// That device may be asked `op` by this one (family.rs may_ask): checked before any request goes out.
    pub fn may_ask(&self, id: &str, op: &str) -> bool { self.get(id).map(|p| crate::family::may_ask(p, op)).unwrap_or(false) }
    /// Your devices and the family devices this one shares `what` with (library, saves, storage, backup).
    pub fn letting<'a>(&'a self, what: &'a str) -> impl Iterator<Item = &'a Peer> + 'a { self.list.iter().filter(move |p| p.lets(what)) }
    /// Paired with a code (pairing), as one of your devices or a friend's. Pairing again clears a removal.
    pub fn add_paired(&mut self, id: &str, name: &str, friend: bool) {
        self.add_paired_as(id, name, if friend { Rel::Friend } else { Rel::Mine }, "");
    }
    /// Paired with a code as `r` (family: `side`, as pair.rs worked it out). Pairing again starts its switches over.
    pub fn add_paired_as(&mut self, id: &str, name: &str, r: Rel, side: &str) {
        self.add(id, name);
        if let Some(p) = self.get_mut(id) {
            p.set_rel(r, side);
            if r == Rel::Family { p.shares = Shares::default(); p.their_shares = Shares::default(); }
            p.introduced_by = None; p.paired_at = crate::now_ms();
        }
        self.removed.retain(|r| r.id != id);
        let _ = self.save();
    }
    /// Removing one of your devices is remembered, so your other devices drop it too (family.rs).
    pub fn remove_and_remember(&mut self, id: &str) -> bool {
        // Yours, or a family member's device your devices share: the others drop it too.
        let mine = self.get(id).map(|p| p.mine() || p.i_help()).unwrap_or(false);
        let changed = self.remove(id);
        if changed && mine { crate::family::remember_removed(&mut self.removed, id, crate::now_ms()); let _ = self.save(); }
        changed
    }
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
mod config_file {
    use super::*;

    #[test]
    fn saving_keeps_a_bak_and_the_keys_it_does_not_know() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("config.json");
        std::fs::write(&p, r#"{"name": "Den", "sync": false, "fromNewer": 7}"#).unwrap();
        let mut c = Config::load(d.path());
        assert_eq!((c.name.as_str(), c.sync, c.unreadable), ("Den", false, false));
        c.apply(&json!({"alwaysAccept": true})).unwrap();
        c.save(d.path()).unwrap();
        let v: Value = serde_json::from_slice(&std::fs::read(&p).unwrap()).unwrap();
        assert_eq!((v["alwaysAccept"].as_bool(), v["sync"].as_bool(), v["fromNewer"].as_i64()), (Some(true), Some(false), Some(7)));
        let bak: Value = serde_json::from_slice(&std::fs::read(d.path().join("config.json.bak")).unwrap()).unwrap();
        assert_eq!(bak["name"], "Den", "the copy before is kept as config.json.bak");
    }

    #[test]
    fn a_damaged_file_comes_back_from_its_bak() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("config.json"), b"{\"name\": \"De").unwrap();
        std::fs::write(d.path().join("config.json.bak"), br#"{"name": "Den", "clipboard": true}"#).unwrap();
        let c = Config::load(d.path());
        assert_eq!((c.name.as_str(), c.clipboard, c.unreadable), ("Den", true, false));
    }

    #[test]
    fn defaults_never_go_over_a_file_that_could_not_be_read() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("config.json");
        std::fs::write(&p, b"{not json, and no bak").unwrap();
        let mut c = Config::load(d.path());
        assert!(c.unreadable);
        // Saving with nothing changed writes nothing at all.
        c.save(d.path()).unwrap();
        assert_eq!(std::fs::read(&p).unwrap(), b"{not json, and no bak");
        // The person changes one thing: only that is written (the damaged file is kept aside), no defaults.
        c.apply(&json!({"clipboard": true})).unwrap();
        c.save(d.path()).unwrap();
        let v: Value = serde_json::from_slice(&std::fs::read(&p).unwrap()).unwrap();
        assert_eq!(v, json!({"clipboard": true}));
        assert!(std::fs::read_dir(d.path()).unwrap().any(|e| e.unwrap().file_name().to_string_lossy().starts_with("config.json.broken-")));
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
    #[test]
    fn devices_paired_before_friends_existed_are_yours() {
        let p: Vec<Peer> = serde_json::from_str(r#"[{"id":"ab","name":"Deck","pairedAt":1,"lastSeen":2,"addrs":[],"alwaysAccept":false,"signedIn":{}}]"#).unwrap();
        assert!(p[0].mine());
        assert!(!p[0].friend);
        assert_eq!(p[0].introduced_by, None);
    }
    #[test]
    fn friends_and_removals_are_kept() {
        let d = tempfile::tempdir().unwrap();
        let mut p = Peers::load(d.path());
        p.add_paired("me1", "Ally", false);
        p.add_paired("fr1", "Sam's Deck", true);
        assert!(p.is_mine("me1"));
        assert!(!p.is_mine("fr1"));
        assert!(p.is_paired("fr1"));
        assert_eq!(p.mine().count(), 1);
        // Removing a friend's device isn't passed on; removing yours is.
        assert!(p.remove_and_remember("fr1"));
        assert!(p.removed.is_empty());
        assert!(p.remove_and_remember("me1"));
        assert_eq!(p.removed.len(), 1);
        let q = Peers::load(d.path());
        assert_eq!(q.removed[0].id, "me1");
        // Pairing again clears it.
        let mut q = q;
        q.add_paired("me1", "Ally", false);
        assert!(Peers::load(d.path()).removed.is_empty());
    }
}
