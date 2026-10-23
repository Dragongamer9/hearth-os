//! Video encoders: which ones really work on this PC, best first, and the arguments each takes in ffmpeg,
//! wf-recorder and GStreamer.
//! - Nvidia: NVENC (h264_nvenc / nvh264enc).
//! - AMD and Intel: VA-API (h264_vaapi, hevc_vaapi, av1_vaapi / vah264enc, vaapih264enc). Fedora's own Mesa leaves
//!   out H.264 and HEVC encoding, so on Anatase AV1 (newer GPUs) or software may be what's left: each one is tried
//!   with a tiny test encode instead of trusting the list.
//! - Software: libx264, else libopenh264 (what Fedora's ffmpeg-free has).
//! The answer is kept in ~/.cache/hearth-media (tried again when the graphics or ffmpeg change).

use crate::tools;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Encoder {
    /// ffmpeg's (or GStreamer's) name for it: h264_vaapi, libx264, nvh264enc...
    pub id: String,
    /// h264, hevc or av1
    pub codec: String,
    /// On the graphics card
    pub hw: bool,
    /// For people: "AMD graphics (VA-API), H.264"
    pub label: String,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Quality { Standard, High }

impl Quality {
    pub fn from_str(s: &str) -> Quality { if s == "high" { Quality::High } else { Quality::Standard } }
    fn crf(self) -> u32 { if self == Quality::High { 18 } else { 23 } }
    fn qp(self) -> u32 { if self == Quality::High { 19 } else { 24 } }
    /// kbit/s for the encoders that only take a bit rate
    fn kbps(self) -> u32 { if self == Quality::High { 20000 } else { 10000 } }
}

// ------------------------------------------------------------------ the graphics card

/// The first render node (/dev/dri/renderD128...), what VA-API encodes on.
pub fn render_node() -> Option<String> {
    if let Ok(v) = std::env::var("HEARTH_RENDER_NODE") { if !v.is_empty() { return Some(v); } }
    let mut v: Vec<String> = std::fs::read_dir("/dev/dri").ok()?.flatten()
        .map(|e| e.file_name().to_string_lossy().to_string()).filter(|n| n.starts_with("renderD")).collect();
    v.sort();
    // Prefer an AMD or Intel one for VA-API (on a PC with Nvidia too, the Nvidia node has no VA-API).
    let pick = v.iter().find(|n| matches!(vendor_of(n), "amd" | "intel")).or_else(|| v.first())?;
    Some(format!("/dev/dri/{pick}"))
}

fn vendor_of(node_name: &str) -> &'static str {
    let id = std::fs::read_to_string(format!("/sys/class/drm/{node_name}/device/vendor")).unwrap_or_default();
    vendor_name(id.trim())
}

pub fn vendor_name(pci: &str) -> &'static str {
    match pci.to_ascii_lowercase().as_str() { "0x1002" => "amd", "0x8086" => "intel", "0x10de" => "nvidia", _ => "other" }
}

/// Every graphics vendor here (amd, intel, nvidia).
pub fn vendors() -> Vec<&'static str> {
    let mut v: Vec<&'static str> = std::fs::read_dir("/sys/class/drm").into_iter().flatten().flatten()
        .filter_map(|e| { let n = e.file_name().to_string_lossy().to_string(); if n.starts_with("card") && !n.contains('-') { Some(vendor_of(&n)) } else { None } })
        .filter(|v| *v != "other").collect();
    v.sort();
    v.dedup();
    v
}

pub fn has_nvidia() -> bool { std::path::Path::new("/proc/driver/nvidia/version").exists() || vendors().contains(&"nvidia") }

fn vendor_label(v: &str) -> &'static str {
    match v { "amd" => "AMD graphics", "intel" => "Intel graphics", "nvidia" => "Nvidia graphics", _ => "Graphics card" }
}

fn codec_label(c: &str) -> &'static str { match c { "hevc" => "HEVC", "av1" => "AV1", "vp9" => "VP9", _ => "H.264" } }

// ------------------------------------------------------------------ ffmpeg (also what wf-recorder uses)

