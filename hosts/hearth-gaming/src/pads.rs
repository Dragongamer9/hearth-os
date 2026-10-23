//! Controller Lab's reading of controllers on Linux: straight from evdev (/dev/input/event*), the way games and
//! SDL see them. One controller usually has several event devices (the pad, its motion sensors, its touchpad);
//! they're put together by the device they belong to. A background thread reads them all; the page asks for a
//! snapshot about 30 times a second.
//!
//! Also: rumble (force feedback, FF_RUMBLE), the driver's stick and trigger ranges and dead zones (EVIOCSABS, so
//! a calibration takes effect for anything that opens the controller afterwards), batteries (power_supply in
//! sysfs), and which driver (xpad, hid-playstation, hid-nintendo...) or virtual pad (Handheld Daemon...) it is.

use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

// ------------------------------------------------------------------ the kernel's evdev interface

const EV_SYN: u16 = 0x00;
const EV_KEY: u16 = 0x01;
const EV_ABS: u16 = 0x03;
const EV_FF: u16 = 0x15;
const SYN_DROPPED: u16 = 3;
const FF_RUMBLE: u16 = 0x50;
const FF_GAIN: u16 = 0x60;
const PROP_POINTER: usize = 0x00;
const PROP_BUTTONPAD: usize = 0x02;
const PROP_ACCELEROMETER: usize = 0x06;
const ABS_MT_SLOT: u16 = 0x2f;
const ABS_MT_POSITION_X: u16 = 0x35;
const ABS_MT_POSITION_Y: u16 = 0x36;
const ABS_MT_TRACKING_ID: u16 = 0x39;
const EVENT_SIZE: usize = 24; // struct input_event on 64-bit: timeval (16) + type + code + value

fn ioc(dir: u64, nr: u64, size: u64) -> u64 { (dir << 30) | (size << 16) | ((b'E' as u64) << 8) | nr }
const R: u64 = 2;
const W: u64 = 1;

/// struct input_absinfo
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct AbsInfo { pub value: i32, pub min: i32, pub max: i32, pub fuzz: i32, pub flat: i32, pub res: i32 }

fn ioctl_buf(fd: i32, req: u64, buf: *mut u8) -> i32 { unsafe { libc::ioctl(fd, req as _, buf) } }
fn ioctl_int(fd: i32, req: u64, v: libc::c_int) -> i32 { unsafe { libc::ioctl(fd, req as _, v) } }

fn get_bits(fd: i32, nr: u64, len: usize) -> Vec<u8> {
    let mut b = vec![0u8; len];
    if ioctl_buf(fd, ioc(R, nr, len as u64), b.as_mut_ptr()) < 0 { b.iter_mut().for_each(|x| *x = 0); }
    b
}
fn bit(b: &[u8], n: usize) -> bool { b.get(n / 8).map(|x| x >> (n % 8) & 1 == 1).unwrap_or(false) }
fn get_string(fd: i32, nr: u64) -> String {
    let mut b = vec![0u8; 256];
    let n = ioctl_buf(fd, ioc(R, nr, 256), b.as_mut_ptr());
    if n <= 0 { return String::new(); }
    let end = b.iter().position(|c| *c == 0).unwrap_or(n as usize);
    String::from_utf8_lossy(&b[..end]).trim().to_string()
}
pub fn get_abs(fd: i32, code: u16) -> Option<AbsInfo> {
    let mut a = AbsInfo::default();
    if ioctl_buf(fd, ioc(R, 0x40 + code as u64, 24), &mut a as *mut AbsInfo as *mut u8) < 0 { return None; }
    Some(a)
}
fn set_abs(fd: i32, code: u16, a: &AbsInfo) -> std::io::Result<()> {
    let mut a = *a;
    if ioctl_buf(fd, ioc(W, 0xc0 + code as u64, 24), &mut a as *mut AbsInfo as *mut u8) < 0 { return Err(std::io::Error::last_os_error()); }
    Ok(())
}
fn event_bytes(t: u16, code: u16, value: i32) -> [u8; EVENT_SIZE] {
    let mut b = [0u8; EVENT_SIZE];
    b[16..18].copy_from_slice(&t.to_ne_bytes());
    b[18..20].copy_from_slice(&code.to_ne_bytes());
    b[20..24].copy_from_slice(&value.to_ne_bytes());
    b
}

