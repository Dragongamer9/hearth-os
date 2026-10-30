//! Hearth Benchmark (`hearth-tools --app benchmark`): how fast this device is, and whether it stays fast.
//!
//! Tests (parse.rs has the list, the scoring and the reference device):
//!   processor, one core and all cores: 7-Zip's own benchmark (`7z b`, its rating in MIPS)
//!   memory speed and latency: measured here (multi-threaded copy; a random pointer chase through 64-256 MiB)
//!   storage: fio, sequential 1 MB and random 4 KB reads and writes, on a test file in one folder,
//!            `<drive>/.hearth-benchmark/`, deleted afterwards (nothing is ever written anywhere else)
//!   graphics: vkmark (Vulkan) and glmark2 (OpenGL), and glmark2's heaviest scenes as the game-style test
//!   sustained load: every core busy (plus glmark2 if asked) for 10-20 minutes, recording clocks, temperatures,
//!            watts and the work done each second: how much of its speed the device keeps once it's hot
//!   battery: what the device draws on battery when idle and under load, and the hours that gives
//!   internet: the same speed test as Network Tools (Cloudflare); home network: Hearth Link to a paired device
//!
//! Safety: a reading every second (sensors.rs) shows on the page and stops the run if the processor or graphics
//! stays at its heat limit (95 °C, lower when the sensor says so) for 3 seconds. The page's B / Cancel stops it at
//! once (the programs are killed). Long or heavy runs on battery need the page to say the person agreed, and are
//! refused under 20 % battery. One run at a time.
//!
//! Results: ~/.local/share/nimbus/benchmark/history.json (the last 60 runs) and summary.json (the latest scores,
//! which Hearth Link shows paired devices: Compare › Your devices).

pub mod parse;
pub mod sensors;

use crate::util;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// The only folder the storage test writes in (on the chosen drive), and its one file.
pub const TMP_NAME: &str = ".hearth-benchmark";
pub const TMP_FILE: &str = "hearth-bench.tmp";
const KEEP: usize = 60;
static RUNNING: AtomicBool = AtomicBool::new(false);

fn dir() -> PathBuf { util::data_dir().join("benchmark") }
fn history_file() -> PathBuf { dir().join("history.json") }
pub fn summary_file() -> PathBuf { dir().join("summary.json") }
fn now_ms() -> u64 { SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0) }
fn r2(v: f64) -> f64 { (v * 100.0).round() / 100.0 }
fn threads() -> usize { std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4) }
fn stop_word(s: &AtomicBool) -> bool { s.load(Ordering::SeqCst) }

// ---------------------------------------------------------------- what's here

fn sevenzip_bin() -> Option<PathBuf> { ["7zz", "7z", "7za"].iter().find_map(|p| util::which(p)) }
/// glmark2 for the window system we're in (X11 is there in Game Mode and Desktop Mode alike).
fn glmark_bin() -> Option<PathBuf> {
    let order: [&str; 3] = if std::env::var_os("DISPLAY").is_some() { ["glmark2", "glmark2-x11", "glmark2-wayland"] } else { ["glmark2-wayland", "glmark2", "glmark2-x11"] };
    order.iter().find_map(|p| util::which(p))
}
fn nvidia() -> bool { Path::new("/sys/module/nvidia").exists() && util::which("nvidia-smi").is_some() }

fn free_space(p: &Path) -> Option<u64> {
    use std::os::unix::ffi::OsStrExt;
    let c = std::ffi::CString::new(p.as_os_str().as_bytes()).ok()?;
    let mut s: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statvfs(c.as_ptr(), &mut s) } != 0 { return None; }
    Some(s.f_bavail as u64 * if s.f_frsize > 0 { s.f_frsize as u64 } else { s.f_bsize as u64 })
}
fn writable(p: &Path) -> bool {
    use std::os::unix::ffi::OsStrExt;
    std::ffi::CString::new(p.as_os_str().as_bytes()).map(|c| unsafe { libc::access(c.as_ptr(), libc::W_OK) } == 0).unwrap_or(false)
}

/// Where the storage test can run: the home folder's drive, each Steam library's drive, and drives mounted for this
/// account (/run/media/<user>/...), one place per drive. [{path, label, free}].
pub fn places() -> Vec<Value> {
    use std::os::unix::fs::MetadataExt;
    let home = util::home();
    let mut cand: Vec<(PathBuf, String)> = vec![(home.clone(), "Home folder (system drive)".into())];
    for f in [".local/share/Steam/steamapps/libraryfolders.vdf", ".steam/steam/steamapps/libraryfolders.vdf"] {
        if let Ok(t) = std::fs::read_to_string(home.join(f)) {
            for p in parse::steam_libraries(&t) { cand.push((PathBuf::from(&p), format!("Steam library ({p})"))); }
        }
    }
    let user = home.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    for root in [format!("/run/media/{user}"), format!("/media/{user}")] {
        let mut ds: Vec<PathBuf> = std::fs::read_dir(&root).map(|rd| rd.flatten().map(|e| e.path()).collect()).unwrap_or_default();
        ds.sort();
        for d in ds { let n = d.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(); cand.push((d, format!("Drive “{n}”"))); }
    }
    let mut seen = Vec::new();
    cand.into_iter().filter_map(|(p, label)| {
        let m = std::fs::metadata(&p).ok().filter(|m| m.is_dir())?;
        if seen.contains(&m.dev()) || !writable(&p) { return None; }
        seen.push(m.dev());
        Some(json!({"path": p.to_string_lossy(), "label": label, "free": free_space(&p)}))
    }).collect()
}

