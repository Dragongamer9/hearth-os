//! Backup and restore (Settings › System): settings, profiles, favorites, collections, play time and your own
//! games in one file on a USB drive or in your Backups folder; sign-ins only if asked. Same file format as the
//! Electron host (data/ and config/ in a .tar.gz with hearth-backup.json). Port of backup.js.

use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub const SKIP: [&str; 8] = ["cache", "tools", "prefixes", "art-cache", "nimbus-dev", "session", "xcloud-catalog.json", "before-restore"];
pub const SIGNINS: [&str; 4] = ["accounts", "legendary", "gog", "remote-play.json"];

fn is_backup_name(n: &str) -> bool { n.starts_with("hearth-backup-") && n.ends_with(".tar.gz") && n[14..n.len() - 7].chars().all(|c| c.is_ascii_alphanumeric() || "._-".contains(c)) }

pub fn destinations(home: &Path) -> Vec<Value> {
    let user = std::env::var("USER").unwrap_or_default();
    let mut out = Vec::new();
    for root in [format!("/run/media/{user}"), format!("/media/{user}")] {
        if let Ok(rd) = std::fs::read_dir(&root) {
            for e in rd.flatten() { if e.path().is_dir() { out.push(json!({"path": e.path(), "label": e.file_name().to_string_lossy(), "removable": true})); } }
        }
    }
    out.push(json!({"path": home.join("Backups"), "label": "Backups folder (on this device)", "removable": false}));
    out
}

pub fn list(dirs: &[PathBuf]) -> Vec<Value> {
    let mut out = Vec::new();
    for d in dirs { for dir in [d.clone(), d.join("Hearth Backups")] {
        let Ok(rd) = std::fs::read_dir(&dir) else { continue };
        for e in rd.flatten() {
            let n = e.file_name().to_string_lossy().to_string();
            if !is_backup_name(&n) { continue; }
            if let Ok(m) = e.metadata() {
                let at = m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_millis() as u64).unwrap_or(0);
                out.push(json!({"file": e.path(), "name": n, "bytes": m.len(), "at": at}));
            }
        }
    } }
    out.sort_by(|a, b| b["at"].as_u64().cmp(&a["at"].as_u64()));
    out
}

fn excludes(signins: bool) -> Vec<String> {
    let mut ex: Vec<String> = vec!["--no-anchored".into(), "--exclude=*.log".into(), "--exclude=*.tmp".into(), "--exclude=*.part".into(), "--anchored".into()];
    for n in SKIP { ex.push(format!("--exclude=./{n}")); }
    ex.push("--exclude=./profiles/*/cache".into());
    if !signins { for n in SIGNINS { ex.push(format!("--exclude=./{n}")); ex.push(format!("--exclude=./profiles/*/{n}")); } }
    ex
}

fn tar(args: &[String]) -> Result<(), String> {
    let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    crate::system::run_timeout("tar", &refs, std::time::Duration::from_secs(600)).map(|_| ())
}

