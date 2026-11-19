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
use crate::lights_auto::{self, Login1};
use crate::root::{self, read, Root};
use crate::running::{self, Running};
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
    /// logind said the PC is going to sleep (PrepareForSleep), until it wakes.
    asleep: bool,
    /// Night light is on now (desktop.json's schedule, lights_auto.rs), and when it next changes: one timer.
    night_on: bool,
    next_night: Option<Instant>,
    /// logind's IdleHint for our session: since when (CLOCK_MONOTONIC µs) the PC has been idle; None: it isn't.
    idle_since: Option<u64>,
    /// Which games run, while there's a game rule (running.rs: told by the window lists, Steam and Game Mode).
    running: Option<Running>,
    session: String,
    rec_fds: Vec<RawFd>,
    next_draw: Instant,
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
    /// The G-key a recording started from the app goes to (MR, or the app's Stop, keeps it there).
    rec_target: Option<u8>,
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
            asleep: false,
            night_on: false,
            next_night: None,
            idle_since: None,
            running: None,
            session: String::new(),
            rec_fds: Vec::new(),
            next_draw: Instant::now(),
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
            rec_target: None,
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
        self.night_on
    }

    /// "Dim the keys with Night light": follows Hearth's Night light (Settings › Display, desktop.json; KWin's on
    /// Plasma) and sets one timer to its next change. Asked again at that timer, when desktop.json changes and when
    /// the PC wakes up (the timer doesn't count time asleep). Without logind's signals (no gdbus) the timer is at
    /// most an hour, so a clock change or a sleep can't leave it wrong for long.
    fn update_night(&mut self) {
        if !config::flag(&self.cfg, "nightDim", false) {
            self.night_on = false;
            self.next_night = None;
            return;
        }
        let n = lights_auto::for_root(&self.root);
        let (on, secs) = lights_auto::state_at(&n, hearth_ui::time::now());
        let cap = if self.lockmon.is_some() { 24 * 3600 } else { 3600 };
        self.night_on = on;
        self.next_night = secs.map(|s| Instant::now() + Duration::from_secs(s.min(cap)));
    }

    /// The levels you set, kept for this sign-in in the runtime folder (not a setting): when the service starts again
    /// while the keys are dimmed or off, it still knows what to put back.
    fn remember_levels(&self) {
        if self.root.fixture.is_some() {
            return;
        }
        let v = json!({"kbd": self.kbd_user, "lcd": self.lcd_user});
        let p = root::runtime_dir().join("g15-lights.json");
        if std::fs::read_to_string(&p).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok()) == Some(v.clone()) {
            return;
        }
        let _ = std::fs::create_dir_all(root::runtime_dir());
        let tmp = root::runtime_dir().join(".g15-lights.tmp");
        if std::fs::write(&tmp, v.to_string()).is_ok() {
            let _ = std::fs::rename(&tmp, &p);
        }
    }

    fn levels_before(&self) -> (Option<u32>, Option<u32>) {
        if self.root.fixture.is_some() {
            return (None, None);
        }
        let v: Value = std::fs::read_to_string(root::runtime_dir().join("g15-lights.json")).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
        (config::light(&v, "kbd"), config::light(&v, "lcd"))
    }

    /// Puts the lights where they should be now: yours, off while locked or asleep, the keys low at night. Never saved.
    fn apply_lights(&mut self) {
        let off = (self.locked || self.asleep) && config::flag(&self.cfg, "lightsOffWhenLocked", false);
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
    /// idle (logind's IdleHint, which KDE and Hearth's desktop set: it comes as a signal, and one timer is set to the
    /// moment it reaches N minutes) or, when logind doesn't say, every screen of the PC asleep (a look at
    /// /sys/class/drm once a minute, no program run, only while this setting is on).
    fn check_idle(&mut self) {
        let mins = self.cfg.get("lcdIdleOff").and_then(|v| v.as_u64()).unwrap_or(0);
        let was = self.idle;
        let now = Instant::now();
        let far = now + Duration::from_secs(24 * 3600);
        if mins == 0 || self.root.fixture.is_some() {
            self.idle = false;
            self.next_idle = far;
        } else if let Some(since) = self.idle_since {
            let secs = mono_us().saturating_sub(since) / 1_000_000;
            self.idle = secs >= mins * 60;
            self.next_idle = if self.idle { far } else { now + Duration::from_secs(mins * 60 - secs) };
        } else {
            self.idle = drm_idle_for(&self.root) >= mins * 60;
            self.next_idle = now + Duration::from_secs(60);
        }
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
        self.remember_levels();
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
        self.remember_levels();
        self.update_night();
        self.check_idle();
        self.apply_lights();
        self.apply_contrast();
        self.m_leds();
        let n = self.pages().len().max(1);
        self.page %= n;
        self.last = None;
        self.next_draw = Instant::now();
        self.lock_monitor();
        self.check_games();
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
            GKey::Mr if self.rec.recording() && self.rec_target.is_some() => {
                let g = self.rec_target.take().unwrap_or(1);
                self.keep_recording(g);
            }
            GKey::Mr => match self.rec.mr(now_ms()) {
                RecOut::Show => {
                    self.rec_target = None;
                    self.start_recording_fds()
                }
                _ => {
                    self.stop_recording_fds();
                    self.popup = Some((Popup::Text("Not kept".into()), Instant::now() + Duration::from_secs(2)));
                }
            },
            GKey::G(g) => {
                if g > self.dev.model.gkeys {
                    return;
                }
                if self.rec.recording() {
                    self.rec_target = None;
                    self.keep_recording(g);
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

    /// The macro being recorded kept on G-key `g` (of the set in use).
    fn keep_recording(&mut self, g: u8) {
        let Some(r) = self.rec.g(g) else { return };
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

    /// The games that pick a key set or a screen by themselves. Only while there are such rules, running.rs follows
    /// what runs (the desktop's window list, Steam, Hearth's Game Mode: told when it changes, nothing looked at on a
    /// timer unless no window list can be had). Called when those change and when g15.json does. The set a game picks
    /// is live only (game_set): g15.json is never written for it, and the saved set is back when the game closes.
    fn check_games(&mut self) {
        let rules: Vec<(String, Option<u8>)> = if !config::flag(&self.cfg, "switchSets", true) { Vec::new() } else { self.cfg.get("gameSets").and_then(|g| g.as_array()).map(|a| a.iter().filter_map(|r| Some((r.get("game")?.as_str()?.to_string(), r.get("set").and_then(|s| s.as_u64()).map(|s| s.clamp(1, 3) as u8)))).collect()).unwrap_or_default() };
        let screens: Vec<(String, String)> = screens_of(&self.cfg).iter().filter_map(|s| Some((s.get("whileRunning")?.as_str()?.to_string(), s.get("id")?.as_str()?.to_string()))).filter(|(g, _)| !g.trim().is_empty()).collect();
        let (gs, sc) = if rules.iter().all(|(g, _)| g.trim().is_empty()) && screens.is_empty() {
            self.running = None;
            (None, None)
        } else {
            if self.running.is_none() {
                let r = Running::start(running::Paths::real(&self.root.proc_, self.root.fixture.is_none()));
                crate::log(&format!("following games through {}", r.sources()));
                self.running = Some(r);
            }
            let names = self.running.as_ref().map(|r| r.names()).unwrap_or_default();
            (rules.iter().find(|(g, _)| running::any_match(g, &names)).and_then(|(_, s)| *s), screens.iter().find(|(g, _)| running::any_match(g, &names)).map(|(_, id)| id.clone()))
        };
        if gs != self.game_set {
            self.game_set = gs;
            self.m_leds();
            if let Some(n) = gs {
                self.popup = Some((Popup::Set(n), Instant::now() + Duration::from_secs(2)));
            }
            self.last = None;
            self.next_draw = Instant::now();
        }
        if sc != self.game_screen {
            self.game_screen = sc;
            self.last = None;
            self.next_draw = Instant::now();
        }
    }

    /// logind's signals, from one `gdbus monitor` (it sleeps until logind says something): going to sleep and waking
    /// up, our session locked / unlocked and idle. On a real keyboard it always runs: waking up puts the lights, the
    /// M-key light, the contrast and the screen back even with "lights off" unset (the keyboard may have lost them).
    fn lock_monitor(&mut self) {
        if self.root.fixture.is_some() || self.lockmon.is_some() || !crate::which("gdbus") {
            return;
        }
        if self.session.is_empty() {
            self.find_session();
        }
        self.lockmon = Follow::start(&["gdbus", "monitor", "--system", "--dest", "org.freedesktop.login1"]);
        self.check_locked();
    }

    fn find_session(&mut self) {
        let uid = unsafe { libc::getuid() }.to_string();
        self.session = crate::run_out(&["loginctl", "show-user", &uid, "-p", "Display", "--value"], 5).unwrap_or_default().trim().to_string();
    }

    /// Asked once (at the start, and when a session comes): locked and idle now. Later changes come as signals.
    fn check_locked(&mut self) {
        if self.session.is_empty() {
            return;
        }
        let o = crate::run_out(&["loginctl", "show-session", &self.session.clone(), "-p", "LockedHint", "-p", "IdleHint", "-p", "IdleSinceHintMonotonic"], 5).unwrap_or_default();
        let get = |k: &str| o.lines().find_map(|l| l.trim().strip_prefix(k)).map(|v| v.trim().to_string());
        let l = get("LockedHint=").as_deref() == Some("yes");
        self.idle_since = if get("IdleHint=").as_deref() == Some("yes") { get("IdleSinceHintMonotonic=").and_then(|v| v.parse().ok()).filter(|&v: &u64| v > 0).or_else(|| Some(mono_us())) } else { None };
        self.on_login1(Login1::Locked(l));
        self.check_idle();
    }

    /// One of logind's signals (lights_auto::parse_login1). Only the live lights change; nothing is saved.
    fn on_login1(&mut self, ev: Login1) {
        match ev {
            Login1::Sleep(true) => {
                self.asleep = true;
                self.apply_lights();
            }
            Login1::Sleep(false) => {
                // Awake: everything put back as you have it, written again (the keyboard may have lost it).
                self.asleep = false;
                self.kbd_now = None;
                self.lcd_now = None;
                self.contrast = None;
                self.update_night();
                self.apply_lights();
                self.apply_contrast();
                self.m_leds();
                self.last = None;
                self.next_draw = Instant::now();
            }
            Login1::Locked(l) => {
                if l != self.locked {
                    self.locked = l;
                    self.apply_lights();
                    self.last = None;
                    self.next_draw = Instant::now();
                }
            }
            Login1::Idle(i, since) => {
                self.idle_since = if i { since.filter(|&v| v > 0).or_else(|| Some(mono_us())) } else { None };
                self.check_idle();
            }
            Login1::Sessions => {
                if self.session.is_empty() {
                    self.find_session();
                    self.check_locked();
                }
            }
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
        self.dev.hidraw.is_some() && self.lcd_now != Some(0) && !((self.locked || self.asleep) && config::flag(&self.cfg, "lightsOffWhenLocked", false))
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
        // Nothing saved: the levels you had earlier in this sign-in (if the service started again while the keys were
        // dimmed or off), else the keyboard's own.
        let (kb, lb) = self.levels_before();
        self.kbd_user = config::light(&self.cfg, "kbdLight").or(kb).or(self.kbd_now);
        self.lcd_user = config::light(&self.cfg, "lcdLight").or(lb).or(self.lcd_now);
        self.remember_levels();
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
            if self.root.fixture.is_none() {
                // Night light's settings (desktop.json), for "Dim the keys with Night light".
                i.watch(&lights_auto::data_dir());
            }
        }
        self.fixture_song();
        self.check_alerts(true);
        if self.root.fixture.is_none() {
            let uid = unsafe { libc::getuid() }.to_string();
            self.session = crate::run_out(&["loginctl", "show-user", &uid, "-p", "Display", "--value"], 5).unwrap_or_default().trim().to_string();
        }
        self.lock_monitor();
        self.update_night();
        self.check_idle();
        self.apply_lights();
        self.check_games();
        let cfg_name = config::path().file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        loop {
            self.player_start();
            if self.next_night.is_some_and(|t| Instant::now() >= t) {
                self.update_night();
                self.apply_lights();
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
            let run_at = fds.len();
            let run_fds = self.running.as_ref().map(|r| r.fds()).unwrap_or_default();
            for fd in &run_fds {
                fds.push((*fd, libc::POLLIN));
            }
            let now = Instant::now();
            // Nothing wakes this on a timer by itself: only what's due below (the screen, Night light's next change,
            // the idle light-off, the next page, a look at the programs when no window list can be had).
            let mut wake = now + Duration::from_secs(3600);
            if let Some(t) = self.next_night {
                wake = wake.min(t);
            }
            if let Some(t) = self.running.as_ref().and_then(|r| r.next_scan()) {
                wake = wake.min(t);
            }
            if let Some((_, until)) = &self.preview {
                wake = wake.min(*until);
            }
            if self.screen_on() {
                wake = wake.min(self.next_draw);
            }
            if self.player.is_none() && self.player_tries <= 20 && self.root.fixture.is_none() && crate::which("playerctl") {
                wake = wake.min(self.player_next);
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
                if names.iter().any(|n| n == "g15-record.json") {
                    self.record_request(&root::runtime_dir().join("g15-record.json"));
                }
                if names.iter().any(|n| n == "g15-show.json") {
                    self.preview_from(&root::runtime_dir().join("g15-show.json"));
                }
                if names.iter().any(|n| n == "desktop.json") && config::flag(&self.cfg, "nightDim", false) {
                    self.update_night();
                }
            }
            if self.running.is_some() {
                let rr: Vec<i16> = (0..run_fds.len()).map(|i| rev.get(run_at + i).copied().unwrap_or(0)).collect();
                if self.running.as_mut().map(|r| r.handle(&rr)).unwrap_or(false) {
                    self.check_games();
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
                        let me = lights_auto::session_path(&self.session);
                        for l in &lines {
                            for ev in lights_auto::parse_login1(l, &me) {
                                self.on_login1(ev);
                            }
                        }
                    }
                    None => {
                        crate::log("logind's signals ended (gdbus monitor)");
                        self.lockmon = None;
                    }
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
            self.apply_lights();
        }
    }

    /// The app's "Record" and "Stop" ($XDG_RUNTIME_DIR/hearth/g15-record.json): {"g": 9} starts recording for G9,
    /// {"stop": true} keeps it there, {"cancel": true} throws it away.
    fn record_request(&mut self, p: &Path) {
        let Ok(v) = serde_json::from_str::<Value>(&std::fs::read_to_string(p).unwrap_or_default()) else { return };
        if let Some(g) = v.get("g").and_then(|g| g.as_u64()).filter(|&g| g >= 1 && g <= self.dev.model.gkeys as u64) {
            if !self.rec.recording() {
                self.rec.mr(now_ms());
                self.start_recording_fds();
            }
            self.rec_target = Some(g as u8);
        } else if v.get("stop").and_then(|x| x.as_bool()) == Some(true) {
            if let Some(g) = self.rec_target.take() {
                self.keep_recording(g);
            }
        } else if v.get("cancel").and_then(|x| x.as_bool()) == Some(true) && self.rec.recording() {
            self.rec_target = None;
            self.rec.cancel();
            self.stop_recording_fds();
        }
        self.last = None;
        self.next_draw = Instant::now();
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

/// CLOCK_MONOTONIC in µs (logind's IdleSinceHintMonotonic is on this clock).
fn mono_us() -> u64 {
    let mut ts = libc::timespec { tv_sec: 0, tv_nsec: 0 };
    unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut ts) };
    ts.tv_sec as u64 * 1_000_000 + ts.tv_nsec as u64 / 1000
}

/// Seconds every screen of the PC has been asleep (DPMS off in /sys/class/drm), else 0: for "turn the light off
/// after N minutes idle" when logind doesn't say the PC is idle. Read once a minute, only while that setting is on.
fn drm_idle_for(root: &Root) -> u64 {
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
        "weather.temp": 41, "weather.unit": "°F", "weather.sky": "☁", "cpu.name": "R7 5700", "gpu.name": "RX 6750 XT",
        "history": {"gpu.temp": [52, 54, 55, 58, 61, 63, 62, 64, 66, 65, 66, 67, 66, 66, 65, 66]}
    }))?)?;
    std::fs::write(dir.join("media.json"), r#"{"status": "Playing", "title": "Hearthfire", "artist": "Hearth OS trailer", "position": 84, "length": 200}"#)?;
    // Night light as Settings › Display has it on the mockups' PC (for "Dim the keys with Night light").
    std::fs::write(dir.join("desktop.json"), r#"{"nightLight": true, "nightMode": "times", "nightFrom": "21:00", "nightTo": "07:00"}"#)?;
    std::fs::write(dir.join("notifications.json"), r#"{"n:7": {"app": "Hearth", "summary": "Update ready:", "body": "Restart to finish", "time": 0}}"#)?;
    Ok(())
}

/// Renders a page of the pretend (or real) keyboard: the CI pictures and `hearth-g15 frame`.
fn no_device() -> Device {
    Device { model: device::model_of(0xc222).unwrap(), hid: None, hidraw: None, keys: None, leds: Default::default() }
}

/// The app's pictures of the screen: pages and your own screens drawn as the keyboard would show them, with the
/// same readings (kept between pictures, so the use numbers and graphs fill in). Works without a keyboard.
pub struct Previewer {
    s: Service,
}

impl Previewer {
    pub fn new(root: Root) -> Previewer {
        let dev = device::find(&root).unwrap_or_else(no_device);
        let mut s = Service::new(root, dev);
        s.fixture_song();
        s.check_alerts(true);
        Previewer { s }
    }
    /// The app's copy of g15.json (what it just changed, before the file has it).
    pub fn reload_with(&mut self, cfg: Value) {
        self.s.cfg = cfg;
    }
    /// g15.json changed (the app saved something).
    pub fn reload(&mut self) {
        self.s.cfg = config::load();
        self.s.check_alerts(true);
    }
    /// A built-in page, a message ("rec", "set", "alert") or your own screen, by its id.
    pub fn page(&mut self, id: &str) -> Frame {
        let own = screens_of(&self.s.cfg);
        let els = match id {
            "rec" | "set" | "alert" => screen::built_in(id).unwrap_or_default(),
            _ => screen::elements_of(id, &own),
        };
        let tab = self.s.pages().iter().position(|p| p == id).unwrap_or(0);
        self.draw(&els, tab)
    }
    pub fn elements(&mut self, els: &[Value]) -> Frame {
        self.draw(els, 0)
    }
    fn draw(&mut self, els: &[Value], tab: usize) -> Frame {
        let r = self.s.reading(els);
        let own = screens_of(&self.s.cfg);
        let tabs: Vec<String> = self.s.pages().iter().map(|p| screen::tab_name(p, &own)).collect();
        let c = screen::Ctx { r: &r, history: &self.s.sampler.history, now: hearth_ui::time::now(), tabs, tab };
        let mut f = Frame::new();
        screen::draw(&mut f, els, &c);
        f
    }
}

/// What the keyboard's screen shows right now (the service's last frame), if it's running.
pub fn frame_now() -> Option<Frame> {
    let p = root::runtime_dir().join("g15-frame.bin");
    let age = std::fs::metadata(&p).ok()?.modified().ok()?.elapsed().ok()?;
    if age > Duration::from_secs(120) {
        return None;
    }
    lcd::unpack(&std::fs::read(p).ok()?)
}

pub fn frame_of(root: Root, page: &str) -> Option<Frame> {
    let dev = device::find(&root).unwrap_or_else(no_device);
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
    fn lights_go_off_and_come_back_as_you_had_them_without_saving() {
        let d = tempfile::tempdir().unwrap();
        make_fixture(d.path()).unwrap();
        std::fs::write(d.path().join("desktop.json"), r#"{"nightLight": false}"#).unwrap();
        let root = Root::at(d.path());
        let dev = device::find(&root).expect("the pretend keyboard");
        let (kbd, lcd) = (dev.leds.kbd.clone().unwrap(), dev.leds.lcd.clone().unwrap());
        let mut s = Service::new(root, dev);
        s.cfg = json!({"kbdLight": 2, "lcdLight": 2, "lightsOffWhenLocked": true, "nightDim": true});
        s.kbd_user = Some(2);
        s.lcd_user = Some(2);
        s.apply_lights();
        assert_eq!((leds::level(&kbd), leds::level(&lcd)), (Some(2), Some(2)));
        // Night light on: the keys go to Low, the screen stays as it is.
        s.night_on = true;
        s.apply_lights();
        assert_eq!((leds::level(&kbd), leds::level(&lcd)), (Some(1), Some(2)));
        // Locked: both off; unlocked: back (still Low at night), and High once Night light ends.
        s.on_login1(Login1::Locked(true));
        assert_eq!((leds::level(&kbd), leds::level(&lcd)), (Some(0), Some(0)));
        assert!(!s.screen_on(), "nothing is drawn while locked with the lights off");
        s.on_login1(Login1::Locked(false));
        assert_eq!((leds::level(&kbd), leds::level(&lcd)), (Some(1), Some(2)));
        s.night_on = false;
        s.apply_lights();
        assert_eq!(leds::level(&kbd), Some(2));
        // Asleep: off; awake: written again even if the keyboard lost them meanwhile.
        s.on_login1(Login1::Sleep(true));
        assert_eq!((leds::level(&kbd), leds::level(&lcd)), (Some(0), Some(0)));
        leds::set(&kbd, 1).unwrap();
        s.on_login1(Login1::Sleep(false));
        assert_eq!((leds::level(&kbd), leds::level(&lcd)), (Some(2), Some(2)));
        // Without "lights off when locked", locking changes nothing.
        s.cfg = json!({"kbdLight": 2, "lcdLight": 2});
        s.on_login1(Login1::Locked(true));
        assert_eq!(leds::level(&kbd), Some(2));
        // None of it was saved (red flag 9): no g15.json was made next to the pretend keyboard.
        assert!(!d.path().join("g15.json").exists() && !d.path().join("g15.json.bak").exists());
    }

    #[test]
    fn a_game_picks_its_set_live_and_gives_it_back() {
        let d = tempfile::tempdir().unwrap();
        make_fixture(d.path()).unwrap();
        let root = Root::at(d.path());
        let dev = device::find(&root).unwrap();
        let m2 = dev.leds.m[1].clone().unwrap();
        let mut s = Service::new(root, dev);
        s.cfg = json!({"set": 1, "gameSets": [{"game": "Trains 2010", "set": 2}], "mLights": true});
        s.set = 1;
        let run = d.path().join("run");
        std::fs::create_dir_all(run.join("hearth")).unwrap();
        let proc_ = d.path().join("fakeproc");
        std::fs::create_dir_all(proc_.join("77")).unwrap();
        std::fs::write(proc_.join("77/comm"), "TRS2010.exe\n").unwrap();
        std::fs::write(proc_.join("77/cmdline"), "TRS2010.exe").unwrap();
        std::fs::write(run.join("hearth/windows.json"), r#"{"windows": [{"appId": "trs2010.exe", "title": "Trainz Simulator 2010", "pid": 77}]}"#).unwrap();
        s.running = Some(Running::start(running::Paths { run: run.clone(), home: d.path().join("home"), proc_: proc_.clone(), link: d.path().join("link"), wayland: false }));
        s.check_games();
        assert_eq!(s.active_set(), 2);
        assert_eq!(leds::level(&m2), Some(1));
        // It closes: the window list says so, and M1 is back.
        std::fs::write(run.join("hearth/.t"), r#"{"windows": [{"appId": "firefox", "title": "News", "pid": 0}]}"#).unwrap();
        std::fs::rename(run.join("hearth/.t"), run.join("hearth/windows.json")).unwrap();
        let fds = s.running.as_ref().unwrap().fds();
        let rev = input::poll(&fds.iter().map(|&f| (f, libc::POLLIN)).collect::<Vec<_>>(), 1000);
        assert!(s.running.as_mut().unwrap().handle(&rev));
        s.check_games();
        assert_eq!(s.active_set(), 1);
        assert_eq!(leds::level(&m2), Some(0));
        assert!(!d.path().join("g15.json").exists(), "the game's set is never saved");
        // No rules: nothing is followed.
        s.cfg = json!({"set": 1});
        s.check_games();
        assert!(s.running.is_none());
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
