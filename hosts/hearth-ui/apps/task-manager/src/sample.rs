//! Task Manager's sampler, in Rust: what `hearth-tasks serve` gives Mantel's Task Manager, read straight from /proc
//! and /sys on its own thread, one sample per tick, only while the window is open (and nothing at all while paused).
//! The process list and Details are read only when the page on show needs them; the graphics cards' per-program use
//! (DRM fdinfo) every other tick, and each program's DRM files are looked for once.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Instant;

use crate::apps::{self, AppIndex};

/// Milliseconds between samples (0: paused). Set by the window (Settings › Real-time update speed).
pub static INTERVAL_MS: AtomicU64 = AtomicU64::new(1000);
/// What the page on show needs: 1 = processes, 2 = details, 4 = sensors (temperatures, battery, fans).
pub static WANT: AtomicU8 = AtomicU8::new(1);
/// Wakes the sampler at once (the page changed, or the speed did): a condition variable, so it sleeps for real.
static WAKE_M: Mutex<bool> = Mutex::new(false);
static WAKE_C: Condvar = Condvar::new();

pub fn wake() {
    if let Ok(mut w) = WAKE_M.lock() {
        *w = true;
    }
    WAKE_C.notify_all();
}

pub const W_PROCS: u8 = 1;
pub const W_DETAILS: u8 = 2;
pub const W_SENSORS: u8 = 4;

fn read(p: &str) -> String {
    std::fs::read_to_string(p).unwrap_or_default()
}
fn read_u64(p: &str) -> Option<u64> {
    read(p).trim().parse().ok()
}

#[derive(Debug, Clone, Default)]
pub struct Cpu {
    pub name: String,
    pub cores_physical: usize,
    pub logical: usize,
    pub total: f32,
    pub cores: Vec<f32>,
    pub mhz: Option<u32>,
    pub max_mhz: Option<u32>,
    pub processes: usize,
    pub threads: usize,
    pub temp: Option<f32>,
    pub cache: String,
}

#[derive(Debug, Clone, Default)]
pub struct Mem {
    pub total: u64,
    pub available: u64,
    pub used: u64,
    pub free: u64,
    pub cached: u64,
    pub committed: u64,
    pub commit_limit: u64,
    pub swap_total: u64,
    pub swap_used: u64,
    /// zram: what's in it (before compression) and what it takes.
    pub zram_data: u64,
    pub zram_used: u64,
    pub drive_swap_total: u64,
    pub drive_swap_used: u64,
    /// /proc/pressure/memory "some" avg10.
    pub pressure: f32,
}

#[derive(Debug, Clone, Default)]
pub struct Disk {
    pub name: String,
    pub model: String,
    pub size: u64,
    pub ssd: bool,
    pub active: f32,
    pub read: u64,
    pub write: u64,
    pub response: f32,
}

#[derive(Debug, Clone, Default)]
pub struct Net {
    pub name: String,
    pub kind: String,
    pub up: bool,
    pub speed: u64,
    pub recv: u64,
    pub send: u64,
    pub ipv4: String,
}

#[derive(Debug, Clone, Default)]
pub struct Gpu {
    pub index: usize,
    pub name: String,
    pub vendor: String,
    pub busy: Option<f32>,
    pub vram_used: u64,
    pub vram_total: u64,
    pub temp: Option<f32>,
}

#[derive(Debug, Clone, Default)]
pub struct Battery {
    pub percent: u32,
    pub status: String,
    pub watts: f32,
    pub full_wh: f32,
    pub design_wh: f32,
    pub cycles: u32,
}

/// One program in an app's group (or one row in All processes).
#[derive(Debug, Clone, Default)]
pub struct Kid {
    pub pid: i32,
    pub name: String,
    pub ai: bool,
    pub bridge: bool,
    pub cpu: f32,
    pub mem: u64,
    pub swap: u64,
    pub disk: u64,
    pub gpu: f32,
    pub state: char,
    pub nice: i32,
}

/// An app and everything it started (Processes › Apps & groups).
#[derive(Debug, Clone, Default)]
pub struct Group {
    pub id: String,
    pub pid: i32,
    pub pids: Vec<i32>,
    pub name: String,
    pub icon: String,
    pub desktop: String,
    /// app | game | background | system
    pub kind: String,
    pub protected: bool,
    pub why: String,
    pub tag: String,
    pub exe: String,
    pub cpu: f32,
    pub mem: u64,
    pub swap: u64,
    pub disk: u64,
    pub gpu: f32,
    pub nice: i32,
    pub suspended: bool,
    pub eco: bool,
    pub children: Vec<Kid>,
}

