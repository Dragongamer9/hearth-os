//! Performance › CPU › Power & settings, Disk › Health & settings, Network › Apps & settings, GPU › Tuning and
//! Battery › Health & settings: Mantel's DeviceSettings.qml in Rust, built to the approved mockups tm-3 … tm-7
//! (docs/mockups/checklists/task-manager-rust.md). Each page: its numbers on top, then its cards.
//!
//! The same helpers and files as Mantel (red flag 4), so Debian's Mantel and the Fedora edition share settings:
//! hearth-hwset (/etc/hearth/hardware.json, through pkexec), hearth-power, hearth-netinfo, hearth-gputune (LACT),
//! hearth-tasks (disk-busy, battery-use) and nmcli. Opening a tab only reads (every 3 s while it's open, the slow
//! ones every 20 s, each with a time limit); only what you press changes something, and it's kept by the helper
//! that owns it, so it comes back after a restart.

use hearth_ui::run;
use hearth_ui::ui::{self, btn, t, t1, Kind};
use hearth_ui::Pal;
use iced::widget::canvas::{self, Frame, Geometry, LineCap, LineJoin, Path, Stroke};
use iced::widget::{column, container, mouse_area, row, stack, Column, Row, Space};
use iced::{mouse, Color, Length, Padding, Point, Rectangle, Renderer, Task, Theme};
use serde_json::Value;

use crate::sample::Sample;
use crate::widgets::{self as w, Cell, E};
use crate::{App, Msg};

pub const HWSET: &str = "/usr/libexec/nimbus/hearth-hwset";
static NULL: Value = Value::Null;

/// `v` at a dotted path ("turbo.on"), or null.
pub fn at<'v>(v: &'v Value, path: &str) -> &'v Value {
    let mut cur = v;
    for k in path.split('.') {
        cur = match cur {
            Value::Object(m) => m.get(k).unwrap_or(&NULL),
            Value::Array(a) => k.parse::<usize>().ok().and_then(|i| a.get(i)).unwrap_or(&NULL),
            _ => &NULL,
        };
    }
    cur
}
pub fn s_of(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        _ => String::new(),
    }
}
pub fn f_of(v: &Value) -> Option<f64> {
    v.as_f64()
}

/// Sizes as Mantel's DeviceSettings shows them (1000s: 96 GB, 2.1 GB).
pub fn size(b: f64) -> String {
    let u = ["B", "KB", "MB", "GB", "TB"];
    let (mut x, mut i) = (b, 0);
    while x >= 1000.0 && i < u.len() - 1 {
        x /= 1000.0;
        i += 1;
    }
    // 61 GB, 3.1 GB (the mockups: no ".0")
    if i == 0 || x >= 100.0 || (x - x.round()).abs() < 0.05 { format!("{} {}", x.round(), u[i]) } else { format!("{:.1} {}", x, u[i]) }
}
pub fn rate(b: f64) -> String {
    if b <= 0.0 {
        "0".into()
    } else if b < 1e6 {
        format!("{} KB/s", (b / 1000.0).round())
    } else {
        format!("{:.1} MB/s", b / 1e6)
    }
}
pub fn thousands(n: f64) -> String {
    let s = format!("{}", n.round() as i64);
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 && c != '-' {
            out.push(',');
        }
        out.push(c);
    }
    out
}
fn opts(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect()
}

/// The device kind of a Performance key (cpu, memory, disk, net, gpu, battery).
pub fn kind_of(key: &str) -> &str {
    key.split(':').next().unwrap_or(key)
}

// ------------------------------------------------------------------ reading
/// One reading, off the window's thread, with a time limit. CI's pictures read HEARTH_TM_FIXTURES/<key>.json instead
/// (never set on a PC).
pub fn read_task(key: &str, cmd: Vec<String>, secs: u32) -> Task<Msg> {
    let key = key.to_string();
    if let Some(dir) = std::env::var_os("HEARTH_TM_FIXTURES") {
        let o = match std::fs::read_to_string(std::path::Path::new(&dir).join(format!("{key}.json"))) {
            Ok(t) => run::Output { ok: true, code: 0, out: t, err: String::new() },
            Err(_) => run::Output { ok: false, code: 1, out: String::new(), err: "not in this test".into() },
        };
        return Task::done(Msg::DevOut(key, o));
    }
    let mut full = vec!["timeout".to_string(), secs.to_string()];
    full.extend(cmd);
    Task::perform(run::spawn(full), move |o| Msg::DevOut(key.clone(), o))
}

fn py(helper: &str, args: &[&str]) -> Vec<String> {
    let mut v = vec!["python3".to_string(), run::shell_helper(helper)];
    v.extend(args.iter().map(|s| s.to_string()));
    v
}
fn hwset_read(args: &[&str], root: bool) -> Vec<String> {
    let mut v = if root { vec!["pkexec".to_string(), HWSET.to_string()] } else { vec!["python3".to_string(), HWSET.to_string()] };
    v.extend(args.iter().map(|s| s.to_string()));
    v
}

const NET_SNIPPET: &str = "c=$(nmcli -t -f NAME,TYPE connection show --active 2>/dev/null | grep -v ':loopback' | head -n1 | cut -d: -f1); m=$(nmcli -g connection.metered connection show \"$c\" 2>/dev/null); h=$(nmcli -t -f NAME connection show --active 2>/dev/null | grep -c '^Hotspot'); printf '{\"name\":\"%s\",\"metered\":\"%s\",\"hotspot\":%s}\\n' \"$c\" \"$m\" \"$h\"";

/// What the open tab reads (fast: every 3 s; slow: every 20 s), as DeviceSettings.qml's refresh() and slowRefresh().
pub fn fetch(a: &mut App, slow: bool) -> Task<Msg> {
    let kind = kind_of(&a.perf_dev).to_string();
    let mut want: Vec<(&str, Vec<String>, u32)> = Vec::new();
    if a.tab == "settings" && kind != "memory" {
        want.push(("hw", hwset_read(&["status"], false), 20));
        match kind.as_str() {
            "cpu" => want.push(("power", py("hearth-power", &["status"]), 20)),
            "net" => {
                want.push(("net", vec!["sh".into(), "-c".into(), NET_SNIPPET.into()], 15));
                want.push(("netapps", py("hearth-netinfo", &["apps"]), 20));
                if slow {
                    want.push(("netinfo", py("hearth-netinfo", &["info"]), 30));
                }
            }
            "disk" => {
                want.push(("drives", hwset_read(&["drives"], false), 20));
                want.push(("busy", py("hearth-tasks", &["disk-busy"]), 20));
                if slow {
                    want.push(("health", hwset_read(&["disk-health"], true), 90));
                    if !a.dev.contains_key("space") {
                        want.push(("space", hwset_read(&["space"], true), 600));
                    }
                }
            }
            "gpu" => want.push(("gpu", py("hearth-gputune", &["status"]), 20)),
            "battery" => want.push(("batt", py("hearth-tasks", &["battery-use"]), 20)),
            _ => {}
        }
    }
    if kind == "memory" {
        // Overview: your memory settings every 20 s (only read); Swap & settings: every 3 s unless you're saving.
        if (a.tab == "settings" && !a.mem_busy) || slow || !a.dev.contains_key("mem") {
            want.push(("mem", vec!["python3".into(), crate::memset::HELPER.into(), "status".into()], 20));
        }
        if !a.dev.contains_key("board") && !a.dev_busy.contains("board") {
            want.push(("board", hwset_read(&["memory-info"], true), 60));
        }
    }
    // Tailscale's other devices, for its entry in the list (mockup tm-5), every 20 s while Performance is open.
    if slow && has_tailscale(a) {
        want.push(("ts", ts_cmd(), 10));
    }
    let mut tasks = Vec::new();
    for (k, cmd, secs) in want {
        if a.dev_busy.insert(k.to_string()) {
            tasks.push(read_task(k, cmd, secs));
        }
    }
    Task::batch(tasks)
}

