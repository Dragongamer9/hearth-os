//! What Hearth Forge knows and how it reaches its service: the list of every model (helper/models.json, built in),
//! hearth-forge.py (built in too, written next to it so a Hearth Store update of the app updates both), the service on
//! the Nvidia PC (http://127.0.0.1:8191, or that PC's Tailscale name from another device) and the models made, kept
//! in ~/Hearth/Forge with their photo, words and settings.

use hearth_oss::http::{self, Req};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub const HELPER: &str = include_str!("../helper/hearth-forge.py");
pub const MODELS_JSON: &str = include_str!("../helper/models.json");
pub const PORT: u16 = 8191;

#[derive(Debug, Clone, Default)]
pub struct Model {
    pub id: String,
    pub name: String,
    pub maker: String,
    pub group: String,
    pub makes: String,
    pub memory: String,
    /// ok, warn, bad, unknown, gated
    pub fit: String,
    pub fit_words: String,
    pub licence: String,
    pub free_for_paid: bool,
    pub pbr: bool,
    pub inputs: Vec<String>,
    pub default: bool,
    pub best: u32,
    pub runner: bool,
    pub pick: String,
    pub fits_note: String,
    pub gated: String,
    pub no_source: String,
    pub page: String,
    pub vram_all: f64,
}

pub struct Catalog {
    pub groups: Vec<(String, String)>,
    pub models: Vec<Model>,
    pub tripo_costs: Value,
}

fn s(v: &Value, k: &str) -> String {
    v.get(k).and_then(Value::as_str).unwrap_or("").to_string()
}

pub fn catalog() -> Catalog {
    let v: Value = serde_json::from_str(MODELS_JSON).unwrap_or(Value::Null);
    let groups = v.get("groups").and_then(Value::as_array).map(|a| a.iter().map(|g| (s(g, "id"), s(g, "label"))).collect()).unwrap_or_default();
    let models = v
        .get("models")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .map(|m| Model {
                    id: s(m, "id"),
                    name: s(m, "name"),
                    maker: s(m, "maker"),
                    group: s(m, "group"),
                    makes: s(m, "makes"),
                    memory: s(m, "memory"),
                    fit: s(m, "fit"),
                    fit_words: s(m, "fitWords"),
                    licence: s(m, "licence"),
                    free_for_paid: m.get("freeForPaid").and_then(Value::as_bool).unwrap_or(false),
                    pbr: m.get("pbr").and_then(Value::as_bool).unwrap_or(false),
                    inputs: m.get("inputs").and_then(Value::as_array).map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default(),
                    default: m.get("default").and_then(Value::as_bool).unwrap_or(false),
                    best: m.get("best").and_then(Value::as_u64).unwrap_or(0) as u32,
                    runner: m.get("runner").and_then(Value::as_str).is_some(),
                    pick: s(m, "pick"),
                    fits_note: s(m, "fitsNote"),
                    gated: s(m, "gated"),
                    no_source: s(m, "noSource"),
                    page: s(m, "page"),
                    vram_all: m.get("vramAll").and_then(Value::as_f64).unwrap_or(0.0),
                })
                .collect()
        })
        .unwrap_or_default();
    Catalog { groups, models, tripo_costs: v.pointer("/tripo/costs").cloned().unwrap_or(Value::Null) }
}

/// Where the app keeps its own copy of the helper (and models.json beside it): ~/.local/share/hearth/forge.
pub fn data() -> PathBuf {
    hearth_oss::data_dir("forge")
}

/// ~/Hearth/Forge: every model made, with its photo, words and settings ($HEARTH_FORGE_DIR in the tests).
pub fn forge_dir() -> PathBuf {
    std::env::var_os("HEARTH_FORGE_DIR").map(PathBuf::from).unwrap_or_else(|| hearth_ui::paths::home().join("Hearth/Forge"))
}

/// hearth-forge.py and models.json written next to each other (only when they differ, atomically): the service runs
/// this copy, so the Store's newer app brings its newer helper.
pub fn helper() -> PathBuf {
    let d = data();
    let _ = std::fs::create_dir_all(&d);
    for (name, text) in [("hearth-forge.py", HELPER), ("models.json", MODELS_JSON)] {
        let p = d.join(name);
        if std::fs::read_to_string(&p).ok().as_deref() != Some(text) {
            let tmp = d.join(format!(".{name}.tmp"));
            if std::fs::write(&tmp, text).is_ok() {
                let _ = std::fs::rename(&tmp, &p);
            }
        }
    }
    d.join("hearth-forge.py")
}

