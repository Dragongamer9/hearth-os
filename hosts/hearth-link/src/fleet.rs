//! Update every device at once: from one Hearth device, Hearth OS updates start on all your paired Hearth devices,
//! each with its own progress.
//!
//! Each device decides for itself, with the same rules as always, and nothing is ever forced:
//! - **Boot safety first** (hearth-boot-guard, only read): after an update didn't start and the device went back to
//!   the previous version, its updates are paused for a few days: it says so and stops. While the version it's on
//!   hasn't been confirmed as starting well yet, it waits (staging another one now could replace the version it
//!   would fall back to).
//! - **Someone's playing**: it waits until the game ends.
//! - **Battery**: below 40% and not charging, it waits until it's plugged in or charged.
//! - **Hold-back** (Hearth Storage, cache.rs): while your devices wait for one of them to start a new version well,
//!   it waits too (the same answer background updates get; at most 3 days, failing open).
//! - Then it runs the same update as Settings › Updates › Update now (`hearth-admin update`): download and stage.
//!   The new version starts at the next restart, which is never done for you; the previous version stays there to
//!   go back to, and hearth-boot-guard's fallback works as always.
//!
//! Between devices: update.start, update.status, update.cancel (only while it waits). From the UI: fleet.update
//! {here?}, fleet.status, fleet.cancel.

use crate::node::Node;
use serde_json::{json, Value};
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

const ADMIN: &str = "/usr/libexec/nimbus/hearth-admin";
/// Below this (and not charging) a device waits for power before updating.
pub const MIN_BATTERY: u64 = 40;
/// A device gives up waiting after a day (someone playing all day, a battery never charged): asked again later.
const WAIT_MAX_MS: u64 = 24 * 3600 * 1000;

#[derive(Debug, PartialEq)]
pub enum Gate { Go, Wait(&'static str), Stop(&'static str) }

/// May this device start updating now? `boot`: health::boot_state; `hold`: cache.rs' hold-back answer; `now_s`:
/// seconds (hearth-boot-guard writes its pause in seconds).
pub fn gate(playing: bool, battery: &Value, boot: &Value, hold: &Value, now_s: u64) -> Gate {
    if boot["pausedUntil"].as_f64().map(|u| u > now_s as f64).unwrap_or(false) { return Gate::Stop("paused"); }
    if boot["good"] == false { return Gate::Wait("confirming"); }
    if playing { return Gate::Wait("waitingGame"); }
    if let Some(p) = battery["percent"].as_u64() { if p < MIN_BATTERY && battery["charging"] != true { return Gate::Wait("waitingBattery"); } }
    if hold["hold"] == true { return Gate::Wait("heldBack"); }
    Gate::Go
}

/// One line of `hearth-admin update`'s output, applied to the status.
pub fn take_line(st: &mut Value, line: &str) {
    if let Some(rest) = line.strip_prefix("progress: ") {
        let (p, what) = rest.split_once(' ').unwrap_or((rest, ""));
        if let Ok(p) = p.parse::<f64>() { st["pct"] = json!(p); }
        if !what.is_empty() { st["desc"] = json!(what); }
    } else if let Some(rest) = line.strip_prefix("stage: ") {
        if let Ok(v) = serde_json::from_str::<Value>(rest) {
            if v["phase"] == "done" { st["staged"] = v["staged"].clone(); }
            if let Some(d) = v["desc"].as_str() { st["desc"] = json!(d); }
        }
    } else if let Some(rest) = line.strip_prefix("error: ") {
        st["error"] = json!(rest.chars().take(300).collect::<String>());
    }
}

/// The end of an update: staged (a restart finishes it), already up to date, waiting for a restart from an earlier
/// update, or failed.
pub fn finish(st: &mut Value, ok: bool) {
    let err = st["error"].as_str().unwrap_or("").to_string();
    st["state"] = json!(if ok {
        if st["staged"] == false { "upToDate" } else { "staged" }
    } else if err.contains("waiting for a restart") { "restart" } else { "error" });
    st["at"] = json!(crate::now_ms());
}

fn status() -> &'static Mutex<Value> { static S: OnceLock<Mutex<Value>> = OnceLock::new(); S.get_or_init(|| Mutex::new(json!({"state": "idle"}))) }
fn set(f: impl FnOnce(&mut Value)) { let mut s = crate::lock(status()); f(&mut s); }
fn busy(state: &str) -> bool { matches!(state, "starting" | "updating" | "confirming" | "waitingGame" | "waitingBattery" | "heldBack") }

/// The update command: Update now's, through pkexec (Hearth's polkit rule: no prompt for the person at the device).
fn update_cmd() -> Option<Vec<String>> {
    if let Ok(c) = std::env::var("HEARTH_LINK_UPDATE_CMD") { return Some(c.split_whitespace().map(str::to_string).collect()); }
    Path::new(ADMIN).exists().then(|| vec!["pkexec".into(), ADMIN.into(), "update".into()])
}

