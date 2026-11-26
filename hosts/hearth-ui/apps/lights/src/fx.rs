//! What Hearth Lights does with the lights: the person's choices (lights.json, through the safe writer), the effects
//! (each device's own mode where it has one, so it's kept on the device itself; Hearth's own frames through OpenRGB's
//! Direct mode where it hasn't, or for the effects only Hearth has), and the runner that keeps them going.
//!
//! The runner (`hearth-lights-app --run`, the user service hearth-lights.service) is light: one frame every 50 ms
//! only while a moving effect of Hearth's own is on, otherwise it wakes once a second to see whether the choices or
//! the profile's moment (a game started, Desktop Mode, the screen asleep) changed. It never writes the person's
//! settings: which profile is on right now is in $XDG_RUNTIME_DIR/hearth-lights-now.json (gone at sign-out).

use crate::orgb::{self, Conn, Device, Rgb};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub const APP: &str = "lights";

/// The effects, as the board names them (id, name).
pub const EFFECTS: [(&str, &str); 9] = [
    ("still", "Still"),
    ("breathing", "Breathing"),
    ("ember", "Ember wave"),
    ("rainbow", "Rainbow"),
    ("keys", "Reacts to keys"),
    ("heat", "Hotter = redder"),
    ("music", "To the music"),
    ("game", "Game colours"),
    ("off", "Off"),
];
pub fn effect_name(id: &str) -> &'static str {
    EFFECTS.iter().find(|e| e.0 == id).map(|e| e.1).unwrap_or("Still")
}

pub const DIRECTIONS: [(&str, &str); 5] = [("right", "→"), ("left", "←"), ("up", "↑"), ("down", "↓"), ("centre", "From centre")];

pub const EMBER: [Rgb; 3] = [[0xff, 0x8a, 0x3d], [0xff, 0x3d, 0x6a], [0xb0, 0x4f, 0xc8]];

#[derive(Debug, Clone, PartialEq)]
pub struct Look {
    pub effect: String,
    pub colour: Rgb,
    /// 0-100
    pub speed: u8,
    pub brightness: u8,
    pub direction: String,
}

impl Default for Look {
    fn default() -> Look {
        Look { effect: "ember".into(), colour: [0xff, 0x8a, 0x3d], speed: 40, brightness: 80, direction: "right".into() }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Profile {
    pub id: String,
    pub name: String,
    /// game | desktop | sleep | hand
    pub when: String,
    pub look: Look,
}

impl Profile {
    pub fn when_words(&self) -> String {
        match self.when.as_str() {
            "game" => "when a game starts".into(),
            "desktop" => "in Desktop Mode".into(),
            "sleep" => "off when the screen sleeps".into(),
            _ if self.look.effect == "still" && self.look.brightness <= 40 => "dim, one colour".into(),
            _ => "by hand".into(),
        }
    }
}

pub fn default_profiles() -> Vec<Profile> {
    vec![
        Profile { id: "gaming".into(), name: "Gaming".into(), when: "game".into(), look: Look::default() },
        Profile { id: "desk".into(), name: "Desk".into(), when: "desktop".into(), look: Look { effect: "still".into(), colour: [0x5e, 0xc8, 0xff], speed: 40, brightness: 70, direction: "right".into() } },
        Profile { id: "movie".into(), name: "Movie night".into(), when: "hand".into(), look: Look { effect: "still".into(), colour: [0xb0, 0x4f, 0xc8], speed: 40, brightness: 25, direction: "right".into() } },
        Profile { id: "sleep".into(), name: "Sleep".into(), when: "sleep".into(), look: Look { effect: "off".into(), colour: [0, 0, 0], speed: 40, brightness: 0, direction: "right".into() } },
    ]
}

pub fn rgb_hex(c: Rgb) -> String {
    format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2])
}
pub fn hex_rgb(s: &str) -> Option<Rgb> {
    let s = s.trim().trim_start_matches('#');
    if s.len() != 6 {
        return None;
    }
    let v = u32::from_str_radix(s, 16).ok()?;
    Some([(v >> 16) as u8, (v >> 8) as u8, v as u8])
}

