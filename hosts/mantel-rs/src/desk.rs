//! The desktop's layout, as the QML Mantel reads it (docs/mockups/checklists/desktop-layouts.md; hosts/hearth-layout):
//! desktop.json's barLayout, barEdge, dockFloat, dockMode, gameModeAt and clockAt, and bar.json's order of the main
//! bar's blocks ("items": start, centre, end, hidden) and its extra panels ("p:<id>"). Read only: hearth-layout and
//! Settings write them; this follows them live (an inotify watch on the two folders: asleep otherwise).
//! The order is filled in as widget-tools.js barLayout() does it: every known block once, a missing one in its
//! default place, the layouts' own blocks only where they're put, the flame never hidden.

use serde_json::{Map, Value};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Panel {
    pub key: String,
    pub edge: String,
    pub items: Vec<String>,
    pub size: u32,
    pub auto_hide: bool,
    /// "" (full width) | "floating" | "centred"
    pub style: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Desk {
    /// the main bar's edge: top | bottom | left | right
    pub edge: String,
    /// the floating dock is on
    pub dock: bool,
    /// your apps are in the main bar (no floating dock, and not GNOME-style's "none")
    pub apps_in_bar: bool,
    pub start: Vec<String>,
    pub centre: Vec<String>,
    pub end: Vec<String>,
    /// "" | "start" | "before-status"
    pub game_at: String,
    /// "" | "end" | "end-one"
    pub clock_at: String,
    pub panels: Vec<Panel>,
    /// the layout in place ("" none chosen) and the app menu's style (the Search block shows with a layout or Tiles)
    pub layout: String,
    pub app_menu: String,
}

impl Default for Desk {
    fn default() -> Desk {
        from(&Map::new(), &Map::new())
    }
}

impl Desk {
    pub fn vertical(&self) -> bool {
        self.edge == "left" || self.edge == "right"
    }
    /// What the surfaces depend on: a change here makes the bar, the dock and the panels again.
    pub fn surfaces_key(&self) -> String {
        let p: Vec<String> = self.panels.iter().map(|p| format!("{}:{}:{}:{}", p.key, p.edge, p.size, p.style)).collect();
        format!("{}/{}/{}", self.edge, self.dock, p.join(","))
    }
}

fn obj(p: &std::path::Path) -> Map<String, Value> {
    std::fs::read(p).ok().and_then(|b| serde_json::from_slice::<Value>(&b).ok()).and_then(|v| v.as_object().cloned()).unwrap_or_default()
}
fn config() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".config"))
}
pub fn bar_json() -> PathBuf {
    config().join("nimbus/bar.json")
}

pub fn read() -> Desk {
    from(&obj(&hearth_ui::theme::desktop_json()), &obj(&bar_json()))
}

fn strs(v: Option<&Value>) -> Vec<String> {
    v.and_then(Value::as_array).map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default()
}

/// The main bar's order with every known block once (widget-tools.js barLayout).
pub fn order(saved: Option<&Value>) -> (Vec<String>, Vec<String>, Vec<String>) {
    use hearth_layout::blocks;
    let o = saved.and_then(Value::as_object).cloned().unwrap_or_default();
    let mut seen: Vec<String> = Vec::new();
    let (mut start, mut centre, mut end, mut hidden) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for z in ["start", "centre", "end", "hidden"] {
        for id in strs(o.get(z)) {
            let Some(b) = blocks::find(&id) else { continue };
            if !blocks::in_bar(&id) || seen.contains(&id) {
                continue;
            }
            seen.push(id.clone());
            let zone = if z == "hidden" && b.keep { b.zone } else { z };
            match zone {
                "start" => start.push(id),
                "centre" => centre.push(id),
                "end" => end.push(id),
                _ => hidden.push(id),
            }
        }
    }
    // the bar's own items not saved: in their default place (next to the one before them in the default list)
    let all: Vec<&blocks::Block> = blocks::bar_items().collect();
    for (k, b) in all.iter().enumerate() {
        if seen.iter().any(|x| x == b.id) {
            continue;
        }
        let z = if b.zone == "start" { &mut start } else { &mut end };
        let mut at: Option<usize> = None;
        for prev in all[..k].iter().rev() {
            if let Some(i) = z.iter().position(|x| x == prev.id) {
                at = Some(i + 1);
                break;
            }
        }
        let i = at.unwrap_or(if b.zone == "start" { z.len() } else { 0 });
        z.insert(i.min(z.len()), b.id.to_string());
    }
    let _ = hidden;
    (start, centre, end)
}

