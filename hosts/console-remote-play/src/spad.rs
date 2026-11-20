//! The controller while streaming: every button, stick and trigger of one controller, read from /dev/input and sent
//! to the Xbox the moment it changes (input.rs's packets), with no polling: the thread sleeps in poll() until the
//! controller sends something. Hold View + Menu for 1 s (input::Combo) and the Console Remote Play menu opens; while
//! it's open the Xbox gets nothing and the buttons move the menu (they come back as `Nav`). Rumble the game asks for
//! plays on the controller (force feedback) when Vibration is on. It starts with the stream and stops with it.
//!
//! Which controller (Settings › Play with): "builtin" prefers the handheld's own controls (the ROG Ally's, through
//! Handheld Daemon or the kernel's xpad), "paired" a Bluetooth or USB controller; either way the one used first wins.

use crate::input::{Combo, Pad};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::time::Instant;

const EV_KEY: u16 = 0x01;
const EV_ABS: u16 = 0x03;
const EV_FF: u16 = 0x15;
const BTN_SOUTH: u16 = 0x130;
const BUS_USB: u16 = 0x03;
const BUS_BLUETOOTH: u16 = 0x05;
const SONY: u16 = 0x054c;

const fn ioc(dir: u32, nr: u32, size: u32) -> u64 {
    ((dir << 30) | (size << 16) | (0x45 << 8) | nr) as u64
}

/// What the stream's reader sends back to the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Nav {
    /// View + Menu held 1 s: the menu opens (or closes).
    Menu,
    A,
    B,
    Up,
    Down,
    Left,
    Right,
}

/// Shared with the stream: the menu is open (the Xbox gets nothing), and the rumble switch.
#[derive(Default)]
pub struct Flags {
    pub menu_open: AtomicBool,
    pub vibration: AtomicBool,
    pub stop: AtomicBool,
}

struct Dev {
    fd: i32,
    path: String,
    sony: bool,
    preferred: bool,
    /// ABS 0..5 and the hats: (min, max).
    range: [(i32, i32); 6],
    has_abs_triggers: bool,
    pad: Pad,
    rumble_id: i16,
}

fn bit(bits: &[u8], n: usize) -> bool {
    bits.get(n / 8).map(|b| b & (1 << (n % 8)) != 0).unwrap_or(false)
}

fn open(path: &str, prefer_builtin: bool) -> Option<Dev> {
    let c = std::ffi::CString::new(path).ok()?;
    // SAFETY: plain syscalls on a descriptor this function owns; every buffer is as large as the ioctl says.
    unsafe {
        let mut fd = libc::open(c.as_ptr(), libc::O_RDWR | libc::O_NONBLOCK | libc::O_CLOEXEC);
        if fd < 0 {
            fd = libc::open(c.as_ptr(), libc::O_RDONLY | libc::O_NONBLOCK | libc::O_CLOEXEC);
        }
        if fd < 0 {
            return None;
        }
        let mut name = [0u8; 128];
        libc::ioctl(fd, ioc(2, 0x06, 128) as _, name.as_mut_ptr());
        let name = String::from_utf8_lossy(&name[..name.iter().position(|b| *b == 0).unwrap_or(0)]).to_lowercase();
        let mut evs = [0u8; 4];
        let mut keys = [0u8; 96];
        libc::ioctl(fd, ioc(2, 0x20, 4) as _, evs.as_mut_ptr());
        libc::ioctl(fd, ioc(2, 0x20 + EV_KEY as u32, 96) as _, keys.as_mut_ptr());
        if name.starts_with("hearth ") || !bit(&evs, EV_ABS as usize) || !bit(&keys, BTN_SOUTH as usize) {
            libc::close(fd);
            return None;
        }
        let mut id = [0u16; 4];
        libc::ioctl(fd, ioc(2, 0x02, 8) as _, id.as_mut_ptr());
        let builtin = ["ally", "handheld", "hhd", "steam deck", "legion", "ayaneo", "gpd"].iter().any(|w| name.contains(w));
        let paired = id[0] == BUS_BLUETOOTH || (id[0] == BUS_USB && !builtin);
        let mut range = [(-32768, 32767); 6];
        let mut abs_bits = [0u8; 8];
        libc::ioctl(fd, ioc(2, 0x20 + EV_ABS as u32, 8) as _, abs_bits.as_mut_ptr());
        for (i, r) in range.iter_mut().enumerate() {
            let mut abs = [0i32; 6];
            if libc::ioctl(fd, ioc(2, 0x40 + i as u32, 24) as _, abs.as_mut_ptr()) == 0 && abs[2] > abs[1] {
                *r = (abs[1], abs[2]);
            }
        }
        let sony = id[1] == SONY;
        let has_abs_triggers = if sony { bit(&abs_bits, 3) && bit(&abs_bits, 4) } else { bit(&abs_bits, 2) && bit(&abs_bits, 5) };
        Some(Dev { fd, path: path.to_string(), sony, preferred: if prefer_builtin { builtin } else { paired }, range, has_abs_triggers, pad: Pad::default(), rumble_id: -1 })
    }
}

