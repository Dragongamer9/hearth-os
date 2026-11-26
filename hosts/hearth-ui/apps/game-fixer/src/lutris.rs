//! What Hearth Game Fixer reads and writes, all of it Lutris' own (Lutris, GPL-3, https://lutris.net):
//!
//! * the games: `lutris --list-games --installed --json` (Lutris' command, its own or its Flathub app's);
//! * each game's settings: Lutris' YAML file for it (`~/.config/lutris/games/<slug>-<n>.yml`, or Lutris' newer
//!   `~/.local/share/lutris/games/`, or the Flathub app's own folders), changed one line at a time ([`yset`]: every
//!   other line stays exactly as Lutris wrote it), atomically with a .bak (settings_file::update_text, red flag 9);
//! * the wine versions and runners Lutris has (`lutris --list-wine-versions`, its runners folder);
//! * the community's install scripts (lutris.net's API, read with curl) and installing one (`lutris lutris:install/…`);
//! * starting a game (`lutris lutris:rungame/<slug>`), its output kept as the game's last-start log.
//!
//! Hearth's own notes on a game (the script it came from, its fixes, how big it is) are in
//! ~/.local/share/hearth/game-fixer/games/<slug>.json, written with the safe writer.

use serde_json::{json, Value};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default)]
pub struct Game {
    pub id: i64,
    pub slug: String,
    pub name: String,
    /// Lutris' runner (wine, dosbox, scummvm…)
    pub runner: String,
    pub platform: String,
    pub directory: String,
    pub playtime: f64,
    /// its Lutris settings file, and what's in it
    pub config: Option<PathBuf>,
    pub yaml: String,
    /// Hearth's notes on it
    pub note: Value,
}

impl Game {
    pub fn get(&self, path: &[&str]) -> Option<String> {
        yget(&self.yaml, path)
    }
    /// The wine (or Proton) version it runs with, else its runner's name.
    pub fn runs_with(&self) -> String {
        if self.runner == "wine" {
            self.get(&["wine", "version"]).map(|v| pretty(&v)).unwrap_or_else(|| "Wine (Lutris' default)".into())
        } else {
            runner_name(&self.runner)
        }
    }
    /// ready · needs a fix · Installing · 64%
    pub fn state(&self) -> (String, u8) {
        if let Some(p) = self.note.get("installing").and_then(Value::as_f64) {
            return (format!("Installing · {}%", p.round() as i64), 2);
        }
        if self.note.get("needsFix").and_then(Value::as_bool).unwrap_or(false) {
            return ("needs a fix".into(), 1);
        }
        ("ready".into(), 0)
    }
    pub fn line(&self) -> String {
        format!("{} · {}", self.runs_with(), self.state().0)
    }
    pub fn dir(&self) -> PathBuf {
        let d = self.get(&["game", "prefix"]).filter(|_| self.directory.is_empty()).unwrap_or_else(|| self.directory.clone());
        PathBuf::from(expand(&d))
    }
}

/// "~/…" and "$HOME/…" in a path written in a settings file.
pub fn expand(p: &str) -> String {
    let h = hearth_oss_kit::home().to_string_lossy().to_string();
    if let Some(r) = p.strip_prefix("~/") {
        format!("{h}/{r}")
    } else if let Some(r) = p.strip_prefix("$HOME/") {
        format!("{h}/{r}")
    } else {
        p.to_string()
    }
}

/// Lutris' runners by their names as people say them.
pub fn runner_name(r: &str) -> String {
    match r {
        "wine" => "Wine".into(),
        "dosbox" => "DOSBox".into(),
        "scummvm" => "ScummVM".into(),
        "linux" => "Linux (native)".into(),
        "steam" => "Steam".into(),
        "mame" => "MAME".into(),
        "pcsx2" => "PCSX2".into(),
        "dolphin" => "Dolphin".into(),
        "retroarch" => "RetroArch".into(),
        "" => "Not set".into(),
        o => {
            let mut c = o.chars();
            c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
        }
    }
}

