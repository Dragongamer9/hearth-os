//! What Settings reads and the commands it runs. Every read is one short program (nmcli, bluetoothctl, wpctl, pactl,
//! kscreen-doctor, powerprofilesctl, timedatectl, df, Hearth's own helpers) run off the window's thread; the parsers
//! here turn their output into plain values (tested below). Nothing here writes a file itself: Hearth's settings go
//! through hearth_settings_io.py (atomic, .bak, merged, the change history: red flag 9), the system's through its own
//! tools.

use serde_json::Value;

pub fn sh(script: &str) -> Vec<String> {
    vec!["sh".into(), "-c".into(), script.into()]
}

/// The parts of a combined read: lines after "#name" markers.
pub fn sections(out: &str) -> std::collections::HashMap<String, String> {
    let mut m = std::collections::HashMap::new();
    let mut cur = String::new();
    let mut buf = String::new();
    for l in out.lines() {
        if let Some(name) = l.strip_prefix("#@") {
            if !cur.is_empty() {
                m.insert(cur.clone(), buf.trim_end().to_string());
            }
            cur = name.trim().to_string();
            buf.clear();
        } else {
            buf.push_str(l);
            buf.push('\n');
        }
    }
    if !cur.is_empty() {
        m.insert(cur, buf.trim_end().to_string());
    }
    m
}

fn sec(m: &std::collections::HashMap<String, String>, k: &str) -> String {
    m.get(k).cloned().unwrap_or_default()
}

