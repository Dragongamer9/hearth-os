//! System Info: a friendly overview of the device. The basics come from hearthfetch's own detection (the same
//! code as the `hearthfetch` command, used as a library); the details it doesn't need (graphics drivers, memory
//! and swap, each drive with its health from Hearth Disks, battery health, each screen's name, firmware) are
//! read here, from /proc and /sys, without running slow programs.

use serde_json::{json, Value};
#[cfg(target_os = "linux")]
use std::path::{Path, PathBuf};

#[cfg(target_os = "linux")]
fn read(p: impl AsRef<Path>) -> Option<String> { std::fs::read_to_string(p).ok() }
#[cfg(target_os = "linux")]
fn line(p: impl AsRef<Path>) -> Option<String> { read(p).map(|s| s.trim().to_string()).filter(|s| !s.is_empty()) }

/// Everything System Info shows, as {sections: [{id, title, rows: [[label, value]], items?}]}.
pub fn overview() -> Result<Value, String> {
    let f = hearthfetch::collect();
    let basic = hearthfetch::to_json(&f);
    let mut v = json!({"fetch": basic, "lines": hearthfetch::lines(&f).into_iter().map(|(k, t)| json!([k, t])).collect::<Vec<_>>()});
    #[cfg(target_os = "linux")]
    {
        v["device"] = device();
        v["os"] = os();
        v["cpu"] = cpu();
        v["gpus"] = json!(gpus());
        v["memory"] = memory();
        v["drives"] = drives();
        v["batteries"] = json!(batteries());
        v["displays"] = json!(displays());
    }
    Ok(v)
}

#[cfg(target_os = "linux")]
fn device() -> Value {
    let dmi = |f: &str| line(format!("/sys/class/dmi/id/{f}")).filter(|s| !s.eq_ignore_ascii_case("To be filled by O.E.M.") && !s.eq_ignore_ascii_case("Default string"));
    let chassis = match dmi("chassis_type").and_then(|c| c.parse::<u32>().ok()) {
        Some(3) | Some(4) | Some(6) | Some(7) | Some(15) | Some(35) => "Desktop PC",
        Some(8) | Some(9) | Some(10) | Some(14) => "Laptop",
        Some(30) | Some(32) => "Tablet or handheld",
        Some(31) => "Convertible",
        Some(13) => "All-in-one",
        Some(36) => "Mini PC",
        _ => "",
    };
    let board = [dmi("board_vendor"), dmi("board_name")].iter().flatten().cloned().collect::<Vec<_>>().join(" ");
    let bios = [dmi("bios_vendor"), dmi("bios_version"), dmi("bios_date").map(|d| format!("({d})"))].iter().flatten().cloned().collect::<Vec<_>>().join(" ");
    json!({"vendor": dmi("sys_vendor"), "product": dmi("product_name"), "version": dmi("product_version"), "family": dmi("product_family"),
        "board": board, "bios": bios,
        "chassis": chassis, "firmware": if Path::new("/sys/firmware/efi").exists() { "UEFI" } else { "BIOS (legacy)" },
        "secureBoot": secure_boot(), "hostname": line("/proc/sys/kernel/hostname")})
}

/// Secure Boot from its EFI variable (the last byte is 1 when it's on).
#[cfg(target_os = "linux")]
fn secure_boot() -> Value {
    let Ok(dir) = std::fs::read_dir("/sys/firmware/efi/efivars") else { return Value::Null };
    for e in dir.flatten() {
        if e.file_name().to_string_lossy().starts_with("SecureBoot-") {
            if let Ok(b) = std::fs::read(e.path()) { return json!(b.last().copied() == Some(1)); }
        }
    }
    Value::Null
}

