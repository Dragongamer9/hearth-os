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
/// The power limit you chose, set again (`hearth-power restore`): many handhelds (ryzenadj on the ROG Ally) go back
/// to their own limit after sleep. Nothing to do (Ok(None)) for an extra multi-seat player (hearth-power refuses),
/// without the helper, or with no limit chosen.
pub fn restore() -> Result<Option<Value>, String> {
    if std::env::var_os("HEARTH_SEAT").is_some_and(|s| !s.is_empty()) || !helper().exists() { return Ok(None); }
    let r = call(&["restore"])?;
    Ok(Some(r["tdp"].clone()).filter(|t| !t.is_null()))
}

/// What the power menu asked for. Returns true when Hearth should close (switching to the desktop, signing out).
/// "hibernate" turns the PC off with the games kept; "hibernateRestart" restarts with them kept (hibernation that
/// ends in a restart, through hearth-admin hibernate-reboot: only that one hibernation restarts).
pub fn action(what: &str, in_session: bool) -> Result<bool, String> {
    let sys = |verb: &str| crate::system::run("systemctl", &[verb]).map(|_| false);
    match what {
        "sleep" => sys("suspend"),
        "restart" => sys("reboot"),
        "shutdown" => sys("poweroff"),
        "hibernate" => sys("hibernate"),
        "hibernateRestart" => crate::admin::run(&["hibernate-reboot"], None, &mut |_| {}).map(|_| false),
        "desktop" | "signout" if in_session => {
            // The session reads the next mode from this file when Hearth closes ("desktop" → Desktop Mode).
            let f = PathBuf::from(std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into())).join("nimbus-next");
            if what == "desktop" { std::fs::write(&f, "desktop\n").map_err(|e| e.to_string())?; } else { let _ = std::fs::remove_file(&f); }
            Ok(true)
        }
        _ => Ok(false),
    }
}

/// The power button set to "Turn off" (Settings › Power button): with Quick Resume games to keep and keeping them
/// supported, it turns off with them kept, like the power menu's Shut down.
pub fn shutdown_verb(games_to_keep: bool) -> &'static str {
    if games_to_keep && hibernate_status()["supported"] == true { "hibernate" } else { "shutdown" }
}

/// Nvidia's driver, for keeping games: without its video memory saved (PreserveVideoMemoryAllocations, in a
/// folder on the drive with room for it) and its hibernate helper, the picture comes back black or broken.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NvFacts {
    /// PreserveVideoMemoryAllocations from /proc/driver/nvidia/params (None: not there).
    pub preserve: Option<bool>,
    /// TemporaryFilePath (where it saves video memory; empty: /tmp, which is in memory here).
    pub temp_path: String,
    /// Free space there and the card's memory, in KiB (None: couldn't tell).
    pub temp_free_kb: Option<u64>,
    pub vram_kb: Option<u64>,
    /// nvidia-hibernate.service is installed, and turned on. Newer drivers (Anatase's) don't ship it: they save
    /// the memory themselves from the kernel's own suspend notifications (UseKernelSuspendNotifiers).
    pub helper_installed: bool,
    pub hibernate_service: bool,
    pub kernel_notifiers: bool,
}

/// Everything hibernate_status() looks at, read by facts(); a plain struct so verdict() can be tested.
#[derive(Clone, Debug, Default)]
pub struct HibFacts {
    pub state: String,
    pub disk_modes: String,
    pub lockdown: String,
    pub mem_kb: u64,
    pub disk_swap_kb: u64,
    pub hearth_swap_kb: u64,
    pub resume_arg: bool,
    pub logind: String,
    pub nvidia: Option<NvFacts>,
    pub can_admin: bool,
}

