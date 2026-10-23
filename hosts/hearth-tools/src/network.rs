//! Network Tools: a speed test (Cloudflare's public speed test endpoints, no account or key), what's connected
//! and which apps use the network (/proc/net and `ss`, like nethogs without root: your own apps' TCP traffic),
//! Wi-Fi signal and channels (NetworkManager), ping, traceroute and DNS lookups (a small DNS client here, so
//! every record type works without extra tools).

use crate::util;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

// ------------------------------------------------------------------ overview: interfaces, addresses, router, DNS

pub fn overview() -> Result<Value, String> {
    let addrs: Value = util::run("ip", &["-j", "addr"], 10).ok().and_then(|o| serde_json::from_str(&o.out).ok()).unwrap_or(json!([]));
    let routes: Value = util::run("ip", &["-j", "route", "show", "default"], 10).ok().and_then(|o| serde_json::from_str(&o.out).ok()).unwrap_or(json!([]));
    let mut ifaces = vec![];
    for a in addrs.as_array().into_iter().flatten() {
        let name = a["ifname"].as_str().unwrap_or("").to_string();
        if name == "lo" || name.is_empty() { continue; }
        let sys = format!("/sys/class/net/{name}");
        let kind = if std::path::Path::new(&format!("{sys}/wireless")).exists() { "wifi" }
            else if !std::path::Path::new(&format!("{sys}/device")).exists() { "virtual" } else { "wired" };
        let stat = |f: &str| std::fs::read_to_string(format!("{sys}/statistics/{f}")).ok().and_then(|s| s.trim().parse::<u64>().ok()).unwrap_or(0);
        let ips: Vec<String> = a["addr_info"].as_array().into_iter().flatten()
            .filter(|i| i["scope"].as_str() != Some("link")).filter_map(|i| Some(format!("{}/{}", i["local"].as_str()?, i["prefixlen"].as_u64()?))).collect();
        let speed = std::fs::read_to_string(format!("{sys}/speed")).ok().and_then(|s| s.trim().parse::<i64>().ok()).filter(|s| *s > 0);
        ifaces.push(json!({"name": name, "kind": kind, "up": a["operstate"].as_str() == Some("UP"), "mac": a["address"], "ips": ips,
            "rx": stat("rx_bytes"), "tx": stat("tx_bytes"), "mbps": speed}));
    }
    let gw = routes.as_array().and_then(|r| r.first()).map(|r| json!({"via": r["gateway"], "dev": r["dev"]})).unwrap_or(Value::Null);
    Ok(json!({"interfaces": ifaces, "gateway": gw, "dns": dns_servers()}))
}

/// The DNS servers in use: resolv.conf's, or systemd-resolved's real ones when resolv.conf points at its stub.
fn dns_servers() -> Vec<String> {
    let conf: Vec<String> = std::fs::read_to_string("/etc/resolv.conf").unwrap_or_default().lines()
        .filter_map(|l| l.trim().strip_prefix("nameserver").map(|s| s.trim().to_string())).collect();
    if conf.iter().all(|s| s == "127.0.0.53" || s == "127.0.0.54") {
        if let Ok(o) = util::run("resolvectl", &["dns"], 5) {
            let mut out: Vec<String> = o.out.lines().filter_map(|l| l.split_once(':').map(|x| x.1.trim().to_string())).flat_map(|s| s.split_whitespace().map(str::to_string).collect::<Vec<_>>()).collect();
            out.dedup();
            if !out.is_empty() { return out; }
        }
    }
    conf
}

// ------------------------------------------------------------------ Wi-Fi

/// nmcli -t with -e yes: fields split on ':' except escaped '\:'.
pub fn split_nmcli(line: &str) -> Vec<String> {
    let mut out = vec![String::new()];
    let mut esc = false;
    for c in line.chars() {
        if esc { out.last_mut().unwrap().push(c); esc = false; }
        else if c == '\\' { esc = true; }
        else if c == ':' { out.push(String::new()); }
        else { out.last_mut().unwrap().push(c); }
    }
    out
}

pub fn wifi_rows(text: &str) -> Vec<Value> {
    text.lines().filter_map(|l| {
        let f = split_nmcli(l);
        if f.len() < 8 { return None; }
        Some(json!({"active": f[0] == "*", "ssid": f[1], "bssid": f[2], "channel": f[3].parse::<u32>().ok(), "freq": f[4].trim_end_matches(" MHz").parse::<u32>().ok(),
            "rate": f[5], "signal": f[6].parse::<u32>().ok(), "security": f[7]}))
    }).collect()
}