// ------------------------------------------------------------------ one event device

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kind { Pad, Motion, Touchpad }

struct Node {
    path: PathBuf,
    num: u32,
    file: File,
    name: String,
    phys: String,
    uniq: String,
    /// bustype, vendor, product, version
    id: [u16; 4],
    kind: Kind,
    keys: BTreeSet<u16>,
    abs: BTreeMap<u16, AbsInfo>,
    pressed: BTreeSet<u16>,
    rumble: bool,
    writable: bool,
    held: bool,
    group: String,
    sysdir: Option<PathBuf>,
    virt: bool,
    driver: String,
    slot: usize,
    touches: [Option<(i32, i32)>; 10],
    events: u64,
    gone: bool,
}

fn event_num(p: &Path) -> Option<u32> { p.file_name()?.to_str()?.strip_prefix("event")?.parse().ok() }

impl Node {
    fn open(path: &Path) -> Result<Option<Node>, std::io::Error> {
        let num = event_num(path).unwrap_or(9999);
        let (file, writable) = match OpenOptions::new().read(true).write(true).custom_flags(libc::O_NONBLOCK).open(path) {
            Ok(f) => (f, true),
            Err(_) => (OpenOptions::new().read(true).custom_flags(libc::O_NONBLOCK).open(path)?, false),
        };
        let fd = file.as_raw_fd();
        let types = get_bits(fd, 0x20, 4);
        let keyb = if bit(&types, EV_KEY as usize) { get_bits(fd, 0x20 + EV_KEY as u64, 96) } else { vec![] };
        let absb = if bit(&types, EV_ABS as usize) { get_bits(fd, 0x20 + EV_ABS as u64, 8) } else { vec![] };
        let ffb = if bit(&types, EV_FF as usize) { get_bits(fd, 0x20 + EV_FF as u64, 16) } else { vec![] };
        let props = get_bits(fd, 0x09, 4);
        let keys: BTreeSet<u16> = (0..768u16).filter(|k| bit(&keyb, *k as usize)).collect();
        let mut abs = BTreeMap::new();
        for c in 0..64u16 { if bit(&absb, c as usize) { if let Some(a) = get_abs(fd, c) { abs.insert(c, a); } } }
        let gamepad_keys = keys.iter().any(|k| (0x130..=0x13e).contains(k));
        let joystick_keys = keys.iter().any(|k| (0x120..=0x12f).contains(k));
        let kind = if bit(&props, PROP_ACCELEROMETER) && abs.len() >= 3 { Kind::Motion }
            else if abs.contains_key(&ABS_MT_POSITION_X) && (bit(&props, PROP_BUTTONPAD) || bit(&props, PROP_POINTER)) { Kind::Touchpad }
            else if (gamepad_keys && (abs.contains_key(&0) || abs.contains_key(&0x10) || keys.contains(&0x220))) || (joystick_keys && abs.contains_key(&0)) { Kind::Pad }
            else { return Ok(None) };
        let mut id = [0u16; 4];
        ioctl_buf(fd, ioc(R, 0x02, 8), id.as_mut_ptr() as *mut u8);
        let name = get_string(fd, 0x06);
        let phys = get_string(fd, 0x07);
        let uniq = get_string(fd, 0x08);
        let input_dir = std::fs::canonicalize(format!("/sys/class/input/{}/device", path.file_name().unwrap_or_default().to_string_lossy())).ok();
        let virt = input_dir.as_ref().map(|d| d.starts_with("/sys/devices/virtual")).unwrap_or(false);
        let parent = input_dir.as_ref().and_then(|d| std::fs::canonicalize(d.join("device")).ok());
        let group = match (&parent, virt) {
            (Some(p), false) => p.to_string_lossy().to_string(),
            _ if !uniq.is_empty() => format!("virtual:uniq:{uniq}"),
            _ if !phys.is_empty() => format!("virtual:phys:{}", phys.split('/').next().unwrap_or(&phys)),
            _ => format!("virtual:{}:{}", name, num),
        };
        let driver = parent.as_ref().and_then(|p| std::fs::canonicalize(p.join("driver")).ok()).and_then(|d| d.file_name().map(|n| n.to_string_lossy().to_string())).unwrap_or_default();
        // Someone else holding it (Handheld Daemon hides the built-in pad this way): a grab of ours fails with EBUSY.
        let held = if ioctl_int(fd, ioc(W, 0x90, 4), 1) < 0 { std::io::Error::last_os_error().raw_os_error() == Some(libc::EBUSY) } else { ioctl_int(fd, ioc(W, 0x90, 4), 0); false };
        let pressed = { let kb = get_bits(fd, 0x18, 96); keys.iter().copied().filter(|k| bit(&kb, *k as usize)).collect() };
        Ok(Some(Node { path: path.to_path_buf(), num, file, name, phys, uniq, id, kind, keys, abs, pressed, rumble: bit(&ffb, FF_RUMBLE as usize), writable, held,
                       group, sysdir: parent, virt, driver, slot: 0, touches: [None; 10], events: 0, gone: false }))
    }

