//! Hearth Camera: photos and videos from the webcam.
//! How: ffmpeg reads the camera (V4L2, /dev/video*) and writes JPEG pictures to a pipe; they're served to the
//! window as a live MJPEG picture from 127.0.0.1 (hearth-media's http.rs), which works the same in WebKitGTK as
//! in any browser (WebKitGTK's own camera access needs permission handling Tauri doesn't have on Linux).
//! A photo is the newest picture, saved as it is. Recording a video starts ffmpeg again with a second output: the
//! MP4 (graphics card encoder when there's one, hearth-media's encoders.rs) with the microphone.
//! Photos go to ~/Pictures/Camera, videos to ~/Videos/Camera.
//! HEARTH_CAMERA_TEST=1 adds a test camera (ffmpeg's test picture), for CI and for trying it without a webcam.
//! Not covered: cameras that only work through libcamera / PipeWire (some new laptops' MIPI cameras, e.g. Intel
//! IPU6); those need GStreamer's pipewiresrc, a later step.

use hearth_media::encoders::{self, Quality};
use hearth_media::hub::FrameHub;
use hearth_media::{dirs, jpeg, tools};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::Duration;

pub const APP: &str = "Camera";
pub const ICON: &str = "hearth-camera";
pub const TEST_ID: &str = "test";

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Mode { pub fmt: String, pub w: u32, pub h: u32 }

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Camera { pub id: String, pub name: String, pub modes: Vec<Mode> }

pub fn photos_dir() -> PathBuf { dirs::pictures().join("Camera") }
pub fn videos_dir() -> PathBuf { dirs::videos().join("Camera") }
pub fn dirs() -> Vec<PathBuf> { vec![photos_dir(), videos_dir()] }

/// The picture formats and sizes ffmpeg lists for a camera (`ffmpeg -f v4l2 -list_formats all -i /dev/videoN`):
///   [video4linux2,v4l2 @ 0x5] Compressed:       mjpeg :          Motion-JPEG : 1280x720 640x480
///   [video4linux2,v4l2 @ 0x5] Raw       :     yuyv422 :           YUYV 4:2:2 : 640x480 1280x720
pub fn parse_formats(text: &str) -> Vec<Mode> {
    let mut v = vec![];
    for line in text.lines() {
        if !(line.contains("Compressed") || line.contains("Raw")) { continue; }
        let Some(rest) = line.split_once(']').map(|x| x.1) else { continue };
        let parts: Vec<&str> = rest.splitn(4, ':').collect();
        if parts.len() < 4 { continue; }
        let fmt = parts[1].trim();
        if fmt.is_empty() || fmt.contains(' ') { continue; }
        for size in parts[3].split_whitespace() {
            let Some((w, h)) = size.split_once('x') else { continue };
            if let (Ok(w), Ok(h)) = (w.parse(), h.parse()) { v.push(Mode { fmt: fmt.to_string(), w, h }); }
        }
    }
    v
}

/// The best picture to use: Motion-JPEG as large as 1920x1080 (cameras send it at full speed), else raw up to
/// 1280x720 (raw at larger sizes is often only 5 or 10 pictures a second).
pub fn best_mode(modes: &[Mode]) -> Option<Mode> {
    let pick = |fmt: &dyn Fn(&str) -> bool, maxw: u32| modes.iter().filter(|m| fmt(&m.fmt) && m.w <= maxw && m.h <= 1080).max_by_key(|m| m.w * m.h).cloned();
    pick(&|f| f == "mjpeg", 1920).or_else(|| pick(&|f| f != "mjpeg" && f != "h264", 1280)).or_else(|| modes.first().cloned())
}

fn sys_name(dev: &str) -> Option<String> {
    let n = Path::new(dev).file_name()?.to_string_lossy().to_string();
    std::fs::read_to_string(format!("/sys/class/video4linux/{n}/name")).ok().map(|s| s.trim().to_string())
}

