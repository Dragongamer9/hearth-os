//! DVD-Video: the titles, chapters, sound tracks and subtitles a disc has (read from its IFO files, which are never
//! encrypted), and the title's video as one stream of 2048-byte sectors (its cells in order), for ffmpeg to turn into
//! something the web view plays.
//!
//! The disc is read with libdvdread (loaded when it's there, not linked: Hearth Player starts without it), which reads
//! a drive (/dev/sr0), an ISO file or a VIDEO_TS folder, and uses libdvdcss by itself when that is installed. Without
//! libdvdread, a VIDEO_TS folder (a copied disc, or a mounted one) is read straight from its files.
//!
//! Encrypted discs (CSS): the sectors of a film on most shop-bought DVDs are scrambled, and only libdvdcss can unscramble
//! them. Fedora doesn't ship libdvdcss (legal reasons): the player notices scrambled sectors (the PES scrambling bits
//! that libdvdcss clears once it has unscrambled one) and offers to add it (docs/PLAYER-DISCS.md).
//!
//! Disc menus: libdvdread reads the disc but doesn't run its menus (that's libdvdnav with a renderer of its own), so the
//! player shows the disc's titles and chapters as its menu instead.

use serde::Serialize;
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub const SECTOR: usize = 2048;

// ------------------------------------------------------------------ reading the disc

/// Where a DVD's files come from.
pub trait DvdIo: Send + Sync {
    /// VIDEO_TS.IFO (vts 0) or VTS_nn_0.IFO.
    fn ifo(&self, vts: u32) -> io::Result<Vec<u8>>;
    /// How many sectors the title set's VOBs (VTS_nn_1.VOB to VTS_nn_9.VOB, one after another) have.
    fn title_sectors(&self, vts: u32) -> io::Result<u64>;
    /// Reads `n` sectors of the title set's VOBs from `sector` into `out` (n * 2048 bytes); how many were read.
    fn read(&self, vts: u32, sector: u32, n: u32, out: &mut [u8]) -> io::Result<u32>;
    /// "libdvdread" or "folder".
    fn how(&self) -> &'static str;
}

/// A VIDEO_TS folder (or a folder holding one), read straight from its files.
pub struct Folder {
    dir: PathBuf,
    names: HashMap<String, PathBuf>,
}

impl Folder {
    pub fn open(path: &Path) -> io::Result<Folder> {
        let dir = video_ts(path).ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no VIDEO_TS folder there"))?;
        let mut names = HashMap::new();
        for e in std::fs::read_dir(&dir)?.flatten() {
            names.insert(e.file_name().to_string_lossy().to_ascii_uppercase(), e.path());
        }
        Ok(Folder { dir, names })
    }
    fn file(&self, name: &str) -> Option<&PathBuf> { self.names.get(name) }
    fn vobs(&self, vts: u32) -> Vec<(PathBuf, u64)> {
        (1..=9).filter_map(|i| self.file(&format!("VTS_{vts:02}_{i}.VOB")).map(|p| (p.clone(), std::fs::metadata(p).map(|m| m.len()).unwrap_or(0)))).collect()
    }
}

/// The VIDEO_TS folder in `path` (or `path` itself), any letter case.
pub fn video_ts(path: &Path) -> Option<PathBuf> {
    if path.file_name().map(|n| n.to_string_lossy().eq_ignore_ascii_case("VIDEO_TS")).unwrap_or(false) && path.is_dir() { return Some(path.to_path_buf()); }
    std::fs::read_dir(path).ok()?.flatten().map(|e| e.path()).find(|p| p.is_dir() && p.file_name().map(|n| n.to_string_lossy().eq_ignore_ascii_case("VIDEO_TS")).unwrap_or(false))
}

impl DvdIo for Folder {
    fn ifo(&self, vts: u32) -> io::Result<Vec<u8>> {
        let names = if vts == 0 { ["VIDEO_TS.IFO".to_string(), "VIDEO_TS.BUP".to_string()] } else { [format!("VTS_{vts:02}_0.IFO"), format!("VTS_{vts:02}_0.BUP")] };
        for n in &names { if let Some(p) = self.file(n) { if let Ok(b) = std::fs::read(p) { return Ok(b); } } }
        Err(io::Error::new(io::ErrorKind::NotFound, format!("{} isn't in {}", names[0], self.dir.display())))
    }
    fn title_sectors(&self, vts: u32) -> io::Result<u64> { Ok(self.vobs(vts).iter().map(|v| v.1 / SECTOR as u64).sum()) }
    fn read(&self, vts: u32, sector: u32, n: u32, out: &mut [u8]) -> io::Result<u32> {
        use std::io::{Read, Seek, SeekFrom};
        let mut pos = sector as u64 * SECTOR as u64;
        let want = n as usize * SECTOR;
        let mut got = 0usize;
        for (p, len) in self.vobs(vts) {
            if got >= want { break; }
            if pos >= len { pos -= len; continue; }
            let mut f = std::fs::File::open(&p)?;
            f.seek(SeekFrom::Start(pos))?;
            while got < want {
                let k = f.read(&mut out[got..want])?;
                if k == 0 { break; }
                got += k;
            }
            pos = 0;
        }
        Ok((got / SECTOR) as u32)
    }
    fn how(&self) -> &'static str { "folder" }
}

/// libdvdread, loaded at run time.
pub struct DvdRead {
    inner: Mutex<DvdReadInner>,
}

struct DvdReadInner {
    _lib: libloading::Library,
    reader: *mut std::ffi::c_void,
    files: HashMap<u32, *mut std::ffi::c_void>,
    open_file: unsafe extern "C" fn(*mut std::ffi::c_void, i32, i32) -> *mut std::ffi::c_void,
    close_file: unsafe extern "C" fn(*mut std::ffi::c_void),
    read_blocks: unsafe extern "C" fn(*mut std::ffi::c_void, i32, usize, *mut u8) -> isize,
    file_size: unsafe extern "C" fn(*mut std::ffi::c_void) -> isize,
    read_bytes: unsafe extern "C" fn(*mut std::ffi::c_void, *mut std::ffi::c_void, usize) -> isize,
    close: unsafe extern "C" fn(*mut std::ffi::c_void),
}

