//! Hearth Files' menus and commands (FileManager.qml's command, openMenu, sortMenu, viewMenu, runMenu…), the keys,
//! and what each answer from hearth-files leads to.

use crate::app::*;
use crate::backend::{self, a, Reply};
use crate::drive;
use crate::icons::Ic;
use crate::model::{self, base, parent_of, Entry, APPS_PAGE, HOME_PAGE, THIS_PC};
use iced::keyboard::{key::Named, Event as KEvent, Key};
use iced::widget::operation;
use iced::{Point, Task};
use serde_json::{json, Map, Value};

fn sv(v: &Value, k: &str) -> String {
    v.get(k).and_then(Value::as_str).unwrap_or("").to_string()
}

impl App {
    pub fn show_menu(&mut self, title: &str, items: Vec<MenuItem>, short: bool) {
        // No separator first, last or twice in a row.
        let mut out: Vec<MenuItem> = Vec::new();
        for it in items {
            if it.sep && out.last().map(|l| l.sep).unwrap_or(true) {
                continue;
            }
            out.push(it);
        }
        while out.last().map(|l| l.sep).unwrap_or(false) {
            out.pop();
        }
        let sel = out.iter().position(|i| !i.sep && !i.heading && !i.quick).unwrap_or(0);
        let width = if short { 250.0 } else { 300.0 };
        self.overlay = Some(Overlay::Menu(Menu { title: title.into(), items: out, at: self.cursor, sel, short, width }));
    }

    fn copy_items(&self, n: usize, linux_too: bool, sub: &str) -> Vec<MenuItem> {
        let many = n > 1;
        let cl = self.copy_linux();
        let shortsub = if sub.chars().count() <= 44 && !sub.is_empty() { format!("  ({sub})") } else { String::new() };
        let win = MenuItem::new("copywin", &format!("{}{}", if cl { if many { "Copy Windows paths" } else { "Copy Windows path" } } else if many { "Copy paths" } else { "Copy path" }, if !cl { shortsub.as_str() } else { "" }))
            .key(if cl { "" } else { "Ctrl+Shift+C" })
            .icon(Ic::Link);
        let lin = MenuItem::new("copypath", &format!("{}{}", if cl { if many { "Copy paths" } else { "Copy path" } } else if many { "Copy Linux paths" } else { "Copy Linux path" }, if cl { shortsub.as_str() } else { "" }))
            .key("Ctrl+Alt+C")
            .icon(Ic::Link);
        if !linux_too {
            return vec![MenuItem::new("copywin", &format!("Copy path{shortsub}")).icon(Ic::Link)];
        }
        if cl { vec![lin, win] } else { vec![win, lin] }
    }

    /// The right-click menu (X on the controller, the Menu key): Windows 11's short one first (mockup files-7), the
    /// whole one after Show more options.
    pub fn open_menu(&mut self, for_folder: bool) {
        let cur = if for_folder { None } else { self.current().cloned() };
        if self.on_cview() && !self.picking() {
            return self.open_c_menu(cur);
        }
        if let Some(e) = &cur {
            if e.cv.is_some() && self.marked.is_empty() && !self.picking() {
                return self.open_c_menu(cur);
            }
        }
        if self.on_this_pc() && !self.picking() {
            return self.open_pc_menu(cur);
        }
        if self.on_apps() && !self.picking() {
            return self.open_app_menu(cur);
        }
        self.menu_on_folder = for_folder || cur.is_none();
        let e = cur;
        let t = if for_folder { vec![] } else { self.targets() };
        let n = t.len();
        let what = if n == 1 { String::new() } else { format!(" ({n})") };
        let w = self.writable();
        let mut items: Vec<MenuItem> = Vec::new();
        if let Some(j) = &self.job {
            items.push(MenuItem::new("stop", &format!("Stop {}", verbing(&j.req.verb).to_lowercase())).danger());
            items.push(MenuItem::sep());
        }
        if let Some(pk) = self.pick.clone() {
            items.push(MenuItem::new("pick-accept", &self.pick_accept_label()));
            if pk.multiple {
                if let Some(e) = &e {
                    items.push(MenuItem::new("mark", if self.marked.contains(&e.path) { "Unselect" } else { "Select" }).key("Ctrl+Space"));
                }
            }
            if pk.filters.len() > 1 {
                let f = pk.filters.get(self.pick_filter.max(0) as usize).map(|f| f.0.clone()).unwrap_or_default();
                items.push(MenuItem::new("pick-filter", &format!("Show: {f}  ›")));
            }
            items.push(MenuItem::new("pick-cancel", "Cancel").key("Esc"));
            items.push(MenuItem::sep());
        }
        if !self.picking() && !self.in_trash() && !self.menu_full && !crate::briefcase::in_briefcase(self) {
            if let Some(e) = &e {
                let mut q: Vec<MenuItem> = Vec::new();
                if w {
                    q.push(MenuItem::new("cut", &format!("Cut{what}")).key("Ctrl+X").icon(Ic::Cut).quick());
                }
                q.push(MenuItem::new("copy", &format!("Copy{what}")).key("Ctrl+C").icon(Ic::Copy).quick());
                if n == 1 && w {
                    q.push(MenuItem::new("rename", "Rename…").key("F2").icon(Ic::Rename).quick());
                }
                q.push(MenuItem::new("copywin", "Copy as path").icon(Ic::Share).quick());
                if w {
                    q.push(MenuItem::new("trash", &format!("Move to Trash{what}")).key("Del").icon(Ic::Delete).quick());
                }
                q.push(MenuItem::new("open", "Open").key("Enter").icon(Ic::FolderOpen));
                if e.dir && n == 1 {
                    q.push(MenuItem::new("newtab", "Open in new tab").icon(Ic::NewTab));
                }
                if !e.dir {
                    q.push(MenuItem::new("openwith", "Open with  ▸").icon(Ic::OpenWith));
                }
                if e.dir && !self.places.iter().any(|p| p.raw.get("bookmark").and_then(Value::as_bool) == Some(true) && p.path == e.path) {
                    q.push(MenuItem::new("bookmark", "Pin to the left menu").key("Ctrl+D").icon(Ic::Star));
                }
                if !e.dir && model::kind_of(e) == "image" {
                    q.push(MenuItem::new("wallpaper", "Set as wallpaper").icon(Ic::Image));
                }
                if w && !(n == 1 && model::is_archive(e)) {
                    q.push(MenuItem::new("compress", "Compress to ZIP").icon(Ic::Zip));
                }
                if n == 1 && model::is_archive(e) && w {
                    q.push(MenuItem::new("extracthere", "Extract here").icon(Ic::Zip));
                }
                if e.dir && n == 1 && !e.briefcase.is_object() {
                    q.push(MenuItem::new("lockbc", "Lock as a Briefcase").icon(Ic::Lock));
                }
                if e.dir && n == 1 {
                    q.push(MenuItem::new("terminal", "Open in Terminal").key("Shift+F4").icon(Ic::Keyboard));
                }
                if n == 2 && t.iter().all(|p| p.starts_with('/')) {
                    q.push(MenuItem::new("compare", "Compare").icon(Ic::Copy));
                }
                if n == 1 && e.path.starts_with('/') {
                    q.push(MenuItem::new("prevver", "Restore previous versions").icon(Ic::Clock));
                }
                q.push(MenuItem::new("copywin", "Copy as path").key("C:\\…").icon(Ic::Link));
                q.extend(crate::extras::menu_items(self, Some(e), n));
                q.push(MenuItem::sep());
                q.push(MenuItem::new("props", "Properties").key("Alt+Enter").icon(Ic::Info));
                q.push(MenuItem::new("more", "Show more options").icon(Ic::Dots));
                let title = if n > 1 { format!("{} selected", model::plural(n, "item", "items")) } else { e.name.clone() };
                return self.show_menu(&title, q, true);
            }
        }
        self.menu_full = false;
        if self.in_trash() {
            if n > 0 {
                items.push(MenuItem::new("restore", &format!("Put back{what}")).icon(Ic::Undo));
                items.push(MenuItem::new("delete", &format!("Delete permanently{what}")).danger().key("Del").icon(Ic::Delete));
            }
            items.push(MenuItem::new("empty", "Empty Trash").danger().icon(Ic::Delete));
            if e.is_some() {
                items.push(MenuItem::sep());
                items.push(MenuItem::new("props", "Properties").key("Alt+Enter").icon(Ic::Info));
            }
        } else {
            if let Some(e) = &e {
                items.push(MenuItem::new("open", "Open").key("Enter").icon(Ic::FolderOpen));
                if !e.dir {
                    items.push(MenuItem::new("openwith", "Open with…").icon(Ic::OpenWith));
                }
                if e.dir && n == 1 {
                    items.push(MenuItem::new("newtab", "Open in new tab").icon(Ic::NewTab));
                }
                if n == 1 && e.name.to_lowercase().ends_with(".appimage") {
                    items.push(MenuItem::new("airun", "Run"));
                    items.push(if e.in_menu { MenuItem::new("aimenurm", "Remove from app menu") } else { MenuItem::new("aimenu", "Add to app menu") });
                }
                if self.found.is_some() || self.on_home() {
                    items.push(MenuItem::new("showin", "Show in its folder").icon(Ic::Folder));
                }
                if n == 1 && !e.dir {
                    items.push(MenuItem::new("preview", "Preview").key("Space").icon(Ic::Eye));
                }
            }
            items.extend(crate::briefcase::menu_items(self, e.as_ref(), n, &t));
            items.extend(crate::extras::menu_items(self, e.as_ref(), n));
            items.push(MenuItem::sep());
            if n > 0 {
                if w {
                    items.push(MenuItem::new("cut", &format!("Cut{what}")).key("Ctrl+X").icon(Ic::Cut));
                }
                items.push(MenuItem::new("copy", &format!("Copy{what}")).key("Ctrl+C").icon(Ic::Copy));
            }
            if w {
                let pl = if self.clip.len() > 1 { format!(" ({})", self.clip.len()) } else { String::new() };
                items.push(MenuItem::new("paste", &format!("{}{pl}", if self.clip_mode == "cut" { "Move here" } else { "Paste" })).key("Ctrl+V").icon(Ic::Paste));
            }
            items.extend(self.copy_items(n, true, ""));
            items.push(MenuItem::sep());
            if n == 1 && w {
                items.push(MenuItem::new("rename", "Rename…").key("F2").icon(Ic::Rename));
            }
            if n > 1 && w {
                items.push(MenuItem::new("batchrename", &format!("Rename {n} items…")).key("F2").icon(Ic::Rename));
            }
            if let Some(e) = &e {
                if n > 0 && w && !(n == 1 && model::is_archive(e)) {
                    items.push(MenuItem::new("compress", "Compress…").icon(Ic::Zip));
                }
                if n == 1 && model::is_archive(e) && w {
                    items.push(MenuItem::new("extracthere", "Extract here").icon(Ic::Zip));
                }
                if n == 1 && model::is_archive(e) {
                    items.push(MenuItem::new("extractto", "Extract to…").icon(Ic::Zip));
                }
                if model::kind_of(e) == "image" {
                    items.push(MenuItem::new("wallpaper", "Set as wallpaper").icon(Ic::Image));
                }
            }
            if e.is_none() && w {
                items.push(MenuItem::new("newfolder", "New folder…").key("Ctrl+Shift+N").icon(Ic::Folder));
                items.push(MenuItem::new("newfile", "New empty file…").icon(Ic::File));
                if !self.templates.is_empty() {
                    items.push(MenuItem::new("sub-new", "New document  ›").icon(Ic::Document));
                }
            }
            items.push(MenuItem::sep());
            if n > 0 && w {
                items.push(MenuItem::new("trash", &format!("Move to Trash{what}")).key("Del").icon(Ic::Delete));
                items.push(MenuItem::new("delete", &format!("Delete permanently{what}")).danger().key("Shift+Del").icon(Ic::Close));
            }
            items.push(MenuItem::sep());
            if let Some(e) = &e {
                items.push(MenuItem::new("mark", if self.marked.contains(&e.path) { "Unselect" } else { "Select" }).key("Ctrl+Space"));
            }
            if !self.marked.is_empty() {
                items.push(MenuItem::new("unmarkall", "Clear selection").key("Esc"));
            }
            if !self.entries.is_empty() {
                items.push(MenuItem::new("markall", "Select all").key("Ctrl+A"));
            }
            if !self.marked.is_empty() {
                items.push(MenuItem::new("invert", "Invert selection").key("Ctrl+I"));
            }
            items.push(MenuItem::sep());
            if e.is_none() {
                items.push(MenuItem::new("sub-view", "View  ›").icon(Ic::Grid));
                items.push(MenuItem::new("sub-sort", "Sort and group  ›").icon(Ic::Sort));
                items.push(MenuItem::new("hidden", if self.hidden { "Hide hidden files" } else { "Show hidden files" }).key("Ctrl+H").icon(Ic::Eye));
                if !self.picking() {
                    items.push(MenuItem::new("newtab", "New tab").key("Ctrl+T").icon(Ic::Plus));
                }
                if self.tabs.len() > 1 {
                    items.push(MenuItem::new("closetab", "Close tab").key("Ctrl+W").icon(Ic::Close));
                }
                items.extend(crate::extras::folder_menu_items(self));
            }
            let bm = match &e {
                Some(e) if e.dir => e.path.clone(),
                Some(_) => String::new(),
                None => if self.on_home() { String::new() } else { self.path.clone() },
            };
            if !bm.is_empty() && bm.starts_with('/') && !self.places.iter().any(|p| p.raw.get("bookmark").and_then(Value::as_bool) == Some(true) && p.path == bm) && !self.picking() {
                items.push(MenuItem::new("bookmark", "Pin to the left menu").key("Ctrl+D").icon(Ic::Pin));
            }
            if (e.is_none() && !self.on_home() || e.as_ref().map(|x| x.dir).unwrap_or(false)) && !self.picking() {
                items.push(MenuItem::new("terminal", "Open in Terminal").key("Shift+F4").icon(Ic::Keyboard));
            }
            if e.is_none() && !self.picking() {
                items.push(MenuItem::new("termpanel", if self.term.on { "Hide the terminal panel" } else { "Terminal panel" }).key("F4").icon(Ic::Terminal));
                items.push(MenuItem::new("splitview", if self.split.is_some() { "Close split view" } else { "Split view" }).key("F9").icon(Ic::Split));
            }
            if e.is_some() || !self.on_home() {
                items.push(MenuItem::sep());
                items.push(MenuItem::new("props", if e.is_some() { "Properties" } else { "Folder properties" }).key("Alt+Enter").icon(Ic::Info));
            }
        }
        let title = match &e {
            Some(e) => if n > 1 { format!("{} selected", model::plural(n, "item", "items")) } else { e.name.clone() },
            None => self.listing.as_ref().map(|l| l.name.clone()).filter(|n| !n.is_empty()).unwrap_or_else(|| "Folder".into()),
        };
        self.show_menu(&title, items, false);
    }