/// The page's first look: which programs are here, the drives, the battery, the device, the last run.
pub fn state() -> Result<Value, String> {
    let has = |p: Option<PathBuf>| p.map(|x| x.to_string_lossy().to_string());
    let hist = load_history();
    Ok(json!({
        "tools": {"sevenzip": has(sevenzip_bin()), "fio": has(util::which("fio")), "vkmark": has(util::which("vkmark")), "glmark2": has(glmark_bin()),
                  "nvidiaSmi": nvidia(), "curl": util::which("curl").is_some(), "link": Path::new("/usr/bin/hearth-link").exists()},
        "places": places(),
        "power": sensors::battery(Path::new("/sys/class/power_supply")),
        "reading": sensors::reading(Path::new("/sys"), None, (None, None, None)),
        "device": device(),
        "tests": parse::TESTS.iter().map(|t| json!({"id": t.id, "name": t.name, "unit": t.unit, "overall": t.overall})).collect::<Vec<_>>(),
        "quick": parse::QUICK, "full": parse::FULL,
        "last": hist.first(), "running": RUNNING.load(Ordering::SeqCst), "threads": threads(),
    }))
}

fn device() -> Value {
    let f = hearthfetch::to_json(&hearthfetch::collect());
    json!({"name": f["host"], "model": f["device"], "cpu": f["cpu"]["name"], "threads": f["cpu"]["threads"], "gpu": f["gpus"].get(0).cloned().unwrap_or(Value::Null),
           "memory": f["memory"]["totalBytes"], "os": f["os"], "edition": util::edition()})
}

// ---------------------------------------------------------------- history

pub fn load_history() -> Vec<Value> {
    std::fs::read(history_file()).ok().and_then(|b| serde_json::from_slice::<Vec<Value>>(&b).ok()).unwrap_or_default()
}
fn save_history(h: &[Value]) -> Result<(), String> {
    std::fs::create_dir_all(dir()).map_err(|e| e.to_string())?;
    let tmp = history_file().with_extension("json.new");
    std::fs::write(&tmp, serde_json::to_vec(h).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, history_file()).map_err(|e| e.to_string())
}
/// The newest run first; each with how it compares with the run of the same kind before it.
pub fn history() -> Result<Value, String> {
    let h = load_history();
    let runs: Vec<Value> = h.iter().enumerate().map(|(i, r)| {
        let mut r = r.clone();
        if let Some(p) = h.iter().skip(i + 1).find(|p| p["mode"] == r["mode"]) { r["compare"] = parse::compare(p, &r); }
        r
    }).collect();
    Ok(json!({"runs": runs}))
}
pub fn compare(args: &Value) -> Result<Value, String> {
    let h = load_history();
    let get = |k: &str| h.iter().find(|r| r["id"].as_str() == Some(util::arg(args, k))).cloned().ok_or("That run isn't in the history anymore");
    Ok(parse::compare(&get("before")?, &get("after")?))
}
pub fn delete(args: &Value) -> Result<Value, String> {
    let id = util::arg(args, "id");
    let mut h = load_history();
    let n = h.len();
    h.retain(|r| r["id"].as_str() != Some(id));
    if h.len() == n { return Err("That run isn't in the history".into()); }
    save_history(&h)?;
    write_summary(h.first());
    Ok(json!({"ok": true}))
}
/// The latest scores, small, for Hearth Link to show paired devices.
fn write_summary(r: Option<&Value>) {
    let Some(r) = r else { let _ = std::fs::remove_file(summary_file()); return };
    let scores: serde_json::Map<String, Value> = r["tests"].as_object().map(|t| t.iter().filter_map(|(k, v)| Some((k.clone(), v.get("score")?.clone()))).collect()).unwrap_or_default();
    let s = json!({"at": r["at"], "mode": r["mode"], "version": r["version"], "overall": r["overall"], "scores": scores,
                   "device": {"model": r["device"]["model"], "cpu": r["device"]["cpu"], "gpu": r["device"]["gpu"]}});
    let _ = std::fs::create_dir_all(dir());
    let _ = std::fs::write(summary_file(), s.to_string());
}

/// Your other Hearth devices' latest scores, through Hearth Link (each paired device that answers).
pub fn devices() -> Result<Value, String> {
    hearth_core::link::call("bench.peers", &json!({}), 30).map_err(|e| if e.contains("isn't running") { "Hearth Link isn't running, so your other devices can't be asked.".into() } else { e })
}

// ---------------------------------------------------------------- one run

