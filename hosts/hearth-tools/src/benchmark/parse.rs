//! Hearth Benchmark: reading the benchmark programs' output, and turning measurements into scores. No I/O here,
//! so all of it is unit tested on real output samples.
//!
//! Scores: every test's measurement divided by the same measurement on a reference handheld (an AMD Zen 2 / RDNA 2
//! four-core handheld with LPDDR5 and a PCIe 3 NVMe drive, at its default power limit), times 1000. So 1000 means
//! "as fast as that handheld", 2000 twice as fast. The overall score is the geometric mean of the device tests'
//! scores (network and battery aren't the device's speed, so they're shown but not counted). Quick and Full runs
//! use different GPU scene lists, so runs are only compared with runs of the same kind.

use serde_json::{json, Value};

/// One test the app knows: (id, name, unit, reference value, higher is better, counts in the overall score).
pub struct TestDef { pub id: &'static str, pub name: &'static str, pub unit: &'static str, pub reference: f64, pub higher: bool, pub overall: bool }

pub const TESTS: [TestDef; 12] = [
    TestDef { id: "cpu1", name: "Processor, one core", unit: "MIPS", reference: 4200.0, higher: true, overall: true },
    TestDef { id: "cpuN", name: "Processor, all cores", unit: "MIPS", reference: 26000.0, higher: true, overall: true },
    TestDef { id: "memBw", name: "Memory speed", unit: "GB/s", reference: 25.0, higher: true, overall: true },
    TestDef { id: "memLat", name: "Memory latency", unit: "ns", reference: 110.0, higher: false, overall: true },
    TestDef { id: "disk", name: "Storage", unit: "MB/s", reference: 1.0, higher: true, overall: true },
    TestDef { id: "vulkan", name: "Graphics, Vulkan", unit: "points", reference: 3000.0, higher: true, overall: true },
    TestDef { id: "opengl", name: "Graphics, OpenGL", unit: "points", reference: 3000.0, higher: true, overall: true },
    TestDef { id: "game", name: "Game-style 3D scenes", unit: "FPS", reference: 120.0, higher: true, overall: true },
    TestDef { id: "sustained", name: "Sustained load and heat", unit: "% kept", reference: 100.0, higher: true, overall: true },
    TestDef { id: "battery", name: "Battery life estimate", unit: "h", reference: 2.0, higher: true, overall: false },
    TestDef { id: "inet", name: "Internet speed", unit: "Mbit/s", reference: 1.0, higher: true, overall: false },
    TestDef { id: "lan", name: "Home network to another Hearth device", unit: "Mbit/s", reference: 1.0, higher: true, overall: false },
];

pub fn def(id: &str) -> Option<&'static TestDef> { TESTS.iter().find(|t| t.id == id) }

/// The tests of a Quick and a Full run (Full adds the game scenes and the network; the long tests are their own runs).
pub const QUICK: [&str; 7] = ["cpu1", "cpuN", "memBw", "memLat", "disk", "vulkan", "opengl"];
pub const FULL: [&str; 10] = ["cpu1", "cpuN", "memBw", "memLat", "disk", "vulkan", "opengl", "game", "inet", "lan"];

/// The tests a run does, from what the page asked: a mode, or the picked tests (unknown ids dropped, the order kept
/// the same as TESTS so a custom run reads like the others).
pub fn plan(mode: &str, picked: &[String]) -> Vec<&'static str> {
    match mode {
        "quick" => QUICK.to_vec(),
        "full" => FULL.to_vec(),
        "sustained" => vec!["sustained"],
        "battery" => vec!["battery"],
        _ => TESTS.iter().map(|t| t.id).filter(|id| picked.iter().any(|p| p == id)).collect(),
    }
}

/// Score of one measurement against the reference (1000 = the reference handheld). Storage, internet and the home
/// network score from their parts (disk_score, net_score), so their `value` is already the ratio.
pub fn score(id: &str, value: f64) -> Option<u64> {
    let d = def(id)?;
    if !value.is_finite() || value <= 0.0 { return None; }
    let r = if d.higher { value / d.reference } else { d.reference / value };
    Some((r * 1000.0).round().max(1.0) as u64)
}

/// Geometric mean of positive ratios.
pub fn geomean(v: &[f64]) -> Option<f64> {
    let v: Vec<f64> = v.iter().copied().filter(|x| x.is_finite() && *x > 0.0).collect();
    if v.is_empty() { return None; }
    Some((v.iter().map(|x| x.ln()).sum::<f64>() / v.len() as f64).exp())
}

