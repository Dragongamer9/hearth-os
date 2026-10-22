//! Reading the system's own files and putting numbers into words. Nothing here touches the system, so all of it
//! is tested with sample text (the tests at the bottom).

use std::collections::HashMap;

/// /etc/os-release: KEY=value lines, values optionally in quotes (with \" \\ \$ \` escapes inside double quotes).
pub fn os_release(text: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') { continue; }
        let Some((k, v)) = line.split_once('=') else { continue };
        let k = k.trim();
        if k.is_empty() || !k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') { continue; }
        let v = v.trim();
        let value = if let Some(inner) = v.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
            let mut s = String::new();
            let mut chars = inner.chars();
            while let Some(c) = chars.next() {
                if c == '\\' {
                    match chars.next() { Some(n @ ('"' | '\\' | '$' | '`')) => s.push(n), Some(n) => { s.push('\\'); s.push(n); } None => s.push('\\') }
                } else { s.push(c); }
            }
            s
        } else if let Some(inner) = v.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')) {
            inner.to_string()
        } else { v.to_string() };
        out.insert(k.to_string(), value);
    }
    out
}

/// /proc/meminfo: (total, available) in bytes. Older kernels without MemAvailable: free + buffers + cache.
pub fn meminfo(text: &str) -> Option<(u64, u64)> {
    let mut m: HashMap<&str, u64> = HashMap::new();
    for line in text.lines() {
        let Some((k, rest)) = line.split_once(':') else { continue };
        let mut parts = rest.split_whitespace();
        let Some(n) = parts.next().and_then(|n| n.parse::<u64>().ok()) else { continue };
        let mult = match parts.next() { Some("kB") | Some("KB") => 1024, Some("mB") | Some("MB") => 1024 * 1024, None => 1, _ => 1024 };
        m.insert(k.trim(), n * mult);
    }
    let total = *m.get("MemTotal")?;
    if total == 0 { return None; }
    let avail = m.get("MemAvailable").copied().unwrap_or_else(|| {
        m.get("MemFree").copied().unwrap_or(0) + m.get("Buffers").copied().unwrap_or(0) + m.get("Cached").copied().unwrap_or(0)
    });
    Some((total, avail.min(total)))
}

/// /proc/uptime: the first number, whole seconds.
pub fn uptime_secs(text: &str) -> Option<u64> {
    let f: f64 = text.split_whitespace().next()?.parse().ok()?;
    if f.is_finite() && f >= 0.0 { Some(f as u64) } else { None }
}

/// "3 days, 4 hours, 5 mins". Under a minute: "less than a minute".
pub fn format_uptime(secs: u64) -> String {
    let (d, h, m) = (secs / 86_400, secs % 86_400 / 3600, secs % 3600 / 60);
    let unit = |n: u64, one: &str, many: &str| format!("{n} {}", if n == 1 { one } else { many });
    let mut parts = vec![];
    if d > 0 { parts.push(unit(d, "day", "days")); }
    if h > 0 { parts.push(unit(h, "hour", "hours")); }
    if m > 0 { parts.push(unit(m, "min", "mins")); }
    if parts.is_empty() { return "less than a minute".into(); }
    parts.join(", ")
}

/// 16_642_998_272 → "15.5 GiB" (powers of 1024, like the tools people compare against).
pub fn format_bytes(b: u64) -> String {
    const UNITS: [&str; 6] = ["B", "KiB", "MiB", "GiB", "TiB", "PiB"];
    let mut v = b as f64;
    let mut i = 0;
    while v >= 1024.0 && i < UNITS.len() - 1 { v /= 1024.0; i += 1; }
    if i == 0 { format!("{b} B") } else { format!("{v:.1} {}", UNITS[i]) }
}

/// "3.2 GiB / 15.5 GiB (21%)"
pub fn usage(used: u64, total: u64) -> String {
    let pct = if total > 0 { (used as f64 * 100.0 / total as f64).round() as u64 } else { 0 };
    format!("{} / {} ({pct}%)", format_bytes(used), format_bytes(total))
}

