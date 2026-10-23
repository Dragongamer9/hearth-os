//! How to record, worked out from what's here (probe.rs) and what was asked. Nothing runs here: the result is a
//! list of programs with their arguments, which session.rs starts and stops (and the tests check word for word).
//!
//! The ways, best first:
//!  - gsr: gpu-screen-recorder. Captures on the graphics card (KMS: the whole screen in Game Mode and Desktop
//!    Mode), encodes there (NVENC or VA-API, or the processor), and has its own instant replay (SIGUSR1 saves).
//!  - wf: wf-recorder, Mantel's wlroots screen capture, with ffmpeg's encoders (VA-API, NVENC, libx264...).
//!    Records an area too (slurp chooses it).
//!  - x11: ffmpeg's x11grab (an X11 session, and CI's virtual screen).
//!  - gst: GStreamer reading gamescope's own PipeWire stream (Game Mode without gpu-screen-recorder).
//! Instant replay without gpu-screen-recorder: the capture writes MPEG-TS into a named pipe, and ffmpeg cuts it
//! into 2-second pieces kept in a ring (segment_wrap); saving joins the newest pieces into an MP4.
//! The sound recorder: ffmpeg from PipeWire's PulseAudio side to Opus or MP3 (pw-record to WAV without ffmpeg).

use crate::probe::Env;
use hearth_media::encoders::{self, Encoder, Quality};
use hearth_media::env::Session;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Kind { Record, Replay, Sound }

impl Kind {
    pub fn name(self) -> &'static str { match self { Kind::Record => "record", Kind::Replay => "replay", Kind::Sound => "sound" } }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect { pub x: i32, pub y: i32, pub w: u32, pub h: u32 }

impl Rect {
    /// slurp's "X,Y WxH" (also what --area= takes).
    pub fn parse(s: &str) -> Option<Rect> {
        let (xy, wh) = s.trim().split_once(' ')?;
        let (x, y) = xy.split_once(',')?;
        let (w, h) = wh.trim().split_once('x')?;
        let r = Rect { x: x.trim().parse().ok()?, y: y.trim().parse().ok()?, w: w.trim().parse().ok()?, h: h.trim().parse().ok()? };
        if r.w < 16 || r.h < 16 { return None; }
        Some(r)
    }
    /// Even sizes (what H.264's 4:2:0 needs).
    pub fn even(self) -> Rect { Rect { w: self.w & !1, h: self.h & !1, ..self } }
}

/// What was asked.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Req {
    pub kind: Kind,
    pub area: Option<Rect>,
    pub fps: u32,
    pub quality: String,
    pub system: bool,
    pub mic: bool,
    pub replay_secs: u32,
    pub encoder: String,
    pub sound_format: String,
    /// Started from a shortcut or Quick Access: say so with a notification (the window shows it itself).
    #[serde(default)]
    pub announce: bool,
}

impl Req {
    pub fn from_settings(kind: Kind, s: &crate::settings::Settings) -> Req {
        Req { kind, area: None, fps: s.fps, quality: s.quality.clone(), system: s.system, mic: s.mic, replay_secs: s.replay_secs,
              encoder: s.encoder.clone(), sound_format: s.sound_format.clone(), announce: false }
    }
}

/// One program to run.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Proc { pub prog: String, pub args: Vec<String> }

impl Proc {
    fn new(prog: &str, args: Vec<String>) -> Proc { Proc { prog: prog.into(), args } }
    pub fn line(&self) -> String { format!("{} {}", self.prog, self.args.join(" ")) }
}

#[derive(Serialize, Clone, Debug)]
pub struct Plan {
    pub backend: String,
    /// For people: which encoder
    pub encoder: String,
    /// procs[0] writes (the capture); any after it read from the pipe
    pub procs: Vec<Proc>,
    pub fifo: Option<PathBuf>,
    /// The finished file (recording, sound)
    pub file: Option<PathBuf>,
    /// Instant replay's ring of pieces
    pub segments: Option<PathBuf>,
    /// gpu-screen-recorder's instant replay saves into this folder
    pub gsr_dir: Option<PathBuf>,
    /// The sound recorder also sends 8 kHz mono samples to its standard output (the level meter)
    pub level: bool,
}