pub fn look_json(l: &Look) -> Value {
    json!({"effect": l.effect, "colour": rgb_hex(l.colour), "speed": l.speed, "brightness": l.brightness, "direction": l.direction})
}
fn look_of(v: &Value, base: &Look) -> Look {
    Look {
        effect: v.get("effect").and_then(Value::as_str).filter(|e| EFFECTS.iter().any(|x| x.0 == *e)).map(str::to_string).unwrap_or_else(|| base.effect.clone()),
        colour: v.get("colour").and_then(Value::as_str).and_then(hex_rgb).unwrap_or(base.colour),
        speed: v.get("speed").and_then(Value::as_u64).map(|x| x.min(100) as u8).unwrap_or(base.speed),
        brightness: v.get("brightness").and_then(Value::as_u64).map(|x| x.min(100) as u8).unwrap_or(base.brightness),
        direction: v.get("direction").and_then(Value::as_str).filter(|d| DIRECTIONS.iter().any(|x| x.0 == *d)).map(str::to_string).unwrap_or_else(|| base.direction.clone()),
    }
}

/// Everything the person chose (lights.json). Defaults fill in what isn't there; nothing here writes it.
#[derive(Debug, Clone)]
pub struct Choices {
    pub profiles: Vec<Profile>,
    /// the profile the panel changes (picked by hand)
    pub current: String,
    /// switch by itself: ask | yes | no
    pub switch: String,
    pub all_off: bool,
    /// devices turned off (their keys)
    pub off: Vec<String>,
    /// a device's lights painted one by one: key → zones → colours
    pub painted: serde_json::Map<String, Value>,
    pub zone_mode: String,
    /// lights only in this device (key), or "" for all
    pub only: String,
    /// a device given its own look (picked on the left, then changed): key → look
    pub own: serde_json::Map<String, Value>,
}

impl Choices {
    pub fn read() -> Choices {
        Choices::from(&crate::kit::read_settings(APP))
    }
    pub fn from(v: &Value) -> Choices {
        let mut profiles = default_profiles();
        if let Some(arr) = v.get("profiles").and_then(Value::as_array) {
            let mut out = Vec::new();
            for p in arr {
                let id = p.get("id").and_then(Value::as_str).unwrap_or("").to_string();
                if id.is_empty() || out.iter().any(|x: &Profile| x.id == id) {
                    continue;
                }
                let base = profiles.iter().find(|d| d.id == id).cloned();
                let look = look_of(p.get("look").unwrap_or(&Value::Null), &base.as_ref().map(|b| b.look.clone()).unwrap_or_default());
                out.push(Profile {
                    name: p.get("name").and_then(Value::as_str).map(str::to_string).or(base.as_ref().map(|b| b.name.clone())).unwrap_or_else(|| "Profile".into()),
                    when: p.get("when").and_then(Value::as_str).map(str::to_string).or(base.as_ref().map(|b| b.when.clone())).unwrap_or_else(|| "hand".into()),
                    id,
                    look,
                });
            }
            if !out.is_empty() {
                profiles = out;
            }
        }
        let current = v.get("current").and_then(Value::as_str).filter(|c| profiles.iter().any(|p| p.id == *c)).map(str::to_string).unwrap_or_else(|| profiles[0].id.clone());
        Choices {
            profiles,
            current,
            switch: v.get("switch").and_then(Value::as_str).filter(|s| matches!(*s, "ask" | "yes" | "no")).unwrap_or("ask").to_string(),
            all_off: v.get("allOff").and_then(Value::as_bool).unwrap_or(false),
            off: v.get("off").and_then(Value::as_array).map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default(),
            painted: v.get("painted").and_then(Value::as_object).cloned().unwrap_or_default(),
            zone_mode: v.get("zoneMode").and_then(Value::as_str).filter(|s| matches!(*s, "same" | "fan" | "light")).unwrap_or("same").to_string(),
            only: v.get("only").and_then(Value::as_str).unwrap_or("").to_string(),
            own: v.get("own").and_then(Value::as_object).cloned().unwrap_or_default(),
        }
    }
    pub fn profile(&self, id: &str) -> &Profile {
        self.profiles.iter().find(|p| p.id == id).unwrap_or(&self.profiles[0])
    }
    pub fn profiles_json(&self) -> Value {
        Value::Array(self.profiles.iter().map(|p| json!({"id": p.id, "name": p.name, "when": p.when, "look": look_json(&p.look)})).collect())
    }
    /// The look a device shows: its own (if it was given one), else the profile's.
    pub fn look_for(&self, key: &str, profile: &Look) -> Look {
        match self.own.get(key) {
            Some(v) => look_of(v, profile),
            None => profile.clone(),
        }
    }
    pub fn painted_of(&self, key: &str) -> Option<Vec<Vec<Rgb>>> {
        let z = self.painted.get(key)?.as_array()?;
        Some(z.iter().map(|l| l.as_array().map(|a| a.iter().map(|c| c.as_str().and_then(hex_rgb).unwrap_or([0, 0, 0])).collect()).unwrap_or_default()).collect())
    }
}

