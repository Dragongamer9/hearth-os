//! What the hub tells the surfaces (and the rules behind it), as plain data: the library and its tiles, the open
//! windows as Firehold names them, games among them, the hero, the recent row, the counts. No drawing here, so it's
//! all tested with `cargo test`.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// One game (or Windows app) in the library, as the tiles read it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Game {
    pub id: String,
    pub title: String,
    /// steam, gog, epic, emu, manual, winapp...
    pub source: String,
    pub installed: bool,
    pub update: bool,
    /// Steam Cloud's own folder for it has saves, or Hearth Saves backed it up.
    pub cloud_saves: bool,
    pub dev_build: bool,
    /// Milliseconds since 1970 (0: never).
    pub last_played: i64,
    pub minutes: i64,
    pub chapter: String,
    pub exe: String,
}

impl Game {
    /// The store's name on the tile.
    pub fn store(&self) -> String {
        store_name(&self.source)
    }
    /// The filter it counts under.
    pub fn kind(&self) -> &'static str {
        match self.source.as_str() {
            "steam" => "steam",
            "gog" => "gog",
            "epic" => "epic",
            "emu" => "emu",
            "winapp" => "winapp",
            "manual" => "yours",
            _ => "other",
        }
    }
    /// The tile's state line (mockup 5): Playing, Update ready, Dev build, Installed, Cloud saves, Not installed.
    pub fn state(&self, playing: bool) -> &'static str {
        if playing {
            "Playing"
        } else if self.installed && self.update {
            "Update ready"
        } else if self.installed && self.dev_build {
            "Dev build"
        } else if self.installed {
            "Installed"
        } else if self.cloud_saves {
            "Cloud saves"
        } else {
            "Not installed"
        }
    }
}

pub fn store_name(source: &str) -> String {
    match source {
        "steam" => "Steam".into(),
        "gog" => "GOG".into(),
        "epic" => "Epic".into(),
        "emu" => "Emulator".into(),
        "manual" => "Your game".into(),
        "winapp" => "Windows app".into(),
        "" => "Game".into(),
        s => {
            let mut c = s.chars();
            c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
        }
    }
}

fn dev_exe(exe: &str) -> bool {
    let e = exe.to_lowercase();
    ["/dev/", "/devel/", "/build/", "/builds/", "/export/", "/exports/", "/project/", "/projects/", "godot"]
        .iter()
        .any(|p| e.contains(p))
        || e.ends_with(".x86_64")
        || e.ends_with(".pck")
}

/// Milliseconds from Hearth's list: a number (ms, or seconds when it's small) or a string of one.
fn ms(v: Option<&Value>) -> i64 {
    let n = match v {
        Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0),
        Some(Value::String(s)) => s.trim().parse::<f64>().unwrap_or(0.0),
        _ => 0.0,
    };
    if n <= 0.0 {
        0
    } else if n < 1e11 {
        (n * 1000.0) as i64
    } else {
        n as i64
    }
}

/// An id Hearth's helpers take: letters, digits and ":._-+", nothing that a shell or a path could misread.
pub fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 200 && id.chars().all(|c| c.is_ascii_alphanumeric() || ":._-+ ".contains(c)) && !id.contains("..")
}

