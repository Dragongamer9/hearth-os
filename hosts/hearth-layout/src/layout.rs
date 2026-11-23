//! A layout file (docs/LAYOUTS.md): where the bars, the panels, the dock, the app menu and the window buttons go.
//! Your look's colours aren't in it (they stay the look's). A layout only names Mantel's own settings and blocks; a file
//! that names anything else, or would leave you without the app menu or Game Mode, is refused with the reasons.
//!
//! ```json
//! { "format": 1, "id": "plasma", "name": "Plasma-style",
//!   "desktop": { "barLayout": "one", "barEdge": "bottom", "dockFloat": "off", "appMenu": "start" },
//!   "bar": { "start": ["menu"], "end": ["tray", "network", "volume", "battery", "notifications"] },
//!   "panels": [],
//!   "windowButtons": { "side": "right", "buttons": ["iconify", "max", "close"] } }
//! ```

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::blocks;

/// The desktop.json keys a layout may set (Settings.qml's "Bar layout and app menus" and the bar style), with the
/// values each may have. "" is that setting's default.
pub const DESKTOP_KEYS: &[(&str, &[&str])] = &[
    ("barLayout", &["", "split", "one"]),
    ("barEdge", &["", "bottom", "top", "left", "right"]),
    // "none": no dock and no apps in the bar (GNOME-style: your apps are in the overview's dash)
    ("dockFloat", &["", "on", "off", "none"]),
    ("dockMode", &["", "shown", "autohide"]),
    ("appMenu", &["", "hearth", "centred", "tiles", "start", "classic"]),
    ("menuStyle", &["", "today", "windows", "compact", "tiles"]),
    ("barStyle", &["", "floating", "bar"]),
    // Game Mode in the main bar: last (""), first ("start": Windows 11-style) or before the status island (macOS-style)
    ("gameModeAt", &["", "start", "before-status"]),
    // the clock: "" (the look's own: the pill in the middle, or at the end with your apps in the bar), "end" (at the
    // end, the time over the date: Plasma-, Windows 11-style), "end-one" (at the end on one line: macOS-style)
    ("clockAt", &["", "end", "end-one"]),
    // the dock floating or attached to the edge whatever the look says ("": the look's own; macOS-style: floating)
    ("dockStyle", &["", "floating", "attached"]),
    // the status icons in a rounded pill even in a full-width bar (GNOME-style)
    ("statusPill", &["", "on"]),
];

