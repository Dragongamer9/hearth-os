//! Power: the power menu (sleep, restart, turn off, Desktop Mode), and power mode and limit through
//! hearth-power (the helper Mantel's settings use too). Port of power-control.js and main.js's power().
//! With Handheld Daemon managing the power limit, hearth-power only goes through HHD (never ryzenadj or the firmware
//! files), and Quick Access uses HHD's own power settings instead (hhd.rs).

use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// hearth-power where the rest of Hearth finds its helpers (paths::desktop_dir, which also follows NIMBUS_SHARE):
/// outside the Hearth session it looked only in /usr/share/nimbus, so a copy with its desktop files elsewhere had
/// no power settings while hearth-hhd and the others worked.
fn helper() -> PathBuf { crate::paths::desktop_dir().join("shell/hearth-power") }

fn call(args: &[&str]) -> Result<Value, String> { call_within(args, Duration::from_secs(30)) }
fn call_within(args: &[&str], limit: Duration) -> Result<Value, String> {
    let h = helper();
    let mut all = vec![h.to_string_lossy().to_string()];
    all.extend(args.iter().map(|s| s.to_string()));
    let refs: Vec<&str> = all.iter().map(|s| s.as_str()).collect();
    let out = crate::system::run_timeout("python3", &refs, limit)?;
    let last = out.trim().lines().last().unwrap_or("").to_string();
    if let Some(e) = last.strip_prefix("error: ") { return Err(e.to_string()); }
    serde_json::from_str(&last).map_err(|_| "Unexpected answer from hearth-power".to_string())
}

fn to_mode(profile: &str) -> Option<&'static str> {
    match profile { "power-saver" => Some("efficiency"), "balanced" => Some("balanced"), "performance" => Some("performance"), _ => None }
}

/// hearth-power's answer, in the page's shape; None when it didn't answer. It asks several services (each up to a
/// few seconds when one is slow), so it gets less time than the fail-safe (15 s) gives a request.
fn ask() -> Option<Value> {
    let r = call_within(&["status"], Duration::from_secs(10)).ok()?;
    let m = &r["mode"];
    let avail: Vec<&str> = m["available"].as_array().map(|a| a.iter().filter_map(|v| v.as_str()).filter_map(to_mode).collect()).unwrap_or_default();
    Some(json!({"mode": {"supported": m["supported"] == true, "mode": m["active"].as_str().and_then(to_mode).unwrap_or("balanced"), "available": avail, "via": m["via"]},
           "tdp": if r["tdp"]["supported"] == true { within_range(r["tdp"].clone()) } else { json!({"supported": false}) }}))
}
/// The limit shown never outside the device's range (a limit remembered from another device showed 30 W of 20).
fn within_range(mut t: Value) -> Value {
    if let (Some(w), Some(lo), Some(hi)) = (t["watts"].as_f64(), t["min"].as_f64(), t["max"].as_f64()) {
        if lo <= hi && (w < lo || w > hi) { t["watts"] = json!(w.clamp(lo, hi)); }
    }
    t
}
fn unsupported() -> Value { json!({"mode": {"supported": false, "mode": "balanced", "available": [], "via": null}, "tdp": {"supported": false}}) }

type Slot = Mutex<Option<(Instant, Value)>>;
/// The last answer, for Quick Access's once-a-second refresh (perf.stats): asking hearth-power every second held
/// the bridge, and every other request with it, while a slow service answered ("stuck in perf.stats").
static LAST: Slot = Mutex::new(None);
static REFRESHING: AtomicBool = AtomicBool::new(false);

/// Asks, and remembers the answer. When there's no answer, the last good one stays (a slow moment doesn't hide the
/// power settings).
fn refresh(slot: &Slot, fetch: fn() -> Option<Value>) -> Value {
    let v = fetch();
    let mut last = slot.lock().unwrap_or_else(|p| p.into_inner());
    let v = v.or_else(|| last.as_ref().map(|(_, v)| v.clone())).unwrap_or_else(unsupported);
    *last = Some((Instant::now(), v.clone()));
    v
}
/// The remembered answer: fresh enough as it is; older, it's still the answer now and a new one is asked for in the
/// background (one at a time); only the very first time is the answer waited for.
fn cached(slot: &'static Slot, busy: &'static AtomicBool, fresh: Duration, fetch: fn() -> Option<Value>) -> Value {
    let last = slot.lock().unwrap_or_else(|p| p.into_inner()).clone();
    match last {
        Some((at, v)) if at.elapsed() < fresh => v,
        Some((_, v)) => {
            if !busy.swap(true, Ordering::SeqCst) {
                std::thread::spawn(move || { refresh(slot, fetch); busy.store(false, Ordering::SeqCst); });
            }
            v
        }
        None => refresh(slot, fetch),
    }
}

