//! The controller on this PC (the laptop's paired one, or the Ally's own buttons; the Xbox's controller plays the
//! Xbox), read straight from /dev/input: Console Remote Play's pad.rs (from the Store's and Settings'), with B's
//! release added for "hold B to leave". Light at rest: one thread that sleeps in the kernel (poll with no timeout) until a controller sends something
//! or /dev/input changes. It only reads (never grabs), so Steam and hearth-pad still see every press.

use futures::channel::mpsc::{unbounded, UnboundedSender};
use futures::Stream;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    A,
    B,
    X,
    Y,
    Lb,
    Rb,
    Up,
    Down,
    Left,
    Right,
    Menu,
    View,
    /// B let go (hold B to leave).
    BUp,
}

/// The controller's presses, for as long as the app runs (Subscription::run).
pub fn presses() -> impl Stream<Item = Button> {
    let (tx, rx) = unbounded();
    let _ = std::thread::Builder::new().name("crp-pad".into()).stack_size(128 * 1024).spawn(move || reader(tx));
    rx
}

const EV_KEY: u16 = 0x01;
const EV_REL: u16 = 0x02;
const EV_ABS: u16 = 0x03;
const KEY_A: u16 = 30;
const KEY_SPACE: u16 = 57;
const BTN_LEFT: u16 = 0x110;
const BTN_TOOL_FINGER: u16 = 0x145;
const BUS_VIRTUAL: u16 = 0x06;

// What the person is using, for the automatic look (main.rs, nav.rs). The owner on the Ally (Nov 19): "the settings
// keep switching between controller and mouse and keyboard mode": hearth-pad's pointer (the stick) and its A click
// reach Settings as ordinary pointer events, so they looked like a mouse. Now:
//  - CTRL_AT: the controller was used at all (a button, or a stick or trigger moved: the pointer it moves is it).
//  - DESK_AT: a real keyboard, mouse or touchpad was used (read from /dev/input where this user may read them).
//  - DESK_READABLE: at least one real keyboard / mouse / touchpad could be opened; then only DESK_AT switches the
//    look back; otherwise (they're usually root-only) a pointer event switches it only with the controller idle.
static CTRL_AT: AtomicU64 = AtomicU64::new(0);
static DESK_AT: AtomicU64 = AtomicU64::new(0);
static DESK_READABLE: AtomicBool = AtomicBool::new(false);
/// The controller look shows (main.rs sets it): real keyboards and mice are watched only then.
pub static PAD_LOOK: AtomicBool = AtomicBool::new(false);

fn now_ms() -> u64 {
    static T: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    T.get_or_init(Instant::now).elapsed().as_millis() as u64 + 1
}
fn recent(at: &AtomicU64, ms: u64) -> bool {
    let t = at.load(Ordering::Relaxed);
    t > 0 && now_ms().saturating_sub(t) < ms
}

/// May a pointer event (moved, clicked, scrolled) or a typed key switch to the keyboard-and-mouse look now?
pub fn desk_in_use() -> bool {
    if DESK_READABLE.load(Ordering::Relaxed) {
        return recent(&DESK_AT, 1500);
    }
    !recent(&CTRL_AT, 3000)
}

/// The controller was used (also from main.rs for a press it was told of).
pub fn ctrl_used() {
    CTRL_AT.store(now_ms(), Ordering::Relaxed);
}
const BTN_SOUTH: u16 = 0x130;
const SONY: u16 = 0x054c;

const fn ior(nr: u32, size: u32) -> u64 {
    ((2u32 << 30) | (size << 16) | (0x45 << 8) | nr) as u64
}

/// What a button code is (hearth-pad's button_map).
pub fn button(code: u16, sony: bool) -> Option<Button> {
    Some(match code {
        0x130 => Button::A,
        0x131 => Button::B,
        0x133 => if sony { Button::Y } else { Button::X },
        0x134 => if sony { Button::X } else { Button::Y },
        0x136 => Button::Lb,
        0x137 => Button::Rb,
        0x13a => Button::View,
        0x13b => Button::Menu,
        0x220 | 0x2c2 => Button::Up,
        0x221 | 0x2c3 => Button::Down,
        0x222 | 0x2c0 => Button::Left,
        0x223 | 0x2c1 => Button::Right,
        _ => return None,
    })
}

/// A button by its name (`--press`, hearth-pad's names).
pub fn named(s: &str) -> Option<Button> {
    Some(match s.to_lowercase().as_str() {
        "a" => Button::A,
        "b" => Button::B,
        "x" => Button::X,
        "y" => Button::Y,
        "lb" => Button::Lb,
        "rb" => Button::Rb,
        "up" => Button::Up,
        "down" => Button::Down,
        "left" => Button::Left,
        "right" => Button::Right,
        "menu" => Button::Menu,
        "view" => Button::View,
        _ => return None,
    })
}

