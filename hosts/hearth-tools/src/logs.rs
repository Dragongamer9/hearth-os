//! Logs: the system's journal (journalctl), your session's journal and Hearth's own log files, easy to read and
//! to filter (which start, how serious, which app, how recent, words). Reading the whole system's journal can
//! need more rights than your account has; then it's asked for once through hearth-admin (journal-read, read
//! only). "Send to the developer" is Hearth's own bug report (hearth-core's logs.rs, the same as Settings ›
//! System › Send logs), with what you were looking at as the note.

use crate::util;
use serde_json::{json, Value};
use std::path::PathBuf;

/// One journal entry for the page: when (ms), how serious (0 = emergency ... 7 = debug), which app, the message.
pub fn entry(v: &Value) -> Option<Value> {
    let s = |k: &str| v.get(k).and_then(Value::as_str).unwrap_or("");
    let t: u64 = s("__REALTIME_TIMESTAMP").parse().ok()?;
    // A message that isn't valid UTF-8 comes as an array of bytes.
    let msg = match v.get("MESSAGE") {
        Some(Value::String(m)) => m.clone(),
        Some(Value::Array(a)) => String::from_utf8_lossy(&a.iter().filter_map(|b| b.as_u64().map(|b| b as u8)).collect::<Vec<u8>>()).into_owned(),
        _ => String::new(),
    };
    let app = [s("SYSLOG_IDENTIFIER"), s("_COMM"), s("_SYSTEMD_UNIT")].into_iter().find(|x| !x.is_empty()).unwrap_or("system").to_string();
    Some(json!({"t": t / 1000, "p": s("PRIORITY").parse::<u8>().unwrap_or(6), "app": app, "pid": s("_PID"), "unit": s("_SYSTEMD_UNIT"), "msg": msg}))
}

/// The page's filters, checked: which start (0 = this one, -1 the one before...), the most serious level to
/// show down to, one app (its name in the journal), the last N minutes.
pub struct Filter {
    pub user: bool,
    pub boot: i64,
    pub priority: u8,
    pub app: String,
    pub since_min: u64,
    pub limit: usize,
    pub search: String,
}

pub fn filter(args: &Value) -> Result<Filter, String> {
    let app = util::arg(args, "app").to_string();
    if !app.is_empty() && !(app.len() <= 128 && app.chars().all(|c| c.is_ascii_alphanumeric() || "._-@:+".contains(c))) { return Err("That app name can't be searched for.".into()); }
    let boot = util::arg_i64(args, "boot", 0);
    if !(-999..=0).contains(&boot) { return Err("Unknown start.".into()); }
    Ok(Filter { user: util::arg(args, "source") == "user", boot, priority: util::arg_u64(args, "priority", 7).min(7) as u8, app,
        since_min: util::arg_u64(args, "sinceMin", 0).min(60 * 24 * 365), limit: util::arg_u64(args, "limit", 1500).clamp(50, 20000) as usize,
        search: util::arg(args, "search").to_lowercase() })
}

/// journalctl's arguments for a filter (also what hearth-admin's journal-read accepts, rebuilt there).
pub fn journal_args(f: &Filter, lines: usize) -> Vec<String> {
    let mut a = vec!["--no-pager".to_string(), "-o".into(), "json".into(), "-n".into(), lines.to_string(), "-b".into(), f.boot.to_string(), "-p".into(), f.priority.to_string()];
    if f.user { a.push("--user".into()); }
    if !f.app.is_empty() { a.push("-t".into()); a.push(f.app.clone()); }
    if f.since_min > 0 { a.push(format!("--since=-{}min", f.since_min)); }
    a
}

fn matches(e: &Value, search: &str) -> bool {
    search.is_empty() || e["msg"].as_str().unwrap_or("").to_lowercase().contains(search) || e["app"].as_str().unwrap_or("").to_lowercase().contains(search)
}

fn parse_lines(text: &str, f: &Filter) -> Vec<Value> {
    let mut out: Vec<Value> = text.lines().filter_map(|l| serde_json::from_str::<Value>(l).ok()).filter_map(|v| entry(&v)).filter(|e| matches(e, &f.search)).collect();
    if out.len() > f.limit { out.drain(..out.len() - f.limit); }
    out
}

