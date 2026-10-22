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
//! Since Beacon 0.0.6 Hearth has Beacon's screens itself (Library › Beacon › a game › Game setup / Mods), through
//! Beacon's JSON commands (`beacon --json …`, README "For Hearth / scripting"): status, mods, catalog, community,
//! runners and log to read; set-path, set-key (the CD key on stdin, never on the command line), set-runner,
//! resolution, windowed, edition, virtual-desktop, via-steam, args, add-mod, remove-mod to change things; scan,
//! find-mods, setup, run, add-files and (Beacon 0.0.8+) aio-launcher get as long jobs with progress lines. Every value is checked here first
//! (bridge_call); every Beacon process has a time limit; Hearth changes one thing in Beacon at a time (Busy), off
//! the bridge thread, then reads Beacon again in the background (refresh → Library::beacon_tick).
//!
//! Where `beacon` comes from: a `beacon` executable on PATH, in ~/.local/bin, /usr/bin, /usr/local/bin or
//! /opt/beacon/bin; or Beacon's Flatpak (FLATPAK_ID), but only when that Flatpak ships the `beacon` tool
//! (files/bin/beacon), run as `flatpak run --command=beacon <id> …`. Game ids are checked against Beacon's own
//! list (GAMES) and mod ids against valid_mod_id before they go on a command line.
//!
//! Hearth notices changes (a game set up or a mod added in Beacon) by watching Beacon's config file and mods
//! folder (signature()); when they change the games are read again in the background (Library::tick).

use serde_json::{json, Value};
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
/// The games the BFME All-in-One launcher installs.
pub const AIO_GAMES: [&str; 3] = ["bfme1", "bfme2", "rotwk"];
/// Jobs that open a window of their own (an installer, a game's launcher, the All-in-One launcher): in Game Mode
/// that window is put in front while the job runs (show_job_windows), or it would sit behind Hearth.
pub fn job_has_window(job: &str) -> bool { matches!(job, "install" | "patch" | "tool" | "addFiles" | "aioGet") }

/// Game Mode: while a Beacon job runs, the windows its processes open are pinned in front (gamescope only shows
/// what it's asked for, and nobody asks for a Wine installer). Hearth comes back when the job is done.
fn show_job_windows(stop: std::sync::Arc<std::sync::atomic::AtomicBool>) {
    use std::sync::atomic::Ordering;
    if !crate::gamescope::inside() { return; }
    std::thread::spawn(move || {
        let proc_root = Path::new("/proc");
        let mut ours: Option<u64> = None;
        while !stop.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(1000));
            let jobs = job_pids();
            let wins = crate::gamescope::focusable_windows();
            if ours.map(|w| wins.iter().any(|x| x.win == w)).unwrap_or(false) { continue; }
            let pick = wins.iter().rev().find(|w| w.pid > 0 && w.pid < u32::MAX as u64
                && jobs.iter().any(|j| crate::procs::descends_from(proc_root, w.pid as u32, *j)));
            if let Some(w) = pick {
                crate::bridge::log(&format!("beacon: showing the job's window 0x{:x} (process {})", w.win, w.pid));
                crate::gamescope::focus_window(w.win);
                ours = Some(w.win);
            } else if ours.take().is_some() && crate::gamescope::pinned().is_some() {
                crate::gamescope::clear_pin();
            }
        }
        if ours.is_some() && crate::gamescope::pinned() == ours { crate::gamescope::clear_pin(); }
    });
}

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
    if !game.installed { return Err(format!("{} isn't set up yet: select Set up on its page.", game.title)); }
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

// ------------------------------------------------------------------ Beacon's screens in Hearth (Beacon 0.0.6+)

/// Time limits: reading something, a quick change, the long jobs.
const READ_LIMIT: Duration = Duration::from_secs(20);
const QUICK_LIMIT: Duration = Duration::from_secs(20);
const SCAN_LIMIT: Duration = Duration::from_secs(30 * 60);
const SETUP_LIMIT: Duration = Duration::from_secs(30 * 60);
/// Installers and patches wait for the person to click through them.
const PROGRAM_LIMIT: Duration = Duration::from_secs(6 * 60 * 60);
/// The most of Beacon's output Hearth keeps (a status list is a few KB).
const MAX_OUTPUT: usize = 4 << 20;
pub const RELEASES: &str = "https://github.com/Dragongamer9/beacon/releases";

/// What Hearth is having Beacon do right now: one change at a time, since each Beacon process reads its config at the
/// start and writes it at the end (two at once would lose one's change).
static BUSY: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);
/// Beacon processes running long jobs: Hearth's fail-safe (watchdog.rs) leaves them be.
static JOB_PIDS: std::sync::Mutex<Vec<u32>> = std::sync::Mutex::new(Vec::new());

pub fn job_pids() -> Vec<u32> { JOB_PIDS.lock().unwrap_or_else(|e| e.into_inner()).clone() }
pub fn busy_with() -> Option<String> { BUSY.lock().unwrap_or_else(|e| e.into_inner()).clone() }

/// Held while Beacon changes something; let go when dropped.
#[derive(Debug)]
pub struct Busy;
impl Busy {
    pub fn take(what: &str) -> Result<Busy, String> {
        let mut b = BUSY.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(w) = b.as_ref() { return Err(format!("Beacon is still busy ({w}). Try again when it's done.")); }
        *b = Some(what.to_string());
        Ok(Busy)
    }
}
impl Drop for Busy {
    fn drop(&mut self) { *BUSY.lock().unwrap_or_else(|e| e.into_inner()) = None; }
}

fn untrack(pid: u32) { JOB_PIDS.lock().unwrap_or_else(|e| e.into_inner()).retain(|p| *p != pid); }

/// One line of `beacon --json` output: a progress line goes to `on_progress`, any other JSON object is the answer.
fn take_line(line: &str, result: &mut Option<Value>, on_progress: &mut dyn FnMut(Option<u64>, &str)) {
    let t = line.trim();
    if !t.starts_with('{') { return; }
    let Ok(v) = serde_json::from_str::<Value>(t) else { return };
    if v["event"] == "progress" {
        on_progress(v["pct"].as_u64().map(|p| p.min(100)), &text(&v["text"], 300).unwrap_or_default());
    } else if v.is_object() {
        *result = Some(v);
    }
}

