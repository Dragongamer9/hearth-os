//! Describe it in Hearth's own apps (desktop-layouts board 11, Hearth Files; part 12: every Hearth app). One shared
//! core: each app names its own layout choices (its knobs), its places a new feature can go, and what it can show and
//! do there besides Hearth's own (feature.rs SOURCES and ACTIONS). Your words go to your AI with only those; the answer
//! is checked here like a layout or a feature, and what it can't do becomes an idea (idea.rs).
//!
//! What you keep is one file per app, `~/.config/nimbus/apps/<app>.json` (settings_file: atomic, .bak):
//! `{"format": 1, "layout": {knob: value}, "features": [Feature], "undo": {"layout", "features"}}`. Keep this saves it
//! with the one before as `undo`; Undo puts that back; Back to the original (View › Layout) empties it. Nothing an app
//! does changes: only how it's laid out, and features made from its own parts.

use crate::{feature, idea};
use serde_json::{json, Map, Value};

pub struct Knob {
    pub key: &'static str,
    pub what: &'static str,
    /// (value, how the chip says it); "" is the app's own way
    pub values: &'static [(&'static str, &'static str)],
}

pub struct AppSchema {
    pub app: &'static str,
    pub name: &'static str,
    pub knobs: &'static [Knob],
    /// places a feature can go in this app: (id, what it is); a feature's part is "<app>:<id>"
    pub parts: &'static [(&'static str, &'static str)],
    pub sources: &'static [(&'static str, &'static str)],
    pub actions: &'static [(&'static str, &'static str)],
    /// every piece of the app's own bars (the whole app can be rearranged: the owner, Nov 22, "they need to be able to
    /// edit the top bar as well, basicly the enditre app"): (id, what it is)
    pub items: &'static [(&'static str, &'static str)],
    /// where pieces can go: (zone, what it is, its pieces by default, in order); "hidden" is always there too
    pub zones: &'static [(&'static str, &'static str, &'static [&'static str])],
    /// pieces that can't be hidden (the way to View › Layout, Back to the original)
    pub keep: &'static [&'static str],
    /// the app's menus and right-click menus (the owner, Nov 22: "Modular app menues and right clicks as well"): their
    /// items reordered or hidden through the zone "menu:<id>", new items from features in the place "<app>:menu-<id>"
    pub menus: &'static [(&'static str, &'static str)],
}

/// Hearth Files (board 11).
pub const FILES: AppSchema = AppSchema {
    app: "files",
    name: "Hearth Files",
    knobs: &[
        Knob { key: "sidebar", what: "the sidebar", values: &[("", "sidebar as it is"), ("icons", "sidebar icons only"), ("hidden", "no sidebar")] },
        Knob { key: "preview", what: "the preview of the picked file", values: &[("", "preview as it is"), ("right", "preview on the right"), ("right-big", "preview right, bigger"), ("off", "no preview")] },
        Knob { key: "view", what: "how files are shown", values: &[("", "view as it is"), ("icons", "big icons"), ("list", "a list"), ("details", "details")] },
        Knob { key: "toolbar", what: "the buttons' labels", values: &[("", "buttons as they are"), ("compact", "icons only on the buttons")] },
        Knob { key: "tabs", what: "the tabs along the top", values: &[("", "tabs as they are"), ("hidden", "no tabs")] },
        Knob { key: "winbuttons", what: "how the window's own buttons look", values: &[("", "window buttons as they are"), ("dots", "window buttons as coloured dots"), ("square", "square window buttons")] },
        Knob { key: "status", what: "the status bar at the bottom", values: &[("", "status bar as it is"), ("hidden", "no status bar")] },
        Knob { key: "density", what: "how roomy it is", values: &[("", "as roomy as it is"), ("roomy", "roomier"), ("compact", "tighter")] },
    ],
    parts: &[
        ("downloads", "the Downloads folder's own bar (next to Sort my Downloads)"),
        ("folder", "every folder's bar, at the top"),
        ("menu-context", "the right-click menu, as a new item"),
        ("menu-view", "the View menu, as a new item"),
        ("menu-more", "the More (⋯) menu, as a new item"),
    ],
    sources: &[
        ("folder-pictures", "how many pictures are in this folder"),
        ("folder-videos", "how many videos are in this folder"),
        ("folder-old", "how many files here are older than a month"),
        ("folder-big", "how many files here are over 1 GB"),
        ("folder-count", "how many files are in this folder"),
        ("folder-size", "how big this folder is"),
    ],
    actions: &[
        ("sort-downloads", "Sort my Downloads"),
        ("select-pictures", "select the pictures here"),
        ("select-old", "select the files here older than a month"),
        ("select-big", "select the files here over 1 GB"),
    ],
    items: &[
        ("back", "Back"), ("forward", "Forward"), ("up", "Up a folder"), ("refresh", "Refresh"), ("address", "the address (where you are)"),
        ("search", "the search field"), ("new", "New"), ("cut", "Cut"), ("copy", "Copy"), ("paste", "Paste"), ("rename", "Rename"),
        ("share", "Share"), ("delete", "Delete"), ("sort", "Sort"), ("view", "View (and View › Layout)"), ("details", "Details (the preview)"),
        ("more", "More (⋯)"), ("sortdl", "Sort my Downloads (in Downloads)"), ("features", "your features' buttons"),
        ("tools", "a Briefcase's own buttons"), ("sep", "a divider"), ("spacer", "empty space that pushes the rest to the end"),
        ("tabs", "the tabs"), ("newtab", "New tab (+)"), ("title", "the folder's name"), ("minimize", "the window's Minimize"),
        ("maximize", "the window's Maximize"), ("close", "the window's Close"),
    ],
    zones: &[
        ("titlebar", "the window's own bar at the very top (it moves the window)", &["tabs", "newtab", "spacer", "minimize", "maximize", "close"]),
        ("top", "the top bar (under the tabs)", &["back", "forward", "up", "refresh", "address", "search"]),
        ("bar", "the button bar (under the top bar)", &["new", "sep", "cut", "copy", "paste", "rename", "share", "delete", "sep", "sort", "view", "tools", "spacer", "sortdl", "features", "details", "more"]),
        ("bottom", "a bar at the bottom (above the status bar)", &[]),
    ],
    // nothing has to stay (the owner, Nov 22: "it can be Whatever they want it to be"): a window without Close still
    // closes with Alt+F4 or Super+Q, and Describe it and Back to the original are also on the right-click of empty space
    keep: &[],
    menus: &[("context", "the right-click menu (on files and on empty space)"), ("view", "View"), ("sort", "Sort and group"), ("new", "New"), ("more", "More (⋯)")],
};

