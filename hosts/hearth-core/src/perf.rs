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

pub fn sample() -> Value {
    let stat = std::fs::read_to_string("/proc/stat").unwrap_or_default();
    let mem = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
    let (used, total) = memory(&mem);
    let (g, v) = gpu(Path::new("/sys/class/drm"));
    json!({"cpu": cpu_percent(&stat), "ramUsed": used, "ramTotal": total, "gpu": g, "vramUsed": v})
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
}