/// The cameras here (each camera's first node; the others are its metadata).
pub fn cameras() -> Vec<Camera> {
    let mut nodes: Vec<String> = std::fs::read_dir("/sys/class/video4linux").into_iter().flatten().flatten()
        .map(|e| e.file_name().to_string_lossy().to_string()).filter(|n| n.starts_with("video"))
        .filter(|n| std::fs::read_to_string(format!("/sys/class/video4linux/{n}/index")).map(|i| i.trim() == "0").unwrap_or(true))
        .map(|n| format!("/dev/{n}")).collect();
    nodes.sort_by_key(|n| n.trim_start_matches("/dev/video").parse::<u32>().unwrap_or(999));
    let mut v = vec![];
    for dev in nodes {
        let o = tools::run("ffmpeg", &["-hide_banner", "-f", "v4l2", "-list_formats", "all", "-i", &dev], Duration::from_secs(8));
        let modes = o.map(|o| parse_formats(&o.stderr)).unwrap_or_default();
        if modes.is_empty() { continue; }
        v.push(Camera { name: sys_name(&dev).unwrap_or_else(|| dev.clone()), id: dev, modes });
    }
    if std::env::var("HEARTH_CAMERA_TEST").map(|x| x == "1").unwrap_or(false) {
        v.push(Camera { id: TEST_ID.into(), name: "Test camera".into(), modes: vec![Mode { fmt: "test".into(), w: 1280, h: 720 }] });
    }
    v
}

/// How a recording is encoded: ffmpeg's global options (VA-API's device), the filters on the recorded picture,
/// and the encoder's options.
#[derive(Clone, Debug)]
pub struct Rec { pub file: PathBuf, pub pre: Vec<String>, pub filters: String, pub enc: Vec<String>, pub mic: Option<String> }

/// Turns hearth-media's encoder arguments into a Rec's parts (its -vf goes into the filter graph instead).
pub fn rec_from(file: PathBuf, a: encoders::FfArgs, mic: Option<String>) -> Rec {
    let mut enc = vec![];
    let mut filters = String::new();
    let mut it = a.post.into_iter();
    while let Some(x) = it.next() {
        if x == "-vf" { filters = it.next().unwrap_or_default(); } else { enc.push(x); }
    }
    Rec { file, pre: a.pre, filters, enc, mic }
}

fn s(v: &[&str]) -> Vec<String> { v.iter().map(|x| x.to_string()).collect() }

/// ffmpeg's whole command line: the camera in, JPEG pictures out on standard output, and the video file too when recording.
pub fn ffmpeg_args(cam: &str, mode: &Mode, mirror: bool, rec: Option<&Rec>) -> Vec<String> {
    let mut a = s(&["-hide_banner", "-loglevel", "error", "-nostdin", "-y"]);
    if let Some(r) = rec { a.extend(r.pre.clone()); }
    if cam == TEST_ID {
        a.extend(s(&["-re", "-f", "lavfi", "-i", &format!("testsrc2=size={}x{}:rate=30", mode.w, mode.h)]));
    } else {
        a.extend(s(&["-thread_queue_size", "64", "-f", "v4l2"]));
        if mode.fmt != "test" { a.extend(s(&["-input_format", &mode.fmt])); }
        a.extend(s(&["-video_size", &format!("{}x{}", mode.w, mode.h), "-i", cam]));
    }
    let flip = if mirror { "hflip," } else { "" };
    match rec {
        None => {
            a.extend(s(&["-vf", &format!("{flip}format=yuvj420p"), "-c:v", "mjpeg", "-q:v", "4", "-f", "image2pipe", "pipe:1"]));
        }
        Some(r) => {
            if let Some(m) = &r.mic { a.extend(s(&["-thread_queue_size", "1024", "-f", "pulse", "-i", m])); }
            let rf = if r.filters.is_empty() { "null".to_string() } else { r.filters.clone() };
            a.extend(s(&["-filter_complex", &format!("[0:v]{flip}split=2[p][r];[p]format=yuvj420p[pv];[r]{rf}[rv]"),
                         "-map", "[pv]", "-c:v", "mjpeg", "-q:v", "4", "-f", "image2pipe", "pipe:1", "-map", "[rv]"]));
            if r.mic.is_some() { a.extend(s(&["-map", "1:a", "-c:a", "aac", "-b:a", "128k"])); }
            a.extend(r.enc.clone());
            a.extend(s(&["-movflags", "+faststart"]));
            a.push(r.file.to_string_lossy().to_string());
        }
    }
    a
}

/// A camera at work: ffmpeg's process, the pictures (hub), and the recording when there is one.
pub struct Engine {
    pub hub: Arc<FrameHub>,
    child: Option<Child>,
    pub cam: String,
    pub mode: Option<Mode>,
    pub mirror: bool,
    pub recording: Option<(PathBuf, u64)>,
}