/// What every Hearth app has: its window's own bar (hearth_ui::describe::title_bar) and how its buttons look.
pub const WINDOW_KNOBS: &[Knob] = &[
    Knob { key: "winbuttons", what: "how the window's own buttons look", values: &[("", "window buttons as they are"), ("dots", "window buttons as coloured dots"), ("square", "square window buttons")] },
    Knob { key: "density", what: "how roomy it is", values: &[("", "as roomy as it is"), ("roomy", "roomier"), ("compact", "tighter")] },
];
pub const WINDOW_ITEMS: &[(&str, &str)] = &[
    ("mark", "the app's icon"), ("title", "the app's name"), ("middle", "what the app has in its bar's middle"),
    ("spacer", "empty space (it moves the window)"), ("minimize", "the window's Minimize"), ("maximize", "the window's Maximize"),
    ("close", "the window's Close"),
];
pub const WINDOW_ZONES: &[(&str, &str, &[&str])] = &[("titlebar", "the window's own bar at the very top", &["mark", "title", "spacer", "middle", "spacer", "minimize", "maximize", "close"])];

/// Task Manager (the owner, Nov 22: "EVERY one of our apps needs this", "Modular app menues and right clicks as well"):
/// its window bar, the side rail of pages, each page's buttons and its right-click menus.
pub const TASK_MANAGER: AppSchema = AppSchema {
    app: "task-manager",
    name: "Task Manager",
    knobs: &[
        Knob { key: "winbuttons", what: "how the window's own buttons look", values: &[("", "window buttons as they are"), ("dots", "window buttons as coloured dots"), ("square", "square window buttons")] },
        Knob { key: "density", what: "how roomy it is", values: &[("", "as roomy as it is"), ("roomy", "roomier"), ("compact", "tighter")] },
        Knob { key: "rail", what: "the pages down the side", values: &[("", "the side as it is"), ("icons", "icons only down the side"), ("hidden", "no side")] },
    ],
    parts: &[("window", "the app's window, in its own bar")],
    sources: &[],
    actions: &[],
    items: &[
        ("mark", "the app's icon"), ("title", "the app's name"), ("middle", "the search"), ("spacer", "empty space"),
        ("minimize", "the window's Minimize"), ("maximize", "the window's Maximize"), ("close", "the window's Close"),
        ("fold", "the ☰ that folds the side"), ("processes", "Processes"), ("performance", "Performance"), ("history", "App history"),
        ("startup", "Startup apps"), ("users", "Users"), ("details", "Details"), ("services", "Services"), ("tasks", "Scheduled tasks"),
        ("settings", "Settings"),
        ("run-new-task", "Run new task"), ("efficiency-mode", "Efficiency mode"), ("open-file-location", "Open file location"),
        ("end-task", "End task"), ("columns", "Columns… (Details)"), ("delete-usage-history", "Delete usage history"), ("enable", "Enable"),
        ("disable", "Disable"), ("start", "Start"), ("stop", "Stop"), ("restart", "Restart"), ("new-task", "+ New task"),
        ("turn-on", "Turn on"), ("turn-off", "Turn off"),
        ("expand", "Expand (right-click)"), ("collapse", "Collapse (right-click)"), ("end-process-tree", "End process tree"),
        ("set-priority", "Set priority"), ("run-on-these-cores", "Run on these cores…"), ("limit-cpu-memory", "Limit CPU / memory…"),
        ("resume", "Resume"), ("suspend", "Suspend"), ("keep-this-running-checkpoint", "Keep this running (checkpoint)"),
        ("never-close-when-memory-runs-out", "Never close when memory runs out"), ("go-to-details", "Go to details"),
        ("search-online", "Search online"), ("properties", "Properties"), ("set-affinity", "Set affinity"), ("create-dump-file", "Create dump file"),
    ],
    zones: &[
        ("titlebar", "the window's own bar at the very top", &["mark", "title", "spacer", "middle", "spacer", "minimize", "maximize", "close"]),
        ("rail", "the pages down the side, top to bottom (spacer pushes the rest to the bottom)", &["fold", "processes", "performance", "history", "startup", "users", "details", "services", "tasks", "spacer", "settings"]),
        ("bar", "the page's buttons, on the right of its title", &["run-new-task", "efficiency-mode", "open-file-location", "end-task"]),
    ],
    keep: &[],
    menus: &[("row", "the right-click menu on a process (Processes)"), ("details", "the right-click menu on a process (Details)"), ("list", "the right-click menu on the other pages' rows")],
};

/// An app that has, so far, its window's bar and new features in it (its own bars come per app, as Files' did).
macro_rules! window_app {
    ($name:ident, $id:literal, $title:literal) => {
        pub const $name: AppSchema = AppSchema {
            app: $id,
            name: $title,
            knobs: WINDOW_KNOBS,
            parts: &[("window", "the app's window, in its own bar")],
            sources: &[],
            actions: &[],
            items: WINDOW_ITEMS,
            zones: WINDOW_ZONES,
            keep: &[],
            menus: &[],
        };
    };
}
window_app!(SETTINGS, "settings", "Hearth Settings");
window_app!(STORE, "store", "Hearth Store");
window_app!(NOTES, "notes", "Hearth Notes");
window_app!(CAPTURE, "capture", "Capture");
window_app!(RESMON, "resmon", "Resource Monitor");
window_app!(FAMILY, "family", "Family and friends");
window_app!(SERVER, "server", "Server mode");
window_app!(RESTORE, "system-restore", "System Restore");
window_app!(RELIABILITY, "reliability", "Reliability history");
window_app!(G15, "g15", "Logitech G15");
window_app!(IDEAS, "ideas", "Hearth Ideas");

/// Every app with Describe it, by its id (hearth-layout describe-app <app>).
pub const APPS: &[&AppSchema] = &[&FILES, &SETTINGS, &STORE, &TASK_MANAGER, &NOTES, &CAPTURE, &RESMON, &FAMILY, &SERVER, &RESTORE, &RELIABILITY, &G15, &IDEAS];