/// Hearth's list (`nimbus --list-games --all`: a JSON array of games) as tiles. `cloud` says whether a game has saves
/// in the cloud (Steam's folder or Hearth Saves), by id and title.
pub fn parse_library(raw: &Value, cloud: &dyn Fn(&str, &str) -> bool) -> Vec<Game> {
    let list = match raw {
        Value::Array(a) => a.as_slice(),
        Value::Object(o) => o.get("games").and_then(|g| g.as_array()).map(|a| a.as_slice()).unwrap_or(&[]),
        _ => &[],
    };
    let mut out = Vec::new();
    for g in list {
        let Some(o) = g.as_object() else { continue };
        let Some(id) = o.get("id").and_then(|i| i.as_str()) else { continue };
        if !valid_id(id) {
            continue;
        }
        let s = |k: &str| o.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
        let title = Some(s("title")).filter(|t| !t.trim().is_empty()).unwrap_or_else(|| id.to_string());
        let state = s("state");
        let installed = o.get("installed").and_then(|i| i.as_bool()) != Some(false) && (state.is_empty() || state == "installed");
        let source = s("source");
        let exe = s("exe");
        out.push(Game {
            id: id.to_string(),
            cloud_saves: cloud(id, &title),
            dev_build: source == "manual" && dev_exe(&exe),
            title,
            installed,
            update: o.get("update").and_then(|u| u.as_bool()) == Some(true),
            last_played: ms(o.get("lastPlayed")),
            minutes: o.get("minutes").and_then(|m| m.as_i64()).unwrap_or(0),
            chapter: s("chapter").chars().take(80).collect(),
            source,
            exe,
        });
    }
    out
}

/// Windows apps (`hearth-winapps list`: {"apps": [{id, name, last: {at}}]}) as library entries.
pub fn parse_winapps(raw: &Value) -> Vec<Game> {
    let Some(apps) = raw.get("apps").and_then(|a| a.as_array()) else { return Vec::new() };
    apps.iter()
        .filter_map(|a| {
            let id = a.get("id")?.as_str()?;
            if !valid_id(id) {
                return None;
            }
            let last = a.get("last").map(|l| l.get("at").or_else(|| l.get("time"))).unwrap_or(None);
            Some(Game {
                id: format!("winapp:{id}"),
                title: a.get("name").and_then(|n| n.as_str()).filter(|n| !n.is_empty()).unwrap_or(id).to_string(),
                source: "winapp".into(),
                installed: true,
                last_played: ms(last),
                ..Game::default()
            })
        })
        .collect()
}

/// The filter chips' counts (mockup 5): All, Installed, Steam, GOG, Epic, Windows apps, Emulators, Your games.
pub fn counts(games: &[Game]) -> Vec<(String, String, usize)> {
    let mut v = vec![
        ("all".to_string(), "All".to_string(), games.len()),
        ("installed".into(), "Installed".into(), games.iter().filter(|g| g.installed).count()),
    ];
    for (k, label) in [("steam", "Steam"), ("gog", "GOG"), ("epic", "Epic"), ("winapp", "Windows apps"), ("emu", "Emulators"), ("yours", "Your games")] {
        v.push((k.to_string(), label.to_string(), games.iter().filter(|g| g.kind() == k).count()));
    }
    v
}

pub fn filter_ok(g: &Game, filter: &str) -> bool {
    match filter {
        "" | "all" => true,
        "installed" => g.installed,
        k => g.kind() == k,
    }
}

/// The library's order: installed games by when they were played (latest first), then the rest as Hearth lists them.
pub fn library_order(games: &[Game]) -> Vec<Game> {
    let mut inst: Vec<Game> = games.iter().filter(|g| g.installed).cloned().collect();
    inst.sort_by(|a, b| b.last_played.cmp(&a.last_played));
    inst.extend(games.iter().filter(|g| !g.installed).cloned());
    inst
}

/// "41 MIN AGO", "5 H AGO", "3 DAYS AGO" (the hero's line, upper case there).
pub fn ago(last_ms: i64, now_ms: i64) -> String {
    if last_ms <= 0 {
        return String::new();
    }
    let mins = ((now_ms - last_ms).max(0) / 60000) as i64;
    if mins < 1 {
        "just now".into()
    } else if mins < 60 {
        format!("{mins} min ago")
    } else if mins < 60 * 24 {
        format!("{} h ago", mins / 60)
    } else if mins < 60 * 48 {
        "yesterday".into()
    } else {
        format!("{} days ago", mins / (60 * 24))
    }
}

/// "Playing 1 h 12 min" from seconds.
pub fn playing_for(secs: u64) -> String {
    let m = secs / 60;
    if m < 1 {
        "Playing".into()
    } else if m < 60 {
        format!("Playing {m} min")
    } else {
        format!("Playing {} h {} min", m / 60, m % 60)
    }
}

