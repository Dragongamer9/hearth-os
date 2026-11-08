//! Playing a game with its mods, the same way from every door: Hearth Mods' Play modded, Game Mode's Mods screens
//! (through the worker), and the Steam shortcuts "Add to Steam" makes (`hearth-mods --play-modded <game>`,
//! `--run-mo2 <instance>`, `--organizer <instance> run`).
//!
//! - A game with a Hearth Mod Organizer instance: the organizer lays the mods over the game while it runs (vfs::run).
//! - Otherwise (Hearth Mods' own list: the mods are already in the game's folder): the game's script extender when
//!   it's there (skse64_loader.exe for Skyrim SE...), else the game itself through Hearth (`nimbus --play-now`, the
//!   library's Play). Asking Steam to start the game (1.0) only ever started its own launcher, never the extender.
//! - Windows programs run with Proton through umu-launcher: the system's umu-run, else Hearth's own copy (hearth-core
//!   tools.rs, downloaded once and checked), in the game's own Steam prefix, with the Proton that prefix was made with,
//!   and Steam's app id so Steam games (Steam DRM) find the running Steam client.
//!
//! Every step goes to Hearth's log (bridge.log, which Send logs includes) and to mods/logs/launch.log with the
//! program's own output; Proton's log of the last modded start is mods/logs/steam-<app>.log (PROTON_LOG). Send logs
//! has both (hearth-core logs.rs).

use crate::games::GameEntry;
use crate::util;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub fn logs_dir() -> PathBuf { util::data_dir().join("mods").join("logs") }
pub fn log_file() -> PathBuf { logs_dir().join("launch.log") }

