//! The viewer's controller pointer as a real mouse. The window app reads the controller (its own page, never the
//! site's) and sends "ptr" lines; this turns them into input the system itself delivers, so every page gets real,
//! trusted clicks wherever they land: inside frames from other sites, video players, popups and bot checks.
//!
//!   uinput: a virtual mouse and keyboard ("Hearth Theater pointer", /dev/uinput, as Game Mode's controller mouse
//!           hearth-pad does). Works under gamescope, labwc and any compositor. The default when /dev/uinput opens.
//!   XTest:  the X server's own test input, for an X11 display without /dev/uinput (CI's Xvfb).
//!
//! HEARTH_THEATER_POINTER=uinput|xtest|off picks one (tests). Nothing here can stop Theater: without either, the
//! viewer still shows pages, and the log says why the pointer is missing.

use hearth_theater::{notches, Key, Ptr};
use std::sync::Mutex;

pub trait Backend: Send {
    fn name(&self) -> String;
    fn apply(&mut self, p: &Ptr) -> Result<(), String>;
}

static POINTER: Mutex<Option<Box<dyn Backend>>> = Mutex::new(None);
static STATUS: Mutex<String> = Mutex::new(String::new());

/// Makes the pointer (once, at start). Returns what the log should say.
pub fn init() -> String {
    let want = std::env::var("HEARTH_THEATER_POINTER").unwrap_or_default().to_ascii_lowercase();
    let mut why = Vec::new();
    let made: Option<Box<dyn Backend>> = match want.as_str() {
        "off" => { why.push("turned off (HEARTH_THEATER_POINTER=off)".to_string()); None }
        "xtest" => xtest().map_err(|e| why.push(format!("XTest: {e}"))).ok(),
        "uinput" => uinput().map_err(|e| why.push(format!("uinput: {e}"))).ok(),
        _ => uinput().map_err(|e| why.push(format!("uinput: {e}"))).ok()
            .or_else(|| xtest().map_err(|e| why.push(format!("XTest: {e}"))).ok()),
    };
    let status = match &made {
        Some(b) => format!("pointer: {} (real input: clicks reach frames, players and popups)", b.name()),
        None => format!("pointer: none, so the controller can't click in the viewer ({})", why.join("; ")),
    };
    *POINTER.lock().unwrap_or_else(|e| e.into_inner()) = made;
    *STATUS.lock().unwrap_or_else(|e| e.into_inner()) = status.clone();
    status
}

pub fn status() -> String { STATUS.lock().unwrap_or_else(|e| e.into_inner()).clone() }

/// Does what a "ptr" line asks. An error is logged once in a while by the caller, never fatal.
pub fn apply(p: &Ptr) -> Result<(), String> {
    match POINTER.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
        Some(b) => b.apply(p),
        None => Err("no pointer".into()),
    }
}

// ------------------------------------------------------------------------------------------- uinput

#[cfg(target_os = "linux")]
mod ui {
    pub const EV_SYN: u16 = 0;
    pub const EV_KEY: u16 = 1;
    pub const EV_REL: u16 = 2;
    pub const REL_X: u16 = 0;
    pub const REL_Y: u16 = 1;
    pub const REL_HWHEEL: u16 = 6;
    pub const REL_WHEEL: u16 = 8;
    pub const REL_WHEEL_HI_RES: u16 = 11;
    pub const REL_HWHEEL_HI_RES: u16 = 12;
    pub const BTN_LEFT: u16 = 0x110;
    pub const BTN_RIGHT: u16 = 0x111;
    pub const BTN_MIDDLE: u16 = 0x112;
    // _IOW('U', n, int) and friends (linux/uinput.h).
    pub const UI_DEV_CREATE: u64 = 0x5501;
    pub const UI_DEV_DESTROY: u64 = 0x5502;
    pub const UI_DEV_SETUP: u64 = 0x405c_5503;
    pub const UI_SET_EVBIT: u64 = 0x4004_5564;
    pub const UI_SET_KEYBIT: u64 = 0x4004_5565;
    pub const UI_SET_RELBIT: u64 = 0x4004_5566;

    #[repr(C)]
    pub struct Setup { pub bustype: u16, pub vendor: u16, pub product: u16, pub version: u16, pub name: [u8; 80], pub ff_effects_max: u32 }
}

#[cfg(target_os = "linux")]
fn key_code(k: Key) -> u16 {
    match k {
        Key::Escape => 1, Key::Backspace => 14, Key::Tab => 15, Key::Enter => 28, Key::Space => 57, Key::Home => 102,
        Key::Up => 103, Key::Left => 105, Key::Right => 106, Key::End => 107, Key::Down => 108,
    }
}

#[cfg(target_os = "linux")]
struct Uinput { fd: i32, wheel: [i32; 2] }

