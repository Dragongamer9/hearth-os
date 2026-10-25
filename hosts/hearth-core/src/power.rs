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
        "sleep" => sleep_now("the power menu").map(|_| false),
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

// ------------------------------------------------------------------ sleep

/// One of logind's inhibitors (who holds off sleep, the power button...).
#[derive(Clone, Debug, PartialEq)]
pub struct Inhibitor { pub what: String, pub who: String, pub why: String, pub mode: String, pub pid: u64 }

/// busctl --json=short's answer to logind's ListInhibitors: {"type":"a(ssssuu)","data":[[[what, who, why, mode, uid, pid], ...]]}.
pub fn parse_inhibitors(json: &str) -> Vec<Inhibitor> {
    let v: Value = serde_json::from_str(json.trim()).unwrap_or(Value::Null);
    let s = |x: &Value| x.as_str().unwrap_or("").to_string();
    v["data"][0].as_array().map(|a| a.iter().filter_map(|i| {
        let i = i.as_array()?;
        Some(Inhibitor { what: s(i.first()?), who: s(i.get(1)?), why: s(i.get(2)?), mode: s(i.get(3)?), pid: i.get(5).and_then(|p| p.as_u64()).unwrap_or(0) })
    }).collect()).unwrap_or_default()
}
/// The ones that stop "sleep" outright (mode block, sleep among what they hold): with one of them, `systemctl
/// suspend` is refused. Delay ones only hold sleep off a few seconds (Steam's, logind's own), so they're fine.
pub fn sleep_blockers(list: &[Inhibitor]) -> Vec<&Inhibitor> {
    list.iter().filter(|i| i.mode == "block" && i.what.split(':').any(|w| w == "sleep")).collect()
}
/// "Hearth (handle-power-key:handle-suspend-key, block); Steam (sleep, delay)", for the log.
pub fn describe_inhibitors(list: &[&Inhibitor]) -> String {
    if list.is_empty() { return "none".into(); }
    list.iter().map(|i| format!("{} ({}, {}{})", i.who, i.what, i.mode, if i.why.is_empty() { String::new() } else { format!(": {}", i.why) })).collect::<Vec<_>>().join("; ")
}
/// logind's inhibitors now (empty when it can't be asked).
pub fn inhibitors() -> Vec<Inhibitor> {
    crate::system::run("busctl", &["--json=short", "call", "org.freedesktop.login1", "/org/freedesktop/login1", "org.freedesktop.login1.Manager", "ListInhibitors"])
        .map(|o| parse_inhibitors(&o)).unwrap_or_default()
}

/// How long after waking up the power button is ignored: the press that woke the device is delivered once it's
/// back, and taken as a new press it put the device straight back to sleep (ROG Xbox Ally). The same as hearth-buttons.
pub const WAKE_GRACE_SECS: f64 = 2.5;
/// Hearth's system-sleep hook (os/system_files/usr/lib/systemd/system-sleep/hearth-resumed) writes /proc/uptime here
/// each time the device wakes up.
const RESUMED: &str = "/run/hearth-resumed";

/// Seconds since the device woke up, from the hook's file and /proc/uptime (both count the time asleep). None when
/// either can't be read or the file is from the future (a clock that went backwards: not trusted).
pub fn since_resume_of(resumed: &str, uptime: &str) -> Option<f64> {
    let first = |s: &str| s.split_whitespace().next().and_then(|w| w.parse::<f64>().ok());
    let (at, now) = (first(resumed)?, first(uptime)?);
    (now >= at).then_some(now - at)
}
/// Some(seconds since waking) when that's within WAKE_GRACE_SECS: a power button press then is the one that woke it.
pub fn just_woke(since: Option<f64>) -> Option<f64> { since.filter(|s| *s >= 0.0 && *s < WAKE_GRACE_SECS) }
fn since_resume() -> Option<f64> {
    since_resume_of(&std::fs::read_to_string(RESUMED).ok()?, &std::fs::read_to_string("/proc/uptime").ok()?)
}
/// Whether a sleep asked for with this reason is the power button's (refused just after waking; the menus aren't).
pub fn from_power_button(why: &str) -> bool { why.starts_with("power button") }
/// One line in the journal as hearth-sleep: Send logs' Sleep section lists who asked for each sleep with the sleeps.
fn journal(msg: &str) {
    let _ = std::process::Command::new("logger").args(["-t", "hearth-sleep", "--", msg])
        .stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).status();
}

