//! CDs and DVDs: the drives, what's in them, and their tracks as things the web view can play.
//!
//! - Finding discs: the drives are /sys/block/sr*; what's in one comes from udev (ID_CDROM_MEDIA_*), and a data disc's
//!   top folder (VIDEO_TS, MPEGAV, MPEG2, BDMV) from its ISO 9660 directory, read straight off the disc. A disc going
//!   in or out is a udev event (`watch`: a netlink socket the thread sleeps on, so nothing runs while nothing happens).
//! - Audio CDs: the table of contents and CD-Text come from the drive (SCSI READ TOC through SG_IO, which udev lets the
//!   person at the computer use), the sound from READ CD, served as a WAV file per track (byte ranges, so seeking is
//!   instant). Names when the disc has no CD-Text: MusicBrainz, then CDDB (gnudb), by the disc's ID.
//! - Video CDs and Super Video CDs: their MPEG tracks are Mode 2 sectors the kernel's file system can't read whole,
//!   so they're read raw (READ CD) and the 2324 bytes of each sector's picture and sound are kept.
//! - A disc image works the same way: a .cue with its .bin (an audio CD or VCD), an .iso, or a VIDEO_TS folder.
//! - DVD-Video is dvd.rs.

use serde::Serialize;
use std::collections::HashMap;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

pub const RAW: usize = 2352;
pub const DATA: usize = 2048;
/// User data in a Mode 2 Form 2 sector (a VCD's MPEG).
pub const FORM2: usize = 2324;

// ------------------------------------------------------------------ programs, with a time limit

/// Runs a program and gives its output, or None when it isn't there, fails or takes longer than `secs`.
pub fn run(program: &str, args: &[&str], secs: u64) -> Option<std::process::Output> {
    let child = std::process::Command::new(program).args(args).stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).spawn().ok()?;
    let id = child.id();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || { let _ = tx.send(child.wait_with_output()); });
    match rx.recv_timeout(Duration::from_secs(secs)) {
        Ok(Ok(o)) => Some(o),
        _ => { unsafe { libc::kill(id as i32, libc::SIGKILL); } None }
    }
}

// ------------------------------------------------------------------ the drives

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
pub struct Drive {
    pub device: String,
    /// "DVD drive", "Blu-ray drive" or "CD drive", with the maker's name when there is one.
    pub name: String,
    pub media: bool,
    /// What's in it: "dvd", "vcd", "svcd", "audio-cd", "data", "bluray", "blank", "" (empty).
    pub kind: String,
    pub label: String,
    pub mount: Option<String>,
}

/// udev's properties for a device (udevadm info).
pub fn udev_props(dev: &str) -> HashMap<String, String> {
    let Some(o) = run("udevadm", &["info", "--query=property", "--name", dev], 5) else { return HashMap::new() };
    parse_props(&String::from_utf8_lossy(&o.stdout), '\n')
}

pub fn parse_props(text: &str, sep: char) -> HashMap<String, String> {
    text.split(sep).filter_map(|l| l.split_once('=')).map(|(k, v)| (k.trim().to_string(), v.to_string())).collect()
}

/// Where a device is mounted (/proc/mounts), if it is.
pub fn mount_of(dev: &str) -> Option<String> {
    let real = std::fs::canonicalize(dev).unwrap_or_else(|_| PathBuf::from(dev));
    std::fs::read_to_string("/proc/mounts").ok()?.lines().find_map(|l| {
        let mut it = l.split_whitespace();
        let (d, m) = (it.next()?, it.next()?);
        (Path::new(d) == real || d == dev).then(|| unescape_mount(m))
    })
}

fn unescape_mount(m: &str) -> String {
    let b = m.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'\\' && i + 3 < b.len() && b[i + 1..i + 4].iter().all(|c| (b'0'..=b'7').contains(c)) {
            out.push(((b[i + 1] - b'0') << 6) | ((b[i + 2] - b'0') << 3) | (b[i + 3] - b'0'));
            i += 4;
        } else { out.push(b[i]); i += 1; }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// The device a folder is mounted from (a disc opened by its folder, like Plasma does).
pub fn device_of(dir: &Path) -> Option<String> {
    let want = std::fs::canonicalize(dir).ok()?;
    std::fs::read_to_string("/proc/mounts").ok()?.lines().find_map(|l| {
        let mut it = l.split_whitespace();
        let (d, m) = (it.next()?, it.next()?);
        (Path::new(&unescape_mount(m)) == want && d.starts_with("/dev/sr")).then(|| d.to_string())
    })
}

/// What udev's properties say is in the drive ("" when it's empty or udev doesn't say).
pub fn kind_from_props(p: &HashMap<String, String>) -> &'static str {
    let on = |k: &str| p.get(k).map(|v| v == "1").unwrap_or(false);
    let n = |k: &str| p.get(k).and_then(|v| v.parse::<u32>().ok()).unwrap_or(0);
    if !on("ID_CDROM_MEDIA") { return ""; }
    if p.keys().any(|k| k.starts_with("ID_CDROM_MEDIA_BD")) { return "bluray"; }
    if p.get("ID_CDROM_MEDIA_STATE").map(|s| s == "blank").unwrap_or(false) { return "blank"; }
    if n("ID_CDROM_MEDIA_TRACK_COUNT_AUDIO") > 0 { return "audio-cd"; }
    "data"
}

/// Every CD/DVD drive, and what's in each.
pub fn drives() -> Vec<Drive> {
    let mut v = Vec::new();
    let Ok(rd) = std::fs::read_dir("/sys/block") else { return v };
    let mut names: Vec<String> = rd.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).filter(|n| n.starts_with("sr")).collect();
    names.sort();
    for n in names {
        let dev = format!("/dev/{n}");
        let p = udev_props(&dev);
        let mut kind = kind_from_props(&p).to_string();
        let maker = std::fs::read_to_string(format!("/sys/block/{n}/device/vendor")).unwrap_or_default().trim().to_string();
        let what = if p.contains_key("ID_CDROM_BD") { "Blu-ray drive" } else if p.contains_key("ID_CDROM_DVD") { "DVD drive" } else { "CD drive" };
        let name = if maker.is_empty() || maker.eq_ignore_ascii_case("ata") { what.to_string() } else { format!("{maker} {}", what.to_ascii_lowercase()) };
        let mut label = p.get("ID_FS_LABEL_ENC").map(|l| unescape_udev(l)).or_else(|| p.get("ID_FS_LABEL").cloned()).unwrap_or_default();
        if kind == "data" {
            if let Ok(dev_s) = Device::open(&dev) {
                let (k, l) = iso_kind(&dev_s, 0);
                kind = k.to_string();
                if label.is_empty() { label = l; }
            }
        }
        v.push(Drive { device: dev.clone(), name, media: !kind.is_empty(), kind, label: pretty_label(&label), mount: mount_of(&dev) });
    }
    v
}

