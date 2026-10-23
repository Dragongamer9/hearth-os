//! Keeping the screen on while a video plays (and the computer awake while music plays).
//! Linux: a logind inhibitor ("idle:sleep" for a video, "sleep" for music; Mantel's screen-off and sleep timers
//! check for these, see hosts/linux/desktop/shell/IdlePower.qml) and org.freedesktop.ScreenSaver's Inhibit (Plasma
//! and other desktops). Released the moment playback pauses or stops.

use std::sync::Mutex;

#[cfg(target_os = "linux")]
struct Held {
    _logind: Option<zbus::zvariant::OwnedFd>,
    screensaver: Option<(zbus::blocking::Connection, u32)>,
}

#[cfg(target_os = "linux")]
impl Drop for Held {
    fn drop(&mut self) {
        if let Some((c, cookie)) = self.screensaver.take() {
            let _ = c.call_method(Some("org.freedesktop.ScreenSaver"), "/org/freedesktop/ScreenSaver", Some("org.freedesktop.ScreenSaver"), "UnInhibit", &(cookie,));
        }
    }
}

#[cfg(target_os = "linux")]
static HELD: Mutex<Option<(String, Held)>> = Mutex::new(None);
#[cfg(not(target_os = "linux"))]
static HELD: Mutex<Option<(String, ())>> = Mutex::new(None);

/// "video", "audio" or "" (let the screen and the computer sleep again). Returns what's held now.
pub fn set(mode: &str) -> String {
    let mut h = HELD.lock().unwrap();
    let now = h.as_ref().map(|x| x.0.clone()).unwrap_or_default();
    if now == mode { return now; }
    *h = None;
    if mode.is_empty() { return String::new(); }
    #[cfg(target_os = "linux")]
    {
        let (what, why) = if mode == "video" { ("idle:sleep", "Playing a video") } else { ("sleep", "Playing music") };
        let logind = zbus::blocking::Connection::system().ok().and_then(|c| {
            let m = c.call_method(Some("org.freedesktop.login1"), "/org/freedesktop/login1", Some("org.freedesktop.login1.Manager"), "Inhibit", &(what, "Hearth Player", why, "block")).ok()?;
            m.body().deserialize::<zbus::zvariant::OwnedFd>().ok()
        });
        let screensaver = if mode == "video" {
            zbus::blocking::Connection::session().ok().and_then(|c| {
                let m = c.call_method(Some("org.freedesktop.ScreenSaver"), "/org/freedesktop/ScreenSaver", Some("org.freedesktop.ScreenSaver"), "Inhibit", &("Hearth Player", why)).ok()?;
                let cookie: u32 = m.body().deserialize().ok()?;
                Some((c, cookie))
            })
        } else { None };
        let ok = logind.is_some() || screensaver.is_some();
        *h = Some((mode.to_string(), Held { _logind: logind, screensaver }));
        if !ok { eprintln!("hearth-player: couldn't ask the system to stay awake"); }
    }
    #[cfg(not(target_os = "linux"))]
    { *h = Some((mode.to_string(), ())); }
    mode.to_string()
}
