//! `firehold-shell hub`: the session's shell process (firehold-comp starts it, the way it started Mantel). No drawing
//! here: it starts the surfaces (couch, bar, dock; the guide, a notice, the overlay and Settings when they're needed),
//! keeps one connection to firehold-comp's window IPC (the window list, input kinds, pictures, Firehold's title bars),
//! reads the controller, decides the posture, and tells each surface what to show. Everything waits on something
//! (a socket, the controller, a child, the next minute): nothing here polls or spins.
//!
//! Safe to fail (red flag 2): a surface that crashes is started again (at most 5 times a minute); the hub writes
//! `$XDG_RUNTIME_DIR/firehold-shell-up` once the first surfaces said hello, and nimbus-session falls back to KDE
//! Plasma when that doesn't come or the hub ends.

use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::{json, Map, Value};

use crate::config::{self, Couch};
use crate::model::{self, Game, Win};
use crate::proto::{self, Guide, Notice, State};
use crate::settings_file;
use crate::sys;

enum Ev {
    CompUp(UnixStream),
    Comp(Value),
    CompGone,
    ClientNew(u64, UnixStream),
    Client(u64, Value),
    ClientGone(u64),
    Pad(&'static str),
    Tick,
    PicTick,
    PerfTick,
    NoticeEnd(u64),
    Library(Vec<Game>, String),
    ChildExit(String, u32, i32),
}

struct Client {
    stream: UnixStream,
    surface: String,
}

fn log(msg: &str) {
    eprintln!("firehold-shell hub: {msg}");
}

fn runtime() -> PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap_or_else(|| std::env::temp_dir())
}
fn wayland_name() -> String {
    let n = std::env::var("WAYLAND_DISPLAY").unwrap_or_else(|_| "wayland-0".into());
    Path::new(&n).file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or(n)
}
fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

/// A Hearth helper: on PATH first (tests' stand-ins), then Hearth's own folder ($NIMBUS_SHARE or /usr/share/nimbus).
fn helper(name: &str) -> Option<Vec<String>> {
    if sys::which(name) {
        return Some(vec![name.to_string()]);
    }
    let share = std::env::var("NIMBUS_SHARE").unwrap_or_else(|_| "/usr/share/nimbus".into());
    let p = Path::new(&share).join("desktop/shell").join(name);
    p.is_file().then(|| vec!["python3".into(), p.display().to_string()])
}
fn hearth_app() -> Option<String> {
    for c in [std::env::var("NIMBUS_APP").ok(), Some("/usr/bin/nimbus".into())].into_iter().flatten() {
        if Path::new(&c).is_file() {
            return Some(c);
        }
    }
    sys::which("nimbus").then(|| "nimbus".into())
}