fn unescape_udev(s: &str) -> String {
    let mut out = Vec::new();
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'\\' && i + 3 < b.len() && b[i + 1] == b'x' {
            if let Ok(x) = u8::from_str_radix(&s[i + 2..i + 4], 16) { out.push(x); i += 4; continue; }
        }
        out.push(b[i]); i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// "MY_FILM_DISC_1" -> "My Film Disc 1" (labels written in capitals with underscores), else as it is.
pub fn pretty_label(l: &str) -> String {
    let l = l.trim();
    if l.chars().any(|c| c.is_lowercase()) || l.is_empty() { return l.to_string(); }
    l.replace('_', " ").split_whitespace().map(|w| {
        let mut c = w.chars();
        match c.next() { Some(f) => f.to_string() + &c.as_str().to_lowercase(), None => String::new() }
    }).collect::<Vec<_>>().join(" ")
}

// ------------------------------------------------------------------ sectors

/// Where a disc's sectors come from: a drive, or an image file.
pub trait Sectors: Send + Sync {
    /// `n` whole 2352-byte sectors from `lba` (audio: the sound itself; data: with their headers).
    fn raw(&self, lba: u32, n: u32, audio: bool) -> io::Result<Vec<u8>>;
    /// `n` 2048-byte data sectors (ISO 9660) from `lba`.
    fn data(&self, lba: u32, n: u32) -> io::Result<Vec<u8>>;
}

/// The user data of a whole data sector: Mode 1 at 16, Mode 2 (Form 1) at 24.
fn user_data(raw: &[u8]) -> &[u8] {
    let at = if raw.len() > 15 && raw[15] == 2 { 24 } else { 16 };
    &raw[at..(at + DATA).min(raw.len())]
}

/// A CD/DVD drive.
pub struct Device { file: std::fs::File, pub path: String }

#[repr(C)]
struct SgIoHdr {
    interface_id: i32,
    dxfer_direction: i32,
    cmd_len: u8,
    mx_sb_len: u8,
    iovec_count: u16,
    dxfer_len: u32,
    dxferp: *mut libc::c_void,
    cmdp: *const u8,
    sbp: *mut u8,
    timeout: u32,
    flags: u32,
    pack_id: i32,
    usr_ptr: *mut libc::c_void,
    status: u8,
    masked_status: u8,
    msg_status: u8,
    sb_len_wr: u8,
    host_status: u16,
    driver_status: u16,
    resid: i32,
    duration: u32,
    info: u32,
}

const SG_IO: libc::c_ulong = 0x2285;
const SG_DXFER_FROM_DEV: i32 = -3;
const CDROMEJECT: libc::c_ulong = 0x5309;
const CDROM_LOCKDOOR: libc::c_ulong = 0x5329;

impl Device {
    pub fn open(path: &str) -> io::Result<Device> {
        use std::os::unix::fs::OpenOptionsExt;
        let file = std::fs::OpenOptions::new().read(true).custom_flags(libc::O_NONBLOCK).open(path)?;
        Ok(Device { file, path: path.to_string() })
    }

    /// One SCSI command that reads `len` bytes back.
    pub fn scsi_read(&self, cdb: &[u8], len: usize, secs: u32) -> io::Result<Vec<u8>> {
        use std::os::unix::io::AsRawFd;
        let mut buf = vec![0u8; len];
        let mut sense = [0u8; 32];
        let mut h = SgIoHdr {
            interface_id: 'S' as i32, dxfer_direction: SG_DXFER_FROM_DEV, cmd_len: cdb.len() as u8, mx_sb_len: sense.len() as u8,
            iovec_count: 0, dxfer_len: len as u32, dxferp: buf.as_mut_ptr() as *mut _, cmdp: cdb.as_ptr(), sbp: sense.as_mut_ptr(),
            timeout: secs * 1000, flags: 0, pack_id: 0, usr_ptr: std::ptr::null_mut(), status: 0, masked_status: 0, msg_status: 0,
            sb_len_wr: 0, host_status: 0, driver_status: 0, resid: 0, duration: 0, info: 0,
        };
        let r = unsafe { libc::ioctl(self.file.as_raw_fd(), SG_IO as _, &mut h as *mut SgIoHdr) };
        if r < 0 { return Err(io::Error::last_os_error()); }
        if h.info & 1 != 0 || h.status != 0 || h.host_status != 0 {
            return Err(io::Error::new(io::ErrorKind::Other, format!("the drive said no (status {}, sense {:02x}/{:02x}/{:02x})", h.status, sense[2] & 0x0f, sense[12], sense[13])));
        }
        buf.truncate(len.saturating_sub(h.resid.max(0) as usize));
        Ok(buf)
    }

    pub fn toc(&self) -> io::Result<Toc> {
        let b = self.scsi_read(&[0x43, 0, 0, 0, 0, 0, 0, 0x03, 0x24, 0], 804, 20)?;
        parse_toc(&b).ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "couldn't read the disc's table of contents"))
    }

    pub fn cd_text(&self) -> Option<CdText> {
        let b = self.scsi_read(&[0x43, 0, 0x05, 0, 0, 0, 0, 0x24, 0x04, 0], 9220, 20).ok()?;
        if b.len() < 4 { return None; }
        let n = (u16::from_be_bytes([b[0], b[1]]) as usize).saturating_sub(2).min(b.len() - 4);
        parse_cd_text(&b[4..4 + n])
    }

    /// Opens the tray (after the desktop let go of it).
    pub fn eject(&self) -> io::Result<()> {
        use std::os::unix::io::AsRawFd;
        unsafe { libc::ioctl(self.file.as_raw_fd(), CDROM_LOCKDOOR as _, 0) };
        if unsafe { libc::ioctl(self.file.as_raw_fd(), CDROMEJECT as _) } < 0 { return Err(io::Error::last_os_error()); }
        Ok(())
    }
}

impl Sectors for Device {
    fn raw(&self, lba: u32, n: u32, audio: bool) -> io::Result<Vec<u8>> {
        let mut out = Vec::with_capacity(n as usize * RAW);
        let mut at = lba;
        while at < lba + n {
            let k = (lba + n - at).min(24);
            let l = at.to_be_bytes();
            // READ CD: CD-DA sectors' sound (type 1, user data), or any sector whole (type 0: sync, headers, data, EDC).
            let cdb = [0xBE, if audio { 1 << 2 } else { 0 }, l[0], l[1], l[2], l[3], 0, 0, k as u8, if audio { 0x10 } else { 0xF8 }, 0, 0];
            let mut tries = 0;
            let b = loop {
                match self.scsi_read(&cdb, k as usize * RAW, 30) {
                    Ok(b) => break b,
                    Err(e) if tries >= 2 => return if out.is_empty() { Err(e) } else { Ok(out) },
                    Err(_) => tries += 1,
                }
            };
            if b.is_empty() { break; }
            out.extend_from_slice(&b);
            at += k;
        }
        Ok(out)
    }
    fn data(&self, lba: u32, n: u32) -> io::Result<Vec<u8>> {
        use std::os::unix::fs::FileExt;
        let mut buf = vec![0u8; n as usize * DATA];
        let got = self.file.read_at(&mut buf, lba as u64 * DATA as u64)?;
        buf.truncate(got);
        Ok(buf)
    }
}

/// A disc image: a .bin (whole 2352-byte sectors, from its .cue) or an .iso (2048-byte sectors).
pub struct Image { files: Vec<(u32, PathBuf, bool)>, sector: usize }

