// Hearth Games' window (Tauri: WebKitGTK on Linux), docs/HEARTH-GAMES.md. The page (ui/games.html) shows your GOG,
// Epic and Battle.net games: sign in, the library, install (on the drive you choose), downloads, play, uninstall. Its
// Recomps tab (ui/recomps.js) is Hearth Recomps (hearth-core recomps.rs, docs/RECOMPS.md) through the same Bridge.
// The work is Hearth's own store engine (hearth-core): this process keeps one Bridge and only makes the calls in
// hearth_gaming::gamestore::CALLS. Games start the way their Steam Game Mode shortcuts start them (`nimbus
// --play-now <id>`, hearth-core desktop_games::play_now), so a game that starts here starts there.
//
//   hearth-gamestore [--fullscreen] [--store gog|epic|battlenet]   the window, opened on one store
//   hearth-gamestore --tab recomps                                  the window, opened on a tab (library, downloads,
//                                                                    stores, recomps); an open window switches to it
//
// One window at a time: a second start shows the open one (a socket in $XDG_RUNTIME_DIR/hearth). Closing it while
// games download asks first; "Keep downloading" hides the window, and the app ends by itself once they're done.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[path = "../../../hearth-apptheme/apptheme.rs"]
mod apptheme;

#[cfg(target_os = "linux")]
mod linux {
    use super::apptheme;
    use hearth_core::bridge::Bridge;
    use hearth_gaming::gamestore;
    use serde_json::{json, Value};
    use std::collections::HashMap;
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::{UnixListener, UnixStream};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::{Arc, Condvar, Mutex, OnceLock};
    use std::time::{Duration, Instant};
    use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

    /// A sign-in waits for the person (up to this long); everything else is answered well within it.
    const WAIT: Duration = Duration::from_secs(20 * 60);

    static BRIDGE: Mutex<Option<Bridge>> = Mutex::new(None);
    static ANSWERS: OnceLock<(Mutex<HashMap<u64, Value>>, Condvar)> = OnceLock::new();
    static NEXT: AtomicU64 = AtomicU64::new(1);
    static APP: OnceLock<tauri::AppHandle> = OnceLock::new();
    /// Downloads running (the page's last look at them): closing the window asks first.
    static DOWNLOADING: AtomicBool = AtomicBool::new(false);
    /// The window was hidden to keep downloading: the app ends once they're done.
    static HIDDEN: AtomicBool = AtomicBool::new(false);
    /// Where a sign-in window's answer goes (one at a time): the code, or None when it was closed.
    static SIGNIN: Mutex<Option<std::sync::mpsc::Sender<Option<String>>>> = Mutex::new(None);

    fn answers() -> &'static (Mutex<HashMap<u64, Value>>, Condvar) { ANSWERS.get_or_init(|| (Mutex::new(HashMap::new()), Condvar::new())) }

    fn put_answer(v: Value) {
        let Some(id) = v["id"].as_u64() else { return };
        let (m, cv) = answers();
        m.lock().unwrap_or_else(|p| p.into_inner()).insert(id, v);
        cv.notify_all();
    }

    fn to_page(v: &Value) { if let Some(app) = APP.get() { let _ = app.emit("games://event", v.clone()); } }

    /// What Hearth's engine tells the window app: answers to calls answered later, events, its own background work.
    fn on_notify(v: Value) {
        match v["type"].as_str().unwrap_or("") {
            "internal" => { std::thread::spawn(move || {
                for e in with_bridge(|b| b.internal(&v)) { if e["type"] == "result" { put_answer(e); } else { to_page(&e); } }
            }); }
            "result" => put_answer(v),
            "event" => to_page(&v),
            _ => {}
        }
    }

