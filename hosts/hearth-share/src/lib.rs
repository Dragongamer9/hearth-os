//! Hearth Share: show this screen on a TV, a phone, a tablet or another PC.
//!
//! Two ways, because there isn't one that does everything:
//!  - Moonlight (the best for games: sharp, smooth, with sound, and the other side can play). This is Hearth's own
//!    Remote play (Sunshine; hearth-core's remote.rs, the same as Settings › Remote play), shown here too: Moonlight
//!    runs on Android TV / Google TV, Fire TV, Apple TV, LG and Samsung TVs (webOS / Tizen ports), phones and PCs.
//!  - Any web browser on the same network (nothing to install: a TV's own browser, a phone, a laptop): Share
//!    serves the screen as a live MJPEG picture, plus the sound as an MP3 stream, from a link with a secret in it
//!    (and a QR code). Pictures, not video: slower to the eye than Moonlight and heavier on Wi-Fi, but every
//!    browser shows MJPEG with no player, and the delay is a fraction of a second. Only home-network addresses
//!    get an answer.
//! Not done: Miracast (Wi-Fi Direct plus a Miracast sink stack; GNOME Network Displays is the one Linux project) and
//! Chromecast (its protocol needs a Cast receiver app; the Default Media Receiver would play only HLS with seconds
//! of delay). A Google TV / Chromecast with Google TV runs Moonlight, which is better anyway.
//!
//! Capturing the screen for the browser stream: wf-recorder in Desktop Mode (Mantel), GStreamer reading gamescope's
//! PipeWire stream in Game Mode, ffmpeg's x11grab on X11. Each writes JPEG pictures that hearth-media's jpeg.rs
//! splits and http.rs hands to every viewer.

