//! The fixture discs made in CI (.github/scripts/player-disc-fixtures.sh; HEARTH_DISC_FIXTURES is their folder): a
//! DVD-Video ISO read with libdvdread and as a VIDEO_TS folder, the same DVD "locked", a Video CD and a music CD
//! (BIN/CUE with CD-Text), a Blu-ray folder and a data folder. Their titles, chapters, tracks and names, the title as
//! sectors from the local server, and ffmpeg turning a DVD title and the VCD into Matroska the web view plays.
//! Without the fixtures (a local run) there's nothing to test.
#![cfg(target_os = "linux")]

use hearth_player::discplay::{self, Opened};
use hearth_player::server::Server;
use std::io::{Read, Write};
use std::path::PathBuf;

fn fixtures() -> Option<PathBuf> { std::env::var_os("HEARTH_DISC_FIXTURES").map(PathBuf::from) }

fn open(p: PathBuf) -> Opened { discplay::open(&p.to_string_lossy()).unwrap_or_else(|e| panic!("{}: {e}", p.display())) }

/// The first `n` bytes at a local address.
fn first_bytes(url: &str, n: usize) -> Vec<u8> {
    let rest = url.strip_prefix("http://").unwrap();
    let (host, path) = rest.split_once('/').unwrap();
    let mut s = std::net::TcpStream::connect(host).unwrap();
    s.set_read_timeout(Some(std::time::Duration::from_secs(60))).unwrap();
    write!(s, "GET /{path} HTTP/1.1\r\nHost: {host}\r\n\r\n").unwrap();
    let mut all = Vec::new();
    let mut buf = [0u8; 65536];
    loop {
        match s.read(&mut buf) { Ok(0) | Err(_) => break, Ok(k) => all.extend_from_slice(&buf[..k]) }
        if let Some(h) = all.windows(4).position(|w| w == b"\r\n\r\n") { if all.len() >= h + 4 + n { return all[h + 4..h + 4 + n].to_vec(); } }
    }
    let h = all.windows(4).position(|w| w == b"\r\n\r\n").map(|h| h + 4).unwrap_or(all.len());
    all[h..].to_vec()
}

