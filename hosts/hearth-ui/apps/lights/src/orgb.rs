//! OpenRGB's SDK server protocol (OpenRGB, GPL-2: gitlab.com/CalcProgrammer1/OpenRGB, Documentation/OpenRGBSDK.md):
//! the client half, over TCP to the OpenRGB server on this PC (127.0.0.1:6742, `openrgb --server`).
//!
//! Every packet: "ORGB", the device index, the packet id and the data's length (u32s, little-endian), then the data.
//! Hearth Lights asks for protocol 3 (modes with brightness) and reads whatever the server says it speaks (0 to 3).
//! Nothing here keeps a connection open longer than it's used, and every read has a time limit, so a stuck server can
//! never hang the window (all of it runs off the window's thread).

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

pub const PORT: u16 = 6742;
const WANT: u32 = 3;

const COUNT: u32 = 0;
const DATA: u32 = 1;
const VERSION: u32 = 40;
const NAME: u32 = 50;
const LIST_UPDATED: u32 = 100;
const RESCAN: u32 = 140;
const UPDATE_LEDS: u32 = 1050;
const UPDATE_ZONE_LEDS: u32 = 1051;
const CUSTOM_MODE: u32 = 1100;
const UPDATE_MODE: u32 = 1101;
const SAVE_MODE: u32 = 1102;

pub const F_SPEED: u32 = 1 << 0;
pub const F_DIR_LR: u32 = 1 << 1;
pub const F_DIR_UD: u32 = 1 << 2;
pub const F_DIR_HV: u32 = 1 << 3;
pub const F_BRIGHTNESS: u32 = 1 << 4;
pub const F_PER_LED: u32 = 1 << 5;
pub const F_MODE_COLOUR: u32 = 1 << 6;
pub const F_MANUAL_SAVE: u32 = 1 << 8;

/// OpenRGB's device types (RGBController.h device_type).
pub mod kind {
    pub const MOTHERBOARD: i32 = 0;
    pub const DRAM: i32 = 1;
    pub const GPU: i32 = 2;
    pub const COOLER: i32 = 3;
    pub const LEDSTRIP: i32 = 4;
    pub const KEYBOARD: i32 = 5;
    pub const MOUSE: i32 = 6;
    pub const MOUSEMAT: i32 = 7;
    pub const HEADSET: i32 = 8;
    pub const GAMEPAD: i32 = 10;
    pub const LIGHT: i32 = 11;
    pub const CASE: i32 = 15;
}

pub type Rgb = [u8; 3];

#[derive(Debug, Clone, Default)]
pub struct Mode {
    pub name: String,
    pub value: i32,
    pub flags: u32,
    pub speed_min: u32,
    pub speed_max: u32,
    pub bright_min: u32,
    pub bright_max: u32,
    pub colors_min: u32,
    pub colors_max: u32,
    pub speed: u32,
    pub brightness: u32,
    pub direction: u32,
    pub color_mode: u32,
    pub colors: Vec<Rgb>,
}

#[derive(Debug, Clone, Default)]
pub struct Zone {
    pub name: String,
    pub kind: i32,
    pub leds: u32,
    /// a matrix's rows and columns (a per-key keyboard), else 0
    pub rows: u32,
    pub cols: u32,
}

#[derive(Debug, Clone, Default)]
pub struct Device {
    pub index: u32,
    pub kind: i32,
    pub name: String,
    pub vendor: String,
    pub location: String,
    pub modes: Vec<Mode>,
    pub active: i32,
    pub zones: Vec<Zone>,
    pub leds: Vec<String>,
    pub colors: Vec<Rgb>,
}

impl Device {
    pub fn mode(&self, words: &[&str]) -> Option<usize> {
        self.modes.iter().position(|m| {
            let n = m.name.to_lowercase();
            words.iter().any(|w| n.contains(w))
        })
    }
    /// Its Direct mode (OpenRGB's per-light control, what Hearth's own effects use).
    pub fn direct(&self) -> Option<usize> {
        self.modes.iter().position(|m| m.name.eq_ignore_ascii_case("direct")).or_else(|| self.modes.iter().position(|m| m.flags & F_PER_LED != 0 && m.name.to_lowercase().contains("custom")))
    }
    pub fn zone_start(&self, z: usize) -> usize {
        self.zones.iter().take(z).map(|z| z.leds as usize).sum()
    }
}

