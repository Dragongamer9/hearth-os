//! Mantel's Games section (Desktop Mode's app launcher, desktop/shell/Launcher.qml and its helper
//! desktop/shell/hearth-games): your installed games from Hearth's library, started through Hearth.
//!
//! - `hearth --list-games` prints them as JSON (`list_now`): the same scan the library does, without starting
//!   anything (no download is resumed, nothing is saved).
//! - `hearth --windowed --play <id>` opens Hearth in a window and plays that game the way the library's Play
//!   button does (the page gets it as host.info "play"). A windowed Hearth that's already open is handed the game
//!   instead ($XDG_RUNTIME_DIR/hearth-play-route.json, looked for every second), like store sign-ins
//!   (signin_handoff.rs).

use crate::library::{Game, Library};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// A game handed to an open Hearth older than this is left over from before.
pub const ROUTE_MAX_AGE_SECS: u64 = 60;

/// A library id as the launcher may pass it: "steam:570", "manual:<id>", "beacon:bfme2"... Nothing odd.
pub fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 200 && id.contains(':') && !id.chars().any(|c| c.is_control() || c == '"' || c == '\\')
}

/// The picture for Mantel: a file on this device as it is, a web picture's copy if Hearth downloaded one (the art
/// cache), otherwise its web address.
pub fn art_for_desktop(v: &str) -> String {
    let l = v.to_ascii_lowercase();
    if l.starts_with("http://") || l.starts_with("https://") {
        let cached = crate::paths::data_dir().join("art-cache");
        let name = crate::artcache::name_of(v);
        for ext in ["jpg", "png", "webp", "jpeg", "gif", "avif"] {
            let f = cached.join(format!("{name}.{ext}"));
            if f.is_file() { return f.to_string_lossy().to_string(); }
        }
    }
    v.to_string()
}

/// One game as the launcher shows it.
pub fn entry(g: &Game) -> Value {
    let art = |k: &str| g.art.iter().find(|(kind, _)| *kind == k).map(|(_, v)| art_for_desktop(v));
    json!({"id": g.id, "title": g.title, "source": g.source, "cover": art("cover").or_else(|| art("portrait")), "wide": art("wide").or_else(|| art("hero")), "icon": art("icon")})
}

/// The installed games you can play from the desktop (not hidden ones, not cloud games), A to Z.
pub fn list(lib: &Library) -> Value {
    let mut games: Vec<&Game> = lib.games.values()
        .filter(|g| g.installed && !g.cloud && g.state == "installed" && !lib.store.list_has("hidden", &g.id)).collect();
    games.sort_by_key(|g| g.title.to_lowercase());
    Value::Array(games.into_iter().map(entry).collect())
}

