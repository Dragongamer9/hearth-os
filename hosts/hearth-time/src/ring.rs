//! Getting a ringing alarm in front of you, from the background service.
//!
//! The service starts when you sign in, before either mode's display exists, and Game Mode and Desktop Mode
//! replace each other without signing out. Each mode tells systemd its display when it starts (labwc's autostart
//! and nimbus-watchdog: dbus-update-activation-environment), so at ring time the service asks systemd for the
//! current one (`session_env`) and opens the ringing window there:
//!   Desktop Mode  WAYLAND_DISPLAY is set: the screen is switched on (wlopm) and the window opens over everything.
//!   Game Mode     WAYLAND_DISPLAY is empty and DISPLAY is gamescope's: the window opens on gamescope's X display
//!                 and pins itself in front of the game the way Hearth shows a Steam question or an app picked in
//!                 Quick Access (GAMESCOPECTRL_BASELAYER_WINDOW, hosts/hearth-core/src/gamescope.rs); when it closes
//!                 the pin is taken away again and gamescope goes back to what Hearth asked for.

use std::process::{Command, Stdio};
use std::time::Duration;

/// systemd's user environment now (`systemctl --user show-environment`), as (name, value) pairs.
pub fn session_env() -> Vec<(String, String)> {
    let out = Command::new("systemctl").args(["--user", "show-environment"]).stderr(Stdio::null()).output();
    let text = out.map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default();
    parse_env(&text)
}

/// KEY=value lines; systemd writes values with special characters as $'...' (C-style escapes).
pub fn parse_env(text: &str) -> Vec<(String, String)> {
    text.lines().filter_map(|l| {
        let (k, v) = l.split_once('=')?;
        if k.is_empty() || !k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') { return None; }
        let v = if let Some(inner) = v.strip_prefix("$'").and_then(|x| x.strip_suffix('\'')) {
            let mut o = String::new();
            let mut it = inner.chars();
            while let Some(c) = it.next() {
                if c == '\\' { match it.next() { Some('n') => o.push('\n'), Some('t') => o.push('\t'), Some(x) => o.push(x), None => {} } } else { o.push(c); }
            }
            o
        } else { v.to_string() };
        Some((k.to_string(), v))
    }).collect()
}

pub fn get<'a>(env: &'a [(String, String)], k: &str) -> Option<&'a str> {
    env.iter().rev().find(|(n, _)| n == k).map(|(_, v)| v.as_str()).filter(|v| !v.is_empty())
}

/// Which mode the session is in by its environment: "desktop", "game", or "none" (no display: signed out, or
/// started outside Hearth's session).
pub fn mode(env: &[(String, String)]) -> &'static str {
    if get(env, "WAYLAND_DISPLAY").is_some() { "desktop" } else if get(env, "DISPLAY").is_some() { "game" } else { "none" }
}

/// Switches the screen on (Desktop Mode turns it off when idle: Settings › Power).
pub fn wake_screen(env: &[(String, String)]) {
    if mode(env) == "desktop" && crate::alert::which("wlopm").is_some() {
        let mut c = Command::new("wlopm");
        c.args(["--on", "*"]).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
        apply_env(&mut c, env, false);
        let _ = c.status();
    }
    if mode(env) == "game" && crate::alert::which("xset").is_some() {
        let mut c = Command::new("xset");
        c.args(["dpms", "force", "on"]).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
        apply_env(&mut c, env, true);
        let _ = c.status();
    }
}

/// Gives a program the session's display. `x11`: gamescope's X display only (GTK would otherwise look for Wayland).
pub fn apply_env(c: &mut Command, env: &[(String, String)], x11: bool) {
    for k in ["DISPLAY", "WAYLAND_DISPLAY", "XAUTHORITY", "XDG_CURRENT_DESKTOP", "XDG_SESSION_TYPE", "NIMBUS_SHARE", "DBUS_SESSION_BUS_ADDRESS"] {
        match get(env, k) { Some(v) => { c.env(k, v); } None => { c.env_remove(k); } }
    }
    if x11 { c.env_remove("WAYLAND_DISPLAY"); c.env("GDK_BACKEND", "x11"); }
}

// ---------------------------------------------------------------------------------------------- Game Mode

const PIN: &str = "GAMESCOPECTRL_BASELAYER_WINDOW";

fn xprop(args: &[&str]) -> String {
    Command::new("xprop").args(args).stderr(Stdio::null()).output().map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default()
}

/// This process's X window (the ringing window), found by its process id.
fn own_window() -> Option<u64> {
    let pid = std::process::id().to_string();
    let out = Command::new("xdotool").args(["search", "--onlyvisible", "--pid", &pid]).stderr(Stdio::null()).output().ok()?;
    String::from_utf8_lossy(&out.stdout).lines().filter_map(|l| l.trim().parse::<u64>().ok()).last()
}

/// The window pinned in front now (0: none).
fn pinned() -> u64 {
    let out = xprop(&["-root", PIN]);
    out.split_once('=').and_then(|(_, v)| v.split(',').next()).and_then(|v| v.trim().parse().ok()).unwrap_or(0)
}

/// In Game Mode: puts the ringing window in front of the game (tries for a few seconds while the window maps).
/// Returns the window pinned.
pub fn pin_in_front() -> Option<u64> {
    for _ in 0..20 {
        if let Some(w) = own_window() {
            let v = w.to_string();
            let _ = Command::new("xprop").args(["-root", "-f", PIN, "32c", "-set", PIN, &v]).status();
            let _ = Command::new("xdotool").args(["windowactivate", &v]).stderr(Stdio::null()).status();
            return Some(w);
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    None
}

/// Takes the pin away again, if it's still ours (Hearth may have shown something else meanwhile).
pub fn unpin(w: u64) {
    if w != 0 && pinned() == w { let _ = Command::new("xprop").args(["-root", "-remove", PIN]).status(); }
}

#[cfg(test)]
mod tests {
    #[test]
    fn env() {
        let e = super::parse_env("DISPLAY=:1\nWAYLAND_DISPLAY=\nPATH=/usr/bin\nX=$'a\\nb'\nbad line\n");
        assert_eq!(super::get(&e, "DISPLAY"), Some(":1"));
        assert_eq!(super::get(&e, "WAYLAND_DISPLAY"), None);
        assert_eq!(super::get(&e, "X"), Some("a\nb"));
        assert_eq!(super::mode(&e), "game");
        assert_eq!(super::mode(&super::parse_env("WAYLAND_DISPLAY=wayland-0\nDISPLAY=:0")), "desktop");
        assert_eq!(super::mode(&[]), "none");
    }
}
