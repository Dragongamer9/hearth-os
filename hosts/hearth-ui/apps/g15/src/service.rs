//! `hearth-g15 service`: the small user service (hearth-g15.service), started by udev when the keyboard's G-keys
//! appear (70-hearth-g15.rules) and at sign-in when it's already plugged in. It ends by itself when the keyboard is
//! unplugged. Light at rest: it sleeps in poll() on the keyboard and on a few files; the screen is redrawn at most
//! once a second (once a minute for the clock), only when something on it changed, and not at all while the
//! screen's light is off or the PC is locked. Nothing here can stop Game Mode or the desktop from starting: nothing
//! waits for it, and every program it runs has a time limit.

use crate::actions::{self, Action, RecOut, Recorder};
use crate::config;
use crate::device::{self, Device};
use crate::input::{self, Ev, Output, ReadResult};
use crate::keys::GKey;
use crate::lcd::{self, screen, Frame};
use crate::leds;
use crate::root::{self, read, Root};
use crate::values::{Reading, Sampler};
use serde_json::{json, Value};
use std::io::{BufRead, Write};
use std::os::unix::io::{AsRawFd, RawFd};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// The exit code when the keyboard can't be read (its udev rule is missing): the unit doesn't restart for it.
pub const NO_ACCESS: i32 = 3;

fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

#[derive(Default, Clone)]
struct Song {
    known: bool,
    playing: bool,
    title: String,
    artist: String,
    pos: f64,
    len: f64,
    at: Option<Instant>,
}

impl Song {
    fn pos_now(&self) -> f64 {
        let extra = if self.playing { self.at.map(|t| t.elapsed().as_secs_f64()).unwrap_or(0.0) } else { 0.0 };
        (self.pos + extra).min(if self.len > 0.0 { self.len } else { f64::MAX })
    }
}

/// One line of `playerctl --follow metadata --format …`: status, position (µs), length (µs), artist, title.
fn parse_song(line: &str) -> Song {
    let f: Vec<&str> = line.split('\t').collect();
    if f.len() < 5 || f[0].trim().is_empty() {
        return Song::default();
    }
    let us = |s: &str| s.trim().parse::<f64>().map(|v| v / 1e6).unwrap_or(0.0);
    Song { known: true, playing: f[0].trim() == "Playing", pos: us(f[1]), len: us(f[2]), artist: f[3].trim().to_string(), title: f[4].trim().to_string(), at: Some(Instant::now()) }
}

fn mmss(s: f64) -> String {
    let s = s.max(0.0) as u64;
    format!("{}:{:02}", s / 60, s % 60)
}

#[derive(Clone, Default)]
struct Alert {
    app: String,
    title: String,
    body: String,
}

/// The newest notification in Mantel's history (~/.local/state/nimbus/notifications.json, "n:<number>" keys).
fn newest_alert(p: &Path) -> Option<(u64, Alert)> {
    let v: Value = serde_json::from_str(&std::fs::read_to_string(p).ok()?).ok()?;
    let (k, h) = v.as_object()?.iter().filter_map(|(k, h)| Some((k.strip_prefix("n:")?.parse::<u64>().ok()?, h))).max_by_key(|(k, _)| *k)?;
    let s = |x: &str| h.get(x).and_then(|v| v.as_str()).unwrap_or("").to_string();
    let app = s("app");
    Some((k, Alert { app: if app.is_empty() { "HEARTH".into() } else { app.to_uppercase() }, title: s("summary"), body: s("body").replace('\n', " ") }))
}

struct Inotify {
    fd: RawFd,
}

impl Inotify {
    fn new() -> Option<Inotify> {
        let fd = unsafe { libc::inotify_init1(libc::IN_NONBLOCK | libc::IN_CLOEXEC) };
        (fd >= 0).then_some(Inotify { fd })
    }
    fn watch(&self, dir: &Path) {
        let _ = std::fs::create_dir_all(dir);
        if let Ok(c) = std::ffi::CString::new(dir.as_os_str().as_encoded_bytes()) {
            unsafe {
                libc::inotify_add_watch(self.fd, c.as_ptr(), libc::IN_CLOSE_WRITE | libc::IN_MOVED_TO);
            }
        }
    }
    /// The names that changed.
    fn names(&self) -> Vec<String> {
        let mut out = Vec::new();
        let mut buf = [0u8; 4096];
        loop {
            let n = unsafe { libc::read(self.fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
            if n <= 0 {
                break;
            }
            let mut i = 0usize;
            let hdr = std::mem::size_of::<libc::inotify_event>();
            while i + hdr <= n as usize {
                let ev: libc::inotify_event = unsafe { std::ptr::read_unaligned(buf[i..].as_ptr() as *const libc::inotify_event) };
                let name = &buf[i + hdr..(i + hdr + ev.len as usize).min(n as usize)];
                let name = String::from_utf8_lossy(name).trim_end_matches('\0').to_string();
                out.push(name);
                i += hdr + ev.len as usize;
            }
        }
        out
    }
}

/// A long-running helper whose lines are read without blocking (playerctl --follow, gdbus monitor).
struct Follow {
    child: std::process::Child,
    lines: std::io::BufReader<std::process::ChildStdout>,
}

impl Follow {
    fn start(cmd: &[&str]) -> Option<Follow> {
        let mut child = std::process::Command::new(cmd[0]).args(&cmd[1..]).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::null()).spawn().ok()?;
        let out = child.stdout.take()?;
        unsafe {
            let fd = out.as_raw_fd();
            let fl = libc::fcntl(fd, libc::F_GETFL);
            libc::fcntl(fd, libc::F_SETFL, fl | libc::O_NONBLOCK);
        }
        Some(Follow { child, lines: std::io::BufReader::new(out) })
    }
    fn fd(&self) -> RawFd {
        self.lines.get_ref().as_raw_fd()
    }
    /// The complete lines waiting; None when it has ended.
    fn read(&mut self) -> Option<Vec<String>> {
        let mut out = Vec::new();
        loop {
            let mut l = String::new();
            match self.lines.read_line(&mut l) {
                Ok(0) => return if out.is_empty() { None } else { Some(out) },
                Ok(_) => out.push(l.trim_end_matches('\n').to_string()),
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => return Some(out),
                Err(_) => return None,
            }
        }
    }
}

impl Drop for Follow {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

enum Popup {
    Set(u8),
    Alert,
    Kept(u8),
    Text(String),
}

pub struct Service {
    root: Root,
    dev: Device,
    cfg: Value,
    set: u8,
    /// A set picked by itself while a game runs (config "gameSets"); None: the saved one.
    game_set: Option<u8>,
    game_screen: Option<String>,
    rec: Recorder,
    out: Arc<Mutex<Option<Box<dyn Output>>>>,
    page: usize,
    sampler: Sampler,
    song: Song,
    alert: Option<Alert>,
    alert_seen: u64,
    popup: Option<(Popup, Instant)>,
    last: Option<Frame>,
    lcd_fd: Option<std::fs::File>,
    lcd_regular: bool,
    /// The lights as you set them (None: as the keyboard was) and as they are now.
    kbd_user: Option<u32>,
    lcd_user: Option<u32>,
    kbd_now: Option<u32>,
    lcd_now: Option<u32>,
    locked: bool,
    session: String,
    rec_fds: Vec<RawFd>,
    next_draw: Instant,
    next_games: Instant,
    player: Option<Follow>,
    player_tries: u32,
    player_next: Instant,
    lockmon: Option<Follow>,
    preview: Option<(Vec<Value>, Instant)>,
    /// The G-keys held now (bit g-1), for macros that repeat while held.
    held: Arc<AtomicU32>,
    next_auto_page: Instant,
    idle: bool,
    next_idle: Instant,
    contrast: Option<u32>,
}

fn screens_of(cfg: &Value) -> Vec<Value> {
    cfg.get("screens").and_then(|s| s.as_array()).cloned().unwrap_or_default()
}

impl Service {
    pub fn new(root: Root, dev: Device) -> Service {
        let cfg = config::load();
        let set = config::set_in_use(&cfg);
        let mut rec = Recorder::new();
        rec.delays = config::flag(&cfg, "recordDelays", true);
        Service {
            root,
            dev,
            set,
            cfg,
            game_set: None,
            game_screen: None,
            rec,
            out: Arc::new(Mutex::new(None)),
            page: 0,
            sampler: Sampler::default(),
            song: Song::default(),
            alert: None,
            alert_seen: 0,
            popup: None,
            last: None,
            lcd_fd: None,
            lcd_regular: false,
            kbd_user: None,
            lcd_user: None,
            kbd_now: None,
            lcd_now: None,
            locked: false,
            session: String::new(),
            rec_fds: Vec::new(),
            next_draw: Instant::now(),
            next_games: Instant::now(),
            player: None,
            player_tries: 0,
            player_next: Instant::now(),
            lockmon: None,
            preview: None,
            held: Arc::new(AtomicU32::new(0)),
            next_auto_page: Instant::now() + Duration::from_secs(10),
            idle: false,
            next_idle: Instant::now() + Duration::from_secs(60),
            contrast: None,
        }
    }

