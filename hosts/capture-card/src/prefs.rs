//! Your choices (red flag 9): ~/.config/nimbus/capture-card.json, only through hearth-ui's settings_file.rs (atomic, a
//! .bak kept, merged under its lock, never defaults over a file that didn't read). Opening the app or a page writes
//! nothing: a choice is written the moment you change it, and only that choice.

use hearth_ui::settings_file;
use serde_json::{Map, Value};
use std::path::PathBuf;

pub fn path() -> PathBuf {
    crate::home().join(".config/nimbus/capture-card.json")
}

#[derive(Debug, Clone, PartialEq)]
pub struct Prefs {
    /// The card's id (card.rs), "" for the first one found.
    pub card: String,
    /// "MJPG" or "YUYV".
    pub format: String,
    pub w: u32,
    pub h: u32,
    pub fps: u32,
    pub low_delay: bool,
    /// The sound's PipeWire source, "" for the card's own microphone (picked by itself).
    pub sound: String,
    pub play_sound: bool,
    pub volume: u32,
    pub muted: bool,
    /// "fit", "fill", "stretch".
    pub fit: String,
    /// "" for the usual folders.
    pub rec_dir: String,
    pub shot_dir: String,
    pub full_screen: bool,
    pub keep_awake: bool,
    /// "newest" or "oldest" (Recordings & screenshots).
    pub sort: String,
    /// The file couldn't be read (and wasn't restored from its .bak): nothing is written until it can be.
    pub unreadable: bool,
}

impl Default for Prefs {
    fn default() -> Self {
        Prefs {
            card: String::new(),
            format: "MJPG".into(),
            w: 1920,
            h: 1080,
            fps: 60,
            low_delay: true,
            sound: String::new(),
            play_sound: true,
            volume: 70,
            muted: false,
            fit: "fit".into(),
            rec_dir: String::new(),
            shot_dir: String::new(),
            full_screen: true,
            keep_awake: true,
            sort: "newest".into(),
            unreadable: false,
        }
    }
}

fn s(m: &Map<String, Value>, k: &str, ok: &[&str], d: &str) -> String {
    m.get(k).and_then(Value::as_str).filter(|v| ok.is_empty() || ok.contains(v)).unwrap_or(d).to_string()
}
fn b(m: &Map<String, Value>, k: &str, d: bool) -> bool {
    m.get(k).and_then(Value::as_bool).unwrap_or(d)
}
fn n(m: &Map<String, Value>, k: &str, lo: u64, hi: u64, d: u32) -> u32 {
    m.get(k).and_then(Value::as_u64).filter(|v| (lo..=hi).contains(v)).map(|v| v as u32).unwrap_or(d)
}

pub fn from_map(m: &Map<String, Value>) -> Prefs {
    let d = Prefs::default();
    let (w, h) = m.get("size").and_then(Value::as_str).and_then(parse_size).unwrap_or((d.w, d.h));
    Prefs {
        card: s(m, "card", &[], ""),
        format: s(m, "format", &["MJPG", "YUYV"], &d.format),
        w,
        h,
        fps: n(m, "fps", 1, 240, d.fps),
        low_delay: b(m, "lowDelay", d.low_delay),
        sound: s(m, "sound", &[], ""),
        play_sound: b(m, "playSound", d.play_sound),
        volume: n(m, "volume", 0, 100, d.volume),
        muted: b(m, "muted", d.muted),
        fit: s(m, "fit", &["fit", "fill", "stretch"], &d.fit),
        rec_dir: s(m, "recordingsDir", &[], ""),
        shot_dir: s(m, "screenshotsDir", &[], ""),
        full_screen: b(m, "startFullScreen", d.full_screen),
        keep_awake: b(m, "keepAwake", d.keep_awake),
        sort: s(m, "sort", &["newest", "oldest"], &d.sort),
        unreadable: false,
    }
}

pub fn parse_size(s: &str) -> Option<(u32, u32)> {
    let (a, b) = s.split_once('x')?;
    Some((a.trim().parse().ok()?, b.trim().parse().ok()?))
}

pub fn load() -> Prefs {
    load_from(&path())
}

