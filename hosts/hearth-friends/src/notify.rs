//! Desktop Mode notifications: Mantel's notification server (desktop/shell/Notifications.qml) shows what comes in
//! through org.freedesktop.Notifications. In Game Mode Hearth shows its own toast instead (the core tells the
//! service it's watching as "gamemode", and then nothing goes to the desktop).

#[cfg(target_os = "linux")]
pub fn desktop(title: &str, body: &str) -> Result<(), String> {
    use std::collections::HashMap;
    use std::time::Duration;
    let c = dbus::blocking::Connection::new_session().map_err(|e| format!("no session bus ({e})"))?;
    let p = c.with_proxy("org.freedesktop.Notifications", "/org/freedesktop/Notifications", Duration::from_secs(3));
    let hints: dbus::arg::PropMap = HashMap::new();
    let actions: Vec<String> = vec![];
    let (_id,): (u32,) = p
        .method_call("org.freedesktop.Notifications", "Notify", ("Hearth Friends", 0u32, "system-users", title, body, actions, hints, 8000i32))
        .map_err(|e| format!("no notification server ({e})"))?;
    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub fn desktop(_title: &str, _body: &str) -> Result<(), String> { Ok(()) }
