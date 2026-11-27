//! What libratbag can't set yet, written by Hearth itself to the mice whose own protocol is known (board 11, Reports
//! per second: Sleep after). Through the mouse's hidraw device, with the access the person at the PC gets from Hearth
//! Mouse's udev rule (udev.rs; only for these models), never as root:
//!   * Razer's wireless mice: the idle time before the mouse sleeps (its 90-byte feature report, command class 0x07,
//!     0x03 to set, 0x83 to read back; 60–900 s), as OpenRazer sends it. Read back after setting, so Hearth only
//!     says it's set when the mouse says so (the report's transaction id differs between models: 0x1F, then 0x3F,
//!     then 0xFF are tried);
//!   * SteelSeries Aerox 3 Wireless (wired): its sleep timer (output report 0x29 and the minutes in ms, then 0x11 0x00
//!     saves it on the mouse), as rivalcfg sends it.
//! Lift-off distance: no maker's protocol for it is known well enough to write (Razer's command for it isn't
//! described anywhere, rivalcfg and libratbag don't have it), so the row says the mouse doesn't have it from Linux.

use std::io::Write;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kind {
    Razer,
    SteelAerox,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Known {
    pub vid: u16,
    pub pid: u16,
    pub name: &'static str,
    pub kind: Kind,
}

pub const KNOWN: [Known; 15] = [
    Known { vid: 0x1532, pid: 0x007A, name: "Razer Viper Ultimate (cable)", kind: Kind::Razer },
    Known { vid: 0x1532, pid: 0x007B, name: "Razer Viper Ultimate", kind: Kind::Razer },
    Known { vid: 0x1532, pid: 0x007C, name: "Razer DeathAdder V2 Pro (cable)", kind: Kind::Razer },
    Known { vid: 0x1532, pid: 0x007D, name: "Razer DeathAdder V2 Pro", kind: Kind::Razer },
    Known { vid: 0x1532, pid: 0x0086, name: "Razer Basilisk Ultimate (cable)", kind: Kind::Razer },
    Known { vid: 0x1532, pid: 0x0088, name: "Razer Basilisk Ultimate", kind: Kind::Razer },
    Known { vid: 0x1532, pid: 0x008F, name: "Razer Naga Pro (cable)", kind: Kind::Razer },
    Known { vid: 0x1532, pid: 0x0090, name: "Razer Naga Pro", kind: Kind::Razer },
    Known { vid: 0x1532, pid: 0x00A5, name: "Razer Viper V2 Pro (cable)", kind: Kind::Razer },
    Known { vid: 0x1532, pid: 0x00A6, name: "Razer Viper V2 Pro", kind: Kind::Razer },
    Known { vid: 0x1532, pid: 0x00AA, name: "Razer Basilisk V3 Pro (cable)", kind: Kind::Razer },
    Known { vid: 0x1532, pid: 0x00AB, name: "Razer Basilisk V3 Pro", kind: Kind::Razer },
    Known { vid: 0x1532, pid: 0x00B6, name: "Razer DeathAdder V3 Pro (cable)", kind: Kind::Razer },
    Known { vid: 0x1532, pid: 0x00B7, name: "Razer DeathAdder V3 Pro", kind: Kind::Razer },
    Known { vid: 0x1038, pid: 0x183A, name: "SteelSeries Aerox 3 Wireless (cable)", kind: Kind::SteelAerox },
];

/// The sleep times the board's ▾ offers for this kind of mouse (minutes).
pub fn choices(k: Kind) -> &'static [u32] {
    match k {
        Kind::Razer => &[1, 2, 3, 5, 10, 15],
        Kind::SteelAerox => &[1, 2, 5, 10, 15, 20],
    }
}

/// ratbagd's model ("usb:1532:007c:0") → vendor and product.
pub fn ids(model: &str) -> Option<(u16, u16)> {
    let mut p = model.split(':');
    if p.next()? != "usb" {
        return None;
    }
    Some((u16::from_str_radix(p.next()?, 16).ok()?, u16::from_str_radix(p.next()?, 16).ok()?))
}

pub fn known(model: &str) -> Option<Known> {
    // CI's pictures: $HEARTH_MOUSE_HID names a model as known
    let (v, p) = std::env::var("HEARTH_MOUSE_HID_AS").ok().and_then(|m| ids(&m)).or_else(|| ids(model))?;
    KNOWN.iter().copied().find(|k| k.vid == v && k.pid == p)
}

/// The mouse's hidraw device on USB interface `iface`.
pub fn hidraw(k: &Known, iface: u8) -> Option<PathBuf> {
    let want = format!("HID_ID=0003:{:08X}:{:08X}", k.vid, k.pid);
    for e in std::fs::read_dir("/sys/class/hidraw").ok()?.flatten() {
        let dev = e.path().join("device");
        let ue = std::fs::read_to_string(dev.join("uevent")).unwrap_or_default();
        if !ue.lines().any(|l| l.eq_ignore_ascii_case(&want)) {
            continue;
        }
        let real = std::fs::canonicalize(&dev).ok()?;
        let n = std::fs::read_to_string(real.parent()?.join("bInterfaceNumber")).ok().and_then(|s| u8::from_str_radix(s.trim(), 16).ok());
        if n == Some(iface) {
            return Some(PathBuf::from("/dev").join(e.file_name()));
        }
    }
    None
}

/// A Razer report (90 bytes), its checksum filled in.
pub fn razer_report(tid: u8, class: u8, id: u8, args: &[u8]) -> [u8; 90] {
    let mut r = [0u8; 90];
    r[1] = tid;
    r[5] = args.len() as u8;
    r[6] = class;
    r[7] = id;
    r[8..8 + args.len()].copy_from_slice(args);
    r[88] = r[2..88].iter().fold(0, |a, b| a ^ b);
    r
}

