//! What every page shares: the look, this PC, Hearth's own settings (desktop.json) and the change history, the line
//! at the top of the page (an error or a note), and where to go next.

use crate::data;
use hearth_ui::run;
use hearth_ui::Pal;
use serde_json::{Map, Value};

pub struct Shared {
    pub p: Pal,
    pub info: data::Info,
    /// ~/.local/share/nimbus/desktop.json as it is now (read only; changes go through `desktop_cmd`).
    pub desktop: Map<String, Value>,
    pub history: Vec<data::Change>,
    pub error: String,
    pub note: String,
    /// A page to open after this update (a module's "go to …").
    pub go: Option<&'static str>,
    /// Read the change history again after this update (a setting was changed).
    pub reread_history: bool,
    /// The part of the page asked for (`hearth-settings keyboard:osk`), for the page to scroll to or open.
    pub part: String,
    /// The controller look (layout B) is showing.
    pub big: bool,
    /// Your account picture's file (my_face), "" without one: drawn round in the corner (frame.rs).
    pub face: String,
}

/// Your account picture, where Accounts › Your account finds it (hearth-system users, the same order): ~/.face,
/// ~/.face.icon (KDE's), then AccountsService's /var/lib/AccountsService/icons/<you> (what Plasma's and the login
/// screen's picture is). "" when there's none readable.
pub fn my_face() -> String {
    let home = hearth_ui::paths::home();
    let user = std::env::var("USER").ok().filter(|u| !u.is_empty()).unwrap_or_else(|| home.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default());
    let mut c = vec![home.join(".face"), home.join(".face.icon")];
    if !user.is_empty() {
        c.push(std::path::PathBuf::from(std::env::var("HEARTH_ACCOUNTS_ICONS").unwrap_or_else(|_| "/var/lib/AccountsService/icons".into())).join(&user));
    }
    c.into_iter().find(|p| std::fs::File::open(p).map(|f| f.metadata().map(|m| m.is_file() && m.len() > 0).unwrap_or(false)).unwrap_or(false)).map(|p| p.to_string_lossy().into_owned()).unwrap_or_default()
}

pub fn io_helper() -> String {
    run::shell_helper("hearth_settings_io.py")
}

impl Shared {
    pub fn new() -> Shared {
        Shared {
            p: hearth_ui::theme::load(),
            info: data::Info::default(),
            desktop: read_desktop(),
            face: my_face(),
            history: Vec::new(),
            error: String::new(),
            note: String::new(),
            go: None,
            reread_history: false,
            part: String::new(),
            big: false,
        }
    }

    /// The command that changes Hearth's own settings in desktop.json: hearth_settings_io.py merges it in (atomic,
    /// .bak, never over a file that failed to read) and records it in the change history, for Undo. The copy here
    /// is changed at once so the page shows it.
    pub fn desktop_cmd(&mut self, changes: Value) -> Vec<String> {
        if let Value::Object(m) = &changes {
            for (k, v) in m {
                self.desktop.insert(k.clone(), v.clone());
            }
        }
        self.reread_history = true;
        let path = hearth_ui::theme::desktop_json().to_string_lossy().to_string();
        vec!["python3".into(), io_helper(), "merge".into(), path, changes.to_string(), "--history".into()]
    }

    /// The command that merges `changes` into another of Hearth's JSON settings files (firehold.json, taskmgr.json…).
    pub fn json_cmd(&mut self, path: &std::path::Path, changes: Value) -> Vec<String> {
        self.reread_history = true;
        vec!["python3".into(), io_helper(), "merge".into(), path.to_string_lossy().to_string(), changes.to_string(), "--history".into()]
    }

    /// The command that sets KEY=VALUE in a shell-style settings file (session.conf).
    pub fn conf_cmd(&mut self, path: &std::path::Path, key: &str, value: &str) -> Vec<String> {
        self.reread_history = true;
        vec!["python3".into(), io_helper(), "conf-set".into(), path.to_string_lossy().to_string(), key.into(), value.into()]
    }

    pub fn d_str(&self, k: &str) -> String {
        self.desktop.get(k).and_then(Value::as_str).unwrap_or("").to_string()
    }
    pub fn d_bool(&self, k: &str, default: bool) -> bool {
        self.desktop.get(k).and_then(Value::as_bool).unwrap_or(default)
    }
    pub fn d_num(&self, k: &str, default: f64) -> f64 {
        self.desktop.get(k).and_then(Value::as_f64).unwrap_or(default)
    }

    /// A change that ran: its reason at the top of the page when it failed.
    pub fn result(&mut self, what: &str, o: &run::Output) {
        if !o.ok {
            let why = o.why();
            self.error = if why.is_empty() { what.to_string() } else { format!("{what}: {why}") };
        }
    }
    pub fn go(&mut self, id: &'static str) {
        self.go = Some(id);
    }
}

pub fn read_desktop() -> Map<String, Value> {
    match hearth_ui::settings_file::read_json(&hearth_ui::theme::desktop_json()) {
        Value::Object(m) => m,
        _ => Map::new(),
    }
}

/// Whether this desktop is on KWin (KDE Plasma, or Mantel on KWin): KWin's own pages and rows (Window effects, Virtual
/// desktops, Tiling, Screen edges, Window behaviour, Window rules, Display's HDR on the desktop…) are shown only then.
/// KWin is retired and Mantel runs on labwc, so elsewhere they could never work (the owner, Nov 28: "every button in
/// the settings have to work").
pub fn kwin_desktop() -> bool {
    std::env::var("HEARTH_WM").map(|v| v == "kwin").unwrap_or(false) || std::env::var("XDG_CURRENT_DESKTOP").map(|v| v.to_uppercase().contains("KDE")).unwrap_or(false)
}

/// A KWin-only page (its id starts with kwin-), hidden from lists and search off KWin.
pub fn kwin_page(id: &str) -> bool {
    id.starts_with("kwin-")
}
