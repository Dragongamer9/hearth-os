//! Discord (docs/DISCORD.md): "Playing X" while a game from Hearth's library runs (Discord's Rich Presence, over the
//! Discord app's local IPC socket), the "Ask to join" / "Join" buttons Discord shows friends for it (a party and a join
//! secret), what happens when a friend's Discord says they joined (ACTIVITY_JOIN), and Discord in Quick Access.
//! Since 1.6 this is how Hearth does friends: Hearth Friends (Matrix) is archived (docs/archive/FRIENDS.md).
//!
//! Only Discord's official, local mechanisms: the desktop app's IPC socket (SET_ACTIVITY, SUBSCRIBE ACTIVITY_JOIN) and
//! discord-<app id>:// links. Hearth never asks for, reads or stores a Discord token or password, and never reads or
//! sends messages or the friends list.
//!
//! Rich Presence needs a Discord application ID (the owner makes one at discord.com/developers/applications). Hearth
//! never ships one hardcoded; it's read from, in order:
//!   1. HEARTH_DISCORD_APP_ID in the environment (testing),
//!   2. ~/.config/nimbus/discord.json {"appId": "..."} (Settings › Gaming › Discord: paste it, one step),
//!   3. /etc/hearth/discord.conf, a line `app_id=<the application ID>` (# starts a comment),
//!   4. HEARTH_DISCORD_APP_ID set when Hearth is built (option_env!; CI can pass it from a repository variable).
//! Without one, Rich Presence stays off and nothing talks to Discord (Discord itself still opens, from the bar).
//!
//! discord.json is shared with Mantel's helper (desktop/shell/hearth-discord) and only ever merged one key at a time
//! (settings_file.rs): `enabled` (Rich Presence, default on), `askToJoin` (the join buttons, default on), `appId`,
//! `client` (which Discord the bar opens). Defaults are only read, never written.
//!
//! Play together over Discord: while a game runs, Sunshine is on (Settings › Remote play; Hearth never turns it on by
//! itself here) and "Let friends ask to join" is on, the join secret also carries this device's address (Tailscale's
//! when it has one, else the local network's) and a one-time 4-digit PIN that expires in 10 minutes. Discord only
//! hands the secret to friends you invited or let join. The friend's Hearth pairs Moonlight with that PIN; this side
//! enters it in Sunshine while it's valid, then makes a new one. The PIN shows on both screens.
//! All talking to Discord happens on its own thread: a missing or slow Discord never holds Hearth up.

use serde_json::{json, Map, Value};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{channel, RecvTimeoutError, Sender};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

pub const CONF: &str = "/etc/hearth/discord.conf";
pub const FLATPAK: &str = "com.discordapp.Discord";
const MAX_FRAME: usize = 64 * 1024;
/// Discord's limit for a join secret.
pub const SECRET_MAX: usize = 128;
/// How long a Play together PIN in a join secret lasts.
pub const PIN_SECS: u64 = 600;

fn log(s: &str) { crate::bridge::log(&format!("discord: {s}")); }
fn now_secs() -> u64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) }

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

// ------------------------------------------------------------------ settings (discord.json, merged one key at a time)

fn settings_path() -> PathBuf {
    if let Some(p) = std::env::var_os("HEARTH_DISCORD_SETTINGS") { return PathBuf::from(p); }
    crate::paths::home().join(".config/nimbus/discord.json")
}
fn settings() -> Value { crate::settings_file::read_json(&settings_path()) }
fn merge(key: &str, v: Value) -> Result<(), String> {
    let mut m = Map::new();
    m.insert(key.into(), v);
    crate::settings_file::merge_json(&settings_path(), &m).map(|_| ()).map_err(|e| e.to_string())
}

/// The application ID, if the owner has set one (see the top of this file).
pub fn app_id() -> Option<String> {
    if let Ok(v) = std::env::var("HEARTH_DISCORD_APP_ID") { if valid_id(v.trim()) { return Some(v.trim().to_string()); } }
    if let Some(v) = settings()["appId"].as_str().map(str::trim).filter(|v| valid_id(v)) { return Some(v.to_string()); }
    if let Some(v) = std::fs::read_to_string(CONF).ok().as_deref().and_then(parse_conf) { return Some(v); }
    option_env!("HEARTH_DISCORD_APP_ID").map(str::trim).filter(|v| valid_id(v)).map(str::to_string)
}