pub fn wifi(args: &Value) -> Result<Value, String> {
    let rescan = if util::arg(args, "rescan") == "yes" { "yes" } else { "no" };
    let o = util::run("nmcli", &["-t", "-e", "yes", "-f", "IN-USE,SSID,BSSID,CHAN,FREQ,RATE,SIGNAL,SECURITY", "dev", "wifi", "list", "--rescan", rescan], 30)?;
    if !o.ok() { return Err(format!("Couldn't ask NetworkManager about Wi-Fi ({})", o.why())); }
    let nets = wifi_rows(&o.out);
    // The link itself (signal in dBm, speeds) from iw, when there's a Wi-Fi device and iw is installed.
    let mut link = Value::Null;
    if let Ok(dev) = util::run("nmcli", &["-t", "-f", "DEVICE,TYPE,STATE", "dev"], 10) {
        if let Some(d) = dev.out.lines().map(split_nmcli).find(|f| f.len() >= 3 && f[1] == "wifi" && f[2] == "connected").map(|f| f[0].clone()) {
            if let Ok(o) = util::run("iw", &["dev", &d, "link"], 5) { link = parse_iw_link(&o.out); link["device"] = json!(d); }
        }
    }
    Ok(json!({"networks": nets, "link": link}))
}

pub fn parse_iw_link(text: &str) -> Value {
    let get = |k: &str| text.lines().find_map(|l| l.trim().strip_prefix(k).map(|v| v.trim().to_string()));
    json!({"ssid": get("SSID:"), "freq": get("freq:"), "signal": get("signal:"), "rx": get("rx bitrate:"), "tx": get("tx bitrate:")})
}

// ------------------------------------------------------------------ connections (/proc/net) and per-app use (ss)

const TCP_STATES: [&str; 12] = ["", "Connected", "Opening", "Opening", "Closing", "Closing", "Closed", "Closed", "Closing", "Closing", "Listening", "Closing"];

fn hex_ip(s: &str) -> Option<IpAddr> {
    if s.len() == 8 {
        let n = u32::from_str_radix(s, 16).ok()?;
        return Some(IpAddr::V4(Ipv4Addr::from(n.swap_bytes())));
    }
    if s.len() == 32 {
        // Four 32-bit words, each in the machine's (little-endian) order.
        let mut b = [0u8; 16];
        for w in 0..4 {
            let n = u32::from_str_radix(&s[w * 8..w * 8 + 8], 16).ok()?;
            b[w * 4..w * 4 + 4].copy_from_slice(&n.to_le_bytes());
        }
        let v6 = Ipv6Addr::from(b);
        return Some(v6.to_ipv4_mapped().map(IpAddr::V4).unwrap_or(IpAddr::V6(v6)));
    }
    None
}

fn hex_addr(s: &str) -> Option<(IpAddr, u16)> {
    let (ip, port) = s.split_once(':')?;
    Some((hex_ip(ip)?, u16::from_str_radix(port, 16).ok()?))
}

/// One table of /proc/net (tcp, tcp6, udp, udp6): (proto, local, remote, state, inode, uid).
pub fn parse_proc_net(text: &str, proto: &str) -> Vec<(String, (IpAddr, u16), (IpAddr, u16), String, u64, u32)> {
    text.lines().skip(1).filter_map(|l| {
        let f: Vec<&str> = l.split_whitespace().collect();
        if f.len() < 10 { return None; }
        let st = usize::from_str_radix(f[3], 16).ok()?;
        let state = if proto.starts_with("tcp") { TCP_STATES.get(st).copied().unwrap_or("").to_string() } else if st == 7 { "Open".into() } else { "Connected".into() };
        Some((proto.to_string(), hex_addr(f[1])?, hex_addr(f[2])?, state, f[9].parse().ok()?, f[7].parse().ok()?))
    }).collect()
}

/// socket inode -> (pid, app name), for the processes we can look into (yours, or all as root).
fn socket_owners() -> HashMap<u64, (u32, String)> {
    let mut m = HashMap::new();
    for e in std::fs::read_dir("/proc").into_iter().flatten().flatten() {
        let Ok(pid) = e.file_name().to_string_lossy().parse::<u32>() else { continue };
        let Ok(fds) = std::fs::read_dir(e.path().join("fd")) else { continue };
        let mut name: Option<String> = None;
        for fd in fds.flatten() {
            let Ok(t) = std::fs::read_link(fd.path()) else { continue };
            let t = t.to_string_lossy();
            if let Some(i) = t.strip_prefix("socket:[").and_then(|r| r.strip_suffix(']')).and_then(|n| n.parse::<u64>().ok()) {
                let n = name.get_or_insert_with(|| std::fs::read_to_string(e.path().join("comm")).map(|s| s.trim().to_string()).unwrap_or_default()).clone();
                m.insert(i, (pid, n));
            }
        }
    }
    m
}

