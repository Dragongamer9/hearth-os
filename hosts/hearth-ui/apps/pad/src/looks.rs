//! The on-screen keyboard's looks, Mantel's OskThemes.qml as it is: picked in Settings › Keyboard › Look ("deskTheme"
//! in ~/.config/nimbus/osk.json), else the engine's older pick, else Steam-style. On Plasma, Steam-style follows the
//! look the Hearth apps draw with (theme.rs: KDE's Global Theme through theme.json), as on Debian it follows Settings ›
//! Appearance.

use hearth_ui::theme::{hex, palette_map};
use hearth_ui::Pal;
use iced::font::Weight;
use iced::Color;
use serde_json::Value;

#[derive(Debug, Clone, Copy)]
pub struct Look {
    pub bg: Color,
    pub bg2: Color,
    pub key: Color,
    pub modk: Color,
    pub text: Color,
    pub muted: Color,
    pub sel: Color,
    pub sel_text: Color,
    pub accent: Color,
    pub ink: Color,
    /// key corner (None: by size; Some(-1): round keys)
    pub kr: Option<f32>,
    /// board corner
    pub br: f32,
    /// key outline width and colours (keys, modifier keys)
    pub kb: f32,
    pub kbc: Color,
    pub mbc: Option<Color>,
    /// board outline
    pub bbc: Color,
    pub fw: Weight,
    pub fs: f32,
    pub glow: Option<Color>,
}

fn c(s: &str) -> Color {
    hex(s).unwrap_or(Color::BLACK)
}
fn a(col: Color, al: f32) -> Color {
    Color { a: al, ..col }
}
/// Qt.tint(base, a(col, al))
fn tint(base: Color, col: Color, al: f32) -> Color {
    Pal::mix(base, col, al)
}

fn base(bg: &str, bg2: &str, key: &str, modk: &str, text: &str, muted: &str, sel: &str, sel_text: &str, accent: Color, ink: Color) -> Look {
    Look {
        bg: c(bg),
        bg2: c(bg2),
        key: c(key),
        modk: c(modk),
        text: c(text),
        muted: c(muted),
        sel: c(sel),
        sel_text: c(sel_text),
        accent,
        ink,
        kr: None,
        br: 20.0,
        kb: 0.0,
        kbc: Color::TRANSPARENT,
        mbc: None,
        bbc: a(Color::WHITE, 0.06),
        fw: Weight::Semibold,
        fs: 1.0,
        glow: None,
    }
}

pub const IDS: [&str; 9] = ["steam", "warm", "contrast", "frost", "pocket", "hearth", "ember", "slate", "daylight"];

/// The look in use (OskThemes.resolve).
pub fn resolve(chosen: &str, engine_theme: &str) -> &'static str {
    if let Some(id) = IDS.iter().find(|i| **i == chosen) {
        return id;
    }
    if engine_theme != "hearth" {
        if let Some(id) = IDS.iter().find(|i| **i == engine_theme) {
            return id;
        }
    }
    "steam"
}