// libdvdread's handles are only ever used under the mutex.
unsafe impl Send for DvdReadInner {}

const DVD_READ_INFO_FILE: i32 = 0;
const DVD_READ_INFO_BACKUP_FILE: i32 = 1;
const DVD_READ_TITLE_VOBS: i32 = 3;

/// The names libdvdread goes by.
pub const DVDREAD_NAMES: &[&str] = &["libdvdread.so.8", "libdvdread.so.7", "libdvdread.so.4", "libdvdread.so"];
/// The names libdvdcss goes by (libdvdread loads it by itself; this is only to say whether it's there).
pub const DVDCSS_NAMES: &[&str] = &["libdvdcss.so.2", "libdvdcss.so"];

fn load_any(names: &[&str]) -> Option<libloading::Library> {
    names.iter().find_map(|n| unsafe { libloading::Library::new(n) }.ok())
}

/// libdvdread is installed.
pub fn have_dvdread() -> bool { load_any(DVDREAD_NAMES).is_some() }

/// libdvdcss (DVD decoding) is installed. HEARTH_PLAYER_NO_DVDCSS=1 pretends it isn't (tests).
pub fn have_dvdcss() -> bool {
    if std::env::var_os("HEARTH_PLAYER_NO_DVDCSS").is_some() { return false; }
    load_any(DVDCSS_NAMES).is_some()
}

impl DvdRead {
    pub fn open(target: &Path) -> io::Result<DvdRead> {
        let err = |s: &str| io::Error::new(io::ErrorKind::Other, s.to_string());
        let lib = load_any(DVDREAD_NAMES).ok_or_else(|| err("libdvdread isn't installed"))?;
        unsafe {
            let open: unsafe extern "C" fn(*const std::ffi::c_char) -> *mut std::ffi::c_void = *lib.get(b"DVDOpen\0").map_err(|e| err(&e.to_string()))?;
            macro_rules! sym { ($n:literal) => { *lib.get($n).map_err(|e| err(&e.to_string()))? } }
            type P = *mut std::ffi::c_void;
            let open_file: unsafe extern "C" fn(P, i32, i32) -> P = sym!(b"DVDOpenFile\0");
            let close_file: unsafe extern "C" fn(P) = sym!(b"DVDCloseFile\0");
            let read_blocks: unsafe extern "C" fn(P, i32, usize, *mut u8) -> isize = sym!(b"DVDReadBlocks\0");
            let file_size: unsafe extern "C" fn(P) -> isize = sym!(b"DVDFileSize\0");
            let read_bytes: unsafe extern "C" fn(P, P, usize) -> isize = sym!(b"DVDReadBytes\0");
            let close: unsafe extern "C" fn(P) = sym!(b"DVDClose\0");
            let c = std::ffi::CString::new(target.to_string_lossy().as_bytes()).map_err(|_| err("bad path"))?;
            let reader = open(c.as_ptr());
            if reader.is_null() { return Err(err("libdvdread couldn't open the disc")); }
            Ok(DvdRead { inner: Mutex::new(DvdReadInner { _lib: lib, reader, files: HashMap::new(), open_file, close_file, read_blocks, file_size, read_bytes, close }) })
        }
    }
}

impl Drop for DvdRead {
    fn drop(&mut self) {
        let i = self.inner.get_mut().unwrap();
        unsafe {
            for (_, f) in i.files.drain() { (i.close_file)(f); }
            (i.close)(i.reader);
        }
    }
}

impl DvdReadInner {
    fn title_file(&mut self, vts: u32) -> io::Result<*mut std::ffi::c_void> {
        if let Some(f) = self.files.get(&vts) { return Ok(*f); }
        let f = unsafe { (self.open_file)(self.reader, vts as i32, DVD_READ_TITLE_VOBS) };
        if f.is_null() { return Err(io::Error::new(io::ErrorKind::NotFound, format!("title set {vts} isn't on the disc"))); }
        self.files.insert(vts, f);
        Ok(f)
    }
}

impl DvdIo for DvdRead {
    fn ifo(&self, vts: u32) -> io::Result<Vec<u8>> {
        let i = self.inner.lock().unwrap();
        for dom in [DVD_READ_INFO_FILE, DVD_READ_INFO_BACKUP_FILE] {
            unsafe {
                let f = (i.open_file)(i.reader, vts as i32, dom);
                if f.is_null() { continue; }
                let blocks = (i.file_size)(f);
                let len = (blocks.max(0) as usize * SECTOR).min(64 << 20);
                let mut buf = vec![0u8; len];
                let n = (i.read_bytes)(f, buf.as_mut_ptr() as *mut _, len);
                (i.close_file)(f);
                if n > 0 { buf.truncate(n as usize); return Ok(buf); }
            }
        }
        Err(io::Error::new(io::ErrorKind::NotFound, "couldn't read the disc's IFO file"))
    }
    fn title_sectors(&self, vts: u32) -> io::Result<u64> {
        let mut i = self.inner.lock().unwrap();
        let f = i.title_file(vts)?;
        Ok(unsafe { (i.file_size)(f) }.max(0) as u64)
    }
    fn read(&self, vts: u32, sector: u32, n: u32, out: &mut [u8]) -> io::Result<u32> {
        let mut i = self.inner.lock().unwrap();
        let f = i.title_file(vts)?;
        let r = unsafe { (i.read_blocks)(f, sector as i32, n as usize, out.as_mut_ptr()) };
        if r < 0 { return Err(io::Error::new(io::ErrorKind::Other, "the disc couldn't be read there")); }
        Ok(r as u32)
    }
    fn how(&self) -> &'static str { "libdvdread" }
}