/// The verdict: (supported, canSetUp, code, plain-words reason). The reasons say what's wrong and what to do.
pub fn verdict(f: &HibFacts) -> (bool, bool, &'static str, String) {
    let gb = |kb: u64| ((kb as f64) / 1048576.0).ceil() as u64;
    if !f.state.split_whitespace().any(|w| w == "disk") {
        return (false, false, "kernel", "This PC's system can't save memory to the drive, so Quick Resume keeps games through sleep only.".into());
    }
    if !f.lockdown.trim().is_empty() && !f.lockdown.contains("[none]") {
        return (false, false, "secure-boot", "Secure Boot is on, and it doesn't allow saving memory to the drive. Quick Resume keeps games through sleep only. To keep them through restarts and turning off too, turn Secure Boot off in the PC's firmware settings (BIOS).".into());
    }
    if f.disk_swap_kb < f.mem_kb {
        let why = if f.hearth_swap_kb > 0 {
            format!("The room set aside on the drive ({} GB) is smaller than this PC's memory now ({} GB). Set it up again to keep games through restarts and turning off.", gb(f.hearth_swap_kb), gb(f.mem_kb))
        } else {
            format!("Quick Resume keeps games through sleep only right now. To keep them through restarts and turning off, set aside room on the drive the size of this PC's memory (about {} GB), then restart once.", gb(f.mem_kb))
        };
        return (false, f.can_admin, "no-room", why);
    }
    if let Some(nv) = &f.nvidia {
        if nv.preserve == Some(false) {
            return (false, false, "nvidia-memory", "Nvidia's driver isn't set to save the graphics card's memory, so the picture would come back black. Update Hearth OS (the update sets it), then restart.".into());
        }
        let p = nv.temp_path.trim_matches('"');
        if nv.preserve == Some(true) && (p.is_empty() || p == "/tmp" || p.starts_with("/tmp/") || p.starts_with("/dev/shm") || p.starts_with("/run")) {
            return (false, false, "nvidia-folder", "Nvidia's driver saves the graphics card's memory to a folder that's in memory itself, which is lost when the PC turns off. Update Hearth OS (the update moves it to the drive), then restart.".into());
        }
        if let (Some(free), Some(vram)) = (nv.temp_free_kb, nv.vram_kb) {
            if free < vram {
                return (false, false, "nvidia-space", format!("Not enough free space on the drive for the graphics card's memory: it needs about {} GB free, and {} GB is. Free up some space to keep games through restarts and turning off.", gb(vram), free / 1048576));
            }
        }
        if nv.helper_installed && !nv.hibernate_service && !nv.kernel_notifiers {
            return (false, false, "nvidia-helper", "Nvidia's helper for turning off with memory saved (nvidia-hibernate) isn't turned on, so the picture would come back black. Update Hearth OS, then restart.".into());
        }
    }
    if !f.logind.contains("'yes'") {
        if !f.resume_arg {
            return (false, false, "restart-needed", "Almost ready: restart the PC once to finish setting up, then Quick Resume keeps games through restarts and turning off.".into());
        }
        let said = f.logind.split('\'').nth(1).unwrap_or("no answer");
        return (false, false, "system", format!("The system isn't allowing it right now (it says \"{said}\"). Quick Resume keeps games through sleep only. The bug report has the details."));
    }
    (true, false, "", String::new())
}

/// "PreserveVideoMemoryAllocations: 1", "TemporaryFilePath: \"/var/tmp\"" from /proc/driver/nvidia/params.
pub fn nv_params(text: &str) -> (Option<bool>, String, bool) {
    let get = |k: &str| text.lines().find_map(|l| l.trim().strip_prefix(&format!("{k}:")).map(|v| v.trim().to_string()));
    (get("PreserveVideoMemoryAllocations").map(|v| v != "0"), get("TemporaryFilePath").unwrap_or_default().trim_matches('"').to_string(),
     get("UseKernelSuspendNotifiers").map(|v| v != "0").unwrap_or(false))
}
/// /proc/swaps: (swap on the drive in KiB, Hearth's own swap file in KiB). zram is in memory, so it can't keep anything.
pub fn disk_swap(swaps: &str) -> (u64, u64) {
    let mut all = 0; let mut hearth = 0;
    for l in swaps.lines().skip(1) {
        let c: Vec<&str> = l.split_whitespace().collect();
        if c.len() < 3 || c[0].starts_with("/dev/zram") { continue; }
        let kb = c[2].parse::<u64>().unwrap_or(0);
        all += kb;
        if c[0] == "/var/swap/hearth-hibernate" || c[0].ends_with("/swap/hearth-hibernate") { hearth += kb; }
    }
    (all, hearth)
}
/// `df -Pk PATH`'s free KiB.
pub fn df_free_kb(out: &str) -> Option<u64> { out.lines().nth(1)?.split_whitespace().nth(3)?.parse().ok() }

fn nvidia_facts() -> Option<NvFacts> {
    if !std::path::Path::new("/proc/driver/nvidia").exists() { return None; }
    let (preserve, temp_path, kernel_notifiers) = nv_params(&std::fs::read_to_string("/proc/driver/nvidia/params").unwrap_or_default());
    let dir = if temp_path.is_empty() { "/tmp".to_string() } else { temp_path.clone() };
    let temp_free_kb = crate::system::run("df", &["-Pk", dir.as_str()]).ok().and_then(|o| df_free_kb(&o));
    let vram_kb = crate::system::run("nvidia-smi", &["--query-gpu=memory.total", "--format=csv,noheader,nounits"]).ok()
        .map(|o| o.lines().filter_map(|l| l.trim().parse::<u64>().ok()).sum::<u64>() * 1024).filter(|k| *k > 0);
    let helper_installed = ["/usr/lib/systemd/system/nvidia-hibernate.service", "/etc/systemd/system/nvidia-hibernate.service"].iter().any(|p| std::path::Path::new(p).exists());
    let hibernate_service = ["/etc/systemd/system/systemd-hibernate.service.wants/nvidia-hibernate.service", "/usr/lib/systemd/system/systemd-hibernate.service.wants/nvidia-hibernate.service"]
        .iter().any(|p| std::path::Path::new(p).exists())
        || crate::system::run("systemctl", &["is-enabled", "nvidia-hibernate.service"]).map(|o| o.trim() == "enabled").unwrap_or(false);
    Some(NvFacts { preserve, temp_path, temp_free_kb, vram_kb, helper_installed, hibernate_service, kernel_notifiers })
}

