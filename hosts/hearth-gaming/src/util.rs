//! Small things the three apps share: Hearth's folders, writing a file all at once, safe file names, times,
//! opening a folder, and reading the command line.

use std::path::{Component, Path, PathBuf};

pub fn home() -> PathBuf { std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/")) }

fn xdg(var: &str, fallback: &str) -> PathBuf {
    std::env::var_os(var).map(PathBuf::from).filter(|p| p.is_absolute()).unwrap_or_else(|| home().join(fallback))
}
/// ~/.local/share (or $XDG_DATA_HOME)
pub fn xdg_data() -> PathBuf { xdg("XDG_DATA_HOME", ".local/share") }
/// ~/.config (or $XDG_CONFIG_HOME)
pub fn xdg_config() -> PathBuf { xdg("XDG_CONFIG_HOME", ".config") }
/// ~/.cache (or $XDG_CACHE_HOME)
pub fn xdg_cache() -> PathBuf { xdg("XDG_CACHE_HOME", ".cache") }
/// Hearth's data folder (~/.local/share/nimbus), the same one the Hearth app uses.
pub fn data_dir() -> PathBuf { xdg_data().join("nimbus") }
/// Hearth's settings folder (~/.config/nimbus).
pub fn config_dir() -> PathBuf { xdg_config().join("nimbus") }

/// Writes a file all at once (a temporary file next to it, then a rename), so a crash never leaves half a file.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    if let Some(dir) = path.parent() { if !dir.as_os_str().is_empty() { std::fs::create_dir_all(dir)?; } }
    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let tmp = path.with_file_name(format!(".{name}.{}-{}.tmp", std::process::id(), SEQ.fetch_add(1, Ordering::SeqCst)));
    let r = (|| {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        drop(f);
        std::fs::rename(&tmp, path)
    })();
    if r.is_err() { let _ = std::fs::remove_file(&tmp); }
    r
}

