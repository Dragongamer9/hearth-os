//! What Settings › About and › System show: OS, device, processor, graphics, memory, storage, uptime.
//! Port of hosts/linux/src/sysinfo.js (same shape as the Windows host).

use serde_json::{json, Value};
use std::path::Path;

fn read(root: &Path, f: &str) -> String { std::fs::read_to_string(root.join(f.trim_start_matches('/'))).map(|s| s.trim().to_string()).unwrap_or_default() }

pub fn os_release(root: &Path) -> std::collections::HashMap<String, String> {
    read(root, "/etc/os-release").lines().filter_map(|l| l.split_once('=')).map(|(k, v)| (k.to_string(), v.trim_matches(|c| c == '"' || c == '\'').to_string())).collect()
}

pub fn cpu_name(root: &Path) -> String {
    read(root, "/proc/cpuinfo").lines().find(|l| l.starts_with("model name")).and_then(|l| l.split_once(':')).map(|(_, v)| v.split_whitespace().collect::<Vec<_>>().join(" ")).unwrap_or_default()
}

pub fn gpus(root: &Path) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut cards: Vec<String> = std::fs::read_dir(root.join("sys/class/drm")).map(|rd| rd.flatten().map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.starts_with("card") && n[4..].chars().all(|c| c.is_ascii_digit()) && n.len() > 4).collect()).unwrap_or_default();
    cards.sort();
    for c in cards {
        let dev = format!("/sys/class/drm/{c}/device");
        let vendor = read(root, &format!("{dev}/vendor"));
        if vendor.is_empty() { continue; }
        let named = read(root, &format!("{dev}/product_name"));
        let name = if !named.is_empty() { named } else { match vendor.as_str() { "0x1002" => "AMD".into(), "0x10de" => "NVIDIA".into(), "0x8086" => "Intel".into(), v => v.into() } };
        let vram: u64 = read(root, &format!("{dev}/mem_info_vram_total")).parse().unwrap_or(0);
        let label = if vram > 0 { format!("{name} ({} GB)", (vram as f64 / 1073741824.0).round()) } else { name };
        if !out.contains(&label) { out.push(label); }
    }
    out
}

fn storage(dir: &Path) -> Value {
    // statvfs through `df` keeps this crate free of system bindings.
    let out = crate::system::run("df", &["-B1", "--output=size,avail", &dir.to_string_lossy()]).unwrap_or_default();
    let nums: Vec<u64> = out.lines().nth(1).map(|l| l.split_whitespace().filter_map(|n| n.parse().ok()).collect()).unwrap_or_default();
    if nums.len() == 2 { json!({"path": dir, "totalBytes": nums[0], "freeBytes": nums[1]}) } else { Value::Null }
}

pub fn info(root: &Path) -> Value {
    let rel = os_release(root);
    let vendor = read(root, "/sys/class/dmi/id/sys_vendor");
    let product = read(root, "/sys/class/dmi/id/product_name");
    let device = [vendor, product].iter().filter(|s| !s.is_empty()).cloned().collect::<Vec<_>>().join(" ").replacen("ASUSTeK COMPUTER INC.", "ASUS", 1);
    let mem_kb: u64 = read(root, "/proc/meminfo").lines().find(|l| l.starts_with("MemTotal:")).and_then(|l| l.split_whitespace().nth(1)).and_then(|n| n.parse().ok()).unwrap_or(0);
    let uptime: f64 = read(root, "/proc/uptime").split_whitespace().next().and_then(|n| n.parse().ok()).unwrap_or(0.0);
    let cores = read(root, "/proc/cpuinfo").lines().filter(|l| l.starts_with("processor")).count();
    json!({
        "os": rel.get("NAME").cloned().unwrap_or_else(|| "Linux".into()),
        "osVersion": rel.get("VERSION").or(rel.get("VERSION_ID")).cloned().unwrap_or_default(),
        "osPretty": rel.get("PRETTY_NAME").cloned().unwrap_or_default(),
        "device": device, "cpu": cpu_name(root), "cores": cores, "gpu": gpus(root).join(", "),
        "memoryBytes": mem_kb * 1024, "storage": storage(&crate::paths::home()), "uptimeSec": uptime.round() as u64,
        "kernel": read(root, "/proc/sys/kernel/osrelease"), "hostname": read(root, "/proc/sys/kernel/hostname"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reads_the_device() {
        let d = tempfile::tempdir().unwrap();
        let put = |f: &str, v: &str| { let p = d.path().join(f); std::fs::create_dir_all(p.parent().unwrap()).unwrap(); std::fs::write(p, v).unwrap(); };
        put("etc/os-release", "NAME=\"Hearth\"\nPRETTY_NAME=\"Hearth OS (based on Bazzite)\"\nVERSION_ID=43\n");
        put("proc/cpuinfo", "processor\t: 0\nmodel name\t: AMD Ryzen   Z1 Extreme\nprocessor\t: 1\n");
        put("sys/class/dmi/id/sys_vendor", "ASUSTeK COMPUTER INC.\n"); put("sys/class/dmi/id/product_name", "ROG Ally RC71L\n");
        put("sys/class/drm/card0/device/vendor", "0x1002\n"); put("sys/class/drm/card0/device/mem_info_vram_total", "4294967296");
        put("sys/class/drm/card1/device/vendor", "0x10de\n"); put("sys/class/drm/card1/device/product_name", "GeForce RTX 4070");
        let i = info(d.path());
        assert_eq!(i["os"], "Hearth"); assert_eq!(i["osVersion"], "43");
        assert_eq!(i["device"], "ASUS ROG Ally RC71L");
        assert_eq!(i["cpu"], "AMD Ryzen Z1 Extreme"); assert_eq!(i["cores"], 2);
        assert_eq!(i["gpu"], "AMD (4 GB), GeForce RTX 4070");
    }
}