pub fn facts() -> HibFacts {
    let rd = |p: &str| std::fs::read_to_string(p).unwrap_or_default();
    let kb = |text: &str, k: &str| text.lines().find(|l| l.starts_with(&format!("{k}:"))).and_then(|l| l.split_whitespace().nth(1)).and_then(|n| n.parse::<u64>().ok()).unwrap_or(0);
    let (disk_swap_kb, hearth_swap_kb) = disk_swap(&rd("/proc/swaps"));
    HibFacts {
        state: rd("/sys/power/state").trim().to_string(),
        disk_modes: rd("/sys/power/disk").trim().to_string(),
        lockdown: rd("/sys/kernel/security/lockdown").trim().to_string(),
        mem_kb: kb(&rd("/proc/meminfo"), "MemTotal"),
        disk_swap_kb, hearth_swap_kb,
        resume_arg: rd("/proc/cmdline").split_whitespace().any(|a| a.starts_with("resume=")),
        logind: crate::system::run("gdbus", &["call", "--system", "--dest", "org.freedesktop.login1", "--object-path", "/org/freedesktop/login1", "--method", "org.freedesktop.login1.Manager.CanHibernate"]).unwrap_or_default().trim().to_string(),
        nvidia: nvidia_facts(),
        can_admin: crate::admin::can_admin(),
    }
}

/// The page's answer for the facts: {supported, canSetUp, restartKeep, reason, code, memoryGB, details}.
/// restartKeep: a restart can keep the games too (the kernel can hibernate-then-restart, and hearth-admin is there
/// to ask for it).
pub fn status_of(f: &HibFacts) -> Value {
    let (supported, can_set_up, code, reason) = verdict(f);
    let restart_keep = supported && f.can_admin && f.disk_modes.split_whitespace().any(|m| m.trim_matches(|c: char| c == '[' || c == ']') == "reboot");
    let nv = f.nvidia.as_ref().map(|n| json!({"preserveVideoMemory": n.preserve, "temporaryFilePath": n.temp_path, "freeKB": n.temp_free_kb, "videoMemoryKB": n.vram_kb,
        "hibernateHelperInstalled": n.helper_installed, "hibernateHelper": n.hibernate_service, "kernelSuspendNotifiers": n.kernel_notifiers}));
    json!({"supported": supported, "canSetUp": can_set_up, "restartKeep": restart_keep, "code": if code.is_empty() { Value::Null } else { json!(code) },
           "reason": if reason.is_empty() { Value::Null } else { json!(reason) }, "memoryGB": (f.mem_kb as f64 / 1048576.0).round(),
           "details": {"powerState": f.state, "hibernateModes": f.disk_modes, "lockdown": f.lockdown, "memoryKB": f.mem_kb, "driveSwapKB": f.disk_swap_kb,
                       "hearthSwapKB": f.hearth_swap_kb, "resumeArgument": f.resume_arg, "canHibernate": f.logind, "nvidia": nv}})
}

