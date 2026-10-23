//! The phone as touchpad and keyboard (KDE Connect's "Remote input": kdeconnect.mousepad.request). Hearth makes a
//! virtual mouse and keyboard through the kernel's uinput (/dev/uinput; Hearth OS lets the person at the device use
//! it: 70-hearth-uinput.rules). That works the same in Game Mode (gamescope) and Desktop Mode (labwc), which is why
//! Hearth Connect does this itself instead of using kdeconnectd (whose Wayland input needs a desktop portal those
//! compositors don't have). Text the US keyboard layout can't type is typed with wtype in Desktop Mode.

use serde_json::Value;

/// KDE Connect's special keys (the index the phone sends) as Linux key codes.
pub const SPECIAL_KEYS: [u16; 33] = [
    0, 14, 15, 28, 105, 103, 106, 108, 104, 109, 102, 107, 28, 111, 1, 99, 70, 29, 56, 42, 125,
    59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 87, 88,
];

pub const KEY_LEFTSHIFT: u16 = 42;
pub const KEY_LEFTCTRL: u16 = 29;
pub const KEY_LEFTALT: u16 = 56;
pub const KEY_LEFTMETA: u16 = 125;
pub const BTN_LEFT: u16 = 0x110;
pub const BTN_RIGHT: u16 = 0x111;
pub const BTN_MIDDLE: u16 = 0x112;
pub const KEY_MUTE: u16 = 113;
pub const KEY_VOLUMEDOWN: u16 = 114;
pub const KEY_VOLUMEUP: u16 = 115;
pub const KEY_NEXTSONG: u16 = 163;
pub const KEY_PLAYPAUSE: u16 = 164;
pub const KEY_PREVIOUSSONG: u16 = 165;
pub const KEY_STOPCD: u16 = 166;

/// A character on a US keyboard: (key code, shift held).
pub fn char_key(c: char) -> Option<(u16, bool)> {
    const ROW1: &str = "1234567890";
    const SHIFT1: &str = "!@#$%^&*()";
    const LETTERS: [(char, u16); 26] = [('q', 16), ('w', 17), ('e', 18), ('r', 19), ('t', 20), ('y', 21), ('u', 22), ('i', 23), ('o', 24), ('p', 25),
        ('a', 30), ('s', 31), ('d', 32), ('f', 33), ('g', 34), ('h', 35), ('j', 36), ('k', 37), ('l', 38),
        ('z', 44), ('x', 45), ('c', 46), ('v', 47), ('b', 48), ('n', 49), ('m', 50)];
    if let Some(i) = ROW1.find(c) { return Some((2 + i as u16, false)); }
    if let Some(i) = SHIFT1.find(c) { return Some((2 + i as u16, true)); }
    let lower = c.to_ascii_lowercase();
    if let Some((_, k)) = LETTERS.iter().find(|(l, _)| *l == lower) { return Some((*k, c.is_ascii_uppercase())); }
    Some(match c {
        '-' => (12, false), '_' => (12, true), '=' => (13, false), '+' => (13, true),
        '[' => (26, false), '{' => (26, true), ']' => (27, false), '}' => (27, true),
        ';' => (39, false), ':' => (39, true), '\'' => (40, false), '"' => (40, true), '`' => (41, false), '~' => (41, true),
        '\\' => (43, false), '|' => (43, true), ',' => (51, false), '<' => (51, true), '.' => (52, false), '>' => (52, true),
        '/' => (53, false), '?' => (53, true), ' ' => (57, false), '\n' => (28, false), '\t' => (15, false),
        _ => return None,
    })
}

/// What one mousepad packet asks for, worked out (separate from the device, so it can be tested).
#[derive(Debug, PartialEq)]
pub enum Action {
    Move(f64, f64),
    Click(u16),
    DoubleClick,
    Hold(bool),
    Scroll(i32),
    Keys { mods: Vec<u16>, keys: Vec<(u16, bool)>, unicode: String },
    Nothing,
}