    fn active_set(&self) -> u8 {
        self.game_set.unwrap_or(self.set)
    }

    fn pages(&self) -> Vec<String> {
        let own = screens_of(&self.cfg);
        config::pages(&self.cfg)
            .into_iter()
            .filter(|p| screen::BUILT_IN.contains(&p.as_str()) || own.iter().any(|s| s.get("id").and_then(|v| v.as_str()) == Some(p.as_str()) && s.get("show").and_then(|v| v.as_str()) != Some("hidden")))
            .collect()
    }

    // ---------------------------------------------------------------- lights
    fn m_leds(&self) {
        let on = config::flag(&self.cfg, "mLights", true);
        for i in 0..3u8 {
            leds::set_on(&self.dev.leds.m[i as usize], on && self.active_set() == i + 1);
        }
        leds::set_on(&self.dev.leds.mr, on && self.rec.recording());
    }

    fn night(&self) -> bool {
        if !config::flag(&self.cfg, "nightDim", false) {
            return false;
        }
        let t = hearth_ui::time::local(hearth_ui::time::now());
        let now = (t.hour * 60 + t.min) as u32;
        let from = config::minutes(config::text(&self.cfg, "nightFrom", "21:00")).unwrap_or(21 * 60);
        let to = config::minutes(config::text(&self.cfg, "nightTo", "07:00")).unwrap_or(7 * 60);
        config::in_window(now, from, to)
    }

    /// Puts the lights where they should be now: yours, off while locked, the keys low at night. Never saved.
    fn apply_lights(&mut self) {
        let off = self.locked && config::flag(&self.cfg, "lightsOffWhenLocked", false);
        if let (Some(d), Some(u)) = (self.dev.leds.kbd.clone(), self.kbd_user) {
            let want = if off { 0 } else if self.night() { u.min(1) } else { u };
            if self.kbd_now != Some(want) {
                if let Err(e) = leds::set(&d, want) {
                    crate::log(&format!("key light: {e}"));
                }
                self.kbd_now = Some(want);
            }
        }
        if let (Some(d), Some(u)) = (self.dev.leds.lcd.clone(), self.lcd_user) {
            let want = if off || self.idle { 0 } else { u };
            if self.lcd_now != Some(want) {
                if let Err(e) = leds::set(&d, want) {
                    crate::log(&format!("screen light: {e}"));
                }
                self.lcd_now = Some(want);
                self.last = None;
            }
        }
    }

    /// The screen's contrast, when you've chosen one (never sent otherwise).
    fn apply_contrast(&mut self) {
        let Some(c) = config::light(&self.cfg, "lcdContrast") else { return };
        if self.contrast == Some(c) {
            return;
        }
        if let Some(h) = &self.dev.hidraw {
            if let Err(e) = lcd::set_contrast(h, c) {
                crate::log(&format!("contrast: {e}"));
            }
        }
        self.contrast = Some(c);
    }

    /// "Turn the light off after N minutes idle" (lcdIdleOff, minutes; 0 or missing: never): idle is the PC's own
    /// idle (logind's IdleHint, which KDE and Hearth's desktop set) or every screen of the PC asleep.
    fn check_idle(&mut self) {
        let mins = self.cfg.get("lcdIdleOff").and_then(|v| v.as_u64()).unwrap_or(0);
        let was = self.idle;
        self.idle = mins > 0 && self.root.fixture.is_none() && idle_for(&self.root, &self.session) >= mins * 60;
        if self.idle != was {
            self.apply_lights();
            self.last = None;
        }
    }

