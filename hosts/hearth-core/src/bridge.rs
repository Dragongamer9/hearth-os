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
    pub profiles: crate::profiles::Profiles,
    /// The parental-controls PIN unlocks managing profiles for a few minutes.
    pub pin_until: std::time::Instant,
    /// Asks the person to pick a file (set by the app: a file dialog).
    pub pick_file: std::sync::Arc<dyn Fn(&str) -> Option<String> + Send + Sync>,
    /// Asks for a folder (set by the app): (title, starting folder).
    pub pick_folder: std::sync::Arc<dyn Fn(&str, &str) -> Option<String> + Send + Sync>,
    /// Opens a store's sign-in page and waits for the code it ends with (None: closed). Runs off the bridge.
    pub sign_in: SignInFn,
    /// Opens Xbox Cloud Gaming at an address (Some(url, title)) or closes it (None).
    pub stream: std::sync::Arc<dyn Fn(Option<(&str, &str)>) + Send + Sync>,
    /// The call being answered (calls that answer later, from background work, reply to it themselves).
    cur_id: Value,
}

pub type SignInFn = std::sync::Arc<dyn Fn(&crate::stores::SignIn) -> Option<String> + Send + Sync>;
const DEFERRED: &str = "__deferred";

impl Bridge {
    pub fn new(version: &str) -> Bridge {
        let store = Store::open(paths::data_dir().join("library.json"));
        Bridge::with(Library::new(store, paths::home()), crate::profiles::Profiles::open(&paths::data_dir()), version)
    }
    pub fn with(library: Library, profiles: crate::profiles::Profiles, version: &str) -> Bridge {
        use std::sync::Arc;
        Bridge { library, version: version.to_string(), emit: Arc::new(|_| {}), updates: crate::admin::Updates::new(), profiles, pin_until: std::time::Instant::now(),
                 pick_file: Arc::new(|_| None), pick_folder: Arc::new(|_, _| None), sign_in: Arc::new(|_| None), stream: Arc::new(|_| {}), cur_id: Value::Null }
    }

    /// Messages from the host's own background work: {type:'internal', what, ...}. Returns what to send the page.
    pub fn internal(&mut self, v: &Value) -> Vec<Value> {
        let mut out = Vec::new();
        match v["what"].as_str().unwrap_or("") {
            "rescan" => {
                self.library.scan();
                out.push(Bridge::event("library.changed", json!({})));
                if !v["reply"].is_null() { out.push(json!({"type": "result", "id": v["reply"], "ok": true, "value": null})); }
            }
            "installDone" => {
                let d = self.library.install_done(v["id"].as_str().unwrap_or(""), v["error"].as_str(), v["gogDir"].as_str());
                out.push(Bridge::event("install.done", d));
            }
            "queueChanged" => self.library.save_queue(),
            "streamClosed" => if self.library.running.as_ref().map(|r| r.cloud).unwrap_or(false) { out.push(self.library.exited()); },
            _ => {}
        }
        out
    }

    /// The Steam account signed in on this device: (name, avatar picture).
    fn steam_account(&self) -> (Option<String>, Option<String>, bool) {
        let Some(dir) = crate::steam::steam_path(&self.library.home) else { return (None, None, system::has("steam")) };
        let users = crate::vdf::load(&dir.join("config/loginusers.vdf"));
        let auto = std::fs::read_to_string(paths::home().join(".steam/registry.vdf")).ok().map(|t| crate::vdf::parse(&t))
            .and_then(|r| r.str(&["Registry", "HKCU", "Software", "Valve", "Steam", "AutoLoginUser"]).map(str::to_string));
        let list = users.as_ref().and_then(|u| u.obj(&["users"]).cloned()).unwrap_or_default();
        let pick = list.iter().find(|(_, u)| auto.is_some() && u.str(&["AccountName"]) == auto.as_deref())
            .or_else(|| list.iter().find(|(_, u)| u.str(&["MostRecent"]) == Some("1"))).or(list.first());
        match pick {
            Some((id64, u)) => {
                let name = u.str(&["PersonaName"]).or(u.str(&["AccountName"])).map(str::to_string);
                let av = dir.join("config/avatarcache").join(format!("{id64}.png"));
                (name.or(Some("Signed in".into())), if av.exists() { Some(crate::library::art_url(&av.to_string_lossy())) } else { None }, true)
            }
            None => (None, None, true),
        }
    }