/// A command's JSON output, or None (it's killed after `secs`).
fn run_json(cmd: &[String], secs: u64) -> Option<Value> {
    let (prog, args) = cmd.split_first()?;
    let mut child = Command::new(prog).args(args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().ok()?;
    let mut out = child.stdout.take()?;
    let pid = child.id();
    let done = Arc::new(AtomicBool::new(false));
    let d2 = done.clone();
    std::thread::spawn(move || {
        let until = Instant::now() + Duration::from_secs(secs);
        while Instant::now() < until {
            std::thread::sleep(Duration::from_millis(250));
            if d2.load(Ordering::Relaxed) {
                return;
            }
        }
        unsafe {
            libc::kill(pid as i32, libc::SIGKILL);
        }
    });
    let mut s = String::new();
    let _ = out.read_to_string(&mut s);
    let _ = child.wait();
    done.store(true, Ordering::Relaxed);
    let line = s.lines().rev().find(|l| l.trim_start().starts_with(['[', '{']))?;
    serde_json::from_str(line).ok()
}

/// Steam's own cloud folders: userdata/<account>/<appid>/remote with a file in it.
fn steam_cloud(appid: &str) -> bool {
    let h = config::home();
    for base in [h.join(".local/share/Steam/userdata"), h.join(".steam/steam/userdata")] {
        let Ok(accts) = std::fs::read_dir(&base) else { continue };
        for a in accts.flatten() {
            let r = a.path().join(appid).join("remote");
            if std::fs::read_dir(&r).map(|mut d| d.next().is_some()).unwrap_or(false) {
                return true;
            }
        }
    }
    false
}

fn load_library() -> (Vec<Game>, String) {
    let Some(app) = hearth_app() else { return (Vec::new(), "Hearth's app isn't installed, so there's no library here.".into()) };
    let raw = run_json(&[app.clone(), "--list-games".into(), "--all".into()], 25).or_else(|| run_json(&[app, "--list-games".into()], 25));
    let saved: HashSet<String> = helper("hearth-saves")
        .and_then(|mut h| {
            h.push("status".into());
            run_json(&h, 10)
        })
        .and_then(|v| v.get("games").and_then(|g| g.as_array()).cloned())
        .map(|g| g.iter().filter(|x| x.get("backedUp").and_then(|b| b.as_bool()) == Some(true)).filter_map(|x| x.get("name")?.as_str().map(|n| n.to_lowercase())).collect())
        .unwrap_or_default();
    let cloud = |id: &str, title: &str| saved.contains(&title.to_lowercase()) || id.strip_prefix("steam:").is_some_and(steam_cloud);
    let mut games = raw.as_ref().map(|r| model::parse_library(r, &cloud)).unwrap_or_default();
    if let Some(mut h) = helper("hearth-winapps") {
        h.push("list".into());
        if let Some(v) = run_json(&h, 10) {
            games.extend(model::parse_winapps(&v));
        }
    }
    let note = if raw.is_none() { "Hearth's library didn't answer; it's tried again when you come back to Home.".to_string() } else { String::new() };
    (games, note)
}

// ------------------------------------------------------------------------------------- the game rows (mockup 3)

const FRAME_LIMITS: [&str; 7] = ["off", "30", "40", "60", "90", "120", "screen"];
const SCALINGS: [&str; 4] = ["off", "quality", "balanced", "performance"];

fn games_path() -> PathBuf {
    std::env::var_os("FIREHOLD_GAMES").map(PathBuf::from).unwrap_or_else(|| config::config_dir().join("hearth/firehold-games.json"))
}

/// firehold-comp's game socket: one line in, one JSON line out.
fn game_ask(cmd: &str) -> Option<Value> {
    let p = runtime().join(format!("firehold-game-{}.sock", wayland_name()));
    let mut s = UnixStream::connect(p).ok()?;
    let _ = s.set_read_timeout(Some(Duration::from_millis(500)));
    let _ = s.set_write_timeout(Some(Duration::from_millis(500)));
    s.write_all(format!("{cmd}\n").as_bytes()).ok()?;
    let mut line = String::new();
    BufReader::new(s).read_line(&mut line).ok()?;
    serde_json::from_str(&line).ok()
}

/// A game's choice from firehold-games.json (read only here; written by `game_step`), as text.
fn game_choice(app: &str, key: &str) -> String {
    let m = settings_file::read(&games_path()).map().unwrap_or_default();
    let v = m.get("games").and_then(|g| g.get(app)).and_then(|g| g.get(key)).cloned();
    match (key, v) {
        ("frameLimit", Some(Value::Number(n))) => n.to_string(),
        ("frameLimit" | "scaling", Some(Value::String(s))) => s,
        ("frameLimit" | "scaling", _) => "off".into(),
        (_, Some(Value::Bool(b))) => (if b { "on" } else { "off" }).to_string(),
        ("tearing", _) => "off".into(),
        _ => "on".into(),
    }
}

/// The Guide's ‹ › on a game row: the next value, merged into firehold-games.json for that game only, then the
/// compositor is asked to read it again.
fn game_step(app: &str, key: &str, up: bool) -> Result<(), String> {
    if app.is_empty() || app.len() > 200 {
        return Err("no game in front".into());
    }
    let cur = game_choice(app, key);
    let next: Value = match key {
        "frameLimit" | "scaling" => {
            let seq: &[&str] = if key == "frameLimit" { &FRAME_LIMITS } else { &SCALINGS };
            let i = seq.iter().position(|s| *s == cur).unwrap_or(0) as i64;
            let n = seq[((i + if up { 1 } else { -1 }).rem_euclid(seq.len() as i64)) as usize];
            n.parse::<i64>().map(Value::from).unwrap_or_else(|_| Value::from(n))
        }
        "vrr" | "tearing" | "direct" | "sharpen" => Value::Bool(cur != "on"),
        _ => return Err(format!("unknown row {key}")),
    };
    settings_file::update_json(&games_path(), |m| {
        let games = m.entry("games").or_insert_with(|| json!({}));
        if !games.is_object() {
            *games = json!({});
        }
        let g = games.as_object_mut().unwrap().entry(app.to_string()).or_insert_with(|| json!({}));
        if !g.is_object() {
            *g = json!({});
        }
        g.as_object_mut().unwrap().insert(key.to_string(), next.clone());
    })
    .map_err(|e| e.to_string())?;
    let _ = game_ask("reload");
    Ok(())
}

// ------------------------------------------------------------------------------------- the hub

struct Hub {
    tx: Sender<Ev>,
    exe: PathBuf,
    comp: Option<UnixStream>,
    clients: HashMap<u64, Client>,
    st: State,
    raw_wins: Vec<Value>,
    library: Vec<Game>,
    library_at: Option<Instant>,
    library_loading: bool,
    apps: Vec<sys::App>,
    children: HashMap<String, u32>,
    restarts: HashMap<String, Vec<Instant>>,
    wanted: HashSet<String>,
    hello: HashSet<String>,
    up_written: bool,
    pics_on: Arc<AtomicBool>,
    perf_on: Arc<AtomicBool>,
    pic_n: u64,
    pics: HashMap<String, String>,
    seen_games: HashMap<String, Instant>,
    known_wins: HashSet<String>,
    ours_fullscreen: HashSet<String>,
    frames_sent: String,
    notice_serial: u64,
    sent: HashMap<u64, String>,
    started: Instant,
}

impl Hub {
    fn send_to(&mut self, id: u64, v: &Value) {
        let dead = match self.clients.get_mut(&id) {
            Some(c) => (&c.stream).write_all(format!("{v}\n").as_bytes()).is_err(),
            None => false,
        };
        if dead {
            self.clients.remove(&id);
        }
    }
    fn send_surface(&mut self, surface: &str, v: &Value) {
        let ids: Vec<u64> = self.clients.iter().filter(|(_, c)| c.surface == surface).map(|(i, _)| *i).collect();
        for i in ids {
            self.send_to(i, v);
        }
    }
    fn comp_send(&mut self, v: Value) {
        if let Some(s) = self.comp.as_mut() {
            if s.write_all(format!("{v}\n").as_bytes()).is_err() {
                self.comp = None;
            }
        }
    }

    /// The state to everyone who said hello, only where it changed for them.
    fn broadcast(&mut self) {
        self.st.now_ms = now_ms();
        let mut v = serde_json::to_value(&self.st).unwrap_or(Value::Null);
        if let Some(o) = v.as_object_mut() {
            o.insert("t".into(), json!("state"));
        }
        // now_ms alone doesn't count as a change.
        let key = {
            let mut k = v.clone();
            if let Some(o) = k.as_object_mut() {
                o.remove("now_ms");
            }
            k.to_string()
        };
        let ids: Vec<u64> = self.clients.iter().filter(|(_, c)| !c.surface.is_empty() && c.surface != "ctl").map(|(i, _)| *i).collect();
        for i in ids {
            if self.sent.get(&i) != Some(&key) {
                self.send_to(i, &v);
                self.sent.insert(i, key.clone());
            }
        }
    }

    fn spawn(&mut self, name: &str) {
        if self.children.contains_key(name) {
            return;
        }
        let now = Instant::now();
        let r = self.restarts.entry(name.to_string()).or_default();
        r.retain(|t| now.duration_since(*t) < Duration::from_secs(60));
        if r.len() >= 5 {
            log(&format!("{name} keeps ending: not started again for now"));
            return;
        }
        r.push(now);
        match Command::new(&self.exe).arg(name).stdin(Stdio::null()).spawn() {
            Ok(mut child) => {
                let pid = child.id();
                self.children.insert(name.to_string(), pid);
                let tx = self.tx.clone();
                let n = name.to_string();
                std::thread::spawn(move || {
                    let code = child.wait().ok().and_then(|s| s.code()).unwrap_or(-1);
                    let _ = tx.send(Ev::ChildExit(n, pid, code));
                });
            }
            Err(e) => log(&format!("couldn't start {name}: {e}")),
        }
    }
    fn stop(&mut self, name: &str) {
        self.wanted.remove(name);
        if let Some(pid) = self.children.get(name) {
            unsafe {
                libc::kill(*pid as i32, libc::SIGTERM);
            }
        }
    }
    fn want(&mut self, name: &str, on: bool) {
        if on {
            self.wanted.insert(name.to_string());
            self.spawn(name);
        } else if self.wanted.contains(name) || self.children.contains_key(name) {
            self.stop(name);
        }
    }

    fn front_game(&self) -> Option<&Win> {
        self.st.wins.iter().find(|w| w.game && w.fullscreen && !w.minimized && (w.activated || w.id == self.st.active))
    }

    /// The surfaces that should be running now.
    fn surfaces(&mut self) {
        let desk = self.st.posture == "desk";
        self.want("couch", true);
        self.want("bar", desk);
        self.want("dock", desk);
        let perf = self.st.cfg.overlay != "off" && self.front_game().is_some();
        self.want("perf", perf);
        self.perf_ticker(perf);
        self.want("guide", self.st.guide.up);
        self.want("notice", self.st.notice.is_some());
    }

    fn perf_ticker(&mut self, on: bool) {
        if on == self.perf_on.load(Ordering::Relaxed) {
            return;
        }
        self.perf_on.store(on, Ordering::Relaxed);
        if on {
            let flag = self.perf_on.clone();
            let tx = self.tx.clone();
            std::thread::spawn(move || {
                while flag.load(Ordering::Relaxed) {
                    if tx.send(Ev::PerfTick).is_err() {
                        return;
                    }
                    std::thread::sleep(Duration::from_secs(1));
                }
            });
        }
    }

    fn notice(&mut self, title: &str, body: &str) {
        self.notice_serial += 1;
        let serial = self.notice_serial;
        self.st.notice = Some(Notice { title: title.into(), body: body.into(), serial });
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(6));
            let _ = tx.send(Ev::NoticeEnd(serial));
        });
    }

    fn set_posture(&mut self, to: &str, why: &str) {
        if to != "couch" && to != "desk" {
            return;
        }
        if self.st.posture == to {
            if to == "couch" && !self.st.couch_up && self.front_game().is_none() {
                self.st.couch_up = true;
            }
            return;
        }
        log(&format!("posture: {to} ({why})"));
        self.st.posture = to.to_string();
        config::save_last_posture(to);
        if to == "desk" {
            self.st.couch_up = false;
            self.st.guide.up = false;
            // Windows the Couch made full screen go back to windows (and games too, when they follow the posture).
            let ids: Vec<String> = self.ours_fullscreen.drain().collect();
            for id in ids {
                let game = self.st.wins.iter().any(|w| w.id == id && w.game);
                if !game || self.st.cfg.games == "follow" {
                    self.comp_send(json!({"op": "fullscreen", "id": id, "on": false}));
                }
            }
            if why == "mouse" || why == "keys" {
                let how = if why == "mouse" { "You moved the mouse." } else { "You used the keyboard." };
                self.notice("Switched to Desk", &format!("{how} Everything stayed open. Press the Guide button to go back to the Couch."));
            }
        } else {
            self.st.couch_up = self.front_game().is_none();
            self.st.notice = None;
        }
        self.surfaces();
        self.broadcast();
    }

    fn comp_line(&mut self, v: Value) {
        match v.get("t").and_then(|t| t.as_str()) {
            Some("windows") => {
                self.raw_wins = v.get("windows").and_then(|w| w.as_array()).cloned().unwrap_or_default();
                self.st.active = v.get("active").and_then(|a| a.as_str()).unwrap_or("").to_string();
                self.rename_windows();
            }
            Some("input") => {
                let kind = v.get("kind").and_then(|k| k.as_str()).unwrap_or("");
                // Automatic only, and never while a game is in front (a game played with the keyboard stays full screen).
                if self.st.cfg.posture == "auto" && self.st.posture == "couch" && (self.front_game().is_none() || self.st.couch_up) && !self.st.guide.up {
                    match kind {
                        "mouse" => self.set_posture("desk", "mouse"),
                        "keys" => self.set_posture("desk", "keys"),
                        _ => {}
                    }
                }
            }
            Some("shot") => {
                let id = v.get("id").map(|i| i.as_str().map(String::from).unwrap_or_else(|| i.to_string())).unwrap_or_default();
                if v.get("ok").and_then(|o| o.as_bool()) == Some(true) {
                    if let Some(p) = v.get("path").and_then(|p| p.as_str()) {
                        if let Some(old) = self.pics.insert(id.clone(), p.to_string()) {
                            if old != p {
                                let _ = std::fs::remove_file(old);
                            }
                        }
                        for w in self.st.wins.iter_mut().filter(|w| w.id == id) {
                            w.pic = p.to_string();
                        }
                        self.broadcast();
                    }
                }
            }
            _ => {}
        }
    }

    fn rename_windows(&mut self) {
        let mut wins: Vec<Win> = self.raw_wins.iter().map(|w| model::name_window(w, &self.library)).collect();
        for w in wins.iter_mut() {
            if let Some(p) = self.pics.get(&w.id) {
                w.pic = p.clone();
            }
        }
        let ids: HashSet<String> = wins.iter().map(|w| w.id.clone()).collect();
        // New windows: games open the way Settings › Couch and Desk › Games says; anything opened on the Couch is shown.
        let new: Vec<Win> = wins.iter().filter(|w| !self.known_wins.contains(&w.id)).cloned().collect();
        let first = self.known_wins.is_empty() && self.started.elapsed() < Duration::from_secs(3);
        for w in &new {
            if w.game {
                self.seen_games.entry(w.id.clone()).or_insert_with(Instant::now);
            }
            if first {
                continue;
            }
            let couch = self.st.posture == "couch";
            // Only what comes to the front: a dialog or a tool window behind stays as it is.
            let front = w.activated || w.id == self.st.active;
            let full = if w.game { self.st.cfg.games == "fullscreen" || (self.st.cfg.games == "follow" && couch) } else { couch && front };
            if full && !w.fullscreen {
                self.comp_send(json!({"op": "fullscreen", "id": w.id, "on": true}));
                self.ours_fullscreen.insert(w.id.clone());
            }
            if couch && (front || w.game) {
                self.st.couch_up = false;
                self.st.guide.up = false;
            }
        }
        self.known_wins = ids.clone();
        self.seen_games.retain(|id, _| ids.contains(id));
        self.ours_fullscreen.retain(|id| ids.contains(id));
        self.pics.retain(|id, p| {
            let keep = ids.contains(id);
            if !keep {
                let _ = std::fs::remove_file(p);
            }
            keep
        });
        self.st.wins = wins;
        // Everything over the Couch closed (or went to the background): the Couch comes back.
        if self.st.posture == "couch" && !self.st.couch_up {
            let shown = self.st.wins.iter().any(|w| !w.minimized && (w.activated || w.id == self.st.active));
            if !shown {
                self.st.couch_up = true;
            }
        }
        if self.st.guide.up {
            self.refresh_guide();
        }
        self.send_frames();
        self.surfaces();
        self.broadcast();
    }

    /// Firehold's own title bars on the Desk (frames.rs): each window's name, a game's line.
    fn send_frames(&mut self) {
        let mut names = Map::new();
        for w in &self.st.wins {
            let (title, line) = if w.game {
                let mut parts = vec![if w.minimized { "paused".to_string() } else { "running".to_string() }];
                let fl = game_choice(&w.app_id, "frameLimit");
                if fl.parse::<u32>().is_ok() {
                    parts.push(format!("{fl} fps limit"));
                }
                if game_choice(&w.app_id, "vrr") == "on" {
                    parts.push("VRR".into());
                }
                (w.name.clone(), parts.join(" · "))
            } else if w.sub.is_empty() {
                (w.name.clone(), String::new())
            } else {
                (format!("{} — {}", w.name, w.sub), String::new())
            };
            names.insert(w.id.clone(), json!({"title": title, "game": w.game, "line": line}));
        }
        let v = json!({"op": "frames", "style": "firehold", "names": names});
        let key = v.to_string();
        if key != self.frames_sent && self.comp.is_some() {
            self.frames_sent = key;
            self.comp_send(v);
        }
    }

    fn refresh_guide(&mut self) {
        let front = self.front_game().cloned().or_else(|| self.st.wins.iter().find(|w| w.game && !w.minimized).cloned());
        let mut g = Guide { up: self.st.guide.up, screenshot: self.st.guide.screenshot.clone(), ..Guide::default() };
        if let Some(w) = front {
            g.game = w.name.clone();
            g.game_id = w.game_id.clone();
            g.win = w.id.clone();
            let secs = self.seen_games.get(&w.id).map(|t| t.elapsed().as_secs()).unwrap_or(0);
            if let Some(st) = game_ask("status") {
                if let Some(cg) = st.get("game").filter(|x| x.is_object()) {
                    g.comp = true;
                    let fl = cg.get("frameLimit").cloned().unwrap_or(json!("off"));
                    g.frame_limit = match fl {
                        Value::Number(n) => n.to_string(),
                        Value::String(s) if s == "screen" => "Screen".into(),
                        _ => "Off".into(),
                    };
                    let sc = cg.get("scaling").and_then(|s| s.as_str()).unwrap_or("off").to_string();
                    let h = cg.get("renderSize").and_then(|r| r.get(1)).and_then(|h| h.as_i64()).unwrap_or(0);
                    g.scaling = if sc == "off" {
                        "Off".into()
                    } else {
                        let mut c = sc.chars();
                        format!("{}{} · {h}p", c.next().map(|f| f.to_uppercase().to_string()).unwrap_or_default(), c.as_str())
                    };
                    g.vrr = cg.get("vrr").and_then(|v| v.as_bool()).unwrap_or(true);
                    g.tearing = cg.get("tearing").and_then(|v| v.as_bool()).unwrap_or(false);
                    g.scaling_note = cg.get("scalingNote").and_then(|n| n.as_str()).unwrap_or("").to_string();
                    let s = cg.get("seconds").and_then(|s| s.as_u64()).unwrap_or(secs);
                    g.playing = model::playing_for(s);
                }
            }
            if !g.comp {
                g.playing = model::playing_for(secs);
                g.frame_limit = "Not yet".into();
                g.scaling = "Not yet".into();
                g.vrr = game_choice(&w.app_id, "vrr") == "on";
                g.tearing = game_choice(&w.app_id, "tearing") == "on";
            }
        }
        g.volume = sys::volume();
        g.brightness = sys::brightness();
        g.power = match (sys::power_profile(), sys::watts()) {
            (Some(p), Some(w)) => format!("{p} · {} W", w.round() as i64),
            (Some(p), None) => p,
            (None, Some(w)) => format!("{} W", w.round() as i64),
            (None, None) => "Not here".into(),
        };
        self.st.guide = g;
    }

    fn perf_line(&self) -> String {
        let mut p = Vec::new();
        if let Some(st) = game_ask("status") {
            if let Some(f) = st.get("game").and_then(|g| g.get("pacedFps")).and_then(|f| f.as_f64()).filter(|f| *f > 0.0) {
                p.push(format!("{} fps", f.round() as i64));
                p.push(format!("{:.1} ms", 1000.0 / f));
            }
        }
        let (busy, temp) = sys::gpu();
        if let Some(b) = busy {
            p.push(format!("GPU {b}%"));
        }
        if let Some(t) = temp {
            p.push(format!("{t} °C"));
        }
        if let Some(w) = sys::watts() {
            p.push(format!("{} W", w.round() as i64));
        }
        if p.is_empty() {
            "Firehold".into()
        } else {
            p.join(" · ")
        }
    }

    fn guide_button(&mut self) {
        if self.st.guide.up {
            // Twice: everything open (over the game too; the game keeps running under it).
            self.st.guide.up = false;
            if self.st.posture != "couch" {
                self.set_posture("couch", "Guide button");
            }
            self.st.couch_up = true;
            self.send_surface("couch", &json!({"t": "cmd", "cmd": "page", "arg": "switcher"}));
        } else if self.front_game().is_some() {
            self.st.guide.up = true;
            self.refresh_guide();
        } else if self.st.posture == "desk" {
            self.set_posture("couch", "Guide button");
            self.st.couch_up = true;
            self.send_surface("couch", &json!({"t": "cmd", "cmd": "page", "arg": "home"}));
        } else if self.st.cfg.guide == "open" {
            self.st.couch_up = true;
            self.send_surface("couch", &json!({"t": "cmd", "cmd": "page", "arg": "switcher"}));
        } else {
            self.st.guide.up = true;
            self.refresh_guide();
        }
        self.surfaces();
        self.broadcast();
    }

    fn pad(&mut self, key: &'static str) {
        if key == "guide" {
            return self.guide_button();
        }
        if self.st.guide.up {
            self.send_surface("guide", &json!({"t": "nav", "key": key}));
            return;
        }
        if self.front_game().is_some() && !self.st.couch_up {
            return; // the game has the controller
        }
        if self.st.posture == "desk" {
            if self.st.cfg.posture == "auto" {
                self.set_posture("couch", "controller");
            }
            return;
        }
        if !self.st.couch_up {
            self.st.couch_up = true;
            self.broadcast();
        }
        self.send_surface("couch", &json!({"t": "nav", "key": key}));
    }

    fn pictures(&mut self, on: bool) {
        if on == self.pics_on.load(Ordering::Relaxed) {
            return;
        }
        self.pics_on.store(on, Ordering::Relaxed);
        if on {
            let flag = self.pics_on.clone();
            let tx = self.tx.clone();
            std::thread::spawn(move || {
                while flag.load(Ordering::Relaxed) {
                    if tx.send(Ev::PicTick).is_err() {
                        return;
                    }
                    std::thread::sleep(Duration::from_secs(3));
                }
            });
        }
    }
    fn take_pictures(&mut self) {
        let dir = runtime().join("firehold-shell");
        let _ = std::fs::create_dir_all(&dir);
        self.pic_n += 1;
        let ids: Vec<String> = self.st.wins.iter().map(|w| w.id.clone()).collect();
        for id in ids {
            if !id.chars().all(|c| c.is_ascii_digit()) {
                continue;
            }
            let path = dir.join(format!("pic-{id}-{}.png", self.pic_n));
            self.comp_send(json!({"op": "shot", "id": id, "path": path.display().to_string(), "max": 640}));
        }
    }

    fn reload_library(&mut self, force: bool) {
        if self.library_loading || (!force && self.library_at.is_some_and(|t| t.elapsed() < Duration::from_secs(120))) {
            return;
        }
        self.library_loading = true;
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let (g, note) = load_library();
            let _ = tx.send(Ev::Library(g, note));
        });
    }

    fn send_library(&mut self, id: Option<u64>) {
        let v = json!({"t": "library", "games": self.library});
        let a = json!({"t": "apps", "apps": self.apps});
        match id {
            Some(i) => {
                self.send_to(i, &v);
                self.send_to(i, &a);
            }
            None => {
                for s in ["couch", "dock", "guide", "settings"] {
                    self.send_surface(s, &v);
                }
            }
        }
    }

    fn window_id(v: &Value) -> Option<String> {
        let id = match v.get("id")? {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            _ => return None,
        };
        id.chars().all(|c| c.is_ascii_digit()).then_some(id)
    }

    fn activate(&mut self, id: &str) {
        let Some(w) = self.st.wins.iter().find(|w| w.id == id).cloned() else { return };
        self.comp_send(json!({"op": "activate", "id": id}));
        if self.st.posture == "couch" {
            self.st.couch_up = false;
            let full = !w.game || self.st.cfg.games != "window";
            if full && !w.fullscreen {
                self.comp_send(json!({"op": "fullscreen", "id": id, "on": true}));
                self.ours_fullscreen.insert(id.to_string());
            }
        }
        self.surfaces();
        self.broadcast();
    }

    fn play(&mut self, game_id: &str, windowed: bool) {
        if let Some(w) = self.st.wins.iter().find(|w| w.game && w.game_id == game_id).cloned() {
            if windowed {
                self.set_posture("desk", "Open in a window");
                self.comp_send(json!({"op": "fullscreen", "id": w.id, "on": false}));
                self.comp_send(json!({"op": "activate", "id": w.id}));
            } else {
                self.activate(&w.id);
            }
            return;
        }
        if !model::valid_id(game_id) {
            return;
        }
        if windowed {
            self.set_posture("desk", "Open in a window");
        }
        if let Some(app) = game_id.strip_prefix("winapp:") {
            if let Some(mut h) = helper("hearth-winapps") {
                h.push("run".into());
                h.push(app.into());
                let a: Vec<&str> = h.iter().map(|s| s.as_str()).collect();
                sys::detach(a[0], &a[1..]);
            }
        } else if let Some(mut h) = helper("hearth-games") {
            h.push("play".into());
            h.push(game_id.into());
            let a: Vec<&str> = h.iter().map(|s| s.as_str()).collect();
            sys::detach(a[0], &a[1..]);
        } else if let Some(app) = hearth_app() {
            sys::detach(&app, &["--play-now", game_id]);
        }
        log(&format!("play {game_id}"));
    }

    fn reply(&mut self, from: u64, ok: bool, extra: Value) {
        let mut r = json!({"t": "reply", "ok": ok});
        if let (Some(o), Some(e)) = (r.as_object_mut(), extra.as_object()) {
            for (k, v) in e {
                o.insert(k.clone(), v.clone());
            }
        }
        self.send_to(from, &r);
    }

    fn command(&mut self, from: u64, v: Value) {
        let op = v.get("op").and_then(|o| o.as_str()).unwrap_or("").to_string();
        let s = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
        let mut ok = true;
        let mut extra = json!({});
        match op.as_str() {
            "hello" => {
                let surface = s("surface");
                if let Some(c) = self.clients.get_mut(&from) {
                    c.surface = surface.clone();
                }
                self.hello.insert(surface.clone());
                self.sent.remove(&from);
                if ["couch", "dock", "guide", "settings"].contains(&surface.as_str()) {
                    self.send_library(Some(from));
                }
                self.broadcast();
                if !self.up_written && self.hello.contains("couch") && (self.st.posture == "couch" || (self.hello.contains("bar") && self.hello.contains("dock"))) {
                    self.up_written = true;
                    let _ = std::fs::write(runtime().join("firehold-shell-up"), format!("{}\n", self.started.elapsed().as_secs()));
                    log(&format!("up after {} s", self.started.elapsed().as_secs()));
                }
                return;
            }
            "state" => {
                let mut st = serde_json::to_value(&self.st).unwrap_or(Value::Null);
                if let Some(o) = st.as_object_mut() {
                    o.insert("t".into(), json!("state"));
                    o.insert("surfaces".into(), json!(self.children.keys().collect::<Vec<_>>()));
                    o.insert("hello".into(), json!(self.hello.iter().collect::<Vec<_>>()));
                    o.insert("games".into(), json!(self.library.len()));
                    o.insert("comp".into(), json!(self.comp.is_some()));
                    o.insert("pids".into(), json!(self.children));
                }
                self.send_to(from, &st);
                return;
            }
            "library" => {
                let v = json!({"t": "library", "games": self.library});
                self.send_to(from, &v);
                return;
            }
            "posture" => {
                let to = s("to");
                self.set_posture(&to, "asked");
                if to == "couch" {
                    self.st.couch_up = true;
                    self.send_surface("couch", &json!({"t": "cmd", "cmd": "page", "arg": "home"}));
                }
            }
            "home" => {
                self.st.guide.up = false;
                if self.st.posture != "couch" {
                    self.set_posture("couch", "Home");
                }
                self.st.couch_up = true;
                self.send_surface("couch", &json!({"t": "cmd", "cmd": "page", "arg": "home"}));
                self.reload_library(false);
            }
            "tab" | "page" => {
                let arg = s(if op == "tab" { "tab" } else { "page" });
                if op == "page" && arg == "settings" {
                    self.want("settings", true);
                } else {
                    if self.st.posture != "couch" {
                        self.set_posture("couch", "asked");
                    }
                    self.st.couch_up = true;
                    self.st.guide.up = false;
                    self.send_surface("couch", &json!({"t": "cmd", "cmd": op, "arg": arg}));
                }
            }
            "couchPage" => {
                // The Couch says which page it shows: Everything open wants the windows' pictures.
                self.pictures(s("page") == "switcher" && self.st.couch_up);
                if s("page") == "home" {
                    self.reload_library(false);
                }
                return;
            }
            "guide" => self.guide_button(),
            "guideClose" => {
                self.st.guide.up = false;
            }
            "pad" => {
                let k = s("key");
                let key: Option<&'static str> =
                    ["up", "down", "left", "right", "a", "b", "x", "y", "lb", "rb", "lt", "rt", "guide", "start"].into_iter().find(|x| *x == k);
                match key {
                    Some(k) => self.pad(k),
                    None => ok = false,
                }
            }
            "input" => {
                // As firehold-comp sends it (tests): {"op": "input", "kind": "mouse"}
                self.comp_line(json!({"t": "input", "kind": s("kind")}));
            }
            "activate" | "close" | "minimize" => match Self::window_id(&v) {
                Some(id) if op == "activate" => self.activate(&id),
                Some(id) => self.comp_send(json!({"op": op, "id": id, "on": true})),
                None => ok = false,
            },
            "fullscreen" => match Self::window_id(&v) {
                Some(id) => {
                    let cur = self.st.wins.iter().find(|w| w.id == id).map(|w| w.fullscreen).unwrap_or(false);
                    let on = v.get("on").and_then(|o| o.as_bool()).unwrap_or(!cur);
                    self.comp_send(json!({"op": "fullscreen", "id": id, "on": on}));
                    if !on {
                        self.ours_fullscreen.remove(&id);
                    }
                }
                None => ok = false,
            },
            "play" => self.play(&s("id"), false),
            "openWindowed" => self.play(&s("id"), true),
            "launch" => {
                let id = s("app");
                if model::valid_id(&id) {
                    sys::launch_app(&id);
                    if self.st.posture == "couch" {
                        self.st.couch_up = false;
                    }
                } else {
                    ok = false;
                }
            }
            "store" => {
                for app in ["io.github.kolunmi.Bazaar", "org.kde.discover"] {
                    if sys::which(if app.contains("Bazaar") { "bazaar" } else { "plasma-discover" }) {
                        sys::launch_app(app);
                        break;
                    }
                }
            }
            "gamePage" => {
                // X on a game: Hearth's own page for it (options, achievements, saves), in a window.
                let id = s("id");
                match hearth_app() {
                    Some(app) if model::valid_id(&id) && !id.starts_with("winapp:") => {
                        sys::detach(&app, &["--windowed", "--game-page", &id]);
                        if self.st.posture == "couch" {
                            self.st.couch_up = false;
                        }
                    }
                    _ => ok = false,
                }
            }
            "pictures" => self.pictures(v.get("on").and_then(|o| o.as_bool()).unwrap_or(false)),
            "set" => {
                let key = s("key");
                let value = v.get("value").cloned().unwrap_or(Value::Null);
                match config::set(&key, value) {
                    Ok(changed) => {
                        extra = json!({"changed": changed});
                        self.st.cfg = Couch::read();
                        if key == "posture" && self.st.cfg.posture != "auto" {
                            let p = self.st.cfg.posture.clone();
                            self.set_posture(&p, "Settings › Couch and Desk");
                        }
                    }
                    Err(e) => {
                        ok = false;
                        extra = json!({"error": e});
                    }
                }
            }
            "choose" => match config::choose(&s("what")) {
                Ok(changed) => {
                    extra = json!({"changed": changed});
                    self.st.desktop = config::desktop_choice();
                }
                Err(e) => {
                    ok = false;
                    extra = json!({"error": e});
                }
            },
            "gameRow" => {
                let app = self.front_game().or_else(|| self.st.wins.iter().find(|w| w.game)).map(|w| w.app_id.clone()).unwrap_or_default();
                match game_step(&app, &s("key"), s("dir") != "down") {
                    Ok(()) => {
                        self.refresh_guide();
                        self.send_frames();
                    }
                    Err(e) => {
                        ok = false;
                        extra = json!({"error": e});
                    }
                }
            }
            "guideTile" => {
                match s("what").as_str() {
                    "screenshot" => {
                        let dir = config::home().join("Pictures/Screenshots");
                        let _ = std::fs::create_dir_all(&dir);
                        let f = dir.join(format!("Screenshot {}.png", chrono::Local::now().format("%Y-%m-%d %H-%M-%S")));
                        // The Guide steps aside for the picture, then comes back.
                        let fs = f.display().to_string();
                        let tx = self.tx.clone();
                        self.st.guide.up = false;
                        self.surfaces();
                        std::thread::spawn(move || {
                            std::thread::sleep(Duration::from_millis(500));
                            let _ = Command::new("grim").arg(&fs).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status();
                            let _ = tx.send(Ev::Client(0, json!({"op": "guideBack", "shot": fs})));
                        });
                    }
                    "keyboard" => {
                        for k in ["wvkbd-mobintl", "squeekboard"] {
                            if sys::which(k) {
                                sys::detach(k, &[]);
                                break;
                            }
                        }
                    }
                    "volumeUp" | "volumeDown" => {
                        let cur = sys::volume().unwrap_or(50);
                        sys::set_volume(cur + if s("what") == "volumeUp" { 5 } else { -5 });
                        std::thread::sleep(Duration::from_millis(80));
                        self.refresh_guide();
                    }
                    "brightnessUp" | "brightnessDown" => {
                        let cur = sys::brightness().unwrap_or(50);
                        sys::set_brightness(cur + if s("what") == "brightnessUp" { 5 } else { -5 });
                        std::thread::sleep(Duration::from_millis(80));
                        self.refresh_guide();
                    }
                    "powerUp" | "powerDown" => {
                        sys::step_power(s("what") == "powerUp");
                        std::thread::sleep(Duration::from_millis(150));
                        self.refresh_guide();
                    }
                    _ => ok = false,
                }
            }
            "guideBack" => {
                self.st.guide.up = true;
                self.st.guide.screenshot = s("shot");
                self.refresh_guide();
            }
            "overlayStep" => {
                let order = ["off", "basic", "full"];
                let i = order.iter().position(|o| *o == self.st.cfg.overlay).unwrap_or(0) as i64;
                let n = order[((i + if s("dir") == "down" { -1 } else { 1 }).rem_euclid(3)) as usize];
                let _ = config::set("overlay", json!(n));
                self.st.cfg = Couch::read();
            }
            "gameToDesk" => {
                if let Some(id) = self.front_game().map(|w| w.id.clone()) {
                    self.st.guide.up = false;
                    self.set_posture("desk", "Put it in a window on the Desk");
                    self.comp_send(json!({"op": "fullscreen", "id": id, "on": false}));
                    self.ours_fullscreen.remove(&id);
                }
            }
            "gameHome" => {
                self.st.guide.up = false;
                if self.st.posture != "couch" {
                    self.set_posture("couch", "Home");
                }
                self.st.couch_up = true;
                self.send_surface("couch", &json!({"t": "cmd", "cmd": "page", "arg": "home"}));
            }
            "quitGame" => {
                if let Some(id) = self.front_game().or_else(|| self.st.wins.iter().find(|w| w.game)).map(|w| w.id.clone()) {
                    self.comp_send(json!({"op": "close", "id": id}));
                }
                self.st.guide.up = false;
                self.st.couch_up = self.st.posture == "couch";
            }
            "leave" => {
                // Back to Game Mode (nimbus-mode, which ends this session the way it ends Plasma), or to KDE Plasma
                // for this sign-in (the session starts Plasma when Firehold ends by itself).
                match s("to").as_str() {
                    "game" => sys::detach("nimbus-mode", &["game"]),
                    "plasma" => {
                        let _ = std::fs::write(runtime().join("firehold-leave"), "plasma\n");
                        if let Ok(pid) = std::env::var("HEARTH_DESKTOP_PID").unwrap_or_default().parse::<i32>() {
                            unsafe {
                                libc::kill(pid, libc::SIGTERM);
                            }
                        }
                    }
                    _ => ok = false,
                }
            }
            "quit" => {
                self.reply(from, true, json!({}));
                for pid in self.children.values() {
                    unsafe {
                        libc::kill(*pid as i32, libc::SIGTERM);
                    }
                }
                std::process::exit(0);
            }
            _ => {
                ok = false;
                extra = json!({"error": format!("unknown op {op:?}")});
            }
        }
        self.surfaces();
        self.broadcast();
        if from != 0 {
            self.reply(from, ok, extra);
        }
    }

    fn handle(&mut self, ev: Ev) {
        match ev {
            Ev::CompUp(s) => {
                log("firehold-comp's window IPC: connected");
                self.comp = Some(s);
                self.frames_sent.clear();
                self.send_frames();
            }
            Ev::Comp(v) => self.comp_line(v),
            Ev::CompGone => {
                log("firehold-comp's window IPC: gone");
                self.comp = None;
            }
            Ev::ClientNew(id, s) => {
                self.clients.insert(id, Client { stream: s, surface: String::new() });
            }
            Ev::Client(id, v) => self.command(id, v),
            Ev::ClientGone(id) => {
                self.clients.remove(&id);
                self.sent.remove(&id);
            }
            Ev::Pad(k) => self.pad(k),
            Ev::Tick => {
                let (c, d) = sys::clock();
                self.st.clock = c;
                self.st.date = d;
                self.st.net = sys::network();
                self.st.battery = sys::battery();
                if self.st.guide.up {
                    self.refresh_guide();
                }
                self.broadcast();
            }
            Ev::PicTick => {
                if self.pics_on.load(Ordering::Relaxed) {
                    self.take_pictures();
                }
            }
            Ev::PerfTick => {
                if self.perf_on.load(Ordering::Relaxed) {
                    self.st.perf.line = self.perf_line();
                    self.broadcast();
                }
            }
            Ev::NoticeEnd(serial) => {
                if self.st.notice.as_ref().is_some_and(|n| n.serial == serial) {
                    self.st.notice = None;
                    self.surfaces();
                    self.broadcast();
                }
            }
            Ev::Library(g, note) => {
                self.library_loading = false;
                self.library_at = Some(Instant::now());
                log(&format!("library: {} games", g.len()));
                self.library = g;
                self.st.library_note = note;
                self.send_library(None);
                self.rename_windows();
            }
            Ev::ChildExit(name, pid, code) => {
                if self.children.get(&name) == Some(&pid) {
                    self.children.remove(&name);
                }
                self.hello.remove(&name);
                if code != 0 && code != 143 {
                    log(&format!("{name} ended (code {code})"));
                }
                if name == "guide" && self.st.guide.up && code == 0 {
                    // Closed by itself (B): the Guide is down.
                    self.st.guide.up = false;
                    self.wanted.remove("guide");
                    self.broadcast();
                }
                if name == "settings" {
                    self.wanted.remove("settings");
                }
                if self.wanted.contains(&name) {
                    let tx = self.tx.clone();
                    std::thread::spawn(move || {
                        std::thread::sleep(Duration::from_millis(800));
                        let _ = tx.send(Ev::Client(0, json!({"op": "respawn"})));
                    });
                }
            }
        }
    }
}

