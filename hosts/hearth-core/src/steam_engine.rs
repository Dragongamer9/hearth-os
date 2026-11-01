//! Settings › Library › Steam engine: Classic (the normal Steam client, the default) or Lightweight (experimental):
//! hosts/hearth-steam, a headless service that loads Valve's own steamclient.so with no Steam UI and answers on a
//! local socket. Lightweight never stands between the player and a game: whenever it can't do something (not
//! installed, not running, still starting, signed out, a VAC game, any error, the normal client already running),
//! Hearth tells it to let go and does the same thing with the normal Steam client, exactly as with Classic.
//!
//! Hearth never stores or logs what's typed to sign in: it's handed to the service, which hands it to steamclient.
//!
//! Default: Open, for new installs and for everyone who never chose an engine (once, with a short
//! explainer in Settings; Classic stays one switch away). Until the player signs in to it, everything behaves like
//! Classic. Steam's own Game Mode (Decky) always uses Valve's client.
//!
//! Open is the same service started with HEARTH_STEAM_ENGINE=open: Hearth's own
//! clean-room Steam client on SteamKit2, with no steamclient at all. It signs in, lists the owned library, and
//! installs, updates and verifies games into the user's Steam libraries (normal appmanifest records, so the Downloads
//! page and Valve's client see them). DRM-free games it answers with how to start them, and Hearth starts them itself
//! (natively or with Proton through umu, like Epic and GOG games); Steamworks, VAC and anything else start through
//! Valve's client. The same fallback rules apply: anything it can't do is done with Valve's client, as with Classic.

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

pub const UNIT: &str = "hearth-steam-engine.service";
pub const PROGRAM: &str = "/usr/lib/hearth-steam/hearth-steam";
/// After letting go for the normal client, don't start the engine again for this long.
const COOLDOWN_MS: u64 = 5 * 60_000;
const TICK_MS: u64 = 30_000;
/// The Open engine only lets go when switching engines or on an error it can't stand by for: back sooner.
const OPEN_COOLDOWN_MS: u64 = 60_000;

/// Is the engine part of this system (a build where it didn't build goes without it)?
pub fn installed() -> bool { Path::new(PROGRAM).exists() }

pub fn socket_path() -> PathBuf {
    let run = std::env::var("XDG_RUNTIME_DIR").ok().filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            use std::os::unix::fs::MetadataExt;
            format!("/run/user/{}", std::fs::metadata("/proc/self").map(|m| m.uid()).unwrap_or(0))
        });
    PathBuf::from(run).join("hearth-steam.sock")
}

/// One answer from the service.
#[derive(Debug, Clone, PartialEq)]
pub enum Answer {
    Ok(Value),
    /// It answered no: its code (not_ready, signed_out, vac, steam_error, classic_running…) and why.
    No { code: String, error: String },
    /// No answer at all (not running, socket gone, timed out).
    Down(String),
}

impl Answer {
    pub fn from_line(line: &str) -> Answer {
        match serde_json::from_str::<Value>(line) {
            Ok(v) if v["ok"] == true => Answer::Ok(v),
            Ok(v) => Answer::No { code: v["code"].as_str().unwrap_or("error").into(), error: v["error"].as_str().unwrap_or("").into() },
            Err(e) => Answer::Down(format!("bad answer: {e}")),
        }
    }
    /// The service is there (it answered, yes or no).
    pub fn up(&self) -> bool { !matches!(self, Answer::Down(_)) }
}

pub fn ask_at(path: &Path, req: &Value, timeout: Duration) -> Answer {
    let mut s = match UnixStream::connect(path) { Ok(s) => s, Err(e) => return Answer::Down(e.to_string()) };
    let _ = s.set_read_timeout(Some(timeout));
    let _ = s.set_write_timeout(Some(Duration::from_secs(2)));
    if let Err(e) = s.write_all(format!("{req}\n").as_bytes()) { return Answer::Down(e.to_string()); }
    let mut line = String::new();
    match BufReader::new(s).read_line(&mut line) {
        Ok(n) if n > 0 => Answer::from_line(&line),
        Ok(_) => Answer::Down("closed".into()),
        Err(e) => Answer::Down(e.to_string()),
    }
}

pub fn ask(req: &Value, timeout: Duration) -> Answer { ask_at(&socket_path(), req, timeout) }

/// Settings › Library › Steam engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode { Classic, Lightweight, Open }

impl Mode {
    /// From library.json's "steamEngine": nothing chosen is Open (the default); anything unknown is Classic.
    pub fn from_setting(v: &Value) -> Mode {
        match v.as_str() { Some("lightweight") => Mode::Lightweight, Some("open") => Mode::Open, None if v.is_null() => Mode::Open, _ => Mode::Classic }
    }
    pub fn name(self) -> &'static str { match self { Mode::Classic => "classic", Mode::Lightweight => "lightweight", Mode::Open => "open" } }
    /// The service runs for this mode.
    pub fn service(self) -> bool { self != Mode::Classic }
}

