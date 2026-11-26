//! Hearth's own synth: the song rendered to sound (44.1 kHz, stereo), the same for playing as you go and for the
//! export, so what you hear is what you get. Subtractive synths (detuned saws through a low-pass: cutoff, warmth,
//! attack, reverb), a sub bass, four-layer strings, drums (kick, snare, clap, hats, tom, big hit, rim, shaker, crash),
//! trailer rises and hits, and recordings (16-bit WAV). Plus WAV reading and writing. No library: plain Rust.

use crate::song::{Body, Drum, Kind, Song, Track};

pub const RATE: u32 = 44100;

/// What one track plays: (start, length in seconds, MIDI key or drum, how hard 0–1).
#[derive(Debug, Clone)]
enum Ev {
    Note { at: f32, len: f32, key: i32, vel: f32 },
    Hit { at: f32, drum: Drum, vel: f32 },
    Audio { at: f32, len: f32, file: String },
}

fn events(sg: &Song, t: &Track) -> Vec<Ev> {
    let spb = sg.sec_per_beat();
    let per_bar = sg.per_bar();
    let mut v = Vec::new();
    for c in &t.clips {
        let Some(p) = sg.patterns.get(c.pat) else { continue };
        let clip_beats = c.len * per_bar;
        let base = c.start * per_bar;
        match &p.body {
            Body::Beat { rows, steps, swing } => {
                let pass = *steps as f32 / 4.0;
                let mut at = 0.0;
                while at < clip_beats - 1e-4 {
                    for r in rows {
                        for (i, h) in r.hits.iter().enumerate() {
                            if *h == 0 {
                                continue;
                            }
                            let mut beat = at + i as f32 / 4.0;
                            if i % 2 == 1 {
                                beat += swing / 100.0 * 0.25;
                            }
                            if beat < clip_beats {
                                v.push(Ev::Hit { at: (base + beat) * spb, drum: r.drum, vel: *h as f32 / 127.0 });
                            }
                        }
                    }
                    at += pass;
                }
            }
            Body::Notes(ns) => {
                let pass = p.beats(per_bar);
                let mut at = 0.0;
                while at < clip_beats - 1e-4 {
                    for n in ns {
                        let st = at + n.start;
                        if st < clip_beats {
                            let len = n.len.min(clip_beats - st);
                            v.push(Ev::Note { at: (base + st) * spb, len: len * spb, key: n.key, vel: n.vel as f32 / 127.0 });
                        }
                    }
                    at += pass;
                }
            }
            Body::Audio { file } => v.push(Ev::Audio { at: base * spb, len: clip_beats * spb, file: file.clone() }),
        }
    }
    v
}

