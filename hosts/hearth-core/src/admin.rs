//! Settings › Sign-in and Settings › Updates. What needs root goes through hearth-admin (part of Hearth OS)
//! with pkexec; a polkit rule lets the person at the device run it without a password. Port of system-admin.js.

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

pub const HELPER: &str = "/usr/libexec/nimbus/hearth-admin";
const NO_AUTOLOGIN: &str = "/etc/nimbus/no-autologin";

pub fn session_conf() -> PathBuf { crate::paths::config_dir().join("session.conf") }
fn desktop_json() -> PathBuf { crate::paths::data_dir().join("desktop.json") }
fn read_json(p: &Path) -> Value { std::fs::read(p).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(json!({})) }

/// NIMBUS_START_MODE from session.conf (shell syntax): game or desktop.
pub fn read_start_mode(text: &str) -> &'static str {
    for l in text.lines() {
        let v = l.trim().strip_prefix("NIMBUS_START_MODE=").map(|v| v.trim_matches(|c| c == '"' || c == '\''));
        if v == Some("desktop") { return "desktop"; }
        if v == Some("game") { return "game"; }
    }
    "game"
}
/// Sets NIMBUS_START_MODE, keeping everything else in the file.
pub fn write_start_mode(text: &str, mode: &str) -> String {
    let line = format!("NIMBUS_START_MODE={mode}");
    if text.lines().any(|l| l.trim_start().starts_with("NIMBUS_START_MODE=")) {
        let mut out: Vec<String> = text.lines().map(|l| if l.trim_start().starts_with("NIMBUS_START_MODE=") { line.clone() } else { l.to_string() }).collect();
        out.push(String::new());
        out.join("\n")
    } else {
        format!("{}{}{line}\n", text, if !text.is_empty() && !text.ends_with('\n') { "\n" } else { "" })
    }
}

/// hearth-admin's output: the last JSON line is the answer; "error: ..." is a failure.
pub fn parse_helper(out: &str) -> Result<Value, String> {
    let lines: Vec<&str> = out.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    if let Some(e) = lines.iter().find_map(|l| l.strip_prefix("error: ")) { return Err(e.to_string()); }
    Ok(lines.iter().rev().find_map(|l| if l.starts_with('{') { serde_json::from_str(l).ok() } else { None }).unwrap_or(Value::Null))
}

pub fn can_admin() -> bool { Path::new(HELPER).exists() }

/// Runs hearth-admin; each output line also goes to `on_line` (progress, stages).
pub fn run(args: &[&str], input: Option<&str>, on_line: &mut dyn FnMut(&str)) -> Result<Value, String> {
    if !can_admin() { return Err("This needs Hearth, the OS.".into()); }
    let mut child = Command::new("pkexec").arg(HELPER).args(args).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())
        .spawn().map_err(|e| e.to_string())?;
    if let Some(mut stdin) = child.stdin.take() { if let Some(i) = input { let _ = stdin.write_all(i.as_bytes()); } }
    let mut out = String::new();
    if let Some(stdout) = child.stdout.take() {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) { on_line(line.trim()); out.push_str(&line); out.push('\n'); }
    }
    let o = child.wait_with_output().map_err(|e| e.to_string())?;
    let parsed = parse_helper(&out);
    if parsed.is_err() { return parsed; }
    match o.status.code() {
        Some(0) => parsed,
        Some(126) | Some(127) => Err("Not allowed (the polkit rule for Hearth is missing).".into()),
        _ => Err(String::from_utf8_lossy(&o.stderr).lines().last().unwrap_or("hearth-admin failed").to_string()),
    }
}

pub fn login() -> Value {
    json!({
        "canAdmin": can_admin(),
        "autologin": if can_admin() { json!(!Path::new(NO_AUTOLOGIN).exists()) } else { Value::Null },
        "startMode": read_start_mode(&std::fs::read_to_string(session_conf()).unwrap_or_default()),
        "hostname": std::fs::read_to_string("/proc/sys/kernel/hostname").map(|s| s.trim().to_string()).unwrap_or_default(),
        "user": std::env::var("USER").unwrap_or_default(),
        "lockOnWake": read_json(&desktop_json())["lockOnWake"] == true,
    })
}
pub fn set_start_mode(mode: &str) -> Result<Value, String> {
    if mode != "game" && mode != "desktop" { return Err("Start mode is game or desktop".into()); }
    let f = session_conf();
    let text = std::fs::read_to_string(&f).unwrap_or_default();
    crate::paths::write_atomic(&f, write_start_mode(&text, mode).as_bytes()).map_err(|e| e.to_string())?;
    Ok(login())
}
pub fn set_lock_on_wake(on: bool) -> Result<Value, String> {
    let f = desktop_json();
    let mut o = read_json(&f);
    o["lockOnWake"] = json!(on);
    crate::paths::write_atomic(&f, format!("{}\n", serde_json::to_string_pretty(&o).unwrap()).as_bytes()).map_err(|e| e.to_string())?;
    Ok(login())
}
/// Checks this user's password the way the login screen does (pam_unix's helper). No password: empty passes.
pub fn check_password(pw: &str) -> bool {
    let Some(helper) = ["/usr/sbin/unix_chkpwd", "/usr/bin/unix_chkpwd", "/sbin/unix_chkpwd"].into_iter().find(|f| Path::new(f).exists()) else { return false };
    let user = std::env::var("USER").unwrap_or_default();
    let Ok(mut child) = Command::new(helper).args([user.as_str(), "nullok"]).stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::null()).spawn() else { return false };
    if let Some(mut stdin) = child.stdin.take() { let _ = stdin.write_all(format!("{pw}\0").as_bytes()); }
    child.wait().map(|s| s.success()).unwrap_or(false)
}

