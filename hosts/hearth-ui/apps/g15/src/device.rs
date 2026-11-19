//! Finding the keyboard's parts in sysfs. The first G15 shows up as three USB devices: 046d:c223 (the hub inside
//! it), 046d:c221 (the ordinary keys) and 046d:c222 (the G-keys, the keys under the screen and the screen itself).
//! hid-lg-g15 binds the c222 part: it makes the "Logitech Gaming Keyboard Gaming Keys" input device and the g15::*
//! LEDs, and leaves its hidraw node for the screen. The G15 v2 (c227, 6 G-keys), the G11 (c225, no screen) and the
//! G510 (c22d / c22e) use the same driver and are found the same way.

use crate::root::{read, Root};
use std::path::{Path, PathBuf};

/// The name hid-lg-g15 gives the G-keys' input device.
pub const GAMING_KEYS: &str = "Logitech Gaming Keyboard Gaming Keys";
pub const VENDOR: u32 = 0x046d;
/// The ordinary keys' USB ids (read only while you record a macro with MR).
pub const KEYBOARD_PARTS: &[u32] = &[0xc221, 0xc226];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Model {
    pub product: u32,
    pub name: &'static str,
    pub gkeys: u8,
    pub lcd: bool,
}

pub fn model_of(product: u32) -> Option<Model> {
    Some(match product {
        0xc222 => Model { product, name: "G15", gkeys: 18, lcd: true },
        0xc227 => Model { product, name: "G15 v2", gkeys: 6, lcd: true },
        0xc225 => Model { product, name: "G11", gkeys: 18, lcd: false },
        0xc22d | 0xc22e => Model { product, name: "G510", gkeys: 18, lcd: true },
        _ => return None,
    })
}

#[derive(Debug, Clone, Default)]
pub struct Leds {
    /// The folders in /sys/class/leds.
    pub kbd: Option<PathBuf>,
    pub lcd: Option<PathBuf>,
    pub m: [Option<PathBuf>; 3],
    pub mr: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct Device {
    pub model: Model,
    /// The hid device in sysfs (what the parts below hang off).
    pub hid: Option<PathBuf>,
    /// /dev/hidrawN: the screen.
    pub hidraw: Option<PathBuf>,
    /// /dev/input/eventN: the G-keys, M-keys, MR and the keys under the screen.
    pub keys: Option<PathBuf>,
    pub leds: Leds,
}

fn canon(p: &Path) -> Option<PathBuf> {
    std::fs::canonicalize(p).ok()
}

fn hex(s: &str) -> Option<u32> {
    u32::from_str_radix(s.trim().trim_start_matches("0x"), 16).ok()
}

/// HID_ID=0003:0000046D:0000C222 → (vendor, product).
pub fn hid_id(uevent: &str) -> Option<(u32, u32)> {
    let line = uevent.lines().find_map(|l| l.strip_prefix("HID_ID="))?;
    let mut it = line.split(':');
    let _bus = it.next()?;
    Some((hex(it.next()?)?, hex(it.next()?)?))
}

fn list(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir).map(|r| r.filter_map(|e| e.ok().map(|e| e.path())).collect()).unwrap_or_default();
    v.sort();
    v
}

fn name_of(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()
}