/// One open window, as the hub names it for the surfaces.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Win {
    pub id: String,
    pub app_id: String,
    pub title: String,
    /// Firehold's name for it (Hearth Files, Firefox, the game's own name).
    pub name: String,
    /// What's in it (Pictures › Trips, 3 tabs) or, for a game, "game · paused".
    pub sub: String,
    pub game: bool,
    pub game_id: String,
    pub source: String,
    pub minimized: bool,
    pub fullscreen: bool,
    pub activated: bool,
    /// The latest picture of it (a PNG in the runtime folder), when Everything open asked for one.
    pub pic: String,
    pub pid: i64,
}

/// Friendly names for app ids Firehold knows (and the dock's letter: Firefox is W, for the web).
pub fn app_name(app_id: &str) -> String {
    let a = app_id.to_lowercase();
    let known: &[(&str, &str)] = &[
        ("org.hearth.files", "Hearth Files"),
        ("hearth-files", "Hearth Files"),
        ("firefox", "Firefox"),
        ("org.mozilla.firefox", "Firefox"),
        ("discord", "Discord"),
        ("com.discordapp.discord", "Discord"),
        ("steam", "Steam"),
        ("org.kde.dolphin", "Dolphin"),
        ("org.kde.konsole", "Konsole"),
        ("org.kde.kate", "Kate"),
        ("foot", "Terminal"),
        ("org.hearth.settings", "Settings"),
        ("firehold-settings", "Settings"),
    ];
    if let Some((_, n)) = known.iter().find(|(k, _)| *k == a) {
        return n.to_string();
    }
    let last = app_id.rsplit('.').next().unwrap_or(app_id);
    let last = last.replace(['-', '_'], " ");
    let mut c = last.chars();
    c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_else(|| "Window".into())
}

pub fn dock_letter(name: &str, game: bool) -> String {
    if game {
        return "▶".into();
    }
    if name == "Firefox" {
        return "W".into();
    }
    if name == "Hearth Files" {
        return "F".into();
    }
    name.chars().find(|c| c.is_alphanumeric()).map(|c| c.to_uppercase().collect()).unwrap_or_else(|| "?".into())
}

/// Which library game a window is: Steam's `steam_app_<n>`, or the game's own title or id.
pub fn game_for<'a>(app_id: &str, title: &str, library: &'a [Game]) -> Option<&'a Game> {
    if let Some(n) = app_id.strip_prefix("steam_app_") {
        let id = format!("steam:{n}");
        if let Some(g) = library.iter().find(|g| g.id == id) {
            return Some(g);
        }
    }
    let t = title.trim().to_lowercase();
    let a = app_id.trim().to_lowercase();
    if t.is_empty() && a.is_empty() {
        return None;
    }
    library.iter().filter(|g| g.source != "winapp").find(|g| {
        let gt = g.title.to_lowercase();
        (!t.is_empty() && (t == gt || t.starts_with(&format!("{gt} ")) || t.starts_with(&format!("{gt} —")))) || a == g.id.to_lowercase() || (!a.is_empty() && a == gt)
    })
}

