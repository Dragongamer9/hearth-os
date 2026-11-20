//! Opening a disc (a drive, a .cue, an .iso or a disc's folder) and playing a title or a track from it.
//!
//! - Audio CD tracks play as WAV files from the local server (seeking is the web view's own).
//! - DVD titles and Video CD tracks are MPEG program streams the web view can't play, so ffmpeg reads them from the
//!   local server and sends Matroska on as it goes (the picture as it is, the sound as Opus or AAC). Seeking starts
//!   a new stream from the right sector (a DVD's next VOBU, whose own time is the time shown).
//! - DVD subtitles are pictures: with subtitles on, ffmpeg draws them into the picture (and the picture is encoded
//!   again: H.264, or MPEG-2 without it).

use crate::disc::{self, Cue, Device, Image, Sectors, Toc, VcdTrack, WavTrack};
use crate::dvd::{self, DvdIo, Title};
use crate::server::{Live, Server};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Serialize, Clone, Debug, Default)]
pub struct CdTrack {
    pub n: u32,
    pub duration: f64,
    pub title: String,
    pub artist: String,
    #[serde(skip)]
    pub start: u32,
    #[serde(skip)]
    pub sectors: u32,
}

/// What the page is told about a disc.
#[derive(Serialize, Clone, Debug, Default)]
pub struct Info {
    pub id: String,
    /// "dvd", "vcd", "svcd", "audio-cd", "data", "bluray", "blank", "empty".
    pub kind: String,
    pub label: String,
    pub device: Option<String>,
    pub source: String,
    pub titles: Vec<Title>,
    pub main: Option<u32>,
    pub tracks: Vec<CdTrack>,
    pub album: String,
    pub artist: String,
    /// Its film is scrambled (CSS) and can't be unscrambled here: libdvdcss isn't installed, or couldn't do it.
    pub encrypted: bool,
    pub dvdcss: bool,
    /// How it's read: "libdvdread", "folder", "drive", "image".
    pub reader: String,
    /// A data disc's files (its folder).
    pub folder: Option<String>,
    /// What to tell you when it can't be played here.
    pub note: String,
    /// A stable name for remembering where you stopped.
    pub key: String,
}

pub struct Opened {
    pub info: Info,
    dvd: Option<Arc<dyn DvdIo>>,
    sectors: Option<Arc<dyn Sectors>>,
    pub toc: Option<Toc>,
    urls: Mutex<Vec<String>>,
}

pub const BLURAY_NOTE: &str = "This is a Blu-ray disc. Hearth Player plays DVDs, Video CDs and music CDs, but not Blu-ray discs.";
pub const BLANK_NOTE: &str = "This disc is empty: there's nothing on it to play.";
pub const EMPTY_NOTE: &str = "There's no disc in the drive.";
/// The note for a DVD whose film is locked (CSS), and why.
pub const ENCRYPTED_NOTE: &str = "This DVD is locked against copying (most films from a shop are), so it needs DVD decoding to play. \
Hearth OS can't include DVD decoding for legal reasons, but you can add it yourself: it comes from RPM Fusion, a well-known \
add-on source for Fedora. Whether it's allowed to use where you live is up to you.";
pub const ENCRYPTED_NOTE_HAVE: &str = "This DVD is locked against copying, and DVD decoding couldn't unlock it. Some drives \
need their region set first, or the disc may be damaged. Try it again, or another drive.";

fn id_for(source: &str) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(source.as_bytes()).iter().take(6).map(|b| format!("{b:02x}")).collect()
}

fn label_from(s: &dyn Sectors) -> String { disc::iso_root(s, 0).map(|r| disc::pretty_label(&r.1)).unwrap_or_default() }

