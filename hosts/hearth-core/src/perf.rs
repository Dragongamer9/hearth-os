//! Live numbers for Quick Access › Performance, from the kernel: CPU (/proc/stat since the last sample),
//! memory (/proc/meminfo), GPU busy and video memory (AMD: /sys/class/drm/card*/device). Port of perf.js.

use serde_json::{json, Value};
use std::path::Path;
use std::sync::Mutex;

static LAST: Mutex<Option<(u64, u64)>> = Mutex::new(None);

pub fn cpu_times(stat: &str) -> (u64, u64) {
    let f: Vec<u64> = stat.lines().next().unwrap_or("").split_whitespace().skip(1).filter_map(|n| n.parse().ok()).collect();
    let idle = f.get(3).copied().unwrap_or(0) + f.get(4).copied().unwrap_or(0);
    (idle, f.iter().sum())
}

pub fn cpu_percent(stat: &str) -> u64 {
    let now = cpu_times(stat);
    let mut last = LAST.lock().unwrap_or_else(|p| p.into_inner());
    let prev = last.replace(now);
    match prev {
        Some((pi, pt)) if now.1 > pt => (100.0 * (1.0 - (now.0.saturating_sub(pi)) as f64 / (now.1 - pt) as f64)).round().max(0.0) as u64,
        _ => 0,
    }
}

pub fn memory(meminfo: &str) -> (f64, f64) {
    let kb = |k: &str| meminfo.lines().find(|l| l.starts_with(&format!("{k}:"))).and_then(|l| l.split_whitespace().nth(1)).and_then(|n| n.parse::<f64>().ok()).unwrap_or(0.0);
    let gb = |n: f64| (n / 1024.0 / 1024.0 * 10.0).round() / 10.0;
    (gb(kb("MemTotal") - kb("MemAvailable")), gb(kb("MemTotal")))
}

pub fn gpu(root: &Path) -> (Option<u64>, Option<f64>) {
    let mut cards: Vec<_> = std::fs::read_dir(root).map(|rd| rd.flatten().map(|e| e.path()).filter(|p| { let n = p.file_name().unwrap().to_string_lossy().to_string(); n.starts_with("card") && n[4..].chars().all(|c| c.is_ascii_digit()) && n.len() > 4 }).collect()).unwrap_or_default();
    cards.sort();
    for c in cards {
        let dev = c.join("device");
        if let Ok(busy) = std::fs::read_to_string(dev.join("gpu_busy_percent")) {
            let vram = std::fs::read_to_string(dev.join("mem_info_vram_used")).ok().and_then(|s| s.trim().parse::<f64>().ok()).map(|b| (b / 1073741824.0 * 10.0).round() / 10.0);
            return (busy.trim().parse().ok(), vram);
        }
    }
    nvidia_smi()
}

/// Nvidia cards don't report their load in /sys: their driver's own tool does (part of the driver). Quick Access
/// asks every second with the bridge held, and nvidia-smi takes from a fraction of a second to its 2 s limit (the
/// card waking up): asked there, every other request waited on it. So it runs in the background, one at a time,
/// and the last answer is what's shown (nothing until the first one is in).
fn nvidia_smi() -> (Option<u64>, Option<f64>) {
    if !Path::new("/sys/module/nvidia").exists() { return (None, None); }
    nvidia_cached(ask_nvidia_smi)
}
fn ask_nvidia_smi() -> (Option<u64>, Option<f64>) {
    let Ok(out) = crate::system::run_timeout("nvidia-smi", &["--query-gpu=utilization.gpu,memory.used", "--format=csv,noheader,nounits"], std::time::Duration::from_secs(2)) else { return (None, None) };
    parse_nvidia_smi(&out)
}
type Gpu = (Option<u64>, Option<f64>);
static NVIDIA: Mutex<Gpu> = Mutex::new((None, None));
static NVIDIA_ASKING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
/// The last answer, with a new one asked for in the background unless one is on its way already. Never waits.
fn nvidia_cached(ask: fn() -> Gpu) -> Gpu {
    use std::sync::atomic::Ordering;
    if !NVIDIA_ASKING.swap(true, Ordering::SeqCst) {
        std::thread::spawn(move || {
            let v = ask();
            *NVIDIA.lock().unwrap_or_else(|p| p.into_inner()) = v;
            NVIDIA_ASKING.store(false, Ordering::SeqCst);
        });
    }
    *NVIDIA.lock().unwrap_or_else(|p| p.into_inner())
}
/// "37, 2048" (percent, MiB) -> (37, 2.0 GB)
pub fn parse_nvidia_smi(out: &str) -> (Option<u64>, Option<f64>) {
    let line = out.lines().next().unwrap_or("");
    let mut it = line.split(',').map(str::trim);
    let busy = it.next().and_then(|s| s.parse::<u64>().ok());
    let vram = it.next().and_then(|s| s.parse::<f64>().ok()).map(|mib| (mib / 1024.0 * 10.0).round() / 10.0);
    (busy, vram)
}

