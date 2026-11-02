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
    json!({"id": g.id, "title": g.title, "source": g.source, "cover": art("cover").or_else(|| art("portrait")), "wide": art("wide").or_else(|| art("hero")),
           "hero": art("hero").or_else(|| art("wide")), "logo": art("logo"), "icon": art("icon"), "cloud": g.cloud})
}

/// The installed games you can play from the desktop (not hidden ones, not cloud games), A to Z.
pub fn list(lib: &Library) -> Value {
    let mut games: Vec<&Game> = lib.games.values()
        .filter(|g| g.installed && !g.cloud && g.state == "installed" && !lib.store.list_has("hidden", &g.id)).collect();
    games.sort_by_key(|g| g.title.to_lowercase());
    Value::Array(games.into_iter().map(entry).collect())
}

/// `--list-games --with-cloud` (Hearth Sync, hosts/decky-hearth-sync): the same, plus the installed cloud tiles (Xbox
/// Cloud Gaming; not each catalog title, which isn't installed), which play in Hearth's window.
pub fn list_with_cloud(lib: &Library) -> Value {
    let mut games: Vec<&Game> = lib.games.values()
        .filter(|g| g.installed && (g.cloud || g.state == "installed") && !lib.store.list_has("hidden", &g.id)).collect();
    games.sort_by_key(|g| g.title.to_lowercase());
    Value::Array(games.into_iter().map(entry).collect())
}

/// The library read the way Hearth reads it, without starting anything (no download is resumed, nothing is saved).
pub fn read_library() -> Library {
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
    lib
}

/// `hearth --list-games`: the library read the way Hearth reads it, as JSON.
pub fn list_now() -> Value { list(&read_library()) }
/// `hearth --list-games --with-cloud`.
pub fn list_with_cloud_now() -> Value { list_with_cloud(&read_library()) }

// ---- Steam Game Mode (Experimental, docs/STEAM-GAME-MODE.md): the Hearth Decky plugin (hosts/decky-hearth) adds Hearth's
// games to Steam as non-Steam shortcuts that run `hearth --play-now <id>`. No window opens: the game starts the way the
// library's Play button starts it (Proton, Epic, GOG, Beacon, Battle.net, emulators...), and this process waits until
// it has closed, so Steam shows it as running for as long as it plays and its overlay and focus follow it.

/// Whether `id` can be played this way: an installed game of Hearth's own (not a Steam game: those play from
/// Steam's own library; not a cloud game: Xbox Cloud Gaming needs Hearth's stream window).
pub fn play_check(lib: &Library, id: &str) -> Result<(), String> {
    if !valid_id(id) { return Err("That isn't a game id Hearth knows.".into()); }
    if id.starts_with("steam:") { return Err("Steam games play from Steam's own library.".into()); }
    let g = lib.games.get(id).ok_or("That game isn't in Hearth's library anymore. Sync again from the Hearth plugin.")?;
    if g.cloud { return Err(format!("{} is a cloud game: play it in Hearth's Game Mode.", g.title)); }
    if !g.installed { return Err(format!("{} isn't installed.", g.title)); }
    Ok(())
}

/// What a tick's events mean for `--play-now`: Some(exit code) once the game is over (0) or never started (1).
pub fn play_outcome(events: &[Value]) -> Option<i32> {
    events.iter().find_map(|e| match e["name"].as_str() { Some("game.exited") => Some(0), Some("game.timeout") => Some(1), _ => None })
}

