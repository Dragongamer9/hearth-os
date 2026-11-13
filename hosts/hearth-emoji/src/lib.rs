//! Hearth's emoji and symbol picker: what doesn't need a window. Your recent picks, how often you picked each
//! (the Most used row) and skin tone (~/.local/share/hearth/emoji.json), and how a pick reaches the app you were in: copied to the clipboard
//! (wl-copy) and typed (wtype; xdotool on X11) a moment after the picker's window has closed and that app has
//! the keyboard again.

#[path = "../../hearth-apptheme/appfile.rs"]
mod appfile;

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::{Command, Stdio};

pub const RECENT_MAX: usize = 36;
/// How many picks' counts are kept for Most used (the least used go first when there are more).
pub const USES_MAX: usize = 200;

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Saved {
    /// Newest first.
    pub recent: Vec<String>,
    /// 0 = none (yellow), 1-5 = light to dark.
    pub tone: u8,
    /// How many times each was picked (Most used). Files from before 1.10 have none: Most used starts from Recent.
    pub uses: BTreeMap<String, u32>,
}

pub fn data_file() -> PathBuf {
    let base = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        .or_else(|| std::env::var_os("APPDATA").map(PathBuf::from))
        .unwrap_or_else(std::env::temp_dir);
    base.join("hearth").join("emoji.json")
}

/// A file a power cut damaged is put back from emoji.json.bak, so the next pick can't save an empty list and the
/// default skin tone over yours.
pub fn load() -> Saved {
    appfile::read_json(&data_file()).and_then(|v| serde_json::from_value(v).ok()).map(clean).unwrap_or_default()
}

pub fn save(s: &Saved) -> Result<(), String> {
    appfile::write_json(&data_file(), &serde_json::to_value(s).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

/// Anything odd in the file dropped: too many, too long, empty, repeated, a tone that doesn't exist.
pub fn clean(mut s: Saved) -> Saved {
    let mut seen = Vec::new();
    for r in s.recent.drain(..) {
        if !r.is_empty() && r.chars().count() <= 40 && !seen.contains(&r) { seen.push(r); }
        if seen.len() == RECENT_MAX { break; }
    }
    s.recent = seen;
    if s.tone > 5 { s.tone = 0; }
    s.uses.retain(|k, v| !k.is_empty() && k.chars().count() <= 40 && *v > 0);
    trim_uses(&mut s.uses);
    s
}

/// At most USES_MAX counts: the least used (then the last in order) are dropped.
fn trim_uses(uses: &mut BTreeMap<String, u32>) {
    while uses.len() > USES_MAX {
        let least = uses.iter().min_by(|a, b| a.1.cmp(b.1).then_with(|| b.0.cmp(a.0))).map(|(k, _)| k.clone());
        match least { Some(k) => { uses.remove(&k); } None => break }
    }
}

/// The Most used row: by how often you picked them, then how recently; picks without a count (older files) follow
/// in recent order. At most `n`.
pub fn most_used(s: &Saved, n: usize) -> Vec<String> {
    let pos = |e: &str| s.recent.iter().position(|r| r == e).unwrap_or(usize::MAX);
    let mut counted: Vec<(&String, &u32)> = s.uses.iter().collect();
    counted.sort_by(|a, b| b.1.cmp(a.1).then_with(|| pos(a.0.as_str()).cmp(&pos(b.0.as_str()))).then_with(|| a.0.cmp(b.0)));
    let mut out: Vec<String> = counted.into_iter().map(|(k, _)| k.clone()).collect();
    for r in &s.recent { if !out.contains(r) { out.push(r.clone()); } }
    out.truncate(n);
    out
}

/// The pick goes to the front of the recent list.
pub fn remember(mut s: Saved, pick: &str) -> Saved {
    s.recent.retain(|r| r != pick);
    s.recent.insert(0, pick.to_string());
    s.recent.truncate(RECENT_MAX);
    *s.uses.entry(pick.to_string()).or_insert(0) += 1;
    trim_uses(&mut s.uses);
    s
}

/// A pick is something to type: one emoji, a symbol or a kaomoji. Never control characters.
pub fn valid_pick(text: &str) -> bool {
    !text.is_empty() && text.chars().count() <= 40 && !text.chars().any(|c| c.is_control())
}

fn have(cmd: &str) -> bool {
    std::env::var_os("PATH").map(|p| std::env::split_paths(&p).any(|d| d.join(cmd).is_file())).unwrap_or(false)
}

/// Puts the text on the clipboard. wl-copy keeps serving it after the picker has closed.
pub fn copy(text: &str) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        if std::env::var_os("WAYLAND_DISPLAY").is_some() && have("wl-copy") {
            return Command::new("wl-copy").arg("--").arg(text).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null())
                .spawn().map(|_| ()).map_err(|e| e.to_string());
        }
        if have("xclip") {
            let mut c = Command::new("xclip").args(["-selection", "clipboard"]).stdin(Stdio::piped()).spawn().map_err(|e| e.to_string())?;
            if let Some(mut i) = c.stdin.take() { use std::io::Write; let _ = i.write_all(text.as_bytes()); }
            return Ok(());
        }
        Err("No clipboard tool (wl-copy) is installed.".into())
    }
    #[cfg(windows)]
    {
        // (Ember gets its own picker later; this keeps the app usable there.)
        let mut c = Command::new("powershell").args(["-NoProfile", "-Command", "$input | Set-Clipboard"]).stdin(Stdio::piped()).spawn().map_err(|e| e.to_string())?;
        if let Some(mut i) = c.stdin.take() { use std::io::Write; let _ = i.write_all(text.as_bytes()); }
        Ok(())
    }
    #[cfg(not(any(target_os = "linux", windows)))]
    { let _ = text; Err("Not on this system".into()) }
}