    /// Reads what's waiting (never blocks).
    fn pump(&mut self) {
        let mut buf = [0u8; EVENT_SIZE * 64];
        loop {
            match self.file.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    for ev in buf[..n].chunks_exact(EVENT_SIZE) {
                        let t = u16::from_ne_bytes([ev[16], ev[17]]);
                        let code = u16::from_ne_bytes([ev[18], ev[19]]);
                        let value = i32::from_ne_bytes([ev[20], ev[21], ev[22], ev[23]]);
                        self.apply(t, code, value);
                    }
                    if n < buf.len() { break; }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => { self.gone = true; break; }
            }
        }
    }

    fn apply(&mut self, t: u16, code: u16, value: i32) {
        self.events += 1;
        match t {
            EV_KEY => { if value != 0 { self.pressed.insert(code); } else { self.pressed.remove(&code); } }
            EV_ABS => {
                if self.kind == Kind::Touchpad {
                    match code {
                        ABS_MT_SLOT => self.slot = (value.max(0) as usize).min(9),
                        ABS_MT_TRACKING_ID => { if value < 0 { self.touches[self.slot] = None; } else if self.touches[self.slot].is_none() { self.touches[self.slot] = Some((-1, -1)); } }
                        ABS_MT_POSITION_X => { if let Some(t) = self.touches[self.slot].as_mut() { t.0 = value; } }
                        ABS_MT_POSITION_Y => { if let Some(t) = self.touches[self.slot].as_mut() { t.1 = value; } }
                        _ => {}
                    }
                }
                if let Some(a) = self.abs.get_mut(&code) { a.value = value; }
            }
            EV_SYN if code == SYN_DROPPED => self.resync(),
            _ => {}
        }
    }

    /// After the kernel dropped events: ask for the current state again.
    fn resync(&mut self) {
        let fd = self.file.as_raw_fd();
        let codes: Vec<u16> = self.abs.keys().copied().collect();
        for c in codes { if let Some(a) = get_abs(fd, c) { self.abs.insert(c, a); } }
        let kb = get_bits(fd, 0x18, 96);
        self.pressed = self.keys.iter().copied().filter(|k| bit(&kb, *k as usize)).collect();
    }
}

// ------------------------------------------------------------------ what the page gets

fn norm_axis(a: &AbsInfo) -> f64 {
    let half = (a.max as f64 - a.min as f64) / 2.0;
    if half <= 0.0 { return 0.0; }
    ((a.value as f64 - (a.max as f64 + a.min as f64) / 2.0) / half).clamp(-1.25, 1.25)
}
fn axis_json(a: &AbsInfo) -> Value { json!({"raw": a.value, "min": a.min, "max": a.max, "flat": a.flat, "fuzz": a.fuzz, "res": a.res}) }

pub fn family(vendor: u16, driver: &str) -> &'static str {
    if vendor == 0x054c || driver == "playstation" || driver == "sony" { "playstation" }
    else if vendor == 0x057e || driver == "nintendo" { "nintendo" }
    else { "xbox" }
}

