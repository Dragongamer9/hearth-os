//! Decky plugins from your other devices (docs/HEARTH-LINK-PLUGIN.md, "Sync Decky plugins").
//!
//! A paired device asks for one of this device's Decky plugins by its folder name (`decky.plugin`), and gets its
//! files: every regular file under ~/homebrew/plugins/<folder> (no links, no hidden files at the top, at most
//! MAX_TOTAL bytes in all). Hearth's own plugins are on every device already and are never sent.
//!
//! Nothing is installed here: the Hearth Link Decky plugin checks every path again, packs the files as a zip and asks
//! Decky Loader to install it, which shows Decky's own "Install …?" prompt first (Decky runs plugins as root, so a
//! plugin from another device is only ever installed when you say yes on this one).

use crate::node::Node;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

/// The image's own plugins (hearth-decky's PLUGINS, and the older Hearth plugin): never sent.
pub const HEARTH_PLUGINS: [&str; 3] = ["decky-hearth-sync", "decky-hearth-link", "decky-hearth"];
/// At most this much in all (the message carries it as base64, under wire::MAX_MESSAGE).
pub const MAX_TOTAL: u64 = 40 << 20;
pub const MAX_FILES: usize = 4000;

pub fn plugins_dir(home: &Path) -> PathBuf { home.join("homebrew").join("plugins") }

/// A plugin's folder name: one plain part, not hidden, not one of Hearth's own.
pub fn valid_folder(s: &str) -> bool {
    crate::sync::valid_part(s) && s.len() <= 100 && !s.starts_with('.') && !HEARTH_PLUGINS.contains(&s)
}

/// A file's path inside a plugin, as sent: relative, plain parts, nothing that climbs out.
pub fn valid_rel(p: &str) -> bool {
    !p.is_empty() && p.len() <= 400 && !p.starts_with('/') && p.split('/').all(|c| crate::sync::valid_part(c))
}

