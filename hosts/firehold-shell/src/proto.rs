//! The hub's socket: `$XDG_RUNTIME_DIR/firehold-shell-<wayland socket>.sock`, one JSON object a line both ways.
//!
//! Surfaces say `{"op": "hello", "surface": "couch"}` and get `{"t": "state", ...}` whenever anything they show changed
//! (never on a timer of their own), `{"t": "library", "games": [...]}`, `{"t": "apps", ...}`, and `{"t": "nav",
//! "key": ...}` (the controller's presses for the surface that has the controller), `{"t": "cmd", ...}` and `{"t":
//! "quit"}`. Everything else they send is a command (hub.rs `command`). `firehold-shell ctl LINE` sends one by hand.

use serde::{Deserialize, Serialize};

use crate::config::Couch;
use crate::model::Win;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Notice {
    pub title: String,
    pub body: String,
    pub serial: u64,
}

/// The Guide over a game (mockup 3): what it shows, live.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Guide {
    pub up: bool,
    /// The game in front (fullscreen), if there's one.
    pub game: String,
    pub game_id: String,
    pub win: String,
    pub playing: String,
    /// firehold-comp's game rows answered (its game socket).
    pub comp: bool,
    pub frame_limit: String,
    pub scaling: String,
    pub vrr: bool,
    pub tearing: bool,
    pub scaling_note: String,
    pub volume: Option<i64>,
    pub brightness: Option<i64>,
    pub power: String,
    pub screenshot: String,
    /// The Record tile's line (Hearth Recorder's: instant replay's "Last 30 s too", recording, or how to start).
    pub record: String,
    /// Steam is here: Friends opens Steam's friends list (how many are online is Steam's own data: not read).
    pub friends: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Perf {
    pub line: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct State {
    /// couch or desk.
    pub posture: String,
    /// The Couch's screens are up (in Couch, with nothing launched over them).
    pub couch_up: bool,
    pub wins: Vec<Win>,
    pub active: String,
    /// The windows' ids, the one used last first (Everything open's order).
    pub mru: Vec<String>,
    pub clock: String,
    pub date: String,
    pub net: String,
    pub battery: String,
    pub initial: String,
    pub cfg: Couch,
    /// Desktop Mode on Anatase: firehold, firehold-plasma (the other Firehold test) or plasma.
    pub desktop: String,
    /// Firehold on Plasma is on this system (Settings › System offers it too).
    pub desktop_plasma: bool,
    pub guide: Guide,
    pub notice: Option<Notice>,
    pub now_ms: i64,
    pub perf: Perf,
    /// Where the hub reads the library from said it failed (shown on the Library instead of an empty grid).
    pub library_note: String,
}

pub fn socket_path() -> Option<std::path::PathBuf> {
    let dir = std::env::var_os("XDG_RUNTIME_DIR")?;
    let name = std::env::var("WAYLAND_DISPLAY").unwrap_or_else(|_| "wayland-0".into());
    let name = std::path::Path::new(&name).file_name()?.to_string_lossy().to_string();
    Some(std::path::Path::new(&dir).join(format!("firehold-shell-{name}.sock")))
}
