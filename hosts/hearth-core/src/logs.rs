//! Settings › System › Send logs: everything useful for a bug report in one text file, with personal details
//! taken out, saved in the home folder and on any USB drive or SD card that's plugged in, and (Send) sent to
//! Hearth OS's own bug-report service (the player counter's server), which answers with a short code: the
//! person shares the code or link with the developer, who reads the logs with it. Kept there 14 days.

use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::time::Duration;


fn tail(text: &str, n: usize) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let skip = lines.len().saturating_sub(n);
    let mut out = String::new();
    if skip > 0 { out.push_str(&format!("[… {skip} older lines left out …]\n")); }
    out.push_str(&lines[skip..].join("\n"));
    out
}

/// The newest `max` bytes of a section (whole lines).
fn cap(text: &str, max: usize) -> String {
    if text.len() <= max { return text.to_string(); }
    let mut from = text.len() - max;
    while !text.is_char_boundary(from) { from += 1; }
    let from = text[from..].find('\n').map(|i| from + i + 1).unwrap_or(from);
    format!("[… {} KB of older lines left out …]\n{}", from / 1024, &text[from..])
}

/// The journal without crash dumps' module lists and stack traces (thousands of lines each, which pushed Hearth's
/// own log out of a report): each crash keeps its "Process … dumped core" line and the first frames of its first
/// thread.
pub fn without_dump_details(journal: &str) -> String {
    let mut out = Vec::new();
    let mut frames = 0usize;
    let mut in_dump = false;
    let mut hidden = 0usize;
    for l in journal.lines() {
        let t = l.trim_start();
        let continuation = l.starts_with(' ') || l.starts_with('\t');
        if !continuation { in_dump = l.contains("dumped core"); frames = 0; if hidden > 0 { out.push(format!("    [… {hidden} lines of the crash dump left out …]")); hidden = 0; } out.push(l.to_string()); continue; }
        if in_dump {
            if t.starts_with('#') && frames < 6 { frames += 1; out.push(l.to_string()); continue; }
            if t.starts_with("Stack trace of thread") && frames == 0 { out.push(l.to_string()); continue; }
            if !t.is_empty() { hidden += 1; }
            continue;
        }
        out.push(l.to_string());
    }
    if hidden > 0 { out.push(format!("    [… {hidden} lines of the crash dump left out …]")); }
    out.join("\n")
}

fn sh(cmd: &str) -> String {
    crate::system::run_timeout("sh", &["-c", &format!("{cmd} 2>&1")], Duration::from_secs(15)).unwrap_or_else(|e| format!("({e})"))
}