fn uid_name(uid: u32) -> String {
    std::fs::read_to_string("/etc/passwd").unwrap_or_default().lines().find_map(|l| {
        let f: Vec<&str> = l.split(':').collect();
        (f.len() > 2 && f[2] == uid.to_string()).then(|| f[0].to_string())
    }).unwrap_or_else(|| uid.to_string())
}

pub fn connections() -> Result<Value, String> {
    let owners = socket_owners();
    let mut rows = vec![];
    let mut hidden = 0;
    for proto in ["tcp", "tcp6", "udp", "udp6"] {
        let text = std::fs::read_to_string(format!("/proc/net/{proto}")).unwrap_or_default();
        for (p, l, r, state, inode, uid) in parse_proc_net(&text, proto) {
            if l.0.is_loopback() && (r.0.is_loopback() || r.0.is_unspecified()) { continue; }   // the device talking to itself
            if state == "Closed" { continue; }
            let owner = owners.get(&inode);
            if owner.is_none() { hidden += 1; }
            rows.push(json!({"proto": if p.starts_with("tcp") { "TCP" } else { "UDP" }, "local": fmt_addr(l), "remote": if r.0.is_unspecified() { String::new() } else { fmt_addr(r) },
                "state": state, "app": owner.map(|o| o.1.clone()).unwrap_or_default(), "pid": owner.map(|o| o.0), "user": uid_name(uid)}));
        }
    }
    rows.sort_by(|a, b| (a["state"].as_str() != Some("Connected")).cmp(&(b["state"].as_str() != Some("Connected"))).then(a["app"].as_str().cmp(&b["app"].as_str())));
    Ok(json!({"connections": rows, "unknownOwner": hidden}))
}

fn fmt_addr(a: (IpAddr, u16)) -> String { match a.0 { IpAddr::V6(v) => format!("[{v}]:{}", a.1), IpAddr::V4(v) => format!("{v}:{}", a.1) } }

/// `ss -tinpH`: each TCP socket (its header line), then an indented line with its counters. Returns
/// (key, app, bytes sent, bytes received).
pub fn parse_ss(text: &str) -> Vec<(String, String, u64, u64)> {
    let mut out: Vec<(String, String, u64, u64)> = vec![];
    for l in text.lines() {
        if l.starts_with(' ') || l.starts_with('\t') {
            if let Some(last) = out.last_mut() {
                for w in l.split_whitespace() {
                    if let Some(v) = w.strip_prefix("bytes_sent:") { last.2 = v.parse().unwrap_or(0); }
                    if let Some(v) = w.strip_prefix("bytes_received:") { last.3 = v.parse().unwrap_or(0); }
                }
            }
            continue;
        }
        let f: Vec<&str> = l.split_whitespace().collect();
        if f.len() < 5 { continue; }
        let app = l.split("users:((\"").nth(1).and_then(|r| r.split('"').next()).unwrap_or("").to_string();
        out.push((format!("{} {}", f[3], f[4]), app, 0, 0));
    }
    out
}

static LAST_SS: Mutex<Option<(Instant, HashMap<String, (u64, u64)>, HashMap<String, (u64, u64)>)>> = Mutex::new(None);