/// The journal for the page: {entries, limited} (limited: only what your account may read was shown).
pub fn query(args: &Value) -> Result<Value, String> {
    let f = filter(args)?;
    // Searching looks further back, then keeps the newest matches.
    let lines = if f.search.is_empty() { f.limit } else { (f.limit * 10).min(50000) };
    let a = journal_args(&f, lines);
    let all = util::arg(args, "admin") == "yes";
    let text = if all && !f.user {
        let mut ad: Vec<String> = vec!["journal-read".into(), "--boot".into(), f.boot.to_string(), "--priority".into(), f.priority.to_string(), "--lines".into(), lines.min(20000).to_string()];
        if !f.app.is_empty() { ad.push("--ident".into()); ad.push(f.app.clone()); }
        if f.since_min > 0 { ad.push("--since-min".into()); ad.push(f.since_min.to_string()); }
        util::admin_raw(&ad.iter().map(String::as_str).collect::<Vec<_>>(), 120)?
    } else {
        let o = util::run("journalctl", &a.iter().map(String::as_str).collect::<Vec<_>>(), 60)?;
        if !o.ok() && o.out.trim().is_empty() {
            let why = o.why();
            if why.contains("Specifying boot ID or boot offset has no effect") || why.contains("Data from the specified boot") { return Ok(json!({"entries": [], "limited": false, "note": "Nothing was saved from that start."})); }
            return Err(format!("Couldn't read the journal ({why})"));
        }
        // Without the adm/wheel/systemd-journal groups journalctl only shows your own logs and says so.
        let limited = !f.user && (o.err.contains("insufficient permissions") || o.err.contains("not seeing messages from other users"));
        let entries = parse_lines(&o.out, &f);
        return Ok(json!({"entries": entries, "limited": limited}));
    };
    Ok(json!({"entries": parse_lines(&text, &f), "limited": false}))
}

/// "  -1 0a1b2c... Mon 2026-10-20 10:00:00 CEST Mon 2026-10-20 12:00:00 CEST" (older journalctl without JSON).
pub fn parse_boots_text(text: &str) -> Vec<Value> {
    text.lines().filter_map(|l| {
        let mut it = l.split_whitespace();
        let idx: i64 = it.next()?.parse().ok()?;
        let id = it.next()?.to_string();
        if id.len() < 16 { return None; }
        let rest: Vec<&str> = it.collect();
        let half = rest.len() / 2;
        Some(json!({"index": idx, "id": id, "first": rest[..half].join(" "), "last": rest[half..].join(" ")}))
    }).collect()
}

/// The recorded starts, newest first: [{index, id, first, last}] (first/last as ms, or text from older journalctl).
pub fn boots() -> Result<Value, String> {
    let o = util::run("journalctl", &["--list-boots", "--no-pager", "-o", "json"], 30)?;
    let mut list: Vec<Value> = match serde_json::from_str::<Value>(o.out.trim()) {
        Ok(Value::Array(a)) => a.iter().map(|b| json!({"index": b["index"], "id": b["boot_id"],
            "first": b["first_entry"].as_u64().map(|u| u / 1000), "last": b["last_entry"].as_u64().map(|u| u / 1000)})).collect(),
        _ => {
            let t = util::run("journalctl", &["--list-boots", "--no-pager"], 30)?;
            parse_boots_text(&t.out)
        }
    };
    list.sort_by(|a, b| b["index"].as_i64().cmp(&a["index"].as_i64()));
    list.truncate(50);
    Ok(json!({"boots": list}))
}

/// The apps that wrote to the journal in a start (for the app filter).
pub fn apps(args: &Value) -> Result<Value, String> {
    let f = filter(args)?;
    let b = f.boot.to_string();
    let mut a = vec!["--no-pager", "-b", b.as_str(), "-F", "SYSLOG_IDENTIFIER"];
    if f.user { a.push("--user"); }
    let o = util::run("journalctl", &a, 60)?;
    let mut list: Vec<&str> = o.out.lines().map(str::trim).filter(|l| !l.is_empty() && l.len() <= 128).collect();
    list.sort_by_key(|s| s.to_lowercase());
    list.dedup();
    Ok(json!({"apps": list}))
}

