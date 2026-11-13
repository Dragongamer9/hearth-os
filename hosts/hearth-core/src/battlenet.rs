//! Battle.net (experimental): Blizzard's own Battle.net app, set up with one press through **Faugus Launcher**
//! (github.com/Faugus/faugus-launcher, preinstalled in Hearth OS), which runs Windows launchers with umu-launcher and
//! Proton. Battle.net has no open-source downloader like legendary or gogdl, so Hearth runs the real app.
//!
//! Faugus's own Battle.net option lives in its Add game window (no command line), so Hearth does exactly what that
//! option does, through Faugus: the same entry in Faugus's games.json ("Battle.net", in Faugus's prefix folder,
//! `~/Faugus/battlenet` by default, with Faugus's launch options for it), and Blizzard's installer run by Faugus's own
//! runner (`faugus-launcher --run`), which gets umu-launcher and Proton the way Faugus does. Battle.net then shows in
//! Faugus too, and Faugus's settings for it (Proton version, options) are what Hearth uses.
//!
//! Without Faugus (somewhere it isn't installed), and for Battle.net set up before Faugus (its own Proton prefix in
//! `~/.local/share/nimbus/battlenet/prefix`, started by `run.sh`), Hearth keeps its built-in setup. It runs Battle.net
//! with hearth-proton-run (proton_run.rs: the Proton built into Hearth OS, offline and without Steam); a `run.sh` from
//! before, which named umu-run, is rewritten to it before Battle.net starts (refresh_runner). Faugus itself is left
//! alone: it gets its own umu-run and Proton, the way Faugus does. Battle.net's installer and sign-in need the internet
//! by nature (Blizzard's servers); starting Proton never does.
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

// ------------------------------------------------------------------ Faugus Launcher

/// Faugus Launcher on Flathub (used when it isn't installed as a package).
pub const FAUGUS_FLATPAK: &str = "io.github.Faugus.faugus-launcher";
/// Battle.net in its prefix, as Faugus's Battle.net option writes it (LAUNCHER_EXE_PATHS["battle"]).
const FAUGUS_EXE: &str = "drive_c/Program Files (x86)/Battle.net/Battle.net.exe";
/// The launch options Faugus's Battle.net option gives it.
const FAUGUS_OPTIONS: &str = "WINE_SIMULATE_WRITECOPY=1\nPROTON_ENABLE_WAYLAND=0";
/// Hearth's own entries in Faugus's games.json, one per Blizzard game (hidden in Faugus: they're Battle.net with
/// "--exec=launch <code>"), so Play goes through Faugus with Battle.net's settings.
const HIDDEN: &str = "hearth-battlenet-";
/// Marks the installer's processes (Faugus's runner passes it on to everything it starts), so they can be stopped.
const SETUP_MARK: &str = "hearth-battlenet-setup";

/// How Faugus Launcher is installed: a package (its `faugus-launcher` command) or the Flatpak.
#[derive(Clone, Debug, PartialEq)]
pub enum Faugus { Native(PathBuf), Flatpak }

impl Faugus {
    /// The package first (Hearth OS ships it), else Flathub's.
    pub fn find() -> Option<Faugus> {
        if let Some(p) = std::env::var_os("PATH").and_then(|p| std::env::split_paths(&p).map(|d| d.join("faugus-launcher")).find(|f| f.is_file())) {
            return Some(Faugus::Native(p));
        }
        let home = crate::paths::home();
        let flat = [PathBuf::from("/var/lib/flatpak/app").join(FAUGUS_FLATPAK), home.join(".local/share/flatpak/app").join(FAUGUS_FLATPAK)];
        if crate::system::has("flatpak") && flat.iter().any(|d| d.join("current").exists()) { return Some(Faugus::Flatpak); }
        None
    }
    /// The command that starts Faugus (arguments follow).
    fn argv(&self) -> Vec<String> {
        match self {
            Faugus::Native(p) => vec![p.to_string_lossy().to_string()],
            Faugus::Flatpak => vec!["flatpak".into(), "run".into(), FAUGUS_FLATPAK.into()],
        }
    }
    /// Faugus's data folder (games.json, its umu-run): XDG's, or the Flatpak's own.
    fn data(&self, home: &Path) -> PathBuf {
        match self {
            Faugus::Native(_) => std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|| home.join(".local/share")).join("faugus-launcher"),
            Faugus::Flatpak => home.join(".var/app").join(FAUGUS_FLATPAK).join("data/faugus-launcher"),
        }
    }
    fn config(&self, home: &Path) -> PathBuf {
        match self {
            Faugus::Native(_) => std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| home.join(".config")).join("faugus-launcher"),
            Faugus::Flatpak => home.join(".var/app").join(FAUGUS_FLATPAK).join("config/faugus-launcher"),
        }
    }
    fn games_json(&self) -> PathBuf { self.data(&crate::paths::home()).join("games.json") }
    /// The umu-run Faugus downloads for itself (its runner gets it before running anything).
    fn umu_run(&self) -> PathBuf { self.data(&crate::paths::home()).join("umu-run") }
    /// (default prefix folder, default Proton) from Faugus's settings: ~/Faugus and Proton-CachyOS Latest unless changed.
    fn defaults(&self) -> (PathBuf, String) {
        let home = crate::paths::home();
        let cfg: Value = std::fs::read_to_string(self.config(&home).join("config.json")).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or(json!({}));
        let s = |k: &str| cfg[k].as_str().map(|v| v.trim_matches('"').to_string()).filter(|v| !v.is_empty());
        (s("default-prefix").map(|p| expand(&p, &home)).unwrap_or_else(|| home.join("Faugus")), s("default-runner").unwrap_or_else(|| "Proton-CachyOS Latest".into()))
    }
}