/// ffmpeg's encoders, best first, that might be here (before trying them).
pub fn ffmpeg_candidates(nvidia: bool, render: bool) -> Vec<(&'static str, &'static str, bool)> {
    let mut v = vec![];
    if nvidia { v.push(("h264_nvenc", "h264", true)); v.push(("hevc_nvenc", "hevc", true)); }
    if render { v.push(("h264_vaapi", "h264", true)); v.push(("hevc_vaapi", "hevc", true)); v.push(("av1_vaapi", "av1", true)); }
    v.push(("libx264", "h264", false));
    v.push(("libopenh264", "h264", false));
    // Last resorts (an ffmpeg without H.264, like Fedora's ffmpeg-free without Cisco's openh264): MP4 takes these too.
    v.push(("libvpx-vp9", "vp9", false));
    v.push(("libsvtav1", "av1", false));
    v
}

/// ffmpeg's arguments for one encoder. `pre` goes before the inputs (VA-API's device), `post` after them.
/// `filters` (cropping, scaling, mirroring) run before the encoder's own conversion.
pub struct FfArgs { pub pre: Vec<String>, pub post: Vec<String> }

pub fn ffmpeg_video_args(enc: &str, q: Quality, fps: u32, render: &str, filters: &[String]) -> FfArgs {
    let gop = (fps.max(1) * 2).to_string();
    let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<String>>();
    let vf = |extra: &str| { let mut f: Vec<String> = filters.to_vec(); if !extra.is_empty() { f.push(extra.to_string()); } f.join(",") };
    let mut pre = vec![];
    let mut post: Vec<String>;
    if enc.ends_with("_vaapi") {
        pre = s(&["-vaapi_device", render]);
        post = s(&["-vf", &vf("format=nv12,hwupload"), "-c:v", enc]);
        if enc == "av1_vaapi" { post.extend(s(&["-b:v", &format!("{}k", q.kbps())])); }
        else { post.extend(s(&["-rc_mode", "CQP", "-qp", &q.qp().to_string()])); }
    } else if enc.ends_with("_nvenc") {
        post = vec![];
        let f = vf("");
        if !f.is_empty() { post.extend(s(&["-vf", &f])); }
        post.extend(s(&["-c:v", enc, "-preset", "p4", "-rc", "vbr", "-cq", &q.qp().to_string(), "-b:v", "0"]));
    } else if enc == "libx264" {
        post = s(&["-vf", &vf("format=yuv420p"), "-c:v", "libx264", "-preset", "veryfast", "-crf", &q.crf().to_string()]);
    } else if enc == "libvpx-vp9" {
        post = s(&["-vf", &vf("format=yuv420p"), "-c:v", enc, "-deadline", "realtime", "-cpu-used", "8", "-row-mt", "1", "-b:v", &format!("{}k", q.kbps())]);
    } else if enc == "libsvtav1" {
        post = s(&["-vf", &vf("format=yuv420p"), "-c:v", enc, "-preset", "10", "-crf", if q == Quality::High { "30" } else { "36" }]);
    } else {
        post = s(&["-vf", &vf("format=yuv420p"), "-c:v", enc, "-b:v", &format!("{}k", q.kbps())]);
    }
    post.extend(s(&["-g", &gop]));
    FfArgs { pre, post }
}

/// wf-recorder's arguments for one encoder (it takes ffmpeg's encoder names; -p sets their options).
pub fn wf_video_args(enc: &str, q: Quality, fps: u32, render: &str) -> Vec<String> {
    let gop = format!("g={}", fps.max(1) * 2);
    let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<String>>();
    let mut a = s(&["-c", enc]);
    if enc.ends_with("_vaapi") {
        a.extend(s(&["-d", render]));
        if enc == "av1_vaapi" { a.extend(s(&["-p", &format!("b={}k", q.kbps())])); }
        else { a.extend(s(&["-p", "rc_mode=CQP", "-p", &format!("qp={}", q.qp())])); }
    } else if enc.ends_with("_nvenc") {
        a.extend(s(&["-p", "preset=p4", "-p", "rc=vbr", "-p", &format!("cq={}", q.qp()), "-p", "b=0"]));
    } else if enc == "libx264" {
        a.extend(s(&["-x", "yuv420p", "-p", "preset=veryfast", "-p", &format!("crf={}", q.crf())]));
    } else if enc == "libvpx-vp9" {
        a.extend(s(&["-x", "yuv420p", "-p", "deadline=realtime", "-p", "cpu-used=8", "-p", "row-mt=1", "-p", &format!("b={}k", q.kbps())]));
    } else if enc == "libsvtav1" {
        a.extend(s(&["-x", "yuv420p", "-p", "preset=10", "-p", if q == Quality::High { "crf=30" } else { "crf=36" }]));
    } else {
        a.extend(s(&["-x", "yuv420p", "-p", &format!("b={}k", q.kbps())]));
    }
    a.extend(s(&["-p", &gop]));
    a
}

