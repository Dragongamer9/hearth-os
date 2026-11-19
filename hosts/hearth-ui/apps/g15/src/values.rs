//! What a screen can show, read cheaply from the kernel's files (no program runs once a second): the CPU's use
//! (/proc/stat) and heat (hwmon: k10temp, zenpower, coretemp), the graphics card's use (gpu_busy_percent; nvidia-smi
//! at most every 5 s on Nvidia) and heat, memory, the drive, the network, fans, a controller's battery, the time, the
//! weather your Weather widget last fetched (~/.cache/hearth/widgets: nothing is fetched from here), the song playing
//! (MPRIS through playerctl, service.rs). Only what the screen on show uses is read.

use crate::root::{read, read_u32, Root};
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};

/// Every value a screen can show: (id, what the app calls it, its unit).
pub const VALUES: &[(&str, &str, &str)] = &[
    ("cpu.use", "CPU use", "%"),
    ("cpu.temp", "CPU temperature", "°"),
    ("gpu.use", "Graphics card use", "%"),
    ("gpu.temp", "Graphics card temperature", "°"),
    ("mem.use", "Memory use", "%"),
    ("mem.used", "Memory used", "GB"),
    ("disk.use", "Drive use", "%"),
    ("disk.free", "Drive free", "GB"),
    ("net.down", "Download speed", "KB/s"),
    ("net.up", "Upload speed", "KB/s"),
    ("fan", "Fan speed", "rpm"),
    ("fps", "Frames per second", "fps"),
    ("weather.temp", "Weather", "°"),
    ("pad.battery", "Controller battery", "%"),
    ("song.pos", "Song played", "%"),
];

#[derive(Debug, Clone, Default)]
pub struct Reading {
    pub nums: HashMap<String, f64>,
    pub texts: HashMap<String, String>,
}

impl Reading {
    pub fn num(&self, k: &str) -> Option<f64> {
        self.nums.get(k).copied()
    }
    pub fn text(&self, k: &str) -> String {
        self.texts.get(k).cloned().unwrap_or_default()
    }
}

#[derive(Default)]
pub struct Sampler {
    cpu_prev: Option<(u64, u64)>,
    net_prev: Option<(u64, u64, std::time::Instant)>,
    nvidia_at: Option<std::time::Instant>,
    nvidia: Option<(f64, f64)>,
    weather_at: Option<std::time::Instant>,
    weather: Option<(f64, String, String)>,
    names: Option<(String, String)>,
    /// Graph history: value → (second, value), newest last, at most an hour.
    pub history: HashMap<String, VecDeque<(u64, f64)>>,
}

fn first_line_nums(s: &str) -> Vec<u64> {
    s.split_whitespace().skip(1).filter_map(|x| x.parse().ok()).collect()
}

/// /proc/stat's first line → (busy, total).
pub fn cpu_counters(stat: &str) -> Option<(u64, u64)> {
    let n = first_line_nums(stat.lines().next()?);
    if n.len() < 4 {
        return None;
    }
    let idle = n[3] + n.get(4).copied().unwrap_or(0);
    let total: u64 = n.iter().take(8).sum();
    Some((total - idle, total))
}

fn hwmons(sys: &Path) -> Vec<(String, PathBuf)> {
    let mut v: Vec<(String, PathBuf)> = std::fs::read_dir(sys.join("class/hwmon"))
        .map(|r| r.filter_map(|e| e.ok()).map(|e| (read(&e.path().join("name")).unwrap_or_default(), e.path())).collect())
        .unwrap_or_default();
    v.sort_by(|a, b| a.1.cmp(&b.1));
    v
}

fn temp_of(dir: &Path, prefer: &[&str]) -> Option<f64> {
    let mut best: Option<f64> = None;
    for i in 1..=16 {
        let Some(v) = read_u32(&dir.join(format!("temp{i}_input"))) else { continue };
        let c = v as f64 / 1000.0;
        if !(1.0..150.0).contains(&c) {
            continue;
        }
        let label = read(&dir.join(format!("temp{i}_label"))).unwrap_or_default().to_lowercase();
        if prefer.iter().any(|p| label == *p) {
            return Some(c);
        }
        if best.is_none() {
            best = Some(c);
        }
    }
    best
}