/// Hearth's own log files: Game Mode's (bridge.log), the session's, Desktop Mode's shell and helpers.
fn hearth_files() -> Vec<(String, String, PathBuf)> {
    let data = util::data_dir();
    let run = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/tmp"));
    let mut out: Vec<(String, String, PathBuf)> = vec![
        ("bridge".into(), "Hearth (Game Mode)".into(), data.join("bridge.log")),
        ("session".into(), "Session (starting Game Mode and Desktop Mode)".into(), data.join("session.log")),
        ("shell".into(), "Desktop Mode's taskbar and menus".into(), run.join("nimbus-shell.log")),
        ("pad".into(), "Desktop Mode's controller helper".into(), data.join("pad.log")),
        ("labwc".into(), "Desktop Mode's window manager (labwc)".into(), data.join("labwc.log")),
        ("input".into(), "Hearth Input (controller layouts)".into(), data.join("input.log")),
        ("seat".into(), "Multi-seat host".into(), data.join("seat-host.log")),
    ];
    // Any other .log Hearth keeps in its folder.
    for e in std::fs::read_dir(&data).into_iter().flatten().flatten() {
        let n = e.file_name().to_string_lossy().into_owned();
        if n.ends_with(".log") && !out.iter().any(|x| x.2 == e.path()) {
            let id = format!("file:{}", n.trim_end_matches(".log"));
            out.push((id, n.clone(), e.path()));
        }
    }
    out.into_iter().filter(|x| x.2.is_file()).collect()
}

pub fn files() -> Result<Value, String> {
    Ok(json!({"files": hearth_files().into_iter().map(|(id, name, p)| {
        let m = std::fs::metadata(&p).ok();
        json!({"id": id, "name": name, "path": p.to_string_lossy(), "size": m.as_ref().map(|m| m.len()),
            "mtime": m.and_then(|m| m.modified().ok()).and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_millis() as u64)})
    }).collect::<Vec<_>>()}))
}

/// How serious a plain log line looks (Hearth's files have no levels).
pub fn guess_priority(l: &str) -> u8 {
    let low = l.to_lowercase();
    if low.contains("panic") || low.contains("fatal") || low.contains("crash") { 2 }
    else if low.contains("error") || low.contains("failed") || low.contains("fail ") || low.contains("exception") { 3 }
    else if low.contains("warn") { 4 }
    else { 6 }
}

/// The newest lines of one of Hearth's files, as entries (a leading HH:MM:SS is kept as the time text).
pub fn file(args: &Value) -> Result<Value, String> {
    let id = util::arg(args, "id");
    let (_, name, p) = hearth_files().into_iter().find(|x| x.0 == id).ok_or("That log isn't there any more.")?;
    let data = std::fs::read(&p).map_err(|e| format!("Couldn't read {name}: {e}"))?;
    // The last 4 MB at most.
    let start = data.len().saturating_sub(4 << 20);
    let text = String::from_utf8_lossy(&data[start..]).into_owned();
    let search = util::arg(args, "search").to_lowercase();
    let maxp = util::arg_u64(args, "priority", 7) as u8;
    let limit = util::arg_u64(args, "limit", 3000).clamp(50, 20000) as usize;
    let mut out: Vec<Value> = text.lines().skip(if start > 0 { 1 } else { 0 }).filter(|l| !l.trim().is_empty()).filter_map(|l| {
        let p = guess_priority(l);
        if p > maxp || (!search.is_empty() && !l.to_lowercase().contains(&search)) { return None; }
        let (time, msg) = match l.get(..8) { Some(t) if t.len() == 8 && t.as_bytes()[2] == b':' && t.as_bytes()[5] == b':' && t.chars().filter(|c| c.is_ascii_digit()).count() == 6 => (t.to_string(), l[8..].trim_start().to_string()), _ => (String::new(), l.to_string()) };
        Some(json!({"time": time, "p": p, "app": "", "msg": msg}))
    }).collect();
    if out.len() > limit { out.drain(..out.len() - limit); }
    Ok(json!({"entries": out, "path": p.to_string_lossy()}))
}