/// What one run shares between its tests and the reading thread.
struct Run<'a> {
    cancel: &'a AtomicBool,
    /// The whole run stops (cancelled, or too hot).
    stop: AtomicBool,
    /// The current test stops (the run stops, or the test ran out of time).
    test_stop: AtomicBool,
    deadline: Mutex<Option<Instant>>,
    hot: Mutex<Option<String>>,
    fps: Mutex<Option<f64>>,
    /// The work counter of the load the long tests make (chunks done).
    work: AtomicU64,
    /// The readings of the test that records them (sustained, battery).
    record: Mutex<Option<Vec<Value>>>,
    done: AtomicBool,
}

impl<'a> Run<'a> {
    fn new(cancel: &'a AtomicBool) -> Self {
        Run { cancel, stop: AtomicBool::new(false), test_stop: AtomicBool::new(false), deadline: Mutex::new(None), hot: Mutex::new(None),
              fps: Mutex::new(None), work: AtomicU64::new(0), record: Mutex::new(None), done: AtomicBool::new(false) }
    }
    fn begin(&self, limit_secs: u64) {
        self.test_stop.store(stop_word(&self.stop), Ordering::SeqCst);
        *self.deadline.lock().unwrap() = Some(Instant::now() + Duration::from_secs(limit_secs));
        *self.fps.lock().unwrap() = None;
    }
    fn stopped(&self) -> bool { stop_word(&self.test_stop) || stop_word(&self.stop) || stop_word(self.cancel) }
    fn why(&self) -> String {
        if let Some(h) = self.hot.lock().unwrap().clone() { return h; }
        if stop_word(self.cancel) { return "Stopped.".into(); }
        "It took too long and was stopped.".into()
    }
}

/// The reading thread: every second, a reading for the page; stops the run when it's too hot, the current test when
/// its time is up.
fn monitor(r: &Run, emit: &(dyn Fn(Value) + Sync)) {
    let stat = || std::fs::read_to_string("/proc/stat").map(|s| sensors::cpu_times(&s)).unwrap_or((0, 0));
    let mut prev = stat();
    let mut hot_for = 0;
    let nv_on = nvidia();
    let mut nv = (None, None, None);
    let mut last_work = r.work.load(Ordering::SeqCst);
    let mut last_at = Instant::now();
    let started = Instant::now();
    let mut tick = 0u64;
    while !stop_word(&r.done) {
        for _ in 0..4 { if stop_word(&r.done) { return; } if stop_word(r.cancel) { r.stop.store(true, Ordering::SeqCst); } std::thread::sleep(Duration::from_millis(250)); }
        let now = stat();
        let busy = if now.1 > prev.1 { Some((100.0 * (1.0 - now.0.saturating_sub(prev.0) as f64 / (now.1 - prev.1) as f64)).clamp(0.0, 100.0).round()) } else { None };
        prev = now;
        if nv_on && tick % 3 == 0 {
            if let Ok(o) = util::run("nvidia-smi", &["--query-gpu=temperature.gpu,clocks.gr,power.draw", "--format=csv,noheader,nounits"], 2) { nv = parse::nvidia_query(&o.out); }
        }
        tick += 1;
        let mut v = sensors::reading(Path::new("/sys"), busy, nv);
        v["fps"] = json!(*r.fps.lock().unwrap());
        let w = r.work.load(Ordering::SeqCst);
        let rate = (w - last_work) as f64 / last_at.elapsed().as_secs_f64().max(0.1);
        last_work = w;
        last_at = Instant::now();
        v["rate"] = json!(r2(rate));
        match sensors::too_hot(&v) {
            Some((part, t, limit)) => {
                hot_for += 1;
                if hot_for >= 3 && r.hot.lock().unwrap().is_none() {
                    *r.hot.lock().unwrap() = Some(format!("Stopped for safety: the {part} reached {t:.0} °C (the limit is {limit:.0} °C). Let the device cool down before trying again."));
                    r.stop.store(true, Ordering::SeqCst);
                }
            }
            None => hot_for = 0,
        }
        let late = r.deadline.lock().unwrap().map(|d| Instant::now() > d).unwrap_or(false);
        if stop_word(&r.stop) || late { r.test_stop.store(true, Ordering::SeqCst); }
        if let Some(rec) = r.record.lock().unwrap().as_mut() {
            rec.push(json!({"t": started.elapsed().as_secs(), "mhz": v["cpuMhz"], "cpuT": v["cpuTemp"], "gpuT": v["gpuTemp"], "gpuMhz": v["gpuMhz"],
                            "w": if v["battW"].is_null() { v["gpuW"].clone() } else { v["battW"].clone() }, "rate": v["rate"]}));
        }
        emit(json!({"live": v}));
    }
}

fn measured(id: &str, value: f64, detail: Value) -> Value {
    let d = parse::def(id);
    json!({"value": r2(value), "unit": d.map(|d| d.unit).unwrap_or(""), "score": parse::score(id, value), "detail": detail})
}

/// Runs a program line by line until it ends or the test stops; its output, or why it stopped.
fn program(r: &Run, bin: &Path, args: &[String], mut each: impl FnMut(&str)) -> Result<String, String> {
    let a: Vec<&str> = args.iter().map(String::as_str).collect();
    let mut out = String::new();
    let code = util::run_lines(&bin.to_string_lossy(), &a, &r.test_stop, |l| { each(l); if out.len() < 1 << 20 { out.push_str(l); out.push('\n'); } })?;
    match code {
        None => Err(r.why()),
        Some(_) if r.stopped() => Err(r.why()),
        Some(_) => Ok(out),
    }
}
fn last_words(out: &str) -> String { out.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("no output").trim().chars().take(200).collect() }

