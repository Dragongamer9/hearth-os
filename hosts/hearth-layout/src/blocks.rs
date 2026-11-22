//! Every block a layout can put in the main bar or a panel. The names are the ones Mantel already uses
//! (widget-tools.js BAR_ITEMS and PANEL_ITEMS), so a layout written here is read as it is by the QML Mantel's
//! BarLayout.qml and ExtraPanels.qml, and by Mantel in Rust. The blocks the approved layouts need that neither Mantel
//! draws yet are marked `built: false`: a layout may name them (they're part of the approved mockups), and both
//! Mantels leave them out until they're built.

/// Where a block can go.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Where {
    /// the main bar (its start, centre or end)
    Bar,
    /// an extra panel
    Panel,
    /// both
    Both,
}

#[derive(Debug, Clone, Copy)]
pub struct Block {
    pub id: &'static str,
    pub name: &'static str,
    pub place: Where,
    /// the main bar's default end ("start" | "end"), as BAR_ITEMS has it; "" for the panel-only and new ones
    pub zone: &'static str,
    /// drawn by both Mantels today
    pub built: bool,
    /// never hidden (the flame; Game Mode, which the main bar always has)
    pub keep: bool,
}

const fn b(id: &'static str, name: &'static str, place: Where, zone: &'static str, built: bool, keep: bool) -> Block {
    Block { id, name, place, zone, built, keep }
}

use Where::*;

/// The main bar's items in their default order (BAR_ITEMS), then the panels' (PANEL_ITEMS), then the new ones.
pub const BLOCKS: &[Block] = &[
    b("menu", "App menu (the flame)", Both, "start", true, true),
    b("search", "Search field", Bar, "start", true, false),
    b("desktops", "Desktops", Bar, "start", true, false),
    b("activity", "Activity", Bar, "start", true, false),
    b("window", "The window in front", Bar, "start", true, false),
    b("jobs", "Copies and downloads", Bar, "end", true, false),
    b("tray", "Apps' tray icons", Both, "end", true, false),
    b("background", "Background apps", Bar, "end", true, false),
    b("privacy", "Camera and microphone in use", Bar, "end", true, false),
    b("printers", "Printing", Bar, "end", true, false),
    b("disks", "Disks and devices", Bar, "end", true, false),
    b("discord", "Discord", Bar, "end", true, false),
    b("keyboard", "On-screen keyboard", Bar, "end", true, false),
    b("layout", "Keyboard layout", Both, "end", true, false),
    b("network", "Network", Bar, "end", true, false),
    b("bluetooth", "Bluetooth", Bar, "end", true, false),
    b("volume", "Volume", Bar, "end", true, false),
    b("battery", "Battery", Bar, "end", true, false),
    b("notifications", "Notifications", Both, "end", true, false),
    // the extra panels' own (ExtraPanel.qml)
    b("apps", "Your apps (the dock)", Panel, "", true, false),
    b("windows", "Window list", Panel, "", true, false),
    b("pager", "Desktops (pager)", Panel, "", true, false),
    b("spacer", "Space", Panel, "", true, false),
    b("clock", "Clock", Panel, "", true, false),
    b("status", "Volume, network and battery", Panel, "", true, false),
    b("gamemode", "Game Mode", Panel, "", true, true),
    b("widget:weather", "Weather", Panel, "", true, false),
    b("widget:monitor", "System monitor", Panel, "", true, false),
    b("widget:battery", "Battery", Panel, "", true, false),
    b("widget:timer", "Timer", Panel, "", true, false),
    // new with the approved layouts (desktop-layouts boards 1-6): named here, drawn once they're built
    b("overview", "Desktops pill (opens the overview)", Both, "", false, false),
    b("appname", "The app in front, by name", Both, "", false, false),
    b("appmenus", "App menus (File, Edit…)", Both, "", false, false),
    b("quickaccess", "Quick Access", Both, "", false, false),
    b("showdesktop", "Show desktop", Both, "", false, false),
    b("taskview", "Task view", Both, "", false, false),
];

pub fn find(id: &str) -> Option<&'static Block> {
    BLOCKS.iter().find(|x| x.id == id)
}

/// May this block go in the main bar?
pub fn in_bar(id: &str) -> bool {
    find(id).map(|x| matches!(x.place, Bar | Both)).unwrap_or(false)
}
/// May this block go in an extra panel?
pub fn in_panel(id: &str) -> bool {
    find(id).map(|x| matches!(x.place, Panel | Both)).unwrap_or(false)
}

/// The main bar's items that BarLayout.qml knows (its saved order may only hold these).
pub fn bar_items() -> impl Iterator<Item = &'static Block> {
    BLOCKS.iter().filter(|x| !x.zone.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The ids must stay the ones the QML Mantel reads (widget-tools.js), or a layout would lose its blocks there.
    #[test]
    fn ids_match_the_qml_mantel() {
        let js = include_str!("../../linux/desktop/shell/widget-tools.js");
        for x in BLOCKS.iter().filter(|x| x.built) {
            assert!(js.contains(&format!("id: \"{}\"", x.id)), "{} isn't in widget-tools.js", x.id);
        }
    }

    #[test]
    fn ids_are_unique() {
        for (i, x) in BLOCKS.iter().enumerate() {
            assert!(BLOCKS[i + 1..].iter().all(|y| y.id != x.id), "{} twice", x.id);
        }
    }
}