pub fn from(d: &Map<String, Value>, bar: &Map<String, Value>) -> Desk {
    let s = |k: &str| d.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    let one = s("barLayout") == "one";
    let edge = if one {
        match s("barEdge").as_str() {
            e @ ("bottom" | "top" | "left" | "right") => e.to_string(),
            _ => "bottom".into(),
        }
    } else {
        "top".into()
    };
    let df = s("dockFloat");
    let dock = df != "off" && df != "none";
    let (start, centre, end) = order(bar.get("items"));
    let mut panels: Vec<Panel> = Vec::new();
    let mut keys: Vec<&String> = bar.keys().filter(|k| k.starts_with("p:")).collect();
    keys.sort();
    for k in keys {
        let Some(p) = bar[k].as_object() else { continue };
        let edge = p.get("edge").and_then(Value::as_str).filter(|e| ["top", "bottom", "left", "right"].contains(e)).unwrap_or("bottom").to_string();
        let items: Vec<String> = strs(p.get("items")).into_iter().filter(|x| hearth_layout::blocks::in_panel(x)).collect();
        let size = p.get("size").and_then(Value::as_f64).filter(|v| (28.0..=96.0).contains(v)).map(|v| v.round() as u32).unwrap_or(44);
        // a panel on another screen waits for it (this test draws on one screen)
        let scr = p.get("screen").and_then(Value::as_str).unwrap_or("");
        if !scr.is_empty() && std::env::var("HEARTH_MANTEL_ALL_SCREENS").is_err() && !scr.is_empty() && scr != primary_screen_name() {
            continue;
        }
        panels.push(Panel {
            key: k[2..].to_string(),
            edge,
            items,
            size,
            auto_hide: p.get("autoHide").and_then(Value::as_bool) == Some(true),
            style: p.get("style").and_then(Value::as_str).filter(|v| ["floating", "centred"].contains(v)).unwrap_or("").to_string(),
        });
    }
    Desk { edge, dock, apps_in_bar: !dock && df != "none", start, centre, end, game_at: s("gameModeAt"), clock_at: s("clockAt"), panels, layout: s("layout"), app_menu: s("appMenu") }
}

/// The main screen's name, when labwc says it (WAYLAND output names aren't read here yet: every panel with a screen
/// name other than "" waits; the panels a layout makes have none).
fn primary_screen_name() -> String {
    std::env::var("HEARTH_MANTEL_SCREEN").unwrap_or_default()
}

/// Wakes when desktop.json or bar.json is written (inotify on their folders).
pub fn changes() -> futures::channel::mpsc::UnboundedReceiver<()> {
    let (tx, rx) = futures::channel::mpsc::unbounded();
    let _ = std::thread::Builder::new().name("mantel-desk".into()).stack_size(64 * 1024).spawn(move || loop {
        let files = [hearth_ui::theme::desktop_json(), bar_json()];
        let fd = unsafe { libc::inotify_init1(libc::IN_CLOEXEC) };
        if fd < 0 {
            return;
        }
        let mut n = 0;
        for f in &files {
            let Some(dir) = f.parent() else { continue };
            let _ = std::fs::create_dir_all(dir);
            let c = std::ffi::CString::new(dir.to_string_lossy().as_bytes()).unwrap_or_default();
            if unsafe { libc::inotify_add_watch(fd, c.as_ptr(), libc::IN_CLOSE_WRITE | libc::IN_MOVED_TO | libc::IN_DELETE) } >= 0 {
                n += 1;
            }
        }
        if n == 0 {
            unsafe { libc::close(fd) };
            std::thread::sleep(std::time::Duration::from_secs(60));
            continue;
        }
        let mut buf = [0u8; 4096];
        loop {
            let r = unsafe { libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
            if r <= 0 {
                break;
            }
            let text = String::from_utf8_lossy(&buf[..r as usize]);
            if text.contains("desktop.json") || text.contains("bar.json") {
                if tx.unbounded_send(()).is_err() {
                    unsafe { libc::close(fd) };
                    return;
                }
            }
        }
        unsafe { libc::close(fd) };
    });
    rx
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn m(v: Value) -> Map<String, Value> {
        v.as_object().unwrap().clone()
    }

    #[test]
    fn the_default_is_hearths_own() {
        let d = from(&Map::new(), &Map::new());
        assert_eq!(d.edge, "top");
        assert!(d.dock && !d.apps_in_bar);
        assert_eq!(d.start[0], "menu");
        assert_eq!(d.end.last().map(String::as_str), Some("notifications"));
        assert!(d.centre.is_empty());
        assert!(!d.start.iter().chain(&d.end).any(|x| x == "overview" || x == "showdesktop"), "a layout's own blocks only where they're put");
    }

    #[test]
    fn a_layout_is_read_as_the_qml_mantel_reads_it() {
        let d = from(
            &m(json!({"barLayout": "one", "barEdge": "bottom", "dockFloat": "off", "gameModeAt": "start", "clockAt": "end"})),
            &m(json!({"items": {"start": [], "centre": ["menu", "search", "taskview"], "end": ["tray", "network"], "hidden": ["menu", "desktops"]},
                      "p:a": {"edge": "left", "items": ["clock", "nope"], "size": 500, "style": "floating"}})),
        );
        assert_eq!(d.edge, "bottom");
        assert!(!d.dock && d.apps_in_bar);
        assert_eq!(d.centre, ["menu", "search", "taskview"]);
        assert!(!d.start.contains(&"desktops".to_string()) && !d.end.contains(&"desktops".to_string()));
        assert_eq!(d.panels.len(), 1);
        assert_eq!(d.panels[0].items, ["clock"]);
        assert_eq!(d.panels[0].size, 44);
        assert_eq!(d.panels[0].style, "floating");
        let g = from(&m(json!({"barLayout": "one", "barEdge": "top", "dockFloat": "none"})), &Map::new());
        assert!(!g.dock && !g.apps_in_bar, "GNOME-style: no dock and no apps in the bar");
    }
}