pub fn find(app: &str) -> Option<&'static AppSchema> {
    APPS.iter().copied().find(|a| a.app == app)
}

/// What the AI is told for this app.
pub fn rules(a: &AppSchema) -> String {
    let knobs: Vec<String> = a.knobs.iter().map(|k| format!("{} ({}): {}", k.key, k.what, k.values.iter().map(|(v, l)| format!("\"{v}\" = {l}")).collect::<Vec<_>>().join(", "))).collect();
    let parts: Vec<String> = a.parts.iter().map(|(id, n)| format!("{}:{id} ({n})", a.app)).collect();
    let sources: Vec<String> = feature::SOURCES.iter().chain(a.sources.iter()).map(|(id, n)| format!("{id} ({n})")).collect();
    let actions: Vec<String> = feature::ACTIONS.iter().chain(a.actions.iter()).map(|(id, n)| format!("{id} ({n})")).collect();
    format!(
        "You change how {name} is laid out for its owner, and may add features made only from its own parts. What its buttons do never changes.\n\
Answer with ONE JSON object and nothing else, one of:\n\
1. {{\"kind\": \"app\", \"layout\": {{<knob>: <value>, ...only the ones that change}}, \"features\": [<feature>, ...up to 3], \"changes\": [\"short plain sentences of what changed\"]}}\n\
   knobs: {knobs}\n\
   and \"zones\": {{<zone>: [pieces in order], ..., \"hidden\": [...]}} to move, reorder or hide the app's own pieces (only the zones that change; a piece in one place only; \"sep\" and \"spacer\" as often as you like)\n\
   zones: {zones}\n\
   menus (zone \"menu:<id>\": the items' ids in the order wanted, the rest after them; ids in \"hidden\" leave every menu and bar): {menus}\n\
   pieces: {items}\n\
{blocks}\
   <feature> = {{\"format\": 1, \"id\": \"<lowercase-dashes>\", \"name\": \"...\", \"part\": <a place>, \"shows\": [{{\"source\", \"label\"}}], \"every\": seconds, \"click\": {{\"action\", \"target\"}}}}\n\
   places: {parts}\n\
   sources: {sources}\n\
   actions: {actions}\n\
2. When it needs something {name} doesn't have: {{\"kind\": \"needs\", \"needs\": [\"what's missing, in a few words\"], \"why\": \"one or two plain sentences\", \"closest\": <feature or null>, \"changes\": []}}\n\
Never write code, commands or file paths. Use plain words for the owner in \"changes\".",
        name = a.name,
        knobs = knobs.join("; "),
        zones = a.zones.iter().map(|(z, w, d)| format!("{z} ({w}; now {})", d.join(", "))).collect::<Vec<_>>().join("; "),
        menus = a.menus.iter().map(|(m, w)| format!("{m} ({w})")).collect::<Vec<_>>().join(", "),
        items = a.items.iter().map(|(i, w)| format!("{i} ({w})")).collect::<Vec<_>>().join(", "),
        blocks = block_rules(&catalog()),
        parts = parts.join(", "),
        sources = sources.join(", "),
        actions = actions.join(", ")
    )
}

/// What the AI is told of the app's blocks (none when the app didn't say).
pub fn block_rules(cat: &Value) -> String {
    let Some(pages) = cat.get("pages").and_then(Value::as_array).filter(|p| !p.is_empty()) else { return String::new() };
    let list: Vec<String> = pages
        .iter()
        .map(|p| {
            let bl: Vec<String> = p.get("blocks").and_then(Value::as_array).map(|l| l.iter().map(|b| format!("{} ({}; now x {} y {} w {} h {})", b["id"].as_str().unwrap_or(""), b["title"].as_str().unwrap_or(""), b["x"], b["y"], b["w"], b["h"])).collect()).unwrap_or_default();
            format!("{} ({}): {}", p["id"].as_str().unwrap_or(""), p["title"].as_str().unwrap_or(""), bl.join(", "))
        })
        .collect();
    format!(
        "   and \"blocks\": {{<page>: {{<block>: {{\"x\", \"y\", \"w\", \"h\" on a 12 x 12 grid (x + w and y + h at most 12), \"hidden\": true|false, \"corners\" 0-48, \"seeThrough\" 0-90, \"font\": {}, \"size\": \"\"|\"smaller\"|\"bigger\", \"accent\": \"#rrggbb\"}}}}}} to move, size, hide, add back (hidden false) or restyle the app's blocks (only the blocks that change; {{}} for a page puts it back as it was)\n   pages and blocks: {}\n",
        BLOCK_FONTS.iter().map(|f| format!("\"{f}\"")).collect::<Vec<_>>().join("|"),
        list.join("; ")
    )
}

/// A feature for this app, checked: Hearth's own rules (feature.rs), with the app's places, sources and actions too.
pub fn check_feature(a: &AppSchema, f: &feature::Feature) -> feature::Checked {
    let mut g = f.clone();
    let mut needs = Vec::new();
    match f.part.strip_prefix(&format!("{}:", a.app)) {
        Some(p) if a.parts.iter().any(|(id, _)| *id == p) => g.part = a.app.to_string(),
        _ => {
            needs.push(format!("a place in {} called \"{}\"", a.name, f.part));
            g.part = a.app.to_string();
        }
    }
    for s in g.shows.iter_mut() {
        if a.sources.iter().any(|(id, _)| *id == s.source) {
            s.source = "text".into();
        }
    }
    if let Some(c) = g.click.as_mut() {
        if a.actions.iter().any(|(id, _)| *id == c.action) {
            c.action = "none".into();
            c.target.clear();
        }
    }
    // "files" must be a place feature.rs knows; it is (feature::PARTS)
    let mut c = feature::check(&g);
    for n in needs {
        if !c.needs.contains(&n) {
            c.needs.push(n);
        }
    }
    c
}