fn cpu(r: &Run, single: bool, full: bool) -> Result<Value, String> {
    let bin = sevenzip_bin().ok_or("7-Zip isn't installed (package 7zip), so the processor can't be measured.")?;
    let passes = if full { "6" } else { "2" };
    let mut args = vec!["b".to_string(), passes.to_string()];
    if single { args.push("-mmt1".into()); }
    r.begin(if full { 420 } else { 180 });
    let out = program(r, &bin, &args, |_| {})?;
    let mips = parse::sevenzip(&out).ok_or_else(|| format!("7-Zip didn't give a result ({})", last_words(&out)))?;
    Ok(measured(if single { "cpu1" } else { "cpuN" }, mips, json!({"tool": "7-Zip benchmark", "passes": passes, "threads": if single { 1 } else { threads() }})))
}

/// Memory copy speed in GB/s (bytes read + written), with `threads` threads over two buffers of `mib` MiB, for
/// about `secs` seconds. None when stopped.
pub fn copy_bandwidth(mib: usize, threads: usize, secs: f64, stop: &AtomicBool) -> Option<(f64, f64)> {
    let n = (mib << 20) / 8;
    let src: Vec<u64> = (0..n as u64).collect();
    let mut dst = vec![0u64; n];
    let chunk = n.div_ceil(threads.max(1)).max(1);
    let t0 = Instant::now();
    let mut bytes = 0f64;
    while t0.elapsed().as_secs_f64() < secs {
        if stop_word(stop) { return None; }
        std::thread::scope(|s| { for (d, x) in dst.chunks_mut(chunk).zip(src.chunks(chunk)) { s.spawn(move || d.copy_from_slice(x)); } });
        bytes += (n * 16) as f64;
    }
    let copy = bytes / t0.elapsed().as_secs_f64() / 1e9;
    // Reading only: every thread adds up its part.
    let t1 = Instant::now();
    let mut rbytes = 0f64;
    while t1.elapsed().as_secs_f64() < secs / 2.0 {
        if stop_word(stop) { return None; }
        let sum: u64 = std::thread::scope(|s| {
            let hs: Vec<_> = dst.chunks(chunk).map(|d| s.spawn(move || d.iter().fold(0u64, |a, x| a.wrapping_add(*x)))).collect();
            hs.into_iter().map(|h| h.join().unwrap_or(0)).fold(0u64, |a, x| a.wrapping_add(x))
        });
        std::hint::black_box(sum);
        rbytes += (n * 8) as f64;
    }
    Some((copy, rbytes / t1.elapsed().as_secs_f64() / 1e9))
}

/// Memory latency in nanoseconds: `steps` dependent loads in a random cycle through `mib` MiB. None when stopped.
pub fn latency_ns(mib: usize, steps: u64, stop: &AtomicBool) -> Option<f64> {
    let n = ((mib << 20) / 8).max(2);
    let mut next: Vec<usize> = (0..n).collect();
    let mut x: u64 = 0x9E37_79B9_7F4A_7C15;
    for i in (1..n).rev() { x ^= x << 13; x ^= x >> 7; x ^= x << 17; next.swap(i, (x % i as u64) as usize); } // Sattolo: one cycle
    let mut p = 0usize;
    let t0 = Instant::now();
    let mut done = 0u64;
    while done < steps {
        if stop_word(stop) { return None; }
        let k = (steps - done).min(1 << 20);
        for _ in 0..k { p = next[p]; }
        done += k;
    }
    std::hint::black_box(p);
    Some(t0.elapsed().as_nanos() as f64 / steps as f64)
}

fn mem_available_mib() -> usize {
    std::fs::read_to_string("/proc/meminfo").ok().and_then(|m| m.lines().find(|l| l.starts_with("MemAvailable:")).and_then(|l| l.split_whitespace().nth(1)?.parse::<usize>().ok())).map(|kb| kb / 1024).unwrap_or(2048)
}

fn mem_bw(r: &Run, full: bool) -> Result<Value, String> {
    let mib = (if full { 512 } else { 256 }).min(mem_available_mib() / 4).max(32);
    r.begin(90);
    let (copy, read) = copy_bandwidth(mib, threads().min(16), if full { 4.0 } else { 2.0 }, &r.test_stop).ok_or_else(|| r.why())?;
    Ok(measured("memBw", copy, json!({"copyGBs": r2(copy), "readGBs": r2(read), "mib": mib, "threads": threads().min(16), "tool": "Hearth (built in)"})))
}

fn mem_lat(r: &Run, full: bool) -> Result<Value, String> {
    let mib = (if full { 256 } else { 64 }).min(mem_available_mib() / 4).max(16);
    r.begin(90);
    let ns = latency_ns(mib, if full { 40_000_000 } else { 15_000_000 }, &r.test_stop).ok_or_else(|| r.why())?;
    Ok(measured("memLat", ns, json!({"mib": mib, "tool": "Hearth (built in)"})))
}