/// A device's lasting name (OpenRGB's name and where it's plugged in).
pub fn key_of(d: &Device) -> String {
    format!("{}@{}", d.name, d.location)
}

// ------------------------------------------------------------------ colours
fn lerp(a: Rgb, b: Rgb, t: f32) -> Rgb {
    let t = t.clamp(0.0, 1.0);
    [(a[0] as f32 + (b[0] as f32 - a[0] as f32) * t) as u8, (a[1] as f32 + (b[1] as f32 - a[1] as f32) * t) as u8, (a[2] as f32 + (b[2] as f32 - a[2] as f32) * t) as u8]
}
/// A colour along a looped list of colours (0..1 goes once round).
pub fn along(cols: &[Rgb], x: f32) -> Rgb {
    if cols.is_empty() {
        return [0, 0, 0];
    }
    let n = cols.len() as f32;
    let x = x.rem_euclid(1.0) * n;
    let i = x.floor() as usize % cols.len();
    lerp(cols[i], cols[(i + 1) % cols.len()], x - x.floor())
}
pub fn hsv(h: f32, s: f32, v: f32) -> Rgb {
    let h = h.rem_euclid(1.0) * 6.0;
    let i = h.floor();
    let f = h - i;
    let (p, q, t) = (v * (1.0 - s), v * (1.0 - s * f), v * (1.0 - s * (1.0 - f)));
    let (r, g, b) = match i as i32 {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    [(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8]
}
fn dim(c: Rgb, k: f32) -> Rgb {
    let k = k.clamp(0.0, 1.0);
    [(c[0] as f32 * k) as u8, (c[1] as f32 * k) as u8, (c[2] as f32 * k) as u8]
}

/// What Hearth's own effects follow: the processor's temperature, the sound playing, the game's colours.
#[derive(Debug, Clone, Default)]
pub struct Inputs {
    pub temp: f32,
    pub level: f32,
    pub game: Vec<Rgb>,
}

/// One light's colour at time `t` (seconds) at place `x` (0..1 along the effect's direction).
pub fn colour_at(l: &Look, t: f32, x: f32, i: &Inputs) -> Rgb {
    let sp = 0.05 + l.speed as f32 / 100.0 * 0.6;
    let c = match l.effect.as_str() {
        "off" => [0, 0, 0],
        "breathing" => dim(l.colour, 0.25 + 0.75 * (0.5 + 0.5 * (t * sp * std::f32::consts::TAU).sin())),
        "ember" => along(&EMBER, x - t * sp * 0.5),
        "rainbow" => hsv(x - t * sp * 0.5, 1.0, 1.0),
        "heat" => {
            let k = ((i.temp - 40.0) / 40.0).clamp(0.0, 1.0);
            if k < 0.5 { lerp([0x9b, 0xe3, 0x6b], [0xff, 0xd3, 0x6b], k * 2.0) } else { lerp([0xff, 0xd3, 0x6b], [0xff, 0x3d, 0x3d], (k - 0.5) * 2.0) }
        }
        "music" => dim(along(&EMBER, x), 0.15 + 0.85 * i.level.clamp(0.0, 1.0)),
        "game" => {
            if i.game.is_empty() { l.colour } else { along(&i.game, x - t * sp * 0.2) }
        }
        _ => l.colour,
    };
    dim(c, l.brightness as f32 / 100.0)
}

/// Does this effect need Hearth to send frames (it moves, and the device has no mode of its own for it)?
pub fn moves(effect: &str) -> bool {
    matches!(effect, "breathing" | "ember" | "rainbow" | "heat" | "music" | "game")
}

/// Where each light of a device is along the direction (0..1).
pub fn places(d: &Device, dir: &str) -> Vec<f32> {
    let n = d.leds.len().max(d.colors.len()).max(1);
    let mut out = Vec::with_capacity(n);
    for z in 0..d.zones.len() {
        let zn = &d.zones[z];
        for j in 0..zn.leds as usize {
            let x = if zn.rows > 0 && zn.cols > 0 {
                let (r, c) = (j as u32 / zn.cols, j as u32 % zn.cols);
                match dir {
                    "up" | "down" => r as f32 / zn.rows.max(1) as f32,
                    _ => c as f32 / zn.cols.max(1) as f32,
                }
            } else {
                j as f32 / zn.leds.max(1) as f32
            };
            out.push(x);
        }
    }
    while out.len() < n {
        out.push(out.len() as f32 / n as f32);
    }
    out.truncate(n);
    for x in out.iter_mut() {
        *x = match dir {
            "left" | "up" => 1.0 - *x,
            "centre" => (*x - 0.5).abs() * 2.0,
            _ => *x,
        };
    }
    out
}

/// A device's frame: every light's colour now (painted lights kept as painted while the effect is Still).
pub fn frame(d: &Device, l: &Look, t: f32, inp: &Inputs, painted: Option<&Vec<Vec<Rgb>>>) -> Vec<Rgb> {
    let xs = places(d, &l.direction);
    let mut v: Vec<Rgb> = xs.iter().map(|x| colour_at(l, t, *x, inp)).collect();
    if let (Some(p), "still") = (painted, l.effect.as_str()) {
        for (z, zone) in p.iter().enumerate() {
            let s = d.zone_start(z);
            for (j, c) in zone.iter().enumerate() {
                if let Some(x) = v.get_mut(s + j) {
                    *x = dim(*c, l.brightness as f32 / 100.0);
                }
            }
        }
    }
    v
}

/// How a device shows a look: one of its own modes (kept on it when it can keep it), or Hearth's frames.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Way {
    /// one of its modes; true when it's saved on the device
    Own(bool),
    /// Hearth's frames (Direct mode)
    Frames,
    /// it can't show it
    No,
}

fn speed_of(m: &orgb::Mode, pct: u8) -> u32 {
    let (a, b) = (m.speed_min as f32, m.speed_max as f32);
    (a + (b - a) * pct as f32 / 100.0).round() as u32
}
fn bright_of(m: &orgb::Mode, pct: u8) -> u32 {
    let (a, b) = (m.bright_min as f32, m.bright_max as f32);
    (a + (b - a) * pct as f32 / 100.0).round() as u32
}

/// Which of the device's own modes shows this look (None: Hearth's frames).
pub fn own_mode(d: &Device, l: &Look, painted: bool) -> Option<usize> {
    let pick = match l.effect.as_str() {
        "off" => d.mode(&["off"]),
        "still" if !painted && d.direct().is_none() => d.mode(&["static", "fixed", "solid", "still"]),
        "still" => None,
        "breathing" => d.mode(&["breath"]),
        "rainbow" => d.mode(&["rainbow", "spectrum", "color cycle", "colour cycle"]),
        "keys" => d.mode(&["reactive", "ripple", "react"]).or_else(|| d.mode(&["static", "fixed"])),
        _ => None,
    };
    pick.or_else(|| if l.effect == "still" && d.direct().is_none() { d.mode(&["static", "fixed", "solid"]) } else { None })
}

/// Show `l` on device `d` once (its own mode, or the first of Hearth's frames).
pub fn apply(c: &mut Conn, d: &Device, l: &Look, inp: &Inputs, painted: Option<&Vec<Vec<Rgb>>>, off: bool) -> std::io::Result<Way> {
    if off {
        let black = Look { effect: "off".into(), ..l.clone() };
        return apply(c, d, &black, inp, None, false);
    }
    if let Some(i) = own_mode(d, l, painted.is_some()) {
        let mut m = d.modes[i].clone();
        if m.flags & orgb::F_SPEED != 0 {
            m.speed = speed_of(&m, l.speed);
        }
        if m.flags & orgb::F_BRIGHTNESS != 0 {
            m.brightness = bright_of(&m, l.brightness);
        }
        if m.flags & (orgb::F_DIR_LR | orgb::F_DIR_UD | orgb::F_DIR_HV) != 0 {
            m.direction = match l.direction.as_str() {
                "left" => 0,
                "up" if m.flags & orgb::F_DIR_UD != 0 => 2,
                "down" if m.flags & orgb::F_DIR_UD != 0 => 3,
                "centre" if m.flags & orgb::F_DIR_HV != 0 => 4,
                _ => 1,
            };
        }
        if m.flags & orgb::F_MODE_COLOUR != 0 {
            let n = (m.colors_min.max(1) as usize).max(m.colors.len()).min(8);
            let col = if m.flags & orgb::F_BRIGHTNESS != 0 { l.colour } else { dim(l.colour, l.brightness as f32 / 100.0) };
            m.colors = vec![col; n];
            m.color_mode = 2;
        }
        let save = m.flags & orgb::F_MANUAL_SAVE != 0;
        c.set_mode(d.index, i, &m, save)?;
        return Ok(Way::Own(save || m.flags & (1 << 9) != 0));
    }
    if d.direct().is_some() || d.modes.iter().any(|m| m.flags & orgb::F_PER_LED != 0) {
        c.custom(d.index)?;
        c.leds(d.index, &frame(d, l, 0.0, inp, painted))?;
        return Ok(Way::Frames);
    }
    Ok(Way::No)
}

// ------------------------------------------------------------------ what's on now (the runner's file)
pub fn now_file() -> std::path::PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR").map(std::path::PathBuf::from).unwrap_or_else(std::env::temp_dir).join("hearth-lights-now.json")
}
pub fn now_read() -> Value {
    std::fs::read(now_file()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null)
}
fn now_write(v: &Value) {
    let f = now_file();
    let tmp = f.with_extension("tmp");
    if std::fs::write(&tmp, v.to_string()).is_ok() {
        let _ = std::fs::rename(&tmp, &f);
    }
}