impl Image {
    pub fn iso(path: &Path) -> Image { Image { files: vec![(0, path.to_path_buf(), false)], sector: DATA } }
    pub fn bin(cue: &Cue) -> Image { Image { files: cue.files.iter().map(|f| (f.lba, f.path.clone(), f.big_endian)).collect(), sector: RAW } }
    fn read(&self, lba: u32, n: u32) -> io::Result<(Vec<u8>, bool)> {
        let (start, path, big) = self.files.iter().rev().find(|f| f.0 <= lba).ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "before the first file"))?;
        let mut f = std::fs::File::open(path)?;
        f.seek(SeekFrom::Start((lba - start) as u64 * self.sector as u64))?;
        let mut buf = vec![0u8; n as usize * self.sector];
        let mut got = 0;
        while got < buf.len() {
            let k = f.read(&mut buf[got..])?;
            if k == 0 { break; }
            got += k;
        }
        buf.truncate(got - got % self.sector);
        Ok((buf, *big))
    }
}

impl Sectors for Image {
    fn raw(&self, lba: u32, n: u32, audio: bool) -> io::Result<Vec<u8>> {
        if self.sector != RAW { return Err(io::Error::new(io::ErrorKind::Unsupported, "an .iso has no sound sectors")); }
        let (mut b, big) = self.read(lba, n)?;
        if audio && big { for s in b.chunks_exact_mut(2) { s.swap(0, 1); } }
        Ok(b)
    }
    fn data(&self, lba: u32, n: u32) -> io::Result<Vec<u8>> {
        let (b, _) = self.read(lba, n)?;
        if self.sector == DATA { return Ok(b); }
        Ok(b.chunks(RAW).flat_map(|s| user_data(s).to_vec()).collect())
    }
}

// ------------------------------------------------------------------ the table of contents

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct TocTrack { pub n: u32, pub lba: u32, pub audio: bool, pub mode2: bool }

#[derive(Serialize, Clone, Debug, PartialEq, Default)]
pub struct Toc { pub first: u32, pub last: u32, pub tracks: Vec<TocTrack>, pub leadout: u32 }

impl Toc {
    /// Where track `i` (an index into `tracks`) ends: the next track, or the lead-out. Before a data track on an
    /// Enhanced CD there's a gap of 11 400 sectors that isn't part of the last song.
    pub fn end(&self, i: usize) -> u32 {
        match self.tracks.get(i + 1) {
            Some(n) if self.tracks[i].audio && !n.audio => n.lba.saturating_sub(11400),
            Some(n) => n.lba,
            None => self.leadout,
        }
    }
}

/// READ TOC's answer (format 0, LBA).
pub fn parse_toc(b: &[u8]) -> Option<Toc> {
    if b.len() < 4 { return None; }
    let len = (u16::from_be_bytes([b[0], b[1]]) as usize + 2).min(b.len());
    let mut t = Toc { first: b[2] as u32, last: b[3] as u32, ..Default::default() };
    for d in b[4..len].chunks_exact(8) {
        let lba = u32::from_be_bytes([d[4], d[5], d[6], d[7]]);
        if d[2] == 0xAA { t.leadout = lba; } else { t.tracks.push(TocTrack { n: d[2] as u32, lba, audio: d[1] & 0x04 == 0, mode2: false }); }
    }
    (!t.tracks.is_empty() && t.leadout > 0).then_some(t)
}

// ------------------------------------------------------------------ .cue files

#[derive(Clone, Debug, PartialEq)]
pub struct CueFile { pub path: PathBuf, pub lba: u32, pub big_endian: bool }

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Cue { pub files: Vec<CueFile>, pub toc: Toc, pub text: CdText }

fn msf(s: &str) -> Option<u32> {
    let p: Vec<u32> = s.split(':').filter_map(|x| x.trim().parse().ok()).collect();
    (p.len() == 3).then(|| (p[0] * 60 + p[1]) * 75 + p[2])
}

fn cue_str(rest: &str) -> String {
    let r = rest.trim();
    r.strip_prefix('"').and_then(|x| x.rfind('"').map(|i| x[..i].to_string())).unwrap_or_else(|| r.to_string())
}

/// A .cue sheet: its files, tracks (INDEX 01) and CD-Text (TITLE, PERFORMER).
pub fn parse_cue(text: &str, dir: &Path) -> Result<Cue, String> {
    let mut c = Cue::default();
    let mut file_lba = 0u32;
    let mut cur_track: Option<usize> = None;
    let mut file_len_sectors = 0u32;
    for line in text.lines() {
        let l = line.trim();
        let (kw, rest) = l.split_once(char::is_whitespace).unwrap_or((l, ""));
        match kw.to_ascii_uppercase().as_str() {
            "FILE" => {
                let name = cue_str(rest.rsplit_once(char::is_whitespace).map(|x| x.0).unwrap_or(rest));
                let big = rest.trim_end().to_ascii_uppercase().ends_with("MOTOROLA");
                if !c.files.is_empty() { file_lba += file_len_sectors; }
                let path = dir.join(&name);
                file_len_sectors = std::fs::metadata(&path).map(|m| (m.len() / RAW as u64) as u32).unwrap_or(0);
                c.files.push(CueFile { path, lba: file_lba, big_endian: big });
            }
            "TRACK" => {
                let mut it = rest.split_whitespace();
                let n: u32 = it.next().and_then(|x| x.parse().ok()).ok_or("A TRACK line without a number")?;
                let mode = it.next().unwrap_or("").to_ascii_uppercase();
                c.toc.tracks.push(TocTrack { n, lba: file_lba, audio: mode == "AUDIO", mode2: mode.starts_with("MODE2") });
                cur_track = Some(c.toc.tracks.len() - 1);
                if c.text.titles.len() < c.toc.tracks.len() + 1 { c.text.titles.resize(c.toc.tracks.len() + 1, String::new()); c.text.performers.resize(c.toc.tracks.len() + 1, String::new()); }
            }
            "INDEX" => {
                let mut it = rest.split_whitespace();
                if it.next().and_then(|x| x.parse::<u32>().ok()) == Some(1) {
                    if let (Some(i), Some(t)) = (cur_track, it.next().and_then(msf)) { c.toc.tracks[i].lba = file_lba + t; }
                }
            }
            "TITLE" | "PERFORMER" => {
                let s = cue_str(rest);
                let i = cur_track.map(|i| i + 1).unwrap_or(0);
                let list = if kw.eq_ignore_ascii_case("TITLE") { &mut c.text.titles } else { &mut c.text.performers };
                if list.len() <= i { list.resize(i + 1, String::new()); }
                list[i] = s;
            }
            _ => {}
        }
    }
    if c.files.is_empty() || c.toc.tracks.is_empty() { return Err("That .cue file has no tracks.".into()); }
    c.toc.first = c.toc.tracks[0].n;
    c.toc.last = c.toc.tracks.last().unwrap().n;
    c.toc.leadout = file_lba + file_len_sectors;
    let n = c.toc.tracks.len() + 1;
    c.text.titles.resize(n, String::new());
    c.text.performers.resize(n, String::new());
    Ok(c)
}