/// "~/x" and "$HOME/x" as Faugus writes them (expand_path), else the path itself.
fn expand(p: &str, home: &Path) -> PathBuf {
    if let Some(r) = p.strip_prefix("~/") { return home.join(r); }
    if let Some(r) = p.strip_prefix("$HOME/") { return home.join(r); }
    PathBuf::from(p)
}

/// Faugus's games.json: [] when there's none yet, None when it can't be read (then Hearth leaves it alone, as Faugus does).
fn read_games(f: &Path) -> Option<Vec<Value>> {
    match std::fs::read_to_string(f) {
        Ok(t) if t.trim().is_empty() => Some(vec![]),
        Ok(t) => serde_json::from_str::<Value>(&t).ok().and_then(|v| v.as_array().cloned()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Some(vec![]),
        Err(_) => None,
    }
}

/// Writes games.json whole, through a temporary file, so Faugus never reads half of it.
fn write_games(f: &Path, games: &[Value]) -> Result<(), String> {
    if let Some(d) = f.parent() { std::fs::create_dir_all(d).map_err(|e| e.to_string())?; }
    let tmp = f.with_extension("json.hearth-tmp");
    std::fs::write(&tmp, serde_json::to_string_pretty(&Value::Array(games.to_vec())).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, f).map_err(|e| e.to_string())
}

/// Battle.net's entry in Faugus's games (made by Faugus's Battle.net option or by Hearth): (its game id, its prefix).
/// Only one whose Battle.net is really installed in its prefix counts; Faugus's own "battlenet" id first.
pub fn faugus_entry(games: &[Value], home: &Path) -> Option<(String, PathBuf)> {
    let mut found: Vec<(String, PathBuf)> = games.iter().filter_map(|g| {
        let id = g["gameid"].as_str()?;
        if id.starts_with(HIDDEN) { return None; }
        let path = g["path"].as_str()?.replace('\\', "/").to_lowercase();
        if !(path.ends_with("battle.net/battle.net.exe") || path.ends_with("battle.net/battle.net launcher.exe")) { return None; }
        let prefix = expand(g["prefix"].as_str().filter(|p| !p.is_empty())?, home);
        prefix.join(FAUGUS_EXE).is_file().then(|| (id.to_string(), prefix))
    }).collect();
    found.sort_by_key(|(id, _)| id.as_str() != "battlenet");
    found.into_iter().next()
}

/// The entry Faugus's Battle.net option makes (its Add game window with "Battle.net" picked, default settings).
pub fn new_entry(gameid: &str, prefix: &Path) -> Value {
    let p = prefix.to_string_lossy();
    json!({"gameid": gameid, "title": "Battle.net", "path": format!("{p}/{FAUGUS_EXE}"), "prefix": p,
        "launch_arguments": FAUGUS_OPTIONS, "game_arguments": "", "mangohud": "", "gamemode": "", "sdl_enabled": "", "runner": "Default",
        "addapp_enabled": "", "addapp": "", "addapp_bat": format!("{p}/drive_c/Program Files (x86)/Battle.net/faugus-{gameid}.bat"), "addapp_delay": "", "addapp_first": "",
        "cover": "", "lossless_enabled": "", "lossless_multiplier": "", "lossless_flow": "", "lossless_performance": "", "lossless_hdr": "", "lossless_present": "",
        "playtime": 0, "hidden": false, "no_sleep": "", "category": false, "icon": "", "steamgriddb_id": "", "pre_launch": "", "post_launch": "",
        "steam_user": "", "disable_umu": "", "runtime": "", "last_played": ""})
}

/// Faugus's game id for a new entry: its own rule (unique_gameid), "battlenet", else "battlenet-2", "battlenet-3"...
fn unique_id(games: &[Value], base: &str) -> String {
    let taken = |id: &str| games.iter().any(|g| g["gameid"].as_str() == Some(id));
    if !taken(base) { return base.into(); }
    (2..).map(|n| format!("{base}-{n}")).find(|id| !taken(id)).unwrap()
}

/// games.json with one hidden entry per Blizzard game found: Battle.net's own entry (so its Proton and options) with
/// "--exec=launch <code>", keeping each one's play time. Entries for games no longer installed go. Nothing else changes.
pub fn with_game_entries(games: &[Value], parent: &str, found: &[(String, String, String, PathBuf)]) -> Vec<Value> {
    let Some(base) = games.iter().find(|g| g["gameid"].as_str() == Some(parent)).cloned() else { return games.to_vec() };
    let want: Vec<(String, Value)> = found.iter().map(|(id, title, code, _)| {
        let gid = format!("{HIDDEN}{}", id.trim_start_matches("battlenet:"));
        let old = games.iter().find(|g| g["gameid"].as_str() == Some(gid.as_str()));
        let mut e = base.clone();
        e["gameid"] = json!(gid); e["title"] = json!(title); e["hidden"] = json!(true);
        e["game_arguments"] = json!(format!("--exec=\"launch {code}\""));
        for k in ["cover", "icon", "steamgriddb_id", "addapp_enabled", "addapp", "last_played"] { e[k] = json!(""); }
        e["category"] = json!(false);
        e["playtime"] = old.map(|o| o["playtime"].clone()).unwrap_or(json!(0));
        if let Some(lp) = old.and_then(|o| o.get("last_played")) { e["last_played"] = lp.clone(); }
        (gid, e)
    }).collect();
    let mut out: Vec<Value> = games.iter().filter_map(|g| {
        let id = g["gameid"].as_str().unwrap_or("");
        if !id.starts_with(HIDDEN) { return Some(g.clone()); }
        want.iter().find(|(w, _)| w == id).map(|(_, e)| e.clone())
    }).collect();
    for (gid, e) in want { if !out.iter().any(|g| g["gameid"].as_str() == Some(gid.as_str())) { out.push(e); } }
    out
}

/// What the library starts for Battle.net set up through Faugus (made by Hearth). With no arguments it opens Battle.net;
/// a Blizzard game's Play passes `--game hearth-battlenet-<game>`.
pub fn faugus_script() -> PathBuf { dir().join("faugus.sh") }
fn faugus_sh(faugus: &Faugus, gameid: &str) -> String {
    let cmd: Vec<String> = faugus.argv().iter().map(|a| shell_quote(a)).collect();
    format!("#!/bin/sh\n# Starts Battle.net for Hearth through Faugus Launcher (made by Hearth; replaced when Battle.net is set up again).\n\
[ $# -eq 0 ] && set -- --game {}\nexec {} \"$@\"\n", shell_quote(gameid), cmd.join(" "))
}
fn write_script(faugus: &Faugus, gameid: &str) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let text = faugus_sh(faugus, gameid);
    if std::fs::read_to_string(faugus_script()).ok().as_deref() == Some(text.as_str()) { return Ok(()); }
    std::fs::create_dir_all(dir()).map_err(|e| e.to_string())?;
    std::fs::write(faugus_script(), text).map_err(|e| e.to_string())?;
    std::fs::set_permissions(faugus_script(), std::fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())
}

