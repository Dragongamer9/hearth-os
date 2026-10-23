//! What counts as a video, a song, a playlist or a subtitle file; opening folders and playlists as a play queue;
//! finding the subtitles and cover picture next to a file; and turning .srt and .ass subtitles into WebVTT (what
//! the web view's <track> reads).

use serde::Serialize;
use std::cmp::Ordering;
use std::path::{Path, PathBuf};

pub const VIDEO_EXT: &[&str] = &[
    "mp4", "m4v", "mkv", "webm", "mov", "avi", "wmv", "flv", "mpg", "mpeg", "ts", "m2ts", "mts", "3gp", "ogv", "vob", "divx",
];
pub const AUDIO_EXT: &[&str] = &[
    "mp3", "flac", "ogg", "oga", "opus", "m4a", "m4b", "aac", "wav", "wma", "aiff", "aif", "ape", "wv", "mka", "alac",
];
pub const PLAYLIST_EXT: &[&str] = &["m3u", "m3u8", "pls"];
pub const SUB_EXT: &[&str] = &["srt", "ass", "ssa", "vtt"];
pub const COVER_NAMES: &[&str] = &["cover", "folder", "front", "album", "albumart", "albumartsmall"];

/// One thing to play.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Item {
    pub path: String,
    pub name: String,
    /// "video" or "audio" (a link from a playlist that can't be told apart counts as audio).
    pub kind: String,
}

/// A subtitle file next to a video.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Sub {
    pub path: String,
    pub label: String,
    pub lang: String,
}

fn ext(p: &Path) -> String {
    p.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default()
}

/// "video", "audio", "playlist", "subtitle" or None.
pub fn kind(p: &Path) -> Option<&'static str> {
    let e = ext(p);
    let e = e.as_str();
    if VIDEO_EXT.contains(&e) { Some("video") }
    else if AUDIO_EXT.contains(&e) { Some("audio") }
    else if PLAYLIST_EXT.contains(&e) { Some("playlist") }
    else if SUB_EXT.contains(&e) { Some("subtitle") }
    else { None }
}

pub fn is_media(p: &Path) -> bool { matches!(kind(p), Some("video") | Some("audio")) }

fn item(p: &Path) -> Item {
    let k = kind(p).filter(|k| *k == "video").unwrap_or("audio");
    Item { path: p.to_string_lossy().into_owned(), name: display_name(p), kind: k.into() }
}

fn display_name(p: &Path) -> String {
    let s = p.to_string_lossy();
    if s.starts_with("http://") || s.starts_with("https://") {
        return s.rsplit('/').find(|x| !x.is_empty()).unwrap_or(&s).to_string();
    }
    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| s.into_owned())
}

/// "Track 2" before "Track 10": numbers inside names compare as numbers; otherwise case doesn't matter.
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let (mut x, mut y) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (x.peek().copied(), y.peek().copied()) {
            (None, None) => return a.cmp(b),
            (None, _) => return Ordering::Less,
            (_, None) => return Ordering::Greater,
            (Some(c), Some(d)) if c.is_ascii_digit() && d.is_ascii_digit() => {
                let mut n1 = String::new();
                while let Some(c) = x.peek().copied().filter(|c| c.is_ascii_digit()) { n1.push(c); x.next(); }
                let mut n2 = String::new();
                while let Some(d) = y.peek().copied().filter(|c| c.is_ascii_digit()) { n2.push(d); y.next(); }
                let (t1, t2) = (n1.trim_start_matches('0'), n2.trim_start_matches('0'));
                let o = t1.len().cmp(&t2.len()).then_with(|| t1.cmp(t2));
                if o != Ordering::Equal { return o; }
            }
            (Some(c), Some(d)) => {
                let o = c.to_lowercase().cmp(d.to_lowercase());
                if o != Ordering::Equal { return o; }
                x.next();
                y.next();
            }
        }
    }
}

