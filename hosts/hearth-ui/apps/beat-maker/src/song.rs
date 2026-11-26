//! A song as Hearth Beat Maker keeps it: tracks (each an instrument or sound), clips on the song's bars, and patterns
//! (a beat of 16 steps, notes for the piano roll, or a recording). Saved as JSON (<name>.hbm.json, settings_file's
//! safe write with a .bak) and written as an LMMS project (.mmp: LMMS's own XML) so LMMS opens it and renders it.

use serde_json::{json, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Synth,
    Bass,
    Drums,
    Strings,
    Fx,
    Audio,
}

impl Kind {
    pub fn id(self) -> &'static str {
        match self {
            Kind::Synth => "synth",
            Kind::Bass => "bass",
            Kind::Drums => "drums",
            Kind::Strings => "strings",
            Kind::Fx => "fx",
            Kind::Audio => "audio",
        }
    }
    pub fn from_id(s: &str) -> Kind {
        match s {
            "bass" => Kind::Bass,
            "drums" => Kind::Drums,
            "strings" => Kind::Strings,
            "fx" => Kind::Fx,
            "audio" => Kind::Audio,
            _ => Kind::Synth,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Drum {
    Kick,
    Snare,
    Clap,
    ClosedHat,
    OpenHat,
    Tom,
    BigHit,
    Rim,
    Shaker,
    Crash,
}

pub const DRUMS: [(Drum, &str); 10] = [
    (Drum::Kick, "Kick"),
    (Drum::Snare, "Snare"),
    (Drum::Clap, "Clap"),
    (Drum::ClosedHat, "Closed hat"),
    (Drum::OpenHat, "Open hat"),
    (Drum::Tom, "Tom"),
    (Drum::BigHit, "Big hit"),
    (Drum::Rim, "Rim"),
    (Drum::Shaker, "Shaker"),
    (Drum::Crash, "Crash"),
];

pub fn drum_name(d: Drum) -> &'static str {
    DRUMS.iter().find(|x| x.0 == d).map(|x| x.1).unwrap_or("Kick")
}
pub fn drum_of(name: &str) -> Drum {
    DRUMS.iter().find(|x| x.1 == name).map(|x| x.0).unwrap_or(Drum::Kick)
}

/// An instrument: its kind, name, what it is, its mark (letters and two colours) and its four knobs (0–1): cutoff,
/// warmth, attack, reverb; and its effects (delay, glue).
#[derive(Debug, Clone, PartialEq)]
pub struct Inst {
    pub kind: Kind,
    pub name: String,
    pub sub: String,
    pub mark: String,
    pub from: u32,
    pub to: u32,
    pub knobs: [f32; 4],
    pub delay: bool,
    pub glue: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Clip {
    /// where it starts and how long it is, in bars (0 is the first bar)
    pub start: f32,
    pub len: f32,
    pub pat: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Track {
    pub name: String,
    pub color: u32,
    pub inst: Inst,
    pub mute: bool,
    pub solo: bool,
    pub clips: Vec<Clip>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub name: String,
    pub drum: Drum,
    /// each step: 0 no hit, else how hard (1–127)
    pub hits: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Note {
    /// MIDI note (60 is C4)
    pub key: i32,
    /// in beats from the pattern's start
    pub start: f32,
    pub len: f32,
    pub vel: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Body {
    Beat { rows: Vec<Row>, steps: usize, swing: f32 },
    Notes(Vec<Note>),
    /// a recording (a WAV file)
    Audio { file: String },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Pattern {
    pub name: String,
    pub body: Body,
}

impl Pattern {
    /// How long one pass of it is, in beats (a beat pattern: its steps as 16ths; notes: to the bar after the last).
    pub fn beats(&self, per_bar: f32) -> f32 {
        match &self.body {
            Body::Beat { steps, .. } => *steps as f32 / 4.0,
            Body::Notes(n) => {
                let end = n.iter().map(|x| x.start + x.len).fold(0.0f32, f32::max);
                ((end / per_bar).ceil().max(1.0)) * per_bar
            }
            Body::Audio { .. } => f32::MAX,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Song {
    pub name: String,
    pub note: String,
    pub bpm: f32,
    pub sig: (u8, u8),
    /// the key: 0 C … 11 B, and minor or major
    pub key: u8,
    pub minor: bool,
    pub bars: u32,
    pub tracks: Vec<Track>,
    pub patterns: Vec<Pattern>,
}

pub const KEYS: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];

impl Song {
    pub fn per_bar(&self) -> f32 {
        self.sig.0 as f32 * 4.0 / self.sig.1.max(1) as f32
    }
    pub fn sec_per_beat(&self) -> f32 {
        60.0 / self.bpm.max(20.0)
    }
    pub fn length_secs(&self) -> f32 {
        self.bars as f32 * self.per_bar() * self.sec_per_beat()
    }
    pub fn key_name(&self) -> String {
        format!("{} {}", KEYS[self.key as usize % 12], if self.minor { "minor" } else { "major" })
    }
    /// The notes of the key (pitch classes).
    pub fn scale(&self) -> Vec<i32> {
        let steps: &[i32] = if self.minor { &[0, 2, 3, 5, 7, 8, 10] } else { &[0, 2, 4, 5, 7, 9, 11] };
        steps.iter().map(|s| (s + self.key as i32) % 12).collect()
    }
    /// The nearest note in the key.
    pub fn fit(&self, key: i32) -> i32 {
        let sc = self.scale();
        for d in [0, -1, 1, -2, 2] {
            if sc.contains(&(key + d).rem_euclid(12)) {
                return key + d;
            }
        }
        key
    }
}

fn inst(kind: Kind, name: &str, sub: &str, mark: &str, from: u32, to: u32, knobs: [f32; 4]) -> Inst {
    Inst { kind, name: name.into(), sub: sub.into(), mark: mark.into(), from, to, knobs, delay: false, glue: false }
}

/// Hearth's own instruments (the Library's Synths, Drum kits and the trailer effects).
pub fn presets() -> Vec<Inst> {
    vec![
        inst(Kind::Synth, "Ember Pad", "synth · warm", "EP", 0xffb36b, 0xff6a1f, [0.52, 0.30, 0.62, 0.45]),
        inst(Kind::Synth, "Glass Lead", "synth · bright", "LD", 0xb9a2ff, 0x6a4fe0, [0.78, 0.15, 0.05, 0.35]),
        inst(Kind::Bass, "Deep Bass", "synth · sub", "BS", 0x7fd6ff, 0x2c7fd8, [0.35, 0.40, 0.02, 0.05]),
        inst(Kind::Drums, "Drum kit · Cinematic", "kick, snare, hats, toms", "DR", 0xff9a8a, 0xd8413a, [0.70, 0.50, 0.0, 0.25]),
        inst(Kind::Strings, "Strings", "sampled · 4 layers", "ST", 0x7ff0d8, 0x1f9f8f, [0.60, 0.35, 0.70, 0.60]),
        inst(Kind::Fx, "Rises and hits", "trailer effects", "FX", 0xffe28a, 0xf0a020, [0.80, 0.50, 0.30, 0.55]),
        inst(Kind::Synth, "Hearth Keys", "synth · soft", "HK", 0xffb36b, 0xd9531e, [0.45, 0.25, 0.08, 0.30]),
        inst(Kind::Synth, "Coal Pluck", "synth · short", "CP", 0xff9a8a, 0xd8413a, [0.65, 0.20, 0.0, 0.20]),
        inst(Kind::Bass, "Furnace Bass", "synth · growl", "FB", 0x7fd6ff, 0x2c7fd8, [0.55, 0.80, 0.01, 0.05]),
        inst(Kind::Strings, "Night Choir", "pad · voices", "NC", 0xb9a2ff, 0x6a4fe0, [0.40, 0.30, 0.85, 0.75]),
        inst(Kind::Drums, "Drum kit · Tight", "kick, snare, hats", "TK", 0xff9a8a, 0xd8413a, [0.85, 0.30, 0.0, 0.10]),
        inst(Kind::Drums, "Drum kit · Big Room", "kick, clap, crash", "BR", 0xff9a8a, 0xd8413a, [0.75, 0.60, 0.0, 0.45]),
    ]
}

fn beat(name: &str, swing: f32, rows: &[(&str, &[usize])]) -> Pattern {
    Pattern {
        name: name.into(),
        body: Body::Beat { steps: 16, swing, rows: rows.iter().map(|(n, on)| Row { name: n.to_string(), drum: drum_of(n), hits: (0..16).map(|i| if on.contains(&i) { if i % 4 == 0 { 110 } else { 92 } } else { 0 }).collect() }).collect() },
    }
}

fn notes(name: &str, n: &[(i32, f32, f32, u8)]) -> Pattern {
    Pattern { name: name.into(), body: Body::Notes(n.iter().map(|&(key, start, len, vel)| Note { key, start, len, vel }).collect()) }
}

/// A new song: one drum track with a beat, one pad, at 120 BPM in C minor.
pub fn blank(name: &str) -> Song {
    let p = presets();
    Song {
        name: name.into(),
        note: String::new(),
        bpm: 120.0,
        sig: (4, 4),
        key: 0,
        minor: true,
        bars: 16,
        tracks: vec![
            Track { name: "Drum kit".into(), color: 0xff9a8a, inst: p[3].clone(), mute: false, solo: false, clips: vec![Clip { start: 0.0, len: 4.0, pat: 0 }] },
            Track { name: "Ember Pad".into(), color: 0xffb36b, inst: p[0].clone(), mute: false, solo: false, clips: vec![Clip { start: 0.0, len: 4.0, pat: 1 }] },
        ],
        patterns: vec![beat("Beat 1", 0.0, &[("Kick", &[0, 8]), ("Snare", &[4, 12]), ("Closed hat", &[0, 2, 4, 6, 8, 10, 12, 14])]), notes("Chords 1", &[(60, 0.0, 4.0, 90), (63, 0.0, 4.0, 90), (67, 0.0, 4.0, 90)])],
    }
}

// ------------------------------------------------------------------ JSON
fn s(v: &Value, k: &str) -> String {
    v.get(k).and_then(Value::as_str).unwrap_or("").to_string()
}
fn f(v: &Value, k: &str, d: f64) -> f64 {
    v.get(k).and_then(Value::as_f64).unwrap_or(d)
}
fn b(v: &Value, k: &str) -> bool {
    v.get(k).and_then(Value::as_bool).unwrap_or(false)
}

pub fn to_json(sg: &Song) -> Value {
    json!({
        "format": 1,
        "app": "Hearth Beat Maker",
        "name": sg.name, "note": sg.note, "bpm": sg.bpm, "sig": [sg.sig.0, sg.sig.1], "key": sg.key, "minor": sg.minor, "bars": sg.bars,
        "tracks": sg.tracks.iter().map(|t| json!({
            "name": t.name, "color": format!("#{:06x}", t.color), "mute": t.mute, "solo": t.solo,
            "inst": {"kind": t.inst.kind.id(), "name": t.inst.name, "sub": t.inst.sub, "mark": t.inst.mark, "from": format!("#{:06x}", t.inst.from), "to": format!("#{:06x}", t.inst.to), "knobs": t.inst.knobs, "delay": t.inst.delay, "glue": t.inst.glue},
            "clips": t.clips.iter().map(|c| json!({"start": c.start, "len": c.len, "pattern": c.pat})).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "patterns": sg.patterns.iter().map(|p| match &p.body {
            Body::Beat { rows, steps, swing } => json!({"name": p.name, "beat": {"steps": steps, "swing": swing, "rows": rows.iter().map(|r| json!({"name": r.name, "drum": drum_name(r.drum), "hits": r.hits})).collect::<Vec<_>>()}}),
            Body::Notes(n) => json!({"name": p.name, "notes": n.iter().map(|x| json!([x.key, x.start, x.len, x.vel])).collect::<Vec<_>>()}),
            Body::Audio { file } => json!({"name": p.name, "audio": file}),
        }).collect::<Vec<_>>(),
    })
}

fn hex(s: &str) -> u32 {
    u32::from_str_radix(s.trim_start_matches('#'), 16).unwrap_or(0xc9c1ba)
}

pub fn from_json(v: &Value) -> Option<Song> {
    v.get("tracks")?.as_array()?;
    let sig = v.get("sig").and_then(Value::as_array).map(|a| (a.first().and_then(Value::as_u64).unwrap_or(4) as u8, a.get(1).and_then(Value::as_u64).unwrap_or(4) as u8)).unwrap_or((4, 4));
    let mut patterns = Vec::new();
    for p in v.get("patterns").and_then(Value::as_array).cloned().unwrap_or_default() {
        let body = if let Some(bt) = p.get("beat") {
            Body::Beat {
                steps: bt.get("steps").and_then(Value::as_u64).unwrap_or(16).clamp(4, 64) as usize,
                swing: f(bt, "swing", 0.0) as f32,
                rows: bt.get("rows").and_then(Value::as_array).cloned().unwrap_or_default().iter().map(|r| Row { name: s(r, "name"), drum: drum_of(&s(r, "drum")), hits: r.get("hits").and_then(Value::as_array).map(|h| h.iter().map(|x| x.as_u64().unwrap_or(0).min(127) as u8).collect()).unwrap_or_default() }).collect(),
            }
        } else if let Some(a) = p.get("audio").and_then(Value::as_str) {
            Body::Audio { file: a.to_string() }
        } else {
            Body::Notes(
                p.get("notes")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|n| {
                        let a = n.as_array()?;
                        Some(Note { key: a.first()?.as_i64()? as i32, start: a.get(1)?.as_f64()? as f32, len: a.get(2)?.as_f64()? as f32, vel: a.get(3).and_then(Value::as_u64).unwrap_or(100).min(127) as u8 })
                    })
                    .collect(),
            )
        };
        patterns.push(Pattern { name: s(&p, "name"), body });
    }
    let np = patterns.len();
    let tracks = v
        .get("tracks")?
        .as_array()?
        .iter()
        .map(|t| {
            let i = t.get("inst").cloned().unwrap_or(Value::Null);
            let kn = i.get("knobs").and_then(Value::as_array).map(|a| a.iter().map(|x| x.as_f64().unwrap_or(0.5) as f32).collect::<Vec<_>>()).unwrap_or_default();
            let k = |n: usize| kn.get(n).copied().unwrap_or(0.5).clamp(0.0, 1.0);
            Track {
                name: s(t, "name"),
                color: hex(&s(t, "color")),
                mute: b(t, "mute"),
                solo: b(t, "solo"),
                inst: Inst { kind: Kind::from_id(&s(&i, "kind")), name: s(&i, "name"), sub: s(&i, "sub"), mark: s(&i, "mark"), from: hex(&s(&i, "from")), to: hex(&s(&i, "to")), knobs: [k(0), k(1), k(2), k(3)], delay: b(&i, "delay"), glue: b(&i, "glue") },
                clips: t.get("clips").and_then(Value::as_array).cloned().unwrap_or_default().iter().filter_map(|c| {
                    let pat = c.get("pattern")?.as_u64()? as usize;
                    (pat < np).then(|| Clip { start: f(c, "start", 0.0) as f32, len: f(c, "len", 1.0).max(0.25) as f32, pat })
                }).collect(),
            }
        })
        .collect();
    Some(Song { name: s(v, "name"), note: s(v, "note"), bpm: f(v, "bpm", 120.0).clamp(20.0, 300.0) as f32, sig, key: (f(v, "key", 0.0) as u8) % 12, minor: v.get("minor").and_then(Value::as_bool).unwrap_or(true), bars: f(v, "bars", 16.0).clamp(1.0, 999.0) as u32, tracks, patterns })
}

pub fn valid(b: &[u8]) -> bool {
    serde_json::from_slice::<Value>(b).ok().and_then(|v| from_json(&v)).is_some()
}

// ------------------------------------------------------------------ LMMS
fn xml(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// LMMS's MIDI-ish key: LMMS counts from C-1 = 0 too, but its default base note is A4 = 57 (its own numbering, one
/// octave down from MIDI's).
fn lmms_key(midi: i32) -> i32 {
    midi - 12
}

/// The song as an LMMS project (.mmp): every track as an instrument track (TripleOscillator for synths, bass and
/// strings; Kicker for each drum; AudioFileProcessor for recordings), its clips as LMMS patterns (192 ticks a bar).
pub fn to_mmp(sg: &Song) -> String {
    let tpb = 48.0; // ticks a beat
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\"?>\n<!DOCTYPE lmms-project>\n");
    out.push_str(&format!("<lmms-project version=\"1.0\" creator=\"Hearth Beat Maker\" creatorversion=\"1.2\" type=\"song\">\n  <head timesig_numerator=\"{}\" timesig_denominator=\"{}\" bpm=\"{}\" mastervol=\"100\" masterpitch=\"0\"/>\n  <song>\n    <trackcontainer type=\"song\">\n", sg.sig.0, sg.sig.1, sg.bpm.round()));
    let per_bar = sg.per_bar();
    let osc = |t: &Track| -> String {
        let wave = match t.inst.kind {
            Kind::Bass => 0,
            Kind::Strings => 2,
            _ => 2,
        };
        format!(
            "<instrument name=\"tripleoscillator\"><tripleoscillator vol0=\"33\" wavetype0=\"{wave}\" coarse0=\"0\" finel0=\"-{d}\" finer0=\"{d}\" vol1=\"33\" wavetype1=\"{wave}\" coarse1=\"-12\" finel1=\"0\" finer1=\"0\" vol2=\"20\" wavetype2=\"0\" coarse2=\"0\" finel2=\"0\" finer2=\"0\"/></instrument><eldata fcut=\"{c}\" fres=\"0.5\" ftype=\"0\" fwet=\"1\"><elvol att=\"{a}\" dec=\"0.5\" sustain=\"0.6\" rel=\"0.3\" amt=\"1\"/></eldata>",
            d = (t.inst.knobs[1] * 15.0) as i32,
            c = (200.0 + t.inst.knobs[0] * 7800.0) as i32,
            a = t.inst.knobs[2]
        )
    };
    for t in &sg.tracks {
        let notes_of = |p: &Pattern| -> Vec<(i32, f32, f32, u8)> {
            match &p.body {
                Body::Notes(n) => n.iter().map(|x| (lmms_key(x.key), x.start, x.len, x.vel)).collect(),
                _ => vec![],
            }
        };
        match t.inst.kind {
            Kind::Drums => {
                // one LMMS track per drum sound (Kicker), as LMMS's Beat+Bassline would hold them
                let mut drums: Vec<Drum> = Vec::new();
                for c in &t.clips {
                    if let Some(Body::Beat { rows, .. }) = sg.patterns.get(c.pat).map(|p| &p.body) {
                        for r in rows {
                            if !drums.contains(&r.drum) {
                                drums.push(r.drum);
                            }
                        }
                    }
                }
                for d in drums {
                    out.push_str(&format!("      <track type=\"0\" name=\"{} · {}\" muted=\"{}\">\n        <instrumenttrack vol=\"100\" pan=\"0\" basenote=\"57\" pitch=\"0\" fxch=\"0\"><instrument name=\"kicker\"><kicker startfreq=\"{}\" endfreq=\"{}\" decay=\"{}\" noise=\"{}\" gain=\"1\"/></instrument></instrumenttrack>\n", xml(&t.name), drum_name(d), t.mute as u8, match d { Drum::Kick | Drum::BigHit => 150, Drum::Tom => 200, _ => 600 }, match d { Drum::Kick => 40, Drum::BigHit => 30, Drum::Tom => 90, _ => 300 }, match d { Drum::BigHit | Drum::Crash => 900, Drum::OpenHat => 300, Drum::Kick | Drum::Tom => 300, _ => 80 }, match d { Drum::Kick | Drum::Tom => 0.0, Drum::Snare | Drum::Clap => 0.6, _ => 1.0 }));
                    for c in &t.clips {
                        let Some(Body::Beat { rows, steps, .. }) = sg.patterns.get(c.pat).map(|p| &p.body) else { continue };
                        let len_beats = *steps as f32 / 4.0;
                        out.push_str(&format!("        <pattern name=\"{}\" pos=\"{}\" len=\"{}\" type=\"1\" muted=\"0\" steps=\"16\">\n", xml(&sg.patterns[c.pat].name), (c.start * per_bar * tpb) as i64, (c.len * per_bar * tpb) as i64));
                        let mut at = 0.0;
                        while at < c.len * per_bar {
                            for r in rows.iter().filter(|r| r.drum == d) {
                                for (i, h) in r.hits.iter().enumerate() {
                                    let beat = at + i as f32 / 4.0;
                                    if *h > 0 && beat < c.len * per_bar {
                                        out.push_str(&format!("          <note key=\"57\" vol=\"{}\" pos=\"{}\" len=\"-192\" pan=\"0\"/>\n", (*h as u32 * 100 / 127).max(1), (beat * tpb) as i64));
                                    }
                                }
                            }
                            at += len_beats;
                        }
                        out.push_str("        </pattern>\n");
                    }
                    out.push_str("      </track>\n");
                }
            }
            Kind::Audio => {
                for c in &t.clips {
                    if let Some(Body::Audio { file }) = sg.patterns.get(c.pat).map(|p| &p.body) {
                        out.push_str(&format!("      <track type=\"2\" name=\"{}\" muted=\"{}\">\n        <sampletrack vol=\"100\" pan=\"0\"/>\n        <sampletco pos=\"{}\" len=\"{}\" src=\"{}\" muted=\"0\"/>\n      </track>\n", xml(&t.name), t.mute as u8, (c.start * per_bar * tpb) as i64, (c.len * per_bar * tpb) as i64, xml(file)));
                    }
                }
            }
            _ => {
                out.push_str(&format!("      <track type=\"0\" name=\"{}\" muted=\"{}\">\n        <instrumenttrack vol=\"100\" pan=\"0\" basenote=\"57\" pitch=\"0\" fxch=\"0\">{}</instrumenttrack>\n", xml(&t.name), t.mute as u8, osc(t)));
                for c in &t.clips {
                    let Some(p) = sg.patterns.get(c.pat) else { continue };
                    let pass = p.beats(per_bar);
                    out.push_str(&format!("        <pattern name=\"{}\" pos=\"{}\" len=\"{}\" type=\"1\" muted=\"0\" steps=\"16\">\n", xml(&p.name), (c.start * per_bar * tpb) as i64, (c.len * per_bar * tpb) as i64));
                    let mut at = 0.0;
                    while at < c.len * per_bar && pass.is_finite() && pass > 0.0 {
                        for (k, st, ln, v) in notes_of(p) {
                            if at + st < c.len * per_bar {
                                out.push_str(&format!("          <note key=\"{k}\" vol=\"{}\" pos=\"{}\" len=\"{}\" pan=\"0\"/>\n", (v as u32 * 100 / 127).max(1), ((at + st) * tpb) as i64, (ln * tpb).max(1.0) as i64));
                            }
                        }
                        at += pass;
                    }
                    out.push_str("        </pattern>\n");
                }
                out.push_str("      </track>\n");
            }
        }
    }
    out.push_str("    </trackcontainer>\n  </song>\n</lmms-project>\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn json_reads_back_the_same() {
        let sg = blank("Test");
        let v = to_json(&sg);
        assert_eq!(from_json(&v).unwrap(), sg);
        assert!(valid(v.to_string().as_bytes()));
        assert!(!valid(b"{\"name\": 1}"));
    }
    #[test]
    fn keys_and_lmms() {
        let mut sg = blank("Test");
        sg.key = 2;
        sg.minor = true;
        assert_eq!(sg.key_name(), "D minor");
        assert_eq!(sg.fit(61), 60, "C# isn't in D minor: C is");
        let m = to_mmp(&sg);
        assert!(m.contains("<lmms-project") && m.contains("tripleoscillator") && m.contains("kicker"));
        assert_eq!(sg.length_secs(), 32.0);
    }
}