/// Steam's environment for a shortcut, cleaned for Hearth (`--play-now` runs as a Steam shortcut): Steam's runtime
/// libraries (LD_LIBRARY_PATH, its PATH), its preloads (LD_PRELOAD: Steam's overlay library), the Steam Runtime and
/// pressure-vessel settings and Steam's compatibility-tool paths are meant for Steam's own games and break Hearth's
/// program and its Proton/umu launches. Returns (names to remove, names to set). Kept: DISPLAY, WAYLAND_DISPLAY,
/// XDG_*, HOME, the session's D-Bus, gamescope's variables, and SteamGameId/SteamOverlayGameId (Steam's id for the
/// shortcut; a game started by umu gets umu's own anyway). Steam's overlay library goes back in LD_PRELOAD for the game
/// (the wrapper the plugin gives the shortcut sets it aside in HEARTH_STEAM_LD_PRELOAD) unless HEARTH_STEAM_OVERLAY=0:
/// gamescope keeps the game in front through Steam's reaper process, which the game still descends from.
pub fn steam_env_scrub(vars: &[(String, String)]) -> (Vec<String>, Vec<(String, String)>) {
    let get = |k: &str| vars.iter().find(|(n, _)| n == k).map(|(_, v)| v.as_str());
    let drop_exact = ["LD_LIBRARY_PATH", "LD_PRELOAD", "SYSTEM_LD_LIBRARY_PATH", "SYSTEM_PATH", "STEAM_RUNTIME", "STEAM_ZENITY",
        "SteamAppId", "SteamClientLaunch", "SteamEnv", "STEAMSCRIPT", "STEAMSCRIPT_VERSION", "STEAM_LD_LIBRARY_PATH",
        "ENABLE_VK_LAYER_VALVE_steam_overlay_1", "ENABLE_VK_LAYER_VALVE_steam_fossilize_1", "DISABLE_VK_LAYER_VALVE_steam_overlay_1",
        "LD_LIBRARY_PATH_ORIG", "HEARTH_STEAM_LD_PRELOAD", "STEAM_COMPAT_FLAGS"];
    let drop_prefix = ["STEAM_RUNTIME_", "PRESSURE_VESSEL_", "STEAM_COMPAT_", "SRT_", "STEAMDECK_RUNTIME"];
    let remove: Vec<String> = vars.iter().map(|(n, _)| n.clone())
        .filter(|n| drop_exact.contains(&n.as_str()) || drop_prefix.iter().any(|p| n.starts_with(p))).collect();
    let mut set = Vec::new();
    // PATH: the system's when Steam kept it (SYSTEM_PATH), else Steam's own entries taken out; the usual folders always there.
    let path = get("SYSTEM_PATH").filter(|p| !p.is_empty()).or(get("PATH")).unwrap_or("");
    let mut parts: Vec<&str> = path.split(':').filter(|p| !p.is_empty() && !p.contains("steam-runtime") && !p.contains("/ubuntu12_") && !p.contains("pressure-vessel")).collect();
    for d in ["/usr/local/bin", "/usr/bin", "/bin"] { if !parts.contains(&d) { parts.push(d); } }
    let clean = parts.join(":");
    if get("PATH") != Some(clean.as_str()) { set.push(("PATH".to_string(), clean)); }
    let overlay: Vec<&str> = get("HEARTH_STEAM_LD_PRELOAD").or(get("LD_PRELOAD")).unwrap_or("")
        .split([':', ' ']).filter(|p| p.contains("gameoverlayrenderer.so")).collect();
    if !overlay.is_empty() && get("HEARTH_STEAM_OVERLAY") != Some("0") { set.push(("LD_PRELOAD".to_string(), overlay.join(":"))); }
    (remove, set)
}

