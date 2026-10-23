// Hearth Time: Clock, Calendar, Notes and Weather in one program. Each app is its own window and process with its
// own page (ui/<app>.html, sharing ui/hearth.css and ui/hearth.js), started as `hearth-time --app <app>` or through
// the links hearth-clock, hearth-calendar, hearth-notes and hearth-weather. Each window also gets its own app id
// (io.github.dragongamer9.hearth.<app>), so the taskbar shows them as four apps with their own icons.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use hearth_time::{calendar, clock, notes, paths, ring, service, store, weather};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};
use tauri::{WebviewUrl, WebviewWindowBuilder};

const USAGE: &str = "hearth-time: Hearth's Clock, Calendar, Notes and Weather

  hearth-time --app clock|calendar|notes|weather [--fullscreen]   open an app (also: hearth-clock, hearth-calendar...)
  hearth-time --service                     ring alarms and timers, show reminders (run by systemd: hearth-time.service)
  hearth-time --ring alarm:ID[,timer:ID]    the ringing window (the service opens it)
  hearth-time --events FROM TO              calendar events between two dates (YYYY-MM-DD) as JSON, for the taskbar
  hearth-time --weather-summary             the weather in a few words as JSON (empty without a chosen place)
  hearth-time --version
";

struct Start { app: &'static str, ring: Vec<(String, String)> }

static PINNED: AtomicU64 = AtomicU64::new(0);

// ---------------------------------------------------------------------------------------------- every app

#[tauri::command]
fn init(start: tauri::State<Start>) -> Value {
    json!({
        "app": start.app,
        "platform": if cfg!(windows) { "windows" } else { "linux" },
        "version": env!("CARGO_PKG_VERSION"),
        "ring": start.ring.iter().map(|(k, i)| json!({"kind": k, "id": i})).collect::<Vec<_>>(),
        "notesDir": notes::dir(),
        "dnd": paths::dnd_on(),
    })
}

#[tauri::command]
fn quit(app: tauri::AppHandle) { app.exit(0); }

/// Opens another of the four apps (Calendar from the clock, and so on).
#[tauri::command]
fn open_app(name: String) -> Result<(), String> {
    let app = hearth_time::app_from_name(&name).ok_or("No such app.")?;
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    std::process::Command::new(exe).args(["--app", app]).spawn().map(|_| ()).map_err(|e| e.to_string())
}

/// Opens a folder in the file manager (the Notes folder).
#[tauri::command]
fn open_folder(which: String) -> Result<(), String> {
    let p = match which.as_str() { "notes" => notes::dir(), _ => return Err("No such folder.".into()) };
    let _ = std::fs::create_dir_all(&p);
    #[cfg(windows)]
    let r = std::process::Command::new("explorer").arg(&p).spawn();
    #[cfg(not(windows))]
    let r = std::process::Command::new("xdg-open").arg(&p).spawn();
    r.map(|_| ()).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------------------------- Clock

#[tauri::command(async)]
fn clock_get() -> Value {
    let ringing = store::load(&service::ringing_file());
    json!({"clock": clock::load(), "mtime": store::mtime(&paths::clock_file()), "ringing": if ringing["items"].is_array() { ringing } else { Value::Null }, "now": hearth_time::now_ms()})
}

#[tauri::command(async)]
fn clock_save(value: Value) -> Result<i64, String> {
    if !value.is_object() { return Err("Nothing to save.".into()); }
    store::save(&paths::clock_file(), &value)?;
    Ok(store::mtime(&paths::clock_file()))
}

/// The next ring of every alarm (ms or null), worked out the same way the service does.
#[tauri::command(async)]
fn clock_next() -> Value {
    let c = clock::load();
    let now = hearth_time::now_ms();
    let m: serde_json::Map<String, Value> = c["alarms"].as_array().into_iter().flatten()
        .map(|a| (a["id"].as_str().unwrap_or("").to_string(), json!(clock::next_ring(a, now)))).collect();
    Value::Object(m)
}

/// Stops whatever the service is ringing (from the Clock app: for a ring that has no window).
#[tauri::command(async)]
fn ring_stop() -> Result<(), String> { store::write_bytes(&service::stop_file(), b"stop\n") }

/// The ringing window: what's ringing.
#[tauri::command(async)]
fn ring_info(start: tauri::State<Start>) -> Value {
    let c = clock::load();
    let items: Vec<Value> = start.ring.iter().map(|(kind, id)| {
        let list = if kind == "alarm" { &c["alarms"] } else { &c["timers"] };
        let it = list.as_array().into_iter().flatten().find(|x| x["id"] == id.as_str()).cloned().unwrap_or(Value::Null);
        json!({"kind": kind, "id": id, "item": it})
    }).collect();
    json!({"items": items, "settings": c["settings"]})
}

/// The ringing window's buttons: "snooze" or "dismiss". Closes it (which stops the tone in the service).
#[tauri::command(async)]
fn ring_done(app: tauri::AppHandle, start: tauri::State<Start>, action: String) -> Result<(), String> {
    let mut c = clock::load();
    let now = hearth_time::now_ms();
    for (kind, id) in &start.ring { clock::after_ring(&mut c, kind, id, &action, now); }
    let r = store::save(&paths::clock_file(), &c);
    app.exit(0);
    r
}

// ---------------------------------------------------------------------------------------------- Calendar

#[tauri::command(async)]
fn cal_get() -> Value {
    let v = calendar::load();
    json!({"settings": v["settings"], "subs": v["subs"], "mtime": store::mtime(&paths::calendar_file())})
}

#[tauri::command(async)]
fn cal_range(from: i64, to: i64) -> Value {
    json!(calendar::range(&calendar::load(), from, to))
}

#[tauri::command(async)]
fn cal_event(id: String) -> Value {
    calendar::all_events(&calendar::load()).into_iter().find(|e| e.id == id).map(|e| json!(e)).unwrap_or(Value::Null)
}

#[tauri::command(async)]
fn cal_put(event: calendar::Event) -> Result<Value, String> {
    let mut v = calendar::load();
    let e = calendar::put_event(&mut v, event)?;
    calendar::save(&v)?;
    Ok(json!(e))
}

#[tauri::command(async)]
fn cal_delete(id: String, occ: Option<String>) -> Result<bool, String> {
    let mut v = calendar::load();
    let r = calendar::delete_event(&mut v, &id, occ.as_deref());
    calendar::save(&v)?;
    Ok(r)
}

#[tauri::command(async)]
fn cal_settings(settings: Value) -> Result<(), String> {
    let mut v = calendar::load();
    if let (Some(dst), Some(src)) = (v["settings"].as_object_mut(), settings.as_object()) { for (k, x) in src { dst.insert(k.clone(), x.clone()); } }
    calendar::save(&v)
}

#[tauri::command(async)]
fn cal_import_text(text: String) -> Result<Value, String> {
    let mut v = calendar::load();
    let (added, updated) = calendar::import(&mut v, &text)?;
    calendar::save(&v)?;
    Ok(json!({"added": added, "updated": updated}))
}

/// .ics files in Downloads, Documents and on the Desktop (for importing with a controller: no file dialog needed).
#[tauri::command(async)]
fn cal_ics_files() -> Vec<Value> {
    let mut out = Vec::new();
    for d in paths::ics_places() {
        for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
            let p = e.path();
            if p.extension().map(|x| x.eq_ignore_ascii_case("ics")).unwrap_or(false) && p.is_file() {
                out.push(json!({"path": p, "name": e.file_name().to_string_lossy(), "folder": d.file_name().map(|n| n.to_string_lossy().into_owned()), "modified": store::mtime(&p)}));
            }
        }
    }
    out.sort_by_key(|v| -v["modified"].as_i64().unwrap_or(0));
    out
}

#[tauri::command(async)]
fn cal_import_file(path: String) -> Result<Value, String> {
    let p = std::path::PathBuf::from(&path);
    if !p.extension().map(|x| x.eq_ignore_ascii_case("ics")).unwrap_or(false) { return Err("Only .ics files can be imported.".into()); }
    let text = std::fs::read_to_string(&p).map_err(|e| format!("Couldn't read it: {e}"))?;
    cal_import_text(text)
}

/// Writes your events to an .ics file in Documents. Returns its path.
#[tauri::command(async)]
fn cal_export() -> Result<String, String> {
    let v = calendar::load();
    let ev = calendar::own_events(&v);
    if ev.is_empty() { return Err("There are no events of yours to export yet.".into()); }
    let dir = dirs::document_dir().or_else(|| dirs::home_dir().map(|h| h.join("Documents"))).ok_or("No Documents folder.")?;
    let p = dir.join(format!("Hearth Calendar {}.ics", chrono::Local::now().format("%Y-%m-%d")));
    store::write_bytes(&p, hearth_time::ics::export(&ev).as_bytes())?;
    Ok(p.to_string_lossy().into_owned())
}

#[tauri::command(async)]
fn cal_subscribe(url: String, name: String) -> Result<Value, String> {
    let mut v = calendar::load();
    let s = calendar::subscribe(&mut v, &url, &name)?;
    calendar::save(&v)?;
    Ok(s)
}

#[tauri::command(async)]
fn cal_unsubscribe(id: String) -> Result<(), String> {
    let mut v = calendar::load();
    calendar::unsubscribe(&mut v, &id);
    calendar::save(&v)
}

#[tauri::command(async)]
fn cal_sub_set(id: String, hidden: Option<bool>, name: Option<String>) -> Result<(), String> {
    let mut v = calendar::load();
    if let Some(s) = v["subs"].as_array_mut().and_then(|l| l.iter_mut().find(|s| s["id"] == id.as_str())) {
        if let Some(h) = hidden { s["hidden"] = json!(h); }
        if let Some(n) = name.filter(|n| !n.trim().is_empty()) { s["name"] = json!(n.trim()); }
    }
    calendar::save(&v)
}

/// Downloads every subscribed calendar again. Returns the subscriptions (with any errors).
#[tauri::command(async)]
fn cal_refresh() -> Result<Value, String> {
    let mut v = calendar::load();
    if let Some(subs) = v["subs"].as_array_mut() { for s in subs.iter_mut() { let _ = calendar::refresh_sub(s); } }
    calendar::save(&v)?;
    Ok(v["subs"].clone())
}

// ---------------------------------------------------------------------------------------------- Notes

#[tauri::command(async)]
fn notes_list() -> Value { json!({"notes": notes::list(), "dir": notes::dir()}) }

#[tauri::command(async)]
fn note_read(name: String) -> Result<String, String> { notes::read_in(&notes::dir(), &name) }

#[tauri::command(async)]
fn note_write(name: String, text: String) -> Result<Value, String> { notes::write_in(&notes::dir(), &name, &text) }

#[tauri::command(async)]
fn note_delete(name: String) -> Result<(), String> { notes::delete_in(&notes::dir(), &name) }

#[tauri::command(async)]
fn note_pin(name: String, on: bool) -> Result<(), String> { notes::pin_in(&notes::dir(), &name, on) }

#[tauri::command(async)]
fn note_image_save(data: String) -> Result<String, String> { notes::save_image_in(&notes::dir(), &data) }

#[tauri::command(async)]
fn note_image(path: String) -> Result<String, String> { notes::image_in(&notes::dir(), &path) }

// ---------------------------------------------------------------------------------------------- Weather

#[tauri::command(async)]
fn weather_get(force: bool) -> Value {
    let s = weather::settings();
    match weather::current(force) {
        Ok(f) => json!({"settings": s, "forecast": f, "now": hearth_time::now_ms()}),
        Err(e) => json!({"settings": s, "forecast": weather::cached(), "error": e, "now": hearth_time::now_ms()}),
    }
}

#[tauri::command(async)]
fn weather_search(q: String) -> Result<Vec<Value>, String> { weather::search(&q) }

#[tauri::command(async)]
fn weather_set(place: Option<Value>, units: Option<String>) -> Result<Value, String> { weather::set(place, units) }

// ---------------------------------------------------------------------------------------------- start

/// Names this program for the desktop (Wayland app id / X11 class): io.github.dragongamer9.hearth.<app>, which
/// the .desktop files name as StartupWMClass.
#[cfg(target_os = "linux")]
fn set_program_name(id: &str) {
    #[link(name = "glib-2.0")]
    extern "C" { fn g_set_prgname(name: *const std::ffi::c_char); }
    if let Ok(c) = std::ffi::CString::new(id) { unsafe { g_set_prgname(c.as_ptr()); } }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let has = |f: &str| args.iter().any(|a| a == f);
    let value = |f: &str| args.iter().position(|a| a == f).and_then(|i| args.get(i + 1)).cloned();
    if has("--help") || has("-h") { print!("{USAGE}"); return; }
    if has("--version") { println!("hearth-time {}", env!("CARGO_PKG_VERSION")); return; }
    if has("--service") { std::process::exit(service::run()); }
    if has("--events") {
        let i = args.iter().position(|a| a == "--events").unwrap_or(0);
        let (from, to) = (args.get(i + 1).cloned().unwrap_or_default(), args.get(i + 2).cloned().unwrap_or_default());
        match calendar::events_json(&from, &to) {
            Ok(v) => { println!("{v}"); return; }
            Err(e) => { eprintln!("hearth-time: {e}"); std::process::exit(2); }
        }
    }
    if has("--weather-summary") { println!("{}", weather::summary_json()); return; }

    let ring_list: Vec<(String, String)> = value("--ring").map(|r| r.split(',').filter_map(|x| {
        let (k, id) = x.split_once(':')?;
        (k == "alarm" || k == "timer").then(|| (k.to_string(), id.to_string()))
    }).collect()).unwrap_or_default();
    let ringing = !ring_list.is_empty();
    let app: &'static str = if ringing { "clock" } else {
        value("--app").and_then(|a| hearth_time::app_from_name(&a))
            .or_else(|| args.first().and_then(|a| hearth_time::app_from_name(a)))
            .unwrap_or("clock")
    };
    let fullscreen = has("--fullscreen");
    let identifier = format!("io.github.dragongamer9.hearth.{app}");