    fn open_pc_menu(&mut self, e: Option<Entry>) {
        self.menu_on_folder = e.is_none();
        let Some(e) = e else {
            let items = vec![
                MenuItem::new("tp-view:grid", "Tiles").check(self.pc_view == "grid"),
                MenuItem::new("tp-view:details", "Details").check(self.pc_view == "details"),
                MenuItem::sep(),
                MenuItem::new("tp-apps", "Installed apps"),
                MenuItem::new("refresh", "Refresh").key("F5"),
            ];
            return self.show_menu("This PC", items, false);
        };
        let d = e.item.clone();
        let real = e.path.starts_with('/');
        let mut items = vec![MenuItem::new("open", if d.get("mounted") == Some(&Value::Bool(false)) && e.pc == "drive" { "Mount and open" } else { "Open" }).key("Enter").icon(Ic::FolderOpen)];
        if real {
            items.push(MenuItem::new("newtab", "Open in new tab").icon(Ic::NewTab));
        }
        if e.pc == "drive" && d.get("removable") == Some(&Value::Bool(true)) && d.get("mounted") != Some(&Value::Bool(false)) {
            items.push(MenuItem::sep());
            items.push(MenuItem::new("tp-unmount", "Unmount"));
            items.push(MenuItem::new("tp-eject", "Safely remove"));
        }
        if e.pc == "net" && d.get("net").map(|n| n.is_object()).unwrap_or(false) {
            items.push(MenuItem::sep());
            items.push(MenuItem::new("tp-auto", "Connect automatically").check(sv(&d, "sub") == "Connects by itself"));
        }
        if real {
            items.push(MenuItem::sep());
            items.push(MenuItem::new("copywin", "Copy path").icon(Ic::Link));
            items.push(MenuItem::new("copypath", "Copy Linux path").key("Ctrl+Alt+C").icon(Ic::Link));
            items.push(MenuItem::sep());
            items.push(MenuItem::new("props", "Properties").key("Alt+Enter").icon(Ic::Info));
        }
        self.show_menu(&e.name, items, false);
    }

    fn open_app_menu(&mut self, e: Option<Entry>) {
        self.menu_on_folder = e.is_none();
        let Some(e) = e else {
            return self.show_menu("Installed apps", vec![MenuItem::new("refresh", "Refresh").key("F5")], false);
        };
        let a = e.app.clone().unwrap_or(Value::Null);
        let mut items = vec![MenuItem::new("ia-open", "Open").key("Enter").icon(Ic::FolderOpen)];
        if !sv(&a, "path").is_empty() {
            items.push(MenuItem::new("ia-folder", "Open its folder").icon(Ic::Folder));
        }
        items.push(MenuItem::sep());
        items.push(MenuItem::new("ia-settings", "Uninstall or change it in Settings › Apps").icon(Ic::Apps));
        self.show_menu(&e.name, items, false);
    }

    fn open_c_menu(&mut self, e: Option<Entry>) {
        self.menu_on_folder = e.is_none();
        let Some(e) = e else {
            let mut its = vec![];
            if self.c_page() == "root" {
                its.push(MenuItem::new("c-linux", "Show the Linux folders too").check(self.c_linux()));
            }
            let w = self.win_path(&self.path.clone());
            its.extend(self.copy_items(1, false, &w));
            its.push(MenuItem::sep());
            its.push(MenuItem::new("refresh", "Refresh").key("F5"));
            its.push(MenuItem::sep());
            its.push(MenuItem::new("c-props", "Properties").key("Alt+Enter").icon(Ic::Info));
            let title = self.listing.as_ref().map(|l| l.name.clone()).unwrap_or_else(|| "Hearth (C:)".into());
            return self.show_menu(&title, its, false);
        };
        let cv = e.cv.clone().unwrap_or_default();
        let real = self.c_real_of(&e);
        let mut items = vec![MenuItem::new("open", "Open").key("Enter").icon(Ic::FolderOpen), MenuItem::new("c-newtab", "Open in new tab").icon(Ic::NewTab), MenuItem::sep()];
        if !real.is_empty() {
            let sub = if real.starts_with(&format!("{}/", self.home)) { format!("~{}", &real[self.home.len()..]) } else { real.clone() };
            items.push(MenuItem::new("c-real", "Go to the real folder").sub(&sub).icon(Ic::Folder));
        }
        let win = if cv.win.is_empty() { self.win_path(&e.path) } else { cv.win.clone() };
        for it in self.copy_items(1, e.path.starts_with('/'), "") {
            let sub = if it.id == "copywin" { win.clone() } else { e.path.clone() };
            items.push(it.sub(&sub));
        }
        if cv.kind == "hearth" {
            items.push(MenuItem::sep());
            items.push(MenuItem::new("c-part", "Part of Hearth").icon(Ic::Info));
        }
        items.push(MenuItem::sep());
        items.push(MenuItem::new("c-props", "Properties").key("Alt+Enter").icon(Ic::Info));
        self.show_menu(&e.name, items, false);
    }

    pub fn c_real_of(&self, e: &Entry) -> String {
        if e.path.starts_with('/') {
            return e.path.clone();
        }
        if let drive::Res::Path(p) = drive::c_resolve(&e.path, &self.ctx()) {
            if p.starts_with('/') {
                return p;
            }
        }
        let first = e.cv.as_ref().and_then(|c| c.real.first().cloned()).unwrap_or_default();
        let p = first.split(": ").last().unwrap_or("").split(',').next().unwrap_or("").trim().to_string();
        if let Some(r) = p.strip_prefix('~') {
            return format!("{}{}", self.home, r);
        }
        if p.starts_with('/') { p } else { String::new() }
    }

    /// View (mockup files-8): LAYOUT, then SHOW.
    pub fn view_menu(&mut self) {
        let mut items = vec![];
        if self.on_this_pc() {
            items.push(MenuItem::heading("LAYOUT"));
            items.push(MenuItem::new("tp-view:grid", "Tiles").key("Ctrl+1").check(self.pc_view == "grid").icon(Ic::Grid));
            items.push(MenuItem::new("tp-view:details", "Details").key("Ctrl+3").check(self.pc_view == "details").icon(Ic::ViewList));
        } else if !self.on_cview() && !self.on_apps() {
            items.push(MenuItem::heading("LAYOUT"));
            for (v, l, k, ic) in [
                (View::XLarge, "Extra large icons", "Ctrl+7", Ic::Grid),
                (View::Large, "Large icons", "Ctrl+4", Ic::Grid),
                (View::Grid, "Medium icons", "Ctrl+1", Ic::Grid),
                (View::Small, "Small icons", "Ctrl+5", Ic::Grid),
                (View::Tiles, "Tiles", "Ctrl+6", Ic::Grid),
                (View::Compact, "List", "Ctrl+2", Ic::ListRows),
                (View::Details, "Details", "Ctrl+3", Ic::ViewList),
            ] {
                items.push(MenuItem::new(&format!("view:{}", v.id()), l).key(k).check(self.view == v).icon(ic));
            }
            items.push(MenuItem::new("compactview", "Compact view").check(self.prefs.get("compact").and_then(Value::as_bool).unwrap_or(false)).icon(Ic::Compact));
        }
        items.push(MenuItem::sep());
        items.push(MenuItem::heading("SHOW"));
        items.push(MenuItem::new("navpane", "Navigation pane").check(self.nav_shown()));
        items.push(MenuItem::new("detailspane", "Details pane").key("Alt+P").check(self.details_pane));
        items.push(MenuItem::new("exts", "File name extensions").check(self.show_ext));
        items.push(MenuItem::new("hidden", "Hidden items").key("Ctrl+H").check(self.hidden));
        if !self.picking() {
            // Describe it (board 11): say how you'd like Files laid out; Back to the original undoes all of it
            items.push(MenuItem::sep());
            items.push(MenuItem::heading("LAYOUT"));
            items.push(MenuItem::new("describe", "Describe it…"));
            if self.describe.has_kept() {
                items.push(MenuItem::new("layout-original", "Back to the original"));
            }
        }
        if !self.picking() {
            items.push(MenuItem::new("cset:linuxFolders=toggle", "Linux folders (/, /home, /usr…)").check(self.c_linux()));
            items.push(MenuItem::new("cset:addressBar=toggle", "Linux paths in the address bar").check(self.addr_linux()));
            items.push(MenuItem::sep());
            let start_home = self.start_page() == HOME_PAGE;
            items.push(MenuItem::new("tp-start:home", "Open Files to: Home").check(start_home));
            items.push(MenuItem::new("tp-start:thispc", "Open Files to: This PC").check(!start_home));
            items.push(MenuItem::new("cset:cdrive=toggle", "Show Hearth (C:) with Windows folder names").check(self.cdrive_on()));
            if self.cdrive_on() {
                items.push(MenuItem::new("cset:copyPath=windows", "Copy path: C:\\ form").check(!self.copy_linux()));
                items.push(MenuItem::new("cset:copyPath=linux", "Copy path: Linux path").check(self.copy_linux()));
            }
        }
        self.show_menu("View", items, false);
        if let Some(Overlay::Menu(m)) = &mut self.overlay {
            m.width = 270.0;
            m.sel = m.items.iter().position(|i| i.check == Some(true)).unwrap_or(m.sel);
        }
    }

