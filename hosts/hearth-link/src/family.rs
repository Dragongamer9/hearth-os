//! Your devices, family members' devices and friends' devices.
//!
//! Pairing asks "My device", "A family member's device" or "A friend's device" (pair.rs). A device is yours only when
//! both sides said My device; a friend's when either side said so; otherwise a family member's (pair.rs relation:
//! the side that picked "A family member's device" helps on the other). Your own devices then share who else is yours
//! and the family members' devices they help on (family.sync), so pairing a new device with any one of them pairs it
//! with all of them: no code on each. A device of yours (or a family member's) that was removed is remembered
//! (state.rs `Removed`) and the others drop it too; pairing with it again later wins over the removal.
//!
//! Safety: a device is only ever added on the word of one of your own devices (an already paired, already trusted
//! peer, over Hearth Link's end-to-end encrypted connection, which only paired devices can open). A friend's or a
//! family member's device is never listened to: family.sync from it is refused, and it's never told about your other
//! devices, except that a family member's device hears the names (and the relay) of your devices that talk to it
//! (family.intro), and only from the device of yours it paired with with a code. Nobody can add a device by showing
//! up and asking.
//!
//! A friend's device can: be seen online, send you a game copy (you accept it, as before), copy a game you pick
//! for it, and unpair. It can't: sync settings or the library, see or back up saves, use Hearth Storage (shares,
//! archive, backups, the download cache, updates), see your status, hand off a game, use your controllers, start
//! updates, use your AI, run benchmarks, reach Decky, or learn about your other devices.
//!
//! A family member's device (your dad's PC): everything a friend's can, and it shows in Your devices on every device
//! of yours with its status (its own switch per device of yours: on by default; yours toward it: off by default),
//! you can wake it and use its desktop (Remote desktop: Sunshine and Moonlight, docs/REMOTE-DESKTOP.md; its
//! Moonlight PIN can be typed on your device). On the family member's own device, per device of yours, switches
//! decide whether it also shares its game library (favorites, hidden games, collections, play time; never
//! settings), backs up its saves to your Hearth Storage, uses your Hearth Storage, and backs up the PC to it: all off
//! by default, changed only there (family.shares tells your devices; what counts is checked on each side before
//! anything runs, from the family member's own choice). Never: your settings onto it, handing off games, your
//! controllers, starting updates, your AI, Decky, your sign-ins.
//!
//! A device on a Hearth OS from before this answers family.sync with "Unknown request": nothing is shared with it and
//! it stays paired as before. One from before family devices reads a family member's device as a friend's.

use crate::node::Node;
use crate::state::{Peer, Peers, Rel, Removed, Shares};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;

/// How many removals are kept (the oldest go first).
const KEEP_REMOVED: usize = 200;
/// How often your devices compare lists (and right after pairing).
const EVERY: Duration = Duration::from_secs(300);
/// How long one device has to answer.
const ASK_WAIT: Duration = Duration::from_secs(20);

/// Requests a friend's device may make. Everything else is only for your own devices.
pub const FRIEND_OPS: &[&str] = &["hello", "ping", "unpair", "games", "manifest", "hashes", "chunks", "offer"];
/// Requests between your devices and a family member's device on top of a friend's: what the family member's
/// device turns on for you (family.shares), the names of your devices (family.intro), whose it is (family.relation),
/// its status (status.get, per its switch) and its desktop (family.remote). Each is checked again in its handler for
/// which side may make it.
pub const FAMILY_OPS: &[&str] = &["family.intro", "family.shares", "family.relation", "family.remote", "family.remote.pin", "status.get"];
/// The family member's switches and the requests each one lets through (on the family member's device and on yours).
pub const SWITCHES: &[&str] = &["library", "saves", "storage", "backup"];

pub fn friend_may(op: &str) -> bool { FRIEND_OPS.contains(&op) }

/// Which of the family member's switches a request belongs to (any one of them lets it through).
pub fn switch_of(op: &str) -> &'static [&'static str] {
    match op {
        "sync" => &["library"],
        // Finding your Hearth Storage: for any of them.
        "storage.info" => &["storage", "backup", "saves"],
        o if o.starts_with("saves.") => &["saves"],
        o if o.starts_with("backup.") => &["backup"],
        o if o.starts_with("storage.") || o.starts_with("archive.") || o.starts_with("cache.") || o.starts_with("updates.") => &["storage"],
        _ => &[],
    }
}

/// Which switch one of Hearth Storage's own things (archive:, share:, backup:, saves) belongs to, for copies.
pub fn switch_of_item(id: &str) -> &'static str {
    if id.starts_with("backup:") { "backup" } else if id.starts_with("savedev:") || id.starts_with("savesrv:") { "saves" } else { "storage" }
}

/// A request `p` sent this device may be answered: checked before anything runs.
pub fn allowed(p: &Peer, op: &str) -> bool {
    // Family and friends with Moonlight: between any paired devices; each request checks the PC's own switch and
    // list itself (moonlight.rs).
    if crate::moonlight::is_op(op) { return true; }
    match p.rel() {
        Rel::Mine => true,
        Rel::Friend => friend_may(op),
        Rel::Family => {
            if friend_may(op) { return true; }
            if !FAMILY_OPS.contains(&op) && switch_of(op).is_empty() { return false; }
            match op {
                "status.get" => p.status_shared(),
                // Only toward the family member's device, from the people who help on it.
                "family.intro" | "family.remote" | "family.remote.pin" => p.helps_me(),
                "family.shares" | "family.relation" => true,
                _ => switch_of(op).iter().any(|w| p.lets(w)),
            }
        }
    }
}

/// This device may ask `p` for `op` (the family member's own switches stop it before anything goes out).
pub fn may_ask(p: &Peer, op: &str) -> bool {
    if crate::moonlight::is_op(op) { return true; }
    match p.rel() {
        Rel::Mine => true,
        // Saying it's a friend's device now (after this one made it one).
        Rel::Friend => friend_may(op) || op == "family.relation",
        Rel::Family => {
            if friend_may(op) { return true; }
            match op {
                "status.get" | "family.shares" | "family.relation" => true,
                "family.intro" | "family.remote" | "family.remote.pin" => p.i_help(),
                _ => switch_of(op).iter().any(|w| p.lets(w)),
            }
        }
    }
}

