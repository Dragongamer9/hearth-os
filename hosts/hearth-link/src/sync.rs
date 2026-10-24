//! Your Hearth data, kept the same on every paired device.
//!
//! What's synced, as fields ("p/<profile>/..." for a profile's own, "g/..." for the device's):
//!   p/<p>/fav/<game>, p/<p>/hidden/<game>   favorites and hidden games
//!   p/<p>/col/<id>                          a collection (name and games)
//!   p/<p>/opt/<game>                        per-game settings (not the power limits: those belong to a device)
//!   p/<p>/profile, p/<p>/pic/<kind>         your profile (name, status, colour...) and its pictures
//!   p/<p>/qr                                Quick Resume on or off
//!   g/ui/<key>                              appearance, layouts, Made for, controller choices (the UI's own)
//!   g/input/<file>                          Hearth Input controller layouts (config/input)
//!   g/beacon/custom                         Beacon's games added by hand (custom-games.json)
//! and play time: every device's own minutes and last played per game, added up (never overwritten).
//!
//! Rules: newest change wins, per field (time, then device id on a tie); a removed thing is a field set to null,
//! so removing a favorite on one device removes it on the other. Local changes are found by comparing with what
//! was there last time (the shadow), so nothing in Hearth has to tell Hearth Link what changed.
//!
//! Hearth's library.json is the running app's (it keeps it in memory), so changes to it go into an inbox
//! (inbox.json, written under a lock) that Hearth applies itself (hearth-core link.rs) the next time it reads the
//! library; the other files are written here directly. Sign-ins are never part of this.

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Field {
    pub v: Value,
    pub t: u64,
    pub d: String,
    #[serde(default)]
    pub seq: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Play { pub m: u64, pub l: u64 }

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SyncState {
    pub fields: BTreeMap<String, Field>,
    pub shadow: BTreeMap<String, String>,
    /// "p/<profile>/<game>" → device → its own play time.
    pub play: BTreeMap<String, BTreeMap<String, Play>>,
    /// Other devices' minutes already added to this device's play time, per "p/<profile>/<game>".
    pub applied: BTreeMap<String, u64>,
    pub rev: u64,
    /// Our rev each peer has everything up to; each peer's rev we have everything up to.
    pub ack: BTreeMap<String, u64>,
    pub got: BTreeMap<String, u64>,
}

/// Where the synced things are on this device.
#[derive(Debug, Clone)]
pub struct Sources { pub data: PathBuf, pub config: PathBuf, pub link: PathBuf, pub beacon_custom: PathBuf }

impl Sources {
    pub fn here() -> Sources {
        let home = hearth_core::paths::home();
        Sources { data: hearth_core::paths::data_dir(), config: hearth_core::paths::config_dir(), link: crate::home_dir(), beacon_custom: crate::games::beacon_custom_file(&home) }
    }
    fn inbox(&self) -> PathBuf { self.link.join("inbox.json") }
    fn ui(&self) -> PathBuf { self.link.join("ui.json") }
}

pub fn inbox_file() -> PathBuf { crate::home_dir().join("inbox.json") }

const OPT_DEVICE_ONLY: [&str; 3] = ["tdp", "hhdPower", "powerLabel"];
const PROFILE_KEYS: [&str; 6] = ["name", "status", "avatar", "accent", "background", "imageStamp"];
const PIC_EXT: [&str; 5] = ["png", "jpg", "jpeg", "webp", "gif"];
const PIC_MAX: u64 = 3 << 20;

fn read_json(p: &Path) -> Value { std::fs::read(p).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null) }
fn hash_of(v: &Value) -> String { blake3::hash(serde_json::to_string(v).unwrap_or_default().as_bytes()).to_hex().to_string() }

pub fn active_profile(data: &Path) -> String {
    read_json(&data.join("profiles.json"))["active"].as_str().filter(|s| valid_part(s)).unwrap_or("main").to_string()
}

/// One part of a field's key from the network or from files: no slashes, nothing odd, not too long.
pub fn valid_part(s: &str) -> bool { !s.is_empty() && s.len() <= 200 && !s.contains('/') && !s.chars().any(|c| c.is_control()) && s != "." && s != ".." }

