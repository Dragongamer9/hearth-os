//! Windows: the registry (what WMI reads its answers from too), and a few system calls that answer at once.
//! No PowerShell, no WMI queries: those take seconds.

use crate::{parse, Info};
use std::ffi::c_void;
use std::path::{Path, PathBuf};

type Hkey = isize;
// Predefined keys are 32-bit values sign-extended to the pointer size.
const HKEY_CURRENT_USER: Hkey = 0x8000_0001u32 as i32 as isize;
const HKEY_LOCAL_MACHINE: Hkey = 0x8000_0002u32 as i32 as isize;
const RRF_RT_REG_SZ: u32 = 0x0000_0002;
const RRF_RT_REG_DWORD: u32 = 0x0000_0010;
const KEY_READ: u32 = 0x2_0019;

#[repr(C)]
#[allow(dead_code)]
struct MemoryStatusEx { length: u32, load: u32, total_phys: u64, avail_phys: u64, total_page: u64, avail_page: u64, total_virtual: u64, avail_virtual: u64, avail_ext: u64 }

#[repr(C)]
#[allow(dead_code)]
struct PowerStatus { ac_line: u8, battery_flag: u8, battery_percent: u8, saver: u8, life_time: u32, full_life_time: u32 }

#[repr(C)]
#[allow(dead_code)]
struct DisplayDevice { cb: u32, name: [u16; 32], string: [u16; 128], state: u32, id: [u16; 128], key: [u16; 128] }

#[repr(C)]
#[allow(dead_code)]
struct ProcessEntry { size: u32, usage: u32, pid: u32, heap: usize, module: u32, threads: u32, ppid: u32, priority: i32, flags: u32, exe: [u16; 260] }

#[link(name = "advapi32")]
extern "system" {
    fn RegGetValueW(key: Hkey, sub: *const u16, value: *const u16, flags: u32, kind: *mut u32, data: *mut c_void, size: *mut u32) -> i32;
}
#[link(name = "kernel32")]
extern "system" {
    fn GlobalMemoryStatusEx(m: *mut MemoryStatusEx) -> i32;
    fn GetTickCount64() -> u64;
    fn GetDiskFreeSpaceExW(dir: *const u16, free_caller: *mut u64, total: *mut u64, free: *mut u64) -> i32;
    fn GetSystemPowerStatus(s: *mut PowerStatus) -> i32;
    fn GetStdHandle(which: u32) -> isize;
    fn GetConsoleMode(h: isize, mode: *mut u32) -> i32;
    fn SetConsoleMode(h: isize, mode: u32) -> i32;
    fn CreateToolhelp32Snapshot(flags: u32, pid: u32) -> isize;
    fn Process32FirstW(snap: isize, e: *mut ProcessEntry) -> i32;
    fn Process32NextW(snap: isize, e: *mut ProcessEntry) -> i32;
    fn CloseHandle(h: isize) -> i32;
}
#[link(name = "user32")]
extern "system" {
    fn EnumDisplayDevicesW(device: *const u16, n: u32, d: *mut DisplayDevice, flags: u32) -> i32;
    fn EnumDisplaySettingsW(device: *const u16, mode: u32, dm: *mut u8) -> i32;
}

fn wide(s: &str) -> Vec<u16> { s.encode_utf16().chain(std::iter::once(0)).collect() }
fn from_wide(w: &[u16]) -> String { String::from_utf16_lossy(&w[..w.iter().position(|c| *c == 0).unwrap_or(w.len())]) }

fn reg_sz(key: Hkey, sub: &str, value: &str) -> Option<String> {
    let (s, v) = (wide(sub), wide(value));
    let mut size = 0u32;
    unsafe {
        if RegGetValueW(key, s.as_ptr(), v.as_ptr(), RRF_RT_REG_SZ, std::ptr::null_mut(), std::ptr::null_mut(), &mut size) != 0 || size == 0 { return None; }
        let mut buf = vec![0u16; (size as usize + 1) / 2 + 1];
        let mut size2 = (buf.len() * 2) as u32;
        if RegGetValueW(key, s.as_ptr(), v.as_ptr(), RRF_RT_REG_SZ, std::ptr::null_mut(), buf.as_mut_ptr() as *mut c_void, &mut size2) != 0 { return None; }
        Some(from_wide(&buf).trim().to_string()).filter(|s| !s.is_empty())
    }
}