/// The knobs that change, checked: an unknown knob or value is a problem, never kept.
pub fn check_layout(a: &AppSchema, l: &Map<String, Value>) -> Result<Map<String, Value>, Vec<String>> {
    let mut out = Map::new();
    let mut errs = Vec::new();
    for (k, v) in l {
        if k == "blocks" {
            match check_blocks(a, v, &catalog()) {
                Ok(b) => {
                    out.insert(k.clone(), b);
                }
                Err(mut e) => errs.append(&mut e),
            }
            continue;
        }
        if k == "zones" {
            match check_zones(a, v) {
                Ok(z) => {
                    out.insert(k.clone(), z);
                }
                Err(mut e) => errs.append(&mut e),
            }
            continue;
        }
        let Some(knob) = a.knobs.iter().find(|x| x.key == k) else {
            errs.push(format!("{} has no \"{k}\" to change.", a.name));
            continue;
        };
        match v.as_str() {
            Some(s) if knob.values.iter().any(|(x, _)| *x == s) => {
                out.insert(k.clone(), json!(s));
            }
            _ => errs.push(format!("{} can't be \"{}\".", knob.what, v.as_str().unwrap_or("that"))),
        }
    }
    if errs.is_empty() { Ok(out) } else { Err(errs) }
}

/// The app's pieces moved, reordered or hidden, checked: known pieces and zones, each piece in one place (dividers and
/// space as often as wanted), and the pieces that must stay somewhere still there.
pub fn check_zones(a: &AppSchema, v: &Value) -> Result<Value, Vec<String>> {
    // (kept pieces taken out of their zone and put nowhere come back in their own place: the app does that)
    let Some(m) = v.as_object() else { return Err(vec!["The app's bars couldn't be read.".into()]) };
    let mut errs = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    let mut out = Map::new();
    for (z, items) in m {
        if let Some(menu) = z.strip_prefix("menu:") {
            // a menu's items: its own ids (any of the menu's, by name), in the order wanted; "sep" for a line
            if !a.menus.iter().any(|(id, _)| *id == menu) {
                errs.push(format!("{} has no menu called \"{menu}\".", a.name));
                continue;
            }
            let list: Vec<String> = items.as_array().map(|x| x.iter().filter_map(|i| i.as_str().map(str::to_string)).collect()).unwrap_or_default();
            if list.iter().any(|i| i.is_empty() || i.len() > 48 || !i.chars().all(|c| c.is_ascii_alphanumeric() || "-_:=.".contains(c))) {
                errs.push(format!("A menu item is named by its id (\"{menu}\")."));
            }
            out.insert(z.clone(), json!(list));
            continue;
        }
        if z != "hidden" && !a.zones.iter().any(|(id, _, _)| id == z) {
            errs.push(format!("{} has no place called \"{z}\".", a.name));
            continue;
        }
        let list: Vec<String> = items.as_array().map(|x| x.iter().filter_map(|i| i.as_str().map(str::to_string)).collect()).unwrap_or_default();
        for i in &list {
            if !a.items.iter().any(|(id, _)| id == i) {
                errs.push(format!("{} has no \"{i}\" to move.", a.name));
            } else if i != "sep" && i != "spacer" {
                if seen.contains(i) {
                    errs.push(format!("\"{i}\" can be in one place only."));
                }
                seen.push(i.clone());
                if z == "hidden" && a.keep.contains(&i.as_str()) {
                    errs.push(format!("\"{i}\" stays (it's the way back to the original)."));
                }
            }
        }
        out.insert(z.clone(), json!(list));
    }
    // a piece that leaves its zone without being placed elsewhere stays where it was: the app fills it in, so nothing is
    // lost by a half answer; only the zones named change
    if errs.is_empty() { Ok(Value::Object(out)) } else { Err(errs) }
}

/// The app's pages made of blocks, as the app itself tells (hearth_ui blocks.rs catalog, in HEARTH_APP_BLOCKS when the
/// app runs hearth-layout): {"cols", "rows", "pages": [{"id", "title", "blocks": [{"id", "title", "x", "y", "w", "h"}]}]}.
/// Null when the app didn't say (then only the shape of a block is checked).
pub fn catalog() -> Value {
    std::env::var("HEARTH_APP_BLOCKS").ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(Value::Null)
}

pub const BLOCK_FONTS: &[&str] = &["", "Noto Serif", "Noto Sans Mono", "Inter Display"];

/// Blocks moved, sized, hidden or restyled, checked (hearth_ui blocks.rs reads the same): {page: {block: {x, y, w, h,
/// hidden, corners 0-48, seeThrough 0-90, font, size "" | smaller | bigger, accent "#rrggbb"}}} on a 12 × 12 grid. An
/// empty page is "that page as the app has it".
pub fn check_blocks(a: &AppSchema, v: &Value, cat: &Value) -> Result<Value, Vec<String>> {
    let Some(pages) = v.as_object() else { return Err(vec!["The app's blocks couldn't be read.".into()]) };
    let cols = cat.get("cols").and_then(Value::as_u64).unwrap_or(12);
    let rows = cat.get("rows").and_then(Value::as_u64).unwrap_or(12);
    let idok = |s: &str| !s.is_empty() && s.len() <= 48 && s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    let mut errs = Vec::new();
    let mut out = Map::new();
    for (pg, bl) in pages {
        let known = cat.get("pages").and_then(Value::as_array).map(|ps| ps.iter().find(|p| p.get("id").and_then(Value::as_str) == Some(pg.as_str())).cloned());
        let page = match known {
            Some(None) => {
                errs.push(format!("{} has no page called \"{pg}\".", a.name));
                continue;
            }
            Some(Some(p)) => Some(p),
            None if idok(pg) => None,
            None => {
                errs.push(format!("A page is named by its id (\"{pg}\")."));
                continue;
            }
        };
        let Some(bm) = bl.as_object() else {
            errs.push(format!("The blocks of \"{pg}\" couldn't be read."));
            continue;
        };
        let mut po = Map::new();
        for (id, b) in bm {
            let def = page.as_ref().map(|p| p.get("blocks").and_then(Value::as_array).and_then(|l| l.iter().find(|x| x.get("id").and_then(Value::as_str) == Some(id.as_str())).cloned()));
            let def = match def {
                Some(None) => {
                    errs.push(format!("\"{pg}\" has no block called \"{id}\"."));
                    continue;
                }
                Some(Some(d)) => d,
                None if idok(id) => json!({}),
                None => {
                    errs.push(format!("A block is named by its id (\"{id}\")."));
                    continue;
                }
            };
            let Some(o) = b.as_object() else {
                errs.push(format!("\"{id}\" couldn't be read."));
                continue;
            };
            let mut bo = Map::new();
            for (k, x) in o {
                let n = x.as_u64();
                let ok = match k.as_str() {
                    "x" => n.map(|n| n < cols).unwrap_or(false),
                    "y" => n.map(|n| n < rows).unwrap_or(false),
                    "w" => n.map(|n| (1..=cols).contains(&n)).unwrap_or(false),
                    "h" => n.map(|n| (1..=rows).contains(&n)).unwrap_or(false),
                    "hidden" => x.is_boolean(),
                    "corners" => n.map(|n| n <= 48).unwrap_or(false),
                    "seeThrough" => n.map(|n| n <= 90).unwrap_or(false),
                    "font" => x.as_str().map(|f| BLOCK_FONTS.contains(&f)).unwrap_or(false),
                    "size" => x.as_str().map(|f| ["", "smaller", "bigger"].contains(&f)).unwrap_or(false),
                    "accent" => x.as_str().map(|c| c.is_empty() || (c.len() == 7 && c.starts_with('#') && c[1..].chars().all(|h| h.is_ascii_hexdigit()))).unwrap_or(false),
                    _ => {
                        errs.push(format!("A block has no \"{k}\" to change."));
                        continue;
                    }
                };
                if !ok {
                    errs.push(format!("\"{id}\" can't have {k} {x}."));
                } else {
                    bo.insert(k.clone(), if k == "accent" { json!(x.as_str().unwrap_or("").to_lowercase()) } else { x.clone() });
                }
            }
            // it stays on the grid (its own place for what isn't named)
            let g = |k: &str, d: u64| bo.get(k).and_then(Value::as_u64).or_else(|| def.get(k).and_then(Value::as_u64)).unwrap_or(d);
            if g("x", 0) + g("w", 1) > cols || g("y", 0) + g("h", 1) > rows {
                errs.push(format!("\"{id}\" would go off the page."));
            }
            po.insert(id.clone(), Value::Object(bo));
        }
        out.insert(pg.clone(), Value::Object(po));
    }
    if errs.is_empty() { Ok(Value::Object(out)) } else { Err(errs) }
}