fn norm(v: i32, (lo, hi): (i32, i32)) -> f32 {
    let mid = (lo as f32 + hi as f32) / 2.0;
    let half = ((hi - lo) as f32 / 2.0).max(1.0);
    let x = (v as f32 - mid) / half;
    if x.abs() < 0.06 { 0.0 } else { x.clamp(-1.0, 1.0) }
}

fn norm01(v: i32, (lo, hi): (i32, i32)) -> f32 {
    ((v - lo) as f32 / ((hi - lo) as f32).max(1.0)).clamp(0.0, 1.0)
}

fn apply(d: &mut Dev, ty: u16, code: u16, value: i32) {
    let p = &mut d.pad;
    let on = value != 0;
    match ty {
        EV_KEY => match code {
            0x130 => p.a = on,
            0x131 => p.b = on,
            0x133 => if d.sony { p.y = on } else { p.x = on },
            0x134 => if d.sony { p.x = on } else { p.y = on },
            0x136 => p.lb = on,
            0x137 => p.rb = on,
            0x138 if !d.has_abs_triggers => p.lt = if on { 1.0 } else { 0.0 },
            0x139 if !d.has_abs_triggers => p.rt = if on { 1.0 } else { 0.0 },
            0x13a => p.view = on,
            0x13b => p.menu = on,
            0x13c => p.nexus = on,
            0x13d => p.ls = on,
            0x13e => p.rs = on,
            0x220 => p.up = on,
            0x221 => p.down = on,
            0x222 => p.left = on,
            0x223 => p.right = on,
            _ => {}
        },
        EV_ABS => {
            let r = |i: usize| d.range[i.min(5)];
            match (code, d.sony) {
                (0x00, _) => p.lx = norm(value, r(0)),
                (0x01, _) => p.ly = norm(value, r(1)),
                // xpad: Z and RZ are the triggers, RX / RY the right stick; Sony: Z / RZ the right stick, RX / RY the triggers.
                (0x02, false) => p.lt = norm01(value, r(2)),
                (0x05, false) => p.rt = norm01(value, r(5)),
                (0x03, false) => p.rx = norm(value, r(3)),
                (0x04, false) => p.ry = norm(value, r(4)),
                (0x02, true) => p.rx = norm(value, r(2)),
                (0x05, true) => p.ry = norm(value, r(5)),
                (0x03, true) => p.lt = norm01(value, r(3)),
                (0x04, true) => p.rt = norm01(value, r(4)),
                (0x10, _) => {
                    p.left = value < 0;
                    p.right = value > 0;
                }
                (0x11, _) => {
                    p.up = value < 0;
                    p.down = value > 0;
                }
                _ => {}
            }
        }
        _ => {}
    }
}