/// The answer in Beacon's output, as `run_json` reads it: the last JSON line that isn't progress, else the whole
/// output as one JSON document (`status` prints its list over many lines). Some(Err) for `{"ok":false}`.
pub fn answer(result: Option<Value>, out: &str) -> Option<Result<Value, String>> {
    let v = match result { Some(v) => v, None => serde_json::from_str::<Value>(out.trim()).ok()? };
    if v["ok"] == false {
        return Some(Err(text(&v["error"], 400).unwrap_or_else(|| "Beacon couldn't do that.".into())));
    }
    Some(Ok(v))
}

fn quiet(_: Option<u64>, _: &str) {}

/// A line of output: read for progress and the answer, and kept (up to MAX_OUTPUT) for answers over many lines.
fn keep(line: String, result: &mut Option<Value>, text_out: &mut String, on_progress: &mut dyn FnMut(Option<u64>, &str)) {
    take_line(&line, result, on_progress);
    if text_out.len() + line.len() < MAX_OUTPUT { text_out.push_str(&line); text_out.push('\n'); }
}

/// Runs `beacon --json <args>` (an argument list, never a shell) for at most `limit`, giving `stdin` (the CD key)
/// on its standard input. Progress lines go to `on_progress`. `track`: a long job, which the fail-safe leaves be.
pub fn run_json(exe: &Exe, args: &[String], stdin: Option<&str>, limit: Duration, track: bool,
                on_progress: &mut dyn FnMut(Option<u64>, &str)) -> Result<Value, String> {
    use std::io::{BufRead, BufReader, Read, Write};
    use std::process::{Command, Stdio};
    use std::sync::mpsc::{channel, RecvTimeoutError};
    let mut all = vec!["--json".to_string()];
    all.extend(args.iter().cloned());
    let (prog, argv) = exe.command(&all, 0);
    let mut child = Command::new(&prog).args(&argv).env("LC_ALL", "C").current_dir(crate::paths::home())
        .stdin(if stdin.is_some() { Stdio::piped() } else { Stdio::null() })
        .stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()
        .map_err(|e| format!("Couldn't run Beacon: {e}"))?;
    let pid = child.id();
    if track { JOB_PIDS.lock().unwrap_or_else(|e| e.into_inner()).push(pid); }
    if let Some(t) = stdin {
        // Written and closed at once (Beacon reads one line); never logged.
        if let Some(mut si) = child.stdin.take() { let _ = si.write_all(t.as_bytes()); let _ = si.write_all(b"\n"); }
    }
    // Read on threads, so a long job's progress comes as it's printed and a full pipe never stalls Beacon. Not joined:
    // a Windows program Beacon started could keep the pipe open after Beacon is done.
    let (tx, rx) = channel::<String>();
    let out = child.stdout.take();
    std::thread::spawn(move || {
        if let Some(o) = out { for line in BufReader::new(o).lines().map_while(Result::ok) { if tx.send(line).is_err() { break; } } }
    });
    let (etx, erx) = channel::<String>();
    let err = child.stderr.take();
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(mut e) = err { let _ = e.by_ref().take(64 * 1024).read_to_end(&mut buf); let _ = std::io::copy(&mut e, &mut std::io::sink()); }
        let _ = etx.send(String::from_utf8_lossy(&buf).into_owned());
    });
    let start = std::time::Instant::now();
    let (mut result, mut text_out): (Option<Value>, String) = (None, String::new());
    let status = loop {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(line) => keep(line, &mut result, &mut text_out, &mut *on_progress),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => std::thread::sleep(Duration::from_millis(20)),
        }
        match child.try_wait() {
            Ok(Some(st)) => break st,
            Ok(None) if start.elapsed() > limit => {
                let _ = child.kill();
                let _ = child.wait();
                untrack(pid);
                crate::bridge::log(&format!("beacon: {} took too long; stopped", args.first().map(String::as_str).unwrap_or("?")));
                return Err("Beacon took too long, so Hearth stopped it.".into());
            }
            Ok(None) => {}
            Err(e) => { untrack(pid); return Err(e.to_string()); }
        }
    };
    untrack(pid);
    // What's still in the pipe.
    while let Ok(line) = rx.recv_timeout(Duration::from_millis(300)) { keep(line, &mut result, &mut text_out, &mut *on_progress); }
    match answer(result, &text_out) {
        Some(r) => r,
        None => {
            let e = erx.recv_timeout(Duration::from_millis(300)).unwrap_or_default();
            let last = e.lines().filter(|l| !l.trim().is_empty()).last().map(|l| l.trim().trim_start_matches("Error: ").chars().take(300).collect::<String>());
            Err(match (status.success(), last) {
                (_, Some(l)) if l.contains("unexpected argument") || l.contains("unrecognized subcommand") => "This needs a newer Beacon (0.0.6 or later).".to_string(),
                (_, Some(l)) => l,
                (true, None) => "Beacon didn't answer. It may need updating (0.0.6 or later).".into(),
                (false, None) => "Beacon couldn't do that.".into(),
            })
        }
    }
}

// ---- checking what the page sends, before it goes on a command line

/// A game from the page: "beacon:rotwk" or "rotwk".
pub fn page_game(v: &Value) -> Result<String, String> {
    let s = v.as_str().unwrap_or("");
    let g = s.strip_prefix("beacon:").unwrap_or(s);
    if known_game(g) { Ok(g.to_string()) } else { Err("Beacon doesn't know that game.".into()) }
}