/// Sleep, as the power button (or the power menu) asks: real suspend through logind, like SteamOS. A running game
/// is kept as it is (the kernel freezes everything during sleep and it carries on after waking). Who holds sleep
/// off is logged, and a refusal comes back as an error in plain words instead of nothing happening. Who asked goes to
/// the journal (hearth-sleep); a power button press within WAKE_GRACE_SECS of waking up is ignored (and logged).
pub fn sleep_now(why: &str) -> Result<(), String> {
    if from_power_button(why) {
        if let Some(s) = just_woke(since_resume()) {
            let m = format!("Hearth Game Mode: {why}: sleep ignored, the device woke up {s:.1} s ago (the press that woke it)");
            crate::bridge::log(&format!("sleep: {why}: ignored, woke up {s:.1} s ago"));
            journal(&m);
            return Ok(());
        }
    }
    journal(&format!("Hearth Game Mode: {why} → sleep"));
    let all = inhibitors();
    let blockers = sleep_blockers(&all);
    let delays: Vec<&Inhibitor> = all.iter().filter(|i| i.mode == "delay" && i.what.split(':').any(|w| w == "sleep")).collect();
    crate::bridge::log(&format!("sleep: {why}: going to sleep; blocking it: {}; delaying it: {}", describe_inhibitors(&blockers), describe_inhibitors(&delays)));
    match crate::system::run("systemctl", &["suspend"]) {
        Ok(_) => Ok(()),
        Err(e) => {
            crate::bridge::log(&format!("sleep: systemctl suspend refused: {}", e.trim()));
            Err(match blockers.first() {
                Some(b) => format!("Can't sleep right now: {} is keeping the device awake{}.", b.who, if b.why.is_empty() { String::new() } else { format!(" ({})", b.why) }),
                None => "Couldn't go to sleep. Send the logs (Settings › System) so it can be fixed.".into(),
            })
        }
    }
}

/// What the device's sleep looked like at one moment: the kernel's suspend counters (/sys/power/suspend_stats) and
/// the battery. total_hw_sleep is the time the processor spent in its deepest sleep (s0i3 on AMD, from amd_pmc):
/// the part of sleep where the fans stop and the battery barely drops.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SleepSnap { pub at_ms: u64, pub battery: Option<u32>, pub hw_sleep_us: Option<u64>, pub success: Option<u64>, pub fail: Option<u64> }

fn read_num(p: &std::path::Path) -> Option<u64> { std::fs::read_to_string(p).ok()?.trim().parse().ok() }

/// The snapshot, read under `root` ("/" on a real system).
pub fn sleep_snapshot_in(root: &std::path::Path, at_ms: u64) -> SleepSnap {
    let stats = root.join("sys/power/suspend_stats");
    let battery = std::fs::read_dir(root.join("sys/class/power_supply")).ok().and_then(|d| {
        let mut ps: Vec<PathBuf> = d.flatten().map(|e| e.path()).collect();
        ps.sort();
        ps.into_iter().find(|p| std::fs::read_to_string(p.join("type")).map(|t| t.trim() == "Battery").unwrap_or(false))
            .and_then(|p| read_num(&p.join("capacity"))).map(|c| c as u32)
    });
    SleepSnap { at_ms, battery, hw_sleep_us: read_num(&stats.join("total_hw_sleep")), success: read_num(&stats.join("success")), fail: read_num(&stats.join("fail")) }
}
pub fn sleep_snapshot() -> SleepSnap {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);
    sleep_snapshot_in(std::path::Path::new("/"), now)
}