    pub fn socket() -> PathBuf {
        PathBuf::from(std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into())).join("hearth").join("gamestore.sock")
    }

    fn with_bridge<R>(f: impl FnOnce(&mut Bridge) -> R) -> R {
        let mut g = BRIDGE.lock().unwrap_or_else(|p| p.into_inner());
        if g.is_none() {
            hearth_core::stores::set_notify(Arc::new(on_notify));
            let data = hearth_core::paths::data_dir();
            let lib = hearth_core::library::Library::new(hearth_core::store::Store::open(data.join("library.json")), hearth_core::paths::home());
            let mut b = Bridge::with(lib, hearth_core::profiles::Profiles::open(&data), env!("CARGO_PKG_VERSION"));
            b.emit = Arc::new(|v: Value| to_page(&v));
            b.sign_in = Arc::new(|page: &hearth_core::stores::SignIn| sign_in(page));
            // Recomps' "Needs your copy of …": the desktop's file chooser (Hearth Files through the portal), as Mods uses.
            b.pick_file = Arc::new(|title: &str| pick_file(title));
            b.cur_label = "games".into();
            hearth_core::bridge::log("hearth games: opened (GOG, Epic, Battle.net)");
            *g = Some(b);
        }
        f(g.as_mut().expect("made above"))
    }

    /// One of Hearth's calls, waiting for it when it's answered later (a sign-in, a store's game list).
    fn bridge_call(method: &str, args: &Value) -> Result<Value, String> {
        let id = 1_000 + NEXT.fetch_add(1, Ordering::SeqCst);
        let reply = with_bridge(|b| b.handle(&json!({"type": "call", "id": id, "method": method, "args": args})));
        let r = match reply {
            Some(r) => r,
            None => {
                let start = Instant::now();
                let (m, cv) = answers();
                let mut g = m.lock().unwrap_or_else(|p| p.into_inner());
                loop {
                    if let Some(v) = g.remove(&id) { break v; }
                    if start.elapsed() > WAIT { return Err("That's taking too long. Try again.".into()); }
                    g = cv.wait_timeout(g, Duration::from_millis(500)).map(|x| x.0).unwrap_or_else(|p| p.into_inner().0);
                }
            }
        };
        if r["ok"] == true { Ok(r["value"].clone()) } else { Err(r["error"].as_str().unwrap_or("Hearth couldn't do that").to_string()) }
    }

    /// A store's sign-in page in its own window (the store's own page: Hearth never sees the password). Blocks until
    /// it ends with a code, or is closed. The same window Hearth's own sign-in opens (hearth-rs main.rs sign_in), with
    /// the store's browser profile shared with it.
    fn sign_in(page: &hearth_core::stores::SignIn) -> Option<String> {
        let app = APP.get()?;
        let (tx, rx) = std::sync::mpsc::channel();
        if let Some(old) = SIGNIN.lock().unwrap().take() { let _ = old.send(None); }
        *SIGNIN.lock().unwrap() = Some(tx);
        if let Some(old) = app.get_webview_window("signin") { let _ = old.destroy(); }
        let done = page.done;
        let script = format!("{}\n{}", include_str!("../../../hearth-rs/src/js/signin-nogamepad.js"), page.script.unwrap_or(""));
        let answer = |v: Option<String>| { if let Some(tx) = SIGNIN.lock().unwrap().take() { let _ = tx.send(v); } };
        let b = WebviewWindowBuilder::new(app, "signin", WebviewUrl::External(page.url.parse().ok()?))
            .title(page.title)
            .inner_size(900.0, 760.0)
            .center()
            .data_directory(hearth_core::storefront::web_profile(page.store, "webkit"))
            .initialization_script(&script)
            .on_navigation(move |url| {
                if url.as_str().starts_with("hearth-signin://cancel") { answer(None); return false; }
                match done(url.as_str()) { Some(code) => { answer(Some(code)); false } None => true }
            });
        let w = match b.build() { Ok(w) => w, Err(e) => { hearth_core::bridge::log(&format!("hearth games: sign-in window: {e}")); return None; } };
        w.on_window_event(|e| if let tauri::WindowEvent::Destroyed = e { if let Some(tx) = SIGNIN.lock().unwrap().take() { let _ = tx.send(None); } });
        let code = rx.recv().ok().flatten();
        let _ = w.destroy();
        if let Some(main) = app.get_webview_window("main") { let _ = main.set_focus(); }
        code
    }

    /// A file from the standard file chooser (None: closed). Runs on hearth-core's picker thread (picker.rs), so the
    /// window and the Bridge keep answering while it's open.
    fn pick_file(title: &str) -> Option<String> {
        use tauri_plugin_dialog::DialogExt;
        let app = APP.get()?;
        let home = hearth_core::paths::home();
        app.dialog().file().set_title(title).set_directory(home).blocking_pick_file()
            .and_then(|f| f.into_path().ok()).map(|p| p.to_string_lossy().into_owned())
    }

    fn art(uri: &str) -> tauri::http::Response<Vec<u8>> {
        match gamestore::art_file(uri).and_then(|(p, m)| std::fs::read(&p).ok().map(|b| (b, m))) {
            None => tauri::http::Response::builder().status(404).body(Vec::new()).unwrap(),
            Some((bytes, m)) => tauri::http::Response::builder().header("Content-Type", m).header("Cache-Control", "max-age=86400")
                .header("Access-Control-Allow-Origin", "*").body(bytes).unwrap(),
        }
    }

    #[tauri::command]
    async fn call(method: String, args: Value) -> Result<Value, String> {
        tauri::async_runtime::spawn_blocking(move || {
            let (m, a) = gamestore::check(&method, &args)?;
            if m == "recomps.useGame" {
                let id = a["id"].as_str().unwrap_or("");
                let job = with_bridge(|b| b.recomps.jobs.lock().map(|j| j.get(id).cloned()).ok().flatten()).unwrap_or(Value::Null);
                if !gamestore::use_game_ok(&job, a["path"].as_str().unwrap_or("")) { return Err("Choose your copy again first.".into()); }
            }
            let v = bridge_call(&m, &a)?;
            Ok(match m.as_str() {
                "library.get" => gamestore::only_ours(v),
                "library.downloads" => {
                    DOWNLOADING.store(v["active"].as_array().map(|a| a.iter().any(|g| g["id"].as_str().map(gamestore::game_id_ok).unwrap_or(false))).unwrap_or(false), Ordering::SeqCst);
                    gamestore::only_our_downloads(v)
                }
                "game.uninstall" => gamestore::only_ours(v),
                _ => v,
            })
        }).await.map_err(|e| e.to_string())?
    }

    #[tauri::command]
    async fn drives() -> Value {
        tauri::async_runtime::spawn_blocking(|| with_bridge(|b| gamestore::drives(&b.library))).await.unwrap_or(Value::Null)
    }

    #[tauri::command]
    async fn use_drive(mount: String) -> Result<Value, String> {
        tauri::async_runtime::spawn_blocking(move || with_bridge(|b| gamestore::use_drive(&mut b.library, &mount))).await.map_err(|e| e.to_string())?
    }

    /// The Hearth app (the session's NIMBUS_APP, or the installed one), the same search as hearth-games' app().
    fn nimbus() -> Option<PathBuf> {
        [std::env::var_os("NIMBUS_APP").map(PathBuf::from), Some(PathBuf::from("/usr/bin/nimbus"))].into_iter().flatten()
            .find(|p| p.is_file())
    }

    /// Play: the same start as the game's Steam Game Mode shortcut (`nimbus --play-now <id>`), in its own session so
    /// closing this window doesn't end the game.
    #[tauri::command]
    fn play(id: String) -> Result<Value, String> {
        if !gamestore::playable(&id) { return Err("That isn't a GOG, Epic or Battle.net game or an installed recomp".into()); }
        let app = nimbus().ok_or("Hearth isn't installed here, so it can't start games.")?;
        use std::os::unix::process::CommandExt;
        let mut c = std::process::Command::new(app);
        c.args(["--play-now", &id]).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
        unsafe { c.pre_exec(|| { libc::setsid(); Ok(()) }); }
        c.spawn().map_err(|e| format!("Couldn't start the game: {e}"))?;
        hearth_core::bridge::log(&format!("hearth games: play {id}"));
        Ok(json!({"ok": true}))
    }

    /// The store's own website, for buying and claiming (in the default browser; purchases show after a refresh).
    #[tauri::command]
    fn open_store(store: String) -> Result<(), String> {
        let url = match store.as_str() {
            "gog" => "https://www.gog.com/",
            "epic" => "https://store.epicgames.com/",
            "battlenet" => "https://shop.battle.net/",
            _ => return Err("Unknown store".into()),
        };
        std::process::Command::new("xdg-open").arg(url).spawn().map(|_| ()).map_err(|e| e.to_string())
    }

    #[tauri::command]
    fn info() -> Value {
        let args: Vec<String> = std::env::args().collect();
        let (tab, store) = gamestore::start_view(&args);
        json!({
            "version": env!("CARGO_PKG_VERSION"),
            "store": store,
            "tab": tab,
            "proton": hearth_core::proton_pick::builtin_ge().and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string())),
            "canPlay": nimbus().is_some(),
        })
    }

    /// The page's answer to "close while downloading": hide and keep going, or close (downloads carry on next time).
    #[tauri::command]
    fn close_window(app: tauri::AppHandle, keep: bool) {
        if keep {
            HIDDEN.store(true, Ordering::SeqCst);
            if let Some(w) = app.get_webview_window("main") { let _ = w.hide(); }
            hearth_core::bridge::log("hearth games: hidden while games download");
        } else {
            DOWNLOADING.store(false, Ordering::SeqCst);
            if let Some(w) = app.get_webview_window("main") { let _ = w.close(); }
        }
    }

    fn show(app: &tauri::AppHandle) {
        HIDDEN.store(false, Ordering::SeqCst);
        if let Some(w) = app.get_webview_window("main") { let _ = w.show(); let _ = w.unminimize(); let _ = w.set_focus(); }
    }

    /// Another start of Hearth Games while this one is open: show this one, on the tab or store that start asked for
    /// ("show recomps", "show library epic"). Ok(true) when it was handed over.
    pub fn hand_over() -> bool {
        let args: Vec<String> = std::env::args().collect();
        let (tab, store) = gamestore::start_view(&args);
        let line = ["show".to_string(), tab.unwrap_or_default(), store.unwrap_or_default()].join(" ");
        match UnixStream::connect(socket()) {
            Ok(mut s) => { let _ = s.write_all(format!("{}\n", line.trim_end()).as_bytes()); true }
            Err(_) => false,
        }
    }

    fn listen(app: tauri::AppHandle) {
        let p = socket();
        if let Some(d) = p.parent() {
            let _ = std::fs::create_dir_all(d);
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(d, std::fs::Permissions::from_mode(0o700));
        }
        let _ = std::fs::remove_file(&p);
        let Ok(l) = UnixListener::bind(&p) else { return };
        std::thread::spawn(move || {
            for s in l.incoming().flatten() {
                let mut line = String::new();
                let _ = BufReader::new(s).read_line(&mut line);
                let mut w = line.split_whitespace();
                if w.next() != Some("show") { continue; }
                // Only the names start_view allows get to the page.
                let tab = w.next().filter(|t| gamestore::TABS.contains(t)).map(str::to_string);
                let store = w.next().filter(|s| gamestore::SOURCES.contains(s)).map(str::to_string);
                if tab.is_some() { let _ = app.emit("games://open", json!({"tab": tab, "store": store})); }
                let a = app.clone(); let _ = app.run_on_main_thread(move || show(&a));
            }
        });
    }

    /// Hidden to keep downloading: once nothing of ours downloads for a little while, the app ends.
    fn watch_hidden(app: tauri::AppHandle) {
        std::thread::spawn(move || {
            let mut idle = 0;
            loop {
                std::thread::sleep(Duration::from_secs(5));
                if !HIDDEN.load(Ordering::SeqCst) { idle = 0; continue; }
                let busy = BRIDGE.try_lock().ok().and_then(|g| g.as_ref().map(|b| b.library.queue.lock().map(|q| !q.ids().is_empty()).unwrap_or(true))).unwrap_or(true);
                idle = if busy { 0 } else { idle + 1 };
                if idle >= 3 { hearth_core::bridge::log("hearth games: downloads done, closing"); app.exit(0); }
            }
        });
    }

    pub fn run() {
        if hand_over() { return; }
        let args: Vec<String> = std::env::args().collect();
        let fullscreen = hearth_gaming::util::has_arg(&args, "--fullscreen");
        if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() { std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1"); }
        tauri::Builder::default().plugin(apptheme::plugin())
            .plugin(tauri_plugin_dialog::init())
            // The library's pictures, as Hearth's own window serves them (nimbus-art://cache/…, nimbus-art://file/…).
            .register_asynchronous_uri_scheme_protocol("nimbus-art", |_ctx, req, responder| {
                let uri = req.uri().to_string();
                std::thread::spawn(move || responder.respond(art(&uri)));
            })
            // Epic's sign-in page hands its code over as hearth-signin://done?code=… (if the window lets it through).
            .register_uri_scheme_protocol("hearth-signin", |_ctx, req| {
                let u = req.uri().to_string();
                if let Some(tx) = SIGNIN.lock().unwrap().take() { let _ = tx.send(hearth_core::http::query_param(&u, "code")); }
                tauri::http::Response::builder().status(200).body(Vec::new()).unwrap()
            })
            .invoke_handler(tauri::generate_handler![call, drives, use_drive, play, open_store, info, close_window])
            .setup(move |app| {
                let _ = APP.set(app.handle().clone());
                listen(app.handle().clone());
                watch_hidden(app.handle().clone());
                let w = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("games.html".into()))
                    .title("Hearth Games")
                    .inner_size(1280.0, 800.0)
                    .min_inner_size(720.0, 520.0)
                    .fullscreen(fullscreen)
                    .background_color(apptheme::bg())
                    .build()?;
                let h = app.handle().clone();
                w.on_window_event(move |e| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = e {
                        if DOWNLOADING.load(Ordering::SeqCst) { api.prevent_close(); let _ = h.emit("games://closing", json!({})); }
                    }
                });
                Ok(())
            })
            .run(tauri::generate_context!())
            .expect("Hearth Games couldn't start");
        let _ = std::fs::remove_file(socket());
    }
}

fn main() {
    #[cfg(target_os = "linux")]
    linux::run();
    #[cfg(not(target_os = "linux"))]
    eprintln!("Hearth Games is for Hearth OS (Linux) for now.");
}
