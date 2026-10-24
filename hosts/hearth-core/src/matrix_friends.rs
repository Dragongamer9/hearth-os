//! Hearth Friends in the core: Game Mode's page (and Quick Access over a game) reach the Hearth Friends service
//! (hosts/hearth-friends: Matrix with end-to-end encryption) through `mfriends.*` calls. The service runs on its own
//! (hearth-friends.service in Hearth's session); when it isn't running the core starts it. The core:
//!  * passes the page's requests on over the local socket ($XDG_RUNTIME_DIR/hearth-friends.sock),
//!  * listens for the service's events (messages, presence, requests) and hands them to the page as
//!    {type:'event', name:'mfriends'} — in Game Mode it registers as "gamemode", so the toast is Hearth's own and
//!    nothing pops up on the desktop,
//!  * tells the service what's being played (the "Playing <game>" status; the service leaves it out when the player
//!    turned sharing off),
//!  * does the Play together side on this device (Sunshine to host, Moonlight to join: play_together below), and
//!    opens Steam's own friends and join windows with steam:// links (never a Steam account or token).
//! No tokens pass through here: the session lives in the service and the system keyring.

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

pub fn socket() -> PathBuf {
    if let Some(p) = std::env::var_os("HEARTH_FRIENDS_SOCKET") { return PathBuf::from(p); }
    std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap_or_else(std::env::temp_dir).join("hearth-friends.sock")
}

/// The service's program: $HEARTH_FRIENDS_BIN (tests), else the one Hearth OS installs.
pub fn program() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("HEARTH_FRIENDS_BIN").map(PathBuf::from) { if p.is_file() { return Some(p); } }
    ["/usr/bin/hearth-friends", "/usr/local/bin/hearth-friends"].iter().map(PathBuf::from).find(|p| p.is_file())
}

pub fn available() -> bool { program().is_some() || UnixStream::connect(socket()).is_ok() }

fn connect() -> Result<UnixStream, String> {
    if let Ok(s) = UnixStream::connect(socket()) { return Ok(s); }
    let prog = program().ok_or("Hearth Friends isn't on this system yet (it comes with Hearth OS).")?;
    static STARTING: Mutex<()> = Mutex::new(());
    let _g = STARTING.lock().unwrap_or_else(|e| e.into_inner());
    if let Ok(s) = UnixStream::connect(socket()) { return Ok(s); }
    // Its own log, next to its data (it never writes tokens there).
    let dir = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|| crate::paths::home().join(".local/share")).join("hearth/friends");
    let _ = std::fs::create_dir_all(&dir);
    let log = std::fs::OpenOptions::new().create(true).append(true).open(dir.join("service.log")).ok();
    let mut cmd = std::process::Command::new(prog);
    cmd.arg("serve").stdin(std::process::Stdio::null());
    match log.and_then(|l| l.try_clone().ok().map(|c| (l, c))) {
        Some((a, b)) => { cmd.stdout(a).stderr(b); }
        None => { cmd.stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()); }
    }
    let child = cmd.spawn().map_err(|e| format!("Couldn't start Hearth Friends ({e})"))?;
    crate::procs::detach(child);
    for _ in 0..50 {
        std::thread::sleep(Duration::from_millis(100));
        if let Ok(s) = UnixStream::connect(socket()) { return Ok(s); }
    }
    Err("Hearth Friends didn't start. Try again in a moment.".into())
}

/// One request to the service. Sign-in and sending pictures can take a while; the rest answer quickly.
pub fn call(method: &str, args: &Value) -> Result<Value, String> {
    let mut s = connect()?;
    let secs = match method { "login" | "register" => 90, "sendImage" | "image" | "messages" => 60, _ => 30 };
    s.set_read_timeout(Some(Duration::from_secs(secs))).ok();
    let mut line = json!({"id": 1, "method": method, "args": args}).to_string();
    line.push('\n');
    s.write_all(line.as_bytes()).map_err(|e| e.to_string())?;
    let mut out = String::new();
    BufReader::new(s).read_line(&mut out).map_err(|_| "Hearth Friends didn't answer in time.".to_string())?;
    let v: Value = serde_json::from_str(&out).map_err(|_| "Hearth Friends gave a garbled answer.".to_string())?;
    if v["ok"] == true { Ok(v["value"].clone()) } else { Err(v["error"].as_str().unwrap_or("Failed").to_string()) }
}