/// `hearth --play-now <id>`: plays the game and returns when it has closed. Returns the exit code (0 played, 1 it
/// couldn't start, 2 not a game this can play). Every step and failure goes to Hearth's log with the reason.
pub fn play_now(id: &str) -> i32 {
    let vars: Vec<(String, String)> = std::env::vars().collect();
    let (remove, set) = steam_env_scrub(&vars);
    crate::bridge::log(&format!("play-now: {id}: starting (from Steam's Game Mode; Steam's game id {}; cleaned {} of Steam's variables{}{})",
        std::env::var("SteamGameId").unwrap_or_else(|_| "none".into()), remove.len(),
        if remove.is_empty() { String::new() } else { format!(": {}", remove.iter().take(12).cloned().collect::<Vec<_>>().join(" ")) },
        if set.iter().any(|(k, _)| k == "LD_PRELOAD") { "; Steam's overlay kept for the game" } else { "" }));
    for k in &remove { std::env::remove_var(k); }
    for (k, v) in &set { std::env::set_var(k, v); }
    let mut lib = read_library();
    if let Err(e) = play_check(&lib, id) { eprintln!("error: {e}"); crate::bridge::log(&format!("play-now: {id}: not started: {e}")); return 2; }
    let g = &lib.games[id];
    crate::bridge::log(&format!("play-now: {id} ({}, {}) launching", g.title, g.source));
    match lib.launch(id) {
        Err(e) => { eprintln!("error: {e}"); crate::bridge::log(&format!("play-now: {id} couldn't start: {e}")); return 1; }
        Ok(v) => crate::bridge::log(&format!("play-now: {id} launched ({v}); running: {}", lib.running.is_some())),
    }
    let started = std::time::Instant::now();
    loop {
        std::thread::sleep(std::time::Duration::from_secs(2));
        if let Some(code) = play_outcome(&lib.tick_game()) {
            crate::bridge::log(&format!("play-now: {id} {} after {}s", if code == 0 { "closed" } else { "never started (no game process appeared)" }, started.elapsed().as_secs()));
            return code;
        }
        if lib.running.is_none() { crate::bridge::log(&format!("play-now: {id}: Hearth isn't tracking it any more after {}s; done", started.elapsed().as_secs())); return 0; }
    }
}
// ---- end Steam Game Mode

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
    fn steams_environment_is_cleaned_for_hearth() {
        let v = |pairs: &[(&str, &str)]| pairs.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect::<Vec<_>>();
        let env = v(&[("HOME", "/home/me"), ("DISPLAY", ":0"), ("XDG_RUNTIME_DIR", "/run/user/1000"), ("GAMESCOPE_WAYLAND_DISPLAY", "gamescope-0"),
            ("LD_LIBRARY_PATH", "/home/me/.steam/steam/ubuntu12_32/steam-runtime/usr/lib"), ("LD_PRELOAD", "/home/me/.steam/steam/ubuntu12_32/gameoverlayrenderer.so:/home/me/.steam/steam/ubuntu12_64/gameoverlayrenderer.so"),
            ("STEAM_RUNTIME", "/x"), ("STEAM_RUNTIME_LIBRARY_PATH", "/x"), ("PRESSURE_VESSEL_FILESYSTEMS_RO", "/y"), ("STEAM_COMPAT_CLIENT_INSTALL_PATH", "/s"),
            ("SteamAppId", "0"), ("SteamGameId", "13416523480334614528"), ("PATH", "/home/me/.steam/steam/ubuntu12_32/steam-runtime/usr/bin:/usr/bin"), ("SYSTEM_PATH", "")]);
        let (remove, set) = steam_env_scrub(&env);
        for k in ["LD_LIBRARY_PATH", "LD_PRELOAD", "STEAM_RUNTIME", "STEAM_RUNTIME_LIBRARY_PATH", "PRESSURE_VESSEL_FILESYSTEMS_RO", "STEAM_COMPAT_CLIENT_INSTALL_PATH", "SteamAppId", "SYSTEM_PATH"] {
            assert!(remove.contains(&k.to_string()), "{k} removed: {remove:?}");
        }
        for k in ["HOME", "DISPLAY", "XDG_RUNTIME_DIR", "GAMESCOPE_WAYLAND_DISPLAY", "SteamGameId"] { assert!(!remove.contains(&k.to_string()), "{k} kept"); }
        let get = |k: &str| set.iter().find(|(n, _)| n == k).map(|(_, v)| v.clone());
        assert_eq!(get("PATH").as_deref(), Some("/usr/bin:/usr/local/bin:/bin"), "Steam's runtime folders out of PATH");
        assert_eq!(get("LD_PRELOAD").as_deref(), Some("/home/me/.steam/steam/ubuntu12_32/gameoverlayrenderer.so:/home/me/.steam/steam/ubuntu12_64/gameoverlayrenderer.so"),
            "only Steam's overlay goes back, for the game");
        // The plugin's wrapper set the preload aside; the system PATH Steam kept is used; the overlay can be turned off.
        let env2 = v(&[("HEARTH_STEAM_LD_PRELOAD", "/evil/libx.so /s/ubuntu12_64/gameoverlayrenderer.so"), ("PATH", "/steam/bin"), ("SYSTEM_PATH", "/usr/local/bin:/usr/bin:/bin:/usr/games")]);
        let (r2, s2) = steam_env_scrub(&env2);
        assert!(r2.contains(&"HEARTH_STEAM_LD_PRELOAD".to_string()));
        assert!(s2.contains(&("LD_PRELOAD".to_string(), "/s/ubuntu12_64/gameoverlayrenderer.so".to_string())), "{s2:?}");
        assert!(s2.contains(&("PATH".to_string(), "/usr/local/bin:/usr/bin:/bin:/usr/games".to_string())), "{s2:?}");
        let (_, s3) = steam_env_scrub(&v(&[("LD_PRELOAD", "/s/gameoverlayrenderer.so"), ("HEARTH_STEAM_OVERLAY", "0"), ("PATH", "/usr/local/bin:/usr/bin:/bin")]));
        assert!(s3.is_empty(), "nothing to change: {s3:?}");
    }

    #[test]
    fn play_now_checks_and_outcome() {
        let dir = tempfile::tempdir().unwrap();
        let store = crate::store::Store::open(dir.path().join("library.json"));
        let mut lib = Library::new(store, dir.path().to_path_buf());
        let mk = |id: &str, installed: bool, cloud: bool| Game { id: id.into(), title: id.into(), source: "manual", installed, cloud,
            state: if installed { "installed" } else { "notInstalled" }, ..Default::default() };
        lib.games.insert("manual:a".into(), mk("manual:a", true, false));
        lib.games.insert("epic:b".into(), mk("epic:b", false, false));
        lib.games.insert("xcloud:c".into(), mk("xcloud:c", false, true));
        assert!(play_check(&lib, "manual:a").is_ok());
        assert!(play_check(&lib, "epic:b").unwrap_err().contains("isn't installed"));
        assert!(play_check(&lib, "xcloud:c").unwrap_err().contains("cloud"));
        assert!(play_check(&lib, "steam:570").unwrap_err().contains("Steam's own library"));
        assert!(play_check(&lib, "manual:gone").is_err());
        assert!(play_check(&lib, "nope").is_err());
        assert_eq!(play_outcome(&[json!({"name": "game.started"})]), None);
        assert_eq!(play_outcome(&[json!({"name": "game.started"}), json!({"name": "game.exited"})]), Some(0));
        assert_eq!(play_outcome(&[json!({"name": "game.timeout"})]), Some(1));
        assert_eq!(play_outcome(&[]), None);
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
        assert_eq!(l[0]["hero"], "https://example.com/h.jpg");
        assert!(l[0]["logo"].is_null());
        assert_eq!(l[0]["cloud"], false);
        // Hearth Sync's list: the installed cloud tile too, never a catalog title that isn't installed.
        let mut title = mk("xbox:halo", "Halo (cloud)", false);
        title.cloud = true;
        title.state = "cloud";
        lib.games.insert(title.id.clone(), title);
        let w = list_with_cloud(&lib);
        let ids: Vec<&str> = w.as_array().unwrap().iter().map(|g| g["id"].as_str().unwrap()).collect();
        assert_eq!(ids, ["manual:a", "manual:b", "xbox:cloud"]);
        assert_eq!(w[2]["cloud"], true);
    }
}