    pub fn sort_menu(&mut self) {
        let items = vec![
            MenuItem::new("sort:name", "Name").check(self.sort == "name"),
            MenuItem::new("sort:date", "Date modified").check(self.sort == "date"),
            MenuItem::new("sort:size", "Size").check(self.sort == "size"),
            MenuItem::new("sort:type", "Type").check(self.sort == "type"),
            MenuItem::sep(),
            MenuItem::new("asc", "Ascending").check(!self.desc),
            MenuItem::new("desc", "Descending").check(self.desc),
            MenuItem::sep(),
            MenuItem::heading("GROUP BY"),
            MenuItem::new("group:none", "(None)").check(self.group == "none"),
            MenuItem::new("group:type", "Type").check(self.group == "type"),
            MenuItem::new("group:date", "Date modified").check(self.group == "date"),
            MenuItem::new("group:size", "Size").check(self.group == "size"),
            MenuItem::new("group:tags", "Tag").check(self.group == "tags"),
        ];
        self.show_menu("Sort and group", items, false);
    }

    fn new_menu(&mut self) {
        let mut items = vec![
            MenuItem::new("newfolder", "Folder").key("Ctrl+Shift+N").icon(Ic::Folder),
            MenuItem::new("newfile", "Empty file").key("Ctrl+Alt+N").icon(Ic::File),
            MenuItem::new("bc-new", "Briefcase").icon(Ic::Briefcase),
        ];
        if !self.templates.is_empty() {
            items.push(MenuItem::sep());
            for (i, t) in self.templates.iter().enumerate() {
                items.push(MenuItem::new(&format!("tpl:{i}"), &sv(t, "name")).icon(Ic::Document));
            }
        }
        self.show_menu("New", items, false);
    }

    /// "···": what doesn't fit the command bar (Undo, Redo, Select all, the terminal and split view, Properties).
    fn more_menu(&mut self) {
        let mut items = vec![
            MenuItem::new("undo", "Undo").key("Ctrl+Z").icon(Ic::Undo),
            MenuItem::new("redo", "Redo").key("Ctrl+Y"),
            MenuItem::sep(),
            MenuItem::new("markall", "Select all").key("Ctrl+A"),
            MenuItem::new("unmarkall", "Select none"),
            MenuItem::new("invert", "Invert selection").key("Ctrl+I"),
            MenuItem::sep(),
        ];
        if !self.on_home() && !self.on_special() {
            items.push(MenuItem::new("terminal", "Open in Terminal").key("Shift+F4").icon(Ic::Keyboard));
        }
        items.push(MenuItem::new("termpanel", if self.term.on { "Hide the terminal panel" } else { "Terminal panel" }).key("F4").icon(Ic::Terminal));
        items.push(MenuItem::new("splitview", if self.split.is_some() { "Close split view" } else { "Split view" }).key("F9").icon(Ic::Split));
        items.push(MenuItem::new("newtab", "New tab").key("Ctrl+T").icon(Ic::Plus));
        if !self.on_home() {
            items.push(MenuItem::sep());
            items.push(MenuItem::new("props", "Properties").key("Alt+Enter").icon(Ic::Info));
        }
        self.menu_on_folder = true;
        self.show_menu("More", items, false);
    }
}

pub fn place_menu(a: &mut App) {
    let Some(p) = a.places.get(a.place_index).cloned() else { return };
    a.menu_on_folder = false;
    let mut items = vec![];
    if !p.path.is_empty() {
        items.push(MenuItem::new("p-open", "Open"));
        items.push(MenuItem::new("p-newtab", "Open in new tab"));
    }
    if p.path.is_empty() && !sv(&p.raw, "device").is_empty() {
        items.push(MenuItem::new("p-mount", "Mount"));
    }
    if sv(&p.raw, "section") == "drives" && !p.path.is_empty() && p.raw.get("removable") == Some(&Value::Bool(true)) {
        items.push(MenuItem::new("p-unmount", "Unmount"));
        items.push(MenuItem::new("p-eject", "Safely remove"));
    }
    if let Some(net) = p.raw.get("net").filter(|n| n.is_object()) {
        if !sv(net, "share").is_empty() {
            items.push(MenuItem::new("p-open", "Open"));
            items.push(MenuItem::new("p-auto", "Connect automatically").check(p.raw.get("auto") == Some(&Value::Bool(true)) || sv(&p.raw, "sub") == "Connects by itself"));
        }
    }
    if p.raw.get("bookmark") == Some(&Value::Bool(true)) {
        items.push(MenuItem::new("p-unbookmark", "Remove bookmark").danger());
    }
    if p.kind == "trash" {
        items.push(MenuItem::new("empty", "Empty Trash").danger());
    }
    if !p.path.is_empty() && p.path.starts_with('/') {
        items.push(MenuItem::sep());
        items.push(MenuItem::new("p-props", "Properties"));
    }
    if items.is_empty() {
        return;
    }
    a.show_menu(&p.label, items, false);
}

pub fn versions_menu(a: &mut App) {
    if a.versions.is_empty() {
        return;
    }
    let of = base(&a.versions_of);
    let items = a.versions.iter().take(30).enumerate().map(|(i, v)| MenuItem::new(&format!("ver:{i}"), &format!("{}  ›", version_name(v))).icon(Ic::Clock)).collect();
    a.show_menu(&format!("Previous versions of {of}"), items, false);
}

fn version_name(p: &str) -> String {
    // …/.hearth-snapshots/2026-11-10_2355/…
    if let Some(i) = p.find(".hearth-snapshots/") {
        let s = &p[i + 18..];
        if s.len() >= 15 && s.as_bytes()[10] == b'_' {
            let months = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
            let m: usize = s[5..7].parse().unwrap_or(1);
            return format!("{} {}, {}:{}", months[m.clamp(1, 12) - 1], s[8..10].trim_start_matches('0'), &s[11..13], &s[13..15]);
        }
    }
    p.to_string()
}

pub fn filter_menu(a: &mut App) {
    let Some(pk) = a.pick.clone() else { return };
    let items = pk.filters.iter().enumerate().map(|(i, f)| MenuItem::new(&format!("pf:{i}"), &f.0).check(i as i32 == a.pick_filter)).collect();
    a.show_menu("Show", items, false);
}

pub fn choice_run(a: &mut App, i: usize) -> Task<Msg> {
    let Some(c) = a.pick.as_ref().and_then(|p| p.choices.get(i).cloned()) else { return Task::none() };
    let now = a.pick_choice.get(&c.0).and_then(Value::as_str).unwrap_or("").to_string();
    if c.2.is_empty() {
        a.pick_choice.insert(c.0.clone(), Value::String(if now == "true" { "false".into() } else { "true".into() }));
        return Task::none();
    }
    let items = c.2.iter().enumerate().map(|(j, o)| MenuItem::new(&format!("pc:{i}:{j}"), &o.1).check(o.0 == now)).collect();
    a.show_menu(&c.1, items, false);
    Task::none()
}

fn verbing(v: &str) -> &'static str {
    match v {
        "copy" => "Copying",
        "move" => "Moving",
        "compress" => "Compressing",
        "extract" => "Unpacking",
        _ => "Working on",
    }
}

pub fn copy_text(a: &mut App, t: &str) -> Task<Msg> {
    backend::detached(&[String::from("wl-copy"), "--".into(), t.to_string()], None);
    a.say(&format!("Copied “{t}”"), false)
}

fn copy_targets(a: &App, on_folder: bool, t: &[String]) -> Vec<String> {
    if a.on_apps() {
        return vec![];
    }
    if a.on_this_pc() {
        return a.current().filter(|c| c.path.starts_with('/')).map(|c| vec![c.path.clone()]).unwrap_or_default();
    }
    if a.on_cview() {
        return if !on_folder { a.current().map(|c| vec![c.path.clone()]).unwrap_or_default() } else { vec![a.path.clone()] };
    }
    if !on_folder {
        if let Some(c) = a.current() {
            if c.cv.is_some() && a.marked.is_empty() {
                return vec![c.path.clone()];
            }
        }
    }
    if !t.is_empty() && !on_folder {
        return t.to_vec();
    }
    if a.listing.is_some() && !a.on_home() { vec![a.path.clone()] } else { vec![] }
}

/// The command bar's and the navigation row's buttons (also reached with the keyboard and the controller).
pub fn command(a: &mut App, c: &str) -> Task<Msg> {
    match c {
        "refresh" => return Task::batch(vec![a.reload(), a.refresh_places()]),
        "new" => {
            if !a.writable() {
                return a.say("You can't make anything here", false);
            }
            a.menu_on_folder = true;
            a.new_menu();
        }
        "cut" => {
            let t = a.targets();
            return a.set_clip("cut", t);
        }
        "copy" => {
            let t = a.targets();
            return a.set_clip("copy", t);
        }
        "paste" => return a.paste(),
        "rename" => {
            if a.current().map(|e| e.cv.is_some()).unwrap_or(false) && !a.on_special() {
                let n = a.current().map(|e| e.name.clone()).unwrap_or_default();
                return a.say(&format!("“{n}” is one of Hearth (C:)'s names: it can't be renamed"), false);
            }
            if a.current().is_none() || a.on_special() || a.on_home() {
                return a.say("Pick something to rename", false);
            }
            a.menu_on_folder = false;
            return run_menu(a, "rename");
        }
        "copypath" | "share" => {
            let on_folder = a.current().is_none() || a.on_this_pc();
            let t = a.targets();
            let cps = copy_targets(a, on_folder, &t);
            if cps.is_empty() {
                return a.say(if a.on_apps() { "Apps have no path to copy" } else { "Pick something first" }, false);
            }
            let cl = a.copy_linux();
            let text = cps.iter().map(|p| a.copy_form(p, cl)).collect::<Vec<_>>().join("\n");
            return copy_text(a, &text);
        }
        "delete" => {
            if a.targets().is_empty() {
                return a.say("Pick something first", false);
            }
            a.menu_on_folder = false;
            return run_menu(a, if a.in_trash() { "delete" } else { "trash" });
        }
        "sort" => {
            if a.on_this_pc() {
                return a.say("This PC keeps Windows' order: drives, then network locations", false);
            }
            if a.on_cview() {
                return a.say("Hearth (C:)'s folders keep Windows' order, by name", false);
            }
            a.sort_menu();
        }
        "view" => a.view_menu(),
        "details" => {
            a.details_pane = !a.details_pane;
            let t = a.remember_view_pub();
            return Task::batch(vec![t, a.details_lookups()]);
        }
        "more" => a.more_menu(),
        "undo" => return a.run_undo(true),
        "redo" => return a.run_undo(false),
        "home" => return a.navigate(HOME_PAGE, ""),
        "sortname" => return run_menu(a, "sort:name"),
        "sortdate" => return run_menu(a, "sort:date"),
        "sorttype" => return run_menu(a, "sort:type"),
        "sortsize" => return run_menu(a, "sort:size"),
        "clinux" => return run_menu(a, "c-linux"),
        c if c.starts_with("bc-") => return crate::briefcase::command(a, c),
        _ => {}
    }
    Task::none()
}

