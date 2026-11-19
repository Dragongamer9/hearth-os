//! The person's chosen look, the same way Mantel's Theme.qml and theme/hearth-theme work it out:
//! the look (desktop.json "look", else Steam-style), dark or light ("mode", with Sunset), the sub theme ("subTheme",
//! subthemes.js's rules) and the accent ("accent", else the look's own; Steam-style's Sky on new installs).
//! The colours come from palettes.json: the system's (/usr/share/nimbus/desktop/theme/palettes.json), else the copy
//! built into the app. Reading only: an app never writes desktop.json.
//!
//! On Plasma the colours are the Global Theme's (see [`apps_theme_json`]).
//!
//! [`watch`] gives a stream that says when desktop.json, palettes.json or theme.json changed (inotify, no polling), so an open
//! app follows a new look at once and does nothing at all while nothing changes.

use iced::Color;
use serde_json::{Map, Value};
use std::path::PathBuf;

const BUILT_IN: &str = include_str!("../../linux/desktop/theme/palettes.json");
const TOKENS: [&str; 12] = ["bg", "bg2", "panel", "panel2", "panel3", "line", "text", "muted", "ember", "flame", "third", "ink"];

/// One look's colours, ready to draw with.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pal {
    pub bg: Color,
    pub bg2: Color,
    pub panel: Color,
    pub panel2: Color,
    pub panel3: Color,
    pub line: Color,
    pub text: Color,
    pub muted: Color,
    pub danger: Color,
    /// The accent in use (yours, the sub theme's or the look's own).
    pub accent: Color,
    /// Text on the accent.
    pub ink: Color,
    pub flame: Color,
    pub good: Color,
    pub warn: Color,
    pub dark: bool,
    /// The look's largest corner (0: none), and its windows' corner.
    pub corner: f32,
    pub window_corner: f32,
}

impl Pal {
    /// A corner of `r` px, capped by the look (Steam-style is square-ish).
    pub fn r(&self, r: f32) -> f32 {
        if self.corner > 0.0 { r.min(self.corner.max(2.0)) } else { r }
    }
    /// `c` at this opacity.
    pub fn a(c: Color, a: f32) -> Color {
        Color { a, ..c }
    }
    /// Between two colours (0 = a, 1 = b).
    pub fn mix(a: Color, b: Color, t: f32) -> Color {
        Color { r: a.r + (b.r - a.r) * t, g: a.g + (b.g - a.g) * t, b: a.b + (b.b - a.b) * t, a: a.a + (b.a - a.a) * t }
    }
    /// A row that's picked or hovered.
    pub fn sel(&self) -> Color {
        Pal::mix(self.panel2, self.accent, if self.dark { 0.22 } else { 0.16 })
    }
    pub fn hover(&self) -> Color {
        Pal::mix(self.bg2, self.text, 0.05)
    }
}

pub fn hex(s: &str) -> Option<Color> {
    let s = s.strip_prefix('#')?;
    if s.len() != 6 || !s.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let v = u32::from_str_radix(s, 16).ok()?;
    Some(Color::from_rgb8((v >> 16) as u8, (v >> 8) as u8, v as u8))
}

fn home() -> PathBuf {
    crate::paths::home()
}

/// ~/.local/share/nimbus/desktop.json (Mantel's Settings.qml and hearth-plasma-looks sync keep it).
pub fn desktop_json() -> PathBuf {
    std::env::var_os("HEARTH_DESKTOP_JSON").map(PathBuf::from).unwrap_or_else(|| {
        std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|| home().join(".local/share")).join("nimbus/desktop.json")
    })
}

pub fn palettes_json() -> PathBuf {
    std::env::var_os("HEARTH_PALETTES").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/usr/share/nimbus/desktop/theme/palettes.json"))
}

fn read_obj(p: &PathBuf) -> Option<Map<String, Value>> {
    match serde_json::from_slice::<Value>(&std::fs::read(p).ok()?).ok()? {
        Value::Object(m) => Some(m),
        _ => None,
    }
}

/// palettes.json as a map (the system's, else the copy built in): Settings › Personalization lists the looks from it.
pub fn palette_map() -> Map<String, Value> {
    palettes()
}

fn palettes() -> Map<String, Value> {
    read_obj(&palettes_json()).unwrap_or_else(|| match serde_json::from_str::<Value>(BUILT_IN) {
        Ok(Value::Object(m)) => m,
        _ => Map::new(),
    })
}

