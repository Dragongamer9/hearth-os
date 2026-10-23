//! Settings › Accessibility in Game Mode: the zoom and colour filters (gamescope's), the screen reader (Orca), shared
//! with Desktop Mode's page. All of it is desktop/shell/hearth-access (Python), which keeps the settings in
//! ~/.config/nimbus/accessibility.json and applies them to the mode it runs in.

use serde_json::Value;
use std::process::{Command, Stdio};

/// Runs hearth-access with these arguments: its JSON answer, or its "error: ..." in plain words.
pub fn run(args: &[&str]) -> Result<Value, String> {
    let helper = crate::paths::desktop_dir().join("shell/hearth-access");
    if !helper.exists() { return Err("Accessibility settings need Hearth, the OS.".into()); }
    let out = Command::new("python3").arg(&helper).args(args)
        // Inside gamescope (Game Mode) unless labwc says otherwise (Hearth as a window in Desktop Mode).
        .env("HEARTH_GAME_MODE", "1")
        .stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped())
        .output().map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&out.stdout);
    let v = crate::admin::parse_helper(&text)?;
    if !out.status.success() && v.is_null() {
        return Err(String::from_utf8_lossy(&out.stderr).lines().last().unwrap_or("It didn't work.").to_string());
    }
    Ok(v)
}

/// Only the settings Hearth knows go to the helper (it checks them again).
pub fn set_args(a: &Value) -> Result<String, String> {
    let o = a.get("set").and_then(Value::as_object).ok_or("Nothing to change")?;
    const KEYS: &[&str] = &["magnifierSize", "magnifierZoom", "magnifierStep", "gameZoom", "filter", "screenReader", "textScale", "highContrast", "largePointer"];
    let kept: serde_json::Map<String, Value> = o.iter().filter(|(k, _)| KEYS.contains(&k.as_str())).map(|(k, v)| (k.clone(), v.clone())).collect();
    if kept.is_empty() { return Err("Nothing to change".into()); }
    Ok(Value::Object(kept).to_string())
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    #[test]
    fn only_known_settings_are_passed_on() {
        assert_eq!(super::set_args(&json!({"set": {"filter": "grayscale", "evil": 1}})).unwrap(), r#"{"filter":"grayscale"}"#);
        assert!(super::set_args(&json!({"set": {"evil": 1}})).is_err());
        assert!(super::set_args(&json!({})).is_err());
    }
}