use hearth_media::env::Session;
use hearth_media::http::{self, Request};
use hearth_media::hub::{ByteHub, FrameHub};
use hearth_media::{dirs, jpeg, pulse, tools};
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::net::{IpAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

pub const PORT: u16 = 47800;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Options { pub height: u32, pub fps: u32, pub sound: bool }
impl Default for Options { fn default() -> Self { Options { height: 720, fps: 20, sound: true } } }

impl Options {
    pub fn clean(mut self) -> Self {
        if ![480, 720, 1080].contains(&self.height) { self.height = 720; }
        self.fps = self.fps.clamp(5, 30);
        self
    }
}

/// One program to run, and how its output arrives: a named pipe (wf-recorder) or its standard output.
#[derive(Debug, Clone, PartialEq)]
pub struct Capture { pub backend: &'static str, pub prog: String, pub args: Vec<String>, pub fifo: Option<PathBuf> }

fn s(v: &[&str]) -> Vec<String> { v.iter().map(|x| x.to_string()).collect() }

/// How to capture the screen as JPEG pictures in this kind of session.
pub fn capture(session: Session, o: &Options, display: &str, gamescope_node: Option<u32>, fifo: &std::path::Path, have: &dyn Fn(&str) -> bool) -> Result<Capture, String> {
    let h = o.height.to_string();
    let fps = o.fps.to_string();
    match session {
        Session::Wayland => {
            if !have("wf-recorder") { return Err("Sharing the desktop needs wf-recorder".into()); }
            let mut a = s(&["-y", "-r", &fps, "-c", "mjpeg", "-x", "yuvj420p", "-F", &format!("scale=-2:{h}"), "-p", "flags=+qscale", "-p", "global_quality=708", "-m", "image2pipe", "-f"]);
            a.push(fifo.to_string_lossy().to_string());
            Ok(Capture { backend: "wf-recorder", prog: "wf-recorder".into(), args: a, fifo: Some(fifo.to_path_buf()) })
        }
        Session::Gamescope => {
            if !have("gst-launch-1.0") { return Err("Sharing Game Mode needs GStreamer (gst-launch-1.0 and its PipeWire plugin)".into()); }
            let node = gamescope_node.ok_or("gamescope isn't sharing its picture (no PipeWire stream called gamescope)")?;
            let a = s(&["-q", "pipewiresrc", &format!("path={node}"), "do-timestamp=true", "!", "videoconvert", "!", "videoscale", "!",
                        &format!("video/x-raw,height={h},pixel-aspect-ratio=1/1"), "!", "videorate", "!", &format!("video/x-raw,framerate={fps}/1"), "!",
                        "jpegenc", "quality=80", "!", "fdsink", "fd=1"]);
            Ok(Capture { backend: "gstreamer", prog: "gst-launch-1.0".into(), args: a, fifo: None })
        }
        Session::X11 => {
            if !have("ffmpeg") { return Err("Sharing needs ffmpeg".into()); }
            let disp = if display.is_empty() { ":0" } else { display };
            let a = s(&["-hide_banner", "-loglevel", "error", "-nostdin", "-f", "x11grab", "-framerate", &fps, "-draw_mouse", "1", "-i", disp,
                        "-vf", &format!("scale=-2:{h}:flags=fast_bilinear,format=yuvj420p"), "-c:v", "mjpeg", "-q:v", "6", "-f", "image2pipe", "pipe:1"]);
            Ok(Capture { backend: "ffmpeg", prog: "ffmpeg".into(), args: a, fifo: None })
        }
        Session::None => Err("There's no screen to share here".into()),
    }
}

/// The sound: what's playing, as MP3 (else Ogg Opus) on standard output.
pub fn sound_args(source: &str, mp3: bool) -> (Vec<String>, &'static str) {
    let mut a = s(&["-hide_banner", "-loglevel", "error", "-nostdin", "-f", "pulse", "-fragment_size", "4096", "-i", source, "-ac", "2"]);
    if mp3 { a.extend(s(&["-c:a", "libmp3lame", "-b:a", "128k", "-f", "mp3", "pipe:1"])); (a, "audio/mpeg") }
    else { a.extend(s(&["-c:a", "libopus", "-b:a", "96k", "-f", "ogg", "-page_duration", "200000", "pipe:1"])); (a, "audio/ogg") }
}

/// The page a viewer opens (the link's secret is already in its address, so the streams are relative to it).
pub const VIEWER: &str = include_str!("viewer.html");

fn log_file() -> PathBuf { dirs::runtime("hearth-share").join("capture.log") }

/// A share that's on.
pub struct Sharing {
    pub token: String,
    pub port: u16,
    pub started: u64,
    pub backend: String,
    pub options: Options,
    pub frames: Arc<FrameHub>,
    pub sound: Arc<ByteHub>,
    pub sound_type: &'static str,
    pub has_sound: bool,
    pub viewers: Arc<AtomicUsize>,
    pub listeners: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
    kids: Vec<Child>,
    fifo: Option<PathBuf>,
    qr: Option<Option<String>>,
}

impl Sharing {
    /// Starts capturing and serving. `port` 0 tries Share's own (47800), then any free one.
    pub fn start(o: Options, port: u16) -> Result<Sharing, String> {
        let o = o.clean();
        let session = hearth_media::env::session();
        let node = if session == Session::Gamescope { gamescope_node() } else { None };
        let fifo_path = dirs::runtime("hearth-share").join("screen.mjpg");
        let cap = capture(session, &o, &std::env::var("DISPLAY").unwrap_or_default(), node, &fifo_path, &tools::have)?;
        let listener = bind(port)?;
        let port = listener.local_addr().map_err(|e| e.to_string())?.port();
        let frames = FrameHub::new();
        let sound = ByteHub::new();
        let stop = Arc::new(AtomicBool::new(false));
        let mut kids = vec![];
        let log = std::fs::File::create(log_file()).map_err(|e| e.to_string())?;
        let err = || log.try_clone().map(Stdio::from).unwrap_or_else(|_| Stdio::null());

        // The screen.
        if let Some(f) = &cap.fifo { tools::mkfifo(f)?; }
        let mut c = Command::new(&cap.prog);
        c.args(&cap.args).stdin(Stdio::null()).stderr(err()).stdout(if cap.fifo.is_some() { err() } else { Stdio::piped() });
        let mut child = c.spawn().map_err(|e| format!("Couldn't start {}: {e}", cap.prog))?;
        let fr = frames.clone();
        if let Some(f) = cap.fifo.clone() {
            std::thread::spawn(move || { if let Ok(file) = std::fs::File::open(&f) { jpeg::pump(file, move |x| fr.publish(x)); } });
        } else if let Some(out) = child.stdout.take() {
            std::thread::spawn(move || jpeg::pump(out, move |x| fr.publish(x)));
        }
        kids.push(child);

        // The sound (what's playing), when asked and possible.
        let audio_list = tools::run("ffmpeg", &["-hide_banner", "-encoders"], Duration::from_secs(8)).map(|o| o.stdout).unwrap_or_default();
        let mp3 = hearth_media::encoders::ffmpeg_has_encoder(&audio_list, "libmp3lame");
        let opus = hearth_media::encoders::ffmpeg_has_encoder(&audio_list, "libopus");
        let mut has_sound = false;
        let mut sound_type = "audio/mpeg";
        if o.sound && tools::have("ffmpeg") && (mp3 || opus) {
            let (args, t) = sound_args(&pulse::system_source(), mp3);
            sound_type = t;
            if let Ok(mut a) = Command::new("ffmpeg").args(&args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(err()).spawn() {
                if let Some(mut out) = a.stdout.take() {
                    let sh = sound.clone();
                    std::thread::spawn(move || {
                        let mut buf = vec![0u8; 4096];
                        loop { match out.read(&mut buf) { Ok(0) | Err(_) => break, Ok(n) => sh.push(buf[..n].to_vec()) } }
                    });
                }
                kids.push(a);
                has_sound = true;
            }
        }

        let mut sh = Sharing { token: tools::token(6), port, started: dirs::now_ms(), backend: cap.backend.into(), options: o, frames, sound, sound_type, has_sound,
                               viewers: Arc::new(AtomicUsize::new(0)), listeners: Arc::new(AtomicUsize::new(0)), stop, kids, fifo: cap.fifo, qr: None };
        // The first picture, or why there isn't one.
        let until = std::time::Instant::now() + Duration::from_secs(10);
        while sh.frames.seq() == 0 {
            if !sh.alive() { let t = tools::tail(&log_file(), 3); sh.halt(); return Err(format!("Couldn't capture the screen: {t}")); }
            if std::time::Instant::now() > until { let t = tools::tail(&log_file(), 3); sh.halt(); return Err(format!("The screen capture didn't send a picture. {t}")); }
            std::thread::sleep(Duration::from_millis(50));
        }
        sh.serve(listener);
        Ok(sh)
    }

    fn serve(&self, l: TcpListener) {
        let (token, frames, sound, stop, viewers, listeners) = (self.token.clone(), self.frames.clone(), self.sound.clone(), self.stop.clone(), self.viewers.clone(), self.listeners.clone());
        let (has_sound, stype) = (self.has_sound, self.sound_type);
        let stop2 = stop.clone();
        http::serve(l, stop, Arc::new(move |r: Request, mut s: TcpStream| {
            // Only this PC and home networks.
            if !r.peer.map(|p| http::is_local(&p.ip())).unwrap_or(false) { let _ = http::respond(&mut s, "403 Forbidden", "text/plain", b"Hearth Share only answers on your own network."); return; }
            let rest = r.path.strip_prefix('/').and_then(|p| p.strip_prefix(token.as_str()));
            match rest {
                Some("") => { let st = format!("301 Moved Permanently\r\nLocation: /{token}/"); let _ = http::respond(&mut s, &st, "text/plain", b""); }
                Some("/") => {
                    let page = VIEWER.replace("{{SOUND}}", if has_sound { "1" } else { "" }).replace("{{SOUNDFILE}}", if stype == "audio/ogg" { "live.ogg" } else { "live.mp3" });
                    let _ = http::respond(&mut s, "200 OK", "text/html; charset=utf-8", page.as_bytes());
                }
                Some("/live.mjpg") => http::stream_mjpeg(&mut s, &frames, &stop2, &viewers, Duration::from_secs(20)),
                Some("/frame.jpg") => match frames.latest() { Some((_, f)) => { let _ = http::respond(&mut s, "200 OK", "image/jpeg", &f); } None => { let _ = http::respond(&mut s, "503 Service Unavailable", "text/plain", b"no picture yet"); } },
                Some("/live.mp3") | Some("/live.ogg") if has_sound => http::stream_bytes(&mut s, &sound, stype, &stop2, &listeners),
                _ => { let _ = http::respond(&mut s, "404 Not Found", "text/html; charset=utf-8", b"<!doctype html><meta name=viewport content='width=device-width'><body style='background:#0f0b09;color:#f4ece3;font:18px system-ui;padding:2em'>Open the link shown in Hearth Share on the PC (it changes each time sharing starts)."); }
            }
        }));
    }

    pub fn alive(&mut self) -> bool { self.kids.first_mut().map(|c| matches!(c.try_wait(), Ok(None))).unwrap_or(false) }

    pub fn halt(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.frames.close();
        for k in self.kids.iter_mut() { tools::stop_child(k, Duration::from_secs(3)); }
        self.kids.clear();
        if let Some(f) = self.fifo.take() {
            // A reader still waiting at the pipe (the capture never started) is let go: open it once to write.
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                let _ = std::fs::OpenOptions::new().write(true).custom_flags(0o4000).open(&f);
            }
            let _ = std::fs::remove_file(f);
        }
    }

    /// The first link as a QR code (made once).
    pub fn qr(&mut self) -> Option<String> {
        if self.qr.is_none() { self.qr = Some(self.urls().first().and_then(|u| qr_svg(u))); }
        self.qr.clone().flatten()
    }

    /// The links to open on the other device, one per network address.
    pub fn urls(&self) -> Vec<String> { addresses().into_iter().map(|a| url(&a, self.port, &self.token)).collect() }
}

impl Drop for Sharing {
    fn drop(&mut self) { self.halt(); }
}

pub fn url(addr: &str, port: u16, token: &str) -> String {
    let host = if addr.contains(':') { format!("[{addr}]") } else { addr.to_string() };
    format!("http://{host}:{port}/{token}/")
}

fn bind(port: u16) -> Result<TcpListener, String> {
    if port != 0 { return TcpListener::bind(("0.0.0.0", port)).map_err(|e| format!("Port {port} is taken: {e}")); }
    TcpListener::bind(("0.0.0.0", PORT)).or_else(|_| TcpListener::bind(("0.0.0.0", 0))).map_err(|e| e.to_string())
}

/// This PC's home-network addresses (the same list Remote play shows).
pub fn addresses() -> Vec<String> {
    #[cfg(target_os = "linux")]
    let mut v = hearth_core::remote::addresses();
    #[cfg(not(target_os = "linux"))]
    let mut v: Vec<String> = vec![];
    v.retain(|a| a.parse::<IpAddr>().map(|ip| http::is_local(&ip) && !ip.is_loopback()).unwrap_or(false));
    if v.is_empty() { v.push("127.0.0.1".into()); }
    v
}

/// gamescope's PipeWire stream.
pub fn gamescope_node() -> Option<u32> {
    let o = tools::run("pw-dump", &[], Duration::from_secs(5)).ok()?;
    let v: serde_json::Value = serde_json::from_str(&o.stdout).ok()?;
    v.as_array()?.iter().find(|n| n["type"] == "PipeWire:Interface:Node" && n["info"]["props"]["node.name"] == "gamescope")
        .and_then(|n| n["id"].as_u64()).map(|n| n as u32)
}

/// A QR code of the link as SVG (qrencode), for a phone's camera.
pub fn qr_svg(text: &str) -> Option<String> {
    let o = tools::run("qrencode", &["-t", "SVG", "-m", "1", "-o", "-", text], Duration::from_secs(5)).ok()?;
    if o.ok() && o.stdout.contains("<svg") { Some(o.stdout[o.stdout.find("<svg")?..].to_string()) } else { None }
}

// ------------------------------------------------------------------ Moonlight: Hearth's Remote play

#[cfg(target_os = "linux")]
pub mod moonlight {
    use serde_json::Value;
    pub fn status() -> Value { hearth_core::remote::status() }
    pub fn set_on(on: bool) -> Result<Value, String> { hearth_core::remote::set_on(on) }
    pub fn pair(pin: &str, name: &str) -> Result<bool, String> { hearth_core::remote::pair(pin, name) }
    pub fn unpair(uuid: &str) -> Result<bool, String> { hearth_core::remote::unpair(uuid) }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn captures() {
        let all = |_: &str| true;
        let none = |_: &str| false;
        let f = std::path::Path::new("/run/s/screen.mjpg");
        let o = Options::default();
        let c = capture(Session::Wayland, &o, "", None, f, &all).unwrap();
        assert_eq!(c.args.join(" "), "-y -r 20 -c mjpeg -x yuvj420p -F scale=-2:720 -p flags=+qscale -p global_quality=708 -m image2pipe -f /run/s/screen.mjpg");
        assert_eq!(c.fifo.as_deref(), Some(f));
        assert!(capture(Session::Wayland, &o, "", None, f, &none).is_err());
        let c = capture(Session::X11, &Options { height: 1080, fps: 30, sound: false }, ":99", None, f, &all).unwrap();
        assert_eq!(c.args.join(" "), "-hide_banner -loglevel error -nostdin -f x11grab -framerate 30 -draw_mouse 1 -i :99 -vf scale=-2:1080:flags=fast_bilinear,format=yuvj420p -c:v mjpeg -q:v 6 -f image2pipe pipe:1");
        assert!(capture(Session::Gamescope, &o, "", None, f, &all).unwrap_err().contains("PipeWire"));
        let c = capture(Session::Gamescope, &o, "", Some(51), f, &all).unwrap();
        assert!(c.args.join(" ").starts_with("-q pipewiresrc path=51 do-timestamp=true ! videoconvert ! videoscale ! video/x-raw,height=720,pixel-aspect-ratio=1/1 ! videorate ! video/x-raw,framerate=20/1 ! jpegenc"));
        assert!(capture(Session::None, &o, "", None, f, &all).is_err());
    }
    #[test]
    fn links_and_options() {
        assert_eq!(url("192.168.1.5", 47800, "ab12"), "http://192.168.1.5:47800/ab12/");
        assert_eq!(url("fd00::5", 47800, "ab12"), "http://[fd00::5]:47800/ab12/");
        let o = Options { height: 999, fps: 100, sound: true }.clean();
        assert_eq!((o.height, o.fps), (720, 30));
        let (a, t) = sound_args("speakers.monitor", true);
        assert_eq!(t, "audio/mpeg");
        assert!(a.join(" ").ends_with("-c:a libmp3lame -b:a 128k -f mp3 pipe:1"));
        assert!(VIEWER.contains("live.mjpg") && VIEWER.contains("{{SOUND}}"));
    }
}
