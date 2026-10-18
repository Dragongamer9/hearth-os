//! The system, through the standard tools a SteamOS-like system ships (port of hosts/linux/src/system.js):
//! Wi-Fi with nmcli, Bluetooth with bluetoothctl, radios with rfkill, sound with wpctl, brightness with
//! brightnessctl, battery from /sys/class/power_supply. A missing tool reads as "not available", never a crash.

use serde_json::{json, Value};
use std::path::Path;
use std::process::Command;
use std::time::Duration;

/// Runs a program and returns what it printed; Err with its last line when it fails or isn't there.
pub fn run(cmd: &str, args: &[&str]) -> Result<String, String> {
    run_timeout(cmd, args, Duration::from_secs(15))
}

pub fn run_timeout(cmd: &str, args: &[&str], limit: Duration) -> Result<String, String> {
    let mut child = Command::new(cmd).args(args).env("LC_ALL", "C")
        .stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).spawn()
        .map_err(|e| if e.kind() == std::io::ErrorKind::NotFound { format!("{cmd} is not installed") } else { e.to_string() })?;
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if start.elapsed() > limit => { let _ = child.kill(); return Err(format!("{cmd} took too long")); }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(e) => return Err(e.to_string()),
        }
    }
    let out = child.wait_with_output().map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    if out.status.success() { return Ok(text); }
    let err = String::from_utf8_lossy(&out.stderr).into_owned();
    let last = err.lines().chain(text.lines()).filter(|l| !l.trim().is_empty()).last().unwrap_or("failed").trim();
    Err(last.trim_start_matches("Error: ").to_string())
}

pub fn has(cmd: &str) -> bool {
    std::env::var_os("PATH").map(|p| std::env::split_paths(&p).any(|d| d.join(cmd).is_file())).unwrap_or(false)
}

/// One line of `nmcli -t`: ':'-separated, with '\:' and '\\' escaped.
pub fn nm_fields(line: &str) -> Vec<String> {
    let mut out = vec![String::new()];
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => { if let Some(n) = chars.next() { out.last_mut().unwrap().push(n); } }
            ':' => out.push(String::new()),
            _ => out.last_mut().unwrap().push(c),
        }
    }
    out
}
fn nm_lines(text: &str) -> Vec<Vec<String>> { text.lines().filter(|l| !l.is_empty()).map(nm_fields).collect() }
pub fn bars(signal: i64) -> u8 { if signal >= 75 { 4 } else if signal >= 50 { 3 } else if signal >= 25 { 2 } else if signal > 0 { 1 } else { 0 } }

// ------------------------------------------------------------------ Wi-Fi