/// A wine version's folder name as people say it: "wine-ge-8-26-x86_64" → "Wine-GE 8-26",
/// "lutris-GE-Proton8-26-x86_64" and "GE-Proton9-20" → "Proton-GE 8-26" / "Proton-GE 9-20". Others as they are.
pub fn pretty(v: &str) -> String {
    let s = v.trim().trim_end_matches("-x86_64").trim_end_matches("-i686");
    let low = s.to_lowercase();
    if let Some(r) = low.strip_prefix("wine-ge-") {
        return format!("Wine-GE {}", &s[s.len() - r.len()..]);
    }
    for pre in ["lutris-ge-proton", "ge-proton"] {
        if let Some(r) = low.strip_prefix(pre) {
            return format!("Proton-GE {}", &s[s.len() - r.len()..]);
        }
    }
    if let Some(r) = low.strip_prefix("lutris-") {
        return format!("Lutris Wine {}", &s[s.len() - r.len()..]);
    }
    s.to_string()
}

/// `lutris --list-games --installed --json`'s answer.
pub fn games_of(j: &Value) -> Vec<Game> {
    let mut v: Vec<Game> = j
        .as_array()
        .map(|a| {
            a.iter()
                .map(|g| Game {
                    id: g.get("id").and_then(Value::as_i64).unwrap_or(0),
                    slug: g.get("slug").and_then(Value::as_str).unwrap_or("").to_string(),
                    name: g.get("name").and_then(Value::as_str).unwrap_or("").to_string(),
                    runner: g.get("runner").and_then(Value::as_str).unwrap_or("").to_string(),
                    platform: g.get("platform").and_then(Value::as_str).unwrap_or("").to_string(),
                    directory: g.get("directory").and_then(Value::as_str).unwrap_or("").to_string(),
                    playtime: g.get("playtime").and_then(Value::as_f64).unwrap_or(0.0),
                    ..Game::default()
                })
                .filter(|g| !g.slug.is_empty())
                .collect()
        })
        .unwrap_or_default();
    v.sort_by_key(|g| g.id);
    v
}

/// Lutris' folders for its games' settings files: its own (old and new places) and its Flathub app's.
pub fn config_dirs() -> Vec<PathBuf> {
    let h = hearth_oss_kit::home();
    let cfg = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| h.join(".config"));
    let data = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|| h.join(".local/share"));
    vec![
        data.join("lutris/games"),
        cfg.join("lutris/games"),
        h.join(".var/app/net.lutris.Lutris/data/lutris/games"),
        h.join(".var/app/net.lutris.Lutris/config/lutris/games"),
    ]
}

/// The game's settings file: `<slug>-<n>.yml` (the newest when there are several).
pub fn config_for(slug: &str) -> Option<PathBuf> {
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for d in config_dirs() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let n = e.file_name().to_string_lossy().to_string();
            let stem = n.trim_end_matches(".yml");
            let ok = n.ends_with(".yml") && (stem == slug || stem.strip_prefix(slug).map(|r| r.starts_with('-') && r[1..].chars().all(|c| c.is_ascii_digit())).unwrap_or(false));
            if ok {
                let t = e.metadata().and_then(|m| m.modified()).unwrap_or(std::time::UNIX_EPOCH);
                if best.as_ref().map(|b| t > b.0).unwrap_or(true) {
                    best = Some((t, e.path()));
                }
            }
        }
    }
    best.map(|b| b.1)
}

/// Lutris' runners folder: which runners it has (DOSBox, ScummVM…), wine and Proton aside.
pub fn other_runners() -> Vec<String> {
    let h = hearth_oss_kit::home();
    let data = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|| h.join(".local/share"));
    let mut v: Vec<String> = Vec::new();
    for d in [data.join("lutris/runners"), h.join(".var/app/net.lutris.Lutris/data/lutris/runners")] {
        if let Ok(rd) = std::fs::read_dir(&d) {
            for e in rd.flatten() {
                let n = e.file_name().to_string_lossy().to_string();
                if e.path().is_dir() && !matches!(n.as_str(), "wine" | "proton") && !v.contains(&n) {
                    v.push(n);
                }
            }
        }
    }
    v.sort();
    v.into_iter().map(|r| runner_name(&r)).collect()
}

/// `lutris --list-wine-versions`' lines: one version each.
pub fn wine_versions(out: &str) -> Vec<String> {
    let mut v: Vec<String> = Vec::new();
    for l in out.lines() {
        let l = l.trim().trim_start_matches("- ");
        if l.is_empty() || l.ends_with(':') || l.starts_with("INFO") || l.starts_with("WARNING") || l.starts_with("DEBUG") {
            continue;
        }
        let name = l.split(" (").next().unwrap_or(l).trim().to_string();
        if !v.contains(&name) {
            v.push(name);
        }
    }
    v
}