/// The driver, in words.
pub fn driver_label(driver: &str, virt: bool, name: &str, phys: &str, uniq: &str, vendor: u16) -> String {
    let l = format!("{name} {phys} {uniq}").to_lowercase();
    if virt {
        if l.contains("hhd") || l.contains("handheld daemon") { return "Handheld Daemon (virtual pad)".into(); }
        if l.contains("inputplumber") { return "InputPlumber (virtual pad)".into(); }
        if l.contains("steam") || vendor == 0x28de { return "Steam Input (virtual pad)".into(); }
        if l.contains("hearth") { return "Hearth (virtual pad)".into(); }
        return "Virtual pad (uinput)".into();
    }
    match driver {
        "xpad" => "xpad (Xbox)".into(),
        "playstation" => "hid-playstation".into(),
        "sony" => "hid-sony".into(),
        "nintendo" => "hid-nintendo".into(),
        "microsoft" => "hid-microsoft".into(),
        "xpadneo" => "xpadneo".into(),
        "steam" => "hid-steam".into(),
        "hid-generic" => "hid-generic (no controller driver)".into(),
        "" => "unknown driver".into(),
        d => d.into(),
    }
}

fn battery(sysdir: &Option<PathBuf>) -> Value {
    let mut cur = sysdir.clone();
    for _ in 0..4 {
        let Some(d) = cur else { break };
        if let Ok(rd) = std::fs::read_dir(d.join("power_supply")) {
            for e in rd.flatten() {
                let p = e.path();
                let rd = |n: &str| std::fs::read_to_string(p.join(n)).ok().map(|s| s.trim().to_string());
                let pct = rd("capacity").and_then(|s| s.parse::<u32>().ok());
                let level = rd("capacity_level");
                if pct.is_some() || level.is_some() { return json!({"pct": pct, "level": level, "status": rd("status")}); }
            }
        }
        cur = d.parent().map(Path::to_path_buf);
    }
    Value::Null
}

/// Names for keys that aren't one of the named buttons.
fn key_name(k: u16) -> String {
    match k {
        0x110 => "Touchpad click".into(), 0x14a => "Touch".into(), 0xa7 => "Share / Record".into(), 0x248 => "Mic mute".into(),
        0x132 => "C".into(), 0x135 => "Z".into(), 0x120..=0x12f => format!("Button {}", k - 0x120 + 1),
        0x2c0..=0x2e7 => format!("Extra {}", k - 0x2c0 + 1), _ => format!("Key 0x{k:x}"),
    }
}

