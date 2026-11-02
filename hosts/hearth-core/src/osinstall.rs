//! Hearth's own installer, the app's side (live USB only): talks to /usr/libexec/nimbus/hearth-install (the
//! engine, shared with Mantel's install window) through pkexec. Port of hosts/linux/src/os-installer.js.
//!   disks()      the drives Hearth can go on
//!   check(part, disk)  next to Windows (no part; the one on `disk`, the chosen drive) or the Linux on `part`
//!                (SteamOS...): room it can give, what's in the way
//!   start(plan)  installs; progress goes to the page as 'osInstall' events ({state, stage, pct, text...})
//!   restart()    restarts into the installed Hearth
//! The 1.1 USB installer (ui/installer.html) also uses:
//!   screen(name) the installer's screen, on the serial console as "HEARTH-UI screen=<name>" (only under CI's
//!                `hearth.ci`, which drives the screens in a virtual PC and screenshots each one)
//!   tz_guess()   the time zone for where this PC is (online only; ipapi.co), else the USB's own
//!   rotate(turn) the screen turned 0-3 quarter turns clockwise (handhelds whose panel is portrait): wlr-randr in cage
//!   poweroff()   turns the PC off

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

    pub fn check(&self, part: Option<&str>, disk: Option<&str>) -> Result<Value, String> {
        Self::need()?;
        let dev_ok = |p: &str| p.starts_with("/dev/") && p[5..].chars().all(|c| c.is_ascii_alphanumeric() || "_/-".contains(c)) && p.len() > 5;
        if let Some(p) = part {
            if !dev_ok(p) { return Err("Unknown partition".into()); }
            return Self::helper(ENGINE, &["check", p]);
        }
        // The Windows on the chosen drive (the one Hearth goes next to), not the biggest one on any drive.
        let mut r = match disk {
            Some(d) if !dev_ok(d) => return Err("Unknown drive".into()),
            Some(d) => Self::helper(PREP, &["check", d])?,
            None => Self::helper(PREP, &["check"])?,
        };
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
        if let Some(mut i) = child.stdin.take() { let _ = i.write_all(plan_for_engine(plan, under_ci()).to_string().as_bytes()); }
        let state = self.state.clone();
        let push = |s: &Value| { let mut s = s.clone(); s["available"] = json!(true); crate::stores::notify(json!({"type": "event", "name": "osInstall", "data": s})); };
        let read = |r: Box<dyn std::io::Read + Send>, state: Arc<Mutex<Value>>| std::thread::spawn(move || {
            for line in std::io::BufReader::new(r).lines().map_while(Result::ok) {
                // Everything the installer says goes in Hearth's log (in bug reports), except the progress ticks.
                if !line.starts_with("stage: ") && !line.trim().is_empty() { crate::bridge::log(&format!("osinstall: {}", line.chars().take(600).collect::<String>())); }
                let Some((kind, v)) = parse_line(&line) else { continue };
                let mut s = state.lock().unwrap();
                match kind {
                    "stage" => { for k in ["stage", "pct", "text"] { s[k] = v[k].clone(); } for k in ["left", "bytes", "got", "speed", "percent"] { s[k] = v[k].clone(); } push(&s); }
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

/// CI's end-to-end test (`hearth.ci` on the kernel command line) reads the serial console.
pub fn under_ci() -> bool { std::fs::read_to_string("/proc/cmdline").map(|c| c.split_whitespace().any(|w| w == "hearth.ci")).unwrap_or(false) }

/// The plan as the engine gets it: under CI the installed system talks on the serial console too.
pub fn plan_for_engine(plan: &Value, ci: bool) -> Value {
    let mut p = plan.clone();
    if ci && p.is_object() {
        p["kargs"] = json!(["console=ttyS0,115200", "console=tty0", "plymouth.ignore-serial-consoles"]);
    }
    p
}

/// The screen the installer shows, for the CI test that drives it (nothing happens on a real PC).
pub fn screen(name: &str) {
    if !under_ci() { return; }
    let name: String = name.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').take(40).collect();
    if let Ok(mut f) = std::fs::OpenOptions::new().write(true).open("/dev/ttyS0") { let _ = writeln!(f, "HEARTH-UI screen={name}"); }
    crate::bridge::log(&format!("osinstall: screen {name}"));
}

/// A time zone name as the engine accepts it ("America/Chicago").
pub fn valid_tz(z: &str) -> bool {
    !z.is_empty() && z.len() < 64 && !z.contains("..") && z.chars().all(|c| c.is_ascii_alphanumeric() || "_+/-".contains(c))
        && std::path::Path::new("/usr/share/zoneinfo").join(z).is_file()
}

/// Where this PC is, by its internet address (one request, 6 s at most): {"zone": "...", "how": "online"}, or the
/// USB's own zone with "how": "default" when offline.
pub fn tz_guess() -> Value {
    if let Ok(r) = crate::http::request("GET", "https://ipapi.co/timezone/", &[], None, 6) {
        let z = r.text.trim();
        if r.ok() && valid_tz(z) { return json!({"zone": z, "how": "online"}); }
    }
    json!({"zone": "UTC", "how": "default"})
}

/// wlr-randr's transform for a number of quarter turns clockwise.
pub fn transform(turn: u32) -> &'static str { match turn % 4 { 1 => "90", 2 => "180", 3 => "270", _ => "normal" } }

/// Turns every screen cage shows (the installer is the only thing on it).
pub fn rotate(turn: u32) -> Result<(), String> {
    let list = crate::system::run_timeout("wlr-randr", &[], std::time::Duration::from_secs(5))?;
    let outputs: Vec<String> = list.lines().filter(|l| !l.starts_with(' ') && !l.trim().is_empty())
        .filter_map(|l| l.split_whitespace().next().map(str::to_string)).collect();
    if outputs.is_empty() { return Err("No screen found".into()); }
    for o in outputs {
        crate::system::run_timeout("wlr-randr", &["--output", &o, "--transform", transform(turn)], std::time::Duration::from_secs(5))?;
    }
    Ok(())
}

pub fn poweroff() -> bool { Command::new("systemctl").arg("poweroff").spawn().is_ok() }

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
        assert_eq!(transform(1), "90");
        assert_eq!(transform(4), "normal");
        assert_eq!(plan_for_engine(&json!({"mode": "whole"}), false), json!({"mode": "whole"}));
        assert_eq!(plan_for_engine(&json!({"mode": "whole"}), true)["kargs"][0], "console=ttyS0,115200");
        assert!(!valid_tz("../etc/passwd"));
        assert!(!valid_tz(""));
        assert_eq!(answer(0, "hello\n{\"disks\":[]}\n", "x").unwrap(), json!({"disks": []}));
        assert_eq!(answer(0, "error: nope\n{\"a\":1}", "x").unwrap_err(), "nope");
        assert_eq!(answer(1, "something broke", "x").unwrap_err(), "something broke");
    }
}