/// What Faugus's runner runs to install Battle.net: Faugus's own command for its Battle.net option (Faugus's umu-run,
/// its Proton, its options), in the system's language. Faugus's runner reads the VAR=value words as the environment.
pub fn installer_message(umu: &Path, prefix: &Path, setup: &Path, runner: &str, lang: &str) -> String {
    let q = |p: &Path| shell_quote(&p.to_string_lossy());
    let proton = match runner {
        "" | "Default" => String::new(),
        "Proton-CachyOS (System)" => format!("PROTONPATH={} ", shell_quote("/usr/share/steam/compatibilitytools.d/proton-cachyos-slr/")),
        r => format!("PROTONPATH={} ", shell_quote(r)),
    };
    format!("FAUGUSID={SETUP_MARK} LOG_DIR=battlenet PROTON_ENABLE_WAYLAND=0 WINE_SIMULATE_WRITECOPY=1 WINEPREFIX={} {proton}{} {} '--installpath=C:\\Program Files (x86)\\Battle.net' --lang={lang}",
        q(prefix), q(umu), q(setup))
}

// ------------------------------------------------------------------ Which Battle.net Hearth uses

/// Hearth's built-in setup (before Faugus, or without it): its folder, prefix and script.
pub fn dir() -> PathBuf { crate::paths::data_dir().join("battlenet") }
/// The built-in setup's Proton prefix (umu makes drive_c right in it).
pub fn prefix() -> PathBuf { dir().join("prefix") }
/// What the library starts for the built-in setup: a small script that runs Battle.net with Proton in its prefix.
pub fn runner() -> PathBuf { dir().join("run.sh") }
fn drive_c() -> PathBuf { prefix().join("drive_c") }
fn app_folder() -> PathBuf { drive_c().join("Program Files (x86)").join("Battle.net") }
fn launcher_exe() -> PathBuf { app_folder().join("Battle.net Launcher.exe") }
/// Written by Battle.net the first time it runs, after its install has finished.
fn config_in(prefix: &Path) -> PathBuf { prefix.join("drive_c/users/steamuser/AppData/Roaming/Battle.net/Battle.net.config") }
fn log_file() -> PathBuf { dir().join("setup.log") }

/// Battle.net as Hearth found it.
#[derive(Clone, Debug, PartialEq)]
pub enum Setup {
    /// Through Faugus Launcher: Faugus's entry for it and its prefix.
    Faugus { faugus: Faugus, gameid: String, prefix: PathBuf },
    /// Hearth's built-in setup in ~/.local/share/nimbus/battlenet (set up before Faugus, or without it): kept working.
    Builtin,
}

