//! Beacon: the launcher for The Battle for Middle-earth games, Age of the Ring, LOTR: Conquest and the community
//! versions of Universe at War, Armies of Exigo, Rise of Legends and Galactic Battlegrounds. Hearth shows Beacon's
//! games in its library (Library › Beacon) and starts them through Beacon's own command-line tool, `beacon`.
//! The whole handshake is described in docs/BEACON.md.
//!
//! What Hearth runs, and nothing else (always Beacon's own binary, with an argument list, never a shell):
//!   beacon status --json                              every game Beacon knows, with its state (GameStatus)
//!   beacon mods <game>                                a game's mods: "* <id> <mod|launcher> <path>", * = chosen
//!   beacon select-mod <game> <mod id|none>            the mod a game starts with (shared with Beacon's own window)
//!   beacon launch <game> [--mod=<id>|--vanilla] --wait
//!                                                     starts it; with --wait Beacon stays the parent of the Wine
//!                                                     processes, which carry HEARTH_GAME_ID (appid.rs)
//!   beacon-app [--fullscreen]                         Beacon's window: mods, settings, setting up a game
//!
//! Where `beacon` comes from: a `beacon` executable on PATH, in ~/.local/bin, /usr/bin, /usr/local/bin or
//! /opt/beacon/bin; or Beacon's Flatpak (FLATPAK_ID), but only when that Flatpak ships the `beacon` tool
//! (files/bin/beacon), run as `flatpak run --command=beacon <id> …`. Game ids are checked against Beacon's own
//! list (GAMES) and mod ids against valid_mod_id before they go on a command line.
//!
//! Hearth notices changes (a game set up or a mod added in Beacon) by watching Beacon's config file and mods
//! folder (signature()); when they change the games are read again in the background (Library::tick).

use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Beacon's app id (its Tauri identifier), for a Flatpak install.
pub const FLATPAK_ID: &str = "io.github.dragongamer9.beacon";
/// Beacon's games: GameId's names in `beacon status --json`, in Beacon's order.
pub const GAMES: [&str; 9] = ["bfme1", "bfme2", "rotwk", "aotr", "conquest", "uaw", "exigo", "rol", "swgb"];
/// Steam app ids Beacon knows (its GameId::steam_appid), for art.
const STEAM_IDS: [(&str, u32); 2] = [("uaw", 10430), ("swgb", 356500)];
const MAX_MODS: usize = 200;

pub fn known_game(id: &str) -> bool { GAMES.contains(&id) }

/// A mod id as Beacon makes them ("rotwk:edain": the game, then the mod's name as lowercase letters, digits and
/// dashes), checked before it goes on a command line: letters, digits and . _ : - only, not starting with a dash.
pub fn valid_mod_id(s: &str) -> bool {
    !s.is_empty() && s.len() <= 128 && !s.starts_with(['-', '.', ':']) && s.bytes().all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
}

/// Games that get Hearth Input's "Strategy (RTS)" controller layout until you pick another (or Off): all of
/// Beacon's but LOTR: Conquest, an action game with controller support of its own.
pub fn rts_by_default(library_id: &str) -> bool {
    library_id.strip_prefix("beacon:").map(|g| known_game(g) && g != "conquest").unwrap_or(false)
}

// ------------------------------------------------------------------ finding Beacon