/// "AMD Ryzen 7 5700 8-Core Processor" → "R7 5700"; "12th Gen Intel(R) Core(TM) i7-12700K" → "i7-12700K".
pub fn short_cpu(name: &str) -> String {
    let n = name.replace("(R)", "").replace("(TM)", "");
    let w: Vec<&str> = n.split_whitespace().collect();
    if let Some(i) = w.iter().position(|x| *x == "Ryzen") {
        let tier = w.get(i + 1).copied().unwrap_or("");
        let model = w.get(i + 2).copied().unwrap_or("");
        if tier.chars().all(|c| c.is_ascii_digit()) && !model.is_empty() {
            return format!("R{tier} {model}");
        }
    }
    if let Some(x) = w.iter().find(|x| x.starts_with("i3-") || x.starts_with("i5-") || x.starts_with("i7-") || x.starts_with("i9-")) {
        return x.to_string();
    }
    w.iter().filter(|x| !matches!(**x, "AMD" | "Intel" | "Processor" | "CPU")).take(3).cloned().collect::<Vec<_>>().join(" ")
}

/// The graphics card's marketing name from pci.ids' line, best effort: "[Radeon RX 6700/6700 XT/6750 XT]" → the
/// first one with the RX/GTX/RTX/Arc word.
pub fn short_gpu(name: &str) -> String {
    let inner = name.split_once('[').map(|(_, r)| r.trim_end_matches(']')).unwrap_or(name);
    let first = inner.split('/').next().unwrap_or(inner).trim();
    first.trim_start_matches("Radeon ").trim_start_matches("GeForce ").to_string()
}

fn pci_name(vendor: &str, device: &str) -> Option<String> {
    let v = vendor.trim_start_matches("0x").to_lowercase();
    let d = device.trim_start_matches("0x").to_lowercase();
    for f in ["/usr/share/hwdata/pci.ids", "/usr/share/misc/pci.ids", "/usr/share/pci.ids"] {
        let Ok(text) = std::fs::read_to_string(f) else { continue };
        let mut in_vendor = false;
        for l in text.lines() {
            if !l.starts_with('\t') && !l.starts_with('#') {
                in_vendor = l.starts_with(&v);
            } else if in_vendor && l.starts_with('\t') && !l.starts_with("\t\t") && l[1..].starts_with(&d) {
                return Some(l[1..].get(4..).unwrap_or("").trim().to_string());
            }
        }
    }
    None
}

impl Sampler {
    pub fn names(&mut self, root: &Root) -> (String, String) {
        if let Some(n) = &self.names {
            return n.clone();
        }
        let cpu = std::fs::read_to_string(root.proc_.join("cpuinfo"))
            .ok()
            .and_then(|t| t.lines().find(|l| l.starts_with("model name")).and_then(|l| l.split_once(':')).map(|(_, v)| short_cpu(v.trim())))
            .unwrap_or_default();
        let mut gpu = String::new();
        for i in 0..4 {
            let dev = root.sys.join(format!("class/drm/card{i}/device"));
            let (Some(v), Some(d)) = (read(&dev.join("vendor")), read(&dev.join("device"))) else { continue };
            if let Some(n) = read(&dev.join("product_name")).filter(|s| !s.is_empty()) {
                gpu = n;
                break;
            }
            if let Some(n) = pci_name(&v, &d) {
                gpu = short_gpu(&n);
                break;
            }
        }
        let n = (cpu, gpu);
        self.names = Some(n.clone());
        n
    }

