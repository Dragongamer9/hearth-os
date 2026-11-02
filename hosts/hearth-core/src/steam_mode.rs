//! Steam Game Mode (docs/STEAM-GAME-MODE.md): Valve's Steam gamepad UI in gamescope, the way SteamOS runs it. Hearth OS is
//! Debian + Mantel (Desktop Mode) + Steam Game Mode: nimbus-session starts in it and every "Game Mode" request opens it
//! (`run_steam_mode`); when it can't start or fails twice the session opens Desktop Mode instead, with the reason in
//! FAILED_NOTE (Mantel shows it with "Try Steam Game Mode again"). Hearth's own Game Mode is archived (first-time setup,
//! the live USB's installer, or the developer switch HEARTH_LEGACY_GAME_MODE=1). The old per-device "Start in Steam Game
//! Mode" setting (`set_default`) isn't offered any more: starting in it is always on; the session's pause after two
//! failures uses the same DEFAULT_OFF_NOTE / DEFAULT_FAILS files.
//!
//! With Decky Loader (installed from Settings › System › Steam Game Mode, hearth-admin decky-install), the Hearth Decky
//! plugin (hosts/decky-hearth) brings Hearth into Steam: your other stores' games with Play buttons, and a sync that adds
//! them to Steam's library as shortcuts which play through Hearth (`hearth --play-now`, desktop_games.rs).

use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// The Hearth Decky plugin as the image ships it (built by CI from hosts/decky-hearth).
pub const SHIPPED_PLUGIN: &str = "usr/share/hearth/decky-hearth";
/// Hearth's own Decky Loader service (hearth-decky, through hearth-admin): started only for Steam Game Mode.
pub const HEARTH_UNIT: &str = "etc/systemd/system/hearth-decky.service";
/// Decky Loader installed the usual way (its own installer): left alone, it starts by itself.
pub const UPSTREAM_UNIT: &str = "etc/systemd/system/plugin_loader.service";
/// The note nimbus-session leaves when Steam Game Mode couldn't start or ended right away (this sign-in only).
/// Desktop Mode (Mantel's SteamFallback.qml) shows it once with "Try Steam Game Mode again".
pub const FAILED_NOTE: &str = "hearth-steam-mode-failed";

fn runtime_dir() -> PathBuf { PathBuf::from(std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into())) }

/// Which Steam would run: "native" (the system's `steam`, both editions) or "flatpak" (Flathub's), or None.
pub fn steam_kind(home: &Path, has_native: bool) -> Option<&'static str> {
    if has_native { return Some("native"); }
    let flatpak = [PathBuf::from("/var/lib/flatpak/app/com.valvesoftware.Steam"), home.join(".local/share/flatpak/app/com.valvesoftware.Steam")];
    if flatpak.iter().any(|p| p.is_dir()) { Some("flatpak") } else { None }
}

/// Whether the switch can be offered: inside Hearth's session, not an extra multi-seat player, gamescope and a Steam.
pub fn can_offer(in_session: bool, extra_seat: bool, has_gamescope: bool, steam: Option<&str>) -> bool {
    in_session && !extra_seat && has_gamescope && steam.is_some()
}

fn in_session() -> bool { std::env::var("NIMBUS_SESSION").map(|v| v == "1").unwrap_or(false) }
fn extra_seat() -> bool { std::env::var_os("HEARTH_SEAT").is_some_and(|s| !s.is_empty()) }

pub fn available() -> bool {
    cfg!(target_os = "linux") && can_offer(in_session(), extra_seat(), crate::system::has("gamescope"), steam_kind(&crate::paths::home(), crate::system::has("steam")))
}

/// Decky Loader and the Hearth plugin on this device, read from files only (`root` is "/" normally).
pub fn decky_status(root: &Path, home: &Path) -> Value {
    let hearth = root.join(HEARTH_UNIT).is_file();
    let upstream = root.join(UPSTREAM_UNIT).is_file();
    let cef = [".steam/steam/.cef-enable-remote-debugging", ".var/app/com.valvesoftware.Steam/data/Steam/.cef-enable-remote-debugging"]
        .iter().any(|p| home.join(p).exists());
    json!({
        "installed": hearth || upstream,
        // Hearth's own install (started only for Steam Game Mode) or Decky's own installer's (always running).
        "via": if hearth { json!("hearth") } else if upstream { json!("decky") } else { Value::Null },
        "plugin": home.join("homebrew/plugins/decky-hearth/plugin.json").is_file(),
        "shipped": root.join(SHIPPED_PLUGIN).join("plugin.json").is_file(),
        "cefDebugging": cef,
    })
}