/// `hearth --list-games`: the library read the way Hearth reads it, as JSON.
pub fn list_now() -> Value {
    let store = crate::store::Store::open(crate::paths::data_dir().join("library.json"));
    let mut lib = Library::new(store, crate::paths::home());
    lib.scan_quietly();
    // Beacon's games are read in the background: wait a moment for them.
    if lib.store.source_on("beacon") {
        let slot = lib.beacon_slot();
        for _ in 0..40 {
            if let Some(s) = slot.lock().unwrap().take() { lib.beacon = s; lib.apply_beacon(); break; }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
    list(&lib)
}

/// `--play <id>` (or `--play=<id>`).
pub fn id_arg<I: IntoIterator<Item = String>>(args: I) -> Option<String> {
    let args: Vec<String> = args.into_iter().collect();
    for (i, a) in args.iter().enumerate() {
        let v = if a == "--play" { args.get(i + 1).cloned() } else { a.strip_prefix("--play=").map(str::to_string) };
        if let Some(v) = v { return Some(v).filter(|s| valid_id(s)); }
    }
    None
}
pub fn play_arg() -> Option<String> { id_arg(std::env::args()) }

pub fn route_file() -> PathBuf {
    PathBuf::from(std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into())).join("hearth-play-route.json")
}

/// The game a fresh hand-over asks for (`at` in seconds, or milliseconds).
pub fn parse_route(text: &str, now: u64, max_age: u64) -> Option<String> {
    let v: Value = serde_json::from_str(text).ok()?;
    let id = v["id"].as_str().filter(|s| valid_id(s))?;
    let mut at = v["at"].as_f64()?;
    if at > 1e11 { at /= 1000.0; }
    let age = now as f64 - at;
    if !(-60.0..(max_age as f64)).contains(&age) { return None; }
    Some(id.to_string())
}

/// Reads a hand-over and removes it (taken once: moved aside first).
pub fn take_file(path: &Path, now: u64, max_age: u64) -> Option<String> {
    let taken = path.with_extension(format!("taken-{}", std::process::id()));
    std::fs::rename(path, &taken).ok()?;
    let text = std::fs::read_to_string(&taken).unwrap_or_default();
    let _ = std::fs::remove_file(&taken);
    parse_route(&text, now, max_age)
}
pub fn take_route() -> Option<String> { take_file(&route_file(), crate::signin_handoff::now_secs(), ROUTE_MAX_AGE_SECS) }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids() {
        for ok in ["steam:570", "manual:3f2a", "beacon:bfme2", "epic:Fortnite", "emu:snes:Super Mario World.sfc"] { assert!(valid_id(ok), "{ok}"); }
        for bad in ["", "570", "steam:\n570", "steam:\"x", "a\\b:c"] { assert!(!valid_id(bad), "{bad:?}"); }
        assert!(!valid_id(&format!("steam:{}", "9".repeat(300))));
        let a = |v: &[&str]| id_arg(v.iter().map(|s| s.to_string()));
        assert_eq!(a(&["nimbus", "--windowed", "--play", "steam:570"]).as_deref(), Some("steam:570"));
        assert_eq!(a(&["nimbus", "--play=gog:1207658924"]).as_deref(), Some("gog:1207658924"));
        assert_eq!(a(&["nimbus", "--play"]), None);
        assert_eq!(a(&["nimbus", "--play", "570"]), None);
        assert_eq!(a(&["nimbus", "--windowed"]), None);
    }

    #[test]
    fn hand_over_is_fresh_and_taken_once() {
        let now = 1_800_000_000;
        let r = |id: &str, at: u64| json!({"id": id, "at": at}).to_string();
        assert_eq!(parse_route(&r("steam:570", now), now, ROUTE_MAX_AGE_SECS).as_deref(), Some("steam:570"));
        assert_eq!(parse_route(&r("steam:570", now - 61), now, ROUTE_MAX_AGE_SECS), None);
        assert_eq!(parse_route(&r("steam:570", (now - 5) * 1000), now, ROUTE_MAX_AGE_SECS).as_deref(), Some("steam:570"));
        assert_eq!(parse_route(&r("nope", now), now, ROUTE_MAX_AGE_SECS), None);
        assert_eq!(parse_route("{}", now, ROUTE_MAX_AGE_SECS), None);
        assert_eq!(parse_route("not json", now, ROUTE_MAX_AGE_SECS), None);
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("hearth-play-route.json");
        let t = crate::signin_handoff::now_secs();
        std::fs::write(&f, r("manual:abc", t)).unwrap();
        assert_eq!(take_file(&f, t, ROUTE_MAX_AGE_SECS).as_deref(), Some("manual:abc"));
        assert!(!f.exists());
        assert_eq!(take_file(&f, t, ROUTE_MAX_AGE_SECS), None);
    }

    #[test]
    fn installed_games_for_the_launcher() {
        let dir = tempfile::tempdir().unwrap();
        let store = crate::store::Store::open(dir.path().join("library.json"));
        let mut lib = Library::new(store, dir.path().to_path_buf());
        let mk = |id: &str, title: &str, installed: bool| Game { id: id.into(), title: title.into(), source: "manual", installed,
            state: if installed { "installed" } else { "notInstalled" }, art: vec![("cover", "/games/c.png".into()), ("hero", "https://example.com/h.jpg".into())], ..Default::default() };
        lib.games.insert("manual:b".into(), mk("manual:b", "beta", true));
        lib.games.insert("manual:a".into(), mk("manual:a", "Alpha", true));
        lib.games.insert("epic:x".into(), mk("epic:x", "Not here", false));
        lib.games.insert("manual:h".into(), mk("manual:h", "Hidden one", true));
        let mut cloud = mk("xbox:cloud", "Xbox Cloud Gaming", true);
        cloud.cloud = true;
        lib.games.insert(cloud.id.clone(), cloud);
        lib.store.set_in("hidden", "manual:h", true).unwrap();
        let l = list(&lib);
        let ids: Vec<&str> = l.as_array().unwrap().iter().map(|g| g["id"].as_str().unwrap()).collect();
        assert_eq!(ids, ["manual:a", "manual:b"]);
        assert_eq!(l[0]["cover"], "/games/c.png");
        assert_eq!(l[0]["wide"], "https://example.com/h.jpg");
        assert!(l[0]["icon"].is_null());
    }
}
