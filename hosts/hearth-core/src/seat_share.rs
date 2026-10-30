//! Multi-seat shared games (docs/MULTI-SEAT.md, "Sharing games"). hearth-admin seats-setup puts every player's
//! account in the hearth-games group and makes a shared games folder; each player's Hearth then offers its Epic and
//! GOG installs to the others (hearth-seats share-publish writes <shared folder>/.hearth/library-<user>.json and
//! lets the group read the game folders). This reads the other players' lists, and keeps the "In use by …" notes:
//! a file per running game in /run/hearth-seats/in-use, so a copy Steam Family or GOG lets one player use at a time
//! shows who has it.
//!
//! Only what the stores allow is shared: GOG games are DRM-free installs played from another player's folder on the
//! same PC; an Epic install is only offered to a player whose own Epic account has the game, and is started with
//! their own Epic sign-in; Steam games come from the shared folder's Steam library, and Steam itself decides what
//! each account may play (Steam Family).

use serde_json::{json, Value};
use std::collections::HashMap;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

pub const CONFIG: &str = "/etc/hearth/seats.json";
pub const IN_USE: &str = "/run/hearth-seats/in-use";
pub const GROUP: &str = "hearth-games";
const PASSWD: &str = "/etc/passwd";

fn read_json(p: &Path) -> Option<Value> { std::fs::read(p).ok().and_then(|b| serde_json::from_slice(&b).ok()) }

/// The multi-seat set-up, when there is one.
pub fn config() -> Option<Value> { read_json(Path::new(CONFIG)).filter(|v| v.is_object()) }

/// The shared games folder of a set-up made with sharing (older set-ups and ones where the group couldn't be made
/// have none).
pub fn shared_dir(cfg: &Value) -> Option<PathBuf> {
    if cfg["share_group"] != GROUP { return None; }
    let d = cfg["shared_dir"].as_str().filter(|d| d.ends_with("/hearth-games"))?;
    let p = PathBuf::from(d);
    if p.is_dir() { Some(p) } else { None }
}

/// The shared folder, when this PC has multi-seat with shared games.
pub fn active() -> Option<(Value, PathBuf)> { let c = config()?; let d = shared_dir(&c)?; Some((c, d)) }

/// This account's name.
pub fn me() -> String {
    std::env::var("USER").ok().filter(|u| !u.is_empty())
        .unwrap_or_else(|| crate::paths::home().file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default())
}

/// What the other players see: "Player 1", "Player 2"… (the account name for anyone else).
pub fn player_name(cfg: &Value, user: &str) -> String {
    if cfg["main_user"] == user { return "Player 1".into(); }
    cfg["seats"].as_array().and_then(|a| a.iter().find(|s| s["user"] == user)).and_then(|s| s["seat"].as_u64())
        .map(|n| format!("Player {n}")).unwrap_or_else(|| user.to_string())
}
fn is_player(cfg: &Value, user: &str) -> bool {
    cfg["main_user"] == user || cfg["seats"].as_array().is_some_and(|a| a.iter().any(|s| s["user"] == user))
}

/// An account's user id, from /etc/passwd.
pub fn uid_of(user: &str) -> Option<u32> { uid_in(&std::fs::read_to_string(PASSWD).unwrap_or_default(), user) }
pub fn uid_in(passwd: &str, user: &str) -> Option<u32> {
    passwd.lines().map(|l| l.split(':').collect::<Vec<_>>()).find(|f| f.len() >= 3 && f[0] == user).and_then(|f| f[2].parse().ok())
}