pub fn look(id: &str, p: &Pal) -> Look {
    let dark = p.dark;
    let (acc, ink) = (p.accent, p.ink);
    // Hearth's own palette (Hearth warm), from palettes.json as theme.rs reads it.
    let pals = palette_map();
    let hw = pals.get("hearth").and_then(|l| l.get(if dark { "dark" } else { "light" })).cloned().unwrap_or(Value::Null);
    let h = |k: &str, d: &str| hw.get(k).and_then(Value::as_str).and_then(hex).unwrap_or_else(|| c(d));
    match id {
        "warm" => {
            let warm = h("ember", "#ff8a3d");
            let key = h("panel2", "#261d18");
            Look {
                bg: h("bg", "#0f0b09"),
                bg2: h("bg2", "#17110e"),
                key,
                modk: h("panel", "#1c1512"),
                text: h("text", "#f4ece3"),
                muted: h("muted", "#a69484"),
                sel: tint(key, warm, if dark { 0.5 } else { 0.35 }),
                sel_text: h("text", "#f4ece3"),
                accent: warm,
                ink: h("ink", "#2a1206"),
                kr: Some(12.0),
                br: 22.0,
                glow: Some(c(if dark { "#3a1a0c" } else { "#f3d2b4" })),
                bbc: a(warm, 0.18),
                ..base("#000", "#000", "#000", "#000", "#000", "#000", "#000", "#000", warm, ink)
            }
        }
        "contrast" => {
            let l = if dark {
                base("#000000", "#141414", "#000000", "#000000", "#ffffff", "#e6e6e6", "#ffd400", "#000000", c("#ffd400"), c("#000000"))
            } else {
                base("#ffffff", "#ededed", "#ffffff", "#ffffff", "#000000", "#1a1a1a", "#0047d6", "#ffffff", c("#0047d6"), c("#ffffff"))
            };
            Look {
                kr: Some(6.0),
                br: 8.0,
                kb: 2.0,
                kbc: c(if dark { "#ffffff" } else { "#000000" }),
                mbc: Some(c(if dark { "#9a9a9a" } else { "#5a5a5a" })),
                bbc: c(if dark { "#ffffff" } else { "#000000" }),
                fw: Weight::Bold,
                fs: 1.08,
                ..l
            }
        }
        "frost" => Look {
            bg: if dark { a(p.panel, 0.55) } else { a(Color::WHITE, 0.55) },
            bg2: if dark { a(Color::WHITE, 0.08) } else { a(Color::WHITE, 0.6) },
            key: if dark { a(Color::WHITE, 0.09) } else { a(Color::WHITE, 0.7) },
            modk: if dark { a(Color::WHITE, 0.04) } else { a(Color::WHITE, 0.4) },
            text: p.text,
            muted: p.muted,
            sel: a(acc, 0.9),
            sel_text: ink,
            accent: acc,
            ink,
            kr: Some(14.0),
            br: 24.0,
            kb: 1.0,
            kbc: if dark { a(Color::WHITE, 0.14) } else { a(Color::BLACK, 0.08) },
            mbc: None,
            bbc: if dark { a(Color::WHITE, 0.18) } else { a(Color::WHITE, 0.8) },
            fw: Weight::Normal,
            fs: 1.0,
            glow: None,
        },
        "pocket" => {
            let l = if dark {
                base("#0d0f14", "#171a22", "#1d212b", "#14171e", "#f2f5f9", "#8e97a6", "#000000", "#000000", acc, ink)
            } else {
                base("#eef1f5", "#e1e6ed", "#ffffff", "#d7dde6", "#121620", "#5d6676", "#000000", "#000000", acc, ink)
            };
            Look { sel: acc, sel_text: ink, kr: Some(-1.0), br: 28.0, fw: Weight::Bold, fs: 1.18, bbc: Color::TRANSPARENT, ..l }
        }
        "hearth" => base("#17171c", "#1f1f25", "#2b2b32", "#222228", "#f3efe8", "#a7a3ad", "#f6f2ea", "#141217", acc, ink),
        "ember" => Look { glow: Some(c("#3a1a0c")), ..base("#120c0a", "#1d1310", "#2a1a14", "#1f1410", "#ffeadb", "#c9a18a", "#ffd9bd", "#2a1206", c("#ff7a2f"), c("#1d0b02")) },
        "slate" => base("#1b2028", "#222833", "#2d3542", "#252c37", "#eef2f7", "#9aa6b6", "#f2f6fb", "#111722", acc, ink),
        "daylight" => base("#e9e6e1", "#dedad3", "#ffffff", "#d4d0c8", "#1d1b20", "#6b6670", "#1d1b20", "#ffffff", acc, ink),
        // Steam-style: the look in use (on Plasma: the Global Theme), your accent, dark or light.
        _ => Look {
            bg: if dark { p.bg } else { p.bg2 },
            bg2: if dark { p.bg2 } else { p.panel3 },
            key: if dark { p.panel2 } else { p.panel },
            modk: if dark { p.panel } else { p.panel3 },
            text: p.text,
            muted: p.muted,
            sel: tint(if dark { p.panel2 } else { Color::WHITE }, acc, if dark { 0.45 } else { 0.3 }),
            sel_text: p.text,
            accent: acc,
            ink,
            kr: Some(4.0),
            br: 6.0,
            kb: 0.0,
            kbc: Color::TRANSPARENT,
            mbc: None,
            bbc: if dark { a(Color::WHITE, 0.06) } else { a(Color::BLACK, 0.08) },
            fw: Weight::Medium,
            fs: 1.0,
            glow: None,
        },
    }
}
