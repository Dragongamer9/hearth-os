//! Putting a layout in place, through Mantel's own settings files and their helpers (settings_file.rs: merged,
//! atomic, .bak, never saved over a file that couldn't be read), so both Mantels pick it up as they pick up a change
//! made in Settings. Only the keys a layout owns are touched; everything else in those files stays as it is.
//! Before anything changes, what was there is kept in layouts/undo.json, and Undo puts it back (Undo again redoes).

use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};

use crate::layout::{self, Layout, DESKTOP_KEYS};
use crate::{paths, settings_file as sf};

/// desktop.json keys a layout owns: its settings, which layout it is, and the window buttons.
fn desktop_keys() -> Vec<&'static str> {
    let mut v: Vec<&str> = DESKTOP_KEYS.iter().map(|(k, _)| *k).collect();
    v.extend(["layout", "windowButtons"]);
    v
}

fn undo_path() -> PathBuf {
    paths::layouts_dir().join("undo.json")
}

/// What the layout keys are now: {"desktop": {key: value | null}, "bar": {"items": … | null, "p:…": …}}.
pub fn snapshot() -> Value {
    let d = sf::read_json(&paths::desktop_json());
    let b = sf::read_json(&paths::bar_json());
    let mut desk = Map::new();
    for k in desktop_keys() {
        desk.insert(k.into(), d.get(k).cloned().unwrap_or(Value::Null));
    }
    let mut bar = Map::new();
    bar.insert("items".into(), b.get("items").cloned().unwrap_or(Value::Null));
    if let Some(m) = b.as_object() {
        for (k, v) in m {
            if k.starts_with("p:") {
                bar.insert(k.clone(), v.clone());
            }
        }
    }
    json!({ "desktop": desk, "bar": bar })
}

/// Puts a snapshot's keys back: desktop keys set or removed, the bar's order set or removed, the extra panels
/// replaced by the snapshot's.
fn restore(s: &Value) -> std::io::Result<()> {
    let desk = s.get("desktop").and_then(Value::as_object).cloned().unwrap_or_default();
    sf::update_json(&paths::desktop_json(), |m| {
        for k in desktop_keys() {
            match desk.get(k) {
                Some(v) if !v.is_null() => {
                    m.insert(k.into(), v.clone());
                }
                _ => {
                    m.remove(k);
                }
            }
        }
    })?;
    let bar = s.get("bar").and_then(Value::as_object).cloned().unwrap_or_default();
    sf::update_json(&paths::bar_json(), |m| {
        m.retain(|k, _| !k.starts_with("p:"));
        match bar.get("items") {
            Some(v) if !v.is_null() => {
                m.insert("items".into(), v.clone());
            }
            _ => {
                m.remove("items");
            }
        }
        for (k, v) in &bar {
            if k.starts_with("p:") && !v.is_null() {
                m.insert(k.clone(), v.clone());
            }
        }
    })?;
    Ok(())
}

/// The snapshot this layout makes: what applying it leaves in the layout keys.
pub fn target(l: &Layout) -> Value {
    let mut desk = Map::new();
    for (k, _) in DESKTOP_KEYS {
        let v = l.desktop.get(*k).and_then(Value::as_str).unwrap_or("");
        desk.insert((*k).into(), if v.is_empty() { Value::Null } else { Value::String(v.into()) });
    }
    desk.insert("layout".into(), Value::String(l.id.clone()));
    desk.insert("windowButtons".into(), l.window_buttons.as_ref().map(|w| Value::String(layout::titlebar(w))).unwrap_or(Value::Null));
    let mut bar = Map::new();
    bar.insert(
        "items".into(),
        match &l.bar {
            None => Value::Null,
            Some(b) => {
                // the bar's other items hidden, so the bar is exactly the layout's (Mantel would put a missing one
                // back in its default place)
                let placed: Vec<&String> = b.start.iter().chain(&b.centre).chain(&b.end).collect();
                let mut hidden = b.hidden.clone();
                for it in crate::blocks::bar_items() {
                    if !it.keep && !placed.iter().any(|x| *x == it.id) && !hidden.iter().any(|x| x == it.id) {
                        hidden.push(it.id.to_string());
                    }
                }
                let mut o = json!({ "start": b.start, "end": b.end, "hidden": hidden });
                if !b.centre.is_empty() {
                    o["centre"] = json!(b.centre);
                }
                o
            }
        },
    );
    for (i, p) in l.panels.iter().enumerate() {
        let mut o = json!({ "screen": "", "edge": p.edge, "items": p.items, "size": p.size, "autoHide": p.auto_hide });
        if !p.style.is_empty() {
            o["style"] = json!(p.style);
        }
        bar.insert(format!("p:{}-{}", l.id, i + 1), o);
    }
    json!({ "desktop": desk, "bar": bar })
}

