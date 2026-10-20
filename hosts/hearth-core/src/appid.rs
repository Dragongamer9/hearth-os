//! Every game and program Hearth starts gets a number, the way SteamOS does it: gamescope shows windows by app
//! id (GAMESCOPECTRL_BASELAYER_APPID), and each window carries the id of the game it belongs to.
//!
//!   Steam games          their Steam app id (gamescope also finds it itself, from Steam's reaper process)
//!   everything else      a stable number for its library id (Epic, GOG, your own games, emulators, Xbox Cloud
//!                        Gaming), kept in library.json "appIds"; always with the high bit set, so it can never
//!                        be a real Steam app id (those are far below 2^31), like Steam's own non-Steam shortcuts
//!   Hearth itself        HEARTH_ID; its overlay OVERLAY_ID (gamescope keeps overlays out of focus anyway)
//!
//! How a window gets its number: Hearth starts every non-Steam game with HEARTH_GAME_ID=<id> in its environment
//! (children and Wine processes inherit it), finds which game a window's process belongs to (procs.rs), and puts
//! the id on the window as the STEAM_GAME cardinal (gamescope.rs), which gamescope takes as the window's app id.
//!
//! Why not SteamGameId: gamescope doesn't read it (it reads Steam's reaper and the app's cgroup scope), umu-run
//! overwrites it (SteamGameId = SteamAppId = the number in GAMEID, or "0"/"default"), and Steamworks games read it
//! as the game Steam is running. GAMEID and SteamAppId stay what umu and Proton's game fixes expect.

use serde_json::{Map, Value};
use std::collections::HashSet;

/// Hearth's own windows (the main one, its sign-in pages and file pickers).
pub const HEARTH_ID: u32 = 0x8000_0001;
/// Hearth's Quick Access overlay window. Only for the log: gamescope keeps overlay windows (STEAM_OVERLAY) out of
/// what it picks to show, so it isn't put on the window.
pub const OVERLAY_ID: u32 = 0x8000_0002;
/// Windows of programs Hearth opened that aren't games (a browser, Moonlight): they're shown by pinning their window;
/// the number only makes gamescope in Steam mode (-e) willing to show them at all.
pub const APPS_ID: u32 = 0x8000_0003;
/// Numbers up to here are Hearth's own; games never get them.
pub const RESERVED_MAX: u32 = 0x8000_00FF;
/// The environment variable every process of a game Hearth started carries.
pub const ENV: &str = "HEARTH_GAME_ID";
/// Steam app ids are below this; Hearth's own numbers are at or above it.
pub const OWN_MIN: u32 = 0x8000_0000;

/// "steam:620" → 620.
pub fn steam_app_id(library_id: &str) -> Option<u32> {
    library_id.strip_prefix("steam:").and_then(|s| s.parse::<u32>().ok()).filter(|n| *n > 0 && *n < OWN_MIN)
}
/// A Steam app id (not one of Hearth's numbers).
pub fn is_steam(id: u32) -> bool { id > 0 && id < OWN_MIN }
fn own(n: u64) -> bool { n > RESERVED_MAX as u64 && n <= u32::MAX as u64 }

fn fnv(bytes: &[u8], salt: u32) -> u32 {
    let mut h: u32 = 0x811c_9dc5 ^ salt.wrapping_mul(0x9e37_79b9);
    for b in bytes { h ^= *b as u32; h = h.wrapping_mul(0x0100_0193); }
    h
}
/// The number a library id would get on its `attempt`-th try (0 first; later tries after a collision).
pub fn candidate(library_id: &str, attempt: u32) -> u32 {
    let h = fnv(library_id.as_bytes(), attempt) | OWN_MIN;
    if h <= RESERVED_MAX { h + 0x100 } else { h }
}

/// The id for a library id, given the ones handed out so far (library.json "appIds"). Steam games keep their Steam
/// app id (never stored). Returns (id, whether it's new and should be saved).
pub fn assign(known: &Map<String, Value>, library_id: &str) -> (u32, bool) {
    if let Some(n) = steam_app_id(library_id) { return (n, false); }
    if let Some(n) = known.get(library_id).and_then(|v| v.as_u64()).filter(|n| own(*n)) { return (n as u32, false); }
    let used: HashSet<u64> = known.iter().filter(|(k, _)| k.as_str() != library_id).filter_map(|(_, v)| v.as_u64()).collect();
    let mut attempt = 0u32;
    loop {
        let c = candidate(library_id, attempt);
        if !used.contains(&(c as u64)) { return (c, true); }
        attempt += 1;
    }
}

/// The environment entry that marks a game's processes.
pub fn env(id: u32) -> (String, String) { (ENV.to_string(), id.to_string()) }

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn ids_are_stable_collision_free_and_leave_steam_alone() {
        let mut known = Map::new();
        // Steam games: their own app id, never stored.
        assert_eq!(assign(&known, "steam:620"), (620, false));
        assert_eq!(assign(&known, "steam:abc").0 >= OWN_MIN, true);   // not a number: treated like any other game
        // Everything else: high bit set, above Hearth's own, the same every time.
        let (a, new) = assign(&known, "epic:Fortnite");
        assert!(new && a > RESERVED_MAX && !is_steam(a));
        assert_eq!(assign(&known, "epic:Fortnite").0, a);
        known.insert("epic:Fortnite".into(), json!(a));
        assert_eq!(assign(&known, "epic:Fortnite"), (a, false));
        // Another game whose first choice is taken gets the next free one.
        let b0 = candidate("gog:1207658924", 0);
        let mut taken = Map::new();
        taken.insert("someone:else".into(), json!(b0));
        let (b, _) = assign(&taken, "gog:1207658924");
        assert_ne!(b, b0);
        assert_eq!(b, candidate("gog:1207658924", 1));
        // A stored number that isn't one of Hearth's (damaged file, an old Steam id) is replaced.
        let mut bad = Map::new();
        bad.insert("manual:x".into(), json!(620));
        let (c, new) = assign(&bad, "manual:x");
        assert!(new && c > RESERVED_MAX);
        // Many games: all different, none reserved, none a Steam id.
        let mut all = Map::new();
        for i in 0..2000 {
            let id = format!("manual:{i}");
            let (n, _) = assign(&all, &id);
            assert!(n > RESERVED_MAX && n != HEARTH_ID && n != OVERLAY_ID);
            all.insert(id, json!(n));
        }
        let distinct: HashSet<u64> = all.values().filter_map(|v| v.as_u64()).collect();
        assert_eq!(distinct.len(), 2000);
        assert_eq!(steam_app_id("steam:0"), None);
        assert_eq!(steam_app_id("epic:620"), None);
        assert_eq!(env(a).0, "HEARTH_GAME_ID");
    }
}
