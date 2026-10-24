//! The games this device can send, from every store, and where a game copied here goes.
//!
//! Sending side: each game is a list of roots (what gets copied) plus a little metadata:
//! - Steam: steamapps/common/<installdir>, its Proton prefix (steamapps/compatdata/<appid>) if asked, and the
//!   appmanifest's text, which the receiving side writes last, so Steam only sees the game once it's all there.
//!   Steam's own files are only read (libraryfolders.vdf, appmanifest_*.acf); Hearth Link never signs in to Steam,
//!   never touches its sign-in files or steamclient.so.
//! - Epic (legendary) and GOG (gogdl), installed by Hearth: the install folder and Hearth's prefix for the game.
//! - Beacon: its games (set up in Beacon) and the games added by hand in Beacon.
//! - Xbox games from Xodus Nexus: a folder in Nexus's games folder.
//! - Every game: Hearth's controller layout for it (config/input), written on the other side too.
//!
//! Receiving side: a Steam game goes into one of this device's Steam libraries; everything else into a Games
//! folder on a drive the person picks. Then each store is told: Steam reads the appmanifest (restart Steam to see
//! it), Epic gets `legendary import`, GOG is added to Hearth's own list of GOG installs, Beacon gets `set-path`.

use serde_json::{json, Value};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Root {
    /// "install", "prefix" (Proton/Wine prefix: saves and settings).
    pub kind: &'static str,
    pub path: PathBuf,
    /// Parts of it not worth copying (Steam's shader cache is rebuilt by Steam).
    pub skip: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub struct LocalGame {
    pub id: String,
    pub title: String,
    pub store: &'static str,
    /// From the store when it says (Steam's SizeOnDisk), else 0 until the manifest is made.
    pub size: u64,
    pub roots: Vec<Root>,
    /// What the other side needs to put it in place: Steam's appmanifest text and installdir, a GOG/Epic id...
    pub meta: Value,
}

impl LocalGame {
    pub fn to_json(&self) -> Value {
        json!({"id": self.id, "title": self.title, "store": self.store, "size": self.size,
               "hasPrefix": self.roots.iter().any(|r| r.kind == "prefix" && r.path.is_dir())})
    }
}

/// Tools and runtimes Steam installs like games: never offered.
fn steam_tool(name: &str, appid: &str) -> bool {
    let n = name.to_lowercase();
    appid == "228980" || n.starts_with("proton") || n.starts_with("steam linux runtime") || n.starts_with("steamworks common")
}

/// Installed Steam games in every library: (library, appid, name, installdir, size, acf path).
pub fn steam_installed(home: &Path) -> Vec<(PathBuf, String, String, String, u64, PathBuf)> {
    let mut out = Vec::new();
    let Some(steam) = hearth_core::steam::steam_path(home) else { return out };
    for lib in hearth_core::steam::libraries(&steam) {
        let apps = lib.join("steamapps");
        let Ok(rd) = std::fs::read_dir(&apps) else { continue };
        for e in rd.flatten() {
            let n = e.file_name().to_string_lossy().to_string();
            if !(n.starts_with("appmanifest_") && n.ends_with(".acf")) { continue; }
            let Some(kv) = hearth_core::vdf::load(&e.path()) else { continue };
            let appid = kv.str(&["AppState", "appid"]).unwrap_or("").to_string();
            let name = kv.str(&["AppState", "name"]).unwrap_or("").to_string();
            let dir = kv.str(&["AppState", "installdir"]).unwrap_or("").to_string();
            let flags = kv.num(&["AppState", "StateFlags"]);
            // 4: fully installed (no update or download half done).
            if appid.is_empty() || !appid.chars().all(|c| c.is_ascii_digit()) || dir.is_empty() || flags != 4 || steam_tool(&name, &appid) { continue; }
            if crate::manifest::safe_rel(&dir).is_none() || dir.contains('/') { continue; }
            if !apps.join("common").join(&dir).is_dir() { continue; }
            let size = kv.num(&["AppState", "SizeOnDisk"]).max(0) as u64;
            out.push((lib.clone(), appid, name, dir, size, e.path()));
        }
    }
    out
}

/// Every game this device can send.
pub fn list(home: &Path) -> Vec<LocalGame> {
    let mut out = Vec::new();
    for (lib, appid, name, dir, size, acf) in steam_installed(home) {
        let acf_text = std::fs::read_to_string(&acf).unwrap_or_default();
        out.push(LocalGame {
            id: format!("steam:{appid}"), title: if name.is_empty() { dir.clone() } else { name }, store: "steam", size,
            roots: vec![
                Root { kind: "install", path: lib.join("steamapps/common").join(&dir), skip: vec![] },
                Root { kind: "prefix", path: lib.join("steamapps/compatdata").join(&appid), skip: vec![] },
            ],
            meta: json!({"appid": appid, "installdir": dir, "acf": acf_text}),
        });
    }
    for g in hearth_core::stores::epic::installed(&hearth_core::owned::Owned::default()) {
        let Some(dir) = g.install_path.clone() else { continue };
        let app = g.id.trim_start_matches("epic:").to_string();
        out.push(LocalGame {
            id: g.id.clone(), title: g.title.clone(), store: "epic", size: g.size_bytes.unwrap_or(0),
            roots: vec![Root { kind: "install", path: dir.clone(), skip: vec![] }, Root { kind: "prefix", path: hearth_core::stores::prefix_of("epic", &app), skip: vec![] }],
            meta: json!({"app": app, "folder": dir.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| app.clone())}),
        });
    }
    let lib = std::fs::read(hearth_core::paths::data_dir().join("library.json")).ok().and_then(|b| serde_json::from_slice::<Value>(&b).ok()).unwrap_or(Value::Null);
    for g in hearth_core::stores::gog::installed(&lib["gogInstalls"], &hearth_core::owned::Owned::default()) {
        let Some(dir) = g.install_path.clone() else { continue };
        let id = g.id.trim_start_matches("gog:").to_string();
        out.push(LocalGame {
            id: g.id.clone(), title: g.title.clone(), store: "gog", size: 0,
            roots: vec![Root { kind: "install", path: dir.clone(), skip: vec![] }, Root { kind: "prefix", path: hearth_core::stores::prefix_of("gog", &id), skip: vec![] }],
            meta: json!({"gogId": id, "folder": dir.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| id.clone())}),
        });
    }
    // Xbox games from Xodus Nexus: each folder in its games folder.
    if let Some(folder) = hearth_core::xodus::games_folder() {
        for e in std::fs::read_dir(&folder).into_iter().flatten().flatten() {
            let n = e.file_name().to_string_lossy().to_string();
            if !e.path().is_dir() || n.starts_with('.') { continue; }
            out.push(LocalGame {
                id: format!("xodus:{n}"), title: n.clone(), store: "xodus", size: 0,
                roots: vec![Root { kind: "install", path: e.path(), skip: vec![] }],
                meta: json!({"folder": n}),
            });
        }
    }
    out
}