/// The keyboard, if one is plugged in.
pub fn find(root: &Root) -> Option<Device> {
    // The screen's hidraw node, and with it the hid device and the model.
    let mut found: Option<(Model, PathBuf, PathBuf)> = None;
    for h in list(&root.sys.join("class/hidraw")) {
        let Some(ue) = read(&h.join("device/uevent")) else { continue };
        let Some((v, p)) = hid_id(&ue) else { continue };
        if v != VENDOR {
            continue;
        }
        if let Some(m) = model_of(p) {
            if let Some(hid) = canon(&h.join("device")) {
                found = Some((m, hid, root.dev.join(name_of(&h))));
                break;
            }
        }
    }
    let hid = found.as_ref().map(|f| f.1.clone());
    // The G-keys' input device (on the same hid device when the hidraw node was found).
    let mut keys = None;
    let mut keys_product = None;
    for e in list(&root.sys.join("class/input")) {
        let n = name_of(&e);
        if !n.starts_with("event") {
            continue;
        }
        if read(&e.join("device/name")).as_deref() != Some(GAMING_KEYS) {
            continue;
        }
        if let Some(h) = &hid {
            if canon(&e.join("device/device")).as_ref() != Some(h) {
                continue;
            }
        }
        keys_product = read(&e.join("device/id/product")).and_then(|s| hex(&s));
        keys = Some(root.dev.join("input").join(n));
        break;
    }
    let model = match (&found, keys_product.and_then(model_of)) {
        (Some(f), _) => f.0,
        (None, Some(m)) => m,
        (None, None) if keys.is_some() => model_of(0xc222).unwrap(),
        _ => return None,
    };
    // The lights: g15::* on the same hid device.
    let mut leds = Leds::default();
    for l in list(&root.sys.join("class/leds")) {
        let n = name_of(&l);
        if !n.starts_with("g15::") {
            continue;
        }
        if let Some(h) = &hid {
            if let Some(d) = canon(&l.join("device")) {
                if &d != h {
                    continue;
                }
            }
        }
        // A second G15 gets "_1" on its names.
        let base = n.split('_').collect::<Vec<_>>();
        let key = if n.contains("kbd_backlight") {
            "kbd"
        } else if n.contains("lcd_backlight") {
            "lcd"
        } else if n.contains("macro_preset1") {
            "m1"
        } else if n.contains("macro_preset2") {
            "m2"
        } else if n.contains("macro_preset3") {
            "m3"
        } else if n.contains("macro_record") {
            "mr"
        } else {
            continue;
        };
        let _ = base;
        let slot = match key {
            "kbd" => &mut leds.kbd,
            "lcd" => &mut leds.lcd,
            "m1" => &mut leds.m[0],
            "m2" => &mut leds.m[1],
            "m3" => &mut leds.m[2],
            _ => &mut leds.mr,
        };
        if slot.is_none() {
            *slot = Some(l);
        }
    }
    let hidraw = found.as_ref().filter(|f| f.0.lcd).map(|f| f.2.clone());
    Some(Device { model, hid, hidraw, keys, leds })
}

/// The ordinary keys' input devices (046d:c221 on the first G15): read only while a macro is being recorded.
pub fn keyboard_events(root: &Root) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for e in list(&root.sys.join("class/input")) {
        let n = name_of(&e);
        if !n.starts_with("event") {
            continue;
        }
        let v = read(&e.join("device/id/vendor")).and_then(|s| hex(&s));
        let p = read(&e.join("device/id/product")).and_then(|s| hex(&s));
        if v == Some(VENDOR) && p.map(|p| KEYBOARD_PARTS.contains(&p)).unwrap_or(false) {
            out.push(root.dev.join("input").join(n));
        }
    }
    out
}

pub fn writable(p: &Path) -> bool {
    use std::ffi::CString;
    let Ok(c) = CString::new(p.as_os_str().as_encoded_bytes()) else { return false };
    unsafe { libc::access(c.as_ptr(), libc::W_OK) == 0 }
}

pub fn readable(p: &Path) -> bool {
    use std::ffi::CString;
    let Ok(c) = CString::new(p.as_os_str().as_encoded_bytes()) else { return false };
    unsafe { libc::access(c.as_ptr(), libc::R_OK) == 0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reads_the_hid_id() {
        assert_eq!(hid_id("DRIVER=lg-g15\nHID_ID=0003:0000046D:0000C222\nHID_NAME=G15 GamePanel LCD\n"), Some((0x046d, 0xc222)));
        assert_eq!(model_of(0xc222).unwrap().gkeys, 18);
        assert!(!model_of(0xc225).unwrap().lcd);
        assert!(model_of(0xc221).is_none(), "c221 is the ordinary keys, not the G-key part");
    }

    #[test]
    fn finds_the_fixture_keyboard() {
        let d = tempfile::tempdir().unwrap();
        crate::service::make_fixture(d.path()).unwrap();
        let r = Root::at(d.path());
        let dev = find(&r).expect("found");
        assert_eq!(dev.model.name, "G15");
        assert!(dev.hidraw.as_ref().unwrap().ends_with("dev/hidraw3"));
        assert!(dev.keys.as_ref().unwrap().ends_with("dev/input/event5"));
        assert!(dev.leds.kbd.is_some() && dev.leds.lcd.is_some() && dev.leds.mr.is_some());
        assert!(dev.leds.m.iter().all(|m| m.is_some()));
        assert_eq!(keyboard_events(&r).len(), 1);
    }
}