pub fn q(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

// ------------------------------------------------------------------ this PC
#[derive(Debug, Clone, Default)]
pub struct Info {
    pub host: String,
    pub user: String,
    pub real: String,
    pub admin: bool,
    pub os: String,
    pub version: String,
    pub kernel: String,
    pub cpu: String,
    pub gpu: String,
    pub mem_kb: u64,
    pub desktop: String,
}

pub const INFO: &str = "echo '#@host'; cat /etc/hostname 2>/dev/null || hostname; echo '#@user'; id -un; getent passwd \"$(id -un)\" | cut -d: -f5 | cut -d, -f1; echo '#@groups'; id -nG; echo '#@os'; cat /etc/os-release 2>/dev/null; echo '#@kernel'; uname -r; echo '#@cpu'; grep -m1 'model name' /proc/cpuinfo; echo '#@gpu'; lspci 2>/dev/null | grep -iE 'vga|3d|display'; echo '#@mem'; grep -m1 MemTotal /proc/meminfo";

fn os_field(text: &str, k: &str) -> String {
    text.lines().find_map(|l| l.strip_prefix(&format!("{k}="))).map(|v| v.trim_matches('"').to_string()).unwrap_or_default()
}

pub fn info(out: &str) -> Info {
    let m = sections(out);
    let user = sec(&m, "user");
    let mut ul = user.lines();
    let login = ul.next().unwrap_or("").trim().to_string();
    let real = ul.next().unwrap_or("").trim().to_string();
    let os = sec(&m, "os");
    let gpu = sec(&m, "gpu");
    let gpu = gpu.lines().next().unwrap_or("").splitn(2, ": ").nth(1).unwrap_or("").to_string();
    Info {
        host: sec(&m, "host").lines().next().unwrap_or("").trim().to_string(),
        real: if real.is_empty() { login.clone() } else { real },
        user: login,
        admin: sec(&m, "groups").split_whitespace().any(|g| g == "wheel" || g == "sudo" || g == "admin"),
        os: { let n = os_field(&os, "PRETTY_NAME"); if n.is_empty() { os_field(&os, "NAME") } else { n } },
        version: os_field(&os, "VERSION"),
        kernel: sec(&m, "kernel").trim().to_string(),
        cpu: sec(&m, "cpu").splitn(2, ':').nth(1).unwrap_or("").trim().to_string(),
        gpu: short_gpu(&gpu),
        mem_kb: sec(&m, "mem").split_whitespace().nth(1).and_then(|v| v.parse().ok()).unwrap_or(0),
        desktop: std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default(),
    }
}

/// "Intel Corporation Alder Lake-N [UHD Graphics]" -> "UHD Graphics"; "NVIDIA ... [GeForce RTX 5070]" -> "GeForce RTX 5070".
pub fn short_gpu(s: &str) -> String {
    let s = s.trim();
    if let (Some(a), Some(b)) = (s.rfind('['), s.rfind(']')) {
        if b > a + 1 {
            return s[a + 1..b].to_string();
        }
    }
    s.split(" (rev").next().unwrap_or(s).to_string()
}

pub fn short_cpu(s: &str) -> String {
    s.replace("(R)", "").replace("(TM)", "").replace(" CPU", "").replace("  ", " ").split(" @ ").next().unwrap_or("").trim().to_string()
}

/// A PC name as you type it: letters, numbers and dashes, at most 63.
pub fn host_chars(t: &str) -> String {
    t.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').take(63).collect()
}

// ------------------------------------------------------------------ Wi-Fi
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Net {
    pub ssid: String,
    pub signal: u32,
    pub secure: bool,
    pub active: bool,
    pub known: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Wifi {
    pub on: bool,
    pub has: bool,
    pub nets: Vec<Net>,
    pub wired: String,
}

pub const WIFI: &str = "echo '#@radio'; nmcli -t radio wifi 2>/dev/null; echo '#@dev'; nmcli -t -f TYPE,STATE,CONNECTION dev 2>/dev/null; echo '#@list'; nmcli -t -f IN-USE,SSID,SIGNAL,SECURITY dev wifi list 2>/dev/null; echo '#@known'; nmcli -t -f NAME,TYPE con show 2>/dev/null";

/// One nmcli -t line split at its unescaped colons.
pub fn nm_fields(line: &str) -> Vec<String> {
    let mut out = vec![String::new()];
    let mut esc = false;
    for c in line.chars() {
        if esc {
            out.last_mut().unwrap().push(c);
            esc = false;
        } else if c == '\\' {
            esc = true;
        } else if c == ':' {
            out.push(String::new());
        } else {
            out.last_mut().unwrap().push(c);
        }
    }
    out
}

pub fn wifi(out: &str) -> Wifi {
    let m = sections(out);
    let known: Vec<String> = sec(&m, "known")
        .lines()
        .map(nm_fields)
        .filter(|f| f.get(1).map(|t| t.contains("wireless")).unwrap_or(false))
        .map(|f| f[0].clone())
        .collect();
    let mut nets: Vec<Net> = Vec::new();
    for l in sec(&m, "list").lines() {
        let f = nm_fields(l);
        if f.len() < 4 || f[1].is_empty() {
            continue;
        }
        let n = Net { active: f[0].trim() == "*", ssid: f[1].clone(), signal: f[2].parse().unwrap_or(0), secure: !f[3].trim().is_empty() && f[3].trim() != "--", known: known.contains(&f[1]) };
        match nets.iter_mut().find(|x| x.ssid == n.ssid) {
            Some(x) => {
                if n.active || (!x.active && n.signal > x.signal) {
                    *x = n;
                }
            }
            None => nets.push(n),
        }
    }
    nets.sort_by(|a, b| b.active.cmp(&a.active).then(b.signal.cmp(&a.signal)));
    let dev = sec(&m, "dev");
    let wired = dev.lines().map(nm_fields).find(|f| f.first().map(|t| t == "ethernet").unwrap_or(false) && f.get(1).map(|s| s == "connected").unwrap_or(false)).and_then(|f| f.get(2).cloned()).unwrap_or_default();
    Wifi { on: sec(&m, "radio").trim() == "enabled", has: dev.lines().any(|l| l.starts_with("wifi:")), nets, wired }
}

pub fn signal_word(s: u32) -> &'static str {
    match s {
        75..=100 => "strong",
        50..=74 => "good",
        25..=49 => "fair",
        _ => "weak",
    }
}

// ------------------------------------------------------------------ Bluetooth
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BtDev {
    pub mac: String,
    pub name: String,
    pub paired: bool,
    pub connected: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Bt {
    pub has: bool,
    pub on: bool,
    pub devs: Vec<BtDev>,
}

pub const BT: &str = "echo '#@show'; timeout 4 bluetoothctl show 2>/dev/null; echo '#@paired'; timeout 4 bluetoothctl devices Paired 2>/dev/null || timeout 4 bluetoothctl paired-devices 2>/dev/null; echo '#@connected'; timeout 4 bluetoothctl devices Connected 2>/dev/null; echo '#@all'; timeout 4 bluetoothctl devices 2>/dev/null";

fn bt_list(t: &str) -> Vec<(String, String)> {
    t.lines()
        .filter_map(|l| l.strip_prefix("Device "))
        .filter_map(|r| {
            let mut p = r.splitn(2, ' ');
            let mac = p.next()?.to_string();
            Some((mac.clone(), p.next().unwrap_or(&mac).to_string()))
        })
        .collect()
}

pub fn bt(out: &str) -> Bt {
    let m = sections(out);
    let show = sec(&m, "show");
    let paired = bt_list(&sec(&m, "paired"));
    let connected = bt_list(&sec(&m, "connected"));
    let mut devs: Vec<BtDev> = Vec::new();
    for (mac, name) in bt_list(&sec(&m, "all")).into_iter().chain(paired.clone()) {
        if devs.iter().any(|d| d.mac == mac) {
            continue;
        }
        // A device with no name of its own shows its address with dashes: those are left out of the list.
        if name.replace('-', ":") == mac {
            continue;
        }
        devs.push(BtDev { paired: paired.iter().any(|p| p.0 == mac), connected: connected.iter().any(|p| p.0 == mac), mac, name });
    }
    devs.sort_by(|a, b| b.connected.cmp(&a.connected).then(b.paired.cmp(&a.paired)).then(a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    Bt { has: show.contains("Controller"), on: show.lines().any(|l| l.trim() == "Powered: yes"), devs }
}

// ------------------------------------------------------------------ sound
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Port {
    pub name: String,
    pub desc: String,
}

#[derive(Debug, Clone, Default)]
pub struct Sound {
    pub vol: f32,
    pub muted: bool,
    pub mic: f32,
    pub mic_muted: bool,
    pub out: String,
    pub inp: String,
    pub outs: Vec<Port>,
    pub ins: Vec<Port>,
    pub ok: bool,
}

pub const SOUND: &str = "echo '#@vol'; wpctl get-volume @DEFAULT_AUDIO_SINK@ 2>/dev/null; echo '#@mic'; wpctl get-volume @DEFAULT_AUDIO_SOURCE@ 2>/dev/null; echo '#@out'; pactl get-default-sink 2>/dev/null; echo '#@in'; pactl get-default-source 2>/dev/null; echo '#@sinks'; pactl -f json list sinks 2>/dev/null; echo; echo '#@sources'; pactl -f json list sources 2>/dev/null; echo";

/// wpctl's "Volume: 0.45 [MUTED]" -> (0.45, true).
pub fn wp_volume(t: &str) -> Option<(f32, bool)> {
    let v = t.trim().strip_prefix("Volume:")?.split_whitespace().next()?.parse::<f32>().ok()?;
    Some((v, t.contains("[MUTED]")))
}

fn ports(t: &str, skip_monitors: bool) -> Vec<Port> {
    let v: Value = serde_json::from_str(t.trim()).unwrap_or(Value::Null);
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|x| {
                    let name = x.get("name")?.as_str()?.to_string();
                    if skip_monitors && name.ends_with(".monitor") {
                        return None;
                    }
                    let desc = x.get("description").and_then(Value::as_str).unwrap_or(&name).to_string();
                    Some(Port { name, desc })
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn sound(out: &str) -> Sound {
    let m = sections(out);
    let (vol, muted) = wp_volume(&sec(&m, "vol")).unwrap_or((0.0, false));
    let (mic, mic_muted) = wp_volume(&sec(&m, "mic")).unwrap_or((0.0, false));
    Sound {
        ok: wp_volume(&sec(&m, "vol")).is_some(),
        vol,
        muted,
        mic,
        mic_muted,
        out: sec(&m, "out").trim().to_string(),
        inp: sec(&m, "in").trim().to_string(),
        outs: ports(&sec(&m, "sinks"), true),
        ins: ports(&sec(&m, "sources"), true),
    }
}

// ------------------------------------------------------------------ displays
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Mode {
    pub id: String,
    pub w: u32,
    pub h: u32,
    pub hz: f32,
}

#[derive(Debug, Clone, Default)]
pub struct Output {
    pub name: String,
    pub connected: bool,
    pub enabled: bool,
    pub modes: Vec<Mode>,
    pub current: String,
    pub preferred: Vec<String>,
    pub scale: f32,
    pub priority: i64,
}

impl Output {
    pub fn cur(&self) -> Option<&Mode> {
        self.modes.iter().find(|m| m.id == self.current)
    }
    /// The resolutions it can show, biggest first, each once.
    pub fn sizes(&self) -> Vec<(u32, u32)> {
        let mut v: Vec<(u32, u32)> = Vec::new();
        for m in &self.modes {
            if !v.contains(&(m.w, m.h)) {
                v.push((m.w, m.h));
            }
        }
        v.sort_by(|a, b| (b.0 * b.1).cmp(&(a.0 * a.1)));
        v
    }
    /// The refresh rates at its current resolution, highest first (rounded to whole Hz, each once).
    pub fn rates(&self) -> Vec<(u32, String)> {
        let Some(c) = self.cur() else { return vec![] };
        let mut v: Vec<(u32, String)> = Vec::new();
        for m in self.modes.iter().filter(|m| m.w == c.w && m.h == c.h) {
            let r = m.hz.round() as u32;
            if !v.iter().any(|x| x.0 == r) {
                v.push((r, m.id.clone()));
            }
        }
        v.sort_by(|a, b| b.0.cmp(&a.0));
        v
    }
    /// The mode for a size, at the refresh rate closest to the current one.
    pub fn mode_for(&self, w: u32, h: u32) -> Option<String> {
        let hz = self.cur().map(|c| c.hz).unwrap_or(60.0);
        self.modes.iter().filter(|m| m.w == w && m.h == h).min_by(|a, b| (a.hz - hz).abs().partial_cmp(&(b.hz - hz).abs()).unwrap_or(std::cmp::Ordering::Equal)).map(|m| m.id.clone())
    }
}

#[derive(Debug, Clone, Default)]
pub struct Display {
    pub outputs: Vec<Output>,
    pub bright: Option<u32>,
    pub night: bool,
    pub ok: bool,
}

pub const DISPLAY: &str = "echo '#@ks'; kscreen-doctor -j 2>/dev/null; echo; echo '#@br'; brightnessctl -m -c backlight 2>/dev/null | head -1; echo '#@night'; kreadconfig6 --file kwinrc --group NightColor --key Active 2>/dev/null";

pub fn display(out: &str) -> Display {
    let m = sections(out);
    let v: Value = serde_json::from_str(sec(&m, "ks").trim()).unwrap_or(Value::Null);
    let num = |x: &Value| -> f64 { x.as_f64().or_else(|| x.as_str().and_then(|s| s.parse().ok())).unwrap_or(0.0) };
    let id = |x: &Value| -> String { x.as_str().map(String::from).or_else(|| x.as_i64().map(|n| n.to_string())).unwrap_or_default() };
    let mut outputs: Vec<Output> = v
        .get("outputs")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .map(|o| Output {
                    name: o.get("name").and_then(Value::as_str).unwrap_or("").to_string(),
                    connected: o.get("connected").and_then(Value::as_bool).unwrap_or(false),
                    enabled: o.get("enabled").and_then(Value::as_bool).unwrap_or(false),
                    current: o.get("currentModeId").map(id).unwrap_or_default(),
                    preferred: o.get("preferredModes").and_then(Value::as_array).map(|p| p.iter().map(id).collect()).unwrap_or_default(),
                    scale: o.get("scale").map(num).unwrap_or(1.0) as f32,
                    priority: o.get("priority").map(num).unwrap_or(0.0) as i64,
                    modes: o
                        .get("modes")
                        .and_then(Value::as_array)
                        .map(|ms| {
                            ms.iter()
                                .map(|x| Mode {
                                    id: x.get("id").map(id).unwrap_or_default(),
                                    w: x.get("size").and_then(|s| s.get("width")).map(num).unwrap_or(0.0) as u32,
                                    h: x.get("size").and_then(|s| s.get("height")).map(num).unwrap_or(0.0) as u32,
                                    hz: x.get("refreshRate").map(num).unwrap_or(0.0) as f32,
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                })
                .filter(|o| o.connected)
                .collect()
        })
        .unwrap_or_default();
    outputs.sort_by_key(|o| if o.priority <= 0 { 99 } else { o.priority });
    let br = sec(&m, "br");
    let bright = br.split(',').nth(3).and_then(|p| p.trim_end_matches('%').parse().ok());
    Display { ok: !outputs.is_empty(), outputs, bright, night: sec(&m, "night").trim() == "true" }
}

pub fn night_cmd(on: bool) -> Vec<String> {
    sh(&format!(
        "kwriteconfig6 --file kwinrc --group NightColor --key Active {} && dbus-send --session --type=method_call --dest=org.kde.KWin /KWin org.kde.KWin.reconfigure",
        if on { "true" } else { "false" }
    ))
}

// ------------------------------------------------------------------ power
#[derive(Debug, Clone, Default)]
pub struct Power {
    pub profile: String,
    pub profiles: Vec<String>,
    pub battery: Option<u32>,
    pub charging: String,
}

pub const POWER: &str = "echo '#@get'; powerprofilesctl get 2>/dev/null; echo '#@list'; powerprofilesctl list 2>/dev/null; echo '#@bat'; cat /sys/class/power_supply/BAT*/capacity 2>/dev/null | head -1; echo '#@status'; cat /sys/class/power_supply/BAT*/status 2>/dev/null | head -1";

pub fn power(out: &str) -> Power {
    let m = sections(out);
    let profiles: Vec<String> = sec(&m, "list")
        .lines()
        .filter_map(|l| {
            let t = l.trim_start_matches(['*', ' ']);
            let indent = l.len() - l.trim_start().len();
            if indent <= 2 && t.ends_with(':') { Some(t.trim_end_matches(':').to_string()) } else { None }
        })
        .collect();
    let mut order = vec!["power-saver", "balanced", "performance"];
    order.retain(|p| profiles.iter().any(|x| x == p));
    Power {
        profile: sec(&m, "get").trim().to_string(),
        profiles: order.into_iter().map(String::from).collect(),
        battery: sec(&m, "bat").trim().parse().ok(),
        charging: sec(&m, "status").trim().to_string(),
    }
}

pub fn profile_name(p: &str) -> &'static str {
    match p {
        "power-saver" => "Power saver",
        "performance" => "Performance",
        _ => "Balanced",
    }
}

// ------------------------------------------------------------------ storage
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Drive {
    pub name: String,
    pub size: u64,
    pub used: u64,
}

pub const STORAGE: &str = "df -B1 --output=source,size,used,target -x tmpfs -x devtmpfs -x efivarfs -x overlay -x squashfs -x composefs 2>/dev/null";

pub fn storage(out: &str) -> Vec<Drive> {
    let mut seen: Vec<String> = Vec::new();
    let mut v = Vec::new();
    for l in out.lines().skip(1) {
        let f: Vec<&str> = l.split_whitespace().collect();
        if f.len() < 4 || !f[0].starts_with("/dev/") {
            continue;
        }
        let target = f[3..].join(" ");
        if target.starts_with("/boot") || target == "/usr" || target == "/etc" || seen.contains(&f[0].to_string()) {
            continue;
        }
        seen.push(f[0].to_string());
        let name = if target == "/" || target == "/sysroot" || target == "/var" || target.starts_with("/var/home") || target == "/home" {
            "This PC (C:)".to_string()
        } else {
            target.rsplit('/').next().unwrap_or(&target).to_string()
        };
        v.push(Drive { name, size: f[1].parse().unwrap_or(0), used: f[2].parse().unwrap_or(0) });
    }
    v
}

// ------------------------------------------------------------------ date & time
#[derive(Debug, Clone, Default)]
pub struct Time {
    pub zone: String,
    pub ntp: bool,
    pub zones: Vec<String>,
}

pub const TIME: &str = "timedatectl show -p Timezone -p NTP 2>/dev/null";

pub fn time(out: &str) -> Time {
    let g = |k: &str| out.lines().find_map(|l| l.strip_prefix(&format!("{k}="))).unwrap_or("").trim().to_string();
    Time { zone: g("Timezone"), ntp: g("NTP") == "yes", zones: Vec::new() }
}

// ------------------------------------------------------------------ Desktop Mode
#[derive(Debug, Clone, Default)]
pub struct Desk {
    pub plasma_fh: bool,
    pub chosen: String,
    // >>> labwc on the Fedora edition (1.16, feature/labwc-fedora): hearth-labwc-desktop is in the image, and what's missing
    pub labwc: bool,
    pub labwc_missing: String,
    // <<< labwc on the Fedora edition
}

// (Firehold (Rust, test) was archived on Nov 19: hearth-firehold-plasma's `choice` already answers "firehold" for a
// device that had picked it, since that's what opens there now, and its next pick replaces that old pick.)
pub const DESK: &str = "echo '#@fp'; command -v hearth-firehold-plasma >/dev/null && hearth-firehold-plasma choice 2>/dev/null; echo '#@has'; command -v hearth-firehold-plasma; command -v hearth-labwc-desktop; echo '#@lw'; command -v hearth-labwc-desktop >/dev/null && timeout 10 hearth-labwc-desktop status 2>/dev/null";

pub fn desk(out: &str) -> Desk {
    let m = sections(out);
    let has = sec(&m, "has");
    let plasma_fh = has.contains("hearth-firehold-plasma");
    let mut chosen = if plasma_fh && sec(&m, "fp").trim() == "firehold" { "firehold-plasma" } else { "plasma" };
    // >>> labwc on the Fedora edition: its own pick wins (the same session.conf line: classic / classicfirehold)
    let labwc = has.contains("hearth-labwc-desktop");
    let lw: serde_json::Value = serde_json::from_str(sec(&m, "lw").trim()).unwrap_or(serde_json::Value::Null);
    if labwc {
        match lw.get("choice").and_then(|v| v.as_str()).unwrap_or("") {
            "classic" => chosen = "classic",
            "firehold" => chosen = "firehold-labwc",
            _ => {}
        }
    }
    let labwc_missing = lw.get("missing").and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|x| x.as_str()).collect::<Vec<_>>().join(" and ")).unwrap_or_default();
    // <<< labwc on the Fedora edition
    Desk { plasma_fh, chosen: chosen.into(), labwc, labwc_missing }
}

/// Saving a Desktop Mode choice: Firehold on Plasma's own helper saves its own line (hearth_settings_io).
pub fn desk_cmd(d: &Desk, pick: &str) -> Vec<String> {
    let fp = |w: &str| if d.plasma_fh { format!("hearth-firehold-plasma choose {w} >/dev/null") } else { "true".into() };
    // >>> labwc on the Fedora edition: its picks, and KDE Plasma's pick moves off them (hearth-firehold-plasma alone
    // writes nothing when its reader already says plasma)
    let lw = |w: &str| if d.labwc { format!("hearth-labwc-desktop choose {w}") } else { "true".into() };
    let s = match pick {
        "classic" => lw("classic"),
        "firehold-labwc" => lw("firehold"),
        "firehold-plasma" => fp("firehold"),
        _ => format!("{} && {}", lw("plasma"), fp("plasma")),
    };
    // <<< labwc on the Fedora edition
    sh(&s)
}

// ------------------------------------------------------------------ Start in
pub fn session_conf() -> std::path::PathBuf {
    hearth_ui::paths::home().join(".config/nimbus/session.conf")
}

pub fn start_mode() -> String {
    let t = std::fs::read_to_string(session_conf()).unwrap_or_default();
    let mut mode = "game".to_string();
    for l in t.lines() {
        let l = l.trim().trim_start_matches("export ").trim();
        if let Some(v) = l.strip_prefix("NIMBUS_START_MODE=") {
            let v = v.trim_matches(['"', '\'']);
            if ["game", "desktop", "server"].contains(&v) {
                mode = v.to_string();
            }
        }
    }
    mode
}

// ------------------------------------------------------------------ updates
pub const ADMIN: &str = "/usr/libexec/nimbus/hearth-admin";

#[derive(Debug, Clone, Default)]
pub struct Updates {
    pub booted: String,
    pub booted_image: String,
    pub booted_when: String,
    pub staged: String,
    pub available: String,
    pub rollback: String,
    pub read: bool,
    pub error: String,
}

pub fn shown_version(v: &str) -> String {
    let p: Vec<&str> = v.split('.').collect();
    if p.len() == 3 && p[2] == "0" { format!("{}.{}", p[0], p[1]) } else { v.to_string() }
}

pub fn updates(v: &Value) -> Updates {
    let ver = |k: &str| v.get(k).filter(|x| !x.is_null()).map(|x| x.get("version").and_then(Value::as_str).unwrap_or("a new version").to_string()).unwrap_or_default();
    let b = v.get("booted").cloned().unwrap_or(Value::Null);
    Updates {
        booted: b.get("version").and_then(Value::as_str).unwrap_or("").to_string(),
        booted_image: b.get("image").and_then(Value::as_str).unwrap_or("").to_string(),
        booted_when: b.get("timestamp").and_then(Value::as_str).unwrap_or("").to_string(),
        staged: ver("staged"),
        available: ver("available"),
        rollback: ver("rollback"),
        read: true,
        error: String::new(),
    }
}

// ------------------------------------------------------------------ the change history
#[derive(Debug, Clone, Default)]
pub struct Change {
    pub id: i64,
    pub key: String,
    pub to: String,
    pub who: String,
    pub at: i64,
}

pub fn changes(v: &Value) -> Vec<Change> {
    let show = |x: &Value| -> String {
        match x {
            Value::Null => "Off".into(),
            Value::Bool(b) => if *b { "On".into() } else { "Off".into() },
            Value::String(s) if s.is_empty() => "The look's own".into(),
            Value::String(s) => s.clone(),
            other => other.to_string(),
        }
    };
    v.as_array()
        .map(|a| {
            a.iter()
                .filter(|c| !matches!(c.get("key").and_then(Value::as_str), Some("accentFrom" | "wallSwatches" | "menuTiles" | "nightLat" | "nightLon")))
                .map(|c| Change {
                    id: c.get("id").and_then(Value::as_i64).unwrap_or(0),
                    key: c.get("key").and_then(Value::as_str).unwrap_or("").to_string(),
                    to: show(c.get("to").unwrap_or(&Value::Null)),
                    who: c.get("who").and_then(Value::as_str).unwrap_or("you").to_string(),
                    at: c.get("at").and_then(Value::as_i64).unwrap_or(0),
                })
                .collect()
        })
        .unwrap_or_default()
}

/// settings-changes.js NAMES: what a desktop.json key is called.
pub fn setting_name(k: &str) -> String {
    match k {
        "nightLight" => "Night light", "wallpaper" => "Wallpaper", "reduceMotion" => "Reduce motion", "pointerSpeed" => "Pointer speed",
        "lockOnWake" => "Lock when it wakes", "clock24" => "24-hour clock", "clipboardHistory" => "Clipboard history",
        "screenOffMinutes" => "Screen off after", "sleepMinutes" => "Sleep after", "look" => "Look", "mode" => "Mode", "accent" => "Accent colour",
        "barStyle" => "Bar style", "dockMode" => "Dock", "subTheme" => "Sub theme", "sunset" => "Sunset mode", "appMenu" => "App menu",
        other => return other.to_string(),
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nmcli_lines_and_wifi() {
        assert_eq!(nm_fields(r"*:My\:Net:80:WPA2"), vec!["*", "My:Net", "80", "WPA2"]);
        let out = "#@radio\nenabled\n#@dev\nwifi:connected:Home-5G\nethernet:unavailable:\n#@list\n*:Home-5G:82:WPA2\n :Cafe:40:\n :Home-5G:30:WPA2\n :Neighbour:55:WPA1 WPA2\n#@known\nHome-5G:802-11-wireless\nWired:802-3-ethernet\n";
        let w = wifi(out);
        assert!(w.on && w.has);
        assert_eq!(w.nets.len(), 3);
        assert_eq!(w.nets[0].ssid, "Home-5G");
        assert!(w.nets[0].active && w.nets[0].known && w.nets[0].signal == 82);
        assert!(!w.nets.iter().find(|n| n.ssid == "Cafe").unwrap().secure);
    }

    #[test]
    fn bluetooth() {
        let out = "#@show\nController 11:22:33:44:55:66 (public)\n\tPowered: yes\n#@paired\nDevice AA:BB:CC:DD:EE:FF Xbox Controller\n#@connected\nDevice AA:BB:CC:DD:EE:FF Xbox Controller\n#@all\nDevice AA:BB:CC:DD:EE:FF Xbox Controller\nDevice 12:34:56:78:9A:BC Speaker\nDevice 01:02:03:04:05:06 01-02-03-04-05-06\n";
        let b = bt(out);
        assert!(b.has && b.on);
        assert_eq!(b.devs.len(), 2);
        assert!(b.devs[0].connected && b.devs[0].paired);
        assert!(!b.devs[1].paired);
    }

    #[test]
    fn sound_and_power() {
        assert_eq!(wp_volume("Volume: 0.45 [MUTED]"), Some((0.45, true)));
        assert_eq!(wp_volume("Volume: 1.00"), Some((1.0, false)));
        let s = sound("#@vol\nVolume: 0.30\n#@mic\n\n#@out\nalsa_output.pci\n#@in\nalsa_input.pci\n#@sinks\n[{\"name\":\"alsa_output.pci\",\"description\":\"Speakers\"}]\n#@sources\n[{\"name\":\"alsa_output.pci.monitor\",\"description\":\"Monitor\"},{\"name\":\"alsa_input.pci\",\"description\":\"Mic\"}]\n");
        assert!(s.ok && (s.vol - 0.3).abs() < 0.01 && s.outs.len() == 1 && s.ins.len() == 1);
        let p = power("#@get\nbalanced\n#@list\n  performance:\n    CpuDriver:\tintel_pstate\n\n* balanced:\n    CpuDriver:\tintel_pstate\n\n  power-saver:\n#@bat\n82\n#@status\nDischarging\n");
        assert_eq!(p.profiles, vec!["power-saver", "balanced", "performance"]);
        assert_eq!(p.profile, "balanced");
        assert_eq!(p.battery, Some(82));
    }

    #[test]
    fn displays_storage_info() {
        let ks = r#"{"outputs":[{"name":"eDP-1","connected":true,"enabled":true,"currentModeId":"2","priority":1,"scale":1.25,"preferredModes":["2"],"modes":[{"id":"1","refreshRate":60.0,"size":{"width":1280,"height":720}},{"id":"2","refreshRate":60.02,"size":{"width":1920,"height":1080}},{"id":"3","refreshRate":48.0,"size":{"width":1920,"height":1080}}]},{"name":"HDMI-A-1","connected":false,"modes":[]}]}"#;
        let d = display(&format!("#@ks\n{ks}\n#@br\nintel_backlight,backlight,4000,50%,8000\n#@night\ntrue\n"));
        assert_eq!(d.outputs.len(), 1);
        let o = &d.outputs[0];
        assert_eq!(o.sizes()[0], (1920, 1080));
        assert_eq!(o.rates().iter().map(|r| r.0).collect::<Vec<_>>(), vec![60, 48]);
        assert_eq!(o.mode_for(1280, 720).as_deref(), Some("1"));
        assert_eq!(d.bright, Some(50));
        assert!(d.night);
        let st = storage("Filesystem 1B-blocks Used Mounted on\n/dev/nvme0n1p3 256000000000 64000000000 /sysroot\n/dev/nvme0n1p3 256000000000 64000000000 /var\n/dev/nvme0n1p2 1000000000 100000000 /boot\n/dev/sda1 2000000000000 1600000000000 /run/media/chris/Games\n");
        assert_eq!(st.len(), 2);
        assert_eq!(st[0].name, "This PC (C:)");
        assert_eq!(st[1].name, "Games");
        let i = info("#@host\nchris-hearth-laptop\n#@user\nchris\nChris\n#@groups\nchris wheel\n#@os\nNAME=\"Hearth OS\"\nPRETTY_NAME=\"Hearth OS (Fedora edition)\"\nVERSION=\"1.14\"\n#@kernel\n7.3.4\n#@cpu\nmodel name\t: Intel(R) Core(TM) i3-N305\n#@gpu\n00:02.0 VGA compatible controller: Intel Corporation Alder Lake-N [UHD Graphics]\n#@mem\nMemTotal:        7864320 kB\n");
        assert_eq!(i.host, "chris-hearth-laptop");
        assert!(i.admin);
        assert_eq!(i.real, "Chris");
        assert_eq!(i.gpu, "UHD Graphics");
        assert_eq!(short_cpu(&i.cpu), "Intel Core i3-N305");
        assert_eq!(shown_version("1.14.0"), "1.14");
    }

    #[test]
    fn desktop_choice() {
        let d = desk("#@fp\nfirehold\n#@has\n/usr/bin/hearth-firehold-plasma\n");
        assert_eq!(d.chosen, "firehold-plasma");
        let c = desk_cmd(&d, "plasma").join(" ");
        assert!(c.contains("hearth-firehold-plasma choose plasma") && !c.contains("firehold-shell"));
        let none = desk("#@fp\n#@has\n");
        assert_eq!(none.chosen, "plasma");
        // labwc on the Fedora edition
        let l = desk("#@fp\nplasma\n#@has\n/usr/bin/hearth-firehold-plasma\n/usr/bin/hearth-labwc-desktop\n#@lw\n{\"choice\": \"firehold\", \"available\": true, \"missing\": []}\n");
        assert_eq!(l.chosen, "firehold-labwc");
        assert!(l.labwc && l.labwc_missing.is_empty());
        let c = desk_cmd(&l, "plasma").join(" ");
        assert!(c.contains("hearth-labwc-desktop choose plasma") && c.contains("hearth-firehold-plasma choose plasma"));
        assert!(desk_cmd(&l, "classic").join(" ").contains("hearth-labwc-desktop choose classic"));
        let gone = desk("#@fp\n#@has\n/usr/bin/hearth-labwc-desktop\n#@lw\n{\"choice\": \"classic\", \"available\": false, \"missing\": [\"Quickshell\"]}\n");
        assert_eq!((gone.chosen.as_str(), gone.labwc_missing.as_str()), ("classic", "Quickshell"));
        assert_eq!(q("it's"), "'it'\\''s'");
    }
}
