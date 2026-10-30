//! Battle.net (experimental): Blizzard's own Battle.net app, set up with one press in a Proton prefix of its own and
//! started through umu-launcher, the same Proton runner Hearth uses for Epic and GOG games. Battle.net has no
//! open-source downloader like legendary or gogdl, so Hearth runs the real app instead of replacing it.
//!
//! Hearth never signs in to Battle.net and never sees the password: you sign in in Battle.net's own window, and buying,
//! installing and updating Blizzard games happen there too. Hearth only notices the games Battle.net installed (each
//! game folder Battle.net makes has a `.build.info` file) and shows them in the library. Play starts a game through
//! Battle.net (`Battle.net.exe --exec="launch <code>"`), so its sign-in and the game's licence work as on Windows.

use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// The library entry for Battle.net itself.
pub const APP_ID: &str = "battlenet:app";
/// Blizzard's own link for the Battle.net installer (the one battle.net/download hands out).
const INSTALLER: &str = "https://downloader.battle.net/download/getInstallerForGame?os=win&gameProgram=BATTLENET_APP&version=Live";
/// umu's game id: Proton's fixes for Battle.net, when umu's database has them (an unknown id just means none).
const GAMEID: &str = "umu-battlenet";
/// The languages Battle.net's installer takes; anything else installs in English.
const LANGS: [&str; 14] = ["enUS", "enGB", "deDE", "esES", "esMX", "frFR", "itIT", "koKR", "plPL", "ptBR", "ruRU", "zhCN", "zhTW", "jaJP"];

/// Blizzard games Hearth knows: (library id, title, Battle.net's launch code, the folder Battle.net suggests, the
/// product in its .build.info). A game is found by its product, or by its folder for an older .build.info without one.
const GAMES: [(&str, &str, &str, &str, &str); 11] = [
    ("overwatch", "Overwatch 2", "Pro", "Overwatch", "pro"),
    ("wow", "World of Warcraft", "WoW", "World of Warcraft", "wow"),
    ("diablo4", "Diablo IV", "Fen", "Diablo IV", "fenris"),
    ("diablo3", "Diablo III", "D3", "Diablo III", "d3"),
    ("d2r", "Diablo II: Resurrected", "OSI", "Diablo II Resurrected", "osi"),
    ("hearthstone", "Hearthstone", "WTCG", "Hearthstone", "hsb"),
    ("heroes", "Heroes of the Storm", "Hero", "Heroes of the Storm", "hero"),
    ("sc2", "StarCraft II", "S2", "StarCraft II", "s2"),
    ("sc1", "StarCraft: Remastered", "S1", "StarCraft", "s1"),
    ("wc3", "Warcraft III: Reforged", "W3", "Warcraft III", "w3"),
    ("diablo-immortal", "Diablo Immortal", "ANBS", "Diablo Immortal", "anbs"),
];

pub fn dir() -> PathBuf { crate::paths::data_dir().join("battlenet") }
/// The Proton prefix (umu makes drive_c right in it).
pub fn prefix() -> PathBuf { dir().join("prefix") }
/// What the library starts: a small script that runs Battle.net with umu in its prefix (made by Hearth).
pub fn runner() -> PathBuf { dir().join("run.sh") }
fn drive_c() -> PathBuf { prefix().join("drive_c") }
fn app_folder() -> PathBuf { drive_c().join("Program Files (x86)").join("Battle.net") }
fn launcher_exe() -> PathBuf { app_folder().join("Battle.net Launcher.exe") }
/// Written by Battle.net the first time it runs, after its install has finished.
fn config_file() -> PathBuf { drive_c().join("users/steamuser/AppData/Roaming/Battle.net/Battle.net.config") }
fn log_file() -> PathBuf { dir().join("setup.log") }

pub fn installed() -> bool { launcher_exe().is_file() && runner().is_file() }

/// "de_DE.UTF-8" → "deDE": the installer's language from the system's, English when Battle.net doesn't have it.
pub fn lang_of(lang: &str) -> &'static str {
    let code: String = lang.split(['.', '@']).next().unwrap_or("").replace(['_', '-'], "");
    LANGS.iter().find(|l| l.eq_ignore_ascii_case(&code)).copied().unwrap_or("enUS")
}

/// The product named in a .build.info (Blizzard's pipe-separated table: a header row "Name!TYPE:n|…", then rows).
pub fn product_of(build_info: &str) -> Option<String> {
    let mut lines = build_info.lines().filter(|l| !l.trim().is_empty());
    let head: Vec<&str> = lines.next()?.split('|').collect();
    let col = head.iter().position(|h| h.split('!').next().map(|n| n.trim().eq_ignore_ascii_case("product")).unwrap_or(false))?;
    lines.filter_map(|l| l.split('|').nth(col)).map(|p| p.trim().to_lowercase()).find(|p| !p.is_empty())
}

