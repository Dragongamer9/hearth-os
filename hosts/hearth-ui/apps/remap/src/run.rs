//! Hearth Remap's runner (`hearth-remap-app --run`, the user service hearth-remap.service): turns each device's
//! profile on by itself when its game or app comes to the front and off when you leave it (Everywhere when nothing
//! else fits), says so on screen when you asked it to, and lets you hold ≡ for 3 seconds to turn every remap off.
//! It never writes your settings: what's on right now is in $XDG_RUNTIME_DIR/hearth-remap-now.json.
//! Light: it looks once a second; ≡ is read from the controller's own events (a joystick, which the person at the PC
//! may read) on a thread that sleeps until the controller sends something.

use crate::ir;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub fn now_file() -> std::path::PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR").map(std::path::PathBuf::from).unwrap_or_else(std::env::temp_dir).join("hearth-remap-now.json")
}
pub fn now_read() -> Value {
    std::fs::read(now_file()).ok().and_then(|b| serde_json::from_slice(&b).ok()).filter(Value::is_object).unwrap_or_else(|| json!({}))
}
pub fn now_write(v: &Value) {
    let f = now_file();
    let tmp = f.with_extension("tmp");
    if std::fs::write(&tmp, v.to_string()).is_ok() {
        let _ = std::fs::rename(&tmp, &f);
    }
}

/// What's in front: the game Steam is running (its app id), the window in front's app (Mantel's own word for it: `hearth-mantel-ipc call pad state`, its "front"), and
/// whether this is the desktop.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Front {
    pub game: String,
    pub app: String,
    pub desktop: bool,
}

pub fn front() -> Front {
    if let Ok(v) = std::env::var("HEARTH_REMAP_FRONT") {
        let (g, a) = v.split_once(',').unwrap_or((v.as_str(), ""));
        return Front { game: g.into(), app: a.into(), desktop: true };
    }
    let home = hearth_ui::paths::home();
    let reg = std::fs::read_to_string(home.join(".steam/registry.vdf")).unwrap_or_default();
    let game = reg
        .lines()
        .find(|l| l.contains("\"RunningAppID\""))
        .and_then(|l| l.split('"').filter(|x| !x.trim().is_empty()).nth(1).map(str::to_string))
        .filter(|x| x != "0")
        .unwrap_or_default();
    let app = std::process::Command::new("hearth-mantel-ipc")
        .args(["call", "pad", "state"])
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()
        .map(|o| front_of(&String::from_utf8_lossy(&o.stdout)))
        .unwrap_or_default();
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").map(|d| !d.to_lowercase().contains("gamescope")).unwrap_or(false);
    Front { game, app, desktop }
}

/// The app in front from Mantel's pad state ({"front":"<app id>",...}), lower case; "" when Mantel isn't there.
pub fn front_of(state: &str) -> String {
    let key = "\"front\":\"";
    state
        .find(key)
        .map(|i| &state[i + key.len()..])
        .and_then(|r| r.find('"').map(|j| r[..j].to_lowercase()))
        .unwrap_or_default()
}

/// Does a profile's "turns on when" fit what's in front? (Everywhere fits always; it's only used when nothing else does.)
pub fn fits(when: &str, f: &Front) -> bool {
    match when.split_once(':') {
        Some(("game", id)) => !id.is_empty() && f.game == id,
        Some(("app", class)) => !class.is_empty() && f.app.contains(&class.to_lowercase()),
        _ => when == "desktop" && f.desktop && f.game.is_empty(),
    }
}

/// The profile each device should have now: the one that fits, else Everywhere, else none.
pub fn pick(settings: &Value, device: &str, presets: &[String], f: &Front) -> Option<String> {
    let when = |p: &str| settings.pointer(&format!("/profiles/{}/when", pointer_key(&format!("{device}/{p}")))).and_then(Value::as_str).unwrap_or(if p == "Everywhere" { "everywhere" } else { "" }).to_string();
    // a game first, then an app, then Desktop Mode, then Everywhere
    for kind in ["game:", "app:", "desktop"] {
        if let Some(p) = presets.iter().find(|p| {
            let w = when(p);
            w.starts_with(kind) && fits(&w, f)
        }) {
            return Some(p.clone());
        }
    }
    presets.iter().find(|p| when(p) == "everywhere").cloned()
}

/// A JSON pointer's key ("/" and "~" escaped).
pub fn pointer_key(s: &str) -> String {
    s.replace('~', "~0").replace('/', "~1")
}

fn notify(words: &str) {
    let _ = std::process::Command::new("notify-send").args(["-a", "Hearth Remap", "-t", "2500", "Hearth Remap", words]).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).status();
}

/// Reads one controller's events: ≡ (BTN_START) down and up.
fn watch_menu(path: String, held: Arc<Mutex<Option<Instant>>>) {
    use std::io::Read;
    let Ok(mut f) = std::fs::File::open(&path) else { return };
    let mut ev = [0u8; 24];
    while f.read_exact(&mut ev).is_ok() {
        let ty = u16::from_le_bytes([ev[16], ev[17]]);
        let code = u16::from_le_bytes([ev[18], ev[19]]);
        let val = i32::from_le_bytes([ev[20], ev[21], ev[22], ev[23]]);
        if ty == 1 && code == 315 {
            if let Ok(mut h) = held.lock() {
                *h = if val != 0 { Some(Instant::now()) } else { None };
            }
        }
    }
}

