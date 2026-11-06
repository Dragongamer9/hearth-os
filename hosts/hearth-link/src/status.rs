//! Your devices: each paired Hearth device's live status (online, mode, what's playing, battery, CPU / GPU use and
//! temperature, memory, graphics memory, drives, version, uptime, and the last hour in small graphs) on Settings ›
//! Hearth Link › Your devices.
//!
//! Only over Hearth Link: paired devices only (a connection from anyone else is refused before any request, and
//! status.get checks again), end-to-end encrypted, at home or anywhere (direct, Tailscale or the relay). No port is
//! opened and nothing goes to the internet. The numbers come from `hearth-device-status` (Mantel's shell folder),
//! which reads /proc and /sys only and answers `{"shared": false}` when "Let my other devices see this device's
//! status" is off on that device. No secrets: no addresses, accounts, keys or file names.
//!
//! Between devices: status.get. From the UI: status.devices (every paired device, asked at once).

use crate::node::Node;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;

/// Mantel's helper that reads this device's status (and keeps the last hour while sharing is on).
pub const HELPER: &str = "/usr/share/nimbus/desktop/shell/hearth-device-status";
/// How often this device keeps a point for its last-hour graphs (only while sharing is on).
const SAMPLE_EVERY: Duration = Duration::from_secs(60);
/// How long a device has to answer the list.
const ASK_WAIT: Duration = Duration::from_secs(6);

fn helper_path() -> String { std::env::var("HEARTH_DEVICE_STATUS").unwrap_or_else(|_| HELPER.to_string()) }

/// Runs the helper with one argument and reads its last line as JSON.
pub fn run_helper(arg: &str) -> Result<Value, String> {
    let path = helper_path();
    if !std::path::Path::new(&path).exists() { return Err("This comes with the next Hearth OS update".into()); }
    let out = std::process::Command::new("python3").arg(&path).arg(arg).stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null()).output().map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&out.stdout);
    let line = text.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("");
    if !out.status.success() || line.starts_with("error: ") { return Err(line.trim_start_matches("error: ").to_string()); }
    serde_json::from_str(line).map_err(|e| e.to_string())
}

/// What a device may answer: only a paired device gets anything, and only the "not shared" answer when sharing is
/// off. A status that says shared but carries nothing is treated as not shared.
pub fn answer_for(paired: bool, status: Result<Value, String>) -> Result<Value, String> {
    if !paired { return Err("Not a paired device".into()); }
    match status {
        Ok(v) if v["shared"] == true => Ok(v),
        Ok(_) => Ok(json!({"protocol": 1, "shared": false})),
        Err(e) => Err(e),
    }
}

/// An error that means the device couldn't be reached (off, asleep, offline), not that it answered.
fn offline_error(e: &str) -> bool { e.starts_with("connection") || e.starts_with("Can't reach") || e.contains("timed out") }

/// One row of the list: the device as Hearth Link knows it, with its answer (or why there's none).
pub fn row(id: &str, name: &str, last_seen: u64, answer: Option<Result<Value, String>>) -> Value {
    match answer {
        Some(Ok(v)) if v["shared"] == true => json!({"id": id, "name": name, "online": true, "lastSeen": crate::now_ms(), "shared": true, "status": v}),
        Some(Ok(_)) => json!({"id": id, "name": name, "online": true, "lastSeen": crate::now_ms(), "shared": false}),
        Some(Err(e)) if !offline_error(&e) && !e.contains("Unknown request") =>
            json!({"id": id, "name": name, "online": true, "lastSeen": crate::now_ms(), "shared": false, "error": e}),
        Some(Err(e)) if e.contains("Unknown request") =>
            json!({"id": id, "name": name, "online": true, "lastSeen": crate::now_ms(), "shared": false, "old": true}),
        _ => json!({"id": id, "name": name, "online": false, "lastSeen": last_seen}),
    }
}

impl Node {
    pub async fn status_answer(self: Arc<Self>, peer: &str, op: &str, _req: &Value) -> Result<Value, String> {
        match op {
            "status.get" => {
                let paired = crate::lock(&self.peers).is_paired(peer);
                let st = if paired { tokio::task::spawn_blocking(|| run_helper("status")).await.map_err(|e| e.to_string())? } else { Err(String::new()) };
                answer_for(paired, st)
            }
            _ => Err(format!("Unknown request: {op}")),
        }
    }

    pub async fn status_command(self: &Arc<Self>, cmd: &str, _a: &Value) -> Result<Value, String> {
        match cmd {
            "status.devices" => {
                let peers: Vec<(String, String, u64)> = crate::lock(&self.peers).list.iter().map(|p| (p.id.clone(), p.name.clone(), p.last_seen)).collect();
                let mut set = tokio::task::JoinSet::new();
                for (id, name, seen) in peers {
                    let n = self.clone();
                    set.spawn(async move {
                        let r = tokio::time::timeout(ASK_WAIT, n.request(&id, &json!({"op": "status.get"}))).await;
                        let answer = match r { Ok(a) => Some(a), Err(_) => None };
                        row(&id, &name, seen, answer)
                    });
                }
                let mut devices = Vec::new();
                while let Some(r) = set.join_next().await { if let Ok(v) = r { devices.push(v); } }
                devices.sort_by_key(|d| d["name"].as_str().unwrap_or("").to_lowercase());
                Ok(json!({"devices": devices}))
            }
            _ => Err(format!("Unknown command: {cmd}")),
        }
    }

    /// Keeps a point for the last-hour graphs every minute (the helper does nothing while sharing is off).
    pub fn status_start(self: &Arc<Self>) {
        if std::env::var_os("HEARTH_LINK_NO_STATUS").is_some() { return; }
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(SAMPLE_EVERY).await;
                let _ = tokio::task::spawn_blocking(|| run_helper("sample")).await;
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_paired_devices_get_an_answer() {
        let full = json!({"protocol": 1, "shared": true, "cpu": {"use": 12}});
        assert!(answer_for(false, Ok(full.clone())).is_err());
        assert_eq!(answer_for(true, Ok(full.clone())).unwrap(), full);
        // Sharing off on that device: only that it isn't shared.
        assert_eq!(answer_for(true, Ok(json!({"shared": false, "cpu": 1}))).unwrap(), json!({"protocol": 1, "shared": false}));
        assert_eq!(answer_for(true, Ok(json!({"cpu": 1}))).unwrap(), json!({"protocol": 1, "shared": false}));
    }

    #[test]
    fn rows() {
        let r = row("a", "Ally", 5, None);
        assert_eq!(r["online"], false);
        assert_eq!(r["lastSeen"], 5);
        let r = row("a", "Ally", 5, Some(Ok(json!({"shared": true, "mode": "game"}))));
        assert_eq!(r["online"], true);
        assert_eq!(r["status"]["mode"], "game");
        let r = row("a", "Ally", 5, Some(Ok(json!({"shared": false}))));
        assert_eq!(r["shared"], false);
        assert!(r.get("status").is_none());
        let r = row("a", "Ally", 5, Some(Err("Unknown request: status.get".into())));
        assert_eq!(r["old"], true);
        let r = row("a", "Ally", 5, Some(Err("connection: lost".into())));
        assert_eq!(r["online"], false);
        let r = row("a", "Ally", 5, Some(Err("Can't reach Ally (it's off, asleep or offline)".into())));
        assert_eq!(r["online"], false);
    }
}