fn s<'a>(m: &'a Map<String, Value>, k: &str) -> &'a str {
    m.get(k).and_then(Value::as_str).unwrap_or("")
}

/// subthemes.js `list` + `find`: the sub theme in use, or None for the look's own colours.
fn sub_theme<'a>(pal: &'a Map<String, Value>, look: &str, id: &str) -> Option<&'a Map<String, Value>> {
    let l = pal.get(look)?.as_object()?;
    let own = l.get("subThemes")?.as_array()?;
    let mut list: Vec<&Map<String, Value>> = Vec::new();
    for x in own {
        if let Some(o) = x.as_object() {
            if o.get("id").and_then(Value::as_str).map(|i| !i.is_empty()).unwrap_or(false) {
                list.push(o);
            }
        }
    }
    if list.is_empty() {
        return None;
    }
    if let (Some(bw), Some(set)) = (l.get("blackWallpapers").and_then(Value::as_object), pal.get("blackSet").and_then(Value::as_array)) {
        for b in set {
            if let Some(o) = b.as_object() {
                if let Some(bid) = o.get("id").and_then(Value::as_str) {
                    if bw.contains_key(bid) {
                        list.push(o);
                    }
                }
            }
        }
    }
    let found = list.iter().position(|o| o.get("id").and_then(Value::as_str) == Some(id)).unwrap_or(0);
    let own_first = found == 0 || list[found].get("own").and_then(Value::as_bool) == Some(true);
    if own_first { None } else { Some(list[found]) }
}

fn minutes(t: &str, d: i64) -> i64 {
    let mut it = t.split(':');
    match (it.next().and_then(|h| h.parse::<i64>().ok()), it.next().and_then(|m| m.parse::<i64>().ok())) {
        (Some(h), Some(m)) if (0..24).contains(&h) && (0..60).contains(&m) => h * 60 + m,
        _ => d,
    }
}

fn local_minutes() -> i64 {
    // Local time without a time-zone database: libc's localtime_r.
    unsafe {
        let now = libc::time(std::ptr::null_mut());
        let mut tm: libc::tm = std::mem::zeroed();
        if libc::localtime_r(&now, &mut tm).is_null() {
            return 12 * 60;
        }
        tm.tm_hour as i64 * 60 + tm.tm_min as i64
    }
}

static DEFAULT_LOOK: std::sync::OnceLock<&'static str> = std::sync::OnceLock::new();

/// The look used when none is chosen (or the chosen one isn't in palettes.json): Steam-style, unless the program
/// says otherwise before its first `load` (Firehold's shell: Hearth's own warm look, the one its mockups are drawn in).
pub fn set_default_look(look: &'static str) {
    let _ = DEFAULT_LOOK.set(look);
}

/// ~/.config/hearth/theme.json: what Hearth's apps draw with. On Plasma, hearth-plasma-looks writes it from the Global
/// Theme and colours picked in System Settings ("source": "plasma"), so every Hearth app follows KDE's choice, Hearth's
/// looks and any other theme alike.
pub fn apps_theme_json() -> PathBuf {
    std::env::var_os("HEARTH_APPS_THEME").map(PathBuf::from).unwrap_or_else(|| {
        std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| home().join(".config")).join("hearth/theme.json")
    })
}

fn on_plasma() -> bool {
    std::env::var("XDG_CURRENT_DESKTOP").map(|d| d.split(':').any(|x| x.eq_ignore_ascii_case("KDE"))).unwrap_or(false)
        || std::env::var_os("HEARTH_APPS_THEME").is_some()
}

