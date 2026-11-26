//! whisper.cpp (MIT, github.com/ggml-org/whisper.cpp), driven through its own command, whisper-cli: which one is on
//! this PC (the system's, or the one Hearth built for you in ~/.local/share/hearth/listen/bin), its models (whisper.cpp's
//! own ggml files from its Hugging Face page, each checked against the SHA-1 whisper.cpp lists for it), the command
//! for one recording, and its JSON read back into lines with their times and voices.
//!
//! Installing it the Hearth way (only when you press Install): hearth-listen-install, in a transient unit of your own
//! (systemd-run --user, low priority), clones whisper.cpp's release tag (its commit checked against the one Hearth
//! pins), builds whisper-cli (with CUDA when the Nvidia toolkit is there) and puts it in your own folder. Nothing of the
//! system changes, nothing starts at boot.

use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub const TAG: &str = "v1.8.7";
pub const COMMIT: &str = "48f628a84833905ee4a0658ee6d4a5c915ce1997";

/// whisper.cpp's models: (file part, name in the app, size, SHA-1 from whisper.cpp's models/README.md at TAG).
pub const MODELS: &[(&str, &str, &str, &str)] = &[
    ("large-v3-turbo", "Large v3 turbo", "1.5 GB", "4af2b29d7ec73d781377bfd1758ca957a807e941"),
    ("large-v3-turbo-q5_0", "Large v3 turbo (small file)", "547 MB", "e050f7970618a659205450ad97eb95a18d69c9ee"),
    ("large-v3", "Large v3", "2.9 GB", "ad82bf6a9043ceed055076d0fd39f5f186ff8062"),
    ("medium", "Medium", "1.5 GB", "fd9727b6e1217c2f614f9b698455c4ffd82463b4"),
    ("small", "Small", "466 MB", "55356645c2b361a969dfd0ef2c5a50d530afd8d5"),
    ("base", "Base", "142 MB", "465707469ff3a37a2b9b8d8f89f2f99de7299dac"),
    ("tiny", "Tiny", "75 MB", "bd577a113a864445d4c299885e0cb97d4ba92b5f"),
];
/// whisper.cpp's tinydiarize model: it marks where the voice changes (English), for Tell voices apart.
pub const TDRZ: (&str, &str) = ("small.en-tdrz", "b6c6e7e89af1a35c08e6de56b66ca6a02a2fdfa1");

pub const LANGS: &[(&str, &str)] = &[
    ("auto", "Find it by itself"),
    ("en", "English"),
    ("es", "Spanish"),
    ("de", "German"),
    ("fr", "French"),
    ("it", "Italian"),
    ("pt", "Portuguese"),
    ("nl", "Dutch"),
    ("ja", "Japanese"),
    ("zh", "Chinese"),
];

pub fn lang_name(code: &str) -> String {
    LANGS.iter().find(|l| l.0 == code).map(|l| l.1.to_string()).unwrap_or_else(|| code.to_uppercase())
}
pub fn model_name(id: &str) -> String {
    MODELS.iter().find(|m| m.0 == id).map(|m| m.1.to_string()).unwrap_or_else(|| id.to_string())
}

pub fn home() -> PathBuf {
    std::env::var_os("HEARTH_LISTEN_DATA").map(PathBuf::from).unwrap_or_else(|| hearth_oss::data_dir("listen"))
}
pub fn models_dir() -> PathBuf {
    home().join("models")
}
pub fn model_file(id: &str) -> PathBuf {
    models_dir().join(format!("ggml-{id}.bin"))
}
pub fn have_model(id: &str) -> bool {
    model_file(id).is_file()
}
pub fn model_url(id: &str) -> String {
    format!("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-{id}.bin")
}

/// whisper-cli: the one Hearth built for you, else the system's.
pub fn cli() -> Option<PathBuf> {
    hearth_oss::which(&["whisper-cli", "whisper-cpp"], &[home().join("bin/whisper-cli")])
}
pub fn install_state() -> Value {
    hearth_ui::settings_file::read_json(&home().join("install.json"))
}