fn cd_tracks(toc: &Toc, text: Option<&disc::CdText>, only_audio: bool) -> Vec<CdTrack> {
    let mut v = Vec::new();
    for (i, t) in toc.tracks.iter().enumerate() {
        if only_audio != t.audio { continue; }
        let end = toc.end(i);
        let sectors = end.saturating_sub(t.lba);
        let title = text.and_then(|x| x.titles.get(t.n as usize)).filter(|s| !s.is_empty()).cloned().unwrap_or_else(|| format!("Track {}", t.n));
        let artist = text.and_then(|x| x.performers.get(t.n as usize).filter(|s| !s.is_empty()).or_else(|| x.performers.first())).cloned().unwrap_or_default();
        v.push(CdTrack { n: t.n, duration: sectors as f64 / 75.0, title, artist, start: t.lba, sectors });
    }
    v
}

fn dvd_info(info: &mut Info, io: &Arc<dyn DvdIo>) -> Result<(), String> {
    let titles = dvd::titles(io.as_ref()).map_err(|e| format!("Couldn't read the DVD's list of titles: {e}"))?;
    if titles.is_empty() { return Err("This DVD has no titles to play.".into()); }
    info.kind = "dvd".into();
    info.main = dvd::main_title(&titles);
    info.reader = io.how().into();
    info.dvdcss = dvd::have_dvdcss();
    let main = titles.iter().find(|t| Some(t.n) == info.main).unwrap_or(&titles[0]);
    info.encrypted = dvd::title_scrambled(io.as_ref(), main);
    if info.encrypted { info.note = if info.dvdcss { ENCRYPTED_NOTE_HAVE.into() } else { ENCRYPTED_NOTE.into() }; }
    info.key = format!("dvd:{}", dvd::disc_key(io.as_ref()));
    info.titles = titles;
    Ok(())
}

/// The Video CD's MPEG tracks (every track after the first, which holds its files).
fn vcd_info(info: &mut Info, src: &Arc<dyn Sectors>, toc: &Toc, kind: &str) {
    info.kind = kind.into();
    let mut tracks = cd_tracks(toc, None, false);
    if tracks.len() > 1 { tracks.remove(0); }
    for (i, t) in tracks.iter_mut().enumerate() {
        t.title = format!("Part {}", i + 1);
        t.duration = disc::vcd_duration(&VcdTrack { src: src.clone(), start: t.start, sectors: t.sectors });
    }
    info.key = format!("vcd:{}", disc::cddb_id(toc));
    info.tracks = tracks;
}

