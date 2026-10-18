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
    (None, None)
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
    use super::*;
    #[test]
    fn numbers() {
        assert_eq!(cpu_times("cpu  100 0 100 700 100 0 0 0\ncpu0 1"), (800, 1000));
        let _ = cpu_percent("cpu  100 0 100 700 100 0 0 0");
        assert_eq!(cpu_percent("cpu  200 0 200 800 100 0 0 0"), 67);
        assert_eq!(memory("MemTotal:       16384000 kB\nMemAvailable:    8192000 kB\n"), (7.8, 15.6));
    }
}
