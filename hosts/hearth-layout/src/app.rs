//! Describe it in Hearth's own apps (desktop-layouts board 11, Hearth Files; part 12: every Hearth app). One shared
//! core: each app names its own layout choices (its knobs), its places a new feature can go, and what it can show and
//! do there besides Hearth's own (feature.rs SOURCES and ACTIONS). Your words go to your AI with only those; the answer
//! is checked here like a layout or a feature, and what it can't do becomes an idea (idea.rs).
//!
//! What you keep is one file per app, `~/.config/nimbus/apps/<app>.json` (settings_file: atomic, .bak):
//! `{"format": 1, "layout": {knob: value}, "features": [Feature], "undo": {"layout", "features"}}`. Keep this saves it
//! with the one before as `undo`; Undo puts that back; Back to the original (View › Layout) empties it. Nothing an app
//! does changes: only how it's laid out, and features made from its own parts.

use crate::{feature, idea};
use serde_json::{json, Map, Value};

pub struct Knob {
    pub key: &'static str,
    pub what: &'static str,
    /// (value, how the chip says it); "" is the app's own way
    pub values: &'static [(&'static str, &'static str)],
}

pub struct AppSchema {
    pub app: &'static str,
    pub name: &'static str,
    pub knobs: &'static [Knob],
    /// places a feature can go in this app: (id, what it is); a feature's part is "<app>:<id>"
    pub parts: &'static [(&'static str, &'static str)],
    pub sources: &'static [(&'static str, &'static str)],
    pub actions: &'static [(&'static str, &'static str)],
}

/// Hearth Files (board 11).
pub const FILES: AppSchema = AppSchema {
    app: "files",
    name: "Hearth Files",
    knobs: &[
        Knob { key: "sidebar", what: "the sidebar", values: &[("", "sidebar as it is"), ("icons", "sidebar icons only"), ("hidden", "no sidebar")] },
        Knob { key: "preview", what: "the preview of the picked file", values: &[("", "preview as it is"), ("right", "preview on the right"), ("right-big", "preview right, bigger"), ("off", "no preview")] },
        Knob { key: "view", what: "how files are shown", values: &[("", "view as it is"), ("icons", "big icons"), ("list", "a list"), ("details", "details")] },
        Knob { key: "toolbar", what: "the top bar", values: &[("", "top bar as it is"), ("compact", "a smaller top bar")] },
    ],
    parts: &[("downloads", "the Downloads folder's own bar (next to Sort my Downloads)"), ("folder", "every folder's bar, at the top")],
    sources: &[
        ("folder-pictures", "how many pictures are in this folder"),
        ("folder-videos", "how many videos are in this folder"),
        ("folder-old", "how many files here are older than a month"),
        ("folder-big", "how many files here are over 1 GB"),
        ("folder-count", "how many files are in this folder"),
        ("folder-size", "how big this folder is"),
    ],
    actions: &[
        ("sort-downloads", "Sort my Downloads"),
        ("select-pictures", "select the pictures here"),
        ("select-old", "select the files here older than a month"),
        ("select-big", "select the files here over 1 GB"),
    ],
};

/// Every app with Describe it, by its id (hearth-layout describe-app <app>).
pub const APPS: &[&AppSchema] = &[&FILES];

pub fn find(app: &str) -> Option<&'static AppSchema> {
    APPS.iter().copied().find(|a| a.app == app)
}

