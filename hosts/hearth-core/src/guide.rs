//! Watches controllers for the Guide (Xbox / PS / Home) button, so Hearth can come back over a running game the
//! way the Steam button does on a Steam Deck, and passes every button on while the overlay is open (a window
//! gamescope draws on top doesn't reliably get the controller through the web view). Reads the kernel's event
//! devices directly (readable by the signed-in user). Port of hosts/linux/src/guide.js.

use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

const EV_KEY: u16 = 1;
const EV_ABS: u16 = 3;
const BTN_GAMEPAD: usize = 0x130;
const BTN_MODE: u16 = 0x13c;
const BTN_SELECT: u16 = 0x13a;
/// The Xbox button of an Xbox controller over Bluetooth without xpadneo (hid-generic / hid-microsoft): Guide too.
const KEY_HOMEPAGE: u16 = 0xac;
const BTN_SOUTH: usize = 0x130;
const SONY: &str = "054c";
/// How long Guide is held before it counts as a hold (Quick Access) rather than a tap (the glance).
pub const HOLD_MS: u64 = 350;
pub const EVENT_SIZE: usize = 24;   // struct input_event on 64-bit: timeval (16) + type (2) + code (2) + value (4)

/// Does a /sys capabilities bitmap ("key": hex words, most significant first) have this bit?
pub fn has_bit(caps: &str, bit: usize) -> bool {
    let words: Vec<&str> = caps.split_whitespace().rev().collect();
    words.get(bit / 64).and_then(|w| u64::from_str_radix(w, 16).ok()).map(|w| (w >> (bit % 64)) & 1 == 1).unwrap_or(false)
}

/// A device that has a Guide button: BTN_MODE, or KEY_HOMEPAGE on a controller (an Xbox controller over Bluetooth
/// without xpadneo: on the controller itself or on its "Consumer Control" device next to it). Keyboards with a Home
/// page key don't count: the device must have gamepad buttons or be named as a controller.
pub fn has_guide(caps: &str, name: &str) -> bool {
    if has_bit(caps, BTN_MODE as usize) { return true; }
    let named = { let l = name.to_lowercase(); l.contains("controller") || l.contains("gamepad") || l.contains("xbox") };
    has_bit(caps, KEY_HOMEPAGE as usize) && (has_bit(caps, BTN_SOUTH) || named)
}

/// Steam's virtual controllers (Steam Input: vendor 28de, product 11ff), the ones a Steam game reads the controller
/// through. `sys` is /sys/class/input, `event` an event device's name there ("event21").
pub fn is_steam_virtual(sys: &Path, event: &str) -> bool {
    let id = |f: &str| std::fs::read_to_string(sys.join(event).join("device/id").join(f)).map(|s| s.trim().to_ascii_lowercase()).unwrap_or_default();
    id("vendor") == "28de" && id("product") == "11ff"
}

/// Event devices with a Guide button: (path, name)
pub fn gamepad_devices(sys: &Path) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(sys).into_iter().flatten().flatten() {
        let n = e.file_name().to_string_lossy().to_string();
        if !n.starts_with("event") { continue; }
        let Ok(caps) = std::fs::read_to_string(sys.join(&n).join("device/capabilities/key")) else { continue };
        let name = std::fs::read_to_string(sys.join(&n).join("device/name")).map(|s| s.trim().to_string()).ok().filter(|s| !s.is_empty()).unwrap_or(n.clone());
        if !has_guide(&caps, &name) { continue; }
        out.push((format!("/dev/input/{n}"), name));
    }
    let _ = BTN_GAMEPAD;
    out
}