/// The words for a request that isn't let through.
pub fn refusal(p: Option<&Peer>, op: &str) -> String {
    match p.map(|p| p.rel()) {
        Some(Rel::Family) if !switch_of(op).is_empty() => {
            let what = match switch_of(op)[0] { "library" => "Share my game library", "saves" => "Back up my saves", "backup" => "Back up this PC", _ => "Use Hearth Storage" };
            format!("That's off: only the family member can turn on \"{what}\", on their own device")
        }
        Some(Rel::Family) => "A family member's device doesn't share that".into(),
        _ => "That's only for your own devices".into(),
    }
}

/// What this device tells a family device about its own switches in hello and family.shares: what it lets that
/// device have (its status, and on the family member's device the four switches).
pub fn shares_for(p: &Peer) -> Value {
    let on = |w: &str| p.helps_me() && p.shares.get(w);
    json!({"status": p.status_shared(), "library": on("library"), "saves": on("saves"), "storage": on("storage"), "backup": on("backup")})
}

/// What a family device said it lets this one have.
pub fn take_shares(v: &Value) -> Shares {
    Shares { status: Some(v["status"] == true), library: v["library"] == true, saves: v["saves"] == true, storage: v["storage"] == true, backup: v["backup"] == true }
}

/// Changes a switch on this device for that device: `{status: bool}` toward any family device; the four others
/// only on the family member's own device (never from the side that helps on it).
pub fn set_shares(p: &mut Peer, a: &Value) -> Result<bool, String> {
    if !p.family() { return Err("Only a family member's device has these switches".into()); }
    let mut changed = false;
    if let Some(b) = a["status"].as_bool() { if p.shares.status != Some(b) { p.shares.status = Some(b); changed = true; } }
    for w in SWITCHES {
        let Some(b) = a[*w].as_bool() else { continue };
        if !p.helps_me() { return Err("Only the family member can change that, on their own device".into()); }
        let cur = match *w { "library" => &mut p.shares.library, "saves" => &mut p.shares.saves, "storage" => &mut p.shares.storage, _ => &mut p.shares.backup };
        if *cur != b { *cur = b; changed = true; }
    }
    Ok(changed)
}

pub fn remember_removed(list: &mut Vec<Removed>, id: &str, at: u64) {
    match list.iter_mut().find(|r| r.id == id) {
        Some(r) => r.at = r.at.max(at),
        None => list.push(Removed { id: id.to_string(), at }),
    }
    list.sort_by_key(|r| std::cmp::Reverse(r.at));
    list.truncate(KEEP_REMOVED);
}

/// What this device tells one of your devices: your other devices and the family members' devices you help on
/// (never friends', never the one asked) and the removals it knows.
pub fn payload(peers: &Peers, to: &str) -> Value {
    let one = |p: &Peer| json!({"id": p.id, "name": p.name, "pairedAt": p.paired_at, "addrs": p.addrs,
                                "relay": p.signed_in.get("relay").cloned().unwrap_or(Value::Null), "macs": if p.family() { json!(p.macs) } else { json!([]) }});
    let devices: Vec<Value> = peers.mine().filter(|p| p.id != to).map(one).collect();
    // Family members' devices in their own list: a Hearth from before them reads only "devices", so it never takes
    // one for yours.
    let family: Vec<Value> = peers.list.iter().filter(|p| p.i_help() && p.id != to).map(one).collect();
    json!({"op": "family.sync", "v": 2, "devices": devices, "family": family, "removed": peers.removed})
}

/// What the device of yours that paired with a family member's device tells it: the names of your devices that
/// talk to it (their ids, names and relay: nothing else) and which of them were removed.
pub fn intro_payload(peers: &Peers, to: &str) -> Value {
    let devices: Vec<Value> = peers.mine().filter(|p| p.id != to).map(|p| json!({
        "id": p.id, "name": p.name, "relay": p.signed_in.get("relay").cloned().unwrap_or(Value::Null),
    })).collect();
    let mine: Vec<&str> = peers.mine().map(|p| p.id.as_str()).collect();
    // Removals of your own devices only (never of family devices or friends'), so it drops them too.
    let removed: Vec<&Removed> = peers.removed.iter().filter(|r| !mine.contains(&r.id.as_str())).take(64).collect();
    json!({"op": "family.intro", "v": 1, "devices": devices, "removed": removed})
}

/// On the family member's device: takes the names of the devices of the person who helps on it, only from the device
/// of theirs it paired with with a code. Adds them as theirs (switches off, status shared as by default) and drops
/// the ones that device says were removed, if they came from it.
pub fn merge_intro(peers: &mut Peers, me: &str, from: &str, v: &Value, now: u64) -> Result<Changes, String> {
    match peers.get(from) {
        Some(p) if p.helps_me() && p.introduced_by.is_none() => {}
        _ => return Err("Only the device that paired with this one can say that".into()),
    }
    let mut ch = Changes::default();
    for r in v["removed"].as_array().into_iter().flatten().take(64) {
        let Some(id) = r["id"].as_str().filter(|i| valid_id(i)) else { continue };
        if id == me || id == from { continue; }
        let theirs = peers.get(id).map(|p| p.helps_me() && p.introduced_by.as_deref() == Some(from)).unwrap_or(false);
        if theirs { peers.list.retain(|p| p.id != id); ch.dropped.push(id.to_string()); }
    }
    for d in v["devices"].as_array().into_iter().flatten().take(32) {
        let Some(id) = d["id"].as_str().filter(|i| valid_id(i)) else { continue };
        if id == me || peers.is_paired(id) { continue; }
        let name: String = d["name"].as_str().unwrap_or("Hearth").chars().filter(|c| !c.is_control()).take(40).collect();
        let mut signed_in = json!({});
        if let Some(r) = d["relay"].as_str().filter(|r| crate::state::valid_relay_url(r)) { signed_in["relay"] = json!(r); }
        let mut p = Peer { id: id.to_string(), name, paired_at: now, signed_in, introduced_by: Some(from.to_string()), ..Default::default() };
        p.set_rel(Rel::Family, "helper");
        peers.list.push(p);
        ch.added.push(id.to_string());
    }
    Ok(ch)
}

/// What changed after taking another device's list.
#[derive(Debug, Default, PartialEq)]
pub struct Changes { pub added: Vec<String>, pub dropped: Vec<String>,
                     /// Yours here, made a family member's device on another of yours: it is here too (only ever down).
                     pub demoted: Vec<String> }

fn valid_id(id: &str) -> bool { id.len() == 64 && id.chars().all(|c| c.is_ascii_hexdigit()) }

