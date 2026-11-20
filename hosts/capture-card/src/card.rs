//! The capture cards plugged in: every V4L2 device that captures video (UVC: /sys/class/video4linux), with what the
//! settings and the no-signal page show: its name ("Monster 4K"), its chip ("USB3 Video (MS2130)"), the USB port's
//! speed (USB 3.0 or 2.0) and its picture modes (MJPEG and YUYV sizes and frame rates, from the card itself through
//! VIDIOC_ENUM_FMT / FRAMESIZES / FRAMEINTERVALS). Opening a device to ask doesn't start it streaming.
//!
//! CI's stand-in card (HEARTH_CAPTURE_FAKE_CARD, a JSON file): the same Card, read from that file.

use serde_json::Value;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub struct Mode {
    /// "MJPG" or "YUYV".
    pub format: String,
    pub w: u32,
    pub h: u32,
    /// Frames a second, highest first.
    pub fps: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Card {
    /// /dev/videoN.
    pub dev: PathBuf,
    /// The same card next time (USB vendor:product, and its serial when it has one).
    pub id: String,
    pub name: String,
    /// "USB3 Video (MS2130)".
    pub chip: String,
    /// The USB port's speed in Mbit/s (5000 and up: USB 3).
    pub speed: u32,
    /// The USB device's place in /sys ("3-1"): its microphone sits under the same place.
    pub usb_port: String,
    pub modes: Vec<Mode>,
    /// CI's stand-in only: the JPEG it sends as its picture.
    pub fake_picture: Option<PathBuf>,
    /// CI's stand-in only: the PipeWire source it says is its sound.
    pub fake_sound: Option<String>,
}

impl Card {
    pub fn usb3(&self) -> bool {
        self.speed >= 5000
    }
    pub fn usb_words(&self) -> &'static str {
        if self.speed >= 5000 {
            "USB 3.0"
        } else if self.speed > 0 {
            "USB 2.0"
        } else {
            "USB"
        }
    }
    pub fn formats(&self) -> Vec<String> {
        let mut v: Vec<String> = Vec::new();
        for m in &self.modes {
            if !v.contains(&m.format) {
                v.push(m.format.clone());
            }
        }
        v
    }
    /// The sizes this card sends in a format, largest first.
    pub fn sizes(&self, format: &str) -> Vec<(u32, u32)> {
        let mut v: Vec<(u32, u32)> = self.modes.iter().filter(|m| m.format == format).map(|m| (m.w, m.h)).collect();
        v.sort_by(|a, b| (b.0 * b.1).cmp(&(a.0 * a.1)));
        v.dedup();
        v
    }
    pub fn rates(&self, format: &str, w: u32, h: u32) -> Vec<u32> {
        self.modes.iter().find(|m| m.format == format && m.w == w && m.h == h).map(|m| m.fps.clone()).unwrap_or_default()
    }
    /// The mode to ask for: what you chose when the card has it, else the nearest it has (the same format first,
    /// then the size, then the rate).
    pub fn pick(&self, format: &str, w: u32, h: u32, fps: u32) -> Option<(String, u32, u32, u32)> {
        let fmt = if self.modes.iter().any(|m| m.format == format) { format.to_string() } else { self.modes.first()?.format.clone() };
        let sizes = self.sizes(&fmt);
        let (sw, sh) = if sizes.contains(&(w, h)) { (w, h) } else { *sizes.iter().min_by_key(|(a, b)| ((*a as i64 * *b as i64) - (w as i64 * h as i64)).abs())? };
        let rates = self.rates(&fmt, sw, sh);
        let r = if rates.contains(&fps) { fps } else { *rates.iter().min_by_key(|r| (**r as i64 - fps as i64).abs()).unwrap_or(&30) };
        Some((fmt, sw, sh, r))
    }
    /// The lightest mode it has (the no-signal page waits for a picture with it: a few small frames a second).
    pub fn lightest(&self) -> Option<(String, u32, u32, u32)> {
        let mj = self.modes.iter().any(|m| m.format == "MJPG");
        let m = self.modes.iter().filter(|m| !mj || m.format == "MJPG").min_by_key(|m| (m.w * m.h, m.fps.iter().min().copied().unwrap_or(60)))?;
        Some((m.format.clone(), m.w, m.h, m.fps.iter().min().copied().unwrap_or(30)))
    }
}

