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
    pub xodus: crate::xodus::Xodus,
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
    /// Hearth's interface zoom (Settings › Display › Scale in Game Mode): the host applies it.
    pub zoom: std::sync::Arc<dyn Fn(u32) + Send + Sync>,
    pub osinstall: crate::osinstall::OsInstaller,
    pub importer: crate::importer::Importer,
    /// The controllers' Guide button (shared with the host, which starts it).
    pub guide: crate::guide::Guide,
    /// The power and volume buttons Hearth watches in Game Mode: {devices, problems}
    pub buttons: std::sync::Arc<std::sync::Mutex<Value>>,
    /// Things only the app's windows can do: ("focusApp", {id, pid}), ("screenshot", {before: bool}), ("comeBack", null)...
    pub host: std::sync::Arc<dyn Fn(&str, &Value) + Send + Sync>,
    /// Parental controls and per-game settings while a game plays.
    parental_since: u64,
    parental_warned: bool,
    /// The frame-rate limit to put back when the game with its own cap ends.
    per_game: Option<u64>,
    /// Per-game power: the game last seen in front, and what to put back when no game with its own power
    /// settings is in front ({"hhd": {key: value}} on Handheld Daemon systems, {"watts": n} elsewhere).
    power_seen: Option<String>,
    power_back: Option<Value>,
    browser: std::sync::Arc<std::sync::Mutex<Value>>,
    /// Claude Desktop / Claude Code being installed by desktop/shell/hearth-claude (Settings › System › Claude).
    claude: std::sync::Arc<std::sync::Mutex<Value>>,
    pub app_updates: crate::appupdate::AppUpdates,
    /// The call being answered (calls that answer later, from background work, reply to it themselves).
    cur_id: Value,
    /// Which window sent the request in hand ("main", "overlay"): an answer given later goes back to it only (each
    /// window numbers its own requests, so the other one could take it for one of its own).
    pub cur_label: String,
}

pub type SignInFn = std::sync::Arc<dyn Fn(&crate::stores::SignIn) -> Option<String> + Send + Sync>;
/// What a call returns when it's answered later, from a thread (crate::space::answer_later).
pub const DEFERRED: &str = "__deferred";
const WIFI_BUSY_TEXT: &str = "Wi-Fi is already busy (looking for networks or connecting). Try again in a moment.";

impl Bridge {
    pub fn new(version: &str) -> Bridge {
        let store = Store::open(paths::data_dir().join("library.json"));
        let mut b = Bridge::with(Library::new(store, paths::home()), crate::profiles::Profiles::open(&paths::data_dir()), version);
        // Quick Resume games (and a game that was running) are still there after Hearth restarts.
        b.library.restore_quick();
        // Steam: with "Start Steam only when needed" (Settings › Library, on by default) it starts hidden when you
        // play, install or use its features (steam_ctl.rs). Turned off: started hidden now and kept running, as before.
        if !b.library.steam_on_demand() && b.library.store.data["steamBackground"] != false {
            let home = b.library.home.clone();
            std::thread::spawn(move || { std::thread::sleep(std::time::Duration::from_secs(8)); crate::steam::ensure_running_silent(&home); });
        }
        // The Steam games on your account (steam_owned.rs), fetched again when older than 12 hours.
        if b.library.store.source_on("steam") {
            let home = b.library.home.clone();
            std::thread::spawn(move || { std::thread::sleep(std::time::Duration::from_secs(20)); crate::steam_owned::refresh(&home, false); });
        }
        b
    }
    pub fn with(library: Library, profiles: crate::profiles::Profiles, version: &str) -> Bridge {
        use std::sync::Arc;
        Bridge { library, version: version.to_string(), emit: Arc::new(|_| {}), updates: crate::admin::Updates::new(), xodus: crate::xodus::Xodus::new(), profiles, pin_until: std::time::Instant::now(),
                 pick_file: Arc::new(|_| None), pick_folder: Arc::new(|_, _| None), sign_in: Arc::new(|_| None), stream: Arc::new(|_| {}), zoom: Arc::new(|_| {}), osinstall: crate::osinstall::OsInstaller::new(), importer: crate::importer::Importer::new(), guide: crate::guide::Guide::new(),
                 buttons: Arc::new(std::sync::Mutex::new(json!({"devices": [], "problems": ["Only in Game Mode"]}))), host: Arc::new(|_, _| {}),
                 parental_since: 0, parental_warned: false, per_game: None, power_seen: None, power_back: None, browser: Arc::new(std::sync::Mutex::new(json!({"state": "idle", "progress": null, "line": "", "error": null}))),
                 claude: Arc::new(std::sync::Mutex::new(json!({"state": "idle", "progress": null, "line": "", "error": null}))),
                 app_updates: crate::appupdate::AppUpdates::new(version), cur_id: Value::Null, cur_label: "main".into() }
    }