    /// The light button on the keyboard was pressed: that's your choice, so it's remembered.
    fn hw_light(&mut self, which: &str) {
        let dir = if which == "kbd" { self.dev.leds.kbd.clone() } else { self.dev.leds.lcd.clone() };
        let Some(d) = dir else { return };
        let Some(v) = leds::hw_changed(&d) else { return };
        let key = if which == "kbd" { "kbdLight" } else { "lcdLight" };
        if which == "kbd" {
            self.kbd_user = Some(v);
            self.kbd_now = Some(v);
        } else {
            self.lcd_user = Some(v);
            self.lcd_now = Some(v);
            self.last = None;
        }
        if config::light(&self.cfg, key) != Some(v) {
            let _ = config::update(|m| {
                m.insert(key.into(), json!(v));
            });
            self.cfg = config::load();
        }
    }

    fn reload(&mut self) {
        let cfg = config::load();
        if cfg == self.cfg {
            return;
        }
        self.cfg = cfg;
        self.set = config::set_in_use(&self.cfg);
        self.rec.delays = config::flag(&self.cfg, "recordDelays", true);
        if let Some(v) = config::light(&self.cfg, "kbdLight") {
            self.kbd_user = Some(v);
        }
        if let Some(v) = config::light(&self.cfg, "lcdLight") {
            self.lcd_user = Some(v);
        }
        self.apply_lights();
        self.apply_contrast();
        self.m_leds();
        let n = self.pages().len().max(1);
        self.page %= n;
        self.last = None;
        self.next_draw = Instant::now();
        self.lock_monitor();
    }

    // ---------------------------------------------------------------- keys
    fn on_key(&mut self, ev: Ev) {
        let Some(k) = crate::keys::gkey(ev.code) else { return };
        if let GKey::G(g) = k {
            let bit = 1u32 << (g as u32 - 1);
            if ev.value == 0 {
                self.held.fetch_and(!bit, Ordering::SeqCst);
            } else if ev.value == 1 {
                self.held.fetch_or(bit, Ordering::SeqCst);
            }
        }
        if ev.value != 1 {
            return;
        }
        match k {
            GKey::M(n) => {
                let had = self.rec.cancel();
                self.stop_recording_fds();
                if had {
                    self.popup = Some((Popup::Text("Not kept".into()), Instant::now() + Duration::from_secs(2)));
                }
                self.game_set = None;
                if self.set != n {
                    self.set = n;
                    let _ = config::update(|m| {
                        m.insert("set".into(), json!(n));
                    });
                    self.cfg = config::load();
                }
                self.m_leds();
                self.popup = Some((Popup::Set(n), Instant::now() + Duration::from_secs(2)));
            }
            GKey::Mr if !config::flag(&self.cfg, "mrRecord", true) => {}
            GKey::Mr => match self.rec.mr(now_ms()) {
                RecOut::Show => self.start_recording_fds(),
                _ => {
                    self.stop_recording_fds();
                    self.popup = Some((Popup::Text("Not kept".into()), Instant::now() + Duration::from_secs(2)));
                }
            },
            GKey::G(g) => {
                if g > self.dev.model.gkeys {
                    return;
                }
                if let Some(r) = self.rec.g(g) {
                    self.stop_recording_fds();
                    match r {
                        RecOut::Save(g, steps) => {
                            let set = self.active_set();
                            let action = json!({"kind": "macro", "name": format!("Recorded {}", hearth_ui::time::clock(hearth_ui::time::now())), "steps": actions::steps_json(&steps)});
                            match config::update(|m| config::set_key(m, set, g, Some(action))) {
                                Ok(()) => self.popup = Some((Popup::Kept(g), Instant::now() + Duration::from_secs(2))),
                                Err(e) => {
                                    crate::log(&format!("couldn't keep the macro: {e}"));
                                    self.popup = Some((Popup::Text("Couldn't keep it".into()), Instant::now() + Duration::from_secs(3)));
                                }
                            }
                            self.cfg = config::load();
                        }
                        _ => self.popup = Some((Popup::Text("Nothing typed".into()), Instant::now() + Duration::from_secs(2))),
                    }
                    self.m_leds();
                } else if let Some(v) = config::key_action(&self.cfg, self.active_set(), g).cloned() {
                    if let Some(a) = actions::parse(&v) {
                        self.do_action(a, actions::macro_opts(&v), g);
                    }
                }
            }
            GKey::Lcd(n) => self.lcd_key(n),
        }
        self.last = None;
        self.next_draw = Instant::now();
    }

    fn lcd_key(&mut self, n: u8) {
        if self.popup.is_some() {
            self.popup = None;
            return;
        }
        let pages = self.pages();
        let cur = pages.get(self.page).cloned().unwrap_or_default();
        if cur == "music" && n >= 2 {
            let verb = match n {
                2 => "previous",
                5 => "next",
                _ => "play-pause",
            };
            actions::run(&Action::Media(verb.into()));
            return;
        }
        if n == 1 {
            self.page = (self.page + 1) % pages.len().max(1);
        } else if (n as usize - 2) < pages.len() {
            self.page = n as usize - 2;
        }
    }

    fn do_action(&mut self, a: Action, opts: actions::MacroOpts, g: u8) {
        if a == Action::Hearth("next-page".into()) {
            self.lcd_key(1);
            return;
        }
        if !actions::run(&a) {
            crate::log(&format!("nothing here can do \"{}\"", actions::describe(&a)));
        }
        let types = matches!(a, Action::Keys(_) | Action::Text(_) | Action::Macro(_) | Action::Hearth(_));
        if !types {
            return;
        }
        let out = self.out.clone();
        let held = self.held.clone();
        let bit = 1u32 << (g.max(1) as u32 - 1);
        let a = match a {
            Action::Macro(steps) => Action::Macro(actions::with_waits(&steps, opts.wait)),
            other => other,
        };
        // Typing on its own thread, one at a time (a long macro doesn't hold up the keyboard or the screen).
        let _ = std::thread::Builder::new().name("hearth-g15-type".into()).spawn(move || {
            let mut g = match out.lock() {
                Ok(g) => g,
                Err(p) => p.into_inner(),
            };
            match g.as_mut() {
                Some(o) => {
                    // "Repeat": that many times; "While held: repeat": again until the G-key is let go (at most 10 min).
                    let started = Instant::now();
                    let mut n = 0;
                    loop {
                        actions::play(&a, o.as_mut(), &mut |ms| actions::sleep_ms(ms));
                        n += 1;
                        let again = if opts.while_held { held.load(Ordering::SeqCst) & bit != 0 } else { n < opts.repeat };
                        if !again || started.elapsed() > Duration::from_secs(600) {
                            break;
                        }
                        actions::sleep_ms(30);
                    }
                }
                None => crate::log("can't type: no access to /dev/uinput"),
            }
        });
    }

