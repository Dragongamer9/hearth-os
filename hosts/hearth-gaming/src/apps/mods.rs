// Hearth Mods' window (Tauri: WebKitGTK on Linux). The page (ui/mods.html) shows each game's mods in load order
// and installs, turns on and off, reorders and removes them; the work is in the library (mods.rs, games.rs).
// Beacon's games (Battle for Middle-earth...) are Beacon's: they're shown with a button that opens Beacon.
//
//   hearth-mods [--game <id>]                       the window (--fullscreen in Game Mode), opened on one game
//   hearth-mods --list                              the games, as JSON
//   hearth-mods --mods <game>                       one game's mods, as JSON
//   hearth-mods --install <game> <zip|folder> [--name N] [--target game] [--sub Folder] [--off]
//   hearth-mods --enable|--disable|--remove <game> <mod>
//   hearth-mods --move <game> <mod> <-1|1>
//   hearth-mods --deploy <game> <hardlink|copy|overlay>  how the game's mods are put in place
//   hearth-mods --nxm <nxm://...>                   the site's "Mod Manager Download" (the nxm:// handler): queued in
//                                                   the open window, or the window opens on Downloads
//   hearth-mods --nexus-key <key> | --downloads     sign in to Nexus Mods | the download queue, as JSON
//   hearth-mods --remount                           puts the overlays back (at sign-in)
//   hearth-mods --modlists | --wabbajack-engine     the Wabbajack gallery | download the Wabbajack engine
//   hearth-mods --organizer <instance> <command…>   the Hearth Mod Organizer, as JSON (docs/MODS.md); `run` is what
//                                                   the library's Play starts for an organizer instance
//   hearth-mods --serve                             the worker without a window (Game Mode's Mods screens ask it through
//                                                   its socket: service.rs); stops by itself when idle
//
// Accounts (LoversLab, MEGA, Google: accounts.rs) are signed in to from the page only: the browser for LoversLab and
// Google, the on-screen keyboard for MEGA. Files no account fetches go through the guided flow (guided.rs).
// Nexus Mods and Wabbajack: nexus.rs, downloads.rs and wabbajack.rs. nxm:// links that arrive while the window is
// open are dropped in mods/nxm-inbox/ by the handler and picked up here within a second.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use hearth_gaming::downloads;
use hearth_gaming::games::{self, GameEntry};
use hearth_gaming::nexus;
use hearth_gaming::wabbajack;
use hearth_gaming::mods::{self, GameMods, Staged};
use hearth_gaming::organizer;
use hearth_gaming::util;
use hearth_gaming::service;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

static GAMES: Mutex<Vec<GameEntry>> = Mutex::new(Vec::new());
static STAGED: Mutex<Option<HashMap<String, (String, Staged)>>> = Mutex::new(None);
/// One change at a time (two at once could both move the same original).
static WORK: Mutex<()> = Mutex::new(());

fn data_root() -> PathBuf { util::data_dir() }
fn inbox() -> PathBuf { data_root().join("mods").join("nxm-inbox") }
fn pid_file() -> PathBuf { data_root().join("mods").join("hearth-mods.pid") }
/// What happened to nxm links taken from the inbox (shown by the page, then cleared).
static NOTICES: Mutex<Vec<Value>> = Mutex::new(Vec::new());

/// Is a Hearth Mods window open (its pid file names a live hearth-mods)?
fn window_open() -> bool {
    let Ok(t) = std::fs::read_to_string(pid_file()) else { return false };
    let Ok(pid) = t.trim().parse::<u32>() else { return false };
    std::fs::read(format!("/proc/{pid}/cmdline")).map(|c| String::from_utf8_lossy(&c).contains("hearth-mods")).unwrap_or(false)
}

/// Takes nxm links out of the inbox and queues them.
fn drain_inbox() {
    let Ok(rd) = std::fs::read_dir(inbox()) else { return };
    let mut files: Vec<PathBuf> = rd.flatten().map(|e| e.path()).filter(|p| p.extension().map(|x| x == "nxm").unwrap_or(false)).collect();
    files.sort();
    for f in files {
        let link = std::fs::read_to_string(&f).unwrap_or_default();
        let _ = std::fs::remove_file(&f);
        let n = match downloads::add_nxm(link.trim(), downloads::Want::default()) {
            Ok(i) => json!({"ok": true, "text": format!("Downloading {}", i.name)}),
            Err(e) => json!({"ok": false, "text": e}),
        };
        NOTICES.lock().unwrap_or_else(|e| e.into_inner()).push(n);
    }
}

