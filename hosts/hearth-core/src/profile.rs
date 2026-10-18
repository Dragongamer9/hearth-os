//! Your profile in Hearth: name, status, picture, accent color and background (library.json "profile",
//! pictures in ~/.local/share/nimbus/profile). Port of accounts.js's Profile.

use crate::store::Store;
use serde_json::{json, Value};

const IMAGE_EXT: [&str; 5] = [".png", ".jpg", ".jpeg", ".webp", ".gif"];

pub fn image(kind: &str) -> Option<std::path::PathBuf> {
    let dir = crate::paths::data_dir().join("profile");
    IMAGE_EXT.iter().map(|e| dir.join(format!("{kind}{e}"))).find(|p| p.exists())
}

fn data(store: &Store) -> Value {
    let mut p = json!({"name": null, "status": null, "avatar": "initials", "accent": "#3fd0b8", "background": "default", "imageStamp": 0});
    if let Some(saved) = store.data["profile"].as_object() { for (k, v) in saved { p[k] = v.clone(); } }
    p
}

pub fn get(store: &Store) -> Value {
    let p = data(store);
    let stamp = p["imageStamp"].as_u64().unwrap_or(0);
    let url = |kind: &str| image(kind).map(|f| format!("{}?v={stamp}", crate::library::art_url(&f.to_string_lossy())));
    let user = std::env::var("USER").unwrap_or_default();
    json!({
        "name": p["name"].as_str().filter(|s| !s.is_empty()).map(str::to_string).unwrap_or(user),
        "status": p["status"], "avatar": p["avatar"], "accent": p["accent"], "background": p["background"],
        "customAvatar": url("avatar"), "customBackground": url("background"),
        "storeAvatars": {"steam": null, "xbox": null, "gog": null},
    })
}

pub fn set(store: &mut Store, c: &Value) -> Result<Value, String> {
    let clip = |v: &Value, n: usize| -> Value { let t = v.as_str().unwrap_or("").trim(); if t.is_empty() { Value::Null } else { json!(t.chars().take(n).collect::<String>()) } };
    let mut p = data(store);
    if !c["name"].is_null() { p["name"] = clip(&c["name"], 32); }
    if !c["status"].is_null() { p["status"] = clip(&c["status"], 60); }
    if let Some(a) = c["avatar"].as_str() { p["avatar"] = json!(a); }
    if let Some(a) = c["accent"].as_str() { if a.len() == 7 && a.starts_with('#') && a[1..].chars().all(|ch| ch.is_ascii_hexdigit()) { p["accent"] = json!(a); } }
    if let Some(b) = c["background"].as_str() { p["background"] = json!(b); }
    store.update(|d| { d.insert("profile".into(), p); }).map_err(|e| e.to_string())?;
    Ok(get(store))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn keeps_good_values_only() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = Store::open(dir.path().join("l.json"));
        let p = set(&mut s, &json!({"name": "  Sam  ", "accent": "#zzzzzz", "background": "preset:3"})).unwrap();
        assert_eq!(p["name"], "Sam"); assert_eq!(p["accent"], "#3fd0b8"); assert_eq!(p["background"], "preset:3");
        let p = set(&mut s, &json!({"accent": "#ff9f43"})).unwrap();
        assert_eq!(p["accent"], "#ff9f43"); assert_eq!(p["name"], "Sam");
    }
}
