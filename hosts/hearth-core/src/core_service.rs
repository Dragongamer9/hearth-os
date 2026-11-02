//! Hearth's core without a window, in Steam Game Mode (Experimental, docs/STEAM-GAME-MODE.md "Hearth's core in Steam
//! Game Mode"): `nimbus --core-service [--while <file>]`, started by nimbus-session's run_steam_mode next to Steam and
//! stopped with it. It gives the Hearth Decky plugin live answers (Hearth inside Steam's main menu, Quick Access and the
//! in-game overlay) instead of starting `nimbus --steam-mode-data` for every question, and it keeps Hearth Input told
//! which game is in front, so a game's Hearth controller layout works in Steam Game Mode too.
//!
//! What it never does (Steam runs this Game Mode, not Hearth): no window, no gamescope focus control, no padmouse, no
//! Steam keep-ready or closing Steam, no InputPlumber or Handheld Daemon takeover, no controllers of its own. The only
//! thing that can hold a controller is Hearth Input (desktop/shell/hearth-input), and only while a game with a Hearth
//! Input layout is in front: the plugin says which game Steam runs (`front`, every 2 s while one runs), gamescope's
//! GAMESCOPE_FOCUSED_APP (read only) says whether it's in front, and a `front` older than FRONT_TTL counts as no game,
//! so the mapper lets go by itself when the plugin goes away.
//!
//! The socket: <runtime dir>/hearth/core.sock (the folder overlay_link.rs and input_map.rs use: 0700, this user's), the
//! socket 0600, every connection's other end checked to be this user (SO_PEERCRED). Nothing on the network. One JSON
//! object per line each way: {"id", "cmd", ...} answered by {"id", "ok": true, "data"} or {"id", "ok": false, "error"}.
//! Only the commands in COMMANDS, each with its arguments checked (check): the same reads as `--steam-mode-data`, and
//! the few changes Hearth's own Quick Access makes (power mode, a game's Hearth Input layout, its Proton version, a
//! screenshot, recording).
//!
//!   ping                                  which Hearth this is
//!   data {what, arg}                      what `--steam-mode-data` answers (steam_takeover.rs)
//!   front {game|null, appid, overlay}     the game Steam is running (Hearth's id), its Steam app id, Quick Access open
//!   game {game}                           the in-game section: the game, its Hearth Input layout and choices, its Proton
//!   input-set {game, layout}              a game's Hearth Input layout: off, a template, custom or global
//!   set-proton {game, proton}             the Proton version Hearth starts a game with (one of game_options::proton_choices)
//!   mods {game}                           the game's mods in Hearth Mods (only when Hearth Mods' worker runs)
//!   screenshot                            gamescope's screenshot, to Pictures/Screenshots/Hearth
//!   record                                Hearth Recorder: start or stop recording
//!   call {method, args}                   one of Hearth's own calls the Steam pages make (BRIDGE_CALLS: Store installs
//!                                         and downloads, Settings › Storage, Recomps), answered by a Bridge this
//!                                         service keeps (made the first time it's asked). Steam games are never
//!                                         installed, moved or uninstalled from here: that needs Steam closed, and
//!                                         Steam runs this Game Mode (Steam's own pages do it)
//!   mods-call {call, args}                Hearth Mods (MODS_CALLS: a game's mods, on/off, uninstall, install a finished
//!                                         download), its worker started when none runs
//!   games-disk {mount}                    "Use this disk for games": Hearth's install folder (Epic, GOG) on that drive
//!   coop {op, pads, pad}                  Couch Co-op in Steam Game Mode (hearth-coop, multiplay.rs): `players` (the
//!                                         controllers in the saved player order), `order` (save it), `identify` (a
//!                                         short rumble on one), `games` (what plays split-screen), `check`
//!   seats {op, seat, pad}                 Multi-seat beside Steam Game Mode (hearth-seats): `status`, `pads`, `start` /
//!                                         `stop` one extra player (seat 2-4), `assign` a controller (its pad id) to a
//!                                         player (1-4). Never sets multi-seat up or removes it, never touches accounts
//!   aside {op, appid, game, title}        Quick Resume in Steam Game Mode: `put` the game Steam runs aside (its processes
//!                                         stopped, kept in memory), `resume` it, `close` it, `list`. Only processes that
//!                                         carry the game's number (procs::owners); never Steam's own. A game put aside
//!                                         carries on by itself when it's back in front (Steam's Resume), and everything
//!                                         still stopped carries on when the service ends
//!   friends                               Hearth Friends (Matrix), read only: signed in, each friend's name, presence
//!                                         and game. No chats, no account, no tokens
//!   link {op, peer, server}               Hearth Link and Hearth Storage: `status` (short: names, online, copies,
//!                                         servers; never the device's link), `sync` with a paired device, `wake` a
//!                                         storage server
//!   remote {op, pin, name}                Sunshine (Settings › Remote play): `status`, `on`, `off`, `pair` with
//!                                         Moonlight's 4-digit PIN (never logged)

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Every command the service answers; anything else is refused.
pub const COMMANDS: &[&str] = &["ping", "data", "front", "game", "input-set", "set-proton", "mods", "screenshot", "record", "call", "mods-call", "games-disk", "coop", "seats", "aside", "friends", "link", "remote"];
/// Hearth's own calls the Steam pages may make (`call`), each with the arguments it may carry (everything else is dropped).
pub const BRIDGE_CALLS: &[(&str, &[&str])] = &[
    ("library.downloads", &[]), ("game.install", &["id"]), ("game.cancelInstall", &["id"]), ("game.pauseInstall", &["id"]), ("game.resumeInstall", &["id"]),
    ("game.uninstall", &["id"]),
    ("diskspace.status", &["scan"]), ("diskspace.move", &["id", "to"]), ("diskspace.cancel", &[]), ("diskspace.cleanList", &[]), ("diskspace.clean", &["keys", "confirmed"]),
    ("recomps.list", &[]), ("recomps.install", &["id"]), ("recomps.cancel", &["id"]), ("recomps.uninstall", &["id", "keep"]),
];
/// Hearth Mods' calls the Steam pages may make (`mods-call`): a subset of hearth_mods::CALLS.
pub const MODS_CALLS: &[(&str, &[&str])] = &[
    ("list", &[]), ("mods_of", &["id"]), ("toggle", &["id", "m", "on"]), ("remove_mod", &["id", "m"]), ("downloads_list", &[]), ("install_download", &["id", "gameId"]),
];
/// Couch Co-op's questions the Quick Access › Players tab and the split-screen page may ask (`coop`).
pub const COOP_OPS: &[&str] = &["players", "order", "identify", "games", "check"];
/// A controller as hearth-coop names it: its event node (`event12`), nothing else.
pub fn pad_id_ok(s: &str) -> bool { s.len() > 5 && s.len() <= 10 && s.starts_with("event") && s[5..].chars().all(|c| c.is_ascii_digit()) }
/// Quick Resume in Steam Game Mode (`aside`).
pub const ASIDE_OPS: &[&str] = &["list", "put", "resume", "close"];
/// Hearth Link's questions and actions the Hearth Link page may make (`link`).
pub const LINK_OPS: &[&str] = &["status", "sync", "wake"];
/// Sunshine's (`remote`).
pub const REMOTE_OPS: &[&str] = &["status", "on", "off", "pair"];
/// A Hearth Link device or storage server id as hearth-link names them (letters, digits, `.`, `_`, `:`, `-`).
pub fn link_id_ok(s: &str) -> bool { !s.is_empty() && s.len() <= 128 && s.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | ':' | '-')) }
/// Multi-seat's questions and changes the Quick Access › Hearth Players › Seats section may make (`seats`).
pub const SEATS_OPS: &[&str] = &["status", "pads", "start", "stop", "assign"];
/// A controller as hearth-seats names it (hearth-admin's SEAT_PAD_RE): `usb:1-2` (a USB port path, `usb:3-1.4.2`) or
/// `uniq:<serial or Bluetooth address>`, nothing else.
pub fn seat_pad_ok(s: &str) -> bool {
    let digits = |p: &str| (1..=3).contains(&p.len()) && p.chars().all(|c| c.is_ascii_digit());
    if let Some(port) = s.strip_prefix("usb:") {
        let Some((bus, path)) = port.split_once('-') else { return false };
        let parts: Vec<&str> = path.split('.').collect();
        return digits(bus) && parts.len() <= 7 && parts.iter().all(|p| digits(p));
    }
    if let Some(u) = s.strip_prefix("uniq:") {
        return (4..=64).contains(&u.len()) && u.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, ':' | '.' | '_' | '-'));
    }
    false
}
/// What `data` may ask (the same list as the plugin's DATA, main.py).
pub const DATA: &[&str] = &["home", "library", "store", "downloads", "storage", "info", "power", "power-mode"];
pub const POWER_MODES: &[&str] = &["efficiency", "balanced", "performance"];
/// A `front` older than this counts as no game (the plugin sends one every 2 s while a game runs).
pub const FRONT_TTL: Duration = Duration::from_secs(6);
const MAX_LINE: usize = 64 * 1024;
const MAX_CLIENTS: usize = 8;
/// How long the service waits at start for Steam's gamescope (its display, for screenshots and the focused game).
const ENV_WAIT: Duration = Duration::from_secs(45);