/// Which profile's moment it is (None: the one picked by hand).
pub fn moment(ch: &Choices) -> Option<String> {
    let has = |w: &str| ch.profiles.iter().find(|p| p.when == w).map(|p| p.id.clone());
    if screen_asleep() {
        if let Some(p) = has("sleep") {
            return Some(p);
        }
    }
    if crate::kit::game_running() {
        return has("game");
    }
    if std::env::var("XDG_CURRENT_DESKTOP").map(|d| !d.to_lowercase().contains("gamescope")).unwrap_or(false) {
        return has("desktop");
    }
    None
}
fn screen_asleep() -> bool {
    // logind's idle hint for this session (the screen dims and sleeps after it)
    let id = std::env::var("XDG_SESSION_ID").unwrap_or_default();
    if id.is_empty() {
        return false;
    }
    std::process::Command::new("loginctl")
        .args(["show-session", &id, "-p", "IdleHint", "--value"])
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "yes")
        .unwrap_or(false)
}

// ------------------------------------------------------------------ the inputs
/// The processor's temperature (°C) from the kernel's sensors (k10temp, coretemp, zenpower).
pub fn cpu_temp() -> f32 {
    let Ok(rd) = std::fs::read_dir("/sys/class/hwmon") else { return 0.0 };
    for e in rd.flatten() {
        let n = std::fs::read_to_string(e.path().join("name")).unwrap_or_default();
        if matches!(n.trim(), "k10temp" | "coretemp" | "zenpower" | "cpu_thermal") {
            if let Ok(t) = std::fs::read_to_string(e.path().join("temp1_input")) {
                return t.trim().parse::<f32>().unwrap_or(0.0) / 1000.0;
            }
        }
    }
    0.0
}