fn run_update(cmd: Vec<String>) -> bool {
    use std::io::BufRead;
    let mut c = match std::process::Command::new(&cmd[0]).args(&cmd[1..]).stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::null()).spawn() {
        Ok(c) => c,
        Err(e) => { set(|s| s["error"] = json!(format!("Couldn't start the update: {e}"))); return false; }
    };
    if let Some(out) = c.stdout.take() {
        for line in std::io::BufReader::new(out).lines().map_while(Result::ok) { set(|s| take_line(s, &line)); }
    }
    c.wait().map(|s| s.success()).unwrap_or(false)
}

impl Node {
    fn update_status_here(&self) -> Value {
        let mut v = crate::lock(status()).clone();
        v["name"] = json!(self.name());
        v["version"] = json!(hearth_core::VERSION);
        v["battery"] = crate::health::battery(Path::new("/sys/class/power_supply"));
        v
    }

    /// Starts this device's update (in the background, waiting for its turn as the rules say). `from`: who asked.
    pub fn update_start(self: &Arc<Self>, from: &str) -> Value {
        let cur = crate::lock(status())["state"].as_str().unwrap_or("idle").to_string();
        if busy(&cur) { return self.update_status_here(); }
        let Some(cmd) = update_cmd() else {
            set(|s| *s = json!({"state": "unsupported", "at": crate::now_ms()}));
            return self.update_status_here();
        };
        set(|s| *s = json!({"state": "starting", "from": from, "since": crate::now_ms(), "at": crate::now_ms()}));
        eprintln!("hearth-link: update: asked by {from}");
        let n = self.clone();
        tokio::spawn(async move {
            let since = crate::now_ms();
            loop {
                if crate::lock(status())["state"] == "cancelled" { return; }
                let boot = crate::health::boot_state(Path::new("/boot"), Path::new("/var/lib/hearth"));
                let battery = crate::health::battery(Path::new("/sys/class/power_supply"));
                let hold = n.cache_command("storage.updates.hold", &json!({})).await.unwrap_or(Value::Null);
                let now_s = crate::now_ms() / 1000;
                match gate(crate::game_running(), &battery, &boot, &hold, now_s) {
                    Gate::Go => break,
                    Gate::Stop(why) => { eprintln!("hearth-link: update: not now: {why}"); set(|s| { s["state"] = json!(why); s["at"] = json!(crate::now_ms()); }); return; }
                    Gate::Wait(why) => {
                        if crate::now_ms().saturating_sub(since) > WAIT_MAX_MS { set(|s| { s["state"] = json!("gaveUp"); s["why"] = json!(why); }); return; }
                        let changed = crate::lock(status())["state"] != why;
                        if changed { eprintln!("hearth-link: update: waiting ({why})"); }
                        set(|s| { s["state"] = json!(why); s["until"] = hold["until"].clone(); s["at"] = json!(crate::now_ms()); });
                    }
                }
                tokio::time::sleep(Duration::from_secs(if std::env::var_os("HEARTH_LINK_UPDATE_CMD").is_some() { 2 } else { 30 })).await;
            }
            set(|s| { s["state"] = json!("updating"); s["pct"] = json!(0); s["error"] = Value::Null; });
            eprintln!("hearth-link: update: updating");
            let ok = tokio::task::spawn_blocking(move || run_update(cmd)).await.unwrap_or(false);
            set(|s| finish(s, ok));
            eprintln!("hearth-link: update: {}", crate::lock(status())["state"].as_str().unwrap_or(""));
        });
        self.update_status_here()
    }

    pub async fn fleet_answer(self: Arc<Self>, peer: &str, op: &str, _req: &Value) -> Result<Value, String> {
        match op {
            "update.start" => { let who = crate::lock(&self.peers).get(peer).map(|p| p.name.clone()).unwrap_or_else(|| "a paired device".into()); Ok(self.update_start(&who)) }
            "update.status" => Ok(self.update_status_here()),
            "update.cancel" => Ok(self.update_cancel()),
            _ => Err(format!("Unknown request: {op}")),
        }
    }

    fn update_cancel(&self) -> Value {
        set(|s| { let st = s["state"].as_str().unwrap_or("").to_string(); if busy(&st) && st != "updating" { s["state"] = json!("cancelled"); } });
        self.update_status_here()
    }

