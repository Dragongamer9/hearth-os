//! Hearth Terminal: Hearth OS's terminal, on Linux (Mantel) and Windows (Ember). The window is ui/index.html
//! (xterm.js, drawn by the system's web view: WebKitGTK or WebView2); each tab or split is a shell in its own
//! pseudo-terminal (pty.rs), streamed to the page over a Tauri channel.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use hearth_terminal::{cli, keyboard, pty, settings, shells};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use tauri::ipc::Channel;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_opener::OpenerExt;

/// What a shell sends to the page: output, or that it ended.
#[derive(Serialize, Clone)]
#[serde(tag = "t", rename_all = "lowercase")]
enum Out {
    Data { d: String },
    Exit { code: Option<u32> },
}

#[derive(Default)]
struct Terms {
    next: AtomicU32,
    open: Mutex<HashMap<u32, Arc<pty::Pty>>>,
}

struct Startup {
    launch: cli::Launch,
    /// Environment variables this program set for WebKit, kept away from the shells.
    our_env: Vec<String>,
}

fn term(terms: &Terms, id: u32) -> Result<Arc<pty::Pty>, String> {
    terms.open.lock().unwrap().get(&id).cloned().ok_or_else(|| "That tab is closed.".to_string())
}

/// Everything the page needs to start: how it was opened, saved settings, the shells there are.
#[tauri::command(async)]
fn init(start: tauri::State<Startup>) -> Value {
    let list = shells::available();
    json!({
        "launch": start.launch,
        "settings": settings::load(),
        "shells": list.iter().map(|s| json!({ "id": s.id, "name": s.name })).collect::<Vec<_>>(),
        "platform": if cfg!(windows) { "windows" } else { "linux" },
        "home": dirs::home_dir(),
        "version": env!("CARGO_PKG_VERSION"),
    })
}

/// Starts a shell (or `command`) for a new tab or split. Returns its id; output comes on `events`.
#[tauri::command(async)]
fn pty_spawn(
    terms: tauri::State<Terms>,
    start: tauri::State<Startup>,
    shell: Option<String>,
    cwd: Option<String>,
    command: Option<Vec<String>>,
    cols: u16,
    rows: u16,
    events: Channel<Out>,
) -> Result<u32, String> {
    let sh = shells::pick(shell.as_deref());
    let (program, args, env) = match command.filter(|c| !c.is_empty()) {
        Some(c) => (c[0].clone(), c[1..].to_vec(), vec![]),
        None => (sh.program.clone(), sh.args.clone(), sh.env.clone()),
    };
    let opts = pty::SpawnOptions {
        program,
        args,
        cwd: cwd.filter(|c| !c.is_empty()).map(PathBuf::from),
        env,
        env_remove: start.our_env.clone(),
        cols,
        rows,
    };
    let send = move |e: pty::Event| {
        let _ = events.send(match e {
            pty::Event::Data(d) => Out::Data { d },
            pty::Event::Exit(code) => Out::Exit { code },
        });
    };
    let p = match pty::Pty::spawn(opts.clone(), send.clone()) {
        Ok(p) => p,
        // Windows: `-e dir` names a Command Prompt command, not a program; run it through cmd /c.
        #[cfg(windows)]
        Err(_) if opts.program != sh.program => {
            let mut o = opts.clone();
            o.args = std::iter::once("/c".to_string()).chain(std::iter::once(opts.program.clone())).chain(opts.args.clone()).collect();
            o.program = shells::available().into_iter().find(|s| s.id == "cmd").map(|s| s.program).unwrap_or_else(|| "cmd.exe".into());
            pty::Pty::spawn(o, send).map_err(|e| format!("{e:#}"))?
        }
        Err(e) => return Err(format!("{e:#}")),
    };
    let id = terms.next.fetch_add(1, Ordering::SeqCst) + 1;
    terms.open.lock().unwrap().insert(id, Arc::new(p));
    Ok(id)
}

#[tauri::command]
fn pty_write(terms: tauri::State<Terms>, id: u32, data: String) -> Result<(), String> {
    term(&terms, id)?.write(data.as_bytes()).map_err(|e| e.to_string())
}

#[tauri::command]
fn pty_resize(terms: tauri::State<Terms>, id: u32, cols: u16, rows: u16) -> Result<(), String> {
    term(&terms, id)?.resize(cols, rows).map_err(|e| e.to_string())
}

/// Closes a tab's shell (and whatever runs in it).
#[tauri::command(async)]
fn pty_close(terms: tauri::State<Terms>, id: u32) {
    let p = terms.open.lock().unwrap().remove(&id);
    if let Some(p) = p { p.kill(); }
}

