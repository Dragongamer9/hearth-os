//! What Hearth Player remembers, as small JSON files in your config folder (~/.config/hearth-player): where you
//! stopped in each video, settings, podcast subscriptions and played positions, radio favourites. Nothing leaves
//! this computer.

use serde_json::Value;
use std::path::PathBuf;

pub fn dir() -> PathBuf {
    std::env::var_os("HEARTH_PLAYER_CONFIG").map(PathBuf::from)
        .unwrap_or_else(|| dirs::config_dir().unwrap_or_else(std::env::temp_dir).join("hearth-player"))
}

fn path(name: &str) -> Result<PathBuf, String> {
    if name.is_empty() || name.len() > 40 || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err("bad store name".into());
    }
    Ok(dir().join(format!("{name}.json")))
}

pub fn load(name: &str) -> Value {
    path(name).ok().and_then(|p| std::fs::read(p).ok()).and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null)
}

/// Saves through a temporary file, so a crash never leaves half a file.
pub fn save(name: &str, v: &Value) -> Result<(), String> {
    let p = path(name)?;
    std::fs::create_dir_all(dir()).map_err(|e| e.to_string())?;
    let tmp = p.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(v).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &p).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn round_trip() {
        let d = std::env::temp_dir().join(format!("hp-store-{}", std::process::id()));
        std::env::set_var("HEARTH_PLAYER_CONFIG", &d);
        super::save("resume", &serde_json::json!({"a": 1})).unwrap();
        assert_eq!(super::load("resume")["a"], 1);
        assert!(super::save("../x", &serde_json::json!(1)).is_err());
        assert!(super::load("nothing").is_null());
        let _ = std::fs::remove_dir_all(&d);
    }
}
