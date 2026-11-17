//! Run commands (KDE Connect's "Run command"): commands you set up on this PC (Settings › Phone › Run commands) that a
//! paired phone starts with one tap: lock the screen, start a game, put the PC to sleep... They're kept in
//! commands.json in Hearth Connect's folder (written safely: atomic, .bak kept, never replaced by defaults). A phone
//! sees their names, and runs them only while its "Run commands" switch is on. Only commands you added can run: the
//! phone sends a command's key, never a command of its own.

use crate::daemon::{Daemon, PairedDev};
use crate::packet::Packet;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub fn file(home: &Path) -> PathBuf { home.join("commands.json") }

/// [{key, name, command}], in the order they were added.
pub fn list(home: &Path) -> Vec<Value> {
    crate::appfile::read_json(&file(home)).and_then(|v| v.as_array().cloned()).unwrap_or_default()
        .into_iter().filter(|c| c["key"].is_string() && c["name"].is_string() && c["command"].is_string()).collect()
}

/// The list as KDE Connect's app wants it: a JSON *string* of {key: {name, command}}.
pub fn phone_body(home: &Path, allowed: bool) -> Value {
    let mut m = serde_json::Map::new();
    if allowed {
        for c in list(home) { m.insert(c["key"].as_str().unwrap_or("").to_string(), json!({"name": c["name"], "command": c["command"]})); }
    }
    json!({"commandList": Value::Object(m).to_string(), "canAddCommand": true})
}

/// Tells the phone what it can run (nothing while its "Run commands" switch is off).
pub fn send_list(d: &Daemon, id: &str) {
    let Some(pd) = d.paired_dev(id) else { return };
    let _ = d.send_body(id, "kdeconnect.runcommand", phone_body(&d.home, pd.commands));
}

fn send_all(d: &Daemon) { for id in d.linked_paired() { send_list(d, &id); } }

pub fn packet(d: &Arc<Daemon>, id: &str, from: &str, p: &Packet, pd: &PairedDev) {
    if p.bool("requestCommandList") { send_list(d, id); }
    if p.bool("setup") {
        d.emit("commands-setup", json!({"device": id, "from": from}));
        d.notify_desktop(&format!("{from} wants to add commands"), "Add them in Settings › Phone › Run commands.", "smartphone", false);
    }
    let Some(key) = p.str("key") else { return };
    if !pd.commands {
        d.emit("warning", json!({"device": from, "text": format!("{from} asked to run a command, but Run commands is off for it")}));
        return;
    }
    let Some(c) = list(&d.home).into_iter().find(|c| c["key"].as_str() == Some(key)) else { return };
    let name = c["name"].as_str().unwrap_or("").to_string();
    let ok = run(c["command"].as_str().unwrap_or(""));
    d.emit("command-ran", json!({"device": id, "from": from, "name": name, "ok": ok}));
}

/// Starts a command in the person's session, on its own (nothing waits for it).
pub fn run(command: &str) -> bool {
    if command.trim().is_empty() { return false; }
    let mut c = crate::desktop::cmd("sh");
    c.arg("-c").arg(command).stdout(std::process::Stdio::null());
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::process::CommandExt;
        unsafe { c.pre_exec(|| { libc::setsid(); Ok(()) }); }
    }
    match c.spawn() {
        Ok(mut ch) => { std::thread::spawn(move || { let _ = ch.wait(); }); true }
        Err(_) => false,
    }
}

fn change(d: &Daemon, f: impl FnOnce(&mut Vec<Value>)) -> Result<(), String> {
    let _ = std::fs::create_dir_all(&d.home);
    crate::appfile::update(&file(&d.home), |cur| {
        let mut l = cur.and_then(|v| v.as_array().cloned()).unwrap_or_default();
        f(&mut l);
        (Some(Value::Array(l)), ())
    }).map_err(|e| format!("Couldn't save it: {e}"))
}

pub fn add(d: &Daemon, name: &str, command: &str) -> Result<Value, String> {
    let (name, command) = (name.trim(), command.trim());
    if name.is_empty() || command.is_empty() { return Err("Give it a name and a command".into()); }
    let key = crate::random_hex(8);
    let entry = json!({"key": key, "name": name.chars().take(60).collect::<String>(), "command": command});
    change(d, |l| l.push(entry))?;
    send_all(d);
    d.changed();
    Ok(json!({"key": key}))
}

pub fn remove(d: &Daemon, key: &str) -> Result<Value, String> {
    change(d, |l| l.retain(|c| c["key"].as_str() != Some(key)))?;
    send_all(d);
    d.changed();
    Ok(json!({}))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn list_for_the_phone() {
        let dir = std::env::temp_dir().join(format!("connect-rc-{}", crate::random_hex(4)));
        std::fs::create_dir_all(&dir).unwrap();
        crate::appfile::write_json(&file(&dir), &json!([{"key": "k1", "name": "Lock", "command": "loginctl lock-session"}, {"bad": 1}])).unwrap();
        assert_eq!(list(&dir).len(), 1);
        let b = phone_body(&dir, true);
        let inner: Value = serde_json::from_str(b["commandList"].as_str().unwrap()).unwrap();
        assert_eq!(inner["k1"]["name"], "Lock");
        let off = phone_body(&dir, false);
        assert_eq!(off["commandList"], "{}");
        let _ = std::fs::remove_dir_all(dir);
    }
}