/// The media files in a folder (and its folders, a few levels down), in name order.
pub fn folder_media(dir: &Path, depth: u32) -> Vec<Item> {
    let mut files = Vec::new();
    let mut dirs = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') { continue; }
            if p.is_dir() { dirs.push(p); } else if is_media(&p) { files.push(p); }
        }
    }
    files.sort_by(|a, b| natural_cmp(&a.to_string_lossy(), &b.to_string_lossy()));
    dirs.sort_by(|a, b| natural_cmp(&a.to_string_lossy(), &b.to_string_lossy()));
    let mut out: Vec<Item> = files.iter().map(|p| item(p)).collect();
    if depth > 0 {
        for d in dirs { out.extend(folder_media(&d, depth - 1)); }
    }
    out
}

/// The songs in the same folder as `file` (the music view's list).
pub fn folder_audio(file: &Path) -> Vec<Item> {
    let dir = if file.is_dir() { file.to_path_buf() } else { file.parent().map(Path::to_path_buf).unwrap_or_default() };
    folder_media(&dir, 0).into_iter().filter(|i| i.kind == "audio").collect()
}

fn resolve(entry: &str, base: &Path) -> Option<String> {
    let e = entry.trim();
    if e.is_empty() { return None; }
    if e.starts_with("http://") || e.starts_with("https://") { return Some(e.to_string()); }
    let e = e.strip_prefix("file://").map(|s| percent_decode(s)).unwrap_or_else(|| e.to_string());
    #[cfg(windows)]
    let e = e.trim_start_matches('/').to_string().replace('/', "\\");
    let p = Path::new(&e);
    let full = if p.is_absolute() { p.to_path_buf() } else { base.join(p) };
    Some(full.to_string_lossy().into_owned())
}

/// Decodes %20 and friends (file:// links in playlists).
pub fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() && s.is_char_boundary(i + 1) && s.is_char_boundary(i + 3) {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) { out.push(v); i += 3; continue; }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// The entries of an .m3u/.m3u8 playlist (comments and #EXTINF lines skipped), relative ones from `base`.
pub fn parse_m3u(text: &str, base: &Path) -> Vec<String> {
    text.lines().map(|l| l.trim_start_matches('\u{feff}').trim()).filter(|l| !l.is_empty() && !l.starts_with('#')).filter_map(|l| resolve(l, base)).collect()
}

/// The entries of a .pls playlist (File1=..., File2=...), in their numbered order.
pub fn parse_pls(text: &str, base: &Path) -> Vec<String> {
    let mut v: Vec<(u32, String)> = text.lines().filter_map(|l| {
        let l = l.trim();
        let rest = l.strip_prefix("File").or_else(|| l.strip_prefix("file"))?;
        let (n, val) = rest.split_once('=')?;
        Some((n.trim().parse().ok()?, resolve(val, base)?))
    }).collect();
    v.sort_by_key(|x| x.0);
    v.into_iter().map(|x| x.1).collect()
}

/// What was asked to be opened (files, folders, playlists), as one play queue.
pub fn expand(paths: &[String]) -> Vec<Item> {
    let mut out = Vec::new();
    for s in paths {
        if s.starts_with("http://") || s.starts_with("https://") {
            out.push(item(Path::new(s)));
            continue;
        }
        let s = s.strip_prefix("file://").map(percent_decode).unwrap_or_else(|| s.clone());
        let p = PathBuf::from(&s);
        if p.is_dir() { out.extend(folder_media(&p, 3)); continue; }
        match kind(&p) {
            Some("playlist") => {
                let text = std::fs::read(&p).map(|b| String::from_utf8_lossy(&b).into_owned()).unwrap_or_default();
                let base = p.parent().map(Path::to_path_buf).unwrap_or_default();
                let list = if ext(&p) == "pls" { parse_pls(&text, &base) } else { parse_m3u(&text, &base) };
                out.extend(list.iter().map(|e| item(Path::new(e))));
            }
            Some("video") | Some("audio") => out.push(item(&p)),
            // Anything else that exists: let the web view try it.
            _ if p.is_file() => out.push(item(&p)),
            _ => {}
        }
    }
    out
}

