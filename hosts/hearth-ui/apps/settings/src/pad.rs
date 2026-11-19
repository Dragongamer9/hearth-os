//! The controller, read straight from /dev/input (as Firehold's shell and hearth-pad read it), so Settings knows a
//! controller is being used (the controller look, layout B) and its buttons work the same everywhere: Mantel's
//! hearth-pad types keys into Settings (A Enter, B Esc, the D-pad arrows, Y Ctrl+F, X Ctrl+D, LB / RB Ctrl+Page Up /
//! Down), but on KDE Plasma (the Fedora edition) hearth-pad is a mouse there and A is a click, so the buttons are
//! read here too; main.rs drops the keys and clicks a press also sent (nothing happens twice). On Plasma these buttons
//! are this app's alone: hearth-pad --plasma sends nothing for A, B, X, Y, LB, RB, the D-pad and ☰ while Settings
//! (app id hearth-settings) is in front (PLASMA_PAD_READERS), so LB / RB switch categories and never windows too.
//!
//! Light at rest: one thread that sleeps in the kernel (poll with no timeout) until a controller sends something or
//! /dev/input changes (inotify: a controller plugged in or switched on). It only reads (never grabs), so Steam,
//! hearth-pad and games still see every press. A controller it can't open (no permission) is just not read: the
//! keys still work. Buttons are mapped as hearth-pad maps them (button_map: xpad's X is BTN_NORTH, Sony's Y is).

use futures::channel::mpsc::{unbounded, UnboundedSender};
use futures::Stream;
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
}

/// The controller's presses, for as long as the app runs (Subscription::run).
pub fn presses() -> impl Stream<Item = Button> {
    let (tx, rx) = unbounded();
    let _ = std::thread::Builder::new().name("hearth-settings-pad".into()).stack_size(128 * 1024).spawn(move || reader(tx));
    rx
}

const EV_KEY: u16 = 0x01;
const EV_ABS: u16 = 0x03;
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
    stick: [i8; 2],
    hat: [i32; 2],
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
        if !bit(&evs, EV_ABS as usize) || !bit(&keys, BTN_SOUTH as usize) {
            return shut(fd);
        }
        let mut id = [0u16; 4];
        libc::ioctl(fd, ior(0x02, 8) as _, id.as_mut_ptr());
        let mut range = [(-32768, 32767); 2];
        for (i, r) in range.iter_mut().enumerate() {
            let mut abs = [0i32; 6];
            if libc::ioctl(fd, ior(0x40 + i as u32, 24) as _, abs.as_mut_ptr()) == 0 && abs[2] > abs[1] {
                *r = (abs[1], abs[2]);
            }
        }
        Some(Pad { fd, path: path.to_string(), sony: id[1] == SONY, range, stick: [0, 0], hat: [0, 0] })
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

/// Reads what came from one controller. False: it's gone.
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
            match ty {
                EV_KEY if value == 1 => out.extend(button(code, p.sony)),
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
    loop {
        let mut fds: Vec<libc::pollfd> = pads.iter().map(|p| libc::pollfd { fd: p.fd, events: libc::POLLIN, revents: 0 }).collect();
        if ino >= 0 {
            fds.push(libc::pollfd { fd: ino, events: libc::POLLIN, revents: 0 });
        }
        if fds.is_empty() {
            return; // nothing to read and no way to see one come: the keys still work
        }
        // SAFETY: fds is a live array of pollfd of the given length.
        let n = unsafe { libc::poll(fds.as_mut_ptr(), fds.len() as libc::nfds_t, -1) };
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
