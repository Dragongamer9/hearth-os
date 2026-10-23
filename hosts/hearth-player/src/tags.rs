//! A song's tags (title, artist, album, track, year, genre) and cover picture, read and written with Lofty: it
//! changes only the tag part of the file, so the sound itself is never touched.

use lofty::config::WriteOptions;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::picture::{MimeType, Picture, PictureType};
use lofty::tag::items::Timestamp;
use lofty::tag::{Accessor, Tag};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct Tags {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub genre: String,
    pub track: Option<u32>,
    pub year: Option<u32>,
    /// Read only: seconds.
    pub duration: f64,
    /// Read only: there's a cover picture in the file.
    pub has_cover: bool,
}

pub fn read(path: &Path) -> Result<Tags, String> {
    let tf = lofty::read_from_path(path).map_err(|e| format!("Couldn't read the tags: {e}"))?;
    let mut t = Tags { duration: tf.properties().duration().as_secs_f64(), ..Default::default() };
    if let Some(tag) = tf.primary_tag().or_else(|| tf.first_tag()) {
        t.title = tag.title().map(|s| s.into_owned()).unwrap_or_default();
        t.artist = tag.artist().map(|s| s.into_owned()).unwrap_or_default();
        t.album = tag.album().map(|s| s.into_owned()).unwrap_or_default();
        t.genre = tag.genre().map(|s| s.into_owned()).unwrap_or_default();
        t.track = tag.track();
        t.year = tag.date().map(|d| d.year as u32);
    }
    t.has_cover = tf.tags().iter().any(|tag| !tag.pictures().is_empty());
    Ok(t)
}

/// The cover picture inside a file (the front cover first): its bytes and type.
pub fn cover(path: &Path) -> Option<(Vec<u8>, String)> {
    let tf = lofty::read_from_path(path).ok()?;
    for tag in tf.tags() {
        let pics = tag.pictures();
        let p = pics.iter().find(|p| p.pic_type() == PictureType::CoverFront).or_else(|| pics.first());
        if let Some(p) = p {
            let mime = p.mime_type().map(|m| m.as_str().to_string()).unwrap_or_else(|| "image/jpeg".into());
            return Some((p.data().to_vec(), mime));
        }
    }
    None
}

/// What to do with the cover when saving.
#[derive(Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct CoverChange {
    /// A picture file to use as the front cover.
    pub file: Option<String>,
    pub remove: bool,
}

fn picture_from_file(p: &Path) -> Result<Picture, String> {
    let data = std::fs::read(p).map_err(|e| format!("Couldn't read the picture: {e}"))?;
    let mime = if data.starts_with(&[0x89, b'P', b'N', b'G']) { MimeType::Png }
        else if data.starts_with(&[0xff, 0xd8]) { MimeType::Jpeg }
        else { return Err("The cover has to be a JPEG or PNG picture.".into()) };
    Ok(Picture::unchecked(data).pic_type(PictureType::CoverFront).mime_type(mime).build())
}

