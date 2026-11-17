//! Mantel's colours for the compositor's own parts (window frames, the background behind everything): the tokens
//! hearth-theme already writes for Hearth's apps, ~/.config/hearth/theme.json ("colors": bg, bg2, panel2, line, text,
//! muted, ember, danger...). Read only, and read again when the file changes (Settings › Appearance). Missing or
//! broken: Hearth's dark look, the same colours labwc's themerc-override has.

use std::{
    path::PathBuf,
    sync::{RwLock, RwLockReadGuard},
    time::SystemTime,
};

use serde::Deserialize;

pub type Rgba = [f32; 4];

#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    pub bg: Rgba,
    pub title_active: Rgba,
    pub title_inactive: Rgba,
    pub hover: Rgba,
    pub line: Rgba,
    pub text: Rgba,
    pub muted: Rgba,
    pub accent: Rgba,
    pub danger: Rgba,
    /// The font family Mantel uses (the first name in theme.json's "font"), for the titles.
    pub font: String,
    /// Bumped on every change, so frames drawn with older colours are drawn again.
    pub serial: u64,
}

impl Default for Theme {
    fn default() -> Self {
        // Hearth dark (desktop/labwc/themerc-override).
        Theme {
            bg: hex("#0f0b09").unwrap(),
            title_active: hex("#1c1512").unwrap(),
            title_inactive: hex("#140f0c").unwrap(),
            hover: hex("#33271f").unwrap(),
            line: hex("#3b2e26").unwrap(),
            text: hex("#f4ece3").unwrap(),
            muted: hex("#a69484").unwrap(),
            accent: hex("#ff8a3d").unwrap(),
            danger: hex("#e5484d").unwrap(),
            font: String::new(),
            serial: 0,
        }
    }
}

/// "#rrggbb" (or "#rrggbbaa") as 0..1 floats.
pub fn hex(s: &str) -> Option<Rgba> {
    let h = s.trim().strip_prefix('#')?;
    let v = |i: usize| u8::from_str_radix(h.get(i..i + 2)?, 16).ok().map(|b| b as f32 / 255.0);
    match h.len() {
        6 => Some([v(0)?, v(2)?, v(4)?, 1.0]),
        8 => Some([v(0)?, v(2)?, v(4)?, v(6)?]),
        _ => None,
    }
}

#[derive(Deserialize, Default)]
struct File {
    #[serde(default)]
    colors: std::collections::HashMap<String, String>,
    #[serde(default)]
    font: String,
}

/// The theme from theme.json's text (None when it isn't Hearth's JSON).
pub fn parse(text: &str) -> Option<Theme> {
    let f: File = serde_json::from_str(text).ok()?;
    let d = Theme::default();
    let c = |k: &str, fallback: Rgba| f.colors.get(k).and_then(|v| hex(v)).unwrap_or(fallback);
    let bg = c("bg", d.bg);
    let bg2 = c("bg2", d.title_active);
    Some(Theme {
        bg,
        title_active: bg2,
        title_inactive: c("chrome", bg2).map_with(&bg, 0.45),
        hover: c("panel2", d.hover),
        line: c("line", d.line),
        text: c("text", d.text),
        muted: c("muted", d.muted),
        accent: c("ember", c("accent", d.accent)),
        danger: c("danger", d.danger),
        font: f
            .font
            .split(',')
            .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
            .find(|s| !s.is_empty() && !s.contains("sans-serif") && !s.starts_with('-'))
            .unwrap_or_default(),
        serial: 0,
    })
}

trait Mix {
    fn map_with(self, other: &Rgba, t: f32) -> Rgba;
}
impl Mix for Rgba {
    /// self with t of other mixed in.
    fn map_with(self, other: &Rgba, t: f32) -> Rgba {
        let mut o = self;
        for i in 0..3 {
            o[i] = self[i] + (other[i] - self[i]) * t;
        }
        o
    }
}

pub fn path() -> PathBuf {
    if let Some(p) = std::env::var_os("HEARTH_THEME_JSON") {
        return p.into();
    }
    let conf = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".config"));
    conf.join("hearth").join("theme.json")
}

static THEME: RwLock<Option<Theme>> = RwLock::new(None);
static SEEN: RwLock<Option<(PathBuf, Option<SystemTime>)>> = RwLock::new(None);

pub fn current() -> Theme {
    {
        let g: RwLockReadGuard<'_, Option<Theme>> = THEME.read().unwrap();
        if let Some(t) = g.as_ref() {
            return t.clone();
        }
    }
    reload();
    THEME.read().unwrap().clone().unwrap_or_default()
}

/// Reads theme.json again when it changed since the last look (cheap: one stat). True when the colours changed.
pub fn reload() -> bool {
    let p = path();
    let mtime = std::fs::metadata(&p).and_then(|m| m.modified()).ok();
    {
        let seen = SEEN.read().unwrap();
        if let Some((sp, sm)) = seen.as_ref() {
            if *sp == p && *sm == mtime && THEME.read().unwrap().is_some() {
                return false;
            }
        }
    }
    *SEEN.write().unwrap() = Some((p.clone(), mtime));
    let mut next = std::fs::read_to_string(&p).ok().and_then(|t| parse(&t)).unwrap_or_default();
    let mut g = THEME.write().unwrap();
    let old = g.clone();
    if let Some(o) = &old {
        next.serial = o.serial;
        if *o == next {
            return false;
        }
        next.serial = o.serial + 1;
    }
    tracing::info!(path = %p.display(), "theme: Mantel's colours read");
    *g = Some(next);
    old.is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_hearths_tokens() {
        let t = parse(r##"{"look":"hearth","mode":"dark","font":"\"Inter\", system-ui, sans-serif",
            "colors":{"bg":"#101010","bg2":"#202020","ember":"#ff0000","text":"#ffffff"}}"##)
        .unwrap();
        assert_eq!(t.bg, [16.0 / 255.0, 16.0 / 255.0, 16.0 / 255.0, 1.0]);
        assert_eq!(t.accent, [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(t.font, "Inter");
        assert_eq!(t.muted, Theme::default().muted);
    }

    #[test]
    fn broken_files_fall_back() {
        assert!(parse("not json").is_none());
        assert_eq!(hex("#zzzzzz"), None);
        assert_eq!(hex("#ff8a3d").unwrap()[3], 1.0);
    }
}
