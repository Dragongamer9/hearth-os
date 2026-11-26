//! OBS Studio, driven through its own remote control, obs-websocket (built into OBS 28 and newer; protocol v5):
//! one thread connects to it on this PC, signs in with the password OBS keeps in its own settings (or the one you type
//! into Hearth Studio), and hands every event and answer to the window; the window's requests are written straight to
//! the same connection. When OBS isn't running the thread waits (3 s, then up to 15 s between tries) and tries again.
//!
//! Also OBS's own files, only while OBS is closed (it rewrites them as it closes): turning its remote control on for
//! you (a new password only OBS and Hearth Studio know) and adding Hearth's Ember transition to your scene collection.
//! Both with settings_file: merged into the file as it is, its .bak kept (red flag 9).

use crate::ws::{self, Ws};
use futures::channel::mpsc::{unbounded, UnboundedSender};
use futures::Stream;
use serde_json::{json, Map, Value};
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone)]
pub enum Ev {
    Up,
    /// not connected, and why ("not running", "wrong password", …)
    Down(String),
    /// an event (op 5) or an answer to a request (op 7)
    Got(Value),
}

static WRITER: OnceLock<Mutex<Option<TcpStream>>> = OnceLock::new();
static PASSWORD: OnceLock<Mutex<String>> = OnceLock::new();

fn writer() -> &'static Mutex<Option<TcpStream>> {
    WRITER.get_or_init(|| Mutex::new(None))
}

/// The password you typed into Hearth Studio (used when OBS's own settings can't be read, e.g. another PC's OBS).
pub fn set_password(p: &str) {
    if let Ok(mut g) = PASSWORD.get_or_init(|| Mutex::new(String::new())).lock() {
        *g = p.to_string();
    }
}

/// A request to OBS (op 6): its type, an id to know its answer by, its data. Dropped when OBS isn't connected.
pub fn request(kind: &str, id: &str, data: Value) {
    let msg = json!({"op": 6, "d": {"requestType": kind, "requestId": id, "requestData": data}});
    if let Ok(mut g) = writer().lock() {
        if let Some(s) = g.as_mut() {
            if ws::send_text(s, &msg.to_string()).is_err() {
                *g = None;
            }
        }
    }
}

pub fn req(kind: &str, data: Value) {
    request(kind, kind, data);
}

/// OBS's config folder: the system OBS's, else Flathub's.
pub fn config_dir() -> PathBuf {
    let h = hearth_ui::paths::home();
    let flat = h.join(".var/app/com.obsproject.Studio/config/obs-studio");
    let own = h.join(".config/obs-studio");
    if !own.is_dir() && flat.is_dir() {
        flat
    } else {
        own
    }
}

pub fn ws_config() -> PathBuf {
    config_dir().join("plugin_config/obs-websocket/config.json")
}

/// (enabled, port, password) from OBS's own settings.
pub fn ws_settings() -> (bool, u16, String) {
    let m = hearth_ui::settings_file::read(&ws_config()).map().unwrap_or_default();
    let enabled = m.get("server_enabled").and_then(Value::as_bool).unwrap_or(false);
    let port = std::env::var("HEARTH_OBS_PORT").ok().and_then(|p| p.parse().ok()).or_else(|| m.get("server_port").and_then(Value::as_u64).map(|p| p as u16)).unwrap_or(4455);
    let auth = m.get("auth_required").and_then(Value::as_bool).unwrap_or(true);
    let pw = if auth { m.get("server_password").and_then(Value::as_str).unwrap_or("").to_string() } else { String::new() };
    (enabled || std::env::var("HEARTH_OBS_PORT").is_ok(), port, pw)
}

fn on_path(c: &str) -> bool {
    std::env::var_os("PATH").map(|p| std::env::split_paths(&p).any(|d| d.join(c).is_file())).unwrap_or(false)
}

fn flatpak_obs() -> bool {
    let h = hearth_ui::paths::home();
    h.join(".local/share/flatpak/app/com.obsproject.Studio").is_dir() || std::path::Path::new("/var/lib/flatpak/app/com.obsproject.Studio").is_dir()
}

pub fn installed() -> bool {
    std::env::var("HEARTH_OBS_PORT").is_ok() || on_path("obs") || flatpak_obs()
}

pub fn running() -> bool {
    if std::env::var("HEARTH_OBS_PORT").is_ok() {
        return true;
    }
    std::fs::read_dir("/proc")
        .map(|rd| rd.flatten().any(|e| e.file_name().to_string_lossy().chars().all(|c| c.is_ascii_digit()) && std::fs::read_to_string(e.path().join("comm")).map(|c| c.trim() == "obs").unwrap_or(false)))
        .unwrap_or(false)
}

/// Installs OBS Studio the Hearth way: from Flathub, for you only (no password).
pub fn install_cmd() -> Vec<String> {
    vec![
        "sh".into(),
        "-c".into(),
        "flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo && flatpak install --user -y --noninteractive flathub com.obsproject.Studio".into(),
    ]
}

/// Starts OBS in the background (its tray), its remote control and Hearth's Ember transition ready first.
pub fn start() -> Result<(), String> {
    prepare()?;
    let cmd: Vec<&str> = if on_path("obs") { vec!["obs", "--minimize-to-tray", "--disable-shutdown-check"] } else { vec!["flatpak", "run", "com.obsproject.Studio", "--minimize-to-tray", "--disable-shutdown-check"] };
    hearth_ui::run::detached(&cmd);
    Ok(())
}