/// Takes the list `from` (one of your devices) sent. `me` is this device. Friends are never added, changed or
/// dropped by it, and only your own devices are listened to.
pub fn merge(peers: &mut Peers, me: &str, from: &str, v: &Value, now: u64) -> Result<Changes, String> {
    if !peers.is_mine(from) { return Err("Only your own devices share their lists".into()); }
    let mut ch = Changes::default();
    // Removals first: one of yours removed after we paired with it is dropped here too.
    for r in v["removed"].as_array().into_iter().flatten() {
        let Some(id) = r["id"].as_str().filter(|i| valid_id(i)) else { continue };
        let at = r["at"].as_u64().unwrap_or(0).min(now);
        if id == me || id == from || at == 0 { continue; }
        // Friends' devices and the devices of someone who helps on this one are never dropped by it.
        match peers.get(id).map(|p| (!(p.mine() || p.i_help()), p.paired_at)) {
            Some((true, _)) => continue,
            Some((_, paired_at)) if paired_at > at => continue, // paired with it again since
            Some(_) => { peers.list.retain(|p| p.id != id); ch.dropped.push(id.to_string()); remember_removed(&mut peers.removed, id, at); }
            None => remember_removed(&mut peers.removed, id, at),
        }
    }
    let yours = v["devices"].as_array().into_iter().flatten().take(64).map(|d| (d, false));
    let family = v["family"].as_array().into_iter().flatten().take(64).map(|d| (d, true));
    for (d, is_family) in yours.chain(family) {
        let Some(id) = d["id"].as_str().filter(|i| valid_id(i)) else { continue };
        if id == me { continue; }
        if let Some(p) = peers.get_mut(id) {
            // One of yours made it a family member's device: here too, unless it was paired again since.
            if is_family && p.mine() && d["pairedAt"].as_u64().unwrap_or(0) >= p.paired_at {
                p.set_rel(Rel::Family, "member");
                ch.demoted.push(id.to_string());
            }
            continue;
        }
        let paired_at = d["pairedAt"].as_u64().unwrap_or(0);
        if peers.removed.iter().any(|r| r.id == id && r.at >= paired_at) { continue; }
        let name: String = d["name"].as_str().unwrap_or("Hearth").chars().filter(|c| !c.is_control()).take(40).collect();
        let addrs: Vec<String> = d["addrs"].as_array().into_iter().flatten()
            .filter_map(|a| a.as_str().filter(|s| s.parse::<std::net::SocketAddr>().is_ok()).map(str::to_string)).take(4).collect();
        let mut signed_in = json!({});
        if let Some(r) = d["relay"].as_str().filter(|r| crate::state::valid_relay_url(r)) { signed_in["relay"] = json!(r); }
        // last_seen 0: not reached yet. Until it is, "not paired" from it means it hasn't heard yet (node.rs).
        let mut p = Peer { id: id.to_string(), name, paired_at: now, last_seen: 0, addrs, signed_in, introduced_by: Some(from.to_string()), ..Default::default() };
        if is_family {
            // A family member's device one of yours helps on: this one helps on it too (its switches start off;
            // what it shares is what it says to this device).
            p.set_rel(Rel::Family, "member");
            p.macs = d["macs"].as_array().into_iter().flatten().filter_map(|m| m.as_str().filter(|m| crate::storage::valid_mac(m)).map(str::to_string)).take(4).collect();
        } else {
            p.set_rel(Rel::Mine, "");
        }
        peers.list.push(p);
        ch.added.push(id.to_string());
    }
    Ok(ch)
}

impl Node {
    /// One of your devices sent its list: take it and answer with ours. What changed is passed on at once.
    pub async fn family_answer(self: Arc<Self>, peer: &str, req: &Value) -> Result<Value, String> {
        let (ch, out) = {
            let mut ps = crate::lock(&self.peers);
            let ch = merge(&mut ps, &self.me, peer, req, crate::now_ms())?;
            if ch != Changes::default() { let _ = ps.save(); }
            (ch, payload(&ps, peer))
        };
        if self.family_log(peer, &ch).await {
            let n = self.clone(); let skip = peer.to_string();
            tokio::spawn(async move { n.family_round(Some(skip)).await; });
        }
        Ok(out)
    }

    /// Compare lists with one of your devices; what changed here.
    async fn family_sync_with(self: &Arc<Self>, peer: &str) -> Result<Changes, String> {
        let req = { let ps = crate::lock(&self.peers); if !ps.is_mine(peer) { return Ok(Changes::default()); } payload(&ps, peer) };
        let resp = match tokio::time::timeout(ASK_WAIT, self.request(peer, &req)).await {
            Ok(Ok(v)) => v,
            Ok(Err(e)) if e.contains("Unknown request") => return Ok(Changes::default()), // a Hearth OS from before this
            Ok(Err(e)) => return Err(e),
            Err(_) => return Err("didn't answer".into()),
        };
        if let Some(e) = resp["error"].as_str() { return if e.contains("Unknown request") { Ok(Changes::default()) } else { Err(e.to_string()) }; }
        let mut ps = crate::lock(&self.peers);
        let ch = merge(&mut ps, &self.me, peer, &resp, crate::now_ms())?;
        if ch != Changes::default() { let _ = ps.save(); }
        Ok(ch)
    }

    /// Says what changed in the journal (and tells a removed device it's unpaired, if it answers); true when
    /// something did.
    async fn family_log(self: &Arc<Self>, from: &str, ch: &Changes) -> bool {
        let name = |id: &str| crate::lock(&self.peers).get(id).map(|p| p.name.clone()).unwrap_or_else(|| id.get(..8).unwrap_or("").to_string());
        for id in &ch.added { eprintln!("hearth-link: family: {} told this device about {} ({id}): paired", name(from), name(id)); }
        for id in &ch.dropped {
            eprintln!("hearth-link: family: {} said {id} was removed: removed here too", name(from));
            let _ = tokio::time::timeout(Duration::from_secs(8), self.request(id, &json!({"op": "unpair"}))).await;
            self.drop_conn(id).await;
        }
        for id in &ch.demoted {
            eprintln!("hearth-link: family: {} made {} a family member's device: here too", name(from), name(id));
            let _ = tokio::time::timeout(Duration::from_secs(8), self.request(id, &json!({"op": "family.relation", "as": "family"}))).await;
        }
        !ch.added.is_empty() || !ch.dropped.is_empty() || !ch.demoted.is_empty()
    }

