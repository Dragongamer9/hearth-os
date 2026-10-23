// Hearth Office's window (Tauri: WebKitGTK on Linux, WebView2 on Windows). One program for the three apps:
// the page (ui/index.html) asks `init` which one it is (Write, Sheets or Slides) and which file to open, and does
// all the document work itself (formats in ui/formats). This side reads and writes files, shows the file
// pickers, keeps recent files and recovery copies (lib.rs), guards the window against closing with unsaved
// changes, prints, and exports PDFs.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use base64::Engine;
use hearth_office::{App, Launch, Parsed};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

/// The document has changes that aren't saved: closing the window asks the page first.
static DIRTY: AtomicBool = AtomicBool::new(false);

const B64: base64::engine::GeneralPurpose = base64::engine::general_purpose::STANDARD;

#[tauri::command(async)]
fn init(start: tauri::State<Launch>) -> Value {
    json!({
        "launch": start.inner(),
        "app": start.app,
        "settings": hearth_office::settings_load(),
        "recent": hearth_office::recent_for(start.app),
        "recovery": hearth_office::recovery_list(start.app),
        "platform": if cfg!(windows) { "windows" } else { "linux" },
        "home": dirs::home_dir(),
        "documents": dirs::document_dir(),
        "version": env!("CARGO_PKG_VERSION"),
    })
}

/// A file's bytes, as base64 (with its size and when it was changed).
#[tauri::command(async)]
fn read_file(path: String) -> Result<Value, String> {
    let data = std::fs::read(&path).map_err(|e| format!("Couldn't open {path}: {e}"))?;
    let modified = std::fs::metadata(&path).ok().and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0);
    Ok(json!({"data": B64.encode(&data), "size": data.len(), "modified": modified}))
}

/// Saves a file whole (to a temporary file first, so a failed save never leaves half a file).
#[tauri::command(async)]
fn write_file(path: String, data: String) -> Result<(), String> {
    let bytes = B64.decode(data.as_bytes()).map_err(|e| e.to_string())?;
    hearth_office::write_atomic(&PathBuf::from(&path), &bytes).map_err(|e| format!("Couldn't save {path}: {e}"))
}

fn filters(kind: &str) -> Vec<(&'static str, Vec<&'static str>)> {
    match kind {
        "write" => vec![
            ("Documents", vec!["docx", "odt", "rtf", "txt", "md", "markdown", "html", "htm", "hwrite"]),
            ("Word document", vec!["docx"]), ("OpenDocument text", vec!["odt"]), ("Rich text", vec!["rtf"]),
            ("Plain text", vec!["txt"]), ("Markdown", vec!["md", "markdown"]), ("Web page", vec!["html", "htm"]),
        ],
        "sheets" => vec![
            ("Spreadsheets", vec!["xlsx", "xlsm", "xls", "ods", "csv", "tsv", "hsheet"]),
            ("Excel workbook", vec!["xlsx", "xlsm", "xls"]), ("OpenDocument spreadsheet", vec!["ods"]), ("CSV", vec!["csv", "tsv"]),
        ],
        "slides" => vec![("Presentations", vec!["pptx", "hslides"]), ("PowerPoint", vec!["pptx"]), ("Hearth Slides", vec!["hslides"])],
        "image" => vec![("Pictures", vec!["png", "jpg", "jpeg", "gif", "webp", "svg", "bmp"])],
        "csv" => vec![("CSV", vec!["csv", "tsv", "txt"])],
        _ => vec![],
    }
}

fn default_dir() -> Option<PathBuf> { dirs::document_dir().or_else(dirs::home_dir) }

/// The system's file picker for opening: `kind` is the app (write, sheets, slides) or "image".
#[tauri::command(async)]
fn pick_open(app: tauri::AppHandle, kind: String) -> Option<String> {
    let mut d = app.dialog().file();
    for (name, exts) in filters(&kind) { d = d.add_filter(name, &exts); }
    if let Some(dir) = default_dir() { d = d.set_directory(dir); }
    d.blocking_pick_file().and_then(|p| p.into_path().ok()).map(|p| p.to_string_lossy().into_owned())
}

/// The system's file picker for saving: `name` is the suggested file name, `exts` the allowed extensions.
#[tauri::command(async)]
fn pick_save(app: tauri::AppHandle, name: String, label: String, exts: Vec<String>, dir: Option<String>) -> Option<String> {
    let refs: Vec<&str> = exts.iter().map(String::as_str).collect();
    let mut d = app.dialog().file().set_file_name(&name);
    if !refs.is_empty() { d = d.add_filter(&label, &refs); }
    match dir.map(PathBuf::from).filter(|p| p.is_dir()).or_else(default_dir) { Some(p) => d = d.set_directory(p), None => {} }
    let p = d.blocking_save_file().and_then(|p| p.into_path().ok())?;
    // Add the extension when it was left off.
    let mut p = p;
    if let Some(first) = exts.first() {
        let has = p.extension().map(|e| exts.iter().any(|x| x.eq_ignore_ascii_case(&e.to_string_lossy()))).unwrap_or(false);
        if !has { let mut s = p.into_os_string(); s.push(format!(".{first}")); p = PathBuf::from(s); }
    }
    Some(p.to_string_lossy().into_owned())
}