/// The storage test's folder on a drive: `<place>/.hearth-benchmark`, made if needed; refused when something else
/// (a link, a file) has that name.
pub fn temp_dir_for(place: &Path) -> Result<PathBuf, String> {
    if !place.is_absolute() { return Err("Not a drive".into()); }
    let d = place.join(TMP_NAME);
    match std::fs::symlink_metadata(&d) {
        Ok(m) if m.file_type().is_symlink() || !m.is_dir() => return Err(format!("{} is in the way (it isn't a folder Hearth Benchmark made).", d.display())),
        Ok(_) => {}
        Err(_) => std::fs::create_dir(&d).map_err(|e| format!("Couldn't make {}: {e}", d.display()))?,
    }
    Ok(d)
}
/// Removes the test file and the folder (only if nothing else is in it).
pub fn clean(d: &Path) {
    if d.file_name().and_then(|n| n.to_str()) != Some(TMP_NAME) { return; }
    let _ = std::fs::remove_file(d.join(TMP_FILE));
    let _ = std::fs::remove_dir(d);
}

fn fio_one(r: &Run, fio: &Path, d: &Path, rw: &str, bs: &str, mb: u64, secs: u64) -> Result<((f64, f64), (f64, f64)), String> {
    let mut last = String::new();
    // The fastest way the drive and fio allow: libaio at depth 32 with direct I/O; else plain reads with 4 jobs for
    // random; else without direct I/O (file systems that don't support it).
    for (engine, depth, direct) in [("libaio", "32", "1"), ("psync", "1", "1"), ("psync", "1", "0")] {
        let jobs = if engine == "psync" && rw.starts_with("rand") { "4" } else { "1" };
        let mut a: Vec<String> = vec!["--name=hearth".into(), format!("--directory={}", d.display()), format!("--filename={TMP_FILE}"), format!("--size={mb}m"),
            format!("--rw={rw}"), format!("--bs={bs}"), format!("--ioengine={engine}"), format!("--iodepth={depth}"), format!("--numjobs={jobs}"),
            format!("--direct={direct}"), format!("--runtime={secs}"), "--time_based".into(), "--ramp_time=1".into(), "--group_reporting".into(), "--output-format=json".into()];
        if rw.contains("write") { a.push("--end_fsync=1".into()); }
        r.begin(secs + 180);
        let out = program(r, fio, &a, |_| {})?;
        if let Some(v) = parse::fio(&out) { return Ok(v); }
        last = last_words(&out);
    }
    Err(format!("fio couldn't test this drive ({last})"))
}

fn disk(r: &Run, emit: &(dyn Fn(Value) + Sync), place: &Path, full: bool) -> Result<Value, String> {
    let fio = util::which("fio").ok_or("fio isn't installed (package fio), so storage can't be measured.")?;
    let mb: u64 = if full { 1024 } else { 256 };
    let free = free_space(place).unwrap_or(0);
    if free < (mb << 20) + (2 << 30) { return Err(format!("Not enough free space on {} (the test needs {} MB and keeps 2 GB free).", place.display(), mb)); }
    let d = temp_dir_for(place)?;
    let secs = if full { 10 } else { 4 };
    let mut got = Vec::new();
    let mut res = Ok(());
    for (i, (rw, bs)) in [("write", "1M"), ("read", "1M"), ("randwrite", "4k"), ("randread", "4k")].iter().enumerate() {
        emit(json!({"step": {"test": "disk", "part": i, "of": 4, "what": rw}}));
        match fio_one(r, &fio, &d, rw, bs, mb, secs) { Ok(v) => got.push(v), Err(e) => { res = Err(e); break; } }
    }
    clean(&d);
    res?;
    let (sw, sr, rwi, rri) = (got[0].1 .0 / 1e6, got[1].0 .0 / 1e6, got[2].1 .1, got[3].0 .1);
    let ratio = parse::disk_ratio(sr, sw, rri, rwi).ok_or("fio gave no numbers")?;
    Ok(json!({"value": r2(sr), "unit": "MB/s read", "score": parse::score("disk", ratio),
              "detail": {"seqReadMBs": r2(sr), "seqWriteMBs": r2(sw), "randReadIops": rri.round(), "randWriteIops": rwi.round(), "sizeMB": mb, "drive": place.to_string_lossy(), "tool": "fio"}}))
}