fn log_file() -> PathBuf { dirs::runtime("hearth-camera").join("ffmpeg.log") }

fn friendly(log: &str) -> String {
    let l = log.to_ascii_lowercase();
    if l.contains("busy") { "The camera is being used by another app. Close it and try again.".into() }
    else if l.contains("permission denied") { "Hearth isn't allowed to use the camera (permission denied).".into() }
    else if l.contains("no such file") { "That camera isn't there anymore.".into() }
    else if log.trim().is_empty() { "The camera stopped.".into() }
    else { format!("The camera stopped: {}", log.lines().last().unwrap_or("").chars().take(300).collect::<String>()) }
}

impl Engine {
    pub fn new() -> Engine { Engine { hub: FrameHub::new(), child: None, cam: String::new(), mode: None, mirror: true, recording: None } }

    fn spawn(&mut self, rec: Option<&Rec>) -> Result<(), String> {
        self.halt();
        let mode = self.mode.clone().ok_or("Choose a camera first")?;
        let args = ffmpeg_args(&self.cam, &mode, self.mirror, rec);
        let log = std::fs::File::create(log_file()).map_err(|e| e.to_string())?;
        let mut c = Command::new("ffmpeg").args(&args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::from(log))
            .spawn().map_err(|e| format!("Couldn't start ffmpeg: {e}"))?;
        let out = c.stdout.take().ok_or("no picture pipe")?;
        let hub = self.hub.clone();
        std::thread::spawn(move || jpeg::pump(out, move |f| hub.publish(f)));
        let seq = self.hub.seq();
        // The first picture, or why there isn't one.
        let until = std::time::Instant::now() + Duration::from_secs(8);
        loop {
            if self.hub.seq() > seq { break; }
            if let Ok(Some(_)) = c.try_wait() { return Err(friendly(&tools::tail(&log_file(), 4))); }
            if std::time::Instant::now() > until { let _ = c.kill(); let _ = c.wait(); return Err(format!("The camera didn't send a picture. {}", tools::tail(&log_file(), 2))); }
            std::thread::sleep(Duration::from_millis(30));
        }
        self.child = Some(c);
        Ok(())
    }

    /// Stops ffmpeg (properly: SIGINT finishes a recording's file).
    pub fn halt(&mut self) {
        if let Some(mut c) = self.child.take() { tools::stop_child(&mut c, Duration::from_secs(10)); }
    }

    /// Shows a camera (and stops whatever was shown before).
    pub fn open(&mut self, cams: &[Camera], id: &str) -> Result<(), String> {
        if self.recording.is_some() { return Err("Stop recording first".into()); }
        let c = cams.iter().find(|c| c.id == id).ok_or("That camera isn't there anymore")?;
        self.cam = c.id.clone();
        self.mode = best_mode(&c.modes);
        self.spawn(None)
    }

    pub fn set_mirror(&mut self, on: bool) -> Result<(), String> {
        if self.recording.is_some() { return Err("Stop recording first".into()); }
        self.mirror = on;
        if self.mode.is_some() { self.spawn(None) } else { Ok(()) }
    }

    /// Whether ffmpeg is still going (a camera that was unplugged stops it).
    pub fn alive(&mut self) -> bool { self.child.as_mut().map(|c| matches!(c.try_wait(), Ok(None))).unwrap_or(false) }

    /// Saves the newest picture as a photo.
    pub fn photo(&self) -> Result<PathBuf, String> {
        let (_, f) = self.hub.latest().ok_or("The camera hasn't sent a picture yet")?;
        let p = dirs::unique(&photos_dir(), "Photo", "jpg");
        std::fs::write(&p, &*f).map_err(|e| format!("Couldn't save the photo: {e}"))?;
        Ok(p)
    }