/// The shell's current folder (Linux), for a new tab or split in the same place.
#[tauri::command]
fn pty_cwd(terms: tauri::State<Terms>, id: u32) -> Option<String> {
    term(&terms, id).ok()?.cwd().map(|p| p.to_string_lossy().into_owned())
}

#[tauri::command(async)]
fn settings_save(value: Value) -> Result<(), String> {
    settings::save(&value).map_err(|e| e.to_string())
}

#[tauri::command(async)]
fn clipboard_read(app: tauri::AppHandle) -> String {
    app.clipboard().read_text().unwrap_or_default()
}

#[tauri::command(async)]
fn clipboard_write(app: tauri::AppHandle, text: String) -> Result<(), String> {
    app.clipboard().write_text(text).map_err(|e| e.to_string())
}

/// A link clicked in the terminal: web and mail links open in their app; nothing else is opened.
#[tauri::command(async)]
fn open_url(app: tauri::AppHandle, url: String) -> Result<(), String> {
    let lower = url.to_ascii_lowercase();
    if !(lower.starts_with("http://") || lower.starts_with("https://") || lower.starts_with("mailto:")) {
        return Err("Only web and mail links open from here.".into());
    }
    app.opener().open_url(url, None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command(async)]
fn osk(open: bool) -> Result<(), String> { keyboard::set(open) }

#[tauri::command]
fn quit(app: tauri::AppHandle) { app.exit(0); }

/// WebKit's newer drawing path shows a black window on older Nvidia drivers and on basic graphics: there, use
/// the older one (the same rule as the Hearth app, hosts/hearth-rs).
#[cfg(target_os = "linux")]
fn pick_drawing() -> Vec<String> {
    const VAR: &str = "WEBKIT_DISABLE_DMABUF_RENDERER";
    if std::env::var_os(VAR).is_some() { return vec![]; }
    let fallback = dirs::config_dir().map(|d| d.join("nimbus").join("drawing-fallback").exists()).unwrap_or(false)
        || std::fs::read_to_string("/proc/cmdline").map(|c| c.split_whitespace().any(|w| w == "nomodeset")).unwrap_or(false);
    let nvidia = std::fs::read_dir("/sys/class/drm").into_iter().flatten().flatten()
        .any(|e| std::fs::read_to_string(e.path().join("device/vendor")).map(|v| v.trim() == "0x10de").unwrap_or(false));
    let new_driver = std::fs::read_to_string("/sys/module/nvidia/version").unwrap_or_default()
        .trim().split('.').next().and_then(|m| m.parse::<u32>().ok()).map(|m| m >= 560).unwrap_or(false);
    if fallback || (nvidia && !new_driver) {
        std::env::set_var(VAR, "1");
        return vec![VAR.into()];
    }
    vec![]
}

fn main() {
    let launch = match cli::parse(std::env::args().skip(1)) {
        Ok(cli::Parsed::Run(l)) => l,
        Ok(cli::Parsed::Help) => { print!("{}", cli::USAGE); return; }
        Ok(cli::Parsed::Version) => { println!("hearth-terminal {}", env!("CARGO_PKG_VERSION")); return; }
        Err(e) => { eprintln!("hearth-terminal: {e}\n\n{}", cli::USAGE); std::process::exit(2); }
    };
    #[cfg(target_os = "linux")]
    let our_env = pick_drawing();
    #[cfg(not(target_os = "linux"))]
    let our_env: Vec<String> = vec![];

    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_opener::init())
        .manage(Terms::default())
        .manage(Startup { launch, our_env })
        .invoke_handler(tauri::generate_handler![
            init, pty_spawn, pty_write, pty_resize, pty_close, pty_cwd, settings_save,
            clipboard_read, clipboard_write, open_url, osk, quit
        ])
        .setup(|app| {
            // See-through background only when it's asked for (Settings › Opacity): a see-through window costs
            // a little drawing, and some setups draw it black.
            let saved = settings::load();
            let see_through = saved.get("opacity").and_then(Value::as_f64).map(|o| o < 0.999).unwrap_or(false);
            let w = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title("Terminal")
                .inner_size(1000.0, 640.0)
                .min_inner_size(360.0, 240.0)
                .transparent(see_through)
                .build()?;
            let _ = w.set_focus();
            Ok(())
        })
        .on_window_event(|w, e| {
            // Closing the window ends every shell in it.
            if let tauri::WindowEvent::Destroyed = e {
                if let Some(t) = w.app_handle().try_state::<Terms>() {
                    for (_, p) in t.open.lock().unwrap().drain() { p.kill(); }
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("couldn't start Hearth Terminal");
}
