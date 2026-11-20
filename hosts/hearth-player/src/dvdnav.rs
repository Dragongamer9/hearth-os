//! A DVD's own menus, run by libdvdnav (the disc's virtual machine: menus, buttons, highlights), loaded at run time
//! (`dlopen`; without it, or on a locked disc, the player's titles screen is the menu).
//!
//! The menu's picture goes out as a stream of sectors that ffmpeg turns into JPEG frames (an MJPEG stream the page
//! shows as a picture: no video buffering, so a menu answers at once), with the menu's subpicture (its button labels and
//! graphics) drawn in. The buttons' places come from each navigation pack (its PCI) and the selected one from libdvdnav;
//! the page draws the highlight and sends the controller's, the remote's and the mouse's moves back here. When the disc
//! goes on to a title (a button "plays the film"), the page plays that title the usual way (seeking, sound, subtitles)
//! and this stream ends. Still menus wait here for a press (a condition variable: nothing runs meanwhile).

use serde::Serialize;
use serde_json::{json, Value};
use std::ffi::{c_char, c_void};
use std::io::{self, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

type P = *mut c_void;
pub const DVDNAV_NAMES: &[&str] = &["libdvdnav.so.4", "libdvdnav.so"];

/// libdvdnav is installed.
pub fn have_dvdnav() -> bool {
    if std::env::var_os("HEARTH_PLAYER_NO_DVDNAV").is_some() { return false; }
    DVDNAV_NAMES.iter().any(|n| unsafe { libloading::Library::new(n) }.is_ok())
}

const BLOCK_OK: i32 = 0;
const STILL_FRAME: i32 = 2;
const VTS_CHANGE: i32 = 5;
const CELL_CHANGE: i32 = 6;
const NAV_PACKET: i32 = 7;
const STOP: i32 = 8;
const HIGHLIGHT: i32 = 9;
const SPU_CLUT_CHANGE: i32 = 10;
const WAIT: i32 = 13;
const MENU_TITLE: i32 = 2;
const MENU_ROOT: i32 = 3;

struct Fns {
    next: unsafe extern "C" fn(P, *mut u8, *mut i32, *mut i32) -> i32,
    pci: unsafe extern "C" fn(P) -> P,
    up: unsafe extern "C" fn(P, P) -> i32,
    down: unsafe extern "C" fn(P, P) -> i32,
    left: unsafe extern "C" fn(P, P) -> i32,
    right: unsafe extern "C" fn(P, P) -> i32,
    activate: unsafe extern "C" fn(P, P) -> i32,
    mouse_select: unsafe extern "C" fn(P, P, i32, i32) -> i32,
    mouse_activate: unsafe extern "C" fn(P, P, i32, i32) -> i32,
    highlight: unsafe extern "C" fn(P, *mut i32) -> i32,
    still_skip: unsafe extern "C" fn(P) -> i32,
    wait_skip: unsafe extern "C" fn(P) -> i32,
    menu_call: unsafe extern "C" fn(P, i32) -> i32,
    title_info: unsafe extern "C" fn(P, *mut i32, *mut i32) -> i32,
    is_vts: unsafe extern "C" fn(P) -> i8,
    spu: unsafe extern "C" fn(P) -> i8,
    close: unsafe extern "C" fn(P) -> i32,
}

struct Nav { _lib: libloading::Library, h: P, f: Fns }
// libdvdnav's handle is only used under the session's mutex.
unsafe impl Send for Nav {}

impl Nav {
    fn open(target: &Path) -> io::Result<Nav> {
        let err = |s: &str| io::Error::new(io::ErrorKind::Other, s.to_string());
        let lib = DVDNAV_NAMES.iter().find_map(|n| unsafe { libloading::Library::new(n) }.ok()).ok_or_else(|| err("libdvdnav isn't installed"))?;
        unsafe {
            macro_rules! sym { ($n:literal) => { *lib.get($n).map_err(|e| err(&e.to_string()))? } }
            let open: unsafe extern "C" fn(*mut P, *const c_char) -> i32 = sym!(b"dvdnav_open\0");
            let readahead: unsafe extern "C" fn(P, i32) -> i32 = sym!(b"dvdnav_set_readahead_flag\0");
            let pgc_pos: unsafe extern "C" fn(P, i32) -> i32 = sym!(b"dvdnav_set_PGC_positioning_flag\0");
            let lang: unsafe extern "C" fn(P, *const c_char) -> i32 = sym!(b"dvdnav_menu_language_select\0");
            let f = Fns {
                next: sym!(b"dvdnav_get_next_block\0"), pci: sym!(b"dvdnav_get_current_nav_pci\0"),
                up: sym!(b"dvdnav_upper_button_select\0"), down: sym!(b"dvdnav_lower_button_select\0"),
                left: sym!(b"dvdnav_left_button_select\0"), right: sym!(b"dvdnav_right_button_select\0"),
                activate: sym!(b"dvdnav_button_activate\0"), mouse_select: sym!(b"dvdnav_mouse_select\0"),
                mouse_activate: sym!(b"dvdnav_mouse_activate\0"), highlight: sym!(b"dvdnav_get_current_highlight\0"),
                still_skip: sym!(b"dvdnav_still_skip\0"), wait_skip: sym!(b"dvdnav_wait_skip\0"),
                menu_call: sym!(b"dvdnav_menu_call\0"), title_info: sym!(b"dvdnav_current_title_info\0"),
                is_vts: sym!(b"dvdnav_is_domain_vts\0"), spu: sym!(b"dvdnav_get_active_spu_stream\0"), close: sym!(b"dvdnav_close\0"),
            };
            let c = std::ffi::CString::new(target.to_string_lossy().as_bytes()).map_err(|_| err("bad path"))?;
            let mut h: P = std::ptr::null_mut();
            if open(&mut h, c.as_ptr()) != 1 || h.is_null() { return Err(err("libdvdnav couldn't open the disc")); }
            readahead(h, 1);
            pgc_pos(h, 1);
            let en = std::ffi::CString::new("en").unwrap();
            lang(h, en.as_ptr());
            Ok(Nav { _lib: lib, h, f })
        }
    }
    fn next(&mut self, buf: &mut [u8; 2048]) -> (i32, usize) {
        let (mut ev, mut len) = (0i32, 0i32);
        let r = unsafe { (self.f.next)(self.h, buf.as_mut_ptr(), &mut ev, &mut len) };
        if r != 1 { (STOP, 0) } else { (ev, len.max(0) as usize) }
    }
    fn selected(&self) -> i32 { let mut b = 0; unsafe { (self.f.highlight)(self.h, &mut b) }; b }
    fn in_title(&self) -> Option<(i32, i32)> {
        if unsafe { (self.f.is_vts)(self.h) } != 1 { return None; }
        let (mut t, mut p) = (0, 0);
        unsafe { (self.f.title_info)(self.h, &mut t, &mut p) };
        (t > 0).then_some((t, p))
    }
}

impl Drop for Nav {
    fn drop(&mut self) { unsafe { (self.f.close)(self.h); } }
}

/// A button's rectangle on the menu's picture (its own pixels: 720 wide, 480 or 576 high).
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Button { pub x0: u16, pub y0: u16, pub x1: u16, pub y1: u16 }

/// The buttons a navigation pack's PCI describes (none when its highlight information is off).
pub fn pci_buttons(nav_sector: &[u8]) -> Vec<Button> {
    let p = 0x2D;
    if nav_sector.len() < p + 0x8E + 36 * 18 || !crate::dvd::is_nav(nav_sector) { return Vec::new(); }
    let pci = &nav_sector[p..];
    let hli_ss = u16::from_be_bytes([pci[0x60], pci[0x61]]);
    if hli_ss & 3 == 0 { return Vec::new(); }
    let btn_md = u16::from_be_bytes([pci[0x6E], pci[0x6F]]);
    let groups = ((btn_md >> 12) & 3).max(1) as usize;
    let n = (pci[0x71] as usize).min(36 / groups);
    (0..n).map(|i| {
        let b = &pci[0x8E + i * 18..0x8E + i * 18 + 6];
        Button {
            x0: (((b[0] & 0x3F) as u16) << 4) | (b[1] >> 4) as u16, x1: (((b[1] & 0x03) as u16) << 8) | b[2] as u16,
            y0: (((b[3] & 0x3F) as u16) << 4) | (b[4] >> 4) as u16, y1: (((b[4] & 0x03) as u16) << 8) | b[5] as u16,
        }
    }).collect()
}

/// What the page is told about the menu.
#[derive(Serialize, Clone, Debug, Default, PartialEq)]
pub struct MenuState { pub buttons: Vec<Button>, pub selected: i32 }

struct Shared {
    state: MenuState,
    /// A press came (wakes a still menu).
    kicked: bool,
    /// The sectors read before the stream started (the menu's first ones), sent first.
    pending: Vec<[u8; 2048]>,
    palette: Vec<String>,
    spu: u32,
    done: bool,
    /// A still menu waiting for a press.
    waiting: bool,
    /// The last sector sent (for the primer when the picture starts again).
    last: Option<[u8; 2048]>,
    /// The menu's sectors since its picture started (at most 4096: 8 MB), so its sound can start from the same place.
    history: std::collections::VecDeque<[u8; 2048]>,
    /// The menu's sound stream, while one is listening (its own ffmpeg: `menu_audio_args`).
    audio: Option<std::sync::mpsc::SyncSender<[u8; 2048]>>,
}

pub struct MenuSession {
    nav: Mutex<Nav>,
    sh: Mutex<Shared>,
    cv: Condvar,
    gen: AtomicU64,
    emit: Box<dyn Fn(Value) + Send + Sync>,
}

fn ycrcb(v: u32) -> String {
    let (y, cr, cb) = (((v >> 16) & 0xff) as f64, ((v >> 8) & 0xff) as f64 - 128.0, (v & 0xff) as f64 - 128.0);
    let c = |x: f64| x.round().clamp(0.0, 255.0) as u8;
    format!("{:02x}{:02x}{:02x}", c(y + 1.402 * cr), c(y - 0.344136 * cb - 0.714136 * cr), c(y + 1.772 * cb))
}

impl MenuSession {
    /// Opens the disc's menus: plays its first-play program (warnings are skipped by calling the menu) up to the
    /// menu's first sectors, so the palette and the buttons are known before the picture starts.
    pub fn open(target: &Path, emit: Box<dyn Fn(Value) + Send + Sync>) -> io::Result<Arc<MenuSession>> {
        let mut nav = Nav::open(target)?;
        let mut sh = Shared { state: MenuState::default(), kicked: false, pending: Vec::new(), palette: Vec::new(), spu: 0x20, done: false, waiting: false, last: None, history: Default::default(), audio: None };
        let mut buf = [0u8; 2048];
        let mut tried_menu = false;
        for _ in 0..20000 {
            let (ev, _) = nav.next(&mut buf);
            match ev {
                BLOCK_OK | NAV_PACKET => {
                    if nav.in_title().is_some() {
                        // Still in the first-play title (warnings, a logo): go to the menu instead, once.
                        if !tried_menu {
                            tried_menu = true;
                            let ok = unsafe { (nav.f.menu_call)(nav.h, MENU_ROOT) == 1 || (nav.f.menu_call)(nav.h, MENU_TITLE) == 1 };
                            if ok { continue; }
                        }
                        return Err(io::Error::new(io::ErrorKind::NotFound, "this DVD has no menu: it plays a title straight away"));
                    }
                    if ev == NAV_PACKET {
                        let b = pci_buttons(&buf);
                        if !b.is_empty() { sh.state.buttons = b; }
                    }
                    sh.pending.push(buf);
                    if sh.pending.len() >= 64 || (ev == NAV_PACKET && !sh.state.buttons.is_empty() && sh.pending.len() > 8) { break; }
                }
                SPU_CLUT_CHANGE => {
                    sh.palette = (0..16).map(|i| ycrcb(u32::from_ne_bytes([buf[i * 4], buf[i * 4 + 1], buf[i * 4 + 2], buf[i * 4 + 3]]))).collect();
                }
                STILL_FRAME => {
                    if !sh.pending.is_empty() { break; }
                    unsafe { (nav.f.still_skip)(nav.h) };
                }
                WAIT => { unsafe { (nav.f.wait_skip)(nav.h) }; }
                STOP => return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "the disc's menu ended straight away")),
                _ => {}
            }
        }
        if sh.pending.is_empty() { return Err(io::Error::new(io::ErrorKind::NotFound, "the disc's menu didn't start")); }
        let s = unsafe { (nav.f.spu)(nav.h) };
        if s >= 0 { sh.spu = 0x20 + (s as u32 & 0x1f); }
        sh.state.selected = nav.selected();
        Ok(Arc::new(MenuSession { nav: Mutex::new(nav), sh: Mutex::new(sh), cv: Condvar::new(), gen: AtomicU64::new(0), emit }))
    }

    pub fn state(&self) -> MenuState { self.sh.lock().unwrap().state.clone() }
    pub fn palette(&self) -> Vec<String> { self.sh.lock().unwrap().palette.clone() }
    pub fn spu(&self) -> u32 { self.sh.lock().unwrap().spu }

    /// A move or a press: "up", "down", "left", "right", "activate", "hover" or "click" (at x, y on the menu's picture),
    /// "root" or "title" (the disc's menus). Returns the menu as it is now.
    pub fn command(&self, cmd: &str, x: i32, y: i32) -> MenuState {
        {
            let nav = self.nav.lock().unwrap();
            let pci = unsafe { (nav.f.pci)(nav.h) };
            if !pci.is_null() {
                unsafe {
                    match cmd {
                        "up" => { (nav.f.up)(nav.h, pci); }
                        "down" => { (nav.f.down)(nav.h, pci); }
                        "left" => { (nav.f.left)(nav.h, pci); }
                        "right" => { (nav.f.right)(nav.h, pci); }
                        "activate" => { (nav.f.activate)(nav.h, pci); }
                        "hover" => { (nav.f.mouse_select)(nav.h, pci, x, y); }
                        "click" => { (nav.f.mouse_activate)(nav.h, pci, x, y); }
                        "root" => { (nav.f.menu_call)(nav.h, MENU_ROOT); }
                        "title" => { (nav.f.menu_call)(nav.h, MENU_TITLE); }
                        _ => {}
                    }
                }
            }
            let sel = nav.selected();
            let mut sh = self.sh.lock().unwrap();
            sh.state.selected = sel;
            if matches!(cmd, "activate" | "click" | "root" | "title") { sh.kicked = true; }
        }
        self.cv.notify_all();
        self.state()
    }

    /// Stops the picture and the sound (another stream, or the page left the menu, or a title starts).
    pub fn stop(&self) {
        self.gen.fetch_add(1, Ordering::SeqCst);
        { let mut sh = self.sh.lock().unwrap(); sh.done = true; sh.audio = None; }
        self.cv.notify_all();
    }

    /// Hands a sector to the sound stream (and keeps it for a sound stream that starts a moment later).
    fn tap(&self, b: &[u8; 2048]) {
        let mut sh = self.sh.lock().unwrap();
        if sh.history.len() >= 4096 { sh.history.pop_front(); }
        sh.history.push_back(*b);
        let gone = match &sh.audio { Some(tx) => matches!(tx.try_send(*b), Err(std::sync::mpsc::TrySendError::Disconnected(_))), None => false };
        if gone { sh.audio = None; }
    }

    /// The menu's sectors for its sound: from where the picture started, then as they come, until the menu stops.
    pub fn audio_source(self: &Arc<Self>) -> Arc<dyn crate::server::StreamSource> { Arc::new(MenuAudio(self.clone())) }

    /// A sound stream is listening (tests).
    pub fn audio_on(&self) -> bool { self.sh.lock().unwrap().audio.is_some() }

    fn changed(&self, f: impl FnOnce(&mut MenuState)) {
        let st = { let mut sh = self.sh.lock().unwrap(); let before = sh.state.clone(); f(&mut sh.state); (sh.state != before).then(|| sh.state.clone()) };
        if let Some(s) = st { (self.emit)(json!({"menu": s})); }
    }

    /// Waits until a press comes, the stream is stopped, or `secs` pass (None: no time limit).
    fn wait_press(&self, my: u64, secs: Option<u64>) {
        let start = std::time::Instant::now();
        let mut sh = self.sh.lock().unwrap();
        sh.waiting = true;
        loop {
            if sh.kicked || sh.done || self.gen.load(Ordering::SeqCst) != my { break; }
            if let Some(s) = secs { if start.elapsed() >= Duration::from_secs(s) { break; } }
            let left = secs.map(|s| Duration::from_secs(s).saturating_sub(start.elapsed())).unwrap_or(Duration::from_secs(3600));
            sh = self.cv.wait_timeout(sh, left).unwrap().0;
        }
        sh.kicked = false;
        sh.waiting = false;
    }

    /// The menu's sectors, as they come, until the disc goes on to a title (the page is told which), stops, or this
    /// stream is replaced. A primer sector first lets ffmpeg know the menu's subpicture stream at once.
    pub fn stream(&self, out: &mut dyn Write) -> io::Result<()> {
        let my = self.gen.fetch_add(1, Ordering::SeqCst) + 1;
        // A stream before this one waiting on a still menu stops waiting (and ends: it's no longer the newest).
        self.cv.notify_all();
        let (pending, spu, last) = { let mut sh = self.sh.lock().unwrap(); sh.done = false; sh.history.clear(); (std::mem::take(&mut sh.pending), sh.spu, sh.last) };
        if let Some(first) = pending.first().or(last.as_ref()) {
            if let Some(p) = crate::dvd::spu_primer(first, spu) { out.write_all(&p)?; }
        }
        for s in &pending { self.tap(s); out.write_all(s)?; }
        let mut buf = [0u8; 2048];
        loop {
            if self.gen.load(Ordering::SeqCst) != my { return Ok(()); }
            let (ev, len, title, sel) = {
                let mut nav = self.nav.lock().unwrap();
                let (ev, len) = nav.next(&mut buf);
                let title = if ev == VTS_CHANGE || ev == CELL_CHANGE || ev == BLOCK_OK { nav.in_title() } else { None };
                (ev, len, title, if ev == HIGHLIGHT || ev == NAV_PACKET { Some(nav.selected()) } else { None })
            };
            if let Some((t, p)) = title {
                // A title starts: the menu's sound stops at once (the page then plays the title).
                self.sh.lock().unwrap().audio = None;
                (self.emit)(json!({"play": {"title": t, "part": p}}));
                return Ok(());
            }
            match ev {
                BLOCK_OK => { let _ = len; self.tap(&buf); out.write_all(&buf)?; self.sh.lock().unwrap().last = Some(buf); }
                NAV_PACKET => {
                    let b = pci_buttons(&buf);
                    let sel = sel.unwrap_or(0);
                    self.changed(|s| { if !b.is_empty() { s.buttons = b; } s.selected = sel; });
                    self.tap(&buf);
                    out.write_all(&buf)?;
                }
                HIGHLIGHT => { let sel = sel.unwrap_or(0); self.changed(|s| s.selected = sel); }
                SPU_CLUT_CHANGE => {
                    let pal: Vec<String> = (0..16).map(|i| ycrcb(u32::from_ne_bytes([buf[i * 4], buf[i * 4 + 1], buf[i * 4 + 2], buf[i * 4 + 3]]))).collect();
                    let new = { let mut sh = self.sh.lock().unwrap(); let n = sh.palette != pal; sh.palette = pal; n };
                    // Another menu with its own colours: the picture starts again with them.
                    if new { (self.emit)(json!({"restart": true})); }
                }
                STILL_FRAME => {
                    out.flush()?;
                    let secs = i32::from_ne_bytes([buf[0], buf[1], buf[2], buf[3]]);
                    self.wait_press(my, if secs >= 255 { None } else { Some(secs.max(0) as u64) });
                    if self.gen.load(Ordering::SeqCst) != my { return Ok(()); }
                    let nav = self.nav.lock().unwrap();
                    unsafe { (nav.f.still_skip)(nav.h) };
                }
                WAIT => { let nav = self.nav.lock().unwrap(); unsafe { (nav.f.wait_skip)(nav.h) }; }
                STOP => { self.sh.lock().unwrap().audio = None; (self.emit)(json!({"end": true})); return Ok(()); }
                _ => {}
            }
        }
    }
}