/// The chips under the words (board 11): "Layout: sidebar icons only", "New feature: counts in Downloads".
pub fn chips(a: &AppSchema, layout: &Map<String, Value>, features: &[feature::Feature]) -> Vec<Value> {
    let mut v = Vec::new();
    for k in a.knobs {
        if let Some(s) = layout.get(k.key).and_then(Value::as_str) {
            if let Some((_, l)) = k.values.iter().find(|(x, _)| *x == s) {
                v.push(json!({"kind": "layout", "text": format!("Layout: {l}")}));
            }
        }
    }
    if let Some(z) = layout.get("zones").and_then(Value::as_object) {
        for (zone, items) in z {
            let menu = zone.strip_prefix("menu:").and_then(|m| a.menus.iter().find(|(id, _)| *id == m)).map(|(_, w)| *w);
            let what = a.zones.iter().find(|(id, _, _)| id == zone).map(|(_, w, _)| *w).or(menu).unwrap_or(if zone == "hidden" { "hidden" } else { "" });
            let names: Vec<&str> = items.as_array().map(|x| x.iter().filter_map(Value::as_str).filter(|i| *i != "sep" && *i != "spacer").collect()).unwrap_or_default();
            let what = what.split(" (").next().unwrap_or(what);
            v.push(json!({"kind": "layout", "text": if zone == "hidden" { format!("Layout: hide {}", names.join(", ")) } else { format!("Layout: {what}: {}", names.join(", ")) }}));
        }
    }
    if let Some(b) = layout.get("blocks").and_then(Value::as_object) {
        let cat = catalog();
        let title = |pg: &str, id: Option<&str>| -> String {
            let p = cat.get("pages").and_then(Value::as_array).and_then(|ps| ps.iter().find(|p| p.get("id").and_then(Value::as_str) == Some(pg)).cloned());
            match id {
                None => p.and_then(|p| p.get("title").and_then(Value::as_str).map(str::to_string)).unwrap_or_else(|| pg.to_string()),
                Some(id) => p
                    .and_then(|p| p.get("blocks").and_then(Value::as_array).and_then(|l| l.iter().find(|x| x.get("id").and_then(Value::as_str) == Some(id)).and_then(|x| x.get("title").and_then(Value::as_str)).map(str::to_string)))
                    .unwrap_or_else(|| id.to_string())
                    .to_lowercase(),
            }
        };
        for (pg, bl) in b {
            let m = bl.as_object().cloned().unwrap_or_default();
            if m.is_empty() {
                v.push(json!({"kind": "layout", "text": format!("Layout: {} as it was", title(pg, None))}));
                continue;
            }
            let hidden: Vec<String> = m.iter().filter(|(_, x)| x.get("hidden") == Some(&json!(true))).map(|(id, _)| title(pg, Some(id))).collect();
            let moved: Vec<String> = m.iter().filter(|(_, x)| ["x", "y", "w", "h"].iter().any(|k| x.get(*k).is_some()) && x.get("hidden") != Some(&json!(true))).map(|(id, _)| title(pg, Some(id))).collect();
            let looks: Vec<String> = m.iter().filter(|(_, x)| ["corners", "seeThrough", "font", "size", "accent"].iter().any(|k| x.get(*k).is_some())).map(|(id, _)| title(pg, Some(id))).collect();
            let t = title(pg, None);
            if !moved.is_empty() {
                v.push(json!({"kind": "layout", "text": format!("Layout: {t}: {} moved", moved.join(", "))}));
            }
            if !hidden.is_empty() {
                v.push(json!({"kind": "layout", "text": format!("Layout: {t}: hide {}", hidden.join(", "))}));
            }
            if !looks.is_empty() {
                v.push(json!({"kind": "layout", "text": format!("Look: {t}: {}", looks.join(", "))}));
            }
        }
    }
    for f in features {
        v.push(json!({"kind": "feature", "text": format!("New feature: {}", f.name.to_lowercase())}));
    }
    v
}

