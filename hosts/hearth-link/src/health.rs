//! Device health (Hearth Storage): every paired Hearth device at a glance on Settings › Hearth Storage › Servers &
//! devices: online or when it was last seen, battery, free space, last backup, its Hearth version (and whether it's
//! the newest of your devices), and whether its last start went well (hearth-boot-guard's state, only read).
//! Each device reports this to its Hearth Storage server over Hearth Link every few minutes (device.report); any
//! device asks the server for the list (devices.list).
//!
//! Removing a device you don't use anymore: from any device or the server, Hearth Link's unpair, and on the server
//! (if asked) that device's backups and the games it archived are deleted too (devices.usage says how much that
//! frees first, devices.forget does it).

use crate::node::Node;
use serde_json::{json, Value};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

/// How often a device reports to its server.
pub const REPORT_EVERY_MS: u64 = 10 * 60_000;
/// A device that reported within this long counts as online.
const ONLINE_MS: u64 = 15 * 60_000;

/// The battery, if this device has one: {percent, charging}.
pub fn battery(sys: &Path) -> Value {
    for e in std::fs::read_dir(sys).into_iter().flatten().flatten() {
        let p = e.path();
        let read = |f: &str| std::fs::read_to_string(p.join(f)).map(|s| s.trim().to_string()).unwrap_or_default();
        // A controller's or mouse's battery is "Device" scope: not this device's.
        if read("type") != "Battery" || read("scope") == "Device" { continue; }
        let Ok(pct) = read("capacity").parse::<u64>() else { continue };
        let status = read("status");
        return json!({"percent": pct.min(100), "charging": status == "Charging" || status == "Full"});
    }
    Value::Null
}

/// Total bytes of the file system holding `path`.
pub fn total_space(path: &Path) -> Option<u64> {
    use std::os::unix::ffi::OsStrExt;
    let c = std::ffi::CString::new(path.as_os_str().as_bytes()).ok()?;
    let mut s: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statvfs(c.as_ptr(), &mut s) } != 0 { return None; }
    Some(s.f_blocks as u64 * s.f_frsize as u64)
}

