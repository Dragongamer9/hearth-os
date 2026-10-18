//! Hearth (Rust host): puts Hearth's screens (ui/index.html, the same ones Ember uses on Windows) on screen
//! with the system's web view, and answers their messages with hearth-core. Replaces the Electron host.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod gamemode;

use gamemode::GameMode;
use hearth_core::bridge::Bridge;
use std::sync::{Arc, Mutex};
use tauri::http::{header::CONTENT_TYPE, Response, StatusCode};
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_dialog::DialogExt;

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
      // This window's messages: replies to its own calls, and events for every window.
      listening = tauri().then(T => T.webviewWindow.getCurrentWebviewWindow().listen('nimbus:msg', e => fn({ data: e.payload })));
    },
  };
})();
"#;

struct Host(Arc<Mutex<Bridge>>);

/// Game Mode's window handling (what's in front, the overlay), made once the app is up.
static GM: std::sync::OnceLock<Arc<GameMode>> = std::sync::OnceLock::new();
fn in_session() -> bool { hearth_core::gamescope::in_session() }

/// Where a sign-in window's answer goes (one sign-in at a time): the code, or None when it was closed.
static SIGNIN: Mutex<Option<std::sync::mpsc::Sender<Option<String>>>> = Mutex::new(None);
fn signin_answer(v: Option<String>) { if let Some(tx) = SIGNIN.lock().unwrap().take() { let _ = tx.send(v); } }

/// Xbox Cloud Gaming serves its best quality to Edge on Windows, so the stream window says it is one.
const EDGE_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/138.0.0.0 Safari/537.36 Edg/138.0.0.0";

/// A store's sign-in page in its own window. Blocks until it ends with a code, or is closed.
fn sign_in(app: &tauri::AppHandle, page: &hearth_core::stores::SignIn) -> Option<String> {
    let (tx, rx) = std::sync::mpsc::channel();
    signin_answer(None);   // a sign-in window left over from before: that one's done
    *SIGNIN.lock().unwrap() = Some(tx);
    if let Some(old) = app.get_webview_window("signin") { let _ = old.destroy(); }
    let done = page.done;
    // Works with just a controller: a pointer, an on-screen keyboard, B to cancel.
    let script = format!("{}\n{}", include_str!("js/signin-pad.js"), page.script.unwrap_or(""));
    let b = WebviewWindowBuilder::new(app, "signin", WebviewUrl::External(page.url.parse().ok()?))
        .title(page.title)
        .inner_size(900.0, 760.0)
        .center()
        .fullscreen(in_session())
        .incognito(true)
        .initialization_script(&script)
        .on_navigation(move |url| {
            if url.as_str().starts_with("hearth-signin://cancel") { signin_answer(None); return false; }
            match done(url.as_str()) { Some(code) => { signin_answer(Some(code)); false } None => true }
        });
    let w = match b.build() { Ok(w) => w, Err(e) => { hearth_core::bridge::log(&format!("sign-in window: {e}")); return None; } };
    w.on_window_event(|e| if let tauri::WindowEvent::Destroyed = e { signin_answer(None); });
    // gamescope keeps showing whichever window Hearth last put in front: put this one there.
    if let Some(gm) = GM.get() { gm.to_front(page.title); }
    let answer = rx.recv().ok().flatten();
    let _ = w.destroy();
    if let Some(gm) = GM.get() { gm.come_back(); }
    answer
}

