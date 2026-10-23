//! FFmpeg, for what the web view can't do by itself: reading a file's tracks and chapters (ffprobe), showing
//! subtitles stored inside a video, getting a file the web view can't play ready to play, and every edit (trim, cut
//! out a part, rotate/flip, remove the sound, fades, loudness). The edits are planned here as plain ffmpeg command
//! lines (tested without running anything), then run with progress and a Stop button.
//! Edits never write over the file they start from: they always make a new file.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Serialize, Clone, Debug, Default)]
pub struct Tools {
    pub ffmpeg: Option<String>,
    pub ffprobe: Option<String>,
    pub version: String,
    pub encoders: Vec<String>,
}

fn exe(name: &str) -> String { if cfg!(windows) { format!("{name}.exe") } else { name.to_string() } }

/// Where a program is: next to Hearth Player, then on the PATH.
pub fn find(name: &str) -> Option<PathBuf> {
    let file = exe(name);
    if let Some(d) = std::env::var_os("HEARTH_FFMPEG_DIR") { let p = PathBuf::from(d).join(&file); if p.is_file() { return Some(p); } }
    if let Some(d) = std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf)) {
        let p = d.join(&file);
        if p.is_file() { return Some(p); }
    }
    std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()).map(|d| d.join(&file)).find(|p| p.is_file())
}

fn quiet(cmd: &mut Command) -> &mut Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    cmd
}

/// The encoder names in `ffmpeg -encoders` output.
pub fn parse_encoders(text: &str) -> Vec<String> {
    let mut on = false;
    let mut v = Vec::new();
    for l in text.lines() {
        if l.trim_start().starts_with("------") { on = true; continue; }
        if !on { continue; }
        let mut it = l.split_whitespace();
        let (Some(flags), Some(name)) = (it.next(), it.next()) else { continue };
        if flags.len() == 6 { v.push(name.to_string()); }
    }
    v
}

pub fn detect() -> Tools {
    let ffmpeg = find("ffmpeg");
    let ffprobe = find("ffprobe");
    let mut t = Tools { ffmpeg: ffmpeg.as_ref().map(|p| p.to_string_lossy().into_owned()), ffprobe: ffprobe.as_ref().map(|p| p.to_string_lossy().into_owned()), ..Default::default() };
    if let Some(f) = &ffmpeg {
        if let Ok(o) = quiet(Command::new(f).args(["-hide_banner", "-version"])).output() {
            t.version = String::from_utf8_lossy(&o.stdout).lines().next().unwrap_or("").to_string();
        }
        if let Ok(o) = quiet(Command::new(f).args(["-hide_banner", "-encoders"])).output() {
            t.encoders = parse_encoders(&String::from_utf8_lossy(&o.stdout));
        }
    }
    t
}

// ------------------------------------------------------------------ what's in a file

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
pub struct Stream {
    /// The stream's number in the file (for -map 0:N).
    pub index: u32,
    /// "video", "audio", "subtitle", "cover" (a song's picture), "attachment", "data".
    pub kind: String,
    /// Its number among streams of its kind (0, 1, ...).
    pub nth: u32,
    pub codec: String,
    pub lang: String,
    pub title: String,
    pub default: bool,
    pub forced: bool,
    pub label: String,
    pub width: u32,
    pub height: u32,
    pub channels: u32,
    pub sample_rate: u32,
    /// Counter-clockwise degrees from the display matrix (phones' upright videos).
    pub rotation: i32,
    /// A subtitle stream made of text (can be shown); picture subtitles (Blu-ray, DVD) can't.
    pub text: bool,
}

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
pub struct Chapter { pub start: f64, pub end: f64, pub title: String }

#[derive(Serialize, Clone, Debug, Default)]
pub struct Probe {
    pub duration: f64,
    pub format: String,
    pub bit_rate: u64,
    pub streams: Vec<Stream>,
    pub chapters: Vec<Chapter>,
    pub tags: BTreeMap<String, String>,
}

impl Probe {
    pub fn video(&self) -> Option<&Stream> { self.streams.iter().find(|s| s.kind == "video") }
    pub fn audio(&self) -> Option<&Stream> { self.streams.iter().find(|s| s.kind == "audio") }
}

const TEXT_SUBS: &[&str] = &["subrip", "srt", "ass", "ssa", "webvtt", "mov_text", "text", "microdvd", "subviewer", "jacosub", "sami", "realtext"];

fn num(v: &Value) -> f64 {
    v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse().ok())).unwrap_or(0.0)
}

fn tag(v: &Value, k: &str) -> String {
    v.get("tags").and_then(|t| t.as_object()).and_then(|t| t.iter().find(|(kk, _)| kk.eq_ignore_ascii_case(k)).map(|(_, v)| v.as_str().unwrap_or("").to_string())).unwrap_or_default()
}