/// The engine actually used: Steam's own Game Mode (the Decky session) always uses Valve's client.
pub fn effective(mode: Mode, steam_game_mode: bool) -> Mode { if steam_game_mode { Mode::Classic } else { mode } }

/// The one-time switch to the default (pure, for the tests): Some(settings to write) when the player never chose an
/// engine. The explainer flag makes Settings › Library say once what changed and how to turn it off.
pub fn migrate(stored: &serde_json::Map<String, Value>) -> Option<Vec<(&'static str, Value)>> {
    if stored.get("steamEngine").map(|v| !v.is_null()).unwrap_or(false) { return None; }
    Some(vec![("steamEngine", json!("open")), ("steamEngineExplain", json!(true))])
}

/// Which client does a Steam action.
#[derive(Debug, PartialEq)]
pub enum Route { Lightweight, Classic(&'static str) }

/// The decision, from the setting and the service's status (pure, for the tests). The service must be running as the
/// chosen engine (its status "kind": "open", or none for Lightweight).
pub fn route(mode: Mode, installed: bool, classic_running: bool, status: &Answer) -> Route {
    if mode == Mode::Classic { return Route::Classic("Classic is chosen"); }
    if !installed { return Route::Classic("the engine isn't installed"); }
    if classic_running { return Route::Classic("the normal Steam client is running"); }
    match status {
        Answer::Down(_) => Route::Classic("the engine isn't running"),
        Answer::No { .. } => Route::Classic("the engine answered with an error"),
        Answer::Ok(s) if s["kind"].as_str().unwrap_or("lightweight") != mode.name() => Route::Classic("the other engine is running"),
        Answer::Ok(s) if s["engine"] != "ready" => Route::Classic("the engine isn't ready"),
        Answer::Ok(s) if s["signedIn"] != true => Route::Classic("not signed in to the engine"),
        Answer::Ok(_) => Route::Lightweight,
    }
}

/// What happened with an action asked of the engine: done, or the normal client must do it (and the engine
/// must let go first when it's there, so two Steam clients never run at once).
#[derive(Debug, PartialEq)]
pub enum Outcome { Done(Value), Fallback { why: String, release: bool } }

pub fn outcome(answer: Answer) -> Outcome {
    match answer {
        Answer::Ok(v) => Outcome::Done(v),
        Answer::No { code, error } => Outcome::Fallback { release: code != "classic_running", why: format!("{code}: {error}") },
        Answer::Down(e) => Outcome::Fallback { why: e, release: false },
    }
}

struct State { released_at: u64, last_tick: u64 }
static STATE: Mutex<State> = Mutex::new(State { released_at: 0, last_tick: 0 });
fn now() -> u64 { crate::library::now_ms() }

/// The engine the service runs as when started next (the user manager's environment, which the unit gets).
fn choose_kind(mode: Mode) {
    let _ = std::process::Command::new("systemctl").args(["--user", "set-environment", &format!("HEARTH_STEAM_ENGINE={}", mode.name())])
        .stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).status();
}

fn systemctl(verb: &str) -> bool {
    std::process::Command::new("systemctl").args(["--user", "--no-block", verb, UNIT])
        .stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null())
        .status().map(|s| s.success()).unwrap_or(false)
}

/// Tells the engine to let go of Steam (it exits) and waits briefly until it has. Called before the normal Steam
/// client starts. Quick when the engine isn't there.
pub fn release() {
    let path = socket_path();
    if !path.exists() { return; }
    STATE.lock().unwrap_or_else(|p| p.into_inner()).released_at = now();
    let _ = ask_at(&path, &json!({"cmd": "release"}), Duration::from_secs(3));
    for _ in 0..30 { if !path.exists() { break; } std::thread::sleep(Duration::from_millis(100)); }
    crate::bridge::log("steam engine: let go for the normal Steam client");
}

/// Valve's client is about to start. The Open engine stands by (stays signed in, stops its downloads, writes nothing
/// until Valve's client has closed); the Lightweight engine (or an Open engine that doesn't answer) lets go.
pub fn step_aside() {
    let path = socket_path();
    if !path.exists() { return; }
    match ask_at(&path, &json!({"cmd": "standby"}), Duration::from_secs(7)) {
        Answer::Ok(_) => crate::bridge::log("steam engine: standing by while the normal Steam client runs"),
        _ => release(),
    }
}