fn has_tailscale(a: &App) -> bool {
    a.sample.as_ref().map(|s| s.nets.iter().any(|n| n.kind == "Tailscale")).unwrap_or(false)
}
fn ts_cmd() -> Vec<String> {
    vec!["sh".into(), "-c".into(), "tailscale status --json 2>/dev/null || true".into()]
}
/// The first reading of Tailscale's devices (then every 20 s with the slow ones).
pub fn fetch_ts(a: &mut App) -> Task<Msg> {
    if a.dev.contains_key("ts") || !has_tailscale(a) || !a.dev_busy.insert("ts".into()) {
        return Task::none();
    }
    read_task("ts", ts_cmd(), 10)
}

pub fn on_out(a: &mut App, key: String, o: run::Output) {
    a.dev_busy.remove(&key);
    if key == "ts" {
        a.ts_peers = serde_json::from_str::<Value>(o.out.trim()).ok().and_then(|v| v.get("Peer").and_then(Value::as_object).map(|m| m.len() as i32)).unwrap_or(-1);
        a.dev.insert(key, Value::Bool(true));
        return;
    }
    match o.json() {
        Some(v) => {
            a.dev.insert(key, v);
        }
        None => {
            let said: String = if o.out.trim().is_empty() { o.why() } else { o.out.trim().to_string() };
            if key == "speed" {
                a.dev.insert(key, serde_json::json!({ "error": said }));
            } else if key == "fixed" {
                a.dev.insert(key, serde_json::json!({ "found": [said] }));
            } else if key == "board" || key == "space" {
                // asked once: not again on every tick when it can't be read here
                a.dev.insert(key, Value::Null);
            }
        }
    }
}

// ------------------------------------------------------------------ changing
/// Runs a change (through pkexec when it needs root), then reads the page again. Its last line says how it went.
pub fn act(a: &mut App, cmd: Vec<String>, ok: &str) -> Task<Msg> {
    if a.dev_busy.contains("act") {
        return Task::none();
    }
    if std::env::var_os("HEARTH_TM_FIXTURES").is_some() {
        a.dev_note = (ok.to_string(), false);
        return Task::none();
    }
    a.dev_busy.insert("act".into());
    let mut full = vec!["timeout".to_string(), "300".to_string()];
    full.extend(cmd);
    let ok = ok.to_string();
    Task::perform(run::spawn(full), move |o| Msg::DevActed(ok.clone(), o))
}

pub fn acted(a: &mut App, ok: String, o: run::Output) -> Task<Msg> {
    a.dev_busy.remove("act");
    let last = o.out.trim().lines().last().unwrap_or("").to_string();
    if let Some(e) = last.strip_prefix("error: ") {
        a.dev_note = (e.to_string(), true);
    } else if !o.ok {
        a.dev_note = (format!("That didn't work{}", if o.why().is_empty() { ".".to_string() } else { format!(": {}", o.why()) }), true);
    } else {
        a.dev_note = (ok, false);
    }
    a.slides.clear();
    if kind_of(&a.perf_dev) == "disk" {
        a.dev.remove("space");
    }
    let slow = matches!(kind_of(&a.perf_dev), "disk" | "net");
    fetch(a, slow)
}

fn hwset(a: &mut App, key: &str, v: &str) -> Task<Msg> {
    act(a, vec!["pkexec".into(), HWSET.into(), "set".into(), key.into(), v.into()], "Saved ✓")
}
fn gputune(a: &mut App, args: &[&str], ok: &str) -> Task<Msg> {
    act(a, py("hearth-gputune", args), ok)
}
fn host() -> String {
    std::env::var("HOSTNAME").ok().filter(|h| !h.is_empty()).or_else(|| std::fs::read_to_string("/etc/hostname").ok().map(|s| s.trim().to_string()).filter(|h| !h.is_empty())).unwrap_or_else(|| "Hearth".into())
}

/// A choice, a switch or a slider was changed (DeviceSettings.qml's onChose / onFlipped / onMoved).
pub fn pick(a: &mut App, key: &str, v: &str) -> Task<Msg> {
    let hw = a.dev.get("hw").cloned().unwrap_or(Value::Null);
    // hardware.json's keys have dots in them ("power.plugged"): looked up whole
    let saved = |k: &str| hw.get("saved").and_then(|x| x.get(k)).map(s_of).unwrap_or_default();
    if let Some(name) = key.strip_prefix("app.cpu:") {
        return act(a, vec!["pkexec".into(), HWSET.into(), "app-rule".into(), "cpu".into(), name.into(), v.into()], "Saved ✓");
    }
    if let Some(name) = key.strip_prefix("app.net:") {
        return act(a, vec!["pkexec".into(), HWSET.into(), "app-rule".into(), "net".into(), name.into(), v.into()], "Saved ✓");
    }
    match key {
        "palAdd" => {
            if v.is_empty() {
                return Task::none();
            }
            act(a, vec!["pkexec".into(), HWSET.into(), "app-rule".into(), "cpu".into(), v.into(), "50".into()], "Limited to 50% ✓ (change it on the left)")
        }
        "power.mode" => {
            // on battery, this is also the mode the PC goes back to when you unplug (When plugged in)
            let plugged = saved("power.plugged");
            let switches = !plugged.is_empty() && plugged != "off";
            if hw.get("onAC") == Some(&Value::Bool(false)) && switches {
                run::detached(&["pkexec", HWSET, "set", "power.unplugged", v]);
            }
            act(a, py("hearth-power", &["mode", v]), &format!("Power mode: {v}"))
        }
        "power.plugged" => {
            let active = a.dev.get("power").map(|p| s_of(at(p, "mode.active"))).unwrap_or_default();
            if v != "off" && saved("power.unplugged").is_empty() && !active.is_empty() {
                run::detached(&["pkexec", HWSET, "set", "power.unplugged", active.as_str()]);
            }
            hwset(a, "power.plugged", v)
        }
        "cpu.turbo" | "game.cores" | "updates.priority" | "disk.scrub" | "disk.warnPct" | "disk.commit" | "disk.compress" | "disk.snapshots" | "battery.warn" | "battery.at5"
        | "battery.limit" | "battery.saver" | "battery.dim" | "cpu.maxPct" => hwset(a, key, v),
        "optimize" => act(a, vec!["pkexec".into(), HWSET.into(), "optimize".into()], "Optimized ✓ (TRIM done)"),
        "cleanup" => act(a, vec!["pkexec".into(), HWSET.into(), "cleanup".into()], "Cleaned up ✓"),
        "metered" => {
            let name = a.dev.get("net").map(|n| s_of(at(n, "name"))).unwrap_or_default();
            if name.is_empty() {
                return Task::none();
            }
            act(a, vec!["nmcli".into(), "connection".into(), "modify".into(), name, "connection.metered".into(), v.into()], if v == "yes" { "Metered ✓" } else { "Not metered" })
        }
        "dns" => {
            if v == "custom" {
                return Task::none();
            }
            act(a, py("hearth-netinfo", &["dns", v]), &format!("DNS: {v} ✓"))
        }
        "powersave" => act(a, py("hearth-netinfo", &["powersave", v]), "Saved ✓"),
        "hotspot" => {
            let h = host();
            if v == "off" {
                act(a, vec!["nmcli".into(), "connection".into(), "down".into(), "Hotspot".into()], "Hotspot off")
            } else {
                act(a, vec!["nmcli".into(), "device".into(), "wifi".into(), "hotspot".into(), "ssid".into(), h], "Hotspot on ✓")
            }
        }
        "speed" | "fixed" => {
            if a.dev_busy.contains(key) {
                return Task::none();
            }
            a.dev_busy.insert(key.to_string());
            read_task(key, py("hearth-netinfo", &[if key == "speed" { "speedtest" } else { "fix" }]), if key == "speed" { 120 } else { 60 })
        }
        "gpu.fan" => {
            if v == "curve" {
                let c = curve_text(&fan_points(a));
                gputune(a, &["fan", "curve", c.as_str()], "My curve ✓")
            } else {
                gputune(a, &["fan", v], if v == "auto" { "Automatic ✓" } else { "Quiet ✓" })
            }
        }
        "gpu.curve" => gputune(a, &["fan", "curve", v], "Your fan curve ✓"),
        "gpu.power" => {
            let w = v.parse::<f64>().unwrap_or(0.0).round().to_string();
            gputune(a, &["power", w.as_str()], &format!("Power limit {w} W ✓"))
        }
        "gpu.undervolt" => gputune(a, &["undervolt", v], &format!("Undervolt {v} mV ✓")),
        "gpu.profile" => gputune(a, &["profile", v], &format!("Profile: {v} ✓")),
        "gpu.reset" => gputune(a, &["reset"], "Back to default ✓"),
        _ => Task::none(),
    }
}