/// Where things go.
pub struct Paths { pub file: PathBuf, pub work: PathBuf, pub replay_dir: PathBuf }

pub const SEGMENT_SECS: u32 = 2;

fn s(v: &[&str]) -> Vec<String> { v.iter().map(|x| x.to_string()).collect() }

/// Which way, before anything else (the sound source depends on it: gpu-screen-recorder mixes its own).
pub fn backend(env: &Env, req: &Req) -> Result<&'static str, String> {
    if req.kind == Kind::Sound {
        if env.ffmpeg { return Ok("ffmpeg"); }
        if env.pw_record { return Ok("pw-record"); }
        return Err("The sound recorder needs ffmpeg (or PipeWire's pw-record), and neither is installed".into());
    }
    let replay = req.kind == Kind::Replay;
    let enc_ok = !env.encoders.is_empty();
    let wf_ok = env.wf && enc_ok && (!replay || env.ffmpeg);
    let x11_ok = env.ffmpeg && enc_ok;
    let gst_ok = env.gst_pipewire && env.gamescope_node.is_some() && !env.gst_encoders.is_empty() && (!replay || env.ffmpeg);
    let gsr_ok = env.gsr && (req.area.is_none() || env.gsr_region);
    if let Some(f) = env.force.as_deref() {
        let ok = match f { "gsr" => gsr_ok, "wf" => wf_ok, "x11" => x11_ok, "gst" => gst_ok, _ => false };
        if ok { return Ok(match f { "gsr" => "gsr", "wf" => "wf", "x11" => "x11", _ => "gst" }); }
        return Err(format!("HEARTH_RECORDER_BACKEND={f} can't record here"));
    }
    match env.session {
        Session::Gamescope => {
            if req.area.is_some() && !gsr_ok { return Err("In Game Mode Recorder records the whole screen".into()); }
            if gsr_ok { Ok("gsr") }
            else if gst_ok { Ok("gst") }
            else if env.gst_pipewire && env.gamescope_node.is_none() { Err("gamescope isn't sharing its picture (no PipeWire stream called gamescope)".into()) }
            else { Err("Recording in Game Mode needs gpu-screen-recorder, or GStreamer with its PipeWire plugin".into()) }
        }
        Session::Wayland => {
            if req.area.is_some() {
                if wf_ok { Ok("wf") } else if gsr_ok { Ok("gsr") } else { Err("Recording an area needs wf-recorder".into()) }
            } else if gsr_ok { Ok("gsr") }
            else if wf_ok { Ok("wf") }
            else if env.wf && !enc_ok { Err("ffmpeg has no video encoder Recorder can use".into()) }
            else { Err("Recording the desktop needs gpu-screen-recorder or wf-recorder".into()) }
        }
        Session::X11 => {
            if gsr_ok { Ok("gsr") } else if x11_ok { Ok("x11") } else { Err("Recording needs ffmpeg (with a video encoder) or gpu-screen-recorder".into()) }
        }
        Session::None => Err("There's no screen to record here".into()),
    }
}

/// The encoder to use, from what works and the setting; instant replay's MPEG-TS pieces take H.264 and HEVC only.
pub fn encoder<'a>(list: &'a [Encoder], req: &Req) -> Option<&'a Encoder> {
    let replay = req.kind == Kind::Replay;
    encoders::pick(list, &req.encoder, &|e| !replay || e.codec == "h264" || e.codec == "hevc")
}