static WATCHING: AtomicBool = AtomicBool::new(false);

/// Starts listening for the service's events (once): each goes to the page as {name:'mfriends', data:{event, data}}.
pub fn watch() {
    if WATCHING.swap(true, Ordering::SeqCst) { return; }
    std::thread::spawn(|| {
        let client = if crate::gamescope::inside() { "gamemode" } else { "hearth" };
        let mut backoff = 2u64;
        loop {
            if let Ok(mut s) = connect() {
                let mut line = json!({"id": 1, "method": "watch", "args": {"client": client}}).to_string();
                line.push('\n');
                if s.write_all(line.as_bytes()).is_ok() {
                    backoff = 2;
                    crate::stores::notify(json!({"type": "event", "name": "mfriends", "data": {"event": "connected", "data": {}}}));
                    for l in BufReader::new(s).lines().map_while(Result::ok) {
                        let Ok(v) = serde_json::from_str::<Value>(&l) else { continue };
                        let Some(ev) = v["event"].as_str() else { continue };
                        crate::stores::notify(json!({"type": "event", "name": "mfriends", "data": {"event": ev, "data": v["data"]}}));
                    }
                }
            } else if program().is_none() {
                WATCHING.store(false, Ordering::SeqCst);
                return;
            }
            std::thread::sleep(Duration::from_secs(backoff));
            backoff = (backoff * 2).min(60);
        }
    });
}

/// What was last told to the service as being played (Hearth's game id), so it's only told on a change.
fn last_playing() -> &'static Mutex<Option<String>> { static L: OnceLock<Mutex<Option<String>>> = OnceLock::new(); L.get_or_init(Default::default) }

/// Called after each of the page's requests with what's running: tells the service when the game changes.
/// `game`: (Hearth id, title, store, store's own id).
pub fn note_playing(game: Option<(String, String, String, Option<String>)>) {
    let id = game.as_ref().map(|g| g.0.clone());
    {
        let mut l = last_playing().lock().unwrap_or_else(|e| e.into_inner());
        if *l == id { return; }
        *l = id;
    }
    if !available() { return; }
    let joinable = play_together::joinable();
    std::thread::spawn(move || {
        let g = game.map(|(id, title, store, app)| json!({"id": id, "title": title, "store": store, "appId": app}));
        let _ = call("playing", &json!({"game": g, "joinable": joinable}));
    });
}

/// steam:// links Hearth may open for Friends: Steam's friends list (its own Join/Invite), a game, a server.
pub fn steam_link_ok(u: &str) -> bool {
    let ok_chars = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || ".:/-_[]".contains(c));
    u == "steam://open/friends"
        || u.strip_prefix("steam://rungameid/").map_or(false, |s| !s.is_empty() && s.len() <= 12 && s.chars().all(|c| c.is_ascii_digit()))
        || u.strip_prefix("steam://store/").map_or(false, |s| !s.is_empty() && s.len() <= 12 && s.chars().all(|c| c.is_ascii_digit()))
        || u.strip_prefix("steam://install/").map_or(false, |s| !s.is_empty() && s.len() <= 12 && s.chars().all(|c| c.is_ascii_digit()))
        || u.strip_prefix("steam://connect/").map_or(false, |s| ok_chars(s) && s.len() <= 120)
}