/// Storage: sequential read and write (MB/s) and random 4 KB read and write (IOPS) against the reference NVMe
/// drive, as one ratio (geometric mean).
pub fn disk_ratio(seq_read: f64, seq_write: f64, rand_read: f64, rand_write: f64) -> Option<f64> {
    geomean(&[seq_read / 1500.0, seq_write / 1000.0, rand_read / 60000.0, rand_write / 40000.0])
}

/// Internet (against 100 down / 20 up) and home network (against 400 / 400): download and upload in Mbit/s.
pub fn net_ratio(down: f64, up: f64, lan: bool) -> Option<f64> {
    let (rd, ru) = if lan { (400.0, 400.0) } else { (100.0, 20.0) };
    geomean(&[down / rd, up / ru])
}

/// The overall score: geometric mean of the counted tests' scores.
pub fn overall(tests: &Value) -> Option<u64> {
    let scores: Vec<f64> = TESTS.iter().filter(|d| d.overall).filter_map(|d| tests.get(d.id)?.get("score")?.as_f64()).collect();
    geomean(&scores).map(|g| g.round() as u64)
}

/// This run against an earlier one: per test, the change of its score in percent (+ is faster), and the overall.
pub fn compare(prev: &Value, cur: &Value) -> Value {
    let pct = |a: f64, b: f64| if a > 0.0 { ((b - a) / a * 1000.0).round() / 10.0 } else { 0.0 };
    let mut tests = serde_json::Map::new();
    for d in TESTS.iter() {
        let (Some(a), Some(b)) = (prev["tests"][d.id]["score"].as_f64(), cur["tests"][d.id]["score"].as_f64()) else { continue };
        tests.insert(d.id.to_string(), json!({"before": a, "after": b, "change": pct(a, b)}));
    }
    let o = match (prev["overall"].as_f64(), cur["overall"].as_f64()) { (Some(a), Some(b)) => json!({"before": a, "after": b, "change": pct(a, b)}), _ => Value::Null };
    json!({"against": prev["id"], "at": prev["at"], "version": prev["version"], "tests": tests, "overall": o})
}

// ---------------------------------------------------------------- the programs' output

/// 7-Zip's `7z b`: the rating (MIPS) on its last "Tot:" line, e.g. "Tot:     397   5874  23385".
pub fn sevenzip(out: &str) -> Option<f64> {
    let line = out.lines().rev().find(|l| l.trim_start().starts_with("Tot:"))?;
    line.split_whitespace().skip(1).filter_map(|w| w.parse::<f64>().ok()).last().filter(|v| *v > 0.0)
}

/// glmark2's and vkmark's per-scene line: "[build] use-vbo=false: FPS: 1234 FrameTime: 0.810 ms" -> 1234.
pub fn mark_fps(line: &str) -> Option<f64> {
    let rest = &line[line.find("FPS:")? + 4..];
    rest.split_whitespace().next()?.parse::<f64>().ok().filter(|v| v.is_finite() && *v >= 0.0)
}

/// glmark2's and vkmark's final "glmark2 Score: 2345" / "vkmark Score: 2345".
pub fn mark_score(out: &str) -> Option<f64> {
    let line = out.lines().rev().find(|l| l.contains("Score:"))?;
    line[line.find("Score:")? + 6..].split_whitespace().next()?.parse::<f64>().ok().filter(|v| *v > 0.0)
}

/// What fio measured (--output-format=json): (bytes per second, IOPS) of the reads and the writes, all jobs together.
pub fn fio(json_text: &str) -> Option<((f64, f64), (f64, f64))> {
    // fio may print warnings before the JSON.
    let start = json_text.find('{')?;
    let v: Value = serde_json::from_str(&json_text[start..]).ok()?;
    let jobs = v["jobs"].as_array()?;
    if jobs.is_empty() { return None; }
    let side = |k: &str| -> (f64, f64) {
        jobs.iter().fold((0.0, 0.0), |(b, i), j| {
            let s = &j[k];
            let bw = s["bw_bytes"].as_f64().or_else(|| s["bw"].as_f64().map(|kib| kib * 1024.0)).unwrap_or(0.0);
            (b + bw, i + s["iops"].as_f64().unwrap_or(0.0))
        })
    };
    Some((side("read"), side("write")))
}

/// nvidia-smi --query-gpu=temperature.gpu,clocks.gr,power.draw --format=csv,noheader,nounits: "62, 1500, 45.20".
pub fn nvidia_query(out: &str) -> (Option<f64>, Option<f64>, Option<f64>) {
    let mut it = out.lines().next().unwrap_or("").split(',').map(|s| s.trim().parse::<f64>().ok());
    (it.next().flatten(), it.next().flatten(), it.next().flatten())
}

