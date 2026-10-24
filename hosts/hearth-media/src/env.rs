//! Which kind of session this is, because each needs a different way to capture the screen:
//!  - gamescope: Game Mode (gamescope sets GAMESCOPE_WAYLAND_DISPLAY for everything it starts)
//!  - wayland: Mantel, Desktop Mode (labwc, a wlroots compositor: wf-recorder and slurp work there)
//!  - x11: an X session (and CI's Xvfb)
//! HEARTH_SESSION_KIND=gamescope|wayland|x11 overrides it (tests).

use serde::Serialize;

#[derive(Serialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Session { Gamescope, Wayland, X11, None }

impl Session {
    pub fn name(self) -> &'static str {
        match self { Session::Gamescope => "gamescope", Session::Wayland => "wayland", Session::X11 => "x11", Session::None => "none" }
    }
    /// How to say it to people.
    pub fn label(self) -> &'static str {
        match self { Session::Gamescope => "Game Mode", Session::Wayland => "Desktop Mode", Session::X11 => "X11 desktop", Session::None => "no screen" }
    }
}

pub fn session() -> Session { detect(&|k| std::env::var(k).ok()) }

pub fn detect(get: &dyn Fn(&str) -> Option<String>) -> Session {
    let set = |k: &str| get(k).map(|v| !v.trim().is_empty()).unwrap_or(false);
    match get("HEARTH_SESSION_KIND").unwrap_or_default().as_str() {
        "gamescope" => return Session::Gamescope,
        "wayland" => return Session::Wayland,
        "x11" => return Session::X11,
        _ => {}
    }
    if set("GAMESCOPE_WAYLAND_DISPLAY") { Session::Gamescope }
    else if set("WAYLAND_DISPLAY") { Session::Wayland }
    else if set("DISPLAY") { Session::X11 }
    else { Session::None }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn with(pairs: &[(&str, &str)]) -> Session {
        let v: Vec<(String, String)> = pairs.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect();
        detect(&move |k| v.iter().find(|(a, _)| a == k).map(|(_, b)| b.clone()))
    }
    #[test]
    fn kinds() {
        assert_eq!(with(&[("GAMESCOPE_WAYLAND_DISPLAY", "gamescope-0"), ("DISPLAY", ":1"), ("WAYLAND_DISPLAY", "gamescope-0")]), Session::Gamescope);
        assert_eq!(with(&[("WAYLAND_DISPLAY", "wayland-0"), ("DISPLAY", ":0")]), Session::Wayland);
        assert_eq!(with(&[("DISPLAY", ":99")]), Session::X11);
        assert_eq!(with(&[("DISPLAY", "")]), Session::None);
        assert_eq!(with(&[("DISPLAY", ":99"), ("HEARTH_SESSION_KIND", "gamescope")]), Session::Gamescope);
    }
}