pub fn action(body: &Value) -> Action {
    let b = |k: &str| body.get(k).and_then(|v| v.as_bool()).unwrap_or(false);
    let f = |k: &str| body.get(k).and_then(|v| v.as_f64()).unwrap_or(0.0);
    if b("singleclick") { return Action::Click(BTN_LEFT); }
    if b("doubleclick") { return Action::DoubleClick; }
    if b("middleclick") { return Action::Click(BTN_MIDDLE); }
    if b("rightclick") { return Action::Click(BTN_RIGHT); }
    if b("singlehold") { return Action::Hold(true); }
    if b("singlerelease") { return Action::Hold(false); }
    if b("scroll") {
        let dy = f("dy");
        return if dy > 0.0 { Action::Scroll(1) } else if dy < 0.0 { Action::Scroll(-1) } else { Action::Nothing };
    }
    let key = body.get("key").and_then(|v| v.as_str()).unwrap_or("");
    let special = body.get("specialKey").and_then(|v| v.as_i64()).unwrap_or(0);
    if !key.is_empty() || special > 0 {
        let mut mods = vec![];
        if b("ctrl") { mods.push(KEY_LEFTCTRL); }
        if b("alt") { mods.push(KEY_LEFTALT); }
        if b("shift") { mods.push(KEY_LEFTSHIFT); }
        if b("super") { mods.push(KEY_LEFTMETA); }
        let mut keys = vec![];
        let mut unicode = String::new();
        if special > 0 && (special as usize) < SPECIAL_KEYS.len() { keys.push((SPECIAL_KEYS[special as usize], false)); }
        else {
            for c in key.chars() {
                match char_key(c) { Some(k) => keys.push(k), None => unicode.push(c) }
            }
        }
        return Action::Keys { mods, keys, unicode };
    }
    let (dx, dy) = (f("dx"), f("dy"));
    if dx != 0.0 || dy != 0.0 { Action::Move(dx, dy) } else { Action::Nothing }
}

#[cfg(target_os = "linux")]
mod uinput {
    use std::os::unix::io::RawFd;

    const UI_SET_EVBIT: u64 = 0x4004_5564;
    const UI_SET_KEYBIT: u64 = 0x4004_5565;
    const UI_SET_RELBIT: u64 = 0x4004_5566;
    const UI_DEV_SETUP: u64 = 0x405c_5503;
    const UI_DEV_CREATE: u64 = 0x5501;
    const UI_DEV_DESTROY: u64 = 0x5502;
    pub const EV_SYN: u16 = 0;
    pub const EV_KEY: u16 = 1;
    pub const EV_REL: u16 = 2;
    pub const REL_X: u16 = 0;
    pub const REL_Y: u16 = 1;
    pub const REL_HWHEEL: u16 = 6;
    pub const REL_WHEEL: u16 = 8;
    pub const REL_WHEEL_HI_RES: u16 = 11;

    #[repr(C)]
    struct InputId { bustype: u16, vendor: u16, product: u16, version: u16 }
    #[repr(C)]
    struct Setup { id: InputId, name: [u8; 80], ff_effects_max: u32 }
    #[repr(C)]
    struct Event { tv_sec: libc::time_t, tv_usec: libc::suseconds_t, kind: u16, code: u16, value: i32 }

    pub struct Device { fd: RawFd }