/// What the AI is told for this app.
pub fn rules(a: &AppSchema) -> String {
    let knobs: Vec<String> = a.knobs.iter().map(|k| format!("{} ({}): {}", k.key, k.what, k.values.iter().map(|(v, l)| format!("\"{v}\" = {l}")).collect::<Vec<_>>().join(", "))).collect();
    let parts: Vec<String> = a.parts.iter().map(|(id, n)| format!("{}:{id} ({n})", a.app)).collect();
    let sources: Vec<String> = feature::SOURCES.iter().chain(a.sources.iter()).map(|(id, n)| format!("{id} ({n})")).collect();
    let actions: Vec<String> = feature::ACTIONS.iter().chain(a.actions.iter()).map(|(id, n)| format!("{id} ({n})")).collect();
    format!(
        "You change how {name} is laid out for its owner, and may add features made only from its own parts. What its buttons do never changes.\n\
Answer with ONE JSON object and nothing else, one of:\n\
1. {{\"kind\": \"app\", \"layout\": {{<knob>: <value>, ...only the ones that change}}, \"features\": [<feature>, ...up to 3], \"changes\": [\"short plain sentences of what changed\"]}}\n\
   knobs: {knobs}\n\
   <feature> = {{\"format\": 1, \"id\": \"<lowercase-dashes>\", \"name\": \"...\", \"part\": <a place>, \"shows\": [{{\"source\", \"label\"}}], \"every\": seconds, \"click\": {{\"action\", \"target\"}}}}\n\
   places: {parts}\n\
   sources: {sources}\n\
   actions: {actions}\n\
2. When it needs something {name} doesn't have: {{\"kind\": \"needs\", \"needs\": [\"what's missing, in a few words\"], \"why\": \"one or two plain sentences\", \"closest\": <feature or null>, \"changes\": []}}\n\
Never write code, commands or file paths. Use plain words for the owner in \"changes\".",
        name = a.name,
        knobs = knobs.join("; "),
        parts = parts.join(", "),
        sources = sources.join(", "),
        actions = actions.join(", ")
    )
}

/// A feature for this app, checked: Hearth's own rules (feature.rs), with the app's places, sources and actions too.
pub fn check_feature(a: &AppSchema, f: &feature::Feature) -> feature::Checked {
    let mut g = f.clone();
    let mut needs = Vec::new();
    match f.part.strip_prefix(&format!("{}:", a.app)) {
        Some(p) if a.parts.iter().any(|(id, _)| *id == p) => g.part = a.app.to_string(),
        _ => {
            needs.push(format!("a place in {} called \"{}\"", a.name, f.part));
            g.part = a.app.to_string();
        }
    }
    for s in g.shows.iter_mut() {
        if a.sources.iter().any(|(id, _)| *id == s.source) {
            s.source = "text".into();
        }
    }
    if let Some(c) = g.click.as_mut() {
        if a.actions.iter().any(|(id, _)| *id == c.action) {
            c.action = "none".into();
            c.target.clear();
        }
    }
    // "files" must be a place feature.rs knows; it is (feature::PARTS)
    let mut c = feature::check(&g);
    for n in needs {
        if !c.needs.contains(&n) {
            c.needs.push(n);
        }
    }
    c
}

/// The knobs that change, checked: an unknown knob or value is a problem, never kept.
pub fn check_layout(a: &AppSchema, l: &Map<String, Value>) -> Result<Map<String, Value>, Vec<String>> {
    let mut out = Map::new();
    let mut errs = Vec::new();
    for (k, v) in l {
        let Some(knob) = a.knobs.iter().find(|x| x.key == k) else {
            errs.push(format!("{} has no \"{k}\" to change.", a.name));
            continue;
        };
        match v.as_str() {
            Some(s) if knob.values.iter().any(|(x, _)| *x == s) => {
                out.insert(k.clone(), json!(s));
            }
            _ => errs.push(format!("{} can't be \"{}\".", knob.what, v.as_str().unwrap_or("that"))),
        }
    }
    if errs.is_empty() { Ok(out) } else { Err(errs) }
}

/// The chips under the words (board 11): "Layout: sidebar icons only", "New feature: counts in Downloads".
pub fn chips(a: &AppSchema, layout: &Map<String, Value>, features: &[feature::Feature]) -> Vec<Value> {
    let mut v = Vec::new();
    for k in a.knobs {
        if let Some(s) = layout.get(k.key).and_then(Value::as_str) {
            if let Some((_, l)) = k.values.iter().find(|(x, _)| *x == s) {
                v.push(json!({"kind": "layout", "text": format!("Layout: {l}")}));
            }
        }
    }
    for f in features {
        v.push(json!({"kind": "feature", "text": format!("New feature: {}", f.name.to_lowercase())}));
    }
    v
}