/// Xbox Cloud Gaming in a full-screen window (its own sign-in kept between games), or closes it.
fn stream(app: &tauri::AppHandle, open: Option<(&str, &str)>, windowed: bool) {
    let old = app.get_webview_window("stream");
    let Some((url, title)) = open else { if let Some(w) = old { let _ = w.destroy(); } return };
    if let Some(w) = old { if let Ok(u) = url.parse() { let _ = w.navigate(u); } let _ = w.set_focus(); return; }
    let Ok(u) = url.parse() else { return };
    let r = WebviewWindowBuilder::new(app, "stream", WebviewUrl::External(u))
        .title(title)
        .fullscreen(!windowed)
        .inner_size(1280.0, 800.0)
        .user_agent(EDGE_UA)
        .data_directory(hearth_core::paths::data_dir().join("xcloud-web"))
        .initialization_script(include_str!("js/stream-exit.js"))
        .on_navigation(|url| {
            if url.as_str().starts_with("hearth-signin://stream-exit") {
                let gm = GM.get().cloned();
                std::thread::spawn(move || if let Some(gm) = gm { if let Some(w) = gm.app.get_webview_window("stream") { let _ = w.destroy(); } });
                return false;
            }
            true
        })
        .background_color(tauri::window::Color(0, 0, 0, 255))
        .build();
    match r {
        Ok(w) => {
            // WebRTC (the stream itself), media sources and protected media: off in WebKit unless turned on.
            #[cfg(target_os = "linux")]
            {
                let again = url.to_string();
                let _ = w.with_webview(move |wv| {
                    use webkit2gtk::{SettingsExt, WebViewExt};
                    if let Some(s) = wv.inner().settings() {
                        s.set_enable_webrtc(true);
                        s.set_enable_media_stream(true);
                        s.set_enable_mediasource(true);
                        s.set_enable_encrypted_media(true);
                        s.set_media_playback_requires_user_gesture(false);
                    }
                    wv.inner().load_uri(&again);
                });
            }
            w.on_window_event(|e| if let tauri::WindowEvent::Destroyed = e {
                hearth_core::stores::notify(serde_json::json!({"type": "internal", "what": "streamClosed"}));
            });
        }
        Err(e) => hearth_core::bridge::log(&format!("stream window: {e}")),
    }
}