/// A file a player wrote: it must really be theirs (a player can make a file in the shared folder named after
/// another who hasn't written theirs yet; the sticky folders only stop replacing one that's there).
fn written_by(p: &Path, user: &str, uid_of: &dyn Fn(&str) -> Option<u32>) -> bool {
    match (std::fs::metadata(p), uid_of(user)) { (Ok(m), Some(u)) => m.uid() == u, _ => false }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SharedGame {
    /// The library id ("gog:1207658924", "epic:Fortnite").
    pub id: String,
    pub store: String,
    pub title: String,
    pub path: PathBuf,
    /// Whose copy ("Player 1").
    pub from: String,
    pub art: Value,
}

/// The games the other players share, player 1's first. Leaves out this player's own list, games whose folder this
/// account can't get to, stores other than Epic and GOG, and a game listed twice (the first player's copy is used).
pub fn shared_games(dir: &Path, me: &str, cfg: &Value, uid_of: &dyn Fn(&str) -> Option<u32>) -> Vec<SharedGame> {
    let mut lists: Vec<(u64, String, Value)> = std::fs::read_dir(dir.join(".hearth")).into_iter().flatten().flatten().filter_map(|e| {
        let name = e.file_name().to_string_lossy().to_string();
        let user = name.strip_prefix("library-")?.strip_suffix(".json")?.to_string();
        if user == me || !is_player(cfg, &user) || !written_by(&e.path(), &user, uid_of) { return None; }
        let v = read_json(&e.path())?;
        if v["user"] != user.as_str() { return None; }
        let seat = if cfg["main_user"] == user.as_str() { 1 } else { v["seat"].as_u64().unwrap_or(9) };
        Some((seat, user, v))
    }).collect();
    lists.sort_by(|a, b| (a.0, &a.1).cmp(&(b.0, &b.1)));
    let mut out: Vec<SharedGame> = Vec::new();
    for (_, user, v) in lists {
        let from = player_name(cfg, &user);
        for g in v["games"].as_array().cloned().unwrap_or_default() {
            let (Some(id), Some(store), Some(path)) = (g["id"].as_str(), g["store"].as_str(), g["path"].as_str()) else { continue };
            if !matches!(store, "gog" | "epic") || !id.starts_with(&format!("{store}:")) || out.iter().any(|o| o.id == id) { continue; }
            let path = PathBuf::from(path);
            // This account must be able to read it (hearth-seats opened it to the group).
            if !path.is_absolute() || std::fs::read_dir(&path).is_err() { continue; }
            out.push(SharedGame { id: id.to_string(), store: store.to_string(), title: g["title"].as_str().unwrap_or(id).to_string(), path, from: from.clone(), art: g["art"].clone() });
        }
    }
    out
}

fn note_file(dir: &Path, id: &str) -> PathBuf {
    let safe: String = id.chars().map(|c| if c.is_ascii_alphanumeric() || "._-".contains(c) { c } else { '_' }).collect();
    dir.join(format!("{safe}.json"))
}

/// Notes that a game runs on this seat (its Hearth's process: a Hearth that's gone doesn't keep a game busy).
pub fn take(dir: &Path, id: &str, me: &str, pid: u32) -> Result<(), String> {
    if !dir.is_dir() { return Ok(()); }
    let body = json!({"id": id, "user": me, "pid": pid, "since": crate::library::now_ms()});
    crate::paths::write_atomic(&note_file(dir, id), body.to_string().as_bytes()).map_err(|e| e.to_string())
}

/// The game ended on this seat. Only this player's own note goes.
pub fn release(dir: &Path, id: &str, me: &str) {
    let f = note_file(dir, id);
    if read_json(&f).is_some_and(|v| v["user"] == me) { let _ = std::fs::remove_file(f); }
}

/// Games running on the other seats: id → who ("Player 2").
pub fn in_use(dir: &Path, me: &str, cfg: &Value, uid_of: &dyn Fn(&str) -> Option<u32>, alive: &dyn Fn(u32) -> bool) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let Some(v) = read_json(&e.path()) else { continue };
        let (Some(id), Some(user), Some(pid)) = (v["id"].as_str(), v["user"].as_str(), v["pid"].as_u64()) else { continue };
        if user == me || !is_player(cfg, user) || !written_by(&e.path(), user, uid_of) || e.path() != note_file(dir, id) { continue; }
        if !alive(pid as u32) { continue; }
        out.insert(id.to_string(), player_name(cfg, user));
    }
    out
}