/// Whether this session's Game Mode is Steam Game Mode (nimbus-session's HEARTH_STEAM_ONLY=1): host.info "steamOnly".
pub fn steam_only(var: Option<&str>) -> bool { var == Some("1") }

/// The reason Steam Game Mode fell back to Desktop Mode last time this sign-in, if it did.

pub fn last_failure(run_dir: &Path) -> Option<String> {
    std::fs::read_to_string(run_dir.join(FAILED_NOTE)).ok().map(|t| t.trim().to_string()).filter(|t| !t.is_empty())
}

// ---- Start in Steam Game Mode (Experimental, per device, off by default): HEARTH_STEAM_DEFAULT=1 in session.conf,
// read by nimbus-session at every sign-in (steam_default_wanted), which also turns it off by itself when it can't work
// and leaves the reason in ~/.local/state/nimbus/steam-default-off.
pub const DEFAULT_KEY: &str = "HEARTH_STEAM_DEFAULT";
/// Why the session turned "Start in Steam Game Mode" off by itself (kept until it's turned on again).
pub const DEFAULT_OFF_NOTE: &str = "steam-default-off";
/// The session's count of failed starts (cleared here when it's turned on again).
pub const DEFAULT_FAILS: &str = "steam-default-fails";

fn is_default_line(l: &str) -> bool {
    let t = l.trim_start();
    t.strip_prefix("export").map(str::trim_start).unwrap_or(t).starts_with("HEARTH_STEAM_DEFAULT=")
}
/// HEARTH_STEAM_DEFAULT from session.conf: on only when it's exactly 1 (the last line wins, as in the shell).
pub fn read_default(text: &str) -> bool {
    let mut on = false;
    for l in text.lines().filter(|l| is_default_line(l)) {
        let v = l.trim_start().trim_start_matches("export").trim_start().trim_start_matches("HEARTH_STEAM_DEFAULT=");
        on = v.trim().trim_matches(|c| c == '"' || c == '\'') == "1";
    }
    on
}
/// Sets HEARTH_STEAM_DEFAULT to 1 or 0, keeping everything else in the file.
pub fn write_default(text: &str, on: bool) -> String {
    let line = format!("{DEFAULT_KEY}={}", if on { 1 } else { 0 });
    if text.lines().any(is_default_line) {
        let mut out: Vec<String> = text.lines().map(|l| if is_default_line(l) { line.clone() } else { l.to_string() }).collect();
        out.push(String::new());
        out.join("\n")
    } else {
        format!("{}{}{line}\n", text, if !text.is_empty() && !text.ends_with('\n') { "\n" } else { "" })
    }
}

fn state_dir() -> PathBuf {
    std::env::var_os("XDG_STATE_HOME").map(PathBuf::from).unwrap_or_else(|| crate::paths::home().join(".local/state")).join("nimbus")
}

/// Whether Steam has a signed-in account (loginusers.vdf) in the folder of the Steam that nimbus-session will start,
/// with the session's own rules (steam_flatpak_in_use, run_steam_mode): Flathub's Steam when it's signed in and newer
/// than the system Steam's folder (or that has no sign-in), or when it's the only Steam / Hearth's stand-in starts it.
pub fn signed_in(home: &Path, has_native: bool, flatpak_installed: bool, flatpak_standin: bool) -> bool {
    let nonempty = |p: &PathBuf| std::fs::metadata(p).map(|m| m.is_file() && m.len() > 0).unwrap_or(false);
    let fl = [".var/app/com.valvesoftware.Steam/.local/share/Steam/config/loginusers.vdf", ".var/app/com.valvesoftware.Steam/data/Steam/config/loginusers.vdf"]
        .iter().map(|p| home.join(p)).find(|p| nonempty(p));
    let nl = home.join(".local/share/Steam/config/loginusers.vdf");
    let mtime = |p: &Path| std::fs::metadata(p).and_then(|m| m.modified()).ok();
    let flatpak_in_use = flatpak_installed && fl.as_ref().is_some_and(|f| !nonempty(&nl) || mtime(f.as_path()) > mtime(nl.as_path()));
    if flatpak_in_use { return true; }
    if has_native && (!flatpak_standin || !flatpak_installed) {
        return nonempty(&nl) || nonempty(&home.join(".steam/steam/config/loginusers.vdf"));
    }
    fl.is_some()
}

