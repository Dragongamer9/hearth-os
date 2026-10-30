//! Hearth Benchmark's live readings, straight from the kernel (/sys, /proc), cheap enough to read every second:
//! processor clocks and busy time, processor and graphics temperatures (and their critical limits), graphics clock
//! and power, and the battery. Nvidia cards report through their driver's nvidia-smi instead (asked less often).
//! Every reader takes its folder so the tests can point it at a made-up one.

use serde_json::{json, Value};
use std::path::{Path, PathBuf};

fn read(p: impl AsRef<Path>) -> Option<String> { std::fs::read_to_string(p).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()) }
fn num(p: impl AsRef<Path>) -> Option<f64> { read(p)?.parse::<f64>().ok() }
fn dirs(root: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(root).map(|rd| rd.flatten().map(|e| e.path()).collect()).unwrap_or_default();
    v.sort();
    v
}

/// Processor clock in MHz: (average over the cores, the fastest core), from cpufreq.
pub fn cpu_mhz(root: &Path) -> Option<(f64, f64)> {
    let f: Vec<f64> = dirs(root).iter()
        .filter(|d| d.file_name().and_then(|n| n.to_str()).map(|n| n.starts_with("cpu") && n[3..].chars().all(|c| c.is_ascii_digit()) && n.len() > 3).unwrap_or(false))
        .filter_map(|d| num(d.join("cpufreq/scaling_cur_freq"))).map(|khz| khz / 1000.0).collect();
    if f.is_empty() { return None; }
    Some(((f.iter().sum::<f64>() / f.len() as f64).round(), f.iter().cloned().fold(0.0, f64::max).round()))
}

/// /proc/stat's first line: (idle, total) jiffies.
pub fn cpu_times(stat: &str) -> (u64, u64) {
    let f: Vec<u64> = stat.lines().next().unwrap_or("").split_whitespace().skip(1).filter_map(|n| n.parse().ok()).collect();
    (f.get(3).copied().unwrap_or(0) + f.get(4).copied().unwrap_or(0), f.iter().sum())
}

/// One hardware sensor chip (/sys/class/hwmon/hwmonN).
#[derive(Debug, Clone, PartialEq)]
pub struct Chip { pub name: String, pub temp: Option<f64>, pub crit: Option<f64>, pub watts: Option<f64>, pub mhz: Option<f64> }

pub fn chips(root: &Path) -> Vec<Chip> {
    dirs(root).into_iter().filter_map(|d| {
        let name = read(d.join("name"))?;
        let c = |f: &str| num(d.join(f));
        Some(Chip {
            name,
            temp: c("temp1_input").map(|t| (t / 100.0).round() / 10.0).filter(|t| *t > 0.0 && *t < 150.0),
            crit: c("temp1_crit").map(|t| t / 1000.0).filter(|t| *t > 0.0 && *t < 200.0),
            watts: c("power1_average").or_else(|| c("power1_input")).map(|uw| (uw / 1e5).round() / 10.0),
            mhz: c("freq1_input").map(|hz| (hz / 1e6).round()),
        })
    }).collect()
}

pub const CPU_CHIPS: [&str; 6] = ["k10temp", "zenpower", "coretemp", "cpu_thermal", "x86_pkg_temp", "acpitz"];
pub const GPU_CHIPS: [&str; 4] = ["amdgpu", "i915", "xe", "nouveau"];

/// The first chip with one of `names` (in that order of preference) that has a temperature.
pub fn pick<'a>(chips: &'a [Chip], names: &[&str]) -> Option<&'a Chip> {
    names.iter().find_map(|n| chips.iter().find(|c| c.name == *n && c.temp.is_some()))
}

/// The device's own battery: {percent, onBattery, watts (drawn, while on battery), wh (left)}. Null without one.
pub fn battery(root: &Path) -> Value {
    for d in dirs(root) {
        if read(d.join("type")).as_deref() != Some("Battery") || read(d.join("scope")).as_deref() == Some("Device") { continue; }
        let n = |f: &str| num(d.join(f)).filter(|v| *v > 0.0);
        let status = read(d.join("status")).unwrap_or_default();
        let on = status == "Discharging";
        let volts = n("voltage_now").map(|v| v / 1e6);
        let watts = n("power_now").map(|p| p / 1e6).or_else(|| Some(n("current_now")? / 1e6 * volts?));
        let wh = n("energy_now").map(|e| e / 1e6).or_else(|| Some(n("charge_now")? / 1e6 * volts?));
        let full = n("energy_full").map(|e| e / 1e6).or_else(|| Some(n("charge_full")? / 1e6 * volts?));
        let r1 = |x: Option<f64>| x.map(|v| (v * 10.0).round() / 10.0);
        return json!({"percent": n("capacity").map(|c| c.min(100.0)), "onBattery": on, "status": status,
                      "watts": if on { r1(watts) } else { None }, "wh": r1(wh), "fullWh": r1(full)});
    }
    Value::Null
}