/// Opens what `path` names: a drive (/dev/sr0), a .cue, an .iso, or a folder (VIDEO_TS, a mounted disc).
pub fn open(path: &str) -> Result<Opened, String> {
    let mut info = Info { id: id_for(path), source: path.to_string(), ..Default::default() };
    let p = Path::new(path);
    let ext = p.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    let mut o = Opened { info: Info::default(), dvd: None, sectors: None, toc: None, urls: Mutex::new(Vec::new()) };
    let fin = |mut o: Opened, info: Info| { o.info = info; Ok(o) };

    if path.starts_with("/dev/") {
        info.device = Some(path.to_string());
        info.reader = "drive".into();
        let props = disc::udev_props(path);
        let mut kind = disc::kind_from_props(&props).to_string();
        let dev = Device::open(path).map_err(|e| format!("Couldn't open the drive: {e}"))?;
        let toc = dev.toc().ok();
        if props.is_empty() {
            kind = match &toc { None => String::new(), Some(t) if t.tracks.iter().any(|x| x.audio) => "audio-cd".into(), Some(_) => "data".into() };
        }
        let dev: Arc<dyn Sectors> = Arc::new(dev);
        info.label = props.get("ID_FS_LABEL").map(|l| disc::pretty_label(l)).unwrap_or_default();
        match kind.as_str() {
            "" => { info.kind = "empty".into(); info.note = EMPTY_NOTE.into(); }
            "bluray" => { info.kind = "bluray".into(); info.note = BLURAY_NOTE.into(); }
            "blank" => { info.kind = "blank".into(); info.note = BLANK_NOTE.into(); }
            "audio-cd" => {
                let toc = toc.ok_or("Couldn't read the CD's list of tracks.")?;
                let text = Device::open(path).ok().and_then(|d| d.cd_text());
                info.kind = "audio-cd".into();
                info.tracks = cd_tracks(&toc, text.as_ref(), true);
                if let Some(t) = &text { info.album = t.titles.first().cloned().unwrap_or_default(); info.artist = t.performers.first().cloned().unwrap_or_default(); }
                if info.label.is_empty() { info.label = if info.album.is_empty() { "Music CD".into() } else { info.album.clone() }; }
                info.key = format!("cd:{}", disc::musicbrainz_id(&toc));
                o.toc = Some(toc);
                o.sectors = Some(dev);
            }
            _ => {
                let (k, l) = disc::iso_kind(dev.as_ref(), 0);
                if info.label.is_empty() { info.label = disc::pretty_label(&l); }
                match k {
                    "dvd" => {
                        let io = dvd::open(p).map_err(|e| format!("Couldn't read the DVD: {e}"))?;
                        dvd_info(&mut info, &io)?;
                        o.dvd = Some(io);
                    }
                    "vcd" | "svcd" => {
                        let toc = toc.ok_or("Couldn't read the Video CD's list of tracks.")?;
                        vcd_info(&mut info, &dev, &toc, k);
                        o.toc = Some(toc);
                        o.sectors = Some(dev);
                    }
                    "bluray" => { info.kind = "bluray".into(); info.note = BLURAY_NOTE.into(); }
                    _ => {
                        info.kind = "data".into();
                        info.folder = disc::mount(path).ok();
                        if info.folder.is_none() { info.note = "Couldn't open the disc's files.".into(); }
                    }
                }
            }
        }
        if info.label.is_empty() { info.label = match info.kind.as_str() { "dvd" => "DVD", "vcd" => "Video CD", "svcd" => "Super Video CD", "audio-cd" => "Music CD", _ => "Disc" }.into(); }
        return fin(o, info);
    }

    if ext == "cue" {
        let text = std::fs::read_to_string(p).map_err(|e| format!("Couldn't open {path}: {e}"))?;
        let cue: Cue = disc::parse_cue(&text, p.parent().unwrap_or(Path::new(".")))?;
        let img: Arc<dyn Sectors> = Arc::new(Image::bin(&cue));
        info.reader = "image".into();
        info.label = cue.text.titles.first().filter(|s| !s.is_empty()).cloned().unwrap_or_else(|| p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default());
        if cue.toc.tracks.iter().any(|t| t.audio) {
            info.kind = "audio-cd".into();
            info.tracks = cd_tracks(&cue.toc, Some(&cue.text), true);
            info.album = cue.text.titles.first().cloned().unwrap_or_default();
            info.artist = cue.text.performers.first().cloned().unwrap_or_default();
            info.key = format!("cd:{}", disc::musicbrainz_id(&cue.toc));
        } else {
            let (k, l) = disc::iso_kind(img.as_ref(), cue.toc.tracks[0].lba);
            if !l.is_empty() { info.label = disc::pretty_label(&l); }
            match k {
                "vcd" | "svcd" => vcd_info(&mut info, &img, &cue.toc, k),
                _ => { info.kind = "data".into(); info.note = "This disc image holds files, not a film or music.".into(); }
            }
        }
        o.toc = Some(cue.toc.clone());
        o.sectors = Some(img);
        return fin(o, info);
    }

    let folder = p.is_dir();
    let fk = if folder { disc::folder_kind(p) } else { "" };
    if folder && (fk == "vcd" || fk == "svcd") {
        // A mounted Video CD: its MPEG sectors are read from the drive itself.
        if let Some(dev) = disc::device_of(p) { return open(&dev); }
        info.kind = "data".into();
        info.folder = Some(path.to_string());
        return fin(o, info);
    }
    if fk == "bluray" { info.kind = "bluray".into(); info.note = BLURAY_NOTE.into(); info.label = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(); return fin(o, info); }
    if ext == "iso" || fk == "dvd" {
        if folder { info.device = disc::device_of(p); }
        match dvd::open(p) {
            Ok(io) => {
                info.label = if folder { p.file_name().map(|n| n.to_string_lossy().into_owned()).filter(|n| !n.eq_ignore_ascii_case("VIDEO_TS")).unwrap_or_default() } else { label_from(&Image::iso(p)) };
                if info.label.is_empty() { info.label = p.file_stem().map(|n| disc::pretty_label(&n.to_string_lossy())).unwrap_or_else(|| "DVD".into()); }
                dvd_info(&mut info, &io)?;
                o.dvd = Some(io);
                return fin(o, info);
            }
            Err(e) if fk == "dvd" => return Err(format!("Couldn't read the DVD: {e}")),
            Err(_) => {
                let img = Image::iso(p);
                let (k, l) = disc::iso_kind(&img, 0);
                info.label = disc::pretty_label(&l);
                info.kind = if k == "bluray" { "bluray".into() } else { "data".into() };
                info.note = if k == "bluray" { BLURAY_NOTE.into() } else if k == "dvd" {
                    "This looks like a DVD, but it couldn't be read: libdvdread isn't installed.".into()
                } else { "This disc image holds files, not a film. Open it in Hearth Files to see them.".into() };
                return fin(o, info);
            }
        }
    }
    if folder {
        info.kind = "data".into();
        info.folder = Some(path.to_string());
        info.label = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        return fin(o, info);
    }
    Err(format!("{path} isn't a disc, a disc image or a disc's folder."))
}