// ------------------------------------------------------------------ the fan curve (mockup tm-6)
const DEFAULT_CURVE: &str = "30:20,50:30,65:45,75:65,90:100";
pub const FAN_H: f32 = 190.0;
pub const FAN_PAD: f32 = 16.0;

pub fn parse_curve(s: &str) -> Vec<(f32, f32)> {
    let v: Vec<(f32, f32)> = s
        .split(',')
        .filter_map(|p| {
            let (a, b) = p.split_once(':')?;
            Some((a.trim().parse().ok()?, b.trim().parse().ok()?))
        })
        .collect();
    if v.len() < 2 { parse_curve(DEFAULT_CURVE) } else { v }
}
pub fn curve_text(pts: &[(f32, f32)]) -> String {
    pts.iter().map(|(t, s)| format!("{}:{}", t.round(), s.round())).collect::<Vec<_>>().join(",")
}
/// The points on show: yours while you drag, else the saved curve.
pub fn fan_points(a: &App) -> Vec<(f32, f32)> {
    if let Some(p) = &a.fan_pts {
        return p.clone();
    }
    let c = a.dev.get("gpu").map(|g| s_of(at(g, "curve"))).unwrap_or_default();
    parse_curve(if c.is_empty() { DEFAULT_CURVE } else { &c })
}
/// A drag: the point keeps its temperature and takes the height you move it to, between its neighbours.
pub fn fan_move(a: &mut App, y: f32) {
    let Some(i) = a.fan_drag else { return };
    let mut p = fan_points(a);
    if i == 0 || i + 1 >= p.len() {
        return;
    }
    let s = (100.0 * (FAN_H - 14.0 - y) / (FAN_H - 28.0)).round();
    p[i].1 = s.clamp(p[i - 1].1, p[i + 1].1);
    a.fan_pts = Some(p);
}

struct Fan {
    pts: Vec<(f32, f32)>,
    bg: Color,
    line: Color,
    accent: Color,
    dot: Color,
}

impl<M> canvas::Program<M> for Fan {
    type State = ();
    fn draw(&self, _s: &(), renderer: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        let mut f = Frame::new(renderer, b.size());
        let x_of = |t: f32| FAN_PAD + (b.width - 2.0 * FAN_PAD) * (t - 30.0) / 60.0;
        let y_of = |s: f32| b.height - 14.0 - (b.height - 28.0) * s / 100.0;
        f.fill(&Path::rectangle(Point::ORIGIN, b.size()), self.bg);
        f.stroke(&Path::rectangle(Point::new(0.5, 0.5), iced::Size::new(b.width - 1.0, b.height - 1.0)), Stroke::default().with_color(self.line).with_width(1.0));
        if self.pts.len() >= 2 {
            let first = self.pts[0];
            let line = Path::new(|p| {
                p.move_to(Point::new(x_of((first.0 - 10.0).max(30.0)), y_of(first.1)));
                for (t, s) in &self.pts {
                    p.line_to(Point::new(x_of(*t), y_of(*s)));
                }
            });
            f.stroke(&line, Stroke::default().with_color(self.accent).with_width(2.5).with_line_join(LineJoin::Round).with_line_cap(LineCap::Round));
            for (t, s) in &self.pts[1..self.pts.len() - 1] {
                f.fill(&Path::circle(Point::new(x_of(*t), y_of(*s)), 6.0), self.dot);
            }
        }
        vec![f.into_geometry()]
    }
}

/// The curve with a strip over it for each point you can drag (the end points stay put, as in Mantel).
fn fan_curve<'a>(a: &'a App, can: bool) -> E<'a> {
    let p = a.p;
    let pts = fan_points(a);
    let g: E<'a> = canvas::Canvas::new(Fan { pts: pts.clone(), bg: p.bg, line: p.line, accent: p.accent, dot: p.text }).width(Length::Fill).height(Length::Fixed(FAN_H)).into();
    let labels = column![
        Space::new().height(Length::Fill),
        row![t1("30°C", 11.0, p.muted), Space::new().width(Length::Fill), t1("90°C", 11.0, p.muted)].padding(Padding { top: 0.0, right: 10.0, bottom: 2.0, left: 10.0 })
    ]
    .height(Length::Fixed(FAN_H));
    let mut lanes = Row::new().height(Length::Fixed(FAN_H)).push(Space::new().width(Length::Fixed(FAN_PAD)));
    let n = pts.len();
    // lane i spans from halfway to the point before to halfway to the point after (in °C, 30 … 90)
    let mut edges = vec![30.0f32];
    for i in 0..n.saturating_sub(1) {
        edges.push(((pts[i].0 + pts[i + 1].0) / 2.0).clamp(30.0, 90.0));
    }
    edges.push(90.0);
    for i in 0..n {
        let wdt = ((edges[i + 1] - edges[i]).max(0.0) * 10.0).round() as u16;
        if wdt == 0 {
            continue;
        }
        let lane = Space::new().width(Length::Fill).height(Length::Fill);
        let mut m = mouse_area(lane).on_move(Msg::FanMove).on_release(Msg::FanRelease);
        if can && i > 0 && i + 1 < n {
            m = m.on_press(Msg::FanPress(i)).interaction(mouse::Interaction::Grab);
        }
        lanes = lanes.push(container(m).width(Length::FillPortion(wdt)).height(Length::Fill));
    }
    lanes = lanes.push(Space::new().width(Length::Fixed(FAN_PAD)));
    stack![g, labels, lanes].width(Length::Fill).height(Length::Fixed(FAN_H)).into()
}