impl Setup {
    fn prefix(&self) -> PathBuf { match self { Setup::Faugus { prefix, .. } => prefix.clone(), Setup::Builtin => prefix() } }
    /// What the library starts for Battle.net and its games.
    fn script(&self) -> PathBuf { match self { Setup::Faugus { .. } => faugus_script(), Setup::Builtin => runner() } }
}

/// The built-in setup is there (Battle.net in Hearth's own prefix, and its script).
fn builtin_installed() -> bool { launcher_exe().is_file() && runner().is_file() }

/// The Battle.net Hearth uses: Faugus's when it has one, else the built-in one when that's set up.
pub fn current() -> Option<Setup> {
    if let Some(f) = Faugus::find() {
        if let Some((gameid, prefix)) = read_games(&f.games_json()).and_then(|g| faugus_entry(&g, &crate::paths::home())) {
            return Some(Setup::Faugus { faugus: f, gameid, prefix });
        }
    }
    builtin_installed().then_some(Setup::Builtin)
}

pub fn installed() -> bool { current().is_some() }

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
    let Some(setup) = current() else { return vec![] };
    let found = find_games(&setup.prefix().join("drive_c"));
    if let Setup::Faugus { faugus, gameid, .. } = &setup {
        // Faugus starts them: its script, and one hidden Faugus entry per game (written only when something changed).
        if let Err(e) = write_script(faugus, gameid) { crate::bridge::log(&format!("battlenet: couldn't write faugus.sh: {e}")); }
        let f = faugus.games_json();
        if let Some(list) = read_games(&f) {
            let new = with_game_entries(&list, gameid, &found);
            if new != list { if let Err(e) = write_games(&f, &new) { crate::bridge::log(&format!("battlenet: couldn't update Faugus's games: {e}")); } }
        }
    }
    let exe = setup.script().to_string_lossy().to_string();
    let app = setup.prefix().join("drive_c/Program Files (x86)/Battle.net");
    let mut out = vec![crate::library::Game { id: APP_ID.into(), title: "Battle.net".into(), source: "battlenet", install_path: Some(app),
        installed: true, state: "installed", exe: Some(exe.clone()), ..Default::default() }];
    for (id, title, code, folder) in found {
        let args = match &setup {
            Setup::Faugus { .. } => format!("--game {HIDDEN}{}", id.trim_start_matches("battlenet:")),
            // One argument, quoted whole: split_args keeps it together ("--exec=launch Pro").
            Setup::Builtin => format!("\"--exec=launch {code}\""),
        };
        out.push(crate::library::Game { id, title, source: "battlenet", install_path: Some(folder), installed: true, state: "installed",
            exe: Some(exe.clone()), args, ..Default::default() });
    }
    out
}

/// The built-in setup's script. `runner` starts Windows programs with Proton (proton_run::command: hearth-proton-run).
fn run_sh(runner: &str) -> String {
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
exec {runner} "$exe" "$@"
"#, runner = shell_quote(runner), gameid = GAMEID)
}

/// A built-in setup's `run.sh` from before hearth-proton-run (it named umu-run, which downloads itself, the Steam
/// Runtime and UMU-Proton) is rewritten to start Battle.net with hearth-proton-run, offline. Only Hearth's own script;
/// nothing when the image has no hearth-proton-run. Called before Battle.net or one of its games starts.
pub fn refresh_runner() {
    let Some(runner) = crate::proton_run::offline() else { return };
    let f = self::runner();
    let Ok(old) = std::fs::read_to_string(&f) else { return };
    let new = run_sh(&runner);
    if !needs_refresh(&old, &new) { return; }
    let tmp = f.with_extension("sh.tmp");
    use std::os::unix::fs::PermissionsExt;
    let ok = std::fs::write(&tmp, &new).is_ok()
        && std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755)).is_ok()
        && std::fs::rename(&tmp, &f).is_ok();
    if ok { crate::bridge::log("battlenet: run.sh now starts Battle.net with hearth-proton-run (offline, without Steam)"); }
    else { let _ = std::fs::remove_file(&tmp); }
}

/// Hearth's own run.sh (its first comment line) that isn't the current one.
fn needs_refresh(old: &str, new: &str) -> bool {
    old != new && old.lines().nth(1).map(|l| l.starts_with("# Starts Battle.net for Hearth in its own Proton prefix")).unwrap_or(false)
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
fn hw_accel_off(prefix: &Path) {
    let f = config_in(prefix);
    if let Some(new) = without_hw_accel(&std::fs::read_to_string(&f).unwrap_or_default()) {
        if let Some(p) = f.parent() { let _ = std::fs::create_dir_all(p); }
        let _ = std::fs::write(&f, new);
    }
}

/// Opens Battle.net once Desktop Mode starts (its first sign-in, where the controller types), like Xodus Nexus.
pub fn open_in_desktop() -> Result<(), String> {
    let Some(setup) = current() else { return Err("Set up Battle.net first".into()) };
    if let Setup::Faugus { faugus, gameid, .. } = &setup { write_script(faugus, gameid)?; } else { refresh_runner(); }
    let f = PathBuf::from(std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into())).join("hearth-open-after");
    std::fs::write(f, setup.script().to_string_lossy().as_bytes()).map_err(|e| e.to_string())
}

