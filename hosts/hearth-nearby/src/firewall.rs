//! The firewall: on Hearth OS, Nearby Share asks /usr/libexec/nimbus/hearth-link-firewall (through pkexec; a polkit
//! rule lets the person at the device do it without a password) to open its port while it listens, only to local
//! addresses, and to close it again when it stops. firewalld's rules are runtime-only, so a crash can't leave the
//! port open past the next start. Elsewhere (no helper): nothing is changed, and the window says so.

use serde_json::{json, Value};

pub const HELPER: &str = "/usr/libexec/nimbus/hearth-link-firewall";

/// Runs the helper: `action` open/close, `app` nearby/connect. The helper's JSON answer, or what went wrong.
pub fn helper(action: &str, app: &str, port: Option<u16>) -> Value {
    if !std::path::Path::new(HELPER).exists() {
        return json!({"ok": true, "firewall": "unmanaged", "note": "This system doesn't have Hearth's firewall helper, so the firewall wasn't changed."});
    }
    let mut args = vec![HELPER.to_string(), action.to_string(), app.to_string()];
    if let Some(p) = port { args.push(p.to_string()); }
    // pkexec needs no password for this helper (49-hearth-link.rules); without a polkit agent it fails fast.
    match std::process::Command::new("pkexec").args(&args).stdin(std::process::Stdio::null()).output() {
        Ok(o) => {
            let out = String::from_utf8_lossy(&o.stdout);
            out.lines().rev().find_map(|l| serde_json::from_str::<Value>(l).ok())
                .unwrap_or_else(|| json!({"ok": false, "error": format!("The firewall helper didn't answer ({})", String::from_utf8_lossy(&o.stderr).trim())}))
        }
        Err(e) => json!({"ok": false, "error": format!("Couldn't run pkexec: {e}")}),
    }
}

/// What the window says about the firewall.
pub fn describe(v: &Value) -> String {
    match (v["ok"].as_bool().unwrap_or(false), v["firewall"].as_str().unwrap_or("")) {
        (true, "firewalld") | (true, "ufw") => "The firewall lets local devices in while Nearby Share is open".into(),
        (true, "none") => "No firewall is running on this PC".into(),
        (true, _) => v["note"].as_str().unwrap_or("The firewall wasn't changed").into(),
        (false, _) => format!("The firewall couldn't be opened: {}. Phones may not see this PC.", v["error"].as_str().unwrap_or("unknown error")),
    }
}
