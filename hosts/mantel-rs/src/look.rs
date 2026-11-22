//! Mantel's look in Rust, worked out exactly as the QML Mantel's Theme.qml does: the palette (theme/palettes.json and
//! desktop.json through hearth-ui, plus the Hearth look's profile accent from library.json), the panels' see-through
//! amount (Settings › Appearance › Panel transparency, else the look's "glass"), the look's font, its corner cap
//! (Theme.r), the bar style (islandStyle: floating islands, one full-width bar, Zen's thin bar, Console's shelf) and
//! Hearth's own one-colour icons (shell/icon-data.js, drawn in exactly the colour asked for, as Icon.qml does).
//! Read once, and again only when one of those files is written.

use iced::widget::{svg, text, Text};
use iced::{font, Color, Font};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock, RwLock};

pub use hearth_ui::theme::Pal;

/// Everything Theme.qml works out that the bar, dock and menu draw with.
#[derive(Clone, Debug, PartialEq)]
pub struct Look {
    pub p: Pal,
    pub name: String,
    /// "floating" | "bar" | "shelf" (Theme.islandStyle)
    pub style: String,
    /// Zen's thin bar
    pub thin: bool,
    /// Theme.glassAlpha
    pub glass_a: f32,
    pub font: Font,
    /// "plain" | "chamfer" | "neon" | "wood" | "frost"
    pub frame: String,
    pub hints: bool,
    /// desktop.json: the 24-hour clock, Game Mode off on this PC, the wallpaper you chose ("" = the look's own)
    pub clock24: bool,
    pub game_mode_off: bool,
    pub wall: String,
    pub reduce_motion: bool,
}

impl Look {
    /// Theme.r: a corner of n px, capped by the look (Steam-style 4 px).
    pub fn r(&self, n: f32) -> f32 {
        if self.p.corner > 0.0 {
            n.min(self.p.corner)
        } else {
            n
        }
    }
    /// Theme.glass: the panel colour at the panels' see-through amount.
    pub fn glass(&self) -> Color {
        a(self.p.panel, self.glass_a)
    }
    pub fn gaming(&self) -> bool {
        self.name == "gaming"
    }
    pub fn semi(&self) -> Font {
        Font { weight: font::Weight::Semibold, ..self.font }
    }
    pub fn bold(&self) -> Font {
        Font { weight: font::Weight::Bold, ..self.font }
    }
    /// Text in the look's font.
    pub fn t<'a>(&self, s: impl text::IntoFragment<'a>, size: f32, c: Color) -> Text<'a> {
        text(s).size(size).color(c).font(self.font)
    }
    /// DemiBold text (Font.DemiBold).
    pub fn ts<'a>(&self, s: impl text::IntoFragment<'a>, size: f32, c: Color) -> Text<'a> {
        text(s).size(size).color(c).font(self.semi())
    }
    /// Bold text (font.bold).
    pub fn tb<'a>(&self, s: impl text::IntoFragment<'a>, size: f32, c: Color) -> Text<'a> {
        text(s).size(size).color(c).font(self.bold())
    }
}

pub fn a(c: Color, alpha: f32) -> Color {
    Color { a: alpha, ..c }
}
/// Qt.lighter(c, f)
pub fn lighter(c: Color, f: f32) -> Color {
    // Qt lightens in HSV: value × f (and saturation down when value would pass 1); close enough for an accent.
    let m = c.r.max(c.g).max(c.b);
    if m <= 0.0 {
        return c;
    }
    let k = (m * f).min(1.0) / m;
    let over = (m * f - 1.0).max(0.0);
    let lift = |x: f32| (x * k + over * (1.0 - x * k)).min(1.0);
    Color { r: lift(c.r), g: lift(c.g), b: lift(c.b), a: c.a }
}

static LOOK: RwLock<Option<Look>> = RwLock::new(None);

fn obj(p: &std::path::Path) -> Map<String, Value> {
    std::fs::read(p).ok().and_then(|b| serde_json::from_slice::<Value>(&b).ok()).and_then(|v| v.as_object().cloned()).unwrap_or_default()
}

fn home() -> std::path::PathBuf {
    std::env::var_os("HOME").map(std::path::PathBuf::from).unwrap_or_default()
}