pub fn run_forever() -> i32 {
    let path = crate::kit::settings_path(ir::APP);
    let mut seen = None;
    let mut settings = crate::kit::read_settings(ir::APP);
    let held: Arc<Mutex<Option<Instant>>> = Arc::new(Mutex::new(None));
    let mut watched: Vec<String> = Vec::new();
    let mut last_scan = Instant::now() - Duration::from_secs(60);
    let mut devs: Vec<ir::Dev> = Vec::new();
    loop {
        let m = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
        if m != seen {
            seen = m;
            settings = crate::kit::read_settings(ir::APP);
        }
        if last_scan.elapsed() > Duration::from_secs(10) {
            devs = ir::devices();
            last_scan = Instant::now();
            // ≡ on every controller (and on input-remapper's forwarded copy while a profile is on)
            if let Ok(rd) = std::fs::read_dir("/dev/input/by-id") {
                for e in rd.flatten() {
                    let n = e.file_name().to_string_lossy().to_string();
                    if !n.ends_with("-event-joystick") {
                        continue;
                    }
                    let p = e.path().to_string_lossy().to_string();
                    if watched.contains(&p) {
                        continue;
                    }
                    watched.push(p.clone());
                    let h = held.clone();
                    let _ = std::thread::Builder::new().name("hearth-remap-menu".into()).stack_size(64 * 1024).spawn(move || watch_menu(p, h));
                }
            }
        }
        let mut now = now_read();
        // Hold ≡: on unless every profile that's on says no (each profile has its own switch)
        let ons: Vec<(String, String)> = now.get("on").and_then(Value::as_object).map(|o| o.iter().filter_map(|(d, p)| p.as_str().map(|p| (d.clone(), p.to_string()))).collect()).unwrap_or_default();
        let holdoff = ons.is_empty() || ons.iter().any(|(d, p)| settings.pointer(&format!("/profiles/{}/holdOff", pointer_key(&format!("{d}/{p}")))).and_then(Value::as_bool).unwrap_or(true));
        let long = held.lock().ok().and_then(|h| *h).map(|t| t.elapsed() >= Duration::from_secs(3)).unwrap_or(false);
        if holdoff && long {
            if let Ok(mut h) = held.lock() {
                *h = None;
            }
            let off = !now.get("heldOff").and_then(Value::as_bool).unwrap_or(false);
            now["heldOff"] = json!(off);
            if off {
                let _ = ir::stop_all();
                now["on"] = json!({});
                notify("Every remap is off (≡ held). Hold ≡ for 3 seconds again to turn them back on.");
            } else {
                notify("Your remaps are back on.");
            }
            now_write(&now);
        }
        let paused = settings.get("paused").and_then(Value::as_bool).unwrap_or(false) || now.get("heldOff").and_then(Value::as_bool).unwrap_or(false);
        if !paused {
            let f = front();
            let mut changed = false;
            for d in &devs {
                let presets = ir::presets(&d.name);
                if presets.is_empty() {
                    continue;
                }
                let test = now.pointer("/test/device").and_then(Value::as_str) == Some(d.name.as_str());
                let want = if test { now.pointer("/test/preset").and_then(Value::as_str).map(str::to_string) } else { pick(&settings, &d.name, &presets, &f) };
                let have = now.pointer(&format!("/on/{}", pointer_key(&d.name))).and_then(Value::as_str).map(str::to_string);
                if want == have {
                    continue;
                }
                let ok = match &want {
                    Some(p) => ir::start(&d.name, p).is_ok(),
                    None => ir::stop(&d.name).is_ok(),
                };
                if ok {
                    if !now.get("on").map(Value::is_object).unwrap_or(false) {
                        now["on"] = json!({});
                    }
                    now["on"][&d.name] = match &want {
                        Some(p) => json!(p),
                        None => Value::Null,
                    };
                    changed = true;
                    let show = want
                        .as_ref()
                        .and_then(|p| settings.pointer(&format!("/profiles/{}/show", pointer_key(&format!("{}/{p}", d.name)))).and_then(Value::as_bool))
                        .unwrap_or(true);
                    if show {
                        if let Some(p) = &want {
                            notify(&format!("{p}: its remaps are on for {}", d.name));
                        }
                    }
                }
            }
            if changed {
                now_write(&now);
            }
        }
        std::thread::sleep(Duration::from_millis(1000));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_profile_that_fits_wins_and_everywhere_is_the_rest() {
        let s = json!({"profiles": {"Pad/BFME II": {"when": "game:2450"}, "Pad/Files": {"when": "app:hearth-files"}, "Pad/Desktop": {"when": "desktop"}}});
        let ps: Vec<String> = ["BFME II", "Desktop", "Everywhere", "Files"].iter().map(|x| x.to_string()).collect();
        let f = |g: &str, a: &str, d: bool| Front { game: g.into(), app: a.into(), desktop: d };
        assert_eq!(pick(&s, "Pad", &ps, &f("2450", "", true)).as_deref(), Some("BFME II"));
        assert_eq!(pick(&s, "Pad", &ps, &f("", "hearth-files", true)).as_deref(), Some("Files"));
        assert_eq!(pick(&s, "Pad", &ps, &f("", "firefox", true)).as_deref(), Some("Desktop"));
        assert_eq!(pick(&s, "Pad", &ps, &f("", "", false)).as_deref(), Some("Everywhere"));
    }
}