    /// Compare lists with every device of yours (in parallel); `skip`: the one that just told us. When a round
    /// changes something it runs again (a few times at most), so news reaches all of them.
    pub async fn family_round(self: &Arc<Self>, skip: Option<String>) {
        let mut skip = skip;
        for _ in 0..3 {
            let ids: Vec<String> = crate::lock(&self.peers).mine().map(|p| p.id.clone()).filter(|i| Some(i) != skip.as_ref()).collect();
            let mut set = tokio::task::JoinSet::new();
            for id in ids {
                let n = self.clone();
                set.spawn(async move { let r = n.family_sync_with(&id).await; (id, r) });
            }
            let mut changed = false;
            while let Some(r) = set.join_next().await {
                match r {
                    Ok((id, Ok(ch))) => { if self.family_log(&id, &ch).await { changed = true; } }
                    Ok((id, Err(e))) => eprintln!("hearth-link: family: with {}: {e}", id.get(..8).unwrap_or("")),
                    Err(_) => {}
                }
            }
            if !changed { break; }
            skip = None;
        }
        self.family_intro_all().await;
    }

    /// The family members' devices this one paired with with a code hear the names of your devices (family.intro).
    pub async fn family_intro_all(self: &Arc<Self>) {
        let targets: Vec<String> = crate::lock(&self.peers).list.iter().filter(|p| p.i_help() && p.introduced_by.is_none()).map(|p| p.id.clone()).collect();
        for id in targets {
            let req = intro_payload(&crate::lock(&self.peers), &id);
            match tokio::time::timeout(ASK_WAIT, self.request(&id, &req)).await {
                Ok(Ok(v)) if v["error"].is_null() => {}
                Ok(Ok(v)) => { let e = v["error"].as_str().unwrap_or(""); if !e.contains("Unknown request") { eprintln!("hearth-link: family: telling {}: {e}", id.get(..8).unwrap_or("")); } }
                Ok(Err(e)) if e.contains("Unknown request") => {} // a Hearth OS from before family devices
                Ok(Err(e)) => eprintln!("hearth-link: family: telling {}: {e}", id.get(..8).unwrap_or("")),
                Err(_) => {}
            }
        }
    }

    /// Requests about family devices (family.rs allowed() let them through already).
    pub async fn family_request(self: Arc<Self>, peer: &str, op: &str, req: &Value) -> Result<Value, String> {
        match op {
            "family.intro" => {
                let ch = {
                    let mut ps = crate::lock(&self.peers);
                    let ch = merge_intro(&mut ps, &self.me, peer, req, crate::now_ms())?;
                    if ch != Changes::default() { ps.save()?; }
                    ch
                };
                let name = |id: &str| crate::lock(&self.peers).get(id).map(|p| p.name.clone()).unwrap_or_else(|| id.get(..8).unwrap_or("").to_string());
                for id in &ch.added { eprintln!("hearth-link: family: {} told this device about {} ({id}): it can help on this one too", name(peer), name(id)); }
                for id in &ch.dropped { eprintln!("hearth-link: family: {} said {id} was removed: removed here too", name(peer)); self.drop_conn(id).await; }
                // Never anything back: a family member's device's own list isn't asked for.
                Ok(json!({"ok": true}))
            }
            "family.shares" => {
                let mut ps = crate::lock(&self.peers);
                let p = ps.get_mut(peer).ok_or("Not a paired device")?;
                if !p.family() { return Ok(json!({"ok": false})); }
                p.their_shares = take_shares(&req["shares"]);
                ps.save()?;
                Ok(json!({"ok": true}))
            }
            "family.relation" => {
                // The other side changed whose this device is. Only ever toward less (yours › family › friend's):
                // more needs this device's own say (its menu, or pairing again).
                let mut ps = crate::lock(&self.peers);
                let p = ps.get_mut(peer).ok_or("Not a paired device")?;
                let before = p.rel();
                match (before, req["as"].as_str().and_then(Rel::parse)) {
                    (Rel::Mine, Some(Rel::Family)) => p.set_rel(Rel::Family, "helper"),
                    (Rel::Family, Some(Rel::Family)) if p.family_side == "member" => p.family_side = "both".into(),
                    (Rel::Mine | Rel::Family, Some(Rel::Friend)) => p.set_rel(Rel::Friend, ""),
                    _ => return Ok(json!({"ok": false})),
                }
                if before == Rel::Mine { p.introduced_by = None; }
                eprintln!("hearth-link: family: {} is {} now (it said so)", p.name, if p.family() { "a family member's device" } else { "a friend's device" });
                ps.save()?;
                Ok(json!({"ok": true}))
            }
            "family.remote" => {
                // This PC's remote desktop, for the people who help on it: on or not, and its name for Moonlight.
                let v = tokio::task::spawn_blocking(|| run_timed(&remote_helper(), &["status"], 12)).await.map_err(|e| e.to_string())??;
                Ok(remote_view(&v))
            }
            "family.remote.pin" => {
                let pin: String = req["pin"].as_str().unwrap_or("").chars().filter(|c| c.is_ascii_digit()).collect();
                if pin.len() != 4 { return Err("Moonlight's PIN is 4 numbers".into()); }
                let from = crate::lock(&self.peers).get(peer).map(|p| p.name.clone()).unwrap_or_default();
                let name: String = req["name"].as_str().filter(|n| !n.trim().is_empty()).map(str::to_string).unwrap_or(from)
                    .chars().filter(|c| c.is_alphanumeric() || " -'_.".contains(*c)).take(40).collect();
                eprintln!("hearth-link: family: {name}: Moonlight PIN sent to this PC's remote desktop");
                tokio::task::spawn_blocking(move || run_timed(&remote_helper(), &["pin", pin.as_str(), name.as_str()], 20)).await.map_err(|e| e.to_string())?
            }
            _ => Err(format!("Unknown request: {op}")),
        }
    }