/// The command that types the text into whatever has the keyboard, once the picker has gone (a short wait first,
/// so the app you were in has the keyboard back). None: nothing here can type.
pub fn type_command(text: &str, wayland: bool, has_wtype: bool, has_xdotool: bool) -> Option<Vec<String>> {
    let wait = "sleep 0.35; exec \"$@\"".to_string();
    if wayland && has_wtype {
        Some(vec!["sh".into(), "-c".into(), wait, "sh".into(), "wtype".into(), "--".into(), text.into()])
    } else if has_xdotool {
        Some(vec!["sh".into(), "-c".into(), wait, "sh".into(), "xdotool".into(), "type".into(), "--".into(), text.into()])
    } else {
        None
    }
}

/// Starts typing it (in the background: the picker closes right after). False: it can only be copied here.
pub fn type_later(text: &str) -> bool {
    let wayland = std::env::var_os("WAYLAND_DISPLAY").is_some();
    match type_command(text, wayland, have("wtype"), have("xdotool")) {
        Some(argv) => Command::new(&argv[0]).args(&argv[1..]).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().is_ok(),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recent_list() {
        let mut s = Saved::default();
        for e in ["😀", "👍", "😀"] { s = remember(s, e); }
        assert_eq!(s.recent, vec!["😀", "👍"]);
        for i in 0..50 { s = remember(s, &format!("x{i}")); }
        assert_eq!(s.recent.len(), RECENT_MAX);
        assert_eq!(s.recent[0], "x49");
    }

    #[test]
    fn most_used_row() {
        let mut s = Saved::default();
        for e in ["😂", "🔥", "🔥", "👍", "🔥", "😂"] { s = remember(s, e); }
        assert_eq!(most_used(&s, 10), vec!["🔥", "😂", "👍"]);
        assert_eq!(most_used(&s, 2), vec!["🔥", "😂"]);
        // A file from before 1.10 (recent only): Most used is the recent list.
        let old = Saved { recent: vec!["a".into(), "b".into()], ..Default::default() };
        assert_eq!(most_used(&old, 5), vec!["a", "b"]);
        // The counts never grow past USES_MAX.
        for i in 0..(USES_MAX + 30) { s = remember(s, &format!("x{i}")); }
        assert!(s.uses.len() <= USES_MAX);
        assert_eq!(s.uses.get("🔥"), Some(&3));
        // An old file without "uses" still reads.
        let v: Saved = serde_json::from_str(r#"{"recent":["a"],"tone":1}"#).unwrap();
        assert!(v.uses.is_empty());
    }

    #[test]
    fn cleaning() {
        let s = clean(Saved { recent: vec!["".into(), "a".into(), "a".into(), "b".repeat(50)], tone: 9, ..Default::default() });
        assert_eq!(s, Saved { recent: vec!["a".into()], tone: 0, ..Default::default() });
    }

    #[test]
    fn picks() {
        assert!(valid_pick("🧑🏽‍💻"));
        assert!(valid_pick("¯\\_(ツ)_/¯"));
        assert!(!valid_pick(""));
        assert!(!valid_pick("a\nrm -rf"));
    }

    #[test]
    fn typing() {
        let w = type_command("é", true, true, true).unwrap();
        assert_eq!(&w[4..], &["wtype", "--", "é"]);
        let x = type_command("é", false, true, true).unwrap();
        assert_eq!(&x[4..], &["xdotool", "type", "--", "é"]);
        assert!(type_command("é", true, false, false).is_none());
        // The text is an argument, never part of the shell's script.
        assert!(!w[2].contains('é'));
    }

    #[test]
    fn saved_file_round_trip() {
        let d = std::env::temp_dir().join(format!("hearth-emoji-test-{}", std::process::id()));
        std::env::set_var("XDG_DATA_HOME", &d);
        let s = Saved { recent: vec!["🔥".into()], tone: 3, uses: BTreeMap::from([("🔥".to_string(), 2)]) };
        save(&s).unwrap();
        assert_eq!(load(), s);
        // A power cut that left the file empty: the last good copy comes back (the tone isn't reset).
        save(&remember(s.clone(), "🎉")).unwrap();
        std::fs::write(data_file(), b"").unwrap();
        assert_eq!(load(), s);
        let _ = std::fs::remove_dir_all(&d);
    }
}