/// The Hearth game a Nexus download is for: the one the Nexus game matches.
fn game_for_domain(domain: &str) -> Option<GameEntry> {
    let list = GAMES.lock().unwrap_or_else(|e| e.into_inner()).clone();
    let key = nexus::account().map(|a| a.key).unwrap_or_default();
    let all = if key.is_empty() { Value::Array(vec![]) } else { nexus::games(&key) };
    list.into_iter().find(|g| {
        let gm = GameMods::open(&data_root(), g);
        if !gm.state.nexus.is_empty() { return gm.state.nexus == domain; }
        nexus::domain_for(g, &all).as_deref() == Some(domain)
    })
}

fn load_games() -> Vec<GameEntry> {
    let list: Vec<GameEntry> = games::list().into_iter().filter(|g| g.kind == "game" && g.install.as_ref().map(|p| p.is_dir()).unwrap_or(false)).collect();
    *GAMES.lock().unwrap_or_else(|e| e.into_inner()) = list.clone();
    list
}
fn game(id: &str) -> Result<GameEntry, String> {
    let found = GAMES.lock().unwrap_or_else(|e| e.into_inner()).iter().find(|g| g.id == id).cloned();
    match found { Some(g) => Ok(g), None => load_games().into_iter().find(|g| g.id == id).ok_or_else(|| "That game isn't installed anymore.".to_string()) }
}

fn games_json(list: &[GameEntry]) -> Value {
    Value::Array(list.iter().map(|g| {
        let gm = GameMods::open(&data_root(), g);
        let mut v = serde_json::to_value(g).unwrap_or(Value::Null);
        v["mods"] = json!(gm.state.mods.len());
        v["modsOn"] = json!(gm.state.mods.iter().filter(|m| m.enabled).count());
        v
    }).collect())
}

fn summary(g: &GameEntry) -> Value {
    let mut s = GameMods::open(&data_root(), g).summary(g);
    s["beacon"] = json!(g.beacon);
    if s["nexus"].as_str().map(|d| d.is_empty()).unwrap_or(true) {
        if let Some(d) = nexus::STEAM_DOMAINS.iter().find(|x| Some(x.0) == g.steam_app) { s["nexus"] = json!(d.1); }
    }
    s
}

fn change(id: &str, f: impl FnOnce(&mut GameMods, &GameEntry) -> Result<mods::Report, String>) -> Result<Value, String> {
    let _w = WORK.lock().unwrap_or_else(|e| e.into_inner());
    let g = game(id)?;
    if g.beacon { return Err("Beacon manages this game's mods. Open Beacon to change them.".into()); }
    let mut gm = GameMods::open(&data_root(), &g);
    let r = f(&mut gm, &g)?;
    Ok(json!({"report": r, "summary": summary(&g)}))
}

// ------------------------------------------------------------------ the calls
// Every call the page makes, and every call Game Mode makes (through the worker's socket: service.rs), by name. The
// window sends its calls through `call`: answered here when this process is the worker, else by the worker.

fn s(a: &Value, k: &str) -> String { a[k].as_str().unwrap_or("").to_string() }
fn b(a: &Value, k: &str) -> bool { a[k].as_bool().unwrap_or(false) }
fn u(a: &Value, k: &str) -> u64 { a[k].as_u64().or_else(|| a[k].as_str().and_then(|x| x.parse().ok())).unwrap_or(0) }

/// A finished download, unpacked and waiting for "Install" into a game (the game it was for, or the one asked).
fn stage_download(id: &str, game_id: &str) -> Result<(GameEntry, Staged), String> {
    let d = downloads::get(id).ok_or("That download isn't in the list anymore.")?;
    if d.state != "done" { return Err("It hasn't finished downloading.".to_string()); }
    let g = if !game_id.is_empty() { game(game_id)? } else if !d.game.is_empty() { game(&d.game)? } else {
        if GAMES.lock().unwrap_or_else(|e| e.into_inner()).is_empty() { load_games(); }
        game_for_domain(&d.domain).ok_or("Choose which game it's for.")?
    };
    if g.beacon { return Err("Beacon manages this game's mods. Open Beacon to add one.".to_string()); }
    let _w = WORK.lock().unwrap_or_else(|e| e.into_inner());
    let mut st = mods::stage(&data_root(), &g, &d.path())?;
    st.name = d.name.clone();
    st.nexus = Some(mods::NexusRef { domain: d.domain.clone(), mod_id: d.mod_id, file_id: d.file_id, version: String::new() });
    Ok((g, st))
}