/// The graphics card whisper.cpp runs on (nvidia-smi's name, shortened: RTX 5070), or "" for none.
pub fn gpu() -> String {
    let o = hearth_ui::run::blocking(&hearth_ui::run::args(&["nvidia-smi", "--query-gpu=name", "--format=csv,noheader"]));
    if !o.ok {
        return String::new();
    }
    let n = o.out.lines().next().unwrap_or("").trim().to_string();
    n.replace("NVIDIA ", "").replace("GeForce ", "").trim().to_string()
}

/// How it listens (the panel's choices).
#[derive(Debug, Clone)]
pub struct How {
    pub model: String,
    pub gpu: bool,
    pub lang: String,
    pub voices: bool,
    pub punct: bool,
    pub translate: bool,
    pub words: Vec<String>,
}

/// whisper-cli's arguments for one 16 kHz WAV; its JSON goes to OUT.json.
pub fn args(bin: &Path, h: &How, wav: &Path, out: &Path, model: &str, tdrz: bool) -> Vec<String> {
    let mut a = vec![bin.to_string_lossy().to_string(), "-m".into(), model_file(model).to_string_lossy().to_string(), "-f".into(), wav.to_string_lossy().to_string(), "-oj".into(), "-ojf".into(), "-of".into(), out.to_string_lossy().to_string(), "-np".into()];
    a.push("-l".into());
    a.push(if tdrz { "en".into() } else { h.lang.clone() });
    if h.translate && !tdrz {
        a.push("-tr".into());
    }
    if !h.gpu {
        a.push("-ng".into());
    }
    if tdrz {
        a.push("-tdrz".into());
    }
    let words: Vec<&str> = h.words.iter().map(|w| w.trim()).filter(|w| !w.is_empty()).collect();
    if !words.is_empty() {
        // the words it should know, as whisper.cpp's prompt (it spells them that way)
        a.push("--prompt".into());
        a.push(format!("{}.", words.join(", ")));
    }
    a
}

/// One line of what was said.
#[derive(Debug, Clone, PartialEq)]
pub struct Seg {
    pub t0: f64,
    pub t1: f64,
    pub text: String,
    /// 0: one voice (not told apart), else VOICE n
    pub voice: u8,
    /// when each word starts (whisper.cpp's token times, -ojf); empty: spread evenly over the line
    pub wt: Vec<f64>,
}

/// Each word's start from whisper.cpp's tokens (a token starting with a space starts a word; [_…] are its own marks).
fn word_times(s: &Value) -> Vec<f64> {
    let mut wt = Vec::new();
    for tk in s.get("tokens").and_then(Value::as_array).cloned().unwrap_or_default() {
        let tx = tk.get("text").and_then(Value::as_str).unwrap_or("");
        if tx.starts_with("[_") || tx.trim().is_empty() && !tx.is_empty() && wt.is_empty() {
            continue;
        }
        let at = tk.pointer("/offsets/from").and_then(Value::as_f64).unwrap_or(0.0) / 1000.0;
        if tx.starts_with(' ') || wt.is_empty() {
            wt.push(at);
        }
    }
    wt
}

fn ms(v: &Value, k: &str) -> f64 {
    v.pointer(&format!("/offsets/{k}")).and_then(Value::as_f64).unwrap_or(0.0) / 1000.0
}