/// The look Plasma shows (theme.json written by hearth-plasma-looks), or None.
fn from_plasma() -> Option<Pal> {
    if !on_plasma() {
        return None;
    }
    let t = read_obj(&apps_theme_json())?;
    if s(&t, "source") != "plasma" {
        return None;
    }
    let c = t.get("colors")?.as_object()?;
    let g = |k: &str| c.get(k).and_then(Value::as_str).and_then(hex);
    let dark = s(&t, "mode") != "light";
    let bg = g("bg")?;
    let text = g("text")?;
    let accent = g("ember").or_else(|| g("accent"))?;
    let corner = t.get("corner").and_then(Value::as_f64).map(|v| v as f32).filter(|v| *v < 900.0).unwrap_or(0.0);
    Some(Pal {
        bg,
        bg2: g("bg2").unwrap_or(bg),
        panel: g("panel").unwrap_or(bg),
        panel2: g("panel2").unwrap_or(bg),
        panel3: g("panel3").unwrap_or(bg),
        line: g("line").unwrap_or_else(|| Pal::mix(bg, text, 0.18)),
        text,
        muted: g("muted").unwrap_or_else(|| Pal::mix(text, bg, 0.4)),
        danger: g("danger").unwrap_or_else(|| hex(if dark { "#ff7a6e" } else { "#c9363b" }).unwrap()),
        accent,
        ink: g("ink").unwrap_or(Color::WHITE),
        flame: g("flame").unwrap_or(accent),
        good: if dark { hex("#7fe0a4").unwrap() } else { hex("#1f8a4c").unwrap() },
        warn: if dark { hex("#f2c14e").unwrap() } else { hex("#a86a00").unwrap() },
        dark,
        corner,
        window_corner: t.get("radius").and_then(Value::as_f64).map(|v| v as f32).unwrap_or(10.0),
    })
}

/// The look as it is now.
pub fn load() -> Pal {
    if let Some(p) = from_plasma() {
        return p;
    }
    let pal = palettes();
    let d = read_obj(&desktop_json()).unwrap_or_default();
    let mut look = s(&d, "look").to_string();
    if !pal.get(&look).map(Value::is_object).unwrap_or(false) || look.starts_with('_') || look == "accents" || look == "blackSet" {
        look = DEFAULT_LOOK.get().copied().filter(|l| pal.get(*l).map(Value::is_object).unwrap_or(false)).unwrap_or("steam").into();
    }
    // Sunset: light between sunsetLight and sunsetDark (never written; Settings.mode stays as it is).
    let mut mode = if s(&d, "mode") == "light" { "light" } else { "dark" };
    if d.get("sunset").and_then(Value::as_bool) == Some(true) {
        let (a, b, m) = (minutes(s(&d, "sunsetLight"), 420), minutes(s(&d, "sunsetDark"), 1170), local_minutes());
        let light = if a <= b { m >= a && m < b } else { m >= a || m < b };
        mode = if light { "light" } else { "dark" };
    }
    let empty = Map::new();
    let l = pal.get(&look).and_then(Value::as_object).unwrap_or(&empty);
    let base = l.get(mode).and_then(Value::as_object).unwrap_or(&empty);
    let base_dark = l.get("dark").and_then(Value::as_object).unwrap_or(&empty);
    let sub = sub_theme(&pal, &look, s(&d, "subTheme"));
    let pick = |k: &str, fallback: &str| -> Color {
        if let Some(st) = sub {
            if TOKENS.contains(&k) {
                if let Some(c) = st.get(mode).and_then(Value::as_object).and_then(|m| m.get(k)).and_then(Value::as_str).and_then(hex) {
                    return c;
                }
            }
        }
        base.get(k).and_then(Value::as_str).and_then(hex).unwrap_or_else(|| hex(fallback).unwrap())
    };
    let dark = mode == "dark";
    let ember = pick("ember", "#ff8a3d");
    let chosen = hex(s(&d, "accent"));
    let defaults_gen = d.get("defaults").and_then(Value::as_i64).unwrap_or(0);
    let sky = look == "steam" && sub.is_none() && defaults_gen >= 2;
    let accent = chosen.unwrap_or(if sky { hex("#66c0f4").unwrap() } else { ember });
    let ink = match (sub, chosen) {
        (Some(st), None) => st.get(mode).and_then(Value::as_object).and_then(|m| m.get("ink")).and_then(Value::as_str).and_then(hex),
        _ => None,
    }
    .unwrap_or_else(|| {
        let lum = accent.r * 0.2126 + accent.g * 0.7152 + accent.b * 0.0722;
        if lum > 0.45 {
            base_dark.get("ink").and_then(Value::as_str).and_then(hex).unwrap_or(Color::from_rgb8(0x2a, 0x12, 0x06))
        } else {
            Color::WHITE
        }
    });
    let num = |k: &str, d: f32| l.get(k).and_then(Value::as_f64).map(|v| v as f32).unwrap_or(d);
    Pal {
        bg: pick("bg", "#171a21"),
        bg2: pick("bg2", "#1b2838"),
        panel: pick("panel", "#1e2633"),
        panel2: pick("panel2", "#24303f"),
        panel3: pick("panel3", "#2a475e"),
        line: pick("line", "#32404f"),
        text: pick("text", "#dce5ed"),
        muted: pick("muted", "#8f98a0"),
        danger: pick("danger", if dark { "#ff7a6e" } else { "#c9363b" }),
        accent,
        ink,
        flame: pick("flame", "#ffd166"),
        good: if dark { hex("#7fe0a4").unwrap() } else { hex("#1f8a4c").unwrap() },
        warn: if dark { hex("#f2c14e").unwrap() } else { hex("#a86a00").unwrap() },
        dark,
        corner: num("corner", 0.0),
        window_corner: num("windowCorner", 10.0),
    }
}