/// Whether a busy game must wait for the other player: a GOG copy that's another player's (one copy, one player at
/// a time), or a Steam game another account installed in the shared library (Steam Family lends each copy to one
/// player at a time; Steam has the last word, this only says who has it first). Epic installs are each played with
/// the player's own Epic copy, so they never wait.
pub fn must_wait(source: &str, shared_from: Option<&str>, borrowed: bool) -> bool {
    match source { "gog" => shared_from.is_some(), "steam" => borrowed, _ => false }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn cfg() -> Value {
        json!({"main_user": "bob", "share_group": "hearth-games", "seats": [{"seat": 1, "user": "bob"}, {"seat": 2, "user": "hearth-p2"}, {"seat": 3, "user": "hearth-p3"}]})
    }
    fn mine(_: &str) -> Option<u32> { Some(std::fs::metadata("/proc/self").map(|m| m.uid()).unwrap_or(0)) }
    fn put(p: &Path, v: Value) { std::fs::create_dir_all(p.parent().unwrap()).unwrap(); std::fs::write(p, v.to_string()).unwrap(); }

    #[test]
    fn names_players() {
        assert_eq!(player_name(&cfg(), "bob"), "Player 1");
        assert_eq!(player_name(&cfg(), "hearth-p3"), "Player 3");
        assert_eq!(player_name(&cfg(), "eve"), "eve");
        assert_eq!(uid_in("root:x:0:0::/root:/bin/bash\nbob:x:1000:1000::/var/home/bob:/bin/bash\n", "bob"), Some(1000));
        assert_eq!(uid_in("bob:x:1000:1000::/var/home/bob:/bin/bash\n", "eve"), None);
    }

    #[test]
    fn shared_folder_only_with_sharing() {
        let t = tempfile::tempdir().unwrap();
        let d = t.path().join("hearth-games");
        std::fs::create_dir_all(&d).unwrap();
        let mut c = cfg();
        c["shared_dir"] = json!(d.to_string_lossy());
        assert_eq!(shared_dir(&c), Some(d.clone()));
        c["share_group"] = Value::Null;
        assert_eq!(shared_dir(&c), None, "a set-up from before sharing");
        c["share_group"] = json!("hearth-games");
        c["shared_dir"] = json!("/etc");
        assert_eq!(shared_dir(&c), None);
    }

    #[test]
    fn reads_the_other_players_lists() {
        let t = tempfile::tempdir().unwrap();
        let dir = t.path();
        let game = dir.join("GOG/Witcher");
        let epic = dir.join("Epic/Hades");
        std::fs::create_dir_all(&game).unwrap();
        std::fs::create_dir_all(&epic).unwrap();
        put(&dir.join(".hearth/library-bob.json"), json!({"user": "bob", "seat": 1, "games": [
            {"id": "gog:1207658924", "store": "gog", "title": "The Witcher", "path": game, "art": {"cover": "https://x/c.jpg"}},
            {"id": "epic:Min", "store": "epic", "title": "Hades", "path": epic},
            {"id": "gog:2", "store": "gog", "title": "Gone", "path": dir.join("nowhere")},
            {"id": "steam:1", "store": "steam", "title": "Not offered this way", "path": game}]}));
        put(&dir.join(".hearth/library-hearth-p3.json"), json!({"user": "hearth-p3", "seat": 3, "games": [
            {"id": "gog:1207658924", "store": "gog", "title": "The Witcher (p3)", "path": game}]}));
        put(&dir.join(".hearth/library-hearth-p2.json"), json!({"user": "hearth-p2", "seat": 2, "games": [
            {"id": "gog:9", "store": "gog", "title": "Mine", "path": game}]}));
        // Someone's list under another name, and a list from an account that isn't a player.
        put(&dir.join(".hearth/library-eve.json"), json!({"user": "eve", "games": [{"id": "gog:7", "store": "gog", "path": game}]}));
        put(&dir.join(".hearth/library-hearth-p4.json"), json!({"user": "hearth-p4", "games": [{"id": "gog:8", "store": "gog", "path": game}]}));
        let got = shared_games(dir, "hearth-p2", &cfg(), &mine);
        let ids: Vec<&str> = got.iter().map(|g| g.id.as_str()).collect();
        assert_eq!(ids, vec!["gog:1207658924", "epic:Min"], "{got:?}");
        assert_eq!((got[0].from.as_str(), got[0].title.as_str()), ("Player 1", "The Witcher"), "player 1's copy first");
        assert_eq!(got[0].art["cover"], "https://x/c.jpg");
        // A file that isn't the named player's own is ignored.
        let nobody = |_: &str| Some(u32::MAX);
        assert!(shared_games(dir, "hearth-p2", &cfg(), &nobody).is_empty());
        // Player 1 sees the others' copies, in player order (and not their own list).
        let p1: Vec<String> = shared_games(dir, "bob", &cfg(), &mine).into_iter().map(|g| format!("{} {}", g.id, g.from)).collect();
        assert_eq!(p1, vec!["gog:9 Player 2", "gog:1207658924 Player 3"]);
    }

    #[test]
    fn in_use_notes() {
        let t = tempfile::tempdir().unwrap();
        let dir = t.path();
        take(dir, "steam:620", "hearth-p2", 4242).unwrap();
        take(dir, "gog:1", "bob", 1).unwrap();
        let alive = |p: u32| p == 4242 || p == 1;
        let seen = in_use(dir, "bob", &cfg(), &mine, &alive);
        assert_eq!(seen.get("steam:620").map(String::as_str), Some("Player 2"));
        assert!(!seen.contains_key("gog:1"), "your own game isn't 'in use by' someone else");
        assert_eq!(in_use(dir, "hearth-p3", &cfg(), &mine, &alive).len(), 2);
        // A Hearth that's gone doesn't keep it busy.
        assert!(in_use(dir, "bob", &cfg(), &mine, &|_| false).is_empty());
        // Only your own note goes.
        release(dir, "steam:620", "bob");
        assert!(in_use(dir, "bob", &cfg(), &mine, &alive).contains_key("steam:620"));
        release(dir, "steam:620", "hearth-p2");
        assert!(in_use(dir, "bob", &cfg(), &mine, &alive).is_empty());
        // No in-use folder (no sharing): nothing to note, no error.
        assert!(take(&dir.join("missing"), "gog:1", "bob", 1).is_ok());
    }

    #[test]
    fn who_waits() {
        assert!(must_wait("gog", Some("Player 1"), false));
        assert!(!must_wait("gog", None, false), "the owner of a GOG copy never waits for the copy they lent");
        assert!(must_wait("steam", None, true));
        assert!(!must_wait("steam", None, false));
        assert!(!must_wait("epic", Some("Player 1"), false), "each player plays their own Epic copy");
    }
}
