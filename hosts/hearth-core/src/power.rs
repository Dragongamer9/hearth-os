//! Power: the power menu (sleep, restart, turn off, Desktop Mode), and power mode and limit through
//! hearth-power (the helper Mantel's settings use too). Port of power-control.js and main.js's power().

use serde_json::{json, Value};
use std::path::PathBuf;

fn helper() -> PathBuf {
    let share = std::env::var_os("NIMBUS_SHARE").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/usr/share/nimbus"));
    share.join("desktop/shell/hearth-power")
}

fn call(args: &[&str]) -> Result<Value, String> {
    let h = helper();
    let mut all = vec![h.to_string_lossy().to_string()];
    all.extend(args.iter().map(|s| s.to_string()));
    let refs: Vec<&str> = all.iter().map(|s| s.as_str()).collect();
    let out = crate::system::run_timeout("python3", &refs, std::time::Duration::from_secs(30))?;
    let last = out.trim().lines().last().unwrap_or("").to_string();
    if let Some(e) = last.strip_prefix("error: ") { return Err(e.to_string()); }
    serde_json::from_str(&last).map_err(|_| "Unexpected answer from hearth-power".to_string())
}

fn to_mode(profile: &str) -> Option<&'static str> {
    match profile { "power-saver" => Some("efficiency"), "balanced" => Some("balanced"), "performance" => Some("performance"), _ => None }
}

/// {mode: {supported, mode, available, via}, tdp: {...}}
pub fn status() -> Value {
    match call(&["status"]) {
        Ok(r) => {
            let m = &r["mode"];
            let avail: Vec<&str> = m["available"].as_array().map(|a| a.iter().filter_map(|v| v.as_str()).filter_map(to_mode).collect()).unwrap_or_default();
            json!({"mode": {"supported": m["supported"] == true, "mode": m["active"].as_str().and_then(to_mode).unwrap_or("balanced"), "available": avail, "via": m["via"]},
                   "tdp": if r["tdp"]["supported"] == true { r["tdp"].clone() } else { json!({"supported": false}) }})
        }
        Err(_) => json!({"mode": {"supported": false, "mode": "balanced", "available": [], "via": null}, "tdp": {"supported": false}}),
    }
}

pub fn set_mode(mode: &str) -> Result<(), String> {
    let p = match mode { "efficiency" => "power-saver", "balanced" => "balanced", "performance" => "performance", _ => return Err("Unknown mode".into()) };
    call(&["mode", p]).map(|_| ())
}
pub fn set_tdp(watts: u32) -> Result<Value, String> { call(&["tdp", &watts.to_string()])?; Ok(status()["tdp"].clone()) }

/// What the power menu asked for. Returns true when Hearth should close (switching to the desktop, signing out).
pub fn action(what: &str, in_session: bool) -> Result<bool, String> {
    let sys = |verb: &str| crate::system::run("systemctl", &[verb]).map(|_| false);
    match what {
        "sleep" => sys("suspend"),
        "restart" => sys("reboot"),
        "shutdown" => sys("poweroff"),
        "hibernate" => sys("hibernate"),
        "desktop" | "signout" if in_session => {
            // The session reads the next mode from this file when Hearth closes ("desktop" → Desktop Mode).
            let f = PathBuf::from(std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into())).join("nimbus-next");
            if what == "desktop" { std::fs::write(&f, "desktop\n").map_err(|e| e.to_string())?; } else { let _ = std::fs::remove_file(&f); }
            Ok(true)
        }
        _ => Ok(false),
    }
}

/// Can this PC turn off with everything kept (hibernation)? {supported, canSetUp, reason}
/// Needs: the kernel allows it (Secure Boot's lockdown doesn't), swap on the drive at least as big as memory
/// (Hearth sets that up: hearth-admin hibernate-setup), and the system says yes.
pub fn hibernate_status() -> Value {
    let state = std::fs::read_to_string("/sys/power/state").unwrap_or_default();
    let lockdown = std::fs::read_to_string("/sys/kernel/security/lockdown").unwrap_or_default();
    let locked = !lockdown.is_empty() && !lockdown.contains("[none]");
    let kb = |text: &str, k: &str| text.lines().find(|l| l.starts_with(&format!("{k}:"))).and_then(|l| l.split_whitespace().nth(1)).and_then(|n| n.parse::<u64>().ok()).unwrap_or(0);
    let mem = kb(&std::fs::read_to_string("/proc/meminfo").unwrap_or_default(), "MemTotal");
    // Swap on the drive (zram is in memory, so it can't keep anything while off): sizes in KiB.
    let disk_swap: u64 = std::fs::read_to_string("/proc/swaps").unwrap_or_default().lines().skip(1)
        .filter(|l| !l.starts_with("/dev/zram")).filter_map(|l| l.split_whitespace().nth(2).and_then(|n| n.parse::<u64>().ok())).sum();
    let logind = crate::system::run("gdbus", &["call", "--system", "--dest", "org.freedesktop.login1", "--object-path", "/org/freedesktop/login1", "--method", "org.freedesktop.login1.Manager.CanHibernate"]).unwrap_or_default();
    let (supported, can_set_up, reason) = if !state.contains("disk") { (false, false, "This PC's system doesn't support turning off with games kept.") }
        else if locked { (false, false, "Secure Boot is on, which doesn't allow saving memory to the drive. Games put aside close when the PC turns off.") }
        else if disk_swap < mem { (false, crate::admin::can_admin(), "Needs room on the drive the size of this PC's memory. Set it up once, then restart.") }
        else if !logind.contains("'yes'") { (false, false, "The system isn't allowing it right now.") }
        else { (true, false, "") };
    json!({"supported": supported, "canSetUp": can_set_up, "reason": if reason.is_empty() { Value::Null } else { json!(reason) }, "memoryGB": (mem as f64 / 1048576.0).round()})
}
