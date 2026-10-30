//! Hearth Link (hosts/hearth-link): your Hearth devices paired, Hearth data kept in sync, games copied between them.
//! The service is its own program (`hearth-link serve`, a systemd user unit); this is Hearth's side of it:
//! - the bridge's "link.*" calls (Settings › Hearth Link, Library › Get from another Hearth) go to the service's
//!   socket ($XDG_RUNTIME_DIR/hearth-link/link.sock), as `hearth-link call` would send them;
//! - synced changes to library.json (which Hearth keeps in memory and writes itself) come as an inbox
//!   (link/inbox.json, under link/inbox.lock) that's applied here, the next time the library is read;
//! - the UI's own settings (appearance, Made for, controller choices: localStorage) go through link/ui.json
//!   ("link.ui.sync");
//! - while a game runs Hearth keeps $XDG_RUNTIME_DIR/hearth-playing, and Hearth Link stays quiet.

use crate::store::Store;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub const PROGRAM: &str = "/usr/bin/hearth-link";

fn runtime_dir() -> PathBuf { std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap_or_else(std::env::temp_dir) }
pub fn playing_file() -> PathBuf { runtime_dir().join("hearth-playing") }
pub fn set_playing(on: bool) { if on { let _ = std::fs::write(playing_file(), b"1\n"); } else { let _ = std::fs::remove_file(playing_file()); } }

pub fn link_dir() -> PathBuf { std::env::var_os("HEARTH_LINK_HOME").map(PathBuf::from).unwrap_or_else(|| crate::paths::data_dir().join("link")) }
pub fn socket_path() -> PathBuf {
    std::env::var_os("HEARTH_LINK_SOCKET").map(PathBuf::from).unwrap_or_else(|| runtime_dir().join("hearth-link").join("link.sock"))
}
pub fn available() -> bool { Path::new(PROGRAM).exists() || socket_path().exists() }

/// One request to the service, and its answer.
pub fn call(cmd: &str, args: &Value, timeout_s: u64) -> Result<Value, String> {
    use std::io::{BufRead, BufReader, Write};
    let mut s = std::os::unix::net::UnixStream::connect(socket_path()).map_err(|_| "Hearth Link isn't running on this device".to_string())?;
    s.set_read_timeout(Some(std::time::Duration::from_secs(timeout_s))).ok();
    let req = json!({"id": 1, "cmd": cmd, "args": args}).to_string() + "\n";
    s.write_all(req.as_bytes()).map_err(|e| e.to_string())?;
    let mut line = String::new();
    BufReader::new(s).read_line(&mut line).map_err(|_| "Hearth Link took too long to answer".to_string())?;
    let v: Value = serde_json::from_str(&line).map_err(|_| "Hearth Link didn't answer".to_string())?;
    if v["ok"] == true { Ok(v["data"].clone()) } else { Err(v["error"].as_str().unwrap_or("That didn't work").to_string()) }
}

fn read_json(p: &Path) -> Value { std::fs::read(p).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null) }

fn with_lock<T>(dir: &Path, f: impl FnOnce() -> T) -> T {
    let _ = std::fs::create_dir_all(dir);
    let lock = std::fs::OpenOptions::new().create(true).truncate(false).write(true).open(dir.join("inbox.lock"));
    if let Ok(l) = &lock { let _ = l.lock(); }
    let r = f();
    if let Ok(l) = &lock { let _ = l.unlock(); }
    r
}

const OPT_DEVICE_ONLY: [&str; 3] = ["tdp", "hhdPower", "powerLabel"];

/// Applies one profile's synced changes to its library.json data.
pub fn apply_changes(d: &mut serde_json::Map<String, Value>, pp: &Value) {
    for (k, v) in pp["set"].as_object().into_iter().flatten() {
        let (kind, id) = k.split_once('/').unwrap_or((k.as_str(), ""));
        match kind {
            "fav" | "hidden" if !id.is_empty() => {
                let list = crate::store::array_in(d, if kind == "fav" { "favorites" } else { "hidden" });
                list.retain(|x| x != id);
                if v == &json!(true) { list.push(json!(id)); }
            }
            "col" if !id.is_empty() => {
                let cols = crate::store::array_in(d, "collections");
                let at = cols.iter().position(|c| c["id"] == id);
                match (at, v.is_object()) {
                    (Some(i), true) => { cols[i]["name"] = v["name"].clone(); cols[i]["games"] = v["games"].clone(); }
                    (None, true) => cols.push(json!({"id": id, "name": v["name"], "games": v["games"]})),
                    (Some(i), false) => { cols.remove(i); }
                    (None, false) => {}
                }
            }
            "opt" if !id.is_empty() => {
                let opts = crate::store::object_in(d, "gameOptions");
                let mut keep = serde_json::Map::new();
                if let Some(old) = opts.get(id).and_then(Value::as_object) { for k in OPT_DEVICE_ONLY { if let Some(x) = old.get(k) { keep.insert(k.into(), x.clone()); } } }
                let mut new = v.as_object().cloned().unwrap_or_default();
                for k in OPT_DEVICE_ONLY { new.remove(k); }
                new.extend(keep);
                if new.is_empty() { opts.remove(id); } else { opts.insert(id.to_string(), Value::Object(new)); }
            }
            "profile" => {
                let p = crate::store::object_in(d, "profile");
                for (pk, pv) in v.as_object().into_iter().flatten() { p.insert(pk.clone(), pv.clone()); }
            }
            "qr" => { if v.is_boolean() { d.insert("quickResume".into(), v.clone()); } }
            _ => {}
        }
    }
    for (g, n) in pp["add"].as_object().into_iter().flatten() {
        let s = crate::store::object_at(crate::store::object_in(d, "stats"), g.clone(), json!({"minutes": 0, "lastPlayed": 0}));
        s["minutes"] = json!(s["minutes"].as_u64().unwrap_or(0) + n.as_u64().unwrap_or(0));
    }
    for (g, n) in pp["last"].as_object().into_iter().flatten() {
        let s = crate::store::object_at(crate::store::object_in(d, "stats"), g.clone(), json!({"minutes": 0, "lastPlayed": 0}));
        s["lastPlayed"] = json!(s["lastPlayed"].as_u64().unwrap_or(0).max(n.as_u64().unwrap_or(0)));
    }
}

