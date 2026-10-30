//! "Continue on this device": a game that's running (or just quit) on one paired Hearth device carries on on
//! another, with its save.
//!
//! - Hearth (hearth-core link.rs) keeps `now-playing.json` in Hearth Link's folder: {game, title, at} when a game
//!   starts, plus `quitAt` when it ends.
//! - Any paired device asks the others what they're playing (`handoff.now`). Game Mode shows a card "Continue <game>
//!   from <device>" for a game that's running there, or quit there within the last 15 minutes, when it's installed
//!   here.
//! - Picking it (`handoff.take`): this device asks the other (`handoff.ask`). A game still running there is never
//!   closed from here (that could lose progress): the other device shows "<device> wants to continue <game>: quit
//!   it here" (Game Mode and a desktop notification), and this device waits (at most 5 minutes) until it's quit.
//!   Then the other device sends its save the usual way (saves follow you: `saves_after`, to the Hearth Storage
//!   server), this device waits for that to finish, and Game Mode starts the game here the usual way, which first
//!   brings the newest save from the server (saves.rs).
//! - A game whose store syncs saves itself (Steam Cloud and the like) needs nothing more; without a Hearth Storage
//!   server and without store cloud saves the card says the save won't come along, and asks first.
//!
//! Between devices: handoff.now, handoff.ask {game}, handoff.cancel. From the UI: handoff.status, handoff.take
//! {peer, game, wait?}, handoff.decline {peer}.

use crate::node::Node;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

/// A quit game is offered to the other devices this long.
pub const RECENT_MS: u64 = 15 * 60_000;
/// An ask from another device is shown this long.
const ASK_MS: u64 = 10 * 60_000;
/// How long `handoff.take` waits for the other device (the game quit there, its save sent).
const TAKE_WAIT: Duration = Duration::from_secs(300);

#[derive(Default)]
struct State {
    /// Asks to this device: peer → {game, title, at, declined}.
    asks: HashMap<String, Value>,
    /// What the other devices said they're playing: peer → (when asked, answer).
    seen: HashMap<String, (Instant, Value)>,
}

fn state() -> &'static Mutex<State> { static S: OnceLock<Mutex<State>> = OnceLock::new(); S.get_or_init(|| Mutex::new(State::default())) }

