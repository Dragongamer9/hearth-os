//! A whole-screen layout (build-anything boards 1-4, 6): the desktop (or Game Mode) as one screen of blocks placed
//! anywhere on a grid, at any size, each styled freely: a console dashboard, an icon-row home, a cockpit, a cabin, a
//! notebook, a 1995 computer, or anything else. Hearth powers it underneath (apps, games, windows, settings); no two
//! need to look alike. Windows open over it as always (a block can be put above them, like a bar).
//!
//! ```json
//! { "grid": {"cols": 24, "rows": 14}, "background": {"kind": "art"}, "look": {"font": "Inter", "accent": "#ffffff"},
//!   "blocks": [ {"id": "hero", "block": "hero", "x": 0, "y": 2, "w": 11, "h": 6,
//!                "style": {"corners": 8, "focus": "grow", "sound": "click", "animation": "slide-left"},
//!                "press": {"action": "play"}} ] }
//! ```
//!
//! A block named `own:<id>` is one your AI wrote (build-anything board 5): it runs as its own small program, boxed in
//! (made.rs); the layout only says where it goes and how it looks.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Every block a screen can hold: (id, what it is, what it shows by default).
pub const SCREEN_BLOCKS: &[(&str, &str)] = &[
    ("hero", "a big tile of one game or app (the last one played, or the one picked), with its art"),
    ("tilerow", "a row of tiles (recent games, your apps, favourites, a collection)"),
    ("iconrow", "a row of rounded icons, the picked one bigger, its art filling the screen behind"),
    ("tabs", "tabs across the screen (Games, Media, Desktop, your own)"),
    ("guide", "a side menu that slides in on the Home button (Home, Quick settings, Friends, Capture, windows, Game Mode, Power)"),
    ("clock", "the time and date, any size"),
    ("profile", "your picture, name and who's online"),
    ("battery", "the battery"),
    ("status", "network, volume, battery and notifications"),
    ("bar", "a bar with Mantel's blocks (blocks.rs)"),
    ("dock", "your apps as a dock"),
    ("media", "what's playing, with its controls"),
    ("friends", "friends and what they're playing"),
    ("notes", "a note or list of yours"),
    ("weather", "the weather"),
    ("windows", "your open windows"),
    ("downloads", "copies and downloads"),
    ("activity", "cards of what you did (time played, best times, what's next)"),
    ("widget", "one of Hearth's widgets (system monitor, timer, Hearth Link devices)"),
    ("text", "your own words"),
    ("picture", "a picture of yours"),
    ("play", "a big Play button for the picked game"),
    ("search", "the search field"),
    ("menu", "the app menu (the flame)"),
    ("gamemode", "Game Mode"),
    ("list", "a plain list of words: your games, your apps (a HUD's readout, a notebook's lines)"),
    ("icons", "desktop icons of Hearth's things (My Hearth, the Bin, Files, Games)"),
];