pub mod wifi {
    use super::*;
    pub fn device() -> Option<(String, String)> {
        let rows = nm_lines(&run("nmcli", &["-t", "-f", "DEVICE,TYPE,STATE", "device"]).ok()?);
        rows.into_iter().find(|r| r.get(1).map(|t| t == "wifi").unwrap_or(false)).map(|r| (r[0].clone(), r.get(2).cloned().unwrap_or_default()))
    }
    /// Some(on) when there's a Wi-Fi card, None when there isn't (or no NetworkManager).
    pub fn radio_on() -> Option<bool> {
        if !has("nmcli") { return None; }
        device()?;
        Some(run("nmcli", &["radio", "wifi"]).ok()?.trim() == "enabled")
    }
    pub fn set_radio(on: bool) -> Result<(), String> { run("nmcli", &["radio", "wifi", if on { "on" } else { "off" }]).map(|_| ()) }
    pub fn current() -> (Option<String>, u8) {
        let rows = nm_lines(&run("nmcli", &["-t", "-f", "ACTIVE,SSID,SIGNAL", "device", "wifi", "list", "--rescan", "no"]).unwrap_or_default());
        rows.into_iter().find(|r| r[0] == "yes").map(|r| (r.get(1).filter(|s| !s.is_empty()).cloned(), bars(r.get(2).and_then(|s| s.parse().ok()).unwrap_or(0)))).unwrap_or((None, 0))
    }
    pub fn saved() -> Vec<String> {
        nm_lines(&run("nmcli", &["-t", "-f", "NAME,TYPE", "connection", "show"]).unwrap_or_default()).into_iter()
            .filter(|r| r.get(1).map(|t| t.contains("wireless")).unwrap_or(false)).map(|r| r[0].clone()).collect()
    }
    /// Networks around, best first: the connected one, then saved ones, then by signal. One entry per name.
    pub fn parse_scan(text: &str, saved: &[String]) -> Vec<Value> {
        let mut best: Vec<(String, bool, bool, u8, bool)> = Vec::new();   // ssid, connected, saved, bars, secure
        for r in nm_lines(text) {
            let (in_use, ssid, signal, sec) = (r.first().cloned().unwrap_or_default(), r.get(1).cloned().unwrap_or_default(), r.get(2).and_then(|s| s.parse().ok()).unwrap_or(0), r.get(3).cloned().unwrap_or_default());
            if ssid.is_empty() { continue; }
            let n = (ssid.clone(), in_use == "*", saved.contains(&ssid), bars(signal), !sec.is_empty() && sec != "--");
            match best.iter_mut().find(|b| b.0 == ssid) {
                Some(p) => if n.1 || (!p.1 && n.3 > p.3) { *p = n },
                None => best.push(n),
            }
        }
        best.sort_by(|a, b| b.1.cmp(&a.1).then(b.2.cmp(&a.2)).then(b.3.cmp(&a.3)));
        best.into_iter().map(|(ssid, connected, saved, bars, secure)| json!({"ssid": ssid, "bars": bars, "secure": secure, "saved": saved, "connected": connected})).collect()
    }
    pub fn scan() -> Result<Value, String> {
        if radio_on() != Some(true) { return Err("Wi-Fi is off".into()); }
        let text = run_timeout("nmcli", &["-t", "-f", "IN-USE,SSID,SIGNAL,SECURITY", "device", "wifi", "list", "--rescan", "yes"], Duration::from_secs(30))?;
        Ok(Value::Array(parse_scan(&text, &saved())))
    }
    pub fn connect(ssid: &str, password: Option<&str>) -> Value {
        let r = if saved().iter().any(|s| s == ssid) && password.is_none() {
            run_timeout("nmcli", &["connection", "up", "id", ssid], Duration::from_secs(45))
        } else {
            let mut args = vec!["device", "wifi", "connect", ssid];
            if let Some(p) = password { args.push("password"); args.push(p); }
            run_timeout("nmcli", &args, Duration::from_secs(45))
        };
        match r {
            Ok(_) => json!({"ok": true}),
            Err(e) => {
                let low = e.to_lowercase();
                let msg = if ["secrets were required", "802-1x", "psk", "password"].iter().any(|k| low.contains(k)) { "The password is incorrect".to_string() } else { e };
                json!({"ok": false, "error": msg})
            }
        }
    }
    pub fn disconnect() -> Result<(), String> { if let Some((d, _)) = device() { run("nmcli", &["device", "disconnect", &d])?; } Ok(()) }
    pub fn forget(ssid: &str) -> Result<(), String> { run("nmcli", &["connection", "delete", "id", ssid]).map(|_| ()) }
}

// ------------------------------------------------------------------ Bluetooth, radios

pub mod bt {
    use super::*;
    pub fn powered() -> Option<bool> {
        if !has("bluetoothctl") { return None; }
        let text = run("bluetoothctl", &["show"]).unwrap_or_default();
        if !text.contains("Controller") { return None; }
        Some(text.contains("Powered: yes"))
    }
    pub fn set_power(on: bool) -> Result<(), String> {
        if on && has("rfkill") { let _ = run("rfkill", &["unblock", "bluetooth"]); }
        run("bluetoothctl", &["power", if on { "on" } else { "off" }]).map(|_| ())
    }
}

pub mod bt_devices {
    use super::*;
    pub fn kind(icon: &str) -> &'static str {
        match icon { "input-gaming" => "Input.Gamepad", "input-keyboard" => "Input.Keyboard", "input-mouse" => "Input.Mouse",
            "audio-headset" | "audio-headphones" => "Audio.Headphone", "audio-card" => "Audio.Speaker", "phone" => "Phone",
            "computer" => "Computer", "video-display" => "Display.Television", _ => "Other" }
    }
    pub fn parse_info(mac: &str, text: &str) -> Value {
        let val = |k: &str| text.lines().find_map(|l| l.trim().strip_prefix(&format!("{k}: ")).map(|v| v.trim().to_string())).unwrap_or_default();
        let name = [val("Alias"), val("Name")].into_iter().find(|s| !s.is_empty()).unwrap_or_else(|| mac.to_string());
        json!({"id": mac, "name": name, "kind": kind(&val("Icon")), "connected": val("Connected") == "yes", "paired": val("Paired") == "yes"})
    }
    pub fn devices(filter: Option<&str>) -> Vec<String> {
        let mut args = vec!["devices"]; if let Some(f) = filter { args.push(f); }
        run("bluetoothctl", &args).unwrap_or_default().lines().filter_map(|l| l.strip_prefix("Device ")).map(|l| l.chars().take(17).collect()).collect()
    }
    pub fn info(mac: &str) -> Option<Value> { run("bluetoothctl", &["info", mac]).ok().map(|t| parse_info(mac, &t)) }
    pub fn paired() -> Value { Value::Array(devices(Some("Paired")).iter().filter_map(|m| info(m)).collect()) }
    pub fn pair(mac: &str) -> Value {
        match run_timeout("bluetoothctl", &["pair", mac], Duration::from_secs(40)) {
            Ok(_) => { let _ = run("bluetoothctl", &["trust", mac]); let _ = run_timeout("bluetoothctl", &["connect", mac], Duration::from_secs(20)); json!({"ok": true}) }
            Err(e) => json!({"ok": false, "error": e}),
        }
    }
    pub fn unpair(mac: &str) -> Value { match run("bluetoothctl", &["remove", mac]) { Ok(_) => json!({"ok": true}), Err(e) => json!({"ok": false, "error": e}) } }
}

