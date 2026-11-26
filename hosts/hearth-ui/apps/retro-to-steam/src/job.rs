//! Adding to Steam (and taking out what was added), on a thread of its own: Steam closed first if it's open (it's
//! asked first), each game's art downloaded, shortcuts.vdf written (the old one kept as .bak; settings_file's atomic
//! write, checked before it's kept), the art copied into config/grid, what was added remembered, and Steam opened
//! again if it was open. Steam's games and anything Retro to Steam didn't add are never touched.

use crate::art::{self, Cand, Kind};
use crate::steam;
use crate::vdf::{self, Node};
use crate::{rom, App};
use hearth_ui::settings_file;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct One {
    pub rom: String,
    pub name: String,
    pub system: String,
    pub stem: String,
    pub tags: Vec<String>,
    /// each kind: the person's own picture, else the picked choice
    pub art: Vec<(Kind, Option<PathBuf>, Option<Cand>)>,
}

#[derive(Debug, Clone)]
pub struct Plan {
    pub user: String,
    pub user_name: String,
    pub add: Vec<One>,
    pub remove: bool,
    /// the ids Retro to Steam added before (prefs "added")
    pub added: Vec<u32>,
}

pub fn plan(a: &App, remove: bool) -> Plan {
    let mut add = Vec::new();
    if !remove {
        for g in a.included() {
            if a.in_steam(g) {
                continue;
            }
            let mut tags = vec![a.collection_of(g)];
            if a.shelf_of(g) && !tags.iter().any(|t| t == "Retro") {
                tags.push("Retro".into());
            }
            tags.retain(|t| !t.is_empty() && t != "None");
            let art = Kind::ALL
                .iter()
                .map(|k| {
                    let c = a.cands(g, *k);
                    let pick = c.get(a.pick_of(g, *k).min(c.len().saturating_sub(1))).cloned();
                    (*k, a.own(g, *k), pick)
                })
                .collect();
            // two games of one name (Tetris on two consoles) would be one shortcut in Steam: the console goes in the name
            let mut name = a.name_of(g);
            if add.iter().any(|o: &One| o.name == name) || a.existing.iter().any(|(n, _)| *n == name.to_lowercase()) {
                name = format!("{name} ({})", hearth_retro::library::system(g.system).map(|s| s.name).unwrap_or(g.system));
            }
            add.push(One { rom: rom(g), name, system: g.system.to_string(), stem: g.stem.clone(), tags, art });
        }
    }
    let added = a.prefs.get("added").and_then(|v| v.get(&a.user)).and_then(Value::as_array).map(|v| v.iter().filter_map(|x| x.as_u64().map(|n| n as u32)).collect()).unwrap_or_default();
    let user_name = a.users.iter().find(|u| u.id == a.user).map(|u| u.name.clone()).unwrap_or_default();
    Plan { user: a.user.clone(), user_name, add, remove, added }
}

/// What it would do, in words (Preview only: nothing is written).
pub fn describe(p: &Plan) -> String {
    let file = steam::shortcuts_path(&p.user).map(|x| x.display().to_string()).unwrap_or_default();
    let grid = steam::grid_dir(&p.user).map(|x| x.display().to_string()).unwrap_or_default();
    if p.remove {
        return format!("Would take out the {} games Retro to Steam added to {} and their art. Nothing else in Steam is touched.", p.added.len(), file);
    }
    let count = |k: Kind| p.add.iter().filter(|o| o.art.iter().any(|(kk, own, c)| *kk == k && (own.is_some() || c.is_some()))).count();
    let names: Vec<String> = p.add.iter().take(8).map(|o| o.name.clone()).collect();
    format!(
        "Would add {} games to {}'s Steam library ({}; the one there now is kept as shortcuts.vdf.bak).\nArt into {}: {} grid, {} wide, {} hero, {} logo, {} icon.\nEach starts with hearth-retro-app --play, in the emulator Hearth Retro set up.\nFirst: {}{}\nNothing has been written.",
        p.add.len(),
        if p.user_name.is_empty() { "this PC" } else { &p.user_name },
        file,
        grid,
        count(Kind::Grid),
        count(Kind::Wide),
        count(Kind::Hero),
        count(Kind::Logo),
        count(Kind::Icon),
        names.join(", "),
        if p.add.len() > 8 { "…" } else { "" }
    )
}

fn say(progress: &Arc<Mutex<String>>, s: String) {
    if let Ok(mut g) = progress.lock() {
        *g = s;
    }
}

fn entry(o: &One, exe: &str, icon: &str) -> (u32, Node) {
    let id = vdf::shortcut_id(exe, &o.name);
    let start = PathBuf::from(exe.trim_matches('"')).parent().map(|p| format!("\"{}\"", p.display())).unwrap_or_default();
    let tags = Node::Map(o.tags.iter().enumerate().map(|(i, t)| (i.to_string(), Node::Str(t.clone()))).collect());
    let n = Node::Map(vec![
        ("appid".into(), Node::Int(id as i32)),
        ("AppName".into(), Node::Str(o.name.clone())),
        ("Exe".into(), Node::Str(exe.to_string())),
        ("StartDir".into(), Node::Str(start)),
        ("icon".into(), Node::Str(icon.to_string())),
        ("ShortcutPath".into(), Node::Str(String::new())),
        ("LaunchOptions".into(), Node::Str(format!("--play \"{}\"", o.rom.replace('"', "\\\"")))),
        ("IsHidden".into(), Node::Int(0)),
        ("AllowDesktopConfig".into(), Node::Int(1)),
        ("AllowOverlay".into(), Node::Int(1)),
        ("OpenVR".into(), Node::Int(0)),
        ("Devkit".into(), Node::Int(0)),
        ("DevkitGameID".into(), Node::Str(String::new())),
        ("DevkitOverrideAppID".into(), Node::Int(0)),
        ("LastPlayTime".into(), Node::Int(0)),
        ("FlatpakAppID".into(), Node::Str(String::new())),
        ("tags".into(), tags),
    ]);
    (id, n)
}