impl Opened {
    pub fn titles(&self) -> &[Title] { &self.info.titles }
    pub fn toc(&self) -> Option<&Toc> { self.toc.as_ref() }
    /// Forgets the addresses made for this disc before (one thing plays at a time).
    pub fn forget(&self, server: &Server) {
        for u in self.urls.lock().unwrap().drain(..) { server.forget(&u); }
    }
}

/// How a stream should be made.
#[derive(Clone, Debug, Default)]
pub struct StreamOpts {
    pub input: String,
    pub audio: Option<u32>,
    pub sub: Option<u32>,
    pub palette: Vec<String>,
    pub size: (u32, u32),
    pub reencode: bool,
    pub encoders: Vec<String>,
}

/// ffmpeg's command line: an MPEG program stream in, Matroska out on stdout.
pub fn stream_args(o: &StreamOpts) -> Vec<String> {
    let mut a: Vec<String> = ["-hide_banner", "-loglevel", "error", "-nostdin", "-fflags", "+genpts+discardcorrupt", "-probesize", "16M", "-analyzeduration", "10M"].iter().map(|s| s.to_string()).collect();
    if o.sub.is_some() {
        if o.palette.len() == 16 { a.extend(["-palette".into(), o.palette.join(",")]); }
        if o.size.0 > 0 { a.extend(["-canvas_size".into(), format!("{}x{}", o.size.0, o.size.1)]); }
    }
    a.extend(["-f".into(), "mpeg".into(), "-i".into(), o.input.clone()]);
    let encode = o.reencode || o.sub.is_some();
    if let Some(s) = o.sub {
        a.extend(["-filter_complex".into(), format!("[0:v:0]yadif=deint=interlaced[v0];[v0][0:i:{s:#x}]overlay=eof_action=pass:format=auto[v]"), "-map".into(), "[v]".into()]);
    } else if o.reencode {
        a.extend(["-map".into(), "0:v:0".into(), "-vf".into(), "yadif=deint=interlaced".into()]);
    } else {
        a.extend(["-map".into(), "0:v:0".into()]);
    }
    a.extend(["-map".into(), match o.audio { Some(id) => format!("0:i:{id:#x}?"), None => "0:a:0?".into() }]);
    let has = |e: &str| o.encoders.iter().any(|x| x == e);
    if encode {
        if has("libx264") { a.extend(["-c:v", "libx264", "-preset", "veryfast", "-crf", "20", "-pix_fmt", "yuv420p"].map(String::from)); }
        else if has("libopenh264") { a.extend(["-c:v", "libopenh264", "-b:v", "6M", "-pix_fmt", "yuv420p"].map(String::from)); }
        else { a.extend(["-c:v", "mpeg2video", "-q:v", "3"].map(String::from)); }
    } else {
        a.extend(["-c:v", "copy"].map(String::from));
    }
    if has("libopus") { a.extend(["-c:a", "libopus", "-b:a", "192k", "-ac", "2"].map(String::from)); }
    else { a.extend(["-c:a", "aac", "-b:a", "192k", "-ac", "2"].map(String::from)); }
    a.extend(["-sn", "-dn", "-max_muxing_queue_size", "4096", "-avoid_negative_ts", "make_zero", "-f", "matroska", "-live", "1", "pipe:1"].map(String::from));
    a
}