struct Noise(u32);
impl Noise {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

fn hz(key: i32) -> f32 {
    440.0 * 2f32.powf((key - 69) as f32 / 12.0)
}

fn saw(ph: f32) -> f32 {
    2.0 * (ph - ph.floor()) - 1.0
}

/// One synth note into `out` (mono), by its instrument.
fn note(out: &mut [f32], t: &Track, at: usize, len: usize, key: i32, vel: f32, nz: &mut Noise) {
    let k = &t.inst.knobs;
    let sr = RATE as f32;
    let (voices, detune, sub, release): (usize, f32, f32, f32) = match t.inst.kind {
        Kind::Strings => (4, 0.004 + k[1] * 0.01, 0.0, 0.6),
        Kind::Bass => (1, 0.0, 1.0, 0.12),
        _ => (2, 0.002 + k[1] * 0.008, 0.0, 0.35),
    };
    let attack = match t.inst.kind {
        Kind::Bass => 0.004 + k[2] * 0.05,
        _ => 0.003 + k[2] * k[2] * 1.6,
    };
    let cutoff = 120.0 * (8000.0f32 / 120.0).powf(k[0]);
    let a_lp = 1.0 - (-2.0 * std::f32::consts::PI * cutoff / sr).exp();
    let f0 = hz(key);
    let total = len + (release * sr) as usize;
    let mut phases: Vec<f32> = (0..voices).map(|i| i as f32 * 0.37).collect();
    let (mut lp, mut lp2) = (0.0f32, 0.0f32);
    let mut sub_ph = 0.0f32;
    let drive = 1.0 + k[1] * 2.5;
    for i in 0..total {
        let idx = at + i;
        if idx >= out.len() {
            break;
        }
        let ts = i as f32 / sr;
        let env = if i < len {
            (ts / attack).min(1.0)
        } else {
            let r = (i - len) as f32 / (release * sr);
            ((len as f32 / sr / attack).min(1.0)) * (1.0 - r).max(0.0)
        };
        let mut x = 0.0;
        for (vi, ph) in phases.iter_mut().enumerate() {
            let spread = if voices > 1 { (vi as f32 / (voices - 1) as f32 - 0.5) * 2.0 } else { 0.0 };
            let f = f0 * (1.0 + spread * detune);
            *ph += f / sr;
            x += saw(*ph);
        }
        x /= voices as f32;
        if sub > 0.0 {
            sub_ph += f0 / sr;
            x = x * 0.35 * (0.3 + k[1]) + (sub_ph * std::f32::consts::TAU).sin() * 0.9;
        }
        lp += a_lp * (x - lp);
        lp2 += a_lp * (lp - lp2);
        let y = (lp2 * drive).tanh() * 0.9;
        let _ = nz;
        out[idx] += y * env * vel * 0.32;
    }
}

fn hit(out: &mut [f32], at: usize, d: Drum, vel: f32, nz: &mut Noise, tone: f32) {
    let sr = RATE as f32;
    let (len_s, gain) = match d {
        Drum::Kick => (0.45, 1.0),
        Drum::Snare => (0.25, 0.7),
        Drum::Clap => (0.22, 0.6),
        Drum::ClosedHat => (0.06, 0.3),
        Drum::OpenHat => (0.32, 0.28),
        Drum::Tom => (0.35, 0.75),
        Drum::BigHit => (1.6, 1.1),
        Drum::Rim => (0.04, 0.45),
        Drum::Shaker => (0.09, 0.22),
        Drum::Crash => (1.4, 0.3),
    };
    let n = (len_s * sr) as usize;
    let (mut ph, mut hp_lp, mut bp) = (0.0f32, 0.0f32, 0.0f32);
    for i in 0..n {
        let idx = at + i;
        if idx >= out.len() {
            break;
        }
        let ts = i as f32 / sr;
        let w = nz.next();
        let y = match d {
            Drum::Kick | Drum::BigHit | Drum::Tom => {
                let (f_hi, f_lo, fall) = match d {
                    Drum::Kick => (150.0, 45.0, 28.0),
                    Drum::Tom => (210.0, 95.0, 14.0),
                    _ => (90.0, 30.0, 6.0),
                };
                let f = f_lo + (f_hi - f_lo) * (-ts * fall).exp();
                ph += f / sr;
                let body = (ph * std::f32::consts::TAU).sin() * (-ts * if d == Drum::BigHit { 2.2 } else { 8.0 }).exp();
                let crack = if d == Drum::BigHit { w * (-ts * 3.5).exp() * 0.5 } else { w * (-ts * 120.0).exp() * 0.2 };
                (body + crack).tanh()
            }
            Drum::Snare | Drum::Clap => {
                ph += 190.0 / sr;
                hp_lp += 0.25 * (w - hp_lp);
                let noise = w - hp_lp;
                let env = if d == Drum::Clap { let burst = (ts * 100.0) as i32; if burst < 3 { (-(ts - burst as f32 / 100.0) * 300.0).exp() } else { (-ts * 18.0).exp() } } else { (-ts * 16.0).exp() };
                let tonal = if d == Drum::Snare { (ph * std::f32::consts::TAU).sin() * (-ts * 30.0).exp() * 0.5 } else { 0.0 };
                noise * env + tonal
            }
            Drum::ClosedHat | Drum::OpenHat | Drum::Shaker | Drum::Crash => {
                hp_lp += 0.6 * (w - hp_lp);
                let noise = w - hp_lp;
                let decay = match d {
                    Drum::ClosedHat => 60.0,
                    Drum::OpenHat => 9.0,
                    Drum::Shaker => 35.0,
                    _ => 2.6,
                };
                noise * (-ts * decay).exp()
            }
            Drum::Rim => {
                ph += 1700.0 / sr;
                bp += 0.5 * (w - bp);
                ((ph * std::f32::consts::TAU).sin() * 0.7 + bp * 0.3) * (-ts * 90.0).exp()
            }
        };
        out[idx] += y * gain * vel * (0.6 + tone * 0.6) * 0.5;
    }
}

/// A trailer rise (noise and a tone sweeping up over the note) or hit (a big boom), for the effects track.
fn fx(out: &mut [f32], at: usize, len: usize, key: i32, vel: f32, nz: &mut Noise) {
    if key < 60 {
        hit(out, at, Drum::BigHit, vel, nz, 1.0);
        return;
    }
    let sr = RATE as f32;
    let (mut lp, mut ph) = (0.0f32, 0.0f32);
    for i in 0..len {
        let idx = at + i;
        if idx >= out.len() {
            break;
        }
        let u = i as f32 / len.max(1) as f32;
        let cut = 200.0 * (40.0f32).powf(u);
        let a = 1.0 - (-2.0 * std::f32::consts::PI * cut / sr).exp();
        lp += a * (nz.next() - lp);
        ph += hz(key - 24) * (1.0 + u * 2.0) / sr;
        out[idx] += (lp * 0.8 + (ph * std::f32::consts::TAU).sin() * 0.25) * u * u * vel * 0.5;
    }
}

/// A simple room: a few feedback delays (Schroeder's combs), mixed by the reverb knob; and the delay effect.
fn space(x: &mut [f32], amount: f32, delay: bool, spb: f32) {
    if amount > 0.01 {
        let combs = [1557usize, 1617, 1491, 1422];
        let mut wet = vec![0.0f32; x.len()];
        for c in combs {
            let mut buf = vec![0.0f32; c];
            let mut i = 0;
            for (n, w) in wet.iter_mut().enumerate() {
                let y = buf[i];
                buf[i] = x[n] + y * 0.82;
                *w += y * 0.25;
                i = (i + 1) % c;
            }
        }
        for (a, w) in x.iter_mut().zip(wet) {
            *a += w * amount * 0.8;
        }
    }
    if delay {
        let d = (spb * 0.75 * RATE as f32) as usize;
        for n in d..x.len() {
            x[n] += x[n - d] * 0.35;
        }
    }
}

/// A 16-bit WAV's samples (mono, at 44.1 kHz: other rates and stereo are mixed and stepped to it).
pub fn read_wav(path: &str) -> Vec<f32> {
    let Ok(b) = std::fs::read(path) else { return vec![] };
    if b.len() < 44 || &b[0..4] != b"RIFF" || &b[8..12] != b"WAVE" {
        return vec![];
    }
    let (mut ch, mut rate, mut bits) = (1u16, RATE, 16u16);
    let mut i = 12;
    while i + 8 <= b.len() {
        let id = &b[i..i + 4];
        let sz = u32::from_le_bytes([b[i + 4], b[i + 5], b[i + 6], b[i + 7]]) as usize;
        let body = i + 8;
        if id == b"fmt " && body + 16 <= b.len() {
            ch = u16::from_le_bytes([b[body + 2], b[body + 3]]).max(1);
            rate = u32::from_le_bytes([b[body + 4], b[body + 5], b[body + 6], b[body + 7]]).max(1);
            bits = u16::from_le_bytes([b[body + 14], b[body + 15]]);
        } else if id == b"data" {
            if bits != 16 {
                return vec![];
            }
            let end = (body + sz).min(b.len());
            let frames: Vec<f32> = b[body..end].chunks_exact(2 * ch as usize).map(|f| f.chunks_exact(2).map(|s| i16::from_le_bytes([s[0], s[1]]) as f32 / 32768.0).sum::<f32>() / ch as f32).collect();
            if rate == RATE {
                return frames;
            }
            let ratio = rate as f32 / RATE as f32;
            let n = (frames.len() as f32 / ratio) as usize;
            return (0..n).map(|k| frames[((k as f32 * ratio) as usize).min(frames.len() - 1)]).collect();
        }
        i = body + sz + (sz & 1);
    }
    vec![]
}

/// One track rendered (mono), from the song's start, `secs` long.
pub fn track(sg: &Song, t: &Track, secs: f32) -> Vec<f32> {
    let n = (secs * RATE as f32) as usize;
    let mut out = vec![0.0f32; n];
    let mut nz = Noise(0x9e37_79b9 ^ t.color);
    for e in events(sg, t) {
        match e {
            Ev::Note { at, len, key, vel } => {
                let (a, l) = ((at * RATE as f32) as usize, (len * RATE as f32) as usize);
                if t.inst.kind == Kind::Fx {
                    fx(&mut out, a, l, key, vel, &mut nz);
                } else {
                    note(&mut out, t, a, l, key, vel, &mut nz);
                }
            }
            Ev::Hit { at, drum, vel } => hit(&mut out, (at * RATE as f32) as usize, drum, vel, &mut nz, t.inst.knobs[0]),
            Ev::Audio { at, len, file } => {
                let w = read_wav(&file);
                let a = (at * RATE as f32) as usize;
                let l = ((len * RATE as f32) as usize).min(w.len());
                for k in 0..l {
                    if a + k < out.len() {
                        out[a + k] += w[k] * 0.9;
                    }
                }
            }
        }
    }
    space(&mut out, t.inst.knobs[3], t.inst.delay, sg.sec_per_beat());
    if t.inst.glue {
        for x in out.iter_mut() {
            *x = (*x * 1.6).tanh() * 0.7;
        }
    }
    out
}

/// The whole song (or the tracks `only` names), stereo, as 16-bit samples, from `from` seconds.
pub fn mix(sg: &Song, only: Option<usize>, from: f32) -> Vec<i16> {
    let secs = sg.length_secs() + 2.0;
    let n = (secs * RATE as f32) as usize;
    let mut l = vec![0.0f32; n];
    let mut r = vec![0.0f32; n];
    let any_solo = sg.tracks.iter().any(|t| t.solo);
    for (i, t) in sg.tracks.iter().enumerate() {
        if let Some(o) = only {
            if o != i {
                continue;
            }
        } else if t.mute || (any_solo && !t.solo) {
            continue;
        }
        let x = track(sg, t, secs);
        // a little width: each track a step left or right of the middle
        let pan = match t.inst.kind {
            Kind::Drums | Kind::Bass | Kind::Audio => 0.0,
            _ => ((i % 3) as f32 - 1.0) * 0.25,
        };
        let (gl, gr) = ((1.0 - pan).min(1.0), (1.0 + pan).min(1.0));
        for k in 0..n.min(x.len()) {
            l[k] += x[k] * gl;
            r[k] += x[k] * gr;
        }
    }
    let start = ((from.max(0.0)) * RATE as f32) as usize;
    let mut out = Vec::with_capacity((n.saturating_sub(start)) * 2);
    for k in start..n {
        out.push(((l[k]).tanh() * 32000.0) as i16);
        out.push(((r[k]).tanh() * 32000.0) as i16);
    }
    out
}

/// 16-bit stereo WAV bytes.
pub fn wav(samples: &[i16], channels: u16) -> Vec<u8> {
    let data = samples.len() * 2;
    let mut b = Vec::with_capacity(44 + data);
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&((36 + data) as u32).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&channels.to_le_bytes());
    b.extend_from_slice(&RATE.to_le_bytes());
    b.extend_from_slice(&(RATE * channels as u32 * 2).to_le_bytes());
    b.extend_from_slice(&(channels * 2).to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&(data as u32).to_le_bytes());
    for s in samples {
        b.extend_from_slice(&s.to_le_bytes());
    }
    b
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_song_makes_sound_and_reads_back() {
        let sg = crate::song::blank("T");
        let m = mix(&sg, None, 0.0);
        assert!(m.iter().any(|s| s.unsigned_abs() > 1000), "it's not silent");
        let w = wav(&m[..44100 * 2], 2);
        let p = std::env::temp_dir().join("hearth-beat-test.wav");
        std::fs::write(&p, &w).unwrap();
        assert_eq!(read_wav(p.to_str().unwrap()).len(), 44100);
    }
}
