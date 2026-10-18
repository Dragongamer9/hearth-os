//! The cached list of games on a store account: {fetched, games: [{id, title, art, added, lastPlayed, note}]}
//! in ~/.local/share/nimbus/cache (the same files the Electron host keeps).

use serde_json::{json, Value};

#[derive(Debug, Clone, Default)]
pub struct Owned { pub fetched: u64, pub games: Vec<Value> }

fn file(name: &str) -> std::path::PathBuf { crate::paths::data_dir().join("cache").join(name) }

impl Owned {
    pub fn load(name: &str) -> Owned {
        let v: Value = std::fs::read(file(name)).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(json!({}));
        Owned { fetched: v["fetched"].as_u64().unwrap_or(0), games: v["games"].as_array().cloned().unwrap_or_default() }
    }
    pub fn save(&self, name: &str) { let _ = crate::paths::write_atomic(&file(name), json!({"fetched": self.fetched, "games": self.games}).to_string().as_bytes()); }
    pub fn remove(name: &str) { let _ = std::fs::remove_file(file(name)); }
    pub fn find(&self, id: &str) -> Option<&Value> { self.games.iter().find(|g| g["id"] == id) }
}

/// http:// and //host links as https://
pub fn https(u: &str) -> String { if let Some(r) = u.strip_prefix("//") { format!("https://{r}") } else if let Some(r) = u.strip_prefix("http://") { format!("https://{r}") } else { u.to_string() } }

/// "2023-05-01T10:00:00.000Z" → ms (0 if it can't be read).
pub fn ms(iso: &str) -> u64 { chrono::DateTime::parse_from_rfc3339(iso).map(|d| d.timestamp_millis().max(0) as u64).unwrap_or(0) }

/// The first JSON value in text that may have log lines around it (legendary prints both).
pub fn json_in(text: &str, open: char) -> Option<Value> {
    let mut start = 0;
    while let Some(i) = text[start..].find(open) {
        let at = start + i;
        let mut de = serde_json::Deserializer::from_str(&text[at..]).into_iter::<Value>();
        if let Some(Ok(v)) = de.next() { return Some(v); }
        start = at + 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn json_after_log_lines() {
        assert_eq!(json_in("[cli] INFO: x\n[{\"app_name\":\"A\"}]\n", '['), Some(json!([{"app_name": "A"}])));
        assert_eq!(ms("2023-05-01T10:00:00.000Z"), 1682935200000);
        assert_eq!(https("//images.gog.com/a"), "https://images.gog.com/a");
    }
}
