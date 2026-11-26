//! Your recordings, each in its own folder under ~/.local/share/hearth/listen/recordings: the sound (audio.wav, 16 kHz,
//! what whisper.cpp reads), meta.json (its name, when, where it came from) and words.json (what was said, with the
//! times). Written whole and atomically (settings_file: a temporary file, then renamed, with a .bak), never half.

use crate::whisper::{self, Seg};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Rec {
    pub id: String,
    pub dir: PathBuf,
    pub title: String,
    pub when: i64,
    pub secs: f64,
    /// where it came from: "Voice Studio", "Hearth Listen" (recorded here), or the file it was made from
    pub source: String,
    pub lang: String,
    /// how long whisper.cpp took
    pub took: f64,
    pub model: String,
    pub segs: Vec<Seg>,
    pub peaks: Vec<f32>,
    /// being made into words (or waiting)
    pub busy: String,
    pub error: String,
}

impl Rec {
    pub fn audio(&self) -> PathBuf {
        self.dir.join("audio.wav")
    }
    pub fn voices(&self) -> usize {
        let mut v: Vec<u8> = self.segs.iter().map(|s| s.voice).filter(|v| *v > 0).collect();
        v.sort();
        v.dedup();
        v.len()
    }
    pub fn has_words(&self) -> bool {
        !self.segs.is_empty()
    }
}

pub fn root() -> PathBuf {
    whisper::home().join("recordings")
}

fn read(p: &Path) -> Value {
    std::fs::read(p).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null)
}

pub fn load(dir: &Path) -> Option<Rec> {
    let meta = read(&dir.join("meta.json"));
    if !meta.is_object() {
        return None;
    }
    let words = read(&dir.join("words.json"));
    let segs = whisper::from_json(&words);
    let mut secs = words.get("duration").and_then(Value::as_f64).or_else(|| meta.get("secs").and_then(Value::as_f64)).unwrap_or(0.0);
    let mut peaks: Vec<f32> = meta.get("peaks").and_then(Value::as_array).map(|a| a.iter().filter_map(|x| x.as_f64().map(|f| f as f32)).collect()).unwrap_or_default();
    if peaks.is_empty() || secs <= 0.0 {
        if let Some(w) = whisper::read_wav(&dir.join("audio.wav")) {
            if secs <= 0.0 {
                secs = w.secs();
            }
            if peaks.is_empty() {
                peaks = w.peaks(160);
            }
        }
    }
    Some(Rec {
        id: dir.file_name()?.to_string_lossy().to_string(),
        dir: dir.to_path_buf(),
        title: meta.get("title").and_then(Value::as_str).unwrap_or("A recording").to_string(),
        when: meta.get("created").and_then(Value::as_i64).unwrap_or(0),
        secs,
        source: meta.get("source").and_then(Value::as_str).unwrap_or("Hearth Listen").to_string(),
        lang: words.get("language").and_then(Value::as_str).unwrap_or("").to_string(),
        took: words.get("took").and_then(Value::as_f64).unwrap_or(0.0),
        model: words.get("model").and_then(Value::as_str).unwrap_or("").to_string(),
        segs,
        peaks,
        busy: String::new(),
        error: meta.get("error").and_then(Value::as_str).unwrap_or("").to_string(),
    })
}

/// Every recording, newest first.
pub fn all() -> Vec<Rec> {
    let mut v: Vec<Rec> = std::fs::read_dir(root()).map(|rd| rd.flatten().filter_map(|e| load(&e.path())).collect()).unwrap_or_default();
    v.sort_by(|a, b| b.when.cmp(&a.when));
    v
}

/// A new recording's folder (named by when it was made), with its meta.json.
pub fn create(title: &str, source: &str) -> Result<PathBuf, String> {
    let t = hearth_oss::now();
    let mut id = format!("{t}");
    let mut n = 1;
    while root().join(&id).exists() {
        n += 1;
        id = format!("{t}-{n}");
    }
    let dir = root().join(&id);
    std::fs::create_dir_all(&dir).map_err(|e| format!("The recording's folder couldn't be made: {e}"))?;
    write(&dir.join("meta.json"), &json!({"title": title, "created": t, "source": source}))?;
    Ok(dir)
}

pub fn write(p: &Path, v: &Value) -> Result<(), String> {
    hearth_ui::settings_file::write_json(p, v).map(|_| ()).map_err(|e| format!("{} couldn't be saved: {e}", p.display()))
}

pub fn set_meta(dir: &Path, k: &str, v: Value) -> Result<(), String> {
    let p = dir.join("meta.json");
    let key = k.to_string();
    hearth_ui::settings_file::update_json(&p, move |m| {
        m.insert(key, v);
    })
    .map(|_| ())
    .map_err(|e| e.to_string())
}

/// Words in every recording that has them, for Search every word: (id, the line's index) of each match.
pub fn search(recs: &[Rec], q: &str) -> Vec<(String, usize)> {
    let q = q.trim().to_lowercase();
    if q.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for r in recs {
        if r.title.to_lowercase().contains(&q) {
            out.push((r.id.clone(), usize::MAX));
        }
        for (i, s) in r.segs.iter().enumerate() {
            if s.text.to_lowercase().contains(&q) {
                out.push((r.id.clone(), i));
            }
        }
    }
    out
}