pub fn load_from(p: &std::path::Path) -> Prefs {
    match settings_file::read(p) {
        settings_file::Read::Ok(m) | settings_file::Read::Restored(m) => from_map(&m),
        settings_file::Read::Missing => Prefs::default(),
        _ => Prefs { unreadable: true, ..Prefs::default() },
    }
}

/// Writes one change (merged into what's there). Refused while the file can't be read.
pub fn set(p: &Prefs, key: &str, v: Value) -> Result<(), String> {
    set_in(&path(), p, key, v)
}

pub fn set_in(file: &std::path::Path, p: &Prefs, key: &str, v: Value) -> Result<(), String> {
    if p.unreadable {
        return Err("Your settings file couldn't be read, so nothing was saved (it's left as it is).".into());
    }
    let mut m = Map::new();
    m.insert(key.to_string(), v);
    settings_file::merge_json(file, &m).map(|_| ()).map_err(|e| format!("Couldn't save it: {e}"))
}

/// The folder a kind of file is saved in: your choice, else the usual one (XDG's Videos / Pictures).
pub fn rec_dir(p: &Prefs) -> PathBuf {
    if !p.rec_dir.is_empty() {
        return PathBuf::from(&p.rec_dir);
    }
    xdg("VIDEOS", "Videos").join("Capture Card")
}

pub fn shot_dir(p: &Prefs) -> PathBuf {
    if !p.shot_dir.is_empty() {
        return PathBuf::from(&p.shot_dir);
    }
    xdg("PICTURES", "Pictures").join("Screenshots")
}

/// XDG's user folder (XDG_PICTURES_DIR in the environment or user-dirs.dirs), else ~/<fallback>.
pub fn xdg(kind: &str, fallback: &str) -> PathBuf {
    let home = crate::home();
    if let Ok(v) = std::env::var(format!("XDG_{kind}_DIR")) {
        if !v.is_empty() {
            return PathBuf::from(v);
        }
    }
    if let Ok(t) = std::fs::read_to_string(home.join(".config/user-dirs.dirs")) {
        for l in t.lines() {
            if let Some(r) = l.strip_prefix(&format!("XDG_{kind}_DIR=")) {
                let v = r.trim().trim_matches('"').replace("$HOME", &home.to_string_lossy());
                if !v.is_empty() && PathBuf::from(&v) != home {
                    return PathBuf::from(v);
                }
            }
        }
    }
    home.join(fallback)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn defaults_choices_and_a_broken_file() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("capture-card.json");
        let p = load_from(&f);
        assert_eq!(p, Prefs::default());
        assert!(!f.exists(), "reading writes nothing");
        set_in(&f, &p, "fit", json!("fill")).unwrap();
        set_in(&f, &p, "size", json!("1280x720")).unwrap();
        set_in(&f, &p, "volume", json!(35)).unwrap();
        let q = load_from(&f);
        assert_eq!((q.fit.as_str(), q.w, q.h, q.volume, q.fps), ("fill", 1280, 720, 35, 60));
        // Something else in the file stays (merge-on-write).
        std::fs::write(&f, "{\"fit\": \"stretch\", \"other\": 1}").unwrap();
        set_in(&f, &load_from(&f), "lowDelay", json!(false)).unwrap();
        let t = std::fs::read_to_string(&f).unwrap();
        assert!(t.contains("\"other\"") && t.contains("stretch"));
        // Unknown values fall back to the default, and a broken file is never written over.
        assert_eq!(from_map(json!({"format": "H264"}).as_object().unwrap()).format, "MJPG");
        std::fs::write(&f, "{broken").unwrap();
        let _ = std::fs::remove_file(settings_file::bak_path(&f));
        let bad = load_from(&f);
        assert!(bad.unreadable);
        assert!(set_in(&f, &bad, "fit", json!("fit")).is_err());
        assert_eq!(std::fs::read_to_string(&f).unwrap(), "{broken");
    }

    #[test]
    fn the_usual_folders() {
        let p = Prefs::default();
        assert!(rec_dir(&p).ends_with("Capture Card"));
        assert!(shot_dir(&p).ends_with("Screenshots"));
        let q = Prefs { rec_dir: "/data/Clips".into(), ..p };
        assert_eq!(rec_dir(&q), PathBuf::from("/data/Clips"));
        assert_eq!(parse_size("1920x1080"), Some((1920, 1080)));
        assert_eq!(parse_size("big"), None);
    }
}