// ------------------------------------------------------------------ CD-Text

/// Names on the disc itself: [0] is the album (and its artist), [n] track n.
#[derive(Serialize, Clone, Debug, Default, PartialEq)]
pub struct CdText { pub titles: Vec<String>, pub performers: Vec<String> }

impl CdText {
    pub fn is_empty(&self) -> bool { self.titles.iter().chain(&self.performers).all(|s| s.is_empty()) }
}

/// CD-Text packs (READ TOC format 5): 18 bytes each; the first language block's titles (0x80) and performers (0x81).
pub fn parse_cd_text(b: &[u8]) -> Option<CdText> {
    let mut by_type: HashMap<u8, (u32, Vec<u8>)> = HashMap::new();
    let mut packs: Vec<&[u8]> = b.chunks_exact(18).collect();
    packs.sort_by_key(|p| p[2]);
    for p in packs {
        let (kind, track, block) = (p[0], (p[1] & 0x7f) as u32, (p[3] >> 4) & 7);
        if block != 0 || p[3] & 0x80 != 0 || !(kind == 0x80 || kind == 0x81) { continue; }
        let e = by_type.entry(kind).or_insert((track, Vec::new()));
        e.1.extend_from_slice(&p[4..16]);
    }
    let mut t = CdText::default();
    for (kind, (first, bytes)) in by_type {
        let list = if kind == 0x80 { &mut t.titles } else { &mut t.performers };
        let mut prev = String::new();
        let mut parts: Vec<&[u8]> = bytes.split(|c| *c == 0).collect();
        // The last pack is padded with zeros.
        while parts.last().map(|p| p.is_empty()).unwrap_or(false) { parts.pop(); }
        for (i, raw) in parts.iter().enumerate() {
            // ISO 8859-1; a tab means "the same as the one before".
            let mut s: String = raw.iter().map(|c| *c as char).collect();
            if s == "\t" { s = prev.clone(); }
            let at = first as usize + i;
            if list.len() <= at { list.resize(at + 1, String::new()); }
            list[at] = s.trim().to_string();
            prev = s;
        }
    }
    let n = t.titles.len().max(t.performers.len());
    t.titles.resize(n, String::new());
    t.performers.resize(n, String::new());
    (!t.is_empty()).then_some(t)
}

// ------------------------------------------------------------------ disc IDs and names from the internet

fn audio_toc(t: &Toc) -> (u32, u32, u32, Vec<u32>) {
    let audio: Vec<&TocTrack> = t.tracks.iter().filter(|x| x.audio).collect();
    let last_i = t.tracks.iter().rposition(|x| x.audio).unwrap_or(0);
    let leadout = t.end(last_i);
    (audio.first().map(|x| x.n).unwrap_or(1), audio.last().map(|x| x.n).unwrap_or(1), leadout + 150, audio.iter().map(|x| x.lba + 150).collect())
}

/// MusicBrainz's disc ID (libdiscid's way: SHA-1 of the table of contents, in their base64).
pub fn musicbrainz_id(t: &Toc) -> String {
    use base64::Engine;
    use sha1::{Digest, Sha1};
    let (first, last, leadout, offs) = audio_toc(t);
    let mut s = format!("{first:02X}{last:02X}{leadout:08X}");
    for i in 0..99 { s.push_str(&format!("{:08X}", offs.get(i).copied().unwrap_or(0))); }
    let d = Sha1::digest(s.as_bytes());
    base64::engine::general_purpose::STANDARD.encode(d).replace('+', ".").replace('/', "_").replace('=', "-")
}

/// MusicBrainz's "toc" parameter: first last lead-out offsets...
pub fn musicbrainz_toc(t: &Toc) -> String {
    let (first, last, leadout, offs) = audio_toc(t);
    std::iter::once(first).chain([last, leadout]).chain(offs).map(|x| x.to_string()).collect::<Vec<_>>().join("+")
}

/// The CDDB (freedb, gnudb) disc ID.
pub fn cddb_id(t: &Toc) -> u32 {
    let digits = |mut n: u32| { let mut s = 0; while n > 0 { s += n % 10; n /= 10; } s };
    let n: u32 = t.tracks.iter().map(|x| digits((x.lba + 150) / 75)).sum();
    let len = (t.leadout + 150) / 75 - (t.tracks[0].lba + 150) / 75;
    ((n % 255) << 24) | (len << 8) | t.tracks.len() as u32
}

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
pub struct Names { pub album: String, pub artist: String, pub tracks: Vec<(String, String)>, pub from: String }

/// MusicBrainz's answer for a disc ID: the release that has this disc, else the first one with as many tracks.
pub fn parse_musicbrainz(json: &serde_json::Value, id: &str, ntracks: usize) -> Option<Names> {
    let credit = |v: &serde_json::Value| v["artist-credit"].as_array().map(|a| a.iter().map(|c| format!("{}{}", c["name"].as_str().unwrap_or(""), c["joinphrase"].as_str().unwrap_or(""))).collect::<String>()).unwrap_or_default();
    let releases = json["releases"].as_array()?;
    let mut best: Option<(&serde_json::Value, &serde_json::Value)> = None;
    for r in releases {
        for m in r["media"].as_array().into_iter().flatten() {
            let has = m["discs"].as_array().map(|d| d.iter().any(|d| d["id"] == id)).unwrap_or(false);
            let fits = m["tracks"].as_array().map(|t| t.len() == ntracks).unwrap_or(false);
            if has { best = Some((r, m)); break; }
            if fits && best.is_none() { best = Some((r, m)); }
        }
        if best.map(|b| b.1["discs"].as_array().map(|d| d.iter().any(|d| d["id"] == id)).unwrap_or(false)).unwrap_or(false) { break; }
    }
    let (r, m) = best?;
    let artist = credit(r);
    let tracks = m["tracks"].as_array()?.iter().map(|t| {
        let a = credit(t);
        (t["title"].as_str().unwrap_or("").to_string(), if a.is_empty() { artist.clone() } else { a })
    }).collect();
    Some(Names { album: r["title"].as_str().unwrap_or("").to_string(), artist, tracks, from: "MusicBrainz".into() })
}

/// A CDDB entry (xmcd text): DTITLE=Artist / Album, TTITLEn=Song (long lines come in several pieces).
pub fn parse_xmcd(text: &str) -> Option<Names> {
    let mut d = String::new();
    let mut t: Vec<String> = Vec::new();
    for l in text.lines() {
        if let Some(v) = l.strip_prefix("DTITLE=") { d.push_str(v); }
        else if let Some(rest) = l.strip_prefix("TTITLE") {
            if let Some((n, v)) = rest.split_once('=') {
                if let Ok(i) = n.parse::<usize>() { if t.len() <= i { t.resize(i + 1, String::new()); } t[i].push_str(v); }
            }
        }
    }
    if d.is_empty() && t.is_empty() { return None; }
    let (artist, album) = d.split_once(" / ").map(|(a, b)| (a.trim().to_string(), b.trim().to_string())).unwrap_or((String::new(), d.trim().to_string()));
    let tracks = t.into_iter().map(|s| match s.split_once(" / ") { Some((a, b)) if !artist.is_empty() && artist.eq_ignore_ascii_case("various") => (b.trim().to_string(), a.trim().to_string()), _ => (s.trim().to_string(), artist.clone()) }).collect();
    Some(Names { album, artist, tracks, from: "CDDB".into() })
}