fn mark(r: &Run, kind: &str, full: bool) -> Result<Value, String> {
    let (bin, name) = if kind == "vulkan" { (util::which("vkmark"), "vkmark") } else { (glmark_bin(), "glmark2") };
    let bin = bin.ok_or(format!("{name} isn't installed (package {name}), so this can't be measured."))?;
    let mut args: Vec<String> = vec!["-s".into(), "1280x720".into()];
    let scenes: &[&str] = match (kind, full) {
        ("vulkan", false) => &["vertex:duration=4", "texture:duration=4", "shading:duration=4", "cube:duration=4", "effect2d:duration=4"],
        ("opengl", false) => &["build:use-vbo=true:duration=4", "texture:texture-filter=linear:duration=4", "shading:shading=phong:duration=4", "bump:bump-render=high-poly:duration=4", "refract:duration=4"],
        ("game", _) => &["terrain:duration=15", "jellyfish:duration=10", "refract:duration=10", "shadow:duration=10"],
        _ => &[],
    };
    for s in scenes { args.push("-b".into()); args.push(s.to_string()); }
    r.begin(if full { 600 } else { 150 });
    let mut fps = Vec::new();
    let out = program(r, &bin, &args, |l| if let Some(f) = parse::mark_fps(l) { fps.push(f); *r.fps.lock().unwrap() = Some(f); })?;
    let avg = (!fps.is_empty()).then(|| fps.iter().sum::<f64>() / fps.len() as f64);
    let detail = json!({"tool": name, "scenes": fps.len(), "avgFps": avg.map(r2), "minFps": fps.iter().cloned().reduce(f64::min), "size": "1280x720"});
    if kind == "game" {
        let a = avg.ok_or_else(|| format!("{name} didn't draw anything ({})", last_words(&out)))?;
        return Ok(measured("game", a, detail));
    }
    let s = parse::mark_score(&out).or(avg).ok_or_else(|| format!("{name} didn't finish ({})", last_words(&out)))?;
    Ok(measured(kind, s, detail))
}

fn inet(r: &Run, emit: &(dyn Fn(Value) + Sync)) -> Result<Value, String> {
    r.begin(150);
    let v = crate::network::speed(&r.test_stop, &|p| emit(json!({"step": {"test": "inet", "progress": p}}))).map_err(|e| if r.stopped() { r.why() } else { e })?;
    let (down, up) = (v["download"].as_f64().unwrap_or(0.0), v["upload"].as_f64().unwrap_or(0.0));
    let ratio = parse::net_ratio(down, up, false).ok_or("The speed test gave no numbers")?;
    Ok(json!({"value": r2(down), "unit": "Mbit/s down", "score": parse::score("inet", ratio),
              "detail": {"downMbps": r2(down), "upMbps": r2(up), "latencyMs": v["latency"], "server": v["server"], "tool": "speed.cloudflare.com"}}))
}

fn lan(r: &Run, peer: &str) -> Result<Value, String> {
    if peer.is_empty() { return Err("No paired Hearth device was chosen for the home network test.".into()); }
    if !peer.chars().all(|c| c.is_ascii_alphanumeric()) { return Err("Not a device".into()); }
    r.begin(120);
    let v = hearth_core::link::call("bench.net", &json!({"peer": peer}), 100)?;
    let (down, up) = (v["download"].as_f64().unwrap_or(0.0), v["upload"].as_f64().unwrap_or(0.0));
    let ratio = parse::net_ratio(down, up, true).ok_or("Hearth Link gave no numbers")?;
    Ok(json!({"value": r2(down), "unit": "Mbit/s down", "score": parse::score("lan", ratio),
              "detail": {"downMbps": r2(down), "upMbps": r2(up), "latencyMs": v["latency"], "device": v["name"], "tool": "Hearth Link"}}))
}

/// Busy work for the long tests: integer and floating-point math, counted in chunks.
fn work_chunk(seed: u64) -> u64 {
    let (mut x, mut f) = (seed | 1, 1.0001f64);
    for _ in 0..200_000 { x ^= x << 13; x ^= x >> 7; x ^= x << 17; f = (f * 1.000_000_1 + (x & 0xff) as f64 * 1e-9).sqrt() + 0.5; }
    x ^ f.to_bits()
}

/// Every core busy (and glmark2's terrain scene too when `gpu`) for `secs` seconds or until the test stops.
fn load_for(r: &Run, emit: &(dyn Fn(Value) + Sync), secs: u64, gpu: bool, label: &str) {
    let end = AtomicBool::new(false);
    let mut child = if gpu {
        glmark_bin().and_then(|b| std::process::Command::new(b).args(["-s", "1280x720", "--run-forever", "-b", "terrain"])
            .stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn().ok())
    } else { None };
    let t0 = Instant::now();
    std::thread::scope(|s| {
        for i in 0..threads() {
            let (end, work) = (&end, &r.work);
            s.spawn(move || { let mut seed = 0x5DEE_CE66_Du64 + i as u64; while !stop_word(end) { seed = work_chunk(seed); work.fetch_add(1, Ordering::Relaxed); } });
        }
        while t0.elapsed().as_secs() < secs && !r.stopped() {
            std::thread::sleep(Duration::from_millis(500));
            emit(json!({"step": {"what": label, "elapsed": t0.elapsed().as_secs(), "total": secs}}));
        }
        end.store(true, Ordering::SeqCst);
    });
    if let Some(c) = child.as_mut() { let _ = c.kill(); let _ = c.wait(); }
}
fn wait_for(r: &Run, emit: &(dyn Fn(Value) + Sync), secs: u64, label: &str) {
    let t0 = Instant::now();
    while t0.elapsed().as_secs() < secs && !r.stopped() {
        std::thread::sleep(Duration::from_millis(500));
        emit(json!({"step": {"what": label, "elapsed": t0.elapsed().as_secs(), "total": secs}}));
    }
}
fn avg_of(v: &[Value], k: &str) -> Option<f64> {
    let x: Vec<f64> = v.iter().filter_map(|s| s[k].as_f64()).collect();
    (!x.is_empty()).then(|| r2(x.iter().sum::<f64>() / x.len() as f64))
}
fn max_of(v: &[Value], k: &str) -> Option<f64> { v.iter().filter_map(|s| s[k].as_f64()).reduce(f64::max) }