fn font_named(name: &str) -> Font {
    static NAMES: OnceLock<Mutex<HashMap<String, &'static str>>> = OnceLock::new();
    match name {
        "" | "sans-serif" | "sans" => Font::DEFAULT,
        "monospace" => Font::MONOSPACE,
        n => {
            let m = NAMES.get_or_init(|| Mutex::new(HashMap::new()));
            let mut g = m.lock().unwrap_or_else(|e| e.into_inner());
            let s: &'static str = g.entry(n.to_string()).or_insert_with(|| Box::leak(n.to_string().into_boxed_str()));
            Font::with_name(s)
        }
    }
}

pub fn mono() -> Font {
    font_named("DejaVu Sans Mono")
}

fn read() -> Look {
    let mut p = hearth_ui::theme::load();
    let d = obj(&hearth_ui::theme::desktop_json());
    let map = hearth_ui::theme::palette_map();
    let s = |k: &str| d.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    let mut name = s("look");
    if !map.get(&name).map(Value::is_object).unwrap_or(false) || name.starts_with('_') || name == "accents" || name == "blackSet" {
        name = "steam".into();
    }
    let empty = Map::new();
    let def = map.get(&name).and_then(Value::as_object).unwrap_or(&empty);
    let mode = if p.dark { "dark" } else { "light" };
    let sub = s("subTheme");
    let sub_own = sub.is_empty()
        || def.get("subThemes").and_then(Value::as_array).map(|l| l.iter().any(|t| t.get("id").and_then(Value::as_str) == Some(sub.as_str()) && t.get("own").and_then(Value::as_bool) == Some(true))).unwrap_or(true)
        || !def.get("subThemes").and_then(Value::as_array).map(|l| l.iter().any(|t| t.get("id").and_then(Value::as_str) == Some(sub.as_str()))).unwrap_or(false);
    // The Hearth look: your profile's accent (library.json), when you haven't picked one.
    let chosen = hearth_ui::theme::hex(&s("accent")).is_some();
    if name == "hearth" && sub_own && !chosen {
        let lib = obj(&home().join(".local/share/nimbus/library.json"));
        if let Some(c) = lib.get("profile").and_then(|x| x.get("accent")).and_then(Value::as_str).and_then(hearth_ui::theme::hex) {
            p.accent = c;
            let lum = c.r * 0.2126 + c.g * 0.7152 + c.b * 0.0722;
            p.ink = if lum > 0.45 {
                def.get("dark").and_then(|x| x.get("ink")).and_then(Value::as_str).and_then(hearth_ui::theme::hex).unwrap_or(Color::from_rgb8(0x2a, 0x12, 0x06))
            } else {
                Color::WHITE
            };
        }
    }
    let look_glass = def.get(mode).and_then(|m| m.get("glass")).and_then(Value::as_f64).map(|v| v as f32).filter(|v| *v > 0.0).unwrap_or(0.92);
    let glass_a = match s("panelGlass").as_str() {
        "solid" => 0.985,
        "frosted" => 0.9,
        "clear" => 0.62,
        _ => look_glass,
    };
    let look_islands = def.get("islandStyle").and_then(Value::as_str).filter(|v| ["floating", "bar", "shelf"].contains(v)).map(str::to_string).unwrap_or_else(|| if name == "steam" { "bar".into() } else { "floating".into() });
    let bar_style = s("barStyle");
    let style = if bar_style == "floating" || bar_style == "bar" { bar_style.clone() } else { look_islands };
    let thin = style == "bar" && def.get("thin").and_then(Value::as_bool) == Some(true) && bar_style != "bar";
    let frame = def.get("frame").and_then(Value::as_str).filter(|v| ["plain", "chamfer", "neon", "wood", "frost"].contains(v)).unwrap_or("plain").to_string();
    // Theme.wall: yours, else the sub theme's (Match sub theme on), else the look's own.
    let mut wall = s("wallpaper");
    if wall.is_empty() {
        let matched = d.get("wallMatch").and_then(Value::as_bool) != Some(false);
        let sub_wall = if matched && !sub.is_empty() {
            def.get("subThemes").and_then(Value::as_array).and_then(|l| l.iter().find(|t| t.get("id").and_then(Value::as_str) == Some(sub.as_str()))).and_then(|t| t.get("wallpaper")).and_then(Value::as_str).map(str::to_string)
        } else {
            None
        };
        wall = sub_wall.unwrap_or_else(|| def.get("wallpaper").and_then(Value::as_str).unwrap_or("").to_string());
    }
    Look {
        font: font_named(def.get("font").and_then(Value::as_str).unwrap_or("sans-serif")),
        hints: def.get("hints").and_then(Value::as_bool) == Some(true),
        clock24: d.get("clock24").and_then(Value::as_bool) == Some(true),
        game_mode_off: d.get("gameModeOff").and_then(Value::as_bool) == Some(true),
        reduce_motion: d.get("reduceMotion").and_then(Value::as_bool) == Some(true),
        p,
        name,
        style,
        thin,
        glass_a,
        frame,
        wall,
    }
}