/// {mode: {supported, mode, available, via}, tdp: {...}}
pub fn status() -> Value { refresh(&LAST, ask) }
/// The same for something asked every second (Quick Access): never waits for hearth-power once it has answered.
pub fn status_cached() -> Value { cached(&LAST, &REFRESHING, Duration::from_secs(5), ask) }

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU32;

    static A_SLOT: Slot = Mutex::new(None);
    static A_BUSY: AtomicBool = AtomicBool::new(false);
    static A_CALLS: AtomicU32 = AtomicU32::new(0);
    fn a_fetch() -> Option<Value> { let n = A_CALLS.fetch_add(1, Ordering::SeqCst) + 1; Some(json!({"n": n})) }

    #[test]
    fn asks_once_then_answers_from_memory() {
        assert_eq!(cached(&A_SLOT, &A_BUSY, Duration::from_secs(60), a_fetch)["n"], 1);
        assert_eq!(cached(&A_SLOT, &A_BUSY, Duration::from_secs(60), a_fetch)["n"], 1);
        assert_eq!(A_CALLS.load(Ordering::SeqCst), 1);
    }

    static B_SLOT: Slot = Mutex::new(None);
    static B_BUSY: AtomicBool = AtomicBool::new(false);
    static B_CALLS: AtomicU32 = AtomicU32::new(0);
    fn b_fetch() -> Option<Value> {
        let n = B_CALLS.fetch_add(1, Ordering::SeqCst) + 1;
        if n > 1 { std::thread::sleep(Duration::from_millis(300)); }   // a slow helper
        Some(json!({"n": n}))
    }

    #[test]
    fn an_old_answer_comes_at_once_and_a_new_one_in_the_background() {
        assert_eq!(cached(&B_SLOT, &B_BUSY, Duration::ZERO, b_fetch)["n"], 1);
        let t = Instant::now();
        assert_eq!(cached(&B_SLOT, &B_BUSY, Duration::ZERO, b_fetch)["n"], 1);
        assert_eq!(cached(&B_SLOT, &B_BUSY, Duration::ZERO, b_fetch)["n"], 1);
        assert!(t.elapsed() < Duration::from_millis(200), "waited for the helper");
        for _ in 0..50 { if B_SLOT.lock().unwrap().as_ref().map(|(_, v)| v["n"] == 2).unwrap_or(false) { break; } std::thread::sleep(Duration::from_millis(20)); }
        assert_eq!(cached(&B_SLOT, &B_BUSY, Duration::from_secs(60), b_fetch)["n"], 2);
        assert_eq!(B_CALLS.load(Ordering::SeqCst), 2, "one background refresh at a time");
    }

    static C_SLOT: Slot = Mutex::new(None);
    fn c_good() -> Option<Value> { Some(json!({"mode": {"supported": true}, "tdp": {"supported": true}})) }
    fn c_none() -> Option<Value> { None }

    #[test]
    fn limit_shown_within_the_range() {
        assert_eq!(within_range(json!({"watts": 30, "min": 4, "max": 20}))["watts"], json!(20.0));
        assert_eq!(within_range(json!({"watts": 15, "min": 4, "max": 20}))["watts"], json!(15));
        assert_eq!(within_range(json!({"watts": null, "min": 4, "max": 20}))["watts"], Value::Null);
    }
    #[test]
    fn no_answer_keeps_the_last_good_one() {
        assert_eq!(refresh(&C_SLOT, c_none)["tdp"]["supported"], false);
        refresh(&C_SLOT, c_good);
        assert_eq!(refresh(&C_SLOT, c_none)["tdp"]["supported"], true);
    }
}