/// Which apps use the network right now: bytes per second up and down for each app (TCP), from the change since
/// the last call, and each network device's total.
pub fn usage() -> Result<Value, String> {
    let o = util::run("ss", &["-tinpH"], 10)?;
    let socks = parse_ss(&o.out);
    let now = Instant::now();
    let mut cur: HashMap<String, (u64, u64)> = HashMap::new();
    let mut apps_now: HashMap<String, String> = HashMap::new();
    for (k, app, s, r) in &socks { cur.insert(k.clone(), (*s, *r)); apps_now.insert(k.clone(), app.clone()); }
    let mut ifs: HashMap<String, (u64, u64)> = HashMap::new();
    for e in std::fs::read_dir("/sys/class/net").into_iter().flatten().flatten() {
        let n = e.file_name().to_string_lossy().into_owned();
        if n == "lo" { continue; }
        let st = |f: &str| std::fs::read_to_string(e.path().join("statistics").join(f)).ok().and_then(|s| s.trim().parse::<u64>().ok()).unwrap_or(0);
        ifs.insert(n, (st("tx_bytes"), st("rx_bytes")));
    }
    let mut g = LAST_SS.lock().map_err(|_| "busy")?;
    let mut per_app: HashMap<String, (f64, f64, u32)> = HashMap::new();
    let mut devices = vec![];
    if let Some((t0, prev, prev_if)) = g.as_ref() {
        let dt = now.duration_since(*t0).as_secs_f64().max(0.2);
        for (k, (s, r)) in &cur {
            let (ps, pr) = prev.get(k).copied().unwrap_or((0, 0));
            let app = apps_now.get(k).cloned().filter(|a| !a.is_empty()).unwrap_or_else(|| "Other (not yours)".into());
            let e = per_app.entry(app).or_insert((0.0, 0.0, 0));
            e.0 += s.saturating_sub(ps) as f64 / dt;
            e.1 += r.saturating_sub(pr) as f64 / dt;
            e.2 += 1;
        }
        for (n, (tx, rx)) in &ifs {
            let (ptx, prx) = prev_if.get(n).copied().unwrap_or((*tx, *rx));
            devices.push(json!({"name": n, "up": (tx.saturating_sub(ptx)) as f64 / dt, "down": (rx.saturating_sub(prx)) as f64 / dt}));
        }
    } else {
        for (k, _) in &cur { let app = apps_now.get(k).cloned().filter(|a| !a.is_empty()).unwrap_or_else(|| "Other (not yours)".into()); per_app.entry(app).or_insert((0.0, 0.0, 0)).2 += 1; }
    }
    *g = Some((now, cur, ifs));
    let mut apps: Vec<Value> = per_app.into_iter().map(|(a, (up, down, n))| json!({"app": a, "up": up, "down": down, "sockets": n})).collect();
    apps.sort_by(|a, b| (b["up"].as_f64().unwrap_or(0.0) + b["down"].as_f64().unwrap_or(0.0)).partial_cmp(&(a["up"].as_f64().unwrap_or(0.0) + a["down"].as_f64().unwrap_or(0.0))).unwrap_or(std::cmp::Ordering::Equal));
    Ok(json!({"apps": apps, "devices": devices}))
}

// ------------------------------------------------------------------ speed test (Cloudflare)

const SPEED: &str = "https://speed.cloudflare.com";

fn null_dev() -> &'static str { if cfg!(windows) { "NUL" } else { "/dev/null" } }

/// Runs `streams` curl downloads (or uploads of `file`) at once; returns (bytes, seconds).
fn round(bytes: u64, streams: usize, upload: Option<&std::path::Path>, cancel: &AtomicBool) -> Result<(u64, f64), String> {
    let start = Instant::now();
    let url = if upload.is_some() { format!("{SPEED}/__up") } else { format!("{SPEED}/__down?bytes={bytes}") };
    let results: Vec<Result<u64, String>> = std::thread::scope(|s| {
        let hs: Vec<_> = (0..streams).map(|_| s.spawn(|| {
            if cancel.load(Ordering::SeqCst) { return Err("Stopped.".to_string()); }
            let data = upload.map(|p| format!("@{}", p.to_string_lossy()));
            let mut a: Vec<&str> = vec!["-sS", "-m", "30", "-o", null_dev(), "-w", "%{size_download} %{size_upload} %{http_code}"];
            if let Some(d) = data.as_deref() { a.extend(["-X", "POST", "-H", "Content-Type: application/octet-stream", "--data-binary", d]); }
            a.push(&url);
            let o = util::run("curl", &a, 40)?;
            let f: Vec<u64> = o.out.split_whitespace().filter_map(|x| x.parse::<f64>().ok().map(|v| v as u64)).collect();
            if !o.ok() || f.len() < 3 || f[2] >= 400 { return Err(format!("The speed test server didn't answer ({})", o.why())); }
            Ok(if upload.is_some() { f[1] } else { f[0] })
        })).collect();
        hs.into_iter().map(|h| h.join().unwrap_or_else(|_| Err("crashed".into()))).collect()
    });
    let secs = start.elapsed().as_secs_f64();
    let mut total = 0;
    for r in results { total += r?; }
    Ok((total, secs))
}

pub fn median(v: &mut [f64]) -> f64 {
    if v.is_empty() { return 0.0; }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let m = v.len() / 2;
    if v.len() % 2 == 0 { (v[m - 1] + v[m]) / 2.0 } else { v[m] }
}