/// The mods of a game that came from Nexus, with a newer file there (the files list's file_updates).
fn check_updates(id: &str) -> Result<Value, String> {
    let g = game(id)?;
    let acct = nexus::account().ok_or("Sign in to Nexus Mods first (Mods › Accounts).")?;
    let gm = GameMods::open(&data_root(), &g);
    let mut out = Vec::new();
    let mut seen: HashMap<(String, u64), Value> = HashMap::new();
    for m in gm.state.mods.iter() {
        let Some(n) = &m.nexus else { continue };
        let key = (n.domain.clone(), n.mod_id);
        if !seen.contains_key(&key) { seen.insert(key.clone(), nexus::files(&acct.key, &n.domain, n.mod_id).unwrap_or(Value::Null)); }
        if let Some(f) = nexus::newer_file(&seen[&key], n.file_id) {
            out.push(json!({"mod": m.id, "name": m.name, "domain": n.domain, "modId": n.mod_id, "fileId": n.file_id, "newer": f, "page": nexus::files_page(&n.domain, n.mod_id, f["fileId"].as_u64())}));
        }
    }
    Ok(json!({"checked": gm.state.mods.iter().filter(|m| m.nexus.is_some()).count(), "updates": out, "premium": acct.premium}))
}

fn nexus_files(url: &str) -> Result<Value, String> {
    let acct = nexus::account().ok_or("Sign in to Nexus Mods first.")?;
    let rest = url.trim().split("nexusmods.com/").nth(1).unwrap_or(url.trim());
    let parts: Vec<&str> = rest.split(['?', '#']).next().unwrap_or("").split('/').collect();
    if parts.len() < 3 || parts[1] != "mods" { return Err("Paste a mod's address from Nexus Mods (nexusmods.com/<game>/mods/<number>).".to_string()); }
    let (domain, mod_id) = (parts[0].to_ascii_lowercase(), parts[2].parse::<u64>().map_err(|_| "That address has no mod number.".to_string())?);
    let info = nexus::mod_info(&acct.key, &domain, mod_id)?;
    let files = nexus::files(&acct.key, &domain, mod_id)?;
    let list: Vec<Value> = files["files"].as_array().cloned().unwrap_or_default().into_iter().rev()
        .filter(|f| f["category_name"].as_str().map(|c| c != "ARCHIVED" && c != "DELETED").unwrap_or(true))
        .map(|f| json!({"fileId": f["file_id"], "name": f["name"], "version": f["version"], "category": f["category_name"], "size": f["size_in_bytes"].as_u64().or_else(|| f["size_kb"].as_u64().map(|k| k * 1024)), "fileName": f["file_name"], "primary": f["is_primary"], "description": f["description"]}))
        .collect();
    Ok(json!({"domain": domain, "modId": mod_id, "name": info["name"], "summary": info["summary"], "description": info["description"], "author": info["author"], "version": info["version"], "picture": info["picture_url"],
              "endorsements": info["endorsement_count"], "downloads": info["mod_downloads"],
              "page": nexus::files_page(&domain, mod_id, None), "premium": acct.premium, "files": list, "adult": info["contains_adult_content"]}))
}

fn wj_defaults(title: &str, gname: &str) -> Value {
    let info = wabbajack::game_info(gname);
    let list = if GAMES.lock().unwrap_or_else(|e| e.into_inner()).is_empty() { load_games() } else { GAMES.lock().unwrap_or_else(|e| e.into_inner()).clone() };
    let found = info.and_then(|i| list.iter().find(|g| g.steam_app == Some(i.1) || (i.1 == 22380 && g.steam_app == Some(22490))).cloned());
    let base = util::home().join("Modlists");
    let dir = base.join(util::safe_name(title));
    json!({"gameDir": found.as_ref().and_then(|g| g.install.clone()), "gameTitle": found.as_ref().map(|g| g.title.clone()), "known": info.is_some(),
           "installDir": dir, "downloadsDir": base.join("Downloads").join(util::safe_name(title)), "free": wabbajack::free_space(&dir)})
}