/// A window from firehold-comp's list, named the Firehold way.
pub fn name_window(w: &Value, library: &[Game]) -> Win {
    let s = |k: &str| w.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
    let b = |k: &str| w.get(k).and_then(|x| x.as_bool()).unwrap_or(false);
    let app_id = s("appId");
    let title = s("title");
    let mut win = Win {
        id: s("id"),
        app_id: app_id.clone(),
        title: title.clone(),
        minimized: b("minimized"),
        fullscreen: b("fullscreen"),
        activated: b("activated"),
        pid: w.get("pid").and_then(|p| p.as_i64()).unwrap_or(0),
        ..Win::default()
    };
    if let Some(g) = game_for(&app_id, &title, library) {
        win.game = true;
        win.game_id = g.id.clone();
        win.source = g.source.clone();
        win.name = g.title.clone();
        let state = if win.minimized { if g.source == "emu" { "suspended" } else { "paused" } } else { "running" };
        win.sub = format!("{} · {state}", if g.source == "emu" { "emulator" } else { "game" });
        return win;
    }
    let app = app_name(&app_id);
    // "Pictures › Trips — Hearth Files": the place, then the app.
    let (name, sub) = match title.rsplit_once(" — ").or_else(|| title.rsplit_once(" - ")) {
        Some((place, tail)) if tail.trim().eq_ignore_ascii_case(&app) || app_id.is_empty() => (tail.trim().to_string(), place.trim().to_string()),
        _ if title.trim().eq_ignore_ascii_case(&app) || title.trim().is_empty() => (app.clone(), String::new()),
        _ => (app.clone(), title.trim().to_string()),
    };
    win.name = name;
    win.sub = sub.chars().take(80).collect();
    win
}

/// The game the hero offers: the one that's open, or the latest played that's installed.
pub fn continue_game<'a>(library: &'a [Game], open: &[Win]) -> Option<&'a Game> {
    if let Some(w) = open.iter().find(|w| w.game) {
        if let Some(g) = library.iter().find(|g| g.id == w.game_id) {
            return Some(g);
        }
    }
    library.iter().filter(|g| g.installed && g.last_played > 0).max_by_key(|g| g.last_played)
}

/// The Recent row: up to `n` played games and apps after the hero's, latest first.
pub fn recent<'a>(library: &'a [Game], hero: Option<&Game>, n: usize) -> Vec<&'a Game> {
    let mut v: Vec<&Game> = library.iter().filter(|g| g.last_played > 0 && Some(g.id.as_str()) != hero.map(|h| h.id.as_str())).collect();
    v.sort_by(|a, b| b.last_played.cmp(&a.last_played));
    v.truncate(n);
    v
}

/// The hero's line under the name (mockup 1): chapter · where it's saved · how it runs.
pub fn hero_line(g: &Game) -> String {
    let mut parts = Vec::new();
    if !g.chapter.is_empty() {
        parts.push(g.chapter.clone());
    }
    parts.push(if g.cloud_saves { "Saved on this PC and in the cloud".to_string() } else { "Saved on this PC".to_string() });
    parts.push("runs natively, no Game Mode".to_string());
    parts.join(" · ")
}

/// A tile's colour (the stand-in for art Hearth doesn't have): the mockups' eight, picked by the id.
pub const TILE_COLOURS: [(u8, u8, u8); 8] =
    [(0x3a, 0x4a, 0x52), (0x2c, 0x2a, 0x3e), (0x5a, 0x3a, 0x2c), (0x2e, 0x3e, 0x34), (0x6b, 0x4a, 0x2a), (0x32, 0x28, 0x22), (0x4a, 0x34, 0x44), (0x5a, 0x2c, 0x1c)];