    /// Messages from the host's own background work: {type:'internal', what, ...}. Returns what to send the page.
    pub fn internal(&mut self, v: &Value) -> Vec<Value> {
        let mut out = Vec::new();
        match v["what"].as_str().unwrap_or("") {
            "rescan" => {
                self.library.scan();
                out.push(Bridge::event("library.changed", json!({})));
                // {id, to}: the answer to accounts.refresh, for the window that asked (a bare id: any window).
                let r = &v["reply"];
                if !r.is_null() {
                    let mut m = json!({"type": "result", "id": if r.is_object() { r["id"].clone() } else { r.clone() }, "ok": true, "value": null});
                    if let Some(t) = r["to"].as_str() { m["to"] = json!(t); }
                    out.push(m);
                }
            }
            "installDone" => {
                let d = self.library.install_done(v["id"].as_str().unwrap_or(""), v["error"].as_str(), v["gogDir"].as_str());
                out.push(Bridge::event("install.done", d));
            }
            "queueChanged" => self.library.save_queue(),
            "importDone" => out.extend(self.import_done(&v["result"])),
            "epicImported" => {
                let arr = |k: &str| v[k].as_array().cloned().unwrap_or_default();
                let tried = arr("tried");
                let now = crate::importer::pending_after(&self.pending_epic(), &tried, &arr("left"));
                let fresh: Vec<Value> = now.iter().filter(|g| !tried.contains(g)).cloned().collect();
                let _ = self.library.store.update(|d| { d.insert("pendingEpicImports".into(), json!(now)); });
                // Added while that round ran: their turn now.
                if !fresh.is_empty() { self.importer.finish_pending_epic(fresh); }
                if v["done"].as_u64().unwrap_or(0) > 0 { self.library.scan(); out.push(Bridge::event("library.changed", json!({}))); }
            }
            "importOffer" => {
                if crate::admin::can_admin() && self.library.store.data["importOffered"].is_null() {
                    let _ = self.library.store.update(|d| { d.insert("importOffered".into(), json!(now_ms())); });
                    self.importer.offer();
                }
            }
            "epicSignedIn" => self.importer.finish_pending_epic(self.pending_epic()),
            // Steam's own sign-in window finished (steam_ctl::watch_sign_in saw Steam write its account list).
            "steamSignedIn" => {
                (self.host)("comeBack", &Value::Null);
                let (name, _, _) = self.steam_account();
                crate::steam_owned::refresh(&self.library.home, true);
                self.library.scan();
                out.push(Bridge::event("steam.signedIn", json!({"name": name})));
                out.push(Bridge::event("accounts.changed", json!({})));
                out.push(Bridge::event("library.changed", json!({})));
            }
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

    /// Opens a steam:// address (Steam started hidden first when it isn't running), then brings Steam's window in
    /// front in Game Mode. Off the bridge's thread: starting Steam takes a while.
    fn open_steam(&self, url: &str) {
        let (home, host, url) = (self.library.home.clone(), self.host.clone(), url.to_string());
        std::thread::spawn(move || {
            if let Err(e) = crate::steam_ctl::open_url(&home, &url) { log(&format!("steam: {e}")); return; }
            host("showSteam", &json!({"wait": 45}));
        });
    }

    /// What Library › Stores shows.
    fn accounts_status(&self) -> Value {
        let mut inst: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
        for g in self.library.games.values() { if g.installed { *inst.entry(g.source).or_default() += 1; } }
        let acc = self.library.accounts.lock().unwrap();
        // expired / expiredAt: the store refused the saved sign-in (stores::expiry); the page offers Sign in again.
        let base = |s: &str| { let x = crate::stores::expiry::get(s);
            json!({"store": s, "installed": inst.get(s).copied().unwrap_or(0), "busy": acc.busy.contains(s), "error": acc.errors.get(s),
                   "expired": x.is_some(), "expiredAt": x.as_ref().map(|x| x["at"].clone())}) };
        // The name to sign in again as: the account's, or the one last seen (legendary forgets it when Epic refuses).
        let named = |s: &str, n: Option<String>| -> Option<String> {
            match n { Some(n) => { crate::stores::expiry::remember_name(s, &n); Some(n) }
                      None => crate::stores::expiry::get(s).and_then(|x| x["name"].as_str().map(str::to_string)).or_else(|| crate::stores::expiry::known_name(s).filter(|_| crate::stores::expiry::is_expired(s))) } };
        let with = |mut b: Value, extra: Value| { for (k, v) in extra.as_object().cloned().unwrap_or_default() { b[k] = v; } b };
        let owned = |s: &str| { let o = crate::owned::Owned::load(crate::stores::cache_of(s)); (o.games.len(), o.fetched) };
        let (sname, savatar, sinstalled) = self.steam_account();
        let steam_owned = self.library.games.values().filter(|g| g.source == "steam").count();
        let so = crate::steam_owned::load_current(&self.library.home).unwrap_or(json!({}));
        let (s_hint, s_error) = match so["problem"].as_str() {
            Some("private") => (Some(crate::steam_owned::PRIVATE_NOTE.to_string()), None),
            Some(p) => (None, Some(format!("Couldn't get your Steam games list: {p}"))),
            None => (None, None),
        };
        let (eo, ef) = owned("epic"); let (go, gf) = owned("gog"); let (xo, xf) = owned("xbox");
        let (gname, gavatar) = crate::stores::gog::profile();
        let gname = named("gog", gname);
        let xb = crate::stores::xbox::saved().unwrap_or(Value::Null);
        let ename = named("epic", crate::stores::epic::display_name());
        let xname = named("xbox", xb["gamertag"].as_str().map(str::to_string));
        let accounts = vec![
            with(base("steam"), json!({"signedIn": sname.is_some(), "name": sname, "avatar": savatar, "owned": steam_owned, "fetched": so["fetched"].as_u64().unwrap_or(0), "toolReady": true,
                "busy": crate::steam_owned::busy(), "signing": crate::steam_ctl::signing_in(), "hint": s_hint, "error": s_error,
                "library": self.library.steam_library().map(|l| l.to_string_lossy().to_string()),
                "note": if !sinstalled { Some("Steam isn't installed on this device.") } else if sname.is_none() { Some("Not signed in") } else { None }})),
            with(base("epic"), json!({"signedIn": crate::stores::epic::signed_in(), "name": ename, "avatar": null, "owned": eo, "fetched": ef,
                "toolReady": crate::tools::is_ready(&crate::tools::LEGENDARY), "note": null})),
            with(base("gog"), json!({"signedIn": crate::stores::gog::signed_in(), "name": gname, "avatar": gavatar, "owned": go, "fetched": gf,
                "toolReady": crate::tools::is_ready(&crate::tools::GOGDL), "note": null})),
            with(base("xbox"), json!({"signedIn": !xb.is_null(), "name": xname, "avatar": xb["gamerpic"], "owned": xo, "fetched": xf, "toolReady": true, "note": null})),
        ];
        let bs = &self.library.beacon;
        let beacon = json!({"installed": bs.cli.is_some(), "app": bs.app.is_some(), "via": bs.cli.as_ref().map(|c| c.via()),
            "games": bs.games.len(), "setUp": bs.games.iter().filter(|g| g.installed).count(), "error": bs.error});
        json!({"accounts": accounts, "installRoot": self.library.install_root().to_string_lossy(), "beacon": beacon})
    }

    /// Signs in to a store: the sign-in page opens in its own window, and the answer comes later (this call is
    /// answered from the background, so Hearth keeps working while you type your password).
    fn start_sign_in(&mut self, store: &str) -> Result<Value, String> {
        let page = crate::stores::sign_in_page(store).ok_or("Unknown store")?;
        let s: &'static str = crate::stores::STORES.into_iter().find(|x| *x == store).unwrap();
        self.library.accounts.lock().unwrap().errors.remove(s);
        let (id, open, acc, to) = (self.cur_id.clone(), self.sign_in.clone(), self.library.accounts.clone(), self.cur_label.clone());
        std::thread::spawn(move || {
            // To the window that asked only (each window numbers its own requests).
            let reply = |r: Result<bool, String>| crate::stores::notify(match r {
                Ok(ok) => json!({"type": "result", "id": id, "ok": true, "value": {"ok": ok}, "to": to}),
                Err(e) => json!({"type": "result", "id": id, "ok": false, "error": e, "to": to}),
            });
            let Some(code) = open(&page) else { return reply(Ok(false)) };
            if code.is_empty() { return reply(Err("The sign-in page didn't give Hearth a code. Try again.".into())); }
            match crate::stores::finish_sign_in(s, &code) {
                Ok(()) => {
                    log(&format!("accounts: signed in to {s}"));
                    crate::stores::expiry::clear(s);
                    crate::stores::notify(Bridge::event("accounts.changed", json!({})));
                    reply(Ok(true));
                    if s == "epic" { crate::stores::notify(json!({"type": "internal", "what": "epicSignedIn"})); }
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
        // A request that panics (bad data somewhere) still gets an answer. Caught here, with the bridge still held,
        // its lock isn't poisoned: before, the page waited forever, the fail-safe saw "stuck" (killing helpers,
        // sending a report, restarting Hearth), and Game Mode's own uses of the bridge failed until then.
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.dispatch(method, &args))).unwrap_or_else(|p| {
            let why = p.downcast_ref::<&str>().map(|s| s.to_string()).or_else(|| p.downcast_ref::<String>().cloned()).unwrap_or_default();
            log(&format!("{method}: panicked: {why}"));
            Err("Something went wrong in Hearth. Try again; if it keeps happening, send the logs (Settings › System).".to_string())
        });
        Some(match r {
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
                "platform": "linux", "name": os_name(), "host": "Hearth (Rust)", "version": self.version, "autoReport": self.auto_report(),
                // Drawn the compatible way (no DMA-BUF): the page turns its heavy effects down (body.lite).
                "compatDrawing": std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_some() && std::env::var("HEARTH_RENDERER_RUNNING").as_deref() != Ok("chromium"),
                // The engine drawing these screens: "webkit" (Tauri) or "chromium" (the Electron window app).
                "renderer": if std::env::var("HEARTH_RENDERER_RUNNING").as_deref() == Ok("chromium") { "chromium" } else { "webkit" },
                // Multi-seat window: Hearth passes this player's controller on itself (the page doesn't read it).
                "seatPads": crate::multiplay::seat_hosted(),
                // Part of Hearth, the OS: the app updates with the system (Settings › Updates shows Update now).
                "system": system_install(),
                // Opened just for Settings (from Mantel's taskbar), or for one of its pages (--page hhd: Handheld
                // Daemon's settings, from Mantel's Controller page).
                "start": start_page(&std::env::args().collect::<Vec<_>>()),
                // Game Mode (full screen in gamescope): store sign-ins go to Desktop Mode. A windowed Hearth opened there
                // for one gets --signin <store> and starts it (see signin_handoff).
                "gameMode": crate::signin_handoff::game_mode(),
                "windowed": crate::signin_handoff::windowed(),
                "signin": crate::signin_handoff::signin_arg(),
                // Opened from Mantel's Games section with --play <id>: the page plays that game (desktop_games.rs).
                "play": crate::desktop_games::play_arg(),
                "edition": edition(),
                // The speed test (CI) asks the screens to time themselves.
                "perfLog": std::env::var_os("HEARTH_PERF_LOG").is_some(),
                "session": std::env::var("NIMBUS_SESSION").map(|v| v == "1").unwrap_or(false),
                "user": std::env::var("USER").unwrap_or_default(),
                // The player counter's id for this device: the same across updates (a hash of the machine id).
                "deviceId": counter_device_id(),
                "live": std::path::Path::new("/etc/hearth-live").exists(),
                "installer": std::path::Path::new("/etc/hearth-installer").exists(),
                "features": features(), "setupDone": self.setup_done(),
                "openInstaller": std::path::Path::new("/etc/hearth-live").exists() && crate::osinstall::handoff(false),
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
            "library.setSteamOnDemand" => {
                let on = a["on"] == true;
                // Off: Steam runs all the time again (started now, hidden).
                if !on { let home = self.library.home.clone(); std::thread::spawn(move || crate::steam::ensure_running_silent(&home)); }
                io(self.library.store.update(|d| { d.insert("steamOnDemand".into(), json!(on)); }))
            }
            "library.setSteamBackground" => {
                let on = a["on"] == true;
                if on { let home = self.library.home.clone(); std::thread::spawn(move || crate::steam::ensure_running_silent(&home)); }
                io(self.library.store.update(|d| { d.insert("steamBackground".into(), json!(on)); }))
            }
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
            // Starting Sunshine and waiting for it: up to 45 s, off the bridge.
            "remote.setOn" => { let on = a["on"] == true; self.later(move || crate::remote::set_on(on)) }
            "remote.pair" => crate::remote::pair(a["pin"].as_str().unwrap_or(""), a["name"].as_str().unwrap_or("Moonlight")).map(|v| json!(v)),
            "remote.unpair" => crate::remote::unpair(&req("uuid")?).map(|v| json!(v)),
            "remote.login" => crate::remote::login(a["user"].as_str().unwrap_or(""), a["pass"].as_str().unwrap_or("")).map(|v| json!(v)),
            "moonlight.open" => Ok(json!(crate::remote::open_moonlight())),
            "moonlight.install" => crate::remote::install_moonlight().map(|v| json!(v)),
            // ---- Hearth's gaming apps (gaming_apps.rs): Controller Lab, Saves and Mods, opened on a game
            "gamingApps.open" => crate::gaming_apps::open(&req("app")?, a["game"].as_str(), crate::signin_handoff::game_mode()).map(|_| json!({"ok": true})),
            // ---- end of Hearth's gaming apps
            "emu.status" => Ok(crate::emulators::status(&paths::home())),
            "emu.makeFolders" => crate::emulators::make_folders(&paths::home()).map(|p| json!(p)),
            "emu.install" => crate::emulators::install_retroarch().map(|_| json!(true)),
            // Downloads and installs (Epic and GOG through Hearth; Steam's in Steam)
            "library.downloads" => Ok(self.library.downloads()),
            "game.install" => {
                let id = req("id")?;
                // Steam games install without Steam's dialog (steam_ctl.rs): {silent, waiting}.
                if id.starts_with("steam:") { return self.library.install_steam(&id); }
                self.library.install(&id).map(|_| Value::Null)
            }
            "game.cancelInstall" => {
                let id = req("id")?;
                if let Some(app) = id.strip_prefix("steam:").and_then(|a| a.parse::<u32>().ok()) {
                    if !crate::steam_ctl::cancel_pending(app) { return Err("Steam is downloading it already: cancel it in Steam (Manage in Steam).".into()); }
                    self.library.scan();
                    (self.emit)(Bridge::event("library.changed", json!({})));
                    return Ok(json!({"ok": true}));
                }
                let (ok, removed) = self.library.cancel_install(&id);
                if removed { (self.emit)(Bridge::event("install.done", self.library.install_done(&id, Some("cancelled"), None))); }
                Ok(json!({"ok": ok}))
            }
            "game.pauseInstall" => Ok(json!({"ok": self.library.pause_install(&req("id")?)})),
            "game.resumeInstall" => Ok(json!({"ok": self.library.resume_install(&req("id")?)})),
            "game.installFirst" => Ok(json!({"ok": self.library.install_first(&req("id")?)})),
            "downloads.clearRecent" => { self.library.clear_recent(); Ok(json!({"ok": true})) }
            "steam.downloads" => { self.open_steam("steam://open/downloads"); Ok(json!({"ok": true})) }
            // Steam's window in front again (during a sign-in, after coming back to Hearth).
            "steam.showWindow" => { (self.host)("showSteam", &json!({"wait": 30})); Ok(json!({"ok": true})) }
            "steam.signInCancel" => { crate::steam_ctl::cancel_sign_in(); (self.host)("comeBack", &Value::Null); Ok(json!({"ok": true})) }
            // Where Steam games install: Steam's libraries, and the one picked.
            "steam.libraries" => {
                let steam = crate::steam::steam_path(&self.library.home).ok_or("Steam isn't set up yet. Open Steam once, then try again.")?;
                let chosen = self.library.steam_library().unwrap_or_default();
                Ok(json!({"list": crate::steam_ctl::libraries_dto(&steam, &chosen), "chosen": chosen.to_string_lossy()}))
            }
            "steam.setLibrary" => {
                let path = req("path")?;
                let steam = crate::steam::steam_path(&self.library.home).ok_or("Steam isn't set up yet.")?;
                if !crate::steam::libraries(&steam).iter().any(|l| l.to_string_lossy() == path) { return Err("That folder isn't a Steam library.".into()); }
                io(self.library.store.update(|d| { d.insert("steamLibrary".into(), json!(path)); }))?;
                Ok(json!({"chosen": path}))
            }
            "game.uninstall" => {
                let id = req("id")?;
                // Steam games: deleted without Steam's dialog once Hearth's own confirmation was given (steam_ctl::uninstall:
                // never while Steam works on the game or it runs, only inside its Steam library). Answered later: Steam
                // may have to close first.
                if let Some(app) = id.strip_prefix("steam:") {
                    if self.library.running_id().as_deref() == Some(id.as_str()) || self.library.frozen.iter().any(|f| f.id == id) { return Err("Exit the game first.".into()); }
                    let app: u32 = app.parse().map_err(|_| "That isn't a Steam game.".to_string())?;
                    let title = self.library.games.get(&id).map(|g| g.title.clone()).unwrap_or_else(|| "the game".into());
                    let (home, on_demand) = (self.library.home.clone(), self.library.steam_on_demand());
                    crate::space::answer_later(self.cur_id.clone(), move || {
                        let was_running = crate::steam_ctl::uninstall(&home, app, &title)?;
                        // Steam was closed for it: back hidden, unless it only runs when needed.
                        if was_running && !on_demand { crate::steam::ensure_running_silent(&home); }
                        crate::stores::notify(json!({"type": "internal", "what": "rescan", "reply": null}));
                        Ok(json!({"steam": true, "silent": true}))
                    });
                    return Ok(json!(DEFERRED));
                }
                self.library.uninstall(&id)?;
                Ok(self.library.get(false))
            }
            // Library › Stores
            "accounts.status" => Ok(self.accounts_status()),
            "accounts.signIn" => {
                let store = req("store")?;
                // Steam: its own sign-in window, in front; Hearth only watches Steam's files to see when it's done
                // (steam_ctl::watch_sign_in). Hearth never signs in itself.
                if store == "steam" {
                    if !crate::steam::client_installed(&self.library.home) { return Err("Steam isn't installed on this device.".into()); }
                    crate::steam_ctl::watch_sign_in(&self.library.home);
                    self.open_steam("steam://open/main");
                    return Ok(json!({"ok": true, "watching": true}));
                }
                self.start_sign_in(&store)
            }
            // Game Mode: the sign-in opens in Desktop Mode (the page switches there right after this).
            "accounts.signInDesktop" => { crate::signin_handoff::write_request(&req("store")?)?; Ok(json!({"ok": true})) }
            // A windowed Hearth in Desktop Mode, after a sign-in: back to Game Mode.
            "session.gameMode" => crate::signin_handoff::back_to_game_mode().map(|_| Value::Null),
            "accounts.signOut" => { crate::stores::sign_out(&req("store")?)?; self.library.scan(); (self.emit)(Bridge::event("accounts.changed", json!({}))); Ok(Value::Null) }
            "accounts.refresh" => {
                let store = req("store")?;
                if store == "steam" { crate::steam_owned::refresh(&self.library.home, true); return Ok(json!({"ok": true})); }
                let s = crate::stores::STORES.into_iter().find(|x| *x == store).ok_or("Unknown store")?;
                crate::stores::refresh(s, self.library.accounts.clone(), json!({"id": self.cur_id, "to": self.cur_label}));
                Ok(json!(DEFERRED))
            }
            "accounts.pickFolder" => {
                let cur = self.library.install_root().to_string_lossy().to_string();
                if let Some(dir) = (self.pick_folder)("Where should Hearth install games?", &cur) { io(self.library.store.update(|d| { d.insert("installRoot".into(), json!(dir)); }))?; }
                Ok(json!({"installRoot": self.library.install_root().to_string_lossy()}))
            }
            // Settings › Import games, and removing the old system afterwards
            "import.status" => Ok(self.importer.status(self.pending_epic().len())),
            "import.scan" => self.importer.scan(self.pending_epic().len()),
            "import.start" => self.importer.start(&a["items"], a["mode"].as_str().unwrap_or("copy"), &self.library.install_root(), self.pending_epic().len()),
            "import.cancel" => Ok(self.importer.cancel(self.pending_epic().len())),
            "takeover.plan" => { let (st, n) = (self.importer.state.clone(), self.pending_epic().len()); self.later(move || crate::importer::takeover_plan_with(&st, n)) }
            "takeover.run" => self.importer.takeover_run(&a["remove"], a["windowsBoot"] != false, self.pending_epic().len()),
            // Move (Beta Experimental): copy, check every file, register, then delete the original (hearth-import).
            "import.move" => self.importer.start(&a["items"], "move", &self.library.install_root(), self.pending_epic().len()),
            // Make Hearth bigger (Beta Experimental): space from the other systems on Hearth's drive (hearth-space,
            // docs/MOVE-AND-GROW.md). Looking can take half a minute, so it answers from another thread.
            "space.status" => self.later(crate::space::status),
            "space.grow" => {
                let (from, gb) = (req("from")?, a["gb"].as_u64().unwrap_or(0));
                if a["dryRun"] == true {
                    self.later(move || crate::space::plan(&from, gb))
                } else { crate::space::grow(&from, gb, self.emit.clone()) }
            }
            "space.clearJournal" => crate::space::clear_journal(),
            "art.status" => Ok(crate::artcache::status()),
            // Steam friends: who's online and what they're playing
            "steam.friends" => Ok(crate::friends::list(&self.library.home)),
            "friends.action" => {
                let r = crate::friends::action(a["platform"].as_str().unwrap_or(""), a["id"].as_str().unwrap_or(""), a["what"].as_str().unwrap_or(""), a["server"].as_str())?;
                if let Some(u) = r["open"].as_str() {
                    // Steam's chat or game in front of Hearth (it's a separate window in Game Mode), Steam started
                    // hidden first if it isn't running.
                    self.open_steam(u);
                }
                Ok(r)
            }
            // Game Mode: frame-rate limit, performance overlay, Quick Access over the game, open apps
            "perf.setFpsLimit" => { let n = crate::gamescope::set_fps_limit(a["fps"].as_f64().unwrap_or(0.0).max(0.0).round() as u64)?; io(self.library.store.update(|d| { d.insert("fpsLimit".into(), json!(n)); }))?; Ok(json!({"fpsLimit": n})) }
            "perf.setOverlay" => { let n = crate::gamescope::set_perf_overlay(a["level"].as_f64().unwrap_or(0.0).max(0.0).round() as u64)?; io(self.library.store.update(|d| { d.insert("perfOverlay".into(), json!(n)); }))?; Ok(json!({"perfOverlay": n})) }
            "overlay.settings" | "overlay.setGameBarButton" => Ok(self.overlay_settings()),
            "overlay.setHoldView" => {
                let on = a["on"] == true;
                io(self.library.store.update(|d| { d.insert("holdView".into(), json!(on)); }))?;
                self.guide.hold_view.store(on, std::sync::atomic::Ordering::SeqCst);
                Ok(self.overlay_settings())
            }
            "overlay.screenshot" => {
                (self.host)("screenshot", &json!({"before": true}));
                let r = crate::gamescope::screenshot();
                (self.host)("screenshot", &json!({"before": false}));
                r
            }
            "screenshots.list" => Ok(json!({"dir": crate::gamescope::screenshot_dir().to_string_lossy(), "list": crate::gamescope::screenshots()})),
            "screenshots.delete" => { crate::gamescope::delete_screenshot(a["file"].as_str().unwrap_or(""))?; Ok(json!({"list": crate::gamescope::screenshots()})) }
            "apps.list" => Ok(if crate::gamescope::inside() { self.apps_list() } else { json!([]) }),
            "apps.focus" => { (self.host)("focusApp", &json!({"id": a["id"], "pid": a["pid"]})); Ok(Value::Null) }
            "apps.close" => {
                let pid = a["pid"].as_u64().unwrap_or(0);
                if pid == 0 || pid == std::process::id() as u64 { return Err("Hearth can't close itself here.".into()); }
                if self.library.game_pids().contains(&(pid as u32)) { return Ok(json!({"ok": self.library.close_running()})); }
                std::process::Command::new("kill").args(["-TERM", &pid.to_string()]).status().map_err(|e| format!("Couldn't close it: {e}"))?;
                Ok(json!({"ok": true}))
            }
            // Game Mode's controller mouse (padmouse.rs): Quick Access › Controller mouse, or View + Menu.
            "pad.status" => Ok(json!({"connected": 0, "mouseMode": "off", "mouseOn": crate::padmouse::is_on()})),
            "pad.toggleMouse" => Ok(json!({"mouseOn": crate::padmouse::toggle()?})),
            "pad.setMouseMode" => Err("On Hearth OS the controller mouse is in Quick Access (or View + Menu together).".into()),
            // Settings › Controls: remapping (InputPlumber)
            "controller.status" => crate::guide::controller(&["status"]),
            "controller.set" => {
                let mut r = Value::Null;
                if !a["button"].is_null() { r = crate::guide::controller(&["set", &value_text(&a["button"]), &value_text(&a["to"])])?; }
                if !a["target"].is_null() { r = crate::guide::controller(&["type", &value_text(&a["target"])])?; }
                if r.is_null() { crate::guide::controller(&["status"]) } else { Ok(r) }
            }
            "controller.preset" => crate::guide::controller(&["preset", &req("name")?]),
            "controller.reset" => crate::guide::controller(&["reset"]),
            // Is Handheld Daemon running, its controllers and power limit (Settings › Controls shows the way in).
            "hhd.status" => Ok(crate::hhd::status()),
            // Settings › Controls › Handheld Daemon: HHD's whole settings tree with its state, the state alone (the
            // page keeps current while it's open), and changes to any setting it shows (checked against the tree).
            // hhd.set answers with hhd.status, or with the new state when the page asks for it ("state": true).
            "hhd.settings" => crate::hhd::settings(),
            "hhd.state" => crate::hhd::state(),
            // From Quick Access › Performance while a game is in front ("game": true), a power setting is kept for
            // that game too (see game_power_set).
            "hhd.set" => {
                let key = req("key")?;
                if a["game"] == true && crate::hhd::game_power_key(&key) {
                    if let Some(id) = self.library.running_id() { return self.game_power_set(&id, &key, &a["value"]); }
                }
                if a["state"] == true { crate::hhd::write(&key, &a["value"]) } else { crate::hhd::set(&key, &a["value"]) }
            }
            // Settings › Updates: the new Hearth OS (on Anatase) as a clean install that keeps your games, from the
            // Bazzite-based one; and on the new one, Settings › System: the files put aside (~/.old-hearth-<date>).
            "newos.status" => self.later(|| Ok(crate::newos::status())),
            "newos.move" => crate::newos::start(self.emit.clone(), a["edition"].as_str().unwrap_or("anatase"), a["testing"] == true),
            "oldfiles.status" => Ok(crate::newos::old_files_status()),
            "oldfiles.delete" => { let name = req("name")?; self.later(move || crate::newos::delete(&name)) }
            "oldfiles.restore" => crate::newos::restore(&req("name")?, &req("item")?),
            // Web browsers: Hearth Browser (Hearth's own: Firefox, hearth_browser.rs) and Aloha (optional, downloaded
            // on the device)
            "browser.status" => Ok(self.browser_status()),
            "browser.act" => { self.browser_act(&req("action")?)?; Ok(self.browser_status()) }
            "browser.hearth" => {
                if a["action"] == "open" {
                    let url = a["url"].as_str().filter(|u| crate::beacon::valid_link(u));
                    crate::hearth_browser::open(url)?;
                    return Ok(json!(true));
                }
                if a["action"] == "default" { system::run("xdg-settings", &["set", "default-web-browser", "hearth-browser.desktop"]).map_err(|_| "Couldn't make it the default".to_string())?; }
                Ok(self.browser_status())
            }
            // Claude (Anthropic): Claude Desktop from Anthropic's repository and Claude Code with Anthropic's installer,
            // both put in your home folder by desktop/shell/hearth-claude (also Desktop Mode's Settings › Claude).
            "claude.status" => Ok(self.claude_status()),
            "claude.act" => { self.claude_act(&req("action")?)?; Ok(self.claude_status()) }
            // "Make Hearth my shell" (a Hearth session on the login screen)
            "shell.status" => Ok(shell_status()),
            "shell.register" => { shell_install(false)?; Ok(shell_status()) }
            "shell.unregister" => { let _ = std::fs::remove_file(paths::config_dir().join("app-path")); if std::path::Path::new(SESSION_FILE).exists() { shell_install(true)?; } Ok(shell_status()) }
            "shell.returnToDesktop" => {
                if crate::power::action("desktop", crate::gamescope::in_session())? {
                    let emit = self.emit.clone();
                    std::thread::spawn(move || { std::thread::sleep(std::time::Duration::from_millis(50)); emit(json!({"type": "quit"})); });
                }
                Ok(Value::Null)
            }
            "app.restart" => { (self.emit)(json!({"type": "restart"})); Ok(Value::Null) }
            "app.exportLogs" => export_logs(&self.version),
            "logs.save" => crate::logs::save(&self.version).map(|(p, _)| json!({"path": p.to_string_lossy()})),
            "logs.send" => crate::logs::send(&self.version),
            "logs.sendToDeveloper" => crate::logs::send_to_developer_note(&self.version, edition(), a["note"].as_str().unwrap_or("")),
            "game.resetPrefix" => {
                let id = req("id")?;
                let g = self.library.games.get(&id).cloned().ok_or("That game isn't in the library anymore.")?;
                if !(g.can_uninstall && (g.source == "epic" || g.source == "gog")) { return Err("This game doesn't have a Hearth Wine prefix.".into()); }
                if self.library.running_id().as_deref() == Some(id.as_str()) { return Err("Close the game first.".into()); }
                crate::stores::remove_prefix(g.source, id.split_once(':').map(|x| x.1).unwrap_or(""));
                Ok(json!({"ok": true}))
            }
            "profile.pickImage" => {
                let kind = req("kind")?;
                match (self.pick_file)(if kind == "avatar" { "Choose a profile picture" } else { "Choose a background picture" }) {
                    Some(f) => crate::profile::set_image(&mut self.library.store, &kind, &f),
                    None => Ok(Value::Null),
                }
            }
            "desktop.wallpaper" => Ok(json!({"wallpaper": crate::admin::desktop_wallpaper()})),
            "desktop.setWallpaper" => {
                if a["value"] == "profile" {
                    let f = crate::profile::image("background").ok_or("Choose a background picture in your profile first")?;
                    return Ok(json!({"wallpaper": crate::admin::set_desktop_wallpaper("", Some(&f))?}));
                }
                Ok(json!({"wallpaper": crate::admin::set_desktop_wallpaper(a["value"].as_str().unwrap_or(""), None)?}))
            }
            // Profiles: switching closes Steam, signs it in as theirs, puts their files out and restarts Hearth.
            "profiles.switch" => {
                let id = req("id")?;
                let to = self.profiles.find(&id).ok_or("No such profile")?;
                if self.library.running_id().is_some() { return Err("Close the game first".into()); }
                if self.profiles.restricted(&self.profiles.active()) && id != self.profiles.active() { self.need_pin()?; }
                log(&format!("profiles: switching to {}", to["name"]));
                (self.emit)(Bridge::event("profiles.switching", json!({"name": to["name"]})));
                crate::profiles::stop_steam();
                // The Steam account this profile used stays its own (Steam writes its files as it closes, so read after).
                let leaving = self.profiles.active();
                if let Some(acc) = crate::profiles::keep_steam_account(&self.profiles.list(), &leaving, crate::profiles::current_steam_account().as_deref()) {
                    if self.profiles.update(&leaving, &json!({"steam": acc})).is_ok() { log(&format!("profiles: {leaving} keeps Steam account {acc}")); }
                }
                // Theirs, or Steam's own sign-in for a profile that hasn't signed in yet (that account becomes theirs).
                crate::profiles::set_steam_account(to["steam"].as_str().unwrap_or(""));
                self.profiles.switch_to(&id)?;
                let _ = std::fs::write(picked_file(), "1");
                (self.emit)(json!({"type": "restart"}));
                Ok(json!(true))
            }
            "profiles.addSteam" => {
                let id = req("id")?;
                self.need_pin()?;
                let before: Vec<String> = crate::profiles::known_steam_accounts().iter().filter_map(|a| a["account"].as_str().map(str::to_lowercase)).collect();
                crate::profiles::stop_steam();
                crate::profiles::set_steam_account("");
                let _ = crate::steam::spawn(&self.library.home, "steam://open/main");
                // The account that signs in (within 10 minutes) is linked to the profile.
                std::thread::spawn(move || {
                    for _ in 0..300 {
                        std::thread::sleep(std::time::Duration::from_secs(2));
                        let now = crate::profiles::known_steam_accounts().into_iter().find(|a| a["account"].as_str().map(|x| !before.contains(&x.to_lowercase())).unwrap_or(false));
                        if let Some(acc) = now {
                            let mut ps = crate::profiles::Profiles::open(&paths::data_dir());
                            if ps.update(&id, &json!({"steam": acc["account"]})).is_ok() { log(&format!("profiles: linked Steam account {}", acc["account"])); }
                            crate::stores::notify(Bridge::event("profiles.changed", json!({})));
                            return;
                        }
                    }
                });
                Ok(json!(true))
            }
            // Hearth's own installer (live USB)
            "osinstall.status" => Ok(self.osinstall.status()),
            "osinstall.disks" => self.osinstall.disks(),
            "osinstall.check" => self.osinstall.check(a["part"].as_str(), a["disk"].as_str()),
            "osinstall.start" => self.osinstall.start(&a["plan"]),
            "osinstall.restart" => Ok(json!(self.osinstall.restart())),
            "osinstall.useMantel" => Ok(json!(crate::osinstall::handoff(true))),
            // Updates to the app itself
            "update.status" => Ok(self.app_updates.status(self.auto_update())),
            "update.check" => Ok(self.app_updates.check(self.auto_update())),
            "update.download" => Ok(self.app_updates.download(self.auto_update())),
            "update.install" => { self.app_updates.install()?; (self.emit)(json!({"type": "restart"})); Ok(Value::Null) }
            "logs.setAutoReport" => { let on = a["on"] == true; io(self.library.store.update(|d| { d.insert("autoReportStuck".into(), json!(on)); }))?; crate::watchdog::set_auto_send(on); Ok(json!(on)) }
            "update.setAuto" => { io(self.library.store.update(|d| { d.insert("autoUpdateCheck".into(), json!(a["on"] == true)); }))?; Ok(self.app_updates.status(self.auto_update())) }
            // Settings › Display
            "display.get" => Ok(self.display_get()),
            "display.setMode" => {
                let (w, h, hz) = (a["w"].as_f64().unwrap_or(0.0).round() as u32, a["h"].as_f64().unwrap_or(0.0).round() as u32, a["hz"].as_f64().unwrap_or(0.0));
                if w == 0 || h == 0 || hz <= 0.0 { return Err("Pick a resolution and refresh rate".into()); }
                if crate::display::set_mode(w, h, hz, self.zoom_pct())? {
                    // New gamescope options take effect when Game Mode starts again.
                    let mut d = self.display_get(); d["restarting"] = json!(true);
                    self.restart_game_mode();
                    return Ok(d);
                }
                Ok(self.display_get())
            }
            // A new Game Mode mode on trial: keep it, or go back now (Game Mode restarts).
            "display.keep" => { crate::display::keep(); Ok(self.display_get()) }
            "display.revert" => {
                if crate::display::revert() { self.restart_game_mode(); }
                Ok(self.display_get())
            }
            "display.setScale" => {
                let pct = a["v"].as_f64().unwrap_or(0.0).round() as u32;
                if let Some(z) = crate::display::set_scale(pct, self.zoom_pct())? {
                    io(self.library.store.update(|d| { d.insert("uiScale".into(), json!(z)); }))?;
                    (self.zoom)(z);
                }
                Ok(self.display_get())
            }
            "game.info" => {
                let id = req("id")?;
                let g = self.library.games.get(&id).cloned().ok_or("That game isn't in the library anymore.")?;
                let home = paths::home();
                let ach = if let Some(app) = id.strip_prefix("steam:") { crate::gameinfo::achievements(&home, app) } else { Value::Null };
                Ok(json!({"protondb": crate::gameinfo::protondb(&id, &g.title, g.source), "achievements": ach}))
            }
            "game.options.get" => self.game_options(&req("id")?),
            // Beacon (beacon.rs): the mod a game starts with (saved in Beacon), and Beacon's own window.
            "beacon.selectMod" => {
                let m = a["mod"].as_str().filter(|m| !m.is_empty() && *m != "@none");
                self.library.beacon_select_mod(&req("id")?, m)
            }
            "beacon.open" => self.library.beacon_open().map(|_| json!({"ok": true})),
            // Beacon's screens in Hearth (Game setup, Mods, the Beacon tab): beacon.rs checks everything and runs Beacon
            // off this thread.
            m if m.starts_with("beacon.") => { let id = self.cur_id.clone(); crate::beacon::bridge_call(self, m, a, id) }
            "game.options.set" => { let id = req("id")?; crate::game_options::set(&mut self.library.store, &id, &a["options"])?; self.game_options(&id) }
            "game.launch" => {
                let id = req("id")?;
                // Parental controls: allowed games and play time a day.
                let pc = self.profiles.can_play(Some(&id), chrono::Local::now(), 0.0);
                if pc["ok"] != true { return Err(pc["reason"].as_str().unwrap_or("Not now").to_string()); }
                // Beacon's games: "mod" plays with that mod this time ("@none": without mods).
                let m = a["mod"].as_str().map(|m| if m == "@none" { "" } else { m });
                let r = self.library.launch_with(&id, m)?;
                if let Some(url) = r["stream"].as_str() { (self.stream)(Some((url, r["title"].as_str().unwrap_or("Xbox Cloud Gaming")))); }
                (self.emit)(Bridge::event("quickResume", self.library.quick_state()));
                if r["resumed"] == true { (self.host)("resumeGame", &Value::Null); }
                Ok(json!({"tracked": r["tracked"], "resumed": r["resumed"] == true, "expired": r["expired"]}))
            }
            // Quick Resume: games put aside stay frozen in memory and carry on where they were
            "game.quickResume" => {
                let id = req("id")?;
                let pc = self.profiles.can_play(Some(&id), chrono::Local::now(), 0.0);
                if pc["ok"] != true { return Err(pc["reason"].as_str().unwrap_or("Not now").to_string()); }
                self.library.quick_resume(&id)?;
                (self.host)("resumeGame", &Value::Null);
                let st = self.library.quick_state();
                (self.emit)(Bridge::event("quickResume", st.clone()));
                Ok(st)
            }
            "game.putAside" => {
                self.library.freeze_running()?;
                (self.host)("comeBack", &Value::Null);
                let st = self.library.quick_state();
                (self.emit)(Bridge::event("quickResume", st.clone()));
                Ok(st)
            }
            "game.quickClose" => {
                let dto = self.library.close_frozen(&req("id")?)?;
                let st = self.library.quick_state();
                (self.emit)(Bridge::event("quickResume", st.clone()));
                Ok(json!({"state": st, "game": dto}))
            }
            "quickResume.set" => {
                let on = a["on"] != false;
                io(self.library.store.update(|d| { d.insert("quickResume".into(), json!(on)); }))?;
                Ok(self.library.quick_state())
            }
            // Turned off with the games kept (hibernation: memory saved to the drive), the closest Linux gets to
            // Xbox's Quick Resume surviving a power-off.
            "power.hibernateStatus" => Ok(crate::power::hibernate_status()),
            "power.hibernateSetup" => { crate::admin::run(&["hibernate-setup"], None, &mut |_| {})?; Ok(crate::power::hibernate_status()) }
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
                io(self.library.store.update(|d| { crate::store::array_in(d, "manual").push(m); }))?;
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
            // Scanning (up to 30 s) and connecting (up to 45 s) run off the bridge and answer when done: under it, every
            // other request waited, and the fail-safe stopped nmcli at 15 s (slow networks never connected).
            "wifi.scan" => { let t = system::take_turn(&system::WIFI_BUSY).ok_or(WIFI_BUSY_TEXT)?; self.later(move || { let _t = t; system::wifi::scan() }) }
            "wifi.connect" => {
                let (ssid, pw) = (req("ssid")?, a["password"].as_str().filter(|p| !p.is_empty()).map(str::to_string));
                let t = system::take_turn(&system::WIFI_BUSY).ok_or(WIFI_BUSY_TEXT)?;
                self.later(move || { let _t = t; Ok(system::wifi::connect(&ssid, pw.as_deref())) })
            }
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
            "xodus.status" => Ok(self.xodus.status(true)),
            "xodus.install" => {
                if let Some(f) = a["folder"].as_str() { crate::xodus::set_folder(f)?; }
                Ok(self.xodus.install(self.emit.clone()))
            }
            "xodus.places" => Ok(json!(crate::xodus::places())),
            "xodus.setFolder" => { crate::xodus::set_folder(a["folder"].as_str().unwrap_or(""))?; Ok(self.xodus.status(false)) }
            "xodus.signInDesktop" => { crate::xodus::open_in_desktop()?; Ok(json!({})) }
            "xodus.remove" => { crate::xodus::remove()?; self.library.scan(); Ok(self.xodus.status(false)) }
            // Which engine draws Hearth: Chromium (the Electron window app; automatic on Nvidia) or WebKit (renderer.rs).
            "display.renderer" => {
                if let Some(v) = a["set"].as_str() { crate::renderer::set(v)?; }
                Ok(crate::renderer::status())
            }
            // Nvidia: how Hearth draws (fast, the default on newer drivers, or compatible if fast draws a black window).
            "display.nvidiaDrawing" => {
                let f = crate::paths::config_dir().join("nvidia-drawing");
                if let Some(v) = a["set"].as_str() {
                    if !["fast", "compatible", "auto"].contains(&v) { return Err("Unknown choice".into()); }
                    let _ = std::fs::create_dir_all(crate::paths::config_dir());
                    if v == "auto" { let _ = std::fs::remove_file(&f); } else { std::fs::write(&f, v).map_err(|e| e.to_string())?; }
                }
                let pick = std::fs::read_to_string(&f).unwrap_or_default().trim().to_string();
                Ok(json!({"pick": if pick.is_empty() { "auto".to_string() } else { pick }, "fast": std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none()}))
            }
            // Couch Co-op: several players on this screen, each with their own controller and copy of the game.
            "coop.check" => crate::multiplay::ask(crate::multiplay::COOP, &["check"]),
            "coop.pads" => crate::multiplay::ask(crate::multiplay::COOP, &["pads"]),
            "coop.games" => crate::multiplay::ask(crate::multiplay::COOP, &["games"]),
            "coop.profiles" => crate::multiplay::ask(crate::multiplay::COOP, &["profiles"]),
            "coop.setName" => {
                let seat = a["seat"].as_u64().unwrap_or(0).to_string();
                crate::multiplay::ask(crate::multiplay::COOP, &["profile-set", "--seat", &seat, "--name", a["name"].as_str().unwrap_or("")])
            }
            "coop.status" => crate::multiplay::ask(crate::multiplay::COOP, &["status"]),
            // "Player N, press A": the next controller with a button pressed (as the event coop.pad.done).
            "coop.waitButton" => {
                let exclude: Vec<String> = a["exclude"].as_array().map(|l| l.iter().filter_map(|v| v.as_str().map(str::to_string)).collect()).unwrap_or_default();
                let mut args = vec!["wait-button".to_string(), "--timeout".into(), a["timeout"].as_u64().unwrap_or(20).to_string()];
                if !exclude.is_empty() { args.push("--exclude".into()); args.push(exclude.join(",")); }
                crate::multiplay::spawn_job("coop.pad", crate::multiplay::COOP, args, self.emit.clone(), None);
                Ok(Value::Null)
            }
            "coop.start" => {
                let pads: Vec<String> = a["pads"].as_array().map(|l| l.iter().filter_map(|v| v.as_str().map(str::to_string)).collect()).unwrap_or_default();
                if pads.len() < 2 { return Err("Couch Co-op needs at least two players.".into()); }
                let args = vec!["start".to_string(), "--game".into(), req("game")?, "--players".into(), pads.len().to_string(),
                                "--pads".into(), pads.join(","), "--layout".into(), a["layout"].as_str().unwrap_or("auto").to_string()];
                let host = self.host.clone();
                crate::multiplay::spawn_job("coop.start", crate::multiplay::COOP, args, self.emit.clone(), Some(Box::new(move |v: &Value| {
                    let pid = v["labwcPid"].as_u64().or(v["containerPid"].as_u64()).unwrap_or(0);
                    host("showPid", &json!({"pid": pid}));
                })));
                Ok(Value::Null)
            }
            // Back to the split screen from Hearth.
            "coop.show" => {
                let st = crate::multiplay::ask(crate::multiplay::COOP, &["status"])?;
                if st["running"] != true { return Err("Couch Co-op isn't running.".into()); }
                (self.host)("showPid", &json!({"pid": st["labwcPid"].as_u64().or(st["containerPid"].as_u64()).unwrap_or(0)}));
                Ok(st)
            }
            "coop.stop" => { let r = crate::multiplay::ask(crate::multiplay::COOP, &["stop"])?; (self.host)("comeBack", &Value::Null); Ok(r) }
            // Multi-seat: a separate Game Mode on each screen, for other people with their own accounts.
            "seats.scan" => crate::multiplay::ask(crate::multiplay::SEATS, &["scan"]),
            "seats.plan" => crate::multiplay::ask(crate::multiplay::SEATS, &["plan"]),
            "seats.status" => crate::multiplay::ask(crate::multiplay::SEATS, &["status"]),
            "seats.start" => crate::multiplay::ask(crate::multiplay::SEATS, &["start"]),
            // Game Mode starts again to share the screens out (or to stop sharing them): the session's next round
            // (signing out instead left you at the sign-in screen without automatic sign-in).
            "seats.restartGameMode" => { self.restart_game_mode(); Ok(Value::Null) }
            "seats.stop" => crate::multiplay::ask(crate::multiplay::SEATS, &["stop"]),
            "seats.assignPad" => {
                let seat = a["seat"].as_u64().unwrap_or(1).to_string();
                crate::multiplay::ask(crate::multiplay::SEATS, &["assign-pad", "--pad", &req("pad")?, "--seat", &seat])
            }
            "seats.setup" => {
                let mut args = vec!["setup".to_string(), "--seats".into(), a["layout"].to_string()];
                if a["start"] == true { args.push("--start".into()); }
                crate::multiplay::spawn_job("seats.setup", crate::multiplay::SEATS, args, self.emit.clone(), None);
                Ok(Value::Null)
            }
            "seats.remove" => {
                let mut args = vec!["remove".to_string()];
                if a["deleteUsers"] == true { args.push("--delete-users".into()); }
                crate::multiplay::spawn_job("seats.remove", crate::multiplay::SEATS, args, self.emit.clone(), None);
                Ok(Value::Null)
            }
            // "Player N, press A on your controller" (as the event seats.pad.done {pad}).
            "seats.waitButton" => {
                let exclude: Vec<String> = a["exclude"].as_array().map(|l| l.iter().filter_map(|v| v.as_str().map(str::to_string)).collect()).unwrap_or_default();
                let mut args = vec!["wait-button".to_string(), "--timeout".into(), a["timeout"].as_u64().unwrap_or(20).clamp(1, 120).to_string()];
                if !exclude.is_empty() { args.push("--exclude".into()); args.push(exclude.join(",")); }
                crate::multiplay::spawn_job("seats.pad", crate::multiplay::SEATS, args, self.emit.clone(), None);
                Ok(Value::Null)
            }
            // Streamed players (Beta Experimental): their Sunshine's address, paired devices and Moonlight's PIN.
            "seats.streamInfo" => crate::multiplay::ask(crate::multiplay::SEATS, &["stream-info"]),
            "seats.pair" => {
                // Sunshine holds the answer until Moonlight finishes pairing, so this runs as a job (seats.pair.done).
                let mut args = vec!["stream-pair".to_string(), "--seat".into(), a["seat"].as_u64().unwrap_or(0).to_string(), "--pin".into(), req("pin")?];
                if let Some(n) = a["name"].as_str().filter(|n| !n.is_empty()) { args.push("--name".into()); args.push(n.to_string()); }
                crate::multiplay::spawn_job("seats.pair", crate::multiplay::SEATS, args, self.emit.clone(), None);
                Ok(Value::Null)
            }
            "seats.unpair" => {
                let seat = a["seat"].as_u64().unwrap_or(0).to_string();
                crate::multiplay::ask(crate::multiplay::SEATS, &["stream-unpair", "--seat", &seat, "--uuid", &req("uuid")?])
            }
            // Split screen sound: a player's volume, or only player 1's sound.
            "seats.audio" => {
                let mut args: Vec<String> = vec!["audio".into()];
                if let (Some(seat), Some(vol)) = (a["seat"].as_u64(), a["volume"].as_u64()) {
                    args.extend(["--seat".to_string(), seat.to_string(), "--volume".to_string(), vol.min(100).to_string()]);
                }
                if let Some(on) = a["onlyP1"].as_bool() { args.extend(["--only-p1".to_string(), if on { "on" } else { "off" }.to_string()]); }
                let refs: Vec<&str> = args.iter().map(String::as_str).collect();
                crate::multiplay::ask(crate::multiplay::SEATS, &refs)
            }
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
                s["quickResume"] = self.library.quick_state();
                // The game in front, and whether it keeps its own power settings (Quick Access says so).
                s["gamePower"] = match self.library.running_id() {
                    Some(id) => { let o = crate::game_options::get(&self.library.store, &id);
                        json!({"id": id, "saved": Self::has_game_power(&o), "label": o["powerLabel"], "tdp": o["tdp"]}) }
                    None => Value::Null,
                };
                // Quick Access shows Handheld Daemon's own power settings when it runs ("hhd": true): Hearth's power
                // mode and limit (hearth-power, a helper run every second otherwise) aren't needed then.
                let st = if a["hhd"] == true { json!({"mode": {"supported": false}, "tdp": {"supported": false}}) } else { crate::power::status_cached() };
                s["battery"] = system::battery(std::path::Path::new("/sys/class/power_supply"));
                s["runningId"] = json!(self.library.running_id()); s["runningSince"] = json!(self.library.running_since());
                s["canFpsLimit"] = json!(crate::gamescope::can_fps_limit()); s["fpsLimit"] = json!(self.library.store.data["fpsLimit"].as_u64().unwrap_or(0));
                s["canOverlay"] = json!(crate::gamescope::perf_overlay_supported()); s["perfOverlay"] = json!(self.library.store.data["perfOverlay"].as_u64().unwrap_or(0));
                s["powerMode"] = if st["mode"]["supported"] == true { st["mode"]["mode"].clone() } else { Value::Null };
                s["powerModes"] = st["mode"]["available"].clone(); s["tdp"] = st["tdp"].clone();
                Ok(s)
            }
            "system.info" => Ok(crate::sysinfo::info(std::path::Path::new("/"))),
            // hearth-power can take seconds: what it said last answers at once, and changing the mode runs off the
            // bridge (every other request waited for it, and the fail-safe saw Hearth as stuck).
            "power.get" => Ok(power_get(crate::power::status_cached())),
            "power.setMode" => {
                let mode = req("mode")?;
                self.later(move || { crate::power::set_mode(&mode)?; Ok(power_get(crate::power::status())) })
            }
            "perf.setTdp" => { let w = a["watts"].as_f64().unwrap_or(0.0).round().max(0.0) as u32; self.later(move || crate::power::set_tdp(w)) }
            // Quick Access › Performance: the game in front goes back to the usual power settings.
            "perf.gamePowerForget" => {
                let id = req("id")?;
                crate::game_options::set(&mut self.library.store, &id, &json!({"tdp": 0, "hhdPower": {}, "powerLabel": ""}))?;
                if self.library.running_id().as_deref() == Some(id.as_str()) { self.power_restore(); }
                Ok(crate::hhd::state().unwrap_or(json!({"running": false})))
            }
            "bt.paired" => Ok(system::bt_devices::paired()),
            // Pairing waits for the controller (up to a minute): it runs off the bridge and answers when done. Under
            // the bridge, every other request waited, and the fail-safe stopped bluetoothctl at 15 s mid-pairing.
            "bt.pair" => {
                let id = req("id")?;
                let t = system::take_turn(&system::BT_BUSY).ok_or("A Bluetooth device is already pairing. Try again when it's done.")?;
                self.later(move || { let _t = t; Ok(system::bt_devices::pair(&id)) })
            }
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
            // Hearth Input: controller layouts for games without controller support (input_map.rs)
            m if m.starts_with("input.") => crate::input_map::call(m, a, self.library.running_id()),
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
        let own_proton = (g.source == "epic" || g.source == "gog") && g.can_uninstall;   // installed by Hearth, started with Proton
        let kind = match g.source { "steam" => "steam", "emu" => "emu", "beacon" => "beacon", "manual" if g.exe.as_deref().map(|e| e.to_lowercase().ends_with(".exe")).unwrap_or(false) => "proton", "manual" => "native", _ if own_proton => "proton", _ => "external" };
        Ok(json!({"id": id, "kind": kind, "options": crate::game_options::get(&self.library.store, id),
                  "protonChoices": if kind == "proton" { crate::game_options::proton_choices(&paths::home()) } else { vec![] },
                  "gamemode": system::has("gamemoderun"), "canResetPrefix": own_proton}))
    }
    /// Runs `f` on its own thread and answers the request in hand with its result then (to the window that asked).
    fn later(&self, f: impl FnOnce() -> Result<Value, String> + Send + 'static) -> Result<Value, String> {
        crate::space::answer_later_to(self.cur_id.clone(), Some(self.cur_label.clone()), f);
        Ok(json!(DEFERRED))
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
        json!({"active": active, "askAtStart": self.profiles.data["askAtStart"] == true, "picked": picked(),
               "hasPin": self.profiles.has_pin(), "unlocked": self.profiles.has_pin() && std::time::Instant::now() < self.pin_until,
               "restricted": self.profiles.restricted(&active), "left": self.profiles.can_play(None, now, 0.0)["minutesLeft"],
               "list": list, "steamAccounts": crate::profiles::steam_accounts(&steam_users)})
    }
    /// First-start setup: only on Hearth, the OS, and only for a fresh install. An install already in use
    /// before setup existed (games played, accounts, the import offer) counts as set up.
    pub fn setup_done(&mut self) -> bool {
        let d = &self.library.store.data;
        if !d["setupDone"].is_null() && d["setupDone"] != false { return true; }
        if !crate::admin::can_admin() { return true; }
        let used = d["stats"].as_object().map(|o| !o.is_empty()).unwrap_or(false) || d["manual"].as_array().map(|a| !a.is_empty()).unwrap_or(false)
            || d["importOffered"] == true || d["profile"].is_object();
        if used { let _ = self.library.store.update(|x| { x.insert("setupDone".into(), json!(now_ms())); }); }
        used
    }
    fn overlay_settings(&self) -> Value {
        json!({"holdView": self.library.store.data["holdView"] == true, "gameBarButton": false, "hotkey": "Ctrl+Alt+Q", "guide": self.guide.status()})
    }
    /// Open apps for Quick Access: Hearth first, then every window, then Steam if it runs without a window.
    fn apps_list(&self) -> Value {
        let game: Vec<u64> = self.library.game_pids().into_iter().map(u64::from).collect();
        let me = std::process::id() as u64;
        let own: Vec<u64> = ["Hearth", "Hearth Overlay"].iter().filter_map(|t| crate::gamescope::window_named(t)).collect();
        let mut exclude = vec![me]; exclude.extend(own.iter().copied());
        let list: Vec<Value> = crate::gamescope::apps(&exclude).into_iter().map(|mut a| { let pid = a["pid"].as_u64().unwrap_or(0); a["hearth"] = json!(false); a["game"] = json!(game.contains(&pid)); a }).collect();
        let front = !list.iter().any(|a| a["focused"] == true);
        let mut out = vec![json!({"id": own.first().copied().unwrap_or(0), "pid": me, "title": "Hearth", "app": "hearth", "hearth": true, "game": false, "focused": front})];
        let steam_listed = list.iter().any(|a| a["app"].as_str().unwrap_or("").to_lowercase().contains("steam") || a["title"].as_str().map(|t| t.eq_ignore_ascii_case("steam")).unwrap_or(false));
        out.extend(list);
        let steam_up = std::process::Command::new("pgrep").args(["-x", "steam"]).output().map(|o| o.status.success()).unwrap_or(false);
        if steam_up && !steam_listed { out.push(json!({"id": 0, "pid": 0, "title": "Steam", "app": "steam", "steam": true, "game": false, "focused": false})); }
        json!(out)
    }

    /// After the library's events (every couple of seconds): parental controls count play time, and a game's own
    /// frame-rate cap and power limit go on while it plays.
    pub fn after_events(&mut self, events: &[Value]) -> Vec<Value> {
        let mut out = Vec::new();
        for e in events {
            match e["name"].as_str().unwrap_or("") {
                // Steam's window is needed (its install window after a silent install didn't start): in front.
                "steam.window" => (self.host)("showSteam", &json!({"wait": 30})),
                "game.started" => {
                    self.parental_since = now_ms(); self.parental_warned = false;
                    out.extend(self.parental_tick());
                    let id = e["data"]["id"].as_str().unwrap_or("").to_string();
                    let o = crate::game_options::get(&self.library.store, &id);
                    let fps = o["fpsLimit"].as_u64().unwrap_or(0);
                    if fps > 0 {
                        let back = self.library.store.data["fpsLimit"].as_u64().unwrap_or(0);
                        if crate::gamescope::in_session() { let _ = crate::gamescope::set_root("GAMESCOPE_FPS_LIMIT", fps); }
                        log(&format!("game settings: {id} fps {fps}"));
                        self.per_game = Some(back);
                    }
                }
                "game.exited" => {
                    if self.parental_since > 0 {
                        let mins = now_ms().saturating_sub(self.parental_since) as f64 / 60000.0;   // the clock set back (time sync) wrapped to years
                        let active = self.profiles.active();
                        let _ = self.profiles.add_played(&active, mins, chrono::Local::now());
                    }
                    self.parental_since = 0;
                    if let Some(fps) = self.per_game.take() {
                        if crate::gamescope::in_session() { let _ = crate::gamescope::set_root("GAMESCOPE_FPS_LIMIT", fps); }
                    }
                }
                _ => {}
            }
        }
        // A game's own power settings go on while it's in front (started, or back from Quick Resume), and what
        // was there before comes back when it leaves the front.
        let front = self.library.running_id();
        if front != self.power_seen {
            self.power_seen = front.clone();
            self.game_power_front(front.as_deref());
        }
        out
    }

    /// Does this game keep its own power settings (a limit, or Handheld Daemon's settings)?
    fn has_game_power(o: &Value) -> bool {
        o["tdp"].as_u64().unwrap_or(0) > 0 || o["hhdPower"].as_object().map(|m| !m.is_empty()).unwrap_or(false)
    }

    /// A game came to the front (or none is): its power settings on, or the usual ones back. With Handheld
    /// Daemon running they go through HHD (it owns the power limit; anything else would fight it); elsewhere the
    /// limit goes through hearth-power, as before.
    fn game_power_front(&mut self, id: Option<&str>) {
        let saved = id.map(|i| crate::game_options::get(&self.library.store, i)).filter(Self::has_game_power);
        let Some(o) = saved else { return self.power_restore() };
        let id = id.unwrap_or("");
        let st = crate::hhd::current_state();
        if !st.is_null() && st["tdp"].is_object() {
            let changes = crate::hhd::game_changes(&o["hhdPower"], o["tdp"].as_u64().unwrap_or(0), &st);
            if changes.is_empty() { return; }
            if self.power_back.is_none() { self.power_back = Some(json!({"hhd": crate::hhd::game_power(&st)})); }
            match crate::hhd::write_many(&changes, true) {
                Ok((_, left)) => log(&format!("game power: {id} through Handheld Daemon {}{}", Value::Object(changes.clone()),
                    if left.is_empty() { String::new() } else { format!(" (left out: {})", left.join("; ")) })),
                Err(e) => log(&format!("game power: {id}: {e}")),
            }
            return;
        }
        let w = o["tdp"].as_u64().unwrap_or(0) as u32;
        if w == 0 { return; }
        let st = crate::power::status();
        if st["tdp"]["supported"] != true { return; }
        if self.power_back.is_none() { self.power_back = st["tdp"]["watts"].as_u64().map(|w| json!({"watts": w})); }
        match crate::power::set_tdp(w) { Ok(_) => log(&format!("game power: {id} {w} W")), Err(e) => log(&format!("game power: {id}: {e}")) }
    }

    /// The power settings from before a game's own went on, back (nothing to do when none are kept).
    fn power_restore(&mut self) {
        let Some(back) = self.power_back.take() else { return };
        if let Some(m) = back["hhd"].as_object() {
            if m.is_empty() { return; }
            match crate::hhd::write_many(m, true) {
                Ok(_) => log("game power: the usual settings are back (Handheld Daemon)"),
                Err(e) => log(&format!("game power: couldn't put the usual settings back: {e}")),
            }
        } else if let Some(w) = back["watts"].as_u64() {
            match crate::power::set_tdp(w as u32) { Ok(_) => log(&format!("game power: back to {w} W")), Err(e) => log(&format!("game power: {e}")) }
        }
    }

    /// A Handheld Daemon power setting changed in Quick Access while a game is in front: HHD gets it, and the game
    /// keeps the power settings now in effect (mode, and the custom limit and boost), which come back whenever it's
    /// in front again. What was there before the game had its own is kept, to put back when it leaves the front.
    /// Answers like hhd.set with "state": {running, version, state}, plus the game's kept settings.
    fn game_power_set(&mut self, id: &str, key: &str, value: &Value) -> Result<Value, String> {
        if self.power_back.is_none() {
            let st = crate::hhd::current_state();
            if !st.is_null() { self.power_back = Some(json!({"hhd": crate::hhd::game_power(&st)})); }
        }
        let r = crate::hhd::write(key, value)?;
        let st = &r["state"];
        let kept = crate::hhd::game_power(st);
        let watts = crate::hhd::tdp(st)["watts"].as_f64().map(|w| w.round().max(0.0) as u64).unwrap_or(0);
        let label = crate::hhd::power_label(&crate::hhd::settings_tree(), st);
        crate::game_options::set(&mut self.library.store, id, &json!({"hhdPower": kept, "tdp": watts, "powerLabel": label}))?;
        self.power_seen = Some(id.to_string());
        log(&format!("game power: {id} keeps {label}"));
        let mut out = r;
        out["gamePower"] = json!({"id": id, "saved": true, "label": label, "tdp": watts});
        Ok(out)
    }
    /// Every 20 seconds while a game plays, for profiles with a daily limit: a warning 5 minutes before the
    /// end, then the game closes.
    pub fn parental_tick(&mut self) -> Vec<Value> {
        if self.parental_since == 0 || !self.profiles.restricted(&self.profiles.active()) { return vec![]; }
        let session = now_ms().saturating_sub(self.parental_since) as f64 / 60000.0;
        let r = self.profiles.can_play(self.library.running_id().as_deref(), chrono::Local::now(), session);
        let Some(left) = r["minutesLeft"].as_f64() else { return vec![] };
        if r["ok"] != true {
            log("parental: time is up");
            self.parental_since = 0;
            let emit = self.emit.clone();
            let lib_close = self.host.clone();
            std::thread::spawn(move || { std::thread::sleep(std::time::Duration::from_secs(5)); lib_close("closeGame", &Value::Null); let _ = emit; });
            return vec![Bridge::event("parental.timeup", json!({})), Bridge::event("osd", json!({"kind": "msg", "icon": "clock", "text": "Play time's up for today. The game is closing.", "ms": 6000}))];
        }
        if left <= 5.0 && !self.parental_warned {
            self.parental_warned = true;
            let m = left.round() as u64;
            return vec![Bridge::event("osd", json!({"kind": "msg", "icon": "clock", "text": format!("{m} minute{} of play time left today. Time to find a save point!", if m == 1 { "" } else { "s" }), "ms": 7000}))];
        }
        vec![]
    }

    /// Aloha Browser, through desktop/shell/hearth-browser; and Hearth Browser (`hearth`).
    fn browser_status(&self) -> Value {
        let helper = paths::desktop_dir().join("shell/hearth-browser");
        let job = self.browser.lock().unwrap().clone();
        let mut s = if helper.exists() {
            let last = if job["state"] == "idle" { helper_run(&helper, &["status"], &mut |_, _| {}).unwrap_or(Value::Null) } else { Value::Null };
            let mut s = if last.is_object() { last } else { json!({}) };
            s["available"] = json!(true); s
        } else { json!({"available": false}) };
        for (k, v) in job.as_object().cloned().unwrap_or_default() { s[k] = v; }
        let hb = std::path::Path::new("/usr/bin/hearth-browser").exists();
        s["hearth"] = if hb { json!({"available": true, "installed": true, "engine": "Firefox", "version": crate::hearth_browser::version(), "default": system::run("xdg-settings", &["get", "default-web-browser"]).map(|o| o.trim() == "hearth-browser.desktop").unwrap_or(false)}) } else { json!({"available": false}) };
        s
    }
    fn browser_act(&self, what: &str) -> Result<(), String> {
        if !["install", "update", "default", "remove"].contains(&what) { return Err("Unknown action".into()); }
        let helper = paths::desktop_dir().join("shell/hearth-browser");
        { let mut j = self.browser.lock().unwrap(); if j["state"] != "idle" { return Ok(()); } *j = json!({"state": what, "progress": if what == "install" || what == "update" { json!(0) } else { Value::Null }, "line": "", "error": null}); }
        let (job, what) = (self.browser.clone(), what.to_string());
        std::thread::spawn(move || {
            let push = |j: &Value| { let mut s = j.clone(); s["available"] = json!(true); crate::stores::notify(Bridge::event("browser", s)); };
            let r = helper_run(&helper, &[&what], &mut |p, line| { let mut j = job.lock().unwrap(); j["progress"] = json!(p); j["line"] = json!(line); push(&j); });
            let mut j = job.lock().unwrap();
            *j = json!({"state": "idle", "progress": null, "line": "", "error": r.err()});
            push(&j);
        });
        Ok(())
    }

    /// Claude Desktop and Claude Code (desktop/shell/hearth-claude): its status, with the running job's progress.
    fn claude_status(&self) -> Value {
        let helper = paths::desktop_dir().join("shell/hearth-claude");
        let job = self.claude.lock().unwrap().clone();
        let mut s = if helper.exists() {
            let last = if job["state"] == "idle" { helper_run(&helper, &["status"], &mut |_, _| {}).unwrap_or(Value::Null) } else { Value::Null };
            let mut s = if last.is_object() { last } else { json!({}) };
            s["available"] = json!(true); s
        } else { json!({"available": false}) };
        for (k, v) in job.as_object().cloned().unwrap_or_default() { s[k] = v; }
        s
    }
    fn claude_act(&self, what: &str) -> Result<(), String> {
        let args = claude_args(what).ok_or("Unknown action")?;
        let helper = paths::desktop_dir().join("shell/hearth-claude");
        if !helper.exists() { return Err("Claude is installed from Hearth OS's Desktop Mode helper, which isn't here".into()); }
        if what == "open" { return helper_run(&helper, &args, &mut |_, _| {}).map(|_| ()); }
        { let mut j = self.claude.lock().unwrap(); if j["state"] != "idle" { return Ok(()); } *j = json!({"state": what, "progress": json!(0), "line": "", "error": null}); }
        let job = self.claude.clone();
        std::thread::spawn(move || {
            let push = |j: &Value| { let mut s = j.clone(); s["available"] = json!(true); crate::stores::notify(Bridge::event("claude", s)); };
            let r = helper_run(&helper, &args, &mut |p, line| { let mut j = job.lock().unwrap(); j["progress"] = json!(p); j["line"] = json!(line); push(&j); });
            let mut j = job.lock().unwrap();
            *j = json!({"state": "idle", "progress": null, "line": "", "error": r.err()});
            push(&j);
        });
        Ok(())
    }

    fn pending_epic(&self) -> Vec<Value> { self.library.store.data["pendingEpicImports"].as_array().cloned().unwrap_or_default() }

    /// An import finished: what was copied goes in the library (GOG list, your own games, Epic waiting list).
    fn import_done(&mut self, r: &Value) -> Vec<Value> {
        let mut notes: Vec<String> = Vec::new();
        let list = |k: &str| r[k].as_array().cloned().unwrap_or_default();
        let gog = list("gog");
        if !gog.is_empty() {
            let _ = self.library.store.update(|d| {
                { let o = crate::store::object_in(d, "gogInstalls"); for g in &gog { if let (Some(id), Some(p)) = (g["id"].as_str().map(str::to_string).or(g["id"].as_u64().map(|n| n.to_string())), g["path"].as_str()) { o.insert(id, json!(p)); } } }
            });
        }
        let lutris = list("lutris");
        if !lutris.is_empty() {
            let missing: Vec<String> = lutris.iter().filter(|g| g["exe"].as_str().map(str::is_empty).unwrap_or(true)).filter_map(|g| g["name"].as_str().map(str::to_string)).collect();
            let _ = self.library.store.update(|d| {
                let arr = crate::store::array_in(d, "manual");
                for (i, g) in lutris.iter().enumerate() {
                    let Some(exe) = g["exe"].as_str().filter(|e| !e.is_empty()) else { continue };
                    if arr.iter().any(|x| x["path"] == exe) { continue; }
                    arr.push(json!({"id": format!("manual:{:012x}", now_ms() * 100 + i as u64), "title": g["name"], "path": exe, "added": now_ms()}));
                }
            });
            if !missing.is_empty() {
                let one = missing.len() == 1;
                notes.push(format!("Copied {} from Lutris, but couldn't tell which program starts {}. Add {} with Library › Add a game.", missing.join(", "), if one { "it" } else { "them" }, if one { "it" } else { "them" }));
            }
        }
        let epic = list("epic");
        if !epic.is_empty() {
            let mut p = self.pending_epic(); p.extend(epic);
            let n = p.len();
            let _ = self.library.store.update(|d| { d.insert("pendingEpicImports".into(), json!(p.clone())); });
            if crate::stores::epic::signed_in() { self.importer.finish_pending_epic(p); }
            else { notes.push(format!("Sign in to Epic Games (Library › Stores) to finish {}.", if n == 1 { "1 Epic game".to_string() } else { format!("{n} Epic games") })); }
        }
        // Steam reads its game list when it starts: restart it if it's running.
        let steam_games = r["steam"].as_u64().unwrap_or(0);
        let running = std::process::Command::new("pgrep").args(["-x", "steam"]).output().map(|o| o.status.success()).unwrap_or(false);
        if steam_games > 0 && running {
            notes.push("Steam restarted to find the new games. It checks each one before you play.".into());
            let _ = crate::steam::spawn(&self.library.home, "steam://exit");
            let home = self.library.home.clone();
            std::thread::spawn(move || { std::thread::sleep(std::time::Duration::from_secs(8)); crate::steam::ensure_running_silent(&home); });
        } else if steam_games > 0 { notes.push("Steam checks each game the first time you start it.".into()); }
        self.importer.finish(r, notes);
        self.library.scan();
        vec![Bridge::event("library.changed", json!({}))]
    }
    /// Whether the fail-safe sends a report by itself when Hearth gets stuck (Settings; on unless turned off).
    pub fn auto_report(&self) -> bool { self.library.store.data["autoReportStuck"] != false }
    pub fn edition_name() -> &'static str { edition() }
    pub fn version_str(&self) -> String { self.version.clone() }
    pub fn zoom_pct(&self) -> u32 { self.library.store.data["uiScale"].as_u64().unwrap_or(100) as u32 }
    fn auto_update(&self) -> bool { self.library.store.data["autoUpdateCheck"] != false }
    /// Game Mode starts again (gamescope with its new options): the session's next round is Game Mode, and Hearth
    /// leaves (outside the session, Hearth restarts itself).
    pub fn restart_game_mode(&self) {
        let in_session = std::env::var("NIMBUS_SESSION").map(|v| v == "1").unwrap_or(false);
        if in_session { let _ = std::fs::write(std::path::PathBuf::from(std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into())).join("nimbus-next"), "game\n"); }
        let emit = self.emit.clone();
        std::thread::spawn(move || { std::thread::sleep(std::time::Duration::from_millis(600)); emit(json!({"type": if in_session { "quit" } else { "restart" }})); });
    }
    fn display_get(&self) -> Value {
        let mut d = crate::display::get(self.zoom_pct());
        let b = system::brightness();
        d["supported"] = json!(b.is_some());
        d["brightness"] = json!(b.unwrap_or(0));
        d
    }
    /// An event for the UI ({type:'event', name, data}), like the status the top bar refreshes every 30 s.
    pub fn event(name: &str, data: Value) -> Value { json!({"type": "event", "name": name, "data": data}) }
}

/// Hearth's log (the same file the Electron host writes): ~/.local/share/nimbus/bridge.log
pub fn log(text: &str) {
    use std::io::Write;
    let f = paths::data_dir().join("bridge.log");
    // Kept to a few MB: the old one becomes bridge.log.old.
    if std::fs::metadata(&f).map(|m| m.len() > 4_000_000).unwrap_or(false) { let _ = std::fs::rename(&f, paths::data_dir().join("bridge.log.old")); }
    // One write per line: writeln! on a file writes the pieces separately, and lines from several threads (and the
    // other processes writing this log) came out mixed up.
    let line = format!("{} [rust] {}\n", now_ms(), text);
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(f) { let _ = file.write_all(line.as_bytes()); }
}

fn now_ms() -> u64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0) }

/// Started again after switching profiles (the profile picker doesn't show again).
fn picked_file() -> std::path::PathBuf { std::path::PathBuf::from(std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into())).join("hearth-picked") }
pub fn picked() -> bool {
    static P: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *P.get_or_init(|| std::env::args().any(|a| a == "--picked") || std::fs::remove_file(picked_file()).is_ok())
}

/// The player counter's id: 32 hex characters, the same on every update and reinstall of the app on this device.
/// From /etc/machine-id, hashed (the machine id itself never leaves the device). The live USB has a new machine
/// id every boot, so it gets none (the page then uses its own).
pub fn counter_device_id() -> String {
    if std::path::Path::new("/etc/hearth-live").exists() { return String::new(); }
    let mid = std::fs::read_to_string("/etc/machine-id").unwrap_or_default();
    let mid = mid.trim();
    if mid.len() < 16 { return String::new(); }
    let h = sha1_smol::Sha1::from(format!("hearth-counter:{mid}")).digest().to_string();
    h[..32].to_string()
}

/// Settings › System › Save logs: Hearth's logs in one file in your home folder, for bug reports.
fn export_logs(version: &str) -> Result<Value, String> {
    let stamp = chrono::Local::now().format("%Y-%m-%d-%H-%M").to_string();
    let dir = std::env::temp_dir().join(format!("hearth-logs-{}", now_ms()));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let data = paths::data_dir();
    let run = std::path::PathBuf::from(std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into()));
    let mut files: Vec<(&str, String)> = Vec::new();
    for (from, name) in [(data.join("bridge.log"), "bridge.log"), (data.join("session.log"), "session.log"), (run.join("nimbus-shell.log"), "desktop-shell.log"), ("/etc/os-release".into(), "os-release.txt")] {
        if let Ok(b) = std::fs::read(from) { files.push((name, String::from_utf8_lossy(&b).into_owned())); }
    }
    let mut info = crate::sysinfo::info(std::path::Path::new("/")); info["version"] = json!(version); info["host"] = json!("rust");
    files.push(("about.json", serde_json::to_string_pretty(&info).unwrap_or_default()));
    let limit = std::time::Duration::from_secs(15);
    files.push(("bootc-status.txt", system::run_timeout("bootc", &["status"], limit).unwrap_or_default()));
    files.push(("journal-user.txt", system::run_timeout("journalctl", &["--user", "-b", "-n", "3000", "--no-pager"], limit).unwrap_or_default()));
    let user = std::env::var("USER").unwrap_or_default();
    write_redacted(&dir, &files, &user, &paths::home().to_string_lossy());
    let out = paths::home().join(format!("Hearth-logs-{stamp}.tar.gz"));
    let ok = std::process::Command::new("tar").arg("-czf").arg(&out).arg("-C").arg(&dir).arg(".").status().map(|s| s.success()).unwrap_or(false);
    let _ = std::fs::remove_dir_all(&dir);
    if !ok { return Err("Couldn't save the logs".into()); }
    Ok(json!({"path": out.to_string_lossy()}))
}