/// A temperature in °C from the first hardware sensor (/sys/class/hwmon) named one of `names` (its temp1_input,
/// in thousandths of a degree). None when there's no such sensor (Quick Access then shows no temperature).
pub fn hwmon_temp(root: &Path, names: &[&str]) -> Option<u64> {
    let mut dirs: Vec<_> = std::fs::read_dir(root).ok()?.flatten().map(|e| e.path()).collect();
    dirs.sort();
    for want in names {
        for d in &dirs {
            if std::fs::read_to_string(d.join("name")).map(|n| n.trim() == *want).unwrap_or(false) {
                if let Some(t) = std::fs::read_to_string(d.join("temp1_input")).ok().and_then(|s| s.trim().parse::<f64>().ok()) {
                    if t > 0.0 { return Some((t / 1000.0).round() as u64); }
                }
            }
        }
    }
    None
}

/// What the device draws from its battery, in watts, while it runs on battery (power_now, or current × voltage);
/// None when charging or there's no battery.
pub fn battery_watts(root: &Path) -> Option<f64> {
    let read = |p: std::path::PathBuf| std::fs::read_to_string(p).map(|s| s.trim().to_string()).unwrap_or_default();
    let mut dirs: Vec<_> = std::fs::read_dir(root).ok()?.flatten().map(|e| e.path()).collect();
    dirs.sort();
    for d in dirs {
        if read(d.join("type")) != "Battery" || read(d.join("scope")) == "Device" { continue; }
        if read(d.join("status")) != "Discharging" { return None; }
        let num = |f: &str| read(d.join(f)).parse::<f64>().ok().filter(|v| *v > 0.0);
        let uw = num("power_now").or_else(|| Some(num("current_now")? * num("voltage_now")? / 1e6))?;
        return Some((uw / 1e6 * 10.0).round() / 10.0);
    }
    None
}

/// A connected controller's battery (the Dragonfire look's performance card): power_supply entries with scope
/// "Device" are batteries in things plugged in or paired (controllers, mice, headsets); only the ones that name a
/// controller count, so a wireless mouse's battery isn't shown as the controller's. The lowest one when several
/// controllers are connected, since that's the one about to run out. None without one.
pub fn pad_battery(root: &Path) -> Option<Value> {
    let read = |p: std::path::PathBuf| std::fs::read_to_string(p).map(|s| s.trim().to_string()).unwrap_or_default();
    let mut best: Option<(u64, bool)> = None;
    for d in std::fs::read_dir(root).ok()?.flatten().map(|e| e.path()) {
        if read(d.join("type")) != "Battery" || read(d.join("scope")) != "Device" { continue; }
        let name = format!("{} {}", d.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default(), read(d.join("model_name")).to_lowercase());
        if !["controller", "gamepad", "joy-con", "xpadneo", "dualsense", "dualshock"].iter().any(|k| name.contains(k)) { continue; }
        let Ok(pct) = read(d.join("capacity")).parse::<u64>() else { continue };
        let charging = read(d.join("status")) == "Charging";
        if best.map(|(p, _)| pct < p).unwrap_or(true) { best = Some((pct.min(100), charging)); }
    }
    best.map(|(pct, charging)| json!({"pct": pct, "charging": charging}))
}

pub fn sample() -> Value {
    let stat = std::fs::read_to_string("/proc/stat").unwrap_or_default();
    let mem = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
    let (used, total) = memory(&mem);
    let (g, v) = gpu(Path::new("/sys/class/drm"));
    let hw = Path::new("/sys/class/hwmon");
    json!({"cpu": cpu_percent(&stat), "ramUsed": used, "ramTotal": total, "gpu": g, "vramUsed": v,
           "cpuTemp": hwmon_temp(hw, &["k10temp", "zenpower", "coretemp", "cpu_thermal"]), "gpuTemp": hwmon_temp(hw, &["amdgpu", "xe"]),
           "powerW": battery_watts(Path::new("/sys/class/power_supply")), "padBattery": pad_battery(Path::new("/sys/class/power_supply"))})
}

