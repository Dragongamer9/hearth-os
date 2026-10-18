//! Hearth's own installer, the app's side (live USB only): talks to /usr/libexec/nimbus/hearth-install (the
//! engine, shared with Mantel's install window) through pkexec. Port of hosts/linux/src/os-installer.js.
//!   disks()      the drives Hearth can go on
//!   check(part)  next to Windows (no part) or the Linux on `part` (SteamOS...): room it can give, what's in the way
//!   start(plan)  installs; progress goes to the page as 'osInstall' events ({state, stage, pct, text...})
//!   restart()    restarts into the installed Hearth

use serde_json::{json, Value};
use std::io::{BufRead, Write};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

const ENGINE: &str = "/usr/libexec/nimbus/hearth-install";
const PREP: &str = "/usr/libexec/nimbus/hearth-live-prep";

/// One helper output line: a stage, the result or an error.
pub fn parse_line(line: &str) -> Option<(&'static str, Value)> {
    let line = line.trim();
    if let Some(j) = line.strip_prefix("stage: ") { return serde_json::from_str(j).ok().filter(Value::is_object).map(|v| ("stage", v)); }
    if let Some(j) = line.strip_prefix("result: ") { return serde_json::from_str(j).ok().filter(Value::is_object).map(|v| ("result", v)); }
    line.strip_prefix("error: ").map(|e| ("error", json!(e)))
}

/// A helper's JSON answer (the last line that is JSON), or its error line.
pub fn answer(code: i32, out: &str, cmd: &str) -> Result<Value, String> {
    let lines: Vec<&str> = out.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    if let Some(e) = lines.iter().find_map(|l| l.strip_prefix("error: ")) { return Err(e.to_string()); }
    let j = lines.iter().rev().find(|l| l.starts_with('{'));
    match (code, j) {
        (0, Some(j)) => serde_json::from_str(j).map_err(|e| e.to_string()),
        _ => Err(lines.last().map(|l| l.to_string()).unwrap_or_else(|| format!("{cmd} failed"))),
    }
}

pub struct OsInstaller { pub state: Arc<Mutex<Value>> }

impl Default for OsInstaller { fn default() -> Self { Self::new() } }

impl OsInstaller {
    pub fn new() -> OsInstaller { OsInstaller { state: Arc::new(Mutex::new(json!({"state": "idle", "stage": null, "pct": 0, "text": "", "error": null, "result": null}))) } }
    pub fn available() -> bool { std::path::Path::new("/etc/hearth-live").exists() && std::path::Path::new(ENGINE).exists() }
    pub fn status(&self) -> Value { let mut s = self.state.lock().unwrap().clone(); s["available"] = json!(Self::available()); s }

    fn helper(cmd: &str, args: &[&str]) -> Result<Value, String> {
        let o = Command::new("pkexec").arg(cmd).args(args).stdin(Stdio::null()).output().map_err(|e| e.to_string())?;
        let mut out = String::from_utf8_lossy(&o.stdout).to_string();
        out.push('\n'); out.push_str(&String::from_utf8_lossy(&o.stderr));
        answer(o.status.code().unwrap_or(-1), &out, cmd)
    }
    fn need() -> Result<(), String> { if Self::available() { Ok(()) } else { Err("The installer is on the Hearth USB stick.".into()) } }

    pub fn disks(&self) -> Result<Value, String> { Self::need()?; Self::helper(ENGINE, &["disks"]) }

    pub fn check(&self, part: Option<&str>) -> Result<Value, String> {
        Self::need()?;
        if let Some(p) = part {
            let ok = p.starts_with("/dev/") && p[5..].chars().all(|c| c.is_ascii_alphanumeric() || "_/-".contains(c)) && p.len() > 5;
            if !ok { return Err("Unknown partition".into()); }
            return Self::helper(ENGINE, &["check", p]);
        }
        let mut r = Self::helper(PREP, &["check"])?;
        let w = r["windows"].clone();
        r["target"] = if w.is_object() { let mut t = w.clone(); t["kind"] = json!("windows"); t["name"] = json!("Windows"); t["part"] = w["dev"].clone(); t } else { Value::Null };
        Ok(r)
    }