/// Tries an action with the lightweight engine. Some(answer) when it did it; None when the normal client must
/// (the engine has been told to let go by then).
pub fn try_do(mode: Mode, req: Value, timeout: Duration) -> Option<Value> {
    if !mode.service() { return None; }
    let mode = effective(mode, crate::gamescope::inside() && crate::gamescope::steam_mode());
    if !mode.service() { return None; }
    let cmd = req["cmd"].as_str().unwrap_or("").to_string();
    let status = ask(&json!({"cmd": "status"}), Duration::from_secs(2));
    match route(mode, installed(), crate::steam::client_running(), &status) {
        Route::Classic(why) => {
            crate::bridge::log(&format!("steam engine: {cmd} with the normal Steam client ({why})"));
            if status.up() { if why == "the other engine is running" { release(); } else { step_aside(); } }
            None
        }
        Route::Lightweight => match outcome(ask(&req, timeout)) {
            Outcome::Done(v) => { crate::bridge::log(&format!("steam engine: {cmd} done by the lightweight engine")); Some(v) }
            Outcome::Fallback { why, release: r } => {
                crate::bridge::log(&format!("steam engine: {cmd} falls back to the normal Steam client ({why})"));
                if r { step_aside(); }
                None
            }
        },
    }
}

/// Lightweight: the engine starts the game itself (true). Open: use launch_info instead.
pub fn launch(mode: Mode, app: u32) -> bool { mode == Mode::Lightweight && try_do(mode, json!({"cmd": "launch", "appid": app}), Duration::from_secs(30)).is_some() }
pub fn install(mode: Mode, app: u32, library: &Path) -> Option<Value> {
    try_do(mode, json!({"cmd": "install", "appid": app, "library": library.to_string_lossy()}), Duration::from_secs(15))
}
pub fn uninstall(mode: Mode, app: u32) -> bool { try_do(mode, json!({"cmd": "uninstall", "appid": app}), Duration::from_secs(15)).is_some() }

/// Open: how Hearth starts a DRM-free game itself ({direct, exe, args, cwd, os, prefix, firstRun, dir}). None: start it
/// through Valve's client (Steamworks, VAC, a failed first start, or anything the engine can't answer).
pub fn launch_info(mode: Mode, app: u32) -> Option<Value> {
    if mode != Mode::Open { return None; }
    try_do(mode, json!({"cmd": "launch", "appid": app}), Duration::from_secs(12)).filter(|v| v["direct"] == true && v["exe"].is_string())
}

/// Open: how a first direct start went (remembered by the engine).
pub fn launch_result(app: u32, ok: bool) { let _ = ask(&json!({"cmd": "launch.result", "appid": app, "ok": ok}), Duration::from_secs(3)); }

/// Open: the owned games ({appid, name, …}) when the engine is ready and signed in; None otherwise (the Community
/// page / device files are used as with Classic).
pub fn owned_games() -> Option<Vec<Value>> {
    if current_mode() != Mode::Open { return None; }
    match ask(&json!({"cmd": "library"}), Duration::from_secs(5)) {
        Answer::Ok(v) if v["loading"] != true => v["apps"].as_array().cloned().filter(|a| !a.is_empty()),
        _ => None,
    }
}

/// Open: the engine's download queue ({appid, state, paused, position, done, total, speed, eta, kind, auto}), when it's
/// the engine in use and running. None otherwise.
pub fn engine_downloads() -> Option<Vec<Value>> {
    if current_mode() != Mode::Open || !socket_path().exists() { return None; }
    match ask(&json!({"cmd": "downloads"}), Duration::from_millis(900)) {
        Answer::Ok(v) => v["downloads"].as_array().cloned(),
        _ => None,
    }
}

/// The Open engine's queue merged into the Downloads page's list (pure, for the tests). Its jobs carry "engine"
/// {position, paused, eta, kind, auto} (so the page offers pause, move and cancel for them) and its speed and byte
/// counts; queued jobs without an install record yet are added from `dto` (the game's library entry).
pub fn merge_downloads(active: &mut Vec<Value>, jobs: &[Value], dto: impl Fn(&str) -> Option<Value>) {
    for j in jobs {
        let pos = j["position"].as_i64().unwrap_or(-1);
        let Some(app) = j["appid"].as_u64() else { continue };
        if pos < 0 { continue; }
        let id = format!("steam:{app}");
        let paused = j["paused"] == true;
        let i = match active.iter().position(|a| a["id"] == id.as_str()) {
            Some(i) => i,
            None => match dto(&id) {
                Some(mut d) => { d["state"] = json!(if paused { "paused" } else { "queued" }); active.push(d); active.len() - 1 }
                None => continue,
            },
        };
        let e = &mut active[i];
        e["engine"] = json!({"position": pos, "paused": paused, "eta": j["eta"], "kind": j["kind"], "auto": j["auto"] == true, "state": j["state"], "error": j["error"]});
        if paused { e["state"] = json!("paused"); e["bytesPerSec"] = json!(0); }
        else if matches!(j["state"].as_str(), Some("downloading" | "verifying" | "preparing")) {
            e["bytesPerSec"] = j["speed"].clone();
            if e["state"] == "queued" || e["state"] == "paused" { e["state"] = json!(if e["installed"] == true { "updating" } else { "downloading" }); }
        } else if e["state"] == "downloading" || e["state"] == "updating" { e["state"] = json!("queued"); e["bytesPerSec"] = json!(0); }
        if let (Some(d), Some(t)) = (j["done"].as_u64(), j["total"].as_u64()) {
            if t > 0 { e["bytesDone"] = json!(d); e["bytesTotal"] = json!(t); e["progress"] = json!((d as f64 / t as f64).min(1.0)); }
        }
    }
}

