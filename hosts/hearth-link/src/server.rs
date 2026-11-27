//! Hearth Server between your own PCs (docs/SERVER-HUB.md): the Home server helper's Game servers, Remote play and
//! PCs pages ask another of your Hearth PCs something, and that PC answers with its own `hearth-server-hub
//! link-answer` (Mantel's shell folder: the Store's copy when there is one, else the built-in one).
//!
//! - **Only your own devices** (relation "mine"): a family member's or a friend's device is never asked and never
//!   answered (family.rs `allowed` refuses `server.ask` for them before anything runs; this checks again).
//! - **Only what hearth-server-hub answers** (`what` is one of [`WHATS`]); each checks its own arguments, and the PC
//!   being asked decides for itself (it takes jobs only when "Let this PC take jobs" is on there).
//! - **Waking from away**: `server.wake` asks your other devices that are on (at home) to send the Wake-on-LAN packet
//!   for a PC that's asleep, since the packet can't cross the internet (the cards are the ones Hearth Storage or that
//!   device taught this one).
//! - **Hearth OS updates counted**: what this device serves your others from its update copy (`updates.read`) is
//!   added up per day and device in `updates-served.json`, for the Game cache page.
//!
//! Between devices: server.ask, server.relay-wake. From the UI: server.ask {peer, op, args}, server.wake {peer}.

use crate::node::Node;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

/// What another of your PCs may ask hearth-server-hub here.
pub const WHATS: &[&str] = &["status", "games", "mods", "file", "history", "relay-wake", "job-start", "job-stop", "job-world", "play-quality", "play-open", "play-report"];
/// Mantel's folder built into Hearth OS (hearth-part finds the Store's copy).
const SHELL: &str = "/usr/share/nimbus/desktop/shell";
const PART: &str = "/usr/libexec/hearth/hearth-part";
/// How long the other PC may take (a world moving, a game server starting).
const ASK_WAIT: Duration = Duration::from_secs(900);

pub fn valid_what(w: &str) -> bool { WHATS.contains(&w) }

/// Mantel's shell folder: $HEARTH_SHELL_DIR, the Store's copy (hearth-part path mantel), or the built-in one.
fn shell_dir() -> String {
    if let Ok(d) = std::env::var("HEARTH_SHELL_DIR") { return d; }
    if let Ok(o) = std::process::Command::new(PART).args(["path", "mantel", SHELL]).stdin(std::process::Stdio::null()).stderr(std::process::Stdio::null()).output() {
        let p = String::from_utf8_lossy(&o.stdout).trim().to_string();
        if o.status.success() && !p.is_empty() && std::path::Path::new(&p).join("hearth-server-hub").exists() { return p; }
    }
    SHELL.to_string()
}

/// One answer from hearth-server-hub: its last line as JSON ("error: …" becomes the error).
pub fn run_hub(what: &str, args: &Value) -> Result<Value, String> {
    if !valid_what(what) { return Err("Not something a PC answers".into()); }
    let hub = std::path::Path::new(&shell_dir()).join("hearth-server-hub");
    if !hub.exists() { return Err("Hearth Server isn't on that PC yet (it comes with Mantel from the Hearth Store)".into()); }
    let out = std::process::Command::new("python3").arg(&hub).arg("link-answer").arg(what).arg(args.to_string())
        .stdin(std::process::Stdio::null()).stderr(std::process::Stdio::null()).output().map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&out.stdout);
    let line = text.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("");
    if let Some(e) = line.strip_prefix("error: ") { return Err(e.to_string()); }
    serde_json::from_str(line).map_err(|_| "That PC's Hearth Server didn't answer".to_string())
}

/// Hearth OS updates served to your other devices: {day: {device: bytes}}, kept 60 days.
pub fn count_served(v: &mut Value, day: &str, device: &str, bytes: u64) {
    if !v.is_object() { *v = json!({}); }
    let d = &mut v[day];
    if !d.is_object() { *d = json!({}); }
    let cur = d[device].as_u64().unwrap_or(0);
    d[device] = json!(cur + bytes);
    if let Some(m) = v.as_object_mut() {
        let mut days: Vec<String> = m.keys().cloned().collect();
        days.sort();
        let extra = days.len().saturating_sub(60);
        for k in days.into_iter().take(extra) { m.remove(&k); }
    }
}

fn served() -> &'static Mutex<()> { static M: OnceLock<Mutex<()>> = OnceLock::new(); M.get_or_init(|| Mutex::new(())) }

fn today() -> String {
    let days = crate::now_ms() / 86_400_000;
    // civil date from days since 1970 (Howard Hinnant's algorithm)
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{y:04}-{m:02}-{d:02}")
}

