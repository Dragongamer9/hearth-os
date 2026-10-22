//! Hearth Browser opened from Hearth: Mozilla's Firefox with Hearth's settings, through /usr/bin/hearth-browser
//! (os/system_files/usr/bin/hearth-browser; Firefox itself is in /usr/lib/hearth-browser/firefox).
//!
//! In Game Mode the controller mouse (padmouse.rs: right stick pointer, A click, B Esc, D-pad arrows, left stick
//! scroll) comes on while Hearth Browser is open, and goes off again when its last window closes, unless it was on
//! already. Typing is Hearth Browser's own on-screen keyboard (its Keyboard button, or the address bar).
//! The window comes to the front by itself: it's a new window of a program Hearth opened (gamemode.rs).

use serde_json::json;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

pub const COMMAND: &str = "/usr/bin/hearth-browser";
pub const FIREFOX_DIR: &str = "/usr/lib/hearth-browser/firefox";
/// The class its windows carry (`--class hearth-browser`), also on its command line.
const CLASS: &str = "hearth-browser";

pub fn available() -> bool { Path::new(COMMAND).exists() }

/// Firefox's version from its application.ini ("Version=150.0").
pub fn parse_version(ini: &str) -> Option<String> {
    ini.lines().find_map(|l| l.strip_prefix("Version=")).map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
}

/// The Firefox version Hearth Browser runs on, None when it's not Firefox (an older image).
pub fn version() -> Option<String> {
    std::fs::read_to_string(Path::new(FIREFOX_DIR).join("application.ini")).ok().and_then(|s| parse_version(&s))
}

/// A process's command line (NUL-separated, as in /proc/<pid>/cmdline) is Hearth Browser's Firefox.
pub fn is_browser_cmdline(cmdline: &[u8]) -> bool {
    let args: Vec<&[u8]> = cmdline.split(|b| *b == 0).collect();
    args.windows(2).any(|w| w[0] == b"--class" && w[1] == CLASS.as_bytes())
}

/// Is a Hearth Browser window (still) open?
fn running() -> bool {
    let Ok(dir) = std::fs::read_dir("/proc") else { return false };
    dir.flatten().filter(|e| e.file_name().to_string_lossy().bytes().all(|b| b.is_ascii_digit()))
        .any(|e| std::fs::read(e.path().join("cmdline")).map(|c| is_browser_cmdline(&c)).unwrap_or(false))
}

/// Opens Hearth Browser, or a page in it (an http(s) address checked by the caller).
pub fn open(url: Option<&str>) -> Result<(), String> {
    let mut c = Command::new(COMMAND);
    if let Some(u) = url { c.arg(u); }
    let child = c.current_dir(crate::paths::home()).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null())
        .spawn().map_err(|e| format!("Couldn't open Hearth Browser: {e}"))?;
    crate::bridge::log(&format!("hearth browser: opened (process {}){}", child.id(), url.map(|u| format!(" {u}")).unwrap_or_default()));
    let mouse = crate::gamescope::inside() && !crate::padmouse::is_on() && match crate::padmouse::start() {
        Ok(()) => { crate::stores::notify(json!({"type": "event", "name": "padMouse", "data": {"on": true}})); true }
        Err(e) => { crate::bridge::log(&format!("hearth browser: no controller mouse ({e})")); false }
    };
    std::thread::spawn(move || {
        let mut child = child;
        let _ = child.wait();
        if !mouse { return; }
        // It may have handed the page to a Hearth Browser that was open already: the mouse stays while one is.
        while running() { std::thread::sleep(Duration::from_secs(2)); }
        if crate::padmouse::is_on() {
            crate::padmouse::stop();
            crate::stores::notify(json!({"type": "event", "name": "padMouse", "data": {"on": false}}));
        }
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_from_application_ini() {
        let ini = "[App]\nVendor=Mozilla\nName=Firefox\nRemotingName=firefox\nVersion=150.0.1\nBuildID=20261020\n";
        assert_eq!(parse_version(ini).as_deref(), Some("150.0.1"));
        assert_eq!(parse_version("[App]\nName=Firefox\n"), None);
        assert_eq!(parse_version("Version=\n"), None);
    }

    #[test]
    fn finds_its_firefox_by_class() {
        assert!(is_browser_cmdline(b"/usr/lib/hearth-browser/firefox/firefox\0--name\0hearth-browser\0--class\0hearth-browser\0--profile\0/home/a/.local/share/nimbus/hearth-browser\0"));
        assert!(!is_browser_cmdline(b"/usr/lib/firefox/firefox\0--class\0firefox\0"));
        assert!(!is_browser_cmdline(b"/usr/bin/python3\0/usr/bin/hearth-browser\0"));
        assert!(!is_browser_cmdline(b""));
    }
}
