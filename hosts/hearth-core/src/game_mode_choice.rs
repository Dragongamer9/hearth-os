//! Which Game Mode this PC goes into (docs/HEARTH-GAME-MODE.md, Hearth 1.10): Steam Game Mode (the default) or Hearth
//! Game Mode. Hearth Game Mode's Settings › System shows the same two cards as Mantel's Settings › Gaming. The choice has
//! one source of truth and one writer, desktop/shell/hearth-game-mode (Python), which keeps it as one line of
//! ~/.config/nimbus/session.conf (HEARTH_GAME_MODE_CHOICE, through hearth_settings_io: atomic, .bak, under the lock);
//! nimbus-session reads it every time Game Mode starts. Hearth never changes it by itself.

use serde_json::{json, Value};
use std::process::{Command, Stdio};

/// The two choices, and nothing else is ever passed on to the helper.
pub fn valid(choice: &str) -> bool { matches!(choice, "steam" | "hearth") }

/// Runs hearth-game-mode with these arguments: its JSON answer, or its "error: ..." in plain words.
pub fn run(args: &[&str]) -> Result<Value, String> {
    let helper = crate::paths::desktop_dir().join("shell/hearth-game-mode");
    if !helper.exists() { return Err("Choosing the Game Mode needs Hearth, the OS.".into()); }
    let out = Command::new("python3").arg(&helper).args(args)
        .stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped())
        .output().map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&out.stdout);
    let v = crate::admin::parse_helper(&text)?;
    if !out.status.success() && v.is_null() {
        return Err(String::from_utf8_lossy(&out.stderr).lines().last().unwrap_or("It didn't work.").to_string());
    }
    Ok(v)
}

/// `gameMode.get`: {choice, saved, hearth: {ok, why}, steam: {ok, why}, now, fallback, paused, live}.
pub fn get() -> Result<Value, String> { run(&["get"]) }

/// `gameMode.set` {choice: "steam" | "hearth"}: saved for the next time Game Mode starts.
pub fn set(args: &Value) -> Result<Value, String> {
    let c = args["choice"].as_str().unwrap_or("");
    if !valid(c) { return Err("Pick Steam Game Mode or Hearth Game Mode".into()); }
    let v = run(&["set", c])?;
    crate::bridge::log(&format!("game mode: you picked {} (applies the next time Game Mode starts)", if c == "hearth" { "Hearth Game Mode" } else { "Steam Game Mode" }));
    Ok(v)
}

/// `gameMode.setSteamGames` {on}: "Play Steam games in Steam Game Mode" (on unless turned off; written only when flipped).
pub fn set_steam_games(args: &Value) -> Result<Value, String> {
    let on = args["on"].as_bool().ok_or("on or off")?;
    run(&["steam-games", if on { "on" } else { "off" }])
}

/// A Steam game's app id from a library id ("steam:570"): only digits, at most 10; anything else isn't one.
pub fn steam_app_id(id: &str) -> Option<&str> {
    id.strip_prefix("steam:").filter(|a| !a.is_empty() && a.len() <= 10 && a.bytes().all(|b| b.is_ascii_digit()))
}

/// Whether Play on this game should open Steam Game Mode, from the helper's answer: Hearth Game Mode is the one you
/// picked and the one running, the switch is on, Steam Game Mode can run and isn't paused.
pub fn steam_game_ok(st: &Value, in_hearth_gm: bool) -> Result<(), String> {
    if !in_hearth_gm { return Err("Only from Hearth Game Mode".into()); }
    if st["choice"] != "hearth" || st["live"] == true { return Err("Hearth Game Mode isn't your Game Mode".into()); }
    if st["steamGames"] == false { return Err("Play Steam games in Steam Game Mode is off".into()); }
    if st["steam"]["ok"] == false { return Err(st["steam"]["why"].as_str().unwrap_or("Steam Game Mode can't run here").to_string()); }
    Ok(())
}

/// The one-shot request the session reads and deletes ($XDG_RUNTIME_DIR/hearth-steam-game): the app id, written
/// whole (a temporary file renamed over it), then "steamgame" for the session. Never session.conf.
pub fn write_steam_request(run_dir: &std::path::Path, app: &str) -> Result<(), String> {
    let tmp = run_dir.join(".hearth-steam-game.tmp");
    std::fs::write(&tmp, format!("{app}\n")).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, run_dir.join("hearth-steam-game")).map_err(|e| e.to_string())?;
    std::fs::write(run_dir.join("nimbus-next"), "steamgame\n").map_err(|e| e.to_string())
}

