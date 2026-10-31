//! Discord: "Playing X" while a game from Hearth's library runs (Discord's Rich Presence, over its local IPC socket),
//! and Discord in Quick Access.
//!
//! Rich Presence needs a Discord application ID (the owner makes one at discord.com/developers/applications). Hearth
//! never ships one hardcoded; it's read from, in order:
//!   1. HEARTH_DISCORD_APP_ID in the environment (testing),
//!   2. /etc/hearth/discord.conf, a line `app_id=<the application ID>` (# starts a comment),
//!   3. HEARTH_DISCORD_APP_ID set when Hearth is built (option_env!; CI can pass it from a repository variable).
//! Without one the feature stays hidden (`status().available` is false) and nothing talks to Discord.
//!
//! You can turn it off in Quick Access (~/.config/nimbus/discord.json {"enabled": false}). Discord itself shows the
//! application's name as what's played ("Playing Hearth OS"), with the game's title under it and the time since it
//! started. Discord installed as a Flatpak or a package both work (its socket is looked for in both places).
//! All talking to Discord happens on its own thread: a missing or slow Discord never holds Hearth up.

use serde_json::{json, Value};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, RecvTimeoutError, Sender};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

pub const CONF: &str = "/etc/hearth/discord.conf";
const MAX_FRAME: usize = 64 * 1024;

fn log(s: &str) { crate::bridge::log(&format!("discord: {s}")); }

/// An application ID as Discord makes them (a snowflake: 15 to 21 digits).
pub fn valid_id(s: &str) -> bool { (15..=21).contains(&s.len()) && s.chars().all(|c| c.is_ascii_digit()) }

/// `app_id=…` from a discord.conf's text.
pub fn parse_conf(text: &str) -> Option<String> {
    for line in text.lines() {
        let l = line.split('#').next().unwrap_or("").trim();
        if let Some((k, v)) = l.split_once('=') {
            if k.trim() == "app_id" {
                let v = v.trim().trim_matches('"').trim_matches('\'');
                if valid_id(v) { return Some(v.to_string()); }
            }
        }
    }
    None
}

/// The application ID, if the owner has set one (see the top of this file).
pub fn app_id() -> Option<String> {
    if let Ok(v) = std::env::var("HEARTH_DISCORD_APP_ID") { if valid_id(v.trim()) { return Some(v.trim().to_string()); } }
    if let Some(v) = std::fs::read_to_string(CONF).ok().as_deref().and_then(parse_conf) { return Some(v); }
    option_env!("HEARTH_DISCORD_APP_ID").map(str::trim).filter(|v| valid_id(v)).map(str::to_string)
}

fn settings_path() -> PathBuf { crate::paths::home().join(".config/nimbus/discord.json") }
pub fn enabled() -> bool {
    std::fs::read(settings_path()).ok().and_then(|b| serde_json::from_slice::<Value>(&b).ok()).map(|v| v["enabled"] != false).unwrap_or(true)
}
pub fn set_enabled(on: bool) -> Result<Value, String> {
    let p = settings_path();
    if let Some(d) = p.parent() { std::fs::create_dir_all(d).map_err(|e| e.to_string())?; }
    std::fs::write(&p, serde_json::to_vec(&json!({"enabled": on})).unwrap_or_default()).map_err(|e| e.to_string())?;
    if !on { send(Msg::Clear); } else if let Some(p) = PLAYING.lock().ok().and_then(|g| g.clone()) { send(Msg::Play(p.0, p.1)); }
    Ok(status())
}

fn installed() -> bool {
    crate::matrix_friends::discord_installed() || ["discord", "Discord", "discord-canary", "vesktop"].iter().any(|b| crate::system::has(b))
}

static CONNECTED: AtomicBool = AtomicBool::new(false);
static PLAYING: Mutex<Option<(String, u64)>> = Mutex::new(None);

/// What Quick Access shows: {available, enabled, installed, connected, playing}.
pub fn status() -> Value {
    let available = app_id().is_some();
    let playing = PLAYING.lock().ok().and_then(|g| g.as_ref().map(|p| p.0.clone()));
    json!({"available": available, "enabled": enabled(), "installed": available && installed(),
           "connected": available && CONNECTED.load(Ordering::SeqCst), "playing": playing})
}

// ------------------------------------------------------------------ the socket

/// Where Discord's IPC socket can be: $XDG_RUNTIME_DIR (or TMPDIR, /tmp), and inside Flatpak's and Snap's folders.
pub fn socket_candidates(runtime: &Path) -> Vec<PathBuf> {
    let mut out = vec![];
    for sub in ["", "app/com.discordapp.Discord", "app/com.discordapp.DiscordCanary", "app/dev.vencord.Vesktop", "snap.discord", ".flatpak/com.discordapp.Discord/xdg-run"] {
        for i in 0..10 { out.push(runtime.join(sub).join(format!("discord-ipc-{i}"))); }
    }
    out
}