/// /proc/cpuinfo: the processor's name and how many threads it runs.
pub fn cpuinfo(text: &str) -> (Option<String>, usize) {
    let mut name = None;
    let mut threads = 0;
    for line in text.lines() {
        let Some((k, v)) = line.split_once(':') else { continue };
        let (k, v) = (k.trim(), v.trim());
        if k == "processor" { threads += 1; }
        if name.is_none() && (k == "model name" || k == "Model" || k == "Hardware" || k == "cpu model") && !v.is_empty() { name = Some(v.to_string()); }
    }
    (name, threads)
}

/// "AMD Ryzen 7 5800X 8-Core Processor" → "AMD Ryzen 7 5800X"; "Intel(R) Core(TM) i7-8700 CPU @ 3.20GHz" → "Intel Core i7-8700".
pub fn clean_cpu(name: &str) -> String {
    let mut s = name.to_string();
    for junk in ["(R)", "(r)", "(TM)", "(tm)", "®", "™"] { s = s.replace(junk, ""); }
    if let Some(i) = s.find(" @ ") { s.truncate(i); }
    let mut words: Vec<&str> = s.split_whitespace().collect();
    // Trailing "CPU", "Processor", "8-Core Processor", "with Radeon Graphics" say nothing the name doesn't.
    if let Some(i) = words.iter().position(|w| *w == "with") { if i > 1 { words.truncate(i); } }
    loop {
        match words.last() {
            Some(w) if *w == "CPU" || *w == "Processor" => { words.pop(); }
            Some(w) if w.ends_with("-Core") || w.ends_with("-core") => { words.pop(); }
            _ => break,
        }
    }
    words.join(" ")
}

/// The short name of a PCI vendor Hearth OS devices are likely to have.
pub fn vendor_short(vendor: u16) -> Option<&'static str> {
    Some(match vendor {
        0x1002 | 0x1022 => "AMD",
        0x10de => "NVIDIA",
        0x8086 => "Intel",
        0x1414 => "Microsoft",
        0x15ad => "VMware",
        0x1af4 => "Virtio",
        0x1234 => "QEMU",
        0x80ee => "VirtualBox",
        0x5143 => "Qualcomm",
        0x1a03 => "ASPEED",
        _ => return None,
    })
}

/// A PCI device's vendor and device names from pci.ids (the file lspci reads).
pub fn pci_name(ids: &str, vendor: u16, device: u16) -> Option<(String, Option<String>)> {
    let vhex = format!("{vendor:04x}");
    let dhex = format!("{device:04x}");
    let mut vendor_name: Option<String> = None;
    for line in ids.lines() {
        if line.starts_with('#') || line.is_empty() { continue; }
        if vendor_name.is_none() {
            if line.starts_with("C ") { return None; }   // device classes: past every vendor
            if line.len() > 6 && line[..4].eq_ignore_ascii_case(&vhex) && line.as_bytes()[4] == b' ' {
                vendor_name = Some(line[4..].trim().to_string());
            }
            continue;
        }
        if line.starts_with("\t\t") { continue; }
        if let Some(rest) = line.strip_prefix('\t') {
            if rest.len() > 6 && rest[..4].eq_ignore_ascii_case(&dhex) && rest.as_bytes()[4] == b' ' {
                return Some((vendor_name.unwrap(), Some(rest[4..].trim().to_string())));
            }
            continue;
        }
        break;   // the next vendor: this device isn't listed
    }
    vendor_name.map(|v| (v, None))
}

/// A graphics card's name for people: "NVIDIA GeForce RTX 3070", "AMD Radeon RX 6800/6800 XT / 6900 XT".
pub fn gpu_label(vendor: u16, vendor_name: Option<&str>, device_name: Option<&str>) -> String {
    let short = vendor_short(vendor).map(str::to_string)
        .or_else(|| vendor_name.map(|v| v.split([',', '[']).next().unwrap_or(v).trim().to_string()))
        .unwrap_or_else(|| format!("{vendor:04x}"));
    let model = device_name.map(|d| {
        // pci.ids names many chips "Codename [Marketing name]": the marketing name is what people know.
        match (d.find('['), d.rfind(']')) { (Some(a), Some(b)) if b > a + 1 => d[a + 1..b].trim().to_string(), _ => d.trim().to_string() }
    });
    match model {
        Some(m) if m.to_lowercase().starts_with(&short.to_lowercase()) => m,
        Some(m) => format!("{short} {m}"),
        None => format!("{short} graphics"),
    }
}