    /// The UI's commands about a family member's device: wake it, its remote desktop, Moonlight's PIN for it.
    pub async fn family_command(self: &Arc<Self>, cmd: &str, a: &Value) -> Result<Value, String> {
        let id = a["id"].as_str().filter(|x| !x.is_empty()).ok_or("Missing 'id'")?.to_string();
        let p = crate::lock(&self.peers).get(&id).cloned().ok_or("Not a paired device")?;
        match cmd {
            "peer.wake" => {
                // Its own network cards (a family member's device says them), else what Hearth Storage learned.
                let mut macs = p.macs.clone();
                if macs.is_empty() && p.mine() { macs = self.st().servers.get(&id).and_then(|v| v["macs"].as_array().cloned()).unwrap_or_default().iter().filter_map(|m| m.as_str().map(str::to_string)).collect(); }
                if macs.is_empty() { return Err(format!("This device doesn't know how to wake {} yet: it learns that the next time both are on", p.name)); }
                Ok(json!({"sent": crate::storage::wake(&macs)}))
            }
            "peer.remote" => {
                if !may_ask(&p, "family.remote") { return Err("Remote desktop is for your own devices and the family members' devices you help on".into()); }
                match tokio::time::timeout(Duration::from_secs(20), self.request(&id, &json!({"op": "family.remote"}))).await {
                    Ok(Ok(v)) => { if let Some(e) = v["error"].as_str() { return Err(remote_error(e)); } Ok(v) }
                    Ok(Err(e)) => Err(remote_error(&e)),
                    Err(_) => Err(format!("{} didn't answer", p.name)),
                }
            }
            "peer.remote.pin" => {
                if !may_ask(&p, "family.remote.pin") { return Err("Remote desktop is for your own devices and the family members' devices you help on".into()); }
                let req = json!({"op": "family.remote.pin", "pin": a["pin"], "name": a["name"]});
                match tokio::time::timeout(Duration::from_secs(30), self.request(&id, &req)).await {
                    Ok(Ok(v)) => { if let Some(e) = v["error"].as_str() { return Err(remote_error(e)); } Ok(v) }
                    Ok(Err(e)) => Err(remote_error(&e)),
                    Err(_) => Err(format!("{} didn't answer", p.name)),
                }
            }
            _ => Err(format!("Unknown command: {cmd}")),
        }
    }

    /// Tells a family device what this one lets it have now (after a switch changed here). Best effort: it hears
    /// again in the next hello.
    pub fn family_tell_shares(self: &Arc<Self>, id: &str) {
        let Some(p) = crate::lock(&self.peers).get(id).cloned() else { return };
        if !p.family() { return; }
        let (n, id) = (self.clone(), id.to_string());
        tokio::spawn(async move { let _ = tokio::time::timeout(ASK_WAIT, n.request(&id, &json!({"op": "family.shares", "shares": shares_for(&p)}))).await; });
    }

    /// Tells the other device it's a family member's or a friend's now (it takes only a step down).
    pub fn family_tell_relation(self: &Arc<Self>, id: &str, r: Rel) {
        let (n, id) = (self.clone(), id.to_string());
        tokio::spawn(async move { let _ = tokio::time::timeout(ASK_WAIT, n.request(&id, &json!({"op": "family.relation", "as": r.as_str()}))).await; });
    }

    /// Every few minutes, so a device that was off catches up.
    pub fn family_start(self: &Arc<Self>) {
        let n = self.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(EVERY).await;
                if crate::game_running() { continue; }
                n.family_round(None).await;
            }
        });
    }

    /// Right after pairing with one of your devices: everyone of yours hears about the new one, and it about them.
    pub fn family_after_pairing(self: &Arc<Self>, peer: &str) {
        // Yours, or a family member's device you help on (your other devices hear of it, and it of them).
        if !crate::lock(&self.peers).get(peer).map(|p| p.mine() || p.i_help()).unwrap_or(false) { return; }
        let n = self.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(2)).await;
            n.family_round(None).await;
        });
    }
}

/// Mantel's remote desktop helper (Settings › Remote play › Use this PC from anywhere).
pub const REMOTE_HELPER: &str = "/usr/share/nimbus/desktop/shell/hearth-remote-desktop";
pub(crate) fn remote_helper() -> String { std::env::var("HEARTH_REMOTE_DESKTOP").unwrap_or_else(|_| REMOTE_HELPER.to_string()) }

/// Runs a helper (python3 PATH ARGS...) and reads its last line as JSON, stopped after `secs`.
pub fn run_timed(path: &str, args: &[&str], secs: u64) -> Result<Value, String> {
    use std::process::{Command, Stdio};
    if !std::path::Path::new(path).exists() { return Err("Remote desktop comes with the next Hearth OS update on that device".into()); }
    let mut child = Command::new("python3").arg(path).args(args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null())
        .spawn().map_err(|e| e.to_string())?;
    let t = std::time::Instant::now();
    loop {
        match child.try_wait().map_err(|e| e.to_string())? {
            Some(_) => break,
            None if t.elapsed() > Duration::from_secs(secs) => { let _ = child.kill(); let _ = child.wait(); return Err("Remote desktop took too long to answer".into()); }
            None => std::thread::sleep(Duration::from_millis(50)),
        }
    }
    let out = child.wait_with_output().map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&out.stdout);
    let line = text.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("");
    if line.starts_with("error: ") || !out.status.success() { return Err(line.trim_start_matches("error: ").to_string()); }
    serde_json::from_str(line).map_err(|e| e.to_string())
}

/// What another device needs of this PC's remote desktop: on, running, its name and address for Moonlight. No
/// logins, no paired devices, no other Tailscale devices.
pub fn remote_view(v: &Value) -> Value {
    let a = &v["address"];
    let host = [&a["name"], &a["dns"], &a["ip"]].iter().filter_map(|x| x.as_str()).find(|x| !x.is_empty()).unwrap_or("").to_string();
    json!({"on": v["on"] == true, "running": v["running"] == true, "installed": v["installed"] != false,
           "tailscale": v["tailscale"]["running"] == true, "name": a["name"], "ip": a["ip"], "host": host,
           "streaming": v["stream"]["connected"] == true})
}