/// Opens a web page (Hearth Browser, the default browser on Hearth OS). Only https.
fn open_url(url: &str) -> Result<Value, String> {
    if !url.starts_with("https://") { return Err("Only https pages open from here.".into()); }
    std::process::Command::new("xdg-open").arg(url).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null())
        .spawn().map(|mut c| { std::thread::spawn(move || { let _ = c.wait(); }); Value::Null }).map_err(|e| format!("Couldn't open the page ({e})"))
}

/// Everything at once for a quick look (Game Mode's notifications and Quick Access): the queue and the installs.
fn status() -> Value {
    let mut d = downloads::list();
    d["notices"] = json!(std::mem::take(&mut *NOTICES.lock().unwrap_or_else(|e| e.into_inner())));
    json!({"downloads": d, "jobs": wabbajack::jobs()})
}

/// Something going on that a worker without a window must stay for: downloads, installs.
fn busy() -> bool {
    downloads::list()["active"].as_u64().unwrap_or(0) > 0 || wabbajack::jobs().as_array().map(|a| a.iter().any(|j| j["running"] == true)).unwrap_or(false)
}

fn dispatch(name: &str, a: &Value) -> Result<Value, String> {
    let null = |_: ()| Value::Null;
    // A game with a Hearth Mod Organizer instance: its mod list is the organizer's (Game Mode's Installed tab too).
    if let Some(r) = organizer::gamemode::simple_call(&organizer::Ctx::system(Vec::new()), name, a) { let _w = WORK.lock().unwrap_or_else(|e| e.into_inner()); return r; }
    match name {
        "list" => Ok(games_json(&load_games())),
        "mods_of" => game(&s(a, "id")).map(|g| summary(&g)),
        "stage" => {
            let (id, path) = (s(a, "id"), s(a, "path"));
            let g = game(&id)?;
            if g.beacon { return Err("Beacon manages this game's mods. Open Beacon to add one.".to_string()); }
            let _w = WORK.lock().unwrap_or_else(|e| e.into_inner());
            let st = mods::stage(&data_root(), &g, Path::new(&path))?;
            let v = serde_json::to_value(&st).unwrap_or(Value::Null);
            STAGED.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(HashMap::new).insert(st.token.clone(), (id, st));
            Ok(v)
        }
        "install" => {
            let (id, token) = (s(a, "id"), s(a, "token"));
            let staged = STAGED.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(HashMap::new).remove(&token);
            let Some((gid, st)) = staged.filter(|(g, _)| g == &id) else { return Err("Choose the mod again (it was put away).".to_string()) };
            change(&gid, |gm, g| gm.install(g, &st, &s(a, "name"), &s(a, "target"), &s(a, "sub"), b(a, "enable")))
        }
        "cancel_stage" => {
            if let Some((_, st)) = STAGED.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(HashMap::new).remove(&s(a, "token")) { let _ = std::fs::remove_dir_all(&st.dir); }
            Ok(Value::Null)
        }
        "toggle" => change(&s(a, "id"), |gm, g| gm.set_enabled(g, &s(a, "m"), b(a, "on"))),
        "move_mod" => change(&s(a, "id"), |gm, g| gm.move_by(g, &s(a, "m"), a["by"].as_i64().unwrap_or(0))),
        "remove_mod" => change(&s(a, "id"), |gm, g| gm.remove(g, &s(a, "m"))),
        "set_deploy" => change(&s(a, "id"), |gm, g| gm.set_mode(g, &s(a, "mode"))),
        "set_nexus_game" => { let g = game(&s(a, "id"))?; GameMods::open(&data_root(), &g).set_nexus_game(&s(a, "domain"))?; Ok(summary(&g)) }
        "check_updates" => check_updates(&s(a, "id")),
        // Nexus Mods
        "nexus_account" => Ok(nexus::account().map(|x| x.public()).unwrap_or(Value::Null)),
        "nexus_sign_in" => nexus::sign_in(&s(a, "key")).map(|x| x.public()),
        "nexus_refresh" => nexus::refresh().map(|x| x.public()),
        "nexus_sign_out" => { nexus::sign_out(); Ok(Value::Null) }
        "nexus_pages" => Ok(json!({"keys": nexus::KEY_PAGE, "site": nexus::SITE})),
        "nexus_files" => nexus_files(&s(a, "url")),
        "nexus_browse" => { let k = nexus::account().ok_or("Sign in to Nexus Mods first (Mods › Accounts).")?.key; nexus::browse(&k, &s(a, "domain"), &s(a, "kind")) }
        "nexus_search" => { let k = nexus::account().ok_or("Sign in to Nexus Mods first (Mods › Accounts).")?.key; nexus::search(&k, &s(a, "domain"), &s(a, "q")) }
        // Accounts besides Nexus (accounts.rs): never a token or a password back.
        "accounts_list" => Ok(hearth_gaming::accounts::all()),
        "account_begin" => hearth_gaming::accounts::oauth_begin(&s(a, "id")),
        "account_sign_out" => { hearth_gaming::accounts::sign_out(&s(a, "id")); Ok(Value::Null) }
        "mega_sign_in" => hearth_gaming::accounts::mega_sign_in(&s(a, "email"), &s(a, "password")),
        // Downloads
        "downloads_list" => { let mut v = downloads::list(); v["notices"] = json!(std::mem::take(&mut *NOTICES.lock().unwrap_or_else(|e| e.into_inner()))); Ok(v) }
        "status" => Ok(status()),
        "download_nxm" => downloads::add_nxm(&s(a, "link"), downloads::Want::default()).map(|i| i.public()),
        "download_file" => downloads::add_file(&s(a, "domain"), u(a, "modId"), u(a, "fileId"), downloads::Want { game: s(a, "game"), ..Default::default() }).map(|i| i.public()),
        "download_pause" => { downloads::pause(&s(a, "id")); Ok(Value::Null) }
        "download_resume" => downloads::resume(&s(a, "id")).map(null),
        "download_remove" => { downloads::remove(&s(a, "id"), b(a, "deleteFile")); Ok(Value::Null) }
        "downloads_clear" => { downloads::clear_finished(); Ok(Value::Null) }
        "stage_download" => {
            let (g, st) = stage_download(&s(a, "id"), &s(a, "gameId"))?;
            let mut v = serde_json::to_value(&st).unwrap_or(Value::Null);
            v["game"] = json!(g.id);
            STAGED.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(HashMap::new).insert(st.token.clone(), (g.id.clone(), st));
            Ok(v)
        }
        // Game Mode: a finished download straight into its game, where Hearth suggests, turned on.
        "install_download" => {
            // Into the game's organizer instance when it has one (a FOMOD answers with its steps for Game Mode).
            if let Some(d) = downloads::get(&s(a, "id")).filter(|d| d.state == "done") {
                let gid = if s(a, "gameId").is_empty() { d.game.clone() } else { s(a, "gameId") };
                if let Some(r) = organizer::gamemode::install_file(&organizer::Ctx::system(Vec::new()), &gid, &d.path(), &d.name) { return r; }
            }
            let (g, st) = stage_download(&s(a, "id"), &s(a, "gameId"))?;
            let (t, sub, name) = (st.suggest_target.clone(), st.suggest_sub.clone(), st.name.clone());
            let r = change(&g.id, |gm, gg| gm.install(gg, &st, &name, &t, &sub, true));
            let _ = std::fs::remove_dir_all(&st.dir);
            r
        }
        // Wabbajack
        "wj_engine" => Ok(wabbajack::engine_info()),
        "wj_fetch_engine" => wabbajack::fetch_engine(),
        "wj_gallery" => wabbajack::gallery(b(a, "refresh")),
        "wj_jobs" => Ok(wabbajack::jobs()),
        "wj_defaults" => Ok(wj_defaults(&s(a, "title"), &s(a, "game"))),
        "wj_start" => wabbajack::start(&s(a, "title"), &s(a, "list"), &s(a, "game"), Path::new(&s(a, "gameDir")), Path::new(&s(a, "installDir")), Path::new(&s(a, "downloadsDir"))).map(|j| j.public()),
        "wj_resume" => wabbajack::resume(&s(a, "id")).map(|j| j.public()),
        "wj_cancel" => { wabbajack::cancel(&s(a, "id")); Ok(Value::Null) }
        "wj_forget" => wabbajack::forget(&s(a, "id"), b(a, "deleteFiles"), b(a, "deleteDownloads")).map(null),
        "wj_repair" => wabbajack::repair(&s(a, "id")).map(|n| json!(n)),
        "wj_log" => {
            let j = wabbajack::job(&s(a, "id")).ok_or("That install isn't in the list anymore.")?;
            let t = std::fs::read_to_string(j.log()).unwrap_or_default();
            let lines: Vec<&str> = t.lines().collect();
            Ok(json!(lines[lines.len().saturating_sub(400)..].join("\n")))
        }
        "wj_guide_skip" => Ok(wabbajack::guide_skip(&s(a, "id"), &s(a, "file")).unwrap_or(Value::Null)),
        "wj_manual_file" => wabbajack::manual_file(&s(a, "id"), &s(a, "file"), Path::new(&s(a, "path"))),
        "open_page" => hearth_gaming::guided::open_in_browser(&s(a, "url")).map(null),
        "free_space" => Ok(json!(wabbajack::free_space(Path::new(&s(a, "path"))))),
        "open_url" => open_url(&s(a, "url")),
        "open_folder" => util::open_folder(Path::new(&s(a, "path"))).map(null),
        "open_beacon" => games::open_beacon().map(null),
        "places" => Ok(hearth_gaming::browse::places()),
        "browse" => hearth_gaming::browse::list(Path::new(&s(a, "path")), b(a, "files"), b(a, "archives"), false),
        "target_root" => { let g = game(&s(a, "id"))?; mods::target_root(&g, &s(a, "target")).map(|p| json!(p)).ok_or_else(|| "This game has no folder for that.".to_string()) }
        // The Hearth Mod Organizer: the window's Organizer tab (the same commands as `hearth-mods --organizer …`) and
        // Game Mode's calls (organizer/gamemode.rs).
        "organizer" => {
            let args: Vec<String> = a["args"].as_array().map(|x| x.iter().filter_map(|v| v.as_str().map(str::to_string)).collect()).unwrap_or_default();
            if args.iter().any(|x| x == "run") { return Err("Start it with organizer_run.".to_string()); }
            let _w = WORK.lock().unwrap_or_else(|e| e.into_inner());
            let games = if GAMES.lock().unwrap_or_else(|e| e.into_inner()).is_empty() { load_games() } else { GAMES.lock().unwrap_or_else(|e| e.into_inner()).clone() };
            organizer::command(&organizer::Ctx::system(games), &args)
        }
        // Plays through the organizer in its own process (it waits for the game, then takes the mods away again).
        "organizer_run" => {
            let me = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("hearth-mods"));
            let mut c = std::process::Command::new(me);
            c.arg("--organizer").arg(s(a, "id")).arg("run");
            if !s(a, "exe").is_empty() { c.arg("--exe").arg(s(a, "exe")); }
            c.stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null())
                .spawn().map(|mut ch| { std::thread::spawn(move || { let _ = ch.wait(); }); Value::Null }).map_err(|e| format!("Couldn't start it ({e})"))
        }
        "organizer_game" | "organizer_set_profile" | "organizer_plugin_toggle" | "organizer_plugin_move" | "organizer_fomod_finish" => {
            let _w = WORK.lock().unwrap_or_else(|e| e.into_inner());
            organizer::gamemode::call(&organizer::Ctx::system(Vec::new()), name, a)
        }
        "ping" => Ok(json!({"version": env!("CARGO_PKG_VERSION"), "busy": busy()})),
        _ => Err(format!("Hearth Mods doesn't know \"{name}\".")),
    }
}