    pub fn start(&self, plan: &Value) -> Result<Value, String> {
        Self::need()?;
        {
            let mut s = self.state.lock().unwrap();
            if s["state"] == "running" { return Err("Hearth is already installing.".into()); }
            *s = json!({"state": "running", "stage": "start", "pct": 0, "text": "Starting", "error": null, "result": null, "plan": {"mode": plan["mode"], "disk": plan["disk"]}});
        }
        let mut child = Command::new("pkexec").args([ENGINE, "run"]).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()
            .map_err(|e| { self.state.lock().unwrap()["state"] = json!("failed"); e.to_string() })?;
        if let Some(mut i) = child.stdin.take() { let _ = i.write_all(plan.to_string().as_bytes()); }
        let state = self.state.clone();
        let push = |s: &Value| { let mut s = s.clone(); s["available"] = json!(true); crate::stores::notify(json!({"type": "event", "name": "osInstall", "data": s})); };
        let read = |r: Box<dyn std::io::Read + Send>, state: Arc<Mutex<Value>>| std::thread::spawn(move || {
            for line in std::io::BufReader::new(r).lines().map_while(Result::ok) {
                let Some((kind, v)) = parse_line(&line) else { continue };
                let mut s = state.lock().unwrap();
                match kind {
                    "stage" => { for k in ["stage", "pct", "text"] { s[k] = v[k].clone(); } s["left"] = v["left"].clone(); s["bytes"] = v["bytes"].clone(); push(&s); }
                    "result" => s["result"] = v,
                    _ => s["error"] = v,
                }
            }
        });
        let a = read(Box::new(child.stdout.take().unwrap()), state.clone());
        let b = read(Box::new(child.stderr.take().unwrap()), state.clone());
        std::thread::spawn(move || {
            let code = child.wait().ok().and_then(|s| s.code()).unwrap_or(-1);
            let _ = (a.join(), b.join());
            let mut s = state.lock().unwrap();
            if code == 0 && !s["result"].is_null() { s["state"] = json!("done"); s["pct"] = json!(100); }
            else {
                s["state"] = json!("failed");
                if s["error"].is_null() { s["error"] = json!(if code == 126 || code == 127 { "Permission to install was refused." } else { "The install stopped." }); }
            }
            crate::bridge::log(&format!("osinstall: finished {} {}", s["state"], s["error"]));
            push(&s);
        });
        Ok(self.status())
    }

    pub fn restart(&self) -> bool { Command::new("systemctl").arg("reboot").spawn().is_ok() }
}

/// "Install with Mantel": Mantel opens its install window when it starts. And Mantel's "use Hearth's
/// installer" asks the other way round (the live USB's welcome screen then opens the installer).
fn handoff_file() -> std::path::PathBuf { std::path::PathBuf::from(std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into())).join("hearth-open-installer") }
pub fn handoff(set: bool) -> bool {
    if set { return std::fs::write(handoff_file(), "mantel\n").is_ok(); }
    match std::fs::read_to_string(handoff_file()) { Ok(t) if t.trim() == "hearth" => std::fs::remove_file(handoff_file()).is_ok(), _ => false }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reads_the_engine() {
        assert_eq!(parse_line("stage: {\"stage\":\"copy\",\"pct\":40,\"text\":\"Copying\"}").unwrap().1["pct"], 40);
        assert_eq!(parse_line("error: Disk too small").unwrap(), ("error", json!("Disk too small")));
        assert!(parse_line("random noise").is_none());
        assert_eq!(answer(0, "hello\n{\"disks\":[]}\n", "x").unwrap(), json!({"disks": []}));
        assert_eq!(answer(0, "error: nope\n{\"a\":1}", "x").unwrap_err(), "nope");
        assert_eq!(answer(1, "something broke", "x").unwrap_err(), "something broke");
    }
}