/// Emoji & symbols' list ([{e, n, g, k}]), from where Hearth OS installs it (or the repository, when run from it).
pub fn emoji() -> Value {
    let mut places: Vec<PathBuf> = vec![];
    if let Some(s) = std::env::var_os("NIMBUS_SHARE") { places.push(PathBuf::from(s).join("emoji-data.js")); }
    places.push(PathBuf::from("/usr/share/nimbus/emoji-data.js"));
    if let Ok(exe) = std::env::current_exe() { for a in exe.ancestors().take(6) { places.push(a.join("hosts/hearth-emoji/ui/emoji-data.js")); } }
    for p in places {
        let Ok(t) = std::fs::read_to_string(&p) else { continue };
        let Some(start) = t.find('[') else { continue };
        let body = t[start..].trim().trim_end_matches(';');
        if let Ok(v) = serde_json::from_str::<Value>(body) { if v.is_array() { return v; } }
    }
    json!([])
}

pub const DISCORD: &str = "com.discordapp.Discord";
pub fn discord_installed() -> bool { crate::system::has("flatpak") && crate::system::run("flatpak", &["info", DISCORD]).is_ok() }

/// Play together (Beta Experimental): Remote Play Together, Hearth's way. The host streams the game it's playing
/// with Sunshine (Settings › Remote play's own Sunshine) and a friend's Hearth plays it through Moonlight, their
/// controller becoming another player on the host's game. The steps travel as messages in the encrypted chat:
/// friend → request (with a one-time 4-digit PIN), host accepts (Sunshine pairs that PIN only), friend's Moonlight
/// pairs and streams. The host can end it or remove the friend at any time, and can make the friend's input
/// controller only (no keyboard or mouse on the host's PC).
/// How the friend reaches the host is a Transport: the local network now; Hearth Link (hosts/hearth-link,
/// peer-to-peer with relays) plugs in through `hearth-link call play.address`, so this code doesn't change when it does.
pub mod play_together {
    use serde_json::{json, Value};
    use std::path::Path;
    use std::time::{Duration, Instant};

    pub trait Transport: Send + Sync {
        fn id(&self) -> &'static str;
        fn ready(&self) -> bool;
        /// Addresses a friend can reach this device's stream at.
        fn host_addresses(&self) -> Vec<String>;
    }

    /// The same network: this device's own addresses.
    pub struct Lan;
    impl Transport for Lan {
        fn id(&self) -> &'static str { "lan" }
        fn ready(&self) -> bool { true }
        fn host_addresses(&self) -> Vec<String> { crate::remote::addresses() }
    }