/// Which known game a folder is: by its .build.info's product, else by the folder's name.
fn known(folder_name: &str, product: Option<&str>) -> Option<(&'static str, &'static str, &'static str, &'static str, &'static str)> {
    if let Some(p) = product { if let Some(g) = GAMES.iter().find(|g| g.4 == p) { return Some(*g); } }
    GAMES.iter().find(|g| g.3.eq_ignore_ascii_case(folder_name)).copied()
}

/// Blizzard games Battle.net installed in its prefix: [(library id, title, launch code, folder)]. Looks in the places
/// Battle.net offers (Program Files, Program Files (x86)) and C:\Games, one folder deep.
pub fn find_games(drive_c: &Path) -> Vec<(String, String, String, PathBuf)> {
    let mut out: Vec<(String, String, String, PathBuf)> = Vec::new();
    for root in ["Program Files (x86)", "Program Files", "Games"] {
        for e in std::fs::read_dir(drive_c.join(root)).into_iter().flatten().flatten() {
            let p = e.path();
            let Ok(info) = std::fs::read_to_string(p.join(".build.info")) else { continue };
            let name = e.file_name().to_string_lossy().to_string();
            let Some(g) = known(&name, product_of(&info).as_deref()) else { continue };
            let id = format!("battlenet:{}", g.0);
            if out.iter().any(|x| x.0 == id) { continue; }
            out.push((id, g.1.to_string(), g.2.to_string(), p));
        }
    }
    out
}

/// The library entries: Battle.net itself, and each Blizzard game it installed (only once Battle.net is set up).
pub fn games() -> Vec<crate::library::Game> {
    if !installed() { return vec![]; }
    let exe = runner().to_string_lossy().to_string();
    let mut out = vec![crate::library::Game { id: APP_ID.into(), title: "Battle.net".into(), source: "battlenet", install_path: Some(app_folder()),
        installed: true, state: "installed", exe: Some(exe.clone()), ..Default::default() }];
    for (id, title, code, folder) in find_games(&drive_c()) {
        // One argument, quoted whole: split_args keeps it together ("--exec=launch Pro").
        out.push(crate::library::Game { id, title, source: "battlenet", install_path: Some(folder), installed: true, state: "installed",
            exe: Some(exe.clone()), args: format!("\"--exec=launch {code}\""), ..Default::default() });
    }
    out
}

/// The script the library runs. `umu` is where umu-run is (the system's, or the one Hearth downloaded).
fn run_sh(umu: &str) -> String {
    format!(r#"#!/bin/sh
# Starts Battle.net for Hearth in its own Proton prefix (made by Hearth; replaced when Battle.net is set up again).
here="$(dirname "$(readlink -f "$0")")"
export WINEPREFIX="$here/prefix"
export GAMEID="${{GAMEID:-{gameid}}}"
export STORE="${{STORE:-battlenet}}"
app="$WINEPREFIX/drive_c/Program Files (x86)/Battle.net"
# A game's Play passes --exec="launch <code>": that's Battle.net.exe's option. Battle.net itself opens through its launcher.
exe="$app/Battle.net Launcher.exe"
case "$1" in --exec=*) [ -f "$app/Battle.net.exe" ] && exe="$app/Battle.net.exe" ;; esac
exec {umu} "$exe" "$@"
"#, umu = shell_quote(umu), gameid = GAMEID)
}

/// 'text' for sh, with any ' inside kept.
fn shell_quote(s: &str) -> String { format!("'{}'", s.replace('\'', "'\\''")) }

/// Battle.net draws a black or flickering window in Proton with hardware acceleration on: turn it off, unless you've
/// already chosen (Battle.net's Settings › App).
pub fn without_hw_accel(config: &str) -> Option<String> {
    let mut v: Value = if config.trim().is_empty() { json!({}) } else { serde_json::from_str(config).ok()? };
    if !v.is_object() { return None; }
    if !v["Client"].is_object() { v["Client"] = json!({}); }
    if v["Client"].get("HardwareAcceleration").is_some() { return None; }
    v["Client"]["HardwareAcceleration"] = json!("false");
    serde_json::to_string_pretty(&v).ok()
}

/// Opens Battle.net once Desktop Mode starts (its first sign-in, where the controller types), like Xodus Nexus.
pub fn open_in_desktop() -> Result<(), String> {
    if !installed() { return Err("Set up Battle.net first".into()); }
    let f = PathBuf::from(std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into())).join("hearth-open-after");
    std::fs::write(f, runner().to_string_lossy().as_bytes()).map_err(|e| e.to_string())
}