/// ffprobe's JSON (-show_format -show_streams -show_chapters) as a Probe.
pub fn parse_probe(json: &str) -> Result<Probe, String> {
    let v: Value = serde_json::from_str(json).map_err(|e| format!("ffprobe said something unexpected: {e}"))?;
    let mut p = Probe::default();
    if let Some(f) = v.get("format") {
        p.duration = num(&f["duration"]);
        p.format = f["format_name"].as_str().unwrap_or("").to_string();
        p.bit_rate = num(&f["bit_rate"]) as u64;
        if let Some(t) = f.get("tags").and_then(Value::as_object) {
            for (k, val) in t { p.tags.insert(k.to_ascii_lowercase(), val.as_str().unwrap_or("").to_string()); }
        }
    }
    let mut counts: BTreeMap<String, u32> = BTreeMap::new();
    for s in v["streams"].as_array().cloned().unwrap_or_default() {
        let mut kind = s["codec_type"].as_str().unwrap_or("data").to_string();
        let disp = |k: &str| s.get("disposition").and_then(|d| d.get(k)).and_then(Value::as_i64).unwrap_or(0) == 1;
        if kind == "video" && disp("attached_pic") { kind = "cover".into(); }
        let nth = { let c = counts.entry(kind.clone()).or_insert(0); let n = *c; *c += 1; n };
        let codec = s["codec_name"].as_str().unwrap_or("").to_string();
        let lang = tag(&s, "language");
        let title = tag(&s, "title");
        let rotation = s["side_data_list"].as_array().and_then(|l| l.iter().find_map(|d| d.get("rotation").map(num))).unwrap_or(0.0) as i32;
        let mut st = Stream {
            index: s["index"].as_u64().unwrap_or(0) as u32, kind: kind.clone(), nth, codec: codec.clone(), lang: lang.clone(), title: title.clone(),
            default: disp("default"), forced: disp("forced"), label: String::new(),
            width: s["width"].as_u64().unwrap_or(0) as u32, height: s["height"].as_u64().unwrap_or(0) as u32,
            channels: s["channels"].as_u64().unwrap_or(0) as u32, sample_rate: num(&s["sample_rate"]) as u32,
            rotation, text: kind == "subtitle" && TEXT_SUBS.contains(&codec.as_str()),
        };
        st.label = stream_label(&st);
        p.streams.push(st);
    }
    for c in v["chapters"].as_array().cloned().unwrap_or_default() {
        let start = num(&c["start_time"]);
        let end = num(&c["end_time"]);
        let mut title = tag(&c, "title");
        if title.is_empty() { title = format!("Chapter {}", p.chapters.len() + 1); }
        p.chapters.push(Chapter { start, end, title });
    }
    if p.duration <= 0.0 {
        p.duration = v["streams"].as_array().and_then(|a| a.iter().map(|s| num(&s["duration"])).reduce(f64::max)).unwrap_or(0.0);
    }
    Ok(p)
}

fn channels_name(n: u32) -> String {
    match n { 0 => String::new(), 1 => "mono".into(), 2 => "stereo".into(), 6 => "5.1".into(), 8 => "7.1".into(), n => format!("{n} channels") }
}

fn stream_label(s: &Stream) -> String {
    let lang = if s.lang.is_empty() || s.lang == "und" { String::new() } else { crate::media::lang_name(&s.lang) };
    let mut bits: Vec<String> = Vec::new();
    if !s.title.is_empty() { bits.push(s.title.clone()); }
    if !lang.is_empty() && !s.title.to_lowercase().contains(&lang.to_lowercase()) { bits.push(lang); }
    let codec = s.codec.to_uppercase().replace("SUBRIP", "SRT").replace("HDMV_PGS_SUBTITLE", "PGS").replace("DVD_SUBTITLE", "DVD").replace("MOV_TEXT", "Text");
    match s.kind.as_str() {
        "audio" => { let c = channels_name(s.channels); bits.push(if c.is_empty() { codec } else { format!("{codec} {c}") }); }
        "subtitle" => { bits.push(codec); if s.forced { bits.push("forced".into()); } }
        _ => bits.push(codec),
    }
    let head = if bits.len() > 1 { format!("{} ({})", bits[0], bits[1..].join(", ")) } else { bits.join("") };
    if head.is_empty() { format!("Track {}", s.nth + 1) } else { head }
}

pub fn probe(tools: &Tools, path: &Path) -> Result<Probe, String> {
    let fp = tools.ffprobe.as_ref().ok_or("ffprobe isn't installed")?;
    let o = quiet(Command::new(fp).args(["-v", "error", "-print_format", "json", "-show_format", "-show_streams", "-show_chapters"]).arg(path))
        .output().map_err(|e| format!("Couldn't run ffprobe: {e}"))?;
    if !o.status.success() { return Err(format!("ffprobe couldn't read it: {}", String::from_utf8_lossy(&o.stderr).trim())); }
    parse_probe(&String::from_utf8_lossy(&o.stdout))
}

/// A subtitle stream inside the file, as WebVTT.
pub fn extract_subtitle(tools: &Tools, path: &Path, index: u32) -> Result<String, String> {
    let ff = tools.ffmpeg.as_ref().ok_or("ffmpeg isn't installed, so subtitles inside the video can't be shown")?;
    let o = quiet(Command::new(ff).args(["-hide_banner", "-loglevel", "error", "-i"]).arg(path).args(["-map", &format!("0:{index}"), "-f", "webvtt", "pipe:1"]))
        .output().map_err(|e| format!("Couldn't run ffmpeg: {e}"))?;
    if !o.status.success() { return Err(format!("Couldn't read those subtitles: {}", last_lines(&String::from_utf8_lossy(&o.stderr)))); }
    Ok(String::from_utf8_lossy(&o.stdout).into_owned())
}