/// Reads a JSON file; anything missing or damaged is `Null`.
pub fn read_json(path: &Path) -> serde_json::Value {
    std::fs::read(path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(serde_json::Value::Null)
}
pub fn write_json(path: &Path, v: &serde_json::Value) -> Result<(), String> {
    write_atomic(path, serde_json::to_string_pretty(v).unwrap_or_default().as_bytes()).map_err(|e| format!("Couldn't save {} ({e})", path.display()))
}

/// A name for a file or folder made from a title: no slashes or characters Windows and FAT/exFAT drives refuse,
/// at most 80 characters, never "." or "..". Spaces and other letters stay readable.
pub fn safe_name(title: &str) -> String {
    let s: String = title.chars().map(|c| if c.is_control() || "/\\:*?\"<>|".contains(c) { ' ' } else { c }).collect();
    let s = s.split_whitespace().collect::<Vec<_>>().join(" ");
    let s: String = s.trim_matches(|c: char| c == '.' || c == ' ').chars().take(80).collect();
    let s = s.trim_end_matches(['.', ' ']).to_string();
    if s.is_empty() { "Game".into() } else { s }
}

/// A library id as a folder name: letters, digits, - and _ as they are, anything else %XX (so "steam:620" is
/// "steam%3A620"). No id can name another folder.
pub fn id_key(id: &str) -> String {
    let mut out = String::new();
    for b in id.bytes() {
        if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' { out.push(b as char); } else { out.push_str(&format!("%{b:02X}")); }
    }
    if out.is_empty() { "game".into() } else { out }
}

/// A path inside an archive or a backup, checked: relative, no "..", no empty parts. Returned with / between parts.
pub fn clean_rel(p: &str) -> Option<PathBuf> {
    let p = p.replace('\\', "/");
    if p.starts_with('/') || p.is_empty() { return None; }
    let mut out = PathBuf::new();
    for c in Path::new(&p).components() {
        match c {
            Component::Normal(s) => out.push(s),
            Component::CurDir => {}
            _ => return None,
        }
    }
    if out.as_os_str().is_empty() { None } else { Some(out) }
}

pub fn now_ms() -> u64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0) }
pub fn mtime_ms(m: &std::fs::Metadata) -> u64 {
    m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_millis() as u64).unwrap_or(0)
}
/// The local date and time for a file name: 2026-10-22 18-05.
pub fn stamp() -> String { chrono::Local::now().format("%Y-%m-%d %H-%M").to_string() }
pub fn iso_now() -> String { chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%:z").to_string() }

/// This computer's name (for "backed up on ...").
pub fn host_name() -> String {
    std::fs::read_to_string("/etc/hostname").ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
        .or_else(|| std::env::var("HOSTNAME").ok()).or_else(|| std::env::var("COMPUTERNAME").ok()).unwrap_or_else(|| "this PC".into())
}

/// Opens a folder in the file manager.
pub fn open_folder(p: &Path) -> Result<(), String> {
    if !p.exists() { return Err(format!("{} isn't there anymore.", p.display())); }
    #[cfg(windows)]
    let r = std::process::Command::new("explorer").arg(p).spawn();
    #[cfg(not(windows))]
    let r = std::process::Command::new("xdg-open").arg(p).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn();
    r.map(|mut c| { std::thread::spawn(move || { let _ = c.wait(); }); }).map_err(|e| format!("Couldn't open the folder ({e})"))
}

/// `--name value` or `--name=value` from the command line.
pub fn arg_value(args: &[String], name: &str) -> Option<String> {
    let eq = format!("{name}=");
    for (i, a) in args.iter().enumerate() {
        if a == name { return args.get(i + 1).cloned(); }
        if let Some(v) = a.strip_prefix(&eq) { return Some(v.to_string()); }
    }
    None
}
pub fn has_arg(args: &[String], name: &str) -> bool { args.iter().any(|a| a == name) }

/// Size of a file or everything in a folder (links aren't followed).
pub fn tree_size(p: &Path) -> u64 {
    let Ok(m) = std::fs::symlink_metadata(p) else { return 0 };
    if m.is_file() { return m.len(); }
    if !m.is_dir() { return 0; }
    std::fs::read_dir(p).map(|rd| rd.flatten().map(|e| tree_size(&e.path())).sum()).unwrap_or(0)
}

/// Every file under a folder (or the file itself), links not followed.
pub fn files_under(p: &Path, out: &mut Vec<PathBuf>) {
    let Ok(m) = std::fs::symlink_metadata(p) else { return };
    if m.is_file() { out.push(p.to_path_buf()); return; }
    if !m.is_dir() { return; }
    if let Ok(rd) = std::fs::read_dir(p) {
        let mut v: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
        v.sort();
        for e in v { files_under(&e, out); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn names() {
        assert_eq!(safe_name("Baldur's Gate 3: Deluxe"), "Baldur's Gate 3 Deluxe");
        assert_eq!(safe_name("../.."), "Game");
        assert_eq!(safe_name("  A/B  "), "A B");
        assert_eq!(id_key("steam:620"), "steam%3A620");
        assert_eq!(id_key("../x"), "%2E%2E%2Fx");
    }
    #[test]
    fn relative_paths() {
        assert_eq!(clean_rel("a/b/c.txt"), Some(PathBuf::from("a/b/c.txt")));
        assert_eq!(clean_rel("a\\b"), Some(PathBuf::from("a/b")));
        assert_eq!(clean_rel("./a"), Some(PathBuf::from("a")));
        assert!(clean_rel("../a").is_none());
        assert!(clean_rel("a/../../b").is_none());
        assert!(clean_rel("/etc/passwd").is_none());
        assert!(clean_rel("").is_none());
    }
    #[test]
    fn args() {
        let a: Vec<String> = ["x", "--game", "steam:1", "--to=/tmp"].iter().map(|s| s.to_string()).collect();
        assert_eq!(arg_value(&a, "--game").as_deref(), Some("steam:1"));
        assert_eq!(arg_value(&a, "--to").as_deref(), Some("/tmp"));
        assert!(arg_value(&a, "--nope").is_none());
    }
}