/// Beacon's games: asks Beacon (its own command-line tool, with time limits), so only when someone asks for the list.
pub fn list_beacon(home: &Path) -> Vec<LocalGame> {
    let snap = hearth_core::beacon::load(home);
    let mut out = Vec::new();
    for g in snap.games.iter().filter(|g| g.installed) {
        let Some(dir) = g.path.as_deref().map(PathBuf::from).filter(|p| p.is_dir()) else { continue };
        let folder = dir.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| g.id.replace(':', "-"));
        // A game added by hand: its entry in Beacon's list, so the other side can add it there too.
        let custom = if g.custom { beacon_custom_entry(home, g.id.trim_start_matches("custom:")) } else { Value::Null };
        out.push(LocalGame {
            id: format!("beacon:{}", g.id), title: g.title.clone(), store: "beacon", size: 0,
            roots: vec![Root { kind: "install", path: dir.clone(), skip: vec![] }],
            meta: json!({"game": g.id, "folder": folder, "from": dir, "custom": custom}),
        });
    }
    out
}

pub fn beacon_custom_file(home: &Path) -> PathBuf {
    let cfg = std::env::var_os("BEACON_CONFIG").map(PathBuf::from)
        .unwrap_or_else(|| std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| home.join(".config")).join("beacon/config.json"));
    let native = cfg.with_file_name("custom-games.json");
    let flatpak = home.join(".var/app").join(hearth_core::beacon::FLATPAK_ID).join("config/beacon/custom-games.json");
    if !native.exists() && flatpak.exists() { flatpak } else { native }
}

fn beacon_custom_entry(home: &Path, cid: &str) -> Value {
    let v: Value = std::fs::read(beacon_custom_file(home)).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null);
    let list = if v.is_array() { v.clone() } else { v["games"].clone() };
    list.as_array().and_then(|a| a.iter().find(|g| g["id"] == cid).cloned()).unwrap_or(Value::Null)
}

/// One game by id (Beacon's are only looked up when the id is Beacon's).
pub fn find(home: &Path, id: &str) -> Option<LocalGame> {
    if id.starts_with("beacon:") { return list_beacon(home).into_iter().find(|g| g.id == id); }
    list(home).into_iter().find(|g| g.id == id)
}