fn last_lines(s: &str) -> String { s.lines().filter(|l| !l.trim().is_empty()).rev().take(3).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join(" ") }

// ------------------------------------------------------------------ edits

fn d0() -> f64 { 0.0 }

/// What the edit panel asks for. Times are seconds in the original file.
#[derive(Deserialize, Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Edit {
    pub input: String,
    /// Where to save; empty: "<name> (edited).<ext>" beside the original.
    pub output: String,
    #[serde(default = "d0")]
    pub start: f64,
    /// 0 or past the end: to the end.
    pub end: f64,
    /// A part in the middle to leave out.
    pub cut_start: Option<f64>,
    pub cut_end: Option<f64>,
    /// Clockwise: 0, 90, 180, 270.
    pub rotate: i32,
    pub hflip: bool,
    pub vflip: bool,
    pub mute: bool,
    pub fade_in: f64,
    pub fade_out: f64,
    pub normalize: bool,
    /// Cut exactly where asked (re-encodes); otherwise trims copy the streams and start at the nearest keyframe.
    pub exact: bool,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct Plan {
    /// ffmpeg argument lists, run one after the other.
    pub passes: Vec<Vec<String>>,
    /// Files to write before running (the concat list).
    pub files: Vec<(PathBuf, String)>,
    /// Temporary files to delete afterwards.
    pub temp: Vec<PathBuf>,
    pub output: PathBuf,
    /// How long the result is, for the progress bar.
    pub duration: f64,
    pub reencode: bool,
    /// Said under the progress bar: "Copying the streams (no quality loss)" or which encoder.
    pub how: String,
    /// The result is a song: copy its tags and cover over afterwards.
    pub audio: bool,
}

fn f(t: f64) -> String { format!("{:.3}", t.max(0.0)) }
fn s(v: &[&str]) -> Vec<String> { v.iter().map(|x| x.to_string()).collect() }
fn has(enc: &[String], name: &str) -> bool { enc.iter().any(|e| e == name) }
fn ext_of(p: &Path) -> String { p.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default() }

/// The video encoder (and its quality settings) for a container, from what this ffmpeg has.
pub fn video_encoder(ext: &str, enc: &[String], bit_rate: u64) -> Option<(String, Vec<String>)> {
    let br = format!("{}k", ((bit_rate / 1000).clamp(1500, 40000)));
    let webm = ext == "webm";
    let order: &[&str] = if webm { &["libvpx-vp9", "libsvtav1", "libaom-av1", "libvpx"] } else { &["libx264", "libopenh264", "h264_qsv", "h264_nvenc", "h264_mf", "mpeg4"] };
    for e in order {
        if !has(enc, e) { continue; }
        let opts: Vec<String> = match *e {
            "libx264" => s(&["-crf", "18", "-preset", "medium", "-pix_fmt", "yuv420p"]),
            "libvpx-vp9" => s(&["-crf", "30", "-b:v", "0", "-row-mt", "1"]),
            "libsvtav1" => s(&["-crf", "32", "-preset", "8"]),
            "libaom-av1" => s(&["-crf", "32", "-b:v", "0", "-cpu-used", "6"]),
            "mpeg4" => s(&["-q:v", "2"]),
            _ => vec!["-b:v".into(), br.clone()],
        };
        let mut o = opts;
        if !webm && *e != "libx264" { o.extend(s(&["-pix_fmt", "yuv420p"])); }
        return Some((e.to_string(), o));
    }
    None
}

/// The audio encoder for a container (and the source's codec, so an Opus .ogg stays Opus).
pub fn audio_encoder(ext: &str, src_codec: &str, enc: &[String], src_fmt_bits: u32) -> Option<(String, Vec<String>)> {
    let pick = |list: &[(&str, &[&str])]| list.iter().find(|(n, _)| has(enc, n)).map(|(n, o)| (n.to_string(), s(o)));
    match ext {
        "mp3" => pick(&[("libmp3lame", &["-q:a", "2"]), ("mp3_mf", &["-b:a", "256k"])]),
        "flac" => pick(&[("flac", &["-compression_level", "5"])]),
        "ogg" | "oga" if src_codec == "opus" => pick(&[("libopus", &["-b:a", "192k"]), ("opus", &["-b:a", "192k", "-strict", "-2"])]),
        "ogg" | "oga" => pick(&[("libvorbis", &["-q:a", "6"]), ("vorbis", &["-q:a", "6", "-strict", "-2"]), ("libopus", &["-b:a", "192k"])]),
        "opus" => pick(&[("libopus", &["-b:a", "192k"]), ("opus", &["-b:a", "192k", "-strict", "-2"])]),
        "m4a" | "m4b" | "aac" | "mp4" | "mov" | "m4v" | "mkv" | "mka" => pick(&[("aac", &["-b:a", "256k"]), ("libfdk_aac", &["-b:a", "256k"]), ("libopus", &["-b:a", "192k"])]),
        "webm" => pick(&[("libopus", &["-b:a", "160k"]), ("libvorbis", &["-q:a", "5"])]),
        "wav" => if src_fmt_bits > 16 { pick(&[("pcm_s24le", &[])]) } else { pick(&[("pcm_s16le", &[])]) },
        "aiff" | "aif" => pick(&[("pcm_s16be", &[])]),
        "wma" => pick(&[("wmav2", &["-b:a", "192k"])]),
        "wv" => pick(&[("wavpack", &[])]),
        _ => None,
    }
}

