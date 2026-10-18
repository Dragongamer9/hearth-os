//! The UI's messages, answered: {type:'call', id, method, args} → {type:'result', id, ok, value|error}.
//! Methods come over from the Electron host (hosts/linux/src/bridge.js) a group at a time; anything not here
//! yet answers with a clear error, so the UI says so instead of waiting.

use crate::library::Library;
use crate::paths;
use crate::store::Store;
use crate::system;
use serde_json::{json, Value};

const POWER_ACTIONS: [&str; 4] = ["sleep", "menu", "shutdown", "nothing"];

pub struct Bridge {
    pub library: Library,
    pub version: String,
    /// Sends an event to the UI (set by the app; does nothing in tests).
    pub emit: system::bt_scan::Emit,
    pub updates: crate::admin::Updates,
}

impl Bridge {
    pub fn new(version: &str) -> Bridge {
        let store = Store::open(paths::data_dir().join("library.json"));
        Bridge { library: Library::new(store, paths::home()), version: version.to_string(), emit: std::sync::Arc::new(|_| {}), updates: crate::admin::Updates::new() }
    }

    /// Messages that aren't calls: the power menu, log lines. Returns true when Hearth should close
    /// (switching to the desktop or signing out, in the Hearth session).
    pub fn other(&mut self, msg: &Value) -> bool {
        match msg["type"].as_str().unwrap_or("") {
            "power" => {
                let in_session = std::env::var("NIMBUS_SESSION").map(|v| v == "1").unwrap_or(false);
                match crate::power::action(msg["action"].as_str().unwrap_or(""), in_session) {
                    Ok(quit) => quit,
                    Err(e) => { (self.emit)(json!({"type": "toast", "text": e})); false }
                }
            }
            "log" => { log(msg["text"].as_str().unwrap_or("")); false }
            _ => false,
        }
    }

    /// One message from the UI. Returns the reply to send back (None for messages that don't get one).
    pub fn handle(&mut self, msg: &Value) -> Option<Value> {
        if msg["type"] != "call" { return None; }
        let id = msg["id"].clone();
        let method = msg["method"].as_str().unwrap_or("");
        let args = if msg["args"].is_object() { msg["args"].clone() } else { json!({}) };
        Some(match self.dispatch(method, &args) {
            Ok(value) => json!({"type": "result", "id": id, "ok": true, "value": value}),
            Err(error) => json!({"type": "result", "id": id, "ok": false, "error": error}),
        })
    }

