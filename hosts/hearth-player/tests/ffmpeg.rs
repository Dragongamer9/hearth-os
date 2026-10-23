//! The edits for real, with the ffmpeg on this computer: a short clip and a song are made with ffmpeg, then
//! trimmed, cut, turned, muted, faded and converted the way the edit panel asks, and the results are checked with
//! ffprobe. Skipped (with a note) when there's no ffmpeg.

use hearth_player::{media, tags, tools};
use std::path::{Path, PathBuf};
use std::process::Command;

fn dir() -> PathBuf {
    let d = std::env::temp_dir().join(format!("hp-ffmpeg-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn make(args: &[&str]) {
    let o = Command::new("ffmpeg").args(["-hide_banner", "-loglevel", "error", "-y"]).args(args).output().expect("ffmpeg");
    assert!(o.status.success(), "ffmpeg {:?}: {}", args, String::from_utf8_lossy(&o.stderr));
}

fn setup() -> Option<(tools::Tools, PathBuf)> {
    let t = tools::detect();
    if t.ffmpeg.is_none() || t.ffprobe.is_none() { eprintln!("no ffmpeg here: skipped"); return None; }
    eprintln!("{} with {} encoders", t.version, t.encoders.len());
    Some((t, dir()))
}

fn run(t: &tools::Tools, e: tools::Edit, tmp: &Path) -> (tools::Plan, PathBuf) {
    let p = tools::probe(t, Path::new(&e.input)).unwrap();
    let plan = tools::plan(&e, &p, &t.encoders, tmp).unwrap();
    let last = std::sync::Mutex::new(0.0);
    let out = tools::run(t, &plan, &|f| { *last.lock().unwrap() = f; }, &tools::Cancel::default()).unwrap_or_else(|e| panic!("{e}\n{:?}", plan.passes));
    assert_eq!(*last.lock().unwrap(), 1.0);
    (plan, out)
}

#[test]
fn edits_video_and_music() {
    let Some((t, d)) = setup() else { return };
    let clip = d.join("clip.mp4");
    // 10 s, keyframe every second, with sound, two chapters.
    let meta = d.join("chapters.txt");
    std::fs::write(&meta, ";FFMETADATA1\n[CHAPTER]\nTIMEBASE=1/1000\nSTART=0\nEND=5000\ntitle=Opening\n[CHAPTER]\nTIMEBASE=1/1000\nSTART=5000\nEND=10000\ntitle=Ending\n").unwrap();
    let venc = if t.encoders.iter().any(|e| e == "libx264") { "libx264" } else { "mpeg4" };
    make(&["-f", "lavfi", "-i", "testsrc2=size=320x240:rate=25:duration=10", "-f", "lavfi", "-i", "sine=f=440:duration=10",
        "-i", meta.to_str().unwrap(), "-map", "0:v", "-map", "1:a", "-map_metadata", "2", "-map_chapters", "2",
        "-c:v", venc, "-g", "25", "-c:a", "aac", "-shortest", clip.to_str().unwrap()]);
    let p = tools::probe(&t, &clip).unwrap();
    assert!((p.duration - 10.0).abs() < 0.3, "{}", p.duration);
    assert_eq!(p.chapters.len(), 2);
    assert_eq!(p.chapters[1].title, "Ending");

    // Trim (stream copy).
    let (plan, out) = run(&t, tools::Edit { input: clip.to_string_lossy().into(), start: 2.0, end: 6.0, ..Default::default() }, &d);
    assert!(!plan.reencode);
    assert!(out.ends_with("clip (edited).mp4"));
    let q = tools::probe(&t, &out).unwrap();
    assert!((q.duration - 4.0).abs() < 1.1, "copied trim is about 4 s: {}", q.duration);

    // Cut out the middle (copy: two pieces joined).
    let (plan, out) = run(&t, tools::Edit { input: clip.to_string_lossy().into(), cut_start: Some(3.0), cut_end: Some(7.0), ..Default::default() }, &d);
    assert_eq!(plan.passes.len(), 3);
    let q = tools::probe(&t, &out).unwrap();
    assert!((q.duration - 6.0).abs() < 1.1, "6 s left: {}", q.duration);
    for f in &plan.temp { assert!(!f.exists(), "the pieces are cleaned up"); }

    // Exact cut with a turn and no sound (re-encoded).
    let (plan, out) = run(&t, tools::Edit { input: clip.to_string_lossy().into(), start: 1.0, end: 9.0, cut_start: Some(4.0), cut_end: Some(5.0), rotate: 90, mute: true, exact: true, ..Default::default() }, &d);
    assert!(plan.reencode);
    let q = tools::probe(&t, &out).unwrap();
    assert!((q.duration - 7.0).abs() < 0.3, "exactly 7 s: {}", q.duration);
    let v = q.video().unwrap();
    assert_eq!((v.width, v.height), (240, 320), "turned on its side");
    assert!(q.audio().is_none(), "no sound");

    // Turning an MP4 without re-encoding: only how it's shown changes.
    let (plan, out) = run(&t, tools::Edit { input: clip.to_string_lossy().into(), rotate: 270, ..Default::default() }, &d);
    assert!(!plan.reencode);
    let q = tools::probe(&t, &out).unwrap();
    assert_eq!(q.video().unwrap().rotation, 90, "{:?}", q.video());

    // A frame as a picture.
    let png = d.join("frame.png");
    let o = Command::new(t.ffmpeg.as_ref().unwrap()).args(tools::frame_args(&clip, 2.5, &png)).output().unwrap();
    assert!(o.status.success() && png.exists());

    // Getting a file ready to play: MKV with AC-3 sound becomes MP4 with AAC.
    let mkv = d.join("film.mkv");
    make(&["-i", clip.to_str().unwrap(), "-f", "lavfi", "-i", "sine=f=220:duration=10", "-map", "0:v", "-map", "0:a", "-map", "1:a", "-c:v", "copy", "-c:a", "ac3", mkv.to_str().unwrap()]);
    let p = tools::probe(&t, &mkv).unwrap();
    assert_eq!(p.streams.iter().filter(|s| s.kind == "audio").count(), 2);
    let ready = d.join("ready.mp4");
    let plan = tools::prepare_plan(&p, &mkv, &ready, Some(1), &t.encoders).unwrap();
    tools::run(&t, &plan, &|_| {}, &tools::Cancel::default()).unwrap();
    let q = tools::probe(&t, &ready).unwrap();
    assert_eq!(q.audio().unwrap().codec, "aac");

    // Subtitles inside a file, as WebVTT.
    let srt = d.join("clip.srt");
    std::fs::write(&srt, "1\n00:00:01,000 --> 00:00:03,000\nHello <i>there</i>\n").unwrap();
    let withsubs = d.join("subs.mkv");
    make(&["-i", clip.to_str().unwrap(), "-i", srt.to_str().unwrap(), "-map", "0", "-map", "1", "-c", "copy", withsubs.to_str().unwrap()]);
    let p = tools::probe(&t, &withsubs).unwrap();
    let s = p.streams.iter().find(|s| s.kind == "subtitle").unwrap();
    assert!(s.text);
    let vtt = tools::extract_subtitle(&t, &withsubs, s.index).unwrap();
    assert!(vtt.starts_with("WEBVTT") && vtt.contains("Hello"), "{vtt}");
    assert!(media::sidecar_subs(&clip).iter().any(|x| x.path.ends_with("clip.srt")));

    // Songs: trim by copying, then fades and loudness (re-encoded); tags and cover come along.
    let song = d.join("song.mp3");
    make(&["-f", "lavfi", "-i", "sine=f=330:duration=20", "-c:a", "libmp3lame", "-q:a", "4", song.to_str().unwrap()]);
    let cover = d.join("cover.png");
    make(&["-f", "lavfi", "-i", "color=c=orange:size=64x64", "-frames:v", "1", cover.to_str().unwrap()]);
    tags::write(&song, &tags::Tags { title: "Ember Song".into(), artist: "Hearth Band".into(), album: "Kiln".into(), track: Some(2), year: Some(2026), ..Default::default() },
        &tags::CoverChange { file: Some(cover.to_string_lossy().into()), remove: false }).unwrap();
    let r = tags::read(&song).unwrap();
    assert_eq!((r.title.as_str(), r.track, r.year), ("Ember Song", Some(2), Some(2026)));
    assert!(r.has_cover && tags::cover(&song).is_some());
    let (plan, out) = run(&t, tools::Edit { input: song.to_string_lossy().into(), start: 5.0, end: 15.0, ..Default::default() }, &d);
    assert!(!plan.reencode && plan.audio);
    tags::copy(&song, &out).unwrap();
    assert!((tools::probe(&t, &out).unwrap().duration - 10.0).abs() < 0.3);
    assert_eq!(tags::read(&out).unwrap().title, "Ember Song");
    assert!(tags::read(&out).unwrap().has_cover, "the cover comes along");
    let (plan, out) = run(&t, tools::Edit { input: song.to_string_lossy().into(), fade_in: 2.0, fade_out: 3.0, normalize: true, ..Default::default() }, &d);
    assert!(plan.reencode);
    let q = tools::probe(&t, &out).unwrap();
    assert_eq!(q.audio().unwrap().codec, "mp3");
    assert_eq!(q.audio().unwrap().sample_rate, 44100, "loudness evening doesn't change the sample rate");

    // Stopping a job deletes the half-made file.
    let e = tools::Edit { input: clip.to_string_lossy().into(), exact: true, output: d.join("stopped.mp4").to_string_lossy().into(), ..Default::default() };
    let p = tools::probe(&t, &clip).unwrap();
    let plan = tools::plan(&e, &p, &t.encoders, &d).unwrap();
    let c = tools::Cancel::default();
    c.0.store(true, std::sync::atomic::Ordering::SeqCst);
    assert!(tools::run(&t, &plan, &|_| {}, &c).is_err());
    assert!(!d.join("stopped.mp4").exists());
    let _ = std::fs::remove_dir_all(&d);
}