/// /usr/share/nimbus/image-info.json (Hearth OS's editions): (version, base).
pub fn image_info(text: &str) -> Option<(Option<String>, Option<String>)> {
    let v: serde_json::Value = serde_json::from_str(text).ok()?;
    let s = |k: &str| v[k].as_str().map(str::to_string).filter(|s| !s.is_empty());
    Some((s("version").filter(|v| v != "0.0.0"), s("base")))
}

/// Hearth OS's editions by their base's name in image-info.json.
pub fn edition_name(base: &str) -> String {
    match base {
        "anatase" => "Anatase".into(),
        "ubuntu" => "Ubuntu".into(),
        "cachyos" => "CachyOS".into(),
        "peppermint" => "Peppermint".into(),
        other => { let mut c = other.chars(); c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default() }
    }
}

/// The OS line on Linux: "Hearth OS 0.3.1 (Ubuntu edition)" on Hearth OS, the system's own name elsewhere.
pub fn linux_os_line(osr: &HashMap<String, String>, image: Option<&(Option<String>, Option<String>)>, arch: &str) -> String {
    if let Some((ver, base)) = image {
        let mut s = String::from("Hearth OS");
        if let Some(v) = ver { s.push(' '); s.push_str(v); }
        if let Some(b) = base { s.push_str(&format!(" ({} edition)", edition_name(b))); }
        return format!("{s} {arch}");
    }
    let name = osr.get("PRETTY_NAME").cloned()
        .or_else(|| osr.get("NAME").map(|n| format!("{n} {}", osr.get("VERSION").cloned().unwrap_or_default()).trim().to_string()))
        .unwrap_or_else(|| "Linux".into());
    format!("{name} {arch}")
}

/// Values that firmware leaves in its device fields when the maker didn't fill them in.
pub fn dmi_clean(s: &str) -> Option<String> {
    let t = s.trim();
    let l = t.to_lowercase();
    const JUNK: [&str; 16] = ["to be filled by o.e.m.", "default string", "system product name", "system manufacturer", "not applicable",
        "not specified", "none", "o.e.m.", "oem", "type1productconfigid", "invalid", "0123456789", "unknown", "system version", "n/a", "x.x"];
    if t.is_empty() || JUNK.contains(&l.as_str()) || l.starts_with("to be filled") { None } else { Some(t.to_string()) }
}

/// "Valve Jupiter", "ASUSTeK ROG Ally RC71L", "Lenovo ThinkPad X1 Carbon Gen 9" (Lenovo puts the name people know in
/// the version field and a part number in the product field).
pub fn device_model(vendor: Option<&str>, product: Option<&str>, version: Option<&str>) -> Option<String> {
    let vendor = vendor.and_then(dmi_clean);
    let mut product = product.and_then(dmi_clean);
    let version = version.and_then(dmi_clean);
    let lenovo = vendor.as_deref().map(|v| v.eq_ignore_ascii_case("lenovo")).unwrap_or(false);
    if lenovo { if let Some(v) = &version { if v.chars().any(|c| c.is_ascii_alphabetic()) { product = Some(v.clone()); } } }
    let vendor = vendor.map(|v| {
        // Long legal names say the same as the short one.
        let short = v.replace("ASUSTeK COMPUTER INC.", "ASUS").replace("Micro-Star International Co., Ltd.", "MSI")
            .replace("Hewlett-Packard", "HP").replace("Dell Inc.", "Dell").replace("LENOVO", "Lenovo").replace("Gigabyte Technology Co., Ltd.", "Gigabyte");
        short.trim().to_string()
    });
    match (vendor, product) {
        (Some(v), Some(p)) if p.to_lowercase().starts_with(&v.to_lowercase()) => Some(p),
        (Some(v), Some(p)) => Some(format!("{v} {p}")),
        (None, Some(p)) => Some(p),
        (Some(v), None) => Some(v),
        (None, None) => None,
    }
}