/// Puts LAYOUT in place (it's checked again first). What was there is kept for Undo.
pub fn apply(l: &Layout) -> Result<(), String> {
    let errs = layout::check(l);
    if !errs.is_empty() {
        return Err(errs.join(" "));
    }
    let before = snapshot();
    std::fs::create_dir_all(paths::layouts_dir()).map_err(|e| e.to_string())?;
    sf::write_json(&undo_path(), &before).map_err(|e| format!("Couldn't keep what's there now for Undo ({e}); nothing changed."))?;
    restore(&target(l)).map_err(|e| format!("Couldn't save the layout ({e})."))?;
    sf::log_event(&format!("layout {} put in place (undo: {})", l.id, undo_path().display()));
    Ok(())
}

/// Puts back what was there before the last layout (and keeps this one, so Undo again redoes it).
pub fn undo() -> Result<(), String> {
    let s = match sf::read(&undo_path()) {
        sf::Read::Ok(m) | sf::Read::Restored(m) => Value::Object(m),
        _ => return Err("There's nothing to undo.".into()),
    };
    let now = snapshot();
    restore(&s).map_err(|e| format!("Couldn't put it back ({e})."))?;
    sf::write_json(&undo_path(), &now).map_err(|e| e.to_string())?;
    sf::log_event("layout: undo");
    Ok(())
}

/// Which layout is in place ("" for none chosen, or after changes made by hand in Settings it's still the last one).
pub fn current() -> String {
    sf::read_json(&paths::desktop_json()).get("layout").and_then(Value::as_str).unwrap_or("").to_string()
}

/// Today's desktop as a layout named NAME (Save as a layout), kept with your own layouts.
pub fn save(name: &str) -> Result<PathBuf, String> {
    let id: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    let id: String = id.chars().take(40).collect();
    if !crate::is_slug(&id) {
        return Err("Give it a name with some letters or digits.".into());
    }
    let l = today(&id, name);
    let errs = layout::check(&l);
    if !errs.is_empty() {
        return Err(errs.join(" "));
    }
    let p = paths::layouts_dir().join(format!("{id}.json"));
    std::fs::create_dir_all(paths::layouts_dir()).map_err(|e| e.to_string())?;
    sf::write_json(&p, &serde_json::to_value(&l).unwrap()).map_err(|e| e.to_string())?;
    Ok(p)
}