impl App {
    pub fn remember_view_pub(&mut self) -> Task<Msg> {
        self.remember_view()
    }
}

pub fn run_menu(a: &mut App, id: &str) -> Task<Msg> {
    a.overlay = None;
    let on_folder = a.menu_on_folder;
    a.menu_on_folder = false;
    let t = if on_folder { vec![] } else { a.targets() };
    let e = if on_folder { None } else { a.current().cloned() };
    if let Some(v) = id.strip_prefix("view:") {
        if let Some(v) = View::from(v) {
            a.view = v;
            let t1 = a.remember_view_pub();
            return Task::batch(vec![t1, a.want_thumbs()]);
        }
    }
    if let Some(k) = id.strip_prefix("sort:") {
        if a.sort == k {
            a.desc = !a.desc;
        } else {
            a.sort = k.into();
            a.desc = k == "date" || k == "size";
        }
        a.rebuild();
        return a.remember_view_pub();
    }
    if let Some(g) = id.strip_prefix("group:") {
        a.group = g.into();
        a.rebuild();
        return a.remember_view_pub();
    }
    if let Some(v) = id.strip_prefix("tp-view:") {
        a.pc_view = v.into();
        a.prefs.insert("thisPcView".into(), json!(v));
        return Task::perform(backend::call(vec!["prefs-set".into(), json!({ "thisPcView": v }).to_string()], 15), |r| Msg::Did(Then::Nothing, r));
    }
    if let Some(k) = id.strip_prefix("tp-start:") {
        return a.set_files_pref("startPage", json!(k));
    }
    if let Some(kv) = id.strip_prefix("cset:") {
        let (k, v) = kv.split_once('=').unwrap_or((kv, ""));
        let val = match (k, v) {
            ("cdrive", "toggle") => json!(!a.cdrive_on()),
            ("linuxFolders", "toggle") => json!(!a.c_linux()),
            ("addressBar", "toggle") => json!(if a.addr_linux() { "windows" } else { "linux" }),
            (_, "true") => json!(true),
            (_, "false") => json!(false),
            (_, s) => json!(s),
        };
        return a.set_files_pref(k, val);
    }
    if let Some(i) = id.strip_prefix("tpl:").and_then(|s| s.parse::<usize>().ok()) {
        let Some(tp) = a.templates.get(i).cloned() else { return Task::none() };
        if !a.writable() {
            return Task::none();
        }
        let file = sv(&tp, "file");
        let dot = file.rfind('.').filter(|d| *d > 0).unwrap_or(file.len());
        return a.ask(&format!("New {}", sv(&tp, "name")), &file, "Create", file[..dot].chars().count(), Ask::FromTemplate(sv(&tp, "path"), a.path.clone()));
    }
    if let Some(i) = id.strip_prefix("pf:").and_then(|s| s.parse::<i32>().ok()) {
        a.pick_filter = i;
        a.rebuild();
        return Task::none();
    }
    if let Some(ij) = id.strip_prefix("pc:") {
        let mut it = ij.split(':').filter_map(|x| x.parse::<usize>().ok());
        if let (Some(i), Some(j)) = (it.next(), it.next()) {
            if let Some(c) = a.pick.as_ref().and_then(|p| p.choices.get(i).cloned()) {
                if let Some(o) = c.2.get(j) {
                    a.pick_choice.insert(c.0.clone(), Value::String(o.0.clone()));
                }
            }
        }
        return Task::none();
    }
    if let Some(i) = id.strip_prefix("ver:").and_then(|s| s.parse::<usize>().ok()) {
        let Some(v) = a.versions.get(i).cloned() else { return Task::none() };
        let items = vec![
            MenuItem::new(&format!("veropen:{i}"), "Open this version").icon(Ic::FolderOpen),
            MenuItem::new(&format!("verrestore:{i}"), "Restore it (yours now goes to the Trash)").icon(Ic::Undo),
            MenuItem::new(&format!("vercopy:{i}"), "Copy it next to yours").icon(Ic::Copy),
        ];
        let title = format!("{} · {}", base(&a.versions_of), version_name(&v));
        a.show_menu(&title, items, false);
        return Task::none();
    }
    for (pre, kind) in [("veropen:", 0), ("verrestore:", 1), ("vercopy:", 2)] {
        if let Some(i) = id.strip_prefix(pre).and_then(|s| s.parse::<usize>().ok()) {
            let Some(v) = a.versions.get(i).cloned() else { return Task::none() };
            let orig = a.versions_of.clone();
            let vn = version_name(&v);
            match kind {
                0 => return a.navigate(&parent_of(&v), &base(&v)),
                2 => {
                    let b = base(&orig);
                    let (stem, ext) = match b.rfind('.') {
                        Some(d) if d > 0 => (b[..d].to_string(), b[d..].to_string()),
                        _ => (b.clone(), String::new()),
                    };
                    let to = format!("{}/{stem} ({}){ext}", parent_of(&orig), vn.replace(['/', ':'], "."));
                    backend::detached(&[String::from("cp"), "-a".into(), "--".into(), v.clone(), to], None);
                    return a.say(&format!("Copied the {vn} version next to it"), false);
                }
                _ => {
                    backend::detached(&[String::from("sh"), "-c".into(), "gio trash -- \"$2\" 2>/dev/null || mv -- \"$2\" \"$2.before-restore\"; cp -a -- \"$1\" \"$2\"".into(), "sh".into(), v.clone(), orig.clone()], None);
                    return a.say(&format!("Put back the {vn} version of “{}” (yours is in the Trash)", base(&orig)), false);
                }
            }
        }
    }
    if id.starts_with("x-") {
        let e2 = e.clone();
        return crate::extras::run(a, id, t, e2);
    }
    if id.starts_with("bc-") {
        return crate::briefcase::run(a, id, t);
    }
    if let Some(choice) = id.strip_prefix("fo-") {
        // A script or program: Run in Terminal, Run, or Open in Hearth Text (hearth-files remembers nothing here).
        if let Some(e) = &e {
            let args = match choice {
                "terminal" => a_(&["run", &e.path, "--terminal"]),
                "run" => a_(&["run", &e.path]),
                _ => a_(&["open-text", &e.path]),
            };
            return Task::perform(backend::call(args, 15), |r| Msg::Did(Then::Nothing, r));
        }
        return Task::none();
    }
    match id {
        "stop" => return update(a, Msg::StopJob),
        "pick-accept" => return update(a, Msg::PickAccept),
        "pick-cancel" => return update(a, Msg::PickCancel),
        "pick-filter" => filter_menu(a),
        "more" => {
            a.menu_full = true;
            a.open_menu(on_folder);
        }
        "open" => return a.activate(a.cur),
        "airun" => {
            if let Some(e) = &e {
                let p = e.path.clone();
                return a.run_appimage(&p, false);
            }
        }
        "aimenu" | "aimenurm" => {
            if let Some(e) = &e {
                let cmd = if id == "aimenu" { "appimage-menu-add" } else { "appimage-menu-remove" };
                let msg = if id == "aimenu" { format!("{} is in your app menu", e.name) } else { format!("“{}” is out of the app menu (the AppImage is still here)", e.name) };
                return Task::perform(backend::call(a_(&[cmd, &e.path]), 60), move |r| Msg::Did(Then::Reload(msg.clone()), r));
            }
        }
        "openwith" => {
            if let Some(e) = &e {
                let e = e.clone();
                return show_apps(a, &e, false);
            }
        }
        "wallpaper" => return update(a, Msg::Wallpaper),
        "newtab" => {
            let p = match &e {
                Some(e) if e.dir => e.path.clone(),
                _ => a.path.clone(),
            };
            return a.new_tab(&p);
        }
        "closetab" => {
            let i = a.tab_index;
            return a.close_tab(i);
        }
        "showin" => {
            if let Some(e) = &e {
                let dir = parent_of(&e.path);
                a.found = None;
                a.search = None;
                let n = e.name.clone();
                return a.navigate(&dir, &n);
            }
        }
        "preview" => {
            if let Some(e) = &e {
                let e = e.clone();
                a.overlay = Some(Overlay::Preview(Pv { e: e.clone(), text: String::new(), error: String::new() }));
                let k = model::kind_of(&e);
                if k == "text" || (k == "file" && e.size < 4 * 1024 * 1024) {
                    let p = e.path.clone();
                    return Task::perform(backend::call(a_(&["peek", &p]), 10), move |r| Msg::Did(Then::Peek(p.clone()), r));
                }
            }
        }
        "mark" => {
            if let Some(e) = &e {
                if !a.marked.remove(&e.path) {
                    a.marked.insert(e.path.clone());
                }
            }
        }
        "markall" => a.marked = a.entries.iter().map(|e| e.path.clone()).collect(),
        "unmarkall" => a.marked.clear(),
        "invert" => a.marked = a.entries.iter().filter(|e| !a.marked.contains(&e.path)).map(|e| e.path.clone()).collect(),
        "copy" => return a.set_clip("copy", t),
        "cut" => return a.set_clip("cut", t),
        "paste" => return a.paste(),
        "copypath" | "copywin" => {
            let cps = copy_targets(a, on_folder, &t);
            if !cps.is_empty() {
                let linux = id == "copypath";
                let text = cps.iter().map(|p| a.copy_form(p, linux)).collect::<Vec<_>>().join("\n");
                return copy_text(a, &text);
            }
        }
        "rename" => {
            if let Some(e) = &e {
                if a.entries.iter().any(|x| x.path == e.path) && !a.in_trash() {
                    a.renaming = Some(e.path.clone());
                    a.rename_text = e.name.clone();
                    let to = if e.dir { e.name.chars().count() } else { e.name.rfind('.').filter(|d| *d > 0).map(|d| e.name[..d].chars().count()).unwrap_or(e.name.chars().count()) };
                    return Task::batch(vec![operation::focus(RENAME_ID), operation::select_range(RENAME_ID, 0, to)]);
                }
            }
        }
        "batchrename" => return a.ask(&format!("Rename {} items (# becomes 1, 2, 3…)", t.len()), "Name #", "Rename all", 4, Ask::BatchRename(t)),
        "compress" => {
            let name = format!("{}.zip", if t.len() == 1 { let b = base(&t[0]); b.rfind('.').filter(|d| *d > 0).map(|d| b[..d].to_string()).unwrap_or(b) } else { a.listing.as_ref().map(|l| l.name.clone()).unwrap_or_else(|| "Archive".into()) });
            let n = name.chars().count() - 4;
            return a.ask("Compress (end the name with .zip, .7z, .tar.gz or .tar.xz)", &name, "Compress", n, Ask::Compress(t, a.path.clone()));
        }
        "extracthere" => {
            if let Some(e) = &e {
                let req = JobReq { verb: "extract".into(), paths: vec![e.path.clone()], dest: a.path.clone(), decide: Map::new(), fmt: String::new(), archive: e.path.clone(), here: true, undo: None };
                return a.start_job(req);
            }
        }
        "extractto" => {
            if let Some(e) = &e {
                let into = format!("{}/{}", if a.writable() { a.path.clone() } else { a.home.clone() }, model::archive_stem(&e.name));
                let n = into.chars().count();
                return a.ask(&format!("Extract “{}” to", e.name), &into, "Extract", n, Ask::ExtractTo(e.path.clone()));
            }
        }
        "newfolder" => {
            if !a.writable() {
                return Task::none();
            }
            let names: Vec<String> = a.listing.as_ref().map(|l| l.entries.iter().map(|x| x.name.clone()).collect()).unwrap_or_default();
            let mut name = "New folder".to_string();
            let mut n = 1;
            while names.contains(&name) {
                n += 1;
                name = format!("New folder {n}");
            }
            let c = name.chars().count();
            return a.ask("New folder", &name, "Create", c, Ask::NewFolder(a.path.clone()));
        }
        "newfile" => {
            if !a.writable() {
                return Task::none();
            }
            let names: Vec<String> = a.listing.as_ref().map(|l| l.entries.iter().map(|x| x.name.clone()).collect()).unwrap_or_default();
            let mut name = "New file.txt".to_string();
            let mut n = 1;
            while names.contains(&name) {
                n += 1;
                name = format!("New file {n}.txt");
            }
            let c = name.rfind('.').unwrap_or(name.len());
            return a.ask("New empty file", &name, "Create", c, Ask::NewFile(a.path.clone()));
        }
        "sub-new" => {
            let mut items = vec![MenuItem::new("newfile", "Empty file")];
            for (i, tp) in a.templates.iter().enumerate() {
                items.push(MenuItem::new(&format!("tpl:{i}"), &sv(tp, "name")));
            }
            a.menu_on_folder = true;
            a.show_menu("New document", items, false);
        }
        "sub-sort" => a.sort_menu(),
        "sub-view" => a.view_menu(),
        "asc" => {
            a.desc = false;
            a.rebuild();
            return a.remember_view_pub();
        }
        "desc" => {
            a.desc = true;
            a.rebuild();
            return a.remember_view_pub();
        }
        "trash" => {
            // As administrator in a system folder: deleted for good (there's no Trash for system files).
            if crate::extras::admin_here(a) && t.iter().any(|p| backend::system_path(p)) {
                let title = if t.len() == 1 { format!("Delete “{}” for good?", base(&t[0])) } else { format!("Delete {} items for good?", t.len()) };
                a.confirm(&title, "As administrator, system files have no Trash: this can't be undone.", "Delete", Yes::Delete(t));
                return Task::none();
            }
            return a.trash_paths(t);
        }
        "termpanel" => return crate::terminal::update(a, crate::terminal::TMsg::Toggle),
        "splitview" => return crate::split::update(a, crate::split::SMsg::Toggle),
        "p-auto" | "tp-auto" => {
            let (net, on) = if id == "p-auto" {
                match a.places.get(a.place_index) {
                    Some(p) => (p.raw.get("net").cloned().unwrap_or(Value::Null), !(p.raw.get("auto") == Some(&Value::Bool(true)) || sv(&p.raw, "sub") == "Connects by itself")),
                    None => return Task::none(),
                }
            } else {
                match &e {
                    Some(e) => (e.item.get("net").cloned().unwrap_or(Value::Null), sv(&e.item, "sub") != "Connects by itself"),
                    None => return Task::none(),
                }
            };
            if !net.is_object() {
                return Task::none();
            }
            return crate::extras::update(a, crate::extras::XMsg::Auto(net, on));
        }
        "delete" => {
            if t.is_empty() {
                return Task::none();
            }
            let title = if t.len() == 1 { format!("Delete “{}” for good?", base(&t[0])) } else { format!("Delete {} items for good?", t.len()) };
            a.confirm(&title, "This can't be undone. To keep a way back, use Move to Trash instead.", "Delete", Yes::Delete(t));
        }
        "restore" => {
            let n = t.len();
            let mut args = vec!["restore".to_string()];
            args.extend(t);
            return Task::perform(backend::call(args, 60), move |r| Msg::Did(Then::Restored(n), r));
        }
        "empty" => a.confirm("Empty the Trash?", "Everything in the Trash will be deleted for good.", "Empty Trash", Yes::Empty),
        "hidden" => {
            a.hidden = !a.hidden;
            let t1 = a.remember_view_pub();
            let t2 = a.reload();
            let t3 = a.say(if a.hidden { "Showing hidden files" } else { "Hiding hidden files" }, false);
            return Task::batch(vec![t1, t2, t3]);
        }
        "exts" => {
            a.show_ext = !a.show_ext;
            let t1 = a.remember_view_pub();
            let t2 = a.say(if a.show_ext { "Showing file name extensions" } else { "Hiding file name extensions" }, false);
            return Task::batch(vec![t1, t2]);
        }
        "compactview" => {
            let on = !a.prefs.get("compact").and_then(Value::as_bool).unwrap_or(false);
            a.prefs.insert("compact".into(), json!(on));
            return Task::perform(backend::call(vec!["prefs-set".into(), json!({ "compact": on }).to_string()], 15), |r| Msg::Did(Then::Nothing, r));
        }
        "detailspane" => {
            a.details_pane = !a.details_pane;
            let t1 = a.remember_view_pub();
            return Task::batch(vec![t1, a.details_lookups()]);
        }
        "describe" => return a.describe.update(hearth_ui::describe::DMsg::Open).map(Msg::Describe),
        "layout-original" => return a.describe.update(hearth_ui::describe::DMsg::Reset).map(Msg::Describe),
        "navpane" => {
            let on = !a.nav_shown();
            return a.set_files_pref("navPane", json!(on));
        }
        "undo" => return a.run_undo(true),
        "redo" => return a.run_undo(false),
        "refresh" => return Task::batch(vec![a.reload(), a.refresh_places()]),
        "bookmark" => {
            let bp = match &e {
                Some(e) if e.dir => e.path.clone(),
                _ => a.path.clone(),
            };
            if !bp.starts_with('/') {
                return Task::none();
            }
            let msg = format!("Pinned “{}” to the left menu", base(&bp));
            return Task::perform(backend::call(a_(&["bookmark-add", &bp]), 15), move |r| Msg::Did(Then::PlacesToo(msg.clone()), r));
        }
        "lockbc" => {
            if let Some(e) = &e {
                // Briefcase.qml's lockAsBriefcase: a new Briefcase beside it holding this folder, then its lock.
                return crate::briefcase::lock_as_briefcase(e.path.clone());
            }
        }
        "prevver" => {
            if let Some(e) = &e {
                let p = e.path.clone();
                return a.look_for_versions(p, true);
            }
        }
        "terminal" => {
            let p = match &e {
                Some(e) if e.dir => e.path.clone(),
                _ => if a.on_home() { a.home.clone() } else { a.path.clone() },
            };
            return Task::perform(backend::call(a_(&["terminal", &p]), 15), |r| Msg::Did(Then::Nothing, r));
        }
        "compare" => {
            if t.len() == 2 {
                let mut args = vec!["compare".to_string()];
                args.extend(t.iter().cloned());
                return Task::perform(backend::call(args, 120), move |r| Msg::Did(Then::Compare(t.clone()), r));
            }
        }
        "props" => {
            if a.on_cview() || e.as_ref().map(|x| x.cv.is_some()).unwrap_or(false) {
                return a.c_props(e);
            }
            if a.on_special() {
                if a.on_this_pc() {
                    if let Some(c) = a.current().filter(|c| c.path.starts_with('/')).map(|c| c.path.clone()) {
                        return a.properties(vec![c]);
                    }
                }
                return Task::none();
            }
            let paths = if !t.is_empty() { t } else if a.on_home() { vec![a.home.clone()] } else { vec![a.path.clone()] };
            return a.properties(paths);
        }
        "c-props" => return a.c_props(e),
        "c-newtab" => {
            if let Some(e) = &e {
                let p = e.path.clone();
                return a.new_tab(&p);
            }
        }
        "c-real" => {
            if let Some(e) = &e {
                let r = a.c_real_of(e);
                if !r.is_empty() {
                    return a.navigate(&r, "");
                }
            }
        }
        "c-linux" => {
            let on = !a.c_linux();
            return a.set_files_pref("linuxFolders", json!(on));
        }
        "c-part" => return a.say("Hearth's own apps are part of Hearth OS: look only, and they can't be uninstalled", false),
        "tp-apps" => return a.navigate(APPS_PAGE, ""),
        "tp-unmount" | "tp-eject" | "p-unmount" | "p-eject" => {
            let (label, path) = if id.starts_with("tp-") {
                match &e {
                    Some(e) => (e.name.clone(), e.path.clone()),
                    None => return Task::none(),
                }
            } else {
                match a.places.get(a.place_index) {
                    Some(p) => (p.label.clone(), p.path.clone()),
                    None => return Task::none(),
                }
            };
            let ej = id.ends_with("eject");
            let t1 = a.say(&format!("{} “{label}”…", if ej { "Removing" } else { "Unmounting" }), false);
            return Task::batch(vec![t1, Task::perform(backend::call(a_(&[if ej { "eject" } else { "unmount" }, &path]), 60), move |r| Msg::Did(Then::Unmounted(ej, label.clone(), path.clone()), r))]);
        }
        "ia-open" => {
            if let Some(app) = e.as_ref().and_then(|e| e.app.clone()) {
                let id = sv(&app, "desktop");
                let id = if id.is_empty() { sv(&app, "id") } else { id };
                backend::detached(&[String::from("gtk-launch"), id], None);
                return a.say(&format!("Opening {}…", sv(&app, "name")), false);
            }
        }
        "ia-folder" => {
            if let Some(app) = e.as_ref().and_then(|e| e.app.clone()) {
                let p = sv(&app, "path");
                if !p.is_empty() {
                    return a.navigate(&p, "");
                }
            }
        }
        "ia-settings" => {
            backend::detached(&[String::from("hearth-settings-app"), "--page".into(), "apps".into()], None);
        }
        "p-open" => {
            let i = a.place_index;
            return a.open_place(i);
        }
        "p-newtab" => {
            if let Some(p) = a.places.get(a.place_index).map(|p| p.path.clone()) {
                return a.new_tab(&p);
            }
        }
        "p-props" => {
            if let Some(p) = a.places.get(a.place_index).map(|p| p.path.clone()) {
                return a.properties(vec![p]);
            }
        }
        "p-mount" => {
            if let Some(p) = a.places.get(a.place_index).cloned() {
                let label = p.label.clone();
                return Task::perform(backend::call(a_(&["mount", &sv(&p.raw, "device")]), 60), move |r| Msg::Did(Then::Mounted(false, label.clone()), r));
            }
        }
        "p-unbookmark" => {
            if let Some(p) = a.places.get(a.place_index).map(|p| p.path.clone()) {
                return Task::perform(backend::call(a_(&["bookmark-remove", &p]), 15), |r| Msg::Did(Then::PlacesToo(String::new()), r));
            }
        }
        _ => {}
    }
    Task::none()
}