/// power.get's answer: the battery, with the power mode and limit from hearth-power's answer `st`.
fn power_get(st: Value) -> Value {
    let mut b = system::battery(std::path::Path::new("/sys/class/power_supply"));
    b["mode"] = st["mode"]["mode"].clone(); b["modes"] = st["mode"]["available"].clone(); b["modeSupported"] = st["mode"]["supported"].clone(); b["tdp"] = st["tdp"].clone();
    b
}

/// The files for "Save logs", with personal details taken out the same way as Send logs (they were copied as they
/// were: sign-in tokens, e-mail addresses, the user name and home folder).
fn write_redacted(dir: &std::path::Path, files: &[(&str, String)], user: &str, home: &str) {
    for (name, text) in files { let _ = std::fs::write(dir.join(name), crate::logs::redact(text, user, home)); }
}

fn value_text(v: &Value) -> String { match v { Value::String(s) => s.clone(), Value::Null => String::new(), other => other.to_string() } }

/// What `claude.act` may ask hearth-claude to do (nothing else reaches its command line).
fn claude_args(what: &str) -> Option<Vec<&'static str>> {
    Some(match what {
        "install" => vec!["install"], "update" => vec!["update"], "remove" => vec!["remove"], "open" => vec!["open"],
        "check" => vec!["status", "--check"], "install-code" => vec!["install-code"], "remove-code" => vec!["remove-code"],
        _ => return None,
    })
}

