//! Linux: everything from /proc, /sys and a few files under /usr and /etc. No programs are run.

use crate::{parse, Info};
use std::fs;
use std::path::{Path, PathBuf};

fn read(p: impl AsRef<Path>) -> Option<String> { fs::read_to_string(p).ok() }
fn line(p: impl AsRef<Path>) -> Option<String> { read(p).map(|s| s.trim().to_string()).filter(|s| !s.is_empty()) }
fn home() -> PathBuf { std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/")) }

pub fn collect() -> Info {
    let osr = parse::os_release(&read("/etc/os-release").or_else(|| read("/usr/lib/os-release")).unwrap_or_default());
    let image = read("/usr/share/nimbus/image-info.json").and_then(|t| parse::image_info(&t));
    let (cpu, threads) = parse::cpuinfo(&read("/proc/cpuinfo").unwrap_or_default());
    let shell = std::env::var("SHELL").ok().filter(|s| !s.is_empty()).map(|s| parse::program_name(&s));
    Info {
        user: user(),
        host: line("/proc/sys/kernel/hostname"),
        os: Some(parse::linux_os_line(&osr, image.as_ref(), std::env::consts::ARCH)),
        kernel: line("/proc/sys/kernel/osrelease").map(|k| format!("Linux {k}")),
        uptime: read("/proc/uptime").and_then(|t| parse::uptime_secs(&t)),
        device: device(),
        cpu,
        cpu_threads: threads,
        cpu_mhz: line("/sys/devices/system/cpu/cpu0/cpufreq/cpuinfo_max_freq").and_then(|k| k.parse::<u64>().ok()).map(|k| (k / 1000) as u32),
        gpus: gpus(),
        memory: read("/proc/meminfo").and_then(|t| parse::meminfo(&t)).map(|(t, a)| (t - a, t)),
        disk: disk(),
        displays: displays(),
        battery: battery(),
        terminal: terminal(shell.as_deref()),
        shell,
        hearth: hearth(image.as_ref()),
        games: games(),
    }
}

fn user() -> Option<String> {
    if let Some(u) = std::env::var("USER").ok().or_else(|| std::env::var("LOGNAME").ok()).filter(|u| !u.is_empty()) { return Some(u); }
    // From the password database (a service or a bare shell without $USER).
    unsafe {
        let pw = libc::getpwuid(libc::getuid());
        if pw.is_null() || (*pw).pw_name.is_null() { return None; }
        std::ffi::CStr::from_ptr((*pw).pw_name).to_str().ok().map(str::to_string)
    }
}

fn device() -> Option<String> {
    let dmi = |f: &str| line(format!("/sys/class/dmi/id/{f}"));
    parse::device_model(dmi("sys_vendor").as_deref(), dmi("product_name").as_deref(), dmi("product_version").as_deref())
        // Arm boards name themselves in the device tree.
        .or_else(|| read("/sys/firmware/devicetree/base/model").map(|s| s.trim_end_matches('\0').trim().to_string()).filter(|s| !s.is_empty()))
}

fn hex(p: impl AsRef<Path>) -> Option<u16> { line(p).and_then(|s| u16::from_str_radix(s.trim_start_matches("0x"), 16).ok()) }

/// Graphics: every card in /sys/class/drm, named from pci.ids (the list lspci reads).
fn gpus() -> Vec<String> {
    let mut ids: Option<String> = None;
    let mut seen = vec![];
    let mut out = vec![];
    let mut cards: Vec<PathBuf> = fs::read_dir("/sys/class/drm").into_iter().flatten().flatten()
        .filter(|e| { let n = e.file_name().to_string_lossy().to_string(); n.starts_with("card") && n[4..].chars().all(|c| c.is_ascii_digit()) && n.len() > 4 })
        .map(|e| e.path()).collect();
    cards.sort();
    for card in cards {
        let dev = card.join("device");
        let real = fs::canonicalize(&dev).unwrap_or_else(|_| dev.clone());
        if seen.contains(&real) { continue; }
        seen.push(real);
        let uevent = read(dev.join("uevent")).unwrap_or_default();
        let driver = uevent.lines().find_map(|l| l.strip_prefix("DRIVER=")).unwrap_or("").to_string();
        if driver == "simple-framebuffer" || driver == "simpledrm" || driver == "efifb" { continue; }
        match (hex(dev.join("vendor")), hex(dev.join("device"))) {
            (Some(v), Some(d)) => {
                if ids.is_none() {
                    ids = ["/usr/share/hwdata/pci.ids", "/usr/share/misc/pci.ids", "/usr/share/pci.ids"].iter().find_map(|p| read(p)).or(Some(String::new()));
                }
                let found = parse::pci_name(ids.as_deref().unwrap_or(""), v, d);
                out.push(parse::gpu_label(v, found.as_ref().map(|f| f.0.as_str()), found.as_ref().and_then(|f| f.1.as_deref())));
            }
            _ if !driver.is_empty() => out.push(driver),
            _ => {}
        }
    }
    out
}

/// The system drive. On Hearth OS (bootc) "/" is the read-only image; the drive itself is mounted at /sysroot.
fn disk() -> Option<(String, u64, u64)> {
    for p in ["/sysroot", "/"] {
        if !Path::new(p).is_dir() { continue; }
        let c = std::ffi::CString::new(p).ok()?;
        let mut s: libc::statvfs = unsafe { std::mem::zeroed() };
        if unsafe { libc::statvfs(c.as_ptr(), &mut s) } != 0 { continue; }
        let frag = if s.f_frsize > 0 { s.f_frsize as u64 } else { s.f_bsize as u64 };
        let total = s.f_blocks as u64 * frag;
        if total == 0 { continue; }
        let free = s.f_bfree as u64 * frag;
        return Some(("/".into(), total.saturating_sub(free), total));
    }
    None
}

/// Connected screens and their preferred size (the first mode the kernel lists).
fn displays() -> Vec<String> {
    let mut outs: Vec<(String, String)> = fs::read_dir("/sys/class/drm").into_iter().flatten().flatten().filter_map(|e| {
        let n = e.file_name().to_string_lossy().to_string();
        if !n.contains('-') || line(e.path().join("status")).as_deref() != Some("connected") { return None; }
        let mode = read(e.path().join("modes"))?.lines().next()?.trim().to_string();
        if mode.is_empty() { return None; }
        let port = n.split_once('-').map(|x| x.1.to_string()).unwrap_or(n);
        Some((port, mode))
    }).collect();
    outs.sort();
    outs.into_iter().map(|(port, mode)| if port.starts_with("eDP") || port.starts_with("DSI") { format!("{mode} (built-in)") } else { mode }).collect()
}

fn battery() -> Option<(u8, String)> {
    for e in fs::read_dir("/sys/class/power_supply").ok()?.flatten() {
        let p = e.path();
        if line(p.join("type")).as_deref() != Some("Battery") { continue; }
        // Controllers and mice report batteries too ("Device" scope): only the device's own counts.
        if line(p.join("scope")).as_deref() == Some("Device") { continue; }
        let Some(pct) = line(p.join("capacity")).and_then(|c| c.parse::<u8>().ok()) else { continue };
        let status = match line(p.join("status")).as_deref() {
            Some("Charging") => "Charging", Some("Discharging") => "Discharging", Some("Full") => "Full",
            Some("Not charging") => "Plugged in", _ => "",
        };
        return Some((pct.min(100), status.to_string()));
    }
    None
}

/// The parent of a process and its name, from /proc/<pid>/stat.
fn parent(pid: u32) -> Option<(u32, String)> {
    let stat = read(format!("/proc/{pid}/stat"))?;
    let (l, r) = (stat.find('(')?, stat.rfind(')')?);
    let name = stat[l + 1..r].to_string();
    let ppid = stat[r + 1..].split_whitespace().nth(1)?.parse().ok()?;
    Some((ppid, name))
}

/// The terminal: the first program above the shell that isn't another shell (or sudo, su...).
fn terminal(shell: Option<&str>) -> Option<String> {
    if let Ok(t) = std::env::var("TERM_PROGRAM") { if !t.is_empty() { return Some(pretty_terminal(&t)); } }
    const SKIP: [&str; 14] = ["bash", "zsh", "fish", "sh", "dash", "nu", "ksh", "tcsh", "sudo", "su", "doas", "login", "hearthfetch", "run0"];
    let mut pid = std::process::id();
    for _ in 0..10 {
        let (ppid, _) = parent(pid)?;
        if ppid <= 1 { break; }
        let (_, name) = parent(ppid)?;
        if !(SKIP.contains(&name.as_str()) || Some(name.as_str()) == shell || name.ends_with("sh")) {
            return Some(pretty_terminal(&name));
        }
        pid = ppid;
    }
    if std::env::var("TERM").as_deref() == Ok("linux") { return Some("Linux console".into()); }
    None
}

fn pretty_terminal(n: &str) -> String {
    match n {
        "konsole" => "Konsole".into(),
        "gnome-terminal-" | "gnome-terminal-server" | "gnome-terminal" => "GNOME Terminal".into(),
        "ptyxis-agent" | "ptyxis" => "Ptyxis".into(),
        "kgx" => "Console".into(),
        "foot" | "footclient" => "foot".into(),
        "alacritty" => "Alacritty".into(),
        "kitty" => "kitty".into(),
        "wezterm-gui" => "WezTerm".into(),
        "sshd" | "sshd-session" => "SSH".into(),
        "vscode" | "code" => "Visual Studio Code".into(),
        "xfce4-terminal" => "Xfce Terminal".into(),
        "qterminal" => "QTerminal".into(),
        other => other.to_string(),
    }
}

/// Hearth (Game Mode): the app that's part of Hearth OS, with the image's version.
fn hearth(image: Option<&(Option<String>, Option<String>)>) -> Option<String> {
    let installed = ["/usr/lib/nimbus/nimbus", "/usr/bin/nimbus", "/usr/local/bin/nimbus"].iter().any(|p| Path::new(p).exists());
    if !installed { return None; }
    Some(match image.and_then(|i| i.0.clone()) { Some(v) => format!("Hearth {v}"), None => "Hearth".into() })
}

/// Installed games: Steam's (its install records, without Proton and its runtimes) and the ones you added in Hearth.
fn games() -> Option<usize> {
    let h = home();
    let mut n = 0usize;
    let mut found = false;
    for root in [h.join(".local/share/Steam"), h.join(".steam/steam"), h.join(".var/app/com.valvesoftware.Steam/.local/share/Steam")] {
        let Some(vdf) = read(root.join("steamapps/libraryfolders.vdf")) else { continue };
        found = true;
        let mut libs = parse::steam_libraries(&vdf);
        if libs.is_empty() { libs.push(root.to_string_lossy().to_string()); }
        let mut seen = vec![];
        for lib in libs {
            let dir = PathBuf::from(&lib).join("steamapps");
            let real = fs::canonicalize(&dir).unwrap_or(dir.clone());
            if seen.contains(&real) { continue; }
            seen.push(real);
            for e in fs::read_dir(&dir).into_iter().flatten().flatten() {
                let name = e.file_name().to_string_lossy().to_string();
                if !(name.starts_with("appmanifest_") && name.ends_with(".acf")) { continue; }
                let text = read(e.path()).unwrap_or_default();
                let title = text.lines().find_map(|l| l.trim().strip_prefix("\"name\"")).unwrap_or("").trim().trim_matches('"').to_string();
                if title.starts_with("Proton") || title.starts_with("Steam Linux Runtime") || title.starts_with("Steamworks Common") { continue; }
                n += 1;
            }
        }
        break;
    }
    if let Some(lib) = read(h.join(".local/share/nimbus/library.json")).and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok()) {
        if let Some(m) = lib["manual"].as_array() { n += m.len(); found = true; }
    }
    if found { Some(n) } else { None }
}