/// How Beacon is run: its own executable, or `flatpak run` of Beacon's Flatpak.
#[derive(Debug, Clone, PartialEq)]
pub enum Exe {
    Native(PathBuf),
    /// The flatpak executable, and the command inside the Flatpak (Some("beacon") for the tool, None for the app).
    Flatpak(PathBuf, Option<&'static str>),
}

impl Exe {
    /// The program to start and its whole argument list: Beacon's arguments after Hearth's own (for a Flatpak,
    /// `run [--command=beacon] [--env=HEARTH_GAME_ID=<n>] <id>` first). `game_id` 0: no number.
    pub fn command(&self, args: &[String], game_id: u32) -> (PathBuf, Vec<String>) {
        match self {
            Exe::Native(p) => (p.clone(), args.to_vec()),
            Exe::Flatpak(fp, cmd) => {
                let mut a = vec!["run".to_string()];
                if let Some(c) = cmd { a.push(format!("--command={c}")); }
                // The sandbox doesn't keep every variable: the number goes in explicitly.
                if game_id != 0 { a.push(format!("--env={}={game_id}", crate::appid::ENV)); }
                a.push(FLATPAK_ID.to_string());
                a.extend(args.iter().cloned());
                (fp.clone(), a)
            }
        }
    }
    pub fn via(&self) -> &'static str { match self { Exe::Native(_) => "native", Exe::Flatpak(..) => "flatpak" } }
}

fn is_exe(p: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(p).map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0).unwrap_or(false)
}

/// An executable called `name` on PATH (absolute folders only) or in the usual install places.
pub fn find_native(home: &Path, path_env: &str, name: &str) -> Option<PathBuf> {
    let mut c: Vec<PathBuf> = std::env::split_paths(path_env).filter(|d| d.is_absolute()).map(|d| d.join(name)).collect();
    for d in [home.join(".local/bin"), "/usr/bin".into(), "/usr/local/bin".into(), "/opt/beacon/bin".into()] { c.push(d.join(name)); }
    c.into_iter().find(|p| is_exe(p))
}

/// Beacon's Flatpak, installed for this user or system-wide: its active files folder.
pub fn flatpak_files(home: &Path, roots: &[PathBuf]) -> Option<PathBuf> {
    let mut all = vec![home.join(".local/share/flatpak/app")];
    all.extend(roots.iter().cloned());
    all.into_iter().map(|r| r.join(FLATPAK_ID).join("current/active/files")).find(|f| f.is_dir())
}
fn system_flatpak_roots() -> Vec<PathBuf> { vec!["/var/lib/flatpak/app".into()] }

/// `beacon`, the command-line tool: native first, else the Flatpak's when it ships it.
pub fn locate_cli(home: &Path, path_env: &str, flatpak_roots: &[PathBuf]) -> Option<Exe> {
    if let Some(p) = find_native(home, path_env, "beacon") { return Some(Exe::Native(p)); }
    let files = flatpak_files(home, flatpak_roots)?;
    if !is_exe(&files.join("bin/beacon")) { return None; }
    find_native(home, path_env, "flatpak").map(|fp| Exe::Flatpak(fp, Some("beacon")))
}
/// `beacon-app`, Beacon's window: native, else the Flatpak (whose main program it is).
pub fn locate_app(home: &Path, path_env: &str, flatpak_roots: &[PathBuf]) -> Option<Exe> {
    if let Some(p) = find_native(home, path_env, "beacon-app") { return Some(Exe::Native(p)); }
    flatpak_files(home, flatpak_roots)?;
    find_native(home, path_env, "flatpak").map(|fp| Exe::Flatpak(fp, None))
}

// ------------------------------------------------------------------ what Beacon says

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Mod { pub id: String, pub name: String, pub kind: String, pub path: String }

#[derive(Debug, Clone, Default, PartialEq)]
pub struct BeaconGame {
    pub id: String,
    /// bfme, lotr or other (Beacon's Family).
    pub family: String,
    pub title: String,
    pub short: String,
    pub path: Option<String>,
    pub installed: bool,
    pub ready: bool,
    /// (error|warn, message)
    pub issues: Vec<(String, String)>,
    pub selected_mod: Option<String>,
    pub edition: Option<String>,
    pub steam_appid: Option<u32>,
    pub via_steam: bool,
    pub mods: Vec<Mod>,
}

impl BeaconGame {
    /// The first thing stopping it from starting (Beacon's own words).
    pub fn blocker(&self) -> Option<&str> { self.issues.iter().find(|(l, _)| l == "error").map(|(_, m)| m.as_str()) }
}

