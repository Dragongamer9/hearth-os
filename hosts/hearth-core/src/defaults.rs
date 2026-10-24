//! Settings › Default apps in Game Mode: which app opens web links, folders, text, pictures, videos, music, PDFs,
//! office files, calendars, contacts, maps and archives, and which terminal starts. Shared with Desktop Mode's page
//! (SetDefaults.qml): all of it is desktop/shell/hearth-system (Python), which reads every installed app's .desktop
//! file and writes your choices in ~/.config/mimeapps.list ("Reset to Hearth's defaults" takes them out again).

use serde_json::Value;
use std::process::{Command, Stdio};

/// The kinds of app the page has (hearth-system's CATEGORIES).
pub const KINDS: &[&str] = &["browser", "files", "text", "image", "video", "music", "ebooks", "documents", "sheets", "slides",
    "terminal", "email", "calendar", "contacts", "maps", "archives"];

/// hearth-system's arguments for a bridge call: defaults.list, defaults.set {kind, app}, defaults.reset {kind?}.
pub fn args(method: &str, a: &Value) -> Result<Vec<String>, String> {
    let kind = |required: bool| -> Result<Option<String>, String> {
        match a.get("kind").and_then(Value::as_str).filter(|k| !k.is_empty()) {
            Some(k) if KINDS.contains(&k) => Ok(Some(k.to_string())),
            Some(_) => Err("Unknown kind of app".into()),
            None if required => Err("Which kind of app?".into()),
            None => Ok(None),
        }
    };
    match method {
        "defaults.list" => Ok(vec!["defaults".into()]),
        "defaults.set" => {
            let k = kind(true)?.unwrap_or_default();
            let app = a.get("app").and_then(Value::as_str).unwrap_or("");
            let ok = app.len() <= 208 && app.ends_with(".desktop") && app.len() > 8
                && app.chars().all(|c| c.is_ascii_alphanumeric() || "_.+-".contains(c));
            if !ok { return Err("That isn't an app".into()); }
            Ok(vec!["default-set".into(), k, app.to_string()])
        }
        "defaults.reset" => Ok(match kind(false)? { Some(k) => vec!["default-reset".into(), k], None => vec!["default-reset".into()] }),
        _ => Err("Unknown call".into()),
    }
}

/// Runs hearth-system with these arguments: its JSON answer, or its "error: ..." in plain words.
fn run(helper: &std::path::Path, args: &[String]) -> Result<Value, String> {
    if !helper.exists() { return Err("Default apps need Hearth, the OS.".into()); }
    let out = Command::new("python3").arg(helper).args(args)
        .stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped())
        .output().map_err(|e| e.to_string())?;
    let v = crate::admin::parse_helper(&String::from_utf8_lossy(&out.stdout))?;
    if !out.status.success() && v.is_null() {
        return Err(String::from_utf8_lossy(&out.stderr).lines().last().unwrap_or("It didn't work.").to_string());
    }
    Ok(v)
}

/// A bridge call: a change answers with the whole list again, so the page shows what's in use now.
pub fn call(method: &str, a: &Value) -> Result<Value, String> {
    call_with(&crate::paths::desktop_dir().join("shell/hearth-system"), method, a)
}

fn call_with(helper: &std::path::Path, method: &str, a: &Value) -> Result<Value, String> {
    let argv = args(method, a)?;
    let changed = run(helper, &argv)?;
    if method == "defaults.list" { return Ok(changed); }
    let mut list = run(helper, &args("defaults.list", a)?)?;
    if let Some(o) = list.as_object_mut() { o.insert("changed".into(), changed); }
    Ok(list)
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    #[test]
    fn calls_become_helper_arguments() {
        assert_eq!(super::args("defaults.list", &json!({})).unwrap(), vec!["defaults"]);
        assert_eq!(super::args("defaults.set", &json!({"kind": "image", "app": "org.gnome.Loupe.desktop"})).unwrap(),
                   vec!["default-set", "image", "org.gnome.Loupe.desktop"]);
        assert_eq!(super::args("defaults.reset", &json!({"kind": "browser"})).unwrap(), vec!["default-reset", "browser"]);
        assert_eq!(super::args("defaults.reset", &json!({})).unwrap(), vec!["default-reset"]);
    }
    #[test]
    fn bad_kinds_and_apps_are_refused() {
        for a in [json!({"kind": "image", "app": "../x.desktop"}), json!({"kind": "image", "app": "x.desktop; rm -rf"}),
                  json!({"kind": "image", "app": ".desktop"}), json!({"kind": "image"}), json!({"kind": "nope", "app": "a.desktop"}),
                  json!({"app": "a.desktop"}), json!({"kind": "image", "app": "--help.desktop.x"})] {
            assert!(super::args("defaults.set", &a).is_err(), "{a}");
        }
        assert!(super::args("defaults.reset", &json!({"kind": "nope"})).is_err());
        assert!(super::args("defaults.other", &json!({})).is_err());
    }
    #[test]
    fn helper_answers_and_errors() {
        let dir = tempfile::tempdir().unwrap();
        let helper = dir.path().join("hearth-system");
        std::fs::write(&helper,
            "import sys, json\nif sys.argv[1] == 'default-set' and sys.argv[3] == 'gone.desktop':\n    print(\"error: that app isn't installed\"); sys.exit(1)\nprint(json.dumps({'kinds': [], 'args': sys.argv[1:]}))\n").unwrap();
        let v = super::call_with(&helper, "defaults.set", &json!({"kind": "text", "app": "kate.desktop"})).unwrap();
        assert_eq!(v["args"], json!(["defaults"]));
        assert_eq!(v["changed"]["args"], json!(["default-set", "text", "kate.desktop"]));
        assert_eq!(super::call_with(&helper, "defaults.set", &json!({"kind": "text", "app": "gone.desktop"})).unwrap_err(), "that app isn't installed");
        assert!(super::call_with(&dir.path().join("missing"), "defaults.list", &json!({})).is_err());
    }
}