/// Bluetooth scanning (Settings › Bluetooth) and "Connect a controller" (pairs controllers by itself).
/// Both send events through `emit`; starting one again replaces the running one.
pub mod bt_scan {
    use super::*;
    use std::collections::{HashMap, HashSet};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;

    pub type Emit = Arc<dyn Fn(Value) + Send + Sync>;
    static SCAN: AtomicU64 = AtomicU64::new(0);
    static AUTO: AtomicU64 = AtomicU64::new(0);
    fn ev(name: &str, data: Value) -> Value { json!({"type": "event", "name": name, "data": data}) }

    pub fn is_gamepad(d: &Value) -> bool {
        let name = d["name"].as_str().unwrap_or("").to_lowercase();
        d["kind"] == "Input.Gamepad" || ["controller", "gamepad", "xbox", "dualsense", "dualshock", "pro controller", "joy-con", "joycon", "8bitdo", "stadia", "gamesir", "flydigi", "nimbus", "razer kishi", "backbone"].iter().any(|k| name.contains(k))
    }

    pub fn start(seconds: u64, emit: Emit) {
        let me = SCAN.fetch_add(1, Ordering::SeqCst) + 1;
        std::thread::spawn(move || {
            let mut seen: HashSet<String> = bt_devices::devices(None).into_iter().collect();
            let scanner = std::thread::spawn(move || { let _ = run_timeout("bluetoothctl", &["--timeout", &seconds.to_string(), "scan", "on"], Duration::from_secs(seconds + 5)); });
            let until = std::time::Instant::now() + Duration::from_secs(seconds);
            while std::time::Instant::now() < until && SCAN.load(Ordering::SeqCst) == me {
                std::thread::sleep(Duration::from_millis(1500));
                for mac in bt_devices::devices(None) {
                    if !seen.insert(mac.clone()) { continue; }
                    if let Some(d) = bt_devices::info(&mac) { if d["paired"] != true && d["name"] != mac.as_str() { emit(ev("bt.found", d)); } }
                }
            }
            let _ = scanner.join();
            if SCAN.load(Ordering::SeqCst) == me { emit(ev("bt.scanDone", json!({}))); }
        });
    }
    pub fn stop() { SCAN.fetch_add(1, Ordering::SeqCst); let _ = run("bluetoothctl", &["scan", "off"]); }

    pub fn auto_pads(on: bool, emit: Emit) {
        let me = AUTO.fetch_add(1, Ordering::SeqCst) + 1;
        if !on { let _ = run("bluetoothctl", &["scan", "off"]); return; }
        if !has("bluetoothctl") { return; }
        std::thread::spawn(move || {
            let _ = bt::set_power(true);
            let alive = move || AUTO.load(Ordering::SeqCst) == me;
            std::thread::spawn(move || while alive() { let _ = run_timeout("bluetoothctl", &["--timeout", "120", "scan", "on"], Duration::from_secs(125)); });
            let mut tried: HashMap<String, std::time::Instant> = HashMap::new();
            while AUTO.load(Ordering::SeqCst) == me {
                std::thread::sleep(Duration::from_secs(2));
                for mac in bt_devices::devices(None) {
                    if tried.get(&mac).map(|t| t.elapsed() < Duration::from_secs(20)).unwrap_or(false) { continue; }
                    let Some(d) = bt_devices::info(&mac) else { continue };
                    if d["paired"] == true || !is_gamepad(&d) { continue; }
                    tried.insert(mac.clone(), std::time::Instant::now());
                    emit(ev("bt.pad", json!({"state": "pairing", "name": d["name"]})));
                    let r = bt_devices::pair(&mac);
                    emit(ev("bt.pad", json!({"state": if r["ok"] == true { "paired" } else { "failed" }, "name": d["name"]})));
                    break;
                }
            }
        });
    }
}