/// Writes the queue as an .m3u8 playlist.
pub fn write_m3u(items: &[String]) -> String {
    let mut s = String::from("#EXTM3U\n");
    for i in items { s.push_str(i); s.push('\n'); }
    s
}

const LANGS: &[(&str, &str)] = &[
    ("en", "English"), ("eng", "English"), ("english", "English"), ("es", "Spanish"), ("spa", "Spanish"), ("spanish", "Spanish"),
    ("fr", "French"), ("fre", "French"), ("fra", "French"), ("french", "French"), ("de", "German"), ("ger", "German"), ("deu", "German"),
    ("german", "German"), ("it", "Italian"), ("ita", "Italian"), ("pt", "Portuguese"), ("por", "Portuguese"), ("pt-br", "Portuguese (Brazil)"),
    ("nl", "Dutch"), ("dut", "Dutch"), ("nld", "Dutch"), ("ja", "Japanese"), ("jpn", "Japanese"), ("ko", "Korean"), ("kor", "Korean"),
    ("zh", "Chinese"), ("chi", "Chinese"), ("zho", "Chinese"), ("ru", "Russian"), ("rus", "Russian"), ("pl", "Polish"), ("pol", "Polish"),
    ("sv", "Swedish"), ("swe", "Swedish"), ("ar", "Arabic"), ("ara", "Arabic"), ("tr", "Turkish"), ("tur", "Turkish"),
];

/// A readable name for a language code ("en", "eng"), or the code itself.
pub fn lang_name(code: &str) -> String {
    let c = code.to_ascii_lowercase();
    LANGS.iter().find(|(k, _)| *k == c).map(|(_, v)| v.to_string()).unwrap_or_else(|| code.to_string())
}

/// Subtitle files that belong to `video`: "Movie.srt", "Movie.en.srt", "Movie.English.forced.ass", and the same
/// in a "Subs" or "Subtitles" folder beside it.
pub fn sidecar_subs(video: &Path) -> Vec<Sub> {
    let Some(stem) = video.file_stem().map(|s| s.to_string_lossy().to_lowercase()) else { return vec![] };
    let dir = video.parent().map(Path::to_path_buf).unwrap_or_default();
    let mut dirs = vec![dir.clone()];
    for d in ["Subs", "subs", "Subtitles", "subtitles"] { if dir.join(d).is_dir() { dirs.push(dir.join(d)); } }
    let mut out = Vec::new();
    for d in dirs {
        let in_sub_dir = d != dir;
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        let mut found: Vec<PathBuf> = rd.flatten().map(|e| e.path()).filter(|p| kind(p) == Some("subtitle")).collect();
        found.sort_by(|a, b| natural_cmp(&a.to_string_lossy(), &b.to_string_lossy()));
        for p in found {
            let name = p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            let lower = name.to_lowercase();
            let rest = if lower == stem { Some(String::new()) }
                else if let Some(r) = lower.strip_prefix(&format!("{stem}.")) { Some(r.to_string()) }
                else if in_sub_dir { Some(lower.clone()) }
                else { None };
            let Some(rest) = rest else { continue };
            let parts: Vec<&str> = rest.split(['.', '_', ' ']).filter(|s| !s.is_empty()).collect();
            let lang = parts.iter().find(|s| LANGS.iter().any(|(k, _)| k == *s)).map(|s| s.to_string()).unwrap_or_default();
            let mut label = if lang.is_empty() { String::new() } else { lang_name(&lang) };
            if parts.iter().any(|s| *s == "forced") { label = format!("{} (forced)", if label.is_empty() { "Subtitles" } else { label.as_str() }).trim().to_string(); }
            if parts.iter().any(|s| *s == "sdh" || *s == "cc") { label = format!("{} (SDH)", if label.is_empty() { "Subtitles" } else { label.as_str() }); }
            if label.is_empty() { label = if rest.is_empty() { "Subtitles".into() } else { rest.clone() }; }
            let e = ext(&p).to_uppercase();
            out.push(Sub { path: p.to_string_lossy().into_owned(), label: format!("{label} ({e} file)"), lang });
        }
    }
    out
}