fn remote_error(e: &str) -> String {
    if e.contains("Unknown request") { "That device needs a Hearth OS update for remote desktop through Hearth Link".into() } else { e.trim_start_matches("connection: ").to_string() }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn id(c: char) -> String { std::iter::repeat(c).take(64).collect() }
    fn peers(d: &std::path::Path) -> Peers { Peers::load(d) }

    #[test]
    fn a_new_device_learns_your_others_and_never_friends() {
        let d = tempfile::tempdir().unwrap();
        // B (this device) has A (yours, just paired) and F (a friend).
        let (a, c, f, me) = (id('a'), id('c'), id('f'), id('b'));
        let mut b = peers(d.path());
        b.add_paired(&a, "Ally", false);
        b.add_paired(&f, "Sam's Deck", true);
        // A says: C is mine too.
        let msg = json!({"devices": [{"id": c, "name": "Laptop", "pairedAt": 5, "addrs": ["192.168.1.9:47740", "nope"], "relay": "https://relay.example.com"}], "removed": []});
        let ch = merge(&mut b, &me, &a, &msg, 1000).unwrap();
        assert_eq!(ch.added, vec![c.clone()]);
        let p = b.get(&c).unwrap();
        assert!(p.mine());
        assert_eq!(p.introduced_by.as_deref(), Some(a.as_str()));
        assert_eq!(p.last_seen, 0);
        assert_eq!(p.addrs, vec!["192.168.1.9:47740".to_string()]);
        assert_eq!(p.signed_in["relay"], "https://relay.example.com");
        // What B tells A: C, never the friend, never A itself.
        let out = payload(&b, &a);
        let ids: Vec<&str> = out["devices"].as_array().unwrap().iter().map(|x| x["id"].as_str().unwrap()).collect();
        assert_eq!(ids, vec![c.as_str()]);
    }

    #[test]
    fn a_friend_is_never_listened_to() {
        let d = tempfile::tempdir().unwrap();
        let (a, f, x, me) = (id('a'), id('f'), id('e'), id('b'));
        let mut b = peers(d.path());
        b.add_paired(&a, "Ally", false);
        b.add_paired(&f, "Sam's Deck", true);
        let msg = json!({"devices": [{"id": x, "name": "Stranger", "pairedAt": 5}], "removed": [{"id": a, "at": 999}]});
        assert!(merge(&mut b, &me, &f, &msg, 1000).is_err());
        assert!(!b.is_paired(&x));
        assert!(b.is_paired(&a));
        // Nor is a device that isn't paired at all.
        assert!(merge(&mut b, &me, &x, &msg, 1000).is_err());
    }

    #[test]
    fn removed_everywhere_and_not_added_back() {
        let d = tempfile::tempdir().unwrap();
        let (a, c, me) = (id('a'), id('c'), id('b'));
        let mut b = peers(d.path());
        b.add_paired(&a, "Ally", false);
        b.add_paired(&c, "Laptop", false);
        let paired_c = b.get(&c).unwrap().paired_at;
        // A removed C after B paired with it: B drops it and remembers.
        let msg = json!({"devices": [], "removed": [{"id": c, "at": paired_c + 10}]});
        let ch = merge(&mut b, &me, &a, &msg, paired_c + 20).unwrap();
        assert_eq!(ch.dropped, vec![c.clone()]);
        assert!(!b.is_paired(&c));
        assert_eq!(b.removed.len(), 1);
        // An older list that still has C doesn't add it back.
        let old = json!({"devices": [{"id": c, "name": "Laptop", "pairedAt": paired_c}], "removed": []});
        assert!(merge(&mut b, &me, &a, &old, paired_c + 30).unwrap().added.is_empty());
        // Paired with it again later (newer pairedAt): it's yours again.
        let again = json!({"devices": [{"id": c, "name": "Laptop", "pairedAt": paired_c + 100}], "removed": []});
        assert_eq!(merge(&mut b, &me, &a, &again, paired_c + 200).unwrap().added, vec![c.clone()]);
    }

    #[test]
    fn a_removal_older_than_pairing_again_is_ignored_and_friends_stay() {
        let d = tempfile::tempdir().unwrap();
        let (a, c, f, me) = (id('a'), id('c'), id('f'), id('b'));
        let mut b = peers(d.path());
        b.add_paired(&a, "Ally", false);
        b.add_paired(&c, "Laptop", false);
        b.add_paired(&f, "Sam's Deck", true);
        let msg = json!({"removed": [{"id": c, "at": 1}, {"id": f, "at": u64::MAX}, {"id": me, "at": 5}, {"id": "short", "at": 5}]});
        let ch = merge(&mut b, &me, &a, &msg, crate::now_ms()).unwrap();
        assert!(ch.dropped.is_empty());
        assert!(b.is_paired(&c) && b.is_paired(&f));
    }

    #[test]
    fn friends_only_get_what_makes_sense_between_two_people() {
        for op in ["games", "manifest", "hashes", "chunks", "offer", "hello", "ping", "unpair"] { assert!(friend_may(op), "{op}"); }
        for op in ["sync", "family.sync", "family.intro", "family.remote", "family.remote.pin", "family.shares", "saves.list", "status.get", "storage.read", "updates.read", "archive.list", "backup.put", "cache.get",
                   "ai.chat", "update.start", "handoff.offer", "input.list", "bench.run", "decky.list", "device.report", "devices.list"] {
            assert!(!friend_may(op), "{op}");
        }
    }

    fn one(w: &str, b: bool) -> Value { let mut m = serde_json::Map::new(); m.insert(w.to_string(), json!(b)); Value::Object(m) }

    /// A family member's device as this device has it: `side` "member" (this one helps on it) or "helper".
    fn fam(ps: &mut Peers, id: &str, name: &str, side: &str) {
        ps.add_paired_as(id, name, Rel::Family, side);
    }

    const PERSONAL: &[&str] = &["sync", "family.sync", "saves.list", "saves.offer", "storage.read", "storage.ls", "updates.read", "archive.list",
        "backup.offer", "cache.get", "ai.chat", "update.start", "handoff.offer", "input.list", "bench.run", "decky.list", "device.report", "devices.list", "tv.send"];

    #[test]
    fn a_family_device_sees_nothing_personal() {
        let d = tempfile::tempdir().unwrap();
        let mut ps = peers(d.path());
        fam(&mut ps, &id('d'), "Dad's PC", "member");
        let p = ps.get(&id('d')).unwrap().clone();
        assert!(p.family() && p.i_help() && !p.helps_me() && !p.mine());
        // Everything a friend's device may, nothing personal, and not your status unless you turn it on for it.
        for op in FRIEND_OPS { assert!(allowed(&p, op), "{op}"); }
        for op in PERSONAL.iter().chain(&["status.get", "family.intro", "family.remote", "family.remote.pin"]) { assert!(!allowed(&p, op), "{op}"); }
        // Its own say (what it said it shares) never opens your settings, AI, Decky or updates.
        let mut q = p.clone();
        q.their_shares = Shares { status: Some(true), library: true, saves: true, storage: true, backup: true };
        for op in ["family.sync", "ai.chat", "update.start", "handoff.offer", "input.list", "decky.list", "device.report", "tv.send"] { assert!(!allowed(&q, op), "{op}"); }
        // This side may ask it for its status and its desktop, nothing personal of its own.
        for op in ["status.get", "family.remote", "family.remote.pin", "family.intro"] { assert!(may_ask(&p, op), "{op}"); }
        for op in ["sync", "saves.info", "backup.offer", "storage.ls", "ai.chat"] { assert!(!may_ask(&p, op), "{op}"); }
        // Your status toward it: off until you turn it on for that device.
        assert!(!p.status_shared());
        let mut r = p.clone();
        set_shares(&mut r, &json!({"status": true})).unwrap();
        assert!(r.status_shared() && allowed(&r, "status.get"));
        // The four switches are the family member's only.
        for w in SWITCHES { assert!(set_shares(&mut r, &one(w, true)).is_err(), "{w}"); }
        // Neither yours nor a friend's has them.
        ps.add_paired(&id('f'), "Sam's Deck", true);
        assert!(set_shares(&mut ps.get(&id('f')).unwrap().clone(), &json!({"status": true})).is_err());
    }

    #[test]
    fn the_family_members_switches_each_open_only_their_kind() {
        let d = tempfile::tempdir().unwrap();
        let mut ps = peers(d.path());
        // On Dad's PC: Chris's Ally helps on it.
        fam(&mut ps, &id('a'), "Chris's Ally", "helper");
        let mut p = ps.get(&id('a')).unwrap().clone();
        assert!(p.helps_me() && !p.i_help());
        // Status shared by default toward the people who help on it; everything else off.
        assert!(p.status_shared() && allowed(&p, "status.get"));
        assert!(allowed(&p, "family.remote") && allowed(&p, "family.remote.pin") && allowed(&p, "family.intro"));
        for op in PERSONAL { assert!(!allowed(&p, op), "{op}"); assert!(!may_ask(&p, op) || friend_may(op), "{op}"); }
        let groups: &[(&str, &[&str], &[&str])] = &[
            ("library", &["sync"], &["saves.offer", "backup.offer", "storage.ls"]),
            ("saves", &["saves.info", "saves.offer", "saves.done", "storage.info"], &["sync", "backup.offer", "storage.ls"]),
            ("storage", &["storage.info", "storage.ls", "storage.read", "archive.offer", "archive.list", "cache.list", "updates.read"], &["sync", "saves.offer", "backup.offer"]),
            ("backup", &["backup.offer", "storage.info"], &["sync", "saves.offer", "storage.ls", "archive.list"]),
        ];
        for (w, on, off) in groups {
            assert!(set_shares(&mut p, &one(w, true)).unwrap());
            for op in *on { assert!(allowed(&p, op) && may_ask(&p, op), "{w} on: {op}"); }
            for op in *off { assert!(!allowed(&p, op) && !may_ask(&p, op), "{w} on, {op} still off"); }
            for op in ["family.sync", "ai.chat", "decky.list", "update.start", "handoff.offer", "input.list"] { assert!(!allowed(&p, op), "{w} on: {op}"); }
            assert_eq!(p.lets(switch_of_item(match *w { "backup" => "backup:saves", "saves" => "savedev:x", _ => "share:abc" })), *w != "library");
            // Off again: refused at once.
            assert!(set_shares(&mut p, &one(w, false)).unwrap());
            for op in *on { assert!(!allowed(&p, op), "{w} off: {op}"); }
        }
        // Its status can be turned off too, for that device only.
        set_shares(&mut p, &json!({"status": false})).unwrap();
        assert!(!allowed(&p, "status.get"));
        // On your side, what it said is what counts there, the same way.
        let mut mine_side = Peer::default();
        mine_side.set_rel(Rel::Family, "member");
        mine_side.their_shares = take_shares(&json!({"storage": true}));
        assert!(allowed(&mine_side, "storage.ls") && !allowed(&mine_side, "backup.offer") && !allowed(&mine_side, "sync"));
        // Kept in peers.json, through the safe writer.
        let q = ps.get_mut(&id('a')).unwrap();
        set_shares(q, &json!({"backup": true})).unwrap();
        ps.save().unwrap();
        let back = Peers::load(d.path());
        assert!(back.get(&id('a')).unwrap().shares.backup && !back.get(&id('a')).unwrap().shares.storage);
    }

    #[test]
    fn a_family_device_is_shared_among_your_devices_and_never_listened_to() {
        let d = tempfile::tempdir().unwrap();
        let (a, c, dad, f, me) = (id('a'), id('c'), id('d'), id('f'), id('b'));
        let mut b = peers(d.path());
        b.add_paired(&a, "Ally", false);
        b.add_paired(&f, "Sam's Deck", true);
        // A says: C is mine, and Dad's PC is a family member's device I help on.
        let msg = json!({"devices": [{"id": c, "name": "Laptop", "pairedAt": 5}],
                         "family": [{"id": dad, "name": "Dad's PC", "pairedAt": 5, "macs": ["aa:bb:cc:00:11:22", "nope"]}], "removed": []});
        let ch = merge(&mut b, &me, &a, &msg, 1000).unwrap();
        assert_eq!(ch.added, vec![c.clone(), dad.clone()]);
        let p = b.get(&dad).unwrap();
        assert!(p.family() && p.i_help() && !p.mine() && p.friend, "an older Hearth reads it as a friend's");
        assert_eq!(p.macs, vec!["aa:bb:cc:00:11:22".to_string()]);
        assert!(!p.status_shared());
        // B tells A about it in the family list only (an older Hearth reads "devices" only: never takes it for yours).
        let out = payload(&b, &a);
        let ids = |k: &str| out[k].as_array().unwrap().iter().map(|x| x["id"].as_str().unwrap().to_string()).collect::<Vec<_>>();
        assert_eq!(ids("devices"), vec![c.clone()]);
        assert_eq!(ids("family"), vec![dad.clone()]);
        // Dad's PC is never told your list, and its own is never listened to.
        let theirs = json!({"devices": [{"id": id('e'), "name": "Stranger", "pairedAt": 5}], "family": [], "removed": [{"id": a, "at": 999}]});
        assert!(merge(&mut b, &me, &dad, &theirs, 2000).is_err());
        assert!(!b.is_paired(&id('e')) && b.is_paired(&a));
        assert!(!allowed(b.get(&dad).unwrap(), "family.sync"));
        // A friend's device stays out of both lists.
        assert!(!out.to_string().contains(&f));
        // Removed on another of yours: dropped here too.
        let gone = json!({"devices": [], "family": [], "removed": [{"id": dad, "at": 1500}]});
        assert_eq!(merge(&mut b, &me, &a, &gone, 3000).unwrap().dropped, vec![dad.clone()]);
    }

    #[test]
    fn made_a_family_device_on_one_of_yours_is_one_here_too() {
        let d = tempfile::tempdir().unwrap();
        let (a, c, me) = (id('a'), id('c'), id('b'));
        let mut b = peers(d.path());
        b.add_paired(&a, "Ally", false);
        b.add_paired(&c, "Dad's PC", false);
        let paired = b.get(&c).unwrap().paired_at;
        let msg = json!({"devices": [], "family": [{"id": c, "name": "Dad's PC", "pairedAt": paired}], "removed": []});
        assert_eq!(merge(&mut b, &me, &a, &msg, paired + 10).unwrap().demoted, vec![c.clone()]);
        assert!(b.get(&c).unwrap().i_help());
        // Never the other way: a list never makes one yours again.
        let up = json!({"devices": [{"id": c, "name": "Dad's PC", "pairedAt": paired + 50}], "family": [], "removed": []});
        merge(&mut b, &me, &a, &up, paired + 60).unwrap();
        assert!(b.get(&c).unwrap().family());
    }

    #[test]
    fn the_family_device_hears_names_only_from_the_device_it_paired_with() {
        let d = tempfile::tempdir().unwrap();
        let (ally, laptop, deck, stranger, me) = (id('a'), id('c'), id('e'), id('9'), id('d'));
        // On Dad's PC: paired with Chris's Ally with a code.
        let mut dad = peers(d.path());
        fam(&mut dad, &ally, "Chris's Ally", "helper");
        dad.add_paired(&id('f'), "Uncle's Deck", true);
        // What the Ally says: its other devices' names (and relay), nothing else.
        let d2 = tempfile::tempdir().unwrap();
        let mut chris = peers(d2.path());
        chris.add_paired(&laptop, "Laptop", false);
        fam(&mut chris, &me, "Dad's PC", "member");
        chris.add_paired(&id('f'), "Sam's Deck", true);
        let intro = intro_payload(&chris, &me);
        let devs = intro["devices"].as_array().unwrap();
        assert_eq!(devs.len(), 1);
        assert_eq!(devs[0]["id"], laptop.as_str());
        let keys: Vec<&String> = devs[0].as_object().unwrap().keys().collect();
        assert_eq!(keys, vec!["id", "name", "relay"]);
        let ch = merge_intro(&mut dad, &me, &ally, &intro, 1000).unwrap();
        assert_eq!(ch.added, vec![laptop.clone()]);
        let l = dad.get(&laptop).unwrap();
        assert!(l.helps_me() && l.status_shared() && !l.shares.library && !l.shares.storage && !l.shares.saves && !l.shares.backup);
        assert_eq!(l.introduced_by.as_deref(), Some(ally.as_str()));
        // Only the device it paired with: not one it heard of, not a friend's, not a stranger.
        let more = json!({"devices": [{"id": deck, "name": "Deck"}], "removed": []});
        assert!(merge_intro(&mut dad, &me, &laptop, &more, 1100).is_err());
        assert!(merge_intro(&mut dad, &me, &id('f'), &more, 1100).is_err());
        assert!(merge_intro(&mut dad, &me, &stranger, &more, 1100).is_err());
        assert!(!dad.is_paired(&deck));
        // Removals only of what that device introduced (never the Ally itself, never a friend's).
        let rm = json!({"devices": [], "removed": [{"id": laptop, "at": 5}, {"id": id('f'), "at": 5}, {"id": ally, "at": 5}]});
        assert_eq!(merge_intro(&mut dad, &me, &ally, &rm, 1200).unwrap().dropped, vec![laptop.clone()]);
        assert!(dad.is_paired(&id('f')) && dad.is_paired(&ally));
        // On Chris's side it's never something it can tell you.
        assert!(merge_intro(&mut chris, &id('c'), &me, &intro, 1000).is_err());
    }

    #[test]
    fn old_peers_files_read_as_before_and_older_hearth_reads_family_as_friend() {
        let old: Vec<Peer> = serde_json::from_str(r#"[{"id":"ab","name":"Deck","friend":false},{"id":"cd","name":"Sam","friend":true},{"id":"ef","name":"Old"}]"#).unwrap();
        assert_eq!(old[0].rel(), Rel::Mine);
        assert_eq!(old[1].rel(), Rel::Friend);
        assert_eq!(old[2].rel(), Rel::Mine);
        // Written now, read by a Hearth from before family devices (it only knows `friend`).
        #[derive(serde::Deserialize)]
        struct Older { friend: bool }
        let mut p = Peer { id: "ab".into(), ..Default::default() };
        p.set_rel(Rel::Family, "member");
        let o: Older = serde_json::from_value(serde_json::to_value(&p).unwrap()).unwrap();
        assert!(o.friend);
        let back: Peer = serde_json::from_value(serde_json::to_value(&p).unwrap()).unwrap();
        assert_eq!(back.rel(), Rel::Family);
        assert_eq!(back.family_side, "member");
        // An older Hearth made one of yours a friend's (it kept relation "mine" but set friend): a friend's.
        let mixed: Peer = serde_json::from_str(r#"{"id":"ab","relation":"mine","friend":true}"#).unwrap();
        assert_eq!(mixed.rel(), Rel::Friend);
        let mut m = Peer::default();
        m.set_rel(Rel::Mine, "");
        assert!(!m.friend && m.mine());
    }

    #[test]
    fn only_the_library_goes_to_a_family_device() {
        let mut msg = json!({"rev": 3, "fields": {"p/default/fav/steam:1": {"v": true}, "p/default/hidden/gog:2": {"v": true}, "p/default/col/x": {"v": {}},
            "p/default/opt/steam:1": {"v": {}}, "p/default/profile": {"v": {}}, "g/ui/theme": {"v": "dark"}, "g/input/pad.json": {"v": {}}, "g/x/prefs/k": {"v": 1}},
            "play": {"p/default/steam:1": {"x": {"m": 5, "l": 9}}}});
        crate::sync::library_only(&mut msg);
        let keys: Vec<&String> = msg["fields"].as_object().unwrap().keys().collect();
        assert_eq!(keys, vec!["p/default/col/x", "p/default/fav/steam:1", "p/default/hidden/gog:2"]);
        assert!(msg["play"].is_object());
    }

    #[test]
    fn remote_desktop_says_only_how_to_reach_it() {
        let v = json!({"on": true, "running": true, "installed": true, "address": {"name": "dads-pc", "dns": "dads-pc.tail.ts.net", "ip": "100.1.2.3"},
                       "tailscale": {"running": true}, "clients": [{"name": "Work laptop", "uuid": "x"}], "peers": [{"name": "phone"}], "stream": {"connected": false}});
        let r = remote_view(&v);
        assert_eq!(r["host"], "dads-pc");
        assert_eq!(r["on"], true);
        assert!(r.get("clients").is_none() && r.get("peers").is_none());
    }

    #[test]
    fn removals_are_kept_newest_first_and_capped() {
        let mut l = Vec::new();
        for i in 0..250u64 { remember_removed(&mut l, &format!("{i}"), i); }
        assert_eq!(l.len(), KEEP_REMOVED);
        assert_eq!(l[0].at, 249);
        remember_removed(&mut l, "249", 10);
        assert_eq!(l[0].at, 249);
    }
}
