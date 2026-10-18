//! Updates to the Hearth app itself, from GitHub Releases (Settings › Updates). Port of
//! hosts/linux/src/updates.js. The Rust app is one file (Hearth-linux-x86_64, with a .sha256 next to it):
//! Hearth downloads the new one beside itself, checks it, swaps the file (the old one kept as .old) and
//! restarts. On Hearth OS the app is part of the system and updates with it (System updates).

use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

const REPO: &str = "Dragongamer9/hearth-os";
pub const ASSET: &str = "Hearth-linux-x86_64";

/// 0.4.0 (a release) or 0.4.0-linux.13 (preview 13 on the way to the next one): (numbers, preview)
pub fn parse_version(v: &str) -> (Vec<u64>, Option<u64>) {
    let v = v.trim_start_matches('v');
    let (base, rest) = match v.find(|c: char| !(c.is_ascii_digit() || c == '.')) { Some(i) => (&v[..i], &v[i..]), None => (v, "") };
    let parts: Vec<u64> = base.split('.').filter(|s| !s.is_empty()).map(|s| s.parse().unwrap_or(0)).collect();
    let preview = rest.strip_prefix("-linux.").and_then(|p| p.split(|c: char| !c.is_ascii_digit()).next()).and_then(|p| p.parse().ok());
    (if parts.is_empty() { vec![0] } else { parts }, preview)
}
/// Is a newer than b? A preview of a version counts as newer than that release (it's built on top of it).
pub fn newer(a: &str, b: &str) -> bool {
    let ((pa, xa), (pb, xb)) = (parse_version(a), parse_version(b));
    for i in 0..pa.len().max(pb.len()) {
        let (x, y) = (pa.get(i).copied().unwrap_or(0), pb.get(i).copied().unwrap_or(0));
        if x != y { return x > y; }
    }
    match (xa, xb) { (Some(x), Some(y)) => x > y, (Some(_), None) => true, _ => false }
}
/// Releases from before Hearth was renumbered to 0.0.x look newer than they are: never offered.
fn legacy(tag: &str) -> bool { let t = tag.trim_start_matches('v'); t.starts_with("0.3.") || t.starts_with("0.4.") || t.starts_with("0.5.") }

pub struct AppUpdates { pub state: Arc<Mutex<Value>>, current: String }

impl AppUpdates {
    pub fn new(version: &str) -> AppUpdates {
        AppUpdates { current: version.into(), state: Arc::new(Mutex::new(json!({"state": "idle", "latest": null, "progress": null, "error": null, "lastChecked": 0, "ready": null}))) }
    }
    /// The app's own file, when Hearth may replace it (not one that's part of the system).
    pub fn exe() -> Option<PathBuf> {
        let e = std::env::current_exe().ok()?;
        if e.starts_with("/usr") || e.starts_with("/opt") || e.to_string_lossy().contains("/target/") { return None; }
        let dir = e.parent()?;
        let probe = dir.join(".hearth-write-test");
        std::fs::write(&probe, b"").ok()?;
        let _ = std::fs::remove_file(probe);
        Some(e)
    }
    pub fn status(&self, auto: bool) -> Value {
        let mut s = self.state.lock().unwrap().clone();
        let exe = Self::exe();
        s["current"] = json!(self.current);
        s["canInstallHere"] = json!(exe.is_some());
        s["installFolder"] = json!(exe.map(|e| e.to_string_lossy().to_string()).unwrap_or_else(|| "the Hearth app".into()));
        s["autoCheck"] = json!(auto);
        s.as_object_mut().unwrap().remove("ready");
        s
    }
    fn set(state: &Arc<Mutex<Value>>, patch: Value, current: &str) {
        let mut s = state.lock().unwrap();
        for (k, v) in patch.as_object().cloned().unwrap_or_default() { s[k] = v; }
        let mut out = s.clone();
        out["current"] = json!(current); out["canInstallHere"] = json!(Self::exe().is_some());
        out.as_object_mut().unwrap().remove("ready");
        crate::stores::notify(json!({"type": "event", "name": "update", "data": out}));
    }