pub fn enabled() -> bool { settings()["enabled"] != false }
pub fn ask_to_join() -> bool { settings()["askToJoin"] != false }

pub fn set_enabled(on: bool) -> Result<Value, String> {
    merge("enabled", json!(on))?;
    send(Msg::Refresh);
    Ok(status())
}
pub fn set_ask_to_join(on: bool) -> Result<Value, String> {
    merge("askToJoin", json!(on))?;
    send(Msg::Refresh);
    Ok(status())
}
/// Settings › Discord's one step: the application ID pasted (an empty one takes it out again).
pub fn set_app_id(id: &str) -> Result<Value, String> {
    let id = id.trim();
    if !id.is_empty() && !valid_id(id) { return Err("That isn't a Discord application ID (15 to 21 digits, from discord.com/developers/applications).".into()); }
    merge("appId", if id.is_empty() { Value::Null } else { json!(id) })?;
    send(Msg::Refresh);
    Ok(status())
}

pub fn flatpak_installed() -> bool { crate::system::has("flatpak") && crate::system::run("flatpak", &["info", FLATPAK]).is_ok() }
fn installed() -> bool {
    flatpak_installed() || ["discord", "Discord", "discord-canary", "vesktop", "dorion"].iter().any(|b| crate::system::has(b))
}

static CONNECTED: AtomicBool = AtomicBool::new(false);
static PLAYING: Mutex<Option<Playing>> = Mutex::new(None);
static INVITE: Mutex<Option<Invite>> = Mutex::new(None);

/// What Quick Access and Settings show: {available (an app ID is set), enabled, askToJoin, installed, connected,
/// playing, invite: {pin, expires, playTogether} while friends can join}.
pub fn status() -> Value {
    let available = app_id().is_some();
    let playing = PLAYING.lock().ok().and_then(|g| g.as_ref().map(|p| p.title.clone()));
    let invite = INVITE.lock().ok().and_then(|g| g.as_ref().map(|i| json!({"pin": i.pin, "expires": i.expires, "playTogether": !i.addr.is_empty(), "link": invite_link(&i.secret)})));
    json!({"available": available, "enabled": enabled(), "askToJoin": ask_to_join(), "installed": installed(),
           "connected": available && CONNECTED.load(Ordering::SeqCst), "playing": playing, "invite": invite})
}

// ------------------------------------------------------------------ the game and its join secret

#[derive(Clone, Debug, PartialEq)]
pub struct Playing { pub id: String, pub title: String, pub art: Option<String>, pub start: u64 }

#[derive(Clone, Debug)]
struct Invite { party: String, secret: String, pin: String, addr: String, expires: u64 }

/// A game's title as Discord shows it: no ™/®/©, no store suffixes, no doubled spaces, at most 120 characters.
pub fn clean_title(t: &str) -> String {
    let mut s: String = t.chars().filter(|c| !matches!(c, '™' | '®' | '©')).collect();
    for suffix in [" (Steam)", " - Steam", " (Epic Games)", " (GOG)", " (Windows)", " (PC)"] {
        if s.ends_with(suffix) { s.truncate(s.len() - suffix.len()); }
    }
    let s = s.split_whitespace().collect::<Vec<_>>().join(" ");
    s.chars().take(120).collect()
}

/// The picture Discord shows: a game's web picture (Discord takes https addresses), else Hearth's own asset.
pub fn art_url(art: &[(&str, String)]) -> Option<String> {
    for kind in ["cover", "capsule", "grid", "hero", "icon"] {
        if let Some((_, u)) = art.iter().find(|(k, u)| *k == kind && u.starts_with("https://") && u.len() <= 256) { return Some(u.clone()); }
    }
    None
}

/// The join secret (at most 128 characters): `h1|<Hearth game id>|<host address>|<PIN>|<expires>|<title>`.
/// Address, PIN and expiry are empty / 0 when it's only "open the game" (no Play together). The title is cut to fit.
pub fn make_secret(id: &str, addr: &str, pin: &str, expires: u64, title: &str) -> Option<String> {
    if id.contains('|') || addr.contains('|') || pin.contains('|') { return None; }
    let head = format!("h1|{id}|{addr}|{pin}|{expires}|");
    if head.len() > SECRET_MAX { return None; }
    let room = SECRET_MAX - head.len();
    let mut t = String::new();
    for c in title.chars().filter(|c| *c != '|' && !c.is_control()) {
        if t.len() + c.len_utf8() > room { break; }
        t.push(c);
    }
    Some(head + &t)
}

