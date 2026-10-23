//! What's being recorded right now, shared between the recording itself (a process of its own, session.rs), the
//! window, the shortcuts (hearth-recorder --toggle...) and Hearth's Quick Access: files in
//! $XDG_RUNTIME_DIR/hearth-recorder.
//!  - state.json: the recording that's going (its process, kind, file, when it started...)
//!  - last.json: how the last one ended (the file it saved, or why it stopped)
//!  - req-stop, req-save: asks the recording to stop, or instant replay to save (it looks every 100 ms)

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct State {
    /// record, replay or sound
    pub kind: String,
    /// starting, recording, saving, stopping
    pub status: String,
    pub pid: u32,
    /// When it started (ms since 1970)
    pub started: u64,
    pub file: String,
    pub backend: String,
    pub encoder: String,
    /// What sound is recorded, for people
    pub audio: String,
    pub area: bool,
    pub replay_secs: u32,
    /// Instant replay: how many times it saved, and the last file
    pub saves: u32,
    pub last_save: String,
    /// The sound recorder's level, 0 to 1
    pub level: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Last { pub kind: String, pub ok: bool, pub file: String, pub error: String, pub time: u64 }

pub fn dir() -> PathBuf { hearth_media::dirs::runtime("hearth-recorder") }
fn state_file() -> PathBuf { dir().join("state.json") }
fn last_file() -> PathBuf { dir().join("last.json") }

pub fn write(s: &State) { let _ = hearth_media::dirs::write_atomic(&state_file(), &serde_json::to_vec(s).unwrap_or_default()); }
pub fn clear() { let _ = std::fs::remove_file(state_file()); }

/// The recording that's going, if its process is still there (a leftover from a crash is cleaned up).
pub fn current() -> Option<State> {
    let s: State = serde_json::from_slice(&std::fs::read(state_file()).ok()?).ok()?;
    if hearth_media::tools::alive(s.pid) { Some(s) } else { clear(); None }
}

pub fn set_last(l: &Last) { let _ = hearth_media::dirs::write_atomic(&last_file(), &serde_json::to_vec(l).unwrap_or_default()); }
pub fn last() -> Option<Last> { serde_json::from_slice(&std::fs::read(last_file()).ok()?).ok() }
pub fn clear_last() { let _ = std::fs::remove_file(last_file()); }

/// Asks the recording for something ("stop", "save").
pub fn request(what: &str) { let _ = std::fs::write(dir().join(format!("req-{what}")), b""); }
/// The recording takes a request (true once per request).
pub fn take(what: &str) -> bool { std::fs::remove_file(dir().join(format!("req-{what}"))).is_ok() }
pub fn clear_requests() { for r in ["stop", "save"] { let _ = take(r); } }

/// The log of the recording's programs (their error messages).
pub fn log_file() -> PathBuf { dir().join("log.txt") }