fn span(ms: u64) -> String {
    let s = ms / 1000;
    if s >= 3600 { format!("{} h {} min", s / 3600, (s % 3600) / 60) } else if s >= 60 { format!("{} min {} s", s / 60, s % 60) } else { format!("{s} s") }
}

/// What woke the device: /sys/power/pm_wakeup_irq, named from /proc/interrupts ("IRQ 9 (acpi)").
pub fn wake_source_in(root: &std::path::Path) -> Option<String> {
    let irq = std::fs::read_to_string(root.join("sys/power/pm_wakeup_irq")).ok()?.trim().to_string();
    if irq.is_empty() { return None; }
    let name = std::fs::read_to_string(root.join("proc/interrupts")).ok().and_then(|t| t.lines()
        .find(|l| l.trim_start().strip_prefix(&format!("{irq}:")).is_some())
        .and_then(|l| l.split_whitespace().last().map(str::to_string)));
    Some(match name { Some(n) => format!("IRQ {irq} ({n})"), None => format!("IRQ {irq}") })
}

/// The line logged on waking up: how long it slept, how much of that was real deep sleep, the battery, and what
/// woke it. "Never reached deep sleep" is the case where the screen is off but the fans keep running.
pub fn woke_report(before: &SleepSnap, after: &SleepSnap, wake: Option<&str>) -> String {
    let slept = after.at_ms.saturating_sub(before.at_ms);
    let mut parts = vec![format!("sleep: awake after {}", span(slept))];
    let failed = matches!((before.fail, after.fail), (Some(a), Some(b)) if b > a);
    let entered = matches!((before.success, after.success), (Some(a), Some(b)) if b > a);
    if failed && !entered {
        parts.push("the system refused to sleep (suspend failed: see the Sleep section of the logs)".into());
    } else {
        match (before.hw_sleep_us, after.hw_sleep_us) {
            (Some(a), Some(b)) => {
                let hw = b.saturating_sub(a) / 1000;
                let pct = if slept > 0 { (hw.min(slept) * 100 / slept) as u32 } else { 0 };
                if hw == 0 && slept >= 20_000 {
                    parts.push("never reached deep sleep (s0i3): the processor stayed partly awake, so the fans and the battery kept going".into());
                } else {
                    parts.push(format!("deep sleep (s0i3) {} of it ({pct}%)", span(hw)));
                }
            }
            _ => parts.push("the kernel doesn't say how long it was in deep sleep".into()),
        }
    }
    if let (Some(a), Some(b)) = (before.battery, after.battery) { parts.push(format!("battery {a}% → {b}%")); }
    if let Some(w) = wake { parts.push(format!("woken by {w}")); }
    parts.join("; ")
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

    #[test]
    fn power_button_ignored_just_after_waking() {
        assert_eq!(since_resume_of("1000.50\n", "1001.75 3000.00\n"), Some(1.25));
        assert_eq!(since_resume_of("1000.5", "999.0 1.0"), None);          // from the future: not trusted
        assert_eq!(since_resume_of("", "5.0 1.0"), None);
        assert_eq!(since_resume_of("x", "5.0 1.0"), None);
        assert_eq!(just_woke(Some(0.4)), Some(0.4));
        assert_eq!(just_woke(Some(WAKE_GRACE_SECS)), None);
        assert_eq!(just_woke(Some(690.0)), None);
        assert_eq!(just_woke(None), None);
        assert!(from_power_button("power button"));
        assert!(!from_power_button("the power menu"));
    }
    #[test]
    fn finds_who_blocks_sleep() {
        let j = r#"{"type":"a(ssssuu)","data":[[["handle-power-key:handle-suspend-key","Hearth","Hearth handles the power button","block",1000,2100],["sleep","Steam","Steam wants to know","delay",1000,3300],["idle:sleep","Hearth Player","Playing music","block",1000,4400],["shutdown:sleep","NetworkManager","","delay",0,800]]]}"#;
        let all = parse_inhibitors(j);
        assert_eq!(all.len(), 4);
        assert_eq!(all[0].who, "Hearth");
        assert_eq!(all[2].pid, 4400);
        let b = sleep_blockers(&all);
        assert_eq!(b.len(), 1);
        assert_eq!(b[0].who, "Hearth Player");
        assert_eq!(describe_inhibitors(&b), "Hearth Player (idle:sleep, block: Playing music)");
        assert_eq!(describe_inhibitors(&[]), "none");
        // The power button inhibitor (Hearth's own) never counts as holding off sleep.
        assert!(sleep_blockers(&all[..1]).is_empty());
        assert!(parse_inhibitors("").is_empty());
        assert!(parse_inhibitors("not json").is_empty());
    }

    fn put(root: &std::path::Path, rel: &str, text: &str) {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }

    #[test]
    fn reads_sleep_counters_and_battery() {
        let t = tempfile::tempdir().unwrap();
        put(t.path(), "sys/power/suspend_stats/total_hw_sleep", "5000000\n");
        put(t.path(), "sys/power/suspend_stats/success", "3\n");
        put(t.path(), "sys/power/suspend_stats/fail", "0\n");
        put(t.path(), "sys/class/power_supply/ACAD/type", "Mains\n");
        put(t.path(), "sys/class/power_supply/BAT0/type", "Battery\n");
        put(t.path(), "sys/class/power_supply/BAT0/capacity", "87\n");
        let s = sleep_snapshot_in(t.path(), 42);
        assert_eq!(s, SleepSnap { at_ms: 42, battery: Some(87), hw_sleep_us: Some(5_000_000), success: Some(3), fail: Some(0) });
        // Nothing there (a desktop PC, an older kernel): nothing made up.
        let e = tempfile::tempdir().unwrap();
        assert_eq!(sleep_snapshot_in(e.path(), 1), SleepSnap { at_ms: 1, ..Default::default() });
        put(t.path(), "sys/power/pm_wakeup_irq", "9\n");
        put(t.path(), "proc/interrupts", "           CPU0       CPU1\n  1:          0          0  IR-IO-APIC    1-edge      i8042\n  9:         12          0  IR-IO-APIC    9-fasteoi   acpi\n");
        assert_eq!(wake_source_in(t.path()).as_deref(), Some("IRQ 9 (acpi)"));
        assert_eq!(wake_source_in(e.path()), None);
    }

    #[test]
    fn says_whether_sleep_was_real() {
        let before = SleepSnap { at_ms: 0, battery: Some(90), hw_sleep_us: Some(1_000_000), success: Some(1), fail: Some(0) };
        // 10 minutes asleep, 9 min 30 s of it in s0i3.
        let good = SleepSnap { at_ms: 600_000, battery: Some(89), hw_sleep_us: Some(1_000_000 + 570_000_000), success: Some(2), fail: Some(0) };
        assert_eq!(woke_report(&before, &good, Some("IRQ 9 (acpi)")), "sleep: awake after 10 min 0 s; deep sleep (s0i3) 9 min 30 s of it (95%); battery 90% → 89%; woken by IRQ 9 (acpi)");
        // The screen went off but the processor never got to s0i3: the fans kept running.
        let shallow = SleepSnap { at_ms: 3_600_000, battery: Some(70), hw_sleep_us: Some(1_000_000), success: Some(2), fail: Some(0) };
        let r = woke_report(&before, &shallow, None);
        assert!(r.starts_with("sleep: awake after 1 h 0 min; never reached deep sleep (s0i3)"), "{r}");
        assert!(r.ends_with("battery 90% → 70%"), "{r}");
        // A suspend the kernel gave up on.
        let failed = SleepSnap { at_ms: 3_000, battery: None, hw_sleep_us: Some(1_000_000), success: Some(1), fail: Some(1) };
        assert!(woke_report(&before, &failed, None).contains("the system refused to sleep"));
        // No counters (older kernel, not AMD): said, not guessed.
        let none = SleepSnap { at_ms: 30_000, ..Default::default() };
        assert_eq!(woke_report(&SleepSnap::default(), &none, None), "sleep: awake after 30 s; the kernel doesn't say how long it was in deep sleep");
    }
}