fn sustained(r: &Run, emit: &(dyn Fn(Value) + Sync), minutes: u64, gpu: bool) -> Result<Value, String> {
    let secs = minutes.clamp(1, 30) * 60;
    r.begin(secs + 60);
    *r.record.lock().unwrap() = Some(Vec::new());
    load_for(r, emit, secs, gpu && glmark_bin().is_some(), "sustained");
    let rec = r.record.lock().unwrap().take().unwrap_or_default();
    if stop_word(r.cancel) { return Err("Stopped.".into()); }
    let rates: Vec<f64> = rec.iter().filter_map(|s| s["rate"].as_f64()).collect();
    let kept = parse::kept_percent(&rates).ok_or_else(|| if r.stopped() { r.why() } else { "Too few readings to judge.".into() })?;
    let first: Vec<Value> = rec.iter().skip(10).take(60).cloned().collect();
    let last: Vec<Value> = rec.iter().skip(rec.len().saturating_sub(120)).cloned().collect();
    let mut v = measured("sustained", kept, json!({
        "minutes": minutes, "ranSeconds": rec.len(), "gpuLoad": gpu, "throttled": kept < 90.0,
        "peakCpuTemp": max_of(&rec, "cpuT"), "peakGpuTemp": max_of(&rec, "gpuT"), "avgWatts": avg_of(&rec, "w"),
        "startMhz": avg_of(&first, "mhz"), "endMhz": avg_of(&last, "mhz"), "samples": parse::thin(&rec, 240), "tool": "Hearth (built in)"}));
    if let Some(h) = r.hot.lock().unwrap().clone() { v["stopped"] = json!(h); }
    Ok(v)
}

fn battery(r: &Run, emit: &(dyn Fn(Value) + Sync), full: bool) -> Result<Value, String> {
    let b = sensors::battery(Path::new("/sys/class/power_supply"));
    if b.is_null() { return Err("This device has no battery.".into()); }
    if b["onBattery"] != true { return Err("Unplug the charger first: drain can only be measured on battery.".into()); }
    let phase = if full { 180 } else { 90 };
    r.begin(phase * 2 + 60);
    *r.record.lock().unwrap() = Some(Vec::new());
    wait_for(r, emit, phase, "battery-light");
    let light: Vec<Value> = r.record.lock().unwrap().replace(Vec::new()).unwrap_or_default();
    load_for(r, emit, phase, true, "battery-load");
    let load: Vec<Value> = r.record.lock().unwrap().take().unwrap_or_default();
    if r.stopped() { return Err(r.why()); }
    let after = sensors::battery(Path::new("/sys/class/power_supply"));
    let wh = after["wh"].as_f64().or(b["wh"].as_f64()).unwrap_or(0.0);
    let (lw, hw) = (avg_of(&light, "w"), avg_of(&load, "w"));
    let hours = hw.and_then(|w| parse::battery_hours(wh, w)).ok_or("The battery doesn't report what it draws.")?;
    Ok(measured("battery", hours, json!({"lightWatts": lw, "loadWatts": hw, "lightHours": lw.and_then(|w| parse::battery_hours(wh, w)), "loadHours": hours,
                                          "whLeft": wh, "percent": after["percent"], "fullWh": after["fullWh"], "tool": "Hearth (built in)"})))
}

