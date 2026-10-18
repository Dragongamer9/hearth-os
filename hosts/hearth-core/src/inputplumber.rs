//! InputPlumber (Hearth OS's controller service) for Quick Access over a game, the way SteamOS-style overlays
//! do it: in "pass" mode the game gets the controller as usual, but the Xbox / Guide button is caught instead of
//! passed on; InputPlumber then sends every press to Hearth (over D-Bus) and none to the game, until Hearth hands
//! the controller back. That's what keeps the game from moving while Quick Access is open, without freezing it,
//! and makes Guide the only button that opens it (the Ally's other buttons don't).

use std::time::Duration;

const BUS: &str = "org.shadowblip.InputPlumber";
const IFACE: &str = "org.shadowblip.Input.CompositeDevice";
pub const MODE_NONE: u32 = 0;
pub const MODE_PASS: u32 = 1;
pub const MODE_ALWAYS: u32 = 2;

fn busctl(args: &[&str]) -> Result<String, String> {
    let mut a = vec!["--system"]; a.extend_from_slice(args);
    crate::system::run_timeout("busctl", &a, Duration::from_secs(5))
}

/// Is InputPlumber running on this device?
pub fn available() -> bool { crate::system::has("busctl") && busctl(&["status", BUS]).is_ok() }

/// The combined controllers InputPlumber manages (the built-in one, and paired ones): their D-Bus paths.
pub fn devices() -> Vec<String> {
    busctl(&["--list", "tree", BUS]).unwrap_or_default().lines().map(str::trim)
        .filter(|l| l.starts_with("/org/shadowblip/InputPlumber/CompositeDevice")).map(str::to_string).collect()
}

/// Pass (game gets the controller, Guide is caught), Always (everything to Hearth) or None (InputPlumber's normal).
pub fn set_mode(mode: u32) {
    for d in devices() { let _ = busctl(&["set-property", BUS, &d, IFACE, "InterceptMode", "u", &mode.to_string()]); }
}

/// Only the Xbox / Guide button starts the catch (not the Ally's other buttons).
pub fn guide_only() {
    for d in devices() { let _ = busctl(&["call", BUS, &d, IFACE, "SetInterceptActivation", "ass", "1", "Gamepad:Button:Guide", "Gamepad:Button:Guide"]); }
}

/// One line of `gdbus monitor` from InputPlumber's D-Bus device: (event, pressed).
/// "/org/shadowblip/InputPlumber/devices/target/dbus0: org.shadowblip.Input.DBusDevice.InputEvent ('ui_guide', 1.0)"
pub fn parse_event(line: &str) -> Option<(String, bool)> {
    let i = line.find("DBusDevice.InputEvent (")? + "DBusDevice.InputEvent (".len();
    let rest = &line[i..];
    let name = rest.split('\'').nth(1)?.to_string();
    let value: f64 = rest.rsplit(',').next()?.trim().trim_end_matches(')').trim().parse().ok()?;
    Some((name, value > 0.5))
}

/// InputPlumber's names for presses, as the overlay's buttons.
pub fn button(event: &str) -> Option<&'static str> {
    Some(match event {
        "ui_up" => "up", "ui_down" => "down", "ui_left" => "left", "ui_right" => "right",
        "ui_accept" => "a", "ui_back" => "b", "ui_action" => "x", "ui_context" => "y",
        "ui_l1" => "lb", "ui_r1" => "rb", "ui_select" => "select", "ui_option" => "start", "ui_guide" => "guide",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reads_inputplumber_events() {
        assert_eq!(parse_event("/org/shadowblip/InputPlumber/devices/target/dbus0: org.shadowblip.Input.DBusDevice.InputEvent ('ui_guide', 1.0)"), Some(("ui_guide".into(), true)));
        assert_eq!(parse_event("/org/shadowblip/InputPlumber/devices/target/dbus0: org.shadowblip.Input.DBusDevice.InputEvent ('ui_down', 0.0)"), Some(("ui_down".into(), false)));
        assert_eq!(parse_event("something else"), None);
        assert_eq!(button("ui_accept"), Some("a"));
        assert_eq!(button("ui_quick"), None);
    }
}