    #[cfg(target_os = "linux")]
    {
        set_program_name(&identifier);
        // WebKit's newer GPU path draws a black window on some Nvidia drivers; these apps don't need its speed.
        if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() { std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1"); }
    }

    let (title, page, size) = match (ringing, app) {
        (true, _) => ("Ringing - Clock", "ring.html", (900.0, 600.0)),
        (_, "calendar") => ("Calendar", "calendar.html", (1180.0, 780.0)),
        (_, "notes") => ("Notes", "notes.html", (1100.0, 740.0)),
        (_, "weather") => ("Weather", "weather.html", (1100.0, 740.0)),
        _ => ("Clock", "clock.html", (1000.0, 700.0)),
    };
    let pin = ringing && std::env::var("HEARTH_RING_PIN").map(|v| v == "1").unwrap_or(false);

    let mut ctx = tauri::generate_context!();
    ctx.config_mut().identifier = identifier;
    let built = tauri::Builder::default()
        .manage(Start { app, ring: ring_list })
        .invoke_handler(tauri::generate_handler![
            init, quit, open_app, open_folder,
            clock_get, clock_save, clock_next, ring_stop, ring_info, ring_done,
            cal_get, cal_range, cal_event, cal_put, cal_delete, cal_settings, cal_import_text, cal_ics_files, cal_import_file,
            cal_export, cal_subscribe, cal_unsubscribe, cal_sub_set, cal_refresh,
            notes_list, note_read, note_write, note_delete, note_pin, note_image_save, note_image,
            weather_get, weather_search, weather_set
        ])
        .setup(move |app| {
            let mut b = WebviewWindowBuilder::new(app, "main", WebviewUrl::App(page.into()))
                .title(title)
                .inner_size(size.0, size.1)
                .min_inner_size(400.0, 360.0)
                .fullscreen(fullscreen || ringing)
                .background_color(tauri::window::Color(15, 11, 9, 255));
            if ringing { b = b.always_on_top(true).decorations(false); }
            let w = b.build()?;
            let _ = w.set_focus();
            if pin {
                // Game Mode: in front of the game (ring.rs).
                std::thread::spawn(|| { if let Some(x) = ring::pin_in_front() { PINNED.store(x, Ordering::SeqCst); } });
            }
            Ok(())
        })
        .build(ctx)
        .expect("Hearth Time couldn't start");
    built.run(|_, e| {
        if let tauri::RunEvent::Exit = e {
            let w = PINNED.swap(0, Ordering::SeqCst);
            if w != 0 { ring::unpin(w); }
        }
    });
}