/// whisper.cpp's JSON (-oj): its lines, the language it heard; with -tdrz each line says whether the voice changes
/// after it (speaker_turn_next), with -di which voice it is ("speaker").
pub fn parse(j: &Value) -> (Vec<Seg>, String, Vec<f64>) {
    let mut out = Vec::new();
    let mut turns = Vec::new();
    for s in j.get("transcription").and_then(Value::as_array).cloned().unwrap_or_default() {
        let text = s.get("text").and_then(Value::as_str).unwrap_or("").trim().to_string();
        let (a, b) = (ms(&s, "from"), ms(&s, "to"));
        if s.get("speaker_turn_next").and_then(Value::as_bool).unwrap_or(false) {
            turns.push(b);
        }
        if text.is_empty() || text == "[BLANK_AUDIO]" {
            continue;
        }
        let voice = s.get("speaker").and_then(Value::as_str).and_then(|x| x.chars().filter(|c| c.is_ascii_digit()).collect::<String>().parse::<u8>().ok()).map(|n| n + 1).unwrap_or(0);
        let text = text.replace(" [SPEAKER_TURN]", "").replace("[SPEAKER_TURN]", "");
        let mut wt = word_times(&s);
        if wt.len() != text.split_whitespace().count() {
            wt.clear();
        }
        out.push(Seg { t0: a, t1: b, text, voice, wt });
    }
    let lang = j.pointer("/result/language").and_then(Value::as_str).unwrap_or("").to_string();
    (out, lang, turns)
}

/// The voices from tinydiarize's turns: the voice changes at each turn (two voices: 1, 2, 1…).
pub fn voices_from_turns(segs: &mut [Seg], turns: &[f64]) {
    for s in segs.iter_mut() {
        let mid = (s.t0 + s.t1) / 2.0;
        let n = turns.iter().filter(|t| **t <= mid).count();
        s.voice = (n % 2) as u8 + 1;
    }
}

/// Commas and capitals off: the words as they were said, all small, no marks.
pub fn plain(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric() || c.is_whitespace() || *c == '\'' || *c == '-').collect::<String>().to_lowercase()
}

