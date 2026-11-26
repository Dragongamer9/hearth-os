//! Steam on this PC, only through its own files (never a sign-in, a token or an account: red flag 6): where it is,
//! the people who use it here (userdata/<id>, their names from config/loginusers.vdf), each one's non-Steam games
//! (shortcuts.vdf) and art folder (config/grid), whether Steam is running, and closing and opening it again.

use crate::vdf::{self, Node};
use std::path::{Path, PathBuf};

/// Steam's own folder: the system Steam's, else Flathub's.
pub fn root() -> Option<PathBuf> {
    let h = hearth_ui::paths::home();
    [h.join(".local/share/Steam"), h.join(".steam/steam"), h.join(".var/app/com.valvesoftware.Steam/.local/share/Steam")].into_iter().find(|p| p.join("userdata").is_dir())
}

pub fn flatpak_steam() -> bool {
    root().map(|r| r.to_string_lossy().contains("com.valvesoftware.Steam")).unwrap_or(false)
}

/// One person who uses Steam on this PC: their folder's id and their name (Steam's own file says it).
#[derive(Debug, Clone, PartialEq)]
pub struct User {
    pub id: String,
    pub name: String,
}

pub fn users() -> Vec<User> {
    let Some(r) = root() else { return vec![] };
    let names = std::fs::read_to_string(r.join("config/loginusers.vdf")).map(|t| vdf::read_text(&t)).unwrap_or(Node::Map(vec![]));
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(r.join("userdata")) else { return out };
    for e in rd.flatten() {
        let id = e.file_name().to_string_lossy().to_string();
        if id == "0" || id == "ac" || !id.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        // loginusers.vdf names people by SteamID64: the folder's id is its lower 32 bits
        let mut name = String::new();
        if let Some(Node::Map(us)) = names.get("users") {
            for (sid, u) in us {
                if sid.parse::<u64>().map(|n| (n & 0xffff_ffff).to_string() == id).unwrap_or(false) {
                    name = if u.str("PersonaName").is_empty() { u.str("AccountName").to_string() } else { u.str("PersonaName").to_string() };
                }
            }
        }
        out.push(User { id, name });
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

pub fn shortcuts_path(user: &str) -> Option<PathBuf> {
    Some(root()?.join("userdata").join(user).join("config/shortcuts.vdf"))
}

pub fn grid_dir(user: &str) -> Option<PathBuf> {
    Some(root()?.join("userdata").join(user).join("config/grid"))
}

/// A person's shortcuts.vdf (an empty list when they have none yet); Err when it's there but can't be read (then
/// nothing is written over it).
pub fn read_shortcuts(user: &str) -> Result<Node, String> {
    let p = shortcuts_path(user).ok_or("Steam isn't on this PC")?;
    match std::fs::read(&p) {
        Ok(b) => vdf::read_binary(&b).map_err(|e| format!("{} can't be read ({e}); nothing was changed", p.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => vdf::read_binary(b""),
        Err(e) => Err(e.to_string()),
    }
}

/// Is Steam running (any process called steam)?
pub fn running() -> bool {
    if let Some(v) = crate::fixture().get("steamRunning").and_then(|v| v.as_bool()) {
        return v;
    }
    let Ok(rd) = std::fs::read_dir("/proc") else { return false };
    rd.flatten().any(|e| {
        let n = e.file_name();
        n.to_string_lossy().chars().all(|c| c.is_ascii_digit()) && std::fs::read_to_string(e.path().join("comm")).map(|c| c.trim() == "steam").unwrap_or(false)
    })
}

fn steam_cmd(args: &[&str]) -> Vec<String> {
    let mut v: Vec<String> = if flatpak_steam() { vec!["flatpak".into(), "run".into(), "com.valvesoftware.Steam".into()] } else { vec!["steam".into()] };
    v.extend(args.iter().map(|a| a.to_string()));
    v
}

/// Asks Steam to close (its own -shutdown) and waits for it, up to a minute. Blocking: on a thread of its own.
pub fn close_and_wait() -> Result<(), String> {
    if !running() {
        return Ok(());
    }
    let c = steam_cmd(&["-shutdown"]);
    let _ = std::process::Command::new(&c[0]).args(&c[1..]).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn();
    for _ in 0..60 {
        std::thread::sleep(std::time::Duration::from_secs(1));
        if !running() {
            // its files are written as it ends: a moment more
            std::thread::sleep(std::time::Duration::from_secs(2));
            return Ok(());
        }
    }
    Err("Steam didn't close within a minute, so nothing was changed".into())
}

/// Opens Steam again (in the background, as it was).
pub fn open_again() {
    let c = steam_cmd(&["-silent"]);
    let refs: Vec<&str> = c.iter().map(|s| s.as_str()).collect();
    hearth_ui::run::detached(&refs);
}

/// The program each shortcut starts: Hearth Retro's --play, so each game starts in the emulator Hearth Retro set up.
pub fn retro_exe() -> String {
    let p = if Path::new("/usr/bin/hearth-retro-app").exists() { "/usr/bin/hearth-retro-app".to_string() } else { "hearth-retro-app".to_string() };
    format!("\"{p}\"")
}