pub fn airplane_on() -> bool {
    if !has("rfkill") { return false; }
    let rows: Vec<String> = run("rfkill", &["-n", "-o", "SOFT"]).unwrap_or_default().lines().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
    !rows.is_empty() && rows.iter().all(|s| s == "blocked")
}
pub fn set_airplane(on: bool) -> Result<(), String> {
    if has("rfkill") { return run("rfkill", &[if on { "block" } else { "unblock" }, "all"]).map(|_| ()); }
    let _ = wifi::set_radio(!on);
    let _ = bt::set_power(!on);
    Ok(())
}

// ------------------------------------------------------------------ sound, brightness, battery

pub mod audio {
    use super::*;
    pub fn parse_volume(out: &str) -> (u32, bool) {
        let v = out.split_whitespace().skip_while(|w| *w != "Volume:").nth(1).and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
        ((v * 100.0).round() as u32, out.contains("MUTED"))
    }
    pub fn volume() -> Option<(u32, bool)> {
        if !has("wpctl") { return None; }
        run("wpctl", &["get-volume", "@DEFAULT_AUDIO_SINK@"]).ok().map(|o| parse_volume(&o))
    }
    pub fn set_volume(v: u32, max: u32) -> Result<(), String> {
        let v = v.min(max) as f64 / 100.0;
        run("wpctl", &["set-volume", "@DEFAULT_AUDIO_SINK@", &format!("{v:.2}")]).map(|_| ())
    }
    /// Output devices from `wpctl status` (the Sinks list under Audio); the default one is marked with '*'.
    pub fn parse_devices(text: &str) -> Vec<Value> {
        let (mut in_audio, mut in_sinks) = (false, false);
        let mut out = Vec::new();
        for raw in text.lines() {
            let line: String = raw.chars().map(|c| if "│├└─".contains(c) { ' ' } else { c }).collect();
            if line.starts_with("Audio") { in_audio = true; continue; }
            if line.starts_with("Video") || line.starts_with("Settings") { in_audio = false; }
            if !in_audio { continue; }
            if line.contains("Sinks:") { in_sinks = true; continue; }
            if ["Sources:", "Filters:", "Streams:", "Sink endpoints:", "Devices:"].iter().any(|k| line.contains(k)) { in_sinks = false; continue; }
            if !in_sinks { continue; }
            let t = line.trim();
            let is_default = t.starts_with('*');
            let t = t.trim_start_matches('*').trim();
            if let Some((id, rest)) = t.split_once(". ") {
                if id.chars().all(|c| c.is_ascii_digit()) && !id.is_empty() {
                    let name = rest.split(" [vol:").next().unwrap_or(rest).trim();
                    out.push(json!({"id": id, "name": name, "isDefault": is_default}));
                }
            }
        }
        out
    }
    pub fn get(max: u32) -> Value {
        let (v, m) = volume().unwrap_or((0, false));
        json!({"volume": v, "muted": m, "max": max, "devices": parse_devices(&run("wpctl", &["status"]).unwrap_or_default())})
    }
    pub fn set_default(id: &str) -> Result<(), String> { run("wpctl", &["set-default", id]).map(|_| ()) }
    pub fn set_mute(m: bool) -> Result<(), String> { run("wpctl", &["set-mute", "@DEFAULT_AUDIO_SINK@", if m { "1" } else { "0" }]).map(|_| ()) }
}

pub fn brightness() -> Option<u32> {
    if !has("brightnessctl") { return None; }
    let out = run("brightnessctl", &["-m", "-c", "backlight", "info"]).ok()?;
    out.split(',').find(|f| f.ends_with('%')).and_then(|f| f.trim_end_matches('%').parse().ok())
}
pub fn set_brightness(v: u32) -> Result<(), String> { run("brightnessctl", &["-c", "backlight", "set", &format!("{}%", v.clamp(1, 100))]).map(|_| ()) }

/// The device's own battery (not a controller's or a mouse's).
pub fn battery(root: &Path) -> Value {
    let read = |p: std::path::PathBuf| std::fs::read_to_string(p).map(|s| s.trim().to_string()).unwrap_or_default();
    if let Ok(rd) = std::fs::read_dir(root) {
        let mut names: Vec<_> = rd.flatten().map(|e| e.path()).collect();
        names.sort();
        for dir in names {
            if read(dir.join("type")) != "Battery" || read(dir.join("scope")) == "Device" { continue; }
            let Ok(pct) = read(dir.join("capacity")).parse::<u32>() else { continue };
            let status = read(dir.join("status"));
            return json!({"present": true, "pct": pct, "charging": status == "Charging" || status == "Full"});
        }
    }
    json!({"present": false, "pct": 0, "charging": false})
}