/// The stand-in card's file, when CI set one.
pub fn fake_file() -> Option<PathBuf> {
    std::env::var_os("HEARTH_CAPTURE_FAKE_CARD").filter(|s| !s.is_empty()).map(PathBuf::from)
}

/// Every capture card plugged in now.
pub fn list() -> Vec<Card> {
    if let Some(f) = fake_file() {
        return read_fake(&f).into_iter().collect();
    }
    let mut out = Vec::new();
    let Ok(dir) = std::fs::read_dir("/sys/class/video4linux") else { return out };
    let mut names: Vec<String> = dir.flatten().map(|e| e.file_name().to_string_lossy().to_string()).filter(|n| n.starts_with("video")).collect();
    names.sort_by_key(|n| n[5..].parse::<u32>().unwrap_or(999));
    for n in names {
        if let Some(c) = read_one(&n) {
            out.push(c);
        }
    }
    out
}

pub fn read_fake(f: &Path) -> Option<Card> {
    let v: Value = serde_json::from_slice(&std::fs::read(f).ok()?).ok()?;
    let modes = v["modes"]
        .as_array()?
        .iter()
        .filter_map(|m| Some(Mode { format: m["format"].as_str()?.to_string(), w: m["w"].as_u64()? as u32, h: m["h"].as_u64()? as u32, fps: m["fps"].as_array()?.iter().filter_map(|x| x.as_u64().map(|x| x as u32)).collect() }))
        .collect();
    Some(Card {
        dev: PathBuf::from("/dev/video-stand-in"),
        id: v["id"].as_str().unwrap_or("stand-in").to_string(),
        name: v["name"].as_str().unwrap_or("Capture card").to_string(),
        chip: v["chip"].as_str().unwrap_or("").to_string(),
        speed: v["speed"].as_u64().unwrap_or(5000) as u32,
        usb_port: "stand-in".into(),
        modes,
        fake_picture: v["picture"].as_str().map(PathBuf::from),
        fake_sound: v["sound"].as_str().map(String::from),
    })
}

fn rd(p: &Path) -> String {
    std::fs::read_to_string(p).map(|s| s.trim().to_string()).unwrap_or_default()
}

/// Chips by their USB ids (vendor:product), for the "USB3 Video (MS2130)" line.
pub fn chip_of(vid: &str, pid: &str) -> Option<&'static str> {
    Some(match (vid, pid) {
        ("345f", "2130") | ("345f", "2131") => "MS2130",
        ("534d", "2109") => "MS2109",
        ("534d", "0021") => "MS2106",
        ("1e4e", "7103") => "MS2130",
        ("eba4", "7588") => "MS2131",
        _ => return None,
    })
}

/// A friendly name: the USB product's, unless it's a generic one and the maker has a better one.
pub fn friendly(product: &str, maker: &str, card_line: &str) -> String {
    let generic = |s: &str| {
        let l = s.to_lowercase();
        l.is_empty() || l == "usb video" || l == "usb3 video" || l == "usb3.0 video" || l == "usb2.0 video" || l.contains("uvc camera") || l == "unknown"
    };
    if !generic(product) {
        return product.to_string();
    }
    if !generic(maker) && !maker.to_lowercase().contains("macrosilicon") {
        return maker.to_string();
    }
    let c = card_line.split(':').next().unwrap_or("").trim();
    if !c.is_empty() {
        return c.to_string();
    }
    if product.is_empty() { "Capture card".into() } else { product.to_string() }
}

/// The USB device's folder for a /sys/class/video4linux/videoN node, and its port name ("3-1").
fn usb_dev(node: &str) -> Option<(PathBuf, String)> {
    let dev = std::fs::canonicalize(format!("/sys/class/video4linux/{node}/device")).ok()?;
    // device -> .../usb3/3-1/3-1:1.0 (the interface); its parent is the USB device.
    let parent = dev.parent()?.to_path_buf();
    let port = parent.file_name()?.to_string_lossy().to_string();
    parent.join("idVendor").exists().then_some((parent, port))
}