/// Button names as Hearth's pages use them. Triggers and stick presses too (the on-screen keyboard over a game uses
/// LT shift, RT symbols, L3 emoji, R3 layout); the d-pad also as xpad's BTN_TRIGGER_HAPPY1-4.
fn key_name(code: u16) -> Option<&'static str> {
    Some(match code { 0x130 => "a", 0x131 => "b", 0x133 => "x", 0x134 => "y", 0x136 => "lb", 0x137 => "rb", 0x138 => "lt", 0x139 => "rt",
        0x13a => "select", 0x13b => "start", 0x13c | KEY_HOMEPAGE => "guide", 0x13d => "l3", 0x13e => "r3",
        0x220 | 0x2c2 => "up", 0x221 | 0x2c3 => "down", 0x222 | 0x2c0 => "left", 0x223 | 0x2c1 => "right", _ => return None })
}

/// The name of a key code on this controller: Sony pads (hid-playstation) have Triangle (Y's place) on BTN_NORTH and
/// Square (X's place) on BTN_WEST, the other way round from xpad (X and Y were swapped in Quick Access).
pub fn button_for(code: u16, sony: bool) -> Option<&'static str> {
    match key_name(code)? { "x" if sony => Some("y"), "y" if sony => Some("x"), b => Some(b) }
}

/// Controller input in raw input_event records: (button, down). The d-pad comes as hat axes or buttons, the
/// left stick as directions too. `state` keeps the axes between reads.
pub fn pad_events(buf: &[u8], state: &mut HashMap<&'static str, i32>) -> Vec<(&'static str, bool)> {
    let mut out = Vec::new();
    let mut dir = |out: &mut Vec<(&'static str, bool)>, state: &mut HashMap<&'static str, i32>, neg: &'static str, pos: &'static str, v: i32, key: &'static str| {
        let was = *state.get(key).unwrap_or(&0);
        let now = v.signum();
        if was == now { return; }
        if was != 0 { out.push((if was < 0 { neg } else { pos }, false)); }
        if now != 0 { out.push((if now < 0 { neg } else { pos }, true)); }
        state.insert(key, now);
    };
    for rec in buf.chunks_exact(EVENT_SIZE) {
        let ty = u16::from_le_bytes([rec[16], rec[17]]);
        let code = u16::from_le_bytes([rec[18], rec[19]]);
        let value = i32::from_le_bytes([rec[20], rec[21], rec[22], rec[23]]);
        if ty == EV_KEY && value != 2 { if let Some(b) = button_for(code, state.get("sony") == Some(&1)) { out.push((b, value == 1)); } }
        else if ty == EV_ABS {
            match code {
                // Analog triggers (only where read_triggers found them: Z/RZ is the right stick on some pads).
                2 | 5 | 9 | 10 => {
                    let max_key = match code { 2 => "z_trig", 5 => "rz_trig", 10 => "lt_max", _ => "rt_max" };
                    let (on_key, name) = if code == 2 || code == 10 { ("lt_on", "lt") } else { ("rt_on", "rt") };
                    let Some(&max) = state.get(max_key).filter(|m| **m > 0) else { continue };
                    let was = state.get(on_key) == Some(&1);
                    let now = if was { value * 10 > max * 4 } else { value * 20 > max * 11 };
                    if now != was { state.insert(on_key, now as i32); out.push((name, now)); }
                }
                16 => dir(&mut out, state, "left", "right", value, "hx"),
                17 => dir(&mut out, state, "up", "down", value, "hy"),
                0 | 1 => {
                    // xpad-style sticks go -32768..32767; some 0..65535. Past ~60% counts as a direction.
                    if value > 40000 { state.insert("unsigned", 1); }
                    let v = if state.get("unsigned") == Some(&1) { value - 32768 } else { value };
                    let k = if code == 0 { "sx" } else { "sy" };
                    let d = if v < -20000 { -1 } else if v > 20000 { 1 } else if v.abs() < 12000 { 0 } else { *state.get(k).unwrap_or(&0) };
                    if code == 0 { dir(&mut out, state, "left", "right", d, "sx"); } else { dir(&mut out, state, "up", "down", d, "sy"); }
                }
                _ => {}
            }
        }
    }
    out
}