fn runtimes() -> Vec<PathBuf> {
    let mut v = vec![];
    if let Some(r) = std::env::var_os("XDG_RUNTIME_DIR") { v.push(PathBuf::from(r)); }
    if let Some(t) = std::env::var_os("TMPDIR") { v.push(PathBuf::from(t)); }
    v.push(PathBuf::from("/tmp"));
    v
}

/// One IPC frame: opcode and length (little-endian u32s), then the JSON.
pub fn frame(op: u32, payload: &Value) -> Vec<u8> {
    let body = serde_json::to_vec(payload).unwrap_or_default();
    let mut out = Vec::with_capacity(8 + body.len());
    out.extend_from_slice(&op.to_le_bytes());
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(&body);
    out
}

pub fn read_frame(r: &mut impl Read) -> Result<(u32, Value), String> {
    let mut head = [0u8; 8];
    r.read_exact(&mut head).map_err(|e| e.to_string())?;
    let op = u32::from_le_bytes([head[0], head[1], head[2], head[3]]);
    let len = u32::from_le_bytes([head[4], head[5], head[6], head[7]]) as usize;
    if len > MAX_FRAME { return Err(format!("a frame of {len} bytes")); }
    let mut body = vec![0u8; len];
    r.read_exact(&mut body).map_err(|e| e.to_string())?;
    Ok((op, serde_json::from_slice(&body).unwrap_or(Value::Null)))
}

/// The activity Discord shows for a game.
pub fn activity(title: &str, start_secs: u64) -> Value {
    let t: String = title.chars().take(120).collect();
    json!({"details": t, "state": "on Hearth OS", "timestamps": {"start": start_secs}, "assets": {"large_image": "hearth", "large_text": "Hearth OS"}, "instance": false})
}

fn nonce() -> String { let mut b = [0u8; 8]; let _ = getrandom::getrandom(&mut b); b.iter().map(|x| format!("{x:02x}")).collect() }

#[cfg(unix)]
fn connect(id: &str) -> Result<std::os::unix::net::UnixStream, String> { connect_in(&runtimes(), id) }

#[cfg(unix)]
fn connect_in(dirs: &[PathBuf], id: &str) -> Result<std::os::unix::net::UnixStream, String> {
    use std::os::unix::net::UnixStream;
    for rt in dirs {
        for p in socket_candidates(rt) {
            if !p.exists() { continue; }
            let Ok(mut s) = UnixStream::connect(&p) else { continue };
            let _ = s.set_read_timeout(Some(Duration::from_secs(5)));
            let _ = s.set_write_timeout(Some(Duration::from_secs(5)));
            if s.write_all(&frame(0, &json!({"v": 1, "client_id": id}))).is_err() { continue; }
            match read_frame(&mut s) {
                Ok((1, v)) if v["evt"] == "READY" => { log(&format!("connected ({})", p.display())); return Ok(s); }
                Ok((op, v)) => { log(&format!("{} refused the handshake (op {op}): {}", p.display(), v["message"].as_str().unwrap_or(""))); }
                Err(e) => log(&format!("{}: {e}", p.display())),
            }
        }
    }
    Err("Discord isn't running".into())
}

enum Msg { Play(String, u64), Clear }

static TX: OnceLock<Mutex<Sender<Msg>>> = OnceLock::new();

fn send(m: Msg) {
    let tx = TX.get_or_init(|| {
        let (tx, rx) = channel::<Msg>();
        std::thread::Builder::new().name("discord".into()).spawn(move || worker(rx)).ok();
        Mutex::new(tx)
    });
    if let Ok(t) = tx.lock() { let _ = t.send(m); }
}