/// The speed test: where it connects, latency and jitter, download, upload. Reports each step as it goes.
pub fn speed(cancel: &AtomicBool, progress: &(dyn Fn(Value) + Sync)) -> Result<Value, String> {
    if util::which("curl").is_none() { return Err("curl isn't installed.".into()); }
    progress(json!({"stage": "server"}));
    let trace = util::run("curl", &["-sS", "-m", "10", &format!("{SPEED}/cdn-cgi/trace")], 15)?;
    if !trace.ok() { return Err(format!("Couldn't reach the speed test (speed.cloudflare.com): {}", trace.why())); }
    let field = |k: &str| trace.out.lines().find_map(|l| l.strip_prefix(&format!("{k}="))).unwrap_or("").to_string();
    let server = json!({"colo": field("colo"), "loc": field("loc"), "http": field("http")});
    progress(json!({"stage": "latency", "server": server}));
    // Latency: 12 tiny requests on one connection; the time from sending to the first byte back.
    let mut a: Vec<String> = vec!["-sS".into(), "-m".into(), "20".into(), "-w".into(), "%{time_pretransfer} %{time_starttransfer}\\n".into()];
    for _ in 0..12 { a.push("-o".into()); a.push(null_dev().into()); a.push(format!("{SPEED}/__down?bytes=0")); }
    let o = util::run("curl", &a.iter().map(String::as_str).collect::<Vec<_>>(), 30)?;
    let mut pings: Vec<f64> = o.out.lines().filter_map(|l| { let f: Vec<f64> = l.split_whitespace().filter_map(|x| x.parse().ok()).collect(); (f.len() == 2).then(|| (f[1] - f[0]) * 1000.0) }).filter(|v| *v > 0.0).collect();
    let jitter = if pings.len() > 1 { pings.windows(2).map(|w| (w[1] - w[0]).abs()).sum::<f64>() / (pings.len() - 1) as f64 } else { 0.0 };
    let latency = median(&mut pings);
    progress(json!({"stage": "download", "latency": latency, "jitter": jitter}));
    // Download and upload: rounds of 4 streams, growing until a round takes a few seconds.
    let mut down = 0.0;
    for size in [1_000_000u64, 10_000_000, 25_000_000, 50_000_000] {
        if cancel.load(Ordering::SeqCst) { return Err("Stopped.".into()); }
        let (b, s) = round(size, 4, None, cancel)?;
        down = b as f64 * 8.0 / s;
        progress(json!({"stage": "download", "mbps": down / 1e6}));
        if s > 3.0 { break; }
    }
    progress(json!({"stage": "upload", "download": down / 1e6}));
    let tmp = std::env::temp_dir().join(format!("hearth-speed-{}.bin", std::process::id()));
    let mut up = 0.0;
    for size in [1_000_000usize, 5_000_000, 15_000_000] {
        if cancel.load(Ordering::SeqCst) { let _ = std::fs::remove_file(&tmp); return Err("Stopped.".into()); }
        // Not compressible: a simple generator's bytes.
        let mut x: u32 = 2463534242;
        let data: Vec<u8> = (0..size).map(|_| { x ^= x << 13; x ^= x >> 17; x ^= x << 5; x as u8 }).collect();
        std::fs::write(&tmp, &data).map_err(|e| e.to_string())?;
        let r = round(size as u64, 4, Some(&tmp), cancel);
        let (b, s) = match r { Ok(v) => v, Err(e) => { let _ = std::fs::remove_file(&tmp); return Err(e); } };
        up = b as f64 * 8.0 / s;
        progress(json!({"stage": "upload", "mbps": up / 1e6}));
        if s > 3.0 { break; }
    }
    let _ = std::fs::remove_file(&tmp);
    Ok(json!({"server": server, "latency": latency, "jitter": jitter, "download": down / 1e6, "upload": up / 1e6}))
}

// ------------------------------------------------------------------ ping and traceroute

pub fn ping(args: &Value, cancel: &AtomicBool, line: &(dyn Fn(Value) + Sync)) -> Result<Value, String> {
    let host = util::arg(args, "host").trim().to_string();
    if !util::valid_host(&host) { return Err("Type a name like example.com or an address like 1.1.1.1.".into()); }
    let count = util::arg_u64(args, "count", 4).clamp(1, 50).to_string();
    let code = util::run_lines("ping", &["-c", &count, "-W", "3", "--", &host], cancel, |l| line(json!({"line": l})))?;
    Ok(json!({"code": code}))
}

pub fn trace(args: &Value, cancel: &AtomicBool, line: &(dyn Fn(Value) + Sync)) -> Result<Value, String> {
    let host = util::arg(args, "host").trim().to_string();
    if !util::valid_host(&host) { return Err("Type a name like example.com or an address like 1.1.1.1.".into()); }
    let code = if util::which("traceroute").is_some() {
        util::run_lines("traceroute", &["-q", "1", "-w", "2", "-m", "30", "--", &host], cancel, |l| line(json!({"line": l})))?
    } else if util::which("tracepath").is_some() {
        util::run_lines("tracepath", &["-m", "30", &host], cancel, |l| line(json!({"line": l})))?
    } else { return Err("Neither traceroute nor tracepath is installed.".into()); };
    Ok(json!({"code": code}))
}

// ------------------------------------------------------------------ DNS lookup