/// folder.jpg, cover.png, ... next to a song.
pub fn cover_file(dir: &Path) -> Option<PathBuf> {
    let rd = std::fs::read_dir(dir).ok()?;
    let mut imgs: Vec<PathBuf> = rd.flatten().map(|e| e.path())
        .filter(|p| matches!(ext(p).as_str(), "jpg" | "jpeg" | "png" | "webp")).collect();
    imgs.sort();
    for n in COVER_NAMES {
        if let Some(p) = imgs.iter().find(|p| p.file_stem().map(|s| s.to_string_lossy().to_lowercase() == *n).unwrap_or(false)) {
            return Some(p.clone());
        }
    }
    None
}

fn vtt_time(t: &str) -> Option<String> {
    // SRT: 00:01:02,345   ASS: 0:01:02.34
    let t = t.trim().replace(',', ".");
    let (hms, frac) = t.split_once('.').unwrap_or((t.as_str(), "0"));
    let parts: Vec<&str> = hms.split(':').collect();
    let (h, m, s) = match parts.as_slice() {
        [h, m, s] => (h.parse::<u32>().ok()?, m.parse::<u32>().ok()?, s.parse::<u32>().ok()?),
        [m, s] => (0, m.parse::<u32>().ok()?, s.parse::<u32>().ok()?),
        _ => return None,
    };
    let mut ms: String = frac.chars().filter(|c| c.is_ascii_digit()).take(3).collect();
    while ms.len() < 3 { ms.push('0'); }
    Some(format!("{h:02}:{m:02}:{s:02}.{ms}"))
}

fn escape_cue(s: &str) -> String {
    // Keep the simple tags WebVTT knows (<i>, <b>, <u>); everything else is text.
    let mut out = s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
    for t in ["i", "b", "u"] {
        out = out.replace(&format!("&lt;{t}&gt;"), &format!("<{t}>")).replace(&format!("&lt;/{t}&gt;"), &format!("</{t}>"));
        out = out.replace(&format!("&lt;{}&gt;", t.to_uppercase()), &format!("<{t}>")).replace(&format!("&lt;/{}&gt;", t.to_uppercase()), &format!("</{t}>"));
    }
    // <font color=...> and friends are dropped.
    let mut clean = String::new();
    let mut rest = out.as_str();
    while let Some(i) = rest.find("&lt;font") {
        clean.push_str(&rest[..i]);
        match rest[i..].find("&gt;") { Some(j) => rest = &rest[i + j + 4..], None => { rest = ""; } }
    }
    clean.push_str(rest);
    clean.replace("&lt;/font&gt;", "")
}

/// SubRip (.srt) to WebVTT.
pub fn srt_to_vtt(text: &str) -> String {
    let text = text.trim_start_matches('\u{feff}').replace("\r\n", "\n").replace('\r', "\n");
    let mut out = String::from("WEBVTT\n\n");
    for block in text.split("\n\n") {
        let lines: Vec<&str> = block.lines().filter(|l| !l.trim().is_empty()).collect();
        let Some(ti) = lines.iter().position(|l| l.contains("-->")) else { continue };
        let (a, b) = lines[ti].split_once("-->").unwrap();
        let b = b.split_whitespace().next().unwrap_or("");
        let (Some(a), Some(b)) = (vtt_time(a), vtt_time(b)) else { continue };
        out.push_str(&format!("{a} --> {b}\n"));
        for l in &lines[ti + 1..] { out.push_str(&escape_cue(l)); out.push('\n'); }
        out.push('\n');
    }
    out
}

