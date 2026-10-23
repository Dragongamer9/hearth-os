//! What Documents remembers: the recent documents (newest first), where you were in each (a PDF's page and zoom, an
//! e-book's place), and your settings (dark pages, e-book font size and colours). One JSON file in
//! ~/.config/hearth-documents/state.json; the page stores what it likes in each document's entry.

use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};

const MAX_RECENT: usize = 30;

pub fn file() -> PathBuf {
    std::env::var_os("HEARTH_DOCUMENTS_STATE").map(PathBuf::from)
        .unwrap_or_else(|| dirs::config_dir().unwrap_or_else(std::env::temp_dir).join("hearth-documents").join("state.json"))
}

pub fn load_from(path: &Path) -> Value {
    let v: Value = std::fs::read_to_string(path).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(Value::Null);
    let mut o = v.as_object().cloned().unwrap_or_default();
    if !o.get("recent").map(Value::is_array).unwrap_or(false) { o.insert("recent".into(), json!([])); }
    if !o.get("settings").map(Value::is_object).unwrap_or(false) { o.insert("settings".into(), json!({})); }
    Value::Object(o)
}

fn save_to(path: &Path, v: &Value) -> Result<(), String> {
    if let Some(d) = path.parent() { std::fs::create_dir_all(d).map_err(|e| e.to_string())?; }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(v).unwrap_or_default()).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())
}

/// Everything, with each recent document marked as still there or not.
pub fn load() -> Value {
    let mut v = load_from(&file());
    if let Some(list) = v.get_mut("recent").and_then(Value::as_array_mut) {
        for e in list.iter_mut() {
            let there = e.get("path").and_then(Value::as_str).map(|p| Path::new(p).is_file()).unwrap_or(false);
            if let Some(o) = e.as_object_mut() { o.insert("there".into(), Value::Bool(there)); }
        }
    }
    v
}

/// Puts a document at the top of the recent list, merging `fields` (page, zoom, place…) into its entry.
pub fn remember_in(path: &Path, doc: &str, fields: &Value) -> Result<Value, String> {
    let mut v = load_from(path);
    let list = v.get_mut("recent").and_then(Value::as_array_mut).ok_or("bad state")?;
    let mut entry: Map<String, Value> = list.iter().find(|e| e.get("path").and_then(Value::as_str) == Some(doc))
        .and_then(|e| e.as_object().cloned()).unwrap_or_default();
    list.retain(|e| e.get("path").and_then(Value::as_str) != Some(doc));
    entry.insert("path".into(), Value::String(doc.into()));
    entry.insert("name".into(), Value::String(Path::new(doc).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()));
    if let Some(f) = fields.as_object() { for (k, val) in f { if k != "path" && k != "there" { entry.insert(k.clone(), val.clone()); } } }
    entry.insert("opened".into(), json!(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)));
    entry.remove("there");
    list.insert(0, Value::Object(entry));
    list.truncate(MAX_RECENT);
    save_to(path, &v)?;
    Ok(v)
}

pub fn remember(doc: &str, fields: &Value) -> Result<Value, String> { remember_in(&file(), doc, fields) }

/// Takes a document off the recent list.
pub fn forget(doc: &str) -> Result<(), String> {
    let path = file();
    let mut v = load_from(&path);
    if let Some(list) = v.get_mut("recent").and_then(Value::as_array_mut) { list.retain(|e| e.get("path").and_then(Value::as_str) != Some(doc)); }
    save_to(&path, &v)
}

/// Merges `settings` into the saved settings.
pub fn set_settings(settings: &Value) -> Result<(), String> {
    let path = file();
    let mut v = load_from(&path);
    if let (Some(dst), Some(src)) = (v.get_mut("settings").and_then(Value::as_object_mut), settings.as_object()) {
        for (k, val) in src { dst.insert(k.clone(), val.clone()); }
    }
    save_to(&path, &v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recent_documents() {
        let f = std::env::temp_dir().join(format!("hd-state-{}.json", std::process::id()));
        let _ = std::fs::remove_file(&f);
        assert_eq!(load_from(&f)["recent"], json!([]));
        remember_in(&f, "/a/one.pdf", &json!({"page": 3})).unwrap();
        remember_in(&f, "/a/two.epub", &json!({"cfi": "epubcfi(/6/4)"})).unwrap();
        let v = remember_in(&f, "/a/one.pdf", &json!({"zoom": "page-width"})).unwrap();
        let r = v["recent"].as_array().unwrap();
        assert_eq!(r.len(), 2);
        assert_eq!(r[0]["path"], "/a/one.pdf");
        assert_eq!(r[0]["page"], 3); // kept from before
        assert_eq!(r[0]["zoom"], "page-width");
        assert_eq!(r[0]["name"], "one.pdf");
        for k in 0..40 { remember_in(&f, &format!("/b/{k}.pdf"), &json!({})).unwrap(); }
        assert_eq!(load_from(&f)["recent"].as_array().unwrap().len(), MAX_RECENT);
        std::fs::write(&f, b"{broken").unwrap();
        assert_eq!(load_from(&f)["settings"], json!({}));
        std::fs::remove_file(&f).unwrap();
    }
}