/// The menu's sound: the same sectors as its picture, for a second ffmpeg that keeps only the sound.
struct MenuAudio(Arc<MenuSession>);

impl crate::server::StreamSource for MenuAudio {
    fn mime(&self) -> &str { "video/mpeg" }
    fn stream(&self, out: &mut dyn Write) -> io::Result<()> {
        let (tx, rx) = std::sync::mpsc::sync_channel::<[u8; 2048]>(1024);
        // What the picture has had so far, then the rest as it comes (in one go under the lock: nothing is missed).
        let before: Vec<[u8; 2048]> = {
            let mut sh = self.0.sh.lock().unwrap();
            if sh.done { return Ok(()); }
            sh.audio = Some(tx);
            sh.history.iter().copied().collect()
        };
        for b in &before { out.write_all(b)?; }
        // recv ends when the menu stops (the sender is dropped): ffmpeg then reaches the end and stops too.
        while let Ok(b) = rx.recv() { out.write_all(&b)?; }
        Ok(())
    }
}

impl crate::server::StreamSource for MenuSession {
    fn mime(&self) -> &str { "video/mpeg" }
    fn stream(&self, out: &mut dyn Write) -> io::Result<()> { MenuSession::stream(self, out) }
}

/// ffmpeg for the menu's picture: the sectors in, JPEG frames out as they're decoded (multipart, the page's <img>),
/// at the disc's own pace, the subpicture (buttons) drawn in, square pixels, every third frame of a moving menu.
pub fn menu_args(input: &str, palette: &[String], spu: u32) -> Vec<String> {
    let mut a: Vec<String> = ["-hide_banner", "-loglevel", "error", "-nostdin", "-re", "-fflags", "+genpts+discardcorrupt", "-probesize", "256k", "-analyzeduration", "500k"].map(String::from).to_vec();
    if palette.len() == 16 { a.extend(["-palette".into(), palette.join(",")]); }
    a.extend(["-canvas_size", "720x576"].map(String::from));
    a.extend(["-f".into(), "mpeg".into(), "-i".into(), input.to_string()]);
    a.extend(["-filter_complex".into(), format!("[0:v:0]scale=720:ih[b];[b][0:i:{spu:#x}]overlay=eof_action=repeat:repeatlast=1,mpdecimate,select='not(mod(n,3))',scale='trunc(ih*dar/2)*2':ih,setsar=1[v]"), "-map".into(), "[v]".into()]);
    a.extend(["-an", "-sn", "-fps_mode", "passthrough", "-c:v", "mjpeg", "-q:v", "5", "-f", "mpjpeg", "-flush_packets", "1", "pipe:1"].map(String::from));
    a
}