pub use hearth_oss_kit::yaml::{quote, yget, ymap, yset};

// ------------------------------------------------------------------ fixes

/// One fix: what it sets in the game's Lutris settings when it's on (taken out when it's off), and files of the
/// game's own it puts in place (moved aside into the game folder's .hearth-off when it's off).
#[derive(Debug, Clone, PartialEq)]
pub struct Fix {
    pub id: String,
    pub name: String,
    pub about: String,
    /// "system.env.WINE_LARGE_ADDRESS_AWARE" → "1"
    pub set: Vec<(String, String)>,
    /// what "off" leaves for a key that's a switch in Lutris (false), else taken out
    pub files: Vec<String>,
    pub from_script: bool,
}

/// Hearth's own fixes, for any Windows game (the script's come first, from its notes).
pub fn known_fixes() -> Vec<Fix> {
    let f = |id: &str, name: &str, about: &str, set: &[(&str, &str)]| Fix { id: id.into(), name: name.into(), about: about.into(), set: set.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect(), files: vec![], from_script: false };
    vec![
        f("laa", "Large address aware", "Lets the game use 4 GB, stops the late-game crash", &[("system.env.WINE_LARGE_ADDRESS_AWARE", "1"), ("system.env.PROTON_FORCE_LARGE_ADDRESS_AWARE", "1")]),
        f("sync", "Esync / Fsync", "Smoother with many units on screen", &[("wine.esync", "true"), ("wine.fsync", "true")]),
        f("cursor", "Keep the cursor inside the game", "Off: you play with two screens", &[("wine.MouseWarpOverride", "force")]),
    ]
}

/// The game's fixes: the ones its script brought (Hearth's notes), then Hearth's own that the script didn't.
pub fn fixes_of(g: &Game) -> Vec<Fix> {
    let mut v: Vec<Fix> = g
        .note
        .get("fixes")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .map(|x| Fix {
                    id: x.get("id").and_then(Value::as_str).unwrap_or("").to_string(),
                    name: x.get("name").and_then(Value::as_str).unwrap_or("").to_string(),
                    about: x.get("about").and_then(Value::as_str).unwrap_or("").to_string(),
                    set: x.get("set").and_then(Value::as_object).map(|m| m.iter().map(|(k, v)| (k.clone(), v.as_str().map(str::to_string).unwrap_or_else(|| v.to_string()))).collect()).unwrap_or_default(),
                    files: x.get("files").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).map(str::to_string).collect()).unwrap_or_default(),
                    from_script: true,
                })
                .filter(|f| !f.id.is_empty())
                .collect()
        })
        .unwrap_or_default();
    for k in known_fixes() {
        if !v.iter().any(|f| f.id == k.id || f.name == k.name) {
            v.push(k);
        }
    }
    v
}

fn split_path(p: &str) -> Vec<&str> {
    // "system.env.WINEDLLOVERRIDES": the first two dots part it, the rest is the key's own
    let mut it = p.splitn(3, '.');
    it.by_ref().collect()
}

/// Is it on: every setting it makes is there with its value, and its files are in place.
pub fn fix_on(g: &Game, f: &Fix) -> bool {
    if f.set.is_empty() && f.files.is_empty() {
        return false;
    }
    let sets = f.set.iter().all(|(k, v)| {
        let path = split_path(k);
        match g.get(&path) {
            Some(x) => x == *v || (x == "True" && v == "true"),
            None => false,
        }
    });
    let dir = g.dir();
    let files = f.files.iter().all(|x| dir.join(x).exists());
    sets && files
}

/// The game's settings with a fix turned on or off.
pub fn apply_fix(yaml: &str, f: &Fix, on: bool) -> String {
    let mut t = yaml.to_string();
    for (k, v) in &f.set {
        let path = split_path(k);
        t = if on {
            yset(&t, &path, Some(quote_plain(v).as_str()))
        } else if v == "true" {
            yset(&t, &path, Some("false"))
        } else {
            yset(&t, &path, None)
        };
    }
    t
}