/// Where a copy can go on this device, for a game of `store`: Steam libraries for Steam games, Games folders on
/// each drive otherwise. [{id, label, path, free}]
pub fn places(home: &Path, store: &str) -> Vec<Value> {
    let mut out = Vec::new();
    if store == "steam" {
        if let Some(steam) = hearth_core::steam::steam_path(home) {
            for (i, lib) in hearth_core::steam::libraries(&steam).into_iter().enumerate() {
                let label = if i == 0 { "Steam library (this device)".to_string() } else { format!("Steam library on {}", lib.display()) };
                out.push(json!({"id": lib, "label": label, "path": lib, "free": crate::free_space(&lib).unwrap_or(0)}));
            }
        }
        return out;
    }
    if store == "xodus" {
        let p = hearth_core::xodus::games_folder().map(PathBuf::from).unwrap_or_else(|| home.join("Games/Xbox"));
        out.push(json!({"id": p, "label": "Xodus Nexus games folder", "path": p, "free": crate::free_space(&p).unwrap_or(0)}));
        return out;
    }
    let g = home.join("Games");
    out.push(json!({"id": g, "label": "This device", "path": g, "free": crate::free_space(&g).unwrap_or(0)}));
    let user = std::env::var("USER").unwrap_or_default();
    for base in [PathBuf::from("/run/media").join(&user), PathBuf::from("/media").join(&user)] {
        for d in std::fs::read_dir(&base).into_iter().flatten().flatten() {
            if !d.path().is_dir() { continue; }
            let p = d.path().join("Games");
            out.push(json!({"id": p, "label": d.file_name().to_string_lossy(), "path": p, "free": crate::free_space(&d.path()).unwrap_or(0)}));
        }
    }
    out
}

/// A place the person picked, checked against the ones offered (nothing else can be written to).
pub fn check_place(home: &Path, store: &str, chosen: Option<&str>) -> Result<PathBuf, String> {
    let ps = places(home, store);
    if ps.is_empty() {
        return Err(if store == "steam" { "Steam isn't set up on this device: install Steam and open it once, then try again".into() } else { "No place to put it".into() });
    }
    match chosen.filter(|c| !c.is_empty()) {
        None => Ok(PathBuf::from(ps[0]["path"].as_str().unwrap_or(""))),
        Some(c) => ps.iter().find(|p| p["id"].as_str() == Some(c)).and_then(|p| p["path"].as_str()).map(PathBuf::from)
            .ok_or_else(|| "Pick one of the places offered".into()),
    }
}

/// A name for a folder from the network: one plain part.
pub fn safe_folder(s: &str) -> Option<String> {
    let s = s.trim();
    (crate::manifest::safe_rel(s).is_some() && !s.contains('/') && !s.starts_with('.')).then(|| s.to_string())
}

/// Where each root of a game from another device goes here (None: that root isn't copied).
pub fn destinations(game: &Value, roots: &[Value], place: &Path) -> Result<Vec<Option<PathBuf>>, String> {
    let store = game["store"].as_str().unwrap_or("");
    let meta = &game["meta"];
    let folder = |k: &str| meta[k].as_str().and_then(safe_folder).ok_or_else(|| "That game's folder name can't be used here".to_string());
    let mut out = Vec::new();
    for r in roots {
        let kind = r["kind"].as_str().unwrap_or("");
        let d = match (store, kind) {
            ("steam", "install") => Some(place.join("steamapps/common").join(folder("installdir")?)),
            ("steam", "prefix") => {
                let appid = meta["appid"].as_str().filter(|a| !a.is_empty() && a.chars().all(|c| c.is_ascii_digit())).ok_or("Not a Steam game")?;
                Some(place.join("steamapps/compatdata").join(appid))
            }
            ("epic", "install") => Some(place.join("Epic").join(folder("folder")?)),
            ("epic", "prefix") => Some(hearth_core::stores::prefix_of("epic", meta["app"].as_str().unwrap_or(""))),
            ("gog", "install") => Some(place.join("GOG").join(folder("folder")?)),
            ("gog", "prefix") => Some(hearth_core::stores::prefix_of("gog", meta["gogId"].as_str().unwrap_or(""))),
            ("beacon", "install") => Some(place.join("Beacon").join(folder("folder")?)),
            ("xodus", "install") => Some(place.join(folder("folder")?)),
            _ => None,
        };
        out.push(d);
    }
    Ok(out)
}