/// The whole plan. `audio` is the PulseAudio source to record from (None: silent); gpu-screen-recorder uses
/// req.system / req.mic itself.
pub fn plan(env: &Env, req: &Req, audio: Option<&str>, p: &Paths) -> Result<Plan, String> {
    let b = backend(env, req)?;
    let q = Quality::from_str(&req.quality);
    let fps = req.fps.max(1);
    let replay = req.kind == Kind::Replay;
    let wrap = (req.replay_secs / SEGMENT_SECS + 3).to_string();
    let seg_args = |dir: &Path| -> Vec<String> {
        let mut a = s(&["-f", "segment", "-segment_time", &SEGMENT_SECS.to_string(), "-segment_wrap", &wrap, "-segment_format", "mpegts", "-reset_timestamps", "0"]);
        a.push(dir.join("seg%03d.ts").to_string_lossy().to_string());
        a
    };
    let segmenter = |fifo: &Path, dir: &Path| -> Proc {
        let mut a = s(&["-hide_banner", "-loglevel", "error", "-nostdin", "-y", "-f", "mpegts", "-i"]);
        a.push(fifo.to_string_lossy().to_string());
        a.extend(s(&["-map", "0", "-c", "copy"]));
        a.extend(seg_args(dir));
        Proc::new("ffmpeg", a)
    };
    let file = p.file.to_string_lossy().to_string();
    let fifo = p.work.join("capture.ts");
    let segdir = p.work.join("pieces");
    let mut plan = Plan { backend: b.into(), encoder: String::new(), procs: vec![], fifo: None, file: None, segments: None, gsr_dir: None, level: false };

    match b {
        "ffmpeg" | "pw-record" => {
            let src = audio.ok_or("Choose what to record: the microphone or what's playing")?.to_string();
            if b == "pw-record" {
                plan.encoder = "WAV".into();
                plan.procs.push(Proc::new("pw-record", vec!["--target".into(), src, file.clone()]));
            } else {
                let (codec, _, extra) = sound_codec(&env.audio_encoders, &req.sound_format);
                plan.encoder = match codec { "libmp3lame" => "MP3", "libvorbis" => "Ogg Vorbis", "flac" => "FLAC", _ => "Opus" }.into();
                let mut v = s(&["-hide_banner", "-loglevel", "error", "-nostdin", "-y", "-thread_queue_size", "1024", "-f", "pulse", "-i", &src,
                                "-map", "0:a", "-c:a", codec]);
                v.extend(extra);
                v.push(file.clone());
                // A second output for the level meter: 8 kHz mono samples on standard output.
                v.extend(s(&["-map", "0:a", "-ac", "1", "-ar", "8000", "-f", "s16le", "pipe:1"]));
                plan.level = true;
                plan.procs.push(Proc::new("ffmpeg", v));
            }
            plan.file = Some(p.file.clone());
        }
        "gsr" => {
            let mut a: Vec<String> = vec![];
            match req.area { Some(r) => { let r = r.even(); a.extend(s(&["-w", "region", "-region", &format!("{}x{}+{}+{}", r.w, r.h, r.x, r.y)])); } None => a.extend(s(&["-w", "screen"])) }
            a.extend(s(&["-f", &fps.to_string(), "-q", if q == Quality::High { "very_high" } else { "high" }, "-c", "mp4"]));
            if req.encoder == "software" { a.extend(s(&["-encoder", "cpu"])); }
            match (req.system, req.mic) {
                (true, true) => a.extend(s(&["-a", "default_output|default_input"])),
                (true, false) => a.extend(s(&["-a", "default_output"])),
                (false, true) => a.extend(s(&["-a", "default_input"])),
                _ => {}
            }
            if replay {
                a.extend(s(&["-r", &req.replay_secs.to_string(), "-o", &p.replay_dir.to_string_lossy()]));
                plan.gsr_dir = Some(p.replay_dir.clone());
            } else {
                a.extend(s(&["-o", &file]));
                plan.file = Some(p.file.clone());
            }
            plan.encoder = if req.encoder == "software" { "gpu-screen-recorder, software".into() } else { "gpu-screen-recorder, graphics card".into() };
            plan.procs.push(Proc::new("gpu-screen-recorder", a));
        }
        "wf" => {
            let e = encoder(&env.encoders, req).ok_or("ffmpeg has no video encoder Recorder can use")?;
            plan.encoder = e.label.clone();
            let mut a = s(&["-y"]);
            if let Some(r) = req.area { let r = r.even(); a.extend(s(&["-g", &format!("{},{} {}x{}", r.x, r.y, r.w, r.h)])); }
            a.extend(s(&["-r", &fps.to_string()]));
            a.extend(encoders::wf_video_args(&e.id, q, fps, &env.render));
            if let Some(src) = audio { a.push(format!("--audio={src}")); }
            if replay {
                a.extend(s(&["-m", "mpegts", "-f", &fifo.to_string_lossy()]));
                plan.procs.push(Proc::new("wf-recorder", a));
                plan.procs.push(segmenter(&fifo, &segdir));
                plan.fifo = Some(fifo);
                plan.segments = Some(segdir);
            } else {
                a.extend(s(&["-f", &file]));
                plan.procs.push(Proc::new("wf-recorder", a));
                plan.file = Some(p.file.clone());
            }
        }
        "x11" => {
            let e = encoder(&env.encoders, req).ok_or("ffmpeg has no video encoder Recorder can use")?;
            plan.encoder = e.label.clone();
            let v = encoders::ffmpeg_video_args(&e.id, q, fps, &env.render, &[]);
            let mut a = s(&["-hide_banner", "-loglevel", "error", "-nostdin", "-y"]);
            a.extend(v.pre);
            a.extend(s(&["-thread_queue_size", "512", "-f", "x11grab", "-framerate", &fps.to_string(), "-draw_mouse", "1"]));
            let disp = if env.display.is_empty() { ":0".to_string() } else { env.display.clone() };
            match req.area {
                Some(r) => { let r = r.even(); a.extend(s(&["-video_size", &format!("{}x{}", r.w, r.h), "-i", &format!("{disp}+{},{}", r.x, r.y)])); }
                None => a.extend(s(&["-i", &disp])),
            }
            if let Some(src) = audio { a.extend(s(&["-thread_queue_size", "1024", "-f", "pulse", "-i", src])); }
            a.extend(s(&["-map", "0:v"]));
            if audio.is_some() { a.extend(s(&["-map", "1:a"])); }
            a.extend(v.post);
            if audio.is_some() { a.extend(s(&["-c:a", "aac", "-b:a", "160k"])); }
            if replay {
                a.extend(seg_args(&segdir));
                plan.segments = Some(segdir);
            } else {
                a.extend(s(&["-movflags", "+faststart", &file]));
                plan.file = Some(p.file.clone());
            }
            plan.procs.push(Proc::new("ffmpeg", a));
        }
        _ => {
            // gst: gamescope's PipeWire stream.
            let e = encoders::pick(&env.gst_encoders, &req.encoder, &|_| true).ok_or("GStreamer has no H.264 encoder")?;
            plan.encoder = e.label.clone();
            let node = env.gamescope_node.ok_or("gamescope isn't sharing its picture")?;
            let mut a = s(&["-e", "-q", "pipewiresrc", &format!("path={node}"), "do-timestamp=true", "!"]);
            a.extend(encoders::gst_video_enc(&e.id, q, fps));
            let mux = if replay { "mpegtsmux" } else { "mp4mux" };
            let out = if replay { fifo.to_string_lossy().to_string() } else { file.clone() };
            a.extend(s(&["!", "queue", "!", mux, "name=mux", "!", "filesink", &format!("location={out}")]));
            if let (Some(src), Some(aenc)) = (audio, env.gst_audio.as_deref()) {
                a.extend(s(&["pulsesrc", &format!("device={src}"), "do-timestamp=true", "!", "audioconvert", "!", "audioresample", "!", "queue", "!", aenc, "!", "queue", "!", "mux."]));
            }
            plan.procs.push(Proc::new("gst-launch-1.0", a));
            if replay {
                plan.procs.push(segmenter(&fifo, &segdir));
                plan.fifo = Some(fifo);
                plan.segments = Some(segdir);
            } else {
                plan.file = Some(p.file.clone());
            }
        }
    }
    Ok(plan)
}