fn a_(v: &[&str]) -> Vec<String> {
    a(v)
}

pub fn show_apps(a: &mut App, e: &Entry, all: bool) -> Task<Msg> {
    a.overlay = Some(Overlay::Apps(AppsDlg { title: format!("Open “{}” with", e.name), path: e.path.clone(), all, apps: vec![], loaded: false, sel: 0 }));
    let mut args = a_(&["apps", &e.path]);
    if all {
        args.push("--all".into());
    }
    let p = e.path.clone();
    Task::perform(backend::call(args, 20), move |r| Msg::Did(Then::OpenApps(p.clone(), all), r))
}

pub fn open_with(a: &mut App, i: Option<usize>) -> Task<Msg> {
    let Some(Overlay::Apps(d)) = a.overlay.clone() else { return Task::none() };
    match i.and_then(|i| d.apps.get(i).cloned()) {
        Some(app) => {
            a.overlay = None;
            let name = sv(&app, "name");
            let t1 = a.say(&format!("Opening with {name}…"), false);
            Task::batch(vec![t1, Task::perform(backend::call(a_(&["open", &d.path, &sv(&app, "id")]), 20), |r| Msg::Did(Then::Nothing, r))])
        }
        None => {
            let e = Entry { name: base(&d.path), path: d.path.clone(), ..Default::default() };
            show_apps(a, &e, !d.all)
        }
    }
}

