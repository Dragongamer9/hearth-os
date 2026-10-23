//! JSON files: read (an empty object when missing or damaged) and written safely (a temporary file first, then
//! renamed over the old one, so a crash or a full disk never leaves half a file).

use serde_json::Value;
use std::path::Path;

pub fn load(p: &Path) -> Value {
    std::fs::read_to_string(p).ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .filter(Value::is_object)
        .unwrap_or_else(|| Value::Object(Default::default()))
}

pub fn save(p: &Path, v: &Value) -> Result<(), String> {
    write_bytes(p, &serde_json::to_vec_pretty(v).map_err(|e| e.to_string())?)
}

pub fn write_bytes(p: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(dir) = p.parent() { std::fs::create_dir_all(dir).map_err(|e| format!("Couldn't make {}: {e}", dir.display()))?; }
    let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let tmp = p.with_file_name(format!(".{name}.tmp{}", std::process::id()));
    std::fs::write(&tmp, bytes).map_err(|e| format!("Couldn't save {}: {e}", p.display()))?;
    std::fs::rename(&tmp, p).map_err(|e| { let _ = std::fs::remove_file(&tmp); format!("Couldn't save {}: {e}", p.display()) })
}

/// When a file last changed, in ms (0 when it isn't there): the pages and the service look again when it moves.
pub fn mtime(p: &Path) -> i64 {
    std::fs::metadata(p).and_then(|m| m.modified()).ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// A short unique id: time plus a counter, in base 36.
pub fn new_id(prefix: &str) -> String {
    use std::sync::atomic::{AtomicU32, Ordering};
    static N: AtomicU32 = AtomicU32::new(0);
    let n = N.fetch_add(1, Ordering::SeqCst);
    format!("{prefix}{}{}", radix36(crate::now_ms() as u64), radix36((n as u64) ^ (std::process::id() as u64 & 0xfff) << 8))
}

fn radix36(mut v: u64) -> String {
    const D: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    if v == 0 { return "0".into(); }
    let mut s = Vec::new();
    while v > 0 { s.push(D[(v % 36) as usize]); v /= 36; }
    s.reverse();
    String::from_utf8(s).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    #[test]
    fn save_and_load() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("a/b.json");
        assert!(super::load(&p).as_object().unwrap().is_empty());
        super::save(&p, &serde_json::json!({"x": 1})).unwrap();
        assert_eq!(super::load(&p)["x"], 1);
        std::fs::write(&p, "{broken").unwrap();
        assert!(super::load(&p).as_object().unwrap().is_empty());
        assert!(super::mtime(&p) > 0);
        assert_ne!(super::new_id("e"), super::new_id("e"));
    }
}