/// A stream that yields () when the look may have changed: desktop.json or palettes.json was written. One blocked
/// thread reading inotify; nothing runs while nothing changes. A folder that isn't there yet is looked for again
/// once a minute (only until it's there).
pub fn watch() -> futures::channel::mpsc::UnboundedReceiver<()> {
    let (tx, rx) = futures::channel::mpsc::unbounded();
    let _ = std::thread::Builder::new().name("hearth-ui-theme".into()).stack_size(64 * 1024).spawn(move || watch_loop(tx));
    rx
}

fn watch_loop(tx: futures::channel::mpsc::UnboundedSender<()>) {
    let files = [desktop_json(), palettes_json(), apps_theme_json()];
    loop {
        let fd = unsafe { libc::inotify_init1(libc::IN_CLOEXEC) };
        if fd < 0 {
            return;
        }
        let mut names: Vec<String> = Vec::new();
        let mut watched = 0;
        for f in &files {
            let (Some(dir), Some(name)) = (f.parent(), f.file_name()) else { continue };
            names.push(name.to_string_lossy().to_string());
            let c = std::ffi::CString::new(dir.to_string_lossy().as_bytes()).unwrap_or_default();
            let mask = libc::IN_CLOSE_WRITE | libc::IN_MOVED_TO | libc::IN_CREATE | libc::IN_DELETE;
            if unsafe { libc::inotify_add_watch(fd, c.as_ptr(), mask) } >= 0 {
                watched += 1;
            }
        }
        let all = watched == files.len();
        if watched == 0 {
            unsafe { libc::close(fd) };
            std::thread::sleep(std::time::Duration::from_secs(60));
            continue;
        }
        let mut buf = [0u8; 4096];
        loop {
            // A folder that's missing is looked for again in a minute: poll() with a timeout only then.
            let mut pfd = libc::pollfd { fd, events: libc::POLLIN, revents: 0 };
            let r = unsafe { libc::poll(&mut pfd, 1, if all { -1 } else { 60_000 }) };
            if r == 0 {
                break; // look for the missing folder again
            }
            if r < 0 {
                if std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted {
                    continue;
                }
                unsafe { libc::close(fd) };
                return;
            }
            let n = unsafe { libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
            if n <= 0 {
                continue;
            }
            let mut i = 0usize;
            let mut hit = false;
            let head = std::mem::size_of::<libc::inotify_event>();
            while i + head <= n as usize {
                let len = u32::from_ne_bytes([buf[i + 12], buf[i + 13], buf[i + 14], buf[i + 15]]) as usize;
                let name_bytes = &buf[i + head..(i + head + len).min(n as usize)];
                let name = String::from_utf8_lossy(name_bytes).trim_end_matches('\0').to_string();
                if names.iter().any(|x| *x == name) {
                    hit = true;
                }
                i += head + len;
            }
            if hit && tx.unbounded_send(()).is_err() {
                unsafe { libc::close(fd) };
                return;
            }
        }
        unsafe { libc::close(fd) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steam_is_the_default_and_hearth_reads_its_own() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("desktop.json");
        std::env::set_var("HEARTH_DESKTOP_JSON", &p);
        std::env::set_var("HEARTH_PALETTES", d.path().join("none.json"));
        let steam = load();
        assert_eq!(steam.bg, hex("#171a21").unwrap());
        std::fs::write(&p, r##"{"look": "hearth", "mode": "dark", "accent": "#00ff00"}"##).unwrap();
        let h = load();
        assert_eq!(h.bg, hex("#0f0b09").unwrap());
        assert_eq!(h.accent, hex("#00ff00").unwrap());
        std::fs::write(&p, r##"{"look": "steam", "mode": "light"}"##).unwrap();
        assert!(!load().dark);
        std::fs::write(&p, "not json").unwrap();
        assert_eq!(load().bg, hex("#171a21").unwrap(), "a broken file reads as the defaults, and is never written");
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "not json");
    }
}