/// A Faugus prefix Hearth may delete with Battle.net: a real prefix (drive_c) that isn't the home folder or Faugus's
/// shared "default" prefix, and that no other Faugus game uses.
fn prefix_removable(prefix: &Path, games: &[Value], gameid: &str, home: &Path) -> bool {
    let others = games.iter().any(|g| {
        let id = g["gameid"].as_str().unwrap_or("");
        id != gameid && !id.starts_with(HIDDEN) && g["prefix"].as_str().map(|p| expand(p, home) == prefix).unwrap_or(false)
    });
    prefix.join("drive_c").is_dir() && prefix != home && prefix.parent().is_some() && prefix.file_name().map(|n| n != "default").unwrap_or(false) && !others
}

/// Removes Battle.net and every Blizzard game installed with it: Faugus's entries for it and its prefix, and Hearth's
/// built-in setup when there is one.
pub fn remove() -> Result<(), String> {
    if let Some(Setup::Faugus { faugus, gameid, prefix }) = current() {
        let f = faugus.games_json();
        let list = read_games(&f).ok_or("Couldn't read Faugus Launcher's game list")?;
        let removable = prefix_removable(&prefix, &list, &gameid, &crate::paths::home());
        let keep: Vec<Value> = list.iter().filter(|g| { let id = g["gameid"].as_str().unwrap_or(""); id != gameid && !id.starts_with(HIDDEN) }).cloned().collect();
        write_games(&f, &keep)?;
        if removable { std::fs::remove_dir_all(&prefix).map_err(|e| format!("Couldn't remove it: {e}"))?; }
    }
    if dir().exists() { std::fs::remove_dir_all(dir()).map_err(|e| format!("Couldn't remove it: {e}"))?; }
    Ok(())
}

/// Sends `sig` to every process carrying `KEY=value` in its environment (as Faugus's kill_by_faugusid does).
fn signal_marked(marker: &str, sig: &str) {
    let mark = marker.as_bytes();
    let me = std::process::id().to_string();
    let pids: Vec<String> = std::fs::read_dir("/proc").into_iter().flatten().flatten().filter_map(|e| {
        let pid = e.file_name().to_string_lossy().to_string();
        if !pid.bytes().all(|b| b.is_ascii_digit()) || pid == me { return None; }
        let env = std::fs::read(e.path().join("environ")).ok()?;
        env.split(|b| *b == 0).any(|kv| kv == mark).then_some(pid)
    }).collect();
    if !pids.is_empty() { let _ = std::process::Command::new("kill").arg(sig).args(&pids).status(); }
}

/// Stops the installer and everything it started (its own process group, and what Faugus's runner started in a
/// session of its own, found by its mark).
fn stop(child: &mut std::process::Child, mark: Option<&str>) {
    let _ = std::process::Command::new("kill").args(["-TERM", &format!("-{}", child.id())]).status();
    if let Some(m) = mark { signal_marked(m, "-TERM"); }
    for _ in 0..20 { if matches!(child.try_wait(), Ok(Some(_))) { break; } std::thread::sleep(Duration::from_millis(500)); }
    if let Some(m) = mark { signal_marked(m, "-KILL"); }
    if !matches!(child.try_wait(), Ok(Some(_))) {
        let _ = std::process::Command::new("kill").args(["-KILL", &format!("-{}", child.id())]).status();
        let _ = child.wait();
    }
}

/// Waits for the installer (`child`) until Battle.net (`exe`) is in and done, then closes it. Battle.net opens by itself
/// at the end (its settings file appears), or it's been there for 3 minutes; its window is closed, and Hearth opens it
/// from the library, where the sign-in happens.
fn wait_installed(child: &mut std::process::Child, exe: &Path, config: &Path, mark: Option<&str>, limit: Duration) -> Result<(), String> {
    let started = Instant::now();
    let mut seen: Option<Instant> = None;
    loop {
        if child.try_wait().map_err(|e| e.to_string())?.is_some() {
            // Proton (waitforexitandrun) waits for every program in the prefix, so the installer closing means it's all over.
            if exe.is_file() { if let Some(m) = mark { signal_marked(m, "-TERM"); } return Ok(()); }
            return Err("Battle.net's installer closed before Battle.net was installed. Try again; the details are in battlenet/setup.log.".into());
        }
        if exe.is_file() { seen.get_or_insert_with(Instant::now); }
        if let Some(t) = seen { if (config.is_file() && t.elapsed() > Duration::from_secs(10)) || t.elapsed() > Duration::from_secs(180) { stop(child, mark); return Ok(()); } }
        if started.elapsed() > limit { stop(child, mark); return Err("Battle.net's installer took too long and was stopped. Try again.".into()); }
        std::thread::sleep(Duration::from_secs(1));
    }
}

/// Downloads Blizzard's installer to `to`.
fn download_installer(to: &Path) -> Result<(), String> {
    let to = to.to_string_lossy().to_string();
    crate::system::run_timeout("curl", &["-fsSL", "--retry", "3", "-A", "Hearth", "-o", to.as_str(), INSTALLER], Duration::from_secs(600))
        .map(|_| ()).map_err(|_| "Couldn't download Battle.net's installer. Check the internet connection and try again.".to_string())
}