pub fn socket_path() -> PathBuf { crate::overlay_link::runtime_dir().join("hearth").join("core.sock") }

/// The game Steam runs, as the plugin last said.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Front { pub game: Option<String>, pub appid: u64, pub overlay: bool }

/// The service's state: the last `front` and when it came, gamescope's focused app (asked at most once a second).
#[derive(Default)]
pub struct Shared {
    front: Mutex<Option<(Front, Instant)>>,
    focus: Mutex<Option<(Option<u64>, Instant)>>,
    clients: AtomicUsize,
    aside: Mutex<Vec<Aside>>,
}

/// A game put aside (Quick Resume in Steam Game Mode): Steam's app id, Hearth's id, the numbers its processes carry.
#[derive(Clone, Debug)]
pub struct Aside { pub appid: u32, pub game: Option<String>, pub title: String, pub numbers: Vec<u32>, pub at: Instant, pub at_ms: u64 }

impl Shared {
    pub fn set_front(&self, f: Front) { *self.front.lock().unwrap_or_else(|p| p.into_inner()) = Some((f, Instant::now())); }
    fn front_now(&self) -> Option<(Front, Instant)> { self.front.lock().unwrap_or_else(|p| p.into_inner()).clone() }
    /// gamescope's GAMESCOPE_FOCUSED_APP (Steam's gamescope publishes it), read only, at most once a second.
    fn focused_app(&self) -> Option<u64> {
        let mut f = self.focus.lock().unwrap_or_else(|p| p.into_inner());
        if let Some((v, at)) = *f { if at.elapsed() < Duration::from_secs(1) { return v; } }
        let v = if crate::gamescope::available() { crate::gamescope::cardinals("GAMESCOPE_FOCUSED_APP", None).first().copied() } else { None };
        *f = Some((v, Instant::now()));
        v
    }
    /// What Hearth Input is told: (game, in front, Quick Access open).
    pub fn input_state(&self) -> (Option<String>, bool, bool) {
        let f = self.front_now();
        let focused = if f.as_ref().map(|(x, _)| x.game.is_some()).unwrap_or(false) { self.focused_app() } else { None };
        front_state(f.as_ref(), Instant::now(), focused)
    }
}

/// (game, in front, overlay) from the plugin's last `front`: nothing when it's older than FRONT_TTL or names no game;
/// in front only when Quick Access isn't open and gamescope's focused app (when it's known) is the game's app id.
pub fn front_state(f: Option<&(Front, Instant)>, now: Instant, focused: Option<u64>) -> (Option<String>, bool, bool) {
    let Some((f, at)) = f else { return (None, false, false) };
    if now.saturating_duration_since(*at) > FRONT_TTL { return (None, false, false); }
    let Some(game) = f.game.clone() else { return (None, false, false) };
    let focus_ok = match focused { Some(a) if a != 0 && f.appid != 0 => a == f.appid, _ => true };
    (Some(game), !f.overlay && focus_ok, f.overlay)
}

fn id_ok(v: &Value) -> Option<String> { v.as_str().filter(|i| crate::desktop_games::valid_id(i)).map(str::to_string) }

/// A request checked against the allow-list: (command, its arguments, cleaned). Err: why it's refused.
pub fn check(req: &Value) -> Result<(String, Value), String> {
    let cmd = req["cmd"].as_str().ok_or("No command")?;
    if !COMMANDS.contains(&cmd) { return Err(format!("{cmd:.40} isn't something Hearth's core answers here")); }
    let need_game = || id_ok(&req["game"]).ok_or_else(|| "That isn't a game id Hearth knows.".to_string());
    let args = match cmd {
        "data" => {
            let what = req["what"].as_str().filter(|w| DATA.contains(w)).ok_or("That isn't something the Hearth plugin asks")?;
            let arg = req["arg"].as_str().unwrap_or("");
            let ok = match what {
                "store" => crate::steam_takeover::STORE_PAGES.contains(&arg),
                "info" => crate::desktop_games::valid_id(arg),
                "power-mode" => POWER_MODES.contains(&arg),
                _ => true,
            };
            if !ok { return Err("That isn't something the Hearth plugin asks".into()); }
            json!({"what": what, "arg": arg})
        }
        "front" => {
            let game = if req["game"].is_null() || req["game"] == "" { Value::Null } else { json!(need_game()?) };
            let appid = req["appid"].as_u64().or_else(|| req["appid"].as_str().and_then(|s| s.parse().ok())).unwrap_or(0);
            if appid > u32::MAX as u64 { return Err("That isn't a Steam app id".into()); }
            json!({"game": game, "appid": appid, "overlay": req["overlay"] == true})
        }
        "game" | "mods" => json!({"game": need_game()?}),
        "call" => {
            let m = req["method"].as_str().ok_or("No method")?;
            let (m, keys) = BRIDGE_CALLS.iter().find(|(n, _)| *n == m).ok_or("That isn't something the Hearth plugin asks")?;
            let args = clean_args(&req["args"], keys)?;
            if let Some(id) = args["id"].as_str() {
                let game_id = m.starts_with("game.") || *m == "diskspace.move";
                if game_id && !crate::desktop_games::valid_id(id) { return Err("That isn't a game id Hearth knows.".into()); }
                if !game_id && !recomp_id_ok(id) { return Err("That isn't a recomp Hearth knows.".into()); }
            }
            if let Some(why) = steam_refusal(m, &args) { return Err(why.into()); }
            json!({"method": m, "args": args})
        }
        "mods-call" => {
            let c = req["call"].as_str().ok_or("No call")?;
            let (c, keys) = MODS_CALLS.iter().find(|(n, _)| *n == c).ok_or("That isn't something the Hearth plugin asks")?;
            let args = clean_args(&req["args"], keys)?;
            if *c != "install_download" { if let Some(id) = args["id"].as_str() { if !crate::desktop_games::valid_id(id) { return Err("That isn't a game id Hearth knows.".into()); } } }
            if let Some(g) = args["gameId"].as_str() { if !g.is_empty() && !crate::desktop_games::valid_id(g) { return Err("That isn't a game id Hearth knows.".into()); } }
            json!({"call": c, "args": args})
        }
        "games-disk" => {
            let m = req["mount"].as_str().filter(|m| m.starts_with('/') && m.len() < 400 && !m.contains("..") && !m.chars().any(|c| c.is_control())).ok_or("That isn't a drive")?;
            json!({"mount": m})
        }
        "input-set" => {
            let layout = req["layout"].as_str().ok_or("No layout")?;
            if !matches!(layout, "off" | "custom" | "global") && crate::input_map::template(layout).is_none() { return Err("Unknown layout".into()); }
            json!({"game": need_game()?, "layout": layout})
        }
        "coop" => {
            let op = req["op"].as_str().filter(|o| COOP_OPS.contains(o)).ok_or("That isn't something the Hearth plugin asks")?;
            let pads: Vec<String> = match &req["pads"] {
                Value::Null => Vec::new(),
                Value::Array(l) if l.len() <= 16 => l.iter().map(|v| v.as_str().filter(|p| pad_id_ok(p)).map(str::to_string).ok_or("That isn't a controller"))
                    .collect::<Result<Vec<String>, &str>>()?,
                _ => return Err("That isn't a controller".into()),
            };
            let pad = match req["pad"].as_str() {
                None | Some("") => String::new(),
                Some(p) if pad_id_ok(p) => p.to_string(),
                _ => return Err("That isn't a controller".into()),
            };
            if op == "identify" && pad.is_empty() { return Err("Which controller?".into()); }
            if op == "order" && pads.is_empty() { return Err("No player order".into()); }
            json!({"op": op, "pads": pads, "pad": pad})
        }
        "seats" => {
            let op = req["op"].as_str().filter(|o| SEATS_OPS.contains(o)).ok_or("That isn't something the Hearth plugin asks")?;
            let seat = req["seat"].as_u64().unwrap_or(0);
            let pad = req["pad"].as_str().unwrap_or("");
            match op {
                "start" | "stop" if !(2..=4).contains(&seat) => return Err("Which player? (2 to 4)".into()),
                "assign" if !(1..=4).contains(&seat) => return Err("Which player? (1 to 4)".into()),
                "assign" if !seat_pad_ok(pad) => return Err("That isn't a controller".into()),
                _ => {}
            }
            json!({"op": op, "seat": if matches!(op, "start" | "stop" | "assign") { seat } else { 0 }, "pad": if op == "assign" { pad } else { "" }})
        }
        "aside" => {
            let op = req["op"].as_str().filter(|o| ASIDE_OPS.contains(o)).ok_or("That isn't something the Hearth plugin asks")?;
            let appid = req["appid"].as_u64().unwrap_or(0);
            if appid > u32::MAX as u64 || (op != "list" && appid == 0) { return Err("That isn't a Steam app id".into()); }
            let game = if req["game"].is_null() || req["game"] == "" { Value::Null } else { json!(need_game()?) };
            let title: String = req["title"].as_str().unwrap_or("").chars().filter(|c| !c.is_control()).take(120).collect();
            json!({"op": op, "appid": appid, "game": game, "title": title})
        }
        "link" => {
            let op = req["op"].as_str().filter(|o| LINK_OPS.contains(o)).ok_or("That isn't something the Hearth plugin asks")?;
            let id = |k: &str| req[k].as_str().filter(|v| link_id_ok(v)).map(str::to_string).ok_or("That isn't a Hearth Link device");
            match op {
                "sync" => json!({"op": op, "peer": id("peer")?}),
                "wake" => json!({"op": op, "server": id("server")?}),
                _ => json!({"op": op}),
            }
        }
        "remote" => {
            let op = req["op"].as_str().filter(|o| REMOTE_OPS.contains(o)).ok_or("That isn't something the Hearth plugin asks")?;
            if op != "pair" { json!({"op": op}) } else {
                let pin = req["pin"].as_str().filter(|p| p.len() == 4 && p.chars().all(|c| c.is_ascii_digit())).ok_or("The PIN is the 4 numbers Moonlight shows")?;
                let mut name: String = req["name"].as_str().unwrap_or("").chars().filter(|c| !c.is_control()).take(40).collect();
                if name.trim().is_empty() { name = "Moonlight".into(); }
                json!({"op": op, "pin": pin, "name": name})
            }
        }
        "set-proton" => {
            let p = req["proton"].as_str().ok_or("No Proton version")?;
            if p.len() > 200 || p.contains('/') || p.contains("..") || p.chars().any(|c| c.is_control()) { return Err("That isn't a Proton version".into()); }
            json!({"game": need_game()?, "proton": p})
        }
        _ => json!({}),
    };
    Ok((cmd.to_string(), args))
}