fn text(v: &Value, max: usize) -> Option<String> {
    v.as_str().map(|s| s.chars().filter(|c| !c.is_control()).take(max).collect::<String>()).map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}
fn default_family(id: &str) -> &'static str {
    match id { "conquest" => "lotr", "uaw" | "exigo" | "rol" | "swgb" => "other", _ => "bfme" }
}

/// `beacon status --json`, checked: unknown games are left out, every field is the right kind and length.
pub fn parse_status(json: &str) -> Result<Vec<BeaconGame>, String> {
    let v: Value = serde_json::from_str(json).map_err(|e| format!("Beacon's game list isn't valid: {e}"))?;
    let list = v.as_array().ok_or("Beacon's game list isn't a list")?;
    let mut out: Vec<BeaconGame> = Vec::new();
    for g in list.iter().take(64) {
        let Some(id) = g["id"].as_str() else { continue };
        if !known_game(id) {
            crate::bridge::log(&format!("beacon: left out a game Hearth doesn't know: {:?}", id.chars().take(40).collect::<String>()));
            continue;
        }
        if out.iter().any(|x| x.id == id) { continue; }
        let family = g["family"].as_str().filter(|f| ["bfme", "lotr", "other"].contains(f)).unwrap_or(default_family(id)).to_string();
        let issues = g["issues"].as_array().map(|a| a.iter().take(20).filter_map(|i| {
            let level = if i["level"] == "error" { "error" } else { "warn" };
            text(&i["message"], 400).map(|m| (level.to_string(), m))
        }).collect()).unwrap_or_default();
        let selected = g["selected_mod"].as_str().filter(|m| valid_mod_id(m)).map(str::to_string);
        if g["selected_mod"].is_string() && selected.is_none() { crate::bridge::log(&format!("beacon: {id}: ignored a mod id that isn't safe to use")); }
        out.push(BeaconGame {
            id: id.to_string(), family,
            title: text(&g["title"], 120).unwrap_or_else(|| id.to_uppercase()),
            short: text(&g["short"], 40).unwrap_or_else(|| id.to_uppercase()),
            path: g["path"].as_str().filter(|p| p.starts_with('/') && !p.contains('\0') && p.len() < 4096).map(str::to_string),
            installed: g["installed"] == true, ready: g["ready"] == true, issues, selected_mod: selected,
            edition: text(&g["edition"], 80),
            steam_appid: g["steam_appid"].as_u64().filter(|n| *n > 0 && *n < crate::appid::OWN_MIN as u64).map(|n| n as u32),
            via_steam: g["via_steam"] == true, mods: vec![],
        });
    }
    Ok(out)
}

/// A mod's name from its id: "rotwk:age-of-the-ring" → "Age of the Ring".
pub fn mod_name(id: &str) -> String {
    let slug = id.rsplit(':').next().unwrap_or(id);
    let small = ["of", "the", "and", "a", "in", "for"];
    slug.split(['-', '_', '.']).filter(|w| !w.is_empty()).enumerate().map(|(i, w)| {
        if i > 0 && small.contains(&w) { w.to_string() } else { let mut c = w.chars(); c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default() }
    }).collect::<Vec<_>>().join(" ")
}

/// `beacon mods <game>`: one mod a line, "<* or space> <id> <mod|launcher> <path>". Anything else (the "No mods"
/// line) is skipped, as are ids that aren't safe and lines whose path isn't absolute. Returns the mods and the one marked chosen.
pub fn parse_mods(out: &str) -> (Vec<Mod>, Option<String>) {
    let mut mods: Vec<Mod> = Vec::new();
    let mut chosen = None;
    for line in out.lines() {
        let Some(first) = line.chars().next() else { continue };
        if first != '*' && first != ' ' { continue; }
        let rest = line[first.len_utf8()..].trim_start();
        let Some((id, rest)) = rest.split_once(char::is_whitespace) else { continue };
        let rest = rest.trim_start();
        let (kind, path) = rest.split_once(char::is_whitespace).map(|(k, p)| (k, p.trim())).unwrap_or((rest.trim(), ""));
        // Beacon prints absolute paths; anything else means the line isn't what it looks like (an id with a space).
        if !["mod", "launcher"].contains(&kind) || !path.starts_with('/') { continue; }
        if !valid_mod_id(id) { crate::bridge::log(&format!("beacon: left out a mod whose id isn't safe to use: {:?}", id.chars().take(40).collect::<String>())); continue; }
        if mods.iter().any(|m| m.id == id) || mods.len() >= MAX_MODS { continue; }
        if first == '*' { chosen = Some(id.to_string()); }
        mods.push(Mod { id: id.to_string(), name: mod_name(id), kind: kind.to_string(), path: path.chars().take(4096).collect() });
    }
    (mods, chosen)
}