    pub fn dispatch(&mut self, method: &str, a: &Value) -> Result<Value, String> {
        let req = |k: &str| a[k].as_str().filter(|s| !s.is_empty()).map(str::to_string).ok_or(format!("Missing '{k}'"));
        let io = |r: std::io::Result<()>| r.map(|_| Value::Null).map_err(|e| e.to_string());
        match method {
            "host.info" => Ok(json!({
                "platform": "linux", "name": hostname(), "host": "Hearth (Rust)", "version": self.version,
                "session": std::env::var("NIMBUS_SESSION").map(|v| v == "1").unwrap_or(false),
                "user": std::env::var("USER").unwrap_or_default(),
                "live": std::path::Path::new("/etc/hearth-live").exists(),
                "installer": std::path::Path::new("/etc/hearth-installer").exists(),
                "features": [], "setupDone": true,
            })),
            "library.get" => Ok(self.library.get(a["rescan"].as_bool().unwrap_or(false))),
            "library.setFav" => io(self.library.store.set_in("favorites", &req("id")?, a["on"].as_bool().unwrap_or(false))),
            "library.setHidden" => io(self.library.store.set_in("hidden", &req("id")?, a["on"].as_bool().unwrap_or(false))),
            "game.launch" => {
                let id = req("id")?;
                let g = self.library.games.get(&id).ok_or("That game isn't in the library anymore. Try rescanning.")?;
                if !g.installed { return Err(format!("{} isn't installed yet.", g.title)); }
                let uri = g.launch_uri.clone().ok_or("The Rust version can't start this kind of game yet.")?;
                std::process::Command::new("steam").arg(&uri).spawn().map_err(|e| format!("Couldn't start Steam: {e}"))?;
                Ok(json!({"tracked": false}))
            }
            // The top bar and Quick Access
            "sys.status" => Ok(system::status(self.max_volume())),
            "radios.set" => {
                let on = a["on"].as_bool().unwrap_or(false);
                if a["kind"] == "wifi" { system::wifi::set_radio(on)?; } else { system::bt::set_power(on)?; }
                Ok(system::status(self.max_volume()))
            }
            "airplane.set" => { system::set_airplane(a["on"].as_bool().unwrap_or(false))?; Ok(system::status(self.max_volume())) }
            "wifi.scan" => system::wifi::scan(),
            "wifi.connect" => Ok(system::wifi::connect(&req("ssid")?, a["password"].as_str().filter(|p| !p.is_empty()))),
            "wifi.disconnect" => system::wifi::disconnect().map(|_| Value::Null),
            "wifi.forget" => system::wifi::forget(&req("ssid")?).map(|_| Value::Null),
            "audio.setVolume" => system::audio::set_volume(a["v"].as_f64().unwrap_or(0.0).round().max(0.0) as u32, self.max_volume()).map(|_| Value::Null),
            "audio.setMute" => system::audio::set_mute(a["m"].as_bool().unwrap_or(false)).map(|_| Value::Null),
            "display.setBrightness" => system::set_brightness(a["v"].as_f64().unwrap_or(0.0).round().max(0.0) as u32).map(|_| Value::Null),
            // Settings › Sign-in
            "login.get" => Ok(crate::admin::login()),
            "login.setAutologin" => { crate::admin::run(&["autologin", if a["on"] == true { "on" } else { "off" }], None, &mut |_| {})?; Ok(crate::admin::login()) }
            "login.setStartMode" => crate::admin::set_start_mode(&req("mode")?),
            "login.setLockOnWake" => crate::admin::set_lock_on_wake(a["on"] == true),
            "login.setHostname" => { crate::admin::run(&["hostname", req("name")?.trim()], None, &mut |_| {})?; Ok(crate::admin::login()) }
            "login.password" => { crate::admin::run(&["password"], Some(&format!("{}\n", json!({"old": req("old")?, "new": req("new")?}))), &mut |_| {})?; Ok(json!({"ok": true})) }
            "login.unlock" => Ok(json!({"ok": crate::admin::check_password(a["password"].as_str().unwrap_or(""))})),
            // Settings › Updates (Hearth OS itself)
            "system.update.status" => Ok(self.updates.refresh()),
            "system.update.check" => Ok(self.updates.start(false, self.emit.clone())),
            "system.update.install" => Ok(self.updates.start(true, self.emit.clone())),
            "profile.get" => Ok(crate::profile::get(&self.library.store)),
            "profile.set" => crate::profile::set(&mut self.library.store, a),
            "audio.get" => Ok(system::audio::get(self.max_volume())),
            "audio.setDefault" => { system::audio::set_default(&req("id")?)?; Ok(system::audio::get(self.max_volume())) }
            "perf.stats" => {
                let mut s = crate::perf::sample();
                let st = crate::power::status();
                s["battery"] = system::battery(std::path::Path::new("/sys/class/power_supply"));
                s["runningId"] = Value::Null; s["runningSince"] = Value::Null;
                s["canFpsLimit"] = json!(false); s["fpsLimit"] = self.library.store.data["fpsLimit"].clone();
                s["canOverlay"] = json!(false); s["perfOverlay"] = self.library.store.data["perfOverlay"].clone();
                s["powerMode"] = if st["mode"]["supported"] == true { st["mode"]["mode"].clone() } else { Value::Null };
                s["powerModes"] = st["mode"]["available"].clone(); s["tdp"] = st["tdp"].clone();
                Ok(s)
            }
            "system.info" => Ok(crate::sysinfo::info(std::path::Path::new("/"))),
            "power.get" => { let st = crate::power::status(); let mut b = system::battery(std::path::Path::new("/sys/class/power_supply"));
                b["mode"] = st["mode"]["mode"].clone(); b["modes"] = st["mode"]["available"].clone(); b["modeSupported"] = st["mode"]["supported"].clone(); b["tdp"] = st["tdp"].clone(); Ok(b) }
            "power.setMode" => { crate::power::set_mode(&req("mode")?)?; self.dispatch("power.get", &json!({})) }
            "perf.setTdp" => crate::power::set_tdp(a["watts"].as_f64().unwrap_or(0.0).round() as u32),
            "bt.paired" => Ok(system::bt_devices::paired()),
            "bt.pair" => Ok(system::bt_devices::pair(&req("id")?)),
            "bt.unpair" => Ok(system::bt_devices::unpair(&req("id")?)),
            "bt.confirmPin" => Ok(Value::Null),
            "bt.scanStart" => {
                if system::bt::powered() != Some(true) { return Err("Bluetooth is off".into()); }
                system::bt_scan::start(a["seconds"].as_u64().unwrap_or(15), self.emit.clone()); Ok(Value::Null)
            }
            "bt.scanStop" => { system::bt_scan::stop(); Ok(Value::Null) }
            "bt.autoPads" => { system::bt_scan::auto_pads(a["on"].as_bool().unwrap_or(false), self.emit.clone()); Ok(Value::Null) }
            "buttons.get" => Ok(self.button_prefs()),
            "buttons.set" => self.set_button_prefs(a),
            "setup.done" => io(self.library.store.update(|d| { d.insert("setupDone".into(), json!(now_ms())); })).map(|_| json!({"ok": true})),
            _ => Err(format!("{method} isn't in the Rust version of Hearth yet")),
        }
    }
}

