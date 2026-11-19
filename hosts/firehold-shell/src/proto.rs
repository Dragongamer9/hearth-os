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

/// One screen (firehold-comp's outputs) and what Settings › Couch and Desk calls it: TV, Built-in or Monitor.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Screen {
    pub name: String,
    pub label: String,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// A notification (org.freedesktop.Notifications, notify.rs): on screen for a few seconds, then in Quick Settings.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Note {
    pub id: u32,
    pub app: String,
    pub title: String,
    pub body: String,
    /// (key, label): the app's own buttons ("default" is a click on the card itself and isn't shown).
    pub actions: Vec<(String, String)>,
    /// When it came ("10:22 PM").
    pub at: String,
    pub urgent: bool,
}

/// Quick Settings (Mantel's QuickAccess.qml, for Firehold): what it shows, read when it opens.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Quick {
    pub up: bool,
    /// Which screen it's open on (the Desk's bar there), empty for the first.
    pub output: String,
    pub user: String,
    /// "78% · about 3 h 10 min left", or the date.
    pub line: String,
    pub wifi: Option<bool>,
    /// Bluetooth: None with no adapter; on or off.
    pub bt: Option<bool>,
    pub bt_line: String,
    pub night: Option<bool>,
    pub dnd: bool,
    pub volume: Option<i64>,
    pub muted: bool,
    pub brightness: Option<i64>,
    /// power-saver, balanced, performance ("" when power-profiles-daemon isn't there).
    pub profile: String,
    pub can_lock: bool,
    /// The power row (Sleep, Restart, Shut down) is open.
    pub power_open: bool,
}

/// Away (the home server keeps running): what the Away screen shows, from `hearth-home-server quick`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Away {
    pub on: bool,
    pub since: String,
    pub host: String,
    /// The home server's parts: (what, how it's doing, lit).
    pub lines: Vec<(String, String, bool)>,
    /// Hearth's home server isn't set up (or its helper isn't here): what Away says instead.
    pub note: String,
    /// Jobs paused for a game can run now (Light mode), and whether they were let.
    pub paused: Vec<String>,
    pub resumed: bool,
    /// What Firehold was before (couch or desk), for coming back.
    pub back_to: String,
}

