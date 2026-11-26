//! Steam's own files, read the way ProtonUp-Qt reads them: its folders (native and Flatpak), its libraries
//! (libraryfolders.vdf), its games (appmanifest_*.acf), which Proton each game's prefix last ran with
//! (compatdata/<app>/version), its compatibility tools (compatibilitytools.d, steamapps/common/Proton*) and each game's
//! pick (config.vdf's CompatToolMapping). Nothing here signs in to Steam or touches an account; config.vdf is changed
//! only while Steam is closed (it writes its own copy back when it closes), atomically with a .bak.

use std::path::{Path, PathBuf};

// ------------------------------------------------------------------ VDF (Valve's KeyValues text)

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    /// a value as written between its quotes (escapes kept as they are, so writing it back changes nothing)
    Str(String),
    Map(Vec<(String, Node)>),
}

impl Node {
    pub fn get(&self, path: &[&str]) -> Option<&Node> {
        let mut n = self;
        for k in path {
            match n {
                Node::Map(v) => n = &v.iter().find(|(x, _)| x.eq_ignore_ascii_case(k))?.1,
                Node::Str(_) => return None,
            }
        }
        Some(n)
    }
    pub fn str(&self, path: &[&str]) -> Option<&str> {
        match self.get(path)? {
            Node::Str(s) => Some(s),
            _ => None,
        }
    }
    pub fn entries(&self) -> &[(String, Node)] {
        match self {
            Node::Map(v) => v,
            _ => &[],
        }
    }
    /// The map at `path`, made where it's missing.
    pub fn map_mut(&mut self, path: &[&str]) -> &mut Vec<(String, Node)> {
        if let Node::Str(_) = self {
            *self = Node::Map(Vec::new());
        }
        let Node::Map(v) = self else { unreachable!() };
        match path.split_first() {
            None => v,
            Some((k, rest)) => {
                let i = match v.iter().position(|(x, _)| x.eq_ignore_ascii_case(k)) {
                    Some(i) => i,
                    None => {
                        v.push((k.to_string(), Node::Map(Vec::new())));
                        v.len() - 1
                    }
                };
                v[i].1.map_mut(rest)
            }
        }
    }
}

/// An unquoted token (a conditional like [$WIN32], or a bare word) is marked apart from a quoted one.
const BARE: char = '\u{1}';

fn tokens(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let b: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if c.is_whitespace() {
            i += 1;
        } else if c == '/' && b.get(i + 1) == Some(&'/') {
            while i < b.len() && b[i] != '\n' {
                i += 1;
            }
        } else if c == '{' || c == '}' {
            out.push(c.to_string());
            i += 1;
        } else if c == '"' {
            let mut s = String::from("\"");
            i += 1;
            while i < b.len() && b[i] != '"' {
                if b[i] == '\\' && i + 1 < b.len() {
                    s.push(b[i]);
                    i += 1;
                }
                s.push(b[i]);
                i += 1;
            }
            i += 1;
            out.push(s);
        } else {
            let mut s = String::from(BARE);
            while i < b.len() && !b[i].is_whitespace() && b[i] != '{' && b[i] != '}' && b[i] != '"' {
                s.push(b[i]);
                i += 1;
            }
            out.push(s);
        }
    }
    out
}

fn parse_map(t: &[String], i: &mut usize) -> Vec<(String, Node)> {
    let mut v = Vec::new();
    while *i < t.len() {
        let k = &t[*i];
        if k == "}" {
            *i += 1;
            break;
        }
        let key = k.trim_start_matches(['"', BARE]).to_string();
        *i += 1;
        match t.get(*i).map(String::as_str) {
            Some("{") => {
                *i += 1;
                v.push((key, Node::Map(parse_map(t, i))));
            }
            Some(s) if s.starts_with('"') || s.starts_with(BARE) => {
                v.push((key, Node::Str(s.trim_start_matches(['"', BARE]).to_string())));
                *i += 1;
            }
            _ => break,
        }
        // a conditional after the value ([$WIN32]) is a plain token: skipped
        while t.get(*i).map(|s| s.starts_with(BARE) && s[BARE.len_utf8()..].starts_with('[')).unwrap_or(false) {
            *i += 1;
        }
    }
    v
}

/// A VDF text's whole tree (a root map of its top keys).
pub fn parse(text: &str) -> Node {
    let t = tokens(text);
    let mut i = 0;
    Node::Map(parse_map(&t, &mut i))
}

fn write_map(v: &[(String, Node)], depth: usize, out: &mut String) {
    let ind = "\t".repeat(depth);
    for (k, n) in v {
        match n {
            Node::Str(s) => out.push_str(&format!("{ind}\"{k}\"\t\t\"{s}\"\n")),
            Node::Map(m) => {
                out.push_str(&format!("{ind}\"{k}\"\n{ind}{{\n"));
                write_map(m, depth + 1, out);
                out.push_str(&format!("{ind}}}\n"));
            }
        }
    }
}