#[derive(Serialize, Clone, Debug)]
pub struct Plan {
    pub url: String,
    /// The time the stream starts at (the page adds the video's own time to it).
    pub start: f64,
    pub duration: f64,
    /// Seeking starts a new stream (DVD, VCD); else the web view seeks by itself (audio CD).
    pub restart: bool,
}

/// Plays title (DVD) or track (CD, VCD) `n` from `at` seconds.
pub fn play(o: &Opened, server: &Server, ffmpeg: Option<&str>, encoders: &[String], n: u32, at: f64, audio: Option<u32>, sub: Option<u32>, reencode: bool) -> Result<Plan, String> {
    o.forget(server);
    let mut urls = Vec::new();
    let plan = match o.info.kind.as_str() {
        "dvd" => {
            let io = o.dvd.clone().ok_or("The DVD isn't open.")?;
            let t = o.info.titles.iter().find(|t| t.n == n).ok_or("That title isn't on the disc.")?;
            let ff = ffmpeg.ok_or("Playing DVDs needs ffmpeg, which isn't installed.")?;
            let (from, start) = dvd::seek_point(io.as_ref(), t, at);
            let vurl = server.virtual_url(Arc::new(dvd::TitleFile::new(io, t, from)), &format!("title-{n}.vob"));
            let args = stream_args(&StreamOpts { input: vurl.clone(), audio, sub, palette: t.palette.clone(), size: (t.width, t.height), reencode, encoders: encoders.to_vec() });
            let url = server.live_url(Live { program: ff.into(), args, mime: "video/x-matroska".into() }, &format!("Title {n}.mkv"));
            urls.extend([vurl, url.clone()]);
            Plan { url, start, duration: t.duration, restart: true }
        }
        "vcd" | "svcd" => {
            let src = o.sectors.clone().ok_or("The disc isn't open.")?;
            let t = o.info.tracks.iter().find(|t| t.n == n).ok_or("That part isn't on the disc.")?;
            let ff = ffmpeg.ok_or("Playing Video CDs needs ffmpeg, which isn't installed.")?;
            let frac = if t.duration > 0.0 { (at / t.duration).clamp(0.0, 0.995) } else { 0.0 };
            let from = (t.sectors as f64 * frac) as u32;
            let vurl = server.virtual_url(Arc::new(VcdTrack { src, start: t.start + from, sectors: t.sectors - from }), &format!("track-{n}.mpg"));
            let args = stream_args(&StreamOpts { input: vurl.clone(), audio: None, sub: None, reencode, encoders: encoders.to_vec(), ..Default::default() });
            let url = server.live_url(Live { program: ff.into(), args, mime: "video/x-matroska".into() }, &format!("Part {n}.mkv"));
            urls.extend([vurl, url.clone()]);
            Plan { url, start: t.duration * frac, duration: t.duration, restart: true }
        }
        "audio-cd" => {
            let src = o.sectors.clone().ok_or("The CD isn't open.")?;
            let t = o.info.tracks.iter().find(|t| t.n == n).ok_or("That track isn't on the CD.")?;
            let url = server.virtual_url(Arc::new(WavTrack { src, start: t.start, sectors: t.sectors }), &format!("Track {n}.wav"));
            urls.push(url.clone());
            Plan { url, start: 0.0, duration: t.duration, restart: false }
        }
        _ => return Err(if o.info.note.is_empty() { "There's nothing on this disc to play here.".into() } else { o.info.note.clone() }),
    };
    *o.urls.lock().unwrap() = urls;
    Ok(plan)
}

/// The drive a disc came from, or the first drive with a disc in it.
pub fn default_source() -> Option<String> {
    disc::drives().into_iter().find(|d| d.media).map(|d| d.device)
}