/// Advanced SubStation (.ass/.ssa) to WebVTT: the text and timing of each Dialogue line, with italics and bold
/// kept and the rest of the styling (positions, karaoke, drawings) left out.
pub fn ass_to_vtt(text: &str) -> String {
    let text = text.trim_start_matches('\u{feff}').replace("\r\n", "\n");
    let mut format: Vec<String> = vec![];
    let mut in_events = false;
    let mut cues: Vec<(String, String, String)> = vec![];
    for line in text.lines() {
        let l = line.trim();
        if l.starts_with('[') { in_events = l.eq_ignore_ascii_case("[events]"); continue; }
        if !in_events { continue; }
        if let Some(f) = l.strip_prefix("Format:") { format = f.split(',').map(|s| s.trim().to_ascii_lowercase()).collect(); continue; }
        let Some(d) = l.strip_prefix("Dialogue:") else { continue };
        if format.is_empty() { format = "layer,start,end,style,name,marginl,marginr,marginv,effect,text".split(',').map(String::from).collect(); }
        let fields: Vec<&str> = d.splitn(format.len(), ',').collect();
        if fields.len() < format.len() { continue; }
        let get = |n: &str| format.iter().position(|f| f == n).map(|i| fields[i].trim());
        let (Some(s), Some(e), Some(t)) = (get("start").and_then(vtt_time), get("end").and_then(vtt_time), format.iter().position(|f| f == "text").map(|i| fields[i])) else { continue };
        // Drawings ({\p1}) aren't text.
        if t.contains("\\p1") { continue; }
        let mut body = String::new();
        let mut italic = false;
        let mut bold = false;
        let mut rest = t;
        while let Some(i) = rest.find('{') {
            body.push_str(&rest[..i]);
            let Some(j) = rest[i..].find('}') else { rest = ""; break };
            let tag = &rest[i + 1..i + j];
            for part in tag.split('\\') {
                if part == "i1" && !italic { body.push_str("\u{1}i\u{2}"); italic = true; }
                if part == "i0" && italic { body.push_str("\u{1}/i\u{2}"); italic = false; }
                if part == "b1" && !bold { body.push_str("\u{1}b\u{2}"); bold = true; }
                if part == "b0" && bold { body.push_str("\u{1}/b\u{2}"); bold = false; }
            }
            rest = &rest[i + j + 1..];
        }
        body.push_str(rest);
        if italic { body.push_str("\u{1}/i\u{2}"); }
        if bold { body.push_str("\u{1}/b\u{2}"); }
        let body = body.replace("\\N", "\n").replace("\\n", "\n").replace("\\h", " ");
        let body = body.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('\u{1}', "<").replace('\u{2}', ">");
        if body.trim().is_empty() { continue; }
        cues.push((s, e, body.trim().to_string()));
    }
    cues.sort_by(|a, b| a.0.cmp(&b.0));
    let mut out = String::from("WEBVTT\n\n");
    for (s, e, b) in cues { out.push_str(&format!("{s} --> {e}\n{b}\n\n")); }
    out
}

/// Any subtitle file as WebVTT.
pub fn to_vtt(path: &Path) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("Couldn't read the subtitles: {e}"))?;
    let text = decode_text(&bytes);
    Ok(match ext(path).as_str() {
        "srt" => srt_to_vtt(&text),
        "ass" | "ssa" => ass_to_vtt(&text),
        "vtt" => text,
        _ => return Err("That isn't a subtitle file Hearth Player knows.".into()),
    })
}