/// The plugin's files, ready to send: {"folder", "name", "version", "files": [{"p", "x", "b64"}]}.
pub fn pack(home: &Path, folder: &str) -> Result<Value, String> {
    if !valid_folder(folder) { return Err("That isn't a Decky plugin this device sends".into()); }
    let root = plugins_dir(home).join(folder);
    let md = std::fs::symlink_metadata(&root).map_err(|_| "That plugin isn't on this device".to_string())?;
    if !md.is_dir() { return Err("That plugin isn't on this device".into()); }
    let meta: Value = std::fs::read(root.join("plugin.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).ok_or("That plugin has no plugin.json")?;
    let version = std::fs::read(root.join("package.json")).ok().and_then(|b| serde_json::from_slice::<Value>(&b).ok())
        .and_then(|v| v["version"].as_str().map(str::to_string)).unwrap_or_default();
    let mut files = Vec::new();
    let mut total = 0u64;
    let mut stack = vec![(root.clone(), String::new())];
    while let Some((dir, rel)) = stack.pop() {
        let mut entries: Vec<_> = std::fs::read_dir(&dir).map_err(|e| e.to_string())?.flatten().collect();
        entries.sort_by_key(|e| e.file_name());
        for e in entries {
            let name = e.file_name().to_string_lossy().to_string();
            if !crate::sync::valid_part(&name) || (rel.is_empty() && name.starts_with('.')) { continue; }
            let r = if rel.is_empty() { name.clone() } else { format!("{rel}/{name}") };
            let Ok(m) = std::fs::symlink_metadata(e.path()) else { continue };
            if m.file_type().is_symlink() { continue; }
            if m.is_dir() { stack.push((e.path(), r)); continue; }
            if !m.is_file() { continue; }
            total += m.len();
            if total > MAX_TOTAL { return Err(format!("That plugin is too big to copy this way (over {})", crate::human_bytes(MAX_TOTAL))); }
            if files.len() >= MAX_FILES { return Err("That plugin has too many files to copy this way".into()); }
            use std::os::unix::fs::PermissionsExt;
            let b = std::fs::read(e.path()).map_err(|e| format!("{r}: {e}"))?;
            files.push(json!({"p": r, "x": m.permissions().mode() & 0o111 != 0, "b64": crate::sync::b64(&b)}));
        }
    }
    Ok(json!({"folder": folder, "name": meta["name"].as_str().unwrap_or(folder), "version": version, "files": files}))
}

/// Checks what came from the other device: the folder asked for, and only plain relative paths.
pub fn check(folder: &str, v: &Value) -> Result<Value, String> {
    if v["folder"].as_str() != Some(folder) { return Err("The other device sent a different plugin".into()); }
    let files = v["files"].as_array().ok_or("The other device sent no files")?;
    if files.is_empty() || files.len() > MAX_FILES { return Err("The other device sent no files".into()); }
    if !files.iter().any(|f| f["p"] == "plugin.json") { return Err("That isn't a Decky plugin (no plugin.json)".into()); }
    for f in files {
        if !f["p"].as_str().map(valid_rel).unwrap_or(false) || f["b64"].as_str().is_none() { return Err("The other device sent a path that isn't allowed".into()); }
    }
    Ok(v.clone())
}

impl Node {
    /// Another device asks for one of this device's plugins.
    pub async fn decky_answer(self: Arc<Self>, _peer: &str, op: &str, req: &Value) -> Result<Value, String> {
        match op {
            "decky.plugin" => {
                let (home, folder) = (self.home.clone(), req["folder"].as_str().unwrap_or("").to_string());
                tokio::task::spawn_blocking(move || pack(&home, &folder)).await.map_err(|e| e.to_string())?
            }
            _ => Err(format!("Unknown request: {op}")),
        }
    }

    /// The UI's decky.* commands: decky.fetch {peer, folder} gets a plugin's files from a paired device (checked).
    pub async fn decky_command(self: &Arc<Self>, cmd: &str, a: &Value) -> Result<Value, String> {
        match cmd {
            "decky.fetch" => {
                let peer = a["peer"].as_str().unwrap_or("").to_string();
                let folder = a["folder"].as_str().unwrap_or("").to_string();
                if !valid_folder(&folder) { return Err("That isn't a Decky plugin Hearth Link copies".into()); }
                if !crate::lock(&self.peers).is_mine(&peer) { return Err("Not a paired device".into()); }
                let v = match tokio::time::timeout(Duration::from_secs(110), self.request(&peer, &json!({"op": "decky.plugin", "folder": folder}))).await {
                    Ok(Ok(v)) => v, Ok(Err(e)) => return Err(e), Err(_) => return Err("The other device took too long to answer".into()),
                };
                check(&folder, &v)
            }
            _ => Err(format!("Unknown command: {cmd}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("hl-decky-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn packs_a_plugin_without_links_or_hidden_files() {
        let home = tmp("pack");
        let p = plugins_dir(&home).join("CSS Loader");
        std::fs::create_dir_all(p.join("dist")).unwrap();
        std::fs::write(p.join("plugin.json"), r#"{"name": "CSS Loader"}"#).unwrap();
        std::fs::write(p.join("package.json"), r#"{"version": "2.1.0"}"#).unwrap();
        std::fs::write(p.join("dist/index.js"), "x").unwrap();
        std::fs::write(p.join(".git"), "x").unwrap();
        std::os::unix::fs::symlink("/etc/passwd", p.join("dist/passwd")).unwrap();
        let v = pack(&home, "CSS Loader").unwrap();
        assert_eq!(v["name"], "CSS Loader");
        assert_eq!(v["version"], "2.1.0");
        let mut names: Vec<&str> = v["files"].as_array().unwrap().iter().map(|f| f["p"].as_str().unwrap()).collect();
        names.sort();
        assert_eq!(names, ["dist/index.js", "package.json", "plugin.json"]);
        assert!(check("CSS Loader", &v).is_ok());
    }

    #[test]
    fn never_sends_hearths_own_or_paths_that_climb_out() {
        let home = tmp("own");
        std::fs::create_dir_all(plugins_dir(&home).join("decky-hearth-link")).unwrap();
        assert!(pack(&home, "decky-hearth-link").is_err());
        assert!(pack(&home, "../etc").is_err());
        assert!(pack(&home, "missing").is_err());
        for bad in ["../x", "/etc/passwd", "a/../../b", "a//b", ""] { assert!(!valid_rel(bad), "{bad}"); }
        let v = json!({"folder": "p", "files": [{"p": "plugin.json", "b64": ""}, {"p": "../../.bashrc", "b64": ""}]});
        assert!(check("p", &v).is_err());
        assert!(check("q", &json!({"folder": "p", "files": [{"p": "plugin.json", "b64": ""}]})).is_err());
    }
}