// ------------------------------------------------------------------------------------- the threads

fn comp_thread(tx: Sender<Ev>) {
    let path = runtime().join(format!("firehold-wm-{}.sock", wayland_name()));
    let mut wait = Duration::from_millis(250);
    loop {
        match UnixStream::connect(&path) {
            Ok(s) => {
                wait = Duration::from_millis(250);
                let Ok(w) = s.try_clone() else { return };
                let _ = w.set_write_timeout(Some(Duration::from_millis(200)));
                let mut w2 = match w.try_clone() {
                    Ok(x) => x,
                    Err(_) => return,
                };
                if w2.write_all(b"{\"op\":\"watch\",\"input\":true}\n").is_err() {
                    continue;
                }
                if tx.send(Ev::CompUp(w)).is_err() {
                    return;
                }
                for line in BufReader::new(s).lines() {
                    let Ok(line) = line else { break };
                    if let Ok(v) = serde_json::from_str::<Value>(&line) {
                        if tx.send(Ev::Comp(v)).is_err() {
                            return;
                        }
                    }
                }
                if tx.send(Ev::CompGone).is_err() {
                    return;
                }
            }
            Err(_) => {}
        }
        std::thread::sleep(wait);
        wait = (wait * 2).min(Duration::from_secs(5));
    }
}