fn reg_dword(key: Hkey, sub: &str, value: &str) -> Option<u32> {
    let (s, v) = (wide(sub), wide(value));
    let mut data = 0u32;
    let mut size = 4u32;
    let r = unsafe { RegGetValueW(key, s.as_ptr(), v.as_ptr(), RRF_RT_REG_DWORD, std::ptr::null_mut(), &mut data as *mut u32 as *mut c_void, &mut size) };
    if r == 0 { Some(data) } else { None }
}

/// Ember's colours: Windows 10 and 11 consoles understand them once asked.
pub fn enable_ansi() -> bool {
    unsafe {
        let h = GetStdHandle(-11i32 as u32);   // STD_OUTPUT_HANDLE
        let mut mode = 0u32;
        if GetConsoleMode(h, &mut mode) == 0 { return false; }
        mode & 0x4 != 0 || SetConsoleMode(h, mode | 0x4) != 0   // ENABLE_VIRTUAL_TERMINAL_PROCESSING
    }
}

pub fn collect() -> Info {
    const NT: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion";
    const BIOS: &str = r"HARDWARE\DESCRIPTION\System\BIOS";
    const CPU: &str = r"HARDWARE\DESCRIPTION\System\CentralProcessor\0";
    let build: u32 = reg_sz(HKEY_LOCAL_MACHINE, NT, "CurrentBuildNumber").and_then(|b| b.parse().ok()).unwrap_or(0);
    let ubr = reg_dword(HKEY_LOCAL_MACHINE, NT, "UBR");
    let os = reg_sz(HKEY_LOCAL_MACHINE, NT, "ProductName").map(|p| {
        let mut s = parse::windows_name(&p, build);
        if let Some(d) = reg_sz(HKEY_LOCAL_MACHINE, NT, "DisplayVersion") { s.push(' '); s.push_str(&d); }
        if build > 0 { s.push_str(&format!(" (build {build}{})", ubr.map(|u| format!(".{u}")).unwrap_or_default())); }
        format!("{s} {}", std::env::consts::ARCH)
    });
    let kernel = match (reg_dword(HKEY_LOCAL_MACHINE, NT, "CurrentMajorVersionNumber"), reg_dword(HKEY_LOCAL_MACHINE, NT, "CurrentMinorVersionNumber")) {
        (Some(a), Some(b)) if build > 0 => Some(format!("Windows NT {a}.{b}.{build}{}", ubr.map(|u| format!(".{u}")).unwrap_or_default())),
        _ => None,
    };
    let device = parse::device_model(reg_sz(HKEY_LOCAL_MACHINE, BIOS, "SystemManufacturer").as_deref(),
        reg_sz(HKEY_LOCAL_MACHINE, BIOS, "SystemProductName").as_deref(), reg_sz(HKEY_LOCAL_MACHINE, BIOS, "SystemVersion").as_deref());
    let (gpus, displays) = screens();
    let (shell, terminal) = shell_and_terminal();
    Info {
        user: std::env::var("USERNAME").ok().filter(|s| !s.is_empty()),
        host: std::env::var("COMPUTERNAME").ok().filter(|s| !s.is_empty()),
        os,
        kernel,
        uptime: Some(unsafe { GetTickCount64() } / 1000),
        device,
        cpu: reg_sz(HKEY_LOCAL_MACHINE, CPU, "ProcessorNameString"),
        cpu_threads: std::thread::available_parallelism().map(|n| n.get()).unwrap_or(0),
        cpu_mhz: reg_dword(HKEY_LOCAL_MACHINE, CPU, "~MHz"),
        gpus,
        memory: memory(),
        disk: disk(),
        displays,
        battery: battery(),
        shell,
        terminal,
        hearth: ember(),
        games: games(),
    }
}