/// The left stick past 60 % (and back under 30 % before it counts again), as Firehold's shell reads it.
pub fn stick(was: i8, v: f32) -> (i8, bool) {
    let now: i8 = if v > 0.6 { 1 } else if v < -0.6 { -1 } else { 0 };
    let next = if v.abs() < 0.3 { 0 } else if now != 0 { now } else { was };
    (next, now != 0 && now != was)
}

struct Pad {
    fd: i32,
    path: String,
    sony: bool,
    /// ABS_X, ABS_Y: (min, max) and where it points now (-1, 0, 1).
    range: [(i32, i32); 2],
    /// ABS_X … ABS_RZ: (min, max, where it was when opened: its rest), to tell a stick or trigger in use.
    rest: [(i32, i32, i32); 6],
    stick: [i8; 2],
    hat: [i32; 2],
    /// A real keyboard, mouse or touchpad (not a controller): read only to know the person went back to them.
    desk: bool,
    /// (desk devices) being watched now.
    armed: bool,
}

fn drain(fd: i32) {
    let mut buf = [0u8; 24 * 64];
    // SAFETY: reading into a local buffer from a non-blocking descriptor this thread owns.
    while unsafe { libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) } > 0 {}
}

fn bit(bits: &[u8], n: usize) -> bool {
    bits.get(n / 8).map(|b| b & (1 << (n % 8)) != 0).unwrap_or(false)
}

fn shut(fd: i32) -> Option<Pad> {
    // SAFETY: closing a descriptor the caller opened and gives up.
    unsafe { libc::close(fd) };
    None
}

fn open(path: &str) -> Option<Pad> {
    let c = std::ffi::CString::new(path).ok()?;
    // SAFETY: plain syscalls on a file descriptor this function owns; every buffer is as large as the ioctl says.
    unsafe {
        let fd = libc::open(c.as_ptr(), libc::O_RDONLY | libc::O_NONBLOCK | libc::O_CLOEXEC);
        if fd < 0 {
            return None;
        }
        let mut name = [0u8; 128];
        libc::ioctl(fd, ior(0x06, 128) as _, name.as_mut_ptr());
        let name = String::from_utf8_lossy(&name[..name.iter().position(|b| *b == 0).unwrap_or(0)]).to_string();
        // Hearth's own virtual devices (hearth-pad's keyboard, the login controller) aren't controllers to read.
        if name.starts_with("Hearth ") {
            return shut(fd);
        }
        let mut evs = [0u8; 4];
        let mut keys = [0u8; 96];
        libc::ioctl(fd, ior(0x20, 4) as _, evs.as_mut_ptr());
        libc::ioctl(fd, ior(0x20 + EV_KEY as u32, 96) as _, keys.as_mut_ptr());
        let mut id = [0u16; 4];
        libc::ioctl(fd, ior(0x02, 8) as _, id.as_mut_ptr());
        if !bit(&evs, EV_ABS as usize) || !bit(&keys, BTN_SOUTH as usize) {
            // Not a controller: a real keyboard, mouse or touchpad? (Virtual ones, Steam's and Handheld Daemon's
            // copies, and touch screens don't count: they're the controller, or neither.)
            let mut rels = [0u8; 2];
            libc::ioctl(fd, ior(0x20 + EV_REL as u32, 2) as _, rels.as_mut_ptr());
            let lname = name.to_lowercase();
            let virt = id[0] == BUS_VIRTUAL || ["steam", "handheld daemon", "hhd", "virtual", "hearth"].iter().any(|w| lname.contains(w));
            let keyboard = bit(&evs, EV_KEY as usize) && bit(&keys, KEY_A as usize) && bit(&keys, KEY_SPACE as usize);
            let mouse = bit(&evs, EV_REL as usize) && bit(&rels, 0) && bit(&rels, 1) && bit(&keys, BTN_LEFT as usize);
            let touchpad = bit(&evs, EV_ABS as usize) && bit(&keys, BTN_TOOL_FINGER as usize) && bit(&keys, BTN_LEFT as usize);
            if virt || !(keyboard || mouse || touchpad) {
                return shut(fd);
            }
            DESK_READABLE.store(true, Ordering::Relaxed);
            return Some(Pad { fd, path: path.to_string(), sony: false, range: [(0, 1); 2], rest: [(0, 0, 0); 6], stick: [0, 0], hat: [0, 0], desk: true, armed: false });
        }
        let mut range = [(-32768, 32767); 2];
        let mut rest = [(0, 0, 0); 6];
        for (i, r) in rest.iter_mut().enumerate() {
            let mut abs = [0i32; 6];
            if libc::ioctl(fd, ior(0x40 + i as u32, 24) as _, abs.as_mut_ptr()) == 0 && abs[2] > abs[1] {
                *r = (abs[1], abs[2], abs[0]);
                if i < 2 {
                    range[i] = (abs[1], abs[2]);
                }
            }
        }
        Some(Pad { fd, path: path.to_string(), sony: id[1] == SONY, range, rest, stick: [0, 0], hat: [0, 0], desk: false, armed: true })
    }
}