    /// What Library › Stores shows.
    fn accounts_status(&self) -> Value {
        let mut inst: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
        for g in self.library.games.values() { if g.installed { *inst.entry(g.source).or_default() += 1; } }
        let acc = self.library.accounts.lock().unwrap();
        let base = |s: &str| json!({"store": s, "installed": inst.get(s).copied().unwrap_or(0), "busy": acc.busy.contains(s), "error": acc.errors.get(s)});
        let with = |mut b: Value, extra: Value| { for (k, v) in extra.as_object().cloned().unwrap_or_default() { b[k] = v; } b };
        let owned = |s: &str| { let o = crate::owned::Owned::load(crate::stores::cache_of(s)); (o.games.len(), o.fetched) };
        let (sname, savatar, sinstalled) = self.steam_account();
        let steam_owned = self.library.games.values().filter(|g| g.source == "steam").count();
        let (eo, ef) = owned("epic"); let (go, gf) = owned("gog"); let (xo, xf) = owned("xbox");
        let (gname, gavatar) = crate::stores::gog::profile();
        let xb = crate::stores::xbox::saved().unwrap_or(Value::Null);
        let accounts = vec![
            with(base("steam"), json!({"signedIn": sname.is_some(), "name": sname, "avatar": savatar, "owned": steam_owned, "fetched": 0, "toolReady": true,
                "note": if !sinstalled { Some("Steam isn't installed on this device.") } else if sname.is_none() { Some("Open Steam once to sign in") } else { None }})),
            with(base("epic"), json!({"signedIn": crate::stores::epic::signed_in(), "name": crate::stores::epic::display_name(), "avatar": null, "owned": eo, "fetched": ef,
                "toolReady": crate::tools::is_ready(&crate::tools::LEGENDARY), "note": null})),
            with(base("gog"), json!({"signedIn": crate::stores::gog::signed_in(), "name": gname, "avatar": gavatar, "owned": go, "fetched": gf,
                "toolReady": crate::tools::is_ready(&crate::tools::GOGDL), "note": null})),
            with(base("xbox"), json!({"signedIn": !xb.is_null(), "name": xb["gamertag"], "avatar": xb["gamerpic"], "owned": xo, "fetched": xf, "toolReady": true, "note": null})),
        ];
        json!({"accounts": accounts, "installRoot": self.library.install_root().to_string_lossy()})
    }