    impl Device {
        pub fn open(name: &str) -> Result<Device, String> {
            let path = std::ffi::CString::new("/dev/uinput").unwrap();
            let fd = unsafe { libc::open(path.as_ptr(), libc::O_WRONLY | libc::O_NONBLOCK | libc::O_CLOEXEC) };
            if fd < 0 { return Err(format!("Can't use /dev/uinput: {}", std::io::Error::last_os_error())); }
            let d = Device { fd };
            unsafe {
                let ok = |r: libc::c_int| if r < 0 { Err(std::io::Error::last_os_error().to_string()) } else { Ok(()) };
                ok(libc::ioctl(fd, UI_SET_EVBIT as _, EV_KEY as libc::c_int))?;
                ok(libc::ioctl(fd, UI_SET_EVBIT as _, EV_REL as libc::c_int))?;
                ok(libc::ioctl(fd, UI_SET_EVBIT as _, EV_SYN as libc::c_int))?;
                for k in 1..=248 { ok(libc::ioctl(fd, UI_SET_KEYBIT as _, k as libc::c_int))?; }
                for k in [super::BTN_LEFT, super::BTN_RIGHT, super::BTN_MIDDLE] { ok(libc::ioctl(fd, UI_SET_KEYBIT as _, k as libc::c_int))?; }
                for r in [REL_X, REL_Y, REL_WHEEL, REL_HWHEEL, REL_WHEEL_HI_RES] { ok(libc::ioctl(fd, UI_SET_RELBIT as _, r as libc::c_int))?; }
                let mut s = Setup { id: InputId { bustype: 0x06, vendor: 0x4865, product: 0x0001, version: 1 }, name: [0; 80], ff_effects_max: 0 };
                for (i, b) in name.bytes().take(79).enumerate() { s.name[i] = b; }
                ok(libc::ioctl(fd, UI_DEV_SETUP as _, &s as *const Setup))?;
                ok(libc::ioctl(fd, UI_DEV_CREATE as _))?;
            }
            // The compositor needs a moment to pick up a new device before its first events.
            std::thread::sleep(std::time::Duration::from_millis(250));
            Ok(d)
        }

        pub fn emit(&self, kind: u16, code: u16, value: i32) {
            let e = Event { tv_sec: 0, tv_usec: 0, kind, code, value };
            unsafe { libc::write(self.fd, &e as *const Event as *const libc::c_void, std::mem::size_of::<Event>()); }
        }
        pub fn sync(&self) { self.emit(EV_SYN, 0, 0); }
    }

    impl Drop for Device {
        fn drop(&mut self) { unsafe { libc::ioctl(self.fd, UI_DEV_DESTROY as _); libc::close(self.fd); } }
    }
}

/// The virtual mouse and keyboard (made the first time the phone moves the pointer or types).
pub struct Input {
    #[cfg(target_os = "linux")]
    dev: Option<uinput::Device>,
    rest: (f64, f64),
    pub error: Option<String>,
}

impl Default for Input { fn default() -> Self { Self::new() } }