pub struct Conn {
    s: TcpStream,
    pub ver: u32,
}

fn put_u32(v: &mut Vec<u8>, x: u32) {
    v.extend_from_slice(&x.to_le_bytes());
}
fn put_u16(v: &mut Vec<u8>, x: u16) {
    v.extend_from_slice(&x.to_le_bytes());
}
fn put_str(v: &mut Vec<u8>, s: &str) {
    put_u16(v, s.len() as u16 + 1);
    v.extend_from_slice(s.as_bytes());
    v.push(0);
}

/// The server's address: $HEARTH_OPENRGB (host:port, CI's pretend server) or this PC.
pub fn addr() -> SocketAddr {
    std::env::var("HEARTH_OPENRGB").ok().and_then(|a| a.parse().ok()).unwrap_or_else(|| SocketAddr::from(([127, 0, 0, 1], PORT)))
}

impl Conn {
    pub fn open() -> std::io::Result<Conn> {
        let s = TcpStream::connect_timeout(&addr(), Duration::from_millis(800))?;
        s.set_nodelay(true).ok();
        s.set_read_timeout(Some(Duration::from_millis(1500)))?;
        s.set_write_timeout(Some(Duration::from_millis(1500)))?;
        let mut c = Conn { s, ver: 0 };
        c.send(0, VERSION, &WANT.to_le_bytes())?;
        // a server older than protocol 1 doesn't answer this: it speaks 0
        match c.wait(VERSION) {
            Ok(d) if d.len() >= 4 => c.ver = u32::from_le_bytes([d[0], d[1], d[2], d[3]]).min(WANT),
            _ => c.ver = 0,
        }
        let mut n = Vec::new();
        n.extend_from_slice(b"Hearth Lights\0");
        c.send(0, NAME, &n)?;
        Ok(c)
    }

    fn send(&mut self, dev: u32, id: u32, data: &[u8]) -> std::io::Result<()> {
        let mut p = Vec::with_capacity(16 + data.len());
        p.extend_from_slice(b"ORGB");
        put_u32(&mut p, dev);
        put_u32(&mut p, id);
        put_u32(&mut p, data.len() as u32);
        p.extend_from_slice(data);
        self.s.write_all(&p)
    }