/// Today's desktop as a layout (what Settings' Desktop layout page starts from when no layout is in place, and what
/// Save as my own keeps). Blocks this Hearth doesn't know are left out, as both Mantels leave them out.
pub fn today(id: &str, name: &str) -> Layout {
    let s = snapshot();
    let mut desktop = Map::new();
    for (k, _) in DESKTOP_KEYS {
        if let Some(v) = s["desktop"].get(*k).and_then(Value::as_str) {
            if !v.is_empty() {
                desktop.insert((*k).into(), Value::String(v.into()));
            }
        }
    }
    let bar: Option<layout::Bar> = s["bar"].get("items").filter(|v| v.is_object()).and_then(|v| serde_json::from_value(v.clone()).ok()).map(|mut b: layout::Bar| {
        let mut seen: Vec<String> = Vec::new();
        for z in [&mut b.start, &mut b.centre, &mut b.end, &mut b.hidden] {
            z.retain(|id| {
                let ok = crate::blocks::in_bar(id) && !seen.contains(id);
                seen.push(id.clone());
                ok
            });
        }
        b.hidden.retain(|id| !crate::blocks::find(id).map(|x| x.keep).unwrap_or(false));
        b
    });
    let mut panels = Vec::new();
    if let Some(m) = s["bar"].as_object() {
        let mut keys: Vec<&String> = m.keys().filter(|k| k.starts_with("p:")).collect();
        keys.sort();
        for k in keys {
            if let Ok(mut p) = serde_json::from_value::<layout::Panel>(m[k].clone()) {
                p.items.retain(|id| crate::blocks::in_panel(id));
                p.size = p.size.clamp(28, 96);
                if !p.items.is_empty() {
                    panels.push(p);
                }
            }
        }
    }
    let window_buttons = s["desktop"].get("windowButtons").and_then(Value::as_str).and_then(from_titlebar);
    Layout { format: 1, id: id.into(), name: name.trim().chars().take(60).collect(), about: String::new(), desktop, bar, panels, window_buttons, screen: None }
}

/// labwc's titlebar text back to buttons.
pub fn from_titlebar(t: &str) -> Option<layout::WindowButtons> {
    let (left, right) = t.split_once(':')?;
    let (side, list) = if right.trim().is_empty() { ("left", left) } else { ("right", right) };
    let buttons: Vec<String> = list.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty() && s != "icon").collect();
    Some(layout::WindowButtons { side: side.into(), buttons })
}

/// Every layout to pick from: Hearth's own, then yours (yours win on the same id). Files that can't be used are
/// left out and named in the second list with why.
pub fn all() -> (Vec<(Layout, bool)>, Vec<(PathBuf, String)>) {
    // Hearth's own in this order (the editor's pictures), then yours by name
    const ORDER: &[&str] = &["hearth", "steam", "plasma", "gnome", "windows11", "macos"];
    let mut out: Vec<(Layout, bool)> = Vec::new();
    let mut bad = Vec::new();
    for (dir, mine) in [(paths::builtin_dir(), false), (paths::layouts_dir(), true)] {
        let mut files: Vec<PathBuf> = std::fs::read_dir(&dir).map(|r| r.flatten().map(|e| e.path()).collect()).unwrap_or_default();
        files.sort_by_key(|f| {
            let stem = f.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            (ORDER.iter().position(|x| *x == stem).unwrap_or(ORDER.len()), stem)
        });
        for f in files {
            if f.extension().and_then(|x| x.to_str()) != Some("json") || f.file_name().map(|n| n == "undo.json").unwrap_or(false) {
                continue;
            }
            match load(&f) {
                Ok(l) => {
                    out.retain(|(o, _)| o.id != l.id);
                    out.push((l, mine));
                }
                Err(e) => bad.push((f, e)),
            }
        }
    }
    (out, bad)
}

pub fn load(f: &Path) -> Result<Layout, String> {
    let t = std::fs::read_to_string(f).map_err(|e| e.to_string())?;
    layout::parse(&t).map_err(|e| e.join(" "))
}