/// An answer from the AI, checked (as describe.rs check_answer does for Mantel).
pub fn check_answer(a: &AppSchema, v: &Value, asked: &str, sketch: bool) -> Result<Value, Vec<String>> {
    let changes: Vec<String> = v.get("changes").and_then(Value::as_array).map(|x| x.iter().filter_map(|s| s.as_str().map(|s| s.chars().take(200).collect())).take(12).collect()).unwrap_or_default();
    let versions = (crate::describe::hearth_version(), std::env::var("HEARTH_DESCRIBE_APP").unwrap_or_else(|_| a.name.to_string()).chars().take(40).collect::<String>());
    match v.get("kind").and_then(Value::as_str) {
        Some("app") | Some("layout") | Some("feature") => {
            let layout = check_layout(a, &v.get("layout").and_then(Value::as_object).cloned().unwrap_or_default())?;
            let mut fs: Vec<feature::Feature> = Vec::new();
            let mut raw: Vec<Value> = v.get("features").and_then(Value::as_array).cloned().unwrap_or_default();
            if let Some(one) = v.get("feature") {
                raw.push(one.clone());
            }
            let mut needs: Vec<String> = Vec::new();
            for f in raw.into_iter().take(3) {
                let f: feature::Feature = serde_json::from_value(f).map_err(|e| vec![format!("A feature couldn't be read ({e}).")])?;
                let c = check_feature(a, &f);
                if !c.problems.is_empty() {
                    return Err(c.problems);
                }
                needs.extend(c.needs.iter().cloned());
                fs.push(f);
            }
            if !needs.is_empty() {
                let closest = fs.first().cloned();
                let i = idea::Idea::new(a.app, asked, needs.clone(), closest.clone(), sketch, &versions.0, &versions.1);
                return Ok(json!({"ok": true, "kind": "needs", "app": a.app, "needs": needs, "closest": closest, "idea": i, "preview": i.preview(), "changes": changes}));
            }
            if layout.is_empty() && fs.is_empty() {
                return Err(vec!["Nothing would change.".into()]);
            }
            let chips = chips(a, &layout, &fs);
            Ok(json!({"ok": true, "kind": "app", "app": a.app, "layout": layout, "features": fs, "chips": chips, "changes": changes}))
        }
        Some("needs") => {
            let needs: Vec<String> = v.get("needs").and_then(Value::as_array).map(|x| x.iter().filter_map(|s| s.as_str().map(|s| s.chars().take(120).collect())).take(8).collect()).unwrap_or_default();
            let closest: Option<feature::Feature> = v.get("closest").cloned().and_then(|c| serde_json::from_value(c).ok()).filter(|f| check_feature(a, f).usable());
            let why: String = v.get("why").and_then(Value::as_str).unwrap_or("").chars().take(400).collect();
            let i = idea::Idea::new(a.app, asked, needs.clone(), closest.clone(), sketch, &versions.0, &versions.1);
            Ok(json!({"ok": true, "kind": "needs", "app": a.app, "needs": needs, "why": why, "closest": closest, "idea": i, "preview": i.preview(), "changes": changes}))
        }
        _ => Err(vec!["The answer wasn't a change to the app or what's needed.".into()]),
    }
}

/// The whole of Describe it for an app: ask, check, ask once more with the reasons.
pub fn describe(a: &AppSchema, text: &str, sketch: Option<&std::path::Path>) -> Value {
    let text: String = text.trim().chars().take(2000).collect();
    if text.is_empty() && sketch.is_none() {
        return json!({"ok": false, "error": "Type what you'd like, or add a sketch."});
    }
    let Some(e) = crate::describe::engine() else {
        return json!({"ok": false, "error": crate::describe::NO_AI});
    };
    let system = rules(a);
    let mut user = format!("Now: {}\nThe owner asks: {}", get(a.app), text);
    for attempt in 0..2 {
        let reply = match crate::describe::chat(&e, &system, &user, sketch) {
            Ok(r) => r,
            Err(m) => return json!({"ok": false, "error": m, "engine": e.label}),
        };
        let Some(v) = crate::describe::first_object(&reply) else {
            user = format!("{user}\nYour last answer wasn't one JSON object. Answer again with only the JSON object.");
            continue;
        };
        match check_answer(a, &v, &text, sketch.is_some()) {
            Ok(mut ok) => {
                ok["engine"] = json!(e.label);
                return ok;
            }
            Err(why) if attempt == 0 => user = format!("{user}\nYour last answer can't be used: {}. Answer again, fixed.", why.join(" ")),
            Err(why) => return json!({"ok": false, "error": format!("Your AI's answer couldn't be used: {}", why.join(" ")), "engine": e.label}),
        }
    }
    json!({"ok": false, "error": "Your AI didn't give an answer Hearth can use. Try again, or say it another way.", "engine": e.label})
}

// ---------------- what you kept ----------------

pub fn file(app: &str) -> std::path::PathBuf {
    let cfg = std::env::var_os("XDG_CONFIG_HOME").map(std::path::PathBuf::from).unwrap_or_else(|| crate::paths::home().join(".config"));
    cfg.join("nimbus/apps").join(format!("{app}.json"))
}

/// What this app has now: {"layout": {...}, "features": [...]} (empty when nothing was kept, or the file can't be read).
pub fn get(app: &str) -> Value {
    let j = crate::settings_file::read_json(&file(app));
    json!({"layout": j.get("layout").cloned().unwrap_or(json!({})), "features": j.get("features").cloned().unwrap_or(json!([]))})
}