/// What a block's look may be (build-anything board 4: Look).
pub const FOCUS: &[&str] = &["", "grow", "outline", "glow", "lift", "none"];
pub const SOUNDS: &[&str] = &["", "click", "soft", "pop", "chime", "wood", "retro", "none"];
pub const ANIMATIONS: &[&str] = &["", "slide-left", "slide-right", "slide-up", "slide-down", "fade", "zoom", "flip", "none"];
pub const FONTS: &[&str] = &["", "Inter", "Inter Display", "Noto Serif", "Noto Sans Mono", "Comic Neue", "Pixel", "Rounded", "Condensed"];
pub const BACKGROUNDS: &[&str] = &["", "art", "colour", "gradient", "paper", "picture", "moving", "wallpaper", "none"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Grid {
    pub cols: u32,
    pub rows: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Style {
    /// "#rrggbb" or "#rrggbbaa"
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub colour: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub background: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub text: String,
    /// 0-64 px
    #[serde(default)]
    pub corners: u32,
    /// 0-100
    #[serde(default, rename = "seeThrough")]
    pub see_through: u32,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub font: String,
    /// 100-900
    #[serde(default)]
    pub weight: u32,
    /// text size, 8-160 px (0: the block's own)
    #[serde(default)]
    pub size: u32,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub focus: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub sound: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub animation: String,
    /// over windows (a bar, a clock you always see) rather than under them
    #[serde(default, rename = "aboveWindows")]
    pub above_windows: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Placed {
    pub id: String,
    pub block: String,
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    #[serde(default)]
    pub style: Style,
    /// what it shows: a source (feature.rs SOURCES), a collection ("recent", "installed", "favourites", "apps"), words…
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub shows: String,
    /// what a press does (feature.rs ACTIONS, and "play")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub press: Option<crate::feature::Click>,
    /// a bar's or a row's own items
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<String>,
    /// a text block's words, a picture's file (in the layout's own folder)
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub value: String,
    /// the words over a row ("Jump back in", "Your apps and things")
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub title: String,
    /// a row of games: how many to leave out from the start (the hero has the first) and how many to show (0: all)
    #[serde(default, skip_serializing_if = "is_zero")]
    pub skip: u32,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub count: u32,
}

fn is_zero(n: &u32) -> bool {
    *n == 0
}

/// What a row's `items` may name: Hearth's own things, or an app by its menu entry ("app:org.gnome.Rhythmbox3").
pub const ITEMS: &[(&str, &str)] = &[
    ("games", "Games (the app menu's games)"), ("store", "Hearth Store"), ("files", "Files"), ("browser", "the web browser"),
    ("music", "Music"), ("friends", "Family and friends"), ("downloads", "downloads and updates"), ("settings", "Settings"),
    ("capture", "Capture"), ("notes", "Hearth Notes"), ("gamemode", "Game Mode"), ("power", "Power"), ("quick", "Quick settings"),
    ("search", "Search"), ("menu", "the app menu"), ("windows", "your windows"), ("bin", "the Bin"), ("home", "your home folder"),
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Background {
    #[serde(default)]
    pub kind: String,
    /// a picture's file
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub value: String,
    /// a colour, or a gradient's two colours (top to bottom)
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub from: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Look {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub font: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub accent: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub text: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub sound: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub animation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Screen {
    pub grid: Grid,
    #[serde(default)]
    pub background: Background,
    #[serde(default)]
    pub look: Look,
    #[serde(default)]
    pub blocks: Vec<Placed>,
    /// where it's for: "desktop" (the default) or "gamemode"
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub r#for: String,
    /// "hidden": the screen does what Mantel's bar and dock do (its Guide, menu, Game Mode) and covers them; "": they stay
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub bars: String,
    /// the words that made it, kept with it
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub said: String,
}

fn colour_ok(s: &str) -> bool {
    s.is_empty() || ((s.len() == 7 || s.len() == 9) && s.starts_with('#') && s[1..].chars().all(|c| c.is_ascii_hexdigit()))
}

fn block_known(b: &str) -> bool {
    SCREEN_BLOCKS.iter().any(|(id, _)| *id == b) || b.strip_prefix("own:").map(crate::is_slug).unwrap_or(false)
}

/// Everything wrong with a screen, in plain words (empty: it can be put in place).
pub fn check(s: &Screen) -> Vec<String> {
    let mut e = Vec::new();
    if !(4..=96).contains(&s.grid.cols) || !(4..=64).contains(&s.grid.rows) {
        e.push("The grid is 4 to 96 squares across and 4 to 64 down.".into());
    }
    if !BACKGROUNDS.contains(&s.background.kind.as_str()) {
        e.push(format!("\"{}\" isn't a background Hearth can draw.", s.background.kind));
    }
    if !colour_ok(&s.background.from) || !colour_ok(&s.background.to) || s.background.kind == "colour" && s.background.from.is_empty() {
        e.push("A colour is written like #1e3a5f.".into());
    }
    if !["", "hidden"].contains(&s.bars.as_str()) {
        e.push("The bar and dock either stay (\"\") or are \"hidden\".".into());
    }
    if s.said.chars().count() > 2000 {
        e.push("What made it is up to 2000 letters.".into());
    }
    if s.background.value.contains("..") || s.background.value.starts_with('/') && s.background.kind != "picture" {
        e.push("A background picture is a file of yours, named plainly.".into());
    }
    for c in [&s.look.accent, &s.look.text] {
        if !colour_ok(c) {
            e.push("A colour is written like #1e3a5f.".into());
        }
    }
    if !FONTS.contains(&s.look.font.as_str()) {
        e.push(format!("\"{}\" isn't a font Hearth has.", s.look.font));
    }
    if s.blocks.len() > 64 {
        e.push("Up to 64 blocks on one screen.".into());
    }
    let mut ids: Vec<&str> = Vec::new();
    for b in &s.blocks {
        let name = if b.id.is_empty() { b.block.as_str() } else { b.id.as_str() };
        if !crate::is_slug(&b.id) {
            e.push(format!("A block's id must be short, with lowercase letters, digits and dashes (\"{}\").", b.id));
        }
        if ids.contains(&b.id.as_str()) {
            e.push(format!("Two blocks are called \"{}\".", b.id));
        }
        ids.push(&b.id);
        if !block_known(&b.block) {
            e.push(format!("\"{}\" isn't a block Hearth has (your AI can make it: own:<name>).", b.block));
        }
        if b.w == 0 || b.h == 0 || b.x + b.w > s.grid.cols || b.y + b.h > s.grid.rows {
            e.push(format!("\"{name}\" goes off the screen."));
        }
        let st = &b.style;
        for c in [&st.colour, &st.background, &st.text] {
            if !colour_ok(c) {
                e.push(format!("\"{name}\": a colour is written like #1e3a5f."));
            }
        }
        if st.corners > 64 || st.see_through > 100 || (st.weight != 0 && !(100..=900).contains(&st.weight)) || (st.size != 0 && !(8..=160).contains(&st.size)) {
            e.push(format!("\"{name}\": corners 0-64, see-through 0-100, weight 100-900, text 8-160."));
        }
        if !FOCUS.contains(&st.focus.as_str()) || !SOUNDS.contains(&st.sound.as_str()) || !ANIMATIONS.contains(&st.animation.as_str()) || !FONTS.contains(&st.font.as_str()) {
            e.push(format!("\"{name}\": a focus, sound, animation or font Hearth hasn't got."));
        }
        if let Some(p) = &b.press {
            if p.action != "play" {
                let f = crate::feature::Feature { format: 1, id: "x".into(), name: "x".into(), part: "desktop".into(), shows: vec![], every: 30, click: Some(p.clone()), needs: vec![] };
                let c = crate::feature::check(&f);
                e.extend(c.problems.into_iter().map(|x| format!("\"{name}\": {x}")));
                e.extend(c.needs.into_iter().map(|x| format!("\"{name}\" needs {x}.")));
            }
        }
        if b.value.chars().count() > 2000 || b.value.contains("..") || b.title.chars().count() > 80 {
            e.push(format!("\"{name}\": its words or picture can't be used."));
        }
        for i in &b.items {
            let ok = ITEMS.iter().any(|(id, _)| id == i) || i.strip_prefix("app:").map(|a| !a.is_empty() && a.len() < 120 && a.chars().all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))).unwrap_or(false);
            if !ok {
                e.push(format!("\"{name}\": \"{i}\" isn't something Hearth has to put in a row."));
            }
        }
    }
    // the way back to everything else: the app menu or the Guide, and Game Mode (Game Mode itself needn't have it)
    if s.r#for != "gamemode" && s.bars == "hidden" {
        let has = |k: &str| s.blocks.iter().any(|b| b.block == k || b.items.iter().any(|i| i == k));
        if !has("menu") && !has("guide") && !has("bar") && !has("dock") {
            e.push("Keep a way to your apps: the app menu, the Guide, a bar or a dock.".into());
        }
    }
    e
}

/// The blocks your AI made that this screen uses ("own:<id>").
pub fn own_blocks(s: &Screen) -> Vec<String> {
    s.blocks.iter().filter_map(|b| b.block.strip_prefix("own:").map(str::to_string)).collect()
}

/// A block's grid rectangle in pixels on a screen of `w` × `h`.
pub fn rect(s: &Screen, b: &Placed, w: f32, h: f32) -> (f32, f32, f32, f32) {
    let cw = w / s.grid.cols.max(1) as f32;
    let ch = h / s.grid.rows.max(1) as f32;
    (b.x as f32 * cw, b.y as f32 * ch, b.w as f32 * cw, b.h as f32 * ch)
}

/// What the AI is told about screens (describe.rs adds it to the rules).
pub fn rules() -> String {
    let blocks: Vec<String> = SCREEN_BLOCKS.iter().map(|(id, n)| format!("{id} ({n})")).collect();
    format!(
        "A whole screen: \"screen\": {{\"grid\": {{\"cols\": 24, \"rows\": 14}}, \"background\": {{\"kind\": {bg}, \"value\"}}, \"look\": {{\"font\", \"accent\": \"#rrggbb\", \"text\", \"sound\", \"animation\"}}, \"blocks\": [{{\"id\", \"block\", \"x\", \"y\", \"w\", \"h\", \"style\": {{\"colour\", \"background\", \"text\", \"corners\" 0-64, \"seeThrough\" 0-100, \"font\", \"weight\", \"size\", \"focus\": {fo}, \"sound\": {so}, \"animation\": {an}, \"aboveWindows\"}}, \"shows\", \"title\", \"skip\", \"count\", \"press\": {{\"action\", \"target\"}}, \"items\", \"value\"}}], \"bars\": \"\"|\"hidden\"}}\n\
   background: {{\"kind\", \"from\": \"#rrggbb\", \"to\": \"#rrggbb\", \"value\": a picture}}; a row's items: {items}; with \"bars\": \"hidden\" keep a guide or menu block.\n\
   screen blocks: {b}; a block Hearth hasn't got: \"own:<name>\" plus a \"make\" entry saying what it should do.\n\
   fonts: {fonts}. Use Hearth's own icons and sounds; never another company's logos, names or sounds.",
        bg = BACKGROUNDS.iter().filter(|x| !x.is_empty()).map(|x| format!("\"{x}\"")).collect::<Vec<_>>().join("|"),
        fo = FOCUS.iter().filter(|x| !x.is_empty()).map(|x| format!("\"{x}\"")).collect::<Vec<_>>().join("|"),
        so = SOUNDS.iter().filter(|x| !x.is_empty()).map(|x| format!("\"{x}\"")).collect::<Vec<_>>().join("|"),
        an = ANIMATIONS.iter().filter(|x| !x.is_empty()).map(|x| format!("\"{x}\"")).collect::<Vec<_>>().join("|"),
        b = blocks.join(", "),
        items = ITEMS.iter().map(|(i, _)| *i).collect::<Vec<_>>().join(", "),
        fonts = FONTS.iter().filter(|x| !x.is_empty()).copied().collect::<Vec<_>>().join(", "),
    )
}

/// Mantel's ready-made desktops (build-anything board 1), one copy for both Mantels: the QML Mantel reads the files,
/// Mantel in Rust and the checks have them built in.
pub const READY: &[(&str, &str)] = &[
    ("console", include_str!("../../linux/desktop/shell/screens/console.json")),
    ("iconrow", include_str!("../../linux/desktop/shell/screens/iconrow.json")),
    ("cockpit", include_str!("../../linux/desktop/shell/screens/cockpit.json")),
    ("cabin", include_str!("../../linux/desktop/shell/screens/cabin.json")),
    ("notebook", include_str!("../../linux/desktop/shell/screens/notebook.json")),
    ("retro", include_str!("../../linux/desktop/shell/screens/retro.json")),
];

/// A screen from JSON (a layout's "screen").
pub fn parse(v: &Value) -> Result<Screen, Vec<String>> {
    let s: Screen = serde_json::from_value(v.clone()).map_err(|e| vec![format!("The screen couldn't be read ({e}).")])?;
    let errs = check(&s);
    if errs.is_empty() { Ok(s) } else { Err(errs) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// build-anything board 2: the console dashboard.
    pub fn console() -> Value {
        json!({"grid": {"cols": 24, "rows": 14}, "background": {"kind": "art"}, "look": {"font": "Inter", "accent": "#ffffff"},
               "blocks": [
                 {"id": "me", "block": "profile", "x": 0, "y": 0, "w": 5, "h": 1},
                 {"id": "corner", "block": "status", "x": 18, "y": 0, "w": 6, "h": 1},
                 {"id": "hero", "block": "hero", "x": 0, "y": 2, "w": 8, "h": 5, "shows": "recent",
                  "style": {"corners": 8, "focus": "grow", "sound": "click", "animation": "slide-left"}, "press": {"action": "play"}},
                 {"id": "games", "block": "tilerow", "x": 8, "y": 2, "w": 13, "h": 5, "shows": "recent"},
                 {"id": "things", "block": "tilerow", "x": 0, "y": 8, "w": 21, "h": 3, "shows": "apps"},
                 {"id": "guide", "block": "guide", "x": 18, "y": 0, "w": 6, "h": 14, "style": {"aboveWindows": true}},
                 {"id": "printer", "block": "own:printer-camera", "x": 21, "y": 11, "w": 3, "h": 3}
               ]})
    }

    #[test]
    fn the_console_dashboard_is_a_screen() {
        let s = parse(&console()).unwrap();
        assert_eq!(own_blocks(&s), ["printer-camera"]);
        let (x, y, w, h) = rect(&s, &s.blocks[2], 1920.0, 1120.0);
        assert_eq!((x, y, w, h), (0.0, 160.0, 640.0, 400.0));
    }

    #[test]
    fn anything_off_the_list_is_said_plainly() {
        let mut v = console();
        v["blocks"][2]["x"] = json!(20);
        assert!(parse(&v).unwrap_err().iter().any(|e| e.contains("off the screen")));
        let mut v = console();
        v["blocks"][2]["style"]["corners"] = json!(400);
        assert!(parse(&v).is_err());
        let mut v = console();
        v["blocks"][2]["block"] = json!("rocket-launcher");
        assert!(parse(&v).unwrap_err()[0].contains("your AI can make it"));
        let mut v = console();
        v["blocks"][2]["press"] = json!({"action": "open-app", "target": "x; rm -rf ~"});
        assert!(parse(&v).is_err());
        let mut v = console();
        v["blocks"] = json!([{"id": "c", "block": "clock", "x": 0, "y": 0, "w": 4, "h": 2}]);
        assert!(parse(&v).is_ok(), "with Mantel's bar and dock still there, that's the way to your apps");
        v["bars"] = json!("hidden");
        assert!(parse(&v).unwrap_err().iter().any(|e| e.contains("a way to your apps")));
        v["for"] = json!("gamemode");
        assert!(parse(&v).is_ok(), "Game Mode is its own way back");
    }

    #[test]
    fn the_rules_name_every_block() {
        let r = rules();
        for (b, _) in SCREEN_BLOCKS {
            assert!(r.contains(b));
        }
        assert!(r.contains("own:<name>"));
    }

    /// Mantel's ready-made desktops (hosts/linux/desktop/shell/screens, both Mantels read them) are all good screens.
    #[test]
    fn the_ready_made_desktops_are_good_screens() {
        for (id, text) in READY {
            let v: Value = serde_json::from_str(text).unwrap_or_else(|e| panic!("{id}: {e}"));
            let s = parse(&v).unwrap_or_else(|e| panic!("{id}: {e:?}"));
            assert!(!s.said.is_empty(), "{id}: the words that made it");
        }
    }
}