/// Writes tags (and the cover) into the file in place: only the tags change.
pub fn write(path: &Path, t: &Tags, cover: &CoverChange) -> Result<(), String> {
    let mut tf = lofty::read_from_path(path).map_err(|e| format!("Couldn't read the tags: {e}"))?;
    let tt = tf.primary_tag_type();
    if tf.primary_tag().is_none() { tf.insert_tag(Tag::new(tt)); }
    let tag = tf.primary_tag_mut().ok_or("This kind of file can't hold tags.")?;
    let set = |tag: &mut Tag, v: &str, f: fn(&mut Tag, String), r: fn(&mut Tag)| { if v.trim().is_empty() { r(tag) } else { f(tag, v.trim().to_string()) } };
    set(tag, &t.title, |g, v| g.set_title(v), |g| g.remove_title());
    set(tag, &t.artist, |g, v| g.set_artist(v), |g| g.remove_artist());
    set(tag, &t.album, |g, v| g.set_album(v), |g| g.remove_album());
    set(tag, &t.genre, |g, v| g.set_genre(v), |g| g.remove_genre());
    match t.track { Some(n) if n > 0 => tag.set_track(n), _ => tag.remove_track() }
    match t.year {
        Some(y) if (1..=9999).contains(&y) => tag.set_date(Timestamp { year: y as u16, ..Default::default() }),
        _ => tag.remove_date(),
    }
    if cover.remove { tag.remove_picture_type(PictureType::CoverFront); while !tag.pictures().is_empty() { tag.remove_picture(0); } }
    if let Some(f) = &cover.file {
        let pic = picture_from_file(Path::new(f))?;
        tag.remove_picture_type(PictureType::CoverFront);
        tag.push_picture(pic);
    }
    tf.save_to_path(path, WriteOptions::default()).map_err(|e| format!("Couldn't save the tags: {e}"))
}

/// After an edit made a new copy of a song: its tags and pictures, copied from the original.
pub fn copy(from: &Path, to: &Path) -> Result<(), String> {
    let src = lofty::read_from_path(from).map_err(|e| e.to_string())?;
    let Some(stag) = src.primary_tag().or_else(|| src.first_tag()) else { return Ok(()) };
    let mut dst = lofty::read_from_path(to).map_err(|e| e.to_string())?;
    let tt = dst.primary_tag_type();
    let mut tag = Tag::new(tt);
    if let Some(v) = stag.title() { tag.set_title(v.into_owned()); }
    if let Some(v) = stag.artist() { tag.set_artist(v.into_owned()); }
    if let Some(v) = stag.album() { tag.set_album(v.into_owned()); }
    if let Some(v) = stag.genre() { tag.set_genre(v.into_owned()); }
    if let Some(v) = stag.track() { tag.set_track(v); }
    if let Some(v) = stag.date() { tag.set_date(v); }
    for p in stag.pictures() { tag.push_picture(p.clone()); }
    dst.insert_tag(tag);
    dst.save_to_path(to, WriteOptions::default()).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tiny valid WAV file (silence) to write tags into.
    fn wav(path: &Path) {
        let samples = 8000u32;
        let mut b = Vec::new();
        b.extend(b"RIFF"); b.extend((36 + samples * 2).to_le_bytes()); b.extend(b"WAVEfmt ");
        b.extend(16u32.to_le_bytes()); b.extend(1u16.to_le_bytes()); b.extend(1u16.to_le_bytes());
        b.extend(8000u32.to_le_bytes()); b.extend(16000u32.to_le_bytes()); b.extend(2u16.to_le_bytes()); b.extend(16u16.to_le_bytes());
        b.extend(b"data"); b.extend((samples * 2).to_le_bytes()); b.extend(vec![0u8; (samples * 2) as usize]);
        std::fs::write(path, b).unwrap();
    }

    #[test]
    fn writes_and_reads_tags() {
        let p = std::env::temp_dir().join(format!("hp-tags-{}.wav", std::process::id()));
        wav(&p);
        let t = Tags { title: "Song".into(), artist: "Band".into(), album: "LP".into(), track: Some(3), year: Some(1999), ..Default::default() };
        write(&p, &t, &CoverChange::default()).unwrap();
        let r = read(&p).unwrap();
        assert_eq!((r.title.as_str(), r.artist.as_str(), r.album.as_str()), ("Song", "Band", "LP"));
        assert!((r.duration - 1.0).abs() < 0.05, "{}", r.duration);
        let q = std::env::temp_dir().join(format!("hp-tags-{}-copy.wav", std::process::id()));
        wav(&q);
        copy(&p, &q).unwrap();
        assert_eq!(read(&q).unwrap().title, "Song");
        let _ = std::fs::remove_file(&p);
        let _ = std::fs::remove_file(&q);
    }
}
