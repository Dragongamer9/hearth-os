//! Quick Access › Resources in Hearth's own Game Mode (ui/): Resource Monitor's numbers in short (graphics, processor,
//! memory and network with temperatures, the game, what else runs) and "Pause downloads and backups while I play",
//! through Mantel's helper desktop/shell/hearth-resmon (docs/RESOURCE-MONITOR.md). Read-only; the page asks only while it
//! shows. The pause is Hearth's existing pauses only (the home server's runtime flag; Light mode only when the person
//! pressed that option); Hearth's own downloads pause with downloads.pauseAll as before.

use serde_json::Value;
use std::process::{Command, Stdio};

/// The pause actions the helper takes; anything else is refused here before anything runs.
const PAUSE: &[&str] = &["status", "on", "off", "light-on"];

/// The pause action asked for ({"action": "on"}), if it's one of PAUSE.
pub fn pause_arg(a: &Value) -> Result<&'static str, String> {
    let want = a["action"].as_str().unwrap_or("status");
    PAUSE.iter().copied().find(|x| *x == want).ok_or_else(|| "Unknown pause action".to_string())
}

/// Runs hearth-resmon with these (fixed) arguments: its JSON answer, or its "error: ..." in plain words.
pub fn run(args: &[&str]) -> Result<Value, String> {
    let helper = crate::paths::desktop_dir().join("shell/hearth-resmon");
    if !helper.exists() { return Err("Resource Monitor needs Hearth, the OS.".into()); }
    let out = Command::new("python3").arg(&helper).args(args)
        .stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped())
        .output().map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&out.stdout);
    let v = crate::admin::parse_helper(&text)?;
    if !out.status.success() && v.is_null() {
        return Err(String::from_utf8_lossy(&out.stderr).lines().last().unwrap_or("It didn't work.").to_string());
    }
    Ok(v)
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    #[test]
    fn only_known_pause_actions() {
        assert_eq!(super::pause_arg(&json!({"action": "on"})).unwrap(), "on");
        assert_eq!(super::pause_arg(&json!({"action": "light-on"})).unwrap(), "light-on");
        assert_eq!(super::pause_arg(&json!({})).unwrap(), "status");
        assert!(super::pause_arg(&json!({"action": "on; reboot"})).is_err());
        assert!(super::pause_arg(&json!({"action": 3})).is_ok());   // not a string: the status
    }
}