/// Setting up through Faugus: its entry for Battle.net (one left by an unfinished setup is used again), Blizzard's
/// installer run by Faugus's runner in that prefix, then the entry saved in Faugus's games.
fn install_faugus(faugus: &Faugus, stage: &dyn Fn(&str)) -> Result<(), String> {
    let f = faugus.games_json();
    let list = read_games(&f).ok_or("Faugus Launcher's game list (games.json) can't be read, so Hearth left it alone. Open Faugus Launcher once, then try again.")?;
    let (default_prefix, runner) = faugus.defaults();
    let home = crate::paths::home();
    let again = list.iter().find(|g| !g["gameid"].as_str().unwrap_or("").starts_with(HIDDEN) && g["title"].as_str().map(|t| t.eq_ignore_ascii_case("Battle.net")).unwrap_or(false));
    let (gameid, prefix) = match again.and_then(|g| Some((g["gameid"].as_str()?.to_string(), expand(g["prefix"].as_str().filter(|p| !p.is_empty())?, &home)))) {
        Some(x) => x,
        None => { let id = unique_id(&list, "battlenet"); let p = default_prefix.join(&id); (id, p) }
    };
    std::fs::create_dir_all(&prefix).map_err(|e| format!("Couldn't make Battle.net's folder {}: {e}", prefix.display()))?;
    std::fs::create_dir_all(dir()).map_err(|e| e.to_string())?;
    stage("Downloading Blizzard's installer…");
    // In the prefix, where Faugus (even its Flatpak) can always read it.
    let setup = prefix.join("Battle.net-Setup.exe");
    download_installer(&setup)?;
    stage("Installing Battle.net with Faugus Launcher… Faugus may get Proton first, then Battle.net downloads itself. This can take a while.");
    let lang = lang_of(&std::env::var("LANG").unwrap_or_default());
    let msg = installer_message(&faugus.umu_run(), &prefix, &setup, &runner, lang);
    let log = std::fs::File::create(log_file()).map_err(|e| e.to_string())?;
    let argv = faugus.argv();
    use std::os::unix::process::CommandExt;
    let mut child = std::process::Command::new(&argv[0]).args(&argv[1..]).arg("--run").arg(&msg)
        .stdin(std::process::Stdio::null()).stdout(log.try_clone().map_err(|e| e.to_string())?).stderr(log)
        .process_group(0).spawn().map_err(|e| format!("Couldn't start Faugus Launcher: {e}"))?;
    let mark = format!("FAUGUSID={SETUP_MARK}");
    let r = wait_installed(&mut child, &prefix.join(FAUGUS_EXE), &config_in(&prefix), Some(&mark), Duration::from_secs(90 * 60));
    let _ = std::fs::remove_file(&setup);
    r?;
    hw_accel_off(&prefix);
    // Saved now that it's installed (Faugus's own option drops the entry when the install fails, too). Read again:
    // Faugus may have changed its list meanwhile.
    let mut list = read_games(&f).ok_or("Faugus Launcher's game list (games.json) can't be read")?;
    if !list.iter().any(|g| g["gameid"].as_str() == Some(gameid.as_str())) { list.push(new_entry(&gameid, &prefix)); write_games(&f, &list)?; }
    write_script(faugus, &gameid)?;
    crate::bridge::log(&format!("battlenet: set up through Faugus Launcher ({gameid} in {})", prefix.display()));
    Ok(())
}