/// How this device's last start went, from hearth-boot-guard's files (only read; nothing here touches booting):
/// {good: true (counted as good) | false (not yet) | null (no guard here), fellBack: when the previous version had
/// to start instead, recently}.
pub fn boot_state(boot: &Path, state: &Path) -> Value {
    let env = ["grub2/grubenv", "boot/grub2/grubenv"].iter().map(|f| boot.join(f)).find(|p| p.is_file()).and_then(|p| std::fs::read_to_string(p).ok());
    let tries = env.as_deref().and_then(|t| t.lines().find_map(|l| l.strip_prefix("hearth_tries=").map(|v| v.trim().to_string())));
    let good = match tries.as_deref() { Some("0") => json!(true), Some(_) => json!(false), None => Value::Null };
    let fb: Value = std::fs::read(state.join("boot-fallback.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null);
    json!({"good": good, "fellBack": fb["when"], "pausedUntil": fb["updates_paused_until"]})
}

/// "0.6.56" > "0.6.6"? Compared number by number.
pub fn newer(a: &str, b: &str) -> bool {
    let p = |s: &str| s.split(|c: char| !c.is_ascii_digit()).filter(|x| !x.is_empty()).map(|x| x.parse::<u64>().unwrap_or(0)).collect::<Vec<_>>();
    p(a) > p(b)
}

/// Hearth Benchmark's latest overall score for the health page: {overall, at, mode} (or null).
pub fn bench_brief(s: &Value) -> Value {
    if s["overall"].is_null() { return Value::Null; }
    json!({"overall": s["overall"], "at": s["at"], "mode": s["mode"]})
}

/// The rows of the health page: every device with its report, online, and whether it runs the newest Hearth.
pub fn rows(devices: Vec<(String, String, bool, u64, bool)>, reports: &std::collections::HashMap<String, Value>, now: u64) -> Vec<Value> {
    let newest = reports.values().filter_map(|r| r["version"].as_str()).fold(String::new(), |m, v| if newest_of(&m, v) { v.to_string() } else { m });
    devices.into_iter().map(|(id, name, me, last_seen, connected)| {
        let r = reports.get(&id).cloned().unwrap_or(Value::Null);
        let at = r["at"].as_u64().unwrap_or(0);
        let seen = last_seen.max(at);
        let version = r["version"].as_str().unwrap_or("");
        json!({"id": id, "name": name, "me": me, "online": me || connected || now.saturating_sub(at) < ONLINE_MS, "lastSeen": seen,
               "kind": r["kind"], "edition": r["edition"], "battery": r["battery"], "free": r["free"], "total": r["total"],
               "lastBackup": r["lastBackup"], "version": r["version"], "boot": r["boot"], "server": r["server"] == true, "reported": at,
               "bench": r["bench"], "upToDate": if version.is_empty() { Value::Null } else { json!(!newer(&newest, version)) }, "newest": newest})
    }).collect()
}
fn newest_of(cur: &str, v: &str) -> bool { cur.is_empty() || newer(v, cur) }

/// The size of a folder (files only, links not followed).
pub fn du(p: &Path) -> u64 {
    let mut files = Vec::new();
    if crate::manifest::walk(p, 0, &[], &mut files).is_err() { return 0; }
    files.iter().filter(|e| e.k == "f").map(|e| e.s).sum()
}

/// What the server keeps for a device: its backup folder(s) and the games it archived. [(path, bytes)].
pub fn device_data(backups: &Path, archive: &Path, id: &str) -> (Vec<(std::path::PathBuf, u64)>, Vec<(std::path::PathBuf, u64, String)>) {
    let dev: String = id.chars().filter(|c| c.is_ascii_alphanumeric()).take(8).collect();
    if dev.len() < 8 { return (vec![], vec![]); }
    let b = std::fs::read_dir(backups).into_iter().flatten().flatten()
        .filter(|e| { let n = e.file_name().to_string_lossy().to_string(); n == dev || n.ends_with(&format!("({dev})")) })
        .map(|e| { let p = e.path(); let s = du(&p); (p, s) }).collect();
    let a = std::fs::read_dir(archive).into_iter().flatten().flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with(&format!("{dev}-")))
        .map(|e| {
            let p = e.path();
            let g: Value = std::fs::read(p.join("game.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null);
            // An archive is also kept for the device it came from: only the ones that device sent.
            let title = g["game"]["title"].as_str().unwrap_or("A game").to_string();
            let s = g["size"].as_u64().unwrap_or_else(|| du(&p));
            (p, s, title)
        }).collect();
    (b, a)
}

impl Node {
    /// This device, as it reports itself.
    pub fn health_report(&self) -> Value {
        let c = self.st();
        json!({"name": self.name(), "kind": self.kind, "edition": self.edition, "version": hearth_core::VERSION,
               "battery": battery(Path::new("/sys/class/power_supply")), "free": crate::free_space(&self.home), "total": total_space(&self.home),
               "lastBackup": if c.backup_to.is_empty() { Value::Null } else { json!(c.last_backup) }, "server": c.server,
               "boot": boot_state(Path::new("/boot"), Path::new("/var/lib/hearth")), "at": crate::now_ms(),
               "bench": bench_brief(&crate::bench::summary())})
    }

    fn reports_save(&self) {
        let r = crate::lock(&self.storage.reports).clone();
        let _ = crate::write_atomic(&self.dir.join("devices.json"), serde_json::to_string(&r).unwrap().as_bytes());
    }

    /// The health page's rows as this device (a server) sees them.
    fn health_rows(&self) -> Vec<Value> {
        let mut reports = crate::lock(&self.storage.reports).clone();
        reports.insert(self.me.clone(), self.health_report());
        let now = crate::now_ms();
        let near_ids: Vec<String> = crate::lock(&self.found).values().filter(|f| now.saturating_sub(f.at) < 60_000).map(|f| f.id.clone()).collect();
        let mut devs = vec![(self.me.clone(), self.name(), true, now, true)];
        for p in crate::lock(&self.peers).list.iter() {
            let near = near_ids.contains(&p.id);
            devs.push((p.id.clone(), p.name.clone(), false, p.last_seen, near || now.saturating_sub(p.last_seen) < 60_000));
        }
        rows(devs, &reports, now)
    }

    /// Requests from paired devices: device.report, devices.list, devices.usage, devices.forget (server).
    pub async fn health_answer(self: Arc<Self>, peer: &str, op: &str, req: &Value) -> Result<Value, String> {
        match op {
            "device.report" => {
                if !self.st().server { return Err("Hearth Storage is off on that device".into()); }
                let mut r = req["report"].clone();
                if !r.is_object() { return Err("No report".into()); }
                // Only what the page shows, and short.
                let keep = ["name", "kind", "edition", "version", "battery", "free", "total", "lastBackup", "boot", "server", "bench"];
                r.as_object_mut().unwrap().retain(|k, v| keep.contains(&k.as_str()) && v.to_string().len() < 400);
                r["at"] = json!(crate::now_ms());
                crate::lock(&self.storage.reports).insert(peer.to_string(), r);
                self.reports_save();
                Ok(json!({"ok": true}))
            }
            "devices.list" => { self.storage_allowed(peer)?; Ok(json!({"devices": self.health_rows(), "server": self.me})) }
            "devices.usage" => { let c = self.storage_allowed(peer)?; self.device_usage(&c, req["id"].as_str().unwrap_or("")).await }
            "devices.forget" => {
                let c = self.storage_allowed(peer)?;
                let id = req["id"].as_str().unwrap_or("").to_string();
                eprintln!("hearth-link: storage: {peer} asked to remove {id}");
                self.device_forget(&c, &id, req["backups"] == true, req["archive"] == true).await
            }
            _ => Err(format!("Unknown request: {op}")),
        }
    }

    async fn device_usage(&self, c: &crate::storage::StorageConfig, id: &str) -> Result<Value, String> {
        if !id.chars().all(|c| c.is_ascii_hexdigit()) || id.len() < 8 { return Err("Not a device".into()); }
        let (b, a, id2) = (c.backups_dir(&self.home), c.archive_dir(&self.home), id.to_string());
        let (bk, ar) = tokio::task::spawn_blocking(move || device_data(&b, &a, &id2)).await.map_err(|e| e.to_string())?;
        Ok(json!({"backups": bk.iter().map(|x| x.1).sum::<u64>(), "archive": ar.iter().map(|x| x.1).sum::<u64>(),
                  "games": ar.iter().map(|x| x.2.clone()).collect::<Vec<_>>()}))
    }

    /// The server forgets a device: unpaired here (and told), and its backups and archived games deleted if asked.
    async fn device_forget(self: &Arc<Self>, c: &crate::storage::StorageConfig, id: &str, backups: bool, archive: bool) -> Result<Value, String> {
        if id == self.me { return Err("That's the Hearth Storage device itself".into()); }
        if !id.chars().all(|c| c.is_ascii_hexdigit()) || id.len() < 8 { return Err("Not a device".into()); }
        let (b, a, id2) = (c.backups_dir(&self.home), c.archive_dir(&self.home), id.to_string());
        let busy: Vec<std::path::PathBuf> = crate::lock(&self.jobs).iter().filter(|j| j.peer == id && j.running.load(std::sync::atomic::Ordering::SeqCst)).map(|j| j.place.clone()).collect();
        let freed = tokio::task::spawn_blocking(move || -> u64 {
            let (bk, ar) = device_data(&b, &a, &id2);
            let mut freed = 0;
            if backups { for (p, s) in bk { if !busy.iter().any(|x| x.starts_with(&p)) && std::fs::remove_dir_all(&p).is_ok() { freed += s; } } }
            if archive { for (p, s, _) in ar { if !busy.iter().any(|x| x.starts_with(&p)) && std::fs::remove_dir_all(&p).is_ok() { freed += s; } } }
            freed
        }).await.map_err(|e| e.to_string())?;
        crate::lock(&self.storage.reports).remove(id);
        self.reports_save();
        let name = crate::lock(&self.peers).get(id).map(|p| p.name.clone());
        if name.is_some() { self.unpair_device(id).await; }
        eprintln!("hearth-link: storage: removed {id}{}{} ({} freed)", if backups { ", its backups" } else { "" }, if archive { ", its archived games" } else { "" }, crate::human_bytes(freed));
        Ok(json!({"freed": freed, "name": name}))
    }

    /// This device's report to its servers (the one it backs up to, and any other it knows), now and then.
    pub async fn health_send(self: &Arc<Self>) {
        let c = self.st();
        let mut to: Vec<String> = c.servers.keys().cloned().collect();
        if !c.backup_to.is_empty() && !to.contains(&c.backup_to) { to.push(c.backup_to.clone()); }
        let report = self.health_report();
        for id in to {
            if !crate::lock(&self.peers).is_paired(&id) { continue; }
            // Never woken for this, and never long.
            let _ = tokio::time::timeout(Duration::from_secs(8), self.request(&id, &json!({"op": "device.report", "report": report}))).await;
        }
    }

    /// The server to ask for the health list and for removing devices: this device when it's one, else the one it
    /// backs up to, else any it knows.
    fn health_server(&self) -> Option<String> {
        let c = self.st();
        if c.server { return Some(self.me.clone()); }
        let ps = crate::lock(&self.peers);
        if !c.backup_to.is_empty() && ps.is_paired(&c.backup_to) { return Some(c.backup_to); }
        c.servers.keys().find(|k| ps.is_paired(k)).cloned()
    }

    /// The UI's storage.devices and storage.device.* commands.
    pub async fn health_command(self: &Arc<Self>, cmd: &str, a: &Value) -> Result<Value, String> {
        let id = || a["id"].as_str().filter(|x| x.len() >= 8 && x.chars().all(|c| c.is_ascii_hexdigit())).map(str::to_string).ok_or("Missing 'id'".to_string());
        let server = self.health_server();
        match cmd {
            "storage.devices" => {
                match server.as_deref() {
                    Some(s) if s == self.me => Ok(json!({"devices": self.health_rows(), "server": self.me, "serverName": self.name()})),
                    Some(s) => {
                        // A fresh report first, so this device's own row is current.
                        let report = self.health_report();
                        let _ = tokio::time::timeout(Duration::from_secs(4), self.request(s, &json!({"op": "device.report", "report": report}))).await;
                        match tokio::time::timeout(Duration::from_secs(5), self.request(s, &json!({"op": "devices.list"}))).await {
                            Ok(Ok(mut v)) => {
                                if let Some(l) = v["devices"].as_array_mut() { for d in l.iter_mut() { d["me"] = json!(d["id"] == self.me.as_str()); } }
                                v["serverName"] = json!(self.server_name_of(s));
                                Ok(v)
                            }
                            // The server's asleep or away: what this device knows itself.
                            _ => Ok(json!({"devices": self.health_rows_local(), "server": s, "serverName": self.server_name_of(s), "serverOffline": true})),
                        }
                    }
                    None => Ok(json!({"devices": self.health_rows_local(), "server": null})),
                }
            }
            "storage.device.usage" => {
                let id = id()?;
                match server.as_deref() {
                    Some(s) if s == self.me => { let c = self.st(); self.device_usage(&c, &id).await }
                    Some(s) if s != id => self.storage_request(s, &json!({"op": "devices.usage", "id": id})).await,
                    _ => Ok(json!({"backups": 0, "archive": 0, "games": []})),
                }
            }
            "storage.device.remove" => {
                let id = id()?;
                if id == self.me { return Err("This is the device you're using: remove it from another one".into()); }
                let (backups, archive) = (a["backups"] == true, a["archive"] == true);
                let name = crate::lock(&self.peers).get(&id).map(|p| p.name.clone()).or_else(|| a["name"].as_str().map(str::to_string)).unwrap_or_else(|| "That device".into());
                let mut freed = 0u64;
                match server.as_deref() {
                    Some(s) if s == self.me => { let c = self.st(); freed = self.device_forget(&c, &id, backups, archive).await?["freed"].as_u64().unwrap_or(0); }
                    Some(s) if s != id => {
                        // The server forgets it too (and deletes what was asked). Nothing's deleted unless it answers.
                        match self.storage_request(s, &json!({"op": "devices.forget", "id": id, "backups": backups, "archive": archive})).await {
                            Ok(v) => freed = v["freed"].as_u64().unwrap_or(0),
                            Err(e) if backups || archive => return Err(format!("{} didn't answer, so nothing was removed: {e}", self.server_name_of(s))),
                            Err(_) => {}
                        }
                    }
                    _ => {}
                }
                if crate::lock(&self.peers).is_paired(&id) { self.unpair_device(&id).await; }
                let mut c = self.st();
                if c.backup_to == id { c.backup_to = String::new(); }
                c.servers.remove(&id);
                let _ = self.st_save(c);
                crate::lock(&self.storage.servers).remove(&id);
                eprintln!("hearth-link: storage: removed {id} from this device");
                Ok(json!({"freed": freed, "name": name}))
            }
            _ => Err(format!("Unknown command: {cmd}")),
        }
    }

    fn server_name_of(&self, id: &str) -> String { crate::lock(&self.peers).get(id).map(|p| p.name.clone()).unwrap_or_else(|| "Hearth Storage".into()) }

    /// Without a server to ask: this device and its paired devices, with this device's own report only.
    fn health_rows_local(&self) -> Vec<Value> {
        let mut reports = std::collections::HashMap::new();
        reports.insert(self.me.clone(), self.health_report());
        let now = crate::now_ms();
        let mut devs = vec![(self.me.clone(), self.name(), true, now, true)];
        for p in crate::lock(&self.peers).list.iter() { devs.push((p.id.clone(), p.name.clone(), false, p.last_seen, now.saturating_sub(p.last_seen) < 60_000)); }
        rows(devs, &reports, now)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn battery_of_this_device_only() {
        let d = tempfile::tempdir().unwrap();
        let pad = d.path().join("ps-controller-battery");
        std::fs::create_dir_all(&pad).unwrap();
        for (f, v) in [("type", "Battery"), ("scope", "Device"), ("capacity", "40"), ("status", "Discharging")] { std::fs::write(pad.join(f), v).unwrap(); }
        assert_eq!(battery(d.path()), Value::Null);
        let bat = d.path().join("BAT0");
        std::fs::create_dir_all(&bat).unwrap();
        for (f, v) in [("type", "Battery\n"), ("capacity", "87\n"), ("status", "Charging\n")] { std::fs::write(bat.join(f), v).unwrap(); }
        assert_eq!(battery(d.path()), json!({"percent": 87, "charging": true}));
    }

    #[test]
    fn boot_state_is_read_not_written() {
        let d = tempfile::tempdir().unwrap();
        assert_eq!(boot_state(d.path(), d.path())["good"], Value::Null);
        std::fs::create_dir_all(d.path().join("grub2")).unwrap();
        std::fs::write(d.path().join("grub2/grubenv"), "# GRUB Environment Block\nhearth_tries=1\n#####").unwrap();
        assert_eq!(boot_state(d.path(), d.path())["good"], false);
        std::fs::write(d.path().join("grub2/grubenv"), "# GRUB Environment Block\nhearth_tries=0\n#####").unwrap();
        std::fs::write(d.path().join("boot-fallback.json"), r#"{"when": "2026-10-29T10:00:00", "rolled_back": true}"#).unwrap();
        let s = boot_state(d.path(), d.path());
        assert_eq!(s["good"], true);
        assert_eq!(s["fellBack"], "2026-10-29T10:00:00");
        assert_eq!(std::fs::read_to_string(d.path().join("grub2/grubenv")).unwrap(), "# GRUB Environment Block\nhearth_tries=0\n#####");
    }

    #[test]
    fn versions_compare_by_number() {
        assert!(newer("0.6.56", "0.6.6"));
        assert!(newer("0.6.10", "0.6.9"));
        assert!(!newer("0.6.5", "0.6.5"));
        assert!(newer("0.7", "0.6.99"));
    }

    #[test]
    fn rows_say_who_is_behind_and_online() {
        let mut r = std::collections::HashMap::new();
        r.insert("a".to_string(), json!({"version": "0.6.56", "at": 1000}));
        r.insert("b".to_string(), json!({"version": "0.6.52", "at": 1000}));
        let now = 1000 + 20 * 60_000;
        let v = rows(vec![("a".into(), "Desk".into(), true, now, true), ("b".into(), "Ally".into(), false, 0, false), ("c".into(), "Old".into(), false, 5, false)], &r, now);
        assert_eq!(v[0]["upToDate"], true);
        assert_eq!(v[1]["upToDate"], false);
        assert_eq!(v[1]["online"], false);
        assert_eq!(v[1]["lastSeen"], 1000);
        assert_eq!(v[2]["upToDate"], Value::Null);
        assert_eq!(v[0]["newest"], "0.6.56");
    }

    #[test]
    fn a_devices_data_on_the_server() {
        let d = tempfile::tempdir().unwrap();
        let (b, a) = (d.path().join("Backups"), d.path().join("Archive"));
        std::fs::create_dir_all(b.join("Ally (abcdef12)/Saves")).unwrap();
        std::fs::write(b.join("Ally (abcdef12)/Saves/x"), vec![0u8; 100]).unwrap();
        std::fs::create_dir_all(b.join("Desk (99999999)")).unwrap();
        std::fs::create_dir_all(a.join("abcdef12-steam-620")).unwrap();
        std::fs::write(a.join("abcdef12-steam-620/game.json"), r#"{"game": {"title": "Portal 2"}, "size": 5000}"#).unwrap();
        std::fs::create_dir_all(a.join("99999999-steam-400")).unwrap();
        let (bk, ar) = device_data(&b, &a, "abcdef1234567890");
        assert_eq!(bk.len(), 1);
        assert_eq!(bk[0].1, 100);
        assert_eq!(ar.len(), 1);
        assert_eq!((ar[0].1, ar[0].2.as_str()), (5000, "Portal 2"));
        assert!(device_data(&b, &a, "ab").0.is_empty());
    }
}