/// The sound playing, 0..1 (PipeWire's pw-record on what's playing, at 8 kHz, one channel), while it's asked for.
pub struct Music {
    level: Arc<AtomicU32>,
    child: Option<std::process::Child>,
}
impl Music {
    pub fn start() -> Music {
        let level = Arc::new(AtomicU32::new(0));
        let child = std::process::Command::new("pw-record")
            .args(["--rate", "8000", "--channels", "1", "--format", "s16", "-P", "{ stream.capture.sink = true }", "-"])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .ok();
        let mut child = child;
        if let Some(out) = child.as_mut().and_then(|c| c.stdout.take()) {
            let l = level.clone();
            let _ = std::thread::Builder::new().name("hearth-lights-music".into()).spawn(move || {
                use std::io::Read;
                let mut out = out;
                let mut buf = [0u8; 800];
                while out.read_exact(&mut buf).is_ok() {
                    let mut peak = 0i32;
                    for s in buf.chunks(2) {
                        peak = peak.max((i16::from_le_bytes([s[0], s[1]]) as i32).abs());
                    }
                    l.store(((peak as f32 / 32768.0).min(1.0) * 1000.0) as u32, Ordering::Relaxed);
                }
            });
        }
        Music { level, child }
    }
    pub fn level(&self) -> f32 {
        self.level.load(Ordering::Relaxed) as f32 / 1000.0
    }
}
impl Drop for Music {
    fn drop(&mut self) {
        if let Some(c) = self.child.as_mut() {
            let _ = c.kill();
            let _ = c.wait();
        }
    }
}