/// An answer from the AI, checked (as describe.rs check_answer does for Mantel).
pub fn check_answer(a: &AppSchema, v: &Value, asked: &str, sketch: bool) -> Result<Value, Vec<String>> {
    let changes: Vec<String> = v.get("changes").and_then(Value::as_array).map(|x| x.iter().filter_map(|s| s.as_str().map(|s| s.chars().take(200).collect())).take(12).collect()).unwrap_or_default();
    let versions = (crate::describe::hearth_version(), std::env::var("HEARTH_DESCRIBE_APP").unwrap_or_else(|_| a.name.to_string()).chars().take(40).collect::<String>());
    match v.get("kind").and_then(Value::as_str) {
        Some("app") | Some("layout") | Some("feature") => {
            let layout = check_layout(a, &v.get("layout").and_then(Value::as_object).cloned().unwrap_or_default())?;
            let mut fs: Vec<feature::Feature> = Vec::new();
            let mut raw: Vec<Value> = v.get("features").and_then(Value::as_array).cloned().unwrap_or_default();
            if let Some(one) = v.get("feature") {
                raw.push(one.clone());
            }
            let mut needs: Vec<String> = Vec::new();
            for f in raw.into_iter().take(3) {
                let f: feature::Feature = serde_json::from_value(f).map_err(|e| vec![format!("A feature couldn't be read ({e}).")])?;
                let c = check_feature(a, &f);
                if !c.problems.is_empty() {
                    return Err(c.problems);
                }
                needs.extend(c.needs.iter().cloned());
                fs.push(f);
            }
            if !needs.is_empty() {
                let closest = fs.first().cloned();
                let i = idea::Idea::new(a.app, asked, needs.clone(), closest.clone(), sketch, &versions.0, &versions.1);
                return Ok(json!({"ok": true, "kind": "needs", "app": a.app, "needs": needs, "closest": closest, "idea": i, "preview": i.preview(), "changes": changes}));
            }
            if layout.is_empty() && fs.is_empty() {
                return Err(vec!["Nothing would change.".into()]);
            }
            let chips = chips(a, &layout, &fs);
            Ok(json!({"ok": true, "kind": "app", "app": a.app, "layout": layout, "features": fs, "chips": chips, "changes": changes}))
        }
        Some("needs") => {
            let needs: Vec<String> = v.get("needs").and_then(Value::as_array).map(|x| x.iter().filter_map(|s| s.as_str().map(|s| s.chars().take(120).collect())).take(8).collect()).unwrap_or_default();
            let closest: Option<feature::Feature> = v.get("closest").cloned().and_then(|c| serde_json::from_value(c).ok()).filter(|f| check_feature(a, f).usable());
            let why: String = v.get("why").and_then(Value::as_str).unwrap_or("").chars().take(400).collect();
            let i = idea::Idea::new(a.app, asked, needs.clone(), closest.clone(), sketch, &versions.0, &versions.1);
            Ok(json!({"ok": true, "kind": "needs", "app": a.app, "needs": needs, "why": why, "closest": closest, "idea": i, "preview": i.preview(), "changes": changes}))
        }
        _ => Err(vec!["The answer wasn't a change to the app or what's needed.".into()]),
    }
}

/// The whole of Describe it for an app: ask, check, ask once more with the reasons.
pub fn describe(a: &AppSchema, text: &str, sketch: Option<&std::path::Path>) -> Value {
    let text: String = text.trim().chars().take(2000).collect();
    if text.is_empty() && sketch.is_none() {
        return json!({"ok": false, "error": "Type what you'd like, or add a sketch."});
    }
    let Some(e) = crate::describe::engine() else {
        return json!({"ok": false, "error": "No AI is set for Describe it: pick one in Settings › Personalization › Desktop layout, or turn on an AI engine in Settings › Home server."});
    };
    let system = rules(a);
    let mut user = format!("Now: {}\nThe owner asks: {}", get(a.app), text);
    for attempt in 0..2 {
        let reply = match crate::describe::chat(&e, &system, &user, sketch) {
            Ok(r) => r,
            Err(m) => return json!({"ok": false, "error": m, "engine": e.label}),
        };
        let Some(v) = crate::describe::first_object(&reply) else {
            user = format!("{user}\nYour last answer wasn't one JSON object. Answer again with only the JSON object.");
            continue;
        };
        match check_answer(a, &v, &text, sketch.is_some()) {
            Ok(mut ok) => {
                ok["engine"] = json!(e.label);
                return ok;
            }
            Err(why) if attempt == 0 => user = format!("{user}\nYour last answer can't be used: {}. Answer again, fixed.", why.join(" ")),
            Err(why) => return json!({"ok": false, "error": format!("Your AI's answer couldn't be used: {}", why.join(" ")), "engine": e.label}),
        }
    }
    json!({"ok": false, "error": "Your AI didn't give an answer Hearth can use. Try again, or say it another way.", "engine": e.label})
}