fn flatpak_installed(home: &Path) -> bool {
    Path::new("/var/lib/flatpak/app/com.valvesoftware.Steam").is_dir() || home.join(".local/share/flatpak/app/com.valvesoftware.Steam").is_dir()
}
/// The live USB / installer never starts in Steam Game Mode.
fn live() -> bool { Path::new("/etc/hearth-live").exists() }

/// Whether "Start in Steam Game Mode" can be turned on: where the switch can be offered, Steam signed in, not the live USB.
pub fn can_default(available: bool, signed_in: bool, live: bool) -> bool { available && signed_in && !live }

fn steam_signed_in_here() -> bool {
    let home = crate::paths::home();
    signed_in(&home, crate::system::has("steam"), crate::system::has("flatpak") && flatpak_installed(&home), Path::new("/usr/share/nimbus/steam-flatpak").exists())
}

/// {on, can, signedIn, offNote}: the setting as the session will see it.
pub fn default_status_in(conf: &Path, state: &Path, can: bool, signed_in: bool) -> Value {
    let on = read_default(&std::fs::read_to_string(conf).unwrap_or_default());
    let note = std::fs::read_to_string(state.join(DEFAULT_OFF_NOTE)).ok().map(|t| t.trim().to_string()).filter(|t| !t.is_empty());
    json!({ "on": on, "can": can, "signedIn": signed_in, "offNote": if on { Value::Null } else { json!(note) } })
}

/// Turns "Start in Steam Game Mode" on or off. Turning it on is refused where it can't work; it clears the session's
/// old note and failure count, so it gets its two chances again.
pub fn set_default_in(conf: &Path, state: &Path, on: bool, can: bool) -> Result<(), String> {
    if on && !can { return Err("Starting in Steam Game Mode needs Steam installed and signed in, in Hearth OS's session.".into()); }
    let text = std::fs::read_to_string(conf).unwrap_or_default();
    crate::paths::write_atomic(conf, write_default(&text, on).as_bytes()).map_err(|e| e.to_string())?;
    if on {
        let _ = std::fs::remove_file(state.join(DEFAULT_OFF_NOTE));
        let _ = std::fs::remove_file(state.join(DEFAULT_FAILS));
    }
    crate::bridge::log(&format!("steam game mode: start in Steam Game Mode {}", if on { "on" } else { "off" }));
    Ok(())
}

pub fn set_default(on: bool) -> Result<Value, String> {
    let signed = steam_signed_in_here();
    set_default_in(&crate::admin::session_conf(), &state_dir(), on, can_default(available(), signed, live()))?;
    Ok(status())
}

/// Settings › System › Steam Game Mode: {available, steam, decky: {...}, lastFailure, startDefault: {...}}.
pub fn status() -> Value {
    let home = crate::paths::home();
    let avail = available();
    let signed = avail && steam_signed_in_here();
    json!({
        "available": avail,
        "steam": steam_kind(&home, crate::system::has("steam")),
        "gamescope": crate::system::has("gamescope"),
        "inSession": in_session(),
        "decky": decky_status(Path::new("/"), &home),
        "lastFailure": last_failure(&runtime_dir()),
        "startDefault": default_status_in(&crate::admin::session_conf(), &state_dir(), can_default(avail, signed, live()), signed),
    })
}

/// What Settings may ask hearth-admin to do with Decky Loader.
pub fn decky_action_ok(action: &str) -> bool { ["install", "remove"].contains(&action) }

/// Installs or removes Decky Loader and the Hearth plugin (hearth-admin decky-install asks for an administrator's
/// password: it adds a service that runs as root). Returns the status after.
pub fn decky(action: &str) -> Result<Value, String> {
    if !decky_action_ok(action) { return Err("Unknown choice".into()); }
    crate::bridge::log(&format!("steam game mode: decky {action}"));
    let verb = format!("decky-{action}");
    crate::admin::run(&[verb.as_str()], None, &mut |l| crate::bridge::log(&format!("decky: {l}")))?;
    Ok(status())
}