    /// Signs in to a store: the sign-in page opens in its own window, and the answer comes later (this call is
    /// answered from the background, so Hearth keeps working while you type your password).
    fn start_sign_in(&mut self, store: &str) -> Result<Value, String> {
        let page = crate::stores::sign_in_page(store).ok_or("Unknown store")?;
        let s: &'static str = crate::stores::STORES.into_iter().find(|x| *x == store).unwrap();
        self.library.accounts.lock().unwrap().errors.remove(s);
        let (id, open, acc) = (self.cur_id.clone(), self.sign_in.clone(), self.library.accounts.clone());
        std::thread::spawn(move || {
            let reply = |r: Result<bool, String>| crate::stores::notify(match r {
                Ok(ok) => json!({"type": "result", "id": id, "ok": true, "value": {"ok": ok}}),
                Err(e) => json!({"type": "result", "id": id, "ok": false, "error": e}),
            });
            let Some(code) = open(&page) else { return reply(Ok(false)) };
            if code.is_empty() { return reply(Err("The sign-in page didn't give Hearth a code. Try again.".into())); }
            match crate::stores::finish_sign_in(s, &code) {
                Ok(()) => {
                    log(&format!("accounts: signed in to {s}"));
                    crate::stores::notify(Bridge::event("accounts.changed", json!({})));
                    reply(Ok(true));
                    crate::stores::refresh(s, acc, Value::Null);
                }
                Err(e) => { log(&format!("accounts: {s} sign-in FAILED {e}")); reply(Err(e)); }
            }
        });
        Ok(json!(DEFERRED))
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
        self.cur_id = id.clone();
        Some(match self.dispatch(method, &args) {
            Ok(Value::String(s)) if s == DEFERRED => return None,
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
            "library.setSources" => {
                let src = a.clone();
                io(self.library.store.update(|d| {
                    let s = d.entry("sources").or_insert(json!({}));
                    for k in ["steam", "epic", "gog", "xbox", "emu"] { if let Some(v) = src.get(k) { s[k] = json!(v == true); } }
                }))?;
                Ok(self.library.get(true))
            }
            "library.setSteamBackground" => io(self.library.store.update(|d| { d.insert("steamBackground".into(), json!(a["on"] == true)); })),
            "library.collection.create" => crate::collections::create(&mut self.library.store, a["name"].as_str().unwrap_or("")),
            "library.collection.rename" => crate::collections::rename(&mut self.library.store, &req("id")?, a["name"].as_str().unwrap_or("")).map(|_| Value::Null),
            "library.collection.remove" => crate::collections::remove(&mut self.library.store, &req("id")?).map(|_| Value::Null),
            "library.collection.set" => crate::collections::set_game(&mut self.library.store, &req("id")?, a["game"].as_str().unwrap_or(""), a["on"] == true).map(|_| Value::Null),
            // Backup and restore
            "backup.status" => { let d = crate::backup::destinations(&paths::home()); let dirs: Vec<std::path::PathBuf> = d.iter().filter_map(|x| x["path"].as_str().map(Into::into)).collect(); Ok(json!({"destinations": d, "backups": crate::backup::list(&dirs)})) }
            "backup.create" => crate::backup::create(std::path::Path::new(&req("dest")?), a["signins"] == true, &paths::data_dir(), &paths::config_dir(), &self.version, &chrono::Local::now().format("%Y-%m-%d-%H-%M").to_string()),
            "backup.restore" => {
                let file = req("file")?;
                let d = crate::backup::destinations(&paths::home());
                let dirs: Vec<std::path::PathBuf> = d.iter().filter_map(|x| x["path"].as_str().map(Into::into)).collect();
                if !crate::backup::list(&dirs).iter().any(|b| b["file"] == file.as_str()) { return Err("That backup isn't on a drive Hearth can see anymore".into()); }
                if self.library.running_id().is_some() { return Err("Close the game first".into()); }
                let meta = crate::backup::restore(std::path::Path::new(&file), &paths::data_dir(), &paths::config_dir(), &chrono::Local::now().format("%Y-%m-%d-%H-%M-%S").to_string())?;
                (self.emit)(json!({"type": "restart"}));   // Hearth reads it all at start
                Ok(meta)
            }
            // Remote play
            "remote.status" => Ok(crate::remote::status()),
            "remote.setOn" => crate::remote::set_on(a["on"] == true),
            "remote.pair" => crate::remote::pair(a["pin"].as_str().unwrap_or(""), a["name"].as_str().unwrap_or("Moonlight")).map(|v| json!(v)),
            "remote.unpair" => crate::remote::unpair(&req("uuid")?).map(|v| json!(v)),
            "remote.login" => crate::remote::login(a["user"].as_str().unwrap_or(""), a["pass"].as_str().unwrap_or("")).map(|v| json!(v)),
            "moonlight.open" => Ok(json!(crate::remote::open_moonlight())),
            "moonlight.install" => crate::remote::install_moonlight().map(|v| json!(v)),
            "emu.status" => Ok(crate::emulators::status(&paths::home())),
            "emu.makeFolders" => crate::emulators::make_folders(&paths::home()).map(|p| json!(p)),
            "emu.install" => crate::emulators::install_retroarch().map(|_| json!(true)),
            // Downloads and installs (Epic and GOG through Hearth; Steam's in Steam)
            "library.downloads" => Ok(self.library.downloads()),
            "game.install" => self.library.install(&req("id")?).map(|_| Value::Null),
            "game.cancelInstall" => {
                let id = req("id")?;
                let (ok, removed) = self.library.cancel_install(&id);
                if removed { (self.emit)(Bridge::event("install.done", self.library.install_done(&id, Some("cancelled"), None))); }
                Ok(json!({"ok": ok}))
            }
            "game.pauseInstall" => Ok(json!({"ok": self.library.pause_install(&req("id")?)})),
            "game.resumeInstall" => Ok(json!({"ok": self.library.resume_install(&req("id")?)})),
            "game.installFirst" => Ok(json!({"ok": self.library.install_first(&req("id")?)})),
            "downloads.clearRecent" => { self.library.clear_recent(); Ok(json!({"ok": true})) }
            "steam.downloads" => { let _ = std::process::Command::new("steam").arg("steam://open/downloads").spawn(); Ok(json!({"ok": true})) }
            "game.uninstall" => { self.library.uninstall(&req("id")?)?; Ok(self.library.get(false)) }
            // Library › Stores
            "accounts.status" => Ok(self.accounts_status()),
            "accounts.signIn" => {
                let store = req("store")?;
                if store == "steam" { let _ = std::process::Command::new("steam").arg("steam://open/main").spawn(); return Ok(json!({"ok": true})); }
                self.start_sign_in(&store)
            }
            "accounts.signOut" => { crate::stores::sign_out(&req("store")?)?; self.library.scan(); (self.emit)(Bridge::event("accounts.changed", json!({}))); Ok(Value::Null) }
            "accounts.refresh" => {
                let store = req("store")?;
                let s = crate::stores::STORES.into_iter().find(|x| *x == store).ok_or("Unknown store")?;
                crate::stores::refresh(s, self.library.accounts.clone(), self.cur_id.clone());
                Ok(json!(DEFERRED))
            }
            "accounts.pickFolder" => {
                let cur = self.library.install_root().to_string_lossy().to_string();
                if let Some(dir) = (self.pick_folder)("Where should Hearth install games?", &cur) { io(self.library.store.update(|d| { d.insert("installRoot".into(), json!(dir)); }))?; }
                Ok(json!({"installRoot": self.library.install_root().to_string_lossy()}))
            }
            "game.info" => {
                let id = req("id")?;
                let g = self.library.games.get(&id).cloned().ok_or("That game isn't in the library anymore.")?;
                let home = paths::home();
                let ach = if let Some(app) = id.strip_prefix("steam:") { crate::gameinfo::achievements(&home, app) } else { Value::Null };
                Ok(json!({"protondb": crate::gameinfo::protondb(&id, &g.title, g.source), "achievements": ach}))
            }
            "game.options.get" => self.game_options(&req("id")?),
            "game.options.set" => { let id = req("id")?; crate::game_options::set(&mut self.library.store, &id, &a["options"])?; self.game_options(&id) }
            "game.launch" => {
                let id = req("id")?;
                // Parental controls: allowed games and play time a day.
                let pc = self.profiles.can_play(Some(&id), chrono::Local::now(), 0.0);
                if pc["ok"] != true { return Err(pc["reason"].as_str().unwrap_or("Not now").to_string()); }
                let r = self.library.launch(&id)?;
                if let Some(url) = r["stream"].as_str() { (self.stream)(Some((url, r["title"].as_str().unwrap_or("Xbox Cloud Gaming")))); }
                Ok(json!({"tracked": r["tracked"]}))
            }
            "game.close" => {
                if self.library.running.as_ref().map(|r| r.cloud).unwrap_or(false) {
                    (self.stream)(None);
                    let e = self.library.exited();
                    (self.emit)(e);
                    return Ok(json!({"ok": true}));
                }
                Ok(json!({"ok": self.library.close_running()}))
            }
            "game.pause" => {
                if self.library.running.as_ref().map(|r| r.cloud).unwrap_or(false) { return Err("Cloud games can't be paused here: pause from the game's own menu.".into()); }
                Ok(json!({"paused": self.library.pause()?}))
            }
            "game.unpause" => { self.library.resume(); Ok(json!({"paused": false})) }
            "library.addManual" => {
                let file = match a["path"].as_str() { Some(p) if !p.is_empty() => p.to_string(), _ => match (self.pick_file)("Choose a game: a program, AppImage, .desktop file or Windows .exe") { Some(f) => f, None => return Ok(Value::Null) } };
                if !std::path::Path::new(&file).exists() { return Err(format!("File not found: {file}")); }
                let id = format!("manual:{:012x}", crate::library::now_ms() * 1000 + (std::process::id() as u64 % 1000));
                let m = json!({"id": id, "title": crate::manual::title_for(&file), "path": file, "added": crate::library::now_ms()});
                io(self.library.store.update(|d| { d.entry("manual").or_insert(json!([])).as_array_mut().unwrap().push(m); }))?;
                self.library.scan();
                self.library.games.get(&id).map(|g| self.library.dto(g)).ok_or("Couldn't add it".into())
            }
            "library.remove" => {
                let id = req("id")?;
                io(self.library.store.update(|d| {
                    for list in ["manual"] { if let Some(a) = d.get_mut(list).and_then(|v| v.as_array_mut()) { a.retain(|m| m["id"] != id.as_str()); } }
                    for list in ["favorites", "hidden"] { if let Some(a) = d.get_mut(list).and_then(|v| v.as_array_mut()) { a.retain(|m| m != id.as_str()); } }
                    if let Some(st) = d.get_mut("stats").and_then(|v| v.as_object_mut()) { st.remove(&id); }
                }))?;
                self.library.scan();
                Ok(Value::Null)
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
            // Profiles and parental controls (switching restarts Hearth; that part comes with the app's restart)
            "profiles.get" => Ok(self.profiles_info()),
            "profiles.create" => { self.need_pin()?; self.profiles.create(a["name"].as_str().unwrap_or(""), a["color"].as_str()) }
            "profiles.update" => {
                let id = req("id")?;
                let own_look = id == self.profiles.active() && ["steam"].iter().all(|k| a.get(*k).is_none());
                if !own_look { self.need_pin()?; }
                self.profiles.update(&id, a)
            }
            "profiles.remove" => { self.need_pin()?; self.profiles.remove(&req("id")?).map(|_| Value::Null) }
            "profiles.setAsk" => { self.need_pin()?; self.profiles.set_ask_at_start(a["on"] == true).map(|_| Value::Null) }
            "profiles.unlock" => {
                if !self.profiles.check_pin(a["pin"].as_str().unwrap_or("")) { return Err("That PIN isn't right".into()); }
                self.pin_until = std::time::Instant::now() + std::time::Duration::from_secs(300); Ok(json!(true))
            }
            "profiles.setPin" => { self.need_pin()?; let pin = a["pin"].as_str().unwrap_or(""); self.profiles.set_pin(pin)?;
                if !pin.is_empty() { self.pin_until = std::time::Instant::now() + std::time::Duration::from_secs(300); } Ok(json!(true)) }
            "profiles.setLimits" => { self.need_pin()?; self.profiles.set_limits(&req("id")?, a) }
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
    pub fn game_options(&self, id: &str) -> Result<Value, String> {
        let g = self.library.games.get(id).ok_or("That game isn't in the library anymore.")?;
        let kind = match g.source { "steam" => "steam", "emu" => "emu", "manual" if g.exe.as_deref().map(|e| e.to_lowercase().ends_with(".exe")).unwrap_or(false) => "proton", "manual" => "native", _ => "external" };
        Ok(json!({"id": id, "kind": kind, "options": crate::game_options::get(&self.library.store, id),
                  "protonChoices": if kind == "proton" { crate::game_options::proton_choices(&paths::home()) } else { vec![] },
                  "gamemode": system::has("gamemoderun"), "canResetPrefix": false}))
    }
    fn need_pin(&self) -> Result<(), String> {
        if self.profiles.has_pin() && std::time::Instant::now() >= self.pin_until { Err("Enter the PIN first".into()) } else { Ok(()) }
    }
    pub fn profiles_info(&self) -> Value {
        let now = chrono::Local::now();
        let active = self.profiles.active();
        let steam_users = crate::steam::steam_path(&paths::home()).and_then(|s| std::fs::read_to_string(s.join("config/loginusers.vdf")).ok()).unwrap_or_default();
        let list: Vec<Value> = self.profiles.list().into_iter().map(|mut p| {
            let id = p["id"].as_str().unwrap_or("").to_string();
            p["current"] = json!(id == active); p["limits"] = self.profiles.limits(&id); p["playedToday"] = json!(self.profiles.played_today(&id, now).round());
            p
        }).collect();
        json!({"active": active, "askAtStart": self.profiles.data["askAtStart"] == true, "picked": std::env::args().any(|a| a == "--picked"),
               "hasPin": self.profiles.has_pin(), "unlocked": self.profiles.has_pin() && std::time::Instant::now() < self.pin_until,
               "restricted": self.profiles.restricted(&active), "left": self.profiles.can_play(None, now, 0.0)["minutesLeft"],
               "list": list, "steamAccounts": crate::profiles::steam_accounts(&steam_users)})
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
        let mut b = Bridge::with(Library::new(Store::open(dir.path().join("l.json")), dir.path().to_path_buf()), crate::profiles::Profiles::open(dir.path()), "0.0.23");
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