// ------------------------------------------------------------------ the page
/// What the page's top right says (as in the mockups): "Intel Core i3-N305 · 8 cores", "256 GB NVMe · SK hynix",
/// "Home-5G · Wi-Fi 6 · 866 Mbps · signal ▂▄▆█", "NVIDIA GeForce RTX 3060 · 12 GB", "Lenovo 82XB · 47 Wh".
pub fn headline(a: &App, s: &Sample) -> String {
    let hw = a.dev.get("hw").unwrap_or(&NULL);
    let key = a.perf_dev.clone();
    match kind_of(&key) {
        "cpu" => {
            let m = s_of(at(hw, "cpuModel"));
            format!("{} · {} cores", if m.is_empty() { s.cpu.name.clone() } else { m }, s.cpu.cores_physical)
        }
        "disk" => {
            let d = disk_of(a, s);
            let smart = smart_of(a, s);
            let model = if !d.model.is_empty() { d.model.clone() } else { s_of(at(smart, "model")) };
            let typ = if d.name.starts_with("nvme") { "NVMe" } else if d.ssd { "SSD" } else { "HDD" };
            [if d.size > 0 { format!("{} {typ}", size(d.size as f64)) } else { typ.into() }, model].iter().filter(|x| !x.is_empty()).cloned().collect::<Vec<_>>().join(" · ")
        }
        "net" => {
            let wifi = a.dev.get("netinfo").map(|n| at(n, "wifi")).unwrap_or(&NULL);
            let ssid = s_of(at(wifi, "ssid"));
            let n = net_of(a, s);
            if ssid.is_empty() {
                return format!("{}{}", n.name, if n.speed > 0 { format!(" · {} Mbps", n.speed) } else { String::new() });
            }
            let bars = f_of(at(wifi, "bars")).unwrap_or(0.0) as usize;
            let mbps = s_of(at(wifi, "mbps"));
            [ssid, s_of(at(wifi, "standard")), if mbps.is_empty() { String::new() } else { format!("{mbps} Mbps") }, if bars > 0 { format!("signal {}", "▂▄▆█".chars().take(bars.min(4)).collect::<String>()) } else { String::new() }]
                .iter()
                .filter(|x| !x.is_empty())
                .cloned()
                .collect::<Vec<_>>()
                .join(" · ")
        }
        "gpu" => {
            let g = gpu_of(a, s);
            format!("{}{}", g.name, if g.vram_total > 0 { format!(" · {} GB", (g.vram_total as f64 / 1073741824.0).round()) } else { String::new() })
        }
        "battery" => {
            let b = at(hw, "batteries.0");
            let m = s_of(at(b, "model"));
            let name = if m.is_empty() { s_of(at(b, "name")) } else { m };
            let wh = f_of(at(b, "designWh")).map(|x| format!("{x} Wh")).unwrap_or_default();
            [name, wh].iter().filter(|x| !x.is_empty()).cloned().collect::<Vec<_>>().join(" · ")
        }
        _ => String::new(),
    }
}

fn disk_of(a: &App, s: &Sample) -> crate::sample::Disk {
    let name = a.perf_dev.strip_prefix("disk:").unwrap_or("");
    s.disks.iter().find(|d| d.name == name).cloned().or_else(|| s.disks.first().cloned()).unwrap_or_default()
}
fn net_of(a: &App, s: &Sample) -> crate::sample::Net {
    let name = a.perf_dev.strip_prefix("net:").unwrap_or("");
    s.nets.iter().find(|d| d.name == name).cloned().unwrap_or_default()
}
fn gpu_of(a: &App, s: &Sample) -> crate::sample::Gpu {
    let i = a.perf_dev.strip_prefix("gpu:").unwrap_or("0");
    s.gpus.iter().find(|g| g.index.to_string() == i).cloned().or_else(|| s.gpus.first().cloned()).unwrap_or_default()
}
/// This drive's health from smartctl (hearth-hwset disk-health): by its name, or its name without the partition.
fn smart_of<'v>(a: &'v App, s: &Sample) -> &'v Value {
    let d = disk_of(a, s);
    let Some(h) = a.dev.get("health") else { return &NULL };
    let drives = at(h, "drives");
    let short = d.name.trim_end_matches(|c: char| c.is_ascii_digit()).trim_end_matches('p').to_string();
    for k in [d.name.clone(), short] {
        let v = at(drives, &k);
        if !v.is_null() {
            return v;
        }
    }
    &NULL
}

/// The numbers on top of each page (DeviceSettings.qml's topStats).
pub fn top_stats(a: &App, s: &Sample) -> Vec<(String, String, String, Option<Color>)> {
    let p = a.p;
    let hw = a.dev.get("hw").unwrap_or(&NULL);
    let dash = || "–".to_string();
    let x = |v: &str| v.to_string();
    match kind_of(&a.perf_dev) {
        "disk" => {
            let h = smart_of(a, s);
            let ok = at(h, "ok");
            let life = f_of(at(h, "lifeLeft"));
            let written = f_of(at(h, "written"));
            let c = at(hw, "c");
            vec![
                (
                    x("Health"),
                    if ok == &Value::Bool(false) { x("Failing") } else if ok == &Value::Bool(true) { x("Good") } else { dash() },
                    life.map(|l| format!("{l}% life left")).unwrap_or_default(),
                    if ok == &Value::Bool(false) { Some(p.danger) } else if ok == &Value::Bool(true) { Some(w::green()) } else { None },
                ),
                (
                    x("Written so far"),
                    written.map(size).unwrap_or_else(dash),
                    match (written, life) {
                        (Some(wr), Some(l)) if l < 100.0 => format!("of ~{} rated", size(wr * 100.0 / (100.0 - l).max(1.0))),
                        _ => String::new(),
                    },
                    None,
                ),
                (x("Temperature"), f_of(at(h, "temp")).map(|t| format!("{t}°C")).unwrap_or_else(dash), String::new(), None),
                (x("Free on C:"), f_of(at(c, "free")).map(size).unwrap_or_else(dash), f_of(at(c, "size")).map(|z| format!("of {}", size(z))).unwrap_or_default(), None),
                (x("Power-on hours"), f_of(at(h, "hours")).filter(|v| *v > 0.0).map(thousands).unwrap_or_else(dash), String::new(), None),
            ]
        }
        "net" => {
            let n = a.dev.get("netinfo").unwrap_or(&NULL);
            let ips = at(n, "ips.0");
            let ts = s_of(at(n, "tailscale"));
            let dns = at(n, "dns");
            vec![
                (x("Ping (to the internet)"), f_of(at(n, "ping")).map(|v| format!("{v} ms")).unwrap_or_else(dash), f_of(at(n, "jitter")).map(|j| format!("jitter {j} ms")).unwrap_or_default(), None),
                (x("Used today"), f_of(at(n, "today")).map(size).unwrap_or_else(dash), String::new(), None),
                (x("IP"), format!("{}{}", if ips.is_null() { dash() } else { s_of(ips) }, if ts.is_empty() { String::new() } else { format!(" · Tailscale {ts}") }), String::new(), None),
                (
                    x("DNS"),
                    if dns.is_null() {
                        dash()
                    } else if s_of(at(dns, "choice")) == "auto" {
                        x("Automatic (router)")
                    } else {
                        at(dns, "servers").as_array().map(|v| v.iter().map(s_of).collect::<Vec<_>>().join(", ")).unwrap_or_default()
                    },
                    String::new(),
                    None,
                ),
            ]
        }
        "gpu" => {
            let g = gpu_of(a, s);
            let tn = a.dev.get("gpu").unwrap_or(&NULL);
            let cap = f_of(at(tn, "powerCap")).or_else(|| f_of(at(tn, "powerMax")));
            // the game the frames are counted for (mockup tm-6: "71 Universe at War")
            let fps_game = if f_of(at(tn, "fps")).unwrap_or(0.0) > 0.0 { s.groups.iter().find(|g| g.kind == "game").map(|g| g.name.clone()).unwrap_or_default() } else { String::new() };
            vec![
                (x("Busy"), g.busy.map(|b| format!("{}%", b.round())).unwrap_or_else(dash), String::new(), None),
                (
                    x("Video memory"),
                    if g.vram_total > 0 { format!("{:.1}", g.vram_used as f64 / 1073741824.0) } else { dash() },
                    if g.vram_total > 0 { format!("/ {} GB", (g.vram_total as f64 / 1073741824.0).round()) } else { String::new() },
                    None,
                ),
                (x("Temperature"), g.temp.map(|t| format!("{}°C", t.round())).unwrap_or_else(dash), String::new(), None),
                (x("Power"), f_of(at(tn, "power")).map(|w| format!("{w} W")).unwrap_or_else(dash), cap.map(|c| format!("/ {}", c.round())).unwrap_or_default(), None),
                (x("Fan"), f_of(at(tn, "fanPct")).map(|v| format!("{v}%")).unwrap_or_else(dash), f_of(at(tn, "fanRpm")).filter(|v| *v > 0.0).map(|r| format!("{} rpm", thousands(r))).unwrap_or_default(), None),
                (x("Game FPS"), f_of(at(tn, "fps")).filter(|v| *v > 0.0).map(|f| format!("{}", f.round())).unwrap_or_else(dash), fps_game, None),
            ]
        }
        "battery" => {
            let b = s.battery.clone().unwrap_or_default();
            let hb = at(hw, "batteries.0");
            let health = f_of(at(hb, "health"));
            let hours = b.hours;
            vec![
                (x("Charge"), format!("{}%", b.percent), if b.status == "Discharging" { x("on battery") } else { b.status.to_lowercase() }, None),
                (x(if b.status == "Charging" { "Full in" } else { "Time left" }), if hours > 0.0 { format!("{} h {} min", hours.floor(), ((hours % 1.0) * 60.0).round()) } else { dash() }, String::new(), None),
                (x("Using now"), if b.watts > 0.0 { format!("{:.1} W", b.watts) } else { dash() }, String::new(), None),
                (
                    x("Health"),
                    health.map(|h| format!("{h}%")).unwrap_or_else(dash),
                    match (f_of(at(hb, "fullWh")), f_of(at(hb, "designWh"))) {
                        (Some(f), Some(d)) => format!("{f} of {d} Wh"),
                        _ => String::new(),
                    },
                    if health.unwrap_or(0.0) >= 80.0 { Some(w::green()) } else { None },
                ),
                (x("Charge cycles"), f_of(at(hb, "cycles")).filter(|v| *v > 0.0).map(|c| format!("{c}")).unwrap_or_else(dash), String::new(), None),
            ]
        }
        _ => vec![],
    }
}

