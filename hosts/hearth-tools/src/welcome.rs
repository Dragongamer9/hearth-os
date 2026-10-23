//! Welcome & Tips: the first-run tour (shown once per account: Desktop Mode's autostart runs
//! `hearth-tools --app welcome --first-run`, which does nothing once you've seen it) and the searchable help,
//! which is ui/help.json, made from the website's docs at build time (build.rs, tools/build-docs.py).

use crate::util;
use serde_json::{json, Value};
use std::path::PathBuf;

pub fn marker() -> PathBuf { util::data_dir().join("welcome-done") }

pub fn info() -> Result<Value, String> {
    Ok(json!({"edition": util::edition(), "version": util::version(), "seen": marker().exists(), "user": std::env::var("USER").unwrap_or_default()}))
}

pub fn done() -> Result<Value, String> {
    let m = marker();
    if let Some(d) = m.parent() { let _ = std::fs::create_dir_all(d); }
    std::fs::write(&m, util::version()).map_err(|e| e.to_string())?;
    Ok(json!({"ok": true}))
}

/// Opens what a tour card or help answer points to. Only these.
pub fn open(args: &Value) -> Result<Value, String> {
    let what = util::arg(args, "what");
    const SETTINGS: [&str; 18] = ["wifi", "bluetooth", "sound", "display", "appearance", "power", "controller", "keyboard", "printers", "datetime", "defaults", "clipboard", "users", "signin", "browser", "claude", "updates", "about"];
    const TOOLS: [&str; 7] = ["disk-usage", "system-info", "logs", "network", "passwords", "printers", "welcome"];
    if let Some(page) = what.strip_prefix("settings:") {
        if !page.is_empty() && !SETTINGS.contains(&page) { return Err("Unknown settings page.".into()); }
        if page.is_empty() { util::spawn("hearth-settings", &[])?; } else { util::spawn("hearth-settings", &[page])?; }
    } else if let Some(app) = what.strip_prefix("tool:") {
        if !TOOLS.contains(&app) { return Err("Unknown tool.".into()); }
        let me = std::env::current_exe().map_err(|e| e.to_string())?;
        util::spawn(&me.to_string_lossy(), &["--app", app])?;
    } else if what == "files" {
        util::spawn("hearth-filemanager", &[])?;
    } else if what == "disks" {
        util::spawn("hearth-disks", &[])?;
    } else if what == "terminal" {
        util::spawn("hearth-terminal", &[])?;
    } else if let Some(url) = what.strip_prefix("url:") {
        if !(url.starts_with("https://hearthos.fun/") || url.starts_with("https://github.com/Dragongamer9/hearth-os")) { return Err("Only Hearth's own pages open from here.".into()); }
        util::spawn("xdg-open", &[url])?;
    } else {
        return Err("Unknown place.".into());
    }
    Ok(json!({"ok": true}))
}