#[derive(Debug, PartialEq)]
pub struct Secret { pub id: String, pub addr: String, pub pin: String, pub expires: u64, pub title: String }

pub fn parse_secret(s: &str) -> Option<Secret> {
    if s.len() > SECRET_MAX { return None; }
    let mut p = s.splitn(6, '|');
    if p.next()? != "h1" { return None; }
    let id = p.next()?.to_string();
    let addr = p.next()?.to_string();
    let pin = p.next()?.to_string();
    let expires = p.next()?.parse().ok()?;
    let title = p.next().unwrap_or("").to_string();
    if !crate::desktop_games::valid_id(&id) { return None; }
    if !pin.is_empty() && (pin.len() != 4 || !pin.chars().all(|c| c.is_ascii_digit())) { return None; }
    if !addr.is_empty() && !(addr.len() <= 100 && addr.chars().all(|c| c.is_ascii_alphanumeric() || ".:-[]".contains(c))) { return None; }
    Some(Secret { id, addr, pin, expires, title })
}

/// What a friend can open from a Discord chat: Hearth's page for it (site/join.html), which hands the secret to
/// Hearth (hearth://join/…). The secret goes in the part after # (never sent to the web server).
pub fn invite_link(secret: &str) -> String {
    let enc: String = secret.bytes().map(|b| if b.is_ascii_alphanumeric() || b".-_:".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect();
    format!("https://hearthos.fun/join#{enc}")
}

/// The activity Discord shows for a game, with the party and join secret when friends may join.
pub fn activity(p: &Playing, invite: Option<(&str, &str)>) -> Value {
    let t = clean_title(&p.title);
    let mut a = json!({"details": t, "state": "on Hearth OS", "timestamps": {"start": p.start},
        "assets": {"large_image": p.art.clone().unwrap_or_else(|| "hearth".into()), "large_text": t, "small_image": "hearth", "small_text": "Hearth OS"},
        "instance": false});
    if let Some((party, secret)) = invite {
        a["party"] = json!({"id": party, "size": [1, 4]});
        a["secrets"] = json!({"join": secret});
        a["instance"] = json!(true);
    }
    a
}

fn random_hex(n: usize) -> String { let mut b = vec![0u8; n]; let _ = getrandom::getrandom(&mut b); b.iter().map(|x| format!("{x:02x}")).collect() }
fn random_pin() -> String { let mut b = [0u8; 2]; let _ = getrandom::getrandom(&mut b); format!("{:04}", u16::from_le_bytes(b) % 10000) }
fn nonce() -> String { random_hex(8) }

/// A new invite for what's playing: Play together's address and PIN when Sunshine is on, else just the game.
fn new_invite(p: &Playing, party: Option<String>) -> Option<Invite> {
    if !ask_to_join() { return None; }
    let (addr, pin, expires) = match crate::play_together::host_address() {
        Some(a) => (a, random_pin(), now_secs() + PIN_SECS),
        None => (String::new(), String::new(), 0),
    };
    let secret = make_secret(&p.id, &addr, &pin, expires, &clean_title(&p.title))?;
    Some(Invite { party: party.unwrap_or_else(|| format!("hearth-{}", random_hex(8))), secret, pin, addr, expires })
}

/// The invite for Mantel's "Copy invite link" (hearth-discord reads it): $XDG_RUNTIME_DIR/hearth/discord-invite.json,
/// 0600, gone when the game ends.
fn invite_file() -> PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap_or_else(std::env::temp_dir).join("hearth/discord-invite.json")
}
fn write_invite(inv: Option<&Invite>, title: &str) {
    let f = invite_file();
    match inv {
        None => { let _ = std::fs::remove_file(&f); }
        Some(i) => {
            if let Some(d) = f.parent() { let _ = std::fs::create_dir_all(d); }
            let body = json!({"title": title, "pin": i.pin, "expires": i.expires, "playTogether": !i.addr.is_empty(), "link": invite_link(&i.secret)});
            let tmp = f.with_extension("tmp");
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                if let Ok(mut fh) = std::fs::OpenOptions::new().create(true).write(true).truncate(true).mode(0o600).open(&tmp) {
                    let _ = fh.write_all(body.to_string().as_bytes());
                    let _ = std::fs::rename(&tmp, &f);
                }
            }
        }
    }
}