fn clock_thread(tx: Sender<Ev>) {
    loop {
        std::thread::sleep(Duration::from_secs(sys::secs_to_next_minute()) + Duration::from_millis(200));
        if tx.send(Ev::Tick).is_err() {
            return;
        }
    }
}

/// The controller: presses as the Couch's keys. Blocks in the kernel between presses (gilrs' blocking wait).
fn pad_thread(tx: Sender<Ev>) {
    use gilrs::{Axis, Button, EventType};
    let mut g = match gilrs::Gilrs::new() {
        Ok(g) => g,
        Err(e) => {
            log(&format!("no controller reader ({e}); the keyboard and mouse still work"));
            return;
        }
    };
    let mut stick: [i8; 2] = [0, 0];
    loop {
        let Some(ev) = g.next_event_blocking(None) else { continue };
        let key: Option<&'static str> = match ev.event {
            EventType::ButtonPressed(b, _) => match b {
                Button::South => Some("a"),
                Button::East => Some("b"),
                Button::West => Some("x"),
                Button::North => Some("y"),
                Button::LeftTrigger => Some("lb"),
                Button::RightTrigger => Some("rb"),
                Button::LeftTrigger2 => Some("lt"),
                Button::RightTrigger2 => Some("rt"),
                Button::DPadUp => Some("up"),
                Button::DPadDown => Some("down"),
                Button::DPadLeft => Some("left"),
                Button::DPadRight => Some("right"),
                Button::Mode => Some("guide"),
                Button::Start => Some("start"),
                _ => None,
            },
            EventType::AxisChanged(a, v, _) => {
                let (i, neg, pos) = match a {
                    Axis::LeftStickX => (0, "left", "right"),
                    Axis::LeftStickY => (1, "down", "up"),
                    _ => continue,
                };
                let now: i8 = if v > 0.6 { 1 } else if v < -0.6 { -1 } else { 0 };
                let was = stick[i];
                stick[i] = if v.abs() < 0.3 { 0 } else if now != 0 { now } else { was };
                if now != 0 && now != was {
                    Some(if now > 0 { pos } else { neg })
                } else {
                    None
                }
            }
            _ => None,
        };
        if let Some(k) = key {
            if tx.send(Ev::Pad(k)).is_err() {
                return;
            }
        }
    }
}