/// The sound recorder's encoder (from those ffmpeg has), its file extension and its options.
pub fn sound_codec(have: &[String], format: &str) -> (&'static str, &'static str, Vec<String>) {
    let has = |n: &str| have.iter().any(|x| x == n);
    if format == "mp3" && has("libmp3lame") { ("libmp3lame", "mp3", s(&["-b:a", "192k"])) }
    else if has("libopus") { ("libopus", "opus", s(&["-b:a", "96k"])) }
    else if has("opus") { ("opus", "opus", s(&["-strict", "-2", "-b:a", "96k"])) }
    else if has("libvorbis") { ("libvorbis", "ogg", s(&["-q:a", "5"])) }
    else { ("flac", "flac", vec![]) }
}

/// The sound recording's file extension for the way it'll be recorded.
pub fn sound_ext(env: &Env, req: &Req, backend: &str) -> &'static str {
    if backend == "pw-record" { "wav" } else { sound_codec(&env.audio_encoders, &req.sound_format).1 }
}

/// Instant replay's pieces to join for the last `secs` seconds: newest last, leaving out the one being written.
pub fn pieces_for(mut files: Vec<(u64, PathBuf)>, secs: u32) -> Vec<PathBuf> {
    files.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    files.pop();
    let n = (secs.div_ceil(SEGMENT_SECS)) as usize;
    let start = files.len().saturating_sub(n);
    files.into_iter().skip(start).map(|(_, p)| p).collect()
}