pub const EDGES: &[&str] = &["top", "bottom", "left", "right"];
pub const BUTTONS: &[&str] = &["icon", "iconify", "max", "close", "shade", "desk"];

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Bar {
    #[serde(default)]
    pub start: Vec<String>,
    /// the middle of the main bar (the clock pill today; new blocks there are drawn once built)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub centre: Vec<String>,
    #[serde(default)]
    pub end: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hidden: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Panel {
    pub edge: String,
    #[serde(default)]
    pub items: Vec<String>,
    #[serde(default = "default_size")]
    pub size: u32,
    #[serde(default)]
    pub auto_hide: bool,
    /// "" (full width) | "floating" | "centred"
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub style: String,
}
fn default_size() -> u32 {
    44
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WindowButtons {
    /// "right" | "left"
    pub side: String,
    pub buttons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Layout {
    pub format: u32,
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub about: String,
    #[serde(default)]
    pub desktop: Map<String, Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bar: Option<Bar>,
    #[serde(default)]
    pub panels: Vec<Panel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window_buttons: Option<WindowButtons>,
    /// a whole-screen layout of blocks (screen.rs; build-anything boards 1-4)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub screen: Option<crate::screen::Screen>,
}

impl Layout {
    /// The main bar's edge this layout gives (BarLayout.qml: one main bar on barEdge, else the top bar of Split).
    pub fn main_edge(&self) -> &str {
        let s = |k: &str| self.desktop.get(k).and_then(Value::as_str).unwrap_or("");
        if s("barLayout") == "one" {
            match s("barEdge") {
                "" => "bottom",
                e => e,
            }
        } else {
            "top"
        }
    }
    /// The floating dock along the bottom is on (Split, or One bar with the dock floating).
    pub fn dock_on(&self) -> bool {
        let s = |k: &str| self.desktop.get(k).and_then(Value::as_str).unwrap_or("");
        s("dockFloat") != "off" && s("dockFloat") != "none"
    }
    /// The blocks this layout names that neither Mantel draws yet (shown in the editor: "coming").
    pub fn not_built(&self) -> Vec<String> {
        let mut v: Vec<String> = Vec::new();
        let mut add = |id: &String| {
            if blocks::find(id).map(|b| !b.built).unwrap_or(false) && !v.contains(id) {
                v.push(id.clone());
            }
        };
        if let Some(b) = &self.bar {
            b.start.iter().chain(&b.centre).chain(&b.end).for_each(&mut add);
        }
        self.panels.iter().flat_map(|p| p.items.iter()).for_each(&mut add);
        v
    }
}

/// Reads a layout file's text: the layout, or every reason it can't be used (in plain words).
pub fn parse(text: &str) -> Result<Layout, Vec<String>> {
    let l: Layout = serde_json::from_str(text).map_err(|e| vec![format!("It isn't a layout file Hearth can read ({e}).")])?;
    let errs = check(&l);
    if errs.is_empty() {
        Ok(l)
    } else {
        Err(errs)
    }
}

/// Everything wrong with a layout, in plain words; empty = it can be used.
pub fn check(l: &Layout) -> Vec<String> {
    let mut e = Vec::new();
    if let Some(s) = &l.screen {
        e.extend(crate::screen::check(s));
    }
    if l.format != 1 {
        e.push(format!("It's made for a newer Hearth (layout format {}).", l.format));
    }
    if !crate::is_slug(&l.id) {
        e.push("Its id must be short, with lowercase letters, digits and dashes.".into());
    }
    if l.name.trim().is_empty() || l.name.chars().count() > 60 {
        e.push("It needs a name (up to 60 letters).".into());
    }
    for (k, v) in &l.desktop {
        match DESKTOP_KEYS.iter().find(|(name, _)| name == k) {
            None => e.push(format!("\"{k}\" isn't a setting a layout can change.")),
            Some((_, ok)) => {
                if !v.as_str().map(|s| ok.contains(&s)).unwrap_or(false) {
                    e.push(format!("\"{k}\" can't be {v}."));
                }
            }
        }
    }
    if let Some(bar) = &l.bar {
        let mut seen: Vec<&String> = Vec::new();
        for (zone, ids) in [("start", &bar.start), ("centre", &bar.centre), ("end", &bar.end), ("hidden", &bar.hidden)] {
            for id in ids {
                if !blocks::in_bar(id) {
                    e.push(format!("\"{id}\" can't go in the main bar ({zone})."));
                } else if seen.contains(&id) {
                    e.push(format!("\"{id}\" is in the main bar twice."));
                } else if zone == "hidden" && blocks::find(id).map(|b| b.keep).unwrap_or(false) {
                    e.push(format!("{} can't be hidden.", blocks::find(id).map(|b| b.name).unwrap_or(id)));
                }
                seen.push(id);
            }
        }
    }
    if l.panels.len() > 4 {
        e.push("Up to four extra panels.".into());
    }
    let main = l.main_edge().to_string();
    let mut edges: Vec<&str> = Vec::new();
    for (i, p) in l.panels.iter().enumerate() {
        let n = i + 1;
        if !EDGES.contains(&p.edge.as_str()) {
            e.push(format!("Panel {n}: its edge can't be \"{}\".", p.edge));
        } else if p.edge == main {
            e.push(format!("Panel {n}: the main bar is on the {} already.", p.edge));
        } else if p.edge == "bottom" && l.dock_on() {
            e.push(format!("Panel {n}: the dock is along the bottom."));
        } else if edges.contains(&p.edge.as_str()) {
            e.push(format!("Panel {n}: there's a panel on the {} already.", p.edge));
        }
        edges.push(p.edge.as_str());
        if !(28..=96).contains(&p.size) {
            e.push(format!("Panel {n}: its size must be 28 to 96."));
        }
        if !["", "floating", "centred"].contains(&p.style.as_str()) {
            e.push(format!("Panel {n}: its style can't be \"{}\".", p.style));
        }
        if p.items.is_empty() {
            e.push(format!("Panel {n} is empty."));
        }
        for id in &p.items {
            if !blocks::in_panel(id) {
                e.push(format!("Panel {n}: \"{id}\" can't go in a panel."));
            }
        }
    }
    if let Some(w) = &l.window_buttons {
        if w.side != "left" && w.side != "right" {
            e.push("The window buttons go on the left or the right.".into());
        }
        if !w.buttons.iter().any(|x| x == "close") {
            e.push("Windows always keep their close button.".into());
        }
        for x in &w.buttons {
            if !BUTTONS.contains(&x.as_str()) {
                e.push(format!("\"{x}\" isn't a window button."));
            }
        }
    }
    e
}

/// labwc's <titlebar><layout> for these buttons (rc.xml): "icon:iconify,max,close" (right) or "close,iconify,max:" (left).
pub fn titlebar(w: &WindowButtons) -> String {
    let bs: Vec<&str> = w.buttons.iter().map(String::as_str).filter(|b| *b != "icon").collect();
    if w.side == "left" {
        format!("{}:", bs.join(","))
    } else {
        format!("icon:{}", bs.join(","))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lay(j: &str) -> Layout {
        serde_json::from_str(j).unwrap()
    }

    #[test]
    fn the_ready_made_layouts_are_fine() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../linux/desktop/layouts");
        let mut n = 0;
        for f in std::fs::read_dir(&dir).unwrap().flatten() {
            if f.path().extension().and_then(|x| x.to_str()) != Some("json") {
                continue;
            }
            let t = std::fs::read_to_string(f.path()).unwrap();
            let l = parse(&t).unwrap_or_else(|e| panic!("{}: {e:?}", f.path().display()));
            assert_eq!(format!("{}.json", l.id), f.file_name().to_string_lossy(), "the file is named after its id");
            n += 1;
        }
        assert!(n >= 6, "Hearth, Steam-style, Plasma-, GNOME-, Windows 11- and macOS-style");
    }

    #[test]
    fn unknown_settings_and_blocks_are_refused() {
        let l = lay(r#"{"format":1,"id":"x","name":"X","desktop":{"look":"steam","barEdge":"middle"},
            "bar":{"start":["menu","rm -rf"],"hidden":["menu"]}}"#);
        let e = check(&l);
        assert!(e.iter().any(|x| x.contains("\"look\" isn't a setting")));
        assert!(e.iter().any(|x| x.contains("\"barEdge\" can't be")));
        assert!(e.iter().any(|x| x.contains("\"rm -rf\" can't go")));
        assert!(e.iter().any(|x| x.contains("can't be hidden") || x.contains("twice")));
    }

    #[test]
    fn panels_dont_sit_on_the_bar_or_the_dock_or_each_other() {
        let l = lay(r#"{"format":1,"id":"x","name":"X","desktop":{"barLayout":"one","barEdge":"bottom","dockFloat":"off"},
            "panels":[{"edge":"bottom","items":["clock"]},{"edge":"left","items":["clock"]},{"edge":"left","items":["pager"]}]}"#);
        let e = check(&l);
        assert!(e.iter().any(|x| x.contains("main bar is on the bottom")));
        assert!(e.iter().any(|x| x.contains("panel on the left already")));
        let split = lay(r#"{"format":1,"id":"x","name":"X","panels":[{"edge":"bottom","items":["clock"]}]}"#);
        assert!(check(&split).iter().any(|x| x.contains("dock is along the bottom")));
    }

    #[test]
    fn windows_keep_their_close_button() {
        let l = lay(r#"{"format":1,"id":"x","name":"X","windowButtons":{"side":"left","buttons":["iconify","max"]}}"#);
        assert!(check(&l).iter().any(|x| x.contains("close button")));
        let w = WindowButtons { side: "left".into(), buttons: vec!["close".into(), "iconify".into(), "max".into()] };
        assert_eq!(titlebar(&w), "close,iconify,max:");
        let r = WindowButtons { side: "right".into(), buttons: vec!["close".into()] };
        assert_eq!(titlebar(&r), "icon:close");
    }
}