pub fn finish_rename(a: &mut App) -> Task<Msg> {
    let Some(p) = a.renaming.take() else { return Task::none() };
    let text = a.rename_text.trim().to_string();
    let Some(e) = a.entries.iter().find(|x| x.path == p).cloned() else { return Task::none() };
    if text.is_empty() || text == e.name {
        return Task::none();
    }
    let from = e.path.clone();
    Task::perform(backend::call(vec!["rename".into(), e.path.clone(), text], 30), move |r| Msg::Did(Then::Renamed(from.clone()), r))
}

pub fn end_edit_path(a: &mut App, go: bool) -> Task<Msg> {
    let t = a.path_text.trim().to_string();
    a.editing_path = false;
    a.zone = Zone::Main;
    if !go || t.is_empty() {
        return Task::none();
    }
    let lt = t.to_lowercase();
    let lt = lt.trim_end_matches(['\\', '/']);
    if lt == "this pc" {
        return a.navigate(THIS_PC, "");
    }
    if lt == "home" {
        return a.navigate(HOME_PAGE, "");
    }
    if lt == "this pc\\installed apps" || lt == "this pc/installed apps" || lt == "installed apps" {
        return a.navigate(APPS_PAGE, "");
    }
    let ctx = a.ctx();
    match drive::from_windows(&t, &ctx) {
        Some(drive::Res::Path(p)) => return a.navigate(&p, ""),
        Some(drive::Res::Error(e)) => return a.say(&e, true),
        None => {}
    }
    let mut p = t.strip_prefix("file://").map(|s| s.to_string()).unwrap_or(t);
    if let Some(r) = p.strip_prefix('~') {
        p = format!("{}{}", a.home, r);
    }
    if !p.starts_with('/') {
        p = format!("{}/{}", a.path, p);
    }
    a.navigate(&p, "")
}

pub fn dialog_ok(a: &mut App) -> Task<Msg> {
    match a.overlay.clone() {
        Some(Overlay::Input(i)) => {
            a.overlay = None;
            // A password is sent as it was typed; other answers without spaces around them.
            let text = if i.secret { i.text.clone() } else { i.text.trim().to_string() };
            if text.is_empty() {
                return Task::none();
            }
            match i.ask {
                Ask::Rename(e) => {
                    let from = e.path.clone();
                    Task::perform(backend::call(vec!["rename".into(), e.path.clone(), text], 30), move |r| Msg::Did(Then::Renamed(from.clone()), r))
                }
                Ask::BatchRename(t) => {
                    let n = t.len();
                    let mut args = vec!["batch-rename".to_string(), text];
                    args.extend(t);
                    Task::perform(backend::call(args, 60), move |r| Msg::Did(Then::Batch(n), r))
                }
                Ask::NewFolder(dir) => {
                    let name = text.clone();
                    Task::perform(backend::call(vec!["mkdir".into(), format!("{dir}/{text}")], 15), move |r| Msg::Did(Then::Made(dir.clone(), name.clone()), r))
                }
                Ask::NewFile(dir) => {
                    let name = text.clone();
                    Task::perform(backend::call(vec!["new-file".into(), format!("{dir}/{text}")], 15), move |r| Msg::Did(Then::Made(dir.clone(), name.clone()), r))
                }
                Ask::FromTemplate(tp, dir) => {
                    let name = text.clone();
                    Task::perform(backend::call(vec!["from-template".into(), tp, dir.clone(), text], 30), move |r| Msg::Did(Then::Made(dir.clone(), name.clone()), r))
                }
                Ask::Compress(t, dir) => {
                    let low = text.to_lowercase();
                    let fmt = if low.ends_with(".tar.gz") || low.ends_with(".tgz") { "tar.gz" } else if low.ends_with(".tar.xz") || low.ends_with(".txz") { "tar.xz" } else if low.ends_with(".tar") { "tar" } else if low.ends_with(".7z") { "7z" } else { "zip" };
                    let file = if fmt == "zip" && !low.ends_with(".zip") { format!("{text}.zip") } else { text };
                    let req = JobReq { verb: "compress".into(), paths: t, dest: dir.clone(), decide: Map::new(), fmt: fmt.into(), archive: format!("{dir}/{file}"), here: false, undo: None };
                    a.start_job(req)
                }
                Ask::Fx(f) => crate::extras::ask_done(a, f, text),
                Ask::ExtractTo(arch) => {
                    let mut d = text;
                    if let Some(r) = d.strip_prefix('~') {
                        d = format!("{}{}", a.home, r);
                    }
                    if !d.starts_with('/') {
                        d = format!("{}/{}", a.path, d);
                    }
                    let req = JobReq { verb: "extract".into(), paths: vec![arch.clone()], dest: d, decide: Map::new(), fmt: String::new(), archive: arch, here: false, undo: None };
                    a.start_job(req)
                }
            }
        }
        Some(Overlay::Confirm(c)) => {
            a.overlay = None;
            if c.button == 2 {
                if let Some((_, y)) = c.alt {
                    return run_yes(a, y);
                }
            }
            run_yes(a, c.yes)
        }
        Some(Overlay::Bc) => crate::briefcase::update(a, crate::briefcase::BMsg::Ok),
        Some(Overlay::Sort) => crate::sortdl::update(a, crate::sortdl::SMsg::Go),
        Some(Overlay::Apps(d)) => open_with(a, Some(d.sel)),
        Some(Overlay::Preview(pv)) => {
            a.overlay = None;
            if let Some(i) = a.entries.iter().position(|e| e.path == pv.e.path) {
                return a.activate(i);
            }
            Task::none()
        }
        Some(Overlay::Props(_)) => props_apply(a, true),
        Some(Overlay::Conflict(c)) => conflict_choose(a, ["skip", "rename", "replace"][c.button.min(2)]),
        Some(Overlay::Menu(m)) => {
            if let Some(it) = m.items.get(m.sel).filter(|i| !i.sep && !i.heading) {
                let id = it.id.clone();
                return run_menu(a, &id);
            }
            Task::none()
        }
        None => Task::none(),
    }
}

impl App {
    pub fn pick_done_with(&mut self, paths: Vec<String>) -> Task<Msg> {
        if self.pick_done {
            return Task::none();
        }
        self.pick_done = true;
        let res = json!({ "paths": paths, "filter": self.pick_filter, "choices": Value::Object(self.pick_choice.clone()) });
        if let Ok(p) = std::env::var("HEARTH_PICK_RESULT") {
            let part = format!("{p}.part");
            if std::fs::write(&part, res.to_string()).is_ok() {
                let _ = std::fs::rename(&part, &p);
            }
        } else {
            println!("{res}");
        }
        iced::exit()
    }
}

/// What a yes/no question does when you say yes.
pub fn run_yes(a: &mut App, y: Yes) -> Task<Msg> {
    match y {
        Yes::Delete(t) => {
            let n = t.len();
            let mut args = vec!["delete".to_string()];
            args.extend(t);
            Task::perform(backend::call(args, 120), move |r| Msg::Did(Then::Deleted(n), r))
        }
        Yes::Empty => Task::perform(backend::call(a_(&["empty-trash"]), 120), |r| Msg::Did(Then::Emptied, r)),
        Yes::PickFinish(p) => a.pick_done_with(p),
        Yes::Wallpaper(p) => {
            backend::detached(&[String::from("hearth-wallpaper"), p], None);
            Task::none()
        }
        Yes::RunAppImage(p) => a.run_appimage(&p, true),
        Yes::Note => Task::none(),
        Yes::Then(m) => update(a, m),
    }
}

pub fn conflict_choose(a: &mut App, choice: &str) -> Task<Msg> {
    let Some(Overlay::Conflict(c)) = a.overlay.clone() else { return Task::none() };
    let mut d = c.decide.clone();
    let last = if c.all { c.list.len() - 1 } else { c.i };
    for k in c.i..=last {
        if let Some(x) = c.list.get(k) {
            d.insert(sv(x, "name"), json!(choice));
        }
    }
    if last + 1 >= c.list.len() {
        a.overlay = None;
        let req = JobReq { verb: c.verb.clone(), paths: c.paths.clone(), dest: c.dest.clone(), decide: d, fmt: String::new(), archive: String::new(), here: false, undo: None };
        return a.start_job(req);
    }
    a.overlay = Some(Overlay::Conflict(Cf { i: last + 1, decide: d, button: 1, ..c }));
    Task::none()
}

pub fn props_apply(a: &mut App, close: bool) -> Task<Msg> {
    let Some(Overlay::Props(p)) = a.overlay.clone() else { return Task::none() };
    let mut tasks = vec![];
    if p.cview.is_none() && p.paths.len() == 1 && p.loaded {
        let path = p.paths[0].clone();
        if p.mode != p.mode0 {
            tasks.push(Task::perform(backend::call(vec!["chmod".into(), path.clone(), format!("{:o}", p.mode)], 30), |r| Msg::Did(Then::Chmod, r)));
        }
        let name = p.name.trim().to_string();
        if !name.is_empty() && name != base(&path) {
            let from = path.clone();
            tasks.push(Task::perform(backend::call(vec!["rename".into(), path.clone(), name], 30), move |r| Msg::Did(Then::Renamed(from.clone()), r)));
        }
    }
    if close {
        a.overlay = None;
        if a.props_only && tasks.is_empty() {
            return a.close();
        }
    } else if let Some(Overlay::Props(pp)) = &mut a.overlay {
        pp.mode0 = pp.mode;
    }
    Task::batch(tasks)
}

