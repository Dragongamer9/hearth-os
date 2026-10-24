//! Hearth's on-screen keyboard (ui/osk/hearth-osk.js, docs/OSK.md): what Game Mode keeps on disk, so Desktop Mode's
//! keyboard (desktop/shell/Keyboard.qml) reads the same choices and learned words.
//!   ~/.config/nimbus/osk.json         its settings (Settings › Keyboard)
//!   ~/.local/share/nimbus/osk-words.json   learned words: {version, learned: {lang: {word: n}}, next: {...}, recentEmoji}
//! and the system's keyboard layouts (Desktop Mode's Settings › Keyboard, XKB) for its Automatic layout.

use serde_json::{json, Value};
use std::path::{Path, PathBuf};

use crate::paths;

pub fn settings_path() -> PathBuf { paths::config_dir().join("osk.json") }
pub fn words_path() -> PathBuf { paths::data_dir().join("osk-words.json") }

fn read_json(p: &Path) -> Value {
    std::fs::read_to_string(p).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok()).filter(|v| v.is_object()).unwrap_or_else(|| json!({}))
}

/// The value of KEY= in an environment-style file (KEY=value, KEY="value").
fn env_value(text: &str, key: &str) -> Option<String> {
    text.lines().filter_map(|l| {
        let l = l.trim();
        if l.starts_with('#') { return None; }
        let (k, v) = l.split_once('=')?;
        (k.trim().trim_start_matches("export ").trim() == key).then(|| v.trim().trim_matches('"').trim_matches('\'').to_string())
    }).last().filter(|v| !v.is_empty())
}

/// The system's keyboard layouts (XKB names: "us", "de"...), in order: Desktop Mode's own (Settings › Keyboard writes
/// ~/.config/labwc/environment), the session's, then the system's (Debian /etc/default/keyboard, Fedora's X11 config).
pub fn system_layouts() -> Vec<String> {
    let split = |s: &str| s.split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect::<Vec<_>>();
    let home = paths::home();
    let cfg = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| home.join(".config"));
    if let Some(v) = std::fs::read_to_string(cfg.join("labwc/environment")).ok().and_then(|t| env_value(&t, "XKB_DEFAULT_LAYOUT")) { return split(&v); }
    if let Ok(v) = std::env::var("XKB_DEFAULT_LAYOUT") { if !v.trim().is_empty() { return split(&v); } }
    if let Some(v) = std::fs::read_to_string("/etc/default/keyboard").ok().and_then(|t| env_value(&t, "XKBLAYOUT")) { return split(&v); }
    if let Ok(t) = std::fs::read_to_string("/etc/X11/xorg.conf.d/00-keyboard.conf") {
        for l in t.lines() {
            let l = l.trim();
            if l.starts_with("Option") && l.contains("\"XkbLayout\"") {
                if let Some(v) = l.split('"').nth(3) { return split(v); }
            }
        }
    }
    Vec::new()
}

/// osk.get: {settings, learned, systemLayouts}
pub fn get() -> Value {
    json!({"settings": read_json(&settings_path()), "learned": read_json(&words_path()), "systemLayouts": system_layouts()})
}

/// osk.set: the settings, as the page has them (only known keys, small values).
pub fn set(settings: &Value) -> Result<(), String> {
    let Some(obj) = settings.as_object() else { return Err("settings must be an object".into()) };
    const KEYS: &[&str] = &["layouts", "suggestions", "learn", "autocap", "doubleSpace", "fast", "shape", "size", "position", "theme", "haptics", "trail"];
    let clean: serde_json::Map<String, Value> = obj.iter().filter(|(k, v)| KEYS.contains(&k.as_str()) && v.to_string().len() < 400).map(|(k, v)| (k.clone(), v.clone())).collect();
    let text = serde_json::to_string_pretty(&Value::Object(clean)).map_err(|e| e.to_string())?;
    paths::write_atomic(&settings_path(), text.as_bytes()).map_err(|e| format!("couldn't save the keyboard's settings: {e}"))
}