/// Removes Battle.net's prefix, and with it every Blizzard game installed there.
pub fn remove() -> Result<(), String> {
    if dir().exists() { std::fs::remove_dir_all(dir()).map_err(|e| format!("Couldn't remove it: {e}"))?; }
    Ok(())
}

/// Stops the installer and everything it started (its own process group).
fn stop(child: &mut std::process::Child) {
    let _ = std::process::Command::new("kill").args(["-TERM", &format!("-{}", child.id())]).status();
    for _ in 0..20 { if matches!(child.try_wait(), Ok(Some(_))) { return; } std::thread::sleep(Duration::from_millis(500)); }
    let _ = std::process::Command::new("kill").args(["-KILL", &format!("-{}", child.id())]).status();
    let _ = child.wait();
}

/// Setting up, in the background, telling the page how it's going ("battlenet" events).
pub struct Battlenet { pub state: Arc<Mutex<Value>> }
impl Default for Battlenet { fn default() -> Self { Self::new() } }
impl Battlenet {
    pub fn new() -> Battlenet { Battlenet { state: Arc::new(Mutex::new(json!({"busy": false, "stage": null, "error": null}))) } }

    /// {installed, busy, stage, error, games}
    pub fn status(&self) -> Value {
        let mut s = self.state.lock().unwrap().clone();
        s["installed"] = json!(installed());
        s["games"] = json!(if installed() { find_games(&drive_c()).len() } else { 0 });
        s
    }

