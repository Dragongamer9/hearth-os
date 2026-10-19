//! Settings › System › Send logs: everything useful for a bug report in one text file, with personal details
//! taken out, saved in the home folder and on any USB drive or SD card that's plugged in. Nothing leaves the device.

use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Lines kept from the end of each log (the newest part is what matters).
const TAIL: usize = 4000;

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
    for (f, name) in [(data.join("bridge.log"), "Hearth log"), (data.join("session.log"), "Session log"), (run.join("nimbus-shell.log"), "Desktop shell log")] {
        if let Ok(t) = std::fs::read_to_string(&f) { section(name, tail(&t, TAIL)); }
    }
    section("Memory", sh("free -m; echo; ps -eo rss=,comm= --sort=-rss | head -25 | awk '{printf \"%7.0f MB  %s\\n\", $1/1024, $2}'"));
    section("Games and processes", sh("ps -eo pid,ppid,stat,etime,comm --sort=start_time | tail -60"));
    section("Power (hearth-power)", sh("python3 /usr/share/nimbus/desktop/shell/hearth-power status"));
    section("Controllers (InputPlumber)", sh("busctl --system --list tree org.shadowblip.InputPlumber | head -40; for d in $(busctl --system --list tree org.shadowblip.InputPlumber | grep -o '/org/shadowblip/InputPlumber/CompositeDevice[0-9]*' | sort -u); do echo \"$d\"; busctl --system get-property org.shadowblip.InputPlumber $d org.shadowblip.Input.CompositeDevice Name InterceptMode ProfileName 2>&1 | sed 's/^/  /'; done"));
    section("gamescope", sh("xprop -root | grep -E '^GAMESCOPE|^GAMESCOPECTRL' | cut -c1-300"));
    section("Bluetooth", sh("bluetoothctl devices; bluetoothctl show | grep -E 'Powered|Discoverable|Pairable'"));
    section("System journal: controllers, power, display (this boot)", sh("journalctl -b --no-pager -n 400 -u inputplumber -u steamos-manager -u powerstation -u bluetooth -u tuned -u tuned-ppd"));
    section("System journal: errors (this boot)", sh("journalctl -b --no-pager -p err -n 300"));
    section("Your session's journal (this boot)", sh("journalctl --user -b --no-pager -n 3000"));
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