/// Edition and setup-step ids (Beacon's are lowercase words with dashes).
pub fn valid_slug(s: &str) -> bool {
    !s.is_empty() && s.len() <= 64 && !s.starts_with('-') && s.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// A runner id as Beacon makes them ("proton:GE-Proton9-20", "wine:system", "umu"): Proton folders can have
/// spaces. It must also be in Beacon's own list (checked against `runners` before it's used).
pub fn valid_runner_id(s: &str) -> bool {
    !s.is_empty() && s.len() <= 200 && !s.starts_with('-') && !s.chars().any(|c| c.is_control())
}

/// "1920x1080" (Beacon's smallest is 640x480).
pub fn valid_resolution(s: &str) -> Option<(u32, u32)> {
    let (w, h) = s.split_once('x')?;
    if w.is_empty() || h.is_empty() || w.len() > 5 || h.len() > 5 || !w.bytes().chain(h.bytes()).all(|b| b.is_ascii_digit()) { return None; }
    let (w, h): (u32, u32) = (w.parse().ok()?, h.parse().ok()?);
    ((640..=16384).contains(&w) && (480..=16384).contains(&h)).then_some((w, h))
}

/// A CD key as printed on the case: letters, digits and dashes. Returned trimmed.
pub fn valid_cd_key(s: &str) -> Option<String> {
    let k = s.trim();
    (k.len() >= 4 && k.len() <= 64 && k.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')).then(|| k.to_string())
}

/// A file or folder the person picked: absolute, there, and the right kind.
pub fn existing_path(p: &str, dir: bool) -> Result<String, String> {
    if !p.starts_with('/') || p.contains('\0') || p.len() > 4096 { return Err("That isn't a file Hearth can use.".into()); }
    let path = Path::new(p);
    match (dir, std::fs::metadata(path)) {
        (true, Ok(m)) if m.is_dir() => Ok(p.to_string()),
        (false, Ok(m)) if m.is_file() => Ok(p.to_string()),
        (true, Ok(_)) => Err("Choose a folder.".into()),
        (false, Ok(_)) => Err("Choose a file.".into()),
        (_, Err(_)) => Err("That file isn't there anymore.".into()),
    }
}

fn on_off(v: &Value) -> Result<&'static str, String> {
    match v { Value::Bool(true) => Ok("on"), Value::Bool(false) => Ok("off"), _ => match v.as_str() { Some("on") => Ok("on"), Some("off") => Ok("off"), _ => Err("Expected on or off.".into()) } }
}

/// A setting from Game setup as Beacon's arguments. `what`: runner, resolution, windowed, edition, virtualDesktop,
/// viaSteam or args.
pub fn set_args(game: &str, what: &str, v: &Value) -> Result<Vec<String>, String> {
    if !known_game(game) { return Err("Beacon doesn't know that game.".into()); }
    let s = v.as_str().unwrap_or("");
    let mk = |cmd: &str, val: String| vec![cmd.to_string(), game.to_string(), val];
    Ok(match what {
        "runner" if s == "auto" || valid_runner_id(s) => mk("set-runner", s.to_string()),
        "resolution" if s == "clear" => mk("resolution", "clear".into()),
        "resolution" => { let (w, h) = valid_resolution(s).ok_or("Choose a resolution like 1920x1080.")?; mk("resolution", format!("{w}x{h}")) }
        "windowed" => mk("windowed", on_off(v)?.into()),
        "edition" if s == "auto" || valid_slug(s) => mk("edition", s.to_string()),
        "virtualDesktop" if s == "default" => mk("virtual-desktop", "default".into()),
        "virtualDesktop" => mk("virtual-desktop", on_off(v)?.into()),
        "viaSteam" => mk("via-steam", on_off(v)?.into()),
        "args" => {
            if s.chars().count() > 400 || s.chars().any(|c| c.is_control()) { return Err("Those arguments can't be used.".into()); }
            // After "--", so arguments starting with a dash are the game's, not Beacon's.
            vec!["args".into(), game.into(), "--".into(), s.trim().to_string()]
        }
        "runner" | "edition" => return Err("That choice can't be used.".into()),
        _ => return Err("Hearth can't change that in Beacon.".into()),
    })
}

/// A long job as Beacon's arguments and its time limit. `game` for all but scan and findMods; `file`: the program
/// (install, patch), the download (addFiles) or a folder to look in (scan); `step`: addFiles' setup step.
pub fn job_args(job: &str, game: Option<&str>, file: Option<&str>, step: Option<&str>) -> Result<(Vec<String>, Duration), String> {
    let g = || game.filter(|g| known_game(g)).map(str::to_string).ok_or_else(|| "Beacon doesn't know that game.".to_string());
    let f = || file.map(str::to_string).ok_or_else(|| "Choose a file first.".to_string());
    Ok(match job {
        "setup" => (vec!["setup".into(), g()?], SETUP_LIMIT),
        "install" | "patch" => (vec!["run".into(), g()?, f()?], PROGRAM_LIMIT),
        "tool" => (vec!["run".into(), g()?, "--tool".into()], PROGRAM_LIMIT),
        "scan" => { let mut a = vec!["scan".to_string()]; if let Some(d) = file { a.push(d.to_string()); } (a, SCAN_LIMIT) }
        "findMods" => (vec!["find-mods".into()], SCAN_LIMIT),
        // BFME, BFME II and RotWK through the BFME Foundation's All-in-One launcher (Beacon 0.0.8+): Beacon opens
        // it, waits while the player installs the game there, then closes it and adds the game. Beacon never
        // downloads the game itself; the launcher does.
        "aioGet" => {
            let g = g()?;
            if !AIO_GAMES.contains(&g.as_str()) { return Err("Only BFME, BFME II and RotWK come from the All-in-One launcher.".into()); }
            (vec!["aio-launcher".into(), "get".into(), g], PROGRAM_LIMIT)
        }
        "addFiles" => {
            let s = step.filter(|s| valid_slug(s)).ok_or("That setup step can't be used.")?;
            (vec!["add-files".into(), g()?, s.to_string(), f()?], PROGRAM_LIMIT)
        }
        _ => return Err("Hearth can't do that with Beacon.".into()),
    })
}

/// What a job is called while it runs (for "Beacon is still busy (…)" and the progress row).
pub fn job_label(job: &str) -> &'static str {
    match job {
        "setup" => "preparing the game", "install" => "running the installer", "patch" => "running the patch",
        "tool" => "running the game's launcher", "scan" => "looking for games", "findMods" => "looking for mods",
        "addFiles" => "adding the files", "aioGet" => "installing through the All-in-One launcher", _ => "working",
    }
}

// ---- what the page gets: Beacon's JSON, checked and in the page's words