/// A menu open from the Desk bar (File, Edit, View, or the posture pill's): where and what's in it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Menu {
    /// "File", "Edit", "View", "posture", or "" when none is open.
    pub open: String,
    pub output: String,
    /// From the screen's left edge, where the menu's title is in the bar.
    pub x: i32,
    pub items: Vec<MenuItem>,
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MenuItem {
    pub id: String,
    pub label: String,
    /// The shortcut it sends ("Ctrl+C"), shown on the right.
    pub keys: String,
    /// A line above it.
    pub sep: bool,
    pub on: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Perf {
    pub line: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct State {
    /// couch, desk or away: what every screen without its own posture is (Settings › Couch and Desk › Per screen).
    pub posture: String,
    /// The screens, and each one's posture now (its own when Per screen gives it one, else `posture`).
    pub screens: Vec<Screen>,
    pub postures: std::collections::BTreeMap<String, String>,
    /// Per screen is in use: two screens or more, and one has its own posture. Each surface is on its own screen then.
    pub per_screen: bool,
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
    pub quick: Quick,
    /// Notifications on screen now (newest first), and the last ones (Quick Settings).
    pub notes: Vec<Note>,
    pub history: Vec<Note>,
    pub dnd: bool,
    /// Firehold's on-screen keyboard is up.
    pub osk: bool,
    pub away: Away,
    pub menu: Menu,
}

impl State {
    /// The posture of one screen ("" or an unknown one: the shared posture).
    pub fn posture_of(&self, output: &str) -> String {
        if self.posture == "away" {
            return "away".into();
        }
        self.postures.get(output).cloned().unwrap_or_else(|| self.posture.clone())
    }
    /// Some screen is on the Couch.
    pub fn any_couch(&self) -> bool {
        self.posture == "couch" || (self.per_screen && self.posture != "away" && self.postures.values().any(|p| p == "couch"))
    }
    /// Some screen is on the Desk.
    pub fn any_desk(&self) -> bool {
        self.posture == "desk" || (self.per_screen && self.posture != "away" && self.postures.values().any(|p| p == "desk"))
    }
}

/// What Settings calls a screen (Mantel's firehold.js screenLabel): an HDMI screen that's big (or doesn't say) is the
/// TV, eDP / LVDS / DSI the built-in one, anything else its model or "Monitor".
pub fn screen_label(name: &str, model: &str, width_mm: i32) -> String {
    let n = name.to_ascii_lowercase();
    if n.contains("hdmi") && (width_mm <= 0 || width_mm > 700) {
        return "TV".into();
    }
    if n.starts_with("edp") || n.starts_with("lvds") || n.starts_with("dsi") {
        return "Built-in".into();
    }
    if model.is_empty() || model == "Headless" || model.eq_ignore_ascii_case("unknown") {
        "Monitor".into()
    } else {
        model.chars().take(24).collect()
    }
}

/// "TV: Couch · Monitor: Desk" (mockup 6), or "Every screen the same".
pub fn per_screen_text(per: &serde_json::Map<String, serde_json::Value>, screens: &[Screen]) -> String {
    let mut parts = Vec::new();
    for s in screens {
        match per.get(&s.name).and_then(|v| v.as_str()) {
            Some("couch") => parts.push(format!("{}: Couch", s.label)),
            Some("desk") => parts.push(format!("{}: Desk", s.label)),
            _ => {}
        }
    }
    // Screens that aren't plugged in now keep their posture: named as they're saved.
    for (k, v) in per {
        if !screens.iter().any(|s| &s.name == k) {
            if let Some(p) = v.as_str().filter(|p| *p == "couch" || *p == "desk") {
                parts.push(format!("{k}: {}", if p == "couch" { "Couch" } else { "Desk" }));
            }
        }
    }
    if parts.is_empty() {
        "Every screen the same".into()
    } else {
        parts.join(" · ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn screens_are_named_and_postured() {
        assert_eq!(screen_label("HDMI-A-1", "TV", 1210), "TV");
        assert_eq!(screen_label("HDMI-A-1", "", 0), "TV");
        assert_eq!(screen_label("HDMI-A-2", "Small", 300), "Small");
        assert_eq!(screen_label("eDP-1", "", 300), "Built-in");
        assert_eq!(screen_label("HEADLESS-1", "Headless", 0), "Monitor");
        let screens = vec![
            Screen { name: "HDMI-A-1".into(), label: "TV".into(), ..Screen::default() },
            Screen { name: "HEADLESS-1".into(), label: "Monitor".into(), ..Screen::default() },
        ];
        let per = json!({"HDMI-A-1": "couch", "HEADLESS-1": "desk"});
        assert_eq!(per_screen_text(per.as_object().unwrap(), &screens), "TV: Couch · Monitor: Desk");
        assert_eq!(per_screen_text(&serde_json::Map::new(), &screens), "Every screen the same");
        let mut st = State { posture: "desk".into(), per_screen: true, ..State::default() };
        st.postures.insert("HDMI-A-1".into(), "couch".into());
        st.postures.insert("HEADLESS-1".into(), "desk".into());
        assert_eq!(st.posture_of("HDMI-A-1"), "couch");
        assert_eq!(st.posture_of("DP-9"), "desk");
        assert!(st.any_couch() && st.any_desk());
        st.posture = "away".into();
        assert_eq!(st.posture_of("HDMI-A-1"), "away");
        assert!(!st.any_couch());
    }
}

pub fn socket_path() -> Option<std::path::PathBuf> {
    let dir = std::env::var_os("XDG_RUNTIME_DIR")?;
    let name = std::env::var("WAYLAND_DISPLAY").unwrap_or_else(|_| "wayland-0".into());
    let name = std::path::Path::new(&name).file_name()?.to_string_lossy().to_string();
    Some(std::path::Path::new(&dir).join(format!("firehold-shell-{name}.sock")))
}