fn listen_thread(tx: Sender<Ev>, l: UnixListener) {
    let mut next: u64 = 1;
    for s in l.incoming() {
        let Ok(s) = s else { continue };
        next += 1;
        let id = next;
        let Ok(w) = s.try_clone() else { continue };
        let _ = w.set_write_timeout(Some(Duration::from_millis(300)));
        if tx.send(Ev::ClientNew(id, w)).is_err() {
            return;
        }
        let tx2 = tx.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(s).lines() {
                let Ok(line) = line else { break };
                let t = line.trim();
                if t.is_empty() {
                    continue;
                }
                let v = if t.starts_with('{') {
                    serde_json::from_str::<Value>(t).unwrap_or(json!({"op": "bad"}))
                } else {
                    let mut it = t.split_whitespace();
                    let op = it.next().unwrap_or("");
                    let rest: Vec<&str> = it.collect();
                    let mut o = json!({"op": op});
                    // `ctl tab library`, `ctl posture desk`, `ctl pad a`, `ctl page switcher`
                    if let Some(a) = rest.first() {
                        let k = match op {
                            "tab" => "tab",
                            "page" => "page",
                            "posture" => "to",
                            "pad" => "key",
                            "play" | "openWindowed" => "id",
                            "leave" => "to",
                            "choose" => "what",
                            _ => "arg",
                        };
                        o[k] = json!(a);
                    }
                    o
                };
                if tx2.send(Ev::Client(id, v)).is_err() {
                    return;
                }
            }
            let _ = tx2.send(Ev::ClientGone(id));
        });
    }
}