    /// Reads the values in `wanted` (ids from VALUES, plus "time", "names", "weather").
    pub fn sample(&mut self, root: &Root, wanted: &[String]) -> Reading {
        let mut r = Reading::default();
        let want = |k: &str| wanted.iter().any(|w| w == k || w.starts_with(&format!("{k}.")) || k.starts_with(&format!("{w}.")));
        // A pretend PC (the fixture): its readings as they are.
        if let Some(f) = root.fixture_file("stats.json") {
            if let Ok(Value::Object(m)) = std::fs::read_to_string(&f).map(|t| serde_json::from_str(&t).unwrap_or(Value::Null)) {
                let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
                for (k, v) in m {
                    // Pretend history for the graphs: the values over the last few minutes, oldest first.
                    if k == "history" {
                        for (hk, hv) in v.as_object().cloned().unwrap_or_default() {
                            let vals: Vec<f64> = hv.as_array().map(|a| a.iter().filter_map(|x| x.as_f64()).collect()).unwrap_or_default();
                            let h = self.history.entry(hk).or_default();
                            if h.is_empty() {
                                let n = vals.len() as u64;
                                for (i, x) in vals.iter().enumerate() {
                                    h.push_back((now.saturating_sub((n - 1 - i as u64) * 18 + 1), *x));
                                }
                            }
                        }
                        continue;
                    }
                    if let Some(n) = v.as_f64() {
                        r.nums.insert(k, n);
                    } else if let Some(s) = v.as_str() {
                        r.texts.insert(k, s.to_string());
                    }
                }
            }
            self.remember(&r);
            return r;
        }
        if want("cpu.use") {
            if let Some((busy, total)) = std::fs::read_to_string(root.proc_.join("stat")).ok().and_then(|s| cpu_counters(&s)) {
                if let Some((pb, pt)) = self.cpu_prev {
                    if total > pt {
                        r.nums.insert("cpu.use".into(), 100.0 * (busy.saturating_sub(pb)) as f64 / (total - pt) as f64);
                    }
                }
                self.cpu_prev = Some((busy, total));
            }
        }
        let mons = if want("cpu.temp") || want("gpu.temp") || want("fan") { hwmons(&root.sys) } else { Vec::new() };
        if want("cpu.temp") {
            for (n, d) in &mons {
                if matches!(n.as_str(), "k10temp" | "zenpower" | "coretemp" | "cpu_thermal" | "soc_thermal") {
                    if let Some(c) = temp_of(d, &["tctl", "tdie", "package id 0"]) {
                        r.nums.insert("cpu.temp".into(), c);
                        break;
                    }
                }
            }
        }
        if want("gpu.use") || want("gpu.temp") {
            let mut found = false;
            for i in 0..4 {
                let dev = root.sys.join(format!("class/drm/card{i}/device"));
                if let Some(b) = read_u32(&dev.join("gpu_busy_percent")) {
                    r.nums.insert("gpu.use".into(), b as f64);
                    found = true;
                    if let Ok(rd) = std::fs::read_dir(dev.join("hwmon")) {
                        if let Some(h) = rd.filter_map(|e| e.ok()).next() {
                            if let Some(c) = temp_of(&h.path(), &["edge"]) {
                                r.nums.insert("gpu.temp".into(), c);
                            }
                        }
                    }
                    break;
                }
            }
            if !found && crate::which("nvidia-smi") {
                let stale = self.nvidia_at.map(|t| t.elapsed().as_secs() >= 5).unwrap_or(true);
                if stale {
                    self.nvidia_at = Some(std::time::Instant::now());
                    self.nvidia = crate::run_out(&["nvidia-smi", "--query-gpu=utilization.gpu,temperature.gpu", "--format=csv,noheader,nounits"], 3).and_then(|o| {
                        let f: Vec<f64> = o.lines().next()?.split(',').filter_map(|x| x.trim().parse().ok()).collect();
                        (f.len() >= 2).then(|| (f[0], f[1]))
                    });
                }
                if let Some((u, t)) = self.nvidia {
                    r.nums.insert("gpu.use".into(), u);
                    r.nums.insert("gpu.temp".into(), t);
                }
            }
            if !r.nums.contains_key("gpu.temp") {
                for (n, d) in &mons {
                    if matches!(n.as_str(), "amdgpu" | "radeon" | "nouveau" | "i915" | "xe") {
                        if let Some(c) = temp_of(d, &["edge"]) {
                            r.nums.insert("gpu.temp".into(), c);
                            break;
                        }
                    }
                }
            }
        }
        if want("mem") {
            if let Ok(t) = std::fs::read_to_string(root.proc_.join("meminfo")) {
                let kb = |k: &str| t.lines().find(|l| l.starts_with(k)).and_then(|l| l.split_whitespace().nth(1)).and_then(|x| x.parse::<f64>().ok());
                if let (Some(total), Some(avail)) = (kb("MemTotal:"), kb("MemAvailable:")) {
                    r.nums.insert("mem.used".into(), (total - avail) / 1048576.0);
                    r.nums.insert("mem.total".into(), total / 1048576.0);
                    r.nums.insert("mem.use".into(), 100.0 * (total - avail) / total.max(1.0));
                }
            }
        }
        if want("disk") {
            let home = crate::root::home();
            if let Ok(c) = std::ffi::CString::new(home.as_os_str().as_encoded_bytes()) {
                let mut s: libc::statvfs = unsafe { std::mem::zeroed() };
                if unsafe { libc::statvfs(c.as_ptr(), &mut s) } == 0 && s.f_blocks > 0 {
                    let (total, free) = (s.f_blocks as f64 * s.f_frsize as f64, s.f_bavail as f64 * s.f_frsize as f64);
                    r.nums.insert("disk.use".into(), 100.0 * (1.0 - free / total));
                    r.nums.insert("disk.free".into(), free / 1e9);
                }
            }
        }
        if want("net") {
            if let Ok(t) = std::fs::read_to_string(root.proc_.join("net/dev")) {
                let (mut rx, mut tx) = (0u64, 0u64);
                for l in t.lines().skip(2) {
                    let Some((name, rest)) = l.split_once(':') else { continue };
                    if name.trim() == "lo" {
                        continue;
                    }
                    let f: Vec<u64> = rest.split_whitespace().filter_map(|x| x.parse().ok()).collect();
                    if f.len() >= 9 {
                        rx += f[0];
                        tx += f[8];
                    }
                }
                let now = std::time::Instant::now();
                if let Some((prx, ptx, at)) = self.net_prev {
                    let s = now.duration_since(at).as_secs_f64().max(0.2);
                    r.nums.insert("net.down".into(), rx.saturating_sub(prx) as f64 / 1024.0 / s);
                    r.nums.insert("net.up".into(), tx.saturating_sub(ptx) as f64 / 1024.0 / s);
                }
                self.net_prev = Some((rx, tx, now));
            }
        }
        if want("fan") {
            'f: for (_, d) in &mons {
                for i in 1..=8 {
                    if let Some(v) = read_u32(&d.join(format!("fan{i}_input"))).filter(|&v| v > 0) {
                        r.nums.insert("fan".into(), v as f64);
                        break 'f;
                    }
                }
            }
        }
        if want("pad.battery") {
            if let Ok(rd) = std::fs::read_dir(root.sys.join("class/power_supply")) {
                for e in rd.filter_map(|e| e.ok()) {
                    if read(&e.path().join("scope")).as_deref() == Some("Device") {
                        if let Some(c) = read_u32(&e.path().join("capacity")) {
                            r.nums.insert("pad.battery".into(), c as f64);
                            break;
                        }
                    }
                }
            }
        }
        if want("weather") {
            self.read_weather();
            if let Some((t, unit, sky)) = &self.weather {
                r.nums.insert("weather.temp".into(), *t);
                r.texts.insert("weather.unit".into(), unit.clone());
                r.texts.insert("weather.sky".into(), sky.clone());
            }
        }
        if want("names") {
            let (c, g) = self.names(root);
            r.texts.insert("cpu.name".into(), c);
            r.texts.insert("gpu.name".into(), g);
        }
        self.remember(&r);
        r
    }

    /// The newest open-meteo answer the Weather widget kept (at most every 10 minutes, and only one 3 h old or newer).
    fn read_weather(&mut self) {
        if self.weather_at.map(|t| t.elapsed().as_secs() < 600).unwrap_or(false) {
            return;
        }
        self.weather_at = Some(std::time::Instant::now());
        let dir = std::env::var_os("XDG_CACHE_HOME").map(PathBuf::from).unwrap_or_else(|| crate::root::home().join(".cache")).join("hearth/widgets");
        let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
        if let Ok(rd) = std::fs::read_dir(&dir) {
            for e in rd.filter_map(|e| e.ok()) {
                let Ok(md) = e.metadata() else { continue };
                if !md.is_file() || md.len() > 200_000 {
                    continue;
                }
                let Ok(m) = md.modified() else { continue };
                if m.elapsed().map(|d| d.as_secs() > 3 * 3600).unwrap_or(true) {
                    continue;
                }
                if best.as_ref().map(|b| m > b.0).unwrap_or(true) {
                    if std::fs::read_to_string(e.path()).map(|t| t.contains("\"temperature_2m\"") && t.contains("\"current\"")).unwrap_or(false) {
                        best = Some((m, e.path()));
                    }
                }
            }
        }
        self.weather = best.and_then(|(_, p)| {
            let v: Value = serde_json::from_str(&std::fs::read_to_string(p).ok()?).ok()?;
            let t = v.get("current")?.get("temperature_2m")?.as_f64()?;
            let unit = v.get("current_units").and_then(|u| u.get("temperature_2m")).and_then(|u| u.as_str()).unwrap_or("°C").to_string();
            let code = v.get("current").and_then(|c| c.get("weather_code")).and_then(|c| c.as_u64()).unwrap_or(3);
            Some((t, unit, sky(code).to_string()))
        });
    }

    fn remember(&mut self, r: &Reading) {
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        for (k, v) in &r.nums {
            let h = self.history.entry(k.clone()).or_default();
            if h.back().map(|b| b.0 == now).unwrap_or(false) {
                h.pop_back();
            }
            h.push_back((now, *v));
            while h.len() > 3600 {
                h.pop_front();
            }
        }
    }
}

/// A one-character picture of the sky (the font's ☁ for clouds and rain, · for clear).
pub fn sky(code: u64) -> &'static str {
    match code {
        0 | 1 => "·",
        _ => "☁",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reads_the_cpu_and_names() {
        assert_eq!(cpu_counters("cpu  100 0 50 800 50 0 0 0 0 0\ncpu0 1 2 3 4"), Some((150, 1000)));
        assert_eq!(short_cpu("AMD Ryzen 7 5700 8-Core Processor"), "R7 5700");
        assert_eq!(short_cpu("12th Gen Intel(R) Core(TM) i7-12700K"), "i7-12700K");
        assert_eq!(short_gpu("Navi 22 [Radeon RX 6700/6700 XT/6750 XT / 6800M/6850M XT]"), "RX 6700");
    }
}