// ---------------- what you kept ----------------

pub fn file(app: &str) -> std::path::PathBuf {
    let cfg = std::env::var_os("XDG_CONFIG_HOME").map(std::path::PathBuf::from).unwrap_or_else(|| crate::paths::home().join(".config"));
    cfg.join("nimbus/apps").join(format!("{app}.json"))
}

/// What this app has now: {"layout": {...}, "features": [...]} (empty when nothing was kept, or the file can't be read).
pub fn get(app: &str) -> Value {
    let j = crate::settings_file::read_json(&file(app));
    json!({"layout": j.get("layout").cloned().unwrap_or(json!({})), "features": j.get("features").cloned().unwrap_or(json!([]))})
}

/// Keep this: {"layout", "features"} checked again, saved with the one before as Undo. The layout is merged onto what
/// was kept (a knob it doesn't name stays as it was); features with the same id are replaced.
pub fn keep(a: &AppSchema, change: &Value) -> Result<Value, String> {
    let layout = check_layout(a, &change.get("layout").and_then(Value::as_object).cloned().unwrap_or_default()).map_err(|e| e.join(" "))?;
    let mut fs: Vec<feature::Feature> = Vec::new();
    for f in change.get("features").and_then(Value::as_array).cloned().unwrap_or_default() {
        let f: feature::Feature = serde_json::from_value(f).map_err(|e| format!("A feature couldn't be read ({e})."))?;
        let c = check_feature(a, &f);
        if !c.usable() {
            return Err(c.problems.into_iter().chain(c.needs.into_iter().map(|n| format!("needs {n}"))).collect::<Vec<_>>().join(" "));
        }
        fs.push(f);
    }
    let p = file(a.app);
    if let Some(d) = p.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    let (m, _) = crate::settings_file::update_json(&p, |m| {
        let before = json!({"layout": m.get("layout").cloned().unwrap_or(json!({})), "features": m.get("features").cloned().unwrap_or(json!([]))});
        let mut l = m.get("layout").and_then(Value::as_object).cloned().unwrap_or_default();
        for (k, v) in layout {
            if v.as_str() == Some("") {
                l.remove(&k);
            } else {
                l.insert(k, v);
            }
        }
        let mut have: Vec<Value> = m.get("features").and_then(Value::as_array).cloned().unwrap_or_default();
        for f in &fs {
            have.retain(|x| x.get("id").and_then(Value::as_str) != Some(f.id.as_str()));
            have.push(serde_json::to_value(f).unwrap_or(Value::Null));
        }
        m.insert("format".into(), json!(1));
        m.insert("layout".into(), Value::Object(l));
        m.insert("features".into(), Value::Array(have));
        m.insert("undo".into(), before);
    })
    .map_err(|e| e.to_string())?;
    Ok(Value::Object(m))
}

/// Undo: what was kept before Keep this, back.
pub fn undo(app: &str) -> Result<Value, String> {
    let p = file(app);
    if !p.exists() {
        return Ok(get(app));
    }
    let (m, _) = crate::settings_file::update_json(&p, |m| {
        let now = json!({"layout": m.get("layout").cloned().unwrap_or(json!({})), "features": m.get("features").cloned().unwrap_or(json!([]))});
        if let Some(u) = m.get("undo").cloned() {
            m.insert("layout".into(), u.get("layout").cloned().unwrap_or(json!({})));
            m.insert("features".into(), u.get("features").cloned().unwrap_or(json!([])));
        }
        m.insert("undo".into(), now);
    })
    .map_err(|e| e.to_string())?;
    Ok(Value::Object(m))
}