/// Names for an audio CD from the internet: MusicBrainz, else CDDB (gnudb). Nothing about you is sent: the disc's
/// table of contents, and the program's own name.
pub fn lookup_names(t: &Toc) -> Result<Names, String> {
    let id = musicbrainz_id(t);
    let n = t.tracks.iter().filter(|x| x.audio).count();
    let mb = format!("https://musicbrainz.org/ws/2/discid/{id}?toc={}&inc=recordings+artist-credits&fmt=json", musicbrainz_toc(t));
    if let Ok(text) = crate::net::get_text(&mb, 4 << 20) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) { if let Some(names) = parse_musicbrainz(&v, &id, n) { return Ok(names); } }
    }
    let cid = cddb_id(t);
    let offs = t.tracks.iter().map(|x| (x.lba + 150).to_string()).collect::<Vec<_>>().join("+");
    let secs = (t.leadout + 150) / 75;
    let hello = "hello=hearth+hearthos.fun+HearthPlayer+1&proto=6";
    let base = "https://gnudb.gnudb.org/~cddb/cddb.cgi";
    let q = crate::net::get_text(&format!("{base}?cmd=cddb+query+{cid:08x}+{}+{offs}+{secs}&{hello}", t.tracks.len()), 1 << 20).map_err(|e| format!("Couldn't look the disc up: {e}"))?;
    let mut lines = q.lines();
    let first = lines.next().unwrap_or("");
    let hit = if first.starts_with("200 ") { first[4..].to_string() } else if first.starts_with("210") || first.starts_with("211") { lines.next().unwrap_or("").to_string() } else { String::new() };
    let mut it = hit.split_whitespace();
    let (Some(cat), Some(did)) = (it.next(), it.next()) else { return Err("The disc wasn't found in MusicBrainz or CDDB.".into()) };
    let r = crate::net::get_text(&format!("{base}?cmd=cddb+read+{cat}+{did}&{hello}"), 1 << 20).map_err(|e| format!("Couldn't look the disc up: {e}"))?;
    parse_xmcd(&r).ok_or_else(|| "The disc wasn't found in MusicBrainz or CDDB.".into())
}

// ------------------------------------------------------------------ what kind of disc

/// The names in a disc's top folder (ISO 9660), and its volume name. `base`: the session's first sector (0).
pub fn iso_root(s: &dyn Sectors, base: u32) -> Option<(Vec<String>, String)> {
    let pvd = s.data(base + 16, 1).ok()?;
    if pvd.len() < DATA || pvd[0] != 1 || &pvd[1..6] != b"CD001" { return None; }
    let label = String::from_utf8_lossy(&pvd[40..72]).trim().to_string();
    let lba = u32::from_le_bytes([pvd[158], pvd[159], pvd[160], pvd[161]]);
    let len = u32::from_le_bytes([pvd[166], pvd[167], pvd[168], pvd[169]]).min(64 * DATA as u32);
    let dir = s.data(lba, len.div_ceil(DATA as u32)).ok()?;
    let mut names = Vec::new();
    for sec in dir.chunks(DATA) {
        let mut i = 0;
        while i + 33 < sec.len() {
            let rl = sec[i] as usize;
            if rl == 0 { break; }
            let nl = sec[i + 32] as usize;
            if i + 33 + nl <= sec.len() && nl > 0 && !(nl == 1 && sec[i + 33] <= 1) {
                let n = String::from_utf8_lossy(&sec[i + 33..i + 33 + nl]);
                names.push(n.split(';').next().unwrap_or("").trim_end_matches('.').to_ascii_uppercase());
            }
            i += rl;
        }
    }
    Some((names, label))
}

/// What a data disc holds, from its top folder: "dvd", "vcd", "svcd", "bluray" or "data"; and its volume name.
pub fn iso_kind(s: &dyn Sectors, base: u32) -> (&'static str, String) {
    let Some((names, label)) = iso_root(s, base) else { return ("data", String::new()) };
    let has = |n: &str| names.iter().any(|x| x == n);
    let k = if has("BDMV") { "bluray" } else if has("VIDEO_TS") { "dvd" } else if has("MPEG2") && (has("SVCD") || has("SEGMENT") || has("EXT")) { "svcd" }
        else if has("MPEGAV") { "vcd" } else { "data" };
    (k, label)
}

/// What a folder holds: "dvd" (VIDEO_TS), "bluray" (BDMV), "vcd" or "svcd" (a mounted Video CD), else "data".
pub fn folder_kind(dir: &Path) -> &'static str {
    let names: Vec<String> = std::fs::read_dir(dir).map(|r| r.flatten().map(|e| e.file_name().to_string_lossy().to_ascii_uppercase()).collect()).unwrap_or_default();
    let has = |n: &str| names.iter().any(|x| x == n);
    if has("BDMV") { "bluray" } else if has("VIDEO_TS") || dir.file_name().map(|n| n.to_string_lossy().eq_ignore_ascii_case("VIDEO_TS")).unwrap_or(false) { "dvd" }
    else if has("MPEG2") && (has("SVCD") || has("EXT")) { "svcd" } else if has("MPEGAV") { "vcd" } else { "data" }
}

/// The path names a disc: a drive, a .cue, an .iso, or a disc's folder (a mounted disc, a copied DVD).
pub fn looks_like_disc(p: &Path) -> bool {
    let s = p.to_string_lossy();
    if s.starts_with("/dev/sr") || s.starts_with("/dev/cdrom") || s.starts_with("/dev/dvd") { return true; }
    let e = p.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    if e == "cue" || e == "iso" { return true; }
    p.is_dir() && folder_kind(p) != "data"
}

// ------------------------------------------------------------------ tracks as files

/// An audio CD track as a WAV file (44 bytes of header, then the sound).
pub struct WavTrack { pub src: Arc<dyn Sectors>, pub start: u32, pub sectors: u32 }

pub fn wav_header(data_len: u32) -> [u8; 44] {
    let mut h = [0u8; 44];
    h[0..4].copy_from_slice(b"RIFF");
    h[4..8].copy_from_slice(&(data_len + 36).to_le_bytes());
    h[8..16].copy_from_slice(b"WAVEfmt ");
    h[16..20].copy_from_slice(&16u32.to_le_bytes());
    h[20..22].copy_from_slice(&1u16.to_le_bytes());
    h[22..24].copy_from_slice(&2u16.to_le_bytes());
    h[24..28].copy_from_slice(&44100u32.to_le_bytes());
    h[28..32].copy_from_slice(&176400u32.to_le_bytes());
    h[32..34].copy_from_slice(&4u16.to_le_bytes());
    h[34..36].copy_from_slice(&16u16.to_le_bytes());
    h[36..40].copy_from_slice(b"data");
    h[40..44].copy_from_slice(&data_len.to_le_bytes());
    h
}

