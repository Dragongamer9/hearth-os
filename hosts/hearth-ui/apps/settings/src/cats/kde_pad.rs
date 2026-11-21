//! Controllers in KDE › the live test (KDE's Game Controller page): every controller's buttons, sticks, triggers and
//! D-pad as they're pressed, read straight from /dev/input (only reading, never grabbing: Steam and games still get
//! everything). It runs only while the page is on the screen (kde.rs asks for it from its subscription); then one
//! thread waits in the kernel (poll, half a second at most so it notices the page was closed), sends at most 30
//! updates a second, and ends itself when the page goes away. Nothing at rest.
//!
//! HEARTH_PADTEST_FIXTURE=<json file>: CI's pictures (no controller there) show the state in the file instead.

use futures::channel::mpsc::{unbounded, UnboundedSender};
use futures::Stream;
use std::time::{Duration, Instant};

/// One controller as it is now.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Pad {
    pub name: String,
    /// The buttons held now (Linux's codes: BTN_SOUTH 0x130 …).
    pub held: Vec<u16>,
    /// Left stick x, y; right stick x, y (−1 … 1); left and right trigger (0 … 1).
    pub axes: [f32; 6],
    /// D-pad: x, y (−1, 0, 1).
    pub hat: [i32; 2],
}

pub fn fixture() -> Option<Vec<Pad>> {
    let p = std::env::var_os("HEARTH_PADTEST_FIXTURE")?;
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(p).ok()?).ok()?;
    Some(
        v.as_array()?
            .iter()
            .map(|x| {
                let mut axes = [0.0f32; 6];
                for (i, a) in x.get("axes").and_then(|a| a.as_array()).cloned().unwrap_or_default().iter().enumerate().take(6) {
                    axes[i] = a.as_f64().unwrap_or(0.0) as f32;
                }
                let hat = x.get("hat").and_then(|h| h.as_array()).map(|h| [h.first().and_then(|v| v.as_i64()).unwrap_or(0) as i32, h.get(1).and_then(|v| v.as_i64()).unwrap_or(0) as i32]).unwrap_or([0, 0]);
                Pad {
                    name: x.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string(),
                    held: x.get("held").and_then(|h| h.as_array()).map(|h| h.iter().filter_map(|b| b.as_u64()).map(|b| b as u16).collect()).unwrap_or_default(),
                    axes,
                    hat,
                }
            })
            .collect(),
    )
}

/// The controllers' state, for as long as the test page is shown (Subscription::run).
pub fn watch() -> impl Stream<Item = Vec<Pad>> {
    let (tx, rx) = unbounded();
    if let Some(f) = fixture() {
        let _ = tx.unbounded_send(f);
        // The sender stays open (dropping it would end the stream, and the page would be told no more).
        std::mem::forget(tx);
        return rx;
    }
    let _ = std::thread::Builder::new().name("hearth-settings-padtest".into()).stack_size(128 * 1024).spawn(move || reader(tx));
    rx
}

const EV_KEY: u16 = 0x01;
const EV_ABS: u16 = 0x03;
const BTN_SOUTH: usize = 0x130;

fn ior(nr: u32, size: u32) -> libc::c_ulong {
    ((2u32 << 30) | (size << 16) | ((b'E' as u32) << 8) | nr) as libc::c_ulong
}
fn bit(bits: &[u8], n: usize) -> bool {
    bits.get(n / 8).map(|b| b & (1 << (n % 8)) != 0).unwrap_or(false)
}

struct Dev {
    fd: i32,
    path: String,
    /// ABS_X, Y, Z, RX, RY, RZ: (min, max).
    range: [(i32, i32); 6],
    pad: Pad,
}

fn open(path: &str) -> Option<Dev> {
    let c = std::ffi::CString::new(path).ok()?;
    // SAFETY: plain syscalls on a descriptor this function owns; each buffer is as large as its ioctl says.
    unsafe {
        let fd = libc::open(c.as_ptr(), libc::O_RDONLY | libc::O_NONBLOCK | libc::O_CLOEXEC);
        if fd < 0 {
            return None;
        }
        let mut evs = [0u8; 4];
        let mut keys = [0u8; 96];
        libc::ioctl(fd, ior(0x20, 4) as _, evs.as_mut_ptr());
        libc::ioctl(fd, ior(0x20 + EV_KEY as u32, 96) as _, keys.as_mut_ptr());
        if !bit(&evs, EV_ABS as usize) || !bit(&keys, BTN_SOUTH) {
            libc::close(fd);
            return None;
        }
        let mut name = [0u8; 128];
        libc::ioctl(fd, ior(0x06, 128) as _, name.as_mut_ptr());
        let name = String::from_utf8_lossy(&name[..name.iter().position(|b| *b == 0).unwrap_or(0)]).to_string();
        let mut range = [(-32768, 32767); 6];
        for (i, r) in range.iter_mut().enumerate() {
            let mut abs = [0i32; 6];
            if libc::ioctl(fd, ior(0x40 + i as u32, 24) as _, abs.as_mut_ptr()) == 0 && abs[2] > abs[1] {
                *r = (abs[1], abs[2]);
            }
        }
        Some(Dev { fd, path: path.to_string(), range, pad: Pad { name, ..Pad::default() } })
    }
}