/// This process answers the socket (the worker), or None when another one does.
static WORKER: Mutex<bool> = Mutex::new(false);
static LAST_CALL: std::sync::OnceLock<Arc<AtomicU64>> = std::sync::OnceLock::new();
fn last_call() -> Arc<AtomicU64> { LAST_CALL.get_or_init(|| Arc::new(AtomicU64::new(util::now_ms()))).clone() }

/// Becomes the worker if no other answers: the socket, the download queue and the nxm inbox. True when this is it.
fn become_worker() -> bool {
    let mut w = WORKER.lock().unwrap_or_else(|e| e.into_inner());
    if *w { return true; }
    let Some(l) = service::bind() else { return false };
    service::serve(l, dispatch, last_call());
    *w = true;
    std::thread::spawn(|| { downloads::kick(); loop { drain_inbox(); std::thread::sleep(std::time::Duration::from_millis(800)); } });
    true
}

/// A call from the window: here when this is the worker (or becomes it), else the worker's answer.
fn route(name: &str, a: &Value) -> Result<Value, String> {
    if *WORKER.lock().unwrap_or_else(|e| e.into_inner()) { return dispatch(name, a); }
    match service::request(name, a) {
        Err(e) if e == "Hearth Mods isn't running." => { become_worker(); dispatch(name, a) }
        r => r,
    }
}