/// The game in front's colours: its cover in Steam's library, three colours from left to right.
pub fn game_colours() -> Vec<Rgb> {
    let home = hearth_ui::paths::home();
    let steam = [home.join(".local/share/Steam"), home.join(".steam/steam")];
    let reg = std::fs::read_to_string(home.join(".steam/registry.vdf")).unwrap_or_default();
    let id = reg
        .lines()
        .find(|l| l.contains("\"RunningAppID\""))
        .and_then(|l| l.split('"').filter(|x| !x.trim().is_empty()).nth(1).map(str::to_string))
        .unwrap_or_default();
    if id.is_empty() || id == "0" {
        return Vec::new();
    }
    for s in &steam {
        for f in [s.join(format!("appcache/librarycache/{id}/library_600x900.jpg")), s.join(format!("appcache/librarycache/{id}_library_600x900.jpg"))] {
            if let Ok(img) = image::open(&f) {
                let small = img.thumbnail(12, 18).to_rgb8();
                let (w, h) = small.dimensions();
                let mut out = Vec::new();
                for part in 0..3u32 {
                    let (mut r, mut g, mut b, mut n) = (0u32, 0u32, 0u32, 0u32);
                    for y in 0..h {
                        for x in (part * w / 3)..((part + 1) * w / 3) {
                            let p = small.get_pixel(x, y);
                            r += p[0] as u32;
                            g += p[1] as u32;
                            b += p[2] as u32;
                            n += 1;
                        }
                    }
                    if n > 0 {
                        out.push([(r / n) as u8, (g / n) as u8, (b / n) as u8]);
                    }
                }
                return out;
            }
        }
    }
    Vec::new()
}