pub fn qtype(name: &str) -> Option<u16> {
    Some(match name { "A" => 1, "NS" => 2, "CNAME" => 5, "SOA" => 6, "PTR" => 12, "MX" => 15, "TXT" => 16, "AAAA" => 28, "SRV" => 33, "CAA" => 257, _ => return None })
}
fn type_name(t: u16) -> String {
    let n = match t { 1 => "A", 2 => "NS", 5 => "CNAME", 6 => "SOA", 12 => "PTR", 15 => "MX", 16 => "TXT", 28 => "AAAA", 33 => "SRV", 257 => "CAA", _ => "" };
    if n.is_empty() { format!("TYPE{t}") } else { n.to_string() }
}

/// A query for `name` (dots) of type `t`, with id `id` and recursion wanted.
pub fn build_query(id: u16, name: &str, t: u16) -> Result<Vec<u8>, String> {
    let mut q = vec![(id >> 8) as u8, id as u8, 0x01, 0x00, 0, 1, 0, 0, 0, 0, 0, 0];
    for label in name.trim_end_matches('.').split('.') {
        if label.is_empty() || label.len() > 63 { return Err("That isn't a valid name.".into()); }
        q.push(label.len() as u8);
        q.extend_from_slice(label.as_bytes());
    }
    q.push(0);
    q.extend_from_slice(&[(t >> 8) as u8, t as u8, 0, 1]);
    Ok(q)
}

/// A name in a DNS message at `pos` (following compression pointers): (name, where the name ends).
fn read_name(m: &[u8], mut pos: usize) -> Option<(String, usize)> {
    let mut labels = vec![];
    let mut end = None;
    for _ in 0..128 {
        let len = *m.get(pos)? as usize;
        if len == 0 { if end.is_none() { end = Some(pos + 1); } break; }
        if len & 0xC0 == 0xC0 {
            let ptr = ((len & 0x3F) << 8) | *m.get(pos + 1)? as usize;
            if end.is_none() { end = Some(pos + 2); }
            pos = ptr;
            continue;
        }
        labels.push(String::from_utf8_lossy(m.get(pos + 1..pos + 1 + len)?).into_owned());
        pos += 1 + len;
    }
    Some((if labels.is_empty() { ".".into() } else { labels.join(".") }, end?))
}

fn u16_at(m: &[u8], p: usize) -> Option<u16> { Some(((*m.get(p)? as u16) << 8) | *m.get(p + 1)? as u16) }
fn u32_at(m: &[u8], p: usize) -> Option<u32> { Some(((u16_at(m, p)? as u32) << 16) | u16_at(m, p + 2)? as u32) }

/// The answers of a DNS reply: (rcode, truncated, [{type, name, ttl, value}]).
pub fn parse_reply(m: &[u8], id: u16) -> Result<(u8, bool, Vec<Value>), String> {
    if m.len() < 12 || u16_at(m, 0) != Some(id) { return Err("The DNS server's answer didn't match the question.".into()); }
    let rcode = m[3] & 0x0F;
    let tc = m[2] & 0x02 != 0;
    let qd = u16_at(m, 4).unwrap_or(0);
    let an = u16_at(m, 6).unwrap_or(0) as usize + u16_at(m, 8).unwrap_or(0) as usize;
    let bad = || "The DNS server's answer was damaged.".to_string();
    let mut pos = 12;
    for _ in 0..qd { pos = read_name(m, pos).ok_or_else(bad)?.1 + 4; }
    let mut out = vec![];
    for i in 0..an {
        let (name, p) = read_name(m, pos).ok_or_else(bad)?;
        let t = u16_at(m, p).ok_or_else(bad)?;
        let ttl = u32_at(m, p + 4).ok_or_else(bad)?;
        let len = u16_at(m, p + 8).ok_or_else(bad)? as usize;
        let d = p + 10;
        let data = m.get(d..d + len).ok_or_else(bad)?;
        let value = match t {
            1 if len == 4 => Ipv4Addr::new(data[0], data[1], data[2], data[3]).to_string(),
            28 if len == 16 => { let mut b = [0u8; 16]; b.copy_from_slice(data); Ipv6Addr::from(b).to_string() }
            2 | 5 | 12 => read_name(m, d).map(|x| x.0).unwrap_or_default(),
            15 => format!("{} {}", u16_at(m, d).unwrap_or(0), read_name(m, d + 2).map(|x| x.0).unwrap_or_default()),
            16 => { let mut s = String::new(); let mut q = 0; while q < data.len() { let l = data[q] as usize; s.push_str(&String::from_utf8_lossy(data.get(q + 1..q + 1 + l).unwrap_or(&[]))); q += 1 + l; } s }
            33 => format!("priority {} weight {} port {} {}", u16_at(m, d).unwrap_or(0), u16_at(m, d + 2).unwrap_or(0), u16_at(m, d + 4).unwrap_or(0), read_name(m, d + 6).map(|x| x.0).unwrap_or_default()),
            6 => { let (mn, e) = read_name(m, d).unwrap_or_default(); let (rn, e2) = read_name(m, e).unwrap_or_default(); format!("{mn} {rn} serial {}", u32_at(m, e2).unwrap_or(0)) }
            _ => data.iter().map(|b| format!("{b:02x}")).collect::<String>(),
        };
        out.push(json!({"type": type_name(t), "name": name, "ttl": ttl, "value": value, "section": if i < u16_at(m, 6).unwrap_or(0) as usize { "answer" } else { "authority" }}));
        pos = d + len;
    }
    Ok((rcode, tc, out))
}