/// One test: does this encoder really encode here? (A short black clip into nothing.)
fn ffmpeg_works(enc: &str, render: &str) -> bool {
    let a = ffmpeg_video_args(enc, Quality::Standard, 30, render, &[]);
    let mut args: Vec<String> = vec!["-hide_banner".into(), "-loglevel".into(), "error".into(), "-nostdin".into()];
    args.extend(a.pre);
    args.extend(["-f", "lavfi", "-i", "color=c=black:s=320x240:r=30:d=0.3"].iter().map(|x| x.to_string()));
    args.extend(a.post);
    args.extend(["-f", "null", "-"].iter().map(|x| x.to_string()));
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    tools::run("ffmpeg", &refs, Duration::from_secs(10)).map(|o| o.ok()).unwrap_or(false)
}

fn ffmpeg_listed() -> String {
    tools::run("ffmpeg", &["-hide_banner", "-encoders"], Duration::from_secs(10)).map(|o| o.stdout).unwrap_or_default()
}

/// Whether ffmpeg lists an encoder (audio ones too: libopus, libmp3lame, aac).
pub fn ffmpeg_has_encoder(list: &str, name: &str) -> bool {
    list.lines().any(|l| l.split_whitespace().nth(1) == Some(name))
}

fn cache_key() -> String {
    let ff = tools::which("ffmpeg").and_then(|p| p.metadata().ok()).and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0);
    let nv = std::fs::read_to_string("/proc/driver/nvidia/version").unwrap_or_default();
    let mut dri: Vec<String> = std::fs::read_dir("/dev/dri").into_iter().flatten().flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect();
    dri.sort();
    format!("{ff}|{}|{}|{}", nv.lines().next().unwrap_or(""), dri.join(","), vendors().join(","))
}

#[derive(Serialize, Deserialize)]
struct Cached { key: String, ffmpeg: Vec<Encoder>, audio: Vec<String> }

fn cache_file() -> PathBuf { crate::dirs::cache("hearth-media").join("encoders.json") }

/// ffmpeg's working video encoders, best first, and the audio encoders it lists. Tried once per graphics/ffmpeg
/// change (`fresh` tries again now).
pub fn ffmpeg(fresh: bool) -> (Vec<Encoder>, Vec<String>) {
    if !tools::have("ffmpeg") { return (vec![], vec![]); }
    let key = cache_key();
    if !fresh {
        if let Some(c) = std::fs::read(cache_file()).ok().and_then(|b| serde_json::from_slice::<Cached>(&b).ok()) {
            if c.key == key { return (c.ffmpeg, c.audio); }
        }
    }
    let list = ffmpeg_listed();
    let render = render_node();
    let rvendor = render.as_deref().map(|r| vendor_of(r.trim_start_matches("/dev/dri/"))).unwrap_or("other");
    let mut out = vec![];
    for (id, codec, hw) in ffmpeg_candidates(has_nvidia(), render.is_some()) {
        if !ffmpeg_has_encoder(&list, id) { continue; }
        // Every one is tried: a listed encoder can still fail (no driver, or Fedora's stand-in openh264).
        if !ffmpeg_works(id, render.as_deref().unwrap_or("")) { continue; }
        let who = if id.ends_with("_nvenc") { "Nvidia graphics (NVENC)".to_string() }
            else if hw { format!("{} (VA-API)", vendor_label(rvendor)) }
            else { "Software (the processor)".to_string() };
        out.push(Encoder { id: id.into(), codec: codec.into(), hw, label: format!("{who}, {}", codec_label(codec)) });
    }
    let audio: Vec<String> = ["aac", "libfdk_aac", "libopus", "opus", "libmp3lame", "libvorbis", "flac"].iter()
        .filter(|a| ffmpeg_has_encoder(&list, a)).map(|a| a.to_string()).collect();
    let _ = crate::dirs::write_atomic(&cache_file(), &serde_json::to_vec(&Cached { key, ffmpeg: out.clone(), audio: audio.clone() }).unwrap_or_default());
    (out, audio)
}

