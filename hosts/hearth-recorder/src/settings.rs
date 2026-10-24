//! Recorder's settings (~/.config/hearth-recorder/settings.json), what the window and the shortcuts both use.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// 30 or 60
    pub fps: u32,
    /// "standard" or "high"
    pub quality: String,
    /// Record what's playing
    pub system: bool,
    /// Record the microphone
    pub mic: bool,
    /// Instant replay keeps this many seconds (30, 60, 300)
    pub replay_secs: u32,
    /// "auto", "hardware" or "software"
    pub encoder: String,
    /// The sound recorder's format: "opus" or "mp3"
    pub sound_format: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { fps: 60, quality: "standard".into(), system: true, mic: false, replay_secs: 30, encoder: "auto".into(), sound_format: "opus".into() }
    }
}

impl Settings {
    /// Anything out of range back to what works.
    pub fn clean(mut self) -> Self {
        if ![24, 30, 60].contains(&self.fps) { self.fps = 60; }
        if !["standard", "high"].contains(&self.quality.as_str()) { self.quality = "standard".into(); }
        self.replay_secs = self.replay_secs.clamp(10, 600);
        if !["auto", "hardware", "software"].contains(&self.encoder.as_str()) { self.encoder = "auto".into(); }
        if !["opus", "mp3"].contains(&self.sound_format.as_str()) { self.sound_format = "opus".into(); }
        self
    }
}

fn file() -> std::path::PathBuf { hearth_media::dirs::config("hearth-recorder").join("settings.json") }

pub fn load() -> Settings {
    std::fs::read(file()).ok().and_then(|b| serde_json::from_slice::<Settings>(&b).ok()).unwrap_or_default().clean()
}

pub fn save(s: &Settings) -> Result<Settings, String> {
    let s = s.clone().clean();
    hearth_media::dirs::write_atomic(&file(), &serde_json::to_vec_pretty(&s).unwrap_or_default()).map_err(|e| format!("Couldn't save the settings: {e}"))?;
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cleaning() {
        let s: Settings = serde_json::from_str(r#"{"fps": 144, "replaySecs": 5, "encoder": "magic", "mic": true}"#).unwrap();
        let s = s.clean();
        assert_eq!((s.fps, s.replay_secs, s.encoder.as_str(), s.mic, s.system), (60, 10, "auto", true, true));
    }
}