/// Reads what came; false: it's gone. True in `.1` when something changed.
fn read(d: &mut Dev) -> (bool, bool) {
    let mut buf = [0u8; 24 * 64];
    let before = d.pad;
    loop {
        // SAFETY: reading into a local buffer from a descriptor this thread owns.
        let n = unsafe { libc::read(d.fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
        if n < 0 {
            let e = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
            return (e == libc::EAGAIN || e == libc::EINTR, d.pad != before);
        }
        if n == 0 {
            return (false, false);
        }
        for ev in buf[..n as usize].chunks_exact(24) {
            let ty = u16::from_ne_bytes([ev[16], ev[17]]);
            let code = u16::from_ne_bytes([ev[18], ev[19]]);
            let value = i32::from_ne_bytes([ev[20], ev[21], ev[22], ev[23]]);
            apply(d, ty, code, value);
        }
        if (n as usize) < buf.len() {
            return (true, d.pad != before);
        }
    }
}

#[repr(C)]
struct FfEffect {
    kind: u16,
    id: i16,
    direction: u16,
    trigger: [u16; 2],
    replay: [u16; 2],
    u: [u64; 4],
}

const FF_RUMBLE: u16 = 0x50;

/// Plays a rumble on the controller (force feedback): uploads it once, then plays it.
fn rumble(d: &mut Dev, strong: f32, weak: f32, ms: u16) {
    let mut e = FfEffect { kind: FF_RUMBLE, id: d.rumble_id, direction: 0, trigger: [0, 0], replay: [ms.max(1), 0], u: [0; 4] };
    let s = (strong.clamp(0.0, 1.0) * 65535.0) as u64;
    let w = (weak.clamp(0.0, 1.0) * 65535.0) as u64;
    e.u[0] = s | (w << 16);
    // SAFETY: EVIOCSFF with a ff_effect-sized struct; the kernel writes back its id.
    let ok = unsafe { libc::ioctl(d.fd, ioc(1, 0x80, std::mem::size_of::<FfEffect>() as u32) as _, &mut e as *mut FfEffect) } >= 0;
    if !ok {
        return;
    }
    d.rumble_id = e.id;
    let mut ev = [0u8; 24];
    ev[16..18].copy_from_slice(&EV_FF.to_ne_bytes());
    ev[18..20].copy_from_slice(&(e.id as u16).to_ne_bytes());
    ev[20..24].copy_from_slice(&1i32.to_ne_bytes());
    // SAFETY: writing one input_event to a descriptor this thread owns.
    unsafe { libc::write(d.fd, ev.as_ptr() as *const libc::c_void, ev.len()) };
}

fn now_ms(t0: Instant) -> u64 {
    t0.elapsed().as_millis() as u64
}

/// Runs until `flags.stop`: `send` gets every change while the menu is closed; `nav` the combo and the menu's buttons.
/// `rumbles` brings (strong, weak, ms) from the stream.
pub fn run(prefer_builtin: bool, flags: Arc<Flags>, mut send: impl FnMut(Pad) + Send + 'static, nav: Sender<Nav>, rumbles: Receiver<(f32, f32, u16)>) {
    let _ = std::thread::Builder::new().name("crp-stream-pad".into()).stack_size(256 * 1024).spawn(move || {
        let t0 = Instant::now();
        let mut devs: Vec<Dev> = Vec::new();
        if let Ok(dir) = std::fs::read_dir("/dev/input") {
            for e in dir.flatten() {
                let n = e.file_name().to_string_lossy().to_string();
                if n.starts_with("event") {
                    if let Some(d) = open(&format!("/dev/input/{n}"), prefer_builtin) {
                        devs.push(d);
                    }
                }
            }
        }
        devs.sort_by_key(|d| !d.preferred);
        let mut active: Option<String> = devs.iter().find(|d| d.preferred).map(|d| d.path.clone());
        let mut combo = Combo::default();
        let mut last_nav = Pad::default();
        while !flags.stop.load(Ordering::Relaxed) {
            let mut fds: Vec<libc::pollfd> = devs.iter().map(|d| libc::pollfd { fd: d.fd, events: libc::POLLIN, revents: 0 }).collect();
            // While View + Menu are held, look again every 100 ms (to open the menu at 1 s); else only on input,
            // and once every 250 ms for the stop flag and rumbles (the stream is live: this is its only timer).
            let timeout = if combo.holding() { 100 } else { 250 };
            if fds.is_empty() {
                std::thread::sleep(std::time::Duration::from_millis(500));
            } else {
                // SAFETY: fds is a live array of pollfd.
                unsafe { libc::poll(fds.as_mut_ptr(), fds.len() as libc::nfds_t, timeout) };
            }
            while let Ok((s, w, ms)) = rumbles.try_recv() {
                if flags.vibration.load(Ordering::Relaxed) {
                    if let Some(d) = devs.iter_mut().find(|d| Some(&d.path) == active.as_ref()) {
                        rumble(d, s, w, ms);
                    }
                }
            }
            let mut gone = Vec::new();
            for (i, f) in fds.iter().enumerate() {
                if f.revents & (libc::POLLERR | libc::POLLHUP | libc::POLLNVAL) != 0 {
                    gone.push(i);
                    continue;
                }
                if f.revents & libc::POLLIN == 0 {
                    continue;
                }
                let (alive, changed) = read(&mut devs[i]);
                if !alive {
                    gone.push(i);
                    continue;
                }
                if !changed {
                    continue;
                }
                if active.is_none() {
                    active = Some(devs[i].path.clone());
                }
                if active.as_deref() != Some(devs[i].path.as_str()) {
                    continue;
                }
                let p = devs[i].pad;
                if combo.feed(&p, now_ms(t0)) {
                    let _ = nav.send(Nav::Menu);
                    continue;
                }
                if flags.menu_open.load(Ordering::Relaxed) {
                    // The menu has the buttons: only fresh presses, the D-pad or the stick past half way.
                    let pressed = |now: bool, was: bool| now && !was;
                    let dir = |v: f32| if v > 0.5 { 1 } else if v < -0.5 { -1 } else { 0 };
                    let moves = [
                        (pressed(p.a, last_nav.a), Nav::A),
                        (pressed(p.b, last_nav.b), Nav::B),
                        (pressed(p.up || dir(p.ly) < 0, last_nav.up || dir(last_nav.ly) < 0), Nav::Up),
                        (pressed(p.down || dir(p.ly) > 0, last_nav.down || dir(last_nav.ly) > 0), Nav::Down),
                        (pressed(p.left || dir(p.lx) < 0, last_nav.left || dir(last_nav.lx) < 0), Nav::Left),
                        (pressed(p.right || dir(p.lx) > 0, last_nav.right || dir(last_nav.lx) > 0), Nav::Right),
                    ];
                    for (go, n) in moves {
                        if go {
                            let _ = nav.send(n);
                        }
                    }
                    last_nav = p;
                    continue;
                }
                last_nav = p;
                send(p);
            }
            if combo.holding() {
                let p = devs.iter().find(|d| Some(&d.path) == active.as_ref()).map(|d| d.pad).unwrap_or_default();
                if combo.feed(&p, now_ms(t0)) {
                    let _ = nav.send(Nav::Menu);
                }
            }
            for i in gone.into_iter().rev() {
                let d = devs.remove(i);
                if active.as_deref() == Some(d.path.as_str()) {
                    active = None;
                }
                // SAFETY: closing a descriptor this thread opened.
                unsafe { libc::close(d.fd) };
            }
        }
        for d in devs {
            // SAFETY: closing descriptors this thread opened.
            unsafe { libc::close(d.fd) };
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_effect_is_the_kernels_size() {
        assert_eq!(std::mem::size_of::<FfEffect>(), 48);
        assert_eq!(ioc(1, 0x80, 48), 0x4030_4580);
        assert_eq!(norm(0, (-32768, 32767)), 0.0);
        assert!((norm(32767, (-32768, 32767)) - 1.0).abs() < 0.001);
        assert_eq!(norm01(255, (0, 255)), 1.0);
    }
}