/// ffmpeg for the menu's sound: its first sound track at the disc's own pace, as Opus in Ogg (the page's <audio>).
pub fn menu_audio_args(input: &str, encoders: &[String]) -> Vec<String> {
    let mut a: Vec<String> = ["-hide_banner", "-loglevel", "error", "-nostdin", "-re", "-fflags", "+genpts+discardcorrupt", "-probesize", "512k", "-analyzeduration", "1M", "-f", "mpeg", "-i"].map(String::from).to_vec();
    a.push(input.to_string());
    a.extend(["-map", "0:a:0", "-vn", "-sn", "-ac", "2"].map(String::from));
    if encoders.iter().any(|e| e == "libopus") { a.extend(["-c:a", "libopus", "-b:a", "128k"].map(String::from)); }
    else { a.extend(["-c:a", "libvorbis", "-q:a", "5"].map(String::from)); }
    a.extend(["-f", "ogg", "-flush_packets", "1", "pipe:1"].map(String::from));
    a
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buttons_from_a_nav_pack() {
        let mut s = vec![0u8; 2048];
        s[0..4].copy_from_slice(&[0, 0, 1, 0xBA]); s[4] = 0x44; s[13] = 0xF8;
        s[14..18].copy_from_slice(&[0, 0, 1, 0xBB]);
        let pci = 0x2D;
        s[pci + 0x60..pci + 0x62].copy_from_slice(&1u16.to_be_bytes());
        s[pci + 0x6E..pci + 0x70].copy_from_slice(&0x1000u16.to_be_bytes());
        s[pci + 0x71] = 2;
        let put = |s: &mut Vec<u8>, i: usize, x0: u16, x1: u16, y0: u16, y1: u16| {
            let b = pci + 0x8E + i * 18;
            s[b] = (x0 >> 4) as u8 & 0x3F; s[b + 1] = ((x0 & 0xF) << 4) as u8 | (x1 >> 8) as u8 & 3; s[b + 2] = x1 as u8;
            s[b + 3] = (y0 >> 4) as u8 & 0x3F; s[b + 4] = ((y0 & 0xF) << 4) as u8 | (y1 >> 8) as u8 & 3; s[b + 5] = y1 as u8;
        };
        put(&mut s, 0, 160, 560, 300, 350);
        put(&mut s, 1, 160, 560, 370, 420);
        let b = pci_buttons(&s);
        assert_eq!(b, vec![Button { x0: 160, y0: 300, x1: 560, y1: 350 }, Button { x0: 160, y0: 370, x1: 560, y1: 420 }]);
        s[pci + 0x60] = 0; s[pci + 0x61] = 0;
        assert!(pci_buttons(&s).is_empty(), "highlight information off: no buttons");
        assert_eq!(ycrcb(0x00EB8080), "ebebeb");
    }

    #[test]
    fn menu_ffmpeg_line() {
        let pal: Vec<String> = (0..16).map(|_| "101010".into()).collect();
        let a = menu_args("http://x/m", &pal, 0x20).join(" ");
        assert!(a.contains("-re "), "{a}");
        assert!(a.contains("[b][0:i:0x20]overlay"), "{a}");
        assert!(a.contains("-c:v mjpeg") && a.ends_with("-f mpjpeg -flush_packets 1 pipe:1"), "{a}");
        assert!(a.contains("mpdecimate"), "a still picture is encoded once, not again and again: {a}");
        let s = menu_audio_args("http://x/m", &["libopus".to_string()]).join(" ");
        assert!(s.contains("-re ") && s.contains("-map 0:a:0 -vn") && s.contains("-c:a libopus") && s.ends_with("-f ogg -flush_packets 1 pipe:1"), "{s}");
    }
}