fn entry_id(n: &Node) -> Option<u32> {
    n.int("appid").map(|i| i as u32)
}

pub fn run(p: Plan, progress: Arc<Mutex<String>>) -> Result<String, String> {
    let file = steam::shortcuts_path(&p.user).ok_or("Steam isn't on this PC")?;
    let grid = steam::grid_dir(&p.user).ok_or("Steam isn't on this PC")?;
    // read first: a list that can't be read is never written over
    let mut root = steam::read_shortcuts(&p.user)?;
    let was_open = steam::running();
    if was_open {
        say(&progress, "Closing Steam…".into());
        steam::close_and_wait()?;
        // Steam may have written its list as it closed
        root = steam::read_shortcuts(&p.user)?;
    }
    let result = (|| -> Result<String, String> {
        let list = match &mut root {
            Node::Map(top) => {
                if !top.iter().any(|(k, _)| k == "shortcuts") {
                    top.push(("shortcuts".into(), Node::Map(vec![])));
                }
                top.iter_mut().find(|(k, _)| k == "shortcuts").map(|x| &mut x.1).ok_or("no list")?
            }
            _ => return Err("Steam's list isn't a list".into()),
        };
        let Node::Map(entries) = list else { return Err("Steam's list isn't a list".into()) };
        let mut kept: Vec<Node> = entries.drain(..).map(|x| x.1).collect();
        let mut added: Vec<u32> = Vec::new();
        let mut removed = 0usize;
        if p.remove {
            let before = kept.len();
            kept.retain(|n| !entry_id(n).map(|id| p.added.contains(&id)).unwrap_or(false));
            removed = before - kept.len();
            for id in &p.added {
                for k in Kind::ALL {
                    let _ = std::fs::remove_file(grid.join(k.steam_file(*id)));
                }
            }
        } else {
            let exe = steam::retro_exe();
            let _ = std::fs::create_dir_all(&grid);
            let n = p.add.len();
            for (i, o) in p.add.iter().enumerate() {
                say(&progress, format!("Getting art {} of {n} · {}", i + 1, o.name));
                let id = vdf::shortcut_id(&exe, &o.name);
                let mut icon = String::new();
                for (k, own, cand) in &o.art {
                    let src = match (own, cand) {
                        (Some(p), _) => Some(p.clone()),
                        (None, Some(c)) => art::fetch(c).ok().filter(|f| f.is_file()),
                        _ => None,
                    };
                    let Some(src) = src else { continue };
                    let dst = grid.join(k.steam_file(id));
                    if std::fs::copy(&src, &dst).is_ok() && *k == Kind::Icon {
                        icon = dst.display().to_string();
                    }
                    // Hearth Retro's shelves show the same box art
                    if *k == Kind::Grid {
                        let r = hearth_retro::retro_dir().join("art").join(&o.system);
                        let _ = std::fs::create_dir_all(&r);
                        let _ = std::fs::copy(&src, r.join(format!("{}.png", o.stem)));
                    }
                }
                let (id, node) = entry(o, &exe, &icon);
                kept.retain(|x| entry_id(x) != Some(id));
                kept.push(node);
                added.push(id);
            }
            // Hearth Retro itself, so Game Mode has its shelves too (full screen, the controller): once
            if !p.add.is_empty() && !kept.iter().any(|n| n.str("AppName") == "Hearth Retro") {
                let id = vdf::shortcut_id(&exe, "Hearth Retro");
                let mut node = entry(&One { rom: String::new(), name: "Hearth Retro".into(), system: String::new(), stem: String::new(), tags: vec!["Retro".into()], art: vec![] }, &exe, "").1;
                if let Node::Map(fields) = &mut node {
                    for (k, v) in fields.iter_mut() {
                        if k == "LaunchOptions" {
                            *v = Node::Str("--game-mode".into());
                        }
                    }
                }
                kept.push(node);
                added.push(id);
            }
        }
        *entries = kept.into_iter().enumerate().map(|(i, n)| (i.to_string(), n)).collect();
        say(&progress, "Writing Steam's list…".into());
        if let Some(d) = file.parent() {
            std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
        }
        let bytes = vdf::write_binary(&root);
        if !vdf::valid(&bytes) {
            return Err("the new list didn't check, so nothing was written".into());
        }
        settings_file::write_bytes(&file, &bytes, vdf::valid).map_err(|e| e.to_string())?;
        // what Retro to Steam added, so only those are ever taken out
        let user = p.user.clone();
        let remove = p.remove;
        let was = p.added.clone();
        let path = crate::prefs_path();
        if let Some(d) = path.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        let _ = settings_file::update_json(&path, move |m| {
            let mut all = m.get("added").and_then(Value::as_object).cloned().unwrap_or_default();
            let mut ids: Vec<u32> = if remove { vec![] } else { was };
            for id in added.iter() {
                if !ids.contains(id) {
                    ids.push(*id);
                }
            }
            all.insert(user, json!(ids));
            m.insert("added".into(), Value::Object(all));
        });
        Ok(if p.remove {
            format!("Took out {removed} games Retro to Steam had added, and their art.")
        } else {
            format!("Added {} games to Steam, with their art. They're in your library under their collections.", p.add.len())
        })
    })();
    if was_open {
        say(&progress, "Opening Steam again…".into());
        steam::open_again();
    }
    result
}
