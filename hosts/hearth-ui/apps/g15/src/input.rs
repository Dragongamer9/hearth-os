//! Reading key events (evdev) and sending them through a virtual keyboard (uinput), as hearth-pad does, with plain
//! system calls. /dev/uinput is the signed-in person's (70-hearth-uinput.rules); the G-keys' input device too
//! (70-hearth-g15.rules).

use crate::keys::{EV_KEY, EV_SYN};
use std::ffi::CString;
use std::os::unix::io::RawFd;
use std::path::Path;

pub fn open(path: &Path, flags: i32) -> std::io::Result<RawFd> {
    let c = CString::new(path.as_os_str().as_encoded_bytes()).map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidInput))?;
    let fd = unsafe { libc::open(c.as_ptr(), flags | libc::O_CLOEXEC) };
    if fd < 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(fd)
    }
}

pub fn close(fd: RawFd) {
    if fd >= 0 {
        unsafe {
            libc::close(fd);
        }
    }
}

/// One key event: (code, value) with value 1 down, 0 up, 2 repeat.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ev {
    pub code: u16,
    pub value: i32,
}

pub enum ReadResult {
    Events(Vec<Ev>),
    /// Nothing to read now.
    Nothing,
    /// The device is gone (unplugged) or the other end closed.
    Gone,
}

const EV_SIZE: usize = std::mem::size_of::<libc::input_event>();

/// What's waiting on an evdev fd (non-blocking), key events only.
pub fn read_events(fd: RawFd) -> ReadResult {
    let mut buf = vec![0u8; EV_SIZE * 64];
    let n = unsafe { libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
    if n < 0 {
        let e = std::io::Error::last_os_error();
        return match e.raw_os_error() {
            Some(libc::EAGAIN) | Some(libc::EINTR) => ReadResult::Nothing,
            _ => ReadResult::Gone,
        };
    }
    if n == 0 {
        return ReadResult::Gone;
    }
    let mut out = Vec::new();
    for chunk in buf[..n as usize].chunks_exact(EV_SIZE) {
        let ev: libc::input_event = unsafe { std::ptr::read_unaligned(chunk.as_ptr() as *const libc::input_event) };
        if ev.type_ == EV_KEY {
            out.push(Ev { code: ev.code, value: ev.value });
        }
    }
    ReadResult::Events(out)
}

/// Where typing goes: the virtual keyboard, or a test's list.
pub trait Output: Send {
    fn key(&mut self, code: u16, down: bool);
    fn sync(&mut self);
}

const UI_SET_EVBIT: u64 = 0x4004_5564;
const UI_SET_KEYBIT: u64 = 0x4004_5565;
const UI_DEV_SETUP: u64 = 0x405c_5503;
const UI_DEV_CREATE: u64 = 0x5501;
const UI_DEV_DESTROY: u64 = 0x5502;

#[repr(C)]
struct UinputSetup {
    bustype: u16,
    vendor: u16,
    product: u16,
    version: u16,
    name: [u8; 80],
    ff_effects_max: u32,
}

/// "Hearth G15 keyboard": the virtual keyboard G-keys type through.
pub struct Uinput {
    fd: RawFd,
}

impl Uinput {
    pub fn new(dev: &Path) -> std::io::Result<Uinput> {
        let fd = open(&dev.join("uinput"), libc::O_WRONLY | libc::O_NONBLOCK)?;
        let fail = |fd| {
            let e = std::io::Error::last_os_error();
            close(fd);
            e
        };
        unsafe {
            if libc::ioctl(fd, UI_SET_EVBIT as _, EV_KEY as libc::c_int) < 0 {
                return Err(fail(fd));
            }
            for c in crate::keys::all_codes() {
                if libc::ioctl(fd, UI_SET_KEYBIT as _, c as libc::c_int) < 0 {
                    return Err(fail(fd));
                }
            }
            let mut s = UinputSetup { bustype: 0x06 /* BUS_VIRTUAL */, vendor: 0x046d, product: 0x4815, version: 1, name: [0; 80], ff_effects_max: 0 };
            let n = b"Hearth G15 keyboard";
            s.name[..n.len()].copy_from_slice(n);
            if libc::ioctl(fd, UI_DEV_SETUP as _, &s as *const UinputSetup) < 0 {
                return Err(fail(fd));
            }
            if libc::ioctl(fd, UI_DEV_CREATE as _) < 0 {
                return Err(fail(fd));
            }
        }
        // The desktop needs a moment to see a new keyboard before the first key.
        std::thread::sleep(std::time::Duration::from_millis(150));
        Ok(Uinput { fd })
    }

    fn write(&mut self, type_: u16, code: u16, value: i32) {
        let ev = libc::input_event { time: libc::timeval { tv_sec: 0, tv_usec: 0 }, type_, code, value };
        let _ = unsafe { libc::write(self.fd, &ev as *const libc::input_event as *const libc::c_void, EV_SIZE) };
    }
}

impl Output for Uinput {
    fn key(&mut self, code: u16, down: bool) {
        self.write(EV_KEY, code, down as i32);
    }
    fn sync(&mut self) {
        self.write(EV_SYN, 0, 0);
    }
}

impl Drop for Uinput {
    fn drop(&mut self) {
        unsafe {
            libc::ioctl(self.fd, UI_DEV_DESTROY as _);
        }
        close(self.fd);
    }
}

/// A test's (and the fixture's) keyboard: what would have been typed.
#[derive(Default)]
pub struct Recorded {
    pub keys: Vec<(u16, bool)>,
}

impl Output for Recorded {
    fn key(&mut self, code: u16, down: bool) {
        self.keys.push((code, down));
    }
    fn sync(&mut self) {}
}

/// poll() on some fds for `ms` (−1: until something happens). Returns each fd's revents.
pub fn poll(fds: &[(RawFd, i16)], ms: i32) -> Vec<i16> {
    let mut p: Vec<libc::pollfd> = fds.iter().map(|&(fd, ev)| libc::pollfd { fd, events: ev, revents: 0 }).collect();
    let n = unsafe { libc::poll(p.as_mut_ptr(), p.len() as libc::nfds_t, ms) };
    if n <= 0 {
        return vec![0; fds.len()];
    }
    p.iter().map(|x| x.revents).collect()
}
