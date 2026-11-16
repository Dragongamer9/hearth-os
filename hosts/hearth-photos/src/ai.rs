//! What's in a photo and whose face is in it (1.14): ai/hearth_photos_ai.py does the looking, on this PC, in the
//! background; this starts it, gives it the library's pictures (a small copy for kinds OpenCV can't open, like HEIC)
//! and reads its results for the page. Faces and "Found in it" can be turned off (photos.json: faces / tags false);
//! turning faces off forgets every group.

use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::{Command, Stdio};

const HELPER: &str = include_str!("../ai/hearth_photos_ai.py");

fn base() -> PathBuf {
    std::env::var_os("HEARTH_PHOTOS_AI").map(PathBuf::from)
        .unwrap_or_else(|| dirs::data_dir().unwrap_or_else(|| PathBuf::from("/tmp")).join("hearth").join("photos-ai"))
}

/// The helper, written next to its results when it's new or changed.
fn helper() -> Result<PathBuf, String> {
    let p = base().join("hearth_photos_ai.py");
    if std::fs::read_to_string(&p).ok().as_deref() != Some(HELPER) {
        std::fs::create_dir_all(base()).map_err(|e| e.to_string())?;
        crate::write_atomic(&p, HELPER.as_bytes())?;
    }
    Ok(p)
}

fn python() -> &'static str { if crate::system::which("python3") { "python3" } else { "python" } }

fn call(args: &[&str]) -> Result<Value, String> {
    let out = Command::new(python()).arg(helper()?).args(args).stdin(Stdio::null()).output().map_err(|e| format!("Python isn't here: {e}"))?;
    let text = String::from_utf8_lossy(&out.stdout);
    serde_json::from_str(text.lines().last().unwrap_or("")).map_err(|_| String::from_utf8_lossy(&out.stderr).trim().chars().take(400).collect())
}

pub fn status() -> Value { call(&["status"]).unwrap_or_else(|e| json!({"ready": false, "error": e})) }

fn running(kind: &str) -> bool {
    let f = base().join(format!("{kind}.pid"));
    std::fs::read_to_string(&f).ok().and_then(|s| s.trim().parse::<u32>().ok())
        .map(|pid| std::path::Path::new(&format!("/proc/{pid}")).exists()).unwrap_or(false)
}

fn spawn(kind: &str, args: &[String]) -> Result<(), String> {
    if running(kind) { return Ok(()); }
    let log = std::fs::File::create(base().join(format!("{kind}.log"))).map_err(|e| e.to_string())?;
    let child = Command::new("nice").args(["-n", "15", python()]).arg(helper()?).args(args)
        .stdin(Stdio::null()).stdout(log.try_clone().map_err(|e| e.to_string())?).stderr(log)
        .spawn().map_err(|e| format!("Couldn't start: {e}"))?;
    let _ = std::fs::write(base().join(format!("{kind}.pid")), child.id().to_string());
    Ok(())
}

/// One-time setup (OpenCV and the small models), in the background.
pub fn setup() -> Result<(), String> { spawn("setup", &["setup".into()]) }

/// Looks at the library's pictures it hasn't seen (in the background, at low priority).
pub fn run() -> Result<Value, String> {
    let st = status();
    if !st["ready"].as_bool().unwrap_or(false) { return Ok(json!({"started": false, "status": st})); }
    let thumbs = crate::imaging::cache_dir("ai");
    let _ = std::fs::create_dir_all(&thumbs);
    let opencv = ["jpg", "jpeg", "jpe", "jfif", "png", "webp", "bmp", "tif", "tiff"];
    let list: Vec<Value> = crate::library::cached().into_iter().filter(|i| i.kind != "video").map(|i| {
        let p = std::path::Path::new(&i.path);
        let thumb = (!opencv.contains(&crate::formats::ext(p).as_str())).then(|| {
            let out = thumbs.join(format!("{:x}.jpg", fxhash(&i.path, i.mtime)));
            if !out.exists() { if let Ok((b, _)) = crate::imaging::thumbnail(p, 1024) { let _ = crate::write_atomic(&out, &b); } }
            out.to_string_lossy().into_owned()
        });
        json!({"path": i.path, "mtime": i.mtime, "thumb": thumb})
    }).collect();
    let file = base().join("todo.json");
    crate::write_atomic(&file, serde_json::to_string(&list).unwrap_or_default().as_bytes())?;
    spawn("run", &["run".into(), file.to_string_lossy().into_owned()])?;
    Ok(json!({"started": true, "count": list.len()}))
}

fn fxhash(s: &str, n: u64) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325 ^ n;
    for b in s.bytes() { h = (h ^ b as u64).wrapping_mul(0x0100_0000_01b3); }
    h
}

/// What it found: {photos: {path: {faces: [{g, box}], tags, by}}, groups: [{id, n, cover}], progress}.
pub fn results() -> Value {
    let mut v: Value = std::fs::read(base().join("results.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_else(|| json!({}));
    if let Some(gs) = v.get_mut("groups").and_then(Value::as_array_mut) {
        for g in gs { if let Some(o) = g.as_object_mut() { o.remove("c"); } }
    }
    v["progress"] = std::fs::read(base().join("progress.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null);
    v["running"] = json!(running("run") || running("setup"));
    v
}

/// Faces off: every face and group is forgotten (names too: the page clears them in photos.json). On: looked for again.
pub fn faces(on: bool) -> Result<Value, String> { call(&[if on { "faces-on" } else { "forget-faces" }]) }

#[cfg(test)]
mod tests {
    #[test]
    fn helper_is_written() {
        let d = std::env::temp_dir().join(format!("hp-ai-{}", std::process::id()));
        std::env::set_var("HEARTH_PHOTOS_AI", &d);
        let p = super::helper().unwrap();
        assert!(std::fs::read_to_string(&p).unwrap().contains("FaceDetectorYN"));
        let s = super::status();
        assert_eq!(s["ready"], false, "{s}");
        assert_eq!(super::results()["running"], false);
        let _ = std::fs::remove_dir_all(&d);
    }
}