pub fn helper_argv(args: &[&str]) -> Vec<String> {
    let mut v = vec!["python3".to_string(), helper().to_string_lossy().to_string()];
    v.extend(args.iter().map(|s| s.to_string()));
    v
}

/// Work off the window's thread.
pub fn bg<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> impl std::future::Future<Output = Option<T>> + Send + 'static {
    let (tx, rx) = futures::channel::oneshot::channel();
    let _ = std::thread::Builder::new().name("hearth-forge".into()).spawn(move || {
        let _ = tx.send(f());
    });
    async move { rx.await.ok() }
}

pub fn get_json(base: &str, path: &str) -> Result<Value, String> {
    let r = http::send(&Req::get(&format!("{base}{path}")).timeout(10))?;
    if !r.ok() {
        return Err(r.json().and_then(|j| j.get("error").and_then(Value::as_str).map(str::to_string)).unwrap_or_else(|| format!("it said {}", r.status)));
    }
    r.json().ok_or_else(|| "not JSON".into())
}

pub fn post_json(base: &str, path: &str, body: &Value) -> Result<Value, String> {
    let r = http::send(&Req::new("POST", &format!("{base}{path}")).json(body).timeout(60))?;
    let j = r.json().unwrap_or(Value::Null);
    if !r.ok() {
        return Err(j.get("error").and_then(Value::as_str).map(str::to_string).unwrap_or_else(|| format!("it said {}", r.status)));
    }
    Ok(j)
}

/// A file of a made model: from this device's ~/Hearth/Forge when it's here, else from the service.
pub fn made_file(base: &str, id: &str, name: &str) -> Option<Vec<u8>> {
    let local = forge_dir().join(id).join(name);
    if let Ok(b) = std::fs::read(&local) {
        return Some(b);
    }
    let r = http::send(&Req::get(&format!("{base}/made/{}/file/{}", http::enc_path(id), http::enc_path(name))).timeout(60)).ok()?;
    r.ok().then_some(r.body)
}

/// The models made on this device itself (Tripo3D's, or the Nvidia PC's own when the app runs there).
pub fn local_made() -> Vec<Value> {
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(forge_dir()) else { return out };
    for e in rd.flatten() {
        let d = e.path();
        let name = e.file_name().to_string_lossy().to_string();
        if name == "models" || name == "exports" || name.starts_with('.') {
            continue;
        }
        let read = |f: &str| std::fs::read(d.join(f)).ok().and_then(|b| serde_json::from_slice::<Value>(&b).ok()).unwrap_or(Value::Null);
        let ask = read("ask.json");
        if ask.is_null() {
            continue;
        }
        let done = read("done.json");
        let files: Vec<Value> = std::fs::read_dir(&d).map(|r| r.flatten().map(|f| json!(f.file_name().to_string_lossy())).collect()).unwrap_or_default();
        let at = done.get("at").and_then(Value::as_i64).or_else(|| ask.get("at").and_then(Value::as_i64)).unwrap_or(0);
        out.push(json!({"id": name, "title": ask.get("title").cloned().unwrap_or(json!(name)), "ask": ask, "stats": read("stats.json"), "done": done, "files": files, "at": at, "local": true}));
    }
    out
}

/// A made model's folder on this device, its model and words copied from the service when it was made elsewhere.
pub fn ensure_local(base: &str, id: &str) -> Result<PathBuf, String> {
    let d = forge_dir().join(id);
    std::fs::create_dir_all(&d).map_err(|e| e.to_string())?;
    for f in ["ask.json", "stats.json", "done.json", "model.glb", "photo.png"] {
        if !d.join(f).is_file() {
            if let Some(b) = made_file(base, id, f) {
                let tmp = d.join(format!(".{f}.tmp"));
                std::fs::write(&tmp, &b).map_err(|e| e.to_string())?;
                std::fs::rename(&tmp, d.join(f)).map_err(|e| e.to_string())?;
            }
        }
    }
    if !d.join("model.glb").is_file() {
        return Err("its model isn't made yet".into());
    }
    Ok(d)
}