impl crate::server::VFile for WavTrack {
    fn len(&self) -> u64 { 44 + self.sectors as u64 * RAW as u64 }
    fn mime(&self) -> &str { "audio/wav" }
    fn read_at(&self, off: u64, buf: &mut [u8]) -> io::Result<usize> {
        if off < 44 {
            let h = wav_header(self.sectors * RAW as u32);
            let k = (44 - off as usize).min(buf.len());
            buf[..k].copy_from_slice(&h[off as usize..off as usize + k]);
            return Ok(k);
        }
        let p = off - 44;
        let sector = (p / RAW as u64) as u32;
        if sector >= self.sectors { return Ok(0); }
        let skip = (p % RAW as u64) as usize;
        let n = ((buf.len() + skip).div_ceil(RAW) as u32).clamp(1, 24).min(self.sectors - sector);
        let b = self.src.raw(self.start + sector, n, true)?;
        if b.len() <= skip { return Ok(0); }
        let k = (b.len() - skip).min(buf.len());
        buf[..k].copy_from_slice(&b[skip..skip + k]);
        Ok(k)
    }
}

/// A Video CD track's MPEG: each Mode 2 sector's 2324 bytes after its 24 bytes of headers.
pub struct VcdTrack { pub src: Arc<dyn Sectors>, pub start: u32, pub sectors: u32 }

impl crate::server::VFile for VcdTrack {
    fn len(&self) -> u64 { self.sectors as u64 * FORM2 as u64 }
    fn mime(&self) -> &str { "video/mpeg" }
    fn read_at(&self, off: u64, buf: &mut [u8]) -> io::Result<usize> {
        let sector = (off / FORM2 as u64) as u32;
        if sector >= self.sectors { return Ok(0); }
        let skip = (off % FORM2 as u64) as usize;
        let n = ((buf.len() + skip).div_ceil(FORM2) as u32).clamp(1, 24).min(self.sectors - sector);
        let raw = self.src.raw(self.start + sector, n, false)?;
        let data: Vec<u8> = raw.chunks(RAW).flat_map(|s| s.get(24..24 + FORM2).unwrap_or(&[]).to_vec()).collect();
        if data.len() <= skip { return Ok(0); }
        let k = (data.len() - skip).min(buf.len());
        buf[..k].copy_from_slice(&data[skip..skip + k]);
        Ok(k)
    }
}

/// The time (seconds) an MPEG-1 or MPEG-2 pack header at the start of `b` says (its SCR).
pub fn pack_time(b: &[u8]) -> Option<f64> {
    let i = b.windows(4).take(FORM2).position(|w| w == [0, 0, 1, 0xBA])?;
    let p = b.get(i + 4..i + 10)?;
    let scr = if p[0] >> 6 == 1 {
        ((p[0] as u64 >> 3) & 7) << 30 | (p[0] as u64 & 3) << 28 | (p[1] as u64) << 20 | ((p[2] as u64) >> 3) << 15 | (p[2] as u64 & 3) << 13 | (p[3] as u64) << 5 | (p[4] as u64) >> 3
    } else if p[0] >> 4 == 2 {
        ((p[0] as u64 >> 1) & 7) << 30 | (p[1] as u64) << 22 | ((p[2] as u64) >> 1) << 15 | (p[3] as u64) << 7 | (p[4] as u64) >> 1
    } else { return None };
    Some(scr as f64 / 90000.0)
}

/// How long a VCD track plays: the last sector's time minus the first's (else 75 sectors a second).
pub fn vcd_duration(t: &VcdTrack) -> f64 {
    use crate::server::VFile;
    let mut first = None;
    let mut b = vec![0u8; FORM2 * 8];
    for s in 0..t.sectors.min(300) {
        if let Ok(n) = t.read_at(s as u64 * FORM2 as u64, &mut b[..FORM2]) { if let Some(x) = pack_time(&b[..n]) { first = Some(x); break; } }
    }
    let mut last = None;
    for back in 1..=t.sectors.min(600) {
        let s = t.sectors - back;
        if let Ok(n) = t.read_at(s as u64 * FORM2 as u64, &mut b[..FORM2]) { if let Some(x) = pack_time(&b[..n]) { last = Some(x); break; } }
    }
    match (first, last) { (Some(a), Some(b)) if b > a => b - a, _ => t.sectors as f64 / 75.0 }
}

// ------------------------------------------------------------------ disc events (udev), never polling

/// One uevent: a udev one (after udev added what it knows, "libudev" header) or the kernel's ("ACTION@path").
pub fn parse_uevent(b: &[u8]) -> HashMap<String, String> {
    let body: &[u8] = if b.starts_with(b"libudev\0") && b.len() >= 24 {
        let off = u32::from_ne_bytes([b[16], b[17], b[18], b[19]]) as usize;
        let len = u32::from_ne_bytes([b[20], b[21], b[22], b[23]]) as usize;
        b.get(off..off + len).unwrap_or(&[])
    } else { b };
    body.split(|c| *c == 0).filter_map(|kv| std::str::from_utf8(kv).ok()).filter_map(|kv| kv.split_once('=')).map(|(k, v)| (k.to_string(), v.to_string())).collect()
}

/// A disc went in or came out of a drive (or a drive came or went).
pub fn is_disc_event(p: &HashMap<String, String>) -> Option<String> {
    if p.get("SUBSYSTEM").map(|s| s != "block").unwrap_or(true) { return None; }
    let name = p.get("DEVNAME")?;
    let dev = if name.starts_with("/dev/") { name.clone() } else { format!("/dev/{name}") };
    if !dev.starts_with("/dev/sr") { return None; }
    let action = p.get("ACTION").map(String::as_str).unwrap_or("");
    let media = p.contains_key("DISK_MEDIA_CHANGE") || p.contains_key("ID_CDROM_MEDIA") || p.contains_key("DISK_EJECT_REQUEST");
    (matches!(action, "add" | "remove") || (action == "change" && media)).then_some(dev)
}

/// Calls `f` with the drive each time a disc goes in or comes out. A thread that waits on udev's netlink socket: it
/// sleeps in the kernel until something happens (nothing is checked over and over). False when there's no socket.
pub fn watch(f: impl Fn(String) + Send + 'static) -> bool {
    unsafe {
        let fd = libc::socket(libc::AF_NETLINK, libc::SOCK_DGRAM | libc::SOCK_CLOEXEC, 15 /* NETLINK_KOBJECT_UEVENT */);
        if fd < 0 { return false; }
        let mut bound = false;
        // udev's events (group 2) first: by then udev has added ID_CDROM_MEDIA; the kernel's (group 1) if that's refused.
        for group in [2u32, 1u32] {
            let mut sa: libc::sockaddr_nl = std::mem::zeroed();
            sa.nl_family = libc::AF_NETLINK as u16;
            sa.nl_groups = group;
            if libc::bind(fd, &sa as *const _ as *const libc::sockaddr, std::mem::size_of::<libc::sockaddr_nl>() as u32) == 0 { bound = true; break; }
        }
        if !bound { libc::close(fd); return false; }
        std::thread::Builder::new().name("disc-events".into()).stack_size(128 * 1024).spawn(move || {
            let mut buf = vec![0u8; 16384];
            loop {
                let n = libc::recv(fd, buf.as_mut_ptr() as *mut _, buf.len(), 0);
                if n < 0 {
                    match io::Error::last_os_error().raw_os_error() { Some(libc::EINTR) | Some(libc::ENOBUFS) => continue, _ => break }
                }
                if let Some(dev) = is_disc_event(&parse_uevent(&buf[..n as usize])) { f(dev); }
            }
        }).is_ok()
    }
}