#[cfg(target_os = "linux")]
fn os() -> Value {
    let osr = hearthfetch::parse::os_release(&read("/etc/os-release").or_else(|| read("/usr/lib/os-release")).unwrap_or_default());
    let image: Value = read("/usr/share/nimbus/image-info.json").and_then(|t| serde_json::from_str(&t).ok()).unwrap_or(Value::Null);
    let base = image["base"].as_str().unwrap_or("");
    json!({
        "name": osr.get("PRETTY_NAME").or(osr.get("NAME")),
        "edition": if base.is_empty() { Value::Null } else { json!(hearthfetch::parse::edition_name(base)) },
        "editionDetail": image["base-detail"], "imageRef": image["image-ref"], "imageTag": image["image-tag"],
        "hearth": crate::util::version(), "toolsVersion": env!("CARGO_PKG_VERSION"),
        "kernel": line("/proc/sys/kernel/osrelease"), "arch": std::env::consts::ARCH,
        "desktop": std::env::var("XDG_CURRENT_DESKTOP").ok(), "session": std::env::var("XDG_SESSION_TYPE").ok(),
        "uptime": read("/proc/uptime").and_then(|t| hearthfetch::parse::uptime_secs(&t)),
        "imageBased": Path::new("/run/ostree-booted").exists(),
    })
}

/// Physical cores: distinct (package, core) pairs in /proc/cpuinfo.
pub fn cores(cpuinfo: &str) -> usize {
    let mut seen = std::collections::HashSet::new();
    let (mut pkg, mut core) = (String::new(), String::new());
    for l in cpuinfo.lines().chain(std::iter::once("")) {
        if l.trim().is_empty() { if !core.is_empty() { seen.insert((pkg.clone(), core.clone())); } pkg.clear(); core.clear(); continue; }
        if let Some((k, v)) = l.split_once(':') {
            match k.trim() { "physical id" => pkg = v.trim().into(), "core id" => core = v.trim().into(), _ => {} }
        }
    }
    seen.len()
}

#[cfg(target_os = "linux")]
fn cpu() -> Value {
    let info = read("/proc/cpuinfo").unwrap_or_default();
    let (name, threads) = hearthfetch::parse::cpuinfo(&info);
    let khz = |f: &str| line(format!("/sys/devices/system/cpu/cpu0/cpufreq/{f}")).and_then(|k| k.parse::<u64>().ok());
    let governor = line("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor");
    let flags = info.lines().find(|l| l.starts_with("flags")).unwrap_or("");
    json!({"name": name.map(|n| hearthfetch::parse::clean_cpu(&n)), "threads": threads, "cores": cores(&info),
        "maxMhz": khz("cpuinfo_max_freq").map(|k| k / 1000), "nowMhz": khz("scaling_cur_freq").map(|k| k / 1000), "governor": governor,
        "virtualization": if flags.contains(" svm") { "AMD-V" } else if flags.contains(" vmx") { "VT-x" } else { "" },
        "load": read("/proc/loadavg").map(|l| l.split_whitespace().take(3).collect::<Vec<_>>().join(" "))})
}

fn hex(s: &str) -> Option<u16> { u16::from_str_radix(s.trim().trim_start_matches("0x"), 16).ok() }

/// "NVRM version: NVIDIA UNIX x86_64 Kernel Module  560.35.03  ..." -> 560.35.03
pub fn nvidia_version(text: &str) -> Option<String> {
    let l = text.lines().find(|l| l.contains("Kernel Module"))?;
    l.split("Kernel Module").nth(1)?.split_whitespace().next().map(str::to_string)
}