/// One controller (its event devices together) for the page.
fn pad_json(player: usize, nodes: &[&Node]) -> Value {
    let pad = nodes.iter().find(|n| n.kind == Kind::Pad).copied().unwrap_or(nodes[0]);
    let fam = family(pad.id[1], &pad.driver);
    let all_pressed: BTreeSet<u16> = nodes.iter().filter(|n| n.kind != Kind::Motion).flat_map(|n| n.pressed.iter().copied()).collect();
    let p = |k: u16| all_pressed.contains(&k);
    // xpad reports the left face button (X) as 0x133 and the top one (Y) as 0x134; the drivers that follow the
    // kernel's gamepad spec (hid-playstation, hid-nintendo) put the top one first.
    let (north, west) = if fam == "xbox" { (0x134, 0x133) } else { (0x133, 0x134) };
    let abs = &pad.abs;
    let hat = |c: u16| abs.get(&c).map(|a| a.value.signum()).unwrap_or(0);
    let hat_dpad = pad.driver != "steam";
    let (hx, hy) = if hat_dpad { (hat(0x10), hat(0x11)) } else { (0, 0) };
    let mut named: BTreeSet<u16> = [0x130, 0x131, 0x133, 0x134, 0x136, 0x137, 0x138, 0x139, 0x13a, 0x13b, 0x13c, 0x13d, 0x13e, 0x220, 0x221, 0x222, 0x223,
        0xa7, 0x110, 0x14a, 0x2c0, 0x2c1, 0x2c2, 0x2c3, 0x224, 0x225, 0x226, 0x227].into_iter().collect();
    named.insert(0x248);
    let buttons = json!({
        "south": p(0x130), "east": p(0x131), "north": p(north), "west": p(west),
        "lb": p(0x136), "rb": p(0x137), "lt": p(0x138), "rt": p(0x139), "back": p(0x13a), "start": p(0x13b), "guide": p(0x13c),
        "ls": p(0x13d), "rs": p(0x13e),
        "up": p(0x220) || hy < 0, "down": p(0x221) || hy > 0, "left": p(0x222) || hx < 0, "right": p(0x223) || hx > 0,
        "share": p(0xa7) || p(0x248), "touch": p(0x110),
        "p1": p(0x2c0) || p(0x224), "p2": p(0x2c1) || p(0x225), "p3": p(0x2c2) || p(0x226), "p4": p(0x2c3) || p(0x227),
    });
    let has = |k: u16| nodes.iter().any(|n| n.keys.contains(&k));
    let present = json!({"paddles": has(0x2c0) || has(0x224), "share": has(0xa7) || has(0x248), "guide": has(0x13c), "digitalTriggers": has(0x138) && !abs.contains_key(&2) && !abs.contains_key(&10) && !abs.contains_key(&0x15),
        "dpad": has(0x220) || (hat_dpad && abs.contains_key(&0x10)), "ls": has(0x13d), "rs": has(0x13e)});
    let other: Vec<String> = all_pressed.iter().filter(|k| !named.contains(k)).map(|k| key_name(*k)).collect();
    let stick = |x: u16, y: u16| -> Value {
        match (abs.get(&x), abs.get(&y)) {
            (Some(ax), Some(ay)) => json!({"x": norm_axis(ax), "y": norm_axis(ay), "codeX": x, "codeY": y, "ax": axis_json(ax), "ay": axis_json(ay)}),
            _ => Value::Null,
        }
    };
    let right = if abs.contains_key(&3) { stick(3, 4) } else { stick(2, 5) };
    let trig = |codes: &[u16], digital: u16| -> Value {
        for c in codes {
            if let Some(a) = abs.get(c) {
                let span = (a.max - a.min).max(1) as f64;
                return json!({"v": ((a.value - a.min) as f64 / span).clamp(0.0, 1.0), "code": c, "a": axis_json(a), "digital": p(digital)});
            }
        }
        if has(digital) { json!({"v": if p(digital) { 1.0 } else { 0.0 }, "digital": p(digital), "onlyDigital": true}) } else { Value::Null }
    };
    let (lt, rt) = if abs.contains_key(&3) { (trig(&[2u16, 10, 0x15][..], 0x138), trig(&[5u16, 9, 0x14][..], 0x139)) } else { (trig(&[10u16, 0x15][..], 0x138), trig(&[9u16, 0x14][..], 0x139)) };
    let motion = nodes.iter().find(|n| n.kind == Kind::Motion).map(|m| {
        let v = |c: u16| m.abs.get(&c).map(|a| if a.res > 0 { a.value as f64 / a.res as f64 } else { a.value as f64 / 1000.0 }).unwrap_or(0.0);
        json!({"accel": [v(0), v(1), v(2)], "gyro": [v(3), v(4), v(5)], "accelRes": m.abs.get(&0).map(|a| a.res), "gyroRes": m.abs.get(&3).map(|a| a.res), "name": m.name})
    });
    let touch = nodes.iter().find(|n| n.kind == Kind::Touchpad).map(|t| {
        let ax = t.abs.get(&ABS_MT_POSITION_X).copied().unwrap_or_default();
        let ay = t.abs.get(&ABS_MT_POSITION_Y).copied().unwrap_or_default();
        let sx = (ax.max - ax.min).max(1) as f64;
        let sy = (ay.max - ay.min).max(1) as f64;
        let pts: Vec<Value> = t.touches.iter().flatten().filter(|(x, y)| *x >= 0 && *y >= 0).map(|(x, y)| json!({"x": (*x - ax.min) as f64 / sx, "y": (*y - ay.min) as f64 / sy})).collect();
        json!({"points": pts, "click": t.pressed.contains(&0x110), "w": sx, "h": sy, "name": t.name})
    });
    let axes: Vec<Value> = abs.iter().filter(|(c, _)| **c < 0x28).map(|(c, a)| json!({"code": c, "a": axis_json(a)})).collect();
    json!({
        "id": pad.group, "player": player, "name": pad.name, "family": fam, "vendor": format!("{:04x}", pad.id[1]), "product": format!("{:04x}", pad.id[2]),
        "bus": match pad.id[0] { 0x03 => "USB", 0x05 => "Bluetooth", 0x06 => "Virtual", 0x19 => "Built in", _ => "Other" },
        "driver": pad.driver, "driverLabel": driver_label(&pad.driver, pad.virt, &pad.name, &pad.phys, &pad.uniq, pad.id[1]), "virtual": pad.virt,
        "held": pad.held, "writable": pad.writable, "rumble": nodes.iter().any(|n| n.rumble), "battery": battery(&pad.sysdir),
        "nodes": nodes.iter().map(|n| json!({"path": n.path, "kind": format!("{:?}", n.kind), "name": n.name, "events": n.events})).collect::<Vec<_>>(),
        "buttons": buttons, "present": present, "other": other, "sticks": {"l": stick(0, 1), "r": right}, "triggers": {"l": lt, "r": rt},
        "motion": motion, "touch": touch, "axes": axes,
        "events": nodes.iter().map(|n| n.events).sum::<u64>(),
    })
}

