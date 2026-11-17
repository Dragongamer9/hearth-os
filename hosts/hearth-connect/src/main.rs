// Hearth Connect:
//   hearth-connect                  the window (starts the service if it isn't running)
//   hearth-connect --fullscreen     the window, full screen (opened from Game Mode)
//   hearth-connect --daemon         the background service (hearth-connect.service, a systemd user unit)
//   hearth-connect --call CMD [JSON]  one request to the service, its answer as JSON (scripts and tests)
//   hearth-connect --events         the service's events as JSON lines, until it stops
//   hearth-connect --autostart      starts the service if Phone link is on (Mantel's autostart runs this at sign-in)
//   hearth-connect --turn-on / --turn-off   Phone link on or off (remembered in settings.json, "enabled")
// The window's page (ui/index.html) calls `call` (forwarded to the service) and hears "connect://event".
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Hearth's look (Settings › Appearance), shared by Hearth's apps.
#[path = "../../hearth-apptheme/apptheme.rs"]
mod apptheme;
use serde_json::{json, Value};
use std::time::Duration;

#[cfg(unix)]
use hearth_connect::{daemon::Daemon, ipc};

/// The service: links, discovery, the window's socket, the firewall (opened while it runs).
#[cfg(unix)]
fn run_daemon() -> i32 {
    if ipc::running() { eprintln!("Hearth Connect's service is already running"); return 0; }
    let rt = tokio::runtime::Runtime::new().expect("tokio");
    rt.block_on(async {
        let home = hearth_connect::config_dir();
        let me = match hearth_connect::identity::load_or_create(&home) { Ok(m) => m, Err(e) => { eprintln!("{e}"); return 1; } };
        let d = Daemon::new(me, home, hearth_connect::receive_dir());
        let stop = d.stop_tx.subscribe();
        let sock = hearth_connect::socket_path();
        {
            let (d, stop) = (d.clone(), stop.clone());
            tokio::spawn(async move { if let Err(e) = ipc::serve(d, &sock, stop).await { eprintln!("socket: {e}"); } });
        }
        tokio::spawn(d.clone().housekeeping(stop.clone()));
        // The firewall: open to the local network while the service runs (not in tests).
        let manage_fw = std::env::var("HEARTH_CONNECT_TEST").as_deref() != Ok("1") && std::path::Path::new(hearth_connect_fw::HELPER).exists();
        if manage_fw {
            let d2 = d.clone();
            tokio::task::spawn_blocking(move || { let v = hearth_connect_fw::helper("open"); *hearth_connect::lock(&d2.firewall) = v; });
        }
        let lan = {
            let (d, stop) = (d.clone(), stop.clone());
            tokio::spawn(async move { hearth_connect::lan::run(d, stop).await })
        };
        eprintln!("Hearth Connect: this PC is {} ({})", hearth_connect::lock(&d.name), d.me.device_id);
        let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).expect("signal");
        let mut stop_rx = stop.clone();
        let code = tokio::select! {
            r = lan => match r { Ok(Ok(())) => 0, Ok(Err(e)) => { eprintln!("{e}"); 1 }, Err(_) => 1 },
            _ = tokio::signal::ctrl_c() => 0,
            _ = term.recv() => 0,
            _ = stop_rx.changed() => 0,
        };
        let _ = d.stop_tx.send(true);
        if manage_fw { let _ = tokio::task::spawn_blocking(|| hearth_connect_fw::helper("close")).await; }
        tokio::time::sleep(Duration::from_millis(200)).await;
        code
    })
}

/// The firewall helper (the same one Nearby Share uses; see os/system_files/usr/libexec/nimbus/hearth-link-firewall).
mod hearth_connect_fw {
    use serde_json::{json, Value};
    pub const HELPER: &str = "/usr/libexec/nimbus/hearth-link-firewall";
    pub fn helper(action: &str) -> Value {
        match std::process::Command::new("pkexec").args([HELPER, action, "connect"]).stdin(std::process::Stdio::null()).output() {
            Ok(o) => String::from_utf8_lossy(&o.stdout).lines().rev().find_map(|l| serde_json::from_str::<Value>(l).ok())
                .unwrap_or_else(|| json!({"ok": false, "error": String::from_utf8_lossy(&o.stderr).trim().to_string()})),
            Err(e) => json!({"ok": false, "error": e.to_string()}),
        }
    }
}