pub fn run() -> i32 {
    let Some(sock) = proto::socket_path() else {
        log("XDG_RUNTIME_DIR isn't set");
        return 2;
    };
    let _ = std::fs::remove_file(&sock);
    let _ = std::fs::remove_file(runtime().join("firehold-shell-up"));
    // nimbus-session watches this: the hub gone means the compositor is ended too (a desktop with no shell is no use).
    let _ = std::fs::write(runtime().join("firehold-shell-hub.pid"), format!("{}\n", std::process::id()));
    let listener = match UnixListener::bind(&sock) {
        Ok(l) => l,
        Err(e) => {
            log(&format!("{}: {e}", sock.display()));
            return 1;
        }
    };
    let (tx, rx): (Sender<Ev>, Receiver<Ev>) = channel();
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("firehold-shell"));
    let cfg = Couch::read();
    let posture = match cfg.posture.as_str() {
        "couch" | "desk" => cfg.posture.clone(),
        _ => match cfg.start_in.as_str() {
            "couch" => "couch".into(),
            "last" => config::last_posture().unwrap_or_else(|| "desk".into()),
            _ => "desk".into(),
        },
    };
    let (clock, date) = sys::clock();
    let st = State {
        couch_up: posture == "couch",
        posture,
        clock,
        date,
        net: sys::network(),
        battery: sys::battery(),
        initial: sys::initial(),
        cfg,
        desktop: config::desktop_choice(),
        ..State::default()
    };
    log(&format!("starting in {} ({})", st.posture, sock.display()));
    let mut hub = Hub {
        tx: tx.clone(),
        exe,
        comp: None,
        clients: HashMap::new(),
        st,
        raw_wins: Vec::new(),
        library: Vec::new(),
        library_at: None,
        library_loading: false,
        apps: sys::apps(),
        children: HashMap::new(),
        restarts: HashMap::new(),
        wanted: HashSet::new(),
        hello: HashSet::new(),
        up_written: false,
        pics_on: Arc::new(AtomicBool::new(false)),
        perf_on: Arc::new(AtomicBool::new(false)),
        pic_n: 0,
        pics: HashMap::new(),
        seen_games: HashMap::new(),
        known_wins: HashSet::new(),
        ours_fullscreen: HashSet::new(),
        frames_sent: String::new(),
        notice_serial: 0,
        sent: HashMap::new(),
        started: Instant::now(),
    };
    {
        let t = tx.clone();
        std::thread::spawn(move || listen_thread(t, listener));
        let t = tx.clone();
        std::thread::spawn(move || comp_thread(t));
        let t = tx.clone();
        std::thread::spawn(move || clock_thread(t));
        if std::env::var_os("FIREHOLD_NO_PAD").is_none() {
            let t = tx.clone();
            std::thread::spawn(move || pad_thread(t));
        }
    }
    // A clean end on SIGTERM: the surfaces go with the hub.
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_IGN);
    }
    hub.reload_library(true);
    hub.surfaces();
    drop(tx);
    while let Ok(ev) = rx.recv() {
        if let Ev::Client(0, v) = &ev {
            if v.get("op").and_then(|o| o.as_str()) == Some("respawn") {
                let names: Vec<String> = hub.wanted.iter().cloned().collect();
                for n in names {
                    hub.spawn(&n);
                }
                continue;
            }
        }
        hub.handle(ev);
    }
    0
}

