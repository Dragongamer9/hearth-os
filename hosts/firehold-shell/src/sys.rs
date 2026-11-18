//! What the bar, the Couch's top row and the Guide show about the machine: the clock, Wi-Fi, the battery, volume,
//! brightness, power and the GPU. Read from /sys and a few standard tools, once a minute (or when the Guide opens),
//! never in a loop. `FIREHOLD_SYS` points at another /sys (the tests' stand-in battery and Wi-Fi).

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn sys() -> PathBuf {
    std::env::var_os("FIREHOLD_SYS").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/sys"))
}
fn read(p: &Path) -> Option<String> {
    std::fs::read_to_string(p).ok().map(|s| s.trim().to_string())
}
fn num(p: &Path) -> Option<f64> {
    read(p)?.parse().ok()
}
fn dir(p: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(p).map(|d| d.flatten().map(|e| e.path()).collect()).unwrap_or_default();
    v.sort();
    v
}

/// "10:22 PM" and "Mon Nov 16".
pub fn clock() -> (String, String) {
    let now = chrono::Local::now();
    (now.format("%-I:%M %p").to_string(), now.format("%a %b %-d").to_string())
}

/// Seconds until the next minute starts (the clock's only wake-up).
pub fn secs_to_next_minute() -> u64 {
    let now = chrono::Local::now();
    60 - (now.timestamp() % 60) as u64
}

/// "Wi-Fi", "Wired" or "Offline".
pub fn network() -> String {
    let mut wired = false;
    for n in dir(&sys().join("class/net")) {
        let name = n.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        if name == "lo" {
            continue;
        }
        let up = read(&n.join("operstate")).as_deref() == Some("up");
        if !up {
            continue;
        }
        if n.join("wireless").exists() || n.join("phy80211").exists() {
            return "Wi-Fi".into();
        }
        if !name.starts_with("veth") && !name.starts_with("docker") && !name.starts_with("virbr") {
            wired = true;
        }
    }
    if wired { "Wired".into() } else { "Offline".into() }
}

/// "78%" (the first battery), or "" with none.
pub fn battery() -> String {
    for b in dir(&sys().join("class/power_supply")) {
        if read(&b.join("type")).as_deref() == Some("Battery") || b.file_name().is_some_and(|n| n.to_string_lossy().starts_with("BAT")) {
            if let Some(c) = num(&b.join("capacity")) {
                return format!("{}%", c.round() as i64);
            }
        }
    }
    String::new()
}

/// Watts the battery is giving (Power · "Balanced · 18 W"), when it says.
pub fn watts() -> Option<f64> {
    for b in dir(&sys().join("class/power_supply")) {
        if let Some(p) = num(&b.join("power_now")) {
            if p > 0.0 {
                return Some(p / 1e6);
            }
        }
        if let (Some(v), Some(c)) = (num(&b.join("voltage_now")), num(&b.join("current_now"))) {
            if v > 0.0 && c > 0.0 {
                return Some(v * c / 1e12);
            }
        }
    }
    None
}

/// Brightness in percent (the first backlight).
pub fn brightness() -> Option<i64> {
    let b = dir(&sys().join("class/backlight")).into_iter().next()?;
    let (cur, max) = (num(&b.join("brightness"))?, num(&b.join("max_brightness"))?);
    (max > 0.0).then(|| (cur * 100.0 / max).round() as i64)
}

pub fn set_brightness(pct: i64) {
    let pct = pct.clamp(5, 100);
    run_quiet("brightnessctl", &["-q", "set", &format!("{pct}%")]);
}