/// One row of Details (every process with its number).
#[derive(Debug, Clone, Default)]
pub struct Row {
    pub pid: i32,
    pub ppid: i32,
    pub name: String,
    pub status: &'static str,
    pub user: String,
    pub mine: bool,
    pub cpu: f32,
    pub cputime: u64,
    pub mem: u64,
    pub swap: u64,
    pub threads: u32,
    pub nice: i32,
    pub cores: String,
    pub cmd: String,
    pub path: String,
}

#[derive(Debug, Clone, Default)]
pub struct Sample {
    pub cpu: Cpu,
    pub mem: Mem,
    pub disks: Vec<Disk>,
    pub nets: Vec<Net>,
    pub gpus: Vec<Gpu>,
    pub battery: Option<Battery>,
    pub uptime: u64,
    pub groups: Vec<Group>,
    pub details: Vec<Row>,
    pub has_procs: bool,
    pub has_details: bool,
}

// ------------------------------------------------------------------ /proc parsing
#[derive(Debug, Clone, Default)]
pub struct P {
    pub pid: i32,
    pub ppid: i32,
    pub comm: String,
    pub exe: String,
    pub path: String,
    pub cmd: Vec<String>,
    pub uid: u32,
    pub state: char,
    pub nice: i32,
    pub threads: u32,
    pub mem: u64,
    pub swap: u64,
    pub jiffies: u64,
    pub start: u64,
    pub kthread: bool,
    pub cpus: String,
}

impl P {
    pub fn key(&self) -> (i32, u64) {
        (self.pid, self.start)
    }
}

pub fn parse_stat(text: &str) -> Option<(i32, String, char, i32, u64, u64, i32, u32, u64, u64)> {
    let open = text.find('(')?;
    let close = text.rfind(')')?;
    let pid = text[..open].trim().parse().ok()?;
    let comm = text[open + 1..close].to_string();
    let f: Vec<&str> = text[close + 1..].split_whitespace().collect();
    if f.len() < 20 {
        return None;
    }
    let state = f[0].chars().next().unwrap_or('?');
    let ppid = f[1].parse().ok()?;
    let flags: u64 = f[6].parse().unwrap_or(0);
    let jiffies = f[11].parse::<u64>().unwrap_or(0) + f[12].parse::<u64>().unwrap_or(0);
    let nice = f[16].parse().unwrap_or(0);
    let threads = f[17].parse().unwrap_or(1);
    let start = f[19].parse().unwrap_or(0);
    Some((pid, comm, state, ppid, jiffies, flags, nice, threads, start, 0))
}

fn status_fields(text: &str) -> (u32, u64, u64, String, bool) {
    let (mut uid, mut rss, mut swap, mut cpus, mut kt) = (u32::MAX, 0u64, 0u64, String::new(), false);
    for l in text.lines() {
        let mut it = l.splitn(2, ':');
        let (k, v) = (it.next().unwrap_or(""), it.next().unwrap_or("").trim());
        match k {
            "Uid" => uid = v.split_whitespace().next().and_then(|x| x.parse().ok()).unwrap_or(u32::MAX),
            "VmRSS" => rss = kb(v),
            "VmSwap" => swap = kb(v),
            "Cpus_allowed_list" => cpus = v.to_string(),
            "Kthread" => kt = v == "1",
            _ => {}
        }
    }
    (uid, rss, swap, cpus, kt)
}

fn kb(v: &str) -> u64 {
    v.split_whitespace().next().and_then(|x| x.parse::<u64>().ok()).unwrap_or(0) * 1024
}

