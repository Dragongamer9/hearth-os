//! MangoHud's own settings files (MangoHud, MIT; the options GOverlay, GPL-3, writes): `~/.config/MangoHud/MangoHud.conf`
//! for every game, `wine-<program>.conf` (or `<program>.conf`) for one game, and `presets.conf` for the levels of Steam's
//! Quick menu › Performance in Game Mode. One `key=value` (or a bare `key`) a line; every other line is kept as it was.
//! MangoHud reads its file again when it changes, so a change shows in a running game at once.

use std::path::PathBuf;

pub fn dir() -> PathBuf {
    let h = hearth_oss_kit::home();
    std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| h.join(".config")).join("MangoHud")
}

fn key_of(line: &str) -> Option<&str> {
    let l = line.trim();
    if l.is_empty() || l.starts_with('#') || l.starts_with('[') {
        return None;
    }
    Some(l.split('=').next().unwrap_or(l).trim())
}

/// A key's value: Some("") for a bare key (on), Some(v) for key=v, None when it isn't there.
pub fn get(text: &str, key: &str) -> Option<String> {
    text.lines().rev().find(|l| key_of(l) == Some(key)).map(|l| l.split_once('=').map(|(_, v)| v.trim().to_string()).unwrap_or_default())
}

/// Is a switch on (a bare key, or key=1/true; key=0 is off).
pub fn on(text: &str, key: &str) -> bool {
    match get(text, key) {
        Some(v) => !matches!(v.as_str(), "0" | "false"),
        None => false,
    }
}

/// `text` with `key` set (Some("") a bare key, Some(v) key=v) or taken out (None), every other line kept.
pub fn set(text: &str, key: &str, value: Option<&str>) -> String {
    let line = value.map(|v| if v.is_empty() { key.to_string() } else { format!("{key}={v}") });
    let mut out: Vec<String> = Vec::new();
    let mut done = false;
    for l in text.lines() {
        if key_of(l) == Some(key) {
            if !done {
                if let Some(n) = &line {
                    out.push(n.clone());
                }
                done = true;
            }
            continue;
        }
        out.push(l.to_string());
    }
    if !done {
        if let Some(n) = line {
            out.push(n);
        }
    }
    let mut s = out.join("\n");
    if !s.is_empty() {
        s.push('\n');
    }
    s
}

/// presets.conf with `[preset N]` holding `body` (the other presets kept as they were).
pub fn set_preset(text: &str, n: u8, body: &str) -> String {
    let head = format!("[preset {n}]");
    let mut out: Vec<String> = Vec::new();
    let mut skip = false;
    let mut done = false;
    for l in text.lines() {
        let t = l.trim();
        if t.starts_with('[') {
            skip = t == head;
            if skip {
                out.push(head.clone());
                out.extend(body.lines().map(str::to_string));
                done = true;
                continue;
            }
        }
        if !skip {
            out.push(l.to_string());
        }
    }
    if !done {
        if !out.is_empty() && !out.last().map(|l| l.trim().is_empty()).unwrap_or(true) {
            out.push(String::new());
        }
        out.push(head);
        out.extend(body.lines().map(str::to_string));
    }
    let mut s = out.join("\n");
    s.push('\n');
    s
}

/// The game a per-game file is for: "wine-ForzaHorizon5.conf" → "ForzaHorizon5".
pub fn program_of(file: &str) -> Option<String> {
    let stem = file.strip_suffix(".conf")?;
    if matches!(stem, "MangoHud" | "presets") {
        return None;
    }
    Some(stem.strip_prefix("wine-").unwrap_or(stem).to_string())
}

/// The per-game file for a program ("ForzaHorizon5.exe" → wine-ForzaHorizon5.conf; a Linux program as it is).
pub fn file_for(program: &str) -> String {
    let p = program.trim();
    match p.strip_suffix(".exe").or_else(|| p.strip_suffix(".EXE")) {
        Some(s) => format!("wine-{s}.conf"),
        None => format!("{p}.conf"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const C: &str = "# Hearth Overlay\nlegacy_layout=false\nfps\nframetime=0\nfont_size=14\nposition=top-left\n";

    #[test]
    fn reads_and_changes_one_line() {
        assert!(on(C, "fps"));
        assert!(!on(C, "frametime"));
        assert!(!on(C, "gpu_stats"));
        assert_eq!(get(C, "font_size").as_deref(), Some("14"));
        let a = set(C, "font_size", Some("18"));
        assert!(a.contains("font_size=18\n") && a.starts_with("# Hearth Overlay\n"));
        assert_eq!(a.lines().count(), C.lines().count());
        let b = set(C, "gpu_stats", Some(""));
        assert!(b.ends_with("position=top-left\ngpu_stats\n"));
        let c = set(C, "fps", None);
        assert!(!c.contains("\nfps\n"));
        assert_eq!(set("", "fps", Some("")), "fps\n");
    }

    #[test]
    fn presets_and_files() {
        let p = set_preset("[preset 1]\nfps_only\n\n[preset 3]\nold\n[preset 4]\nfull\n", 3, "fps\ngpu_stats");
        assert_eq!(p, "[preset 1]\nfps_only\n\n[preset 3]\nfps\ngpu_stats\n[preset 4]\nfull\n");
        assert_eq!(set_preset("", 3, "fps"), "[preset 3]\nfps\n");
        assert_eq!(program_of("wine-ForzaHorizon5.conf").as_deref(), Some("ForzaHorizon5"));
        assert_eq!(program_of("MangoHud.conf"), None);
        assert_eq!(file_for("eldenring.exe"), "wine-eldenring.conf");
        assert_eq!(file_for("supertuxkart"), "supertuxkart.conf");
    }
}