fn output(cmd: &str, args: &[&str]) -> Option<String> {
    let o = Command::new(cmd).args(args).stdin(Stdio::null()).stderr(Stdio::null()).output().ok()?;
    o.status.success().then(|| String::from_utf8_lossy(&o.stdout).trim().to_string())
}
pub fn run_quiet(cmd: &str, args: &[&str]) {
    let _ = Command::new(cmd).args(args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().map(|mut c| {
        std::thread::spawn(move || {
            let _ = c.wait();
        })
    });
}

/// Volume in percent (PipeWire's default sink, through wpctl).
pub fn volume() -> Option<i64> {
    let t = output("wpctl", &["get-volume", "@DEFAULT_AUDIO_SINK@"])?;
    let v: f64 = t.split_whitespace().nth(1)?.parse().ok()?;
    Some((v * 100.0).round() as i64)
}
pub fn set_volume(pct: i64) {
    run_quiet("wpctl", &["set-volume", "-l", "1.0", "@DEFAULT_AUDIO_SINK@", &format!("{}%", pct.clamp(0, 100))]);
}

/// power-profiles-daemon's profile, as the Guide says it: Power saver, Balanced, Performance.
pub fn power_profile() -> Option<String> {
    let p = output("powerprofilesctl", &["get"])?;
    Some(match p.as_str() {
        "power-saver" => "Power saver".into(),
        "balanced" => "Balanced".into(),
        "performance" => "Performance".into(),
        other => other.to_string(),
    })
}
pub fn step_power(up: bool) {
    let order = ["power-saver", "balanced", "performance"];
    let cur = output("powerprofilesctl", &["get"]).unwrap_or_else(|| "balanced".into());
    let i = order.iter().position(|p| *p == cur).unwrap_or(1) as i64;
    let n = (i + if up { 1 } else { -1 }).clamp(0, 2) as usize;
    run_quiet("powerprofilesctl", &["set", order[n]]);
}

/// The performance overlay's numbers that the system has: GPU busy %, its temperature, watts.
pub fn gpu() -> (Option<i64>, Option<i64>) {
    let mut busy = None;
    let mut temp = None;
    for card in dir(&sys().join("class/drm")) {
        let dev = card.join("device");
        if busy.is_none() {
            busy = num(&dev.join("gpu_busy_percent")).map(|v| v as i64);
        }
        if temp.is_none() {
            for h in dir(&dev.join("hwmon")) {
                if let Some(t) = num(&h.join("temp1_input")) {
                    temp = Some((t / 1000.0).round() as i64);
                    break;
                }
            }
        }
    }
    (busy, temp)
}

/// The account's initial for the round avatar: the first letter of its full name (or its login).
pub fn initial() -> String {
    let user = std::env::var("USER").unwrap_or_default();
    let uid = unsafe { libc::getuid() };
    let mut name = String::new();
    if let Ok(pw) = std::fs::read_to_string("/etc/passwd") {
        for l in pw.lines() {
            let f: Vec<&str> = l.split(':').collect();
            if f.len() > 4 && f[2].parse::<u32>().ok() == Some(uid) {
                name = f[4].split(',').next().unwrap_or("").to_string();
                if name.is_empty() {
                    name = f[0].to_string();
                }
            }
        }
    }
    if name.is_empty() {
        name = user;
    }
    name.chars().find(|c| c.is_alphanumeric()).map(|c| c.to_uppercase().collect()).unwrap_or_else(|| "?".into())
}

/// An app from the menu (Couch › Apps, the dock's launchers).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, Default)]
#[serde(default)]
pub struct App {
    pub id: String,
    pub name: String,
}

/// The menu's apps (.desktop files with a Name, not NoDisplay or Hidden), by name.
pub fn apps() -> Vec<App> {
    let mut dirs: Vec<PathBuf> = vec![crate::config::home().join(".local/share/applications")];
    let data = std::env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".into());
    dirs.extend(data.split(':').filter(|d| !d.is_empty()).map(|d| Path::new(d).join("applications")));
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for d in dirs {
        for f in dir(&d) {
            if f.extension().and_then(|e| e.to_str()) != Some("desktop") {
                continue;
            }
            let id = f.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            if !seen.insert(id.clone()) {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&f) else { continue };
            let mut in_entry = false;
            let (mut name, mut hidden, mut app) = (String::new(), false, false);
            for l in text.lines() {
                let l = l.trim();
                if l.starts_with('[') {
                    in_entry = l == "[Desktop Entry]";
                    continue;
                }
                if !in_entry {
                    continue;
                }
                if let Some(v) = l.strip_prefix("Name=") {
                    name = v.to_string();
                } else if l == "NoDisplay=true" || l == "Hidden=true" {
                    hidden = true;
                } else if l == "Type=Application" {
                    app = true;
                }
            }
            if app && !hidden && !name.is_empty() {
                out.push(App { id, name });
            }
        }
    }
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    out
}

/// Opens an app by its desktop id (gtk-launch, or KDE's kioclient, whichever is there), detached.
pub fn launch_app(id: &str) {
    if !crate::model::valid_id(id) {
        return;
    }
    for (cmd, args) in [("gtk-launch", vec![id.to_string()]), ("kioclient", vec!["exec".into(), format!("{id}.desktop")]), ("gio", vec!["launch".into(), format!("/usr/share/applications/{id}.desktop")])] {
        if which(cmd) {
            let a: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
            detach(cmd, &a);
            return;
        }
    }
}

pub fn which(cmd: &str) -> bool {
    std::env::var("PATH").unwrap_or_default().split(':').any(|d| {
        let p = Path::new(d).join(cmd);
        std::fs::metadata(&p).map(|m| m.is_file()).unwrap_or(false)
    })
}

/// Starts a program in its own session (it outlives the shell), with nothing on its output.
pub fn detach(cmd: &str, args: &[&str]) {
    use std::os::unix::process::CommandExt;
    let mut c = Command::new(cmd);
    c.args(args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    unsafe {
        c.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }
    if let Ok(mut child) = c.spawn() {
        std::thread::spawn(move || {
            let _ = child.wait();
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn battery_and_wifi_from_a_stand_in_sys() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path();
        std::fs::create_dir_all(p.join("class/power_supply/BAT0")).unwrap();
        std::fs::write(p.join("class/power_supply/BAT0/type"), "Battery\n").unwrap();
        std::fs::write(p.join("class/power_supply/BAT0/capacity"), "78\n").unwrap();
        std::fs::write(p.join("class/power_supply/BAT0/power_now"), "18000000\n").unwrap();
        std::fs::create_dir_all(p.join("class/net/wlan0/wireless")).unwrap();
        std::fs::write(p.join("class/net/wlan0/operstate"), "up\n").unwrap();
        std::fs::create_dir_all(p.join("class/backlight/x")).unwrap();
        std::fs::write(p.join("class/backlight/x/brightness"), "80").unwrap();
        std::fs::write(p.join("class/backlight/x/max_brightness"), "100").unwrap();
        std::env::set_var("FIREHOLD_SYS", p);
        assert_eq!(battery(), "78%");
        assert_eq!(network(), "Wi-Fi");
        assert_eq!(watts().map(|w| w.round() as i64), Some(18));
        assert_eq!(brightness(), Some(80));
        std::env::remove_var("FIREHOLD_SYS");
    }
}