/// Picks from the working ones: "auto" the best, "hardware" the best on the graphics card, "software" the
/// processor's. `ok` filters further (e.g. codecs a container takes).
pub fn pick<'a>(list: &'a [Encoder], pref: &str, ok: &dyn Fn(&Encoder) -> bool) -> Option<&'a Encoder> {
    let good: Vec<&Encoder> = list.iter().filter(|e| ok(e)).collect();
    match pref {
        "software" => good.iter().find(|e| !e.hw).or(good.first()).copied(),
        "hardware" => good.iter().find(|e| e.hw).or(good.first()).copied(),
        _ => good.first().copied(),
    }
}

// ------------------------------------------------------------------ GStreamer (Game Mode without gpu-screen-recorder)

pub fn gst_candidates(nvidia: bool) -> Vec<(&'static str, bool)> {
    let mut v = vec![];
    if nvidia { v.push(("nvh264enc", true)); }
    v.push(("vah264enc", true));
    v.push(("vaapih264enc", true));
    v.push(("x264enc", false));
    v.push(("openh264enc", false));
    v
}

pub fn gst_has(element: &str) -> bool {
    tools::have("gst-inspect-1.0") && tools::run("gst-inspect-1.0", &["--exists", element], Duration::from_secs(10)).map(|o| o.ok()).unwrap_or(false)
}

/// The raw format and encoder of a GStreamer H.264 encoder, as gst-launch words.
pub fn gst_video_enc(enc: &str, q: Quality, fps: u32) -> Vec<String> {
    let gop = fps.max(1) * 2;
    let k = q.kbps();
    let fmt = if enc == "openh264enc" { "I420" } else { "NV12" };
    let mut v = vec!["videoconvert".to_string(), "!".into(), format!("video/x-raw,format={fmt}"), "!".into(),
                     "queue".into(), "max-size-buffers=8".into(), "leaky=downstream".into(), "!".into()];
    let e: Vec<String> = match enc {
        "nvh264enc" => vec![enc.into(), format!("bitrate={k}"), format!("gop-size={gop}")],
        "vah264enc" => vec![enc.into(), format!("bitrate={k}"), format!("key-int-max={gop}")],
        "vaapih264enc" => vec![enc.into(), format!("bitrate={k}"), format!("keyframe-period={gop}")],
        "x264enc" => vec![enc.into(), "speed-preset=veryfast".into(), "tune=zerolatency".into(), format!("bitrate={k}"), format!("key-int-max={gop}")],
        _ => vec![enc.into(), format!("bitrate={}", k * 1000), format!("gop-size={gop}")],
    };
    v.extend(e);
    v.extend(["!".to_string(), "h264parse".into()]);
    v
}

fn gst_works(enc: &str) -> bool {
    let mut a: Vec<String> = vec!["-q".into(), "videotestsrc".into(), "num-buffers=15".into(), "!".into(), "video/x-raw,width=320,height=240".into(), "!".into()];
    a.extend(gst_video_enc(enc, Quality::Standard, 30));
    a.extend(["!".to_string(), "fakesink".into()]);
    let refs: Vec<&str> = a.iter().map(String::as_str).collect();
    tools::run("gst-launch-1.0", &refs, Duration::from_secs(12)).map(|o| o.ok()).unwrap_or(false)
}

/// GStreamer's working H.264 encoders, best first.
pub fn gstreamer() -> Vec<Encoder> {
    if !tools::have("gst-launch-1.0") { return vec![]; }
    let mut out = vec![];
    for (id, hw) in gst_candidates(has_nvidia()) {
        if !gst_has(id) { continue; }
        if hw && !gst_works(id) { continue; }
        let who = match id { "nvh264enc" => "Nvidia graphics (NVENC)", "vah264enc" | "vaapih264enc" => "Graphics card (VA-API)", _ => "Software (the processor)" };
        out.push(Encoder { id: id.into(), codec: "h264".into(), hw, label: format!("{who}, H.264") });
    }
    out
}