/// A message from a page (Hearth's window or the overlay): handled off the window's thread (scanning the
/// library can take a moment). Replies go back to the window that asked.
#[tauri::command]
async fn nimbus_msg(app: tauri::AppHandle, window: tauri::WebviewWindow, host: tauri::State<'_, Host>, msg: serde_json::Value) -> Result<(), String> {
    let bridge = host.0.clone();
    let label = window.label().to_string();
    let ty = msg["type"].as_str().unwrap_or("").to_string();
    // Window things: the overlay opening and closing, back to the game.
    if let Some(gm) = GM.get() {
        let gm = gm.clone();
        if label == "overlay" && ty != "call" && ty != "log" {
            let m = msg.clone();
            let handled = tauri::async_runtime::spawn_blocking(move || gm.from_overlay(&m)).await.map_err(|e| e.to_string())?;
            if handled { return Ok(()); }
        } else if ty == "resume" {
            tauri::async_runtime::spawn_blocking(move || gm.resume_game()).await.map_err(|e| e.to_string())?;
            return Ok(());
        }
    }
    if ty == "ready" { let _ = app.emit_to(label.as_str(), "nimbus:msg", Bridge::event("padHost", serde_json::json!({"connected": 0}))); return Ok(()); }
    let (reply, quit) = tauri::async_runtime::spawn_blocking(move || {
        let mut b = match bridge.lock() { Ok(b) => b, Err(p) => p.into_inner() };
        if msg["type"] == "call" { (b.handle(&msg), false) } else { (None, b.other(&msg)) }
    })
    .await
    .map_err(|e| e.to_string())?;
    if let Some(r) = reply {
        app.emit_to(label.as_str(), "nimbus:msg", r).map_err(|e| e.to_string())?;
    }
    if quit {
        app.exit(0);   // the Hearth session starts Desktop Mode, or ends, when Hearth closes
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

/// nimbus-art://file/<path>: game art from the disk (Steam's covers, your own pictures); nimbus-art://cache/<name>:
/// web art downloaded to the device. Pictures only.
fn art(uri: &str) -> Response<Vec<u8>> {
    let path = if let Some(name) = uri.splitn(2, "://cache/").nth(1) {
        hearth_core::artcache::file(name.split('?').next().unwrap_or(name)).map(|p| p.to_string_lossy().to_string()).unwrap_or_default()
    } else {
        uri.splitn(2, "://file/").nth(1).map(|p| percent_decode(p.split('?').next().unwrap_or(p))).unwrap_or_default()
    };
    let ext = path.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    let mime = match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "avif" => "image/avif",
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

/// Game Mode: the overlay window, the Guide button, the power and volume buttons, and the settings that
/// only work in gamescope (frame-rate limit, performance overlay).
fn start_game_mode(app: tauri::AppHandle, bridge: Arc<Mutex<Bridge>>, windowed: bool) {
    let (guide, hold) = match bridge.lock() { Ok(b) => (b.guide.clone(), b.library.store.data["holdView"] == true), Err(_) => return };
    let gm = GameMode::new(app.clone(), bridge.clone(), guide.clone(), windowed);
    let _ = GM.set(gm.clone());
    guide.hold_view.store(hold, std::sync::atomic::Ordering::SeqCst);
    let (g1, g2) = (gm.clone(), gm.clone());
    guide.start(Arc::new(move |how: &str| { let (gm, how) = (g1.clone(), how.to_string()); std::thread::spawn(move || gm.guide_pressed(&how)); }),
                Arc::new(move |b: &str, down: bool| g2.overlay_pad(b, down)));
    if in_session() && !windowed {
        let g3 = gm.clone();
        let devices = bridge.lock().map(|b| b.buttons.clone()).ok();
        if let Some(d) = devices { hearth_core::guide::start_buttons(Arc::new(move |m| g3.button(&m)), d); }
        let (fps, perf) = bridge.lock().map(|b| (b.library.store.data["fpsLimit"].as_u64().unwrap_or(0), b.library.store.data["perfOverlay"].as_u64().unwrap_or(0))).unwrap_or((0, 0));
        if fps > 0 { let _ = hearth_core::gamescope::set_fps_limit(fps); }
        if perf > 0 { let _ = hearth_core::gamescope::set_perf_overlay(perf); }
    }
    // Keyboard backup for the Guide button, like Windows: Ctrl+Alt+Q (works while the game runs under X11 /
    // Xwayland, as all do in gamescope).
    {
        use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
        let g = gm.clone();
        let plugin = tauri_plugin_global_shortcut::Builder::new()
            .with_handler(move |_app, _sc, event| { if event.state() == ShortcutState::Pressed { let g = g.clone(); std::thread::spawn(move || g.guide_pressed("hotkey")); } })
            .build();
        if app.plugin(plugin).is_ok() {
            if let Err(e) = app.global_shortcut().register(Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyQ)) { hearth_core::bridge::log(&format!("Ctrl+Alt+Q: {e}")); }
        }
    }
    // "Ask for my password after sleep" in Game Mode: locked as the device goes to sleep, so it's locked when it
    // wakes; on waking, Hearth comes to the front with the lock screen. (logind says when through D-Bus.)
    if in_session() && gm.inside() {
        let g = gm.clone();
        std::thread::spawn(move || {
            use std::io::BufRead;
            let Ok(mut child) = std::process::Command::new("gdbus").args(["monitor", "--system", "--dest", "org.freedesktop.login1", "--object-path", "/org/freedesktop/login1"])
                .stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::null()).spawn() else { return };
            let Some(out) = child.stdout.take() else { return };
            let mut locked = false;
            for line in std::io::BufReader::new(out).lines().map_while(Result::ok) {
                if !line.contains("PrepareForSleep") { continue; }
                if line.contains("(true") {
                    if hearth_core::admin::login()["lockOnWake"] == true { locked = true; let _ = g.app.emit_to("main", "nimbus:msg", Bridge::event("lock", serde_json::json!({}))); }
                } else if locked {
                    locked = false;
                    g.hide_overlay(); g.come_back();
                    let _ = g.app.emit_to("main", "nimbus:msg", Bridge::event("lock", serde_json::json!({})));
                }
            }
        });
    }
    // Quick Access over a running game: a see-through window gamescope draws on top.
    if gm.overlay_available() {
        let size = app.primary_monitor().ok().flatten().map(|m| { let s = m.size(); let f = m.scale_factor(); (s.width as f64 / f, s.height as f64 / f) }).unwrap_or((1280.0, 800.0));
        let built = WebviewWindowBuilder::new(&app, "overlay", WebviewUrl::App("index.html?overlay=full".into()))
            .title(gamemode::OVERLAY_TITLE)
            .transparent(true)
            .decorations(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .position(0.0, 0.0)
            .inner_size(size.0, size.1)
            .visible(false)
            .background_color(tauri::window::Color(0, 0, 0, 0))
            .initialization_script(SHIM)
            .build();
        match built {
            Ok(_) => { let g = gm.clone(); std::thread::spawn(move || { std::thread::sleep(std::time::Duration::from_secs(3)); g.arm_overlay(); }); }
            Err(e) => hearth_core::bridge::log(&format!("overlay: couldn't make its window: {e}")),
        }
    }
    // Desktop Mode couldn't start and sent us back here: say why.
    let h = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(2500));
        let f = std::path::PathBuf::from(std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into())).join("nimbus-desktop-failed");
        if let Ok(text) = std::fs::read_to_string(&f) {
            let _ = std::fs::remove_file(&f);
            hearth_core::bridge::log(&format!("desktop mode failed: {text}"));
            let t: String = text.trim().chars().take(1500).collect();
            let _ = h.emit("nimbus:msg", Bridge::event("desktopFailed", serde_json::json!({"text": t})));
        }
    });
    // Updates to the app: looked for 20 seconds after starting, then every 6 hours (when turned on).
    let upd = bridge.lock().map(|b| b.app_updates.clone()).ok();
    if let Some(upd) = upd {
        let b = bridge.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(20));
            loop {
                let auto = b.lock().map(|x| x.library.store.data["autoUpdateCheck"] != false).unwrap_or(false);
                if auto { upd.check(true); }
                std::thread::sleep(std::time::Duration::from_secs(6 * 3600));
            }
        });
    }
}