pub fn tile_colour(id: &str) -> (u8, u8, u8) {
    let h = id.bytes().fold(5381u32, |h, b| h.wrapping_mul(33) ^ b as u32);
    TILE_COLOURS[(h % TILE_COLOURS.len() as u32) as usize]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn lib() -> Vec<Game> {
        let raw = json!([
            {"id": "steam:1000", "title": "Ember Trails", "source": "steam", "installed": true, "state": "installed", "lastPlayed": 1_000_000_000_000i64},
            {"id": "gog:2001", "title": "Night Harbor", "source": "gog", "installed": true, "lastPlayed": 900_000_000_000i64},
            {"id": "epic:hollow", "title": "Hollow Deep", "source": "epic", "installed": true, "update": true, "lastPlayed": 800_000_000_000i64},
            {"id": "manual:quad", "title": "Quad Mech", "source": "manual", "exe": "/home/x/projects/quad-mech/build/q.x86_64", "lastPlayed": 700_000_000_000i64},
            {"id": "steam:1003", "title": "Cloud Lanes", "source": "steam", "installed": false, "state": "notInstalled"},
            {"id": "steam:1004", "title": "Frost Keep", "source": "steam", "installed": false},
            {"id": "bad id; rm", "title": "x"},
        ]);
        parse_library(&raw, &|id, _| id == "steam:1003")
    }

    #[test]
    fn tiles_say_what_the_mockup_says() {
        let l = lib();
        assert_eq!(l.len(), 6, "the bad id is left out");
        assert_eq!(l[0].store(), "Steam");
        assert_eq!(l[0].state(true), "Playing");
        assert_eq!(l[2].state(false), "Update ready");
        assert_eq!(l[3].state(false), "Dev build");
        assert_eq!(l[3].store(), "Your game");
        assert_eq!(l[4].state(false), "Cloud saves");
        assert_eq!(l[5].state(false), "Not installed");
        let c = counts(&l);
        assert_eq!(c[0], ("all".into(), "All".into(), 6));
        assert_eq!(c[1].2, 4);
        assert_eq!(c.iter().find(|x| x.0 == "steam").unwrap().2, 3);
        assert_eq!(c.iter().find(|x| x.0 == "yours").unwrap().2, 1);
    }

    #[test]
    fn windows_are_named_like_firehold() {
        let l = lib();
        let w = name_window(&json!({"id": "7", "appId": "org.hearth.files", "title": "Pictures › Trips — Hearth Files"}), &l);
        assert_eq!((w.name.as_str(), w.sub.as_str(), w.game), ("Hearth Files", "Pictures › Trips", false));
        let w = name_window(&json!({"id": "8", "appId": "firefox", "title": "3 tabs — Firefox"}), &l);
        assert_eq!((w.name.as_str(), w.sub.as_str()), ("Firefox", "3 tabs"));
        let w = name_window(&json!({"id": "9", "appId": "firehold-game", "title": "Ember Trails", "minimized": true}), &l);
        assert!(w.game);
        assert_eq!((w.name.as_str(), w.sub.as_str(), w.game_id.as_str()), ("Ember Trails", "game · paused", "steam:1000"));
        let w = name_window(&json!({"id": "10", "appId": "steam_app_1000", "title": "whatever"}), &l);
        assert_eq!(w.game_id, "steam:1000");
        let w = name_window(&json!({"id": "11", "appId": "steam", "title": "Steam"}), &l);
        assert_eq!((w.name.as_str(), w.sub.as_str()), ("Steam", ""));
        assert_eq!(dock_letter("Firefox", false), "W");
        assert_eq!(dock_letter("Discord", false), "D");
        assert_eq!(dock_letter("Ember Trails", true), "▶");
    }

    #[test]
    fn hero_recent_and_order() {
        let l = lib();
        let hero = continue_game(&l, &[]).unwrap();
        assert_eq!(hero.title, "Ember Trails");
        let r = recent(&l, Some(hero), 7);
        assert_eq!(r.iter().map(|g| g.title.as_str()).collect::<Vec<_>>(), ["Night Harbor", "Hollow Deep", "Quad Mech"]);
        let o = library_order(&l);
        assert_eq!(o.last().unwrap().title, "Frost Keep");
        assert_eq!(hero_line(hero), "Saved on this PC · runs natively, no Game Mode");
        assert_eq!(ago(1_000_000_000_000 - 41 * 60000, 1_000_000_000_000), "41 min ago");
        assert_eq!(playing_for(72 * 60), "Playing 1 h 12 min");
        assert!(filter_ok(&l[3], "yours") && !filter_ok(&l[3], "steam"));
    }

    #[test]
    fn winapps() {
        let w = parse_winapps(&json!({"apps": [{"id": "old-office", "name": "Old Office", "last": {"at": 1_700_000_000}}, {"id": "../x"}]}));
        assert_eq!(w.len(), 1);
        assert_eq!(w[0].store(), "Windows app");
        assert_eq!(w[0].last_played, 1_700_000_000_000);
    }
}