/// Only `keys` of a request's arguments, each a short string, a bool or a list of short strings.
pub fn clean_args(a: &Value, keys: &[&str]) -> Result<Value, String> {
    fn ok_str(s: &str) -> bool { s.len() <= 400 && !s.chars().any(|c| c.is_control()) }
    let mut out = serde_json::Map::new();
    for k in keys {
        let v = &a[*k];
        match v {
            Value::Null => continue,
            Value::Bool(_) => {}
            Value::String(s) if ok_str(s.as_str()) => {}
            Value::Array(l) if l.len() <= 500 && l.iter().all(|x| x.as_str().map(ok_str).unwrap_or(false)) => {}
            _ => return Err(format!("That isn't something the Hearth plugin sends ({k})")),
        }
        out.insert(k.to_string(), v.clone());
    }
    Ok(Value::Object(out))
}

/// A Hearth Recomps id (its catalogue's): letters, digits, '-', '_', '.'.
pub fn recomp_id_ok(id: &str) -> bool {
    !id.is_empty() && id.len() <= 80 && !id.starts_with('.') && id.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

/// Steam games are installed, moved and uninstalled by Steam itself here: Hearth's way closes Steam for it, and Steam
/// runs this Game Mode (closing it would end the session).
pub fn steam_refusal(method: &str, args: &Value) -> Option<&'static str> {
    let steam = args["id"].as_str().map(|i| i.starts_with("steam:")).unwrap_or(false);
    if !steam { return None; }
    match method {
        "game.install" => Some("Steam games install from Steam's own page in Steam Game Mode."),
        "game.uninstall" => Some("Uninstall Steam games from Steam's own page (Manage › Uninstall) in Steam Game Mode."),
        "diskspace.move" => Some("Move Steam games in Steam's Settings › Storage in Steam Game Mode (Hearth would have to close Steam)."),
        _ => None,
    }
}

// ---- Hearth's own calls (`call`): a Bridge kept by this service, made the first time one is asked. Its background
// work's messages come back through stores::notify: answers given later wake the call waiting for them, internal
// messages (an install finished, a GOG move) go to the bridge, events are dropped (the pages ask again).
struct Hub {
    bridge: Mutex<Option<crate::bridge::Bridge>>,
    answers: Mutex<std::collections::HashMap<u64, Value>>,
    cv: std::sync::Condvar,
    next: std::sync::atomic::AtomicU64,
}

fn hub() -> &'static Hub {
    static H: std::sync::OnceLock<Hub> = std::sync::OnceLock::new();
    H.get_or_init(|| {
        crate::stores::set_notify(Arc::new(on_notify));
        Hub { bridge: Mutex::new(None), answers: Mutex::new(Default::default()), cv: std::sync::Condvar::new(), next: std::sync::atomic::AtomicU64::new(1) }
    })
}

fn put_answer(v: Value) {
    let Some(id) = v["id"].as_u64() else { return };
    let h = hub();
    h.answers.lock().unwrap_or_else(|p| p.into_inner()).insert(id, v);
    h.cv.notify_all();
}

fn on_notify(v: Value) {
    match v["type"].as_str().unwrap_or("") {
        "internal" => { std::thread::spawn(move || { for e in with_bridge(|b| b.internal(&v)) { if e["type"] == "result" { put_answer(e); } } }); }
        "result" => put_answer(v),
        _ => {}
    }
}

fn bridge_ready() -> bool { hub().bridge.lock().unwrap_or_else(|p| p.into_inner()).is_some() }

fn with_bridge<R>(f: impl FnOnce(&mut crate::bridge::Bridge) -> R) -> R {
    let mut g = hub().bridge.lock().unwrap_or_else(|p| p.into_inner());
    if g.is_none() {
        let mut b = crate::bridge::Bridge::with(crate::desktop_games::read_library(), crate::profiles::Profiles::open(&crate::paths::data_dir()), env!("CARGO_PKG_VERSION"));
        b.emit = Arc::new(on_notify);
        b.cur_label = "steam".into();
        crate::bridge::log("core service: Hearth's library loaded for the Steam pages (Store, Storage, Recomps)");
        *g = Some(b);
    }
    f(g.as_mut().expect("made above"))
}

const CALL_WAIT: Duration = Duration::from_secs(110);

fn bridge_call(method: &str, args: &Value) -> Result<Value, String> { bridge_call_within(method, args, CALL_WAIT) }

/// The same, waiting up to `wait` for work answered later (Mantel's Settings, settings_cli.rs: installing Decky Loader
/// or cleaning up takes minutes, and the call's process must not end before it's done).
pub(crate) fn bridge_call_within(method: &str, args: &Value, wait: Duration) -> Result<Value, String> {
    let h = hub();
    let id = 1_000_000 + h.next.fetch_add(1, Ordering::SeqCst);
    let reply = with_bridge(|b| b.handle(&json!({"type": "call", "id": id, "method": method, "args": args})));
    let r = match reply {
        Some(r) => r,
        None => {
            let start = Instant::now();
            let mut m = h.answers.lock().unwrap_or_else(|p| p.into_inner());
            loop {
                if let Some(v) = m.remove(&id) { break v; }
                if start.elapsed() > wait { return Err("Hearth is still working on it; look again in a moment.".into()); }
                m = h.cv.wait_timeout(m, Duration::from_millis(500)).map(|x| x.0).unwrap_or_else(|p| p.into_inner().0);
            }
        }
    };
    if r["ok"] == true { Ok(r["value"].clone()) } else { Err(r["error"].as_str().unwrap_or("Hearth couldn't do that").to_string()) }
}

/// "Use this disk for games": Hearth's install folder (new Epic and GOG installs) on a writable drive Hearth lists.
fn games_disk(mount: &str) -> Result<Value, String> {
    use std::os::unix::fs::MetadataExt;
    with_bridge(|b| -> Result<Value, String> {
        let ctx = crate::diskspace::ctx_from(&b.library);
        let d = crate::diskspace::drives(&ctx).into_iter().find(|d| d.mount.to_string_lossy() == mount).ok_or("That drive isn't one Hearth can put games on.")?;
        if d.read_only { return Err("That drive is read only.".into()); }
        let root = b.library.install_root();
        let base = crate::diskspace::games_base(&d.mount, d.dev, &root);
        std::fs::create_dir_all(&base).map_err(|e| format!("Couldn't make {} ({e})", base.display()))?;
        if std::fs::metadata(&base).map(|m| m.dev() != d.dev).unwrap_or(true) { return Err("That folder isn't on that drive.".into()); }
        let dir = base.to_string_lossy().to_string();
        b.library.store.update(|x| { x.insert("installRoot".into(), json!(dir)); }).map_err(|e| e.to_string())?;
        crate::bridge::log(&format!("core service: new Epic and GOG installs go to {} (Steam pages, Storage)", base.display()));
        Ok(json!({"installRoot": b.library.install_root().to_string_lossy(), "drive": d.name}))
    })
}