/// What `gameMode.switchNow` asks the session for: Hearth Game Mode by name ("hearth": also ends its pause after two
/// failures, as Try again should) or the Game Mode you picked ("game").
pub fn switch_target(args: &Value) -> &'static str { if args["to"] == "hearth" { "hearth" } else { "game" } }

/// From a Hearth window in Desktop Mode: the switch through nimbus-mode (it closes Desktop Mode first).
pub fn nimbus_mode(to: &str) -> Result<(), String> {
    if !crate::gamescope::in_session() { return Err("Game Mode is part of the Hearth session, which isn't running".into()); }
    let mut last = String::new();
    for cmd in ["nimbus-mode", "/usr/bin/nimbus-mode", "/usr/local/bin/nimbus-mode"] {
        match Command::new(cmd).arg(to).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn() {
            Ok(mut child) => { std::thread::spawn(move || { let _ = child.wait(); }); return Ok(()); }
            Err(e) => last = e.to_string(),
        }
    }
    Err(format!("Couldn't switch: {last}"))
}

/// Whether the choice can be offered here: inside Hearth's session on Hearth OS, where the helper is.
pub fn available() -> bool {
    cfg!(target_os = "linux") && crate::paths::desktop_dir().join("shell/hearth-game-mode").is_file()
}

/// For Send logs: the choice and the last fallback, from the same helper (no settings are written).
pub fn report() -> String {
    let v = match get() {
        Ok(v) => json!({"choice": v["choice"], "saved": v["saved"], "fallback": v["fallback"], "paused": v["paused"], "hearth": v["hearth"], "steam": v["steam"]}),
        Err(e) => json!({"error": e}),
    };
    serde_json::to_string_pretty(&v).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_two_choices() {
        assert!(valid("steam") && valid("hearth"));
        for bad in ["", "Hearth", "desktop", "steam;rm", "hearth\n"] { assert!(!valid(bad), "{bad:?}"); }
        assert!(set(&json!({"choice": "desktop"})).is_err());
        assert!(set(&json!({})).is_err());
    }

    #[test]
    fn steam_games_request() {
        assert_eq!(steam_app_id("steam:570"), Some("570"));
        for bad in ["steam:", "steam:57a", "epic:570", "steam:570;rm", "steam:12345678901", "570"] { assert_eq!(steam_app_id(bad), None, "{bad}"); }
        let ok = json!({"choice": "hearth", "live": false, "steamGames": true, "steam": {"ok": true, "why": ""}});
        assert!(steam_game_ok(&ok, true).is_ok());
        assert!(steam_game_ok(&ok, false).is_err(), "not from a Hearth window in Desktop Mode");
        let mut v = ok.clone(); v["steamGames"] = json!(false); assert!(steam_game_ok(&v, true).is_err());
        let mut v = ok.clone(); v["choice"] = json!("steam"); assert!(steam_game_ok(&v, true).is_err());
        let mut v = ok.clone(); v["steam"] = json!({"ok": false, "why": "Steam isn't installed"});
        assert_eq!(steam_game_ok(&v, true).unwrap_err(), "Steam isn't installed");
        let dir = tempfile::tempdir().unwrap();
        write_steam_request(dir.path(), "570").unwrap();
        assert_eq!(std::fs::read_to_string(dir.path().join("hearth-steam-game")).unwrap(), "570\n");
        assert_eq!(std::fs::read_to_string(dir.path().join("nimbus-next")).unwrap(), "steamgame\n");
        assert!(!dir.path().join(".hearth-steam-game.tmp").exists());
    }

    #[test]
    fn switch_now_only_names_a_game_mode() {
        assert_eq!(switch_target(&json!({"to": "hearth"})), "hearth");
        assert_eq!(switch_target(&json!({"to": "steam"})), "game");
        assert_eq!(switch_target(&json!({"to": "desktop; rm -rf ~"})), "game");
        assert_eq!(switch_target(&json!({})), "game");
    }
}
