//! Hearth (Rust host): puts Hearth's screens (ui/index.html, the same ones Ember uses on Windows) on screen
//! with the system's web view, and answers their messages with hearth-core. Replaces the Electron host.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use hearth_core::bridge::Bridge;
use std::sync::{Arc, Mutex};
use tauri::http::{header::CONTENT_TYPE, Response, StatusCode};
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

/// The page's side of the message channel, the same one the Electron host's preload gave it:
/// nimbusHost.postMessage(obj) and nimbusHost.addEventListener('message', fn). Messages wait until the
/// page is listening, so no reply arrives before there's someone to hear it.
const SHIM: &str = r#"
(() => {
  const tauri = () => new Promise(ok => { const t = () => window.__TAURI__ ? ok(window.__TAURI__) : setTimeout(t, 5); t(); });
  let listening = null;
  window.nimbusHost = {
    platform: 'linux',
    postMessage(msg) { (listening || tauri()).then(() => tauri()).then(T => T.core.invoke('nimbus_msg', { msg })); },
    addEventListener(type, fn) {
      if (type !== 'message' || typeof fn !== 'function') return;
      listening = tauri().then(T => T.event.listen('nimbus:msg', e => fn({ data: e.payload })));
    },
  };
})();
"#;

struct Host(Arc<Mutex<Bridge>>);

/// A message from the page: handled off the window's thread (scanning the library can take a moment).
#[tauri::command]
async fn nimbus_msg(app: tauri::AppHandle, host: tauri::State<'_, Host>, msg: serde_json::Value) -> Result<(), String> {
    let bridge = host.0.clone();
    let reply = tauri::async_runtime::spawn_blocking(move || bridge.lock().ok().and_then(|mut b| b.handle(&msg)))
        .await
        .map_err(|e| e.to_string())?;
    if let Some(r) = reply {
        app.emit("nimbus:msg", r).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// nimbus-art://file/<path>: game art from the disk (Steam's covers, your own pictures). Pictures only.
fn art(uri: &str) -> Response<Vec<u8>> {
    let path = uri
        .splitn(2, "://file/")
        .nth(1)
        .map(|p| percent_decode(p.split('?').next().unwrap_or(p)))
        .unwrap_or_default();
    let ext = path.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    let mime = match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        _ => "",
    };
    match (mime, std::fs::read(&path)) {
        ("", _) | (_, Err(_)) => Response::builder().status(StatusCode::NOT_FOUND).body(Vec::new()).unwrap(),
        (m, Ok(bytes)) => Response::builder()
            .header(CONTENT_TYPE, m)
            .header("Access-Control-Allow-Origin", "*")
            .body(bytes)
            .unwrap(),
    }
}

fn main() {
    // WebKit's newer GPU path draws a black window on some Nvidia drivers: the older one works everywhere.
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }
    let windowed = std::env::args().any(|a| a == "--windowed");
    tauri::Builder::default()
        .manage(Host(Arc::new(Mutex::new(Bridge::new(env!("CARGO_PKG_VERSION"))))))
        .register_uri_scheme_protocol("nimbus-art", |_ctx, req| art(&req.uri().to_string()))
        .invoke_handler(tauri::generate_handler![nimbus_msg])
        .setup(move |app| {
            // The top bar's battery, Wi-Fi and volume, refreshed every 30 seconds (the UI asks too, after changes).
            let handle = app.handle().clone();
            let bridge = app.state::<Host>().0.clone();
            std::thread::spawn(move || loop {
                std::thread::sleep(std::time::Duration::from_secs(30));
                let max = bridge.lock().map(|b| b.max_volume()).unwrap_or(100);
                let _ = handle.emit("nimbus:msg", Bridge::event("status", hearth_core::system::status(max)));
            });
            WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title("Hearth")
                .fullscreen(!windowed)
                .inner_size(1280.0, 800.0)
                .background_color(tauri::window::Color(12, 16, 22, 255))
                .initialization_script(SHIM)
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Hearth couldn't start");
}