#[tauri::command]
fn info() -> Value {
    let args: Vec<String> = std::env::args().collect();
    json!({"version": env!("CARGO_PKG_VERSION"), "game": util::arg_value(&args, "--game"), "page": util::arg_value(&args, "--page"), "targets": mods::TARGETS.iter().map(|t| json!({"id": t.0, "label": t.1, "prefix": t.2})).collect::<Vec<_>>()})
}
#[tauri::command]
async fn call(name: String, args: Value) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || route(&name, &args)).await.map_err(|e| e.to_string())?
}

/// The standard Linux file chooser (Hearth Files through the portal in Hearth OS): a file (with `filters`, extensions
/// without dots) or a folder. None: closed without a choice. Runs off the window's thread, so the window stays live.
#[tauri::command(async)]
fn pick_path(app: tauri::AppHandle, folder: bool, title: String, start: Option<String>, filters: Option<Vec<String>>) -> Option<String> {
    use tauri_plugin_dialog::DialogExt;
    let mut d = app.dialog().file().set_title(if title.is_empty() { if folder { "Choose a folder" } else { "Choose a file" } } else { title.as_str() });
    if let Some(dir) = start.map(PathBuf::from).map(|p| if p.is_dir() { p } else { p.parent().map(Path::to_path_buf).unwrap_or(p) }).filter(|p| p.is_dir()) { d = d.set_directory(dir); }
    if let Some(f) = filters.filter(|f| !f.is_empty() && !folder) {
        let exts: Vec<&str> = f.iter().map(|s| s.as_str()).collect();
        d = d.add_filter("Mods", &exts);
    }
    let picked = if folder { d.blocking_pick_folder() } else { d.blocking_pick_file() };
    picked.and_then(|f| f.into_path().ok()).map(|p| p.to_string_lossy().into_owned())
}

