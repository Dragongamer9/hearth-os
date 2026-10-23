//! Nearby Share (hosts/hearth-nearby) and Hearth Connect (hosts/hearth-connect) opened from Game Mode: Settings ›
//! Bluetooth › "Phones and nearby devices" (ui/index.html, pageLinkApps). Only these two programs, full screen
//! inside gamescope, like Moonlight is opened.

use std::path::Path;

/// The program for `app` ("nearby" or "connect"); nothing else can be started this way.
pub fn program(app: &str) -> Option<&'static str> {
    match app {
        "nearby" => Some("/usr/bin/hearth-nearby"),
        "connect" => Some("/usr/bin/hearth-connect"),
        _ => None,
    }
}

pub fn open(app: &str) -> Result<(), String> {
    let bin = program(app).ok_or_else(|| format!("Not one of Hearth's apps: {app}"))?;
    if !Path::new(bin).exists() { return Err("That app isn't on this system yet: it comes with the next Hearth OS update".into()); }
    let mut c = std::process::Command::new(bin);
    if crate::gamescope::inside() { c.arg("--fullscreen"); }
    c.spawn().map(crate::procs::detach).map(|_| ()).map_err(|e| format!("Couldn't open it: {e}"))
}

#[cfg(test)]
mod tests {
    #[test]
    fn only_the_two_apps() {
        assert_eq!(super::program("nearby"), Some("/usr/bin/hearth-nearby"));
        assert_eq!(super::program("connect"), Some("/usr/bin/hearth-connect"));
        assert_eq!(super::program("/bin/sh"), None);
        assert!(super::open("../../bin/sh").is_err());
    }
}