fn read_one(node: &str) -> Option<Card> {
    let devp = PathBuf::from(format!("/dev/{node}"));
    let (caps, card_line, modes) = query(&devp)?;
    // Only nodes that capture video (UVC makes a second, metadata node for each card).
    const CAP_VIDEO_CAPTURE: u32 = 0x1;
    if caps & CAP_VIDEO_CAPTURE == 0 || modes.is_empty() {
        return None;
    }
    let (usb, port) = usb_dev(node).unwrap_or((PathBuf::new(), String::new()));
    let vid = rd(&usb.join("idVendor"));
    let pid = rd(&usb.join("idProduct"));
    let product = rd(&usb.join("product"));
    let maker = rd(&usb.join("manufacturer"));
    let serial = rd(&usb.join("serial"));
    let speed = rd(&usb.join("speed")).parse::<f32>().unwrap_or(0.0) as u32;
    let base = if product.is_empty() { card_line.split(':').next().unwrap_or("").trim().to_string() } else { product.clone() };
    let chip = match chip_of(&vid, &pid) {
        Some(c) => format!("{base} ({c})"),
        None => base.clone(),
    };
    Some(Card {
        dev: devp,
        id: if serial.is_empty() { format!("{vid}:{pid}") } else { format!("{vid}:{pid}:{serial}") },
        name: friendly(&product, &maker, &card_line),
        chip,
        speed,
        usb_port: port,
        modes,
        fake_picture: None,
        fake_sound: None,
    })
}

const fn iowr(nr: u32, size: u32) -> u64 {
    ((3u32 << 30) | (size << 16) | ((b'V' as u32) << 8) | nr) as u64
}
const fn ior(nr: u32, size: u32) -> u64 {
    ((2u32 << 30) | (size << 16) | ((b'V' as u32) << 8) | nr) as u64
}
pub const VIDIOC_QUERYCAP: u64 = ior(0, 104);
pub const VIDIOC_ENUM_FMT: u64 = iowr(2, 64);
pub const VIDIOC_ENUM_FRAMESIZES: u64 = iowr(74, 44);
pub const VIDIOC_ENUM_FRAMEINTERVALS: u64 = iowr(75, 52);

fn fourcc(s: &[u8; 4]) -> u32 {
    u32::from_le_bytes(*s)
}

fn u32at(b: &[u8], off: usize) -> u32 {
    u32::from_ne_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]])
}

fn put(b: &mut [u8], off: usize, v: u32) {
    b[off..off + 4].copy_from_slice(&v.to_ne_bytes());
}

fn cstr(b: &[u8]) -> String {
    String::from_utf8_lossy(&b[..b.iter().position(|x| *x == 0).unwrap_or(b.len())]).trim().to_string()
}

