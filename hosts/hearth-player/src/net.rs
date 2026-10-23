//! The internet side of Podcasts & Radio: reading a feed or a directory's answer, and downloading an episode.
//! Nothing here signs in anywhere or sends anything about you: the feeds you add, iTunes' public podcast search (no
//! key needed) and radio-browser.info's public station list.

use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

pub const USER_AGENT: &str = concat!("HearthPlayer/", env!("CARGO_PKG_VERSION"), " (Hearth OS; +https://github.com/Dragongamer9/hearth-os)");

pub fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .user_agent(USER_AGENT)
        .timeout_connect(Duration::from_secs(15))
        .timeout_read(Duration::from_secs(30))
        .redirects(8)
        .build()
}

fn check_url(url: &str) -> Result<(), String> {
    if url.starts_with("https://") || url.starts_with("http://") { Ok(()) } else { Err("That isn't a web address (it should start with https://).".into()) }
}

/// The text at `url` (a feed, a search answer), up to `max` bytes.
pub fn get_text(url: &str, max: u64) -> Result<String, String> {
    check_url(url)?;
    let resp = agent().get(url).call().map_err(|e| match e {
        ureq::Error::Status(code, _) => format!("The site answered {code}."),
        ureq::Error::Transport(t) => format!("Couldn't reach it: {t}"),
    })?;
    let mut buf = Vec::new();
    resp.into_reader().take(max).read_to_end(&mut buf).map_err(|e| e.to_string())?;
    Ok(crate::media::decode_text(&buf))
}

/// Downloads `url` to `dest` (through a .part file, so a stopped download never looks finished).
pub fn download(url: &str, dest: &Path, progress: &dyn Fn(u64, Option<u64>), cancel: &AtomicBool) -> Result<(), String> {
    check_url(url)?;
    let resp = agent().get(url).call().map_err(|e| format!("The download didn't start: {e}"))?;
    let total = resp.header("content-length").and_then(|v| v.parse::<u64>().ok());
    if let Some(d) = dest.parent() { std::fs::create_dir_all(d).map_err(|e| e.to_string())?; }
    let part = dest.with_extension(format!("{}.part", dest.extension().map(|e| e.to_string_lossy().into_owned()).unwrap_or_default()));
    let mut f = std::fs::File::create(&part).map_err(|e| format!("Couldn't save there: {e}"))?;
    let mut r = resp.into_reader();
    let mut buf = vec![0u8; 128 * 1024];
    let mut got = 0u64;
    let mut last = std::time::Instant::now();
    loop {
        if cancel.load(Ordering::SeqCst) { drop(f); let _ = std::fs::remove_file(&part); return Err("Stopped.".into()); }
        let n = r.read(&mut buf).map_err(|e| { let _ = std::fs::remove_file(&part); format!("The download broke off: {e}") })?;
        if n == 0 { break; }
        f.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        got += n as u64;
        if last.elapsed() > Duration::from_millis(250) { progress(got, total); last = std::time::Instant::now(); }
    }
    f.flush().map_err(|e| e.to_string())?;
    drop(f);
    if let Some(t) = total { if got < t { let _ = std::fs::remove_file(&part); return Err("The download stopped early.".into()); } }
    std::fs::rename(&part, dest).map_err(|e| e.to_string())?;
    progress(got, total);
    Ok(())
}

/// A name that's safe as a file or folder name on any system.
pub fn safe_file_name(s: &str) -> String {
    let t: String = s.chars().map(|c| if c.is_control() || "/\\:*?\"<>|".contains(c) { ' ' } else { c }).collect();
    let t = t.split_whitespace().collect::<Vec<_>>().join(" ");
    let t = t.trim_matches('.').trim();
    let t: String = t.chars().take(120).collect();
    if t.is_empty() { "Untitled".into() } else { t }
}

#[cfg(test)]
mod tests {
    #[test]
    fn names() {
        assert_eq!(super::safe_file_name("Ep 1: \"Hello\"/World?"), "Ep 1 Hello World");
        assert_eq!(super::safe_file_name("..."), "Untitled");
    }
}