fn abs_path(v: &Value) -> Option<String> {
    v.as_str().filter(|p| p.starts_with('/') && !p.contains('\0') && p.len() < 4096).map(str::to_string)
}
fn https(v: &Value) -> Option<String> {
    v.as_str().filter(|u| valid_link(u)).map(str::to_string)
}
/// A web page Hearth may open: https, one line, no spaces or quotes.
pub fn valid_link(u: &str) -> bool {
    u.starts_with("https://") && u.len() > 8 && u.len() <= 2048 && !u.chars().any(|c| c.is_control() || c.is_whitespace() || c == '"' || c == '\'' || c == '<' || c == '>' || c == '\\')
}
fn word(v: &Value, allowed: &[&str]) -> Option<String> { v.as_str().filter(|s| allowed.contains(s)).map(str::to_string) }

/// One game's GameStatus, for Game setup.
pub fn clean_status(st: &Value) -> Value {
    let issues: Vec<Value> = st["issues"].as_array().map(|a| a.iter().take(20).filter_map(|i| {
        text(&i["message"], 400).map(|m| json!({"level": if i["level"] == "error" { "error" } else { "warn" }, "message": m}))
    }).collect()).unwrap_or_default();
    let res = match (st["resolution"]["width"].as_u64(), st["resolution"]["height"].as_u64()) {
        (Some(w), Some(h)) if w > 0 && h > 0 && w <= 16384 && h <= 16384 => Some(format!("{w}x{h}")),
        _ => None,
    };
    let runner = st["runner"]["id"].as_str().filter(|r| valid_runner_id(r)).map(|id| json!({"id": id, "name": text(&st["runner"]["name"], 120).unwrap_or_else(|| id.to_string())}));
    json!({
        "title": text(&st["title"], 120), "short": text(&st["short"], 40), "family": word(&st["family"], &["bfme", "lotr", "other"]),
        "installed": st["installed"] == true, "ready": st["ready"] == true, "path": abs_path(&st["path"]), "issues": issues,
        "runner": runner, "runnerChoice": st["runner_choice"].as_str().filter(|r| valid_runner_id(r)),
        "hasCdKey": st["has_cd_key"] == true, "wantsCdKey": st["wants_cd_key"] == true,
        "resolution": res, "windowed": st["windowed"] == true, "extraArgs": text(&st["extra_args"], 400).unwrap_or_default(),
        "prefixReady": st["prefix_ready"] == true, "virtualDesktop": st["virtual_desktop"] == true,
        "vdChoice": st["virtual_desktop_choice"].as_bool(), "vdDefault": st["virtual_desktop_default"] == true,
        "steamAppid": st["steam_appid"].as_u64().filter(|n| *n > 0 && *n < crate::appid::OWN_MIN as u64), "viaSteam": st["via_steam"] == true,
        "edition": text(&st["edition"], 80), "selectedMod": st["selected_mod"].as_str().filter(|m| valid_mod_id(m)),
        "toolLauncher": abs_path(&st["tool_launcher"]).is_some(), "sage": st["sage"] == true,
        "requires": st["requires"].as_str().filter(|g| known_game(g)), "needsSetup": st["needs_setup"].as_bool(),
        "hasLog": st["has_log"] == true, "hasCommunity": st["has_community"] == true,
    })
}

/// Game setup and Mods for one game: status, mods, catalog, community versions, runners (all checked).
pub fn clean_details(game: &str, st: &Value, mods: &Value, catalog: &Value, community: &Value, runners: &Value) -> Value {
    let mut ids: Vec<String> = Vec::new();
    let mod_list: Vec<Value> = mods["mods"].as_array().map(|a| a.iter().take(MAX_MODS).filter_map(|m| {
        let id = m["id"].as_str().filter(|i| valid_mod_id(i))?;
        if ids.iter().any(|x| x == id) { return None; }
        ids.push(id.to_string());
        Some(json!({"id": id, "name": text(&m["name"], 120).unwrap_or_else(|| mod_name(id)), "kind": word(&m["kind"], &["mod", "launcher"]).unwrap_or_else(|| "mod".into()),
                    "path": abs_path(&m["path"]), "linked": m["in_library"] != true}))
    }).collect()).unwrap_or_default();
    let items: Vec<Value> = catalog["items"].as_array().map(|a| a.iter().take(100).filter_map(|i| {
        let id = i["id"].as_str().filter(|s| valid_mod_id(s))?;
        Some(json!({"id": id, "name": text(&i["name"], 120).unwrap_or_else(|| id.to_string()), "kind": word(&i["kind"], &["mod", "patch", "game"]).unwrap_or_else(|| "mod".into()),
                    "blurb": text(&i["blurb"], 400), "url": https(&i["url"]), "installed": i["installed"].as_str().filter(|m| valid_mod_id(m))}))
    }).collect()).unwrap_or_default();
    let community = if community.is_object() {
        let eds: Vec<Value> = community["editions"].as_array().map(|a| a.iter().take(20).filter_map(|e| {
            let id = e["id"].as_str().filter(|s| valid_slug(s))?;
            Some(json!({"id": id, "name": text(&e["name"], 120).unwrap_or_else(|| id.to_string()), "blurb": text(&e["blurb"], 400),
                        "community": e["community"] == true, "installed": e["installed"] == true, "active": e["active"] == true}))
        }).collect()).unwrap_or_default();
        let steps: Vec<Value> = community["steps"].as_array().map(|a| a.iter().take(20).filter_map(|s| {
            let id = s["id"].as_str().filter(|x| valid_slug(x))?;
            Some(json!({"id": id, "title": text(&s["title"], 120).unwrap_or_else(|| id.to_string()), "detail": text(&s["detail"], 600),
                        "url": https(&s["url"]), "action": word(&s["action"], &["link", "installer", "merge", "mods"]).unwrap_or_else(|| "link".into()), "done": s["done"] == true}))
        }).collect()).unwrap_or_default();
        json!({"chosen": community["chosen"].as_str().filter(|s| valid_slug(s)), "editions": eds, "steps": steps})
    } else { Value::Null };
    let rl: Vec<Value> = runners["runners"].as_array().map(|a| a.iter().take(60).filter_map(|r| {
        let id = r["id"].as_str().filter(|s| valid_runner_id(s))?;
        Some(json!({"id": id, "name": text(&r["name"], 120).unwrap_or_else(|| id.to_string()), "kind": word(&r["kind"], &["umu", "proton", "wine"])}))
    }).collect()).unwrap_or_default();
    json!({
        "game": game, "old": false, "status": clean_status(st),
        "mods": {"selected": mods["selected"].as_str().filter(|m| valid_mod_id(m)), "list": mod_list},
        "catalog": items, "community": community, "runners": rl,
        "best": runners["best"].as_str().filter(|s| valid_runner_id(s)), "default": runners["default"].as_str().filter(|s| valid_runner_id(s)),
        "busy": busy_with(),
    })
}