// ------------------------------------------------------------------ the reader

#[derive(Default)]
struct Inner {
    nodes: Vec<Node>,
    /// Joysticks this account can't open (permissions), by name.
    denied: BTreeMap<String, String>,
    /// The driver's own ranges before a calibration, to put back.
    original: BTreeMap<(String, u16), AbsInfo>,
    scans: u64,
}

#[derive(Clone)]
pub struct Lab { inner: Arc<Mutex<Inner>> }

/// The udev database says whether an event device is a joystick (without opening it).
fn udev_joystick(num: u32) -> bool {
    std::fs::read_to_string(format!("/run/udev/data/c13:{}", 64 + num)).map(|s| s.lines().any(|l| l == "E:ID_INPUT_JOYSTICK=1")).unwrap_or(false)
}

impl Lab {
    /// Starts reading controllers in the background.
    pub fn start() -> Lab {
        let lab = Lab { inner: Arc::new(Mutex::new(Inner::default())) };
        let l = lab.clone();
        std::thread::spawn(move || l.run());
        lab
    }
    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> { self.inner.lock().unwrap_or_else(|e| e.into_inner()) }

    fn run(&self) {
        let mut last_scan = std::time::Instant::now() - std::time::Duration::from_secs(10);
        loop {
            if last_scan.elapsed() > std::time::Duration::from_millis(1000) { self.rescan(); last_scan = std::time::Instant::now(); }
            let fds: Vec<libc::pollfd> = self.lock().nodes.iter().map(|n| libc::pollfd { fd: n.file.as_raw_fd(), events: libc::POLLIN, revents: 0 }).collect();
            if fds.is_empty() { std::thread::sleep(std::time::Duration::from_millis(200)); continue; }
            let mut fds = fds;
            unsafe { libc::poll(fds.as_mut_ptr(), fds.len() as libc::nfds_t, 50) };
            let mut g = self.lock();
            for n in g.nodes.iter_mut() { n.pump(); }
            g.nodes.retain(|n| !n.gone);
        }
    }