impl Bridge {
    /// 100, or 150 with louder-than-100% sound turned on (Settings › Audio).
    pub fn max_volume(&self) -> u32 {
        let prefs: Value = std::fs::read(paths::config_dir().join("buttons.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null);
        if prefs["overamp"] == true { 150 } else { 100 }
    }
    pub fn button_prefs(&self) -> Value {
        let saved: Value = std::fs::read(paths::config_dir().join("buttons.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null);
        let pick = |k: &str, d: &str| saved[k].as_str().filter(|v| POWER_ACTIONS.contains(v)).unwrap_or(d).to_string();
        json!({"press": pick("press", "sleep"), "hold": pick("hold", "menu"), "overamp": saved["overamp"] == true, "maxVolume": self.max_volume(),
               "watching": {"devices": [], "problems": ["Only in Game Mode"]}})
    }
    pub fn set_button_prefs(&mut self, a: &Value) -> Result<Value, String> {
        let mut cur = self.button_prefs();
        for k in ["press", "hold"] { if let Some(v) = a[k].as_str().filter(|v| POWER_ACTIONS.contains(v)) { cur[k] = json!(v); } }
        if let Some(o) = a["overamp"].as_bool() { cur["overamp"] = json!(o); }
        let save = json!({"press": cur["press"], "hold": cur["hold"], "overamp": cur["overamp"]});
        paths::write_atomic(&paths::config_dir().join("buttons.json"), format!("{}\n", serde_json::to_string_pretty(&save).unwrap()).as_bytes()).map_err(|e| e.to_string())?;
        Ok(self.button_prefs())
    }
    /// An event for the UI ({type:'event', name, data}), like the status the top bar refreshes every 30 s.
    pub fn event(name: &str, data: Value) -> Value { json!({"type": "event", "name": name, "data": data}) }
}

/// Hearth's log (the same file the Electron host writes): ~/.local/share/nimbus/bridge.log
pub fn log(text: &str) {
    use std::io::Write;
    let f = paths::data_dir().join("bridge.log");
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(f) { let _ = writeln!(file, "{} [rust] {}", now_ms(), text); }
}

fn now_ms() -> u64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0) }

fn hostname() -> String { std::fs::read_to_string("/etc/hostname").map(|s| s.trim().to_string()).unwrap_or_default() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn answers_calls_and_says_what_is_missing() {
        let dir = tempfile::tempdir().unwrap();
        let mut b = Bridge { library: Library::new(Store::open(dir.path().join("l.json")), dir.path().to_path_buf()), version: "0.0.23".into(), emit: std::sync::Arc::new(|_| {}), updates: crate::admin::Updates::new() };
        let r = b.handle(&json!({"type":"call","id":7,"method":"library.get","args":{}})).unwrap();
        assert_eq!(r["id"], 7); assert_eq!(r["ok"], true); assert_eq!(r["value"]["games"], json!([]));
        let r = b.handle(&json!({"type":"call","id":8,"method":"epic.signIn"})).unwrap();
        assert_eq!(r["ok"], false); assert!(r["error"].as_str().unwrap().contains("Rust version"));
        assert!(b.handle(&json!({"type":"log","text":"hi"})).is_none());
        let r = b.handle(&json!({"type":"call","id":9,"method":"library.setFav","args":{"id":"steam:1","on":true}})).unwrap();
        assert_eq!(r["ok"], true);
        assert!(b.library.store.list_has("favorites", "steam:1"));
    }
}