impl Node {
    /// updates.read sent `bytes` of an update to `peer`: counted for the Game cache page.
    pub fn server_count_update(&self, peer: &str, bytes: u64) {
        if bytes == 0 { return; }
        let _g = served().lock();
        let path = self.dir.join("updates-served.json");
        let mut v: Value = std::fs::read(&path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_else(|| json!({}));
        let name = crate::lock(&self.peers).get(peer).map(|p| p.name.clone()).unwrap_or_else(|| "a device".into());
        count_served(&mut v, &today(), &name, bytes);
        let _ = crate::write_atomic(&path, v.to_string().as_bytes());
    }

    pub async fn server_answer(self: Arc<Self>, peer: &str, op: &str, req: &Value) -> Result<Value, String> {
        if !crate::lock(&self.peers).is_mine(peer) { return Err("Hearth Server answers only your own devices".into()); }
        match op {
            "server.ask" => {
                let what = req["what"].as_str().unwrap_or("").to_string();
                if !valid_what(&what) { return Err("Not something a PC answers".into()); }
                let args = if req["args"].is_object() { req["args"].clone() } else { json!({}) };
                tokio::task::spawn_blocking(move || run_hub(&what, &args)).await.map_err(|e| e.to_string())?
            }
            "server.relay-wake" => {
                let macs: Vec<String> = req["macs"].as_array().cloned().unwrap_or_default().iter().filter_map(|m| m.as_str().map(str::to_string)).take(8).collect();
                if macs.is_empty() { return Err("No network card to wake".into()); }
                Ok(json!({"sent": crate::storage::wake(&macs)}))
            }
            _ => Err(format!("Unknown request: {op}")),
        }
    }

    pub async fn server_command(self: &Arc<Self>, cmd: &str, a: &Value) -> Result<Value, String> {
        match cmd {
            "server.ask" => {
                let peer = a["peer"].as_str().unwrap_or("").to_string();
                let what = a["op"].as_str().unwrap_or("");
                if !valid_what(what) { return Err("Not something a PC answers".into()); }
                let (mine, name) = { let ps = crate::lock(&self.peers); ps.get(&peer).map(|p| (p.mine(), p.name.clone())).unwrap_or((false, String::new())) };
                if !mine { return Err("Only your own devices".into()); }
                let req = json!({"op": "server.ask", "what": what, "args": if a["args"].is_object() { a["args"].clone() } else { json!({}) }});
                match tokio::time::timeout(ASK_WAIT, self.request(&peer, &req)).await {
                    Ok(Ok(v)) => { if let Some(e) = v["error"].as_str() { return Err(e.to_string()); } Ok(v) }
                    Ok(Err(e)) => Err(e),
                    Err(_) => Err(format!("{name} didn't answer")),
                }
            }
            "server.wake" => {
                // The asleep PC's network cards (its own, or what Hearth Storage learned), sent from this device and
                // from every other of your devices that's on, so it wakes from away too.
                let id = a["peer"].as_str().unwrap_or("").to_string();
                let (macs, name, others) = {
                    let ps = crate::lock(&self.peers);
                    let p = ps.get(&id).ok_or("Not one of your devices")?;
                    if !p.mine() { return Err("Only your own devices".into()); }
                    let others: Vec<String> = ps.mine().filter(|q| q.id != id).map(|q| q.id.clone()).collect();
                    (p.macs.clone(), p.name.clone(), others)
                };
                let macs = if macs.is_empty() {
                    self.st().servers.get(&id).and_then(|v| v["macs"].as_array().cloned()).unwrap_or_default().iter().filter_map(|m| m.as_str().map(str::to_string)).collect()
                } else { macs };
                if macs.is_empty() { return Err(format!("This device doesn't know how to wake {name} yet: it learns that the next time both are on")); }
                let mut sent = crate::storage::wake(&macs);
                let mut via = Vec::new();
                for o in others {
                    let req = json!({"op": "server.relay-wake", "macs": macs});
                    if let Ok(Ok(v)) = tokio::time::timeout(Duration::from_secs(8), self.request(&o, &req)).await {
                        if v["sent"].as_u64().unwrap_or(0) > 0 {
                            sent += v["sent"].as_u64().unwrap_or(0) as usize;
                            if let Some(n) = crate::lock(&self.peers).get(&o).map(|p| p.name.clone()) { via.push(n); }
                        }
                    }
                }
                Ok(json!({"sent": sent, "via": via.join(", ")}))
            }
            _ => Err(format!("Unknown command: {cmd}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_these_questions() {
        assert!(valid_what("status"));
        assert!(valid_what("job-start"));
        assert!(!valid_what("tick"));
        assert!(!valid_what("game-kick"));
        assert!(!valid_what(""));
        assert!(run_hub("set", &json!({})).is_err());
    }

    #[test]
    fn updates_counted_per_day_and_device() {
        let mut v = json!({});
        count_served(&mut v, "2026-11-25", "ROG Ally", 100);
        count_served(&mut v, "2026-11-25", "ROG Ally", 50);
        count_served(&mut v, "2026-11-26", "Laptop", 7);
        assert_eq!(v["2026-11-25"]["ROG Ally"], 150);
        assert_eq!(v["2026-11-26"]["Laptop"], 7);
        for d in 1..=70 { count_served(&mut v, &format!("2027-01-{d:02}"), "x", 1); }
        assert_eq!(v.as_object().unwrap().len(), 60);
    }

    #[test]
    fn today_is_a_date() {
        let t = today();
        assert_eq!(t.len(), 10);
        assert!(t.starts_with("20"));
    }
}