/// System updates (Settings › Updates): the state the page shows, changed by the update's progress lines.
pub struct Updates { pub state: Arc<Mutex<Value>> }
impl Updates {
    pub fn new() -> Updates { Updates { state: Arc::new(Mutex::new(json!({"state": "idle", "line": "", "progress": null, "status": null, "error": null, "stage": null, "phases": null, "speeds": []}))) } }
    pub fn status(&self) -> Value { let mut s = self.state.lock().unwrap().clone(); s["canAdmin"] = json!(can_admin()); s }
    fn busy(&self) -> bool { let s = self.state.lock().unwrap(); s["state"] == "checking" || s["state"] == "updating" }
    pub fn refresh(&self) -> Value {
        if can_admin() && !self.busy() {
            let r = run(&["update-status"], None, &mut |_| {});
            let mut s = self.state.lock().unwrap();
            match r { Ok(v) => { s["status"] = v; s["error"] = Value::Null; } Err(e) => s["error"] = json!(e) }
        }
        self.status()
    }
    /// Checks (or installs) in the background; `emit` sends each change to the page.
    pub fn start(&self, install: bool, emit: crate::system::bt_scan::Emit) -> Value {
        if self.busy() { return self.status(); }
        {
            let mut s = self.state.lock().unwrap();
            s["state"] = json!(if install { "updating" } else { "checking" });
            s["error"] = Value::Null; s["line"] = json!(if install { "Starting…" } else { "" });
            if install { s["progress"] = json!(0); s["phases"] = json!({"download": {"pct": 0}, "install": {"pct": 0}, "apps": {"pct": 0}}); }
        }
        let state = self.state.clone();
        std::thread::spawn(move || {
            let push = |state: &Arc<Mutex<Value>>, emit: &crate::system::bt_scan::Emit| {
                let mut s = state.lock().unwrap().clone(); s["canAdmin"] = json!(can_admin());
                emit(json!({"type": "event", "name": "systemUpdate", "data": s}));
            };
            let mut last = std::time::Instant::now() - std::time::Duration::from_secs(1);
            let r = run(&[if install { "update" } else { "update-check" }], None, &mut |line: &str| {
                let mut s = state.lock().unwrap();
                if let Some(l) = line.strip_prefix("log: ") { s["line"] = json!(l); }
                else if let Some(p) = line.strip_prefix("progress: ") {
                    let (pct, what) = p.split_once(' ').unwrap_or((p, ""));
                    s["progress"] = json!(pct.parse::<f64>().unwrap_or(0.0) / 100.0);
                    if !what.is_empty() { s["line"] = json!(what); }
                } else if let Some(st) = line.strip_prefix("stage: ").and_then(|j| serde_json::from_str::<Value>(j).ok()) {
                    if let Some(ph) = st["phase"].as_str() { if s["phases"].is_object() { s["phases"][ph] = st.clone(); } }
                    s["stage"] = st;
                } else { return; }
                drop(s);
                if last.elapsed().as_millis() >= 500 { last = std::time::Instant::now(); push(&state, &emit); }
            });
            {
                let mut s = state.lock().unwrap();
                s["state"] = json!("idle"); s["progress"] = Value::Null;
                match r { Ok(v) => { s["status"] = v; s["line"] = json!(""); } Err(e) => s["error"] = json!(e) }
            }
            push(&state, &emit);
        });
        self.status()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn start_mode_file() {
        assert_eq!(read_start_mode("FOO=1\nNIMBUS_START_MODE=\"desktop\"\n"), "desktop");
        assert_eq!(read_start_mode(""), "game");
        assert_eq!(write_start_mode("FOO=1\nNIMBUS_START_MODE=game\n", "desktop"), "FOO=1\nNIMBUS_START_MODE=desktop\n");
        assert_eq!(write_start_mode("FOO=1", "game"), "FOO=1\nNIMBUS_START_MODE=game\n");
        assert_eq!(write_start_mode("", "game"), "NIMBUS_START_MODE=game\n");
    }
    #[test]
    fn helper_output() {
        assert_eq!(parse_helper("log: x\n{\"a\":1}\n").unwrap(), json!({"a": 1}));
        assert_eq!(parse_helper("progress: 5\nerror: no network\n").unwrap_err(), "no network");
    }
}