/// Starts the service if it isn't running: the systemd user unit if there is one, else this program in the background.
#[cfg(unix)]
fn ensure_service() -> Result<(), String> {
    if ipc::running() { return Ok(()); }
    let unit = std::process::Command::new("systemctl").args(["--user", "start", "hearth-connect.service"]).stderr(std::process::Stdio::null()).status().map(|s| s.success()).unwrap_or(false);
    if !unit {
        use std::os::unix::process::CommandExt;
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let mut c = std::process::Command::new(exe);
        c.arg("--daemon").stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
        unsafe { c.pre_exec(|| { libc::setsid(); Ok(()) }); }
        c.spawn().map_err(|e| e.to_string())?;
    }
    for _ in 0..50 { if ipc::running() { return Ok(()); } std::thread::sleep(Duration::from_millis(100)); }
    Err("Hearth Connect's service didn't start".into())
}

#[cfg(unix)]
#[tauri::command]
async fn call(cmd: String, args: Option<Value>) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let r = ipc::call(&cmd, args.clone().unwrap_or(json!({})));
        if r.is_err() && !ipc::running() { ensure_service()?; return ipc::call(&cmd, args.unwrap_or(json!({}))); }
        r
    }).await.map_err(|e| e.to_string())?
}

// ---- Phone link on or off. The service's unit has no [Install]: Mantel's autostart runs `hearth-connect --autostart`
// at sign-in, which starts it only when the person turned Phone link on. The choice is "enabled" in settings.json
// (merged in, never written by itself). Before this, "on" was the unit being enabled; that still counts as on.

fn settings_path() -> std::path::PathBuf { hearth_connect::config_dir().join("settings.json") }

/// Is Phone link on? "enabled" in settings.json, or, where it was never set, the old enabled unit.
fn enabled() -> bool {
    if let Some(on) = hearth_connect::appfile::read_json(&settings_path()).and_then(|v| v.get("enabled").and_then(|e| e.as_bool())) { return on; }
    let home = std::env::var_os("HOME").map(std::path::PathBuf::from).unwrap_or_default();
    let cfg = std::env::var_os("XDG_CONFIG_HOME").map(std::path::PathBuf::from).unwrap_or_else(|| home.join(".config"));
    cfg.join("systemd/user/default.target.wants/hearth-connect.service").exists()
}

fn set_enabled(on: bool) -> Result<(), String> {
    let p = settings_path();
    if let Some(dir) = p.parent() { let _ = std::fs::create_dir_all(dir); }
    hearth_connect::appfile::update(&p, |cur| {
        let mut o = cur.and_then(|v| v.as_object().cloned()).unwrap_or_default();
        o.insert("enabled".into(), json!(on));
        (Some(Value::Object(o)), ())
    }).map_err(|e| format!("Couldn't save it: {e}"))
}

#[cfg(unix)]
fn stop_service() {
    let _ = std::process::Command::new("systemctl").args(["--user", "stop", "hearth-connect.service"]).stderr(std::process::Stdio::null()).status();
    if ipc::running() { let _ = ipc::call("quit", json!({})); }
}

/// "Start when I sign in" (the window's switch): the same as Phone link on or off.
#[cfg(unix)]
#[tauri::command]
fn autostart(on: Option<bool>) -> Result<bool, String> {
    if let Some(on) = on { set_enabled(on)?; }
    Ok(enabled())
}

/// Turns Hearth Connect off: the service stops (phones can't reach this PC) and doesn't start at sign-in.
#[cfg(unix)]
#[tauri::command]
fn turn_off() -> Result<(), String> {
    set_enabled(false)?;
    stop_service();
    Ok(())
}

#[cfg(unix)]
#[tauri::command]
fn turn_on() -> Result<(), String> {
    set_enabled(true)?;
    ensure_service()
}

#[cfg(unix)]
#[tauri::command]
fn open_path(path: String) -> Result<(), String> {
    std::process::Command::new("xdg-open").arg(&path).spawn().map(|_| ()).map_err(|e| format!("Couldn't open it: {e}"))
}

#[tauri::command]
async fn list_dir(path: String) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || list(&path)).await.map_err(|e| e.to_string())?
}