/// Open: a Downloads page control for a Steam game ("pause", "resume", "move" with "up"/"down"/"top", "cancel"; no app:
/// all). Err when the Open engine isn't the one doing it (the caller then does what it did before).
pub fn download_op(op: &str, app: Option<u32>, to: Option<&str>) -> Result<Value, String> {
    if current_mode() != Mode::Open { return Err("Hearth's Steam client isn't in use.".into()); }
    let mut req = json!({"cmd": op});
    if let Some(a) = app { req["appid"] = json!(a); }
    if let Some(t) = to { req["to"] = json!(t); }
    match ask(&req, Duration::from_secs(5)) {
        Answer::Ok(v) => Ok(v),
        Answer::No { error, .. } => Err(error),
        Answer::Down(e) => Err(e),
    }
}

/// Open: Hearth's download settings and whether a game runs, sent to the engine when they change (and once a
/// minute, so a restarted engine has them).
pub fn push_config(cfg: Value) {
    if current_mode() != Mode::Open || !socket_path().exists() { return; }
    static LAST: Mutex<(String, u64)> = Mutex::new((String::new(), 0));
    let text = cfg.to_string();
    let t = now();
    {
        let mut l = LAST.lock().unwrap_or_else(|p| p.into_inner());
        if l.0 == text && t.saturating_sub(l.1) < 60_000 { return; }
        *l = (text, t);
    }
    let mut req = cfg;
    req["cmd"] = json!("config");
    let _ = ask(&req, Duration::from_secs(2));
}

/// Settings › Library › Automatic updates: "off", "idle" (the default: when no game is running) or "always".
pub fn auto_update_setting(v: &Value) -> &'static str { match v.as_str() { Some("off") => "off", Some("always") => "always", _ => "idle" } }

/// The command line for a direct start (pure, for the tests): (program, arguments, environment). Windows games go
/// through umu (Proton) with the game's own Steam prefix, so saves are shared with Valve's client.
pub fn direct_command(info: &Value, opts: &Value, app: u32, umu: bool) -> Result<(String, Vec<String>, Vec<(String, String)>), String> {
    let exe = info["exe"].as_str().ok_or("No program.")?.to_string();
    let mut args = crate::manual::split_args(info["args"].as_str().unwrap_or(""));
    if info["os"] == "windows" {
        if !umu { return Err("umu-launcher isn't installed".into()); }
        args.insert(0, exe);
        let (c, a, mut e) = crate::manual::apply(opts, "umu-run", args, true);
        e.push(("GAMEID".into(), format!("umu-{app}")));
        e.push(("STORE".into(), "steam".into()));
        if let Some(p) = info["prefix"].as_str() { e.push(("WINEPREFIX".into(), p.into())); }
        Ok((c, a, e))
    } else {
        Ok(crate::manual::apply(opts, &exe, args, false))
    }
}

/// Starts a DRM-free game the Open engine answered for. Returns its process id.
pub fn launch_direct(info: &Value, opts: &Value, app: u32, game_id: u32) -> Result<u32, String> {
    let (c, a, mut e) = direct_command(info, opts, app, crate::system::has("umu-run"))?;
    if let Some(p) = info["prefix"].as_str().filter(|_| info["os"] == "windows") { let _ = std::fs::create_dir_all(p); }
    if game_id != 0 { e.push(crate::appid::env(game_id)); }
    let cwd = info["cwd"].as_str().unwrap_or("/");
    crate::bridge::log(&format!("steam (open): {app} started directly ({})", info["reason"].as_str().unwrap_or("")));
    let child = std::process::Command::new(&c).args(&a).envs(e).current_dir(cwd)
        .stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null())
        .spawn().map_err(|err| format!("Couldn't start it: {err}"))?;
    Ok(crate::procs::detach(child))
}