    fn start_recording_fds(&mut self) {
        self.stop_recording_fds();
        for p in device::keyboard_events(&self.root) {
            match input::open(&p, libc::O_RDONLY | libc::O_NONBLOCK) {
                Ok(fd) => self.rec_fds.push(fd),
                Err(e) => crate::log(&format!("can't read {} to record: {e}", p.display())),
            }
        }
        if self.rec_fds.is_empty() {
            crate::log("recording: none of the G15's own keys can be read (70-hearth-g15.rules)");
        }
        self.m_leds();
    }

    fn stop_recording_fds(&mut self) {
        for fd in self.rec_fds.drain(..) {
            input::close(fd);
        }
        self.m_leds();
    }

    // ---------------------------------------------------------------- what's playing, alerts, games, locking
    fn player_start(&mut self) {
        if self.root.fixture.is_some() || self.player.is_some() || Instant::now() < self.player_next || self.player_tries > 20 || !crate::which("playerctl") {
            return;
        }
        self.player_tries += 1;
        self.player_next = Instant::now() + Duration::from_secs(30);
        self.player = Follow::start(&["playerctl", "--follow", "metadata", "--format", "{{status}}\t{{position}}\t{{mpris:length}}\t{{artist}}\t{{title}}"]);
    }

    fn fixture_song(&mut self) {
        if let Some(p) = self.root.fixture_file("media.json") {
            if let Ok(v) = serde_json::from_str::<Value>(&std::fs::read_to_string(p).unwrap_or_default()) {
                let s = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
                self.song = Song { known: true, playing: s("status") == "Playing", title: s("title"), artist: s("artist"), pos: v.get("position").and_then(|x| x.as_f64()).unwrap_or(0.0), len: v.get("length").and_then(|x| x.as_f64()).unwrap_or(0.0), at: None };
            }
        }
    }

    fn alerts_file(&self) -> PathBuf {
        self.root.fixture_file("notifications.json").unwrap_or_else(|| root::state_dir().join("notifications.json"))
    }

    fn check_alerts(&mut self, first: bool) {
        let Some((k, a)) = newest_alert(&self.alerts_file()) else { return };
        if k > self.alert_seen {
            self.alert_seen = k;
            self.alert = Some(a);
            if !first && config::flag(&self.cfg, "alerts", true) && !self.rec.recording() {
                self.popup = Some((Popup::Alert, Instant::now() + Duration::from_secs(5)));
                self.next_draw = Instant::now();
            }
        }
    }

    /// The games that pick a key set or a screen by themselves: checked every 5 s, only when there are any.
    fn check_games(&mut self) {
        let rules: Vec<(String, Option<u8>)> = if !config::flag(&self.cfg, "switchSets", true) { Vec::new() } else { self.cfg.get("gameSets").and_then(|g| g.as_array()).map(|a| a.iter().filter_map(|r| Some((r.get("game")?.as_str()?.to_lowercase(), r.get("set").and_then(|s| s.as_u64()).map(|s| s.clamp(1, 3) as u8)))).collect()).unwrap_or_default() };
        let screens: Vec<(String, String)> = screens_of(&self.cfg).iter().filter_map(|s| Some((s.get("whileRunning")?.as_str()?.to_lowercase(), s.get("id")?.as_str()?.to_string()))).filter(|(g, _)| !g.is_empty()).collect();
        if rules.is_empty() && screens.is_empty() {
            self.game_set = None;
            self.game_screen = None;
            return;
        }
        let running = running_names(&self.root.proc_);
        let hit = |g: &str| running.iter().any(|r| r.contains(g));
        let gs = rules.iter().find(|(g, _)| !g.is_empty() && hit(g)).and_then(|(_, s)| *s);
        let sc = screens.iter().find(|(g, _)| hit(g)).map(|(_, id)| id.clone());
        if gs != self.game_set {
            self.game_set = gs;
            self.m_leds();
            if let Some(n) = gs {
                self.popup = Some((Popup::Set(n), Instant::now() + Duration::from_secs(2)));
            }
            self.last = None;
        }
        if sc != self.game_screen {
            self.game_screen = sc;
            self.last = None;
        }
    }

    fn lock_monitor(&mut self) {
        let want = config::flag(&self.cfg, "lightsOffWhenLocked", false) && self.root.fixture.is_none();
        if !want {
            self.lockmon = None;
            if self.locked {
                self.locked = false;
                self.apply_lights();
            }
            return;
        }
        if self.lockmon.is_some() || !crate::which("gdbus") {
            return;
        }
        if self.session.is_empty() {
            let uid = unsafe { libc::getuid() }.to_string();
            self.session = crate::run_out(&["loginctl", "show-user", &uid, "-p", "Display", "--value"], 5).unwrap_or_default().trim().to_string();
        }
        self.lockmon = Follow::start(&["gdbus", "monitor", "--system", "--dest", "org.freedesktop.login1"]);
        self.check_locked();
    }

    fn check_locked(&mut self) {
        if self.session.is_empty() {
            return;
        }
        let l = crate::run_out(&["loginctl", "show-session", &self.session.clone(), "-p", "LockedHint", "--value"], 5).map(|s| s.trim() == "yes").unwrap_or(false);
        if l != self.locked {
            self.locked = l;
            self.apply_lights();
            self.last = None;
        }
    }

    // ---------------------------------------------------------------- the screen
    fn reading(&mut self, elements: &[Value]) -> Reading {
        let wanted = screen::wanted(elements);
        let mut r = self.sampler.sample(&self.root, &wanted);
        let t = &mut r.texts;
        if self.song.known && !self.song.title.is_empty() {
            t.insert("song.title".into(), self.song.title.clone());
            t.insert("song.artist".into(), self.song.artist.clone());
            let p = self.song.pos_now();
            t.insert("song.time".into(), mmss(p));
            if self.song.len > 0.0 {
                r.nums.insert("song.pos".into(), 100.0 * p / self.song.len);
            }
        } else {
            t.insert("song.title".into(), "Nothing playing".into());
            t.insert("song.artist".into(), String::new());
            t.insert("song.time".into(), String::new());
        }
        let t = &mut r.texts;
        t.insert("song.button".into(), if self.song.playing { "▶‖".into() } else { "▶".into() });
        let a = self.alert.clone().unwrap_or(Alert { app: "HEARTH".into(), title: "No messages".into(), body: String::new() });
        t.insert("alert.app".into(), a.app);
        t.insert("alert.title".into(), a.title);
        t.insert("alert.body".into(), a.body);
        if let Some((_, until)) = &self.popup {
            t.insert("alert.left".into(), until.saturating_duration_since(Instant::now()).as_secs_f32().ceil().to_string());
        }
        let set = self.active_set();
        t.insert("set.n".into(), set.to_string());
        let name = self.cfg.get("names").and_then(|n| n.get(format!("m{set}"))).and_then(|v| v.as_str()).unwrap_or("").to_string();
        t.insert("set.name".into(), if name.is_empty() { format!("Key set {set}") } else { name });
        t.insert("set.keys".into(), config::keys_set(&self.cfg, set).to_string());
        let (k, s) = self.rec.summary(now_ms());
        t.insert("rec.keys".into(), k.to_string());
        t.insert("rec.secs".into(), format!("{s:.1}"));
        r
    }