/// A value from a fix as YAML (true/false and numbers as they are).
fn quote_plain(v: &str) -> String {
    if matches!(v, "true" | "false") || v.parse::<f64>().is_ok() {
        v.to_string()
    } else {
        quote(v)
    }
}

/// A fix's files moved aside (off) or back (on), inside the game's own folder.
pub fn move_fix_files(dir: &Path, f: &Fix, on: bool) -> Result<(), String> {
    let off = dir.join(".hearth-off");
    for x in &f.files {
        if x.contains("..") {
            continue;
        }
        let (from, to) = if on { (off.join(x), dir.join(x)) } else { (dir.join(x), off.join(x)) };
        if !from.exists() {
            continue;
        }
        if let Some(p) = to.parent() {
            std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
        }
        std::fs::rename(&from, &to).map_err(|e| format!("{}: {e}", from.display()))?;
    }
    Ok(())
}

// ------------------------------------------------------------------ Hearth's notes on a game, logs

pub fn notes_dir() -> PathBuf {
    hearth_oss_kit::data_dir("game-fixer").join("games")
}
pub fn logs_dir() -> PathBuf {
    hearth_oss_kit::data_dir("game-fixer").join("logs")
}
pub fn read_note(slug: &str) -> Value {
    hearth_oss_kit::settings_file::read_json(&notes_dir().join(format!("{slug}.json")))
}
/// Sets these keys of the game's note (merge-on-write, atomic, a .bak).
pub fn write_note(slug: &str, keys: serde_json::Map<String, Value>) -> Result<(), String> {
    let d = notes_dir();
    std::fs::create_dir_all(&d).map_err(|e| e.to_string())?;
    hearth_oss_kit::settings_file::merge_json(&d.join(format!("{slug}.json")), &keys).map(|_| ()).map_err(|e| e.to_string())
}

/// A log's lines and how each is coloured: 0 plain, 1 good (green), 2 a warning (yellow), 3 an error (red).
pub fn log_lines(text: &str) -> Vec<(String, String, u8)> {
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let (time, rest) = match l.split_once(' ') {
                Some((a, b)) if a.len() == 8 && a.chars().filter(|c| *c == ':').count() == 2 => (a.to_string(), b.to_string()),
                _ => (String::new(), l.to_string()),
            };
            let low = rest.to_lowercase();
            let kind = if low.starts_with("err:") || low.contains(" error") || low.contains("crash") || low.contains("stopped") {
                3
            } else if low.starts_with("fixme") || low.starts_with("warn") || low.contains("warning") {
                2
            } else if low.contains("ready") || low.contains("first frame") || low.contains("up and running") || low.contains("installed") {
                1
            } else {
                0
            };
            (time, rest, kind)
        })
        .collect()
}

/// A script's step in words, from Lutris' installer task.
pub fn step_words(task: &Value, wine: &str) -> String {
    let s = |k: &str| task.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    if let Some(t) = task.get("task") {
        let name = t.get("name").and_then(Value::as_str).unwrap_or("");
        let g = |k: &str| t.get(k).and_then(Value::as_str).unwrap_or("").to_string();
        return match name {
            "create_prefix" => format!("Make a {} prefix{}", if g("arch") == "win32" { "32-bit" } else { "64-bit" }, if wine.is_empty() { String::new() } else { format!(" with {wine}") }),
            "winetricks" => format!("Install {} into the prefix", g("app")),
            "wineexec" => format!("Run {}", if g("executable").is_empty() { "the installer".into() } else { g("executable") }),
            "set_regedit" => format!("Set {} in the registry", g("key")),
            "winekill" => "Close the installer".into(),
            o => o.replace('_', " "),
        };
    }
    if task.get("extract").is_some() {
        return "Unpack the game's files".into();
    }
    if task.get("insert-disc").is_some() {
        return "Read your disc image".into();
    }
    if task.get("move").is_some() || task.get("copy").is_some() || task.get("merge").is_some() {
        return "Put the files in place".into();
    }
    if task.get("execute").is_some() {
        return "Run the script's own step".into();
    }
    let k = task.as_object().and_then(|m| m.keys().next().cloned()).unwrap_or_default();
    if k.is_empty() { s("name") } else { k.replace('_', " ") }
}