static MODE: Mutex<Mode> = Mutex::new(Mode::Classic);
/// The mode chosen in Settings (kept here for the parts of Hearth that don't see the library's settings).
pub fn current_mode() -> Mode { *MODE.lock().unwrap_or_else(|p| p.into_inner()) }
pub fn note_mode(mode: Mode) { *MODE.lock().unwrap_or_else(|p| p.into_inner()) = mode; }

/// Should the engine be started now (pure, for the tests)? The Open engine runs alongside Valve's client (standing
/// by, so the player can still sign in to it); the Lightweight engine loads Valve's steamclient, so it waits until
/// Valve's client has closed and a while after letting go.
pub fn should_start(mode: Mode, installed: bool, classic_running: bool, socket_there: bool, released_at: u64, now: u64) -> bool {
    if !mode.service() || !installed || socket_there { return false; }
    if mode == Mode::Open { return released_at == 0 || now.saturating_sub(released_at) >= OPEN_COOLDOWN_MS; }
    !classic_running && (released_at == 0 || now.saturating_sub(released_at) >= COOLDOWN_MS)
}

/// Every 30 seconds (from the library's tick): the engine runs while it's chosen (Lightweight: when the normal client
/// isn't running, again 5 minutes after it let go; Open: alongside it, standing by), and stops when Classic is
/// chosen. Returns whether the Open engine is signed in (None: not asked this time), which decides whether Valve's
/// client is kept ready (steam_launch::keep_ready_effective).
pub fn tick(mode: Mode) -> Option<bool> {
    let mode = effective(mode, crate::gamescope::inside() && crate::gamescope::steam_mode());
    note_mode(mode);
    let t = now();
    let released_at = {
        let mut s = STATE.lock().unwrap_or_else(|p| p.into_inner());
        if t.saturating_sub(s.last_tick) < TICK_MS { return None; }
        s.last_tick = t;
        s.released_at
    };
    let there = socket_path().exists();
    if !mode.service() { if there { release(); systemctl("stop"); } return Some(false); }
    if should_start(mode, installed(), crate::steam::client_running(), there, released_at, t) {
        crate::bridge::log(&format!("steam engine: starting the {} engine", mode.name()));
        choose_kind(mode);
        systemctl("start");
    }
    if mode != Mode::Open { return Some(false); }
    if !there { return None; }
    Some(open_signed_in(&ask(&json!({"cmd": "status"}), Duration::from_secs(2))))
}

/// Is the Open engine signed in, from its status (pure, for the tests)?
pub fn open_signed_in(status: &Answer) -> bool {
    matches!(status, Answer::Ok(s) if s["kind"] == "open" && s["signedIn"] == true)
}

/// Settings › Library › Steam engine changed.
pub fn set(mode: Mode) {
    note_mode(mode);
    STATE.lock().unwrap_or_else(|p| p.into_inner()).released_at = 0;
    // Whatever runs now lets go first (switching between Lightweight and Open restarts the service as the other one).
    release(); systemctl("stop");
    if mode.service() && installed() && (mode == Mode::Open || !crate::steam::client_running()) {
        // A stop that's still under way would swallow the start: wait until the socket is gone.
        for _ in 0..30 { if !socket_path().exists() { break; } std::thread::sleep(Duration::from_millis(100)); }
        choose_kind(mode);
        systemctl("start");
    }
}

/// For the Settings page: the choice, whether it's installed, and the engine's own status.
pub fn status(mode: Mode) -> Value {
    let on = mode.service();
    let st = if on || socket_path().exists() { ask(&json!({"cmd": "status"}), Duration::from_secs(2)) } else { Answer::Down("off".into()) };
    let engine = match &st { Answer::Ok(v) => v.clone(), Answer::No { code, error } => json!({"engine": "error", "code": code, "reason": error}), Answer::Down(_) => json!({"engine": "stopped"}) };
    json!({"lightweight": mode == Mode::Lightweight, "open": mode == Mode::Open, "mode": mode.name(), "installed": installed(), "classicRunning": crate::steam::client_running(), "status": engine})
}

/// Sign-in requests from Hearth's sign-in screen, handed to the service as they are. Nothing here is logged.
pub fn sign_in(a: &Value) -> Result<Value, String> {
    let req = match a["method"].as_str().unwrap_or("") {
        "password" => json!({"cmd": "signin.password", "user": a["user"], "password": a["password"], "remember": a["remember"] != false}),
        "code" => json!({"cmd": "signin.code", "code": a["code"], "kind": a["kind"].as_str().unwrap_or("device")}),
        "qr" => json!({"cmd": "signin.qr"}),
        "cancel" => json!({"cmd": "signin.cancel"}),
        "signout" => json!({"cmd": "signout"}),
        _ => return Err("Unknown sign-in step.".into()),
    };
    match ask(&req, Duration::from_secs(30)) {
        Answer::Ok(v) => Ok(v),
        Answer::No { error, .. } => Err(if error.is_empty() { "Steam said no.".into() } else { error }),
        Answer::Down(_) => Err(sign_in_down_message(installed(), crate::steam::client_running())),
    }
}