/// One run (the page's `bench.run` job). args: {mode: quick|full|custom|sustained|battery, tests: [ids] (custom),
/// place (a drive from places()), peer (a paired device's id), minutes (sustained), gpuLoad, onBatteryOk}.
pub fn run(args: &Value, cancel: &AtomicBool, emit: &(dyn Fn(Value) + Sync)) -> Result<Value, String> {
    let mode = match util::arg(args, "mode") { "" => "quick", m => m };
    if !["quick", "full", "custom", "sustained", "battery"].contains(&mode) { return Err("Unknown kind of run".into()); }
    let picked: Vec<String> = args["tests"].as_array().map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect()).unwrap_or_default();
    let plan = parse::plan(mode, &picked);
    if plan.is_empty() { return Err("Pick at least one test.".into()); }
    let full = mode == "full" || args["thorough"] == true;
    let long = mode == "full" || plan.contains(&"sustained") || plan.contains(&"battery");
    let power = sensors::battery(Path::new("/sys/class/power_supply"));
    if power["onBattery"] == true {
        let pct = power["percent"].as_f64().unwrap_or(100.0);
        if long && pct < 20.0 { return Err(format!("The battery is at {pct:.0} %. Plug in the charger for this test.")); }
        if long && args["onBatteryOk"] != true && mode != "battery" { return Err("on-battery: This is a long, heavy test and the device is on battery.".into()); }
    }
    // The drive for the storage test: one of places(), nothing else.
    let ps = places();
    let want = util::arg(args, "place");
    let place = if want.is_empty() { ps.first().and_then(|p| p["path"].as_str()).map(PathBuf::from) } else { ps.iter().filter_map(|p| p["path"].as_str()).find(|p| *p == want).map(PathBuf::from) };
    if plan.contains(&"disk") && place.is_none() { return Err("That drive can't be used for the storage test.".into()); }
    if RUNNING.swap(true, Ordering::SeqCst) { return Err("A benchmark is already running.".into()); }
    struct Free;
    impl Drop for Free { fn drop(&mut self) { RUNNING.store(false, Ordering::SeqCst); } }
    let _free = Free;

    let r = Run::new(cancel);
    let started = now_ms();
    let tests = std::thread::scope(|s| {
        s.spawn(|| monitor(&r, emit));
        let mut tests = serde_json::Map::new();
        for (i, id) in plan.iter().enumerate() {
            if stop_word(&r.stop) || stop_word(cancel) { break; }
            emit(json!({"test": id, "index": i, "count": plan.len(), "state": "running"}));
            let out = match *id {
                "cpu1" => cpu(&r, true, full),
                "cpuN" => cpu(&r, false, full),
                "memBw" => mem_bw(&r, full),
                "memLat" => mem_lat(&r, full),
                "disk" => disk(&r, emit, place.as_deref().unwrap_or(Path::new("/nonexistent")), full),
                "vulkan" | "opengl" | "game" => mark(&r, id, full),
                "inet" => inet(&r, emit),
                "lan" => lan(&r, util::arg(args, "peer")),
                "sustained" => sustained(&r, emit, util::arg_u64(args, "minutes", 10), args["gpuLoad"] == true),
                "battery" => battery(&r, emit, full),
                _ => Err("Unknown test".into()),
            };
            let v = out.unwrap_or_else(|e| json!({"skipped": e}));
            emit(json!({"test": id, "index": i, "count": plan.len(), "state": "done", "result": v}));
            tests.insert(id.to_string(), v);
        }
        r.done.store(true, Ordering::SeqCst);
        Value::Object(tests)
    });
    if stop_word(cancel) { return Err("Stopped. Nothing was saved.".into()); }
    let mut rec = json!({"id": format!("{started:x}"), "at": started, "seconds": (now_ms() - started) / 1000, "mode": mode, "version": util::version(),
                         "device": device(), "tests": tests, "overall": Value::Null, "stopped": r.hot.lock().unwrap().clone()});
    rec["overall"] = json!(parse::overall(&rec["tests"]));
    let mut h = load_history();
    let prev = h.iter().find(|p| p["mode"] == rec["mode"]).cloned();
    h.insert(0, rec.clone());
    h.truncate(KEEP);
    save_history(&h)?;
    write_summary(Some(&rec));
    if let Some(p) = prev { rec["compare"] = parse::compare(&p, &rec); }
    Ok(rec)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_measures() {
        let stop = AtomicBool::new(false);
        let (copy, read) = copy_bandwidth(8, 2, 0.05, &stop).unwrap();
        assert!(copy > 0.0 && read > 0.0);
        let ns = latency_ns(4, 200_000, &stop).unwrap();
        assert!(ns > 0.0 && ns < 10_000.0);
        stop.store(true, Ordering::SeqCst);
        assert!(copy_bandwidth(8, 2, 0.05, &stop).is_none());
        assert!(latency_ns(4, 200_000, &stop).is_none());
    }

    #[test]
    fn storage_folder_stays_put() {
        let root = std::env::temp_dir().join(format!("hearth-bench-tmp-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let d = temp_dir_for(&root).unwrap();
        assert_eq!(d, root.join(TMP_NAME));
        std::fs::write(d.join(TMP_FILE), b"x").unwrap();
        clean(&d);
        assert!(!d.exists());
        // Something else in the folder: only the test file goes, the folder stays with the other file.
        let d = temp_dir_for(&root).unwrap();
        std::fs::write(d.join("keep.txt"), b"x").unwrap();
        clean(&d);
        assert!(d.join("keep.txt").exists());
        // clean() refuses any other folder.
        clean(&root);
        assert!(root.exists());
        std::fs::remove_dir_all(&d).unwrap();
        // A link in the way is refused.
        std::os::unix::fs::symlink("/tmp", root.join(TMP_NAME)).unwrap();
        assert!(temp_dir_for(&root).is_err());
        assert!(temp_dir_for(Path::new("relative")).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn refuses_bad_runs() {
        let stop = AtomicBool::new(false);
        let none = |_: Value| {};
        assert!(run(&json!({"mode": "rm"}), &stop, &none).is_err());
        assert!(run(&json!({"mode": "custom", "tests": []}), &stop, &none).is_err());
        assert!(run(&json!({"mode": "custom", "tests": ["disk"], "place": "/etc"}), &stop, &none).unwrap_err().contains("drive"));
    }

    #[test]
    fn work_is_work() { assert_ne!(work_chunk(1), work_chunk(2)); }
}