// ------------------------------------------------------------------ the runner
fn mtime(p: &std::path::Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(p).and_then(|m| m.modified()).ok()
}

/// `hearth-lights-app --run`: keeps the person's lights as they set them (hearth-lights.service).
pub fn run_forever() -> i32 {
    let path = crate::kit::settings_path(APP);
    let start = Instant::now();
    let mut seen = None;
    let mut ch = Choices::read();
    let mut conn: Option<Conn> = None;
    let mut devs: Vec<Device> = Vec::new();
    let mut shown: Option<(String, Look)> = None;
    let mut asked: Option<String> = None;
    let mut wait = Duration::from_secs(2);
    let mut last_devs = Instant::now() - Duration::from_secs(3600);
    let mut music: Option<Music> = None;
    let mut inp = Inputs::default();
    let mut last_in = Instant::now() - Duration::from_secs(60);
    loop {
        let m = mtime(&path);
        if m != seen {
            seen = m;
            ch = Choices::read();
            shown = None;
        }
        if conn.is_none() {
            match Conn::open() {
                Ok(c) => {
                    conn = Some(c);
                    wait = Duration::from_secs(2);
                    shown = None;
                    last_devs = Instant::now() - Duration::from_secs(3600);
                }
                Err(_) => {
                    std::thread::sleep(wait);
                    wait = (wait * 2).min(Duration::from_secs(60));
                    continue;
                }
            }
        }
        let c = conn.as_mut().unwrap();
        if last_devs.elapsed() > Duration::from_secs(30) {
            match c.devices() {
                Ok(d) => {
                    if d.len() != devs.len() {
                        shown = None;
                    }
                    devs = d;
                    last_devs = Instant::now();
                }
                Err(_) => {
                    conn = None;
                    continue;
                }
            }
        }
        // the profile for this moment (by itself only if the person said so: yes, or ask and they said yes)
        let mut pid = ch.current.clone();
        if let Some(mo) = moment(&ch) {
            match ch.switch.as_str() {
                "yes" => pid = mo,
                "ask" => {
                    if asked.as_deref() != Some(mo.as_str()) {
                        asked = Some(mo.clone());
                        let name = ch.profile(&mo).name.clone();
                        let mo2 = mo.clone();
                        let _ = std::thread::Builder::new().name("hearth-lights-ask".into()).spawn(move || ask(&name, &mo2));
                    }
                    if now_read().get("agreed").and_then(Value::as_str) == Some(mo.as_str()) {
                        pid = mo;
                    }
                }
                _ => {}
            }
        } else {
            asked = None;
        }
        let look = ch.profile(&pid).look.clone();
        let want = (pid.clone(), look.clone());
        if look.effect == "music" && music.is_none() {
            music = Some(Music::start());
        } else if look.effect != "music" {
            music = None;
        }
        if last_in.elapsed() > Duration::from_secs(2) {
            inp.temp = cpu_temp();
            if look.effect == "game" {
                inp.game = game_colours();
            }
            last_in = Instant::now();
        }
        if let Some(mu) = music.as_ref() {
            inp.level = inp.level * 0.6 + mu.level() * 0.4;
        }
        let mut framed = false;
        if shown.as_ref() != Some(&want) {
            let mut ways = serde_json::Map::new();
            let mut bad = false;
            for d in &devs {
                let k = key_of(d);
                let off = ch.all_off || ch.off.contains(&k) || (!ch.only.is_empty() && ch.only != k);
                let pt = ch.painted_of(&k);
                match apply(c, d, &ch.look_for(&k, &look), &inp, pt.as_ref(), off) {
                    Ok(w) => {
                        ways.insert(k, json!(match w { Way::Own(true) => "saved", Way::Own(false) => "own", Way::Frames => "frames", Way::No => "none" }));
                    }
                    Err(_) => bad = true,
                }
            }
            if bad {
                conn = None;
                continue;
            }
            let mut now = now_read();
            if !now.is_object() {
                now = json!({});
            }
            now["profile"] = json!(pid);
            now["ways"] = Value::Object(ways);
            now["at"] = json!(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0));
            now_write(&now);
            shown = Some(want);
        } else if (moves(&look.effect) || !ch.own.is_empty()) && !ch.all_off {
            // Hearth's own frames, for the devices that have no mode of their own for it
            let t = start.elapsed().as_secs_f32();
            for d in &devs {
                let k = key_of(d);
                let lk = ch.look_for(&k, &look);
                if ch.off.contains(&k) || (!ch.only.is_empty() && ch.only != k) || !moves(&lk.effect) || own_mode(d, &lk, false).is_some() {
                    continue;
                }
                if c.leds(d.index, &frame(d, &lk, t, &inp, None)).is_err() {
                    conn = None;
                    break;
                }
                framed = true;
            }
        }
        std::thread::sleep(if framed { Duration::from_millis(50) } else { Duration::from_millis(1000) });
    }
}

