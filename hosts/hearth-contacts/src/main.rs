// Hearth Contacts' window (Tauri: WebKitGTK on Linux). The page (ui/index.html) shows the address books and
// contacts and edits them through the commands below; the work is in the library (store.rs, vcard.rs, carddav.rs).
//   hearth-contacts                 open the address book
//   hearth-contacts FILE.vcf        open it and offer to add the contacts in FILE (the .vcf handler)
//   hearth-contacts --export FILE   write every contact to FILE without a window
//   hearth-contacts --import FILE   add the contacts in FILE to "On this computer" without a window
//   hearth-contacts --list          every contact as JSON, without a window
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use hearth_contacts::{carddav, secrets, store::{self, Account, Store}, vcard::Contact};
use serde_json::{json, Value};
use std::sync::Mutex;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

struct Launch(Mutex<Option<String>>);

fn st() -> Store { Store::open_default() }

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> Result<T, String> + Send + 'static) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn state() -> Result<Value, String> {
    blocking(|| {
        let s = st();
        let accounts: Vec<Value> = s.accounts().into_iter().map(|a| json!({"id": a.id, "name": a.name, "provider": a.provider, "server": a.server,
            "username": a.username, "bookName": a.book_name, "lastSync": a.last_sync, "lastError": a.last_error})).collect();
        Ok(json!({"books": s.books(), "contacts": s.list(), "accounts": accounts, "presets": carddav::presets()}))
    }).await
}

#[tauri::command]
async fn save(book: String, contact: Contact) -> Result<Contact, String> { blocking(move || st().save(&book, contact)).await }

#[tauri::command]
async fn delete(book: String, uid: String) -> Result<(), String> { blocking(move || st().delete(&book, &uid)).await }

#[tauri::command]
async fn move_to(from: String, uid: String, to: String) -> Result<(), String> { blocking(move || st().move_to(&from, &uid, &to)).await }

#[tauri::command]
async fn preview_file(path: String) -> Result<Value, String> {
    blocking(move || {
        let text = std::fs::read_to_string(&path).map_err(|e| format!("Couldn't open {path}: {e}"))?;
        let cards = hearth_contacts::vcard::parse(&text);
        if cards.is_empty() { return Err("There are no contacts in that file.".into()); }
        Ok(json!({"path": path, "count": cards.len(), "names": cards.iter().take(8).map(|c| c.shown_name()).collect::<Vec<_>>()}))
    }).await
}

#[tauri::command]
async fn import_file(path: String, book: String) -> Result<store::ImportReport, String> {
    blocking(move || {
        let text = std::fs::read_to_string(&path).map_err(|e| format!("Couldn't open {path}: {e}"))?;
        st().import(&book, &text)
    }).await
}

/// `which`: [book, uid] pairs; empty means every contact.
#[tauri::command]
async fn export_file(path: String, which: Vec<(String, String)>) -> Result<usize, String> {
    blocking(move || {
        let text = st().export(&which);
        let n = text.matches("BEGIN:VCARD").count();
        let path = if path.to_lowercase().ends_with(".vcf") { path } else { format!("{path}.vcf") };
        store::write_atomic(std::path::Path::new(&path), text.as_bytes())?;
        Ok(n)
    }).await
}

/// Finds the address books on a server. The password is only used for this, and kept nowhere.
#[tauri::command]
async fn discover(server: String, username: String, password: String) -> Result<Vec<carddav::BookInfo>, String> {
    blocking(move || carddav::discover(&server, &username, &password)).await
}

/// Adds the account: its password goes into the system keyring, then its address book is copied here.
#[tauri::command]
async fn add_account(name: String, provider: String, server: String, username: String, password: String, book_url: String, book_name: String) -> Result<Value, String> {
    blocking(move || {
        let s = st();
        let id = format!("dav-{}", &store::new_uid()[..8]);
        secrets::set(&id, &password)?;
        let mut a = Account { id: id.clone(), name, provider, server, username, book_url, book_name, ..Default::default() };
        if let Err(e) = s.put_account(a.clone()) { secrets::remove(&id); return Err(e); }
        let r = carddav::sync(&s, &a, &password);
        a.last_sync = store::now_rev();
        a.last_error = r.as_ref().err().cloned().unwrap_or_default();
        s.put_account(a)?;
        Ok(json!({"id": id, "report": r.ok(), "error": r.err()}))
    }).await
}

#[tauri::command]
async fn remove_account(id: String, keep_copy: bool) -> Result<(), String> {
    blocking(move || { st().remove_account(&id, keep_copy)?; secrets::remove(&id); Ok(()) }).await
}

/// Changes the account's password (kept in the keyring) after the server stopped taking the old one.
#[tauri::command]
async fn set_password(id: String, password: String) -> Result<(), String> {
    blocking(move || secrets::set(&id, &password)).await
}