fn main() {
    // WebKit's newer GPU path draws a black window on some Nvidia drivers: the older one works everywhere.
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }
    let windowed = std::env::args().any(|a| a == "--windowed");
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(Host(Arc::new(Mutex::new(Bridge::new(env!("CARGO_PKG_VERSION"))))))
        .register_uri_scheme_protocol("nimbus-art", |_ctx, req| art(&req.uri().to_string()))
        // Epic's sign-in page hands its code over as hearth-signin://done?code=… (if the window lets it through).
        .register_uri_scheme_protocol("hearth-signin", |_ctx, req| {
            let u = req.uri().to_string();
            if u.contains("stream-exit") { if let Some(gm) = GM.get() { if let Some(w) = gm.app.get_webview_window("stream") { let _ = w.destroy(); } } }
            else { signin_answer(hearth_core::http::query_param(&u, "code")); }
            Response::builder().status(StatusCode::OK).body(Vec::new()).unwrap()
        })
        .invoke_handler(tauri::generate_handler![nimbus_msg])
        .setup(move |app| {
            // The top bar's battery, Wi-Fi and volume, refreshed every 30 seconds (the UI asks too, after changes).
            let handle = app.handle().clone();
            let bridge = app.state::<Host>().0.clone();
            // Events from the host's own work (Bluetooth found a device, a controller paired...) go to the page.
            {
                let h = handle.clone();
                let picker = handle.clone();
                let folder = handle.clone();
                let signer = handle.clone();
                let streamer = handle.clone();
                let br = bridge.clone();
                if let Ok(mut b) = bridge.lock() {
                    let emit: hearth_core::system::bt_scan::Emit = Arc::new(move |v| {
                        // The host's background work (a download finished, a store list came in): handled by the
                        // bridge on its own thread, since whoever sent it may be holding the bridge right now.
                        if v["type"] == "internal" {
                            let (br, h) = (br.clone(), h.clone());
                            std::thread::spawn(move || {
                                let out = match br.lock() { Ok(mut b) => b.internal(&v), Err(p) => p.into_inner().internal(&v) };
                                for e in out { let _ = h.emit("nimbus:msg", e); }
                            });
                            return;
                        }
                        // The host asks for a restart (after restoring a backup): Hearth reads everything at start.
                        // Game Mode restarts with new options (the session starts it again when Hearth closes).
                        if v["type"] == "quit" { h.exit(0); return; }
                        // The volume bar and messages: over the game when one is in front.
                        if v["name"] == "osd" { if let Some(gm) = GM.get() { let (gm, d) = (gm.clone(), v["data"].clone()); std::thread::spawn(move || gm.osd(d)); return; } }
                        // In the Hearth session, the session's watchdog starts Hearth again right away (exit code 75).
                        if v["type"] == "restart" && in_session() { h.exit(75); return; }
                        if v["type"] == "restart" {
                            let h2 = h.clone();
                            std::thread::spawn(move || { std::thread::sleep(std::time::Duration::from_millis(1500)); h2.restart(); });
                            return;
                        }
                        let _ = h.emit("nimbus:msg", v);
                    });
                    hearth_core::stores::set_notify(emit.clone());
                    b.emit = emit;
                    b.pick_folder = Arc::new(move |title: &str, start: &str| {
                        if let Some(gm) = GM.get() { gm.before_dialog(); }
                        let r = folder.dialog().file().set_title(title).set_directory(start).blocking_pick_folder().and_then(|f| f.into_path().ok()).map(|p| p.to_string_lossy().to_string());
                        if let Some(gm) = GM.get() { gm.come_back(); }
                        r
                    });
                    b.sign_in = Arc::new(move |page| sign_in(&signer, page));
                    b.stream = Arc::new(move |open| stream(&streamer, open, windowed));
                    let zoomer = handle.clone();
                    b.zoom = Arc::new(move |pct| { if let Some(w) = zoomer.get_webview_window("main") { let _ = w.set_zoom(pct as f64 / 100.0); } });
                    // First start after installing: a look for games on other drives, offered once (after 90 s).
                    std::thread::spawn(|| { std::thread::sleep(std::time::Duration::from_secs(90)); hearth_core::stores::notify(serde_json::json!({"type": "internal", "what": "importOffer"})); });
                    // Store game lists older than 12 hours are fetched again, in the background.
                    hearth_core::stores::refresh_stale(&b.library.accounts);
                    b.pick_file = Arc::new(move |title: &str| {
                        if let Some(gm) = GM.get() { gm.before_dialog(); }
                        let r = picker.dialog().file().set_title(title).blocking_pick_file().and_then(|f| f.into_path().ok()).map(|p| p.to_string_lossy().to_string());
                        if let Some(gm) = GM.get() { gm.come_back(); }
                        r
                    });
                    b.host = Arc::new(|what: &str, a: &serde_json::Value| { if let Some(gm) = GM.get() { gm.host_action(what, a); } });
                }
            }
            // The running game, every 2 seconds: started, closed, handed off to the real game (launchers).
            {
                let h = handle.clone();
                let b = bridge.clone();
                std::thread::spawn(move || {
                    let mut n: u64 = 0;
                    loop {
                        std::thread::sleep(std::time::Duration::from_secs(2));
                        n += 1;
                        let (events, extra) = match b.lock() {
                            Ok(mut br) => {
                                let ev = br.library.tick();
                                let mut extra = br.after_events(&ev);
                                if n % 10 == 0 { extra.extend(br.parental_tick()); }   // every 20 seconds
                                (ev, extra)
                            }
                            Err(_) => (vec![], vec![]),
                        };
                        for e in &events { let _ = h.emit("nimbus:msg", e.clone()); }
                        if let Some(gm) = GM.get() { for e in &events { gm.on_library_event(e); } }
                        for e in extra {
                            match (GM.get(), e["name"].as_str()) {
                                (Some(gm), Some("osd")) => gm.osd(e["data"].clone()),
                                _ => { let _ = h.emit("nimbus:msg", e); }
                            }
                        }
                    }
                });
            }
            std::thread::spawn(move || loop {
                std::thread::sleep(std::time::Duration::from_secs(30));
                let max = bridge.lock().map(|b| b.max_volume()).unwrap_or(100);
                let _ = handle.emit("nimbus:msg", Bridge::event("status", hearth_core::system::status(max)));
            });
            let zoom = app.state::<Host>().0.lock().map(|b| b.zoom_pct()).unwrap_or(100);
            let main = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title("Hearth")
                .fullscreen(!windowed)
                .inner_size(1280.0, 800.0)
                .background_color(tauri::window::Color(12, 16, 22, 255))
                .initialization_script(SHIM)
                .build()?;
            if zoom != 100 { let _ = main.set_zoom(zoom as f64 / 100.0); }
            // As the session there's nothing behind Hearth, so Alt+F4 must not close it by accident.
            main.on_window_event(|e| if let tauri::WindowEvent::CloseRequested { api, .. } = e { if in_session() { api.prevent_close(); } });
            start_game_mode(app.handle().clone(), app.state::<Host>().0.clone(), windowed);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Hearth couldn't start");
}