/// Personal details out: the user name and home folder, sign-in tokens, e-mail addresses, Steam account ids,
/// Wi-Fi passwords.
pub fn redact(text: &str, user: &str, home: &str) -> String {
    let mut t = text.to_string();
    if home.len() > 1 { t = t.replace(home, "~"); }
    let rules: [(&str, &str); 6] = [
        (r#"(?i)("?(?:access_token|refresh_token|id_token|token|password|passwd|psk|secret|cookie|authorization|session_?id|code)"?\s*[:=]\s*"?)[^\s",&}]{4,}"#, "$1[removed]"),
        (r"(?i)(bearer\s+)[A-Za-z0-9._~+/=-]{8,}", "$1[removed]"),
        (r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}", "[email]"),
        (r"\b7656119\d{10}\b", "[steam-id]"),
        (r"(?i)([?&](?:code|token|key|sig|signature)=)[^&\s]+", "$1[removed]"),
        (r"(?i)(ssid[\s:=]+)\S+", "$1[wifi]"),
    ];
    for (re, to) in rules { if let Ok(r) = regex::Regex::new(re) { t = r.replace_all(&t, to).to_string(); } }
    if user.len() >= 3 {
        if let Ok(r) = regex::Regex::new(&format!(r"\b{}\b", regex::escape(user))) { t = r.replace_all(&t, "[user]").to_string(); }
    }
    t
}

/// The report: about the device, Hearth's logs, and what the system says about the parts that usually go wrong
/// (controller service, power, gamescope, memory).
pub fn report(version: &str) -> String {
    let data = crate::paths::data_dir();
    let run = PathBuf::from(std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into()));
    let mut info = crate::sysinfo::info(Path::new("/")); info["version"] = json!(version); info["host"] = json!("rust");
    let mut out = format!("Hearth logs · {} · Hearth {version}\n\n", chrono::Local::now().format("%Y-%m-%d %H:%M"));
    let mut section = |title: &str, body: String| { out.push_str(&format!("\n===== {title} =====\n{}\n", body.trim_end())); };
    section("About this device", serde_json::to_string_pretty(&info).unwrap_or_default());
    section("OS", std::fs::read_to_string("/etc/os-release").unwrap_or_default());
    section("System version (bootc)", sh("bootc status --format humanreadable || rpm-ostree status"));
    // Hearth's log: the controller input lines first (the jumping-selection reports), then the newest lines.
    // Thousands of identical "library.downloads ok" lines from the old Electron app are left out.
    if let Ok(t) = std::fs::read_to_string(data.join("bridge.log")) {
        let spam = regex::Regex::new(r"^\d\d:\d\d:\d\d library\.(downloads|get) ok \d+ms$").unwrap();
        let lines: Vec<&str> = t.lines().filter(|l| !spam.is_match(l)).collect();
        let input: Vec<&str> = lines.iter().copied().filter(|l| l.contains(" input ") && !l.contains("art cache")).collect();
        let keep = |v: &[&str], n: usize| -> String { let skip = v.len().saturating_sub(n); let mut o = if skip > 0 { format!("[… {skip} older lines left out …]\n") } else { String::new() }; o.push_str(&v[skip..].join("\n")); o };
        if !input.is_empty() { section("Controller input (newest last)", keep(&input, 400)); }
        section("Hearth log", keep(&lines, 1500));
    }
    for (f, name) in [(data.join("session.log"), "Session log"), (run.join("nimbus-shell.log"), "Desktop shell log"), (data.join("pad.log"), "Desktop controller helper (hearth-pad)")] {
        if let Ok(t) = std::fs::read_to_string(&f) { section(name, tail(&t, 400)); }
    }
    // Multi-seat: how it's set up and what the split-screen host did (labwc's version and warnings, its rules).
    if Path::new(crate::multiplay::SEATS).exists() && (data.join("seat-host.log").exists() || Path::new("/etc/hearth/seats.json").exists() || run.join("hearth-seat-host").exists()) {
        let host_log = std::fs::read_to_string(data.join("seat-host.log")).map(|t| tail(&t, 200)).unwrap_or_else(|_| "(none yet)".into());
        let rules = std::fs::read_to_string(run.join("hearth-seat-host/rc.xml")).unwrap_or_else(|_| "(not running)".into());
        section("Multi-seat", format!("{}\n\nSplit-screen host log:\n{}\n\nHost window rules:\n{}",
            cap(&sh(&format!("{} status", crate::multiplay::SEATS)), 8000), host_log, cap(&rules, 6000)));
    }
    section("Memory", sh("free -m; echo; ps -eo rss=,comm= --sort=-rss | head -25 | awk '{printf \"%7.0f MB  %s\\n\", $1/1024, $2}'"));
    section("Games and processes", sh("ps -eo pid,ppid,stat,etime,comm --sort=start_time | tail -60"));
    section("Power (hearth-power)", sh("python3 /usr/share/nimbus/desktop/shell/hearth-power status"));
    section("Controllers (InputPlumber)", sh("busctl --system --list tree org.shadowblip.InputPlumber | head -40; for d in $(busctl --system --list tree org.shadowblip.InputPlumber | grep -o '/org/shadowblip/InputPlumber/CompositeDevice[0-9]*' | sort -u); do echo \"$d\"; busctl --system get-property org.shadowblip.InputPlumber $d org.shadowblip.Input.CompositeDevice Name InterceptMode ProfileName 2>&1 | sed 's/^/  /'; done"));
    section("Controllers and power (Handheld Daemon)", crate::hhd::report());
    section("gamescope", sh("xprop -root | grep -E '^GAMESCOPE|^GAMESCOPECTRL' | cut -c1-300"));
    section("Bluetooth", sh("bluetoothctl devices; bluetoothctl show | grep -E 'Powered|Discoverable|Pairable'"));
    section("System journal: controllers, power, display (this boot)", cap(&sh("journalctl -b --no-pager -n 400 -u inputplumber -u hhd -u steamos-manager -u powerstation -u bluetooth -u tuned -u tuned-ppd"), 80_000));
    section("System journal: errors (this boot)", cap(&without_dump_details(&sh("journalctl -b --no-pager -p err -n 400")), 80_000));
    section("Your session's journal (this boot)", cap(&without_dump_details(&sh("journalctl --user -b --no-pager -n 800")), 150_000));
    let user = std::env::var("USER").unwrap_or_default();
    redact(&out, &user, &crate::paths::home().to_string_lossy())
}