/// ffmpeg's concat list for those pieces.
pub fn concat_list(files: &[PathBuf]) -> String {
    files.iter().map(|f| format!("file '{}'\n", f.to_string_lossy().replace('\'', "'\\''"))).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;

    fn enc(id: &str, codec: &str, hw: bool) -> Encoder { Encoder { id: id.into(), codec: codec.into(), hw, label: id.into() } }
    fn paths() -> Paths { Paths { file: "/v/Recording.mp4".into(), work: "/run/r/work".into(), replay_dir: "/v".into() } }
    fn req(kind: Kind) -> Req { Req::from_settings(kind, &Settings::default()) }

    #[test]
    fn areas() {
        assert_eq!(Rect::parse("10,20 301x201"), Some(Rect { x: 10, y: 20, w: 301, h: 201 }));
        assert_eq!(Rect::parse("10,20 301x201").unwrap().even(), Rect { x: 10, y: 20, w: 300, h: 200 });
        assert_eq!(Rect::parse("-5,0 1920x1080").unwrap().x, -5);
        assert_eq!(Rect::parse("nonsense"), None);
        assert_eq!(Rect::parse("0,0 4x4"), None);
    }

    #[test]
    fn picks_the_way() {
        let mut e = Env::empty(Session::Wayland);
        assert!(backend(&e, &req(Kind::Record)).is_err());
        e.wf = true; e.ffmpeg = true; e.encoders = vec![enc("libx264", "h264", false)];
        assert_eq!(backend(&e, &req(Kind::Record)).unwrap(), "wf");
        e.gsr = true;
        assert_eq!(backend(&e, &req(Kind::Record)).unwrap(), "gsr");
        let mut area = req(Kind::Record);
        area.area = Rect::parse("0,0 640x480");
        assert_eq!(backend(&e, &area).unwrap(), "wf");
        e.force = Some("wf".into());
        assert_eq!(backend(&e, &req(Kind::Replay)).unwrap(), "wf");
        e.force = Some("gst".into());
        assert!(backend(&e, &req(Kind::Record)).is_err());

        let mut g = Env::empty(Session::Gamescope);
        assert!(backend(&g, &req(Kind::Record)).unwrap_err().contains("Game Mode"));
        g.gst_pipewire = true; g.ffmpeg = true; g.gst_encoders = vec![enc("vah264enc", "h264", true)];
        assert!(backend(&g, &req(Kind::Record)).unwrap_err().contains("PipeWire"));
        g.gamescope_node = Some(42);
        assert_eq!(backend(&g, &req(Kind::Replay)).unwrap(), "gst");
        assert!(backend(&g, &area).is_err());

        let mut x = Env::empty(Session::X11);
        x.ffmpeg = true; x.encoders = vec![enc("libopenh264", "h264", false)];
        assert_eq!(backend(&x, &req(Kind::Record)).unwrap(), "x11");
        assert_eq!(backend(&x, &req(Kind::Sound)).unwrap(), "ffmpeg");
        assert!(backend(&Env::empty(Session::None), &req(Kind::Record)).is_err());
    }

    #[test]
    fn gsr_lines() {
        let mut e = Env::empty(Session::Gamescope);
        e.gsr = true;
        let p = plan(&e, &req(Kind::Record), None, &paths()).unwrap();
        assert_eq!(p.procs[0].line(), "gpu-screen-recorder -w screen -f 60 -q high -c mp4 -a default_output -o /v/Recording.mp4");
        let mut r = req(Kind::Replay);
        r.mic = true; r.replay_secs = 60; r.quality = "high".into(); r.encoder = "software".into();
        let p = plan(&e, &r, None, &paths()).unwrap();
        assert_eq!(p.procs[0].line(), "gpu-screen-recorder -w screen -f 60 -q very_high -c mp4 -encoder cpu -a default_output|default_input -r 60 -o /v");
        assert_eq!(p.gsr_dir, Some(PathBuf::from("/v")));
        e.session = Session::Wayland; e.gsr_region = true;
        let mut a = req(Kind::Record);
        a.area = Rect::parse("5,6 101x99"); a.system = false;
        assert_eq!(plan(&e, &a, None, &paths()).unwrap().procs[0].line(), "gpu-screen-recorder -w region -region 100x98+5+6 -f 60 -q high -c mp4 -o /v/Recording.mp4");
    }

    #[test]
    fn wf_lines() {
        let mut e = Env::empty(Session::Wayland);
        e.wf = true; e.ffmpeg = true; e.render = "/dev/dri/renderD128".into();
        e.encoders = vec![enc("h264_vaapi", "h264", true), enc("libx264", "h264", false)];
        let mut r = req(Kind::Record);
        r.area = Rect::parse("0,0 800x600");
        let p = plan(&e, &r, Some("alsa_output.monitor"), &paths()).unwrap();
        assert_eq!(p.procs[0].line(), "wf-recorder -y -g 0,0 800x600 -r 60 -c h264_vaapi -d /dev/dri/renderD128 -p rc_mode=CQP -p qp=24 -p g=120 --audio=alsa_output.monitor -f /v/Recording.mp4");
        let mut r = req(Kind::Replay);
        r.encoder = "software".into(); r.fps = 30;
        let p = plan(&e, &r, None, &paths()).unwrap();
        assert_eq!(p.procs[0].line(), "wf-recorder -y -r 30 -c libx264 -x yuv420p -p preset=veryfast -p crf=23 -p g=60 -m mpegts -f /run/r/work/capture.ts");
        assert_eq!(p.procs[1].line(), "ffmpeg -hide_banner -loglevel error -nostdin -y -f mpegts -i /run/r/work/capture.ts -map 0 -c copy -f segment -segment_time 2 -segment_wrap 18 -segment_format mpegts -reset_timestamps 0 /run/r/work/pieces/seg%03d.ts");
        assert_eq!(p.fifo, Some(PathBuf::from("/run/r/work/capture.ts")));
    }

    #[test]
    fn x11_lines() {
        let mut e = Env::empty(Session::X11);
        e.ffmpeg = true; e.display = ":99".into();
        e.encoders = vec![enc("libopenh264", "h264", false)];
        let mut r = req(Kind::Record);
        r.fps = 30;
        let p = plan(&e, &r, Some("mix.monitor"), &paths()).unwrap();
        assert_eq!(p.procs[0].line(), "ffmpeg -hide_banner -loglevel error -nostdin -y -thread_queue_size 512 -f x11grab -framerate 30 -draw_mouse 1 -i :99 -thread_queue_size 1024 -f pulse -i mix.monitor -map 0:v -map 1:a -vf format=yuv420p -c:v libopenh264 -b:v 10000k -g 60 -c:a aac -b:a 160k -movflags +faststart /v/Recording.mp4");
        let mut r = req(Kind::Replay);
        r.area = Rect::parse("10,10 640x480"); r.fps = 30; r.replay_secs = 30;
        let p = plan(&e, &r, None, &paths()).unwrap();
        assert!(p.procs[0].line().contains("-video_size 640x480 -i :99+10,10 -map 0:v -vf format=yuv420p"), "{}", p.procs[0].line());
        assert!(p.procs[0].line().ends_with("-f segment -segment_time 2 -segment_wrap 18 -segment_format mpegts -reset_timestamps 0 /run/r/work/pieces/seg%03d.ts"));
        // Instant replay never picks AV1 (MPEG-TS pieces).
        e.encoders = vec![enc("av1_vaapi", "av1", true)];
        assert!(plan(&e, &r, None, &paths()).is_err());
    }

    #[test]
    fn gst_lines() {
        let mut e = Env::empty(Session::Gamescope);
        e.gst_pipewire = true; e.ffmpeg = true; e.gamescope_node = Some(77); e.gst_audio = Some("avenc_aac".into());
        e.gst_encoders = vec![enc("x264enc", "h264", false)];
        let p = plan(&e, &req(Kind::Record), Some("sink.monitor"), &paths()).unwrap();
        assert_eq!(p.procs[0].line(), "gst-launch-1.0 -e -q pipewiresrc path=77 do-timestamp=true ! videoconvert ! video/x-raw,format=NV12 ! queue max-size-buffers=8 leaky=downstream ! x264enc speed-preset=veryfast tune=zerolatency bitrate=10000 key-int-max=120 ! h264parse ! queue ! mp4mux name=mux ! filesink location=/v/Recording.mp4 pulsesrc device=sink.monitor do-timestamp=true ! audioconvert ! audioresample ! queue ! avenc_aac ! queue ! mux.");
        let p = plan(&e, &req(Kind::Replay), None, &paths()).unwrap();
        assert!(p.procs[0].line().ends_with("mpegtsmux name=mux ! filesink location=/run/r/work/capture.ts"));
        assert_eq!(p.procs.len(), 2);
    }

    #[test]
    fn sound_lines() {
        let mut e = Env::empty(Session::None);
        e.ffmpeg = true; e.audio_encoders = vec!["aac".into(), "libopus".into(), "libmp3lame".into()];
        let mut p = Paths { file: "/m/Sound.opus".into(), work: "/w".into(), replay_dir: "/v".into() };
        let pl = plan(&e, &req(Kind::Sound), Some("mic"), &p).unwrap();
        assert_eq!(pl.procs[0].line(), "ffmpeg -hide_banner -loglevel error -nostdin -y -thread_queue_size 1024 -f pulse -i mic -map 0:a -c:a libopus -b:a 96k /m/Sound.opus -map 0:a -ac 1 -ar 8000 -f s16le pipe:1");
        assert!(pl.level);
        let mut r = req(Kind::Sound);
        r.sound_format = "mp3".into();
        p.file = "/m/Sound.mp3".into();
        assert!(plan(&e, &r, Some("mic"), &p).unwrap().procs[0].line().contains("-c:a libmp3lame -b:a 192k /m/Sound.mp3"));
        assert!(plan(&e, &r, None, &p).is_err());
        assert_eq!(sound_ext(&e, &r, "ffmpeg"), "mp3");
        assert_eq!(sound_codec(&["flac".to_string()], "opus").1, "flac");
        let mut w = Env::empty(Session::None);
        w.pw_record = true;
        assert_eq!(plan(&w, &r, Some("mic"), &p).unwrap().procs[0].line(), "pw-record --target mic /m/Sound.mp3");
    }

    #[test]
    fn replay_pieces() {
        let f: Vec<(u64, PathBuf)> = (0..10u64).map(|i| (100 + i, PathBuf::from(format!("seg{:03}.ts", (i + 5) % 10)))).collect();
        let got = pieces_for(f, 6);
        assert_eq!(got, vec![PathBuf::from("seg001.ts"), PathBuf::from("seg002.ts"), PathBuf::from("seg003.ts")]);
        assert_eq!(pieces_for(vec![(1, "a".into())], 30), Vec::<PathBuf>::new());
        assert_eq!(concat_list(&[PathBuf::from("/x/it's.ts")]), "file '/x/it'\\''s.ts'\n");
    }
}
