//! Games you added yourself: a Linux program or script, an AppImage, a .desktop file, or a Windows .exe (with
//! Proton through umu-launcher, or Wine). Port of library/manual.js and game-options.js's apply().

use serde_json::Value;
use std::path::Path;

pub fn parse_desktop(text: &str) -> std::collections::HashMap<String, String> {
    let mut out = std::collections::HashMap::new();
    let mut in_entry = false;
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with('[') { in_entry = t == "[Desktop Entry]"; continue; }
        if !in_entry { continue; }
        for k in ["Name", "Exec", "Path"] { if let Some(v) = t.strip_prefix(&format!("{k}=")) { out.entry(k.to_string()).or_insert(v.to_string()); } }
    }
    out
}

/// A .desktop Exec line as arguments, without field codes like %f %U.
pub fn exec_argv(exec: &str) -> Vec<String> {
    let mut argv = Vec::new();
    let (mut cur, mut q, mut any) = (String::new(), false, false);
    let mut chars = exec.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => { q = !q; any = true; }
            '\\' if q => { if let Some(n) = chars.next() { cur.push(n); } }
            ' ' if !q => { if any || !cur.is_empty() { argv.push(std::mem::take(&mut cur)); } any = false; }
            _ => cur.push(c),
        }
    }
    if any || !cur.is_empty() { argv.push(cur); }
    argv.into_iter().filter(|a| !(a.len() == 2 && a.starts_with('%') && "fFuUdDnNickvm".contains(&a[1..]))).map(|a| a.replace("%%", "%")).collect()
}

pub fn title_for(file: &str) -> String {
    if file.ends_with(".desktop") { if let Some(n) = std::fs::read_to_string(file).ok().map(|t| parse_desktop(&t)).and_then(|d| d.get("Name").cloned()) { return n; } }
    let base = Path::new(file).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let low = base.to_lowercase();
    let stem = [".exe", ".desktop", ".appimage", ".sh"].iter().find(|e| low.ends_with(*e)).map(|e| &base[..base.len() - e.len()]).unwrap_or(&base);
    stem.replace(['_', '.'], " ").trim().to_string()
}

/// "A=1 B='two words'" → [(A, 1), (B, two words)]
pub fn parse_env(text: &str) -> Vec<(String, String)> {
    let re = regex::Regex::new(r#"([A-Za-z_][A-Za-z0-9_]*)=("([^"]*)"|'([^']*)'|(\S*))"#).unwrap();
    re.captures_iter(text).map(|c| (c[1].to_string(), c.get(3).or(c.get(4)).or(c.get(5)).map(|m| m.as_str().to_string()).unwrap_or_default())).collect()
}
pub fn split_args(text: &str) -> Vec<String> {
    let re = regex::Regex::new(r#""([^"]*)"|'([^']*)'|(\S+)"#).unwrap();
    re.captures_iter(text).map(|c| c.get(1).or(c.get(2)).or(c.get(3)).unwrap().as_str().to_string()).collect()
}

/// A launch with the game's settings applied: extra arguments, environment, MangoHud, GameMode, Proton, FSR.
pub fn apply(opts: &Value, cmd: &str, args: Vec<String>, proton: bool) -> (String, Vec<String>, Vec<(String, String)>) {
    let mut env = parse_env(opts["env"].as_str().unwrap_or(""));
    if proton { if let Some(p) = opts["proton"].as_str().filter(|p| !p.is_empty()) { env.push(("PROTONPATH".into(), p.into())); } }
    if proton && opts["fsr"] == true { env.push(("WINE_FULLSCREEN_FSR".into(), "1".into())); }
    if opts["mangohud"] == true { env.push(("MANGOHUD".into(), "1".into())); }
    let mut a = args;
    a.extend(split_args(opts["args"].as_str().unwrap_or("")));
    let mut c = cmd.to_string();
    if opts["gamemode"] == true && crate::system::has("gamemoderun") { a.insert(0, c); c = "gamemoderun".into(); }
    (c, a, env)
}

/// Starts it. Returns the process id when Hearth started a process it can watch. `game_id` (appid.rs) goes in its
/// environment as HEARTH_GAME_ID, so its processes and windows can be told apart from other games'. GAMEID (umu and
/// Proton's game fixes) stays as it was.
pub fn launch(file: &str, extra_args: &str, opts: &Value, game_id: u32) -> Result<u32, String> {
    if !Path::new(file).exists() { return Err(format!("File not found: {file}")); }
    let cwd = Path::new(file).parent().map(|p| p.to_path_buf()).unwrap_or_else(|| "/".into());
    let args = split_args(extra_args);
    let low = file.to_lowercase();
    let (cmd, argv, env, dir, proton) = if low.ends_with(".desktop") {
        let d = parse_desktop(&std::fs::read_to_string(file).unwrap_or_default());
        let exec = d.get("Exec").ok_or("That .desktop file has no Exec line")?;
        let mut v = exec_argv(exec);
        if v.is_empty() { return Err("That .desktop file has no Exec line".into()); }
        let c = v.remove(0); v.extend(args);
        (c, v, vec![], d.get("Path").map(|p| p.into()).unwrap_or(cwd), false)
    } else if low.ends_with(".exe") {
        let mut v = vec![file.to_string()]; v.extend(args);
        if crate::system::has("umu-run") { ("umu-run".to_string(), v, vec![("GAMEID".to_string(), std::env::var("GAMEID").unwrap_or_else(|_| "umu-default".into()))], cwd, true) }
        else if crate::system::has("wine") { ("wine".to_string(), v, vec![], cwd, false) }
        else { return Err("To run Windows games, install umu-launcher (recommended) or Wine".into()); }
    } else {
        (file.to_string(), args, vec![], cwd, false)
    };
    let (c, a, mut e) = apply(opts, &cmd, argv, proton);
    e.extend(env);
    if game_id != 0 { e.push(crate::appid::env(game_id)); }
    crate::bridge::log(&format!("launch: {} with HEARTH_GAME_ID={game_id}", Path::new(file).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()));
    let child = std::process::Command::new(&c).args(&a).envs(e).current_dir(dir)
        .stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null())
        .spawn().map_err(|err| if err.kind() == std::io::ErrorKind::PermissionDenied { format!("{} isn't executable. Mark it as executable (chmod +x) and try again", Path::new(file).file_name().unwrap().to_string_lossy()) } else { format!("Couldn't start it: {err}") })?;
    Ok(crate::procs::detach(child))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn launch_bits() {
        assert_eq!(exec_argv(r#"env WINEDEBUG=-all "/opt/My Game/run" %U --x %%"#), vec!["env", "WINEDEBUG=-all", "/opt/My Game/run", "--x", "%"]);
        assert_eq!(title_for("/g/Hollow_Knight.AppImage"), "Hollow Knight");
        assert_eq!(parse_env("A=1 B='two words' C=\"x y\""), vec![("A".into(), "1".into()), ("B".into(), "two words".into()), ("C".into(), "x y".into())]);
        let (c, a, e) = apply(&json!({"args": "-skipintro \"-w 1\"", "mangohud": true, "proton": "GE-Proton", "fsr": true}), "umu-run", vec!["g.exe".into()], true);
        assert_eq!(c, "umu-run"); assert_eq!(a, vec!["g.exe", "-skipintro", "-w 1"]);
        assert!(e.contains(&("PROTONPATH".into(), "GE-Proton".into())) && e.contains(&("MANGOHUD".into(), "1".into())) && e.contains(&("WINE_FULLSCREEN_FSR".into(), "1".into())));
    }
}