    /// Over the internet through Hearth Link (hosts/hearth-link: iroh QUIC, peer-to-peer with relays): asked with
    /// `hearth-link call play.address {}`, which is to answer {"ready": bool, "addresses": [...]} — addresses a friend's
    /// Moonlight can reach this device's Sunshine at through Link's tunnel. Hearth Link doesn't have that call yet (it
    /// links your own paired devices; a tunnel for a friend's device is still to come), so until it does this
    /// transport says it isn't ready and Play together uses the local network.
    pub struct Link;
    impl Link {
        fn program() -> Option<&'static str> {
            ["/usr/bin/hearth-link", "/usr/libexec/nimbus/hearth-link"].into_iter().find(|p| Path::new(p).is_file())
        }
        fn ask() -> Option<Value> {
            let p = Self::program()?;
            let out = crate::system::run_timeout(p, &["call", "play.address", "{}"], Duration::from_secs(8)).ok()?;
            let v: Value = serde_json::from_str(out.trim()).ok()?;
            if v.get("error").is_some() { None } else { Some(v) }
        }
    }

    pub fn transports() -> Vec<Box<dyn Transport>> { vec![Box::new(Lan), Box::new(Link)] }

    fn sunshine_installed() -> bool { crate::system::has("sunshine") || Path::new("/usr/bin/sunshine").exists() }

    /// Friends see "Joinable" while a game runs on a device that can host (Sunshine is there), unless the player
    /// turned "Let friends ask to join" off.
    pub fn joinable() -> bool {
        sunshine_installed() && crate::store::Store::open(crate::paths::data_dir().join("friends-play.json")).data["askToJoin"] != false
    }

    pub fn set_ask_to_join(on: bool) -> Result<Value, String> {
        let mut s = crate::store::Store::open(crate::paths::data_dir().join("friends-play.json"));
        s.update(|d| { d.insert("askToJoin".into(), json!(on)); }).map_err(|e| e.to_string())?;
        Ok(host_info())
    }

    pub fn host_info() -> Value {
        let ts = transports();
        let ready: Vec<&Box<dyn Transport>> = ts.iter().filter(|t| t.ready()).collect();
        let addrs: Vec<String> = ready.iter().flat_map(|t| t.host_addresses()).collect();
        json!({"canHost": sunshine_installed(), "canJoin": crate::remote::moonlight_installed(), "addresses": addrs,
               "transports": ready.iter().map(|t| t.id()).collect::<Vec<_>>(), "port": 47989,
               "askToJoin": crate::store::Store::open(crate::paths::data_dir().join("friends-play.json")).data["askToJoin"] != false})
    }

    fn emit(ev: &str, data: Value) { crate::stores::notify(json!({"type": "event", "name": "mfriends", "data": {"event": ev, "data": data}})); }

    /// The name a friend's device is paired under in Sunshine, so it can be found and removed later.
    pub fn paired_name(friend: &str, device: &str) -> String {
        let f: String = friend.chars().filter(|c| c.is_alphanumeric() || " ._-@:".contains(*c)).take(24).collect();
        let d: String = device.chars().filter(|c| c.is_alphanumeric() || " ._-".contains(*c)).take(12).collect();
        format!("Friend {f} {d}").trim().to_string()
    }

    /// Host: the friend's request was accepted. Sunshine on, the friend's input set, then the PIN entered as soon as
    /// their Moonlight starts pairing (Sunshine only takes a PIN while one is pending): tried for 90 seconds.
    pub fn host_accept(session: &str, pin: &str, friend: &str, device: &str, controller_only: bool) -> Result<Value, String> {
        if pin.len() != 4 || !pin.chars().all(|c| c.is_ascii_digit()) { return Err("That join request has no valid PIN.".into()); }
        if !sunshine_installed() { return Err("Play together needs Sunshine (Settings › Remote play), which comes with Hearth OS.".into()); }
        let (session, pin, name) = (session.to_string(), pin.to_string(), paired_name(friend, device));
        std::thread::spawn(move || {
            let st = crate::remote::status();
            if st["on"] != true { if let Err(e) = crate::remote::set_on(true) { emit("ptFailed", json!({"session": session, "why": e})); return; } }
            let _ = set_input(controller_only);
            let until = Instant::now() + Duration::from_secs(90);
            while Instant::now() < until {
                if crate::remote::pair(&pin, &name).is_ok() { emit("ptPaired", json!({"session": session, "name": name})); return; }
                std::thread::sleep(Duration::from_secs(2));
            }
            emit("ptFailed", json!({"session": session, "why": "Your friend's device didn't connect in time."}));
        });
        Ok(json!({"waiting": true}))
    }

    /// The friend's keyboard and mouse off (controller only), or back on: Sunshine's own settings, merged with what
    /// it has (its API replaces the whole file), then Sunshine is asked to restart for it to take effect.
    pub fn set_input(controller_only: bool) -> Result<(), String> {
        let want = if controller_only { "disabled" } else { "enabled" };
        let cur = crate::remote::api("GET", "/api/config", None, creds())?;
        let mut cfg = serde_json::Map::new();
        if let Some(o) = cur.as_object() {
            for (k, v) in o {
                // What GET adds that isn't a setting.
                if ["status", "platform", "version", "restart_supported", "vdisplayStatus"].contains(&k.as_str()) { continue; }
                if let Some(s) = v.as_str() { cfg.insert(k.clone(), json!(s)); }
            }
        }
        if cfg.get("keyboard").and_then(|v| v.as_str()) == Some(want) && cfg.get("mouse").and_then(|v| v.as_str()) == Some(want) { return Ok(()); }
        cfg.insert("keyboard".into(), json!(want));
        cfg.insert("mouse".into(), json!(want));
        crate::remote::api("POST", "/api/config", Some(&Value::Object(cfg)), creds())?;
        let _ = crate::remote::api("POST", "/api/restart", None, creds());
        Ok(())
    }

    fn creds() -> Option<(String, String)> {
        let v: Value = std::fs::read(crate::paths::data_dir().join("remote-play.json")).ok().and_then(|b| serde_json::from_slice(&b).ok())?;
        Some((v["user"].as_str()?.to_string(), v["pass"].as_str()?.to_string()))
    }

    /// Host: remove a friend from the game (or end Play together): their device unpaired and the stream closed.
    pub fn host_end(name: Option<&str>) -> Result<Value, String> {
        let st = crate::remote::status();
        let mut removed = 0;
        for c in st["clients"].as_array().cloned().unwrap_or_default() {
            let n = c["name"].as_str().unwrap_or("");
            let hit = match name { Some(want) => n == want, None => n.starts_with("Friend ") };
            if hit { if let Some(u) = c["uuid"].as_str() { if crate::remote::unpair(u).is_ok() { removed += 1; } } }
        }
        let _ = crate::remote::api("POST", "/api/apps/close", None, creds());
        Ok(json!({"removed": removed}))
    }

    fn moonlight(args: &[&str]) -> std::process::Command {
        let mut c = std::process::Command::new("flatpak");
        c.args(["run", "com.moonlight_stream.Moonlight"]).args(args);
        c
    }

    /// Friend: pair Moonlight with the host using our PIN (the host's Hearth enters it), then stream its game.
    pub fn guest_join(session: &str, address: &str, pin: &str) -> Result<Value, String> {
        if !crate::remote::moonlight_installed() { return Err("Joining needs Moonlight. Install it from Settings › Remote play.".into()); }
        if pin.len() != 4 || !pin.chars().all(|c| c.is_ascii_digit()) { return Err("No valid PIN for this join.".into()); }
        let ok_addr = !address.is_empty() && address.len() <= 100 && address.chars().all(|c| c.is_ascii_alphanumeric() || ".:-[]".contains(c));
        if !ok_addr { return Err("That host address isn't right.".into()); }
        let (session, address, pin) = (session.to_string(), address.to_string(), pin.to_string());
        std::thread::spawn(move || {
            let out = moonlight(&["pair", &address, "--pin", &pin]).stdin(std::process::Stdio::null()).output();
            let paired = match &out { Ok(o) => o.status.success() || String::from_utf8_lossy(&o.stdout).to_lowercase().contains("already paired"), Err(_) => false };
            if !paired { emit("ptFailed", json!({"session": session, "why": "Couldn't pair with your friend's device. Are you on the same network?"})); return; }
            match moonlight(&["stream", &address, "Desktop"]).stdin(std::process::Stdio::null()).spawn() {
                Ok(c) => { crate::procs::detach(c); emit("ptStreaming", json!({"session": session})); }
                Err(e) => emit("ptFailed", json!({"session": session, "why": format!("Moonlight didn't start ({e})")})),
            }
        });
        Ok(json!({"joining": true}))
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn paired_names_are_tidy() {
            assert_eq!(paired_name("@sam:matrix.org", "Ally X"), "Friend @sam:matrix.org Ally X");
            assert_eq!(paired_name("@a;rm -rf:x", "d\"ev"), "Friend @arm -rf:x dev");
            assert!(paired_name(&"x".repeat(100), "y").len() < 45);
        }
    }
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
    // One test for both: they set the same variables.
    #[test]
    fn socket_follows_the_environment_and_no_service_is_a_plain_error() {
        std::env::set_var("HEARTH_FRIENDS_SOCKET", "/tmp/x.sock");
        assert_eq!(socket(), PathBuf::from("/tmp/x.sock"));
        std::env::set_var("HEARTH_FRIENDS_SOCKET", "/nonexistent/dir/hearth-friends.sock");
        std::env::set_var("HEARTH_FRIENDS_BIN", "/nonexistent/hearth-friends");
        if program().is_none() { assert!(call("status", &json!({})).unwrap_err().contains("Hearth Friends")); }
        std::env::remove_var("HEARTH_FRIENDS_SOCKET");
        std::env::remove_var("HEARTH_FRIENDS_BIN");
    }
}