/// Runs one of Desktop Mode's helpers (hearth-browser, hearth-claude): "progress: <pct> <line>" lines while it
/// works, then its JSON answer; "error: ..." is the failure to show.
fn helper_run(helper: &std::path::Path, args: &[&str], on_progress: &mut dyn FnMut(f64, &str)) -> Result<Value, String> {
    use std::io::BufRead;
    let mut child = std::process::Command::new("python3").arg(helper).args(args).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::null())
        .spawn().map_err(|e| e.to_string())?;
    let (mut result, mut err) = (Value::Null, None);
    if let Some(out) = child.stdout.take() {
        for line in std::io::BufReader::new(out).lines().map_while(Result::ok) {
            let line = line.trim();
            if let Some(p) = line.strip_prefix("progress: ") {
                let (n, rest) = p.split_once(' ').unwrap_or((p, ""));
                if let Ok(n) = n.parse::<f64>() { on_progress(n / 100.0, rest); }
            } else if let Some(e) = line.strip_prefix("error: ") { err = Some(e.to_string()); }
            else if line.starts_with('{') { if let Ok(v) = serde_json::from_str(line) { result = v; } }
        }
    }
    let code = child.wait().map(|s| s.code().unwrap_or(-1)).unwrap_or(-1);
    let name = helper.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    match (err, code) { (Some(e), _) => Err(e), (None, 0) => Ok(result), (None, c) => Err(format!("{name} exited with {c}")) }
}