fn norm(v: i32, (lo, hi): (i32, i32), trigger: bool) -> f32 {
    let f = (v - lo) as f32 / ((hi - lo).max(1)) as f32;
    if trigger {
        f.clamp(0.0, 1.0)
    } else {
        (f * 2.0 - 1.0).clamp(-1.0, 1.0)
    }
}

fn read(d: &mut Dev) -> bool {
    let mut buf = [0u8; 24 * 64];
    loop {
        // SAFETY: reading into a local buffer from a non-blocking descriptor this thread owns.
        let n = unsafe { libc::read(d.fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
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
                EV_KEY => {
                    d.pad.held.retain(|b| *b != code);
                    if value != 0 {
                        d.pad.held.push(code);
                    }
                }
                EV_ABS => match code {
                    // Left stick (0, 1), the triggers (2, 5), the right stick (3, 4), as xpad and hid-playstation give them.
                    0 => d.pad.axes[0] = norm(value, d.range[0], false),
                    1 => d.pad.axes[1] = norm(value, d.range[1], false),
                    3 => d.pad.axes[2] = norm(value, d.range[3], false),
                    4 => d.pad.axes[3] = norm(value, d.range[4], false),
                    2 => d.pad.axes[4] = norm(value, d.range[2], true),
                    5 => d.pad.axes[5] = norm(value, d.range[5], true),
                    0x10 => d.pad.hat[0] = value.signum(),
                    0x11 => d.pad.hat[1] = value.signum(),
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

fn reader(tx: UnboundedSender<Vec<Pad>>) {
    let mut devs: Vec<Dev> = Vec::new();
    let mut scanned = Instant::now() - Duration::from_secs(10);
    let mut last: Vec<Pad> = Vec::new();
    let mut sent = Instant::now() - Duration::from_secs(1);
    loop {
        if tx.is_closed() {
            break; // the page was closed
        }
        // Controllers plugged in while the page is open (looked for every 2 s, only while it's open).
        if scanned.elapsed() > Duration::from_secs(2) {
            scanned = Instant::now();
            if let Ok(dir) = std::fs::read_dir("/dev/input") {
                for e in dir.flatten() {
                    let path = e.path().to_string_lossy().to_string();
                    if path.contains("/event") && !devs.iter().any(|d| d.path == path) {
                        if let Some(d) = open(&path) {
                            devs.push(d);
                        }
                    }
                }
            }
        }
        let mut fds: Vec<libc::pollfd> = devs.iter().map(|d| libc::pollfd { fd: d.fd, events: libc::POLLIN, revents: 0 }).collect();
        // SAFETY: polling descriptors this thread owns, for at most half a second.
        let r = if fds.is_empty() { 0 } else { unsafe { libc::poll(fds.as_mut_ptr(), fds.len() as libc::nfds_t, 500) } };
        if fds.is_empty() {
            std::thread::sleep(Duration::from_millis(500));
        }
        if r > 0 {
            let mut gone = Vec::new();
            for (i, f) in fds.iter().enumerate() {
                if f.revents & (libc::POLLHUP | libc::POLLERR) != 0 || (f.revents & libc::POLLIN != 0 && !read(&mut devs[i])) {
                    gone.push(i);
                }
            }
            for i in gone.into_iter().rev() {
                // SAFETY: closing a descriptor this thread opened.
                unsafe { libc::close(devs[i].fd) };
                devs.remove(i);
            }
        }
        let now: Vec<Pad> = devs.iter().map(|d| d.pad.clone()).collect();
        if now != last && sent.elapsed() > Duration::from_millis(33) {
            if tx.unbounded_send(now.clone()).is_err() {
                break;
            }
            last = now;
            sent = Instant::now();
        }
    }
    for d in devs {
        // SAFETY: closing descriptors this thread opened.
        unsafe { libc::close(d.fd) };
    }
}

/// The buttons' names, as the test shows them (Xbox names; Sony's controllers send the same codes).
pub const BUTTONS: &[(u16, &str)] = &[
    (0x130, "A"),
    (0x131, "B"),
    (0x133, "X"),
    (0x134, "Y"),
    (0x136, "LB"),
    (0x137, "RB"),
    (0x13a, "View"),
    (0x13b, "Menu"),
    (0x13c, "Guide"),
    (0x13d, "L3"),
    (0x13e, "R3"),
];

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn axes_are_scaled() {
        assert_eq!(norm(0, (-32768, 32767), false).round(), 0.0);
        assert_eq!(norm(32767, (-32768, 32767), false), 1.0);
        assert_eq!(norm(255, (0, 255), true), 1.0);
        assert_eq!(norm(0, (0, 255), true), 0.0);
    }
}