pub fn scan(detail: bool) -> HashMap<i32, P> {
    let mut out = HashMap::new();
    let Ok(rd) = std::fs::read_dir("/proc") else { return out };
    for e in rd.flatten() {
        let name = e.file_name();
        let Some(n) = name.to_str() else { continue };
        if !n.bytes().all(|b| b.is_ascii_digit()) {
            continue;
        }
        let d = format!("/proc/{n}");
        let Some((pid, comm, state, ppid, jiffies, flags, nice, threads, start, _)) = parse_stat(&read(&format!("{d}/stat"))) else { continue };
        let st = read(&format!("{d}/status"));
        if st.is_empty() {
            continue;
        }
        let (uid, mem, swap, cpus, kt) = status_fields(&st);
        let kthread = kt || flags & 0x0020_0000 != 0 || pid == 2 || ppid == 2;
        let (exe, path) = if kthread {
            (String::new(), String::new())
        } else {
            match std::fs::read_link(format!("{d}/exe")) {
                Ok(p) => {
                    let s = p.to_string_lossy().replace(" (deleted)", "");
                    (s.rsplit('/').next().unwrap_or("").to_string(), s)
                }
                Err(_) => (String::new(), String::new()),
            }
        };
        let cmd: Vec<String> = if kthread {
            vec![]
        } else {
            std::fs::read(format!("{d}/cmdline"))
                .map(|b| b.split(|c| *c == 0).filter(|x| !x.is_empty()).map(|x| String::from_utf8_lossy(x).to_string()).collect())
                .unwrap_or_default()
        };
        let _ = detail;
        out.insert(pid, P { pid, ppid, comm, exe, path, cmd, uid, state, nice, threads, mem, swap, jiffies, start, kthread, cpus });
    }
    out
}

/// /proc/stat's cpu lines: [(total, idle)] for the whole machine, then each core.
fn cpu_lines() -> Vec<(u64, u64)> {
    read("/proc/stat")
        .lines()
        .filter(|l| l.starts_with("cpu"))
        .map(|l| {
            let v: Vec<u64> = l.split_whitespace().skip(1).filter_map(|x| x.parse().ok()).collect();
            let total: u64 = v.iter().take(8).sum();
            let idle = v.get(3).copied().unwrap_or(0) + v.get(4).copied().unwrap_or(0);
            (total, idle)
        })
        .collect()
}

fn meminfo() -> HashMap<String, u64> {
    read("/proc/meminfo")
        .lines()
        .filter_map(|l| {
            let mut it = l.splitn(2, ':');
            Some((it.next()?.to_string(), kb(it.next()?.trim())))
        })
        .collect()
}

pub fn user_names() -> HashMap<u32, String> {
    read("/etc/passwd")
        .lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split(':').collect();
            Some((f.get(2)?.parse().ok()?, f.first()?.to_string()))
        })
        .collect()
}

fn cpu_static() -> Cpu {
    let info = read("/proc/cpuinfo");
    let name = info.lines().find(|l| l.starts_with("model name")).and_then(|l| l.split(':').nth(1)).map(|s| s.trim().to_string()).unwrap_or_else(|| "Processor".into());
    let logical = info.lines().filter(|l| l.starts_with("processor")).count().max(1);
    let mut phys: HashSet<(String, String)> = HashSet::new();
    let (mut pid, mut cid) = (String::new(), String::new());
    for l in info.lines() {
        if l.starts_with("physical id") {
            pid = l.split(':').nth(1).unwrap_or("").trim().into();
        } else if l.starts_with("core id") {
            cid = l.split(':').nth(1).unwrap_or("").trim().into();
            phys.insert((pid.clone(), cid.clone()));
        }
    }
    let max_mhz = read_u64("/sys/devices/system/cpu/cpu0/cpufreq/cpuinfo_max_freq").map(|k| (k / 1000) as u32);
    let cache = info.lines().find(|l| l.starts_with("cache size")).and_then(|l| l.split(':').nth(1)).map(|s| s.trim().to_string()).unwrap_or_default();
    Cpu { name, cores_physical: if phys.is_empty() { logical } else { phys.len() }, logical, max_mhz, cache, ..Cpu::default() }
}

fn disk_layout() -> Vec<Disk> {
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir("/sys/block") else { return out };
    let mut names: Vec<String> = rd.flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect();
    names.sort();
    for n in names {
        if n.starts_with("loop") || n.starts_with("ram") || n.starts_with("zram") || n.starts_with("dm-") || n.starts_with("sr") || n.starts_with("md") {
            continue;
        }
        let d = format!("/sys/block/{n}");
        let size = read_u64(&format!("{d}/size")).unwrap_or(0) * 512;
        if size == 0 {
            continue;
        }
        let model = read(&format!("{d}/device/model")).trim().to_string();
        let ssd = read(&format!("{d}/queue/rotational")).trim() == "0";
        out.push(Disk { name: n, model, size, ssd, ..Disk::default() });
    }
    out
}

fn diskstats() -> HashMap<String, (u64, u64, u64, u64, u64, u64, u64)> {
    // name → (reads, rsec, rms, writes, wsec, wms, ioms)
    read("/proc/diskstats")
        .lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            if f.len() < 14 {
                return None;
            }
            let g = |i: usize| f[i].parse::<u64>().unwrap_or(0);
            Some((f[2].to_string(), (g(3), g(5), g(6), g(7), g(9), g(10), g(12))))
        })
        .collect()
}