/// Applies what Hearth Link left in the inbox for the active profile (other profiles' changes wait for theirs).
/// Returns (something changed, a GOG install was added: the library needs a rescan).
pub fn apply_inbox(store: &mut Store, active: &str) -> (bool, bool) {
    let dir = link_dir();
    let file = dir.join("inbox.json");
    if !file.exists() { return (false, false); }
    with_lock(&dir, || {
        let mut inbox = read_json(&file);
        if !inbox.is_object() { let _ = std::fs::remove_file(&file); return (false, false); }
        let pp = inbox["profiles"][active].take();
        let gog = inbox["gogInstalls"].take();
        let changed = pp.is_object() || gog.is_object();
        if changed {
            let _ = store.update(|d| {
                if pp.is_object() { apply_changes(d, &pp); }
                if let Some(g) = gog.as_object() { let gi = crate::store::object_in(d, "gogInstalls"); for (k, v) in g { gi.insert(k.clone(), v.clone()); } }
            });
        }
        if let Some(m) = inbox["profiles"].as_object_mut() { m.remove(active); m.retain(|_, v| v.is_object()); }
        if let Some(m) = inbox.as_object_mut() { m.remove("gogInstalls"); }
        let empty = inbox["profiles"].as_object().map(|m| m.is_empty()).unwrap_or(true);
        if empty { let _ = std::fs::remove_file(&file); } else { let _ = crate::paths::write_atomic(&file, inbox.to_string().as_bytes()); }
        (changed, gog.is_object())
    })
}

/// The UI's own settings: `values` are what it has now, `seen` the times of the synced values it already took.
/// Answers the synced values newer than those (the UI applies them); stores the UI's values for Hearth Link.
pub fn ui_sync(a: &Value) -> Result<Value, String> {
    let dir = link_dir();
    let file = dir.join("ui.json");
    with_lock(&dir, || {
        let mut ui = read_json(&file);
        if !ui.is_object() { ui = json!({}); }
        let mut newer = serde_json::Map::new();
        for (k, r) in ui["remote"].as_object().cloned().unwrap_or_default() {
            let t = r["t"].as_u64().unwrap_or(0);
            if t > a["seen"][&k].as_u64().unwrap_or(0) { newer.insert(k.clone(), json!({"v": r["v"], "t": t})); }
        }
        for (k, v) in a["values"].as_object().into_iter().flatten() {
            if k.len() > 60 || k.contains('/') || newer.contains_key(k) { continue; }
            if serde_json::to_string(v).map(|s| s.len()).unwrap_or(0) > 64 << 10 { continue; }
            ui["local"][k] = v.clone();
        }
        crate::paths::write_atomic(&file, ui.to_string().as_bytes()).map_err(|e| e.to_string())?;
        Ok(json!({"newer": newer}))
    })
}

/// A game closed: Hearth Link sends its save to the Hearth Storage server when saves follow you (it decides; nothing
/// happens when it isn't running or the game's store keeps its saves itself). In the background.
pub fn game_closed(id: &str) {
    if !available() || id.is_empty() { return; }
    let id = id.to_string();
    std::thread::spawn(move || { if let Err(e) = call("storage.saves.after", &json!({"game": id}), 60) { if !e.contains("isn't running") { crate::bridge::log(&format!("hearth link: saves: {e}")); } } });
}