/// Plans an edit: which ffmpeg runs it takes. `tmp` is a folder for the pieces of a copied cut.
pub fn plan(e: &Edit, p: &Probe, enc: &[String], tmp: &Path) -> Result<Plan, String> {
    let input = PathBuf::from(&e.input);
    let video = p.video().is_some();
    if !video && p.audio().is_none() { return Err("There's no sound or picture in this file to edit.".into()); }
    let dur = if p.duration > 0.0 { p.duration } else { return Err("Couldn't tell how long this file is.".into()) };
    let start = e.start.clamp(0.0, dur);
    let end = if e.end <= 0.0 || e.end > dur { dur } else { e.end };
    if end - start < 0.1 { return Err("The part to keep is too short. Move the start and end marks apart.".into()); }
    let cut = match (e.cut_start, e.cut_end) {
        (Some(a), Some(b)) => {
            let (a, b) = (a.max(start), b.min(end));
            if b - a < 0.05 { None } else { Some((a, b)) }
        }
        _ => None,
    };
    let kept = end - start - cut.map(|(a, b)| b - a).unwrap_or(0.0);
    if kept < 0.1 { return Err("Cutting that out would leave nothing.".into()); }
    let mut src_ext = ext_of(&input);
    if src_ext.is_empty() { src_ext = if video { "mp4".into() } else { "m4a".into() }; }
    let transform = video && (e.rotate.rem_euclid(360) != 0 || e.hflip || e.vflip);
    let fades = !video && (e.fade_in > 0.0 || e.fade_out > 0.0 || e.normalize);
    let mp4ish = ["mp4", "mov", "m4v"].contains(&src_ext.as_str());
    // Rotating or flipping an MP4/MOV can be done by changing how it's shown (no re-encoding).
    let meta_transform = transform && !e.exact && mp4ish && cut.is_none();
    let reencode = e.exact || fades || (transform && !meta_transform) || (!video && cut.is_some());

    // Where the result goes: a container the chosen encoders can write.
    let mut out_ext = if e.output.is_empty() { src_ext.clone() } else { ext_of(Path::new(&e.output)) };
    if reencode && video && !["mp4", "mov", "m4v", "mkv", "webm"].contains(&out_ext.as_str()) { out_ext = "mp4".into(); }
    let src_codec = p.audio().map(|a| a.codec.clone()).unwrap_or_default();
    if reencode && !video && audio_encoder(&out_ext, &src_codec, enc, 16).is_none() { out_ext = "flac".into(); }
    let output = if e.output.is_empty() {
        crate::media::copy_name(&input, "edited", Some(&out_ext))
    } else {
        let o = PathBuf::from(&e.output);
        if ext_of(&o) == out_ext { o } else { o.with_extension(&out_ext) }
    };
    if output == input { return Err("Edits are saved as a new file; pick another name.".into()); }
    if output.exists() { return Err(format!("{} already exists. Pick another name.", output.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default())); }
    let out = output.to_string_lossy().into_owned();
    let inp = e.input.clone();
    let mut plan = Plan { output: output.clone(), duration: kept, reencode, audio: !video, ..Default::default() };
    let base = s(&["-hide_banner", "-nostdin", "-loglevel", "error", "-progress", "pipe:1", "-nostats"]);
    let has_audio = p.audio().is_some() && !e.mute;

    if !reencode {
        let mut display: Vec<String> = vec![];
        if meta_transform {
            let existing = p.video().map(|v| v.rotation).unwrap_or(0);
            let ccw = (existing - e.rotate).rem_euclid(360);
            display.extend(["-display_rotation:v:0".into(), ccw.to_string()]);
            if e.hflip { display.push("-display_hflip:v:0".into()); }
            if e.vflip { display.push("-display_vflip:v:0".into()); }
        }
        let maps = |v: &mut Vec<String>| {
            if video {
                v.extend(s(&["-map", "0:v?"]));
                if has_audio { v.extend(s(&["-map", "0:a?"])); }
                v.extend(s(&["-map", "0:s?"]));
                if src_ext == "mkv" { v.extend(s(&["-map", "0:t?"])); }
            } else {
                v.extend(s(&["-map", "0:a:0"]));
            }
            v.extend(s(&["-c", "copy", "-map_metadata", "0", "-avoid_negative_ts", "make_zero"]));
        };
        let piece = |from: f64, len: f64, to: &str, overwrite: bool| {
            let mut v = base.clone();
            v.push(if overwrite { "-y".into() } else { "-n".into() });
            v.extend(display.clone());
            if from > 0.0 { v.extend(["-ss".into(), f(from)]); }
            v.extend(["-i".into(), inp.clone(), "-t".into(), f(len)]);
            maps(&mut v);
            if out_ext == "mp4" || out_ext == "m4v" || out_ext == "mov" { v.extend(s(&["-movflags", "+faststart"])); }
            v.push(to.to_string());
            v
        };
        match cut {
            None => plan.passes.push(piece(start, end - start, &out, false)),
            Some((a, b)) => {
                let mut parts = vec![];
                if a - start >= 0.05 { parts.push((start, a - start)); }
                if end - b >= 0.05 { parts.push((b, end - b)); }
                if parts.len() == 1 {
                    plan.passes.push(piece(parts[0].0, parts[0].1, &out, false));
                } else {
                    let mut list = String::from("ffconcat version 1.0\n");
                    for (i, (from, len)) in parts.iter().enumerate() {
                        let t = tmp.join(format!("part{i}.{src_ext}"));
                        plan.passes.push(piece(*from, *len, &t.to_string_lossy(), true));
                        list.push_str(&format!("file '{}'\n", t.to_string_lossy().replace('\'', "'\\''")));
                        plan.temp.push(t);
                    }
                    let lf = tmp.join("parts.txt");
                    plan.files.push((lf.clone(), list));
                    plan.temp.push(lf.clone());
                    let mut v = base.clone();
                    v.extend(s(&["-n", "-f", "concat", "-safe", "0", "-i"]));
                    v.push(lf.to_string_lossy().into_owned());
                    v.extend(s(&["-map", "0", "-c", "copy"]));
                    if mp4ish { v.extend(s(&["-movflags", "+faststart"])); }
                    v.push(out.clone());
                    plan.passes.push(v);
                }
            }
        }
        plan.how = if meta_transform { "Turning it without re-encoding (no quality loss); trims start at the nearest keyframe".into() }
            else if video { "Copying the streams (no quality loss); the start moves to the nearest keyframe".into() }
            else { "Copying the sound as it is (no quality loss)".into() };
        return Ok(plan);
    }

    // Re-encoding: one run with filters.
    let mut v = base.clone();
    v.push("-n".into());
    if start > 0.0 { v.extend(["-ss".into(), f(start)]); }
    if end < dur { v.extend(["-to".into(), f(end)]); }
    v.extend(["-i".into(), inp.clone()]);
    let rel = cut.map(|(a, b)| (a - start, b - start));
    let mut how = vec![];
    if video {
        let mut vf: Vec<String> = vec![];
        if let Some((a, b)) = rel { vf.push(format!("select='not(between(t,{},{}))',setpts=N/FRAME_RATE/TB", f(a), f(b))); }
        match e.rotate.rem_euclid(360) { 90 => vf.push("transpose=1".into()), 180 => vf.push("transpose=1,transpose=1".into()), 270 => vf.push("transpose=2".into()), _ => {} }
        if e.hflip { vf.push("hflip".into()); }
        if e.vflip { vf.push("vflip".into()); }
        let (venc, vopts) = video_encoder(&out_ext, enc, p.bit_rate).ok_or("This ffmpeg has no video encoder for that kind of file.")?;
        v.extend(s(&["-map", "0:v:0"]));
        if !vf.is_empty() { v.extend(["-vf".into(), vf.join(",")]); }
        v.extend(["-c:v".into(), venc.clone()]);
        v.extend(vopts);
        how.push(format!("video with {venc}"));
        if has_audio {
            let (aenc, aopts) = audio_encoder(&out_ext, &src_codec, enc, 16).ok_or("This ffmpeg has no sound encoder for that kind of file.")?;
            v.extend(s(&["-map", "0:a:0"]));
            if let Some((a, b)) = rel { v.extend(["-af".into(), format!("aselect='not(between(t,{},{}))',asetpts=N/SR/TB", f(a), f(b))]); }
            v.extend(["-c:a".into(), aenc.clone()]);
            v.extend(aopts);
            how.push(format!("sound with {aenc}"));
        } else {
            v.push("-an".into());
        }
        v.extend(s(&["-map_metadata", "0"]));
        if ["mp4", "mov", "m4v"].contains(&out_ext.as_str()) { v.extend(s(&["-movflags", "+faststart"])); }
    } else {
        let a = p.audio().unwrap();
        let mut af: Vec<String> = vec![];
        if let Some((x, y)) = rel { af.push(format!("aselect='not(between(t,{},{}))',asetpts=N/SR/TB", f(x), f(y))); }
        if e.fade_in > 0.0 { af.push(format!("afade=t=in:st=0:d={}", f(e.fade_in.min(kept)))); }
        if e.fade_out > 0.0 { let d = e.fade_out.min(kept); af.push(format!("afade=t=out:st={}:d={}", f(kept - d), f(d))); }
        if e.normalize { af.push("loudnorm=I=-16:TP=-1.5:LRA=11".into()); }
        let bits = if a.codec.contains("24") || a.codec.contains("32") || a.codec == "flac" && a.sample_rate > 48000 { 24 } else { 16 };
        let (aenc, aopts) = audio_encoder(&out_ext, &src_codec, enc, bits).ok_or("This ffmpeg has no encoder for that kind of sound file.")?;
        v.extend(s(&["-map", "0:a:0", "-vn"]));
        if !af.is_empty() { v.extend(["-af".into(), af.join(",")]); }
        v.extend(["-c:a".into(), aenc.clone()]);
        v.extend(aopts);
        // loudnorm works at 192 kHz inside: keep the song's own rate.
        if a.sample_rate > 0 { v.extend(["-ar".into(), a.sample_rate.to_string()]); }
        v.extend(s(&["-map_metadata", "0"]));
        how.push(format!("re-encoding the sound with {aenc}"));
    }
    v.push(out);
    plan.passes.push(v);
    plan.how = format!("Re-encoding ({})", how.join(", "));
    Ok(plan)
}