    /// The screen's elements now, and whether it changes every second.
    fn current(&mut self) -> (Vec<Value>, bool, usize) {
        if let Some((_, until)) = &self.popup {
            if Instant::now() >= *until {
                self.popup = None;
            }
        }
        if let Some((_, until)) = &self.preview {
            if Instant::now() >= *until {
                self.preview = None;
            }
        }
        let own = screens_of(&self.cfg);
        let pages = self.pages();
        if self.rec.recording() {
            return (screen::built_in("rec").unwrap_or_default(), true, self.page);
        }
        if let Some((p, _)) = &self.popup {
            let els = match p {
                Popup::Set(_) => screen::built_in("set").unwrap_or_default(),
                Popup::Alert => screen::built_in("alert").unwrap_or_default(),
                Popup::Kept(g) => vec![json!({"type": "text", "x": 2, "y": 4, "size": 4, "text": format!("G{g}")}), json!({"type": "text", "x": 60, "y": 8, "size": 2, "w": 100, "text": "Kept"}), json!({"type": "text", "x": 60, "y": 26, "w": 100, "text": "{rec.keys} keys"})],
                Popup::Text(t) => vec![json!({"type": "text", "x": 0, "y": 15, "w": 160, "size": 2, "align": "center", "text": t})],
            };
            return (els, true, self.page);
        }
        if let Some((els, _)) = &self.preview {
            let e = els.clone();
            let l = screen::live(&e);
            return (e, l, self.page);
        }
        if let Some(id) = &self.game_screen {
            let e = screen::elements_of(id, &own);
            let l = screen::live(&e);
            return (e, l, self.page);
        }
        let id = pages.get(self.page).cloned().unwrap_or_else(|| "clock".into());
        let e = screen::elements_of(&id, &own);
        let l = screen::live(&e) || (id == "music" && self.song.playing);
        (e, l, self.page)
    }

    pub fn render(&mut self) -> (Frame, bool) {
        let (els, live, tab) = self.current();
        let r = self.reading(&els);
        let own = screens_of(&self.cfg);
        let tabs: Vec<String> = self.pages().iter().map(|p| screen::tab_name(p, &own)).collect();
        let c = screen::Ctx { r: &r, history: &self.sampler.history, now: hearth_ui::time::now(), tabs, tab };
        let mut f = Frame::new();
        screen::draw(&mut f, &els, &c);
        (f, live)
    }

    fn screen_on(&self) -> bool {
        self.dev.hidraw.is_some() && self.lcd_now != Some(0) && !(self.locked && config::flag(&self.cfg, "lightsOffWhenLocked", false))
    }

    fn draw(&mut self) {
        let (f, live) = self.render();
        let now = Instant::now();
        // Next: in a second for a live screen, or at the next minute (the clock), or when a pop-up ends.
        let secs = hearth_ui::time::local(hearth_ui::time::now()).sec as u64;
        let mut next = if live { now + Duration::from_secs(1) } else { now + Duration::from_secs(60 - secs.min(59)) };
        if let Some((_, until)) = &self.popup {
            next = next.min(*until);
        }
        self.next_draw = next;
        if self.last.as_ref() == Some(&f) {
            return;
        }
        let report = lcd::pack(&f);
        if self.lcd_fd.is_none() {
            if let Some(p) = &self.dev.hidraw {
                match std::fs::OpenOptions::new().write(true).open(p) {
                    Ok(file) => {
                        self.lcd_regular = file.metadata().map(|m| m.is_file()).unwrap_or(false);
                        self.lcd_fd = Some(file);
                    }
                    Err(e) => {
                        crate::log(&format!("can't draw on the screen ({}): {e}", p.display()));
                        self.dev.hidraw = None;
                        return;
                    }
                }
            }
        }
        if let Some(file) = &mut self.lcd_fd {
            let ok = if self.lcd_regular {
                use std::os::unix::fs::FileExt;
                file.write_all_at(&report, 0).is_ok()
            } else {
                file.write_all(&report).is_ok()
            };
            if !ok {
                crate::log("the screen didn't take a frame");
                self.lcd_fd = None;
            }
        }
        // For the app's live picture of the screen.
        let rd = root::runtime_dir();
        let _ = std::fs::create_dir_all(&rd);
        let tmp = rd.join(".g15-frame.tmp");
        if std::fs::write(&tmp, &report).is_ok() {
            let _ = std::fs::rename(&tmp, rd.join("g15-frame.bin"));
        }
        self.last = Some(f);
    }