// ------------------------------------------------------------------ without a window

fn cli(args: &[String]) -> Option<i32> {
    let print = |v: &Value| println!("{}", serde_json::to_string_pretty(v).unwrap_or_default());
    let pos = |flag: &str, n: usize| -> Option<Vec<String>> { let i = args.iter().position(|a| a == flag)?; let v: Vec<String> = args.iter().skip(i + 1).take(n).cloned().collect(); if v.len() == n { Some(v) } else { None } };
    let done = |r: Result<Value, String>| -> i32 { match r { Ok(v) => { print(&v); if v["report"]["problems"].as_array().map(|a| a.is_empty()).unwrap_or(true) { 0 } else { 2 } } Err(e) => { eprintln!("{e}"); 1 } } };
    if util::has_arg(args, "--list") { print(&games_json(&load_games())); return Some(0); }
    if let Some(v) = pos("--mods", 1) { load_games(); return Some(match game(&v[0]) { Ok(g) => { print(&summary(&g)); 0 } Err(e) => { eprintln!("{e}"); 1 } }); }
    if let Some(v) = pos("--install", 2) {
        load_games();
        let target = util::arg_value(args, "--target").unwrap_or_else(|| "game".into());
        let name = util::arg_value(args, "--name").unwrap_or_default();
        let r = (|| -> Result<Value, String> {
            let g = game(&v[0])?;
            let s = mods::stage(&data_root(), &g, Path::new(&v[1]))?;
            let sub = util::arg_value(args, "--sub").unwrap_or_else(|| s.suggest_sub.clone());
            change(&v[0], |gm, g| gm.install(g, &s, &name, &target, &sub, !util::has_arg(args, "--off")))
        })();
        return Some(done(r));
    }
    for (flag, what) in [("--enable", 1), ("--disable", 0), ("--remove", 2)] {
        if let Some(v) = pos(flag, 2) {
            load_games();
            return Some(done(change(&v[0], |gm, g| match what { 1 => gm.set_enabled(g, &v[1], true), 0 => gm.set_enabled(g, &v[1], false), _ => gm.remove(g, &v[1]) })));
        }
    }
    if let Some(v) = pos("--nxm", 1) {
        // The nxm:// handler: into the inbox; the open window takes it, or a window opens on Downloads.
        let _ = std::fs::create_dir_all(inbox());
        let f = inbox().join(format!("{}-{}.nxm", util::now_ms(), std::process::id()));
        if let Err(e) = std::fs::write(&f, &v[0]) { eprintln!("{e}"); return Some(1); }
        if !window_open() && !service::alive() {
            let me = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("hearth-mods"));
            let _ = std::process::Command::new(me).arg("--page").arg("downloads").stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn();
        }
        return Some(0);
    }
    if let Some(v) = pos("--nexus-key", 1) { return Some(match nexus::sign_in(&v[0]) { Ok(a) => { print(&a.public()); 0 } Err(e) => { eprintln!("{e}"); 1 } }); }
    if util::has_arg(args, "--downloads") { print(&downloads::list()); return Some(0); }
    if util::has_arg(args, "--modlists") { return Some(match wabbajack::gallery(true) { Ok(v) => { print(&v); 0 } Err(e) => { eprintln!("{e}"); 1 } }); }
    if util::has_arg(args, "--wabbajack-engine") { return Some(match wabbajack::fetch_engine() { Ok(v) => { print(&v); 0 } Err(e) => { eprintln!("{e}"); 1 } }); }
    if let Some(i) = args.iter().position(|a| a == "--organizer") {
        let rest: Vec<String> = args[i + 1..].to_vec();
        let ctx = organizer::Ctx::system(load_games());
        return Some(match organizer::command(&ctx, &rest) {
            Ok(v) => { print(&v); if v["ok"] == false { 2 } else { 0 } }
            Err(e) => { print(&json!({"ok": false, "error": e})); 1 }
        });
    }
    if util::has_arg(args, "--remount") {
        let mut code = 0;
        let games = load_games();
        // Organizer views left by a crash come away first (the game folders back as they were).
        for (id, r) in organizer::recover_all(&organizer::Ctx::system(games.clone())) { if let Err(e) = r { eprintln!("{id}: {e}"); code = 2; } }
        for g in games { if let Err(e) = GameMods::open(&data_root(), &g).remount(&g) { eprintln!("{}: {e}", g.title); code = 2; } }
        return Some(code);
    }
    if let Some(v) = pos("--deploy", 2) { load_games(); return Some(done(change(&v[0], |gm, g| gm.set_mode(g, &v[1])))); }
    if let Some(v) = pos("--move", 3) {
        load_games();
        let by: i64 = v[2].parse().unwrap_or(0);
        return Some(done(change(&v[0], |gm, g| gm.move_by(g, &v[1], by))));
    }
    None
}