#[tauri::command(async)]
fn recent_add(start: tauri::State<Launch>, path: String) -> Value {
    let _ = hearth_office::recent_add(start.app, &path);
    hearth_office::recent_for(start.app)
}

#[tauri::command(async)]
fn recent_remove(start: tauri::State<Launch>, path: String) -> Value {
    let _ = hearth_office::recent_remove(&path);
    hearth_office::recent_for(start.app)
}

#[tauri::command(async)]
fn recent_list(start: tauri::State<Launch>) -> Value { hearth_office::recent_for(start.app) }

#[tauri::command(async)]
fn recovery_save(id: String, meta: Value, body: String) -> Result<(), String> { hearth_office::recovery_save(&id, &meta, &body) }

#[tauri::command(async)]
fn recovery_clear(id: String) { hearth_office::recovery_clear(&id) }

#[tauri::command(async)]
fn recovery_load(id: String) -> Result<String, String> { hearth_office::recovery_load(&id) }

#[tauri::command(async)]
fn settings_save(value: Value) -> Result<(), String> { hearth_office::settings_save(&value).map_err(|e| e.to_string()) }

#[tauri::command]
fn set_dirty(dirty: bool) { DIRTY.store(dirty, Ordering::SeqCst); }

#[tauri::command]
fn set_title(window: tauri::WebviewWindow, title: String) { let _ = window.set_title(&title); }

#[tauri::command]
fn fullscreen(window: tauri::WebviewWindow, on: bool) { let _ = window.set_fullscreen(on); }

/// A link in a document: web and mail links open in their app; nothing else is opened.
#[tauri::command(async)]
fn open_url(app: tauri::AppHandle, url: String) -> Result<(), String> {
    if !hearth_office::link_ok(&url) { return Err("Only web and mail links open from a document.".into()); }
    app.opener().open_url(url, None::<&str>).map_err(|e| e.to_string())
}

/// Opens another window of Hearth Office (another app, or another file) as its own program.
#[tauri::command(async)]
fn open_window(app: String, file: Option<String>) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let mut c = std::process::Command::new(exe);
    c.arg("--app").arg(App::parse(&app).ok_or("no such app")?.id());
    if let Some(f) = file { c.arg(f); }
    c.spawn().map(|_| ()).map_err(|e| e.to_string())
}

/// Font families installed on this system (fontconfig), for the font menus.
#[tauri::command(async)]
fn fonts() -> Vec<String> {
    #[cfg(target_os = "linux")]
    {
        let out = std::process::Command::new("fc-list").args([":", "family"]).output().map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default();
        let mut v: Vec<String> = out.lines().filter_map(|l| l.split(',').next()).map(|s| s.trim().replace('\\', "")).filter(|s| !s.is_empty() && !s.starts_with('.')).collect();
        v.sort_by_key(|s| s.to_lowercase());
        v.dedup();
        v
    }
    #[cfg(not(target_os = "linux"))]
    { Vec::new() }
}

/// The system's print dialog for what the page shows (its print layout).
#[tauri::command]
fn print(window: tauri::WebviewWindow) -> Result<(), String> { window.print().map_err(|e| e.to_string()) }

/// Prints the page's print layout straight to a PDF file, `width_mm` × `height_mm` per page, no dialog
/// (Linux, through WebKitGTK). Answers "office://pdf" when done. Elsewhere: the print dialog (pick a PDF printer).
#[tauri::command]
fn export_pdf(app: tauri::AppHandle, window: tauri::WebviewWindow, path: String, width_mm: f64, height_mm: f64) -> Result<String, String> {
    #[cfg(target_os = "linux")]
    {
        let p2 = path.clone();
        window.with_webview(move |pw| {
            use gtk::prelude::*;
            use webkit2gtk::{PrintOperation, PrintOperationExt};
            let wv = pw.inner();
            let op = PrintOperation::new(&wv);
            let settings = gtk::PrintSettings::new();
            settings.set_printer("Print to File");
            settings.set("output-file-format", Some("pdf"));
            let uri = gtk::glib::filename_to_uri(&p2, None).map(|u| u.to_string()).unwrap_or_else(|_| format!("file://{p2}"));
            settings.set("output-uri", Some(&uri));
            let setup = gtk::PageSetup::new();
            let paper = gtk::PaperSize::new_custom("hearth-office", "Hearth Office", width_mm, height_mm, gtk::Unit::Mm);
            setup.set_paper_size(&paper);
            setup.set_orientation(gtk::PageOrientation::Portrait);
            setup.set_top_margin(0.0, gtk::Unit::Mm);
            setup.set_bottom_margin(0.0, gtk::Unit::Mm);
            setup.set_left_margin(0.0, gtk::Unit::Mm);
            setup.set_right_margin(0.0, gtk::Unit::Mm);
            op.set_print_settings(&settings);
            op.set_page_setup(&setup);
            let a1 = app.clone();
            let done_path = p2.clone();
            op.connect_finished(move |_| { let _ = a1.emit("office://pdf", json!({"ok": true, "path": done_path})); });
            let a2 = app.clone();
            op.connect_failed(move |_, e| { let _ = a2.emit("office://pdf", json!({"ok": false, "error": e.to_string()})); });
            op.print();
        }).map_err(|e| e.to_string())?;
        let _ = (width_mm, height_mm);
        Ok("started".into())
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (app, path, width_mm, height_mm);
        window.print().map_err(|e| e.to_string())?;
        Ok("dialog".into())
    }
}