/// Keep this: {"layout", "features"} checked again, saved with the one before as Undo. The layout is merged onto what
/// was kept (a knob it doesn't name stays as it was); features with the same id are replaced.
pub fn keep(a: &AppSchema, change: &Value) -> Result<Value, String> {
    let layout = check_layout(a, &change.get("layout").and_then(Value::as_object).cloned().unwrap_or_default()).map_err(|e| e.join(" "))?;
    let mut fs: Vec<feature::Feature> = Vec::new();
    for f in change.get("features").and_then(Value::as_array).cloned().unwrap_or_default() {
        let f: feature::Feature = serde_json::from_value(f).map_err(|e| format!("A feature couldn't be read ({e})."))?;
        let c = check_feature(a, &f);
        if !c.usable() {
            return Err(c.problems.into_iter().chain(c.needs.into_iter().map(|n| format!("needs {n}"))).collect::<Vec<_>>().join(" "));
        }
        fs.push(f);
    }
    let p = file(a.app);
    if let Some(d) = p.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    let (m, _) = crate::settings_file::update_json(&p, |m| {
        let before = json!({"layout": m.get("layout").cloned().unwrap_or(json!({})), "features": m.get("features").cloned().unwrap_or(json!([]))});
        let mut l = m.get("layout").and_then(Value::as_object).cloned().unwrap_or_default();
        for (k, v) in layout {
            if k == "blocks" {
                // the pages named: their blocks named replace those blocks; an empty page is the app's own again
                let mut b = l.get("blocks").and_then(Value::as_object).cloned().unwrap_or_default();
                for (pg, bl) in v.as_object().cloned().unwrap_or_default() {
                    let named = bl.as_object().cloned().unwrap_or_default();
                    if named.is_empty() {
                        b.remove(&pg);
                        continue;
                    }
                    let mut page = b.get(&pg).and_then(Value::as_object).cloned().unwrap_or_default();
                    for (id, x) in named {
                        page.insert(id, x);
                    }
                    b.insert(pg, Value::Object(page));
                }
                if b.is_empty() {
                    l.remove("blocks");
                } else {
                    l.insert(k, Value::Object(b));
                }
                continue;
            }
            if k == "zones" {
                // the zones named replace those zones; the others stay
                let mut z = l.get("zones").and_then(Value::as_object).cloned().unwrap_or_default();
                for (zn, items) in v.as_object().cloned().unwrap_or_default() {
                    z.insert(zn, items);
                }
                l.insert(k, Value::Object(z));
                continue;
            }
            if v.as_str() == Some("") {
                l.remove(&k);
            } else {
                l.insert(k, v);
            }
        }
        let mut have: Vec<Value> = m.get("features").and_then(Value::as_array).cloned().unwrap_or_default();
        for f in &fs {
            have.retain(|x| x.get("id").and_then(Value::as_str) != Some(f.id.as_str()));
            have.push(serde_json::to_value(f).unwrap_or(Value::Null));
        }
        m.insert("format".into(), json!(1));
        m.insert("layout".into(), Value::Object(l));
        m.insert("features".into(), Value::Array(have));
        m.insert("undo".into(), before);
    })
    .map_err(|e| e.to_string())?;
    Ok(Value::Object(m))
}

/// Undo: what was kept before Keep this, back.
pub fn undo(app: &str) -> Result<Value, String> {
    let p = file(app);
    if !p.exists() {
        return Ok(get(app));
    }
    let (m, _) = crate::settings_file::update_json(&p, |m| {
        let now = json!({"layout": m.get("layout").cloned().unwrap_or(json!({})), "features": m.get("features").cloned().unwrap_or(json!([]))});
        if let Some(u) = m.get("undo").cloned() {
            m.insert("layout".into(), u.get("layout").cloned().unwrap_or(json!({})));
            m.insert("features".into(), u.get("features").cloned().unwrap_or(json!([])));
        }
        m.insert("undo".into(), now);
    })
    .map_err(|e| e.to_string())?;
    Ok(Value::Object(m))
}