/// Each core's use (core 0 with its speed), the strip on top of CPU's pages (mockup tm-3).
pub fn cores_strip<'a>(a: &'a App, s: &Sample) -> E<'a> {
    let p = a.p;
    let mut strip = Row::new().spacing(6.0);
    let n = s.cpu.cores.len();
    if n <= 1 {
        return Space::new().into();
    }
    for (i, c) in s.cpu.cores.iter().enumerate() {
        let label = if i == 0 && s.cpu.mhz.is_some() { format!("0 · {:.1} GHz", s.cpu.mhz.unwrap_or(0) as f32 / 1000.0) } else { i.to_string() };
        let h = 64.0;
        let fill = (c.min(100.0) / 100.0 * (h - 2.0)).max(2.0);
        let bar = container(column![Space::new().height(Length::Fill), container(Space::new()).width(Length::Fill).height(Length::Fixed(fill)).style(ui::boxed(Pal::a(p.accent, 0.55), 0.0, None))])
            .padding(1.0)
            .width(Length::Fill)
            .height(Length::Fixed(h))
            .style(ui::boxed(Color::TRANSPARENT, 0.0, Some((p.line, 1.0))));
        let label = container(t1(label, 11.0, p.muted)).padding(Padding { top: 4.0, right: 0.0, bottom: 0.0, left: 6.0 });
        strip = strip.push(stack![bar, label].width(Length::FillPortion(1)));
    }
    strip.into()
}

/// The tab's body (DeviceSettings.qml's cards), with the note of the last change under it.
pub fn view<'a>(a: &'a App, s: &Sample) -> E<'a> {
    let p = a.p;
    let kind = kind_of(&a.perf_dev).to_string();
    let mut col = Column::new().spacing(14.0);
    let st = top_stats(a, s);
    if !st.is_empty() {
        col = col.push(w::stats(&p, st));
    }
    let rows: Vec<E<'a>> = match kind.as_str() {
        "cpu" => cpu(a, s),
        "disk" => disk(a, s),
        "net" => net(a, s),
        "gpu" => gpu(a, s),
        "battery" => battery(a, s),
        _ => vec![],
    };
    if kind == "cpu" {
        col = col.push(cores_strip(a, s));
    }
    for r in rows {
        col = col.push(r);
    }
    if !a.dev_note.0.is_empty() {
        col = col.push(t(a.dev_note.0.clone(), 12.0, if a.dev_note.1 { p.danger } else { p.muted }));
    }
    col.into()
}

fn saved(a: &App, k: &str) -> String {
    a.dev.get("hw").and_then(|hw| hw.get("saved")).and_then(|x| x.get(k)).map(s_of).unwrap_or_default()
}
fn saved_or(a: &App, k: &str, d: &str) -> String {
    let v = saved(a, k);
    if v.is_empty() { d.to_string() } else { v }
}
fn busy(a: &App) -> bool {
    a.dev_busy.contains("act")
}
fn small_btn<'a>(a: &'a App, label: &'a str, key: &str) -> E<'a> {
    let b = busy(a) || a.dev_busy.contains(key);
    btn(&a.p, if b { "…" } else { label }, Kind::Normal, false, if b { None } else { Some(Msg::Pick(key.to_string(), String::new())) })
}
fn sw<'a>(a: &'a App, key: &str, on: bool) -> E<'a> {
    w::toggle(&a.p, on, Some(Msg::Pick(key.to_string(), if on { "off".into() } else { "on".into() })))
}
fn slider_v(a: &App, key: &str, v: f32) -> f32 {
    a.slides.get(key).copied().unwrap_or(v)
}