/// A folder's contents for the file browser (folders first, hidden ones left out).
fn list(path: &str) -> Result<Value, String> {
    let home = std::env::var_os("HOME").map(std::path::PathBuf::from).unwrap_or_else(|| "/".into());
    let p = if path.is_empty() { home } else { std::path::PathBuf::from(path) };
    let mut dirs = vec![];
    let mut files = vec![];
    for e in std::fs::read_dir(&p).map_err(|e| format!("Can't open {}: {e}", p.display()))?.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if name.starts_with('.') { continue; }
        let Ok(md) = std::fs::metadata(e.path()) else { continue };
        let v = json!({"name": name, "path": e.path().to_string_lossy(), "dir": md.is_dir(), "size": md.len()});
        if md.is_dir() { dirs.push(v) } else { files.push(v) }
    }
    let k = |v: &Value| v["name"].as_str().unwrap_or("").to_lowercase();
    dirs.sort_by_key(k);
    files.sort_by_key(k);
    dirs.extend(files);
    Ok(json!({"path": p.to_string_lossy(), "parent": p.parent().map(|x| x.to_string_lossy().to_string()), "items": dirs}))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    #[cfg(unix)]
    {
        if args.iter().any(|a| a == "--daemon") { std::process::exit(run_daemon()); }
        if let Some(i) = args.iter().position(|a| a == "--call") {
            let cmd = args.get(i + 1).cloned().unwrap_or_else(|| "state".into());
            let a: Value = args.get(i + 2).and_then(|s| serde_json::from_str(s).ok()).unwrap_or(json!({}));
            match ipc::call(&cmd, a) {
                Ok(v) => { println!("{v}"); std::process::exit(0); }
                Err(e) => { eprintln!("{e}"); std::process::exit(1); }
            }
        }
        if args.iter().any(|a| a == "--autostart") {
            if !enabled() { std::process::exit(0); }
            std::process::exit(match ensure_service() { Ok(()) => 0, Err(e) => { eprintln!("{e}"); 1 } });
        }
        if args.iter().any(|a| a == "--turn-on") {
            let r = set_enabled(true).and_then(|_| ensure_service());
            match r { Ok(()) => { println!("{}", json!({"enabled": true})); std::process::exit(0); } Err(e) => { eprintln!("{e}"); std::process::exit(1); } }
        }
        if args.iter().any(|a| a == "--turn-off") {
            if let Err(e) = set_enabled(false) { eprintln!("{e}"); std::process::exit(1); }
            stop_service();
            println!("{}", json!({"enabled": false}));
            std::process::exit(0);
        }
        if args.iter().any(|a| a == "--is-enabled") { println!("{}", json!({"enabled": enabled(), "running": ipc::running()})); std::process::exit(0); }
        if args.iter().any(|a| a == "--events") {
            let r = ipc::events(|v| println!("{v}"));
            std::process::exit(if r.is_ok() { 0 } else { 1 });
        }
    }
    run_window(args.iter().any(|a| a == "--fullscreen"));
}

#[cfg(unix)]
fn run_window(fullscreen: bool) {
    use tauri::{Emitter, WebviewUrl, WebviewWindowBuilder};
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() { std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1"); }
    let started = ensure_service();
    tauri::Builder::default().plugin(apptheme::plugin())
        .invoke_handler(tauri::generate_handler![call, autostart, turn_off, turn_on, open_path, list_dir])
        .setup(move |app| {
            if let Err(e) = &started { eprintln!("{e}"); }
            // Events from the service to the page, reconnecting if the service restarts.
            let handle = app.handle().clone();
            std::thread::spawn(move || loop {
                let h = handle.clone();
                let _ = ipc::events(move |v| { let _ = h.emit("connect://event", v); });
                let _ = handle.emit("connect://event", json!({"event": "service-gone", "data": {}}));
                std::thread::sleep(Duration::from_secs(2));
            });
            WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title("Hearth Connect")
                .inner_size(1180.0, 760.0)
                .min_inner_size(640.0, 480.0)
                .fullscreen(fullscreen)
                .background_color(apptheme::bg())
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Hearth Connect couldn't start");
}

#[cfg(not(unix))]
fn run_window(_fullscreen: bool) {
    eprintln!("Hearth Connect runs on Hearth OS (Linux) for now.");
}