/// `--convert`: the page saved the file (or couldn't); the program ends with that.
#[tauri::command]
fn convert_done(app: tauri::AppHandle, ok: bool, message: String) {
    if ok { println!("{message}"); } else { eprintln!("hearth-office: {message}"); }
    app.exit(if ok { 0 } else { 1 });
}

/// The page's own messages, for the log (and for CI to read).
#[tauri::command]
fn log(message: String) { eprintln!("[page] {message}"); }

#[tauri::command]
fn quit(app: tauri::AppHandle) {
    DIRTY.store(false, Ordering::SeqCst);
    app.exit(0);
}

/// WebKit's newer drawing path shows a black window on older Nvidia drivers and on basic graphics: there, use
/// the older one (the same rule as Hearth Terminal and the Hearth app).
#[cfg(target_os = "linux")]
fn pick_drawing() {
    const VAR: &str = "WEBKIT_DISABLE_DMABUF_RENDERER";
    if std::env::var_os(VAR).is_some() { return; }
    let fallback = dirs::config_dir().map(|d| d.join("nimbus").join("drawing-fallback").exists()).unwrap_or(false)
        || std::fs::read_to_string("/proc/cmdline").map(|c| c.split_whitespace().any(|w| w == "nomodeset")).unwrap_or(false);
    let nvidia = std::fs::read_dir("/sys/class/drm").into_iter().flatten().flatten()
        .any(|e| std::fs::read_to_string(e.path().join("device/vendor")).map(|v| v.trim() == "0x10de").unwrap_or(false));
    // Nvidia: WebKit's newer drawing path (DMA-BUF) crashes these windows on 615.x too, so the older path is used
    // on every Nvidia driver, as Hearth Text, Clock and the other apps already do.
    let _new_driver = std::fs::read_to_string("/sys/module/nvidia/version").unwrap_or_default()
        .trim().split('.').next().and_then(|m| m.parse::<u32>().ok()).map(|m| m >= 560).unwrap_or(false);
    if fallback || nvidia { std::env::set_var(VAR, "1"); }
}

fn main() {
    let mut args = std::env::args();
    let argv0 = args.next().unwrap_or_default();
    let launch = match hearth_office::parse(&argv0, args) {
        Ok(Parsed::Run(l)) => l,
        Ok(Parsed::Help) => { print!("{}", hearth_office::USAGE); return; }
        Ok(Parsed::Version) => { println!("hearth-office {}", env!("CARGO_PKG_VERSION")); return; }
        Err(e) => { eprintln!("hearth-office: {e}\n\n{}", hearth_office::USAGE); std::process::exit(2); }
    };
    #[cfg(target_os = "linux")]
    pick_drawing();
    let title = launch.app.title();
    let app_id = launch.app;
    let full = launch.fullscreen;
    let converting = launch.convert.is_some();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(launch)
        .invoke_handler(tauri::generate_handler![
            init, read_file, write_file, pick_open, pick_save, recent_add, recent_remove, recent_list,
            recovery_save, recovery_clear, recovery_load, settings_save, set_dirty, set_title, fullscreen,
            open_url, open_window, fonts, print, export_pdf, convert_done, log, quit
        ])
        .setup(move |app| {
            let (w, h) = match app_id { App::Sheets => (1280.0, 800.0), _ => (1200.0, 820.0) };
            let win = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title(title)
                .inner_size(w, h)
                .min_inner_size(560.0, 420.0)
                .fullscreen(full)
                .visible(!converting || std::env::var_os("HEARTH_OFFICE_SHOW").is_some())
                .background_color(tauri::window::Color(15, 11, 9, 255))
                .build()?;
            let _ = win.set_focus();
            Ok(())
        })
        .on_window_event(|w, e| {
            // Unsaved changes: the page asks (Save, Don't save, Cancel) and calls quit itself.
            if let tauri::WindowEvent::CloseRequested { api, .. } = e {
                if DIRTY.load(Ordering::SeqCst) {
                    api.prevent_close();
                    let _ = w.app_handle().emit("office://close", ());
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("couldn't start Hearth Office");
}