    /// The next answer with this id (the server's "the list changed" notes in between are skipped).
    fn wait(&mut self, want: u32) -> std::io::Result<Vec<u8>> {
        for _ in 0..16 {
            let mut h = [0u8; 16];
            self.s.read_exact(&mut h)?;
            if &h[0..4] != b"ORGB" {
                return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "not OpenRGB"));
            }
            let id = u32::from_le_bytes([h[8], h[9], h[10], h[11]]);
            let len = u32::from_le_bytes([h[12], h[13], h[14], h[15]]) as usize;
            if len > 16 << 20 {
                return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "too big"));
            }
            let mut d = vec![0u8; len];
            self.s.read_exact(&mut d)?;
            if id == want {
                return Ok(d);
            }
            // LIST_UPDATED and anything else not asked for: skipped
            let _ = LIST_UPDATED;
        }
        Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "no answer"))
    }

    pub fn count(&mut self) -> std::io::Result<u32> {
        self.send(0, COUNT, &[])?;
        let d = self.wait(COUNT)?;
        Ok(if d.len() >= 4 { u32::from_le_bytes([d[0], d[1], d[2], d[3]]) } else { 0 })
    }

    pub fn device(&mut self, i: u32) -> std::io::Result<Device> {
        let v = self.ver.to_le_bytes();
        self.send(i, DATA, &v)?;
        let d = self.wait(DATA)?;
        parse_device(&d, self.ver, i).ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "a device it couldn't read"))
    }

    pub fn devices(&mut self) -> std::io::Result<Vec<Device>> {
        let n = self.count()?.min(128);
        let mut v = Vec::new();
        for i in 0..n {
            v.push(self.device(i)?);
        }
        Ok(v)
    }

    pub fn rescan(&mut self) -> std::io::Result<()> {
        self.send(0, RESCAN, &[])
    }

    /// Direct mode: Hearth sets each light itself.
    pub fn custom(&mut self, dev: u32) -> std::io::Result<()> {
        self.send(dev, CUSTOM_MODE, &[])
    }

    pub fn leds(&mut self, dev: u32, colors: &[Rgb]) -> std::io::Result<()> {
        let mut v = Vec::with_capacity(6 + colors.len() * 4);
        put_u32(&mut v, (6 + colors.len() * 4) as u32);
        put_u16(&mut v, colors.len() as u16);
        for c in colors {
            v.extend_from_slice(&[c[0], c[1], c[2], 0]);
        }
        self.send(dev, UPDATE_LEDS, &v)
    }

    pub fn zone_leds(&mut self, dev: u32, zone: u32, colors: &[Rgb]) -> std::io::Result<()> {
        let mut v = Vec::with_capacity(10 + colors.len() * 4);
        put_u32(&mut v, (10 + colors.len() * 4) as u32);
        put_u32(&mut v, zone);
        put_u16(&mut v, colors.len() as u16);
        for c in colors {
            v.extend_from_slice(&[c[0], c[1], c[2], 0]);
        }
        self.send(dev, UPDATE_ZONE_LEDS, &v)
    }

    /// One of the device's own modes, with its speed, brightness, direction and colours; `save`: kept on the device
    /// itself (OpenRGB's Save to device), so it stays after a restart even without Hearth.
    pub fn set_mode(&mut self, dev: u32, idx: usize, m: &Mode, save: bool) -> std::io::Result<()> {
        let blob = mode_blob(m, self.ver);
        let mut v = Vec::with_capacity(8 + blob.len());
        put_u32(&mut v, (8 + blob.len()) as u32);
        v.extend_from_slice(&(idx as i32).to_le_bytes());
        v.extend_from_slice(&blob);
        self.send(dev, UPDATE_MODE, &v)?;
        if save {
            self.send(dev, SAVE_MODE, &v)?;
        }
        Ok(())
    }
}

fn mode_blob(m: &Mode, ver: u32) -> Vec<u8> {
    let mut v = Vec::new();
    put_str(&mut v, &m.name);
    v.extend_from_slice(&m.value.to_le_bytes());
    put_u32(&mut v, m.flags);
    put_u32(&mut v, m.speed_min);
    put_u32(&mut v, m.speed_max);
    if ver >= 3 {
        put_u32(&mut v, m.bright_min);
        put_u32(&mut v, m.bright_max);
    }
    put_u32(&mut v, m.colors_min);
    put_u32(&mut v, m.colors_max);
    put_u32(&mut v, m.speed);
    if ver >= 3 {
        put_u32(&mut v, m.brightness);
    }
    put_u32(&mut v, m.direction);
    put_u32(&mut v, m.color_mode);
    put_u16(&mut v, m.colors.len() as u16);
    for c in &m.colors {
        v.extend_from_slice(&[c[0], c[1], c[2], 0]);
    }
    v
}

struct Rd<'a> {
    b: &'a [u8],
    at: usize,
}
impl<'a> Rd<'a> {
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let s = self.b.get(self.at..self.at + n)?;
        self.at += n;
        Some(s)
    }
    fn u16(&mut self) -> Option<u16> {
        let s = self.take(2)?;
        Some(u16::from_le_bytes([s[0], s[1]]))
    }
    fn u32(&mut self) -> Option<u32> {
        let s = self.take(4)?;
        Some(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
    }
    fn i32(&mut self) -> Option<i32> {
        self.u32().map(|x| x as i32)
    }
    fn s(&mut self) -> Option<String> {
        let n = self.u16()? as usize;
        let b = self.take(n)?;
        Some(String::from_utf8_lossy(b).trim_end_matches('\0').to_string())
    }
    fn color(&mut self) -> Option<Rgb> {
        let s = self.take(4)?;
        Some([s[0], s[1], s[2]])
    }
}