/// `hearth-mods --serve`: the worker without a window, for Game Mode. Stops after service::IDLE with no calls and
/// nothing downloading or installing. 0: stopped; 3: another worker was already running.
fn serve_headless() -> i32 {
    let _ = std::fs::create_dir_all(data_root().join("mods"));
    if !become_worker() { return 3; }
    let last = last_call();
    loop {
        std::thread::sleep(std::time::Duration::from_secs(20));
        let quiet = util::now_ms().saturating_sub(last.load(Ordering::SeqCst)) > service::IDLE.as_millis() as u64;
        if quiet && !busy() { let _ = std::fs::remove_file(service::socket_path()); return 0; }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if util::has_arg(&args, "--serve") { std::process::exit(serve_headless()); }
    if let Some(code) = cli(&args) { std::process::exit(code); }
    let fullscreen = util::has_arg(&args, "--fullscreen");
    // This window is the worker (the queue, the installs, nxm links) unless one is already running (Game Mode's).
    let _ = std::fs::create_dir_all(data_root().join("mods"));
    let _ = std::fs::write(pid_file(), std::process::id().to_string());
    become_worker();
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() { std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1"); }
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![info, call, pick_path])
        .setup(move |app| {
            tauri::WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::App("mods.html".into()))
                .title("Mods")
                .inner_size(1280.0, 800.0)
                .min_inner_size(640.0, 480.0)
                .fullscreen(fullscreen)
                .background_color(tauri::window::Color(15, 11, 9, 255))
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Mods couldn't start");
}