/// `firehold-shell ctl LINE`: one command to the running hub, its answer printed.
pub fn ctl(args: &[String]) -> i32 {
    let Some(p) = proto::socket_path() else {
        eprintln!("firehold-shell ctl: XDG_RUNTIME_DIR isn't set");
        return 2;
    };
    let mut s = match UnixStream::connect(&p) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("firehold-shell ctl: {}: {e}", p.display());
            return 1;
        }
    };
    let _ = s.set_read_timeout(Some(Duration::from_secs(5)));
    let line = args.join(" ");
    if s.write_all(format!("{line}\n").as_bytes()).is_err() {
        return 1;
    }
    let mut r = BufReader::new(s);
    let mut out = String::new();
    loop {
        out.clear();
        match r.read_line(&mut out) {
            Ok(0) | Err(_) => return 1,
            Ok(_) => {
                let v: Value = serde_json::from_str(out.trim()).unwrap_or(Value::Null);
                let t = v.get("t").and_then(|t| t.as_str()).unwrap_or("");
                if t == "reply" || t == "state" || t == "library" {
                    println!("{}", out.trim());
                    return if v.get("ok").and_then(|o| o.as_bool()) == Some(false) { 1 } else { 0 };
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_rows_step_and_wrap() {
        let d = tempfile::tempdir().unwrap();
        std::env::set_var("FIREHOLD_GAMES", d.path().join("g.json"));
        assert_eq!(game_choice("ember", "frameLimit"), "off");
        assert_eq!(game_choice("ember", "vrr"), "on");
        assert_eq!(game_choice("ember", "tearing"), "off");
        game_step("ember", "frameLimit", true).unwrap();
        assert_eq!(game_choice("ember", "frameLimit"), "30");
        game_step("ember", "frameLimit", false).unwrap();
        game_step("ember", "frameLimit", false).unwrap();
        assert_eq!(game_choice("ember", "frameLimit"), "screen");
        game_step("ember", "vrr", true).unwrap();
        assert_eq!(game_choice("ember", "vrr"), "off");
        game_step("ember", "scaling", true).unwrap();
        assert_eq!(game_choice("ember", "scaling"), "quality");
        let v: Value = serde_json::from_str(&std::fs::read_to_string(d.path().join("g.json")).unwrap()).unwrap();
        assert_eq!(v["games"]["ember"]["frameLimit"], "screen");
        assert_eq!(v["games"]["ember"]["vrr"], false);
        assert!(game_step("", "vrr", true).is_err());
        assert!(game_step("ember", "nope", true).is_err());
        std::env::remove_var("FIREHOLD_GAMES");
    }
}