/// A layout by id (yours or Hearth's) or a file's path.
pub fn find(what: &str) -> Result<Layout, String> {
    let p = Path::new(what);
    if what.contains('/') || what.ends_with(".json") {
        return load(p);
    }
    all().0.into_iter().map(|(l, _)| l).find(|l| l.id == what).ok_or_else(|| format!("There's no layout called \"{what}\"."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    static ENV: Mutex<()> = Mutex::new(());

    fn home() -> (tempfile::TempDir, std::sync::MutexGuard<'static, ()>) {
        let g = ENV.lock().unwrap_or_else(|e| e.into_inner());
        let d = tempfile::tempdir().unwrap();
        std::env::set_var("HOME", d.path());
        std::env::remove_var("XDG_CONFIG_HOME");
        std::env::remove_var("XDG_DATA_HOME");
        std::env::set_var("XDG_STATE_HOME", d.path().join("state"));
        std::env::set_var("NIMBUS_SHARE", Path::new(env!("CARGO_MANIFEST_DIR")).join("../linux"));
        (d, g)
    }

    #[test]
    fn apply_keeps_other_settings_and_undo_puts_it_back() {
        let (_d, _g) = home();
        std::fs::create_dir_all(paths::desktop_json().parent().unwrap()).unwrap();
        std::fs::create_dir_all(paths::bar_json().parent().unwrap()).unwrap();
        std::fs::write(paths::desktop_json(), r#"{"look":"steam-dark","barLayout":"split","menuStyle":"compact"}"#).unwrap();
        std::fs::write(paths::bar_json(), r#"{"clockZones":["Europe/Paris"],"p:mine":{"edge":"left","items":["clock"]}}"#).unwrap();
        let before_d = std::fs::read_to_string(paths::desktop_json()).unwrap();
        let before_b = std::fs::read_to_string(paths::bar_json()).unwrap();

        let l = find("plasma").unwrap();
        apply(&l).unwrap();
        let d = sf::read_json(&paths::desktop_json());
        let b = sf::read_json(&paths::bar_json());
        assert_eq!(d["look"], "steam-dark", "the look isn't the layout's");
        assert_eq!(d["layout"], "plasma");
        assert_eq!(d["barLayout"], "one");
        assert!(d.get("menuStyle").is_none(), "a layout's own keys go back to their default when it doesn't set them");
        assert_eq!(b["clockZones"][0], "Europe/Paris", "the clock's settings stay");
        assert!(b.get("p:mine").is_none(), "the old panels make way for the layout's");
        assert_eq!(current(), "plasma");

        undo().unwrap();
        let d: Value = serde_json::from_str(&std::fs::read_to_string(paths::desktop_json()).unwrap()).unwrap();
        let b: Value = serde_json::from_str(&std::fs::read_to_string(paths::bar_json()).unwrap()).unwrap();
        assert_eq!(d, serde_json::from_str::<Value>(&before_d).unwrap());
        assert_eq!(b, serde_json::from_str::<Value>(&before_b).unwrap());
        undo().unwrap();
        assert_eq!(current(), "plasma", "Undo again redoes it");
    }

    #[test]
    fn a_broken_settings_file_isnt_saved_over() {
        let (_d, _g) = home();
        std::fs::create_dir_all(paths::desktop_json().parent().unwrap()).unwrap();
        std::fs::write(paths::desktop_json(), r#"{"look":"hearth-light"}"#).unwrap();
        sf::merge_json(&paths::desktop_json(), &serde_json::from_str(r#"{"barStyle":"bar"}"#).unwrap()).unwrap();
        std::fs::write(paths::desktop_json(), "{ half a fi").unwrap();
        apply(&find("gnome").unwrap()).unwrap();
        let d = sf::read_json(&paths::desktop_json());
        assert_eq!(d["look"], "hearth-light", "the last good copy was put back first, then the layout merged in");
        assert_eq!(d["layout"], "gnome");
    }

    #[test]
    fn save_makes_a_layout_from_today() {
        let (_d, _g) = home();
        apply(&find("windows11").unwrap()).unwrap();
        let p = save("My desk!").unwrap();
        let l = load(&p).unwrap();
        assert_eq!(l.id, "my-desk");
        assert_eq!(l.panels, find("windows11").unwrap().panels);
        assert_eq!(find("my-desk").unwrap().name, "My desk!");
    }

    #[test]
    fn titlebar_round_trip() {
        let w = from_titlebar("close,iconify,max:").unwrap();
        assert_eq!(w.side, "left");
        assert_eq!(layout::titlebar(&w), "close,iconify,max:");
        assert_eq!(from_titlebar("icon:iconify,max,close").unwrap().buttons, ["iconify", "max", "close"]);
    }
}