impl Input {
    pub fn new() -> Input {
        Input { #[cfg(target_os = "linux")] dev: None, rest: (0.0, 0.0), error: None }
    }

    #[cfg(target_os = "linux")]
    fn dev(&mut self) -> Option<&uinput::Device> {
        if self.dev.is_none() {
            match uinput::Device::open("Hearth Connect phone touchpad") {
                Ok(d) => { self.dev = Some(d); self.error = None; }
                Err(e) => { self.error = Some(e); return None; }
            }
        }
        self.dev.as_ref()
    }

    pub fn ready(&self) -> bool {
        #[cfg(target_os = "linux")]
        { self.dev.is_some() }
        #[cfg(not(target_os = "linux"))]
        { false }
    }

    /// Does what the packet asks. Text the US layout can't type is returned, for typing another way (wtype).
    pub fn apply(&mut self, a: &Action) -> Result<String, String> {
        #[cfg(target_os = "linux")]
        {
            use uinput::*;
            if matches!(a, Action::Nothing) { return Ok(String::new()); }
            let mut rest = self.rest;
            let mut leftover = String::new();
            {
                if self.dev().is_none() { return Err(self.error.clone().unwrap_or_default()); }
                let Some(d) = self.dev.as_ref() else { return Ok(String::new()) };
                let tap = |code: u16| { d.emit(EV_KEY, code, 1); d.sync(); d.emit(EV_KEY, code, 0); d.sync(); };
                match a {
                    Action::Move(dx, dy) => {
                        rest.0 += dx; rest.1 += dy;
                        let (ix, iy) = (rest.0.trunc() as i32, rest.1.trunc() as i32);
                        rest.0 -= ix as f64; rest.1 -= iy as f64;
                        if ix != 0 { d.emit(EV_REL, REL_X, ix); }
                        if iy != 0 { d.emit(EV_REL, REL_Y, iy); }
                        d.sync();
                    }
                    Action::Click(b) => tap(*b),
                    Action::DoubleClick => { tap(BTN_LEFT); tap(BTN_LEFT); }
                    Action::Hold(down) => { d.emit(EV_KEY, BTN_LEFT, if *down { 1 } else { 0 }); d.sync(); }
                    Action::Scroll(n) => { d.emit(EV_REL, REL_WHEEL, *n); d.emit(EV_REL, REL_WHEEL_HI_RES, n * 120); d.sync(); }
                    Action::Keys { mods, keys, unicode } => {
                        for m in mods { d.emit(EV_KEY, *m, 1); }
                        if !mods.is_empty() { d.sync(); }
                        for (k, shift) in keys {
                            if *shift { d.emit(EV_KEY, KEY_LEFTSHIFT, 1); d.sync(); }
                            tap(*k);
                            if *shift { d.emit(EV_KEY, KEY_LEFTSHIFT, 0); d.sync(); }
                        }
                        for m in mods.iter().rev() { d.emit(EV_KEY, *m, 0); }
                        if !mods.is_empty() { d.sync(); }
                        leftover = unicode.clone();
                    }
                    Action::Nothing => {}
                }
            }
            self.rest = rest;
            Ok(leftover)
        }
        #[cfg(not(target_os = "linux"))]
        { let _ = a; Err("Remote input isn't available on this system yet".into()) }
    }

    /// A media key (the phone's media controls when no player answers over MPRIS).
    pub fn key(&mut self, code: u16) -> Result<(), String> {
        self.apply(&Action::Keys { mods: vec![], keys: vec![(code, false)], unicode: String::new() }).map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn mousepad_packets() {
        assert_eq!(action(&json!({"dx": 3.5, "dy": -1.0})), Action::Move(3.5, -1.0));
        assert_eq!(action(&json!({"singleclick": true})), Action::Click(BTN_LEFT));
        assert_eq!(action(&json!({"rightclick": true})), Action::Click(BTN_RIGHT));
        assert_eq!(action(&json!({"middleclick": true})), Action::Click(BTN_MIDDLE));
        assert_eq!(action(&json!({"doubleclick": true})), Action::DoubleClick);
        assert_eq!(action(&json!({"singlehold": true})), Action::Hold(true));
        assert_eq!(action(&json!({"singlerelease": true})), Action::Hold(false));
        assert_eq!(action(&json!({"scroll": true, "dx": 0.0, "dy": 4.0})), Action::Scroll(1));
        assert_eq!(action(&json!({"scroll": true, "dy": -2.0})), Action::Scroll(-1));
        assert_eq!(action(&json!({"specialKey": 12})), Action::Keys { mods: vec![], keys: vec![(28, false)], unicode: String::new() });
        assert_eq!(action(&json!({"specialKey": 24, "alt": true})), Action::Keys { mods: vec![KEY_LEFTALT], keys: vec![(62, false)], unicode: String::new() });
        assert_eq!(action(&json!({"key": "Hi!é"})), Action::Keys { mods: vec![], keys: vec![(35, true), (23, false), (2, true)], unicode: "é".into() });
        assert_eq!(action(&json!({"key": "c", "ctrl": true})), Action::Keys { mods: vec![KEY_LEFTCTRL], keys: vec![(46, false)], unicode: String::new() });
        assert_eq!(action(&json!({})), Action::Nothing);
    }

    #[test]
    fn us_layout() {
        assert_eq!(char_key('a'), Some((30, false)));
        assert_eq!(char_key('Z'), Some((44, true)));
        assert_eq!(char_key('0'), Some((11, false)));
        assert_eq!(char_key(')'), Some((11, true)));
        assert_eq!(char_key('?'), Some((53, true)));
        assert_eq!(char_key('ß'), None);
        assert_eq!(SPECIAL_KEYS[32], 88);
        assert_eq!(SPECIAL_KEYS[14], 1);
    }
}