    /// Starts recording a video (with the microphone when `sound` and there is one).
    pub fn record_start(&mut self, sound: bool) -> Result<PathBuf, String> {
        if self.recording.is_some() { return Err("Already recording".into()); }
        let (list, _) = encoders::ffmpeg(false);
        let e = encoders::pick(&list, "auto", &|_| true).ok_or("ffmpeg has no video encoder Camera can use")?;
        let render = encoders::render_node().unwrap_or_default();
        let file = dirs::unique(&videos_dir(), "Video", "mp4");
        let mic = if sound { hearth_media::pulse::mic_source() } else { None };
        let rec = rec_from(file.clone(), encoders::ffmpeg_video_args(&e.id, Quality::Standard, 30, &render, &[]), mic);
        if let Err(err) = self.spawn(Some(&rec)) {
            let _ = self.spawn(None);
            return Err(err);
        }
        self.recording = Some((file.clone(), dirs::now_ms()));
        Ok(file)
    }

    /// Stops recording; returns the video.
    pub fn record_stop(&mut self) -> Result<PathBuf, String> {
        let (file, _) = self.recording.take().ok_or("Not recording")?;
        self.halt();
        let ok = std::fs::metadata(&file).map(|m| m.len() > 0).unwrap_or(false);
        let _ = self.spawn(None);
        if ok { Ok(file) } else { let _ = std::fs::remove_file(&file); Err(format!("The video wasn't saved. {}", tools::tail(&log_file(), 2))) }
    }
}

impl Default for Engine {
    fn default() -> Self { Self::new() }
}

impl Drop for Engine {
    fn drop(&mut self) { self.halt(); }
}

#[cfg(test)]
mod tests {
    use super::*;
    const LIST: &str = "[video4linux2,v4l2 @ 0x55] Compressed:       mjpeg :          Motion-JPEG : 1280x720 640x480 1920x1080 2560x1440\n\
                        [video4linux2,v4l2 @ 0x55] Raw       :     yuyv422 :           YUYV 4:2:2 : 640x480 1280x720 1920x1080\n\
                        [video4linux2,v4l2 @ 0x55] Raw       :     yuyv422 :           YUYV 4:2:2 : {32-4096, 2}x{32-2304, 2}\n\
                        /dev/video0: Immediate exit requested";
    #[test]
    fn formats() {
        let m = parse_formats(LIST);
        assert_eq!(m.len(), 7);
        assert_eq!(m[0], Mode { fmt: "mjpeg".into(), w: 1280, h: 720 });
        assert_eq!(best_mode(&m), Some(Mode { fmt: "mjpeg".into(), w: 1920, h: 1080 }));
        let raw: Vec<Mode> = m.into_iter().filter(|x| x.fmt != "mjpeg").collect();
        assert_eq!(best_mode(&raw), Some(Mode { fmt: "yuyv422".into(), w: 1280, h: 720 }));
        assert_eq!(best_mode(&[]), None);
    }
    #[test]
    fn command_lines() {
        let m = Mode { fmt: "mjpeg".into(), w: 1280, h: 720 };
        assert_eq!(ffmpeg_args("/dev/video0", &m, true, None).join(" "),
            "-hide_banner -loglevel error -nostdin -y -thread_queue_size 64 -f v4l2 -input_format mjpeg -video_size 1280x720 -i /dev/video0 -vf hflip,format=yuvj420p -c:v mjpeg -q:v 4 -f image2pipe pipe:1");
        let rec = rec_from("/v/Video.mp4".into(), encoders::ffmpeg_video_args("h264_vaapi", Quality::Standard, 30, "/dev/dri/renderD128", &[]), Some("mic".into()));
        assert_eq!(rec.filters, "format=nv12,hwupload");
        assert_eq!(ffmpeg_args("/dev/video0", &m, false, Some(&rec)).join(" "),
            "-hide_banner -loglevel error -nostdin -y -vaapi_device /dev/dri/renderD128 -thread_queue_size 64 -f v4l2 -input_format mjpeg -video_size 1280x720 -i /dev/video0 -thread_queue_size 1024 -f pulse -i mic -filter_complex [0:v]split=2[p][r];[p]format=yuvj420p[pv];[r]format=nv12,hwupload[rv] -map [pv] -c:v mjpeg -q:v 4 -f image2pipe pipe:1 -map [rv] -map 1:a -c:a aac -b:a 128k -c:v h264_vaapi -rc_mode CQP -qp 24 -g 60 -movflags +faststart /v/Video.mp4");
        let t = Mode { fmt: "test".into(), w: 640, h: 360 };
        assert!(ffmpeg_args(TEST_ID, &t, true, None).join(" ").contains("-re -f lavfi -i testsrc2=size=640x360:rate=30 -vf hflip,format=yuvj420p"));
    }
}
