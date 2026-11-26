//! hearth_oss: what Hearth's open-source apps share (Listen, Imagine, Passwords, Cloud, Home; their boards in
//! docs/mockups/open-source-apps).
//!
//! * [`w`]: the boards' small parts, drawn in the person's chosen look (hearth_ui's Pal): a switch, choices side by
//!   side, a field, chips, tags, a letter mark, a section label, a side row with its count, a slider's look, bars;
//! * [`http`]: a small HTTP/1.1 client for the services on the person's own PC (127.0.0.1) or reached over Tailscale
//!   (WireGuard already encrypts that link), off the window's thread;
//! * [`proc`]: a program run with its input on stdin (a password never goes on a command line, where any program on
//!   the PC could read it);
//! * [`prefs`]: the app's choices, written only when the person changes one, through settings_file (atomic, .bak,
//!   merged; red flag 9);
//! * [`secret`]: a token kept in the desktop's own keyring (Secret Service through `secret-tool`), never in a file;
//! * [`b64`]: base64 both ways.

pub mod b64;
pub mod http;
pub mod prefs;
pub mod proc;
pub mod secret;
pub mod w;

/// The folder an app keeps its own things in: $XDG_DATA_HOME/hearth/<app> (~/.local/share/hearth/<app>).
pub fn data_dir(app: &str) -> std::path::PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| hearth_ui::paths::home().join(".local/share"))
        .join("hearth")
        .join(app)
}

/// Seconds since 1970.
pub fn now() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

/// A value from a JSON object by any of these keys, ignoring case (services answer in camelCase or PascalCase).
pub fn ci<'a>(v: &'a serde_json::Value, keys: &[&str]) -> Option<&'a serde_json::Value> {
    let o = v.as_object()?;
    for k in keys {
        if let Some(x) = o.get(*k) {
            return Some(x);
        }
        if let Some((_, x)) = o.iter().find(|(kk, _)| kk.eq_ignore_ascii_case(k)) {
            return Some(x);
        }
    }
    None
}
pub fn ci_str(v: &serde_json::Value, keys: &[&str]) -> String {
    ci(v, keys).and_then(|x| x.as_str()).unwrap_or("").to_string()
}

/// A size in words (1.6 GB, 64 MB).
pub fn size_words(b: u64) -> String {
    let f = b as f64;
    if f >= 1e12 {
        format!("{:.2} TB", f / 1e12)
    } else if f >= 1e9 {
        format!("{:.1} GB", f / 1e9)
    } else if f >= 1e6 {
        format!("{:.0} MB", f / 1e6)
    } else if f >= 1e3 {
        format!("{:.0} KB", f / 1e3)
    } else {
        format!("{b} bytes")
    }
}

/// A folder's size, its files only (a quick walk, at most 20 000 files).
pub fn dir_size(p: &std::path::Path) -> u64 {
    let mut total = 0u64;
    let mut stack = vec![p.to_path_buf()];
    let mut n = 0;
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            n += 1;
            if n > 20_000 {
                return total;
            }
            match e.metadata() {
                Ok(m) if m.is_dir() => stack.push(e.path()),
                Ok(m) => total += m.len(),
                _ => {}
            }
        }
    }
    total
}

/// Where a program is: on PATH, or one of these full paths.
pub fn which(names: &[&str], also: &[std::path::PathBuf]) -> Option<std::path::PathBuf> {
    for a in also {
        if a.is_file() {
            return Some(a.clone());
        }
    }
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        for n in names {
            let f = dir.join(n);
            if f.is_file() {
                return Some(f);
            }
        }
    }
    None
}