/// With OBS closed: its remote control on (a password of its own when it has none), and the Ember transition in the
/// scene collection it opens. Nothing is written while OBS runs.
pub fn prepare() -> Result<(), String> {
    if running() {
        return Ok(());
    }
    let p = ws_config();
    if let Some(d) = p.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    hearth_ui::settings_file::update_json(&p, |m| {
        if m.get("server_enabled").and_then(Value::as_bool) != Some(true) {
            m.insert("server_enabled".into(), json!(true));
        }
        if !m.contains_key("server_port") {
            m.insert("server_port".into(), json!(4455));
        }
        if m.get("server_password").and_then(Value::as_str).map(|s| s.is_empty()).unwrap_or(true) {
            m.insert("auth_required".into(), json!(true));
            m.insert("server_password".into(), json!(ws::new_password()));
        }
        if !m.contains_key("alerts_enabled") {
            m.insert("alerts_enabled".into(), json!(false));
        }
    })
    .map_err(|e| e.to_string())?;
    add_ember();
    Ok(())
}

/// The scene collection OBS opens (global.ini or, from OBS 31, user.ini: [Basic] SceneCollectionFile).
fn collection_file() -> Option<PathBuf> {
    let d = config_dir();
    for ini in ["user.ini", "global.ini"] {
        let Ok(t) = std::fs::read_to_string(d.join(ini)) else { continue };
        let mut in_basic = false;
        for l in t.lines() {
            let l = l.trim();
            if l.starts_with('[') {
                in_basic = l == "[Basic]";
            } else if in_basic {
                if let Some(v) = l.strip_prefix("SceneCollectionFile=") {
                    let f = d.join("basic/scenes").join(format!("{}.json", v.trim()));
                    if f.is_file() {
                        return Some(f);
                    }
                }
            }
        }
    }
    None
}

/// Hearth's own transition: an ember-orange fade (OBS's "Fade to Color" in Hearth's accent, #ff8a3d).
fn add_ember() {
    let Some(f) = collection_file() else { return };
    let _ = hearth_ui::settings_file::update_json(&f, |m| {
        let mut list: Vec<Value> = m.get("transitions").and_then(Value::as_array).cloned().unwrap_or_default();
        if list.iter().any(|t| t.get("name").and_then(Value::as_str) == Some("Ember")) {
            return;
        }
        // OBS keeps colours as 0xAABBGGRR
        list.push(json!({"id": "fade_to_color_transition", "name": "Ember", "settings": {"color": 0xff3d8affu32, "switch_point": 50}}));
        m.insert("transitions".into(), Value::Array(list));
    });
}

/// OBS's events and answers, for as long as the window is open (Subscription::run).
pub fn stream() -> impl Stream<Item = Ev> {
    let (tx, rx) = unbounded();
    let _ = std::thread::Builder::new().name("hearth-studio-obs".into()).stack_size(512 * 1024).spawn(move || run(tx));
    rx
}

fn run(tx: UnboundedSender<Ev>) {
    let mut wait = 3u64;
    loop {
        let why = match session(&tx) {
            Ok(()) => "OBS closed the connection".to_string(),
            Err(e) => e,
        };
        if let Ok(mut g) = writer().lock() {
            *g = None;
        }
        if tx.unbounded_send(Ev::Down(why)).is_err() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_secs(wait));
        wait = (wait + 3).min(15);
    }
}

fn session(tx: &UnboundedSender<Ev>) -> Result<(), String> {
    let (enabled, port, mut pw) = ws_settings();
    if pw.is_empty() {
        pw = PASSWORD.get().and_then(|m| m.lock().ok().map(|g| g.clone())).unwrap_or_default();
    }
    if !installed() {
        return Err("not installed".into());
    }
    if !running() {
        return Err("not running".into());
    }
    let mut c = Ws::connect(port).map_err(|e| if enabled { format!("can't reach its remote control ({e})") } else { "remote control off".to_string() })?;
    let hello: Value = serde_json::from_str(&c.read().map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    let mut ident = Map::new();
    ident.insert("rpcVersion".into(), json!(1));
    // every ordinary event, and the sound meters (InputVolumeMeters, 1 << 16)
    ident.insert("eventSubscriptions".into(), json!(0x7ff | (1 << 16)));
    if let Some(a) = hello.get("d").and_then(|d| d.get("authentication")) {
        if pw.is_empty() {
            return Err("password needed".into());
        }
        let salt = a.get("salt").and_then(Value::as_str).unwrap_or("");
        let ch = a.get("challenge").and_then(Value::as_str).unwrap_or("");
        ident.insert("authentication".into(), json!(ws::auth(&pw, salt, ch)));
    }
    let mut w = c.writer().map_err(|e| e.to_string())?;
    ws::send_text(&mut w, &json!({"op": 1, "d": ident}).to_string()).map_err(|e| e.to_string())?;
    let ok: Value = serde_json::from_str(&c.read().map_err(|e| if e.to_string().contains("4009") { "wrong password".to_string() } else { e.to_string() })?).map_err(|e| e.to_string())?;
    if ok.get("op").and_then(Value::as_u64) != Some(2) {
        return Err("OBS didn't let Hearth Studio in".into());
    }
    if let Ok(mut g) = writer().lock() {
        *g = Some(w);
    }
    if tx.unbounded_send(Ev::Up).is_err() {
        return Ok(());
    }
    loop {
        let t = c.read().map_err(|e| e.to_string())?;
        let Ok(v) = serde_json::from_str::<Value>(&t) else { continue };
        if matches!(v.get("op").and_then(Value::as_u64), Some(5) | Some(7)) && tx.unbounded_send(Ev::Got(v)).is_err() {
            return Ok(());
        }
    }
}