/// Back to the original (View › Layout): nothing changed, with what you had as Undo.
pub fn reset(app: &str) -> Result<Value, String> {
    let p = file(app);
    if !p.exists() {
        return Ok(get(app));
    }
    let (m, _) = crate::settings_file::update_json(&p, |m| {
        let now = json!({"layout": m.get("layout").cloned().unwrap_or(json!({})), "features": m.get("features").cloned().unwrap_or(json!([]))});
        m.insert("layout".into(), json!({}));
        m.insert("features".into(), json!([]));
        m.insert("undo".into(), now);
    })
    .map_err(|e| e.to_string())?;
    Ok(Value::Object(m))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn board11() -> Value {
        json!({"kind": "app", "layout": {"sidebar": "icons", "preview": "right-big"},
               "features": [{"format": 1, "id": "downloads-counts", "name": "Counts in Downloads", "part": "files:downloads",
                             "shows": [{"source": "folder-pictures", "label": "Pictures here"}, {"source": "folder-old", "label": "Older than a month"}],
                             "click": {"action": "select-pictures"}}],
               "changes": ["The sidebar shows icons only", "The preview is on the right, bigger"]})
    }

    #[test]
    fn board_11s_answer_is_read() {
        let r = check_answer(&FILES, &board11(), "sidebar icons only…", false).unwrap();
        assert_eq!(r["kind"], "app");
        assert_eq!(r["layout"]["sidebar"], "icons");
        let chips: Vec<&str> = r["chips"].as_array().unwrap().iter().map(|c| c["text"].as_str().unwrap()).collect();
        assert_eq!(chips, ["Layout: sidebar icons only", "Layout: preview right, bigger", "New feature: counts in downloads"]);
    }

    #[test]
    fn what_files_cant_do_is_refused_or_an_idea() {
        let mut v = board11();
        v["layout"]["sidebar"] = json!("floating");
        assert!(check_answer(&FILES, &v, "x", false).is_err(), "an unknown value");
        let mut v = board11();
        v["layout"]["colour"] = json!("red");
        assert!(check_answer(&FILES, &v, "x", false).is_err(), "an unknown knob");
        let mut v = board11();
        v["features"][0]["shows"][0]["source"] = json!("folder-faces");
        let r = check_answer(&FILES, &v, "count the faces", true).unwrap();
        assert_eq!(r["kind"], "needs");
        assert!(r["preview"].as_str().unwrap().contains("Sketch: yes"));
        let mut v = board11();
        v["features"][0]["part"] = json!("bar");
        assert_eq!(check_answer(&FILES, &v, "x", false).unwrap()["kind"], "needs", "Mantel's bar isn't a place in Files");
    }

    #[test]
    fn keep_undo_and_back_to_the_original() {
        let d = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_CONFIG_HOME", d.path());
        let r = check_answer(&FILES, &board11(), "x", false).unwrap();
        keep(&FILES, &r).unwrap();
        assert_eq!(get("files")["layout"]["preview"], "right-big");
        assert_eq!(get("files")["features"].as_array().unwrap().len(), 1);
        keep(&FILES, &json!({"layout": {"sidebar": ""}})).unwrap();
        assert!(get("files")["layout"].get("sidebar").is_none(), "\"\" is the app's own way");
        assert_eq!(get("files")["layout"]["preview"], "right-big", "a knob not named stays");
        undo("files").unwrap();
        assert_eq!(get("files")["layout"]["sidebar"], "icons");
        reset("files").unwrap();
        assert_eq!(get("files")["layout"], json!({}));
        assert!(std::path::Path::new(&format!("{}.bak", file("files").display())).exists() || file("files").exists());
        assert!(keep(&FILES, &json!({"layout": {"sidebar": "floating"}})).is_err());
    }

    #[test]
    fn the_rules_name_every_part_of_the_app() {
        let r = rules(&FILES);
        for k in FILES.knobs {
            assert!(r.contains(k.key));
        }
        for (s, _) in FILES.sources {
            assert!(r.contains(s));
        }
        assert!(r.contains("files:downloads"));
    }
}
