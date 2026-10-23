//! Hearth Player: Hearth OS's own video and music player (and Podcasts & Radio), with simple editing built in.
//! The window is ui/index.html, drawn by the system's web view (WebKitGTK on Linux, so playback is GStreamer's,
//! with the codecs the image ships: build.sh writes them to /usr/share/nimbus/media-codecs.txt). This library is everything that doesn't need a
//! window, so the tests can run it: opening folders and playlists (media.rs), the local file server the page plays
//! from (server.rs), ffmpeg for tracks, chapters, subtitles inside files and edits (tools.rs), song tags (tags.rs),
//! podcasts and radio downloads (net.rs), what it remembers (store.rs), keeping the screen awake (awake.rs) and
//! MPRIS for media keys (mpris.rs, Linux).

pub mod awake;
pub mod media;
pub mod net;
pub mod server;
pub mod store;
pub mod tags;
pub mod tools;
#[cfg(target_os = "linux")]
pub mod mpris;

use serde::Serialize;
use std::path::{Path, PathBuf};

/// A place to start browsing from in the Open screen.
#[derive(Serialize, Clone, Debug)]
pub struct Place { pub name: String, pub path: String, pub kind: String }

pub fn places() -> Vec<Place> {
    let mut v = Vec::new();
    let mut add = |name: &str, p: Option<PathBuf>, kind: &str| {
        if let Some(p) = p { if p.is_dir() && !v.iter().any(|x: &Place| x.path == p.to_string_lossy()) { v.push(Place { name: name.into(), path: p.to_string_lossy().into_owned(), kind: kind.into() }); } }
    };
    add("Videos", dirs::video_dir(), "videos");
    add("Music", dirs::audio_dir(), "music");
    add("Downloads", dirs::download_dir(), "downloads");
    add("Home", dirs::home_dir(), "home");
    add("Podcasts", Some(podcast_dir()), "podcasts");
    // USB drives and SD cards.
    #[cfg(target_os = "linux")]
    {
        let user = std::env::var("USER").unwrap_or_default();
        for base in [format!("/run/media/{user}"), format!("/media/{user}"), "/var/mnt".into()] {
            if let Ok(rd) = std::fs::read_dir(&base) {
                for e in rd.flatten() {
                    let p = e.path();
                    if p.is_dir() { add(&e.file_name().to_string_lossy(), Some(p), "drive"); }
                }
            }
        }
    }
    #[cfg(windows)]
    for l in b'D'..=b'Z' {
        let p = PathBuf::from(format!("{}:\\", l as char));
        if p.exists() { add(&format!("Drive {}", l as char), Some(p), "drive"); }
    }
    v
}

/// Where downloaded podcast episodes go: Music/Podcasts.
pub fn podcast_dir() -> PathBuf {
    dirs::audio_dir().or_else(|| dirs::home_dir().map(|h| h.join("Music"))).unwrap_or_else(std::env::temp_dir).join("Podcasts")
}

#[derive(Serialize, Clone, Debug)]
pub struct Entry { pub name: String, pub path: String, pub dir: bool, pub kind: String, pub size: u64 }

/// A folder's subfolders and files of one sort (the Open screen): `filter` "media" (videos, songs, playlists),
/// "images" (a cover picture) or "subs" (a subtitle file). Folders first, in name order.
pub fn list_dir(dir: &Path, hidden: bool, filter: &str) -> Result<Vec<Entry>, String> {
    let rd = std::fs::read_dir(dir).map_err(|e| format!("Couldn't open that folder: {e}"))?;
    let mut v: Vec<Entry> = rd.flatten().filter_map(|e| {
        let name = e.file_name().to_string_lossy().into_owned();
        if !hidden && name.starts_with('.') { return None; }
        let p = e.path();
        let md = std::fs::metadata(&p).ok()?;
        if md.is_dir() { return Some(Entry { name, path: p.to_string_lossy().into_owned(), dir: true, kind: "folder".into(), size: 0 }); }
        let k = match filter {
            "images" => matches!(p.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).as_deref(), Some("jpg" | "jpeg" | "png")).then_some("image")?,
            "subs" => media::kind(&p).filter(|k| *k == "subtitle")?,
            _ => media::kind(&p).filter(|k| *k != "subtitle")?,
        };
        Some(Entry { name, path: p.to_string_lossy().into_owned(), dir: false, kind: k.into(), size: md.len() })
    }).collect();
    v.sort_by(|a, b| b.dir.cmp(&a.dir).then_with(|| media::natural_cmp(&a.name, &b.name)));
    Ok(v)
}