/// The Hearth Input layouts to pick from for a game: Off, the templates, then custom and global when there are.
pub fn layout_choices(choice: &Value) -> Vec<Value> {
    let mut out = vec![json!({"id": "off", "label": "Off (the game's own controls)"})];
    for t in crate::input_map::tables()["templates"].as_array().into_iter().flatten() {
        if let (Some(id), Some(name)) = (t["id"].as_str(), t["name"].as_str()) { out.push(json!({"id": id, "label": name})); }
    }
    if choice["custom"].is_object() { out.push(json!({"id": "custom", "label": choice["custom"]["name"].as_str().unwrap_or("Your own layout")})); }
    if choice["global"]["layout"].is_object() { out.push(json!({"id": "global", "label": "Your layout for every game"})); }
    out
}

/// How Hearth starts a game (the same rule as Bridge::game_options): "proton" when Hearth picks its Proton.
pub fn play_kind(source: &str, exe: Option<&str>, can_uninstall: bool) -> &'static str {
    let exe_win = exe.map(|e| e.to_lowercase().ends_with(".exe")).unwrap_or(false);
    match source {
        "steam" => "steam", "emu" => "emu", "beacon" => "beacon",
        "manual" | "recomps" => if exe_win { "proton" } else { "native" },
        "epic" | "gog" if can_uninstall => "proton",
        _ => "external",
    }
}

/// The game's mods in Hearth Mods, short: how many, how many on, the first names.
pub fn mods_short(summary: &Value) -> Value {
    let list: Vec<&Value> = summary["mods"].as_array().map(|a| a.iter().filter(|m| m["kind"] != "separator").collect()).unwrap_or_default();
    let on = list.iter().filter(|m| m["enabled"] == true).count();
    let names: Vec<Value> = list.iter().take(12).map(|m| json!({"name": m["name"], "enabled": m["enabled"] == true})).collect();
    json!({"count": list.len(), "enabled": on, "mods": names, "beacon": summary["beacon"] == true})
}

fn game_view(id: &str) -> Result<Value, String> {
    let lib = crate::desktop_games::read_library();
    let (title, source, kind) = match lib.games.get(id) {
        Some(g) => (g.title.clone(), g.source.to_string(), play_kind(g.source, g.exe.as_deref(), g.can_uninstall)),
        None if id.starts_with("steam:") => (String::new(), "steam".to_string(), "steam"),
        None => return Err("That game isn't in Hearth's library.".into()),
    };
    let input = crate::input_map::call("input.get", &json!({"game": id}), None).unwrap_or_else(|_| json!({"layout": "off"}));
    let options = crate::game_options::get(&lib.store, id);
    let protons = if kind == "proton" { crate::game_options::proton_choices(&crate::paths::home()) } else { vec![] };
    Ok(json!({
        "id": id, "title": title, "source": source, "store": crate::steam_takeover::store_label(&source), "kind": kind,
        "input": {"layout": input["layout"], "name": input["resolved"]["name"], "choices": layout_choices(&input)},
        "proton": options["proton"], "protonChoices": protons, "mods": crate::hearth_mods::available(),
    }))
}

fn set_proton(id: &str, p: &str) -> Result<Value, String> {
    let choices = crate::game_options::proton_choices(&crate::paths::home());
    if !choices.iter().any(|c| c["id"] == p) { return Err("That Proton version isn't installed here.".into()); }
    // A fresh read of the library file, changed and written at once (nimbus --play-now writes it too).
    let mut store = crate::store::Store::open(crate::paths::data_dir().join("library.json"));
    crate::game_options::set(&mut store, id, &json!({"proton": p}))?;
    Ok(json!({"id": id, "proton": p}))
}

fn record() -> Result<Value, String> {
    if !crate::system::has("hearth-recorder") { return Err("Hearth Recorder isn't installed".into()); }
    let text = match crate::system::run_timeout("hearth-recorder", &["--toggle", "--json"], Duration::from_secs(70)) { Ok(t) => t, Err(e) => e };
    let v: Value = serde_json::from_str(text.trim()).map_err(|_| text.trim().chars().take(300).collect::<String>())?;
    if v["ok"] == false { return Err(v["error"].as_str().unwrap_or("Recorder couldn't do that").to_string()); }
    Ok(v)
}

/// Couch Co-op for Steam Game Mode: hearth-coop's own player order, rumble and game list (the same helper Hearth's
/// Couch Co-op uses; its split screen is started by the game's Steam shortcut, `hearth-coop play`, not from here).
fn coop(a: &Value) -> Result<Value, String> {
    use crate::multiplay::{ask, COOP};
    if !Path::new(COOP).exists() { return Err("Couch Co-op isn't part of this Hearth OS.".into()); }
    let pads = a["pads"].as_array().map(|l| l.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(",")).unwrap_or_default();
    match a["op"].as_str().unwrap_or("") {
        "players" => ask(COOP, &["players"]),
        "order" => ask(COOP, &["order-set", "--pads", &pads]),
        "identify" => ask(COOP, &["identify", "--pad", a["pad"].as_str().unwrap_or("")]),
        "games" => ask(COOP, &["games"]),
        "check" => ask(COOP, &["check"]),
        _ => Err("Unknown command".into()),
    }
}

/// Multi-seat beside Steam Game Mode: what's set up and running, the controllers, one extra player started or stopped, a
/// controller moved (hearth-seats, the same helper Hearth's Settings › Multi-seat uses; its root side asks polkit, which
/// this service can answer for because it runs in the signed-in person's session).
fn seats(a: &Value) -> Result<Value, String> {
    use crate::multiplay::{ask, SEATS};
    if !crate::multiplay::seats_available() { return Err("Multi-seat isn't part of this Hearth OS.".into()); }
    let seat = a["seat"].as_u64().unwrap_or(0).to_string();
    match a["op"].as_str().unwrap_or("") {
        "status" => ask(SEATS, &["status"]),
        "pads" => ask(SEATS, &["pads"]),
        "start" => ask(SEATS, &["start", "--seat", &seat]),
        "stop" => ask(SEATS, &["stop", "--seat", &seat]),
        "assign" => ask(SEATS, &["assign-pad", "--pad", a["pad"].as_str().unwrap_or(""), "--seat", &seat]),
        _ => Err("Unknown command".into()),
    }
}

// ---------------------------------------------------------------- Quick Resume in Steam Game Mode

fn now_ms() -> u64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0) }

/// The numbers a game's processes carry: Steam's app id (its reaper's, a Hearth shortcut's too), a Steam game's own app
/// id (SteamAppId), and Hearth's number for one of Hearth's games (HEARTH_GAME_ID: inside Flathub's Steam, Hearth's
/// games run on the system through flatpak-spawn, outside Steam's reaper). Only numbers Hearth already gave out.
pub fn aside_numbers(appid: u32, game: Option<&str>, known: &Value) -> Vec<u32> {
    let mut n = vec![appid];
    if let Some(g) = game {
        if let Some(a) = crate::appid::steam_app_id(g) { n.push(a); }
        if let Some(h) = known[g].as_u64().filter(|h| *h > 0 && *h <= u32::MAX as u64) { n.push(h as u32); }
    }
    n.retain(|x| *x != 0);
    n.dedup();
    n
}

/// This user's processes that carry one of these numbers (never this service, never Steam's own).
fn aside_pids(numbers: &[u32]) -> Vec<u32> {
    let me = std::process::id();
    let snap = crate::procs::snapshot(Path::new("/proc"));
    let mut v: Vec<u32> = crate::procs::owners(&snap, me).into_iter().filter(|(p, o)| *p != me && numbers.contains(o)).map(|(p, _)| p).collect();
    v.sort();
    v
}

fn aside_list(sh: &Shared) -> Value {
    let mut l = sh.aside.lock().unwrap_or_else(|p| p.into_inner());
    // Closed some other way (Steam's Exit game, it crashed): not aside any more.
    l.retain(|e| !aside_pids(&e.numbers).is_empty());
    json!({"list": l.iter().map(|e| json!({"appid": e.appid, "game": e.game, "title": e.title, "at": e.at_ms})).collect::<Vec<_>>()})
}

fn aside_thaw(e: &Aside, why: &str) {
    let pids = aside_pids(&e.numbers);
    let n = crate::procs::signal(&pids, "CONT");
    crate::bridge::log(&format!("core service: quick resume: {} (app {}) carries on ({why}): {} ({n} resumed)", e.game.as_deref().unwrap_or("steam game"), e.appid, crate::procs::describe(&pids)));
}

