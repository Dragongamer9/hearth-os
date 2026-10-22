//! hearthfetch: Hearth OS's own summary of the device for the terminal, like fastfetch or neofetch. Hearth's mark on
//! the left, the system on the right, for Hearth OS (Linux) and for Windows with Ember.
//! Everything comes from files and system calls that answer at once (no slow programs are run), and anything
//! that can't be read is left out without a word.

mod logo;
mod parse;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(windows)]
mod windows;

use logo::Colors;
use serde_json::{json, Map, Value};
use std::io::{IsTerminal, Write};

const HELP: &str = "hearthfetch: a short summary of this device and Hearth OS

Usage: hearthfetch [options]

  --json            print the summary as JSON (for scripts)
  --no-logo         leave out Hearth's mark
  --color <when>    auto (the default), always, 256 or never
  --time            say how long it took (on the error output)
  -h, --help        show this help
  -V, --version     show hearthfetch's version

Colors are left out when the output isn't a terminal or NO_COLOR is set.";

/// What hearthfetch found. Anything it couldn't read stays empty and isn't shown.
#[derive(Default, Debug)]
pub struct Info {
    pub user: Option<String>,
    pub host: Option<String>,
    pub os: Option<String>,
    pub kernel: Option<String>,
    pub uptime: Option<u64>,
    pub device: Option<String>,
    pub cpu: Option<String>,
    pub cpu_threads: usize,
    /// The processor's top speed in MHz.
    pub cpu_mhz: Option<u32>,
    pub gpus: Vec<String>,
    /// (used, total) bytes.
    pub memory: Option<(u64, u64)>,
    /// (where, used, total) for the system drive.
    pub disk: Option<(String, u64, u64)>,
    pub displays: Vec<String>,
    /// (percent, "Charging" / "Discharging" / "Full" ...).
    pub battery: Option<(u8, String)>,
    pub shell: Option<String>,
    pub terminal: Option<String>,
    /// Hearth (Game Mode) on Linux, Ember on Windows, with its version.
    pub hearth: Option<String>,
    pub games: Option<usize>,
}

#[cfg(target_os = "linux")]
fn collect() -> Info { linux::collect() }
#[cfg(windows)]
fn collect() -> Info { windows::collect() }
#[cfg(not(any(target_os = "linux", windows)))]
fn collect() -> Info { Info { os: Some(std::env::consts::OS.to_string()), ..Default::default() } }

#[cfg(windows)]
fn enable_ansi() -> bool { windows::enable_ansi() }
#[cfg(not(windows))]
fn enable_ansi() -> bool { true }

/// The label and text of each line, in order.
pub fn lines(i: &Info) -> Vec<(&'static str, String)> {
    let mut out = vec![];
    let mut add = |k: &'static str, v: Option<String>| if let Some(v) = v.filter(|v| !v.trim().is_empty()) { out.push((k, v)); };
    add("OS", i.os.clone());
    add("Device", i.device.clone());
    add("Kernel", i.kernel.clone());
    add("Uptime", i.uptime.map(parse::format_uptime));
    add("Hearth", i.hearth.clone());
    add("CPU", i.cpu.as_ref().map(|c| {
        let mut s = parse::clean_cpu(c);
        if i.cpu_threads > 0 { s.push_str(&format!(" ({})", i.cpu_threads)); }
        if let Some(m) = i.cpu_mhz.filter(|m| *m > 0) { s.push_str(&format!(" @ {:.2} GHz", m as f64 / 1000.0)); }
        s
    }));
    for g in &i.gpus { add("GPU", Some(g.clone())); }
    add("Memory", i.memory.map(|(u, t)| parse::usage(u, t)));
    if let Some((at, u, t)) = &i.disk { out.push(if at == "/" { ("Disk (/)", parse::usage(*u, *t)) } else { ("Disk", format!("{} ({at})", parse::usage(*u, *t))) }); }
    let mut add = |k: &'static str, v: Option<String>| if let Some(v) = v.filter(|v| !v.trim().is_empty()) { out.push((k, v)); };
    for d in &i.displays { add("Display", Some(d.clone())); }
    add("Battery", i.battery.as_ref().map(|(p, s)| if s.is_empty() { format!("{p}%") } else { format!("{p}% ({s})") }));
    add("Shell", i.shell.clone());
    add("Terminal", i.terminal.clone());
    add("Games", i.games.filter(|n| *n > 0).map(|n| format!("{n} installed")));
    out
}