/// 1.2.3.4 -> 4.3.2.1.in-addr.arpa (for PTR lookups of an address).
pub fn reverse_name(ip: &IpAddr) -> String {
    match ip {
        IpAddr::V4(v) => { let o = v.octets(); format!("{}.{}.{}.{}.in-addr.arpa", o[3], o[2], o[1], o[0]) }
        IpAddr::V6(v) => v.octets().iter().rev().flat_map(|b| [b & 15, b >> 4]).map(|n| format!("{n:x}.")).collect::<String>() + "ip6.arpa",
    }
}

fn query_tcp(server: SocketAddr, q: &[u8]) -> Result<Vec<u8>, String> {
    use std::io::{Read, Write};
    let mut s = std::net::TcpStream::connect_timeout(&server, Duration::from_secs(4)).map_err(|e| e.to_string())?;
    s.set_read_timeout(Some(Duration::from_secs(4))).ok();
    let mut msg = vec![(q.len() >> 8) as u8, q.len() as u8];
    msg.extend_from_slice(q);
    s.write_all(&msg).map_err(|e| e.to_string())?;
    let mut len = [0u8; 2];
    s.read_exact(&mut len).map_err(|e| e.to_string())?;
    let mut buf = vec![0u8; ((len[0] as usize) << 8) | len[1] as usize];
    s.read_exact(&mut buf).map_err(|e| e.to_string())?;
    Ok(buf)
}