/// Which pairing watch is current (a new PIN or the game ending stops the old one).
static PAIR_GEN: AtomicU64 = AtomicU64::new(0);

/// While a Play together PIN is valid, enter it in Sunshine whenever a friend's Moonlight starts pairing (Sunshine only
/// takes a PIN while one is pending). Once one pairs, the friend's input is made controller only (Settings › Remote
/// play's keyboard and mouse stay as they are for your own devices) and the worker makes a new PIN.
fn watch_pairing(pin: String, expires: u64, title: String) {
    let gen = PAIR_GEN.fetch_add(1, Ordering::SeqCst) + 1;
    std::thread::Builder::new().name("discord-pair".into()).spawn(move || {
        while PAIR_GEN.load(Ordering::SeqCst) == gen && now_secs() < expires {
            std::thread::sleep(Duration::from_secs(4));
            if PAIR_GEN.load(Ordering::SeqCst) != gen { return; }
            let name = format!("Friend Discord {}", &random_hex(2));
            if crate::remote::pair(&pin, &name).is_ok() {
                log("a friend's device paired for Play together");
                let _ = crate::play_together::set_input(true);
                let _ = std::process::Command::new("notify-send").args(["-a", "Hearth", "Play together", &format!("A friend joined {title} from Discord. Remove them in Settings › Remote play.")]).spawn().map(crate::procs::detach);
                crate::stores::notify(json!({"type": "event", "name": "discord", "data": {"event": "ptPaired", "data": {"name": name}}}));
                send(Msg::NewPin);
                return;
            }
        }
    }).ok();
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

/// Whole frames from what has arrived so far (the rest stays in `buf` for later). Err: a frame too big to be Discord's.
pub fn take_frames(buf: &mut Vec<u8>) -> Result<Vec<(u32, Value)>, String> {
    let mut out = vec![];
    loop {
        if buf.len() < 8 { return Ok(out); }
        let op = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
        let len = u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]) as usize;
        if len > MAX_FRAME { return Err(format!("a frame of {len} bytes")); }
        if buf.len() < 8 + len { return Ok(out); }
        let v = serde_json::from_slice(&buf[8..8 + len]).unwrap_or(Value::Null);
        buf.drain(..8 + len);
        out.push((op, v));
    }
}

/// The join secret from a frame, when it's Discord saying a friend pressed Join (ACTIVITY_JOIN).
pub fn join_secret(v: &Value) -> Option<String> {
    if v["cmd"] == "DISPATCH" && v["evt"] == "ACTIVITY_JOIN" { v["data"]["secret"].as_str().map(str::to_string) } else { None }
}

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

/// A friend's Discord said they joined: Mantel's helper finds the game (Play / Install) or starts Play together.
/// The secret goes to it on stdin, never on the command line (other users can read those).
fn joined(secret: String) {
    let helper = crate::paths::desktop_dir().join("shell/hearth-discord");
    log("a friend's invite was accepted here (ACTIVITY_JOIN)");
    std::thread::spawn(move || {
        let child = std::process::Command::new("python3").arg(&helper).args(["join", "--stdin"]).stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn();
        match child {
            Ok(mut c) => { if let Some(mut i) = c.stdin.take() { let _ = i.write_all(secret.as_bytes()); } crate::procs::detach(c); }
            Err(e) => log(&format!("couldn't start hearth-discord: {e}")),
        }
    });
}

enum Msg { Play(Playing), Clear, Refresh, NewPin }

static TX: OnceLock<Mutex<Sender<Msg>>> = OnceLock::new();

fn send(m: Msg) {
    let tx = TX.get_or_init(|| {
        let (tx, rx) = channel::<Msg>();
        std::thread::Builder::new().name("discord".into()).spawn(move || worker(rx)).ok();
        Mutex::new(tx)
    });
    if let Ok(t) = tx.lock() { let _ = t.send(m); }
}

