//! Logitech G15, the app (hearth-g15-app): the approved mockups docs/mockups/g15 (g15-1 Overview, g15-2 G-keys,
//! g15-3 Keyboard screen, g15-4 Lighting, g15-6 Make your own screen), checklist docs/mockups/checklists/g15.md.
//!
//! It only reads and writes ~/.config/nimbus/g15.json (through settings_file.rs: only what you change, merged, with a
//! .bak; opening it writes nothing). The keyboard itself is the user service's (hearth-g15 service), which follows
//! the file at once: the lights, the key sets, the pages. The app asks the service for two live things through
//! small files in $XDG_RUNTIME_DIR/hearth: g15-show.json (Show on the keyboard) and g15-record.json (Record / Stop),
//! and shows what's on the keyboard's screen now from g15-frame.bin.
//!
//! Mouse, keyboard and the controller (as keys: hearth-pad sends A Enter, B Esc, LB / RB Ctrl+Page Up / Down, X the
//! Menu key): Up / Down move between rows, Left / Right change a choice, Enter picks, Esc goes back.
//!   hearth-g15-app [--page overview|g-keys|screen|lighting|editor[:ID]] [--key N] [--set N] [--element N]
//!                  [--add-picture FILE [--at X]] (the editor, for CI's pictures)

mod pics;
mod view;

use hearth_g15::actions;
use hearth_g15::config;
use hearth_g15::lcd::{self, Frame as Lcd};
use hearth_g15::root::{self, Root};
use hearth_g15::service::{self, Previewer};
use hearth_ui::app::{self, win};
use hearth_ui::run;
use hearth_ui::Pal;
use iced::keyboard::{key::Named, Event as KEvent, Key, Modifiers};
use iced::{Subscription, Task};
use serde_json::{json, Map, Value};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Page {
    Overview,
    GKeys,
    Screen,
    Lighting,
    Editor,
}

pub const PAGES: [Page; 4] = [Page::Overview, Page::GKeys, Page::Screen, Page::Lighting];