pub fn dns(args: &Value) -> Result<Value, String> {
    let name = util::arg(args, "name").trim().trim_end_matches('.').to_string();
    if !util::valid_host(&name) { return Err("Type a name like example.com (or an address, to find its name).".into()); }
    let mut tname = util::arg(args, "type").to_uppercase();
    let (qname, t) = match name.parse::<IpAddr>() {
        Ok(ip) => { tname = "PTR".into(); (reverse_name(&ip), 12) }
        Err(_) => (name.clone(), qtype(if tname.is_empty() { "A" } else { &tname }).ok_or("Unknown record type.")?),
    };
    let server_s = match util::arg(args, "server") { "" | "system" => dns_servers().into_iter().next().unwrap_or_else(|| "127.0.0.53".into()), s => s.to_string() };
    let ip: IpAddr = server_s.split('%').next().unwrap_or("").parse().map_err(|_| "Unknown DNS server.")?;
    let server = SocketAddr::new(ip, 53);
    let id: u16 = (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(1) & 0xFFFF) as u16;
    let q = build_query(id, &qname, t)?;
    let start = Instant::now();
    let sock = UdpSocket::bind(if ip.is_ipv4() { "0.0.0.0:0" } else { "[::]:0" }).map_err(|e| e.to_string())?;
    sock.set_read_timeout(Some(Duration::from_secs(3))).ok();
    sock.send_to(&q, server).map_err(|e| format!("Couldn't reach the DNS server {server_s}: {e}"))?;
    let mut buf = [0u8; 4096];
    let mut reply = None;
    for _ in 0..2 {
        match sock.recv_from(&mut buf) {
            Ok((n, _)) => { reply = Some(buf[..n].to_vec()); break; }
            Err(_) => { let _ = sock.send_to(&q, server); }
        }
    }
    let reply = reply.ok_or_else(|| format!("The DNS server {server_s} didn't answer."))?;
    let (mut rcode, tc, mut answers) = parse_reply(&reply, id)?;
    if tc { if let Ok(r) = query_tcp(server, &q) { if let Ok((rc, _, a)) = parse_reply(&r, id) { rcode = rc; answers = a; } } }
    let status = match rcode { 0 => "ok", 2 => "The DNS server failed to answer (SERVFAIL).", 3 => "No such name (NXDOMAIN).", 5 => "The DNS server refused.", _ => "The DNS server returned an error." };
    Ok(json!({"name": name, "query": qname, "type": tname, "server": server_s, "ms": start.elapsed().as_millis() as u64, "status": status, "answers": answers}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nmcli_fields() {
        assert_eq!(split_nmcli(r"*:Home\:5G:AA\:BB\:CC\:DD\:EE\:FF:36:5180 MHz:540 Mbit/s:82:WPA2"), vec!["*", "Home:5G", "AA:BB:CC:DD:EE:FF", "36", "5180 MHz", "540 Mbit/s", "82", "WPA2"]);
        let r = wifi_rows(r" :Cafe:11\:22\:33\:44\:55\:66:6:2437 MHz:130 Mbit/s:40:");
        assert_eq!((r[0]["ssid"].as_str(), r[0]["channel"].as_u64(), r[0]["freq"].as_u64(), r[0]["active"].as_bool()), (Some("Cafe"), Some(6), Some(2437), Some(false)));
    }

    #[test]
    fn proc_net() {
        let t = "  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode\n   0: 0100007F:0277 00000000:0000 0A 00000000:00000000 00:00000000 00000000     0        0 12345 1\n   1: 0501A8C0:D2A4 0E0D0C0B:01BB 01 00000000:00000000 00:00000000 00000000  1000        0 67890 1\n";
        let r = parse_proc_net(t, "tcp");
        assert_eq!(r[0].1, ("127.0.0.1".parse().unwrap(), 631));
        assert_eq!(r[0].3, "Listening");
        assert_eq!(r[1].1, ("192.168.1.5".parse().unwrap(), 53924));
        assert_eq!(r[1].2, ("11.12.13.14".parse().unwrap(), 443));
        assert_eq!((r[1].3.as_str(), r[1].4, r[1].5), ("Connected", 67890, 1000));
        let v6 = parse_proc_net("h\n 0: 0000000000000000FFFF00000501A8C0:0016 00000000000000000000000000000000:0000 0A 0:0 0:0 0 0 0 99 1\n", "tcp6");
        assert_eq!(v6[0].1 .0, "192.168.1.5".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn ss_output() {
        let t = "ESTAB 0 0 192.168.1.5:41000 1.2.3.4:443 users:((\"firefox\",pid=1234,fd=88))\n\t cubic wscale:7,7 rto:204 bytes_sent:1500 bytes_acked:1500 bytes_received:90000 segs_out:20\nESTAB 0 0 192.168.1.5:22 192.168.1.2:5000\n\t bytes_received:10\n";
        let s = parse_ss(t);
        assert_eq!(s.len(), 2);
        assert_eq!((s[0].1.as_str(), s[0].2, s[0].3), ("firefox", 1500, 90000));
        assert_eq!((s[1].1.as_str(), s[1].3), ("", 10));
    }

    #[test]
    fn dns_messages() {
        let q = build_query(0x1234, "example.com", 1).unwrap();
        assert_eq!(&q[..4], &[0x12, 0x34, 1, 0]);
        assert_eq!(&q[12..25], b"\x07example\x03com\x00");
        // A reply: the question, then an A record and an MX record using name compression (pointer to 12).
        let mut r = q.clone();
        r[2] = 0x81; r[3] = 0x80; r[7] = 2;
        r.extend_from_slice(&[0xC0, 12, 0, 1, 0, 1, 0, 0, 0x0E, 0x10, 0, 4, 93, 184, 216, 34]);
        r.extend_from_slice(&[0xC0, 12, 0, 15, 0, 1, 0, 0, 0, 60, 0, 9, 0, 10, 4, b'm', b'a', b'i', b'l', 0xC0, 12]);
        let (rc, tc, a) = parse_reply(&r, 0x1234).unwrap();
        assert_eq!((rc, tc, a.len()), (0, false, 2));
        assert_eq!((a[0]["type"].as_str(), a[0]["value"].as_str(), a[0]["ttl"].as_u64()), (Some("A"), Some("93.184.216.34"), Some(3600)));
        assert_eq!(a[1]["value"], "10 mail.example.com");
        assert!(parse_reply(&r, 1).is_err());
        assert!(build_query(1, "bad..name", 1).is_err());
        assert_eq!(reverse_name(&"1.2.3.4".parse().unwrap()), "4.3.2.1.in-addr.arpa");
        assert_eq!(qtype("AAAA"), Some(28));
        assert_eq!(type_name(99), "TYPE99");
    }

    #[test]
    fn middle() { assert_eq!(median(&mut [3.0, 1.0, 2.0]), 2.0); assert_eq!(median(&mut [4.0, 1.0, 2.0, 3.0]), 2.5); }
}