fn args_of(a: &[&str]) -> Vec<String> { a.iter().map(|s| s.to_string()).collect() }

/// Everything Game setup and Mods show for one game. Several Beacon calls: runs on a thread of its own.
pub fn details(cli: &Exe, game: &str) -> Result<Value, String> {
    if !known_game(game) { return Err("Beacon doesn't know that game.".into()); }
    let status = run_json(cli, &args_of(&["status"]), None, READ_LIMIT, false, &mut quiet)?;
    let st = status.as_array().and_then(|a| a.iter().find(|g| g["id"] == game)).cloned().ok_or("Beacon doesn't list that game.")?;
    // Beacon 0.0.5 and older answer only status as JSON: the page says to update Beacon.
    let mods = match run_json(cli, &args_of(&["mods", game]), None, READ_LIMIT, false, &mut quiet) {
        Ok(v) if v.is_object() => v,
        _ => return Ok(json!({"game": game, "old": true, "status": clean_status(&st), "busy": busy_with()})),
    };
    let catalog = run_json(cli, &args_of(&["catalog", game]), None, READ_LIMIT, false, &mut quiet).unwrap_or(Value::Null);
    let community = if st["has_community"] == true { run_json(cli, &args_of(&["community", game]), None, READ_LIMIT, false, &mut quiet).unwrap_or(Value::Null) } else { Value::Null };
    let runners = run_json(cli, &args_of(&["runners"]), None, READ_LIMIT, false, &mut quiet).unwrap_or(Value::Null);
    Ok(clean_details(game, &st, &mods, &catalog, &community, &runners))
}

/// Beacon read again after a change, handed to the library (Library::beacon_tick applies it and tells the page).
pub fn refresh(home: &Path, slot: &std::sync::Arc<std::sync::Mutex<Option<Snapshot>>>) {
    let s = load(home);
    *slot.lock().unwrap_or_else(|e| e.into_inner()) = Some(s);
}

/// The end of a game's log.
pub fn log_tail(cli: &Exe, game: &str) -> Result<Value, String> {
    if !known_game(game) { return Err("Beacon doesn't know that game.".into()); }
    let v = run_json(cli, &args_of(&["log", game, "--lines", "300"]), None, READ_LIMIT, false, &mut quiet)?;
    let lines: Vec<String> = v["lines"].as_array().map(|a| a.iter().take(300).filter_map(|l| l.as_str())
        .map(|l| l.chars().filter(|c| !c.is_control() || *c == '\t').take(400).collect()).collect()).unwrap_or_default();
    Ok(json!({"game": game, "path": abs_path(&v["path"]), "exists": v["exists"] == true, "lines": lines}))
}

/// Opens a web page (a mod's page, Beacon's releases) in Hearth Browser, or the default browser elsewhere.
pub fn open_link(url: &str) -> Result<(), String> {
    if !valid_link(url) { return Err("That link can't be opened.".into()); }
    if crate::hearth_browser::available() { return crate::hearth_browser::open(Some(url)); }
    std::process::Command::new("xdg-open").arg(url).current_dir(crate::paths::home())
        .stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null())
        .spawn().map(crate::procs::detach).map(|_| ()).map_err(|e| format!("Couldn't open the page: {e}"))
}

/// A quick change: one Beacon call with a short limit (on the calling thread), then Beacon read again.
fn change(cli: &Exe, home: &Path, slot: &std::sync::Arc<std::sync::Mutex<Option<Snapshot>>>, game: &str, args: &[String], stdin: Option<&str>) -> Result<Value, String> {
    let busy = Busy::take("saving a setting")?;
    let r = run_json(cli, args, stdin, QUICK_LIMIT, false, &mut quiet);
    drop(busy);
    let r = r?;
    refresh(home, slot);
    let mut d = details(cli, game)?;
    d["message"] = json!(text(&r["message"], 200));
    Ok(d)
}