// ------------------------------------------------------------------ the arguments Hearth passes

/// The mod to start with: Some(id) that mod, Some("") none (the base game), None Beacon's saved choice.
pub fn launch_args(game: &str, choice: Option<&str>) -> Result<Vec<String>, String> {
    if !known_game(game) { return Err("Beacon doesn't know that game".into()); }
    let mut a = vec!["launch".to_string(), game.to_string()];
    match choice {
        None => {}
        Some("") => a.push("--vanilla".into()),
        Some(m) if valid_mod_id(m) => a.push(format!("--mod={m}")),
        Some(_) => return Err("That mod's name can't be used".into()),
    }
    a.push("--wait".into());
    Ok(a)
}
/// `select-mod`: Some(id) a mod, None the base game.
pub fn select_args(game: &str, choice: Option<&str>) -> Result<Vec<String>, String> {
    if !known_game(game) { return Err("Beacon doesn't know that game".into()); }
    let m = match choice { None | Some("") => "none".to_string(), Some(m) if valid_mod_id(m) => m.to_string(), Some(_) => return Err("That mod's name can't be used".into()) };
    Ok(vec!["select-mod".into(), game.into(), m])
}
pub fn mods_args(game: &str) -> Result<Vec<String>, String> {
    if !known_game(game) { return Err("Beacon doesn't know that game".into()); }
    Ok(vec!["mods".into(), game.into()])
}

// ------------------------------------------------------------------ running it

fn run(exe: &Exe, args: &[String], limit: Duration) -> Result<String, String> {
    let (prog, argv) = exe.command(args, 0);
    let argv: Vec<&str> = argv.iter().map(String::as_str).collect();
    crate::system::run_timeout(&prog.to_string_lossy(), &argv, limit)
}

/// Everything Hearth knows about Beacon right now.
#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    pub cli: Option<Exe>,
    pub app: Option<Exe>,
    pub games: Vec<BeaconGame>,
    pub error: Option<String>,
    /// signature() when it was read.
    pub sig: String,
}

/// Beacon's config file and mods folder (native and Flatpak), as modification times; plus whether Beacon is
/// there at all. Cheap (a few stats), so it can be looked at every few seconds.
pub fn signature(home: &Path) -> String {
    let path_env = std::env::var("PATH").unwrap_or_default();
    let cfg = std::env::var_os("BEACON_CONFIG").map(PathBuf::from)
        .unwrap_or_else(|| std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| home.join(".config")).join("beacon/config.json"));
    let data = std::env::var_os("BEACON_DATA_DIR").map(PathBuf::from)
        .unwrap_or_else(|| std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|| home.join(".local/share")).join("beacon"));
    let fp = home.join(".var/app").join(FLATPAK_ID);
    let mut paths = vec![cfg, fp.join("config/beacon/config.json")];
    for d in [data.join("mods"), fp.join("data/beacon/mods")] {
        paths.push(d.clone());
        for g in GAMES { paths.push(d.join(g)); }
    }
    let mtime = |p: &Path| std::fs::metadata(p).and_then(|m| m.modified()).ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_millis()).unwrap_or(0);
    let mut s: Vec<String> = paths.iter().map(|p| mtime(p).to_string()).collect();
    s.push(format!("{:?}", locate_cli(home, &path_env, &system_flatpak_roots())));
    s.join(",")
}

