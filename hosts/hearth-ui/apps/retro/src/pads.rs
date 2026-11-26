//! The controllers plugged in or paired, in the order the kernel has them (Player 1, Player 2…), from
//! /proc/bus/input/devices (the ones with a joystick handler), each with its battery where the kernel reports one
//! (/sys/class/power_supply's device batteries). Only reads.

#[derive(Debug, Clone, PartialEq)]
pub struct Pad {
    pub name: String,
    pub battery: Option<u8>,
}

/// Steam's and Hearth's own virtual controllers aren't players (they mirror a real one).
fn virtual_pad(name: &str) -> bool {
    let n = name.to_lowercase();
    n.starts_with("hearth ") || n.contains("steam virtual") || n.contains("virtual gamepad") || n.contains("uinput")
}

fn friendly(name: &str) -> String {
    let n = name.trim();
    let l = n.to_lowercase();
    if l.contains("xbox") && (l.contains("wireless") || l.contains("one") || l.contains("series")) {
        return "Xbox Wireless Controller".into();
    }
    if l.contains("x-box 360") || l.contains("xbox 360") {
        return "Xbox 360 Controller".into();
    }
    if l.contains("dualsense") {
        return "DualSense".into();
    }
    if l.contains("wireless controller") && l.contains("sony") {
        return "DualShock 4".into();
    }
    n.to_string()
}

pub fn list() -> Vec<Pad> {
    if let Some(a) = crate::fixture().get("pads").and_then(|v| v.as_array()) {
        return a
            .iter()
            .map(|p| Pad { name: p.get("name").and_then(|x| x.as_str()).unwrap_or("").to_string(), battery: p.get("battery").and_then(|x| x.as_u64()).map(|b| b as u8) })
            .collect();
    }
    let Ok(text) = std::fs::read_to_string("/proc/bus/input/devices") else { return vec![] };
    let bats = batteries();
    let mut out = Vec::new();
    for block in text.split("\n\n") {
        let mut name = String::new();
        let mut sysfs = String::new();
        let mut js = false;
        for line in block.lines() {
            if let Some(n) = line.strip_prefix("N: Name=") {
                name = n.trim_matches('"').to_string();
            } else if let Some(s) = line.strip_prefix("S: Sysfs=") {
                sysfs = format!("/sys{}", s.trim());
            } else if let Some(h) = line.strip_prefix("H: Handlers=") {
                js = h.split_whitespace().any(|x| x.starts_with("js"));
            }
        }
        if !js || name.is_empty() || virtual_pad(&name) {
            continue;
        }
        let real = std::fs::canonicalize(&sysfs).map(|p| p.display().to_string()).unwrap_or(sysfs);
        let battery = bats.iter().find(|(dev, _)| !dev.is_empty() && real.starts_with(dev.as_str())).map(|x| x.1);
        out.push(Pad { name: friendly(&name), battery });
    }
    out
}

/// (the device the battery belongs to, its level) for every device battery (not the laptop's own).
fn batteries() -> Vec<(String, u8)> {
    let mut v = Vec::new();
    let Ok(rd) = std::fs::read_dir("/sys/class/power_supply") else { return v };
    for e in rd.flatten() {
        let p = e.path();
        let scope = std::fs::read_to_string(p.join("scope")).unwrap_or_default();
        if scope.trim() != "Device" {
            continue;
        }
        let Some(cap) = std::fs::read_to_string(p.join("capacity")).ok().and_then(|c| c.trim().parse::<u8>().ok()) else { continue };
        let dev = std::fs::canonicalize(p.join("device")).map(|d| d.display().to_string()).unwrap_or_default();
        v.push((dev, cap));
    }
    v
}

/// This PC's own battery (the top bar's "82%"), None on a PC without one.
pub fn own_battery() -> Option<u8> {
    if let Some(b) = crate::fixture().get("battery").and_then(|v| v.as_u64()) {
        return Some(b as u8);
    }
    let rd = std::fs::read_dir("/sys/class/power_supply").ok()?;
    for e in rd.flatten() {
        let p = e.path();
        if std::fs::read_to_string(p.join("type")).map(|t| t.trim() == "Battery").unwrap_or(false)
            && std::fs::read_to_string(p.join("scope")).map(|s| s.trim() != "Device").unwrap_or(true)
        {
            if let Some(c) = std::fs::read_to_string(p.join("capacity")).ok().and_then(|c| c.trim().parse::<u8>().ok()) {
                return Some(c);
            }
        }
    }
    None
}