#[cfg(target_os = "linux")]
impl Uinput {
    fn send(&self, events: &[(u16, u16, i32)]) -> Result<(), String> {
        let mut buf = Vec::with_capacity((events.len() + 1) * 24);
        for &(t, c, v) in events.iter().chain(std::iter::once(&(ui::EV_SYN, 0, 0))) {
            buf.extend_from_slice(&[0u8; 16]); // the time: the kernel stamps it
            buf.extend_from_slice(&t.to_ne_bytes());
            buf.extend_from_slice(&c.to_ne_bytes());
            buf.extend_from_slice(&v.to_ne_bytes());
        }
        let n = unsafe { libc::write(self.fd, buf.as_ptr() as *const libc::c_void, buf.len()) };
        if n as usize == buf.len() { Ok(()) } else { Err(format!("writing to /dev/uinput: {}", std::io::Error::last_os_error())) }
    }
}

#[cfg(target_os = "linux")]
impl Drop for Uinput {
    fn drop(&mut self) { unsafe { libc::ioctl(self.fd, ui::UI_DEV_DESTROY as _); libc::close(self.fd); } }
}

#[cfg(target_os = "linux")]
impl Backend for Uinput {
    fn name(&self) -> String { "uinput (a virtual mouse, \"Hearth Theater pointer\")".into() }
    fn apply(&mut self, p: &Ptr) -> Result<(), String> {
        match *p {
            Ptr::Move(dx, dy) => self.send(&[(ui::EV_REL, ui::REL_X, dx), (ui::EV_REL, ui::REL_Y, dy)]),
            Ptr::Warp(..) => Err("uinput can't put the pointer at a place".into()),
            Ptr::Button(b, down) => {
                let code = match b { 1 => ui::BTN_LEFT, 2 => ui::BTN_MIDDLE, _ => ui::BTN_RIGHT };
                self.send(&[(ui::EV_KEY, code, down as i32)])
            }
            Ptr::Wheel(dx, dy) => {
                // REL_WHEEL > 0 is away from you (up): the opposite of "scroll down".
                let mut ev = Vec::new();
                if dy != 0 {
                    ev.push((ui::EV_REL, ui::REL_WHEEL_HI_RES, -dy));
                    let n = notches(&mut self.wheel[1], dy);
                    if n != 0 { ev.push((ui::EV_REL, ui::REL_WHEEL, -n)); }
                }
                if dx != 0 {
                    ev.push((ui::EV_REL, ui::REL_HWHEEL_HI_RES, dx));
                    let n = notches(&mut self.wheel[0], dx);
                    if n != 0 { ev.push((ui::EV_REL, ui::REL_HWHEEL, n)); }
                }
                self.send(&ev)
            }
            Ptr::Keys(k, n) => {
                for _ in 0..n {
                    self.send(&[(ui::EV_KEY, key_code(k), 1)])?;
                    self.send(&[(ui::EV_KEY, key_code(k), 0)])?;
                }
                Ok(())
            }
        }
    }
}

#[cfg(target_os = "linux")]
fn uinput() -> Result<Box<dyn Backend>, String> {
    let path = std::ffi::CString::new("/dev/uinput").unwrap();
    let fd = unsafe { libc::open(path.as_ptr(), libc::O_WRONLY | libc::O_NONBLOCK | libc::O_CLOEXEC) };
    if fd < 0 { return Err(format!("can't open /dev/uinput ({})", std::io::Error::last_os_error())); }
    let dev = Uinput { fd, wheel: [0, 0] };
    let set = |req: u64, v: u16| -> Result<(), String> {
        if unsafe { libc::ioctl(fd, req as _, v as libc::c_int) } < 0 { Err(format!("setting it up ({})", std::io::Error::last_os_error())) } else { Ok(()) }
    };
    set(ui::UI_SET_EVBIT, ui::EV_KEY)?;
    set(ui::UI_SET_EVBIT, ui::EV_REL)?;
    for b in [ui::BTN_LEFT, ui::BTN_RIGHT, ui::BTN_MIDDLE] { set(ui::UI_SET_KEYBIT, b)?; }
    for k in [Key::Escape, Key::Backspace, Key::Enter, Key::Tab, Key::Space, Key::Left, Key::Right, Key::Up, Key::Down, Key::Home, Key::End] {
        set(ui::UI_SET_KEYBIT, key_code(k))?;
    }
    for r in [ui::REL_X, ui::REL_Y, ui::REL_WHEEL, ui::REL_HWHEEL, ui::REL_WHEEL_HI_RES, ui::REL_HWHEEL_HI_RES] { set(ui::UI_SET_RELBIT, r)?; }
    let mut setup = ui::Setup { bustype: 0x06 /* BUS_VIRTUAL */, vendor: 0x4854, product: 0x0101, version: 1, name: [0; 80], ff_effects_max: 0 };
    let name = b"Hearth Theater pointer";
    setup.name[..name.len()].copy_from_slice(name);
    if unsafe { libc::ioctl(fd, ui::UI_DEV_SETUP as _, &setup as *const ui::Setup) } < 0 {
        return Err(format!("naming it ({})", std::io::Error::last_os_error()));
    }
    if unsafe { libc::ioctl(fd, ui::UI_DEV_CREATE as _) } < 0 {
        return Err(format!("making it ({})", std::io::Error::last_os_error()));
    }
    Ok(Box::new(dev))
}