// ---- CPU (tm-3)
fn cpu<'a>(a: &'a App, s: &Sample) -> Vec<E<'a>> {
    let p = a.p;
    let hw = a.dev.get("hw").unwrap_or(&NULL);
    let mode = a.dev.get("power").map(|x| s_of(at(x, "mode.active"))).unwrap_or_default();
    let mut pm = vec![w::line(&p, "Mode", "", w::chips(&p, "power.mode", &opts(&[("power-saver", "Battery saver"), ("balanced", "Balanced"), ("performance", "Performance")]), &mode))];
    if hw.get("onAC").map(|v| v.is_boolean()).unwrap_or(false) {
        pm.push(w::line(
            &p,
            "When plugged in",
            "switches by itself when you plug in or unplug",
            w::dropdown(&p, "power.plugged", opts(&[("off", "Don't switch"), ("performance", "Performance"), ("balanced", "Balanced"), ("power-saver", "Battery saver")]), &saved_or(a, "power.plugged", "off")),
        ));
    }
    if at(hw, "turbo.can") == &Value::Bool(true) {
        pm.push(w::line(&p, "Turbo boost", "faster bursts, more heat", sw(a, "cpu.turbo", at(hw, "turbo.on") == &Value::Bool(true))));
    }
    let power = w::card(&p, "Power mode", "How hard the processor works. Changes the fans and battery life too.", false, pm);

    let mut lh = Vec::new();
    if at(hw, "maxPct.can") == &Value::Bool(true) {
        let v = slider_v(a, "cpu.maxPct", f_of(at(hw, "maxPct.value")).unwrap_or(100.0) as f32);
        lh.push(w::line(&p, "Max processor speed", "100% = no limit", w::slide("cpu.maxPct", 30.0, 100.0, 5.0, v, format!("{}%", v.round()), &p)));
    }
    let limit = f_of(at(hw, "tempLimit"));
    lh.push(w::line(
        &p,
        "Temperature now · limit",
        "",
        w::value(&p, format!("{} · {}", s.cpu.temp.map(|t| format!("{}°C", t.round())).unwrap_or_else(|| "–".into()), limit.map(|l| format!("{l}°C")).unwrap_or_else(|| "–".into())), None),
    ));
    let thr = if hw.is_null() { None } else { f_of(at(hw, "throttledToday")).or_else(|| f_of(at(hw, "throttled"))) };
    lh.push(w::line(
        &p,
        "Throttling",
        "when the CPU slowed itself down because of heat",
        w::value(
            &p,
            match thr {
                None => "–".into(),
                Some(n) if n == 0.0 => "none today".into(),
                Some(n) if n == 1.0 => "1 time today".into(),
                Some(n) => format!("{n} times today"),
            },
            if thr == Some(0.0) { Some(w::green()) } else { None },
        ),
    ));
    let heat = w::card(&p, "Limits & heat", "Keep the laptop cool and quiet, or let it go all out.", false, lh);

    // Game cores (mockup tm-3): off unless you turn it on (hearth-hwset game.cores)
    let gc = at(hw, "gameCores");
    let game: Option<E<'a>> = if at(gc, "can") == &Value::Bool(true) {
        let val = s_of(at(gc, "value"));
        let on = !val.is_empty() && val != "off";
        let mut rows = vec![w::line(&p, "Reserve for games", "only while a game is running; everything shares every core again after", sw(a, "game.cores", on))];
        if on {
            let n = f_of(at(gc, "threads")).unwrap_or(8.0) as usize;
            let o = |from: usize| (format!("{from}-{}", n - 1), format!("{} of {n} (cores {from}–{})", n - from, n - 1));
            let list = vec![o(n / 4), o(n / 2), o(3 * n / 4)];
            rows.push(w::line(&p, "Cores for the game", "", w::dropdown(&p, "game.cores", list, &val)));
            let first: usize = val.split(|c: char| !c.is_ascii_digit()).next().and_then(|x| x.parse().ok()).unwrap_or(0);
            rows.push(w::line(&p, "Background work (AI, updates, builds)", "runs on the other cores at low priority", w::value(&p, if first > 0 { format!("Cores 0–{}", first - 1) } else { "–".into() }, None)));
        }
        Some(w::card(&p, "Game cores", "Give the game you're playing its own cores, so background work never stutters it.", true, rows))
    } else {
        None
    };

    // Per-app limits (mockup tm-3): kept, put back whenever it runs (hearth-app-rules.service)
    let rules = hw.get("saved").and_then(|x| x.get("apps.cpu")).and_then(Value::as_object).cloned().unwrap_or_default();
    let pal_opts = opts(&[("low", "Low priority"), ("25", "Max 25%"), ("50", "Max 50%"), ("75", "Max 75%"), ("remove", "No limit")]);
    let mut names: Vec<&String> = rules.keys().collect();
    names.sort();
    let mut pr = Vec::new();
    for n in names {
        let g = s.groups.iter().find(|g| &g.name == n || crate::apps::basename(&g.exe) == *n);
        pr.push(w::line(
            &p,
            n.clone(),
            g.map(|g| format!("PID {} · its AI jobs included", g.pid)).unwrap_or_else(|| "applies whenever it runs".into()),
            w::dropdown(&p, &format!("app.cpu:{n}"), pal_opts.clone(), &s_of(&rules[n.as_str()])),
        ));
    }
    pr.push(w::line(&p, "Updates and downloads", "", w::dropdown(&p, "updates.priority", opts(&[("low", "Low priority"), ("normal", "Normal")]), &saved_or(a, "updates.priority", "normal"))));
    let mut add: Vec<(String, String)> = s
        .groups
        .iter()
        .filter(|g| g.kind == "app" || g.kind == "game")
        .map(|g| (if g.exe.is_empty() { g.name.clone() } else { crate::apps::basename(&g.exe) }, g.name.clone()))
        .filter(|(k, _)| !rules.contains_key(k))
        .collect();
    add.push((String::new(), "Cancel".into()));
    pr.push(row![w::link(&p, "+ Add a program", Msg::DropOpen("palAdd".into(), add, "-".into())), w::spacer()].into());
    let pal = w::card(&p, "Per-app limits", "Stop one program from taking the whole processor.", true, pr);

    vec![w::pair(power, heat), match game {
        Some(g) => w::pair(g, pal),
        None => w::pair(pal, Space::new().into()),
    }]
}

// ---- Disk (tm-4)
fn disk<'a>(a: &'a App, s: &Sample) -> Vec<E<'a>> {
    let p = a.p;
    let hw = a.dev.get("hw").unwrap_or(&NULL);
    let d = disk_of(a, s);
    let btrfs = s_of(at(hw, "home.type")) == "btrfs";
    let drives = a.dev.get("drives").unwrap_or(&NULL);
    let drive = at(drives, "drives").as_array().and_then(|l| l.iter().find(|x| { let n = s_of(at(x, "name")); !n.is_empty() && d.name.starts_with(&n) }));
    let last = s_of(at(drives, "timer.last"));
    let trim_sub = match drive {
        Some(x) if at(x, "trim") == &Value::Bool(false) => s_of(at(x, "what")),
        _ if !last.is_empty() => {
            // "Mon 2026-11-10 03:00:01 CST" → "2026-11-10"
            let mut w: Vec<&str> = last.split_whitespace().collect();
            if w.len() > 1 {
                w.remove(0);
            }
            format!("last run {}", w.first().copied().unwrap_or(""))
        }
        _ => "not run yet".into(),
    };
    let mut opt = vec![w::line(&p, "TRIM", format!("{trim_sub} · weekly"), small_btn(a, "Optimize now", "optimize"))];
    if btrfs {
        let scrub = a.dev.get("health").map(|h| at(h, "scrub")).unwrap_or(&NULL);
        let sl = s_of(at(scrub, "last"));
        opt.push(w::line(
            &p,
            "Check the file system (btrfs scrub)",
            format!("reads everything and checks it isn't damaged{}", if sl.is_empty() { String::new() } else { format!(" · last {sl}{}", if at(scrub, "ok") == &Value::Bool(false) { " · it found problems" } else { "" }) }),
            w::dropdown(&p, "disk.scrub", opts(&[("off", "Off"), ("weekly", "Weekly"), ("monthly", "Monthly")]), &saved_or(a, "disk.scrub", "off")),
        ));
    }
    opt.push(w::line(
        &p,
        "Warn when C: is over",
        "",
        w::dropdown(&p, "disk.warnPct", opts(&[("80", "80% full"), ("85", "85% full"), ("90", "90% full"), ("95", "95% full"), ("off", "Only when nearly full")]), &saved_or(a, "disk.warnPct", "90")),
    ));
    let optimize = w::card(&p, "Optimize", "SSDs are trimmed, never defragmented. Hard drives get a fragmentation check.", false, opt);

    let mut sp: Vec<E<'a>> = Vec::new();
    match a.dev.get("space").and_then(|v| at(v, "rows").as_array().cloned()) {
        None => sp.push(t(if a.dev.contains_key("space") { "Couldn't measure here." } else { "Measuring…" }, 13.0, p.muted).into()),
        Some(rows) => {
            for r in rows {
                let snaps = f_of(at(&r, "snapshots")).unwrap_or(0.0);
                let mut tail = Row::new().spacing(10.0).align_y(iced::Center).push(w::value(&p, size(f_of(at(&r, "bytes")).unwrap_or(0.0)), None));
                if at(&r, "clean") == &Value::Bool(true) {
                    tail = tail.push(small_btn(a, "Clean up", "cleanup"));
                }
                sp.push(w::line(&p, s_of(at(&r, "label")), if s_of(at(&r, "id")) == "restore" && snaps > 0.0 { format!("{snaps} snapshots of your files too") } else { String::new() }, tail.into()));
            }
        }
    }
    let space = w::card(&p, "What's using space", "C: at a glance, biggest first.", true, sp);

    let mut ws: Vec<E<'a>> = Vec::new();
    if btrfs {
        let commit = { let x = saved(a, "disk.commit"); if x.is_empty() { let h = s_of(at(hw, "home.commit")); if h.is_empty() { "30".into() } else { h } } else { x } };
        ws.push(w::line(&p, "Save to the drive every", "shorter = less lost if the power cuts", w::chips(&p, "disk.commit", &opts(&[("5", "5 s"), ("10", "10 s"), ("30", "30 s")]), &commit)));
        let comp = { let x = saved(a, "disk.compress"); if x.is_empty() { let h = s_of(at(hw, "home.compress")); if h.is_empty() { "on".into() } else { h } } else { x } };
        ws.push(w::line(&p, "Compress files on C: (zstd)", "saves space, a little CPU", sw(a, "disk.compress", comp == "on")));
        ws.push(w::line(
            &p,
            "Snapshots of your files",
            "Previous versions in Hearth Files",
            w::dropdown(&p, "disk.snapshots", opts(&[("off", "Off"), ("hourly", "Every hour"), ("daily", "Every day")]), &saved_or(a, "disk.snapshots", "off")),
        ));
    } else if !hw.is_null() {
        ws.push(t("Your files aren't on btrfs here, so these don't apply.", 12.0, p.muted).into());
    } else {
        ws.push(t("Reading…", 12.0, p.muted).into());
    }
    let writes = w::card(&p, "Writes & safety", "", false, ws);

    let rows: Vec<Vec<Cell>> = a
        .dev
        .get("busy")
        .and_then(|v| at(v, "rows").as_array().cloned())
        .unwrap_or_default()
        .iter()
        .map(|r| {
            let pid = f_of(at(r, "pid")).unwrap_or(0.0);
            vec![Cell::T(s_of(at(r, "name"))), Cell::M(if pid > 0.0 { format!("{pid}") } else { "—".into() }), Cell::T(rate(f_of(at(r, "read")).unwrap_or(0.0))), Cell::T(rate(f_of(at(r, "write")).unwrap_or(0.0)))]
        })
        .collect();
    let busiest = w::card(&p, "Busiest programs", "", false, vec![w::table(&p, &[("Program", 0.0, false), ("PID", 80.0, true), ("Read", 90.0, true), ("Write", 90.0, true)], rows)]);
    vec![w::pair(optimize, space), w::pair(writes, busiest)]
}