/// What each answer from hearth-files leads to.
pub fn did(a: &mut App, then: Then, r: Reply) -> Task<Msg> {
    match then {
        Then::PickList(full, dir) => {
            if !r.ok() || sv(&r.value, "path") != dir {
                return a.say(&format!("There's no folder “{dir}”"), true);
            }
            let name = base(&full);
            let hit = r.get("entries").as_array().and_then(|l| l.iter().find(|e| sv(e, "name") == name)).map(Entry::from_json);
            return match hit {
                Some(h) if h.dir => {
                    a.pick_name.clear();
                    a.navigate(&h.path, "")
                }
                Some(_) => {
                    a.confirm(&format!("Replace “{name}”?"), &format!("A file with this name is already in “{}”. Replacing it overwrites what's there.", base(&dir)), "Replace", Yes::PickFinish(vec![full]));
                    Task::none()
                }
                None => a.pick_done_with(vec![full]),
            };
        }
        Then::Props(paths) => {
            if let Some(Overlay::Props(p)) = &mut a.overlay {
                if p.paths != paths {
                    return Task::none();
                }
                if !r.ok() {
                    p.error = r.err.clone();
                    p.loaded = true;
                    return Task::none();
                }
                p.info = r.value.clone();
                p.loaded = true;
                p.mode = r.get("mode").as_u64().unwrap_or(0) as u32;
                p.mode0 = p.mode;
                if r.get("dir").as_bool() == Some(true) || paths.len() > 1 {
                    let mut args = vec!["count".to_string()];
                    args.extend(paths.iter().cloned());
                    let key = paths.join("\n");
                    return Task::perform(backend::call(args, 300), move |r| Msg::Did(Then::Count(key.clone()), r));
                }
            }
            return Task::none();
        }
        Then::Count(key) => {
            if let Some(Overlay::Props(p)) = &mut a.overlay {
                if p.paths.join("\n") == key && r.ok() {
                    p.count = r.value.clone();
                }
            }
            return Task::none();
        }
        Then::Sum(algo) => {
            if let Some(Overlay::Props(p)) = &mut a.overlay {
                let v = if r.ok() { r.get("sum").as_str().or(r.get("hash").as_str()).or(r.get(&algo).as_str()).unwrap_or("").to_string() } else { format!("(couldn't: {})", r.err) };
                p.sums.insert(algo, Value::String(v));
            }
            return Task::none();
        }
        Then::Chmod => {
            if !r.ok() {
                return a.say(&r.err, true);
            }
            return a.reload();
        }
        _ => {}
    }
    if !r.ok() {
        if let Then::Nothing = then {
            if r.err.contains("permission") || r.err.contains("No terminal") || !r.err.is_empty() {
                return a.say(&r.err, true);
            }
            return Task::none();
        }
        if let Then::OpenApps(..) = then {
            a.overlay = None;
        }
        return a.say(&r.err, true);
    }
    match then {
        Then::Nothing => Task::none(),
        Then::Say(s) => a.say(&s, false),
        Then::Reload(s) => {
            let t = a.say(&s, false);
            Task::batch(vec![t, a.reload()])
        }
        Then::Renamed(from) => {
            let to = sv(&r.value, "path");
            if !to.is_empty() && to != from {
                a.push_undo(Op::Rename { from: from.clone(), to: to.clone() });
            }
            if let Some(Overlay::Props(p)) = &mut a.overlay {
                if p.paths.first() == Some(&from) {
                    p.paths = vec![to.clone()];
                }
            }
            let p = a.path.clone();
            Task::batch(vec![a.load(&p, &base(&to), false), a.refresh_places()])
        }
        Then::Made(dir, name) => {
            let p = sv(&r.value, "path");
            a.push_undo(Op::Create { items: vec![if p.is_empty() { format!("{dir}/{name}") } else { p.clone() }] });
            let sel = if p.is_empty() { name } else { base(&p) };
            a.load(&dir, &sel, false)
        }
        Then::Trashed(t) => {
            let paths: Vec<String> = r.get("trashed").as_array().map(|x| x.iter().filter_map(|v| v.as_str().map(String::from)).collect()).filter(|v: &Vec<String>| !v.is_empty()).unwrap_or_else(|| t.clone());
            a.push_undo(Op::Trash { paths });
            a.marked.clear();
            let msg = if t.len() == 1 { format!("Moved “{}” to the Trash", base(&t[0])) } else { format!("Moved {} items to the Trash", t.len()) };
            let s = a.say(&msg, false);
            Task::batch(vec![s, a.reload(), a.refresh_places()])
        }
        Then::Deleted(n) => {
            a.marked.clear();
            let s = a.say(&if n == 1 { "Deleted it".to_string() } else { format!("Deleted {n} items") }, false);
            Task::batch(vec![s, a.reload(), a.refresh_places()])
        }
        Then::Restored(n) => {
            a.marked.clear();
            let s = a.say(&if n == 1 { "Put it back".to_string() } else { format!("Put {n} items back") }, false);
            Task::batch(vec![s, a.reload(), a.refresh_places()])
        }
        Then::Emptied => {
            let s = a.say("The Trash is empty", false);
            let mut t = vec![s, a.refresh_places()];
            if a.in_trash() {
                t.push(a.reload());
            }
            Task::batch(t)
        }
        Then::PlacesToo(msg) => {
            let mut t = vec![a.refresh_places(), a.reload()];
            if !msg.is_empty() {
                t.push(a.say(&msg, false));
            }
            Task::batch(t)
        }
        Then::Batch(n) => {
            a.marked.clear();
            let first = r.get("paths").as_array().and_then(|x| x.first()).and_then(Value::as_str).map(base).unwrap_or_default();
            let s = a.say(&format!("Renamed {n} items"), false);
            let p = a.path.clone();
            Task::batch(vec![s, a.load(&p, &first, false)])
        }
        Then::Conflicts(verb, paths, dest) => {
            let list = r.get("conflicts").as_array().cloned().unwrap_or_default();
            if list.is_empty() {
                let req = JobReq { verb, paths, dest, decide: Map::new(), fmt: String::new(), archive: String::new(), here: false, undo: None };
                return a.start_job(req);
            }
            a.overlay = Some(Overlay::Conflict(Cf { verb, paths, dest, list, i: 0, decide: Map::new(), all: false, button: 1 }));
            Task::none()
        }
        Then::Clip(cut) => {
            let mut mode = if cut { "cut".to_string() } else { "copy".to_string() };
            let mut paths = a.clip.clone();
            let other: Vec<String> = r.get("paths").as_array().map(|x| x.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default();
            if !other.is_empty() && other != a.clip {
                mode = if r.get("cut").as_bool() == Some(true) { "cut".into() } else { "copy".into() };
                paths = other;
            }
            if paths.is_empty() {
                return a.say("Nothing to paste. Copy something first.", false);
            }
            if mode == "cut" {
                a.clip.clear();
                a.clip_mode.clear();
            }
            let dest = a.path.clone();
            a.transfer_to(if mode == "cut" { "move" } else { "copy" }, paths, dest)
        }
        Then::OpenPlan(e) => {
            let kind = sv(&r.value, "kind");
            match kind.as_str() {
                "text" => Task::perform(backend::call(a_(&["open-text", &e.path]), 15), |r| Msg::Did(Then::Nothing, r)),
                "archive" => {
                    let items = vec![
                        MenuItem::new("extracthere", "Extract here").icon(Ic::Zip),
                        MenuItem::new("extractto", "Extract to…").icon(Ic::Zip),
                        MenuItem::new("openwith", "Open with…").icon(Ic::OpenWith),
                    ];
                    a.show_menu(&e.name, items, false);
                    Task::none()
                }
                "script" | "program" => {
                    let rem = sv(&r.value, "remembered");
                    let p = e.path.clone();
                    match rem.as_str() {
                        "terminal" => Task::perform(backend::call(a_(&["run", &p, "--terminal"]), 15), |r| Msg::Did(Then::Nothing, r)),
                        "run" => Task::perform(backend::call(a_(&["run", &p]), 15), |r| Msg::Did(Then::Nothing, r)),
                        "text" => Task::perform(backend::call(a_(&["open-text", &p]), 15), |r| Msg::Did(Then::Nothing, r)),
                        _ => {
                            let items = vec![
                                MenuItem::new("fo-terminal", "Run in Terminal").icon(Ic::Keyboard),
                                MenuItem::new("fo-run", "Run").icon(Ic::Forward),
                                MenuItem::new("fo-text", "Open in Hearth Text").icon(Ic::FileText),
                            ];
                            a.show_menu(&format!("“{}” is a program", e.name), items, false);
                            Task::none()
                        }
                    }
                }
                _ => Task::perform(backend::call(a_(&["open", &e.path]), 20), |r| Msg::Did(Then::Nothing, r)),
            }
        }
        Then::OpenApps(p, _all) => {
            if let Some(Overlay::Apps(d)) = &mut a.overlay {
                if d.path == p {
                    d.apps = r.get("apps").as_array().cloned().unwrap_or_default();
                    d.loaded = true;
                }
            }
            Task::none()
        }
        Then::Peek(p) => {
            if let Some(Overlay::Preview(pv)) = &mut a.overlay {
                if pv.e.path == p {
                    pv.text = format!("{}{}", sv(&r.value, "text"), if r.get("more").as_bool() == Some(true) { "\n…" } else { "" });
                }
            }
            Task::none()
        }
        Then::Undone(is_undo, op) => {
            if is_undo { a.redo.push(op.clone()) } else { a.undo.push(op.clone()) }
            let s = a.say(&format!("{}{}", if is_undo { "Undone: " } else { "Redone: " }, App::op_name(&op)), false);
            Task::batch(vec![s, a.reload(), a.refresh_places()])
        }
        Then::Mounted(go, label) => {
            let p = sv(&r.value, "path");
            let s = a.say(&format!("“{label}” is ready"), false);
            let mut t = vec![s, a.refresh_places()];
            if go && !p.is_empty() {
                t.push(a.navigate(&p, ""));
            }
            Task::batch(t)
        }
        Then::Unmounted(ej, label, path) => {
            let s = a.say(&if ej { format!("You can take out “{label}” now") } else { format!("“{label}” is unmounted") }, false);
            let mut t = vec![s, a.refresh_places()];
            if model::under(&a.path, &path) {
                t.push(a.navigate(THIS_PC, ""));
            } else if a.on_this_pc() {
                t.push(a.reload());
            }
            Task::batch(t)
        }
        Then::Linked(msg) => {
            let items: Vec<String> = r.get("items").as_array().map(|x| x.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default();
            if !items.is_empty() {
                a.push_undo(Op::Create { items });
            }
            let s = a.say(&msg, false);
            Task::batch(vec![s, a.reload()])
        }
        Then::Compare(t) => {
            if r.get("same").as_bool() == Some(true) {
                return a.say(&format!("“{}” and “{}” are the same", base(&t[0]), base(&t[1])), false);
            }
            let diff = sv(&r.value, "diff");
            if !diff.is_empty() {
                let n = r.get("changed").as_u64().unwrap_or(0);
                backend::detached(&backend::argv(&a_(&["open-text", &diff, "--readonly"])), None);
                return a.say(&format!("{n} {} differ: the differences are open in the text editor", if n == 1 { "line" } else { "lines" }), false);
            }
            a.say("They're different", false)
        }
        _ => Task::none(),
    }
}

// ------------------------------------------------------------------ keys
pub fn key(a: &mut App, ev: KEvent) -> Task<Msg> {
    let (key, mods) = match ev {
        KEvent::KeyPressed { key, modifiers, .. } => (key, modifiers),
        KEvent::ModifiersChanged(m) => {
            a.mods = m;
            return Task::none();
        }
        _ => return Task::none(),
    };
    a.mods = mods;
    let ctrl = mods.control();
    let shift = mods.shift();
    let alt = mods.alt();
    let named = match key.as_ref() {
        Key::Named(n) => Some(n),
        _ => None,
    };
    let ch: Option<String> = match key.as_ref() {
        Key::Character(c) => Some(c.to_lowercase()),
        _ => None,
    };
    // Dialogs and menus first.
    if let Some(ov) = &mut a.overlay {
        match ov {
            Overlay::Menu(m) => {
                let n = m.items.len();
                let pickable = |i: usize, m: &Menu| !m.items[i].sep && !m.items[i].heading;
                match named {
                    Some(Named::Escape) | Some(Named::ArrowLeft) | Some(Named::Backspace) => {
                        a.overlay = None;
                    }
                    Some(Named::ArrowDown) | Some(Named::Tab) => {
                        let mut i = m.sel;
                        for _ in 0..n {
                            i = (i + 1) % n.max(1);
                            if pickable(i, m) {
                                break;
                            }
                        }
                        m.sel = i;
                    }
                    Some(Named::ArrowUp) => {
                        let mut i = m.sel;
                        for _ in 0..n {
                            i = (i + n - 1) % n.max(1);
                            if pickable(i, m) {
                                break;
                            }
                        }
                        m.sel = i;
                    }
                    Some(Named::Enter) | Some(Named::Space) | Some(Named::ArrowRight) => {
                        let i = m.sel;
                        return update(a, Msg::MenuPick(i));
                    }
                    _ => {}
                }
                return Task::none();
            }
            Overlay::Confirm(c) => {
                // With a third choice (Just this time): Cancel, it, then OK, left to right.
                let alt = c.alt.is_some();
                match named {
                    Some(Named::ArrowLeft) | Some(Named::ArrowUp) => c.button = if alt && c.button == 1 { 2 } else { 0 },
                    Some(Named::ArrowRight) | Some(Named::ArrowDown) | Some(Named::Tab) => c.button = if alt && c.button == 0 { 2 } else { 1 },
                    Some(Named::Enter) | Some(Named::Space) => {
                        if c.button == 1 || c.button == 2 {
                            return dialog_ok(a);
                        }
                        a.overlay = None;
                    }
                    Some(Named::Escape) | Some(Named::Backspace) => a.overlay = None,
                    _ => {}
                }
                return Task::none();
            }
            Overlay::Conflict(c) => {
                match named {
                    Some(Named::ArrowLeft) => c.button = c.button.min(2).saturating_sub(1),
                    Some(Named::ArrowRight) => c.button = (c.button + 1).min(2),
                    Some(Named::ArrowDown) => c.button = 3,
                    Some(Named::ArrowUp) => c.button = 1,
                    Some(Named::Enter) | Some(Named::Space) => {
                        if c.button == 3 {
                            c.all = !c.all;
                        } else {
                            let pick = ["skip", "rename", "replace"][c.button];
                            return conflict_choose(a, pick);
                        }
                    }
                    Some(Named::Escape) => {
                        a.overlay = None;
                        return a.say("Nothing was copied or moved", false);
                    }
                    _ => {}
                }
                return Task::none();
            }
            Overlay::Apps(d) => {
                let n = d.apps.len() + 1;
                match named {
                    Some(Named::ArrowUp) => d.sel = d.sel.saturating_sub(1),
                    Some(Named::ArrowDown) | Some(Named::Tab) => d.sel = (d.sel + 1).min(n - 1),
                    Some(Named::Enter) | Some(Named::Space) => {
                        let s = d.sel;
                        return open_with(a, if s < n - 1 { Some(s) } else { None });
                    }
                    Some(Named::Escape) | Some(Named::Backspace) | Some(Named::ArrowLeft) => a.overlay = None,
                    _ => {}
                }
                return Task::none();
            }
            Overlay::Preview(_) => {
                match named {
                    Some(Named::ArrowLeft) | Some(Named::ArrowUp) | Some(Named::ArrowRight) | Some(Named::ArrowDown) => {
                        let d: i64 = if matches!(named, Some(Named::ArrowLeft) | Some(Named::ArrowUp)) { -1 } else { 1 };
                        let i = (a.cur as i64 + d).clamp(0, a.entries.len().saturating_sub(1) as i64) as usize;
                        a.cur = i;
                        a.overlay = None;
                        return run_menu(a, "preview");
                    }
                    Some(Named::Enter) => return dialog_ok(a),
                    Some(Named::Escape) | Some(Named::Space) | Some(Named::Backspace) => a.overlay = None,
                    _ => {}
                }
                return Task::none();
            }
            Overlay::Bc => {
                return crate::briefcase::key(a, named, shift);
            }
            Overlay::Sort => {
                match named {
                    Some(Named::Enter) => return crate::sortdl::update(a, crate::sortdl::SMsg::Go),
                    Some(Named::Escape) | Some(Named::Backspace) => return crate::sortdl::update(a, crate::sortdl::SMsg::Cancel),
                    _ => {}
                }
                return Task::none();
            }
            Overlay::Input(_) | Overlay::Props(_) => {
                if let Some(Named::Escape) = named {
                    return update(a, Msg::DlgCancel);
                }
                if let Some(Named::Enter) = named {
                    return dialog_ok(a);
                }
                return Task::none();
            }
        }
    }
    if a.renaming.is_some() {
        if let Some(Named::Escape) = named {
            a.renaming = None;
        }
        return Task::none();
    }
    // The terminal panel's line (F4) and the split view (F9) take their own keys while you're in them.
    if a.zone == Zone::Term && a.term.on {
        if let Some(t) = crate::terminal::key(a, named, ctrl, ch.as_deref()) {
            return t;
        }
        if named.is_none() {
            return Task::none();
        }
    }
    if a.zone == Zone::Split && a.split.is_some() && !ctrl && !alt {
        if let Some(t) = crate::split::key(a, named) {
            return t;
        }
    }
    if a.editing_path {
        if let Some(Named::Escape) = named {
            a.editing_path = false;
            a.zone = Zone::Main;
        }
        return Task::none();
    }
    if alt && !ctrl {
        return match named {
            Some(Named::ArrowLeft) => a.go_back(),
            Some(Named::ArrowRight) => a.go_forward(),
            Some(Named::ArrowUp) => a.go_up(false),
            Some(Named::Home) => {
                let h = a.home.clone();
                a.navigate(&h, "")
            }
            Some(Named::Enter) => run_menu(a, "props"),
            _ if ch.as_deref() == Some("p") => command(a, "details"),
            _ => Task::none(),
        };
    }
    if ctrl {
        if let Some(c) = ch.as_deref() {
            return match c {
                "c" if alt => run_menu(a, "copypath"),
                "c" if shift => {
                    a.menu_on_folder = a.current().is_none();
                    let id = if a.copy_linux() { "copypath" } else { "copywin" };
                    run_menu(a, id)
                }
                "c" => {
                    let t = a.targets();
                    a.set_clip("copy", t)
                }
                "x" => {
                    let t = a.targets();
                    a.set_clip("cut", t)
                }
                "v" => a.paste(),
                "a" => run_menu(a, "markall"),
                "i" => run_menu(a, "invert"),
                "h" => run_menu(a, "hidden"),
                "f" | "e" => operation::focus(SEARCH_ID),
                "l" => update(a, Msg::EditPath),
                "r" => a.reload(),
                "d" => run_menu(a, "bookmark"),
                "t" if !a.picking() => update(a, Msg::NewTab),
                "w" if !a.picking() => {
                    let i = a.tab_index;
                    a.close_tab(i)
                }
                "n" if shift => run_menu(a, "newfolder"),
                "n" if alt => run_menu(a, "newfile"),
                "1" => run_menu(a, "view:grid"),
                "2" => run_menu(a, "view:compact"),
                "3" => run_menu(a, "view:details"),
                "4" => run_menu(a, "view:large"),
                "5" => run_menu(a, "view:small"),
                "6" => run_menu(a, "view:tiles"),
                "7" => run_menu(a, "view:xlarge"),
                "z" if shift => a.run_undo(false),
                "z" => a.run_undo(true),
                "y" => a.run_undo(false),
                "q" => a.close(),
                _ => Task::none(),
            };
        }
        return match named {
            Some(Named::Tab) | Some(Named::PageDown) if a.tabs.len() > 1 => {
                let i = if shift { (a.tab_index + a.tabs.len() - 1) % a.tabs.len() } else { (a.tab_index + 1) % a.tabs.len() };
                a.switch_tab(i)
            }
            Some(Named::PageUp) if a.tabs.len() > 1 => {
                let i = (a.tab_index + a.tabs.len() - 1) % a.tabs.len();
                a.switch_tab(i)
            }
            Some(Named::Space) => run_menu(a, "mark"),
            _ => Task::none(),
        };
    }
    match named {
        Some(Named::ContextMenu) => {
            a.cursor = Point::new(a.nav_w + 300.0, 220.0);
            if a.zone == Zone::Places {
                place_menu(a);
            } else {
                let f = a.current().is_none();
                a.open_menu(f);
            }
            return Task::none();
        }
        Some(Named::F10) if shift => {
            let f = a.current().is_none();
            a.open_menu(f);
            return Task::none();
        }
        Some(Named::F5) => return update(a, Msg::Refresh),
        Some(Named::F3) => return operation::focus(SEARCH_ID),
        Some(Named::F4) if shift => return run_menu(a, "terminal"),
        Some(Named::F4) if !a.picking() => return crate::terminal::update(a, crate::terminal::TMsg::Toggle),
        Some(Named::F9) if !a.picking() => return crate::split::update(a, crate::split::SMsg::Toggle),
        Some(Named::BrowserBack) => return a.go_back(),
        Some(Named::BrowserForward) => return a.go_forward(),
        _ => {}
    }
    if a.zone == Zone::Places {
        let n = a.places.len();
        return match named {
            Some(Named::ArrowDown) | Some(Named::Tab) => {
                a.place_index = (a.place_index + 1).min(n.saturating_sub(1));
                Task::none()
            }
            Some(Named::ArrowUp) => {
                a.place_index = a.place_index.saturating_sub(1);
                Task::none()
            }
            Some(Named::ArrowRight) | Some(Named::Enter) | Some(Named::Space) => {
                let i = a.place_index;
                a.open_place(i)
            }
            Some(Named::Escape) => a.close(),
            Some(Named::Backspace) => {
                a.zone = Zone::Main;
                Task::none()
            }
            _ => Task::none(),
        };
    }
    // The folder.
    let count = a.entries.len();
    let v = a.shown_view();
    let c = if matches!(v, View::Details) || a.on_cview() || a.on_apps() { 1 } else { a.cols.get().max(1) };
    let i = a.cur;
    let go = |a: &mut App, n: i64| -> Task<Msg> {
        if count == 0 {
            return Task::none();
        }
        let n = n.clamp(0, count as i64 - 1) as usize;
        if shift {
            let from = *a.anchor.get_or_insert(i);
            a.select_range(from, n);
        } else if a.marked.is_empty() {
            a.anchor = Some(n);
        }
        a.cur = n;
        a.touched = true;
        Task::batch(vec![a.show_current(), a.details_lookups()])
    };
    match named {
        Some(Named::ArrowLeft) => {
            if c == 1 || i % c == 0 {
                if !shift && a.nav_shown() {
                    a.place_index = a.places.iter().position(|p| p.path == a.path).unwrap_or(0);
                    a.zone = Zone::Places;
                }
                return Task::none();
            }
            return go(a, i as i64 - 1);
        }
        Some(Named::ArrowRight) => {
            if c > 1 && i + 1 < count && (i + 1) % c != 0 {
                return go(a, i as i64 + 1);
            }
            return Task::none();
        }
        Some(Named::ArrowUp) => return go(a, i as i64 - c as i64),
        Some(Named::ArrowDown) => {
            if i + c < count {
                return go(a, (i + c) as i64);
            }
            if a.picking() && !shift {
                a.zone = Zone::Pick;
                return operation::focus(PICKNAME_ID);
            }
            return go(a, count as i64 - 1);
        }
        Some(Named::PageDown) => return go(a, (i + c * 6) as i64),
        Some(Named::PageUp) => return go(a, i as i64 - (c * 6) as i64),
        Some(Named::Home) => return go(a, 0),
        Some(Named::End) => return go(a, count as i64 - 1),
        Some(Named::Enter) => return a.activate(i),
        Some(Named::Space) => {
            if a.current().is_some() {
                return run_menu(a, "preview");
            }
        }
        Some(Named::F2) => {
            if a.writable() && !a.in_trash() && a.marked.len() > 1 {
                return run_menu(a, "batchrename");
            }
            if a.current().is_some() && a.writable() && !a.in_trash() {
                return run_menu(a, "rename");
            }
        }
        Some(Named::Delete) => {
            if !a.targets().is_empty() && (a.writable() || a.in_trash()) {
                return run_menu(a, if shift || a.in_trash() { "delete" } else { "trash" });
            }
        }
        Some(Named::Escape) => {
            if !a.query.is_empty() {
                return update(a, Msg::SearchClear);
            }
            if a.found.is_some() {
                return update(a, Msg::SearchClear);
            }
            if !a.marked.is_empty() {
                a.marked.clear();
                return Task::none();
            }
            if a.picking() {
                return a.close();
            }
            return a.go_up(false);
        }
        Some(Named::Backspace) => {
            if !a.query.is_empty() {
                let mut q = a.query.clone();
                q.pop();
                return update(a, Msg::SearchText(q));
            }
            return a.go_up(false);
        }
        _ => {}
    }
    // Typing jumps to a name (type-ahead); "/" or "~" types a path.
    if let Some(c) = ch {
        if c == "/" || c == "~" {
            a.editing_path = true;
            a.path_text = if c == "~" { "~/".into() } else { "/".into() };
            return Task::batch(vec![operation::focus(PATH_ID), operation::move_cursor_to_end(PATH_ID)]);
        }
        if c.chars().count() == 1 && !alt {
            if let Some(j) = (1..=count).map(|k| (i + k) % count.max(1)).find(|j| a.entries.get(*j).map(|e| e.name.to_lowercase().starts_with(&c)).unwrap_or(false)) {
                a.cur = j;
                a.marked.clear();
                return Task::batch(vec![a.show_current(), a.details_lookups()]);
            }
        }
    }
    Task::none()
}
