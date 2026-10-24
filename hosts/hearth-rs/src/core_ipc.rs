//! `hearth --core-stdio`: Hearth without a window of its own, for Hearth's Electron window app (hosts/electron), which
//! draws the screens with Chromium, and for Hearth's Qt Quick window app (hosts/hearth-qml, Experimental). Everything else is the usual Hearth (main.rs `start_core`, Game Mode); the window
//! app and this process talk over standard input and output (the channel is described in winhost.rs).

use crate::winhost::{parse, IpcHost, WindowHost};
use hearth_core::bridge::Bridge;
use serde_json::json;
use std::io::BufRead;
use std::sync::{Arc, Mutex};

/// Runs until the window app goes away (standard input closes). Returns the exit code.
pub fn run(windowed: bool) -> i32 {
    // host.info says which engine draws Hearth (Settings › Display › Renderer shows it): the window app that started
    // this core says so (the Qt window app sets "qt"); the Electron app, from before it did, is Chromium.
    let qt = std::env::var("HEARTH_RENDERER_RUNNING").as_deref() == Ok("qt");
    if !qt { std::env::set_var("HEARTH_RENDERER_RUNNING", "chromium"); }
    let host = Arc::new(IpcHost::new(Box::new(std::io::stdout())));
    let dynh: Arc<dyn WindowHost> = host.clone();
    let bridge = Arc::new(Mutex::new(Bridge::new(env!("CARGO_PKG_VERSION"))));
    crate::start_core(dynh.clone(), bridge.clone(), windowed);
    // Settings › Display › size, as the WebKit window starts with it (main.rs): the window app applies it once the
    // page is there.
    let zoom = bridge.lock().map(|b| b.zoom_pct()).unwrap_or(100);
    if zoom != 100 { dynh.zoom(zoom); }
    hearth_core::bridge::log(if qt { "renderer: Qt, Experimental (Hearth's Qt Quick window app draws, this core does the rest)" }
                             else { "renderer: Chromium (Hearth's Electron window app draws, this core does the rest)" });
    let mut ok_noted = false;
    for line in std::io::stdin().lock().lines().map_while(Result::ok) {
        let Some(v) = parse(&line) else { continue };
        match v["t"].as_str().unwrap_or("") {
            "page" => {
                let label = v["win"].as_str().unwrap_or("main").to_string();
                let msg = v["msg"].clone();
                // The main page came up: this engine works on this PC (Chromium: two failed starts in a row go back
                // to WebKit; Qt: one gives it up).
                if !ok_noted && label == "main" && msg["type"] == "ready" { ok_noted = true; hearth_core::renderer::note_ok_running(); }
                let (h, b) = (dynh.clone(), bridge.clone());
                std::thread::spawn(move || crate::page_msg(&h, b, label, msg));
            }
            "art" => {
                let r = crate::art_path(v["uri"].as_str().unwrap_or(""));
                host.send(json!({"t": "art", "id": v["id"], "path": r.as_ref().map(|x| x.0.clone()), "mime": r.map(|x| x.1)}));
            }
            "stream.closed" => hearth_core::stores::notify(json!({"type": "internal", "what": "streamClosed"})),
            _ => { host.incoming(&v); }
        }
    }
    // The window app is gone (it closed, or crashed): leave too, putting the controller service back to normal.
    if let Some(gm) = crate::GM.get() { gm.stop_inputplumber(); }
    0
}