/// Subtitle files come in UTF-8, UTF-16 or old Windows code pages: read them as text either way.
pub fn decode_text(b: &[u8]) -> String {
    if b.starts_with(&[0xff, 0xfe]) || b.starts_with(&[0xfe, 0xff]) {
        let le = b[0] == 0xff;
        let u: Vec<u16> = b[2..].chunks_exact(2).map(|c| if le { u16::from_le_bytes([c[0], c[1]]) } else { u16::from_be_bytes([c[0], c[1]]) }).collect();
        return String::from_utf16_lossy(&u);
    }
    match std::str::from_utf8(b) {
        Ok(s) => s.to_string(),
        // Windows-1252 / Latin-1: close enough for the accented letters of most subtitles.
        Err(_) => b.iter().map(|&c| c as char).collect(),
    }
}

/// A file name for a copy that doesn't exist yet: "Clip (edited).mp4", "Clip (edited 2).mp4"...
pub fn copy_name(src: &Path, word: &str, new_ext: Option<&str>) -> PathBuf {
    let dir = src.parent().map(Path::to_path_buf).unwrap_or_default();
    let stem = src.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "video".into());
    let e = new_ext.map(String::from).unwrap_or_else(|| ext(src));
    for n in 1.. {
        let name = if n == 1 { format!("{stem} ({word}).{e}") } else { format!("{stem} ({word} {n}).{e}") };
        let p = dir.join(name);
        if !p.exists() { return p; }
    }
    unreachable!()
}