/// Syncs one account, or every account when `id` is empty.
#[tauri::command]
async fn sync(id: String) -> Result<Value, String> {
    blocking(move || {
        let s = st();
        let mut out = Vec::new();
        for mut a in s.accounts().into_iter().filter(|a| id.is_empty() || a.id == id) {
            let r = secrets::get(&a.id).and_then(|pw| carddav::sync(&s, &a, &pw));
            a.last_sync = store::now_rev();
            a.last_error = r.as_ref().err().cloned().unwrap_or_default();
            let _ = s.put_account(a.clone());
            out.push(json!({"id": a.id, "name": a.name, "report": r.as_ref().ok(), "error": r.err()}));
        }
        Ok(json!(out))
    }).await
}

/// A picture for a contact's photo, as a data: URL (the page makes it small before saving).
#[tauri::command]
async fn read_image(path: String) -> Result<String, String> {
    use base64::Engine;
    blocking(move || {
        let meta = std::fs::metadata(&path).map_err(|e| format!("Couldn't open {path}: {e}"))?;
        if meta.len() > 40 << 20 { return Err("That picture is too big (over 40 MB).".into()); }
        let data = std::fs::read(&path).map_err(|e| format!("Couldn't open {path}: {e}"))?;
        let mime = match path.rsplit('.').next().map(|x| x.to_ascii_lowercase()).as_deref() {
            Some("png") => "image/png", Some("gif") => "image/gif", Some("webp") => "image/webp", Some("bmp") => "image/bmp", _ => "image/jpeg",
        };
        Ok(format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(data)))
    }).await
}

/// Opens tel:, mailto: and web links with the apps the system has for them.
#[tauri::command]
fn open_uri(uri: String) -> Result<(), String> {
    let lower = uri.to_ascii_lowercase();
    if !["tel:", "mailto:", "https://", "http://"].iter().any(|p| lower.starts_with(p)) { return Err("That kind of link isn't opened from here.".into()); }
    open_with_system(&uri)
}

/// An address on a map: in Hearth Maps when it's installed, otherwise OpenStreetMap in the browser.
#[tauri::command]
fn open_map(query: String) -> Result<(), String> {
    let maps = which("hearth-maps");
    if let Some(m) = maps {
        return std::process::Command::new(m).arg("--search").arg(&query).spawn().map(|_| ()).map_err(|e| e.to_string());
    }
    let enc: String = query.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect();
    open_with_system(&format!("https://www.openstreetmap.org/search?query={enc}"))
}

fn which(name: &str) -> Option<std::path::PathBuf> {
    std::env::var_os("PATH").and_then(|p| std::env::split_paths(&p).map(|d| d.join(name)).find(|f| f.is_file()))
}

fn open_with_system(uri: &str) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    let r = std::process::Command::new("xdg-open").arg(uri).spawn();
    #[cfg(windows)]
    let r = std::process::Command::new("rundll32").args(["url.dll,FileProtocolHandler", uri]).spawn();
    #[cfg(target_os = "macos")]
    let r = std::process::Command::new("open").arg(uri).spawn();
    r.map(|_| ()).map_err(|e| format!("Couldn't open it: {e}"))
}

/// The .vcf file Contacts was opened with (once).
#[tauri::command]
fn launch_file(launch: tauri::State<Launch>) -> Option<String> { launch.0.lock().ok().and_then(|mut g| g.take()) }

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--list") {
        println!("{}", serde_json::to_string_pretty(&st().list()).unwrap_or_default());
        return;
    }
    if let Some(i) = args.iter().position(|a| a == "--import") {
        let Some(path) = args.get(i + 1) else { eprintln!("--import needs a file name"); std::process::exit(2) };
        let r = std::fs::read_to_string(path).map_err(|e| e.to_string()).and_then(|t| st().import(store::LOCAL, &t));
        match r {
            Ok(r) => { println!("{} added, {} updated, {} skipped", r.added, r.updated, r.skipped); return; }
            Err(e) => { eprintln!("{e}"); std::process::exit(1); }
        }
    }
    if let Some(i) = args.iter().position(|a| a == "--export") {
        let Some(path) = args.get(i + 1) else { eprintln!("--export needs a file name"); std::process::exit(2) };
        let text = st().export(&[]);
        if let Err(e) = store::write_atomic(std::path::Path::new(path), text.as_bytes()) { eprintln!("{e}"); std::process::exit(1); }
        println!("{} contacts written to {path}", text.matches("BEGIN:VCARD").count());
        return;
    }
    let fullscreen = args.iter().any(|a| a == "--fullscreen");
    // A .vcf given on the command line (or as a file:// URI from a file manager).
    let file = args.iter().find(|a| !a.starts_with("--") && a.as_str() != "").map(|a| a.strip_prefix("file://").map(|p| p.replace("%20", " ")).unwrap_or_else(|| a.clone()));
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() { std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1"); }
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(Launch(Mutex::new(file)))
        .invoke_handler(tauri::generate_handler![state, save, delete, move_to, preview_file, import_file, export_file, discover, add_account, remove_account, set_password, sync, launch_file, read_image, open_uri, open_map])
        .setup(move |app| {
            WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title("Contacts")
                .inner_size(1180.0, 760.0)
                .min_inner_size(560.0, 420.0)
                .fullscreen(fullscreen)
                .background_color(tauri::window::Color(15, 11, 9, 255))
                .build()?;
            let _ = app.get_webview_window("main");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Contacts couldn't start");
}