fn net_layout() -> Vec<Net> {
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir("/sys/class/net") else { return out };
    let mut names: Vec<String> = rd.flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect();
    names.sort();
    for n in names {
        if n == "lo" || n.starts_with("veth") || n.starts_with("docker") || n.starts_with("br-") || n.starts_with("virbr") {
            continue;
        }
        let d = format!("/sys/class/net/{n}");
        let kind = if std::path::Path::new(&format!("{d}/wireless")).exists() || n.starts_with("wl") {
            "Wi-Fi"
        } else if n.starts_with("tailscale") {
            "Tailscale"
        } else if n.starts_with("wg") || n.starts_with("tun") {
            "VPN"
        } else {
            "Ethernet"
        };
        let up = read(&format!("{d}/operstate")).trim() == "up" || kind == "Tailscale";
        let speed = read(&format!("{d}/speed")).trim().parse::<i64>().ok().filter(|s| *s > 0).map(|s| s as u64).unwrap_or(0);
        out.push(Net { name: n, kind: kind.into(), up, speed, ..Net::default() });
    }
    out
}

fn net_dev() -> HashMap<String, (u64, u64)> {
    read("/proc/net/dev")
        .lines()
        .skip(2)
        .filter_map(|l| {
            let (name, rest) = l.split_once(':')?;
            let f: Vec<u64> = rest.split_whitespace().filter_map(|x| x.parse().ok()).collect();
            Some((name.trim().to_string(), (*f.first()?, *f.get(8)?)))
        })
        .collect()
}

fn sysfs_gpus() -> Vec<Gpu> {
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir("/sys/class/drm") else { return out };
    let mut cards: Vec<String> = rd.flatten().map(|e| e.file_name().to_string_lossy().to_string()).filter(|n| n.starts_with("card") && !n.contains('-')).collect();
    cards.sort();
    for c in cards {
        let d = format!("/sys/class/drm/{c}/device");
        let vendor = match read(&format!("{d}/vendor")).trim() {
            "0x1002" => "AMD",
            "0x10de" => "NVIDIA",
            "0x8086" => "Intel",
            _ => continue,
        };
        let busy = read_u64(&format!("{d}/gpu_busy_percent")).map(|v| v as f32);
        let vram_used = read_u64(&format!("{d}/mem_info_vram_used")).unwrap_or(0);
        let vram_total = read_u64(&format!("{d}/mem_info_vram_total")).unwrap_or(0);
        let index = out.len();
        out.push(Gpu { index, name: format!("{vendor} graphics"), vendor: vendor.into(), busy, vram_used, vram_total, temp: None });
    }
    out
}

fn hwmon_temps() -> (Option<f32>, Option<f32>) {
    let (mut cpu, mut gpu) = (None, None);
    let Ok(rd) = std::fs::read_dir("/sys/class/hwmon") else { return (cpu, gpu) };
    for e in rd.flatten() {
        let d = e.path();
        let name = std::fs::read_to_string(d.join("name")).unwrap_or_default().trim().to_string();
        let t = std::fs::read_to_string(d.join("temp1_input")).ok().and_then(|x| x.trim().parse::<f32>().ok()).map(|x| x / 1000.0);
        if ["k10temp", "zenpower", "coretemp", "cpu_thermal", "soc_thermal"].contains(&name.as_str()) && cpu.is_none() {
            cpu = t;
        } else if ["amdgpu", "radeon", "nouveau", "i915", "xe"].contains(&name.as_str()) && gpu.is_none() {
            gpu = t;
        }
    }
    (cpu, gpu)
}

fn battery() -> Option<Battery> {
    let rd = std::fs::read_dir("/sys/class/power_supply").ok()?;
    for e in rd.flatten() {
        let d = e.path();
        if std::fs::read_to_string(d.join("type")).unwrap_or_default().trim() != "Battery" {
            continue;
        }
        let r = |k: &str| std::fs::read_to_string(d.join(k)).ok().and_then(|x| x.trim().parse::<f64>().ok());
        let percent = r("capacity").unwrap_or(0.0) as u32;
        let status = std::fs::read_to_string(d.join("status")).unwrap_or_default().trim().to_string();
        let watts = r("power_now").map(|x| x / 1e6).or_else(|| Some(r("current_now")? * r("voltage_now")? / 1e12)).unwrap_or(0.0) as f32;
        let full = r("energy_full").map(|x| x / 1e6).or_else(|| Some(r("charge_full")? * r("voltage_min_design")? / 1e12)).unwrap_or(0.0) as f32;
        let design = r("energy_full_design").map(|x| x / 1e6).or_else(|| Some(r("charge_full_design")? * r("voltage_min_design")? / 1e12)).unwrap_or(0.0) as f32;
        let cycles = r("cycle_count").unwrap_or(0.0) as u32;
        return Some(Battery { percent, status, watts, full_wh: full, design_wh: design, cycles });
    }
    None
}