/// Makes a copy of a file the web view couldn't play, that it can: MP4 with H.264 (or the video as it is when it
/// already is H.264/H.265/AV1/VP9) and AAC sound. `audio` picks the sound track (its number among sound tracks).
pub fn prepare_plan(p: &Probe, input: &Path, out: &Path, audio: Option<u32>, enc: &[String]) -> Result<Plan, String> {
    let mut v = s(&["-hide_banner", "-nostdin", "-loglevel", "error", "-progress", "pipe:1", "-nostats", "-y", "-i"]);
    v.push(input.to_string_lossy().into_owned());
    let mut how = vec![];
    if let Some(vs) = p.video() {
        v.extend(s(&["-map", "0:v:0"]));
        if ["h264", "hevc", "av1", "vp9"].contains(&vs.codec.as_str()) {
            v.extend(s(&["-c:v", "copy"]));
            if vs.codec == "hevc" { v.extend(s(&["-tag:v", "hvc1"])); }
            how.push("video as it is".to_string());
        } else {
            let (venc, mut o) = video_encoder("mp4", enc, p.bit_rate).ok_or("This ffmpeg can't convert that video.")?;
            if venc == "libx264" { o = s(&["-crf", "20", "-preset", "veryfast", "-pix_fmt", "yuv420p"]); }
            v.extend(["-c:v".into(), venc.clone()]);
            v.extend(o);
            how.push(format!("video with {venc}"));
        }
    }
    let n = audio.unwrap_or(0);
    if let Some(a) = p.streams.iter().filter(|s| s.kind == "audio").nth(n as usize) {
        v.extend(["-map".into(), format!("0:a:{n}")]);
        if ["aac", "mp3"].contains(&a.codec.as_str()) { v.extend(s(&["-c:a", "copy"])); }
        else { v.extend(s(&["-c:a", "aac", "-b:a", "192k", "-ac", "2"])); how.push("sound to AAC".into()); }
    }
    v.extend(s(&["-sn", "-dn", "-movflags", "+faststart", "-f", "mp4"]));
    v.push(out.to_string_lossy().into_owned());
    Ok(Plan { passes: vec![v], output: out.to_path_buf(), duration: p.duration, reencode: !how.iter().all(|h| h == "video as it is"), how: how.join(", "), ..Default::default() })
}

