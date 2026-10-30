//! Settings › Library › Steam engine: Classic (the normal Steam client, the default) or Lightweight (experimental):
//! hosts/hearth-steam, a headless service that loads Valve's own steamclient.so with no Steam UI and answers on a
//! local socket. Lightweight never stands between the player and a game: whenever it can't do something (not
//! installed, not running, still starting, signed out, a VAC game, any error, the normal client already running),
//! Hearth tells it to let go and does the same thing with the normal Steam client, exactly as with Classic.
//!
//! Hearth never stores or logs what's typed to sign in: it's handed to the service, which hands it to steamclient.
//!
//! Open (experimental, off by default) is the same service started with HEARTH_STEAM_ENGINE=open: Hearth's own
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
    /// From library.json's "steamEngine" (anything unknown is Classic).
    pub fn from_setting(v: &Value) -> Mode {
        match v.as_str() { Some("lightweight") => Mode::Lightweight, Some("open") => Mode::Open, _ => Mode::Classic }
    }
    pub fn name(self) -> &'static str { match self { Mode::Classic => "classic", Mode::Lightweight => "lightweight", Mode::Open => "open" } }
    /// The service runs for this mode.
    pub fn service(self) -> bool { self != Mode::Classic }
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

/// Tries an action with the lightweight engine. Some(answer) when it did it; None when the normal client must
/// (the engine has been told to let go by then).
pub fn try_do(mode: Mode, req: Value, timeout: Duration) -> Option<Value> {
    if !mode.service() { return None; }
    let cmd = req["cmd"].as_str().unwrap_or("").to_string();
    let status = ask(&json!({"cmd": "status"}), Duration::from_secs(2));
    match route(mode, installed(), crate::steam::client_running(), &status) {
        Route::Classic(why) => {
            crate::bridge::log(&format!("steam engine: {cmd} with the normal Steam client ({why})"));
            if status.up() { release(); }
            None
        }
        Route::Lightweight => match outcome(ask(&req, timeout)) {
            Outcome::Done(v) => { crate::bridge::log(&format!("steam engine: {cmd} done by the lightweight engine")); Some(v) }
            Outcome::Fallback { why, release: r } => {
                crate::bridge::log(&format!("steam engine: {cmd} falls back to the normal Steam client ({why})"));
                if r { release(); }
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

/// Should the engine be started now (pure, for the tests)?
pub fn should_start(on: bool, installed: bool, classic_running: bool, socket_there: bool, released_at: u64, now: u64) -> bool {
    on && installed && !classic_running && !socket_there && (released_at == 0 || now.saturating_sub(released_at) >= COOLDOWN_MS)
}

/// Every 30 seconds (from the library's tick): the engine runs while Lightweight is on and the normal client
/// isn't running (again 5 minutes after it let go), and stops when Classic is chosen.
pub fn tick(mode: Mode) {
    note_mode(mode);
    let on = mode.service();
    let t = now();
    let released_at = {
        let mut s = STATE.lock().unwrap_or_else(|p| p.into_inner());
        if t.saturating_sub(s.last_tick) < TICK_MS { return; }
        s.last_tick = t;
        s.released_at
    };
    let there = socket_path().exists();
    if !on { if there { release(); systemctl("stop"); } return; }
    if should_start(on, installed(), crate::steam::client_running(), there, released_at, t) {
        crate::bridge::log(&format!("steam engine: starting the {} engine", mode.name()));
        choose_kind(mode);
        systemctl("start");
    }
}

/// Settings › Library › Steam engine changed.
pub fn set(mode: Mode) {
    note_mode(mode);
    STATE.lock().unwrap_or_else(|p| p.into_inner()).released_at = 0;
    // Whatever runs now lets go first (switching between Lightweight and Open restarts the service as the other one).
    release(); systemctl("stop");
    if mode.service() && installed() && !crate::steam::client_running() {
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
        Answer::Down(_) => Err("The lightweight Steam engine isn't running. Turn it on in Settings › Library, or use the normal Steam client.".into()),
    }
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
        assert_eq!(Mode::from_setting(&json!(null)), Mode::Classic);   // off by default
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
    fn reads_answers() {
        assert_eq!(Answer::from_line("{\"ok\":true,\"id\":1}\n"), Answer::Ok(json!({"ok": true, "id": 1})));
        assert_eq!(Answer::from_line("{\"ok\":false,\"code\":\"vac\",\"error\":\"VAC\"}"), Answer::No { code: "vac".into(), error: "VAC".into() });
        assert!(matches!(Answer::from_line("garbage"), Answer::Down(_)));
        assert!(Answer::No { code: "x".into(), error: String::new() }.up());
        assert!(!Answer::Down(String::new()).up());
    }

    #[test]
    fn starts_only_when_it_should() {
        let t = 10 * 60_000;
        assert!(should_start(true, true, false, false, 0, t));
        assert!(!should_start(false, true, false, false, 0, t));
        assert!(!should_start(true, false, false, false, 0, t));
        assert!(!should_start(true, true, true, false, 0, t));
        assert!(!should_start(true, true, false, true, 0, t));
        assert!(!should_start(true, true, false, false, t - 60_000, t));
        assert!(should_start(true, true, false, false, t - COOLDOWN_MS, t));
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
