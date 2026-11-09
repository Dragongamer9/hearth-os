//! Drive letters on Hearth OS: Hearth's own, the same ones Hearth Files' This PC and Windows apps use. They're kept
//! by hearth_drive_letters.py (hosts/linux/desktop/shell, docs/HEARTH-FILES.md "Drive letters"); Disks only calls
//! it: `show` to read them (it never hands out letters, so opening Disks writes nothing) and `set_letter` for
//! Change drive letter… (your choice, kept). C: is always the drive Hearth runs from. A drive's key is
//! "uuid:<its file system UUID>", as Files makes it.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

/// Where hearth_drive_letters.py is: $HEARTH_DRIVE_LETTERS_PY, Hearth OS's copy, or this repository's (tests).
pub fn script() -> PathBuf {
    if let Some(p) = std::env::var_os("HEARTH_DRIVE_LETTERS_PY") { return PathBuf::from(p); }
    let installed = PathBuf::from("/usr/share/nimbus/desktop/shell/hearth_drive_letters.py");
    if installed.is_file() { return installed; }
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../linux/desktop/shell/hearth_drive_letters.py"))
}

/// ~/.config/hearth/drive-letters.json, as hearth_drive_letters.py finds it (only looked at to know when to read
/// the letters again).
fn state_file() -> PathBuf {
    let cfg = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".config"));
    cfg.join("hearth/drive-letters.json")
}

/// The key a partition's letter is kept under (empty: it has no UUID, so it can't keep one).
pub fn key(uuid: &str) -> String { if uuid.is_empty() { String::new() } else { format!("uuid:{uuid}") } }

/// {key: letter} from `hearth_drive_letters.py show`'s JSON.
pub fn parse_show(json: &str) -> HashMap<String, String> {
    let v: serde_json::Value = serde_json::from_str(json.trim()).unwrap_or_default();
    v["letters"].as_object().map(|o| o.iter().filter_map(|(k, x)| {
        let l = x["letter"].as_str()?.trim().to_uppercase();
        (l.len() == 1 && crate::safety::letter_ok(&l)).then(|| (k.clone(), l))
    }).collect()).unwrap_or_default()
}

static CACHE: Mutex<Option<(Option<std::time::SystemTime>, PathBuf, HashMap<String, String>)>> = Mutex::new(None);

/// Every drive's letter: {key: "E"}. Read again only when the letters' file changes (Disks looks every few
/// seconds). No file yet: no letters, and nothing is run.
pub fn load() -> HashMap<String, String> {
    let f = state_file();
    let Ok(meta) = std::fs::metadata(&f) else { return HashMap::new() };
    let when = meta.modified().ok();
    let mut g = CACHE.lock().unwrap_or_else(|p| p.into_inner());
    if let Some((t, p, m)) = g.as_ref() { if *t == when && *p == f { return m.clone(); } }
    let out = std::process::Command::new("python3").arg(script()).arg("show")
        .stdin(std::process::Stdio::null()).stderr(std::process::Stdio::null()).output();
    let map = match out { Ok(o) if o.status.success() => parse_show(&String::from_utf8_lossy(&o.stdout)), _ => HashMap::new() };
    *g = Some((when, f, map.clone()));
    map
}

/// Change drive letter…: `uuid`'s volume gets `letter` (D: to Y:), if no other drive has it. Returns the letter.
pub fn set(uuid: &str, letter: &str) -> Result<String, String> {
    let k = key(uuid);
    if k.is_empty() { return Err("It has no file system ID, so it can't keep a letter.".into()); }
    let letter = letter.trim().trim_end_matches(':').to_uppercase();
    if !crate::safety::letter_ok(&letter) { return Err("Pick a letter from D: to Y:.".into()); }
    let script = script();
    let dir = script.parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
    let code = concat!(
        "import sys, json\n",
        "sys.path.insert(0, sys.argv[1])\n",
        "import hearth_drive_letters as h\n",
        "try:\n",
        "    print(json.dumps({'ok': True, 'letter': h.set_letter(sys.argv[2], sys.argv[3])['letter']}))\n",
        "except h.Fail as e:\n",
        "    print(json.dumps({'ok': False, 'error': str(e)}))\n");
    let out = std::process::Command::new("python3").args(["-c", code, dir.as_str(), k.as_str(), letter.as_str()])
        .stdin(std::process::Stdio::null()).output().map_err(|e| format!("Couldn't change the letter: {e}"))?;
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let v: serde_json::Value = text.lines().rev().find_map(|l| serde_json::from_str(l).ok()).unwrap_or_default();
    *CACHE.lock().unwrap_or_else(|p| p.into_inner()) = None;
    if v["ok"] == true { return Ok(v["letter"].as_str().unwrap_or(&letter).to_string()); }
    if let Some(e) = v["error"].as_str() { return Err(format!("{e}.")); }
    Err(format!("Couldn't change the letter: {}", String::from_utf8_lossy(&out.stderr).lines().last().unwrap_or("hearth_drive_letters didn't answer")))
}

/// The letters a volume can be given (D: to Y:, less those other drives have), its own included.
pub fn free_for(uuid: &str, all: &HashMap<String, String>) -> Vec<String> {
    let mine = key(uuid);
    let taken: Vec<&String> = all.iter().filter(|(k, _)| **k != mine).map(|(_, l)| l).collect();
    "DEFGHIJKLMNOPQRSTUVWXY".chars().map(|c| c.to_string()).filter(|l| !taken.contains(&l)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_letters() {
        let m = parse_show(r#"{"letters": {"uuid:abc": {"letter": "e", "name": "USB"}, "mount:/x": {"letter": "F"}, "bad": {"letter": "C"}, "z": {"letter": "Z"}}, "file": "/x"}"#);
        assert_eq!(m.get("uuid:abc").map(String::as_str), Some("E"));
        assert_eq!(m.get("mount:/x").map(String::as_str), Some("F"));
        assert!(!m.contains_key("bad") && !m.contains_key("z"), "C: and Z: are never a drive's letter here");
        assert!(parse_show("not json").is_empty());
        let free = free_for("abc", &m);
        assert!(free.contains(&"E".to_string()), "its own letter is offered");
        assert!(!free.contains(&"F".to_string()), "another drive's isn't");
        assert_eq!(free.first().map(String::as_str), Some("D"));
        assert_eq!(key(""), "");
    }
}