/// One frame as a PNG (when the web view can't draw it itself).
pub fn frame_args(input: &Path, at: f64, out: &Path) -> Vec<String> {
    let mut v = s(&["-hide_banner", "-nostdin", "-loglevel", "error", "-n", "-ss"]);
    v.push(f(at));
    v.extend(["-i".into(), input.to_string_lossy().into_owned()]);
    v.extend(s(&["-frames:v", "1", "-update", "1"]));
    v.push(out.to_string_lossy().into_owned());
    v
}

/// Seconds done, from ffmpeg's -progress lines ("out_time_us=1234567").
pub fn progress_seconds(line: &str) -> Option<f64> {
    let (k, v) = line.trim().split_once('=')?;
    match k {
        "out_time_us" | "out_time_ms" => v.trim().parse::<f64>().ok().map(|us| us / 1_000_000.0),
        _ => None,
    }
}

/// A running job's Stop button.
#[derive(Clone, Default)]
pub struct Cancel(pub Arc<AtomicBool>);

/// Runs a plan: each pass in turn, progress (0 to 1) as it goes. Stopping or failing deletes the half-made file.
pub fn run(tools: &Tools, plan: &Plan, progress: &dyn Fn(f64), cancel: &Cancel) -> Result<PathBuf, String> {
    let ff = tools.ffmpeg.clone().ok_or("ffmpeg isn't installed")?;
    for (p, text) in &plan.files {
        if let Some(d) = p.parent() { std::fs::create_dir_all(d).map_err(|e| e.to_string())?; }
        std::fs::write(p, text).map_err(|e| e.to_string())?;
    }
    let n = plan.passes.len() as f64;
    let cleanup = |also_output: bool| {
        for t in &plan.temp { let _ = std::fs::remove_file(t); }
        if also_output { let _ = std::fs::remove_file(&plan.output); }
    };
    // Pieces of a copied cut take most of the time; the last pass of several is the quick join.
    for (i, args) in plan.passes.iter().enumerate() {
        let span = if plan.passes.len() > 1 && i == plan.passes.len() - 1 { None } else { Some(plan.duration / (if n > 1.0 { n - 1.0 } else { 1.0 })) };
        let mut child = quiet(Command::new(&ff).args(args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()))
            .spawn().map_err(|e| format!("Couldn't start ffmpeg: {e}"))?;
        let err_text = Arc::new(Mutex::new(String::new()));
        let et = err_text.clone();
        let mut stderr = child.stderr.take().unwrap();
        let err_thread = std::thread::spawn(move || { let mut s = String::new(); let _ = stderr.read_to_string(&mut s); *et.lock().unwrap() = s; });
        let stdout = child.stdout.take().unwrap();
        let done = Arc::new(Mutex::new(0.0f64));
        let d2 = done.clone();
        let out_thread = std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if let Some(t) = progress_seconds(&line) { *d2.lock().unwrap() = t; }
            }
        });
        let status = loop {
            if cancel.0.load(Ordering::SeqCst) {
                let _ = child.kill();
                let _ = child.wait();
                let _ = out_thread.join();
                let _ = err_thread.join();
                cleanup(true);
                return Err("Stopped. Nothing was saved.".into());
            }
            if let Some(st) = child.try_wait().map_err(|e| e.to_string())? { break st; }
            let t = *done.lock().unwrap();
            let frac = match span { Some(sp) if sp > 0.0 => (t / sp).clamp(0.0, 1.0), _ => 0.5 };
            progress(((i as f64) + frac) / n);
            std::thread::sleep(std::time::Duration::from_millis(150));
        };
        let _ = out_thread.join();
        let _ = err_thread.join();
        if !status.success() {
            let e = err_text.lock().unwrap().clone();
            cleanup(true);
            return Err(format!("ffmpeg couldn't do it: {}", if e.trim().is_empty() { format!("it stopped with {status}") } else { last_lines(&e) }));
        }
    }
    cleanup(false);
    progress(1.0);
    Ok(plan.output.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn sample_video() -> Probe {
        parse_probe(r#"{"streams":[
          {"index":0,"codec_type":"video","codec_name":"h264","width":1920,"height":1080,"disposition":{"default":1,"attached_pic":0},"side_data_list":[{"rotation":-90}]},
          {"index":1,"codec_type":"audio","codec_name":"aac","channels":6,"sample_rate":"48000","tags":{"language":"eng","title":"Main"},"disposition":{"default":1}},
          {"index":2,"codec_type":"audio","codec_name":"ac3","channels":2,"sample_rate":"48000","tags":{"language":"fre"}},
          {"index":3,"codec_type":"subtitle","codec_name":"subrip","tags":{"language":"spa"},"disposition":{"forced":1}},
          {"index":4,"codec_type":"subtitle","codec_name":"hdmv_pgs_subtitle","tags":{"language":"eng"}}],
         "chapters":[{"start_time":"0.000000","end_time":"60.5","tags":{"title":"Opening"}},{"start_time":"60.5","end_time":"120"}],
         "format":{"format_name":"mov,mp4,m4a,3gp,3g2,mj2","duration":"120.000000","bit_rate":"5000000","tags":{"title":"Film"}}}"#).unwrap()
    }

    fn sample_song() -> Probe {
        parse_probe(r#"{"streams":[
          {"index":0,"codec_type":"audio","codec_name":"mp3","channels":2,"sample_rate":"44100"},
          {"index":1,"codec_type":"video","codec_name":"mjpeg","disposition":{"attached_pic":1}}],
          "format":{"format_name":"mp3","duration":"200.5","tags":{"ARTIST":"Band","title":"Song"}}}"#).unwrap()
    }

    fn enc() -> Vec<String> { ["libx264", "aac", "libmp3lame", "flac", "libopus", "libvorbis", "libvpx-vp9", "pcm_s16le"].iter().map(|s| s.to_string()).collect() }

    fn joined(p: &Plan, i: usize) -> String { p.passes[i].join(" ") }

    #[test]
    fn reads_probe() {
        let p = sample_video();
        assert_eq!(p.duration, 120.0);
        assert_eq!(p.streams.len(), 5);
        assert_eq!(p.streams[0].rotation, -90);
        assert_eq!(p.streams[1].label, "Main (English, AAC 5.1)");
        assert_eq!(p.streams[2].label, "French (AC3 stereo)");
        assert_eq!(p.streams[2].nth, 1);
        assert!(p.streams[3].text && p.streams[3].forced);
        assert!(!p.streams[4].text);
        assert_eq!(p.chapters[1].title, "Chapter 2");
        assert_eq!(p.tags["title"], "Film");
        let s = sample_song();
        assert_eq!(s.streams[1].kind, "cover");
        assert!(s.video().is_none());
        assert_eq!(s.tags["artist"], "Band");
    }

    #[test]
    fn encoders_list() {
        let t = " V..... = Video\n ------\n V....D libx264              libx264 H.264\n A....D aac                  AAC\n";
        assert_eq!(parse_encoders(t), vec!["libx264", "aac"]);
    }

    #[test]
    fn copy_trim() {
        let tmp = std::env::temp_dir();
        let e = Edit { input: "/v/clip.mp4".into(), start: 10.0, end: 20.0, ..Default::default() };
        let p = plan(&e, &sample_video(), &enc(), &tmp).unwrap();
        assert!(!p.reencode);
        assert_eq!(p.passes.len(), 1);
        let j = joined(&p, 0);
        assert!(j.contains("-n -ss 10.000 -i /v/clip.mp4 -t 10.000 -map 0:v? -map 0:a? -map 0:s? -c copy"), "{j}");
        assert!(j.ends_with("-movflags +faststart /v/clip (edited).mp4"), "{j}");
        assert_eq!(p.duration, 10.0);
    }

    #[test]
    fn copy_cut_middle_joins_two_pieces() {
        let tmp = PathBuf::from("/tmp/x");
        let e = Edit { input: "/v/clip.mkv".into(), cut_start: Some(30.0), cut_end: Some(40.0), mute: true, ..Default::default() };
        let p = plan(&e, &sample_video(), &enc(), &tmp).unwrap();
        assert_eq!(p.passes.len(), 3);
        assert!(joined(&p, 0).contains("-y -i /v/clip.mkv -t 30.000 -map 0:v? -map 0:s? -map 0:t? -c copy"), "{}", joined(&p, 0));
        assert!(joined(&p, 1).contains("-ss 40.000 -i /v/clip.mkv -t 80.000"));
        assert!(p.files[0].1.contains("file '/tmp/x/part0.mkv'\nfile '/tmp/x/part1.mkv'"));
        assert!(joined(&p, 2).contains("-f concat -safe 0 -i /tmp/x/parts.txt -map 0 -c copy /v/clip (edited).mkv"));
        assert_eq!(p.duration, 110.0);
    }

    #[test]
    fn rotating_mp4_uses_the_display_matrix() {
        let e = Edit { input: "/v/clip.mp4".into(), rotate: 90, hflip: true, ..Default::default() };
        let p = plan(&e, &sample_video(), &enc(), Path::new("/tmp")).unwrap();
        assert!(!p.reencode);
        // Already -90 (shown turned right); another 90 clockwise: upside down.
        assert!(joined(&p, 0).contains("-display_rotation:v:0 180 -display_hflip:v:0 -i /v/clip.mp4"), "{}", joined(&p, 0));
    }

    #[test]
    fn exact_cut_reencodes_with_filters() {
        let e = Edit { input: "/v/clip.avi".into(), start: 5.0, end: 65.0, cut_start: Some(20.0), cut_end: Some(25.0), rotate: 270, exact: true, ..Default::default() };
        let p = plan(&e, &sample_video(), &enc(), Path::new("/tmp")).unwrap();
        assert!(p.reencode);
        let j = joined(&p, 0);
        assert!(j.contains("-ss 5.000 -to 65.000 -i /v/clip.avi"), "{j}");
        assert!(j.contains("-vf select='not(between(t,15.000,20.000))',setpts=N/FRAME_RATE/TB,transpose=2 -c:v libx264 -crf 18"), "{j}");
        assert!(j.contains("-af aselect='not(between(t,15.000,20.000))',asetpts=N/SR/TB -c:a aac"), "{j}");
        assert!(j.ends_with("/v/clip (edited).mp4"), "AVI becomes MP4 when re-encoded: {j}");
        assert_eq!(p.duration, 55.0);
    }

    #[test]
    fn song_trim_copies_and_fades_reencode() {
        let e = Edit { input: "/m/song.mp3".into(), start: 1.0, end: 31.0, ..Default::default() };
        let p = plan(&e, &sample_song(), &enc(), Path::new("/tmp")).unwrap();
        assert!(!p.reencode && p.audio);
        assert!(joined(&p, 0).contains("-ss 1.000 -i /m/song.mp3 -t 30.000 -map 0:a:0 -c copy"));
        let e = Edit { input: "/m/song.mp3".into(), end: 100.0, fade_in: 2.0, fade_out: 5.0, normalize: true, ..Default::default() };
        let p = plan(&e, &sample_song(), &enc(), Path::new("/tmp")).unwrap();
        let j = joined(&p, 0);
        assert!(j.contains("-to 100.000 -i /m/song.mp3 -map 0:a:0 -vn -af afade=t=in:st=0:d=2.000,afade=t=out:st=95.000:d=5.000,loudnorm=I=-16:TP=-1.5:LRA=11 -c:a libmp3lame -q:a 2 -ar 44100"), "{j}");
    }

    #[test]
    fn refuses_bad_edits() {
        let v = sample_video();
        assert!(plan(&Edit { input: "/v/a.mp4".into(), start: 50.0, end: 50.05, ..Default::default() }, &v, &enc(), Path::new("/tmp")).is_err());
        assert!(plan(&Edit { input: "/v/a.mp4".into(), output: "/v/a.mp4".into(), ..Default::default() }, &v, &enc(), Path::new("/tmp")).is_err());
        assert!(plan(&Edit { input: "/v/a.mp4".into(), exact: true, ..Default::default() }, &v, &[], Path::new("/tmp")).is_err());
    }

    #[test]
    fn prepare_for_playback() {
        let p = sample_video();
        let pl = prepare_plan(&p, Path::new("/v/a.mkv"), Path::new("/c/a.mp4"), Some(1), &enc()).unwrap();
        let j = joined(&pl, 0);
        assert!(j.contains("-map 0:v:0 -c:v copy -map 0:a:1 -c:a aac -b:a 192k -ac 2"), "{j}");
    }

    #[test]
    fn progress_lines() {
        assert_eq!(progress_seconds("out_time_us=1500000"), Some(1.5));
        assert_eq!(progress_seconds("frame=10"), None);
    }
}
