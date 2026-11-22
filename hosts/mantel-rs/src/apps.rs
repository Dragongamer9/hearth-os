//! The apps the menu and the dock show: every .desktop file in the XDG folders (yours first, then the system's), the
//! hidden ones left out, each with its icon found in the icon theme folders (hicolor and the look's set, PNG or SVG),
//! and how to open one (its Exec line without the %f / %u codes, started on its own so it outlives the menu).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub struct App {
    /// The .desktop file's name without ".desktop" ("org.kde.dolphin").
    pub id: String,
    pub name: String,
    pub exec: String,
    pub icon: String,
    pub icon_path: Option<PathBuf>,
    pub terminal: bool,
    pub keywords: String,
    pub wm_class: String,
}

fn data_dirs() -> Vec<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
    let mut v = vec![std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|| home.join(".local/share"))];
    let sys = std::env::var("XDG_DATA_DIRS").ok().filter(|s| !s.is_empty()).unwrap_or_else(|| "/usr/local/share:/usr/share".into());
    v.extend(sys.split(':').filter(|s| !s.is_empty()).map(PathBuf::from));
    v.push(home.join(".local/share/flatpak/exports/share"));
    v.push(PathBuf::from("/var/lib/flatpak/exports/share"));
    let mut seen = std::collections::HashSet::new();
    v.retain(|d| seen.insert(d.clone()));
    v
}

/// One .desktop file's [Desktop Entry] (None: hidden, not an app, or unreadable).
pub fn parse(id: &str, text: &str) -> Option<App> {
    let mut in_entry = false;
    let mut kv: HashMap<String, String> = HashMap::new();
    for line in text.lines() {
        let l = line.trim();
        if l.starts_with('[') {
            in_entry = l == "[Desktop Entry]";
            continue;
        }
        if !in_entry || l.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = l.split_once('=') {
            kv.entry(k.trim().to_string()).or_insert_with(|| v.trim().to_string());
        }
    }
    let yes = |k: &str| kv.get(k).map(|v| v.eq_ignore_ascii_case("true")).unwrap_or(false);
    if kv.get("Type").map(String::as_str).unwrap_or("Application") != "Application" || yes("NoDisplay") || yes("Hidden") {
        return None;
    }
    // Only for some desktops (OnlyShowIn without Hearth's labwc names) or not on them: left out, as Mantel does.
    let current = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_else(|_| "labwc:wlroots".into()).to_lowercase();
    let on = |list: &str| list.split(';').filter(|x| !x.is_empty()).any(|x| current.split(':').any(|c| c == x.to_lowercase()));
    if let Some(o) = kv.get("OnlyShowIn") {
        if !on(o) {
            return None;
        }
    }
    if let Some(n) = kv.get("NotShowIn") {
        if on(n) {
            return None;
        }
    }
    let name = kv.get("Name")?.clone();
    let exec = kv.get("Exec")?.clone();
    if name.is_empty() || exec.is_empty() {
        return None;
    }
    Some(App {
        id: id.to_string(),
        name,
        exec,
        icon: kv.get("Icon").cloned().unwrap_or_default(),
        icon_path: None,
        terminal: yes("Terminal"),
        keywords: kv.get("Keywords").cloned().unwrap_or_default(),
        wm_class: kv.get("StartupWMClass").cloned().unwrap_or_default(),
    })
}

/// Every app, by name (the first file found for an id wins: yours over the system's).
pub fn all() -> Vec<App> {
    let mut by_id: HashMap<String, App> = HashMap::new();
    let mut hidden = std::collections::HashSet::new();
    for d in data_dirs() {
        let dir = d.join("applications");
        let Ok(rd) = std::fs::read_dir(&dir) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|x| x.to_str()) != Some("desktop") {
                continue;
            }
            let id = p.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
            if id.is_empty() || by_id.contains_key(&id) || hidden.contains(&id) {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&p) else { continue };
            match parse(&id, &text) {
                Some(a) => {
                    by_id.insert(id, a);
                }
                None => {
                    hidden.insert(id);
                }
            }
        }
    }
    let mut v: Vec<App> = by_id.into_values().collect();
    for a in v.iter_mut() {
        a.icon_path = icon_path(&a.icon);
    }
    v.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    v
}

/// The app with this id (a pin's), if it's installed.
pub fn find<'a>(apps: &'a [App], id: &str) -> Option<&'a App> {
    let id = id.trim_end_matches(".desktop");
    apps.iter().find(|a| a.id == id)
}

/// The app a window belongs to: its app id against the .desktop id, the last part of it, or StartupWMClass.
pub fn for_window<'a>(apps: &'a [App], app_id: &str) -> Option<&'a App> {
    if app_id.is_empty() {
        return None;
    }
    let low = app_id.to_lowercase();
    apps.iter()
        .find(|a| a.id.to_lowercase() == low)
        .or_else(|| apps.iter().find(|a| !a.wm_class.is_empty() && a.wm_class.to_lowercase() == low))
        .or_else(|| apps.iter().find(|a| a.id.rsplit('.').next().map(|s| s.to_lowercase() == low).unwrap_or(false)))
}

fn icon_roots() -> Vec<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
    let mut roots = vec![home.join(".icons"), home.join(".local/share/icons")];
    for d in data_dirs() {
        roots.push(d.join("icons"));
    }
    roots
}