/// "A game started: switch to Gaming?" (Switch by itself: ask me). Yes is remembered only until sign-out.
fn ask(name: &str, id: &str) {
    let o = std::process::Command::new("notify-send")
        .args(["-a", "Hearth Lights", "-A", "yes=Switch", "-A", "no=Not now", "-w", "Hearth Lights", &format!("Switch the lights to {name}?")])
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output();
    if let Ok(o) = o {
        if String::from_utf8_lossy(&o.stdout).trim() == "yes" {
            let mut now = now_read();
            if !now.is_object() {
                now = json!({});
            }
            now["agreed"] = json!(id);
            now_write(&now);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn choices_fill_in_what_isnt_there_and_keep_what_is() {
        let c = Choices::from(&json!({}));
        assert_eq!(c.profiles.len(), 4);
        assert_eq!(c.current, "gaming");
        assert_eq!(c.switch, "ask");
        let c = Choices::from(&json!({"profiles": [{"id": "gaming", "look": {"effect": "rainbow", "speed": 70}}, {"id": "mine", "name": "Mine", "when": "hand"}], "current": "mine", "switch": "yes"}));
        assert_eq!(c.profiles.len(), 2);
        assert_eq!(c.profile("gaming").look.effect, "rainbow");
        assert_eq!(c.profile("gaming").look.brightness, 80, "the rest of the look is the default's");
        assert_eq!(c.current, "mine");
        let back = Choices::from(&json!({"profiles": c.profiles_json(), "current": "mine"}));
        assert_eq!(back.profiles, c.profiles);
    }

    #[test]
    fn effects_stay_in_their_colours() {
        let l = Look::default();
        let i = Inputs::default();
        for k in 0..20 {
            let c = colour_at(&l, k as f32 * 0.3, k as f32 / 20.0, &i);
            assert!(c[0] > c[1], "ember stays warm: {c:?}");
        }
        let off = Look { effect: "off".into(), ..Look::default() };
        assert_eq!(colour_at(&off, 1.0, 0.5, &i), [0, 0, 0]);
        assert_eq!(hex_rgb(&rgb_hex([1, 2, 255])), Some([1, 2, 255]));
    }
}