    /// Looks for controllers plugged in or removed.
    pub fn rescan(&self) {
        let mut paths: Vec<PathBuf> = std::fs::read_dir("/dev/input").map(|rd| rd.flatten().map(|e| e.path()).filter(|p| event_num(p).is_some()).collect()).unwrap_or_default();
        paths.sort_by_key(|p| event_num(p));
        let known: BTreeSet<PathBuf> = self.lock().nodes.iter().map(|n| n.path.clone()).collect();
        let mut fresh = Vec::new();
        let mut denied = BTreeMap::new();
        for p in paths.iter().filter(|p| !known.contains(*p)) {
            match Node::open(p) {
                Ok(Some(n)) => fresh.push(n),
                Ok(None) => {}
                Err(e) => {
                    let num = event_num(p).unwrap_or(0);
                    if e.kind() == std::io::ErrorKind::PermissionDenied && udev_joystick(num) {
                        let name = std::fs::read_to_string(format!("/sys/class/input/event{num}/device/name")).map(|s| s.trim().to_string()).unwrap_or_else(|_| format!("event{num}"));
                        denied.insert(p.to_string_lossy().to_string(), name);
                    }
                }
            }
        }
        let mut g = self.lock();
        g.nodes.extend(fresh);
        g.nodes.retain(|n| n.path.exists());
        g.nodes.sort_by_key(|n| n.num);
        g.denied = denied;
        g.scans += 1;
    }

    /// Every controller now, numbered by when it was connected (lowest event device first).
    pub fn snapshot(&self) -> Value {
        let g = self.lock();
        let mut groups: Vec<(u32, String)> = Vec::new();
        for n in g.nodes.iter().filter(|n| n.kind == Kind::Pad) { if !groups.iter().any(|x| x.1 == n.group) { groups.push((n.num, n.group.clone())); } }
        groups.sort();
        let pads: Vec<Value> = groups.iter().enumerate().map(|(i, (_, grp))| {
            let nodes: Vec<&Node> = g.nodes.iter().filter(|n| &n.group == grp).collect();
            let mut v = pad_json(i + 1, &nodes);
            v["calibrated"] = json!(g.original.keys().any(|(k, _)| k == grp));
            v
        }).collect();
        json!({"pads": pads, "denied": g.denied.values().collect::<Vec<_>>(), "scans": g.scans})
    }

    /// Rumbles a controller for `ms` (strong: the big motor, weak: the small one, 0 to 1). Runs on its own thread:
    /// a virtual pad whose maker doesn't answer can keep the kernel waiting.
    pub fn rumble(&self, id: &str, strong: f64, weak: f64, ms: u32) -> Result<(), String> {
        let path = self.lock().nodes.iter().find(|n| n.group == id && n.rumble).map(|n| n.path.clone()).ok_or("This controller doesn't report rumble (or its driver doesn't support it).")?;
        let ms = ms.clamp(50, 5000);
        std::thread::spawn(move || { let _ = rumble_now(&path, strong, weak, ms); });
        Ok(())
    }

    /// Sets the driver's range and dead zone for a stick or trigger axis (what games read when they open the
    /// controller). The driver's own values are kept to put back with `reset`.
    pub fn set_axis(&self, id: &str, code: u16, min: i32, max: i32, flat: i32) -> Result<AbsInfo, String> {
        if max <= min { return Err("That range is empty.".into()); }
        let mut g = self.lock();
        let Some(n) = g.nodes.iter().find(|n| n.group == id && n.kind == Kind::Pad && n.abs.contains_key(&code)) else { return Err("That controller isn't connected anymore.".into()) };
        let fd = n.file.as_raw_fd();
        let cur = get_abs(fd, code).ok_or("Couldn't read that axis.")?;
        let new = AbsInfo { value: cur.value, min, max, fuzz: cur.fuzz, flat: flat.clamp(0, (max - min) / 2), res: cur.res };
        set_abs(fd, code, &new).map_err(|e| format!("The driver didn't accept it ({e})."))?;
        let key = (id.to_string(), code);
        g.original.entry(key).or_insert(cur);
        if let Some(n) = g.nodes.iter_mut().find(|n| n.group == id && n.kind == Kind::Pad) { if let Some(a) = n.abs.get_mut(&code) { *a = new; } }
        Ok(new)
    }

    /// Puts the driver's own ranges and dead zones back.
    pub fn reset(&self, id: &str) -> Result<usize, String> {
        let mut g = self.lock();
        let keys: Vec<(String, u16)> = g.original.keys().filter(|(k, _)| k == id).cloned().collect();
        let mut done = 0;
        for k in keys {
            let orig = g.original[&k];
            if let Some(n) = g.nodes.iter_mut().find(|n| n.group == id && n.kind == Kind::Pad && n.abs.contains_key(&k.1)) {
                let fd = n.file.as_raw_fd();
                let mut o = orig;
                o.value = n.abs[&k.1].value;
                if set_abs(fd, k.1, &o).is_ok() { n.abs.insert(k.1, o); done += 1; }
            }
            g.original.remove(&k);
        }
        Ok(done)
    }
}