/// After every file is in place: tell the store. Returns a note for the person ("Restart Steam to see it").
pub fn finalize(home: &Path, game: &Value, dests: &[Option<PathBuf>], place: &Path) -> Result<String, String> {
    let store = game["store"].as_str().unwrap_or("");
    let meta = &game["meta"];
    let install = dests.first().cloned().flatten().ok_or("No install folder")?;
    let note = match store {
        "steam" => {
            let appid = meta["appid"].as_str().unwrap_or("");
            let acf = meta["acf"].as_str().unwrap_or("");
            if acf.is_empty() || appid.is_empty() || !appid.chars().all(|c| c.is_ascii_digit()) { return Err("The game's Steam details didn't come with it".into()); }
            let text = steam_manifest_for_here(acf);
            crate::write_atomic(&place.join("steamapps").join(format!("appmanifest_{appid}.acf")), text.as_bytes())?;
            if steam_running() { "Restart Steam to see it (Steam reads new games when it starts)".to_string() } else { "Steam will see it the next time it starts".to_string() }
        }
        "epic" => {
            let app = meta["app"].as_str().unwrap_or("");
            match hearth_core::stores::epic::import_game(app, &install.to_string_lossy()) {
                Ok(()) => "It's in your Epic games".to_string(),
                Err(e) => format!("Copied. Epic couldn't add it yet ({e}): sign in to Epic once on this device, then use Settings › Import"),
            }
        }
        "gog" => {
            let id = meta["gogId"].as_str().unwrap_or("");
            crate::sync::inbox_add(&crate::sync::inbox_file(), &json!({"gogInstalls": {id: install}}))?;
            "It's in your GOG games".to_string()
        }
        "beacon" => {
            let g = meta["game"].as_str().unwrap_or("");
            if let Some(cid) = g.strip_prefix("custom:") {
                add_beacon_custom(home, cid, &meta["custom"], meta["from"].as_str().unwrap_or(""), &install)?;
                "It's in Beacon's Other Games".to_string()
            } else {
                let snap = hearth_core::beacon::load(home);
                match snap.cli {
                    Some(cli) if hearth_core::beacon::known_game(g) => {
                        let args = vec!["set-path".to_string(), g.to_string(), install.to_string_lossy().to_string()];
                        match hearth_core::beacon::run_json(&cli, &args, None, std::time::Duration::from_secs(60), false, &mut |_, _| {}) {
                            Ok(_) => "Beacon knows where it is".to_string(),
                            Err(e) => format!("Copied. Tell Beacon where it is (Game setup › Locate): {e}"),
                        }
                    }
                    _ => "Copied. Beacon isn't on this device yet: add Beacon, then Game setup › Locate".to_string(),
                }
            }
        }
        "xodus" => "Copied into Xodus Nexus's games folder: open Xodus Nexus to finish".to_string(),
        _ => "Copied".to_string(),
    };
    // Hearth's controller layout for the game.
    if let (Some(id), Some(layout)) = (game["id"].as_str(), meta.get("input").filter(|v| v.is_object())) {
        if let Ok(f) = hearth_core::input_map::file_name(id) {
            let _ = crate::write_atomic(&hearth_core::input_map::dir().join(f), serde_json::to_string_pretty(layout).unwrap().as_bytes());
        }
    }
    Ok(note)
}

/// Steam's appmanifest as it should be on this device: fully installed, nothing left to download.
pub fn steam_manifest_for_here(acf: &str) -> String {
    let mut out = String::new();
    for line in acf.lines() {
        let t = line.trim_start();
        let indent = &line[..line.len() - t.len()];
        let key = t.split('"').nth(1).unwrap_or("");
        match key {
            "StateFlags" => out.push_str(&format!("{indent}\"StateFlags\"\t\t\"4\"")),
            "BytesToDownload" | "BytesToStage" => out.push_str(&format!("{indent}\"{key}\"\t\t\"0\"")),
            _ => out.push_str(line),
        }
        out.push('\n');
    }
    out
}

pub fn steam_running() -> bool {
    std::fs::read_dir("/proc").into_iter().flatten().flatten().any(|e| {
        std::fs::read(e.path().join("comm")).map(|c| c.starts_with(b"steam\n") || c.starts_with(b"steamwebhelper")).unwrap_or(false)
    })
}