/// Saves the report in the home folder: Hearth-logs-<date>.txt
pub fn save(version: &str) -> Result<(PathBuf, String), String> {
    let text = report(version);
    let file = crate::paths::home().join(format!("Hearth-logs-{}.txt", chrono::Local::now().format("%Y-%m-%d-%H-%M")));
    std::fs::write(&file, &text).map_err(|e| format!("Couldn't save the logs: {e}"))?;
    Ok((file, text))
}

/// Saves it in the home folder, and a copy on each USB drive or SD card plugged in: {path, copies: [...]}.
pub fn send(version: &str) -> Result<Value, String> {
    let (file, _) = save(version)?;
    let name = file.file_name().map(|n| n.to_os_string()).unwrap_or_default();
    let user = std::env::var("USER").unwrap_or_default();
    let mut copies = vec![];
    for base in [PathBuf::from("/run/media").join(&user), PathBuf::from("/media").join(&user)] {
        for d in std::fs::read_dir(&base).into_iter().flatten().flatten() {
            let to = d.path().join(&name);
            if std::fs::copy(&file, &to).is_ok() { copies.push(to.to_string_lossy().to_string()); }
        }
    }
    Ok(json!({"path": file.to_string_lossy(), "copies": copies}))
}

/// A report made to fit in `max` bytes: the biggest section other than Hearth's own (device, Hearth log, session
/// log) loses its oldest lines first, again and again; Hearth's log only as a last resort. (It used to cut from
/// the start of Hearth's log, and a journal full of crash dumps left no Hearth log at all.)
pub fn fit(text: &str, max: usize) -> String {
    if text.len() <= max { return text.to_string(); }
    let mut parts: Vec<(String, String)> = Vec::new();   // (header line, body)
    let mut head = String::new();
    for (i, chunk) in text.split("\n===== ").enumerate() {
        if i == 0 { head = chunk.to_string(); continue; }
        let (title, body) = chunk.split_once('\n').unwrap_or((chunk, ""));
        parts.push((title.to_string(), body.to_string()));
    }
    let own = |t: &str| t.starts_with("About this device") || t.starts_with("Hearth log") || t.starts_with("Session log") || t.starts_with("OS ");
    let total = |p: &[(String, String)]| head.len() + p.iter().map(|(t, b)| t.len() + b.len() + 8).sum::<usize>();
    for round in 0..40 {
        let size = total(&parts);
        if size <= max { break; }
        let over = size - max;
        let pick = parts.iter().enumerate().filter(|(_, (t, b))| (round >= 20 || !own(t)) && b.len() > 2100).max_by_key(|(_, (_, b))| b.len()).map(|(i, _)| i);
        let Some(i) = pick else { break };
        let body = &parts[i].1;
        let keep = body.len().saturating_sub(over.max(body.len() / 4)).max(2000.min(body.len()));
        parts[i].1 = cap(body, keep);
    }
    let mut out = head;
    for (t, b) in parts { out.push_str("\n===== "); out.push_str(&t); out.push('\n'); out.push_str(&b); }
    out
}

/// Hearth OS's bug-report service (the same server as the player counter).
pub const SERVICE: &str = "https://hearth-counter.game305-cl.workers.dev";
const MAX_SEND: usize = 600_000;

/// Saves (as above) and sends: {path, copies, code, url}. Too-long logs lose your session's journal first, then
/// their oldest lines.
/// A note for an HTTP header: printable ASCII kept, everything else percent-encoded (UTF-8), at most 1000 bytes in.
pub fn header_note(note: &str) -> String {
    let mut n = note.trim().to_string();
    if n.len() > 1000 { let mut c = 1000; while !n.is_char_boundary(c) { c -= 1; } n.truncate(c); }
    let mut out = String::new();
    for b in n.bytes() {
        if b.is_ascii_alphanumeric() || b" -_.~!*'()".contains(&b) { out.push(b as char); } else { out.push_str(&format!("%{b:02X}")); }
    }
    out
}