fn scan(pads: &mut Vec<Pad>) {
    let Ok(dir) = std::fs::read_dir("/dev/input") else { return };
    for e in dir.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if !name.starts_with("event") {
            continue;
        }
        let path = format!("/dev/input/{name}");
        if pads.iter().any(|p| p.path == path) {
            continue;
        }
        if let Some(p) = open(&path) {
            pads.push(p);
        }
    }
}

/// A stick or trigger a quarter of its way from where it rested when the controller was opened: the person is using
/// the controller (and the pointer it moves). Axes without a known range don't count.
fn moved(p: &Pad, code: u16, value: i32) -> bool {
    let Some(&(lo, hi, rest)) = p.rest.get(code as usize) else { return false };
    if hi <= lo {
        return false;
    }
    ((value - rest) as f32).abs() > 0.25 * (hi - lo) as f32
}

/// Reads what came from one controller (or, for a desk device, notes it was used). False: it's gone.
fn read(p: &mut Pad, out: &mut Vec<Button>) -> bool {
    let mut buf = [0u8; 24 * 64];
    loop {
        // SAFETY: reading into a buffer of the given length from a descriptor this thread owns.
        let n = unsafe { libc::read(p.fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
        if n < 0 {
            let e = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
            return e == libc::EAGAIN || e == libc::EINTR;
        }
        if n == 0 {
            return false;
        }
        for ev in buf[..n as usize].chunks_exact(24) {
            let ty = u16::from_ne_bytes([ev[16], ev[17]]);
            let code = u16::from_ne_bytes([ev[18], ev[19]]);
            let value = i32::from_ne_bytes([ev[20], ev[21], ev[22], ev[23]]);
            if p.desk {
                // A real keyboard's keys (not its media / vendor keys), a mouse moving or clicking, a touchpad.
                if (ty == EV_KEY && value == 1 && (code < 128 || (0x110..0x118).contains(&code))) || ty == EV_REL || (ty == EV_ABS && code < 2) {
                    DESK_AT.store(now_ms(), Ordering::Relaxed);
                }
                continue;
            }
            if ty == EV_KEY || (ty == EV_ABS && code < 6 && moved(p, code, value)) || (ty == EV_ABS && (0x10..=0x11).contains(&code) && value != 0) {
                CTRL_AT.store(now_ms(), Ordering::Relaxed);
            }
            match ty {
                EV_KEY if value == 1 => out.extend(button(code, p.sony)),
                EV_KEY if value == 0 && code == 0x131 => out.push(Button::BUp),
                EV_ABS => match code {
                    0x10 | 0x11 => {
                        let i = (code - 0x10) as usize;
                        let v = value.signum();
                        if v != 0 && v != p.hat[i] {
                            out.push(match (i, v) {
                                (0, -1) => Button::Left,
                                (0, _) => Button::Right,
                                (_, -1) => Button::Up,
                                _ => Button::Down,
                            });
                        }
                        p.hat[i] = v;
                    }
                    0x00 | 0x01 => {
                        let i = code as usize;
                        let (lo, hi) = p.range[i];
                        let mid = (lo as f32 + hi as f32) / 2.0;
                        let half = ((hi - lo) as f32 / 2.0).max(1.0);
                        let (next, pressed) = stick(p.stick[i], (value as f32 - mid) / half);
                        if pressed {
                            out.push(match (i, next) {
                                (0, -1) => Button::Left,
                                (0, _) => Button::Right,
                                (_, -1) => Button::Up,
                                _ => Button::Down,
                            });
                        }
                        p.stick[i] = next;
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        if (n as usize) < buf.len() {
            return true;
        }
    }
}

fn reader(tx: UnboundedSender<Button>) {
    // SAFETY: inotify on a path literal; the descriptor is this thread's.
    let ino = unsafe {
        let fd = libc::inotify_init1(libc::IN_CLOEXEC | libc::IN_NONBLOCK);
        if fd >= 0 && libc::inotify_add_watch(fd, c"/dev/input".as_ptr(), libc::IN_CREATE | libc::IN_ATTRIB) < 0 {
            libc::close(fd);
            -1
        } else {
            fd
        }
    };
    let mut pads: Vec<Pad> = Vec::new();
    scan(&mut pads);
    // The same press seen twice (a controller and Steam's or Handheld Daemon's copy of it) counts once.
    let mut last: Option<(Button, Instant, i32)> = None;
    let mut recheck = false;
    loop {
        // Real keyboards, mice and touchpads are only watched while the controller look shows (that's when their use
        // matters): no wake-ups for every mouse move otherwise. What they sent while unwatched is dropped unread.
        let look = PAD_LOOK.load(Ordering::Relaxed);
        for p in pads.iter_mut().filter(|p| p.desk) {
            if look && !p.armed {
                drain(p.fd);
            }
            p.armed = look;
        }
        let mut fds: Vec<libc::pollfd> = pads.iter().map(|p| libc::pollfd { fd: p.fd, events: if p.desk && !p.armed { 0 } else { libc::POLLIN }, revents: 0 }).collect();
        if ino >= 0 {
            fds.push(libc::pollfd { fd: ino, events: libc::POLLIN, revents: 0 });
        }
        if fds.is_empty() {
            return; // nothing to read and no way to see one come: the keys still work
        }
        // Right after a press, one look again a second later (the look it switched to is known by then), so a
        // real mouse used next is watched; otherwise it sleeps until something comes.
        let timeout = if std::mem::take(&mut recheck) && !look && pads.iter().any(|p| p.desk) { 1000 } else { -1 };
        // SAFETY: fds is a live array of pollfd of the given length.
        let n = unsafe { libc::poll(fds.as_mut_ptr(), fds.len() as libc::nfds_t, timeout) };
        if n < 0 {
            if std::io::Error::last_os_error().raw_os_error() == Some(libc::EINTR) {
                continue;
            }
            return;
        }
        let mut gone = Vec::new();
        let mut got = Vec::new();
        for (i, f) in fds.iter().enumerate().take(pads.len()) {
            if f.revents & (libc::POLLERR | libc::POLLHUP | libc::POLLNVAL) != 0 {
                gone.push(i);
            } else if f.revents & libc::POLLIN != 0 {
                let mut b = Vec::new();
                if !read(&mut pads[i], &mut b) {
                    gone.push(i);
                }
                got.extend(b.into_iter().map(|x| (x, pads[i].fd)));
            }
        }
        for (b, fd) in got {
            let now = Instant::now();
            if let Some((lb, at, lfd)) = last {
                if lb == b && lfd != fd && now.duration_since(at).as_millis() < 80 {
                    continue;
                }
            }
            last = Some((b, now, fd));
            recheck = true;
            if tx.unbounded_send(b).is_err() {
                return;
            }
        }
        for i in gone.into_iter().rev() {
            let p = pads.remove(i);
            // SAFETY: closing a descriptor this thread opened.
            unsafe { libc::close(p.fd) };
        }
        if ino >= 0 && fds.last().map(|f| f.revents & libc::POLLIN != 0).unwrap_or(false) {
            let mut buf = [0u8; 4096];
            // SAFETY: draining the inotify descriptor into a local buffer.
            while unsafe { libc::read(ino, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) } > 0 {}
            scan(&mut pads);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buttons_as_hearth_pad_maps_them() {
        assert_eq!(button(0x130, false), Some(Button::A));
        assert_eq!(button(0x133, false), Some(Button::X));
        assert_eq!(button(0x133, true), Some(Button::Y));
        assert_eq!(button(0x134, false), Some(Button::Y));
        assert_eq!(button(0x136, false), Some(Button::Lb));
        assert_eq!(button(0x2c0, false), Some(Button::Left));
        assert_eq!(button(0x13c, false), None); // Guide stays the desktop's
        assert_eq!(named("RB"), Some(Button::Rb));
        assert_eq!(named("guide"), None);
        assert_eq!(ior(0x06, 128), 0x8080_4506);
        assert_eq!(ior(0x02, 8), 0x8008_4502);
    }

    #[test]
    fn the_stick_counts_once_per_push() {
        let (s, p) = stick(0, 0.7);
        assert!(p && s == 1);
        let (s, p) = stick(s, 0.9);
        assert!(!p && s == 1);
        let (s, p) = stick(s, 0.45);
        assert!(!p && s == 1);
        let (s, p) = stick(s, 0.1);
        assert!(!p && s == 0);
        let (s, p) = stick(s, -0.8);
        assert!(p && s == -1);
    }
}