fn zram() -> (u64, u64) {
    let (mut data, mut used) = (0, 0);
    if let Ok(rd) = std::fs::read_dir("/sys/block") {
        for e in rd.flatten() {
            let n = e.file_name().to_string_lossy().to_string();
            if !n.starts_with("zram") {
                continue;
            }
            let f: Vec<u64> = read(&format!("/sys/block/{n}/mm_stat")).split_whitespace().filter_map(|x| x.parse().ok()).collect();
            data += f.first().copied().unwrap_or(0);
            used += f.get(2).copied().unwrap_or(0);
        }
    }
    (data, used)
}

fn drive_swap() -> (u64, u64) {
    let (mut t, mut u) = (0, 0);
    for l in read("/proc/swaps").lines().skip(1) {
        let f: Vec<&str> = l.split_whitespace().collect();
        if f.len() >= 4 && !f[0].contains("zram") {
            t += f[2].parse::<u64>().unwrap_or(0) * 1024;
            u += f[3].parse::<u64>().unwrap_or(0) * 1024;
        }
    }
    (t, u)
}

fn proc_io(pid: i32) -> u64 {
    let t = read(&format!("/proc/{pid}/io"));
    let mut sum = 0;
    for l in t.lines() {
        if let Some(v) = l.strip_prefix("read_bytes: ").or_else(|| l.strip_prefix("write_bytes: ")) {
            sum += v.trim().parse::<u64>().unwrap_or(0);
        }
    }
    sum
}

pub fn state_label(s: char) -> &'static str {
    match s {
        'T' | 't' => "Suspended",
        'Z' => "Not responding",
        _ => "Running",
    }
}

pub fn ai_job(cmd: &[String]) -> bool {
    let j = cmd.iter().take(3).map(|s| s.to_lowercase()).collect::<Vec<_>>().join(" ");
    ["claude", "codex", "gemini", "aider", "freebuff", "opencode", "goose", "ollama", "llama-server", "colibri"].iter().any(|a| j.contains(a))
}

/// Cpus_allowed_list "0-3,6" → "5 of 16" style label (all → "All").
pub fn cores_label(list: &str, n: usize) -> String {
    let mut count = 0usize;
    for part in list.split(',').filter(|x| !x.is_empty()) {
        match part.split_once('-') {
            Some((a, b)) => count += b.trim().parse::<usize>().unwrap_or(0).saturating_sub(a.trim().parse::<usize>().unwrap_or(0)) + 1,
            None => count += 1,
        }
    }
    if count == 0 || count >= n { "All".into() } else { count.to_string() }
}

// ------------------------------------------------------------------ the sampler
pub struct Sampler {
    t: Option<Instant>,
    cpu_prev: Vec<(u64, u64)>,
    static_cpu: Option<Cpu>,
    layout_t: Option<Instant>,
    disks: Vec<Disk>,
    nets: Vec<Net>,
    disk_prev: HashMap<String, (u64, u64, u64, u64, u64, u64, u64)>,
    net_prev: HashMap<String, (u64, u64)>,
    jiffies: HashMap<(i32, u64), u64>,
    io: HashMap<(i32, u64), u64>,
    index: Option<(Instant, Arc<AppIndex>)>,
    users: HashMap<u32, String>,
    uid: u32,
    me: i32,
    steam_names: HashMap<String, Option<String>>,
    sensors_t: Option<Instant>,
    temps: (Option<f32>, Option<f32>),
    batt: Option<Battery>,
    tick: u64,
    drm: apps::Drm,
}