/// An input layout's file name: what hearth-core's input_map makes (letters, digits, - _ . %, .json).
fn valid_input_file(s: &str) -> bool {
    s.ends_with(".json") && s.len() <= 220 && !s.starts_with('.') && s.chars().all(|c| c.is_ascii_alphanumeric() || "-_.%".contains(c))
}

pub fn valid_key(k: &str) -> bool {
    if k.len() > 300 { return false; }
    let parts: Vec<&str> = k.split('/').collect();
    match parts.as_slice() {
        ["p", p, "profile"] | ["p", p, "qr"] => valid_part(p),
        ["p", p, kind, x] if ["fav", "hidden", "col", "opt"].contains(kind) => valid_part(p) && valid_part(x),
        ["p", p, "pic", kind] => valid_part(p) && (*kind == "avatar" || *kind == "background"),
        ["g", "ui", x] => valid_part(x) && x.len() <= 60,
        ["g", "input", f] => valid_input_file(f),
        ["g", "beacon", "custom"] => true,
        _ => false,
    }
}

// ------------------------------------------------------------------ base64 (pictures)

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
pub fn b64(data: &[u8]) -> String {
    let mut s = String::with_capacity(data.len().div_ceil(3) * 4);
    for c in data.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        for i in 0..4 { if i <= c.len() { s.push(B64[(n >> (18 - 6 * i) & 63) as usize] as char); } else { s.push('='); } }
    }
    s
}
pub fn unb64(s: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(s.len() / 4 * 3);
    let mut buf = 0u32; let mut bits = 0;
    for ch in s.bytes() {
        if ch == b'=' { break; }
        let v = B64.iter().position(|x| *x == ch)? as u32;
        buf = buf << 6 | v; bits += 6;
        if bits >= 8 { bits -= 8; out.push((buf >> bits) as u8); buf &= (1 << bits) - 1; }
    }
    Some(out)
}

// ------------------------------------------------------------------ this device's data as fields

/// The active profile, every field it and the device have now, and the profile's play time (minutes, last played).
pub fn extract(src: &Sources) -> (String, BTreeMap<String, Value>, BTreeMap<String, Play>) {
    let pid = active_profile(&src.data);
    let p = format!("p/{pid}/");
    let mut out = BTreeMap::new();
    let lib = read_json(&src.data.join("library.json"));
    for (list, kind) in [("favorites", "fav"), ("hidden", "hidden")] {
        for id in lib[list].as_array().into_iter().flatten().filter_map(Value::as_str).filter(|s| valid_part(s)) { out.insert(format!("{p}{kind}/{id}"), json!(true)); }
    }
    for c in lib["collections"].as_array().into_iter().flatten() {
        if let Some(id) = c["id"].as_str().filter(|s| valid_part(s)) { out.insert(format!("{p}col/{id}"), json!({"name": c["name"], "games": c["games"]})); }
    }
    for (gid, o) in lib["gameOptions"].as_object().into_iter().flatten() {
        if !valid_part(gid) { continue; }
        let mut o = o.clone();
        if let Some(m) = o.as_object_mut() { for k in OPT_DEVICE_ONLY { m.remove(k); } }
        out.insert(format!("{p}opt/{gid}"), o);
    }
    if let Some(prof) = lib["profile"].as_object() {
        let m: Map<String, Value> = prof.iter().filter(|(k, _)| PROFILE_KEYS.contains(&k.as_str())).map(|(k, v)| (k.clone(), v.clone())).collect();
        out.insert(format!("{p}profile"), Value::Object(m));
    }
    if let Some(q) = lib.get("quickResume").filter(|v| v.is_boolean()) { out.insert(format!("{p}qr"), q.clone()); }
    for kind in ["avatar", "background"] {
        for ext in PIC_EXT {
            let f = src.data.join("profile").join(format!("{kind}.{ext}"));
            let Ok(md) = std::fs::metadata(&f) else { continue };
            if md.len() > PIC_MAX { break; }
            if let Ok(b) = std::fs::read(&f) { out.insert(format!("{p}pic/{kind}"), json!({"ext": ext, "b64": b64(&b)})); }
            break;
        }
    }
    let ui = read_json(&src.ui());
    for (k, v) in ui["local"].as_object().into_iter().flatten() { if valid_part(k) && k.len() <= 60 { out.insert(format!("g/ui/{k}"), v.clone()); } }
    for e in std::fs::read_dir(src.config.join("input")).into_iter().flatten().flatten() {
        let n = e.file_name().to_string_lossy().to_string();
        if !valid_input_file(&n) { continue; }
        let v = read_json(&e.path());
        if !v.is_null() { out.insert(format!("g/input/{n}"), v); }
    }
    let bc = read_json(&src.beacon_custom);
    if !bc.is_null() { out.insert("g/beacon/custom".into(), bc); }
    let mut stats = BTreeMap::new();
    for (gid, s) in lib["stats"].as_object().into_iter().flatten() {
        if !valid_part(gid) { continue; }
        stats.insert(gid.clone(), Play { m: s["minutes"].as_u64().unwrap_or(0), l: s["lastPlayed"].as_u64().unwrap_or(0) });
    }
    (pid, out, stats)
}