pub fn to_json(i: &Info) -> Value {
    let mut m = Map::new();
    let mut put = |k: &str, v: Value| if !v.is_null() { m.insert(k.to_string(), v); };
    put("user", json!(i.user));
    put("host", json!(i.host));
    put("os", json!(i.os));
    put("device", json!(i.device));
    put("kernel", json!(i.kernel));
    put("uptimeSeconds", json!(i.uptime));
    put("hearth", json!(i.hearth));
    put("cpu", i.cpu.as_ref().map(|c| json!({"name": parse::clean_cpu(c), "threads": i.cpu_threads, "mhz": i.cpu_mhz})).unwrap_or(Value::Null));
    put("gpus", json!(i.gpus));
    put("memory", i.memory.map(|(u, t)| json!({"usedBytes": u, "totalBytes": t})).unwrap_or(Value::Null));
    put("disk", i.disk.as_ref().map(|(at, u, t)| json!({"mount": at, "usedBytes": u, "totalBytes": t})).unwrap_or(Value::Null));
    put("displays", json!(i.displays));
    put("battery", i.battery.as_ref().map(|(p, s)| json!({"percent": p, "status": s})).unwrap_or(Value::Null));
    put("shell", json!(i.shell));
    put("terminal", json!(i.terminal));
    put("games", json!(i.games));
    put("version", json!(env!("CARGO_PKG_VERSION")));
    Value::Object(m)
}

fn color_mode(flag: Option<&str>) -> Colors {
    match flag {
        Some("never") | Some("none") | Some("plain") => return Colors::Plain,
        Some("always") | Some("truecolor") | Some("24bit") => { enable_ansi(); return Colors::True; }
        Some("256") => { enable_ansi(); return Colors::X256; }
        _ => {}
    }
    if std::env::var_os("NO_COLOR").map(|v| !v.is_empty()).unwrap_or(false) { return Colors::Plain; }
    if !std::io::stdout().is_terminal() { return Colors::Plain; }
    if !enable_ansi() { return Colors::Plain; }
    let colorterm = std::env::var("COLORTERM").unwrap_or_default().to_lowercase();
    if colorterm.contains("truecolor") || colorterm.contains("24bit") || cfg!(windows) { return Colors::True; }
    Colors::X256
}

/// The whole picture: the art on the left, the lines on the right, then the colour blocks.
pub fn render(i: &Info, mode: Colors, with_logo: bool) -> String {
    let (accent, bold, reset) = match mode {
        Colors::True => ("\x1b[1;38;2;255;138;61m", "\x1b[1;38;2;255;209;102m", "\x1b[0m"),
        Colors::X256 => ("\x1b[1;38;5;209m", "\x1b[1;38;5;221m", "\x1b[0m"),
        Colors::Plain => ("", "", ""),
    };
    let mut right: Vec<String> = vec![];
    let user = i.user.clone().unwrap_or_else(|| "user".into());
    let host = i.host.clone().unwrap_or_else(|| "hearth".into());
    right.push(format!("{bold}{user}{reset}@{bold}{host}{reset}"));
    right.push("-".repeat(user.chars().count() + 1 + host.chars().count()));
    let ls = lines(i);
    let w = ls.iter().map(|(k, _)| k.chars().count()).max().unwrap_or(0);
    for (k, v) in &ls { right.push(format!("{accent}{k:<w$}{reset}  {v}")); }
    if mode != Colors::Plain {
        right.push(String::new());
        right.push((0..8).map(|n| format!("\x1b[4{n}m   ")).collect::<String>() + reset);
        right.push((0..8).map(|n| format!("\x1b[10{n}m   ")).collect::<String>() + reset);
    }
    let left = if with_logo { logo::lines(mode) } else { vec![] };
    let rows = left.len().max(right.len());
    let mut out = String::new();
    for r in 0..rows {
        if with_logo {
            match left.get(r) { Some(l) => out.push_str(l), None => out.push_str(&" ".repeat(logo::WIDTH)) }
            out.push_str("   ");
        }
        if let Some(t) = right.get(r) { out.push_str(t); }
        let trimmed = out.trim_end_matches(' ').len();
        out.truncate(trimmed);
        out.push('\n');
    }
    out
}