/// Back to the original (View › Layout): nothing changed, with what you had as Undo.
pub fn reset(app: &str) -> Result<Value, String> {
    let p = file(app);
    if !p.exists() {
        return Ok(get(app));
    }
    let (m, _) = crate::settings_file::update_json(&p, |m| {
        let now = json!({"layout": m.get("layout").cloned().unwrap_or(json!({})), "features": m.get("features").cloned().unwrap_or(json!([]))});
        m.insert("layout".into(), json!({}));
        m.insert("features".into(), json!([]));
        m.insert("undo".into(), now);
    })
    .map_err(|e| e.to_string())?;
    Ok(Value::Object(m))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn board11() -> Value {
        json!({"kind": "app", "layout": {"sidebar": "icons", "preview": "right-big"},
               "features": [{"format": 1, "id": "downloads-counts", "name": "Counts in Downloads", "part": "files:downloads",
                             "shows": [{"source": "folder-pictures", "label": "Pictures here"}, {"source": "folder-old", "label": "Older than a month"}],
                             "click": {"action": "select-pictures"}}],
               "changes": ["The sidebar shows icons only", "The preview is on the right, bigger"]})
    }

    #[test]
    fn board_11s_answer_is_read() {
        let r = check_answer(&FILES, &board11(), "sidebar icons only…", false).unwrap();
        assert_eq!(r["kind"], "app");
        assert_eq!(r["layout"]["sidebar"], "icons");
        let chips: Vec<&str> = r["chips"].as_array().unwrap().iter().map(|c| c["text"].as_str().unwrap()).collect();
        assert_eq!(chips, ["Layout: sidebar icons only", "Layout: preview right, bigger", "New feature: counts in downloads"]);
    }

    #[test]
    fn what_files_cant_do_is_refused_or_an_idea() {
        let mut v = board11();
        v["layout"]["sidebar"] = json!("floating");
        assert!(check_answer(&FILES, &v, "x", false).is_err(), "an unknown value");
        let mut v = board11();
        v["layout"]["colour"] = json!("red");
        assert!(check_answer(&FILES, &v, "x", false).is_err(), "an unknown knob");
        let mut v = board11();
        v["features"][0]["shows"][0]["source"] = json!("folder-faces");
        let r = check_answer(&FILES, &v, "count the faces", true).unwrap();
        assert_eq!(r["kind"], "needs");
        assert!(r["preview"].as_str().unwrap().contains("Sketch: yes"));
        let mut v = board11();
        v["features"][0]["part"] = json!("bar");
        assert_eq!(check_answer(&FILES, &v, "x", false).unwrap()["kind"], "needs", "Mantel's bar isn't a place in Files");
    }

    #[test]
    fn keep_undo_and_back_to_the_original() {
        let d = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_CONFIG_HOME", d.path());
        let r = check_answer(&FILES, &board11(), "x", false).unwrap();
        keep(&FILES, &r).unwrap();
        assert_eq!(get("files")["layout"]["preview"], "right-big");
        assert_eq!(get("files")["features"].as_array().unwrap().len(), 1);
        keep(&FILES, &json!({"layout": {"sidebar": ""}})).unwrap();
        assert!(get("files")["layout"].get("sidebar").is_none(), "\"\" is the app's own way");
        assert_eq!(get("files")["layout"]["preview"], "right-big", "a knob not named stays");
        undo("files").unwrap();
        assert_eq!(get("files")["layout"]["sidebar"], "icons");
        reset("files").unwrap();
        assert_eq!(get("files")["layout"], json!({}));
        assert!(std::path::Path::new(&format!("{}.bak", file("files").display())).exists() || file("files").exists());
        assert!(keep(&FILES, &json!({"layout": {"sidebar": "floating"}})).is_err());
    }

    #[test]
    fn the_whole_app_can_be_rearranged() {
        let v = json!({"kind": "app", "layout": {"tabs": "hidden", "zones": {"top": ["address", "spacer", "search"], "bottom": ["back", "forward", "up"], "hidden": ["share", "rename"]}}, "changes": ["x"]});
        let r = check_answer(&FILES, &v, "x", false).unwrap();
        let chips: Vec<&str> = r["chips"].as_array().unwrap().iter().map(|c| c["text"].as_str().unwrap()).collect();
        assert!(chips.contains(&"Layout: no tabs") && chips.iter().any(|c| c.starts_with("Layout: a bar at the bottom: back, forward, up")), "{chips:?}");
        let bad = |z: Value| check_answer(&FILES, &json!({"kind": "app", "layout": {"zones": z}}), "x", false).is_err();
        assert!(!bad(json!({"hidden": ["view", "close", "minimize"]})), "anything can go: the ways back are elsewhere too");
        assert!(!bad(json!({"titlebar": ["close", "minimize", "maximize", "spacer", "title"]})), "macOS-style: the buttons on the left");
        assert!(bad(json!({"top": ["search"], "bottom": ["search"]})), "one place only");
        assert!(bad(json!({"top": ["rocket"]})));
        assert!(bad(json!({"sky": ["back"]})));
        assert!(!bad(json!({"top": ["sep", "sep", "spacer", "spacer"]})));
    }

    #[test]
    fn every_app_has_its_window_bar() {
        for a in APPS {
            assert!(a.zones.iter().any(|(z, _, _)| *z == "titlebar"), "{}", a.app);
            let v = json!({"kind": "app", "layout": {"winbuttons": "dots", "zones": {"titlebar": ["close", "minimize", "maximize", "spacer", "title"]}}});
            assert!(check_answer(a, &v, "like a Mac", false).is_ok(), "{}", a.app);
        }
    }

    #[test]
    fn blocks_are_checked_and_kept_page_by_page() {
        let cat = json!({"cols": 12, "rows": 12, "pages": [{"id": "main", "title": "Main", "blocks": [
            {"id": "rail", "title": "Pages", "x": 0, "y": 0, "w": 2, "h": 12}, {"id": "page", "title": "The page", "x": 2, "y": 0, "w": 10, "h": 12}]}]});
        let ok = |v: Value| check_blocks(&TASK_MANAGER, &v, &cat);
        assert!(ok(json!({"main": {"page": {"x": 0, "w": 12, "corners": 16, "accent": "#FF7A2F"}, "rail": {"hidden": true}}})).is_ok());
        assert_eq!(ok(json!({"main": {"page": {"accent": "#FF7A2F"}}})).unwrap()["main"]["page"]["accent"], "#ff7a2f");
        assert!(ok(json!({"main": {}})).is_ok(), "a page back as it was");
        assert!(ok(json!({"main": {"page": {"x": 4}}})).is_err(), "its own width 10 from 4 goes off the page");
        assert!(ok(json!({"main": {"page": {"w": 0}}})).is_err());
        assert!(ok(json!({"main": {"page": {"font": "Wingdings"}}})).is_err());
        assert!(ok(json!({"main": {"page": {"run": "rm"}}})).is_err());
        assert!(ok(json!({"main": {"sky": {"x": 0}}})).is_err(), "a block the page hasn't got");
        assert!(ok(json!({"other": {"page": {"x": 0}}})).is_err(), "a page the app hasn't got");
        assert!(check_blocks(&TASK_MANAGER, &json!({"main": {"page": {"x": 0, "w": 12}}}), &Value::Null).is_ok(), "no catalog: only the shape");

        let d = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_CONFIG_HOME", d.path());
        keep(&TASK_MANAGER, &json!({"layout": {"blocks": {"main": {"rail": {"hidden": true}}}}})).unwrap();
        keep(&TASK_MANAGER, &json!({"layout": {"blocks": {"main": {"page": {"x": 0, "w": 12}}}}})).unwrap();
        let g = get("task-manager");
        assert_eq!(g["layout"]["blocks"]["main"]["rail"]["hidden"], true, "a block not named stays as kept");
        assert_eq!(g["layout"]["blocks"]["main"]["page"]["w"], 12);
        undo("task-manager").unwrap();
        assert!(get("task-manager")["layout"]["blocks"]["main"].get("page").is_none());
        keep(&TASK_MANAGER, &json!({"layout": {"blocks": {"main": {}}}})).unwrap();
        assert!(get("task-manager")["layout"].get("blocks").is_none(), "an empty page: the app's own again");
        let chips = chips(&TASK_MANAGER, &check_layout(&TASK_MANAGER, &json!({"blocks": {"main": {"rail": {"hidden": true}, "page": {"x": 0, "w": 12}}}}).as_object().cloned().unwrap()).unwrap(), &[]);
        assert_eq!(chips.len(), 2, "{chips:?}");
    }

    #[test]
    fn menus_can_be_rearranged() {
        let ok = check_answer(&FILES, &json!({"kind": "app", "layout": {"zones": {"menu:context": ["props", "sep", "copypath"], "hidden": ["share"]}}}), "x", false);
        assert!(ok.is_ok(), "{ok:?}");
        assert!(check_answer(&FILES, &json!({"kind": "app", "layout": {"zones": {"menu:start": ["x"]}}}), "x", false).is_err());
        assert!(check_answer(&FILES, &json!({"kind": "app", "layout": {"zones": {"menu:view": ["rm -rf"]}}}), "x", false).is_err());
    }

    #[test]
    fn the_rules_name_every_part_of_the_app() {
        let r = rules(&FILES);
        for k in FILES.knobs {
            assert!(r.contains(k.key));
        }
        for (s, _) in FILES.sources {
            assert!(r.contains(s));
        }
        assert!(r.contains("files:downloads"));
    }
}