impl Sampler {
    pub fn new() -> Sampler {
        Sampler {
            t: None,
            cpu_prev: Vec::new(),
            static_cpu: None,
            layout_t: None,
            disks: Vec::new(),
            nets: Vec::new(),
            disk_prev: HashMap::new(),
            net_prev: HashMap::new(),
            jiffies: HashMap::new(),
            io: HashMap::new(),
            index: None,
            users: user_names(),
            uid: unsafe { libc::getuid() },
            me: std::process::id() as i32,
            steam_names: HashMap::new(),
            sensors_t: None,
            temps: (None, None),
            batt: None,
            tick: 0,
            drm: apps::Drm::default(),
        }
    }

    pub fn sample(&mut self, want: u8) -> Sample {
        let now = Instant::now();
        let dt = self.t.map(|t| now.duration_since(t).as_secs_f64()).unwrap_or(0.0);
        self.tick += 1;
        let mut s = Sample::default();

        // CPU
        let cur = cpu_lines();
        let prev = if self.cpu_prev.len() == cur.len() { self.cpu_prev.clone() } else { cur.clone() };
        let pct: Vec<f32> = prev
            .iter()
            .zip(cur.iter())
            .map(|(p, c)| {
                let dt = c.0.saturating_sub(p.0) as f64;
                let di = c.1.saturating_sub(p.1) as f64;
                if dt > 0.0 { ((1.0 - di / dt) * 100.0).clamp(0.0, 100.0) as f32 } else { 0.0 }
            })
            .collect();
        let dj = cur.first().map(|c| c.0).unwrap_or(0).saturating_sub(prev.first().map(|p| p.0).unwrap_or(0));
        let mut cpu = self.static_cpu.get_or_insert_with(cpu_static).clone();
        cpu.total = pct.first().copied().unwrap_or(0.0);
        cpu.cores = pct.iter().skip(1).copied().collect();
        let mut freqs = Vec::new();
        for i in 0..cpu.logical {
            if let Some(k) = read_u64(&format!("/sys/devices/system/cpu/cpu{i}/cpufreq/scaling_cur_freq")) {
                freqs.push(k);
            }
        }
        cpu.mhz = if freqs.is_empty() { None } else { Some((freqs.iter().sum::<u64>() / freqs.len() as u64 / 1000) as u32) };
        let la = read("/proc/loadavg");
        cpu.threads = la.split_whitespace().nth(3).and_then(|x| x.split('/').nth(1)).and_then(|x| x.parse().ok()).unwrap_or(0);
        self.cpu_prev = cur;

        // Memory
        let mi = meminfo();
        let g = |k: &str| mi.get(k).copied().unwrap_or(0);
        let (zd, zu) = zram();
        let (dst, dsu) = drive_swap();
        let used = g("MemTotal").saturating_sub(g("MemAvailable"));
        s.mem = Mem {
            total: g("MemTotal"),
            available: g("MemAvailable"),
            used,
            free: g("MemFree"),
            cached: g("Cached") + g("SReclaimable") + g("Buffers"),
            committed: g("Committed_AS"),
            commit_limit: g("CommitLimit"),
            swap_total: g("SwapTotal"),
            swap_used: g("SwapTotal").saturating_sub(g("SwapFree")),
            zram_data: zd,
            zram_used: zu,
            drive_swap_total: dst,
            drive_swap_used: dsu,
            pressure: read("/proc/pressure/memory").lines().next().and_then(|l| l.split_whitespace().find(|w| w.starts_with("avg10="))).and_then(|w| w[6..].parse().ok()).unwrap_or(0.0),
        };
        s.uptime = read("/proc/uptime").split_whitespace().next().and_then(|x| x.parse::<f64>().ok()).unwrap_or(0.0) as u64;

        // Disks and network: the layout every 30 s, the counters every tick.
        if self.layout_t.map(|t| now.duration_since(t).as_secs() > 30).unwrap_or(true) {
            self.layout_t = Some(now);
            self.disks = disk_layout();
            self.nets = net_layout();
        }
        let ds = diskstats();
        for d in &self.disks {
            let mut row = d.clone();
            if let (Some(c), Some(p)) = (ds.get(&d.name), self.disk_prev.get(&d.name)) {
                if dt > 0.0 {
                    row.active = ((c.6.saturating_sub(p.6)) as f64 / (dt * 10.0)).clamp(0.0, 100.0) as f32;
                    row.read = (c.1.saturating_sub(p.1) as f64 * 512.0 / dt) as u64;
                    row.write = (c.4.saturating_sub(p.4) as f64 * 512.0 / dt) as u64;
                    let ios = c.0.saturating_sub(p.0) + c.3.saturating_sub(p.3);
                    row.response = if ios > 0 { (c.2.saturating_sub(p.2) + c.5.saturating_sub(p.5)) as f32 / ios as f32 } else { 0.0 };
                }
            }
            s.disks.push(row);
        }
        for (k, v) in ds.iter().filter(|(k, _)| self.disks.iter().any(|d| &d.name == *k)) {
            self.disk_prev.insert(k.clone(), *v);
        }
        let nd = net_dev();
        for n in &self.nets {
            let mut row = n.clone();
            if let (Some(c), Some(p)) = (nd.get(&n.name), self.net_prev.get(&n.name)) {
                if dt > 0.0 {
                    row.recv = (c.0.saturating_sub(p.0) as f64 / dt) as u64;
                    row.send = (c.1.saturating_sub(p.1) as f64 / dt) as u64;
                }
            }
            if let Some(c) = nd.get(&n.name) {
                self.net_prev.insert(n.name.clone(), *c);
            }
            s.nets.push(row);
        }

        // Graphics (sysfs), sensors and the battery every few seconds when asked.
        s.gpus = sysfs_gpus();
        if want & W_SENSORS != 0 && self.sensors_t.map(|t| now.duration_since(t).as_secs() >= 2).unwrap_or(true) {
            self.sensors_t = Some(now);
            self.temps = hwmon_temps();
            self.batt = battery();
        } else if self.batt.is_none() && self.tick == 1 {
            self.batt = battery();
        }
        cpu.temp = self.temps.0;
        for gp in s.gpus.iter_mut() {
            if gp.temp.is_none() && gp.vendor != "NVIDIA" {
                gp.temp = self.temps.1;
            }
        }
        s.battery = self.batt.clone();

        // Processes
        if want & (W_PROCS | W_DETAILS) != 0 {
            let procs = scan(want & W_DETAILS != 0);
            cpu.processes = procs.len();
            let jprev = std::mem::take(&mut self.jiffies);
            let mut cpu_of: HashMap<i32, f32> = HashMap::new();
            for p in procs.values() {
                self.jiffies.insert(p.key(), p.jiffies);
                let v = match jprev.get(&p.key()) {
                    Some(j) if dj > 0 => (100.0 * p.jiffies.saturating_sub(*j) as f64 / dj as f64) as f32,
                    _ => 0.0,
                };
                cpu_of.insert(p.pid, v);
            }
            if want & W_PROCS != 0 {
                s.groups = self.groups(&procs, &cpu_of, dt);
                s.has_procs = true;
            }
            if want & W_DETAILS != 0 {
                let tck = unsafe { libc::sysconf(libc::_SC_CLK_TCK) }.max(1) as u64;
                let mut rows: Vec<Row> = procs
                    .values()
                    .filter(|p| !p.kthread)
                    .map(|p| Row {
                        pid: p.pid,
                        ppid: p.ppid,
                        name: if p.exe.is_empty() { p.comm.clone() } else { p.exe.clone() },
                        status: state_label(p.state),
                        user: self.users.get(&p.uid).cloned().unwrap_or_else(|| p.uid.to_string()),
                        mine: p.uid == self.uid,
                        cpu: cpu_of.get(&p.pid).copied().unwrap_or(0.0),
                        cputime: p.jiffies / tck,
                        mem: p.mem,
                        swap: p.swap,
                        threads: p.threads,
                        nice: p.nice,
                        cores: cores_label(&p.cpus, cpu.logical),
                        cmd: { let c = p.cmd.join(" "); if c.is_empty() { format!("[{}]", p.comm) } else { c.chars().take(400).collect() } },
                        path: p.path.clone(),
                    })
                    .collect();
                rows.sort_by_key(|r| r.pid);
                s.details = rows;
                s.has_details = true;
            }
        } else {
            self.jiffies.clear();
            cpu.processes = std::fs::read_dir("/proc").map(|r| r.flatten().filter(|e| e.file_name().to_string_lossy().bytes().all(|b| b.is_ascii_digit())).count()).unwrap_or(0);
        }
        s.cpu = cpu;
        self.t = Some(now);
        s
    }