fn aside(sh: &Shared, a: &Value) -> Result<Value, String> {
    let appid = a["appid"].as_u64().unwrap_or(0) as u32;
    let game = a["game"].as_str().map(str::to_string);
    let taken = |sh: &Shared| -> Option<Aside> {
        let mut l = sh.aside.lock().unwrap_or_else(|p| p.into_inner());
        l.iter().position(|e| e.appid == appid).map(|i| l.remove(i))
    };
    match a["op"].as_str().unwrap_or("") {
        "put" => {
            if sh.aside.lock().unwrap_or_else(|p| p.into_inner()).iter().any(|e| e.appid == appid) { return Err("It's already put aside.".into()); }
            let known = crate::store::Store::open(crate::paths::data_dir().join("library.json")).data["appIds"].clone();
            let numbers = aside_numbers(appid, game.as_deref(), &known);
            let pids = aside_pids(&numbers);
            let stopped = if pids.is_empty() { 0 } else { crate::procs::signal(&pids, "STOP") };
            if stopped == 0 {
                crate::bridge::log(&format!("core service: quick resume: couldn't put app {appid} aside: {} found", crate::procs::describe(&pids)));
                return Err("Couldn't find the game's processes, so it can't be put aside.".into());
            }
            crate::bridge::log(&format!("core service: quick resume: {} (app {appid}) put aside: {} ({stopped} paused)", game.as_deref().unwrap_or("steam game"), crate::procs::describe(&pids)));
            let title = a["title"].as_str().unwrap_or("").to_string();
            sh.aside.lock().unwrap_or_else(|p| p.into_inner()).insert(0, Aside { appid, game, title, numbers, at: Instant::now(), at_ms: now_ms() });
            Ok(aside_list(sh))
        }
        "resume" => {
            let e = taken(sh).ok_or("That game isn't put aside any more.")?;
            aside_thaw(&e, "Resume in Quick Access");
            Ok(aside_list(sh))
        }
        "close" => {
            let e = taken(sh).ok_or("That game isn't put aside any more.")?;
            let pids = aside_pids(&e.numbers);
            crate::bridge::log(&format!("core service: quick resume: closing app {} ({})", e.appid, crate::procs::describe(&pids)));
            crate::procs::stop(&pids, Duration::from_secs(5));   // woken first: a stopped game only acts on "close" once it runs
            Ok(aside_list(sh))
        }
        _ => Ok(aside_list(sh)),
    }
}

/// Every `front`: a game put aside carries on once it's in front again (Steam's Resume, gamescope's focused app) with
/// Quick Access closed, and is let go when Steam runs another game or none (Steam closed it).
pub fn aside_follow(sh: &Shared, running: u32, overlay: bool) {
    if sh.aside.lock().unwrap_or_else(|p| p.into_inner()).is_empty() { return; }
    let focused = sh.focused_app();
    let mut l = sh.aside.lock().unwrap_or_else(|p| p.into_inner());
    let mut keep = Vec::new();
    for e in l.drain(..) {
        match aside_step(&e, running, focused, overlay, e.at.elapsed()) {
            Some(why) => aside_thaw(&e, why),
            None => keep.push(e),
        }
    }
    *l = keep;
}

/// Why a game put aside should carry on now, or None to leave it. The first seconds after putting it aside are left
/// alone (Steam's screen takes a moment to come in front of it).
pub fn aside_step(e: &Aside, running: u32, focused: Option<u64>, overlay: bool, age: Duration) -> Option<&'static str> {
    if running != e.appid { return Some("Steam isn't running it any more"); }
    if age < Duration::from_secs(5) || overlay { return None; }
    match focused { Some(f) if f == e.appid as u64 => Some("back in front"), _ => None }
}

/// When the service ends (Steam Game Mode ended): nothing it stopped stays stopped.
fn aside_release(sh: &Shared) {
    for e in sh.aside.lock().unwrap_or_else(|p| p.into_inner()).drain(..) { aside_thaw(&e, "Steam Game Mode ended"); }
}

// ---------------------------------------------------------------- Hearth Friends, Hearth Link, Sunshine

/// Hearth Friends, read only: who's a friend, online, playing what. Never chats, accounts or tokens.
fn friends() -> Result<Value, String> {
    let st = crate::matrix_friends::call("status", &json!({}))?;
    if st["signedIn"] != true { return Ok(json!({"signedIn": false, "friends": []})); }
    let list = crate::matrix_friends::call("friends", &json!({}))?;
    Ok(json!({"signedIn": true, "friends": friends_short(&list)}))
}

/// Friends (not pending requests), online and playing first, only what the page shows.
pub fn friends_short(list: &Value) -> Vec<Value> {
    let mut v: Vec<Value> = list.as_array().into_iter().flatten().filter(|f| f["state"] == "friend")
        .map(|f| json!({"name": f["name"].as_str().or(f["userId"].as_str()).unwrap_or("Friend"), "presence": f["presence"].as_str().unwrap_or("unknown"),
                        "game": f["game"].as_str(), "joinable": f["joinable"] == true}))
        .collect();
    let rank = |f: &Value| (if f["game"].is_string() { 0 } else if f["presence"] == "online" { 1 } else { 2 }, f["name"].as_str().unwrap_or("").to_lowercase());
    v.sort_by_key(rank);
    v.truncate(100);
    v
}

/// Hearth Link's status, short: never the device's long link (its ticket) or settings.
pub fn link_short(v: &Value) -> Value {
    let peers: Vec<Value> = v["peers"].as_array().into_iter().flatten().map(|p| json!({"id": p["id"], "name": p["name"], "online": p["online"] == true})).collect();
    let copies: Vec<Value> = v["transfers"].as_array().into_iter().flatten().take(10)
        .map(|t| json!({"title": t["title"], "state": t["state"], "done": t["done"], "total": t["total"], "from": t["from"]})).collect();
    let servers: Vec<Value> = v["storage"]["servers"].as_array().into_iter().flatten()
        .map(|s| json!({"id": s["id"], "name": s["name"], "free": s["free"], "usable": s["usable"], "canWake": s["canWake"] == true, "seen": s["at"]})).collect();
    json!({"running": true, "name": v["me"]["name"], "peers": peers, "copies": copies, "offers": v["offers"].as_array().map(|o| o.len()).unwrap_or(0),
           "servers": servers, "playing": v["playing"] == true})
}

fn link(a: &Value) -> Result<Value, String> {
    match a["op"].as_str().unwrap_or("") {
        "sync" => crate::link::call("sync.now", &json!({"peer": a["peer"]}), 60).map(|_| json!({"ok": true})),
        "wake" => crate::link::call("storage.wake", &json!({"server": a["server"]}), 15),
        _ => Ok(match crate::link::call("status", &json!({}), 8) {
            Ok(v) => link_short(&v),
            Err(e) => json!({"running": false, "installed": crate::link::available(), "error": e}),
        }),
    }
}

/// Sunshine: the same as Settings › Remote play (remote.rs).
fn remote(a: &Value) -> Result<Value, String> {
    match a["op"].as_str().unwrap_or("") {
        "on" => crate::remote::set_on(true),
        "off" => crate::remote::set_on(false),
        "pair" => {
            crate::remote::pair(a["pin"].as_str().unwrap_or(""), a["name"].as_str().unwrap_or("Moonlight"))?;
            crate::bridge::log("core service: remote play: a Moonlight device was paired");
            Ok(crate::remote::status())
        }
        _ => Ok(crate::remote::status()),
    }
}

/// One checked command's answer.
pub fn answer(sh: &Shared, cmd: &str, a: &Value) -> Result<Value, String> {
    let game = a["game"].as_str().unwrap_or("");
    match cmd {
        "ping" => Ok(json!({"service": "hearth-core", "version": env!("CARGO_PKG_VERSION"), "pid": std::process::id()})),
        // Downloads Hearth started from the Steam pages are in this service's own queue.
        "data" if a["what"] == "downloads" && bridge_ready() => Ok(with_bridge(|b| crate::steam_takeover::downloads_page(&b.library.downloads()))),
        "data" => crate::steam_takeover::answer(a["what"].as_str().unwrap_or(""), a["arg"].as_str().filter(|s| !s.is_empty())),
        "front" => {
            sh.set_front(Front { game: a["game"].as_str().map(str::to_string), appid: a["appid"].as_u64().unwrap_or(0), overlay: a["overlay"] == true });
            aside_follow(sh, a["appid"].as_u64().unwrap_or(0) as u32, a["overlay"] == true);
            let (g, front, overlay) = sh.input_state();
            let st = crate::input_map::state(g.as_deref(), front, overlay);
            Ok(json!({"game": g, "front": front, "overlay": overlay, "layout": st["layout"]["name"], "active": front && st["layout"].is_object()}))
        }
        "game" => game_view(game),
        "input-set" => crate::input_map::call("input.set", &json!({"game": game, "layout": a["layout"]}), None)
            .map(|c| json!({"game": game, "layout": c["layout"], "name": c["resolved"]["name"]})),
        "set-proton" => set_proton(game, a["proton"].as_str().unwrap_or("")),
        "mods" => {
            if !crate::hearth_mods::available() { return Err("Hearth Mods isn't installed".into()); }
            crate::hearth_mods::call("mods_of", &json!({"id": game}), false).map(|s| mods_short(&s))
                .map_err(|_| "Open Hearth Mods to see this game's mods".to_string())
        }
        "screenshot" => crate::gamescope::screenshot(),
        "record" => record(),
        "call" => bridge_call(a["method"].as_str().unwrap_or(""), &a["args"]).map(|v| if v.is_object() { v } else { json!({"value": v}) }),
        "mods-call" => {
            if !crate::hearth_mods::available() { return Err("Hearth Mods isn't installed".into()); }
            let c = a["call"].as_str().unwrap_or("");
            crate::hearth_mods::call(c, &a["args"], true).map(|v| if v.is_object() { v } else { json!({"value": v}) })
        }
        "games-disk" => games_disk(a["mount"].as_str().unwrap_or("")),
        "coop" => coop(a),
        "seats" => seats(a),
        "aside" => aside(sh, a),
        "friends" => friends(),
        "link" => link(a),
        "remote" => remote(a),
        _ => Err("Unknown command".into()),
    }
}