/// Two sets of learned words together (each word's higher count): Game Mode and Desktop Mode both add to the file.
pub fn merge(old: &Value, new: &Value) -> Value {
    let mut out = old.clone();
    if !out.is_object() { out = json!({}); }
    out["version"] = json!(1);
    for part in ["learned", "next"] {
        let mut langs = out[part].as_object().cloned().unwrap_or_default();
        if let Some(n) = new[part].as_object() {
            for (lang, words) in n {
                let mut have = langs.get(lang).and_then(|v| v.as_object()).cloned().unwrap_or_default();
                if let Some(w) = words.as_object() {
                    for (word, val) in w {
                        if word.chars().count() > 40 { continue; }
                        if part == "learned" {
                            let a = have.get(word).and_then(|v| v.as_u64()).unwrap_or(0);
                            let b = val.as_u64().unwrap_or(0).min(9999);
                            have.insert(word.clone(), json!(a.max(b)));
                        } else if let Some(nexts) = val.as_object() {
                            let mut m = have.get(word).and_then(|v| v.as_object()).cloned().unwrap_or_default();
                            for (nw, c) in nexts {
                                let a = m.get(nw).and_then(|v| v.as_u64()).unwrap_or(0);
                                m.insert(nw.clone(), json!(a.max(c.as_u64().unwrap_or(0).min(9999))));
                            }
                            have.insert(word.clone(), Value::Object(m));
                        }
                    }
                }
                langs.insert(lang.clone(), Value::Object(have));
            }
        }
        out[part] = Value::Object(langs);
    }
    if let Some(r) = new["recentEmoji"].as_array() { out["recentEmoji"] = json!(r.iter().take(30).cloned().collect::<Vec<_>>()); }
    out
}

/// osk.learn: what the page learned, merged into the file.
pub fn learn(data: &Value) -> Result<(), String> {
    if !data.is_object() { return Err("expected the learned words".into()); }
    let merged = merge(&read_json(&words_path()), data);
    let text = serde_json::to_string(&merged).map_err(|e| e.to_string())?;
    if text.len() > 4_000_000 { return Err("too many learned words".into()); }
    paths::write_atomic(&words_path(), text.as_bytes()).map_err(|e| format!("couldn't save learned words: {e}"))
}

/// osk.clear: forgets every learned word.
pub fn clear() -> Result<(), String> {
    let p = words_path();
    if p.exists() { std::fs::remove_file(&p).map_err(|e| format!("couldn't clear learned words: {e}"))?; }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_values() {
        assert_eq!(env_value("# x\nXKB_DEFAULT_LAYOUT=us,de\n", "XKB_DEFAULT_LAYOUT").as_deref(), Some("us,de"));
        assert_eq!(env_value("XKBLAYOUT=\"fr\"\n", "XKBLAYOUT").as_deref(), Some("fr"));
        assert_eq!(env_value("XKBLAYOUT=\n", "XKBLAYOUT"), None);
    }

    #[test]
    fn merging_keeps_the_higher_counts() {
        let a = json!({"learned": {"en": {"hello": 3, "hearth": 1}}, "next": {"en": {"hello": {"there": 2}}}});
        let b = json!({"learned": {"en": {"hello": 1, "ember": 2}, "fr": {"bonjour": 1}}, "next": {"en": {"hello": {"there": 5, "you": 1}}}, "recentEmoji": ["🔥"]});
        let m = merge(&a, &b);
        assert_eq!(m["learned"]["en"]["hello"], 3);
        assert_eq!(m["learned"]["en"]["ember"], 2);
        assert_eq!(m["learned"]["en"]["hearth"], 1);
        assert_eq!(m["learned"]["fr"]["bonjour"], 1);
        assert_eq!(m["next"]["en"]["hello"]["there"], 5);
        assert_eq!(m["next"]["en"]["hello"]["you"], 1);
        assert_eq!(m["recentEmoji"][0], "🔥");
    }
}