/// Where the disc's own folder is (for a data disc opened by its drive).
pub fn folder_of(o: &Opened) -> Option<PathBuf> { o.info.folder.as_ref().map(PathBuf::from) }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ffmpeg_lines() {
        let enc = vec!["libx264".to_string(), "libopus".into(), "aac".into()];
        let a = stream_args(&StreamOpts { input: "http://x/t.vob".into(), audio: Some(0x81), encoders: enc.clone(), ..Default::default() });
        let s = a.join(" ");
        assert!(s.contains("-f mpeg -i http://x/t.vob"), "{s}");
        assert!(s.contains("-map 0:v:0 -map 0:i:0x81?"), "{s}");
        assert!(s.contains("-c:v copy"), "{s}");
        assert!(s.contains("-c:a libopus -b:a 192k -ac 2"), "{s}");
        assert!(s.ends_with("-f matroska -live 1 pipe:1"), "{s}");
        assert!(!s.contains("-palette"));
        let pal: Vec<String> = (0..16).map(|_| "ffffff".to_string()).collect();
        let b = stream_args(&StreamOpts { input: "u".into(), audio: None, sub: Some(0x21), palette: pal, size: (720, 576), encoders: enc, ..Default::default() }).join(" ");
        assert!(b.contains("-palette ffffff,"), "{b}");
        assert!(b.contains("-canvas_size 720x576 -f mpeg -i u"), "{b}");
        assert!(b.contains("[v0][0:i:0x21]overlay"), "{b}");
        assert!(b.contains("-map [v] -map 0:a:0?"), "{b}");
        assert!(b.contains("-c:v libx264"), "{b}");
        let c = stream_args(&StreamOpts { input: "u".into(), reencode: true, encoders: vec!["aac".into()], ..Default::default() }).join(" ");
        assert!(c.contains("-vf yadif=deint=interlaced"), "{c}");
        assert!(c.contains("-c:v mpeg2video") && c.contains("-c:a aac"), "{c}");
    }

    #[test]
    fn opens_a_cue_audio_cd_and_plays_wav() {
        let d = std::env::temp_dir().join(format!("hp-dp-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("cd.bin"), vec![1u8; disc::RAW * 750]).unwrap();
        std::fs::write(d.join("cd.cue"), "TITLE \"Kiln\"\nPERFORMER \"Hearth Band\"\nFILE \"cd.bin\" BINARY\n TRACK 01 AUDIO\n  TITLE \"One\"\n  INDEX 01 00:00:00\n TRACK 02 AUDIO\n  TITLE \"Two\"\n  INDEX 01 00:05:00\n").unwrap();
        let o = open(&d.join("cd.cue").to_string_lossy()).unwrap();
        assert_eq!(o.info.kind, "audio-cd");
        assert_eq!(o.info.album, "Kiln");
        assert_eq!(o.info.tracks.iter().map(|t| (t.title.as_str(), t.artist.as_str(), t.duration.round() as u32)).collect::<Vec<_>>(), vec![("One", "Hearth Band", 5), ("Two", "Hearth Band", 5)]);
        let s = Server::start().unwrap();
        let p = play(&o, &s, None, &[], 2, 0.0, None, None, false).unwrap();
        assert!(!p.restart && p.url.ends_with("Track%202.wav"), "{}", p.url);
        assert_eq!(play(&o, &s, None, &[], 9, 0.0, None, None, false).unwrap_err(), "That track isn't on the CD.");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn blu_ray_and_data_folders() {
        let d = std::env::temp_dir().join(format!("hp-bd-{}", std::process::id()));
        std::fs::create_dir_all(d.join("BDMV")).unwrap();
        let o = open(&d.to_string_lossy()).unwrap();
        assert_eq!(o.info.kind, "bluray");
        assert_eq!(o.info.note, BLURAY_NOTE);
        std::fs::remove_dir_all(d.join("BDMV")).unwrap();
        let o = open(&d.to_string_lossy()).unwrap();
        assert_eq!(o.info.kind, "data");
        assert_eq!(o.info.folder.as_deref(), Some(d.to_string_lossy().as_ref()));
        let _ = std::fs::remove_dir_all(&d);
    }
}
