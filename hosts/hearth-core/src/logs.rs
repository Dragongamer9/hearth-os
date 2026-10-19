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
        let input: Vec<&str> = lines.iter().copied().filter(|l| l.contains(" input ")).collect();
        let keep = |v: &[&str], n: usize| -> String { let skip = v.len().saturating_sub(n); let mut o = if skip > 0 { format!("[… {skip} older lines left out …]\n") } else { String::new() }; o.push_str(&v[skip..].join("\n")); o };
        if !input.is_empty() { section("Controller input (newest last)", keep(&input, 400)); }
        section("Hearth log", keep(&lines, 1500));
    }
    for (f, name) in [(data.join("session.log"), "Session log"), (run.join("nimbus-shell.log"), "Desktop shell log")] {
        if let Ok(t) = std::fs::read_to_string(&f) { section(name, tail(&t, 400)); }
    }
    section("Memory", sh("free -m; echo; ps -eo rss=,comm= --sort=-rss | head -25 | awk '{printf \"%7.0f MB  %s\\n\", $1/1024, $2}'"));
    section("Games and processes", sh("ps -eo pid,ppid,stat,etime,comm --sort=start_time | tail -60"));
    section("Power (hearth-power)", sh("python3 /usr/share/nimbus/desktop/shell/hearth-power status"));
    section("Controllers (InputPlumber)", sh("busctl --system --list tree org.shadowblip.InputPlumber | head -40; for d in $(busctl --system --list tree org.shadowblip.InputPlumber | grep -o '/org/shadowblip/InputPlumber/CompositeDevice[0-9]*' | sort -u); do echo \"$d\"; busctl --system get-property org.shadowblip.InputPlumber $d org.shadowblip.Input.CompositeDevice Name InterceptMode ProfileName 2>&1 | sed 's/^/  /'; done"));
    section("Controllers and power (Handheld Daemon)", crate::hhd::report());
    section("gamescope", sh("xprop -root | grep -E '^GAMESCOPE|^GAMESCOPECTRL' | cut -c1-300"));
    section("Bluetooth", sh("bluetoothctl devices; bluetoothctl show | grep -E 'Powered|Discoverable|Pairable'"));
    section("System journal: controllers, power, display (this boot)", sh("journalctl -b --no-pager -n 400 -u inputplumber -u hhd -u steamos-manager -u powerstation -u bluetooth -u tuned -u tuned-ppd"));
    section("System journal: errors (this boot)", sh("journalctl -b --no-pager -p err -n 200"));
    section("Your session's journal (this boot)", sh("journalctl --user -b --no-pager -n 800"));
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

/// Hearth OS's bug-report service (the same server as the player counter).
pub const SERVICE: &str = "https://hearth-counter.game305-cl.workers.dev";
const MAX_SEND: usize = 600_000;

/// Saves (as above) and sends: {path, copies, code, url}. Too-long logs lose your session's journal first, then
/// their oldest lines.
pub fn send_to_developer(version: &str, kind: &str) -> Result<Value, String> {
    let mut out = send(version)?;
    let path = out["path"].as_str().unwrap_or("").to_string();
    let mut text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    if text.len() > MAX_SEND {
        if let Some(i) = text.find("\n===== Your session's journal") { text.truncate(i); text.push_str("\n[your session's journal left out: too long]\n"); }
    }
    if text.len() > MAX_SEND {
        let mut cut = text.len() - MAX_SEND;
        while !text.is_char_boundary(cut) { cut += 1; }
        let head_end = text.find("\n===== Hearth log").unwrap_or(0);
        if cut > head_end { text = format!("{}\n[… older lines left out …]\n{}", &text[..head_end], &text[head_end + (cut - head_end)..]); }
    }
    let tmp = std::env::temp_dir().join(format!("hearth-send-{}.txt", std::process::id()));
    std::fs::write(&tmp, &text).map_err(|e| e.to_string())?;
    let r = crate::system::run_timeout("curl", &["-sS", "-m", "60", "-X", "POST", "-H", "Content-Type: text/plain; charset=utf-8",
        "-H", &format!("X-Hearth-Version: {version}"), "-H", &format!("X-Hearth-Kind: {kind}"),
        "--data-binary", &format!("@{}", tmp.to_string_lossy()), &format!("{SERVICE}/v1/logs")], Duration::from_secs(70));
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
    fn takes_out_personal_details() {
        let t = redact("home /home/sam/x refresh_token=abcdef123456 mail sam.smith@gmail.com id 76561198012345678 Bearer abcdefghijkl sam logged in, ssid: CasaWifi",
                       "sam", "/home/sam");
        assert!(!t.contains("/home/sam"), "{t}");
        assert!(!t.contains("abcdef123456") && !t.contains("gmail") && !t.contains("76561198012345678") && !t.contains("abcdefghijkl") && !t.contains("CasaWifi"), "{t}");
        assert!(t.contains("[user] logged in"), "{t}");
        assert_eq!(tail("a\nb\nc", 2), "[… 1 older lines left out …]\nb\nc");
    }
}