// ------------------------------------------------------------------ "Make Hearth my shell"

const SESSION_FILE: &str = "/usr/share/wayland-sessions/nimbus.desktop";
/// Part of Hearth, the OS (installed under /usr), not a downloaded copy.
fn system_install() -> bool { std::env::current_exe().map(|e| e.starts_with("/usr/")).unwrap_or(false) }
fn shell_status() -> Value {
    let value = std::fs::read_to_string(paths::config_dir().join("app-path")).ok().map(|s| s.trim().to_string());
    let system = system_install();
    json!({"running": crate::gamescope::in_session(), "registered": std::path::Path::new(SESSION_FILE).exists() && (value.is_some() || system), "system": system,
           "value": value, "exe": std::env::current_exe().map(|e| e.to_string_lossy().to_string()).unwrap_or_default(),
           "gamescope": system::has("gamescope"), "labwc": system::has("labwc"), "quickshell": system::has("quickshell") || system::has("qs")})
}
/// Copies the session scripts somewhere root can read, then installs (or removes) the session with pkexec.
fn shell_install(remove: bool) -> Result<(), String> {
    if !system::has("pkexec") { return Err("pkexec is missing. Run install-session.sh as root instead".into()); }
    let desk = paths::desktop_dir();
    let session = desk.parent().map(|p| p.join("session")).filter(|p| p.is_dir()).ok_or("Hearth's session files are missing")?;
    let dst = paths::data_dir().join("session");
    let _ = std::fs::remove_dir_all(&dst);
    system::run("cp", &["-r", &session.to_string_lossy(), &dst.to_string_lossy()])?;
    let _ = system::run("cp", &["-r", &desk.to_string_lossy(), &dst.join("desktop").to_string_lossy()]);
    let _ = system::run("sh", &["-c", "cd \"$0\" && for f in *; do [ -f \"$f\" ] || continue; case \"$f\" in *.desktop) chmod 644 \"$f\";; *) chmod 755 \"$f\";; esac; done", &dst.to_string_lossy()]);
    if !remove {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        std::fs::create_dir_all(paths::config_dir()).map_err(|e| e.to_string())?;
        std::fs::write(paths::config_dir().join("app-path"), format!("{}\n", exe.to_string_lossy())).map_err(|e| e.to_string())?;
    }
    let script = dst.join("install-session.sh").to_string_lossy().to_string();
    let mut args = vec![script.as_str()];
    if remove { args.push("--remove"); }
    system::run_timeout("pkexec", &args, std::time::Duration::from_secs(120)).map(|_| ())
}

