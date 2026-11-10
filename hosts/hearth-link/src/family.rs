//! Your devices and friends' devices.
//!
//! Pairing asks "My device" or "A friend's device" (pair.rs). A device is yours only when both sides said My device.
//! Your own devices then share who else is yours (family.sync), so pairing a new device with any one of them pairs
//! it with all of them: no code on each. A device of yours that was removed is remembered (state.rs `Removed`) and
//! the others drop it too; pairing with it again later wins over the removal.
//!
//! Safety: a device is only ever added on the word of one of your own devices (an already paired, already trusted
//! peer, over Hearth Link's end-to-end encrypted connection, which only paired devices can open). A friend's device
//! is never asked and never listened to: family.sync from it is refused, and it's never told about your other
//! devices. Nobody can add a device by showing up and asking.
//!
//! A friend's device can: be seen online, send you a game copy (you accept it, as before), copy a game you pick
//! for it, and unpair. It can't: sync settings or the library, see or back up saves, use Hearth Storage (shares,
//! archive, backups, the download cache, updates), see your status, hand off a game, use your controllers, start
//! updates, use your AI, run benchmarks, reach Decky, or learn about your other devices.
//!
//! A device on a Hearth OS from before this answers family.sync with "Unknown request": nothing is shared with it and
//! it stays paired as before.

use crate::node::Node;
use crate::state::{Peer, Peers, Removed};
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

pub fn friend_may(op: &str) -> bool { FRIEND_OPS.contains(&op) }

pub fn remember_removed(list: &mut Vec<Removed>, id: &str, at: u64) {
    match list.iter_mut().find(|r| r.id == id) {
        Some(r) => r.at = r.at.max(at),
        None => list.push(Removed { id: id.to_string(), at }),
    }
    list.sort_by_key(|r| std::cmp::Reverse(r.at));
    list.truncate(KEEP_REMOVED);
}

/// What this device tells one of your devices: your other devices (never friends', never the one asked) and the
/// removals it knows.
pub fn payload(peers: &Peers, to: &str) -> Value {
    let devices: Vec<Value> = peers.mine().filter(|p| p.id != to).map(|p| json!({
        "id": p.id, "name": p.name, "pairedAt": p.paired_at, "addrs": p.addrs,
        "relay": p.signed_in.get("relay").cloned().unwrap_or(Value::Null),
    })).collect();
    json!({"op": "family.sync", "v": 1, "devices": devices, "removed": peers.removed})
}

/// What changed after taking another device's list.
#[derive(Debug, Default, PartialEq)]
pub struct Changes { pub added: Vec<String>, pub dropped: Vec<String> }

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
        match peers.get(id).map(|p| (p.friend, p.paired_at)) {
            Some((true, _)) => continue,
            Some((_, paired_at)) if paired_at > at => continue, // paired with it again since
            Some(_) => { peers.list.retain(|p| p.id != id); ch.dropped.push(id.to_string()); remember_removed(&mut peers.removed, id, at); }
            None => remember_removed(&mut peers.removed, id, at),
        }
    }
    for d in v["devices"].as_array().into_iter().flatten().take(64) {
        let Some(id) = d["id"].as_str().filter(|i| valid_id(i)) else { continue };
        if id == me || peers.is_paired(id) { continue; }
        let paired_at = d["pairedAt"].as_u64().unwrap_or(0);
        if peers.removed.iter().any(|r| r.id == id && r.at >= paired_at) { continue; }
        let name: String = d["name"].as_str().unwrap_or("Hearth").chars().filter(|c| !c.is_control()).take(40).collect();
        let addrs: Vec<String> = d["addrs"].as_array().into_iter().flatten()
            .filter_map(|a| a.as_str().filter(|s| s.parse::<std::net::SocketAddr>().is_ok()).map(str::to_string)).take(4).collect();
        let mut signed_in = json!({});
        if let Some(r) = d["relay"].as_str().filter(|r| crate::state::valid_relay_url(r)) { signed_in["relay"] = json!(r); }
        // last_seen 0: not reached yet. Until it is, "not paired" from it means it hasn't heard yet (node.rs).
        peers.list.push(Peer { id: id.to_string(), name, paired_at: now, last_seen: 0, addrs, always_accept: false, signed_in,
                               friend: false, introduced_by: Some(from.to_string()) });
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
        !ch.added.is_empty() || !ch.dropped.is_empty()
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
        if !crate::lock(&self.peers).is_mine(peer) { return; }
        let n = self.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(2)).await;
            n.family_round(None).await;
        });
    }
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
        for op in ["sync", "family.sync", "saves.list", "status.get", "storage.read", "updates.read", "archive.list", "backup.put", "cache.get",
                   "ai.chat", "update.start", "handoff.offer", "input.list", "bench.run", "decky.list", "device.report", "devices.list"] {
            assert!(!friend_may(op), "{op}");
        }
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