/// The look in use.
pub fn get() -> Look {
    if let Some(v) = LOOK.read().ok().and_then(|g| g.clone()) {
        return v;
    }
    let v = read();
    if let Ok(mut g) = LOOK.write() {
        *g = Some(v.clone());
    }
    v
}

pub fn pal() -> Pal {
    get().p
}
pub fn look_name() -> String {
    get().name
}

/// desktop.json or palettes.json was written: read the look again; true when it changed.
pub fn reload() -> bool {
    let v = read();
    match LOOK.write() {
        Ok(mut g) => {
            let changed = g.as_ref() != Some(&v);
            *g = Some(v);
            changed
        }
        Err(_) => false,
    }
}

/// A stream that wakes the shell when the look may have changed (hearth-ui's inotify watch: asleep otherwise).
pub fn changes() -> impl futures::Stream<Item = ()> {
    use futures::StreamExt;
    hearth_ui::theme::watch().filter(|_| futures::future::ready(reload()))
}

/// Where Hearth's desktop files are (the QML Mantel's folder: its icons, pictures and wallpapers).
pub fn share() -> std::path::PathBuf {
    if let Some(s) = std::env::var_os("NIMBUS_SHARE") {
        return std::path::PathBuf::from(s);
    }
    for p in ["/usr/share/nimbus", "/usr/local/share/nimbus"] {
        if std::path::Path::new(p).join("desktop").exists() {
            return p.into();
        }
    }
    "/usr/share/nimbus".into()
}

pub fn shell_file(rel: &str) -> std::path::PathBuf {
    share().join("desktop/shell").join(rel)
}

// ---------------------------------------------------------------- Hearth's own icons (Icon.qml)

/// The freedesktop names used around the shell, to Hearth's own (Icon.qml's table).
fn own_name(n: &str) -> &str {
    match n {
        "open-menu-symbolic" => "menu",
        "input-keyboard-symbolic" => "kbd",
        "input-gaming-symbolic" => "pad",
        "view-app-grid-symbolic" => "grid",
        "window-close-symbolic" => "close",
        "audio-volume-high-symbolic" | "audio-volume-medium-symbolic" => "vol",
        "audio-volume-low-symbolic" => "vol-low",
        "audio-volume-muted-symbolic" => "vol-mute",
        "network-wireless-symbolic" | "network-wireless-signal-good-symbolic" => "wifi",
        "network-wired-symbolic" => "wired",
        "network-offline-symbolic" => "offline",
        "emblem-system-symbolic" => "gear",
        x => x,
    }
}

fn icon_data() -> &'static HashMap<String, String> {
    static DATA: OnceLock<HashMap<String, String>> = OnceLock::new();
    DATA.get_or_init(|| {
        let t = std::fs::read_to_string(shell_file("icon-data.js")).unwrap_or_default();
        parse_icon_data(&t)
    })
}

/// icon-data.js ("var icons = { "name": "<svg …{c}…>", … };") as name → SVG with {c} for the colour.
pub fn parse_icon_data(t: &str) -> HashMap<String, String> {
    let Some(start) = t.find("var icons") else { return HashMap::new() };
    let Some(open) = t[start..].find('{').map(|i| start + i) else { return HashMap::new() };
    let Some(close) = t.rfind('}') else { return HashMap::new() };
    if close <= open {
        return HashMap::new();
    }
    let mut body = t[open..=close].to_string();
    // a trailing comma before the closing brace isn't JSON
    if let Some(i) = body.rfind(',') {
        if body[i + 1..].trim() == "}" {
            body.replace_range(i..i + 1, "");
        }
    }
    serde_json::from_str::<HashMap<String, String>>(&body).unwrap_or_default()
}