/// Mesa's version from its library's file name (libgallium-25.1.4.so), which newer Mesa carries.
#[cfg(target_os = "linux")]
fn mesa_version() -> Option<String> {
    for dir in ["/usr/lib64", "/usr/lib/x86_64-linux-gnu", "/usr/lib"] {
        for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let n = e.file_name().to_string_lossy().into_owned();
            if let Some(v) = n.strip_prefix("libgallium-").and_then(|r| r.strip_suffix(".so")) { return Some(v.to_string()); }
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn gpus() -> Vec<Value> {
    let ids = ["/usr/share/hwdata/pci.ids", "/usr/share/misc/pci.ids", "/usr/share/pci.ids"].iter().find_map(|p| read(p)).unwrap_or_default();
    let mut seen: Vec<PathBuf> = vec![];
    let mut out = vec![];
    let mut cards: Vec<PathBuf> = std::fs::read_dir("/sys/class/drm").into_iter().flatten().flatten()
        .filter(|e| { let n = e.file_name().to_string_lossy().into_owned(); n.len() > 4 && n.starts_with("card") && n[4..].chars().all(|c| c.is_ascii_digit()) })
        .map(|e| e.path()).collect();
    cards.sort();
    let kernel = line("/proc/sys/kernel/osrelease").unwrap_or_default();
    let mesa = mesa_version();
    for card in cards {
        let dev = card.join("device");
        let real = std::fs::canonicalize(&dev).unwrap_or_else(|_| dev.clone());
        if seen.contains(&real) { continue; }
        seen.push(real.clone());
        let uevent = read(dev.join("uevent")).unwrap_or_default();
        let driver = uevent.lines().find_map(|l| l.strip_prefix("DRIVER=")).unwrap_or("").to_string();
        if ["simple-framebuffer", "simpledrm", "efifb", ""].contains(&driver.as_str()) { continue; }
        let (v, d) = (line(dev.join("vendor")).and_then(|s| hex(&s)), line(dev.join("device")).and_then(|s| hex(&s)));
        let name = match (v, d) {
            (Some(v), Some(d)) => { let f = hearthfetch::parse::pci_name(&ids, v, d); hearthfetch::parse::gpu_label(v, f.as_ref().map(|x| x.0.as_str()), f.as_ref().and_then(|x| x.1.as_deref())) }
            _ => driver.clone(),
        };
        let (drv_name, drv_ver) = match driver.as_str() {
            "nvidia" => ("NVIDIA (proprietary)".to_string(), read("/proc/driver/nvidia/version").and_then(|t| nvidia_version(&t)).unwrap_or_default()),
            "nouveau" => ("nouveau (open source)".to_string(), mesa.clone().map(|m| format!("Mesa {m}")).unwrap_or_default()),
            "amdgpu" | "radeon" | "i915" | "xe" => (format!("{driver} (open source)"), mesa.clone().map(|m| format!("Mesa {m}, kernel {kernel}")).unwrap_or_else(|| format!("kernel {kernel}"))),
            other => (other.to_string(), String::new()),
        };
        let vram = line(dev.join("mem_info_vram_total")).and_then(|s| s.parse::<u64>().ok());
        let vram_used = line(dev.join("mem_info_vram_used")).and_then(|s| s.parse::<u64>().ok());
        let busy = line(dev.join("gpu_busy_percent")).and_then(|s| s.parse::<u64>().ok());
        // Built in (shares memory) or its own card: Intel's are built in, Nvidia's are cards; AMD's built-in graphics
        // usually get 1 GB or less (larger amounts set in the firmware can't be told apart here, so then it's not said).
        let integrated: Value = match driver.as_str() {
            "i915" | "xe" => json!(true),
            "nvidia" | "nouveau" => json!(false),
            "amdgpu" if vram.map(|b| b <= 1 << 30).unwrap_or(false) => json!(true),
            "amdgpu" if vram.map(|b| b >= 6 << 30).unwrap_or(false) => json!(false),
            _ => Value::Null,
        };
        let boot_vga = line(dev.join("boot_vga")).as_deref() == Some("1");
        out.push(json!({"name": name, "driver": drv_name, "driverVersion": drv_ver, "vram": vram, "vramUsed": vram_used, "busy": busy,
            "integrated": integrated, "primary": boot_vga, "pci": real.file_name().map(|n| n.to_string_lossy().into_owned())}));
    }
    out
}

/// (total, available, swap total, swap free) in bytes from /proc/meminfo.
pub fn meminfo(text: &str) -> (u64, u64, u64, u64) {
    let get = |k: &str| text.lines().find_map(|l| l.strip_prefix(k).and_then(|r| r.trim_start_matches(':').split_whitespace().next()).and_then(|n| n.parse::<u64>().ok())).unwrap_or(0) * 1024;
    (get("MemTotal"), get("MemAvailable"), get("SwapTotal"), get("SwapFree"))
}

#[cfg(target_os = "linux")]
fn memory() -> Value {
    let (total, avail, st, sf) = meminfo(&read("/proc/meminfo").unwrap_or_default());
    let zram = std::fs::read_dir("/sys/block").into_iter().flatten().flatten().any(|e| e.file_name().to_string_lossy().starts_with("zram"));
    json!({"total": total, "used": total.saturating_sub(avail), "available": avail, "swapTotal": st, "swapUsed": st.saturating_sub(sf), "zram": zram})
}

#[cfg(target_os = "linux")]
fn drives() -> Value {
    match hearth_disks::list() {
        Ok(ds) => json!(ds.iter().filter(|d| d.kind != "image" && d.kind != "optical").map(|d| json!({
            "name": d.name(), "size": d.size, "kind": d.kind, "connection": d.connection, "system": d.system, "removable": d.removable,
            "health": d.health, "device": d.device,
            "partitions": d.partitions.iter().filter(|p| p.kind != "free").map(|p| json!({"label": p.label, "what": p.what, "fs": p.fs, "size": p.size,
                "used": p.used, "mounts": p.mount_points})).collect::<Vec<_>>(),
        })).collect::<Vec<_>>()),
        Err(e) => json!({"error": e}),
    }
}

/// Battery health: what it holds now compared with when it was new, from /sys/class/power_supply.
pub fn health_pct(full: Option<f64>, design: Option<f64>) -> Option<u64> {
    match (full, design) { (Some(f), Some(d)) if d > 0.0 && f > 0.0 => Some(((f / d) * 100.0).round().min(100.0) as u64), _ => None }
}

#[cfg(target_os = "linux")]
fn batteries() -> Vec<Value> {
    let mut out = vec![];
    for e in std::fs::read_dir("/sys/class/power_supply").into_iter().flatten().flatten() {
        let p = e.path();
        if line(p.join("type")).as_deref() != Some("Battery") || line(p.join("scope")).as_deref() == Some("Device") { continue; }
        let num = |f: &str| line(p.join(f)).and_then(|s| s.parse::<f64>().ok());
        let (full, design, now, unit) = if num("energy_full").is_some() { (num("energy_full"), num("energy_full_design"), num("energy_now"), "Wh") }
            else { (num("charge_full"), num("charge_full_design"), num("charge_now"), "Ah") };
        let power = num("power_now").map(|w| w / 1e6);
        out.push(json!({"name": e.file_name().to_string_lossy(), "maker": line(p.join("manufacturer")), "model": line(p.join("model_name")),
            "technology": line(p.join("technology")), "percent": num("capacity"), "status": line(p.join("status")),
            "health": health_pct(full, design), "full": full.map(|x| x / 1e6), "design": design.map(|x| x / 1e6), "now": now.map(|x| x / 1e6), "unit": unit,
            "cycles": num("cycle_count").filter(|c| *c > 0.0), "watts": power}));
    }
    out
}

/// A screen's name and maker from its EDID (the description it sends): (maker code, name, width cm, height cm).
pub fn edid(b: &[u8]) -> Option<(String, Option<String>, u8, u8)> {
    if b.len() < 128 || b[0..8] != [0, 255, 255, 255, 255, 255, 255, 0] { return None; }
    let m = ((b[8] as u16) << 8) | b[9] as u16;
    let letter = |n: u16| (b'A' - 1 + (n & 31) as u8) as char;
    let maker: String = [letter(m >> 10), letter(m >> 5), letter(m)].iter().collect();
    let mut name = None;
    for off in [54usize, 72, 90, 108] {
        let d = &b[off..off + 18];
        if d[0] == 0 && d[1] == 0 && d[3] == 0xFC {
            let s: String = d[5..18].iter().take_while(|c| **c != 0x0A).map(|c| *c as char).collect();
            let s = s.trim().to_string();
            if !s.is_empty() { name = Some(s); }
        }
    }
    Some((maker, name, b[21], b[22]))
}

#[cfg(target_os = "linux")]
fn displays() -> Vec<Value> {
    let mut out: Vec<(String, Value)> = vec![];
    for e in std::fs::read_dir("/sys/class/drm").into_iter().flatten().flatten() {
        let n = e.file_name().to_string_lossy().into_owned();
        if !n.contains('-') || line(e.path().join("status")).as_deref() != Some("connected") { continue; }
        let port = n.split_once('-').map(|x| x.1.to_string()).unwrap_or(n.clone());
        let modes: Vec<String> = read(e.path().join("modes")).unwrap_or_default().lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect();
        let info = std::fs::read(e.path().join("edid")).ok().and_then(|b| edid(&b));
        let inch = info.as_ref().filter(|i| i.2 > 0 && i.3 > 0).map(|i| ((i.2 as f64).powi(2) + (i.3 as f64).powi(2)).sqrt() / 2.54);
        let built_in = port.starts_with("eDP") || port.starts_with("DSI") || port.starts_with("LVDS");
        out.push((port.clone(), json!({"port": port, "builtIn": built_in, "best": modes.first(), "modes": modes.len(),
            "maker": info.as_ref().map(|i| i.0.clone()), "name": info.as_ref().and_then(|i| i.1.clone()), "inches": inch.map(|x| (x * 10.0).round() / 10.0)})));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out.into_iter().map(|x| x.1).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_cores() {
        let c = "processor : 0\nphysical id : 0\ncore id : 0\n\nprocessor : 1\nphysical id : 0\ncore id : 0\n\nprocessor : 2\nphysical id : 0\ncore id : 1\n";
        assert_eq!(cores(c), 2);
    }

    #[test]
    fn memory_and_battery() {
        let (t, a, st, sf) = meminfo("MemTotal:       16000000 kB\nMemFree: 1 kB\nMemAvailable:    8000000 kB\nSwapTotal: 0 kB\nSwapFree: 0 kB\n");
        assert_eq!((t, a, st, sf), (16_000_000 * 1024, 8_000_000 * 1024, 0, 0));
        assert_eq!(health_pct(Some(40.0), Some(50.0)), Some(80));
        assert_eq!(health_pct(Some(55.0), Some(50.0)), Some(100));
        assert_eq!(health_pct(None, Some(50.0)), None);
    }

    #[test]
    fn nvidia() {
        assert_eq!(nvidia_version("NVRM version: NVIDIA UNIX Open Kernel Module for x86_64  580.82.09  Release Build  (dvs-builder@U16-I3-B03-4-1)  Tue Sep  2 2025\nGCC version:"), Some("580.82.09".into()));
    }

    #[test]
    fn edid_name() {
        let mut b = vec![0u8; 128];
        b[0..8].copy_from_slice(&[0, 255, 255, 255, 255, 255, 255, 0]);
        // "DEL" = 4, 5, 12
        let m: u16 = (4 << 10) | (5 << 5) | 12;
        b[8] = (m >> 8) as u8; b[9] = m as u8;
        b[21] = 60; b[22] = 34;
        b[54 + 3] = 0xFC;
        b[54 + 5..54 + 5 + 9].copy_from_slice(b"DELL U27\n");
        let (maker, name, w, h) = edid(&b).unwrap();
        assert_eq!((maker.as_str(), name.as_deref(), w, h), ("DEL", Some("DELL U27"), 60, 34));
        assert!(edid(&[0u8; 10]).is_none());
    }
}