/// A dialog over the page.
#[derive(Debug, Clone, PartialEq)]
pub enum Dlg {
    Rename(String),
    Hearth,
    Apps(Vec<(String, String)>),
    Games(Vec<String>),
    Pick(&'static str, Vec<(String, String)>),
}

#[derive(Debug, Clone)]
pub enum Msg {
    Theme,
    Tick,
    Live,
    Key(KEvent),
    Drag,
    Maximize,
    Minimize,
    Close,
    Resize(iced::window::Direction),
    Go(Page),
    Focus(usize),
    // settings
    Flag(&'static str, bool),
    Level(&'static str, u32),
    Num(&'static str, u64),
    UseSet(u8),
    // G-keys
    ViewSet(u8),
    PickG(u8),
    Kind(&'static str),
    TestIt,
    ClearKey,
    Change,
    CaptureKeys,
    Text(String),
    TextSave,
    HearthAction(&'static str),
    AppPicked(String, String),
    Record,
    StopRecord,
    MacroOpt(&'static str),
    RenameOpen,
    RenameText(String),
    RenameSave,
    GamesOpen,
    GameText(usize, String),
    GamesSave,
    CloseDlg,
    Picked(&'static str, String),
    // keyboard screen
    TogglePage(String),
    MovePage(String, i32),
    DragPage(String),
    OverPage(String),
    DropPage,
    Edit(String),
    NewScreen,
    Show,
    // editor
    Add(&'static str),
    PickEl(usize),
    Press(iced::Point),
    Move(iced::Point),
    Release,
    Nudge(i32, i32),
    ElSet(&'static str, Value),
    ElText(String),
    ScreenName(String),
    ScreenField(&'static str, Value),
    WhileText(String),
    Undo,
    Done,
    Delete,
    PictureFile(run::Output),
    /// Pressed on one of Add's buttons: let go on the screen to put it there, anywhere else to add it at its usual place.
    AddGrab(&'static str),
    AddDrop(iced::Point),
    AddEnd,
    /// "Also show it": done typing a game's name.
    WhileDone,
    Saved(Result<(), String>),
}

pub struct App {
    pub p: Pal,
    pub root: Root,
    pub page: Page,
    pub cfg: Value,
    pub status: Value,
    pub prev: Previewer,
    /// What the keyboard's screen shows now (the service's last frame), and the pages' pictures.
    pub now: Option<Lcd>,
    pub shots: Vec<(String, Lcd)>,
    pub focus: usize,
    pub dlg: Option<Dlg>,
    pub dlg_focus: usize,
    // G-keys
    pub set: u8,
    pub g: u8,
    pub text: String,
    pub capturing: bool,
    pub recording: bool,
    pub note: String,
    // Keyboard screen
    pub dragging: Option<String>,
    pub over: Option<String>,
    // the editor
    pub screen: String,
    pub el: Option<usize>,
    pub grab: Option<(i32, i32, bool)>,
    pub undo: Vec<Value>,
    pub editor_frame: Option<Lcd>,
    /// The picked element is picked up (A, or clicked): the arrows move it one dot; A or B puts it down.
    pub holding: bool,
    /// Add's button being dragged, where it was let go on the screen (in dots), and the screens before a drag.
    pub adding: Option<&'static str>,
    pub drop_at: Option<(i32, i32)>,
    pub grab_before: Option<Value>,
    /// "Also show it": typing a game's name.
    pub while_typing: bool,
}

/// One change to g15.json: the value at a path (None removes it). Applied to the app's copy at once, and to the file
/// on its own thread (merged under the lock: only this path changes).
#[derive(Debug, Clone)]
pub struct Change {
    pub path: Vec<String>,
    pub value: Option<Value>,
}

pub fn apply(m: &mut Map<String, Value>, c: &Change) {
    let mut cur = m;
    for (i, k) in c.path.iter().enumerate() {
        if i + 1 == c.path.len() {
            match &c.value {
                Some(v) => {
                    cur.insert(k.clone(), v.clone());
                }
                None => {
                    cur.remove(k);
                }
            }
            return;
        }
        let next = cur.entry(k.clone()).or_insert_with(|| json!({}));
        if !next.is_object() {
            *next = json!({});
        }
        cur = next.as_object_mut().unwrap();
    }
}

fn s(v: &Value, k: &str) -> String {
    v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string()
}

impl App {
    fn new() -> (App, Task<Msg>) {
        let root = Root::from_env();
        let cfg = config::load();
        let set = app::arg("--set").and_then(|x| x.parse().ok()).filter(|&n: &u8| (1..=3).contains(&n)).unwrap_or_else(|| config::set_in_use(&cfg));
        let g = app::arg("--key").and_then(|x| x.parse().ok()).filter(|&n: &u8| (1..=18).contains(&n)).unwrap_or(5);
        let mut a = App {
            p: hearth_ui::theme::load(),
            status: service::status(&root),
            prev: Previewer::new(root.clone()),
            root,
            page: Page::Overview,
            cfg,
            now: None,
            shots: Vec::new(),
            focus: 0,
            dlg: None,
            dlg_focus: 0,
            set,
            g,
            text: String::new(),
            capturing: false,
            recording: false,
            note: String::new(),
            dragging: None,
            over: None,
            screen: String::new(),
            el: None,
            grab: None,
            undo: Vec::new(),
            editor_frame: None,
            holding: false,
            adding: None,
            drop_at: None,
            grab_before: None,
            while_typing: false,
        };
        let first = app::arg("--page").unwrap_or_default();
        let (pg, part) = first.split_once(':').map(|(a, b)| (a.to_string(), b.to_string())).unwrap_or((first.clone(), String::new()));
        a.page = match pg.as_str() {
            "g-keys" | "gkeys" => Page::GKeys,
            "screen" => Page::Screen,
            "lighting" => Page::Lighting,
            "editor" => Page::Editor,
            _ => Page::Overview,
        };
        if a.page == Page::Editor {
            let id = if part.is_empty() { a.own().first().map(|x| s(x, "id")).unwrap_or_default() } else { part };
            if id.is_empty() {
                a.page = Page::Screen;
            } else {
                a.screen = id;
                a.el = app::arg("--element").and_then(|x| x.parse().ok()).or(Some(0));
            }
        }
        a.load_key();
        let pg = a.page;
        if pg != Page::Editor {
            let _ = update(&mut a, Msg::Go(pg));
        }
        a.refresh_pictures();
        if let Some(i) = a.el.filter(|_| a.page == Page::Editor) {
            a.focus = view::ix(&a, &format!("el{i}"));
        }
        // CI's pictures: --add-picture FILE adds a picture to the open screen as Add › Picture does (on a copy).
        if let Some(path) = app::arg("--add-picture").filter(|_| a.page == Page::Editor) {
            match lcd::picture::element(&path, app::arg("--at").and_then(|x| x.parse().ok()).unwrap_or(0), 0) {
                Ok(e) => {
                    let t = a.put_new(e);
                    return (a, t);
                }
                Err(e) => a.note = format!("Couldn't use that picture: {e}"),
            }
        }
        // CI's pictures: --show hearth|apps|rename|games opens that dialog.
        match app::arg("--show").as_deref() {
            Some("hearth") => a.dlg = Some(Dlg::Hearth),
            Some("rename") => a.dlg = Some(Dlg::Rename(a.set_name(a.set))),
            Some("games") => a.dlg = Some(Dlg::Games(a.games())),
            Some("apps") => a.dlg = Some(Dlg::Apps(apps_list())),
            _ => {}
        }
        (a, Task::none())
    }

    pub fn own(&self) -> Vec<Value> {
        self.cfg.get("screens").and_then(|s| s.as_array()).cloned().unwrap_or_default()
    }
    pub fn plugged(&self) -> bool {
        self.status.get("plugged").and_then(|v| v.as_bool()).unwrap_or(false)
    }
    pub fn set_name(&self, n: u8) -> String {
        let name = self.cfg.get("names").and_then(|x| x.get(format!("m{n}"))).and_then(|v| v.as_str()).unwrap_or("").to_string();
        if !name.is_empty() {
            name
        } else if config::keys_set(&self.cfg, n) == 0 {
            "Empty".into()
        } else {
            format!("Key set {n}")
        }
    }
    pub fn action(&self, set: u8, g: u8) -> Option<Value> {
        config::key_action(&self.cfg, set, g).cloned()
    }
    /// What a key does, in words ("Not set" when nothing).
    pub fn says(&self, set: u8, g: u8) -> String {
        match self.action(set, g) {
            Some(v) => {
                if let Some(n) = v.get("label").and_then(|x| x.as_str()).filter(|x| !x.is_empty()) {
                    return n.to_string();
                }
                match actions::parse(&v) {
                    Some(actions::Action::Macro(_)) => format!("Macro: {}", { let n = s(&v, "name"); if n.is_empty() { "recorded".into() } else { n } }),
                    Some(actions::Action::Keys(k)) => key_words(&hearth_g15::keys::combo_text(&k)),
                    Some(actions::Action::App { desktop, command }) => format!("Open {}", app_name(&desktop).unwrap_or(if command.is_empty() { desktop } else { command })),
                    Some(a) => actions::describe(&a),
                    None => "Not set".into(),
                }
            }
            None => "Not set".into(),
        }
    }
    pub fn kind(&self) -> String {
        self.action(self.set, self.g).map(|v| s(&v, "kind")).unwrap_or_else(|| "none".into())
    }
    fn load_key(&mut self) {
        self.capturing = false;
        self.text = self.action(self.set, self.g).map(|v| s(&v, "text")).unwrap_or_default();
    }
    pub fn games(&self) -> Vec<String> {
        let rules: Vec<Value> = self.cfg.get("gameSets").and_then(|g| g.as_array()).cloned().unwrap_or_default();
        (1..=3u8).map(|n| rules.iter().find(|r| r.get("set").and_then(|x| x.as_u64()) == Some(n as u64)).map(|r| s(r, "game")).unwrap_or_default()).collect()
    }
    pub fn pages_on(&self) -> Vec<String> {
        config::pages(&self.cfg)
    }
    /// Every page that can be on the keyboard's screen, on or off, in their order (the ones on first, as saved).
    pub fn all_pages(&self) -> Vec<String> {
        let mut v = self.pages_on();
        for b in hearth_g15::lcd::screen::BUILT_IN {
            if !v.iter().any(|x| x == b) {
                v.push(b.to_string());
            }
        }
        v
    }

    /// Saves changes: the app's copy at once, the file on its own thread.
    fn save(&mut self, changes: Vec<Change>) -> Task<Msg> {
        if let Some(m) = self.cfg.as_object_mut() {
            for c in &changes {
                apply(m, c);
            }
        } else {
            let mut m = Map::new();
            for c in &changes {
                apply(&mut m, c);
            }
            self.cfg = Value::Object(m);
        }
        self.prev.reload_with(self.cfg.clone());
        let (tx, rx) = futures::channel::oneshot::channel();
        let _ = std::thread::Builder::new().name("hearth-g15-save".into()).spawn(move || {
            let r = config::update(|m| {
                for c in &changes {
                    apply(m, c);
                }
            });
            let _ = tx.send(r.map_err(|e| e.to_string()));
        });
        Task::perform(async move { rx.await.unwrap_or(Err("it stopped".into())) }, Msg::Saved)
    }
    fn set1(&mut self, path: &[&str], v: Option<Value>) -> Task<Msg> {
        self.save(vec![Change { path: path.iter().map(|x| x.to_string()).collect(), value: v }])
    }
    fn set_action(&mut self, v: Option<Value>) -> Task<Msg> {
        let (set, g) = (self.set, self.g);
        self.set1(&["keys", &format!("m{set}"), &format!("g{g}")], v)
    }

    pub fn refresh_pictures(&mut self) {
        let first = self.pages_on().first().cloned().unwrap_or_else(|| "clock".into());
        self.now = match service::frame_now() {
            Some(f) => Some(f),
            None => Some(self.prev.page(&first)),
        };
        if self.page == Page::Screen {
            let mut ids = self.all_pages();
            for o in self.own() {
                let id = s(&o, "id");
                if !ids.contains(&id) {
                    ids.push(id);
                }
            }
            self.shots = ids.iter().map(|id| (id.clone(), self.prev.page(id))).collect();
        }
        if self.page == Page::Editor {
            let els = self.cur_screen().and_then(|x| x.get("elements").and_then(|e| e.as_array()).cloned()).unwrap_or_default();
            self.editor_frame = Some(self.prev.elements(&els));
        }
    }
    pub fn shot(&self, id: &str) -> Lcd {
        self.shots.iter().find(|(k, _)| k == id).map(|(_, f)| f.clone()).unwrap_or_default()
    }

    // ---------------------------------------------------------------- the editor's screen
    pub fn cur_screen(&self) -> Option<Value> {
        self.own().into_iter().find(|x| s(x, "id") == self.screen)
    }
    pub fn elements(&self) -> Vec<Value> {
        self.cur_screen().and_then(|x| x.get("elements").and_then(|e| e.as_array()).cloned()).unwrap_or_default()
    }
    fn put_screen(&mut self, scr: Value, keep_undo: bool) -> Task<Msg> {
        let mut all = self.own();
        if keep_undo {
            self.undo.push(Value::Array(all.clone()));
            if self.undo.len() > 50 {
                self.undo.remove(0);
            }
        }
        match all.iter().position(|x| s(x, "id") == self.screen) {
            Some(i) => all[i] = scr,
            None => all.push(scr),
        }
        let t = self.set1(&["screens"], Some(Value::Array(all)));
        self.refresh_pictures();
        t
    }
    fn put_elements(&mut self, els: Vec<Value>, keep_undo: bool) -> Task<Msg> {
        let Some(mut scr) = self.cur_screen() else { return Task::none() };
        scr["elements"] = Value::Array(els);
        self.put_screen(scr, keep_undo)
    }
    fn set_el(&mut self, k: &str, v: Value) -> Task<Msg> {
        let Some(i) = self.el else { return Task::none() };
        let mut els = self.elements();
        let Some(e) = els.get_mut(i) else { return Task::none() };
        let gi = |e: &Value, k: &str, d: i64| e.get(k).and_then(|x| x.as_i64()).unwrap_or(d);
        let was = s(e, "type");
        if k == "type" && was == "box" && v == json!("line") {
            // A box becomes a line along its top; a line becomes the box around it.
            let (x, y, w) = (gi(e, "x", 0), gi(e, "y", 0), gi(e, "w", 20));
            e["x2"] = json!(x + w - 1);
            e["y2"] = json!(y);
        } else if k == "type" && was == "line" && v == json!("box") {
            let (x, y, w, h) = bounds(e);
            e["x"] = json!(x);
            e["y"] = json!(y);
            e["w"] = json!(w.max(2));
            e["h"] = json!(h.max(2));
        }
        e[k] = v;
        if s(e, "type") == "picture" && matches!(k, "shade" | "threshold" | "w" | "h") {
            lcd::picture::redo(e);
        }
        self.put_elements(els, true)
    }

    /// Adds an element (Add's buttons): at the dot it was dropped on, or its usual place; picked up, so the arrows move it.
    fn add(&mut self, kind: &'static str) -> Task<Msg> {
        if kind == "picture" {
            let cmd = vec!["sh".into(), "-c".into(), "if command -v kdialog >/dev/null; then kdialog --title 'Pick a picture' --getopenfilename \"$HOME\" 'image/png image/jpeg'; elif command -v zenity >/dev/null; then zenity --file-selection --title='Pick a picture' --file-filter='*.png *.jpg *.jpeg'; else exit 3; fi".into()];
            return Task::perform(run::spawn(cmd), Msg::PictureFile);
        }
        let mut e = new_element(kind);
        if let Some((x, y)) = self.drop_at.take() {
            move_to(&mut e, x, y);
        }
        self.put_new(e)
    }
    fn put_new(&mut self, e: Value) -> Task<Msg> {
        let mut els = self.elements();
        els.push(e);
        let i = els.len() - 1;
        self.el = Some(i);
        self.holding = true;
        let t = self.put_elements(els, true);
        self.focus = view::ix(self, &format!("el{i}"));
        t
    }

    pub fn items(&self) -> usize {
        view::focusables(self).len()
    }
}

/// "Ctrl+C" as the mockups say it ("Copy"), for the common ones; otherwise the keys.
pub fn key_words(k: &str) -> String {
    match k {
        "Ctrl+C" => "Copy".into(),
        "Ctrl+V" => "Paste".into(),
        "Ctrl+X" => "Cut".into(),
        "Ctrl+Z" => "Undo".into(),
        "Ctrl+Shift+Z" | "Ctrl+Y" => "Redo".into(),
        "Ctrl+S" => "Save".into(),
        "VolumeUp" => "Volume up".into(),
        "VolumeDown" => "Volume down".into(),
        "Mute" => "Mute".into(),
        "PlayPause" => "Play / pause".into(),
        _ => k.to_string(),
    }
}

/// The apps in the menu: (desktop id, name), sorted by name.
pub fn apps_list() -> Vec<(String, String)> {
    let home = root::home();
    let data = std::env::var_os("XDG_DATA_HOME").map(std::path::PathBuf::from).unwrap_or_else(|| home.join(".local/share"));
    let mut dirs = vec![data.join("applications"), data.join("flatpak/exports/share/applications")];
    let sys = std::env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".into());
    dirs.extend(sys.split(':').filter(|d| !d.is_empty()).map(|d| std::path::Path::new(d).join("applications")));
    dirs.push("/var/lib/flatpak/exports/share/applications".into());
    let mut out: Vec<(String, String)> = Vec::new();
    for d in dirs {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.filter_map(|e| e.ok()) {
            let id = e.file_name().to_string_lossy().to_string();
            if !id.ends_with(".desktop") || out.iter().any(|(i, _)| *i == id) {
                continue;
            }
            let Ok(t) = std::fs::read_to_string(e.path()) else { continue };
            let entry = t.split("\n[").next().unwrap_or("");
            if entry.lines().any(|l| l.trim() == "NoDisplay=true" || l.trim() == "Hidden=true") || !entry.lines().any(|l| l.trim() == "Type=Application") {
                continue;
            }
            if let Some(n) = entry.lines().find_map(|l| l.strip_prefix("Name=")) {
                out.push((id, n.trim().to_string()));
            }
        }
    }
    out.sort_by(|a, b| a.1.to_lowercase().cmp(&b.1.to_lowercase()));
    out
}

pub fn app_name(desktop: &str) -> Option<String> {
    let p = actions::find_desktop(desktop)?;
    let t = std::fs::read_to_string(p).ok()?;
    t.lines().find_map(|l| l.strip_prefix("Name=")).map(|n| n.trim().to_string())
}

/// A key pressed in the app as a G15 combination ("Ctrl+Shift+T"); None for a modifier on its own.
fn combo_of(key: &Key, m: Modifiers) -> Option<String> {
    let k = match key.as_ref() {
        Key::Character(c) => {
            let c = c.to_string();
            let up = c.to_uppercase();
            let named = match c.as_str() {
                " " => "Space".to_string(),
                _ => up,
            };
            hearth_g15::keys::code_of(&named)?;
            named
        }
        Key::Named(n) => match n {
            Named::Enter => "Enter",
            Named::Escape => "Esc",
            Named::Tab => "Tab",
            Named::Space => "Space",
            Named::Backspace => "Backspace",
            Named::Delete => "Delete",
            Named::Insert => "Insert",
            Named::Home => "Home",
            Named::End => "End",
            Named::PageUp => "PageUp",
            Named::PageDown => "PageDown",
            Named::ArrowUp => "Up",
            Named::ArrowDown => "Down",
            Named::ArrowLeft => "Left",
            Named::ArrowRight => "Right",
            Named::PrintScreen => "Print",
            Named::AudioVolumeUp => "VolumeUp",
            Named::AudioVolumeDown => "VolumeDown",
            Named::AudioVolumeMute => "Mute",
            Named::MediaPlayPause => "PlayPause",
            Named::MediaTrackNext => "Next",
            Named::MediaTrackPrevious => "Previous",
            Named::F1 => "F1",
            Named::F2 => "F2",
            Named::F3 => "F3",
            Named::F4 => "F4",
            Named::F5 => "F5",
            Named::F6 => "F6",
            Named::F7 => "F7",
            Named::F8 => "F8",
            Named::F9 => "F9",
            Named::F10 => "F10",
            Named::F11 => "F11",
            Named::F12 => "F12",
            _ => return None,
        }
        .to_string(),
        _ => return None,
    };
    let mut parts = Vec::new();
    if m.control() {
        parts.push("Ctrl");
    }
    if m.alt() {
        parts.push("Alt");
    }
    if m.shift() {
        parts.push("Shift");
    }
    if m.logo() {
        parts.push("Super");
    }
    let mut out: Vec<String> = parts.into_iter().map(String::from).collect();
    out.push(k);
    Some(out.join("+"))
}

fn write_runtime(name: &str, v: &Value) {
    let d = root::runtime_dir();
    let _ = std::fs::create_dir_all(&d);
    let tmp = d.join(format!(".{name}.tmp"));
    if std::fs::write(&tmp, v.to_string()).is_ok() {
        let _ = std::fs::rename(&tmp, d.join(name));
    }
}

/// Where an element is, in dots (for picking it with the pointer and the outline).
pub fn bounds(e: &Value) -> (i32, i32, i32, i32) {
    let n = |k: &str, d: i64| e.get(k).and_then(|x| x.as_i64()).unwrap_or(d) as i32;
    let (x, y) = (n("x", 0), n("y", 0));
    let z = n("size", 1).clamp(1, 6);
    match s(e, "type").as_str() {
        "text" => {
            let w = n("w", 0);
            let t = s(e, "text");
            (x, y, if w > 0 { w } else { lcd::font::width(&t.replace(|c| c == '{' || c == '}', ""), z).clamp(6, 160).min(lcd::font::width(&t, z).max(6)) }, 7 * z)
        }
        "value" => (x, y, lcd::font::width(&format!("{}66°", s(e, "label")), z), 7 * z),
        "clock" => (x, y, lcd::font::width(if s(e, "format") == "seconds" { "00:00:00" } else { "00:00" }, z), 7 * z),
        "line" => {
            let (x2, y2) = (n("x2", x as i64), n("y2", y as i64));
            (x.min(x2), y.min(y2), (x2 - x).abs().max(1), (y2 - y).abs().max(1))
        }
        "tabs" => (0, 34, 160, 9),
        _ => (x, y, n("w", 20), n("h", 6)),
    }
}

/// A value's name as the editor says it (the mockup's "GPU temperature", "CPU use").
pub fn value_name(id: &str) -> String {
    hearth_g15::values::VALUES.iter().find(|(k, _, _)| *k == id).map(|(_, l, _)| l.replace("Graphics card", "GPU")).unwrap_or_default()
}

pub fn el_name(e: &Value) -> String {
    let what = value_name(&s(e, "value"));
    match s(e, "type").as_str() {
        "text" => format!("Text \"{}\"", s(e, "text").chars().take(18).collect::<String>()),
        "value" => format!("Value · {what}{}", if e.get("size").and_then(|x| x.as_i64()).unwrap_or(1) >= 3 { ", big" } else { "" }),
        "bar" => format!("Bar · {what}"),
        "graph" => format!("Graph · {what}"),
        "clock" => format!("Clock · {}", if e.get("size").and_then(|x| x.as_i64()).unwrap_or(1) >= 3 { "big" } else { "small" }),
        "picture" => "Picture".into(),
        "line" => "Line".into(),
        "box" => "Box".into(),
        "tabs" => "Page names".into(),
        t => t.to_string(),
    }
}

fn new_element(kind: &str) -> Value {
    match kind {
        "text" => json!({"type": "text", "x": 2, "y": 2, "size": 1, "text": "Text"}),
        "value" => json!({"type": "value", "x": 2, "y": 2, "size": 2, "value": "cpu.temp"}),
        "bar" => json!({"type": "bar", "x": 2, "y": 30, "w": 70, "h": 6, "value": "cpu.use", "min": 0, "max": 100}),
        "graph" => json!({"type": "graph", "x": 86, "y": 2, "w": 74, "h": 30, "value": "gpu.temp", "span": 300, "min": 30, "max": 90, "style": "line"}),
        "clock" => json!({"type": "clock", "x": 2, "y": 34, "size": 1, "format": "24h"}),
        "line" => json!({"type": "line", "x": 0, "y": 33, "x2": 159, "y2": 33}),
        "box" => json!({"type": "box", "x": 2, "y": 2, "w": 40, "h": 12, "fill": false}),
        _ => json!({"type": "text", "x": 2, "y": 2, "text": "?"}),
    }
}

fn gi(e: &Value, k: &str) -> i32 {
    e.get(k).and_then(|x| x.as_i64()).unwrap_or(0) as i32
}

/// What the pointer at dot (dx, dy) is on: the picked element's corner (to resize it), the picked element, or else
/// the top-most element under it.
pub fn hit(els: &[Value], picked: Option<usize>, dx: i32, dy: i32) -> Option<(usize, bool)> {
    let inside = |e: &Value| {
        let (x, y, w, h) = bounds(e);
        dx >= x - 1 && dx <= x + w && dy >= y - 1 && dy <= y + h
    };
    if let Some(e) = picked.and_then(|i| els.get(i)) {
        let (x, y, w, h) = bounds(e);
        if s(e, "type") != "tabs" && (dx - (x + w)).abs() <= 2 && (dy - (y + h)).abs() <= 2 {
            return picked.map(|i| (i, true));
        }
        if inside(e) {
            return picked.map(|i| (i, false));
        }
    }
    els.iter().enumerate().rev().find(|(_, e)| inside(e)).map(|(i, _)| (i, false))
}

/// Moves an element so its top-left is at (x, y) (a line keeps its length).
pub fn move_to(e: &mut Value, x: i32, y: i32) {
    if s(e, "type") == "tabs" {
        return;
    }
    let (nx, ny) = (x.clamp(-20, 159), y.clamp(-6, 42));
    let (ox, oy) = (gi(e, "x"), gi(e, "y"));
    if s(e, "type") == "line" {
        let (x2, y2) = (e.get("x2").and_then(|v| v.as_i64()).unwrap_or(ox as i64), e.get("y2").and_then(|v| v.as_i64()).unwrap_or(oy as i64));
        e["x2"] = json!(x2 + (nx - ox) as i64);
        e["y2"] = json!(y2 + (ny - oy) as i64);
    }
    e["x"] = json!(nx);
    e["y"] = json!(ny);
}

/// Drags an element's bottom-right corner to dot (cx, cy): boxes, bars, graphs and pictures change size; text, values
/// and clocks change their letters' size (7 dots a step); a line's end follows.
pub fn resize_to(e: &mut Value, cx: i32, cy: i32) {
    let (x, y, _, _) = bounds(e);
    match s(e, "type").as_str() {
        "tabs" => {}
        "line" => {
            e["x2"] = json!(cx.clamp(0, 159));
            e["y2"] = json!(cy.clamp(0, 42));
        }
        "text" | "value" | "clock" => {
            e["size"] = json!(((cy - y) as f32 / 7.0).round().clamp(1.0, 6.0) as i64);
            if s(e, "type") == "text" && e.get("w").and_then(|v| v.as_i64()).unwrap_or(0) > 0 {
                e["w"] = json!((cx - x).clamp(4, 160));
            }
        }
        _ => {
            e["w"] = json!((cx - x).clamp(2, (160 - x.max(0)).max(2)));
            e["h"] = json!((cy - y).clamp(2, (43 - y.max(0)).max(2)));
        }
    }
}

fn page_index(p: Page) -> usize {
    PAGES.iter().position(|x| *x == p).unwrap_or(0)
}

fn update(a: &mut App, m: Msg) -> Task<Msg> {
    match m {
        Msg::Theme => a.p = hearth_ui::theme::load(),
        Msg::Tick => {
            // Light at rest: the pictures are drawn again only when something changed (the live page has its own
            // once-a-second tick).
            let status = service::status(&a.root);
            let cfg = config::load();
            let mut changed = status != a.status;
            a.status = status;
            if cfg != a.cfg && !cfg.as_object().map(|o| o.is_empty()).unwrap_or(true) {
                a.cfg = cfg;
                a.prev.reload_with(a.cfg.clone());
                changed = true;
            }
            if changed {
                a.refresh_pictures();
            }
        }
        Msg::Live => a.refresh_pictures(),
        Msg::Saved(r) => {
            if let Err(e) = r {
                a.note = format!("Couldn't save that: {e}");
            }
        }
        Msg::Drag => return win::drag(),
        Msg::Maximize => return win::toggle_maximize(),
        Msg::Minimize => return win::minimize(),
        Msg::Close => return win::close(),
        Msg::Resize(d) => return win::resize(d),
        Msg::Go(p) => {
            a.page = p;
            // The controller starts on the page's main thing (the mockups: the key set in use, the picked G-key,
            // the first page).
            let start = match p {
                Page::Overview => format!("set{}", config::set_in_use(&a.cfg)),
                Page::GKeys => format!("g{}", a.g),
                Page::Screen => "page-clock".into(),
                _ => "kbd".into(),
            };
            a.focus = view::focusables(a).iter().position(|i| i.id == start).unwrap_or(0);
            a.dlg = None;
            a.note.clear();
            a.refresh_pictures();
        }
        Msg::Focus(i) => a.focus = i,
        Msg::Key(ev) => return key(a, ev),
        Msg::Flag(k, v) => return a.set1(&[k], Some(json!(v))),
        Msg::Level(k, v) => return a.set1(&[k], Some(json!(v))),
        Msg::Num(k, v) => return a.set1(&[k], Some(json!(v))),
        Msg::UseSet(n) => return a.set1(&["set"], Some(json!(n))),
        Msg::ViewSet(n) => {
            a.set = n;
            a.load_key();
        }
        Msg::PickG(g) => {
            if a.kind() == "text" {
                let t = a.text.clone();
                let _ = a.save_text(t);
            }
            a.g = g;
            a.load_key();
        }
        Msg::Kind(k) => {
            a.capturing = false;
            return match k {
                "none" => a.set_action(None),
                "keys" => {
                    a.capturing = true;
                    a.note = "Press the key or keys on your keyboard".into();
                    Task::none()
                }
                "text" => {
                    a.text.clear();
                    a.set_action(Some(json!({"kind": "text", "text": ""})))
                }
                "macro" => {
                    let t = a.record_start();
                    t
                }
                "app" => {
                    a.dlg = Some(Dlg::Apps(apps_list()));
                    a.dlg_focus = 0;
                    Task::none()
                }
                _ => {
                    a.dlg = Some(Dlg::Hearth);
                    a.dlg_focus = 0;
                    Task::none()
                }
            };
        }
        Msg::Change => {
            return match a.kind().as_str() {
                "keys" => {
                    a.capturing = true;
                    a.note = "Press the key or keys on your keyboard".into();
                    Task::none()
                }
                "app" => {
                    a.dlg = Some(Dlg::Apps(apps_list()));
                    Task::none()
                }
                "macro" => a.record_start(),
                _ => {
                    a.dlg = Some(Dlg::Hearth);
                    Task::none()
                }
            }
        }
        Msg::CaptureKeys => a.capturing = true,
        Msg::TestIt => {
            if let Some(v) = a.action(a.set, a.g) {
                if let Some(act) = actions::parse(&v) {
                    let opts = actions::macro_opts(&v);
                    let dev = a.root.dev.clone();
                    let _ = std::thread::Builder::new().name("hearth-g15-test".into()).spawn(move || {
                        // A moment for you to put the cursor where it should type.
                        std::thread::sleep(std::time::Duration::from_millis(400));
                        actions::run(&act);
                        if let Ok(mut u) = hearth_g15::input::Uinput::new(&dev) {
                            let act = match act {
                                actions::Action::Macro(st) => actions::Action::Macro(actions::with_waits(&st, opts.wait)),
                                o => o,
                            };
                            actions::play(&act, &mut u, &mut |ms| actions::sleep_ms(ms));
                        }
                    });
                }
            }
        }
        Msg::ClearKey => return a.set_action(None),
        Msg::Text(t) => a.text = t,
        Msg::TextSave => {
            let t = a.text.clone();
            return a.save_text(t);
        }
        Msg::HearthAction(h) => {
            a.dlg = None;
            let label = actions::HEARTH_ACTIONS.iter().find(|(k, _)| *k == h).map(|(_, l)| *l).unwrap_or("");
            return a.set_action(Some(json!({"kind": "hearth", "action": h, "label": label})));
        }
        Msg::AppPicked(id, name) => {
            a.dlg = None;
            return a.set_action(Some(json!({"kind": "app", "desktop": id, "label": format!("Open {name}")})));
        }
        Msg::Record => return a.record_start(),
        Msg::StopRecord => {
            a.recording = false;
            a.note = "Kept.".into();
            write_runtime("g15-record.json", &json!({"stop": true}));
        }
        Msg::MacroOpt(which) => {
            let Some(mut v) = a.macro_key().and_then(|g| a.action(a.set, g)) else { return Task::none() };
            let g = a.macro_key().unwrap_or(a.g);
            match which {
                "wait" => {
                    let next = match v.get("wait") {
                        None => json!("none"),
                        Some(Value::String(x)) if x == "none" => json!(100),
                        Some(Value::Number(n)) if n.as_u64() == Some(100) => json!(500),
                        _ => Value::Null,
                    };
                    if let Some(o) = v.as_object_mut() {
                        if next.is_null() {
                            o.remove("wait");
                        } else {
                            o.insert("wait".into(), next);
                        }
                    }
                }
                "repeat" => {
                    let cur = v.get("repeat").and_then(|x| x.as_u64()).unwrap_or(1);
                    let next = match cur {
                        1 => 2,
                        2 => 3,
                        3 => 5,
                        5 => 10,
                        _ => 1,
                    };
                    v["repeat"] = json!(next);
                }
                _ => {
                    let rep = v.get("whileHeld").and_then(|x| x.as_str()) == Some("repeat");
                    v["whileHeld"] = json!(if rep { "once" } else { "repeat" });
                }
            }
            let set = a.set;
            return a.set1(&["keys", &format!("m{set}"), &format!("g{g}")], Some(v));
        }
        Msg::RenameOpen => {
            a.dlg = Some(Dlg::Rename(a.cfg.get("names").and_then(|x| x.get(format!("m{}", a.set))).and_then(|v| v.as_str()).unwrap_or("").to_string()));
        }
        Msg::RenameText(t) => {
            if let Some(Dlg::Rename(x)) = &mut a.dlg {
                *x = t.chars().take(24).collect();
            }
        }
        Msg::RenameSave => {
            if let Some(Dlg::Rename(x)) = a.dlg.take() {
                let set = a.set;
                let v = x.trim().to_string();
                return a.set1(&["names", &format!("m{set}")], if v.is_empty() { None } else { Some(json!(v)) });
            }
        }
        Msg::GamesOpen => a.dlg = Some(Dlg::Games(a.games())),
        Msg::GameText(i, t) => {
            if let Some(Dlg::Games(v)) = &mut a.dlg {
                if let Some(x) = v.get_mut(i) {
                    *x = t.chars().take(60).collect();
                }
            }
        }
        Msg::GamesSave => {
            if let Some(Dlg::Games(v)) = a.dlg.take() {
                let rules: Vec<Value> = v.iter().enumerate().filter(|(_, g)| !g.trim().is_empty()).map(|(i, g)| json!({"game": g.trim(), "set": i + 1})).collect();
                return a.set1(&["gameSets"], if rules.is_empty() { None } else { Some(Value::Array(rules)) });
            }
        }
        Msg::CloseDlg => a.dlg = None,
        Msg::Picked(what, v) => {
            if v == "@open" {
                a.dlg = Some(Dlg::Pick(what, view::choices(a, what)));
                a.dlg_focus = 0;
                return Task::none();
            }
            a.dlg = None;
            let val = if what == "span" || what == "min" || what == "max" || what == "size" { v.parse::<i64>().map(|n| json!(n)).unwrap_or(json!(v)) } else { json!(v) };
            if what == "range" {
                let (lo, hi) = v.split_once('-').unwrap_or(("0", "100"));
                let lo: i64 = lo.parse().unwrap_or(0);
                let hi: i64 = hi.parse().unwrap_or(100);
                let Some(i) = a.el else { return Task::none() };
                let mut els = a.elements();
                if let Some(e) = els.get_mut(i) {
                    e["min"] = json!(lo);
                    e["max"] = json!(hi);
                }
                return a.put_elements(els, true);
            }
            if what == "show" {
                return a.screen_field("show", json!(v));
            }
            if what == "while" {
                if v == "@other" {
                    a.while_typing = true;
                    return Task::none();
                }
                a.while_typing = false;
                return a.screen_field("whileRunning", json!(v));
            }
            if what == "idle" {
                return a.set1(&["lcdIdleOff"], Some(json!(v.parse::<u64>().unwrap_or(0))));
            }
            return a.set_el(what, val);
        }
        Msg::TogglePage(id) => {
            let mut on = a.pages_on();
            if on.contains(&id) {
                if on.len() > 1 {
                    on.retain(|x| *x != id);
                }
            } else {
                on.push(id.clone());
            }
            let t = a.set1(&["pages"], Some(json!(on)));
            // Your own screens: "show" follows the switch too.
            if !hearth_g15::lcd::screen::BUILT_IN.contains(&id.as_str()) {
                let mut all = a.own();
                if let Some(x) = all.iter_mut().find(|x| s(x, "id") == id) {
                    x["show"] = json!(if a.pages_on().contains(&id) { "list" } else { "hidden" });
                }
                let t2 = a.set1(&["screens"], Some(Value::Array(all)));
                a.refresh_pictures();
                return Task::batch([t, t2]);
            }
            a.refresh_pictures();
            return t;
        }
        Msg::MovePage(id, d) => {
            let mut on = a.all_pages();
            if let Some(i) = on.iter().position(|x| *x == id) {
                let j = (i as i32 + d).clamp(0, on.len() as i32 - 1) as usize;
                on.swap(i, j);
                let keep: Vec<String> = on.into_iter().filter(|x| a.pages_on().contains(x)).collect();
                return a.set1(&["pages"], Some(json!(keep)));
            }
        }
        Msg::DragPage(id) => a.dragging = Some(id),
        Msg::OverPage(id) => {
            if a.dragging.is_some() {
                a.over = Some(id)
            }
        }
        Msg::DropPage => {
            if let (Some(from), Some(to)) = (a.dragging.take(), a.over.take()) {
                if from != to {
                    let mut all = a.all_pages();
                    if let (Some(i), Some(j)) = (all.iter().position(|x| *x == from), all.iter().position(|x| *x == to)) {
                        let x = all.remove(i);
                        all.insert(j, x);
                        let keep: Vec<String> = all.into_iter().filter(|x| a.pages_on().contains(x)).collect();
                        return a.set1(&["pages"], Some(json!(keep)));
                    }
                }
            }
            a.dragging = None;
            a.over = None;
        }
        Msg::Edit(id) => {
            a.screen = id;
            a.page = Page::Editor;
            a.el = if a.elements().is_empty() { None } else { Some(0) };
            a.undo.clear();
            a.holding = false;
            a.while_typing = false;
            a.focus = if a.el.is_some() { view::ix(a, "el0") } else { 0 };
            a.refresh_pictures();
        }
        Msg::NewScreen => {
            let n = a.own().len() + 1;
            let id = format!("screen-{}", hearth_ui::time::now());
            a.screen = id.clone();
            a.page = Page::Editor;
            a.undo.clear();
            let scr = json!({"id": id, "name": format!("My screen {n}"), "show": "list", "elements": [
                {"type": "text", "x": 1, "y": 1, "text": "CPU"}, {"type": "value", "x": 1, "y": 10, "size": 3, "value": "cpu.temp"}
            ]});
            let t = a.put_screen(scr, false);
            let mut on = a.pages_on();
            on.push(id);
            let t2 = a.set1(&["pages"], Some(json!(on)));
            a.el = Some(0);
            a.holding = false;
            a.while_typing = false;
            a.focus = view::ix(a, "el0");
            a.refresh_pictures();
            return Task::batch([t, t2]);
        }
        Msg::Show => {
            let els = if a.page == Page::Editor { a.elements() } else { a.pages_on().first().map(|p| hearth_g15::lcd::screen::elements_of(p, &a.own())).unwrap_or_default() };
            write_runtime("g15-show.json", &json!({"elements": els}));
            a.note = if a.plugged() { "On the keyboard's screen for 30 seconds.".into() } else { "Plug the keyboard in to see it there.".into() };
        }
        Msg::Add(kind) => {
            a.adding = None;
            a.drop_at = None;
            return a.add(kind);
        }
        Msg::AddGrab(kind) => a.adding = Some(kind),
        Msg::AddDrop(pt) => {
            let Some(kind) = a.adding.take() else { return Task::none() };
            let (ox, oy, sc) = pics::screen_map(view::EDITOR_SIZE, true);
            a.drop_at = Some((((pt.x - ox) / sc).floor() as i32, ((pt.y - oy) / sc).floor() as i32));
            return a.add(kind);
        }
        Msg::AddEnd => {
            if let Some(kind) = a.adding.take() {
                a.drop_at = None;
                return a.add(kind);
            }
        }
        Msg::PictureFile(o) => {
            let path = o.out.trim().to_string();
            let (x, y) = a.drop_at.take().unwrap_or((0, 0));
            if o.code == 3 {
                a.note = "No file picker is installed.".into();
                return Task::none();
            }
            if path.is_empty() {
                return Task::none();
            }
            match lcd::picture::element(&path, x, y) {
                Ok(e) => return a.put_new(e),
                Err(e) => a.note = format!("Couldn't use that picture: {e}"),
            }
        }
        Msg::PickEl(i) => {
            // A: pick it up; A again: put it down.
            if a.el == Some(i) && a.holding {
                a.holding = false;
            } else {
                a.el = Some(i);
                a.holding = true;
            }
            a.focus = view::ix(a, &format!("el{i}"));
        }
        Msg::Press(pt) => {
            let (ox, oy, sc) = pics::screen_map(view::EDITOR_SIZE, true);
            let (dx, dy) = (((pt.x - ox) / sc).floor() as i32, ((pt.y - oy) / sc).floor() as i32);
            let els = a.elements();
            a.adding = None;
            match hit(&els, a.el, dx, dy) {
                Some((i, corner)) => {
                    let e = &els[i];
                    a.grab = Some(if corner { (0, 0, true) } else { (dx - gi(e, "x"), dy - gi(e, "y"), false) });
                    a.el = Some(i);
                    a.holding = true;
                    a.grab_before = Some(Value::Array(a.own()));
                    a.focus = view::ix(a, &format!("el{i}"));
                }
                None => {
                    a.grab = None;
                    a.holding = false;
                }
            }
        }
        Msg::Move(pt) => {
            let Some((gx, gy, resize)) = a.grab else { return Task::none() };
            let Some(i) = a.el else { return Task::none() };
            let (ox, oy, sc) = pics::screen_map(view::EDITOR_SIZE, true);
            let (dx, dy) = (((pt.x - ox) / sc).floor() as i32, ((pt.y - oy) / sc).floor() as i32);
            let mut els = a.elements();
            let Some(e) = els.get_mut(i) else { return Task::none() };
            if resize {
                resize_to(e, dx, dy);
            } else {
                move_to(e, dx - gx, dy - gy);
            }
            // While dragging: the picture follows; it's saved when you let go.
            if let Some(mut scr) = a.cur_screen() {
                scr["elements"] = Value::Array(els.clone());
                let mut all = a.own();
                if let Some(j) = all.iter().position(|x| s(x, "id") == a.screen) {
                    all[j] = scr;
                }
                if let Some(o) = a.cfg.as_object_mut() {
                    o.insert("screens".into(), Value::Array(all));
                }
                a.editor_frame = Some(a.prev.elements(&els));
            }
        }
        Msg::Release => {
            if a.grab.take().is_some() {
                let mut all = a.own();
                // A picture let go at a new size: its dots made again from the picture.
                if let Some(e) = a.el.and_then(|i| all.iter_mut().find(|x| s(x, "id") == a.screen).and_then(|x| x.get_mut("elements")).and_then(|v| v.as_array_mut()).and_then(|v| v.get_mut(i))) {
                    if s(e, "type") == "picture" {
                        lcd::picture::redo(e);
                    }
                }
                let before = a.grab_before.take();
                // Only clicked (nothing moved): nothing to save.
                if before.as_ref() == Some(&Value::Array(all.clone())) {
                    return Task::none();
                }
                if let Some(b) = before {
                    a.undo.push(b);
                }
                let t = a.set1(&["screens"], Some(Value::Array(all)));
                a.refresh_pictures();
                return t;
            }
        }
        Msg::Nudge(dx, dy) => {
            let Some(i) = a.el else { return Task::none() };
            let mut els = a.elements();
            if let Some(e) = els.get_mut(i) {
                for (k, d) in [("x", dx), ("y", dy), ("x2", dx), ("y2", dy)] {
                    if let Some(v) = e.get(k).and_then(|v| v.as_i64()) {
                        e[k] = json!(v + d as i64);
                    }
                }
            }
            return a.put_elements(els, true);
        }
        Msg::ElSet(k, v) => return a.set_el(k, v),
        Msg::ElText(t) => {
            let Some(i) = a.el else { return Task::none() };
            let mut els = a.elements();
            if let Some(e) = els.get_mut(i) {
                e["text"] = json!(t.chars().take(60).collect::<String>());
            }
            return a.put_elements(els, false);
        }
        Msg::ScreenName(t) => return a.screen_field("name", json!(t.chars().take(30).collect::<String>())),
        Msg::ScreenField(k, v) => return a.screen_field(k, v),
        Msg::WhileText(t) => return a.screen_field("whileRunning", json!(t.chars().take(60).collect::<String>())),
        Msg::WhileDone => a.while_typing = false,
        Msg::Undo => {
            if let Some(prev) = a.undo.pop() {
                let t = a.set1(&["screens"], Some(prev));
                a.refresh_pictures();
                return t;
            }
        }
        Msg::Done => {
            a.holding = false;
            a.while_typing = false;
            a.page = Page::Screen;
            a.focus = 0;
            a.refresh_pictures();
        }
        Msg::Delete => {
            let Some(i) = a.el else { return Task::none() };
            let mut els = a.elements();
            if i < els.len() {
                els.remove(i);
            }
            a.el = if els.is_empty() { None } else { Some(i.min(els.len() - 1)) };
            a.holding = false;
            let t = a.put_elements(els, true);
            a.focus = a.focus.min(view::focusables(a).len().saturating_sub(1));
            return t;
        }
    }
    Task::none()
}

impl App {
    fn save_text(&mut self, t: String) -> Task<Msg> {
        if self.kind() != "text" {
            return Task::none();
        }
        let short: String = t.chars().take(24).collect();
        self.set_action(Some(json!({"kind": "text", "text": t, "label": format!("Type: {short}")})))
    }
    fn screen_field(&mut self, k: &str, v: Value) -> Task<Msg> {
        let Some(mut scr) = self.cur_screen() else { return Task::none() };
        scr[k] = v;
        self.put_screen(scr, false)
    }
    /// The key whose macro the macro card shows: the picked one when it's a macro, else the set's first macro.
    pub fn macro_key(&self) -> Option<u8> {
        let is_macro = |g: u8| self.action(self.set, g).map(|v| s(&v, "kind") == "macro").unwrap_or(false);
        if is_macro(self.g) {
            return Some(self.g);
        }
        (1..=18).find(|&g| is_macro(g))
    }
    fn record_start(&mut self) -> Task<Msg> {
        if !self.plugged() {
            self.note = "Plug the keyboard in to record on it.".into();
            return Task::none();
        }
        self.recording = true;
        self.note = format!("Recording for G{}: type on the G15, then press MR (or Stop) to keep it.", self.g);
        write_runtime("g15-record.json", &json!({"g": self.g}));
        Task::none()
    }
}

fn key(a: &mut App, ev: KEvent) -> Task<Msg> {
    let KEvent::KeyPressed { key, modifiers, .. } = ev else { return Task::none() };
    // Keys: the next combination pressed is what the G-key sends.
    if a.capturing {
        if let Key::Named(Named::Escape) = key.as_ref() {
            a.capturing = false;
            a.note.clear();
            return Task::none();
        }
        if let Some(c) = combo_of(&key, modifiers) {
            a.capturing = false;
            a.note.clear();
            let words = key_words(&c);
            return a.set_action(Some(json!({"kind": "keys", "keys": c, "label": words})));
        }
        return Task::none();
    }
    // Typing in a text field: leave the keys to it.
    let text_field = (a.page == Page::GKeys && a.kind() == "text") || a.page == Page::Editor;
    let ctrl = modifiers.control();
    match key.as_ref() {
        Key::Named(Named::PageUp) if ctrl => {
            let i = page_index(a.page);
            return update(a, Msg::Go(PAGES[(i + PAGES.len() - 1) % PAGES.len()]));
        }
        Key::Named(Named::PageDown) if ctrl => {
            let i = page_index(a.page);
            return update(a, Msg::Go(PAGES[(i + 1) % PAGES.len()]));
        }
        Key::Named(Named::Escape) => {
            if a.dlg.is_some() {
                a.dlg = None;
            } else if a.page == Page::Editor && a.while_typing {
                a.while_typing = false;
            } else if a.page == Page::Editor && a.holding {
                // B puts it down first; B again goes back.
                a.holding = false;
            } else if a.page == Page::Editor {
                return update(a, Msg::Done);
            } else if a.recording {
                a.recording = false;
                write_runtime("g15-record.json", &json!({"cancel": true}));
            } else {
                return win::close();
            }
            return Task::none();
        }
        _ => {}
    }
    if let Some(d) = &a.dlg {
        let n = match d {
            Dlg::Hearth => actions::HEARTH_ACTIONS.len(),
            Dlg::Apps(v) => v.len(),
            Dlg::Pick(_, v) => v.len(),
            _ => 0,
        };
        match key.as_ref() {
            Key::Named(Named::ArrowDown) => a.dlg_focus = (a.dlg_focus + 1).min(n.saturating_sub(1)),
            Key::Named(Named::ArrowUp) => a.dlg_focus = a.dlg_focus.saturating_sub(1),
            Key::Named(Named::Enter) => {
                let m = match d {
                    Dlg::Hearth => actions::HEARTH_ACTIONS.get(a.dlg_focus).map(|(k, _)| Msg::HearthAction(*k)),
                    Dlg::Apps(v) => v.get(a.dlg_focus).map(|(i, n)| Msg::AppPicked(i.clone(), n.clone())),
                    Dlg::Pick(w, v) => v.get(a.dlg_focus).map(|(k, _)| Msg::Picked(*w, k.clone())),
                    Dlg::Rename(_) => Some(Msg::RenameSave),
                    Dlg::Games(_) => Some(Msg::GamesSave),
                };
                if let Some(m) = m {
                    return update(a, m);
                }
            }
            _ => {}
        }
        return Task::none();
    }
    // Picked up: the arrows move it one dot (Shift: five), A puts it down. Otherwise they move between the rows.
    if a.page == Page::Editor && a.el.is_some() && !modifiers.control() && matches!(key.as_ref(), Key::Named(Named::Delete)) {
        return update(a, Msg::Delete);
    }
    if a.page == Page::Editor && a.holding && a.el.is_some() && !modifiers.control() {
        let d = if modifiers.shift() { 5 } else { 1 };
        let on_it = view::focusables(a).get(a.focus).map(|it| it.id == format!("el{}", a.el.unwrap_or(0))).unwrap_or(false);
        match key.as_ref() {
            // A on the picked-up element (its chip) puts it down; A elsewhere does what that row does.
            Key::Named(Named::Enter) if on_it => {
                a.holding = false;
                return Task::none();
            }
            Key::Named(Named::ArrowLeft) => return update(a, Msg::Nudge(-d, 0)),
            Key::Named(Named::ArrowRight) => return update(a, Msg::Nudge(d, 0)),
            Key::Named(Named::ArrowUp) => return update(a, Msg::Nudge(0, -d)),
            Key::Named(Named::ArrowDown) => return update(a, Msg::Nudge(0, d)),
            _ => {}
        }
    }
    if text_field && matches!(key.as_ref(), Key::Character(_) | Key::Named(Named::Space) | Key::Named(Named::Backspace)) {
        return Task::none();
    }
    let items = view::focusables(a);
    let n = items.len();
    match key.as_ref() {
        Key::Named(Named::ArrowDown) | Key::Named(Named::Tab) => a.focus = (a.focus + 1).min(n.saturating_sub(1)),
        Key::Named(Named::ArrowUp) => a.focus = a.focus.saturating_sub(1),
        Key::Named(Named::ArrowLeft) => {
            if let Some(m) = items.get(a.focus).and_then(|i| i.left.clone()) {
                return update(a, m);
            }
        }
        Key::Named(Named::ArrowRight) => {
            if let Some(m) = items.get(a.focus).and_then(|i| i.right.clone()) {
                return update(a, m);
            }
        }
        Key::Named(Named::Enter) => {
            if let Some(m) = items.get(a.focus).and_then(|i| i.enter.clone()) {
                return update(a, m);
            }
        }
        // X on the controller (the Menu key) and Ctrl+D: the page's own X (Test it, Show on the keyboard, Delete).
        Key::Named(Named::ContextMenu) => {
            let m = view::x_action(a);
            return update(a, m);
        }
        Key::Character(c) if ctrl && c.eq_ignore_ascii_case("d") => {
            let m = view::x_action(a);
            return update(a, m);
        }
        Key::Character(c) if ctrl && c.eq_ignore_ascii_case("f") => {
            if let Some(m) = view::y_action(a) {
                return update(a, m);
            }
        }
        _ => {}
    }
    Task::none()
}

fn subscription(a: &App) -> Subscription<Msg> {
    let mut v = vec![
        Subscription::run(hearth_ui::theme::watch).map(|_| Msg::Theme),
        iced::keyboard::listen().map(Msg::Key),
        // Plugged in or not, and changes made with the keyboard's own keys (M1–M3, MR, the light button).
        iced::time::every(std::time::Duration::from_secs(10)).map(|_| Msg::Tick),
    ];
    // What's on the keyboard's screen now: once a second, only while a page showing it is open.
    if matches!(a.page, Page::Screen | Page::Editor) || a.recording {
        v.push(iced::time::every(std::time::Duration::from_secs(1)).map(|_| Msg::Live));
    }
    Subscription::batch(v)
}

fn main() -> iced::Result {
    if app::handle_cli("hearth-g15-app", "Logitech G15: G-keys and macros, the keyboard's screen, its lights.\n  --page overview|g-keys|screen|lighting|editor[:SCREEN]   the page to open\n  --key N  --set N   the G-key and key set the G-keys page opens at\n  --show hearth|apps|rename|games   a dialog (pictures)") {
        return Ok(());
    }
    iced::application(App::new, update, view::view)
        .title("Logitech G15")
        .subscription(subscription)
        .theme(|a: &App| app::iced_theme(&a.p))
        .window(app::window("hearth-g15-app", (1280.0, 800.0), (1000.0, 640.0)))
        .default_font(app::INTER)
        .run()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn dads() -> Vec<Value> {
        vec![
            json!({"type": "value", "x": 1, "y": 10, "size": 3, "value": "gpu.temp"}),
            json!({"type": "text", "x": 1, "y": 1, "text": "GPU"}),
            json!({"type": "graph", "x": 86, "y": 2, "w": 74, "h": 30, "value": "gpu.temp"}),
            json!({"type": "clock", "x": 1, "y": 34, "size": 1}),
            json!({"type": "bar", "x": 92, "y": 35, "w": 66, "h": 6, "value": "cpu.use"}),
        ]
    }
    #[test]
    fn picks_what_the_pointer_is_on() {
        let els = dads();
        assert_eq!(hit(&els, None, 100, 15), Some((2, false)), "the graph");
        assert_eq!(hit(&els, None, 5, 20), Some((0, false)), "the big value");
        assert_eq!(hit(&els, None, 2, 3), Some((1, false)), "the text over the value");
        assert_eq!(hit(&els, None, 70, 20), None, "nothing there");
        // The picked graph's bottom-right corner (86+74, 2+30) resizes it, a dot or two off too.
        assert_eq!(hit(&els, Some(2), 160, 32), Some((2, true)));
        assert_eq!(hit(&els, Some(2), 158, 31), Some((2, true)));
        assert_eq!(hit(&els, Some(2), 120, 10), Some((2, false)));
        // The picked one wins where two overlap.
        assert_eq!(hit(&els, Some(0), 2, 11), Some((0, false)));
    }
    #[test]
    fn moves_and_resizes() {
        let mut g = dads()[2].clone();
        resize_to(&mut g, 150, 40);
        assert_eq!((g["w"].as_i64(), g["h"].as_i64()), (Some(64), Some(38)));
        resize_to(&mut g, 400, 400);
        assert_eq!((g["w"].as_i64(), g["h"].as_i64()), (Some(74), Some(41)), "kept on the screen");
        resize_to(&mut g, 0, 0);
        assert_eq!((g["w"].as_i64(), g["h"].as_i64()), (Some(2), Some(2)));
        let mut v = dads()[0].clone();
        resize_to(&mut v, 30, 10 + 14);
        assert_eq!(v["size"].as_i64(), Some(2), "letters two steps tall");
        let mut l = json!({"type": "line", "x": 0, "y": 33, "x2": 159, "y2": 33});
        move_to(&mut l, 10, 20);
        assert_eq!((l["x"].as_i64(), l["y"].as_i64(), l["x2"].as_i64(), l["y2"].as_i64()), (Some(10), Some(20), Some(169), Some(20)));
        resize_to(&mut l, 50, 25);
        assert_eq!((l["x2"].as_i64(), l["y2"].as_i64()), (Some(50), Some(25)));
        let mut t = json!({"type": "tabs"});
        move_to(&mut t, 5, 5);
        assert!(t.get("x").is_none(), "the page names stay over the keys");
        assert_eq!(el_name(&dads()[2]), "Graph · GPU temperature");
        assert_eq!(el_name(&dads()[0]), "Value · GPU temperature, big");
    }
}