fn memory() -> Option<(u64, u64)> {
    let mut m = MemoryStatusEx { length: std::mem::size_of::<MemoryStatusEx>() as u32, load: 0, total_phys: 0, avail_phys: 0, total_page: 0, avail_page: 0, total_virtual: 0, avail_virtual: 0, avail_ext: 0 };
    if unsafe { GlobalMemoryStatusEx(&mut m) } == 0 || m.total_phys == 0 { return None; }
    Some((m.total_phys.saturating_sub(m.avail_phys), m.total_phys))
}

fn disk() -> Option<(String, u64, u64)> {
    let drive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into());
    let root = format!("{}\\", drive.trim_end_matches('\\'));
    let (mut free_caller, mut total, mut free) = (0u64, 0u64, 0u64);
    if unsafe { GetDiskFreeSpaceExW(wide(&root).as_ptr(), &mut free_caller, &mut total, &mut free) } == 0 || total == 0 { return None; }
    Some((drive, total.saturating_sub(free), total))
}

fn battery() -> Option<(u8, String)> {
    let mut s = PowerStatus { ac_line: 0, battery_flag: 0, battery_percent: 0, saver: 0, life_time: 0, full_life_time: 0 };
    if unsafe { GetSystemPowerStatus(&mut s) } == 0 { return None; }
    // 128: no battery; 255: unknown.
    if s.battery_flag & 128 != 0 || s.battery_flag == 255 || s.battery_percent > 100 { return None; }
    let status = if s.battery_flag & 8 != 0 { "Charging" } else if s.ac_line == 1 { if s.battery_percent >= 100 { "Full" } else { "Plugged in" } } else { "Discharging" };
    Some((s.battery_percent, status.to_string()))
}

/// Graphics adapters and the screens on the desktop (size and refresh rate), from Windows' display list.
fn screens() -> (Vec<String>, Vec<String>) {
    let (mut gpus, mut displays) = (vec![], vec![]);
    for n in 0..16u32 {
        let mut d = DisplayDevice { cb: std::mem::size_of::<DisplayDevice>() as u32, name: [0; 32], string: [0; 128], state: 0, id: [0; 128], key: [0; 128] };
        if unsafe { EnumDisplayDevicesW(std::ptr::null(), n, &mut d, 0) } == 0 { break; }
        let adapter = from_wide(&d.string).trim().to_string();
        let mirroring = d.state & 0x8 != 0;   // DISPLAY_DEVICE_MIRRORING_DRIVER
        if !adapter.is_empty() && !mirroring && !gpus.contains(&adapter) && adapter != "Microsoft Basic Render Driver" { gpus.push(adapter); }
        if d.state & 0x1 == 0 { continue; }   // DISPLAY_DEVICE_ATTACHED_TO_DESKTOP
        // DEVMODEW: 220 bytes; dmSize at 68, dmPelsWidth at 172, dmPelsHeight at 176, dmDisplayFrequency at 184.
        let mut dm = [0u32; 55];
        let bytes = dm.as_mut_ptr() as *mut u8;
        unsafe { *(bytes.add(68) as *mut u16) = 220; }
        if unsafe { EnumDisplaySettingsW(d.name.as_ptr(), 0xFFFF_FFFF, bytes) } == 0 { continue; }   // ENUM_CURRENT_SETTINGS
        let (w, h, hz) = (dm[172 / 4], dm[176 / 4], dm[184 / 4]);
        if w == 0 || h == 0 { continue; }
        displays.push(if hz > 1 { format!("{w}x{h} @ {hz} Hz") } else { format!("{w}x{h}") });
    }
    // Basic display (no driver yet) only when it's all there is.
    if gpus.len() > 1 { gpus.retain(|g| !g.starts_with("Microsoft Basic Display")); }
    (gpus, displays)
}

