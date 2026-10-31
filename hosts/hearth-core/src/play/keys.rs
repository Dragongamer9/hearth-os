//! Settings › Games › Integrations: the optional services the player can connect with **their own** key or account
//! name (docs/HYDRA-FEATURES.md). Every one is off until the player turns it on and types its key; Hearth ships no
//! keys of its own and never signs in anywhere for the player.
//!
//! Kept in the person's own `accounts/integrations.json` (0600; it moves with their profile like their other
//! sign-ins). Keys are never sent to the page: it only learns whether one is set, and its last four characters.

use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};

/// The services and the fields each needs.
pub const SERVICES: &[(&str, &[&str])] = &[
    // Artwork for games you added yourself (and any game's custom pictures).
    ("steamgriddb", &["key"]),
    // How long a game takes to beat: IGDB's game_time_to_beats, through a Twitch developer app of your own.
    ("igdb", &["clientId", "clientSecret"]),
    // Steam achievements from Steam's official Web API, for games whose achievements aren't on this device.
    ("steamweb", &["key", "steamId"]),
    // RetroAchievements for your emulated games.
    ("retro", &["user", "key"]),
    // itch.io: your itch.io library and free games, downloaded with Hearth's download manager.
    ("itch", &["key"]),
];
/// Fields shown to the page as they are (not secrets).
const PLAIN: &[&str] = &["steamId", "user", "clientId"];

pub fn path() -> PathBuf { crate::paths::data_dir().join("accounts/integrations.json") }

pub fn load_from(p: &Path) -> Value {
    std::fs::read(p).ok().and_then(|b| serde_json::from_slice::<Value>(&b).ok()).filter(|v| v.is_object()).unwrap_or(json!({}))
}
pub fn load() -> Value { load_from(&path()) }

fn save_to(p: &Path, v: &Value) -> Result<(), String> {
    crate::paths::write_atomic(p, serde_json::to_string_pretty(v).unwrap_or_default().as_bytes()).map_err(|e| e.to_string())?;
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o600)).map_err(|e| e.to_string())
}

fn fields(service: &str) -> Result<&'static [&'static str], String> {
    SERVICES.iter().find(|(s, _)| *s == service).map(|(_, f)| *f).ok_or_else(|| "Unknown service".to_string())
}

/// A key as typed: trimmed, at most 200 characters, no spaces or control characters inside.
pub fn clean(value: &str) -> Result<String, String> {
    let v = value.trim();
    if v.chars().count() > 200 { return Err("That's too long for a key.".into()); }
    if v.chars().any(|c| c.is_control() || c.is_whitespace()) { return Err("A key has no spaces in it. Check what you typed.".into()); }
    Ok(v.to_string())
}

fn entry<'a>(v: &'a mut Value, service: &str) -> &'a mut Map<String, Value> {
    if !v.is_object() { *v = json!({}); }
    let o = v.as_object_mut().unwrap();
    let e = o.entry(service).or_insert(json!({"on": false}));
    if !e.is_object() { *e = json!({"on": false}); }
    e.as_object_mut().unwrap()
}

/// Sets one field (empty clears it). A service is turned on when all its fields are set the first time.
pub fn set_in(p: &Path, service: &str, field: &str, value: &str) -> Result<Value, String> {
    let fs = fields(service)?;
    if !fs.contains(&field) { return Err("Unknown field".into()); }
    let value = clean(value)?;
    if service == "steamweb" && field == "steamId" && !value.is_empty() && !(value.len() == 17 && value.chars().all(|c| c.is_ascii_digit())) {
        return Err("Your SteamID64 is the 17-digit number in your Steam profile's address (steamcommunity.com/profiles/…).".into());
    }
    let mut v = load_from(p);
    let was_complete = complete(&v, service);
    let e = entry(&mut v, service);
    if value.is_empty() { e.remove(field); } else { e.insert(field.to_string(), json!(value)); }
    if !was_complete && complete(&v, service) { entry(&mut v, service).insert("on".into(), json!(true)); }
    if !complete(&v, service) { entry(&mut v, service).insert("on".into(), json!(false)); }
    save_to(p, &v)?;
    Ok(status(&v))
}