/// SteelSeries Aerox 3's sleep timer report (its minutes in ms, 3 bytes, low byte first).
pub fn aerox_sleep(minutes: u32) -> Vec<u8> {
    let ms = minutes.min(20) * 60_000;
    vec![0x00, 0x29, (ms & 0xff) as u8, ((ms >> 8) & 0xff) as u8, ((ms >> 16) & 0xff) as u8]
}

extern "C" {
    fn ioctl(fd: i32, req: u64, ...) -> i32;
}
fn ioc_feature(set: bool, len: usize) -> u64 {
    (3u64 << 30) | ((len as u64) << 16) | (0x48 << 8) | if set { 0x06 } else { 0x07 }
}

fn razer_call(dev: &PathBuf, tid: u8, class: u8, id: u8, args: &[u8]) -> Result<[u8; 90], String> {
    use std::os::unix::io::AsRawFd;
    let f = std::fs::OpenOptions::new().read(true).write(true).open(dev).map_err(|e| if e.kind() == std::io::ErrorKind::PermissionDenied { "no access to the mouse yet (Let Hearth Mouse set it)".to_string() } else { e.to_string() })?;
    let mut buf = [0u8; 91];
    buf[1..].copy_from_slice(&razer_report(tid, class, id, args));
    if unsafe { ioctl(f.as_raw_fd(), ioc_feature(true, 91), buf.as_mut_ptr()) } < 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    std::thread::sleep(std::time::Duration::from_millis(60));
    let mut back = [0u8; 91];
    if unsafe { ioctl(f.as_raw_fd(), ioc_feature(false, 91), back.as_mut_ptr()) } < 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    let mut r = [0u8; 90];
    r.copy_from_slice(&back[1..]);
    if r[0] != 0x02 || r[6] != class || r[7] != id {
        return Err(format!("the mouse said {:#04x}", r[0]));
    }
    Ok(r)
}

/// The mouse's own sleep time now (minutes), where it can say (Razer).
pub fn sleep_now(k: &Known) -> Option<u32> {
    if k.kind != Kind::Razer {
        return None;
    }
    let dev = hidraw(k, 0)?;
    for tid in [0x1F, 0x3F, 0xFF] {
        if let Ok(r) = razer_call(&dev, tid, 0x07, 0x83, &[0, 0]) {
            return Some(((u32::from(r[8]) << 8 | u32::from(r[9])) + 30) / 60);
        }
    }
    None
}

/// Sets the time before the mouse sleeps; Ok only when the mouse took it.
pub fn set_sleep(k: &Known, minutes: u32) -> Result<(), String> {
    match k.kind {
        Kind::Razer => {
            let dev = hidraw(k, 0).ok_or("the mouse isn't plugged in")?;
            let s = (minutes * 60).clamp(60, 900);
            let mut last = String::new();
            for tid in [0x1F, 0x3F, 0xFF] {
                match razer_call(&dev, tid, 0x07, 0x03, &[(s >> 8) as u8, (s & 0xff) as u8]) {
                    Ok(_) => {
                        let back = razer_call(&dev, tid, 0x07, 0x83, &[0, 0])?;
                        let got = u32::from(back[8]) << 8 | u32::from(back[9]);
                        return if got == s { Ok(()) } else { Err(format!("the mouse kept {got} s")) };
                    }
                    Err(e) => last = e,
                }
            }
            Err(last)
        }
        Kind::SteelAerox => {
            let dev = hidraw(k, 3).ok_or("the mouse isn't plugged in by its cable")?;
            let mut f = std::fs::OpenOptions::new().write(true).open(&dev).map_err(|e| if e.kind() == std::io::ErrorKind::PermissionDenied { "no access to the mouse yet (Let Hearth Mouse set it)".to_string() } else { e.to_string() })?;
            f.write_all(&aerox_sleep(minutes)).map_err(|e| e.to_string())?;
            std::thread::sleep(std::time::Duration::from_millis(40));
            f.write_all(&[0x00, 0x11, 0x00]).map_err(|e| e.to_string())?;
            Ok(())
        }
    }
}

/// Hearth Mouse's udev rule: these models' hidraw devices for the person at the PC (uaccess), nothing else.
pub fn udev_rule() -> String {
    let mut s = String::from("# Hearth Mouse (hosts/hearth-ui/apps/mouse, hid.rs): the person at the PC may set the sleep time of these mice\n");
    for k in KNOWN {
        s.push_str(&format!("SUBSYSTEM==\"hidraw\", ATTRS{{idVendor}}==\"{:04x}\", ATTRS{{idProduct}}==\"{:04x}\", TAG+=\"uaccess\"\n", k.vid, k.pid));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn the_mices_own_words() {
        assert_eq!(ids("usb:1532:007c:0"), Some((0x1532, 0x007C)));
        let r = razer_report(0x1F, 0x07, 0x03, &[0x01, 0x2C]);
        assert_eq!((r[1], r[5], r[6], r[7], r[8], r[9]), (0x1F, 2, 0x07, 0x03, 0x01, 0x2C));
        assert_eq!(r[88], 2 ^ 0x07 ^ 0x03 ^ 0x01 ^ 0x2C);
        assert_eq!(aerox_sleep(5), vec![0x00, 0x29, 0xE0, 0x93, 0x04]);
        assert_eq!(ioc_feature(true, 91), 0xC05B4806);
        assert!(udev_rule().contains("ATTRS{idProduct}==\"183a\""));
    }
}