/// Hearth's built-in setup (no Faugus): hearth-proton-run (the built-in Proton, offline), Blizzard's installer, Battle.net
/// in its prefix.
fn install_builtin(stage: &dyn Fn(&str)) -> Result<(), String> {
    stage("Faugus Launcher isn't installed, so Hearth sets Battle.net up itself with the Proton built into Hearth OS…");
    let proton = crate::proton_run::command()?;
    std::fs::create_dir_all(prefix()).map_err(|e| e.to_string())?;
    stage("Downloading Blizzard's installer…");
    let setup = dir().join("Battle.net-Setup.exe");
    download_installer(&setup)?;
    use std::os::unix::fs::PermissionsExt;
    std::fs::write(runner(), run_sh(&proton)).map_err(|e| e.to_string())?;
    std::fs::set_permissions(runner(), std::fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;
    stage("Installing Battle.net… It downloads itself, which can take a few minutes.");
    let lang = format!("--lang={}", lang_of(&std::env::var("LANG").unwrap_or_default()));
    let log = std::fs::File::create(log_file()).map_err(|e| e.to_string())?;
    use std::os::unix::process::CommandExt;
    // Its own process group, so the installer and the Battle.net it opens at the end can be stopped together.
    let setup_s = setup.to_string_lossy().to_string();
    let mut child = std::process::Command::new(&proton)
        .args([setup_s.as_str(), lang.as_str(), "--installpath=C:\\Program Files (x86)\\Battle.net"])
        .env("WINEPREFIX", prefix()).env("GAMEID", GAMEID).env("STORE", "battlenet")
        .stdin(std::process::Stdio::null()).stdout(log.try_clone().map_err(|e| e.to_string())?).stderr(log)
        .process_group(0).spawn().map_err(|e| format!("Couldn't start the installer: {e}"))?;
    let r = wait_installed(&mut child, &launcher_exe(), &config_in(&prefix()), None, Duration::from_secs(45 * 60));
    let _ = std::fs::remove_file(&setup);
    r?;
    hw_accel_off(&prefix());
    crate::bridge::log("battlenet: set up (built in, without Faugus Launcher)");
    Ok(())
}

/// Setting up, in the background, telling the page how it's going ("battlenet" events).
pub struct Battlenet { pub state: Arc<Mutex<Value>> }
impl Default for Battlenet { fn default() -> Self { Self::new() } }
impl Battlenet {
    pub fn new() -> Battlenet { Battlenet { state: Arc::new(Mutex::new(json!({"busy": false, "stage": null, "error": null}))) } }

    /// {installed, busy, stage, error, games, faugus (Faugus Launcher is there), via ("faugus" / "hearth" once set up)}
    pub fn status(&self) -> Value {
        let mut s = self.state.lock().unwrap().clone();
        let cur = current();
        s["installed"] = json!(cur.is_some());
        s["games"] = json!(cur.as_ref().map(|c| find_games(&c.prefix().join("drive_c")).len()).unwrap_or(0));
        s["faugus"] = json!(Faugus::find().is_some());
        s["via"] = match cur { Some(Setup::Faugus { .. }) => json!("faugus"), Some(Setup::Builtin) => json!("hearth"), None => Value::Null };
        s
    }

    /// One press: through Faugus Launcher when it's installed, else Hearth's built-in setup.
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
            let stage = |t: &str| { state.lock().unwrap()["stage"] = json!(t); push(&state); };
            let r = match Faugus::find() { Some(f) => install_faugus(&f, &stage), None => install_builtin(&stage) };
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
        let s = run_sh("/usr/bin/hearth-proton-run");
        assert!(s.starts_with("#!/bin/sh"));
        assert!(s.contains("exec '/usr/bin/hearth-proton-run' \"$exe\" \"$@\""));
        // A run.sh from before (umu-run) is Hearth's own and gets rewritten; anything else, or the current one, isn't.
        let old = run_sh("/home/me/.local/share/nimbus/tools/umu/umu-run");
        assert!(needs_refresh(&old, &s) && !needs_refresh(&s, &s) && !needs_refresh("#!/bin/sh\n# my own script\n", &s));
        assert!(s.contains("export GAMEID=\"${GAMEID:-umu-battlenet}\""));
        assert_eq!(shell_quote("it's"), "'it'\\''s'");
    }

    /// A Faugus prefix with Battle.net installed in it.
    fn faugus_prefix(root: &Path, name: &str) -> PathBuf {
        let p = root.join("Faugus").join(name);
        std::fs::create_dir_all(p.join("drive_c/Program Files (x86)/Battle.net")).unwrap();
        std::fs::write(p.join(FAUGUS_EXE), b"").unwrap();
        p
    }

    #[test]
    fn faugus_battlenet_entry() {
        let d = tempfile::tempdir().unwrap();
        let home = d.path();
        let p = faugus_prefix(home, "battlenet");
        // Made by Faugus's own Battle.net option ("~" as Faugus may write it), next to other games.
        let games = vec![
            json!({"gameid": "hollow-knight", "title": "Hollow Knight", "path": "/g/hk.exe", "prefix": home.join("Faugus/hollow-knight").to_string_lossy()}),
            json!({"gameid": "battlenet", "title": "Battle.net", "path": "~/Faugus/battlenet/drive_c/Program Files (x86)/Battle.net/Battle.net.exe", "prefix": "~/Faugus/battlenet"}),
        ];
        assert_eq!(faugus_entry(&games, home), Some(("battlenet".to_string(), p.clone())));
        // Listed but not installed (an unfinished setup): not counted.
        let half = vec![json!({"gameid": "battlenet", "title": "Battle.net", "path": "/nowhere/drive_c/Program Files (x86)/Battle.net/Battle.net.exe", "prefix": "/nowhere"})];
        assert_eq!(faugus_entry(&half, home), None);
        // Hearth's own entries for games are never taken for Battle.net itself.
        let ours = vec![json!({"gameid": "hearth-battlenet-overwatch", "path": format!("{}/{FAUGUS_EXE}", p.display()), "prefix": p.to_string_lossy()})];
        assert_eq!(faugus_entry(&ours, home), None);
        // The entry Hearth writes is found the same way, and is what Faugus's option writes.
        let e = new_entry("battlenet", &p);
        assert_eq!(e["runner"], "Default");
        assert_eq!(e["launch_arguments"], "WINE_SIMULATE_WRITECOPY=1\nPROTON_ENABLE_WAYLAND=0");
        assert_eq!(e["hidden"], false);
        assert_eq!(faugus_entry(&[e], home), Some(("battlenet".to_string(), p)));
    }

    #[test]
    fn faugus_ids_like_faugus() {
        assert_eq!(unique_id(&[], "battlenet"), "battlenet");
        assert_eq!(unique_id(&[json!({"gameid": "battlenet"}), json!({"gameid": "battlenet-2"})], "battlenet"), "battlenet-3");
    }

    #[test]
    fn faugus_entries_for_games() {
        let p = PathBuf::from("/h/Faugus/battlenet");
        let mut parent = new_entry("battlenet", &p);
        parent["runner"] = json!("GE-Proton10-1");   // your own choice in Faugus carries over to the games
        let games = vec![
            json!({"gameid": "hk", "title": "Hollow Knight"}),
            parent,
            json!({"gameid": "hearth-battlenet-wow", "title": "World of Warcraft", "hidden": true, "playtime": 99}),
            json!({"gameid": "hearth-battlenet-diablo4", "title": "Diablo IV", "hidden": true, "playtime": 5}),
        ];
        let found = vec![
            ("battlenet:overwatch".to_string(), "Overwatch 2".to_string(), "Pro".to_string(), p.join("drive_c/Program Files (x86)/Overwatch")),
            ("battlenet:wow".to_string(), "World of Warcraft".to_string(), "WoW".to_string(), p.join("drive_c/Program Files (x86)/World of Warcraft")),
        ];
        let out = with_game_entries(&games, "battlenet", &found);
        let ids: Vec<&str> = out.iter().map(|g| g["gameid"].as_str().unwrap()).collect();
        assert_eq!(ids, vec!["hk", "battlenet", "hearth-battlenet-wow", "hearth-battlenet-overwatch"]);   // Diablo IV is gone
        let ow = &out[3];
        assert_eq!(ow["hidden"], true);
        assert_eq!(ow["game_arguments"], "--exec=\"launch Pro\"");
        assert_eq!(ow["runner"], "GE-Proton10-1");
        assert_eq!(ow["path"], format!("/h/Faugus/battlenet/{FAUGUS_EXE}"));
        assert_eq!(out[2]["playtime"], 99);   // play time kept
        // Nothing to change: the same list, so games.json isn't written again.
        assert_eq!(with_game_entries(&out, "battlenet", &found), out);
        // Without Battle.net's entry nothing is touched.
        assert_eq!(with_game_entries(&games, "nope", &found), games);
    }

    #[test]
    fn faugus_games_json_roundtrip() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("faugus-launcher/games.json");
        assert_eq!(read_games(&f), Some(vec![]));   // none yet
        write_games(&f, &[json!({"gameid": "x", "extra": {"kept": 1}})]).unwrap();
        assert_eq!(read_games(&f).unwrap()[0]["extra"]["kept"], 1);
        std::fs::write(&f, "{broken").unwrap();
        assert_eq!(read_games(&f), None);   // left alone
    }

    #[test]
    fn faugus_installer_command() {
        let m = installer_message(Path::new("/h/.local/share/faugus-launcher/umu-run"), Path::new("/h/Faugus/battlenet"), Path::new("/h/Faugus/battlenet/Battle.net-Setup.exe"), "Proton-CachyOS Latest", "deDE");
        assert!(m.starts_with("FAUGUSID=hearth-battlenet-setup "));
        assert!(m.contains("WINE_SIMULATE_WRITECOPY=1") && m.contains("PROTON_ENABLE_WAYLAND=0"));
        assert!(m.contains("WINEPREFIX='/h/Faugus/battlenet' PROTONPATH='Proton-CachyOS Latest' '/h/.local/share/faugus-launcher/umu-run' '/h/Faugus/battlenet/Battle.net-Setup.exe'"));
        assert!(m.ends_with("'--installpath=C:\\Program Files (x86)\\Battle.net' --lang=deDE"));
        assert!(!installer_message(Path::new("/u"), Path::new("/p"), Path::new("/s"), "Default", "enUS").contains("PROTONPATH"));
    }

    #[test]
    fn faugus_script_text() {
        let s = faugus_sh(&Faugus::Native(PathBuf::from("/usr/bin/faugus-launcher")), "battlenet");
        assert!(s.starts_with("#!/bin/sh\n"));
        assert!(s.contains("[ $# -eq 0 ] && set -- --game 'battlenet'\nexec '/usr/bin/faugus-launcher' \"$@\"\n"));
        assert!(faugus_sh(&Faugus::Flatpak, "battlenet").contains("exec 'flatpak' 'run' 'io.github.Faugus.faugus-launcher' \"$@\""));
    }

    #[test]
    fn faugus_prefix_removed_only_when_safe() {
        let d = tempfile::tempdir().unwrap();
        let home = d.path();
        let p = faugus_prefix(home, "battlenet");
        let alone = vec![json!({"gameid": "battlenet", "prefix": p.to_string_lossy()}), json!({"gameid": "hearth-battlenet-wow", "prefix": p.to_string_lossy()})];
        assert!(prefix_removable(&p, &alone, "battlenet", home));
        let shared = vec![json!({"gameid": "battlenet", "prefix": p.to_string_lossy()}), json!({"gameid": "hk", "prefix": p.to_string_lossy()})];
        assert!(!prefix_removable(&p, &shared, "battlenet", home));   // another Faugus game lives there too
        let def = faugus_prefix(home, "default");
        assert!(!prefix_removable(&def, &[], "battlenet", home));   // Faugus's shared prefix
        assert!(!prefix_removable(home, &[], "battlenet", home));
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