/// Where a screenshot of a video goes: beside the video when that folder can be written to, else
/// Pictures/Screenshots. "<video name> 00-01-23.png", never over an existing file.
pub fn screenshot_path(video: &Path, at: f64) -> PathBuf {
    let stem = video.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "Screenshot".into());
    let t = at.max(0.0) as u64;
    let base = format!("{stem} {:02}-{:02}-{:02}", t / 3600, t / 60 % 60, t % 60);
    let beside = video.parent().map(Path::to_path_buf).filter(|d| {
        let probe = d.join(format!(".hearth-player-{}", std::process::id()));
        let ok = std::fs::write(&probe, b"").is_ok();
        let _ = std::fs::remove_file(&probe);
        ok
    });
    let dir = beside.unwrap_or_else(|| dirs::picture_dir().or_else(|| dirs::home_dir().map(|h| h.join("Pictures"))).unwrap_or_else(std::env::temp_dir).join("Screenshots"));
    let _ = std::fs::create_dir_all(&dir);
    for n in 1.. {
        let p = dir.join(if n == 1 { format!("{base}.png") } else { format!("{base} ({n}).png") });
        if !p.exists() { return p; }
    }
    unreachable!()
}

/// The cache folder (copies made so the web view can play a file, subtitles taken out of files).
pub fn cache_dir() -> PathBuf {
    dirs::cache_dir().unwrap_or_else(std::env::temp_dir).join("hearth-player")
}

/// A short stable name for a file (its path, size and time), for cache file names.
pub fn file_key(p: &Path, extra: &str) -> String {
    use sha2::{Digest, Sha256};
    let md = std::fs::metadata(p).ok();
    let mut h = Sha256::new();
    h.update(p.to_string_lossy().as_bytes());
    h.update(md.as_ref().map(|m| m.len()).unwrap_or(0).to_le_bytes());
    h.update(format!("{:?}", md.and_then(|m| m.modified().ok())).as_bytes());
    h.update(extra.as_bytes());
    h.finalize().iter().take(12).map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn screenshot_names() {
        let d = std::env::temp_dir().join(format!("hp-shot-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let p = super::screenshot_path(&d.join("Film.mkv"), 3725.4);
        assert_eq!(p, d.join("Film 01-02-05.png"));
        std::fs::write(&p, b"x").unwrap();
        assert_eq!(super::screenshot_path(&d.join("Film.mkv"), 3725.0), d.join("Film 01-02-05 (2).png"));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn lists_folders_first() {
        let d = std::env::temp_dir().join(format!("hp-list-{}", std::process::id()));
        std::fs::create_dir_all(d.join("Season 1")).unwrap();
        for f in ["b.mkv", "a.mp3", "x.txt", "a.srt", ".hidden.mp4"] { std::fs::write(d.join(f), b"x").unwrap(); }
        let l = super::list_dir(&d, false, "media").unwrap();
        assert_eq!(super::list_dir(&d, false, "subs").unwrap().iter().map(|e| e.name.as_str()).collect::<Vec<_>>(), vec!["Season 1", "a.srt"]);
        assert_eq!(l.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(), vec!["Season 1", "a.mp3", "b.mkv"]);
        assert_ne!(super::file_key(&d.join("a.mp3"), ""), super::file_key(&d.join("b.mkv"), ""));
        let _ = std::fs::remove_dir_all(&d);
    }
}