/// The first GStreamer audio encoder there is, for MP4 and MPEG-TS: AAC, else Opus.
pub fn gst_audio_enc() -> Option<&'static str> {
    ["avenc_aac", "fdkaacenc", "voaacenc", "opusenc"].into_iter().find(|e| gst_has(e))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn j(v: &[String]) -> String { v.join(" ") }
    #[test]
    fn ffmpeg_args() {
        let a = ffmpeg_video_args("h264_vaapi", Quality::Standard, 60, "/dev/dri/renderD128", &["crop=100:100:0:0".into()]);
        assert_eq!(j(&a.pre), "-vaapi_device /dev/dri/renderD128");
        assert_eq!(j(&a.post), "-vf crop=100:100:0:0,format=nv12,hwupload -c:v h264_vaapi -rc_mode CQP -qp 24 -g 120");
        let a = ffmpeg_video_args("av1_vaapi", Quality::High, 30, "/dev/dri/renderD129", &[]);
        assert_eq!(j(&a.post), "-vf format=nv12,hwupload -c:v av1_vaapi -b:v 20000k -g 60");
        let a = ffmpeg_video_args("h264_nvenc", Quality::Standard, 30, "", &[]);
        assert!(a.pre.is_empty());
        assert_eq!(j(&a.post), "-c:v h264_nvenc -preset p4 -rc vbr -cq 24 -b:v 0 -g 60");
        let a = ffmpeg_video_args("libx264", Quality::High, 30, "", &["hflip".into()]);
        assert_eq!(j(&a.post), "-vf hflip,format=yuv420p -c:v libx264 -preset veryfast -crf 18 -g 60");
        let a = ffmpeg_video_args("libopenh264", Quality::Standard, 30, "", &[]);
        assert_eq!(j(&a.post), "-vf format=yuv420p -c:v libopenh264 -b:v 10000k -g 60");
    }
    #[test]
    fn wf_args() {
        assert_eq!(j(&wf_video_args("h264_vaapi", Quality::Standard, 60, "/dev/dri/renderD128")), "-c h264_vaapi -d /dev/dri/renderD128 -p rc_mode=CQP -p qp=24 -p g=120");
        assert_eq!(j(&wf_video_args("libx264", Quality::Standard, 30, "")), "-c libx264 -x yuv420p -p preset=veryfast -p crf=23 -p g=60");
        assert_eq!(j(&wf_video_args("h264_nvenc", Quality::High, 30, "")), "-c h264_nvenc -p preset=p4 -p rc=vbr -p cq=19 -p b=0 -p g=60");
    }
    #[test]
    fn gst_args() {
        assert_eq!(j(&gst_video_enc("vah264enc", Quality::Standard, 60)), "videoconvert ! video/x-raw,format=NV12 ! queue max-size-buffers=8 leaky=downstream ! vah264enc bitrate=10000 key-int-max=120 ! h264parse");
        assert!(j(&gst_video_enc("openh264enc", Quality::Standard, 30)).contains("format=I420 ! queue max-size-buffers=8 leaky=downstream ! openh264enc bitrate=10000000 gop-size=60"));
    }
    #[test]
    fn listing_and_picking() {
        let list = " V....D libx264              libx264 H.264 / AVC\n A....D aac                  AAC (Advanced Audio Coding)\n V....D h264_vaapi           H.264/AVC (VAAPI)\n";
        assert!(ffmpeg_has_encoder(list, "libx264"));
        assert!(ffmpeg_has_encoder(list, "aac"));
        assert!(!ffmpeg_has_encoder(list, "libopus"));
        let e = |id: &str, hw: bool| Encoder { id: id.into(), codec: "h264".into(), hw, label: String::new() };
        let l = vec![e("h264_vaapi", true), e("libx264", false)];
        assert_eq!(pick(&l, "auto", &|_| true).unwrap().id, "h264_vaapi");
        assert_eq!(pick(&l, "software", &|_| true).unwrap().id, "libx264");
        assert_eq!(pick(&l[1..], "hardware", &|_| true).unwrap().id, "libx264");
        assert!(pick(&l, "auto", &|x| x.codec == "av1").is_none());
        assert_eq!(vendor_name("0x10DE"), "nvidia");
        assert_eq!(ffmpeg_candidates(false, false).len(), 4);
        let a = ffmpeg_video_args("libvpx-vp9", Quality::Standard, 30, "", &[]);
        assert_eq!(j(&a.post), "-vf format=yuv420p -c:v libvpx-vp9 -deadline realtime -cpu-used 8 -row-mt 1 -b:v 10000k -g 60");
    }
}