pub fn mime_for(p: &Path) -> &'static str {
    match ext(p).as_str() {
        "mp4" | "m4v" => "video/mp4",
        "mkv" => "video/x-matroska",
        "webm" => "video/webm",
        "mov" => "video/quicktime",
        "avi" => "video/x-msvideo",
        "wmv" => "video/x-ms-wmv",
        "ogv" => "video/ogg",
        "ts" | "m2ts" | "mts" => "video/mp2t",
        "mpg" | "mpeg" | "vob" => "video/mpeg",
        "3gp" => "video/3gpp",
        "flv" => "video/x-flv",
        "mp3" => "audio/mpeg",
        "flac" => "audio/flac",
        "ogg" | "oga" => "audio/ogg",
        "opus" => "audio/ogg; codecs=opus",
        "m4a" | "m4b" | "alac" => "audio/mp4",
        "aac" => "audio/aac",
        "wav" => "audio/wav",
        "wma" => "audio/x-ms-wma",
        "aiff" | "aif" => "audio/aiff",
        "mka" => "audio/x-matroska",
        "vtt" => "text/vtt; charset=utf-8",
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "webp" => "image/webp",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds() {
        assert_eq!(kind(Path::new("/a/B.MKV")), Some("video"));
        assert_eq!(kind(Path::new("x.flac")), Some("audio"));
        assert_eq!(kind(Path::new("x.m3u8")), Some("playlist"));
        assert_eq!(kind(Path::new("x.ass")), Some("subtitle"));
        assert_eq!(kind(Path::new("x.txt")), None);
    }

    #[test]
    fn natural_order() {
        let mut v = vec!["Track 10.mp3", "track 2.mp3", "Track 1.mp3", "Intro.mp3"];
        v.sort_by(|a, b| natural_cmp(a, b));
        assert_eq!(v, vec!["Intro.mp3", "Track 1.mp3", "track 2.mp3", "Track 10.mp3"]);
    }

    #[test]
    fn playlists() {
        let base = Path::new("/music");
        let m = parse_m3u("#EXTM3U\n#EXTINF:123,Song\nA/one.mp3\n\n/abs/two.flac\nhttps://radio.example/stream\nfile:///x/three%20b.ogg\n", base);
        assert_eq!(m[0], Path::new("/music").join("A/one.mp3").to_string_lossy());
        assert!(m[1].ends_with("two.flac"));
        assert_eq!(m[2], "https://radio.example/stream");
        assert!(m[3].ends_with("three b.ogg"));
        let p = parse_pls("[playlist]\nFile2=b.mp3\nTitle2=B\nFile1=http://a/x\nNumberOfEntries=2\n", base);
        assert_eq!(p[0], "http://a/x");
        assert!(p[1].ends_with("b.mp3"));
    }

    #[test]
    fn srt() {
        let v = srt_to_vtt("\u{feff}1\r\n00:00:01,500 --> 00:00:03,000\r\nHello <i>there</i>\r\n<font color=\"red\">red</font> & co\r\n\r\n2\r\n00:01:00,000 --> 00:01:02,250 X1:0\r\nBye\r\n");
        assert!(v.starts_with("WEBVTT\n\n"));
        assert!(v.contains("00:00:01.500 --> 00:00:03.000\nHello <i>there</i>\nred &amp; co\n"));
        assert!(v.contains("00:01:00.000 --> 00:01:02.250\nBye"));
    }

    #[test]
    fn ass() {
        let a = "[Script Info]\nTitle: x\n\n[V4+ Styles]\nFormat: Name, Fontname\n\n[Events]\nFormat: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\nDialogue: 0,0:00:05.10,0:00:07.00,Default,,0,0,0,,{\\i1}Hi{\\i0}, you\\Nsecond <line>\nDialogue: 0,0:00:01.00,0:00:02.00,Default,,0,0,0,,{\\pos(10,10)}First\nDialogue: 0,0:00:03.00,0:00:04.00,Default,,0,0,0,,{\\p1}m 0 0 l 1 1{\\p0}\n";
        let v = ass_to_vtt(a);
        let first = v.find("First").unwrap();
        let hi = v.find("<i>Hi</i>, you\nsecond &lt;line&gt;").unwrap();
        assert!(first < hi, "{v}");
        assert!(v.contains("00:00:05.100 --> 00:00:07.000"));
        assert!(!v.contains("m 0 0"));
    }

    #[test]
    fn sidecars_and_covers() {
        let d = std::env::temp_dir().join(format!("hp-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("Subs")).unwrap();
        for f in ["Movie.mkv", "Movie.srt", "Movie.en.srt", "Movie.fre.forced.ass", "Other.srt", "Subs/2_English.srt", "Folder.JPG", "a.mp3", "b.flac", "notes.txt"] {
            std::fs::write(d.join(f), b"x").unwrap();
        }
        let s = sidecar_subs(&d.join("Movie.mkv"));
        let labels: Vec<&str> = s.iter().map(|x| x.label.as_str()).collect();
        assert!(labels.contains(&"Subtitles (SRT file)"), "{labels:?}");
        assert!(labels.contains(&"English (SRT file)"), "{labels:?}");
        assert!(labels.contains(&"French (forced) (ASS file)"), "{labels:?}");
        assert!(s.iter().any(|x| x.path.contains("Subs") && x.lang == "english"), "{s:?}");
        assert!(!labels.iter().any(|l| l.contains("Other")));
        assert!(cover_file(&d).unwrap().ends_with("Folder.JPG"));
        let songs = folder_audio(&d.join("a.mp3"));
        assert_eq!(songs.iter().map(|i| i.name.as_str()).collect::<Vec<_>>(), vec!["a.mp3", "b.flac"]);
        let q = expand(&[d.to_string_lossy().into_owned()]);
        assert_eq!(q.len(), 3);
        assert_eq!(q.iter().find(|i| i.name == "Movie.mkv").unwrap().kind, "video");
        assert_eq!(copy_name(&d.join("Movie.mkv"), "edited", None).file_name().unwrap(), "Movie (edited).mkv");
        std::fs::write(d.join("Movie (edited).mkv"), b"x").unwrap();
        assert_eq!(copy_name(&d.join("Movie.mkv"), "edited", None).file_name().unwrap(), "Movie (edited 2).mkv");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn utf16_subs() {
        let mut b = vec![0xff, 0xfe];
        for u in "Hé".encode_utf16() { b.extend(u.to_le_bytes()); }
        assert_eq!(decode_text(&b), "Hé");
        assert_eq!(decode_text(&[b'c', 0xe9]), "cé");
    }
}