/// Drops presses made while Guide is held (Handheld Daemon's Guide + A chord); Guide itself and every release
/// still pass. `held` keeps whether Guide is down between reads.
pub fn chord_filter(events: Vec<(&'static str, bool)>, held: &mut bool) -> Vec<(&'static str, bool)> {
    let mut out = Vec::new();
    for (b, down) in events {
        if b == "guide" { *held = down; out.push((b, down)); }
        else if !(*held && down) { out.push((b, down)); }
    }
    out
}

/// What a Guide press or release means now: pressed (`down`), with tap/hold on (`timed`: the glance is on and the
/// overlay isn't open), whether this press is being timed (`timing`) and whether its hold already fired.
#[derive(Debug, PartialEq)]
pub enum GuideNow { Press, Wait, Tap, Nothing }
pub fn tap_or_hold(down: bool, timed: bool, timing: bool, fired: bool) -> GuideNow {
    if down { return if timed { GuideNow::Wait } else { GuideNow::Press }; }
    if timing && !fired { GuideNow::Tap } else { GuideNow::Nothing }
}

/// What the Guide button does: "guide" (pressed, or held with the glance on), "guide-tap" (a short press with the
/// glance on), "osk" (Guide + X: the on-screen keyboard), "view" (View held a second, if turned on) or "hotkey"; and every
/// button while `buttons` is on (for the overlay).
pub type OnPress = Arc<dyn Fn(&str) + Send + Sync>;
pub type OnButton = Arc<dyn Fn(&str, bool) + Send + Sync>;

extern "C" { fn ioctl(fd: i32, request: u64, ...) -> i32; }
const EVIOCGRAB: u64 = 0x4004_4590;   // _IOW('E', 0x90, int)
const EVIOCGKEY: u64 = 0x8060_4518;   // _IOR('E', 0x18, 96 bytes): the keys held down now
fn eviocgabs(code: u16) -> u64 { 0x8018_4540 + code as u64 }   // _IOR('E', 0x40 + code, struct input_absinfo)

/// What pad_events needs to know about a device: Sony (X and Y swapped) and which axes are analog triggers, with
/// their top value. Z/RZ are triggers only when the device also has RX (otherwise they're the right stick: an Xbox
/// controller over Bluetooth with hid-generic).
fn setup_state(path: &str, fd: i32, state: &mut HashMap<&'static str, i32>) {
    let ev = path.rsplit('/').next().unwrap_or("");
    let dev = Path::new("/sys/class/input").join(ev).join("device");
    if std::fs::read_to_string(dev.join("id/vendor")).map(|v| v.trim() == SONY).unwrap_or(false) { state.insert("sony", 1); }
    let abs = std::fs::read_to_string(dev.join("capabilities/abs")).unwrap_or_default();
    let has_rx = has_bit(&abs, 3);
    for (code, key) in [(2u16, "z_trig"), (5, "rz_trig"), (10, "lt_max"), (9, "rt_max")] {
        if !has_bit(&abs, code as usize) || ((code == 2 || code == 5) && !has_rx) { continue; }
        let mut info = [0i32; 6];
        if unsafe { ioctl(fd, eviocgabs(code), info.as_mut_ptr()) } == 0 && info[1] >= 0 && info[2] > info[1] { state.insert(key, info[2]); }
    }
}

/// Is Guide held down on this device right now?
pub fn guide_down(fd: i32) -> bool { key_down(fd, BTN_MODE) || key_down(fd, KEY_HOMEPAGE) }

/// Is this key held down on the device right now (EVIOCGKEY)? Works whoever has it grabbed, and whatever we saw.
pub fn key_down(fd: i32, code: u16) -> bool {
    let mut bits = [0u8; 96];
    if unsafe { ioctl(fd, EVIOCGKEY, bits.as_mut_ptr()) } < 0 { return false; }
    bits.get(code as usize / 8).map(|b| b >> (code % 8) & 1 == 1).unwrap_or(false)
}

#[derive(Clone)]
pub struct Guide {
    /// The controllers' event devices, while they're read: grabbed while the overlay is open, so only Hearth
    /// gets their buttons (the game underneath doesn't).
    fds: Arc<Mutex<HashMap<String, i32>>>,
    pub hold_view: Arc<AtomicBool>,
    pub buttons: Arc<AtomicBool>,
    /// Handheld Daemon (the HHD editions): without an overlay app its Quick Access button sends Guide + A (Steam's
    /// chord for its Quick Access menu), so buttons pressed while Guide is held aren't passed on (the A would pick
    /// the first thing in Quick Access). With Hearth's overlay app installed as HHD's (overlay_link.rs) that button
    /// opens Quick Access through the app instead, and Guide still comes through here. Off with InputPlumber
    /// (Bazzite), where nothing changes.
    pub chord_guard: Arc<AtomicBool>,
    /// Quick glance (on by default; Settings › Controls): a short Guide press is told apart from a held one. Short
    /// ("guide-tap", on release) shows the glance bar over a game; held for HOLD_MS ("guide") opens Quick Access.
    /// Off: every press is "guide" the moment it's pressed, as before. While the overlay is open (grabbed) a press
    /// is "guide" at once too (it closes it either way).
    pub tap_hold: Arc<AtomicBool>,
    open: Arc<Mutex<HashMap<String, String>>>,
    /// Grabbed now (the overlay is open). Not the same as `buttons`: in multi-seat every button is passed on
    /// while nothing is grabbed (games still get the controller).
    grabbed: Arc<AtomicBool>,
    problems: Arc<Mutex<HashMap<String, String>>>,
    last_press: Arc<Mutex<u64>>,
    /// Steam's virtual controllers among the devices read (is_steam_virtual), and whether they're held now
    /// (hold_steam_pads).
    steam_pads: Arc<Mutex<HashSet<String>>>,
    steam_held: Arc<AtomicBool>,
}

impl Default for Guide { fn default() -> Self { Self::new() } }

impl Guide {
    pub fn new() -> Guide { Guide { fds: Arc::default(), hold_view: Arc::default(), buttons: Arc::default(), chord_guard: Arc::default(), tap_hold: Arc::default(), open: Arc::default(), grabbed: Arc::default(), problems: Arc::default(), last_press: Arc::default(), steam_pads: Arc::default(), steam_held: Arc::default() } }
    /// For Settings › Controls: which devices are watched, and when Guide last came through.
    pub fn status(&self) -> Value {
        json!({"devices": self.open.lock().unwrap().values().cloned().collect::<Vec<_>>(),
               "problems": self.problems.lock().unwrap().iter().map(|(p, w)| format!("{p}: {w}")).collect::<Vec<_>>(), "lastPress": *self.last_press.lock().unwrap()})
    }
    pub fn connected(&self) -> usize { self.open.lock().unwrap().len() }
    /// Only Hearth gets the controllers' input (on), or everyone again (off).
    pub fn grab(&self, on: bool) {
        self.grabbed.store(on, Ordering::SeqCst);
        for fd in self.fds.lock().unwrap().values() { unsafe { ioctl(*fd, EVIOCGRAB, if on { 1i32 } else { 0i32 }); } }
    }
    /// Quick Access open over a game on the Handheld Daemon editions: HHD holds the real controllers, and Steam keeps
    /// running now (steam_thaw.rs; HHD used to stop it), so Steam's virtual controllers are held here meanwhile: a
    /// Steam game doesn't get the presses meant for Quick Access. Their presses are dropped too (HHD hands Quick
    /// Access the real controller's; these would be the same presses twice).
    pub fn hold_steam_pads(&self, on: bool) {
        self.steam_held.store(on, Ordering::SeqCst);
        let pads: Vec<String> = self.steam_pads.lock().unwrap().iter().cloned().collect();
        let fds = self.fds.lock().unwrap();
        for p in pads { if let Some(fd) = fds.get(&p) { unsafe { ioctl(*fd, EVIOCGRAB, if on { 1i32 } else { 0i32 }); } } }
    }
    pub fn steam_pads_held(&self) -> bool { self.steam_held.load(Ordering::SeqCst) }
    pub fn fire(&self, how: &str, on_press: &OnPress) { *self.last_press.lock().unwrap() = crate::library::now_ms(); on_press(how); }

    /// Watches controllers as they come and go (looks every 5 seconds).
    pub fn start(&self, on_press: OnPress, on_button: OnButton) {
        let me = self.clone();
        std::thread::spawn(move || loop {
            for (path, name) in gamepad_devices(Path::new("/sys/class/input")) {
                if me.open.lock().unwrap().contains_key(&path) { continue; }
                match std::fs::File::open(&path) {
                    Err(e) => { me.problems.lock().unwrap().insert(path.clone(), if e.kind() == std::io::ErrorKind::PermissionDenied { "no permission".into() } else { e.to_string() }); }
                    Ok(f) => {
                        me.problems.lock().unwrap().remove(&path);
                        me.open.lock().unwrap().insert(path.clone(), name);
                        let (me2, press, button) = (me.clone(), on_press.clone(), on_button.clone());
                        std::thread::spawn(move || me2.read(f, &path, press, button));
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_secs(5));
        });
    }

    fn read(&self, mut f: std::fs::File, path: &str, on_press: OnPress, on_button: OnButton) {
        use std::os::unix::io::AsRawFd;
        self.fds.lock().unwrap().insert(path.to_string(), f.as_raw_fd());
        // A controller plugged in while the overlay is open is grabbed straight away.
        if self.grabbed.load(Ordering::SeqCst) { unsafe { ioctl(f.as_raw_fd(), EVIOCGRAB, 1i32); } }
        let fd = f.as_raw_fd();
        // Steam's virtual controller (Steam starts one with each game): held straight away too while Quick Access is
        // open on the HHD editions (hold_steam_pads).
        let steam_pad = path.strip_prefix("/dev/input/").map(|e| is_steam_virtual(Path::new("/sys/class/input"), e)).unwrap_or(false);
        if steam_pad {
            self.steam_pads.lock().unwrap().insert(path.to_string());
            if self.steam_held.load(Ordering::SeqCst) { unsafe { ioctl(f.as_raw_fd(), EVIOCGRAB, 1i32); } }
        }
        let mut buf = [0u8; EVENT_SIZE * 64];
        let mut rest: Vec<u8> = Vec::new();
        let mut state = HashMap::new();
        setup_state(path, f.as_raw_fd(), &mut state);
        let view_gen = Arc::new(std::sync::atomic::AtomicU64::new(0));
        // Guide's own press number: a hold fires when this press is still the latest (not released) after HOLD_MS.
        let mode_gen = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let mode_fired = Arc::new(AtomicBool::new(false));
        let mut mode_timed = false;
        // Another button pressed while Guide is held (Handheld Daemon's Guide + A / Guide + B shortcuts): that press
        // of Guide was a shortcut, neither a tap nor a hold (HHD acts on it itself).
        let mode_chord = Arc::new(AtomicBool::new(false));
        let mut guide_held = false;
        loop {
            let n = match f.read(&mut buf) { Ok(0) | Err(_) => break, Ok(n) => n };
            // Held for Quick Access (hold_steam_pads): the real controller's presses already reach it through HHD.
            if steam_pad && self.steam_held.load(Ordering::SeqCst) { rest.clear(); continue; }
            rest.extend_from_slice(&buf[..n]);
            let whole = rest.len() - rest.len() % EVENT_SIZE;
            let chunk: Vec<u8> = rest.drain(..whole).collect();
            // One record at a time, so a press in the same read as Guide's is judged in order.
            let guard = self.chord_guard.load(Ordering::SeqCst);
            let mut presses = Vec::new();
            let mut osk = false;
            for rec in chunk.chunks_exact(EVENT_SIZE) {
                let raw = pad_events(rec, &mut state);
                if raw.iter().any(|(b, down)| *down && *b == "guide") { mode_chord.store(false, Ordering::SeqCst); }   // a new press of Guide
                else if raw.iter().any(|(b, down)| *down && *b == "x") && (guide_held || guide_down(fd)) {
                    // Guide + X: the on-screen keyboard (like Steam + X). That press of Guide is neither a tap nor a
                    // hold, and the X goes nowhere else. Guide held is asked from the device too, so it counts when
                    // Hearth Input had the controller as Guide went down.
                    mode_chord.store(true, Ordering::SeqCst);
                    osk = true;
                    continue;
                }
                else if guard && guide_held && raw.iter().any(|(b, down)| *down && *b != "guide") { mode_chord.store(true, Ordering::SeqCst); }
                if guard { presses.extend(chord_filter(raw, &mut guide_held)); }
                else {
                    for (b, down) in &raw { if *b == "guide" { guide_held = *down; } }
                    presses.extend(raw);
                }
            }
            if osk { self.fire("osk", &on_press); }
            if self.buttons.load(Ordering::SeqCst) { for (b, down) in presses { on_button(b, down); } }
            for rec in chunk.chunks_exact(EVENT_SIZE) {
                let (ty, code, value) = (u16::from_le_bytes([rec[16], rec[17]]), u16::from_le_bytes([rec[18], rec[19]]), i32::from_le_bytes([rec[20], rec[21], rec[22], rec[23]]));
                if ty != EV_KEY { continue; }
                if (code == BTN_MODE || code == KEY_HOMEPAGE) && (value == 0 || value == 1) {
                    let g = mode_gen.fetch_add(1, Ordering::SeqCst) + 1;
                    let fired = mode_fired.load(Ordering::SeqCst) || mode_chord.load(Ordering::SeqCst);
                    match tap_or_hold(value == 1, self.tap_hold.load(Ordering::SeqCst) && !self.grabbed.load(Ordering::SeqCst), mode_timed, fired) {
                        GuideNow::Press => { mode_timed = false; self.fire("guide", &on_press); }
                        GuideNow::Wait => {
                            mode_timed = true; mode_fired.store(false, Ordering::SeqCst);
                            let (mg, fired, chord, me, press) = (mode_gen.clone(), mode_fired.clone(), mode_chord.clone(), self.clone(), on_press.clone());
                            std::thread::spawn(move || {
                                std::thread::sleep(std::time::Duration::from_millis(HOLD_MS));
                                if mg.load(Ordering::SeqCst) == g && !chord.load(Ordering::SeqCst) && !fired.swap(true, Ordering::SeqCst) { me.fire("guide", &press); }
                            });
                        }
                        GuideNow::Tap => { mode_timed = false; if !mode_fired.swap(true, Ordering::SeqCst) { self.fire("guide-tap", &on_press); } }
                        GuideNow::Nothing => { mode_timed = false; }
                    }
                }
                if code == BTN_SELECT {
                    let g = view_gen.fetch_add(1, Ordering::SeqCst) + 1;
                    if value == 1 && self.hold_view.load(Ordering::SeqCst) {
                        let (vg, me, press) = (view_gen.clone(), self.clone(), on_press.clone());
                        std::thread::spawn(move || { std::thread::sleep(std::time::Duration::from_millis(1000)); if vg.load(Ordering::SeqCst) == g { me.fire("view", &press); } });
                    }
                }
            }
        }
        self.fds.lock().unwrap().remove(path);
        self.open.lock().unwrap().remove(path);
        if steam_pad { self.steam_pads.lock().unwrap().remove(path); }
    }
}

// ------------------------------------------------------------------ power and volume buttons (Game Mode)

/// Runs desktop/shell/hearth-buttons (reads the buttons from the kernel) and passes on its JSON lines:
/// {t:'power', press:'long'|'short'}, {t:'vol', d}, {t:'mute'}, {t:'devices', list, problems, power}; {t:'ignored', after}
/// (a power press just after waking up, the one that woke it) is only logged.
pub fn start_buttons(on: Arc<dyn Fn(Value) + Send + Sync>, devices: Arc<Mutex<Value>>) {
    let helper = crate::paths::desktop_dir().join("shell/hearth-buttons");
    if !helper.exists() { *devices.lock().unwrap() = json!({"devices": [], "problems": ["hearth-buttons is missing"]}); return; }
    std::thread::spawn(move || loop {
        let child = std::process::Command::new("python3").arg(&helper).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::null()).spawn();
        let Ok(mut child) = child else { return };
        if let Some(out) = child.stdout.take() {
            use std::io::BufRead;
            for line in std::io::BufReader::new(out).lines().map_while(Result::ok) {
                let Ok(m) = serde_json::from_str::<Value>(&line) else { continue };
                if m["t"] == "devices" {
                    crate::bridge::log(&format!("buttons: watching {}", m["list"]));
                    *devices.lock().unwrap() = json!({"devices": m["list"], "problems": m["problems"]});
                } else if m["t"] == "ignored" {
                    // The press that woke the device (hearth-buttons also wrote it to the journal as hearth-sleep).
                    crate::bridge::log(&format!("power button: press ignored {} s after waking up (the press that woke it)", m["after"]));
                } else { on(m); }
            }
        }
        let code = child.wait().ok().and_then(|s| s.code());
        if code == Some(2) { return; }   // python3-evdev missing: trying again won't help
        std::thread::sleep(std::time::Duration::from_secs(3));
    });
}

// ------------------------------------------------------------------ controller remapping (InputPlumber)

/// Button remaps and the controller type through desktop/shell/hearth-controller (shared with Desktop Mode).
pub fn controller(args: &[&str]) -> Result<Value, String> {
    let helper = crate::paths::desktop_dir().join("shell/hearth-controller");
    if !helper.exists() { return Err("Controller remapping needs Hearth, the OS.".into()); }
    let mut cmd = vec![helper.to_string_lossy().to_string()];
    cmd.extend(args.iter().map(|s| s.to_string()));
    let refs: Vec<&str> = cmd.iter().map(String::as_str).collect();
    let out = std::process::Command::new("python3").args(&refs).output().map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&out.stdout);
    let last = text.trim().lines().last().unwrap_or("").to_string();
    if let Some(e) = last.strip_prefix("error: ") { return Err(e.to_string()); }
    if !out.status.success() { return Err(format!("hearth-controller failed ({})", out.status)); }
    Ok(serde_json::from_str(&last).unwrap_or(Value::Null))
}
pub fn controller_available() -> bool { crate::paths::desktop_dir().join("shell/hearth-controller").exists() }

#[cfg(test)]
mod tests {
    use super::*;
    fn ev(ty: u16, code: u16, value: i32) -> Vec<u8> { let mut v = vec![0u8; 16]; v.extend(ty.to_le_bytes()); v.extend(code.to_le_bytes()); v.extend(value.to_le_bytes()); v }
    #[test]
    fn steam_virtual_controllers() {
        let sys = std::env::temp_dir().join(format!("hearth-guide-sys-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&sys);
        for (ev, vendor, product) in [("event21", "28de\n", "11ff\n"), ("event5", "045e\n", "0b13\n"), ("event9", "28DE", "1205")] {
            let id = sys.join(ev).join("device/id");
            std::fs::create_dir_all(&id).unwrap();
            std::fs::write(id.join("vendor"), vendor).unwrap();
            std::fs::write(id.join("product"), product).unwrap();
        }
        assert!(is_steam_virtual(&sys, "event21"));     // Steam Input's virtual gamepad
        assert!(!is_steam_virtual(&sys, "event5"));     // the Xbox controller itself
        assert!(!is_steam_virtual(&sys, "event9"));     // a real Steam Deck / Steam Controller: not a virtual one
        assert!(!is_steam_virtual(&sys, "event77"));    // gone
        let _ = std::fs::remove_dir_all(&sys);
        // Held and let go with nothing read: no panic, and the state is kept for devices that come later.
        let g = Guide::new();
        g.hold_steam_pads(true);
        assert!(g.steam_pads_held());
        g.hold_steam_pads(false);
        assert!(!g.steam_pads_held());
    }
    #[test]
    fn reads_controllers() {
        assert!(has_bit("10000000 0 0 0 0", 0x13c) == false);
        assert!(has_bit("1000000000000000 0 0 0 0", 0x13c));
        let mut st = HashMap::new();
        let mut buf = ev(1, 0x130, 1); buf.extend(ev(3, 17, -1)); buf.extend(ev(3, 17, 0)); buf.extend(ev(3, 0, -30000)); buf.extend(ev(1, 0x130, 2));
        assert_eq!(pad_events(&buf, &mut st), vec![("a", true), ("up", true), ("up", false), ("left", true)]);
    }
    #[test]
    fn more_controllers() {
        // Sony: BTN_NORTH is Triangle (Y), BTN_WEST Square (X).
        let mut st = HashMap::new();
        st.insert("sony", 1);
        let buf = [ev(1, 0x133, 1), ev(1, 0x134, 1)].concat();
        assert_eq!(pad_events(&buf, &mut st), vec![("y", true), ("x", true)]);
        // Triggers, stick presses, xpad's d-pad buttons, an Xbox controller's Guide over Bluetooth.
        let mut st = HashMap::new();
        let buf = [ev(1, 0x138, 1), ev(1, 0x13d, 1), ev(1, 0x2c2, 1), ev(1, 0xac, 1)].concat();
        assert_eq!(pad_events(&buf, &mut st), vec![("lt", true), ("l3", true), ("up", true), ("guide", true)]);
        // Analog triggers once their range is known (0..1023): down past 55%, up below 40%.
        st.insert("rz_trig", 1023);
        let buf = [ev(3, 5, 300), ev(3, 5, 700), ev(3, 5, 500), ev(3, 5, 100)].concat();
        assert_eq!(pad_events(&buf, &mut st), vec![("rt", true), ("rt", false)]);
        // Without it (Z/RZ is the right stick there) nothing.
        let mut st = HashMap::new();
        assert!(pad_events(&ev(3, 5, 1000), &mut st).is_empty());
        assert!(has_guide("1000000000000000 0 0 0 0", "x"));
        let home = "0 100000000000 0 0".to_string();   // bit 0xac (word 2, bit 44)
        assert!(has_bit(&home, 0xac));
        assert!(has_guide(&home, "Xbox Wireless Controller Consumer Control"));
        assert!(!has_guide(&home, "Logitech USB Keyboard Consumer Control"));
    }
    #[test]
    fn guide_tap_or_hold() {
        // The glance off (or the overlay open): a press is Quick Access at once, the release nothing.
        assert_eq!(tap_or_hold(true, false, false, false), GuideNow::Press);
        assert_eq!(tap_or_hold(false, false, false, false), GuideNow::Nothing);
        // On: a press waits; let go before HOLD_MS is a tap; after the hold fired, the release is nothing.
        assert_eq!(tap_or_hold(true, true, false, false), GuideNow::Wait);
        assert_eq!(tap_or_hold(false, true, true, false), GuideNow::Tap);
        assert_eq!(tap_or_hold(false, true, true, true), GuideNow::Nothing);
        // A release whose press wasn't timed (the glance turned on mid-press) is nothing.
        assert_eq!(tap_or_hold(false, true, false, false), GuideNow::Nothing);
    }
    #[test]
    fn handheld_daemon_chord() {
        // HHD's Quick Access button: Guide down, A down, A up, Guide up. Only Guide (and the releases) pass.
        let mut held = false;
        let evs = vec![("guide", true), ("a", true), ("a", false), ("guide", false), ("b", true)];
        assert_eq!(chord_filter(evs, &mut held), vec![("guide", true), ("a", false), ("guide", false), ("b", true)]);
        assert!(!held);
        let mut held = false;
        assert_eq!(chord_filter(vec![("guide", true)], &mut held), vec![("guide", true)]);
        assert_eq!(chord_filter(vec![("x", true)], &mut held), Vec::<(&str, bool)>::new());   // across reads
    }
}