/// Steam's libraryfolders.vdf: every library's "path".
pub fn steam_libraries(vdf: &str) -> Vec<String> {
    vdf.lines().filter_map(|l| {
        let t = l.trim();
        let rest = t.strip_prefix("\"path\"")?.trim();
        let p = rest.trim_matches('"').replace("\\\\", "\\");
        (p.starts_with('/')).then_some(p)
    }).collect()
}

/// How much of its starting speed a device kept under a long load: the work done per second over the last fifth of
/// the run against its best minute near the start (after 10 s of warming up). 100 = no slowing down.
pub fn kept_percent(rates: &[f64]) -> Option<f64> {
    if rates.len() < 30 { return None; }
    let start: Vec<f64> = rates.iter().skip(10).take(60).copied().collect();
    let peak = start.iter().sum::<f64>() / start.len() as f64;
    let tail = &rates[rates.len() - (rates.len() / 5).max(5)..];
    let end = tail.iter().sum::<f64>() / tail.len() as f64;
    (peak > 0.0).then(|| ((end / peak * 100.0).min(100.0) * 10.0).round() / 10.0)
}

/// Hours a battery lasts: watt-hours left over watts drawn.
pub fn battery_hours(wh: f64, watts: f64) -> Option<f64> {
    (wh > 0.0 && watts > 0.3).then(|| ((wh / watts) * 10.0).round() / 10.0)
}

/// Every nth sample so a long run's graph has at most `max` points.
pub fn thin<T: Clone>(v: &[T], max: usize) -> Vec<T> {
    if v.len() <= max || max == 0 { return v.to_vec(); }
    let step = v.len() as f64 / max as f64;
    (0..max).map(|i| v[((i as f64 * step) as usize).min(v.len() - 1)].clone()).collect()
}