pub fn set_on_in(p: &Path, service: &str, on: bool) -> Result<Value, String> {
    fields(service)?;
    let mut v = load_from(p);
    if on && !complete(&v, service) { return Err("Type its key first.".into()); }
    entry(&mut v, service).insert("on".into(), json!(on));
    save_to(p, &v)?;
    Ok(status(&v))
}

/// Forgets everything kept for a service.
pub fn clear_in(p: &Path, service: &str) -> Result<Value, String> {
    fields(service)?;
    let mut v = load_from(p);
    if let Some(o) = v.as_object_mut() { o.remove(service); }
    save_to(p, &v)?;
    Ok(status(&v))
}

fn complete(v: &Value, service: &str) -> bool {
    fields(service).map(|fs| fs.iter().all(|f| v[service][*f].as_str().map(|s| !s.is_empty()).unwrap_or(false))).unwrap_or(false)
}

/// What the page sees: per service, whether it's on, and per field whether it's set (plain fields in full, keys as
/// their last four characters only).
pub fn status(v: &Value) -> Value {
    let mut out = Map::new();
    for (s, fs) in SERVICES {
        let mut f = Map::new();
        for k in *fs {
            let val = v[*s][*k].as_str().unwrap_or("");
            let shown = if val.is_empty() { Value::Null } else if PLAIN.contains(k) { json!(val) } else {
                let tail: String = val.chars().rev().take(4).collect::<Vec<_>>().into_iter().rev().collect();
                json!(format!("••••{tail}"))
            };
            f.insert(k.to_string(), shown);
        }
        out.insert(s.to_string(), json!({"on": v[*s]["on"] == true && complete(v, s), "fields": f}));
    }
    Value::Object(out)
}

/// A service's fields, only when it's on and complete.
pub fn get_from(v: &Value, service: &str) -> Option<Value> {
    if v[service]["on"] == true && complete(v, service) { Some(v[service].clone()) } else { None }
}
pub fn get(service: &str) -> Option<Value> { get_from(&load(), service) }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_off_until_typed_and_never_shown_whole() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("accounts/integrations.json");
        let s = status(&load_from(&p));
        assert_eq!(s["steamgriddb"]["on"], false);
        assert!(get_from(&load_from(&p), "steamgriddb").is_none());
        let s = set_in(&p, "steamgriddb", "key", "  abcdef123456  ").unwrap();
        assert_eq!(s["steamgriddb"]["on"], true, "typing the only field turns it on");
        assert_eq!(s["steamgriddb"]["fields"]["key"], "••••3456");
        assert!(!s.to_string().contains("abcdef"), "the whole key never goes to the page");
        assert_eq!(get_from(&load_from(&p), "steamgriddb").unwrap()["key"], "abcdef123456");
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(std::fs::metadata(&p).unwrap().permissions().mode() & 0o777, 0o600);
        let s = set_on_in(&p, "steamgriddb", false).unwrap();
        assert_eq!(s["steamgriddb"]["on"], false);
        assert!(get_from(&load_from(&p), "steamgriddb").is_none(), "off means not used");
        let s = clear_in(&p, "steamgriddb").unwrap();
        assert!(s["steamgriddb"]["fields"]["key"].is_null());
    }

    #[test]
    fn two_field_services_and_checks() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("i.json");
        let s = set_in(&p, "retro", "user", "Dragon").unwrap();
        assert_eq!(s["retro"]["on"], false, "half set stays off");
        assert_eq!(s["retro"]["fields"]["user"], "Dragon", "an account name is shown as it is");
        assert!(set_on_in(&p, "retro", true).is_err());
        let s = set_in(&p, "retro", "key", "k3yk3yk3y").unwrap();
        assert_eq!(s["retro"]["on"], true);
        assert!(set_in(&p, "steamweb", "steamId", "12345").is_err(), "a SteamID64 has 17 digits");
        assert!(set_in(&p, "steamweb", "steamId", "76561198000000000").is_ok());
        assert!(set_in(&p, "nope", "key", "x").is_err());
        assert!(set_in(&p, "itch", "token", "x").is_err());
        assert!(set_in(&p, "itch", "key", "has space").is_err());
        assert!(set_in(&p, "itch", "key", &"x".repeat(201)).is_err());
        let s = set_in(&p, "retro", "key", "").unwrap();
        assert_eq!(s["retro"]["on"], false, "clearing a field turns it off");
    }
}