/// What this device is playing or just quit, from now-playing.json: {game, title, playing, at, quitAt}, or null
/// when there's nothing to offer (no game, quit too long ago, or a file left from a game that can't be running).
pub fn read_now(dir: &Path, running: bool, now: u64) -> Value {
    let v: Value = std::fs::read(dir.join("now-playing.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null);
    now_from(&v, running, now)
}

pub fn now_from(v: &Value, running: bool, now: u64) -> Value {
    let Some(game) = v["game"].as_str().filter(|g| !g.is_empty() && g.len() < 200) else { return Value::Null };
    let quit = v["quitAt"].as_u64();
    // Running: the file says so and Hearth's "a game is running" file agrees. Otherwise it ended (even if Hearth
    // didn't get to write quitAt): offered for RECENT_MS from when it ended, or from when it started.
    let playing = quit.is_none() && running;
    let ended = quit.unwrap_or_else(|| v["at"].as_u64().unwrap_or(0));
    if !playing && now.saturating_sub(ended) > RECENT_MS { return Value::Null; }
    json!({"game": game, "title": v["title"].as_str().unwrap_or(game), "playing": playing, "at": v["at"], "quitAt": quit})
}

/// The answer to an ask once the game isn't running there: the save's state after `saves_after` (None: it wasn't
/// called because a push is already under way).
pub fn ready_state(playing: bool, pushing: bool, after: Option<&Result<Value, String>>) -> &'static str {
    if playing { return "playing"; }
    if pushing { return "saving"; }
    match after {
        None => "ready",
        Some(Err(_)) => "unsynced",
        Some(Ok(v)) if v["conflict"] == true => "conflict",
        Some(Ok(v)) if v["pushed"] == true => "saving",
        Some(Ok(v)) if v["later"] == true => "unsynced",
        Some(Ok(_)) => "ready",
    }
}

/// States `handoff.take` keeps waiting through.
pub fn still_waiting(state: &str) -> bool { matches!(state, "playing" | "saving") }

impl Node {
    fn handoff_peer_name(&self, id: &str) -> String { crate::lock(&self.peers).get(id).map(|p| p.name.clone()).unwrap_or_else(|| "another device".into()) }

    pub async fn handoff_answer(self: Arc<Self>, peer: &str, op: &str, req: &Value) -> Result<Value, String> {
        let now = crate::now_ms();
        match op {
            "handoff.now" => Ok(json!({"name": self.name(), "now": read_now(&self.dir, crate::game_running(), now)})),
            "handoff.ask" => {
                let game = req["game"].as_str().unwrap_or("").to_string();
                let cur = read_now(&self.dir, crate::game_running(), now);
                if cur["game"].as_str() != Some(game.as_str()) { return Err(format!("{} isn't playing that anymore", self.name())); }
                let title = cur["title"].as_str().unwrap_or("the game").to_string();
                if cur["playing"] == true {
                    let first = {
                        let mut s = crate::lock(state());
                        if s.asks.get(peer).map(|a| a["declined"] == true && a["game"] == game.as_str()).unwrap_or(false) { return Ok(json!({"state": "declined"})); }
                        let first = !s.asks.contains_key(peer);
                        s.asks.insert(peer.to_string(), json!({"game": game, "title": title, "at": now, "declined": false}));
                        first
                    };
                    if first {
                        let who = self.handoff_peer_name(peer);
                        eprintln!("hearth-link: handoff: {who} wants to continue {title}: waiting for it to be quit here");
                        let (t, b) = (format!("{who} wants to continue {title}"), "Quit the game here to hand it over (your save goes with it).".to_string());
                        std::thread::spawn(move || { let _ = std::process::Command::new("notify-send").args(["-a", "Hearth", t.as_str(), b.as_str()]).status(); });
                    }
                    return Ok(json!({"state": "playing", "title": title}));
                }
                crate::lock(state()).asks.remove(peer);
                let pushing = !crate::saves::load_state(&self.dir)["games"][&game]["pushing"].is_null();
                let after = if pushing { None } else { Some(self.saves_after(&game).await) };
                let st = ready_state(false, pushing, after.as_ref());
                let why = after.and_then(|r| r.err());
                Ok(json!({"state": st, "title": title, "synced": self.save_server().is_some(), "why": why}))
            }
            "handoff.cancel" => { crate::lock(state()).asks.remove(peer); Ok(json!({"ok": true})) }
            _ => Err(format!("Unknown request: {op}")),
        }
    }

    pub async fn handoff_command(self: &Arc<Self>, cmd: &str, a: &Value) -> Result<Value, String> {
        let now = crate::now_ms();
        match cmd {
            "handoff.status" => {
                // The other devices' games (asked at most every 15 seconds; never while a game runs here).
                let peers: Vec<(String, String)> = crate::lock(&self.peers).list.iter().map(|p| (p.id.clone(), p.name.clone())).collect();
                if !crate::game_running() {
                    let stale: Vec<(String, String)> = { let s = crate::lock(state()); peers.iter().filter(|(id, _)| s.seen.get(id).map(|(t, _)| t.elapsed() > Duration::from_secs(15)).unwrap_or(true)).cloned().collect() };
                    let mut set = tokio::task::JoinSet::new();
                    for (id, _) in stale {
                        let n = self.clone();
                        set.spawn(async move { let r = tokio::time::timeout(Duration::from_secs(4), n.request(&id, &json!({"op": "handoff.now"}))).await; (id, r) });
                    }
                    while let Some(r) = set.join_next().await {
                        let Ok((id, r)) = r else { continue };
                        let v = match r { Ok(Ok(v)) => v["now"].clone(), _ => Value::Null };
                        crate::lock(state()).seen.insert(id, (Instant::now(), v));
                    }
                }
                let s = crate::lock(state());
                let offers: Vec<Value> = peers.iter().filter_map(|(id, name)| {
                    let v = &s.seen.get(id)?.1;
                    v["game"].as_str()?;
                    Some(json!({"peer": id, "name": name, "game": v["game"], "title": v["title"], "playing": v["playing"], "quitAt": v["quitAt"]}))
                }).collect();
                let asked: Vec<Value> = s.asks.iter().filter(|(_, v)| now.saturating_sub(v["at"].as_u64().unwrap_or(0)) < ASK_MS && v["declined"] != true)
                    .map(|(id, v)| json!({"peer": id, "name": peers.iter().find(|p| &p.0 == id).map(|p| p.1.clone()).unwrap_or_default(), "game": v["game"], "title": v["title"]})).collect();
                Ok(json!({"offers": offers, "asked": asked, "savesServer": self.save_server().is_some()}))
            }
            "handoff.decline" => {
                let peer = a["peer"].as_str().unwrap_or("");
                if let Some(x) = crate::lock(state()).asks.get_mut(peer) { x["declined"] = json!(true); }
                Ok(json!({"ok": true}))
            }
            "handoff.take" => {
                let peer = a["peer"].as_str().filter(|p| !p.is_empty()).ok_or("Missing 'peer'")?.to_string();
                let game = a["game"].as_str().filter(|g| !g.is_empty()).ok_or("Missing 'game'")?.to_string();
                if !crate::lock(&self.peers).is_paired(&peer) { return Err("Not a paired device".into()); }
                if crate::game_running() { return Err("A game is running on this device: quit it first".into()); }
                let name = self.handoff_peer_name(&peer);
                let wait = a["wait"].as_u64().map(Duration::from_secs).unwrap_or(TAKE_WAIT).min(TAKE_WAIT);
                let t = Instant::now();
                eprintln!("hearth-link: handoff: continuing {game} from {name}");
                loop {
                    let v = match tokio::time::timeout(Duration::from_secs(60), self.request(&peer, &json!({"op": "handoff.ask", "game": game}))).await {
                        Ok(Ok(v)) => v,
                        Ok(Err(e)) => return Err(e.trim_start_matches("connection: ").to_string()),
                        Err(_) => return Err(format!("{name} didn't answer")),
                    };
                    let st = v["state"].as_str().unwrap_or("").to_string();
                    if !still_waiting(&st) {
                        eprintln!("hearth-link: handoff: {game} from {name}: {st}");
                        crate::lock(state()).seen.remove(&peer);
                        return Ok(json!({"state": st, "name": name, "title": v["title"], "synced": v["synced"], "why": v["why"]}));
                    }
                    if t.elapsed() > wait {
                        let _ = tokio::time::timeout(Duration::from_secs(10), self.request(&peer, &json!({"op": "handoff.cancel"}))).await;
                        return Err(if st == "playing" { format!("It's still running on {name}: quit it there, then try again") } else { format!("{name} is still sending the save: try again in a minute") });
                    }
                    tokio::time::sleep(Duration::from_secs(3)).await;
                }
            }
            _ => Err(format!("Unknown command: {cmd}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_is_offered() {
        let now = 100 * 60_000;
        let playing = json!({"game": "steam:10", "title": "Portal", "at": now - 3_600_000});
        assert_eq!(now_from(&playing, true, now)["playing"], true, "running for an hour: still offered");
        // The file says running but Hearth's running file is gone: it ended, an hour ago.
        assert!(now_from(&playing, false, now).is_null());
        let quit = json!({"game": "steam:10", "title": "Portal", "at": now - 3_600_000, "quitAt": now - 60_000});
        let v = now_from(&quit, true, now);
        assert_eq!(v["playing"], false, "quit, even though another game may be running");
        assert_eq!(v["title"], "Portal");
        let old = json!({"game": "steam:10", "at": 0, "quitAt": now - RECENT_MS - 1});
        assert!(now_from(&old, false, now).is_null(), "quit too long ago");
        assert!(now_from(&json!({}), true, now).is_null());
        assert!(now_from(&json!({"game": ""}), true, now).is_null());
    }

    #[test]
    fn reading_the_file() {
        let d = tempfile::tempdir().unwrap();
        assert!(read_now(d.path(), true, 1).is_null(), "no file: nothing");
        std::fs::write(d.path().join("now-playing.json"), r#"{"game":"gog:1","title":"X","at":1000}"#).unwrap();
        assert_eq!(read_now(d.path(), true, 2000)["game"], "gog:1");
        std::fs::write(d.path().join("now-playing.json"), "not json").unwrap();
        assert!(read_now(d.path(), true, 2000).is_null());
    }

    #[test]
    fn the_answer_once_quit() {
        assert_eq!(ready_state(true, false, None), "playing");
        assert_eq!(ready_state(false, true, None), "saving", "a push under way: wait for it");
        assert_eq!(ready_state(false, false, None), "ready");
        assert_eq!(ready_state(false, false, Some(&Ok(json!({"pushed": false})))), "ready", "nothing changed, or not followed");
        assert_eq!(ready_state(false, false, Some(&Ok(json!({"pushed": true})))), "saving");
        assert_eq!(ready_state(false, false, Some(&Ok(json!({"pushed": false, "conflict": true})))), "conflict");
        assert_eq!(ready_state(false, false, Some(&Ok(json!({"pushed": false, "later": true})))), "unsynced");
        assert_eq!(ready_state(false, false, Some(&Err("off".into()))), "unsynced");
        assert!(still_waiting("playing") && still_waiting("saving"));
        assert!(!still_waiting("ready") && !still_waiting("declined") && !still_waiting("unsynced") && !still_waiting("conflict"));
    }
}
