//! Watches controllers for the Guide (Xbox / PS / Home) button, so Hearth can come back over a running game the
//! way the Steam button does on a Steam Deck, and passes every button on while the overlay is open (a window
//! gamescope draws on top doesn't reliably get the controller through the web view). Reads the kernel's event
//! devices directly (readable by the signed-in user). Port of hosts/linux/src/guide.js.

use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

const EV_KEY: u16 = 1;
const EV_ABS: u16 = 3;
const BTN_GAMEPAD: usize = 0x130;
const BTN_MODE: u16 = 0x13c;
const BTN_SELECT: u16 = 0x13a;
pub const EVENT_SIZE: usize = 24;   // struct input_event on 64-bit: timeval (16) + type (2) + code (2) + value (4)

/// Does a /sys capabilities bitmap ("key": hex words, most significant first) have this bit?
pub fn has_bit(caps: &str, bit: usize) -> bool {
    let words: Vec<&str> = caps.split_whitespace().rev().collect();
    words.get(bit / 64).and_then(|w| u64::from_str_radix(w, 16).ok()).map(|w| (w >> (bit % 64)) & 1 == 1).unwrap_or(false)
}

/// Event devices with a Guide button: (path, name)
pub fn gamepad_devices(sys: &Path) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(sys).into_iter().flatten().flatten() {
        let n = e.file_name().to_string_lossy().to_string();
        if !n.starts_with("event") { continue; }
        let Ok(caps) = std::fs::read_to_string(sys.join(&n).join("device/capabilities/key")) else { continue };
        if !has_bit(&caps, BTN_MODE as usize) { continue; }
        let name = std::fs::read_to_string(sys.join(&n).join("device/name")).map(|s| s.trim().to_string()).ok().filter(|s| !s.is_empty()).unwrap_or(n.clone());
        out.push((format!("/dev/input/{n}"), name));
    }
    let _ = BTN_GAMEPAD;
    out
}

fn key_name(code: u16) -> Option<&'static str> {
    Some(match code { 0x130 => "a", 0x131 => "b", 0x133 => "x", 0x134 => "y", 0x136 => "lb", 0x137 => "rb", 0x13a => "select", 0x13b => "start",
        0x13c => "guide", 0x220 => "up", 0x221 => "down", 0x222 => "left", 0x223 => "right", _ => return None })
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
        if ty == EV_KEY && value != 2 { if let Some(b) = key_name(code) { out.push((b, value == 1)); } }
        else if ty == EV_ABS {
            match code {
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

/// What the Guide button does: "guide", "view" (View held a second, if turned on) or "hotkey"; and every
/// button while `buttons` is on (for the overlay).
pub type OnPress = Arc<dyn Fn(&str) + Send + Sync>;
pub type OnButton = Arc<dyn Fn(&str, bool) + Send + Sync>;

extern "C" { fn ioctl(fd: i32, request: u64, ...) -> i32; }
const EVIOCGRAB: u64 = 0x4004_4590;   // _IOW('E', 0x90, int)

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
    open: Arc<Mutex<HashMap<String, String>>>,
    /// Grabbed now (the overlay is open). Not the same as `buttons`: in multi-seat every button is passed on
    /// while nothing is grabbed (games still get the controller).
    grabbed: Arc<AtomicBool>,
    problems: Arc<Mutex<HashMap<String, String>>>,
    last_press: Arc<Mutex<u64>>,
}

impl Default for Guide { fn default() -> Self { Self::new() } }

impl Guide {
    pub fn new() -> Guide { Guide { fds: Arc::default(), hold_view: Arc::default(), buttons: Arc::default(), chord_guard: Arc::default(), open: Arc::default(), grabbed: Arc::default(), problems: Arc::default(), last_press: Arc::default() } }
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
        let mut buf = [0u8; EVENT_SIZE * 64];
        let mut rest: Vec<u8> = Vec::new();
        let mut state = HashMap::new();
        let view_gen = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let mut guide_held = false;
        loop {
            let n = match f.read(&mut buf) { Ok(0) | Err(_) => break, Ok(n) => n };
            rest.extend_from_slice(&buf[..n]);
            let whole = rest.len() - rest.len() % EVENT_SIZE;
            let chunk: Vec<u8> = rest.drain(..whole).collect();
            let presses = if self.chord_guard.load(Ordering::SeqCst) {
                // One record at a time, so a press in the same read as Guide's is judged in order.
                let mut all = Vec::new();
                for rec in chunk.chunks_exact(EVENT_SIZE) { all.extend(chord_filter(pad_events(rec, &mut state), &mut guide_held)); }
                all
            } else { pad_events(&chunk, &mut state) };
            if self.buttons.load(Ordering::SeqCst) { for (b, down) in presses { on_button(b, down); } }
            for rec in chunk.chunks_exact(EVENT_SIZE) {
                let (ty, code, value) = (u16::from_le_bytes([rec[16], rec[17]]), u16::from_le_bytes([rec[18], rec[19]]), i32::from_le_bytes([rec[20], rec[21], rec[22], rec[23]]));
                if ty != EV_KEY { continue; }
                if code == BTN_MODE && value == 1 { self.fire("guide", &on_press); }
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
    }
}

// ------------------------------------------------------------------ power and volume buttons (Game Mode)

/// Runs desktop/shell/hearth-buttons (reads the buttons from the kernel) and passes on its JSON lines:
/// {t:'power', press:'long'|'short'}, {t:'vol', d}, {t:'mute'}, {t:'devices', list, problems}.
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
    fn reads_controllers() {
        assert!(has_bit("10000000 0 0 0 0", 0x13c) == false);
        assert!(has_bit("1000000000000000 0 0 0 0", 0x13c));
        let mut st = HashMap::new();
        let mut buf = ev(1, 0x130, 1); buf.extend(ev(3, 17, -1)); buf.extend(ev(3, 17, 0)); buf.extend(ev(3, 0, -30000)); buf.extend(ev(1, 0x130, 2));
        assert_eq!(pad_events(&buf, &mut st), vec![("a", true), ("up", true), ("up", false), ("left", true)]);
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