/// The icon theme in use (as Qt picks it: qt6ct's, else KDE's kdeglobals, else GTK's), then what it inherits, then
/// hicolor; Adwaita behind them (Fedora's Qt finds it there when nothing is set).
pub fn icon_themes() -> &'static Vec<String> {
    static T: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    T.get_or_init(|| {
        let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
        let cfg = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| home.join(".config"));
        let key_in = |f: PathBuf, sect: &str, key: &str| -> Option<String> {
            let t = std::fs::read_to_string(f).ok()?;
            let mut inside = sect.is_empty();
            for l in t.lines() {
                let l = l.trim();
                if l.starts_with('[') {
                    inside = l == format!("[{sect}]");
                    continue;
                }
                if inside {
                    if let Some(v) = l.strip_prefix(&format!("{key}=")) {
                        let v = v.trim().trim_matches('"').to_string();
                        if !v.is_empty() {
                            return Some(v);
                        }
                    }
                }
            }
            None
        };
        let first = key_in(cfg.join("qt6ct/qt6ct.conf"), "Appearance", "icon_theme")
            .or_else(|| key_in(cfg.join("kdeglobals"), "Icons", "Theme"))
            .or_else(|| key_in(cfg.join("gtk-3.0/settings.ini"), "Settings", "gtk-icon-theme-name"));
        let mut out: Vec<String> = Vec::new();
        let mut todo: Vec<String> = first.into_iter().collect();
        while let Some(t) = todo.pop() {
            if out.contains(&t) || out.len() > 8 {
                continue;
            }
            out.push(t.clone());
            let inh = icon_roots().into_iter().find_map(|r| key_in(r.join(&t).join("index.theme"), "Icon Theme", "Inherits"));
            for x in inh.unwrap_or_default().split(',').rev() {
                let x = x.trim().to_string();
                if !x.is_empty() {
                    todo.push(x);
                }
            }
        }
        for t in ["hicolor", "Adwaita"] {
            if !out.iter().any(|x| x == t) {
                out.push(t.to_string());
            }
        }
        out
    })
}

/// An icon's file: a path as it is, else the icon theme in use and what it inherits (as Qt looks), PNG or SVG,
/// else /usr/share/pixmaps.
pub fn icon_path(name: &str) -> Option<PathBuf> {
    if name.is_empty() {
        return None;
    }
    let p = Path::new(name);
    if p.is_absolute() {
        return p.exists().then(|| p.to_path_buf());
    }
    let name = name.trim_end_matches(".png").trim_end_matches(".svg");
    let roots = icon_roots();
    let sizes = ["scalable", "512x512", "256x256", "128x128", "96x96", "64x64", "48x48", "32x32", "24x24", "16x16", "symbolic"];
    for th in icon_themes() {
        for r in &roots {
            let base = r.join(th);
            if !base.is_dir() {
                continue;
            }
            for sz in sizes {
                for sub in ["apps", "places", "devices", "categories", "mimetypes", "legacy", "actions", "status", "emblems"] {
                    for ext in ["svg", "png"] {
                        for f in [base.join(sz).join(sub).join(format!("{name}.{ext}")), base.join(sub).join(sz).join(format!("{name}.{ext}"))] {
                            if f.exists() {
                                return Some(f);
                            }
                        }
                    }
                }
            }
        }
    }
    for ext in ["svg", "png"] {
        let f = PathBuf::from(format!("/usr/share/pixmaps/{name}.{ext}"));
        if f.exists() {
            return Some(f);
        }
    }
    None
}

/// The Exec line ready to run: the field codes (%f %F %u %U %i %c %k…) taken out, %% kept as %.
pub fn command_line(exec: &str) -> String {
    let mut out = String::new();
    let mut it = exec.chars().peekable();
    while let Some(c) = it.next() {
        if c == '%' {
            match it.next() {
                Some('%') => out.push('%'),
                Some(_) | None => {}
            }
        } else {
            out.push(c);
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Opens an app on its own (setsid: it outlives the menu and the dock; nothing waits for it).
pub fn launch(a: &App) {
    let mut cmd = command_line(&a.exec);
    if a.terminal {
        cmd = format!("foot -e {cmd}");
    }
    run_detached(&cmd);
}

/// Runs a command line on its own, not waited for (the shell in between exits at once).
pub fn run_detached(cmd: &str) {
    let _ = std::process::Command::new("sh")
        .arg("-c")
        .arg(format!("setsid -f sh -c {} >/dev/null 2>&1 </dev/null", shell_quote(cmd)))
        .status();
}

pub fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// The menu's search: name, id and keywords, every word typed somewhere in them.
pub fn matches(a: &App, q: &str) -> bool {
    let hay = format!("{} {} {}", a.name, a.id, a.keywords).to_lowercase();
    q.to_lowercase().split_whitespace().all(|w| hay.contains(w))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_desktop_file_is_read_and_hidden_ones_left_out() {
        let a = parse("org.x.files", "[Desktop Entry]\nType=Application\nName=Files\nExec=files %U\nIcon=folder\nKeywords=browse;\n[Desktop Action New]\nName=Other\n").unwrap();
        assert_eq!(a.name, "Files");
        assert_eq!(a.icon, "folder");
        assert!(parse("x", "[Desktop Entry]\nName=X\nExec=x\nNoDisplay=true\n").is_none());
        assert!(parse("x", "[Desktop Entry]\nType=Link\nName=X\nExec=x\n").is_none());
    }

    #[test]
    fn field_codes_are_taken_out() {
        assert_eq!(command_line("firefox %u"), "firefox");
        assert_eq!(command_line("app --name=100%% %F"), "app --name=100%");
    }

    #[test]
    fn search_needs_every_word() {
        let a = parse("org.hearth.files", "[Desktop Entry]\nName=Hearth Files\nExec=f\nKeywords=folder;drive;\n").unwrap();
        assert!(matches(&a, "files"));
        assert!(matches(&a, "hearth drive"));
        assert!(!matches(&a, "music"));
    }

    #[test]
    fn quoting_survives_quotes() {
        assert_eq!(shell_quote("a 'b'"), "'a '\\''b'\\'''");
    }
}