/// One step: Hearth's log and the launch log.
pub fn log(line: &str) {
    use std::io::Write;
    let _ = std::fs::create_dir_all(logs_dir());
    let f = log_file();
    if std::fs::metadata(&f).map(|m| m.len() > 2_000_000).unwrap_or(false) { let _ = std::fs::rename(&f, logs_dir().join("launch.log.old")); }
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&f) {
        let _ = file.write_all(format!("{} {}\n", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"), line).as_bytes());
    }
    #[cfg(target_os = "linux")]
    hearth_core::bridge::log(&format!("hearth mods: {line}"));
}

/// The last lines of the launch log (what a failed start says).
pub fn log_tail(n: usize) -> String {
    let t = std::fs::read_to_string(log_file()).unwrap_or_default();
    let lines: Vec<&str> = t.lines().collect();
    lines[lines.len().saturating_sub(n)..].join("\n")
}

fn on_path(cmd: &str) -> Option<PathBuf> {
    std::env::var_os("PATH").and_then(|p| std::env::split_paths(&p).map(|d| d.join(cmd)).find(|f| f.is_file()))
}

/// umu-run: the system's, else Hearth's own (downloaded the first time).
pub fn umu() -> Result<String, String> {
    if let Some(p) = on_path("umu-run") { return Ok(p.to_string_lossy().to_string()); }
    own_umu()
}
#[cfg(target_os = "linux")]
fn own_umu() -> Result<String, String> {
    hearth_core::tools::ensure(&hearth_core::tools::UMU).map_err(|e| format!("umu-launcher (which runs Windows programs with Proton) isn't here and couldn't be fetched: {e}"))
}
#[cfg(not(target_os = "linux"))]
fn own_umu() -> Result<String, String> { Err("Windows programs can't be started from here.".into()) }

/// Steam's folder and its libraries.
#[cfg(target_os = "linux")]
fn steam_dirs() -> (Option<PathBuf>, Vec<PathBuf>) {
    let s = hearth_core::steam::steam_path(&util::home());
    let libs = s.as_deref().map(hearth_core::steam::libraries).unwrap_or_default();
    (s, libs)
}
#[cfg(not(target_os = "linux"))]
fn steam_dirs() -> (Option<PathBuf>, Vec<PathBuf>) { (None, Vec::new()) }

/// Proton's version from a prefix's own note ("10.0-100" in compatdata/<app>/version) → "10.0"; a GE-Proton's
/// note is its whole name ("GE-Proton11-3").
pub fn prefix_proton(compat: &Path) -> Option<String> {
    let v = std::fs::read_to_string(compat.join("version")).ok()?;
    let v = v.trim();
    if v.is_empty() { return None; }
    if v.starts_with("experimental") { return Some("Experimental".into()); }
    if v.starts_with("GE-Proton") { return Some(v.split_whitespace().next().unwrap_or(v).to_string()); }
    Some(v.split(['-', ' ']).next().unwrap_or(v).to_string())
}

/// Hearth's note of the Proton it starts one of its own prefixes with (a Wabbajack list's, an organizer instance's).
const PROTON_NOTE: &str = "hearth-proton.txt";

/// A prefix Hearth made (not one of Steam's compatdata folders, whose Proton Steam chooses).
pub fn hearths_own_prefix(compat: &Path) -> bool { !compat.to_string_lossy().replace('\\', "/").contains("/steamapps/compatdata/") }

/// The Proton to use (PROTONPATH), first that applies:
/// 1. $HEARTH_PROTON;
/// 2. for Hearth's own prefixes, the Proton Hearth used last for it (hearth-proton.txt), so it's never switched;
/// 3. the Proton the prefix was made with (compatdata/<app>/version): the same Steam Proton, the same GE-Proton, or
///    Proton - Experimental for a Valve-made prefix with no matching numbered Proton (as before 1.7);
/// 4. the Proton detector (hearth-core proton_pick, the same as Hearth Games and Windows apps): the Proton the game
///    last worked with, Hearth's table, the GE-Proton built into Hearth OS (offline), Valve's newest;
/// 5. the newest Valve Proton in Steam's libraries; else GE-Proton (umu downloads it).
pub fn proton_for(compat: Option<&Path>, libraries: &[PathBuf]) -> String { proton_for_game(compat, libraries, None) }

pub fn proton_for_game(compat: Option<&Path>, libraries: &[PathBuf], app: Option<u64>) -> String {
    if let Some(p) = std::env::var("HEARTH_PROTON").ok().filter(|p| !p.is_empty()) { return p; }
    if let Some(c) = compat.filter(|c| hearths_own_prefix(c)) {
        if let Some(p) = std::fs::read_to_string(c.join(PROTON_NOTE)).ok().map(|s| PathBuf::from(s.trim())).filter(|p| p.is_absolute() && p.join("proton").is_file()) {
            return p.to_string_lossy().to_string();
        }
    }
    let mut found: Vec<(String, PathBuf)> = Vec::new();
    for l in libraries {
        let Ok(rd) = std::fs::read_dir(l.join("steamapps/common")) else { continue };
        for e in rd.flatten() {
            let n = e.file_name().to_string_lossy().to_string();
            if n.starts_with("Proton") && e.path().join("proton").is_file() { found.push((n, e.path())); }
        }
    }
    let experimental = found.iter().find(|(n, _)| n == "Proton - Experimental").map(|(_, p)| p.to_string_lossy().to_string());
    if let Some(w) = compat.and_then(prefix_proton) {
        if w.starts_with("GE-Proton") {
            if let Some(p) = named_proton(&w) { return p.to_string_lossy().to_string(); }
        } else {
            if let Some((_, p)) = found.iter().find(|(n, _)| n.trim_start_matches("Proton").trim_start_matches(" - ").trim() == w) { return p.to_string_lossy().to_string(); }
            if let Some(p) = &experimental { return p.clone(); }
        }
    }
    if let Some(p) = detected_proton(app) { return p.to_string_lossy().to_string(); }
    if let Some(p) = experimental { return p; }
    let ver = |n: &str| -> Vec<u32> { n.trim_start_matches("Proton").trim().split('.').map(|x| x.parse().unwrap_or(0)).collect() };
    found.sort_by_key(|(n, _)| ver(n));
    found.last().map(|(_, p)| p.to_string_lossy().to_string()).unwrap_or_else(|| "GE-Proton".into())
}
#[cfg(target_os = "linux")]
fn named_proton(name: &str) -> Option<PathBuf> { hearth_core::proton_pick::named(name) }
#[cfg(not(target_os = "linux"))]
fn named_proton(_: &str) -> Option<PathBuf> { None }
#[cfg(target_os = "linux")]
fn detected_proton(app: Option<u64>) -> Option<PathBuf> { hearth_core::proton_pick::auto_for("steam", &app.map(|a| a.to_string()).unwrap_or_default()) }
#[cfg(not(target_os = "linux"))]
fn detected_proton(_: Option<u64>) -> Option<PathBuf> { None }

/// Proton's log for a modded start without its heaviest channels (+unwind, +debugstr): those slow a game with many
/// script-extender plugins down and fill the disk over a long session. Your own WINEDEBUG wins.
pub const PROTON_LOG_CHANNELS: &str = "+timestamp,+pid,+tid,+seh,+loaddll,+mscoree,+threadname";

/// The environment for a Windows program under umu: its prefix, Steam's app id (Steam DRM games ask the running Steam
/// client through Proton's bridge), Steam's folder for that bridge, the Proton, and Proton's log for the last start.
/// For Hearth's own prefixes the Proton used is noted in the prefix, so the next start uses the same one.
pub fn umu_env(prefix: &Path, app: Option<u64>, steam: Option<&Path>, libraries: &[PathBuf]) -> Vec<(String, String)> {
    let compat = if prefix.file_name().map(|n| n == "pfx").unwrap_or(false) { prefix.parent().map(Path::to_path_buf).unwrap_or_else(|| prefix.to_path_buf()) } else { prefix.to_path_buf() };
    let proton = proton_for_game(Some(&compat), libraries, app);
    if hearths_own_prefix(&compat) && Path::new(&proton).is_absolute() && std::env::var("HEARTH_PROTON").map(|p| p.is_empty()).unwrap_or(true) {
        let _ = std::fs::create_dir_all(&compat);
        let _ = util::write_atomic(&compat.join(PROTON_NOTE), proton.as_bytes());
    }
    let mut env = vec![
        ("WINEPREFIX".to_string(), prefix.to_string_lossy().to_string()),
        ("STEAM_COMPAT_DATA_PATH".to_string(), compat.to_string_lossy().to_string()),
        ("GAMEID".to_string(), app.map(|a| format!("umu-{a}")).unwrap_or_else(|| "umu-default".into())),
        ("PROTONPATH".to_string(), proton),
        ("PROTON_LOG".to_string(), "1".to_string()),
        ("PROTON_LOG_DIR".to_string(), logs_dir().to_string_lossy().to_string()),
    ];
    if std::env::var_os("WINEDEBUG").is_none() { env.push(("WINEDEBUG".into(), PROTON_LOG_CHANNELS.into())); }
    if let Some(a) = app {
        env.push(("STORE".into(), "steam".into()));
        env.push(("SteamAppId".into(), a.to_string()));
        env.push(("SteamGameId".into(), a.to_string()));
    }
    if let Some(s) = steam { env.push(("STEAM_COMPAT_CLIENT_INSTALL_PATH".into(), s.to_string_lossy().to_string())); }
    env
}

/// Helpers that work in a game's folder without being the game: the overlay's own fuse-overlayfs (its command line
/// names the game's Data folder for as long as the mods are laid over it, so waiting for it never ended and the
/// mods stayed on: "The game is running through the organizer" at every later Play) and Hearth Mods itself.
pub fn not_the_game(comm: &str, cmdline: &str) -> bool {
    let argv0 = cmdline.split('\0').next().unwrap_or("");
    let exe = Path::new(argv0).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let c = comm.trim();
    ["fuse-overlayfs", "fusermount", "fusermount3"].iter().any(|h| c == *h || exe == *h) || c == "hearth-mods" || exe == "hearth-mods"
}

/// Processes (other than this one) working in a folder: their working folder or command line is in it.
pub fn processes_in(dir: &Path) -> Vec<u32> {
    let me = std::process::id();
    let want = dir.to_string_lossy().to_string();
    let Ok(rd) = std::fs::read_dir("/proc") else { return Vec::new() };
    rd.flatten().filter_map(|e| e.file_name().to_string_lossy().parse::<u32>().ok()).filter(|&p| p != me).filter(|p| {
        let cmdline = std::fs::read(format!("/proc/{p}/cmdline")).map(|c| String::from_utf8_lossy(&c).to_string()).unwrap_or_default();
        let comm = std::fs::read_to_string(format!("/proc/{p}/comm")).unwrap_or_default();
        if not_the_game(&comm, &cmdline) { return false; }
        let cwd = std::fs::read_link(format!("/proc/{p}/cwd")).map(|c| c.starts_with(dir)).unwrap_or(false);
        cwd || cmdline.replace('\\', "/").contains(&want)
    }).collect()
}

/// Runs a program (its output into the launch log) and waits for it, then for anything still working in `game_dir`
/// (a script extender's loader ends once the game has started). Returns its exit code.
pub fn run_logged(title: &str, cmd: &str, args: &[String], env: &[(String, String)], cwd: &Path, game_dir: Option<&Path>) -> Result<Option<i32>, String> {
    let shown: Vec<String> = env.iter().filter(|(k, _)| matches!(k.as_str(), "WINEPREFIX" | "GAMEID" | "PROTONPATH" | "SteamAppId")).map(|(k, v)| format!("{k}={v}")).collect();
    log(&format!("{title}: running {} {} {} (in {})", shown.join(" "), cmd, args.iter().map(|a| format!("{a:?}")).collect::<Vec<_>>().join(" "), cwd.display()));
    let _ = std::fs::create_dir_all(logs_dir());
    let out = std::fs::OpenOptions::new().create(true).append(true).open(log_file()).map_err(|e| e.to_string())?;
    let err = out.try_clone().map_err(|e| e.to_string())?;
    let mut c = std::process::Command::new(cmd);
    c.args(args).current_dir(cwd).stdin(std::process::Stdio::null()).stdout(out).stderr(err);
    for (k, v) in env { c.env(k, v); }
    // Steam's runtime libraries (a Steam shortcut starts us with them) break the system's programs; its overlay is kept.
    c.env_remove("LD_LIBRARY_PATH");
    let t0 = std::time::Instant::now();
    let mut child = c.spawn().map_err(|e| { let m = format!("Couldn't start {title} ({e})"); log(&format!("{title}: {m}")); m })?;
    let code = child.wait().ok().and_then(|s| s.code());
    log(&format!("{title}: {} ended with exit code {} after {}s", Path::new(cmd).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(), code.map(|c| c.to_string()).unwrap_or_else(|| "none (killed)".into()), t0.elapsed().as_secs()));
    if let Some(d) = game_dir {
        let t1 = std::time::Instant::now();
        while processes_in(d).is_empty() && t1.elapsed().as_secs() < 20 { std::thread::sleep(std::time::Duration::from_secs(1)); }
        let t2 = std::time::Instant::now();
        while !processes_in(d).is_empty() && t2.elapsed().as_secs() < 24 * 3600 { std::thread::sleep(std::time::Duration::from_secs(2)); }
        log(&format!("{title}: the game's processes are gone ({}s in all)", t0.elapsed().as_secs()));
    }
    Ok(code)
}

/// The Hearth app (for `--play-now`): the session's NIMBUS_APP, /usr/bin/nimbus or nimbus on the PATH.
pub fn hearth_app() -> Option<PathBuf> {
    std::env::var_os("NIMBUS_APP").map(PathBuf::from).filter(|p| p.is_file()).or_else(|| Some(PathBuf::from("/usr/bin/nimbus")).filter(|p| p.is_file())).or_else(|| on_path("nimbus"))
}

/// What Play modded starts for a game on Hearth Mods' own list (pure, for the tests): ("extender", loader) when the
/// game's script extender is in its folder, else ("hearth", none): the game through Hearth's library.
pub fn simple_plan(g: &GameEntry) -> (&'static str, Option<PathBuf>) {
    let kind = g.steam_app.and_then(crate::organizer::kinds::Kind::by_steam);
    if let (Some(k), Some(dir)) = (kind, g.install.as_deref()) {
        if !k.extender.is_empty() {
            if let Some(p) = crate::organizer::vfs::ci_find(dir, k.extender).filter(|p| p.is_file()) { return ("extender", Some(p)); }
        }
    }
    ("hearth", None)
}

/// Plays a game with its mods and returns when it has closed (`hearth-mods --play-modded`).
pub fn play_modded(g: &GameEntry) -> Result<Value, String> {
    log(&format!("play modded: {} ({}) asked", g.title, g.id));
    if g.beacon { return Err("Beacon starts this game with its mods.".into()); }
    let ctx = crate::organizer::Ctx::system(vec![g.clone()]);
    if let Some(inst) = crate::organizer::gamemode::for_game(&ctx, &g.id) {
        log(&format!("play modded: {} has the organizer instance {}: the organizer lays its mods over the game while it runs", g.title, inst.rec.id));
        crate::organizer::refuse_simple_list(&ctx, &inst).map_err(|e| { log(&format!("play modded: {e}")); e })?;
        let ex = inst.executable(None).map_err(|e| { log(&format!("play modded: {e}")); e })?;
        let r = crate::organizer::vfs::run(&inst, &ex);
        log(&format!("play modded: organizer run ended: {}", match &r { Ok(v) => v.to_string(), Err(e) => e.clone() }));
        return r;
    }
    let install = g.install.clone().ok_or("The game's folder isn't known.")?;
    match simple_plan(g) {
        ("extender", Some(loader)) => {
            let prefix = g.prefix.clone().ok_or_else(|| { let m = "The game's Proton folder isn't there yet: start the game once without mods, then Play modded again.".to_string(); log(&format!("play modded: {m}")); m })?;
            let (steam, libs) = steam_dirs();
            #[cfg(target_os = "linux")]
            if g.store == "steam" && !hearth_core::steam::client_running() { log("play modded: Steam isn't running; a Steam game may need it (Steam DRM)"); }
            let umu = umu().map_err(|e| { log(&format!("play modded: {e}")); e })?;
            let env = umu_env(&prefix, g.steam_app, steam.as_deref(), &libs);
            let code = run_logged(&format!("play modded {}", g.title), &umu, &[loader.to_string_lossy().to_string()], &env, &install, Some(&install))?;
            Ok(json!({"ok": true, "how": "extender", "exit": code}))
        }
        _ => {
            let app = hearth_app().ok_or_else(|| { let m = "Hearth's app (nimbus) isn't installed, so the game can't be started from here.".to_string(); log(&format!("play modded: {m}")); m })?;
            let code = run_logged(&format!("play modded {}", g.title), &app.to_string_lossy(), &["--play-now".to_string(), g.id.clone()], &[], &util::home(), None)?;
            if code.unwrap_or(1) != 0 { return Err(format!("Hearth couldn't start {} (exit code {}). Its log says why: Settings › System › Send logs.", g.title, code.map(|c| c.to_string()).unwrap_or_default())); }
            Ok(json!({"ok": true, "how": "hearth", "exit": code}))
        }
    }
}

/// Mod Organizer 2 itself (Windows, under Proton) for an instance that has it (a Wabbajack list, an MO2 folder).
pub fn mo2_exe(inst: &crate::organizer::Instance) -> Option<PathBuf> { crate::organizer::vfs::ci_find(&inst.dir, "ModOrganizer.exe").filter(|p| p.is_file()) }

pub fn run_mo2(inst: &crate::organizer::Instance) -> Result<Value, String> {
    log(&format!("mod organizer 2: {} asked", inst.rec.title));
    let exe = mo2_exe(inst).ok_or_else(|| { let m = format!("{} has no ModOrganizer.exe in its folder.", inst.rec.title); log(&format!("mod organizer 2: {m}")); m })?;
    let (steam, libs) = steam_dirs();
    let umu = umu().map_err(|e| { log(&format!("mod organizer 2: {e}")); e })?;
    let app = inst.steam_app.or(if inst.kind.steam > 0 { Some(inst.kind.steam) } else { None });
    let env = umu_env(&inst.wine_prefix(), app, steam.as_deref(), &libs);
    let code = run_logged(&format!("mod organizer 2 {}", inst.rec.title), &umu, &[exe.to_string_lossy().to_string()], &env, &inst.dir, inst.game_dir.as_deref())?;
    Ok(json!({"ok": true, "exit": code}))
}

/// Starts `hearth-mods <args>` on its own (the window's Play, Game Mode's): its steps go to the logs; when it ends
/// within a few seconds with a problem, that problem is the answer, so a start that fails is never silent.
pub fn spawn_watched(what: &str, args: &[String]) -> Result<Value, String> {
    let me = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("hearth-mods"));
    log(&format!("{what}: starting hearth-mods {}", args.join(" ")));
    let mut c = std::process::Command::new(me);
    c.args(args).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::null());
    c.env_remove("LD_LIBRARY_PATH");
    let mut child = c.spawn().map_err(|e| format!("Couldn't start it ({e})"))?;
    let t0 = std::time::Instant::now();
    while t0.elapsed().as_secs() < 6 {
        if let Ok(Some(st)) = child.try_wait() {
            let mut s = String::new();
            if let Some(mut o) = child.stdout.take() { use std::io::Read; let _ = o.read_to_string(&mut s); }
            let v: Value = serde_json::from_str(s.trim()).ok().or_else(|| s.lines().rev().find_map(|l| serde_json::from_str(l).ok())).unwrap_or(Value::Null);
            let err = v["error"].as_str().map(str::to_string).or_else(|| if !st.success() { Some(format!("It stopped at once (exit code {}).\n{}", st.code().unwrap_or(-1), log_tail(6))) } else { None });
            if let Some(e) = err { log(&format!("{what}: failed: {e}")); return Err(e); }
            return Ok(json!({"started": true, "ended": true}));
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
    std::thread::spawn(move || { let _ = child.wait(); });
    Ok(json!({"started": true}))
}

// ------------------------------------------------------------------ Add to Steam
// Hearth Mods doesn't write Steam's files. It keeps the entries asked for in mods/steam-shortcuts.json, and Hearth Sync
// (hosts/decky-hearth-sync, in Steam's Game Mode) adds, updates and removes them as non-Steam shortcuts with the
// game's art, the same way it does Hearth's own games: one shortcut per entry id, so adding again updates it.

pub fn shortcuts_file() -> PathBuf { util::data_dir().join("mods").join("steam-shortcuts.json") }

/// The three ways in: (kind, title suffix, arguments for hearth-mods).
pub fn entry_for(kind: &str, game_id: &str, inst: Option<&str>) -> Option<(String, Vec<String>)> {
    match (kind, inst) {
        ("modded", _) => Some((" (Modded)".into(), vec!["--play-modded".into(), game_id.into()])),
        ("mo2", Some(i)) => Some((" — Mod Organizer 2".into(), vec!["--run-mo2".into(), i.into()])),
        ("hearth", Some(i)) => Some((" — Hearth Mod Organizer".into(), vec!["--organizer".into(), i.into(), "run".into()])),
        _ => None,
    }
}

/// Adds (or updates) a game's entries; `kinds` from modded, mo2, hearth. Returns the whole list.
pub fn add_to_steam(file: &Path, exe: &Path, g: &GameEntry, inst: Option<&str>, kinds: &[&str]) -> Result<Value, String> {
    let mut list: Vec<Value> = util::read_json(file).as_array().cloned().unwrap_or_default();
    let mut added = Vec::new();
    for k in kinds {
        let Some((suffix, args)) = entry_for(k, &g.id, inst) else { continue };
        let id = format!("hearth-mods:{k}:{}", g.id);
        let mut launch = vec![exe.to_string_lossy().to_string()];
        launch.extend(args);
        let e = json!({"id": id, "title": format!("{}{suffix}", g.title), "game": g.id, "kind": k, "launch": launch, "cwd": util::home(),
                       "cover": g.art.get("cover"), "wide": g.art.get("wide").or(g.art.get("hero")), "hero": g.art.get("hero").or(g.art.get("wide")),
                       "logo": g.art.get("logo"), "icon": g.art.get("icon"), "added": util::now_ms()});
        if let Some(i) = list.iter().position(|x| x["id"] == json!(id)) { list[i] = e; } else { list.push(e); }
        added.push(id);
    }
    if added.is_empty() { return Err("Nothing to add for this game.".into()); }
    util::write_json(file, &Value::Array(list.clone()))?;
    log(&format!("add to steam: {} ({})", g.title, added.join(", ")));
    Ok(json!({"added": added, "entries": list}))
}

/// Takes a game's entries out (Hearth Sync removes their shortcuts).
pub fn remove_from_steam(file: &Path, game_id: &str) -> Result<Value, String> {
    let list: Vec<Value> = util::read_json(file).as_array().cloned().unwrap_or_default();
    let keep: Vec<Value> = list.iter().filter(|x| x["game"] != json!(game_id)).cloned().collect();
    util::write_json(file, &Value::Array(keep.clone()))?;
    Ok(json!({"removed": list.len() - keep.len()}))
}

/// The entries a game has in Steam (kinds).
pub fn in_steam(file: &Path, game_id: &str) -> Vec<String> {
    util::read_json(file).as_array().cloned().unwrap_or_default().iter().filter(|x| x["game"] == json!(game_id)).filter_map(|x| x["kind"].as_str().map(str::to_string)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Runs `f` with HOME and the built-in Proton folder pointed at empty test folders (no Steam, no GE-Proton).
    fn bare<T>(t: &Path, f: impl FnOnce() -> T) -> T {
        let (home, builtin) = (std::env::var_os("HOME"), std::env::var_os("HEARTH_BUILTIN_PROTON_DIR"));
        std::fs::create_dir_all(t.join("home")).unwrap();
        std::fs::create_dir_all(t.join("builtin")).unwrap();
        std::env::set_var("HOME", t.join("home"));
        std::env::set_var("HEARTH_BUILTIN_PROTON_DIR", t.join("builtin"));
        std::env::remove_var("HEARTH_PROTON");
        std::env::remove_var("WINEDEBUG");
        let r = f();
        match home { Some(h) => std::env::set_var("HOME", h), None => std::env::remove_var("HOME") }
        match builtin { Some(b) => std::env::set_var("HEARTH_BUILTIN_PROTON_DIR", b), None => std::env::remove_var("HEARTH_BUILTIN_PROTON_DIR") }
        r
    }

    #[test]
    fn the_proton_the_prefix_was_made_with() {
        let t = tempfile::tempdir().unwrap();
        bare(t.path(), || {
            let lib = t.path().join("lib");
            for n in ["Proton 9.0 (Beta)", "Proton 10.0", "Proton - Experimental"] { std::fs::create_dir_all(lib.join("steamapps/common").join(n)).unwrap(); std::fs::write(lib.join("steamapps/common").join(n).join("proton"), "").unwrap(); }
            let compat = lib.join("steamapps/compatdata/489830");
            std::fs::create_dir_all(&compat).unwrap();
            std::fs::write(compat.join("version"), "10.0-100\n").unwrap();
            assert_eq!(proton_for(Some(&compat), &[lib.clone()]), lib.join("steamapps/common/Proton 10.0").to_string_lossy());
            std::fs::write(compat.join("version"), "8.0-5\n").unwrap();
            assert_eq!(proton_for(Some(&compat), &[lib.clone()]), lib.join("steamapps/common/Proton - Experimental").to_string_lossy(), "no matching one: Experimental");
            assert_eq!(proton_for(None, &[]), "GE-Proton");
            let env = umu_env(&compat.join("pfx"), Some(489830), Some(Path::new("/s")), &[lib.clone()]);
            let get = |k: &str| env.iter().find(|(a, _)| a == k).map(|(_, v)| v.clone()).unwrap_or_default();
            assert_eq!((get("STEAM_COMPAT_DATA_PATH"), get("SteamAppId"), get("GAMEID"), get("STEAM_COMPAT_CLIENT_INSTALL_PATH")), (compat.to_string_lossy().to_string(), "489830".into(), "umu-489830".into(), "/s".into()));
            assert_eq!(get("PROTON_LOG"), "1");
            assert_eq!(get("WINEDEBUG"), PROTON_LOG_CHANNELS);
            assert!(!PROTON_LOG_CHANNELS.contains("unwind"), "the heaviest log channel stays off");
            assert!(!compat.join(PROTON_NOTE).exists(), "Steam's own prefixes get no note of Hearth's");
        });
    }

    #[test]
    fn hearths_own_prefixes_use_the_builtin_ge_proton_and_keep_it() {
        let t = tempfile::tempdir().unwrap();
        bare(t.path(), || {
            let ge = t.path().join("builtin/GE-Proton11-3");
            std::fs::create_dir_all(&ge).unwrap();
            std::fs::write(ge.join("proton"), "").unwrap();
            let lib = t.path().join("lib");
            std::fs::create_dir_all(lib.join("steamapps/common/Proton - Experimental")).unwrap();
            std::fs::write(lib.join("steamapps/common/Proton - Experimental/proton"), "").unwrap();
            // A new Wabbajack list's prefix: the GE-Proton built into Hearth OS, noted for next time.
            let fresh = t.path().join("data/prefixes/wabbajack-w1");
            let env = umu_env(&fresh.join("pfx"), Some(489830), None, &[lib.clone()]);
            assert_eq!(env.iter().find(|(k, _)| k == "PROTONPATH").unwrap().1, ge.to_string_lossy());
            assert_eq!(std::fs::read_to_string(fresh.join(PROTON_NOTE)).unwrap(), ge.to_string_lossy());
            // Experimental is installed later: the noted one stays.
            assert_eq!(proton_for(Some(&fresh), &[lib.clone()]), ge.to_string_lossy());
            // A prefix made before 1.7 by Proton - Experimental keeps Experimental.
            let old = t.path().join("data/prefixes/wabbajack-w0");
            std::fs::create_dir_all(&old).unwrap();
            std::fs::write(old.join("version"), "11.0-100\n").unwrap();
            assert_eq!(proton_for(Some(&old), &[lib.clone()]), lib.join("steamapps/common/Proton - Experimental").to_string_lossy());
            // A prefix GE-Proton made is started with that GE-Proton, by its name.
            let by_ge = t.path().join("data/prefixes/wabbajack-w2");
            std::fs::create_dir_all(&by_ge).unwrap();
            std::fs::write(by_ge.join("version"), "GE-Proton11-3\n").unwrap();
            assert_eq!(prefix_proton(&by_ge).as_deref(), Some("GE-Proton11-3"));
            assert_eq!(proton_for(Some(&by_ge), &[]), ge.to_string_lossy());
            assert!(hearths_own_prefix(&fresh) && !hearths_own_prefix(Path::new("/h/.local/share/Steam/steamapps/compatdata/489830")));
        });
    }

    #[test]
    fn the_overlay_and_hearth_mods_are_not_the_game() {
        let data = "/home/me/Modlists/Tuxborn/Game Root/Data";
        assert!(not_the_game("fuse-overlayfs\n", &format!("fuse-overlayfs\0-o\0lowerdir=/x/.hearth/layer:{data},upperdir=/x/overwrite,workdir=/x/.hearth/work\0{data}\0")));
        assert!(not_the_game("fusermount3", "fusermount3\0-u\0/g\0"));
        assert!(not_the_game("hearth-mods", "/usr/bin/hearth-mods\0--organizer\0tuxborn\0run\0"));
        assert!(!not_the_game("SkyrimSE.exe", "Z:\\home\\me\\Modlists\\Tuxborn\\Game Root\\SkyrimSE.exe\0"));
        assert!(!not_the_game("umu-run", "/usr/bin/python3\0/usr/bin/umu-run\0/g/skse64_loader.exe\0"));
    }

    #[test]
    fn skyrim_plays_through_skse_when_it_is_there() {
        let t = tempfile::tempdir().unwrap();
        let game = t.path().join("Skyrim Special Edition");
        std::fs::create_dir_all(&game).unwrap();
        std::fs::write(game.join("SkyrimSE.exe"), "").unwrap();
        let g = GameEntry { id: "steam:489830".into(), title: "Skyrim".into(), source: "steam".into(), store: "steam".into(), steam_app: Some(489830), install: Some(game.clone()), kind: "game".into(), ..Default::default() };
        assert_eq!(simple_plan(&g).0, "hearth", "no SKSE: the game through Hearth");
        std::fs::write(game.join("skse64_loader.exe"), "").unwrap();
        assert_eq!(simple_plan(&g), ("extender", Some(game.join("skse64_loader.exe"))));
        let other = GameEntry { steam_app: Some(1), ..g.clone() };
        assert_eq!(simple_plan(&other).0, "hearth");
    }

    #[test]
    fn steam_entries_are_added_once_and_updated() {
        let t = tempfile::tempdir().unwrap();
        let f = t.path().join("steam-shortcuts.json");
        let mut g = GameEntry { id: "steam:489830".into(), title: "Skyrim Special Edition".into(), ..Default::default() };
        g.art.insert("cover".into(), "/art/c.jpg".into());
        let exe = Path::new("/usr/bin/hearth-mods");
        let r = add_to_steam(&f, exe, &g, Some("skyrimse"), &["modded", "mo2", "hearth"]).unwrap();
        assert_eq!(r["entries"].as_array().unwrap().len(), 3);
        let e = &r["entries"][0];
        assert_eq!((e["title"].as_str(), e["launch"][1].as_str(), e["cover"].as_str()), (Some("Skyrim Special Edition (Modded)"), Some("--play-modded"), Some("/art/c.jpg")));
        assert_eq!(r["entries"][1]["title"], "Skyrim Special Edition — Mod Organizer 2");
        assert_eq!(r["entries"][2]["launch"], json!(["/usr/bin/hearth-mods", "--organizer", "skyrimse", "run"]));
        g.title = "Skyrim SE".into();
        let r = add_to_steam(&f, exe, &g, Some("skyrimse"), &["modded"]).unwrap();
        assert_eq!(r["entries"].as_array().unwrap().len(), 3, "adding again updates");
        assert_eq!(r["entries"][0]["title"], "Skyrim SE (Modded)");
        assert!(add_to_steam(&f, exe, &g, None, &["mo2", "hearth"]).is_err(), "no instance: no organizer entries");
        assert_eq!(in_steam(&f, "steam:489830").len(), 3);
        assert_eq!(remove_from_steam(&f, "steam:489830").unwrap()["removed"], 3);
        assert!(in_steam(&f, "steam:489830").is_empty());
    }
}