/// One reading of everything, for the page and the safety check. `nvidia` is the last nvidia-smi answer
/// (temperature, clock, watts).
pub fn reading(sys: &Path, busy: Option<f64>, nvidia: (Option<f64>, Option<f64>, Option<f64>)) -> Value {
    let ch = chips(&sys.join("class/hwmon"));
    let cpu = pick(&ch, &CPU_CHIPS);
    let gpu = pick(&ch, &GPU_CHIPS);
    let (avg, max) = cpu_mhz(&sys.join("devices/system/cpu")).map(|(a, m)| (Some(a), Some(m))).unwrap_or((None, None));
    let b = battery(&sys.join("class/power_supply"));
    let gpu_temp = nvidia.0.or(gpu.and_then(|g| g.temp));
    json!({
        "cpuMhz": avg, "cpuMaxMhz": max, "cpuPct": busy,
        "cpuTemp": cpu.and_then(|c| c.temp), "cpuCrit": cpu.and_then(|c| c.crit),
        "gpuTemp": gpu_temp, "gpuCrit": if nvidia.0.is_some() { None } else { gpu.and_then(|g| g.crit) },
        "gpuMhz": nvidia.1.or(gpu.and_then(|g| g.mhz)), "gpuW": nvidia.2.or(gpu.and_then(|g| g.watts)),
        "battW": b["watts"], "battPct": b["percent"], "onBattery": b["onBattery"] == true,
    })
}

/// Too hot? (which part, its temperature, the limit) when the processor or graphics is at or over its limit.
pub fn too_hot(r: &Value) -> Option<(&'static str, f64, f64)> {
    for (part, t, c) in [("processor", "cpuTemp", "cpuCrit"), ("graphics chip", "gpuTemp", "gpuCrit")] {
        if let Some(temp) = r[t].as_f64() {
            let limit = super::parse::heat_limit(r[c].as_f64());
            if temp >= limit { return Some((part, temp, limit)); }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake() -> PathBuf {
        let root = std::env::temp_dir().join(format!("hearth-bench-sensors-{}-{:?}", std::process::id(), std::thread::current().id()));
        let _ = std::fs::remove_dir_all(&root);
        let w = |p: &str, s: &str| { let f = root.join(p); std::fs::create_dir_all(f.parent().unwrap()).unwrap(); std::fs::write(f, s).unwrap(); };
        w("class/hwmon/hwmon0/name", "acpitz\n");
        w("class/hwmon/hwmon0/temp1_input", "40000\n");
        w("class/hwmon/hwmon1/name", "k10temp\n");
        w("class/hwmon/hwmon1/temp1_input", "71250\n");
        w("class/hwmon/hwmon2/name", "amdgpu\n");
        w("class/hwmon/hwmon2/temp1_input", "64000\n");
        w("class/hwmon/hwmon2/temp1_crit", "100000\n");
        w("class/hwmon/hwmon2/power1_average", "15300000\n");
        w("class/hwmon/hwmon2/freq1_input", "2200000000\n");
        w("devices/system/cpu/cpu0/cpufreq/scaling_cur_freq", "3000000\n");
        w("devices/system/cpu/cpu1/cpufreq/scaling_cur_freq", "4000000\n");
        w("devices/system/cpu/cpufreq/policy0/scaling_cur_freq", "9999999\n");
        w("class/power_supply/ACAD/type", "Mains\n");
        w("class/power_supply/BAT0/type", "Battery\n");
        w("class/power_supply/BAT0/status", "Discharging\n");
        w("class/power_supply/BAT0/capacity", "81\n");
        w("class/power_supply/BAT0/voltage_now", "15000000\n");
        w("class/power_supply/BAT0/current_now", "1000000\n");
        w("class/power_supply/BAT0/charge_now", "2000000\n");
        root
    }

    #[test]
    fn reads_the_sensors() {
        let root = fake();
        assert_eq!(cpu_mhz(&root.join("devices/system/cpu")), Some((3500.0, 4000.0)));
        let ch = chips(&root.join("class/hwmon"));
        assert_eq!(pick(&ch, &CPU_CHIPS).unwrap().name, "k10temp"); // k10temp before acpitz
        let g = pick(&ch, &GPU_CHIPS).unwrap();
        assert_eq!((g.temp, g.crit, g.watts, g.mhz), (Some(64.0), Some(100.0), Some(15.3), Some(2200.0)));
        let b = battery(&root.join("class/power_supply"));
        assert_eq!(b["percent"], json!(81.0));
        assert_eq!(b["watts"], json!(15.0));
        assert_eq!(b["wh"], json!(30.0));
        let r = reading(&root, Some(50.0), (None, None, None));
        assert_eq!(r["cpuTemp"], json!(71.3));
        assert_eq!(r["gpuW"], json!(15.3));
        assert_eq!(r["onBattery"], json!(true));
        assert!(too_hot(&r).is_none());
        // Nvidia's answer wins for the graphics numbers.
        let r = reading(&root, None, (Some(97.0), Some(1800.0), Some(120.0)));
        assert_eq!(r["gpuMhz"], json!(1800.0));
        assert_eq!(too_hot(&r).map(|x| x.0), Some("graphics chip"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn heat_check() {
        assert_eq!(too_hot(&json!({"cpuTemp": 96.0})), Some(("processor", 96.0, 95.0)));
        assert_eq!(too_hot(&json!({"cpuTemp": 86.0, "cpuCrit": 88.0})), Some(("processor", 86.0, 85.0)));
        assert!(too_hot(&json!({"cpuTemp": 80.0, "gpuTemp": null})).is_none());
        assert_eq!(cpu_times("cpu  10 0 10 70 10 0 0 0 0 0\ncpu0 1"), (80, 100));
    }
}