/// The page's Beacon calls (Game setup, Mods, the Beacon tab), other than beacon.selectMod and beacon.open. Anything
/// that runs Beacon answers later, from a thread (`reply_to`: the call being answered); file and folder pickers open
/// here first, as elsewhere in Hearth.
pub fn bridge_call(br: &mut crate::bridge::Bridge, method: &str, a: &Value, reply_to: Value) -> Result<Value, String> {
    use std::sync::Arc;
    let later = crate::bridge::DEFERRED;
    // Answers go back to the window that asked only (each window numbers its own requests).
    let to = Some(br.cur_label.clone());
    if method == "beacon.openLink" {
        open_link(a["url"].as_str().unwrap_or(""))?;
        return Ok(json!({"ok": true}));
    }
    let cli = br.library.beacon.cli.clone().ok_or("Beacon isn't installed. Library › Stores says how to get it.")?;
    let home = br.library.home.clone();
    let slot: Arc<std::sync::Mutex<Option<Snapshot>>> = br.library.beacon_slot();
    let title = |g: &str| br.library.beacon.games.iter().find(|x| x.id == g).map(|x| x.title.clone()).unwrap_or_else(|| g.to_uppercase());
    match method {
        "beacon.details" => {
            let g = page_game(&a["id"])?;
            crate::space::answer_later_to(reply_to, to, move || details(&cli, &g));
            Ok(json!(later))
        }
        "beacon.log" => {
            let g = page_game(&a["id"])?;
            crate::space::answer_later_to(reply_to, to, move || log_tail(&cli, &g));
            Ok(json!(later))
        }
        "beacon.set" => {
            let g = page_game(&a["id"])?;
            let what = a["what"].as_str().unwrap_or("").to_string();
            let args = set_args(&g, &what, &a["value"])?;
            crate::bridge::log(&format!("beacon: {} {g}", args[0]));
            crate::space::answer_later_to(reply_to, to, move || {
                // A runner must be one Beacon has.
                if what == "runner" && args[2] != "auto" {
                    let rs = run_json(&cli, &args_of(&["runners"]), None, READ_LIMIT, false, &mut quiet)?;
                    if !rs["runners"].as_array().map(|l| l.iter().any(|r| r["id"].as_str() == Some(args[2].as_str()))).unwrap_or(false) {
                        return Err("Beacon doesn't have that runner anymore.".into());
                    }
                }
                change(&cli, &home, &slot, &g, &args, None)
            });
            Ok(json!(later))
        }
        "beacon.setKey" => {
            let g = page_game(&a["id"])?;
            let clear = a["clear"] == true;
            let key = if clear { None } else { Some(valid_cd_key(a["key"].as_str().unwrap_or("")).ok_or("A CD key has only letters, numbers and dashes.")?) };
            let mut args = vec!["set-key".to_string(), g.clone()];
            if clear { args.push("--clear".into()); }
            // The key itself is never logged and never on the command line: it goes to Beacon's stdin.
            crate::bridge::log(&format!("beacon: set-key {g}{}", if clear { " --clear" } else { "" }));
            crate::space::answer_later_to(reply_to, to, move || change(&cli, &home, &slot, &g, &args, key.as_deref()));
            Ok(json!(later))
        }
        "beacon.locate" => {
            let g = page_game(&a["id"])?;
            crate::watchdog::long_task();   // a folder picker waits for the person
            let start = home.to_string_lossy().to_string();
            let Some(dir) = (br.pick_folder)(&format!("Where is {} installed?", title(&g)), &start) else { return Ok(Value::Null) };
            let dir = existing_path(&dir, true)?;
            crate::bridge::log(&format!("beacon: set-path {g}"));
            let args = vec!["set-path".to_string(), g.clone(), dir];
            crate::space::answer_later_to(reply_to, to, move || change(&cli, &home, &slot, &g, &args, None));
            Ok(json!(later))
        }
        "beacon.addMod" => {
            let g = page_game(&a["id"])?;
            crate::watchdog::long_task();
            let folder = a["folder"] == true;
            let picked = if folder { (br.pick_folder)("Choose the mod's folder", &home.to_string_lossy()) }
                         else { (br.pick_file)("Choose the mod you downloaded: a .big file or its launcher (.exe)") };
            let Some(p) = picked else { return Ok(Value::Null) };
            let p = existing_path(&p, folder)?;
            let mut args = vec!["add-mod".to_string(), g.clone(), p];
            if a["select"] == true { args.push("--select".into()); }
            crate::bridge::log(&format!("beacon: add-mod {g}"));
            crate::space::answer_later_to(reply_to, to, move || change(&cli, &home, &slot, &g, &args, None));
            Ok(json!(later))
        }
        "beacon.removeMod" => {
            let g = page_game(&a["id"])?;
            let m = a["mod"].as_str().filter(|m| valid_mod_id(m) && m.starts_with(&format!("{g}:"))).ok_or("That mod's name can't be used.")?.to_string();
            crate::bridge::log(&format!("beacon: remove-mod {m}"));
            let args = vec!["remove-mod".to_string(), m];
            crate::space::answer_later_to(reply_to, to, move || change(&cli, &home, &slot, &g, &args, None));
            Ok(json!(later))
        }
        "beacon.job" => {
            let job = a["job"].as_str().unwrap_or("").to_string();
            let game = if a["id"].is_null() || a["id"] == "" { None } else { Some(page_game(&a["id"])?) };
            if let Some(w) = busy_with() { return Err(format!("Beacon is still busy ({w}). Try again when it's done.")); }
            let file = match job.as_str() {
                "install" | "patch" | "addFiles" => {
                    crate::watchdog::long_task();
                    let g = game.clone().unwrap_or_default();
                    let folder = job == "addFiles" && a["folder"] == true;
                    let prompt = match job.as_str() {
                        "install" => format!("Choose {}'s installer (setup.exe, or the .exe on the disc)", title(&g)),
                        "patch" => "Choose the patch or installer (.exe) to run".to_string(),
                        _ => "Choose what you downloaded (an archive, a file or a folder)".to_string(),
                    };
                    let picked = if folder { (br.pick_folder)(&prompt, &home.to_string_lossy()) } else { (br.pick_file)(&prompt) };
                    let Some(p) = picked else { return Ok(Value::Null) };
                    Some(existing_path(&p, folder)?)
                }
                _ => None,
            };
            let (args, limit) = job_args(&job, game.as_deref(), file.as_deref(), a["step"].as_str())?;
            let busy = Busy::take(job_label(&job))?;
            crate::bridge::log(&format!("beacon: {job} {}", game.as_deref().unwrap_or("")));
            let ev = |name: &str, data: Value| json!({"type": "event", "name": name, "data": data});
            crate::stores::notify(ev("beacon.progress", json!({"game": game, "job": job, "pct": 0, "text": "Starting"})));
            let (g2, job2) = (game.clone(), job.clone());
            let stop_showing = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            if job_has_window(&job) { show_job_windows(stop_showing.clone()); }
            std::thread::spawn(move || {
                let r = run_json(&cli, &args, None, limit, true, &mut |pct: Option<u64>, t: &str| {
                    crate::stores::notify(ev("beacon.progress", json!({"game": g2, "job": job2, "pct": pct, "text": t})));
                });
                stop_showing.store(true, std::sync::atomic::Ordering::SeqCst);
                drop(busy);
                crate::stores::notify(ev("beacon.progress", json!({"game": g2, "job": job2, "pct": 98, "text": "Reading Beacon's games again"})));
                refresh(&home, &slot);
                let done = match r {
                    Ok(v) => {
                        crate::bridge::log(&format!("beacon: {job2}: done"));
                        json!({"game": g2, "job": job2, "ok": true, "message": text(&v["message"], 200),
                               "found": v["found"].as_array().map(|f| f.len()), "added": v["added"].as_array().map(|f| f.len()), "installed": v["installed"].as_bool(),
                               "ready": v["ready"].as_bool()})
                    }
                    Err(e) => {
                        crate::bridge::log(&format!("beacon: {job2}: {e}"));
                        json!({"game": g2, "job": job2, "ok": false, "error": e})
                    }
                };
                crate::stores::notify(ev("beacon.done", done));
            });
            Ok(json!({"started": true, "job": job, "label": job_label(&job)}))
        }
        _ => Err(format!("Unknown Beacon call: {method}")),
    }
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

    #[test]
    fn checks_what_the_page_sends() {
        assert_eq!(page_game(&json!("beacon:rotwk")).unwrap(), "rotwk");
        assert_eq!(page_game(&json!("swgb")).unwrap(), "swgb");
        assert!(page_game(&json!("beacon:doom")).is_err() && page_game(&json!(null)).is_err() && page_game(&json!("beacon:../x")).is_err());
        assert_eq!(valid_resolution("1920x1080"), Some((1920, 1080)));
        for bad in ["1920X1080", "320x200", "1920x", "x1080", "99999x1080", "1920x1080 ", "-1920x1080", "1e3x900", "19201x1080"] { assert_eq!(valid_resolution(bad), None, "{bad}"); }
        assert_eq!(valid_cd_key(" ABCD-1234-EFGH \n").as_deref(), Some("ABCD-1234-EFGH"));
        let long = "A".repeat(65);
        for bad in ["", "abc", "ABCD 1234", "ABCD;rm", "ABCD\n1234", long.as_str()] { assert!(valid_cd_key(bad).is_none(), "{bad:?}"); }
        assert!(valid_runner_id("proton:GE-Proton9-20") && valid_runner_id("proton:Proton 9.0 (Beta)") && valid_runner_id("umu"));
        assert!(!valid_runner_id("") && !valid_runner_id("--help") && !valid_runner_id("wine:a\nb"));
        assert!(valid_slug("expanding-fronts") && valid_slug("patch3") && !valid_slug("-x") && !valid_slug("A") && !valid_slug("a b") && !valid_slug(""));
        assert!(valid_link("https://www.moddb.com/mods/edain-mod") && !valid_link("http://x.y") && !valid_link("https://a b") && !valid_link("javascript:alert(1)") && !valid_link("https://"));
        let t = tempfile::tempdir().unwrap();
        let f = t.path().join("setup.exe");
        std::fs::write(&f, b"").unwrap();
        let (fs, ds) = (f.to_string_lossy().to_string(), t.path().to_string_lossy().to_string());
        assert_eq!(existing_path(&fs, false).unwrap(), fs);
        assert_eq!(existing_path(&ds, true).unwrap(), ds);
        assert!(existing_path(&fs, true).is_err() && existing_path(&ds, false).is_err());
        assert!(existing_path("relative/setup.exe", false).is_err() && existing_path("/nonexistent/x.exe", false).is_err() && existing_path("/tmp/a\0b", false).is_err());
    }

    #[test]
    fn settings_as_arguments() {
        let s = |w: &str, v: Value| set_args("rotwk", w, &v);
        assert_eq!(s("runner", json!("auto")).unwrap(), vec!["set-runner", "rotwk", "auto"]);
        assert_eq!(s("runner", json!("proton:GE-Proton9-20")).unwrap(), vec!["set-runner", "rotwk", "proton:GE-Proton9-20"]);
        assert!(s("runner", json!("--x")).is_err());
        assert_eq!(s("resolution", json!("1280x800")).unwrap(), vec!["resolution", "rotwk", "1280x800"]);
        assert_eq!(s("resolution", json!("clear")).unwrap(), vec!["resolution", "rotwk", "clear"]);
        assert!(s("resolution", json!("huge")).is_err());
        assert_eq!(s("windowed", json!(true)).unwrap(), vec!["windowed", "rotwk", "on"]);
        assert_eq!(s("windowed", json!("off")).unwrap(), vec!["windowed", "rotwk", "off"]);
        assert!(s("windowed", json!("maybe")).is_err());
        assert_eq!(set_args("swgb", "virtualDesktop", &json!("default")).unwrap(), vec!["virtual-desktop", "swgb", "default"]);
        assert_eq!(set_args("swgb", "viaSteam", &json!(false)).unwrap(), vec!["via-steam", "swgb", "off"]);
        assert_eq!(set_args("uaw", "edition", &json!("human-resistance")).unwrap(), vec!["edition", "uaw", "human-resistance"]);
        assert!(set_args("uaw", "edition", &json!("../x")).is_err());
        // Extra arguments go after "--": a leading dash is the game's, never one of Beacon's options.
        assert_eq!(s("args", json!("-noshellmap -win")).unwrap(), vec!["args", "rotwk", "--", "-noshellmap -win"]);
        assert!(s("args", json!("a\nb")).is_err() && s("args", json!("x".repeat(401))).is_err());
        assert!(s("cdKey", json!("x")).is_err() && set_args("doom", "windowed", &json!(true)).is_err());
    }

    #[test]
    fn jobs_as_arguments() {
        let (a, l) = job_args("setup", Some("bfme2"), None, None).unwrap();
        assert_eq!((a, l), (vec!["setup".to_string(), "bfme2".into()], SETUP_LIMIT));
        assert_eq!(job_args("install", Some("conquest"), Some("/m/setup.exe"), None).unwrap().0, vec!["run", "conquest", "/m/setup.exe"]);
        assert_eq!(job_args("tool", Some("aotr"), None, None).unwrap().0, vec!["run", "aotr", "--tool"]);
        assert_eq!(job_args("aioGet", Some("bfme1"), None, None).unwrap().0, vec!["aio-launcher", "get", "bfme1"]);
        assert!(job_args("aioGet", Some("uaw"), None, None).is_err(), "only the launcher's games");
        assert!(job_args("aioGet", None, None, None).is_err());
        assert!(job_has_window("aioGet") && job_has_window("install") && !job_has_window("scan") && !job_has_window("setup"));
        assert_eq!(job_args("scan", None, None, None).unwrap().0, vec!["scan"]);
        assert_eq!(job_args("findMods", None, None, None).unwrap().0, vec!["find-mods"]);
        assert_eq!(job_args("addFiles", Some("swgb"), Some("/d/EF.zip"), Some("expanding-fronts")).unwrap().0, vec!["add-files", "swgb", "expanding-fronts", "/d/EF.zip"]);
        assert!(job_args("install", Some("conquest"), None, None).is_err(), "no file");
        assert!(job_args("setup", None, None, None).is_err() && job_args("setup", Some("doom"), None, None).is_err());
        assert!(job_args("addFiles", Some("swgb"), Some("/d/x"), Some("../x")).is_err());
        assert!(job_args("rm", Some("swgb"), None, None).is_err());
        assert!(job_args("install", Some("aotr"), Some("/x"), None).unwrap().1 >= Duration::from_secs(3600));
    }

    #[test]
    fn reads_beacons_answers() {
        let mut said = vec![];
        let mut r = None;
        for l in [r#"{"event":"progress","pct":140,"text":"Looking in /home/me/Games"}"#, "plain text", r#"{"ok":true,"game":null,"message":"Found 2 games"}"#] {
            take_line(l, &mut r, &mut |p: Option<u64>, t: &str| said.push((p, t.to_string())));
        }
        assert_eq!(said, vec![(Some(100), "Looking in /home/me/Games".to_string())]);
        assert_eq!(answer(r, "").unwrap().unwrap()["message"], "Found 2 games");
        assert_eq!(answer(Some(json!({"ok": false, "kind": "usage", "error": "no runner x"})), "").unwrap().unwrap_err(), "no runner x");
        // status prints its list over many lines.
        assert_eq!(answer(None, "[\n  {\"id\": \"bfme2\"}\n]\n").unwrap().unwrap()[0]["id"], "bfme2");
        assert!(answer(None, "error: unexpected argument").is_none());
        let _b = Busy::take("preparing the game").unwrap();
        assert!(Busy::take("x").unwrap_err().contains("preparing the game"));
        drop(_b);
        assert!(Busy::take("x").is_ok());
    }

    #[test]
    fn details_for_the_page() {
        let st = json!({"id":"rotwk","family":"bfme","title":"The Rise of the Witch-king","short":"RotWK","path":"/g/RotWK","installed":true,"ready":false,
            "runner":{"id":"proton:GE-Proton9-20","name":"GE-Proton9-20","kind":"proton","path":"/x"},"runner_choice":null,"has_cd_key":false,
            "resolution":{"width":1280,"height":800},"windowed":true,"extra_args":"-noshellmap","prefix_ready":false,"virtual_desktop":false,
            "steam_appid":null,"via_steam":false,"edition":null,"selected_mod":"rotwk:edain","tool_launcher":null,"ready":false,
            "issues":[{"level":"error","message":"The prefix isn't set up"},{"level":"bogus","message":"No CD key\u{7}"}],
            "sage":true,"wants_cd_key":true,"requires":"bfme2","virtual_desktop_choice":null,"virtual_desktop_default":false,"needs_setup":true,
            "log_file":"/l/rotwk.log","has_log":false,"has_community":false});
        let mods = json!({"game":"rotwk","selected":"rotwk:edain","mods_dir":"/m","mods":[
            {"id":"rotwk:edain","name":"Edain","game":"rotwk","kind":"launcher","path":"/m/Edain/Edain Launcher.exe","args":[],"in_library":true},
            {"id":"rotwk:wide","name":"Widescreen","game":"rotwk","kind":"mod","path":"/d/wide.big","args":[],"in_library":false},
            {"id":"rotwk:bad id","name":"x","kind":"mod","path":"/x"},{"id":"rotwk:edain","name":"dup","kind":"mod","path":"/y"}]});
        let cat = json!({"game":"rotwk","items":[{"id":"edain","name":"Edain Mod","game":"rotwk","kind":"mod","blurb":"Lore-friendly","url":"https://www.moddb.com/mods/edain-mod","installed":"rotwk:edain"},
            {"id":"evil","name":"Evil","kind":"mod","url":"javascript:alert(1)","installed":null}]});
        let runners = json!({"runners":[{"id":"proton:GE-Proton9-20","name":"GE-Proton9-20","kind":"proton","path":"/x"},{"id":"--bad","name":"x","kind":"wine"}],"best":"proton:GE-Proton9-20","default":null,"umu":null});
        let d = clean_details("rotwk", &st, &mods, &cat, &Value::Null, &runners);
        let s = &d["status"];
        assert_eq!((s["resolution"].as_str(), s["windowed"].as_bool(), s["requires"].as_str()), (Some("1280x800"), Some(true), Some("bfme2")));
        assert_eq!((s["runner"]["id"].as_str(), s["wantsCdKey"].as_bool(), s["needsSetup"].as_bool(), s["sage"].as_bool()), (Some("proton:GE-Proton9-20"), Some(true), Some(true), Some(true)));
        assert_eq!(s["issues"][1], json!({"level": "warn", "message": "No CD key"}));
        assert_eq!(d["mods"]["list"].as_array().unwrap().len(), 2);
        assert_eq!((d["mods"]["list"][0]["linked"].as_bool(), d["mods"]["list"][1]["linked"].as_bool()), (Some(false), Some(true)));
        assert_eq!(d["catalog"][0]["installed"], "rotwk:edain");
        assert!(d["catalog"][1]["url"].is_null(), "only https links");
        assert_eq!(d["runners"].as_array().unwrap().len(), 1);
        assert!(d["community"].is_null());
        let c = json!({"game":"swgb","chosen":null,"editions":[{"id":"saga","name":"Saga","blurb":"x","community":false,"installed":true,"active":true}],
            "steps":[{"id":"expanding-fronts","title":"Expanding Fronts","detail":"d","url":"https://ef.example/dl","action":"mods","done":false}]});
        let d = clean_details("swgb", &json!({"id":"swgb"}), &json!({"mods":[]}), &json!({}), &c, &json!({}));
        assert_eq!((d["community"]["editions"][0]["active"].as_bool(), d["community"]["steps"][0]["action"].as_str()), (Some(true), Some("mods")));
        assert!(d["status"]["resolution"].is_null() && d["catalog"] == json!([]));
    }
}
