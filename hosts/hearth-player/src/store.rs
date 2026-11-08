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

/// A file a power cut damaged is put back from its .bak (so the next save can't replace your podcasts or radio
/// favourites with an empty list).
pub fn load(name: &str) -> Value {
    path(name).ok().and_then(|p| crate::appfile::read_json(&p)).unwrap_or(Value::Null)
}

/// Saves through a temporary file (a crash never leaves half a file), keeping the last good one as NAME.json.bak.
pub fn save(name: &str, v: &Value) -> Result<(), String> {
    crate::appfile::write_json(&path(name)?, v).map_err(|e| e.to_string())
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
        // Damaged by a power cut: the last good copy comes back instead of nothing (then saved over everything).
        super::save("radio", &serde_json::json!({"favs": ["a"]})).unwrap();
        super::save("radio", &serde_json::json!({"favs": ["a", "b"]})).unwrap();
        std::fs::write(d.join("radio.json"), b"{\"favs\": [\"a").unwrap();
        assert_eq!(super::load("radio")["favs"], serde_json::json!(["a"]));
        let _ = std::fs::remove_dir_all(&d);
    }
}