/// What Hearth keeps of a script when a game is installed with it: its name, who made it, when, its steps.
pub fn script_note(inst: &Value) -> Value {
    let script = inst.get("script").cloned().unwrap_or(Value::Null);
    let wine = script.pointer("/wine/version").and_then(Value::as_str).map(pretty).unwrap_or_default();
    let steps: Vec<Value> = script
        .get("installer")
        .and_then(Value::as_array)
        .map(|a| a.iter().map(|t| json!({"text": step_words(t, &wine), "done": false})).collect())
        .unwrap_or_default();
    json!({
        "name": inst.get("version").and_then(Value::as_str).unwrap_or(""),
        "slug": inst.get("slug").and_then(Value::as_str).unwrap_or(""),
        "by": inst.get("user").and_then(Value::as_str).unwrap_or(""),
        "updated": inst.get("updated_at").and_then(Value::as_str).unwrap_or(""),
        "asks": wine,
        "steps": steps,
    })
}

/// A desktop shortcut for the game (it starts through Lutris).
pub fn shortcut(g: &Game, lutris: &[String]) -> String {
    let mut exec: Vec<String> = lutris.iter().map(|a| if a.contains(' ') { format!("\"{a}\"") } else { a.clone() }).collect();
    exec.push(format!("lutris:rungame/{}", g.slug));
    format!(
        "[Desktop Entry]\nType=Application\nName={}\nComment=Started with Lutris (Hearth Game Fixer)\nExec={}\nIcon=lutris_{}\nCategories=Game;\nStartupNotify=true\n",
        g.name.replace('\n', " "),
        exec.join(" "),
        g.slug
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const Y: &str = "game:\n  exe: drive_c/BFME2/lotrbfme2ep1.exe\n  prefix: ~/Games/bfme2/prefix\n  arch: win64\nsystem:\n  env:\n    WINEDLLOVERRIDES: d3d9=n\n    DXVK_HUD: '0'\n  prelaunch_command: set-hd-mode.sh\nwine:\n  version: Proton Hearth 9-4\n  dxvk: true\n";

    #[test]
    fn fixes_turn_on_and_off() {
        let laa = known_fixes().remove(0);
        let on = apply_fix(Y, &laa, true);
        let g = Game { yaml: on.clone(), ..Game::default() };
        assert!(fix_on(&g, &laa));
        let off = apply_fix(&on, &laa, false);
        let g2 = Game { yaml: off, ..Game::default() };
        assert!(!fix_on(&g2, &laa));
        let sync = &known_fixes()[1];
        let s = apply_fix(Y, sync, false);
        assert!(s.contains("esync: false") && s.contains("fsync: false"));
    }

    #[test]
    fn names() {
        assert_eq!(pretty("wine-ge-8-26-x86_64"), "Wine-GE 8-26");
        assert_eq!(pretty("lutris-GE-Proton8-26-x86_64"), "Proton-GE 8-26");
        assert_eq!(pretty("GE-Proton9-20"), "Proton-GE 9-20");
        assert_eq!(pretty("Proton Hearth 9-4"), "Proton Hearth 9-4");
        assert_eq!(wine_versions("lutris-7.2-2\nwine-ge-8-26-x86_64\n\nINFO 2026 x\n"), ["lutris-7.2-2", "wine-ge-8-26-x86_64"]);
        let l = log_lines("01:12:04 Starting game.dat\n01:12:05 fixme: dsound: x\n01:12:11 First frame shown after 6.8 s\n");
        assert_eq!(l.iter().map(|x| x.2).collect::<Vec<_>>(), [0, 2, 1]);
        assert_eq!(l[0].0, "01:12:04");
    }

    #[test]
    fn script_steps_in_words() {
        let inst = json!({"version": "RotWK + Edain", "user": "someone", "updated_at": "2026-11-09T10:00:00Z", "script": {"wine": {"version": "wine-ge-8-26-x86_64"}, "installer": [{"task": {"name": "create_prefix", "arch": "win64"}}, {"task": {"name": "winetricks", "app": "d3dx9 vcrun2005"}}, {"insert-disc": {"requires": "x"}}]}});
        let n = script_note(&inst);
        let steps: Vec<&str> = n["steps"].as_array().unwrap().iter().map(|s| s["text"].as_str().unwrap()).collect();
        assert_eq!(steps, ["Make a 64-bit prefix with Wine-GE 8-26", "Install d3dx9 vcrun2005 into the prefix", "Read your disc image"]);
    }
}
