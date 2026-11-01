//! Steam Game Mode (Experimental, docs/STEAM-GAME-MODE.md): Valve's Steam gamepad UI in gamescope, the way SteamOS runs
//! it, as a one-time switch from Hearth's Game Mode (the power menu, or Settings › System). nimbus-session runs it
//! (`run_steam_mode`) and always comes back to Hearth's Game Mode afterwards: the choice is never saved, so a restart
//! always starts Hearth's Game Mode, and a Steam that's missing or fails right away falls back to it at once.
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

/// The reason Steam Game Mode fell back to Hearth's last time this sign-in, if it did.
pub fn last_failure(run_dir: &Path) -> Option<String> {
    std::fs::read_to_string(run_dir.join(FAILED_NOTE)).ok().map(|t| t.trim().to_string()).filter(|t| !t.is_empty())
}

/// Settings › System › Steam Game Mode: {available, steam, decky: {...}, lastFailure}.
pub fn status() -> Value {
    let home = crate::paths::home();
    json!({
        "available": available(),
        "steam": steam_kind(&home, crate::system::has("steam")),
        "gamescope": crate::system::has("gamescope"),
        "inSession": in_session(),
        "decky": decky_status(Path::new("/"), &home),
        "lastFailure": last_failure(&runtime_dir()),
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
    crate::bridge::log("steam game mode: switching (one time; Hearth's Game Mode comes back when Steam's ends)");
    Ok(())
}
pub fn switch() -> Result<(), String> { switch_now(&runtime_dir(), available()) }

#[cfg(test)]
mod tests {
    use super::*;

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
}