/// A number with a space between thousands (48 210, 71 204), as the board writes them.
pub fn spaced(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(' ');
        }
        out.push(c);
    }
    out
}
/// A number with commas (2,275 credits), as the board writes credits.
pub fn commas(n: u64) -> String {
    spaced(n).replace(' ', ",")
}

/// "30 cm tall" → metres (0.3); a plain number is centimetres.
pub fn real_size_m(s: &str) -> f64 {
    let low = s.to_lowercase();
    let num: String = low.chars().take_while(|c| c.is_ascii_digit() || *c == '.' || *c == ',' || *c == ' ').filter(|c| *c != ' ').collect::<String>().replace(',', ".");
    let v: f64 = num.parse().unwrap_or(0.0);
    let unit = low.trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == ',' || c == ' ');
    if unit.starts_with("mm") {
        v / 1000.0
    } else if unit.starts_with("m ") || unit == "m" || unit.starts_with("m tall") || unit.starts_with("metre") || unit.starts_with("meter") {
        v
    } else if unit.starts_with("in") {
        v * 0.0254
    } else {
        v / 100.0
    }
}

/// m:ss (0:48, 2:24).
pub fn mmss(secs: u64) -> String {
    format!("{}:{:02}", secs / 60, secs % 60)
}
/// "3 min 12 s".
pub fn min_s(secs: u64) -> String {
    if secs >= 60 {
        format!("{} min {} s", secs / 60, secs % 60)
    } else {
        format!("{secs} s")
    }
}

/// The project a game's folder is: the first project.godot (or O3DE project.json) under the home folder, three
/// folders down at most, whose own name has this word in it.
pub fn find_project(file: &str, word: &str) -> Option<PathBuf> {
    let word = word.to_lowercase();
    let mut stack = vec![(hearth_ui::paths::home(), 0)];
    let mut seen = 0;
    while let Some((d, depth)) = stack.pop() {
        seen += 1;
        if seen > 4000 {
            break;
        }
        let f = d.join(file);
        if f.is_file() && std::fs::read_to_string(&f).map(|t| t.to_lowercase().contains(&word)).unwrap_or(false) {
            return Some(d);
        }
        if depth >= 3 {
            continue;
        }
        if let Ok(rd) = std::fs::read_dir(&d) {
            for e in rd.flatten() {
                let n = e.file_name().to_string_lossy().to_string();
                if n.starts_with('.') || n == "node_modules" || n == "target" || n == "models" {
                    continue;
                }
                if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    stack.push((e.path(), depth + 1));
                }
            }
        }
    }
    None
}

pub fn copy_into(src: &Path, dir: &Path, name: &str) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let to = dir.join(name);
    std::fs::copy(src, &to).map_err(|e| e.to_string())?;
    Ok(to)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn the_list_is_the_boards() {
        let c = catalog();
        assert_eq!(c.models.len(), 22, "All 22 models");
        assert_eq!(c.groups.len(), 4);
        assert!(c.models.iter().any(|m| m.default && m.id == "hunyuan3d-2"));
        assert_eq!(c.models.iter().filter(|m| m.best > 0).count(), 4, "Get the best 4 for my PC");
        let v: Value = serde_json::from_str(MODELS_JSON).unwrap();
        for m in v["models"].as_array().unwrap() {
            if let Some(r) = m.get("runner").and_then(Value::as_str) {
                assert!(v["runners"].get(r).is_some(), "{r}");
                assert_eq!(v["runners"][r]["commit"].as_str().unwrap().len(), 40);
            }
        }
    }
    #[test]
    fn numbers_as_the_board_writes_them() {
        assert_eq!(spaced(48210), "48 210");
        assert_eq!(spaced(71204), "71 204");
        assert_eq!(commas(2275), "2,275");
        assert_eq!(mmss(48), "0:48");
        assert_eq!(mmss(144), "2:24");
        assert_eq!(min_s(192), "3 min 12 s");
        assert!((real_size_m("30 cm tall") - 0.3).abs() < 1e-9);
        assert!((real_size_m("120 mm") - 0.12).abs() < 1e-9);
    }
}