#[cfg(test)]
mod tests {
    #[test]
    fn reads_nvidia_smi() { assert_eq!(super::parse_nvidia_smi("37, 2048\n"), (Some(37), Some(2.0))); assert_eq!(super::parse_nvidia_smi(""), (None, None)); }
    fn slow_smi() -> super::Gpu { std::thread::sleep(std::time::Duration::from_millis(400)); (Some(50), Some(1.0)) }
    #[test]
    fn nvidia_numbers_never_hold_up_a_request() {
        let t = std::time::Instant::now();
        assert_eq!(super::nvidia_cached(slow_smi), (None, None));
        assert_eq!(super::nvidia_cached(slow_smi), (None, None));
        assert!(t.elapsed() < std::time::Duration::from_millis(200), "waited for nvidia-smi");
        let mut got = (None, None);
        for _ in 0..50 { got = super::nvidia_cached(slow_smi); if got.0.is_some() { break; } std::thread::sleep(std::time::Duration::from_millis(50)); }
        assert_eq!(got, (Some(50), Some(1.0)));
    }
    use super::*;
    #[test]
    fn numbers() {
        assert_eq!(cpu_times("cpu  100 0 100 700 100 0 0 0\ncpu0 1"), (800, 1000));
        let _ = cpu_percent("cpu  100 0 100 700 100 0 0 0");
        assert_eq!(cpu_percent("cpu  200 0 200 800 100 0 0 0"), 67);
        assert_eq!(memory("MemTotal:       16384000 kB\nMemAvailable:    8192000 kB\n"), (7.8, 15.6));
    }
    #[test]
    fn temperatures_and_battery_power() {
        let dir = tempfile::tempdir().unwrap();
        let hw = dir.path().join("hwmon");
        for (n, name, t) in [("hwmon0", "acpitz", "40000"), ("hwmon1", "amdgpu", "61000"), ("hwmon2", "k10temp", "67600")] {
            std::fs::create_dir_all(hw.join(n)).unwrap();
            std::fs::write(hw.join(n).join("name"), format!("{name}\n")).unwrap();
            std::fs::write(hw.join(n).join("temp1_input"), t).unwrap();
        }
        assert_eq!(hwmon_temp(&hw, &["k10temp", "coretemp"]), Some(68));
        assert_eq!(hwmon_temp(&hw, &["amdgpu"]), Some(61));
        assert_eq!(hwmon_temp(&hw, &["coretemp"]), None);
        let ps = dir.path().join("ps");
        std::fs::create_dir_all(ps.join("BAT0")).unwrap();
        for (f, v) in [("type", "Battery"), ("status", "Discharging"), ("current_now", "1500000"), ("voltage_now", "12000000")] { std::fs::write(ps.join("BAT0").join(f), v).unwrap(); }
        assert_eq!(battery_watts(&ps), Some(18.0));
        std::fs::write(ps.join("BAT0").join("power_now"), "9100000").unwrap();
        assert_eq!(battery_watts(&ps), Some(9.1));
        std::fs::write(ps.join("BAT0").join("status"), "Charging").unwrap();
        assert_eq!(battery_watts(&ps), None);
    }
    #[test]
    fn controller_battery() {
        let dir = tempfile::tempdir().unwrap();
        let ps = dir.path();
        let put = |n: &str, f: &str, v: &str| { std::fs::create_dir_all(ps.join(n)).unwrap(); std::fs::write(ps.join(n).join(f), v).unwrap(); };
        assert_eq!(pad_battery(ps), None);
        // The device's own battery and a mouse's don't count.
        put("BAT0", "type", "Battery"); put("BAT0", "capacity", "50");
        put("hidpp_battery_0", "type", "Battery"); put("hidpp_battery_0", "scope", "Device"); put("hidpp_battery_0", "capacity", "10");
        assert_eq!(pad_battery(ps), None);
        put("ps-controller-battery-aa", "type", "Battery"); put("ps-controller-battery-aa", "scope", "Device");
        put("ps-controller-battery-aa", "capacity", "87\n"); put("ps-controller-battery-aa", "status", "Discharging");
        assert_eq!(pad_battery(ps), Some(json!({"pct": 87, "charging": false})));
        // Named only by its model (xpadneo's Xbox controllers): the lower of two.
        put("gip0", "type", "Battery"); put("gip0", "scope", "Device"); put("gip0", "model_name", "Xbox Wireless Controller");
        put("gip0", "capacity", "40"); put("gip0", "status", "Charging");
        assert_eq!(pad_battery(ps), Some(json!({"pct": 40, "charging": true})));
    }
}