// ---- Network (tm-5)
fn allow_options(cur: &str) -> Vec<(String, String)> {
    let mut o = opts(&[("allow", "Allowed"), ("limit:1", "Limit to 1 MB/s"), ("limit:5", "Limit to 5 MB/s"), ("limit:20", "Limit to 20 MB/s"), ("block", "Blocked (offline game)")]);
    if !cur.is_empty() && !o.iter().any(|x| x.0 == cur) {
        o.push((cur.to_string(), format!("Limit to {} MB/s", cur.trim_start_matches("limit:"))));
    }
    o
}

fn net<'a>(a: &'a App, _s: &Sample) -> Vec<E<'a>> {
    let p = a.p;
    let apps = a.dev.get("netapps").and_then(|v| at(v, "apps").as_array().cloned()).unwrap_or_default();
    let rows: Vec<Vec<Cell>> = apps
        .iter()
        .take(12)
        .map(|x| {
            let pid = f_of(at(x, "pid")).unwrap_or(0.0);
            let hosts: Vec<String> = at(x, "hosts").as_array().map(|h| h.iter().map(s_of).collect()).unwrap_or_default();
            let talking = if hosts.is_empty() { "—".to_string() } else { format!("{}{}", hosts.iter().take(2).cloned().collect::<Vec<_>>().join(", "), if hosts.len() > 2 { format!(" +{}", hosts.len() - 2) } else { String::new() }) };
            let name = s_of(at(x, "name"));
            let allow = { let v = s_of(at(x, "allow")); if v.is_empty() { "allow".to_string() } else { v } };
            vec![
                Cell::T(name.clone()),
                Cell::M(if pid > 0.0 { format!("{pid}") } else { "—".into() }),
                Cell::T(rate(f_of(at(x, "down")).unwrap_or(0.0))),
                Cell::T(rate(f_of(at(x, "up")).unwrap_or(0.0))),
                Cell::T({ let t = f_of(at(x, "today")).unwrap_or(0.0); if t > 0.0 { size(t) } else { "0".into() } }),
                Cell::T(talking),
                Cell::Drop(format!("app.net:{name}"), allow_options(&allow), allow),
            ]
        })
        .collect();
    let mut body = vec![w::table(&p, &[("Program", -2.0, false), ("PID", 70.0, true), ("Down", 76.0, true), ("Up", 70.0, true), ("Today", 70.0, true), ("Talking to", -3.0, false), ("Allow", 180.0, false)], rows)];
    if !a.dev.contains_key("netapps") {
        body.push(t("Looking…", 12.0, p.muted).into());
    }
    body.push(t("Blocked and limited programs stay that way whenever they run, until you change them here.", 11.0, p.muted).into());
    let which = w::card(&p, "Which programs use the network", "", true, body);

    let netc = a.dev.get("net").unwrap_or(&NULL);
    let name = s_of(at(netc, "name"));
    let metered = s_of(at(netc, "metered")).starts_with("yes");
    let info = a.dev.get("netinfo").unwrap_or(&NULL);
    let mut dns_opts = opts(&[("auto", "Automatic"), ("cloudflare", "Cloudflare (1.1.1.1)"), ("google", "Google (8.8.8.8)"), ("quad9", "Quad9 (9.9.9.9)")]);
    let choice = { let c = s_of(at(info, "dns.choice")); if c.is_empty() { "auto".to_string() } else { c } };
    if choice == "custom" {
        dns_opts.push(("custom".into(), "Your own".into()));
    }
    let mut con = vec![
        w::line(&p, "Metered connection", "pauses updates and big downloads (phone hotspots)", w::toggle(&p, metered, if name.is_empty() { None } else { Some(Msg::Pick("metered".into(), if metered { "no".into() } else { "yes".into() })) })),
        w::line(&p, "DNS", "", w::dropdown(&p, "dns", dns_opts, &choice)),
    ];
    if let Some(ps) = at(info, "powersave").as_bool() {
        con.push(w::line(&p, "Wi-Fi power saving", "off = lower ping in games", sw(a, "powersave", ps)));
    }
    let connection = w::card(&p, "Connection", if name.is_empty() { "Not connected".to_string() } else { name }, false, con);

    let speed = a.dev.get("speed");
    let speed_sub = if a.dev_busy.contains("speed") {
        "testing… (about 30 seconds)".to_string()
    } else {
        match speed {
            Some(v) if !at(v, "error").is_null() => s_of(at(v, "error")).trim_start_matches("error: ").to_string(),
            Some(v) => format!("↓ {} Mbps · ↑ {} Mbps · {} ms", s_of(at(v, "down")), s_of(at(v, "up")), s_of(at(v, "latency"))),
            None => String::new(),
        }
    };
    let fix_sub = if a.dev_busy.contains("fixed") {
        "looking…".to_string()
    } else {
        match a.dev.get("fixed") {
            Some(v) => at(v, "found").as_array().map(|f| f.iter().map(s_of).collect::<Vec<_>>().join(" · ")).unwrap_or_default(),
            None => "resets Wi-Fi and DNS, tells you what it found".into(),
        }
    };
    let hotspot = f_of(at(netc, "hotspot")).unwrap_or(0.0) > 0.0;
    let hs_title: E<'a> = row![t("Share this connection (hotspot)", 14.0, p.text), w::new_tag(&p)].spacing(8.0).align_y(iced::Center).into();
    let hs: E<'a> = row![
        column![hs_title, t(format!("network name: {} · the password is in Settings › Wi-Fi", host()), 11.0, p.muted)].spacing(2.0).width(Length::Fill),
        sw(a, "hotspot", hotspot)
    ]
    .spacing(12.0)
    .align_y(iced::Center)
    .into();
    let tools = w::card(&p, "Tools", "", false, vec![w::line(&p, "Speed test", speed_sub, small_btn(a, "Run", "speed")), w::line(&p, "Fix my connection", fix_sub, small_btn(a, "Run", "fixed")), hs]);
    vec![which, w::pair(connection, tools)]
}

