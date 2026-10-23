// Nearby Share's window (Tauri: WebKitGTK on Linux, WebView2 on Windows), and a few modes without a window:
//   hearth-nearby [FILE...]                    the window (files given are ready to send: "Open with" from Files)
//   hearth-nearby --fullscreen                 the window, full screen (opened from Game Mode)
//   hearth-nearby --fingerprint                this PC's Nearby Share fingerprint
//   hearth-nearby --send-to IP[:PORT] [--pin P] [--text T | FILE...]
//                                              send without a window (exit 0 sent, 3 declined, 4 PIN needed, 1 error)
//   hearth-nearby --discover SECONDS           announce, then list the devices seen as JSON
//   hearth-nearby --serve --answer accept|decline [--port N] [--alias A]
//                                              receive without a window, answering every request the same way.
//                                              Tests only: refused unless HEARTH_NEARBY_TEST=1.
// The page (ui/index.html) asks for the state, answers requests and sends; events come back as "nearby://<name>".
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use hearth_nearby::send::{Outcome, Target, What};
use hearth_nearby::service::{self, lock, Options, Service};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

static SVC: OnceLock<Arc<Service>> = OnceLock::new();
static PRESELECTED: OnceLock<Vec<String>> = OnceLock::new();

fn svc() -> Result<Arc<Service>, String> { SVC.get().cloned().ok_or_else(|| "Nearby Share is still starting".to_string()) }

#[tauri::command]
fn state() -> Result<Value, String> {
    let mut v = svc()?.snapshot();
    v["preselected"] = json!(PRESELECTED.get().cloned().unwrap_or_default());
    Ok(v)
}

#[tauri::command]
async fn refresh() -> Result<(), String> {
    let s = svc()?;
    lock(&s.devices).clear();
    s.refresh.notify_one();
    s.changed();
    Ok(())
}

#[tauri::command]
async fn decide(accept: bool, ids: Option<Vec<String>>, trust: bool) -> Result<(), String> { svc()?.decide(accept, ids, trust) }

#[tauri::command]
async fn cancel_receive() -> Result<(), String> { svc()?.cancel_receive("You cancelled"); Ok(()) }

fn target_for(s: &Service, fingerprint: &str) -> Result<Target, String> {
    let d = lock(&s.devices).get(fingerprint).cloned().ok_or("That device isn't around any more. Look again.")?;
    Ok(Target { ip: d.ip, port: d.port, https: d.https, fingerprint: Some(d.fingerprint), alias: d.alias })
}

#[tauri::command]
async fn send(fingerprint: String, paths: Vec<String>, text: Option<String>, pin: Option<String>) -> Result<(), String> {
    let s = svc()?;
    let t = target_for(&s, &fingerprint)?;
    let what = match text { Some(t) if !t.is_empty() => What::Text(t), _ => What::Files(paths.into_iter().map(PathBuf::from).collect()) };
    if let What::Files(p) = &what { if p.is_empty() { return Err("Choose something to send first".into()); } }
    hearth_nearby::send::start(s, t, what, pin.filter(|p| !p.is_empty()))
}

#[tauri::command]
async fn cancel_send() -> Result<(), String> { let s = svc()?; hearth_nearby::send::cancel(&s); Ok(()) }

#[tauri::command]
async fn add_address(address: String) -> Result<Value, String> {
    let s = svc()?;
    let i = hearth_nearby::discovery::probe(&s, &address).await?;
    Ok(json!({"alias": i.alias}))
}

#[tauri::command]
async fn list_dir(path: String) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || hearth_nearby::files::list_dir(&path)).await.map_err(|e| e.to_string())?
}

#[tauri::command]
fn places() -> Value { hearth_nearby::files::places() }

#[tauri::command]
async fn set_alias(alias: String) -> Result<(), String> { svc()?.set_alias(&alias) }

#[tauri::command]
async fn set_pin(pin: Option<String>) -> Result<(), String> { svc()?.set_pin(pin) }