/// Reads Beacon's games and mods (runs `beacon` once, plus once a game that's set up). Slow-ish: off the UI's
/// way where it can be.
pub fn load(home: &Path) -> Snapshot {
    let path_env = std::env::var("PATH").unwrap_or_default();
    let roots = system_flatpak_roots();
    let mut s = Snapshot { cli: locate_cli(home, &path_env, &roots), app: locate_app(home, &path_env, &roots), sig: signature(home), ..Default::default() };
    let Some(cli) = s.cli.clone() else { return s };
    match run(&cli, &["status".to_string(), "--json".to_string()], Duration::from_secs(20)).and_then(|t| parse_status(&t)) {
        Ok(games) => s.games = games,
        // Taken again after Beacon ran: a Beacon that writes its own files while answering isn't read over and over.
        Err(e) => { crate::bridge::log(&format!("beacon: status: {e}")); s.error = Some(e); s.sig = signature(home); return s; }
    }
    for g in s.games.iter_mut().filter(|g| g.installed) {
        let Ok(args) = mods_args(&g.id) else { continue };
        match run(&cli, &args, Duration::from_secs(15)) {
            Ok(t) => { let (mods, chosen) = parse_mods(&t); g.mods = mods; if g.selected_mod.is_none() { g.selected_mod = chosen; } }
            Err(e) => crate::bridge::log(&format!("beacon: mods {}: {e}", g.id)),
        }
    }
    s.sig = signature(home);
    crate::bridge::log(&format!("beacon: {} games ({} set up) through {}", s.games.len(), s.games.iter().filter(|g| g.installed).count(), cli.via()));
    s
}

/// Hearth's library entries for Beacon's games: art from Steam where Beacon knows a Steam app id, otherwise
/// the UI's own tile. Games that aren't set up yet show with Install (which opens Beacon).
pub fn library_games(s: &Snapshot) -> Vec<crate::library::Game> {
    if s.cli.is_none() { return vec![]; }
    s.games.iter().map(|b| {
        let appid = b.steam_appid.or_else(|| STEAM_IDS.iter().find(|(g, _)| *g == b.id).map(|(_, n)| *n));
        let art = appid.map(|n| crate::steam::find_art(&n.to_string(), None, Path::new("/nonexistent"))).unwrap_or_default();
        crate::library::Game {
            id: format!("beacon:{}", b.id), title: b.title.clone(), source: "beacon",
            install_path: b.path.as_ref().map(PathBuf::from), installed: b.installed,
            state: if b.installed { "installed" } else { "notInstalled" }, art,
            install_uri: if b.installed { None } else { Some("beacon:open".into()) },
            beacon: Some(b.clone()), ..Default::default()
        }
    }).collect()
}

/// Starts a game through `beacon launch … --wait`. `choice`: see launch_args. Its output goes to Hearth's
/// beacon-launch.log. Returns Beacon's process id.
pub fn launch(cli: &Exe, game: &BeaconGame, choice: Option<&str>, game_id: u32) -> Result<u32, String> {
    if !game.installed { return Err(format!("{} isn't set up in Beacon yet. Open Beacon to set it up.", game.title)); }
    if let Some(m) = choice.filter(|m| !m.is_empty()) {
        if !game.mods.iter().any(|x| x.id == m) { return Err("That mod isn't in Beacon anymore.".into()); }
    }
    let args = launch_args(&game.id, choice)?;
    let (prog, argv) = cli.command(&args, game_id);
    let log = crate::paths::data_dir().join("beacon-launch.log");
    let out = std::fs::OpenOptions::new().create(true).append(true).open(&log).ok();
    let err = out.as_ref().and_then(|f| f.try_clone().ok());
    let mut c = std::process::Command::new(&prog);
    c.args(&argv).stdin(std::process::Stdio::null())
        .stdout(out.map(std::process::Stdio::from).unwrap_or_else(std::process::Stdio::null))
        .stderr(err.map(std::process::Stdio::from).unwrap_or_else(std::process::Stdio::null));
    if game_id != 0 { let (k, v) = crate::appid::env(game_id); c.env(k, v); }
    c.current_dir(crate::paths::home());
    crate::bridge::log(&format!("beacon: launch {} ({}) with HEARTH_GAME_ID={game_id}", game.id, match choice { None => "its chosen mod".to_string(), Some("") => "no mod".to_string(), Some(m) => m.to_string() }));
    c.spawn().map(crate::procs::detach).map_err(|e| format!("Couldn't start Beacon: {e}"))
}