#[cfg(not(target_os = "linux"))]
fn uinput() -> Result<Box<dyn Backend>, String> { Err("only on Linux".into()) }

// ------------------------------------------------------------------------------------------- XTest

#[cfg(target_os = "linux")]
struct Xtest { conn: x11rb::rust_connection::RustConnection, root: u32, display: String, keys: std::collections::HashMap<Key, u8>, wheel: [i32; 2] }

#[cfg(target_os = "linux")]
impl Xtest {
    fn fake(&self, kind: u8, detail: u8, x: i16, y: i16) -> Result<(), String> {
        use x11rb::protocol::xtest::ConnectionExt as _;
        self.conn.xtest_fake_input(kind, detail, 0, self.root, x, y, 0).map_err(|e| e.to_string())?;
        Ok(())
    }
    fn click(&self, button: u8, times: i32) -> Result<(), String> {
        for _ in 0..times { self.fake(4, button, 0, 0)?; self.fake(5, button, 0, 0)?; }
        Ok(())
    }
}

#[cfg(target_os = "linux")]
impl Backend for Xtest {
    fn name(&self) -> String { format!("XTest on X display {}", self.display) }
    fn apply(&mut self, p: &Ptr) -> Result<(), String> {
        use x11rb::connection::Connection as _;
        const MOTION: u8 = 6;
        match *p {
            // detail 1: relative to where it is; 0: a place on the screen.
            Ptr::Move(dx, dy) => self.fake(MOTION, 1, dx.clamp(-32000, 32000) as i16, dy.clamp(-32000, 32000) as i16)?,
            Ptr::Warp(x, y) => self.fake(MOTION, 0, x.min(32000) as i16, y.min(32000) as i16)?,
            Ptr::Button(b, down) => self.fake(if down { 4 } else { 5 }, b, 0, 0)?,
            Ptr::Wheel(dx, dy) => {
                let v = notches(&mut self.wheel[1], dy);
                let h = notches(&mut self.wheel[0], dx);
                if v != 0 { self.click(if v > 0 { 5 } else { 4 }, v.abs())?; }
                if h != 0 { self.click(if h > 0 { 7 } else { 6 }, h.abs())?; }
            }
            Ptr::Keys(k, n) => {
                let code = *self.keys.get(&k).ok_or("that key isn't on this keyboard map")?;
                for _ in 0..n { self.fake(2, code, 0, 0)?; self.fake(3, code, 0, 0)?; }
            }
        }
        self.conn.flush().map_err(|e| e.to_string())
    }
}

#[cfg(target_os = "linux")]
fn xtest() -> Result<Box<dyn Backend>, String> {
    use x11rb::connection::Connection as _;
    use x11rb::protocol::xproto::ConnectionExt as _;
    use x11rb::protocol::xtest::ConnectionExt as _;
    let display = std::env::var("DISPLAY").unwrap_or_default();
    if display.is_empty() { return Err("no X display".into()); }
    let (conn, screen) = x11rb::connect(None).map_err(|e| format!("can't connect to {display} ({e})"))?;
    conn.xtest_get_version(2, 2).map_err(|e| e.to_string())?.reply().map_err(|e| format!("the display has no XTest ({e})"))?;
    let root = conn.setup().roots.get(screen).ok_or("no screen")?.root;
    // Each key's keycode on this display's keyboard map.
    let (min, max) = (conn.setup().min_keycode, conn.setup().max_keycode);
    let mut keys = std::collections::HashMap::new();
    if let Ok(Ok(m)) = conn.get_keyboard_mapping(min, max - min + 1).map(|c| c.reply()) {
        let per = m.keysyms_per_keycode.max(1) as usize;
        let want = [(Key::Escape, 0xff1b), (Key::Backspace, 0xff08), (Key::Enter, 0xff0d), (Key::Tab, 0xff09), (Key::Space, 0x20),
                    (Key::Left, 0xff51), (Key::Up, 0xff52), (Key::Right, 0xff53), (Key::Down, 0xff54), (Key::Home, 0xff50), (Key::End, 0xff57)];
        for (k, sym) in want {
            if let Some(i) = m.keysyms.chunks(per).position(|c| c.first() == Some(&sym)) { keys.insert(k, min + i as u8); }
        }
    }
    Ok(Box::new(Xtest { conn, root, display, keys, wheel: [0, 0] }))
}

#[cfg(not(target_os = "linux"))]
fn xtest() -> Result<Box<dyn Backend>, String> { Err("only on Linux".into()) }