#[test]
fn fixture_discs() {
    let Some(dir) = fixtures() else { eprintln!("no fixture discs (HEARTH_DISC_FIXTURES): nothing to test"); return };
    let tools = hearth_player::tools::detect();
    let server = Server::start().unwrap();
    let mut sub_err: Option<String> = None;

    // DVD-Video.
    let d = open(dir.join("hearth-test.iso"));
    let i = &d.info;
    println!("DVD: {} titles, reader {}, label {:?}", i.titles.len(), i.reader, i.label);
    assert_eq!(i.kind, "dvd");
    assert_eq!(i.reader, "libdvdread", "the ISO is read with libdvdread");
    assert_eq!(i.label, "Hearth Test");
    assert_eq!(i.titles.len(), 2);
    assert_eq!(i.main, Some(1));
    assert!(!i.encrypted);
    let t1 = &i.titles[0];
    assert!((t1.duration - 24.0).abs() < 1.5, "title 1 lasts {}", t1.duration);
    assert_eq!(t1.chapters.len(), 4, "{:?}", t1.chapters);
    assert!((t1.chapters[1].start - 6.0).abs() < 1.0, "{:?}", t1.chapters);
    assert_eq!(t1.audio.iter().map(|a| a.lang.as_str()).collect::<Vec<_>>(), vec!["en", "fr"]);
    assert_eq!(t1.audio[0].id, 0x80);
    println!("DVD title 1: sound {:?}, subtitles {:?}", t1.audio.iter().map(|a| &a.label).collect::<Vec<_>>(), t1.subs.iter().map(|s| &s.label).collect::<Vec<_>>());
    assert_eq!(i.titles[1].chapters.len(), 2);
    assert!(i.key.starts_with("dvd:"));

    // The title's sectors from the local server: an MPEG program stream.
    let p = discplay::play(&d, &server, tools.ffmpeg.as_deref(), &tools.encoders, 1, 0.0, Some(0x81), None, false).unwrap();
    assert!(p.restart && (p.duration - t1.duration).abs() < 0.01);
    // ffmpeg's Matroska: starts with the EBML header.
    if tools.ffmpeg.is_some() {
        let mkv = first_bytes(&p.url, 4);
        assert_eq!(mkv, vec![0x1A, 0x45, 0xDF, 0xA3], "the DVD title came out as Matroska");
        // From 13 s: the stream starts at a VOBU near there and says when.
        let p2 = discplay::play(&d, &server, tools.ffmpeg.as_deref(), &tools.encoders, 1, 13.0, None, None, false).unwrap();
        assert!(p2.start > 10.0 && p2.start <= 13.5, "starts at {}", p2.start);
        assert_eq!(first_bytes(&p2.url, 4), vec![0x1A, 0x45, 0xDF, 0xA3], "from the middle too");
        // Subtitles drawn into the picture (when the fixture has them: spumux made them).
        if dir.join("subs.made").exists() {
            assert_eq!(t1.subs.len(), 1, "{:?}", t1.subs);
            let p3 = discplay::play(&d, &server, tools.ffmpeg.as_deref(), &tools.encoders, 1, 2.0, None, Some(t1.subs[0].id), false).unwrap();
            let b = first_bytes(&p3.url, 4);
            std::thread::sleep(std::time::Duration::from_millis(500));
            if b != [0x1A, 0x45, 0xDF, 0xA3] { sub_err = Some(hearth_player::server::last_stream_error()); }
        } else { println!("the fixture has no subtitles (spumux couldn't make them)"); }
    }

    // Locked.
    let l = open(dir.join("hearth-locked.iso"));
    assert_eq!(l.info.kind, "dvd");
    assert!(l.info.encrypted, "the locked DVD is seen as locked");
    assert!(!l.info.dvdcss, "no libdvdcss on the test machine");
    assert_eq!(l.info.note, discplay::ENCRYPTED_NOTE);

    // Video CD.
    let v = open(dir.join("vcd.cue"));
    println!("VCD: {:?} parts {:?}", v.info.kind, v.info.tracks.iter().map(|t| t.duration).collect::<Vec<_>>());
    assert_eq!(v.info.kind, "vcd");
    assert_eq!(v.info.tracks.len(), 1);
    assert!((v.info.tracks[0].duration - 20.0).abs() < 2.0, "{}", v.info.tracks[0].duration);
    if tools.ffmpeg.is_some() {
        let p = discplay::play(&v, &server, tools.ffmpeg.as_deref(), &tools.encoders, v.info.tracks[0].n, 0.0, None, None, false).unwrap();
        assert_eq!(first_bytes(&p.url, 4), vec![0x1A, 0x45, 0xDF, 0xA3], "the VCD came out as Matroska");
    }

    // Music CD with CD-Text.
    let c = open(dir.join("cd.cue"));
    assert_eq!(c.info.kind, "audio-cd");
    assert_eq!((c.info.album.as_str(), c.info.artist.as_str()), ("Kiln", "Hearth Band"));
    assert_eq!(c.info.tracks.iter().map(|t| t.title.as_str()).collect::<Vec<_>>(), vec!["Ember song 1", "Kindling", "The last log"]);
    assert_eq!(c.info.tracks[2].artist, "Guest Player");
    assert_eq!(c.info.tracks.iter().map(|t| t.duration.round() as u32).collect::<Vec<_>>(), vec![10, 12, 14]);
    let p = discplay::play(&c, &server, None, &[], 2, 0.0, None, None, false).unwrap();
    assert!(!p.restart);
    let wav = first_bytes(&p.url, 12);
    assert_eq!(&wav[..4], b"RIFF");
    assert_eq!(&wav[8..12], b"WAVE");

    // Blu-ray, a data folder.
    let b = open(dir.join("bd"));
    assert_eq!(b.info.kind, "bluray");
    assert_eq!(b.info.note, discplay::BLURAY_NOTE);
    let f = open(dir.join("files"));
    assert_eq!(f.info.kind, "data");
    assert!(f.info.folder.is_some());

    // The DVD as a VIDEO_TS folder, without libdvdread.
    std::env::set_var("HEARTH_PLAYER_NO_DVDREAD", "1");
    let ff = open(dir.join("dvd"));
    std::env::remove_var("HEARTH_PLAYER_NO_DVDREAD");
    assert_eq!(ff.info.reader, "folder");
    assert_eq!(ff.info.titles.len(), 2);
    assert_eq!(ff.info.titles[0].chapters.len(), 4);
    assert_eq!(ff.info.key, d.info.key, "the same disc either way");
    assert!(sub_err.is_none(), "a DVD title with subtitles drawn in didn't come out; ffmpeg said: {}", sub_err.unwrap_or_default());
    println!("all fixture discs read");
}