/// A device as the server describes it (REQUEST_CONTROLLER_DATA's answer).
pub fn parse_device(b: &[u8], ver: u32, index: u32) -> Option<Device> {
    let mut r = Rd { b, at: 0 };
    let _size = r.u32()?;
    let mut d = Device { index, kind: r.i32()?, name: r.s()?, ..Default::default() };
    if ver >= 1 {
        d.vendor = r.s()?;
    }
    let _description = r.s()?;
    let _version = r.s()?;
    let _serial = r.s()?;
    d.location = r.s()?;
    let nm = r.u16()?;
    d.active = r.i32()?;
    for _ in 0..nm {
        let mut m = Mode { name: r.s()?, value: r.i32()?, flags: r.u32()?, speed_min: r.u32()?, speed_max: r.u32()?, ..Default::default() };
        if ver >= 3 {
            m.bright_min = r.u32()?;
            m.bright_max = r.u32()?;
        }
        m.colors_min = r.u32()?;
        m.colors_max = r.u32()?;
        m.speed = r.u32()?;
        if ver >= 3 {
            m.brightness = r.u32()?;
        }
        m.direction = r.u32()?;
        m.color_mode = r.u32()?;
        let nc = r.u16()?;
        for _ in 0..nc {
            m.colors.push(r.color()?);
        }
        d.modes.push(m);
    }
    let nz = r.u16()?;
    for _ in 0..nz {
        let mut z = Zone { name: r.s()?, kind: r.i32()?, ..Default::default() };
        let _min = r.u32()?;
        let _max = r.u32()?;
        z.leds = r.u32()?;
        let ml = r.u16()?;
        if ml > 0 {
            z.rows = r.u32()?;
            z.cols = r.u32()?;
            let cells = (z.rows as usize).saturating_mul(z.cols as usize);
            r.take(cells.checked_mul(4)?)?;
        }
        d.zones.push(z);
    }
    let nl = r.u16()?;
    for _ in 0..nl {
        let n = r.s()?;
        let _v = r.u32()?;
        d.leds.push(n);
    }
    let nc = r.u16()?;
    for _ in 0..nc {
        d.colors.push(r.color()?);
    }
    Some(d)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dev_bytes(ver: u32) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&kind::COOLER.to_le_bytes());
        put_str(&mut v, "Front fans");
        if ver >= 1 {
            put_str(&mut v, "Hearth");
        }
        put_str(&mut v, "d");
        put_str(&mut v, "1");
        put_str(&mut v, "s");
        put_str(&mut v, "HID: /dev/hidraw3");
        put_u16(&mut v, 1);
        v.extend_from_slice(&0i32.to_le_bytes());
        let m = Mode { name: "Direct".into(), flags: F_PER_LED, color_mode: 1, ..Default::default() };
        v.extend_from_slice(&mode_blob(&m, ver));
        put_u16(&mut v, 2);
        for z in ["Fan 1", "Fan 2"] {
            put_str(&mut v, z);
            v.extend_from_slice(&1i32.to_le_bytes());
            put_u32(&mut v, 0);
            put_u32(&mut v, 24);
            put_u32(&mut v, 24);
            put_u16(&mut v, 0);
        }
        put_u16(&mut v, 1);
        put_str(&mut v, "LED 1");
        put_u32(&mut v, 0);
        put_u16(&mut v, 1);
        v.extend_from_slice(&[255, 138, 61, 0]);
        let mut out = Vec::new();
        put_u32(&mut out, (v.len() + 4) as u32);
        out.extend_from_slice(&v);
        out
    }

    #[test]
    fn a_device_reads_back_in_every_protocol() {
        for ver in 0..=3 {
            let d = parse_device(&dev_bytes(ver), ver, 2).expect("parsed");
            assert_eq!(d.name, "Front fans");
            assert_eq!(d.zones.len(), 2);
            assert_eq!(d.zones[1].leds, 24);
            assert_eq!(d.modes[0].name, "Direct");
            assert_eq!(d.colors, vec![[255, 138, 61]]);
            assert_eq!(d.direct(), Some(0));
        }
    }

    #[test]
    fn a_cut_short_answer_is_none_not_a_panic() {
        let b = dev_bytes(3);
        for n in 0..b.len() {
            let _ = parse_device(&b[..n], 3, 0);
        }
    }
}