/// What the top bar and Quick Access show (the Electron host's Bridge.status()).
pub fn status(max_volume: u32) -> Value {
    let wifi_on = wifi::radio_on();
    let (ssid, wbars) = if wifi_on == Some(true) { wifi::current() } else { (None, 0) };
    let bt_on = bt::powered();
    let b = brightness();
    json!({
        "wifi": {"available": wifi_on.is_some(), "on": wifi_on == Some(true), "ssid": ssid, "bars": wbars},
        "bt": {"available": bt_on.is_some(), "on": bt_on == Some(true)},
        "airplane": airplane_on(),
        "battery": battery(Path::new("/sys/class/power_supply")),
        "audio": audio::volume().map(|(v, m)| json!({"volume": v, "muted": m, "max": max_volume})),
        "display": {"supported": b.is_some(), "brightness": b.unwrap_or(0)},
        "powerMode": "balanced",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reads_nmcli() {
        assert_eq!(nm_fields(r"yes:My\:Net:80"), vec!["yes", "My:Net", "80"]);
        let saved = vec!["Home".to_string()];
        let list = wifi::parse_scan("*:Cafe:40:WPA2\n:Home:30:WPA2\n:Home:90:WPA2\n::70:\n:Open:95:--\n", &saved);
        let names: Vec<&str> = list.iter().map(|n| n["ssid"].as_str().unwrap()).collect();
        assert_eq!(names, vec!["Cafe", "Home", "Open"]);
        assert_eq!(list[1]["bars"], 4);
        assert_eq!(list[2]["secure"], false);
        assert_eq!(list[0]["connected"], true);
    }
    #[test]
    fn reads_wpctl_status() {
        let text = "PipeWire 'pipewire-0'\nAudio\n ├─ Devices:\n │      42. Family 17h HD Audio\n ├─ Sinks:\n │  *   53. Speaker                    [vol: 0.40]\n │      61. HDMI / DisplayPort 1 Output [vol: 1.00]\n ├─ Sources:\n │  *   54. Mic\nVideo\n";
        let d = audio::parse_devices(text);
        assert_eq!(d, vec![json!({"id": "53", "name": "Speaker", "isDefault": true}), json!({"id": "61", "name": "HDMI / DisplayPort 1 Output", "isDefault": false})]);
    }
    #[test]
    fn knows_controllers() {
        assert!(bt_scan::is_gamepad(&json!({"kind": "Input.Gamepad", "name": "Pad"})));
        for n in ["Xbox Wireless Controller", "DualSense Wireless Controller", "Pro Controller", "8BitDo Ultimate 2C", "Joy-Con (L)"] { assert!(bt_scan::is_gamepad(&json!({"kind": "Other", "name": n})), "{n}"); }
        for n in ["JBL Flip 5", "Logitech K380"] { assert!(!bt_scan::is_gamepad(&json!({"kind": "Other", "name": n})), "{n}"); }
    }
    #[test]
    fn reads_bluetoothctl() {
        let d = bt_devices::parse_info("AA:BB", "Device AA:BB\n\tName: Xbox Wireless Controller\n\tIcon: input-gaming\n\tPaired: yes\n\tConnected: no\n");
        assert_eq!(d, json!({"id": "AA:BB", "name": "Xbox Wireless Controller", "kind": "Input.Gamepad", "connected": false, "paired": true}));
    }
    #[test]
    fn reads_wpctl_and_batteries() {
        assert_eq!(audio::parse_volume("Volume: 0.45\n"), (45, false));
        assert_eq!(audio::parse_volume("Volume: 1.20 [MUTED]\n"), (120, true));
        let dir = tempfile::tempdir().unwrap();
        let put = |n: &str, f: &str, v: &str| { std::fs::create_dir_all(dir.path().join(n)).unwrap(); std::fs::write(dir.path().join(n).join(f), v).unwrap(); };
        put("hidpp_battery_0", "type", "Battery"); put("hidpp_battery_0", "scope", "Device"); put("hidpp_battery_0", "capacity", "10");
        put("BAT1", "type", "Battery"); put("BAT1", "capacity", "73"); put("BAT1", "status", "Charging\n");
        assert_eq!(battery(dir.path()), json!({"present": true, "pct": 73, "charging": true}));
        assert_eq!(battery(&dir.path().join("none")), json!({"present": false, "pct": 0, "charging": false}));
    }
}