/// Can this PC keep Quick Resume games through turning off and restarting (hibernation)? Needs: the kernel allows
/// it (Secure Boot's lockdown doesn't), swap on the drive at least as big as memory (hearth-admin hibernate-setup),
/// the boot options pointing at it, on Nvidia the driver saving video memory, and the system saying yes.
pub fn hibernate_status() -> Value { status_of(&facts()) }

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

    fn ready() -> HibFacts {
        HibFacts { state: "freeze mem disk".into(), disk_modes: "[platform] shutdown reboot suspend test_resume".into(), lockdown: "[none] integrity confidentiality".into(),
                   mem_kb: 16 * 1048576, disk_swap_kb: 17 * 1048576, hearth_swap_kb: 17 * 1048576, resume_arg: true, logind: "('yes',)".into(), nvidia: None, can_admin: true }
    }
    #[test]
    fn keeping_games_ready_and_restart_too() {
        let s = status_of(&ready());
        assert_eq!(s["supported"], true);
        assert_eq!(s["restartKeep"], true);
        assert_eq!(s["reason"], Value::Null);
        assert_eq!(s["details"]["canHibernate"], "('yes',)");
        let mut f = ready(); f.disk_modes = "[platform] shutdown".into();
        assert_eq!(status_of(&f)["restartKeep"], false, "no reboot mode in /sys/power/disk");
        let mut f = ready(); f.can_admin = false;
        assert_eq!(status_of(&f)["restartKeep"], false, "no hearth-admin to ask for it");
    }
    #[test]
    fn says_whats_wrong() {
        let mut f = ready(); f.lockdown = "none [integrity] confidentiality".into();
        let (ok, setup, code, why) = verdict(&f);
        assert!(!ok && !setup); assert_eq!(code, "secure-boot"); assert!(why.contains("Secure Boot") && why.contains("BIOS"));
        let mut f = ready(); f.disk_swap_kb = 8 * 1048576; f.hearth_swap_kb = 0;
        let (ok, setup, code, why) = verdict(&f);
        assert!(!ok && setup); assert_eq!(code, "no-room"); assert!(why.contains("16 GB"), "{why}");
        let mut f = ready(); f.disk_swap_kb = 8 * 1048576; f.hearth_swap_kb = 8 * 1048576;
        assert!(verdict(&f).3.contains("smaller"));
        let mut f = ready(); f.logind = "('na',)".into(); f.resume_arg = false;
        assert_eq!(verdict(&f).2, "restart-needed");
        let mut f = ready(); f.logind = "('na',)".into();
        let (_, _, code, why) = verdict(&f); assert_eq!(code, "system"); assert!(why.contains("\"na\""));
        let mut f = ready(); f.state = "freeze mem".into();
        assert_eq!(verdict(&f).2, "kernel");
        // Without resume= the system can still say yes (systemd finds the swap itself): that's fine.
        let mut f = ready(); f.resume_arg = false; assert!(verdict(&f).0);
    }
    #[test]
    fn nvidia_needs_its_memory_saved_on_the_drive() {
        let good = NvFacts { preserve: Some(true), temp_path: "/var/tmp".into(), temp_free_kb: Some(100 * 1048576), vram_kb: Some(12 * 1048576), helper_installed: true, hibernate_service: true, kernel_notifiers: false };
        let mut f = ready(); f.nvidia = Some(good.clone()); assert!(verdict(&f).0);
        let mut f = ready(); f.nvidia = Some(NvFacts { preserve: Some(false), ..good.clone() }); assert_eq!(verdict(&f).2, "nvidia-memory");
        let mut f = ready(); f.nvidia = Some(NvFacts { temp_path: String::new(), ..good.clone() }); assert_eq!(verdict(&f).2, "nvidia-folder");
        let mut f = ready(); f.nvidia = Some(NvFacts { temp_free_kb: Some(4 * 1048576), ..good.clone() }); assert_eq!(verdict(&f).2, "nvidia-space");
        let mut f = ready(); f.nvidia = Some(NvFacts { hibernate_service: false, ..good.clone() }); assert_eq!(verdict(&f).2, "nvidia-helper");
        let mut f = ready(); f.nvidia = Some(NvFacts { hibernate_service: false, kernel_notifiers: true, ..good.clone() }); assert!(verdict(&f).0);
        // A driver without the helper at all (Anatase's) does it from the kernel's notifications.
        let mut f = ready(); f.nvidia = Some(NvFacts { helper_installed: false, hibernate_service: false, ..good.clone() }); assert!(verdict(&f).0);
        // Can't read the driver's settings: not held against it.
        let mut f = ready(); f.nvidia = Some(NvFacts { preserve: None, temp_free_kb: None, ..good }); assert!(verdict(&f).0);
    }
    #[test]
    fn reads_the_system_files() {
        let (p, t, k) = nv_params("ResmanDebugLevel: 4294967295\nPreserveVideoMemoryAllocations: 1\nTemporaryFilePath: \"/var/tmp\"\nUseKernelSuspendNotifiers: 0\n");
        assert_eq!((p, t.as_str(), k), (Some(true), "/var/tmp", false));
        assert_eq!(nv_params("PreserveVideoMemoryAllocations: 0\nTemporaryFilePath: \"\"\n"), (Some(false), String::new(), false));
        assert_eq!(nv_params(""), (None, String::new(), false));
        let swaps = "Filename\t\t\t\tType\t\tSize\t\tUsed\t\tPriority\n/dev/zram0 partition 8388604 0 100\n/var/swap/hearth-hibernate file 17825788 0 1\n/dev/nvme0n1p3 partition 1048572 0 -2\n";
        assert_eq!(disk_swap(swaps), (17825788 + 1048572, 17825788));
        assert_eq!(df_free_kb("Filesystem 1024-blocks Used Available Capacity Mounted on\n/dev/nvme0n1p3 976762584 400000000 576762584 41% /var\n"), Some(576762584));
        assert_eq!(df_free_kb(""), None);
    }
}