// ------------------------------------------------------------------ the inbox (changes for Hearth to apply)

fn with_lock<T>(link: &Path, f: impl FnOnce() -> T) -> T {
    let _ = std::fs::create_dir_all(link);
    let lock = std::fs::OpenOptions::new().create(true).truncate(false).write(true).open(link.join("inbox.lock"));
    if let Ok(l) = &lock { let _ = l.lock(); }
    let r = f();
    if let Ok(l) = &lock { let _ = l.unlock(); }
    r
}

/// Merges `patch` into the inbox: {"profiles": {p: {"set": {k: v}, "add": {game: minutes}, "last": {game: ms}}},
/// "gogInstalls": {id: folder}}. "set" replaces, "add" adds up, "last" keeps the latest.
pub fn inbox_add(file: &Path, patch: &Value) -> Result<(), String> {
    let link = file.parent().unwrap_or(Path::new(".")).to_path_buf();
    with_lock(&link, || {
        let mut cur = read_json(file);
        if !cur.is_object() { cur = json!({}); }
        merge_inbox(&mut cur, patch);
        crate::write_atomic(file, serde_json::to_string(&cur).unwrap().as_bytes())
    })
}

pub fn merge_inbox(cur: &mut Value, patch: &Value) {
    if let Some(g) = patch["gogInstalls"].as_object() {
        for (k, v) in g { cur["gogInstalls"][k] = v.clone(); }
    }
    for (pid, pp) in patch["profiles"].as_object().into_iter().flatten() {
        let slot = &mut cur["profiles"][pid];
        for (k, v) in pp["set"].as_object().into_iter().flatten() { slot["set"][k] = v.clone(); }
        for (k, v) in pp["add"].as_object().into_iter().flatten() {
            let n = slot["add"][k].as_u64().unwrap_or(0) + v.as_u64().unwrap_or(0);
            slot["add"][k] = json!(n);
        }
        for (k, v) in pp["last"].as_object().into_iter().flatten() {
            let n = slot["last"][k].as_u64().unwrap_or(0).max(v.as_u64().unwrap_or(0));
            slot["last"][k] = json!(n);
        }
    }
}