/// One line in, one line out (the id is given back, short).
pub fn respond(sh: &Shared, line: &str) -> Value {
    let req: Value = match serde_json::from_str(line) { Ok(v) => v, Err(_) => return json!({"ok": false, "error": "Not JSON"}) };
    let id = match &req["id"] { Value::Number(n) => json!(n), Value::String(s) if s.len() <= 64 => json!(s), _ => Value::Null };
    match check(&req).and_then(|(cmd, a)| answer(sh, &cmd, &a)) {
        Ok(v) => json!({"id": id, "ok": true, "data": v}),
        Err(e) => json!({"id": id, "ok": false, "error": e}),
    }
}

/// Only this user may talk to it.
pub fn peer_allowed(peer: Option<u32>, me: u32) -> bool { peer == Some(me) }

/// Listens at `path` (its folder made this user's own, the socket 0600) and answers on a thread per connection.
pub fn listen(path: &Path, sh: Arc<Shared>) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let dir = path.parent().ok_or("no folder for the socket")?;
    crate::overlay_link::prepare_dir(dir)?;
    let _ = std::fs::remove_file(path);
    let listener = UnixListener::bind(path).map_err(|e| format!("{}: {e}", path.display()))?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).map_err(|e| format!("{}: {e}", path.display()))?;
    std::thread::spawn(move || {
        for conn in listener.incoming() {
            let Ok(s) = conn else { continue };
            if !peer_allowed(crate::overlay_link::peer_uid(&s), crate::overlay_link::uid()) { crate::bridge::log("core service: refused a connection from another user"); continue; }
            if sh.clients.load(Ordering::SeqCst) >= MAX_CLIENTS { continue; }
            sh.clients.fetch_add(1, Ordering::SeqCst);
            let sh2 = sh.clone();
            std::thread::spawn(move || { serve(&sh2, s); sh2.clients.fetch_sub(1, Ordering::SeqCst); });
        }
    });
    Ok(())
}

fn serve(sh: &Shared, s: UnixStream) {
    let Ok(mut w) = s.try_clone() else { return };
    let _ = w.set_write_timeout(Some(Duration::from_secs(10)));
    let mut r = BufReader::new(s);
    let mut buf = Vec::new();
    loop {
        buf.clear();
        match (&mut r).take(MAX_LINE as u64 + 1).read_until(b'\n', &mut buf) {
            Ok(0) | Err(_) => break,
            Ok(_) => {
                if buf.len() > MAX_LINE { let _ = w.write_all(b"{\"ok\":false,\"error\":\"Too long\"}\n"); break; }
                let text = String::from_utf8_lossy(&buf);
                let t = text.trim();
                if t.is_empty() { continue; }
                let out = respond(sh, t);
                if w.write_all(format!("{out}\n").as_bytes()).is_err() { break; }
            }
        }
    }
}

/// From a process's environment (NUL-separated): gamescope's display variables, when it's a program gamescope started.
pub fn gamescope_env(environ: &[u8]) -> Vec<(String, String)> {
    let vars: Vec<(String, String)> = environ.split(|b| *b == 0).filter_map(|e| {
        let s = String::from_utf8_lossy(e);
        let (k, v) = s.split_once('=')?;
        Some((k.to_string(), v.to_string()))
    }).collect();
    if !vars.iter().any(|(k, v)| k == "GAMESCOPE_WAYLAND_DISPLAY" && !v.is_empty()) { return vec![]; }
    vars.into_iter().filter(|(k, v)| matches!(k.as_str(), "DISPLAY" | "GAMESCOPE_WAYLAND_DISPLAY" | "XAUTHORITY") && !v.is_empty()).collect()
}

/// Steam's gamescope's display variables, from this user's process that run_steam_mode started (HEARTH_STEAM_MODE=1).
fn find_gamescope_env() -> Vec<(String, String)> {
    use std::os::unix::fs::MetadataExt;
    let me = crate::overlay_link::uid();
    let Ok(rd) = std::fs::read_dir("/proc") else { return vec![] };
    for e in rd.flatten() {
        if e.file_name().to_str().and_then(|n| n.parse::<u32>().ok()).is_none() { continue; }
        if e.metadata().map(|m| m.uid() != me).unwrap_or(true) { continue; }
        let Ok(env) = std::fs::read(e.path().join("environ")) else { continue };
        if !env.split(|b| *b == 0).any(|x| x == b"HEARTH_STEAM_MODE=1") { continue; }
        let v = gamescope_env(&env);
        if v.iter().any(|(k, _)| k == "DISPLAY") { return v; }
    }
    vec![]
}

/// Already running (the socket answers a ping)?
fn running(path: &Path) -> bool {
    let Ok(mut s) = UnixStream::connect(path) else { return false };
    let _ = s.set_read_timeout(Some(Duration::from_secs(2)));
    if s.write_all(b"{\"cmd\":\"ping\"}\n").is_err() { return false; }
    let mut line = String::new();
    BufReader::new(s).read_line(&mut line).map(|n| n > 0 && line.contains("\"ok\":true")).unwrap_or(false)
}

/// `--while <file>`: the file the service lives with (run_steam_mode's running mark).
pub fn while_file(args: &[String]) -> Option<PathBuf> {
    let i = args.iter().position(|a| a == "--while")?;
    args.get(i + 1).filter(|s| !s.is_empty()).map(PathBuf::from)
}