/// Windows names Windows 11 "Windows 10" in the registry; the build number tells them apart.
pub fn windows_name(product: &str, build: u32) -> String {
    if build >= 22000 && product.contains("Windows 10") { product.replace("Windows 10", "Windows 11") } else { product.to_string() }
}

/// A Windows program's product version (its version resource), from the file's bytes: "0.3.1".
pub fn pe_product_version(bytes: &[u8]) -> Option<String> {
    let key: Vec<u8> = "ProductVersion".encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
    let mut from = 0;
    while let Some(off) = find(&bytes[from..], &key) {
        let mut i = from + off + key.len();
        // The key ends with a 0 and is padded to 4 bytes; the value follows.
        while i + 1 < bytes.len() && bytes[i] == 0 && bytes[i + 1] == 0 { i += 2; }
        let mut units = vec![];
        while i + 1 < bytes.len() && units.len() < 64 {
            let u = u16::from_le_bytes([bytes[i], bytes[i + 1]]);
            if u == 0 { break; }
            units.push(u);
            i += 2;
        }
        let s = String::from_utf16_lossy(&units);
        let s = s.split('+').next().unwrap_or("").trim().to_string();
        if !s.is_empty() && s.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) { return Some(s); }
        from += off + key.len();
    }
    None
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || hay.len() < needle.len() { return None; }
    hay.windows(needle.len()).position(|w| w == needle)
}

/// Steam's libraryfolders.vdf: every library folder ("path" lines).
pub fn steam_libraries(vdf: &str) -> Vec<String> {
    vdf.lines().filter_map(|l| {
        let l = l.trim();
        let rest = l.strip_prefix("\"path\"")?.trim();
        let v = rest.strip_prefix('"')?.strip_suffix('"')?;
        Some(v.replace("\\\\", "\\"))
    }).collect()
}