/// Asks the device itself: its capabilities, its card line, and its MJPEG and YUYV modes.
fn query(dev: &Path) -> Option<(u32, String, Vec<Mode>)> {
    let c = std::ffi::CString::new(dev.as_os_str().as_encoded_bytes()).ok()?;
    // SAFETY: plain ioctls on a descriptor this function owns, each into a buffer of the size its number says.
    unsafe {
        let fd = libc::open(c.as_ptr(), libc::O_RDONLY | libc::O_NONBLOCK | libc::O_CLOEXEC);
        if fd < 0 {
            return None;
        }
        let mut cap = [0u8; 104];
        if libc::ioctl(fd, VIDIOC_QUERYCAP as _, cap.as_mut_ptr()) != 0 {
            libc::close(fd);
            return None;
        }
        let card_line = cstr(&cap[16..48]);
        let all = u32at(&cap, 84);
        let devcaps = u32at(&cap, 88);
        // V4L2_CAP_DEVICE_CAPS: device_caps says what this node does.
        let caps = if all & 0x8000_0000 != 0 { devcaps } else { all };
        let mut modes = Vec::new();
        let wanted = [(fourcc(b"MJPG"), "MJPG"), (fourcc(b"YUYV"), "YUYV")];
        for i in 0..32u32 {
            let mut f = [0u8; 64];
            put(&mut f, 0, i);
            put(&mut f, 4, 1); // V4L2_BUF_TYPE_VIDEO_CAPTURE
            if libc::ioctl(fd, VIDIOC_ENUM_FMT as _, f.as_mut_ptr()) != 0 {
                break;
            }
            let pf = u32at(&f, 44);
            let Some((_, name)) = wanted.iter().find(|(c, _)| *c == pf) else { continue };
            for j in 0..64u32 {
                let mut s = [0u8; 44];
                put(&mut s, 0, j);
                put(&mut s, 4, pf);
                if libc::ioctl(fd, VIDIOC_ENUM_FRAMESIZES as _, s.as_mut_ptr()) != 0 {
                    break;
                }
                if u32at(&s, 8) != 1 {
                    break; // only discrete sizes (UVC cards list them)
                }
                let (w, h) = (u32at(&s, 12), u32at(&s, 16));
                let mut fps = Vec::new();
                for k in 0..32u32 {
                    let mut iv = [0u8; 52];
                    put(&mut iv, 0, k);
                    put(&mut iv, 4, pf);
                    put(&mut iv, 8, w);
                    put(&mut iv, 12, h);
                    if libc::ioctl(fd, VIDIOC_ENUM_FRAMEINTERVALS as _, iv.as_mut_ptr()) != 0 || u32at(&iv, 16) != 1 {
                        break;
                    }
                    let (num, den) = (u32at(&iv, 20), u32at(&iv, 24));
                    if num > 0 {
                        let r = ((den as f32 / num as f32).round()) as u32;
                        if r > 0 && !fps.contains(&r) {
                            fps.push(r);
                        }
                    }
                }
                fps.sort_by(|a, b| b.cmp(a));
                if !fps.is_empty() {
                    modes.push(Mode { format: name.to_string(), w, h, fps });
                }
            }
        }
        libc::close(fd);
        Some((caps, card_line, modes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card() -> Card {
        Card {
            dev: "/dev/video0".into(),
            id: "345f:2130".into(),
            name: "Monster 4K".into(),
            chip: "USB3 Video (MS2130)".into(),
            speed: 5000,
            usb_port: "3-1".into(),
            modes: vec![
                Mode { format: "MJPG".into(), w: 1920, h: 1080, fps: vec![60, 50, 30] },
                Mode { format: "MJPG".into(), w: 1280, h: 720, fps: vec![60, 30] },
                Mode { format: "MJPG".into(), w: 640, h: 480, fps: vec![60, 30, 10, 5] },
                Mode { format: "YUYV".into(), w: 1920, h: 1080, fps: vec![30, 5] },
            ],
            fake_picture: None,
            fake_sound: None,
        }
    }

    #[test]
    fn the_ioctl_numbers_are_v4l2s() {
        assert_eq!(VIDIOC_QUERYCAP, 0x8068_5600);
        assert_eq!(VIDIOC_ENUM_FMT, 0xc040_5602);
        assert_eq!(VIDIOC_ENUM_FRAMESIZES, 0xc02c_564a);
        assert_eq!(VIDIOC_ENUM_FRAMEINTERVALS, 0xc034_564b);
        assert_eq!(fourcc(b"MJPG"), 0x4750_4a4d);
    }

    #[test]
    fn picking_a_mode() {
        let c = card();
        assert_eq!(c.pick("MJPG", 1920, 1080, 60), Some(("MJPG".into(), 1920, 1080, 60)));
        // YUYV 1080p only goes to 30: the nearest rate.
        assert_eq!(c.pick("YUYV", 1920, 1080, 60), Some(("YUYV".into(), 1920, 1080, 30)));
        // A size it hasn't: the nearest.
        assert_eq!(c.pick("MJPG", 2560, 1440, 60).map(|m| (m.1, m.2)), Some((1920, 1080)));
        assert_eq!(c.lightest(), Some(("MJPG".into(), 640, 480, 5)));
        assert_eq!(c.sizes("MJPG")[0], (1920, 1080));
        assert_eq!(c.formats(), vec!["MJPG".to_string(), "YUYV".to_string()]);
        assert!(c.usb3());
        assert_eq!(Card { speed: 480, ..c }.usb_words(), "USB 2.0");
    }

    #[test]
    fn names_and_chips() {
        assert_eq!(friendly("USB3 Video", "MACROSILICON", "USB3 Video: USB3 Video"), "USB3 Video");
        assert_eq!(friendly("USB3 Video", "Monster", "USB3 Video: USB3 Video"), "Monster");
        assert_eq!(friendly("Monster 4K", "MACROSILICON", ""), "Monster 4K");
        assert_eq!(chip_of("345f", "2130"), Some("MS2130"));
        assert_eq!(chip_of("1234", "5678"), None);
    }

    #[test]
    fn the_stand_in_card() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("card.json");
        std::fs::write(&f, r#"{"name":"Monster 4K","chip":"USB3 Video (MS2130)","speed":5000,"modes":[{"format":"MJPG","w":1920,"h":1080,"fps":[60,30]}],"picture":"/x.jpg"}"#).unwrap();
        let c = read_fake(&f).unwrap();
        assert_eq!(c.name, "Monster 4K");
        assert_eq!(c.fake_picture.as_deref(), Some(Path::new("/x.jpg")));
        assert_eq!(c.pick("MJPG", 1920, 1080, 60).unwrap().3, 60);
    }
}