    // ---------------------------------------------------------------- the loop
    pub fn run(mut self) -> i32 {
        let Some(keys) = self.dev.keys.clone() else {
            crate::log("the G-keys' input device isn't there");
            return 0;
        };
        // The pretend keyboard's G-keys are a FIFO: opened both ways, so it never looks unplugged between writers.
        let how = if self.root.fixture.is_some() { libc::O_RDWR } else { libc::O_RDONLY };
        let kfd = match input::open(&keys, how | libc::O_NONBLOCK) {
            Ok(fd) => fd,
            Err(e) => {
                crate::log(&format!("can't read the G-keys ({}): {e}. Hearth's udev rule (70-hearth-g15.rules) gives you access; unplug the keyboard and plug it in again, or restart.", keys.display()));
                return NO_ACCESS;
            }
        };
        match input::Uinput::new(&self.root.dev) {
            Ok(u) => *self.out.lock().unwrap() = Some(Box::new(u)),
            Err(e) => crate::log(&format!("no virtual keyboard ({}/uinput: {e}): G-keys can't type", self.root.dev.display())),
        }
        // The lights as the keyboard has them, then yours (only what you saved).
        self.kbd_now = self.dev.leds.kbd.as_ref().and_then(|d| leds::level(d));
        self.lcd_now = self.dev.leds.lcd.as_ref().and_then(|d| leds::level(d));
        self.kbd_user = config::light(&self.cfg, "kbdLight").or(self.kbd_now);
        self.lcd_user = config::light(&self.cfg, "lcdLight").or(self.lcd_now);
        self.apply_lights();
        self.apply_contrast();
        self.m_leds();
        // The light button: brightness_hw_changed is read once, then poll()ed.
        let hw: Vec<(RawFd, &'static str)> = [(self.dev.leds.kbd.clone(), "kbd"), (self.dev.leds.lcd.clone(), "lcd")]
            .into_iter()
            .filter_map(|(d, w)| {
                let p = d?.join("brightness_hw_changed");
                let fd = input::open(&p, libc::O_RDONLY | libc::O_NONBLOCK).ok()?;
                let mut b = [0u8; 16];
                unsafe { libc::read(fd, b.as_mut_ptr() as *mut libc::c_void, b.len()) };
                Some((fd, w))
            })
            .collect();
        let ino = Inotify::new();
        if let Some(i) = &ino {
            i.watch(&config::path().parent().map(Path::to_path_buf).unwrap_or_else(root::config_dir));
            i.watch(self.alerts_file().parent().unwrap_or(Path::new("/")));
            i.watch(&root::runtime_dir());
        }
        self.fixture_song();
        self.check_alerts(true);
        if self.root.fixture.is_none() {
            let uid = unsafe { libc::getuid() }.to_string();
            self.session = crate::run_out(&["loginctl", "show-user", &uid, "-p", "Display", "--value"], 5).unwrap_or_default().trim().to_string();
        }
        self.lock_monitor();
        let cfg_name = config::path().file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        loop {
            self.player_start();
            if Instant::now() >= self.next_games {
                self.check_games();
                self.next_games = Instant::now() + Duration::from_secs(5);
            }
            if config::flag(&self.cfg, "autoPage", false) && Instant::now() >= self.next_auto_page {
                let secs = self.cfg.get("autoPageSecs").and_then(|v| v.as_u64()).unwrap_or(10).clamp(3, 600);
                self.next_auto_page = Instant::now() + Duration::from_secs(secs);
                if self.popup.is_none() && !self.rec.recording() && self.game_screen.is_none() && self.preview.is_none() {
                    self.page = (self.page + 1) % self.pages().len().max(1);
                    self.next_draw = Instant::now();
                }
            }
            if Instant::now() >= self.next_idle {
                self.next_idle = Instant::now() + Duration::from_secs(60);
                self.check_idle();
            }
            if self.screen_on() && Instant::now() >= self.next_draw {
                self.draw();
            }
            let mut fds: Vec<(RawFd, i16)> = vec![(kfd, libc::POLLIN)];
            for (fd, _) in &hw {
                fds.push((*fd, libc::POLLPRI | libc::POLLERR));
            }
            let ino_at = fds.len();
            if let Some(i) = &ino {
                fds.push((i.fd, libc::POLLIN));
            }
            let player_at = fds.len();
            if let Some(p) = &self.player {
                fds.push((p.fd(), libc::POLLIN));
            }
            let lock_at = fds.len();
            if let Some(l) = &self.lockmon {
                fds.push((l.fd(), libc::POLLIN));
            }
            let rec_at = fds.len();
            for fd in &self.rec_fds {
                fds.push((*fd, libc::POLLIN));
            }
            let now = Instant::now();
            let mut wake = self.next_games.max(now + Duration::from_secs(5));
            if self.screen_on() {
                wake = wake.min(self.next_draw);
            }
            if self.player.is_none() && self.player_tries <= 20 && self.root.fixture.is_none() && crate::which("playerctl") {
                wake = wake.min(self.player_next);
            }
            if self.night() || config::flag(&self.cfg, "nightDim", false) {
                wake = wake.min(now + Duration::from_secs(60));
            }
            if config::flag(&self.cfg, "autoPage", false) {
                wake = wake.min(self.next_auto_page);
            }
            if self.cfg.get("lcdIdleOff").and_then(|v| v.as_u64()).unwrap_or(0) > 0 {
                wake = wake.min(self.next_idle);
            }
            let ms = wake.saturating_duration_since(now).as_millis().min(3_600_000) as i32;
            let rev = input::poll(&fds, ms.max(10));
            // The G-keys (and the keyboard going away).
            if rev[0] & (libc::POLLERR | libc::POLLHUP | libc::POLLNVAL) != 0 {
                crate::log("the keyboard was unplugged");
                return 0;
            }
            if rev[0] & libc::POLLIN != 0 {
                match input::read_events(kfd) {
                    ReadResult::Events(evs) => {
                        for e in evs {
                            self.on_key(e);
                        }
                    }
                    ReadResult::Gone => {
                        crate::log("the keyboard was unplugged");
                        return 0;
                    }
                    ReadResult::Nothing => {}
                }
            }
            for (i, (fd, which)) in hw.iter().enumerate() {
                if rev[1 + i] & (libc::POLLPRI | libc::POLLERR) != 0 {
                    let mut b = [0u8; 16];
                    unsafe {
                        libc::lseek(*fd, 0, libc::SEEK_SET);
                        libc::read(*fd, b.as_mut_ptr() as *mut libc::c_void, b.len());
                    }
                    self.hw_light(which);
                }
            }
            if ino.is_some() && rev[ino_at] & libc::POLLIN != 0 {
                let names = ino.as_ref().unwrap().names();
                if names.iter().any(|n| *n == cfg_name) {
                    self.reload();
                }
                if names.iter().any(|n| n == "notifications.json") {
                    self.check_alerts(false);
                }
                if names.iter().any(|n| n == "g15-show.json") {
                    self.preview_from(&root::runtime_dir().join("g15-show.json"));
                }
            }
            if self.player.is_some() && rev[player_at] & (libc::POLLIN | libc::POLLHUP) != 0 {
                match self.player.as_mut().unwrap().read() {
                    Some(lines) => {
                        if let Some(l) = lines.last() {
                            self.song = parse_song(l);
                            self.last = None;
                            self.next_draw = Instant::now();
                        }
                    }
                    None => self.player = None,
                }
            }
            if self.lockmon.is_some() && rev[lock_at] & (libc::POLLIN | libc::POLLHUP) != 0 {
                match self.lockmon.as_mut().unwrap().read() {
                    Some(lines) => {
                        if lines.iter().any(|l| l.contains("LockedHint") || l.contains("PrepareForSleep") || l.contains("Lock") ) {
                            self.check_locked();
                        }
                    }
                    None => self.lockmon = None,
                }
            }
            let fds_now = self.rec_fds.clone();
            for (i, fd) in fds_now.iter().enumerate() {
                if rev.get(rec_at + i).map(|r| r & libc::POLLIN != 0).unwrap_or(false) {
                    if let ReadResult::Events(evs) = input::read_events(*fd) {
                        for e in evs {
                            self.rec.key(e.code, e.value, now_ms());
                        }
                    }
                }
            }
            // The night window starting or ending.
            self.apply_lights();
        }
    }

    /// The app's "Show on the keyboard": a screen shown for 30 s ($XDG_RUNTIME_DIR/hearth/g15-show.json).
    fn preview_from(&mut self, p: &Path) {
        let Ok(t) = std::fs::read_to_string(p) else { return };
        let Ok(v) = serde_json::from_str::<Value>(&t) else { return };
        if let Some(e) = v.get("elements").and_then(|e| e.as_array()) {
            self.preview = Some((e.clone(), Instant::now() + Duration::from_secs(30)));
            self.last = None;
            self.next_draw = Instant::now();
        }
    }
}

/// Seconds the PC has been idle: logind's IdleSinceHint when the session says it's idle, else how long every screen
/// has been asleep (DPMS off in /sys/class/drm), else 0. Run once a minute, only when the setting is on.
fn idle_for(root: &Root, session: &str) -> u64 {
    if !session.is_empty() {
        if let Some(o) = crate::run_out(&["loginctl", "show-session", session, "-p", "IdleHint", "-p", "IdleSinceHintMonotonic"], 5) {
            let idle = o.lines().any(|l| l.trim() == "IdleHint=yes");
            let since = o.lines().find_map(|l| l.strip_prefix("IdleSinceHintMonotonic=")).and_then(|v| v.trim().parse::<u64>().ok()).unwrap_or(0);
            if idle && since > 0 {
                let mut ts = libc::timespec { tv_sec: 0, tv_nsec: 0 };
                unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut ts) };
                let now_us = ts.tv_sec as u64 * 1_000_000 + ts.tv_nsec as u64 / 1000;
                return now_us.saturating_sub(since) / 1_000_000;
            }
        }
    }
    static SINCE: Mutex<Option<Instant>> = Mutex::new(None);
    let mut any = false;
    let mut all_off = true;
    if let Ok(rd) = std::fs::read_dir(root.sys.join("class/drm")) {
        for e in rd.filter_map(|e| e.ok()) {
            let n = e.file_name().to_string_lossy().to_string();
            if !n.contains('-') || read(&e.path().join("status")).as_deref() != Some("connected") {
                continue;
            }
            any = true;
            if read(&e.path().join("dpms")).as_deref() != Some("Off") {
                all_off = false;
            }
        }
    }
    let mut g = SINCE.lock().unwrap_or_else(|p| p.into_inner());
    if any && all_off {
        g.get_or_insert_with(Instant::now).elapsed().as_secs()
    } else {
        *g = None;
        0
    }
}

/// Every running program's name and the file names on its command line (lowercase), for "while a game runs".
fn running_names(proc_: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(proc_) else { return out };
    for e in rd.filter_map(|e| e.ok()) {
        let name = e.file_name().to_string_lossy().to_string();
        if !name.bytes().all(|b| b.is_ascii_digit()) {
            continue;
        }
        if let Some(c) = read(&e.path().join("comm")) {
            out.push(c.to_lowercase());
        }
        if let Ok(cl) = std::fs::read(e.path().join("cmdline")) {
            for a in cl.split(|&b| b == 0).take(3) {
                let a = String::from_utf8_lossy(a).to_lowercase();
                let base = a.rsplit(['/', '\\']).next().unwrap_or("").to_string();
                if !base.is_empty() {
                    out.push(base);
                }
            }
        }
    }
    out
}

// ------------------------------------------------------------------ the command line's status and the fixture
/// What the app shows about the keyboard (`hearth-g15 status`).
pub fn status(root: &Root) -> Value {
    let Some(d) = device::find(root) else { return json!({"plugged": false}) };
    let light = |p: &Option<PathBuf>| match p {
        Some(dir) => json!({"level": leds::level(dir), "max": leds::max(dir), "writable": device::writable(&dir.join("brightness"))}),
        None => Value::Null,
    };
    let uinput = root.dev.join("uinput");
    json!({
        "plugged": true,
        "model": d.model.name,
        "gkeys": d.model.gkeys,
        "screen": d.model.lcd,
        "screenWritable": d.hidraw.as_ref().map(|p| device::writable(p)).unwrap_or(false),
        "keysReadable": d.keys.as_ref().map(|p| device::readable(p)).unwrap_or(false),
        "typing": device::writable(&uinput),
        "recording": device::keyboard_events(root).iter().any(|p| device::readable(p)),
        "kbdLight": light(&d.leds.kbd),
        "lcdLight": light(&d.leds.lcd),
        "set": config::set_in_use(&config::load()),
    })
}

/// A pretend G15 in DIR (CI, the pictures, the tests): sysfs as the kernel lays it out (symlinks and all), a regular
/// file for the screen's hidraw node (each frame overwrites it), a FIFO for the G-keys (write events into it),
/// pretend readings (stats.json, media.json, notifications.json).
pub fn make_fixture(dir: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::symlink;
    let hid = dir.join("sys/devices/pci0000:00/usb1/1-4/1-4.2/1-4.2:1.0/0003:046D:C222.0005");
    let mk = |p: &Path, s: &str| -> std::io::Result<()> {
        if let Some(d) = p.parent() {
            std::fs::create_dir_all(d)?;
        }
        std::fs::write(p, s)
    };
    mk(&hid.join("uevent"), "DRIVER=lg-g15\nHID_ID=0003:0000046D:0000C222\nHID_NAME=G15 GamePanel LCD\n")?;
    std::fs::create_dir_all(hid.join("hidraw/hidraw3"))?;
    let _ = symlink("../..", hid.join("hidraw/hidraw3/device"));
    for (n, level, max) in [("g15::kbd_backlight", 2, 2), ("g15::lcd_backlight", 2, 2), ("g15::macro_preset1", 1, 1), ("g15::macro_preset2", 0, 1), ("g15::macro_preset3", 0, 1), ("g15::macro_record", 0, 1)] {
        let l = hid.join("leds").join(n);
        mk(&l.join("brightness"), &level.to_string())?;
        mk(&l.join("max_brightness"), &max.to_string())?;
        if max == 2 {
            mk(&l.join("brightness_hw_changed"), &level.to_string())?;
        }
        let _ = symlink("../..", l.join("device"));
    }
    let inp = hid.join("input/input12");
    mk(&inp.join("name"), &format!("{}\n", device::GAMING_KEYS))?;
    mk(&inp.join("id/vendor"), "046d\n")?;
    mk(&inp.join("id/product"), "c222\n")?;
    std::fs::create_dir_all(inp.join("event5"))?;
    let _ = symlink("../..", inp.join("device"));
    let _ = symlink("..", inp.join("event5/device"));
    // The ordinary keys (046d:c221), for recording.
    let kb = dir.join("sys/devices/pci0000:00/usb1/1-4/1-4.1/1-4.1:1.0/0003:046D:C221.0004/input/input11");
    mk(&kb.join("name"), "Logitech Gaming Keyboard\n")?;
    mk(&kb.join("id/vendor"), "046d\n")?;
    mk(&kb.join("id/product"), "c221\n")?;
    std::fs::create_dir_all(kb.join("event4"))?;
    let _ = symlink("..", kb.join("event4/device"));
    for (cls, name, target) in [
        ("hidraw", "hidraw3", hid.join("hidraw/hidraw3")),
        ("input", "event5", inp.join("event5")),
        ("input", "event4", kb.join("event4")),
    ] {
        std::fs::create_dir_all(dir.join("sys/class").join(cls))?;
        let _ = symlink(&target, dir.join("sys/class").join(cls).join(name));
    }
    std::fs::create_dir_all(dir.join("sys/class/leds"))?;
    for n in ["g15::kbd_backlight", "g15::lcd_backlight", "g15::macro_preset1", "g15::macro_preset2", "g15::macro_preset3", "g15::macro_record"] {
        let _ = symlink(hid.join("leds").join(n), dir.join("sys/class/leds").join(n));
    }
    std::fs::create_dir_all(dir.join("dev/input"))?;
    std::fs::write(dir.join("dev/hidraw3"), [])?;
    for ev in ["event5", "event4"] {
        let p = dir.join("dev/input").join(ev);
        if !p.exists() {
            let c = std::ffi::CString::new(p.as_os_str().as_encoded_bytes()).unwrap();
            if unsafe { libc::mkfifo(c.as_ptr(), 0o600) } != 0 {
                std::fs::write(&p, [])?;
            }
        }
    }
    // What the pretend PC shows (the dad's PC, as in the mockups).
    std::fs::write(dir.join("stats.json"), serde_json::to_string_pretty(&json!({
        "cpu.use": 23, "cpu.temp": 48, "gpu.use": 61, "gpu.temp": 66, "mem.use": 34, "mem.used": 11, "mem.total": 32,
        "disk.use": 41, "disk.free": 588, "net.down": 312, "net.up": 24, "fan": 1120, "pad.battery": 80,
        "weather.temp": 41, "weather.unit": "°F", "weather.sky": "☁", "cpu.name": "R7 5700", "gpu.name": "RX 6750 XT"
    }))?)?;
    std::fs::write(dir.join("media.json"), r#"{"status": "Playing", "title": "Hearthfire", "artist": "Hearth OS trailer", "position": 84, "length": 200}"#)?;
    std::fs::write(dir.join("notifications.json"), r#"{"n:7": {"app": "Hearth", "summary": "Update ready:", "body": "Restart to finish", "time": 0}}"#)?;
    Ok(())
}

/// Renders a page of the pretend (or real) keyboard: the CI pictures and `hearth-g15 frame`.
pub fn frame_of(root: Root, page: &str) -> Option<Frame> {
    let dev = device::find(&root)?;
    let mut s = Service::new(root, dev);
    s.fixture_song();
    s.check_alerts(true);
    // Two readings a second apart give the use numbers on a real PC; the fixture has them already.
    let _ = s.render();
    match page {
        "rec" => {
            s.rec.mr(now_ms().saturating_sub(3200));
            for (c, v) in [(30, 1), (30, 0), (31, 1), (31, 0), (32, 1), (32, 0), (33, 1), (33, 0), (34, 1), (34, 0), (35, 1), (35, 0), (36, 1), (36, 0)] {
                s.rec.key(c, v, now_ms());
            }
        }
        "set" => s.popup = Some((Popup::Set(2), Instant::now() + Duration::from_secs(5))),
        "alert" => s.popup = Some((Popup::Alert, Instant::now() + Duration::from_secs(5))),
        p => {
            let pages = s.pages();
            if let Some(i) = pages.iter().position(|x| x == p) {
                s.page = i;
            } else if screens_of(&s.cfg).iter().any(|x| x.get("id").and_then(|v| v.as_str()) == Some(p)) {
                s.game_screen = Some(p.to_string());
            } else {
                return None;
            }
        }
    }
    Some(s.render().0)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reads_playerctl_and_alerts() {
        let s = parse_song("Playing\t84000000\t200000000\tHearth OS trailer\tHearthfire");
        assert!(s.known && s.playing);
        assert_eq!((s.title.as_str(), s.len), ("Hearthfire", 200.0));
        assert!(!parse_song("").known);
        assert_eq!(mmss(84.0), "1:24");
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("n.json");
        std::fs::write(&p, r#"{"n:3": {"app": "x", "summary": "old"}, "n:12": {"app": "Hearth", "summary": "Update ready:", "body": "Restart\nto finish"}, "other": 1}"#).unwrap();
        let (k, a) = newest_alert(&p).unwrap();
        assert_eq!((k, a.app.as_str(), a.body.as_str()), (12, "HEARTH", "Restart to finish"));
    }

    #[test]
    fn pictures_every_page_of_the_pretend_keyboard() {
        let d = tempfile::tempdir().unwrap();
        make_fixture(d.path()).unwrap();
        std::env::set_var("HEARTH_G15_CONFIG", d.path().join("g15.json"));
        for p in ["clock", "system", "music", "alerts", "rec", "set", "alert"] {
            let f = frame_of(Root::at(d.path()), p).unwrap_or_else(|| panic!("{p}"));
            assert!(f.lit() > 40, "{p} is nearly empty");
        }
        assert!(!d.path().join("g15.json").exists(), "picturing the pages wrote nothing");
    }
}