/// The tree as Steam writes it (tabs, each map's braces on their own lines).
pub fn write(n: &Node) -> String {
    let mut s = String::new();
    write_map(n.entries(), 0, &mut s);
    s
}

// ------------------------------------------------------------------ Steam's folders

/// Steam's own folders on this device (native first, then the Flatpak's), as ProtonUp-Qt looks for them.
pub fn roots() -> Vec<PathBuf> {
    let h = hearth_oss_kit_a::home();
    let mut v: Vec<PathBuf> = Vec::new();
    for p in [h.join(".steam/root"), h.join(".steam/steam"), h.join(".local/share/Steam"), h.join(".var/app/com.valvesoftware.Steam/data/Steam")] {
        if p.join("steamapps").is_dir() || p.join("config").is_dir() {
            let real = std::fs::canonicalize(&p).unwrap_or(p.clone());
            if !v.iter().any(|x| std::fs::canonicalize(x).unwrap_or(x.clone()) == real) {
                v.push(p);
            }
        }
    }
    v
}

/// Every Steam library (libraryfolders.vdf's paths, and each root).
pub fn libraries(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = Vec::new();
    for r in roots {
        let mut add = |p: PathBuf| {
            if p.join("steamapps").is_dir() && !v.contains(&p) {
                v.push(p);
            }
        };
        add(r.clone());
        if let Ok(t) = std::fs::read_to_string(r.join("steamapps/libraryfolders.vdf")) {
            let n = parse(&t);
            if let Some(lf) = n.get(&["libraryfolders"]) {
                for (_, e) in lf.entries() {
                    if let Some(p) = e.str(&["path"]) {
                        add(PathBuf::from(p.replace("\\\\", "\\")));
                    }
                }
            }
        }
    }
    v
}

/// One of Steam's games: its id, name, library and the Proton its prefix last ran with.
#[derive(Debug, Clone)]
pub struct SteamGame {
    pub app: String,
    pub name: String,
    pub ran_with: String,
}

fn is_tool(name: &str) -> bool {
    name.starts_with("Proton") || name.starts_with("Steam Linux Runtime") || name.starts_with("Steamworks Common") || name.starts_with("Steam Runtime")
}

pub fn games(libs: &[PathBuf]) -> Vec<SteamGame> {
    let mut v: Vec<SteamGame> = Vec::new();
    for l in libs {
        let Ok(rd) = std::fs::read_dir(l.join("steamapps")) else { continue };
        for e in rd.flatten() {
            let n = e.file_name().to_string_lossy().to_string();
            let Some(app) = n.strip_prefix("appmanifest_").and_then(|x| x.strip_suffix(".acf")) else { continue };
            let Ok(t) = std::fs::read_to_string(e.path()) else { continue };
            let tree = parse(&t);
            let name = tree.str(&["AppState", "name"]).unwrap_or(app).to_string();
            if is_tool(&name) || v.iter().any(|g| g.app == app) {
                continue;
            }
            let ran_with = std::fs::read_to_string(l.join("steamapps/compatdata").join(app).join("version")).map(|s| s.trim().to_string()).unwrap_or_default();
            v.push(SteamGame { app: app.to_string(), name, ran_with });
        }
    }
    v.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    v
}

/// config.vdf's CompatToolMapping: each game's pick (app id → the tool's name); "0" is Steam's default.
pub fn mapping(roots: &[PathBuf]) -> Vec<(String, String)> {
    for r in roots {
        if let Ok(t) = std::fs::read_to_string(r.join("config/config.vdf")) {
            let n = parse(&t);
            if let Some(m) = n.get(&["InstallConfigStore", "Software", "Valve", "Steam", "CompatToolMapping"]) {
                return m.entries().iter().filter_map(|(k, e)| e.str(&["name"]).filter(|s| !s.is_empty()).map(|s| (k.clone(), s.to_string()))).collect();
            }
        }
    }
    Vec::new()
}

/// Is Steam open (it would write its own copy of config.vdf back over a change)?
pub fn running() -> bool {
    let Ok(rd) = std::fs::read_dir("/proc") else { return false };
    for e in rd.flatten() {
        if !e.file_name().to_string_lossy().chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        if let Ok(c) = std::fs::read_to_string(e.path().join("comm")) {
            let c = c.trim();
            if c == "steam" || c == "steamwebhelper" {
                return true;
            }
        }
    }
    false
}