fn main() {
    let start = std::time::Instant::now();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut json_out = false;
    let mut with_logo = true;
    let mut time = false;
    let mut color: Option<String> = None;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "-h" | "--help" => { println!("{HELP}"); return; }
            "-V" | "--version" => { println!("hearthfetch {}", env!("CARGO_PKG_VERSION")); return; }
            "--json" => json_out = true,
            "--no-logo" => with_logo = false,
            "--time" => time = true,
            "--color" | "--colour" => color = it.next().cloned(),
            s if s.starts_with("--color=") || s.starts_with("--colour=") => color = s.split_once('=').map(|x| x.1.to_string()),
            other => { eprintln!("hearthfetch: unknown option {other}\n\n{HELP}"); std::process::exit(2); }
        }
    }
    let info = collect();
    let text = if json_out {
        serde_json::to_string_pretty(&to_json(&info)).unwrap_or_default() + "\n"
    } else {
        render(&info, color_mode(color.as_deref()), with_logo)
    };
    let mut so = std::io::stdout().lock();
    let _ = so.write_all(text.as_bytes());
    let _ = so.flush();
    if time { eprintln!("hearthfetch took {} ms", start.elapsed().as_millis()); }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Info {
        Info { user: Some("deck".into()), host: Some("steamdeck".into()), os: Some("Hearth OS 0.3.1 (Ubuntu edition) x86_64".into()),
            kernel: Some("Linux 6.17.0".into()), uptime: Some(3720), device: Some("Valve Jupiter".into()), cpu: Some("AMD Custom APU 0405".into()),
            cpu_threads: 8, cpu_mhz: Some(3500), gpus: vec!["AMD Custom GPU 0405".into()], memory: Some((4 << 30, 16 << 30)),
            disk: Some(("/".into(), 100 << 30, 512 << 30)), displays: vec!["1280x800".into()], battery: Some((81, "Discharging".into())),
            shell: Some("bash".into()), terminal: Some("Konsole".into()), hearth: Some("Hearth 0.3.1".into()), games: Some(12) }
    }

    #[test]
    fn lines_in_order() {
        let l = lines(&sample());
        let keys: Vec<&str> = l.iter().map(|x| x.0).collect();
        assert_eq!(keys, ["OS", "Device", "Kernel", "Uptime", "Hearth", "CPU", "GPU", "Memory", "Disk (/)", "Display", "Battery", "Shell", "Terminal", "Games"]);
        assert_eq!(l[3].1, "1 hour, 2 mins");
        assert_eq!(l[5].1, "AMD Custom APU 0405 (8) @ 3.50 GHz");
        assert_eq!(l[7].1, "4.0 GiB / 16.0 GiB (25%)");
        // Nothing found: only what's there.
        assert!(lines(&Info::default()).is_empty());
    }

    #[test]
    fn plain_output_lines_up() {
        let out = render(&sample(), Colors::Plain, true);
        assert!(!out.contains('\x1b'));
        let rows: Vec<&str> = out.lines().collect();
        assert!(rows[0].ends_with("deck@steamdeck"));
        assert!(rows[0].starts_with(&" ".repeat(10)));
        assert_eq!(rows[0].find("deck@").unwrap(), logo::WIDTH + 3);
        assert!(rows.iter().any(|r| r.contains("OS        Hearth OS 0.3.1")));
        let bare = render(&sample(), Colors::Plain, false);
        assert!(bare.starts_with("deck@steamdeck\n--------------\nOS  "));
        assert!(render(&sample(), Colors::True, true).contains("\x1b[38;2;"));
    }

    #[test]
    fn json_has_what_was_found() {
        let j = to_json(&sample());
        assert_eq!(j["memory"]["totalBytes"], 16u64 << 30);
        assert_eq!(j["cpu"]["name"], "AMD Custom APU 0405");
        assert_eq!(j["battery"]["percent"], 81);
        let e = to_json(&Info::default());
        assert!(e.get("os").is_none() && e.get("memory").is_none());
        assert_eq!(e["version"], env!("CARGO_PKG_VERSION"));
    }
}