/// The command line. Never returns while it serves; ends when the `--while` file is gone (Steam Game Mode ended).
pub fn run(args: &[String]) -> i32 {
    if std::env::var_os("HEARTH_SEAT").is_some_and(|s| !s.is_empty()) { eprintln!("core service: not for extra multi-seat players"); return 0; }
    let path = socket_path();
    if running(&path) { eprintln!("core service: already running"); return 0; }
    let keep = while_file(args);
    // Before any thread starts: gamescope's display, for screenshots and gamescope's focused game (read only).
    if std::env::var_os("GAMESCOPE_WAYLAND_DISPLAY").is_none() {
        let start = Instant::now();
        loop {
            let v = find_gamescope_env();
            if !v.is_empty() { for (k, val) in v { std::env::set_var(k, val); } break; }
            if start.elapsed() > ENV_WAIT || keep.as_ref().map(|f| !f.exists()).unwrap_or(false) { break; }
            std::thread::sleep(Duration::from_millis(500));
        }
    }
    let sh = Arc::new(Shared::default());
    if let Err(e) = listen(&path, sh.clone()) { eprintln!("core service: {e}"); crate::bridge::log(&format!("core service: couldn't listen: {e}")); return 1; }
    crate::bridge::log(&format!("core service: Hearth's core without a window for Steam Game Mode ({}; display {})", path.display(),
        std::env::var("DISPLAY").unwrap_or_else(|_| "none".into())));
    // Hearth Input (when the session started it): told the game in front; it holds the controller only for a layout.
    let st = sh.clone();
    crate::input_map::start_link(Arc::new(move || st.input_state()), Arc::new(|ev: Value| {
        if ev["t"] == "osd" { crate::bridge::log(&format!("core service: Hearth Input: {}", ev["text"].as_str().unwrap_or(""))); }
    }));
    loop {
        std::thread::sleep(Duration::from_secs(3));
        if let Some(f) = &keep {
            if !f.exists() {
                aside_release(&sh);
                let _ = std::fs::remove_file(&path);
                crate::bridge::log("core service: Steam Game Mode ended; stopping");
                return 0;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(v: Value) -> Result<(String, Value), String> { check(&v) }

    #[test]
    fn only_known_commands_with_checked_arguments() {
        assert!(req(json!({"cmd": "ping"})).is_ok());
        for bad in [json!({}), json!({"cmd": "exec"}), json!({"cmd": "rm -rf"}), json!({"cmd": 5}), json!({"cmd": "power.action"})] {
            assert!(req(bad.clone()).is_err(), "{bad} should be refused");
        }
        assert_eq!(req(json!({"cmd": "data", "what": "home"})).unwrap().1, json!({"what": "home", "arg": ""}));
        assert!(req(json!({"cmd": "data", "what": "store", "arg": "epic"})).is_ok());
        assert!(req(json!({"cmd": "data", "what": "store", "arg": "nope"})).is_err());
        assert!(req(json!({"cmd": "data", "what": "power-mode", "arg": "turbo"})).is_err());
        assert!(req(json!({"cmd": "data", "what": "info", "arg": "no colon"})).is_err());
        assert!(req(json!({"cmd": "data", "what": "delete"})).is_err());
        assert!(req(json!({"cmd": "game", "game": "gog:1"})).is_ok());
        assert!(req(json!({"cmd": "game", "game": "bad\"id:1"})).is_err());
        assert!(req(json!({"cmd": "mods"})).is_err());
        assert!(req(json!({"cmd": "input-set", "game": "gog:1", "layout": "shooter"})).is_ok());
        assert!(req(json!({"cmd": "input-set", "game": "gog:1", "layout": "off"})).is_ok());
        assert!(req(json!({"cmd": "input-set", "game": "gog:1", "layout": "exec:x"})).is_err());
        assert!(req(json!({"cmd": "set-proton", "game": "gog:1", "proton": "GE-Proton9-20"})).is_ok());
        assert!(req(json!({"cmd": "set-proton", "game": "gog:1", "proton": "../../bin/sh"})).is_err());
        let f = req(json!({"cmd": "front", "game": "epic:a", "appid": "3000000000", "overlay": true})).unwrap().1;
        assert_eq!(f, json!({"game": "epic:a", "appid": 3000000000u64, "overlay": true}));
        assert_eq!(req(json!({"cmd": "front", "game": null})).unwrap().1["game"], Value::Null);
        assert!(req(json!({"cmd": "front", "game": "epic:a", "appid": 99999999999u64})).is_err());
        // The Steam pages' calls: only listed ones, only their own arguments, never a Steam game installed or moved here.
        let c = req(json!({"cmd": "call", "method": "game.install", "args": {"id": "gog:1", "extra": "x"}})).unwrap().1;
        assert_eq!(c, json!({"method": "game.install", "args": {"id": "gog:1"}}));
        assert!(req(json!({"cmd": "call", "method": "game.install", "args": {"id": "steam:620"}})).unwrap_err().contains("Steam's own page"));
        assert!(req(json!({"cmd": "call", "method": "diskspace.move", "args": {"id": "steam:620", "to": "/run/media/a/sd"}})).is_err());
        assert!(req(json!({"cmd": "call", "method": "game.uninstall", "args": {"id": "steam:620"}})).is_err());
        // Quick Resume, Hearth Link, Sunshine: only their own operations, checked arguments.
        assert_eq!(req(json!({"cmd": "aside", "op": "put", "appid": 620, "game": "steam:620", "title": "Portal 2\n"})).unwrap().1,
                   json!({"op": "put", "appid": 620, "game": "steam:620", "title": "Portal 2"}));
        assert!(req(json!({"cmd": "aside", "op": "put"})).is_err());
        assert!(req(json!({"cmd": "aside", "op": "kill", "appid": 620})).is_err());
        assert!(req(json!({"cmd": "aside", "op": "list"})).is_ok());
        assert!(req(json!({"cmd": "link", "op": "sync", "peer": "abc123"})).is_ok());
        assert!(req(json!({"cmd": "link", "op": "sync", "peer": "../x y"})).is_err());
        assert!(req(json!({"cmd": "link", "op": "unpair", "peer": "abc"})).is_err());
        assert!(req(json!({"cmd": "remote", "op": "pair", "pin": "1234"})).unwrap().1["name"] == "Moonlight");
        assert!(req(json!({"cmd": "remote", "op": "pair", "pin": "12a4"})).is_err());
        assert!(req(json!({"cmd": "remote", "op": "login", "user": "x"})).is_err());
        assert!(req(json!({"cmd": "call", "method": "diskspace.move", "args": {"id": "epic:a", "to": "/run/media/a/sd"}})).is_ok());
        assert!(req(json!({"cmd": "call", "method": "accounts.signIn", "args": {"store": "epic"}})).is_err(), "never sign-ins");
        assert!(req(json!({"cmd": "call", "method": "store.web"})).is_err());
        assert!(req(json!({"cmd": "call", "method": "recomps.install", "args": {"id": "../x"}})).is_err());
        assert!(req(json!({"cmd": "call", "method": "recomps.install", "args": {"id": "zelda64recomp"}})).is_ok());
        assert!(req(json!({"cmd": "call", "method": "diskspace.clean", "args": {"keys": ["a", "b"], "confirmed": []}})).is_ok());
        assert!(req(json!({"cmd": "call", "method": "diskspace.clean", "args": {"keys": [1]}})).is_err());
        assert!(req(json!({"cmd": "mods-call", "call": "toggle", "args": {"id": "steam:489830", "m": "skyui", "on": true}})).is_ok());
        assert!(req(json!({"cmd": "mods-call", "call": "nexus_sign_in", "args": {"key": "k"}})).is_err(), "no accounts from here");
        assert!(req(json!({"cmd": "mods-call", "call": "toggle", "args": {"id": "bad", "m": "x"}})).is_err());
        assert!(req(json!({"cmd": "games-disk", "mount": "/run/media/a/sd"})).is_ok());
        assert!(req(json!({"cmd": "games-disk", "mount": "../etc"})).is_err());
        // Couch Co-op: only its questions, only event nodes as controllers.
        assert_eq!(req(json!({"cmd": "coop", "op": "players"})).unwrap().1, json!({"op": "players", "pads": [], "pad": ""}));
        assert!(req(json!({"cmd": "coop", "op": "order", "pads": ["event30", "event20"]})).is_ok());
        assert!(req(json!({"cmd": "coop", "op": "order", "pads": ["event30", "../dev/sda"]})).is_err());
        assert!(req(json!({"cmd": "coop", "op": "order", "pads": []})).is_err());
        assert!(req(json!({"cmd": "coop", "op": "identify", "pad": "event7"})).is_ok());
        assert!(req(json!({"cmd": "coop", "op": "identify", "pad": "js0"})).is_err());
        assert!(req(json!({"cmd": "coop", "op": "identify"})).is_err());
        assert!(req(json!({"cmd": "coop", "op": "start", "pads": ["event1", "event2"]})).is_err(), "the split screen starts from Steam's shortcut");
        // Multi-seat: only its questions, one extra player at a time, only pad ids hearth-admin takes.
        assert_eq!(req(json!({"cmd": "seats", "op": "status", "seat": 3})).unwrap().1, json!({"op": "status", "seat": 0, "pad": ""}));
        assert_eq!(req(json!({"cmd": "seats", "op": "stop", "seat": 2})).unwrap().1, json!({"op": "stop", "seat": 2, "pad": ""}));
        assert!(req(json!({"cmd": "seats", "op": "start", "seat": 1})).is_err(), "player 1 is never started or stopped here");
        assert!(req(json!({"cmd": "seats", "op": "stop", "seat": 5})).is_err());
        assert!(req(json!({"cmd": "seats", "op": "start"})).is_err());
        assert!(req(json!({"cmd": "seats", "op": "assign", "seat": 2, "pad": "usb:1-2.4"})).is_ok());
        assert!(req(json!({"cmd": "seats", "op": "assign", "seat": 1, "pad": "uniq:aa:bb:cc:dd:ee:ff"})).is_ok());
        assert!(req(json!({"cmd": "seats", "op": "assign", "seat": 2, "pad": "event7"})).is_err());
        assert!(req(json!({"cmd": "seats", "op": "assign", "seat": 2, "pad": "usb:1-2; rm"})).is_err());
        assert!(req(json!({"cmd": "seats", "op": "setup"})).is_err(), "never set up from here");
        assert!(req(json!({"cmd": "seats", "op": "remove"})).is_err(), "never removed from here");
        assert!(seat_pad_ok("usb:3-1.4.2") && seat_pad_ok("uniq:0123") && !seat_pad_ok("usb:1") && !seat_pad_ok("usb:1-") && !seat_pad_ok("uniq:a b c"));
        assert!(pad_id_ok("event12") && !pad_id_ok("event") && !pad_id_ok("event1x") && !pad_id_ok("event12345678"));
        for (m, _) in MODS_CALLS { assert!(crate::hearth_mods::allowed(m), "{m} is a Hearth Mods call"); }
        // The same lists as the plugin's backend (main.py DATA, CORE_COMMANDS; its tests check the other way).
        assert_eq!(DATA, ["home", "library", "store", "downloads", "storage", "info", "power", "power-mode"]);
    }

    #[test]
    fn the_game_in_front() {
        let now = Instant::now();
        let f = |game: Option<&str>, appid: u64, overlay: bool| (Front { game: game.map(str::to_string), appid, overlay }, now);
        assert_eq!(front_state(None, now, None), (None, false, false));
        assert_eq!(front_state(Some(&f(None, 0, false)), now, None), (None, false, false));
        assert_eq!(front_state(Some(&f(Some("gog:a"), 10, false)), now, None), (Some("gog:a".into()), true, false));
        assert_eq!(front_state(Some(&f(Some("gog:a"), 10, false)), now, Some(10)), (Some("gog:a".into()), true, false));
        assert_eq!(front_state(Some(&f(Some("gog:a"), 10, false)), now, Some(769)), (Some("gog:a".into()), false, false), "Steam's own screen in front");
        assert_eq!(front_state(Some(&f(Some("gog:a"), 10, true)), now, Some(10)), (Some("gog:a".into()), false, true), "Quick Access open");
        let old = (Front { game: Some("gog:a".into()), appid: 10, overlay: false }, now - Duration::from_secs(10));
        assert_eq!(front_state(Some(&old), now, Some(10)), (None, false, false), "the plugin went quiet: let go");
        let sh = Shared::default();
        assert_eq!(sh.input_state(), (None, false, false));
    }

    #[test]
    fn answers_and_refusals_are_lines() {
        let sh = Shared::default();
        let r = respond(&sh, r#"{"id": 7, "cmd": "ping"}"#);
        assert_eq!(r["id"], 7);
        assert_eq!(r["ok"], true);
        assert_eq!(r["data"]["service"], "hearth-core");
        let r = respond(&sh, r#"{"id": "x", "cmd": "shutdown"}"#);
        assert_eq!(r["ok"], false);
        assert!(r["error"].as_str().unwrap().contains("isn't something"));
        assert_eq!(respond(&sh, "not json")["ok"], false);
        let r = respond(&sh, r#"{"cmd": "front", "game": "gog:a", "appid": 5}"#);
        assert_eq!(r["data"]["game"], "gog:a");
        assert_eq!(r["data"]["active"], false, "no layout: Hearth Input never holds the controller");
    }

    #[test]
    fn layouts_kinds_and_mods() {
        let c = layout_choices(&json!({"custom": null, "global": {"layout": null}}));
        assert_eq!(c[0]["id"], "off");
        assert!(c.iter().any(|x| x["id"] == "shooter"));
        assert!(!c.iter().any(|x| x["id"] == "custom" || x["id"] == "global"));
        let c = layout_choices(&json!({"custom": {"name": "Mine"}, "global": {"layout": {"name": "G"}}}));
        assert!(c.iter().any(|x| x["id"] == "custom" && x["label"] == "Mine"));
        assert!(c.iter().any(|x| x["id"] == "global"));
        assert_eq!(play_kind("gog", None, true), "proton");
        assert_eq!(play_kind("gog", None, false), "external");
        assert_eq!(play_kind("manual", Some("C:/x/Game.EXE"), false), "proton");
        assert_eq!(play_kind("manual", Some("/usr/bin/game"), false), "native");
        assert_eq!(play_kind("steam", None, false), "steam");
        let m = mods_short(&json!({"mods": [{"name": "A", "enabled": true}, {"name": "S", "kind": "separator"}, {"name": "B", "enabled": false}], "beacon": false}));
        assert_eq!(m["count"], 2);
        assert_eq!(m["enabled"], 1);
        assert_eq!(m["mods"][1]["name"], "B");
        assert_eq!(mods_short(&json!({}))["count"], 0);
    }

    #[test]
    fn gamescope_display_from_steams_environment() {
        let env = b"HOME=/home/a\0DISPLAY=:1\0GAMESCOPE_WAYLAND_DISPLAY=gamescope-0\0XAUTHORITY=/run/x\0STEAM_X=1\0";
        let v = gamescope_env(env);
        assert_eq!(v.len(), 3);
        assert!(v.contains(&("DISPLAY".into(), ":1".into())));
        assert!(gamescope_env(b"DISPLAY=:0\0HOME=/x\0").is_empty(), "not a program gamescope started");
        assert_eq!(while_file(&["nimbus".into(), "--core-service".into(), "--while".into(), "/run/x.pid".into()]), Some(PathBuf::from("/run/x.pid")));
        assert_eq!(while_file(&["nimbus".into(), "--core-service".into()]), None);
    }

    #[test]
    fn socket_is_private_and_answers_this_user() {
        use std::os::unix::fs::PermissionsExt;
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("hearth").join("core.sock");
        listen(&path, Arc::new(Shared::default())).unwrap();
        assert_eq!(std::fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
        assert_eq!(std::fs::metadata(path.parent().unwrap()).unwrap().permissions().mode() & 0o777, 0o700);
        let mut s = UnixStream::connect(&path).unwrap();
        assert_eq!(crate::overlay_link::peer_uid(&s), Some(crate::overlay_link::uid()));
        s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        s.write_all(b"{\"id\":1,\"cmd\":\"ping\"}\n{\"id\":2,\"cmd\":\"exec\",\"arg\":\"sh\"}\n").unwrap();
        let mut r = BufReader::new(s.try_clone().unwrap());
        let mut a = String::new();
        r.read_line(&mut a).unwrap();
        let mut b = String::new();
        r.read_line(&mut b).unwrap();
        let a: Value = serde_json::from_str(&a).unwrap();
        let b: Value = serde_json::from_str(&b).unwrap();
        assert_eq!((a["id"].clone(), a["ok"].clone()), (json!(1), json!(true)));
        assert_eq!((b["id"].clone(), b["ok"].clone()), (json!(2), json!(false)));
        assert!(running(&path));
        // Another user is refused (the check every connection goes through).
        assert!(peer_allowed(Some(1000), 1000));
        assert!(!peer_allowed(Some(0), 1000));
        assert!(!peer_allowed(None, 1000));
        // A line too long is refused and the connection closed.
        let mut s2 = UnixStream::connect(&path).unwrap();
        s2.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        let _ = s2.write_all(&vec![b'a'; MAX_LINE + 10]);
        let mut c = String::new();
        let _ = BufReader::new(s2).read_line(&mut c);
        assert!(c.contains("Too long"), "{c}");
    }

    #[test]
    fn quick_resume_follows_steam() {
        let e = Aside { appid: 620, game: Some("steam:620".into()), title: "Portal 2".into(), numbers: vec![620], at: Instant::now(), at_ms: 0 };
        let s5 = Duration::from_secs(6);
        assert_eq!(aside_step(&e, 620, Some(769), false, s5), None, "Steam's own screen in front: stays aside");
        assert_eq!(aside_step(&e, 620, Some(620), false, Duration::from_secs(1)), None, "just put aside: Steam's screen still coming");
        assert_eq!(aside_step(&e, 620, Some(620), true, s5), None, "Quick Access open over it");
        assert_eq!(aside_step(&e, 620, Some(620), false, s5), Some("back in front"));
        assert_eq!(aside_step(&e, 620, None, false, s5), None, "focus unknown: only Resume in Quick Access");
        assert!(aside_step(&e, 0, Some(769), false, s5).is_some(), "Steam closed it");
        assert!(aside_step(&e, 440, Some(440), false, Duration::ZERO).is_some(), "Steam runs another game");
        let known = json!({"epic:a": 2147487744u64});
        assert_eq!(aside_numbers(3000000001, Some("epic:a"), &known), vec![3000000001, 2147487744]);
        assert_eq!(aside_numbers(620, Some("steam:620"), &known), vec![620]);
        assert_eq!(aside_numbers(620, Some("gog:new"), &known), vec![620], "only numbers Hearth gave out");
    }

    #[test]
    fn friends_and_link_answers_are_short() {
        let f = friends_short(&json!([{"userId": "@a:x", "name": "Ann", "state": "friend", "presence": "offline", "game": null},
            {"userId": "@b:x", "name": "Bo", "state": "friend", "presence": "online", "game": "Hades", "joinable": true, "roomId": "!r"},
            {"userId": "@c:x", "name": "Cy", "state": "pending", "presence": "online"}, {"userId": "@d:x", "name": "Di", "state": "friend", "presence": "online"}]));
        assert_eq!(f.iter().map(|x| x["name"].as_str().unwrap()).collect::<Vec<_>>(), vec!["Bo", "Di", "Ann"]);
        assert!(f[0].get("roomId").is_none() && f[0]["joinable"] == true);
        let l = link_short(&json!({"me": {"name": "Ally", "ticket": "SECRET"}, "settings": {"x": 1}, "peers": [{"id": "p1", "name": "Desk", "online": true, "pairedAt": 1}],
            "transfers": [{"title": "Hades", "state": "copying", "done": 1, "total": 2}], "offers": [{}], "storage": {"servers": [{"id": "s1", "name": "NAS", "canWake": true}]}}));
        assert!(!l.to_string().contains("SECRET"));
        assert_eq!(l["name"], "Ally");
        assert_eq!(l["peers"][0]["name"], "Desk");
        assert_eq!(l["servers"][0]["canWake"], true);
        assert_eq!(l["offers"], 1);
    }
}