fn features() -> Vec<&'static str> {
    let mut f = vec!["restartApp", "exportLogs", "powerButton", "overamp", "quickResume", "friends", "screenshots", "sendLogs", "xodus"];
    // Quick Access on the starting screen (a game or Steam loading) is the overlay, as over the game (gamemode.rs).
    f.push("overlayOverLaunch");
    if crate::guide::controller_available() { f.push("remap"); }
    if crate::multiplay::coop_available() { f.push("coop"); }
    if crate::multiplay::seats_available() { f.push("seats"); }
    if std::env::var_os("HEARTH_SEAT").is_some() { f.push("extraSeat"); }
    if std::path::Path::new("/sys/module/nvidia").exists() { f.push("nvidiaDrawing"); }
    if std::path::Path::new(crate::renderer::ELECTRON_APP).is_file() { f.push("renderer"); }
    // ---- Hearth's gaming apps: Settings › Controls › Controller Lab, a game's ⋯ › Back up saves / Mods
    if crate::gaming_apps::available() { f.push("gamingApps"); }
    // ---- end of Hearth's gaming apps
    f
}
/// Where Hearth opens when started for one thing: `--settings` (Settings), `--page <page>` or `--page=<page>` for a
/// page of Settings (only the ones listed here; "hhd" is Settings › Controls › Handheld Daemon). Null: the usual start.
pub fn start_page(args: &[String]) -> Value {
    const PAGES: &[&str] = &["hhd", "settings"];
    let mut start = Value::Null;
    let mut it = args.iter().skip(1);
    while let Some(a) = it.next() {
        let page = if a == "--page" { it.next().map(String::as_str) } else { a.strip_prefix("--page=") };
        if let Some(p) = page.and_then(|p| PAGES.iter().find(|k| **k == p)) { return json!(p); }
        if a == "--settings" { start = json!("settings"); }
    }
    start
}

