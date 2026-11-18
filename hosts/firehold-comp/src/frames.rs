//! Firehold's own title bars (the Desk, mockup docs/mockups/firehold/2-desk.png), drawn by the compositor itself:
//! "App — place" on the left, three round buttons on the right (minimize, maximize, close in the accent colour), and a
//! game in a window gets the accent border all round and its own line ("Ember Trails · running · 60 fps limit · VRR")
//! with the chip "Full screen: F11 or ≡". Mantel (FireholdFramesComp.qml, through the window IPC, ipc.rs) turns the look on
//! while the Desk is up and names each window the way Firehold does (it knows the apps and the games); the
//! compositor draws them, so they're always in the right order and drag, click and resize like any frame.
//!
//! Turned off (Hearth's plain frames, shell/ssd.rs) when Mantel says so, and whenever the IPC connection that turned it
//! on goes away (Mantel quit or restarted): nothing is kept in a file.

use std::{cell::RefCell, collections::HashMap};

use serde_json::Value;

use crate::theme::{hex, Rgba};

/// Firehold's palette (firehold.js's C), or what Mantel sends.
#[derive(Debug, Clone, PartialEq)]
pub struct Colours {
    pub panel2: Rgba,
    pub text: Rgba,
    pub muted: Rgba,
    pub line: Rgba,
    pub card: Rgba,
    pub card2: Rgba,
    pub accent: Rgba,
    pub flame: Rgba,
}

impl Default for Colours {
    fn default() -> Self {
        Colours {
            panel2: hex("#17110e").unwrap(),
            text: hex("#f4ece3").unwrap(),
            muted: hex("#a69484").unwrap(),
            line: hex("#3b2e26").unwrap(),
            card: hex("#261d18").unwrap(),
            card2: hex("#33271f").unwrap(),
            accent: hex("#ff8a3d").unwrap(),
            flame: hex("#ffd166").unwrap(),
        }
    }
}

/// How Firehold names a window.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Label {
    /// "Hearth Files — Pictures", or the game's name.
    pub title: String,
    pub game: bool,
    /// A game's line after its name: "running · 60 fps limit · VRR".
    pub line: String,
}

#[derive(Debug, Default)]
pub struct Look {
    pub firehold: bool,
    pub colours: Colours,
    pub labels: HashMap<u64, Label>,
    /// Bumped on every change, so bars are drawn again.
    pub serial: u64,
    /// The IPC connection that turned it on (it goes back when that one goes).
    pub owner: Option<u64>,
}

thread_local! {
    static LOOK: RefCell<Look> = RefCell::new(Look::default());
}

pub fn with<T>(f: impl FnOnce(&Look) -> T) -> T {
    LOOK.with(|l| f(&l.borrow()))
}

pub const CHIP_TEXT: &str = "Full screen: F11 or ≡";

/// Mantel's `frames` message: {"style": "firehold"|"hearth", "colours": {name: "#rrggbb"}, "names": {id: {title,
/// game, line}}}. Returns whether anything changed.
pub fn apply(v: &Value, owner: u64) -> Result<bool, String> {
    let o = v.as_object().ok_or("frames: not an object")?;
    let style = o.get("style").and_then(|s| s.as_str()).unwrap_or("firehold");
    let firehold = match style {
        "firehold" => true,
        "hearth" | "plain" => false,
        other => return Err(format!("frames: unknown style {other:?} (firehold, hearth)")),
    };
    let mut colours = Colours::default();
    if let Some(c) = o.get("colours").and_then(|c| c.as_object()) {
        for (k, slot) in [
            ("panel2", &mut colours.panel2),
            ("text", &mut colours.text),
            ("muted", &mut colours.muted),
            ("line", &mut colours.line),
            ("card", &mut colours.card),
            ("card2", &mut colours.card2),
            ("accent", &mut colours.accent),
            ("flame", &mut colours.flame),
        ] {
            if let Some(h) = c.get(k).and_then(|x| x.as_str()).and_then(hex) {
                *slot = h;
            }
        }
    }
    let mut labels = HashMap::new();
    if let Some(n) = o.get("names").and_then(|n| n.as_object()) {
        for (id, l) in n {
            let Ok(id) = id.parse::<u64>() else { continue };
            let s = |k: &str| l.get(k).and_then(|x| x.as_str()).unwrap_or("").chars().take(200).collect::<String>();
            labels.insert(id, Label { title: s("title"), game: l.get("game").and_then(|g| g.as_bool()) == Some(true), line: s("line") });
        }
    }
    Ok(LOOK.with(|look| {
        let mut l = look.borrow_mut();
        let changed = l.firehold != firehold || l.colours != colours || l.labels != labels;
        l.owner = firehold.then_some(owner);
        if changed {
            l.firehold = firehold;
            l.colours = colours;
            l.labels = labels;
            l.serial += 1;
        }
        changed
    }))
}

/// The connection that turned the look on is gone: Hearth's plain frames again. Returns whether it changed.
pub fn owner_gone(owner: u64) -> bool {
    LOOK.with(|look| {
        let mut l = look.borrow_mut();
        if l.owner != Some(owner) {
            return false;
        }
        l.owner = None;
        l.labels.clear();
        let was = l.firehold;
        l.firehold = false;
        l.serial += 1;
        was
    })
}

/// The label for a window: Mantel's, or its own title.
pub fn label_for(id: u64, title: &str) -> Label {
    with(|l| l.labels.get(&id).cloned()).unwrap_or_else(|| Label { title: title.to_string(), ..Label::default() })
}

/// The round buttons' centres from the bar's right edge (logical pixels): minimize, maximize, close. 14 px across,
/// 10 apart, 16 from the edge, as in the mockup.
pub const DOT: f64 = 14.0;
pub fn dot_centres(width: f64) -> [f64; 3] {
    let close = width - 16.0 - DOT / 2.0;
    [close - 2.0 * (DOT + 10.0), close - (DOT + 10.0), close]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn mantel_turns_it_on_and_it_goes_with_the_connection() {
        let v = json!({"style": "firehold", "colours": {"accent": "#00ff00"},
                       "names": {"7": {"title": "Hearth Files — Pictures"}, "9": {"title": "Ember Trails", "game": true, "line": "running · VRR"}, "x": {}}});
        assert!(apply(&v, 1).unwrap());
        assert!(!apply(&v, 1).unwrap(), "the same again changes nothing");
        with(|l| {
            assert!(l.firehold);
            assert_eq!(l.colours.accent, [0.0, 1.0, 0.0, 1.0]);
            assert_eq!(l.labels.len(), 2);
        });
        assert_eq!(label_for(9, "x").line, "running · VRR");
        assert!(label_for(9, "x").game);
        assert_eq!(label_for(5, "Its title").title, "Its title");
        assert!(!owner_gone(2), "another connection can't turn it off");
        assert!(owner_gone(1));
        with(|l| assert!(!l.firehold && l.labels.is_empty()));
        assert!(apply(&json!({"style": "fancy"}), 1).is_err());
    }

    #[test]
    fn buttons_sit_where_the_mockup_has_them() {
        let c = dot_centres(980.0);
        assert_eq!(c[2], 980.0 - 23.0);
        assert_eq!(c[1], c[2] - 24.0);
        assert_eq!(c[0], c[1] - 24.0);
    }
}