pub fn create(dest: &Path, signins: bool, data_dir: &Path, conf_dir: &Path, version: &str, stamp: &str) -> Result<Value, String> {
    let removable = dest.starts_with("/run/media") || dest.starts_with("/media");
    let dir = if removable { dest.join("Hearth Backups") } else { dest.to_path_buf() };
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let file = dir.join(format!("hearth-backup-{stamp}.tar.gz"));
    let stage = std::env::temp_dir().join(format!("hearth-backup-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&stage);
    std::fs::create_dir_all(&stage).map_err(|e| e.to_string())?;
    let result = (|| {
        std::fs::write(stage.join("hearth-backup.json"), json!({"made": stamp, "version": version, "signins": signins}).to_string()).map_err(|e| e.to_string())?;
        let t = stage.join("backup.tar").to_string_lossy().to_string();
        tar(&["-cf".into(), t.clone(), "-C".into(), stage.to_string_lossy().into(), "hearth-backup.json".into()])?;
        if data_dir.exists() {
            let mut a = vec!["-rf".to_string(), t.clone(), "-C".into(), data_dir.to_string_lossy().into()];
            a.extend(excludes(signins)); a.push("--transform=s,^\\.,data,".into()); a.push(".".into());
            tar(&a)?;
        }
        if conf_dir.exists() { tar(&["-rf".into(), t.clone(), "-C".into(), conf_dir.to_string_lossy().into(), "--transform=s,^\\.,config,".into(), ".".into()])?; }
        crate::system::run_timeout("gzip", &["-6", &t], std::time::Duration::from_secs(600))?;
        let part = file.with_extension("part");
        std::fs::copy(format!("{t}.gz"), &part).map_err(|e| e.to_string())?;
        std::fs::rename(&part, &file).map_err(|e| e.to_string())?;
        let _ = crate::system::run("sync", &[]);   // safe to unplug the USB drive
        Ok(json!({"file": file, "bytes": std::fs::metadata(&file).map(|m| m.len()).unwrap_or(0)}))
    })();
    let _ = std::fs::remove_dir_all(&stage);
    result
}

pub fn info(file: &Path) -> Result<Value, String> {
    let f = file.to_string_lossy();
    let names = crate::system::run_timeout("tar", &["-tzf", &f], std::time::Duration::from_secs(60))?;
    if !names.lines().any(|l| l == "hearth-backup.json") { return Err("That isn't a Hearth backup".into()); }
    let meta: Value = serde_json::from_str(&crate::system::run("tar", &["-xzOf", &f, "hearth-backup.json"])?).unwrap_or(json!({}));
    let mut m = meta; m["files"] = json!(names.lines().count());
    Ok(m)
}

/// Puts a backup back; what it replaces is kept in data/before-restore/<time>/ first. Hearth restarts after.
pub fn restore(file: &Path, data_dir: &Path, conf_dir: &Path, stamp: &str) -> Result<Value, String> {
    let meta = info(file)?;
    // Unpacked next to the data folder, not in /tmp: /tmp is a separate (memory) file system on most editions, where
    // moving the files in failed ("Invalid cross-device link") after the first file had already been put aside.
    let tmp = data_dir.with_file_name(format!(".hearth-restore-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    let r = (|| {
        tar(&["-xzf".into(), file.to_string_lossy().into(), "-C".into(), tmp.to_string_lossy().into(), "--no-same-owner".into()])?;
        let keep = data_dir.join("before-restore").join(stamp);
        let swap = |from: &Path, to: &Path, keep_dir: &Path| -> Result<(), String> {
            let Ok(rd) = std::fs::read_dir(from) else { return Ok(()) };
            std::fs::create_dir_all(to).map_err(|e| e.to_string())?;
            for e in rd.flatten() {
                let dst = to.join(e.file_name());
                if dst.exists() { std::fs::create_dir_all(keep_dir).map_err(|e| e.to_string())?; move_path(&dst, &keep_dir.join(e.file_name()))?; }
                move_path(&e.path(), &dst)?;
            }
            Ok(())
        };
        swap(&tmp.join("data"), data_dir, &keep)?;
        swap(&tmp.join("config"), conf_dir, &keep.join("config"))?;
        Ok(meta.clone())
    })();
    let _ = std::fs::remove_dir_all(&tmp);
    r
}

/// Moves a file or folder: a rename, or a copy then delete when it's on another file system (the config folder
/// can be elsewhere).
fn move_path(from: &Path, to: &Path) -> Result<(), String> {
    if std::fs::rename(from, to).is_ok() { return Ok(()); }
    crate::system::run_timeout("cp", &["-a", "--", &from.to_string_lossy(), &to.to_string_lossy()], std::time::Duration::from_secs(600))?;
    if from.is_dir() { std::fs::remove_dir_all(from) } else { std::fs::remove_file(from) }.map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn put(f: &Path, t: &str) { std::fs::create_dir_all(f.parent().unwrap()).unwrap(); std::fs::write(f, t).unwrap(); }
    #[test]
    fn backs_up_and_restores() {
        let root = tempfile::tempdir().unwrap();
        let (data, conf, dest) = (root.path().join("data"), root.path().join("conf"), root.path().join("Backups"));
        put(&data.join("library.json"), r#"{"favorites":["steam:1"]}"#);
        put(&data.join("profiles/p1/library.json"), "{}");
        put(&data.join("accounts/xbox.json"), "token");
        put(&data.join("cache/big.bin"), "xxxx");
        put(&data.join("bridge.log"), "log");
        put(&conf.join("session.conf"), "NIMBUS_START_MODE=game");
        let r = create(&dest, false, &data, &conf, "0.0.23", "2026-10-18-12-30").unwrap();
        let file = PathBuf::from(r["file"].as_str().unwrap());
        assert_eq!(list(&[dest.clone()]).len(), 1);
        let names = crate::system::run("tar", &["-tzf", &file.to_string_lossy()]).unwrap();
        assert!(names.lines().any(|l| l == "data/library.json") && names.lines().any(|l| l == "config/session.conf"));
        assert!(!names.contains("cache/") && !names.contains("bridge.log") && !names.contains("accounts"), "{names}");
        assert_eq!(info(&file).unwrap()["version"], "0.0.23");
        std::fs::write(data.join("library.json"), r#"{"favorites":[]}"#).unwrap();
        restore(&file, &data, &conf, "2026-10-19-08-00-00").unwrap();
        assert_eq!(std::fs::read_to_string(data.join("library.json")).unwrap(), r#"{"favorites":["steam:1"]}"#);
        assert_eq!(std::fs::read_to_string(data.join("accounts/xbox.json")).unwrap(), "token");
        assert!(data.join("before-restore/2026-10-19-08-00-00/library.json").exists());
    }
}