/// "Hearth 0.0.25 (Bazzite)": the system's own name, for About and the top of Settings.
fn os_name() -> String {
    std::fs::read_to_string("/etc/os-release").ok().and_then(|t| t.lines().find_map(|l| l.strip_prefix("PRETTY_NAME=").map(|v| v.trim_matches('"').to_string())))
        .unwrap_or_else(|| "Linux".into())
}
/// Which Hearth this is, for the player counter: the live USB, the Nvidia or AMD/Intel image, or a download.
fn edition() -> &'static str {
    if std::path::Path::new("/etc/hearth-live").exists() { return "live"; }
    if !system_install() { return "appimage"; }
    // The non-Bazzite editions (Anatase, Ubuntu, CachyOS, Peppermint) describe themselves here.
    let own = std::fs::read_to_string("/usr/share/nimbus/image-info.json").ok().and_then(|t| serde_json::from_str::<Value>(&t).ok());
    if let Some(base) = own.as_ref().and_then(|v| v["base"].as_str()) {
        return match base { "anatase" => "anatase", "ubuntu" => "ubuntu", "cachyos" => "cachyos", "peppermint" => "peppermint", _ => "other" };
    }
    let info = std::fs::read_to_string("/usr/share/ublue-os/image-info.json").unwrap_or_default();
    if info.to_lowercase().contains("nvidia") { "nvidia" } else { "amd" }
}