    /// Every paired device's answer to `op` (in parallel, a few seconds each), and this device's.
    async fn fleet_ask(self: &Arc<Self>, op: &str) -> Vec<Value> {
        let peers: Vec<(String, String)> = crate::lock(&self.peers).list.iter().map(|p| (p.id.clone(), p.name.clone())).collect();
        let mut set = tokio::task::JoinSet::new();
        for (id, name) in peers {
            let (n, op) = (self.clone(), op.to_string());
            set.spawn(async move {
                let mut v = match tokio::time::timeout(Duration::from_secs(10), n.request(&id, &json!({"op": op}))).await {
                    Ok(Ok(v)) => v,
                    Ok(Err(e)) if e.starts_with("Unknown request") => json!({"state": "unsupported"}),
                    _ => json!({"state": "offline"}),
                };
                v["id"] = json!(id); v["name"] = json!(name);
                v
            });
        }
        let mut out = Vec::new();
        while let Some(r) = set.join_next().await { if let Ok(v) = r { out.push(v); } }
        out.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
        out
    }

    pub async fn fleet_command(self: &Arc<Self>, cmd: &str, a: &Value) -> Result<Value, String> {
        let me = |v: Value| { let mut v = v; v["id"] = json!(self.me); v["me"] = json!(true); v };
        match cmd {
            "fleet.update" => {
                let mut devices = self.fleet_ask("update.start").await;
                if a["here"] != false { devices.insert(0, me(self.update_start("this device"))); }
                Ok(json!({"devices": devices}))
            }
            "fleet.status" => {
                let mut devices = self.fleet_ask("update.status").await;
                devices.insert(0, me(self.update_status_here()));
                Ok(json!({"devices": devices}))
            }
            "fleet.cancel" => {
                let mut devices = self.fleet_ask("update.cancel").await;
                devices.insert(0, me(self.update_cancel()));
                Ok(json!({"devices": devices}))
            }
            _ => Err(format!("Unknown command: {cmd}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn who_may_update_now() {
        let none = Value::Null;
        let good = json!({"good": true});
        assert_eq!(gate(false, &none, &none, &none, 1000), Gate::Go, "no battery, no boot guard, no server: go");
        assert_eq!(gate(false, &none, &good, &json!({"hold": false}), 1000), Gate::Go);
        assert_eq!(gate(true, &none, &good, &none, 1000), Gate::Wait("waitingGame"), "never while someone plays");
        assert_eq!(gate(false, &json!({"percent": 20, "charging": false}), &good, &none, 1000), Gate::Wait("waitingBattery"));
        assert_eq!(gate(false, &json!({"percent": 20, "charging": true}), &good, &none, 1000), Gate::Go, "plugged in");
        assert_eq!(gate(false, &json!({"percent": 80, "charging": false}), &good, &none, 1000), Gate::Go);
        assert_eq!(gate(false, &none, &good, &json!({"hold": true}), 1000), Gate::Wait("heldBack"));
        // Boot safety comes first: a fallback's pause stops it, an unconfirmed start waits.
        assert_eq!(gate(false, &none, &json!({"good": true, "pausedUntil": 2000.5}), &none, 1000), Gate::Stop("paused"));
        assert_eq!(gate(false, &none, &json!({"good": true, "pausedUntil": 500}), &none, 1000), Gate::Go, "the pause is over");
        assert_eq!(gate(true, &none, &json!({"good": false}), &none, 1000), Gate::Wait("confirming"));
    }

    #[test]
    fn reading_update_now() {
        let mut s = json!({"state": "updating"});
        take_line(&mut s, "log: Downloading the system update…");
        take_line(&mut s, "progress: 42.5 Downloading the system update");
        assert_eq!(s["pct"], 42.5);
        assert_eq!(s["desc"], "Downloading the system update");
        take_line(&mut s, r#"stage: {"phase": "done", "pct": 100.0, "desc": "Done. Restart to finish updating.", "staged": true}"#);
        finish(&mut s, true);
        assert_eq!(s["state"], "staged", "staged: the restart is left to the person");
        let mut s = json!({});
        take_line(&mut s, r#"stage: {"phase": "done", "staged": false}"#);
        finish(&mut s, true);
        assert_eq!(s["state"], "upToDate");
        let mut s = json!({});
        take_line(&mut s, "error: an update is waiting for a restart. Restart first");
        finish(&mut s, false);
        assert_eq!(s["state"], "restart");
        let mut s = json!({});
        take_line(&mut s, "error: couldn't update: no network");
        finish(&mut s, false);
        assert_eq!(s["state"], "error");
        assert!(s["error"].as_str().unwrap().contains("no network"));
    }

    #[test]
    fn what_counts_as_busy() {
        for b in ["starting", "updating", "confirming", "waitingGame", "waitingBattery", "heldBack"] { assert!(busy(b), "{b}"); }
        for b in ["idle", "staged", "upToDate", "error", "paused", "cancelled", "restart", "gaveUp", "unsupported"] { assert!(!busy(b), "{b}"); }
    }
}