/// A game added by hand in Beacon, copied here: its entry in Beacon's list, with paths in the old folder moved to
/// the new one.
fn add_beacon_custom(home: &Path, cid: &str, entry: &Value, from: &str, to: &Path) -> Result<(), String> {
    if !entry.is_object() || !hearth_core::beacon::valid_custom_id(cid) { return Ok(()); }
    let file = beacon_custom_file(home);
    let mut v: Value = std::fs::read(&file).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(json!({"games": []}));
    let to_s = to.to_string_lossy().to_string();
    let mut e = entry.clone();
    if let Some(m) = e.as_object_mut() {
        for (_, x) in m.iter_mut() {
            if let Some(s) = x.as_str() { if !from.is_empty() && s.starts_with(from) { *x = json!(format!("{to_s}{}", &s[from.len()..])); } }
        }
    }
    let list = if v.is_array() { &mut v } else { v.as_object_mut().map(|o| o.entry("games").or_insert(json!([]))).ok_or("Beacon's list isn't readable")? };
    if let Some(a) = list.as_array_mut() { a.retain(|g| g["id"] != cid); a.push(e); }
    crate::write_atomic(&file, serde_json::to_string_pretty(&v).unwrap().as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fake_steam(home: &Path) -> PathBuf {
        let s = home.join(".local/share/Steam");
        std::fs::create_dir_all(s.join("steamapps/common/Portal 2/bin")).unwrap();
        std::fs::create_dir_all(s.join("config")).unwrap();
        std::fs::write(s.join("steamapps/common/Portal 2/bin/x.bin"), "x").unwrap();
        std::fs::write(s.join("steamapps/libraryfolders.vdf"), format!("\"libraryfolders\" {{ \"0\" {{ \"path\" \"{}\" }} }}", s.display())).unwrap();
        std::fs::write(s.join("steamapps/appmanifest_620.acf"), "\"AppState\"\n{\n\t\"appid\"\t\t\"620\"\n\t\"name\"\t\t\"Portal 2\"\n\t\"StateFlags\"\t\t\"4\"\n\t\"installdir\"\t\t\"Portal 2\"\n\t\"SizeOnDisk\"\t\t\"1000\"\n}\n").unwrap();
        std::fs::write(s.join("steamapps/appmanifest_1493710.acf"), "\"AppState\" { \"appid\" \"1493710\" \"name\" \"Proton Experimental\" \"StateFlags\" \"4\" \"installdir\" \"Proton - Experimental\" }").unwrap();
        std::fs::write(s.join("steamapps/appmanifest_400.acf"), "\"AppState\" { \"appid\" \"400\" \"name\" \"Portal\" \"StateFlags\" \"6\" \"installdir\" \"Portal\" }").unwrap();
        s
    }
    #[test]
    fn steam_games_and_where_they_land() {
        let d = tempfile::tempdir().unwrap();
        let s = fake_steam(d.path());
        let got = steam_installed(d.path());
        assert_eq!(got.len(), 1, "only the fully installed game, no tools: {got:?}");
        assert_eq!(got[0].1, "620");
        let game = json!({"id": "steam:620", "store": "steam", "meta": {"appid": "620", "installdir": "Portal 2", "acf": "\"AppState\"\n{\n\t\"StateFlags\"\t\t\"1026\"\n\t\"BytesToDownload\"\t\t\"5\"\n}\n"}});
        let roots = vec![json!({"kind": "install"}), json!({"kind": "prefix"})];
        let dests = destinations(&game, &roots, &s).unwrap();
        assert_eq!(dests[0].as_deref(), Some(s.join("steamapps/common/Portal 2").as_path()));
        assert_eq!(dests[1].as_deref(), Some(s.join("steamapps/compatdata/620").as_path()));
        finalize(d.path(), &game, &dests, &s).unwrap();
        let acf = std::fs::read_to_string(s.join("steamapps/appmanifest_620.acf")).unwrap();
        assert!(acf.contains("\"StateFlags\"\t\t\"4\"") && acf.contains("\"BytesToDownload\"\t\t\"0\""), "{acf}");
    }
    #[test]
    fn folder_names_from_the_network_are_checked() {
        let game = json!({"store": "steam", "meta": {"appid": "620", "installdir": "../../.ssh"}});
        assert!(destinations(&game, &[json!({"kind": "install"})], Path::new("/x")).is_err());
        let game = json!({"store": "gog", "meta": {"gogId": "1", "folder": "a/b"}});
        assert!(destinations(&game, &[json!({"kind": "install"})], Path::new("/x")).is_err());
        assert_eq!(safe_folder("Witcher 3"), Some("Witcher 3".into()));
        assert_eq!(safe_folder(".hidden"), None);
    }
    #[test]
    fn places_are_only_the_ones_offered() {
        let d = tempfile::tempdir().unwrap();
        assert!(check_place(d.path(), "gog", Some("/etc")).is_err());
        assert_eq!(check_place(d.path(), "gog", None).unwrap(), d.path().join("Games"));
        assert!(check_place(d.path(), "steam", None).is_err(), "no Steam here");
    }
}