#[allow(dead_code)]
fn hostname() -> String { std::fs::read_to_string("/etc/hostname").map(|s| s.trim().to_string()).unwrap_or_default() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn answers_calls_and_says_what_is_missing() {
        let dir = tempfile::tempdir().unwrap();
        let mut b = Bridge::with(Library::new(Store::open(dir.path().join("l.json")), dir.path().to_path_buf()), crate::profiles::Profiles::open(dir.path()), "0.0.23");
        let r = b.handle(&json!({"type":"call","id":7,"method":"library.get","args":{}})).unwrap();
        assert_eq!(r["id"], 7); assert_eq!(r["ok"], true);
        // Nothing installed: only Xbox Cloud Gaming's tile (it needs no install).
        let ids: Vec<&str> = r["value"]["games"].as_array().unwrap().iter().filter_map(|g| g["id"].as_str()).collect();
        assert_eq!(ids, vec!["xbox:cloud"]);
        let r = b.handle(&json!({"type":"call","id":8,"method":"epic.signIn"})).unwrap();
        assert_eq!(r["ok"], false); assert!(r["error"].as_str().unwrap().contains("Rust version"));
        assert!(b.handle(&json!({"type":"log","text":"hi"})).is_none());
        let r = b.handle(&json!({"type":"call","id":9,"method":"library.setFav","args":{"id":"steam:1","on":true}})).unwrap();
        assert_eq!(r["ok"], true);
        assert!(b.library.store.list_has("favorites", "steam:1"));
    }
    #[test]
    fn claude_actions_are_a_fixed_list() {
        assert_eq!(claude_args("install"), Some(vec!["install"]));
        assert_eq!(claude_args("check"), Some(vec!["status", "--check"]));
        assert_eq!(claude_args("install-code"), Some(vec!["install-code"]));
        assert_eq!(claude_args("remove-code"), Some(vec!["remove-code"]));
        for bad in ["", "auto-update", "install; rm -rf ~", "--help", "status"] { assert_eq!(claude_args(bad), None, "{bad}"); }
    }
    #[test]
    fn helper_run_reads_progress_answer_and_errors() {
        let dir = tempfile::tempdir().unwrap();
        let ok = dir.path().join("hearth-claude");
        std::fs::write(&ok, "import sys\nprint('progress: 40 Downloading Claude Desktop', flush=True)\nprint('log: ignored')\nprint('{\"installed\": true, \"args\": \"' + ' '.join(sys.argv[1:]) + '\"}')\n").unwrap();
        let mut seen = Vec::new();
        let v = helper_run(&ok, &["status", "--check"], &mut |p, l| seen.push((p, l.to_string()))).unwrap();
        assert_eq!(v["installed"], true);
        assert_eq!(v["args"], "status --check");
        assert_eq!(seen, vec![(0.4, "Downloading Claude Desktop".to_string())]);
        let bad = dir.path().join("bad");
        std::fs::write(&bad, "print('error: Close Claude first, then try again')\nraise SystemExit(1)\n").unwrap();
        assert_eq!(helper_run(&bad, &[], &mut |_, _| {}).unwrap_err(), "Close Claude first, then try again");
        let silent = dir.path().join("silent");
        std::fs::write(&silent, "raise SystemExit(3)\n").unwrap();
        assert_eq!(helper_run(&silent, &[], &mut |_, _| {}).unwrap_err(), "silent exited with 3");
    }
    #[test]
    fn claude_act_refuses_what_isnt_on_the_list() {
        let dir = tempfile::tempdir().unwrap();
        let mut b = Bridge::with(Library::new(Store::open(dir.path().join("l.json")), dir.path().to_path_buf()), crate::profiles::Profiles::open(dir.path()), "0.0.23");
        let r = b.handle(&json!({"type":"call","id":4,"method":"claude.act","args":{"action":"auto-update"}})).unwrap();
        assert_eq!(r["ok"], false);
        assert_eq!(r["error"], "Unknown action");
    }
    #[test]
    fn saved_logs_leave_out_personal_details() {
        let dir = tempfile::tempdir().unwrap();
        write_redacted(dir.path(), &[("bridge.log", "12:00:00 gog: refresh_token=abcdef123456 for sam.smith@gmail.com in /home/sam/Games".to_string())], "sam", "/home/sam");
        let t = std::fs::read_to_string(dir.path().join("bridge.log")).unwrap();
        assert!(!t.contains("abcdef123456") && !t.contains("gmail") && !t.contains("/home/sam"), "{t}");
    }
    #[test]
    fn a_request_that_panics_still_gets_an_answer() {
        let dir = tempfile::tempdir().unwrap();
        let mut b = Bridge::with(Library::new(Store::open(dir.path().join("l.json")), dir.path().to_path_buf()), crate::profiles::Profiles::open(dir.path()), "0.0.23");
        // A damaged library.json ("manual" isn't a list) made adding a game panic.
        b.library.store.data["manual"] = json!(5);
        let file = dir.path().join("game.sh");
        std::fs::write(&file, "#!/bin/sh\n").unwrap();
        let r = b.handle(&json!({"type":"call","id":1,"method":"library.addManual","args":{"path": file.to_string_lossy()}})).unwrap();
        assert_eq!(r["id"], 1);
        // And now it's repaired instead: the game goes in a fresh list.
        assert_eq!(r["ok"], true, "{r}");
        assert_eq!(b.library.store.data["manual"].as_array().map(|a| a.len()), Some(1));
        let r = b.handle(&json!({"type":"call","id":2,"method":"library.setFav","args":{"id":"steam:1","on":true}})).unwrap();
        assert_eq!(r["ok"], true);
    }
    #[test]
    fn answers_everything_the_electron_host_does() {
        // Every method the Electron host's bridge answers has an answer here too.
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let node = std::fs::read_to_string(dir.join("../linux/src/bridge.js")).unwrap();
        let rust = std::fs::read_to_string(dir.join("src/bridge.rs")).unwrap();
        let methods = regex::Regex::new(r"case '([a-zA-Z]+\.[a-zA-Z.]+)'").unwrap();
        let missing: Vec<String> = methods.captures_iter(&node).map(|c| c[1].to_string())
            .filter(|m| !rust.contains(&format!("\"{m}\" =>")) && !rust.contains(&format!("\"{m}\" |")) && !rust.contains(&format!("| \"{m}\""))).collect();
        assert!(missing.is_empty(), "not in the Rust host yet: {missing:?}");
    }
    #[test]
    fn reads_the_start_page() {
        let a = |v: &[&str]| start_page(&v.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        assert_eq!(a(&["nimbus"]), Value::Null);
        assert_eq!(a(&["nimbus", "--windowed", "--settings"]), json!("settings"));
        assert_eq!(a(&["nimbus", "--windowed", "--page", "hhd"]), json!("hhd"));
        assert_eq!(a(&["nimbus", "--page=hhd", "--windowed"]), json!("hhd"));
        assert_eq!(a(&["nimbus", "--settings", "--page", "hhd"]), json!("hhd"));
        // Pages Hearth doesn't know, or none given: the usual start (or Settings, when asked for too).
        assert_eq!(a(&["nimbus", "--page", "rm -rf"]), Value::Null);
        assert_eq!(a(&["nimbus", "--page"]), Value::Null);
        assert_eq!(a(&["nimbus", "--page", "nope", "--settings"]), json!("settings"));
        // The program's own name is never read as an option.
        assert_eq!(a(&["--settings"]), Value::Null);
    }
    #[test]
    fn read_only_calls_answer() {
        let dir = tempfile::tempdir().unwrap();
        let mut b = Bridge::with(Library::new(Store::open(dir.path().join("l.json")), dir.path().to_path_buf()), crate::profiles::Profiles::open(dir.path()), "0.0.23");
        for (i, m) in ["host.info", "library.get", "library.downloads", "accounts.status", "display.get", "update.status", "import.status", "osinstall.status",
                       "overlay.settings", "apps.list", "shell.status", "art.status", "perf.stats", "pad.status", "profiles.get", "profile.get", "desktop.wallpaper",
                       "audio.get", "sys.status", "power.get", "system.info"].iter().enumerate() {
            let r = b.handle(&json!({"type": "call", "id": i, "method": m, "args": {}})).unwrap_or_else(|| panic!("{m}: no answer"));
            assert!(r["ok"] == true || !r["error"].as_str().unwrap_or("").contains("isn't in the Rust version"), "{m}: {r}");
        }
        // Settings › Updates shows Update now only when it knows this is the OS install ("system").
        let info = b.handle(&json!({"type": "call", "id": 98, "method": "host.info", "args": {}})).unwrap();
        for k in ["system", "start", "edition", "live", "installer", "session", "features", "setupDone", "version", "name"] {
            assert!(info["value"].get(k).is_some(), "host.info is missing {k}");
        }
        let a = b.handle(&json!({"type": "call", "id": 99, "method": "accounts.status", "args": {}})).unwrap();
        assert_eq!(a["value"]["accounts"].as_array().unwrap().len(), 4);
        assert_eq!(b.handle(&json!({"type": "call", "id": 100, "method": "library.downloads", "args": {}})).unwrap()["value"]["active"], json!([]));
    }
}
