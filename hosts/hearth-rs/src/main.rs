//! Hearth (Rust host): puts Hearth's screens (ui/index.html, the same ones Ember uses on Windows) on screen
//! with the system's web view, and answers their messages with hearth-core. Replaces the Electron host.
//! On Nvidia graphics it hands the drawing to Chromium instead: it starts Hearth's Electron window app
//! (hosts/electron), which runs this same program windowless (`--core-stdio`) for everything else (winhost.rs,
//! core_ipc.rs, docs/RENDERERS.md). One core, two ways to draw it.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod core_ipc;
mod gamemode;
mod winhost;

use gamemode::GameMode;
use winhost::WindowHost;
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
pub(crate) static GM: std::sync::OnceLock<Arc<GameMode>> = std::sync::OnceLock::new();
pub(crate) fn in_session() -> bool { hearth_core::gamescope::in_session() }
/// A windowed Hearth (Desktop Mode, or another Linux desktop): its windows are ordinary desktop windows.
fn desktop_window() -> bool { hearth_core::signin_handoff::windowed() || !in_session() }
/// A sign-in page gets the controller helper (pointer, on-screen keyboard, B to cancel) in Game Mode.
pub(crate) fn signin_pad_wanted() -> bool { !desktop_window() }

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
    // In Game Mode it works with just a controller: a pointer, an on-screen keyboard, B to cancel. In Desktop Mode
    // (a windowed Hearth) there's a real mouse and keyboard, and the controller already moves the desktop's pointer.
    let pad = if desktop_window() { "" } else { include_str!("js/signin-pad.js") };
    // The page itself never sees the controller (Epic's and Microsoft's react to it on their own: random clicks).
    let script = format!("{}\n{}\n{}", include_str!("js/signin-nogamepad.js"), pad, page.script.unwrap_or(""));
    let b = WebviewWindowBuilder::new(app, "signin", WebviewUrl::External(page.url.parse().ok()?))
        .title(page.title)
        .inner_size(900.0, 760.0)
        .center()
        // Full screen in Game Mode; in Desktop Mode (a windowed Hearth) an ordinary window.
        .fullscreen(in_session() && !hearth_core::signin_handoff::windowed())
        // The store's own browser profile, shared with Game Mode › Store's store window: signing in here signs that
        // in too (and the other way round), so it's one sign-in. Signing out removes it (storefront::reset_web_profile).
        .data_directory(hearth_core::storefront::web_profile(page.store, "webkit"))
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
        .initialization_script(if windowed { concat!(include_str!("js/stream-exit.js"), "\n", include_str!("js/stream-signin.js")) }
            else { concat!(include_str!("js/stream-exit.js"), "\n", include_str!("js/stream-signin.js"), "\n", include_str!("js/signin-pad.js")) })
        .on_navigation(|url| {
            if url.as_str().starts_with("hearth-signin://stream-exit") {
                let gm = GM.get().cloned();
                std::thread::spawn(move || if let Some(gm) = gm { gm.host.destroy("stream"); });
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

/// Game Mode › Store's store window: a store's own page (GOG, Epic, Xbox) for buying and wishlists, signed in with
/// the browser profile its sign-in window uses (Library › Stores). Full screen with the controller helper in Game
/// Mode (B closes it), an ordinary window on the desktop. Closing it tells the core, which refreshes that store's
/// games (a purchase shows in the library).
fn store_web(app: &tauri::AppHandle, page: Option<&hearth_core::storefront::WebPage>) {
    let old = app.get_webview_window("storeweb");
    let Some(p) = page else { if let Some(w) = old { let _ = w.destroy(); } return };
    let Ok(u) = p.url.parse() else { return };
    if let Some(w) = old {
        // Already open for this store: go to the new page. Another store's sign-in can't be reused: open again.
        if STORE_WEB.lock().unwrap().as_deref() == Some(p.store.as_str()) { let _ = w.navigate(u); let _ = w.set_focus(); return; }
        let _ = w.destroy();
    }
    *STORE_WEB.lock().unwrap() = Some(p.store.clone());
    let pad = if desktop_window() { "" } else { include_str!("js/signin-pad.js") };
    let script = format!("{}\n{}\n{}", include_str!("js/signin-nogamepad.js"), include_str!("js/storeweb.js"), pad);
    let store = p.store.clone();
    let closer = app.clone();
    let r = WebviewWindowBuilder::new(app, "storeweb", WebviewUrl::External(u))
        .title(&p.title)
        .inner_size(1180.0, 800.0)
        .center()
        .fullscreen(in_session() && !hearth_core::signin_handoff::windowed())
        .data_directory(p.profile_for("webkit"))
        .initialization_script(&script)
        .on_navigation(move |url| {
            // B (the controller helper) leaves through this address; only web pages load here otherwise.
            if url.as_str().starts_with("hearth-signin://cancel") {
                let app = closer.clone();
                std::thread::spawn(move || if let Some(w) = app.get_webview_window("storeweb") { let _ = w.destroy(); });
                return false;
            }
            matches!(url.scheme(), "https" | "about" | "blob" | "data")
        })
        .build();
    match r {
        Ok(w) => {
            w.on_window_event(move |e| if let tauri::WindowEvent::Destroyed = e {
                hearth_core::stores::notify(serde_json::json!({"type": "internal", "what": "storeWebClosed", "store": store}));
                if let Some(gm) = GM.get() { gm.come_back(); }
            });
            if let Some(gm) = GM.get() { gm.to_front(&p.title); }
        }
        Err(e) => hearth_core::bridge::log(&format!("store window: {e}")),
    }
}
/// The store the store window is signed in as (its browser profile).
static STORE_WEB: Mutex<Option<String>> = Mutex::new(None);

/// Hearth's windows in this process (WebKit, through Tauri).
struct TauriHost { app: tauri::AppHandle }

impl WindowHost for TauriHost {
    fn emit_to(&self, label: &str, v: serde_json::Value) { let _ = self.app.emit_to(label, "nimbus:msg", v); }
    fn emit_all(&self, v: serde_json::Value) { let _ = self.app.emit("nimbus:msg", v); }
    fn exists(&self, label: &str) -> bool { self.app.get_webview_window(label).is_some() }
    fn show(&self, label: &str) { if let Some(w) = self.app.get_webview_window(label) { let _ = w.show(); } }
    fn hide(&self, label: &str) { if let Some(w) = self.app.get_webview_window(label) { let _ = w.hide(); } }
    fn focus(&self, label: &str) { if let Some(w) = self.app.get_webview_window(label) { let _ = w.set_focus(); } }
    fn unminimize(&self, label: &str) { if let Some(w) = self.app.get_webview_window(label) { let _ = w.unminimize(); } }
    fn is_visible(&self, label: &str) -> bool { self.app.get_webview_window(label).map(|w| w.is_visible().unwrap_or(false)).unwrap_or(false) }
    fn is_focused(&self, label: &str) -> bool { self.app.get_webview_window(label).map(|w| w.is_focused().unwrap_or(false)).unwrap_or(false) }
    fn title(&self, label: &str) -> Option<String> { self.app.get_webview_window(label).and_then(|w| w.title().ok()) }
    fn eval(&self, label: &str, js: &str) { if let Some(w) = self.app.get_webview_window(label) { let _ = w.eval(js); } }
    fn destroy(&self, label: &str) { if let Some(w) = self.app.get_webview_window(label) { let _ = w.destroy(); } }
    fn exit(&self, code: i32) { self.app.exit(code); }
    fn restart(&self) { self.app.restart(); }
    fn pick_file(&self, title: &str) -> Option<String> {
        self.app.dialog().file().set_title(title).blocking_pick_file().and_then(|f| f.into_path().ok()).map(|p| p.to_string_lossy().to_string())
    }
    fn pick_folder(&self, title: &str, start: &str) -> Option<String> {
        self.app.dialog().file().set_title(title).set_directory(start).blocking_pick_folder().and_then(|f| f.into_path().ok()).map(|p| p.to_string_lossy().to_string())
    }
    fn sign_in(&self, page: &hearth_core::stores::SignIn) -> Option<String> { sign_in(&self.app, page) }
    fn stream(&self, open: Option<(&str, &str)>, windowed: bool) { stream(&self.app, open, windowed) }
    fn store_web(&self, page: Option<&hearth_core::storefront::WebPage>) { store_web(&self.app, page) }
    fn zoom(&self, pct: u32) { if let Some(w) = self.app.get_webview_window("main") { let _ = w.set_zoom(pct as f64 / 100.0); } }
    fn create_overlay(&self) -> bool {
        let app = &self.app;
        let size = app.primary_monitor().ok().flatten().map(|m| { let s = m.size(); let f = m.scale_factor(); (s.width as f64 / f, s.height as f64 / f) }).unwrap_or((1280.0, 800.0));
        let built = WebviewWindowBuilder::new(app, "overlay", WebviewUrl::App("index.html?overlay=full".into()))
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
            Ok(w) => { gpu_always(&w); true }
            Err(e) => { hearth_core::bridge::log(&format!("overlay: couldn't make its window: {e}")); false }
        }
    }
    fn register_hotkey(&self, on: winhost::Hotkey) {
        use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
        let plugin = tauri_plugin_global_shortcut::Builder::new()
            .with_handler(move |_app, _sc, event| { if event.state() == ShortcutState::Pressed { let on = on.clone(); std::thread::spawn(move || on()); } })
            .build();
        if self.app.plugin(plugin).is_ok() {
            if let Err(e) = self.app.global_shortcut().register(Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyQ)) { hearth_core::bridge::log(&format!("Ctrl+Alt+Q: {e}")); }
        }
    }
}

/// To every window, and to HHD's overlay app when it's connected.
pub(crate) fn broadcast(host: &Arc<dyn WindowHost>, v: serde_json::Value) {
    hearth_core::overlay_link::send(&v);
    host.emit_all(v);
}
/// To one window. "overlay" is also Handheld Daemon's overlay app on the HHD editions (Quick Access over games),
/// reached through hearth-core's overlay socket (nothing happens there when it isn't connected).
pub(crate) fn send_to(host: &Arc<dyn WindowHost>, label: &str, v: serde_json::Value) {
    if label == "overlay" { hearth_core::overlay_link::send(&v); }
    host.emit_to(label, v);
}

/// A message from a page (Hearth's window or the overlay), through Tauri.
#[tauri::command]
async fn nimbus_msg(window: tauri::WebviewWindow, host: tauri::State<'_, Host>, msg: serde_json::Value) -> Result<(), String> {
    let Some(wh) = HOST.get().cloned() else { return Ok(()) };
    let (bridge, label) = (host.0.clone(), window.label().to_string());
    tauri::async_runtime::spawn_blocking(move || page_msg(&wh, bridge, label, msg)).await.map_err(|e| e.to_string())
}

/// This process's window host, once it's made.
pub(crate) static HOST: std::sync::OnceLock<Arc<dyn WindowHost>> = std::sync::OnceLock::new();
/// The WebKit app (Tauri), whose built-in copy of Hearth's screens HHD's overlay app is given (overlay_hello).
static UI_APP: std::sync::OnceLock<tauri::AppHandle> = std::sync::OnceLock::new();

/// What HHD's overlay app is told when it connects: this Hearth's version and where its screens come from, so Quick
/// Access over games is the page this Hearth draws. WebKit: Hearth gives the files itself (ui_file_for); Chromium
/// (the Electron window app draws): that app's folder.
fn overlay_hello() -> serde_json::Value {
    let mut h = serde_json::json!({"type": "overlay-host", "what": "hello", "version": env!("CARGO_PKG_VERSION")});
    if UI_APP.get().is_some() { h["ui"] = serde_json::json!("hearth"); }
    else {
        let app = std::env::var("HEARTH_ELECTRON_APP").map(std::path::PathBuf::from).unwrap_or_else(|_| std::path::PathBuf::from(hearth_core::renderer::ELECTRON_APP));
        if let Some(dir) = app.parent().map(|d| d.join("resources").join("ui")).filter(|d| d.join("index.html").is_file()) { h["uiDir"] = serde_json::json!(dir.to_string_lossy()); }
    }
    h
}
/// One file of the WebKit app's built-in screens, for HHD's overlay app.
fn ui_file_for(path: &str) -> Option<(Vec<u8>, String)> {
    let app = UI_APP.get()?;
    let a = app.asset_resolver().get(path.to_string())?;
    Some((a.bytes().to_vec(), a.mime_type().to_string()))
}

/// A message from a page: Hearth's window, the overlay window, or HHD's overlay app (as "overlay", through the overlay
/// socket). Run off the window's thread (scanning the library can take a moment). Replies go back to whoever asked.
pub(crate) fn page_msg(host: &Arc<dyn WindowHost>, bridge: Arc<Mutex<Bridge>>, label: String, msg: serde_json::Value) {
    let ty = msg["type"].as_str().unwrap_or("").to_string();
    // Window things: the overlay opening and closing, back to the game.
    if let Some(gm) = GM.get() {
        if label == "overlay" && ty != "call" && ty != "log" {
            if gm.from_overlay(&msg) { return; }
        } else if ty == "overlay" && msg["action"] == "open" {
            // Hearth's starting screen asks for Quick Access: the overlay over it (gamemode.rs open_from_hearth).
            gm.open_from_hearth();
            return;
        } else if ty == "resume" {
            // Hearth's own page: not while Quick Access is open over the game (gamemode.rs front_decision).
            gm.resume_from_main();
            return;
        } else if ty == "ready" && label == "main" {
            // Hearth's page (re)loaded: told again whether it may act on the controller (gamemode.rs main_page_reads_pad).
            gm.main_ready();
        }
    }
    if ty == "overlay" && msg["action"] == "open" { send_to(host, &label, Bridge::event("overlay.unavailable", serde_json::json!({}))); return; }
    if ty == "ready" { send_to(host, &label, Bridge::event("padHost", serde_json::json!({"connected": 0}))); return; }
    let (reply, quit) = {
        let mut b = match bridge.lock() { Ok(b) => b, Err(p) => p.into_inner() };
        // The fail-safe watches what holds the bridge (hearth_core::watchdog).
        let n = hearth_core::watchdog::enter(msg["method"].as_str().or(msg["type"].as_str()).unwrap_or("?"));
        b.cur_label = label.clone();
        let r = if msg["type"] == "call" { (b.handle(&msg), false) } else { (None, b.other(&msg)) };
        hearth_core::watchdog::leave(n);
        r
    };
    if let Some(r) = reply { send_to(host, &label, r); }
    if quit { host.exit(0); }   // the Hearth session starts Desktop Mode, or ends, when Hearth closes
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            // Bytes, not &s[..]: a '%' before a non-ASCII letter in a file name cut the letter in half and panicked.
            if let Some(v) = hearth_core::http::hex_byte(b[i + 1], b[i + 2]) {
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
/// web art downloaded to the device. Pictures only: (file, its type), or None.
pub(crate) fn art_path(uri: &str) -> Option<(String, &'static str)> {
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
        _ => return None,
    };
    if path.is_empty() || !std::path::Path::new(&path).is_file() { return None; }
    Some((path, mime))
}

fn art(uri: &str) -> Response<Vec<u8>> {
    match art_path(uri).and_then(|(p, m)| std::fs::read(&p).ok().map(|b| (b, m))) {
        None => Response::builder().status(StatusCode::NOT_FOUND).body(Vec::new()).unwrap(),
        Some((bytes, m)) => Response::builder()
            .header(CONTENT_TYPE, m)
            // Kept by the web view, so drawing the library again doesn't read every picture again.
            .header("Cache-Control", "max-age=86400")
            .header("Access-Control-Allow-Origin", "*")
            .body(bytes)
            .unwrap(),
    }
}

/// Everything behind Hearth's windows, whoever draws them: the bridge's hooks and events, the fail-safe, the game
/// watcher, the status bar, then Game Mode.
pub(crate) fn start_core(host: Arc<dyn WindowHost>, bridge: Arc<Mutex<Bridge>>, windowed: bool) {
    let _ = HOST.set(host.clone());
    // Events from the host's own work (Bluetooth found a device, a controller paired...) go to the page.
    {
        let h = host.clone();
        let br = bridge.clone();
        if let Ok(mut b) = bridge.lock() {
            let emit: hearth_core::system::bt_scan::Emit = Arc::new(move |v| {
                // The host's background work (a download finished, a store list came in): handled by the
                // bridge on its own thread, since whoever sent it may be holding the bridge right now.
                if v["type"] == "internal" {
                    let (br, h) = (br.clone(), h.clone());
                    std::thread::spawn(move || {
                        let out = match br.lock() { Ok(mut b) => b.internal(&v), Err(p) => p.into_inner().internal(&v) };
                        for e in out {
                            // An answer for one window (internal's replies carry "to").
                            if e["type"] == "result" { if let Some(to) = e["to"].as_str().map(str::to_string) { send_to(&h, &to, e); continue; } }
                            broadcast(&h, e);
                        }
                    });
                    return;
                }
                // An answer given later (bridge.rs `later`): only to the window that asked.
                if v["type"] == "result" { if let Some(to) = v["to"].as_str().map(str::to_string) { send_to(&h, &to, v); return; } }
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
                broadcast(&h, v);
            });
            hearth_core::stores::set_notify(emit.clone());
            b.emit = emit;
            let folder = host.clone();
            b.pick_folder = Arc::new(move |title: &str, start: &str| {
                if let Some(gm) = GM.get() { gm.before_dialog(); }
                let r = folder.pick_folder(title, start);
                if let Some(gm) = GM.get() { gm.come_back(); }
                r
            });
            let signer = host.clone();
            b.sign_in = Arc::new(move |page| signer.sign_in(page));
            let streamer = host.clone();
            b.stream = Arc::new(move |open| streamer.stream(open, windowed));
            // Game Mode › Store's store window (storefront.rs).
            let web = host.clone();
            hearth_core::storefront::set_web_opener(Arc::new(move |page: Option<hearth_core::storefront::WebPage>| web.store_web(page.as_ref())));
            let zoomer = host.clone();
            b.zoom = Arc::new(move |pct| zoomer.zoom(pct));
            // First start after installing: a look for games on other drives, offered once (after 90 s).
            std::thread::spawn(|| { std::thread::sleep(std::time::Duration::from_secs(90)); hearth_core::stores::notify(serde_json::json!({"type": "internal", "what": "importOffer"})); });
            // Store game lists older than 12 hours are fetched again, in the background.
            hearth_core::stores::refresh_stale(&b.library.accounts);
            let picker = host.clone();
            b.pick_file = Arc::new(move |title: &str| {
                if let Some(gm) = GM.get() { gm.before_dialog(); }
                let r = picker.pick_file(title);
                if let Some(gm) = GM.get() { gm.come_back(); }
                r
            });
            b.host = Arc::new(|what: &str, a: &serde_json::Value| { if let Some(gm) = GM.get() { gm.host_action(what, a); } });
        }
    }
    // The fail-safe: Hearth stuck on something gets it stopped, a report sent, and if need be a restart.
    {
        let (version, auto) = match bridge.lock() { Ok(b) => (b.version.clone(), b.auto_report()), Err(_) => (String::new(), true) };
        hearth_core::watchdog::set_auto_send(auto);
        let (h, h2) = (host.clone(), host.clone());
        hearth_core::watchdog::start(version, Bridge::edition_name(),
            Arc::new(move |v: serde_json::Value| broadcast(&h, v)),
            Arc::new(move || if in_session() { h2.exit(75) } else { h2.restart() }));
    }
    // The running game, every 2 seconds: started, closed, handed off to the real game (launchers).
    {
        let h = host.clone();
        let b = bridge.clone();
        std::thread::spawn(move || {
            let mut n: u64 = 0;
            loop {
                std::thread::sleep(std::time::Duration::from_secs(2));
                n += 1;
                let (events, extra) = match b.lock() {
                    Ok(mut br) => {
                        let w = hearth_core::watchdog::enter("library.tick");
                        let ev = br.library.tick();
                        let mut extra = br.after_events(&ev);
                        if n % 10 == 0 { extra.extend(br.parental_tick()); }   // every 20 seconds
                        hearth_core::watchdog::leave(w);
                        (ev, extra)
                    }
                    Err(_) => (vec![], vec![]),
                };
                for e in &events { broadcast(&h, e.clone()); }
                if let Some(gm) = GM.get() { for e in &events { gm.on_library_event(e); } }
                for e in extra {
                    match (GM.get(), e["name"].as_str()) {
                        (Some(gm), Some("osd")) => gm.osd(e["data"].clone()),
                        _ => { broadcast(&h, e); }
                    }
                }
            }
        });
    }
    // The top bar's battery, Wi-Fi and volume, refreshed every 30 seconds (the UI asks too, after changes).
    {
        let (h, b) = (host.clone(), bridge.clone());
        std::thread::spawn(move || loop {
            std::thread::sleep(std::time::Duration::from_secs(30));
            let max = b.lock().map(|b| b.max_volume()).unwrap_or(100);
            broadcast(&h, Bridge::event("status", hearth_core::system::status(max)));
        });
    }
    // A windowed Hearth that's open in Desktop Mode: a store sign-in asked for in Game Mode is handed to it
    // (desktop/shell/hearth-signin) instead of opening a second Hearth.
    if windowed {
        let h = host.clone();
        std::thread::spawn(move || loop {
            std::thread::sleep(std::time::Duration::from_millis(1500));
            let Some(store) = hearth_core::signin_handoff::take_route() else { continue };
            hearth_core::bridge::log(&format!("accounts: {store} sign-in handed to this window"));
            if h.exists("main") { h.unminimize("main"); h.show("main"); h.focus("main"); }
            h.emit_to("main", Bridge::event("signinStart", serde_json::json!({"store": store})));
        });
    }
    // A windowed Hearth that's open in Desktop Mode: a game started from Mantel's Games section is handed to it
    // (desktop/shell/hearth-games) and played the way the library's Play button does.
    if windowed {
        let h = host.clone();
        std::thread::spawn(move || loop {
            std::thread::sleep(std::time::Duration::from_millis(1000));
            let Some(id) = hearth_core::desktop_games::take_route() else { continue };
            hearth_core::bridge::log(&format!("desktop: {id} started from Mantel's Games section"));
            if h.exists("main") { h.unminimize("main"); h.show("main"); h.focus("main"); }
            h.emit_to("main", Bridge::event("play", serde_json::json!({"id": id})));
        });
    }
    start_game_mode(host, bridge, windowed);
}

/// Game Mode: the overlay window, the Guide button, the power and volume buttons, and the settings that
/// only work in gamescope (frame-rate limit, performance overlay).
fn start_game_mode(host: Arc<dyn WindowHost>, bridge: Arc<Mutex<Bridge>>, windowed: bool) {
    let (guide, hold) = match bridge.lock() { Ok(b) => (b.guide.clone(), b.library.store.data["holdView"] == true), Err(_) => return };
    let gm = GameMode::new(host.clone(), bridge.clone(), guide.clone(), windowed);
    {
        // The controller mouse pauses while Hearth or its Quick Access is in front (the controller is Hearth's then).
        let gm2 = gm.clone();
        hearth_core::padmouse::set_pause_probe(std::sync::Arc::new(move || gm2.overlay_on.load(std::sync::atomic::Ordering::SeqCst) || gm2.hearth_in_front()));
    }
    let _ = GM.set(gm.clone());
    // Moved from the old Electron AppImage: tidy up once this version has run fine for 5 minutes.
    std::thread::spawn(|| { std::thread::sleep(std::time::Duration::from_secs(300)); hearth_core::appupdate::tidy_after_move(); });
    // Off the window's thread: talking to InputPlumber can take a few seconds.
    { let g = gm.clone(); std::thread::spawn(move || g.start_inputplumber()); }
    // Hearth Input: each game's controller layout (keyboard and mouse from the controller) while it's in front.
    gm.start_input_map();
    // A new display mode on trial (Settings › Display): switched back by itself unless it's kept within 20 seconds,
    // so a screen that can't show it doesn't stay black.
    {
        let b = bridge.clone();
        hearth_core::display::watch_pending(std::sync::Arc::new(move || { b.lock().unwrap_or_else(|e| e.into_inner()).restart_game_mode(); }));
    }
    // Every window carries its game's number (or Hearth's), and gamescope follows them (Quick Resume shows the
    // game asked for, never another one's frozen picture).
    gm.start_window_ids();
    guide.hold_view.store(hold, std::sync::atomic::Ordering::SeqCst);
    let (g1, g2) = (gm.clone(), gm.clone());
    guide.start(Arc::new(move |how: &str| { let (gm, how) = (g1.clone(), how.to_string()); std::thread::spawn(move || gm.guide_pressed(&how)); }),
                Arc::new(move |b: &str, down: bool| { g2.overlay_pad(b, down); g2.seat_pad(b, down); }));
    // Multi-seat window: every button comes through Hearth, not only while the overlay is open (see seat_pad).
    if hearth_core::multiplay::seat_hosted() { guide.buttons.store(true, std::sync::atomic::Ordering::SeqCst); }
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
    { let g = gm.clone(); host.register_hotkey(Arc::new(move || g.guide_pressed("hotkey"))); }
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
                if !line.contains("(true") {
                    // Awake: the power limit you chose, set again (the handheld's own one is back after sleep on
                    // many, ryzenadj's on the ROG Ally). A few seconds first, for the firmware to settle.
                    std::thread::spawn(|| {
                        std::thread::sleep(std::time::Duration::from_secs(5));
                        match hearth_core::power::restore() {
                            Ok(Some(t)) => hearth_core::bridge::log(&format!("power: limit set again after waking ({t})")),
                            Ok(None) => {}
                            Err(e) => hearth_core::bridge::log(&format!("power: couldn't set the limit again after waking: {e}")),
                        }
                    });
                }
                if line.contains("(true") {
                    if hearth_core::admin::login()["lockOnWake"] == true { locked = true; g.to_main(Bridge::event("lock", serde_json::json!({}))); }
                } else if locked {
                    locked = false;
                    g.hide_overlay(); g.come_back();
                    g.to_main(Bridge::event("lock", serde_json::json!({})));
                }
            }
        });
    }
    // Quick Access over a running game on the Handheld Daemon editions: HHD's overlay app is Hearth's (hosts/linux/src/
    // overlay, in Chromium); HHD shows it over the game, and it talks to Hearth through the overlay socket. If the socket
    // can't be opened, Hearth's own overlay window instead.
    if gm.hhd() {
        let (h, b, g) = (host.clone(), bridge.clone(), gm.clone());
        let on_msg: hearth_core::overlay_link::OnMessage = Arc::new(move |m: serde_json::Value| {
            // A file of Hearth's screens for the page (never through the bridge).
            if m["type"] == "ui-file" { std::thread::spawn(move || hearth_core::overlay_link::send(&hearth_core::overlay_link::ui_file(&m, &ui_file_for))); return; }
            let (h, b) = (h.clone(), b.clone()); std::thread::spawn(move || page_msg(&h, b, "overlay".to_string(), m));
        });
        hearth_core::overlay_link::set_hello(&overlay_hello());
        let on_connect: hearth_core::overlay_link::OnConnect = Arc::new(move || { let g = g.clone(); std::thread::spawn(move || g.overlay_app_connected()); });
        match hearth_core::overlay_link::start(on_msg, on_connect) {
            Ok(p) => {
                hearth_core::bridge::log(&format!("overlay: Handheld Daemon shows Quick Access over games; its overlay app talks to Hearth at {}", p.display()));
                gm.start_overlay_keeper();
            }
            Err(e) => {
                hearth_core::bridge::log(&format!("overlay: couldn't open the overlay socket ({e}); using Hearth's own overlay window"));
                gm.hhd_overlay.store(false, std::sync::atomic::Ordering::SeqCst);
            }
        }
    }
    // Quick Access over a running game: a see-through window gamescope draws on top.
    if gm.overlay_available() && !gm.hhd() && host.create_overlay() {
        let g = gm.clone();
        std::thread::spawn(move || { std::thread::sleep(std::time::Duration::from_secs(3)); g.arm_overlay(); });
    }
    // Desktop Mode couldn't start and sent us back here: say why.
    let h = host.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(2500));
        let f = std::path::PathBuf::from(std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into())).join("nimbus-desktop-failed");
        if let Ok(text) = std::fs::read_to_string(&f) {
            let _ = std::fs::remove_file(&f);
            hearth_core::bridge::log(&format!("desktop mode failed: {text}"));
            let t: String = text.trim().chars().take(1500).collect();
            broadcast(&h, Bridge::event("desktopFailed", serde_json::json!({"text": t})));
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

/// Every check that Settings › Updates (Game Mode) and Mantel's Settings › Updates (Desktop Mode) depend on.
/// Prints one line per check; returns the exit code (1 if any failed).
fn check_update_path() -> i32 {
    let mut b = Bridge::new(env!("CARGO_PKG_VERSION"));
    let info = b.handle(&serde_json::json!({"type": "call", "id": 1, "method": "host.info", "args": {}}))
        .map(|r| r["value"].clone()).unwrap_or_default();
    let ui = String::from_utf8_lossy(include_bytes!("../../../ui/index.html")).to_string();
    let exists = |p: &str| std::path::Path::new(p).exists();
    let qml = std::fs::read_to_string("/usr/share/nimbus/desktop/shell/SetUpdates.qml").unwrap_or_default();
    let admin = std::fs::read_to_string(hearth_core::admin::HELPER).unwrap_or_default();
    let checks: Vec<(&str, bool)> = vec![
        ("Game Mode: Hearth knows it's the OS install (host.info system), so Update now shows", info["system"] == true),
        ("Game Mode: the Updates page shows Update now for the OS install", ui.contains("if(LINUX && HOSTINFO.system) return pageSystemUpdate();") && ui.contains("data-act=\"sysUpdInstall\"") && ui.contains("case 'sysUpdInstall'")),
        ("Game Mode: Hearth answers system.update.install", include_str!("../../hearth-core/src/bridge.rs").contains("\"system.update.install\"")),
        ("hearth-admin is installed", exists(hearth_core::admin::HELPER)),
        ("hearth-admin can check, show and install updates", ["'update-status'", "'update-check'", "'update'"].iter().all(|c| admin.contains(c))),
        ("the polkit rule lets the person at the device run hearth-admin", std::fs::read_to_string("/usr/share/polkit-1/rules.d/49-hearth-admin.rules").map(|t| t.contains("/usr/libexec/nimbus/hearth-admin")).unwrap_or(false)),
        ("pkexec and bootc are installed", hearth_core::system::has("pkexec") && hearth_core::system::has("bootc")),
        ("Desktop Mode: Mantel's Settings › Updates is installed and runs hearth-admin", qml.contains("/usr/libexec/nimbus/hearth-admin") && qml.contains("\"update\"")),
        ("Desktop Mode: Hearth Settings opens", exists("/usr/bin/hearth-settings")),
    ];
    let mut code = 0;
    for (name, ok) in checks {
        if ok { println!("ok: {name}"); } else { println!("::error title=update path::{name}"); code = 1; }
    }
    code
}

/// An Nvidia graphics card in this PC.
fn nvidia() -> bool {
    std::fs::read_dir("/sys/class/drm").into_iter().flatten().flatten()
        .any(|e| std::fs::read_to_string(e.path().join("device/vendor")).map(|v| v.trim() == "0x10de").unwrap_or(false))
}

/// Whether WebKit's fast drawing path can be used on this Nvidia PC. The black window it drew was on older
/// drivers; from 560 (explicit sync, GBM) it works, and without it Hearth is sluggish (a slow copy every frame).
/// ~/.config/nimbus/nvidia-drawing says otherwise when it holds "fast" or "compatible" (Settings › Display).
fn nvidia_dmabuf_ok() -> bool {
    let pick = std::fs::read_to_string(hearth_core::paths::config_dir().join("nvidia-drawing")).unwrap_or_default();
    match pick.trim() {
        "fast" => return true,
        "compatible" => return false,
        _ => {}
    }
    let v = std::fs::read_to_string("/sys/module/nvidia/version").unwrap_or_default();
    v.trim().split('.').next().and_then(|m| m.parse::<u32>().ok()).map(|m| m >= 560).unwrap_or(false)
}

/// Hearth's pages drawn by the graphics card all the time (WebKit otherwise decides page by page, and can fall
/// back to drawing on the processor, which is what made scrolling the library slow).
fn gpu_always(w: &tauri::WebviewWindow) {
    #[cfg(target_os = "linux")]
    let _ = w.with_webview(|wv| {
        use webkit2gtk::{SettingsExt, WebViewExt};
        if let Some(s) = wv.inner().settings() {
            s.set_hardware_acceleration_policy(webkit2gtk::HardwareAccelerationPolicy::Always);
            s.set_enable_smooth_scrolling(true);
        }
    });
}


fn main() {
    let args: Vec<String> = std::env::args().collect();
    let has = |a: &str| args.iter().any(|x| x == a);
    // Release check (os.yml runs it inside each new Hearth OS image, and won't publish one that fails):
    // Hearth must be able to update itself from Game Mode and from Desktop Mode.
    if has("--check-update-path") { std::process::exit(check_update_path()); }
    // hearth-send-logs (a terminal, e.g. when Hearth's screen won't come up): the same logs as Settings › System ›
    // Send logs to the developer, with an optional note; prints the code. No window is opened.
    if args.get(1).map(String::as_str) == Some("--send-logs") {
        let note = args.iter().skip(2).cloned().collect::<Vec<_>>().join(" ");
        println!("Sending Hearth's logs…");
        match hearth_core::logs::send_report(env!("CARGO_PKG_VERSION"), Bridge::edition_name(), &note) {
            Ok(r) => { println!("Sent. Your code: {}\n{}", r["code"].as_str().unwrap_or("?"), r["url"].as_str().unwrap_or("")); std::process::exit(0); }
            Err(e) => { eprintln!("{e}"); std::process::exit(1); }
        }
    }
    // Mantel's Games section (Desktop Mode's launcher): the installed games as JSON. No window is opened.
    if has("--list-games") { println!("{}", hearth_core::desktop_games::list_now()); std::process::exit(0); }
    let windowed = has("--windowed");
    // Started by Hearth's Electron window app (Chromium): no window here, everything else as usual.
    if has("--core-stdio") { std::process::exit(core_ipc::run(windowed)); }
    // Which engine draws Hearth: Chromium (the Electron window app) on Nvidia graphics, WebKit elsewhere; Settings ›
    // Display › Drawing engine, or HEARTH_RENDERER, says otherwise. Chromium that failed to start twice lately: WebKit.
    if let Some(app) = hearth_core::renderer::chromium_to_run() {
        use std::os::unix::process::CommandExt;
        hearth_core::renderer::note_chromium_start();
        hearth_core::bridge::log(&format!("renderer: Chromium (Electron window app {})", app.display()));
        let me = std::env::current_exe().map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|_| "/usr/bin/nimbus".into());
        let err = std::process::Command::new(&app).args(&args[1..]).env("HEARTH_CORE_BIN", me).exec();
        hearth_core::bridge::log(&format!("renderer: couldn't start the Electron window app ({err}); drawing with WebKit"));
    }
    // WebKit's newer GPU path draws a black window on some Nvidia drivers: the older one works everywhere.
    // Only there: on AMD and Intel (the ROG Ally, Steam Deck) it's the fast path, and without it WebKit draws
    // through a slow copy (a sluggish library, 5 to 10 frames a second).
    // Basic graphics (nomodeset), or a PC where the fast way already failed (nimbus-watchdog saw WebKit's "Failed to
    // create GBM buffer" and wrote ~/.config/nimbus/drawing-fallback): the compatible way, or nothing is drawn.
    let fallback = hearth_core::paths::config_dir().join("drawing-fallback").exists()
        || std::fs::read_to_string("/proc/cmdline").map(|c| c.split_whitespace().any(|w| w == "nomodeset")).unwrap_or(false);
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() && fallback {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() && nvidia() && !nvidia_dmabuf_ok() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(Host(Arc::new(Mutex::new(Bridge::new(env!("CARGO_PKG_VERSION"))))))
        // Game art is read off the window's thread: with hundreds of covers in the library, reading them on it
        // froze the screen while you scrolled.
        .register_asynchronous_uri_scheme_protocol("nimbus-art", |_ctx, req, responder| {
            let uri = req.uri().to_string();
            std::thread::spawn(move || responder.respond(art(&uri)));
        })
        // Epic's sign-in page hands its code over as hearth-signin://done?code=… (if the window lets it through).
        .register_uri_scheme_protocol("hearth-signin", |_ctx, req| {
            let u = req.uri().to_string();
            if u.contains("stream-exit") { if let Some(gm) = GM.get() { gm.host.destroy("stream"); } }
            else { signin_answer(hearth_core::http::query_param(&u, "code")); }
            Response::builder().status(StatusCode::OK).body(Vec::new()).unwrap()
        })
        .invoke_handler(tauri::generate_handler![nimbus_msg])
        .setup(move |app| {
            let bridge = app.state::<Host>().0.clone();
            let zoom = bridge.lock().map(|b| b.zoom_pct()).unwrap_or(100);
            let main = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title("Hearth")
                .fullscreen(!windowed)
                .inner_size(1280.0, 800.0)
                .background_color(tauri::window::Color(12, 16, 22, 255))
                .initialization_script(SHIM)
                .build()?;
            gpu_always(&main);
            hearth_core::bridge::log(&format!("graphics: {} (DMA-BUF renderer {})", if nvidia() { "Nvidia" } else { "AMD/Intel" }, if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_some() { "off" } else { "on" }));
            if zoom != 100 { let _ = main.set_zoom(zoom as f64 / 100.0); }
            // As the session there's nothing behind Hearth, so Alt+F4 must not close it by accident. (A windowed Hearth
            // in Desktop Mode is also in the session, and closes like any window.)
            main.on_window_event(move |e| if let tauri::WindowEvent::CloseRequested { api, .. } = e { if in_session() && !windowed { api.prevent_close(); } });
            let _ = UI_APP.set(app.handle().clone());
            start_core(Arc::new(TauriHost { app: app.handle().clone() }), bridge, windowed);
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("Hearth couldn't start")
        .run(|_app, e| {
            // Leaving: the controller service goes back to normal.
            if let tauri::RunEvent::Exit = e { if let Some(gm) = GM.get() { gm.stop_inputplumber(); } }
        });
}