/// Saves what the page shows (its text, made there) as a file in your home folder: its path.
pub fn export(args: &Value) -> Result<Value, String> {
    let text = util::arg(args, "text");
    if text.is_empty() { return Err("Nothing to save.".into()); }
    let label: String = util::arg(args, "label").chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').take(30).collect();
    let name = format!("Hearth-logs-{}{}.txt", if label.is_empty() { String::new() } else { format!("{label}-") }, util::stamp().replace(' ', "-").replace('.', "-"));
    let p = util::unique_path(&util::home(), &name);
    std::fs::write(&p, text).map_err(|e| format!("Couldn't save it: {e}"))?;
    Ok(json!({"path": p.to_string_lossy()}))
}

/// Hearth's bug report, as Settings sends it, with the person's note and what they were looking at.
#[cfg(target_os = "linux")]
pub fn send(args: &Value) -> Result<Value, String> {
    let note = util::arg(args, "note").chars().take(900).collect::<String>();
    let looking = util::arg(args, "looking").chars().take(200).collect::<String>();
    let full = if looking.is_empty() { note } else if note.trim().is_empty() { format!("(from the Logs app; looking at {looking})") } else { format!("{note} (from the Logs app; looking at {looking})") };
    let kind = match util::edition().as_str() { "" => "other".to_string(), e => e.to_string() };
    hearth_core::logs::send_report(&util::version(), &kind, &full)
}
#[cfg(not(target_os = "linux"))]
pub fn send(_: &Value) -> Result<Value, String> { Err("Sending logs from here works on Hearth OS for now.".into()) }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entries() {
        let v: Value = serde_json::from_str(r#"{"__REALTIME_TIMESTAMP":"1761130000123456","PRIORITY":"3","SYSLOG_IDENTIFIER":"kernel","MESSAGE":"usb 1-2: device not accepting address","_PID":"1"}"#).unwrap();
        let e = entry(&v).unwrap();
        assert_eq!((e["t"].as_u64(), e["p"].as_u64(), e["app"].as_str()), (Some(1761130000123), Some(3), Some("kernel")));
        let b: Value = serde_json::from_str(r#"{"__REALTIME_TIMESTAMP":"1","MESSAGE":[104,105,255],"_COMM":"x"}"#).unwrap();
        assert!(entry(&b).unwrap()["msg"].as_str().unwrap().starts_with("hi"));
        assert!(entry(&json!({"MESSAGE": "no time"})).is_none());
    }

    #[test]
    fn filters_are_checked() {
        assert!(filter(&json!({"app": "gnome-shell"})).is_ok());
        assert!(filter(&json!({"app": "x; rm -rf"})).is_err());
        assert!(filter(&json!({"boot": 3})).is_err());
        let f = filter(&json!({"boot": -1, "priority": 3, "app": "hhd", "sinceMin": 60, "source": "user"})).unwrap();
        let a = journal_args(&f, 100);
        assert_eq!(a.join(" "), "--no-pager -o json -n 100 -b -1 -p 3 --user -t hhd --since=-60min");
    }

    #[test]
    fn boots_text() {
        let b = parse_boots_text(" -1 0123456789abcdef0123456789abcdef Mon 2026-10-20 10:00:00 CEST Mon 2026-10-20 12:00:00 CEST\n  0 fedcba9876543210fedcba9876543210 Tue 2026-10-21 09:00:00 CEST Tue 2026-10-21 18:00:00 CEST\n");
        assert_eq!(b.len(), 2);
        assert_eq!(b[0]["index"], -1);
        assert_eq!(b[1]["first"], "Tue 2026-10-21 09:00:00 CEST");
    }

    #[test]
    fn plain_lines() {
        assert_eq!(guess_priority("12:00:01 store.launch failed: no such file"), 3);
        assert_eq!(guess_priority("all good"), 6);
        assert_eq!(guess_priority("WARNING: low battery"), 4);
    }
}