/// Every process's (parent, program name), once.
fn processes() -> Vec<(u32, u32, String)> {
    let mut out = vec![];
    unsafe {
        let snap = CreateToolhelp32Snapshot(0x2, 0);   // TH32CS_SNAPPROCESS
        if snap == -1 || snap == 0 { return out; }
        let mut e: ProcessEntry = std::mem::zeroed();
        e.size = std::mem::size_of::<ProcessEntry>() as u32;
        if Process32FirstW(snap, &mut e) != 0 {
            loop {
                out.push((e.pid, e.ppid, from_wide(&e.exe)));
                if Process32NextW(snap, &mut e) == 0 { break; }
            }
        }
        CloseHandle(snap);
    }
    out
}

/// The shell is the program that started hearthfetch; the terminal is the first one above it that isn't a shell.
fn shell_and_terminal() -> (Option<String>, Option<String>) {
    const SHELLS: [&str; 7] = ["pwsh", "powershell", "cmd", "bash", "nu", "sh", "zsh"];
    let ps = processes();
    let find = |pid: u32| ps.iter().find(|p| p.0 == pid);
    let me = std::process::id();
    let mut shell = None;
    let mut terminal = None;
    let mut cur = find(me).map(|p| p.1);
    for _ in 0..8 {
        let Some(pid) = cur else { break };
        let Some(p) = find(pid) else { break };
        let name = parse::program_name(&p.2);
        let lower = name.to_lowercase();
        if SHELLS.contains(&lower.as_str()) {
            if shell.is_none() { shell = Some(match lower.as_str() { "pwsh" => "PowerShell 7".into(), "powershell" => "Windows PowerShell".into(), "cmd" => "Command Prompt".into(), _ => name }); }
        } else {
            terminal = Some(match lower.as_str() {
                "windowsterminal" | "openconsole" => "Windows Terminal".into(),
                "explorer" | "nimbus" | "conhost" => "Windows Console".into(),
                "code" => "Visual Studio Code".into(),
                _ => name,
            });
            break;
        }
        cur = Some(p.1);
    }
    if std::env::var_os("WT_SESSION").is_some() { terminal = Some("Windows Terminal".into()); }
    (shell, terminal)
}

/// Ember (Hearth for Windows): beside hearthfetch (it comes with Ember, in Ember's bin folder) or where its
/// installer puts it. The version is Nimbus.exe's own.
fn ember() -> Option<String> {
    let mut places: Vec<PathBuf> = vec![];
    if let Ok(me) = std::env::current_exe() {
        if let Some(dir) = me.parent() { places.push(dir.join("Nimbus.exe")); places.push(dir.join("..").join("Nimbus.exe")); }
    }
    if let Some(local) = std::env::var_os("LOCALAPPDATA") { places.push(Path::new(&local).join("Nimbus").join("app").join("Nimbus.exe")); }
    let exe = places.into_iter().find(|p| p.is_file())?;
    let version = std::fs::read(&exe).ok().and_then(|b| parse::pe_product_version(&b));
    Some(match version { Some(v) => format!("Ember {v}"), None => "Ember".into() })
}

/// Steam games installed (its install records), in every Steam library folder.
fn games() -> Option<usize> {
    let steam = reg_sz(HKEY_CURRENT_USER, r"Software\Valve\Steam", "SteamPath")?;
    let vdf = std::fs::read_to_string(Path::new(&steam).join("steamapps").join("libraryfolders.vdf")).ok()?;
    let mut libs = parse::steam_libraries(&vdf);
    if libs.is_empty() { libs.push(steam); }
    let mut n = 0;
    let mut seen: Vec<String> = vec![];
    for lib in libs {
        let key = lib.to_lowercase().replace('/', "\\");
        if seen.contains(&key) { continue; }
        seen.push(key);
        for e in std::fs::read_dir(Path::new(&lib).join("steamapps")).into_iter().flatten().flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if !(name.starts_with("appmanifest_") && name.ends_with(".acf")) { continue; }
            let text = std::fs::read_to_string(e.path()).unwrap_or_default();
            let title = text.lines().find_map(|l| l.trim().strip_prefix("\"name\"")).unwrap_or("").trim().trim_matches('"').to_string();
            if title.starts_with("Steamworks Common") { continue; }
            n += 1;
        }
    }
    Some(n)
}
