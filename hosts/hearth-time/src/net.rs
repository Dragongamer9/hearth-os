//! Web requests through curl (on every Hearth OS, and in Windows 10 and later): no TLS code in this program. The
//! address goes to curl on its standard input (a config file), not on its command line, because a subscribed
//! calendar's address can carry a private key and any program on the PC can read command lines.

use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const MAX: usize = 20 * 1024 * 1024;

/// GETs a web address (http/https only). Returns the body of a 2xx answer.
pub fn get(url: &str, timeout_s: u32) -> Result<String, String> {
    let lower = url.trim().to_ascii_lowercase();
    if !(lower.starts_with("https://") || lower.starts_with("http://")) {
        return Err("Only web addresses (https://...) work here.".into());
    }
    let t = timeout_s.to_string();
    let mut cmd = Command::new("curl");
    cmd.args(["-sS", "-L", "--max-redirs", "5", "-m", &t, "-A", "Hearth", "-w", "\n%{http_code}", "-K", "-"])
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(windows)]
    { use std::os::windows::process::CommandExt; cmd.creation_flags(0x0800_0000); }
    let mut child = cmd.spawn().map_err(|_| "No connection: curl isn't installed.".to_string())?;
    if let Some(mut si) = child.stdin.take() {
        let _ = si.write_all(format!("url = {}\n", quote(url.trim())).as_bytes());
    }
    let mut out = Vec::new();
    let mut so = child.stdout.take().ok_or("curl didn't start")?;
    let reader = std::thread::spawn(move || { let mut b = Vec::new(); let _ = so.by_ref().take(MAX as u64 + 16).read_to_end(&mut b); b });
    let start = Instant::now();
    let deadline = Duration::from_secs(timeout_s as u64 + 5);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if start.elapsed() > deadline => { let _ = child.kill(); let _ = child.wait(); return Err("No answer (it took too long).".into()); }
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(e) => return Err(e.to_string()),
        }
    }
    out.extend(reader.join().unwrap_or_default());
    let mut err = String::new();
    if let Some(mut se) = child.stderr.take() { let _ = se.read_to_string(&mut err); }
    let text = String::from_utf8_lossy(&out).into_owned();
    let (body, code) = text.rsplit_once('\n').unwrap_or(("", text.as_str()));
    let code: u16 = code.trim().parse().unwrap_or(0);
    if code == 0 {
        let e = err.trim();
        return Err(if e.is_empty() { "No connection.".into() } else { format!("No connection ({})", e.trim_start_matches("curl: ")) });
    }
    if !(200..300).contains(&code) { return Err(format!("The server answered {code}.")); }
    Ok(body.to_string())
}

/// A value for curl's config file: in double quotes, with \\ \" and line breaks escaped.
pub fn quote(v: &str) -> String {
    let mut o = String::from("\"");
    for c in v.chars() {
        match c { '\\' => o.push_str("\\\\"), '"' => o.push_str("\\\""), '\n' => o.push_str("\\n"), '\r' => o.push_str("\\r"), '\t' => o.push_str("\\t"), c => o.push(c) }
    }
    o.push('"');
    o
}

/// %-encodes a value for a web address.
pub fn urlencode(s: &str) -> String {
    let mut o = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) { o.push(b as char) } else { o.push_str(&format!("%{b:02X}")) }
    }
    o
}

#[cfg(test)]
mod tests {
    #[test]
    fn encoding() {
        assert_eq!(super::urlencode("São Paulo"), "S%C3%A3o%20Paulo");
        assert_eq!(super::quote("a\"b\\c"), "\"a\\\"b\\\\c\"");
        assert!(super::get("file:///etc/passwd", 5).is_err());
    }
}