#[tauri::command]
async fn trust(fingerprint: String, alias: String, on: bool) -> Result<(), String> { svc()?.trust(&fingerprint, &alias, on) }

/// Opens the receive folder (or a received file's folder) in the file manager.
#[tauri::command]
fn open_folder(path: Option<String>) -> Result<(), String> {
    let s = svc()?;
    let p = path.map(PathBuf::from).unwrap_or_else(|| s.opts.dest.clone());
    let dir = if p.is_dir() { p } else { p.parent().map(|x| x.to_path_buf()).unwrap_or_else(|| s.opts.dest.clone()) };
    let _ = std::fs::create_dir_all(&dir);
    #[cfg(target_os = "linux")]
    let r = std::process::Command::new("xdg-open").arg(&dir).spawn();
    #[cfg(windows)]
    let r = std::process::Command::new("explorer").arg(&dir).spawn();
    #[cfg(not(any(target_os = "linux", windows)))]
    let r = std::process::Command::new("open").arg(&dir).spawn();
    r.map(|_| ()).map_err(|e| format!("Couldn't open the folder: {e}"))
}

fn arg_value(args: &[String], name: &str) -> Option<String> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1).cloned())
}

/// The modes without a window. Some(exit code) if one ran.
fn headless(args: &[String]) -> Option<i32> {
    let mode = ["--fingerprint", "--send-to", "--discover", "--serve"].into_iter().find(|m| args.iter().any(|a| a == m))?;
    let rt = tokio::runtime::Runtime::new().expect("tokio");
    Some(rt.block_on(async move {
        let print: hearth_nearby::Emit = Arc::new(|ev: &str, v: Value| { println!("{}", json!({"event": ev, "data": v})); });
        match mode {
            "--fingerprint" => match hearth_nearby::tls::load_or_create(&hearth_nearby::config_dir()) {
                Ok(id) => { println!("{}", id.fingerprint); 0 }
                Err(e) => { eprintln!("{e}"); 1 }
            },
            "--serve" => {
                let answer = match arg_value(args, "--answer").as_deref() { Some("accept") => Some(true), Some("decline") => Some(false), _ => None };
                if std::env::var("HEARTH_NEARBY_TEST").as_deref() != Ok("1") || answer.is_none() {
                    eprintln!("--serve is for tests: it needs HEARTH_NEARBY_TEST=1 and --answer accept|decline");
                    return 2;
                }
                let mut o = Options { answer, ..Options::default() };
                if let Some(p) = arg_value(args, "--port").and_then(|p| p.parse().ok()) { o.port = p; }
                o.alias = arg_value(args, "--alias");
                let s = match service::start(o, print).await { Ok(s) => s, Err(e) => { eprintln!("{e}"); return 1; } };
                if let Some(pin) = arg_value(args, "--pin") { let _ = s.set_pin(Some(pin)); }
                println!("{}", json!({"event": "listening", "data": {"port": s.port, "fingerprint": s.id.fingerprint, "folder": s.opts.dest}}));
                let _ = tokio::signal::ctrl_c().await;
                s.shutdown();
                0
            }
            "--discover" => {
                let secs: u64 = arg_value(args, "--discover").and_then(|s| s.parse().ok()).unwrap_or(5);
                let o = Options { port: hearth_nearby::DEFAULT_PORT + 20, ..Options::default() };
                let s = match service::start(o, Arc::new(|_: &str, _: Value| {})).await { Ok(s) => s, Err(e) => { eprintln!("{e}"); return 1; } };
                tokio::time::sleep(Duration::from_secs(secs)).await;
                println!("{}", s.snapshot()["devices"]);
                s.shutdown();
                0
            }
            _ => {
                let Some(to) = arg_value(args, "--send-to") else { eprintln!("--send-to IP[:PORT]"); return 2 };
                let (ip, port) = match to.parse::<std::net::SocketAddr>() { Ok(a) => (a.ip(), a.port()), Err(_) => match to.parse() { Ok(ip) => (ip, hearth_nearby::DEFAULT_PORT), Err(_) => { eprintln!("not an address: {to}"); return 2; } } };
                let o = Options { port: hearth_nearby::DEFAULT_PORT + 30, discovery: false, ..Options::default() };
                let s = match service::start(o, print).await { Ok(s) => s, Err(e) => { eprintln!("{e}"); return 1; } };
                // Learn the receiver's name and fingerprint first (so its certificate is pinned), as the window does.
                let info = hearth_nearby::discovery::register(&s, ip, port, true, None).await.ok();
                let target = Target { ip, port, https: true, fingerprint: info.as_ref().map(|i| i.fingerprint.clone()), alias: info.map(|i| i.alias).unwrap_or_else(|| to.clone()) };
                let what = match arg_value(args, "--text") { Some(t) => What::Text(t), None => {
                    let mut files = vec![]; let mut skip = false;
                    for a in args.iter().skip(1) {
                        if skip { skip = false; continue; }
                        if a == "--send-to" || a == "--pin" || a == "--text" { skip = true; continue; }
                        if a.starts_with("--") { continue; }
                        files.push(PathBuf::from(a));
                    }
                    What::Files(files)
                } };
                let r = hearth_nearby::send::send(s.clone(), target, what, arg_value(args, "--pin"), tokio_util::sync::CancellationToken::new()).await;
                s.shutdown();
                match r {
                    Ok(Outcome::Sent(n)) => { println!("{}", json!({"event": "sent", "data": {"count": n}})); 0 }
                    Ok(Outcome::Declined) => { println!("{}", json!({"event": "declined"})); 3 }
                    Ok(Outcome::NeedPin) => { println!("{}", json!({"event": "need-pin"})); 4 }
                    Ok(o) => { println!("{}", json!({"event": format!("{o:?}")})); 1 }
                    Err(e) => { eprintln!("{e}"); 1 }
                }
            }
        }
    }))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let Some(code) = headless(&args) { std::process::exit(code); }
    let fullscreen = args.iter().any(|a| a == "--fullscreen");
    let files: Vec<String> = args.iter().skip(1).filter(|a| !a.starts_with("--"))
        .filter_map(|a| std::fs::canonicalize(a.strip_prefix("file://").unwrap_or(a)).ok()).map(|p| p.to_string_lossy().to_string()).collect();
    let _ = PRESELECTED.set(files);
    // WebKit's newer GPU path draws a black window on some Nvidia drivers; Nearby Share doesn't need its speed.
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() { std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1"); }
    let app = tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![state, refresh, decide, cancel_receive, send, cancel_send, add_address, list_dir, places, set_alias, set_pin, trust, open_folder])
        .setup(move |app| {
            let handle = app.handle().clone();
            let emit: hearth_nearby::Emit = Arc::new(move |ev: &str, v: Value| { let _ = handle.emit(&format!("nearby://{ev}"), v); });
            let o = Options { firewall: cfg!(target_os = "linux"), ..Options::default() };
            match tauri::async_runtime::block_on(service::start(o, emit)) {
                Ok(s) => { let _ = SVC.set(s); }
                Err(e) => eprintln!("Nearby Share couldn't start listening: {e}"),
            }
            WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title("Nearby Share")
                .inner_size(1180.0, 760.0)
                .min_inner_size(640.0, 480.0)
                .fullscreen(fullscreen)
                .background_color(tauri::window::Color(15, 11, 9, 255))
                .build()?;
            let _ = app.get_webview_window("main");
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("Nearby Share couldn't start");
    app.run(|_, ev| {
        if let tauri::RunEvent::Exit = ev {
            // Stop listening and close the firewall again.
            if let Some(s) = SVC.get() { s.shutdown(); }
        }
    });
}