/// One rumble: upload an FF_RUMBLE effect, play it, wait, take it away.
pub fn rumble_now(path: &Path, strong: f64, weak: f64, ms: u32) -> Result<(), String> {
    let mut f = OpenOptions::new().read(true).write(true).open(path).map_err(|e| format!("Can't send rumble to it ({e})."))?;
    let fd = f.as_raw_fd();
    let _ = f.write_all(&event_bytes(EV_FF, FF_GAIN, 0xffff));
    // struct ff_effect (48 bytes on 64-bit): type, id, direction, trigger, replay {length, delay}, then the union.
    let mut e = [0u64; 6];
    {
        let b: &mut [u8; 48] = unsafe { &mut *(e.as_mut_ptr() as *mut [u8; 48]) };
        b[0..2].copy_from_slice(&FF_RUMBLE.to_ne_bytes());
        b[2..4].copy_from_slice(&(-1i16).to_ne_bytes());
        b[10..12].copy_from_slice(&(ms.min(65535) as u16).to_ne_bytes());
        b[16..18].copy_from_slice(&((strong.clamp(0.0, 1.0) * 65535.0) as u16).to_ne_bytes());
        b[18..20].copy_from_slice(&((weak.clamp(0.0, 1.0) * 65535.0) as u16).to_ne_bytes());
    }
    if ioctl_buf(fd, ioc(W, 0x80, 48), e.as_mut_ptr() as *mut u8) < 0 { return Err(format!("The driver refused the rumble ({}).", std::io::Error::last_os_error())); }
    let id = { let b: &[u8; 48] = unsafe { &*(e.as_ptr() as *const [u8; 48]) }; i16::from_ne_bytes([b[2], b[3]]) };
    let _ = f.write_all(&event_bytes(EV_FF, id as u16, 1));
    std::thread::sleep(std::time::Duration::from_millis(ms as u64 + 30));
    let _ = f.write_all(&event_bytes(EV_FF, id as u16, 0));
    ioctl_int(fd, ioc(W, 0x81, 4), id as libc::c_int);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ioctl_numbers_match_the_kernel() {
        // From <linux/input.h> on x86_64.
        assert_eq!(ioc(R, 0x02, 8), 0x8008_4502); // EVIOCGID
        assert_eq!(ioc(R, 0x06, 256), 0x8100_4506); // EVIOCGNAME(256)
        assert_eq!(ioc(R, 0x40, 24), 0x8018_4540); // EVIOCGABS(0)
        assert_eq!(ioc(W, 0xc0, 24), 0x4018_45c0); // EVIOCSABS(0)
        assert_eq!(ioc(W, 0x80, 48), 0x4030_4580); // EVIOCSFF
        assert_eq!(ioc(W, 0x90, 4), 0x4004_4590); // EVIOCGRAB
        assert_eq!(std::mem::size_of::<AbsInfo>(), 24);
    }
    #[test]
    fn axes_and_names() {
        let a = AbsInfo { value: 0, min: -32768, max: 32767, ..Default::default() };
        assert!(norm_axis(&a).abs() < 0.001);
        let t = AbsInfo { value: 255, min: 0, max: 255, ..Default::default() };
        assert!((norm_axis(&t) - 1.0).abs() < 0.001);
        assert_eq!(family(0x054c, ""), "playstation");
        assert_eq!(family(0, "nintendo"), "nintendo");
        assert_eq!(family(0x045e, "xpad"), "xbox");
        assert_eq!(driver_label("", true, "Handheld Daemon Controller", "", "", 0x045e), "Handheld Daemon (virtual pad)");
        assert_eq!(driver_label("", true, "x", "phys-hhd-main", "", 0), "Handheld Daemon (virtual pad)");
        assert_eq!(driver_label("playstation", false, "DualSense", "", "", 0x054c), "hid-playstation");
    }
}