pub fn send_to_developer(version: &str, kind: &str) -> Result<Value, String> { send_to_developer_note(version, kind, "") }
/// The same, with the person's note ("What happened?"): it goes with the logs as X-Hearth-Note.
pub fn send_to_developer_note(version: &str, kind: &str, note: &str) -> Result<Value, String> {
    crate::watchdog::long_task();   // asked for from the page: up to a minute on purpose
    send_report(version, kind, note)
}
/// The same, from the fail-safe's own thread (which mustn't mark the stuck request as slow on purpose).
pub fn send_report(version: &str, kind: &str, note: &str) -> Result<Value, String> {
    let mut out = send(version)?;
    let path = out["path"].as_str().unwrap_or("").to_string();
    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let text = fit(&text, MAX_SEND);
    let tmp = std::env::temp_dir().join(format!("hearth-send-{}.txt", std::process::id()));
    std::fs::write(&tmp, &text).map_err(|e| e.to_string())?;
    let note_h = format!("X-Hearth-Note: {}", header_note(note));
    let ver_h = format!("X-Hearth-Version: {version}");
    let kind_h = format!("X-Hearth-Kind: {kind}");
    let data = format!("@{}", tmp.to_string_lossy());
    let url = format!("{SERVICE}/v1/logs");
    let mut args: Vec<&str> = vec!["-sS", "-m", "60", "-X", "POST", "-H", "Content-Type: text/plain; charset=utf-8", "-H", &ver_h, "-H", &kind_h];
    if !note.trim().is_empty() { args.push("-H"); args.push(&note_h); }
    args.extend(["--data-binary", &data, &url]);
    let r = crate::system::run_timeout("curl", &args, Duration::from_secs(70));
    let _ = std::fs::remove_file(&tmp);
    let reply: Value = r.ok().and_then(|t| serde_json::from_str(t.trim()).ok()).unwrap_or(Value::Null);
    let code = reply["code"].as_str().ok_or_else(|| format!("Saved to {path}, but sending didn't work{}. Check the internet connection and try again.",
        reply["error"].as_str().map(|e| format!(" ({e})")).unwrap_or_default()))?;
    out["code"] = json!(code);
    out["url"] = json!(format!("{SERVICE}/v1/logs/{code}"));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn notes_fit_in_a_header() {
        assert_eq!(header_note("  Game froze  "), "Game froze");
        assert_eq!(header_note("a\nb"), "a%0Ab");
        assert_eq!(header_note("é"), "%C3%A9");
        assert_eq!(header_note("x: y"), "x%3A y");
        assert!(header_note(&"é".repeat(800)).len() <= 3000);
    }
    #[test]
    fn takes_out_personal_details() {
        let t = redact("home /home/sam/x refresh_token=abcdef123456 mail sam.smith@gmail.com id 76561198012345678 Bearer abcdefghijkl sam logged in, ssid: CasaWifi",
                       "sam", "/home/sam");
        assert!(!t.contains("/home/sam"), "{t}");
        assert!(!t.contains("abcdef123456") && !t.contains("gmail") && !t.contains("76561198012345678") && !t.contains("abcdefghijkl") && !t.contains("CasaWifi"), "{t}");
        assert!(t.contains("[user] logged in"), "{t}");
        assert_eq!(tail("a\nb\nc", 2), "[… 1 older lines left out …]\nb\nc");
    }
    #[test]
    fn crash_dumps_shortened() {
        let j = "Oct 21 x systemd-coredump[1]: Process 5 (mangoapp) of user 1000 dumped core.\n    \n    Module libz.so.1 from rpm zlib\n    Module libx.so\n    Stack trace of thread 5:\n    #0 a\n    #1 b\nOct 21 y kernel: something else";
        let o = without_dump_details(j);
        assert!(o.contains("dumped core") && o.contains("#0 a") && o.contains("something else"));
        assert!(!o.contains("Module libz") && o.contains("2 lines of the crash dump left out"));
    }
    #[test]
    fn fits_without_losing_hearths_log() {
        let big = "x\n".repeat(400_000);
        let text = format!("Hearth logs\n\n===== About this device =====\n{{}}\n\n===== Hearth log =====\nnewest hearth line\n\n===== System journal: errors (this boot) =====\n{big}\n===== Your session's journal (this boot) =====\n{big}");
        let out = fit(&text, 100_000);
        assert!(out.len() <= 100_000 + 200, "{}", out.len());
        assert!(out.contains("newest hearth line") && out.contains("===== System journal: errors") && out.contains("===== Your session's journal"));
    }
}