impl SyncState {
    pub fn load(link: &Path) -> SyncState { std::fs::read(link.join("sync.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default() }
    pub fn save(&self, link: &Path) -> Result<(), String> { crate::write_atomic(&link.join("sync.json"), serde_json::to_string(self).unwrap().as_bytes()) }

    fn stamp(&mut self, k: &str, v: Value, me: &str) {
        self.rev += 1;
        self.fields.insert(k.to_string(), Field { v, t: crate::now_ms(), d: me.to_string(), seq: self.rev });
    }

    /// Finds what changed on this device since last time. True when something did.
    pub fn scan(&mut self, me: &str, src: &Sources) -> bool {
        let (pid, cur, stats) = extract(src);
        let inbox = read_json(&src.inbox());
        let pending: BTreeSet<String> = inbox["profiles"].as_object().into_iter().flatten()
            .flat_map(|(p, x)| x["set"].as_object().into_iter().flatten().map(move |(k, _)| format!("p/{p}/{k}"))).collect();
        let before = self.rev;
        for (k, v) in &cur {
            if pending.contains(k) { continue; }
            let h = hash_of(v);
            if self.shadow.get(k) == Some(&h) { continue; }
            self.shadow.insert(k.clone(), h);
            if self.fields.get(k).map(|f| &f.v) != Some(v) { self.stamp(k, v.clone(), me); }
        }
        // Gone since last time: only this profile's and the device's own (another profile's fields aren't here).
        let mine = format!("p/{pid}/");
        let gone: Vec<String> = self.shadow.keys().filter(|k| (k.starts_with(&mine) || k.starts_with("g/")) && !cur.contains_key(*k) && !pending.contains(*k)).cloned().collect();
        for k in gone {
            self.shadow.remove(&k);
            if self.fields.get(&k).map(|f| !f.v.is_null()).unwrap_or(false) { self.stamp(&k, Value::Null, me); }
        }
        // Play time: this device's own minutes are its total less what other devices' minutes added (and what's
        // still waiting in the inbox to be added).
        for (g, s) in stats {
            let pg = format!("p/{pid}/{g}");
            let pend = inbox["profiles"][&pid]["add"][&g].as_u64().unwrap_or(0);
            let own = (s.m + pend).saturating_sub(*self.applied.get(&pg).unwrap_or(&0));
            let e = self.play.entry(pg).or_default().entry(me.to_string()).or_default();
            if own > e.m || s.l > e.l { e.m = e.m.max(own); e.l = e.l.max(s.l); self.rev += 1; }
        }
        self.rev != before
    }

    /// What to send a peer: fields it hasn't got yet, and all play time.
    pub fn outgoing(&self, peer: &str) -> Value {
        let since = *self.ack.get(peer).unwrap_or(&0);
        let f: Map<String, Value> = self.fields.iter().filter(|(_, f)| f.seq > since).map(|(k, f)| (k.clone(), json!({"v": f.v, "t": f.t, "d": f.d}))).collect();
        json!({"rev": self.rev, "got": self.got.get(peer).copied().unwrap_or(0), "fields": f, "play": self.play})
    }

    /// Takes a peer's fields and play time. Returns the keys that changed here and the play entries that grew.
    pub fn merge(&mut self, me: &str, msg: &Value) -> (Vec<String>, Vec<String>) {
        let mut changed = Vec::new();
        for (k, f) in msg["fields"].as_object().into_iter().flatten() {
            if !valid_key(k) { continue; }
            let (Some(t), Some(d)) = (f["t"].as_u64(), f["d"].as_str()) else { continue };
            let newer = match self.fields.get(k) { None => true, Some(l) => (t, d) > (l.t, l.d.as_str()) };
            if !newer { continue; }
            self.rev += 1;
            self.fields.insert(k.clone(), Field { v: f["v"].clone(), t, d: d.chars().take(80).collect(), seq: self.rev });
            changed.push(k.clone());
        }
        let mut grew = Vec::new();
        for (pg, devs) in msg["play"].as_object().into_iter().flatten() {
            let parts: Vec<&str> = pg.splitn(3, '/').collect();
            if parts.len() != 3 || parts[0] != "p" || !valid_part(parts[1]) || !valid_part(parts[2]) { continue; }
            for (dev, p) in devs.as_object().into_iter().flatten() {
                if dev == me || dev.len() > 80 { continue; }   // this device's own minutes are counted here
                let (m, l) = (p["m"].as_u64().unwrap_or(0), p["l"].as_u64().unwrap_or(0));
                let e = self.play.entry(pg.clone()).or_default().entry(dev.clone()).or_default();
                if m > e.m || l > e.l { e.m = e.m.max(m); e.l = e.l.max(l); if !grew.contains(pg) { grew.push(pg.clone()); } }
            }
        }
        if !grew.is_empty() { self.rev += 1; }
        (changed, grew)
    }

    /// Puts merged changes in place: files written here, library.json's through the inbox.
    pub fn apply(&mut self, me: &str, src: &Sources, changed: &[String], grew: &[String]) -> Result<(), String> {
        let active = active_profile(&src.data);
        let mut patch = json!({"profiles": {}});
        let mut ui = read_json(&src.ui());
        let mut ui_changed = false;
        for k in changed {
            let Some(f) = self.fields.get(k).cloned() else { continue };
            let parts: Vec<&str> = k.split('/').collect();
            let mut set_shadow = true;
            match parts.as_slice() {
                ["g", "input", file] => {
                    let path = src.config.join("input").join(file);
                    if f.v.is_null() { let _ = std::fs::remove_file(&path); } else { crate::write_atomic(&path, serde_json::to_string_pretty(&f.v).unwrap().as_bytes())?; }
                }
                ["g", "beacon", "custom"] => {
                    if !f.v.is_null() { crate::write_atomic(&src.beacon_custom, serde_json::to_string_pretty(&f.v).unwrap().as_bytes())?; }
                }
                ["g", "ui", key] => {
                    if !ui.is_object() { ui = json!({}); }
                    ui["remote"][*key] = json!({"v": f.v, "t": f.t});
                    if f.v.is_null() { if let Some(m) = ui["local"].as_object_mut() { m.remove(*key); } } else { ui["local"][*key] = f.v.clone(); }
                    ui_changed = true;
                }
                ["p", pid, "pic", kind] => {
                    if *pid != active { set_shadow = false; }
                    else {
                        let dir = src.data.join("profile");
                        for ext in PIC_EXT { let _ = std::fs::remove_file(dir.join(format!("{kind}.{ext}"))); }
                        let ext = f.v["ext"].as_str().filter(|e| PIC_EXT.contains(e));
                        let bytes = f.v["b64"].as_str().and_then(unb64).filter(|b| b.len() as u64 <= PIC_MAX);
                        if let (Some(ext), Some(b)) = (ext, bytes) { crate::write_atomic(&dir.join(format!("{kind}.{ext}")), &b)?; }
                    }
                }
                ["p", pid, rest @ ..] => {
                    patch["profiles"][*pid]["set"][rest.join("/")] = f.v.clone();
                    set_shadow = false;   // when Hearth has applied it, the next scan finds it there
                }
                _ => { set_shadow = false; }
            }
            if set_shadow {
                if f.v.is_null() { self.shadow.remove(k); } else { self.shadow.insert(k.clone(), hash_of(&f.v)); }
            }
        }
        for pg in grew {
            let parts: Vec<&str> = pg.splitn(3, '/').collect();
            let (pid, g) = (parts[1], parts[2]);
            let devs = self.play.get(pg).cloned().unwrap_or_default();
            let others: u64 = devs.iter().filter(|(d, _)| d.as_str() != me).map(|(_, p)| p.m).sum();
            let last = devs.values().map(|p| p.l).max().unwrap_or(0);
            let prev = *self.applied.get(pg).unwrap_or(&0);
            if others > prev { patch["profiles"][pid]["add"][g] = json!(others - prev); }
            patch["profiles"][pid]["last"][g] = json!(last);
            self.applied.insert(pg.clone(), others.max(prev));
        }
        if ui_changed { crate::write_atomic(&src.ui(), serde_json::to_string(&ui).unwrap().as_bytes())?; }
        if patch["profiles"].as_object().map(|m| !m.is_empty()).unwrap_or(false) { inbox_add(&src.inbox(), &patch)?; }
        Ok(())
    }

    /// The device-specific fields that were never synced (for the UI): how many fields, and when the last change was.
    pub fn summary(&self) -> Value {
        let last = self.fields.values().map(|f| f.t).max().unwrap_or(0);
        json!({"fields": self.fields.values().filter(|f| !f.v.is_null()).count(), "lastChange": last, "games": self.play.len()})
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn device(root: &Path, name: &str) -> Sources {
        let base = root.join(name);
        let s = Sources { data: base.join("data"), config: base.join("config"), link: base.join("data/link"), beacon_custom: base.join("config/beacon/custom-games.json") };
        std::fs::create_dir_all(&s.link).unwrap();
        s
    }
    fn write(p: &Path, v: Value) { std::fs::create_dir_all(p.parent().unwrap()).unwrap(); std::fs::write(p, v.to_string()).unwrap(); }

    /// What hearth-core's link.rs does with the inbox (a copy of its rules, for these tests).
    fn hearth_applies(s: &Sources) {
        let inbox = read_json(&s.inbox());
        let mut lib = read_json(&s.data.join("library.json"));
        if !lib.is_object() { lib = json!({}); }
        let pid = active_profile(&s.data);
        let pp = &inbox["profiles"][&pid];
        for (k, v) in pp["set"].as_object().into_iter().flatten() {
            let (kind, id) = k.split_once('/').unwrap_or((k.as_str(), ""));
            match kind {
                "fav" | "hidden" => {
                    let list = if kind == "fav" { "favorites" } else { "hidden" };
                    let mut a: Vec<Value> = lib[list].as_array().cloned().unwrap_or_default();
                    a.retain(|x| x != id);
                    if v == &json!(true) { a.push(json!(id)); }
                    lib[list] = json!(a);
                }
                "profile" => { for (pk, pv) in v.as_object().into_iter().flatten() { lib["profile"][pk] = pv.clone(); } }
                _ => {}
            }
        }
        for (g, n) in pp["add"].as_object().into_iter().flatten() {
            let m = lib["stats"][g]["minutes"].as_u64().unwrap_or(0) + n.as_u64().unwrap();
            lib["stats"][g]["minutes"] = json!(m);
        }
        for (g, n) in pp["last"].as_object().into_iter().flatten() {
            let l = lib["stats"][g]["lastPlayed"].as_u64().unwrap_or(0).max(n.as_u64().unwrap());
            lib["stats"][g]["lastPlayed"] = json!(l);
        }
        write(&s.data.join("library.json"), lib);
        let _ = std::fs::remove_file(s.inbox());
    }

    fn exchange(a: &mut SyncState, an: &str, asrc: &Sources, b: &mut SyncState, bn: &str, bsrc: &Sources) {
        a.scan(an, asrc); b.scan(bn, bsrc);
        // A asks, B answers (what node.rs does).
        let req = a.outgoing(bn);
        let (c, g) = b.merge(bn, &req);
        b.apply(bn, bsrc, &c, &g).unwrap();
        b.got.insert(an.into(), req["rev"].as_u64().unwrap());
        b.ack.insert(an.into(), req["got"].as_u64().unwrap());
        let resp = b.outgoing(an);
        let (c, g) = a.merge(an, &resp);
        a.apply(an, asrc, &c, &g).unwrap();
        a.got.insert(bn.into(), resp["rev"].as_u64().unwrap());
        a.ack.insert(bn.into(), resp["got"].as_u64().unwrap());
    }

    #[test]
    fn favorites_profile_and_play_time_both_ways() {
        let root = tempfile::tempdir().unwrap();
        let (sa, sb) = (device(root.path(), "a"), device(root.path(), "b"));
        write(&sa.data.join("library.json"), json!({"favorites": ["steam:620"], "profile": {"name": "Ana", "accent": "#ff8a3d"}, "stats": {"steam:620": {"minutes": 100, "lastPlayed": 1000}}}));
        write(&sb.data.join("library.json"), json!({"favorites": ["gog:1"], "stats": {"steam:620": {"minutes": 50, "lastPlayed": 2000}}}));
        write(&sa.config.join("input/steam%3A620.json"), json!({"preset": "fps"}));
        let (mut a, mut b) = (SyncState::default(), SyncState::default());
        exchange(&mut a, "A", &sa, &mut b, "B", &sb);
        hearth_applies(&sa); hearth_applies(&sb);
        let la = read_json(&sa.data.join("library.json"));
        let lb = read_json(&sb.data.join("library.json"));
        let favs = |l: &Value| { let mut v: Vec<String> = l["favorites"].as_array().unwrap().iter().map(|x| x.as_str().unwrap().to_string()).collect(); v.sort(); v };
        assert_eq!(favs(&la), vec!["gog:1", "steam:620"]);
        assert_eq!(favs(&lb), vec!["gog:1", "steam:620"]);
        assert_eq!(lb["profile"]["name"], "Ana");
        // Play time adds up: 100 here + 50 there, on both. Last played: the latest.
        assert_eq!(la["stats"]["steam:620"]["minutes"], 150);
        assert_eq!(lb["stats"]["steam:620"]["minutes"], 150);
        assert_eq!(la["stats"]["steam:620"]["lastPlayed"], 2000);
        assert_eq!(read_json(&sb.config.join("input/steam%3A620.json")), json!({"preset": "fps"}));

        // Play more on B, unfavorite on A: both reach the other, nothing counted twice.
        let mut lb2 = lb.clone(); lb2["stats"]["steam:620"]["minutes"] = json!(160);
        write(&sb.data.join("library.json"), lb2);
        std::thread::sleep(std::time::Duration::from_millis(5));
        let mut la2 = la.clone(); la2["favorites"] = json!(["gog:1"]);
        write(&sa.data.join("library.json"), la2);
        exchange(&mut a, "A", &sa, &mut b, "B", &sb);
        hearth_applies(&sa); hearth_applies(&sb);
        exchange(&mut a, "A", &sa, &mut b, "B", &sb);   // a round with nothing new changes nothing
        hearth_applies(&sa); hearth_applies(&sb);
        let la = read_json(&sa.data.join("library.json"));
        let lb = read_json(&sb.data.join("library.json"));
        assert_eq!(la["stats"]["steam:620"]["minutes"], 160);
        assert_eq!(lb["stats"]["steam:620"]["minutes"], 160);
        assert_eq!(favs(&la), vec!["gog:1"]);
        assert_eq!(favs(&lb), vec!["gog:1"]);
    }

    #[test]
    fn newest_change_wins() {
        let root = tempfile::tempdir().unwrap();
        let (sa, sb) = (device(root.path(), "a"), device(root.path(), "b"));
        write(&sa.data.join("library.json"), json!({"profile": {"name": "Old"}}));
        let mut a = SyncState::default();
        a.scan("A", &sa);
        let mut b = SyncState::default();
        write(&sb.data.join("library.json"), json!({"profile": {"name": "New"}}));
        std::thread::sleep(std::time::Duration::from_millis(5));
        b.scan("B", &sb);
        exchange(&mut a, "A", &sa, &mut b, "B", &sb);
        hearth_applies(&sa); hearth_applies(&sb);
        assert_eq!(read_json(&sa.data.join("library.json"))["profile"]["name"], "New");
        assert_eq!(read_json(&sb.data.join("library.json"))["profile"]["name"], "New");
    }

    #[test]
    fn keys_from_the_network_are_checked() {
        assert!(valid_key("p/main/fav/steam:620"));
        assert!(valid_key("g/input/steam%3A620.json"));
        assert!(!valid_key("g/input/../../.bashrc"));
        assert!(!valid_key("g/input/x.sh"));
        assert!(!valid_key("p/main/pic/../../x"));
        assert!(!valid_key("p/../fav/x"));
        assert!(!valid_key("accounts/xbox"));
        let mut s = SyncState::default();
        let (c, _) = s.merge("A", &json!({"fields": {"g/input/../x": {"v": 1, "t": 1, "d": "B"}, "accounts/x": {"v": 1, "t": 1, "d": "B"}}}));
        assert!(c.is_empty());
    }

    #[test]
    fn base64_both_ways() {
        for n in 0..10 { let d: Vec<u8> = (0..n * 37).map(|i| (i * 7) as u8).collect(); assert_eq!(unb64(&b64(&d)).unwrap(), d); }
        assert_eq!(b64(b"Man"), "TWFu");
        assert_eq!(b64(b"Ma"), "TWE=");
    }

    #[test]
    fn inbox_adds_up_minutes_and_replaces_settings() {
        let mut cur = json!({});
        merge_inbox(&mut cur, &json!({"profiles": {"main": {"set": {"fav/x": true}, "add": {"g": 5}, "last": {"g": 10}}}}));
        merge_inbox(&mut cur, &json!({"profiles": {"main": {"set": {"fav/x": null}, "add": {"g": 7}, "last": {"g": 3}}}, "gogInstalls": {"1": "/g"}}));
        assert_eq!(cur["profiles"]["main"]["set"]["fav/x"], Value::Null);
        assert_eq!(cur["profiles"]["main"]["add"]["g"], 12);
        assert_eq!(cur["profiles"]["main"]["last"]["g"], 10);
        assert_eq!(cur["gogInstalls"]["1"], "/g");
    }
}