/// Why signing in can't happen when the engine doesn't answer at all (pure, for the tests): never an empty error.
pub fn sign_in_down_message(installed: bool, classic_running: bool) -> String {
    if !installed { return "Hearth's Steam engine isn't part of this build. The normal Steam client is used.".into(); }
    if classic_running { return "The normal Steam client is running, so Hearth's Steam engine has stepped aside. Sign in here once Steam has closed.".into(); }
    "Hearth's Steam engine isn't running (it stopped or didn't start). Choose \"Start the engine again\" in Settings › Library, or keep using the normal Steam client.".into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ready(signed: bool) -> Answer { Answer::Ok(json!({"ok": true, "engine": "ready", "signedIn": signed})) }

    #[test]
    fn classic_unless_everything_is_ready() {
        use Mode::*;
        assert_eq!(route(Classic, true, false, &ready(true)), Route::Classic("Classic is chosen"));
        assert_eq!(route(Lightweight, false, false, &ready(true)), Route::Classic("the engine isn't installed"));
        assert_eq!(route(Lightweight, true, true, &ready(true)), Route::Classic("the normal Steam client is running"));
        assert_eq!(route(Lightweight, true, false, &Answer::Down("x".into())), Route::Classic("the engine isn't running"));
        assert_eq!(route(Lightweight, true, false, &Answer::No { code: "not_ready".into(), error: String::new() }), Route::Classic("the engine answered with an error"));
        assert_eq!(route(Lightweight, true, false, &Answer::Ok(json!({"engine": "bootstrapping"}))), Route::Classic("the engine isn't ready"));
        assert_eq!(route(Lightweight, true, false, &ready(false)), Route::Classic("not signed in to the engine"));
        assert_eq!(route(Lightweight, true, false, &ready(true)), Route::Lightweight);
        // Open: the service must be running as the Open engine, and the same rules apply.
        let open = |signed: bool| Answer::Ok(json!({"ok": true, "kind": "open", "engine": "ready", "signedIn": signed}));
        assert_eq!(route(Open, true, false, &open(true)), Route::Lightweight);
        assert_eq!(route(Open, true, false, &ready(true)), Route::Classic("the other engine is running"));
        assert_eq!(route(Lightweight, true, false, &open(true)), Route::Classic("the other engine is running"));
        assert_eq!(route(Open, true, false, &open(false)), Route::Classic("not signed in to the engine"));
        assert_eq!(route(Open, true, true, &open(true)), Route::Classic("the normal Steam client is running"));
        assert_eq!(route(Open, false, false, &open(true)), Route::Classic("the engine isn't installed"));
        assert_eq!(route(Open, true, false, &Answer::Down("x".into())), Route::Classic("the engine isn't running"));
    }

    #[test]
    fn modes_from_the_setting() {
        assert_eq!(Mode::from_setting(&json!("open")), Mode::Open);
        assert_eq!(Mode::from_setting(&json!("lightweight")), Mode::Lightweight);
        assert_eq!(Mode::from_setting(&json!("classic")), Mode::Classic);
        assert_eq!(Mode::from_setting(&json!(null)), Mode::Open);   // the default now
        assert_eq!(Mode::from_setting(&json!("something")), Mode::Classic);
        assert!(!Mode::Classic.service() && Mode::Open.service() && Mode::Lightweight.service());
        // Classic never asks the engine anything.
        assert_eq!(try_do(Mode::Classic, json!({"cmd": "launch", "appid": 1}), Duration::from_millis(10)), None);
        assert_eq!(launch_info(Mode::Lightweight, 1), None);
        assert!(!launch(Mode::Open, 1));
    }

    #[test]
    fn direct_starts() {
        let win = json!({"direct": true, "exe": "/lib/steamapps/common/G/g.exe", "args": "-a \"b c\"", "os": "windows", "prefix": "/lib/steamapps/compatdata/70/pfx"});
        let (c, a, e) = direct_command(&win, &json!({"proton": "GE-Proton"}), 70, true).unwrap();
        assert_eq!(c, "umu-run");
        assert_eq!(a, vec!["/lib/steamapps/common/G/g.exe", "-a", "b c"]);
        assert!(e.contains(&("GAMEID".into(), "umu-70".into())) && e.contains(&("STORE".into(), "steam".into())));
        assert!(e.contains(&("WINEPREFIX".into(), "/lib/steamapps/compatdata/70/pfx".into())) && e.contains(&("PROTONPATH".into(), "GE-Proton".into())));
        assert!(direct_command(&win, &json!({}), 70, false).is_err());   // no umu: Valve's client starts it
        let lin = json!({"direct": true, "exe": "/g/run.sh", "args": "", "os": "linux"});
        let (c, a, e) = direct_command(&lin, &json!({}), 70, false).unwrap();
        assert_eq!(c, "/g/run.sh");
        assert!(a.is_empty());
        assert!(!e.iter().any(|(k, _)| k == "WINEPREFIX" || k == "PROTONPATH"));
    }

    #[test]
    fn every_failure_falls_back() {
        assert_eq!(outcome(Answer::Ok(json!({"ok": true}))), Outcome::Done(json!({"ok": true})));
        for code in ["vac", "steam_error", "not_ready", "signed_out", "unsupported", "busy"] {
            match outcome(Answer::No { code: code.into(), error: "e".into() }) {
                Outcome::Fallback { release, .. } => assert!(release, "{code} should let go for the normal client"),
                o => panic!("{code}: {o:?}"),
            }
        }
        // The normal client is already running: nothing to let go of. Not running at all: nothing either.
        assert_eq!(outcome(Answer::No { code: "classic_running".into(), error: "e".into() }), Outcome::Fallback { why: "classic_running: e".into(), release: false });
        assert!(matches!(outcome(Answer::Down("gone".into())), Outcome::Fallback { release: false, .. }));
    }

    #[test]
    fn sign_in_errors_are_never_empty() {
        for (i, c) in [(false, false), (true, true), (true, false)] {
            let m = sign_in_down_message(i, c);
            assert!(m.len() > 20 && m.contains("Steam"), "{m}");
        }
        assert!(sign_in_down_message(true, false).contains("Start the engine again"));
        assert!(sign_in_down_message(true, true).contains("normal Steam client is running"));
    }

    #[test]
    fn the_open_engines_queue_on_the_downloads_page() {
        let mut active = vec![json!({"id": "steam:10", "state": "downloading", "installed": false, "bytesPerSec": 0})];
        let jobs = vec![
            json!({"appid": 10, "state": "downloading", "paused": false, "position": 0, "done": 50, "total": 200, "speed": 7000, "eta": 1, "kind": "install"}),
            json!({"appid": 20, "state": "queued", "paused": true, "position": 1, "done": 0, "total": 0, "speed": 0, "kind": "auto", "auto": true}),
            json!({"appid": 30, "state": "queued", "paused": false, "position": 2, "kind": "update"}),
            json!({"appid": 40, "state": "done", "position": -1}),
            json!({"appid": 50, "state": "queued", "position": 3}),
        ];
        merge_downloads(&mut active, &jobs, |id| match id {
            "steam:20" | "steam:30" | "steam:40" => Some(json!({"id": id, "state": "installed", "installed": true})),
            _ => None,   // not in the library: left out
        });
        assert_eq!(active.len(), 3);
        assert_eq!(active[0]["engine"]["position"], 0);
        assert_eq!(active[0]["bytesPerSec"], 7000);
        assert_eq!(active[0]["bytesTotal"], 200);
        assert_eq!(active[0]["progress"], 0.25);
        let b = active.iter().find(|a| a["id"] == "steam:20").unwrap();
        assert_eq!(b["state"], "paused");
        assert_eq!(b["engine"]["auto"], true);
        let c = active.iter().find(|a| a["id"] == "steam:30").unwrap();
        assert_eq!(c["state"], "queued");
        assert!(!active.iter().any(|a| a["id"] == "steam:40"), "finished jobs aren't downloads");
        // Classic: never asked, nothing merged.
        assert!(engine_downloads().is_none() || current_mode() == Mode::Open);
        assert!(download_op("pause", None, None).is_err() || current_mode() == Mode::Open);
        assert_eq!(auto_update_setting(&json!(null)), "idle");
        assert_eq!(auto_update_setting(&json!("off")), "off");
        assert_eq!(auto_update_setting(&json!("always")), "always");
        assert_eq!(auto_update_setting(&json!("weird")), "idle");
    }

    #[test]
    fn reads_answers() {
        assert_eq!(Answer::from_line("{\"ok\":true,\"id\":1}\n"), Answer::Ok(json!({"ok": true, "id": 1})));
        assert_eq!(Answer::from_line("{\"ok\":false,\"code\":\"vac\",\"error\":\"VAC\"}"), Answer::No { code: "vac".into(), error: "VAC".into() });
        assert!(matches!(Answer::from_line("garbage"), Answer::Down(_)));
        assert!(Answer::No { code: "x".into(), error: String::new() }.up());
        assert!(!Answer::Down(String::new()).up());
    }

    #[test]
    fn starts_only_when_it_should() {
        use Mode::*;
        let t = 10 * 60_000;
        assert!(should_start(Lightweight, true, false, false, 0, t));
        assert!(!should_start(Classic, true, false, false, 0, t));
        assert!(!should_start(Lightweight, false, false, false, 0, t));
        assert!(!should_start(Lightweight, true, true, false, 0, t));
        assert!(!should_start(Lightweight, true, false, true, 0, t));
        assert!(!should_start(Lightweight, true, false, false, t - 60_000, t));
        assert!(should_start(Lightweight, true, false, false, t - COOLDOWN_MS, t));
        // Open runs alongside Valve's client (standing by), so the player can sign in whenever.
        assert!(should_start(Open, true, true, false, 0, t));
        assert!(!should_start(Open, false, false, false, 0, t));
        assert!(!should_start(Open, true, false, true, 0, t));
        assert!(!should_start(Open, true, false, false, t - 10_000, t));
        assert!(should_start(Open, true, true, false, t - OPEN_COOLDOWN_MS, t));
    }

    #[test]
    fn open_is_the_default_once_and_can_be_turned_off() {
        let mut m = serde_json::Map::new();
        let w = migrate(&m).expect("never chosen: switched once");
        assert!(w.contains(&("steamEngine", json!("open"))) && w.contains(&("steamEngineExplain", json!(true))));
        for (k, v) in w { m.insert(k.into(), v); }
        assert_eq!(migrate(&m), None, "only once");
        assert_eq!(Mode::from_setting(&m["steamEngine"]), Mode::Open);
        // Chosen before (Classic, Lightweight or Open): left alone; turning it off sticks.
        for e in ["classic", "lightweight", "open"] {
            let mut c = serde_json::Map::new();
            c.insert("steamEngine".into(), json!(e));
            assert_eq!(migrate(&c), None, "{e}");
        }
        m.insert("steamEngine".into(), json!("classic"));
        assert_eq!(migrate(&m), None);
        assert_eq!(Mode::from_setting(&m["steamEngine"]), Mode::Classic);
        // Steam's own Game Mode (Decky) uses Valve's client whatever is chosen.
        assert_eq!(effective(Mode::Open, true), Mode::Classic);
        assert_eq!(effective(Mode::Lightweight, true), Mode::Classic);
        assert_eq!(effective(Mode::Open, false), Mode::Open);
    }

    #[test]
    fn signed_in_only_from_the_open_engine() {
        assert!(open_signed_in(&Answer::Ok(json!({"kind": "open", "signedIn": true, "engine": "ready"}))));
        assert!(!open_signed_in(&Answer::Ok(json!({"kind": "open", "signedIn": false}))));
        assert!(!open_signed_in(&Answer::Ok(json!({"signedIn": true}))), "the Lightweight engine");
        assert!(!open_signed_in(&Answer::Down("x".into())));
        assert!(!open_signed_in(&Answer::No { code: "not_ready".into(), error: String::new() }));
        // Not signed in: everything goes to Valve's client, exactly as with Classic.
        let st = Answer::Ok(json!({"kind": "open", "engine": "ready", "signedIn": false}));
        assert_eq!(route(Mode::Open, true, false, &st), Route::Classic("not signed in to the engine"));
    }

    #[test]
    fn talks_over_the_socket() {
        let dir = std::env::temp_dir().join(format!("hearth-steam-engine-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("s.sock");
        let _ = std::fs::remove_file(&path);
        let l = std::os::unix::net::UnixListener::bind(&path).unwrap();
        let t = std::thread::spawn(move || {
            let (s, _) = l.accept().unwrap();
            let mut r = BufReader::new(s.try_clone().unwrap());
            let mut line = String::new();
            r.read_line(&mut line).unwrap();
            let v: Value = serde_json::from_str(&line).unwrap();
            let mut s = s;
            let answer = if v["cmd"] == "launch" { json!({"ok": false, "code": "vac", "error": "VAC"}) } else { json!({"ok": true}) };
            s.write_all(format!("{answer}\n").as_bytes()).unwrap();
        });
        let a = ask_at(&path, &json!({"cmd": "launch", "appid": 730}), Duration::from_secs(5));
        t.join().unwrap();
        assert_eq!(a, Answer::No { code: "vac".into(), error: "VAC".into() });
        assert!(matches!(outcome(a), Outcome::Fallback { release: true, .. }));
        let _ = std::fs::remove_file(&path);
        assert!(matches!(ask_at(&path, &json!({"cmd": "status"}), Duration::from_secs(1)), Answer::Down(_)));
    }
}