    /// Looks for a newer release (blocking; a few seconds at most).
    pub fn check(&self, auto: bool) -> Value {
        let st = self.state.lock().unwrap()["state"].clone();
        if st == "downloading" || st == "ready" { return self.status(auto); }
        Self::set(&self.state, json!({"state": "checking", "error": null}), &self.current);
        let r = crate::http::get_json(&format!("https://api.github.com/repos/{REPO}/releases?per_page=30"), &[("Accept", "application/vnd.github+json")]);
        let now = crate::library::now_ms();
        match r {
            Err(e) => Self::set(&self.state, json!({"state": "error", "error": format!("Couldn't check for updates: {e}"), "lastChecked": now}), &self.current),
            Ok(list) => {
                let preview_ok = parse_version(&self.current).1.is_some();
                let best = list.as_array().cloned().unwrap_or_default().into_iter()
                    .filter(|x| x["draft"] != true && !legacy(x["tag_name"].as_str().unwrap_or("")) && (preview_ok || x["prerelease"] != true))
                    .filter(|x| x["assets"].as_array().map(|a| a.iter().any(|a| a["name"] == ASSET)).unwrap_or(false))
                    .fold(None::<Value>, |best, x| match &best { Some(b) if !newer(x["tag_name"].as_str().unwrap_or(""), b["tag_name"].as_str().unwrap_or("")) => best, _ => Some(x) });
                match best {
                    None => Self::set(&self.state, json!({"state": "upToDate", "latest": null, "lastChecked": now}), &self.current),
                    Some(rel) => {
                        let version = rel["tag_name"].as_str().unwrap_or("").trim_start_matches('v').to_string();
                        let assets = rel["assets"].as_array().cloned().unwrap_or_default();
                        let asset = assets.iter().find(|a| a["name"] == ASSET).cloned().unwrap_or(Value::Null);
                        let sha = assets.iter().find(|a| a["name"] == format!("{ASSET}.sha256").as_str()).and_then(|a| a["browser_download_url"].as_str().map(str::to_string));
                        let latest = json!({"version": version, "tag": rel["tag_name"], "notes": rel["body"], "zipUrl": asset["browser_download_url"], "shaUrl": sha,
                                            "size": asset["size"], "published": rel["published_at"]});
                        let state = if newer(&version, &self.current) { "available" } else { "upToDate" };
                        Self::set(&self.state, json!({"state": state, "latest": latest, "lastChecked": now}), &self.current);
                    }
                }
            }
        }
        self.status(auto)
    }

    /// Downloads the update beside the app, in the background, and checks it.
    pub fn download(&self, auto: bool) -> Value {
        let (st, latest) = { let s = self.state.lock().unwrap(); (s["state"].clone(), s["latest"].clone()) };
        let Some(exe) = Self::exe() else { return self.status(auto) };
        if st != "available" || latest.is_null() { return self.status(auto); }
        Self::set(&self.state, json!({"state": "downloading", "progress": 0}), &self.current);
        let (state, current) = (self.state.clone(), self.current.clone());
        std::thread::spawn(move || {
            let target = PathBuf::from(format!("{}.new", exe.to_string_lossy()));
            let url = latest["zipUrl"].as_str().unwrap_or("").to_string();
            let total = latest["size"].as_u64().unwrap_or(0);
            let mut child = match std::process::Command::new("curl").args(["-fsSL", "--retry", "3", "-o"]).arg(&target).arg(&url).spawn() {
                Ok(c) => c, Err(e) => { Self::set(&state, json!({"state": "error", "error": format!("Update failed: {e}"), "progress": null}), &current); return; }
            };
            let ok = loop {
                if let Ok(Some(s)) = child.try_wait() { break s.success(); }
                std::thread::sleep(std::time::Duration::from_millis(500));
                if total > 0 { let done = std::fs::metadata(&target).map(|m| m.len()).unwrap_or(0); Self::set(&state, json!({"progress": (done as f64 / total as f64).min(1.0)}), &current); }
            };
            let fail = |msg: String| { let _ = std::fs::remove_file(&target); Self::set(&state, json!({"state": "error", "error": format!("Update failed: {msg}"), "progress": null}), &current); };
            if !ok { return fail("the download didn't finish".into()); }
            if let Some(sha_url) = latest["shaUrl"].as_str() {
                let want = crate::http::get(sha_url, &[]).map(|r| r.text.split_whitespace().next().unwrap_or("").to_lowercase()).unwrap_or_default();
                let got = crate::system::run("sha256sum", &[&target.to_string_lossy()]).map(|o| o.split_whitespace().next().unwrap_or("").to_string()).unwrap_or_default();
                if !want.is_empty() && want != got { return fail("the download is damaged (checksum mismatch)".into()); }
            }
            let _ = crate::system::run("chmod", &["755", &target.to_string_lossy()]);
            Self::set(&state, json!({"state": "ready", "progress": 1, "ready": target.to_string_lossy()}), &current);
        });
        self.status(auto)
    }

    /// Swaps the app file (keeping the old one as .old). The caller restarts Hearth afterwards.
    pub fn install(&self) -> Result<(), String> {
        let ready = self.state.lock().unwrap()["ready"].as_str().map(PathBuf::from).ok_or("No update is ready")?;
        let exe = Self::exe().ok_or("No update is ready")?;
        std::fs::copy(&exe, format!("{}.old", exe.to_string_lossy())).map_err(|e| e.to_string())?;
        std::fs::rename(&ready, &exe).map_err(|e| e.to_string())?;
        crate::bridge::log(&format!("update: installed {}", self.state.lock().unwrap()["latest"]["version"]));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn versions() {
        assert!(newer("0.0.24", "0.0.23"));
        assert!(!newer("0.0.23", "0.0.23"));
        assert!(newer("0.0.23-linux.2", "0.0.23"));
        assert!(!newer("0.0.23", "0.0.23-linux.2"));
        assert!(newer("v0.1.0", "0.0.99"));
        assert!(legacy("v0.4.0-linux.100"));
    }
}