/// The temperature at which a test is stopped: 95 °C, or a few degrees under the sensor's own critical limit
/// when that's lower.
pub fn heat_limit(crit: Option<f64>) -> f64 {
    match crit { Some(c) if c > 60.0 && c - 3.0 < 95.0 => c - 3.0, _ => 95.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEVENZIP: &str = "7-Zip 23.01 (x64) : Copyright (c) 1999-2023 Igor Pavlov : 2023-06-20

Compiler: 13.2.0 GCC 13.2.0: SSE2
Linux : 6.17.4 : #1 SMP PREEMPT_DYNAMIC : x86_64
CPU Freq: 64000000 - - - - - - - -

RAM size:   15626 MB,  # CPU hardware threads:  16
RAM usage:   3559 MB,  # Benchmark threads:     16

                       Compressing  |                  Decompressing
Dict     Speed Usage    R/U Rating  |      Speed Usage    R/U Rating
         KiB/s     %   MIPS   MIPS  |      KiB/s     %   MIPS   MIPS

22:      62453  1341   4530  60756  |     774812  1564   4226  66103
23:      60123  1388   4413  61258  |     763116  1580   4180  66042
----------------------------------  | ------------------------------
Avr:     61288  1364   4471  61007  |     768964  1572   4203  66072
Tot:            1468   4337  63540
";

    #[test]
    fn reads_7zip() {
        assert_eq!(sevenzip(SEVENZIP), Some(63540.0));
        assert_eq!(sevenzip("Tot:     99   3800   3790\n"), Some(3790.0));
        assert_eq!(sevenzip("no benchmark here"), None);
    }

    #[test]
    fn reads_marks() {
        assert_eq!(mark_fps("[build] use-vbo=false: FPS: 1234 FrameTime: 0.810 ms"), Some(1234.0));
        assert_eq!(mark_fps("[vertex] device-local=true: FPS: 4563 FrameTime: 0.219 ms"), Some(4563.0));
        assert_eq!(mark_fps("=== glmark2 2023.01 ==="), None);
        let gl = "=======================================================\n    glmark2 2023.01\n[build] use-vbo=true: FPS: 900 FrameTime: 1.1 ms\n=======================================================\n                                  glmark2 Score: 2711 \n=======================================================\n";
        assert_eq!(mark_score(gl), Some(2711.0));
        assert_eq!(mark_score("=================\n                                   vkmark Score: 5120\n=================\n"), Some(5120.0));
        assert_eq!(mark_score("Error: no Vulkan device"), None);
    }

    #[test]
    fn reads_fio() {
        let out = r#"fio: note: both iodepth >= 1 and synchronous I/O engine are selected
{
  "fio version" : "fio-3.36",
  "jobs" : [ { "jobname" : "hearth", "read" : { "bw_bytes" : 2147483648, "bw" : 2097152, "iops" : 2048.0 },
               "write" : { "bw_bytes" : 0, "bw" : 0, "iops" : 0.0 } } ]
}"#;
        let ((rb, ri), (wb, wi)) = fio(out).unwrap();
        assert_eq!((rb, ri, wb, wi), (2147483648.0, 2048.0, 0.0, 0.0));
        // Older fio: only "bw" in KiB/s.
        let old = r#"{"jobs":[{"read":{"bw":1024,"iops":256},"write":{"bw":2048,"iops":512}},{"read":{"bw":1024,"iops":256},"write":{"bw":0,"iops":0}}]}"#;
        assert_eq!(fio(old).unwrap(), ((2097152.0, 512.0), (2097152.0, 512.0)));
        assert!(fio("fio: engine libaio not loadable").is_none());
    }

    #[test]
    fn reads_nvidia_and_steam() {
        assert_eq!(nvidia_query("62, 1500, 45.20\n"), (Some(62.0), Some(1500.0), Some(45.2)));
        assert_eq!(nvidia_query("62, [N/A], [N/A]"), (Some(62.0), None, None));
        let vdf = "\"libraryfolders\"\n{\n\t\"0\"\n\t{\n\t\t\"path\"\t\t\"/home/sam/.local/share/Steam\"\n\t}\n\t\"1\"\n\t{\n\t\t\"path\"\t\t\"/run/media/sam/Games/SteamLibrary\"\n\t\t\"label\"\t\t\"\"\n\t}\n}\n";
        assert_eq!(steam_libraries(vdf), vec!["/home/sam/.local/share/Steam".to_string(), "/run/media/sam/Games/SteamLibrary".into()]);
    }

    #[test]
    fn scores() {
        assert_eq!(score("cpuN", 26000.0), Some(1000));
        assert_eq!(score("cpu1", 8400.0), Some(2000));
        assert_eq!(score("memLat", 55.0), Some(2000)); // lower is better
        assert_eq!(score("memLat", 0.0), None);
        assert_eq!(score("nope", 1.0), None);
        assert_eq!(score("disk", disk_ratio(1500.0, 1000.0, 60000.0, 40000.0).unwrap()), Some(1000));
        assert_eq!(score("inet", net_ratio(400.0, 80.0, false).unwrap()), Some(4000));
        assert!((geomean(&[1.0, 4.0]).unwrap() - 2.0).abs() < 1e-9);
        assert_eq!(geomean(&[]), None);
        let tests = json!({"cpu1": {"score": 1000}, "cpuN": {"score": 4000}, "inet": {"score": 99999}, "vulkan": {"skipped": "no Vulkan"}});
        assert_eq!(overall(&tests), Some(2000)); // the internet doesn't count
        assert_eq!(overall(&json!({})), None);
    }

    #[test]
    fn compares_runs() {
        let a = json!({"id": "a", "at": 1, "version": "0.6.5", "overall": 1000, "tests": {"cpu1": {"score": 1000}, "disk": {"score": 800}}});
        let b = json!({"id": "b", "overall": 1100, "tests": {"cpu1": {"score": 1100}, "disk": {"skipped": "x"}, "vulkan": {"score": 5}}});
        let c = compare(&a, &b);
        assert_eq!(c["tests"]["cpu1"]["change"], json!(10.0));
        assert!(c["tests"].get("disk").is_none());
        assert_eq!(c["overall"]["change"], json!(10.0));
        assert_eq!(c["version"], json!("0.6.5"));
    }

    #[test]
    fn plans() {
        assert_eq!(plan("quick", &[]).len(), 7);
        assert!(plan("full", &[]).contains(&"lan"));
        assert_eq!(plan("custom", &["opengl".into(), "cpu1".into(), "rm -rf".into()]), vec!["cpu1", "opengl"]);
        assert_eq!(plan("sustained", &[]), vec!["sustained"]);
    }

    #[test]
    fn sustained_and_battery() {
        let mut r = vec![100.0; 120];
        for (i, x) in r.iter_mut().enumerate().skip(70) { *x = if i >= 96 { 80.0 } else { 90.0 }; }
        assert_eq!(kept_percent(&r), Some(80.0));
        assert_eq!(kept_percent(&[1.0; 10]), None);
        assert_eq!(kept_percent(&vec![50.0; 100]), Some(100.0));
        assert_eq!(battery_hours(40.0, 20.0), Some(2.0));
        assert_eq!(battery_hours(40.0, 0.0), None);
        assert_eq!(thin(&(0..1000).collect::<Vec<_>>(), 10).len(), 10);
        assert_eq!(thin(&[1, 2, 3], 10), vec![1, 2, 3]);
        assert_eq!(heat_limit(None), 95.0);
        assert_eq!(heat_limit(Some(90.0)), 87.0);
        assert_eq!(heat_limit(Some(105.0)), 95.0);
    }
}