fn set_invite(inv: Option<Invite>, title: &str) {
    write_invite(inv.as_ref(), title);
    if let Some(i) = &inv { if !i.pin.is_empty() { watch_pairing(i.pin.clone(), i.expires, clean_title(title)); } } else { PAIR_GEN.fetch_add(1, Ordering::SeqCst); }
    if let Ok(mut g) = INVITE.lock() { *g = inv; }
}

#[cfg(unix)]
fn worker(rx: std::sync::mpsc::Receiver<Msg>) {
    let mut want: Option<Playing> = None;
    let mut sock: Option<std::os::unix::net::UnixStream> = None;
    let mut buf: Vec<u8> = vec![];
    let mut shown = false;   // what Discord has is `want`
    let mut retry_at = 0u64;
    loop {
        let msg = rx.recv_timeout(Duration::from_secs(1));
        match msg {
            Ok(Msg::Play(p)) => { let inv = new_invite(&p, None); set_invite(inv, &p.title); want = Some(p); shown = false; retry_at = 0; }
            Ok(Msg::Clear) => { want = None; set_invite(None, ""); shown = false; retry_at = 0; }
            Ok(Msg::Refresh) => {
                if let Some(p) = &want { let inv = new_invite(p, None); set_invite(inv, &p.title); }
                shown = false; retry_at = 0;
            }
            Ok(Msg::NewPin) => {
                if let Some(p) = &want { let party = INVITE.lock().ok().and_then(|g| g.as_ref().map(|i| i.party.clone())); let inv = new_invite(p, party); set_invite(inv, &p.title); }
                shown = false;
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        // A PIN that ran out: a new one (the activity is sent again).
        let expired = INVITE.lock().ok().and_then(|g| g.as_ref().map(|i| i.expires != 0 && i.expires <= now_secs())).unwrap_or(false);
        if expired { if let Some(p) = &want { let party = INVITE.lock().ok().and_then(|g| g.as_ref().map(|i| i.party.clone())); let inv = new_invite(p, party); set_invite(inv, &p.title); shown = false; } }
        // What Discord says: answers, pings, and a friend joining.
        let mut gone = false;
        if let Some(s) = sock.as_mut() {
            let mut chunk = [0u8; 4096];
            loop {
                match s.read(&mut chunk) {
                    Ok(0) => { gone = true; break; }
                    Ok(n) => buf.extend_from_slice(&chunk[..n]),
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::TimedOut => break,
                    Err(_) => { gone = true; break; }
                }
            }
            match take_frames(&mut buf) {
                Ok(frames) => {
                    for (op, v) in frames {
                        if op == 2 { gone = true; }
                        else if op == 3 { let _ = s.write_all(&frame(4, &v)); }
                        else {
                            if v["evt"] == "ERROR" { log(&format!("Discord said: {}", v["data"]["message"].as_str().unwrap_or("?"))); }
                            if let Some(secret) = join_secret(&v) { joined(secret); }
                        }
                    }
                }
                Err(_) => gone = true,
            }
        }
        if gone { sock = None; buf.clear(); CONNECTED.store(false, Ordering::SeqCst); shown = false; }
        if shown || now_secs() < retry_at { continue; }
        let Some(id) = app_id() else { sock = None; CONNECTED.store(false, Ordering::SeqCst); shown = true; continue };
        let show = want.as_ref().filter(|_| enabled());
        if show.is_none() && sock.is_none() { shown = true; continue; }   // nothing to clear
        if sock.is_none() {
            match connect(&id) {
                Ok(s) => {
                    let _ = s.set_nonblocking(true);
                    let mut s = s;
                    let sub = json!({"cmd": "SUBSCRIBE", "evt": "ACTIVITY_JOIN", "args": {}, "nonce": nonce()});
                    let _ = s.write_all(&frame(1, &sub));
                    sock = Some(s);
                }
                Err(_) => { CONNECTED.store(false, Ordering::SeqCst); retry_at = now_secs() + 20; continue; }
            }
        }
        let invite = INVITE.lock().ok().and_then(|g| g.clone());
        let act = show.map(|p| activity(p, invite.as_ref().map(|i| (i.party.as_str(), i.secret.as_str())))).unwrap_or(Value::Null);
        let msg = json!({"cmd": "SET_ACTIVITY", "args": {"pid": std::process::id(), "activity": act}, "nonce": nonce()});
        let ok = sock.as_mut().map(|s| s.write_all(&frame(1, &msg)).is_ok()).unwrap_or(false);
        CONNECTED.store(ok, Ordering::SeqCst);
        if ok { shown = true; } else { sock = None; buf.clear(); retry_at = now_secs() + 20; }
        // Nothing playing any more: let go of Discord.
        if ok && show.is_none() { sock = None; buf.clear(); }
    }
}

#[cfg(not(unix))]
fn worker(rx: std::sync::mpsc::Receiver<Msg>) { while rx.recv().is_ok() {} }

/// A game from the library started (Some) or ended (None). Does nothing without an application ID.
pub fn set_playing(game: Option<Playing>) {
    if let Ok(mut g) = PLAYING.lock() { *g = game.clone(); }
    if app_id().is_none() { return; }
    match game {
        Some(p) => send(Msg::Play(p)),
        None => send(Msg::Clear),
    }
}

/// `set_playing` for a library game: its id, title and picture.
pub fn playing_from(id: &str, title: &str, art: &[(&str, String)]) -> Playing {
    Playing { id: id.to_string(), title: title.to_string(), art: art_url(art), start: now_secs() }
}

/// steam:// links Hearth may open (a joined game's Steam page, Steam's friends list): nothing that runs commands.
pub fn steam_link_ok(u: &str) -> bool {
    let ok_chars = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || ".:/-_[]".contains(c));
    let num = |s: &str| !s.is_empty() && s.len() <= 12 && s.chars().all(|c| c.is_ascii_digit());
    u == "steam://open/friends"
        || u.strip_prefix("steam://rungameid/").map_or(false, num)
        || u.strip_prefix("steam://store/").map_or(false, num)
        || u.strip_prefix("steam://install/").map_or(false, num)
        || u.strip_prefix("steam://connect/").map_or(false, |s| ok_chars(s) && s.len() <= 120)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_safe_steam_links() {
        assert!(steam_link_ok("steam://open/friends"));
        assert!(steam_link_ok("steam://rungameid/620"));
        assert!(steam_link_ok("steam://store/620"));
        assert!(steam_link_ok("steam://connect/192.168.1.2:27015"));
        assert!(!steam_link_ok("steam://rungameid/620;rm"));
        assert!(!steam_link_ok("steam://run/620//-console"));
        assert!(!steam_link_ok("steam://connect/1.2.3.4:27015 +exec evil"));
        assert!(!steam_link_ok("https://example.org"));
        assert!(!steam_link_ok("steam://openurl/https://evil"));
    }

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
    fn frames_arrive_in_pieces() {
        let mut all = frame(1, &json!({"cmd": "DISPATCH", "evt": "ACTIVITY_JOIN", "data": {"secret": "h1|steam:620||||0|Portal 2"}}));
        all.extend(frame(3, &json!({"x": 1})));
        let mut buf = all[..5].to_vec();
        assert!(take_frames(&mut buf).unwrap().is_empty());
        buf.extend_from_slice(&all[5..]);
        let got = take_frames(&mut buf).unwrap();
        assert_eq!(got.len(), 2);
        assert!(buf.is_empty());
        assert_eq!(join_secret(&got[0].1).as_deref(), Some("h1|steam:620||||0|Portal 2"));
        assert_eq!(got[1].0, 3);
        assert_eq!(join_secret(&json!({"cmd": "SET_ACTIVITY", "data": {"secret": "x"}})), None);
    }

    #[test]
    fn sockets_and_activity() {
        let c = socket_candidates(Path::new("/run/user/1000"));
        assert!(c.contains(&PathBuf::from("/run/user/1000/discord-ipc-0")));
        assert!(c.contains(&PathBuf::from("/run/user/1000/app/com.discordapp.Discord/discord-ipc-0")));
        let p = Playing { id: "steam:367520".into(), title: "Hollow Knight™".into(), art: None, start: 1700000000 };
        let a = activity(&p, None);
        assert_eq!(a["details"], "Hollow Knight");
        assert_eq!(a["timestamps"]["start"], 1700000000u64);
        assert_eq!(a["assets"]["large_image"], "hearth");
        assert!(a.get("party").is_none() && a.get("secrets").is_none());
        let a = activity(&p, Some(("hearth-1", "h1|steam:367520||||0|Hollow Knight")));
        assert_eq!(a["party"]["id"], "hearth-1");
        assert_eq!(a["party"]["size"], json!([1, 4]));
        assert_eq!(a["secrets"]["join"], "h1|steam:367520||||0|Hollow Knight");
        assert!(a.get("buttons").is_none(), "Discord doesn't show join buttons next to custom buttons");
    }

    #[test]
    fn titles_and_art() {
        assert_eq!(clean_title("  DOOM®   Eternal ™ "), "DOOM Eternal");
        assert_eq!(clean_title("Portal 2 - Steam"), "Portal 2");
        assert_eq!(clean_title(&"x".repeat(300)).len(), 120);
        assert_eq!(art_url(&[("hero", "file:///x.png".into()), ("cover", "https://cdn.example/c.jpg".into())]).as_deref(), Some("https://cdn.example/c.jpg"));
        assert_eq!(art_url(&[("cover", "/home/me/c.png".into())]), None);
    }

    #[test]
    fn secrets_round_trip_and_fit() {
        let s = make_secret("steam:620", "100.101.102.103", "0420", 1790000000, "Portal 2").unwrap();
        assert_eq!(s, "h1|steam:620|100.101.102.103|0420|1790000000|Portal 2");
        assert_eq!(parse_secret(&s), Some(Secret { id: "steam:620".into(), addr: "100.101.102.103".into(), pin: "0420".into(), expires: 1790000000, title: "Portal 2".into() }));
        let long = make_secret("epic:abcdef", "192.168.1.20", "1234", 1790000000, &"Ü|".repeat(200)).unwrap();
        assert!(long.len() <= SECRET_MAX && !long[long.rfind("|").unwrap() + 1..].contains('|'));
        assert!(parse_secret(&long).is_some());
        let plain = make_secret("gog:1", "", "", 0, "Celeste").unwrap();
        let p = parse_secret(&plain).unwrap();
        assert!(p.addr.is_empty() && p.pin.is_empty());
        assert!(make_secret("bad|id", "", "", 0, "x").is_none());
        assert!(parse_secret("h2|steam:1||||0|x").is_none());
        assert!(parse_secret("h1|steam:1|1.2.3.4;rm|1234|0|x").is_none());
        assert!(parse_secret("h1|steam:1|1.2.3.4|12a4|0|x").is_none());
        assert!(parse_secret("h1|nocolon|||0|x").is_none());
        assert!(invite_link(&plain).starts_with("https://hearthos.fun/join#h1%7Cgog:1%7C"));
    }

    #[test]
    fn settings_are_merged_and_defaults_never_written() {
        let dir = std::env::temp_dir().join(format!("hearth-discord-set-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let f = dir.join("discord.json");
        std::fs::write(&f, r#"{"client":"web","other":1}"#).unwrap();
        let old = std::env::var_os("HEARTH_DISCORD_SETTINGS");
        std::env::set_var("HEARTH_DISCORD_SETTINGS", &f);
        assert!(enabled() && ask_to_join());
        assert_eq!(std::fs::read_to_string(&f).unwrap(), r#"{"client":"web","other":1}"#, "reading never writes defaults");
        merge("enabled", json!(false)).unwrap();
        let v: Value = serde_json::from_str(&std::fs::read_to_string(&f).unwrap()).unwrap();
        assert_eq!(v["client"], "web");
        assert_eq!(v["other"], 1);
        assert_eq!(v["enabled"], false);
        assert!(!enabled());
        assert!(set_app_id("123").is_err());
        match old { Some(o) => std::env::set_var("HEARTH_DISCORD_SETTINGS", o), None => std::env::remove_var("HEARTH_DISCORD_SETTINGS") }
        let _ = std::fs::remove_dir_all(&dir);
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
        let p = Playing { id: "steam:504230".into(), title: "Celeste".into(), art: None, start: 5 };
        let msg = json!({"cmd": "SET_ACTIVITY", "args": {"pid": 1, "activity": activity(&p, None)}, "nonce": nonce()});
        s.write_all(&frame(1, &msg)).unwrap();
        read_frame(&mut s).unwrap();
        let got = server.join().unwrap();
        assert_eq!(got["args"]["activity"]["details"], "Celeste");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