/// Our own file for a recording's words (words.json), what Send to Voice Studio and Export use.
pub fn to_json(segs: &[Seg], lang: &str, secs: f64, took: f64, model: &str) -> Value {
    json!({
        "language": lang, "duration": secs, "took": took, "model": model, "engine": format!("whisper.cpp {TAG}"),
        "segments": segs.iter().map(|s| json!({"start": s.t0, "end": s.t1, "text": s.text, "speaker": if s.voice > 0 { format!("VOICE {}", s.voice) } else { String::new() }, "words": s.wt})).collect::<Vec<_>>(),
    })
}
pub fn from_json(v: &Value) -> Vec<Seg> {
    v.get("segments")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .map(|s| Seg {
                    t0: s.get("start").and_then(Value::as_f64).unwrap_or(0.0),
                    t1: s.get("end").and_then(Value::as_f64).unwrap_or(0.0),
                    text: s.get("text").and_then(Value::as_str).unwrap_or("").to_string(),
                    voice: s.get("speaker").and_then(Value::as_str).and_then(|x| x.trim_start_matches("VOICE ").parse::<u8>().ok()).unwrap_or(0),
                    wt: s.get("words").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_f64).collect()).unwrap_or_default(),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn stamp(t: f64, sep: char) -> String {
    let ms = (t.max(0.0) * 1000.0).round() as u64;
    format!("{:02}:{:02}:{:02}{sep}{:03}", ms / 3_600_000, ms / 60_000 % 60, ms / 1000 % 60, ms % 1000)
}
pub fn to_srt(segs: &[Seg]) -> String {
    segs.iter().enumerate().map(|(i, s)| format!("{}\n{} --> {}\n{}{}\n", i + 1, stamp(s.t0, ','), stamp(s.t1, ','), if s.voice > 0 { format!("[VOICE {}] ", s.voice) } else { String::new() }, s.text)).collect::<Vec<_>>().join("\n")
}
pub fn to_vtt(segs: &[Seg]) -> String {
    let mut o = String::from("WEBVTT\n\n");
    for s in segs {
        o.push_str(&format!("{} --> {}\n{}{}\n\n", stamp(s.t0, '.'), stamp(s.t1, '.'), if s.voice > 0 { format!("<v VOICE {}>", s.voice) } else { String::new() }, s.text));
    }
    o
}
pub fn to_txt(segs: &[Seg]) -> String {
    let mut o = String::new();
    let mut last = 255u8;
    for s in segs {
        if s.voice > 0 && s.voice != last {
            o.push_str(&format!("\nVOICE {}: ", s.voice));
            last = s.voice;
        } else if !o.is_empty() {
            o.push(' ');
        }
        o.push_str(&s.text);
    }
    o.trim().to_string() + "\n"
}

/// m:ss
pub fn mmss(t: f64) -> String {
    let s = t.max(0.0) as u64;
    if s >= 3600 {
        format!("{}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
    } else {
        format!("{}:{:02}", s / 60, s % 60)
    }
}

// ------------------------------------------------------------------ WAV (16-bit PCM): length, peaks, a slice

pub struct Wav {
    pub rate: u32,
    pub ch: u16,
    pub data: Vec<i16>,
}

pub fn read_wav(p: &Path) -> Option<Wav> {
    let b = std::fs::read(p).ok()?;
    if b.len() < 44 || &b[0..4] != b"RIFF" || &b[8..12] != b"WAVE" {
        return None;
    }
    let (mut i, mut rate, mut ch, mut bits) = (12usize, 0u32, 0u16, 0u16);
    while i + 8 <= b.len() {
        let id = &b[i..i + 4];
        let n = u32::from_le_bytes([b[i + 4], b[i + 5], b[i + 6], b[i + 7]]) as usize;
        let body = i + 8;
        if id == b"fmt " && body + 16 <= b.len() {
            ch = u16::from_le_bytes([b[body + 2], b[body + 3]]);
            rate = u32::from_le_bytes([b[body + 4], b[body + 5], b[body + 6], b[body + 7]]);
            bits = u16::from_le_bytes([b[body + 14], b[body + 15]]);
        } else if id == b"data" {
            if bits != 16 || ch == 0 {
                return None;
            }
            let end = (body + n).min(b.len());
            let data = b[body..end].chunks_exact(2).map(|c| i16::from_le_bytes([c[0], c[1]])).collect();
            return Some(Wav { rate, ch, data });
        }
        i = body + n + (n & 1);
    }
    None
}

impl Wav {
    pub fn secs(&self) -> f64 {
        self.data.len() as f64 / self.ch.max(1) as f64 / self.rate.max(1) as f64
    }
    /// n bars, each the loudest moment in its part (0..1).
    pub fn peaks(&self, n: usize) -> Vec<f32> {
        let len = self.data.len().max(1);
        let step = (len / n.max(1)).max(1);
        let mut v: Vec<f32> = self.data.chunks(step).take(n).map(|c| c.iter().map(|x| (*x as i32).unsigned_abs()).max().unwrap_or(0) as f32 / 32768.0).collect();
        let top = v.iter().cloned().fold(0.01f32, f32::max);
        for x in v.iter_mut() {
            *x = (*x / top).sqrt();
        }
        v
    }
}

/// A WAV from `from` seconds on, its rate multiplied by `speed` (played faster or slower: the voice's pitch moves with
/// it, as pw-play has no tempo of its own).
pub fn slice(w: &Wav, from: f64, speed: f32) -> Vec<u8> {
    let start = ((from.max(0.0) * w.rate as f64) as usize * w.ch as usize).min(w.data.len());
    let pcm = &w.data[start..];
    let rate = ((w.rate as f32) * speed.clamp(0.5, 2.0)) as u32;
    wav_bytes(pcm, rate, w.ch)
}
pub fn wav_bytes(pcm: &[i16], rate: u32, ch: u16) -> Vec<u8> {
    let n = (pcm.len() * 2) as u32;
    let mut o = Vec::with_capacity(44 + n as usize);
    o.extend_from_slice(b"RIFF");
    o.extend_from_slice(&(36 + n).to_le_bytes());
    o.extend_from_slice(b"WAVEfmt ");
    o.extend_from_slice(&16u32.to_le_bytes());
    o.extend_from_slice(&1u16.to_le_bytes());
    o.extend_from_slice(&ch.to_le_bytes());
    o.extend_from_slice(&rate.to_le_bytes());
    o.extend_from_slice(&(rate * ch as u32 * 2).to_le_bytes());
    o.extend_from_slice(&(ch * 2).to_le_bytes());
    o.extend_from_slice(&16u16.to_le_bytes());
    o.extend_from_slice(b"data");
    o.extend_from_slice(&n.to_le_bytes());
    for s in pcm {
        o.extend_from_slice(&s.to_le_bytes());
    }
    o
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whisper_cpp_json_is_read_with_times_and_turns() {
        let j = json!({"result": {"language": "en"}, "transcription": [
            {"offsets": {"from": 91000, "to": 97000}, "text": " This is Hearth OS one point eighteen.", "speaker_turn_next": false,
             "tokens": [{"text": "[_BEG_]", "offsets": {"from": 91000}}, {"text": " This", "offsets": {"from": 91000}}, {"text": " is", "offsets": {"from": 91400}}, {"text": " Hearth", "offsets": {"from": 91700}},
                        {"text": " OS", "offsets": {"from": 92100}}, {"text": " one", "offsets": {"from": 92500}}, {"text": " point", "offsets": {"from": 92800}}, {"text": " eigh", "offsets": {"from": 93200}}, {"text": "teen", "offsets": {"from": 93500}}, {"text": ".", "offsets": {"from": 93900}}]},
            {"offsets": {"from": 98000, "to": 104000}, "text": " Start with the desktop.", "speaker_turn_next": true},
            {"offsets": {"from": 105000, "to": 107000}, "text": " And it's not just the desktop. [SPEAKER_TURN]"},
            {"offsets": {"from": 108000, "to": 109000}, "text": " [BLANK_AUDIO]"}]});
        let (mut s, lang, turns) = parse(&j);
        assert_eq!((s.len(), lang.as_str(), turns.len()), (3, "en", 1));
        voices_from_turns(&mut s, &turns);
        assert_eq!(s.iter().map(|x| x.voice).collect::<Vec<_>>(), [1, 1, 2]);
        assert_eq!(s[2].text, "And it's not just the desktop.");
        assert_eq!(s[0].wt, [91.0, 91.4, 91.7, 92.1, 92.5, 92.8, 93.2]);
        assert!(s[1].wt.is_empty());
        assert!(to_srt(&s).contains("00:01:31,000 --> 00:01:37,000\n[VOICE 1] This is Hearth OS"));
        assert!(to_vtt(&s).starts_with("WEBVTT"));
        assert_eq!(from_json(&to_json(&s, "en", 252.0, 21.0, "large-v3-turbo")), s);
        assert_eq!(plain("Right-click any part, and pick Customize."), "right-click any part and pick customize");
        assert_eq!(mmss(252.0), "4:12");
    }

    #[test]
    fn the_command_carries_every_choice() {
        let h = How { model: "large-v3-turbo".into(), gpu: false, lang: "auto".into(), voices: false, punct: true, translate: true, words: vec!["Hearth".into(), "Mantel".into()] };
        let a = args(Path::new("/x/whisper-cli"), &h, Path::new("/r/a.wav"), Path::new("/r/w"), "large-v3-turbo", false);
        for w in ["-oj", "-tr", "-ng", "--prompt", "Hearth, Mantel.", "auto"] {
            assert!(a.iter().any(|x| x == w), "{w} in {a:?}");
        }
    }

    #[test]
    fn wav_both_ways() {
        let pcm: Vec<i16> = (0..16000).map(|i| ((i as f32 / 10.0).sin() * 8000.0) as i16).collect();
        let b = wav_bytes(&pcm, 16000, 1);
        let p = std::env::temp_dir().join("hearth-listen-test.wav");
        std::fs::write(&p, &b).unwrap();
        let w = read_wav(&p).unwrap();
        assert_eq!((w.rate, w.ch, w.data.len()), (16000, 1, 16000));
        assert!((w.secs() - 1.0).abs() < 1e-6);
        assert_eq!(w.peaks(50).len(), 50);
        assert_eq!(slice(&w, 0.5, 1.0).len(), 44 + 16000);
        let _ = std::fs::remove_file(p);
    }
}
