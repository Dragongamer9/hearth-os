//! Per-game settings (game page › Game settings): Proton version, launch options, environment variables,
//! MangoHud, GameMode, FSR, frame-rate cap, power limit, cloud saves. Port of library/game-options.js.

use crate::store::Store;
use serde_json::{json, Map, Value};
use std::path::Path;

pub fn defaults() -> Value {
    json!({"proton": "", "args": "", "env": "", "mangohud": false, "gamemode": false, "fsr": false, "fpsLimit": 0, "tdp": 0, "cloudSaves": true})
}

pub fn get(store: &Store, id: &str) -> Value {
    let mut o = defaults();
    if let Some(saved) = store.data["gameOptions"][id].as_object() { for (k, v) in saved { if o.get(k).is_some() { o[k] = v.clone(); } } }
    o
}

/// Keeps only known settings, each the right kind of value.
pub fn clean(patch: &Value) -> Map<String, Value> {
    let d = defaults();
    let mut out = Map::new();
    for (k, def) in d.as_object().unwrap() {
        let Some(v) = patch.get(k) else { continue };
        let c = match def {
            Value::Bool(_) => json!(v.as_bool().unwrap_or(v.as_f64().map(|n| n != 0.0).unwrap_or(false))),
            Value::Number(_) => { let max = if k == "tdp" { 120.0 } else { 240.0 }; json!(v.as_f64().or(v.as_str().and_then(|s| s.parse().ok())).unwrap_or(0.0).round().clamp(0.0, max) as u64) }
            _ => json!(v.as_str().map(|s| s.chars().take(2000).collect::<String>()).unwrap_or_default()),
        };
        out.insert(k.clone(), c);
    }
    out
}

pub fn set(store: &mut Store, id: &str, patch: &Value) -> Result<(), String> {
    let c = clean(patch);
    let id = id.to_string();
    store.update(|d| {
        let all = d.entry("gameOptions").or_insert(json!({}));
        let cur = all.as_object_mut().unwrap().entry(id).or_insert(json!({}));
        for (k, v) in c { cur[k] = v; }
    }).map_err(|e| e.to_string())
}

/// Proton versions to pick from: umu's own, then any installed (Steam, ProtonUp, Bazzite's).
pub fn proton_choices(home: &Path) -> Vec<Value> {
    let mut out = vec![json!({"id": "", "label": "Default", "sub": "UMU-Proton, the newest, downloaded when needed"}),
                       json!({"id": "GE-Proton", "label": "GE-Proton (newest)", "sub": "Community build with extra fixes, downloaded when needed"})];
    let roots = [home.join(".local/share/Steam/compatibilitytools.d"), home.join(".steam/root/compatibilitytools.d"),
        home.join(".var/app/com.valvesoftware.Steam/data/Steam/compatibilitytools.d"), "/usr/share/steam/compatibilitytools.d".into(),
        home.join(".local/share/Steam/steamapps/common"), home.join(".var/app/com.valvesoftware.Steam/data/Steam/steamapps/common")];
    let mut seen = std::collections::HashSet::new();
    for root in roots.iter() {
        let Ok(rd) = std::fs::read_dir(root) else { continue };
        let mut names: Vec<String> = rd.flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect();
        names.sort(); names.reverse();
        for n in names {
            let dir = root.join(&n);
            if !dir.join("proton").exists() { continue; }
            let real = std::fs::canonicalize(&dir).unwrap_or(dir.clone());
            if !seen.insert(real) || !seen.insert(Path::new(&n).to_path_buf()) { continue; }
            out.push(json!({"id": dir, "label": n, "sub": if root.to_string_lossy().contains("steamapps") { "Installed with Steam" } else { "Installed" }}));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn keeps_known_settings_only() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = Store::open(dir.path().join("l.json"));
        set(&mut s, "epic:x", &json!({"mangohud": 1, "proton": "GE-Proton", "bogus": "no", "fpsLimit": "45", "tdp": 999})).unwrap();
        let o = get(&s, "epic:x");
        assert_eq!(o["mangohud"], true); assert_eq!(o["proton"], "GE-Proton"); assert_eq!(o["fpsLimit"], 45); assert_eq!(o["tdp"], 120);
        assert!(o.get("bogus").is_none());
        assert_eq!(get(&s, "other")["cloudSaves"], true);
    }
}
