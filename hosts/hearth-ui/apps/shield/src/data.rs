//! Where Hearth Shield's numbers come from, and where its changes go:
//!
//! * **On this PC** (Hearth's own): AdGuard Home (GPL-3) as Server mode runs it (hearth-server-dns: a system container,
//!   never part of starting the PC). Numbers from its local API (127.0.0.1:3053, this PC only), Hearth's choices from
//!   /run/hearth/server-dns/status.json, each day's totals from days.json; changes through `pkexec hearth-server-dns`
//!   (the one place they're kept, written safely there).
//! * **A Pi-hole** (EUPL-1.2) on your network: its v6 API, signed in with its password from the system keyring (never
//!   on a command line: curl reads it from stdin).
//!
//! Either way the devices only use it while it answers (hearth-dns-client on Hearth devices; the router's second
//! address for the rest), so Shield can never cut a device or the network off.
//!
//! $HEARTH_SHIELD_FIXTURE: a JSON file of the answers by path (CI's pictures); nothing is changed then.

use serde_json::{json, Value};

pub const API: &str = "http://127.0.0.1:3053";
pub const HELPER: &str = "/usr/libexec/nimbus/hearth-server-dns";
pub const RUN: &str = "/run/hearth/server-dns";

/// The day: 24 hours of (asked, blocked).
#[derive(Debug, Clone, Default)]
pub struct Data {
    pub asked: u64,
    pub blocked: u64,
    pub devices_seen: usize,
    pub addresses: u64,
    pub lists_n: usize,
    pub lists_at: String,
    pub hours: Vec<(u64, u64)>,
    pub devices: Vec<Dev>,
    pub top: Vec<(String, String, u64)>,
    pub lists: Vec<List>,
    pub allow: Vec<String>,
    pub block: Vec<String>,
    pub safe: bool,
    pub log: Vec<Q>,
    pub on: bool,
    pub running: bool,
    pub paused_until: u64,
    /// earlier days (7 days, 30 days): asked, blocked
    pub days: Vec<(String, u64, u64)>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Dev {
    pub id: String,
    pub name: String,
    pub about: String,
    pub asked: u64,
    pub blocked: u64,
    /// "Using it", "Off", "Paused 42 min"
    pub state: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct List {
    pub id: String,
    pub label: String,
    pub on: bool,
    pub count: u64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Q {
    pub at: String,
    pub domain: String,
    pub who: String,
    pub blocked: bool,
}

fn fixture() -> Option<Value> {
    let f = std::env::var("HEARTH_SHIELD_FIXTURE").ok()?;
    serde_json::from_slice(&std::fs::read(f).ok()?).ok()
}
pub fn is_fixture() -> bool {
    std::env::var("HEARTH_SHIELD_FIXTURE").is_ok()
}

/// curl with everything it's told on stdin (a config: url, headers, data), so nothing sensitive is on a command line.
fn curl(lines: &[String]) -> Result<Value, String> {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let mut c = Command::new("curl").args(["-s", "-S", "--max-time", "5", "-K", "-"]).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().map_err(|_| "curl isn't here".to_string())?;
    if let Some(mut i) = c.stdin.take() {
        let _ = i.write_all(lines.join("\n").as_bytes());
    }
    let o = c.wait_with_output().map_err(|e| e.to_string())?;
    if !o.status.success() {
        return Err(String::from_utf8_lossy(&o.stderr).trim().to_string());
    }
    let t = String::from_utf8_lossy(&o.stdout).to_string();
    if t.trim().is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_str(&t).map_err(|_| "an answer it couldn't read".to_string())
}
fn q(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}
fn get(url: &str, headers: &[String]) -> Result<Value, String> {
    let mut l = vec![format!("url = {}", q(url))];
    for h in headers {
        l.push(format!("header = {}", q(h)));
    }
    curl(&l)
}
fn send(method: &str, url: &str, headers: &[String], body: Option<&Value>) -> Result<Value, String> {
    let mut l = vec![format!("url = {}", q(url)), format!("request = {}", q(method)), format!("header = {}", q("Content-Type: application/json"))];
    for h in headers {
        l.push(format!("header = {}", q(h)));
    }
    if let Some(b) = body {
        l.push(format!("data = {}", q(&b.to_string())));
    }
    curl(&l)
}

fn ag(path: &str) -> Result<Value, String> {
    if let Some(f) = fixture() {
        return f.get(path).cloned().ok_or_else(|| format!("{path}: not in the fixture"));
    }
    get(&format!("{API}{path}"), &[])
}

/// Hearth's own choices (Server mode's ad blocker), readable without root.
fn status() -> Value {
    if let Some(f) = fixture() {
        return f.get("status").cloned().unwrap_or(Value::Null);
    }
    std::fs::read(format!("{RUN}/status.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_else(|| {
        let o = hearth_ui::run::blocking(&hearth_ui::run::args(&["python3", HELPER, "status"]));
        o.json().unwrap_or(Value::Null)
    })
}
fn clients_today() -> Value {
    if let Some(f) = fixture() {
        return f.get("clients-today").cloned().unwrap_or(Value::Null);
    }
    std::fs::read(format!("{RUN}/clients-today.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null)
}
fn days_file() -> Value {
    if let Some(f) = fixture() {
        return f.get("days").cloned().unwrap_or(Value::Null);
    }
    std::fs::read(format!("{RUN}/days.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null)
}

fn first_kv(v: &Value) -> Option<(String, u64)> {
    let o = v.as_object()?;
    let (k, n) = o.iter().next()?;
    Some((k.clone(), n.as_u64().unwrap_or(0)))
}

/// Since midnight, from the last 24 hours by the hour (the newest hour last).
pub fn since_midnight(v: &[u64], hour_now: usize) -> Vec<u64> {
    let n = (hour_now + 1).min(v.len());
    v[v.len() - n..].to_vec()
}

fn hour_now() -> usize {
    if let Ok(h) = std::env::var("HEARTH_SHIELD_HOUR") {
        return h.parse().unwrap_or(23);
    }
    chrono_hour()
}
fn chrono_hour() -> usize {
    hearth_ui::time::local(hearth_ui::time::now()).hour.clamp(0, 23) as usize
}

/// RFC 3339 ("2026-11-26T03:00:00Z", "…:00.123+01:00") as seconds since 1970.
pub fn rfc3339(t: &str) -> Option<i64> {
    let (date, rest) = t.split_once('T')?;
    let mut d = date.split('-').map(|x| x.parse::<i64>().ok());
    let (y, m, dd) = (d.next()??, d.next()??, d.next()??);
    let time: String = rest.chars().take(8).collect();
    let mut h = time.split(':').map(|x| x.parse::<i64>().ok());
    let (hh, mm, ss) = (h.next()??, h.next()??, h.next().flatten().unwrap_or(0));
    let tail = &rest[8.min(rest.len())..];
    let tz = tail.find(|c| c == '+' || c == '-').map(|i| &tail[i..]).unwrap_or("");
    let off = if tz.len() >= 6 {
        let sign = if tz.starts_with('-') { -1 } else { 1 };
        sign * (tz[1..3].parse::<i64>().unwrap_or(0) * 3600 + tz[4..6].parse::<i64>().unwrap_or(0) * 60)
    } else {
        0
    };
    // days from 1970-01-01 (Howard Hinnant's days_from_civil)
    let (y2, m2) = if m <= 2 { (y - 1, m + 9) } else { (y, m - 3) };
    let era = y2.div_euclid(400);
    let yoe = y2 - era * 400;
    let doy = (153 * m2 + 2) / 5 + dd - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    Some(days * 86400 + hh * 3600 + mm * 60 + ss - off)
}

/// A blocked address in words (the board's "TV maker's viewing reports"), when it's one of the well-known kinds.
pub fn words_for(domain: &str) -> String {
    let d = domain.to_lowercase();
    let kinds: [(&[&str], &str); 8] = [
        (&["acr", "lgtvsdp", "samsungacr", "logs.roku", "tvinteractive", "alexa.amazon", "smartclip", "tcl"], "TV maker's viewing reports"),
        (&["imasdk", "ads.youtube", "pubads", "innovid", "spotx", "freewheel", "doubleverify"], "Ad network (video ads)"),
        (&["crashlytics", "app-measurement", "firebaselogging", "appcenter", "bugsnag", "sentry", "mixpanel", "app-analytics"], "App crash and usage reports"),
        (&["doubleclick", "googlesyndication", "adservice", "adnxs", "criteo", "taboola", "outbrain", "amazon-adsystem"], "Ad network (web banners)"),
        (&["samsungads", "ads.roku", "lgad", "tvads", "home-screen", "ott"], "Smart TV home-screen ads"),
        (&["pixel", "facebook.com/tr", "tr.snapchat", "bat.bing", "scorecardresearch", "hotjar"], "Tracking pixels"),
        (&["epicgames.com/ads", "store-ads", "launcher-ads", "steamstatic.com/ads", "news.blizzard"], "Game launcher news ads"),
        (&["analytics", "telemetry", "metrics"], "Usage reports"),
    ];
    for (keys, w) in kinds {
        if keys.iter().any(|k| d.contains(k)) {
            return w.to_string();
        }
    }
    domain.to_string()
}

/// Everything for the window, from Hearth's own AdGuard Home on this PC.
pub fn read_here(names: &Value) -> Result<Data, String> {
    let s = ag("/control/stats")?;
    let st = status();
    let ads = st.get("ads").cloned().unwrap_or(Value::Null);
    let h = hour_now();
    let qs: Vec<u64> = s.get("dns_queries").and_then(Value::as_array).map(|a| a.iter().map(|x| x.as_u64().unwrap_or(0)).collect()).unwrap_or_default();
    let bs: Vec<u64> = s.get("blocked_filtering").and_then(Value::as_array).map(|a| a.iter().map(|x| x.as_u64().unwrap_or(0)).collect()).unwrap_or_default();
    let (qd, bd) = (since_midnight(&qs, h), since_midnight(&bs, h));
    let mut d = Data { asked: qd.iter().sum(), blocked: bd.iter().sum(), ..Default::default() };
    d.hours = qd.iter().zip(bd.iter().chain(std::iter::repeat(&0))).map(|(a, b)| (a.saturating_sub(*b), *b)).collect();
    // the lists
    let f = ag("/control/filtering/status").unwrap_or(Value::Null);
    let names_of = ads.get("listNames").cloned().unwrap_or(Value::Null);
    let filters = f.get("filters").and_then(Value::as_array).cloned().unwrap_or_default();
    let mut latest = String::new();
    if let Some(o) = names_of.as_object() {
        let mut v: Vec<(&String, &Value)> = o.iter().collect();
        v.sort_by_key(|(_, x)| x.get("id").and_then(Value::as_u64).unwrap_or(99));
        for (id, x) in v {
            let fid = x.get("id").and_then(Value::as_u64).unwrap_or(0);
            let fl = filters.iter().find(|f| f.get("id").and_then(Value::as_u64) == Some(fid));
            let count = fl.and_then(|f| f.get("rules_count")).and_then(Value::as_u64).unwrap_or(0);
            let on = x.get("on").and_then(Value::as_bool).unwrap_or(false);
            if on {
                d.addresses += count;
                if let Some(t) = fl.and_then(|f| f.get("last_updated")).and_then(Value::as_str) {
                    if t > latest.as_str() {
                        latest = t.to_string();
                    }
                }
            }
            d.lists.push(List { id: id.clone(), label: x.get("label").and_then(Value::as_str).unwrap_or(id).to_string(), on, count });
        }
    }
    d.lists_n = d.lists.len();
    d.lists_at = clock_of(&latest);
    // what was asked: the newest, and who asked for what was blocked the most
    let log = ag("/control/querylog?limit=500").unwrap_or(Value::Null);
    let mut per: std::collections::HashMap<String, (u64, u64)> = Default::default();
    for e in log.get("data").and_then(Value::as_array).cloned().unwrap_or_default() {
        let dom = e.pointer("/question/name").and_then(Value::as_str).unwrap_or("").trim_end_matches('.').to_string();
        let who = e.get("client").and_then(Value::as_str).unwrap_or("").to_string();
        let reason = e.get("reason").and_then(Value::as_str).unwrap_or("");
        let blocked = reason.starts_with("FilteredBlack") || reason.starts_with("FilteredBlockedService") || reason.starts_with("FilteredParental") || reason.starts_with("FilteredSafeBrowsing");
        let pc = per.entry(who.clone()).or_default();
        pc.0 += 1;
        if blocked {
            pc.1 += 1;
        }
        if d.log.len() < 200 {
            d.log.push(Q { at: clock_of(e.get("time").and_then(Value::as_str).unwrap_or("")), domain: dom, who, blocked });
        }
    }
    // the devices: every address that asked today, its own name (yours, else Tailscale's / your router's)
    let clients = ag("/control/clients").unwrap_or(Value::Null);
    let auto: Vec<(String, String)> = clients.get("auto_clients").and_then(Value::as_array).map(|a| a.iter().filter_map(|c| Some((c.get("ip")?.as_str()?.to_string(), c.get("name").and_then(Value::as_str).unwrap_or("").to_string()))).collect()).unwrap_or_default();
    let dev_states = ads.pointer("/view/devices").and_then(Value::as_array).cloned().unwrap_or_default();
    let today = clients_today();
    for c in s.get("top_clients").and_then(Value::as_array).cloned().unwrap_or_default() {
        let Some((ip, asked)) = first_kv(&c) else { continue };
        let mine = names.get(&ip);
        let name = mine.and_then(|m| m.get("name")).and_then(Value::as_str).map(str::to_string).or_else(|| auto.iter().find(|x| x.0 == ip).map(|x| x.1.split('.').next().unwrap_or("").to_string()).filter(|x| !x.is_empty())).unwrap_or_else(|| ip.clone());
        let about = mine.and_then(|m| m.get("about")).and_then(Value::as_str).unwrap_or("on Wi-Fi").to_string();
        // its blocked ones since midnight (hearth-server-dns counts them from the log as it grows); before that has
        // anything, the share blocked of what it asked lately (AdGuard counts blocked only in total)
        let counted = today.pointer(&format!("/clients/{}", ip.replace('~', "~0").replace('/', "~1"))).cloned();
        let (asked, blocked) = match counted {
            Some(c) if c.get("asked").and_then(Value::as_u64).unwrap_or(0) > 0 => (asked.max(c.get("asked").and_then(Value::as_u64).unwrap_or(0)), c.get("blocked").and_then(Value::as_u64).unwrap_or(0)),
            _ => {
                let (n, b) = per.get(&ip).copied().unwrap_or((0, 0));
                (asked, if n > 0 { asked * b / n } else { 0 })
            }
        };
        let state = dev_states.iter().find(|x| x.get("id").and_then(Value::as_str) == Some(ip.as_str()) || x.get("name").and_then(Value::as_str).map(|n| n.starts_with(&name)).unwrap_or(false)).and_then(|x| x.get("state")).and_then(Value::as_str).unwrap_or("Using it").to_string();
        d.devices.push(Dev { id: ip, name, about, asked, blocked, state });
    }
    d.devices_seen = d.devices.len();
    for x in s.get("top_blocked_domains").and_then(Value::as_array).cloned().unwrap_or_default() {
        let Some((dom, n)) = first_kv(&x) else { continue };
        let mut who: Vec<String> = Vec::new();
        for e in &d.log {
            if e.domain == dom && e.blocked {
                let nm = d.devices.iter().find(|v| v.id == e.who).map(|v| short_name(&v.name)).unwrap_or_else(|| e.who.clone());
                if !who.contains(&nm) {
                    who.push(nm);
                }
            }
        }
        d.top.push((words_for(&dom), who.join(", "), n));
    }
    d.allow = strs(ads.get("allow"));
    d.block = strs(ads.get("block"));
    d.safe = ads.get("safeSearch").and_then(Value::as_bool).unwrap_or(false);
    d.on = ads.get("on").and_then(Value::as_bool).unwrap_or(false);
    d.running = ads.get("running").and_then(Value::as_bool).unwrap_or(false);
    d.paused_until = ads.get("pausedUntil").and_then(Value::as_u64).unwrap_or(0);
    if let Some(o) = days_file().as_object() {
        let mut v: Vec<(String, u64, u64)> = o.iter().map(|(k, x)| (k.clone(), x.get("asked").and_then(Value::as_u64).unwrap_or(0), x.get("blocked").and_then(Value::as_u64).unwrap_or(0))).collect();
        v.sort();
        d.days = v;
    }
    Ok(d)
}

fn strs(v: Option<&Value>) -> Vec<String> {
    v.and_then(Value::as_array).map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default()
}
/// "TCL 55\" TV" → "TCL TV" (the board's short names under Blocked the most).
pub fn short_name(n: &str) -> String {
    let w: Vec<&str> = n.split_whitespace().filter(|w| !w.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false)).collect();
    if w.is_empty() { n.to_string() } else { w.join(" ").replace("TCL TV", "TCL TV") }
}

/// "2026-11-26T03:00:00Z" → "3:00 AM" (in this PC's time).
pub fn clock_of(t: &str) -> String {
    match rfc3339(t) {
        Some(s) => hearth_ui::time::clock(s),
        None => String::new(),
    }
}

// ------------------------------------------------------------------ changes (Hearth's own)
/// `pkexec hearth-server-dns ARGS` (the person at the PC, no password: 49-hearth-memory.rules).
pub fn helper(args: &[&str]) -> Result<(), String> {
    if is_fixture() {
        return Ok(());
    }
    let mut v = vec!["pkexec".to_string(), HELPER.to_string()];
    v.extend(args.iter().map(|s| s.to_string()));
    let o = hearth_ui::run::blocking(&v);
    if o.ok && !o.out.trim_start().starts_with("error:") { Ok(()) } else { Err(o.why()) }
}

// ------------------------------------------------------------------ a Pi-hole on the network
pub struct PiHole {
    pub url: String,
    sid: String,
}

impl PiHole {
    pub fn sign_in(url: &str, password: &str) -> Result<PiHole, String> {
        let base = url.trim_end_matches('/').to_string();
        let v = send("POST", &format!("{base}/api/auth"), &[], Some(&json!({"password": password})))?;
        let sid = v.pointer("/session/sid").and_then(Value::as_str).unwrap_or("").to_string();
        if sid.is_empty() {
            return Err("the Pi-hole didn't take the password".into());
        }
        Ok(PiHole { url: base, sid })
    }
    fn h(&self) -> Vec<String> {
        vec![format!("X-FTL-SID: {}", self.sid)]
    }
    fn get(&self, p: &str) -> Result<Value, String> {
        get(&format!("{}{p}", self.url), &self.h())
    }
    pub fn act(&self, method: &str, p: &str, body: Option<Value>) -> Result<(), String> {
        send(method, &format!("{}{p}", self.url), &self.h(), body.as_ref()).map(|_| ())
    }
    pub fn read(&self, names: &Value) -> Result<Data, String> {
        let s = self.get("/api/stats/summary")?;
        let mut d = Data {
            asked: s.pointer("/queries/total").and_then(Value::as_u64).unwrap_or(0),
            blocked: s.pointer("/queries/blocked").and_then(Value::as_u64).unwrap_or(0),
            addresses: s.pointer("/gravity/domains_being_blocked").and_then(Value::as_u64).unwrap_or(0),
            ..Default::default()
        };
        if let Some(t) = s.pointer("/gravity/last_update").and_then(Value::as_i64) {
            d.lists_at = hearth_ui::time::clock(t);
        }
        // the day in hours (Pi-hole keeps 10-minute steps)
        let hist = self.get("/api/history").unwrap_or(Value::Null);
        let mut hours = vec![(0u64, 0u64); 24];
        for x in hist.get("history").and_then(Value::as_array).cloned().unwrap_or_default() {
            let t = x.get("timestamp").and_then(Value::as_f64).unwrap_or(0.0) as i64;
            let hr = hearth_ui::time::local(t).hour.clamp(0, 23) as usize;
            let tot = x.get("total").and_then(Value::as_u64).unwrap_or(0);
            let bl = x.get("blocked").and_then(Value::as_u64).unwrap_or(0);
            hours[hr.min(23)].0 += tot.saturating_sub(bl);
            hours[hr.min(23)].1 += bl;
        }
        d.hours = hours.into_iter().take(hour_now() + 1).collect();
        let cl = self.get("/api/stats/top_clients?count=20").unwrap_or(Value::Null);
        let cb = self.get("/api/stats/top_clients?blocked=true&count=50").unwrap_or(Value::Null);
        for c in cl.get("clients").and_then(Value::as_array).cloned().unwrap_or_default() {
            let ip = c.get("ip").and_then(Value::as_str).unwrap_or("").to_string();
            let mine = names.get(&ip);
            let name = mine.and_then(|m| m.get("name")).and_then(Value::as_str).map(str::to_string).or_else(|| c.get("name").and_then(Value::as_str).filter(|x| !x.is_empty()).map(|x| x.split('.').next().unwrap_or(x).to_string())).unwrap_or_else(|| ip.clone());
            let blocked = cb.get("clients").and_then(Value::as_array).and_then(|a| a.iter().find(|x| x.get("ip").and_then(Value::as_str) == Some(ip.as_str()))).and_then(|x| x.get("count")).and_then(Value::as_u64).unwrap_or(0);
            d.devices.push(Dev { about: mine.and_then(|m| m.get("about")).and_then(Value::as_str).unwrap_or("on Wi-Fi").to_string(), id: ip, name, asked: c.get("count").and_then(Value::as_u64).unwrap_or(0), blocked, state: "Using it".into() });
        }
        d.devices_seen = s.pointer("/clients/active").and_then(Value::as_u64).map(|x| x as usize).unwrap_or(d.devices.len());
        let top = self.get("/api/stats/top_domains?blocked=true&count=7").unwrap_or(Value::Null);
        for x in top.get("domains").and_then(Value::as_array).cloned().unwrap_or_default() {
            let dom = x.get("domain").and_then(Value::as_str).unwrap_or("").to_string();
            d.top.push((words_for(&dom), String::new(), x.get("count").and_then(Value::as_u64).unwrap_or(0)));
        }
        let lists = self.get("/api/lists?type=block").unwrap_or(Value::Null);
        for l in lists.get("lists").and_then(Value::as_array).cloned().unwrap_or_default() {
            let addr = l.get("address").and_then(Value::as_str).unwrap_or("").to_string();
            let label = l.get("comment").and_then(Value::as_str).filter(|x| !x.is_empty()).map(str::to_string).unwrap_or_else(|| addr.rsplit('/').next().unwrap_or(&addr).to_string());
            d.lists.push(List { id: addr, label, on: l.get("enabled").and_then(Value::as_bool).unwrap_or(false), count: l.get("number").and_then(Value::as_u64).unwrap_or(0) });
        }
        d.lists_n = d.lists.len();
        for (kind, into) in [("allow", 0), ("deny", 1)] {
            let v = self.get(&format!("/api/domains/{kind}/exact")).unwrap_or(Value::Null);
            let ds: Vec<String> = v.get("domains").and_then(Value::as_array).map(|a| a.iter().filter_map(|x| x.get("domain").and_then(Value::as_str).map(str::to_string)).collect()).unwrap_or_default();
            if into == 0 {
                d.allow = ds;
            } else {
                d.block = ds;
            }
        }
        let qs = self.get("/api/queries?length=200").unwrap_or(Value::Null);
        for e in qs.get("queries").and_then(Value::as_array).cloned().unwrap_or_default() {
            let st = e.get("status").and_then(Value::as_str).unwrap_or("");
            d.log.push(Q {
                at: hearth_ui::time::clock(e.get("time").and_then(Value::as_f64).unwrap_or(0.0) as i64),
                domain: e.get("domain").and_then(Value::as_str).unwrap_or("").to_string(),
                who: e.pointer("/client/name").and_then(Value::as_str).filter(|x| !x.is_empty()).or(e.pointer("/client/ip").and_then(Value::as_str)).unwrap_or("").to_string(),
                blocked: matches!(st, "GRAVITY" | "DENYLIST" | "REGEX" | "EXTERNAL_BLOCKED_IP" | "EXTERNAL_BLOCKED_NULL" | "EXTERNAL_BLOCKED_NXRA" | "GRAVITY_CNAME" | "REGEX_CNAME" | "DENYLIST_CNAME"),
            });
        }
        let b = self.get("/api/dns/blocking").unwrap_or(Value::Null);
        d.on = b.get("blocking").and_then(Value::as_str) == Some("enabled");
        d.running = true;
        if let Some(t) = b.get("timer").and_then(Value::as_f64) {
            d.paused_until = (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|x| x.as_secs()).unwrap_or(0) as f64 + t) as u64;
        }
        Ok(d)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_day_is_since_midnight() {
        let v: Vec<u64> = (1..=24).collect();
        assert_eq!(since_midnight(&v, 1), vec![23, 24]);
        assert_eq!(since_midnight(&v, 23).len(), 24);
        assert_eq!(words_for("ads.doubleclick.net"), "Ad network (web banners)");
        assert_eq!(words_for("example.org"), "example.org");
        assert_eq!(q("a\"b"), "\"a\\\"b\"");
        assert_eq!(rfc3339("1970-01-02T00:00:00Z"), Some(86400));
        assert_eq!(rfc3339("2026-11-26T03:00:00.5+01:00"), Some(1795658400));
    }
}
