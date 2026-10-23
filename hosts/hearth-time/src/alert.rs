//! Telling you something: a notification (the desktop's notification service over D-Bus; Hearth's taskbar shows
//! it, and in Do not disturb keeps it in the clock panel quietly), and the alarm sound (a tone made here and
//! played by PipeWire, PulseAudio or ALSA's player, whichever is installed, until it's stopped).

use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

/// Urgency as the notification spec has it: critical ones show even in Do not disturb (alarms only).
#[derive(Clone, Copy, PartialEq)]
pub enum Urgency { Normal, Critical }

/// Shows a notification. Returns false when there's no notification service (Game Mode, no desktop).
pub fn notify(app: &str, icon: &str, summary: &str, body: &str, urgency: Urgency) -> bool {
    #[cfg(target_os = "linux")]
    {
        use std::collections::HashMap;
        use zbus::zvariant::Value as V;
        let Ok(conn) = zbus::blocking::Connection::session() else { return false };
        let mut hints: HashMap<&str, V> = HashMap::new();
        hints.insert("urgency", V::U8(if urgency == Urgency::Critical { 2 } else { 1 }));
        hints.insert("desktop-entry", V::from(icon));
        let actions: Vec<&str> = Vec::new();
        let r = conn.call_method(
            Some("org.freedesktop.Notifications"), "/org/freedesktop/Notifications", Some("org.freedesktop.Notifications"), "Notify",
            &(app, 0u32, icon, summary, body, actions, hints, if urgency == Urgency::Critical { 0i32 } else { -1i32 }),
        );
        r.is_ok()
    }
    #[cfg(not(target_os = "linux"))]
    { let _ = (app, icon, summary, body, urgency); false }
}

/// The alarm tone as a WAV file: three soft rising beeps and a pause, 2 seconds (played over and over).
pub fn tone_wav() -> Vec<u8> {
    let rate = 22_050u32;
    let total = rate as usize * 2;
    let mut samples: Vec<i16> = vec![0; total];
    let beeps = [(0.00f32, 880.0f32), (0.28, 988.0), (0.56, 1175.0)];
    for (start, freq) in beeps {
        let s0 = (start * rate as f32) as usize;
        let len = (0.22 * rate as f32) as usize;
        for i in 0..len {
            let t = i as f32 / rate as f32;
            // A quick fade in and out (no clicks), and a touch of the octave for a warmer bell.
            let env = (i.min(len - i) as f32 / (0.02 * rate as f32)).min(1.0);
            let v = (t * freq * std::f32::consts::TAU).sin() * 0.8 + (t * freq * 2.0 * std::f32::consts::TAU).sin() * 0.2;
            if let Some(x) = samples.get_mut(s0 + i) { *x = (v * env * 0.55 * i16::MAX as f32) as i16; }
        }
    }
    let data_len = (samples.len() * 2) as u32;
    let mut w = Vec::with_capacity(44 + data_len as usize);
    w.extend_from_slice(b"RIFF");
    w.extend_from_slice(&(36 + data_len).to_le_bytes());
    w.extend_from_slice(b"WAVEfmt ");
    w.extend_from_slice(&16u32.to_le_bytes());
    w.extend_from_slice(&1u16.to_le_bytes()); // PCM
    w.extend_from_slice(&1u16.to_le_bytes()); // mono
    w.extend_from_slice(&rate.to_le_bytes());
    w.extend_from_slice(&(rate * 2).to_le_bytes());
    w.extend_from_slice(&2u16.to_le_bytes());
    w.extend_from_slice(&16u16.to_le_bytes());
    w.extend_from_slice(b"data");
    w.extend_from_slice(&data_len.to_le_bytes());
    for s in samples { w.extend_from_slice(&s.to_le_bytes()); }
    w
}

/// The tone's file (written the first time).
pub fn tone_file() -> Option<PathBuf> {
    let p = crate::paths::sound_file();
    if !p.exists() { crate::store::write_bytes(&p, &tone_wav()).ok()?; }
    Some(p)
}

/// A program on PATH.
pub fn which(name: &str) -> Option<PathBuf> {
    std::env::var_os("PATH").and_then(|paths| std::env::split_paths(&paths).map(|d| d.join(name)).find(|p| p.is_file()))
}

/// Plays the tone over and over until `stop` (or the returned player is dropped by `Ringer::stop`).
pub struct Ringer { child: Option<Child>, file: Option<PathBuf>, player: Option<(PathBuf, Vec<String>)> }

impl Ringer {
    pub fn new() -> Ringer {
        let file = tone_file();
        let player = [("pw-play", vec![]), ("paplay", vec![]), ("aplay", vec!["-q".to_string()])].into_iter()
            .find_map(|(p, a)| which(p).map(|x| (x, a)));
        Ringer { child: None, file, player }
    }
    pub fn can_play(&self) -> bool { self.file.is_some() && self.player.is_some() }
    /// Call often: starts the tone again each time it ends.
    pub fn tick(&mut self, env: &[(String, String)]) {
        let running = self.child.as_mut().map(|c| matches!(c.try_wait(), Ok(None))).unwrap_or(false);
        if running { return; }
        let (Some(f), Some((p, args))) = (&self.file, &self.player) else { return };
        let mut cmd = Command::new(p);
        cmd.args(args).arg(f).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
        for (k, v) in env { if k == "XDG_RUNTIME_DIR" || k == "PULSE_SERVER" || k == "PIPEWIRE_REMOTE" { cmd.env(k, v); } }
        self.child = cmd.spawn().ok();
    }
    pub fn stop(&mut self) {
        if let Some(mut c) = self.child.take() { let _ = c.kill(); let _ = c.wait(); }
    }
}

impl Default for Ringer { fn default() -> Self { Ringer::new() } }
impl Drop for Ringer { fn drop(&mut self) { self.stop(); } }

#[cfg(test)]
mod tests {
    #[test]
    fn wav() {
        let w = super::tone_wav();
        assert_eq!(&w[0..4], b"RIFF");
        assert_eq!(&w[8..12], b"WAVE");
        assert_eq!(w.len(), 44 + 22_050 * 2 * 2);
        assert!(w[44..].iter().any(|b| *b != 0));
    }
}