fn hex_of(c: Color) -> String {
    let t = |x: f32| (x.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!("#{:02x}{:02x}{:02x}", t(c.r), t(c.g), t(c.b))
}

/// One of Hearth's icons in exactly this colour (cached: the same picture is made once).
pub fn icon_handle(name: &str, c: Color) -> Option<svg::Handle> {
    static CACHE: OnceLock<Mutex<HashMap<(String, String), svg::Handle>>> = OnceLock::new();
    let file = own_name(name);
    let data = icon_data().get(file)?;
    let hex = hex_of(c);
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut g = cache.lock().unwrap_or_else(|e| e.into_inner());
    let h = g.entry((file.to_string(), hex.clone())).or_insert_with(|| svg::Handle::from_memory(data.replace("{c}", &hex).into_bytes()));
    Some(h.clone())
}

/// Icon { name; size; color } as an element (with its colour's opacity).
pub fn icon<'a, M: 'a>(name: &str, size: f32, c: Color) -> iced::Element<'a, M> {
    match icon_handle(name, c) {
        Some(h) => svg(h).width(size).height(size).opacity(c.a).into(),
        None => iced::widget::Space::new().width(size).height(size).into(),
    }
}

/// An image file as a handle, made once (the flame, the generic app picture, the wallpaper's layers).
pub fn image_handle(path: &std::path::Path) -> iced::widget::image::Handle {
    static CACHE: OnceLock<Mutex<HashMap<std::path::PathBuf, iced::widget::image::Handle>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut g = cache.lock().unwrap_or_else(|e| e.into_inner());
    g.entry(path.to_path_buf()).or_insert_with(|| iced::widget::image::Handle::from_path(path)).clone()
}

pub fn svg_file(path: &std::path::Path) -> svg::Handle {
    static CACHE: OnceLock<Mutex<HashMap<std::path::PathBuf, svg::Handle>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut g = cache.lock().unwrap_or_else(|e| e.into_inner());
    g.entry(path.to_path_buf()).or_insert_with(|| svg::Handle::from_path(path)).clone()
}

/// An app's picture (AppIcon.qml): its icon file, else Hearth's generic app picture.
pub fn app_pic<'a, M: 'a>(path: Option<&std::path::Path>, size: f32) -> iced::Element<'a, M> {
    let generic = shell_file("images/app-generic.svg");
    let f = path.map(|p| p.to_path_buf()).unwrap_or(generic);
    if f.extension().and_then(|e| e.to_str()) == Some("svg") {
        svg(svg_file(&f)).width(size).height(size).into()
    } else {
        iced::widget::image(image_handle(&f)).width(size).height(size).filter_method(iced::widget::image::FilterMethod::Linear).into()
    }
}

/// The Hearth flame (images/hearth-logo.png).
pub fn flame<'a, M: 'a>(size: f32) -> iced::Element<'a, M> {
    iced::widget::image(image_handle(&shell_file("images/hearth-logo.png"))).width(size).height(size).filter_method(iced::widget::image::FilterMethod::Linear).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_data_is_read() {
        let t = "// x\n.pragma library\nvar icons = {\n\"bell\": \"<svg stroke=\\\"{c}\\\"/>\",\n\"kbd\": \"<svg/>\",\n};\n";
        let m = parse_icon_data(t);
        assert_eq!(m.get("bell").map(String::as_str), Some("<svg stroke=\"{c}\"/>"));
        assert!(m.contains_key("kbd"));
    }

    #[test]
    fn corners_are_capped_by_the_look() {
        let mut l = Look {
            p: hearth_ui::theme::load(),
            name: "steam".into(),
            style: "bar".into(),
            thin: false,
            glass_a: 0.97,
            font: Font::DEFAULT,
            frame: "plain".into(),
            hints: false,
            clock24: false,
            game_mode_off: false,
            wall: String::new(),
            reduce_motion: false,
        };
        l.p.corner = 4.0;
        assert_eq!(l.r(14.0), 4.0);
        l.p.corner = 0.0;
        assert_eq!(l.r(14.0), 14.0);
    }
}