#[cfg(unix)]
fn worker(rx: std::sync::mpsc::Receiver<Msg>) {
    let mut want: Option<(String, u64)> = None;
    let mut sock: Option<std::os::unix::net::UnixStream> = None;
    let mut shown = false;   // what Discord has is `want`
    loop {
        match rx.recv_timeout(Duration::from_secs(20)) {
            Ok(Msg::Play(t, s)) => { want = Some((t, s)); shown = false; }
            Ok(Msg::Clear) => { want = None; shown = false; }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        let Some(id) = app_id() else { sock = None; CONNECTED.store(false, Ordering::SeqCst); continue };
        if shown { continue; }
        if want.is_none() && sock.is_none() { shown = true; continue; }   // nothing to clear
        if sock.is_none() {
            match connect(&id) { Ok(s) => sock = Some(s), Err(_) => { CONNECTED.store(false, Ordering::SeqCst); continue; } }
        }
        let act = want.as_ref().map(|(t, s)| activity(t, *s)).unwrap_or(Value::Null);
        let msg = json!({"cmd": "SET_ACTIVITY", "args": {"pid": std::process::id(), "activity": act}, "nonce": nonce()});
        let s = sock.as_mut().unwrap();
        let ok = s.write_all(&frame(1, &msg)).is_ok() && read_frame(s).map(|(op, v)| { if v["evt"] == "ERROR" { log(&format!("Discord said: {}", v["data"]["message"].as_str().unwrap_or("?"))); } op != 2 }).unwrap_or(false);
        CONNECTED.store(ok, Ordering::SeqCst);
        if ok { shown = true; } else { sock = None; }
        // Nothing playing any more: let go of Discord.
        if ok && want.is_none() { sock = None; }
    }
}

#[cfg(not(unix))]
fn worker(rx: std::sync::mpsc::Receiver<Msg>) { while rx.recv().is_ok() {} }

/// A game from the library started (Some(title)) or ended (None). Does nothing without an application ID or when
/// it's turned off.
pub fn set_playing(title: Option<&str>) {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    if let Ok(mut g) = PLAYING.lock() { *g = title.map(|t| (t.to_string(), now)); }
    if app_id().is_none() { return; }
    match title {
        Some(t) if enabled() => send(Msg::Play(t.to_string(), now)),
        _ => send(Msg::Clear),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_and_config() {
        assert!(valid_id("123456789012345678"));
        assert!(!valid_id("12345") && !valid_id("12345678901234567x") && !valid_id(""));
        assert_eq!(parse_conf("# Discord\napp_id = 123456789012345678\n"), Some("123456789012345678".into()));
        assert_eq!(parse_conf("app_id=\"123456789012345678\" # quoted"), Some("123456789012345678".into()));
        assert_eq!(parse_conf("app_id=\n"), None);
        assert_eq!(parse_conf("# app_id=123456789012345678"), None);
        assert_eq!(parse_conf("app_id=not-a-number"), None);
    }

    #[test]
    fn frames_round_trip() {
        let f = frame(1, &json!({"cmd": "SET_ACTIVITY"}));
        assert_eq!(&f[..4], &1u32.to_le_bytes());
        let (op, v) = read_frame(&mut std::io::Cursor::new(f)).unwrap();
        assert_eq!(op, 1);
        assert_eq!(v["cmd"], "SET_ACTIVITY");
        let mut big = 1u32.to_le_bytes().to_vec();
        big.extend_from_slice(&(10u32 << 20).to_le_bytes());
        assert!(read_frame(&mut std::io::Cursor::new(big)).is_err());
    }

    #[test]
    fn sockets_and_activity() {
        let c = socket_candidates(Path::new("/run/user/1000"));
        assert!(c.contains(&PathBuf::from("/run/user/1000/discord-ipc-0")));
        assert!(c.contains(&PathBuf::from("/run/user/1000/app/com.discordapp.Discord/discord-ipc-0")));
        let a = activity("Hollow Knight", 1700000000);
        assert_eq!(a["details"], "Hollow Knight");
        assert_eq!(a["timestamps"]["start"], 1700000000u64);
    }

    #[cfg(unix)]
    #[test]
    fn talks_to_a_fake_discord() {
        use std::os::unix::net::UnixListener;
        let dir = std::env::temp_dir().join(format!("hearth-discord-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("discord-ipc-0");
        let _ = std::fs::remove_file(&path);
        let l = UnixListener::bind(&path).unwrap();
        let server = std::thread::spawn(move || {
            let (mut s, _) = l.accept().unwrap();
            let (op, hello) = read_frame(&mut s).unwrap();
            assert_eq!(op, 0);
            assert_eq!(hello["client_id"], "123456789012345678");
            s.write_all(&frame(1, &json!({"cmd": "DISPATCH", "evt": "READY"}))).unwrap();
            let (op, set) = read_frame(&mut s).unwrap();
            assert_eq!(op, 1);
            s.write_all(&frame(1, &json!({"cmd": "SET_ACTIVITY", "evt": null}))).unwrap();
            set
        });
        let mut s = connect_in(&[dir.clone()], "123456789012345678").expect("connects to the fake Discord");
        let msg = json!({"cmd": "SET_ACTIVITY", "args": {"pid": 1, "activity": activity("Celeste", 5)}, "nonce": nonce()});
        s.write_all(&frame(1, &msg)).unwrap();
        read_frame(&mut s).unwrap();
        let got = server.join().unwrap();
        assert_eq!(got["args"]["activity"]["details"], "Celeste");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