// ---- GPU (tm-6)
fn gpu<'a>(a: &'a App, s: &Sample) -> Vec<E<'a>> {
    let p = a.p;
    let tn = a.dev.get("gpu").unwrap_or(&NULL);
    let installed = at(tn, "installed") == &Value::Bool(true);
    let fan_sub = if !tn.is_null() && !installed { "LACT isn't installed on this Hearth, so the fans can't be tuned here." } else { "Faster fans as it gets hotter. Drag the points." };
    let mode = { let m = s_of(at(tn, "fanMode")); if m.is_empty() { "auto".to_string() } else { m } };
    let fan = w::card(
        &p,
        "Fan curve",
        fan_sub,
        false,
        vec![fan_curve(a, installed), w::line(&p, "Mode", "", w::chips(&p, "gpu.fan", &opts(&[("auto", "Automatic"), ("curve", "My curve"), ("quiet", "Quiet")]), &mode))],
    );
    let mut pc = Vec::new();
    let pmax = f_of(at(tn, "powerMax"));
    let pcap = f_of(at(tn, "powerCap"));
    if pmax.is_some() || pcap.is_some() {
        let from = f_of(at(tn, "powerMin")).unwrap_or(10.0) as f32;
        let to = pmax.unwrap_or(300.0) as f32;
        let v = slider_v(a, "gpu.power", pcap.or(pmax).unwrap_or(0.0) as f32);
        pc.push(w::line(&p, "Power limit", "", w::slide("gpu.power", from, to.max(from + 5.0), 5.0, v, format!("{} W", v.round()), &p)));
    }
    if at(tn, "canUndervolt") == &Value::Bool(true) {
        let uv = f_of(at(tn, "undervolt")).unwrap_or(0.0).round().to_string();
        pc.push(w::line(&p, "Undervolt", "same speed, cooler and quieter", w::dropdown(&p, "gpu.undervolt", opts(&[("0", "None"), ("-25", "−25 mV"), ("-50", "−50 mV"), ("-75", "−75 mV"), ("-100", "−100 mV")]), &uv)));
    }
    let prof = { let x = s_of(at(tn, "profile")); if x.is_empty() { "balanced".to_string() } else { x } };
    pc.push(w::line(&p, "Profile", "", w::chips(&p, "gpu.profile", &opts(&[("quiet", "Quiet"), ("balanced", "Balanced"), ("max", "Max")]), &prof)));
    pc.push(row![w::spacer(), btn(&p, "Back to default", Kind::Normal, false, if busy(a) { None } else { Some(Msg::Pick("gpu.reset".into(), String::new())) })].into());
    let clocks = w::card(&p, "Power & clocks", "Off until you change something. Every change remembered and undoable (LACT).", false, pc);

    let mut gs: Vec<&crate::sample::Group> = s.groups.iter().filter(|g| g.gpu > 0.0 || g.vram > 0).collect();
    gs.sort_by(|x, y| y.gpu.partial_cmp(&x.gpu).unwrap_or(std::cmp::Ordering::Equal));
    let rows: Vec<Vec<Cell>> = gs
        .iter()
        .take(10)
        .map(|g| {
            vec![
                Cell::Tag(g.name.clone(), if g.tag == "Game · Proton" { "Proton · DXVK".into() } else { String::new() }),
                Cell::M(g.pid.to_string()),
                Cell::T(format!("{}%", g.gpu.round())),
                Cell::T(if g.vram > 0 { size(g.vram as f64) } else { "—".into() }),
                Cell::T(if g.uses.is_empty() { "—".into() } else { g.uses.join(", ") }),
            ]
        })
        .collect();
    let mut body = vec![w::table(&p, &[("Program", 0.0, false), ("PID", 80.0, true), ("GPU", 70.0, true), ("Video memory", 120.0, true), ("Uses", 160.0, false)], rows)];
    if gs.is_empty() {
        body.push(t(if s.has_procs { "Nothing is using the graphics card right now." } else { "Looking…" }, 12.0, p.muted).into());
    }
    let progs = w::card(&p, "Programs on the graphics card", "", false, body);
    vec![w::pair(fan, clocks), progs]
}

// ---- Battery (tm-7)
fn battery<'a>(a: &'a App, s: &Sample) -> Vec<E<'a>> {
    let p = a.p;
    let hw = a.dev.get("hw").unwrap_or(&NULL);
    let warn = saved_or(a, "battery.warn", "10,5");
    let last: String = if warn == "off" { "5".into() } else { warn.split(',').last().unwrap_or("5").to_string() };
    let hib = s_of(at(hw, "hibernate"));
    let works = hib == "works";
    let mut at5 = Vec::new();
    if works {
        at5.push(("hibernate".to_string(), "Hibernate".to_string()));
    }
    at5.extend(opts(&[("shutdown", "Shut down"), ("sleep", "Sleep")]));
    let at5v = { let v = saved_or(a, "battery.at5", "hibernate"); if v == "hibernate" && !works { "shutdown".to_string() } else { v } };
    let low = w::card(
        &p,
        "When the battery runs low",
        "So it never dies with your work open.",
        true,
        vec![
            w::line(&p, "Warn at", "", w::dropdown(&p, "battery.warn", opts(&[("10,5", "10% and 5%"), ("15,5", "15% and 5%"), ("20,10", "20% and 10%"), ("off", "Don't warn")]), &warn)),
            w::line(&p, format!("At {last}%"), "hibernate is offered only once it's proven to work on this PC", w::chips(&p, "battery.at5", &at5, &at5v)),
            w::line(
                &p,
                "Hibernation on this laptop",
                "",
                w::value(&p, if hw.is_null() { "–" } else if works { "tested · works" } else if hib == "untested" { "not tested yet" } else { "not available" }, Some(if works { w::green() } else { p.muted })),
            ),
        ],
    );
    let bat = at(hw, "batteries.0");
    let mut look = Vec::new();
    if at(bat, "canLimit") == &Value::Bool(true) {
        let lim = f_of(at(bat, "limit")).map(|l| if l >= 95.0 { "100".to_string() } else { format!("{l}") }).unwrap_or_else(|| "100".into());
        look.push(w::line(&p, "Stop charging at", "longer battery life if it's plugged in a lot", w::chips(&p, "battery.limit", &opts(&[("60", "60%"), ("80", "80%"), ("100", "100%")]), &lim)));
    }
    look.push(w::line(&p, "Battery saver from", "", w::dropdown(&p, "battery.saver", opts(&[("off", "Off"), ("10", "10%"), ("20", "20%"), ("30", "30%"), ("50", "50%")]), &saved_or(a, "battery.saver", "off"))));
    look.push(w::line(&p, "Dim the screen on battery", "", sw(a, "battery.dim", saved(a, "battery.dim") == "on")));
    let care = w::card(&p, "Look after the battery", "", false, look);

    let use_rows = a.dev.get("batt").and_then(|v| at(v, "rows").as_array().cloned());
    let rows: Vec<Vec<Cell>> = use_rows
        .clone()
        .unwrap_or_default()
        .iter()
        .map(|r| {
            let name = s_of(at(r, "name"));
            let screen = at(r, "screen") == &Value::Bool(true);
            let g = if screen { None } else { s.groups.iter().find(|g| g.name == name) };
            vec![
                Cell::T(name.clone()),
                Cell::M(g.map(|g| g.pid.to_string()).unwrap_or_else(|| "—".into())),
                Cell::T(format!("{}%", s_of(at(r, "share")))),
                Cell::T(f_of(at(r, "watts")).map(|x| format!("{x} W")).unwrap_or_else(|| "—".into())),
                Cell::T(s_of(at(r, "tip"))),
            ]
        })
        .collect();
    let mut body = Vec::new();
    if use_rows.as_ref().map(|r| r.is_empty()).unwrap_or(false) {
        body.push(t("Nothing yet today: it counts while the PC runs on battery.", 13.0, p.muted).into());
    }
    body.push(w::table(&p, &[("Program", 0.0, false), ("PID", 80.0, true), ("Share", 70.0, true), ("Power now", 100.0, true), ("Tip", 280.0, false)], rows));
    let used = w::card(&p, "What used the battery today", "", false, body);
    vec![w::pair(low, care), used]
}