/// A game's pick set (Some(tool)) or taken out (None) in config.vdf, with Steam closed. `app` "0" is the default.
pub fn set_mapping(root: &Path, app: &str, tool: Option<&str>) -> Result<(), String> {
    if running() {
        return Err("Close Steam first: it keeps its own copy of this while it's open.".into());
    }
    let p = root.join("config/config.vdf");
    if !p.is_file() {
        return Err("Steam hasn't made its settings file yet (start it once).".into());
    }
    let mut bad = false;
    hearth_oss_kit_a::settings_file::update_text(&p, |t| {
        let mut n = parse(t);
        if n.get(&["InstallConfigStore"]).is_none() {
            bad = true;
            return t.to_string();
        }
        let m = n.map_mut(&["InstallConfigStore", "Software", "Valve", "Steam", "CompatToolMapping"]);
        m.retain(|(k, _)| k != app);
        if let Some(tool) = tool {
            let pri = if app == "0" { "75" } else { "250" };
            m.push((app.to_string(), Node::Map(vec![("name".into(), Node::Str(tool.to_string())), ("config".into(), Node::Str(String::new())), ("priority".into(), Node::Str(pri.into()))])));
        }
        write(&n)
    })
    .map_err(|e| e.to_string())?;
    if bad {
        return Err("Steam's settings file couldn't be read, so it wasn't changed.".into());
    }
    Ok(())
}

/// The internal name Steam uses for one of Valve's Protons ("Proton 9.0" → proton_9, "Proton - Experimental" →
/// proton_experimental), and its app id (to install or remove it through Steam).
pub fn valve_tool(folder: &str) -> (String, Option<u32>) {
    let low = folder.to_lowercase();
    if low.contains("experimental") {
        return ("proton_experimental".into(), Some(1493710));
    }
    if low.contains("hotfix") {
        return ("proton_hotfix".into(), Some(2180100));
    }
    let major: String = low.trim_start_matches("proton").trim().chars().take_while(|c| c.is_ascii_digit()).collect();
    let app = match major.as_str() {
        "10" => Some(3658110),
        "9" => Some(2805730),
        "8" => Some(2348590),
        "7" => Some(1887720),
        "6" => Some(1580130),
        "5" => Some(1420170),
        _ => None,
    };
    (format!("proton_{major}"), app)
}

#[cfg(test)]
mod tests {
    use super::*;
    const C: &str = "\"InstallConfigStore\"\n{\n\t\"Software\"\n\t{\n\t\t\"Valve\"\n\t\t{\n\t\t\t\"Steam\"\n\t\t\t{\n\t\t\t\t\"CompatToolMapping\"\n\t\t\t\t{\n\t\t\t\t\t\"0\"\n\t\t\t\t\t{\n\t\t\t\t\t\t\"name\"\t\t\"Proton-Hearth9-4\"\n\t\t\t\t\t\t\"config\"\t\t\"\"\n\t\t\t\t\t\t\"priority\"\t\t\"75\"\n\t\t\t\t\t}\n\t\t\t\t\t\"1245620\"\n\t\t\t\t\t{\n\t\t\t\t\t\t\"name\"\t\t\"GE-Proton9-20\"\n\t\t\t\t\t\t\"config\"\t\t\"\"\n\t\t\t\t\t\t\"priority\"\t\t\"250\"\n\t\t\t\t\t}\n\t\t\t\t}\n\t\t\t\t\"Path\"\t\t\"C:\\\\x \\\"y\\\"\"\n\t\t\t}\n\t\t}\n\t}\n}\n";

    #[test]
    fn reads_and_writes_back_the_same() {
        let n = parse(C);
        assert_eq!(write(&n), C);
        let m = n.get(&["InstallConfigStore", "Software", "Valve", "Steam", "CompatToolMapping"]).unwrap();
        assert_eq!(m.entries().len(), 2);
        assert_eq!(n.str(&["InstallConfigStore", "Software", "Valve", "Steam", "CompatToolMapping", "1245620", "name"]), Some("GE-Proton9-20"));
    }

    #[test]
    fn changes_one_game_only() {
        let mut n = parse(C);
        let m = n.map_mut(&["InstallConfigStore", "Software", "Valve", "Steam", "CompatToolMapping"]);
        m.retain(|(k, _)| k != "1245620");
        let t = write(&n);
        assert!(!t.contains("1245620") && t.contains("Proton-Hearth9-4") && t.contains("\"Path\"\t\t\"C:\\\\x \\\"y\\\"\""));
        let mut e = parse("");
        e.map_mut(&["a", "b"]).push(("c".into(), Node::Str("d".into())));
        assert_eq!(write(&e), "\"a\"\n{\n\t\"b\"\n\t{\n\t\t\"c\"\t\t\"d\"\n\t}\n}\n");
    }

    #[test]
    fn valve_names() {
        assert_eq!(valve_tool("Proton 9.0"), ("proton_9".into(), Some(2805730)));
        assert_eq!(valve_tool("Proton 8.0"), ("proton_8".into(), Some(2348590)));
        assert_eq!(valve_tool("Proton - Experimental").0, "proton_experimental");
    }
}