/// Saves the mod a game starts with, in Beacon (Beacon's own window shows the same choice).
pub fn select_mod(cli: &Exe, game: &str, choice: Option<&str>) -> Result<(), String> {
    let args = select_args(game, choice)?;
    run(cli, &args, Duration::from_secs(15)).map(|_| ())
}

/// Opens Beacon's window (full screen in Game Mode). It's an app Hearth opened, not a game: no number.
pub fn open_app(app: &Exe, fullscreen: bool) -> Result<(), String> {
    let args: Vec<String> = if fullscreen { vec!["--fullscreen".into()] } else { vec![] };
    let (prog, argv) = app.command(&args, 0);
    crate::bridge::log(&format!("beacon: opening Beacon ({})", app.via()));
    std::process::Command::new(&prog).args(&argv).current_dir(crate::paths::home())
        .stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null())
        .spawn().map(crate::procs::detach).map(|_| ()).map_err(|e| format!("Couldn't open Beacon: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const STATUS: &str = r#"[
      {"id":"bfme2","family":"bfme","title":"The Battle for Middle-earth II","short":"BFME II","path":"/home/me/Games/BFME2","installed":true,
       "runner":null,"runner_choice":null,"has_cd_key":true,"resolution":null,"windowed":false,"extra_args":"","prefix_ready":true,
       "virtual_desktop":false,"steam_appid":null,"via_steam":false,"edition":null,"selected_mod":"bfme2:edain","tool_launcher":null,
       "ready":true,"issues":[{"level":"warn","message":"No CD key"}]},
      {"id":"rotwk","family":"bfme","title":"The Rise of the Witch-king","short":"RotWK","path":null,"installed":false,"ready":false,
       "issues":[{"level":"error","message":"Game folder not set"}],"selected_mod":"rotwk:x; rm -rf ~"},
      {"id":"swgb","family":"other","title":"Star Wars: Galactic Battlegrounds Saga","short":"SWGB","path":"relative/path","installed":true,
       "steam_appid":356500,"via_steam":true,"ready":true,"issues":[],"selected_mod":null,"edition":"Galactic Battlegrounds (community)"},
      {"id":"doom","family":"other","title":"Not Beacon's","installed":true},
      {"id":"bfme2","title":"twice","installed":false},
      {"id":"conquest","family":"weird","title":"The Lord of the Rings: Conquest\u0007","installed":true,"ready":true,"issues":[]}
    ]"#;

    #[test]
    fn parses_beacons_status() {
        let g = parse_status(STATUS).unwrap();
        assert_eq!(g.iter().map(|x| x.id.as_str()).collect::<Vec<_>>(), vec!["bfme2", "rotwk", "swgb", "conquest"]);
        let b2 = &g[0];
        assert_eq!((b2.family.as_str(), b2.short.as_str(), b2.installed, b2.ready), ("bfme", "BFME II", true, true));
        assert_eq!(b2.path.as_deref(), Some("/home/me/Games/BFME2"));
        assert_eq!(b2.selected_mod.as_deref(), Some("bfme2:edain"));
        assert_eq!(b2.issues, vec![("warn".to_string(), "No CD key".to_string())]);
        assert_eq!(b2.blocker(), None);
        // A mod id with a space and a semicolon is never kept.
        assert_eq!(g[1].selected_mod, None);
        assert_eq!(g[1].blocker(), Some("Game folder not set"));
        // A relative path is dropped; Steam details kept.
        assert_eq!((g[2].path.clone(), g[2].steam_appid, g[2].via_steam), (None, Some(356500), true));
        // An unknown family falls back to Beacon's own; control characters leave titles.
        assert_eq!(g[3].family, "lotr");
        assert_eq!(g[3].title, "The Lord of the Rings: Conquest");
        assert!(parse_status("not json").is_err());
        assert!(parse_status("{\"id\":\"bfme1\"}").is_err());
        assert!(parse_status("[]").unwrap().is_empty());
    }

    #[test]
    fn parses_mods() {
        let out = [
            "* rotwk:edain                    launcher  /home/me/.local/share/beacon/mods/rotwk/Edain/Edain Launcher.exe",
            "  rotwk:age-of-the-ring          mod       /home/me/mods/aotr.big",
            "  rotwk:bad mod                  mod       /x",
            "  rotwk:--evil                   mod       /x",
            "  -rotwk                         mod       /x",
            "  rotwk:other                    script    /x",
            "  rotwk:relative                 mod       mods/x.big",
            "No mods. Drop them in /x or use `beacon add-mod`.",
            "  rotwk:edain                    mod       /dup",
        ].join("\n");
        let out = out.as_str();
        let (mods, chosen) = parse_mods(out);
        assert_eq!(mods.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(), vec!["rotwk:edain", "rotwk:age-of-the-ring", "rotwk:--evil"]);
        assert_eq!(chosen.as_deref(), Some("rotwk:edain"));
        assert_eq!(mods[0].kind, "launcher");
        assert!(mods[0].path.ends_with("Edain Launcher.exe"));
        assert_eq!(mods[1].name, "Age of the Ring");
        assert_eq!(mod_name("bfme2:edain"), "Edain");
        let (none, c) = parse_mods("No mods. Drop them in /home/me/.local/share/beacon/mods/bfme1 or use `beacon add-mod`.\n");
        assert!(none.is_empty() && c.is_none());
    }

    #[test]
    fn checks_ids() {
        for ok in ["rotwk:edain", "bfme2:age-of-the-ring-8-4", "aotr:x.y_z", "edain"] { assert!(valid_mod_id(ok), "{ok}"); }
        let long = "x".repeat(129);
        for bad in ["", "rotwk:my mod", "rotwk:a;b", "-x", "--mod=x", ".hidden", ":x", "rotwk:a/b", "rotwk:a$b", "ä", long.as_str()] { assert!(!valid_mod_id(bad), "{bad}"); }
        assert!(known_game("rotwk") && known_game("swgb") && !known_game("ROTWK") && !known_game("bfme") && !known_game("../x"));
        assert!(rts_by_default("beacon:bfme2") && rts_by_default("beacon:swgb"));
        assert!(!rts_by_default("beacon:conquest") && !rts_by_default("steam:1") && !rts_by_default("beacon:nope"));
    }

    #[test]
    fn builds_arguments() {
        assert_eq!(launch_args("rotwk", None).unwrap(), vec!["launch", "rotwk", "--wait"]);
        assert_eq!(launch_args("rotwk", Some("")).unwrap(), vec!["launch", "rotwk", "--vanilla", "--wait"]);
        assert_eq!(launch_args("rotwk", Some("rotwk:edain")).unwrap(), vec!["launch", "rotwk", "--mod=rotwk:edain", "--wait"]);
        assert!(launch_args("rotwk", Some("my mod")).is_err());
        assert!(launch_args("rotwk", Some("--dry-run")).is_err());
        assert!(launch_args("doom", None).is_err());
        assert!(launch_args("rotwk --dry-run", None).is_err());
        assert_eq!(select_args("bfme2", Some("bfme2:edain")).unwrap(), vec!["select-mod", "bfme2", "bfme2:edain"]);
        assert_eq!(select_args("bfme2", None).unwrap(), vec!["select-mod", "bfme2", "none"]);
        assert!(select_args("bfme2", Some("a b")).is_err());
        assert_eq!(mods_args("aotr").unwrap(), vec!["mods", "aotr"]);
        assert!(mods_args("x").is_err());
    }

    #[test]
    fn command_forms() {
        let a = launch_args("bfme2", Some("bfme2:edain")).unwrap();
        let (p, argv) = Exe::Native("/usr/bin/beacon".into()).command(&a, 42);
        assert_eq!(p, PathBuf::from("/usr/bin/beacon"));
        assert_eq!(argv, vec!["launch", "bfme2", "--mod=bfme2:edain", "--wait"]);
        let (p, argv) = Exe::Flatpak("/usr/bin/flatpak".into(), Some("beacon")).command(&a, 42);
        assert_eq!(p, PathBuf::from("/usr/bin/flatpak"));
        assert_eq!(argv, vec!["run", "--command=beacon", "--env=HEARTH_GAME_ID=42", FLATPAK_ID, "launch", "bfme2", "--mod=bfme2:edain", "--wait"]);
        // Beacon's window: no number, no --command.
        let (_, argv) = Exe::Flatpak("/usr/bin/flatpak".into(), None).command(&["--fullscreen".to_string()], 0);
        assert_eq!(argv, vec!["run", FLATPAK_ID, "--fullscreen"]);
    }

    #[test]
    fn finds_beacon() {
        use std::os::unix::fs::PermissionsExt;
        let t = tempfile::tempdir().unwrap();
        let home = t.path().join("home");
        let bin = t.path().join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        let mk = |p: &Path, exec: bool| {
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, "#!/bin/sh\n").unwrap();
            std::fs::set_permissions(p, std::fs::Permissions::from_mode(if exec { 0o755 } else { 0o644 })).unwrap();
        };
        let none: Vec<PathBuf> = vec![];
        let path_env = format!("relative:{}", bin.display());
        // Nothing yet (a file that isn't executable doesn't count, nor does a relative PATH folder).
        mk(&bin.join("beacon"), false);
        assert_eq!(find_native(&home, &path_env, "beacon").filter(|p| p.starts_with(t.path())), None);
        // On PATH.
        mk(&bin.join("beacon"), true);
        assert_eq!(locate_cli(&home, &path_env, &none), Some(Exe::Native(bin.join("beacon"))));
        // The Flatpak: its tool only counts when the Flatpak ships it; the window always.
        let other = t.path().join("bin2");
        std::fs::create_dir_all(&other).unwrap();
        mk(&other.join("flatpak"), true);
        let pe = other.display().to_string();
        let sys = t.path().join("sys");
        std::fs::create_dir_all(sys.join(FLATPAK_ID).join("current/active/files")).unwrap();
        let roots = vec![sys.clone()];
        if find_native(&home, &pe, "beacon").is_none() {
            assert_eq!(locate_cli(&home, &pe, &roots), None);
            assert_eq!(locate_app(&home, &pe, &roots), Some(Exe::Flatpak(other.join("flatpak"), None)));
            mk(&sys.join(FLATPAK_ID).join("current/active/files/bin/beacon"), true);
            assert_eq!(locate_cli(&home, &pe, &roots), Some(Exe::Flatpak(other.join("flatpak"), Some("beacon"))));
        }
    }

    #[test]
    fn library_entries() {
        let mut s = Snapshot::default();
        s.games = parse_status(STATUS).unwrap();
        assert!(library_games(&s).is_empty());   // no Beacon, no games
        s.cli = Some(Exe::Native("/usr/bin/beacon".into()));
        let g = library_games(&s);
        assert_eq!(g[0].id, "beacon:bfme2");
        assert_eq!((g[0].source, g[0].installed, g[0].state), ("beacon", true, "installed"));
        assert_eq!((g[1].installed, g[1].install_uri.as_deref()), (false, Some("beacon:open")));
        assert!(g[2].art.iter().any(|(k, v)| *k == "cover" && v.contains("356500")));
        assert!(g[0].art.is_empty());
    }
}