/// A program's own name from its path or command line: "/usr/bin/bash" → "bash", "pwsh.exe" → "pwsh".
pub fn program_name(path: &str) -> String {
    let base = path.rsplit(['/', '\\']).next().unwrap_or(path);
    let base = base.strip_prefix('-').unwrap_or(base);   // login shells: "-bash"
    base.strip_suffix(".exe").or_else(|| base.strip_suffix(".EXE")).unwrap_or(base).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn os_release_values() {
        let t = "NAME=\"Hearth\"\nPRETTY_NAME=\"Hearth OS (Ubuntu edition, 26.04 LTS)\"\nID=ubuntu\n# a comment\nVERSION_ID='26.04'\nQUOTE=\"say \\\"hi\\\" \\$HOME\"\nbad line\n=x\n";
        let m = os_release(t);
        assert_eq!(m["NAME"], "Hearth");
        assert_eq!(m["PRETTY_NAME"], "Hearth OS (Ubuntu edition, 26.04 LTS)");
        assert_eq!(m["ID"], "ubuntu");
        assert_eq!(m["VERSION_ID"], "26.04");
        assert_eq!(m["QUOTE"], "say \"hi\" $HOME");
        assert_eq!(m.len(), 5);
    }

    #[test]
    fn os_line_on_hearth_and_elsewhere() {
        let osr = os_release("NAME=Fedora\nPRETTY_NAME=\"Fedora Linux 44 (Workstation Edition)\"\n");
        let img = image_info(r#"{"image-name": "hearth-ubuntu", "base": "ubuntu", "version": "0.3.1"}"#).unwrap();
        assert_eq!(linux_os_line(&osr, Some(&img), "x86_64"), "Hearth OS 0.3.1 (Ubuntu edition) x86_64");
        let img = image_info(r#"{"base": "cachyos", "version": "0.0.0"}"#).unwrap();
        assert_eq!(linux_os_line(&osr, Some(&img), "x86_64"), "Hearth OS (CachyOS edition) x86_64");
        assert_eq!(linux_os_line(&osr, None, "aarch64"), "Fedora Linux 44 (Workstation Edition) aarch64");
        assert_eq!(linux_os_line(&os_release("NAME=Arch\nVERSION=rolling\n"), None, "x86_64"), "Arch rolling x86_64");
        assert_eq!(linux_os_line(&HashMap::new(), None, "x86_64"), "Linux x86_64");
        assert!(image_info("not json").is_none());
        assert_eq!(edition_name("peppermint"), "Peppermint");
        assert_eq!(edition_name("anatase"), "Anatase");
        assert_eq!(edition_name("other"), "Other");
    }

    #[test]
    fn meminfo_numbers() {
        let t = "MemTotal:       16252928 kB\nMemFree:         1000000 kB\nMemAvailable:   12000000 kB\nBuffers:          200000 kB\nCached:          3000000 kB\n";
        assert_eq!(meminfo(t), Some((16252928 * 1024, 12000000 * 1024)));
        // Old kernels: free + buffers + cache.
        let old = "MemTotal: 1000 kB\nMemFree: 100 kB\nBuffers: 50 kB\nCached: 250 kB\n";
        assert_eq!(meminfo(old), Some((1000 * 1024, 400 * 1024)));
        assert_eq!(meminfo("nothing here"), None);
        assert_eq!(meminfo("MemTotal: 0 kB\n"), None);
    }

    #[test]
    fn uptime_words() {
        assert_eq!(uptime_secs("350735.47 234388.90\n"), Some(350735));
        assert_eq!(uptime_secs("garbage"), None);
        assert_eq!(uptime_secs(""), None);
        assert_eq!(format_uptime(30), "less than a minute");
        assert_eq!(format_uptime(60), "1 min");
        assert_eq!(format_uptime(3600 + 120), "1 hour, 2 mins");
        assert_eq!(format_uptime(86_400), "1 day");
        assert_eq!(format_uptime(350735), "4 days, 1 hour, 25 mins");
    }

    #[test]
    fn bytes_and_usage() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1536), "1.5 KiB");
        assert_eq!(format_bytes(16_642_998_272), "15.5 GiB");
        assert_eq!(usage(4 << 30, 16 << 30), "4.0 GiB / 16.0 GiB (25%)");
        assert_eq!(usage(0, 0), "0 B / 0 B (0%)");
    }

    #[test]
    fn cpu_names() {
        let t = "processor\t: 0\nmodel name\t: AMD Ryzen 7 5800X 8-Core Processor\nprocessor\t: 1\nmodel name\t: AMD Ryzen 7 5800X 8-Core Processor\n";
        let (n, threads) = cpuinfo(t);
        assert_eq!(threads, 2);
        assert_eq!(clean_cpu(&n.unwrap()), "AMD Ryzen 7 5800X");
        assert_eq!(clean_cpu("Intel(R) Core(TM) i7-8700 CPU @ 3.20GHz"), "Intel Core i7-8700");
        assert_eq!(clean_cpu("AMD Custom APU 0405"), "AMD Custom APU 0405");
        assert_eq!(clean_cpu("AMD Ryzen Z1 Extreme w/ Radeon 780M Graphics"), "AMD Ryzen Z1 Extreme w/ Radeon 780M Graphics");
        assert_eq!(clean_cpu("AMD Ryzen 7 7840U with Radeon 780M Graphics"), "AMD Ryzen 7 7840U");
        assert_eq!(cpuinfo("processor : 0\nHardware : Qualcomm\n").0.as_deref(), Some("Qualcomm"));
    }

    #[test]
    fn pci_ids_lookup() {
        let ids = "# comment\n1002  Advanced Micro Devices, Inc. [AMD/ATI]\n\t1435  Custom GPU 0405\n\t73bf  Navi 21 [Radeon RX 6800/6800 XT / 6900 XT]\n\t\t1002 0e3a  Radeon RX 6900 XT\n10de  NVIDIA Corporation\n\t2484  GA104 [GeForce RTX 3070]\n8086  Intel Corporation\n\t46a6  Alder Lake-P GT2 [Iris Xe Graphics]\nC 00  Unclassified device\n";
        let (v, d) = pci_name(ids, 0x1002, 0x73bf).unwrap();
        assert_eq!(gpu_label(0x1002, Some(&v), d.as_deref()), "AMD Radeon RX 6800/6800 XT / 6900 XT");
        let (v, d) = pci_name(ids, 0x1002, 0x1435).unwrap();
        assert_eq!(gpu_label(0x1002, Some(&v), d.as_deref()), "AMD Custom GPU 0405");
        let (v, d) = pci_name(ids, 0x10de, 0x2484).unwrap();
        assert_eq!(gpu_label(0x10de, Some(&v), d.as_deref()), "NVIDIA GeForce RTX 3070");
        let (_, d) = pci_name(ids, 0x8086, 0x46a6).unwrap();
        assert_eq!(gpu_label(0x8086, None, d.as_deref()), "Intel Iris Xe Graphics");
        // A device the file doesn't list: the vendor only.
        assert_eq!(pci_name(ids, 0x10de, 0x9999), Some(("NVIDIA Corporation".into(), None)));
        assert_eq!(gpu_label(0x10de, Some("NVIDIA Corporation"), None), "NVIDIA graphics");
        assert_eq!(pci_name(ids, 0x1234, 0x1111), None);
        assert_eq!(gpu_label(0x1b36, Some("Red Hat, Inc."), Some("QXL paravirtual graphic card")), "Red Hat QXL paravirtual graphic card");
    }

    #[test]
    fn device_names() {
        assert_eq!(device_model(Some("Valve"), Some("Jupiter"), Some("1")).as_deref(), Some("Valve Jupiter"));
        assert_eq!(device_model(Some("ASUSTeK COMPUTER INC."), Some("ROG Ally RC71L_RC71L"), Some("1.0")).as_deref(), Some("ASUS ROG Ally RC71L_RC71L"));
        assert_eq!(device_model(Some("LENOVO"), Some("20XW0026US"), Some("ThinkPad X1 Carbon Gen 9")).as_deref(), Some("Lenovo ThinkPad X1 Carbon Gen 9"));
        assert_eq!(device_model(Some("To Be Filled By O.E.M."), Some("To Be Filled By O.E.M."), None), None);
        assert_eq!(device_model(Some("System manufacturer"), Some("System Product Name"), None), None);
        assert_eq!(device_model(Some("Micro-Star International Co., Ltd."), Some("MS-7C02"), None).as_deref(), Some("MSI MS-7C02"));
        assert_eq!(device_model(Some("Dell Inc."), Some("Dell G15 5511"), None).as_deref(), Some("Dell G15 5511"));
    }

    #[test]
    fn windows_bits() {
        assert_eq!(windows_name("Windows 10 Pro", 22631), "Windows 11 Pro");
        assert_eq!(windows_name("Windows 10 Home", 19045), "Windows 10 Home");
        // A version resource as Windows stores it: UTF-16 key, padding, UTF-16 value.
        let mut b = vec![0u8; 40];
        b.extend("ProductVersion".encode_utf16().flat_map(|u| u.to_le_bytes()));
        b.extend([0, 0, 0, 0]);
        b.extend("0.3.1+abc123".encode_utf16().flat_map(|u| u.to_le_bytes()));
        b.extend([0, 0, 1, 2]);
        assert_eq!(pe_product_version(&b).as_deref(), Some("0.3.1"));
        assert_eq!(pe_product_version(b"no version here"), None);
    }

    #[test]
    fn steam_and_programs() {
        let vdf = "\"libraryfolders\"\n{\n\t\"0\"\n\t{\n\t\t\"path\"\t\t\"/home/deck/.local/share/Steam\"\n\t}\n\t\"1\"\n\t{\n\t\t\"path\"\t\t\"D:\\\\SteamLibrary\"\n\t}\n}\n";
        assert_eq!(steam_libraries(vdf), vec!["/home/deck/.local/share/Steam".to_string(), "D:\\SteamLibrary".to_string()]);
        assert_eq!(program_name("/usr/bin/bash"), "bash");
        assert_eq!(program_name("-zsh"), "zsh");
        assert_eq!(program_name("C:\\Program Files\\PowerShell\\7\\pwsh.exe"), "pwsh");
    }
}