    /// One press: umu-launcher (downloaded if needed), Blizzard's installer, then Battle.net installed in its prefix.
    pub fn install(&self, emit: crate::system::bt_scan::Emit) -> Value {
        {
            let mut s = self.state.lock().unwrap();
            if s["busy"] == true { drop(s); return self.status(); }
            s["busy"] = json!(true); s["stage"] = json!("Starting"); s["error"] = Value::Null;
        }
        let state = self.state.clone();
        std::thread::spawn(move || {
            crate::watchdog::long_task();
            let push = |st: &Arc<Mutex<Value>>| { let mut v = st.lock().unwrap().clone(); v["installed"] = json!(installed()); emit(json!({"type": "event", "name": "battlenet", "data": v})); };
            let stage = |st: &Arc<Mutex<Value>>, t: &str| { st.lock().unwrap()["stage"] = json!(t); push(st); };
            let r = (|| -> Result<(), String> {
                stage(&state, "Getting Proton (umu-launcher)…");
                let umu = crate::tools::ensure(&crate::tools::UMU)?;
                std::fs::create_dir_all(prefix()).map_err(|e| e.to_string())?;
                stage(&state, "Downloading Blizzard's installer…");
                let setup = dir().join("Battle.net-Setup.exe");
                let setup_s = setup.to_string_lossy().to_string();
                crate::system::run_timeout("curl", &["-fsSL", "--retry", "3", "-A", "Hearth", "-o", &setup_s, INSTALLER], Duration::from_secs(600))
                    .map_err(|_| "Couldn't download Battle.net's installer. Check the internet connection and try again.".to_string())?;
                use std::os::unix::fs::PermissionsExt;
                std::fs::write(runner(), run_sh(&umu)).map_err(|e| e.to_string())?;
                std::fs::set_permissions(runner(), std::fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;
                stage(&state, "Installing Battle.net… It downloads itself, which can take a few minutes.");
                let lang = format!("--lang={}", lang_of(&std::env::var("LANG").unwrap_or_default()));
                let log = std::fs::File::create(log_file()).map_err(|e| e.to_string())?;
                use std::os::unix::process::CommandExt;
                // Its own process group, so the installer and the Battle.net it opens at the end can be stopped together.
                let mut child = std::process::Command::new(&umu)
                    .args([setup_s.as_str(), lang.as_str(), "--installpath=C:\\Program Files (x86)\\Battle.net"])
                    .env("WINEPREFIX", prefix()).env("GAMEID", GAMEID).env("STORE", "battlenet")
                    .stdin(std::process::Stdio::null()).stdout(log.try_clone().map_err(|e| e.to_string())?).stderr(log)
                    .process_group(0).spawn().map_err(|e| format!("Couldn't start the installer: {e}"))?;
                let started = Instant::now();
                let mut seen: Option<Instant> = None;
                loop {
                    if let Some(_st) = child.try_wait().map_err(|e| e.to_string())? {
                        // umu waits for every program in the prefix, so the installer closing means it's all over.
                        if launcher_exe().is_file() { break; }
                        return Err("Battle.net's installer closed before Battle.net was installed. Try again; the details are in battlenet/setup.log.".into());
                    }
                    if launcher_exe().is_file() { seen.get_or_insert_with(Instant::now); }
                    // Done: Battle.net opened by itself at the end (its settings file appears), or it's been there for 3
                    // minutes. Its window is closed; Hearth opens it from the library, where the sign-in happens.
                    if let Some(t) = seen { if (config_file().is_file() && t.elapsed() > Duration::from_secs(10)) || t.elapsed() > Duration::from_secs(180) { stop(&mut child); break; } }
                    if started.elapsed() > Duration::from_secs(45 * 60) { stop(&mut child); return Err("Battle.net's installer took too long and was stopped. Try again.".into()); }
                    std::thread::sleep(Duration::from_secs(1));
                }
                let _ = std::fs::remove_file(&setup);
                let cfg = std::fs::read_to_string(config_file()).unwrap_or_default();
                if let Some(new) = without_hw_accel(&cfg) {
                    if let Some(p) = config_file().parent() { let _ = std::fs::create_dir_all(p); }
                    let _ = std::fs::write(config_file(), new);
                }
                crate::bridge::log("battlenet: set up");
                Ok(())
            })();
            { let mut s = state.lock().unwrap(); s["busy"] = json!(false); s["stage"] = Value::Null; s["error"] = match r { Ok(()) => Value::Null, Err(e) => json!(e) }; }
            push(&state);
        });
        self.status()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installer_language() {
        assert_eq!(lang_of("de_DE.UTF-8"), "deDE");
        assert_eq!(lang_of("pt_BR"), "ptBR");
        assert_eq!(lang_of("en_US.UTF-8@euro"), "enUS");
        assert_eq!(lang_of("nl_NL.UTF-8"), "enUS");
        assert_eq!(lang_of(""), "enUS");
    }

    #[test]
    fn product_from_build_info() {
        let b = "Branch!STRING:0|Active!DEC:1|Build Key!HEX:16|Version!STRING:0|Product!STRING:0\nus|1|abc|2.0.1|pro\n";
        assert_eq!(product_of(b).as_deref(), Some("pro"));
        assert_eq!(product_of("Branch!STRING:0|Active!DEC:1\nus|1\n"), None);
        assert_eq!(product_of(""), None);
    }

    #[test]
    fn finds_installed_blizzard_games() {
        let d = tempfile::tempdir().unwrap();
        let c = d.path();
        let mk = |p: &str, info: &str| { std::fs::create_dir_all(c.join(p)).unwrap(); std::fs::write(c.join(p).join(".build.info"), info).unwrap(); };
        mk("Program Files (x86)/Overwatch", "Branch!STRING:0|Product!STRING:0\nus|pro\n");
        mk("Program Files (x86)/My Diablo", "Branch!STRING:0|Product!STRING:0\neu|fenris\n");   // renamed folder: found by product
        mk("Games/Hearthstone", "Branch!STRING:0\nus\n");   // no product: found by folder
        mk("Program Files/Something Else", "Branch!STRING:0|Product!STRING:0\nus|xyz\n");   // not a game Hearth knows
        std::fs::create_dir_all(c.join("Program Files (x86)/StarCraft II")).unwrap();   // no .build.info: not installed
        let mut g = find_games(c);
        g.sort();
        let ids: Vec<&str> = g.iter().map(|x| x.0.as_str()).collect();
        assert_eq!(ids, vec!["battlenet:diablo4", "battlenet:hearthstone", "battlenet:overwatch"]);
        assert_eq!(g.iter().find(|x| x.0 == "battlenet:overwatch").unwrap().2, "Pro");
    }

    #[test]
    fn launch_argument_stays_one_argument() {
        assert_eq!(crate::manual::split_args("\"--exec=launch Pro\""), vec!["--exec=launch Pro"]);
    }

    #[test]
    fn runner_script() {
        let s = run_sh("/home/me/.local/share/nimbus/tools/umu/umu-run");
        assert!(s.starts_with("#!/bin/sh"));
        assert!(s.contains("exec '/home/me/.local/share/nimbus/tools/umu/umu-run' \"$exe\" \"$@\""));
        assert!(s.contains("export GAMEID=\"${GAMEID:-umu-battlenet}\""));
        assert_eq!(shell_quote("it's"), "'it'\\''s'");
    }

    #[test]
    fn hardware_acceleration_off_once() {
        let new: Value = serde_json::from_str(&without_hw_accel("").unwrap()).unwrap();
        assert_eq!(new["Client"]["HardwareAcceleration"], "false");
        let kept = without_hw_accel(r#"{"Client":{"Language":"deDE"}}"#).unwrap();
        assert!(kept.contains("deDE") && kept.contains("HardwareAcceleration"));
        assert_eq!(without_hw_accel(r#"{"Client":{"HardwareAcceleration":"true"}}"#), None);   // your own choice stays
        assert_eq!(without_hw_accel("not json"), None);
    }
}