    fn groups(&mut self, procs: &HashMap<i32, P>, cpu_of: &HashMap<i32, f32>, dt: f64) -> Vec<Group> {
        let now = Instant::now();
        let fresh = match &self.index {
            Some((t, _)) => now.duration_since(*t).as_secs() > 15,
            None => true,
        };
        if fresh {
            self.index = Some((now, Arc::new(apps::desktop_index())));
        }
        let index = self.index.as_ref().map(|x| x.1.clone()).unwrap_or_default();
        let names = &mut self.steam_names;
        let mut game_name = |id: &str| -> Option<String> { names.entry(id.to_string()).or_insert_with(|| apps::steam_name(id)).clone() };
        let mut groups = apps::build_groups(procs, self.uid, self.me, &index, &mut game_name);
        groups.retain(|g| g.pid != self.me);
        // Disk use per program: /proc/PID/io between two ticks.
        let mut io_now = HashMap::new();
        for g in &groups {
            for pid in &g.pids {
                if let Some(p) = procs.get(pid) {
                    io_now.insert(p.key(), proc_io(*pid));
                }
            }
        }
        let gpu_of = if self.tick % 2 == 0 || self.tick < 3 { self.drm.busy(procs, self.uid) } else { self.drm.last.clone() };
        let io_prev = std::mem::replace(&mut self.io, io_now);
        for g in groups.iter_mut() {
            let mut kids = Vec::new();
            for pid in &g.pids {
                let Some(p) = procs.get(pid) else { continue };
                let cur = self.io.get(&p.key()).copied().unwrap_or(0);
                let before = io_prev.get(&p.key()).copied().unwrap_or(cur);
                let disk = if dt > 0.0 { (cur.saturating_sub(before) as f64 / dt) as u64 } else { 0 };
                kids.push(Kid {
                    pid: *pid,
                    name: { let sn = apps::script_name(p); if !sn.is_empty() { sn } else if !p.exe.is_empty() { p.exe.clone() } else { p.comm.clone() } },
                    ai: ai_job(&p.cmd),
                    bridge: p.cmd.iter().take(2).any(|x| x.rsplit('/').next().unwrap_or("").to_lowercase().contains("bridge")),
                    cpu: cpu_of.get(pid).copied().unwrap_or(0.0),
                    mem: p.mem,
                    swap: p.swap,
                    disk,
                    gpu: gpu_of.get(pid).copied().unwrap_or(0.0),
                    state: p.state,
                    nice: p.nice,
                });
            }
            g.cpu = kids.iter().map(|k| k.cpu).sum();
            g.swap = kids.iter().map(|k| k.swap).sum();
            g.disk = kids.iter().map(|k| k.disk).sum();
            g.gpu = kids.iter().map(|k| k.gpu).sum::<f32>().min(100.0);
            g.nice = kids.iter().map(|k| k.nice).min().unwrap_or(0);
            g.suspended = !kids.is_empty() && kids.iter().all(|k| matches!(k.state, 'T' | 't'));
            g.eco = !kids.is_empty() && kids.iter().all(|k| k.nice >= 19);
            g.exe = procs.get(&g.pid).map(|p| p.path.clone()).unwrap_or_default();
            g.children = if kids.len() > 1 { kids } else { Vec::new() };
        }
        groups.sort_by(|a, b| b.mem.cmp(&a.mem).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
        groups
    }
}

/// The sampler's thread: a sample, then a sleep for the interval (woken early when the page changes); nothing while
/// paused. Ends when the window is gone.
pub fn stream() -> futures::channel::mpsc::UnboundedReceiver<Arc<Sample>> {
    let (tx, rx) = futures::channel::mpsc::unbounded();
    let _ = std::thread::Builder::new().name("hearth-tasks-sampler".into()).spawn(move || {
        let mut s = Sampler::new();
        // The first numbers need two readings: a quick one now, so the first frame isn't empty for a second.
        let _ = s.sample(WANT.load(Ordering::Relaxed));
        std::thread::sleep(std::time::Duration::from_millis(250));
        let mut woke = true;
        loop {
            let iv = INTERVAL_MS.load(Ordering::Relaxed);
            if iv > 0 || woke {
                let smp = s.sample(WANT.load(Ordering::Relaxed));
                if tx.unbounded_send(Arc::new(smp)).is_err() {
                    return;
                }
            }
            // Asleep until the next tick, or until the window wakes it (paused: until then).
            let Ok(g) = WAKE_M.lock() else { return };
            let mut g = if iv == 0 {
                match WAKE_C.wait_while(g, |w| !*w) {
                    Ok(g) => g,
                    Err(_) => return,
                }
            } else {
                match WAKE_C.wait_timeout_while(g, std::time::Duration::from_millis(iv), |w| !*w) {
                    Ok((g, _)) => g,
                    Err(_) => return,
                }
            };
            woke = *g;
            *g = false;
        }
    });
    rx
}