/// Opens a DVD: libdvdread when it's installed (a drive, an ISO file or a folder), else a VIDEO_TS folder's files.
pub fn open(target: &Path) -> io::Result<Arc<dyn DvdIo>> {
    if std::env::var_os("HEARTH_PLAYER_NO_DVDREAD").is_none() {
        if let Ok(d) = DvdRead::open(target) {
            if d.ifo(0).is_ok() { return Ok(Arc::new(d)); }
        }
    }
    Ok(Arc::new(Folder::open(target)?))
}

// ------------------------------------------------------------------ the IFO files

fn be16(b: &[u8], o: usize) -> u16 { b.get(o..o + 2).map(|x| u16::from_be_bytes([x[0], x[1]])).unwrap_or(0) }
fn be32(b: &[u8], o: usize) -> u32 { b.get(o..o + 4).map(|x| u32::from_be_bytes([x[0], x[1], x[2], x[3]])).unwrap_or(0) }
fn bcd(b: u8) -> u32 { ((b >> 4) as u32) * 10 + (b & 0x0f) as u32 }

/// A DVD time (hours, minutes, seconds, frames in BCD; the top two bits of the last byte say 25 or 29.97 frames).
pub fn dvd_time(b: &[u8]) -> f64 {
    if b.len() < 4 { return 0.0; }
    let fps = match b[3] >> 6 { 1 => 25.0, 3 => 30000.0 / 1001.0, _ => 30.0 };
    (bcd(b[0]) * 3600 + bcd(b[1]) * 60 + bcd(b[2])) as f64 + bcd(b[3] & 0x3f) as f64 / fps
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Track {
    /// The stream's number on the disc (ffmpeg's stream id: 0x80 AC-3, 0x88 DTS, 0xA0 PCM, 0xC0 MPEG; 0x20 subtitles).
    pub id: u32,
    pub label: String,
    pub lang: String,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Chapter { pub start: f64, pub end: f64, pub title: String }

/// One stretch of the title's video: VOB sectors first..=last, and how long it plays.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Cell { pub first: u32, pub last: u32, pub start: f64, pub duration: f64 }

#[derive(Serialize, Clone, Debug)]
pub struct Title {
    /// Its number on the disc (1, 2, ...).
    pub n: u32,
    pub vts: u32,
    pub duration: f64,
    pub chapters: Vec<Chapter>,
    pub audio: Vec<Track>,
    pub subs: Vec<Track>,
    pub angles: u32,
    #[serde(skip)]
    pub cells: Vec<Cell>,
    /// The subtitles' colours (16 × "rrggbb"), for ffmpeg.
    #[serde(skip)]
    pub palette: Vec<String>,
    /// The picture's size (720×480 NTSC, 720×576 PAL), for the subtitles.
    pub width: u32,
    pub height: u32,
    /// 16:9 (else 4:3).
    pub wide: bool,
}

impl Title {
    /// Sectors of video in the title.
    pub fn sectors(&self) -> u64 { self.cells.iter().map(|c| (c.last - c.first + 1) as u64).sum() }
}

fn lang_of(b: &[u8], o: usize) -> String {
    let (a, c) = (b.get(o).copied().unwrap_or(0), b.get(o + 1).copied().unwrap_or(0));
    if a.is_ascii_alphabetic() && c.is_ascii_alphabetic() { format!("{}{}", a as char, c as char).to_ascii_lowercase() } else { String::new() }
}

fn audio_track(attr: &[u8], ctl: u16, n: usize) -> Option<Track> {
    if ctl & 0x8000 == 0 { return None; }
    let s = ((ctl >> 8) & 0x7f) as u32;
    let mode = attr[0] >> 5;
    let (base, codec) = match mode { 0 => (0x80, "Dolby Digital"), 2 | 3 => (0xC0, "MPEG"), 4 => (0xA0, "PCM"), 6 => (0x88, "DTS"), _ => (0x80, "sound") };
    let ch = (attr[1] & 7) as u32 + 1;
    let lang = if (attr[0] >> 2) & 3 == 1 { lang_of(attr, 2) } else { String::new() };
    let name = if lang.is_empty() { format!("Sound {}", n + 1) } else { crate::media::lang_name(&lang) };
    let extra = match attr[5] { 2 => " (for the visually impaired)", 3 | 4 => " (director's comments)", _ => "" };
    let chs = match ch { 1 => "mono".to_string(), 2 => "stereo".to_string(), 6 => "5.1".to_string(), c => format!("{c} channels") };
    Some(Track { id: base + s, label: format!("{name}{extra} ({codec} {chs})"), lang })
}

fn sub_track(attr: &[u8], ctl: u32, n: usize, wide: bool) -> Option<Track> {
    if ctl & 0x8000_0000 == 0 { return None; }
    let s = if wide { (ctl >> 16) & 0x1f } else { (ctl >> 24) & 0x1f };
    let lang = if attr[0] & 3 == 1 { lang_of(attr, 2) } else { String::new() };
    let name = if lang.is_empty() { format!("Subtitles {}", n + 1) } else { crate::media::lang_name(&lang) };
    let extra = match attr[5] { 2 | 3 | 4 => " (large)", 5 | 6 => " (for children)", 9 => " (forced)", 13 | 14 | 15 => " (director's comments)", _ => "" };
    Some(Track { id: 0x20 + s, label: format!("{name}{extra}"), lang })
}

fn ycrcb_rgb(y: u8, cr: u8, cb: u8) -> String {
    let (y, cr, cb) = (y as f64, cr as f64 - 128.0, cb as f64 - 128.0);
    let c = |v: f64| v.round().clamp(0.0, 255.0) as u8;
    format!("{:02x}{:02x}{:02x}", c(y + 1.402 * cr), c(y - 0.344136 * cb - 0.714136 * cr), c(y + 1.772 * cb))
}

/// The PGC (program chain) at `o` in `b`: its programs' first cells, its cells, its palette and stream controls.
struct Pgc { programs: Vec<u8>, cells: Vec<(u8, f64, u32, u32)>, audio_ctl: Vec<u16>, sub_ctl: Vec<u32>, palette: Vec<String> }

fn pgc_at(b: &[u8], o: usize) -> Option<Pgc> {
    if o + 0xEC > b.len() { return None; }
    let np = b[o + 2] as usize;
    let nc = b[o + 3] as usize;
    let pm = be16(b, o + 0xE6) as usize;
    let cp = be16(b, o + 0xE8) as usize;
    let programs = (0..np).filter_map(|i| b.get(o + pm + i).copied()).collect();
    let mut cells = Vec::new();
    for i in 0..nc {
        let c = o + cp + i * 24;
        if c + 24 > b.len() { break; }
        cells.push((b[c], dvd_time(&b[c + 4..c + 8]), be32(b, c + 8), be32(b, c + 20)));
    }
    let audio_ctl = (0..8).map(|i| be16(b, o + 0x0C + i * 2)).collect();
    let sub_ctl = (0..32).map(|i| be32(b, o + 0x1C + i * 4)).collect();
    let palette = (0..16).map(|i| { let p = o + 0xA4 + i * 4; ycrcb_rgb(b[p + 1], b[p + 2], b[p + 3]) }).collect();
    Some(Pgc { programs, cells, audio_ctl, sub_ctl, palette })
}

/// Every title on the disc, from VIDEO_TS.IFO and each title set's IFO.
pub fn titles(io: &dyn DvdIo) -> io::Result<Vec<Title>> {
    let vmg = io.ifo(0)?;
    if !vmg.starts_with(b"DVDVIDEO-VMG") { return Err(io::Error::new(io::ErrorKind::InvalidData, "VIDEO_TS.IFO isn't a DVD-Video file")); }
    let tt = be32(&vmg, 0xC4) as usize * SECTOR;
    let count = be16(&vmg, tt) as usize;
    let mut vts_cache: HashMap<u32, Vec<u8>> = HashMap::new();
    let mut out = Vec::new();
    for i in 0..count {
        let e = tt + 8 + i * 12;
        if e + 12 > vmg.len() { break; }
        let angles = vmg[e + 1] as u32;
        let vts = vmg[e + 6] as u32;
        let ttn = vmg[e + 7] as usize;
        if vts == 0 || ttn == 0 { continue; }
        if !vts_cache.contains_key(&vts) {
            match io.ifo(vts) { Ok(b) if b.starts_with(b"DVDVIDEO-VTS") => { vts_cache.insert(vts, b); } _ => continue }
        }
        let v = &vts_cache[&vts];
        if let Some(t) = title_in_vts(v, (i + 1) as u32, vts, ttn, angles) { out.push(t); }
    }
    Ok(out)
}

fn title_in_vts(v: &[u8], n: u32, vts: u32, ttn: usize, angles: u32) -> Option<Title> {
    let ptt = be32(v, 0xC8) as usize * SECTOR;
    let pgcit = be32(v, 0xCC) as usize * SECTOR;
    let nt = be16(v, ptt) as usize;
    if ttn > nt { return None; }
    let end = be32(v, ptt + 4) as usize + 1;
    let start = be32(v, ptt + 8 + (ttn - 1) * 4) as usize;
    let stop = if ttn < nt { be32(v, ptt + 8 + ttn * 4) as usize } else { end };
    let mut parts = Vec::new();
    let mut p = start;
    while p + 4 <= stop && p + 4 <= v.len().saturating_sub(ptt) {
        parts.push((be16(v, ptt + p), be16(v, ptt + p + 2)));
        p += 4;
    }
    if parts.is_empty() { return None; }
    // Video: NTSC or PAL, 4:3 or 16:9.
    let va = be16(v, 0x200);
    let pal = (va >> 12) & 3 == 1;
    let wide = (va >> 10) & 3 == 3;
    let width = match (va >> 3) & 7 { 1 => 704, 2 | 3 => 352, _ => 720 };
    let height = if (va >> 3) & 7 == 3 { if pal { 288 } else { 240 } } else if pal { 576 } else { 480 };
    // The title's program chains in order (a title is nearly always one), their cells strung together.
    let mut pgcns: Vec<u16> = Vec::new();
    for (pgcn, _) in &parts { if !pgcns.contains(pgcn) { pgcns.push(*pgcn); } }
    let np = be16(v, pgcit) as usize;
    let mut cells = Vec::new();
    let mut first_pgc: Option<Pgc> = None;
    let mut cell_index: HashMap<(u16, usize), usize> = HashMap::new();
    let mut t = 0.0;
    for pgcn in &pgcns {
        if *pgcn == 0 || *pgcn as usize > np { continue; }
        let off = be32(v, pgcit + 8 + (*pgcn as usize - 1) * 8 + 4) as usize;
        let Some(pgc) = pgc_at(v, pgcit + off) else { continue };
        for (ci, (cat, dur, first, last)) in pgc.cells.iter().enumerate() {
            // Angle blocks: only the first angle's cells.
            let block_type = (cat >> 4) & 3;
            let block_mode = cat >> 6;
            if block_type == 1 && block_mode > 1 { continue; }
            cell_index.insert((*pgcn, ci + 1), cells.len());
            cells.push(Cell { first: *first, last: (*last).max(*first), start: t, duration: *dur });
            t += dur;
        }
        if first_pgc.is_none() { first_pgc = Some(pgc); }
    }
    let pgc0 = first_pgc?;
    // Chapters: each part of the title starts at its program's first cell.
    let mut chapters = Vec::new();
    for (k, (pgcn, pgn)) in parts.iter().enumerate() {
        let off = be32(v, pgcit + 8 + (*pgcn as usize).saturating_sub(1) * 8 + 4) as usize;
        let Some(pgc) = pgc_at(v, pgcit + off) else { continue };
        let Some(first_cell) = pgc.programs.get((*pgn as usize).saturating_sub(1)).copied() else { continue };
        let idx = (first_cell as usize..=pgc.cells.len()).find_map(|c| cell_index.get(&(*pgcn, c)).copied());
        if let Some(i) = idx { chapters.push(Chapter { start: cells[i].start, end: 0.0, title: format!("Chapter {}", k + 1) }); }
    }
    let total = t;
    for i in 0..chapters.len() { chapters[i].end = if i + 1 < chapters.len() { chapters[i + 1].start } else { total }; }
    let na = (be16(v, 0x202) as usize).min(8);
    let audio = (0..na).filter_map(|i| audio_track(&v[0x204 + i * 8..0x204 + i * 8 + 8], pgc0.audio_ctl[i], i)).collect();
    let ns = (be16(v, 0x254) as usize).min(32);
    let subs = (0..ns).filter_map(|i| sub_track(&v[0x256 + i * 6..0x256 + i * 6 + 6], pgc0.sub_ctl[i], i, wide)).collect();
    Some(Title { n, vts, duration: total, chapters, audio, subs, angles: angles.max(1), cells, palette: pgc0.palette, width, height, wide })
}

/// The title most people want: the longest one.
pub fn main_title(t: &[Title]) -> Option<u32> {
    t.iter().max_by(|a, b| a.duration.partial_cmp(&b.duration).unwrap_or(std::cmp::Ordering::Equal)).map(|t| t.n)
}

// ------------------------------------------------------------------ inside the video sectors

/// Where the first PES packet starts in an MPEG-2 pack (after the pack header and its stuffing).
fn pes_start(s: &[u8]) -> Option<usize> {
    if s.len() < 14 || s[0..4] != [0, 0, 1, 0xBA] || s[4] >> 6 != 1 { return None; }
    Some(14 + (s[13] & 7) as usize)
}

/// The sector holds a scrambled (CSS) packet: what libdvdcss unscrambles, and clears the bits of once it has.
pub fn scrambled(s: &[u8]) -> bool {
    let Some(p) = pes_start(s) else { return false };
    if p + 7 > s.len() || s[p..p + 3] != [0, 0, 1] { return false; }
    let id = s[p + 3];
    // Video, private stream 1 (sound, subtitles) and MPEG sound can be scrambled; navigation packs never are.
    let can = id == 0xBD || (0xC0..=0xEF).contains(&id);
    can && s[p + 6] >> 6 == 2 && s[p + 6] & 0x30 != 0
}

/// The sector starts a VOBU (a navigation pack: the system header, then the PCI packet).
pub fn is_nav(s: &[u8]) -> bool {
    pes_start(s).map(|p| s.get(p..p + 4) == Some(&[0, 0, 1, 0xBB])).unwrap_or(false)
}

/// The time into the cell a navigation pack says its VOBU starts at (DSI's c_eltm).
pub fn nav_cell_time(s: &[u8]) -> Option<f64> {
    // DSI from 0x407: nv_pck_scr, nv_pck_lbn, vobu_ea, three ref ends (4 bytes each), vob_idn (2), zero, c_idn, c_eltm.
    if !is_nav(s) || s.len() < 0x427 || s[0x400..0x404] != [0, 0, 1, 0xBF] || s[0x406] != 1 { return None; }
    Some(dvd_time(&s[0x423..0x427]))
}

/// Looks at the start of the title for scrambled sectors (a few hundred sectors: under a megabyte).
pub fn title_scrambled(io: &dyn DvdIo, t: &Title) -> bool {
    let mut buf = vec![0u8; 32 * SECTOR];
    let mut looked = 0;
    for c in &t.cells {
        let mut s = c.first;
        while s <= c.last && looked < 512 {
            let n = (c.last - s + 1).min(32);
            let got = io.read(t.vts, s, n, &mut buf).unwrap_or(0);
            if got == 0 { break; }
            if buf[..got as usize * SECTOR].chunks(SECTOR).any(scrambled) { return true; }
            looked += got;
            s += got;
        }
        if looked >= 512 { break; }
    }
    false
}

/// Where in the title to start for `at` seconds: (index into the title's sectors, the time it really starts at).
/// The guess goes by the cell's length, then moves forward to the next VOBU (whose first picture is a whole one) and
/// reads its real time.
pub fn seek_point(io: &dyn DvdIo, t: &Title, at: f64) -> (u64, f64) {
    if at <= 0.5 || t.cells.is_empty() { return (0, 0.0); }
    let mut before = 0u64;
    for c in &t.cells {
        let len = (c.last - c.first + 1) as u64;
        if at < c.start + c.duration || std::ptr::eq(c, t.cells.last().unwrap()) {
            let frac = if c.duration > 0.0 { ((at - c.start) / c.duration).clamp(0.0, 0.999) } else { 0.0 };
            let guess = (len as f64 * frac) as u64;
            let mut buf = vec![0u8; 16 * SECTOR];
            let mut s = guess;
            while s < len && s < guess + 1024 {
                let n = ((len - s) as u32).min(16);
                let got = io.read(t.vts, c.first + s as u32, n, &mut buf).unwrap_or(0);
                if got == 0 { break; }
                for (k, sec) in buf[..got as usize * SECTOR].chunks(SECTOR).enumerate() {
                    if is_nav(sec) {
                        let ct = nav_cell_time(sec).unwrap_or(c.duration * (s + k as u64) as f64 / len as f64);
                        return (before + s + k as u64, c.start + ct);
                    }
                }
                s += got as u64;
            }
            return (before + guess, at);
        }
        before += len;
    }
    (0, 0.0)
}

/// One sector holding an empty subtitle (SPU) for stream `id` (0x20...), with the pack header and time of `first` (the
/// title's first sector from where it plays). Put in front of the title, it lets ffmpeg know the subtitle stream from
/// the start: a DVD's subtitles otherwise only show up as a stream when the first one is said, which can be minutes in.
pub fn spu_primer(first: &[u8], id: u32) -> Option<Vec<u8>> {
    if first.len() < 14 || first[0..4] != [0, 0, 1, 0xBA] || first[4] >> 6 != 1 { return None; }
    let scr = (crate::disc::pack_time(&first[..14])? * 90000.0).round() as u64;
    let mut s = vec![0u8; SECTOR];
    s[..14].copy_from_slice(&first[..14]);
    s[13] = 0xF8; // no stuffing
    let pts = scr;
    let spu = [0x00, 0x0A, 0x00, 0x04, 0x00, 0x00, 0x00, 0x04, 0x02, 0xFF];
    let mut pes = vec![0, 0, 1, 0xBD, 0, 0, 0x81, 0x80, 5,
        0x21 | ((pts >> 29) & 0x0E) as u8, (pts >> 22) as u8, (((pts >> 14) & 0xFE) | 1) as u8, (pts >> 7) as u8, (((pts << 1) & 0xFE) | 1) as u8,
        id as u8];
    pes.extend_from_slice(&spu);
    let len = (pes.len() - 6) as u16;
    pes[4..6].copy_from_slice(&len.to_be_bytes());
    s[14..14 + pes.len()].copy_from_slice(&pes);
    // The rest of the sector is a padding packet.
    let at = 14 + pes.len();
    let pad = (SECTOR - at - 6) as u16;
    s[at..at + 4].copy_from_slice(&[0, 0, 1, 0xBE]);
    s[at + 4..at + 6].copy_from_slice(&pad.to_be_bytes());
    for b in &mut s[at + 6..] { *b = 0xFF; }
    Some(s)
}

/// The title's video, as one file of sectors (its cells one after another), from sector `from` on.
pub struct TitleFile {
    pub io: Arc<dyn DvdIo>,
    pub vts: u32,
    /// (first sector on the disc, sectors) for each part.
    pub runs: Vec<(u32, u64)>,
    /// Bytes in front of the title (an empty subtitle: `spu_primer`).
    pub prefix: Vec<u8>,
}

impl TitleFile {
    pub fn new(io: Arc<dyn DvdIo>, t: &Title, from: u64) -> TitleFile {
        let mut runs = Vec::new();
        let mut skip = from;
        for c in &t.cells {
            let len = (c.last - c.first + 1) as u64;
            if skip >= len { skip -= len; continue; }
            runs.push((c.first + skip as u32, len - skip));
            skip = 0;
        }
        TitleFile { io, vts: t.vts, runs, prefix: Vec::new() }
    }

    /// With an empty subtitle for stream `id` in front (see `spu_primer`).
    pub fn with_subtitle(mut self, id: u32) -> TitleFile {
        let mut first = vec![0u8; SECTOR];
        if let Some((s, _)) = self.runs.first() {
            if self.io.read(self.vts, *s, 1, &mut first).unwrap_or(0) == 1 {
                if let Some(p) = spu_primer(&first, id) { self.prefix = p; }
            }
        }
        self
    }
}

impl crate::server::VFile for TitleFile {
    fn len(&self) -> u64 { self.prefix.len() as u64 + self.runs.iter().map(|r| r.1).sum::<u64>() * SECTOR as u64 }
    fn mime(&self) -> &str { "video/mpeg" }
    fn read_at(&self, off: u64, buf: &mut [u8]) -> io::Result<usize> {
        let p = self.prefix.len() as u64;
        if off < p {
            let k = ((p - off) as usize).min(buf.len());
            buf[..k].copy_from_slice(&self.prefix[off as usize..off as usize + k]);
            return Ok(k);
        }
        let off = off - p;
        let mut sector = off / SECTOR as u64;
        let skip = (off % SECTOR as u64) as usize;
        for (first, len) in &self.runs {
            if sector >= *len { sector -= len; continue; }
            let n = ((len - sector) as usize).min((buf.len() + skip).div_ceil(SECTOR)).clamp(1, 64) as u32;
            let mut tmp = vec![0u8; n as usize * SECTOR];
            let got = self.io.read(self.vts, first + sector as u32, n, &mut tmp)? as usize * SECTOR;
            if got <= skip { return Ok(0); }
            let k = (got - skip).min(buf.len());
            buf[..k].copy_from_slice(&tmp[skip..skip + k]);
            return Ok(k);
        }
        Ok(0)
    }
}

/// What MPEG-2 picture coding extensions in `b` say: (progressive frames, interlaced frames or fields).
pub fn count_interlace(b: &[u8]) -> (u32, u32) {
    let (mut prog, mut inter) = (0, 0);
    let mut i = 0;
    while i + 9 <= b.len() {
        if b[i] == 0 && b[i + 1] == 0 && b[i + 2] == 1 && b[i + 3] == 0xB5 && b[i + 4] >> 4 == 8 {
            let structure = b[i + 6] & 3;
            let progressive = b[i + 8] >> 7 == 1;
            if progressive && structure == 3 { prog += 1 } else { inter += 1 }
            i += 9;
        } else { i += 1; }
    }
    (prog, inter)
}

/// The title's pictures are interlaced (a TV recording, not a film): a few hundred sectors from a tenth of the way in
/// are looked at (a film's frames are progressive even when they're shown at TV rate).
pub fn title_interlaced(io: Arc<dyn DvdIo>, t: &Title) -> bool {
    use crate::server::VFile;
    let f = TitleFile::new(io, t, t.sectors() / 10);
    let mut buf = vec![0u8; 64 * SECTOR];
    let (mut prog, mut inter) = (0, 0);
    let mut off = 0u64;
    while off < 400 * SECTOR as u64 {
        let n = f.read_at(off, &mut buf).unwrap_or(0);
        if n == 0 { break; }
        let (p, i) = count_interlace(&buf[..n]);
        prog += p; inter += i;
        off += n as u64;
    }
    inter >= 8 && inter > prog
}

/// A short stable name for a disc (its VIDEO_TS.IFO), for remembering where you stopped.
pub fn disc_key(io: &dyn DvdIo) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(io.ifo(0).unwrap_or_default());
    h.finalize().iter().take(8).map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
pub mod tests {
    use super::*;

    /// A small DVD's IFO files, written here (one title set, two titles: 3 chapters and 1), for the tests.
    pub fn fake_ifos() -> (Vec<u8>, Vec<u8>) {
        let mut vmg = vec![0u8; 2 * SECTOR];
        vmg[..12].copy_from_slice(b"DVDVIDEO-VMG");
        vmg[0xC4..0xC8].copy_from_slice(&1u32.to_be_bytes());
        let tt = SECTOR;
        vmg[tt..tt + 2].copy_from_slice(&2u16.to_be_bytes());
        for (i, (ch, ttn)) in [(3u16, 1u8), (1, 2)].iter().enumerate() {
            let e = tt + 8 + i * 12;
            vmg[e + 1] = 1;
            vmg[e + 2..e + 4].copy_from_slice(&ch.to_be_bytes());
            vmg[e + 6] = 1;
            vmg[e + 7] = *ttn;
        }
        let mut v = vec![0u8; 4 * SECTOR];
        v[..12].copy_from_slice(b"DVDVIDEO-VTS");
        v[0xC8..0xCC].copy_from_slice(&1u32.to_be_bytes());
        v[0xCC..0xD0].copy_from_slice(&2u32.to_be_bytes());
        v[0x200..0x202].copy_from_slice(&0x4C00u16.to_be_bytes()); // MPEG-2, NTSC, 16:9
        v[0x202..0x204].copy_from_slice(&2u16.to_be_bytes());
        v[0x204] = 0b0000_0100; v[0x205] = 5; v[0x206] = b'e'; v[0x207] = b'n'; v[0x209] = 1; // AC-3 5.1 English
        v[0x20C] = 0b0000_0100; v[0x20D] = 1; v[0x20E] = b'f'; v[0x20F] = b'r'; // AC-3 stereo French
        v[0x254..0x256].copy_from_slice(&1u16.to_be_bytes());
        v[0x256] = 1; v[0x258] = b'e'; v[0x259] = b'n';
        // PTT table: title 1 -> PGC 1 programs 1..3; title 2 -> PGC 2 program 1.
        let p = SECTOR;
        v[p..p + 2].copy_from_slice(&2u16.to_be_bytes());
        v[p + 4..p + 8].copy_from_slice(&(16u32 + 12 + 4 - 1).to_be_bytes());
        v[p + 8..p + 12].copy_from_slice(&16u32.to_be_bytes());
        v[p + 12..p + 16].copy_from_slice(&28u32.to_be_bytes());
        for (k, (pgcn, pgn)) in [(1u16, 1u16), (1, 2), (1, 3), (2, 1)].iter().enumerate() {
            let o = p + 16 + k * 4;
            v[o..o + 2].copy_from_slice(&pgcn.to_be_bytes());
            v[o + 2..o + 4].copy_from_slice(&pgn.to_be_bytes());
        }
        // PGC table with two PGCs.
        let g = 2 * SECTOR;
        v[g..g + 2].copy_from_slice(&2u16.to_be_bytes());
        let pgc = |v: &mut Vec<u8>, at: usize, cells: &[(u8, u8, u32, u32)], programs: &[u8]| {
            v[at + 2] = programs.len() as u8;
            v[at + 3] = cells.len() as u8;
            v[at + 0x0C..at + 0x0E].copy_from_slice(&0x8000u16.to_be_bytes());
            v[at + 0x0E..at + 0x10].copy_from_slice(&0x8100u16.to_be_bytes());
            v[at + 0x1C..at + 0x20].copy_from_slice(&0x8000_0000u32.to_be_bytes());
            for i in 0..16 { v[at + 0xA4 + i * 4 + 1] = 235; v[at + 0xA4 + i * 4 + 2] = 128; v[at + 0xA4 + i * 4 + 3] = 128; }
            v[at + 0xE6..at + 0xE8].copy_from_slice(&0xECu16.to_be_bytes());
            v[at + 0xE8..at + 0xEA].copy_from_slice(&0xF0u16.to_be_bytes());
            for (i, pr) in programs.iter().enumerate() { v[at + 0xEC + i] = *pr; }
            for (i, (m, s, a, b)) in cells.iter().enumerate() {
                let c = at + 0xF0 + i * 24;
                v[c + 5] = *m; v[c + 6] = *s; v[c + 7] = 0xC0; // 0:MM:SS.00 at 29.97
                v[c + 8..c + 12].copy_from_slice(&a.to_be_bytes());
                v[c + 20..c + 24].copy_from_slice(&b.to_be_bytes());
            }
        };
        v[g + 8] = 0x81; v[g + 12..g + 16].copy_from_slice(&0x18u32.to_be_bytes());
        v[g + 16] = 0x82; v[g + 20..g + 24].copy_from_slice(&0x200u32.to_be_bytes());
        pgc(&mut v, g + 0x18, &[(0, 0x05, 0, 99), (0, 0x05, 100, 199), (0, 0x10, 200, 399)], &[1, 2, 3]);
        pgc(&mut v, g + 0x200, &[(0, 0x04, 400, 479)], &[1]);
        (vmg, v)
    }

    struct Mem(Vec<u8>, Vec<u8>);
    impl DvdIo for Mem {
        fn ifo(&self, vts: u32) -> io::Result<Vec<u8>> { Ok(if vts == 0 { self.0.clone() } else { self.1.clone() }) }
        fn title_sectors(&self, _: u32) -> io::Result<u64> { Ok(480) }
        fn read(&self, _: u32, sector: u32, n: u32, out: &mut [u8]) -> io::Result<u32> {
            for k in 0..n as usize {
                let s = &mut out[k * SECTOR..(k + 1) * SECTOR];
                s.fill(0);
                s[0..4].copy_from_slice(&[0, 0, 1, 0xBA]); s[4] = 0x44; s[13] = 0xF8;
                // A navigation pack every 10 sectors, else video.
                if (sector as usize + k) % 10 == 0 { s[14..18].copy_from_slice(&[0, 0, 1, 0xBB]); s[0x400..0x404].copy_from_slice(&[0, 0, 1, 0xBF]); s[0x406] = 1; s[0x425] = 0x02; s[0x426] = 0xC0; }
                else { s[14..18].copy_from_slice(&[0, 0, 1, 0xE0]); s[20] = 0x80; s[21] = (sector as u8).wrapping_add(k as u8); }
            }
            Ok(n)
        }
        fn how(&self) -> &'static str { "memory" }
    }

    #[test]
    fn reads_titles_chapters_and_tracks() {
        let (vmg, vts) = fake_ifos();
        let io = Mem(vmg, vts);
        let t = titles(&io).unwrap();
        assert_eq!(t.len(), 2);
        assert_eq!(t[0].n, 1);
        assert!((t[0].duration - 20.0).abs() < 0.01, "{}", t[0].duration);
        assert_eq!(t[0].chapters.iter().map(|c| c.start.round() as u32).collect::<Vec<_>>(), vec![0, 5, 10]);
        assert_eq!(t[0].chapters[2].end.round() as u32, 20);
        assert_eq!(t[0].audio.len(), 2);
        assert_eq!(t[0].audio[0].id, 0x80);
        assert_eq!(t[0].audio[1].id, 0x81);
        assert_eq!(t[0].audio[0].lang, "en");
        assert!(t[0].audio[0].label.contains("5.1"), "{}", t[0].audio[0].label);
        assert_eq!(t[0].subs.len(), 1);
        assert_eq!(t[0].subs[0].id, 0x20);
        assert_eq!((t[0].width, t[0].height, t[0].wide), (720, 480, true));
        assert_eq!(t[0].palette[0], "ebebeb");
        assert_eq!(t[0].sectors(), 400);
        assert_eq!(t[1].sectors(), 80);
        assert_eq!(main_title(&t), Some(1));
    }

    #[test]
    fn scrambling_and_nav_packs() {
        let (vmg, vts) = fake_ifos();
        let io = Mem(vmg, vts);
        let mut s = vec![0u8; SECTOR];
        io.read(1, 1, 1, &mut s).unwrap();
        assert!(!scrambled(&s) && !is_nav(&s));
        s[20] |= 0x10;
        assert!(scrambled(&s), "PES scrambling bits set");
        io.read(1, 0, 1, &mut s).unwrap();
        assert!(is_nav(&s));
        assert!((nav_cell_time(&s).unwrap() - 2.0).abs() < 0.01);
        let t = titles(&io).unwrap();
        assert!(!title_scrambled(&io, &t[0]));
        // 7 s into title 1: cell 2 (5 s to 10 s), 40 % in: its sector 40 (disc sector 140, a nav pack), whose own time
        // (2 s into the cell) is where it really starts.
        let (sec, at) = seek_point(&io, &t[0], 7.0);
        assert_eq!(sec, 140);
        let (sec, _) = seek_point(&io, &t[0], 7.2);
        assert_eq!(sec, 150, "moves on to the next nav pack");
        assert!((at - 7.0).abs() < 0.01, "{at}");
        assert_eq!(seek_point(&io, &t[0], 0.0), (0, 0.0));
    }

    #[test]
    fn title_file_strings_cells_together() {
        use crate::server::VFile;
        let (vmg, vts) = fake_ifos();
        let io: Arc<dyn DvdIo> = Arc::new(Mem(vmg, vts));
        let t = titles(io.as_ref()).unwrap();
        let f = TitleFile::new(io.clone(), &t[0], 150);
        assert_eq!(f.runs, vec![(150, 50), (200, 200)]);
        assert_eq!(f.len(), 250 * SECTOR as u64);
        let mut b = vec![0u8; 10];
        assert_eq!(f.read_at(SECTOR as u64 + 20, &mut b).unwrap(), 10);
        assert_eq!(b[1], 151u8, "the second sector read is disc sector 151");
    }

    #[test]
    fn subtitle_primer() {
        let (vmg, vts) = fake_ifos();
        let io: Arc<dyn DvdIo> = Arc::new(Mem(vmg, vts));
        let t = titles(io.as_ref()).unwrap();
        let f = TitleFile::new(io.clone(), &t[0], 1).with_subtitle(0x21);
        assert_eq!(f.prefix.len(), SECTOR);
        let s = &f.prefix;
        assert_eq!(&s[..4], &[0, 0, 1, 0xBA]);
        assert_eq!(&s[14..18], &[0, 0, 1, 0xBD]);
        let len = u16::from_be_bytes([s[18], s[19]]) as usize;
        assert_eq!(s[14 + 6 + 3 + 5], 0x21, "the subtitle stream's id");
        assert_eq!(&s[14 + 6 + len..14 + 6 + len + 4], &[0, 0, 1, 0xBE], "padding after it");
        assert!(!scrambled(s));
        use crate::server::VFile;
        assert_eq!(f.len(), (SECTOR + 399 * SECTOR) as u64);
        let mut b = vec![0u8; 10];
        f.read_at(SECTOR as u64 + 20, &mut b).unwrap();
        assert_eq!(b[1], 1u8, "then the title's own first sector (disc sector 1)");
    }

    #[test]
    fn interlace_flags() {
        let ext = |structure: u8, prog: bool| vec![0, 0, 1, 0xB5, 0x8F, 0xFF, 0xF0 | structure, 0x80, if prog { 0x80 } else { 0 }];
        let mut b = Vec::new();
        for _ in 0..5 { b.extend(ext(3, true)); b.extend_from_slice(&[9, 9, 9]); }
        b.extend(ext(1, false));
        b.extend(ext(3, false));
        assert_eq!(count_interlace(&b), (5, 2));
    }

    #[test]
    fn times() {
        assert!((dvd_time(&[0x01, 0x30, 0x15, 0x40 | 0x12]) - (5415.0 + 12.0 / 25.0)).abs() < 1e-9);
        assert_eq!(ycrcb_rgb(16, 128, 128), "101010");
    }
}