// ------------------------------------------------------------------ the tray and the desktop's mounting

/// Mounts a data disc through udisks (as you; the desktop does the same) and says where.
pub fn mount(dev: &str) -> Result<String, String> {
    if let Some(m) = mount_of(dev) { return Ok(m); }
    let o = run("udisksctl", &["mount", "-b", dev, "--no-user-interaction"], 60).ok_or("Couldn't open the disc (udisks didn't answer).")?;
    if let Some(m) = mount_of(dev) { return Ok(m); }
    let text = String::from_utf8_lossy(&o.stdout).to_string() + &String::from_utf8_lossy(&o.stderr);
    text.split(" at ").nth(1).map(|s| s.trim().trim_end_matches('.').to_string()).filter(|s| Path::new(s).is_dir())
        .ok_or_else(|| format!("Couldn't open the disc: {}", text.lines().last().unwrap_or("").trim()))
}

/// Opens the tray: unmounts the disc first when the desktop mounted it.
pub fn eject(dev: &str) -> Result<(), String> {
    if !dev.starts_with("/dev/sr") && !dev.starts_with("/dev/cdrom") && !dev.starts_with("/dev/dvd") { return Err("That isn't a disc drive.".into()); }
    if mount_of(dev).is_some() {
        let _ = run("udisksctl", &["unmount", "-b", dev, "--no-user-interaction"], 30);
        if mount_of(dev).is_some() { return Err("The disc is still open in another app. Close it there and try again.".into()); }
    }
    if Device::open(dev).and_then(|d| d.eject()).is_ok() { return Ok(()); }
    match run("eject", &[dev], 30) {
        Some(o) if o.status.success() => Ok(()),
        _ => Err("The drive didn't open. Press the button on the drive.".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toc(offs: &[u32], leadout: u32) -> Toc {
        Toc { first: 1, last: offs.len() as u32, tracks: offs.iter().enumerate().map(|(i, o)| TocTrack { n: i as u32 + 1, lba: o - 150, audio: true, mode2: false }).collect(), leadout: leadout - 150 }
    }

    #[test]
    fn disc_ids() {
        // Worked out with Python's hashlib the same way (libdiscid's method).
        let t = toc(&[150, 44942, 61305, 72755, 96360, 130485], 242457);
        assert_eq!(musicbrainz_id(&t), "7lAd6g1Tu2xcmnh41d6rlq1cTAs-");
        assert_eq!(musicbrainz_toc(&t), "1+6+242457+150+44942+61305+72755+96360+130485");
        assert_eq!(format!("{:08x}", cddb_id(&t)), "5c0c9e06");
        let t2 = toc(&[150, 22767, 41887, 58317, 72102, 91375, 104652, 115380, 132165, 143932, 159870, 174597], 267257);
        assert_eq!(musicbrainz_id(&t2), "I5l9cCSFccLKFEKS.7wqSZAorPU-");
        assert_eq!(format!("{:08x}", cddb_id(&t2)), "a70de90c");
        // An Enhanced CD: the data track and the gap before it aren't the last song's.
        let mut e = t.clone();
        e.tracks.push(TocTrack { n: 7, lba: 250000, audio: false, mode2: true });
        e.leadout = 300000;
        assert_eq!(e.end(5), 250000 - 11400);
        assert_eq!(musicbrainz_toc(&e), format!("1+6+{}+150+44942+61305+72755+96360+130485", 250000 - 11400 + 150));
    }

    #[test]
    fn reads_toc_answer() {
        let mut b = vec![0, 26, 1, 2];
        for (n, ctl, lba) in [(1u8, 0x10u8, 0u32), (2, 0x14, 20000), (0xAA, 0x10, 30000)] { b.extend_from_slice(&[0, ctl, n, 0]); b.extend_from_slice(&lba.to_be_bytes()); }
        let t = parse_toc(&b).unwrap();
        assert_eq!(t.tracks.len(), 2);
        assert!(t.tracks[0].audio && !t.tracks[1].audio);
        assert_eq!(t.leadout, 30000);
    }

    #[test]
    fn reads_cd_text() {
        let mut packs = Vec::new();
        let mut text = |kind: u8, s: &[u8]| {
            for (seq, chunk) in s.chunks(12).enumerate() {
                let mut p = vec![kind, 0, (packs.len() as u8), 0];
                let mut c = chunk.to_vec(); c.resize(12, 0);
                p.extend_from_slice(&c); p.extend_from_slice(&[0, 0]);
                let _ = seq;
                packs.push(p);
            }
        };
        text(0x80, b"Kiln\0Ember song 1\0Ember song 2\0");
        text(0x81, b"Hearth Band\0\t\0Guest\0");
        let all: Vec<u8> = packs.concat();
        let t = parse_cd_text(&all).unwrap();
        assert_eq!(t.titles, vec!["Kiln", "Ember song 1", "Ember song 2"]);
        assert_eq!(t.performers, vec!["Hearth Band", "Hearth Band", "Guest"]);
    }

    #[test]
    fn reads_cue_sheets() {
        let d = std::env::temp_dir().join(format!("hp-cue-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("cd.bin"), vec![0u8; RAW * 3000]).unwrap();
        let cue = "PERFORMER \"Hearth Band\"\nTITLE \"Kiln\"\nFILE \"cd.bin\" BINARY\n  TRACK 01 AUDIO\n    TITLE \"Ember song 1\"\n    INDEX 01 00:00:00\n  TRACK 02 AUDIO\n    TITLE \"Ember song 2\"\n    PERFORMER \"Guest\"\n    INDEX 00 00:10:00\n    INDEX 01 00:12:00\n";
        let c = parse_cue(cue, &d).unwrap();
        assert_eq!(c.toc.tracks.iter().map(|t| t.lba).collect::<Vec<_>>(), vec![0, 900]);
        assert_eq!(c.toc.leadout, 3000);
        assert_eq!(c.text.titles, vec!["Kiln", "Ember song 1", "Ember song 2"]);
        assert_eq!(c.text.performers, vec!["Hearth Band", "", "Guest"]);
        assert!(!c.files[0].big_endian);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn wav_tracks_read_like_files() {
        use crate::server::VFile;
        struct Fake;
        impl Sectors for Fake {
            fn raw(&self, lba: u32, n: u32, _: bool) -> io::Result<Vec<u8>> { Ok((0..n).flat_map(|k| vec![((lba + k) % 256) as u8; RAW]).collect()) }
            fn data(&self, _: u32, _: u32) -> io::Result<Vec<u8>> { Ok(vec![]) }
        }
        let w = WavTrack { src: Arc::new(Fake), start: 100, sectors: 10 };
        assert_eq!(w.len(), 44 + 10 * RAW as u64);
        let mut b = vec![0u8; 50];
        assert_eq!(w.read_at(0, &mut b).unwrap(), 44);
        assert_eq!(&b[..4], b"RIFF");
        assert_eq!(u32::from_le_bytes([b[40], b[41], b[42], b[43]]), 10 * RAW as u32);
        let n = w.read_at(44 + RAW as u64 * 2 + 5, &mut b).unwrap();
        assert_eq!(n, 50);
        assert_eq!(b[0], 102);
        let v = VcdTrack { src: Arc::new(Fake), start: 7, sectors: 4 };
        assert_eq!(v.len(), 4 * FORM2 as u64);
        assert_eq!(v.read_at(FORM2 as u64 * 3, &mut b).unwrap(), 50);
        assert_eq!(b[0], 10);
        assert_eq!(v.read_at(FORM2 as u64 * 4, &mut b).unwrap(), 0);
    }

    #[test]
    fn pack_times() {
        // MPEG-2 pack header with SCR = 90000 * 2 (2 s).
        let scr: u64 = 180000;
        let p = [0x44 | (((scr >> 30) & 7) << 3) as u8 | ((scr >> 28) & 3) as u8, ((scr >> 20) & 0xff) as u8, ((((scr >> 15) & 0x1f) << 3) | 4 | ((scr >> 13) & 3)) as u8, ((scr >> 5) & 0xff) as u8, (((scr & 0x1f) << 3) | 4) as u8, 1];
        let mut b = vec![0, 0, 1, 0xBA];
        b.extend_from_slice(&p);
        assert!((pack_time(&b).unwrap() - 2.0).abs() < 1e-6);
        // MPEG-1: '0010' SCR[32..30] 1, SCR[29..15] 1, SCR[14..0] 1.
        let scr: u64 = 90000 * 3;
        let p1 = [0x21 | (((scr >> 30) & 7) << 1) as u8, ((scr >> 22) & 0xff) as u8, ((((scr >> 15) & 0x7f) << 1) | 1) as u8, ((scr >> 7) & 0xff) as u8, (((scr & 0x7f) << 1) | 1) as u8, 0x80];
        let mut b = vec![0, 0, 1, 0xBA];
        b.extend_from_slice(&p1);
        assert!((pack_time(&b).unwrap() - 3.0).abs() < 1e-6);
    }

    #[test]
    fn uevents() {
        let k = b"change@/devices/pci0000:00/ata2/host1/target1:0:0/1:0:0:0/block/sr0\0ACTION=change\0DEVNAME=sr0\0SUBSYSTEM=block\0DISK_MEDIA_CHANGE=1\0";
        assert_eq!(is_disc_event(&parse_uevent(k)), Some("/dev/sr0".into()));
        let mut u = b"libudev\0".to_vec();
        u.extend_from_slice(&0xfeedcafeu32.to_be_bytes());
        let props = b"ACTION=change\0SUBSYSTEM=block\0DEVNAME=/dev/sr1\0ID_CDROM_MEDIA=1\0ID_CDROM_MEDIA_DVD=1\0";
        u.extend_from_slice(&40u32.to_ne_bytes());
        u.extend_from_slice(&40u32.to_ne_bytes());
        u.extend_from_slice(&(props.len() as u32).to_ne_bytes());
        u.resize(40, 0);
        u.extend_from_slice(props);
        let p = parse_uevent(&u);
        assert_eq!(p.get("ID_CDROM_MEDIA_DVD").map(String::as_str), Some("1"));
        assert_eq!(is_disc_event(&p), Some("/dev/sr1".into()));
        assert_eq!(is_disc_event(&parse_uevent(b"change@/x\0ACTION=change\0DEVNAME=sda\0SUBSYSTEM=block\0DISK_MEDIA_CHANGE=1\0")), None);
        assert_eq!(is_disc_event(&parse_uevent(b"change@/x\0ACTION=change\0DEVNAME=sr0\0SUBSYSTEM=block\0")), None, "not about the disc");
    }

    #[test]
    fn kinds_and_labels() {
        let p = |s: &str| parse_props(s, '\n');
        assert_eq!(kind_from_props(&p("ID_CDROM=1\n")), "");
        assert_eq!(kind_from_props(&p("ID_CDROM_MEDIA=1\nID_CDROM_MEDIA_BD=1\n")), "bluray");
        assert_eq!(kind_from_props(&p("ID_CDROM_MEDIA=1\nID_CDROM_MEDIA_CD=1\nID_CDROM_MEDIA_TRACK_COUNT_AUDIO=12\n")), "audio-cd");
        assert_eq!(kind_from_props(&p("ID_CDROM_MEDIA=1\nID_CDROM_MEDIA_DVD=1\nID_CDROM_MEDIA_TRACK_COUNT_DATA=1\n")), "data");
        assert_eq!(kind_from_props(&p("ID_CDROM_MEDIA=1\nID_CDROM_MEDIA_STATE=blank\n")), "blank");
        assert_eq!(pretty_label("MY_FILM_DISC_1"), "My Film Disc 1");
        assert_eq!(pretty_label("Holiday 2019"), "Holiday 2019");
        assert_eq!(unescape_mount("/run/media/you/MY\\040DISC"), "/run/media/you/MY DISC");
        assert_eq!(unescape_udev("Film\\x20One"), "Film One");
        let d = std::env::temp_dir().join(format!("hp-kind-{}", std::process::id()));
        std::fs::create_dir_all(d.join("VIDEO_TS")).unwrap();
        assert_eq!(folder_kind(&d), "dvd");
        assert!(looks_like_disc(&d));
        std::fs::create_dir_all(d.join("BDMV")).unwrap();
        assert_eq!(folder_kind(&d), "bluray");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn names_from_musicbrainz_and_cddb() {
        let j: serde_json::Value = serde_json::from_str(r#"{"releases":[{"title":"Other","artist-credit":[{"name":"X","joinphrase":""}],"media":[{"discs":[],"tracks":[{"title":"a"},{"title":"b"}]}]},
          {"title":"Kiln","artist-credit":[{"name":"Hearth Band","joinphrase":" & "},{"name":"Friends","joinphrase":""}],"media":[{"discs":[{"id":"ID1"}],"tracks":[{"title":"One"},{"title":"Two","artist-credit":[{"name":"Guest","joinphrase":""}]}]}]}]}"#).unwrap();
        let n = parse_musicbrainz(&j, "ID1", 2).unwrap();
        assert_eq!(n.album, "Kiln");
        assert_eq!(n.artist, "Hearth Band & Friends");
        assert_eq!(n.tracks, vec![("One".into(), "Hearth Band & Friends".into()), ("Two".into(), "Guest".into())]);
        let x = parse_xmcd("# xmcd\nDISCID=5c0c9e06\nDTITLE=Hearth Band / Kiln\nTTITLE0=Ember song 1\nTTITLE1=Ember \nTTITLE1=song 2\n.\n").unwrap();
        assert_eq!((x.artist.as_str(), x.album.as_str()), ("Hearth Band", "Kiln"));
        assert_eq!(x.tracks[1].0, "Ember song 2");
    }
}