/// The bridge's "link.*" calls.
pub fn bridge_call(br: &mut crate::bridge::Bridge, method: &str, a: &Value) -> Result<Value, String> {
    let cmd = method.trim_start_matches("link.");
    match cmd {
        "ui.sync" => ui_sync(a),
        "status" => {
            let (changed, gog) = apply_inbox(&mut br.library.store, &br.profiles.active());
            if gog { br.library.get(true); }
            match call("status", a, 8) {
                Ok(mut v) => { v["running"] = json!(true); v["libraryChanged"] = json!(changed); Ok(v) }
                Err(e) => Ok(json!({"running": false, "installed": Path::new(PROGRAM).exists(), "error": e, "libraryChanged": changed})),
            }
        }
        // Pairing waits for the other device (looking a short code up, connecting, the code check) or, for a short
        // code, for the lookup services: answered from another thread, so the page keeps polling "status" meanwhile
        // and can say what's happening ("Found Nvidia-PC…").
        "pair.join" | "pair.open" => { let (c, a) = (cmd.to_string(), a.clone()); br.later(move || call(&c, &a, 90)) }
        // Asking another device can take a while (it lists its games).
        "games" | "local.games" | "send" | "sync.now" | "unpair" => call(cmd, a, 60),
        "pair.close" | "pair.check" | "peer.set" | "settings.set" | "places" | "get" | "offer.accept" | "offer.decline"
        | "transfer.pause" | "transfer.resume" | "transfer.cancel" | "transfer.clear" | "ticket" => call(cmd, a, 15),
        // Hearth Storage: anything that asks the server may first have to wake it (up to about a minute and a half),
        // so it's answered from another thread like pairing.
        "storage.play" => { let a = a.clone(); br.later(move || storage_play(&a)) }
        "storage.servers" | "storage.ls" | "storage.get" | "storage.stream" | "storage.archive" | "storage.archived" | "storage.restore"
        | "storage.backup.now" | "storage.archive.remove" | "storage.set" | "storage.share.add" | "storage.share.remove" | "storage.share.set"
        | "storage.smb.password" => { let (c, a) = (cmd.to_string(), a.clone()); br.later(move || call(&c, &a, 150)) }
        "storage.status" | "storage.candidates" | "storage.backups" | "storage.archive.here" | "storage.archive.forget" | "storage.wake" => call(cmd, a, 15),
        // Saves follow you: the check before a game answers within a few seconds (the server isn't woken for it);
        // pulling a save, the health list (asks the server) and removing a device take longer.
        "storage.saves.before" | "storage.saves.pull" | "storage.saves.resolve" | "storage.saves.status" | "storage.devices" | "storage.device.usage"
        | "storage.device.remove" => { let (c, a) = (cmd.to_string(), a.clone()); br.later(move || call(&c, &a, 150)) }
        _ => Err(format!("{method} isn't a Hearth Link call")),
    }
}

/// A video or song from a Hearth Storage server, in Hearth Player: Hearth Link makes a link for it on this device
/// (127.0.0.1 only, for that one file), and the player opens it like any web address.
fn storage_play(a: &Value) -> Result<Value, String> {
    let v = call("storage.stream", a, 150)?;
    let url = v["url"].as_str().unwrap_or("").to_string();
    if !url.starts_with("http://127.0.0.1:") { return Err("Hearth Link gave a link that isn't on this device".into()); }
    const PLAYER: &str = "/usr/bin/hearth-player";
    let mut c = if Path::new(PLAYER).exists() {
        let mut c = std::process::Command::new(PLAYER);
        if crate::gamescope::inside() { c.arg("--fullscreen"); }
        c
    } else { std::process::Command::new("xdg-open") };
    c.arg(&url);
    c.spawn().map(crate::procs::detach).map_err(|e| format!("Couldn't open the player: {e}"))?;
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn synced_changes_land_in_the_library() {
        let mut d = serde_json::Map::new();
        d.insert("favorites".into(), json!(["a"]));
        d.insert("gameOptions".into(), json!({"g": {"tdp": 12, "fsr": false}}));
        d.insert("stats".into(), json!({"g": {"minutes": 10, "lastPlayed": 5}}));
        apply_changes(&mut d, &json!({"set": {"fav/a": null, "fav/b": true, "col/c1": {"name": "RPGs", "games": ["g"]}, "opt/g": {"fsr": true, "tdp": 99},
                                               "profile": {"name": "Ana"}, "qr": false},
                                       "add": {"g": 15, "h": 3}, "last": {"g": 9}}));
        let v = Value::Object(d.clone());
        assert_eq!(v["favorites"], json!(["b"]));
        assert_eq!(v["collections"][0]["name"], "RPGs");
        // The power limit is this device's own: kept.
        assert_eq!(v["gameOptions"]["g"], json!({"fsr": true, "tdp": 12}));
        assert_eq!(v["profile"]["name"], "Ana");
        assert_eq!(v["quickResume"], false);
        assert_eq!(v["stats"]["g"], json!({"minutes": 25, "lastPlayed": 9}));
        assert_eq!(v["stats"]["h"]["minutes"], 3);
        apply_changes(&mut d, &json!({"set": {"col/c1": null}}));
        assert_eq!(Value::Object(d)["collections"], json!([]));
    }
}