/// The power menu's "Steam Game Mode": the session reads the next mode from nimbus-next when Hearth closes. Refused
/// when it can't run (nothing is written, Hearth stays).
pub fn switch_now(run_dir: &Path, can: bool) -> Result<(), String> {
    if !can { return Err("Steam Game Mode needs Steam and Hearth's session (Hearth OS).".into()); }
    let _ = std::fs::remove_file(run_dir.join(FAILED_NOTE));
    std::fs::write(run_dir.join("nimbus-next"), "steam\n").map_err(|e| e.to_string())?;
    crate::bridge::log("steam game mode: switching");
    Ok(())
}
pub fn switch() -> Result<(), String> { switch_now(&runtime_dir(), available()) }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steam_only_only_when_the_session_says_so() {
        assert!(steam_only(Some("1")));
        assert!(!steam_only(None));
        assert!(!steam_only(Some("0")));
    }

    #[test]
    fn offered_only_where_it_can_run() {
        assert!(can_offer(true, false, true, Some("native")));
        assert!(can_offer(true, false, true, Some("flatpak")));
        assert!(!can_offer(false, false, true, Some("native")), "outside the session");
        assert!(!can_offer(true, true, true, Some("native")), "extra multi-seat player");
        assert!(!can_offer(true, false, false, Some("native")), "no gamescope");
        assert!(!can_offer(true, false, true, None), "no Steam");
    }

    #[test]
    fn steam_kind_prefers_the_system_steam() {
        let d = tempfile::tempdir().unwrap();
        assert_eq!(steam_kind(d.path(), true), Some("native"));
        if !Path::new("/var/lib/flatpak/app/com.valvesoftware.Steam").is_dir() {
            assert_eq!(steam_kind(d.path(), false), None);
            std::fs::create_dir_all(d.path().join(".local/share/flatpak/app/com.valvesoftware.Steam")).unwrap();
            assert_eq!(steam_kind(d.path(), false), Some("flatpak"));
        }
    }

    #[test]
    fn decky_status_from_files() {
        let root = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let s = decky_status(root.path(), home.path());
        assert_eq!(s["installed"], false);
        assert!(s["via"].is_null());
        assert_eq!(s["plugin"], false);
        assert_eq!(s["shipped"], false);
        assert_eq!(s["cefDebugging"], false);
        let mk = |p: PathBuf| { std::fs::create_dir_all(p.parent().unwrap()).unwrap(); std::fs::write(p, "{}").unwrap(); };
        mk(root.path().join(UPSTREAM_UNIT));
        assert_eq!(decky_status(root.path(), home.path())["via"], "decky");
        mk(root.path().join(HEARTH_UNIT));
        mk(root.path().join(SHIPPED_PLUGIN).join("plugin.json"));
        mk(home.path().join("homebrew/plugins/decky-hearth/plugin.json"));
        mk(home.path().join(".steam/steam/.cef-enable-remote-debugging"));
        let s = decky_status(root.path(), home.path());
        assert_eq!(s["installed"], true);
        assert_eq!(s["via"], "hearth");
        assert_eq!(s["plugin"], true);
        assert_eq!(s["shipped"], true);
        assert_eq!(s["cefDebugging"], true);
    }

    #[test]
    fn switching_writes_the_one_time_next_mode() {
        let run = tempfile::tempdir().unwrap();
        assert!(switch_now(run.path(), false).is_err());
        assert!(!run.path().join("nimbus-next").exists(), "nothing written when it can't run");
        std::fs::write(run.path().join(FAILED_NOTE), "Steam ended right away").unwrap();
        assert_eq!(last_failure(run.path()).as_deref(), Some("Steam ended right away"));
        switch_now(run.path(), true).unwrap();
        assert_eq!(std::fs::read_to_string(run.path().join("nimbus-next")).unwrap(), "steam\n");
        assert_eq!(last_failure(run.path()), None, "an old failure note is cleared on a new try");
        assert!(decky_action_ok("install") && decky_action_ok("remove") && !decky_action_ok("start") && !decky_action_ok("rm -rf"));
    }

    #[test]
    fn start_default_in_session_conf() {
        assert!(!read_default(""));
        assert!(!read_default("NIMBUS_START_MODE=game\n"));
        assert!(read_default("HEARTH_STEAM_DEFAULT=1\n"));
        assert!(read_default("export HEARTH_STEAM_DEFAULT=\"1\"\n"));
        assert!(!read_default("HEARTH_STEAM_DEFAULT=1\nHEARTH_STEAM_DEFAULT=0\n"), "the last line wins, as in the shell");
        assert!(!read_default("HEARTH_STEAM_DEFAULT=yes\n"), "only 1 is on (the session reads it the same way)");
        let t = write_default("NIMBUS_GAMESCOPE_ARGS=\"-W 1280\"", true);
        assert_eq!(t, "NIMBUS_GAMESCOPE_ARGS=\"-W 1280\"\nHEARTH_STEAM_DEFAULT=1\n");
        let t = write_default(&t, false);
        assert!(t.contains("NIMBUS_GAMESCOPE_ARGS=\"-W 1280\"") && t.contains("HEARTH_STEAM_DEFAULT=0") && !t.contains("HEARTH_STEAM_DEFAULT=1"));
        assert!(!read_default(&t));
    }

    #[test]
    fn start_default_offered_only_where_it_can_work() {
        assert!(can_default(true, true, false));
        assert!(!can_default(false, true, false), "no Steam / gamescope / session, or an extra multi-seat player");
        assert!(!can_default(true, false, false), "Steam not signed in");
        assert!(!can_default(true, true, true), "the live USB / installer");
    }

    #[test]
    fn signed_in_follows_the_steam_the_session_starts() {
        let h = tempfile::tempdir().unwrap();
        let mk = |p: &str| { let f = h.path().join(p); std::fs::create_dir_all(f.parent().unwrap()).unwrap(); std::fs::write(f, "\"users\" {}").unwrap(); };
        assert!(!signed_in(h.path(), true, false, false), "no loginusers.vdf");
        mk(".var/app/com.valvesoftware.Steam/.local/share/Steam/config/loginusers.vdf");
        assert!(!signed_in(h.path(), true, false, false), "Flathub's sign-in doesn't count when only the system Steam is installed");
        assert!(signed_in(h.path(), true, true, false), "Flathub's Steam in use (signed in there, none in the system Steam's folder)");
        assert!(signed_in(h.path(), false, true, false), "only Flathub's Steam");
        mk(".local/share/Steam/config/loginusers.vdf");
        assert!(signed_in(h.path(), true, false, false));
        let h2 = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(h2.path().join(".local/share/Steam/config")).unwrap();
        std::fs::write(h2.path().join(".local/share/Steam/config/loginusers.vdf"), "").unwrap();
        assert!(!signed_in(h2.path(), true, false, false), "an empty file isn't a sign-in");
    }

    #[test]
    fn start_default_set_and_status() {
        let d = tempfile::tempdir().unwrap();
        let conf = d.path().join("config/session.conf");
        let state = d.path().join("state");
        assert!(set_default_in(&conf, &state, true, false).is_err());
        assert!(!conf.exists(), "nothing written when it can't work");
        std::fs::create_dir_all(&state).unwrap();
        std::fs::write(state.join(DEFAULT_OFF_NOTE), "Steam ended right away (code 1)\n").unwrap();
        std::fs::write(state.join(DEFAULT_FAILS), "1700000000\n").unwrap();
        let s = default_status_in(&conf, &state, true, true);
        assert_eq!(s["on"], false);
        assert_eq!(s["offNote"], "Steam ended right away (code 1)", "the session's note shows while it's off");
        set_default_in(&conf, &state, true, true).unwrap();
        assert!(read_default(&std::fs::read_to_string(&conf).unwrap()));
        assert!(!state.join(DEFAULT_OFF_NOTE).exists() && !state.join(DEFAULT_FAILS).exists(), "a fresh start: two chances again");
        let s = default_status_in(&conf, &state, true, true);
        assert_eq!(s["on"], true);
        assert!(s["offNote"].is_null());
        set_default_in(&conf, &state, false, false).unwrap();
        assert!(!read_default(&std::fs::read_to_string(&conf).unwrap()), "turning it off always works");
    }
}
